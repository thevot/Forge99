use glam::Vec3;

/// Canonical Unreal Tournament EMusicTransition enum values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MusicTransition {
    #[default]
    None = 0,
    Instant = 1,
    Fade = 2,
    FastFade = 3,
    SlowFade = 4,
    Chorus = 5,
}

impl MusicTransition {
    pub fn all() -> &'static [MusicTransition] {
        &[
            MusicTransition::None,
            MusicTransition::Instant,
            MusicTransition::Fade,
            MusicTransition::FastFade,
            MusicTransition::SlowFade,
            MusicTransition::Chorus,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MusicTransition::None => "MTRAN_None (Hold current)",
            MusicTransition::Instant => "MTRAN_Instant (Cut immediately)",
            MusicTransition::Fade => "MTRAN_Fade (Normal ~2.0s cross-fade)",
            MusicTransition::FastFade => "MTRAN_FastFade (Rapid ~0.8s cross-fade)",
            MusicTransition::SlowFade => "MTRAN_SlowFade (Cinematic ~4.5s cross-fade)",
            MusicTransition::Chorus => "MTRAN_Chorus (Jump to chorus)",
        }
    }

    pub fn fade_duration_secs(&self) -> f32 {
        match self {
            MusicTransition::None | MusicTransition::Instant => 0.0,
            MusicTransition::FastFade => 0.8,
            MusicTransition::Fade => 2.0,
            MusicTransition::SlowFade => 4.5,
            MusicTransition::Chorus => 0.5,
        }
    }
}

impl From<u8> for MusicTransition {
    fn from(val: u8) -> Self {
        match val {
            1 => MusicTransition::Instant,
            2 => MusicTransition::Fade,
            3 => MusicTransition::FastFade,
            4 => MusicTransition::SlowFade,
            5 => MusicTransition::Chorus,
            _ => MusicTransition::None,
        }
    }
}

/// Metadata and order index of a song pattern section within a tracker module
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MusicSectionData {
    pub section_index: u8,
    pub name: String,
    pub order_start: usize,
    pub bpm: u16,
    pub mood: String,
}

/// Detailed tracker module metadata (.umx / MOD / S3M / XM / IT)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackerModuleInfo {
    pub package: String,
    pub title: String,
    pub author: String,
    pub format: String,
    pub channels: u8,
    pub default_bpm: u16,
    pub default_speed: u8,
    pub pattern_count: usize,
    pub order_count: usize,
    pub sections: Vec<MusicSectionData>,
}

/// Canonical music presets inspired by legendary Unreal Tournament tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MusicTrackPreset {
    ForegoneDestruction,
    RunDeck16,
    MechanismEight,
    BotpackTheme,
    NetherAnimal,
    Colossus,
}

impl MusicTrackPreset {
    pub fn all() -> &'static [MusicTrackPreset] {
        &[
            MusicTrackPreset::ForegoneDestruction,
            MusicTrackPreset::RunDeck16,
            MusicTrackPreset::MechanismEight,
            MusicTrackPreset::BotpackTheme,
            MusicTrackPreset::NetherAnimal,
            MusicTrackPreset::Colossus,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MusicTrackPreset::ForegoneDestruction => "🌌 Foregone Destruction (Facing Worlds - FastTracker 2 XM)",
            MusicTrackPreset::RunDeck16 => "🧪 Run (Deck 16 - Impulse Tracker IT)",
            MusicTrackPreset::MechanismEight => "⚙ Mechanism Eight (Infiltration/Action - Impulse Tracker IT)",
            MusicTrackPreset::BotpackTheme => "🏆 Botpack Theme (Arena Title - FastTracker 2 XM)",
            MusicTrackPreset::NetherAnimal => "🦇 Nether Animal (Gothic Crypt - ScreamTracker 3 S3M)",
            MusicTrackPreset::Colossus => "🪐 Colossus (Cosmic Void - Impulse Tracker IT)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MusicTrackPreset::ForegoneDestruction => {
                "Liquid Drum'n'Bass masterpiece by Alexander Brandon. Ambient rolling bassline transitioning to energetic sniper duel."
            }
            MusicTrackPreset::RunDeck16 => {
                "Hard acid-techno by Michiel van den Bos. Grimy industrial breakbeats and pulsating filter resonance."
            }
            MusicTrackPreset::MechanismEight => {
                "Stealth-to-assault tracker module by Andrew Sega (Necros). Subtle arpeggios exploding into hard breakbeats."
            }
            MusicTrackPreset::BotpackTheme => {
                "Epic brass fanfare and march rhythms by Alexander Brandon. The iconic tournament anthem."
            }
            MusicTrackPreset::NetherAnimal => {
                "Dark ambient chiptune and haunting strings by Michiel van den Bos for crypts and ancient temples."
            }
            MusicTrackPreset::Colossus => {
                "Sprawling cinematic space ambient with tribal percussion and celestial choir pads."
            }
        }
    }

    pub fn to_tracker_info(&self) -> TrackerModuleInfo {
        match self {
            MusicTrackPreset::ForegoneDestruction => TrackerModuleInfo {
                package: "Foregone".to_string(),
                title: "Foregone Destruction".to_string(),
                author: "Alexander Brandon (Siren)".to_string(),
                format: "FastTracker 2 (XM)".to_string(),
                channels: 18,
                default_bpm: 170,
                default_speed: 6,
                pattern_count: 24,
                order_count: 42,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Ambient Floating Bass".to_string(),
                        order_start: 0,
                        bpm: 170,
                        mood: "Dreamy Liquid DnB".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Tower Sniping Spree".to_string(),
                        order_start: 14,
                        bpm: 170,
                        mood: "High Intensity Action".to_string(),
                    },
                    MusicSectionData {
                        section_index: 2,
                        name: "Flag Return Climax".to_string(),
                        order_start: 28,
                        bpm: 170,
                        mood: "Triumphant Finale".to_string(),
                    },
                ],
            },
            MusicTrackPreset::RunDeck16 => TrackerModuleInfo {
                package: "Run".to_string(),
                title: "Run".to_string(),
                author: "Michiel van den Bos".to_string(),
                format: "Impulse Tracker (IT)".to_string(),
                channels: 24,
                default_bpm: 145,
                default_speed: 6,
                pattern_count: 32,
                order_count: 48,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Vat Chamber Stalking".to_string(),
                        order_start: 0,
                        bpm: 145,
                        mood: "Menacing Acid Groove".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Flak Cannon Mayhem".to_string(),
                        order_start: 16,
                        bpm: 145,
                        mood: "Overdrive Hardcore Break".to_string(),
                    },
                ],
            },
            MusicTrackPreset::MechanismEight => TrackerModuleInfo {
                package: "Mech8".to_string(),
                title: "Mechanism Eight".to_string(),
                author: "Andrew Sega (Necros)".to_string(),
                format: "Impulse Tracker (IT)".to_string(),
                channels: 28,
                default_bpm: 135,
                default_speed: 6,
                pattern_count: 28,
                order_count: 50,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Silent Infiltration".to_string(),
                        order_start: 0,
                        bpm: 135,
                        mood: "Pulsing Sub-bass".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Breach & Assault".to_string(),
                        order_start: 18,
                        bpm: 135,
                        mood: "Searing Synth Leads".to_string(),
                    },
                ],
            },
            MusicTrackPreset::BotpackTheme => TrackerModuleInfo {
                package: "Botpack9".to_string(),
                title: "Botpack Nine".to_string(),
                author: "Alexander Brandon".to_string(),
                format: "FastTracker 2 (XM)".to_string(),
                channels: 20,
                default_bpm: 125,
                default_speed: 6,
                pattern_count: 16,
                order_count: 30,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Tournament Fanfare".to_string(),
                        order_start: 0,
                        bpm: 125,
                        mood: "Regal Brass March".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Combat Rollcall".to_string(),
                        order_start: 8,
                        bpm: 125,
                        mood: "Heavy Arena Drumbeat".to_string(),
                    },
                ],
            },
            MusicTrackPreset::NetherAnimal => TrackerModuleInfo {
                package: "Nether".to_string(),
                title: "Nether Animal".to_string(),
                author: "Michiel van den Bos".to_string(),
                format: "ScreamTracker 3 (S3M)".to_string(),
                channels: 16,
                default_bpm: 140,
                default_speed: 6,
                pattern_count: 18,
                order_count: 36,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Crypt Shadows".to_string(),
                        order_start: 0,
                        bpm: 140,
                        mood: "Gothic Dark Ambient".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Monster Onslaught".to_string(),
                        order_start: 12,
                        bpm: 140,
                        mood: "Driving Industrial Pulse".to_string(),
                    },
                ],
            },
            MusicTrackPreset::Colossus => TrackerModuleInfo {
                package: "Colossus".to_string(),
                title: "Colossus".to_string(),
                author: "Alexander Brandon".to_string(),
                format: "Impulse Tracker (IT)".to_string(),
                channels: 32,
                default_bpm: 110,
                default_speed: 6,
                pattern_count: 22,
                order_count: 38,
                sections: vec![
                    MusicSectionData {
                        section_index: 0,
                        name: "Deep Space Drift".to_string(),
                        order_start: 0,
                        bpm: 110,
                        mood: "Expansive Cosmic Mystery".to_string(),
                    },
                    MusicSectionData {
                        section_index: 1,
                        name: "Titan Encounter".to_string(),
                        order_start: 10,
                        bpm: 110,
                        mood: "Thunderous Orchestral Chants".to_string(),
                    },
                ],
            },
        }
    }
}

/// MusicEvent actor representation for mapping and sequencing
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MusicEventData {
    pub index: usize,
    pub name: String,
    pub tag: String,
    pub location: Vec3,
    pub song: String,
    pub song_section: u8,
    pub transition: MusicTransition,
    pub affect_all_players: bool,
    pub once_only: bool,
    pub silence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MusicIssueSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MusicAuditIssue {
    pub severity: MusicIssueSeverity,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
}

/// Comprehensive audit report for map audio sequencing and tracker music setup
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct MusicAuditReport {
    pub level_song: String,
    pub level_section: u8,
    pub music_events_count: usize,
    pub unique_songs_referenced: Vec<String>,
    pub issues: Vec<MusicAuditIssue>,
    pub is_valid: bool,
}

/// Simulates a volume cross-fade curve between two tracks / sections.
/// Returns (volume_outgoing, volume_incoming) normalized in [0.0, 1.0].
pub fn simulate_crossfade(
    from_vol: f32,
    to_vol: f32,
    transition: MusicTransition,
    elapsed_sec: f32,
) -> (f32, f32) {
    let duration = transition.fade_duration_secs();

    if duration <= 1e-4 || elapsed_sec <= 0.0 {
        match transition {
            MusicTransition::Instant => (0.0, to_vol.clamp(0.0, 1.0)),
            _ => (from_vol.clamp(0.0, 1.0), 0.0),
        }
    } else if elapsed_sec >= duration {
        (0.0, to_vol.clamp(0.0, 1.0))
    } else {
        let t = (elapsed_sec / duration).clamp(0.0, 1.0);
        // Smoothstep (C1) curve for pleasant, natural audio blending without volume dips
        let smooth_t = t * t * (3.0 - 2.0 * t);
        let out_v = from_vol * (1.0 - smooth_t);
        let in_v = to_vol * smooth_t;
        (out_v.clamp(0.0, 1.0), in_v.clamp(0.0, 1.0))
    }
}

/// Analyzes music configuration across the level, inspecting LevelInfo song and all MusicEvent actors
pub fn audit_level_music(
    level_song: &str,
    level_section: u8,
    music_events: &[MusicEventData],
) -> MusicAuditReport {
    let mut report = MusicAuditReport {
        level_song: level_song.to_string(),
        level_section,
        music_events_count: music_events.len(),
        unique_songs_referenced: Vec::new(),
        issues: Vec::new(),
        is_valid: true,
    };

    let song_trimmed = level_song.trim();
    if song_trimmed.is_empty() || song_trimmed.eq_ignore_ascii_case("None") {
        report.issues.push(MusicAuditIssue {
            severity: MusicIssueSeverity::Warning,
            title: "No Default Level Music Set".to_string(),
            description: "The level has no background tracker music assigned in LevelInfo. Map will play silent.".to_string(),
            suggested_fix: "Set Level Song to an iconic UMX track (e.g., 'Foregone.umx' or 'Run.umx') in Level Properties (F6).".to_string(),
        });
    } else {
        report.unique_songs_referenced.push(song_trimmed.to_string());
    }

    if level_section > 8 {
        report.issues.push(MusicAuditIssue {
            severity: MusicIssueSeverity::Warning,
            title: format!("High Starting Song Section ({level_section})"),
            description: "Default Level SongSection is greater than 8. Most UT99 tracker modules only have 2 to 4 sections.".to_string(),
            suggested_fix: "Verify module order table or set default SongSection to 0 or 1.".to_string(),
        });
    }

    // Inspect each MusicEvent
    for event in music_events {
        let ev_song = event.song.trim();
        if !ev_song.is_empty() && !ev_song.eq_ignore_ascii_case("None") {
            if !report.unique_songs_referenced.iter().any(|s| s.eq_ignore_ascii_case(ev_song)) {
                report.unique_songs_referenced.push(ev_song.to_string());
            }
        } else if !event.silence {
            report.issues.push(MusicAuditIssue {
                severity: MusicIssueSeverity::Critical,
                title: format!("MusicEvent '{}' Missing Song Target", event.name),
                description: "MusicEvent has neither a Song specified nor bSilence set to True. It will trigger no music change.".to_string(),
                suggested_fix: "Assign a target UMX song or enable bSilence.".to_string(),
            });
            report.is_valid = false;
        }

        if event.tag.is_empty() {
            report.issues.push(MusicAuditIssue {
                severity: MusicIssueSeverity::Warning,
                title: format!("Orphan MusicEvent '{}' (No Tag)", event.name),
                description: "This MusicEvent has an empty Tag and cannot be activated by triggers, dispatchers, or player proximity.".to_string(),
                suggested_fix: "Assign a Tag (e.g. 'BossMusicTrigger' or 'SecretAreaMusic').".to_string(),
            });
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_music_transitions_durations() {
        assert_eq!(MusicTransition::None.fade_duration_secs(), 0.0);
        assert_eq!(MusicTransition::Instant.fade_duration_secs(), 0.0);
        assert_eq!(MusicTransition::FastFade.fade_duration_secs(), 0.8);
        assert_eq!(MusicTransition::Fade.fade_duration_secs(), 2.0);
        assert_eq!(MusicTransition::SlowFade.fade_duration_secs(), 4.5);
    }

    #[test]
    fn test_crossfade_simulation_smoothness() {
        let from_vol = 1.0;
        let to_vol = 0.8;
        let trans = MusicTransition::Fade; // 2.0 seconds

        let (out_start, in_start) = simulate_crossfade(from_vol, to_vol, trans, 0.0);
        assert_eq!(out_start, 1.0);
        assert_eq!(in_start, 0.0);

        let (out_mid, in_mid) = simulate_crossfade(from_vol, to_vol, trans, 1.0);
        assert!((out_mid - 0.5).abs() < 1e-2);
        assert!((in_mid - 0.4).abs() < 1e-2);

        let (out_end, in_end) = simulate_crossfade(from_vol, to_vol, trans, 2.0);
        assert_eq!(out_end, 0.0);
        assert_eq!(in_end, 0.8);
    }

    #[test]
    fn test_music_presets_and_tracker_info() {
        let presets = MusicTrackPreset::all();
        assert_eq!(presets.len(), 6);

        for p in presets {
            let info = p.to_tracker_info();
            assert!(!info.package.is_empty());
            assert!(!info.title.is_empty());
            assert!(info.channels >= 8);
            assert!(info.default_bpm >= 100);
            assert!(!info.sections.is_empty());
        }
    }

    #[test]
    fn test_audit_level_music() {
        let events = vec![
            MusicEventData {
                index: 0,
                name: "MusicEvent0".to_string(),
                tag: "BossSpawn".to_string(),
                location: Vec3::new(100.0, 200.0, 300.0),
                song: "Mech8".to_string(),
                song_section: 1,
                transition: MusicTransition::FastFade,
                affect_all_players: true,
                once_only: true,
                silence: false,
            },
            MusicEventData {
                index: 1,
                name: "BrokenMusicEvent".to_string(),
                tag: "".to_string(),
                location: Vec3::ZERO,
                song: "".to_string(),
                song_section: 0,
                transition: MusicTransition::Fade,
                affect_all_players: false,
                once_only: false,
                silence: false,
            },
        ];

        let report = audit_level_music("Foregone", 0, &events);
        assert_eq!(report.music_events_count, 2);
        assert_eq!(report.unique_songs_referenced.len(), 2);
        assert!(!report.is_valid, "Event with empty song and no silence must make audit invalid");
        assert!(report.issues.iter().any(|i| i.severity == MusicIssueSeverity::Critical));
    }
}
