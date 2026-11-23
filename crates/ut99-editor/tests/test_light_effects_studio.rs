use glam::Vec3;
use ut99_bsp::{
    evaluate_dynamic_light, sample_light_waveform, DynamicLightPreset, DynamicLightState,
    LightEffect, LightType, SceneLight,
};
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_light_waveform_sampling_all_effects() {
    // 1. None / Steady: Always constant 1.0
    let steady_light = SceneLight {
        light_type: LightType::Steady as u8,
        light_effect: LightEffect::None as u8,
        period: 32,
        phase: 0,
        ..Default::default()
    };
    let steady_samples = sample_light_waveform(&steady_light, 2.0, 10);
    assert_eq!(steady_samples.len(), 10);
    for (_t, intensity) in steady_samples {
        assert!((intensity - 1.0).abs() < 1e-3, "Steady light intensity must be 1.0");
    }

    // 2. Pulse: Smooth sinusoidal oscillation between 0.0 and 1.0
    let pulse_light = SceneLight {
        light_type: LightType::Pulse as u8,
        light_effect: LightEffect::None as u8,
        period: 64, // freq = 64/64 = 1.0 Hz
        phase: 0,
        ..Default::default()
    };
    let s0 = evaluate_dynamic_light(&pulse_light, 0.0);
    let s_quarter = evaluate_dynamic_light(&pulse_light, 0.5);
    let s_half = evaluate_dynamic_light(&pulse_light, 1.0);
    let s_three_quarter = evaluate_dynamic_light(&pulse_light, 1.5);

    assert!((s0.intensity_multiplier - 1.0).abs() < 1e-3, "Pulse at t=0 must start at 1.0, got {}", s0.intensity_multiplier);
    assert!((s_quarter.intensity_multiplier - 0.5).abs() < 1e-3, "Pulse at quarter-cycle must be 0.5, got {}", s_quarter.intensity_multiplier);
    assert!((s_half.intensity_multiplier - 0.0).abs() < 1e-3, "Pulse at half-cycle must be 0.0, got {}", s_half.intensity_multiplier);
    assert!((s_three_quarter.intensity_multiplier - 0.5).abs() < 1e-3, "Pulse at three-quarter cycle must be 0.5, got {}", s_three_quarter.intensity_multiplier);

    // 3. Blink: Square wave
    let blink_light = SceneLight {
        light_type: LightType::Blink as u8,
        light_effect: LightEffect::None as u8,
        period: 64,
        phase: 0,
        ..Default::default()
    };
    let blink_on = evaluate_dynamic_light(&blink_light, 0.2);
    let blink_off = evaluate_dynamic_light(&blink_light, 1.5);
    assert_eq!(blink_on.intensity_multiplier, 1.0, "Blink first half must be ON (1.0)");
    assert_eq!(blink_off.intensity_multiplier, 0.0, "Blink second half must be OFF (0.0)");

    // 4. Strobe: Fast duty cycle
    let strobe_light = SceneLight {
        light_type: LightType::Strobe as u8,
        light_effect: LightEffect::None as u8,
        period: 64,
        phase: 0,
        ..Default::default()
    };
    let strobe_on = evaluate_dynamic_light(&strobe_light, 0.1);
    let strobe_off = evaluate_dynamic_light(&strobe_light, 1.0);
    assert_eq!(strobe_on.intensity_multiplier, 1.0, "Strobe duty burst must be ON");
    assert_eq!(strobe_off.intensity_multiplier, 0.0, "Strobe rest cycle must be OFF");

    // 5. Flicker & TorchWaver
    let flicker_light = SceneLight {
        light_type: LightType::Flicker as u8,
        light_effect: LightEffect::TorchWaver as u8,
        period: 32,
        phase: 0,
        ..Default::default()
    };
    for t in [0.1, 0.35, 0.72, 1.15, 2.8] {
        let f_state = evaluate_dynamic_light(&flicker_light, t);
        assert!((0.0..=1.0).contains(&f_state.intensity_multiplier), "Intensity out of bounds at t={t}: {}", f_state.intensity_multiplier);
    }
}

#[test]
fn test_evaluate_dynamic_light_state_and_presets() {
    let presets = DynamicLightPreset::all();
    assert_eq!(presets.len(), 6, "Must define exactly 6 canonical dynamic light presets");

    for &preset in presets {
        assert!(!preset.display_name().is_empty());
        assert!(!preset.description().is_empty());

        let mut light = SceneLight {
            location: Vec3::new(100.0, 200.0, 300.0),
            ..Default::default()
        };
        preset.apply_to_light(&mut light);

        // Verify parameter bounds
        assert!(light.brightness > 0, "Preset must have non-zero brightness");
        assert!(light.radius > 0, "Preset must have non-zero radius");

        // Evaluate at dynamic time
        let state: DynamicLightState = evaluate_dynamic_light(&light, 1.25);
        assert!(state.effective_rgb.x >= 0.0 && state.effective_rgb.x <= 1.0);
        assert!(state.effective_rgb.y >= 0.0 && state.effective_rgb.y <= 1.0);
        assert!(state.effective_rgb.z >= 0.0 && state.effective_rgb.z <= 1.0);
        assert!(state.current_radius_uu > 0.0);

        // Directional searchlight specific tests
        if light.light_effect == LightEffect::Searchlight as u8 {
            assert!(state.is_directional);
            assert!(state.spot_cone_angle_rad > 0.0 && state.spot_cone_angle_rad <= std::f32::consts::PI);
            assert!(state.spot_direction.is_normalized(), "Spot direction must be normalized");
        }
    }
}

#[test]
fn test_editor_dynamic_light_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());
    state.builder_brush.location = Vec3::new(256.0, 512.0, 128.0);

    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();

    // Place a CyberSearchlightRotor dynamic light
    let placed_idx = state.place_dynamic_light_at_pivot(DynamicLightPreset::CyberSearchlightRotor);
    assert_eq!(placed_idx, initial_actor_count);

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), initial_actor_count + 1);

    let light_actor = &level.actors[placed_idx];
    assert_eq!(light_actor.class_name, "Light");
    assert_eq!(light_actor.location, Vec3::new(256.0, 512.0, 128.0));
    assert_eq!(light_actor.light_type, LightType::Steady as u8);
    assert_eq!(light_actor.light_effect, LightEffect::Searchlight as u8);

    // Verify tagged properties for UT99 engine fidelity
    assert!(light_actor.properties.contains_key("LightPeriod"));
    assert!(light_actor.properties.contains_key("LightPhase"));
    assert!(light_actor.properties.contains_key("LightCone"));

    // Undo placement
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count, "Undo must remove placed light actor");

    // Redo placement
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count + 1, "Redo must restore placed light actor");
}

#[test]
fn test_editor_apply_preset_and_update_light_effects_with_undo_redo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut light1 = ActorRecord::default();
    light1.name = "Light_A".to_string();
    light1.class_name = "Engine.Light".to_string();
    light1.light_brightness = 64;
    light1.light_hue = 10;
    light1.light_saturation = 200;

    let mut light2 = ActorRecord::default();
    light2.name = "Light_B".to_string();
    light2.class_name = "Engine.Light".to_string();
    light2.light_brightness = 80;
    light2.light_hue = 20;
    light2.light_saturation = 180;

    level.actors = vec![light1, light2].into();
    state.loaded_unr = Some(level);

    // Multi-select both lights
    state.selected_actor_index = Some(0);
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);

    // 1. Apply EmergencyAlarmStrobe preset
    let count = state.apply_dynamic_light_preset_to_selected(DynamicLightPreset::EmergencyAlarmStrobe);
    assert_eq!(count, 2, "Must modify 2 selected lights");

    let level_mod = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_mod.actors[0].light_type, LightType::Strobe as u8);
    assert_eq!(level_mod.actors[0].light_hue, 0); // Strobe alarm red
    assert_eq!(level_mod.actors[1].light_type, LightType::Strobe as u8);

    // 2. Undo preset application
    assert!(state.history.can_undo());
    state.undo();
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors[0].light_brightness, 64, "Undo must revert brightness");
    assert_eq!(level_undone.actors[0].light_hue, 10, "Undo must revert hue");
    assert_eq!(level_undone.actors[1].light_brightness, 80);

    // 3. Redo preset application
    assert!(state.history.can_redo());
    state.redo();
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors[0].light_type, LightType::Strobe as u8);
    assert_eq!(level_redone.actors[0].light_brightness, 250);

    // 4. Update custom parameters
    let updated = state.update_selected_light_effects(
        LightType::Pulse as u8,
        LightEffect::Disco as u8,
        16,  // period
        32,  // phase
        48,  // cone
        240, // brightness
        190, // hue
        30,  // saturation
        40,  // radius
    );
    assert_eq!(updated, 2);

    let level_custom = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_custom.actors[0].light_type, LightType::Pulse as u8);
    assert_eq!(level_custom.actors[0].light_effect, LightEffect::Disco as u8);
    assert_eq!(level_custom.actors[0].light_brightness, 240);
    assert_eq!(level_custom.actors[0].light_hue, 190);

    // Undo custom update
    state.undo();
    let level_back = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_back.actors[0].light_type, LightType::Strobe as u8);
}
