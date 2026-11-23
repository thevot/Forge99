// crates/ut99-editor/tests/test_arena_rules_studio.rs
//
// Integration tests for Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio.

use ut99_bsp::{ArenaRulesArchetype, GameTypeProfile, TournamentMutator};
use ut99_editor::editor_state::EditorState;
use ut99_format::ActorRecord;

#[test]
fn test_arena_rules_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is ClassicProTournament1v1
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::ClassicProTournament1v1
    );
    assert_eq!(
        state.arena_rules_settings.active_config.game_type,
        GameTypeProfile::DeathMatchPlus
    );
    assert_eq!(state.arena_rules_settings.active_config.frag_limit, 15);
    assert_eq!(state.arena_rules_settings.active_config.max_players, 2);
    assert!(state.arena_rules_settings.active_config.tournament_mode);

    // Apply InstagibShockShowdown
    state.apply_arena_rules_archetype(ArenaRulesArchetype::InstagibShockShowdown);
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::InstagibShockShowdown
    );
    assert!(state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::InstagibDM));
    assert!(state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::NoPowerups));

    // Apply RocketArenaHeavySiege
    state.apply_arena_rules_archetype(ArenaRulesArchetype::RocketArenaHeavySiege);
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::RocketArenaHeavySiege
    );
    assert_eq!(
        state.arena_rules_settings.active_config.game_type,
        GameTypeProfile::TeamGamePlus
    );
    assert_eq!(state.arena_rules_settings.active_config.frag_limit, 50);

    // Apply LowGravSniperExtravaganza
    state.apply_arena_rules_archetype(ArenaRulesArchetype::LowGravSniperExtravaganza);
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::LowGravSniperExtravaganza
    );
    assert!(state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::SniperArena));
    assert!(state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::LowGrav));
    assert_eq!(state.arena_rules_settings.active_config.gravity_uu_s2, 200.0);

    // Apply TeamCaptureTheFlagMasters
    state.apply_arena_rules_archetype(ArenaRulesArchetype::TeamCaptureTheFlagMasters);
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::TeamCaptureTheFlagMasters
    );
    assert_eq!(
        state.arena_rules_settings.active_config.game_type,
        GameTypeProfile::CTFGame
    );
    assert_eq!(state.arena_rules_settings.active_config.frag_limit, 3);

    // Toggle mutator: Relics
    state.toggle_arena_mutator(TournamentMutator::Relics);
    assert!(state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::Relics));

    // Undo mutator toggle
    state.undo();
    assert!(!state.arena_rules_settings.active_config.active_mutators.contains(&TournamentMutator::Relics));

    // Undo archetype (returns to LowGravSniperExtravaganza)
    state.undo();
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::LowGravSniperExtravaganza
    );

    // Redo returns to TeamCaptureTheFlagMasters
    state.redo();
    assert_eq!(
        state.arena_rules_settings.active_config.archetype,
        ArenaRulesArchetype::TeamCaptureTheFlagMasters
    );
}

#[test]
fn test_arena_rules_telemetry_and_pickup_metrics() {
    let mut state = EditorState::default();

    // Populate level with weapons and ammo
    let mut unr = ut99_format::UnrLevel::default();
    for i in 0..4 {
        let mut w = ActorRecord::default();
        w.name = format!("UT_ShockRifle_{}", i);
        w.class_name = "Botpack.ShockRifle".to_string();
        unr.actors.push(w);
    }
    for i in 0..12 {
        let mut a = ActorRecord::default();
        a.name = format!("ShockCore_{}", i);
        a.class_name = "Botpack.ShockCore".to_string();
        unr.actors.push(a);
    }
    state.loaded_unr = Some(unr);

    // Apply ClassicProTournament1v1 (2 players)
    state.apply_arena_rules_archetype(ArenaRulesArchetype::ClassicProTournament1v1);
    state.recalculate_arena_rules_preview();

    assert_eq!(state.arena_rules_settings.world_weapon_count, 4);
    assert_eq!(state.arena_rules_settings.world_ammo_count, 12);

    let m = &state.arena_rules_settings.cached_metrics;
    assert_eq!(m.weapons_per_player, 2.0); // 4 weapons / 2 players
    assert_eq!(m.ammo_to_weapon_ratio, 3.0); // 12 ammo / 4 weapons
    assert!(m.tournament_readiness_score >= 90.0);
    assert_eq!(m.estimated_first_engagement_ttk_secs, 2.8);

    // Switch to Instagib: estimated TTK should drop to 0.2s
    state.apply_arena_rules_archetype(ArenaRulesArchetype::InstagibShockShowdown);
    assert_eq!(state.arena_rules_settings.cached_metrics.estimated_first_engagement_ttk_secs, 0.2);
}

#[test]
fn test_arena_rules_safety_audit_and_conflict_detection() {
    let mut state = EditorState::default();

    // Populate level with adequate weapons and ammo for 2 players
    let mut unr = ut99_format::UnrLevel::default();
    for i in 0..2 {
        let mut w = ActorRecord::default();
        w.name = format!("ShockRifle_{}", i);
        w.class_name = "Botpack.ShockRifle".to_string();
        unr.actors.push(w);
    }
    for i in 0..4 {
        let mut a = ActorRecord::default();
        a.name = format!("ShockCore_{}", i);
        a.class_name = "Botpack.ShockCore".to_string();
        unr.actors.push(a);
    }
    state.loaded_unr = Some(unr);

    // 1. Initial configuration should be 100% clean
    state.apply_arena_rules_archetype(ArenaRulesArchetype::ClassicProTournament1v1);
    assert!(state.arena_rules_settings.cached_hazards.is_empty());

    // 2. Deadlock match (both 0)
    state.arena_rules_settings.active_config.frag_limit = 0;
    state.arena_rules_settings.active_config.time_limit_mins = 0;
    state.recalculate_arena_rules_preview();
    assert!(state.arena_rules_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Deadlock")));

    // 3. Mutator conflict: Instagib + RocketArena
    state.arena_rules_settings.active_config.frag_limit = 20;
    state.arena_rules_settings.active_config.active_mutators = vec![TournamentMutator::InstagibDM, TournamentMutator::RocketArena];
    state.recalculate_arena_rules_preview();
    assert!(state.arena_rules_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Conflicting Weapon Arena")));

    // 4. Inverted/zero gravity hazard
    state.arena_rules_settings.active_config.active_mutators.clear();
    state.arena_rules_settings.active_config.gravity_uu_s2 = -50.0;
    state.recalculate_arena_rules_preview();
    assert!(state.arena_rules_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Zero or Negative World Gravity")));
}

#[test]
fn test_commit_arena_rules_to_world_and_undo_redo() {
    let mut state = EditorState::default();

    // Setup initial LevelInfo in UNR
    let mut unr = ut99_format::UnrLevel::default();
    let mut initial_info = ActorRecord::default();
    initial_info.name = "LevelInfo0".to_string();
    initial_info.class_name = "Engine.LevelInfo".to_string();
    initial_info.tag = "OriginalTag".to_string();
    unr.actors.push(initial_info);
    state.loaded_unr = Some(unr);
    state.level_game_type = "Botpack.DeathMatchPlus".to_string();

    // Configure RocketArenaHeavySiege (TeamGamePlus, 1 mutator)
    state.arena_rules_settings.active_config = ArenaRulesArchetype::RocketArenaHeavySiege.to_config();

    let (info_idx, mutators) = state.commit_arena_rules_to_world();
    assert_eq!(info_idx, Some(0));
    assert_eq!(mutators.len(), 1);

    // Verify state synchronizations
    assert_eq!(state.level_game_type, "Botpack.TeamGamePlus");
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors[0].tag, "RulesRocketArena");
    assert_eq!(level.actors[mutators[0]].class_name, "Botpack.RocketArena");
    assert_eq!(level.actors[mutators[0]].group, "ArenaMutators");

    // Undo commit
    state.undo();
    assert_eq!(state.level_game_type, "Botpack.DeathMatchPlus");
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 1); // Mutator removed
    assert_eq!(level_undone.actors[0].tag, "OriginalTag");

    // Redo commit
    state.redo();
    assert_eq!(state.level_game_type, "Botpack.TeamGamePlus");
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 2); // Mutator restored
    assert_eq!(level_redone.actors[1].class_name, "Botpack.RocketArena");
}
