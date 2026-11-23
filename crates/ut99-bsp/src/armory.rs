// crates/ut99-bsp/src/armory.rs
//
// Complete UT99 Armory, Pickups, and Powerups Registry.
// Catalogs stock weapons, ammunition pairings, powerups, armor, and health items
// for procedural generation and custom Map Wizard loadout configurations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponCategory {
    Hitscan,
    Projectile,
    ExplosiveHeavy,
    Melee,
    Classic,
}

impl WeaponCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hitscan => "Hitscan / Precision",
            Self::Projectile => "Projectile / Energy",
            Self::ExplosiveHeavy => "Heavy Explosives",
            Self::Melee => "Melee / Close Range",
            Self::Classic => "Classic Unreal 1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerupCategory {
    Powerup,
    Armor,
    Health,
}

impl PowerupCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Powerup => "Powerups & Mobility",
            Self::Armor => "Protective Armor",
            Self::Health => "Health Restoratives",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub class_name: &'static str,
    pub ammo_class: Option<&'static str>,
    pub ammo_display: Option<&'static str>,
    pub icon: &'static str,
    pub category: WeaponCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerupDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub class_name: &'static str,
    pub icon: &'static str,
    pub category: PowerupCategory,
    pub description: &'static str,
}

/// Complete catalog of stock UT99 Tournament weapons.
pub const STOCK_TOURNAMENT_WEAPONS: &[WeaponDescriptor] = &[
    WeaponDescriptor {
        id: "ShockRifle",
        display_name: "ASMD Shock Rifle",
        class_name: "ShockRifle",
        ammo_class: Some("ShockCore"),
        ammo_display: Some("Shock Core"),
        icon: "⚡",
        category: WeaponCategory::Hitscan,
    },
    WeaponDescriptor {
        id: "UT_FlakCannon",
        display_name: "Flak Cannon",
        class_name: "UT_FlakCannon",
        ammo_class: Some("FlakAmmo"),
        ammo_display: Some("Flak Shells"),
        icon: "✦",
        category: WeaponCategory::Projectile,
    },
    WeaponDescriptor {
        id: "UT_Eightball",
        display_name: "Rocket Launcher / Eightball",
        class_name: "UT_Eightball",
        ammo_class: Some("RocketPack"),
        ammo_display: Some("Rocket Pack"),
        icon: "▲",
        category: WeaponCategory::ExplosiveHeavy,
    },
    WeaponDescriptor {
        id: "PulseGun",
        display_name: "Pulse Gun",
        class_name: "PulseGun",
        ammo_class: Some("PAmmo"),
        ammo_display: Some("Pulse Ammo"),
        icon: "◉",
        category: WeaponCategory::Projectile,
    },
    WeaponDescriptor {
        id: "SniperRifle",
        display_name: "Sniper Rifle",
        class_name: "SniperRifle",
        ammo_class: Some("BulletBox"),
        ammo_display: Some("Rifle Rounds"),
        icon: "⊕",
        category: WeaponCategory::Hitscan,
    },
    WeaponDescriptor {
        id: "minigun2",
        display_name: "Minigun",
        class_name: "minigun2",
        ammo_class: Some("MiniAmmo"),
        ammo_display: Some("50 Cal Ammo Box"),
        icon: "❖",
        category: WeaponCategory::Hitscan,
    },
    WeaponDescriptor {
        id: "Ripper",
        display_name: "Ripper",
        class_name: "Ripper",
        ammo_class: Some("BladeHopper"),
        ammo_display: Some("Razor Blades"),
        icon: "❂",
        category: WeaponCategory::Projectile,
    },
    WeaponDescriptor {
        id: "ut_biorifle",
        display_name: "GES BioRifle",
        class_name: "ut_biorifle",
        ammo_class: Some("BioAmmo"),
        ammo_display: Some("Tarydium Sludge"),
        icon: "☣",
        category: WeaponCategory::Projectile,
    },
    WeaponDescriptor {
        id: "WarHeadLauncher",
        display_name: "Redeemer / WarHead Launcher",
        class_name: "WarHeadLauncher",
        ammo_class: Some("WarHeadAmmo"),
        ammo_display: Some("Nuclear Warhead"),
        icon: "☢",
        category: WeaponCategory::ExplosiveHeavy,
    },
    WeaponDescriptor {
        id: "Enforcer",
        display_name: "Enforcer",
        class_name: "Enforcer",
        ammo_class: Some("MiniAmmo"),
        ammo_display: Some("Bullets"),
        icon: "▸",
        category: WeaponCategory::Hitscan,
    },
    WeaponDescriptor {
        id: "ChainSaw",
        display_name: "Chainsaw",
        class_name: "ChainSaw",
        ammo_class: None,
        ammo_display: None,
        icon: "⚔",
        category: WeaponCategory::Melee,
    },
];

/// Classic Unreal 1 weapons functional inside UT99.
pub const CLASSIC_UNREAL_WEAPONS: &[WeaponDescriptor] = &[
    WeaponDescriptor {
        id: "ASMD",
        display_name: "Classic ASMD",
        class_name: "ASMD",
        ammo_class: Some("ASMDAmmo"),
        ammo_display: Some("ASMD Core"),
        icon: "⚡",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "Eightball",
        display_name: "Classic Eightball",
        class_name: "Eightball",
        ammo_class: Some("RocketCan"),
        ammo_display: Some("Rocket Canister"),
        icon: "▲",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "FlakCannon",
        display_name: "Classic Flak Cannon",
        class_name: "FlakCannon",
        ammo_class: Some("FlakBox"),
        ammo_display: Some("Flak Box"),
        icon: "✦",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "Stinger",
        display_name: "Stinger",
        class_name: "Stinger",
        ammo_class: Some("StingerAmmo"),
        ammo_display: Some("Tarydium Shards"),
        icon: "◆",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "AutoMag",
        display_name: "AutoMag",
        class_name: "AutoMag",
        ammo_class: Some("Clip"),
        ammo_display: Some("Clip"),
        icon: "▸",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "RazorJack",
        display_name: "RazorJack",
        class_name: "RazorJack",
        ammo_class: Some("RazorAmmo"),
        ammo_display: Some("Razor Blades"),
        icon: "❂",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "Rifle",
        display_name: "Classic Sniper Rifle",
        class_name: "Rifle",
        ammo_class: Some("RifleAmmo"),
        ammo_display: Some("Rifle Rounds"),
        icon: "⊕",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "Minigun",
        display_name: "Classic Minigun",
        class_name: "Minigun",
        ammo_class: Some("ShellBox"),
        ammo_display: Some("Shell Box"),
        icon: "❖",
        category: WeaponCategory::Classic,
    },
    WeaponDescriptor {
        id: "GESBioRifle",
        display_name: "Classic BioRifle",
        class_name: "GESBioRifle",
        ammo_class: Some("Sludge"),
        ammo_display: Some("Tarydium Sludge"),
        icon: "☣",
        category: WeaponCategory::Classic,
    },
];

/// Complete catalog of stock UT99 Powerups, Armor, and Health pickups.
pub const STOCK_POWERUPS_AND_PICKUPS: &[PowerupDescriptor] = &[
    // Powerups
    PowerupDescriptor {
        id: "UDamage",
        display_name: "Damage Amplifier (UDamage)",
        class_name: "UDamage",
        icon: "⚡",
        category: PowerupCategory::Powerup,
        description: "Triples outgoing weapon damage for 30 seconds",
    },
    PowerupDescriptor {
        id: "UT_ShieldBelt",
        display_name: "Shield Belt",
        class_name: "UT_ShieldBelt",
        icon: "◈",
        category: PowerupCategory::Powerup,
        description: "Provides 150 points of full-body impenetrable energy shield",
    },
    PowerupDescriptor {
        id: "UT_JumpBoots",
        display_name: "Anti-Grav Jump Boots",
        class_name: "UT_JumpBoots",
        icon: "▲",
        category: PowerupCategory::Powerup,
        description: "Grants 3 multi-story kinetic leaps to access high ground",
    },
    PowerupDescriptor {
        id: "UT_Invisibility",
        display_name: "Invisibility",
        class_name: "UT_Invisibility",
        icon: "◎",
        category: PowerupCategory::Powerup,
        description: "Optical cloak making the carrier virtually invisible",
    },
    // Armor
    PowerupDescriptor {
        id: "Armor2",
        display_name: "Body Armor",
        class_name: "Armor2",
        icon: "◆",
        category: PowerupCategory::Armor,
        description: "Absorbs 75% damage up to 100 armor points",
    },
    PowerupDescriptor {
        id: "ThighPads",
        display_name: "Thigh Pads",
        class_name: "ThighPads",
        icon: "◇",
        category: PowerupCategory::Armor,
        description: "Absorbs 50% damage up to 50 armor points",
    },
    // Health
    PowerupDescriptor {
        id: "HealthPack",
        display_name: "Super Health Pack (Keg of Health)",
        class_name: "HealthPack",
        icon: "✚",
        category: PowerupCategory::Health,
        description: "Restores +100 health up to the maximum super-health cap of 199",
    },
    PowerupDescriptor {
        id: "MedBox",
        display_name: "MedBox",
        class_name: "MedBox",
        icon: "✚",
        category: PowerupCategory::Health,
        description: "Restores +20 health up to 100",
    },
    PowerupDescriptor {
        id: "HealthVial",
        display_name: "Health Vial",
        class_name: "HealthVial",
        icon: "♦",
        category: PowerupCategory::Health,
        description: "Restores +5 health, stacking up to the super-health cap of 199",
    },
];

/// Curated weapon loadout presets for the Map Wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WizardWeaponPreset {
    #[default]
    TournamentStandard,
    CloseQuarters,
    HeavyExplosives,
    SniperDuel,
    ClassicUnreal,
    FullTournament,
    Custom,
}

impl WizardWeaponPreset {
    pub fn all() -> &'static [WizardWeaponPreset] {
        &[
            Self::TournamentStandard,
            Self::CloseQuarters,
            Self::HeavyExplosives,
            Self::SniperDuel,
            Self::ClassicUnreal,
            Self::FullTournament,
            Self::Custom,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::TournamentStandard => "Tournament Standard (Shock, Flak, Rocket, Pulse, Sniper, Minigun)",
            Self::CloseQuarters => "Close Quarters / Melee (Flak, Ripper, BioRifle, Chainsaw, Enforcer)",
            Self::HeavyExplosives => "Heavy Explosives (Redeemer, Rocket Launcher, Flak Cannon)",
            Self::SniperDuel => "Precision Duel (Sniper Rifle, Shock Rifle, Enforcer)",
            Self::ClassicUnreal => "Classic Unreal 1 (ASMD, Eightball, Flak, Stinger, AutoMag, RazorJack)",
            Self::FullTournament => "Full Armory (All Stock Tournament Weapons)",
            Self::Custom => "Custom Armory Loadout (Manual Selection)",
        }
    }

    pub fn default_weapons(&self) -> Vec<String> {
        match self {
            Self::TournamentStandard => vec![
                "ShockRifle".to_string(),
                "UT_FlakCannon".to_string(),
                "UT_Eightball".to_string(),
                "PulseGun".to_string(),
                "SniperRifle".to_string(),
                "minigun2".to_string(),
            ],
            Self::CloseQuarters => vec![
                "UT_FlakCannon".to_string(),
                "Ripper".to_string(),
                "ut_biorifle".to_string(),
                "ChainSaw".to_string(),
                "Enforcer".to_string(),
            ],
            Self::HeavyExplosives => vec![
                "WarHeadLauncher".to_string(),
                "UT_Eightball".to_string(),
                "UT_FlakCannon".to_string(),
                "ShockRifle".to_string(),
            ],
            Self::SniperDuel => vec![
                "SniperRifle".to_string(),
                "ShockRifle".to_string(),
                "Enforcer".to_string(),
            ],
            Self::ClassicUnreal => vec![
                "ASMD".to_string(),
                "Eightball".to_string(),
                "FlakCannon".to_string(),
                "Stinger".to_string(),
                "AutoMag".to_string(),
                "RazorJack".to_string(),
            ],
            Self::FullTournament => STOCK_TOURNAMENT_WEAPONS
                .iter()
                .map(|w| w.class_name.to_string())
                .collect(),
            Self::Custom => vec![
                "ShockRifle".to_string(),
                "UT_FlakCannon".to_string(),
                "UT_Eightball".to_string(),
            ],
        }
    }
}

/// Curated powerup & pickup distribution presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WizardPowerupPreset {
    #[default]
    BalancedCompetitive,
    HighMobility,
    MayhemOverkill,
    HardcoreTactical,
    Custom,
}

impl WizardPowerupPreset {
    pub fn all() -> &'static [WizardPowerupPreset] {
        &[
            Self::BalancedCompetitive,
            Self::HighMobility,
            Self::MayhemOverkill,
            Self::HardcoreTactical,
            Self::Custom,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::BalancedCompetitive => "Balanced Competitive (ShieldBelt, UDamage, Armor, MedBoxes, Vials)",
            Self::HighMobility => "High Mobility (Jump Boots, Invisibility, ShieldBelt, Armor, Health)",
            Self::MayhemOverkill => "Mayhem Overkill (All Powerups + Keg of Health + Heavy Shields)",
            Self::HardcoreTactical => "Hardcore Tactical (ThighPads, MedBoxes & Vials Only)",
            Self::Custom => "Custom Pickups Loadout (Manual Selection)",
        }
    }

    pub fn default_powerups(&self) -> Vec<String> {
        match self {
            Self::BalancedCompetitive => vec![
                "UT_ShieldBelt".to_string(),
                "UDamage".to_string(),
                "Armor2".to_string(),
                "ThighPads".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
            Self::HighMobility => vec![
                "UT_JumpBoots".to_string(),
                "UT_Invisibility".to_string(),
                "UT_ShieldBelt".to_string(),
                "ThighPads".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
            Self::MayhemOverkill => vec![
                "UDamage".to_string(),
                "UT_ShieldBelt".to_string(),
                "HealthPack".to_string(),
                "UT_JumpBoots".to_string(),
                "UT_Invisibility".to_string(),
                "Armor2".to_string(),
                "ThighPads".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
            Self::HardcoreTactical => vec![
                "ThighPads".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
            Self::Custom => vec![
                "UT_ShieldBelt".to_string(),
                "UDamage".to_string(),
                "MedBox".to_string(),
                "HealthVial".to_string(),
            ],
        }
    }
}

/// Item density and respawn pacing in the generated arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WizardItemDensity {
    Sparse,
    #[default]
    Standard,
    Abundant,
}

impl WizardItemDensity {
    pub fn all() -> &'static [WizardItemDensity] {
        &[Self::Sparse, Self::Standard, Self::Abundant]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sparse => "Sparse (Tactical Ammo Conservation / Low Drops)",
            Self::Standard => "Standard (Iconic UT99 Arena Pacing)",
            Self::Abundant => "Abundant (Double Weapons, 2x Ammo Boxes & Rich Restoratives)",
        }
    }
}

/// Single item entry to spawn from an exact quota specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaItemEntry {
    pub class_name: &'static str,
    pub display_name: &'static str,
    pub is_weapon: bool,
    pub priority: u8,
}

/// Exact item quota configuration for fine-grained absolute control over weapon,
/// armor, health, and powerup counts across the generated arena.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardItemQuotaConfig {
    pub enabled: bool,

    // Weapons (exact count)
    pub redeemer: u32,
    pub rocket_launcher: u32,
    pub flak_cannon: u32,
    pub shock_rifle: u32,
    pub sniper_rifle: u32,
    pub minigun: u32,
    pub pulse_gun: u32,
    pub ripper: u32,
    pub bio_rifle: u32,
    pub enforcer: u32,
    pub chainsaw: u32,

    // Protective Armor (exact count)
    pub shield_belt: u32,
    pub body_armor: u32,
    pub thigh_pads: u32,

    // Health Restoratives (exact count)
    pub super_keg_health: u32,
    pub medbox: u32,
    pub health_vials: u32,

    // Powerups & Utility (exact count)
    pub udamage: u32,
    pub jump_boots: u32,
    pub invisibility: u32,
}

impl Default for WizardItemQuotaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            redeemer: 1,
            rocket_launcher: 2,
            flak_cannon: 2,
            shock_rifle: 2,
            sniper_rifle: 1,
            minigun: 1,
            pulse_gun: 1,
            ripper: 1,
            bio_rifle: 1,
            enforcer: 2,
            chainsaw: 1,

            shield_belt: 1,
            body_armor: 2,
            thigh_pads: 2,

            super_keg_health: 1,
            medbox: 4,
            health_vials: 8,

            udamage: 1,
            jump_boots: 2,
            invisibility: 1,
        }
    }
}

impl WizardItemQuotaConfig {
    pub fn competitive_tournament() -> Self {
        Self {
            enabled: true,
            redeemer: 0,
            rocket_launcher: 2,
            flak_cannon: 2,
            shock_rifle: 2,
            sniper_rifle: 1,
            minigun: 1,
            pulse_gun: 1,
            ripper: 0,
            bio_rifle: 0,
            enforcer: 1,
            chainsaw: 0,

            shield_belt: 1,
            body_armor: 1,
            thigh_pads: 2,

            super_keg_health: 1,
            medbox: 4,
            health_vials: 8,

            udamage: 1,
            jump_boots: 1,
            invisibility: 0,
        }
    }

    pub fn heavy_carnage() -> Self {
        Self {
            enabled: true,
            redeemer: 2,
            rocket_launcher: 4,
            flak_cannon: 4,
            shock_rifle: 3,
            sniper_rifle: 2,
            minigun: 3,
            pulse_gun: 2,
            ripper: 2,
            bio_rifle: 2,
            enforcer: 2,
            chainsaw: 2,

            shield_belt: 2,
            body_armor: 4,
            thigh_pads: 4,

            super_keg_health: 2,
            medbox: 8,
            health_vials: 16,

            udamage: 2,
            jump_boots: 4,
            invisibility: 2,
        }
    }

    pub fn minimal_duel() -> Self {
        Self {
            enabled: true,
            redeemer: 0,
            rocket_launcher: 1,
            flak_cannon: 1,
            shock_rifle: 1,
            sniper_rifle: 1,
            minigun: 0,
            pulse_gun: 0,
            ripper: 0,
            bio_rifle: 0,
            enforcer: 1,
            chainsaw: 0,

            shield_belt: 1,
            body_armor: 0,
            thigh_pads: 1,

            super_keg_health: 0,
            medbox: 2,
            health_vials: 4,

            udamage: 0,
            jump_boots: 0,
            invisibility: 0,
        }
    }

    pub fn clear_all(&mut self) {
        self.redeemer = 0;
        self.rocket_launcher = 0;
        self.flak_cannon = 0;
        self.shock_rifle = 0;
        self.sniper_rifle = 0;
        self.minigun = 0;
        self.pulse_gun = 0;
        self.ripper = 0;
        self.bio_rifle = 0;
        self.enforcer = 0;
        self.chainsaw = 0;

        self.shield_belt = 0;
        self.body_armor = 0;
        self.thigh_pads = 0;

        self.super_keg_health = 0;
        self.medbox = 0;
        self.health_vials = 0;

        self.udamage = 0;
        self.jump_boots = 0;
        self.invisibility = 0;
    }

    pub fn total_weapons(&self) -> u32 {
        self.redeemer
            + self.rocket_launcher
            + self.flak_cannon
            + self.shock_rifle
            + self.sniper_rifle
            + self.minigun
            + self.pulse_gun
            + self.ripper
            + self.bio_rifle
            + self.enforcer
            + self.chainsaw
    }

    pub fn total_armor(&self) -> u32 {
        self.shield_belt + self.body_armor + self.thigh_pads
    }

    pub fn total_health(&self) -> u32 {
        self.super_keg_health + self.medbox + self.health_vials
    }

    pub fn total_powerups(&self) -> u32 {
        self.udamage + self.jump_boots + self.invisibility
    }

    pub fn total_items(&self) -> u32 {
        self.total_weapons() + self.total_armor() + self.total_health() + self.total_powerups()
    }

    /// Transforms the quota configuration into an ordered list of items ready for equitable distribution.
    pub fn to_quota_entries(&self) -> Vec<QuotaItemEntry> {
        let mut entries = Vec::new();
        let mut add = |count: u32, class_name: &'static str, display_name: &'static str, is_weapon: bool, priority: u8| {
            for _ in 0..count {
                entries.push(QuotaItemEntry {
                    class_name,
                    display_name,
                    is_weapon,
                    priority,
                });
            }
        };

        // 1. Rare Mega-Items (Priority 8..=10: Center, High Balconies, Prime Altars)
        add(self.redeemer, "WarHeadLauncher", "Redeemer", true, 10);
        add(self.shield_belt, "UT_ShieldBelt", "Shield Belt", false, 9);
        add(self.udamage, "UDamage", "Damage Amplifier", false, 9);
        add(self.super_keg_health, "HealthPack", "Keg of Health", false, 8);

        // 2. High-Demand Combat Weapons (Priority 6..=7)
        add(self.rocket_launcher, "UT_Eightball", "Rocket Launcher", true, 7);
        add(self.flak_cannon, "UT_FlakCannon", "Flak Cannon", true, 7);
        add(self.shock_rifle, "ShockRifle", "Shock Rifle", true, 7);
        add(self.sniper_rifle, "SniperRifle", "Sniper Rifle", true, 6);
        add(self.minigun, "minigun2", "Minigun", true, 6);
        add(self.pulse_gun, "PulseGun", "Pulse Gun", true, 6);

        // 3. Tactical Weapons (Priority 4..=5)
        add(self.ripper, "Ripper", "Ripper", true, 5);
        add(self.bio_rifle, "ut_biorifle", "BioRifle", true, 5);
        add(self.enforcer, "Enforcer", "Enforcer", true, 4);
        add(self.chainsaw, "ChainSaw", "Chainsaw", true, 4);

        // 4. Armor & Mobility Pickups (Priority 5..=6)
        add(self.body_armor, "Armor2", "Body Armor", false, 6);
        add(self.thigh_pads, "ThighPads", "Thigh Pads", false, 5);
        add(self.jump_boots, "UT_JumpBoots", "Anti-Grav Jump Boots", false, 5);
        add(self.invisibility, "UT_Invisibility", "Invisibility", false, 5);

        // 5. Baseline Health Restoratives (Priority 2..=3)
        add(self.medbox, "MedBox", "MedBox", false, 3);
        add(self.health_vials, "HealthVial", "Health Vial", false, 2);

        entries
    }
}

/// Looks up the paired ammunition class for any given weapon class name.
pub fn ammo_for_weapon(weapon_class: &str) -> Option<&'static str> {
    let short = weapon_class.trim().rsplit('.').next().unwrap_or(weapon_class.trim());
    let lower = short.to_ascii_lowercase();
    for w in STOCK_TOURNAMENT_WEAPONS.iter().chain(CLASSIC_UNREAL_WEAPONS.iter()) {
        if w.class_name.to_ascii_lowercase() == lower || w.id.to_ascii_lowercase() == lower {
            return w.ammo_class;
        }
    }
    None
}

/// Finds a weapon descriptor by class name or ID.
pub fn find_weapon(class_name: &str) -> Option<&'static WeaponDescriptor> {
    let short = class_name.trim().rsplit('.').next().unwrap_or(class_name.trim());
    let lower = short.to_ascii_lowercase();
    STOCK_TOURNAMENT_WEAPONS
        .iter()
        .chain(CLASSIC_UNREAL_WEAPONS.iter())
        .find(|w| w.class_name.to_ascii_lowercase() == lower || w.id.to_ascii_lowercase() == lower)
}

/// Finds a powerup descriptor by class name or ID.
pub fn find_powerup(class_name: &str) -> Option<&'static PowerupDescriptor> {
    let lower = class_name.trim().to_ascii_lowercase();
    STOCK_POWERUPS_AND_PICKUPS
        .iter()
        .find(|p| p.class_name.to_ascii_lowercase() == lower || p.id.to_ascii_lowercase() == lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_armory_catalogs_and_pairings() {
        assert_eq!(STOCK_TOURNAMENT_WEAPONS.len(), 11);
        assert_eq!(CLASSIC_UNREAL_WEAPONS.len(), 9);
        assert_eq!(STOCK_POWERUPS_AND_PICKUPS.len(), 9);

        assert_eq!(ammo_for_weapon("ShockRifle"), Some("ShockCore"));
        assert_eq!(ammo_for_weapon("UT_FlakCannon"), Some("FlakAmmo"));
        assert_eq!(ammo_for_weapon("UT_Eightball"), Some("RocketPack"));
        assert_eq!(ammo_for_weapon("PulseGun"), Some("PAmmo"));
        assert_eq!(ammo_for_weapon("SniperRifle"), Some("BulletBox"));
        assert_eq!(ammo_for_weapon("minigun2"), Some("MiniAmmo"));
        assert_eq!(ammo_for_weapon("WarHeadLauncher"), Some("WarHeadAmmo"));
        assert_eq!(ammo_for_weapon("ChainSaw"), None);

        // Classic weapon lookup
        assert_eq!(ammo_for_weapon("ASMD"), Some("ASMDAmmo"));
        assert_eq!(ammo_for_weapon("Eightball"), Some("RocketCan"));
        assert_eq!(ammo_for_weapon("FlakCannon"), Some("FlakBox"));
    }

    #[test]
    fn test_armory_presets() {
        for preset in WizardWeaponPreset::all() {
            let weapons = preset.default_weapons();
            assert!(!weapons.is_empty(), "Preset {:?} has empty loadout", preset);
            for w in &weapons {
                assert!(find_weapon(w).is_some(), "Weapon '{}' from preset {:?} not found in catalog", w, preset);
            }
        }

        for preset in WizardPowerupPreset::all() {
            let powerups = preset.default_powerups();
            assert!(!powerups.is_empty(), "Preset {:?} has empty loadout", preset);
            for p in &powerups {
                assert!(find_powerup(p).is_some(), "Powerup '{}' from preset {:?} not found in catalog", p, preset);
            }
        }
    }

    #[test]
    fn test_item_quota_config() {
        let def = WizardItemQuotaConfig::default();
        assert_eq!(def.total_weapons(), 15);
        assert_eq!(def.total_armor(), 5);
        assert_eq!(def.total_health(), 13);
        assert_eq!(def.total_powerups(), 4);
        assert_eq!(def.total_items(), 37);

        let entries = def.to_quota_entries();
        assert_eq!(entries.len(), def.total_items() as usize);

        // Redeemer has highest priority (10)
        let redeemer = entries.iter().find(|e| e.class_name == "WarHeadLauncher").unwrap();
        assert_eq!(redeemer.priority, 10);
        assert!(redeemer.is_weapon);

        // Shield Belt has priority 9
        let shield = entries.iter().find(|e| e.class_name == "UT_ShieldBelt").unwrap();
        assert_eq!(shield.priority, 9);
        assert!(!shield.is_weapon);

        let mut comp = WizardItemQuotaConfig::competitive_tournament();
        assert!(comp.enabled);
        assert_eq!(comp.redeemer, 0);
        assert!(comp.total_weapons() >= 8);

        comp.clear_all();
        assert_eq!(comp.total_items(), 0);
        assert!(comp.to_quota_entries().is_empty());
    }
}
