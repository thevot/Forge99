//! Builds procedural wizard maps with the real UT engine for external measurement.
//! Usage: build_wizard_matrix <ut_dir> <out_dir> <generator_version> <seeds>
use std::path::PathBuf;
use ut99_bsp::{WizardGameMode, WizardLayoutStrategy};
use ut99_editor::compiler::CompilerBridge;
use ut99_editor::editor_state::EditorState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ut_dir = PathBuf::from(&args[1]);
    let out_dir = PathBuf::from(&args[2]);
    let version: u32 = args[3].parse().unwrap();
    let seeds: u64 = args[4].parse().unwrap();
    std::fs::create_dir_all(&out_dir).unwrap();
    for (mode, prefix) in [(WizardGameMode::DeathMatch, "DM"), (WizardGameMode::CaptureTheFlag, "CTF")] {
        for seed in 1..=seeds {
            let mut state = EditorState::default();
            let cfg = &mut state.map_wizard_config;
            cfg.game_mode = mode;
            cfg.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
            cfg.seed = seed;
            cfg.generator_version = version;
            cfg.sync_mode_conventions();
            cfg.auto_run_build_and_audit = false;
            state.generate_map_from_wizard();
            let out = out_dir.join(format!("{prefix}-Probe{version}s{seed}.unr"));
            match CompilerBridge::build_unr_with_engine(&state, &ut_dir, &out) {
                Ok(_) => println!("ok {}", out.display()),
                Err(e) => println!("FAIL {} {e}", out.display()),
            }
        }
    }
}
