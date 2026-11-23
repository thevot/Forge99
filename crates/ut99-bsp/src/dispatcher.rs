use glam::Vec3;

/// A single step in the Dispatcher's sequential execution pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct DispatcherStage {
    pub index: usize,
    pub delay_secs: f32,
    pub cumulative_time_secs: f32,
    pub event_name: String,
    pub matched_targets: Vec<(String, Vec3)>, // (Actor name, location)
}

/// Structured chronological sequence for an Unreal Tournament Dispatcher actor
#[derive(Debug, Clone, PartialEq)]
pub struct DispatcherSequence {
    pub dispatcher_name: String,
    pub tag: String,
    pub location: Vec3,
    pub stages: Vec<DispatcherStage>,
    pub total_duration_secs: f32,
    pub has_unmatched_events: bool,
    pub has_recursive_loop: bool,
}

/// Parses and validates a Dispatcher actor's timeline against map actors
pub fn build_dispatcher_sequence(
    dispatcher_name: &str,
    tag: &str,
    location: Vec3,
    out_delays: &[f32],
    out_events: &[String],
    all_actors: &[(String, String, Vec3)], // (name, tag, location)
) -> DispatcherSequence {
    let mut stages = Vec::new();
    let mut cum_time = 0.0;
    let mut has_unmatched = false;
    let mut has_loop = false;

    let count = out_events.len().min(8);

    for i in 0..count {
        let event_name = &out_events[i];
        if event_name.is_empty() || event_name.eq_ignore_ascii_case("None") {
            continue;
        }

        let delay = if i < out_delays.len() { out_delays[i].max(0.0) } else { 0.0 };
        cum_time += delay;

        // Check if event triggers self (infinite loop hazard)
        if !tag.is_empty() && event_name.eq_ignore_ascii_case(tag) {
            has_loop = true;
        }

        // Find receiver actors with matching Tag
        let mut matched = Vec::new();
        for (tgt_name, tgt_tag, tgt_loc) in all_actors {
            if tgt_name != dispatcher_name && tgt_tag.eq_ignore_ascii_case(event_name) {
                matched.push((tgt_name.clone(), *tgt_loc));
            }
        }

        if matched.is_empty() {
            has_unmatched = true;
        }

        stages.push(DispatcherStage {
            index: i,
            delay_secs: delay,
            cumulative_time_secs: cum_time,
            event_name: event_name.clone(),
            matched_targets: matched,
        });
    }

    DispatcherSequence {
        dispatcher_name: dispatcher_name.to_string(),
        tag: tag.to_string(),
        location,
        total_duration_secs: cum_time,
        stages,
        has_unmatched_events: has_unmatched,
        has_recursive_loop: has_loop,
    }
}

/// Link in a multi-stage cascading dispatcher pipeline
#[derive(Debug, Clone, PartialEq)]
pub struct DispatcherChainLink {
    pub source_dispatcher: String,
    pub target_dispatcher: String,
    pub stage_index: usize,
    pub event_name: String,
    pub trigger_time: f32,
    pub is_circular: bool,
}

/// Detects cascading triggers across multiple Dispatcher sequences in the level
pub fn detect_cascading_dispatcher_chains(sequences: &[DispatcherSequence]) -> Vec<DispatcherChainLink> {
    let mut chains = Vec::new();
    for source in sequences {
        for stage in &source.stages {
            for target in sequences {
                if !target.tag.is_empty() && stage.event_name.eq_ignore_ascii_case(&target.tag) {
                    let is_circular = source.dispatcher_name == target.dispatcher_name
                        || sequences.iter().any(|other| {
                            other.dispatcher_name == target.dispatcher_name
                                && other.stages.iter().any(|st| st.event_name.eq_ignore_ascii_case(&source.tag))
                        });
                    chains.push(DispatcherChainLink {
                        source_dispatcher: source.dispatcher_name.clone(),
                        target_dispatcher: target.dispatcher_name.clone(),
                        stage_index: stage.index,
                        event_name: stage.event_name.clone(),
                        trigger_time: stage.cumulative_time_secs,
                        is_circular,
                    });
                }
            }
        }
    }
    chains
}

/// Canonical scripted cinematic and interactive trigger sequence templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatcherPresetTemplate {
    TimedBlastAndBreach,
    AirlockDecontamination,
    TrapAmbushLockdown,
    CinematicLightSequence,
    EarthquakeTremorAndCollapse,
    MultiFloorElevatorSequence,
    SecretPushWallAndReveal,
}

impl DispatcherPresetTemplate {
    pub fn label(&self) -> &'static str {
        match self {
            DispatcherPresetTemplate::TimedBlastAndBreach => "Timed Blast & Breach (Explosive Entry)",
            DispatcherPresetTemplate::AirlockDecontamination => "Airlock Decontamination (Dual-Door Cycle)",
            DispatcherPresetTemplate::TrapAmbushLockdown => "Trap Ambush Lockdown (Grates & Lights)",
            DispatcherPresetTemplate::CinematicLightSequence => "Cinematic Light Strobe (Emergency Siren)",
            DispatcherPresetTemplate::EarthquakeTremorAndCollapse => "Earthquake Tremor & Collapse (SpecialEvent)",
            DispatcherPresetTemplate::MultiFloorElevatorSequence => "Multi-Floor Elevator Sequence (Callers & Chimes)",
            DispatcherPresetTemplate::SecretPushWallAndReveal => "Secret Push-Wall & Reveal (Puzzle Chamber)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DispatcherPresetTemplate::TimedBlastAndBreach => "Alarm siren -> Red alert strobe -> Wall explosive breach -> Smoke emitter -> Blast door open.",
            DispatcherPresetTemplate::AirlockDecontamination => "Close exterior door -> Spray sound & vapor -> 3s quarantine pause -> Open interior clean room door.",
            DispatcherPresetTemplate::TrapAmbushLockdown => "Step trigger -> 0.5s latch -> Drop steel grates -> Kill ambient lights -> Spawn monster / powerup reveal.",
            DispatcherPresetTemplate::CinematicLightSequence => "Cascading 8-phase runway strobe lighting sequence for landing pads and industrial tunnels.",
            DispatcherPresetTemplate::EarthquakeTremorAndCollapse => "Distant tremor rumble -> SpecialEvent screen shake -> Ceiling dust emitter -> Structural boulder collapse -> Warning klaxon.",
            DispatcherPresetTemplate::MultiFloorElevatorSequence => "Elevator lift depart keyframe -> Floor arrival chime -> 3s boarding pause -> Upper floor travel -> Gate open.",
            DispatcherPresetTemplate::SecretPushWallAndReveal => "Stone grinder friction audio -> Push wall recess movement -> Hidden shrine lighting on -> Relic / powerup spawn.",
        }
    }

    pub fn stages(&self) -> Vec<(f32, &'static str)> {
        match self {
            DispatcherPresetTemplate::TimedBlastAndBreach => vec![
                (0.0, "SirenAudio"),
                (1.5, "RedStrobeOn"),
                (2.0, "DetonationFlash"),
                (0.2, "ExplosiveBreach"),
                (0.5, "SmokeVaporEmitter"),
                (1.0, "OpenBlastDoor"),
            ],
            DispatcherPresetTemplate::AirlockDecontamination => vec![
                (0.0, "CloseOuterDoor"),
                (1.2, "DeconSprayAudio"),
                (0.5, "DeconVaporOn"),
                (2.5, "DeconVaporOff"),
                (0.8, "ChimeAudio"),
                (0.5, "OpenInnerDoor"),
            ],
            DispatcherPresetTemplate::TrapAmbushLockdown => vec![
                (0.0, "TriggerAudio"),
                (0.5, "DropFrontGrate"),
                (0.1, "DropRearGrate"),
                (0.4, "KillMainLight"),
                (0.2, "EmergencyRedLight"),
                (0.8, "AmbushMonsterWake"),
            ],
            DispatcherPresetTemplate::CinematicLightSequence => vec![
                (0.0, "StrobePhase0"),
                (0.25, "StrobePhase1"),
                (0.25, "StrobePhase2"),
                (0.25, "StrobePhase3"),
                (0.25, "StrobePhase4"),
                (0.25, "StrobePhase5"),
                (0.25, "StrobePhase6"),
                (0.25, "StrobePhase7"),
            ],
            DispatcherPresetTemplate::EarthquakeTremorAndCollapse => vec![
                (0.0, "SubBassRumble"),
                (1.0, "SpecialEventShake"),
                (0.4, "CeilingDustFall"),
                (1.2, "BoulderCollapse"),
                (0.3, "EmergencyKlaxon"),
                (0.8, "FlickerLights"),
            ],
            DispatcherPresetTemplate::MultiFloorElevatorSequence => vec![
                (0.0, "ElevatorDepart"),
                (2.5, "Floor1Chime"),
                (0.5, "Floor1DoorsOpen"),
                (3.0, "Floor1DoorsClose"),
                (0.5, "Floor2Depart"),
                (2.5, "Floor2Chime"),
            ],
            DispatcherPresetTemplate::SecretPushWallAndReveal => vec![
                (0.0, "StoneGrindAudio"),
                (0.3, "PushWallRecess"),
                (1.5, "ShrineLightsOn"),
                (0.5, "ChamberAmbientAudio"),
                (0.4, "SpawnSecretPowerup"),
            ],
        }
    }

    pub fn all() -> &'static [DispatcherPresetTemplate] {
        &[
            DispatcherPresetTemplate::TimedBlastAndBreach,
            DispatcherPresetTemplate::AirlockDecontamination,
            DispatcherPresetTemplate::TrapAmbushLockdown,
            DispatcherPresetTemplate::CinematicLightSequence,
            DispatcherPresetTemplate::EarthquakeTremorAndCollapse,
            DispatcherPresetTemplate::MultiFloorElevatorSequence,
            DispatcherPresetTemplate::SecretPushWallAndReveal,
        ]
    }
}

/// A sequential RoundRobin trigger dispatcher (from UT99 Engine.RoundRobin)
#[derive(Debug, Clone, PartialEq)]
pub struct RoundRobinSequence {
    pub name: String,
    pub tag: String,
    pub out_events: Vec<String>,
    pub current_step: usize,
    pub b_loop: bool,
}

impl RoundRobinSequence {
    pub fn new(name: impl Into<String>, tag: impl Into<String>, events: Vec<String>, b_loop: bool) -> Self {
        Self {
            name: name.into(),
            tag: tag.into(),
            out_events: events,
            current_step: 0,
            b_loop,
        }
    }

    /// Triggers the next stage in the RoundRobin sequence, returning the fired event name if any
    pub fn trigger_next(&mut self) -> Option<&str> {
        if self.out_events.is_empty() {
            return None;
        }
        if self.current_step >= self.out_events.len() {
            if self.b_loop {
                self.current_step = 0;
            } else {
                return None;
            }
        }
        let evt = &self.out_events[self.current_step];
        self.current_step += 1;
        if self.current_step >= self.out_events.len() && self.b_loop {
            self.current_step = 0;
        }
        Some(evt.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_timeline_parsing() {
        let delays = vec![0.5, 1.5, 2.0];
        let events = vec!["DoorOpen".to_string(), "AlarmSiren".to_string(), "Explosion".to_string()];

        let actors = vec![
            ("Door_1".to_string(), "DoorOpen".to_string(), Vec3::new(100.0, 0.0, 0.0)),
            ("Siren_A".to_string(), "AlarmSiren".to_string(), Vec3::new(200.0, 0.0, 50.0)),
            ("Siren_B".to_string(), "AlarmSiren".to_string(), Vec3::new(300.0, 0.0, 50.0)),
        ];

        let seq = build_dispatcher_sequence(
            "MainDispatcher",
            "StartSequence",
            Vec3::ZERO,
            &delays,
            &events,
            &actors,
        );

        assert_eq!(seq.stages.len(), 3);
        assert_eq!(seq.stages[0].cumulative_time_secs, 0.5);
        assert_eq!(seq.stages[0].matched_targets.len(), 1);

        assert_eq!(seq.stages[1].cumulative_time_secs, 2.0); // 0.5 + 1.5
        assert_eq!(seq.stages[1].matched_targets.len(), 2); // Siren_A and Siren_B

        assert_eq!(seq.stages[2].cumulative_time_secs, 4.0); // 2.0 + 2.0
        assert_eq!(seq.stages[2].matched_targets.len(), 0); // Unmatched "Explosion"
        assert!(seq.has_unmatched_events);
        assert!(!seq.has_recursive_loop);
        assert_eq!(seq.total_duration_secs, 4.0);
    }

    #[test]
    fn test_dispatcher_recursive_loop_hazard() {
        let delays = vec![1.0];
        let events = vec!["TriggerSelf".to_string()];
        let actors = vec![];

        let seq = build_dispatcher_sequence(
            "LoopDispatcher",
            "TriggerSelf",
            Vec3::ZERO,
            &delays,
            &events,
            &actors,
        );

        assert!(seq.has_recursive_loop);
    }
}
