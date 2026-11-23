/// EAX (Environmental Audio Extensions) reverb presets as defined in Unreal Engine 1 / UT99.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EaxEnvironment {
    Generic,
    PaddedCell,
    Room,
    Bathroom,
    LivingRoom,
    StoneRoom,
    Auditorium,
    ConcertHall,
    Cave,
    Arena,
    Hangar,
    CarpetedHallway,
    Hallway,
    StoneCorridor,
    Alley,
    Forest,
    City,
    Mountains,
    Quarry,
    Plain,
    ParkingLot,
    SewerPipe,
    Underwater,
    Drugged,
    Dizzy,
    Psychotic,
}

impl EaxEnvironment {
    pub const ALL: &'static [EaxEnvironment] = &[
        EaxEnvironment::Generic,
        EaxEnvironment::PaddedCell,
        EaxEnvironment::Room,
        EaxEnvironment::Bathroom,
        EaxEnvironment::LivingRoom,
        EaxEnvironment::StoneRoom,
        EaxEnvironment::Auditorium,
        EaxEnvironment::ConcertHall,
        EaxEnvironment::Cave,
        EaxEnvironment::Arena,
        EaxEnvironment::Hangar,
        EaxEnvironment::CarpetedHallway,
        EaxEnvironment::Hallway,
        EaxEnvironment::StoneCorridor,
        EaxEnvironment::Alley,
        EaxEnvironment::Forest,
        EaxEnvironment::City,
        EaxEnvironment::Mountains,
        EaxEnvironment::Quarry,
        EaxEnvironment::Plain,
        EaxEnvironment::ParkingLot,
        EaxEnvironment::SewerPipe,
        EaxEnvironment::Underwater,
        EaxEnvironment::Drugged,
        EaxEnvironment::Dizzy,
        EaxEnvironment::Psychotic,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            EaxEnvironment::Generic => "Generic",
            EaxEnvironment::PaddedCell => "PaddedCell",
            EaxEnvironment::Room => "Room",
            EaxEnvironment::Bathroom => "Bathroom",
            EaxEnvironment::LivingRoom => "LivingRoom",
            EaxEnvironment::StoneRoom => "StoneRoom",
            EaxEnvironment::Auditorium => "Auditorium",
            EaxEnvironment::ConcertHall => "ConcertHall",
            EaxEnvironment::Cave => "Cave",
            EaxEnvironment::Arena => "Arena",
            EaxEnvironment::Hangar => "Hangar",
            EaxEnvironment::CarpetedHallway => "CarpetedHallway",
            EaxEnvironment::Hallway => "Hallway",
            EaxEnvironment::StoneCorridor => "StoneCorridor",
            EaxEnvironment::Alley => "Alley",
            EaxEnvironment::Forest => "Forest",
            EaxEnvironment::City => "City",
            EaxEnvironment::Mountains => "Mountains",
            EaxEnvironment::Quarry => "Quarry",
            EaxEnvironment::Plain => "Plain",
            EaxEnvironment::ParkingLot => "ParkingLot",
            EaxEnvironment::SewerPipe => "SewerPipe",
            EaxEnvironment::Underwater => "Underwater",
            EaxEnvironment::Drugged => "Drugged",
            EaxEnvironment::Dizzy => "Dizzy",
            EaxEnvironment::Psychotic => "Psychotic",
        }
    }
}

/// Comprehensive acoustic parameters for a Zone in UT99.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneAcoustics {
    pub preset: EaxEnvironment,
    pub environment_size: f32,          // Approximate zone dimensions in Unreal Units (e.g. 1000.0)
    pub reverb_decay_time: f32,         // Reverb time in seconds (T60: time to decay by 60 dB)
    pub high_frequency_decay_ratio: f32,// Ratio of HF decay relative to mid-band (air absorption)
    pub reflections_delay: f32,         // Initial early reflections delay in milliseconds
    pub reverb_delay: f32,              // Late reverb onset delay in milliseconds
    pub room_attenuation_db: f32,       // Room level attenuation in decibels (-100.0..0.0)
    pub description: &'static str,
}

/// Returns the authentic EAX acoustic parameters for a standard UT99 environment preset.
pub fn get_eax_preset(preset: EaxEnvironment) -> ZoneAcoustics {
    match preset {
        EaxEnvironment::Generic => ZoneAcoustics {
            preset,
            environment_size: 1000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.83,
            reflections_delay: 7.0,
            reverb_delay: 11.0,
            room_attenuation_db: -10.0,
            description: "Standard balanced acoustic response suitable for general arenas.",
        },
        EaxEnvironment::PaddedCell => ZoneAcoustics {
            preset,
            environment_size: 400.0,
            reverb_decay_time: 0.17,
            high_frequency_decay_ratio: 0.10,
            reflections_delay: 1.0,
            reverb_delay: 2.0,
            room_attenuation_db: -20.0,
            description: "Extremely dead acoustics with zero echo and severe high-frequency damping.",
        },
        EaxEnvironment::Room => ZoneAcoustics {
            preset,
            environment_size: 600.0,
            reverb_decay_time: 0.40,
            high_frequency_decay_ratio: 0.83,
            reflections_delay: 2.0,
            reverb_delay: 3.0,
            room_attenuation_db: -10.0,
            description: "Tight, dry room with rapid decay suitable for compact control rooms.",
        },
        EaxEnvironment::Bathroom => ZoneAcoustics {
            preset,
            environment_size: 500.0,
            reverb_decay_time: 1.50,
            high_frequency_decay_ratio: 1.37,
            reflections_delay: 7.0,
            reverb_delay: 12.0,
            room_attenuation_db: -12.0,
            description: "Highly reflective tiled surfaces with pronounced flutter echo.",
        },
        EaxEnvironment::LivingRoom => ZoneAcoustics {
            preset,
            environment_size: 700.0,
            reverb_decay_time: 0.50,
            high_frequency_decay_ratio: 0.80,
            reflections_delay: 3.0,
            reverb_delay: 4.0,
            room_attenuation_db: -11.0,
            description: "Furnished domestic environment with soft absorptive materials.",
        },
        EaxEnvironment::StoneRoom => ZoneAcoustics {
            preset,
            environment_size: 1200.0,
            reverb_decay_time: 2.31,
            high_frequency_decay_ratio: 0.64,
            reflections_delay: 12.0,
            reverb_delay: 17.0,
            room_attenuation_db: -10.0,
            description: "Heavy granite/masonry walls with brilliant, resonant early reflections.",
        },
        EaxEnvironment::Auditorium => ZoneAcoustics {
            preset,
            environment_size: 2500.0,
            reverb_decay_time: 4.32,
            high_frequency_decay_ratio: 0.59,
            reflections_delay: 20.0,
            reverb_delay: 30.0,
            room_attenuation_db: -7.0,
            description: "Large lecture or theatrical hall with spacious spatial imaging.",
        },
        EaxEnvironment::ConcertHall => ZoneAcoustics {
            preset,
            environment_size: 3000.0,
            reverb_decay_time: 3.92,
            high_frequency_decay_ratio: 0.70,
            reflections_delay: 20.0,
            reverb_delay: 29.0,
            room_attenuation_db: -6.0,
            description: "Grand symphonic hall with rich, enveloping reverberant wash.",
        },
        EaxEnvironment::Cave => ZoneAcoustics {
            preset,
            environment_size: 2000.0,
            reverb_decay_time: 2.91,
            high_frequency_decay_ratio: 1.30,
            reflections_delay: 15.0,
            reverb_delay: 22.0,
            room_attenuation_db: -10.0,
            description: "Subterranean cavern with pronounced high-frequency slapback and dark decay.",
        },
        EaxEnvironment::Arena => ZoneAcoustics {
            preset,
            environment_size: 4000.0,
            reverb_decay_time: 7.24,
            high_frequency_decay_ratio: 0.33,
            reflections_delay: 20.0,
            reverb_delay: 30.0,
            room_attenuation_db: -8.0,
            description: "Massive tournament coliseum with long, epic reverberant tails.",
        },
        EaxEnvironment::Hangar => ZoneAcoustics {
            preset,
            environment_size: 5000.0,
            reverb_decay_time: 10.05,
            high_frequency_decay_ratio: 0.23,
            reflections_delay: 20.0,
            reverb_delay: 30.0,
            room_attenuation_db: -10.0,
            description: "Enormous industrial hangar with metallic flutter and colossal tail.",
        },
        EaxEnvironment::CarpetedHallway => ZoneAcoustics {
            preset,
            environment_size: 800.0,
            reverb_decay_time: 0.30,
            high_frequency_decay_ratio: 0.10,
            reflections_delay: 2.0,
            reverb_delay: 30.0,
            room_attenuation_db: -18.0,
            description: "Narrow corridor with carpeted flooring; heavily attenuated reflections.",
        },
        EaxEnvironment::Hallway => ZoneAcoustics {
            preset,
            environment_size: 1000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.59,
            reflections_delay: 7.0,
            reverb_delay: 11.0,
            room_attenuation_db: -10.0,
            description: "Standard architectural corridor with moderate reflection and clear focus.",
        },
        EaxEnvironment::StoneCorridor => ZoneAcoustics {
            preset,
            environment_size: 1200.0,
            reverb_decay_time: 2.69,
            high_frequency_decay_ratio: 0.64,
            reflections_delay: 13.0,
            reverb_delay: 20.0,
            room_attenuation_db: -10.0,
            description: "Castle/dungeon stone passage with sharp slap echo.",
        },
        EaxEnvironment::Alley => ZoneAcoustics {
            preset,
            environment_size: 1500.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.34,
            reflections_delay: 7.0,
            reverb_delay: 11.0,
            room_attenuation_db: -12.0,
            description: "Narrow outdoor street flanked by high brick walls.",
        },
        EaxEnvironment::Forest => ZoneAcoustics {
            preset,
            environment_size: 3000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.33,
            reflections_delay: 162.0,
            reverb_delay: 88.0,
            room_attenuation_db: -20.0,
            description: "Outdoor natural canopy with scattered distant reflections.",
        },
        EaxEnvironment::City => ZoneAcoustics {
            preset,
            environment_size: 4000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.50,
            reflections_delay: 7.0,
            reverb_delay: 11.0,
            room_attenuation_db: -15.0,
            description: "Urban downtown street grid with expansive specular wall bounce.",
        },
        EaxEnvironment::Mountains => ZoneAcoustics {
            preset,
            environment_size: 8000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.27,
            reflections_delay: 300.0,
            reverb_delay: 100.0,
            room_attenuation_db: -22.0,
            description: "Open highland terrain with immense echo delay times (>300 ms).",
        },
        EaxEnvironment::Quarry => ZoneAcoustics {
            preset,
            environment_size: 3500.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.21,
            reflections_delay: 61.0,
            reverb_delay: 25.0,
            room_attenuation_db: -10.0,
            description: "Open excavation pit with hard, unyielding stone facets.",
        },
        EaxEnvironment::Plain => ZoneAcoustics {
            preset,
            environment_size: 10000.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.21,
            reflections_delay: 179.0,
            reverb_delay: 100.0,
            room_attenuation_db: -25.0,
            description: "Flat open prairie with minimal ground reflection and zero enclosure.",
        },
        EaxEnvironment::ParkingLot => ZoneAcoustics {
            preset,
            environment_size: 2000.0,
            reverb_decay_time: 1.65,
            high_frequency_decay_ratio: 1.50,
            reflections_delay: 8.0,
            reverb_delay: 12.0,
            room_attenuation_db: -13.0,
            description: "Paved open parking lot bounded by low walls and asphalt.",
        },
        EaxEnvironment::SewerPipe => ZoneAcoustics {
            preset,
            environment_size: 600.0,
            reverb_decay_time: 2.81,
            high_frequency_decay_ratio: 0.14,
            reflections_delay: 14.0,
            reverb_delay: 21.0,
            room_attenuation_db: -10.0,
            description: "Cylindrical concrete tunnel with metallic resonance and ringing resonance.",
        },
        EaxEnvironment::Underwater => ZoneAcoustics {
            preset,
            environment_size: 1200.0,
            reverb_decay_time: 1.49,
            high_frequency_decay_ratio: 0.10,
            reflections_delay: 9.0,
            reverb_delay: 10.0,
            room_attenuation_db: -12.0,
            description: "Submerged liquid volume with severe high-frequency cutoff and muffled propagation.",
        },
        EaxEnvironment::Drugged => ZoneAcoustics {
            preset,
            environment_size: 1500.0,
            reverb_decay_time: 8.39,
            high_frequency_decay_ratio: 1.39,
            reflections_delay: 0.0,
            reverb_delay: 0.0,
            room_attenuation_db: -5.0,
            description: "Surreal perceptual distortion with hallucinatory sustained harmonics.",
        },
        EaxEnvironment::Dizzy => ZoneAcoustics {
            preset,
            environment_size: 1000.0,
            reverb_decay_time: 1.39,
            high_frequency_decay_ratio: 0.83,
            reflections_delay: 100.0,
            reverb_delay: 50.0,
            room_attenuation_db: -8.0,
            description: "Disorienting pitch-warped doppler reflections.",
        },
        EaxEnvironment::Psychotic => ZoneAcoustics {
            preset,
            environment_size: 1800.0,
            reverb_decay_time: 7.56,
            high_frequency_decay_ratio: 0.91,
            reflections_delay: 0.0,
            reverb_delay: 0.0,
            room_attenuation_db: -5.0,
            description: "Extreme, aggressive psychoacoustic resonance for nightmare sequences.",
        },
    }
}

/// Parses an EAX preset name string into its enum variant (case-insensitive).
pub fn parse_eax_preset(name: &str) -> Option<EaxEnvironment> {
    let clean = name.trim().to_lowercase().replace('_', "").replace(' ', "");
    for &env in EaxEnvironment::ALL {
        if env.name().to_lowercase() == clean {
            return Some(env);
        }
    }
    None
}

/// Calculates sound volume attenuation based on distance and sound radius in Unreal Units.
/// In Unreal Engine 1, sound falloff is quadratic: (1.0 - distance / radius)^2 for d < radius.
pub fn calculate_sound_attenuation(distance: f32, sound_radius_units: f32) -> f32 {
    let radius = sound_radius_units.max(1.0);
    if distance >= radius {
        0.0
    } else {
        let ratio = 1.0 - (distance / radius);
        ratio * ratio
    }
}

/// Synthesizes an impulse response waveform decaying exponentially according to the zone's acoustics.
/// Returns normalized amplitudes in range [-1.0 .. 1.0] for graphical preview in the editor.
pub fn simulate_reverb_impulse_samples(acoustics: &ZoneAcoustics, sample_count: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(sample_count);
    if sample_count == 0 {
        return samples;
    }

    let decay = acoustics.reverb_decay_time.max(0.1);
    let total_time = decay * 1.2;
    let dt = total_time / sample_count as f32;
    let tau = decay / 6.91; // Time constant for -60 dB (ln(1000) ~ 6.9078)

    let mut phase: f32 = 0.0;
    let freq = 120.0 + (1000.0 / acoustics.environment_size.clamp(200.0, 5000.0)) * 200.0;

    for i in 0..sample_count {
        let t = i as f32 * dt;
        let env = (-t / tau).exp();

        // Multi-frequency reflection simulation
        phase += freq * dt * std::f32::consts::TAU;
        let noise = ((i * 1103515245 + 12345) % 65536) as f32 / 32768.0 - 1.0;
        let signal = (phase.sin() * 0.7 + noise * 0.3) * env;

        samples.push(signal.clamp(-1.0, 1.0));
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eax_presets() {
        assert_eq!(parse_eax_preset("room"), Some(EaxEnvironment::Room));
        assert_eq!(parse_eax_preset("ARENA"), Some(EaxEnvironment::Arena));
        assert_eq!(parse_eax_preset("Underwater"), Some(EaxEnvironment::Underwater));
        assert_eq!(parse_eax_preset("NonExistent"), None);

        let hall = get_eax_preset(EaxEnvironment::ConcertHall);
        assert!(hall.reverb_decay_time > 3.0);
        assert_eq!(hall.preset, EaxEnvironment::ConcertHall);
    }

    #[test]
    fn test_sound_attenuation() {
        let radius = 400.0;
        assert_eq!(calculate_sound_attenuation(0.0, radius), 1.0);
        assert_eq!(calculate_sound_attenuation(400.0, radius), 0.0);
        assert_eq!(calculate_sound_attenuation(500.0, radius), 0.0);

        // At half radius (200), (1 - 0.5)^2 = 0.25
        let half = calculate_sound_attenuation(200.0, radius);
        assert!((half - 0.25).abs() < 1e-4);
    }

    #[test]
    fn test_simulate_reverb_impulse_samples() {
        let room = get_eax_preset(EaxEnvironment::Room);
        let samples = simulate_reverb_impulse_samples(&room, 500);
        assert_eq!(samples.len(), 500);
        let early_energy: f32 = samples[0..50].iter().map(|s| s.abs()).sum::<f32>() / 50.0;
        let late_energy: f32 = samples[450..500].iter().map(|s| s.abs()).sum::<f32>() / 50.0;
        assert!(early_energy > late_energy, "Early reverb energy must exceed late tail energy");
    }
}
