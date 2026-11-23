use glam::Vec3;
use ut99_bsp::{ParticleEmitterType, ParticlePresetTemplate};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_canonical_particle_preset_templates() {
    let presets = ParticlePresetTemplate::all();
    assert_eq!(presets.len(), 6);

    for &preset in presets {
        let emitter = preset.create_emitter("TestEmitter", Vec3::new(0.0, 0.0, 100.0));
        assert!(!emitter.name.is_empty());
        assert!(emitter.emission_rate > 0.0);
        assert!(emitter.initial_speed_min <= emitter.initial_speed_max);
        assert!(emitter.max_particles >= 64);
        assert!(emitter.size_start > 0.0);
        assert!(emitter.size_end > 0.0);
        assert!(emitter.spread_cone_degrees > 0.0);
        assert!(emitter.color_start[3] > 0.0);
    }

    // Spark shower has high elasticity bounce
    let sparks = ParticlePresetTemplate::SparkShower.create_emitter("Sparks", Vec3::new(0.0, 0.0, 200.0));
    assert_eq!(sparks.emitter_type, ParticleEmitterType::Sparks);
    assert!(sparks.bounce_damping >= 0.5);
    assert!(sparks.floor_z.is_some());
}

#[test]
fn test_particle_physics_simulation_and_size_color_interpolation() {
    let mut emitter = ParticlePresetTemplate::TorchFireFlames.create_emitter("Torch", Vec3::new(10.0, 20.0, 100.0));
    assert_eq!(emitter.particles.len(), 0);

    // Step simulation 0.4s
    emitter.update(0.4);
    assert!(!emitter.particles.is_empty(), "Must emit flame particles");

    for p in &emitter.particles {
        // Flame particles rise upwards
        assert!(p.position.z >= 100.0, "Torch flames must rise upwards");
        assert!(p.lifetime <= p.max_lifetime);
        // Size must interpolate between start (10.0) and end (2.0)
        assert!(p.size <= 10.0 && p.size >= 1.0);
        // Alpha fades over lifetime
        assert!(p.color[3] <= 1.0 && p.color[3] >= 0.0);
    }
}

#[test]
fn test_elastic_floor_bounce_collision() {
    let mut emitter = ParticlePresetTemplate::SparkShower.create_emitter("SparkTest", Vec3::new(0.0, 0.0, 150.0));
    emitter.floor_z = Some(100.0);
    emitter.bounce_damping = 0.65;

    // Simulate for 0.6s to allow sparks to fall, collide and bounce off Z = 100
    for _ in 0..12 {
        emitter.update(0.05);
    }

    assert!(!emitter.particles.is_empty());
    for p in &emitter.particles {
        // No particle should penetrate under the floor plane
        assert!(p.position.z >= 100.0 - 1e-2, "Particle z ({}) cannot penetrate below floor (100.0)", p.position.z);
    }

    // Check if any particle has bounced with positive upward velocity
    let has_bounced = emitter.particles.iter().any(|p| (p.position.z - 100.0).abs() < 15.0 && p.velocity.z > 0.0);
    assert!(has_bounced, "At least one spark particle must have bounced off floor");
}

#[test]
fn test_particle_studio_editor_state_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(256.0, 128.0, 64.0);

    // 1. Place emitter at pivot
    let idx = state.place_particle_emitter_at_pivot(Some(ParticlePresetTemplate::DenseIndustrialSmoke));
    assert_eq!(state.particle_emitters.len(), 1);
    assert_eq!(state.particle_emitters[idx].location, Vec3::new(256.0, 128.0, 64.0));
    assert_eq!(state.particle_emitters[idx].emitter_type, ParticleEmitterType::Smoke);

    // 2. Advance editor simulation
    state.update_particles(0.3);
    assert!(!state.particle_emitters[idx].particles.is_empty());

    // 3. Clear particles
    state.clear_emitter_particles(idx);
    assert!(state.particle_emitters[idx].particles.is_empty());

    // 4. Apply different preset
    state.apply_particle_preset(idx, ParticlePresetTemplate::WaterfallMist);
    assert_eq!(state.particle_emitters[idx].emitter_type, ParticleEmitterType::WaterSplash);

    // 5. Undo preset change
    state.undo();
    assert_eq!(state.particle_emitters[idx].emitter_type, ParticleEmitterType::Smoke);

    // 6. Redo preset change
    state.redo();
    assert_eq!(state.particle_emitters[idx].emitter_type, ParticleEmitterType::WaterSplash);
}
