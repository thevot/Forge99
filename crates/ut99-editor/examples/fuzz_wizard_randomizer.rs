//! Repeats the one-click randomizer roll of `test_wizard_one_click_full_randomizer_zero_errors` and prints every
//! roll that yields Level Doctor errors, with the full config as JSON.
//! Usage: fuzz_wizard_randomizer <rolls>
use ut99_bsp::IssueSeverity;
use ut99_editor::editor_state::EditorState;

fn main() {
    let rolls: usize = std::env::args().nth(1).and_then(|v| v.parse().ok()).unwrap_or(100);
    let mut state = EditorState::default();
    let mut failures = 0;
    for i in 0..rolls {
        state.randomize_all_wizard_settings();
        // FORGE99_FUZZ_DOM_ARENA=1 pins the class of the second known failure (canonical DOM central arena).
        if std::env::var_os("FORGE99_FUZZ_DOM_ARENA").is_some() {
            let c = &mut state.map_wizard_config;
            c.game_mode = ut99_bsp::WizardGameMode::Domination;
            c.layout_strategy = ut99_bsp::WizardLayoutStrategy::CanonicalArchetype;
            c.archetype = ut99_bsp::WizardArchetype::CentralArenaMultiTier;
            c.sync_mode_conventions();
        }
        let (b, _m, a) = state.generate_map_from_wizard();
        let errors: Vec<_> = state.map_audit_issues.iter().filter(|x| x.severity == IssueSeverity::Error).collect();
        if !errors.is_empty() || b < 3 || a < 10 {
            failures += 1;
            println!("FAIL roll {i}: brushes={b} actors={a} errors={:?}", errors.iter().map(|e| format!("{}:{}", e.object_name, e.description)).collect::<Vec<_>>());
            println!("CONFIG {}", serde_json::to_string(&state.map_wizard_config).unwrap());
        }
    }
    println!("done: {failures} failing of {rolls}");
}
