// crates/ut99-bsp/src/arena_rules.rs
//
// Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
//
// Native 64-bit mathematical modeling, mutator conflict resolution,
// weapon-to-player ratio telemetry, and tournament competitive balance auditing
// for Unreal Tournament 99 / UnrealEd 2.0 / 2.1 supremacy.

/// Supported Unreal Tournament 99 game types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GameTypeProfile {
    DeathMatchPlus,
    TeamGamePlus,
    CTFGame,
    Domination,
    Assault,
}

impl GameTypeProfile {
    pub fn display_name(&self) -> &'static str {
        match self {
            GameTypeProfile::DeathMatchPlus => "DeathMatchPlus (FFA)",
            GameTypeProfile::TeamGamePlus => "TeamGamePlus (TDM)",
            GameTypeProfile::CTFGame => "CTFGame (Capture The Flag)",
            GameTypeProfile::Domination => "Domination (DOM Control)",
            GameTypeProfile::Assault => "Assault (AS Objective)",
        }
    }

    pub fn class_path(&self) -> &'static str {
        match self {
            GameTypeProfile::DeathMatchPlus => "Botpack.DeathMatchPlus",
            GameTypeProfile::TeamGamePlus => "Botpack.TeamGamePlus",
            GameTypeProfile::CTFGame => "Botpack.CTFGame",
            GameTypeProfile::Domination => "Botpack.Domination",
            GameTypeProfile::Assault => "Botpack.Assault",
        }
    }
}

/// Official and community tournament mutators for UT99
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TournamentMutator {
    InstagibDM,
    RocketArena,
    FlakArena,
    ShockArena,
    SniperArena,
    LowGrav,
    NoPowerups,
    FatBoy,
    Chameleon,
    Relics,
}

impl TournamentMutator {
    pub fn display_name(&self) -> &'static str {
        match self {
            TournamentMutator::InstagibDM => "Instagib DM",
            TournamentMutator::RocketArena => "Rocket Arena",
            TournamentMutator::FlakArena => "Flak Arena",
            TournamentMutator::ShockArena => "Shock Arena",
            TournamentMutator::SniperArena => "Sniper Arena",
            TournamentMutator::LowGrav => "Low Grav (200 uu/s²)",
            TournamentMutator::NoPowerups => "No Powerups / Armor",
            TournamentMutator::FatBoy => "FatBoy",
            TournamentMutator::Chameleon => "Chameleon Skins",
            TournamentMutator::Relics => "Relics of Power",
        }
    }

    pub fn is_weapon_arena(&self) -> bool {
        matches!(
            self,
            TournamentMutator::InstagibDM
                | TournamentMutator::RocketArena
                | TournamentMutator::FlakArena
                | TournamentMutator::ShockArena
                | TournamentMutator::SniperArena
        )
    }
}

/// Pre-calibrated tournament match archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArenaRulesArchetype {
    ClassicProTournament1v1,
    InstagibShockShowdown,
    RocketArenaHeavySiege,
    LowGravSniperExtravaganza,
    TeamCaptureTheFlagMasters,
}

impl ArenaRulesArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            ArenaRulesArchetype::ClassicProTournament1v1 => "Classic Pro Tournament 1v1",
            ArenaRulesArchetype::InstagibShockShowdown => "Instagib Shock Showdown",
            ArenaRulesArchetype::RocketArenaHeavySiege => "Rocket Arena Heavy Siege",
            ArenaRulesArchetype::LowGravSniperExtravaganza => "LowGrav Sniper Extravaganza",
            ArenaRulesArchetype::TeamCaptureTheFlagMasters => "Team Capture The Flag Masters",
        }
    }

    pub fn to_config(&self) -> ArenaRulesConfig {
        match self {
            ArenaRulesArchetype::ClassicProTournament1v1 => ArenaRulesConfig {
                archetype: *self,
                game_type: GameTypeProfile::DeathMatchPlus,
                frag_limit: 15,
                time_limit_mins: 10,
                max_players: 2,
                spawn_protection_secs: 0.0,
                active_mutators: vec![],
                weapon_respawn_secs: 27.0,
                powerup_respawn_secs: 110.0,
                armor_respawn_secs: 55.0,
                air_control: 0.35,
                gravity_uu_s2: 950.0,
                friendly_fire_scale: 0.5,
                tournament_mode: true,
                tag: "RulesPro1v1".to_string(),
            },
            ArenaRulesArchetype::InstagibShockShowdown => ArenaRulesConfig {
                archetype: *self,
                game_type: GameTypeProfile::DeathMatchPlus,
                frag_limit: 30,
                time_limit_mins: 15,
                max_players: 8,
                spawn_protection_secs: 1.0,
                active_mutators: vec![TournamentMutator::InstagibDM, TournamentMutator::NoPowerups],
                weapon_respawn_secs: 0.0,
                powerup_respawn_secs: 0.0,
                armor_respawn_secs: 0.0,
                air_control: 0.40,
                gravity_uu_s2: 950.0,
                friendly_fire_scale: 0.0,
                tournament_mode: true,
                tag: "RulesInstagib".to_string(),
            },
            ArenaRulesArchetype::RocketArenaHeavySiege => ArenaRulesConfig {
                archetype: *self,
                game_type: GameTypeProfile::TeamGamePlus,
                frag_limit: 50,
                time_limit_mins: 20,
                max_players: 10,
                spawn_protection_secs: 2.0,
                active_mutators: vec![TournamentMutator::RocketArena],
                weapon_respawn_secs: 10.0,
                powerup_respawn_secs: 60.0,
                armor_respawn_secs: 30.0,
                air_control: 0.45,
                gravity_uu_s2: 900.0,
                friendly_fire_scale: 0.0,
                tournament_mode: true,
                tag: "RulesRocketArena".to_string(),
            },
            ArenaRulesArchetype::LowGravSniperExtravaganza => ArenaRulesConfig {
                archetype: *self,
                game_type: GameTypeProfile::DeathMatchPlus,
                frag_limit: 25,
                time_limit_mins: 15,
                max_players: 12,
                spawn_protection_secs: 1.5,
                active_mutators: vec![TournamentMutator::SniperArena, TournamentMutator::LowGrav],
                weapon_respawn_secs: 15.0,
                powerup_respawn_secs: 90.0,
                armor_respawn_secs: 45.0,
                air_control: 0.60,
                gravity_uu_s2: 200.0,
                friendly_fire_scale: 0.0,
                tournament_mode: true,
                tag: "RulesLowGravSniper".to_string(),
            },
            ArenaRulesArchetype::TeamCaptureTheFlagMasters => ArenaRulesConfig {
                archetype: *self,
                game_type: GameTypeProfile::CTFGame,
                frag_limit: 3, // 3 flag captures
                time_limit_mins: 20,
                max_players: 10,
                spawn_protection_secs: 2.5,
                active_mutators: vec![],
                weapon_respawn_secs: 27.0,
                powerup_respawn_secs: 110.0,
                armor_respawn_secs: 55.0,
                air_control: 0.35,
                gravity_uu_s2: 950.0,
                friendly_fire_scale: 0.5,
                tournament_mode: true,
                tag: "RulesCTFMasters".to_string(),
            },
        }
    }
}

/// Comprehensive match rules and mutator configuration
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArenaRulesConfig {
    pub archetype: ArenaRulesArchetype,
    pub game_type: GameTypeProfile,
    pub frag_limit: u32,
    pub time_limit_mins: u32,
    pub max_players: u32,
    pub spawn_protection_secs: f32,
    pub active_mutators: Vec<TournamentMutator>,
    pub weapon_respawn_secs: f32,
    pub powerup_respawn_secs: f32,
    pub armor_respawn_secs: f32,
    pub air_control: f32,
    pub gravity_uu_s2: f32,
    pub friendly_fire_scale: f32,
    pub tournament_mode: bool,
    pub tag: String,
}

impl Default for ArenaRulesConfig {
    fn default() -> Self {
        ArenaRulesArchetype::ClassicProTournament1v1.to_config()
    }
}

/// Analytical metrics of arena match rules
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaRulesMetrics {
    pub weapons_per_player: f32,
    pub ammo_to_weapon_ratio: f32,
    pub estimated_first_engagement_ttk_secs: f32,
    pub tournament_readiness_score: f32, // 0.0 to 100.0%
    pub active_mutator_count: usize,
}

/// Diagnostic health issues for arena rules
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaHazard {
    pub title: String,
    pub description: String,
    pub is_critical: bool,
}

pub type ArenaRulesHazard = ArenaHazard;

/// Evaluates competitive metrics and balance readiness
pub fn evaluate_arena_rules_metrics(
    config: &ArenaRulesConfig,
    world_weapon_count: usize,
    world_ammo_count: usize,
) -> ArenaRulesMetrics {
    let players = config.max_players.max(1) as f32;
    let weapons_per_player = world_weapon_count as f32 / players;
    let ammo_to_weapon_ratio = if world_weapon_count > 0 {
        world_ammo_count as f32 / world_weapon_count as f32
    } else {
        0.0
    };

    // TTK estimation: Instagib = 0.2s, Snipers = 1.0s, Rocket = 1.8s, Standard = 2.8s
    let estimated_first_engagement_ttk_secs = if config.active_mutators.contains(&TournamentMutator::InstagibDM) {
        0.2
    } else if config.active_mutators.contains(&TournamentMutator::SniperArena) {
        1.1
    } else if config.active_mutators.contains(&TournamentMutator::RocketArena) {
        1.7
    } else {
        2.8
    };

    // Balance readiness score calculation
    let mut score = 85.0_f32;
    if config.tournament_mode {
        score += 10.0;
    }
    if config.frag_limit == 0 && config.time_limit_mins == 0 {
        score -= 40.0; // Infinite match penalty
    }
    if !config.active_mutators.contains(&TournamentMutator::InstagibDM) && weapons_per_player < 0.8 {
        score -= 20.0; // Weapon starvation penalty
    }

    ArenaRulesMetrics {
        weapons_per_player,
        ammo_to_weapon_ratio,
        estimated_first_engagement_ttk_secs,
        tournament_readiness_score: score.clamp(0.0, 100.0),
        active_mutator_count: config.active_mutators.len(),
    }
}

/// Audits safety and conflicts in the arena rules
pub fn audit_arena_rules_safety(
    config: &ArenaRulesConfig,
    world_weapon_count: usize,
    world_ammo_count: usize,
) -> Vec<ArenaHazard> {
    let mut hazards = Vec::new();

    // 1. Infinite match deadlock (both frag_limit and time_limit are 0)
    if config.frag_limit == 0 && config.time_limit_mins == 0 {
        hazards.push(ArenaHazard {
            title: "Infinite Match Deadlock Hazard".to_string(),
            description: "Both FragLimit and TimeLimit are set to 0. Match will never terminate normally.".to_string(),
            is_critical: true,
        });
    }

    // 2. Conflicting weapon arenas (e.g. Instagib + RocketArena)
    let arena_count = config.active_mutators.iter().filter(|m| m.is_weapon_arena()).count();
    if arena_count > 1 {
        hazards.push(ArenaHazard {
            title: "Conflicting Weapon Arena Mutators".to_string(),
            description: format!(
                "Multiple weapon arena mutators ({}) are active simultaneously, causing spawn weapon conflicts.",
                arena_count
            ),
            is_critical: true,
        });
    }

    // 3. Weapon starvation in standard modes
    if !config.active_mutators.contains(&TournamentMutator::InstagibDM) {
        let players = config.max_players.max(1) as f32;
        if (world_weapon_count as f32) / players < 0.5 {
            hazards.push(ArenaHazard {
                title: "Severe Weapon Starvation Hazard".to_string(),
                description: format!(
                    "Level has {} weapon pickups for {} max players (< 0.5 per player). Players will have no weapons.",
                    world_weapon_count, config.max_players
                ),
                is_critical: true,
            });
        }

        // 4. Low ammo ratio
        if world_weapon_count > 0 && (world_ammo_count as f32 / world_weapon_count as f32) < 1.0 {
            hazards.push(ArenaHazard {
                title: "Depleted Ammo Supply Ratio".to_string(),
                description: format!(
                    "Level ammo/weapon ratio is {:.2} (< 1.0). Players will run out of ammunition rapidly.",
                    world_ammo_count as f32 / world_weapon_count as f32
                ),
                is_critical: false,
            });
        }
    }

    // 5. Extreme zero gravity or inverted gravity
    if config.gravity_uu_s2 <= 0.0 {
        hazards.push(ArenaHazard {
            title: "Zero or Negative World Gravity".to_string(),
            description: "World gravity is <= 0 uu/s². Players and ragdolls will float uncontrollably into the sky.".to_string(),
            is_critical: true,
        });
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_rules_archetypes_and_defaults() {
        let pro = ArenaRulesArchetype::ClassicProTournament1v1.to_config();
        assert_eq!(pro.game_type, GameTypeProfile::DeathMatchPlus);
        assert_eq!(pro.frag_limit, 15);
        assert_eq!(pro.max_players, 2);
        assert!(pro.tournament_mode);

        let insta = ArenaRulesArchetype::InstagibShockShowdown.to_config();
        assert_eq!(insta.game_type, GameTypeProfile::DeathMatchPlus);
        assert!(insta.active_mutators.contains(&TournamentMutator::InstagibDM));

        let ctf = ArenaRulesArchetype::TeamCaptureTheFlagMasters.to_config();
        assert_eq!(ctf.game_type, GameTypeProfile::CTFGame);
        assert_eq!(ctf.frag_limit, 3);
    }

    #[test]
    fn test_evaluate_arena_rules_metrics() {
        let pro = ArenaRulesArchetype::ClassicProTournament1v1.to_config();
        let m = evaluate_arena_rules_metrics(&pro, 4, 12);

        assert_eq!(m.weapons_per_player, 2.0);
        assert_eq!(m.ammo_to_weapon_ratio, 3.0);
        assert!(m.tournament_readiness_score >= 90.0);
        assert!(m.estimated_first_engagement_ttk_secs > 2.0);
    }

    #[test]
    fn test_audit_arena_rules_safety() {
        let mut cfg = ArenaRulesConfig::default();

        // 1. Deadlock match
        cfg.frag_limit = 0;
        cfg.time_limit_mins = 0;
        let h1 = audit_arena_rules_safety(&cfg, 4, 8);
        assert!(h1.iter().any(|h| h.is_critical && h.title.contains("Deadlock")));

        // 2. Conflicting mutators
        cfg.frag_limit = 20;
        cfg.active_mutators = vec![TournamentMutator::InstagibDM, TournamentMutator::RocketArena];
        let h2 = audit_arena_rules_safety(&cfg, 4, 8);
        assert!(h2.iter().any(|h| h.is_critical && h.title.contains("Conflicting Weapon Arena")));

        // 3. Weapon starvation
        cfg.active_mutators.clear();
        cfg.max_players = 16;
        let h3 = audit_arena_rules_safety(&cfg, 2, 4);
        assert!(h3.iter().any(|h| h.is_critical && h.title.contains("Weapon Starvation")));
    }
}
