// crates/ut99-editor/tests/test_soundscape_studio.rs
//
// Integration tests for Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio.

use ut99_bsp::{AcousticReverbPreset, AudioFalloffCurve, SoundscapeArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_soundscape_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is SubterraneanSewerEcho
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::SubterraneanSewerEcho
    );
    assert_eq!(state.soundscape_settings.active_config.emitter.sound_radius, 32);
    assert_eq!(
        state.soundscape_settings.active_config.acoustics.reverb_preset,
        AcousticReverbPreset::SewerPipe
    );

    // Apply IndustrialGeneratorDrone
    state.apply_soundscape_archetype(SoundscapeArchetype::IndustrialGeneratorDrone);
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::IndustrialGeneratorDrone
    );
    assert_eq!(state.soundscape_settings.active_config.emitter.sound_volume, 255);
    assert_eq!(
        state.soundscape_settings.active_config.emitter.falloff_curve,
        AudioFalloffCurve::InverseSquare
    );

    // Apply CathedralGothicVault
    state.apply_soundscape_archetype(SoundscapeArchetype::CathedralGothicVault);
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::CathedralGothicVault
    );
    assert_eq!(
        state.soundscape_settings.active_config.acoustics.reverb_preset,
        AcousticReverbPreset::Arena
    );

    // Apply HighTechComputerCore
    state.apply_soundscape_archetype(SoundscapeArchetype::HighTechComputerCore);
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::HighTechComputerCore
    );
    assert_eq!(state.soundscape_settings.active_config.emitter.sound_radius, 24);

    // Apply OuterSpaceVacuumBreach
    state.apply_soundscape_archetype(SoundscapeArchetype::OuterSpaceVacuumBreach);
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::OuterSpaceVacuumBreach
    );
    assert_eq!(state.soundscape_settings.active_config.emitter.sound_pitch, 42);

    // Undo returns to HighTechComputerCore
    state.undo();
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::HighTechComputerCore
    );

    // Redo returns to OuterSpaceVacuumBreach
    state.redo();
    assert_eq!(
        state.soundscape_settings.active_config.archetype,
        SoundscapeArchetype::OuterSpaceVacuumBreach
    );
}

#[test]
fn test_soundscape_attenuation_metrics_and_probe_scrubbing() {
    let mut state = EditorState::default();

    state.apply_soundscape_archetype(SoundscapeArchetype::IndustrialGeneratorDrone);

    let m = &state.soundscape_settings.cached_metrics;
    assert_eq!(m.outer_radius_uu, 2560.0); // 64 * 40 uu
    assert_eq!(m.inner_radius_uu, 640.0);  // 2560 * 0.25 uu

    // Probe at 300 uu (within inner radius 640 uu) -> 100% volume
    state.scrub_soundscape_probe(300.0);
    assert_eq!(state.soundscape_settings.cached_probe_sample.raw_volume_fraction, 1.0);
    assert!(state.soundscape_settings.cached_probe_sample.effective_spl_db > 80.0);

    // Probe at 2560 uu (at outer threshold) -> 0% volume
    state.scrub_soundscape_probe(2560.0);
    assert_eq!(state.soundscape_settings.cached_probe_sample.raw_volume_fraction, 0.0);
}

#[test]
fn test_commit_soundscape_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_soundscape_archetype(SoundscapeArchetype::CathedralGothicVault);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (zone_b_idx, ambient_idx, zone_act_idx) = state.commit_soundscape_to_world();

    assert_eq!(zone_b_idx, initial_world_brushes);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);

    // Zone brush: 6 polys, PF_SEMISOLID (0x200000)
    let zone_b = &state.world_brushes[zone_b_idx];
    assert_eq!(zone_b.polys.len(), 6);
    assert_eq!(zone_b.polys[0].flags & 0x00200000, 0x00200000);

    // Actors created: AmbientSound + ZoneInfo = 2 actors
    assert!(ambient_idx.is_some());
    assert!(zone_act_idx.is_some());

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 2);

    let amb_act = &unr.actors[ambient_idx.unwrap()];
    assert_eq!(amb_act.class_name, "Engine.AmbientSound");

    let zone_act = &unr.actors[zone_act_idx.unwrap()];
    assert_eq!(zone_act.class_name, "Engine.ZoneInfo");

    // Undo removes zone brush and both actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores zone brush and both actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 2);
}

#[test]
fn test_soundscape_safety_audit() {
    let mut state = EditorState::default();

    // 1. Sub-audible sound radius (< 8) flags critical hazard
    state.soundscape_settings.active_config.emitter.sound_radius = 4;
    state.recalculate_soundscape_preview();
    let has_subaudible = state.soundscape_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Audible"));
    assert!(has_subaudible, "Must flag critical hazard for sub-audible radius");

    // 2. Zero sound volume (0) flags critical hazard
    state.soundscape_settings.active_config.emitter.sound_radius = 32;
    state.soundscape_settings.active_config.emitter.sound_volume = 0;
    state.recalculate_soundscape_preview();
    let has_muted = state.soundscape_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Zero Sound Volume"));
    assert!(has_muted, "Must flag critical hazard for muted volume");

    // 3. Extreme low pitch (< 32) flags warning
    state.soundscape_settings.active_config.emitter.sound_volume = 200;
    state.soundscape_settings.active_config.emitter.sound_pitch = 24;
    state.recalculate_soundscape_preview();
    let has_low_pitch = state.soundscape_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Low Pitch"));
    assert!(has_low_pitch, "Must flag warning for low pitch rumble");
}
