use ut99_bsp::armory::{WizardPowerupPreset, WizardWeaponPreset};
use ut99_bsp::{WizardArchetype, WizardGameMode, WizardLayoutStrategy, WizardThemePreset};
use ut99_editor::compiler::CompilerBridge;
use ut99_editor::editor_state::EditorState;

fn main() {
    let ut_dir = CompilerBridge::find_ut_dir().expect("UT99 dir not found");
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = WizardGameMode::DeathMatch;
    state.map_wizard_config.layout_strategy = WizardLayoutStrategy::CanonicalArchetype;
    state.map_wizard_config.archetype = WizardArchetype::InterconnectedComplex;
    state.map_wizard_config.theme = WizardThemePreset::Deck16Industrial;
    state.map_wizard_config.seed = 42;
    state.map_wizard_config.map_name = "DM-HeavyArsenal.unr".to_string();
    state.map_wizard_config.title = "Heavy Arsenal Complex".to_string();
    state.map_wizard_config.author = "Forge99 Map Wizard".to_string();
    state.map_wizard_config.apply_weapon_preset(WizardWeaponPreset::HeavyExplosives);
    state.map_wizard_config.apply_powerup_preset(WizardPowerupPreset::MayhemOverkill);
    state.map_wizard_config.sync_mode_conventions();
    state.map_wizard_config.auto_run_build_and_audit = false;
    state.generate_map_from_wizard();

    let dest = ut_dir.join("Maps").join("DM-HeavyArsenal.unr");
    match CompilerBridge::build_unr_with_engine(&state, &ut_dir, &dest) {
        Ok(p) => println!("Successfully built and compiled: {:?}", p),
        Err(e) => eprintln!("Engine build error: {e}"),
    }
}
