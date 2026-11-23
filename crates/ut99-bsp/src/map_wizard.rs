// crates/ut99-bsp/src/map_wizard.rs
//
// Guided & Procedural Valid UT99 Map Generator Engine.
// Generates 100% structurally valid, watertight, grid-aligned Unreal Tournament 99 maps
// combining forensic patterns extracted from the official UT99 map corpus (DM-Deck16][,
// DM-Morbias][, DM-Fractal, DM-Codex, CTF-Face, CTF-Coret, DOM-Sesmar, AS-Frigate/Guardia)
// with a deterministic SplitMix64 procedural topology generator.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::armory::{
    ammo_for_weapon, WizardItemDensity, WizardItemQuotaConfig, WizardPowerupPreset,
    WizardWeaponPreset, CLASSIC_UNREAL_WEAPONS, STOCK_POWERUPS_AND_PICKUPS,
    STOCK_TOURNAMENT_WEAPONS,
};
use crate::brush::{
    build_aligned_octagon, build_cube, build_cylinder, build_ramp_oriented, build_sheet_advanced,
    build_stepped_stairs_oriented, Brush, RampDirection, SheetAxis, SheetPreset,
};
use crate::assault_mission::{
    briefing_lines, plan_assault_mission, AssaultMissionPlan, AssaultObjectiveKind, MissionNode,
};
use crate::class_profiles::{class_profile, is_gameplay_pickup, is_navigation_point, Mount};
use crate::ctf_symmetry::{transform_point_symmetry, CtfSymmetryMode};
use crate::placement::PlacementWorld;
use crate::poly::{
    PF_FAKE_BACKDROP, PF_INVISIBLE, PF_NOT_SOLID, PF_PORTAL, PF_SEMISOLID,
    PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
};
use crate::CsgOperation;
use std::collections::VecDeque;

/// Supported Unreal Tournament 99 Game Modes for the Map Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardGameMode {
    DeathMatch,
    TeamDeathMatch,
    Duel1v1,
    CaptureTheFlag,
    Domination,
    Assault,
}

impl WizardGameMode {
    pub fn all() -> &'static [WizardGameMode] {
        &[
            Self::DeathMatch,
            Self::TeamDeathMatch,
            Self::Duel1v1,
            Self::CaptureTheFlag,
            Self::Domination,
            Self::Assault,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::DeathMatch => "DeathMatch (DM-)",
            Self::TeamDeathMatch => "Team DeathMatch (DM-)",
            Self::Duel1v1 => "Tournament 1v1 Duel (DM-)",
            Self::CaptureTheFlag => "Capture The Flag (CTF-)",
            Self::Domination => "Domination (DOM-)",
            Self::Assault => "Assault (AS-)",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            Self::DeathMatch | Self::TeamDeathMatch | Self::Duel1v1 => "DM",
            Self::CaptureTheFlag => "CTF",
            Self::Domination => "DOM",
            Self::Assault => "AS",
        }
    }

    pub fn default_game_type_class(&self) -> &'static str {
        match self {
            Self::DeathMatch | Self::Duel1v1 => "Botpack.DeathMatchPlus",
            Self::TeamDeathMatch => "Botpack.TeamGamePlus",
            Self::CaptureTheFlag => "Botpack.CTFGame",
            Self::Domination => "Botpack.Domination",
            Self::Assault => "Botpack.Assault",
        }
    }

    pub fn default_ideal_player_count(&self) -> &'static str {
        match self {
            Self::Duel1v1 => "2",
            Self::DeathMatch => "4-8",
            Self::TeamDeathMatch => "6-12",
            Self::CaptureTheFlag => "8-16",
            Self::Domination => "6-12",
            Self::Assault => "8-14",
        }
    }
}

/// Tactical category grouping for the 18 predefined architectural layouts in the Map Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardArchetypeCategory {
    ArenaAndDuel,
    SymmetricalTeam,
    MultiWingDomination,
    VerticalAndOrbital,
    SiegeAndMonument,
}

impl WizardArchetypeCategory {
    pub fn all() -> &'static [WizardArchetypeCategory] {
        &[
            Self::ArenaAndDuel,
            Self::SymmetricalTeam,
            Self::MultiWingDomination,
            Self::VerticalAndOrbital,
            Self::SiegeAndMonument,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ArenaAndDuel => "⚔ Arena & Duel",
            Self::SymmetricalTeam => "⚑ Symmetrical CTF",
            Self::MultiWingDomination => "♛ Multi-Wing & DOM",
            Self::VerticalAndOrbital => "▲ Vertical & Orbital",
            Self::SiegeAndMonument => "■ Siege & Monument",
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::ArenaAndDuel => "ARENA / DUEL",
            Self::SymmetricalTeam => "SYMMETRICAL CTF",
            Self::MultiWingDomination => "MULTI-WING / DOM",
            Self::VerticalAndOrbital => "VERTICAL / ORBITAL",
            Self::SiegeAndMonument => "SIEGE / MONUMENT",
        }
    }
}

/// Canonical architectural archetypes derived from forensic study of official UT99 maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardArchetype {
    // --- Category 1: Arena & Duel (4) ---
    /// Inspired by DM-Morbias][ & DM-Fractal: Central arena with multi-tier upper walkway,
    /// corner elevator/stair shafts, central powerup pedestal, and optional hazard core.
    CentralArenaMultiTier,
    /// Inspired by DM-Grit & DM-Morbias][: Compact octagonal duel crucible with opposed
    /// spawn chambers, North/South weapon shrines, and a continuous outer bypass ring.
    CrucibleDuelPit,
    /// Inspired by DM-Tempest & DM-Agony: Dual-loop Figure-8 arena with North/South combat
    /// halls converging on a central nexus and 4 continuous outer loop corridors.
    FigureEightCrossover,
    /// Inspired by DM-Liandri & DOM-Gearbolt: Grand 8-spoke octagonal star rotunda surrounded
    /// by 4 cardinal halls and 4 corner nodes linked by a 360-degree outer wheel.
    CrossroadsRotunda,
    /// Inspired by DM-Gothic & DM-Codex: Grand vaulted cathedral with a towering nave,
    /// apse high altar with elevated sniper gallery, transept crossing, side aisles, and crypt.
    GothicCathedralNave,
    /// Inspired by DM-Morbias][ & DM-Agony: Multi-tier sunken gladiator arena with low central
    /// combat floor, elevated spectator perimeter balconies, beast holding cells, and 4 axial rampways.
    SunkenColosseumBasin,

    // --- Category 2: Symmetrical CTF & Team Warfare (8) ---
    /// Inspired by CTF-Face, CTF-Coret & CTF-Gauntlet: Two symmetrical opposed bases/towers
    /// connected by a high central bridge and two flanking lower tunnels.
    SymmetricalTwinBases,
    /// Inspired by CTF-Dreary & CTF-Gauntlet: Opposed team fortresses connected by parallel
    /// North/South war trenches, twin midfield pillboxes, and a central bunker breach.
    ParallelTrenchWarfare,
    /// Inspired by CTF-EternalCave & CTF-LavaGiant: Twin octagonal spire citadels with 4
    /// forward staging bays spanning a central chasm via 3 parallel bridges.
    TwinSpireBridges,
    /// Inspired by CTF-Command & CTF-HallOfGiants: Symmetrical hourglass complex with wide
    /// team atriums funneling into a high-intensity central crux plus 2 outer bypass conduits.
    HourglassNexusCTF,
    /// Inspired by CTF-Face & CTF-Face][: Towering opposed orbital spires connected across a deep
    /// space planetary void by an arched central bridge, high sniper decks, and teleporters.
    FacingWorldsOrbitalSpire,
    /// Inspired by CTF-November & CTF-Hydro16: Subterranean naval submarine drydock facility with
    /// central vessel cradle, upper catwalks, drainage conduits, and water turbine ducts.
    HydroSubmarineDock,
    /// Inspired by CTF-LavaGiant & CTF-Kosov: Mountain citadel bastions across a rugged multi-elevation
    /// canyon with upper cliff sniper ridge, central bridge, and lower hazardous chasm.
    CanyonCrestCitadels,
    /// Inspired by CTF-Hydro16 & CTF-November: Symmetrical hydro-electric water turbine dam facility
    /// with high spillway bridge, upper generator turbines, twin penstock sluice tunnels, and team pumping stations.
    SymmetricalHydroStation,

    // --- Category 3: Multi-Wing & Domination Complexes (5) ---
    /// Inspired by DM-Deck16][, DM-Liandri & DM-Codex: Interconnected complex of combat halls,
    /// orthogonal corridors, elevated catwalks, sniper deck, and lower fluid vat.
    InterconnectedComplex,
    /// Inspired by DOM-Sesmar, DM-Turbine & DM-HyperBlast: Central nexus surrounded by 3
    /// tactical combat wings (Alpha, Beta, Gamma) and a continuous outer perimeter ring loop.
    ThreeWayRingArena,
    /// Inspired by DM-Codex & DM-Curse][: Open central courtyard flanked by 4 corner bastions
    /// (NW, NE, SW, SE) linked both by a perimeter cloister ring and a central cross.
    FourCornersCourtyard,
    /// Inspired by DOM-Cryptic & DM-Gothic: Complete 3x3 tactical grid matrix of 9 interconnected
    /// crypt chambers and 6 continuous orthogonal passages with zero dead-ends.
    CatacombLabyrinth,
    /// Inspired by DOM-Cryptic & DOM-Sesmar: Grand central octagonal atrium with contested
    /// Shield Altar connected via 3 symmetrical 120-degree wings and outer bypass tunnels.
    TriadSpireAtrium,
    /// Inspired by DOM-Cinder & DOM-Ghardhen: 3-tier vertical arena featuring a high spire
    /// lookout, central mezzanine crossroad, and sunken basin with dual lift shafts.
    VerticalTierCinder,
    /// Inspired by DOM-Gearbolt & DOM-MetalDream: Industrial smelting facility with an exposed
    /// high-risk center bridge, elevated crane dock, and underground cargo vault.
    GearsIndustrialFurnace,
    /// Inspired by DOM-Ghardhen & DM-Deck16][: 3-tier terraced open pit mine excavation with heavy
    /// mining machinery platforms, conveyor belt chutes, mineral extraction tunnels, and central ore crusher.
    ExcavationMiningQuarry,

    // --- Category 4: Vertical & Orbital (4) ---
    /// Inspired by DM-Phobos, DM-SpaceNoxx & DM-Pyramid: Suspended orbital outpost with
    /// panoramic `PF_FAKE_BACKDROP` skylights/windows and an isolated 3D `SkyZoneInfo` chamber.
    OrbitalOutpostSkybox,
    /// Inspired by DM-Morpheus, DM-Fractal & DM-Fetid: Tall central octagonal shaft connected
    /// to 4 radial sky pods with multi-tier overwatch decks, jump pads, and teleporters.
    VerticalTowerShaft,
    /// Inspired by CTF-Nucleus & DM-Pressure: Towering octagonal inner reactor core surrounded
    /// by 4 cardinal containment bays and a 360-degree outer maintenance ring.
    ReactorCoreConcentric,
    /// Inspired by CTF-Face & DM-SpaceNoxx: Cluster of floating low-gravity asteroid platforms suspended
    /// in deep space, linked by magnetic jump pads, orbital catwalks, and teleporter arches.
    FloatingAsteroidIslands,

    // --- Category 5: Siege & Monument (6) ---
    /// Inspired by AS-Guardia, AS-Frigate & AS-HiSpeed: Progressive multi-stage fortress line
    /// with Beachhead, Outer Courtyard, and Inner Command Citadel connected by breach & flank routes.
    FortressSiegeLine,
    /// Inspired by DM-Pyramid & DM-ArcaneTemple: Monumental central Ziggurat sanctum flanked
    /// by Sun & Moon courtyards, North/South vaults, and processional colonnades.
    ZigguratTempleComplex,
    /// Inspired by AS-Rook & DM-Peak: 5-basin S-curve serpentine canyon run with switchback
    /// gorges and two direct sapper flank tunnels.
    CanyonSerpentineRun,
    /// Inspired by AS-Tutorial & AS-Guardia: 4-stage breach assault across West Landing Bay,
    /// Fuse Generator Chamber, Fortified Bridge Courtyard, and Inner Command Vault.
    GatehouseFuseAssault,
    /// Inspired by AS-Frigate & AS-HiSpeed: 5-compartment linear vessel infiltration across
    /// Stern Boarding Deck, Cargo Hold, Engine Boiler Room, Security Ward, and Forward Helm Bridge.
    LinearInfiltrationVessel,
    /// Inspired by AS-Rook & AS-Overlord: 5-stage medieval mountain castle siege across
    /// Outer Camp, Gatehouse Entry, Grand Library Archive, Winch Tower, and Royal Throne Sanctum.
    CastleKeepWinch,

    // --- 8 New Complex Multi-Tier Architectural Layouts ---
    /// Inspired by DM-Gothic, DM-Codex & DOM-Cryptic: Monumental 3-level hypogeum necropolis
    /// featuring grand sarcophagus hall, vaulted clerestory gallery, sunken catacomb ossuary, and side mortuaries.
    SunkenNecropolisCrypt,
    /// Inspired by AS-Overlord, AS-Guardia & CTF-Gauntlet: Monumental 3-ring concentric assault fortress
    /// with outer approach trenches, elevated barbican ramparts, grand inner courtyard, and fortified keep citadels.
    HyperFortressBastion,
    /// Inspired by CTF-Hydro16 & CTF-November: Subterranean oceanic trench base with central submerged
    /// observation rotunda, pressurized team habitat stations, flooded siphons, and high-pressure drainage conduits.
    DeepSeaAbyssStation,
    /// Inspired by DM-Pressure, DM-Turbine & DM-Deck16: 4-tier vertical industrial refinery complex
    /// with sump vat drainage basin, centrifuge hall, elevated steel catwalks, and overhead crane bridge.
    IndustrialRefineryPlex,
    /// Inspired by DM-Fetid, DM-Pyramid & CTF-LavaGiant: Subterranean volcanic forge with central
    /// basalt caldera, floating crucible platform, stepped radial obsidian galleries, and magma flumes.
    SubterraneanMagmaForge,
    /// Inspired by CTF-Face, DM-HyperBlast & DM-SpaceNoxx: Network of 5 pressurized geodesic biodomes
    /// in deep space with hydroponic nexus, arboretum spires, life support pods, and vacuum skyways.
    OrbitalBiodomeArray,
    /// Inspired by CTF-Nucleus, DM-Fractal & DM-Morpheus: Vertical particle accelerator column with
    /// dual interlocking opposite-phase helical ramp tracks, 4 crossover bridges, and core jump-pads.
    DoubleHelixAccelerator,
    /// Inspired by DM-Morpheus, DM-Phobos & CTF-Face: Staggered multi-skyscraper rooftop arena
    /// across the urban skyline with mag-rail skybridges, helipads, and void leap kickers.
    SkylineRooftopHighway,
}

impl WizardArchetype {
    pub fn all() -> &'static [WizardArchetype] {
        &[
            // Arena & Duel
            Self::CentralArenaMultiTier,
            Self::CrucibleDuelPit,
            Self::FigureEightCrossover,
            Self::CrossroadsRotunda,
            Self::GothicCathedralNave,
            Self::SunkenColosseumBasin,
            Self::SunkenNecropolisCrypt,
            // Symmetrical CTF & Team
            Self::SymmetricalTwinBases,
            Self::ParallelTrenchWarfare,
            Self::TwinSpireBridges,
            Self::HourglassNexusCTF,
            Self::FacingWorldsOrbitalSpire,
            Self::HydroSubmarineDock,
            Self::CanyonCrestCitadels,
            Self::SymmetricalHydroStation,
            Self::HyperFortressBastion,
            Self::DeepSeaAbyssStation,
            // Multi-Wing & Domination
            Self::InterconnectedComplex,
            Self::ThreeWayRingArena,
            Self::FourCornersCourtyard,
            Self::CatacombLabyrinth,
            Self::TriadSpireAtrium,
            Self::VerticalTierCinder,
            Self::GearsIndustrialFurnace,
            Self::ExcavationMiningQuarry,
            Self::IndustrialRefineryPlex,
            Self::SubterraneanMagmaForge,
            // Vertical & Orbital
            Self::OrbitalOutpostSkybox,
            Self::VerticalTowerShaft,
            Self::ReactorCoreConcentric,
            Self::FloatingAsteroidIslands,
            Self::OrbitalBiodomeArray,
            Self::DoubleHelixAccelerator,
            Self::SkylineRooftopHighway,
            // Siege & Monument
            Self::FortressSiegeLine,
            Self::ZigguratTempleComplex,
            Self::CanyonSerpentineRun,
            Self::GatehouseFuseAssault,
            Self::LinearInfiltrationVessel,
            Self::CastleKeepWinch,
        ]
    }

    pub fn category(&self) -> WizardArchetypeCategory {
        match self {
            Self::CentralArenaMultiTier
            | Self::CrucibleDuelPit
            | Self::FigureEightCrossover
            | Self::CrossroadsRotunda
            | Self::GothicCathedralNave
            | Self::SunkenColosseumBasin
            | Self::SunkenNecropolisCrypt => WizardArchetypeCategory::ArenaAndDuel,
            Self::SymmetricalTwinBases
            | Self::ParallelTrenchWarfare
            | Self::TwinSpireBridges
            | Self::HourglassNexusCTF
            | Self::FacingWorldsOrbitalSpire
            | Self::HydroSubmarineDock
            | Self::CanyonCrestCitadels
            | Self::SymmetricalHydroStation
            | Self::HyperFortressBastion
            | Self::DeepSeaAbyssStation => WizardArchetypeCategory::SymmetricalTeam,
            Self::InterconnectedComplex
            | Self::ThreeWayRingArena
            | Self::FourCornersCourtyard
            | Self::CatacombLabyrinth
            | Self::TriadSpireAtrium
            | Self::VerticalTierCinder
            | Self::GearsIndustrialFurnace
            | Self::ExcavationMiningQuarry
            | Self::IndustrialRefineryPlex
            | Self::SubterraneanMagmaForge => WizardArchetypeCategory::MultiWingDomination,
            Self::OrbitalOutpostSkybox
            | Self::VerticalTowerShaft
            | Self::ReactorCoreConcentric
            | Self::FloatingAsteroidIslands
            | Self::OrbitalBiodomeArray
            | Self::DoubleHelixAccelerator
            | Self::SkylineRooftopHighway => WizardArchetypeCategory::VerticalAndOrbital,
            Self::FortressSiegeLine
            | Self::ZigguratTempleComplex
            | Self::CanyonSerpentineRun
            | Self::GatehouseFuseAssault
            | Self::LinearInfiltrationVessel
            | Self::CastleKeepWinch => WizardArchetypeCategory::SiegeAndMonument,
        }
    }

    pub fn is_symmetrical(&self) -> bool {
        matches!(
            self,
            Self::SymmetricalTwinBases
                | Self::ParallelTrenchWarfare
                | Self::TwinSpireBridges
                | Self::HourglassNexusCTF
                | Self::FacingWorldsOrbitalSpire
                | Self::HydroSubmarineDock
                | Self::CanyonCrestCitadels
                | Self::SymmetricalHydroStation
                | Self::HyperFortressBastion
                | Self::DeepSeaAbyssStation
                | Self::CrucibleDuelPit
                | Self::FigureEightCrossover
                | Self::FourCornersCourtyard
                | Self::SunkenColosseumBasin
                | Self::GothicCathedralNave
                | Self::DoubleHelixAccelerator
                | Self::OrbitalBiodomeArray
        )
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::CentralArenaMultiTier => "Central Multi-Tier Arena (Morbias / Fractal DNA)",
            Self::CrucibleDuelPit => "Crucible Octagonal Duel Pit (Grit / 1on1 DNA)",
            Self::FigureEightCrossover => "Figure-8 Dual Loop Crossover (Tempest / Agony DNA)",
            Self::CrossroadsRotunda => "8-Spoke Star Rotunda & Wheel (Liandri / Gearbolt DNA)",
            Self::GothicCathedralNave => "Gothic Cathedral Nave & Crypt (Gothic / Codex DNA)",
            Self::SunkenColosseumBasin => "Sunken Colosseum Basin (Morbias / Agony DNA)",
            Self::SunkenNecropolisCrypt => "Sunken Necropolis Crypts & Tombs (Gothic / Codex DNA)",
            Self::SymmetricalTwinBases => "Symmetrical Twin Bases (Facing Worlds / Coret DNA)",
            Self::ParallelTrenchWarfare => "Parallel War Trenches & Pillboxes (Dreary / Gauntlet DNA)",
            Self::TwinSpireBridges => "Twin Octagonal Spires & Triple Chasm (EternalCave DNA)",
            Self::HourglassNexusCTF => "Hourglass Converging Crux & Bypasses (Command DNA)",
            Self::FacingWorldsOrbitalSpire => "Facing Worlds Orbital Spires & High Void (Face DNA)",
            Self::HydroSubmarineDock => "Hydro Submarine Drydock & Turbines (November / Hydro16 DNA)",
            Self::CanyonCrestCitadels => "Canyon Crest Mountain Citadels (LavaGiant / Kosov DNA)",
            Self::SymmetricalHydroStation => "Symmetrical Hydro Dam Station (Hydro16 / November DNA)",
            Self::HyperFortressBastion => "Hyper-Fortress 3-Ring Bastion (Overlord / Gauntlet DNA)",
            Self::DeepSeaAbyssStation => "Deep Sea Abyss Oceanic Station (Hydro16 / November DNA)",
            Self::InterconnectedComplex => "Interconnected Halls & Vat (Deck16 / Liandri DNA)",
            Self::ThreeWayRingArena => "Three-Way Ring Complex (Turbine / Sesmar DNA)",
            Self::FourCornersCourtyard => "Four Corners Cloister Courtyard (Codex / Curse][ DNA)",
            Self::CatacombLabyrinth => "3x3 Tactical Catacomb Matrix (Cryptic / Gothic DNA)",
            Self::TriadSpireAtrium => "Triad Spire Atrium & Shield Altar (Cryptic / Sesmar DNA)",
            Self::VerticalTierCinder => "Vertical Tier Cinder Arena (Cinder / Ghardhen DNA)",
            Self::GearsIndustrialFurnace => "Gears Industrial Smelter & Bridge (Gearbolt / MetalDream DNA)",
            Self::ExcavationMiningQuarry => "Excavation Mining Quarry Terraces (Ghardhen / Deck16 DNA)",
            Self::IndustrialRefineryPlex => "Industrial Refinery 4-Tier Complex (Pressure / Turbine DNA)",
            Self::SubterraneanMagmaForge => "Subterranean Magma Forge & Caldera (Fetid / LavaGiant DNA)",
            Self::OrbitalOutpostSkybox => "Orbital Outpost + Panoramic Skybox (Phobos DNA)",
            Self::VerticalTowerShaft => "Vertical Tower & Sky Bridges (Morpheus / Fractal DNA)",
            Self::ReactorCoreConcentric => "Concentric Reactor Core & Containment Ring (Nucleus DNA)",
            Self::FloatingAsteroidIslands => "Floating Asteroid Islands & Void (Facing Worlds / SpaceNoxx DNA)",
            Self::OrbitalBiodomeArray => "Orbital Biodome Array & Skyways (Face / SpaceNoxx DNA)",
            Self::DoubleHelixAccelerator => "Double-Helix Quantum Accelerator (Nucleus / Morpheus DNA)",
            Self::SkylineRooftopHighway => "Skyline Rooftop Highway & Penthouses (Morpheus / Phobos DNA)",
            Self::FortressSiegeLine => "Linear Fortress Siege Line (Guardia / Frigate DNA)",
            Self::ZigguratTempleComplex => "Monumental Ziggurat & Sun/Moon Courts (Pyramid DNA)",
            Self::CanyonSerpentineRun => "Serpentine S-Curve Canyon & Sapper Tunnels (Rook DNA)",
            Self::GatehouseFuseAssault => "Gatehouse Fuse & Bridge Breach (Tutorial / Guardia DNA)",
            Self::LinearInfiltrationVessel => "Linear Infiltration Vessel (Frigate / HiSpeed DNA)",
            Self::CastleKeepWinch => "Castle Keep & Winch Bastion (Rook / Overlord DNA)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::CentralArenaMultiTier => {
                "High-paced vertical arena with lower combat floor, upper perimeter gallery, 4 corner transits, and central powerup altar."
            }
            Self::CrucibleDuelPit => {
                "Compact octagonal duel crucible with West/East spawn chambers, North/South weapon shrines, and a fast outer bypass loop."
            }
            Self::FigureEightCrossover => {
                "Dual North/South combat arenas meeting at a central crossover nexus with 4 continuous outer loop corridors."
            }
            Self::CrossroadsRotunda => {
                "Grand octagonal central rotunda linked to 4 cardinal halls and 4 corner nodes via an 8-segment outer perimeter wheel."
            }
            Self::GothicCathedralNave => {
                "Grand vaulted cathedral with towering nave, apse high altar with sniper gallery, transept crossing, side aisles with cloister arches, and subterranean crypt."
            }
            Self::SunkenColosseumBasin => {
                "Multi-tier sunken gladiator arena with low central pit, elevated spectator perimeter balconies, beast holding cells, and 4 axial rampways."
            }
            Self::SunkenNecropolisCrypt => {
                "Monumental 3-level hypogeum necropolis featuring a grand central sarcophagus hall, vaulted clerestory gallery, sunken catacomb ossuary, and side mortuaries."
            }
            Self::SymmetricalTwinBases => {
                "Two mirrored team citadels with flag/objective pedestals, upper sniper balconies, central span, and dual flanking routes."
            }
            Self::ParallelTrenchWarfare => {
                "Opposed team fortresses linked by parallel North/South war trenches, twin midfield pillbox rooms, and a central bunker breach."
            }
            Self::TwinSpireBridges => {
                "Two octagonal team spire citadels and 4 forward staging halls spanning a central sky chasm via 3 parallel bridges."
            }
            Self::HourglassNexusCTF => {
                "Symmetrical hourglass layout where wide team bases converge into an intense central crux flanked by 2 outer bypass tunnels."
            }
            Self::FacingWorldsOrbitalSpire => {
                "Towering opposed orbital spires connected across a deep space void by an arched central bridge, high sniper decks, and teleporters."
            }
            Self::HydroSubmarineDock => {
                "Subterranean naval drydock facility with central vessel cradle, upper catwalks, drainage conduits, and water turbine ducts."
            }
            Self::CanyonCrestCitadels => {
                "Mountain citadel bastions across a rugged multi-elevation canyon with upper cliff sniper ridge, central bridge, and lower hazardous chasm."
            }
            Self::SymmetricalHydroStation => {
                "Symmetrical hydro-electric water turbine dam facility with high spillway bridge, upper generator turbines, twin penstock sluice tunnels, and team pumping stations."
            }
            Self::HyperFortressBastion => {
                "Monumental 3-ring concentric assault fortress with outer approach trenches, elevated barbican ramparts, grand inner courtyard, and fortified keep citadels."
            }
            Self::DeepSeaAbyssStation => {
                "Subterranean oceanic trench base with central submerged observation rotunda, pressurized team habitat stations, flooded siphons, and high-pressure drainage conduits."
            }
            Self::InterconnectedComplex => {
                "Multi-room industrial/temple layout with main atrium, 3 satellite chambers, ring corridors, sniper roost, and fluid basin."
            }
            Self::ThreeWayRingArena => {
                "Three-sector tactical complex (Alpha/Beta/Gamma) linked both through a central nexus and a continuous zero-dead-end outer ring."
            }
            Self::FourCornersCourtyard => {
                "Open central courtyard surrounded by 4 corner bastions (NW, NE, SW, SE) connected by a perimeter cloister and central cross."
            }
            Self::CatacombLabyrinth => {
                "9 interconnected chambers arranged in a 3x3 tactical grid with 6 full-length orthogonal passages and zero dead-ends."
            }
            Self::TriadSpireAtrium => {
                "Grand octagonal central atrium with contested Shield Altar connected to 3 symmetric 120-degree wings and outer bypass tunnels."
            }
            Self::VerticalTierCinder => {
                "3-tier vertical arena featuring a high spire lookout, central mezzanine crossroad, and sunken basin with dual lift shafts."
            }
            Self::GearsIndustrialFurnace => {
                "Industrial smelting plant with an exposed high-risk center bridge, elevated crane dock, and underground cargo vault."
            }
            Self::ExcavationMiningQuarry => {
                "3-tier terraced open pit mine excavation with heavy mining machinery platforms, conveyor belt chutes, mineral extraction tunnels, and central ore crusher."
            }
            Self::IndustrialRefineryPlex => {
                "4-tier vertical industrial refinery complex with sump vat drainage basin, centrifuge hall, elevated steel catwalks, and overhead crane bridge."
            }
            Self::SubterraneanMagmaForge => {
                "Subterranean volcanic forge with central basalt caldera, floating crucible platform, stepped radial obsidian galleries, and magma flumes."
            }
            Self::OrbitalOutpostSkybox => {
                "Deep-space station featuring vast PF_FAKE_BACKDROP ceiling/wall windows coupled to an external SkyZoneInfo planetary chamber."
            }
            Self::VerticalTowerShaft => {
                "Towering octagonal central shaft with 4 cardinal sky pods, radial bridge conduits, elevated decks, and jump-pad trajectories."
            }
            Self::ReactorCoreConcentric => {
                "Tall octagonal inner reactor core surrounded by 4 cardinal containment halls and a 360-degree outer maintenance ring."
            }
            Self::FloatingAsteroidIslands => {
                "Cluster of floating low-gravity asteroid platforms suspended in deep space, linked by magnetic jump pads, orbital catwalks, and teleporter arches."
            }
            Self::OrbitalBiodomeArray => {
                "Network of 5 pressurized geodesic biodomes in deep space with hydroponic nexus, arboretum spires, life support pods, and vacuum skyways."
            }
            Self::DoubleHelixAccelerator => {
                "Vertical particle accelerator column with dual interlocking opposite-phase helical ramp tracks, 4 crossover bridges, and core jump-pads."
            }
            Self::SkylineRooftopHighway => {
                "Staggered multi-skyscraper rooftop arena across the urban skyline with mag-rail skybridges, helipads, and void leap kickers."
            }
            Self::FortressSiegeLine => {
                "Multi-stage fortress progression (Beachhead -> Outer Gate Courtyard -> Inner Command Citadel) with central breach and flanking sapper tunnels."
            }
            Self::ZigguratTempleComplex => {
                "Monumental central temple sanctum flanked by Sun & Moon courtyards, North/South vaults, and processional colonnades."
            }
            Self::CanyonSerpentineRun => {
                "Winding 5-basin S-curve canyon run from West Landing to East Citadel Keep with switchback gorges and sapper flank tunnels."
            }
            Self::GatehouseFuseAssault => {
                "4-stage linear breach assault (West Landing Bay -> Fuse Generator Hall -> Fortified Bridge Yard -> Inner Command Vault) with sequential blast doors."
            }
            Self::LinearInfiltrationVessel => {
                "5-compartment linear vessel infiltration (Stern Boarding Deck -> Cargo Hold -> Engine Core -> Security Ward -> Forward Helm Bridge) with bulkhead gates."
            }
            Self::CastleKeepWinch => {
                "5-stage medieval castle siege (Outer Camp -> Gatehouse Entry -> Grand Library -> Winch Tower Chamber -> Royal Throne Sanctum) with portcullis gates."
            }
        }
    }
}

/// Room footprint geometry style for main combat chambers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WizardRoomShape {
    #[default]
    Rectangular,
    Octagonal,
    Chamfered,
    Mixed,
}

impl WizardRoomShape {
    pub fn all() -> &'static [WizardRoomShape] {
        &[
            Self::Rectangular,
            Self::Octagonal,
            Self::Chamfered,
            Self::Mixed,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rectangular => "Rectangular Halls (Classic Box CSG)",
            Self::Octagonal => "Octagonal Arenas (Cardinal-Aligned 8-Sided)",
            Self::Chamfered => "45° Chamfered Chambers (Beveled Corners)",
            Self::Mixed => "Mixed (Octagonal Hub + Rectangular Wings)",
        }
    }
}

/// Symmetry enforcement for procedural or archetype map generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardSymmetryMode {
    None,
    MirrorX,
    Rotational180,
}

impl WizardSymmetryMode {
    pub fn all() -> &'static [WizardSymmetryMode] {
        &[Self::None, Self::MirrorX, Self::Rotational180]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Asymmetrical (Free Flow)",
            Self::MirrorX => "Mirror X-Axis (Red/Blue Reflection)",
            Self::Rotational180 => "Rotational 180° (Competitive CTF Standard)",
        }
    }
}

impl From<WizardSymmetryMode> for CtfSymmetryMode {
    fn from(mode: WizardSymmetryMode) -> Self {
        match mode {
            WizardSymmetryMode::MirrorX => CtfSymmetryMode::MirrorX,
            WizardSymmetryMode::Rotational180 | WizardSymmetryMode::None => {
                CtfSymmetryMode::Rotational180
            }
        }
    }
}

/// Overall world scale multiplier preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardMapScale {
    Compact,
    Standard,
    Large,
}

impl WizardMapScale {
    pub fn all() -> &'static [WizardMapScale] {
        &[Self::Compact, Self::Standard, Self::Large]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact => "Compact (Fast 1v1 / 2-6 Players)",
            Self::Standard => "Standard (Tournament 4-10 Players)",
            Self::Large => "Large (Epic 8-16 Players)",
        }
    }

    pub fn factor(&self) -> f32 {
        match self {
            Self::Compact => 0.75,
            Self::Standard => 1.0,
            Self::Large => 1.25,
        }
    }
}

/// Layout generation strategy: either a canonical archetype or a seed-driven procedural graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardLayoutStrategy {
    CanonicalArchetype,
    ProceduralSeed,
}

/// Authentic visual, material (`.utx`) and musical (`.umx`) themes extracted from the official UT99 map corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WizardThemePreset {
    Deck16Industrial,
    MorbiasCyberArena,
    PhobosOrbitalSciFi,
    CodexGothicCrypt,
    CoretTechFacility,
    FacingWorldsCitadel,
    SesmarEgyptianTomb,
    MineQuarry,
    WarshipNaval,
    MedievalCastle,
    HeavyFoundry,
    VolcanicCaldera,
    HydroElectric,
    UrbanSkyscraper,
    AncientMonastery,
}

impl WizardThemePreset {
    pub fn all() -> &'static [WizardThemePreset] {
        &[
            Self::Deck16Industrial,
            Self::MorbiasCyberArena,
            Self::PhobosOrbitalSciFi,
            Self::CodexGothicCrypt,
            Self::CoretTechFacility,
            Self::FacingWorldsCitadel,
            Self::SesmarEgyptianTomb,
            Self::MineQuarry,
            Self::WarshipNaval,
            Self::MedievalCastle,
            Self::HeavyFoundry,
            Self::VolcanicCaldera,
            Self::HydroElectric,
            Self::UrbanSkyscraper,
            Self::AncientMonastery,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Deck16Industrial => "Deck16 Industrial Refinery (UTtech2 / DecayedS)",
            Self::MorbiasCyberArena => "Morbias Cyber Arena (uttech2 / SKYBox)",
            Self::PhobosOrbitalSciFi => "Phobos Orbital Reactor (UTtech1 / Starship)",
            Self::CodexGothicCrypt => "Codex Gothic Cathedral (UTcrypt / NaliCast / Ancient)",
            Self::CoretTechFacility => "Coret Military Facility (PlayrShp / Coret_FX)",
            Self::FacingWorldsCitadel => "Facing Worlds Asteroid Citadel (SkyCity / ShaneChurch)",
            Self::SesmarEgyptianTomb => "Sesmar Egyptian Temple (Egypt / Ancient)",
            Self::MineQuarry => "Mine & Military Bunker (Mine / TrenchesFX / Overlord DNA)",
            Self::WarshipNaval => "Naval Warship & High-Tech Vessel (RainFX / Frigate DNA)",
            Self::MedievalCastle => "Medieval Mountain Castle & Keep (NaliCast / Crypt2 / Rook DNA)",
            Self::HeavyFoundry => "Heavy Industrial Foundry (Metalmys / Indus6 / Cinder DNA)",
            Self::VolcanicCaldera => "Volcanic Basalt Caldera & Lava (GenEarth / LavaGiant DNA)",
            Self::HydroElectric => "Hydro-Electric Sluice & Dam (RainFX / Hydro16 DNA)",
            Self::UrbanSkyscraper => "Urban Skyscraper Rooftop & Neon (RainFX / Morpheus DNA)",
            Self::AncientMonastery => "Ancient Monastery & Overgrown Ruins (utcrypt / Curse][ DNA)",
        }
    }

    pub fn palette(&self) -> WizardResolvedPalette {
        match self {
            Self::Deck16Industrial => WizardResolvedPalette {
                wall_texture: "UTtech2.Wall.rClfWl2".to_string(),
                secondary_wall_texture: "UTtech2.Wall.rClfWl3x".to_string(),
                floor_texture: "UTtech2.Floor.rClfFlr9x".to_string(),
                corridor_floor_texture: "UTtech2.Floor.rClfFlr1x".to_string(),
                ceiling_texture: "UTtech2.Ceiling.rClfClg2".to_string(),
                trim_texture: "UTtech2.Trim.rClfTrm3".to_string(),
                pillar_texture: "UTtech2.Base.rClfBsB2".to_string(),
                liquid_texture: "GenFluid.Water.dgoop3".to_string(),
                sky_texture: "GenFluid.Sky.Skyblu".to_string(),
                song: "Godown.Godown".to_string(),
                primary_light_hue: 20,
                primary_light_sat: 160,
                primary_light_bright: 145,
                accent_light_hue: 85,
                accent_light_sat: 40,
                ambient_brightness: 16,
                ambient_hue: 20,
                ambient_saturation: 180,
                hazard_damage_per_sec: 35,
            },
            Self::MorbiasCyberArena => WizardResolvedPalette {
                wall_texture: "uttech2.Trim.rClfTrm3".to_string(),
                secondary_wall_texture: "uttech2.Wall.rClfWl3x".to_string(),
                floor_texture: "uttech2.Floor.rClfFlr1x".to_string(),
                corridor_floor_texture: "DecayedS.Floor.FLOOR8E".to_string(),
                ceiling_texture: "DecayedS.Floor.FLOOR8E".to_string(),
                trim_texture: "DecayedS.Base.Dark1".to_string(),
                pillar_texture: "uttech2.Wall.rClfWl3x".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "GenFluid.Sky.Skyblu".to_string(),
                song: "Run.Run".to_string(),
                primary_light_hue: 128,
                primary_light_sat: 93,
                primary_light_bright: 160,
                accent_light_hue: 160,
                accent_light_sat: 64,
                ambient_brightness: 20,
                ambient_hue: 135,
                ambient_saturation: 160,
                hazard_damage_per_sec: 25,
            },
            Self::PhobosOrbitalSciFi => WizardResolvedPalette {
                wall_texture: "UTtech1.Wall.bmwall3d".to_string(),
                secondary_wall_texture: "Starship.Base.ylw_pnl2".to_string(),
                floor_texture: "UTtech1.Floor.bmFloor4".to_string(),
                corridor_floor_texture: "UTtech1.Ceiling.nmceiling5".to_string(),
                ceiling_texture: "UTtech1.Ceiling.nmceiling5".to_string(),
                trim_texture: "Starship.Base.ylw_pnl2".to_string(),
                pillar_texture: "UTtech1.pillar.pillarblt8lt".to_string(),
                liquid_texture: "GenFluid.lava.Lava2".to_string(),
                sky_texture: "GenFluid.Sky.Skytst6b".to_string(),
                song: "Mech8.Mech8".to_string(),
                primary_light_hue: 160,
                primary_light_sat: 95,
                primary_light_bright: 150,
                accent_light_hue: 0,
                accent_light_sat: 64,
                ambient_brightness: 18,
                ambient_hue: 155,
                ambient_saturation: 150,
                hazard_damage_per_sec: 50,
            },
            Self::CodexGothicCrypt => WizardResolvedPalette {
                wall_texture: "UTcrypt.Wall.utAlanWal2b".to_string(),
                secondary_wall_texture: "Ancient.Base.STONTSTC".to_string(),
                floor_texture: "UTcrypt.Floor.rClfFlr0".to_string(),
                corridor_floor_texture: "Ancient.Base.A_bs3".to_string(),
                ceiling_texture: "Crypt2.Base.Cgold".to_string(),
                trim_texture: "NaliCast.Trim.ttrm2".to_string(),
                pillar_texture: "Ancient.Base.STONTSTC".to_string(),
                liquid_texture: "GenFluid.lava.Lava2".to_string(),
                sky_texture: "ShaneDay.DayskyB".to_string(),
                song: "Course.Course".to_string(),
                primary_light_hue: 24,
                primary_light_sat: 95,
                primary_light_bright: 135,
                accent_light_hue: 0,
                accent_light_sat: 80,
                ambient_brightness: 14,
                ambient_hue: 24,
                ambient_saturation: 140,
                hazard_damage_per_sec: 50,
            },
            Self::CoretTechFacility => WizardResolvedPalette {
                wall_texture: "Coret_FX.Walls.A_Wal1a".to_string(),
                secondary_wall_texture: "PlayrShp.Trim.VTrim2".to_string(),
                floor_texture: "PlayrShp.Base.Blakm_d".to_string(),
                corridor_floor_texture: "PlayrShp.Base.VBlack".to_string(),
                ceiling_texture: "PlayrShp.Light.LIGHTX".to_string(),
                trim_texture: "PlayrShp.Trim.VTrim2".to_string(),
                pillar_texture: "PlayrShp.Base.VBlack".to_string(),
                liquid_texture: "Coret_FX.Glass.SpecGlas".to_string(),
                sky_texture: "Coret_FX.Walls.APanel1".to_string(),
                song: "firebr.firebr".to_string(),
                primary_light_hue: 145,
                primary_light_sat: 110,
                primary_light_bright: 155,
                accent_light_hue: 0,
                accent_light_sat: 120,
                ambient_brightness: 22,
                ambient_hue: 140,
                ambient_saturation: 180,
                hazard_damage_per_sec: 30,
            },
            Self::FacingWorldsCitadel => WizardResolvedPalette {
                wall_texture: "ShaneChurch.archeBloks2".to_string(),
                secondary_wall_texture: "SkyCity.Base.sMarblbs".to_string(),
                floor_texture: "UTtech1.Misc.UTdirt1".to_string(),
                corridor_floor_texture: "SkyCity.Base.sMCoprBs".to_string(),
                ceiling_texture: "SkyCity.Base.sMCoprBs".to_string(),
                trim_texture: "SkyCity.Base.sMarblbs".to_string(),
                pillar_texture: "UTtech1.Misc.rok2".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "JWSky.Sky.HAZE".to_string(),
                song: "Foregone.Foregone".to_string(),
                primary_light_hue: 160,
                primary_light_sat: 95,
                primary_light_bright: 140,
                accent_light_hue: 28,
                accent_light_sat: 80,
                ambient_brightness: 20,
                ambient_hue: 160,
                ambient_saturation: 140,
                hazard_damage_per_sec: 40,
            },
            Self::SesmarEgyptianTomb => WizardResolvedPalette {
                wall_texture: "Egypt.GypWal1".to_string(),
                secondary_wall_texture: "Ancient.Base.STONTSTC".to_string(),
                floor_texture: "Ancient.Base.A_bs3".to_string(),
                corridor_floor_texture: "Ancient.Base.A_bs4".to_string(),
                ceiling_texture: "Ancient.Base.A_bs4".to_string(),
                trim_texture: "Egypt.GypWal1".to_string(),
                pillar_texture: "Ancient.Base.STONTSTC".to_string(),
                liquid_texture: "GenFluid.lava.Lava2".to_string(),
                sky_texture: "ShaneDay.DayskyB".to_string(),
                song: "UnWorld2.UnWorld2".to_string(),
                primary_light_hue: 22,
                primary_light_sat: 90,
                primary_light_bright: 138,
                accent_light_hue: 140,
                accent_light_sat: 90,
                ambient_brightness: 16,
                ambient_hue: 24,
                ambient_saturation: 150,
                hazard_damage_per_sec: 45,
            },
            Self::MineQuarry => WizardResolvedPalette {
                wall_texture: "Mine.Base.BASEIRM3".to_string(),
                secondary_wall_texture: "UT.Woodf2".to_string(),
                floor_texture: "Mine.Base.BASEIRM3".to_string(),
                corridor_floor_texture: "UT.Woodf2".to_string(),
                ceiling_texture: "Mine.Light.LANTRN2".to_string(),
                trim_texture: "Mine.Deco.Pipe3".to_string(),
                pillar_texture: "Mine.pillar.Girderm3".to_string(),
                liquid_texture: "GenFluid.Water.dgoop3".to_string(),
                sky_texture: "ShaneDay.DayskyB".to_string(),
                song: "Mission.Mission".to_string(),
                primary_light_hue: 18,
                primary_light_sat: 180,
                primary_light_bright: 125,
                accent_light_hue: 160,
                accent_light_sat: 64,
                ambient_brightness: 18,
                ambient_hue: 18,
                ambient_saturation: 160,
                hazard_damage_per_sec: 35,
            },
            Self::WarshipNaval => WizardResolvedPalette {
                wall_texture: "RainFX.Rmetl2".to_string(),
                secondary_wall_texture: "RainFX.r-door".to_string(),
                floor_texture: "RainFX.Rtrim1a".to_string(),
                corridor_floor_texture: "RainFX.r-plates".to_string(),
                ceiling_texture: "Starship.Base.Grey".to_string(),
                trim_texture: "RainFX.Rtrim1a".to_string(),
                pillar_texture: "RainFX.r-plates-g".to_string(),
                liquid_texture: "GenIn.Glass.Glass2".to_string(),
                sky_texture: "genfluid.Sky.Skyblu".to_string(),
                song: "Mech8.Mech8".to_string(),
                primary_light_hue: 145,
                primary_light_sat: 70,
                primary_light_bright: 140,
                accent_light_hue: 0,
                accent_light_sat: 180,
                ambient_brightness: 20,
                ambient_hue: 145,
                ambient_saturation: 120,
                hazard_damage_per_sec: 30,
            },
            Self::MedievalCastle => WizardResolvedPalette {
                wall_texture: "UTcrypt.Wall.cbutnubrik22b".to_string(),
                secondary_wall_texture: "NaliCast.Panel.wood2".to_string(),
                floor_texture: "NaliCast.Floor.oldflor2".to_string(),
                corridor_floor_texture: "NaliCast.Base.metal".to_string(),
                ceiling_texture: "Crypt2.Base.Cgold".to_string(),
                trim_texture: "NaliCast.Trim.METWALL".to_string(),
                pillar_texture: "NaliCast.Base.block2".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "GenFluid.Sky.Skyblu".to_string(),
                song: "Colossus.Colossus".to_string(),
                primary_light_hue: 32,
                primary_light_sat: 140,
                primary_light_bright: 115,
                accent_light_hue: 16,
                accent_light_sat: 160,
                ambient_brightness: 14,
                ambient_hue: 32,
                ambient_saturation: 130,
                hazard_damage_per_sec: 40,
            },
            Self::HeavyFoundry => WizardResolvedPalette {
                wall_texture: "UTtech1.Wall.ClifBrk4".to_string(),
                secondary_wall_texture: "Mine.pillar.Girderm3".to_string(),
                floor_texture: "Metalmys.MysMetl2".to_string(),
                corridor_floor_texture: "Mine.Floor.mFloor".to_string(),
                ceiling_texture: "UTtech1.Base.cbutbase_1".to_string(),
                trim_texture: "Mine.pillar.Girderm3".to_string(),
                pillar_texture: "UTtech1.Base.panelbase".to_string(),
                liquid_texture: "hubeffects.LavaX2".to_string(),
                sky_texture: "ShaneDay.DayskyB".to_string(),
                song: "Run.Run".to_string(),
                primary_light_hue: 14,
                primary_light_sat: 180,
                primary_light_bright: 145,
                accent_light_hue: 96,
                accent_light_sat: 80,
                ambient_brightness: 18,
                ambient_hue: 14,
                ambient_saturation: 180,
                hazard_damage_per_sec: 50,
            },
            Self::VolcanicCaldera => WizardResolvedPalette {
                wall_texture: "GenEarth.Rock.Rockwal4".to_string(),
                secondary_wall_texture: "UT.rokwal1".to_string(),
                floor_texture: "UTtech1.Base.mlbbasic1".to_string(),
                corridor_floor_texture: "UT.cementf1".to_string(),
                ceiling_texture: "UT.cementf1".to_string(),
                trim_texture: "UT.metalf3".to_string(),
                pillar_texture: "GenEarth.Rock.Rockwal4".to_string(),
                liquid_texture: "hubeffects.LavaX2".to_string(),
                sky_texture: "shaneDAY.DayskyB".to_string(),
                song: "firebr.firebr".to_string(),
                primary_light_hue: 12,
                primary_light_sat: 220,
                primary_light_bright: 140,
                accent_light_hue: 16,
                accent_light_sat: 160,
                ambient_brightness: 16,
                ambient_hue: 12,
                ambient_saturation: 190,
                hazard_damage_per_sec: 60,
            },
            Self::HydroElectric => WizardResolvedPalette {
                wall_texture: "RainFX.Concrete".to_string(),
                secondary_wall_texture: "Coret_FX.Base.A_Base1".to_string(),
                floor_texture: "RainFX.ConcreteBase".to_string(),
                corridor_floor_texture: "UTtech1.Base.ClifRok2".to_string(),
                ceiling_texture: "Slums.Concrete.Conc22".to_string(),
                trim_texture: "Mine.Base.Ironwalx".to_string(),
                pillar_texture: "UTtech1.Base.panelbase2".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "genfluid.cloud1a".to_string(),
                song: "SaveMe.SaveMe".to_string(),
                primary_light_hue: 120,
                primary_light_sat: 90,
                primary_light_bright: 135,
                accent_light_hue: 144,
                accent_light_sat: 110,
                ambient_brightness: 20,
                ambient_hue: 125,
                ambient_saturation: 120,
                hazard_damage_per_sec: 25,
            },
            Self::UrbanSkyscraper => WizardResolvedPalette {
                wall_texture: "RainFX.Rmetl2".to_string(),
                secondary_wall_texture: "RainFX.Locker".to_string(),
                floor_texture: "FractalFX.ShanConc01".to_string(),
                corridor_floor_texture: "city.Win-gray".to_string(),
                ceiling_texture: "FractalFX.TrimConc01".to_string(),
                trim_texture: "FractalFX.TrimConc01".to_string(),
                pillar_texture: "RainFX.Rmetl2".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "genfluid.Sky.NghSky".to_string(),
                song: "Razor-ub.Razor-ub".to_string(),
                primary_light_hue: 160,
                primary_light_sat: 160,
                primary_light_bright: 135,
                accent_light_hue: 210,
                accent_light_sat: 140,
                ambient_brightness: 20,
                ambient_hue: 170,
                ambient_saturation: 150,
                hazard_damage_per_sec: 30,
            },
            Self::AncientMonastery => WizardResolvedPalette {
                wall_texture: "utcrypt.Wall.utAlanWal2b".to_string(),
                secondary_wall_texture: "utcrypt.Base.rClfFlr0base".to_string(),
                floor_texture: "utcrypt.Floor.rClfFlr0".to_string(),
                corridor_floor_texture: "UTtech1.Floor.floor3b".to_string(),
                ceiling_texture: "NaliCast.Base.metal4".to_string(),
                trim_texture: "NaliCast.Trim.ttrm2".to_string(),
                pillar_texture: "Indus6.Woods.DeckSm1".to_string(),
                liquid_texture: "GenFluid.Water.Water1".to_string(),
                sky_texture: "GenFluid.Sky.Skyblu".to_string(),
                song: "Course.Course".to_string(),
                primary_light_hue: 24,
                primary_light_sat: 120,
                primary_light_bright: 115,
                accent_light_hue: 140,
                accent_light_sat: 90,
                ambient_brightness: 16,
                ambient_hue: 24,
                ambient_saturation: 140,
                hazard_damage_per_sec: 35,
            },
        }
    }
}

/// Resolved visual, texture, lighting, and audio palette (either from a `WizardThemePreset` or cloned from an official `.unr` map).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardResolvedPalette {
    pub wall_texture: String,
    #[serde(default)]
    pub secondary_wall_texture: String,
    pub floor_texture: String,
    #[serde(default)]
    pub corridor_floor_texture: String,
    pub ceiling_texture: String,
    pub trim_texture: String,
    pub pillar_texture: String,
    pub liquid_texture: String,
    pub sky_texture: String,
    pub song: String,
    pub primary_light_hue: u8,
    pub primary_light_sat: u8,
    pub primary_light_bright: u8,
    pub accent_light_hue: u8,
    pub accent_light_sat: u8,
    pub ambient_brightness: u8,
    pub ambient_hue: u8,
    pub ambient_saturation: u8,
    pub hazard_damage_per_sec: i32,
}

impl Default for WizardResolvedPalette {
    fn default() -> Self {
        WizardThemePreset::Deck16Industrial.palette()
    }
}

fn default_true() -> bool {
    true
}

fn default_ammo_per_weapon() -> u32 {
    1
}

fn default_assault_objective_count() -> usize {
    4
}

fn default_domination_point_count() -> usize {
    3
}

/// Step-by-step modular toggles controlling what features are populated in the generated map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardFeatureToggles {
    pub include_lighting: bool,
    pub include_skybox: bool,
    pub include_weapons_and_pickups: bool,
    pub include_bot_ai_paths: bool,
    pub include_elevators_and_movers: bool,
    pub include_environmental_hazards: bool,
    pub include_decorations_and_ambience: bool,
    pub include_jump_pads: bool,
    #[serde(default = "default_true")]
    pub include_ramps_and_bridges: bool,
    #[serde(default = "default_true")]
    pub include_teleporters: bool,
    #[serde(default = "default_true")]
    pub ensure_vertical_reachability: bool,
}

impl Default for WizardFeatureToggles {
    fn default() -> Self {
        Self {
            include_lighting: true,
            include_skybox: true,
            include_weapons_and_pickups: true,
            include_bot_ai_paths: true,
            include_elevators_and_movers: true,
            include_environmental_hazards: true,
            include_decorations_and_ambience: true,
            include_jump_pads: true,
            include_ramps_and_bridges: true,
            include_teleporters: true,
            ensure_vertical_reachability: true,
        }
    }
}

impl WizardFeatureToggles {
    /// Preset: 100% Playtest Ready (all systems enabled).
    pub fn full_playtest_ready() -> Self {
        Self::default()
    }

    /// Preset: Clean Structural Blockout (only CSG geometry, PlayerStarts, and baseline lighting).
    pub fn clean_blockout() -> Self {
        Self {
            include_lighting: true,
            include_skybox: false,
            include_weapons_and_pickups: false,
            include_bot_ai_paths: false,
            include_elevators_and_movers: false,
            include_environmental_hazards: false,
            include_decorations_and_ambience: false,
            include_jump_pads: false,
            include_ramps_and_bridges: false,
            include_teleporters: false,
            ensure_vertical_reachability: false,
        }
    }
}

/// Complete configuration state for the UT99 Valid Map Generator Wizard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapWizardConfig {
    pub map_name: String,
    pub title: String,
    pub author: String,
    pub ideal_player_count: String,
    pub enter_text: String,
    pub game_mode: WizardGameMode,
    pub layout_strategy: WizardLayoutStrategy,
    pub archetype: WizardArchetype,
    #[serde(default)]
    pub room_shape: WizardRoomShape,
    pub scale: WizardMapScale,
    // Procedural Seed parameters
    pub seed: u64,
    pub procedural_room_count: usize,
    pub procedural_vertical_tiers: usize,
    pub procedural_loop_ratio: f32,
    pub symmetry_mode: WizardSymmetryMode,
    // Theme & DNA
    pub theme: WizardThemePreset,
    pub custom_dna_source_map: Option<String>,
    pub custom_palette_override: Option<WizardResolvedPalette>,
    // Armory, Pickups & Powerups configuration
    #[serde(default)]
    pub weapon_preset: WizardWeaponPreset,
    #[serde(default)]
    pub custom_weapon_loadout: Vec<String>,
    #[serde(default)]
    pub powerup_preset: WizardPowerupPreset,
    #[serde(default)]
    pub custom_powerup_loadout: Vec<String>,
    #[serde(default)]
    pub item_density: WizardItemDensity,
    /// Default number of ammo pickups to place alongside each weapon (0..=10, default 1, or 2 for Abundant).
    #[serde(default = "default_ammo_per_weapon")]
    pub default_ammo_per_weapon: u32,
    /// Per-weapon ammunition count overrides (weapon class or ID -> count of ammo boxes).
    #[serde(default)]
    pub custom_ammo_per_weapon: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub item_quota: WizardItemQuotaConfig,
    pub custom_decoration_loadout: Vec<String>,
    pub custom_ambient_sounds: Vec<String>,
    /// Number of sequential `FortStandard` objectives for `WizardGameMode::Assault` (clamped to `2..=5`).
    #[serde(default = "default_assault_objective_count")]
    pub assault_objective_count: usize,
    /// Number of ControlPoints for `WizardGameMode::Domination` (2, 3, or 4; default 3).
    #[serde(default = "default_domination_point_count")]
    pub domination_point_count: usize,
    /// Thematic naming style for Domination ControlPoints.
    #[serde(default)]
    pub domination_naming_style: crate::domination_mission::DominationNamingStyle,
    /// Architectural dais style for Domination ControlPoint pedestals.
    #[serde(default)]
    pub domination_dais_style: crate::domination_mission::DominationDaisStyle,
    /// Whether to generate bot defense points (`DefensePoint`) around each ControlPoint.
    #[serde(default = "default_true")]
    pub domination_include_defense_points: bool,
    /// Whether to place a contested mega-powerup (ShieldBelt/Amp) at the triangle centroid.
    #[serde(default = "default_true")]
    pub domination_contested_center_powerup: bool,
    /// Architectural dais style for CTF FlagBase pedestals.
    #[serde(default)]
    pub ctf_dais_style: crate::ctf_mission::CtfDaisStyle,
    /// Whether to generate elevated sniper overwatch perches with SniperRifles for CTF.
    #[serde(default = "default_true")]
    pub ctf_include_sniper_perches: bool,
    /// Whether to place a contested mega-powerup (UDamage/ShieldBelt) at midfield between bases.
    #[serde(default = "default_true")]
    pub ctf_contested_center_powerup: bool,
    // Custom Rooms & Connections added via interactive 2D minimap
    #[serde(default)]
    pub custom_rooms: Vec<WizardCustomRoom>,
    #[serde(default)]
    pub custom_connections: Vec<WizardCustomConnection>,
    /// User transformation overrides (resizing / moving rooms, including predefined archetype rooms)
    #[serde(default)]
    pub room_overrides: Vec<WizardRoomOverride>,
    /// Custom gameplay objects/actors added via 2D minimap
    #[serde(default)]
    pub custom_markers: Vec<WizardCustomMarker>,
    /// Suppressed/deleted marker labels (allows deleting default archetype markers)
    #[serde(default)]
    pub suppressed_markers: Vec<String>,
    // Feature toggles
    pub toggles: WizardFeatureToggles,
    pub auto_run_build_and_audit: bool,
    /// Selected background music track (empty string = use theme preset's default song).
    #[serde(default)]
    pub music_track: String,
    /// Distance fog preset for atmospheric depth.
    #[serde(default)]
    pub distance_fog: WizardFogPreset,
    /// Architectural detailing complexity level (Clean, StandardTournament [default], EpicHighDetail).
    #[serde(default)]
    pub architectural_detail: WizardArchitecturalDetail,
    /// Construction rules revision. 1 = legacy (v0.23.5 output, bit-for-bit); 2 = portal-aligned
    /// corridors and doorway frames.
    #[serde(default = "default_generator_version")]
    pub generator_version: u32,
}

/// Legacy construction rules (v0.23.5).
pub const GENERATOR_VERSION_LEGACY: u32 = 1;
/// Portal-aligned corridors and doorway frames.
pub const GENERATOR_VERSION_PORTALS: u32 = 2;
/// Rules v2 plus procedural room and corridor heights drawn from a discrete library.
pub const GENERATOR_VERSION_HEIGHTS: u32 = 3;

fn default_generator_version() -> u32 {
    GENERATOR_VERSION_LEGACY
}

/// Architectural detailing complexity level for procedural CSG generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WizardArchitecturalDetail {
    /// Minimalist tournament layout: structural solids, fast CSG (~30-60 brushes)
    Clean,
    /// Authentic tournament standard: recessed ceiling coffers, perimeter trims,
    /// doorway portal frames, sealed zone portals, structural pillars (~120-250 brushes)
    #[default]
    StandardTournament,
    /// Epic high-detail showcase: multi-tier ceiling coffers, unlit light panels,
    /// chamfered wall buttresses, dado rails, weapon alcoves (~300-500 brushes)
    EpicHighDetail,
}

impl WizardArchitecturalDetail {
    pub fn all() -> &'static [Self] {
        &[Self::Clean, Self::StandardTournament, Self::EpicHighDetail]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Clean => "Clean (Fast CSG, Minimal Detail)",
            Self::StandardTournament => "Standard Tournament (Coffers, Trims, Portals) [Default]",
            Self::EpicHighDetail => "Epic High Detail (Multi-Tier Coffers, Buttresses, Niches)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Clean => "Clean",
            Self::StandardTournament => "Standard Tournament",
            Self::EpicHighDetail => "Epic High Detail",
        }
    }
}

/// Room environmental medium / hazard zone type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WizardRoomEnvironment {
    #[default]
    Normal,
    Water,
    Slime,
    Lava,
    LowGravity,
}

impl WizardRoomEnvironment {
    pub fn all() -> &'static [Self] {
        &[Self::Normal, Self::Water, Self::Slime, Self::Lava, Self::LowGravity]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Normal => "Normal (Air)",
            Self::Water => "Water (Swimmable)",
            Self::Slime => "Slime (Toxic Hazard)",
            Self::Lava => "Lava (Lethal Hazard)",
            Self::LowGravity => "Low Gravity (Space Float)",
        }
    }
}

/// Atmospheric distance fog presets for level ambience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WizardFogPreset {
    #[default]
    None,
    CryptMurk,
    SpaceHaze,
    IndustrialSmog,
    LavaGlow,
}

impl WizardFogPreset {
    pub fn all() -> &'static [Self] {
        &[Self::None, Self::CryptMurk, Self::SpaceHaze, Self::IndustrialSmog, Self::LavaGlow]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None (Crisp Visibility)",
            Self::CryptMurk => "Crypt Murk (Dense Toxic Green)",
            Self::SpaceHaze => "Space Haze (Deep Orbital Indigo)",
            Self::IndustrialSmog => "Industrial Smog (Warm Amber Haze)",
            Self::LavaGlow => "Lava Glow (Fiery Crimson Atmosphere)",
        }
    }

    /// Returns (FogColor_R, FogColor_G, FogColor_B, FogDistance)
    pub fn properties(&self) -> (u8, u8, u8, f32) {
        match self {
            Self::None => (0, 0, 0, 0.0),
            Self::CryptMurk => (15, 45, 20, 1800.0),
            Self::SpaceHaze => (25, 35, 75, 3200.0),
            Self::IndustrialSmog => (65, 45, 25, 2400.0),
            Self::LavaGlow => (85, 25, 15, 2000.0),
        }
    }
}

/// Curated iconic UT99 background tracker music tracks.
pub const UT99_MUSIC_TRACKS: &[(&str, &str)] = &[
    ("Theme Default (Auto)", ""),
    ("Foregone Destruction (Facing Worlds)", "Foregone.Foregone"),
    ("Botpack #9 (Tournament Anthem)", "Botmca9.Botmca9"),
    ("Cannonade (Heavy Action)", "Cannon.Cannon"),
    ("Go Down (Dark Gothic Techno)", "Godown.Godown"),
    ("Mechanism Eight (Classic Unreal)", "Mech8.Mech8"),
    ("Mission Landing (Tactical Infiltration)", "Mission.Mission"),
    ("Nether Animal (Temple & Crypt)", "Nether.Nether"),
    ("Organic (Fast Synth Wave)", "Organic.Organic"),
    ("The Course / Phobos (Space Action)", "Course.Course"),
    ("Run (High-Octane Combat)", "Run.Run"),
    ("Firebr (Volcanic Arena)", "firebr.firebr"),
    ("Save Me (Epic Atmosphere)", "SaveMe.SaveMe"),
    ("Wheels of Fire (Speed Arena)", "Wheels.Wheels"),
];

/// User-defined override for a room's position and size (works for both archetype and custom rooms).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardRoomOverride {
    pub room_name: String,
    pub center_offset: Vec3,
    pub size: Vec3,
    #[serde(default)]
    pub environment: WizardRoomEnvironment,
}

/// User-placed gameplay marker (weapon, playerstart, pickup, light, teleporter, etc.) on the 2D minimap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardCustomMarker {
    pub id: String,
    pub label: String,
    pub kind: String, // "PlayerStart", "Weapon", "Powerup", "Health", "Armor", "Light", "Teleporter"
    pub class_name: String, // e.g. "Botpack.ShockRifle", "Engine.PlayerStart", etc.
    pub location: Vec3,
    pub team: Option<u8>,
    #[serde(default)]
    pub teleporter_tag: String,
    #[serde(default)]
    pub teleporter_url: String,
}

/// Severity level of a teleporter configuration issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WizardTeleportIssueSeverity {
    Warning,
    Error,
}

/// Category of teleporter issue detected in the map wizard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WizardTeleportIssueKind {
    /// Exactly 1 teleporter exists in the level (must be >= 2).
    LoneTeleporter,
    /// Teleporter has no destination URL set.
    MissingDestination,
    /// Destination URL does not match any existing Teleporter Tag.
    UnresolvedDestination,
    /// Teleporter has a Tag but is never targeted by any other teleporter.
    UnreachableExit,
    /// Two linked teleporters are placed dangerously close (< 96 UU), risking an infinite loop.
    LoopHazard,
}

/// A specific diagnostic issue for a teleporter in the map wizard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardTeleportIssue {
    pub severity: WizardTeleportIssueSeverity,
    pub kind: WizardTeleportIssueKind,
    pub teleporter_name: String,
    pub target_url: String,
    pub message: String,
}

/// A resolved directional or bidirectional link between two teleporters for 2D visualization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardTeleportLinkPreview {
    pub source_name: String,
    pub dest_name: String,
    pub source_loc: Vec3,
    pub dest_loc: Vec3,
    pub is_bidirectional: bool,
    pub distance: f32,
}

/// Comprehensive audit summary of teleporter network status in the map wizard.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WizardTeleportAuditSummary {
    pub teleporter_count: usize,
    pub valid_links: Vec<WizardTeleportLinkPreview>,
    pub issues: Vec<WizardTeleportIssue>,
    pub is_healthy: bool,
}

/// User-defined custom room added via 2D minimap interaction or scripting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardCustomRoom {
    pub id: String,
    pub name: String,
    pub center: Vec3,
    pub size: Vec3,
    #[serde(default)]
    pub is_octagonal: bool,
    #[serde(default)]
    pub has_hazard_pit: bool,
    #[serde(default)]
    pub sky_ceiling: bool,
    #[serde(default)]
    pub team_owner: Option<u8>,
    #[serde(default)]
    pub environment: WizardRoomEnvironment,
}

/// User-defined custom connection corridor added via 2D minimap interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WizardCustomConnection {
    pub id: String,
    pub from_room: String,
    pub to_room: String,
    #[serde(default = "default_corridor_width")]
    pub width: f32,
    #[serde(default = "default_corridor_height")]
    pub height: f32,
    #[serde(default)]
    pub has_blast_door: bool,
}

fn default_corridor_width() -> f32 {
    288.0
}

fn default_corridor_height() -> f32 {
    320.0
}

/// Generated level identity (filename, spaced human-readable title, and entry message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardMapIdentity {
    pub map_name: String,
    pub title: String,
    pub enter_text: String,
}

static WIZARD_ENTROPY_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0x9E37_79B9_7F4A_7C15);

/// Produces a fresh, non-repeating entropy seed on every invocation by mixing system clock
/// nanoseconds with an atomic monotonic counter via SplitMix64.
pub fn next_wizard_entropy_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1999_11_30);
    let seq = WIZARD_ENTROPY_COUNTER
        .fetch_add(0x9E37_79B9_7F4A_7C15, std::sync::atomic::Ordering::Relaxed);
    let mut rng = SplitMix64::new(nanos ^ seq.rotate_left(17));
    rng.next_u64()
}

// ---------------------------------------------------------------------------
// Semantic-Phonetic UT99 Map Name Lexicon (7 Curated Pools, ~440 Words)
// ---------------------------------------------------------------------------

const ADJECTIVES_POETIC: &[&str] = &[
    "Beautiful", "Silent", "Velvet", "Frozen", "Burning", "Sacred", "Shattered", "Eternal",
    "Crimson", "Obsidian", "Hollow", "Bitter", "Savage", "Lethal", "Twisted", "Fallen",
    "Blind", "Lucid", "Fading", "Brutal", "Cruel", "Divine", "Solemn", "Wicked",
    "Pale", "Bleeding", "Phantom", "Vicious", "Serene", "Merciless", "Electric", "Toxic",
    "Gilded", "Rusted", "Ashen", "Blinding", "Cold", "Dark", "Wild", "Feral",
    "Arcane", "Mystic", "Astral", "Sunken", "Forgotten", "Forbidden", "Cursed", "Blessed",
    "Vivid", "Fatal", "Grim", "Stark", "Sublime", "Violent", "Tragic", "Radiant",
    "Shadow", "Ghost", "Neon", "Chrome", "Scarlet", "Golden", "Silver", "Azure",
    "Broken", "Infinite", "Primal", "Celestial", "Nocturnal", "Infernal", "Spectral", "Sovereign",
];

const NOUNS_DRAMATIC: &[&str] = &[
    "Disaster", "Dream", "Nightmare", "Carnage", "Oblivion", "Requiem", "Paradox", "Vortex",
    "Mirage", "Euphoria", "Agony", "Rapture", "Nemesis", "Genesis", "Eclipse", "Horizon",
    "Tempest", "Fury", "Grace", "Mercy", "Wrath", "Vengeance", "Redemption", "Betrayal",
    "Harmony", "Chaos", "Silence", "Echo", "Abyss", "Void", "Destiny", "Triumph",
    "Ruin", "Sorrow", "Malice", "Fever", "Delirium", "Catalyst", "Anomaly", "Solitude",
    "Frenzy", "Judgment", "Sacrifice", "Covenant", "Prophecy", "Purgatory", "Limbo", "Havoc",
    "Mayhem", "Cataclysm", "Reckoning", "Vendetta", "Illusion", "Symmetry", "Entropy", "Dominion",
    "Odyssey", "Apex", "Zenith", "Nadir", "Solstice", "Equinox", "Reverie", "Omen",
    "Lament", "Torment", "Sanctity", "Heresy",
];

const MODIFIERS_MATERIAL: &[&str] = &[
    "Iron", "Steel", "Titanium", "Cobalt", "Copper", "Bronze", "Metal", "Cyber",
    "Hyper", "Plasma", "Quantum", "Solar", "Lunar", "Orbital", "Cryo", "Pyro",
    "Magma", "Lava", "Acid", "Sulfur", "Carbon", "Quartz", "Granite", "Marble",
    "Basalt", "Onyx", "Amber", "Jade", "Emerald", "Ruby", "Sapphire", "Diamond",
    "Crystal", "Glass", "Mirror", "Laser", "Ion", "Fusion", "Fission", "Thermal",
    "Sonic", "Gravity", "Zero", "Star", "Nebula", "Comet", "Pulsar", "Aether",
    "Neural", "Binary", "Heavy", "Deep", "High", "Upper", "Lower", "Inner",
    "Outer", "Mercury", "Radium", "Argon", "Xenon", "Vector", "Matrix", "Omega",
];

const NOUNS_ARENA: &[&str] = &[
    "Citadel", "Refinery", "Foundry", "Sanctum", "Bastion", "Fortress", "Crucible", "Spire",
    "Tower", "Vault", "Chamber", "Reactor", "Core", "Nexus", "Atrium", "Rotunda",
    "Pantheon", "Colosseum", "Arena", "Pit", "Trench", "Bunker", "Outpost", "Station",
    "Rig", "Platform", "Deck", "Hangar", "Dock", "Bay", "Silo", "Shaft",
    "Conduit", "Turbine", "Furnace", "Smelter", "Kiln", "Forge", "Anvil", "Cathedral",
    "Temple", "Shrine", "Crypt", "Tomb", "Catacomb", "Sepulcher", "Mausoleum", "Monastery",
    "Keep", "Stronghold", "Garrison", "Rampart", "Bulwark", "Bridge", "Crossing", "Junction",
    "Terminal", "Depot", "Observatory", "Courtyard", "Cloister", "Gauntlet", "Labyrinth", "Canyon",
    "Chasm", "Gorge", "Crater", "Basin", "Lagoon", "Dam", "Canal", "Cistern",
    "Aqueduct", "Reliquary", "Armory", "Barracks",
];

const STEMS_MYTH_LORE: &[&str] = &[
    "Liandri", "Tarydium", "Nali", "Skaarj", "Krall", "Phobos", "Deimos", "Morpheus",
    "Hyperion", "Kronos", "Acheron", "Styx", "Tartarus", "Erebus", "Nyx", "Helios",
    "Selene", "Atlas", "Prometheus", "Icarus", "Daedalus", "Minos", "Osiris", "Anubis",
    "Horus", "Thoth", "Sesmar", "Valkyrie", "Odin", "Fenrir", "Midgard", "Asgard",
    "Ymir", "Surtr", "Typhon", "Hydra", "Chimera", "Cerberus", "Gorgon", "Medusa",
    "Pegasus", "Orion", "Sirius", "Vega", "Altair", "Rigel", "Antares", "Polaris",
    "Cygnus", "Draco", "Lyra", "Andromeda", "Cassini", "Kepler", "Halley", "Vanguard",
    "Centurion", "Praetorian", "Gladiator", "Rubicon", "Avalon", "Babylon", "Carthage", "Sparta",
];

const SHORT_PREFIXES: &[&str] = &[
    "Twin", "Red", "Blue", "Black", "Cold", "Dead", "Lost", "Last",
    "Dark", "High", "Deep", "Lone", "Wild", "Blind", "Pure", "Hard",
    "Cross", "Split", "Dual", "Zero", "Skull", "Blood", "Bone", "Ash",
    "Dust", "Rust", "Salt", "Ice", "Fire", "Storm", "Sky", "Sun",
    "Moon", "Star", "Night", "Dawn", "Dusk", "Iron", "Steel", "Gold",
    "Grim", "Pale", "Raw", "True",
];

const SHORT_CORES: &[&str] = &[
    "Sun", "Star", "Sky", "Wind", "Wave", "Rock", "Stone", "Iron",
    "Steel", "Flame", "Frost", "Blood", "Mind", "Soul", "Heart", "Claw",
    "Fang", "Wing", "Horn", "Crown", "Throne", "Gate", "Wall", "Peak",
    "Edge", "Rim", "Core", "Bolt", "Spark", "Pulse", "Shock", "Blast",
    "Storm", "Mist", "Fog", "Sand", "Glass", "Gold", "Lead", "Gear",
    "Arc", "Flux", "Rift", "Vein", "Tide", "Pyre", "Rune", "Ward",
];

const SHORT_LOCI: &[&str] = &[
    "Pit", "Keep", "Spire", "Hall", "Vault", "Crypt", "Tomb", "Peak",
    "Run", "Reach", "Watch", "Hold", "Post", "Base", "Bay", "Deck",
    "Rig", "Well", "Mill", "Forge", "Kiln", "Yard", "Court", "Ring",
    "Loop", "Cross", "Span", "Drop", "Fall", "Rift", "Gap", "Pass",
    "Ford", "Port", "Dock", "Dome", "Arc", "Zone", "Site", "Lair",
    "Den", "Nest", "Lock", "Gate", "Vale", "Glen", "Crag", "Tor",
];

const VERBS_VISCERAL_PARTICIPLE: &[&str] = &[
    "Bleaching", "Bleeding", "Blinding", "Burning", "Choking", "Corroding",
    "Craving", "Crushing", "Decaying", "Devouring", "Draining", "Dying",
    "Echoing", "Festering", "Freezing", "Grieving", "Harrowing", "Haunting",
    "Howling", "Melting", "Piercing", "Pulsing", "Purging", "Raving",
    "Roaring", "Rusting", "Scorching", "Screaming", "Severing", "Shattering",
    "Slashing", "Smoldering", "Stabbing", "Suffocating", "Throbbing", "Trembling",
    "Twitching", "Weeping", "Writhing",
];

const NOUNS_ANATOMICAL_MEDICAL: &[&str] = &[
    "Anus", "Aorta", "Artery", "Bronchus", "Carotid", "Cartilage",
    "Cerebrum", "Clavicle", "Cochlea", "Cortex", "Cranium", "Duodenum",
    "Epidermis", "Esophagus", "Fistula", "Ganglion", "Gluteus", "Larynx",
    "Mandible", "Marrow", "Medulla", "Neuron", "Pelvis", "Pharynx",
    "Plexus", "Retina", "Sphincter", "Spine", "Spleen", "Sternum",
    "Synapse", "Tendon", "Thalamus", "Trachea", "Urethra", "Ventricle",
    "Vertebra", "Vessel", "Viscera",
];

const CELESTIAL_BODIES: &[&str] = &[
    // Planets of the Solar System
    "Mercury", "Venus", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
    // Dwarf planets & trans-Neptunian bodies
    "Ceres", "Charon", "Eris", "Haumea", "Makemake", "Sedna",
    // Major moons & orbital mining bastions
    "Ariel", "Callisto", "Deimos", "Dione", "Enceladus", "Europa", "Ganymede",
    "Hyperion", "Iapetus", "Mimas", "Miranda", "Oberon", "Phobos", "Rhea",
    "Tethys", "Titan", "Titania", "Triton", "Umbriel",
    // Prominent deep-space stars, constellations & cosmic hazards
    "Aldebaran", "Altair", "Andromeda", "Antares", "Arcturus", "Betelgeuse",
    "Deneb", "Kepler", "Nebula", "Nova", "Polaris", "Pulsar", "Quasar",
    "Rigel", "Sirius", "Vega",
];

const SECTOR_SUFFIXES: &[&str] = &[
    "7", "9", "12", "16", "17", "19", "21", "24", "32", "64", "99", "X", "II", "Prime", "Zero",
];

/// Generates a plausible, evocative Unreal Tournament 99 map identity (`map_name`, `title`, `enter_text`)
/// using 8 semantic-phonetic compositional grammar templates over a 550+ word internal lexicon.
/// Capable of generating > 350,000 unique map stems while keeping names compact (`6..=20` stem chars).
pub fn generate_random_map_identity(game_mode: WizardGameMode, seed: u64) -> WizardMapIdentity {
    let mut rng = SplitMix64::new(seed);
    let prefix = if game_mode == WizardGameMode::Duel1v1 {
        "DM-1on1"
    } else {
        game_mode.prefix()
    };

    for _attempt in 0..16 {
        let pattern = rng.range_i32(0, 7);
        let words: Vec<&str> = match pattern {
            // Pattern 0: Poetic Oxymoron / Dramatic Contrast (2 words, e.g. DM-BeautifulDisaster, DOM-MetalDream)
            0 => {
                let adj = if rng.next_f32() < 0.78 {
                    ADJECTIVES_POETIC[(rng.next_u64() as usize) % ADJECTIVES_POETIC.len()]
                } else {
                    MODIFIERS_MATERIAL[(rng.next_u64() as usize) % MODIFIERS_MATERIAL.len()]
                };
                let noun = NOUNS_DRAMATIC[(rng.next_u64() as usize) % NOUNS_DRAMATIC.len()];
                vec![adj, noun]
            }
            // Pattern 1: Atmospheric / Material + Arena Architecture / Biomechanical (2 words, e.g. CTF-ObsidianCitadel, DM-TitaniumFoundry, DOM-CyberCortex)
            1 => {
                let first = if rng.next_f32() < 0.50 {
                    MODIFIERS_MATERIAL[(rng.next_u64() as usize) % MODIFIERS_MATERIAL.len()]
                } else {
                    ADJECTIVES_POETIC[(rng.next_u64() as usize) % ADJECTIVES_POETIC.len()]
                };
                let arch = if rng.next_f32() < 0.82 {
                    NOUNS_ARENA[(rng.next_u64() as usize) % NOUNS_ARENA.len()]
                } else {
                    NOUNS_ANATOMICAL_MEDICAL[(rng.next_u64() as usize) % NOUNS_ANATOMICAL_MEDICAL.len()]
                };
                vec![first, arch]
            }
            // Pattern 2: UT99 Lore / Mythological Stem / Celestial Outpost + Facility (2 words, e.g. DM-HyperionCore, CTF-LiandriVault, DOM-TitanSanctum)
            2 => {
                let lore = if rng.next_f32() < 0.70 {
                    STEMS_MYTH_LORE[(rng.next_u64() as usize) % STEMS_MYTH_LORE.len()]
                } else {
                    CELESTIAL_BODIES[(rng.next_u64() as usize) % CELESTIAL_BODIES.len()]
                };
                let second = if rng.next_f32() < 0.72 {
                    NOUNS_ARENA[(rng.next_u64() as usize) % NOUNS_ARENA.len()]
                } else {
                    NOUNS_DRAMATIC[(rng.next_u64() as usize) % NOUNS_DRAMATIC.len()]
                };
                vec![lore, second]
            }
            // Pattern 3: Compact 3-Word Triad (3 short monosyllabic words, e.g. CTF-TwinSunSpire, DM-DarkStarVault)
            3 => {
                let w1 = SHORT_PREFIXES[(rng.next_u64() as usize) % SHORT_PREFIXES.len()];
                let w2 = SHORT_CORES[(rng.next_u64() as usize) % SHORT_CORES.len()];
                let w3 = SHORT_LOCI[(rng.next_u64() as usize) % SHORT_LOCI.len()];
                vec![w1, w2, w3]
            }
            // Pattern 4: Crisp Tactical Compound (2 short/medium words, e.g. DM-GearBolt, DOM-StormWatch, CTF-StarFall)
            4 => {
                let w1 = SHORT_CORES[(rng.next_u64() as usize) % SHORT_CORES.len()];
                let w2 = SHORT_LOCI[(rng.next_u64() as usize) % SHORT_LOCI.len()];
                vec![w1, w2]
            }
            // Pattern 5: Numbered Industrial / Orbital Sector (2-3 tokens, e.g. DM-Foundry16, CTF-Outpost24, DM-Phobos9)
            5 => {
                let base = if rng.next_f32() < 0.60 {
                    NOUNS_ARENA[(rng.next_u64() as usize) % NOUNS_ARENA.len()]
                } else {
                    STEMS_MYTH_LORE[(rng.next_u64() as usize) % STEMS_MYTH_LORE.len()]
                };
                let num = SECTOR_SUFFIXES[(rng.next_u64() as usize) % SECTOR_SUFFIXES.len()];
                vec![base, num]
            }
            // Pattern 6: Visceral Kinetic / Anatomical Horror (2 words, e.g. DOM-PulsingCerebrum, CTF-ScreamingVentricle)
            6 => {
                let verb = VERBS_VISCERAL_PARTICIPLE[(rng.next_u64() as usize) % VERBS_VISCERAL_PARTICIPLE.len()];
                let noun = NOUNS_ANATOMICAL_MEDICAL[(rng.next_u64() as usize) % NOUNS_ANATOMICAL_MEDICAL.len()];
                vec![verb, noun]
            }
            // Pattern 7: Deep Space Celestial / Planetary Outpost (2 tokens, e.g. CTF-EuropaStation, DM-TitanRefinery, DM-BurningJupiter, DOM-PulsingPluto)
            _ => {
                let roll = rng.next_f32();
                if roll < 0.50 {
                    // Celestial Body + Arena Architecture (e.g. Europa Station, Titan Refinery, Ganymede Vault)
                    let celestial = CELESTIAL_BODIES[(rng.next_u64() as usize) % CELESTIAL_BODIES.len()];
                    let arch = NOUNS_ARENA[(rng.next_u64() as usize) % NOUNS_ARENA.len()];
                    vec![celestial, arch]
                } else if roll < 0.80 {
                    // Visceral Verb / Poetic Adjective + Celestial Body (e.g. Burning Jupiter, Bleeding Venus, Pulsing Pluto)
                    let first = if rng.next_f32() < 0.65 {
                        VERBS_VISCERAL_PARTICIPLE[(rng.next_u64() as usize) % VERBS_VISCERAL_PARTICIPLE.len()]
                    } else {
                        ADJECTIVES_POETIC[(rng.next_u64() as usize) % ADJECTIVES_POETIC.len()]
                    };
                    let celestial = CELESTIAL_BODIES[(rng.next_u64() as usize) % CELESTIAL_BODIES.len()];
                    vec![first, celestial]
                } else {
                    // Celestial Body + Numbered Sector (e.g. Titan 16, Europa 9, Phobos 24)
                    let celestial = CELESTIAL_BODIES[(rng.next_u64() as usize) % CELESTIAL_BODIES.len()];
                    let num = SECTOR_SUFFIXES[(rng.next_u64() as usize) % SECTOR_SUFFIXES.len()];
                    vec![celestial, num]
                }
            }
        };

        // Ensure no two words in the tuple are identical or share the same 4-char prefix
        let mut duplicate = false;
        for i in 0..words.len() {
            for j in (i + 1)..words.len() {
                let a = words[i].to_ascii_lowercase();
                let b = words[j].to_ascii_lowercase();
                if a == b || (a.len() >= 4 && b.len() >= 4 && a[..4] == b[..4]) {
                    duplicate = true;
                }
            }

        }
        if duplicate {
            continue;
        }

        let stem: String = words.concat();
        if !(6..=20).contains(&stem.len()) {
            continue;
        }

        let title = words.join(" ");
        let map_name = format!("{prefix}-{stem}");
        let enter_text = format!("Entering {title}.");
        return WizardMapIdentity {
            map_name,
            title,
            enter_text,
        };
    }

    WizardMapIdentity {
        map_name: format!("{prefix}-BeautifulDisaster"),
        title: "Beautiful Disaster".to_string(),
        enter_text: "Entering Beautiful Disaster.".to_string(),
    }
}

impl Default for MapWizardConfig {
    fn default() -> Self {
        let weapon_preset = WizardWeaponPreset::TournamentStandard;
        let powerup_preset = WizardPowerupPreset::BalancedCompetitive;
        let game_mode = WizardGameMode::DeathMatch;
        let entropy = next_wizard_entropy_seed();
        let identity = generate_random_map_identity(game_mode, entropy);
        Self {
            map_name: identity.map_name,
            title: identity.title,
            author: "UT Modern Map Editor [by VoT]".to_string(),
            ideal_player_count: "4-8".to_string(),
            enter_text: identity.enter_text,
            game_mode,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::InterconnectedComplex,
            room_shape: WizardRoomShape::Rectangular,
            scale: WizardMapScale::Standard,
            seed: entropy,
            procedural_room_count: 6,
            procedural_vertical_tiers: 2,
            procedural_loop_ratio: 0.65,
            symmetry_mode: WizardSymmetryMode::None,
            theme: WizardThemePreset::Deck16Industrial,
            custom_dna_source_map: None,
            custom_palette_override: None,
            weapon_preset,
            custom_weapon_loadout: weapon_preset.default_weapons(),
            powerup_preset,
            custom_powerup_loadout: powerup_preset.default_powerups(),
            item_density: WizardItemDensity::Standard,
            custom_decoration_loadout: Vec::new(),
            custom_ambient_sounds: Vec::new(),
            assault_objective_count: default_assault_objective_count(),
            domination_point_count: default_domination_point_count(),
            domination_naming_style: crate::domination_mission::DominationNamingStyle::AutoThematic,
            domination_dais_style: crate::domination_mission::DominationDaisStyle::SteppedOctagon,
            domination_include_defense_points: true,
            domination_contested_center_powerup: true,
            ctf_dais_style: crate::ctf_mission::CtfDaisStyle::SteppedTeamPedestal,
            ctf_include_sniper_perches: true,
            ctf_contested_center_powerup: true,
            custom_rooms: Vec::new(),
            custom_connections: Vec::new(),
            room_overrides: Vec::new(),
            custom_markers: Vec::new(),
            suppressed_markers: Vec::new(),
            toggles: WizardFeatureToggles::full_playtest_ready(),
            auto_run_build_and_audit: true,
            music_track: String::new(),
            distance_fog: WizardFogPreset::None,
            architectural_detail: WizardArchitecturalDetail::StandardTournament,
            generator_version: GENERATOR_VERSION_LEGACY,
            default_ammo_per_weapon: 1,
            custom_ammo_per_weapon: std::collections::HashMap::new(),
            item_quota: WizardItemQuotaConfig::default(),
        }
    }
}

impl MapWizardConfig {
    /// Returns the configured ammunition count (number of ammo boxes/pickups) for a given weapon.
    pub fn ammo_count_for_weapon(&self, weapon_class: &str) -> u32 {
        let short = weapon_class.trim().rsplit('.').next().unwrap_or(weapon_class.trim());
        let lower = short.to_ascii_lowercase();
        for (k, v) in &self.custom_ammo_per_weapon {
            let k_short = k.trim().rsplit('.').next().unwrap_or(k.trim());
            if k_short.to_ascii_lowercase() == lower {
                return *v;
            }
        }
        if self.item_density == WizardItemDensity::Abundant {
            self.default_ammo_per_weapon.max(2)
        } else {
            self.default_ammo_per_weapon
        }
    }

    /// Returns the environment medium (Normal, Water, Slime, Lava, LowGravity) for a given room name.
    pub fn get_room_environment(&self, room_name: &str) -> WizardRoomEnvironment {
        if let Some(cr) = self.custom_rooms.iter().find(|cr| cr.name == room_name) {
            return cr.environment;
        }
        if let Some(ro) = self.room_overrides.iter().find(|ro| ro.room_name == room_name) {
            return ro.environment;
        }
        WizardRoomEnvironment::Normal
    }

    /// Generates a brand-new random map name, level title, and entry text for the current `game_mode`.
    pub fn randomize_map_identity(&mut self) {
        let entropy = next_wizard_entropy_seed();
        let identity = generate_random_map_identity(self.game_mode, entropy);
        self.map_name = identity.map_name;
        self.title = identity.title;
        self.enter_text = identity.enter_text;
    }

    /// Selects a random predefined `WizardArchetype` (different from the current one).
    pub fn randomize_archetype(&mut self) {
        let all = WizardArchetype::all();
        if all.len() <= 1 {
            return;
        }
        let entropy = next_wizard_entropy_seed();
        let mut idx = (entropy as usize) % all.len();
        if all[idx] == self.archetype {
            idx = (idx + 1) % all.len();
        }
        self.archetype = all[idx];
    }

    /// Returns the effective list of weapons to spawn. If custom_weapon_loadout is empty,
    /// falls back to the current preset's default weapon list.
    pub fn active_weapons(&self) -> Vec<String> {
        if !self.custom_weapon_loadout.is_empty() {
            self.custom_weapon_loadout.clone()
        } else {
            self.weapon_preset.default_weapons()
        }
    }

    /// Returns the effective list of powerups/pickups to spawn. If custom_powerup_loadout is empty,
    /// falls back to the current preset's default powerup list.
    pub fn active_powerups(&self) -> Vec<String> {
        if !self.custom_powerup_loadout.is_empty() {
            self.custom_powerup_loadout.clone()
        } else {
            self.powerup_preset.default_powerups()
        }
    }

    /// Applies a curated weapon loadout preset.
    pub fn apply_weapon_preset(&mut self, preset: WizardWeaponPreset) {
        self.weapon_preset = preset;
        if preset != WizardWeaponPreset::Custom {
            self.custom_weapon_loadout = preset.default_weapons();
        }
    }

    /// Applies a curated powerup/pickups preset.
    pub fn apply_powerup_preset(&mut self, preset: WizardPowerupPreset) {
        self.powerup_preset = preset;
        if preset != WizardPowerupPreset::Custom {
            self.custom_powerup_loadout = preset.default_powerups();
        }
    }

    /// Returns the active resolved palette (either cloned from an official `.unr` map or from the selected theme).
    pub fn active_palette(&self) -> WizardResolvedPalette {
        self.custom_palette_override
            .clone()
            .unwrap_or_else(|| self.theme.palette())
    }

    /// Inspects the prefix of `self.map_name` (e.g. `DM-`, `DOM-`, `CTF-`, `AS-`, `DM-1on1-`).
    /// If a valid mode prefix is detected that differs from `self.game_mode`, updates `self.game_mode`
    /// and synchronizes conventions (ideal player count, symmetry, invariants).
    /// Returns `true` if the game mode was changed.
    pub fn infer_mode_from_map_name_prefix(&mut self) -> bool {
        let upper = self.map_name.trim().to_ascii_uppercase();
        let detected_mode = if upper.starts_with("DM-1ON1-") {
            Some(WizardGameMode::Duel1v1)
        } else if upper.starts_with("CTF-") {
            Some(WizardGameMode::CaptureTheFlag)
        } else if upper.starts_with("DOM-") {
            Some(WizardGameMode::Domination)
        } else if upper.starts_with("AS-") {
            Some(WizardGameMode::Assault)
        } else if upper.starts_with("DM-") {
            if self.game_mode == WizardGameMode::TeamDeathMatch {
                Some(WizardGameMode::TeamDeathMatch)
            } else {
                Some(WizardGameMode::DeathMatch)
            }
        } else {
            None
        };

        if let Some(mode) = detected_mode {
            if mode != self.game_mode {
                self.game_mode = mode;
                self.ideal_player_count = self.game_mode.default_ideal_player_count().to_string();
                self.enforce_mode_invariants();
                return true;
            }
        }
        false
    }

    /// Synchronizes the `map_name` prefix (`DM-`, `DM-1on1-`, `CTF-`, `DOM-`, `AS-`) and `ideal_player_count` with `game_mode`.
    pub fn sync_mode_conventions(&mut self) {
        let prefix = if self.game_mode == WizardGameMode::Duel1v1 {
            "DM-1on1"
        } else {
            self.game_mode.prefix()
        };
        let stem = if let Some(rest) = self.map_name.strip_prefix("DM-1on1-") {
            rest.to_string()
        } else if let Some((_old_pre, rest)) = self.map_name.split_once('-') {
            rest.to_string()
        } else {
            self.map_name.clone()
        };
        let clean_stem = if stem.trim().is_empty() {
            generate_random_map_identity(self.game_mode, next_wizard_entropy_seed())
                .map_name
                .split_once('-')
                .map(|(_, r)| r.to_string())
                .unwrap_or_else(|| "BeautifulDisaster".to_string())
        } else {
            stem
        };
        self.map_name = format!("{prefix}-{clean_stem}");
        self.ideal_player_count = self.game_mode.default_ideal_player_count().to_string();
        if self.game_mode == WizardGameMode::CaptureTheFlag
            && self.symmetry_mode == WizardSymmetryMode::None
        {
            self.symmetry_mode = WizardSymmetryMode::Rotational180;
        }
    }

    /// Enforces UT99 structural and game-mode invariants after any partial or full randomization.
    pub fn enforce_mode_invariants(&mut self) {
        let prefix = if self.game_mode == WizardGameMode::Duel1v1 {
            "DM-1on1"
        } else {
            self.game_mode.prefix()
        };
        let stem = if let Some(rest) = self.map_name.strip_prefix("DM-1on1-") {
            rest.to_string()
        } else if let Some((_old_pre, rest)) = self.map_name.split_once('-') {
            rest.to_string()
        } else {
            self.map_name.clone()
        };
        if !stem.trim().is_empty() {
            self.map_name = format!("{prefix}-{stem}");
        } else {
            self.randomize_map_identity();
        }

        if self.game_mode == WizardGameMode::CaptureTheFlag
            && self.symmetry_mode == WizardSymmetryMode::None
        {
            self.symmetry_mode = WizardSymmetryMode::Rotational180;
        }
        if self.game_mode == WizardGameMode::CaptureTheFlag
            && self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && !matches!(
                self.archetype.category(),
                WizardArchetypeCategory::SymmetricalTeam
            )
        {
            self.archetype = WizardArchetype::SymmetricalTwinBases;
        }
        if self.game_mode == WizardGameMode::Assault
            && self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && !matches!(
                self.archetype.category(),
                WizardArchetypeCategory::SiegeAndMonument
            )
        {
            self.archetype = WizardArchetype::FortressSiegeLine;
        }
        if self.game_mode == WizardGameMode::Domination
            && self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && matches!(
                self.archetype.category(),
                WizardArchetypeCategory::SymmetricalTeam | WizardArchetypeCategory::SiegeAndMonument
            )
        {
            self.archetype = WizardArchetype::ThreeWayRingArena;
        }
        if (self.game_mode == WizardGameMode::DeathMatch || self.game_mode == WizardGameMode::TeamDeathMatch)
            && self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && (matches!(
                self.archetype.category(),
                WizardArchetypeCategory::SymmetricalTeam | WizardArchetypeCategory::SiegeAndMonument
            ) || matches!(
                self.archetype,
                WizardArchetype::ThreeWayRingArena
                    | WizardArchetype::TriadSpireAtrium
                    | WizardArchetype::VerticalTierCinder
                    | WizardArchetype::GearsIndustrialFurnace
            ))
        {
            self.archetype = WizardArchetype::InterconnectedComplex;
        }
        if self.game_mode == WizardGameMode::Duel1v1
            && self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && !matches!(
                self.archetype.category(),
                WizardArchetypeCategory::ArenaAndDuel | WizardArchetypeCategory::VerticalAndOrbital
            )
        {
            self.archetype = WizardArchetype::CrucibleDuelPit;
        }
        if self.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && self.archetype == WizardArchetype::OrbitalOutpostSkybox
        {
            self.toggles.include_skybox = true;
        }
        if self.game_mode == WizardGameMode::Domination {
            self.domination_point_count = self.domination_point_count.clamp(2, 4);
        }
        self.procedural_room_count = self.procedural_room_count.clamp(3, 12);
        self.procedural_vertical_tiers = self.procedural_vertical_tiers.clamp(1, 3);
        self.procedural_loop_ratio = self.procedural_loop_ratio.clamp(0.0, 1.0);
    }

    /// Randomizes Step 1 (Game Mode & Level Identity: `game_mode`, `map_name`, `title`,
    /// `enter_text`, and `ideal_player_count`). When `keep_game_mode` is `true`, preserves
    /// the current `game_mode` and only randomizes identity and player count.
    pub fn randomize_step1_identity(&mut self, keep_game_mode: bool) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        if !keep_game_mode {
            let modes = WizardGameMode::all();
            if modes.len() > 1 {
                let idx = (rng.next_u64() as usize) % modes.len();
                self.game_mode = modes[idx];
            }
        }

        let prev_name = self.map_name.clone();
        for _ in 0..6 {
            let identity = generate_random_map_identity(self.game_mode, rng.next_u64());
            self.map_name = identity.map_name;
            self.title = identity.title;
            self.enter_text = identity.enter_text;
            if self.map_name != prev_name {
                break;
            }
        }

        let player_counts: &[&str] = match self.game_mode {
            WizardGameMode::Duel1v1 => &["2", "2-4"],
            WizardGameMode::DeathMatch => &["4-8", "4-10", "6-10", "6-12", "8-14"],
            WizardGameMode::TeamDeathMatch => &["6-10", "6-12", "8-12", "8-16"],
            WizardGameMode::CaptureTheFlag => &["6-12", "8-12", "8-14", "8-16", "10-16"],
            WizardGameMode::Domination => &["6-10", "6-12", "8-12", "8-14"],
            WizardGameMode::Assault => &["6-12", "8-14", "8-16", "10-16"],
        };
        let pc_idx = (rng.next_u64() as usize) % player_counts.len();
        self.ideal_player_count = player_counts[pc_idx].to_string();

        self.enforce_mode_invariants();
    }

    /// Randomizes Step 2 (Layout Strategy, Predefined Archetype, Procedural Seed & Graph
    /// Parameters, Symmetry Mode, World Scale, and Room Footprint Shape).
    pub fn randomize_step2_layout(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        self.seed = rng.next_u64();

        // ~68% CanonicalArchetype (highlighting the 18 architectural layouts), ~32% ProceduralSeed
        self.layout_strategy = if rng.range_i32(0, 99) < 68 {
            WizardLayoutStrategy::CanonicalArchetype
        } else {
            WizardLayoutStrategy::ProceduralSeed
        };

        // Pick a fresh archetype different from the current one, weighted by mode affinity
        let all_archetypes = WizardArchetype::all();
        let preferred_pool: Vec<WizardArchetype> = all_archetypes
            .iter()
            .copied()
            .filter(|a| *a != self.archetype)
            .filter(|a| match self.game_mode {
                WizardGameMode::Duel1v1 => matches!(
                    a.category(),
                    WizardArchetypeCategory::ArenaAndDuel
                        | WizardArchetypeCategory::VerticalAndOrbital
                ),
                WizardGameMode::CaptureTheFlag => matches!(
                    a.category(),
                    WizardArchetypeCategory::SymmetricalTeam
                ),
                WizardGameMode::Domination => matches!(
                    a.category(),
                    WizardArchetypeCategory::MultiWingDomination
                        | WizardArchetypeCategory::ArenaAndDuel
                        | WizardArchetypeCategory::VerticalAndOrbital
                ),
                WizardGameMode::Assault => matches!(
                    a.category(),
                    WizardArchetypeCategory::SiegeAndMonument
                ),
                WizardGameMode::DeathMatch | WizardGameMode::TeamDeathMatch => {
                    matches!(
                        a.category(),
                        WizardArchetypeCategory::ArenaAndDuel
                            | WizardArchetypeCategory::VerticalAndOrbital
                    ) || matches!(
                        a,
                        WizardArchetype::InterconnectedComplex
                            | WizardArchetype::FourCornersCourtyard
                            | WizardArchetype::CatacombLabyrinth
                            | WizardArchetype::ZigguratTempleComplex
                    )
                }
            })
            .collect();

        if !preferred_pool.is_empty() {
            let idx = (rng.next_u64() as usize) % preferred_pool.len();
            self.archetype = preferred_pool[idx];
        } else if self.game_mode == WizardGameMode::CaptureTheFlag {
            self.archetype = WizardArchetype::SymmetricalTwinBases;
        } else if self.game_mode == WizardGameMode::Assault {
            self.archetype = WizardArchetype::FortressSiegeLine;
        } else if self.game_mode == WizardGameMode::Domination {
            self.archetype = WizardArchetype::ThreeWayRingArena;
        } else {
            self.archetype = WizardArchetype::InterconnectedComplex;
        }

        // Procedural seed graph parameters
        self.procedural_room_count = match self.game_mode {
            WizardGameMode::Duel1v1 => rng.range_i32(3, 6) as usize,
            WizardGameMode::CaptureTheFlag | WizardGameMode::Assault => {
                rng.range_i32(5, 10) as usize
            }
            _ => rng.range_i32(4, 10) as usize,
        };
        self.procedural_vertical_tiers = rng.range_i32(1, 3) as usize;
        let loop_step = rng.range_i32(9, 19); // 0.45 ..= 0.95 in steps of 0.05
        self.procedural_loop_ratio = (loop_step as f32) * 0.05;

        // Symmetry mode (CTF always uses MirrorX or Rotational180)
        if self.game_mode == WizardGameMode::CaptureTheFlag {
            self.symmetry_mode = if rng.range_i32(0, 1) == 0 {
                WizardSymmetryMode::MirrorX
            } else {
                WizardSymmetryMode::Rotational180
            };
        } else {
            let syms = WizardSymmetryMode::all();
            self.symmetry_mode = syms[(rng.next_u64() as usize) % syms.len()];
        }

        // Map scale weighted by mode
        let scales: &[WizardMapScale] = match self.game_mode {
            WizardGameMode::Duel1v1 => &[
                WizardMapScale::Compact,
                WizardMapScale::Compact,
                WizardMapScale::Standard,
            ],
            WizardGameMode::CaptureTheFlag | WizardGameMode::Assault => &[
                WizardMapScale::Standard,
                WizardMapScale::Large,
                WizardMapScale::Large,
            ],
            _ => &[
                WizardMapScale::Compact,
                WizardMapScale::Standard,
                WizardMapScale::Standard,
                WizardMapScale::Large,
            ],
        };
        self.scale = scales[(rng.next_u64() as usize) % scales.len()];

        // Room shape (prefer different from current)
        let shapes = WizardRoomShape::all();
        if shapes.len() > 1 {
            let mut idx = (rng.next_u64() as usize) % shapes.len();
            if shapes[idx] == self.room_shape {
                idx = (idx + 1) % shapes.len();
            }
            self.room_shape = shapes[idx];
        }

        self.enforce_mode_invariants();
    }

    /// Randomizes Step 3 (Stock Theme Preset), picking a theme different from the current one
    /// and clearing any custom `.unr` DNA palette override.
    pub fn randomize_step3_theme(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        let themes = WizardThemePreset::all();
        if themes.len() > 1 {
            let mut idx = (rng.next_u64() as usize) % themes.len();
            if themes[idx] == self.theme {
                idx = (idx + 1 + (rng.range_i32(0, (themes.len() as i32) - 2) as usize))
                    % themes.len();
            }
            self.theme = themes[idx];
        }
        self.custom_dna_source_map = None;
        self.custom_palette_override = None;
    }

    /// Randomizes Step 4 modular gameplay mechanic toggles while keeping core playability
    /// systems (`include_lighting`, `include_bot_ai_paths`, `include_weapons_and_pickups`) active.
    pub fn randomize_mechanics_toggles(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        self.toggles.include_lighting = true;
        self.toggles.include_bot_ai_paths = true;
        self.toggles.include_weapons_and_pickups = true;

        self.toggles.include_skybox =
            self.archetype == WizardArchetype::OrbitalOutpostSkybox || rng.range_i32(0, 99) < 75;
        self.toggles.include_elevators_and_movers = rng.range_i32(0, 99) < 75;
        self.toggles.include_environmental_hazards = rng.range_i32(0, 99) < 65;
        self.toggles.include_decorations_and_ambience = rng.range_i32(0, 99) < 85;
        self.toggles.include_jump_pads = rng.range_i32(0, 99) < 75;
        self.toggles.include_ramps_and_bridges = rng.range_i32(0, 99) < 85;
        self.toggles.include_teleporters = rng.range_i32(0, 99) < 70;

        // Guarantee at least 2 traversal/verticality mechanics are enabled so multi-tier arenas flow well
        let traversal_count = [
            self.toggles.include_elevators_and_movers,
            self.toggles.include_jump_pads,
            self.toggles.include_ramps_and_bridges,
            self.toggles.include_teleporters,
        ]
        .iter()
        .filter(|&&b| b)
        .count();
        if traversal_count < 2 {
            self.toggles.include_ramps_and_bridges = true;
            self.toggles.include_jump_pads = true;
        }
    }

    /// Randomizes the Armory Weapon Loadout (either selecting a curated preset or generating
    /// a balanced Custom weapon selection of 4..=8 weapons with guaranteed hitscan + explosive coverage).
    pub fn randomize_weapon_loadout(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        self.toggles.include_weapons_and_pickups = true;

        // 35% curated preset, 65% procedural custom combination
        if rng.range_i32(0, 99) < 35 {
            let presets = [
                WizardWeaponPreset::TournamentStandard,
                WizardWeaponPreset::CloseQuarters,
                WizardWeaponPreset::HeavyExplosives,
                WizardWeaponPreset::SniperDuel,
                WizardWeaponPreset::ClassicUnreal,
                WizardWeaponPreset::FullTournament,
            ];
            let mut idx = (rng.next_u64() as usize) % presets.len();
            if presets[idx] == self.weapon_preset {
                idx = (idx + 1) % presets.len();
            }
            self.apply_weapon_preset(presets[idx]);
            return;
        }

        self.weapon_preset = WizardWeaponPreset::Custom;
        let target_count = rng.range_i32(4, 8) as usize;
        let mut chosen: Vec<String> = Vec::with_capacity(target_count);

        // 1. Guarantee at least one primary hitscan / beam weapon
        let hitscan_pool = ["ShockRifle", "SniperRifle", "minigun2", "PulseGun", "Enforcer"];
        let h_pick = hitscan_pool[(rng.next_u64() as usize) % hitscan_pool.len()];
        chosen.push(h_pick.to_string());

        // 2. Guarantee at least one heavy / splash weapon
        let splash_pool = [
            "UT_FlakCannon",
            "UT_Eightball",
            "Ripper",
            "ut_biorifle",
            "WarHeadLauncher",
        ];
        let s_pick = splash_pool[(rng.next_u64() as usize) % splash_pool.len()];
        if !chosen.iter().any(|w| w == s_pick) {
            chosen.push(s_pick.to_string());
        }

        // 3. Fill remaining slots from Stock Tournament Weapons (and occasionally Classic Unreal 1)
        let include_classic = rng.range_i32(0, 99) < 30;
        let mut candidates: Vec<&str> = STOCK_TOURNAMENT_WEAPONS
            .iter()
            .map(|w| w.class_name)
            .collect();
        if include_classic {
            for cw in CLASSIC_UNREAL_WEAPONS {
                candidates.push(cw.class_name);
            }
        }

        // Fisher-Yates shuffle on candidates
        for i in (1..candidates.len()).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            candidates.swap(i, j);
        }

        for cand in candidates {
            if chosen.len() >= target_count {
                break;
            }
            if !chosen.iter().any(|w| w.eq_ignore_ascii_case(cand)) {
                chosen.push(cand.to_string());
            }
        }

        self.custom_weapon_loadout = chosen;
    }

    /// Randomizes the Powerup, Armor & Health Pickup Loadout (either selecting a curated preset
    /// or generating a Custom loadout that always includes baseline Health + Armor + random Relics).
    pub fn randomize_powerup_loadout(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        self.toggles.include_weapons_and_pickups = true;

        // 35% curated preset, 65% procedural custom combination
        if rng.range_i32(0, 99) < 35 {
            let presets = [
                WizardPowerupPreset::BalancedCompetitive,
                WizardPowerupPreset::HighMobility,
                WizardPowerupPreset::MayhemOverkill,
                WizardPowerupPreset::HardcoreTactical,
            ];
            let mut idx = (rng.next_u64() as usize) % presets.len();
            if presets[idx] == self.powerup_preset {
                idx = (idx + 1) % presets.len();
            }
            self.apply_powerup_preset(presets[idx]);
            return;
        }

        self.powerup_preset = WizardPowerupPreset::Custom;
        let target_count = rng.range_i32(4, 8) as usize;
        let mut chosen: Vec<String> = Vec::with_capacity(target_count);

        // 1. Guarantee baseline health ("MedBox" and/or "HealthVial")
        let health_mode = rng.range_i32(0, 2);
        if health_mode == 0 || health_mode == 2 {
            chosen.push("MedBox".to_string());
        }
        if health_mode == 1 || health_mode == 2 {
            chosen.push("HealthVial".to_string());
        }

        // 2. Guarantee baseline armor ("Armor2" or "ThighPads")
        let armor_pick = if rng.range_i32(0, 1) == 0 {
            "Armor2"
        } else {
            "ThighPads"
        };
        chosen.push(armor_pick.to_string());

        // 3. Fill remaining slots from all powerups/pickups
        let mut candidates: Vec<&str> = STOCK_POWERUPS_AND_PICKUPS
            .iter()
            .map(|p| p.class_name)
            .collect();
        for i in (1..candidates.len()).rev() {
            let j = (rng.next_u64() as usize) % (i + 1);
            candidates.swap(i, j);
        }

        for cand in candidates {
            if chosen.len() >= target_count {
                break;
            }
            if !chosen.iter().any(|p| p.eq_ignore_ascii_case(cand)) {
                chosen.push(cand.to_string());
            }
        }

        self.custom_powerup_loadout = chosen;
    }

    /// Randomizes all Step 4 parameters (Mechanics Toggles, Armory Weapon Loadout,
    /// Powerup/Pickup Loadout, and Item Density).
    pub fn randomize_step4_gameplay_and_armory(&mut self) {
        let mut rng = SplitMix64::new(next_wizard_entropy_seed());
        self.randomize_mechanics_toggles();
        self.randomize_weapon_loadout();
        self.randomize_powerup_loadout();
        let densities = WizardItemDensity::all();
        self.item_density = densities[(rng.next_u64() as usize) % densities.len()];
        if self.game_mode == WizardGameMode::Assault {
            self.assault_objective_count = rng.range_i32(2, 5) as usize;
        }
        if self.game_mode == WizardGameMode::Domination {
            self.domination_point_count = match rng.range_i32(0, 10) {
                0..=7 => 3, // 80% standard 3 points triad
                8 => 2,     // 10% 2 points duel
                _ => 4,     // 10% 4 points quad
            };
            let daises = crate::domination_mission::DominationDaisStyle::all();
            self.domination_dais_style = daises[(rng.next_u64() as usize) % daises.len()];
            let namings = crate::domination_mission::DominationNamingStyle::all();
            self.domination_naming_style = namings[(rng.next_u64() as usize) % namings.len()];
            self.domination_include_defense_points = rng.next_f32() < 0.9;
            self.domination_contested_center_powerup = rng.next_f32() < 0.85;
        }
        if self.game_mode == WizardGameMode::CaptureTheFlag {
            let daises = crate::ctf_mission::CtfDaisStyle::all();
            self.ctf_dais_style = daises[(rng.next_u64() as usize) % daises.len()];
            self.ctf_include_sniper_perches = rng.next_f32() < 0.85;
            self.ctf_contested_center_powerup = rng.next_f32() < 0.85;
        }
        self.enforce_mode_invariants();
    }

    /// One-click master randomizer across all customizable steps (Steps 1, 2, 3, and 4).
    /// When `keep_game_mode` is `true`, preserves the currently selected `WizardGameMode`.
    pub fn randomize_all(&mut self, keep_game_mode: bool) {
        self.randomize_step1_identity(keep_game_mode);
        self.randomize_step2_layout();
        self.randomize_step3_theme();
        self.randomize_step4_gameplay_and_armory();
        self.enforce_mode_invariants();
    }
}

/// Descriptor for an animated `Mover` generated by the Map Wizard.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardMoverSpec {
    pub name: String,
    pub brush: Brush,
    pub key0_location: Vec3,
    pub key1_location: Vec3,
    pub initial_state: String,
    pub move_time: f32,
    pub stay_open_time: f32,
    pub tag: String,
    pub event: String,
    pub opening_sound: String,
    pub closed_sound: String,
}

/// Descriptor for a generic UT99 actor generated by the Map Wizard.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardActorSpec {
    pub name: String,
    pub class_name: String,
    pub location: Vec3,
    pub rotation: (i32, i32, i32),
    pub tag: String,
    pub event: String,
    pub group: String,
    pub light_brightness: u8,
    pub light_radius: u8,
    pub light_hue: u8,
    pub light_saturation: u8,
    pub int_props: Vec<(String, i32)>,
    pub byte_props: Vec<(String, u8)>,
    pub float_props: Vec<(String, f32)>,
    pub bool_props: Vec<(String, bool)>,
    pub name_props: Vec<(String, String)>,
    pub str_props: Vec<(String, String)>,
    pub vec_props: Vec<(String, Vec3)>,
}

impl Default for WizardActorSpec {
    fn default() -> Self {
        Self {
            name: String::new(),
            class_name: "Actor".to_string(),
            location: Vec3::ZERO,
            rotation: (0, 0, 0),
            tag: String::new(),
            event: String::new(),
            group: String::new(),
            light_brightness: 128,
            light_radius: 48,
            light_hue: 0,
            light_saturation: 128,
            int_props: Vec::new(),
            byte_props: Vec::new(),
            float_props: Vec::new(),
            bool_props: Vec::new(),
            name_props: Vec::new(),
            str_props: Vec::new(),
            vec_props: Vec::new(),
        }
    }
}

/// 2D Room descriptor for the real-time schematic preview in the Wizard dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardPreviewRoom2D {
    pub label: String,
    pub center: Vec3,
    pub size: Vec3,
    pub is_corridor: bool,
    pub is_hazard: bool,
    pub is_octagonal: bool,
    pub team_tint: Option<u8>, // 0 = Red, 1 = Blue, None = Neutral
    pub environment: WizardRoomEnvironment,
}

/// 2D Marker for key gameplay actors in the real-time schematic preview.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardPreviewMarker2D {
    pub label: String,
    pub kind: &'static str, // "PlayerStart", "FlagBase", "ControlPoint", "FortStandard", "Weapon", "Powerup", "Lift"
    pub location: Vec3,
    pub team: Option<u8>,
}

/// 2D schematic preview and estimated telemetry; room edits may add geometric warnings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WizardTopologyPreview {
    pub rooms: Vec<WizardPreviewRoom2D>,
    pub markers: Vec<WizardPreviewMarker2D>,
    /// Bounding-box contacts lost after room overrides; not a pathfinding verdict.
    pub warnings: Vec<String>,
    /// Logical room graph before room overrides and physical traversal checks.
    pub procedural_graph: Option<WizardProceduralGraphSummary>,
    pub symmetry_axis_x: Option<f32>,
    pub estimated_brushes: usize,
    pub estimated_lights: usize,
    pub estimated_spawns: usize,
    pub estimated_weapons_pickups: usize,
    pub estimated_path_nodes: usize,
    pub estimated_movers: usize,
    pub teleport_audit: WizardTeleportAuditSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WizardProceduralGraphSummary {
    pub room_count: usize,
    pub tree_edges: usize,
    pub random_edges: usize,
    pub minimum_degree_edges: usize,
    pub components: usize,
    pub independent_cycles: usize,
    pub minimum_degree: usize,
    pub effective_extra_probability: f32,
    pub edges: Vec<(String, String)>,
}

/// Complete output of `generate_valid_ut99_map`.
#[derive(Debug, Clone, Default)]
pub struct WizardGeneratedMap {
    pub brushes: Vec<Brush>,
    pub movers: Vec<WizardMoverSpec>,
    pub actors: Vec<WizardActorSpec>,
    /// Generation compromises that require review, not Level Doctor errors.
    pub warnings: Vec<String>,
    pub preview: WizardTopologyPreview,
    pub default_game_type: String,
    pub song: String,
    pub ambient_brightness: u8,
    pub ambient_hue: u8,
    pub ambient_saturation: u8,
}

// ---------------------------------------------------------------------------
// Deterministic SplitMix64 PRNG (Zero External Dependencies)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1u32 << 24) as f32)
    }

    fn range_i32(&mut self, min_inclusive: i32, max_inclusive: i32) -> i32 {
        if max_inclusive <= min_inclusive {
            return min_inclusive;
        }
        let span = (max_inclusive - min_inclusive + 1) as u64;
        min_inclusive + (self.next_u64() % span) as i32
    }
}

#[inline]
fn snap_grid(v: f32, step: f32) -> f32 {
    (v / step).round() * step
}

fn apply_room_textures(
    brush: &mut Brush,
    pal: &WizardResolvedPalette,
    sky_ceiling: bool,
    is_corridor: bool,
) {
    for poly in &mut brush.polys {
        let nz = poly.plane.normal.z;
        if brush.csg_oper == CsgOperation::Subtract {
            // In build_cube / build_aligned_octagon, top face has +Z and bottom face has -Z
            if nz < -0.65 {
                if is_corridor && !pal.corridor_floor_texture.is_empty() {
                    poly.texture = pal.corridor_floor_texture.clone();
                } else {
                    poly.texture = pal.floor_texture.clone();
                }
            }
 else if nz > 0.65 {
                if sky_ceiling {
                    poly.texture = pal.sky_texture.clone();
                    poly.flags |= PF_FAKE_BACKDROP;
                } else {
                    poly.texture = pal.ceiling_texture.clone();
                }
            }
 else {
                let is_diagonal_chamfer =
                    poly.plane.normal.x.abs() > 0.3 && poly.plane.normal.y.abs() > 0.3;
                if (is_corridor || is_diagonal_chamfer) && !pal.secondary_wall_texture.is_empty() {
                    poly.texture = pal.secondary_wall_texture.clone();
                } else {
                    poly.texture = pal.wall_texture.clone();
                }
            }

        } else if nz > 0.65 {
            poly.texture = pal.floor_texture.clone();
        } else if nz < -0.65 {
            poly.texture = pal.ceiling_texture.clone();
        } else {
            poly.texture = pal.trim_texture.clone();
        }
    }
    brush.recompute_uvs();
}

/// Internal intermediate room node before CSG materialization.
#[derive(Debug, Clone)]
struct RoomNode {
    name: String,
    center: Vec3,
    size: Vec3,
    is_corridor: bool,
    sky_ceiling: bool,
    has_hazard_pit: bool,
    team_owner: Option<u8>,
    is_octagonal: bool,
}

impl RoomNode {
    fn floor_z(&self) -> f32 {
        self.center.z - self.size.z * 0.5
    }

    fn ceiling_z(&self) -> f32 {
        self.center.z + self.size.z * 0.5
    }

    fn min_x(&self) -> f32 {
        self.center.x - self.size.x * 0.5
    }

    fn max_x(&self) -> f32 {
        self.center.x + self.size.x * 0.5
    }

    fn min_y(&self) -> f32 {
        self.center.y - self.size.y * 0.5
    }

    fn max_y(&self) -> f32 {
        self.center.y + self.size.y * 0.5
    }
}

#[allow(dead_code)]
fn boxes_overlap_xy(a: &RoomNode, b: &RoomNode) -> bool {
    a.min_x() < b.max_x() && a.max_x() > b.min_x() && a.min_y() < b.max_y() && a.max_y() > b.min_y()
}

fn room_boxes_overlap(a: &RoomNode, b: &RoomNode) -> bool {
    // Require a small positive overlap in all axes, not just touching faces.
    let overlap = (a.size + b.size) * 0.5 - (a.center - b.center).abs();
    overlap.x > 4.0 && overlap.y > 4.0 && overlap.z > 4.0
}

fn override_contact_warnings(cfg: &MapWizardConfig, current: &[RoomNode], graph: Option<&WizardProceduralGraphSummary>) -> Vec<String> {
    if cfg.room_overrides.is_empty() { return Vec::new(); }
    let mut baseline_cfg = cfg.clone();
    baseline_cfg.room_overrides.clear();
    let baseline = build_room_nodes(&baseline_cfg);
    let edited: std::collections::HashSet<&str> = cfg.room_overrides.iter().map(|edit| edit.room_name.as_str()).collect();
    let mut warnings = Vec::new();
    for (i, a) in baseline.iter().enumerate() {
        for b in baseline.iter().skip(i + 1) {
            if a.name == b.name || !(edited.contains(a.name.as_str()) || edited.contains(b.name.as_str()))
                || !room_boxes_overlap(a, b) {
                continue;
            }
            let mut aa = current.iter().filter(|r| r.name == a.name);
            let mut bb = current.iter().filter(|r| r.name == b.name);
            let (Some(new_a), Some(new_b)) = (aa.next(), bb.next()) else { continue; };
            if aa.next().is_some() || bb.next().is_some() { continue; }
            if !room_boxes_overlap(new_a, new_b) {
                warnings.push(format!("{} and {} no longer overlap after room edits; check the connection and traversal (bounding-box estimate).", a.name, b.name));
            }
        }
    }
    if let Some(graph) = graph {
        let unique = |nodes: &[RoomNode], name: &str| {
            let mut matches = nodes.iter().filter(|node| !node.is_corridor && node.name == name);
            let first = matches.next().cloned();
            if matches.next().is_some() { None } else { first }
        };
        let changed: std::collections::HashSet<&str> = edited.iter().copied().filter(|name| {
            match (unique(&baseline, name), unique(current, name)) {
                (Some(old), Some(new)) => old.center != new.center || old.size != new.size,
                _ => false,
            }
        }).collect();
        if graph.edges.iter().any(|(a, b)| changed.contains(a.as_str()) || changed.contains(b.as_str())) {
            warnings.push("Procedural corridors were rebuilt from edited room endpoints; verify intersections, clearance and traversal in the engine.".into());
            if cfg.game_mode == WizardGameMode::CaptureTheFlag {
                warnings.push("Edited CTF rooms may alter team symmetry and balance; review both sides.".into());
            }
            if cfg.room_overrides.iter().any(|edit| edit.room_name.starts_with("ProcCorridor")) {
                warnings.push("Explicit procedural corridor overrides were retained; verify their alignment with the edited rooms.".into());
            }
            for (from, to) in &graph.edges {
                if !changed.contains(from.as_str()) && !changed.contains(to.as_str()) { continue; }
                if let (Some(a), Some(b)) = (unique(current, from), unique(current, to)) {
                    let height_difference = (a.floor_z() - b.floor_z()).abs();
                    if height_difference > 54.0 {
                        warnings.push(format!("Edited connection {} ↔ {} has {:.0} UU floor difference; verify the vertical transition.", from, to, height_difference));
                    }
                }
            }
        }
    }
    warnings
}

fn topology_reference_warnings(cfg: &MapWizardConfig, rooms: &[RoomNode]) -> Vec<String> {
    let mut warnings = Vec::new();
    for edit in &cfg.room_overrides {
        let count = rooms.iter().filter(|room| room.name == edit.room_name).count();
        if count != 1 {
            warnings.push(format!("Room edit '{}' matches {} rooms; check the room name before generating.", edit.room_name, count));
        }
    }
    for connection in &cfg.custom_connections {
        if connection.from_room == connection.to_room {
            warnings.push(format!("Connection '{}' has the same room at both endpoints.", connection.id));
        }
        for endpoint in [&connection.from_room, &connection.to_room] {
            let count = rooms.iter().filter(|room| room.name == *endpoint).count();
            if count != 1 {
                warnings.push(format!("Connection '{}' endpoint '{}' matches {} rooms; check the connection.", connection.id, endpoint, count));
            }
        }
    }
    warnings
}

/// Role of a procedural space in the height library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightRole {
    Hub,
    TeamBase,
    Room,
    Corridor,
}

/// Discrete ceiling heights per role with integer weights. The mix is bimodal like the stock
/// maps: a large share of compressed spaces (corridors 192-256, rooms from 256) plus tall halls. Team bases
/// and the hub never go below 384 / 512 because flags, jump pads and lifts assume that clearance.
fn height_library(role: HeightRole) -> &'static [(f32, u32)] {
    match role {
        HeightRole::Hub => &[(512.0, 2), (640.0, 3), (768.0, 2)],
        HeightRole::TeamBase => &[(384.0, 2), (448.0, 2), (512.0, 3)],
        HeightRole::Room => &[(256.0, 3), (320.0, 3), (384.0, 2), (448.0, 1), (512.0, 2), (640.0, 1)],
        HeightRole::Corridor => &[(192.0, 3), (224.0, 3), (256.0, 3), (320.0, 1)],
    }
}

/// Deterministic weighted pick. Uses its own hash stream (seed, salt, index) so that choosing heights never
/// shifts the random draws that place the rooms: layouts stay identical across rules versions.
fn pick_library_height(seed: u64, salt: u64, index: usize, role: HeightRole) -> f32 {
    let mut rng = SplitMix64::new(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (index as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    let _ = rng.next_u64();
    let table = height_library(role);
    let total: u32 = table.iter().map(|(_, w)| *w).sum();
    let mut roll = (rng.next_u64() % total as u64) as u32;
    for (h, w) in table {
        if roll < *w {
            return *h;
        }
        roll -= *w;
    }
    table[0].0
}

/// Procedural room height for the given rules version: 512 (hub 640) before rules v3, library pick after.
fn procedural_room_height(cfg: &MapWizardConfig, index: usize, role: HeightRole) -> f32 {
    if cfg.generator_version >= GENERATOR_VERSION_HEIGHTS {
        pick_library_height(cfg.seed, 0x52_4F4F_4D, index, role)
    } else if role == HeightRole::Hub {
        640.0
    } else {
        512.0
    }
}

/// Minimum clear height above the top step of a stair or ramp (UT99 player collision height is 78 UU).
const MIN_STAIR_HEADROOM: f32 = 112.0;

/// Overlap a clipped corridor keeps inside the room it leaves, so the two carved volumes never
/// merely touch along a face.
const CORRIDOR_PORTAL_OVERLAP: f32 = 8.0;
/// Shortest corridor a clip may leave; transit stairs and frames assume at least ~2 jamb spacings.
const MIN_CLIPPED_CORRIDOR_LEN: f32 = 192.0;
/// A doorway frame deeper than this inside a room's air volume is not on any wall.
const FRAME_ROOM_INTERIOR_MARGIN: f32 = 12.0;

/// Clips a corridor span `[lo, hi]` along one axis to the stretch between the walls of the rooms its
/// ends sit in. `lo_room` / `hi_room` are the room extents `(min, max)` along the same
/// axis when the corresponding end lies inside a room, `None` otherwise. Spans that would collapse
/// (room walls beyond the other end) are returned unchanged.
fn clip_corridor_span(lo: f32, hi: f32, lo_room: Option<(f32, f32)>, hi_room: Option<(f32, f32)>) -> (f32, f32) {
    let mut new_lo = lo;
    let mut new_hi = hi;
    if let Some((_, room_max)) = lo_room {
        if room_max > lo && room_max < hi {
            new_lo = room_max - CORRIDOR_PORTAL_OVERLAP;
        }
    }
    if let Some((room_min, _)) = hi_room {
        if room_min < hi && room_min > new_lo {
            new_hi = room_min + CORRIDOR_PORTAL_OVERLAP;
        }
    }
    if new_hi - new_lo < MIN_CLIPPED_CORRIDOR_LEN { (lo, hi) } else { (new_lo, new_hi) }
}

/// True if `point` lies inside `room`'s footprint and z-range, deeper than `margin` from every wall.
fn point_deep_inside_room(point: Vec3, room: &RoomNode, margin: f32) -> bool {
    point.x > room.min_x() + margin && point.x < room.max_x() - margin
        && point.y > room.min_y() + margin && point.y < room.max_y() - margin
        && point.z > room.floor_z() + margin && point.z < room.ceiling_z() - margin
}

fn procedural_corridor_nodes(index: usize, a: &RoomNode, b: &RoomNode, cfg: &MapWizardConfig) -> Vec<RoomNode> {
    let portal_aligned = cfg.generator_version >= GENERATOR_VERSION_PORTALS;
    let width = 288.0;
    let floor = a.floor_z().max(b.floor_z());
    let height = if cfg.generator_version >= GENERATOR_VERSION_HEIGHTS {
        // Library pick, never taller than the lower of the two room ceilings above the corridor floor
        // (no slot cut into a ceiling), never below 192 UU.
        let cap = (a.ceiling_z().min(b.ceiling_z()) - floor).max(192.0);
        pick_library_height(cfg.seed, 0x43_4F52, index, HeightRole::Corridor).min(cap).max(192.0)
    } else {
        320.0
    };
    let z = floor + height * 0.5;
    let dx = (b.center.x - a.center.x).abs();
    let dy = (b.center.y - a.center.y).abs();
    let mut corridors = Vec::new();
    if dx > 64.0 {
        // The X leg runs along y = a.center.y from a's centre to the corner at x = b.center.x.
        let (lo, hi) = (a.center.x.min(b.center.x) - width * 0.5, a.center.x.max(b.center.x) + width * 0.5);
        let (lo, hi) = if portal_aligned {
            let (lo_room, hi_room) = if a.center.x <= b.center.x { (a, b) } else { (b, a) };
            let inside = |r: &RoomNode, x: f32| (r.min_y()..=r.max_y()).contains(&a.center.y) && (r.min_x()..=r.max_x()).contains(&x);
            clip_corridor_span(
                lo, hi,
                inside(lo_room, lo_room.center.x).then_some((lo_room.min_x(), lo_room.max_x())),
                inside(hi_room, hi_room.center.x).then_some((hi_room.min_x(), hi_room.max_x())),
            )
        } else { (lo, hi) };
        corridors.push(RoomNode {
            name: format!("ProcCorridorX_{index}"),
            center: Vec3::new(if portal_aligned { (lo + hi) * 0.5 } else { (a.center.x + b.center.x) * 0.5 }, a.center.y, z),
            size: Vec3::new(if portal_aligned { hi - lo } else { dx + width }, width, height),
            is_corridor: true, sky_ceiling: false, has_hazard_pit: false,
            team_owner: None, is_octagonal: false,
        });
    }
    if dy > 64.0 {
        // The Y leg runs along x = b.center.x from the corner at y = a.center.y to b's centre.
        let (lo, hi) = (a.center.y.min(b.center.y) - width * 0.5, a.center.y.max(b.center.y) + width * 0.5);
        let (lo, hi) = if portal_aligned {
            let a_is_lo = a.center.y <= b.center.y;
            let (lo_room, hi_room) = if a_is_lo { (a, b) } else { (b, a) };
            // The corner (b.center.x, a.center.y) belongs to a only if a's footprint covers it.
            let covers = |r: &RoomNode, y: f32| (r.min_x()..=r.max_x()).contains(&b.center.x) && (r.min_y()..=r.max_y()).contains(&y);
            clip_corridor_span(
                lo, hi,
                covers(lo_room, if a_is_lo { a.center.y } else { b.center.y }).then_some((lo_room.min_y(), lo_room.max_y())),
                covers(hi_room, if a_is_lo { b.center.y } else { a.center.y }).then_some((hi_room.min_y(), hi_room.max_y())),
            )
        } else { (lo, hi) };
        corridors.push(RoomNode {
            name: format!("ProcCorridorY_{index}"),
            center: Vec3::new(b.center.x, if portal_aligned { (lo + hi) * 0.5 } else { (a.center.y + b.center.y) * 0.5 }, z),
            size: Vec3::new(width, if portal_aligned { hi - lo } else { dy + width }, height),
            is_corridor: true, sky_ceiling: false, has_hazard_pit: false,
            team_owner: None, is_octagonal: false,
        });
    }
    corridors
}

fn rebuild_edited_procedural_corridors(
    cfg: &MapWizardConfig,
    rooms: &mut Vec<RoomNode>,
    original: &[RoomNode],
    graph: &WizardProceduralGraphSummary,
) {
    let unique_room = |nodes: &[RoomNode], name: &str| {
        let mut found = nodes.iter().filter(|room| !room.is_corridor && room.name == name);
        let first = found.next().cloned();
        if found.next().is_some() { None } else { first }
    };
    let mut replacements: Vec<(String, Option<RoomNode>, bool)> = Vec::new();
    for (index, (from, to)) in graph.edges.iter().enumerate() {
        let (Some(old_a), Some(old_b), Some(a), Some(b)) = (
            unique_room(original, from), unique_room(original, to),
            unique_room(rooms, from), unique_room(rooms, to),
        ) else { continue; };
        if old_a.center == a.center && old_a.size == a.size && old_b.center == b.center && old_b.size == b.size {
            continue;
        }
        let new_segments = procedural_corridor_nodes(index, &a, &b, cfg);
        for axis in ['X', 'Y'] {
            let name = format!("ProcCorridor{axis}_{index}");
            if cfg.room_overrides.iter().any(|edit| edit.room_name == name)
                && original.iter().any(|room| room.is_corridor && room.name == name) { continue; }
            let replacement = new_segments.iter().find(|segment| segment.name == name).cloned();
            replacements.push((name, replacement, false));
        }
    }
    if replacements.is_empty() { return; }
    let mut updated = Vec::with_capacity(rooms.len());
    for room in rooms.drain(..) {
        if let Some((_, replacement, seen)) = replacements.iter_mut().find(|(name, _, _)| room.is_corridor && name == &room.name) {
            *seen = true;
            if let Some(segment) = replacement.take() { updated.push(segment); }
        } else {
            updated.push(room);
        }
    }
    for (_, replacement, seen) in replacements {
        if !seen {
            if let Some(segment) = replacement { updated.push(segment); }
        }
    }
    *rooms = updated;
}

/// Builds the room graph and enforces the **single walkable base level** rule: every room and corridor rests
/// on the same base floor (`z = 0`), keeping its own height, while applying `WizardRoomShape` octagonal/chamfered
/// profiles. Multi-tier vertical play is built inside the rooms via grounded ramps, decks, lifts, jump pads,
/// and balconies.
fn build_room_nodes(cfg: &MapWizardConfig) -> Vec<RoomNode> {
    build_room_nodes_with_graph(cfg, false).0
}

fn build_room_nodes_with_graph(cfg: &MapWizardConfig, include_graph: bool) -> (Vec<RoomNode>, Option<WizardProceduralGraphSummary>) {
    let needs_graph = include_graph || (cfg.layout_strategy == WizardLayoutStrategy::ProceduralSeed && !cfg.room_overrides.is_empty());
    let (mut rooms, graph) = build_room_nodes_raw(cfg, needs_graph);
    let original_rooms = if needs_graph && !cfg.room_overrides.is_empty() { rooms.clone() } else { Vec::new() };

    // Merge custom rooms placed via interactive 2D minimap
    for cr in &cfg.custom_rooms {
        rooms.push(RoomNode {
            name: cr.name.clone(),
            center: cr.center,
            size: cr.size,
            is_corridor: false,
            sky_ceiling: cr.sky_ceiling,
            has_hazard_pit: cr.has_hazard_pit,
            team_owner: cr.team_owner,
            is_octagonal: cr.is_octagonal,
        });
    }

    // Apply transformations before deriving custom connection geometry so that
    // corridors follow their current endpoints.
    for ro in &cfg.room_overrides {
        if let Some(r) = rooms.iter_mut().find(|r| r.name == ro.room_name) {
            r.center = ro.center_offset;
            if ro.size.x >= 128.0 && ro.size.y >= 128.0 && ro.size.z >= 128.0 {
                r.size = ro.size;
            }
        }
    }

    if let Some(ref graph) = graph {
        if !original_rooms.is_empty() {
            rebuild_edited_procedural_corridors(cfg, &mut rooms, &original_rooms, graph);
        }
    }

    // Merge custom connection corridors placed via interactive 2D minimap
    for conn in &cfg.custom_connections {
        let room_a = rooms.iter().find(|r| r.name == conn.from_room).cloned();
        let room_b = rooms.iter().find(|r| r.name == conn.to_room).cloned();
        if let (Some(a), Some(b)) = (room_a, room_b) {
            let corr_w = conn.width.clamp(128.0, 512.0);
            let corr_h = conn.height.clamp(192.0, 512.0);
            let z_floor = a.floor_z().max(b.floor_z());
            let z_c = z_floor + corr_h * 0.5;

            let dx = (b.center.x - a.center.x).abs();
            let dy = (b.center.y - a.center.y).abs();

            if dx > 64.0 {
                rooms.push(RoomNode {
                    name: format!("CustomCorrX_{}", conn.id),
                    center: Vec3::new((a.center.x + b.center.x) * 0.5, a.center.y, z_c),
                    size: Vec3::new(dx + corr_w, corr_w, corr_h),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            if dy > 64.0 {
                rooms.push(RoomNode {
                    name: format!("CustomCorrY_{}", conn.id),
                    center: Vec3::new(b.center.x, (a.center.y + b.center.y) * 0.5, z_c),
                    size: Vec3::new(corr_w, dy + corr_w, corr_h),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
        }
    }

    // A direct override of a custom corridor is still respected after creation.
    for ro in &cfg.room_overrides {
        if let Some(r) = rooms.iter_mut().find(|r| r.is_corridor && r.name == ro.room_name) {
            r.center = ro.center_offset;
            if ro.size.x >= 128.0 && ro.size.y >= 128.0 && ro.size.z >= 128.0 {
                r.size = ro.size;
            }
        }
    }

    let mut main_idx = 0usize;
    for r in &mut rooms {
        let is_overridden = cfg.room_overrides.iter().any(|ro| ro.room_name == r.name);
        if !is_overridden {
            // Enforce base floor (z = 0) ONLY for rooms intended to be grounded.
            // If an archetype or custom room intentionally sets an elevated floor (floor_z > 16 UU),
            // preserve its elevated floor level so vertical tiers/spires are never crushed.
            let current_floor = r.floor_z();
            if current_floor.abs() <= 16.0 {
                r.center.z = r.size.z * 0.5;
            } else {
                let snapped_floor = (current_floor / 16.0).round() * 16.0;
                r.center.z = snapped_floor + r.size.z * 0.5;
            }
        }

        if r.is_corridor {
            r.is_octagonal = false;
        } else {
            let can_octagon = r.size.x >= 640.0 && r.size.y >= 640.0;
            r.is_octagonal = match cfg.room_shape {
                WizardRoomShape::Rectangular => {
                    // Preserve native octagonal hubs/spires declared by the archetype
                    can_octagon && r.is_octagonal
                }
                WizardRoomShape::Octagonal | WizardRoomShape::Chamfered => can_octagon,
                WizardRoomShape::Mixed => can_octagon && (r.is_octagonal || main_idx % 2 == 0),
            };
            main_idx += 1;
        }
    }
    (rooms, graph)
}

fn build_room_nodes_raw(cfg: &MapWizardConfig, include_graph: bool) -> (Vec<RoomNode>, Option<WizardProceduralGraphSummary>) {
    let s = snap_grid(cfg.scale.factor() * 1024.0, 256.0).max(768.0);
    let mut rooms = Vec::new();
    let mut procedural_graph = None;

    match cfg.layout_strategy {
        WizardLayoutStrategy::CanonicalArchetype => match cfg.archetype {
            WizardArchetype::CentralArenaMultiTier => {
                // Main central arena + 4 cardinal alcoves + upper perimeter ring
                let main_w = snap_grid(s * 1.5, 256.0);
                let main_h = 768.0;
                rooms.push(RoomNode {
                    name: "ArenaCore".to_string(),
                    center: Vec3::new(0.0, 0.0, main_h * 0.5),
                    size: Vec3::new(main_w, main_w, main_h),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                let alcove_dist = main_w * 0.5 + 256.0;
                for (i, (dx, dy, team)) in [
                    (-alcove_dist, 0.0, Some(0u8)),
                    (alcove_dist, 0.0, Some(1u8)),
                    (0.0, -alcove_dist, None),
                    (0.0, alcove_dist, None),
                ]
                .into_iter()
                .enumerate()
                {
                    let is_ew = dx.abs() > 0.0;
                    rooms.push(RoomNode {
                        name: format!("ArenaAlcove_{i}"),
                        center: Vec3::new(dx, dy, 192.0),
                        size: if is_ew {
                            Vec3::new(640.0, 512.0, 384.0)
                        } else {
                            Vec3::new(512.0, 640.0, 384.0)
                        },
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::InterconnectedComplex => {
                // Deck16 / Liandri style: Central Reactor Hall + West Chamber + East Chamber + North Sniper Loft + Connecting Corridors
                let hub_w = snap_grid(s * 1.25, 256.0);
                rooms.push(RoomNode {
                    name: "CentralVatHall".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(hub_w, hub_w, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                let wing_offset = hub_w * 0.5 + 640.0;
                // West Wing (Red / Alpha)
                rooms.push(RoomNode {
                    name: "WestTurbineHall".to_string(),
                    center: Vec3::new(-wing_offset, 0.0, 256.0),
                    size: Vec3::new(768.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Corridor Hub <-> West
                rooms.push(RoomNode {
                    name: "WestLinkCorridor".to_string(),
                    center: Vec3::new(-wing_offset * 0.5, 0.0, 192.0),
                    size: Vec3::new(wing_offset, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // East Wing (Blue / Beta)
                rooms.push(RoomNode {
                    name: "EastGeneratorHall".to_string(),
                    center: Vec3::new(wing_offset, 0.0, 256.0),
                    size: Vec3::new(768.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                // Corridor Hub <-> East
                rooms.push(RoomNode {
                    name: "EastLinkCorridor".to_string(),
                    center: Vec3::new(wing_offset * 0.5, 0.0, 192.0),
                    size: Vec3::new(wing_offset, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // North Upper Command Deck (Gamma / Final Objective)
                rooms.push(RoomNode {
                    name: "NorthCommandDeck".to_string(),
                    center: Vec3::new(0.0, wing_offset, 384.0),
                    size: Vec3::new(896.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Corridor Hub <-> North
                rooms.push(RoomNode {
                    name: "NorthMainCorridor".to_string(),
                    center: Vec3::new(0.0, wing_offset * 0.5, 256.0),
                    size: Vec3::new(384.0, wing_offset, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Diagonal / Ring Flank Corridors so there are zero dead-ends
                rooms.push(RoomNode {
                    name: "NorthWestRingCorridor".to_string(),
                    center: Vec3::new(-wing_offset, wing_offset * 0.5, 256.0),
                    size: Vec3::new(320.0, wing_offset + 256.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthWestCrossCorridor".to_string(),
                    center: Vec3::new(-wing_offset * 0.675, wing_offset, 256.0),
                    size: Vec3::new(wing_offset * 0.75, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthEastRingCorridor".to_string(),
                    center: Vec3::new(wing_offset, wing_offset * 0.5, 256.0),
                    size: Vec3::new(320.0, wing_offset + 256.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthEastCrossCorridor".to_string(),
                    center: Vec3::new(wing_offset * 0.675, wing_offset, 256.0),
                    size: Vec3::new(wing_offset * 0.75, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::SymmetricalTwinBases => {
                // Facing Worlds / Coret style: Red Base (-X), Blue Base (+X), Central Midfield Nexus (0),
                // Main Bridge Corridor (Y=0) + North Flank Tunnel (+Y) + South Flank Tunnel (-Y)
                let base_dist = snap_grid(s * 1.5, 256.0);
                // Central Midfield
                rooms.push(RoomNode {
                    name: "MidfieldNexus".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1280.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Red Base (-X, Team 0)
                rooms.push(RoomNode {
                    name: "RedTeamCitadel".to_string(),
                    center: Vec3::new(-base_dist, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Blue Base (+X, Team 1)
                rooms.push(RoomNode {
                    name: "BlueTeamCitadel".to_string(),
                    center: Vec3::new(base_dist, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                // Central High Span Corridor
                rooms.push(RoomNode {
                    name: "CenterBridgeSpan".to_string(),
                    center: Vec3::new(0.0, 0.0, 256.0),
                    size: Vec3::new(base_dist * 2.0, 384.0, 512.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // North & South Flanking Routes (AlternatePaths for CTF bots!)
                for (label, y_off) in [("NorthFlankTunnel", 384.0f32), ("SouthFlankTunnel", -384.0f32)] {
                    rooms.push(RoomNode {
                        name: label.to_string(),
                        center: Vec3::new(0.0, y_off, 192.0),
                        size: Vec3::new(base_dist * 2.0, 256.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::OrbitalOutpostSkybox => {
                // Phobos / SpaceNoxx style: Panoramic Starlight Atrium + 4 Observatory Decks
                let span = snap_grid(s * 1.25, 256.0);
                rooms.push(RoomNode {
                    name: "OrbitalAtrium".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(span, span, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                let pod_off = span * 0.5 + 512.0;
                for (i, (dx, dy, team)) in [
                    (-pod_off, 0.0, Some(0u8)),
                    (pod_off, 0.0, Some(1u8)),
                    (0.0, pod_off, None),
                    (0.0, -pod_off, None),
                ]
                .into_iter()
                .enumerate()
                {
                    rooms.push(RoomNode {
                        name: format!("ObservatoryDeck_{i}"),
                        center: Vec3::new(dx, dy, 256.0),
                        size: Vec3::new(768.0, 768.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: true,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                    rooms.push(RoomNode {
                        name: format!("AirlockConduit_{i}"),
                        center: Vec3::new(dx * 0.5, dy * 0.5, 192.0),
                        size: if dx.abs() > 0.0 {
                            Vec3::new(pod_off, 320.0, 384.0)
                        } else {
                            Vec3::new(320.0, pod_off, 384.0)
                        },
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::VerticalTowerShaft => {
                // Morbias][ / Fractal style: Tall Octagonal Reactor Atrium + 4 Cardinal Bays + Outer Ring Loop
                let shaft_w = snap_grid(s * 1.25, 256.0);
                let shaft_h = 1024.0;
                rooms.push(RoomNode {
                    name: "FractalReactorShaft".to_string(),
                    center: Vec3::new(0.0, 0.0, shaft_h * 0.5),
                    size: Vec3::new(shaft_w, shaft_w, shaft_h),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                let bay_dist = shaft_w * 0.5 + 512.0;
                for (i, (dx, dy, team)) in [
                    (-bay_dist, 0.0, Some(0u8)),
                    (bay_dist, 0.0, Some(1u8)),
                    (0.0, bay_dist, None),
                    (0.0, -bay_dist, None),
                ]
                .into_iter()
                .enumerate()
                {
                    rooms.push(RoomNode {
                        name: format!("TowerBay_{i}"),
                        center: Vec3::new(dx, dy, 256.0),
                        size: Vec3::new(640.0, 640.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                    rooms.push(RoomNode {
                        name: format!("ShaftSpoke_{i}"),
                        center: Vec3::new(dx * 0.5, dy * 0.5, 192.0),
                        size: if dx.abs() > 0.0 {
                            Vec3::new(bay_dist, 320.0, 384.0)
                        } else {
                            Vec3::new(320.0, bay_dist, 384.0)
                        },
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::ThreeWayRingArena => {
                // DOM-Sesmar / Leadworks style: Central Rotunda + 3 Control Sanctuaries (Alpha/Beta/Gamma) + Outer Ring
                let rot_w = snap_grid(s * 1.25, 256.0);
                rooms.push(RoomNode {
                    name: "CentralRotunda".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(rot_w, rot_w, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                let wing_dist = rot_w * 0.5 + 640.0;
                // Alpha West Sanctuary
                rooms.push(RoomNode {
                    name: "AlphaWestSanctuary".to_string(),
                    center: Vec3::new(-wing_dist, 0.0, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Beta East Sanctuary
                rooms.push(RoomNode {
                    name: "BetaEastSanctuary".to_string(),
                    center: Vec3::new(wing_dist, 0.0, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                // Gamma South Sanctuary
                rooms.push(RoomNode {
                    name: "GammaSouthSanctuary".to_string(),
                    center: Vec3::new(0.0, -wing_dist, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Radial corridors to Rotunda
                rooms.push(RoomNode {
                    name: "AlphaRadialHall".to_string(),
                    center: Vec3::new(-wing_dist * 0.5, 0.0, 192.0),
                    size: Vec3::new(wing_dist, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BetaRadialHall".to_string(),
                    center: Vec3::new(wing_dist * 0.5, 0.0, 192.0),
                    size: Vec3::new(wing_dist, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "GammaRadialHall".to_string(),
                    center: Vec3::new(0.0, -wing_dist * 0.5, 192.0),
                    size: Vec3::new(320.0, wing_dist, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Southern outer ring corridors directly connecting Alpha <-> Gamma <-> Beta
                rooms.push(RoomNode {
                    name: "SouthWestRingCorridor".to_string(),
                    center: Vec3::new(-wing_dist, -wing_dist * 0.5, 192.0),
                    size: Vec3::new(320.0, wing_dist + 256.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthWestCrossCorridor".to_string(),
                    center: Vec3::new(-wing_dist * 0.5, -wing_dist, 192.0),
                    size: Vec3::new(wing_dist + 256.0, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthEastRingCorridor".to_string(),
                    center: Vec3::new(wing_dist, -wing_dist * 0.5, 192.0),
                    size: Vec3::new(320.0, wing_dist + 256.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthEastCrossCorridor".to_string(),
                    center: Vec3::new(wing_dist * 0.5, -wing_dist, 192.0),
                    size: Vec3::new(wing_dist + 256.0, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::FortressSiegeLine => {
                // AS-Frigate / AS-Guardia style: Progressive Fortress Breach Line
                // (OuterGatehouse Courtyard -> Beachhead Landing -> Inner Security Core -> Command Citadel)
                let step_y = snap_grid(s * 1.25, 256.0).max(1280.0);
                // 0: Central Outer Gatehouse Courtyard
                rooms.push(RoomNode {
                    name: "OuterGatehouseCourtyard".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                // 1: South Attacker Beachhead Landing
                rooms.push(RoomNode {
                    name: "BeachheadLanding".to_string(),
                    center: Vec3::new(0.0, -step_y, 320.0),
                    size: Vec3::new(1024.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // 2: North-West Inner Security Core
                rooms.push(RoomNode {
                    name: "InnerSecurityCore".to_string(),
                    center: Vec3::new(-768.0, step_y, 320.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // 3: North-East Command Citadel
                rooms.push(RoomNode {
                    name: "CommandCitadel".to_string(),
                    center: Vec3::new(768.0, step_y, 384.0),
                    size: Vec3::new(896.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // Breach corridors connecting South Beachhead -> Courtyard -> North Core & Citadel
                rooms.push(RoomNode {
                    name: "MainBreachTunnel".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.5, 192.0),
                    size: Vec3::new(384.0, step_y, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthBreachSpine".to_string(),
                    center: Vec3::new(0.0, step_y * 0.5, 192.0),
                    size: Vec3::new(384.0, step_y + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "CitadelCrossBridge".to_string(),
                    center: Vec3::new(0.0, step_y, 192.0),
                    size: Vec3::new(1536.0, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::CrucibleDuelPit => {
                // DM-Grit / 1on1 Crucible: Octagonal central pit + West/East Duel Chambers + North/South Weapon Shrines + Outer Bypass Loop
                let core_w = snap_grid(s * 1.25, 256.0).max(1024.0);
                rooms.push(RoomNode {
                    name: "CrucibleCore".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(core_w, core_w, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });
                let dist = core_w * 0.5 + 512.0;
                rooms.push(RoomNode {
                    name: "WestDuelChamber".to_string(),
                    center: Vec3::new(-dist, 0.0, 256.0),
                    size: Vec3::new(640.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastDuelChamber".to_string(),
                    center: Vec3::new(dist, 0.0, 256.0),
                    size: Vec3::new(640.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthWeaponShrine".to_string(),
                    center: Vec3::new(0.0, dist, 256.0),
                    size: Vec3::new(768.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthWeaponShrine".to_string(),
                    center: Vec3::new(0.0, -dist, 256.0),
                    size: Vec3::new(768.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Cardinal throats + Outer Bypass Ring
                rooms.push(RoomNode {
                    name: "CrucibleThroatEW".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new(dist * 2.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "CrucibleThroatNS".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new(352.0, dist * 2.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, cx, cy, sx, sy) in [
                    ("CrucibleBypassN", 0.0, dist, dist * 2.0 + 256.0, 288.0),
                    ("CrucibleBypassS", 0.0, -dist, dist * 2.0 + 256.0, 288.0),
                    ("CrucibleBypassW", -dist, 0.0, 288.0, dist * 2.0 + 256.0),
                    ("CrucibleBypassE", dist, 0.0, 288.0, dist * 2.0 + 256.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::FigureEightCrossover => {
                // DM-Tempest / Agony Figure-8 Dual Loop Arena
                let hub_w = snap_grid(s * 1.25, 256.0).max(1024.0);
                let wing_y = hub_w * 0.5 + 768.0;
                let wing_x = hub_w * 0.5 + 640.0;
                rooms.push(RoomNode {
                    name: "CrossoverNexus".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(hub_w, hub_w, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthLoopArena".to_string(),
                    center: Vec3::new(0.0, wing_y, 320.0),
                    size: Vec3::new(1024.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthLoopArena".to_string(),
                    center: Vec3::new(0.0, -wing_y, 320.0),
                    size: Vec3::new(1024.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestCrossoverHall".to_string(),
                    center: Vec3::new(-wing_x, 0.0, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastCrossoverHall".to_string(),
                    center: Vec3::new(wing_x, 0.0, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Central cross + Figure-8 outer loops
                rooms.push(RoomNode {
                    name: "FigureEightSpineNS".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new(384.0, wing_y * 2.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "FigureEightSpineEW".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new(wing_x * 2.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, cx, cy, sx, sy) in [
                    ("NorthOuterSpan", 0.0, wing_y, wing_x * 2.0 + 320.0, 320.0),
                    ("SouthOuterSpan", 0.0, -wing_y, wing_x * 2.0 + 320.0, 320.0),
                    ("WestLoopConduit", -wing_x, 0.0, 320.0, wing_y * 2.0 + 320.0),
                    ("EastLoopConduit", wing_x, 0.0, 320.0, wing_y * 2.0 + 320.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::CrossroadsRotunda => {
                // DM-Liandri / DOM-Gearbolt: Grand 8-Spoke Star Rotunda + 4 Cardinal Halls + 4 Corner Nodes + Outer Wheel
                let rot_w = snap_grid(s * 1.25, 256.0).max(1152.0);
                let ring_d = rot_w * 0.5 + 640.0;
                rooms.push(RoomNode {
                    name: "GrandStarRotunda".to_string(),
                    center: Vec3::new(0.0, 0.0, 448.0),
                    size: Vec3::new(rot_w, rot_w, 896.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });
                for (name, dx, dy, team) in [
                    ("WestStarHall", -ring_d, 0.0, Some(0u8)),
                    ("EastStarHall", ring_d, 0.0, Some(1u8)),
                    ("NorthStarHall", 0.0, ring_d, None),
                    ("SouthStarHall", 0.0, -ring_d, None),
                    ("NWStarNode", -ring_d, ring_d, None),
                    ("NEStarNode", ring_d, ring_d, None),
                    ("SWStarNode", -ring_d, -ring_d, None),
                    ("SEStarNode", ring_d, -ring_d, None),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(dx, dy, 256.0),
                        size: Vec3::new(640.0, 640.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for (name, cx, cy, sx, sy) in [
                    ("SpokeEW", 0.0, 0.0, ring_d * 2.0, 352.0),
                    ("SpokeNS", 0.0, 0.0, 352.0, ring_d * 2.0),
                    ("WheelNorth", 0.0, ring_d, ring_d * 2.0 + 256.0, 288.0),
                    ("WheelSouth", 0.0, -ring_d, ring_d * 2.0 + 256.0, 288.0),
                    ("WheelWest", -ring_d, 0.0, 288.0, ring_d * 2.0 + 256.0),
                    ("WheelEast", ring_d, 0.0, 288.0, ring_d * 2.0 + 256.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::ParallelTrenchWarfare => {
                // CTF-Dreary / Gauntlet style: Central Bunker + Red/Blue Fortresses + North/South Pillboxes + Parallel War Trenches
                let base_x = snap_grid(s * 1.5, 256.0).max(1536.0);
                let trench_y = 1024.0_f32;
                rooms.push(RoomNode {
                    name: "CentralBunkerHub".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RedFortressBase".to_string(),
                    center: Vec3::new(-base_x, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueFortressBase".to_string(),
                    center: Vec3::new(base_x, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthPillboxHall".to_string(),
                    center: Vec3::new(0.0, trench_y, 256.0),
                    size: Vec3::new(896.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthPillboxHall".to_string(),
                    center: Vec3::new(0.0, -trench_y, 256.0),
                    size: Vec3::new(896.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, cx, cy, sx, sy) in [
                    ("CenterNoMansLand", 0.0, 0.0, base_x * 2.0, 352.0),
                    ("BunkerCrossConduit", 0.0, 0.0, 352.0, trench_y * 2.0),
                    ("NorthWarTrench", 0.0, trench_y, base_x * 2.0 + 320.0, 352.0),
                    ("SouthWarTrench", 0.0, -trench_y, base_x * 2.0 + 320.0, 352.0),
                    ("RedBaseFlankConduit", -base_x, 0.0, 352.0, trench_y * 2.0 + 320.0),
                    ("BlueBaseFlankConduit", base_x, 0.0, 352.0, trench_y * 2.0 + 320.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::TwinSpireBridges => {
                // CTF-EternalCave / LavaGiant style: Central Sky Chasm + Twin Octagonal Spires + 4 Forward Staging Halls + 3 Bridges
                let spire_x = snap_grid(s * 1.5, 256.0).max(1664.0);
                let stage_x = 960.0_f32;
                let stage_y = 896.0_f32;
                rooms.push(RoomNode {
                    name: "ChasmCentralSanctum".to_string(),
                    center: Vec3::new(0.0, 0.0, 448.0),
                    size: Vec3::new(1024.0, 1152.0, 896.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RedSpireCitadel".to_string(),
                    center: Vec3::new(-spire_x, 0.0, 448.0),
                    size: Vec3::new(1024.0, 1024.0, 896.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });
                rooms.push(RoomNode {
                    name: "BlueSpireCitadel".to_string(),
                    center: Vec3::new(spire_x, 0.0, 448.0),
                    size: Vec3::new(1024.0, 1024.0, 896.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });
                for (name, dx, dy, team) in [
                    ("RedNorthStaging", -stage_x, stage_y, Some(0u8)),
                    ("RedSouthStaging", -stage_x, -stage_y, Some(0u8)),
                    ("BlueNorthStaging", stage_x, stage_y, Some(1u8)),
                    ("BlueSouthStaging", stage_x, -stage_y, Some(1u8)),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(dx, dy, 256.0),
                        size: Vec3::new(640.0, 640.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for (name, cx, cy, sx, sy) in [
                    ("CenterHighSpan", 0.0, 0.0, spire_x * 2.0, 384.0),
                    ("NorthChasmBridge", 0.0, stage_y, stage_x * 2.0 + 320.0, 320.0),
                    ("SouthChasmBridge", 0.0, -stage_y, stage_x * 2.0 + 320.0, 320.0),
                    ("RedStagingCross", -stage_x, 0.0, 320.0, stage_y * 2.0 + 256.0),
                    ("BlueStagingCross", stage_x, 0.0, 320.0, stage_y * 2.0 + 256.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: cfg.toggles.include_skybox,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::HourglassNexusCTF => {
                // CTF-Command / HallOfGiants style: Converging Hourglass Crux + Red/Blue Atriums + 4 Wing Alcoves
                let base_x = snap_grid(s * 1.5, 256.0).max(1536.0);
                let wing_y = 960.0_f32;
                rooms.push(RoomNode {
                    name: "HourglassCrux".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RedAtriumBase".to_string(),
                    center: Vec3::new(-base_x, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueAtriumBase".to_string(),
                    center: Vec3::new(base_x, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                for (name, dx, dy, team) in [
                    ("RedNorthWing", -base_x, wing_y, Some(0u8)),
                    ("RedSouthWing", -base_x, -wing_y, Some(0u8)),
                    ("BlueNorthWing", base_x, wing_y, Some(1u8)),
                    ("BlueSouthWing", base_x, -wing_y, Some(1u8)),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(dx, dy, 256.0),
                        size: Vec3::new(768.0, 640.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for (name, cx, cy, sx, sy) in [
                    ("HourglassMainThroat", 0.0, 0.0, base_x * 2.0, 384.0),
                    ("RedAtriumSpine", -base_x, 0.0, 352.0, wing_y * 2.0 + 256.0),
                    ("BlueAtriumSpine", base_x, 0.0, 352.0, wing_y * 2.0 + 256.0),
                    ("NorthBypassConduit", 0.0, wing_y, base_x * 2.0 + 256.0, 288.0),
                    ("SouthBypassConduit", 0.0, -wing_y, base_x * 2.0 + 256.0, 288.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::FourCornersCourtyard => {
                // DM-Codex / Curse][ style: Grand Central Courtyard + 4 Corner Bastions + Cloister Ring + Central Cross
                let court_w = snap_grid(s * 1.25, 256.0).max(1152.0);
                let corner_d = court_w * 0.5 + 512.0;
                rooms.push(RoomNode {
                    name: "GrandCourtyard".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(court_w, court_w, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, dx, dy, team) in [
                    ("NorthWestBastion", -corner_d, corner_d, Some(0u8)),
                    ("SouthEastBastion", corner_d, -corner_d, Some(1u8)),
                    ("NorthEastBastion", corner_d, corner_d, None),
                    ("SouthWestBastion", -corner_d, -corner_d, None),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(dx, dy, 320.0),
                        size: Vec3::new(768.0, 768.0, 640.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for (name, cx, cy, sx, sy) in [
                    ("NorthCloister", 0.0, corner_d, corner_d * 2.0 + 320.0, 320.0),
                    ("SouthCloister", 0.0, -corner_d, corner_d * 2.0 + 320.0, 320.0),
                    ("WestCloister", -corner_d, 0.0, 320.0, corner_d * 2.0 + 320.0),
                    ("EastCloister", corner_d, 0.0, 320.0, corner_d * 2.0 + 320.0),
                    ("CourtyardCrossEW", 0.0, 0.0, corner_d * 2.0 + 256.0, 352.0),
                    ("CourtyardCrossNS", 0.0, 0.0, 352.0, corner_d * 2.0 + 256.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::CatacombLabyrinth => {
                // DOM-Cryptic / DM-Gothic style: 3x3 Tactical Grid Matrix of 9 Crypt Chambers + 6 Full Grid Passages
                let step = 1152.0_f32;
                rooms.push(RoomNode {
                    name: "CatacombCenterCrypt".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, gx, gy, team) in [
                    ("Crypt_West", -1, 0, Some(0u8)),
                    ("Crypt_East", 1, 0, Some(1u8)),
                    ("Crypt_North", 0, 1, None),
                    ("Crypt_South", 0, -1, None),
                    ("Crypt_NW", -1, 1, Some(0u8)),
                    ("Crypt_NE", 1, 1, None),
                    ("Crypt_SW", -1, -1, None),
                    ("Crypt_SE", 1, -1, Some(1u8)),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(gx as f32 * step, gy as f32 * step, 256.0),
                        size: Vec3::new(640.0, 640.0, 512.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for gy in [-1, 0, 1] {
                    rooms.push(RoomNode {
                        name: format!("GridPassageEW_{}", gy + 1),
                        center: Vec3::new(0.0, gy as f32 * step, 192.0),
                        size: Vec3::new(step * 2.0 + 320.0, 288.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
                for gx in [-1, 0, 1] {
                    rooms.push(RoomNode {
                        name: format!("GridPassageNS_{}", gx + 1),
                        center: Vec3::new(gx as f32 * step, 0.0, 192.0),
                        size: Vec3::new(288.0, step * 2.0 + 320.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::ReactorCoreConcentric => {
                // CTF-Nucleus / DM-Pressure style: Tall Octagonal Inner Reactor Core + 4 Containment Bays + 360° Ring
                let core_w = snap_grid(s * 1.25, 256.0).max(1152.0);
                let bay_d = core_w * 0.5 + 640.0;
                rooms.push(RoomNode {
                    name: "InnerReactorCore".to_string(),
                    center: Vec3::new(0.0, 0.0, 512.0),
                    size: Vec3::new(core_w, core_w, 1024.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });
                for (name, dx, dy, team) in [
                    ("WestContainmentBay", -bay_d, 0.0, Some(0u8)),
                    ("EastContainmentBay", bay_d, 0.0, Some(1u8)),
                    ("NorthCoolantHall", 0.0, bay_d, None),
                    ("SouthCoolantHall", 0.0, -bay_d, None),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(dx, dy, 320.0),
                        size: Vec3::new(768.0, 768.0, 640.0),
                        is_corridor: false,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: team,
                        is_octagonal: false,
                    });
                }
                for (name, cx, cy, sx, sy) in [
                    ("ReactorAirlockEW", 0.0, 0.0, bay_d * 2.0, 352.0),
                    ("ReactorAirlockNS", 0.0, 0.0, 352.0, bay_d * 2.0),
                    ("ContainmentRingN", 0.0, bay_d, bay_d * 2.0 + 320.0, 320.0),
                    ("ContainmentRingS", 0.0, -bay_d, bay_d * 2.0 + 320.0, 320.0),
                    ("ContainmentRingW", -bay_d, 0.0, 320.0, bay_d * 2.0 + 320.0),
                    ("ContainmentRingE", bay_d, 0.0, 320.0, bay_d * 2.0 + 320.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::ZigguratTempleComplex => {
                // DM-Pyramid / ArcaneTemple style: Monumental Ziggurat Sanctum + Sun/Moon Courtyards + High Altar & Crypt
                let sanctum_w = snap_grid(s * 1.25, 256.0).max(1280.0);
                let court_x = sanctum_w * 0.5 + 640.0;
                let vault_y = sanctum_w * 0.5 + 512.0;
                rooms.push(RoomNode {
                    name: "ZigguratGrandSanctum".to_string(),
                    center: Vec3::new(0.0, 0.0, 448.0),
                    size: Vec3::new(sanctum_w, sanctum_w, 896.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestSunCourtyard".to_string(),
                    center: Vec3::new(-court_x, 0.0, 320.0),
                    size: Vec3::new(896.0, 1024.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastMoonCourtyard".to_string(),
                    center: Vec3::new(court_x, 0.0, 320.0),
                    size: Vec3::new(896.0, 1024.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthHighAltar".to_string(),
                    center: Vec3::new(0.0, vault_y, 320.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthAncestralVault".to_string(),
                    center: Vec3::new(0.0, -vault_y, 320.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                for (name, cx, cy, sx, sy) in [
                    ("TempleColonnadeEW", 0.0, 0.0, court_x * 2.0, 384.0),
                    ("TempleColonnadeNS", 0.0, 0.0, 384.0, vault_y * 2.0),
                    ("NorthProcessional", 0.0, vault_y, court_x * 2.0 + 320.0, 320.0),
                    ("SouthProcessional", 0.0, -vault_y, court_x * 2.0 + 320.0, 320.0),
                    ("WestColonnadeArm", -court_x, 0.0, 320.0, vault_y * 2.0 + 320.0),
                    ("EastColonnadeArm", court_x, 0.0, 320.0, vault_y * 2.0 + 320.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }

            WizardArchetype::CanyonSerpentineRun => {
                // AS-Rook / DM-Peak style: 5-Basin S-Curve Serpentine Canyon + Sapper Flank Tunnels
                let step_x = snap_grid(s * 1.25, 256.0).max(1280.0);
                let step_y = 896.0_f32;
                rooms.push(RoomNode {
                    name: "CenterCanyonGorge".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestLandingOutpost".to_string(),
                    center: Vec3::new(-step_x, -step_y, 320.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthWestSwitchback".to_string(),
                    center: Vec3::new(-step_x, step_y, 320.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthEastSwitchback".to_string(),
                    center: Vec3::new(step_x, -step_y, 320.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastCitadelKeep".to_string(),
                    center: Vec3::new(step_x, step_y, 384.0),
                    size: Vec3::new(896.0, 768.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                // S-Curve Canyon Passes + Direct Sapper Flank Tunnels
                for (name, cx, cy, sx, sy) in [
                    ("WestCanyonPass", -step_x, 0.0, 352.0, step_y * 2.0 + 256.0),
                    ("EastCanyonPass", step_x, 0.0, 352.0, step_y * 2.0 + 256.0),
                    ("NorthGorgeConduit", -step_x * 0.5, step_y, step_x + 352.0, 352.0),
                    ("SouthGorgeConduit", step_x * 0.5, -step_y, step_x + 352.0, 352.0),
                    ("CenterGorgeSpine", 0.0, 0.0, 352.0, step_y * 2.0 + 352.0),
                    ("NorthSapperTunnel", step_x * 0.5, step_y, step_x + 352.0, 288.0),
                    ("SouthSapperTunnel", -step_x * 0.5, -step_y, step_x + 352.0, 288.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 192.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }
            WizardArchetype::GatehouseFuseAssault => {
                // AS-Tutorial / AS-Guardia style: 4-stage breach assault across West Landing Bay,
                // Fuse Generator Hall, Fortified Bridge Yard, and Inner Command Vault.
                let step_x = snap_grid(s * 1.25, 256.0).max(1536.0);
                rooms.push(RoomNode {
                    name: "WestLandingBay".to_string(),
                    center: Vec3::new(-step_x * 1.5, 0.0, 320.0),
                    size: Vec3::new(1024.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "FuseGeneratorHall".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BridgeControlYard".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "InnerCommandVault".to_string(),
                    center: Vec3::new(step_x * 1.5, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Min-cut gate corridors
                for (name, cx, sx) in [
                    ("DockBreachPassage", -step_x, step_x - 896.0 + 128.0),
                    ("GatehouseBridgeSpan", 0.0, step_x - 960.0 + 128.0),
                    ("VaultBlastCorridor", step_x, step_x - 896.0 + 128.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, 0.0, 192.0),
                        size: Vec3::new(sx, 352.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }
            WizardArchetype::LinearInfiltrationVessel => {
                // AS-Frigate / AS-HiSpeed style: 5-compartment linear vessel infiltration across
                // Stern Boarding Deck, Lower Cargo Hold, Boiler Engine Room, Security Ward, and Helm Bridge.
                let step_x = snap_grid(s * 1.25, 256.0).max(1536.0);
                rooms.push(RoomNode {
                    name: "SternBoardingDeck".to_string(),
                    center: Vec3::new(-step_x * 2.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "LowerCargoHold".to_string(),
                    center: Vec3::new(-step_x * 1.0, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BoilerEngineRoom".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "UpperSecurityWard".to_string(),
                    center: Vec3::new(step_x * 1.0, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ForwardHelmBridge".to_string(),
                    center: Vec3::new(step_x * 2.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Min-cut bulkhead corridors
                for (name, cx, sx) in [
                    ("SternBulkhead", -step_x * 1.5, step_x - 896.0 + 128.0),
                    ("EngineAccessShaft", -step_x * 0.5, step_x - 960.0 + 128.0),
                    ("MidshipBulkhead", step_x * 0.5, step_x - 960.0 + 128.0),
                    ("BridgeBlastDoorway", step_x * 1.5, step_x - 896.0 + 128.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, 0.0, 192.0),
                        size: Vec3::new(sx, 352.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }
            WizardArchetype::CastleKeepWinch => {
                // AS-Rook / AS-Overlord style: 5-stage medieval mountain castle fortress
                // Outer Camp -> Gatehouse Entry -> Grand Library -> Winch Tower -> Royal Throne Sanctum
                let step_x = snap_grid(s * 1.25, 256.0).max(1280.0);
                let step_y = 896.0_f32;
                rooms.push(RoomNode {
                    name: "OuterCampOutpost".to_string(),
                    center: Vec3::new(-step_x * 1.5, -step_y, 320.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "GatehouseEntryHall".to_string(),
                    center: Vec3::new(-step_x * 0.5, -step_y, 320.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "GrandLibraryArchive".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 384.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WinchTowerChamber".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 384.0),
                    size: Vec3::new(896.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });
                rooms.push(RoomNode {
                    name: "InnerSanctumKeep".to_string(),
                    center: Vec3::new(step_x * 0.5, step_y, 320.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RoyalThroneSanctum".to_string(),
                    center: Vec3::new(step_x * 1.5, step_y, 384.0),
                    size: Vec3::new(896.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                // Min-cut fortress corridors connecting stages sequentially
                rooms.push(RoomNode {
                    name: "DrawbridgeCauseway".to_string(),
                    center: Vec3::new(-step_x, -step_y, 192.0),
                    size: Vec3::new(step_x - 768.0 + 128.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "OuterCloisterNS".to_string(),
                    center: Vec3::new(-step_x * 0.5, -step_y * 0.5, 192.0),
                    size: Vec3::new(352.0, 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "GrandKeepGallery".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new((step_x - 896.0 + 128.0).max(512.0), 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WinchTowerPassage".to_string(),
                    center: Vec3::new(step_x * 0.5, step_y * 0.5, 192.0),
                    size: Vec3::new(352.0, 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ThronePortcullis".to_string(),
                    center: Vec3::new(step_x, step_y, 192.0),
                    size: Vec3::new(step_x - 768.0 + 128.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::TriadSpireAtrium => {
                // DOM-Cryptic / DOM-Sesmar style: Grand central octagonal atrium surrounded by
                // 3 symmetric 120-degree wings (Alpha, Beta, Gamma) and continuous outer perimeter ring corridors.
                let step_dist = snap_grid(s * 1.25, 256.0).max(1536.0);
                let wing_size = 896.0_f32;
                let hub_size = 1152.0_f32;

                // Central Grand Atrium
                rooms.push(RoomNode {
                    name: "CentralSpireAtrium".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(hub_size, hub_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Wing South (Alpha)
                rooms.push(RoomNode {
                    name: "WingSouthAlpha".to_string(),
                    center: Vec3::new(0.0, -step_dist, 320.0),
                    size: Vec3::new(wing_size, wing_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Wing North-West (Beta)
                let cos30 = 0.8660254_f32;
                let sin30 = 0.5_f32;
                let nw_x = snap_grid(-step_dist * cos30, 256.0);
                let nw_y = snap_grid(step_dist * sin30, 256.0);
                rooms.push(RoomNode {
                    name: "WingNorthWestBeta".to_string(),
                    center: Vec3::new(nw_x, nw_y, 320.0),
                    size: Vec3::new(wing_size, wing_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Wing North-East (Gamma)
                let ne_x = snap_grid(step_dist * cos30, 256.0);
                let ne_y = snap_grid(step_dist * sin30, 256.0);
                rooms.push(RoomNode {
                    name: "WingNorthEastGamma".to_string(),
                    center: Vec3::new(ne_x, ne_y, 320.0),
                    size: Vec3::new(wing_size, wing_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Radial Connector Corridors connecting Center to Wings
                rooms.push(RoomNode {
                    name: "RadialSpokeSouth".to_string(),
                    center: Vec3::new(0.0, -step_dist * 0.5, 192.0),
                    size: Vec3::new(384.0, step_dist - wing_size + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RadialSpokeNW".to_string(),
                    center: Vec3::new(nw_x * 0.5, nw_y * 0.5, 192.0),
                    size: Vec3::new((nw_x.abs() - wing_size * 0.5 + 128.0).max(384.0), 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RadialSpokeNE".to_string(),
                    center: Vec3::new(ne_x * 0.5, ne_y * 0.5, 192.0),
                    size: Vec3::new((ne_x.abs() - wing_size * 0.5 + 128.0).max(384.0), 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Outer Perimeter Flank Loop between the 3 wings
                rooms.push(RoomNode {
                    name: "NorthFlankGallery".to_string(),
                    center: Vec3::new(0.0, nw_y, 192.0),
                    size: Vec3::new(ne_x * 2.0 - wing_size + 128.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestFlankGallery".to_string(),
                    center: Vec3::new(nw_x, (nw_y - step_dist) * 0.5, 192.0),
                    size: Vec3::new(352.0, (nw_y + step_dist - wing_size + 128.0).max(512.0), 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastFlankGallery".to_string(),
                    center: Vec3::new(ne_x, (ne_y - step_dist) * 0.5, 192.0),
                    size: Vec3::new(352.0, (ne_y + step_dist - wing_size + 128.0).max(512.0), 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::VerticalTierCinder => {
                // DOM-Cinder / DOM-Ghardhen style: 3-tier vertical arena featuring a high spire,
                // central mezzanine crossroad, and sunken basin with flanking lift galleries.
                let step_y = snap_grid(s * 1.25, 256.0).max(1280.0);
                let tier_h = 240.0_f32;

                // Tier 2: High Spire Lookout (Top, Z=+240)
                rooms.push(RoomNode {
                    name: "HighSpireLookout".to_string(),
                    center: Vec3::new(0.0, -step_y, 320.0 + tier_h),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Tier 1: Central Mezzanine Crossroad (Center, Z=0)
                rooms.push(RoomNode {
                    name: "CentralMezzanineCrux".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(1024.0, 1024.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Tier 0: Sunken Basin (Bottom, Z=-240)
                rooms.push(RoomNode {
                    name: "SunkenLavaBasin".to_string(),
                    center: Vec3::new(0.0, step_y, 320.0 - tier_h),
                    size: Vec3::new(1024.0, 1024.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Central Stair & Ramp Spine (North-South)
                rooms.push(RoomNode {
                    name: "SpireInclineSpine".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.5, 192.0 + tier_h * 0.5),
                    size: Vec3::new(384.0, step_y - 896.0 + 128.0, 480.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BasinDescentSpine".to_string(),
                    center: Vec3::new(0.0, step_y * 0.5, 192.0 - tier_h * 0.5),
                    size: Vec3::new(384.0, step_y - 896.0 + 128.0, 480.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Flanking East & West Lift Galleries connecting all 3 tiers
                let gal_x = 1024.0_f32;
                rooms.push(RoomNode {
                    name: "EastLiftGallery".to_string(),
                    center: Vec3::new(gal_x, 0.0, 320.0),
                    size: Vec3::new(384.0, step_y * 2.0 + 512.0, 768.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestCatwalkGallery".to_string(),
                    center: Vec3::new(-gal_x, 0.0, 320.0),
                    size: Vec3::new(384.0, step_y * 2.0 + 512.0, 768.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Cross transits connecting East/West galleries to Spire, Mezzanine, and Basin
                for (name, y, z) in [
                    ("SpireCrossTransit", -step_y, tier_h),
                    ("MezzanineCrossTransit", 0.0, 0.0),
                    ("BasinCrossTransit", step_y, -tier_h),
                ] {
                    rooms.push(RoomNode {
                        name: format!("{name}_E"),
                        center: Vec3::new(gal_x * 0.6, y, 192.0 + z),
                        size: Vec3::new(768.0, 320.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                    rooms.push(RoomNode {
                        name: format!("{name}_W"),
                        center: Vec3::new(-gal_x * 0.6, y, 192.0 + z),
                        size: Vec3::new(768.0, 320.0, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }
            WizardArchetype::GearsIndustrialFurnace => {
                // DOM-Gearbolt / DOM-MetalDream style: Industrial smelting plant with exposed
                // center bridge, elevated crane dock, and underground cargo vault.
                let step_x = snap_grid(s * 1.25, 256.0).max(1408.0);
                let step_y = snap_grid(s * 1.0, 256.0).max(1152.0);

                // Central Industrial Furnace Hub
                rooms.push(RoomNode {
                    name: "CentralFurnaceCrux".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(1280.0, 1280.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // East Elevated Crane Dock
                rooms.push(RoomNode {
                    name: "ElevatedCraneDock".to_string(),
                    center: Vec3::new(step_x, 0.0, 320.0 + 128.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // West Underground Cargo Vault
                rooms.push(RoomNode {
                    name: "UndergroundVault".to_string(),
                    center: Vec3::new(-step_x, 0.0, 320.0 - 128.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // North Turbine Deck
                rooms.push(RoomNode {
                    name: "NorthTurbineDeck".to_string(),
                    center: Vec3::new(0.0, step_y, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Corridors
                rooms.push(RoomNode {
                    name: "EastCraneConveyor".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 192.0 + 64.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestVaultChute".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 192.0 - 64.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthTurbinePassage".to_string(),
                    center: Vec3::new(0.0, step_y * 0.5, 192.0),
                    size: Vec3::new(384.0, step_y - 896.0 + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Outer North-East and North-West bypass loops
                rooms.push(RoomNode {
                    name: "NorthEastBypass".to_string(),
                    center: Vec3::new(step_x * 0.5, step_y, 192.0),
                    size: Vec3::new(step_x - 384.0, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthWestBypass".to_string(),
                    center: Vec3::new(-step_x * 0.5, step_y, 192.0),
                    size: Vec3::new(step_x - 384.0, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::FacingWorldsOrbitalSpire => {
                // CTF-Face / CTF-Face][ style: Two opposing multi-tier orbital spires spanning
                // a vast planetary void via an arched central bridge and high sniper perches.
                let step_x = snap_grid(s * 2.2, 256.0).max(3072.0);
                let tower_size = 1024.0_f32;

                // Red Team Spire (Ground Flag Sanctum)
                rooms.push(RoomNode {
                    name: "RedFlagSanctum".to_string(),
                    center: Vec3::new(-step_x, 0.0, 320.0),
                    size: Vec3::new(tower_size, tower_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });
                // Red Sniper Overwatch Roof
                rooms.push(RoomNode {
                    name: "RedSniperRoost".to_string(),
                    center: Vec3::new(-step_x, 0.0, 640.0 + 384.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });

                // Blue Team Spire (Ground Flag Sanctum)
                rooms.push(RoomNode {
                    name: "BlueFlagSanctum".to_string(),
                    center: Vec3::new(step_x, 0.0, 320.0),
                    size: Vec3::new(tower_size, tower_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });
                // Blue Sniper Overwatch Roof
                rooms.push(RoomNode {
                    name: "BlueSniperRoost".to_string(),
                    center: Vec3::new(step_x, 0.0, 640.0 + 384.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });

                // Central Cosmic Void Midfield Chamber
                rooms.push(RoomNode {
                    name: "CosmicVoidMidfield".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Connecting Bridges:
                // 1) Main Arched Sky Bridge (West -> Center -> East)
                rooms.push(RoomNode {
                    name: "ArchedSkyBridge_W".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 224.0),
                    size: Vec3::new(step_x - tower_size + 128.0, 384.0, 448.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ArchedSkyBridge_E".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 224.0),
                    size: Vec3::new(step_x - tower_size + 128.0, 384.0, 448.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 2) Flanking Lower Void Catwalks (North & South spanning entire standoff)
                let flank_y = 640.0_f32;
                let catwalk_length = step_x * 2.0 + 256.0;
                rooms.push(RoomNode {
                    name: "NorthVoidCatwalk".to_string(),
                    center: Vec3::new(0.0, flank_y, 160.0),
                    size: Vec3::new(catwalk_length, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthVoidCatwalk".to_string(),
                    center: Vec3::new(0.0, -flank_y, 160.0),
                    size: Vec3::new(catwalk_length, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 3) Lateral Base Connectors to Catwalks
                rooms.push(RoomNode {
                    name: "RedCatwalkAccess_N".to_string(),
                    center: Vec3::new(-step_x, flank_y * 0.5, 192.0),
                    size: Vec3::new(288.0, flank_y + 64.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RedCatwalkAccess_S".to_string(),
                    center: Vec3::new(-step_x, -flank_y * 0.5, 192.0),
                    size: Vec3::new(288.0, flank_y + 64.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueCatwalkAccess_N".to_string(),
                    center: Vec3::new(step_x, flank_y * 0.5, 192.0),
                    size: Vec3::new(288.0, flank_y + 64.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueCatwalkAccess_S".to_string(),
                    center: Vec3::new(step_x, -flank_y * 0.5, 192.0),
                    size: Vec3::new(288.0, flank_y + 64.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
            }
            WizardArchetype::HydroSubmarineDock => {
                // CTF-November / CTF-Hydro16 style: Subterranean naval submarine drydock facility
                // with central vessel cradle, upper catwalks, drainage conduits, and water turbine ducts.
                let step_x = snap_grid(s * 1.5, 256.0).max(1920.0);
                let dock_w = 1408.0_f32;
                let dock_l = 1152.0_f32;

                // Central Submarine Drydock Hangar (with water hazard basin)
                rooms.push(RoomNode {
                    name: "SubmarineDrydockHangar".to_string(),
                    center: Vec3::new(0.0, 0.0, 256.0),
                    size: Vec3::new(dock_w, dock_l, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Red Team Naval Command Base (-X, Team 0)
                rooms.push(RoomNode {
                    name: "RedNavalCommand".to_string(),
                    center: Vec3::new(-step_x, 0.0, 256.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });

                // Blue Team Naval Command Base (+X, Team 1)
                rooms.push(RoomNode {
                    name: "BlueNavalCommand".to_string(),
                    center: Vec3::new(step_x, 0.0, 256.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });

                // Main Drydock Concourse (West & East)
                rooms.push(RoomNode {
                    name: "DrydockMainConcourse_W".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 192.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "DrydockMainConcourse_E".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 192.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Flanking North & South Turbine Water Ducts
                let turbine_y = 640.0_f32;
                rooms.push(RoomNode {
                    name: "NorthTurbineDuct".to_string(),
                    center: Vec3::new(0.0, turbine_y, 128.0),
                    size: Vec3::new(step_x * 1.4, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthTurbineDuct".to_string(),
                    center: Vec3::new(0.0, -turbine_y, 128.0),
                    size: Vec3::new(step_x * 1.4, 320.0, 320.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::CanyonCrestCitadels => {
                // CTF-LavaGiant / CTF-Kosov style: Opposed mountain citadels across a rugged
                // multi-elevation canyon with upper cliff sniper ridge, central bridge, and lower hazardous chasm.
                let step_x = snap_grid(s * 2.4, 256.0).max(3328.0);
                let keep_w = 1280.0_f32;
                let keep_d = 1536.0_f32;
                let keep_h = 768.0_f32;
                let flank_y = 1088.0_f32;

                // 1. Red Mountain Citadel Keep (-X, Team 0)
                rooms.push(RoomNode {
                    name: "RedMountainKeep".to_string(),
                    center: Vec3::new(-step_x, 0.0, 320.0),
                    size: Vec3::new(keep_w, keep_d, keep_h),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });
                // Red Sniper Ridge Overlook Balcony (Elevated Roost seamlessly overlooking the canyon)
                rooms.push(RoomNode {
                    name: "RedSniperOverlook".to_string(),
                    center: Vec3::new(-step_x + 480.0, 0.0, 672.0),
                    size: Vec3::new(640.0, 768.0, 384.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });

                // 2. Blue Mountain Citadel Keep (+X, Team 1)
                rooms.push(RoomNode {
                    name: "BlueMountainKeep".to_string(),
                    center: Vec3::new(step_x, 0.0, 320.0),
                    size: Vec3::new(keep_w, keep_d, keep_h),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });
                // Blue Sniper Ridge Overlook Balcony (Elevated Roost seamlessly overlooking the canyon)
                rooms.push(RoomNode {
                    name: "BlueSniperOverlook".to_string(),
                    center: Vec3::new(step_x - 480.0, 0.0, 672.0),
                    size: Vec3::new(640.0, 768.0, 384.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });

                // 3. Central Deep Canyon Gorge & Boiling Lava Chasm
                rooms.push(RoomNode {
                    name: "CentralCanyonGorge".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(step_x * 1.5, 2560.0, 1024.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 4. Route 1: Mid-level Canyon Spanning Bridge (Y=0, Z=192, floor Z=-64 matching keep floor)
                let bridge_w_len = step_x * 0.9 + 128.0;
                rooms.push(RoomNode {
                    name: "CanyonBridge_W".to_string(),
                    center: Vec3::new(-step_x * 0.45, 0.0, 192.0),
                    size: Vec3::new(bridge_w_len, 448.0, 512.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "CanyonBridge_E".to_string(),
                    center: Vec3::new(step_x * 0.45, 0.0, 192.0),
                    size: Vec3::new(bridge_w_len, 448.0, 512.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 5. Route 2: Upper Cliffside Sniper Ridge (+Y, Z=+416)
                let ridge_len = step_x * 2.0 + 256.0;
                rooms.push(RoomNode {
                    name: "NorthCliffOverlook".to_string(),
                    center: Vec3::new(0.0, flank_y, 416.0),
                    size: Vec3::new(ridge_len, 448.0, 448.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Connectors to Ridge Overlook
                rooms.push(RoomNode {
                    name: "RedRidgeAccess".to_string(),
                    center: Vec3::new(-step_x + 384.0, flank_y * 0.5, 304.0),
                    size: Vec3::new(384.0, flank_y + 128.0, 448.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueRidgeAccess".to_string(),
                    center: Vec3::new(step_x - 384.0, flank_y * 0.5, 304.0),
                    size: Vec3::new(384.0, flank_y + 128.0, 448.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // 6. Route 3: Lower Subterranean Magma Cavern (-Y, Z=128, floor Z=-64 matching keep floor)
                rooms.push(RoomNode {
                    name: "SouthCavernRun".to_string(),
                    center: Vec3::new(0.0, -flank_y, 128.0),
                    size: Vec3::new(ridge_len, 448.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                // Connectors to Magma Cavern
                rooms.push(RoomNode {
                    name: "RedGrottoAccess".to_string(),
                    center: Vec3::new(-step_x + 384.0, -flank_y * 0.5, 128.0),
                    size: Vec3::new(384.0, flank_y + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BlueGrottoAccess".to_string(),
                    center: Vec3::new(step_x - 384.0, -flank_y * 0.5, 128.0),
                    size: Vec3::new(384.0, flank_y + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
            }
            WizardArchetype::GothicCathedralNave => {
                // Inspired by DM-Gothic & DM-Codex: Grand vaulted cathedral
                // High Nave, Apse Altar, Narthex, Side Aisles, Transept Cross, Subterranean Crypt
                let step_y = snap_grid(s * 1.2, 256.0).max(1152.0);
                let aisle_x = snap_grid(s * 0.7, 128.0).max(640.0);

                // 1. High Nave
                rooms.push(RoomNode {
                    name: "CathedralNave".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, step_y, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 2. Apse High Altar (North, elevated floor)
                rooms.push(RoomNode {
                    name: "ApseHighAltar".to_string(),
                    center: Vec3::new(0.0, step_y, 448.0),
                    size: Vec3::new(768.0, 640.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // 3. Narthex Vestibule (South)
                rooms.push(RoomNode {
                    name: "NarthexVestibule".to_string(),
                    center: Vec3::new(0.0, -step_y, 320.0),
                    size: Vec3::new(768.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 4. West Side Aisle
                rooms.push(RoomNode {
                    name: "WestSideAisle".to_string(),
                    center: Vec3::new(-aisle_x, 0.0, 256.0),
                    size: Vec3::new(384.0, step_y * 1.4, 448.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 5. East Side Aisle
                rooms.push(RoomNode {
                    name: "EastSideAisle".to_string(),
                    center: Vec3::new(aisle_x, 0.0, 256.0),
                    size: Vec3::new(384.0, step_y * 1.4, 448.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 6. Subterranean Crypt
                rooms.push(RoomNode {
                    name: "CryptCatacomb".to_string(),
                    center: Vec3::new(0.0, 0.0, -128.0),
                    size: Vec3::new(896.0, 896.0, 384.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Connecting corridors
                rooms.push(RoomNode {
                    name: "TranseptCross_W".to_string(),
                    center: Vec3::new(-aisle_x * 0.5, 0.0, 192.0),
                    size: Vec3::new(aisle_x - 384.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "TranseptCross_E".to_string(),
                    center: Vec3::new(aisle_x * 0.5, 0.0, 192.0),
                    size: Vec3::new(aisle_x - 384.0 + 128.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "AltarAmbulatory".to_string(),
                    center: Vec3::new(0.0, step_y * 0.625, 256.0),
                    size: Vec3::new(384.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NarthexEntryConduit".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.625, 256.0),
                    size: Vec3::new(384.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "CryptStairTransit".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.25, -32.0),
                    size: Vec3::new(256.0, 448.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::SunkenColosseumBasin => {
                // Inspired by DM-Morbias][ & DM-Agony: Multi-tier sunken gladiator arena
                let basin_r = snap_grid(s * 1.5, 256.0).max(1536.0);
                let ledge_offset = basin_r * 0.5 + 384.0;
                let ramp_offset = basin_r * 0.5 + 64.0;
                let ramp_span = 512.0;

                // Central combat pit
                rooms.push(RoomNode {
                    name: "ColosseumBasinCore".to_string(),
                    center: Vec3::new(0.0, 0.0, 192.0),
                    size: Vec3::new(basin_r, basin_r, 640.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // 4 Cardinal elevated spectator galleries (floor_z = 160 vs -128)
                rooms.push(RoomNode {
                    name: "NorthSpectatorLedge".to_string(),
                    center: Vec3::new(0.0, ledge_offset, 384.0),
                    size: Vec3::new(896.0, 512.0, 448.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthSpectatorLedge".to_string(),
                    center: Vec3::new(0.0, -ledge_offset, 384.0),
                    size: Vec3::new(896.0, 512.0, 448.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastSpectatorLedge".to_string(),
                    center: Vec3::new(ledge_offset, 0.0, 384.0),
                    size: Vec3::new(512.0, 896.0, 448.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestSpectatorLedge".to_string(),
                    center: Vec3::new(-ledge_offset, 0.0, 384.0),
                    size: Vec3::new(512.0, 896.0, 448.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Underground hypogeum beast cells
                rooms.push(RoomNode {
                    name: "SubBasinHypogeum".to_string(),
                    center: Vec3::new(0.0, 0.0, -96.0),
                    size: Vec3::new(768.0, 768.0, 256.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Axial rampways spanning seamlessly from basin core into spectator ledges
                for (name, cx, cy, sx, sy) in [
                    ("ColosseumRamp_N", 0.0, ramp_offset, 384.0, ramp_span),
                    ("ColosseumRamp_S", 0.0, -ramp_offset, 384.0, ramp_span),
                    ("ColosseumRamp_E", ramp_offset, 0.0, ramp_span, 384.0),
                    ("ColosseumRamp_W", -ramp_offset, 0.0, ramp_span, 384.0),
                ] {
                    rooms.push(RoomNode {
                        name: name.to_string(),
                        center: Vec3::new(cx, cy, 208.0),
                        size: Vec3::new(sx, sy, 384.0),
                        is_corridor: true,
                        sky_ceiling: false,
                        has_hazard_pit: false,
                        team_owner: None,
                        is_octagonal: false,
                    });
                }
            }
            WizardArchetype::FloatingAsteroidIslands => {
                // Inspired by CTF-Face & DM-SpaceNoxx: Suspended platform cluster in deep space void
                let pod_dist = snap_grid(s * 1.5, 256.0).max(1408.0);
                let island_size = 1024.0_f32;

                // Central suspended arena
                rooms.push(RoomNode {
                    name: "CentralAsteroidIsland".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(island_size, island_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // 4 Floating satellite platforms at different heights
                rooms.push(RoomNode {
                    name: "NorthOrbitalPlatform".to_string(),
                    center: Vec3::new(0.0, pod_dist, 384.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });
                rooms.push(RoomNode {
                    name: "SouthOrbitalPlatform".to_string(),
                    center: Vec3::new(0.0, -pod_dist, 256.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });
                rooms.push(RoomNode {
                    name: "EastOrbitalPlatform".to_string(),
                    center: Vec3::new(pod_dist, 0.0, 448.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "WestOrbitalPlatform".to_string(),
                    center: Vec3::new(-pod_dist, 0.0, 192.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Void bridge conduits
                rooms.push(RoomNode {
                    name: "VoidBridge_N".to_string(),
                    center: Vec3::new(0.0, pod_dist * 0.5, 256.0),
                    size: Vec3::new(320.0, pod_dist - 768.0 + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "VoidBridge_S".to_string(),
                    center: Vec3::new(0.0, -pod_dist * 0.5, 224.0),
                    size: Vec3::new(320.0, pod_dist - 768.0 + 128.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "VoidBridge_E".to_string(),
                    center: Vec3::new(pod_dist * 0.5, 0.0, 288.0),
                    size: Vec3::new(pod_dist - 768.0 + 128.0, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "VoidBridge_W".to_string(),
                    center: Vec3::new(-pod_dist * 0.5, 0.0, 192.0),
                    size: Vec3::new(pod_dist - 768.0 + 128.0, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::SymmetricalHydroStation => {
                // Inspired by CTF-Hydro16 & CTF-November: Hydroelectric power station & dam
                let step_x = snap_grid(s * 1.5, 256.0).max(1664.0);
                let hub_size = 1280.0_f32;

                // Central Generator Turbine Hall
                rooms.push(RoomNode {
                    name: "CentralTurbineHall".to_string(),
                    center: Vec3::new(0.0, 0.0, 256.0),
                    size: Vec3::new(hub_size, hub_size, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: true,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Red Pumping Station (-X)
                rooms.push(RoomNode {
                    name: "RedPumpingStation".to_string(),
                    center: Vec3::new(-step_x, 0.0, 192.0),
                    size: Vec3::new(896.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });

                // Blue Pumping Station (+X)
                rooms.push(RoomNode {
                    name: "BluePumpingStation".to_string(),
                    center: Vec3::new(step_x, 0.0, 192.0),
                    size: Vec3::new(896.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // High Spillway Crest Bridge (Z=512)
                rooms.push(RoomNode {
                    name: "HighSpillwayBridge".to_string(),
                    center: Vec3::new(0.0, 0.0, 576.0),
                    size: Vec3::new(step_x * 1.8, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Penstock Sluice Tunnels
                rooms.push(RoomNode {
                    name: "RedPenstockSluice".to_string(),
                    center: Vec3::new(-step_x * 0.5, 0.0, 160.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BluePenstockSluice".to_string(),
                    center: Vec3::new(step_x * 0.5, 0.0, 160.0),
                    size: Vec3::new(step_x - 896.0 + 128.0, 352.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NorthBypassConduit".to_string(),
                    center: Vec3::new(0.0, 768.0, 192.0),
                    size: Vec3::new(step_x * 1.3, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SouthBypassConduit".to_string(),
                    center: Vec3::new(0.0, -768.0, 192.0),
                    size: Vec3::new(step_x * 1.3, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
            WizardArchetype::ExcavationMiningQuarry => {
                // Inspired by DOM-Ghardhen & DM-Deck16][: 3-tier terraced open pit mine
                let step_y = snap_grid(s * 1.2, 256.0).max(1152.0);

                // Tier 1: Upper Quarry Rim (Z=512, floor_z=256)
                rooms.push(RoomNode {
                    name: "UpperQuarryRim".to_string(),
                    center: Vec3::new(0.0, step_y, 512.0),
                    size: Vec3::new(1024.0, 1024.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Tier 2: Mid Conveyor Hub (Z=320, floor_z=64)
                rooms.push(RoomNode {
                    name: "MidConveyorHub".to_string(),
                    center: Vec3::new(0.0, 0.0, 320.0),
                    size: Vec3::new(1280.0, 1024.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Tier 3: Deep Extraction Pit (Z=128, floor_z=-128)
                rooms.push(RoomNode {
                    name: "DeepExtractionPit".to_string(),
                    center: Vec3::new(0.0, -step_y, 128.0),
                    size: Vec3::new(1024.0, 1024.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Haul tunnels and conveyor chutes
                rooms.push(RoomNode {
                    name: "WestMineralShaft".to_string(),
                    center: Vec3::new(-768.0, 0.0, 256.0),
                    size: Vec3::new(384.0, step_y * 1.5, 448.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "EastCrusherConduit".to_string(),
                    center: Vec3::new(768.0, 0.0, 256.0),
                    size: Vec3::new(384.0, step_y * 1.5, 448.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "QuarryChute_Upper".to_string(),
                    center: Vec3::new(0.0, step_y * 0.5, 320.0),
                    size: Vec3::new(384.0, (step_y - 256.0).max(512.0), 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "QuarryChute_Lower".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.5, 192.0),
                    size: Vec3::new(384.0, (step_y - 256.0).max(512.0), 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::SunkenNecropolisCrypt => {
                // Inspired by DM-Gothic & DM-Codex: 3-level hypogeum necropolis
                let step_y = snap_grid(s * 1.2, 256.0).max(1152.0);
                let aisle_x = snap_grid(s * 0.8, 128.0).max(896.0);

                // Level 0: Grand Central Sarcophagus Hall
                rooms.push(RoomNode {
                    name: "NecropolisSarcophagusHall".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Level 1: Upper Vaulted Clerestory & Choir (North, elevated +256 UU)
                rooms.push(RoomNode {
                    name: "NecropolisUpperClerestory".to_string(),
                    center: Vec3::new(0.0, step_y, 576.0),
                    size: Vec3::new(768.0, 640.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Level -1: Sunken Bone Ossuary & Crypt (South, sunken -192 UU)
                rooms.push(RoomNode {
                    name: "NecropolisSunkenOssuary".to_string(),
                    center: Vec3::new(0.0, -step_y, 128.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // West Chantry Mortuary (elevated +64 UU)
                rooms.push(RoomNode {
                    name: "NecropolisWestChantry".to_string(),
                    center: Vec3::new(-aisle_x, 0.0, 320.0),
                    size: Vec3::new(640.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // East Embalming Vault (elevated +64 UU)
                rooms.push(RoomNode {
                    name: "NecropolisEastMortuary".to_string(),
                    center: Vec3::new(aisle_x, 0.0, 320.0),
                    size: Vec3::new(640.0, 896.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Connecting vaulted corridors (North, South, East, West)
                rooms.push(RoomNode {
                    name: "NecropolisCrypt_NorthCorr".to_string(),
                    center: Vec3::new(0.0, step_y * 0.5, 480.0),
                    size: Vec3::new(320.0, step_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NecropolisCrypt_SouthCorr".to_string(),
                    center: Vec3::new(0.0, -step_y * 0.5, 256.0),
                    size: Vec3::new(320.0, step_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NecropolisCrypt_WestCorr".to_string(),
                    center: Vec3::new(-aisle_x * 0.5, 0.0, 320.0),
                    size: Vec3::new(aisle_x * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "NecropolisCrypt_EastCorr".to_string(),
                    center: Vec3::new(aisle_x * 0.5, 0.0, 320.0),
                    size: Vec3::new(aisle_x * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::HyperFortressBastion => {
                // Inspired by AS-Overlord & CTF-Gauntlet: 3-ring concentric fortress
                let base_x = snap_grid(s * 1.5, 256.0).max(1664.0);
                let barbican_x = base_x * 0.52;
                let trench_y = 896.0_f32;

                // 1. Central Courtyard
                rooms.push(RoomNode {
                    name: "BastionCentralCourtyard".to_string(),
                    center: Vec3::new(0.0, 0.0, 256.0),
                    size: Vec3::new(1152.0, 1024.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 2. Red Citadel Keep (West Base)
                rooms.push(RoomNode {
                    name: "BastionRedCitadelKeep".to_string(),
                    center: Vec3::new(-base_x, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: true,
                });

                // 3. Blue Citadel Keep (East Base)
                rooms.push(RoomNode {
                    name: "BastionBlueCitadelKeep".to_string(),
                    center: Vec3::new(base_x, 0.0, 320.0),
                    size: Vec3::new(896.0, 896.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: true,
                });

                // 4. Red Elevated Barbican Rampart (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "BastionRedBarbican".to_string(),
                    center: Vec3::new(-barbican_x, 0.0, 384.0),
                    size: Vec3::new(512.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });

                // 5. Blue Elevated Barbican Rampart (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "BastionBlueBarbican".to_string(),
                    center: Vec3::new(barbican_x, 0.0, 384.0),
                    size: Vec3::new(512.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // 6. North Flank Trench (floor_z = -96.0)
                rooms.push(RoomNode {
                    name: "BastionNorthTrench".to_string(),
                    center: Vec3::new(0.0, trench_y, 160.0),
                    size: Vec3::new(base_x * 1.5, 384.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 7. South Flank Trench (floor_z = -96.0)
                rooms.push(RoomNode {
                    name: "BastionSouthTrench".to_string(),
                    center: Vec3::new(0.0, -trench_y, 160.0),
                    size: Vec3::new(base_x * 1.5, 384.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Gate corridors linking Keeps to Barbicans and Courtyard
                rooms.push(RoomNode {
                    name: "BastionGate_RedInner".to_string(),
                    center: Vec3::new(-barbican_x * 0.5, 0.0, 320.0),
                    size: Vec3::new(barbican_x + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BastionGate_BlueInner".to_string(),
                    center: Vec3::new(barbican_x * 0.5, 0.0, 320.0),
                    size: Vec3::new(barbican_x + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BastionGate_RedOuter".to_string(),
                    center: Vec3::new((-base_x - barbican_x) * 0.5, 0.0, 320.0),
                    size: Vec3::new((base_x - barbican_x).abs() + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BastionGate_BlueOuter".to_string(),
                    center: Vec3::new((base_x + barbican_x) * 0.5, 0.0, 320.0),
                    size: Vec3::new((base_x - barbican_x).abs() + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BastionTrenchTunnel_North".to_string(),
                    center: Vec3::new(0.0, trench_y * 0.5, 208.0),
                    size: Vec3::new(320.0, trench_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BastionTrenchTunnel_South".to_string(),
                    center: Vec3::new(0.0, -trench_y * 0.5, 208.0),
                    size: Vec3::new(320.0, trench_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::DeepSeaAbyssStation => {
                // Inspired by CTF-Hydro16 & CTF-November: Oceanic research trench
                let base_x = snap_grid(s * 1.4, 256.0).max(1536.0);
                let flume_y = 896.0_f32;

                // 1. Central Submerged Observation Dome
                rooms.push(RoomNode {
                    name: "AbyssObservationDome".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // 2. Red Pressure Habitation Wing (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "AbyssRedPressureHab".to_string(),
                    center: Vec3::new(-base_x, 0.0, 448.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });

                // 3. Blue Pressure Habitation Wing (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "AbyssBluePressureHab".to_string(),
                    center: Vec3::new(base_x, 0.0, 448.0),
                    size: Vec3::new(896.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // 4. North Drainage Siphon Conduit (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "AbyssHydroFlume_North".to_string(),
                    center: Vec3::new(0.0, flume_y, 320.0),
                    size: Vec3::new(base_x * 1.4, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // 5. South Drainage Siphon Conduit (floor_z = 0.0)
                rooms.push(RoomNode {
                    name: "AbyssHydroFlume_South".to_string(),
                    center: Vec3::new(0.0, -flume_y, 192.0),
                    size: Vec3::new(base_x * 1.4, 320.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Decompression lock corridors
                rooms.push(RoomNode {
                    name: "AbyssAirlock_Red".to_string(),
                    center: Vec3::new(-base_x * 0.5, 0.0, 384.0),
                    size: Vec3::new(base_x * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(0),
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "AbyssAirlock_Blue".to_string(),
                    center: Vec3::new(base_x * 0.5, 0.0, 384.0),
                    size: Vec3::new(base_x * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: Some(1),
                    is_octagonal: false,
                });

                // Vertical drainage conduit shafts connecting Observation Dome to Flumes
                rooms.push(RoomNode {
                    name: "AbyssFlumeConnector_North".to_string(),
                    center: Vec3::new(0.0, flume_y * 0.5, 320.0),
                    size: Vec3::new(320.0, flume_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "AbyssFlumeConnector_South".to_string(),
                    center: Vec3::new(0.0, -flume_y * 0.5, 256.0),
                    size: Vec3::new(320.0, flume_y * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::IndustrialRefineryPlex => {
                // Inspired by DM-Pressure & DM-Turbine: 4-tier vertical processing complex
                let step = snap_grid(s * 1.1, 256.0).max(1152.0);

                // Tier 0: Ground Centrifuge Hall (floor_z = 0.0)
                rooms.push(RoomNode {
                    name: "RefineryCentrifugeCore".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Tier -1: Lower Sump Vat Drain (South, floor_z = -160.0)
                rooms.push(RoomNode {
                    name: "RefinerySumpDrain".to_string(),
                    center: Vec3::new(0.0, -step, 96.0),
                    size: Vec3::new(896.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: true,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Tier 1: Distillation Tower (North, floor_z = 192.0)
                rooms.push(RoomNode {
                    name: "RefineryDistillationTower".to_string(),
                    center: Vec3::new(0.0, step, 576.0),
                    size: Vec3::new(768.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // West Heavy Pump Bay (floor_z = 64.0)
                rooms.push(RoomNode {
                    name: "RefineryWestPumpBay".to_string(),
                    center: Vec3::new(-step, 0.0, 320.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // East Generator Turbine Wing (floor_z = 64.0)
                rooms.push(RoomNode {
                    name: "RefineryEastGeneratorWing".to_string(),
                    center: Vec3::new(step, 0.0, 320.0),
                    size: Vec3::new(768.0, 768.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Ducts & Catwalks connecting tiers
                rooms.push(RoomNode {
                    name: "RefineryConduit_North".to_string(),
                    center: Vec3::new(0.0, step * 0.5, 480.0),
                    size: Vec3::new(288.0, step * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RefineryConduit_South".to_string(),
                    center: Vec3::new(0.0, -step * 0.5, 256.0),
                    size: Vec3::new(288.0, step * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RefineryConduit_West".to_string(),
                    center: Vec3::new(-step * 0.5, 0.0, 320.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "RefineryConduit_East".to_string(),
                    center: Vec3::new(step * 0.5, 0.0, 320.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::SubterraneanMagmaForge => {
                // Inspired by DM-Fetid & CTF-LavaGiant: Volcanic forge caldera with radial galleries
                let step = snap_grid(s * 1.2, 256.0).max(1280.0);

                // Central Basalt Caldera (floor_z = -128.0)
                rooms.push(RoomNode {
                    name: "CalderaForgeCrucible".to_string(),
                    center: Vec3::new(0.0, 0.0, 256.0),
                    size: Vec3::new(1152.0, 1152.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: true,
                    team_owner: None,
                    is_octagonal: true,
                });

                // North High Casting Gallery (floor_z = 128.0)
                rooms.push(RoomNode {
                    name: "ForgeNorthCastingGallery".to_string(),
                    center: Vec3::new(0.0, step, 448.0),
                    size: Vec3::new(768.0, 640.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // South Slag Flume (floor_z = 0.0)
                rooms.push(RoomNode {
                    name: "ForgeSouthSlagFlume".to_string(),
                    center: Vec3::new(0.0, -step, 256.0),
                    size: Vec3::new(768.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // West Ingot Vault (floor_z = -64.0)
                rooms.push(RoomNode {
                    name: "ForgeWestIngotVault".to_string(),
                    center: Vec3::new(-step, 0.0, 224.0),
                    size: Vec3::new(640.0, 768.0, 576.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // East Obsidian Smelter (floor_z = 64.0)
                rooms.push(RoomNode {
                    name: "ForgeEastSmelter".to_string(),
                    center: Vec3::new(step, 0.0, 352.0),
                    size: Vec3::new(640.0, 768.0, 576.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Arched obsidian flume tunnels
                rooms.push(RoomNode {
                    name: "ForgeTunnel_North".to_string(),
                    center: Vec3::new(0.0, step * 0.5, 384.0),
                    size: Vec3::new(288.0, step * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ForgeTunnel_South".to_string(),
                    center: Vec3::new(0.0, -step * 0.5, 192.0),
                    size: Vec3::new(288.0, step * 0.5 + 256.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ForgeTunnel_West".to_string(),
                    center: Vec3::new(-step * 0.5, 0.0, 256.0),
                    size: Vec3::new(step * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "ForgeTunnel_East".to_string(),
                    center: Vec3::new(step * 0.5, 0.0, 288.0),
                    size: Vec3::new(step * 0.5 + 256.0, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::OrbitalBiodomeArray => {
                // Inspired by CTF-Face & DM-SpaceNoxx: 5 pressurized geodesic biodomes in deep space
                let step = snap_grid(s * 1.2, 256.0).max(1280.0);

                // Central Hydroponic Nexus (floor_z = 0.0)
                rooms.push(RoomNode {
                    name: "BiodomeNexusHydroponics".to_string(),
                    center: Vec3::new(0.0, 0.0, 384.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // North Arboretum Spire (floor_z = 192.0)
                rooms.push(RoomNode {
                    name: "BiodomeNorthArboretum".to_string(),
                    center: Vec3::new(0.0, step, 576.0),
                    size: Vec3::new(768.0, 768.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // South Cryogenics Pod (floor_z = -128.0)
                rooms.push(RoomNode {
                    name: "BiodomeSouthCryogenics".to_string(),
                    center: Vec3::new(0.0, -step, 192.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // West Xenobiology Lab (floor_z = 64.0)
                rooms.push(RoomNode {
                    name: "BiodomeWestXenobiology".to_string(),
                    center: Vec3::new(-step, 0.0, 384.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // East Life Support Station (floor_z = 64.0)
                rooms.push(RoomNode {
                    name: "BiodomeEastLifeSupport".to_string(),
                    center: Vec3::new(step, 0.0, 384.0),
                    size: Vec3::new(768.0, 768.0, 640.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Pressurized transparent vacuum skyways
                rooms.push(RoomNode {
                    name: "BioSkyway_North".to_string(),
                    center: Vec3::new(0.0, step * 0.5, 480.0),
                    size: Vec3::new(288.0, step * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BioSkyway_South".to_string(),
                    center: Vec3::new(0.0, -step * 0.5, 256.0),
                    size: Vec3::new(288.0, step * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BioSkyway_West".to_string(),
                    center: Vec3::new(-step * 0.5, 0.0, 352.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "BioSkyway_East".to_string(),
                    center: Vec3::new(step * 0.5, 0.0, 352.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::DoubleHelixAccelerator => {
                // Inspired by CTF-Nucleus & DM-Fractal: Soaring vertical accelerator column
                let dist = snap_grid(s * 1.0, 256.0).max(1024.0);

                // Central Accelerator Containment Shaft (floor_z = 0.0, height = 1024.0)
                rooms.push(RoomNode {
                    name: "AcceleratorCoreShaft".to_string(),
                    center: Vec3::new(0.0, 0.0, 512.0),
                    size: Vec3::new(896.0, 896.0, 1024.0),
                    is_corridor: false,
                    sky_ceiling: cfg.toggles.include_skybox,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Helix Alpha Tiers (West & North, rising from 0 to 512 UU)
                rooms.push(RoomNode {
                    name: "HelixAlpha_Base".to_string(),
                    center: Vec3::new(-dist, 0.0, 256.0),
                    size: Vec3::new(640.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "HelixAlpha_Zenith".to_string(),
                    center: Vec3::new(0.0, dist, 768.0),
                    size: Vec3::new(640.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Helix Beta Tiers (East & South, rising from 128 to 640 UU)
                rooms.push(RoomNode {
                    name: "HelixBeta_Base".to_string(),
                    center: Vec3::new(dist, 0.0, 384.0),
                    size: Vec3::new(640.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "HelixBeta_Zenith".to_string(),
                    center: Vec3::new(0.0, -dist, 896.0),
                    size: Vec3::new(640.0, 640.0, 512.0),
                    is_corridor: false,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: true,
                });

                // 4 crossover radial bridges across the central core
                rooms.push(RoomNode {
                    name: "HelixRadialBridge_AlphaWest".to_string(),
                    center: Vec3::new(-dist * 0.5, 0.0, 320.0),
                    size: Vec3::new(dist * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "HelixRadialBridge_AlphaNorth".to_string(),
                    center: Vec3::new(0.0, dist * 0.5, 640.0),
                    size: Vec3::new(288.0, dist * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "HelixRadialBridge_BetaEast".to_string(),
                    center: Vec3::new(dist * 0.5, 0.0, 448.0),
                    size: Vec3::new(dist * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "HelixRadialBridge_BetaSouth".to_string(),
                    center: Vec3::new(0.0, -dist * 0.5, 768.0),
                    size: Vec3::new(288.0, dist * 0.5, 384.0),
                    is_corridor: true,
                    sky_ceiling: false,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }

            WizardArchetype::SkylineRooftopHighway => {
                // Inspired by DM-Morpheus & DM-Phobos: Staggered multi-skyscraper rooftop arena
                let step = snap_grid(s * 1.1, 256.0).max(1152.0);

                // Skyscraper 1 (Low Rooftop, floor_z = 0.0)
                rooms.push(RoomNode {
                    name: "SkylineRooftop_Alpha".to_string(),
                    center: Vec3::new(-step, 0.0, 384.0),
                    size: Vec3::new(896.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Skyscraper 2 (Mid Central Rooftop, floor_z = 192.0)
                rooms.push(RoomNode {
                    name: "SkylineRooftop_Beta".to_string(),
                    center: Vec3::new(0.0, 0.0, 576.0),
                    size: Vec3::new(1024.0, 1024.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: cfg.toggles.include_environmental_hazards,
                    team_owner: None,
                    is_octagonal: true,
                });

                // Skyscraper 3 (High Penthouse Helipad, floor_z = 384.0)
                rooms.push(RoomNode {
                    name: "SkylineRooftop_Gamma".to_string(),
                    center: Vec3::new(step, 0.0, 768.0),
                    size: Vec3::new(896.0, 896.0, 768.0),
                    is_corridor: false,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Mag-rail skybridges connecting rooftops
                rooms.push(RoomNode {
                    name: "SkylineMagBridge_AlphaBeta".to_string(),
                    center: Vec3::new(-step * 0.5, 0.0, 480.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
                rooms.push(RoomNode {
                    name: "SkylineMagBridge_BetaGamma".to_string(),
                    center: Vec3::new(step * 0.5, 0.0, 672.0),
                    size: Vec3::new(step * 0.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // North High Service Gantry (floor_z = 256.0)
                rooms.push(RoomNode {
                    name: "SkylineNorthServiceGantry".to_string(),
                    center: Vec3::new(0.0, 768.0, 448.0),
                    size: Vec3::new(step * 1.5, 288.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });

                // Service conduit connecting central Skyscraper Beta to the North Service Gantry
                rooms.push(RoomNode {
                    name: "SkylineGantryConduit_Beta".to_string(),
                    center: Vec3::new(0.0, 576.0, 480.0),
                    size: Vec3::new(320.0, 384.0, 384.0),
                    is_corridor: true,
                    sky_ceiling: true,
                    has_hazard_pit: false,
                    team_owner: None,
                    is_octagonal: false,
                });
            }
        },
        WizardLayoutStrategy::ProceduralSeed => {
            let mut rng = SplitMix64::new(cfg.seed);
            let target_rooms = cfg.procedural_room_count.clamp(3, 12);
            let max_tiers = cfg.procedural_vertical_tiers.clamp(1, 3) as i32;
            let force_sym = cfg.game_mode == WizardGameMode::CaptureTheFlag
                || cfg.symmetry_mode != WizardSymmetryMode::None;

            // Central hub at origin
            let hub_w = snap_grid(768.0 + (rng.range_i32(0, 2) as f32) * 256.0, 256.0);
            let hub_l = snap_grid(768.0 + (rng.range_i32(0, 2) as f32) * 256.0, 256.0);
            rooms.push(RoomNode {
                name: "ProcHub_0".to_string(),
                center: Vec3::new(0.0, 0.0, procedural_room_height(cfg, 0, HeightRole::Hub) * 0.5),
                size: Vec3::new(hub_w, hub_l, procedural_room_height(cfg, 0, HeightRole::Hub)),
                is_corridor: false,
                sky_ceiling: cfg.toggles.include_skybox && rng.next_f32() > 0.35,
                has_hazard_pit: cfg.toggles.include_environmental_hazards,
                team_owner: None,
                is_octagonal: false,
            });

            // Candidate grid slots around origin (step 1152 UU so rooms never overlap illegally)
            let slot_step = 1152.0_f32;
            let mut candidate_slots = vec![
                (-1, 0),
                (1, 0),
                (0, 1),
                (0, -1),
                (-1, 1),
                (1, -1),
                (-1, -1),
                (1, 1),
                (-2, 0),
                (2, 0),
                (0, 2),
                (0, -2),
            ];
            // Deterministic Fisher-Yates shuffle on pairs if symmetrical, or individual slots if free-flow
            if force_sym {
                let mut pairs: Vec<((i32, i32), (i32, i32))> = if cfg.symmetry_mode == WizardSymmetryMode::MirrorX {
                    // Under MirrorX reflection across X=0 (pos_b.x = -pos_a.x, pos_b.y = pos_a.y),
                    // slots with X=0 cannot be used for team wing pairs because (-0, y) == (0, y),
                    // which would cause Red and Blue team wings/bases to collapse into the exact same room.
                    // All candidate pairs must have non-zero X (X < 0 for slot_a, reflecting to -X > 0).
                    vec![
                        ((-1, 0), (1, 0)),
                        ((-1, 1), (1, 1)),
                        ((-1, -1), (1, -1)),
                        ((-2, 0), (2, 0)),
                        ((-2, 1), (2, 1)),
                        ((-2, -1), (2, -1)),
                    ]
                } else {
                    vec![
                        ((-1, 0), (1, 0)),
                        ((0, 1), (0, -1)),
                        ((-1, 1), (1, -1)),
                        ((-1, -1), (1, 1)),
                        ((-2, 0), (2, 0)),
                        ((0, 2), (0, -2)),
                    ]
                };
                for i in (1..pairs.len()).rev() {
                    let j = (rng.next_u64() as usize) % (i + 1);
                    pairs.swap(i, j);
                }
                let pair_count = ((target_rooms - 1) / 2).clamp(1, pairs.len());
                for p_idx in 0..pair_count {
                    let (slot_a, _slot_b) = pairs[p_idx];
                    let rw = snap_grid(640.0 + (rng.range_i32(0, 2) as f32) * 128.0, 128.0);
                    let rl = snap_grid(640.0 + (rng.range_i32(0, 2) as f32) * 128.0, 128.0);
                    let rh = procedural_room_height(cfg, p_idx + 1, if p_idx == 0 { HeightRole::TeamBase } else { HeightRole::Room });
                    let tier = if p_idx == 0 {
                        0
                    } else {
                        rng.range_i32(0, max_tiers - 1)
                    };
                    let z_center = rh * 0.5 + (tier as f32) * 128.0;

                    let pos_a = Vec3::new(slot_a.0 as f32 * slot_step, slot_a.1 as f32 * slot_step, z_center);
                    let pos_b = if cfg.symmetry_mode == WizardSymmetryMode::MirrorX {
                        Vec3::new(-pos_a.x, pos_a.y, z_center)
                    } else {
                        Vec3::new(-pos_a.x, -pos_a.y, z_center)
                    };

                    rooms.push(RoomNode {
                        name: format!("ProcWingRed_{p_idx}"),
                        center: pos_a,
                        size: Vec3::new(rw, rl, rh),
                        is_corridor: false,
                        sky_ceiling: cfg.toggles.include_skybox && p_idx == 0,
                        has_hazard_pit: false,
                        team_owner: if (cfg.game_mode == WizardGameMode::CaptureTheFlag
                            || cfg.game_mode == WizardGameMode::TeamDeathMatch
                            || cfg.game_mode == WizardGameMode::Assault)
                            && p_idx == 0
                        {
                            Some(0)
                        } else {
                            None
                        },
                        is_octagonal: false,
                    });
                    rooms.push(RoomNode {
                        name: format!("ProcWingBlue_{p_idx}"),
                        center: pos_b,
                        size: Vec3::new(rw, rl, rh),
                        is_corridor: false,
                        sky_ceiling: cfg.toggles.include_skybox && p_idx == 0,
                        has_hazard_pit: false,
                        team_owner: if (cfg.game_mode == WizardGameMode::CaptureTheFlag
                            || cfg.game_mode == WizardGameMode::TeamDeathMatch
                            || cfg.game_mode == WizardGameMode::Assault)
                            && p_idx == 0
                        {
                            Some(1)
                        } else {
                            None
                        },
                        is_octagonal: false,
                    });
                }
            }
 else {
                for i in (1..candidate_slots.len()).rev() {
                    let j = (rng.next_u64() as usize) % (i + 1);
                    candidate_slots.swap(i, j);
                }
                for (idx, &(gx, gy)) in candidate_slots.iter().take(target_rooms - 1).enumerate() {
                    let rw = snap_grid(640.0 + (rng.range_i32(0, 2) as f32) * 128.0, 128.0);
                    let rl = snap_grid(640.0 + (rng.range_i32(0, 2) as f32) * 128.0, 128.0);
                    let team_base = idx < 2
                        && matches!(
                            cfg.game_mode,
                            WizardGameMode::CaptureTheFlag | WizardGameMode::TeamDeathMatch | WizardGameMode::Assault
                        );
                    let rh = procedural_room_height(cfg, idx + 1, if team_base { HeightRole::TeamBase } else { HeightRole::Room });
                    let tier = rng.range_i32(0, max_tiers - 1);
                    let z_center = rh * 0.5 + (tier as f32) * 128.0;
                    rooms.push(RoomNode {
                        name: format!("ProcChamber_{}", idx + 1),
                        center: Vec3::new(gx as f32 * slot_step, gy as f32 * slot_step, z_center),
                        size: Vec3::new(rw, rl, rh),
                        is_corridor: false,
                        sky_ceiling: cfg.toggles.include_skybox && (idx % 2 == 0),
                        has_hazard_pit: false,
                        team_owner: if cfg.game_mode == WizardGameMode::CaptureTheFlag
                            || cfg.game_mode == WizardGameMode::TeamDeathMatch
                            || cfg.game_mode == WizardGameMode::Assault
                        {
                            if idx == 0 {
                                Some(0)
                            } else if idx == 1 {
                                Some(1)
                            } else {
                                None
                            }
                        } else {
                            None
                        },
                        is_octagonal: false,
                    });
                }
            }


            // Connect rooms with MST + loop corridors (preserving exact symmetry when force_sym is enabled)
            let main_rooms: Vec<RoomNode> = rooms.clone();
            let n = main_rooms.len();
            let sym_partner = |idx: usize| -> usize {
                if !force_sym || idx == 0 {
                    idx
                } else if idx % 2 == 1 {
                    (idx + 1).min(n - 1)
                } else {
                    idx - 1
                }
            }
;
            let mut connected = vec![false; n];
            connected[0] = true;
            let mut edges: Vec<(usize, usize)> = Vec::new();

            while connected.iter().any(|&c| !c) {
                let mut best_edge = None;
                let mut best_dist = f32::MAX;
                for i in 0..n {
                    if !connected[i] {
                        continue;
                    }
                    for j in 0..n {
                        if connected[j] {
                            continue;
                        }
                        let d = main_rooms[i].center.distance(main_rooms[j].center);
                        if d < best_dist {
                            best_dist = d;
                            best_edge = Some((i, j));
                        }
                    }
                }
                if let Some((u, v)) = best_edge {
                    connected[v] = true;
                    edges.push((u, v));
                    if force_sym {
                        let su = sym_partner(u);
                        let sv = sym_partner(v);
                        connected[sv] = true;
                        if (su, sv) != (u, v)
                            && !edges.contains(&(su, sv))
                            && !edges.contains(&(sv, su))
                        {
                            edges.push((su, sv));
                        }
                    }
                } else {
                    break;
                }
            }


            let tree_edges = edges.len();
            // Add extra loop edges so players/bots never get trapped in dead-ends
            for i in 1..n {
                for j in (i + 1)..n {
                    if edges.contains(&(i, j)) || edges.contains(&(j, i)) {
                        continue;
                    }
                    let d = main_rooms[i].center.distance(main_rooms[j].center);
                    if d <= slot_step * 1.55 && rng.next_f32() <= cfg.procedural_loop_ratio.max(0.35) {
                        edges.push((i, j));
                        if force_sym {
                            let si = sym_partner(i);
                            let sj = sym_partner(j);
                            if (si, sj) != (i, j)
                                && !edges.contains(&(si, sj))
                                && !edges.contains(&(sj, si))
                            {
                                edges.push((si, sj));
                            }
                        }
                    }
                }
            }

            let random_edges = edges.len() - tree_edges;
            // Enforce circulation invariant: every combat room must have degree >= 2 (zero dead ends!)
            if n >= 3 {
                for i in 0..n {
                    let degree = edges.iter().filter(|(u, v)| *u == i || *v == i).count();
                    if degree < 2 {
                        let mut candidates: Vec<(usize, f32)> = (0..n)
                            .filter(|&j| j != i && !edges.contains(&(i, j)) && !edges.contains(&(j, i)))
                            .map(|j| (j, main_rooms[i].center.distance(main_rooms[j].center)))
                            .collect();
                        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                        if let Some((best_j, _)) = candidates.first() {
                            edges.push((i, *best_j));
                            if force_sym {
                                let si = sym_partner(i);
                                let sj = sym_partner(*best_j);
                                if (si, sj) != (i, *best_j)
                                    && !edges.contains(&(si, sj))
                                    && !edges.contains(&(sj, si))
                                {
                                    edges.push((si, sj));
                                }
                            }
                        }
                    }
                }
            }


            if include_graph {
                let minimum_degree_edges = edges.len() - tree_edges - random_edges;
                let mut degrees = vec![0usize; n];
                let mut neighbors = vec![Vec::new(); n];
                for &(u, v) in &edges {
                    degrees[u] += 1;
                    degrees[v] += 1;
                    neighbors[u].push(v);
                    neighbors[v].push(u);
                }
                let mut seen = vec![false; n];
                let mut components = 0;
                for start in 0..n {
                    if seen[start] { continue; }
                    components += 1;
                    seen[start] = true;
                    let mut stack = vec![start];
                    while let Some(node) = stack.pop() {
                        for &next in &neighbors[node] {
                            if !seen[next] { seen[next] = true; stack.push(next); }
                        }
                    }
                }
                procedural_graph = Some(WizardProceduralGraphSummary {
                    room_count: n,
                    tree_edges,
                    random_edges,
                    minimum_degree_edges,
                    components,
                    independent_cycles: (edges.len() + components).saturating_sub(n),
                    minimum_degree: degrees.into_iter().min().unwrap_or(0),
                    effective_extra_probability: cfg.procedural_loop_ratio.max(0.35).min(1.0),
                    edges: edges.iter().map(|&(u, v)| (main_rooms[u].name.clone(), main_rooms[v].name.clone())).collect(),
                });
            }

            // Carve orthogonal L-shaped or straight corridors for each edge
            for (e_idx, (u, v)) in edges.into_iter().enumerate() {
                let a = &main_rooms[u];
                let b = &main_rooms[v];
                rooms.extend(procedural_corridor_nodes(e_idx, a, b, cfg));
            }

        }
    }

    (rooms, procedural_graph)
}

/// Internal representation of an in-wizard teleporter node used during auditing and topology routing.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardTeleportEntry {
    pub name: String,
    pub tag: String,
    pub url: String,
    pub location: Vec3,
    pub team: Option<u8>,
    pub is_custom: bool,
    pub custom_index: Option<usize>,
}

/// Collects all planned teleporters (both procedural archetype pairs and user custom markers)
/// taking suppression lists into account.
pub fn collect_wizard_teleporters(cfg: &MapWizardConfig) -> Vec<WizardTeleportEntry> {
    let mut entries = Vec::new();
    let nodes = build_room_nodes(cfg);
    let non_corridor_rooms: Vec<&RoomNode> = nodes.iter().filter(|r| !r.is_corridor).collect();

    // 1. Procedural Archetype Teleporters
    if cfg.toggles.include_teleporters && non_corridor_rooms.len() >= 2 {
        let specs: Vec<(&str, &str, &str, usize, Option<u8>)> = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            if cfg.archetype == WizardArchetype::FacingWorldsOrbitalSpire {
                let red_sanctum = non_corridor_rooms.iter().position(|r| r.name == "RedFlagSanctum");
                let red_roost = non_corridor_rooms.iter().position(|r| r.name == "RedSniperRoost");
                let blue_sanctum = non_corridor_rooms.iter().position(|r| r.name == "BlueFlagSanctum");
                let blue_roost = non_corridor_rooms.iter().position(|r| r.name == "BlueSniperRoost");
                let mut v = Vec::new();
                if let (Some(rs), Some(rr)) = (red_sanctum, red_roost) {
                    v.push(("Tele_RedBase", "Tele_RedBase", "Tele_RedRoost", rs, Some(0u8)));
                    v.push(("Tele_RedRoost", "Tele_RedRoost", "Tele_RedBase", rr, Some(0u8)));
                }
                if let (Some(bs), Some(br)) = (blue_sanctum, blue_roost) {
                    v.push(("Tele_BlueBase", "Tele_BlueBase", "Tele_BlueRoost", bs, Some(1u8)));
                    v.push(("Tele_BlueRoost", "Tele_BlueRoost", "Tele_BlueBase", br, Some(1u8)));
                }
                v
            } else {
                let neutral_indices: Vec<usize> = non_corridor_rooms
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.team_owner.is_none())
                    .map(|(i, _)| i)
                    .collect();
                let candidate_neutrals: Vec<usize> = neutral_indices
                    .iter()
                    .copied()
                    .filter(|&i| i != 0)
                    .collect();
                let effective_neutrals = if candidate_neutrals.len() >= 2 {
                    candidate_neutrals
                } else {
                    neutral_indices
                };
                if effective_neutrals.len() >= 2 {
                    let na = effective_neutrals[0];
                    let nb = effective_neutrals[effective_neutrals.len() - 1];
                    vec![
                        ("WizardTeleA", "WizardTeleA", "WizardTeleB", na, None),
                        ("WizardTeleB", "WizardTeleB", "WizardTeleA", nb, None),
                    ]
                } else if non_corridor_rooms.len() >= 3 {
                    let na = 0;
                    let nb = 0;
                    vec![
                        ("WizardTeleA", "WizardTeleA", "WizardTeleB", na, None),
                        ("WizardTeleB", "WizardTeleB", "WizardTeleA", nb, None),
                    ]
                } else {
                    Vec::new()
                }
            }
        } else {
            let idx_a = if non_corridor_rooms.len() >= 3 { 1 } else { 0 };
            let idx_b = non_corridor_rooms.len() - 1;
            vec![
                ("WizardTeleA", "WizardTeleA", "WizardTeleB", idx_a, non_corridor_rooms[idx_a].team_owner),
                ("WizardTeleB", "WizardTeleB", "WizardTeleA", idx_b, non_corridor_rooms[idx_b].team_owner),
            ]
        };

        let is_same_room = specs.len() == 2 && specs[0].3 == specs[1].3;
        for (s_idx, (name, tag, url, r_idx, team)) in specs.into_iter().enumerate() {
            let is_suppressed = cfg.suppressed_markers.iter().any(|s| {
                s == name || s == tag || s.starts_with(name)
            });
            if !is_suppressed {
                let r = non_corridor_rooms[r_idx];
                let (tx, ty) = if is_same_room {
                    let offset = (r.size.x * 0.32).clamp(180.0, 320.0);
                    if s_idx == 0 {
                        (r.center.x - offset, r.center.y)
                    } else {
                        (r.center.x + offset, r.center.y)
                    }
                } else {
                    (
                        r.center.x - (r.size.x * 0.18).min(160.0),
                        r.center.y + (r.size.y * 0.08).min(64.0),
                    )
                };
                let loc = Vec3::new(tx, ty, r.floor_z() + 48.0);
                entries.push(WizardTeleportEntry {
                    name: name.to_string(),
                    tag: tag.to_string(),
                    url: url.to_string(),
                    location: loc,
                    team,
                    is_custom: false,
                    custom_index: None,
                });
            }
        }
    }

    // 2. User Custom Markers
    for (idx, cm) in cfg.custom_markers.iter().enumerate() {
        if cm.kind == "Teleporter" {
            let is_suppressed = cfg.suppressed_markers.iter().any(|s| s == &cm.label || s == &cm.id);
            if !is_suppressed {
                let tag = if !cm.teleporter_tag.is_empty() {
                    cm.teleporter_tag.clone()
                } else {
                    cm.label.clone()
                };
                entries.push(WizardTeleportEntry {
                    name: cm.label.clone(),
                    tag,
                    url: cm.teleporter_url.clone(),
                    location: cm.location,
                    team: cm.team,
                    is_custom: true,
                    custom_index: Some(idx),
                });
            }
        }
    }

    entries
}

/// Evaluates network integrity, presence invariants, and reachability for all teleporters.
pub fn audit_wizard_teleporters(cfg: &MapWizardConfig) -> WizardTeleportAuditSummary {
    let entries = collect_wizard_teleporters(cfg);
    let mut valid_links = Vec::new();
    let mut issues = Vec::new();

    if entries.is_empty() {
        return WizardTeleportAuditSummary {
            teleporter_count: 0,
            valid_links,
            issues,
            is_healthy: true,
        };
    }

    // Rule 1: Cardinality (at least 2 if present)
    if entries.len() == 1 {
        issues.push(WizardTeleportIssue {
            severity: WizardTeleportIssueSeverity::Error,
            kind: WizardTeleportIssueKind::LoneTeleporter,
            teleporter_name: entries[0].name.clone(),
            target_url: entries[0].url.clone(),
            message: format!(
                "Lone Teleporter '{}': teleporters must exist in connected pairs of 2 or more.",
                entries[0].name
            ),
        });
    }

    // Rule 2: Destination validity and loop safety
    for (i, entry) in entries.iter().enumerate() {
        let trimmed_url = entry.url.trim();
        if trimmed_url.is_empty() {
            issues.push(WizardTeleportIssue {
                severity: WizardTeleportIssueSeverity::Error,
                kind: WizardTeleportIssueKind::MissingDestination,
                teleporter_name: entry.name.clone(),
                target_url: String::new(),
                message: format!("Teleporter '{}' has no destination URL specified.", entry.name),
            });
            continue;
        }

        let mut matches = Vec::new();
        for (j, other) in entries.iter().enumerate() {
            if i != j && other.tag.eq_ignore_ascii_case(trimmed_url) {
                matches.push(other);
            }
        }

        if matches.is_empty() {
            issues.push(WizardTeleportIssue {
                severity: WizardTeleportIssueSeverity::Error,
                kind: WizardTeleportIssueKind::UnresolvedDestination,
                teleporter_name: entry.name.clone(),
                target_url: entry.url.clone(),
                message: format!(
                    "Teleporter '{}' targets URL '{}' which does not match any teleporter Tag in the map.",
                    entry.name, entry.url
                ),
            });
        } else {
            for dest in matches {
                let dist = (dest.location - entry.location).length();
                let is_bidirectional = dest.url.trim().eq_ignore_ascii_case(&entry.tag) && !entry.tag.is_empty();

                if dist < 96.0 {
                    issues.push(WizardTeleportIssue {
                        severity: WizardTeleportIssueSeverity::Error,
                        kind: WizardTeleportIssueKind::LoopHazard,
                        teleporter_name: entry.name.clone(),
                        target_url: entry.url.clone(),
                        message: format!(
                            "Loop Hazard: Teleporters '{}' and '{}' are within {:.1} UU (causes instant re-teleport loop/crash).",
                            entry.name, dest.name, dist
                        ),
                    });
                }

                valid_links.push(WizardTeleportLinkPreview {
                    source_name: entry.name.clone(),
                    dest_name: dest.name.clone(),
                    source_loc: entry.location,
                    dest_loc: dest.location,
                    is_bidirectional,
                    distance: dist,
                });
            }
        }
    }

    // Rule 3: Unreachable / orphan teleporters
    if entries.len() > 1 {
        for entry in &entries {
            if !entry.tag.trim().is_empty() {
                let is_targeted = entries.iter().any(|other| other.name != entry.name && other.url.trim().eq_ignore_ascii_case(&entry.tag));
                if !is_targeted {
                    issues.push(WizardTeleportIssue {
                        severity: WizardTeleportIssueSeverity::Warning,
                        kind: WizardTeleportIssueKind::UnreachableExit,
                        teleporter_name: entry.name.clone(),
                        target_url: entry.url.clone(),
                        message: format!(
                            "Unreachable Teleporter '{}': defines Tag '{}' but is never targeted by any other teleporter.",
                            entry.name, entry.tag
                        ),
                    });
                }
            }
        }
    }

    let is_healthy = issues.is_empty();
    WizardTeleportAuditSummary {
        teleporter_count: entries.len(),
        valid_links,
        issues,
        is_healthy,
    }
}

/// Automatically repairs any teleporter anomalies in `cfg`:
/// - Spawns matching partner for a lone teleporter.
/// - Pairs up disconnected / unlinked teleporters into reciprocal bidirectional links.
/// - Spawns companion if teleporter count is odd.
/// - Separates overlapping teleporters to safe distances (>= 256 UU).
pub fn auto_fix_wizard_teleporters(cfg: &mut MapWizardConfig) -> Vec<String> {
    let mut log = Vec::new();
    let audit = audit_wizard_teleporters(cfg);
    if audit.is_healthy {
        return log;
    }

    let entries = collect_wizard_teleporters(cfg);

    // Case 1: Exactly 1 teleporter present (LoneTeleporter)
    if entries.len() == 1 {
        let lone = &entries[0];

        // 1a. If the lone teleporter is an archetype teleporter whose partner was suppressed in suppressed_markers,
        // unsuppressing the partner instantly restores the clean pair!
        if !lone.url.trim().is_empty() && cfg.suppressed_markers.iter().any(|s| s == lone.url.trim()) {
            let restored = lone.url.trim().to_string();
            cfg.suppressed_markers.retain(|s| s != &restored);
            log.push(format!("Restored suppressed archetype teleporter '{}' to reconnect with '{}'.", restored, lone.name));
            return log;
        }

        // 1b. Otherwise, calculate partner location and spawn a companion custom marker
        let nodes = build_room_nodes(cfg);
        let non_corridor: Vec<&RoomNode> = nodes.iter().filter(|r| !r.is_corridor).collect();

        let partner_loc = if non_corridor.len() >= 2 {
            let furthest_room = non_corridor
                .iter()
                .max_by(|a, b| {
                    let d_a = (a.center - lone.location).length();
                    let d_b = (b.center - lone.location).length();
                    d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            Vec3::new(
                furthest_room.center.x,
                furthest_room.center.y,
                furthest_room.floor_z() + 48.0,
            )
        } else if let Some(single_r) = non_corridor.first() {
            let dx = lone.location.x - single_r.center.x;
            let dy = lone.location.y - single_r.center.y;
            let offset_x = if dx.abs() > 32.0 { -dx.signum() * (single_r.size.x * 0.3).min(240.0) } else { 180.0 };
            let offset_y = if dy.abs() > 32.0 { -dy.signum() * (single_r.size.y * 0.3).min(240.0) } else { 180.0 };
            Vec3::new(
                single_r.center.x + offset_x,
                single_r.center.y + offset_y,
                single_r.floor_z() + 48.0,
            )
        } else {
            lone.location + Vec3::new(512.0, 512.0, 0.0)
        };

        let lone_tag = if !lone.tag.is_empty() {
            lone.tag.clone()
        } else {
            format!("{}_Tag", lone.name)
        };
        let partner_tag = if !lone.url.trim().is_empty() {
            lone.url.trim().to_string()
        } else {
            format!("{}_Partner", lone_tag)
        };
        let partner_label = format!("{}_Partner", lone.name);

        if let Some(c_idx) = lone.custom_index {
            if let Some(cm) = cfg.custom_markers.get_mut(c_idx) {
                cm.teleporter_tag = lone_tag.clone();
                cm.teleporter_url = partner_tag.clone();
            }
        }

        let new_id = format!("obj_tele_{}", cfg.custom_markers.len() + 1);
        cfg.custom_markers.push(WizardCustomMarker {
            id: new_id,
            label: partner_label.clone(),
            kind: "Teleporter".to_string(),
            class_name: "Engine.Teleporter".to_string(),
            location: partner_loc,
            team: None,
            teleporter_tag: partner_tag.clone(),
            teleporter_url: lone_tag.clone(),
        });

        log.push(format!(
            "Spawned companion teleporter '{}' at ({:.0}, {:.0}) paired with '{}'.",
            partner_label, partner_loc.x, partner_loc.y, lone.name
        ));
        return log;
    }

    // Case 2: Teleporter count is odd -> spawn companion for the last odd one
    if entries.len() % 2 != 0 {
        let odd_entry = entries.last().unwrap();
        let nodes = build_room_nodes(cfg);
        let non_corridor: Vec<&RoomNode> = nodes.iter().filter(|r| !r.is_corridor).collect();
        let partner_loc = if let Some(r) = non_corridor.iter().find(|r| (r.center - odd_entry.location).length() > 300.0) {
            Vec3::new(r.center.x, r.center.y, r.floor_z() + 48.0)
        } else {
            odd_entry.location + Vec3::new(384.0, 384.0, 0.0)
        };
        let partner_tag = format!("{}_Partner", odd_entry.tag);
        let partner_label = format!("{}_Partner", odd_entry.name);
        let new_id = format!("obj_tele_{}", cfg.custom_markers.len() + 1);
        cfg.custom_markers.push(WizardCustomMarker {
            id: new_id,
            label: partner_label.clone(),
            kind: "Teleporter".to_string(),
            class_name: "Engine.Teleporter".to_string(),
            location: partner_loc,
            team: None,
            teleporter_tag: partner_tag.clone(),
            teleporter_url: odd_entry.tag.clone(),
        });
        log.push(format!("Resolved odd teleporter count: spawned companion '{}'.", partner_label));
    }

    // Case 3: Re-collect and pair up all unpaired or broken teleporters
    let entries = collect_wizard_teleporters(cfg);
    let mut i = 0;
    while i + 1 < entries.len() {
        let e_a = &entries[i];
        let e_b = &entries[i + 1];

        let tag_a = if !e_a.tag.is_empty() { e_a.tag.clone() } else { format!("{}_Tag", e_a.name) };
        let tag_b = if !e_b.tag.is_empty() { e_b.tag.clone() } else { format!("{}_Tag", e_b.name) };

        let needs_pair_a = e_a.url != tag_b;
        let needs_pair_b = e_b.url != tag_a;

        if needs_pair_a || needs_pair_b {
            if let Some(c_idx_a) = e_a.custom_index {
                if let Some(cm_a) = cfg.custom_markers.get_mut(c_idx_a) {
                    cm_a.teleporter_tag = tag_a.clone();
                    cm_a.teleporter_url = tag_b.clone();
                }
            }
            if let Some(c_idx_b) = e_b.custom_index {
                if let Some(cm_b) = cfg.custom_markers.get_mut(c_idx_b) {
                    cm_b.teleporter_tag = tag_b.clone();
                    cm_b.teleporter_url = tag_a.clone();
                }
            }
            log.push(format!("Connected teleporter pair: '{}' ⟷ '{}'.", e_a.name, e_b.name));
        }

        // Loop hazard proximity check (< 96 UU)
        let dist = (e_a.location - e_b.location).length();
        if dist < 96.0 {
            if let Some(c_idx_b) = e_b.custom_index {
                if let Some(cm_b) = cfg.custom_markers.get_mut(c_idx_b) {
                    cm_b.location += Vec3::new(256.0, 0.0, 0.0);
                    log.push(format!("Separated overlapping teleporters '{}' and '{}' to safe distance.", e_a.name, e_b.name));
                }
            }
        }

        i += 2;
    }

    log
}

#[derive(Debug, Clone, PartialEq)]
pub struct StairAlignmentIssue {
    pub stair_name: String,
    pub issue_type: StairIssueType,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairIssueType {
    TopElevationMismatch,
    TopSeamGap,
    FootElevationMismatch,
    RiserTooHigh,
    TreadTooShallow,
    VerticesOffGrid,
    WidthTooNarrow,
    CollisionMismatch,
}

/// Comprehensive audit of all architectural flights of stairs, catwalk stairs, and transit ramps.
pub fn audit_stair_alignment(cfg: &MapWizardConfig, map: &WizardGeneratedMap) -> Vec<StairAlignmentIssue> {
    let nodes = build_room_nodes(cfg);
    let mut issues = Vec::new();

    for b in &map.brushes {
        let is_catwalk_stairs = b.name.ends_with("_CatwalkStairs");
        let is_transit_stairs = b.name.starts_with("TransitStair_");
        let is_transit_ramp = b.name.starts_with("TransitRamp_");
        let is_egress_stairs = b.name.contains("EgressStairs");

        if !is_catwalk_stairs && !is_transit_stairs && !is_transit_ramp && !is_egress_stairs {
            continue;
        }

        // Bounding box & grid alignment
        let mut min_pt = Vec3::splat(f32::INFINITY);
        let mut max_pt = Vec3::splat(f32::NEG_INFINITY);
        let mut off_grid_verts = 0;
        for p in &b.polys {
            for &v in &p.vertices {
                let w = b.location + v;
                min_pt = min_pt.min(w);
                max_pt = max_pt.max(w);
                let is_frac = (w.x.round() - w.x).abs() > 0.05
                    || (w.y.round() - w.y).abs() > 0.05
                    || (w.z.round() - w.z).abs() > 0.05;
                let is_off_8 = ((w.x.round() as i64) % 8 != 0)
                    || ((w.y.round() as i64) % 8 != 0)
                    || ((w.z.round() as i64) % 8 != 0);
                if is_frac || is_off_8 {
                    off_grid_verts += 1;
                }
            }
        }

        if off_grid_verts > 0 {
            issues.push(StairAlignmentIssue {
                stair_name: b.name.clone(),
                issue_type: StairIssueType::VerticesOffGrid,
                details: format!("{} vertices off 8-UU grid (fractional or non-multiple of 8)", off_grid_verts),
            });
        }

        // Width check
        let sz = max_pt - min_pt;
        let min_xy_dim = sz.x.min(sz.y);
        if min_xy_dim < 64.0 {
            issues.push(StairAlignmentIssue {
                stair_name: b.name.clone(),
                issue_type: StairIssueType::WidthTooNarrow,
                details: format!("Stair/ramp width {:.1} < minimum safe passable 64 UU", min_xy_dim),
            });
        }

        // Collision ramp match check
        let col_name = if is_catwalk_stairs {
            format!("{}Col", b.name)
        } else if is_transit_stairs {
            b.name.replace("TransitStair_", "TransitStairCol_")
        } else {
            String::new()
        };
        if !col_name.is_empty() {
            if let Some(col) = map.brushes.iter().find(|c| c.name == col_name) {
                let mut col_min = Vec3::splat(f32::INFINITY);
                let mut col_max = Vec3::splat(f32::NEG_INFINITY);
                for p in &col.polys {
                    for &v in &p.vertices {
                        let w = col.location + v;
                        col_min = col_min.min(w);
                        col_max = col_max.max(w);
                    }
                }
                if (col_min.z - min_pt.z).abs() > 0.05 || (col_max.z - max_pt.z).abs() > 0.05 {
                    issues.push(StairAlignmentIssue {
                        stair_name: b.name.clone(),
                        issue_type: StairIssueType::CollisionMismatch,
                        details: format!("Collision ramp Z bounds [{:.1}, {:.1}] != visual [{:.1}, {:.1}]", col_min.z, col_max.z, min_pt.z, max_pt.z),
                    });
                }
            }
        }

        // Catwalk stairs checks
        if is_catwalk_stairs {
            let r_name = b.name.trim_end_matches("_CatwalkStairs");
            if let Some(r) = nodes.iter().find(|n| n.name == r_name) {
                let deck_name = format!("{}_CatwalkDeck", r_name);
                if let Some(deck) = map.brushes.iter().find(|d| d.name == deck_name) {
                    let mut deck_min = Vec3::splat(f32::INFINITY);
                    let mut deck_max = Vec3::splat(f32::NEG_INFINITY);
                    for p in &deck.polys {
                        for &v in &p.vertices {
                            let w = deck.location + v;
                            deck_min = deck_min.min(w);
                            deck_max = deck_max.max(w);
                        }
                    }
                    if (max_pt.z - deck_max.z).abs() > 0.05 {
                        issues.push(StairAlignmentIssue {
                            stair_name: b.name.clone(),
                            issue_type: StairIssueType::TopElevationMismatch,
                            details: format!("Stair top Z {:.1} != Deck top Z {:.1}", max_pt.z, deck_max.z),
                        });
                    }
                    if (min_pt.z - r.floor_z()).abs() > 0.05 {
                        issues.push(StairAlignmentIssue {
                            stair_name: b.name.clone(),
                            issue_type: StairIssueType::FootElevationMismatch,
                            details: format!("Stair foot Z {:.1} != Room floor Z {:.1}", min_pt.z, r.floor_z()),
                        });
                    }
                    let dx = (min_pt.x.max(deck_min.x) - max_pt.x.min(deck_max.x)).max(0.0);
                    let dy = (min_pt.y.max(deck_min.y) - max_pt.y.min(deck_max.y)).max(0.0);
                    if dx > 0.05 || dy > 0.05 {
                        issues.push(StairAlignmentIssue {
                            stair_name: b.name.clone(),
                            issue_type: StairIssueType::TopSeamGap,
                            details: format!("XY gap to deck: dx={:.1}, dy={:.1}", dx, dy),
                        });
                    }
                }
            }
        }

        // Transit stairs checks
        if is_transit_stairs || is_transit_ramp {
            let suffix = if let Some(s) = b.name.strip_prefix("TransitStair_") {
                s
            } else {
                b.name.strip_prefix("TransitRamp_").unwrap()
            };
            let mut pair = None;
            for l in &nodes {
                let prefix = format!("{}_", l.name);
                if suffix.starts_with(&prefix) {
                    let h_name = &suffix[prefix.len()..];
                    if let Some(h) = nodes.iter().find(|n| n.name == h_name) {
                        pair = Some((l, h));
                        break;
                    }
                }
            }

            if let Some((lower_node, higher_node)) = pair {
                if (max_pt.z - higher_node.floor_z()).abs() > 0.05 {
                    issues.push(StairAlignmentIssue {
                        stair_name: b.name.clone(),
                        issue_type: StairIssueType::TopElevationMismatch,
                        details: format!("Transit top Z {:.1} != Higher floor Z {:.1}", max_pt.z, higher_node.floor_z()),
                    });
                }
                if (min_pt.z - lower_node.floor_z()).abs() > 0.05 {
                    issues.push(StairAlignmentIssue {
                        stair_name: b.name.clone(),
                        issue_type: StairIssueType::FootElevationMismatch,
                        details: format!("Transit foot Z {:.1} != Lower floor Z {:.1}", min_pt.z, lower_node.floor_z()),
                    });
                }

                // XY Seam check: top edge must meet the exact boundary of lower_node
                let dx = (higher_node.center.x - lower_node.center.x).abs();
                let dy = (higher_node.center.y - lower_node.center.y).abs();
                let (gap, axis) = if dx >= dy {
                    let asc_east = higher_node.center.x > lower_node.center.x;
                    if asc_east {
                        let expected_seam = lower_node.max_x().min(higher_node.max_x());
                        let gap = (expected_seam - max_pt.x).abs();
                        (gap, "East")
                    } else {
                        let expected_seam = lower_node.min_x().max(higher_node.min_x());
                        let gap = (min_pt.x - expected_seam).abs();
                        (gap, "West")
                    }
                } else {
                    let asc_north = higher_node.center.y > lower_node.center.y;
                    if asc_north {
                        let expected_seam = lower_node.max_y().min(higher_node.max_y());
                        let gap = (expected_seam - max_pt.y).abs();
                        (gap, "North")
                    } else {
                        let expected_seam = lower_node.min_y().max(higher_node.min_y());
                        let gap = (min_pt.y - expected_seam).abs();
                        (gap, "South")
                    }
                };
                if gap > 0.05 {
                    issues.push(StairAlignmentIssue {
                        stair_name: b.name.clone(),
                        issue_type: StairIssueType::TopSeamGap,
                        details: format!("Top step terminates {:.1} UU off {} seam", gap, axis),
                    });
                }
            }
        }
    }

    issues
}

/// Computes a 2D topology schematic preview and telemetry summary for the Wizard UI.
pub fn preview_wizard_topology(cfg: &MapWizardConfig) -> WizardTopologyPreview {
    let (nodes, graph) = build_room_nodes_with_graph(cfg, true);
    let mut preview = WizardTopologyPreview::default();
    preview.procedural_graph = graph;
    preview.warnings = override_contact_warnings(cfg, &nodes, preview.procedural_graph.as_ref());
    preview.warnings.extend(topology_reference_warnings(cfg, &nodes));

    let is_sym = cfg.game_mode == WizardGameMode::CaptureTheFlag
        || cfg.symmetry_mode != WizardSymmetryMode::None
        || (cfg.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
            && cfg.archetype.is_symmetrical());
    if is_sym {
        preview.symmetry_axis_x = Some(0.0);
    }

    let mut non_corridor_rooms: Vec<&RoomNode> = Vec::new();
    for r in &nodes {
        preview.rooms.push(WizardPreviewRoom2D {
            label: r.name.clone(),
            center: r.center,
            size: r.size,
            is_corridor: r.is_corridor,
            is_hazard: r.has_hazard_pit,
            is_octagonal: r.is_octagonal,
            team_tint: r.team_owner,
            environment: cfg.get_room_environment(&r.name),
        });
        if !r.is_corridor {
            non_corridor_rooms.push(r);
        }
    }

    // Estimate markers
    let active_w = cfg.active_weapons();
    let active_p = cfg.active_powerups();
    for (idx, r) in non_corridor_rooms.iter().enumerate() {
        let fz = r.floor_z() + 48.0;
        let team = r.team_owner.unwrap_or((idx % 2) as u8);
        preview.markers.push(WizardPreviewMarker2D {
            label: format!("PlayerStart_{idx}"),
            kind: "PlayerStart",
            location: Vec3::new(r.center.x - 96.0, r.center.y - 96.0, fz),
            team: Some(team),
        });
        if cfg.toggles.include_weapons_and_pickups {
            let weap_name = if !active_w.is_empty() {
                active_w[idx % active_w.len()].clone()
            } else {
                format!("Weapon_{idx}")
            };
            preview.markers.push(WizardPreviewMarker2D {
                label: weap_name,
                kind: "Weapon",
                location: Vec3::new(r.center.x + 96.0, r.center.y + 96.0, fz),
                team: None,
            });
        }
    }

    if let Some(hub) = non_corridor_rooms.first() {
        let dy = (hub.size.y * 0.32).clamp(240.0, 272.0);
        let p_y_pos = hub.center.y + dy;
        let p_y_neg = hub.center.y - dy;
        if cfg.toggles.include_ramps_and_bridges {
            preview.markers.push(WizardPreviewMarker2D {
                label: "OverwatchRamp".to_string(),
                kind: "Ramp",
                location: Vec3::new(hub.center.x - 80.0, p_y_pos, hub.floor_z() + 16.0),
                team: None,
            });
        }
        if cfg.toggles.include_weapons_and_pickups {
            if active_p.iter().any(|p| p.eq_ignore_ascii_case("UT_ShieldBelt")) {
                preview.markers.push(WizardPreviewMarker2D {
                    label: "UT_ShieldBelt".to_string(),
                    kind: "Powerup",
                    location: Vec3::new(hub.center.x, p_y_pos, hub.floor_z() + 48.0),
                    team: None,
                });
            }
            if active_p.iter().any(|p| p.eq_ignore_ascii_case("UDamage"))
                && cfg.game_mode != WizardGameMode::Duel1v1
            {
                preview.markers.push(WizardPreviewMarker2D {
                    label: "UDamage".to_string(),
                    kind: "Powerup",
                    location: Vec3::new(hub.center.x, p_y_neg, hub.floor_z() + 48.0),
                    team: None,
                });
            }
        }
    }

    let all_teleporters = collect_wizard_teleporters(cfg);
    for entry in &all_teleporters {
        if !entry.is_custom {
            preview.markers.push(WizardPreviewMarker2D {
                label: entry.name.clone(),
                kind: "Teleporter",
                location: entry.location,
                team: entry.team,
            });
        }
    }

    match cfg.game_mode {
        WizardGameMode::CaptureTheFlag => {
            let ctf_nodes: Vec<crate::ctf_mission::CtfNode> = nodes
                .iter()
                .map(|r| crate::ctf_mission::CtfNode {
                    name: r.name.clone(),
                    center: r.center,
                    size: r.size,
                    is_corridor: r.is_corridor,
                    team_owner: r.team_owner,
                })
                .collect();
            let ctf_plan = crate::ctf_mission::plan_ctf_mission(
                &ctf_nodes,
                cfg.ctf_dais_style,
                cfg.ctf_include_sniper_perches,
                cfg.ctf_contested_center_powerup,
                cfg.seed,
            );
            preview.markers.push(WizardPreviewMarker2D {
                label: "Red FlagBase".to_string(),
                kind: "FlagBase",
                location: ctf_plan.red_flag_loc,
                team: Some(0),
            });
            preview.markers.push(WizardPreviewMarker2D {
                label: "Blue FlagBase".to_string(),
                kind: "FlagBase",
                location: ctf_plan.blue_flag_loc,
                team: Some(1),
            });
            if let Some(cp_loc) = ctf_plan.center_powerup_loc {
                preview.markers.push(WizardPreviewMarker2D {
                    label: ctf_plan.center_powerup_class.clone(),
                    kind: "Powerup",
                    location: cp_loc,
                    team: None,
                });
            }
            for sp in &ctf_plan.sniper_perches {
                preview.markers.push(WizardPreviewMarker2D {
                    label: format!("{}_SniperOverwatch", if sp.team == 0 { "Red" } else { "Blue" }),
                    kind: "SniperPerch",
                    location: sp.perch_location,
                    team: Some(sp.team),
                });
            }
            for (ap_i, ap) in ctf_plan.alternate_paths.iter().enumerate() {
                preview.markers.push(WizardPreviewMarker2D {
                    label: format!("AltPath_T{}_{ap_i}", ap.team),
                    kind: "AlternatePath",
                    location: ap.location,
                    team: Some(ap.team),
                });
            }
        }
        WizardGameMode::Domination => {
            let m_nodes: Vec<crate::domination_mission::DominationNode> = nodes
                .iter()
                .map(|r| crate::domination_mission::DominationNode {
                    center: r.center,
                    size: r.size,
                    is_corridor: r.is_corridor,
                })
                .collect();
            let dom_plan = crate::domination_mission::plan_domination_mission(
                &m_nodes,
                cfg.theme,
                cfg.domination_naming_style,
                cfg.domination_point_count,
                cfg.seed,
            );
            for (p_idx, p) in dom_plan.points.iter().enumerate() {
                preview.markers.push(WizardPreviewMarker2D {
                    label: format!("CP #{}: {}", p_idx + 1, p.point_name),
                    kind: "ControlPoint",
                    location: p.location,
                    team: None,
                });
            }
            if let Some(cent_pos) = dom_plan.center_powerup_location {
                preview.markers.push(WizardPreviewMarker2D {
                    label: "Contested Center ShieldBelt".to_string(),
                    kind: "Powerup",
                    location: cent_pos,
                    team: None,
                });
            }
        }
        WizardGameMode::Assault => {
            let m_nodes: Vec<crate::assault_mission::MissionNode> = nodes
                .iter()
                .map(|r| crate::assault_mission::MissionNode {
                    center: r.center,
                    size: r.size,
                    is_corridor: r.is_corridor,
                })
                .collect();
            let as_plan = crate::assault_mission::plan_assault_mission(
                &m_nodes,
                cfg.assault_objective_count,
                cfg.seed,
            );
            for (idx, obj) in as_plan.objectives.iter().enumerate() {
                if let Some(r) = nodes.get(obj.node_idx) {
                    let label = if obj.is_final {
                        format!("Obj {}: {} [FINAL]", idx + 1, obj.kind.label())
                    } else {
                        format!("Obj {}: {}", idx + 1, obj.kind.label())
                    };
                    preview.markers.push(WizardPreviewMarker2D {
                        label,
                        kind: "FortStandard",
                        location: Vec3::new(r.center.x, r.center.y, r.floor_z() + 48.0),
                        team: Some(0),
                    });
                    if obj.kind.is_switch() {
                        preview.markers.push(WizardPreviewMarker2D {
                            label: format!("Switch {}", idx + 1),
                            kind: "Mover",
                            location: Vec3::new(
                                r.center.x,
                                r.center.y - r.size.y * 0.35,
                                r.floor_z() + 48.0,
                            ),
                            team: Some(1),
                        });
                    }
                }
                if let Some(gate_idx) = obj.gate_node {
                    if let Some(g_node) = nodes.get(gate_idx) {
                        preview.markers.push(WizardPreviewMarker2D {
                            label: format!("Gate Door {}", idx + 1),
                            kind: "Mover",
                            location: Vec3::new(g_node.center.x, g_node.center.y, g_node.floor_z() + 48.0),
                            team: None,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    let room_cnt = non_corridor_rooms.len();
    let detail_extra = match cfg.architectural_detail {
        WizardArchitecturalDetail::Clean => 0,
        WizardArchitecturalDetail::StandardTournament => nodes.len() * 4,
        WizardArchitecturalDetail::EpicHighDetail => nodes.len() * 8,
    };
    preview.estimated_brushes = nodes.len()
        + if cfg.toggles.include_skybox { 2 } else { 0 }
        + if cfg.toggles.include_environmental_hazards { 2 } else { 0 }
        + if cfg.toggles.include_ramps_and_bridges { 2 } else { 0 }
        + if cfg.toggles.include_weapons_and_pickups { room_cnt } else { 0 }
        + detail_extra
        + 2;
    preview.estimated_spawns = if cfg.game_mode == WizardGameMode::Duel1v1 {
        4
    } else {
        (room_cnt * 2).max(6)
    };
    preview.estimated_lights = if cfg.toggles.include_lighting {
        nodes.len() * 2 + 2
    } else {
        1
    };
    preview.estimated_weapons_pickups = if cfg.toggles.include_weapons_and_pickups {
        room_cnt * 4 + 2
    } else {
        0
    };
    preview.estimated_path_nodes = if cfg.toggles.include_bot_ai_paths {
        nodes.len() * 2
            + 4
            + if cfg.toggles.include_ramps_and_bridges { 2 } else { 0 }
            + if cfg.toggles.include_teleporters { 2 } else { 0 }
    } else {
        0
    };
    let assault_movers = if cfg.game_mode == WizardGameMode::Assault {
        let m_nodes: Vec<crate::assault_mission::MissionNode> = nodes
            .iter()
            .map(|r| crate::assault_mission::MissionNode {
                center: r.center,
                size: r.size,
                is_corridor: r.is_corridor,
            })
            .collect();
        let as_plan = crate::assault_mission::plan_assault_mission(
            &m_nodes,
            cfg.assault_objective_count,
            cfg.seed,
        );
        let gates = as_plan.objectives.iter().filter(|o| o.gate_node.is_some()).count();
        let switches = as_plan.objectives.iter().filter(|o| o.kind.is_switch()).count();
        gates + switches
    } else {
        0
    };
    preview.estimated_movers = assault_movers
        + if cfg.toggles.include_elevators_and_movers { 1 } else { 0 };

    // Append user custom markers added via 2D minimap
    for cm in &cfg.custom_markers {
        let static_kind: &'static str = match cm.kind.as_str() {
            "PlayerStart" => "PlayerStart",
            "Weapon" => "Weapon",
            "Powerup" => "Powerup",
            "Health" => "Health",
            "Armor" => "Armor",
            "Light" => "Light",
            "Teleporter" => "Teleporter",
            "ControlPoint" => "ControlPoint",
            "FlagBase" => "FlagBase",
            _ => "Weapon",
        };
        preview.markers.push(WizardPreviewMarker2D {
            label: cm.label.clone(),
            kind: static_kind,
            location: cm.location,
            team: cm.team,
        });
    }

    // Filter out markers suppressed by the user
    if !cfg.suppressed_markers.is_empty() {
        preview.markers.retain(|m| !cfg.suppressed_markers.iter().any(|s| s == &m.label));
    }

    preview.teleport_audit = audit_wizard_teleporters(cfg);

    preview
}

/// Computes safe, grid-aligned, non-intersecting pickup slots in a room.
/// Returns up to 4 quadrant slots and up to 4 secondary perimeter alcove slots.
fn compute_room_pickup_slots(room: &RoomNode, include_hazards: bool) -> Vec<Vec3> {
    let mut slots = Vec::new();
    let fz = room.floor_z() + 36.0;

    let quad_frac = if room.is_octagonal { 0.18 } else { 0.24 };
    let mut dx = (room.size.x * quad_frac).clamp(120.0, 240.0);
    let mut dy = (room.size.y * quad_frac).clamp(120.0, 240.0);

    if room.has_hazard_pit && include_hazards {
        let pit_w = (room.size.x * 0.32).clamp(192.0, 352.0);
        let pit_l = (room.size.y * 0.32).clamp(192.0, 352.0);
        dx = dx.max(pit_w * 0.5 + 48.0);
        dy = dy.max(pit_l * 0.5 + 48.0);
    }

    // 4 primary quadrants (offset quadrant 2 so it never clashes with objective consoles at (-128, +160))
    let q2_dx = if room.is_octagonal { dx } else { (dx + 32.0).min(room.size.x * 0.32) };
    let q2_dy = if room.is_octagonal { dy } else { (dy + 32.0).min(room.size.y * 0.32) };
    slots.push(Vec3::new(room.center.x + dx, room.center.y + dy, fz));
    slots.push(Vec3::new(room.center.x - dx, room.center.y - dy, fz));
    slots.push(Vec3::new(room.center.x - q2_dx, room.center.y + q2_dy, fz));
    slots.push(Vec3::new(room.center.x + dx, room.center.y - dy, fz));

    // 4 secondary perimeter alcove slots:
    // Place on East, West, and South walls away from North wall (Y+) center where OverwatchDeck and ramps are built
    let alcove_frac = if room.is_octagonal { 0.24 } else { 0.30 };
    let alcove_dx = (room.size.x * alcove_frac).clamp(140.0, 280.0);
    let alcove_dy = (room.size.y * alcove_frac).clamp(140.0, 280.0);
    slots.push(Vec3::new(room.center.x + alcove_dx, room.center.y, fz));
    slots.push(Vec3::new(room.center.x - alcove_dx, room.center.y, fz));
    slots.push(Vec3::new(room.center.x + alcove_dx * 0.5, room.center.y - alcove_dy, fz));
    slots.push(Vec3::new(room.center.x - alcove_dx * 0.5, room.center.y - alcove_dy, fz));

    slots
}

/// Computes safe pickup slots along the length of a corridor or tunnel.
fn compute_corridor_pickup_slots(corr: &RoomNode) -> Vec<Vec3> {
    let mut slots = Vec::new();
    let fz = corr.floor_z() + 36.0;

    let len_x = corr.size.x;
    let len_y = corr.size.y;

    if len_x >= len_y && len_x >= 768.0 {
        let offset = (len_x * 0.28).clamp(200.0, 800.0);
        slots.push(Vec3::new(corr.center.x - offset, corr.center.y, fz));
        slots.push(Vec3::new(corr.center.x + offset, corr.center.y, fz));
        if len_x >= 1800.0 {
            slots.push(Vec3::new(corr.center.x - offset * 0.5, corr.center.y, fz));
            slots.push(Vec3::new(corr.center.x + offset * 0.5, corr.center.y, fz));
        }
    } else if len_y > len_x && len_y >= 768.0 {
        let offset = (len_y * 0.28).clamp(200.0, 800.0);
        slots.push(Vec3::new(corr.center.x, corr.center.y - offset, fz));
        slots.push(Vec3::new(corr.center.x, corr.center.y + offset, fz));
        if len_y >= 1800.0 {
            slots.push(Vec3::new(corr.center.x, corr.center.y - offset * 0.5, fz));
            slots.push(Vec3::new(corr.center.x, corr.center.y + offset * 0.5, fz));
        }
    }

    slots
}

fn get_ammo_box_offset(idx: usize) -> Vec3 {
    match idx {
        0 => Vec3::new(-64.0, 0.0, 0.0),
        1 => Vec3::new(64.0, 0.0, 0.0),
        2 => Vec3::new(0.0, -64.0, 0.0),
        3 => Vec3::new(0.0, 64.0, 0.0),
        4 => Vec3::new(-52.0, -52.0, 0.0),
        5 => Vec3::new(52.0, 52.0, 0.0),
        6 => Vec3::new(-52.0, 52.0, 0.0),
        7 => Vec3::new(52.0, -52.0, 0.0),
        _ => {
            let angle = (idx as f32) * 0.785398;
            let radius = 64.0 + ((idx / 8) as f32) * 32.0;
            Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0)
        }
    }
}

/// Spawns a weapon on a semi-solid pedestal with paired ammunition.
/// Spawns a weapon on a semi-solid pedestal with paired ammunition symmetrically for both Red and Blue teams.
/// The Red entity locations are used as primary reference, and the Blue entities are generated at
/// exact mathematical coordinates using `transform_point_symmetry`.
fn spawn_symmetrical_weapon_cluster(
    weap_cls: &str,
    loc_red: Vec3,
    unique_suffix: &str,
    group: &str,
    cfg: &MapWizardConfig,
    sym_mode: CtfSymmetryMode,
    sym_center: Vec3,
    pal: &WizardResolvedPalette,
    actors: &mut Vec<WizardActorSpec>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    let floor_z_red = loc_red.z - 36.0;

    // 1. Red Pedestal
    let mut pad_red = build_cube(
        format!("WeaponPad_Red_{unique_suffix}"),
        CsgOperation::Add,
        Vec3::new(96.0, 96.0, 8.0),
    );
    pad_red.location = Vec3::new(loc_red.x, loc_red.y, floor_z_red + 4.0);
    pad_red.group = Some(group.to_string());
    for p in &mut pad_red.polys {
        p.texture = pal.trim_texture.clone();
        p.flags |= PF_SEMISOLID;
    }
    pad_red.recompute_uvs();

    // Blue Pedestal: transformed directly from Red
    let mut pad_blue = pad_red.clone();
    pad_blue.name = format!("WeaponPad_Blue_{unique_suffix}");
    pad_blue.location = transform_point_symmetry(pad_red.location, sym_mode, sym_center);
    pad_blue.recompute_uvs();

    semisolid_detail_brushes.push(pad_red);
    semisolid_detail_brushes.push(pad_blue);

    // 2. Red Weapon Actor
    let wep_loc_red = Vec3::new(loc_red.x, loc_red.y, loc_red.z + 8.0);
    actors.push(WizardActorSpec {
        name: format!("{weap_cls}_Red_{unique_suffix}"),
        class_name: weap_cls.to_string(),
        location: wep_loc_red,
        group: group.to_string(),
        ..Default::default()
    });

    // Blue Weapon Actor: transformed from Red
    let wep_loc_blue = transform_point_symmetry(wep_loc_red, sym_mode, sym_center);
    actors.push(WizardActorSpec {
        name: format!("{weap_cls}_Blue_{unique_suffix}"),
        class_name: weap_cls.to_string(),
        location: wep_loc_blue,
        group: group.to_string(),
        ..Default::default()
    });

    // 3. Paired Ammunition
    if let Some(ammo) = ammo_for_weapon(weap_cls) {
        let count = cfg.ammo_count_for_weapon(weap_cls) as usize;
        for i in 0..count {
            let off = get_ammo_box_offset(i);
            let ammo_loc_red = Vec3::new(loc_red.x + off.x, loc_red.y + off.y, loc_red.z);
            actors.push(WizardActorSpec {
                name: format!("{ammo}_Red_{unique_suffix}_{i}"),
                class_name: ammo.to_string(),
                location: ammo_loc_red,
                group: group.to_string(),
                ..Default::default()
            });

            let ammo_loc_blue = transform_point_symmetry(ammo_loc_red, sym_mode, sym_center);
            actors.push(WizardActorSpec {
                name: format!("{ammo}_Blue_{unique_suffix}_{i}"),
                class_name: ammo.to_string(),
                location: ammo_loc_blue,
                group: group.to_string(),
                ..Default::default()
            });
        }
    }
}

fn spawn_weapon_with_pedestal(
    weap_cls: &str,
    loc: Vec3,
    unique_suffix: &str,
    group: &str,
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    actors: &mut Vec<WizardActorSpec>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    let floor_z = loc.z - 36.0;

    let mut pad = build_cube(
        format!("WeaponPad_{unique_suffix}"),
        CsgOperation::Add,
        Vec3::new(96.0, 96.0, 8.0),
    );
    pad.location = Vec3::new(loc.x, loc.y, floor_z + 4.0);
    pad.group = Some(group.to_string());
    for p in &mut pad.polys {
        p.texture = pal.trim_texture.clone();
        p.flags |= PF_SEMISOLID;
    }
    pad.recompute_uvs();
    semisolid_detail_brushes.push(pad);

    actors.push(WizardActorSpec {
        name: format!("{weap_cls}_{unique_suffix}"),
        class_name: weap_cls.to_string(),
        location: Vec3::new(loc.x, loc.y, loc.z + 8.0),
        group: group.to_string(),
        ..Default::default()
    });

    if let Some(ammo) = ammo_for_weapon(weap_cls) {
        let count = cfg.ammo_count_for_weapon(weap_cls) as usize;
        for i in 0..count {
            let off = get_ammo_box_offset(i);
            actors.push(WizardActorSpec {
                name: format!("{ammo}_{unique_suffix}_{i}"),
                class_name: ammo.to_string(),
                location: Vec3::new(loc.x + off.x, loc.y + off.y, loc.z),
                group: group.to_string(),
                ..Default::default()
            });
        }
    }
}

/// Spawns health restoratives (MedBoxes and HealthVials) with symmetrical Blue team mirroring.
fn spawn_restoratives_symmetrical(
    loc_red: Vec3,
    unique_suffix: &str,
    has_medbox: bool,
    has_vials: bool,
    density: WizardItemDensity,
    sym_mode: CtfSymmetryMode,
    center: Vec3,
    actors: &mut Vec<WizardActorSpec>,
) {
    if has_medbox {
        actors.push(WizardActorSpec {
            name: format!("MedBox_Red_{unique_suffix}"),
            class_name: "MedBox".to_string(),
            location: loc_red,
            group: "Pickups".to_string(),
            ..Default::default()
        });
        let loc_blue = transform_point_symmetry(loc_red, sym_mode, center);
        actors.push(WizardActorSpec {
            name: format!("MedBox_Blue_{unique_suffix}"),
            class_name: "MedBox".to_string(),
            location: loc_blue,
            group: "Pickups".to_string(),
            ..Default::default()
        });

        if density == WizardItemDensity::Abundant {
            let loc_red_extra = loc_red + Vec3::new(48.0, 0.0, 0.0);
            actors.push(WizardActorSpec {
                name: format!("MedBox_Red_{unique_suffix}_extra"),
                class_name: "MedBox".to_string(),
                location: loc_red_extra,
                group: "Pickups".to_string(),
                ..Default::default()
            });
            let loc_blue_extra = transform_point_symmetry(loc_red_extra, sym_mode, center);
            actors.push(WizardActorSpec {
                name: format!("MedBox_Blue_{unique_suffix}_extra"),
                class_name: "MedBox".to_string(),
                location: loc_blue_extra,
                group: "Pickups".to_string(),
                ..Default::default()
            });
        }
    }

    if has_vials {
        let vial_count = if density == WizardItemDensity::Sparse { 3 } else { 5 };
        for vi in 0..vial_count {
            let offset_y = (vi as f32 - (vial_count as f32 - 1.0) * 0.5) * 26.0;
            let curve_x = (1.0 - ((vi as f32 - (vial_count as f32 - 1.0) * 0.5) / 2.0).powi(2)).max(0.0) * 12.0;
            let v_red = Vec3::new(loc_red.x - 32.0 - curve_x, loc_red.y + offset_y, loc_red.z);
            actors.push(WizardActorSpec {
                name: format!("HealthVial_Red_{unique_suffix}_{vi}"),
                class_name: "HealthVial".to_string(),
                location: v_red,
                group: "Pickups".to_string(),
                ..Default::default()
            });
            let v_blue = transform_point_symmetry(v_red, sym_mode, center);
            actors.push(WizardActorSpec {
                name: format!("HealthVial_Blue_{unique_suffix}_{vi}"),
                class_name: "HealthVial".to_string(),
                location: v_blue,
                group: "Pickups".to_string(),
                ..Default::default()
            });
        }

        if density == WizardItemDensity::Abundant {
            for vi in 0..vial_count {
                let offset_y = (vi as f32 - (vial_count as f32 - 1.0) * 0.5) * 26.0;
                let curve_x = (1.0 - ((vi as f32 - (vial_count as f32 - 1.0) * 0.5) / 2.0).powi(2)).max(0.0) * 12.0;
                let v_red = Vec3::new(loc_red.x - 64.0 - curve_x, loc_red.y + offset_y, loc_red.z);
                actors.push(WizardActorSpec {
                    name: format!("HealthVial_Red_{unique_suffix}_{vi}_extra"),
                    class_name: "HealthVial".to_string(),
                    location: v_red,
                    group: "Pickups".to_string(),
                    ..Default::default()
                });
                let v_blue = transform_point_symmetry(v_red, sym_mode, center);
                actors.push(WizardActorSpec {
                    name: format!("HealthVial_Blue_{unique_suffix}_{vi}_extra"),
                    class_name: "HealthVial".to_string(),
                    location: v_blue,
                    group: "Pickups".to_string(),
                    ..Default::default()
                });
            }
        }
    }
}

/// Computes whether two brushes overlap in 3D world space using strict AABB bounds.
fn brushes_overlap(a: &Brush, b: &Brush) -> bool {
    let (a_min, a_max) = a.world_bounds();
    let (b_min, b_max) = b.world_bounds();
    a_min.x < b_max.x && a_max.x > b_min.x
        && a_min.y < b_max.y && a_max.y > b_min.y
        && a_min.z < b_max.z && a_max.z > b_min.z
}

/// Carves authentic recessed ceiling coffers (`CSG_Subtract`) and adds unlit illuminated ceiling panels
/// with accent mood lights into major combat chambers.
fn generate_ceiling_coffers(
    nodes: &[RoomNode],
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    sub_brushes: &mut Vec<Brush>,
    semisolid_detail_brushes: &mut Vec<Brush>,
    actors: &mut Vec<WizardActorSpec>,
) {
    if cfg.architectural_detail == WizardArchitecturalDetail::Clean {
        return;
    }

    let is_epic = cfg.architectural_detail == WizardArchitecturalDetail::EpicHighDetail;

    for r in nodes.iter().filter(|n| !n.is_corridor && !n.sky_ceiling) {
        if r.size.x < 512.0 || r.size.y < 512.0 || r.size.z < 320.0 {
            continue;
        }

        let coffer_depth = if is_epic { 64.0_f32 } else { 48.0_f32 };
        let margin = 96.0_f32;

        if r.size.x >= 1024.0 && r.size.y >= 1024.0 {
            // 2x2 grid of coffers for grand halls
            let cw = ((r.size.x - margin * 3.0) * 0.5).floor();
            let cl = ((r.size.y - margin * 3.0) * 0.5).floor();
            let ox = (cw + margin) * 0.5;
            let oy = (cl + margin) * 0.5;

            for (idx, (dx, dy)) in [(-ox, -oy), (ox, -oy), (-ox, oy), (ox, oy)].into_iter().enumerate() {
                let center_x = r.center.x + dx;
                let center_y = r.center.y + dy;
                let coffer_z = r.ceiling_z() + coffer_depth * 0.5;

                let mut coffer = build_cube(
                    format!("{}_CeilingCoffer_{idx}", r.name),
                    CsgOperation::Subtract,
                    Vec3::new(cw, cl, coffer_depth),
                );
                coffer.location = Vec3::new(center_x, center_y, coffer_z);
                coffer.group = Some("Architecture".to_string());
                for p in &mut coffer.polys {
                    if p.plane.normal.z < -0.65 {
                        p.texture = pal.ceiling_texture.clone();
                    } else {
                        p.texture = pal.trim_texture.clone();
                    }
                }
                coffer.recompute_uvs();
                sub_brushes.push(coffer);

                if is_epic {
                    // Inner recessed tier for Epic High Detail
                    let inner_w = (cw * 0.6).max(64.0);
                    let inner_l = (cl * 0.6).max(64.0);
                    let inner_depth = 24.0_f32;
                    let mut inner_coffer = build_cube(
                        format!("{}_CeilingInnerCoffer_{idx}", r.name),
                        CsgOperation::Subtract,
                        Vec3::new(inner_w, inner_l, inner_depth),
                    );
                    inner_coffer.location = Vec3::new(center_x, center_y, r.ceiling_z() + coffer_depth + inner_depth * 0.5);
                    inner_coffer.group = Some("Architecture".to_string());
                    for p in &mut inner_coffer.polys {
                        p.texture = pal.trim_texture.clone();
                    }
                    inner_coffer.recompute_uvs();
                    sub_brushes.push(inner_coffer);
                }

                // Unlit illuminated ceiling fixture panel
                let panel_w = (cw * 0.45).max(48.0);
                let panel_l = (cl * 0.45).max(48.0);
                let panel_h = 4.0_f32;
                let mut panel = build_cube(
                    format!("{}_CeilingLightPanel_{idx}", r.name),
                    CsgOperation::Add,
                    Vec3::new(panel_w, panel_l, panel_h),
                );
                let panel_top = if is_epic {
                    r.ceiling_z() + coffer_depth + 24.0
                } else {
                    r.ceiling_z() + coffer_depth
                };
                panel.location = Vec3::new(center_x, center_y, panel_top - panel_h * 0.5);
                panel.group = Some("Architecture".to_string());
                for p in &mut panel.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID | PF_UNLIT;
                }
                panel.recompute_uvs();
                semisolid_detail_brushes.push(panel);

                // Accent ceiling light
                actors.push(WizardActorSpec {
                    name: format!("Light_Coffer_{}_{idx}", r.name),
                    class_name: "Light".to_string(),
                    location: Vec3::new(center_x, center_y, panel_top - 24.0),
                    group: "Lighting".to_string(),
                    light_brightness: 135,
                    light_radius: 28,
                    light_hue: pal.accent_light_hue,
                    light_saturation: pal.accent_light_sat,
                    ..Default::default()
                });
            }
        } else {
            // Single grand central coffer
            let cw = (r.size.x - margin * 2.0).floor();
            let cl = (r.size.y - margin * 2.0).floor();
            let coffer_z = r.ceiling_z() + coffer_depth * 0.5;

            let mut coffer = build_cube(
                format!("{}_CeilingCoffer", r.name),
                CsgOperation::Subtract,
                Vec3::new(cw, cl, coffer_depth),
            );
            coffer.location = Vec3::new(r.center.x, r.center.y, coffer_z);
            coffer.group = Some("Architecture".to_string());
            for p in &mut coffer.polys {
                if p.plane.normal.z < -0.65 {
                    p.texture = pal.ceiling_texture.clone();
                } else {
                    p.texture = pal.trim_texture.clone();
                }
            }
            coffer.recompute_uvs();
            sub_brushes.push(coffer);

            if is_epic {
                let inner_w = (cw * 0.6).max(96.0);
                let inner_l = (cl * 0.6).max(96.0);
                let inner_depth = 24.0_f32;
                let mut inner_coffer = build_cube(
                    format!("{}_CeilingInnerCoffer", r.name),
                    CsgOperation::Subtract,
                    Vec3::new(inner_w, inner_l, inner_depth),
                );
                inner_coffer.location = Vec3::new(r.center.x, r.center.y, r.ceiling_z() + coffer_depth + inner_depth * 0.5);
                inner_coffer.group = Some("Architecture".to_string());
                for p in &mut inner_coffer.polys {
                    p.texture = pal.trim_texture.clone();
                }
                inner_coffer.recompute_uvs();
                sub_brushes.push(inner_coffer);
            }

            // Unlit illuminated ceiling fixture panel
            let panel_w = (cw * 0.4).max(64.0);
            let panel_l = (cl * 0.4).max(64.0);
            let panel_h = 4.0_f32;
            let mut panel = build_cube(
                format!("{}_CeilingLightPanel", r.name),
                CsgOperation::Add,
                Vec3::new(panel_w, panel_l, panel_h),
            );
            let panel_top = if is_epic {
                r.ceiling_z() + coffer_depth + 24.0
            } else {
                r.ceiling_z() + coffer_depth
            };
            panel.location = Vec3::new(r.center.x, r.center.y, panel_top - panel_h * 0.5);
            panel.group = Some("Architecture".to_string());
            for p in &mut panel.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID | PF_UNLIT;
            }
            panel.recompute_uvs();
            semisolid_detail_brushes.push(panel);

            // Accent ceiling light
            actors.push(WizardActorSpec {
                name: format!("Light_Coffer_{}", r.name),
                class_name: "Light".to_string(),
                location: Vec3::new(r.center.x, r.center.y, panel_top - 24.0),
                group: "Lighting".to_string(),
                light_brightness: 145,
                light_radius: 36,
                light_hue: pal.accent_light_hue,
                light_saturation: pal.accent_light_sat,
                ..Default::default()
            });
        }
    }
}

/// Axis-aligned bounds of a brush from its vertices and location (rooms are unrotated, unscaled cubes/octagons).
fn brush_bounds(b: &Brush) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for p in &b.polys {
        for v in &p.vertices {
            let w = *v + b.location;
            lo = lo.min(w);
            hi = hi.max(w);
        }
    }
    (lo, hi)
}

/// True if a detail with bounds `[lo, hi]` touches solid rock: at least one face has, 6 UU beyond it,
/// a point outside every carved (subtractive) volume. Octagonal rooms are treated by
/// their bounding box, which can only make the answer more conservative.
fn detail_touches_solid(carved: &[(Vec3, Vec3)], lo: Vec3, hi: Vec3) -> bool {
    let c = (lo + hi) * 0.5;
    (0..3).any(|axis| {
        [(lo[axis] - 6.0), (hi[axis] + 6.0)].into_iter().any(|coord| {
            let mut p = c;
            p[axis] = coord;
            !carved.iter().any(|(a, b)| p.x > a.x && p.x < b.x && p.y > a.y && p.y < b.y && p.z > a.z && p.z < b.z)
        })
    })
}

/// A doorway frame whose centre is deep inside a room is attached to no wall and would hang in mid-air.
fn frame_floats_in_room(nodes: &[RoomNode], frame_center: Vec3) -> bool {
    nodes.iter().any(|r| !r.is_corridor && point_deep_inside_room(frame_center, r, FRAME_ROOM_INTERIOR_MARGIN))
}

/// Generates watertight zone portal sheets (`PF_PORTAL`) across all corridor midpoints and adds
/// semi-solid doorway frames (jambs + lintels) with strict >= 48 UU clearance from portals.
fn generate_zone_portals_and_doorway_frames(
    nodes: &[RoomNode],
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    movers: &[WizardMoverSpec],
    portal_sheet_brushes: &mut Vec<Brush>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    let corridors: Vec<&RoomNode> = nodes.iter().filter(|n| n.is_corridor).collect();
    let detail = cfg.architectural_detail;
    let portal_aligned = cfg.generator_version >= GENERATOR_VERSION_PORTALS;

    for (c_idx, corr) in corridors.iter().enumerate() {
        let is_ew = corr.size.x >= corr.size.y;
        let corr_len = if is_ew { corr.size.x } else { corr.size.y };
        let corr_w = if is_ew { corr.size.y } else { corr.size.x };
        let corr_h = corr.size.z;

        if corr_len < 128.0 {
            continue;
        }

        // Never place a portal sheet if corridor center is inside a combat room
        let is_inside_room = nodes
            .iter()
            .any(|r| !r.is_corridor && (r.center - corr.center).length_squared() < 64.0 * 64.0);
        if is_inside_room {
            continue;
        }

        // 1. Zone Portal Sheet at corridor midpoint (sealed against all 4 solid walls)
        let has_mover_at_center = movers.iter().any(|m| m.key0_location.distance(corr.center) < 80.0);
        let portal_offset = if has_mover_at_center { 96.0_f32 } else { 0.0_f32 };

        let (sheet_axis, portal_pos) = if is_ew {
            (SheetAxis::YZ, corr.center + Vec3::new(portal_offset, 0.0, 0.0))
        } else {
            (SheetAxis::XZ, corr.center + Vec3::new(0.0, portal_offset, 0.0))
        };

        let mut portal = build_sheet_advanced(
            format!("{}_ZonePortal_{c_idx}", corr.name),
            CsgOperation::Add,
            corr_w,
            corr_h,
            sheet_axis,
            SheetPreset::ZonePortal,
        );
        portal.location = portal_pos;
        portal.group = Some("Portals".to_string());
        for p in &mut portal.polys {
            p.texture = "Engine.DefaultTexture".to_string();
            p.flags |= PF_PORTAL | PF_NOT_SOLID | PF_TRANSLUCENT;
        }
        portal.recompute_uvs();
        let collides = portal_sheet_brushes.iter().any(|p| brushes_overlap(&portal, p))
            || semisolid_detail_brushes.iter().any(|ss| brushes_overlap(&portal, ss));
        if !collides {
            portal_sheet_brushes.push(portal);
        }

        // 2. Doorway Portal Frames (Semi-Solid jambs & lintels) at both ends of corridor
        if detail != WizardArchitecturalDetail::Clean && corr_len >= 256.0 {
            let jamb_w = 16.0_f32;
            let jamb_t = 16.0_f32;
            let jamb_h = (corr_h - 24.0).max(48.0);
            let lintel_h = 24.0_f32;
            let frame_offset = corr_len * 0.5 - jamb_t * 0.5;

            if is_ew {
                // East-West corridor: frames at -X (West end) and +X (East end)
                for (sign, end_name) in [(-1.0f32, "West"), (1.0f32, "East")] {
                    let fx = corr.center.x + sign * frame_offset;
                    // Safe buffer check from portal sheet (>= 48 UU)
                    if (fx - portal_pos.x).abs() < 48.0 {
                        continue;
                    }
                    if portal_aligned && frame_floats_in_room(nodes, Vec3::new(fx, corr.center.y, corr.center.z)) {
                        continue;
                    }

                    // Left jamb (-Y)
                    let mut jamb_l = build_cube(
                        format!("{}_Jamb_{end_name}_L", corr.name),
                        CsgOperation::Add,
                        Vec3::new(jamb_t, jamb_w, jamb_h),
                    );
                    jamb_l.location = Vec3::new(
                        fx,
                        corr.center.y - corr_w * 0.5 + jamb_w * 0.5,
                        corr.floor_z() + jamb_h * 0.5,
                    );
                    jamb_l.group = Some("Architecture".to_string());
                    for p in &mut jamb_l.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    jamb_l.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&jamb_l, p)) {
                        semisolid_detail_brushes.push(jamb_l);
                    }

                    // Right jamb (+Y)
                    let mut jamb_r = build_cube(
                        format!("{}_Jamb_{end_name}_R", corr.name),
                        CsgOperation::Add,
                        Vec3::new(jamb_t, jamb_w, jamb_h),
                    );
                    jamb_r.location = Vec3::new(
                        fx,
                        corr.center.y + corr_w * 0.5 - jamb_w * 0.5,
                        corr.floor_z() + jamb_h * 0.5,
                    );
                    jamb_r.group = Some("Architecture".to_string());
                    for p in &mut jamb_r.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    jamb_r.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&jamb_r, p)) {
                        semisolid_detail_brushes.push(jamb_r);
                    }

                    // Lintel header
                    let mut lintel = build_cube(
                        format!("{}_Lintel_{end_name}", corr.name),
                        CsgOperation::Add,
                        Vec3::new(jamb_t, corr_w, lintel_h),
                    );
                    lintel.location = Vec3::new(fx, corr.center.y, corr.ceiling_z() - lintel_h * 0.5);
                    lintel.group = Some("Architecture".to_string());
                    for p in &mut lintel.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    lintel.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&lintel, p)) {
                        semisolid_detail_brushes.push(lintel);
                    }
                }
            } else {
                // North-South corridor: frames at -Y (South end) and +Y (North end)
                for (sign, end_name) in [(-1.0f32, "South"), (1.0f32, "North")] {
                    let fy = corr.center.y + sign * frame_offset;
                    if (fy - portal_pos.y).abs() < 48.0 {
                        continue;
                    }
                    if portal_aligned && frame_floats_in_room(nodes, Vec3::new(corr.center.x, fy, corr.center.z)) {
                        continue;
                    }

                    // Left jamb (-X)
                    let mut jamb_l = build_cube(
                        format!("{}_Jamb_{end_name}_L", corr.name),
                        CsgOperation::Add,
                        Vec3::new(jamb_w, jamb_t, jamb_h),
                    );
                    jamb_l.location = Vec3::new(
                        corr.center.x - corr_w * 0.5 + jamb_w * 0.5,
                        fy,
                        corr.floor_z() + jamb_h * 0.5,
                    );
                    jamb_l.group = Some("Architecture".to_string());
                    for p in &mut jamb_l.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    jamb_l.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&jamb_l, p)) {
                        semisolid_detail_brushes.push(jamb_l);
                    }

                    // Right jamb (+X)
                    let mut jamb_r = build_cube(
                        format!("{}_Jamb_{end_name}_R", corr.name),
                        CsgOperation::Add,
                        Vec3::new(jamb_w, jamb_t, jamb_h),
                    );
                    jamb_r.location = Vec3::new(
                        corr.center.x + corr_w * 0.5 - jamb_w * 0.5,
                        fy,
                        corr.floor_z() + jamb_h * 0.5,
                    );
                    jamb_r.group = Some("Architecture".to_string());
                    for p in &mut jamb_r.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    jamb_r.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&jamb_r, p)) {
                        semisolid_detail_brushes.push(jamb_r);
                    }

                    // Lintel header
                    let mut lintel = build_cube(
                        format!("{}_Lintel_{end_name}", corr.name),
                        CsgOperation::Add,
                        Vec3::new(corr_w, jamb_t, lintel_h),
                    );
                    lintel.location = Vec3::new(corr.center.x, fy, corr.ceiling_z() - lintel_h * 0.5);
                    lintel.group = Some("Architecture".to_string());
                    for p in &mut lintel.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    lintel.recompute_uvs();
                    if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&lintel, p)) {
                        semisolid_detail_brushes.push(lintel);
                    }
                }
            }
        }

        // 3. Epic High Detail: Periodic wall ribs / pilasters along long corridors
        if detail == WizardArchitecturalDetail::EpicHighDetail && corr_len >= 768.0 {
            let rib_w = 16.0_f32;
            let rib_t = 12.0_f32;
            let rib_h = corr_h;
            let step = 256.0_f32;
            let half_l = corr_len * 0.5;

            let mut d = -half_l + 192.0;
            let mut rib_idx = 0;
            while d <= half_l - 192.0 {
                if d.abs() >= 64.0 {
                    if is_ew {
                        let rx = corr.center.x + d;
                        for (sign, side) in [(-1.0f32, "S"), (1.0f32, "N")] {
                            let mut rib = build_cube(
                                format!("{}_Rib_{side}_{rib_idx}", corr.name),
                                CsgOperation::Add,
                                Vec3::new(rib_w, rib_t, rib_h),
                            );
                            rib.location = Vec3::new(
                                rx,
                                corr.center.y + sign * (corr_w * 0.5 - rib_t * 0.5),
                                corr.floor_z() + rib_h * 0.5,
                            );
                            rib.group = Some("Architecture".to_string());
                            for p in &mut rib.polys {
                                p.texture = pal.trim_texture.clone();
                                p.flags |= PF_SEMISOLID;
                            }
                            rib.recompute_uvs();
                            if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&rib, p)) {
                                semisolid_detail_brushes.push(rib);
                            }
                        }
                    } else {
                        let ry = corr.center.y + d;
                        for (sign, side) in [(-1.0f32, "W"), (1.0f32, "E")] {
                            let mut rib = build_cube(
                                format!("{}_Rib_{side}_{rib_idx}", corr.name),
                                CsgOperation::Add,
                                Vec3::new(rib_t, rib_w, rib_h),
                            );
                            rib.location = Vec3::new(
                                corr.center.x + sign * (corr_w * 0.5 - rib_t * 0.5),
                                ry,
                                corr.floor_z() + rib_h * 0.5,
                            );
                            rib.group = Some("Architecture".to_string());
                            for p in &mut rib.polys {
                                p.texture = pal.trim_texture.clone();
                                p.flags |= PF_SEMISOLID;
                            }
                            rib.recompute_uvs();
                            if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&rib, p)) {
                                semisolid_detail_brushes.push(rib);
                            }
                        }
                    }
                    rib_idx += 1;
                }
                d += step;
            }
        }
    }
}

/// Generates perimeter wall baseboard trims (`PF_SEMISOLID`) along room boundaries,
/// automatically splitting segments to leave clean doorway threshold clearances.
fn generate_perimeter_baseboard_trims(
    nodes: &[RoomNode],
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    if cfg.architectural_detail == WizardArchitecturalDetail::Clean {
        return;
    }

    let trim_h = 16.0_f32;
    let trim_t = 8.0_f32;

    for r in nodes.iter().filter(|n| !n.is_corridor && !n.is_octagonal) {
        if r.size.x < 384.0 || r.size.y < 384.0 {
            continue;
        }

        let z = r.floor_z() + trim_h * 0.5;

        // 1. South Wall (-Y)
        let s_y = r.center.y - r.size.y * 0.5 + trim_t * 0.5;
        let s_doorway = nodes.iter().find(|c| {
            c.name != r.name
                && c.center.y < r.center.y
                && (c.max_y() - r.min_y()).abs() <= 128.0
                && (r.max_x().min(c.max_x()) - r.min_x().max(c.min_x())) >= 64.0
        });
        if let Some(door) = s_doorway {
            let door_min_x = r.min_x().max(door.min_x());
            let door_max_x = r.max_x().min(door.max_x());
            let left_len = (door_min_x - r.min_x() - trim_t).max(0.0);
            if left_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_S_L", r.name), CsgOperation::Add, Vec3::new(left_len, trim_t, trim_h));
                trim.location = Vec3::new(r.min_x() + trim_t + left_len * 0.5, s_y, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
            let right_len = (r.max_x() - door_max_x - trim_t).max(0.0);
            if right_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_S_R", r.name), CsgOperation::Add, Vec3::new(right_len, trim_t, trim_h));
                trim.location = Vec3::new(door_max_x + right_len * 0.5, s_y, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
        } else {
            let len = (r.size.x - trim_t * 2.0).max(64.0);
            let mut trim = build_cube(format!("{}_Baseboard_S", r.name), CsgOperation::Add, Vec3::new(len, trim_t, trim_h));
            trim.location = Vec3::new(r.center.x, s_y, z);
            trim.group = Some("Architecture".to_string());
            for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
            trim.recompute_uvs();
            semisolid_detail_brushes.push(trim);
        }

        // 2. North Wall (+Y)
        let n_y = r.center.y + r.size.y * 0.5 - trim_t * 0.5;
        let n_doorway = nodes.iter().find(|c| {
            c.name != r.name
                && c.center.y > r.center.y
                && (c.min_y() - r.max_y()).abs() <= 128.0
                && (r.max_x().min(c.max_x()) - r.min_x().max(c.min_x())) >= 64.0
        });
        if let Some(door) = n_doorway {
            let door_min_x = r.min_x().max(door.min_x());
            let door_max_x = r.max_x().min(door.max_x());
            let left_len = (door_min_x - r.min_x() - trim_t).max(0.0);
            if left_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_N_L", r.name), CsgOperation::Add, Vec3::new(left_len, trim_t, trim_h));
                trim.location = Vec3::new(r.min_x() + trim_t + left_len * 0.5, n_y, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
            let right_len = (r.max_x() - door_max_x - trim_t).max(0.0);
            if right_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_N_R", r.name), CsgOperation::Add, Vec3::new(right_len, trim_t, trim_h));
                trim.location = Vec3::new(door_max_x + right_len * 0.5, n_y, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
        } else {
            let len = (r.size.x - trim_t * 2.0).max(64.0);
            let mut trim = build_cube(format!("{}_Baseboard_N", r.name), CsgOperation::Add, Vec3::new(len, trim_t, trim_h));
            trim.location = Vec3::new(r.center.x, n_y, z);
            trim.group = Some("Architecture".to_string());
            for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
            trim.recompute_uvs();
            semisolid_detail_brushes.push(trim);
        }

        // 3. West Wall (-X)
        let w_x = r.center.x - r.size.x * 0.5 + trim_t * 0.5;
        let w_doorway = nodes.iter().find(|c| {
            c.name != r.name
                && c.center.x < r.center.x
                && (c.max_x() - r.min_x()).abs() <= 128.0
                && (r.max_y().min(c.max_y()) - r.min_y().max(c.min_y())) >= 64.0
        });
        if let Some(door) = w_doorway {
            let door_min_y = r.min_y().max(door.min_y());
            let door_max_y = r.max_y().min(door.max_y());
            let bottom_len = (door_min_y - r.min_y() - trim_t).max(0.0);
            if bottom_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_W_B", r.name), CsgOperation::Add, Vec3::new(trim_t, bottom_len, trim_h));
                trim.location = Vec3::new(w_x, r.min_y() + trim_t + bottom_len * 0.5, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
            let top_len = (r.max_y() - door_max_y - trim_t).max(0.0);
            if top_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_W_T", r.name), CsgOperation::Add, Vec3::new(trim_t, top_len, trim_h));
                trim.location = Vec3::new(w_x, door_max_y + top_len * 0.5, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
        } else {
            let len = (r.size.y - trim_t * 2.0).max(64.0);
            let mut trim = build_cube(format!("{}_Baseboard_W", r.name), CsgOperation::Add, Vec3::new(trim_t, len, trim_h));
            trim.location = Vec3::new(w_x, r.center.y, z);
            trim.group = Some("Architecture".to_string());
            for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
            trim.recompute_uvs();
            semisolid_detail_brushes.push(trim);
        }

        // 4. East Wall (+X)
        let e_x = r.center.x + r.size.x * 0.5 - trim_t * 0.5;
        let e_doorway = nodes.iter().find(|c| {
            c.name != r.name
                && c.center.x > r.center.x
                && (c.min_x() - r.max_x()).abs() <= 128.0
                && (r.max_y().min(c.max_y()) - r.min_y().max(c.min_y())) >= 64.0
        });
        if let Some(door) = e_doorway {
            let door_min_y = r.min_y().max(door.min_y());
            let door_max_y = r.max_y().min(door.max_y());
            let bottom_len = (door_min_y - r.min_y() - trim_t).max(0.0);
            if bottom_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_E_B", r.name), CsgOperation::Add, Vec3::new(trim_t, bottom_len, trim_h));
                trim.location = Vec3::new(e_x, r.min_y() + trim_t + bottom_len * 0.5, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
            let top_len = (r.max_y() - door_max_y - trim_t).max(0.0);
            if top_len >= 64.0 {
                let mut trim = build_cube(format!("{}_Baseboard_E_T", r.name), CsgOperation::Add, Vec3::new(trim_t, top_len, trim_h));
                trim.location = Vec3::new(e_x, door_max_y + top_len * 0.5, z);
                trim.group = Some("Architecture".to_string());
                for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
                trim.recompute_uvs();
                semisolid_detail_brushes.push(trim);
            }
        } else {
            let len = (r.size.y - trim_t * 2.0).max(64.0);
            let mut trim = build_cube(format!("{}_Baseboard_E", r.name), CsgOperation::Add, Vec3::new(trim_t, len, trim_h));
            trim.location = Vec3::new(e_x, r.center.y, z);
            trim.group = Some("Architecture".to_string());
            for p in &mut trim.polys { p.texture = pal.trim_texture.clone(); p.flags |= PF_SEMISOLID; }
            trim.recompute_uvs();
            semisolid_detail_brushes.push(trim);
        }
    }
}

/// Generates authentic sightline teaser apertures with semi-solid vertical security bars
/// between adjacent combat rooms separated by thin partitions (forensic feature from DM-Tempest, DM-Codex, CTF-Face).
fn generate_sightline_teasers(
    nodes: &[RoomNode],
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    sub_brushes: &mut Vec<Brush>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    if cfg.architectural_detail == WizardArchitecturalDetail::Clean {
        return;
    }

    let combat_rooms: Vec<&RoomNode> = nodes.iter().filter(|n| !n.is_corridor).collect();
    let corridors: Vec<&RoomNode> = nodes.iter().filter(|n| n.is_corridor).collect();
    let mut processed_pairs = std::collections::HashSet::new();

    for (i, a) in combat_rooms.iter().enumerate() {
        for (j, b) in combat_rooms.iter().enumerate().skip(i + 1) {
            // Floor levels must be roughly aligned
            let z_diff = (a.floor_z() - b.floor_z()).abs();
            if z_diff > 96.0 {
                continue;
            }

            let dx = (a.center.x - b.center.x).abs();
            let dy = (a.center.y - b.center.y).abs();

            let a_hx = a.size.x * 0.5;
            let a_hy = a.size.y * 0.5;
            let b_hx = b.size.x * 0.5;
            let b_hy = b.size.y * 0.5;

            // 1. East-West Adjacency
            let wall_thickness_x = dx - (a_hx + b_hx);
            let y_overlap = (a.center.y + a_hy).min(b.center.y + b_hy) - (a.center.y - a_hy).max(b.center.y - b_hy);

            if wall_thickness_x >= 32.0 && wall_thickness_x <= 160.0 && y_overlap >= 320.0 {
                if !processed_pairs.insert((i, j)) {
                    continue;
                }
                let mid_x = (a.center.x + b.center.x) * 0.5;
                let mid_y = ((a.center.y + a_hy).min(b.center.y + b_hy) + (a.center.y - a_hy).max(b.center.y - b_hy)) * 0.5;
                let eye_z = a.floor_z().max(b.floor_z()) + 72.0;

                // Don't place if a corridor already passes through here
                let near_corridor = corridors.iter().any(|c| (c.center.x - mid_x).hypot(c.center.y - mid_y) < 192.0);
                if near_corridor {
                    continue;
                }

                let ap_size = Vec3::new(wall_thickness_x + 32.0, 128.0, 80.0);
                let mut aperture = build_cube(
                    format!("SightlineAperture_{}_{}", a.name, b.name),
                    CsgOperation::Subtract,
                    ap_size,
                );
                aperture.location = Vec3::new(mid_x, mid_y, eye_z);
                aperture.group = Some("Architecture".to_string());
                for p in &mut aperture.polys {
                    p.texture = pal.trim_texture.clone();
                }
                aperture.recompute_uvs();
                sub_brushes.push(aperture);

                // 3 Vertical security bars (PF_SEMISOLID, 8x8x80 UU)
                for (b_idx, y_off) in [-32.0f32, 0.0f32, 32.0f32].into_iter().enumerate() {
                    let mut bar = build_cube(
                        format!("SightlineBar_{}_{}_{b_idx}", a.name, b.name),
                        CsgOperation::Add,
                        Vec3::new(8.0, 8.0, 80.0),
                    );
                    bar.location = Vec3::new(mid_x, mid_y + y_off, eye_z);
                    bar.group = Some("Architecture".to_string());
                    for p in &mut bar.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    bar.recompute_uvs();
                    semisolid_detail_brushes.push(bar);
                }
                continue;
            }

            // 2. North-South Adjacency
            let wall_thickness_y = dy - (a_hy + b_hy);
            let x_overlap = (a.center.x + a_hx).min(b.center.x + b_hx) - (a.center.x - a_hx).max(b.center.x - b_hx);

            if wall_thickness_y >= 32.0 && wall_thickness_y <= 160.0 && x_overlap >= 320.0 {
                if !processed_pairs.insert((i, j)) {
                    continue;
                }
                let mid_y = (a.center.y + b.center.y) * 0.5;
                let mid_x = ((a.center.x + a_hx).min(b.center.x + b_hx) + (a.center.x - a_hx).max(b.center.x - b_hx)) * 0.5;
                let eye_z = a.floor_z().max(b.floor_z()) + 72.0;

                let near_corridor = corridors.iter().any(|c| (c.center.x - mid_x).hypot(c.center.y - mid_y) < 192.0);
                if near_corridor {
                    continue;
                }

                let ap_size = Vec3::new(128.0, wall_thickness_y + 32.0, 80.0);
                let mut aperture = build_cube(
                    format!("SightlineAperture_{}_{}", a.name, b.name),
                    CsgOperation::Subtract,
                    ap_size,
                );
                aperture.location = Vec3::new(mid_x, mid_y, eye_z);
                aperture.group = Some("Architecture".to_string());
                for p in &mut aperture.polys {
                    p.texture = pal.trim_texture.clone();
                }
                aperture.recompute_uvs();
                sub_brushes.push(aperture);

                // 3 Vertical security bars (PF_SEMISOLID, 8x8x80 UU)
                for (b_idx, x_off) in [-32.0f32, 0.0f32, 32.0f32].into_iter().enumerate() {
                    let mut bar = build_cube(
                        format!("SightlineBar_{}_{}_{b_idx}", a.name, b.name),
                        CsgOperation::Add,
                        Vec3::new(8.0, 8.0, 80.0),
                    );
                    bar.location = Vec3::new(mid_x + x_off, mid_y, eye_z);
                    bar.group = Some("Architecture".to_string());
                    for p in &mut bar.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    bar.recompute_uvs();
                    semisolid_detail_brushes.push(bar);
                }
            }
        }
    }
}

/// Generates elevated perimeter catwalk galleries (+192 UU) with access stairs and sniper/armor vantage points
/// in high-ceiling combat rooms (forensic feature from DM-Deck16, DM-Morbias, DM-Turbine, DM-Liandri).
fn generate_room_catwalks(
    nodes: &[RoomNode],
    cfg: &MapWizardConfig,
    pal: &WizardResolvedPalette,
    semisolid_detail_brushes: &mut Vec<Brush>,
    actors: &mut Vec<WizardActorSpec>,
) {
    if cfg.architectural_detail == WizardArchitecturalDetail::Clean {
        return;
    }

    let is_epic = cfg.architectural_detail == WizardArchitecturalDetail::EpicHighDetail;

    for r in nodes.iter().filter(|n| !n.is_corridor) {
        // High ceiling requirement: at least 448 UU (so pawn has >= 240 UU headroom above +192 UU catwalk)
        // and spacious room: size >= 896x896 UU
        if r.size.x < 896.0 || r.size.y < 896.0 || r.size.z < 448.0 {
            continue;
        }

        let catwalk_z = r.floor_z() + 192.0;
        let shelf_w = 96.0_f32;
        let shelf_thick = 16.0_f32;
        let rail_h = 36.0_f32;
        let rail_thick = 8.0_f32;

        // Check which wall has NO intersecting rooms or corridors
        let opening_n = nodes.iter().find(|other| {
            other.name != r.name
                && other.min_y() <= r.max_y() + 16.0
                && other.max_y() >= r.max_y() - 16.0
                && other.max_x() > r.min_x() + 64.0
                && other.min_x() < r.max_x() - 64.0
        });
        let opening_s = nodes.iter().find(|other| {
            other.name != r.name
                && other.max_y() >= r.min_y() - 16.0
                && other.min_y() <= r.min_y() + 16.0
                && other.max_x() > r.min_x() + 64.0
                && other.min_x() < r.max_x() - 64.0
        });
        let opening_e = nodes.iter().find(|other| {
            other.name != r.name
                && other.min_x() <= r.max_x() + 16.0
                && other.max_x() >= r.max_x() - 16.0
                && other.max_y() > r.min_y() + 64.0
                && other.min_y() < r.max_y() - 64.0
        });
        let opening_w = nodes.iter().find(|other| {
            other.name != r.name
                && other.max_x() >= r.min_x() - 16.0
                && other.min_x() <= r.min_x() + 16.0
                && other.max_y() > r.min_y() + 64.0
                && other.min_y() < r.max_y() - 64.0
        });

        let has_opening_n = opening_n.is_some();
        let has_opening_s = opening_s.is_some();
        let has_opening_e = opening_e.is_some();
        let has_opening_w = opening_w.is_some();

        let stair_run = 288.0_f32; // 12 steps x 24 UU run = 288 UU (exact multiple of 8 and 24, on-grid)
        let stair_w = 96.0_f32;
        let stair_h = 192.0_f32; // 12 steps x 16 UU riser = 192 UU (standard UT99 step-up)

        enum CatwalkPlacement {
            FullNorth,
            FullSouth,
            FullEast,
            FullWest,
            CornerNorthWest(f32, f32), // (left_x, right_x)
        }

        let placement = if !has_opening_n {
            CatwalkPlacement::FullNorth
        } else if !has_opening_s {
            CatwalkPlacement::FullSouth
        } else if !has_opening_e {
            CatwalkPlacement::FullEast
        } else if !has_opening_w {
            CatwalkPlacement::FullWest
        } else if let Some(op) = opening_n {
            // All 4 walls have openings (e.g. 4-way central arena like ArenaCore).
            // Check if the North wall has a solid corner segment to the left of the opening.
            let left_span = op.min_x() - r.min_x();
            if left_span >= 448.0 {
                CatwalkPlacement::CornerNorthWest(r.min_x(), op.min_x())
            } else {
                continue;
            }
        } else {
            continue;
        };

        let (deck_loc, deck_size, rail_loc, rail_size, stair_loc, dir_stairs, foot_pos, top_pos, pickup_pos) = match placement {
            CatwalkPlacement::FullNorth => {
                let shelf_len = (r.size.x - 256.0).clamp(512.0, 1024.0);
                let deck_len = shelf_len - stair_run;
                let x_min = r.center.x - shelf_len * 0.5;
                let wall_y = r.center.y + r.size.y * 0.5 - shelf_w * 0.5;
                let stair_x = x_min + stair_run * 0.5;
                let deck_x = x_min + stair_run + deck_len * 0.5;
                let rail_y = wall_y - (shelf_w * 0.5 - rail_thick * 0.5);
                let foot_x = (x_min - 32.0).clamp(r.min_x() + 48.0, r.max_x() - 48.0);
                (
                    Vec3::new(deck_x, wall_y, catwalk_z - shelf_thick * 0.5),
                    Vec3::new(deck_len, shelf_w, shelf_thick),
                    Vec3::new(deck_x, rail_y, catwalk_z + rail_h * 0.5),
                    Vec3::new(deck_len, rail_thick, rail_h),
                    Vec3::new(stair_x, wall_y, r.floor_z() + stair_h * 0.5),
                    RampDirection::East,
                    Vec3::new(foot_x, wall_y, r.floor_z() + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 24.0),
                )
            }
            CatwalkPlacement::FullSouth => {
                let shelf_len = (r.size.x - 256.0).clamp(512.0, 1024.0);
                let deck_len = shelf_len - stair_run;
                let x_min = r.center.x - shelf_len * 0.5;
                let x_max = r.center.x + shelf_len * 0.5;
                let wall_y = r.center.y - r.size.y * 0.5 + shelf_w * 0.5;
                let stair_x = x_max - stair_run * 0.5;
                let deck_x = x_min + deck_len * 0.5;
                let rail_y = wall_y + (shelf_w * 0.5 - rail_thick * 0.5);
                let foot_x = (x_max + 32.0).clamp(r.min_x() + 48.0, r.max_x() - 48.0);
                (
                    Vec3::new(deck_x, wall_y, catwalk_z - shelf_thick * 0.5),
                    Vec3::new(deck_len, shelf_w, shelf_thick),
                    Vec3::new(deck_x, rail_y, catwalk_z + rail_h * 0.5),
                    Vec3::new(deck_len, rail_thick, rail_h),
                    Vec3::new(stair_x, wall_y, r.floor_z() + stair_h * 0.5),
                    RampDirection::West,
                    Vec3::new(foot_x, wall_y, r.floor_z() + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 24.0),
                )
            }
            CatwalkPlacement::FullEast => {
                let shelf_len = (r.size.y - 256.0).clamp(512.0, 1024.0);
                let deck_len = shelf_len - stair_run;
                let y_min = r.center.y - shelf_len * 0.5;
                let wall_x = r.center.x + r.size.x * 0.5 - shelf_w * 0.5;
                let stair_y = y_min + stair_run * 0.5;
                let deck_y = y_min + stair_run + deck_len * 0.5;
                let rail_x = wall_x - (shelf_w * 0.5 - rail_thick * 0.5);
                let foot_y = (y_min - 32.0).clamp(r.min_y() + 48.0, r.max_y() - 48.0);
                (
                    Vec3::new(wall_x, deck_y, catwalk_z - shelf_thick * 0.5),
                    Vec3::new(shelf_w, deck_len, shelf_thick),
                    Vec3::new(rail_x, deck_y, catwalk_z + rail_h * 0.5),
                    Vec3::new(rail_thick, deck_len, rail_h),
                    Vec3::new(wall_x, stair_y, r.floor_z() + stair_h * 0.5),
                    RampDirection::North,
                    Vec3::new(wall_x, foot_y, r.floor_z() + 40.0),
                    Vec3::new(wall_x, deck_y, catwalk_z + 40.0),
                    Vec3::new(wall_x, deck_y, catwalk_z + 24.0),
                )
            }
            CatwalkPlacement::FullWest => {
                let shelf_len = (r.size.y - 256.0).clamp(512.0, 1024.0);
                let deck_len = shelf_len - stair_run;
                let y_min = r.center.y - shelf_len * 0.5;
                let y_max = r.center.y + shelf_len * 0.5;
                let wall_x = r.center.x - r.size.x * 0.5 + shelf_w * 0.5;
                let stair_y = y_max - stair_run * 0.5;
                let deck_y = y_min + deck_len * 0.5;
                let rail_x = wall_x + (shelf_w * 0.5 - rail_thick * 0.5);
                let foot_y = (y_max + 32.0).clamp(r.min_y() + 48.0, r.max_y() - 48.0);
                (
                    Vec3::new(wall_x, deck_y, catwalk_z - shelf_thick * 0.5),
                    Vec3::new(shelf_w, deck_len, shelf_thick),
                    Vec3::new(rail_x, deck_y, catwalk_z + rail_h * 0.5),
                    Vec3::new(rail_thick, deck_len, rail_h),
                    Vec3::new(wall_x, stair_y, r.floor_z() + stair_h * 0.5),
                    RampDirection::South,
                    Vec3::new(wall_x, foot_y, r.floor_z() + 40.0),
                    Vec3::new(wall_x, deck_y, catwalk_z + 40.0),
                    Vec3::new(wall_x, deck_y, catwalk_z + 24.0),
                )
            }
            CatwalkPlacement::CornerNorthWest(min_x, max_x) => {
                // Corner placement along solid segment of North wall (left of doorway)
                let span = max_x - min_x;
                let margin = 32.0_f32;
                let deck_len = (span - stair_run - margin * 2.0).clamp(128.0, 512.0);
                let wall_y = r.center.y + r.size.y * 0.5 - shelf_w * 0.5;
                let deck_x = min_x + margin + deck_len * 0.5;
                let stair_x = min_x + margin + deck_len + stair_run * 0.5;
                let rail_y = wall_y - (shelf_w * 0.5 - rail_thick * 0.5);
                let foot_x = max_x - margin;
                (
                    Vec3::new(deck_x, wall_y, catwalk_z - shelf_thick * 0.5),
                    Vec3::new(deck_len, shelf_w, shelf_thick),
                    Vec3::new(deck_x, rail_y, catwalk_z + rail_h * 0.5),
                    Vec3::new(deck_len, rail_thick, rail_h),
                    Vec3::new(stair_x, wall_y, r.floor_z() + stair_h * 0.5),
                    RampDirection::West, // Ascends West towards corner deck
                    Vec3::new(foot_x, wall_y, r.floor_z() + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 40.0),
                    Vec3::new(deck_x, wall_y, catwalk_z + 24.0),
                )
            }
        };

        // 1. Catwalk Deck (PF_SEMISOLID)
        let mut deck = build_cube(
            format!("{}_CatwalkDeck", r.name),
            CsgOperation::Add,
            deck_size,
        );
        deck.location = deck_loc;
        deck.group = Some("Architecture".to_string());
        for p in &mut deck.polys {
            p.texture = pal.trim_texture.clone();
            p.flags |= PF_SEMISOLID;
        }
        deck.recompute_uvs();
        semisolid_detail_brushes.push(deck);

        // 2. Guard Rail along open side facing room (PF_SEMISOLID)
        let mut rail = build_cube(
            format!("{}_CatwalkRail", r.name),
            CsgOperation::Add,
            rail_size,
        );
        rail.location = rail_loc;
        rail.group = Some("Architecture".to_string());
        for p in &mut rail.polys {
            p.texture = pal.trim_texture.clone();
            p.flags |= PF_SEMISOLID;
        }
        rail.recompute_uvs();
        semisolid_detail_brushes.push(rail);

        // 3. Access Stepped Flight of Stairs from room floor (Z = floor_z) to catwalk (Z = floor_z + 192)
        let stair_brush_size = match placement {
            CatwalkPlacement::FullEast | CatwalkPlacement::FullWest => Vec3::new(stair_w, stair_run, stair_h),
            _ => Vec3::new(stair_run, stair_w, stair_h),
        };

        let mut stairs = build_stepped_stairs_oriented(
            format!("{}_CatwalkStairs", r.name),
            CsgOperation::Add,
            stair_brush_size,
            dir_stairs,
            12, // 12 steps of 16 UU = 192 UU
        );
        stairs.location = stair_loc;
        stairs.group = Some("Architecture".to_string());
        apply_room_textures(&mut stairs, pal, false, false);
        for p in &mut stairs.polys {
            if p.plane.normal.z > 0.7 {
                p.texture = pal.trim_texture.clone();
            } else if p.plane.normal.z.abs() < 0.2 {
                p.texture = pal.wall_texture.clone();
            }
            p.flags |= PF_SEMISOLID;
        }
        stairs.recompute_uvs();
        semisolid_detail_brushes.push(stairs);

        // Smooth collision hull for stairs
        let mut col_ramp = build_ramp_oriented(
            format!("{}_CatwalkStairsCol", r.name),
            CsgOperation::Add,
            stair_brush_size,
            dir_stairs,
        );
        col_ramp.location = stair_loc;
        col_ramp.group = Some("Architecture".to_string());
        for p in &mut col_ramp.polys {
            p.texture = pal.floor_texture.clone();
            p.flags |= PF_SEMISOLID | PF_INVISIBLE;
        }
        col_ramp.recompute_uvs();
        semisolid_detail_brushes.push(col_ramp);

        // 4. Catwalk Pickup & Navigation
        if cfg.toggles.include_bot_ai_paths {
            actors.push(WizardActorSpec {
                name: format!("PathNode_{}_Catwalk_Foot", r.name),
                class_name: "PathNode".to_string(),
                location: foot_pos,
                group: "AI_Navigation".to_string(),
                ..Default::default()
            });
            actors.push(WizardActorSpec {
                name: format!("PathNode_{}_Catwalk_Top", r.name),
                class_name: "PathNode".to_string(),
                location: top_pos,
                group: "AI_Navigation".to_string(),
                ..Default::default()
            });
        }

        // Vantage high-tier pickup (Armor2 / ThighPads)
        if !cfg.item_quota.enabled {
            let pickup_cls = if is_epic { "UT_ShieldBelt" } else { "Armor2" };
            actors.push(WizardActorSpec {
                name: format!("{}_{}_Pickup", r.name, pickup_cls),
                class_name: pickup_cls.to_string(),
                location: pickup_pos,
                group: "Items".to_string(),
                ..Default::default()
            });
        }

        // Limit to 1 major room in Standard, 2 in Epic
        if !is_epic {
            break;
        }
    }
}

/// Generates a complete, 100% valid, watertight UT99 map according to `cfg`.
pub fn generate_valid_ut99_map(cfg: &MapWizardConfig) -> WizardGeneratedMap {
    let mut safe_cfg = cfg.clone();
    auto_fix_wizard_teleporters(&mut safe_cfg);
    let cfg = &safe_cfg;

    let pal = cfg.active_palette();
    let nodes = build_room_nodes(cfg);
    let preview = preview_wizard_topology(cfg);

    let mut sub_brushes: Vec<Brush> = Vec::new();
    let mut add_structural_brushes: Vec<Brush> = Vec::new();
    let mut portal_sheet_brushes: Vec<Brush> = Vec::new();
    let mut semisolid_detail_brushes: Vec<Brush> = Vec::new();
    let mut movers: Vec<WizardMoverSpec> = Vec::new();
    let mut actors: Vec<WizardActorSpec> = Vec::new();
    let mut warnings: Vec<String> = preview.warnings.clone();

    // -----------------------------------------------------------------------
    // PHASE 1: Subtractive World Geometry (Rooms, Corridors, Hazard Pits, Skybox)
    // -----------------------------------------------------------------------
    let mut main_rooms: Vec<RoomNode> = Vec::new();
    for r in &nodes {
        let mut room_brush = if r.is_octagonal && !r.is_corridor {
            build_aligned_octagon(&r.name, CsgOperation::Subtract, r.size)
        } else {
            build_cube(&r.name, CsgOperation::Subtract, r.size)
        };
        room_brush.location = r.center;
        room_brush.group = Some(if r.is_corridor {
            "Corridors".to_string()
        } else {
            "Architecture".to_string()
        });
        apply_room_textures(
            &mut room_brush,
            &pal,
            r.sky_ceiling && cfg.toggles.include_skybox,
            r.is_corridor,
        );
        sub_brushes.push(room_brush);

        // If this room has a hazard basin, carve a recessed central pit and add a translucent fluid sheet + WaterZone
        if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
            let pit_w = (r.size.x * 0.32).clamp(192.0, 352.0);
            let pit_l = (r.size.y * 0.32).clamp(192.0, 352.0);
            let pit_d = 48.0_f32;
            let pit_center = Vec3::new(r.center.x, r.center.y, r.floor_z() - pit_d * 0.5);

            let mut pit = build_cube(
                format!("{}_HazardPit", r.name),
                CsgOperation::Subtract,
                Vec3::new(pit_w, pit_l, pit_d),
            );
            pit.location = pit_center;
            pit.group = Some("Hazards".to_string());
            apply_room_textures(&mut pit, &pal, false, false);
            sub_brushes.push(pit);

            let mut sheet = build_sheet_advanced(
                format!("{}_FluidSheet", r.name),
                CsgOperation::Add,
                pit_w,
                pit_l,
                SheetAxis::XY,
                SheetPreset::AcidSlime,
            );
            sheet.location = Vec3::new(r.center.x, r.center.y, r.floor_z() - 8.0);
            sheet.group = Some("Hazards".to_string());
            for p in &mut sheet.polys {
                p.texture = pal.liquid_texture.clone();
                p.flags = PF_NOT_SOLID | PF_TRANSLUCENT | PF_TWO_SIDED | PF_UNLIT;
            }
            sheet.recompute_uvs();
            add_structural_brushes.push(sheet);
        }

        if !r.is_corridor {
            // Named ZoneInfo / Hazard Zone for HUD location identification and per-room ambience
            let env = cfg.get_room_environment(&r.name);
            let mut zi = match env {
                WizardRoomEnvironment::Water => WizardActorSpec {
                    name: format!("{}_WaterZone", r.name),
                    class_name: "WaterZone".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.center.z - 32.0),
                    group: "Architecture".to_string(),
                    ..Default::default()
                },
                WizardRoomEnvironment::Slime => WizardActorSpec {
                    name: format!("{}_SlimeZone", r.name),
                    class_name: "SlimeZone".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.center.z - 32.0),
                    group: "Architecture".to_string(),
                    ..Default::default()
                },
                WizardRoomEnvironment::Lava => WizardActorSpec {
                    name: format!("{}_LavaZone", r.name),
                    class_name: "LavaZone".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.center.z - 32.0),
                    group: "Architecture".to_string(),
                    ..Default::default()
                },
                WizardRoomEnvironment::LowGravity => {
                    let mut spec = WizardActorSpec {
                        name: format!("{}_ZoneInfo", r.name),
                        class_name: "ZoneInfo".to_string(),
                        location: Vec3::new(r.center.x, r.center.y, r.center.z - 32.0),
                        group: "Architecture".to_string(),
                        ..Default::default()
                    };
                    spec.vec_props.push(("ZoneGravity".to_string(), Vec3::new(0.0, 0.0, -200.0)));
                    spec
                }
                WizardRoomEnvironment::Normal => WizardActorSpec {
                    name: format!("{}_ZoneInfo", r.name),
                    class_name: "ZoneInfo".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.center.z - 32.0),
                    group: "Architecture".to_string(),
                    ..Default::default()
                },
            };
            zi.str_props.push(("ZoneName".to_string(), r.name.clone()));
            zi.byte_props
                .push(("AmbientBrightness".to_string(), pal.ambient_brightness));
            zi.byte_props.push(("AmbientHue".to_string(), pal.ambient_hue));
            zi.byte_props
                .push(("AmbientSaturation".to_string(), pal.ambient_saturation));

            if cfg.distance_fog != WizardFogPreset::None {
                zi.bool_props.push(("bDistanceFog".to_string(), true));
                let (fog_r, fog_g, fog_b, fog_dist) = cfg.distance_fog.properties();
                zi.byte_props.push(("FogColor_R".to_string(), fog_r));
                zi.byte_props.push(("FogColor_G".to_string(), fog_g));
                zi.byte_props.push(("FogColor_B".to_string(), fog_b));
                zi.float_props.push(("FogDistance".to_string(), fog_dist));
            }

            actors.push(zi);
        }

        if !r.is_corridor {
            main_rooms.push(r.clone());
        }
    }

    // Skybox Reference Chamber (Isolated at (-6144, -6144, 2048) so it never intersects arena geometry)
    if cfg.toggles.include_skybox {
        let sky_origin = Vec3::new(-6144.0, -6144.0, 2048.0);
        let mut sky_cube = build_cube(
            "SkyBoxReferenceChamber",
            CsgOperation::Subtract,
            Vec3::new(1024.0, 1024.0, 1024.0),
        );
        sky_cube.location = sky_origin;
        sky_cube.group = Some("SkyBox".to_string());
        for p in &mut sky_cube.polys {
            p.texture = pal.sky_texture.clone();
        }
        sky_cube.recompute_uvs();
        sub_brushes.push(sky_cube);

        actors.push(WizardActorSpec {
            name: "SkyZoneInfo0".to_string(),
            class_name: "SkyZoneInfo".to_string(),
            location: sky_origin,
            group: "SkyBox".to_string(),
            ..Default::default()
        });

        actors.push(WizardActorSpec {
            name: "SkyBoxLight0".to_string(),
            class_name: "Light".to_string(),
            location: sky_origin + Vec3::new(160.0, 160.0, 192.0),
            group: "SkyBox".to_string(),
            light_brightness: 180,
            light_radius: 96,
            light_hue: pal.primary_light_hue,
            light_saturation: pal.primary_light_sat,
            ..Default::default()
        });

        // Multi-Layer Celestial Skybox & Ambient Fill Lighting (Standard & Epic)
        if cfg.architectural_detail != WizardArchitecturalDetail::Clean {
            // Secondary ambient celestial fill light (cool twilight azure)
            actors.push(WizardActorSpec {
                name: "SkyBoxFillLight".to_string(),
                class_name: "Light".to_string(),
                location: sky_origin + Vec3::new(-160.0, -160.0, -64.0),
                group: "SkyBox".to_string(),
                light_brightness: 90,
                light_radius: 96,
                light_hue: 160, // cool azure
                light_saturation: 128,
                ..Default::default()
            });

            // 1. Panning Celestial Cloud Sheet (PF_TRANSLUCENT | PF_NOT_SOLID)
            let mut cloud_sheet = build_sheet_advanced(
                "SkyBoxCloudSheet_0",
                CsgOperation::Add,
                960.0,
                960.0,
                SheetAxis::XY,
                SheetPreset::Normal,
            );
            cloud_sheet.location = sky_origin + Vec3::new(0.0, 0.0, 128.0);
            cloud_sheet.group = Some("SkyBox".to_string());
            for p in &mut cloud_sheet.polys {
                p.texture = pal.sky_texture.clone();
                p.flags |= PF_TRANSLUCENT | PF_NOT_SOLID;
            }
            cloud_sheet.recompute_uvs();
            semisolid_detail_brushes.push(cloud_sheet);

            // 2. Distant Celestial Planetoid / Moon (PF_SEMISOLID | PF_UNLIT)
            let mut moon = build_aligned_octagon(
                "SkyBoxCelestialBody",
                CsgOperation::Add,
                Vec3::new(96.0, 96.0, 16.0),
            );
            moon.location = sky_origin + Vec3::new(280.0, 280.0, 180.0);
            moon.group = Some("SkyBox".to_string());
            for p in &mut moon.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID | PF_UNLIT;
            }
            moon.recompute_uvs();
            semisolid_detail_brushes.push(moon);

            // 3. Secondary High-Altitude Cloud Layer for EpicHighDetail
            if cfg.architectural_detail == WizardArchitecturalDetail::EpicHighDetail {
                let mut cloud_sheet_hi = build_sheet_advanced(
                    "SkyBoxCloudSheet_1",
                    CsgOperation::Add,
                    896.0,
                    896.0,
                    SheetAxis::XY,
                    SheetPreset::Normal,
                );
                cloud_sheet_hi.location = sky_origin + Vec3::new(0.0, 0.0, 256.0);
                cloud_sheet_hi.group = Some("SkyBox".to_string());
                for p in &mut cloud_sheet_hi.polys {
                    p.texture = pal.sky_texture.clone();
                    p.flags |= PF_TRANSLUCENT | PF_NOT_SOLID;
                }
                cloud_sheet_hi.recompute_uvs();
                semisolid_detail_brushes.push(cloud_sheet_hi);
            }
        }
    }

    // Authentic Secret Armory Alcove (Concealed flush sliding wall panel hiding mega-pickup)
    if cfg.toggles.include_elevators_and_movers && main_rooms.len() >= 3 {
        // Do not place a secret at the bottom of a hazardous or deeply sunken
        // room: an alcove there can sit below the room's usable routes with no
        // way for players or bots to reach it. Until route-aware placement is
        // available, omit this optional feature when no safer host exists.
        if let Some(sec_r) = main_rooms.iter().find(|r| {
            !r.is_octagonal && !r.has_hazard_pit && r.floor_z() >= -128.0
                && r.size.x >= 512.0 && r.size.y >= 512.0
        }) {
            let candidates = [
                // North (+Y)
                (
                    Vec3::new(192.0, 160.0, 144.0),
                    Vec3::new(sec_r.center.x, sec_r.center.y + sec_r.size.y * 0.5 + 80.0, sec_r.floor_z() + 72.0),
                    Vec3::new(sec_r.center.x, sec_r.center.y + sec_r.size.y * 0.5, sec_r.floor_z() + 64.0),
                    Vec3::new(96.0, 16.0, 128.0),
                ),
                // South (-Y)
                (
                    Vec3::new(192.0, 160.0, 144.0),
                    Vec3::new(sec_r.center.x, sec_r.center.y - sec_r.size.y * 0.5 - 80.0, sec_r.floor_z() + 72.0),
                    Vec3::new(sec_r.center.x, sec_r.center.y - sec_r.size.y * 0.5, sec_r.floor_z() + 64.0),
                    Vec3::new(96.0, 16.0, 128.0),
                ),
                // East (+X)
                (
                    Vec3::new(160.0, 192.0, 144.0),
                    Vec3::new(sec_r.center.x + sec_r.size.x * 0.5 + 80.0, sec_r.center.y, sec_r.floor_z() + 72.0),
                    Vec3::new(sec_r.center.x + sec_r.size.x * 0.5, sec_r.center.y, sec_r.floor_z() + 64.0),
                    Vec3::new(16.0, 96.0, 128.0),
                ),
                // West (-X)
                (
                    Vec3::new(160.0, 192.0, 144.0),
                    Vec3::new(sec_r.center.x - sec_r.size.x * 0.5 - 80.0, sec_r.center.y, sec_r.floor_z() + 72.0),
                    Vec3::new(sec_r.center.x - sec_r.size.x * 0.5, sec_r.center.y, sec_r.floor_z() + 64.0),
                    Vec3::new(16.0, 96.0, 128.0),
                ),
            ];

            if let Some(&(alcove_size, alcove_center, door_loc, door_size)) = candidates.iter().find(|(asize, acenter, _, _)| {
                let c_min = *acenter - *asize * 0.5;
                let c_max = *acenter + *asize * 0.5;
                !nodes.iter().any(|n| {
                    if n.name == sec_r.name {
                        return false;
                    }
                    let n_min = n.center - n.size * 0.5;
                    let n_max = n.center + n.size * 0.5;
                    c_min.x < n_max.x && c_max.x > n_min.x
                        && c_min.y < n_max.y && c_max.y > n_min.y
                })
            }) {
                let mut alcove_brush = build_cube(
                    "SecretArmory_Alcove".to_string(),
                    CsgOperation::Subtract,
                    alcove_size,
                );
                alcove_brush.location = alcove_center;
                alcove_brush.group = Some("Secrets".to_string());
                apply_room_textures(&mut alcove_brush, &pal, false, false);
                sub_brushes.push(alcove_brush);

                let mut slab_brush = build_cube(
                    "SecretArmory_DoorMover".to_string(),
                    CsgOperation::Active,
                    door_size,
                );
                slab_brush.location = door_loc;
                slab_brush.group = Some("Movers".to_string());
                for p in &mut slab_brush.polys {
                    p.texture = pal.wall_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                slab_brush.recompute_uvs();

                let sec_tag = "SecretArmoryDoor".to_string();
                movers.push(WizardMoverSpec {
                    name: "Mover_SecretArmory".to_string(),
                    brush: slab_brush,
                    key0_location: door_loc,
                    key1_location: door_loc + Vec3::new(0.0, 0.0, -128.0),
                    initial_state: "BumpOpenTimed".to_string(),
                    move_time: 1.2,
                    stay_open_time: 4.5,
                    tag: sec_tag.clone(),
                    event: String::new(),
                    opening_sound: "DoorsMod.General.door2sta".to_string(),
                    closed_sound: "DoorsMod.General.door2end".to_string(),
                });

                // Anti-entrapment internal return trigger inside the alcove
                let mut ret_trig = WizardActorSpec {
                    name: "Secret_ReturnTrigger".to_string(),
                    class_name: "Trigger".to_string(),
                    location: alcove_center,
                    event: sec_tag,
                    group: "Movers".to_string(),
                    ..Default::default()
                };
                ret_trig.float_props.push(("CollisionRadius".to_string(), 64.0));
                ret_trig.float_props.push(("CollisionHeight".to_string(), 48.0));
                actors.push(ret_trig);

                // Secret powerup inside alcove (only if explicit quotas are not overriding)
                if !cfg.item_quota.enabled {
                    let powerup_cls = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
                        "UT_ShieldBelt"
                    } else {
                        "UDamage"
                    };
                    actors.push(WizardActorSpec {
                        name: "Secret_MegaPowerup".to_string(),
                        class_name: powerup_cls.to_string(),
                        location: Vec3::new(alcove_center.x, alcove_center.y, sec_r.floor_z() + 24.0),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }

                // Secret amber mood light inside alcove
                let mut alc_light = WizardActorSpec {
                    name: "Light_Secret_Alcove".to_string(),
                    class_name: "Light".to_string(),
                    location: Vec3::new(alcove_center.x, alcove_center.y, alcove_center.z + 32.0),
                    group: "Lighting".to_string(),
                    light_brightness: 120,
                    light_radius: 24,
                    light_hue: 35,
                    light_saturation: 128,
                    ..Default::default()
                };
                alc_light.byte_props.push(("LightType".to_string(), 1));
                alc_light.byte_props.push(("LightEffect".to_string(), 0));
                actors.push(alc_light);
            }
        }
    }

    // Recessed Ceiling Coffers & Unlit Light Panels (Carved into ceilings before any additive/semisolid brush)
    generate_ceiling_coffers(
        &nodes,
        cfg,
        &pal,
        &mut sub_brushes,
        &mut semisolid_detail_brushes,
        &mut actors,
    );

    // Sightline Teaser Cutout Windows with Security Bars between adjacent combat rooms
    generate_sightline_teasers(
        &nodes,
        cfg,
        &pal,
        &mut sub_brushes,
        &mut semisolid_detail_brushes,
    );

    // -----------------------------------------------------------------------
    // PHASE 2: Structural & Semi-Solid Architectural Details, Ramps & Elevators
    // -----------------------------------------------------------------------
    // Additive Solid Floor Slabs for elevated rooms (floor_z > 16 UU) to guarantee solid walkable floors
    // even when two subtractive brushes abut or overlap vertically (e.g. multi-tier spires/towers)
    for r in &nodes {
        if !r.is_corridor && r.floor_z() > 16.0 {
            let slab_h = 32.0_f32;
            let mut slab = if r.is_octagonal {
                build_aligned_octagon(
                    format!("{}_FloorSlab", r.name),
                    CsgOperation::Add,
                    Vec3::new(r.size.x, r.size.y, slab_h),
                )
            } else {
                build_cube(
                    format!("{}_FloorSlab", r.name),
                    CsgOperation::Add,
                    Vec3::new(r.size.x, r.size.y, slab_h),
                )
            };
            slab.location = Vec3::new(r.center.x, r.center.y, r.floor_z() - slab_h * 0.5);
            slab.group = Some("Architecture".to_string());
            apply_room_textures(&mut slab, &pal, false, false);
            add_structural_brushes.push(slab);
        }
    }

    // Additive Walkway Bridges over hazard pits so corridors and room transits never leave players
    // stranded or dropping into an impassable hazard basin
    for r in &nodes {
        if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
            let pit_w = (r.size.x * 0.32).clamp(192.0, 352.0);
            let pit_l = (r.size.y * 0.32).clamp(192.0, 352.0);
            let pit_d = 48.0_f32;
            let bridge_thick = 16.0_f32;
            let bridge_w = 144.0_f32;
            let mut bridge = build_cube(
                format!("{}_HazardBridge", r.name),
                CsgOperation::Add,
                Vec3::new(pit_w + 96.0, bridge_w, bridge_thick),
            );
            bridge.location = Vec3::new(r.center.x, r.center.y, r.floor_z() - bridge_thick * 0.5);
            bridge.group = Some("Architecture".to_string());
            apply_room_textures(&mut bridge, &pal, false, false);
            add_structural_brushes.push(bridge);

            // Semi-solid safety guardrails along both flanks of the hazard bridge
            let rail_h = 36.0_f32;
            let rail_thick = 8.0_f32;
            for (r_i, sy) in [(-1.0f32), (1.0f32)].into_iter().enumerate() {
                let mut rail = build_cube(
                    format!("{}_HazardBridgeRail_{r_i}", r.name),
                    CsgOperation::Add,
                    Vec3::new(pit_w + 96.0, rail_thick, rail_h),
                );
                rail.location = Vec3::new(
                    r.center.x,
                    r.center.y + sy * (bridge_w * 0.5 - rail_thick * 0.5),
                    r.floor_z() + rail_h * 0.5,
                );
                rail.group = Some("Architecture".to_string());
                for p in &mut rail.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                rail.recompute_uvs();
                semisolid_detail_brushes.push(rail);
            }

            // Submerged egress stairs along both open pool flanks (North and South) so players
            // who fall off the bridge can walk or jump effortlessly out of the water basin
            let step_run = 48.0_f32;
            let step_width = (pit_w - 32.0).clamp(96.0, 192.0);
            for (e_i, sy) in [(-1.0f32), (1.0f32)].into_iter().enumerate() {
                let dir = if sy > 0.0 { RampDirection::North } else { RampDirection::South };
                let stair_size = Vec3::new(step_width, step_run, pit_d);
                let seam_y = r.center.y + sy * (pit_l * 0.5);
                let loc_y = seam_y - sy * (step_run * 0.5);
                let stair_loc = Vec3::new(r.center.x, loc_y, r.floor_z() - pit_d * 0.5);

                let mut egress_stairs = build_stepped_stairs_oriented(
                    format!("{}_HazardPitEgress_{e_i}", r.name),
                    CsgOperation::Add,
                    stair_size,
                    dir,
                    3, // 3 steps of 16 UU riser = 48 UU
                );
                egress_stairs.location = stair_loc;
                egress_stairs.group = Some("Architecture".to_string());
                apply_room_textures(&mut egress_stairs, &pal, false, false);
                for p in &mut egress_stairs.polys {
                    if p.plane.normal.z > 0.7 {
                        p.texture = pal.trim_texture.clone();
                    } else if p.plane.normal.z.abs() < 0.2 {
                        p.texture = pal.wall_texture.clone();
                    }
                    p.flags |= PF_SEMISOLID;
                }
                egress_stairs.recompute_uvs();
                semisolid_detail_brushes.push(egress_stairs);

                let mut col_ramp = build_ramp_oriented(
                    format!("{}_HazardPitEgressCol_{e_i}", r.name),
                    CsgOperation::Add,
                    stair_size,
                    dir,
                );
                col_ramp.location = stair_loc;
                col_ramp.group = Some("Architecture".to_string());
                for p in &mut col_ramp.polys {
                    p.texture = pal.floor_texture.clone();
                    p.flags |= PF_SEMISOLID | PF_INVISIBLE;
                }
                col_ramp.recompute_uvs();
                semisolid_detail_brushes.push(col_ramp);
            }
        }
    }

    // Sealed BSP Zone Portals across all corridor interfaces & Doorway Frames
    generate_zone_portals_and_doorway_frames(
        &nodes,
        cfg,
        &pal,
        &movers,
        &mut portal_sheet_brushes,
        &mut semisolid_detail_brushes,
    );

    // Perimeter Wall Baseboard Trims with doorway threshold clearances
    generate_perimeter_baseboard_trims(
        &nodes,
        cfg,
        &pal,
        &mut semisolid_detail_brushes,
    );

    // Elevated Perimeter Catwalk Galleries & Mezzanines in high-ceiling rooms
    generate_room_catwalks(
        &nodes,
        cfg,
        &pal,
        &mut semisolid_detail_brushes,
        &mut actors,
    );

    // Carved volumes, for the "every detail touches rock" checks (rules v2 and later).
    let anchored_details = cfg.generator_version >= GENERATOR_VERSION_PORTALS;
    let carved_bounds: Vec<(Vec3, Vec3)> = if anchored_details {
        sub_brushes.iter().map(brush_bounds).collect()
    } else {
        Vec::new()
    };

    let mut sniper_balcony_loc: Option<Vec3> = None;
    // Place authentic 3-tier classical/tournament columns (Base Plinth, Shaft, Capital)
    // inside major rectangular rooms (away from doorways)
    for r in nodes.iter().filter(|n| !n.is_corridor && !n.is_octagonal) {
        if r.size.x >= 768.0 && r.size.y >= 768.0 {
            let px = (r.size.x * 0.35).max(160.0);
            let py = (r.size.y * 0.35).max(160.0);
            for (i, (sx, sy)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
                .into_iter()
                .enumerate()
            {
                let base_h = 24.0_f32;
                let cap_h = 24.0_f32;
                let shaft_h = (r.size.z - base_h - cap_h).max(32.0);
                let pillar_x = r.center.x + sx * px;
                let pillar_y = r.center.y + sy * py;

                // 1. Plinth (Base)
                let mut plinth = build_cube(
                    format!("{}_PillarBase_{i}", r.name),
                    CsgOperation::Add,
                    Vec3::new(64.0, 64.0, base_h),
                );
                plinth.location = Vec3::new(pillar_x, pillar_y, r.floor_z() + base_h * 0.5);
                plinth.group = Some("Architecture".to_string());
                for p in &mut plinth.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                plinth.recompute_uvs();
                semisolid_detail_brushes.push(plinth);

                // 2. Shaft (Middle)
                let mut shaft = build_cube(
                    format!("{}_PillarShaft_{i}", r.name),
                    CsgOperation::Add,
                    Vec3::new(48.0, 48.0, shaft_h),
                );
                shaft.location = Vec3::new(pillar_x, pillar_y, r.floor_z() + base_h + shaft_h * 0.5);
                shaft.group = Some("Architecture".to_string());
                for p in &mut shaft.polys {
                    p.texture = pal.pillar_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                shaft.recompute_uvs();
                semisolid_detail_brushes.push(shaft);

                // 3. Capital (Header)
                let mut capital = build_cube(
                    format!("{}_PillarCap_{i}", r.name),
                    CsgOperation::Add,
                    Vec3::new(64.0, 64.0, cap_h),
                );
                capital.location = Vec3::new(pillar_x, pillar_y, r.ceiling_z() - cap_h * 0.5);
                capital.group = Some("Architecture".to_string());
                for p in &mut capital.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                capital.recompute_uvs();
                semisolid_detail_brushes.push(capital);
            }
        }

        // High ceilings (>= 384 UU) get structural ceiling cross-beams/rafters for authentic depth
        if r.size.z >= 384.0 && (r.size.x >= 512.0 || r.size.y >= 512.0) {
            let beam_h = 24.0_f32;
            let beam_w = 32.0_f32;
            let z = r.ceiling_z() - beam_h * 0.5;
            let (b_size, count, step) = if r.size.x >= r.size.y {
                let count = ((r.size.x / 384.0).floor() as usize).clamp(1, 4);
                let step = r.size.x / (count + 1) as f32;
                (Vec3::new(beam_w, r.size.y * 0.88, beam_h), count, step)
            } else {
                let count = ((r.size.y / 384.0).floor() as usize).clamp(1, 4);
                let step = r.size.y / (count + 1) as f32;
                (Vec3::new(r.size.x * 0.88, beam_w, beam_h), count, step)
            };

            for b_i in 1..=count {
                let (bx, by) = if r.size.x >= r.size.y {
                    (r.center.x - r.size.x * 0.5 + b_i as f32 * step, r.center.y)
                } else {
                    (r.center.x, r.center.y - r.size.y * 0.5 + b_i as f32 * step)
                };
                let mut beam = build_cube(
                    format!("{}_Rafter_{b_i}", r.name),
                    CsgOperation::Add,
                    b_size,
                );
                beam.location = Vec3::new(bx, by, z);
                beam.group = Some("Architecture".to_string());
                for p in &mut beam.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                beam.recompute_uvs();
                if anchored_details {
                    let (lo, hi) = brush_bounds(&beam);
                    if !detail_touches_solid(&carved_bounds, lo, hi) {
                        continue;
                    }
                }
                semisolid_detail_brushes.push(beam);
            }

            // High ceilings in major halls also receive perimeter crown moldings along the 4 walls
            if r.size.x >= 768.0 && r.size.y >= 768.0 {
                let crown_h = 24.0_f32;
                let crown_thick = 16.0_f32;
                let cz = r.ceiling_z() - crown_h * 0.5;

                // North crown
                let mut crown_n = build_cube(
                    format!("{}_Crown_N", r.name),
                    CsgOperation::Add,
                    Vec3::new(r.size.x * 0.82, crown_thick, crown_h),
                );
                crown_n.location = Vec3::new(r.center.x, r.center.y + r.size.y * 0.5 - crown_thick * 0.5, cz);
                crown_n.group = Some("Architecture".to_string());
                for p in &mut crown_n.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                crown_n.recompute_uvs();
                semisolid_detail_brushes.push(crown_n);

                // South crown
                let mut crown_s = build_cube(
                    format!("{}_Crown_S", r.name),
                    CsgOperation::Add,
                    Vec3::new(r.size.x * 0.82, crown_thick, crown_h),
                );
                crown_s.location = Vec3::new(r.center.x, r.center.y - r.size.y * 0.5 + crown_thick * 0.5, cz);
                crown_s.group = Some("Architecture".to_string());
                for p in &mut crown_s.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                crown_s.recompute_uvs();
                semisolid_detail_brushes.push(crown_s);

                // East crown
                let mut crown_e = build_cube(
                    format!("{}_Crown_E", r.name),
                    CsgOperation::Add,
                    Vec3::new(crown_thick, r.size.y * 0.82, crown_h),
                );
                crown_e.location = Vec3::new(r.center.x + r.size.x * 0.5 - crown_thick * 0.5, r.center.y, cz);
                crown_e.group = Some("Architecture".to_string());
                for p in &mut crown_e.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                crown_e.recompute_uvs();
                semisolid_detail_brushes.push(crown_e);

                // West crown
                let mut crown_w = build_cube(
                    format!("{}_Crown_W", r.name),
                    CsgOperation::Add,
                    Vec3::new(crown_thick, r.size.y * 0.82, crown_h),
                );
                crown_w.location = Vec3::new(r.center.x - r.size.x * 0.5 + crown_thick * 0.5, r.center.y, cz);
                crown_w.group = Some("Architecture".to_string());
                for p in &mut crown_w.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                crown_w.recompute_uvs();
                semisolid_detail_brushes.push(crown_w);
            }
        }
    }

    if let Some(hub) = main_rooms.first() {

        // Grounded Overwatch Mezzanine Deck + Convex Access Ramp (when ramps & bridges are enabled)
        if cfg.toggles.include_ramps_and_bridges {
            let p_y = hub.center.y + (hub.size.y * 0.32).clamp(240.0, 272.0);
            let deck_h = 32.0_f32;

            let mut deck = build_cube(
                "OverwatchDeck",
                CsgOperation::Add,
                Vec3::new(96.0, 96.0, deck_h),
            );
            deck.location = Vec3::new(hub.center.x, p_y, hub.floor_z() + deck_h * 0.5);
            deck.group = Some("Architecture".to_string());
            apply_room_textures(&mut deck, &pal, false, false);
            for p in &mut deck.polys {
                p.flags |= PF_SEMISOLID;
            }
            deck.recompute_uvs();
            semisolid_detail_brushes.push(deck);

            let mut ramp = build_ramp_oriented(
                "OverwatchRamp",
                CsgOperation::Add,
                Vec3::new(64.0, 96.0, deck_h),
                RampDirection::East,
            );
            ramp.location = Vec3::new(hub.center.x - 80.0, p_y, hub.floor_z() + deck_h * 0.5);
            ramp.group = Some("Architecture".to_string());
            apply_room_textures(&mut ramp, &pal, false, false);
            for p in &mut ramp.polys {
                p.flags |= PF_SEMISOLID;
            }
            ramp.recompute_uvs();
            semisolid_detail_brushes.push(ramp);

            if cfg.toggles.include_bot_ai_paths {
                actors.push(WizardActorSpec {
                    name: "PathNode_RampFoot".to_string(),
                    class_name: "PathNode".to_string(),
                    location: Vec3::new(hub.center.x - 144.0, p_y, hub.floor_z() + 40.0),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                });
                actors.push(WizardActorSpec {
                    name: "PathNode_RampDeck".to_string(),
                    class_name: "PathNode".to_string(),
                    location: Vec3::new(hub.center.x + 16.0, p_y, hub.floor_z() + deck_h + 40.0),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                });
            }
        }

        // If elevators & movers are enabled, add a synchronized vertical lift in the main hub
        if cfg.toggles.include_elevators_and_movers {
            let lift_x = ((hub.center.x - hub.size.x * 0.5 + 96.0) / 16.0).round() * 16.0;
            let lift_y = ((hub.center.y - hub.size.y * 0.18) / 16.0).round() * 16.0;
            let base_z = hub.floor_z() + 8.0;
            let travel_z = 256.0_f32.min(hub.size.z - 160.0).max(128.0);
            let lift_origin = Vec3::new(lift_x, lift_y, base_z);

            // Upper sniper balcony platform at top of lift
            let mut balcony = build_cube(
                "HubSniperBalcony",
                CsgOperation::Add,
                Vec3::new(192.0, 256.0, 16.0),
            );
            let balcony_loc = Vec3::new(lift_x + 160.0, lift_y, base_z + travel_z);
            balcony.location = balcony_loc;
            balcony.group = Some("Architecture".to_string());
            for p in &mut balcony.polys {
                p.texture = pal.trim_texture.clone();
            }
            balcony.recompute_uvs();
            add_structural_brushes.push(balcony);
            sniper_balcony_loc = Some(balcony_loc);

            // Parapet battlements / crenelated guardrails on HubSniperBalcony (North, East, South)
            let rail_h = 44.0_f32;
            let rail_thick = 12.0_f32;
            let balc_floor_z = balcony_loc.z + 8.0;
            let rail_center_z = balc_floor_z + rail_h * 0.5;

            // North safety rail
            let mut north_rail = build_cube(
                "HubSniperBalconyRail_N",
                CsgOperation::Add,
                Vec3::new(192.0, rail_thick, rail_h),
            );
            north_rail.location = Vec3::new(balcony_loc.x, balcony_loc.y + 128.0 - rail_thick * 0.5, rail_center_z);
            north_rail.group = Some("Architecture".to_string());
            for p in &mut north_rail.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID;
            }
            north_rail.recompute_uvs();
            semisolid_detail_brushes.push(north_rail);

            // South safety rail
            let mut south_rail = build_cube(
                "HubSniperBalconyRail_S",
                CsgOperation::Add,
                Vec3::new(192.0, rail_thick, rail_h),
            );
            south_rail.location = Vec3::new(balcony_loc.x, balcony_loc.y - 128.0 + rail_thick * 0.5, rail_center_z);
            south_rail.group = Some("Architecture".to_string());
            for p in &mut south_rail.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID;
            }
            south_rail.recompute_uvs();
            semisolid_detail_brushes.push(south_rail);

            // East overwatch battlement (facing arena)
            let mut east_rail = build_cube(
                "HubSniperBalconyRail_E",
                CsgOperation::Add,
                Vec3::new(rail_thick, 256.0 - rail_thick * 2.0, rail_h),
            );
            east_rail.location = Vec3::new(balcony_loc.x + 96.0 - rail_thick * 0.5, balcony_loc.y, rail_center_z);
            east_rail.group = Some("Architecture".to_string());
            for p in &mut east_rail.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID;
            }
            east_rail.recompute_uvs();
            semisolid_detail_brushes.push(east_rail);

            let mut mover_brush = build_cube(
                "WizardLiftMoverBrush0",
                CsgOperation::Active,
                Vec3::new(128.0, 128.0, 16.0),
            );
            mover_brush.location = lift_origin;
            for p in &mut mover_brush.polys {
                p.texture = pal.trim_texture.clone();
            }
            mover_brush.recompute_uvs();

            let lift_tag = "WizardMainLift".to_string();
            movers.push(WizardMoverSpec {
                name: "Mover0".to_string(),
                brush: mover_brush,
                key0_location: lift_origin,
                key1_location: lift_origin + Vec3::new(0.0, 0.0, travel_z),
                initial_state: "StandOpenTimed".to_string(),
                move_time: 1.5,
                stay_open_time: 3.5,
                tag: lift_tag.clone(),
                event: String::new(),
                opening_sound: "DoorsMod.General.mlift2sta".to_string(),
                closed_sound: "DoorsMod.General.mlift2end".to_string(),
            });

            if cfg.toggles.include_bot_ai_paths {
                let mut lc = WizardActorSpec {
                    name: "LiftCenter0".to_string(),
                    class_name: "LiftCenter".to_string(),
                    location: lift_origin + Vec3::new(0.0, 0.0, 40.0),
                    tag: lift_tag.clone(),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                };
                lc.name_props.push(("LiftTag".to_string(), lift_tag.clone()));
                actors.push(lc);

                let mut le_bot = WizardActorSpec {
                    name: "LiftExit0".to_string(),
                    class_name: "LiftExit".to_string(),
                    location: lift_origin + Vec3::new(128.0, 0.0, 40.0),
                    tag: lift_tag.clone(),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                };
                le_bot
                    .name_props
                    .push(("LiftTag".to_string(), lift_tag.clone()));
                actors.push(le_bot);

                let mut le_top = WizardActorSpec {
                    name: "LiftExit1".to_string(),
                    class_name: "LiftExit".to_string(),
                    location: balcony_loc + Vec3::new(-48.0, 0.0, 40.0),
                    tag: lift_tag.clone(),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                };
                le_top.name_props.push(("LiftTag".to_string(), lift_tag));
                actors.push(le_top);

                // Sniper DefensePoint on the elevated balcony
                let mut sniper_dp = WizardActorSpec {
                    name: "DefensePoint_BalconySniper".to_string(),
                    class_name: "DefensePoint".to_string(),
                    location: balcony_loc + Vec3::new(32.0, 0.0, 48.0),
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                };
                sniper_dp.bool_props.push(("bSniping".to_string(), true));
                sniper_dp.byte_props.push(("priority".to_string(), 2));
                actors.push(sniper_dp);
            }
        }
    }

    // Custom corridor sliding blast doors (TriggerOpenTimed with authentic proximity Trigger + sound)
    for conn in &cfg.custom_connections {
        if conn.has_blast_door {
            if let Some(r) = nodes.iter().find(|r| r.name.contains(&conn.id)) {
                let door_w = (conn.width - 32.0).clamp(64.0, 480.0);
                let door_h = (conn.height - 32.0).clamp(64.0, 480.0);
                let is_y_axis = r.size.y > r.size.x;
                let (brush_size, key1_offset) = if is_y_axis {
                    (Vec3::new(door_w, 24.0, door_h), Vec3::new(0.0, 0.0, door_h))
                } else {
                    (Vec3::new(24.0, door_w, door_h), Vec3::new(0.0, 0.0, door_h))
                };
                let mut door_brush = build_cube(
                    format!("BlastDoor_{}", conn.id),
                    CsgOperation::Active,
                    brush_size,
                );
                let door_loc = Vec3::new(r.center.x, r.center.y, r.floor_z() + door_h * 0.5);
                door_brush.location = door_loc;
                door_brush.group = Some("Movers".to_string());
                for p in &mut door_brush.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                door_brush.recompute_uvs();

                let door_tag = format!("BlastDoor_{}", conn.id);
                movers.push(WizardMoverSpec {
                    name: format!("Mover_Door_{}", conn.id),
                    brush: door_brush,
                    key0_location: door_loc,
                    key1_location: door_loc + key1_offset,
                    initial_state: "TriggerOpenTimed".to_string(),
                    move_time: 1.0,
                    stay_open_time: 3.0,
                    tag: door_tag.clone(),
                    event: String::new(),
                    opening_sound: "DoorsMod.General.door2sta".to_string(),
                    closed_sound: "DoorsMod.General.door2end".to_string(),
                });

                // Proximity Trigger actor so the door glides open smoothly on player approach
                let mut trig = WizardActorSpec {
                    name: format!("DoorTrigger_{}", conn.id),
                    class_name: "Trigger".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.floor_z() + 40.0),
                    event: door_tag,
                    group: "Movers".to_string(),
                    ..Default::default()
                };
                trig.float_props.push(("CollisionRadius".to_string(), 128.0));
                trig.float_props.push(("CollisionHeight".to_string(), 64.0));
                actors.push(trig);
            }
        }
    }

    // Semi-solid ceiling cross-beams in long corridors (adds architectural depth without narrowing floor clearance)
    // Symmetrically offset by +/- 128 UU from center to maintain >= 48 UU clearance from the central Zone Portal sheet
    for (c_idx, corr) in nodes.iter().filter(|n| n.is_corridor).enumerate() {
        let is_ew = corr.size.x >= 512.0 && corr.size.x > corr.size.y;
        let is_ns = corr.size.y >= 512.0 && corr.size.y > corr.size.x;
        if !is_ew && !is_ns {
            continue;
        }

        let beam_size = if is_ew {
            Vec3::new(32.0, corr.size.y, 16.0)
        } else {
            Vec3::new(corr.size.x, 32.0, 16.0)
        };

        for (b_i, sign) in [(-1.0f32), (1.0f32)].into_iter().enumerate() {
            let offset = sign * 128.0;
            let beam_loc = if is_ew {
                Vec3::new(corr.center.x + offset, corr.center.y, corr.ceiling_z() - 8.0)
            } else {
                Vec3::new(corr.center.x, corr.center.y + offset, corr.ceiling_z() - 8.0)
            };

            let mut beam = build_cube(
                format!("CorridorBeam_{c_idx}_{b_i}"),
                CsgOperation::Add,
                beam_size,
            );
            beam.location = beam_loc;
            beam.group = Some("Architecture".to_string());
            for p in &mut beam.polys {
                p.texture = pal.trim_texture.clone();
                p.flags |= PF_SEMISOLID;
            }
            beam.recompute_uvs();
            if anchored_details {
                let (lo, hi) = brush_bounds(&beam);
                if !detail_touches_solid(&carved_bounds, lo, hi) {
                    continue;
                }
            }
            if !portal_sheet_brushes.iter().any(|p| brushes_overlap(&beam, p)) {
                semisolid_detail_brushes.push(beam);
            }
        }
    }



    // -----------------------------------------------------------------------
    // PHASE 2.5: Automatic Vertical Reachability (Stairs, Ramps & AI Waypoints)
    // Guarantees that rooms and corridors at different elevations (|ΔZ| > 24 UU)
    // are connected by navigable step ramps or kinetic kickers with aligned PathNodes
    // so bots and players can traverse seamlessly without getting stuck.
    // -----------------------------------------------------------------------
    if cfg.toggles.ensure_vertical_reachability {
        let mut processed_pairs = std::collections::HashSet::new();
        let mut placed_transit_boxes: Vec<(Vec3, Vec3)> = Vec::new();

        for (i, node_a) in nodes.iter().enumerate() {
            for (j, node_b) in nodes.iter().enumerate().skip(i + 1) {
                // Never build transit ramps between two corridors; only between rooms and corridors, or adjacent rooms
                if node_a.is_corridor && node_b.is_corridor {
                    continue;
                }

                // If both are rooms (neither is a corridor), check if an intermediate corridor already bridges them
                if !node_a.is_corridor && !node_b.is_corridor {
                    let bridged_by_corridor = nodes.iter().any(|c| {
                        if !c.is_corridor {
                            return false;
                        }
                        let a_dx = (node_a.center.x - c.center.x).abs();
                        let a_dy = (node_a.center.y - c.center.y).abs();
                        let a_ov_x = (node_a.size.x + c.size.x) * 0.5 - a_dx;
                        let a_ov_y = (node_a.size.y + c.size.y) * 0.5 - a_dy;
                        let a_touch = (a_ov_x > -16.0 && a_ov_y > 48.0) || (a_ov_y > -16.0 && a_ov_x > 48.0);

                        let b_dx = (node_b.center.x - c.center.x).abs();
                        let b_dy = (node_b.center.y - c.center.y).abs();
                        let b_ov_x = (node_b.size.x + c.size.x) * 0.5 - b_dx;
                        let b_ov_y = (node_b.size.y + c.size.y) * 0.5 - b_dy;
                        let b_touch = (b_ov_x > -16.0 && b_ov_y > 48.0) || (b_ov_y > -16.0 && b_ov_x > 48.0);

                        a_touch && b_touch
                    });
                    if bridged_by_corridor {
                        continue;
                    }
                }

                let delta_z = (node_a.floor_z() - node_b.floor_z()).abs();
                if delta_z <= 24.0 {
                    continue; // Within standard bot step-up height (24 UU)
                }

                // Check XY adjacency / intersection
                let dx = (node_a.center.x - node_b.center.x).abs();
                let dy = (node_a.center.y - node_b.center.y).abs();
                let overlap_x = (node_a.size.x + node_b.size.x) * 0.5 - dx;
                let overlap_y = (node_a.size.y + node_b.size.y) * 0.5 - dy;

                let is_adjacent_x = overlap_x > -32.0 && overlap_y > 64.0;
                let is_adjacent_y = overlap_y > -32.0 && overlap_x > 64.0;

                if (!is_adjacent_x && !is_adjacent_y) || !processed_pairs.insert((i, j)) {
                    continue;
                }

                let (lower_node, higher_node) = if node_a.floor_z() < node_b.floor_z() {
                    (node_a, node_b)
                } else {
                    (node_b, node_a)
                };

                let low_floor = lower_node.floor_z();
                let high_floor = higher_node.floor_z();
                let elevation_diff = high_floor - low_floor;

                let (ramp_dir, ramp_size, ramp_loc, foot_loc, head_loc, step_count) = if dx >= dy {
                        // Connected along East/West axis
                        let asc_east = higher_node.center.x > lower_node.center.x;
                        let dir = if asc_east { RampDirection::East } else { RampDirection::West };
                        let width = overlap_y.clamp(96.0, 256.0);
                        // Seam is placed at the exact boundary of lower_node to ensure 0 gap to higher_node floor
                        let seam_x = if asc_east {
                            (lower_node.center.x + lower_node.size.x * 0.5).min(higher_node.center.x + higher_node.size.x * 0.5)
                        } else {
                            (lower_node.center.x - lower_node.size.x * 0.5).max(higher_node.center.x - higher_node.size.x * 0.5)
                        };
                        let mut max_run = (lower_node.size.x - 64.0).max(128.0);
                        if cfg.game_mode == WizardGameMode::CaptureTheFlag && lower_node.team_owner.is_some() {
                            let flag_clearance_run = ((seam_x - lower_node.center.x).abs() - 96.0).max(96.0);
                            max_run = max_run.min(flag_clearance_run);
                        }
                        let step_count = ((elevation_diff / 16.0).ceil() as usize).max(1);
                        let step_run = (((elevation_diff * 1.5).clamp(96.0, 512.0).min(max_run) / step_count as f32) / 8.0).round().max(2.0) * 8.0;
                        let run = (step_run * step_count as f32).min(max_run);
                        let overlap_mid_y = (lower_node.center.y + higher_node.center.y) * 0.5;
                        // Place stair flight inside lower_node so top step meets high_floor at the seam into higher_node
                        let loc_x = if asc_east {
                            seam_x - run * 0.5
                        } else {
                            seam_x + run * 0.5
                        };
                        let loc = Vec3::new(loc_x, overlap_mid_y, low_floor + elevation_diff * 0.5);

                        let foot = Vec3::new(
                            (if asc_east { seam_x - run - 32.0 } else { seam_x + run + 32.0 })
                                .clamp(lower_node.min_x() + 48.0, lower_node.max_x() - 48.0),
                            overlap_mid_y.clamp(lower_node.min_y() + 48.0, lower_node.max_y() - 48.0),
                            low_floor + 40.0,
                        );
                        let head = Vec3::new(
                            (if asc_east { seam_x + 32.0 } else { seam_x - 32.0 })
                                .clamp(higher_node.min_x() + 48.0, higher_node.max_x() - 48.0),
                            overlap_mid_y.clamp(higher_node.min_y() + 48.0, higher_node.max_y() - 48.0),
                            high_floor + 40.0,
                        );
                        (dir, Vec3::new(run, width, elevation_diff), loc, foot, head, step_count)
                    } else {
                        // Connected along North/South axis
                        let asc_north = higher_node.center.y > lower_node.center.y;
                        let dir = if asc_north { RampDirection::North } else { RampDirection::South };
                        let width = overlap_x.clamp(96.0, 256.0);
                        // Seam is placed at the exact boundary of lower_node to ensure 0 gap to higher_node floor
                        let seam_y = if asc_north {
                            (lower_node.center.y + lower_node.size.y * 0.5).min(higher_node.center.y + higher_node.size.y * 0.5)
                        } else {
                            (lower_node.center.y - lower_node.size.y * 0.5).max(higher_node.center.y - higher_node.size.y * 0.5)
                        };
                        let mut max_run = (lower_node.size.y - 64.0).max(128.0);
                        if cfg.game_mode == WizardGameMode::CaptureTheFlag && lower_node.team_owner.is_some() {
                            let flag_clearance_run = ((seam_y - lower_node.center.y).abs() - 96.0).max(96.0);
                            max_run = max_run.min(flag_clearance_run);
                        }
                        let step_count = ((elevation_diff / 16.0).ceil() as usize).max(1);
                        let step_run = (((elevation_diff * 1.5).clamp(96.0, 512.0).min(max_run) / step_count as f32) / 8.0).round().max(2.0) * 8.0;
                        let run = (step_run * step_count as f32).min(max_run);
                        let overlap_mid_x = (lower_node.center.x + higher_node.center.x) * 0.5;
                        // Place stair flight inside lower_node so top step meets high_floor at the seam into higher_node
                        let loc_y = if asc_north {
                            seam_y - run * 0.5
                        } else {
                            seam_y + run * 0.5
                        };
                        let loc = Vec3::new(overlap_mid_x, loc_y, low_floor + elevation_diff * 0.5);

                        let foot = Vec3::new(
                            overlap_mid_x.clamp(lower_node.min_x() + 48.0, lower_node.max_x() - 48.0),
                            (if asc_north { seam_y - run - 32.0 } else { seam_y + run + 32.0 })
                                .clamp(lower_node.min_y() + 48.0, lower_node.max_y() - 48.0),
                            low_floor + 40.0,
                        );
                        let head = Vec3::new(
                            overlap_mid_x.clamp(higher_node.min_x() + 48.0, higher_node.max_x() - 48.0),
                            (if asc_north { seam_y + 32.0 } else { seam_y - 32.0 })
                                .clamp(higher_node.min_y() + 48.0, higher_node.max_y() - 48.0),
                            high_floor + 40.0,
                        );
                        (dir, Vec3::new(width, run, elevation_diff), loc, foot, head, step_count)
                    };

                    // Safeguard: Never place a transit ramp encroaching a team base core (FlagBase) in CTF
                    let encroaches_flag_base = cfg.game_mode == WizardGameMode::CaptureTheFlag && nodes.iter().any(|n| {
                        n.team_owner.is_some()
                            && (ramp_loc.x - n.center.x).abs() < ramp_size.x * 0.5 + 80.0
                            && (ramp_loc.y - n.center.y).abs() < ramp_size.y * 0.5 + 80.0
                    });
                    if encroaches_flag_base {
                        continue;
                    }

                    // Overlap detection: never place a transit flight intersecting or nested inside an already placed stair
                    let overlaps_existing_transit = placed_transit_boxes.iter().any(|(pos, sz)| {
                        (ramp_loc.x - pos.x).abs() < (ramp_size.x + sz.x) * 0.5 - 24.0
                            && (ramp_loc.y - pos.y).abs() < (ramp_size.y + sz.y) * 0.5 - 24.0
                    });
                    if overlaps_existing_transit {
                        continue;
                    }

                    // Rules v3: the flight must leave walking headroom under every carved volume it
                    // crosses; a low corridor mouth next to an elevated room would otherwise swallow it.
                    if cfg.generator_version >= GENERATOR_VERSION_HEIGHTS {
                        let top_z = high_floor;
                        let blocked = nodes.iter().any(|n| {
                            (ramp_loc.x - n.center.x).abs() < (ramp_size.x + n.size.x) * 0.5
                                && (ramp_loc.y - n.center.y).abs() < (ramp_size.y + n.size.y) * 0.5
                                && n.floor_z() < top_z
                                && n.ceiling_z() > low_floor
                                && n.ceiling_z() - top_z < MIN_STAIR_HEADROOM
                        });
                        if blocked {
                            continue;
                        }
                    }
                    placed_transit_boxes.push((ramp_loc, ramp_size));

                    if elevation_diff <= 256.0 {
                        if cfg.architectural_detail == WizardArchitecturalDetail::Clean {
                        let mut ramp = build_ramp_oriented(
                            format!("TransitRamp_{}_{}", lower_node.name, higher_node.name),
                            CsgOperation::Add,
                            ramp_size,
                            ramp_dir,
                        );
                        ramp.location = ramp_loc;
                        ramp.group = Some("Architecture".to_string());
                        apply_room_textures(&mut ramp, &pal, false, false);
                        for p in &mut ramp.polys {
                            p.flags |= PF_SEMISOLID;
                        }
                        ramp.recompute_uvs();
                        semisolid_detail_brushes.push(ramp);
                    } else {
                        // StandardTournament & EpicHighDetail: True stepped architectural flight of stairs
                        let mut stairs = build_stepped_stairs_oriented(
                            format!("TransitStair_{}_{}", lower_node.name, higher_node.name),
                            CsgOperation::Add,
                            ramp_size,
                            ramp_dir,
                            step_count,
                        );
                        stairs.location = ramp_loc;
                        stairs.group = Some("Architecture".to_string());
                        apply_room_textures(&mut stairs, &pal, false, false);
                        for p in &mut stairs.polys {
                            if p.plane.normal.z > 0.7 {
                                p.texture = pal.trim_texture.clone();
                            } else if p.plane.normal.z.abs() < 0.2 {
                                p.texture = pal.wall_texture.clone();
                            }
                            p.flags |= PF_SEMISOLID;
                        }
                        stairs.recompute_uvs();
                        semisolid_detail_brushes.push(stairs);

                        // Smooth invisible collision ramp for zero camera bobbing in competitive UT99 play
                        let mut col_ramp = build_ramp_oriented(
                            format!("TransitStairCol_{}_{}", lower_node.name, higher_node.name),
                            CsgOperation::Add,
                            ramp_size,
                            ramp_dir,
                        );
                        col_ramp.location = ramp_loc;
                        col_ramp.group = Some("Architecture".to_string());
                        for p in &mut col_ramp.polys {
                            p.texture = pal.floor_texture.clone();
                            p.flags |= PF_SEMISOLID | PF_INVISIBLE;
                        }
                        col_ramp.recompute_uvs();
                        semisolid_detail_brushes.push(col_ramp);
                    }

                    if cfg.toggles.include_bot_ai_paths {
                        actors.push(WizardActorSpec {
                            name: format!("PathNode_Foot_{}_{}", lower_node.name, higher_node.name),
                            class_name: "PathNode".to_string(),
                            location: foot_loc,
                            group: "AI_Navigation".to_string(),
                            ..Default::default()
                        });
                        actors.push(WizardActorSpec {
                            name: format!("PathNode_Head_{}_{}", lower_node.name, higher_node.name),
                            class_name: "PathNode".to_string(),
                            location: head_loc,
                            group: "AI_Navigation".to_string(),
                            ..Default::default()
                        });
                    }
                } else if cfg.toggles.include_jump_pads {
                    // Extreme height difference (> 256 UU): place a kinetic Kicker at the flight foot + landing waypoint at flight head
                    let kick_z = (2.0 * 950.0 * elevation_diff).sqrt() + 100.0;
                    let kick_h = head_loc - foot_loc;
                    let kick_vel = Vec3::new(kick_h.x * 0.8, kick_h.y * 0.8, kick_z);

                    let mut kicker = WizardActorSpec {
                        name: format!("Kicker_{}_{}", lower_node.name, higher_node.name),
                        class_name: "Kicker".to_string(),
                        location: Vec3::new(foot_loc.x, foot_loc.y, low_floor + 24.0),
                        group: "JumpPads".to_string(),
                        ..Default::default()
                    };
                    kicker.vec_props.push(("KickVelocity".to_string(), kick_vel));
                    actors.push(kicker);

                    if cfg.toggles.include_bot_ai_paths {
                        let mut js = WizardActorSpec {
                            name: format!("JumpSpot_Landing_{}_{}", lower_node.name, higher_node.name),
                            class_name: "JumpSpot".to_string(),
                            location: head_loc,
                            group: "AI_Navigation".to_string(),
                            ..Default::default()
                        };
                        js.bool_props.push(("bImpactJump".to_string(), false));
                        actors.push(js);
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 3: Multi-Layer Lighting & Atmospheric Actors
    // Every room gets a ceiling light grid (~384uu spacing) so that no area is dark, plus focal lights
    // and hazard pit underglow. Accent lights keep a controlled saturation.
    // -----------------------------------------------------------------------
    if cfg.toggles.include_lighting {
        let accent_sat = pal.accent_light_sat.max(150);
        let is_torch_theme = matches!(
            cfg.theme,
            WizardThemePreset::CodexGothicCrypt
                | WizardThemePreset::SesmarEgyptianTomb
                | WizardThemePreset::MedievalCastle
                | WizardThemePreset::AncientMonastery
        );
        let (main_type, main_eff) = match cfg.theme {
            WizardThemePreset::PhobosOrbitalSciFi
            | WizardThemePreset::MorbiasCyberArena
            | WizardThemePreset::CoretTechFacility => (6u8, 0u8), // LT_SubtlePulse, LE_None
            _ => (1u8, 0u8), // LT_Steady, LE_None
        };

        let (corona_tex, corona_scale) = match cfg.theme {
            WizardThemePreset::PhobosOrbitalSciFi
            | WizardThemePreset::MorbiasCyberArena
            | WizardThemePreset::CoretTechFacility => ("GenFX.Lensflare", 0.45f32),
            WizardThemePreset::Deck16Industrial | WizardThemePreset::HeavyFoundry => {
                ("GenIn.LensFl1", 0.40f32)
            }
            WizardThemePreset::CodexGothicCrypt
            | WizardThemePreset::SesmarEgyptianTomb
            | WizardThemePreset::MedievalCastle
            | WizardThemePreset::AncientMonastery => {
                ("UnrealShare.Belt_fx.GoldShield", 0.35f32)
            }
            _ => ("GenIn.LensFl1", 0.40f32),
        };

        for (idx, r) in nodes.iter().enumerate() {
            let light_z = r.ceiling_z() - 64.0;
            let (hue, sat) = match r.team_owner {
                Some(0) => (0, 96),    // Red Team warm crimson tint
                Some(1) => (160, 96),  // Blue Team cool azure tint
                _ => (pal.primary_light_hue, pal.primary_light_sat),
            };
            let (room_eff, room_type) = if r.is_corridor {
                (17u8, 1u8) // LE_Cylinder, LT_Steady to strictly constrain light to corridor shaft
            } else if r.size.x >= 768.0 && r.size.y >= 768.0 {
                (13u8, main_type) // LE_NonIncidence for high-clarity tournament visibility
            } else {
                (main_eff, main_type)
            };
            let mut main_light = WizardActorSpec {
                name: format!("Light_Main_{idx}"),
                class_name: "Light".to_string(),
                location: Vec3::new(r.center.x, r.center.y, light_z),
                group: "Lighting".to_string(),
                light_brightness: pal.primary_light_bright,
                light_radius: if r.is_corridor { 40 } else { 64 },
                light_hue: hue,
                light_saturation: sat,
                ..Default::default()
            };
            main_light.byte_props.push(("LightType".to_string(), room_type));
            main_light.byte_props.push(("LightEffect".to_string(), room_eff));
            if !r.is_corridor {
                // Authentic UT99 Coronas (bCorona + Lens Flare billboards)
                main_light.bool_props.push(("bCorona".to_string(), true));
                main_light.str_props.push(("Corona".to_string(), corona_tex.to_string()));
                main_light.float_props.push(("DrawScale".to_string(), corona_scale));
            }
            actors.push(main_light);

            // Light grid along the long axis of corridors / over the floor of rooms
            let step = 384.0_f32;
            let nx = ((r.size.x / step).floor() as i32).max(1);
            let ny = ((r.size.y / step).floor() as i32).max(1);
            let mut k = 0usize;
            for ix in 0..nx {
                for iy in 0..ny {
                    let gx = r.center.x + (ix as f32 - (nx as f32 - 1.0) * 0.5) * step;
                    let gy = r.center.y + (iy as f32 - (ny as f32 - 1.0) * 0.5) * step;
                    // the central cell is already covered by the main light
                    if (gx - r.center.x).abs() < 1.0 && (gy - r.center.y).abs() < 1.0 {
                        continue;
                    }
                    let mut grid_light = WizardActorSpec {
                        name: format!("Light_Grid_{idx}_{k}"),
                        class_name: "Light".to_string(),
                        location: Vec3::new(gx, gy, light_z),
                        group: "Lighting".to_string(),
                        light_brightness: (pal.primary_light_bright as u16 * 9 / 10) as u8,
                        light_radius: if r.is_corridor { 36 } else { 48 },
                        light_hue: hue,
                        light_saturation: sat,
                        ..Default::default()
                    };
                    grid_light.byte_props.push(("LightType".to_string(), 1)); // LT_Steady
                    if r.is_corridor {
                        grid_light.byte_props.push(("LightEffect".to_string(), 17)); // LE_Cylinder
                    }
                    actors.push(grid_light);
                    k += 1;
                }
            }

            // Authentic wall torches in corridors for gothic/crypt/castle/monastery maps
            if is_torch_theme && r.is_corridor {
                let (torch_x, torch_y) = if r.size.x >= r.size.y {
                    let wall_offset = (r.size.y * 0.5 - 12.0).max(16.0);
                    (r.center.x, r.center.y + wall_offset)
                } else {
                    let wall_offset = (r.size.x * 0.5 - 12.0).max(16.0);
                    (r.center.x + wall_offset, r.center.y)
                };

                let mut torch = WizardActorSpec {
                    name: format!("TorchFlame_Corr_{idx}"),
                    class_name: "TorchFlame".to_string(),
                    location: Vec3::new(torch_x, torch_y, r.floor_z() + 56.0),
                    group: "Lighting".to_string(),
                    light_brightness: 160,
                    light_radius: 32,
                    light_hue: 20,
                    light_saturation: 32,
                    ..Default::default()
                };
                torch.byte_props.push(("LightEffect".to_string(), 1)); // LE_TorchWaver
                torch.byte_props.push(("LightType".to_string(), 3));   // LT_Flicker
                torch.bool_props.push(("bCorona".to_string(), true));
                torch.str_props.push(("Corona".to_string(), "Botpack.FlakGlow".to_string()));
                torch.float_props.push(("DrawScale".to_string(), 0.35));
                actors.push(torch);

                if cfg.architectural_detail != WizardArchitecturalDetail::Clean {
                    let mut bracket = build_cube(
                        format!("TorchBracket_Corr_{idx}"),
                        CsgOperation::Add,
                        Vec3::new(8.0, 12.0, 16.0),
                    );
                    bracket.location = Vec3::new(torch_x, torch_y, r.floor_z() + 40.0);
                    bracket.group = Some("Architecture".to_string());
                    for p in &mut bracket.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    bracket.recompute_uvs();
                    semisolid_detail_brushes.push(bracket);
                }
            }

            // Add corner accent lights with dynamic style for large rooms
            if !r.is_corridor && r.size.x >= 768.0 {
                let corner_f = if r.is_octagonal { 0.24 } else { 0.32 };
                let ox = r.size.x * corner_f;
                let oy = r.size.y * corner_f;
                let (acc_eff, acc_type) = match cfg.theme {
                    WizardThemePreset::CodexGothicCrypt
                    | WizardThemePreset::SesmarEgyptianTomb
                    | WizardThemePreset::MedievalCastle
                    | WizardThemePreset::AncientMonastery => (1u8, 3u8), // LE_TorchWaver, LT_Flicker
                    WizardThemePreset::PhobosOrbitalSciFi
                    | WizardThemePreset::MorbiasCyberArena
                    | WizardThemePreset::CoretTechFacility => (0u8, 6u8), // LE_None, LT_SubtlePulse
                    WizardThemePreset::HeavyFoundry | WizardThemePreset::Deck16Industrial => (1u8, 1u8), // LE_TorchWaver (heat shimmer), LT_Steady
                    _ => (0u8, 1u8), // LE_None, LT_Steady
                };
                for (c_idx, (dx, dy)) in [(-ox, -oy), (ox, oy)].into_iter().enumerate() {
                    // Rules v2: the fixture is a wall sconce on the nearest wall instead of a box hovering
                    // in mid-room; the light moves with it. Skipped (light stays) if that wall is not rock.
                    let mut mount: Option<(Vec3, Vec3)> = None;
                    if anchored_details && cfg.architectural_detail != WizardArchitecturalDetail::Clean {
                        let (px, py) = (r.center.x + dx, r.center.y + dy);
                        let walls = [
                            ((px - r.min_x()).abs(), true, r.min_x(), 1.0f32),
                            ((r.max_x() - px).abs(), true, r.max_x(), -1.0),
                            ((py - r.min_y()).abs(), false, r.min_y(), 1.0),
                            ((r.max_y() - py).abs(), false, r.max_y(), -1.0),
                        ];
                        let (_, on_x, wall, inward) = walls.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
                        let z = r.floor_z() + 128.0;
                        let (size, center, light_at) = if on_x {
                            (Vec3::new(12.0, 24.0, 12.0), Vec3::new(wall + inward * 6.0, py, z), Vec3::new(wall + inward * 28.0, py, z))
                        } else {
                            (Vec3::new(24.0, 12.0, 12.0), Vec3::new(px, wall + inward * 6.0, z), Vec3::new(px, wall + inward * 28.0, z))
                        };
                        if detail_touches_solid(&carved_bounds, center - size * 0.5, center + size * 0.5) {
                            mount = Some((center, light_at));
                        }
                    }
                    let light_base = mount.map(|(_, l)| l).unwrap_or(Vec3::new(r.center.x + dx, r.center.y + dy, r.floor_z() + 128.0));
                    let mut acc = WizardActorSpec {
                        name: format!("Light_Accent_{idx}_{c_idx}"),
                        class_name: "Light".to_string(),
                        location: light_base,
                        group: "Lighting".to_string(),
                        light_brightness: (pal.primary_light_bright as u16 * 3 / 4) as u8,
                        light_radius: 40,
                        light_hue: pal.accent_light_hue,
                        light_saturation: accent_sat,
                        ..Default::default()
                    };
                    acc.byte_props.push(("LightEffect".to_string(), acc_eff));
                    acc.byte_props.push(("LightType".to_string(), acc_type));

                    if cfg.architectural_detail != WizardArchitecturalDetail::Clean {
                        let (fixture_size, fixture_loc) = match mount {
                            Some((center, _)) => {
                                let on_x = (center.x - r.min_x()).abs() < 7.0 || (center.x - r.max_x()).abs() < 7.0;
                                (if on_x { Vec3::new(12.0, 24.0, 12.0) } else { Vec3::new(24.0, 12.0, 12.0) }, Some(center))
                            }
                            None => (Vec3::new(24.0, 24.0, 12.0), None),
                        };
                        // v2 without a rock-backed wall: no floating fixture at all.
                        if !anchored_details || fixture_loc.is_some() {
                            let mut fixture = build_cube(
                                format!("LightFixture_Accent_{idx}_{c_idx}"),
                                CsgOperation::Add,
                                fixture_size,
                            );
                            fixture.location = fixture_loc.unwrap_or(Vec3::new(r.center.x + dx, r.center.y + dy, r.floor_z() + 128.0));
                            fixture.group = Some("Architecture".to_string());
                            for p in &mut fixture.polys {
                                p.texture = pal.trim_texture.clone();
                                p.flags |= PF_SEMISOLID;
                            }
                            fixture.recompute_uvs();
                            semisolid_detail_brushes.push(fixture);
                        }

                        acc.bool_props.push(("bCorona".to_string(), true));
                        acc.str_props.push(("Corona".to_string(), corona_tex.to_string()));
                        acc.float_props.push(("DrawScale".to_string(), 0.3));
                    }

                    actors.push(acc);
                }
            }

            // Hazard pit underglow light inside recessed basin with liquid-specific shimmer/fire waver
            if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
                let (h_eff, h_type) = match cfg.theme {
                    WizardThemePreset::Deck16Industrial | WizardThemePreset::HeavyFoundry => (3u8, 1u8), // LE_WateryShimmer, LT_Steady
                    WizardThemePreset::VolcanicCaldera => (2u8, 1u8), // LE_FireWaver, LT_Steady
                    _ => (3u8, 6u8), // LE_WateryShimmer, LT_SubtlePulse
                };
                let mut haz = WizardActorSpec {
                    name: format!("Light_HazardGlow_{idx}"),
                    class_name: "Light".to_string(),
                    location: Vec3::new(r.center.x, r.center.y, r.floor_z() - 48.0),
                    group: "Lighting".to_string(),
                    light_brightness: 180,
                    light_radius: 54,
                    light_hue: pal.accent_light_hue,
                    light_saturation: 64,
                    ..Default::default()
                };
                haz.byte_props.push(("LightEffect".to_string(), h_eff));
                haz.byte_props.push(("LightType".to_string(), h_type));
                actors.push(haz);
            }
        }
    } else if let Some(first) = main_rooms.first() {
        // Minimal baseline light so LevelDoctor never flags an unlit map
        actors.push(WizardActorSpec {
            name: "Light0".to_string(),
            class_name: "Light".to_string(),
            location: Vec3::new(first.center.x, first.center.y, first.ceiling_z() - 64.0),
            group: "Lighting".to_string(),
            light_brightness: 140,
            light_radius: 64,
            light_hue: pal.primary_light_hue,
            light_saturation: pal.primary_light_sat,
            ..Default::default()
        });
    }

    // -----------------------------------------------------------------------
    // PHASE 4: PlayerStarts & Mode-Specific Tournament Objectives (DM, CTF, DOM, AS)
    // -----------------------------------------------------------------------
    let mut ps_idx = 0usize;
    let spawns_per_room = if cfg.game_mode == WizardGameMode::Duel1v1 { 1 } else { 2 };

    // Assault: plan the sequential objective graph (attackers -> gated objectives -> defenders).
    let assault_plan: Option<AssaultMissionPlan> = if cfg.game_mode == WizardGameMode::Assault {
        let mission_nodes: Vec<MissionNode> = nodes
            .iter()
            .map(|n| MissionNode {
                center: n.center,
                size: n.size,
                is_corridor: n.is_corridor,
            })
            .collect();
        Some(plan_assault_mission(
            &mission_nodes,
            cfg.assault_objective_count.clamp(2, 5),
            cfg.seed,
        ))
    } else {
        None
    };

    // Domination: plan the triangular control point circuit
    let domination_plan: Option<crate::domination_mission::DominationMissionPlan> =
        if cfg.game_mode == WizardGameMode::Domination {
            let dom_nodes: Vec<crate::domination_mission::DominationNode> = nodes
                .iter()
                .map(|n| crate::domination_mission::DominationNode {
                    center: n.center,
                    size: n.size,
                    is_corridor: n.is_corridor,
                })
                .collect();
            Some(crate::domination_mission::plan_domination_mission(
                &dom_nodes,
                cfg.theme,
                cfg.domination_naming_style,
                cfg.domination_point_count,
                cfg.seed,
            ))
        } else {
            None
        };

    // Capture The Flag: plan polar base rooms, multi-lane routes, and bot defense network
    let ctf_plan: Option<crate::ctf_mission::CtfMissionPlan> =
        if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            let ctf_nodes: Vec<crate::ctf_mission::CtfNode> = nodes
                .iter()
                .map(|n| crate::ctf_mission::CtfNode {
                    name: n.name.clone(),
                    center: n.center,
                    size: n.size,
                    is_corridor: n.is_corridor,
                    team_owner: n.team_owner,
                })
                .collect();
            Some(crate::ctf_mission::plan_ctf_mission(
                &ctf_nodes,
                cfg.ctf_dais_style,
                cfg.ctf_include_sniper_perches,
                cfg.ctf_contested_center_powerup,
                cfg.seed,
            ))
        } else {
            None
        };

    for (r_idx, r) in main_rooms.iter().enumerate() {
        // Assault spawn roles: 1 = attackers (start room), 0 = defenders (behind the first gate), 2 = none
        let assault_role: Option<u8> = assault_plan.as_ref().map(|p| {
            let is_node = |idx: usize| nodes.get(idx).map_or(false, |n| n.name == r.name);
            if is_node(p.start_node) {
                1
            } else if p.defender_nodes.iter().any(|&d| is_node(d)) {
                0
            } else {
                2
            }
        });
        if assault_role == Some(2) {
            continue;
        }

        // CTF spawn roles: strictly in team bases, never in midfield!
        let ctf_team: Option<u8> = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            if let Some(cp) = &ctf_plan {
                if nodes.get(cp.red_base_node_index).map_or(false, |n| n.name == r.name) || r.team_owner == Some(0) {
                    Some(0)
                } else if nodes.get(cp.blue_base_node_index).map_or(false, |n| n.name == r.name) || r.team_owner == Some(1) {
                    Some(1)
                } else {
                    None
                }
            } else {
                r.team_owner
            }
        } else {
            None
        };

        if r.is_corridor {
            continue; // PlayerStarts belong in combat rooms, never in narrow transit corridors or doorway frames
        }

        if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            let lower_name = r.name.to_ascii_lowercase();
            if lower_name.contains("sniper") || lower_name.contains("roost") || lower_name.contains("overlook") || lower_name.contains("perch") || lower_name.contains("balcony") {
                continue; // In authentic UT99 CTF, PlayerStarts belong on the main fortress ground floor, never in elevated sniper roosts
            }
        }

        if cfg.game_mode == WizardGameMode::CaptureTheFlag && ctf_team.is_none() {
            continue; // Skip midfield rooms in CTF
        }

        let spawns_here = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            4
        } else {
            match assault_role {
                Some(1) => 4,
                Some(_) => 3,
                None => spawns_per_room,
            }
        };
        let fz = r.floor_z() + 48.0; // Canonical +48 UU above floor to prevent collision snag
        let is_dom_cp_room = domination_plan
            .as_ref()
            .map_or(false, |dp| dp.points.iter().any(|p| nodes.get(p.room_index).map_or(false, |n| n.name == r.name)));
        if is_dom_cp_room {
            // In authentic UT99 Domination, PlayerStarts are never placed inside the ControlPoint
            // capture chamber itself to prevent instant spawn-capping and guarantee fair neutral
            // racing with standoff distance >= 500 UU. Spawns reside in adjacent staging wings,
            // staging chambers, and bypass corridors.
            continue;
        }
        let (mut spread_x, mut spread_y) = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            let max_sx = (r.size.x * 0.5 - 64.0).max(64.0);
            let max_sy = (r.size.y * 0.5 - 64.0).max(64.0);
            (
                snap_grid((r.size.x * 0.35).clamp(260.0, 360.0).min(max_sx), 16.0),
                snap_grid((r.size.y * 0.35).clamp(260.0, 360.0).min(max_sy), 16.0),
            )
        } else {
            let f = if r.is_octagonal { 0.16 } else { 0.18 };
            let max_sx = (r.size.x * 0.5 - 64.0).max(112.0);
            let max_sy = (r.size.y * 0.5 - 64.0).max(112.0);
            (
                snap_grid((r.size.x * f).clamp(112.0, 240.0).min(max_sx), 16.0),
                snap_grid((r.size.y * f).clamp(112.0, 240.0).min(max_sy), 16.0),
            )
        };
        if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
            let pit_w = (r.size.x * 0.32).clamp(192.0, 352.0);
            let pit_l = (r.size.y * 0.32).clamp(192.0, 352.0);
            let min_x = snap_grid(pit_w * 0.5 + 80.0, 16.0);
            let min_y = snap_grid(pit_l * 0.5 + 80.0, 16.0);
            let max_f = if r.is_octagonal { 0.28 } else { 0.40 };
            spread_x = spread_x.max(min_x).min(snap_grid(r.size.x * max_f, 16.0));
            spread_y = spread_y.max(min_y).min(snap_grid(r.size.y * max_f, 16.0));
        }
        let offsets = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            [
                Vec3::new(-spread_x, -spread_y, 0.0),
                Vec3::new(spread_x, -spread_y, 0.0),
                Vec3::new(-spread_x, spread_y, 0.0),
                Vec3::new(spread_x, spread_y, 0.0),
            ]
        } else {
            [
                Vec3::new(-spread_x, -spread_y, 0.0),
                Vec3::new(spread_x, spread_y, 0.0),
                Vec3::new(-spread_x, spread_y, 0.0),
                Vec3::new(spread_x, -spread_y, 0.0),
            ]
        };



        for s_i in 0..spawns_here {
            let off = offsets[s_i % offsets.len()];
            let team_num = match cfg.game_mode {
                WizardGameMode::CaptureTheFlag => ctf_team.unwrap_or(0),
                WizardGameMode::TeamDeathMatch => r.team_owner.unwrap_or((r_idx % 2) as u8),
                WizardGameMode::Assault => {
                    // Team 1 = Attackers (start room), Team 0 = Defenders (behind the first gate)
                    if assault_role == Some(1) { 1 } else { 0 }
                }
                _ => 0,
            };
            let mut ps = WizardActorSpec {
                name: format!("PlayerStart{ps_idx}"),
                class_name: "PlayerStart".to_string(),
                location: Vec3::new(r.center.x + off.x, r.center.y + off.y, fz),
                rotation: (0, ((r_idx as i32) * 16384) & 65535, 0),
                group: "Spawns".to_string(),
                ..Default::default()
            };
            ps.byte_props.push(("TeamNumber".to_string(), team_num));
            actors.push(ps);
            ps_idx += 1;
        }
    }

    // Ensure at least 4 PlayerStarts even in small maps
    if ps_idx < 4 {
        if cfg.game_mode == WizardGameMode::Domination {
            let cp_locs: Vec<Vec3> = domination_plan
                .as_ref()
                .map(|dp| dp.points.iter().map(|p| p.location).collect())
                .unwrap_or_default();

            let mut spawn_candidates: Vec<Vec3> = Vec::new();

            // 1. Candidate positions from non-CP main rooms
            for r in &main_rooms {
                let is_cp = domination_plan.as_ref().map_or(false, |dp| {
                    dp.points.iter().any(|p| nodes.get(p.room_index).map_or(false, |n| n.name == r.name))
                });
                if !is_cp {
                    let fz = r.floor_z() + 48.0;
                    let (cand_x, cand_y) = if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
                        let pit_w = (r.size.x * 0.32).clamp(192.0, 352.0);
                        let pit_l = (r.size.y * 0.32).clamp(192.0, 352.0);
                        (
                            snap_grid(pit_w * 0.5 + 80.0, 16.0).min(snap_grid(r.size.x * 0.38, 16.0)),
                            snap_grid(pit_l * 0.5 + 80.0, 16.0).min(snap_grid(r.size.y * 0.38, 16.0)),
                        )
                    } else {
                        (192.0, 192.0)
                    };
                    spawn_candidates.push(Vec3::new(r.center.x - cand_x, r.center.y, fz));
                    spawn_candidates.push(Vec3::new(r.center.x + cand_x, r.center.y, fz));
                    spawn_candidates.push(Vec3::new(r.center.x, r.center.y - cand_y, fz));
                    spawn_candidates.push(Vec3::new(r.center.x, r.center.y + cand_y, fz));
                }
            }


            // 2. Candidate positions from corridors and connecting galleries
            for c in nodes.iter().filter(|n| n.is_corridor) {
                let fz = c.floor_z() + 48.0;
                spawn_candidates.push(Vec3::new(c.center.x, c.center.y, fz));
                if c.size.y > c.size.x + 256.0 {
                    spawn_candidates.push(Vec3::new(c.center.x, c.center.y - 192.0, fz));
                    spawn_candidates.push(Vec3::new(c.center.x, c.center.y + 192.0, fz));
                } else if c.size.x > c.size.y + 256.0 {
                    spawn_candidates.push(Vec3::new(c.center.x - 192.0, c.center.y, fz));
                    spawn_candidates.push(Vec3::new(c.center.x + 192.0, c.center.y, fz));
                }
            }

            // 3. Filter candidates maintaining >= 480 UU standoff distance from all ControlPoints
            let valid_candidates: Vec<Vec3> = spawn_candidates
                .into_iter()
                .filter(|cand| cp_locs.iter().all(|cp| cand.distance(*cp) >= 480.0))
                .collect();

            if !valid_candidates.is_empty() {
                let target_count = if ps_idx == 0 { 6.min(valid_candidates.len()).max(4) } else { 4 };
                while ps_idx < target_count {
                    let loc = valid_candidates[(ps_idx as usize) % valid_candidates.len()];
                    let cycle = (ps_idx as usize) / valid_candidates.len();
                    let cycle_off = if cycle > 0 {
                        let sign = if cycle % 2 == 0 { 1.0 } else { -1.0 };
                        Vec3::new(32.0 * cycle as f32 * sign, 0.0, 0.0)
                    } else {
                        Vec3::ZERO
                    };
                    let mut ps = WizardActorSpec {
                        name: format!("PlayerStart{ps_idx}"),
                        class_name: "PlayerStart".to_string(),
                        location: loc + cycle_off,
                        group: "Spawns".to_string(),
                        ..Default::default()
                    };
                    ps.byte_props.push(("TeamNumber".to_string(), 0));
                    actors.push(ps);
                    ps_idx += 1;
                }
            } else if let Some(hub) = main_rooms.first() {
                // Extreme fallback: perimeter corners of first room
                let fz = hub.floor_z() + 48.0;
                let dist = (hub.size.x.min(hub.size.y) * 0.45).max(500.0);
                for extra in ps_idx..4 {
                    let angle = (extra as f32) * std::f32::consts::FRAC_PI_2 + 0.35;
                    let mut ps = WizardActorSpec {
                        name: format!("PlayerStart{extra}"),
                        class_name: "PlayerStart".to_string(),
                        location: Vec3::new(
                            hub.center.x + angle.cos() * dist,
                            hub.center.y + angle.sin() * dist,
                            fz,
                        ),
                        group: "Spawns".to_string(),
                        ..Default::default()
                    };
                    ps.byte_props.push(("TeamNumber".to_string(), 0));
                    actors.push(ps);
                }
            }
        } else {
            let spawn_room = main_rooms.first();
            if let Some(hub) = spawn_room {
                let fz = hub.floor_z() + 48.0;
                for extra in ps_idx..4 {
                    let angle = (extra as f32) * std::f32::consts::FRAC_PI_2;
                    let mut ps = WizardActorSpec {
                        name: format!("PlayerStart{extra}"),
                        class_name: "PlayerStart".to_string(),
                        location: Vec3::new(
                            hub.center.x + angle.cos() * 192.0,
                            hub.center.y + angle.sin() * 192.0,
                            fz,
                        ),
                        group: "Spawns".to_string(),
                        ..Default::default()
                    };
                    let team_num = (extra % 2) as u8;
                    ps.byte_props.push(("TeamNumber".to_string(), team_num));
                    actors.push(ps);
                }
            }
        }
    }

    // Mode-Specific Objectives
    match cfg.game_mode {
        WizardGameMode::CaptureTheFlag => {
            if let Some(plan) = ctf_plan.as_ref() {
                build_ctf_mission_content(
                    plan,
                    &nodes,
                    &pal,
                    cfg,
                    &mut actors,
                    &mut semisolid_detail_brushes,
                );
            }
        }
        WizardGameMode::Domination => {
            if let Some(plan) = domination_plan.as_ref() {
                build_domination_mission_content(
                    plan,
                    &nodes,
                    &pal,
                    cfg,
                    &mut actors,
                    &mut semisolid_detail_brushes,
                );
            }
        }
        WizardGameMode::Assault => {
            if let Some(plan) = assault_plan.as_ref() {
                build_assault_mission_content(
                    plan,
                    &nodes,
                    &pal,
                    cfg,
                    &mut actors,
                    &mut movers,
                    &mut add_structural_brushes,
                    &mut semisolid_detail_brushes,
                );
            }
        }
        _ => {}
    }

    // -----------------------------------------------------------------------
    // PHASE 5: Authentic Tournament Weapons, Ammo, Health, Armor & Powerups
    // (Symmetrical CTF Weapons, Multi-Slot Room/Corridor Solver & 100% Loadout Guarantee)
    // -----------------------------------------------------------------------
    if cfg.toggles.include_weapons_and_pickups {
        let active_weapons = cfg.active_weapons();
        let active_powerups = cfg.active_powerups();

        let fallback_weapons = [
            "ShockRifle".to_string(),
            "UT_FlakCannon".to_string(),
            "UT_Eightball".to_string(),
            "PulseGun".to_string(),
            "SniperRifle".to_string(),
            "minigun2".to_string(),
        ];
        let weapons_to_spawn: Vec<String> = if !active_weapons.is_empty() {
            active_weapons
        } else {
            fallback_weapons.to_vec()
        };

        let is_ctf_symmetrical = (cfg.game_mode == WizardGameMode::CaptureTheFlag
            && cfg.archetype.category() == WizardArchetypeCategory::SymmetricalTeam)
            || cfg.symmetry_mode != WizardSymmetryMode::None;

        let pick_tactical_weapon = |unplaced: &mut VecDeque<String>,
                                        fallback: &[String],
                                        counter: usize,
                                        is_long_or_high: bool,
                                        is_tight_choke: bool|
         -> String {
            let desired: &[&str] = if is_long_or_high {
                &["SniperRifle", "ShockRifle"]
            } else if is_tight_choke {
                &["UT_FlakCannon", "UT_BioRifle"]
            } else {
                &["UT_Eightball", "minigun2", "PulseGun"]
            };

            for target in desired {
                if let Some(pos) = unplaced.iter().position(|w| w.eq_ignore_ascii_case(target)) {
                    return unplaced.remove(pos).unwrap();
                }
            }

            if let Some(w) = unplaced.pop_front() {
                w
            } else {
                fallback[counter % fallback.len()].clone()
            }
        };

        if cfg.item_quota.enabled {
            // ===============================================================
            // EXACT ITEM QUOTA DISTRIBUTION ENGINE (MAXIMUM CONTROL)
            // ===============================================================
            let quota_entries = cfg.item_quota.to_quota_entries();

            // Collect candidate slots grouped by distinct spatial zones (rooms and corridors)
            let mut zone_slots: Vec<Vec<Vec3>> = Vec::new();
            for r in &main_rooms {
                let slots = compute_room_pickup_slots(r, cfg.toggles.include_environmental_hazards);
                if !slots.is_empty() {
                    zone_slots.push(slots);
                }
            }
            for r in nodes.iter().filter(|n| !n.is_corridor && !main_rooms.iter().any(|m| m.name == n.name)) {
                let slots = compute_room_pickup_slots(r, cfg.toggles.include_environmental_hazards);
                if !slots.is_empty() {
                    zone_slots.push(slots);
                }
            }
            for corr in nodes.iter().filter(|n| n.is_corridor) {
                let slots = compute_corridor_pickup_slots(corr);
                if !slots.is_empty() {
                    zone_slots.push(slots);
                }
            }
            if zone_slots.is_empty() {
                zone_slots.push(vec![Vec3::new(0.0, 0.0, 48.0)]);
            }

            // Interleave candidate slots across zones in round-robin fashion so every room/zone
            // gets 1 item before ANY room gets a 2nd item!
            let max_slots_per_zone = zone_slots.iter().map(|z| z.len()).max().unwrap_or(0);
            let mut available_slots: Vec<Vec3> = Vec::new();
            for s_idx in 0..max_slots_per_zone {
                for z in &zone_slots {
                    if s_idx < z.len() {
                        available_slots.push(z[s_idx]);
                    }
                }
            }

            // Track placement history per class to enforce maximum spatial dispersion for duplicate items
            let mut placed_by_class: std::collections::HashMap<&'static str, Vec<Vec3>> =
                std::collections::HashMap::new();

            for (idx, entry) in quota_entries.into_iter().enumerate() {
                let loc = if let Some(prev_locs) = placed_by_class.get(entry.class_name) {
                    // Duplicate item of same class: pick available slot that maximizes distance to existing instances
                    if !available_slots.is_empty() {
                        let best_idx = available_slots
                            .iter()
                            .enumerate()
                            .max_by(|(_, &a), (_, &b)| {
                                let min_a = prev_locs.iter().map(|p| p.distance(a)).fold(f32::INFINITY, f32::min);
                                let min_b = prev_locs.iter().map(|p| p.distance(b)).fold(f32::INFINITY, f32::min);
                                min_a.partial_cmp(&min_b).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        available_slots.remove(best_idx)
                    } else {
                        // Overflow wrap-around with offset
                        let r = &nodes[idx % nodes.len()];
                        let angle = (idx as f32) * 1.396;
                        let rad = (((idx * 64) % 256) as f32) + 64.0;
                        Vec3::new(
                            r.center.x + angle.cos() * rad,
                            r.center.y + angle.sin() * rad,
                            r.floor_z() + 36.0,
                        )
                    }
                } else if !available_slots.is_empty() {
                    // First instance of this class: take the next round-robin zone slot
                    available_slots.remove(0)
                } else {
                    let r = &nodes[idx % nodes.len()];
                    let angle = (idx as f32) * 1.396;
                    let rad = (((idx * 64) % 256) as f32) + 64.0;
                    Vec3::new(
                        r.center.x + angle.cos() * rad,
                        r.center.y + angle.sin() * rad,
                        r.floor_z() + 36.0,
                    )
                };

                placed_by_class.entry(entry.class_name).or_default().push(loc);

                if entry.is_weapon {
                    spawn_weapon_with_pedestal(
                        entry.class_name,
                        loc,
                        &format!("Quota_{}_{}", entry.class_name, idx),
                        "Pickups",
                        cfg,
                        &pal,
                        &mut actors,
                        &mut semisolid_detail_brushes,
                    );
                } else {
                    actors.push(WizardActorSpec {
                        name: format!("Quota_{}_{}", entry.class_name, idx),
                        class_name: entry.class_name.to_string(),
                        location: loc,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }
        } else if is_ctf_symmetrical {
            // ===============================================================
            // CTF & SYMMETRICAL MAPS: EXACT WEAPON & POWERUP PARITY
            // ===============================================================
            let sym_mode: CtfSymmetryMode = cfg.symmetry_mode.into();
            let sym_center = if let Some(plan) = &ctf_plan {
                (plan.red_flag_loc + plan.blue_flag_loc) * 0.5
            } else {
                Vec3::ZERO
            };

            let mut red_rooms: Vec<&RoomNode> = Vec::new();
            let mut blue_rooms: Vec<&RoomNode> = Vec::new();
            let mut neutral_rooms: Vec<&RoomNode> = Vec::new();
            let mut red_corridors: Vec<&RoomNode> = Vec::new();
            let mut blue_corridors: Vec<&RoomNode> = Vec::new();
            let mut neutral_corridors: Vec<&RoomNode> = Vec::new();

            for n in &nodes {
                let name_lower = n.name.to_ascii_lowercase();
                let is_red = n.team_owner == Some(0) || name_lower.contains("red");
                let is_blue = n.team_owner == Some(1) || name_lower.contains("blue");

                if is_red {
                    if n.is_corridor { red_corridors.push(n); } else { red_rooms.push(n); }
                } else if is_blue {
                    if n.is_corridor { blue_corridors.push(n); } else { blue_rooms.push(n); }
                } else {
                    if let Some(plan) = &ctf_plan {
                        let dr = n.center.distance(plan.red_flag_loc);
                        let db = n.center.distance(plan.blue_flag_loc);
                        if (dr - db).abs() > 250.0 {
                            if dr < db {
                                if n.is_corridor { red_corridors.push(n); } else { red_rooms.push(n); }
                            } else {
                                if n.is_corridor { blue_corridors.push(n); } else { blue_rooms.push(n); }
                            }
                            continue;
                        }
                    }
                    if n.is_corridor { neutral_corridors.push(n); } else { neutral_rooms.push(n); }
                }
            }

            // Ensure Red FlagBase room is at index 0 and Blue FlagBase room is at index 0
            if let Some(plan) = &ctf_plan {
                if let Some(pos) = red_rooms.iter().position(|r| r.center.distance(plan.red_flag_loc) < 220.0) {
                    red_rooms.swap(0, pos);
                }
                if let Some(pos) = blue_rooms.iter().position(|r| r.center.distance(plan.blue_flag_loc) < 220.0) {
                    blue_rooms.swap(0, pos);
                }
            }

            // Build queue of weapons to ensure 100% placement coverage
            let mut unplaced_weapons: VecDeque<String> = weapons_to_spawn.clone().into();
            if cfg.item_density == WizardItemDensity::Abundant {
                unplaced_weapons.extend(weapons_to_spawn.clone()); // DOUBLED!
            }

            // A) Contested Midfield Heavy Weapon: Redeemer (WarHeadLauncher)
            let is_signature_redeemer_archetype = cfg.archetype == WizardArchetype::FacingWorldsOrbitalSpire
                || cfg.archetype == WizardArchetype::CanyonCrestCitadels;
            let has_redeemer = is_signature_redeemer_archetype || unplaced_weapons.iter().any(|w| w.eq_ignore_ascii_case("WarHeadLauncher"));
            if has_redeemer {
                unplaced_weapons.retain(|w| !w.eq_ignore_ascii_case("WarHeadLauncher"));
                let redeemer_loc = if let Some(mid) = neutral_rooms.first() {
                    Vec3::new(mid.center.x, mid.center.y, mid.floor_z() + 36.0)
                } else if let Some(bridge) = neutral_corridors.first() {
                    Vec3::new(sym_center.x, sym_center.y, bridge.floor_z() + 36.0)
                } else {
                    Vec3::new(sym_center.x, sym_center.y, 48.0)
                };

                spawn_weapon_with_pedestal(
                    "WarHeadLauncher",
                    redeemer_loc,
                    "Midfield_Redeemer",
                    "Pickups",
                    cfg,
                    &pal,
                    &mut actors,
                    &mut semisolid_detail_brushes,
                );
            }

            // Target symmetrical weapons to spawn in Red territory (and mirrored in Blue)
            let target_red_weapons = match cfg.item_density {
                WizardItemDensity::Sparse => weapons_to_spawn.len().min(red_rooms.len().max(2)),
                WizardItemDensity::Standard => weapons_to_spawn.len(),
                WizardItemDensity::Abundant => weapons_to_spawn.len() * 2,
            };

            // Collect all available Red territory pickup slots with spatial attributes
            let mut red_slots: Vec<(Vec3, String, bool, bool)> = Vec::new();
            for (r_idx, red_room) in red_rooms.iter().enumerate() {
                let slots = compute_room_pickup_slots(red_room, cfg.toggles.include_environmental_hazards);
                let slots_to_use = match cfg.item_density {
                    WizardItemDensity::Sparse => 2.min(slots.len()),
                    WizardItemDensity::Standard => 4.min(slots.len()),
                    WizardItemDensity::Abundant => slots.len(),
                };
                for (s_idx, s) in slots.into_iter().take(slots_to_use).enumerate() {
                    red_slots.push((s, format!("{r_idx}_{s_idx}"), false, false));
                }
            }
            for (c_idx, corr) in red_corridors.iter().enumerate() {
                let is_long = corr.size.x.max(corr.size.y) >= 1024.0;
                let is_choke = corr.size.x.min(corr.size.y) <= 384.0;
                for (s_idx, s) in compute_corridor_pickup_slots(corr).into_iter().enumerate() {
                    red_slots.push((s, format!("Corr_{c_idx}_{s_idx}"), is_long, is_choke));
                }
            }
            for (c_idx, corr) in neutral_corridors.iter().enumerate() {
                let is_long = corr.size.x.max(corr.size.y) >= 1024.0;
                let is_choke = corr.size.x.min(corr.size.y) <= 384.0;
                for (s_idx, s) in compute_corridor_pickup_slots(corr).into_iter().enumerate() {
                    red_slots.push((s, format!("MidCorr_{c_idx}_{s_idx}"), is_long, is_choke));
                }
            }

            let mut red_slot_iter = red_slots.into_iter();
            let mut spawned_red_count = 0;

            while spawned_red_count < target_red_weapons || !unplaced_weapons.is_empty() {
                let (loc_red, suffix, is_long, is_choke) = if let Some(slot) = red_slot_iter.next() {
                    slot
                } else {
                    let fallback_room = red_rooms.first().copied().unwrap_or(&nodes[0]);
                    let offset_x = ((spawned_red_count as f32 * 72.0) % 288.0) - 144.0;
                    let offset_y = ((spawned_red_count as f32 * 56.0) % 288.0) - 144.0;
                    (
                        Vec3::new(
                            fallback_room.center.x + offset_x,
                            fallback_room.center.y + offset_y,
                            fallback_room.floor_z() + 36.0,
                        ),
                        format!("Extra_{spawned_red_count}"),
                        false,
                        false,
                    )
                };

                let weap_cls = pick_tactical_weapon(
                    &mut unplaced_weapons,
                    &weapons_to_spawn,
                    spawned_red_count,
                    is_long,
                    is_choke,
                );

                spawn_symmetrical_weapon_cluster(
                    &weap_cls,
                    loc_red,
                    &suffix,
                    "Pickups",
                    cfg,
                    sym_mode,
                    sym_center,
                    &pal,
                    &mut actors,
                    &mut semisolid_detail_brushes,
                );
                spawned_red_count += 1;
            }

            // D) Symmetrical Health Restoratives
            let has_medbox = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("MedBox"));
            let has_vials = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("HealthVial"));
            for (r_idx, red_room) in red_rooms.iter().enumerate() {
                let pit_pad = if red_room.has_hazard_pit && cfg.toggles.include_environmental_hazards {
                    let pit_w = (red_room.size.x * 0.32).clamp(192.0, 352.0);
                    (pit_w * 0.5 + 48.0).max((red_room.size.x * 0.22).min(224.0))
                } else {
                    (red_room.size.x * 0.22).min(192.0)
                };
                let hx = red_room.center.x - pit_pad;
                let hy = red_room.center.y + pit_pad;
                let fz = red_room.floor_z() + 36.0;

                spawn_restoratives_symmetrical(
                    Vec3::new(hx, hy, fz),
                    &format!("{r_idx}"),
                    has_medbox,
                    has_vials,
                    cfg.item_density,
                    sym_mode,
                    sym_center,
                    &mut actors,
                );
            }

            // E) Symmetrical Armor (Armor2)
            let has_armor2 = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("Armor2"));
            if has_armor2 {
                if let Some(red_base) = red_rooms.first() {
                    let armor_loc_red = Vec3::new(
                        red_base.center.x,
                        red_base.center.y - (red_base.size.y * 0.28).min(224.0),
                        red_base.floor_z() + 36.0,
                    );
                    actors.push(WizardActorSpec {
                        name: "Armor2_Red".to_string(),
                        class_name: "Armor2".to_string(),
                        location: armor_loc_red,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });

                    let armor_loc_blue = transform_point_symmetry(armor_loc_red, sym_mode, sym_center);
                    actors.push(WizardActorSpec {
                        name: "Armor2_Blue".to_string(),
                        class_name: "Armor2".to_string(),
                        location: armor_loc_blue,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            // F) Symmetrical Jump Boots
            let has_jumpboots = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_JumpBoots"));
            if has_jumpboots {
                if let Some(red_base) = red_rooms.first() {
                    let boots_red = Vec3::new(
                        red_base.center.x,
                        red_base.center.y + (red_base.size.y * 0.28).min(224.0),
                        red_base.floor_z() + 48.0,
                    );
                    actors.push(WizardActorSpec {
                        name: "UT_JumpBoots_Red".to_string(),
                        class_name: "UT_JumpBoots".to_string(),
                        location: boots_red,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });

                    let boots_blue = transform_point_symmetry(boots_red, sym_mode, sym_center);
                    actors.push(WizardActorSpec {
                        name: "UT_JumpBoots_Blue".to_string(),
                        class_name: "UT_JumpBoots".to_string(),
                        location: boots_blue,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            // G) Symmetrical Invisibility
            let has_invis = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_Invisibility"));
            if has_invis {
                if let Some(red_base) = red_rooms.first() {
                    let invis_red = Vec3::new(
                        red_base.center.x - (red_base.size.x * 0.28).min(224.0),
                        red_base.center.y,
                        red_base.floor_z() + 48.0,
                    );
                    actors.push(WizardActorSpec {
                        name: "UT_Invisibility_Red".to_string(),
                        class_name: "UT_Invisibility".to_string(),
                        location: invis_red,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });

                    let invis_blue = transform_point_symmetry(invis_red, sym_mode, sym_center);
                    actors.push(WizardActorSpec {
                        name: "UT_Invisibility_Blue".to_string(),
                        class_name: "UT_Invisibility".to_string(),
                        location: invis_blue,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            // H) Contested Super Health Pack (Keg of Health)
            let has_keg = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("HealthPack"));
            if has_keg {
                if let Some(mid) = neutral_rooms.first() {
                    actors.push(WizardActorSpec {
                        name: "HealthPack_Midfield".to_string(),
                        class_name: "HealthPack".to_string(),
                        location: Vec3::new(mid.center.x, mid.center.y, mid.floor_z() + 48.0),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                } else if let Some(red_base) = red_rooms.first() {
                    let keg_red = Vec3::new(
                        red_base.center.x + (red_base.size.x * 0.28).min(224.0),
                        red_base.center.y,
                        red_base.floor_z() + 48.0,
                    );
                    actors.push(WizardActorSpec {
                        name: "HealthPack_Red".to_string(),
                        class_name: "HealthPack".to_string(),
                        location: keg_red,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });

                    let keg_blue = transform_point_symmetry(keg_red, sym_mode, sym_center);
                    actors.push(WizardActorSpec {
                        name: "HealthPack_Blue".to_string(),
                        class_name: "HealthPack".to_string(),
                        location: keg_blue,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            // I) Contested Mega-Powerup Secondary Placement (if BOTH ShieldBelt and UDamage selected)
            let has_shieldbelt = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_ShieldBelt"));
            let has_udamage = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UDamage"));
            if has_shieldbelt && has_udamage {
                let center_is_udamage = (cfg.seed % 2) == 0;
                let sec_powerup_cls = if center_is_udamage { "UT_ShieldBelt" } else { "UDamage" };
                if let Some(mid) = neutral_rooms.first() {
                    let dy = (mid.size.y * 0.25).clamp(160.0, 260.0);
                    actors.push(WizardActorSpec {
                        name: format!("{sec_powerup_cls}_MidfieldSec"),
                        class_name: sec_powerup_cls.to_string(),
                        location: Vec3::new(mid.center.x, mid.center.y - dy, mid.floor_z() + 48.0),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }
        } else {
            // ===============================================================
            // NON-CTF MODES (DeathMatch, Duel1v1, Domination, Assault)
            // ===============================================================
            let mut unplaced_weapons: VecDeque<String> = weapons_to_spawn.clone().into();
            if cfg.item_density == WizardItemDensity::Abundant {
                unplaced_weapons.extend(weapons_to_spawn.clone()); // DOUBLE!
            }

            let target_weapon_count = match cfg.item_density {
                WizardItemDensity::Sparse => weapons_to_spawn.len().min(main_rooms.len().max(2)),
                WizardItemDensity::Standard => weapons_to_spawn.len(),
                WizardItemDensity::Abundant => weapons_to_spawn.len() * 2,
            };

            let mut room_slots: Vec<(Vec3, usize, bool, bool)> = Vec::new();
            for (r_idx, r) in main_rooms.iter().enumerate() {
                let slots = compute_room_pickup_slots(r, cfg.toggles.include_environmental_hazards);
                let count = match cfg.item_density {
                    WizardItemDensity::Sparse => 2.min(slots.len()),
                    WizardItemDensity::Standard => 4.min(slots.len()),
                    WizardItemDensity::Abundant => slots.len(),
                };
                for s in slots.into_iter().take(count) {
                    room_slots.push((s, r_idx, false, false));
                }
            }
            for corr in &nodes {
                if corr.is_corridor {
                    let is_long = corr.size.x.max(corr.size.y) >= 1024.0;
                    let is_choke = corr.size.x.min(corr.size.y) <= 384.0;
                    for s in compute_corridor_pickup_slots(corr) {
                        room_slots.push((s, 999, is_long, is_choke));
                    }
                }
            }

            let mut slot_iter = room_slots.into_iter();
            let mut spawned_weapon_count = 0;

            while spawned_weapon_count < target_weapon_count || !unplaced_weapons.is_empty() {
                let (loc, r_idx, is_long, is_choke) = if let Some(slot) = slot_iter.next() {
                    slot
                } else {
                    let r = &main_rooms[spawned_weapon_count % main_rooms.len()];
                    (
                        Vec3::new(
                            r.center.x + ((spawned_weapon_count as f32 * 64.0) % 256.0) - 128.0,
                            r.center.y + ((spawned_weapon_count as f32 * 48.0) % 256.0) - 128.0,
                            r.floor_z() + 36.0,
                        ),
                        spawned_weapon_count % main_rooms.len(),
                        false,
                        false,
                    )
                };

                let weap_cls = pick_tactical_weapon(
                    &mut unplaced_weapons,
                    &weapons_to_spawn,
                    spawned_weapon_count,
                    is_long,
                    is_choke,
                );

                spawn_weapon_with_pedestal(
                    &weap_cls,
                    loc,
                    &format!("{r_idx}_{spawned_weapon_count}"),
                    "Pickups",
                    cfg,
                    &pal,
                    &mut actors,
                    &mut semisolid_detail_brushes,
                );
                spawned_weapon_count += 1;
            }

            // Health restoratives & powerup checks
            let has_medbox = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("MedBox"));
            let has_vials = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("HealthVial"));
            let has_shieldbelt = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_ShieldBelt"));
            let has_udamage = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UDamage"));
            for (r_idx, r) in main_rooms.iter().enumerate() {
                let pit_pad = if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
                    let pit_w = (r.size.x * 0.32).clamp(192.0, 352.0);
                    (pit_w * 0.5 + 48.0).max((r.size.x * 0.22).min(224.0))
                } else {
                    (r.size.x * 0.22).min(192.0)
                };
                let hx = r.center.x - pit_pad;
                let hy = r.center.y;
                let fz = r.floor_z() + 36.0;

                if has_medbox {
                    actors.push(WizardActorSpec {
                        name: format!("MedBox_{r_idx}"),
                        class_name: "MedBox".to_string(),
                        location: Vec3::new(hx, hy, fz),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                    if cfg.item_density == WizardItemDensity::Abundant {
                        actors.push(WizardActorSpec {
                            name: format!("MedBox_{r_idx}_extra"),
                            class_name: "MedBox".to_string(),
                            location: Vec3::new(hx, hy + 48.0, fz),
                            group: "Pickups".to_string(),
                            ..Default::default()
                        });
                    }
                }
                if has_vials {
                    let vial_spacing = 32.0_f32;
                    let num_vials = 5;
                    for vi in 0..num_vials {
                        let t = (vi as f32 - 2.0) / 2.0;
                        let lat_offset = (1.0 - t * t) * 14.0;
                        let vx = hx - 32.0 + lat_offset;
                        let vy = hy + (vi as f32 - 2.0) * vial_spacing;
                        actors.push(WizardActorSpec {
                            name: format!("HealthVial_{r_idx}_{vi}"),
                            class_name: "HealthVial".to_string(),
                            location: Vec3::new(vx, vy, fz),
                            group: "Pickups".to_string(),
                            ..Default::default()
                        });
                    }
                    if cfg.item_density == WizardItemDensity::Abundant {
                        for vi in 0..num_vials {
                            let t = (vi as f32 - 2.0) / 2.0;
                            let lat_offset = (1.0 - t * t) * 14.0;
                            let vx = hx - 64.0 + lat_offset;
                            let vy = hy + (vi as f32 - 2.0) * vial_spacing;
                            actors.push(WizardActorSpec {
                                name: format!("HealthVial_{r_idx}_{vi}_extra"),
                                class_name: "HealthVial".to_string(),
                                location: Vec3::new(vx, vy, fz),
                                group: "Pickups".to_string(),
                                ..Default::default()
                            });
                        }
                    }
                }

                // Armor
                let available_armor: Vec<&str> = active_powerups
                    .iter()
                    .filter(|p| matches!(p.to_ascii_lowercase().as_str(), "thighpads" | "armor2"))
                    .map(|s| s.as_str())
                    .collect();
                if !available_armor.is_empty() {
                    let armor_cls = available_armor[r_idx % available_armor.len()];
                    let armor_loc = if r_idx == 0 && (has_udamage || has_shieldbelt) {
                        Vec3::new(r.center.x + (r.size.x * 0.28).min(224.0), r.center.y, fz)
                    } else {
                        Vec3::new(r.center.x, r.center.y - (r.size.y * 0.28).min(224.0), fz)
                    };
                    actors.push(WizardActorSpec {
                        name: format!("{armor_cls}_{r_idx}"),
                        class_name: armor_cls.to_string(),
                        location: armor_loc,
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            // High-tier powerups
            let has_shieldbelt = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_ShieldBelt"));
            let has_udamage = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UDamage"));
            let has_keg = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("HealthPack"));
            let has_jumpboots = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_JumpBoots"));
            let has_invis = active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_Invisibility"));

            if let Some(hub) = main_rooms.first() {
                let p_z = hub.floor_z() + 44.0;
                let dy = (hub.size.y * 0.32).clamp(240.0, 272.0);
                let p_y_pos = hub.center.y + dy;
                let p_y_neg = hub.center.y - dy;

                let skip_hub_shieldbelt = cfg.game_mode == WizardGameMode::Domination
                    && cfg.domination_contested_center_powerup;
                if has_shieldbelt && !skip_hub_shieldbelt {
                    if !cfg.toggles.include_ramps_and_bridges {
                        let mut pad = build_cube(
                            "ShieldBeltPad".to_string(),
                            CsgOperation::Add,
                            Vec3::new(80.0, 80.0, 12.0),
                        );
                        pad.location = Vec3::new(hub.center.x, p_y_pos, hub.floor_z() + 6.0);
                        pad.group = Some("Pickups".to_string());
                        for p in &mut pad.polys {
                            p.texture = pal.trim_texture.clone();
                            p.flags |= PF_SEMISOLID;
                        }
                        pad.recompute_uvs();
                        semisolid_detail_brushes.push(pad);
                    }

                    let belt_z = if cfg.toggles.include_ramps_and_bridges {
                        p_z + 32.0
                    } else {
                        p_z
                    };
                    actors.push(WizardActorSpec {
                        name: "UT_ShieldBelt_0".to_string(),
                        class_name: "UT_ShieldBelt".to_string(),
                        location: Vec3::new(hub.center.x, p_y_pos, belt_z),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }

                if has_udamage && cfg.game_mode != WizardGameMode::Duel1v1 {
                    let mut pad = build_cube(
                        "UDamagePad".to_string(),
                        CsgOperation::Add,
                        Vec3::new(80.0, 80.0, 12.0),
                    );
                    pad.location = Vec3::new(hub.center.x, p_y_neg, hub.floor_z() + 6.0);
                    pad.group = Some("Pickups".to_string());
                    for p in &mut pad.polys {
                        p.texture = pal.trim_texture.clone();
                        p.flags |= PF_SEMISOLID;
                    }
                    pad.recompute_uvs();
                    semisolid_detail_brushes.push(pad);

                    actors.push(WizardActorSpec {
                        name: "UDamage_0".to_string(),
                        class_name: "UDamage".to_string(),
                        location: Vec3::new(hub.center.x, p_y_neg, p_z),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }

            let mut powerup_targets: Vec<Vec3> = Vec::new();
            for r in main_rooms.iter().skip(1) {
                powerup_targets.push(Vec3::new(r.center.x, r.center.y, r.floor_z() + 48.0));
            }
            for c in &nodes {
                if c.is_corridor {
                    powerup_targets.push(Vec3::new(c.center.x, c.center.y, c.floor_z() + 48.0));
                }
            }

            let mut p_idx = 0;
            if has_keg {
                let loc = powerup_targets.get(p_idx).copied().unwrap_or(Vec3::new(0.0, 0.0, 48.0));
                p_idx += 1;
                actors.push(WizardActorSpec {
                    name: "HealthPack_0".to_string(),
                    class_name: "HealthPack".to_string(),
                    location: loc,
                    group: "Pickups".to_string(),
                    ..Default::default()
                });
            }
            if has_jumpboots {
                let loc = powerup_targets.get(p_idx).copied().unwrap_or(Vec3::new(64.0, 64.0, 48.0));
                p_idx += 1;
                actors.push(WizardActorSpec {
                    name: "UT_JumpBoots_0".to_string(),
                    class_name: "UT_JumpBoots".to_string(),
                    location: loc,
                    group: "Pickups".to_string(),
                    ..Default::default()
                });
            }
            if has_invis {
                let loc = powerup_targets.get(p_idx).copied().unwrap_or(Vec3::new(-64.0, -64.0, 48.0));
                actors.push(WizardActorSpec {
                    name: "UT_Invisibility_0".to_string(),
                    class_name: "UT_Invisibility".to_string(),
                    location: loc,
                    group: "Pickups".to_string(),
                    ..Default::default()
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 5.5: Competitive Playability, Combat Readiness & Fun Enforcement Pass
    // Guarantees:
    // 1. Combat Readiness: Every PlayerStart has a weapon within <= 750 UU (< 1.9s sprint).
    // 2. Telefrag Prevention: Every pair of PlayerStarts has >= 280 UU separation.
    // 3. Proactive Orientation: PlayerStart Yaw points toward room center (not a wall).
    // 4. Zero Orphan Weapons: Every primary weapon is paired with matching ammo within 300 UU.
    // 5. Flow & Momentum: Long transit corridors receive smooth HealthVial runway sequences.
    // -----------------------------------------------------------------------
    {
        let is_weapon_cls = |cls: &str| -> bool {
            let lower = cls.to_ascii_lowercase();
            lower.contains("shock")
                || lower.contains("flak")
                || lower.contains("eightball")
                || lower.contains("minigun")
                || lower.contains("pulse")
                || lower.contains("ripper")
                || lower.contains("biorifle")
                || lower.contains("sniper")
                || lower.contains("enforcer")
        };

        // 1. Telefrag Prevention: Enforce >= 280 UU distance between any pair of PlayerStarts
        let player_start_indices: Vec<usize> = actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.class_name.eq_ignore_ascii_case("PlayerStart"))
            .map(|(i, _)| i)
            .collect();

        for i in 0..player_start_indices.len() {
            for j in (i + 1)..player_start_indices.len() {
                let idx_a = player_start_indices[i];
                let idx_b = player_start_indices[j];
                let loc_a = actors[idx_a].location;
                let loc_b = actors[idx_b].location;
                // Vertical tier isolation: players on different floors separated by > 80 UU
                // in Z never telefrag each other and must never repel across elevations.
                if (loc_a.z - loc_b.z).abs() > 80.0 {
                    continue;
                }
                let diff_2d = Vec3::new(loc_b.x - loc_a.x, loc_b.y - loc_a.y, 0.0);
                let dist_2d = diff_2d.length();
                if dist_2d < 280.0 {
                    let dir_2d = if dist_2d > 1e-3 {
                        diff_2d / dist_2d
                    } else {
                        Vec3::new(1.0, 0.0, 0.0)
                    };
                    let push = 300.0 - dist_2d;
                    let mut new_loc = actors[idx_b].location + dir_2d * push;
                    if let Some(r) = nodes.iter().find(|n| {
                        !n.is_corridor
                            && (new_loc.x - n.center.x).abs() <= n.size.x * 0.5 + 32.0
                            && (new_loc.y - n.center.y).abs() <= n.size.y * 0.5 + 32.0
                            && new_loc.z >= n.floor_z() - 32.0
                            && new_loc.z <= n.ceiling_z() + 32.0
                    }) {
                        new_loc.x = new_loc.x.clamp(r.min_x() + 48.0, r.max_x() - 48.0);
                        new_loc.y = new_loc.y.clamp(r.min_y() + 48.0, r.max_y() - 48.0);
                        new_loc.z = r.floor_z() + 48.0;
                    }
                    actors[idx_b].location = new_loc;
                }
            }
        }

        // 2. Proactive Orientation: Point PlayerStarts into the open room instead of facing a wall
        for &idx in &player_start_indices {
            let loc = actors[idx].location;
            if let Some(r) = nodes.iter().find(|n| {
                !n.is_corridor
                    && (loc.x - n.center.x).abs() <= n.size.x * 0.5 + 32.0
                    && (loc.y - n.center.y).abs() <= n.size.y * 0.5 + 32.0
                    && loc.z >= n.floor_z() - 32.0
                    && loc.z <= n.ceiling_z() + 32.0
            }) {
                let dx = r.center.x - loc.x;
                let dy = r.center.y - loc.y;
                if dx * dx + dy * dy > 64.0 {
                    let yaw_uu = ((dy.atan2(dx).to_degrees() / 360.0) * 65536.0).rem_euclid(65536.0) as i32;
                    actors[idx].rotation = (0, yaw_uu, 0);
                }
            }
        }

        // 3. Combat Readiness: Ensure every PlayerStart has a weapon within <= 750 UU
        let current_weapon_locs: Vec<Vec3> = actors
            .iter()
            .filter(|a| a.group == "Pickups" && is_weapon_cls(&a.class_name))
            .map(|a| a.location)
            .collect();

        let mut aux_weapons: Vec<(Vec3, String, String)> = Vec::new();
        for (ps_num, &idx) in player_start_indices.iter().enumerate() {
            let ps_loc = actors[idx].location;
            let nearest_wep_dist = current_weapon_locs
                .iter()
                .map(|w_loc| ps_loc.distance(*w_loc))
                .fold(f32::INFINITY, f32::min);

            if nearest_wep_dist > 750.0 {
                if let Some(r) = nodes.iter().find(|n| {
                    !n.is_corridor
                        && (ps_loc.x - n.center.x).abs() <= n.size.x * 0.5 + 32.0
                        && (ps_loc.y - n.center.y).abs() <= n.size.y * 0.5 + 32.0
                        && ps_loc.z >= n.floor_z() - 32.0
                        && ps_loc.z <= n.ceiling_z() + 32.0
                }) {
                    let to_center = Vec3::new(r.center.x - ps_loc.x, r.center.y - ps_loc.y, 0.0);
                    let dir = if to_center.length_squared() > 1e-3 {
                        to_center.normalize()
                    } else {
                        Vec3::new(1.0, 0.0, 0.0)
                    };
                    let step = (r.size.x.min(r.size.y) * 0.25).clamp(96.0, 240.0);
                    let aux_loc = Vec3::new(
                        snap_grid(ps_loc.x + dir.x * step, 16.0),
                        snap_grid(ps_loc.y + dir.y * step, 16.0),
                        r.floor_z() + 36.0,
                    );
                    let aux_weap = if r.size.x.max(r.size.y) >= 768.0 {
                        "Botpack.PulseGun".to_string()
                    } else {
                        "Botpack.UT_BioRifle".to_string()
                    };
                    aux_weapons.push((aux_loc, aux_weap, format!("AuxWeap_{ps_num}")));
                }
            }
        }

        for (loc, weap_cls, tag) in aux_weapons {
            spawn_weapon_with_pedestal(
                &weap_cls,
                loc,
                &tag,
                "Pickups",
                cfg,
                &pal,
                &mut actors,
                &mut semisolid_detail_brushes,
            );
        }

        // 4. Zero Orphan Weapons: Ensure every primary weapon has matching ammo within 300 UU
        let existing_weapons: Vec<(String, Vec3)> = actors
            .iter()
            .filter(|a| a.group == "Pickups" && is_weapon_cls(&a.class_name))
            .map(|a| (a.class_name.clone(), a.location))
            .collect();

        for (w_i, (weap_cls, w_loc)) in existing_weapons.into_iter().enumerate() {
            let short = weap_cls.rsplit('.').next().unwrap_or(&weap_cls);
            let desired_ammo = cfg.ammo_count_for_weapon(short);
            if desired_ammo == 0 {
                continue;
            }
            if let Some(ammo_cls) = ammo_for_weapon(short) {
                let existing_ammo_count = actors.iter().filter(|a| {
                    a.group == "Pickups"
                        && a.class_name.contains(ammo_cls)
                        && a.location.distance(w_loc) <= 300.0
                }).count() as u32;

                for a_i in existing_ammo_count..desired_ammo {
                    let off = get_ammo_box_offset(a_i as usize);
                    actors.push(WizardActorSpec {
                        name: format!("{ammo_cls}_Paired_{w_i}_{a_i}"),
                        class_name: ammo_cls.to_string(),
                        location: Vec3::new(w_loc.x + off.x, w_loc.y + off.y, w_loc.z),
                        group: "Pickups".to_string(),
                        ..Default::default()
                    });
                }
            }
        }

        // 5. Movement Flow: Smooth HealthVial runways in transit corridors
        for (c_idx, corr) in nodes.iter().filter(|n| n.is_corridor).enumerate() {
            let is_long_x = corr.size.x >= 768.0 && corr.size.x >= corr.size.y;
            let is_long_y = corr.size.y >= 768.0 && corr.size.y > corr.size.x;
            if is_long_x || is_long_y {
                let has_pickups_in_corr = actors.iter().any(|a| {
                    a.group == "Pickups"
                        && (a.location.x - corr.center.x).abs() <= corr.size.x * 0.4
                        && (a.location.y - corr.center.y).abs() <= corr.size.y * 0.4
                });
                if !has_pickups_in_corr {
                    let fz = corr.floor_z() + 36.0;
                    let count = 5;
                    let spacing = 36.0_f32;
                    for vi in 0..count {
                        let off = (vi as f32 - 2.0) * spacing;
                        let (vx, vy) = if is_long_x {
                            (corr.center.x + off, corr.center.y)
                        } else {
                            (corr.center.x, corr.center.y + off)
                        };
                        actors.push(WizardActorSpec {
                            name: format!("HealthVial_Runway_{c_idx}_{vi}"),
                            class_name: "HealthVial".to_string(),
                            location: Vec3::new(vx, vy, fz),
                            group: "Pickups".to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }


    // -----------------------------------------------------------------------
    // PHASE 5B: Authentic UT99 3D Decorations & Ambient Soundscapes
    // -----------------------------------------------------------------------
    if cfg.toggles.include_decorations_and_ambience {
        let theme_decos: &[&str] = match cfg.theme {
            WizardThemePreset::Deck16Industrial
            | WizardThemePreset::CoretTechFacility
            | WizardThemePreset::MorbiasCyberArena => {
                &["SteelBarrel", "Barrel", "LightBox", "Fan2"]
            }
            WizardThemePreset::PhobosOrbitalSciFi => &["LightBox", "SteelBox", "Robot", "Lamp4"],
            WizardThemePreset::CodexGothicCrypt => &["TorchFlame", "Urn", "MonkStatue", "Candle2"],
            WizardThemePreset::FacingWorldsCitadel => &["TorchFlame", "Flag1", "Urn", "Boulder"],
            WizardThemePreset::SesmarEgyptianTomb => &["TorchFlame", "Urn", "Vase", "Plant1"],
            WizardThemePreset::MineQuarry => &["Lantern", "WoodenBox", "SteelBarrel", "Pipe"],
            WizardThemePreset::WarshipNaval => &["SteelBox", "LightBox", "Fan2", "Pipe"],
            WizardThemePreset::MedievalCastle => &["TorchFlame", "Banner1", "Chest", "Chair"],
            WizardThemePreset::HeavyFoundry => &["SteelBarrel", "TarydiumBarrel", "Fan2", "Pipe"],
            WizardThemePreset::VolcanicCaldera => &["Boulder", "TorchFlame", "Urn", "Boulder1"],
            WizardThemePreset::HydroElectric => &["Pipe", "SteelBarrel", "Fan2", "LightCone"],
            WizardThemePreset::UrbanSkyscraper => &["LightBox", "Tubelight", "SteelBox", "Fan2"],
            WizardThemePreset::AncientMonastery => &["TorchFlame", "Urn", "Vase", "MonkStatue"],
        };

        let default_theme_ambience: &str = match cfg.theme {
            WizardThemePreset::Deck16Industrial => "AmbModern.Looping.hum60",
            WizardThemePreset::MorbiasCyberArena => "AmbModern.Looping.hum5",
            WizardThemePreset::PhobosOrbitalSciFi => "AmbModern.Looping.gener3",
            WizardThemePreset::CodexGothicCrypt => "AmbAncient.Looping.afire1",
            WizardThemePreset::CoretTechFacility => "AmbModern.Looping.hum1",
            WizardThemePreset::FacingWorldsCitadel => "AmbOutside.Looping.wind02",
            WizardThemePreset::SesmarEgyptianTomb => "AmbAncient.Looping.afire1",
            WizardThemePreset::MineQuarry => "AmbModern.Looping.hum1",
            WizardThemePreset::WarshipNaval => "AmbOutside.Looping.waves4",
            WizardThemePreset::MedievalCastle => "AmbAncient.Looping.Fire6",
            WizardThemePreset::HeavyFoundry => "AmbModern.Looping.hum60",
            WizardThemePreset::VolcanicCaldera => "AmbOutside.Looping.wind02",
            WizardThemePreset::HydroElectric => "AmbModern.Looping.LNpipes3",
            WizardThemePreset::UrbanSkyscraper => "AmbOutside.Looping.wind04",
            WizardThemePreset::AncientMonastery => "Ambancient.Looping.Mflare2",
        };

        for (r_idx, r) in main_rooms.iter().enumerate() {
            let deco_cls = if !cfg.custom_decoration_loadout.is_empty() {
                cfg.custom_decoration_loadout[r_idx % cfg.custom_decoration_loadout.len()].as_str()
            } else {
                theme_decos[r_idx % theme_decos.len()]
            };

            let corner_f = if r.is_octagonal { 0.20 } else { 0.38 };
            let dx = (r.size.x * corner_f).max(96.0);
            let dy = (r.size.y * corner_f).max(96.0);
            let fz = r.floor_z() + 36.0;

            for (corner_i, (cx, cy)) in [(-dx, -dy), (dx, dy)].into_iter().enumerate() {
                let mut deco = WizardActorSpec {
                    name: format!("{deco_cls}_{}_{}", r_idx, corner_i),
                    class_name: deco_cls.to_string(),
                    location: Vec3::new(r.center.x + cx, r.center.y + cy, fz),
                    group: "Decorations".to_string(),
                    ..Default::default()
                };
                deco.bool_props.push(("bBlockActors".to_string(), false));
                deco.bool_props.push(("bBlockPlayers".to_string(), false));
                actors.push(deco);
            }
        }

        // Place AmbientSound emitters in the primary hub and secondary wing
        if let Some(hub) = main_rooms.first() {
            let mut amb = WizardActorSpec {
                name: "AmbientSound0".to_string(),
                class_name: "AmbientSound".to_string(),
                location: Vec3::new(hub.center.x, hub.center.y, hub.center.z),
                group: "Audio".to_string(),
                ..Default::default()
            };
            amb.byte_props.push(("SoundRadius".to_string(), 96));
            amb.byte_props.push(("SoundVolume".to_string(), 165));
            amb.byte_props.push(("SoundPitch".to_string(), 64));
            let snd_str = cfg
                .custom_ambient_sounds
                .first()
                .cloned()
                .unwrap_or_else(|| default_theme_ambience.to_string());
            amb.str_props.push(("AmbientSound".to_string(), snd_str));
            actors.push(amb);
        }
        if let Some(sec_room) = main_rooms.get(1) {
            let mut amb1 = WizardActorSpec {
                name: "AmbientSound1".to_string(),
                class_name: "AmbientSound".to_string(),
                location: Vec3::new(sec_room.center.x, sec_room.center.y, sec_room.center.z),
                group: "Audio".to_string(),
                ..Default::default()
            };
            amb1.byte_props.push(("SoundRadius".to_string(), 72));
            amb1.byte_props.push(("SoundVolume".to_string(), 140));
            amb1.byte_props.push(("SoundPitch".to_string(), 64));
            let snd_str1 = cfg
                .custom_ambient_sounds
                .get(1)
                .or_else(|| cfg.custom_ambient_sounds.first())
                .cloned()
                .unwrap_or_else(|| default_theme_ambience.to_string());
            amb1.str_props.push(("AmbientSound".to_string(), snd_str1));
            actors.push(amb1);
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 5C: Kinetic Kicker Jump-Pads (Ballistic Arc to Upper Balconies)
    // -----------------------------------------------------------------------
    if cfg.toggles.include_jump_pads {
        if let Some(hub) = main_rooms.first() {
            if hub.size.z >= 512.0 {
                let jx = hub.center.x + (hub.size.x * 0.26).clamp(160.0, 320.0);
                let jy = hub.center.y;
                let jz = hub.floor_z() + 8.0;

                let mut pad = build_cylinder(
                    "JumpPadBase_0".to_string(),
                    CsgOperation::Add,
                    12.0,
                    40.0,
                    8,
                );
                pad.location = Vec3::new(jx, jy, hub.floor_z() + 6.0);
                pad.group = Some("JumpPads".to_string());
                for p in &mut pad.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID;
                }
                pad.recompute_uvs();
                semisolid_detail_brushes.push(pad);

                // Compute ballistic KickVelocity under UT99 gravity (g = 950 UU/s^2)
                let kick_vel = if let Some(target) = sniper_balcony_loc {
                    let g = 950.0_f32;
                    let dz = (target.z + 32.0 - (jz + 24.0)).max(128.0);
                    let vz = (2.0 * g * (dz + 96.0)).sqrt();
                    let t_flight = (vz + (vz * vz - 2.0 * g * dz).max(0.0).sqrt()) / g;
                    let vx = (target.x - jx) / t_flight.max(0.25);
                    let vy = (target.y - jy) / t_flight.max(0.25);
                    Vec3::new(vx.clamp(-600.0, 600.0), vy.clamp(-600.0, 600.0), vz.clamp(600.0, 1100.0))
                } else {
                    Vec3::new(0.0, 220.0, 760.0)
                };

                let mut kicker = WizardActorSpec {
                    name: "Kicker0".to_string(),
                    class_name: "Kicker".to_string(),
                    location: Vec3::new(jx, jy, jz + 24.0),
                    group: "JumpPads".to_string(),
                    ..Default::default()
                };
                kicker
                    .name_props
                    .push(("KickedClasses".to_string(), "Pawn".to_string()));
                kicker.bool_props.push(("bKillVelocity".to_string(), true));
                kicker.vec_props.push(("KickVelocity".to_string(), kick_vel));
                actors.push(kicker);

                if cfg.toggles.include_bot_ai_paths {
                    if let Some(target) = sniper_balcony_loc {
                        let mut js = WizardActorSpec {
                            name: "JumpSpot_BalconyLanding".to_string(),
                            class_name: "JumpSpot".to_string(),
                            location: Vec3::new(target.x, target.y, target.z + 40.0),
                            group: "AI_Navigation".to_string(),
                            ..Default::default()
                        };
                        js.bool_props.push(("bImpactJump".to_string(), false));
                        actors.push(js);
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 5D: Bidirectional Teleporter Portals (`VisibleTeleporter0 <-> VisibleTeleporter1`)
    // -----------------------------------------------------------------------
    if cfg.toggles.include_teleporters && main_rooms.len() >= 2 {
        let pairs: Vec<(usize, usize, String, String)> = if cfg.game_mode == WizardGameMode::CaptureTheFlag {
            if cfg.archetype == WizardArchetype::FacingWorldsOrbitalSpire {
                let red_sanctum = main_rooms.iter().position(|r| r.name == "RedFlagSanctum");
                let red_roost = main_rooms.iter().position(|r| r.name == "RedSniperRoost");
                let blue_sanctum = main_rooms.iter().position(|r| r.name == "BlueFlagSanctum");
                let blue_roost = main_rooms.iter().position(|r| r.name == "BlueSniperRoost");
                let mut p = Vec::new();
                if let (Some(rs), Some(rr)) = (red_sanctum, red_roost) {
                    p.push((0, rs, "Tele_RedBase".to_string(), "Tele_RedRoost".to_string()));
                    p.push((1, rr, "Tele_RedRoost".to_string(), "Tele_RedBase".to_string()));
                }
                if let (Some(bs), Some(br)) = (blue_sanctum, blue_roost) {
                    p.push((2, bs, "Tele_BlueBase".to_string(), "Tele_BlueRoost".to_string()));
                    p.push((3, br, "Tele_BlueRoost".to_string(), "Tele_BlueBase".to_string()));
                }
                p
            } else {
                let neutral_indices: Vec<usize> = main_rooms
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.team_owner.is_none())
                    .map(|(i, _)| i)
                    .collect();
                let candidate_neutrals: Vec<usize> = neutral_indices
                    .iter()
                    .copied()
                    .filter(|&i| i != 0)
                    .collect();
                let effective_neutrals = if candidate_neutrals.len() >= 2 {
                    candidate_neutrals
                } else {
                    neutral_indices
                };
                if effective_neutrals.len() >= 2 {
                    let na = effective_neutrals[0];
                    let nb = effective_neutrals[effective_neutrals.len() - 1];
                    vec![
                        (0, na, "WizardTeleA".to_string(), "WizardTeleB".to_string()),
                        (1, nb, "WizardTeleB".to_string(), "WizardTeleA".to_string()),
                    ]
                } else if main_rooms.len() >= 3 {
                    let na = 0;
                    let nb = 0;
                    vec![
                        (0, na, "WizardTeleA".to_string(), "WizardTeleB".to_string()),
                        (1, nb, "WizardTeleB".to_string(), "WizardTeleA".to_string()),
                    ]
                } else {
                    Vec::new()
                }
            }
        } else {
            let idx_a = if main_rooms.len() >= 3 { 1 } else { 0 };
            let idx_b = main_rooms.len() - 1;
            vec![
                (0, idx_a, "WizardTeleA".to_string(), "WizardTeleB".to_string()),
                (1, idx_b, "WizardTeleB".to_string(), "WizardTeleA".to_string()),
            ]
        };

        let is_same_room = pairs.len() == 2 && pairs[0].1 == pairs[1].1;
        for (t_i, r_idx, tag_name, url_target) in pairs {
            let r = &main_rooms[r_idx];
            let fz = r.floor_z();
            let (tx, ty) = if is_same_room {
                let offset = (r.size.x * 0.32).clamp(180.0, 320.0);
                if t_i == 0 {
                    (r.center.x - offset, r.center.y)
                } else {
                    (r.center.x + offset, r.center.y)
                }
            } else {
                (
                    r.center.x - (r.size.x * 0.18).min(160.0),
                    r.center.y + (r.size.y * 0.08).min(64.0),
                )
            };

            // Frame the portal with two flanking CSG pillars and an overhead lintel header
            for (b_name, b_size, b_loc) in [
                (
                    format!("TeleporterPillarL_{t_i}"),
                    Vec3::new(20.0, 20.0, 128.0),
                    Vec3::new(tx, ty - 76.0, fz + 64.0),
                ),
                (
                    format!("TeleporterPillarR_{t_i}"),
                    Vec3::new(20.0, 20.0, 128.0),
                    Vec3::new(tx, ty + 76.0, fz + 64.0),
                ),
                (
                    format!("TeleporterArchTop_{t_i}"),
                    Vec3::new(24.0, 172.0, 20.0),
                    Vec3::new(tx, ty, fz + 138.0),
                ),
            ] {
                let mut b = build_cube(b_name, CsgOperation::Add, b_size);
                b.location = b_loc;
                b.group = Some("Architecture".to_string());
                for p in &mut b.polys {
                    p.texture = pal.trim_texture.clone();
                    p.flags |= PF_SEMISOLID | PF_NOT_SOLID;
                }
                b.recompute_uvs();
                semisolid_detail_brushes.push(b);
            }

            // Compute exit Yaw facing toward the room center so arriving players never face a wall
            let dx = r.center.x - tx;
            let dy = r.center.y - ty;
            let yaw_uu = ((dy.atan2(dx).to_degrees() / 360.0) * 65536.0)
                .rem_euclid(65536.0) as i32;

            let mut tele = WizardActorSpec {
                name: format!("VisibleTeleporter{t_i}"),
                class_name: "VisibleTeleporter".to_string(),
                location: Vec3::new(tx, ty, fz + 41.0),
                rotation: (0, yaw_uu, 0),
                tag: tag_name.to_string(),
                group: "AI_Navigation".to_string(),
                ..Default::default()
            };
            tele.str_props
                .push(("URL".to_string(), url_target.to_string()));
            tele.bool_props.push(("bEnabled".to_string(), true));
            tele.bool_props.push(("bChangesYaw".to_string(), true));
            tele.bool_props.push(("bHidden".to_string(), false));
            actors.push(tele);

            if cfg.toggles.include_lighting {
                actors.push(WizardActorSpec {
                    name: format!("Light_Teleporter{t_i}"),
                    class_name: "Light".to_string(),
                    location: Vec3::new(tx, ty, fz + 96.0),
                    group: "Lighting".to_string(),
                    light_brightness: 170,
                    light_hue: 190,
                    light_saturation: 64,
                    light_radius: 12,
                    ..Default::default()
                });
            }
        }
    }

    // -----------------------------------------------------------------------
    // PHASE 6: Connected Bot AI Navigation Network (`PathNode`)
    // -----------------------------------------------------------------------
    if cfg.toggles.include_bot_ai_paths {
        let mut pn_idx = 0usize;
        let try_push_pathnode = |loc: Vec3, actors: &mut Vec<WizardActorSpec>, pn_idx: &mut usize| {
            // Check if any existing PathNode or PlayerStart is within proximity or if a dedicated stair waypoint already covers this XY area
            let too_close = actors.iter().any(|a| {
                (a.class_name == "PathNode" || a.class_name == "PlayerStart" || a.class_name == "JumpSpot")
                    && (a.location.distance(loc) < 160.0
                        || ((a.name.starts_with("PathNode_Foot_")
                            || a.name.starts_with("PathNode_Head_")
                            || a.name.starts_with("JumpSpot_Landing_")
                            || a.name.contains("_Catwalk_")
                            || a.name.starts_with("PathNode_Ramp"))
                            && (a.location.x - loc.x).hypot(a.location.y - loc.y) < 220.0))
            });
            if !too_close {
                actors.push(WizardActorSpec {
                    name: format!("PathNode{pn_idx}"),
                    class_name: "PathNode".to_string(),
                    location: loc,
                    group: "AI_Navigation".to_string(),
                    ..Default::default()
                });
                *pn_idx += 1;
            }
        };

        for r in &nodes {
            let fz = r.floor_z() + 40.0;
            // Central node in every room and corridor (unless occupied by a hazard pit)
            if !r.has_hazard_pit || !cfg.toggles.include_environmental_hazards {
                try_push_pathnode(Vec3::new(r.center.x, r.center.y, fz), &mut actors, &mut pn_idx);
            }

            if !r.is_corridor {
                // Place 4 cardinal perimeter waypoints inside each room so bots smoothly enter/exit all corridors
                // and skirt around any central hazard pit, staying safely inside the room bounds.
                let max_rx = (r.size.x * 0.5 - 64.0).max(48.0);
                let max_ry = (r.size.y * 0.5 - 64.0).max(48.0);
                let rx = (r.size.x * 0.32).clamp(48.0, max_rx).min(320.0);
                let ry = (r.size.y * 0.32).clamp(48.0, max_ry).min(320.0);
                for (dx, dy) in [(-rx, 0.0), (rx, 0.0), (0.0, -ry), (0.0, ry)] {
                    try_push_pathnode(Vec3::new(r.center.x + dx, r.center.y + dy, fz), &mut actors, &mut pn_idx);
                }
            } else {
                // Place intermediate waypoints along corridors so ReachSpecs (< 800 UU) always link across all scales
                let span_x = r.size.x;
                let span_y = r.size.y;
                if span_x >= span_y && span_x > 450.0 {
                    let half_usable = span_x * 0.36;
                    let n_side = ((half_usable / 320.0).ceil() as i32).max(1);
                    for s in 1..=n_side {
                        let off = (s as f32 / n_side as f32) * half_usable;
                        for sign in [-1.0_f32, 1.0_f32] {
                            try_push_pathnode(Vec3::new(r.center.x + sign * off, r.center.y, fz), &mut actors, &mut pn_idx);
                        }
                    }
                } else if span_y > 450.0 {
                    let half_usable = span_y * 0.36;
                    let n_side = ((half_usable / 320.0).ceil() as i32).max(1);
                    for s in 1..=n_side {
                        let off = (s as f32 / n_side as f32) * half_usable;
                        for sign in [-1.0_f32, 1.0_f32] {
                            try_push_pathnode(Vec3::new(r.center.x, r.center.y + sign * off, fz), &mut actors, &mut pn_idx);
                        }
                    }
                }
            }

        }
    }

    // -----------------------------------------------------------------------
    // PHASE 6B: Physical placement (rest on surfaces, keep clear of geometry and of each other)
    // Authored positions are only hints; `PlacementWorld` resolves them with the real UT99 collision
    // sizes (see `class_profiles`): MedBox -> floor+9, PlayerStart -> floor+41, Fan2 -> 52uu under the
    // ceiling, nothing inside walls / pillars / pads / pits.
    // -----------------------------------------------------------------------
    {
        let mut world = PlacementWorld::from_brushes(
            sub_brushes
                .iter()
                .chain(add_structural_brushes.iter())
                .chain(portal_sheet_brushes.iter())
                .chain(semisolid_detail_brushes.iter()),
        );
        let rank = |class: &str| -> u8 {
            let l = class.to_ascii_lowercase();
            if l == "playerstart" {
                0
            } else if matches!(
                l.as_str(),
                "flagbase" | "controlpoint" | "fortstandard" | "teleporter" | "visibleteleporter"
            ) {
                1
            } else if is_gameplay_pickup(class) {
                2
            } else if is_navigation_point(class) {
                3
            } else {
                4
            }
        };
        let mut order: Vec<usize> = (0..actors.len()).collect();
        order.sort_by_key(|&i| rank(&actors[i].class_name));
        let mut unplaceable: Vec<usize> = Vec::new();
        for i in order {
            let cls = actors[i].class_name.clone();
            let hint = actors[i].location;
            if cls.eq_ignore_ascii_case("Light") {
                if let Some(p) = world.nudge_free(hint, 10.0) {
                    actors[i].location = p;
                }
                continue;
            }
            let placed = if is_navigation_point(&cls) {
                world.place_nav(&cls, hint)
            } else if class_profile(&cls).mount == Mount::Fixed {
                continue;
            } else {
                world.place(&cls, hint)
            };
            match placed {
                Some(p) => actors[i].location = p,
                None => unplaceable.push(i),
            }
        }
        // Pure decoration that cannot be placed legally is dropped;
        // Gameplay actors that remain inside solid obstacles are also dropped to avoid collision snags / isolated InventorySpots.
        // Navigation points that have no physical floor within reach are also dropped to prevent floating nodes above voids.
        unplaceable.sort_unstable_by(|a, b| b.cmp(a));
        for i in unplaceable {
            let cls = &actors[i].class_name;
            if cls.eq_ignore_ascii_case("PlayerStart") {
                // Safeguard PlayerStarts: never leave them unplaced in solid bedrock/void.
                // Reset safely to their owning room center floor with resting offset.
                if let Some(r) = nodes.iter().find(|n| {
                    !n.is_corridor
                        && (actors[i].location.x - n.center.x).abs() <= n.size.x * 0.5 + 64.0
                        && (actors[i].location.y - n.center.y).abs() <= n.size.y * 0.5 + 64.0
                        && actors[i].location.z >= n.floor_z() - 64.0
                        && actors[i].location.z <= n.ceiling_z() + 64.0
                }) {
                    actors[i].location = Vec3::new(r.center.x, r.center.y, r.floor_z() + 41.0);
                }
            } else if actors[i].group == "Decorations"
                || world.in_solid(actors[i].location)
                || is_navigation_point(cls)
            {
                actors.remove(i);
            }
        }

        // Enforce CTF / Symmetrical exact mathematical parity for all paired Blue actors
        let is_ctf_symmetrical = (cfg.game_mode == WizardGameMode::CaptureTheFlag
            && cfg.archetype.category() == WizardArchetypeCategory::SymmetricalTeam)
            || cfg.symmetry_mode != WizardSymmetryMode::None;
        if is_ctf_symmetrical {
            let sym_mode: CtfSymmetryMode = cfg.symmetry_mode.into();
            let sym_center = if let Some(plan) = &ctf_plan {
                (plan.red_flag_loc + plan.blue_flag_loc) * 0.5
            } else {
                Vec3::ZERO
            };

            let red_locs: Vec<(String, Vec3)> = actors
                .iter()
                .filter(|a| a.group == "Pickups" && (a.name.contains("_Red_") || a.name.ends_with("_Red")))
                .map(|a| (a.name.clone(), a.location))
                .collect();

            for act in &mut actors {
                if act.group == "Pickups" && (act.name.contains("_Blue_") || act.name.ends_with("_Blue")) {
                    let matching_red_name = act.name.replace("_Blue_", "_Red_").replace("_Blue", "_Red");
                    if let Some((_, red_loc)) = red_locs.iter().find(|(name, _)| *name == matching_red_name) {
                        let mirrored = transform_point_symmetry(*red_loc, sym_mode, sym_center);
                        let profile = class_profile(&act.class_name);
                        let supported = match profile.mount {
                            Mount::Floor => world.floor_under(mirrored.x, mirrored.y, mirrored.z + 8.0)
                                .is_some_and(|floor| (mirrored.z - floor - profile.rest_offset).abs() <= 1.0),
                            Mount::Ceiling => world.ceiling_above(mirrored.x, mirrored.y, mirrored.z)
                                .is_some_and(|ceiling| (ceiling - mirrored.z - profile.rest_offset).abs() <= 1.0),
                            Mount::Fixed => true,
                        };
                        // Symmetric layouts can contain asymmetric details: do
                        // not overwrite a valid placement with an invalid one.
                        if supported && world.cylinder_free(mirrored, profile.radius + 2.0, profile.height) {
                            act.location = mirrored;
                        } else if act.location.distance(mirrored) > 1.0 {
                            warnings.push(format!("{} could not be mirrored safely; preserved its independently placed position", act.name));
                        }
                    }
                }
            }
        }

        // Enforce clearance for all pickups against solid detail and structural geometry
        let all_solid_brushes: Vec<&Brush> = add_structural_brushes
            .iter()
            .chain(semisolid_detail_brushes.iter())
            .filter(|b| {
                (b.csg_oper == CsgOperation::Add
                    || b.polys.iter().any(|p| (p.flags & PF_SEMISOLID) != 0))
                    && !b.polys.iter().any(|p| (p.flags & PF_NOT_SOLID) != 0)
            })
            .collect();

        actors.retain(|act| {
            if act.group != "Pickups" {
                return true;
            }
            let loc = act.location;
            for brush in &all_solid_brushes {
                let (min_b, max_b) = brush.world_bounds();
                let margin = 4.0;
                if loc.x >= min_b.x + margin
                    && loc.x <= max_b.x - margin
                    && loc.y >= min_b.y + margin
                    && loc.y <= max_b.y - margin
                    && loc.z >= min_b.z + margin
                    && loc.z <= max_b.z - margin
                {
                    let world_polys = brush.to_world_polys();
                    let inside_convex = !world_polys.is_empty()
                        && world_polys
                            .iter()
                            .all(|p| p.plane.dist_to_point(loc) <= -margin);
                    if inside_convex {
                        return false;
                    }
                }
            }
            true
        });

        // Final Telefrag Prevention: Guarantee >= 220 UU distance between all PlayerStarts
        let mut ps_locs: Vec<(usize, Vec3)> = actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.class_name.eq_ignore_ascii_case("PlayerStart"))
            .map(|(i, a)| (i, a.location))
            .collect();

        for _ in 0..4 {
            let mut moved = false;
            for i in 0..ps_locs.len() {
                for j in (i + 1)..ps_locs.len() {
                    // Vertical tier isolation: players on different floors never telefrag each other
                    if (ps_locs[i].1.z - ps_locs[j].1.z).abs() > 80.0 {
                        continue;
                    }
                    let diff_2d = Vec3::new(ps_locs[j].1.x - ps_locs[i].1.x, ps_locs[j].1.y - ps_locs[i].1.y, 0.0);
                    let d = diff_2d.length();
                    if d < 220.0 {
                        let dir = if d > 1e-3 {
                            diff_2d / d
                        } else {
                            Vec3::new(1.0, 0.0, 0.0)
                        };
                        let push = (225.0 - d) * 0.5;
                        ps_locs[i].1 -= dir * push;
                        ps_locs[j].1 += dir * push;
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
        for (idx, loc) in ps_locs {
            let mut clamped = loc;
            if let Some(r) = nodes.iter().find(|n| {
                !n.is_corridor
                    && (clamped.x - n.center.x).abs() <= n.size.x * 0.5 + 32.0
                    && (clamped.y - n.center.y).abs() <= n.size.y * 0.5 + 32.0
                    && clamped.z >= n.floor_z() - 32.0
                    && clamped.z <= n.ceiling_z() + 32.0
            }) {
                clamped.x = clamped.x.clamp(r.min_x() + 48.0, r.max_x() - 48.0);
                clamped.y = clamped.y.clamp(r.min_y() + 48.0, r.max_y() - 48.0);
                clamped.z = clamped.z.max(r.floor_z() + 41.0);
            }
            actors[idx].location = clamped;
        }
    }

    // Filter out actors suppressed by the user via 2D minimap
    if !cfg.suppressed_markers.is_empty() {
        actors.retain(|a| !cfg.suppressed_markers.iter().any(|s| s == &a.name || s == &a.tag));
    }

    // Instantiate user-created custom markers from 2D minimap into real UT99 actors
    for cm in &cfg.custom_markers {
        if cfg.suppressed_markers.iter().any(|s| s == &cm.label || s == &cm.id) {
            continue;
        }
        let cls = if !cm.class_name.is_empty() {
            cm.class_name.clone()
        } else {
            match cm.kind.as_str() {
                "PlayerStart" => "Engine.PlayerStart".to_string(),
                "Powerup" => "Botpack.UT_ShieldBelt".to_string(),
                "Health" => "Botpack.MedBox".to_string(),
                "Armor" => "Botpack.Armor2".to_string(),
                "Light" => "Engine.Light".to_string(),
                "Teleporter" => "Engine.Teleporter".to_string(),
                _ => "Botpack.ShockRifle".to_string(),
            }
        };
        let tele_tag = if cm.kind == "Teleporter" && !cm.teleporter_tag.is_empty() {
            cm.teleporter_tag.clone()
        } else {
            cm.label.clone()
        };
        let mut spec = WizardActorSpec {
            name: cm.label.clone(),
            class_name: cls,
            location: cm.location,
            rotation: (0, 0, 0),
            tag: tele_tag,
            event: String::new(),
            group: "MinimapCustom".to_string(),
            light_brightness: if cm.kind == "Light" { 255 } else { 128 },
            light_radius: if cm.kind == "Light" { 64 } else { 48 },
            light_hue: 0,
            light_saturation: 128,
            int_props: Vec::new(),
            byte_props: Vec::new(),
            float_props: Vec::new(),
            bool_props: Vec::new(),
            name_props: Vec::new(),
            str_props: Vec::new(),
            vec_props: Vec::new(),
        };
        if cm.kind == "Teleporter" {
            if !cm.teleporter_url.is_empty() {
                spec.str_props.push(("URL".to_string(), cm.teleporter_url.clone()));
            }
            spec.bool_props.push(("bEnabled".to_string(), true));
            spec.bool_props.push(("bChangesYaw".to_string(), true));
            spec.bool_props.push(("bHidden".to_string(), false));
        }
        if let Some(t) = cm.team {
            spec.byte_props.push(("Team".to_string(), t));
        }
        actors.push(spec);
    }

    // -----------------------------------------------------------------------
    // PHASE 7: Assemble Brushes in Strict Chronological CSG Order
    // (1. Subtract -> 2. Structural Add -> 3. Portals/Sheets -> 4. Semi-Solids)
    // -----------------------------------------------------------------------
    // Safety Reconciliation: Enforce Zero-HOM invariant by ensuring no Zone Portal brush
    // intersects any Semi-Solid architectural brush (LevelDoctor Deliverable E.6)
    portal_sheet_brushes.retain(|portal| {
        !semisolid_detail_brushes.iter().any(|ss| brushes_overlap(portal, ss))
    });

    let mut brushes = Vec::with_capacity(
        sub_brushes.len()
            + add_structural_brushes.len()
            + portal_sheet_brushes.len()
            + semisolid_detail_brushes.len(),
    );
    brushes.extend(sub_brushes);
    brushes.extend(add_structural_brushes);
    brushes.extend(portal_sheet_brushes);
    brushes.extend(semisolid_detail_brushes);

    WizardGeneratedMap {
        brushes,
        movers,
        actors,
        warnings,
        preview,
        default_game_type: cfg.game_mode.default_game_type_class().to_string(),
        song: if !cfg.music_track.is_empty() {
            cfg.music_track.clone()
        } else {
            pal.song
        },
        ambient_brightness: pal.ambient_brightness,
        ambient_hue: pal.ambient_hue,
        ambient_saturation: pal.ambient_saturation,
    }
}

/// Materializes a Capture The Flag mission plan: FlagBase objectives with procedural team daises,
/// key spotlights with team hues (Red 0, Blue 160), looping ambient hum audio cues,
/// 3-tier bot defense hierarchy (Goal Guard, Chokepoint Sentries, High Sniper Overwatch with `bSniping = true`),
/// elevated semi-solid sniper perches with `SniperRifle` + `RifleAmmo`, balanced `AlternatePath` multi-lane networks,
/// and contested midfield mega-powerup (UDamage/ShieldBelt) + symmetrical forward armor (ThighPads).
fn build_ctf_mission_content(
    plan: &crate::ctf_mission::CtfMissionPlan,
    nodes: &[RoomNode],
    pal: &WizardResolvedPalette,
    cfg: &MapWizardConfig,
    actors: &mut Vec<WizardActorSpec>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    // 1. Red Base (Team 0) and Blue Base (Team 1) FlagBases & Procedural Daises
    let bases = [
        (0u8, "Red", plan.red_flag_loc, plan.red_base_node_index, 0u8),
        (1u8, "Blue", plan.blue_flag_loc, plan.blue_base_node_index, 160u8),
    ];

    let to_blue = (plan.blue_flag_loc - plan.red_flag_loc).normalize_or_zero();

    for (team_id, team_name, flag_loc, node_idx, light_hue) in bases {
        let room_fz = nodes.get(node_idx).map(|r| r.floor_z()).unwrap_or(flag_loc.z - 64.0);

        // A) Procedural Dais Geometry per Team Base
        match cfg.ctf_dais_style {
            crate::ctf_mission::CtfDaisStyle::SteppedTeamPedestal => {
                // Tier 1: Lower octagonal pedestal
                let mut base = build_aligned_octagon(
                    format!("CtfDaisBase_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(192.0, 192.0, 12.0),
                );
                base.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 6.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // Tier 2: Upper octagonal platform
                let mut top = build_aligned_octagon(
                    format!("CtfDaisTop_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(144.0, 144.0, 12.0),
                );
                top.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 18.0);
                top.group = Some("Objectives".to_string());
                for poly in &mut top.polys {
                    poly.texture = pal.floor_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                top.recompute_uvs();
                semisolid_detail_brushes.push(top);
            }
            crate::ctf_mission::CtfDaisStyle::AltarShrine => {
                // Octagonal shrine base (Height 24.0, flush with flagbase floor)
                let mut base = build_aligned_octagon(
                    format!("CtfShrineBase_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(192.0, 192.0, 24.0),
                );
                base.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 12.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // 4 Corner shrine pillars well clear of FlagBase cylinder (radius 60.0)
                for (pi, (dx, dy)) in [(-96.0, -96.0), (96.0, -96.0), (96.0, 96.0), (-96.0, 96.0)].into_iter().enumerate() {
                    let mut pylon = build_aligned_octagon(
                        format!("CtfShrinePillar_{team_name}_{pi}"),
                        CsgOperation::Add,
                        Vec3::new(20.0, 20.0, 96.0),
                    );
                    pylon.location = Vec3::new(flag_loc.x + dx, flag_loc.y + dy, room_fz + 48.0);
                    pylon.group = Some("Objectives".to_string());
                    for poly in &mut pylon.polys {
                        poly.texture = pal.pillar_texture.clone();
                        poly.flags |= PF_SEMISOLID;
                    }
                    pylon.recompute_uvs();
                    semisolid_detail_brushes.push(pylon);
                }
            }
            crate::ctf_mission::CtfDaisStyle::TechBunkerAlcove => {
                // Armored bunker base (Height 16.0)
                let mut base = build_aligned_octagon(
                    format!("CtfBunkerBase_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(192.0, 192.0, 16.0),
                );
                base.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 8.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.wall_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // Rear blast backplate behind the flag: oriented perpendicular to facing vector
                // with 128 UU standoff distance ensuring 60+ UU of clearance from FlagBase cylinder
                let facing = if team_id == 0 { to_blue } else { -to_blue };
                let (plate_w, plate_d) = if facing.x.abs() >= facing.y.abs() {
                    (16.0, 176.0)
                } else {
                    (176.0, 16.0)
                };
                let mut plate = build_cube(
                    format!("CtfBunkerBackplate_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(plate_w, plate_d, 96.0),
                );
                plate.location = Vec3::new(
                    flag_loc.x - facing.x * 128.0,
                    flag_loc.y - facing.y * 128.0,
                    room_fz + 48.0,
                );
                plate.group = Some("Objectives".to_string());
                for poly in &mut plate.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                plate.recompute_uvs();
                semisolid_detail_brushes.push(plate);
            }
            crate::ctf_mission::CtfDaisStyle::PillarCanopy => {
                // Base platform (Height 12.0)
                let mut base = build_aligned_octagon(
                    format!("CtfCanopyBase_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(176.0, 176.0, 12.0),
                );
                base.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 6.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // 4 Tall columns with wide clearance (±88.0, ±88.0)
                for (pi, (dx, dy)) in [(-88.0, -88.0), (88.0, -88.0), (88.0, 88.0), (-88.0, 88.0)].into_iter().enumerate() {
                    let mut col = build_cylinder(
                        format!("CtfCanopyCol_{team_name}_{pi}"),
                        CsgOperation::Add,
                        192.0,
                        14.0,
                        8,
                    );
                    col.location = Vec3::new(flag_loc.x + dx, flag_loc.y + dy, room_fz + 96.0);
                    col.group = Some("Objectives".to_string());
                    for poly in &mut col.polys {
                        poly.texture = pal.pillar_texture.clone();
                        poly.flags |= PF_SEMISOLID;
                    }
                    col.recompute_uvs();
                    semisolid_detail_brushes.push(col);
                }

                // Overhead canopy slab at 192..208 UU (60+ UU clear above FlagBase top at 132 UU)
                let mut canopy = build_aligned_octagon(
                    format!("CtfCanopyRoof_{team_name}"),
                    CsgOperation::Add,
                    Vec3::new(192.0, 192.0, 16.0),
                );
                canopy.location = Vec3::new(flag_loc.x, flag_loc.y, room_fz + 200.0);
                canopy.group = Some("Objectives".to_string());
                for poly in &mut canopy.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                canopy.recompute_uvs();
                semisolid_detail_brushes.push(canopy);
            }
        }

        // B) FlagBase Actor
        let mut fb = WizardActorSpec {
            name: format!("{team_name}FlagBase"),
            class_name: "FlagBase".to_string(),
            location: flag_loc,
            tag: "FlagBase".to_string(),
            event: format!("FlagTaken_{team_name}"),
            group: "Objectives".to_string(),
            ..Default::default()
        };
        fb.byte_props.push(("Team".to_string(), team_id));
        actors.push(fb);

        // C) Key Spotlight with team hue and dramatic corona
        let mut spot = WizardActorSpec {
            name: format!("CtfLight_{team_name}"),
            class_name: "Light".to_string(),
            location: Vec3::new(flag_loc.x, flag_loc.y, flag_loc.z + 140.0),
            rotation: (-16384, 0, 0), // Directed downwards onto the flag dais
            tag: format!("FlagTaken_{team_name}"),
            group: "Lighting".to_string(),
            ..Default::default()
        };
        spot.byte_props.push(("LightType".to_string(), 1)); // LT_Steady
        spot.byte_props.push(("LightEffect".to_string(), 8)); // LE_StaticSpot
        spot.byte_props.push(("LightCone".to_string(), 48));
        spot.byte_props.push(("LightBrightness".to_string(), 170));
        spot.byte_props.push(("LightHue".to_string(), light_hue));
        spot.byte_props.push(("LightSaturation".to_string(), 60));
        spot.byte_props.push(("LightRadius".to_string(), 32));
        spot.bool_props.push(("bCorona".to_string(), true));
        spot.str_props.push(("Corona".to_string(), "GenFX.Lensflare".to_string()));
        spot.float_props.push(("DrawScale".to_string(), 0.45));
        actors.push(spot);

        // D) Ambient Energy Hum Audio Cue
        let mut snd = WizardActorSpec {
            name: format!("CtfSound_{team_name}"),
            class_name: "AmbientSound".to_string(),
            location: Vec3::new(flag_loc.x, flag_loc.y, flag_loc.z + 32.0),
            group: "Audio".to_string(),
            ..Default::default()
        };
        snd.str_props.push(("AmbientSound".to_string(), "AmbModern.Looping.electric_hum1".to_string()));
        snd.byte_props.push(("SoundRadius".to_string(), 32));
        snd.byte_props.push(("SoundVolume".to_string(), 160));
        actors.push(snd);
    }

    // 2. Tactical Bot DefensePoints
    for dp in &plan.defense_points {
        let mut d_act = WizardActorSpec {
            name: dp.name.clone(),
            class_name: "DefensePoint".to_string(),
            location: dp.location,
            rotation: (0, dp.yaw, 0),
            group: "Objectives".to_string(),
            ..Default::default()
        };
        d_act.byte_props.push(("team".to_string(), dp.team));
        d_act.byte_props.push(("priority".to_string(), dp.priority));
        if dp.is_sniping {
            d_act.bool_props.push(("bSniping".to_string(), true));
        }
        d_act.str_props.push((
            "FortTag".to_string(),
            if dp.team == 0 { "RedFlagBase".to_string() } else { "BlueFlagBase".to_string() },
        ));
        actors.push(d_act);
    }

    // 3. Elevated Sniper Overwatch Perches with SniperRifle + RifleAmmo
    if cfg.ctf_include_sniper_perches {
        for perch in &plan.sniper_perches {
            let team_name = if perch.team == 0 { "Red" } else { "Blue" };
            // Semi-solid sniper deck resting on the floor (height 48 UU)
            let mut deck = build_aligned_octagon(
                format!("CtfSniperDeck_{team_name}"),
                CsgOperation::Add,
                Vec3::new(128.0, 128.0, 48.0),
            );
            deck.location = Vec3::new(perch.perch_location.x, perch.perch_location.y, perch.perch_location.z - 24.0);
            deck.group = Some("Objectives".to_string());
            for poly in &mut deck.polys {
                poly.texture = pal.trim_texture.clone();
                poly.flags |= PF_SEMISOLID;
            }
            deck.recompute_uvs();
            semisolid_detail_brushes.push(deck);

            // PathNode on sniper perch to guarantee full bidirectional navigation
            actors.push(WizardActorSpec {
                name: format!("PathNode_Sniper_{team_name}"),
                class_name: "PathNode".to_string(),
                location: perch.defense_point_loc,
                group: "AI_Navigation".to_string(),
                ..Default::default()
            });

            // SniperRifle weapon pickup
            actors.push(WizardActorSpec {
                name: format!("SniperRifle_{team_name}"),
                class_name: "SniperRifle".to_string(),
                location: perch.weapon_pickup_loc,
                group: "Pickups".to_string(),
                ..Default::default()
            });

            // RifleAmmo pickup adjacent to rifle
            actors.push(WizardActorSpec {
                name: format!("RifleAmmo_{team_name}"),
                class_name: "RifleAmmo".to_string(),
                location: perch.weapon_pickup_loc + Vec3::new(32.0, 0.0, 0.0),
                group: "Pickups".to_string(),
                ..Default::default()
            });
        }
    }

    // 4. Multi-Lane AlternatePath Navigation Network
    for (idx, ap) in plan.alternate_paths.iter().enumerate() {
        let team_name = if ap.team == 0 { "Red" } else { "Blue" };
        let mut ap_act = WizardActorSpec {
            name: format!("AlternatePath_{team_name}_{idx}"),
            class_name: "AlternatePath".to_string(),
            location: ap.location,
            group: "AI_Navigation".to_string(),
            ..Default::default()
        };
        ap_act.byte_props.push(("team".to_string(), ap.team));
        ap_act.float_props.push(("SelectionWeight".to_string(), ap.weight));
        if ap.return_only {
            ap_act.bool_props.push(("bReturnOnly".to_string(), true));
        }
        actors.push(ap_act);
    }

    // 5. Contested Midfield Mega-Powerup & Symmetrical Forward Armor
    if cfg.ctf_contested_center_powerup {
        if let Some(mut cp_loc) = plan.center_powerup_loc {
            if cfg.archetype == WizardArchetype::CanyonCrestCitadels {
                // In Canyon Crest Citadels (LavaGiant DNA), the origin (0,0) belongs to the signature Redeemer on the lava stepping stone.
                // The contested mega-powerup is placed in the subterranean magma cavern route (SouthCavernRun, Y = -1088).
                cp_loc = Vec3::new(0.0, -1088.0, -64.0 + 32.0);
            } else if cfg.archetype == WizardArchetype::FacingWorldsOrbitalSpire {
                // In Facing Worlds, the void crossing center belongs to the Redeemer;
                // Powerup sits on the lower void catwalk
                cp_loc = Vec3::new(0.0, -768.0, -64.0 + 32.0);
            }

            let active_powerups = cfg.active_powerups();
            let has_shieldbelt = active_powerups.is_empty() || active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UT_ShieldBelt"));
            let has_udamage = active_powerups.is_empty() || active_powerups.iter().any(|p| p.eq_ignore_ascii_case("UDamage"));

            let center_powerup_cls = if has_udamage && !has_shieldbelt {
                Some("UDamage")
            } else if has_shieldbelt && !has_udamage {
                Some("UT_ShieldBelt")
            } else if has_shieldbelt && has_udamage {
                if (cfg.seed % 2) == 0 {
                    Some("UDamage")
                } else {
                    Some("UT_ShieldBelt")
                }
            } else {
                None
            };

            if let Some(cls) = center_powerup_cls {
                let mut cent_ped = build_aligned_octagon(
                    "CtfCenterPedestal".to_string(),
                    CsgOperation::Add,
                    Vec3::new(96.0, 96.0, 16.0),
                );
                cent_ped.location = Vec3::new(cp_loc.x, cp_loc.y, cp_loc.z - 32.0 + 8.0);
                cent_ped.group = Some("Pickups".to_string());
                for poly in &mut cent_ped.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                cent_ped.recompute_uvs();
                semisolid_detail_brushes.push(cent_ped);

                actors.push(WizardActorSpec {
                    name: "CtfCenterPowerup".to_string(),
                    class_name: cls.to_string(),
                    location: Vec3::new(cp_loc.x, cp_loc.y, cp_loc.z - 32.0 + 16.0 + 11.0),
                    group: "Pickups".to_string(),
                    ..Default::default()
                });

            let mut c_light = WizardActorSpec {
                name: "CtfCenterLight".to_string(),
                class_name: "Light".to_string(),
                location: Vec3::new(cp_loc.x, cp_loc.y, cp_loc.z + 120.0),
                group: "Lighting".to_string(),
                ..Default::default()
            };
            c_light.byte_props.push(("LightType".to_string(), 1));
            c_light.byte_props.push(("LightEffect".to_string(), 13)); // LE_NonIncidence
            c_light.byte_props.push(("LightBrightness".to_string(), 180));
            c_light.byte_props.push(("LightHue".to_string(), 40));
            c_light.byte_props.push(("LightSaturation".to_string(), 120));
            c_light.byte_props.push(("LightRadius".to_string(), 32));
            c_light.bool_props.push(("bCorona".to_string(), true));
            c_light.str_props.push(("Corona".to_string(), "GenFX.Lensflare".to_string()));
            c_light.float_props.push(("DrawScale".to_string(), 0.45));
            actors.push(c_light);
            }
        }
    }

    // Symmetrical Forward Base Armor (ThighPads)
    if let Some(loc) = plan.forward_powerup_red {
        actors.push(WizardActorSpec {
            name: "RedForwardArmor".to_string(),
            class_name: "ThighPads".to_string(),
            location: loc,
            group: "Pickups".to_string(),
            ..Default::default()
        });
    }
    if let Some(loc) = plan.forward_powerup_blue {
        actors.push(WizardActorSpec {
            name: "BlueForwardArmor".to_string(),
            class_name: "ThighPads".to_string(),
            location: loc,
            group: "Pickups".to_string(),
            ..Default::default()
        });
    }
}

/// Materializes a Domination mission plan: ControlPoint objectives with monumental stepped daises,
/// ambient energy audio cues, focal key lighting, tactical AI `DefensePoint`s, perimeter `PathNode`s,
/// and contested central mega-powerup (ShieldBelt/Amp), mirroring official DOM-* maps.
fn build_domination_mission_content(
    plan: &crate::domination_mission::DominationMissionPlan,
    nodes: &[RoomNode],
    pal: &WizardResolvedPalette,
    cfg: &MapWizardConfig,
    actors: &mut Vec<WizardActorSpec>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    for p in &plan.points {
        let room_fz = nodes.get(p.room_index).map(|r| r.floor_z()).unwrap_or(p.location.z - 52.0);

        // 1. Procedural Dais Geometry
        let dais_top_z = match cfg.domination_dais_style {
            crate::domination_mission::DominationDaisStyle::SteppedOctagon => {
                // Tier 1: Lower base (radius 84, height 12)
                let mut base = build_aligned_octagon(
                    format!("DomDaisBase_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    Vec3::new(168.0, 168.0, 12.0),
                );
                base.location = Vec3::new(p.location.x, p.location.y, room_fz + 6.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // Tier 2: Upper platform (radius 60, height 12)
                let mut top = build_aligned_octagon(
                    format!("DomDaisTop_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    Vec3::new(120.0, 120.0, 12.0),
                );
                top.location = Vec3::new(p.location.x, p.location.y, room_fz + 18.0);
                top.group = Some("Objectives".to_string());
                for poly in &mut top.polys {
                    poly.texture = pal.floor_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                top.recompute_uvs();
                semisolid_detail_brushes.push(top);

                room_fz + 24.0
            }
            crate::domination_mission::DominationDaisStyle::PylonShrine => {
                // Octagonal platform (radius 76, height 14)
                let mut base = build_aligned_octagon(
                    format!("DomShrineBase_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    Vec3::new(152.0, 152.0, 14.0),
                );
                base.location = Vec3::new(p.location.x, p.location.y, room_fz + 7.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                // 4 Corner energy pillars
                for (pi, (dx, dy)) in [(-48.0, -48.0), (48.0, -48.0), (48.0, 48.0), (-48.0, 48.0)].into_iter().enumerate() {
                    let mut pylon = build_aligned_octagon(
                        format!("DomPylon_{}_{pi}", p.point_name.replace(' ', "_")),
                        CsgOperation::Add,
                        Vec3::new(20.0, 20.0, 52.0),
                    );
                    pylon.location = Vec3::new(p.location.x + dx, p.location.y + dy, room_fz + 26.0);
                    pylon.group = Some("Objectives".to_string());
                    for poly in &mut pylon.polys {
                        poly.texture = pal.pillar_texture.clone();
                        poly.flags |= PF_SEMISOLID;
                    }
                    pylon.recompute_uvs();
                    semisolid_detail_brushes.push(pylon);
                }

                room_fz + 14.0
            }
            crate::domination_mission::DominationDaisStyle::IndustrialGrate => {
                // Platform (radius 80, height 16)
                let mut base = build_aligned_octagon(
                    format!("DomGrateBase_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    Vec3::new(160.0, 160.0, 16.0),
                );
                base.location = Vec3::new(p.location.x, p.location.y, room_fz + 8.0);
                base.group = Some("Objectives".to_string());
                for poly in &mut base.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                base.recompute_uvs();
                semisolid_detail_brushes.push(base);

                room_fz + 16.0
            }
            crate::domination_mission::DominationDaisStyle::SunkenAmphitheater => {
                let mut outer = build_cylinder(
                    format!("DomOuterAmphi_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    10.0,
                    92.0,
                    12,
                );
                outer.location = Vec3::new(p.location.x, p.location.y, room_fz + 5.0);
                outer.group = Some("Objectives".to_string());
                for poly in &mut outer.polys {
                    poly.texture = pal.trim_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                outer.recompute_uvs();
                semisolid_detail_brushes.push(outer);

                let mut inner = build_cylinder(
                    format!("DomInnerAmphi_{}", p.point_name.replace(' ', "_")),
                    CsgOperation::Add,
                    10.0,
                    64.0,
                    12,
                );
                inner.location = Vec3::new(p.location.x, p.location.y, room_fz + 15.0);
                inner.group = Some("Objectives".to_string());
                for poly in &mut inner.polys {
                    poly.texture = pal.floor_texture.clone();
                    poly.flags |= PF_SEMISOLID;
                }
                inner.recompute_uvs();
                semisolid_detail_brushes.push(inner);

                room_fz + 20.0
            }
        };

        // 2. ControlPoint Actor (rests on top of dais, height 50.0)
        let mut cp = WizardActorSpec {
            name: format!("ControlPoint_{}", p.point_name.replace(' ', "_")),
            class_name: "ControlPoint".to_string(),
            location: Vec3::new(p.location.x, p.location.y, dais_top_z + 50.0),
            tag: p.tag.clone(),
            event: format!("Captured_{}", p.point_name.replace(' ', "_")),
            group: "Objectives".to_string(),
            ..Default::default()
        };
        cp.str_props.push(("PointName".to_string(), p.point_name.clone()));
        cp.byte_props.push(("LightType".to_string(), 1)); // LT_Steady
        cp.byte_props.push(("LightBrightness".to_string(), 128));
        cp.byte_props.push(("LightHue".to_string(), p.light_hue));
        cp.byte_props.push(("LightSaturation".to_string(), 80));
        cp.byte_props.push(("LightRadius".to_string(), 20));
        actors.push(cp);

        // 3. Focal Key Light with static spotlight beam and corona
        let mut spot = WizardActorSpec {
            name: format!("DomLight_{}", p.point_name.replace(' ', "_")),
            class_name: "Light".to_string(),
            location: Vec3::new(p.location.x, p.location.y, dais_top_z + 160.0),
            rotation: (-16384, 0, 0), // Directed downwards onto the control dais
            group: "Lighting".to_string(),
            ..Default::default()
        };
        spot.byte_props.push(("LightType".to_string(), 1));
        spot.byte_props.push(("LightEffect".to_string(), 8)); // LE_StaticSpot
        spot.byte_props.push(("LightCone".to_string(), 48));
        spot.byte_props.push(("LightBrightness".to_string(), 160));
        spot.byte_props.push(("LightHue".to_string(), p.light_hue));
        spot.byte_props.push(("LightSaturation".to_string(), 90));
        spot.byte_props.push(("LightRadius".to_string(), 32));
        spot.bool_props.push(("bCorona".to_string(), true));
        spot.str_props.push(("Corona".to_string(), "GenFX.Lensflare".to_string()));
        spot.float_props.push(("DrawScale".to_string(), 0.45));
        actors.push(spot);

        // 4. Ambient Energy Hum Sound
        let mut snd = WizardActorSpec {
            name: format!("DomSound_{}", p.point_name.replace(' ', "_")),
            class_name: "AmbientSound".to_string(),
            location: Vec3::new(p.location.x, p.location.y, dais_top_z + 32.0),
            group: "Audio".to_string(),
            ..Default::default()
        };
        snd.str_props.push(("AmbientSound".to_string(), "AmbModern.Looping.electric_hum1".to_string()));
        snd.byte_props.push(("SoundRadius".to_string(), 32));
        snd.byte_props.push(("SoundVolume".to_string(), 160));
        actors.push(snd);

        // 5. Tactical AI DefensePoints
        if cfg.domination_include_defense_points {
            let dp_offsets = [
                (Vec3::new(-200.0, -160.0, 0.0), 16384),
                (Vec3::new(200.0, 160.0, 0.0), 49152),
            ];
            for (dp_i, (off, yaw)) in dp_offsets.into_iter().enumerate() {
                let mut dp = WizardActorSpec {
                    name: format!("DefensePoint_Dom_{}_{dp_i}", p.point_name.replace(' ', "_")),
                    class_name: "DefensePoint".to_string(),
                    location: Vec3::new(p.location.x + off.x, p.location.y + off.y, room_fz + 51.0),
                    rotation: (0, yaw, 0),
                    group: "Objectives".to_string(),
                    ..Default::default()
                };
                dp.str_props.push(("FortTag".to_string(), p.tag.clone()));
                dp.byte_props.push(("Priority".to_string(), if p.role == crate::domination_mission::DominationPointRole::HighTier { 2 } else { 1 }));
                actors.push(dp);
            }
        }

        // 6. Cardinal AI Navigation PathNodes around Dais
        for (wp_i, (dx, dy)) in [(140.0, 0.0), (-140.0, 0.0), (0.0, 140.0), (0.0, -140.0)].into_iter().enumerate() {
            actors.push(WizardActorSpec {
                name: format!("PathNode_Dom_{}_{wp_i}", p.point_name.replace(' ', "_")),
                class_name: "PathNode".to_string(),
                location: Vec3::new(p.location.x + dx, p.location.y + dy, room_fz + 51.0),
                group: "AI_Navigation".to_string(),
                ..Default::default()
            });
        }
    }

    // 7. Contested Central Mega-Powerup
    if cfg.domination_contested_center_powerup {
        if let Some(cent_pos) = plan.center_powerup_location {
            let mut cent_ped = build_aligned_octagon(
                "DomCenterPedestal".to_string(),
                CsgOperation::Add,
                Vec3::new(96.0, 96.0, 16.0),
            );
            cent_ped.location = Vec3::new(cent_pos.x, cent_pos.y, cent_pos.z - 32.0 + 8.0);
            cent_ped.group = Some("Pickups".to_string());
            for poly in &mut cent_ped.polys {
                poly.texture = pal.trim_texture.clone();
                poly.flags |= PF_SEMISOLID;
            }
            cent_ped.recompute_uvs();
            semisolid_detail_brushes.push(cent_ped);

            actors.push(WizardActorSpec {
                name: "DomCenterPowerup".to_string(),
                class_name: "UT_ShieldBelt".to_string(),
                location: Vec3::new(cent_pos.x, cent_pos.y, cent_pos.z - 32.0 + 16.0 + 11.0),
                group: "Pickups".to_string(),
                ..Default::default()
            });

            let mut c_light = WizardActorSpec {
                name: "DomCenterLight".to_string(),
                class_name: "Light".to_string(),
                location: Vec3::new(cent_pos.x, cent_pos.y, cent_pos.z + 120.0),
                group: "Lighting".to_string(),
                ..Default::default()
            };
            c_light.byte_props.push(("LightType".to_string(), 1));
            c_light.byte_props.push(("LightEffect".to_string(), 13)); // LE_NonIncidence
            c_light.byte_props.push(("LightBrightness".to_string(), 180));
            c_light.byte_props.push(("LightHue".to_string(), 40));
            c_light.byte_props.push(("LightSaturation".to_string(), 120));
            c_light.byte_props.push(("LightRadius".to_string(), 32));
            c_light.bool_props.push(("bCorona".to_string(), true));
            c_light.str_props.push(("Corona".to_string(), "GenFX.Lensflare".to_string()));
            c_light.float_props.push(("DrawScale".to_string(), 0.45));
            actors.push(c_light);
        }
    }
}

/// Materializes an Assault mission plan: `FortStandard` objectives (destroy / switch / zone / chain),
/// the sealing doors (`Mover`) that each completed objective opens, switch consoles with button movers,
/// `AssaultInfo` briefing, end camera, `DefensePoint`s and AI nav nodes, mirroring the structure of the
/// official AS-* maps (see `assault_mission.rs`).
#[allow(clippy::too_many_arguments)]
fn build_assault_mission_content(
    plan: &AssaultMissionPlan,
    nodes: &[RoomNode],
    pal: &WizardResolvedPalette,
    cfg: &MapWizardConfig,
    actors: &mut Vec<WizardActorSpec>,
    movers: &mut Vec<WizardMoverSpec>,
    add_structural_brushes: &mut Vec<Brush>,
    semisolid_detail_brushes: &mut Vec<Brush>,
) {
    let n_obj = plan.objectives.len();
    let start = &nodes[plan.start_node];

    // Briefing (AssaultInfo.ObjDesc[i]) + spectator camera at the attacker start
    let mut info = WizardActorSpec {
        name: "AssaultInfo0".to_string(),
        class_name: "AssaultInfo".to_string(),
        location: Vec3::new(start.center.x, start.center.y, start.floor_z() + 64.0),
        group: "Objectives".to_string(),
        ..Default::default()
    };
    for (i, line) in briefing_lines(plan).into_iter().enumerate() {
        let key = if i == 0 {
            "ObjDesc".to_string()
        } else {
            format!("ObjDesc[{i}]")
        };
        info.str_props.push((key, line));
    }
    actors.push(info);
    actors.push(WizardActorSpec {
        name: "SpectatorCam0".to_string(),
        class_name: "SpectatorCam".to_string(),
        location: Vec3::new(start.center.x, start.center.y, start.ceiling_z() - 96.0),
        rotation: (-2048, 0, 0),
        tag: "AS_StartCam".to_string(),
        group: "Objectives".to_string(),
        ..Default::default()
    });

    let textured = |mut b: Brush, group: &str| -> Brush {
        b.group = Some(group.to_string());
        for p in &mut b.polys {
            p.texture = pal.trim_texture.clone();
        }
        b.recompute_uvs();
        b
    };

    for o in &plan.objectives {
        let r = &nodes[o.node_idx];
        let fz = r.floor_z();
        let off_y = if r.has_hazard_pit && cfg.toggles.include_environmental_hazards {
            let pit_l = (r.size.y * 0.32).clamp(192.0, 352.0);
            (pit_l * 0.5 + 96.0).min(r.size.y * 0.38)
        } else {
            (r.size.y * 0.20).min(160.0)
        };
        let base = Vec3::new(
            r.center.x,
            r.center.y + off_y,
            fz,
        );

        let completion_event = o.gate_tag.clone().unwrap_or_default();
        let nav_tag = format!("AS_Nav{}", o.index);
        let priority = (((n_obj - o.index) * 10) as u8).max(1);
        let health = if o.is_final { 50 } else { 10 + 5 * o.index as i32 };

        let make_fort = |name: String,
                         tag: String,
                         event: String,
                         pos: Vec3,
                         trigger_only: bool,
                         radius: f32,
                         height: f32|
         -> WizardActorSpec {
            let mut fs = WizardActorSpec {
                name,
                class_name: "FortStandard".to_string(),
                location: pos,
                tag,
                event,
                group: "Objectives".to_string(),
                ..Default::default()
            };
            fs.str_props
                .push(("FortName".to_string(), o.fort_name.clone()));
            fs.str_props
                .push(("DestroyedMessage".to_string(), o.destroyed_message.clone()));
            fs.byte_props
                .push(("DefensePriority".to_string(), priority));
            fs.int_props.push(("DefenseTime".to_string(), 6));
            fs.float_props
                .push(("CollisionRadius".to_string(), radius));
            fs.float_props
                .push(("CollisionHeight".to_string(), height));
            fs.name_props
                .push(("NearestPathNodeTag".to_string(), nav_tag.clone()));
            if trigger_only {
                fs.bool_props.push(("bTriggerOnly".to_string(), true));
            } else {
                fs.int_props.push(("Health".to_string(), health));
            }
            if o.is_final {
                fs.bool_props.push(("bFinalFort".to_string(), true));
                fs.name_props
                    .push(("EndCamTag".to_string(), "AS_EndCam".to_string()));
            }
            fs
        };

        let pad = |name: String, size: Vec3, at: Vec3| -> Brush {
            let mut b = build_cube(name, CsgOperation::Add, size);
            b.location = at;
            for p in &mut b.polys {
                p.flags |= PF_SEMISOLID;
            }
            textured(b, "Objectives")
        };

        let fort_pos = base + Vec3::new(0.0, 0.0, 56.0);
        match o.kind {
            AssaultObjectiveKind::Destroy => {
                semisolid_detail_brushes.push(pad(
                    format!("AS_TargetPad{}", o.index),
                    Vec3::new(96.0, 96.0, 8.0),
                    base + Vec3::new(0.0, 0.0, 4.0),
                ));
                actors.push(make_fort(
                    format!("FortStandard{}", o.index),
                    o.fort_tag.clone(),
                    completion_event.clone(),
                    fort_pos,
                    false,
                    32.0,
                    64.0,
                ));
            }
            AssaultObjectiveKind::Chain => {
                let cnt_tag = format!("AS_Cnt{}", o.index);
                for (k, dx) in [-80.0_f32, 80.0].into_iter().enumerate() {
                    semisolid_detail_brushes.push(pad(
                        format!("AS_ChainPad{}_{}", o.index, k),
                        Vec3::new(64.0, 64.0, 8.0),
                        base + Vec3::new(dx, 0.0, 4.0),
                    ));
                    let tag = if k == 0 {
                        o.fort_tag.clone()
                    } else {
                        format!("{}B", o.fort_tag)
                    };
                    actors.push(make_fort(
                        format!("FortStandard{}_{}", o.index, k),
                        tag,
                        cnt_tag.clone(),
                        base + Vec3::new(dx, 0.0, 56.0),
                        false,
                        28.0,
                        56.0,
                    ));
                }
                let mut counter = WizardActorSpec {
                    name: format!("Counter{}", o.index),
                    class_name: "Counter".to_string(),
                    location: base + Vec3::new(0.0, 0.0, 120.0),
                    tag: cnt_tag,
                    event: completion_event.clone(),
                    group: "Objectives".to_string(),
                    ..Default::default()
                };
                counter.int_props.push(("NumToCount".to_string(), 2));
                actors.push(counter);
            }
            AssaultObjectiveKind::TouchZone => {
                semisolid_detail_brushes.push(pad(
                    format!("AS_ZonePad{}", o.index),
                    Vec3::new(192.0, 192.0, 4.0),
                    base + Vec3::new(0.0, 0.0, 2.0),
                ));
                actors.push(make_fort(
                    format!("FortStandard{}", o.index),
                    o.fort_tag.clone(),
                    completion_event.clone(),
                    fort_pos,
                    true,
                    192.0,
                    96.0,
                ));
                let mut trig = WizardActorSpec {
                    name: format!("AS_ZoneTrigger{}", o.index),
                    class_name: "Trigger".to_string(),
                    location: base + Vec3::new(0.0, 0.0, 48.0),
                    event: o.fort_tag.clone(),
                    group: "Objectives".to_string(),
                    ..Default::default()
                };
                trig.float_props.push(("CollisionRadius".to_string(), 192.0));
                trig.float_props.push(("CollisionHeight".to_string(), 96.0));
                trig.bool_props.push(("bTriggerOnceOnly".to_string(), true));
                actors.push(trig);
            }
            AssaultObjectiveKind::SwitchTouch | AssaultObjectiveKind::SwitchShoot => {
                // Console block with a pressable button mover on its -Y face
                let mut console = build_cube(
                    format!("AS_Console{}", o.index),
                    CsgOperation::Add,
                    Vec3::new(64.0, 48.0, 96.0),
                );
                console.location = base + Vec3::new(0.0, 0.0, 48.0);
                add_structural_brushes.push(textured(console, "Objectives"));

                let btn_tag = format!("AS_Btn{}", o.index);
                let btn_origin = base + Vec3::new(0.0, -28.0, 56.0);
                let mut btn_brush = build_cube(
                    format!("AS_ButtonBrush{}", o.index),
                    CsgOperation::Active,
                    Vec3::new(32.0, 8.0, 32.0),
                );
                btn_brush.location = btn_origin;
                let btn_brush = textured(btn_brush, "Objectives");
                movers.push(WizardMoverSpec {
                    name: format!("ASButtonMover{}", o.index),
                    brush: btn_brush,
                    key0_location: btn_origin,
                    key1_location: btn_origin + Vec3::new(0.0, 8.0, 0.0),
                    initial_state: "TriggerOpenTimed".to_string(),
                    move_time: 0.4,
                    stay_open_time: 600.0,
                    tag: btn_tag.clone(),
                    event: completion_event.clone(),
                    opening_sound: String::new(),
                    closed_sound: String::new(),
                });

                let stand = base + Vec3::new(0.0, -72.0, 56.0);
                actors.push(make_fort(
                    format!("FortStandard{}", o.index),
                    o.fort_tag.clone(),
                    btn_tag,
                    stand,
                    true,
                    32.0,
                    64.0,
                ));
                if o.kind == AssaultObjectiveKind::SwitchShoot {
                    let mut tt = WizardActorSpec {
                        name: format!("AS_ShootTrigger{}", o.index),
                        class_name: "TeamTrigger".to_string(),
                        location: btn_origin + Vec3::new(0.0, -12.0, 0.0),
                        event: o.fort_tag.clone(),
                        group: "Objectives".to_string(),
                        ..Default::default()
                    };
                    tt.byte_props.push(("TriggerType".to_string(), 4)); // TT_Shoot
                    tt.float_props.push(("DamageThreshold".to_string(), 1.0));
                    tt.float_props.push(("CollisionRadius".to_string(), 24.0));
                    tt.float_props.push(("CollisionHeight".to_string(), 24.0));
                    tt.bool_props.push(("bProjTarget".to_string(), true));
                    tt.bool_props.push(("bTriggerOnceOnly".to_string(), true));
                    actors.push(tt);
                } else {
                    let mut trig = WizardActorSpec {
                        name: format!("AS_TouchTrigger{}", o.index),
                        class_name: "Trigger".to_string(),
                        location: stand,
                        event: o.fort_tag.clone(),
                        group: "Objectives".to_string(),
                        ..Default::default()
                    };
                    trig.float_props.push(("CollisionRadius".to_string(), 96.0));
                    trig.float_props.push(("CollisionHeight".to_string(), 64.0));
                    trig.bool_props.push(("bTriggerOnceOnly".to_string(), true));
                    actors.push(trig);
                }
            }

        }

        // Defenders' positions bound to this objective by FortTag
        if o.kind == AssaultObjectiveKind::Chain {
            let sub_forts = [
                (o.fort_tag.clone(), -80.0_f32, "A"),
                (format!("{}B", o.fort_tag), 80.0_f32, "B"),
            ];
            for (sub_tag, center_x, suffix) in sub_forts {
                for (k, (dx, dy)) in [(-48.0_f32, -64.0_f32), (48.0, -64.0)]
                    .into_iter()
                    .enumerate()
                {
                    let mut dp = WizardActorSpec {
                        name: format!("DefensePoint{}_{}_{}", o.index, suffix, k),
                        class_name: "DefensePoint".to_string(),
                        location: base + Vec3::new(center_x + dx, dy, 40.0),
                        group: "Objectives".to_string(),
                        ..Default::default()
                    };
                    dp.name_props.push(("FortTag".to_string(), sub_tag.clone()));
                    dp.byte_props.push(("Priority".to_string(), priority));
                    actors.push(dp);
                }
            }
        } else {
            for (k, (dx, dy)) in [(-128.0_f32, -96.0_f32), (128.0, -96.0), (0.0, 128.0)]
                .into_iter()
                .enumerate()
            {
                let mut dp = WizardActorSpec {
                    name: format!("DefensePoint{}_{}", o.index, k),
                    class_name: "DefensePoint".to_string(),
                    location: base + Vec3::new(dx, dy, 40.0),
                    group: "Objectives".to_string(),
                    ..Default::default()
                };
                dp.name_props
                    .push(("FortTag".to_string(), o.fort_tag.clone()));
                dp.byte_props.push(("Priority".to_string(), priority));
                actors.push(dp);
            }
        }

        // AI nav node referenced by FortStandard.NearestPathNodeTag
        if cfg.toggles.include_bot_ai_paths {
            actors.push(WizardActorSpec {
                name: format!("ASPathNode{}", o.index),
                class_name: "PathNode".to_string(),
                location: base + Vec3::new(0.0, -48.0, 51.0),

                tag: nav_tag.clone(),
                group: "AI_Navigation".to_string(),
                ..Default::default()
            });
        }

        // Message shown when the objective opens the next area
        if let Some(gate_tag) = &o.gate_tag {
            let mut se = WizardActorSpec {
                name: format!("ASGateMessage{}", o.index),
                class_name: "SpecialEvent".to_string(),
                location: base + Vec3::new(0.0, 0.0, 160.0),
                tag: gate_tag.clone(),
                group: "Objectives".to_string(),
                ..Default::default()
            };
            se.name_props
                .push(("InitialState".to_string(), "DisplayMessage".to_string()));
            se.str_props.push((
                "Message".to_string(),
                format!("{} {} A new area is open!", o.fort_name, o.destroyed_message),
            ));
            se.bool_props.push(("bBroadcast".to_string(), true));
            actors.push(se);
        }

        if o.is_final {
            actors.push(WizardActorSpec {
                name: "AS_EndCam".to_string(),
                class_name: "SpectatorCam".to_string(),
                location: Vec3::new(r.center.x, r.center.y, (r.ceiling_z() - 96.0).max(fz + 160.0)),
                rotation: (-3072, 0, 0),
                tag: "AS_EndCam".to_string(),
                group: "Objectives".to_string(),
                ..Default::default()
            });
        }
    }

    // Sealing doors (Mover) for every gated objective
    for o in &plan.objectives {
        let (Some(gate_idx), Some(gate_tag)) = (o.gate_node, o.gate_tag.as_ref()) else {
            continue;
        };
        let c = &nodes[gate_idx];
        let along_x = c.size.x >= c.size.y;
        let door_size = if along_x {
            Vec3::new(32.0, c.size.y, c.size.z)
        } else {
            Vec3::new(c.size.x, 32.0, c.size.z)
        };
        let origin = Vec3::new(c.center.x, c.center.y, c.center.z);
        let mut brush = build_cube(
            format!("AS_GateDoorBrush{}", o.index),
            CsgOperation::Active,
            door_size,
        );
        brush.location = origin;
        let brush = textured(brush, "Objectives");
        movers.push(WizardMoverSpec {
            name: format!("ASGateMover{}", o.index),
            brush,
            key0_location: origin,
            key1_location: origin + Vec3::new(0.0, 0.0, c.size.z - 16.0),
            initial_state: "TriggerOpenTimed".to_string(),
            move_time: 1.8,
            stay_open_time: 600.0,
            tag: gate_tag.clone(),
            event: String::new(),
            opening_sound: "DoorsMod.General.mlift2sta".to_string(),
            closed_sound: "DoorsMod.General.mlift2end".to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{IssueSeverity, LevelDoctor};

    #[test]
    fn test_wizard_all_archetypes_and_modes_brush_validity() {
        for &mode in WizardGameMode::all() {
            for &arch in WizardArchetype::all() {
                for &shape in WizardRoomShape::all() {
                    let mut cfg = MapWizardConfig {
                        game_mode: mode,
                        layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
                        archetype: arch,
                        room_shape: shape,
                        ..Default::default()
                    };
                    cfg.sync_mode_conventions();
                    let generated = generate_valid_ut99_map(&cfg);
                    assert!(!generated.brushes.is_empty());
                    assert_eq!(generated.brushes[0].csg_oper, CsgOperation::Subtract);

                    let brush_issues = LevelDoctor::audit_brushes(&generated.brushes);
                    let errors: Vec<_> = brush_issues
                        .iter()
                        .filter(|i| i.severity == IssueSeverity::Error)
                        .collect();
                    assert!(
                        errors.is_empty(),
                        "Mode {:?} + Archetype {:?} + Shape {:?} produced brush errors: {:?}",
                        mode,
                        arch,
                        shape,
                        errors
                    );
                }
            }

        }
    }

    #[test]
    fn test_wizard_ramps_octagons_and_teleporters_generation() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::VerticalTowerShaft,
            room_shape: WizardRoomShape::Octagonal,
            ..Default::default()
        };
        cfg.toggles.include_ramps_and_bridges = true;
        cfg.toggles.include_teleporters = true;

        let map = generate_valid_ut99_map(&cfg);
        assert!(
            map.brushes.iter().any(|b| b.name == "OverwatchRamp" && b.polys.len() == 5),
            "Expected 5-face convex OverwatchRamp brush"
        );
        assert!(
            map.brushes.iter().any(|b| b.name == "FractalReactorShaft" && b.polys.len() == 10),
            "Expected 10-face octagonal FractalReactorShaft brush"
        );
        let tele_count = map
            .actors
            .iter()
            .filter(|a| a.class_name == "VisibleTeleporter")
            .count();
        assert_eq!(tele_count, 2, "Expected paired VisibleTeleporter0 and VisibleTeleporter1");
        assert!(
            map.brushes.iter().any(|b| b.name == "TeleporterArchTop_0"),
            "Expected TeleporterArchTop_0 portal frame brush"
        );
    }

    #[test]
    fn test_wizard_procedural_seed_determinism() {
        let cfg1 = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            seed: 424242,
            procedural_room_count: 6,
            symmetry_mode: WizardSymmetryMode::Rotational180,
            ..Default::default()
        };
        let map1 = generate_valid_ut99_map(&cfg1);
        let map2 = generate_valid_ut99_map(&cfg1);
        assert_eq!(map1.brushes.len(), map2.brushes.len());
        assert_eq!(map1.actors.len(), map2.actors.len());

        let mut cfg_diff = cfg1.clone();
        cfg_diff.seed = 999999;
        let map_diff = generate_valid_ut99_map(&cfg_diff);
        let rooms1: Vec<_> = map1.preview.rooms.iter().map(|r| (r.center, r.size)).collect();
        let rooms_diff: Vec<_> = map_diff.preview.rooms.iter().map(|r| (r.center, r.size)).collect();
        assert_ne!(rooms1, rooms_diff);
    }

    #[test]
    fn catwalk_stair_treads_have_headroom_and_reach_the_deck() {
        let room = RoomNode {
            name: "TestRoom".into(), center: Vec3::new(0.0, 0.0, 384.0),
            size: Vec3::new(1536.0, 1536.0, 768.0), is_corridor: false,
            sky_ceiling: false, has_hazard_pit: false, team_owner: None, is_octagonal: false,
        };
        let cfg = MapWizardConfig::default();
        let mut brushes = Vec::new();
        let mut actors = Vec::new();
        generate_room_catwalks(&[room], &cfg, &cfg.active_palette(), &mut brushes, &mut actors);
        let deck = brushes.iter().find(|b| b.name == "TestRoom_CatwalkDeck").unwrap();
        let stairs = brushes.iter().find(|b| b.name == "TestRoom_CatwalkStairs").unwrap();
        eprintln!("[DEBUG-stair] {:?}", LevelDoctor::audit_builder_brush(stairs));
        let (deck_min, deck_max) = deck.world_bounds();
        let (_, stairs_max) = stairs.world_bounds();
        assert!((stairs_max.z - deck_max.z).abs() < 0.01, "stairs must reach the deck height");
        let mut treads = 0;
        for poly in &stairs.polys {
            if poly.plane.normal.z < 0.99 { continue; }
            treads += 1;
            let center = poly.vertices.iter().copied().sum::<Vec3>() / poly.vertices.len() as f32 + stairs.location;
            let under_deck = center.x > deck_min.x && center.x < deck_max.x
                && center.y > deck_min.y && center.y < deck_max.y
                && deck_min.z > center.z + 0.1 && deck_min.z < center.z + 78.0;
            assert!(!under_deck, "stair tread at {center:?} has deck overhead at {}", deck_min.z);
        }
        assert!(treads >= 12, "expected all stair treads");
    }

    #[test]
    fn transit_stairs_reach_higher_floor_and_lie_within_lower_room() {
        let mut cfg = MapWizardConfig {
            archetype: WizardArchetype::GothicCathedralNave, // Has nave (floor_z = 0) vs choir (floor_z = 128)
            architectural_detail: WizardArchitecturalDetail::StandardTournament,
            ..Default::default()
        };
        cfg.toggles.ensure_vertical_reachability = true;
        let map = generate_valid_ut99_map(&cfg);

        let stair_nave_amb = map.brushes.iter()
            .find(|b| b.name == "TransitStair_CathedralNave_AltarAmbulatory")
            .expect("TransitStair_CathedralNave_AltarAmbulatory should be generated");
        let (s1_min, s1_max) = stair_nave_amb.world_bounds();
        assert!((s1_min.z - 0.0).abs() < 1.0, "stair 1 bottom must start at nave floor (0), got {}", s1_min.z);
        assert!((s1_max.z - 64.0).abs() < 1.0, "stair 1 top must reach ambulatory floor (64), got {}", s1_max.z);

        let stair_amb_apse = map.brushes.iter()
            .find(|b| b.name == "TransitStair_AltarAmbulatory_ApseHighAltar")
            .expect("TransitStair_AltarAmbulatory_ApseHighAltar should be generated");
        let (s2_min, s2_max) = stair_amb_apse.world_bounds();
        assert!((s2_min.z - 64.0).abs() < 1.0, "stair 2 bottom must start at ambulatory floor (64), got {}", s2_min.z);
        assert!((s2_max.z - 128.0).abs() < 1.0, "stair 2 top must reach apse high altar floor (128), got {}", s2_max.z);

        // Risers must point in outward direction (towards the low floor)
        let riser_count = stair_nave_amb.polys.iter().filter(|p| {
            p.plane.normal.z.abs() < 0.1
        }).count();
        assert!(riser_count >= 4, "Expected step faces/risers");

        // Top tread must be at high floor level
        let max_tread_z = stair_amb_apse.polys.iter()
            .filter(|p| p.plane.normal.z > 0.9)
            .map(|p| p.vertices.iter().map(|v| v.z + stair_amb_apse.location.z).fold(f32::NEG_INFINITY, f32::max))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((max_tread_z - 128.0).abs() < 1.0, "Top tread must reach 128.0, got {}", max_tread_z);
    }

    #[test]
    fn preset_roundtrip_reproduces_complete_generated_map_across_modes() {
        let modes = [
            (WizardGameMode::DeathMatch, WizardArchetype::InterconnectedComplex),
            (WizardGameMode::Duel1v1, WizardArchetype::CrucibleDuelPit),
            (WizardGameMode::TeamDeathMatch, WizardArchetype::InterconnectedComplex),
            (WizardGameMode::CaptureTheFlag, WizardArchetype::SymmetricalTwinBases),
            (WizardGameMode::Domination, WizardArchetype::ThreeWayRingArena),
            (WizardGameMode::Assault, WizardArchetype::FortressSiegeLine),
        ];
        for (mode, archetype) in modes {
            for strategy in [WizardLayoutStrategy::CanonicalArchetype, WizardLayoutStrategy::ProceduralSeed] {
                let cfg = MapWizardConfig {
                    game_mode: mode, archetype, layout_strategy: strategy,
                    seed: 424242, map_name: format!("{}-Repro", mode.prefix()),
                    procedural_loop_ratio: 0.0, ..Default::default()
                };
                let text = serde_json::to_string(&cfg).unwrap();
                let restored: MapWizardConfig = serde_json::from_str(&text).unwrap();
                assert!(cfg == restored, "config changed after JSON round trip for {mode:?}/{strategy:?}");
                let original = generate_valid_ut99_map(&cfg);
                let repeated = generate_valid_ut99_map(&restored);
                assert!(original.brushes == repeated.brushes, "brushes differ for {mode:?}/{strategy:?}");
                assert!(original.movers == repeated.movers, "movers differ for {mode:?}/{strategy:?}");
                assert!(original.actors == repeated.actors, "actors differ for {mode:?}/{strategy:?}");
                assert!(original.preview == repeated.preview, "preview differs for {mode:?}/{strategy:?}");
                assert_eq!(original.warnings, repeated.warnings, "warnings differ for {mode:?}/{strategy:?}");
                assert_eq!((original.default_game_type, original.song, original.ambient_brightness, original.ambient_hue, original.ambient_saturation),
                    (repeated.default_game_type, repeated.song, repeated.ambient_brightness, repeated.ambient_hue, repeated.ambient_saturation));
            }
        }
    }

    #[test]
    fn test_wizard_custom_armory_and_powerup_loadouts() {
        // Test Heavy Explosives + Mayhem Overkill loadout
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            archetype: WizardArchetype::CentralArenaMultiTier,
            ..Default::default()
        };
        cfg.apply_weapon_preset(WizardWeaponPreset::HeavyExplosives);
        cfg.apply_powerup_preset(WizardPowerupPreset::MayhemOverkill);
        cfg.item_density = WizardItemDensity::Abundant;

        let map = generate_valid_ut99_map(&cfg);
        let actor_classes: Vec<String> = map.actors.iter().map(|a| a.class_name.clone()).collect();

        // Check weapon generation
        assert!(actor_classes.contains(&"WarHeadLauncher".to_string()));
        assert!(actor_classes.contains(&"WarHeadAmmo".to_string()));
        assert!(actor_classes.contains(&"UT_Eightball".to_string()));
        assert!(actor_classes.contains(&"RocketPack".to_string()));
        assert!(actor_classes.contains(&"UT_FlakCannon".to_string()));
        assert!(actor_classes.contains(&"FlakAmmo".to_string()));

        // Check powerup generation
        assert!(actor_classes.contains(&"UDamage".to_string()));
        assert!(actor_classes.contains(&"UT_ShieldBelt".to_string()));
        assert!(actor_classes.contains(&"HealthPack".to_string()));
        assert!(actor_classes.contains(&"UT_JumpBoots".to_string()));

        // Check Classic Unreal 1 loadout
        let mut cfg_classic = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            archetype: WizardArchetype::InterconnectedComplex,
            ..Default::default()
        };
        cfg_classic.apply_weapon_preset(WizardWeaponPreset::ClassicUnreal);
        let map_classic = generate_valid_ut99_map(&cfg_classic);
        let classic_classes: Vec<String> = map_classic.actors.iter().map(|a| a.class_name.clone()).collect();

        assert!(classic_classes.contains(&"ASMD".to_string()));
        assert!(classic_classes.contains(&"ASMDAmmo".to_string()));
        assert!(classic_classes.contains(&"Eightball".to_string()));
        assert!(classic_classes.contains(&"RocketCan".to_string()));
        assert!(classic_classes.contains(&"Stinger".to_string()));
        assert!(classic_classes.contains(&"StingerAmmo".to_string()));

        // Check Level Doctor brush integrity
        let brush_issues = LevelDoctor::audit_brushes(&map.brushes);
        let errors: Vec<_> = brush_issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "Heavy loadout produced brush errors: {:?}", errors);
    }

    #[test]
    fn test_ctf_symmetrical_weapons_and_pickups() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            archetype: WizardArchetype::SymmetricalTwinBases,
            symmetry_mode: WizardSymmetryMode::Rotational180,
            ..Default::default()
        };
        cfg.enforce_mode_invariants();

        let map = generate_valid_ut99_map(&cfg);
        let red_flag = map.actors.iter().find(|a| a.name.contains("Red") && a.class_name == "FlagBase").map(|a| a.location);
        let blue_flag = map.actors.iter().find(|a| a.name.contains("Blue") && a.class_name == "FlagBase").map(|a| a.location);
        let center = if let (Some(rf), Some(bf)) = (red_flag, blue_flag) {
            (rf + bf) * 0.5
        } else {
            Vec3::ZERO
        };
        let sym_mode = CtfSymmetryMode::Rotational180;

        let red_weapons: Vec<&WizardActorSpec> = map
            .actors
            .iter()
            .filter(|a| a.name.starts_with("Red_") || a.name.contains("_Red_"))
            .collect();
        let blue_weapons: Vec<&WizardActorSpec> = map
            .actors
            .iter()
            .filter(|a| a.name.starts_with("Blue_") || a.name.contains("_Blue_"))
            .collect();

        assert!(!red_weapons.is_empty(), "Red base must contain weapons");
        assert_eq!(
            red_weapons.len(),
            blue_weapons.len(),
            "Red ({}) and Blue ({}) weapon counts must match exactly",
            red_weapons.len(),
            blue_weapons.len()
        );

        // For each weapon in Red territory, verify an identical weapon exists in Blue territory at transform_point_symmetry
        for r_act in &red_weapons {
            let expected_loc = transform_point_symmetry(r_act.location, sym_mode, center);
            let matching_blue = blue_weapons.iter().find(|b_act| {
                b_act.class_name == r_act.class_name
                    && b_act.location.distance(expected_loc) < 2.0
            });
            assert!(
                matching_blue.is_some(),
                "Weapon {} at {:?} missing matching Blue symmetrical counterpart at {:?}",
                r_act.class_name,
                r_act.location,
                expected_loc
            );
        }

        // Verify weapon pads symmetry
        let red_pads: Vec<&Brush> = map
            .brushes
            .iter()
            .filter(|b| b.name.contains("Red_") && b.name.starts_with("WeaponPad_"))
            .collect();
        let blue_pads: Vec<&Brush> = map
            .brushes
            .iter()
            .filter(|b| b.name.contains("Blue_") && b.name.starts_with("WeaponPad_"))
            .collect();
        assert_eq!(red_pads.len(), blue_pads.len(), "Red and Blue WeaponPad counts must match");
    }

    #[test]
    fn test_wizard_spawns_100_percent_selected_weapons() {
        // Test with ALL 10 stock tournament weapons on a 3-room layout (SymmetricalTwinBases)
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            archetype: WizardArchetype::SymmetricalTwinBases,
            ..Default::default()
        };
        cfg.weapon_preset = WizardWeaponPreset::Custom;
        cfg.custom_weapon_loadout = STOCK_TOURNAMENT_WEAPONS
            .iter()
            .map(|w| w.class_name.to_string())
            .collect();
        assert_eq!(cfg.custom_weapon_loadout.len(), STOCK_TOURNAMENT_WEAPONS.len());

        let map = generate_valid_ut99_map(&cfg);
        let actor_classes: Vec<String> = map.actors.iter().map(|a| a.class_name.clone()).collect();

        // Verify every single one of the 10 selected weapons was generated
        for w in &cfg.custom_weapon_loadout {
            assert!(
                actor_classes.contains(w),
                "Map failed to spawn selected weapon: {w}"
            );
        }

        // Test with Classic Unreal 1 weapons (19 weapons total!)
        let mut all_19_weapons = cfg.custom_weapon_loadout.clone();
        for cw in CLASSIC_UNREAL_WEAPONS {
            all_19_weapons.push(cw.class_name.to_string());
        }
        cfg.custom_weapon_loadout = all_19_weapons.clone();
        let map_19 = generate_valid_ut99_map(&cfg);
        let actor_classes_19: Vec<String> = map_19.actors.iter().map(|a| a.class_name.clone()).collect();

        for w in &all_19_weapons {
            assert!(
                actor_classes_19.contains(w),
                "Map failed to spawn weapon in 19-weapon loadout: {w}"
            );
        }
    }

    #[test]
    fn test_wizard_abundant_density_doubles_arsenal() {
        let mut cfg_std = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            archetype: WizardArchetype::CentralArenaMultiTier,
            item_density: WizardItemDensity::Standard,
            ..Default::default()
        };
        cfg_std.apply_weapon_preset(WizardWeaponPreset::TournamentStandard);
        let map_std = generate_valid_ut99_map(&cfg_std);

        let mut cfg_abundant = cfg_std.clone();
        cfg_abundant.item_density = WizardItemDensity::Abundant;
        let map_abundant = generate_valid_ut99_map(&cfg_abundant);

        let count_weapons = |map: &WizardGeneratedMap| -> usize {
            map.actors
                .iter()
                .filter(|a| STOCK_TOURNAMENT_WEAPONS.iter().any(|w| w.class_name == a.class_name))
                .count()
        };
        let count_ammo = |map: &WizardGeneratedMap| -> usize {
            map.actors
                .iter()
                .filter(|a| STOCK_TOURNAMENT_WEAPONS.iter().any(|w| w.ammo_class == Some(a.class_name.as_str())))
                .count()
        };

        let std_weapons = count_weapons(&map_std);
        let abundant_weapons = count_weapons(&map_abundant);
        let std_ammo = count_ammo(&map_std);
        let abundant_ammo = count_ammo(&map_abundant);

        assert!(
            abundant_weapons >= (std_weapons as f32 * 1.8) as usize,
            "Abundant density must double weapons: standard {std_weapons} vs abundant {abundant_weapons}"
        );
        assert!(
            abundant_ammo >= (std_ammo as f32 * 1.8) as usize,
            "Abundant density must double ammo: standard {std_ammo} vs abundant {abundant_ammo}"
        );
    }

    #[test]
    fn test_ctf_symmetrical_powerups() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            archetype: WizardArchetype::SymmetricalTwinBases,
            powerup_preset: WizardPowerupPreset::Custom,
            custom_powerup_loadout: vec![
                "UT_ShieldBelt".to_string(),
                "UDamage".to_string(),
                "HealthPack".to_string(),
                "UT_JumpBoots".to_string(),
                "UT_Invisibility".to_string(),
                "Armor2".to_string(),
                "ThighPads".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
            ..Default::default()
        };
        cfg.enforce_mode_invariants();

        let map = generate_valid_ut99_map(&cfg);
        let actor_classes: Vec<String> = map.actors.iter().map(|a| a.class_name.clone()).collect();

        // 1. Verify ALL selected powerups are spawned
        for p in &cfg.custom_powerup_loadout {
            assert!(
                actor_classes.contains(p),
                "CTF map failed to spawn selected powerup: {p}"
            );
        }

        // 2. Verify base powerups exist symmetrically for both Red and Blue
        for team_p in ["Armor2", "UT_JumpBoots", "UT_Invisibility"] {
            let red_count = map.actors.iter().filter(|a| a.class_name == team_p && a.name.contains("Red")).count();
            let blue_count = map.actors.iter().filter(|a| a.class_name == team_p && a.name.contains("Blue")).count();
            assert_eq!(
                red_count, blue_count,
                "Powerup {team_p} must be symmetric: {red_count} Red vs {blue_count} Blue"
            );
            assert!(red_count >= 1, "Must have at least 1 {team_p} per team");
        }
    }

    #[test]
    fn test_map_name_generator_uniqueness_and_plausibility() {
        assert_eq!(
            WizardArchetype::all().len(),
            40,
            "Catalog must expose 40 predefined architectural layouts"
        );
        let mut unique_names = std::collections::HashSet::new();
        let mut saw_two_word = false;
        let mut saw_three_word = false;
        let mut saw_visceral_verb = false;
        let mut saw_anatomical_noun = false;
        let mut saw_celestial_body = false;

        for i in 0..1000 {
            let mode = WizardGameMode::all()[i % WizardGameMode::all().len()];
            let id = generate_random_map_identity(mode, next_wizard_entropy_seed());
            assert!(
                id.map_name.starts_with(mode.prefix()),
                "Generated map_name '{}' must start with mode prefix '{}'",
                id.map_name,
                mode.prefix()
            );
            assert!(
                !id.map_name.contains(' '),
                "Map filename '{}' must not contain spaces",
                id.map_name
            );
            assert!(
                (9..=28).contains(&id.map_name.len()),
                "Map filename '{}' length {} must stay compact (9..=28 chars)",
                id.map_name,
                id.map_name.len()
            );
            let words: Vec<&str> = id.title.split_whitespace().collect();
            let word_count = words.len();
            assert!(
                (2..=3).contains(&word_count),
                "Title '{}' should consist of 2 or 3 words, got {}",
                id.title,
                word_count
            );
            if word_count == 2 {
                saw_two_word = true;
            } else if word_count == 3 {
                saw_three_word = true;
            }

            for w in &words {
                if VERBS_VISCERAL_PARTICIPLE.contains(w) {
                    saw_visceral_verb = true;
                }
                if NOUNS_ANATOMICAL_MEDICAL.contains(w) {
                    saw_anatomical_noun = true;
                }
                if CELESTIAL_BODIES.contains(w) {
                    saw_celestial_body = true;
                }
            }

            unique_names.insert(id.map_name);
        }

        assert!(
            unique_names.len() >= 930,
            "Expected >= 930 unique names across 1,000 samples, got {}",
            unique_names.len()
        );
        assert!(saw_two_word, "Expected 2-word patterns (e.g. DM-BeautifulDisaster)");
        assert!(saw_three_word, "Expected 3-word triad patterns (e.g. CTF-TwinSunSpire)");
        assert!(
            saw_visceral_verb,
            "Expected visceral participle verbs (e.g. Bleeding, Pulsing, Screaming, Burning)"
        );
        assert!(
            saw_anatomical_noun,
            "Expected scientific/anatomical nouns (e.g. Urethra, Gluteus, Anus, Ventricle)"
        );
        assert!(
            saw_celestial_body,
            "Expected celestial bodies / planets (e.g. Jupiter, Titan, Europa, Pluto)"
        );

        // Default config invocations must always yield distinct names
        let cfg_a = MapWizardConfig::default();
        let cfg_b = MapWizardConfig::default();
        assert_ne!(
            cfg_a.map_name, cfg_b.map_name,
            "Consecutive MapWizardConfig::default() calls must generate different map names"
        );
    }

    #[test]
    fn test_preview_sample_generated_names() {
        println!("\n=== GENERAL POOL SAMPLES ===");
        for (i, mode) in WizardGameMode::all().iter().cycle().take(15).enumerate() {
            let id = generate_random_map_identity(*mode, (i as u64 + 42) * 987654321);
            println!("SAMPLE #{:02}: {:<24} | Title: {:<24} | Enter: {}", i + 1, id.map_name, id.title, id.enter_text);
        }

        println!("\n=== VISCERAL & ANATOMICAL / CELESTIAL SPECIFIC SAMPLING ===");
        let mut count = 0;
        for seed in 1000..5000 {
            let id = generate_random_map_identity(WizardGameMode::DeathMatch, seed);
            let has_visceral = VERBS_VISCERAL_PARTICIPLE.iter().any(|v| id.title.contains(v));
            let has_anat = NOUNS_ANATOMICAL_MEDICAL.iter().any(|a| id.title.contains(a));
            let has_celestial = CELESTIAL_BODIES.iter().any(|c| id.title.contains(c));
            if (has_visceral && has_anat) || (has_visceral && has_celestial) || (has_anat && id.title.split_whitespace().count() == 2) {
                println!("FOCUSED #{:02}: {:<24} | Title: {:<24} | Enter: {}", count + 1, id.map_name, id.title, id.enter_text);
                count += 1;
                if count >= 10 {
                    break;
                }
            }
        }
    }

    #[test]
    fn test_wizard_randomize_all_invariants_and_variety() {
        let mut cfg = MapWizardConfig::default();
        let mut seen_modes = std::collections::HashSet::new();
        let mut seen_archetypes = std::collections::HashSet::new();
        let mut seen_themes = std::collections::HashSet::new();
        let mut seen_shapes = std::collections::HashSet::new();
        let mut seen_scales = std::collections::HashSet::new();
        let mut seen_densities = std::collections::HashSet::new();
        let mut seen_custom_weapons = false;
        let mut seen_custom_powerups = false;

        for _ in 0..100 {
            cfg.randomize_all(false);

            // Core playability invariants must always hold
            assert!(cfg.toggles.include_lighting);
            assert!(cfg.toggles.include_bot_ai_paths);
            assert!(cfg.toggles.include_weapons_and_pickups);

            if cfg.game_mode == WizardGameMode::CaptureTheFlag {
                assert_ne!(
                    cfg.symmetry_mode,
                    WizardSymmetryMode::None,
                    "CTF randomization must preserve MirrorX or Rotational180 symmetry"
                );
            }
            if cfg.layout_strategy == WizardLayoutStrategy::CanonicalArchetype
                && cfg.archetype == WizardArchetype::OrbitalOutpostSkybox
            {
                assert!(cfg.toggles.include_skybox);
            }

            let weapons = cfg.active_weapons();
            assert!(
                weapons.len() >= 3,
                "Randomized weapon loadout must never be empty or tiny: {:?}",
                weapons
            );

            let powerups = cfg.active_powerups();
            assert!(
                powerups.len() >= 3,
                "Randomized powerup loadout must never be empty: {:?}",
                powerups
            );
            assert!(
                powerups.iter().any(|p| p == "MedBox" || p == "HealthVial"),
                "Randomized powerups must always include baseline health: {:?}",
                powerups
            );
            assert!(
                powerups.iter().any(|p| p == "Armor2" || p == "ThighPads"),
                "Randomized powerups must always include baseline armor: {:?}",
                powerups
            );

            if cfg.weapon_preset == WizardWeaponPreset::Custom {
                seen_custom_weapons = true;
            }
            if cfg.powerup_preset == WizardPowerupPreset::Custom {
                seen_custom_powerups = true;
            }

            seen_modes.insert(cfg.game_mode);
            seen_archetypes.insert(cfg.archetype);
            seen_themes.insert(cfg.theme);
            seen_shapes.insert(cfg.room_shape);
            seen_scales.insert(cfg.scale);
            seen_densities.insert(cfg.item_density);
        }

        assert_eq!(seen_modes.len(), WizardGameMode::all().len());
        assert!(seen_archetypes.len() >= 12);
        assert_eq!(seen_themes.len(), WizardThemePreset::all().len());
        assert_eq!(seen_shapes.len(), WizardRoomShape::all().len());
        assert_eq!(seen_scales.len(), WizardMapScale::all().len());
        assert_eq!(seen_densities.len(), WizardItemDensity::all().len());
        assert!(seen_custom_weapons);
        assert!(seen_custom_powerups);

        // Verify keep_game_mode = true locks the Game Mode while randomizing everything else
        cfg.game_mode = WizardGameMode::CaptureTheFlag;
        for _ in 0..25 {
            let prev_name = cfg.map_name.clone();
            cfg.randomize_all(true);
            assert_eq!(cfg.game_mode, WizardGameMode::CaptureTheFlag);
            assert!(cfg.map_name.starts_with("CTF-"));
            assert_ne!(cfg.map_name, prev_name);
        }
    }

    #[test]
    fn test_assault_mission_full_generation_and_audit() {
        for &arch in &[
            WizardArchetype::FortressSiegeLine,
            WizardArchetype::GatehouseFuseAssault,
            WizardArchetype::LinearInfiltrationVessel,
            WizardArchetype::CastleKeepWinch,
            WizardArchetype::CanyonSerpentineRun,
        ] {
            let mut cfg = MapWizardConfig {
                game_mode: WizardGameMode::Assault,
                layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
                archetype: arch,
                assault_objective_count: 4,
                ..Default::default()
            };
            cfg.sync_mode_conventions();
            let map = generate_valid_ut99_map(&cfg);

            // 1. Exactly 1 AssaultInfo with ObjDesc matching objectives
            let info = map.actors.iter().find(|a| a.class_name == "AssaultInfo");
            assert!(info.is_some(), "AssaultInfo missing for {:?}", arch);
            let info = info.unwrap();
            let obj_desc_count = info
                .str_props
                .iter()
                .filter(|(k, _)| k.starts_with("ObjDesc"))
                .count();
            assert!(
                obj_desc_count >= 2,
                "Expected multiple ObjDesc lines, got {}",
                obj_desc_count
            );

            // 2. Spectator cameras
            assert!(map
                .actors
                .iter()
                .any(|a| a.class_name == "SpectatorCam" && a.tag == "AS_StartCam"));
            assert!(map
                .actors
                .iter()
                .any(|a| a.class_name == "SpectatorCam" && a.tag == "AS_EndCam"));

            // 3. FortStandards and bFinalFort
            let forts: Vec<_> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "FortStandard")
                .collect();
            assert!(forts.len() >= 2, "Expected >= 2 FortStandards, got {}", forts.len());
            let final_forts: Vec<_> = forts
                .iter()
                .filter(|f| f.bool_props.iter().any(|(k, v)| k == "bFinalFort" && *v))
                .collect();
            assert_eq!(
                final_forts.len(),
                1,
                "Expected exactly 1 bFinalFort, got {}",
                final_forts.len()
            );

            // 4. DefensePoints linked to FortStandards
            let defense_points: Vec<_> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "DefensePoint" && a.group == "Objectives")
                .collect();
            assert!(
                defense_points.len() >= forts.len() * 2,
                "Expected >= 2 DefensePoints per fort, got {} DPs and {} forts for arch {:?}",
                defense_points.len(),
                forts.len(),
                arch
            );
            for dp in &defense_points {
                let fort_tag = dp
                    .name_props
                    .iter()
                    .find(|(k, _)| k == "FortTag")
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("");
                assert!(!fort_tag.is_empty(), "DefensePoint missing FortTag");
                assert!(
                    forts.iter().any(|f| f.tag == fort_tag),
                    "DefensePoint FortTag '{}' not found in forts",
                    fort_tag
                );
            }

            // 5. Audit LevelDoctor: 0 brush errors
            let brush_issues = LevelDoctor::audit_brushes(&map.brushes);
            let errors: Vec<_> = brush_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            assert!(errors.is_empty(), "Brush errors in {:?}: {:?}", arch, errors);

            // 6. Audit LevelDoctor: event/tag link integrity
            let link_actors: Vec<_> = map
                .actors
                .iter()
                .map(|a| {
                    (
                        a.name.clone(),
                        a.class_name.clone(),
                        a.tag.clone(),
                        a.event.clone(),
                        a.location,
                    )
                })
                .collect();
            let link_issues = LevelDoctor::audit_actor_event_tag_links(&link_actors);
            let link_errors: Vec<_> = link_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            assert!(
                link_errors.is_empty(),
                "Event/tag link errors in {:?}: {:?}",
                arch,
                link_errors
            );
        }
    }

    #[test]
    fn test_ctf_mission_full_generation_and_audit() {
        let test_archetypes_and_dais = [
            (WizardArchetype::HourglassNexusCTF, crate::ctf_mission::CtfDaisStyle::TechBunkerAlcove),
            (WizardArchetype::FacingWorldsOrbitalSpire, crate::ctf_mission::CtfDaisStyle::SteppedTeamPedestal),
            (WizardArchetype::HydroSubmarineDock, crate::ctf_mission::CtfDaisStyle::AltarShrine),
            (WizardArchetype::CanyonCrestCitadels, crate::ctf_mission::CtfDaisStyle::PillarCanopy),
            (WizardArchetype::SymmetricalTwinBases, crate::ctf_mission::CtfDaisStyle::SteppedTeamPedestal),
            (WizardArchetype::ParallelTrenchWarfare, crate::ctf_mission::CtfDaisStyle::TechBunkerAlcove),
            (WizardArchetype::TwinSpireBridges, crate::ctf_mission::CtfDaisStyle::PillarCanopy),
        ];

        for &(arch, dais) in &test_archetypes_and_dais {
            let mut cfg = MapWizardConfig {
                game_mode: WizardGameMode::CaptureTheFlag,
                archetype: arch,
                ctf_dais_style: dais,
                ctf_include_sniper_perches: true,
                ctf_contested_center_powerup: true,
                ..Default::default()
            };
            cfg.enforce_mode_invariants();

            let map = generate_valid_ut99_map(&cfg);

            // 1. Check FlagBases
            let flag_bases: Vec<(String, u8, Vec3)> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "FlagBase")
                .map(|a| {
                    let team = a
                        .byte_props
                        .iter()
                        .find(|(k, _)| k == "Team")
                        .map(|(_, v)| *v)
                        .unwrap_or(255);
                    (a.name.clone(), team, a.location)
                })
                .collect();

            assert_eq!(
                flag_bases.len(),
                2,
                "CTF map for {:?} must have exactly 2 FlagBases, found {}",
                arch,
                flag_bases.len()
            );

            let red_flag = flag_bases.iter().find(|(_, t, _)| *t == 0).expect("Missing Red FlagBase");
            let blue_flag = flag_bases.iter().find(|(_, t, _)| *t == 1).expect("Missing Blue FlagBase");
            let flag_dist = red_flag.2.distance(blue_flag.2);
            assert!(
                flag_dist >= 2200.0,
                "CTF Flag distance in {:?} is {:.1} UU (must be >= 2200 UU)",
                arch,
                flag_dist
            );

            // 2. Check PlayerStarts
            let player_starts: Vec<(String, u8, Vec3)> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "PlayerStart")
                .map(|a| {
                    let team = a
                        .byte_props
                        .iter()
                        .find(|(k, _)| k == "TeamNumber")
                        .map(|(_, v)| *v)
                        .unwrap_or(255);
                    (a.name.clone(), team, a.location)
                })
                .collect();

            let red_spawns = player_starts.iter().filter(|(_, t, _)| *t == 0).count();
            let blue_spawns = player_starts.iter().filter(|(_, t, _)| *t == 1).count();
            assert!(
                red_spawns >= 4,
                "CTF {:?} must have >= 4 Red spawns, found {}",
                arch,
                red_spawns
            );
            assert!(
                blue_spawns >= 4,
                "CTF {:?} must have >= 4 Blue spawns, found {}",
                arch,
                blue_spawns
            );
            assert_eq!(
                red_spawns, blue_spawns,
                "CTF {:?} PlayerStart count asymmetric: {} Red vs {} Blue",
                arch, red_spawns, blue_spawns
            );

            // 3. Check DefensePoints
            let defense_points: Vec<(String, u8, bool, Vec3)> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "DefensePoint")
                .map(|a| {
                    let team = a
                        .byte_props
                        .iter()
                        .find(|(k, _)| k == "team")
                        .map(|(_, v)| *v)
                        .unwrap_or(255);
                    let is_sniping = a
                        .bool_props
                        .iter()
                        .find(|(k, _)| k == "bSniping")
                        .map(|(_, v)| *v)
                        .unwrap_or(false);
                    (a.name.clone(), team, is_sniping, a.location)
                })
                .collect();

            assert!(
                defense_points.iter().any(|(_, t, _, _)| *t == 0),
                "Missing Red DefensePoints in {:?}",
                arch
            );
            assert!(
                defense_points.iter().any(|(_, t, _, _)| *t == 1),
                "Missing Blue DefensePoints in {:?}",
                arch
            );
            assert!(
                defense_points.iter().any(|(_, _, is_s, _)| *is_s),
                "Missing Sniper DefensePoints in {:?}",
                arch
            );

            // 4. Check AlternatePaths
            let alternate_paths: Vec<(u8, f32, bool, Vec3)> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "AlternatePath")
                .map(|a| {
                    let team = a
                        .byte_props
                        .iter()
                        .find(|(k, _)| k == "team")
                        .map(|(_, v)| *v)
                        .unwrap_or(255);
                    let weight = a
                        .float_props
                        .iter()
                        .find(|(k, _)| k == "SelectionWeight")
                        .map(|(_, v)| *v)
                        .unwrap_or(1.0);
                    let ret_only = a
                        .bool_props
                        .iter()
                        .find(|(k, _)| k == "bReturnOnly")
                        .map(|(_, v)| *v)
                        .unwrap_or(false);
                    (team, weight, ret_only, a.location)
                })
                .collect();

            assert!(
                !alternate_paths.is_empty(),
                "CTF {:?} must generate AlternatePaths",
                arch
            );
            assert!(
                alternate_paths.iter().any(|(t, _, _, _)| *t == 0),
                "Missing Red AlternatePaths in {:?}",
                arch
            );
            assert!(
                alternate_paths.iter().any(|(t, _, _, _)| *t == 1),
                "Missing Blue AlternatePaths in {:?}",
                arch
            );

            // 5. Check Sniper Rifles
            let sniper_rifles: Vec<_> = map
                .actors
                .iter()
                .filter(|a| a.class_name == "SniperRifle")
                .collect();
            assert!(
                sniper_rifles.len() >= 2,
                "CTF {:?} with snipers enabled must have >= 2 SniperRifles, found {}",
                arch,
                sniper_rifles.len()
            );

            // 6. LevelDoctor Diagnostics Audit: 0 Errors
            let ctf_issues = LevelDoctor::audit_ctf_setup(
                &flag_bases,
                &player_starts,
                &alternate_paths,
                &defense_points,
            );
            let ctf_errors: Vec<_> = ctf_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            assert!(
                ctf_errors.is_empty(),
                "LevelDoctor CTF setup audit found errors in {:?}: {:?}",
                arch,
                ctf_errors
            );

            let brush_issues = LevelDoctor::audit_brushes(&map.brushes);
            let brush_errors: Vec<_> = brush_issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            assert!(
                brush_errors.is_empty(),
                "LevelDoctor brush audit found errors in {:?}: {:?}",
                arch,
                brush_errors
            );

            // 7. Verify zero FlagClearance encroachment against solid brushes
            let actors_for_audit: Vec<(String, String, Vec3)> = map
                .actors
                .iter()
                .map(|a| (a.name.clone(), a.class_name.clone(), a.location))
                .collect();
            let clearance_issues = LevelDoctor::audit_gameplay_clearance(
                &map.brushes,
                &crate::navigation::NavGraph::default(),
                &actors_for_audit,
            );
            let flag_clearance_errors: Vec<_> = clearance_issues
                .iter()
                .filter(|i| i.category == "FlagClearance")
                .collect();
            assert!(
                flag_clearance_errors.is_empty(),
                "Found FlagClearance encroachment in {:?} with dais {:?}: {:?}",
                arch,
                dais,
                flag_clearance_errors
            );
        }
    }

    #[test]
    fn test_facing_worlds_floors_and_ctf_teleporters() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            archetype: WizardArchetype::FacingWorldsOrbitalSpire,
            ..Default::default()
        };
        cfg.toggles.include_teleporters = true;

        let map = generate_valid_ut99_map(&cfg);

        // 1. Verify elevated floor slabs are generated for the sniper roosts
        assert!(
            map.brushes.iter().any(|b| b.name == "RedSniperRoost_FloorSlab" && b.csg_oper == CsgOperation::Add),
            "Expected additive solid floor slab brush for RedSniperRoost"
        );
        assert!(
            map.brushes.iter().any(|b| b.name == "BlueSniperRoost_FloorSlab" && b.csg_oper == CsgOperation::Add),
            "Expected additive solid floor slab brush for BlueSniperRoost"
        );

        // 2. Verify 4 teleporters are generated (2 intra-team pairs: Red Sanctum <-> Red Roost, Blue Sanctum <-> Blue Roost)
        let teleporters: Vec<_> = map.actors.iter().filter(|a| a.class_name == "VisibleTeleporter").collect();
        assert_eq!(teleporters.len(), 4, "Expected 4 intra-team teleporters for Facing Worlds");

        // 3. Verify teleporter pairings and URLs
        let red_base_tele = teleporters.iter().find(|t| t.tag == "Tele_RedBase").expect("Missing Tele_RedBase");
        let red_roost_tele = teleporters.iter().find(|t| t.tag == "Tele_RedRoost").expect("Missing Tele_RedRoost");
        let blue_base_tele = teleporters.iter().find(|t| t.tag == "Tele_BlueBase").expect("Missing Tele_BlueBase");
        let blue_roost_tele = teleporters.iter().find(|t| t.tag == "Tele_BlueRoost").expect("Missing Tele_BlueRoost");

        fn get_url<'a>(a: &'a WizardActorSpec) -> Option<&'a str> {
            a.str_props.iter().find(|(k, _)| k == "URL").map(|(_, v)| v.as_str())
        }
        assert_eq!(get_url(red_base_tele), Some("Tele_RedRoost"));
        assert_eq!(get_url(red_roost_tele), Some("Tele_RedBase"));
        assert_eq!(get_url(blue_base_tele), Some("Tele_BlueRoost"));
        assert_eq!(get_url(blue_roost_tele), Some("Tele_BlueBase"));

        // 4. Verify vertical separation: roost teleporters are elevated (Z > 600 UU)
        assert!(red_roost_tele.location.z >= 600.0);
        assert!(blue_roost_tele.location.z >= 600.0);

        // 5. Verify CTF-Face canonical standoff distance (>= 6000 UU)
        let flag_bases: Vec<_> = map.actors.iter().filter(|a| a.class_name == "FlagBase").collect();
        assert_eq!(flag_bases.len(), 2, "Expected 2 FlagBase actors");
        let flag_dist = flag_bases[0].location.distance(flag_bases[1].location);
        assert!(
            flag_dist >= 6000.0,
            "Facing Worlds flag standoff distance should be >= 6000 UU, got {:.1} UU",
            flag_dist
        );

        // 6. Verify Sniper Rifles perched high on the sniper roosts (Z >= 750 UU)
        let sniper_rifles: Vec<_> = map
            .actors
            .iter()
            .filter(|a| a.class_name == "SniperRifle" && a.location.z >= 750.0)
            .collect();
        assert!(
            sniper_rifles.len() >= 2,
            "Expected at least 2 sniper rifles on elevated roosts (Z >= 750), found {}",
            sniper_rifles.len()
        );

        // 7. Verify Contested Redeemer (WarHeadLauncher) at void crossing
        let redeemer = map.actors.iter().find(|a| a.class_name == "WarHeadLauncher");
        assert!(redeemer.is_some(), "Expected contested WarHeadLauncher in Facing Worlds");

        // 8. Verify multi-lane void routes: Upper bridge + Flanking void catwalks
        assert!(map.brushes.iter().any(|b| b.name == "ArchedSkyBridge_W"));
        assert!(map.brushes.iter().any(|b| b.name == "ArchedSkyBridge_E"));
        assert!(map.brushes.iter().any(|b| b.name == "NorthVoidCatwalk"));
        assert!(map.brushes.iter().any(|b| b.name == "SouthVoidCatwalk"));
        assert!(map.brushes.iter().any(|b| b.name == "RedCatwalkAccess_N"));
        assert!(map.brushes.iter().any(|b| b.name == "BlueCatwalkAccess_N"));
    }

    #[test]
    fn test_canyon_crest_citadels_symmetric_lavagiant_dna() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::CaptureTheFlag,
            archetype: WizardArchetype::CanyonCrestCitadels,
            theme: WizardThemePreset::VolcanicCaldera,
            ..Default::default()
        };
        cfg.toggles.include_weapons_and_pickups = true;
        cfg.toggles.include_environmental_hazards = true;
        cfg.toggles.include_bot_ai_paths = true;

        let map = generate_valid_ut99_map(&cfg);

        // 1. Verify exact 1:1 mathematical symmetry of FlagBases
        let flag_bases: Vec<(u8, Vec3)> = map
            .actors
            .iter()
            .filter(|a| a.class_name == "FlagBase")
            .map(|a| {
                let team = a.byte_props.iter().find(|(k, _)| k == "Team").map(|(_, v)| *v).unwrap_or(255);
                (team, a.location)
            })
            .collect();
        assert_eq!(flag_bases.len(), 2, "Expected exactly 2 FlagBases");
        let red_flag = flag_bases.iter().find(|(t, _)| *t == 0).expect("Missing Red FlagBase");
        let blue_flag = flag_bases.iter().find(|(t, _)| *t == 1).expect("Missing Blue FlagBase");

        // Perfect X-axis reflection symmetry
        assert_eq!(red_flag.1.x, -blue_flag.1.x, "FlagBase X coordinates must be perfectly symmetrical");
        assert_eq!(red_flag.1.y, blue_flag.1.y, "FlagBase Y coordinates must be aligned");
        assert_eq!(red_flag.1.z, blue_flag.1.z, "FlagBase Z coordinates must be identical");

        let flag_dist = red_flag.1.distance(blue_flag.1);
        assert!(flag_dist >= 6000.0, "Standoff distance must be >= 6000 UU (got {:.1} UU)", flag_dist);

        // 2. Verify elevated floor slabs for both sniper roosts
        assert!(map.brushes.iter().any(|b| b.name == "RedSniperOverlook_FloorSlab" && b.csg_oper == CsgOperation::Add));
        assert!(map.brushes.iter().any(|b| b.name == "BlueSniperOverlook_FloorSlab" && b.csg_oper == CsgOperation::Add));

        // 3. Verify Sniper Rifles perched high on roosts (Z >= 500)
        let sniper_rifles: Vec<_> = map
            .actors
            .iter()
            .filter(|a| a.class_name == "SniperRifle" && a.location.z >= 500.0)
            .collect();
        assert!(sniper_rifles.len() >= 2, "Expected at least 2 sniper rifles on elevated roosts");

        // 4. Verify Contested Redeemer (WarHeadLauncher) at exact center of canyon
        let redeemer = map.actors.iter().find(|a| a.name.contains("Redeemer") || a.class_name == "WarHeadLauncher").expect("Expected contested Redeemer");
        assert_eq!(redeemer.location.x, 0.0, "Redeemer must be centered on X=0");
        assert_eq!(redeemer.location.y, 0.0, "Redeemer must be centered on Y=0");

        // 5. Verify the 3 symmetrical canyon routes and access corridors
        assert!(map.brushes.iter().any(|b| b.name == "CanyonBridge_W"));
        assert!(map.brushes.iter().any(|b| b.name == "CanyonBridge_E"));
        assert!(map.brushes.iter().any(|b| b.name == "NorthCliffOverlook"));
        assert!(map.brushes.iter().any(|b| b.name == "SouthCavernRun"));
        assert!(map.brushes.iter().any(|b| b.name == "RedRidgeAccess"));
        assert!(map.brushes.iter().any(|b| b.name == "BlueRidgeAccess"));
        assert!(map.brushes.iter().any(|b| b.name == "RedGrottoAccess"));
        assert!(map.brushes.iter().any(|b| b.name == "BlueGrottoAccess"));

        // 6. LevelDoctor Playability & Brush Audit
        let brush_issues = LevelDoctor::audit_brushes(&map.brushes);
        let brush_errors: Vec<_> = brush_issues.iter().filter(|i| i.severity == IssueSeverity::Error).collect();
        assert!(brush_errors.is_empty(), "LevelDoctor brush errors: {:?}", brush_errors);

        // 7. Verify PlayerStarts are strictly on fortress ground floors, NEVER in sniper roosts
        let roost_spawns: Vec<_> = map
            .actors
            .iter()
            .filter(|a| a.class_name == "PlayerStart" && a.location.z >= 400.0)
            .collect();
        assert!(
            roost_spawns.is_empty(),
            "Zero PlayerStarts should be placed in elevated sniper roosts! Found: {:?}",
            roost_spawns
        );

        let spawns: Vec<_> = map
            .actors
            .iter()
            .filter(|a| a.class_name == "PlayerStart")
            .collect();
        assert_eq!(spawns.len(), 8, "Expected exactly 8 PlayerStarts (4 Red, 4 Blue)");
        for s in &spawns {
            assert!(
                s.location.x.abs() >= 2500.0,
                "PlayerStart {:?} must be inside base keep (X.abs >= 2500)",
                s.location
            );
            assert!(
                s.location.z < 100.0,
                "PlayerStart {:?} must be on ground level floor",
                s.location
            );
        }
    }

    #[test]
    fn test_ctf_teleporter_standoff_distance_from_flags() {
        for arch in [
            WizardArchetype::HourglassNexusCTF,
            WizardArchetype::SymmetricalTwinBases,
            WizardArchetype::VerticalTowerShaft,
        ] {
            let mut cfg = MapWizardConfig {
                game_mode: WizardGameMode::CaptureTheFlag,
                archetype: arch,
                ..Default::default()
            };
            cfg.toggles.include_teleporters = true;
            let map = generate_valid_ut99_map(&cfg);

            let flag_locs: Vec<Vec3> = map.actors.iter().filter(|a| a.class_name == "FlagBase").map(|a| a.location).collect();
            let tele_locs: Vec<Vec3> = map.actors.iter().filter(|a| a.class_name == "VisibleTeleporter").map(|a| a.location).collect();

            for t_loc in &tele_locs {
                for f_loc in &flag_locs {
                    let d = t_loc.distance(*f_loc);
                    assert!(
                        d >= 1000.0,
                        "Teleporter at {:?} in {:?} is too close to flag base at {:?} (dist: {:.1} UU)",
                        t_loc,
                        arch,
                        f_loc,
                        d
                    );
                }
            }
        }
    }

    #[test]
    fn test_wizard_custom_rooms_and_connections() {
        let mut cfg = MapWizardConfig::default();
        cfg.custom_rooms.push(WizardCustomRoom {
            id: "cr_1".to_string(),
            name: "SecretBunker".to_string(),
            center: Vec3::new(1280.0, 1280.0, 256.0),
            size: Vec3::new(640.0, 640.0, 512.0),
            is_octagonal: true,
            has_hazard_pit: false,
            sky_ceiling: false,
            team_owner: None,
            environment: WizardRoomEnvironment::Normal,
        });

        let topo = preview_wizard_topology(&cfg);
        assert!(topo.rooms.iter().any(|r| r.label == "SecretBunker"), "Custom room must be in topology preview");

        let map = generate_valid_ut99_map(&cfg);
        assert!(
            map.brushes.iter().any(|b| b.name == "SecretBunker" && b.csg_oper == CsgOperation::Subtract),
            "Custom room subtract brush must be generated"
        );
    }

    #[test]
    fn custom_connection_tracks_moved_endpoint() {
        let mut cfg = MapWizardConfig::default();
        for (name, x) in [("RoomA", 6000.0), ("RoomB", 8000.0)] {
            cfg.custom_rooms.push(WizardCustomRoom {
                id: name.into(), name: name.into(), center: Vec3::new(x, 0.0, 256.0),
                size: Vec3::new(640.0, 640.0, 512.0), is_octagonal: false,
                has_hazard_pit: false, sky_ceiling: false, team_owner: None,
                environment: WizardRoomEnvironment::Normal,
            });
        }
        cfg.custom_connections.push(WizardCustomConnection {
            id: "link".into(), from_room: "RoomA".into(), to_room: "RoomB".into(),
            width: 288.0, height: 320.0, has_blast_door: false,
        });
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: "RoomB".into(), center_offset: Vec3::new(10000.0, 0.0, 256.0),
            size: Vec3::new(640.0, 640.0, 512.0), environment: WizardRoomEnvironment::Normal,
        });
        let rooms = build_room_nodes(&cfg);
        let corridor = rooms.iter().find(|r| r.name == "CustomCorrX_link").unwrap();
        let moved = rooms.iter().find(|r| r.name == "RoomB").unwrap();
        assert!(boxes_overlap_xy(corridor, moved));
        assert_eq!(corridor.center.x, 8000.0);
        assert!(!preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("RoomB") && warning.contains("CustomCorrX_link")));
    }

    #[test]
    fn procedural_corridors_follow_edited_room_graph_endpoints() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            game_mode: WizardGameMode::DeathMatch, seed: 42,
            procedural_room_count: 6, ..Default::default()
        };
        let (baseline, graph) = build_room_nodes_with_graph(&cfg, true);
        let graph = graph.unwrap();
        let moved = baseline.iter().find(|room| room.name == "ProcChamber_1").unwrap();
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: moved.name.clone(), center_offset: moved.center + Vec3::new(8192.0, 4096.0, 0.0),
            size: moved.size, environment: WizardRoomEnvironment::Normal,
        });
        let updated = build_room_nodes(&cfg);
        let moved_room = updated.iter().find(|room| room.name == moved.name).unwrap();
        let mut checked = 0;
        for (index, (from, to)) in graph.edges.iter().enumerate() {
            if from != &moved.name && to != &moved.name {
                for axis in ['X', 'Y'] {
                    let name = format!("ProcCorridor{axis}_{index}");
                    let before = baseline.iter().find(|room| room.name == name);
                    let after = updated.iter().find(|room| room.name == name);
                    assert_eq!(before.map(|r| (r.center, r.size)), after.map(|r| (r.center, r.size)), "unaffected corridor {name} changed");
                }
                continue;
            }
            checked += 1;
            let names = [format!("ProcCorridorX_{index}"), format!("ProcCorridorY_{index}")];
            assert!(updated.iter().any(|room| names.contains(&room.name) && room_boxes_overlap(room, moved_room)),
                "edge {index} no longer reaches {}", moved.name);
        }
        assert!(checked > 0);
        let preview = preview_wizard_topology(&cfg);
        assert!(preview.warnings.iter().any(|warning| warning.contains("Procedural corridors were rebuilt")));
        let generated = generate_valid_ut99_map(&cfg);
        assert!(generated.warnings.iter().any(|warning| warning.contains("Procedural corridors were rebuilt")));
        for room in preview.rooms.iter().filter(|room| room.label.starts_with("ProcCorridor")) {
            let brush = generated.brushes.iter().find(|brush| brush.name == room.label && brush.csg_oper == CsgOperation::Subtract).unwrap();
            assert_eq!(room.center, brush.location, "preview and generated brush differ for {}", room.label);
        }
        cfg.room_overrides[0].center_offset.z += 512.0;
        assert!(preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("floor difference")), "large height changes must be reported");
    }

    #[test]
    fn unchanged_procedural_room_override_preserves_all_room_geometry() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            game_mode: WizardGameMode::DeathMatch, seed: 42,
            procedural_room_count: 6, ..Default::default()
        };
        let baseline = build_room_nodes(&cfg);
        let room = baseline.iter().find(|room| room.name == "ProcChamber_1").unwrap();
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: room.name.clone(), center_offset: room.center, size: room.size,
            environment: WizardRoomEnvironment::Normal,
        });
        let after = build_room_nodes(&cfg);
        assert_eq!(baseline.len(), after.len());
        for (old, new) in baseline.iter().zip(&after) {
            assert_eq!((&old.name, old.center, old.size), (&new.name, new.center, new.size));
        }
        assert!(!preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("Procedural corridors were rebuilt")));
    }

    #[test]
    fn procedural_room_edits_reconnect_across_modes_and_seeds() {
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag] {
            for seed in [3, 42, 91] {
                let mut cfg = MapWizardConfig {
                    layout_strategy: WizardLayoutStrategy::ProceduralSeed,
                    game_mode: mode, seed, procedural_room_count: 7,
                    procedural_vertical_tiers: 1, ..Default::default()
                };
                let (baseline, graph) = build_room_nodes_with_graph(&cfg, true);
                let graph = graph.unwrap();
                let name = if mode == WizardGameMode::CaptureTheFlag { "ProcWingRed_0" } else { "ProcChamber_1" };
                let room = baseline.iter().find(|room| room.name == name).unwrap();
                cfg.room_overrides.push(WizardRoomOverride {
                    room_name: name.into(), center_offset: room.center + Vec3::new(8192.0, 4096.0, 0.0),
                    size: room.size, environment: WizardRoomEnvironment::Normal,
                });
                let updated = build_room_nodes(&cfg);
                let moved = updated.iter().find(|room| room.name == name).unwrap();
                let mut checked = 0;
                for (index, (a, b)) in graph.edges.iter().enumerate() {
                    if a != name && b != name { continue; }
                    checked += 1;
                    let names = [format!("ProcCorridorX_{index}"), format!("ProcCorridorY_{index}")];
                    assert!(updated.iter().any(|room| names.contains(&room.name) && room_boxes_overlap(room, moved)),
                        "{mode:?}, seed {seed}: edge {index} no longer reaches {name}");
                }
                assert!(checked > 0);
                if mode == WizardGameMode::CaptureTheFlag {
                    assert!(preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("team symmetry")));
                }
            }
        }
    }

    #[test]
    fn procedural_rebuild_removes_obsolete_axis_segment() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            game_mode: WizardGameMode::DeathMatch, seed: 42,
            procedural_room_count: 6, procedural_vertical_tiers: 1, ..Default::default()
        };
        let (baseline, graph) = build_room_nodes_with_graph(&cfg, true);
        let graph = graph.unwrap();
        let (index, (from, to)) = graph.edges.iter().enumerate()
            .find(|(index, _)| baseline.iter().any(|room| room.name == format!("ProcCorridorX_{index}")))
            .unwrap();
        let a = baseline.iter().find(|room| room.name == *from).unwrap();
        let b = baseline.iter().find(|room| room.name == *to).unwrap();
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: a.name.clone(), center_offset: Vec3::new(b.center.x, b.center.y + 2048.0, a.center.z),
            size: a.size, environment: WizardRoomEnvironment::Normal,
        });
        let updated = build_room_nodes(&cfg);
        assert!(!updated.iter().any(|room| room.name == format!("ProcCorridorX_{index}")));
        let segment = updated.iter().find(|room| room.name == format!("ProcCorridorY_{index}")).unwrap();
        let moved = updated.iter().find(|room| room.name == *from).unwrap();
        let other = updated.iter().find(|room| room.name == *to).unwrap();
        assert!(room_boxes_overlap(segment, moved) && room_boxes_overlap(segment, other));
    }

    #[test]
    fn procedural_corridor_rebuild_preserves_explicit_corridor_and_custom_room() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            game_mode: WizardGameMode::DeathMatch, seed: 42,
            procedural_room_count: 6, ..Default::default()
        };
        let (baseline, graph) = build_room_nodes_with_graph(&cfg, true);
        let graph = graph.unwrap();
        let moved = baseline.iter().find(|room| room.name == "ProcChamber_1").unwrap();
        let edge = graph.edges.iter().enumerate().find(|(index, (a, b))| {
            (a == &moved.name || b == &moved.name) && baseline.iter().any(|r| r.name == format!("ProcCorridorX_{index}"))
        }).map(|(index, _)| index).unwrap();
        let name = format!("ProcCorridorX_{edge}");
        let corridor = baseline.iter().find(|room| room.name == name).unwrap();
        let explicit_center = corridor.center + Vec3::new(0.0, 512.0, 0.0);
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: moved.name.clone(), center_offset: moved.center + Vec3::new(8192.0, 4096.0, 0.0),
            size: moved.size, environment: WizardRoomEnvironment::Normal,
        });
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: name.clone(), center_offset: explicit_center, size: corridor.size,
            environment: WizardRoomEnvironment::Normal,
        });
        cfg.custom_rooms.push(WizardCustomRoom {
            id: "custom-collision".into(), name: name.clone(), center: Vec3::new(30000.0, 0.0, 256.0),
            size: Vec3::splat(512.0), is_octagonal: false, has_hazard_pit: false,
            sky_ceiling: false, team_owner: None, environment: WizardRoomEnvironment::Normal,
        });
        let updated = build_room_nodes(&cfg);
        assert_eq!(updated.iter().find(|r| r.name == name && r.is_corridor).unwrap().center, explicit_center);
        assert_eq!(updated.iter().find(|r| r.name == name && !r.is_corridor).unwrap().center, Vec3::new(30000.0, 0.0, 256.0));
        assert!(preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("corridor overrides were retained")));
    }

    #[test]
    #[ignore = "Known limitation: canonical corridors do not yet follow room overrides"]
    fn interconnected_main_corridor_follows_moved_wing() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::InterconnectedComplex,
            game_mode: WizardGameMode::DeathMatch, ..Default::default()
        };
        let baseline = build_room_nodes(&cfg);
        let wing = baseline.iter().find(|room| room.name == "WestTurbineHall").unwrap();
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: wing.name.clone(), center_offset: wing.center + Vec3::new(-4096.0, 2048.0, 0.0),
            size: wing.size, environment: WizardRoomEnvironment::Normal,
        });
        let updated = build_room_nodes(&cfg);
        let core = updated.iter().find(|room| room.name == "CentralVatHall").unwrap();
        let moved = updated.iter().find(|room| room.name == "WestTurbineHall").unwrap();
        let segments: Vec<_> = updated.iter().filter(|room| room.name == "WestLinkCorridor" || room.name == "WestLinkCorridor_AdaptiveTurn").collect();
        assert!(segments.iter().any(|room| room_boxes_overlap(room, core)), "connection must reach the core");
        assert!(segments.iter().any(|room| room_boxes_overlap(room, moved)), "connection must reach the moved wing");
        assert!(segments.len() == 2 && room_boxes_overlap(segments[0], segments[1]), "turn segments must meet");
        for name in ["EastLinkCorridor", "NorthMainCorridor", "NorthWestRingCorridor"] {
            let old = baseline.iter().find(|room| room.name == name).unwrap();
            let new = updated.iter().find(|room| room.name == name).unwrap();
            assert_eq!((old.center, old.size), (new.center, new.size), "unrelated {name} changed");
        }
    }

    #[test]
    fn room_override_reports_lost_archetype_contacts_in_preview_and_generation() {
        let mut cfg = MapWizardConfig {
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::CentralArenaMultiTier,
            ..Default::default()
        };
        let original = build_room_nodes(&cfg);
        let core = original.iter().find(|room| room.name == "ArenaCore").unwrap();
        let alcove = original.iter().find(|room| room.name == "ArenaAlcove_0").unwrap();
        assert!(room_boxes_overlap(core, alcove), "fixture must begin with a geometric contact");
        assert!(preview_wizard_topology(&cfg).warnings.is_empty());
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: alcove.name.clone(), center_offset: alcove.center,
            size: alcove.size, environment: WizardRoomEnvironment::Normal,
        });
        assert!(preview_wizard_topology(&cfg).warnings.is_empty(), "unchanged override must not warn");
        cfg.room_overrides[0].center_offset.x -= 10000.0;
        let preview = preview_wizard_topology(&cfg);
        assert!(preview.warnings.iter().any(|warning| warning.contains("ArenaCore") && warning.contains("ArenaAlcove_0")));
        let generated = generate_valid_ut99_map(&cfg);
        assert!(generated.warnings.iter().any(|warning| warning.contains("ArenaCore") && warning.contains("ArenaAlcove_0")));
        let preview_room = preview.rooms.iter().find(|room| room.label == "ArenaAlcove_0").unwrap();
        let generated_room = generated.brushes.iter().find(|brush| brush.name == "ArenaAlcove_0" && brush.csg_oper == CsgOperation::Subtract).unwrap();
        assert_eq!(preview_room.center, generated_room.location);
        cfg.room_overrides[0].center_offset = alcove.center;
        cfg.room_overrides[0].size.x = 128.0;
        assert!(preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("ArenaCore") && warning.contains("ArenaAlcove_0")), "resizing must also be checked");
    }

    #[test]
    fn preview_reports_missing_connection_endpoints_and_room_edits() {
        let mut cfg = MapWizardConfig::default();
        cfg.room_overrides.push(WizardRoomOverride {
            room_name: "MissingRoom".into(), center_offset: Vec3::ZERO,
            size: Vec3::splat(512.0), environment: WizardRoomEnvironment::Normal,
        });
        cfg.custom_connections.push(WizardCustomConnection {
            id: "broken".into(), from_room: "UnknownStart".into(), to_room: "UnknownEnd".into(),
            width: 256.0, height: 256.0, has_blast_door: false,
        });
        let warnings = preview_wizard_topology(&cfg).warnings;
        for label in ["MissingRoom", "UnknownStart", "UnknownEnd"] {
            assert!(warnings.iter().any(|warning| warning.contains(label)), "missing warning for {label}: {warnings:?}");
        }
        for id in ["one", "two"] {
            cfg.custom_rooms.push(WizardCustomRoom {
                id: id.into(), name: "DuplicateRoom".into(), center: Vec3::new(7000.0, 0.0, 256.0),
                size: Vec3::splat(512.0), is_octagonal: false, has_hazard_pit: false,
                sky_ceiling: false, team_owner: None, environment: WizardRoomEnvironment::Normal,
            });
        }
        cfg.custom_connections[0].from_room = "DuplicateRoom".into();
        assert!(preview_wizard_topology(&cfg).warnings.iter().any(|warning| warning.contains("DuplicateRoom") && warning.contains("2 rooms")));
    }

    #[test]
    fn procedural_graph_reports_actual_edges_and_legacy_probability_floor() {
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag] {
            for seed in [3, 42, 91] {
                let mut cfg = MapWizardConfig {
                    game_mode: mode, layout_strategy: WizardLayoutStrategy::ProceduralSeed,
                    procedural_room_count: 6, seed, ..Default::default()
                };
                cfg.procedural_loop_ratio = 0.0;
                let low = preview_wizard_topology(&cfg).procedural_graph.unwrap();
                let plain = build_room_nodes(&cfg);
                let (with_summary, _) = build_room_nodes_with_graph(&cfg, true);
                assert_eq!(plain.len(), with_summary.len());
                for (left, right) in plain.iter().zip(&with_summary) {
                    assert_eq!((&left.name, left.center, left.size), (&right.name, right.center, right.size));
                }
                cfg.procedural_loop_ratio = 0.35;
                let floor = preview_wizard_topology(&cfg).procedural_graph.unwrap();
                assert_eq!(low.edges, floor.edges, "legacy floor should preserve the graph for {mode:?}, seed {seed}");
                assert_eq!(low.effective_extra_probability, 0.35);
                for graph in [&low, &floor] {
                    assert_eq!(graph.tree_edges + graph.random_edges + graph.minimum_degree_edges, graph.edges.len());
                    assert_eq!(graph.independent_cycles, graph.edges.len() + graph.components - graph.room_count);
                    assert_eq!(graph.components, 1);
                    assert!(graph.minimum_degree >= 2);
                    assert!(graph.edges.iter().all(|(a, b)| a != b));
                }
                cfg.procedural_loop_ratio = 1.0;
                let maximum = preview_wizard_topology(&cfg).procedural_graph.unwrap();
                assert_eq!(maximum.effective_extra_probability, 1.0);
                assert_eq!(maximum.tree_edges + maximum.random_edges + maximum.minimum_degree_edges, maximum.edges.len());
            }
        }
        assert!(preview_wizard_topology(&MapWizardConfig::default()).procedural_graph.is_none());
    }

    #[test]
    fn debug_inspect_complex_player_starts() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::InterconnectedComplex,
            room_shape: WizardRoomShape::Rectangular,
            seed: 1,
            ..Default::default()
        };
        cfg.sync_mode_conventions();
        let map = generate_valid_ut99_map(&cfg);
        for a in &map.actors {
            if a.class_name == "PlayerStart" {
                eprintln!("PS: {} loc: {:?}", a.name, a.location);
            }
        }
    }

    #[test]
    fn test_wizard_teleporter_audit_and_auto_fix_suite() {
        // 1. Default archetype should have healthy teleporters (2 connected teleporters)
        let mut cfg = MapWizardConfig::default();
        cfg.toggles.include_teleporters = true;
        let audit = audit_wizard_teleporters(&cfg);
        assert!(audit.is_healthy, "Default wizard teleporters must be healthy: {:?}", audit.issues);
        assert_eq!(audit.teleporter_count, 2);
        assert_eq!(audit.valid_links.len(), 2);
        assert!(audit.valid_links[0].is_bidirectional);

        // 2. Suppressing one teleporter must trigger LoneTeleporter error
        cfg.suppressed_markers.push("WizardTeleB".to_string());
        let lone_audit = audit_wizard_teleporters(&cfg);
        assert!(!lone_audit.is_healthy);
        assert_eq!(lone_audit.teleporter_count, 1);
        assert!(lone_audit.issues.iter().any(|i| i.kind == WizardTeleportIssueKind::LoneTeleporter));

        // 3. Auto-fix must spawn a companion teleporter and restore full connectivity
        let fixes = auto_fix_wizard_teleporters(&mut cfg);
        assert!(!fixes.is_empty(), "Auto-fix must report repair actions");
        let repaired_audit = audit_wizard_teleporters(&cfg);
        assert!(repaired_audit.is_healthy, "Repaired config must be healthy: {:?}", repaired_audit.issues);
        assert_eq!(repaired_audit.teleporter_count, 2);

        // 4. Test disconnected custom teleporters (missing destinations)
        let mut custom_cfg = MapWizardConfig::default();
        custom_cfg.toggles.include_teleporters = false; // Disable archetypes
        custom_cfg.custom_markers.push(WizardCustomMarker {
            id: "t1".into(),
            label: "CustomTele1".into(),
            kind: "Teleporter".into(),
            class_name: "Engine.Teleporter".into(),
            location: Vec3::new(100.0, 100.0, 50.0),
            team: None,
            teleporter_tag: "PortalAlpha".into(),
            teleporter_url: String::new(), // EMPTY!
        });
        custom_cfg.custom_markers.push(WizardCustomMarker {
            id: "t2".into(),
            label: "CustomTele2".into(),
            kind: "Teleporter".into(),
            class_name: "Engine.Teleporter".into(),
            location: Vec3::new(1000.0, 1000.0, 50.0),
            team: None,
            teleporter_tag: "PortalBeta".into(),
            teleporter_url: "NonExistentTag".into(), // BROKEN!
        });

        let broken_audit = audit_wizard_teleporters(&custom_cfg);
        assert!(!broken_audit.is_healthy);
        assert!(broken_audit.issues.iter().any(|i| i.kind == WizardTeleportIssueKind::MissingDestination));
        assert!(broken_audit.issues.iter().any(|i| i.kind == WizardTeleportIssueKind::UnresolvedDestination));

        // Auto-fix connects them into a bilateral link
        let custom_fixes = auto_fix_wizard_teleporters(&mut custom_cfg);
        assert!(!custom_fixes.is_empty());
        let fixed_custom_audit = audit_wizard_teleporters(&custom_cfg);
        assert!(fixed_custom_audit.is_healthy, "Repaired custom teleporters must be healthy: {:?}", fixed_custom_audit.issues);
        assert_eq!(fixed_custom_audit.valid_links.len(), 2);
        assert!(fixed_custom_audit.valid_links[0].is_bidirectional);

        // 5. Test loop hazard separation (< 96 UU)
        let mut hazard_cfg = MapWizardConfig::default();
        hazard_cfg.toggles.include_teleporters = false;
        hazard_cfg.custom_markers.push(WizardCustomMarker {
            id: "h1".into(),
            label: "TrapA".into(),
            kind: "Teleporter".into(),
            class_name: "Engine.Teleporter".into(),
            location: Vec3::new(100.0, 100.0, 50.0),
            team: None,
            teleporter_tag: "TrapA_Tag".into(),
            teleporter_url: "TrapB_Tag".into(),
        });
        hazard_cfg.custom_markers.push(WizardCustomMarker {
            id: "h2".into(),
            label: "TrapB".into(),
            kind: "Teleporter".into(),
            class_name: "Engine.Teleporter".into(),
            location: Vec3::new(130.0, 100.0, 50.0), // Only 30 UU away!
            team: None,
            teleporter_tag: "TrapB_Tag".into(),
            teleporter_url: "TrapA_Tag".into(),
        });

        let hazard_audit = audit_wizard_teleporters(&hazard_cfg);
        assert!(!hazard_audit.is_healthy);
        assert!(hazard_audit.issues.iter().any(|i| i.kind == WizardTeleportIssueKind::LoopHazard));

        auto_fix_wizard_teleporters(&mut hazard_cfg);
        let separated_audit = audit_wizard_teleporters(&hazard_cfg);
        assert!(separated_audit.is_healthy, "Separated teleporters must clear loop hazard: {:?}", separated_audit.issues);

        // 6. Test map generation enforces invariant and generates valid UT99 actor properties
        let map = generate_valid_ut99_map(&hazard_cfg);
        let tele_actors: Vec<_> = map.actors.iter().filter(|a| a.class_name.contains("Teleporter")).collect();
        assert!(tele_actors.len() >= 2);
        for tele in &tele_actors {
            assert!(!tele.tag.is_empty(), "Teleporter actor must have Tag property");
            let url_prop = tele.str_props.iter().find(|(k, _)| k == "URL");
            assert!(url_prop.is_some(), "Teleporter actor must have URL property");
        }
    }

    #[test]
    fn test_wizard_ensure_vertical_reachability() {
        // Test with GothicCathedralNave which has Nave (floor_z = 0) and Apse Altar (floor_z = 128)
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::GothicCathedralNave,
            ..Default::default()
        };
        cfg.toggles.ensure_vertical_reachability = true;
        let map = generate_valid_ut99_map(&cfg);

        // Verify connecting transit stairs/ramps were generated
        let transit_vertical: Vec<_> = map
            .brushes
            .iter()
            .filter(|b| b.name.contains("TransitRamp") || b.name.contains("TransitStair"))
            .collect();
        assert!(
            !transit_vertical.is_empty(),
            "Expected at least one transit stair or ramp for Gothic Cathedral elevation difference"
        );

        let foot_nodes = map.actors.iter().filter(|a| a.name.starts_with("PathNode_Foot_")).count();
        let head_nodes = map.actors.iter().filter(|a| a.name.starts_with("PathNode_Head_")).count();
        assert!(!transit_vertical.is_empty());
        assert!(foot_nodes >= 1);
        assert_eq!(foot_nodes, head_nodes);
    }

    #[test]
    fn test_wizard_exact_item_quota_distribution() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::SunkenColosseumBasin,
            ..Default::default()
        };
        cfg.item_quota = WizardItemQuotaConfig {
            enabled: true,
            redeemer: 2,
            rocket_launcher: 3,
            flak_cannon: 3,
            shock_rifle: 2,
            sniper_rifle: 1,
            minigun: 1,
            pulse_gun: 1,
            ripper: 0,
            bio_rifle: 0,
            enforcer: 2,
            chainsaw: 1,

            shield_belt: 2,
            body_armor: 2,
            thigh_pads: 3,

            super_keg_health: 1,
            medbox: 5,
            health_vials: 10,

            udamage: 2,
            jump_boots: 2,
            invisibility: 1,
        };

        let map = generate_valid_ut99_map(&cfg);

        let count_class = |cls: &str| -> usize {
            map.actors.iter().filter(|a| a.class_name == cls).count()
        };

        // Verify exact weapon counts
        assert_eq!(count_class("WarHeadLauncher"), 2);
        assert_eq!(count_class("UT_Eightball"), 3);
        assert_eq!(count_class("UT_FlakCannon"), 3);
        assert_eq!(count_class("ShockRifle"), 2);
        assert_eq!(count_class("SniperRifle"), 1);
        assert_eq!(count_class("minigun2"), 1);
        assert_eq!(count_class("PulseGun"), 1);
        assert_eq!(count_class("Ripper"), 0);
        assert_eq!(count_class("ut_biorifle"), 0);
        assert_eq!(count_class("Enforcer"), 2);
        assert_eq!(count_class("ChainSaw"), 1);

        // Verify exact armor & powerup counts
        assert_eq!(count_class("UT_ShieldBelt"), 2);
        assert_eq!(count_class("Armor2"), 2);
        assert_eq!(count_class("ThighPads"), 3);
        assert_eq!(count_class("HealthPack"), 1);
        assert_eq!(count_class("MedBox"), 5);
        assert_eq!(count_class("HealthVial"), 10);
        assert_eq!(count_class("UDamage"), 2);
        assert_eq!(count_class("UT_JumpBoots"), 2);
        assert_eq!(count_class("UT_Invisibility"), 1);

        // Verify that duplicate power items are dispersed across different rooms
        let redeemers: Vec<Vec3> = map.actors.iter().filter(|a| a.class_name == "WarHeadLauncher").map(|a| a.location).collect();
        assert_eq!(redeemers.len(), 2);
        assert!(redeemers[0].distance(redeemers[1]) > 400.0, "Duplicate Redeemers must be dispersed across different rooms, distance was: {}", redeemers[0].distance(redeemers[1]));
    }

    #[test]
    fn test_custom_ammo_per_weapon_generation() {
        let mut cfg = MapWizardConfig {
            game_mode: WizardGameMode::DeathMatch,
            layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
            archetype: WizardArchetype::CrucibleDuelPit,
            default_ammo_per_weapon: 2,
            ..Default::default()
        };
        cfg.custom_ammo_per_weapon.insert("UT_FlakCannon".to_string(), 4);
        cfg.custom_ammo_per_weapon.insert("ShockRifle".to_string(), 1);

        assert_eq!(cfg.ammo_count_for_weapon("UT_FlakCannon"), 4);
        assert_eq!(cfg.ammo_count_for_weapon("ShockRifle"), 1);
        assert_eq!(cfg.ammo_count_for_weapon("UT_Eightball"), 2); // Default

        let map = generate_valid_ut99_map(&cfg);
        let flak_count = map.actors.iter().filter(|a| a.class_name == "UT_FlakCannon").count();
        let flak_ammo_count = map.actors.iter().filter(|a| a.class_name == "FlakAmmo").count();
        assert_eq!(flak_ammo_count, flak_count * 4);
    }

    #[test]
    fn test_all_canonical_archetypes_have_connected_rooms() {
        let mut failures = Vec::new();

        for &arch in WizardArchetype::all() {
            let cfg = MapWizardConfig {
                layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
                archetype: arch,
                scale: WizardMapScale::Standard,
                ..Default::default()
            };
            let rooms = build_room_nodes(&cfg);
            assert!(!rooms.is_empty(), "Archetype {:?} produced no rooms", arch);

            let n = rooms.len();
            let mut adj = vec![vec![false; n]; n];

            // 1. Direct 3D box overlap (subtractive air carving requires positive volume overlap)
            for i in 0..n {
                for j in (i + 1)..n {
                    let a = &rooms[i];
                    let b = &rooms[j];
                    if room_boxes_overlap(a, b) {
                        adj[i][j] = true;
                        adj[j][i] = true;
                    }
                }
            }

            // 2. Teleporter connections
            if arch == WizardArchetype::FacingWorldsOrbitalSpire {
                let red_s = rooms.iter().position(|r| r.name == "RedFlagSanctum");
                let red_r = rooms.iter().position(|r| r.name == "RedSniperRoost");
                let blue_s = rooms.iter().position(|r| r.name == "BlueFlagSanctum");
                let blue_r = rooms.iter().position(|r| r.name == "BlueSniperRoost");
                if let (Some(s), Some(r)) = (red_s, red_r) {
                    adj[s][r] = true; adj[r][s] = true;
                }
                if let (Some(s), Some(r)) = (blue_s, blue_r) {
                    adj[s][r] = true; adj[r][s] = true;
                }
            }

            // BFS from room 0
            let mut visited = vec![false; n];
            let mut queue = vec![0];
            visited[0] = true;
            while let Some(curr) = queue.pop() {
                for next in 0..n {
                    if adj[curr][next] && !visited[next] {
                        visited[next] = true;
                        queue.push(next);
                    }
                }
            }

            let unvisited: Vec<String> = rooms
                .iter()
                .enumerate()
                .filter(|(idx, _)| !visited[*idx])
                .map(|(_, r)| r.name.clone())
                .collect();

            if !unvisited.is_empty() {
                failures.push((arch, unvisited));
            }
        }

        assert!(
            failures.is_empty(),
            "The following archetypes have disconnected rooms: {:?}",
            failures
        );
    }

    #[test]
    fn test_audit_stair_alignment_across_all_archetypes() {
        let mut total_stairs = 0;
        let mut all_issues = Vec::new();

        for &arch in WizardArchetype::all() {
            let mut cfg = MapWizardConfig {
                game_mode: WizardGameMode::DeathMatch,
                layout_strategy: WizardLayoutStrategy::CanonicalArchetype,
                archetype: arch,
                ..Default::default()
            };
            cfg.toggles.ensure_vertical_reachability = true;
            cfg.architectural_detail = WizardArchitecturalDetail::EpicHighDetail;
            cfg.sync_mode_conventions();

            let map = generate_valid_ut99_map(&cfg);
            let mut map_stairs = Vec::new();
            for b in &map.brushes {
                if (b.name.contains("Stairs") || b.name.contains("TransitStair_") || b.name.contains("TransitRamp_"))
                    && !b.name.contains("Col")
                {
                    map_stairs.push(b.name.clone());
                }
            }
            total_stairs += map_stairs.len();
            if !map_stairs.is_empty() {
                println!("  Archetype {:?}: {} stairs -> {:?}", arch, map_stairs.len(), map_stairs);
            }

            let issues = audit_stair_alignment(&cfg, &map);
            for iss in issues {
                all_issues.push((arch, iss));
            }
        }

        println!("\n=== AUDIT STAIR ALIGNMENT: Found {} stairs across 40 archetypes ===", total_stairs);
        println!("=== TOTAL DEFECTS DETECTED: {} ===", all_issues.len());
        for (arch, iss) in &all_issues {
            println!("  [{:?}] {}: {:?} -> {}", arch, iss.stair_name, iss.issue_type, iss.details);
        }

        assert!(
            all_issues.is_empty(),
            "=== STAIR ALIGNMENT AUDIT REGRESSION: Found {} defects ===\n{:#?}",
            all_issues.len(),
            all_issues
        );
    }

    fn procedural_cfg(mode: WizardGameMode, seed: u64, version: u32) -> MapWizardConfig {
        let mut cfg = MapWizardConfig {
            game_mode: mode,
            layout_strategy: WizardLayoutStrategy::ProceduralSeed,
            seed,
            architectural_detail: WizardArchitecturalDetail::StandardTournament,
            generator_version: version,
            ..Default::default()
        };
        cfg.sync_mode_conventions();
        cfg
    }

    fn floating_frame_count(cfg: &MapWizardConfig) -> usize {
        let rooms = build_room_nodes(cfg);
        let map = generate_valid_ut99_map(cfg);
        map.brushes
            .iter()
            .filter(|b| b.name.contains("_Jamb_") || b.name.contains("_Lintel_"))
            .filter(|b| frame_floats_in_room(&rooms, b.location))
            .count()
    }

    #[test]
    fn clip_corridor_span_trims_to_room_walls() {
        // Corridor 0..1000, room A ends at 300, room B starts at 700.
        let (lo, hi) = clip_corridor_span(0.0, 1000.0, Some((-200.0, 300.0)), Some((700.0, 1200.0)));
        assert_eq!((lo, hi), (300.0 - CORRIDOR_PORTAL_OVERLAP, 700.0 + CORRIDOR_PORTAL_OVERLAP));
        // Ends outside any room stay untouched.
        assert_eq!(clip_corridor_span(0.0, 1000.0, None, None), (0.0, 1000.0));
        // Walls beyond the opposite end, or a collapsing span, leave the corridor as it was.
        assert_eq!(clip_corridor_span(0.0, 1000.0, Some((0.0, 990.0)), Some((10.0, 1000.0))), (0.0, 1000.0));
    }

    #[test]
    fn portal_aligned_corridors_do_not_cross_their_end_rooms() {
        let a = RoomNode {
            name: "A".into(), center: Vec3::new(0.0, 0.0, 256.0), size: Vec3::new(768.0, 640.0, 512.0),
            is_corridor: false, sky_ceiling: false, has_hazard_pit: false, team_owner: None, is_octagonal: false,
        };
        let b = RoomNode { name: "B".into(), center: Vec3::new(2400.0, 0.0, 256.0), ..a.clone() };
        let legacy = procedural_corridor_nodes(0, &a, &b, &MapWizardConfig { generator_version: GENERATOR_VERSION_LEGACY, ..Default::default() });
        let aligned = procedural_corridor_nodes(0, &a, &b, &MapWizardConfig { generator_version: GENERATOR_VERSION_PORTALS, ..Default::default() });
        assert_eq!(legacy.len(), 1);
        assert_eq!(aligned.len(), 1);
        // Legacy spans centre-to-centre; aligned spans wall-to-wall (+ overlap).
        assert!(legacy[0].min_x() < a.max_x() - 100.0);
        assert!(aligned[0].min_x() >= a.max_x() - CORRIDOR_PORTAL_OVERLAP - 0.01);
        assert!(aligned[0].max_x() <= b.min_x() + CORRIDOR_PORTAL_OVERLAP + 0.01);
    }

    #[test]
    fn portal_aligned_procedural_maps_have_no_floating_doorway_frames() {
        let (mut legacy_floating, mut aligned_floating) = (0, 0);
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag] {
            for seed in 1..=12u64 {
                legacy_floating += floating_frame_count(&procedural_cfg(mode, seed, GENERATOR_VERSION_LEGACY));
                aligned_floating += floating_frame_count(&procedural_cfg(mode, seed, GENERATOR_VERSION_PORTALS));
            }
        }
        assert!(legacy_floating > 0, "legacy should reproduce the floating-frame defect");
        assert_eq!(aligned_floating, 0);
    }

    #[test]
    fn anchored_details_touch_rock_in_rules_v2() {
        let (mut legacy_loose, mut v2_loose) = (0, 0);
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag] {
            for seed in 1..=8u64 {
                for (version, loose) in [(GENERATOR_VERSION_LEGACY, &mut legacy_loose), (GENERATOR_VERSION_PORTALS, &mut v2_loose)] {
                    let map = generate_valid_ut99_map(&procedural_cfg(mode, seed, version));
                    let carved: Vec<(Vec3, Vec3)> = map
                        .brushes
                        .iter()
                        .filter(|b| b.csg_oper == CsgOperation::Subtract)
                        .map(brush_bounds)
                        .collect();
                    for b in map.brushes.iter().filter(|b| {
                        b.name.starts_with("CorridorBeam_") || b.name.starts_with("LightFixture_Accent_") || b.name.contains("Rafter_")
                    }) {
                        let (lo, hi) = brush_bounds(b);
                        if !detail_touches_solid(&carved, lo, hi) {
                            *loose += 1;
                        }
                    }
                }
            }
        }
        assert!(legacy_loose > 0, "legacy should reproduce the loose-detail defect");
        assert_eq!(v2_loose, 0);
    }

    #[test]
    fn transit_stairs_keep_headroom_in_rules_v3() {
        let (mut legacy_tight, mut v3_tight, mut v3_stairs) = (0, 0, 0);
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag] {
            for seed in [1u64, 2, 8, 13, 7001, 7027] {
                for (version, tight) in [(GENERATOR_VERSION_LEGACY, &mut legacy_tight), (GENERATOR_VERSION_HEIGHTS, &mut v3_tight)] {
                    let map = generate_valid_ut99_map(&procedural_cfg(mode, seed, version));
                    let carved: Vec<(Vec3, Vec3)> = map
                        .brushes
                        .iter()
                        .filter(|b| b.csg_oper == CsgOperation::Subtract)
                        .map(brush_bounds)
                        .collect();
                    for b in map.brushes.iter().filter(|b| b.name.starts_with("TransitStair_") || b.name.starts_with("TransitRamp_")) {
                        let (lo, hi) = brush_bounds(b);
                        if version == GENERATOR_VERSION_HEIGHTS {
                            v3_stairs += 1;
                        }
                        let squeezed = carved.iter().any(|(clo, chi)| {
                            clo.x < hi.x && chi.x > lo.x && clo.y < hi.y && chi.y > lo.y
                                && clo.z < hi.z - 1.0 && chi.z > lo.z + 1.0
                                && chi.z - hi.z < MIN_STAIR_HEADROOM
                        });
                        if squeezed {
                            *tight += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(v3_tight, 0, "rules v3 stairs must keep {MIN_STAIR_HEADROOM} UU of headroom");
        let _ = legacy_tight;
        assert!(v3_stairs > 0, "the sample must contain stairs");
    }

    #[test]
    fn height_library_keeps_layout_and_respects_role_floors() {
        for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag, WizardGameMode::TeamDeathMatch] {
            for seed in 1..=20u64 {
                let v2 = build_room_nodes(&procedural_cfg(mode, seed, GENERATOR_VERSION_PORTALS));
                let v3 = build_room_nodes(&procedural_cfg(mode, seed, GENERATOR_VERSION_HEIGHTS));
                let rooms2: Vec<_> = v2.iter().filter(|r| !r.is_corridor).collect();
                let rooms3: Vec<_> = v3.iter().filter(|r| !r.is_corridor).collect();
                assert_eq!(rooms2.len(), rooms3.len());
                for (a, b) in rooms2.iter().zip(&rooms3) {
                    // Same slots, same footprints, same floors; only the ceiling changes.
                    assert_eq!((a.name.as_str(), a.center.x, a.center.y, a.size.x, a.size.y), (b.name.as_str(), b.center.x, b.center.y, b.size.x, b.size.y));
                    assert!((a.floor_z() - b.floor_z()).abs() < 0.01, "{} floor moved", a.name);
                    assert!(b.size.z >= 256.0 && b.size.z <= 768.0);
                    if b.name == "ProcHub_0" {
                        assert!(b.size.z >= 512.0);
                    }
                    if b.team_owner.is_some() {
                        assert!(b.size.z >= 384.0, "{} team room too low", b.name);
                    }
                }
                for c in v3.iter().filter(|r| r.is_corridor) {
                    assert!((192.0..=320.0).contains(&c.size.z), "{} height {}", c.name, c.size.z);
                }
            }
        }
    }

    #[test]
    fn height_library_mix_is_bimodal() {
        let mut low = 0;
        let mut total = 0;
        for seed in 1..=40u64 {
            for r in build_room_nodes(&procedural_cfg(WizardGameMode::DeathMatch, seed, GENERATOR_VERSION_HEIGHTS)).into_iter().filter(|r| !r.is_corridor) {
                total += 1;
                if r.size.z < 288.0 { low += 1; }
            }
        }
        let share = low as f32 / total as f32;
        assert!((0.1..=0.5).contains(&share), "low room share {share}");
    }
}
