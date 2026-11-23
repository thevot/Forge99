use ut99_bsp::{IssueSeverity, WizardArchetype, WizardGameMode, WizardLayoutStrategy, WizardSymmetryMode, WizardThemePreset};
use ut99_editor::editor_state::EditorState;

/// Regression: a randomizer roll (found by `fuzz_wizard_randomizer`) left the stair foot waypoint
/// `PathNode_Foot_ProcCorridorX_7_ProcChamber_4` with no reach spec, which blocked export.
#[test]
fn orphan_stair_waypoint_does_not_block_export() {
    let mut state = EditorState::default();
    let c = &mut state.map_wizard_config;
    c.game_mode = WizardGameMode::DeathMatch;
    c.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
    c.archetype = WizardArchetype::OrbitalOutpostSkybox;
    c.seed = 6187019826780366788;
    c.procedural_room_count = 7;
    c.procedural_vertical_tiers = 3;
    c.procedural_loop_ratio = 0.55;
    c.symmetry_mode = WizardSymmetryMode::None;
    c.theme = WizardThemePreset::SesmarEgyptianTomb;
    c.sync_mode_conventions();
    state.generate_map_from_wizard();
    let errors: Vec<_> = state.map_audit_issues.iter().filter(|i| i.severity == IssueSeverity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!state.loaded_unr.as_ref().unwrap().actors.iter().any(|a| a.name == "PathNode_Foot_ProcCorridorX_7_ProcChamber_4"));
}
