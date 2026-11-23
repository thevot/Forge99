pub mod acoustics;
pub mod alignment;
pub mod arch;
pub mod bsp;
pub mod brush;
pub(crate) mod bvh;
pub mod class_profiles;
#[rustfmt::skip]
pub mod class_profiles_data;
pub mod clipping;
pub mod diagnostics;
pub mod dispatcher;
pub mod gameplay_balance;
pub mod gltf_export;
pub mod lighting;
pub mod math;
pub mod math_eval;
pub mod mover_anim;
pub mod navigation;
pub mod placement;
pub mod obj;
pub mod bot_combat;
pub mod ctf_symmetry;
pub mod particles;
pub mod poly;
pub mod procedural_texture;
pub mod projector;
pub mod shadow_gradient;
pub mod shape2d;
pub mod skeletal_mesh;
pub use skeletal_mesh::*;
pub mod zone_pvs;
pub mod zone_seal;
pub mod architectural_prefab;
pub mod terrain;
pub mod mesh_lod_collision;
pub mod cinematic_camera;
pub mod music_sequencer;
pub mod corona_flare;
pub mod fluid_physics;
pub mod weapon_ballistics;
pub mod teleporter_network;
pub mod translocator_physics;
pub mod lift_pathing;
pub mod water_surface;
pub mod mover_studio;
pub mod team_objectives;
pub mod destructible;
pub mod kicker_physics;
pub mod bsp_quality;
pub mod shape_loft;
pub mod stairway;
pub mod mesh_scatter;
pub mod light_shaft;
pub mod track_mover;
pub mod surveillance;
pub mod laser_grid;
pub mod ventilator_fan;
pub mod conveyor_belt;
pub mod auto_turret;
pub mod soundscape;
pub mod hazard_chamber;
pub mod pipe_conduit;
pub mod hologram;
pub mod weather_studio;
pub mod arena_rules;
pub mod armor_stacking;
pub mod bot_jump_routes;
pub mod weapon_circuit;
pub mod powerup_control;
pub mod spawn_distribution;
pub mod domination_studio;
pub mod domination_mission;
pub use domination_mission::{
    generate_point_names, plan_domination_mission, DominationDaisStyle, DominationMissionPlan,
    DominationNamingStyle, DominationPointPlan, DominationPointRole,
};
pub mod ctf_mission;
pub use ctf_mission::{
    plan_ctf_mission, CtfAlternatePath, CtfDaisStyle, CtfDefensePoint, CtfMissionPlan, CtfNode,
    CtfRouteStrategy, CtfSniperPerch,
};
pub mod assault_studio;
pub mod assault_mission;
pub use assault_mission::{
    audit_assault_plan, briefing_lines, node_adjacency, plan_assault_mission, reachable_from,
    AssaultMissionPlan, AssaultObjectiveKind, AssaultObjectivePlan, MissionNode,
};
pub mod radar_studio;
pub mod ctf_studio;
pub mod team_cannon_studio;
pub mod bio_trap_studio;
pub mod trigger_network_studio;
pub mod elevator_studio;
pub mod zone_velocity_studio;
pub mod hazard_barrel_studio;
pub mod special_lit_studio;
pub mod sniper_vantage_studio;
pub mod secret_door_studio;
pub mod teleporter_arch_studio;
pub mod fluid_basin_studio;
pub mod alarm_beacon_studio;
pub mod supply_locker_studio;
pub mod skylight_studio;
pub mod vent_duct_studio;
pub mod piston_studio;
pub mod jump_boots_studio;
pub mod force_field_studio;
pub mod reactor_core_studio;
pub mod armory;
pub mod map_wizard;
pub mod skin_studio;
pub use skin_studio::*;
pub mod actorx;
pub use actorx::*;
pub mod asset_extractor;
pub use asset_extractor::*;
pub mod unrealscript_exec;
pub use unrealscript_exec::*;

pub use armory::{
    ammo_for_weapon, find_powerup, find_weapon, PowerupCategory, PowerupDescriptor,
    QuotaItemEntry, WeaponCategory, WeaponDescriptor, WizardItemDensity, WizardItemQuotaConfig,
    WizardPowerupPreset, WizardWeaponPreset, CLASSIC_UNREAL_WEAPONS, STOCK_POWERUPS_AND_PICKUPS,
    STOCK_TOURNAMENT_WEAPONS,
};

pub use map_wizard::{GENERATOR_VERSION_HEIGHTS, GENERATOR_VERSION_LEGACY, GENERATOR_VERSION_PORTALS, 
    audit_wizard_teleporters, auto_fix_wizard_teleporters, collect_wizard_teleporters,
    generate_random_map_identity, generate_valid_ut99_map, next_wizard_entropy_seed,
    preview_wizard_topology, with_deterministic_wizard_entropy, MapWizardConfig, WizardActorSpec, WizardArchetype,
    WizardArchetypeCategory, WizardCustomConnection, WizardCustomMarker, WizardCustomRoom,
    WizardFeatureToggles, WizardGameMode, WizardGeneratedMap, WizardLayoutStrategy,
    WizardMapIdentity, WizardMapScale, WizardMoverSpec, WizardPreviewMarker2D, WizardPreviewRoom2D,
    WizardResolvedPalette, WizardRoomOverride, WizardRoomShape, WizardSymmetryMode,
    WizardTeleportAuditSummary, WizardTeleportEntry, WizardTeleportIssue, WizardTeleportIssueKind,
    WizardTeleportIssueSeverity, WizardTeleportLinkPreview,
    WizardThemePreset, WizardTopologyPreview, WizardFogPreset, WizardRoomEnvironment,
    WizardArchitecturalDetail, UT99_MUSIC_TRACKS,
};

pub use force_field_studio::{
    audit_force_field_safety, evaluate_force_field_metrics, generate_force_field_brushes,
    ForceFieldArchetype, ForceFieldColorPalette, ForceFieldConfig, ForceFieldHazard,
    ForceFieldMetrics, ForceFieldPermeability, LevelForceFieldContextActorRecord,
};

pub use reactor_core_studio::{
    audit_reactor_core_safety, evaluate_reactor_core_metrics, generate_reactor_core_brushes,
    LevelReactorContextActorRecord, ReactorCoreArchetype, ReactorCoreConfig, ReactorCoreHazard,
    ReactorCoreMetrics, ReactorHazardType,
};

pub use piston_studio::{
    audit_piston_safety, evaluate_piston_metrics, generate_piston_brush,
    LevelPistonContextActorRecord, PistonArchetype, PistonConfig, PistonEncroachAction,
    PistonHazard, PistonMetrics, PistonOrientation,
};

pub use jump_boots_studio::{
    audit_jump_boots_safety, evaluate_jump_boots_metrics, generate_jump_boots_brush,
    JumpBootsArchetype, JumpBootsConfig, JumpBootsHazard, JumpBootsMetrics,
    LevelJumpBootsContextActorRecord,
};

pub use skylight_studio::{
    audit_skylight_safety, evaluate_skylight_metrics, generate_skylight_brush,
    LevelSkylightContextActorRecord, SkylightArchetype, SkylightConfig, SkylightHazard,
    SkylightMetrics,
};

pub use vent_duct_studio::{
    audit_vent_duct_safety, evaluate_vent_duct_metrics, generate_vent_duct_brush,
    LevelVentContextActorRecord, VentDuctArchetype, VentDuctConfig, VentDuctHazard,
    VentDuctMetrics,
};

pub use alarm_beacon_studio::{
    audit_alarm_beacon_safety, evaluate_alarm_beacon_metrics, generate_alarm_beacon_brush,
    AlarmBeaconArchetype, AlarmBeaconConfig, AlarmBeaconHazard, AlarmBeaconMetrics,
    LevelAlarmContextActorRecord,
};

pub use supply_locker_studio::{
    audit_supply_locker_safety, evaluate_supply_locker_metrics, generate_supply_locker_brush,
    LevelLockerContextActorRecord, SupplyLockerArchetype, SupplyLockerConfig, SupplyLockerHazard,
    SupplyLockerMetrics,
};

pub use fluid_basin_studio::{
    audit_fluid_basin_safety, evaluate_fluid_basin_metrics, generate_fluid_basin_brush,
    FluidBasinArchetype, FluidBasinConfig, FluidBasinHazard, FluidBasinMetrics,
    LevelBasinContextActorRecord,
};

pub use teleporter_arch_studio::{
    audit_teleporter_arch_safety, evaluate_teleporter_arch_metrics, generate_teleporter_arch_brush,
    LevelTeleportContextActorRecord, TeleporterArchArchetype, TeleporterArchConfig,
    TeleporterArchHazard, TeleporterArchMetrics,
};

pub use secret_door_studio::{
    audit_secret_door_safety, evaluate_secret_door_metrics, generate_secret_door_brush,
    LevelSecretContextActorRecord, SecretConcealmentType, SecretDoorArchetype, SecretDoorConfig,
    SecretDoorMetrics, SecretDoorSafetyHazard,
};

pub use sniper_vantage_studio::{
    audit_sniper_vantage_safety, evaluate_sniper_vantage_metrics, generate_sniper_parapet_brush,
    LevelSniperContextActorRecord, SniperVantageArchetype, SniperVantageConfig, SniperVantageMetrics,
    SniperVantageSafetyAlert,
};

pub use special_lit_studio::{
    audit_special_lit_safety, evaluate_special_lit_metrics, generate_special_lit_fixture_brush,
    LevelLightContextSurfaceRecord, SpecialLitArchetype, SpecialLitConfig, SpecialLitHazard,
    SpecialLitMetrics,
};

pub use hazard_barrel_studio::{
    audit_hazard_barrel_safety, evaluate_hazard_barrel_metrics, generate_hazard_barrel_brush,
    BarrelHazardType, HazardBarrelConfig, HazardBarrelMetrics, HazardBarrelSafetyAlert,
    LevelBarrelContextActorRecord,
};

pub use zone_velocity_studio::{
    audit_zone_velocity_safety, evaluate_zone_velocity_metrics, generate_zone_portal_sheet_brush,
    LevelZoneContextActorRecord, ZoneMediumType, ZoneVelocityArchetype, ZoneVelocityConfig,
    ZoneVelocityHazard, ZoneVelocityMetrics,
};

pub use elevator_studio::{
    audit_elevator_safety, evaluate_elevator_metrics, generate_elevator_platform_brush,
    ElevatorArchetype, ElevatorConfig, ElevatorMetrics, ElevatorSafetyHazard,
    EncroachBehavior, LevelElevatorContextActorRecord,
};

pub use trigger_network_studio::{
    audit_trigger_network_safety, evaluate_trigger_network_metrics,
    generate_pressure_plate_brush, LevelTriggerContextActorRecord,
    TriggerActivationType, TriggerNetworkArchetype, TriggerNetworkConfig,
    TriggerNetworkHazard, TriggerNetworkMetrics,
};

pub use bio_trap_studio::{
    audit_bio_trap_safety, evaluate_bio_trap_metrics, generate_bio_drain_grate_brush,
    BioDispersionMode, BioTrapArchetype, BioTrapConfig, BioTrapHazard, BioTrapMetrics,
    LevelBioContextActorRecord,
};

pub use team_cannon_studio::{
    audit_team_cannon_safety, evaluate_team_cannon_metrics, generate_team_cannon_bunker_brush,
    CannonWeaponType, LevelCombatActorRecord, TeamCannonArchetype, TeamCannonConfig,
    TeamCannonHazard, TeamCannonMetrics,
};

pub use ctf_studio::{
    audit_ctf_base_safety, evaluate_ctf_base_metrics, generate_ctf_flag_base_brush,
    CTFTeam, CtfBaseArchetype, CtfBaseConfig, CtfBaseHazard, CtfBaseMetrics,
    LevelFlagBaseRecord,
};

pub use radar_studio::{
    audit_radar_safety, evaluate_radar_metrics, generate_radar_mast_and_dish_brush,
    LevelRadarNodeRecord, RadarHazard, RadarMetrics, RadarScanMode,
    RadarStationConfig, RadarStudioArchetype,
};

pub use assault_studio::{
    audit_assault_safety, evaluate_assault_metrics, generate_blast_door_brush,
    AssaultArchetype, AssaultChainConfig, AssaultHazard, AssaultMetrics,
    AssaultObjectiveType, LevelAssaultObjectiveRecord,
};

pub use domination_studio::{
    audit_domination_safety, evaluate_domination_metrics, generate_domination_pedestal_brush,
    DominationArchetype, DominationConfig, DominationHazard, DominationMetrics,
    DominationPointId, LevelControlPointRecord,
};

pub use spawn_distribution::{
    audit_spawn_safety, evaluate_spawn_metrics, generate_spawn_pad_brush,
    LevelPlayerStartRecord, SpawnDistributionArchetype, SpawnDistributionConfig,
    SpawnDistributionMetrics, SpawnHazard, TournamentTeamId,
};

pub use powerup_control::{
    audit_powerup_safety, evaluate_powerup_metrics, generate_powerup_altar_brush, NearbyLevelActor,
    PowerupArchetype, PowerupControlConfig, PowerupControlMetrics, PowerupHazard,
    TournamentPowerupType,
};

pub use weapon_circuit::{
    audit_weapon_circuit_safety, evaluate_weapon_circuit_metrics, generate_weapon_altar_brush,
    LevelWeaponMarker, TournamentWeaponType, WeaponCircuitArchetype, WeaponCircuitConfig,
    WeaponCircuitHazard, WeaponCircuitLink, WeaponCircuitMetrics,
};

pub use bot_jump_routes::{
    audit_jump_route_safety, evaluate_jump_metrics, generate_jump_plate_brush, solve_jump_trajectory,
    BotJumpArchetype, BotJumpHazard, BotJumpMetrics, BotJumpRoutesConfig, BotJumpType,
    JumpTrajectorySolution,
};

pub use armor_stacking::{
    audit_armor_stacking_safety, calculate_effective_health, evaluate_armor_stacking_metrics,
    generate_armor_pedestal_brush, simulate_damage_absorption, ArmorArchetype, ArmorHazard,
    ArmorStackingConfig, ArmorStackingHazard, ArmorStackingMetrics, ArmorType,
    DamageAbsorptionResult, HealthType, PickupMarker,
};

pub use arena_rules::{
    audit_arena_rules_safety, evaluate_arena_rules_metrics, ArenaHazard, ArenaRulesArchetype,
    ArenaRulesConfig, ArenaRulesHazard, ArenaRulesMetrics, GameTypeProfile, TournamentMutator,
};

pub use weather_studio::{
    audit_weather_safety, calculate_weather_metrics, generate_lightning_bolt,
    generate_weather_volume_brush, LightningBranch, PrecipitationType, WeatherArchetype,
    WeatherConfig, WeatherHazard, WeatherMetrics,
};

pub use hologram::{
    audit_hologram_safety, evaluate_hologram_kinematics, generate_hologram_brushes,
    HoloDisplayShape, HologramArchetype, HologramBrushes, HologramConfig, HologramHazard,
    HologramMetrics,
};

pub use pipe_conduit::{
    audit_pipe_conduit_safety, calculate_pipe_metrics, generate_pipe_conduit_brushes,
    PipeArchetype, PipeConduitBrushes, PipeConduitConfig, PipeConduitHazard, PipeConduitMetrics,
};

pub use hazard_chamber::{
    audit_hazard_chamber_safety, calculate_chamber_metrics, generate_chamber_bot_waypoints,
    generate_hazard_chamber_brushes, ChamberDoorStyle, HazardChamberArchetype,
    HazardChamberBotWaypoints, HazardChamberBrushes, HazardChamberConfig, HazardChamberHazard,
    HazardChamberMetrics, HazardDamageType,
};

pub use soundscape::{
    audit_soundscape_safety, calculate_soundscape_metrics, generate_acoustic_volume_brush,
    sample_sound_propagation, AcousticProbeSample, AcousticReverbPreset, AcousticZoneConfig,
    AmbientEmitterConfig, AudioFalloffCurve, SoundscapeArchetype, SoundscapeConfig,
    SoundscapeHazard, SoundscapeMetrics,
};

pub use auto_turret::{
    audit_turret_safety, calculate_turret_tracking_metrics, generate_turret_bot_waypoints,
    generate_turret_mount_brushes, TurretArchetype, TurretCombatConfig, TurretConfig,
    TurretEvasionWaypoint, TurretFrustumMetrics, TurretHazard, TurretMountConfig, TurretMountType,
    TurretTeamOwnership, TurretTrackingConfig, TurretWeaponType,
};

pub use conveyor_belt::{
    audit_conveyor_safety, calculate_conveyor_sync_metrics, generate_conveyor_bot_waypoints,
    generate_conveyor_brushes, ConveyorArchetype, ConveyorBotNavConfig, ConveyorBotWaypointSpec,
    ConveyorConfig, ConveyorDeckConfig, ConveyorDirection, ConveyorGuardrailConfig, ConveyorHazard,
    ConveyorKinematicsConfig, ConveyorRollerConfig, ConveyorSyncMetrics,
};

pub use ventilator_fan::{
    audit_ventilator_safety, calculate_aerodynamic_lift, generate_ventilator_brush,
    generate_wind_streamlines, AerodynamicLiftProfile, AirCurrentConfig, AirFlowDirection,
    FanGeometryConfig, FanLethalityConfig, VentilatorArchetype, VentilatorConfig,
    VentilatorHazard, WindStreamline,
};

pub use laser_grid::{
    audit_laser_grid_safety, generate_beam_trigger_specs, generate_laser_beams,
    generate_laser_brush_geometry, BeamTriggerSpec, LaserArchetype, LaserBeamData,
    LaserEmitterConfig, LaserGridConfig, LaserHazard, LaserLayoutMode, LaserSweepKinematics,
    LaserTriggerConfig,
};

pub use surveillance::{
    audit_surveillance_safety, calculate_camera_frustum, generate_monitor_brush,
    sample_patrol_orientation, CameraFrustum, CameraMountConfig, CameraPatrolConfig,
    MonitorDisplayConfig, SurveillanceArchetype, SurveillanceConfig, SurveillanceHazard,
    VideoFilterMode,
};

pub use track_mover::{
    audit_track_safety, bake_mover_keyframes, generate_station_bot_waypoints,
    generate_track_brushes, sample_track_spline, MoverKeyframeRecord, StationWaypoint,
    TrackArchetype, TrackConfig, TrackHazard, TrackPoint, TrackSample,
};

pub use light_shaft::{
    audit_light_shaft_safety, calculate_optical_transmission, generate_dust_motes,
    generate_light_shaft_brush, DustMote, LightShaftArchetype, LightShaftConfig,
    LightShaftCrossSection, LightShaftHazard,
};

pub use mesh_scatter::{
    audit_scatter_hazards, evaluate_scatter_metrics, generate_scatter_instances,
    triangulate_polygon, ScatterAlignmentMode, ScatterArchetype, ScatterConfig, ScatterHazard,
    ScatterInstance, ScatterMetrics,
};

pub use stairway::{
    audit_stair_safety, calculate_stair_metrics, generate_bot_path_nodes,
    generate_handrail_brush, generate_smooth_collision_ramp_brush, generate_stair_brush,
    CollisionHullSolidity, StairArchetype, StairBotPathingMode, StairBotWaypoint, StairConfig,
    StairHazard, StairHazardSeverity, StairMetrics,
};

pub use shape_loft::{
    audit_loft_hazards, generate_loft_brush, generate_rmf_frames, LoftConfig, LoftFrame,
    LoftHazard, LoftPathKeyframe, LoftProfileArchetype,
};

pub use bsp_quality::{
    analyze_brush_cut_offenders, analyze_bsp_tree_quality, detect_coplanar_overlaps,
    detect_sliver_polygons, purge_sliver_triangles, BspQualityMetrics, BrushCutOffender,
    CoplanarOverlap, SliverPolygon,
};
pub use kicker_physics::{
    audit_kicker_hazards, calculate_kicker_trajectory, solve_kick_velocity_for_target,
    KickerArchetype, KickerConfig, KickerHazard, KickerHazardSeverity, TrajectoryAnalysis,
    TrajectorySample,
};
pub use destructible::{
    audit_destructible_hazards, generate_fracture_shards, DebrisParticle, DestructibleArchetype,
    DestructibleConfig, DestructibleHazard, DestructibleHazardSeverity, DestructibleSimulation,
    FracturePatternType, ShardPolygon, UT99_GRAVITY,
};
pub use team_objectives::{
    audit_team_objectives, check_defense_los, compute_flag_routes,
    evaluate_domination_triangle, evaluate_route_asymmetry, generate_coordinated_defense_points,
    DefenseAuditIssue, DefenseAuditSeverity, DefensePointData, DominationTriangleAnalysis,
    FlagRoute, GameObjectiveType, ObjectiveNode, RouteAsymmetryAnalysis, TeamColor,
    TeamObjectivePreset,
};
pub use mover_studio::{
    audit_mover_hazards, generate_door_proximity_trigger, simulate_mover_encroachment,
    GeneratedDoorTrigger, MoverEncroachSimResult, MoverEncroachType, MoverHazardDiagnostic,
    MoverHazardSeverity, MoverSequenceConfig, MoverTemplateArchetype, MoverTriggerType,
};
pub use water_surface::{
    generate_watertight_water_sheet_brush, WaterHeightfieldSimulation, WaterPreset,
    WaterRippleSource, WaterSurfaceConfig,
};
pub use lift_pathing::{
    audit_lift_systems, generate_synchronized_lift_setup, simulate_elevator_bot_ride,
    ElevatorAuditSummary, ElevatorMoverData, GeneratedLiftCenter, GeneratedLiftExit,
    GeneratedLiftSetup, GeneratedTrigger, LiftBotSimulationState, LiftCenterData, LiftDiagnostic,
    LiftDiagnosticCategory, LiftDiagnosticSeverity, LiftExitData, LiftKeyframeFloor,
    LiftSimulationStep, LiftSystemProfile, LiftTriggerData,
};

pub use teleporter_network::{
    TeleportDiagnostic, TeleportDiagnosticCategory, TeleportDiagnosticSeverity, TeleportLink,
    TeleportNetworkGraph, TeleporterNode,
};

pub use translocator_physics::{
    simulate_translocator_flight, TranslocatorFlightSimulation, TranslocatorPreset,
    TranslocatorTossConfig, TranslocatorTrajectoryPoint,
};

pub use fluid_physics::{
    evaluate_player_survival_time, simulate_swimming_trajectory, FluidZonePreset, ZoneFluidPhysics,
};

pub use weapon_ballistics::{
    audit_combat_sightline, simulate_weapon_trajectory, BallisticProjectile,
    SightlineAuditResult, TrajectoryPoint, WeaponCombatPreset,
};

pub use music_sequencer::{
    audit_level_music, simulate_crossfade, MusicAuditIssue, MusicAuditReport, MusicEventData,
    MusicIssueSeverity, MusicSectionData, MusicTrackPreset, MusicTransition, TrackerModuleInfo,
};

pub use corona_flare::{
    calculate_corona_occlusion, project_lens_flare, CoronaConfig, CoronaPreset, FlareShape,
    LensFlareElement, LensFlareProjection,
};

pub use cinematic_camera::{
    CinematicCameraTrack, CinematicPreset, SplineCameraKeyframe,
};

pub use mesh_lod_collision::{
    build_mesh_lod_chain, decimate_mesh_polygons, generate_collision_hull, CollisionHullType,
    MeshLodChain, MeshLodLevel, PolygonBudgetRating,
};

pub use architectural_prefab::{
    build_prefab_brush, build_prefab_polygons, ArchStyle, ArchitecturalPrefabConfig,
    ArchitecturalPrefabPreset, ArchitecturalPrefabType, ClassicPillarConfig, IndustrialConduitConfig,
    LinearStairsConfig, PillarStyle, SpiralStairsConfig, VaultedArchConfig,
};

pub use terrain::{
    TerrainGrid, TerrainPreset, TerrainSculptMode, TerrainSlopeCategory,
};

pub use bot_combat::{
    analyze_combat_sightlines, CombatCoverageReport, CombatCoverRating, CombatSightline,
    CombatVantagePoint,
};
pub use ctf_symmetry::{
    generate_symmetrical_ctf_base, mirror_or_rotate_brush, remap_ctf_name,
    transform_point_symmetry, transform_vector_symmetry, CtfSymmetryConfig, CtfSymmetryMode,
    CtfSymmetryResult,
};
pub use dispatcher::{
    build_dispatcher_sequence, detect_cascading_dispatcher_chains, DispatcherChainLink,
    DispatcherPresetTemplate, DispatcherSequence, DispatcherStage, RoundRobinSequence,
};
pub use gltf_export::export_map_to_gltf_json;
pub use mover_anim::{MoverGlideType, MoverKeyframe, MoverTrajectory};
pub use particles::{Particle, ParticleEmitter, ParticleEmitterType, ParticlePresetTemplate};
pub use procedural_texture::{ProceduralColorPalette, ProceduralTextureSimulator, ProceduralTextureType};
pub use shadow_gradient::{
    evaluate_brush_shadow_gradient, evaluate_shadow_penumbra_profile, generate_light_source_samples,
    LightSourceShape, PenumbraGradientProfile,
};
pub use skeletal_mesh::{
    AnimKeyframe, AnimSequence, BoneTransform, CanonicalAnimType, SkeletalBone, SkeletalMesh,
    SkeletalSocket,
};
pub use zone_pvs::{
    build_zone_pvs_graph, generate_portal_sheet_brush, AntiPortalVolume, ZoneOcclusionAudit,
    ZoneOcclusionPreset, ZonePortalEdge, ZonePvsMatrix,
};
pub use zone_seal::{
    test_zone_watertightness, test_zone_watertightness_with_bounds, ZoneLeak, ZoneLeakConfig,
    ZoneSealReport,
};

pub use acoustics::{
    calculate_sound_attenuation, get_eax_preset, parse_eax_preset,
    simulate_reverb_impulse_samples, EaxEnvironment, ZoneAcoustics,
};
pub use alignment::{
    align_brush_continuous_strip, align_brush_cylindrical, align_polygon,
    align_polygons_continuous_strip, pan_texture, rotate_texture, scale_texture,
    AlignmentMode,
};
pub use arch::{build_arch, ArchConfig, ArchType};
pub use gameplay_balance::{
    analyze_item_balance, BalanceIssue, BalanceSeverity, ItemBalanceReport, ItemTier,
    PickupItem, PickupPairTiming, PLAYER_RUN_SPEED_UU_PER_SEC,
};
pub use bsp::{
    build_bsp_from_brushes, csg_boolean_with_world, default_csg_worker_threads,
    evaluate_csg_brushes, evaluate_csg_brushes_indexed, evaluate_csg_brushes_indexed_cancellable,
    evaluate_csg_brushes_indexed_with_options, BspBuildReport, BspBuilderConfig, BspNode,
    BspOptimizationLevel,
};
pub use brush::{
    build_aligned_octagon, build_cone, build_cube, build_curved_staircase, build_cylinder,
    build_ramp, build_ramp_oriented, build_sheet, build_sheet_advanced, build_sphere,
    build_spiral_staircase, build_spiral_staircase_ex, build_staircase, build_stepped_stairs_oriented, build_terrain,
    build_tetrahedron, build_tetrahedron_extrapolated, build_torus, build_volumetric, Brush,
    RampDirection, SheetAxis, SheetPreset,
};
pub use clipping::{clip_brush_by_plane, split_brush_by_plane};
pub use diagnostics::{BuilderBrushDiagnostics, IssueSeverity, LevelDoctor, MapIssue};
pub use lighting::{
    bake_lighting_for_brushes, bake_lighting_selective, evaluate_dynamic_light,
    evaluate_light_style, sample_light_waveform, ut99_light_to_rgb, DynamicLightPreset,
    DynamicLightState, LightEffect, LightType, LightingBakeQuality, LightingConfig,
    LightingStats, SceneLight,
};
pub use math::*;
pub use math_eval::{evaluate_expression, evaluate_or_default};
pub use navigation::{
    auto_build_navigation_reachspecs, auto_build_navigation_reachspecs_with_brushes,
    auto_build_navigation_reachspecs_with_index, is_nav_link_blocked_by_brushes,
    point_inside_brush, AutoPathReport, BotSimulator, JumpPadTrajectoryArc, LiftSyncReport,
    NavBrushIndex, NavCoverageRating, NavCoverageReport, NavGraph, NavNode, NavReachSpec,
    NavSamplePoint, TacticalCoverEntry, TacticalCoverRating, simulate_jumppad_trajectory,
    R_DOOR, R_FLY, R_JUMP, R_SPECIAL, R_SWIM, R_WALK,
};
pub use obj::{export_brushes_to_obj, ObjFace, ObjFaceVertex, ObjModel};
pub use poly::{
    LightmapData, Polygon, PolygonClassification, SplitResult, PF_ANTI_PORTAL, PF_AUTO_UPAN,
    PF_AUTO_VPAN, PF_BIG_WAVY, PF_BRIGHT_CORNERS, PF_DIRTY_SHADOWS, PF_FAKE_BACKDROP,
    PF_FORCE_VIEW_ZONE, PF_HIGH_SHADOW_DETAIL, PF_INVISIBLE, PF_LOW_SHADOW_DETAIL, PF_MASKED,
    PF_MIRRORED, PF_MODULATED, PF_NO_BOUNDS_REJECT, PF_NO_SMOOTH, PF_NOT_SOLID, PF_PORTAL,
    PF_SEMISOLID, PF_LEGACY_SEMISOLID, PF_SMALL_WAVY, PF_SPECIAL_LIT, PF_SPECIAL_POLY, PF_TRANSLUCENT, PF_TWO_SIDED,
    PF_UNLIT,
};
pub use projector::{ProjectedDecalPoly, Projector, ProjectorBlendMode};
pub use shape2d::{eval_cubic_bezier, Segment2DType, Shape2D};
pub use bsp::{repair_brushes_geometry, BspDoctorReport};
pub use zone_seal::{compute_bsp_zone_flood_fill, ZoneFloodFillResult, ZoneLeakPath};



/// Core CSG Operation types as defined in Unreal Engine 1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsgOperation {
    Active,
    Add,
    Subtract,
    Intersect,
    Deintersect,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_cube_builder() {
        let cube = build_cube("Box0", CsgOperation::Subtract, Vec3::new(256.0, 256.0, 128.0));
        assert_eq!(cube.name, "Box0");
        assert_eq!(cube.polys.len(), 6);

        // Every quad face should have 4 vertices
        for p in &cube.polys {
            assert_eq!(p.vertices.len(), 4);
        }
    }

    #[test]
    fn test_cylinder_builder() {
        let cyl = build_cylinder("Cyl0", CsgOperation::Add, 200.0, 50.0, 8);
        assert_eq!(cyl.name, "Cyl0");
        // 2 caps + 8 side quads = 10 polygons
        assert_eq!(cyl.polys.len(), 10);
    }

    #[test]
    fn test_cone_builder() {
        let cone = build_cone("Cone0", CsgOperation::Active, 256.0, 128.0, 8);
        assert_eq!(cone.name, "Cone0");
        // 1 cap + 8 side triangles = 9 polygons
        assert_eq!(cone.polys.len(), 9);
    }

    #[test]
    fn test_sheet_builder() {
        let sheet = build_sheet("Sheet0", CsgOperation::Active, 256.0, 256.0);
        assert_eq!(sheet.name, "Sheet0");
        assert_eq!(sheet.polys.len(), 2);
    }

    #[test]
    fn test_staircase_builder() {
        let stairs = build_staircase("Stairs0", CsgOperation::Add, 4, 16.0, 128.0, 32.0);
        assert_eq!(stairs.name, "Stairs0");
        assert!(!stairs.polys.is_empty());
    }

    #[test]
    fn test_sphere_builder() {
        let sphere = build_sphere("Sphere0", CsgOperation::Active, 128.0, 6, 8);
        assert_eq!(sphere.name, "Sphere0");
        assert!(!sphere.polys.is_empty());
    }

    #[test]
    fn test_bsp_node_construction() {
        let cube = build_cube("Box0", CsgOperation::Subtract, Vec3::new(100.0, 100.0, 100.0));
        let bsp = BspNode::new(cube.polys);
        assert!(bsp.is_some());
        let tree = bsp.unwrap();
        let collected = tree.collect_polygons();
        assert_eq!(collected.len(), 6);
    }

    #[test]
    fn test_curved_and_spiral_stairs() {
        let curved = build_curved_staircase("Curved0", CsgOperation::Active, 4, 16.0, 128.0, 128.0, 90.0, true);
        assert_eq!(curved.name, "Curved0");
        assert!(!curved.polys.is_empty());

        let spiral = build_spiral_staircase("Spiral0", CsgOperation::Active, 16, 16.0, 64.0, 128.0, 720.0, true);
        assert_eq!(spiral.name, "Spiral0");
        assert!(!spiral.polys.is_empty());
        // Verify upper steps (>360 deg) do not collapse to z_base == 0.0
        let max_min_z = spiral
            .polys
            .iter()
            .map(|p| p.vertices.iter().map(|v| v.z).fold(f32::MAX, f32::min))
            .fold(f32::MIN, f32::max);
        assert!(max_min_z > 100.0);

        let spiral_sloped = build_spiral_staircase_ex(
            "SpiralSloped",
            CsgOperation::Active,
            8,
            16.0,
            24.0,
            64.0,
            128.0,
            360.0,
            true,
            true,
            true,
        );
        assert!(!spiral_sloped.polys.is_empty());
    }

    #[test]
    fn test_tetrahedron_builder() {
        let tet = build_tetrahedron("Tet0", CsgOperation::Active, 128.0);
        assert_eq!(tet.name, "Tet0");
        assert_eq!(tet.polys.len(), 4);

        let tet2 = build_tetrahedron_extrapolated("Tet2", CsgOperation::Active, 128.0, 2);
        assert_eq!(tet2.polys.len(), 16);
        let tet3 = build_tetrahedron_extrapolated("Tet3", CsgOperation::Active, 128.0, 3);
        assert_eq!(tet3.polys.len(), 64);
    }

    #[test]
    fn test_torus_builder() {
        let torus = build_torus("Torus0", CsgOperation::Active, 256.0, 64.0, 8, 6);
        assert_eq!(torus.name, "Torus0");
        assert_eq!(torus.polys.len(), 8 * 6);
    }

    #[test]
    fn test_volumetric_and_terrain_builders() {
        let vol = build_volumetric("Vol0", CsgOperation::Active, 256.0, 512.0, 2);
        assert_eq!(vol.name, "Vol0");
        assert_eq!(vol.polys.len(), 4); // 2 sheets * 2 faces

        let terr = build_terrain("Terr0", CsgOperation::Active, 4, 4, 64.0, 32.0, 42);
        assert_eq!(terr.name, "Terr0");
        assert_eq!(terr.polys.len(), 4 * 4 * 2); // 16 quads * 2 triangles
    }
}


