// crates/ut99-editor/tests/test_ctf_studio.rs
//
// Integration tests for Interactive Capture The Flag Base & Transit Routes Studio.

use glam::Vec3;
use ut99_bsp::{CTFTeam, CtfBaseArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_ctf_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is FaceMonolithBase
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::FaceMonolithBase
    );
    assert_eq!(
        state.ctf_base_settings.active_config.team,
        CTFTeam::Red
    );
    assert_eq!(
        state.ctf_base_settings.active_config.defense_radius_uu,
        1200.0
    );

    // Apply NovemberSubmarineDeck
    state.apply_ctf_base_archetype(CtfBaseArchetype::NovemberSubmarineDeck);
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::NovemberSubmarineDeck
    );
    assert_eq!(
        state.ctf_base_settings.active_config.team,
        CTFTeam::Blue
    );

    // Apply CoretCryoChamber
    state.apply_ctf_base_archetype(CtfBaseArchetype::CoretCryoChamber);
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::CoretCryoChamber
    );

    // Apply GauntletFortressDais
    state.apply_ctf_base_archetype(CtfBaseArchetype::GauntletFortressDais);
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::GauntletFortressDais
    );

    // Apply DrearyIndustrialGutter
    state.apply_ctf_base_archetype(CtfBaseArchetype::DrearyIndustrialGutter);
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::DrearyIndustrialGutter
    );

    // Undo back to Gauntlet
    state.undo();
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::GauntletFortressDais
    );

    // Redo to Dreary
    state.redo();
    assert_eq!(
        state.ctf_base_settings.active_config.archetype,
        CtfBaseArchetype::DrearyIndustrialGutter
    );
}

#[test]
fn test_ctf_base_metrics_and_transit() {
    let mut state = EditorState::default();

    // Populate level with an existing Blue FlagBase
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut blue_flag = ut99_format::ActorRecord::default();
        blue_flag.name = "BlueFlagBase_0".to_string();
        blue_flag.class_name = "Botpack.BlueFlagBase".to_string();
        blue_flag.location = Vec3::new(3200.0, 0.0, 0.0);
        level.actors.push(blue_flag);
    }

    state.recalculate_ctf_base_preview();
    let metrics = &state.ctf_base_settings.cached_metrics;

    assert_eq!(metrics.total_flag_bases_in_level, 2);
    assert_eq!(metrics.flag_to_flag_distance_uu, 3200.0);
    assert_eq!(metrics.runner_transit_time_secs, 8.0); // 3200 / 400 = 8.0s
    assert_eq!(metrics.translocator_transit_time_secs, 4.0); // 3200 / 800 = 4.0s
}

#[test]
fn test_ctf_base_hazard_auditing() {
    let mut state = EditorState::default();

    state.ctf_base_settings.active_config.team = CTFTeam::Red;
    state.ctf_base_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);

    // 1. Missing opposing base hazard when map is empty
    state.recalculate_ctf_base_preview();
    assert!(state.ctf_base_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Missing Opposing")));

    // 2. Add an opposing base too close (<800 uu)
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        let mut blue_flag = ut99_format::ActorRecord::default();
        blue_flag.name = "BlueFlagBase_0".to_string();
        blue_flag.class_name = "Botpack.BlueFlagBase".to_string();
        blue_flag.location = Vec3::new(450.0, 0.0, 0.0); // Instant cap!
        level.actors.push(blue_flag);
    }

    state.recalculate_ctf_base_preview();
    assert!(state.ctf_base_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Instant-Cap")));
}

#[test]
fn test_commit_ctf_base_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Flag Base
    state.apply_ctf_base_archetype(CtfBaseArchetype::FaceMonolithBase);
    let (brush_idx, actor_idx) = state.commit_ctf_base_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.RedFlagBase");
    assert_eq!(act.group, "CTF");
    assert_eq!(act.tag, "RedFlagBase");

    // 2. Undo Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // 3. Redo Commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 1);
    assert_eq!(level_redone.actors[0].class_name, "Botpack.RedFlagBase");
}
