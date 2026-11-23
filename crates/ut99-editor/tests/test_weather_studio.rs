// crates/ut99-editor/tests/test_weather_studio.rs
//
// Integration tests for Interactive Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio.

use glam::Vec3;
use ut99_bsp::{PrecipitationType, WeatherArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_weather_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is FrigateOceanicStorm
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::FrigateOceanicStorm
    );
    assert_eq!(
        state.weather_settings.active_config.precipitation_type,
        PrecipitationType::TorrentialRain
    );
    assert!(state.weather_settings.active_config.lightning_enabled);

    // Apply MorpheusBlizzardDrift
    state.apply_weather_archetype(WeatherArchetype::MorpheusBlizzardDrift);
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::MorpheusBlizzardDrift
    );
    assert_eq!(
        state.weather_settings.active_config.precipitation_type,
        PrecipitationType::GentleSnowfall
    );
    assert!(!state.weather_settings.active_config.lightning_enabled);

    // Apply NaPaliToxicAcidRain
    state.apply_weather_archetype(WeatherArchetype::NaPaliToxicAcidRain);
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::NaPaliToxicAcidRain
    );
    assert_eq!(
        state.weather_settings.active_config.precipitation_type,
        PrecipitationType::AcidHailstorm
    );

    // Apply ChizraMonsoonDeluge
    state.apply_weather_archetype(WeatherArchetype::ChizraMonsoonDeluge);
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::ChizraMonsoonDeluge
    );

    // Apply PhobosMeteorAshFallout
    state.apply_weather_archetype(WeatherArchetype::PhobosMeteorAshFallout);
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::PhobosMeteorAshFallout
    );
    assert_eq!(
        state.weather_settings.active_config.precipitation_type,
        PrecipitationType::AshVolcanicFallout
    );

    // Undo returns to ChizraMonsoonDeluge
    state.undo();
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::ChizraMonsoonDeluge
    );

    // Redo returns to PhobosMeteorAshFallout
    state.redo();
    assert_eq!(
        state.weather_settings.active_config.archetype,
        WeatherArchetype::PhobosMeteorAshFallout
    );
}

#[test]
fn test_weather_simulation_kinematics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_weather_archetype(WeatherArchetype::FrigateOceanicStorm);
    state.weather_settings.listener_probe_pos = Vec3::new(1000.0, 1000.0, 0.0);

    // At strike moment (t = 0.05s)
    state.scrub_weather_time(0.05);
    let m_flash = &state.weather_settings.cached_metrics;
    assert!(m_flash.is_flashing_now, "Must detect lightning flash during strike window");
    assert!(m_flash.flash_brightness > 0.0);
    assert!(m_flash.thunder_delay_at_listener_secs > 0.0, "Must calculate non-zero speed-of-sound delay");

    // At calm moment (t = 2.0s)
    state.scrub_weather_time(2.0);
    let m_calm = &state.weather_settings.cached_metrics;
    assert!(!m_calm.is_flashing_now, "Lightning should not flash during calm window");
    assert_eq!(m_calm.flash_brightness, 0.0);

    // Check wind velocity telemetry
    assert!(m_calm.net_wind_speed_uu_s > 0.0);
    assert!(m_calm.estimated_visibility_uu > 0.0);
}

#[test]
fn test_weather_safety_audit() {
    let mut state = EditorState::default();

    // 1. Extreme hurricane wind (>2000 uu/s) flags critical hazard
    state.weather_settings.active_config.wind_vector = Vec3::new(2500.0, 0.0, 0.0);
    state.recalculate_weather_preview();
    let has_wind = state.weather_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Hurricane Wind"));
    assert!(has_wind, "Must flag critical hazard for wind > 2000 uu/s");

    // 2. Sub-miniature volume extent (<256 uu) flags critical hazard
    state.weather_settings.active_config.wind_vector = Vec3::new(100.0, 0.0, 0.0);
    state.weather_settings.active_config.volume_extents = Vec3::new(128.0, 128.0, 128.0);
    state.recalculate_weather_preview();
    let has_mini = state.weather_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Miniature Weather Volume"));
    assert!(has_mini, "Must flag critical hazard for dimensions < 256 uu");

    // 3. Stagnant fall speed (<= 0 uu/s) flags critical hazard
    state.weather_settings.active_config.volume_extents = Vec3::new(2048.0, 2048.0, 1024.0);
    state.weather_settings.active_config.fall_speed_uu_s = 0.0;
    state.recalculate_weather_preview();
    let has_zero_fall = state.weather_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Zero or Negative"));
    assert!(has_zero_fall, "Must flag critical hazard for zero fall speed");

    // 4. Strobe epilepsy risk (<0.8s) flags warning
    state.weather_settings.active_config.fall_speed_uu_s = 1200.0;
    state.weather_settings.active_config.lightning_enabled = true;
    state.weather_settings.active_config.lightning_interval_secs = 0.5;
    state.recalculate_weather_preview();
    let has_strobe = state.weather_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Rapid Lightning Strobe"));
    assert!(has_strobe, "Must flag warning for lightning interval < 0.8s");
}

#[test]
fn test_commit_weather_to_world_undo_redo() {
    let mut state = EditorState::default();

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    state.apply_weather_archetype(WeatherArchetype::FrigateOceanicStorm);
    let (vol_idx, emit_idx, light_idx, audio_idx) = state.commit_weather_to_world();

    // Verifies 1 volume brush added
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);
    assert_eq!(vol_idx, initial_world_brushes);

    // Verifies 3 actors added (Emitter, Light, Audio)
    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 3);

    let emit_act = &unr.actors[emit_idx.unwrap()];
    assert_eq!(emit_act.class_name, "Engine.WeatherEmitter");
    assert_eq!(emit_act.group, "WeatherStorm");

    let light_act = &unr.actors[light_idx.unwrap()];
    assert_eq!(light_act.class_name, "Engine.Light");
    assert_eq!(light_act.group, "WeatherStorm");

    let audio_act = &unr.actors[audio_idx.unwrap()];
    assert_eq!(audio_act.class_name, "Engine.AmbientSound");
    assert_eq!(audio_act.group, "WeatherStorm");

    // Undo removes volume brush and all 3 actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores volume brush and all 3 actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 3);
}
