// crates/ut99-editor/tests/test_powerup_control_studio.rs
//
// Integration tests for Interactive Tournament Powerup Control & Relic Altar Studio.

use glam::Vec3;
use ut99_bsp::{PowerupArchetype, TournamentPowerupType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_powerup_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is Deck16HighLedgeUDamage
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::Deck16HighLedgeUDamage
    );
    assert_eq!(
        state.powerup_control_settings.active_config.powerup_type,
        TournamentPowerupType::UDamage
    );
    assert!(state.powerup_control_settings.active_config.b_require_jump_skills);

    // Apply LiandriCoreCrossfireUDamage
    state.apply_powerup_archetype(PowerupArchetype::LiandriCoreCrossfireUDamage);
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::LiandriCoreCrossfireUDamage
    );
    assert_eq!(
        state.powerup_control_settings.active_config.powerup_type,
        TournamentPowerupType::UDamage
    );

    // Apply PhobosGravityJumpBoots
    state.apply_powerup_archetype(PowerupArchetype::PhobosGravityJumpBoots);
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::PhobosGravityJumpBoots
    );
    assert_eq!(
        state.powerup_control_settings.active_config.powerup_type,
        TournamentPowerupType::JumpBoots
    );

    // Apply AgonySacredRelicAltar
    state.apply_powerup_archetype(PowerupArchetype::AgonySacredRelicAltar);
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::AgonySacredRelicAltar
    );
    assert_eq!(
        state.powerup_control_settings.active_config.powerup_type,
        TournamentPowerupType::RelicStrength
    );

    // Apply GothicDualPowerupClash
    state.apply_powerup_archetype(PowerupArchetype::GothicDualPowerupClash);
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::GothicDualPowerupClash
    );
    assert_eq!(
        state.powerup_control_settings.active_config.powerup_type,
        TournamentPowerupType::Invisibility
    );

    // Undo back to Agony
    state.undo();
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::AgonySacredRelicAltar
    );

    // Redo to Gothic
    state.redo();
    assert_eq!(
        state.powerup_control_settings.active_config.archetype,
        PowerupArchetype::GothicDualPowerupClash
    );
}

#[test]
fn test_powerup_metrics_and_contention_hotspot() {
    let mut state = EditorState::default();

    // Populate level with a weapon and shieldbelt
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    let level = state.loaded_unr.as_mut().unwrap();

    let mut w = ut99_format::ActorRecord::default();
    w.name = "Shock0".to_string();
    w.class_name = "Botpack.ShockRifle".to_string();
    w.location = Vec3::new(350.0, 0.0, 0.0);
    level.actors.push(w);

    let mut s = ut99_format::ActorRecord::default();
    s.name = "Shield0".to_string();
    s.class_name = "Botpack.UT_ShieldBelt".to_string();
    s.location = Vec3::new(1400.0, 0.0, 0.0);
    level.actors.push(s);

    state.recalculate_powerup_preview();
    let metrics = &state.powerup_control_settings.cached_metrics;

    assert!(metrics.powerup_uptime_pct > 0.0);
    assert_eq!(metrics.nearest_weapon_distance_uu, 350.0);
    assert_eq!(metrics.nearest_shield_distance_uu, 1400.0);
    assert!(metrics.contention_hotspot_rating >= 80.0);
}

#[test]
fn test_powerup_hazard_auditing() {
    let mut state = EditorState::default();

    // Setup an adjacent ShieldBelt at distance 300 (< 500 uu)
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    let level = state.loaded_unr.as_mut().unwrap();

    let mut s = ut99_format::ActorRecord::default();
    s.name = "SuperShield".to_string();
    s.class_name = "Botpack.UT_ShieldBelt".to_string();
    s.location = Vec3::new(300.0, 0.0, 0.0);
    level.actors.push(s);

    state.recalculate_powerup_preview();
    assert!(state.powerup_control_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Super-Stacking")));

    // Test Relic of Death explosive hazard
    state.powerup_control_settings.active_config.powerup_type = TournamentPowerupType::RelicDeath;
    state.recalculate_powerup_preview();
    assert!(state.powerup_control_settings.cached_hazards.iter().any(|h| h.title.contains("Relic of Death")));

    // Test Fast Respawn Risk
    state.powerup_control_settings.active_config.powerup_type = TournamentPowerupType::UDamage;
    state.powerup_control_settings.active_config.respawn_delay_secs = 45.0;
    state.recalculate_powerup_preview();
    assert!(state.powerup_control_settings.cached_hazards.iter().any(|h| h.title.contains("Fast UDamage Cadence")));
}

#[test]
fn test_commit_powerup_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit UDamage Altar
    state.apply_powerup_archetype(PowerupArchetype::Deck16HighLedgeUDamage);
    let (brush_idx, actor_idx) = state.commit_powerup_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(actor_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    let act = &level.actors[actor_idx.unwrap()];
    assert_eq!(act.class_name, "Botpack.UDamage");
    assert_eq!(act.group, "Powerups");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.UDamage");
}
