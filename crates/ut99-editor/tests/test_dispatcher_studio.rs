use glam::Vec3;
use ut99_bsp::DispatcherPresetTemplate;
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_dispatcher_sequence_timing_and_target_matching() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. Dispatcher Actor
    let mut disp = ActorRecord::default();
    disp.class_name = "Dispatcher".to_string();
    disp.name = "AirlockDispatcher".to_string();
    disp.tag = "TriggerAirlock".to_string();
    disp.location = Vec3::new(100.0, 200.0, 0.0);

    disp.properties.insert(
        "OutDelays[0]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 0,
            value: TaggedValue::Float(0.5),
        },
    );
    disp.properties.insert(
        "OutEvents[0]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 0,
            value: TaggedValue::Str("OuterDoor".to_string()),
        },
    );
    disp.properties.insert(
        "OutDelays[1]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 1,
            value: TaggedValue::Float(1.5),
        },
    );
    disp.properties.insert(
        "OutEvents[1]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 1,
            value: TaggedValue::Str("AlarmSiren".to_string()),
        },
    );
    level.actors.push(disp);

    // 2. Target Mover Door
    let mut door = ActorRecord::default();
    door.class_name = "Mover".to_string();
    door.name = "OuterDoor_1".to_string();
    door.tag = "OuterDoor".to_string();
    door.location = Vec3::new(300.0, 200.0, 0.0);
    level.actors.push(door);

    // 3. Target Siren
    let mut siren = ActorRecord::default();
    siren.class_name = "AmbientSound".to_string();
    siren.name = "Siren_A".to_string();
    siren.tag = "AlarmSiren".to_string();
    siren.location = Vec3::new(100.0, 200.0, 150.0);
    level.actors.push(siren);

    state.loaded_unr = Some(level);

    let sequences = state.get_dispatcher_sequences();
    assert_eq!(sequences.len(), 1);

    let seq = &sequences[0];
    assert_eq!(seq.dispatcher_name, "AirlockDispatcher");
    assert_eq!(seq.tag, "TriggerAirlock");
    assert_eq!(seq.stages.len(), 2);

    // Stage 0: OuterDoor
    assert_eq!(seq.stages[0].delay_secs, 0.5);
    assert_eq!(seq.stages[0].cumulative_time_secs, 0.5);
    assert_eq!(seq.stages[0].event_name, "OuterDoor");
    assert_eq!(seq.stages[0].matched_targets.len(), 1);
    assert_eq!(seq.stages[0].matched_targets[0].0, "OuterDoor_1");

    // Stage 1: AlarmSiren
    assert_eq!(seq.stages[1].delay_secs, 1.5);
    assert_eq!(seq.stages[1].cumulative_time_secs, 2.0); // 0.5 + 1.5
    assert_eq!(seq.stages[1].event_name, "AlarmSiren");
    assert_eq!(seq.stages[1].matched_targets.len(), 1);
    assert_eq!(seq.stages[1].matched_targets[0].0, "Siren_A");

    assert_eq!(seq.total_duration_secs, 2.0);
    assert!(!seq.has_unmatched_events);
    assert!(!seq.has_recursive_loop);
}

#[test]
fn test_dispatcher_hazards_infinite_loop_and_orphans() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut disp = ActorRecord::default();
    disp.class_name = "Dispatcher".to_string();
    disp.name = "HazardDispatcher".to_string();
    disp.tag = "SelfTrigger".to_string();

    // Stage 0 triggers self tag -> Infinite recursion hazard!
    disp.properties.insert(
        "OutDelays[0]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 0,
            value: TaggedValue::Float(1.0),
        },
    );
    disp.properties.insert(
        "OutEvents[0]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 0,
            value: TaggedValue::Str("SelfTrigger".to_string()),
        },
    );

    // Stage 1 triggers nonexistent tag -> Orphan broken link hazard!
    disp.properties.insert(
        "OutDelays[1]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 1,
            value: TaggedValue::Float(0.5),
        },
    );
    disp.properties.insert(
        "OutEvents[1]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 1,
            value: TaggedValue::Str("MissingActorTag".to_string()),
        },
    );

    level.actors.push(disp);
    state.loaded_unr = Some(level);

    let sequences = state.get_dispatcher_sequences();
    assert_eq!(sequences.len(), 1);
    let seq = &sequences[0];

    assert!(seq.has_recursive_loop, "Must detect infinite recursion when OutEvent matches Tag");
    assert!(seq.has_unmatched_events, "Must detect orphan broken events");
}

#[test]
fn test_apply_dispatcher_preset_and_undo_redo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut disp = ActorRecord::default();
    disp.class_name = "Dispatcher".to_string();
    disp.name = "TestDisp".to_string();
    level.actors.push(disp);
    state.loaded_unr = Some(level);

    // 1. Apply Preset: AirlockDecontamination
    let applied = state.apply_dispatcher_preset("TestDisp", DispatcherPresetTemplate::AirlockDecontamination);
    assert!(applied);

    let sequences = state.get_dispatcher_sequences();
    assert_eq!(sequences.len(), 1);
    let seq = &sequences[0];
    assert_eq!(seq.stages.len(), 6);
    assert_eq!(seq.stages[0].event_name, "CloseOuterDoor");
    assert_eq!(seq.stages[1].event_name, "DeconSprayAudio");
    assert_eq!(seq.stages[1].delay_secs, 1.2);

    // 2. Modify stage with update_dispatcher_stage
    let stage_updated = state.update_dispatcher_stage("TestDisp", 1, 3.5, "HeavySprayAudio");
    assert!(stage_updated);

    let sequences_after_edit = state.get_dispatcher_sequences();
    assert_eq!(sequences_after_edit[0].stages[1].delay_secs, 3.5);
    assert_eq!(sequences_after_edit[0].stages[1].event_name, "HeavySprayAudio");

    // 3. Undo edit
    state.undo();
    let sequences_undone_edit = state.get_dispatcher_sequences();
    assert_eq!(sequences_undone_edit[0].stages[1].delay_secs, 1.2);
    assert_eq!(sequences_undone_edit[0].stages[1].event_name, "DeconSprayAudio");

    // 4. Undo preset application
    state.undo();
    let sequences_undone_preset = state.get_dispatcher_sequences();
    assert_eq!(sequences_undone_preset[0].stages.len(), 0);

    // 5. Redo preset application
    state.redo();
    let sequences_redone = state.get_dispatcher_sequences();
    assert_eq!(sequences_redone[0].stages.len(), 6);
}

#[test]
fn test_cascading_dispatcher_chains_and_timeline_scrubbing() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // Dispatcher A
    let mut disp_a = ActorRecord::default();
    disp_a.class_name = "Dispatcher".to_string();
    disp_a.name = "Disp_A".to_string();
    disp_a.tag = "StartSequence".to_string();
    disp_a.properties.insert(
        "OutDelays[0]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 0,
            value: TaggedValue::Float(1.0),
        },
    );
    disp_a.properties.insert(
        "OutEvents[0]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 0,
            value: TaggedValue::Str("Invoke_B".to_string()),
        },
    );
    level.actors.push(disp_a);

    // Dispatcher B
    let mut disp_b = ActorRecord::default();
    disp_b.class_name = "Dispatcher".to_string();
    disp_b.name = "Disp_B".to_string();
    disp_b.tag = "Invoke_B".to_string();
    disp_b.properties.insert(
        "OutDelays[0]".to_string(),
        TaggedProperty {
            name: "OutDelays".to_string(),
            array_index: 0,
            value: TaggedValue::Float(2.0),
        },
    );
    disp_b.properties.insert(
        "OutEvents[0]".to_string(),
        TaggedProperty {
            name: "OutEvents".to_string(),
            array_index: 0,
            value: TaggedValue::Str("EndGoalDoor".to_string()),
        },
    );
    level.actors.push(disp_b);

    state.loaded_unr = Some(level);

    // 1. Detect cascading forward chain (A -> B)
    let chains = state.detect_cascading_dispatcher_chains();
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].source_dispatcher, "Disp_A");
    assert_eq!(chains[0].target_dispatcher, "Disp_B");
    assert!(!chains[0].is_circular, "Forward chain is linear and safe");

    // 2. Add circular link: B triggers A -> circular loop hazard
    state.update_dispatcher_stage("Disp_B", 1, 0.5, "StartSequence");
    let circular_chains = state.detect_cascading_dispatcher_chains();
    assert!(circular_chains.iter().any(|c| c.is_circular), "Must detect circular deadlock loop");

    // 3. Timeline scrubbing simulation clock
    state.dispatcher_settings.selected_dispatcher_name = Some("Disp_A".to_string());
    state.dispatcher_settings.is_playing = true;
    state.dispatcher_settings.play_speed = 1.0;
    state.dispatcher_settings.timeline_time = 0.0;
    state.dispatcher_settings.loop_preview = false;

    state.update_dispatcher_timeline(0.6);
    assert!((state.dispatcher_settings.timeline_time - 0.6).abs() < 1e-3);
    assert!(state.dispatcher_settings.is_playing);

    // Advance beyond total duration (1.0s)
    state.update_dispatcher_timeline(0.8);
    assert_eq!(state.dispatcher_settings.timeline_time, 1.0);
    assert!(!state.dispatcher_settings.is_playing, "Timeline should stop at end when loop is disabled");
}
