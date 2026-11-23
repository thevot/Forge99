// crates/ut99-editor/tests/test_audit_defenses_and_logic_studios.rs
//
// Master Audit Suite for Defenses, Hazards & Logic Systems:
// Interactive Capture The Flag Base Studio
// Interactive Team Cannon & Turret Defense Studio
// Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
// Interactive Pressure Plate & Proximity Trigger Network Studio

use glam::Vec3;
use ut99_bsp::{
    BioTrapArchetype, CannonWeaponType, CtfBaseArchetype, TeamCannonArchetype,
    TriggerNetworkArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_audit_cross_studio_commit_and_multi_stage_cascading_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // -------------------------------------------------------------------------
    // Stage 1: Commit CTF Flag Base
    // -------------------------------------------------------------------------
    state.ctf_base_settings.active_config = CtfBaseArchetype::FaceMonolithBase.to_config();
    let (ctf_brush, ctf_actor) = state.commit_ctf_base_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(ctf_brush, initial_brushes);
    assert_eq!(ctf_actor, Some(0));

    // -------------------------------------------------------------------------
    // Stage 2: Commit Team Cannon Turret
    // -------------------------------------------------------------------------
    state.team_cannon_settings.active_config = TeamCannonArchetype::OverlordBeachBunkerCannon.to_config();
    state.team_cannon_settings.active_config.position = Vec3::new(500.0, 0.0, 0.0);
    let (cannon_brush, cannon_actor) = state.commit_team_cannon_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(cannon_brush, initial_brushes + 1);
    assert_eq!(cannon_actor, Some(1));

    // -------------------------------------------------------------------------
    // Stage 3: Commit Bio-Rifle Slime Trap
    // -------------------------------------------------------------------------
    state.bio_trap_settings.active_config = BioTrapArchetype::ChokepointDoorwayAmbushTrap.to_config();
    state.bio_trap_settings.active_config.position = Vec3::new(250.0, 100.0, 0.0);
    let (bio_brush, bio_actor) = state.commit_bio_trap_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(bio_brush, initial_brushes + 2);
    assert_eq!(bio_actor, Some(2));

    // -------------------------------------------------------------------------
    // Stage 4: Commit Pressure Plate Trigger Network
    // -------------------------------------------------------------------------
    state.trigger_network_settings.active_config = TriggerNetworkArchetype::PressureChamberDecompressorSwitch.to_config();
    state.trigger_network_settings.active_config.position = Vec3::new(100.0, -100.0, 0.0);
    let (trig_brush, trig_actor) = state.commit_trigger_network_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(trig_brush, initial_brushes + 3);
    assert_eq!(trig_actor, Some(3));

    // Verify all 4 actors present in UNR level
    {
        let unr = state.loaded_unr.as_ref().unwrap();
        assert_eq!(unr.actors.len(), 4);
        assert_eq!(unr.actors[0].class_name, "Botpack.RedFlagBase");
        assert_eq!(unr.actors[1].class_name, "Botpack.TeamCannon");
        assert_eq!(unr.actors[2].class_name, "Botpack.BioGel");
        assert_eq!(unr.actors[3].class_name, "Engine.Trigger");
    }

    // -------------------------------------------------------------------------
    // Cascading Multi-Stage Undo (4 Stages)
    // -------------------------------------------------------------------------
    // Undo 1: Revert Trigger Network
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[2].class_name, "Botpack.BioGel");

    // Undo 2: Revert Bio Trap
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[1].class_name, "Botpack.TeamCannon");

    // Undo 3: Revert Team Cannon
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].class_name, "Botpack.RedFlagBase");

    // Undo 4: Revert CTF Base
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // -------------------------------------------------------------------------
    // Cascading Multi-Stage Redo (4 Stages)
    // -------------------------------------------------------------------------
    // Redo 1: Restore CTF Base
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].class_name, "Botpack.RedFlagBase");

    // Redo 2: Restore Team Cannon
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[1].class_name, "Botpack.TeamCannon");

    // Redo 3: Restore Bio Trap
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[2].class_name, "Botpack.BioGel");

    // Redo 4: Restore Trigger Network
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[3].class_name, "Engine.Trigger");
}

#[test]
fn test_audit_defenses_and_hazards_cross_safety_evaluation() {
    let mut state = EditorState::default();

    // Setup an initial UNR level with base actors
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        // Player Start at origin
        let mut p_start = ut99_format::ActorRecord::default();
        p_start.name = "PlayerStart_Blue_0".to_string();
        p_start.class_name = "Engine.PlayerStart".to_string();
        p_start.location = Vec3::new(0.0, 0.0, 0.0);
        level.actors.push(p_start);

        // Recipient mover with loopback event
        let mut loop_door = ut99_format::ActorRecord::default();
        loop_door.name = "LoopDoor".to_string();
        loop_door.class_name = "Engine.Mover".to_string();
        loop_door.tag = "BlastDoorTrigger".to_string();
        loop_door.event = "BlastDoorTrigger".to_string();
        loop_door.location = Vec3::new(300.0, 0.0, 0.0);
        level.actors.push(loop_door);
    }

    // 1. Audit Bio Trap placed directly near PlayerStart (50 uu distance < 320 uu)
    state.bio_trap_settings.active_config.position = Vec3::new(50.0, 0.0, 0.0);
    state.recalculate_bio_trap_preview();
    assert!(state.bio_trap_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Point Contamination")));

    // 2. Audit Team Cannon with sluggish tracking
    state.team_cannon_settings.active_config.cannon_type = CannonWeaponType::PulseLaserTurret;
    state.team_cannon_settings.active_config.tracking_speed_deg_per_sec = 20.0; // Slow tracking
    state.recalculate_team_cannon_preview();
    assert!(state.team_cannon_settings.cached_hazards.iter().any(|h| h.title.contains("Sluggish Tracking")));

    // 3. Audit Trigger Network loopback detection
    state.trigger_network_settings.active_config.event_name = "BlastDoorTrigger".to_string();
    state.recalculate_trigger_network_preview();
    assert!(state.trigger_network_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Trigger Feedback Loop")));
}
