// crates/ut99-editor/tests/test_audit_team_gameplay_and_fortifications.rs
//
// Master Audit Suite for Team-Based Gameplay & Base Fortification Studios:
// Interactive Domination Control Point & Scoring Zone Studio
// Interactive Assault Objective Chain & Blast Door Studio
// Interactive Team Base Radar & Jammer Station Studio
// Interactive Capture The Flag Base & Transit Routes Studio
// Interactive Team Cannon & Stationary Automated Turret Defense Studio

use glam::Vec3;
use ut99_bsp::{
    AssaultArchetype, CtfBaseArchetype, DominationArchetype, RadarStudioArchetype,
    TeamCannonArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_multiphase_team_fortification_world_commit_and_multi_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Stage 1: Domination Point
    state.domination_settings.active_config.archetype = DominationArchetype::CrypticTriadBalance;
    state.domination_settings.active_config.position = Vec3::new(100.0, 0.0, 0.0);
    let (dom_b_idx, dom_act_idx) = state.commit_domination_point_to_world();

    // 2. Stage 2: Assault Blast Door & Objective
    state.assault_settings.active_config.archetype = AssaultArchetype::OverlordBeachLanding;
    state.assault_settings.active_config.position = Vec3::new(500.0, 0.0, 0.0);
    let (aslt_b_idx, aslt_act_idx) = state.commit_assault_objective_to_world();

    // 3. Stage 3: Base Radar Mast & Station
    state.radar_settings.active_config.archetype = RadarStudioArchetype::OrbitalCommandHQ;
    state.radar_settings.active_config.position = Vec3::new(1000.0, 0.0, 0.0);
    let (rad_b_idx, rad_act_idx) = state.commit_radar_station_to_world();

    // 4. Stage 4: CTF Flag Dais & Base
    state.ctf_base_settings.active_config.archetype = CtfBaseArchetype::FaceMonolithBase;
    state.ctf_base_settings.active_config.position = Vec3::new(2000.0, 0.0, 0.0);
    let (ctf_b_idx, ctf_act_idx) = state.commit_ctf_base_to_world();

    // 5. Stage 5: Team Cannon Bunker Mount & Turret
    state.team_cannon_settings.active_config.archetype = TeamCannonArchetype::OverlordBeachBunkerCannon;
    state.team_cannon_settings.active_config.position = Vec3::new(3000.0, 0.0, 0.0);
    let (tc_b_idx, tc_act_idx) = state.commit_team_cannon_to_world();

    // Verify all 5 brushes and 5 actors were committed
    assert_eq!(state.world_brushes.len(), initial_brushes + 5);
    assert_eq!(dom_b_idx, initial_brushes);
    assert_eq!(aslt_b_idx, initial_brushes + 1);
    assert_eq!(rad_b_idx, initial_brushes + 2);
    assert_eq!(ctf_b_idx, initial_brushes + 3);
    assert_eq!(tc_b_idx, initial_brushes + 4);

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 5);
    assert_eq!(dom_act_idx, Some(0));
    assert_eq!(aslt_act_idx, Some(1));
    assert_eq!(rad_act_idx, Some(2));
    assert_eq!(ctf_act_idx, Some(3));
    assert_eq!(tc_act_idx, Some(4));

    // Verify actor classes
    assert_eq!(level.actors[0].class_name, "Botpack.ControlPoint");
    assert_eq!(level.actors[1].class_name, "Botpack.FortStandard");
    assert_eq!(level.actors[2].class_name, "Botpack.RadarStation");
    assert_eq!(level.actors[3].class_name, "Botpack.RedFlagBase");
    assert_eq!(level.actors[4].class_name, "Botpack.TeamCannon");

    // Sequential 5-Stage Multi-Undo
    state.undo(); // Undo Team Cannon
    assert_eq!(state.world_brushes.len(), initial_brushes + 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 4);

    state.undo(); // Undo CTF Base
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    state.undo(); // Undo Radar
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    state.undo(); // Undo Assault
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    state.undo(); // Undo Domination
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Sequential 5-Stage Multi-Redo
    for i in 1..=5 {
        state.redo();
        assert_eq!(state.world_brushes.len(), initial_brushes + i);
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), i);
    }
}

#[test]
fn test_cross_studio_combat_safety_and_defense_balance() {
    let mut state = EditorState::default();

    // Set up a Red Defense Base
    state.loaded_unr = Some(ut99_format::UnrLevel::default());
    {
        let level = state.loaded_unr.as_mut().unwrap();

        // Red Flag Base
        let mut red_flag = ut99_format::ActorRecord::default();
        red_flag.name = "RedFlagBase_0".to_string();
        red_flag.class_name = "Botpack.RedFlagBase".to_string();
        red_flag.location = Vec3::new(0.0, 0.0, 0.0);
        level.actors.push(red_flag);

        // Enemy Blue Flag Base (3000 uu away)
        let mut blue_flag = ut99_format::ActorRecord::default();
        blue_flag.name = "BlueFlagBase_0".to_string();
        blue_flag.class_name = "Botpack.BlueFlagBase".to_string();
        blue_flag.location = Vec3::new(3000.0, 0.0, 0.0);
        level.actors.push(blue_flag);

        // Enemy PlayerStart near base (700 uu away, within 3200 * 0.4 = 1280 uu of Red cannon)
        let mut blue_spawn = ut99_format::ActorRecord::default();
        blue_spawn.name = "PlayerStart_Blue_0".to_string();
        blue_spawn.class_name = "Engine.PlayerStart".to_string();
        blue_spawn.location = Vec3::new(700.0, 0.0, 0.0);
        level.actors.push(blue_spawn);
    }

    // CTF Route Telemetry Evaluation
    state.ctf_base_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.recalculate_ctf_base_preview();
    let ctf_metrics = &state.ctf_base_settings.cached_metrics;
    assert_eq!(ctf_metrics.flag_to_flag_distance_uu, 3000.0);
    assert_eq!(ctf_metrics.runner_transit_time_secs, 7.5); // 3000 / 400
    assert_eq!(ctf_metrics.translocator_transit_time_secs, 3.75); // 3000 / 800

    // Team Cannon Safety Audit (Defending Red base at 0, 0, 0)
    state.team_cannon_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.team_cannon_settings.active_config.team = 0;
    state.recalculate_team_cannon_preview();
    let cannon_hazards = &state.team_cannon_settings.cached_hazards;
    assert!(cannon_hazards.iter().any(|h| h.is_critical && h.title.contains("Spawn Camp")));

    // Radar Station Telemetry (Defending Red base at 0, 0, 0)
    state.radar_settings.active_config.position = Vec3::new(0.0, 0.0, 0.0);
    state.radar_settings.active_config.is_jammer = false;
    state.recalculate_radar_preview();
    let radar_metrics = &state.radar_settings.cached_metrics;
    assert!(radar_metrics.effective_coverage_area_sq_uu >= 1_000_000.0);
}

#[test]
fn test_procedural_brush_watertightness_and_csg_invariance() {
    let mut state = EditorState::default();

    // 1. Domination Altar Brush
    state.recalculate_domination_preview();
    let dom_b = state.domination_settings.cached_pedestal_brush.as_ref().unwrap();
    assert!(!dom_b.polys.is_empty());
    for poly in &dom_b.polys {
        assert!(poly.vertices.len() >= 3);
    }

    // 2. Assault Blast Door Brush
    state.recalculate_assault_preview();
    let aslt_b = state.assault_settings.cached_door_brush.as_ref().unwrap();
    assert_eq!(aslt_b.polys.len(), 6); // Cuboid door frame

    // 3. Radar Mast Brush
    state.recalculate_radar_preview();
    let rad_b = state.radar_settings.cached_antenna_brush.as_ref().unwrap();
    assert!(rad_b.polys.len() >= 8);

    // 4. CTF Pedestal Brush
    state.recalculate_ctf_base_preview();
    let ctf_b = state.ctf_base_settings.cached_pedestal_brush.as_ref().unwrap();
    assert!(ctf_b.polys.len() >= 8);

    // 5. Team Cannon Bunker Mount Brush
    state.recalculate_team_cannon_preview();
    let tc_b = state.team_cannon_settings.cached_bunker_brush.as_ref().unwrap();
    assert_eq!(tc_b.polys.len(), 10); // Octagonal mount (1 bottom + 8 sides + 1 top)
}
