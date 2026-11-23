use glam::Vec3;
use ut99_bsp::{Brush, Polygon, Projector};
use ut99_format::unr::actor::ActorRecord;
use crate::editor_state::EditorMover;

#[derive(Debug, Clone)]
pub enum Action {
    ClearAiPaths {
        old_graph: ut99_bsp::NavGraph,
        old_links: Vec<(Vec3, Vec3, [f32; 4])>,
        old_reach_specs: Option<Vec<ut99_format::unr::level::ReachSpecRecord>>,
        old_report: Option<ut99_bsp::AutoPathReport>,
    },
    MoveBrush {
        brush_index: usize,
        old_location: Vec3,
        new_location: Vec3,
    },
    RotateBrush {
        brush_index: usize,
        old_rotation: Vec3,
        new_rotation: Vec3,
    },
    ScaleBrush {
        brush_index: usize,
        old_scale: Vec3,
        new_scale: Vec3,
    },
    ModifyBrushGeometry {
        brush_index: usize,
        old_polys: Vec<Polygon>,
        new_polys: Vec<Polygon>,
        description: String,
    },
    ModifyBrushPivot {
        brush_index: usize,
        old_location: Vec3,
        new_location: Vec3,
        old_polys: Vec<Polygon>,
        new_polys: Vec<Polygon>,
        description: String,
    },
    MoveActor {
        actor_index: usize,
        old_location: Vec3,
        new_location: Vec3,
    },
    RotateActor {
        actor_index: usize,
        old_rotation: (i32, i32, i32),
        new_rotation: (i32, i32, i32),
    },
    AddActor {
        actor: ActorRecord,
    },
    DeleteActor {
        index: usize,
        actor: ActorRecord,
    },
    AddBrush {
        brush: Brush,
    },
    DeleteBrush {
        index: usize,
        brush: Brush,
    },
    MoveMover {
        mover_index: usize,
        old_location: Vec3,
        new_location: Vec3,
    },
    AddMover {
        mover: EditorMover,
    },
    DeleteMover {
        index: usize,
        mover: EditorMover,
    },
    RotateMover {
        mover_index: usize,
        old_rotation: Vec3,
        new_rotation: Vec3,
    },
    MoveStaticMesh {
        mesh_index: usize,
        old_location: Vec3,
        new_location: Vec3,
    },
    TransformStaticMesh {
        mesh_index: usize,
        old_rotation: Vec3,
        new_rotation: Vec3,
        old_scale: Vec3,
        new_scale: Vec3,
    },
    AddStaticMesh {
        mesh: Brush,
    },
    DeleteStaticMesh {
        index: usize,
        mesh: Brush,
    },
    SetSurfaceTexture {
        brush_index: usize,
        poly_index: usize,
        old_texture: String,
        new_texture: String,
    },
    BatchSetSurfaceTextures {
        modifications: Vec<(usize, usize, String, String)>,
    },
    SetMoverSurfaceTexture {
        mover_index: usize,
        poly_index: usize,
        old_texture: String,
        new_texture: String,
    },
    SetStaticMeshSurfaceTexture {
        mesh_index: usize,
        poly_index: usize,
        old_texture: String,
        new_texture: String,
    },
    AddProjector {
        index: usize,
        projector: Projector,
    },
    DeleteProjector {
        index: usize,
        projector: Projector,
    },
    ModifyBuilderBrush {
        old_brush: Brush,
        new_brush: Brush,
        description: String,
    },
    ReorderBrushes {
        old_brushes: Vec<Brush>,
        new_brushes: Vec<Brush>,
        old_selection: Option<usize>,
        new_selection: Option<usize>,
        description: String,
    },
    TransformBrush {
        brush_index: usize,
        old_rotation: Vec3,
        new_rotation: Vec3,
        old_scale: Vec3,
        new_scale: Vec3,
        old_polys: Vec<Polygon>,
        new_polys: Vec<Polygon>,
        description: String,
    },
    ScaleEntireLevel {
        old_brushes: Vec<Brush>,
        new_brushes: Vec<Brush>,
        old_actors: Option<Vec<ActorRecord>>,
        new_actors: Option<Vec<ActorRecord>>,
        old_movers: Vec<EditorMover>,
        new_movers: Vec<EditorMover>,
        factor: Vec3,
    },
    PropagateActorProperties {
        class_name: String,
        old_actors: Vec<ActorRecord>,
        new_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateZoneProperties {
        actor_tag: String,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    UpdateSoundEmitter {
        actor_tag: String,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    UpdateWarpZonePair {
        source_tag: String,
        target_tag: String,
        old_source: ActorRecord,
        new_source: ActorRecord,
        old_target: Option<ActorRecord>,
        new_target: Option<ActorRecord>,
        description: String,
    },
    BakeLighting {
        old_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)>,
        new_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)>,
        description: String,
    },
    UpdateZoneFog {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        old_viewport_fog: (bool, f32, f32, f32, [f32; 4]),
        new_viewport_fog: (bool, f32, f32, f32, [f32; 4]),
        description: String,
    },
    UpdateDispatcherSequence {
        actor_name: String,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    UpdateSkeletalMesh {
        mesh_index: usize,
        old_mesh: ut99_bsp::SkeletalMesh,
        new_mesh: ut99_bsp::SkeletalMesh,
        description: String,
    },
    UpdateParticleEmitter {
        emitter_index: usize,
        old_emitter: ut99_bsp::ParticleEmitter,
        new_emitter: ut99_bsp::ParticleEmitter,
        description: String,
    },
    UpdateLightEffects {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    BatchUpdateLightEffects {
        modifications: Vec<(usize, ActorRecord, ActorRecord)>,
        description: String,
    },
    AddAntiPortal {
        antiportal: ut99_bsp::AntiPortalVolume,
    },
    UpdateAntiPortal {
        antiportal_index: usize,
        old_antiportal: ut99_bsp::AntiPortalVolume,
        new_antiportal: ut99_bsp::AntiPortalVolume,
        description: String,
    },
    UpdateMusicEvent {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    SetLevelMusic {
        old_song: String,
        new_song: String,
        old_section: u8,
        new_section: u8,
        description: String,
    },
    UpdateLightCorona {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    BatchUpdateLightCoronas {
        modifications: Vec<(usize, ActorRecord, ActorRecord)>,
        description: String,
    },
    UpdateZonePhysics {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    BatchUpdateZonePhysics {
        modifications: Vec<(usize, ActorRecord, ActorRecord)>,
        description: String,
    },
    AddBallisticProbe {
        probe_index: usize,
        name: String,
        location: glam::Vec3,
        pitch: f32,
        yaw: f32,
        preset: ut99_bsp::WeaponCombatPreset,
    },
    DeleteBallisticProbe {
        probe_index: usize,
        name: String,
        location: glam::Vec3,
        pitch: f32,
        yaw: f32,
        preset: ut99_bsp::WeaponCombatPreset,
    },
    AddTeleporterPair {
        actor_index_a: usize,
        actor_index_b: usize,
        actor_a: ActorRecord,
        actor_b: ActorRecord,
        description: String,
    },
    UpdateTeleporterConfig {
        actor_index: usize,
        old_actor: ActorRecord,
        new_actor: ActorRecord,
        description: String,
    },
    AddTranslocatorProbe {
        probe_index: usize,
        name: String,
        location: glam::Vec3,
        pitch: f32,
        yaw: f32,
        preset: ut99_bsp::TranslocatorPreset,
    },
    DeleteTranslocatorProbe {
        probe_index: usize,
        name: String,
        location: glam::Vec3,
        pitch: f32,
        yaw: f32,
        preset: ut99_bsp::TranslocatorPreset,
    },
    AddElevatorPathingSet {
        mover_tag: String,
        added_actor_indices: Vec<usize>,
        added_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateElevatorMoverConfig {
        mover_index: usize,
        old_mover: EditorMover,
        new_mover: EditorMover,
        description: String,
    },
    AddWaterSheetBrush {
        brush_index: usize,
        brush: Brush,
        description: String,
    },
    UpdateWaterSurfaceConfig {
        old_config: ut99_bsp::WaterSurfaceConfig,
        new_config: ut99_bsp::WaterSurfaceConfig,
        old_preset: ut99_bsp::WaterPreset,
        new_preset: ut99_bsp::WaterPreset,
        description: String,
    },
    UpdateMoverSequenceConfig {
        mover_index: usize,
        old_config: ut99_bsp::MoverSequenceConfig,
        new_config: ut99_bsp::MoverSequenceConfig,
        old_template: ut99_bsp::MoverTemplateArchetype,
        new_template: ut99_bsp::MoverTemplateArchetype,
        description: String,
    },
    AddDoorTriggerActor {
        actor_index: usize,
        actor: ActorRecord,
        description: String,
    },
    AddDefensePointSet {
        objective_name: String,
        added_actor_indices: Vec<usize>,
        added_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateTeamObjectiveSetup {
        old_mode: ut99_bsp::GameObjectiveType,
        new_mode: ut99_bsp::GameObjectiveType,
        old_preset: Option<ut99_bsp::TeamObjectivePreset>,
        new_preset: Option<ut99_bsp::TeamObjectivePreset>,
        description: String,
    },
    AddDestructibleBarrier {
        brush_index: usize,
        brush: Brush,
        added_actor_indices: Vec<usize>,
        added_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateDestructibleConfig {
        old_config: ut99_bsp::DestructibleConfig,
        new_config: ut99_bsp::DestructibleConfig,
        old_archetype: ut99_bsp::DestructibleArchetype,
        new_archetype: ut99_bsp::DestructibleArchetype,
        description: String,
    },
    AddJumpPadSetup {
        kicker_index: usize,
        jump_spot_index: Option<usize>,
        kicker_actor: ActorRecord,
        jump_spot_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateKickerConfig {
        old_config: ut99_bsp::KickerConfig,
        new_config: ut99_bsp::KickerConfig,
        old_archetype: ut99_bsp::KickerArchetype,
        new_archetype: ut99_bsp::KickerArchetype,
        description: String,
    },
    PurgeSliverPolygons {
        modifications: Vec<(usize, Vec<Polygon>, Vec<Polygon>)>,
        description: String,
    },
    BatchConvertSemiSolid {
        conversions: Vec<(usize, Vec<Polygon>, Vec<Polygon>)>,
        description: String,
    },
    UpdateLoftConfig {
        old_config: ut99_bsp::LoftConfig,
        new_config: ut99_bsp::LoftConfig,
        old_keyframes: Vec<ut99_bsp::LoftPathKeyframe>,
        new_keyframes: Vec<ut99_bsp::LoftPathKeyframe>,
        description: String,
    },
    CommitLoftedBrush {
        brush_index: Option<usize>,
        brush: ut99_bsp::Brush,
        is_builder_brush: bool,
        old_builder_brush: Option<ut99_bsp::Brush>,
        description: String,
    },
    UpdateStairConfig {
        old_config: ut99_bsp::StairConfig,
        new_config: ut99_bsp::StairConfig,
        description: String,
    },
    CommitStairSetup {
        stair_brush_index: Option<usize>,
        stair_brush: ut99_bsp::Brush,
        ramp_brush_index: Option<usize>,
        ramp_brush: Option<ut99_bsp::Brush>,
        is_builder_brush: bool,
        old_builder_brush: Option<ut99_bsp::Brush>,
        description: String,
    },
    UpdateScatterConfig {
        old_config: ut99_bsp::ScatterConfig,
        new_config: ut99_bsp::ScatterConfig,
        description: String,
    },
    CommitScatterBatch {
        added_actor_indices: Vec<usize>,
        actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateLightShaftConfig {
        old_config: ut99_bsp::LightShaftConfig,
        new_config: ut99_bsp::LightShaftConfig,
        description: String,
    },
    CommitLightShaftBrush {
        brush_index: Option<usize>,
        brush: ut99_bsp::Brush,
        is_builder_brush: bool,
        old_builder_brush: Option<ut99_bsp::Brush>,
        description: String,
    },
    UpdateTrackConfig {
        old_config: ut99_bsp::TrackConfig,
        new_config: ut99_bsp::TrackConfig,
        description: String,
    },
    CommitTrackSetup {
        rails_brush_index: Option<usize>,
        ties_brush_index: Option<usize>,
        rails_brush: ut99_bsp::Brush,
        ties_brush: ut99_bsp::Brush,
        mover_actor_index: Option<usize>,
        mover_actor: Option<ActorRecord>,
        waypoint_actor_indices: Vec<usize>,
        waypoint_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateSurveillanceConfig {
        old_config: ut99_bsp::SurveillanceConfig,
        new_config: ut99_bsp::SurveillanceConfig,
        description: String,
    },
    CommitSurveillanceSetup {
        monitor_brush_index: Option<usize>,
        monitor_brush: ut99_bsp::Brush,
        camera_actor_index: Option<usize>,
        camera_actor: Option<ActorRecord>,
        screen_actor_index: Option<usize>,
        screen_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateLaserConfig {
        old_config: ut99_bsp::LaserGridConfig,
        new_config: ut99_bsp::LaserGridConfig,
        description: String,
    },
    CommitLaserGrid {
        beam_brush_index: Option<usize>,
        diode_brush_index: Option<usize>,
        beam_brush: ut99_bsp::Brush,
        diode_brush: ut99_bsp::Brush,
        trigger_actor_indices: Vec<usize>,
        trigger_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateVentilatorConfig {
        old_config: ut99_bsp::VentilatorConfig,
        new_config: ut99_bsp::VentilatorConfig,
        description: String,
    },
    CommitVentilatorSetup {
        fan_brush_index: Option<usize>,
        housing_brush_index: Option<usize>,
        fan_brush: ut99_bsp::Brush,
        housing_brush: ut99_bsp::Brush,
        mover_actor_index: Option<usize>,
        mover_actor: Option<ActorRecord>,
        zone_actor_index: Option<usize>,
        zone_actor: Option<ActorRecord>,
        trigger_actor_index: Option<usize>,
        trigger_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateConveyorConfig {
        old_config: ut99_bsp::ConveyorConfig,
        new_config: ut99_bsp::ConveyorConfig,
        description: String,
    },
    CommitConveyorSetup {
        deck_brush_index: Option<usize>,
        rollers_brush_index: Option<usize>,
        frame_brush_index: Option<usize>,
        deck_brush: ut99_bsp::Brush,
        rollers_brush: ut99_bsp::Brush,
        frame_brush: ut99_bsp::Brush,
        zone_actor_index: Option<usize>,
        zone_actor: Option<ActorRecord>,
        waypoint_actor_indices: Vec<usize>,
        waypoint_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateTurretConfig {
        old_config: ut99_bsp::TurretConfig,
        new_config: ut99_bsp::TurretConfig,
        description: String,
    },
    CommitTurretSetup {
        mount_brush_index: Option<usize>,
        shield_brush_index: Option<usize>,
        mount_brush: ut99_bsp::Brush,
        shield_brush: ut99_bsp::Brush,
        turret_actor_index: Option<usize>,
        turret_actor: Option<ActorRecord>,
        waypoint_actor_indices: Vec<usize>,
        waypoint_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateSoundscapeConfig {
        old_config: ut99_bsp::SoundscapeConfig,
        new_config: ut99_bsp::SoundscapeConfig,
        description: String,
    },
    CommitSoundscapeSetup {
        zone_brush_index: Option<usize>,
        zone_brush: ut99_bsp::Brush,
        ambient_actor_index: Option<usize>,
        ambient_actor: Option<ActorRecord>,
        zone_actor_index: Option<usize>,
        zone_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateHazardChamberConfig {
        old_config: ut99_bsp::HazardChamberConfig,
        new_config: ut99_bsp::HazardChamberConfig,
        description: String,
    },
    CommitHazardChamberSetup {
        chamber_brush_index: Option<usize>,
        door_l_brush_index: Option<usize>,
        door_r_brush_index: Option<usize>,
        window_brush_index: Option<usize>,
        console_brush_index: Option<usize>,
        chamber_brush: ut99_bsp::Brush,
        door_l_brush: ut99_bsp::Brush,
        door_r_brush: ut99_bsp::Brush,
        window_brush: Option<ut99_bsp::Brush>,
        console_brush: ut99_bsp::Brush,
        zone_actor_index: Option<usize>,
        zone_actor: Option<ActorRecord>,
        trigger_actor_index: Option<usize>,
        trigger_actor: Option<ActorRecord>,
        waypoint_actor_indices: Vec<usize>,
        waypoint_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdatePipeConduitConfig {
        old_config: ut99_bsp::PipeConduitConfig,
        new_config: ut99_bsp::PipeConduitConfig,
        description: String,
    },
    CommitPipeConduitSetup {
        straight_brush_index: Option<usize>,
        straight_brush: ut99_bsp::Brush,
        elbow_brush_index: Option<usize>,
        elbow_brush: Option<ut99_bsp::Brush>,
        flange_brush_indices: Vec<usize>,
        flange_brushes: Vec<ut99_bsp::Brush>,
        collar_brush_index: Option<usize>,
        collar_brush: Option<ut99_bsp::Brush>,
        valve_body_brush_index: Option<usize>,
        valve_body_brush: Option<ut99_bsp::Brush>,
        valve_wheel_brush_index: Option<usize>,
        valve_wheel_brush: Option<ut99_bsp::Brush>,
        valve_mover_actor_index: Option<usize>,
        valve_mover_actor: Option<ActorRecord>,
        hazard_trigger_actor_index: Option<usize>,
        hazard_trigger_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateHologramConfig {
        old_config: ut99_bsp::HologramConfig,
        new_config: ut99_bsp::HologramConfig,
        description: String,
    },
    CommitHologramSetup {
        display_brush_index: Option<usize>,
        display_brush: ut99_bsp::Brush,
        pedestal_brush_index: Option<usize>,
        pedestal_brush: Option<ut99_bsp::Brush>,
        projection_cone_brush_index: Option<usize>,
        projection_cone_brush: Option<ut99_bsp::Brush>,
        rotator_mover_actor_index: Option<usize>,
        rotator_mover_actor: Option<ActorRecord>,
        projector_light_actor_index: Option<usize>,
        projector_light_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateWeatherConfig {
        old_config: ut99_bsp::WeatherConfig,
        new_config: ut99_bsp::WeatherConfig,
        description: String,
    },
    CommitWeatherSetup {
        volume_brush_index: Option<usize>,
        volume_brush: ut99_bsp::Brush,
        zone_emitter_actor_index: Option<usize>,
        zone_emitter_actor: Option<ActorRecord>,
        lightning_light_actor_index: Option<usize>,
        lightning_light_actor: Option<ActorRecord>,
        thunder_audio_actor_index: Option<usize>,
        thunder_audio_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateArenaRulesConfig {
        old_config: ut99_bsp::ArenaRulesConfig,
        new_config: ut99_bsp::ArenaRulesConfig,
        description: String,
    },
    CommitArenaRulesSetup {
        level_info_actor_index: Option<usize>,
        old_level_info: Option<ActorRecord>,
        new_level_info: Option<ActorRecord>,
        old_level_game_type: String,
        new_level_game_type: String,
        mutator_actor_indices: Vec<usize>,
        mutator_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateArmorStackingConfig {
        old_config: ut99_bsp::ArmorStackingConfig,
        new_config: ut99_bsp::ArmorStackingConfig,
        description: String,
    },
    CommitArmorStackingSetup {
        pedestal_brush_index: Option<usize>,
        pedestal_brush: ut99_bsp::Brush,
        pickup_actor_indices: Vec<usize>,
        pickup_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateBotJumpRoutesConfig {
        old_config: ut99_bsp::BotJumpRoutesConfig,
        new_config: ut99_bsp::BotJumpRoutesConfig,
        description: String,
    },
    CommitBotJumpRouteSetup {
        jump_plate_brush_index: Option<usize>,
        jump_plate_brush: ut99_bsp::Brush,
        actor_indices: Vec<usize>,
        actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateWeaponCircuitConfig {
        old_config: ut99_bsp::WeaponCircuitConfig,
        new_config: ut99_bsp::WeaponCircuitConfig,
        description: String,
    },
    CommitWeaponCircuitSetup {
        altar_brush_index: Option<usize>,
        altar_brush: ut99_bsp::Brush,
        actor_indices: Vec<usize>,
        actors: Vec<ActorRecord>,
        description: String,
    },
    UpdatePowerupControlConfig {
        old_config: ut99_bsp::PowerupControlConfig,
        new_config: ut99_bsp::PowerupControlConfig,
        description: String,
    },
    CommitPowerupSetup {
        altar_brush_index: Option<usize>,
        altar_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSpawnDistributionConfig {
        old_config: ut99_bsp::SpawnDistributionConfig,
        new_config: ut99_bsp::SpawnDistributionConfig,
        description: String,
    },
    CommitSpawnPointSetup {
        pad_brush_index: Option<usize>,
        pad_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateDominationConfig {
        old_config: ut99_bsp::DominationConfig,
        new_config: ut99_bsp::DominationConfig,
        description: String,
    },
    CommitDominationPointSetup {
        pedestal_brush_index: Option<usize>,
        pedestal_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateAssaultConfig {
        old_config: ut99_bsp::AssaultChainConfig,
        new_config: ut99_bsp::AssaultChainConfig,
        description: String,
    },
    CommitAssaultObjectiveSetup {
        door_brush_index: Option<usize>,
        door_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateRadarConfig {
        old_config: ut99_bsp::RadarStationConfig,
        new_config: ut99_bsp::RadarStationConfig,
        description: String,
    },
    CommitRadarStationSetup {
        mast_brush_index: Option<usize>,
        mast_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateCtfBaseConfig {
        old_config: ut99_bsp::CtfBaseConfig,
        new_config: ut99_bsp::CtfBaseConfig,
        description: String,
    },
    CommitCtfBaseSetup {
        pedestal_brush_index: Option<usize>,
        pedestal_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateTeamCannonConfig {
        old_config: ut99_bsp::TeamCannonConfig,
        new_config: ut99_bsp::TeamCannonConfig,
        description: String,
    },
    CommitTeamCannonSetup {
        mount_brush_index: Option<usize>,
        mount_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateBioTrapConfig {
        old_config: ut99_bsp::BioTrapConfig,
        new_config: ut99_bsp::BioTrapConfig,
        description: String,
    },
    CommitBioTrapSetup {
        grate_brush_index: Option<usize>,
        grate_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateTriggerNetworkConfig {
        old_config: ut99_bsp::TriggerNetworkConfig,
        new_config: ut99_bsp::TriggerNetworkConfig,
        description: String,
    },
    CommitTriggerNetworkSetup {
        plate_brush_index: Option<usize>,
        plate_brush: ut99_bsp::Brush,
        actor_index: Option<usize>,
        actor: Option<ActorRecord>,
        description: String,
    },
    UpdateElevatorConfig {
        old_config: ut99_bsp::ElevatorConfig,
        new_config: ut99_bsp::ElevatorConfig,
        description: String,
    },
    CommitElevatorSetup {
        platform_brush_index: Option<usize>,
        platform_brush: ut99_bsp::Brush,
        lift_center_actor_index: Option<usize>,
        lift_center_actor: Option<ActorRecord>,
        lift_exit_actor_indices: Vec<usize>,
        lift_exit_actors: Vec<ActorRecord>,
        description: String,
    },
    UpdateZoneVelocityConfig {
        old_config: ut99_bsp::ZoneVelocityConfig,
        new_config: ut99_bsp::ZoneVelocityConfig,
        description: String,
    },
    CommitZoneVelocitySetup {
        portal_brush_index: Option<usize>,
        portal_brush: ut99_bsp::Brush,
        zone_info_actor_index: Option<usize>,
        zone_info_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateHazardBarrelConfig {
        old_config: ut99_bsp::HazardBarrelConfig,
        new_config: ut99_bsp::HazardBarrelConfig,
        description: String,
    },
    CommitHazardBarrelSetup {
        barrel_brush_index: Option<usize>,
        barrel_brush: ut99_bsp::Brush,
        barrel_actor_index: Option<usize>,
        barrel_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSpecialLitConfig {
        old_config: ut99_bsp::SpecialLitConfig,
        new_config: ut99_bsp::SpecialLitConfig,
        description: String,
    },
    CommitSpecialLitSetup {
        fixture_brush_index: Option<usize>,
        fixture_brush: ut99_bsp::Brush,
        light_actor_index: Option<usize>,
        light_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSniperVantageConfig {
        old_config: ut99_bsp::SniperVantageConfig,
        new_config: ut99_bsp::SniperVantageConfig,
        description: String,
    },
    CommitSniperVantageSetup {
        parapet_brush_index: Option<usize>,
        parapet_brush: ut99_bsp::Brush,
        sniper_actor_index: Option<usize>,
        sniper_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSecretDoorConfig {
        old_config: ut99_bsp::SecretDoorConfig,
        new_config: ut99_bsp::SecretDoorConfig,
        description: String,
    },
    CommitSecretDoorSetup {
        door_brush_index: Option<usize>,
        door_brush: ut99_bsp::Brush,
        trigger_actor_index: Option<usize>,
        trigger_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateTeleporterArchConfig {
        old_config: ut99_bsp::TeleporterArchConfig,
        new_config: ut99_bsp::TeleporterArchConfig,
        description: String,
    },
    CommitTeleporterArchSetup {
        arch_brush_index: Option<usize>,
        arch_brush: ut99_bsp::Brush,
        teleporter_actor_index: Option<usize>,
        teleporter_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateFluidBasinConfig {
        old_config: ut99_bsp::FluidBasinConfig,
        new_config: ut99_bsp::FluidBasinConfig,
        description: String,
    },
    CommitFluidBasinSetup {
        basin_brush_index: Option<usize>,
        basin_brush: ut99_bsp::Brush,
        zone_actor_index: Option<usize>,
        zone_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateAlarmBeaconConfig {
        old_config: ut99_bsp::AlarmBeaconConfig,
        new_config: ut99_bsp::AlarmBeaconConfig,
        description: String,
    },
    CommitAlarmBeaconSetup {
        beacon_brush_index: Option<usize>,
        beacon_brush: ut99_bsp::Brush,
        light_actor_index: Option<usize>,
        light_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSupplyLockerConfig {
        old_config: ut99_bsp::SupplyLockerConfig,
        new_config: ut99_bsp::SupplyLockerConfig,
        description: String,
    },
    CommitSupplyLockerSetup {
        locker_brush_index: Option<usize>,
        locker_brush: ut99_bsp::Brush,
        weapon_actor_index: Option<usize>,
        weapon_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateSkylightConfig {
        old_config: ut99_bsp::SkylightConfig,
        new_config: ut99_bsp::SkylightConfig,
        description: String,
    },
    CommitSkylightSetup {
        skylight_brush_index: Option<usize>,
        skylight_brush: ut99_bsp::Brush,
        sunlight_actor_index: Option<usize>,
        sunlight_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateVentDuctConfig {
        old_config: ut99_bsp::VentDuctConfig,
        new_config: ut99_bsp::VentDuctConfig,
        description: String,
    },
    CommitVentDuctSetup {
        duct_brush_index: Option<usize>,
        duct_brush: ut99_bsp::Brush,
        grate_actor_index: Option<usize>,
        grate_actor: Option<ActorRecord>,
        description: String,
    },
    UpdatePistonConfig {
        old_config: ut99_bsp::PistonConfig,
        new_config: ut99_bsp::PistonConfig,
        description: String,
    },
    CommitPistonSetup {
        piston_brush_index: Option<usize>,
        piston_brush: ut99_bsp::Brush,
        mover_actor_index: Option<usize>,
        mover_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateJumpBootsConfig {
        old_config: ut99_bsp::JumpBootsConfig,
        new_config: ut99_bsp::JumpBootsConfig,
        description: String,
    },
    CommitJumpBootsSetup {
        cradle_brush_index: Option<usize>,
        cradle_brush: ut99_bsp::Brush,
        boots_actor_index: Option<usize>,
        boots_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateForceFieldConfig {
        old_config: ut99_bsp::ForceFieldConfig,
        new_config: ut99_bsp::ForceFieldConfig,
        description: String,
    },
    CommitForceFieldSetup {
        field_brush_index: Option<usize>,
        field_brush: ut99_bsp::Brush,
        pylon_brush_indices: Vec<usize>,
        pylon_brushes: Vec<ut99_bsp::Brush>,
        generator_actor_index: Option<usize>,
        generator_actor: Option<ActorRecord>,
        description: String,
    },
    UpdateReactorCoreConfig {
        old_config: ut99_bsp::ReactorCoreConfig,
        new_config: ut99_bsp::ReactorCoreConfig,
        description: String,
    },
    CommitReactorCoreSetup {
        core_brush_index: Option<usize>,
        core_brush: ut99_bsp::Brush,
        ring_brush_indices: Vec<usize>,
        ring_brushes: Vec<ut99_bsp::Brush>,
        reactor_actor_index: Option<usize>,
        reactor_actor: Option<ActorRecord>,
        description: String,
    },
    ReplaceEntireLevel {
        old_map_path: Option<std::path::PathBuf>,
        new_map_path: Option<std::path::PathBuf>,
        old_launch_args: String,
        new_launch_args: String,
        old_brushes: Vec<Brush>,
        new_brushes: Vec<Brush>,
        old_movers: Vec<EditorMover>,
        new_movers: Vec<EditorMover>,
        old_unr: Option<ut99_format::UnrLevel>,
        new_unr: Option<ut99_format::UnrLevel>,
        old_metadata: (String, String, String, String, String, String, u8, u8, u8),
        new_metadata: (String, String, String, String, String, String, u8, u8, u8),
        description: String,
    },
    Compound {
        actions: Vec<Action>,
        description: String,
    },
}

impl Action {
    pub fn description(&self) -> &str {
        match self {
            Action::ClearAiPaths { .. } => "Clear AI Paths",
            Action::MoveBrush { .. } => "Move Brush",
            Action::RotateBrush { .. } => "Rotate Brush",
            Action::ScaleBrush { .. } => "Scale Brush",
            Action::ModifyBrushGeometry { description, .. } => description.as_str(),
            Action::ModifyBrushPivot { description, .. } => description.as_str(),
            Action::MoveActor { .. } => "Move Actor",
            Action::RotateActor { .. } => "Rotate Actor",
            Action::AddActor { .. } => "Add Actor",
            Action::DeleteActor { .. } => "Delete Actor",
            Action::AddBrush { .. } => "Add Brush",
            Action::DeleteBrush { .. } => "Delete Brush",
            Action::MoveMover { .. } => "Move Mover",
            Action::AddMover { .. } => "Add Mover",
            Action::DeleteMover { .. } => "Delete Mover",
            Action::RotateMover { .. } => "Rotate Mover",
            Action::MoveStaticMesh { .. } => "Move Static Mesh",
            Action::TransformStaticMesh { .. } => "Transform Static Mesh",
            Action::AddStaticMesh { .. } => "Add Static Mesh",
            Action::DeleteStaticMesh { .. } => "Delete Static Mesh",
            Action::SetSurfaceTexture { .. } => "Set Surface Texture",
            Action::BatchSetSurfaceTextures { .. } => "Batch Set Textures",
            Action::SetMoverSurfaceTexture { .. } => "Set Mover Texture",
            Action::SetStaticMeshSurfaceTexture { .. } => "Set Mesh Texture",
            Action::AddProjector { .. } => "Add Projector",
            Action::DeleteProjector { .. } => "Delete Projector",
            Action::ModifyBuilderBrush { .. } => "Modify Builder Brush",
            Action::ReorderBrushes { .. } => "Reorder Brushes",
            Action::TransformBrush { .. } => "Transform Brush",
            Action::ScaleEntireLevel { .. } => "Scale Entire Level",
            Action::PropagateActorProperties { .. } => "Propagate Properties",
            Action::BakeLighting { .. } => "Bake Lighting",
            Action::ReplaceEntireLevel { description, .. } => description.as_str(),
            Action::Compound { description, .. } => description.as_str(),
            _ => "Action",
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct History {
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
    pub coalescing_active: bool,
}

impl History {
    pub fn next_undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|a| a.description())
    }

    pub fn next_redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|a| a.description())
    }

    pub fn push(&mut self, action: Action) {
        self.coalescing_active = false;
        if self.undo_stack.len() >= 100 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    /// Pushes an interactive/continuous property or transform edit (e.g., slider drag or DragValue),
    /// coalescing consecutive frames on the same target while `is_interacting` remains true.
    pub fn push_interactive(&mut self, action: Action, is_interacting: bool) {
        if self.coalescing_active {
            if let Some(last) = self.undo_stack.last_mut() {
                match (last, &action) {
                    (
                        Action::UpdateLightEffects {
                            actor_index: last_idx,
                            new_actor: last_new,
                            description: last_desc,
                            ..
                        },
                        Action::UpdateLightEffects {
                            actor_index,
                            new_actor,
                            description,
                            ..
                        },
                    ) if *last_idx == *actor_index => {
                        *last_new = new_actor.clone();
                        *last_desc = description.clone();
                        self.redo_stack.clear();
                        self.coalescing_active = is_interacting;
                        return;
                    }
                    (
                        Action::UpdateElevatorMoverConfig {
                            mover_index: last_idx,
                            new_mover: last_new,
                            description: last_desc,
                            ..
                        },
                        Action::UpdateElevatorMoverConfig {
                            mover_index,
                            new_mover,
                            description,
                            ..
                        },
                    ) if *last_idx == *mover_index => {
                        *last_new = new_mover.clone();
                        *last_desc = description.clone();
                        self.redo_stack.clear();
                        self.coalescing_active = is_interacting;
                        return;
                    }
                    (
                        Action::MoveStaticMesh {
                            mesh_index: last_idx,
                            new_location: last_new,
                            ..
                        },
                        Action::MoveStaticMesh {
                            mesh_index,
                            new_location,
                            ..
                        },
                    ) if *last_idx == *mesh_index => {
                        *last_new = *new_location;
                        self.redo_stack.clear();
                        self.coalescing_active = is_interacting;
                        return;
                    }
                    (
                        Action::ModifyBrushGeometry {
                            brush_index: last_idx,
                            new_polys: last_new,
                            description: last_desc,
                            ..
                        },
                        Action::ModifyBrushGeometry {
                            brush_index,
                            new_polys,
                            description,
                            ..
                        },
                    ) if *last_idx == *brush_index => {
                        *last_new = new_polys.clone();
                        *last_desc = description.clone();
                        self.redo_stack.clear();
                        self.coalescing_active = is_interacting;
                        return;
                    }
                    (
                        Action::ModifyBuilderBrush {
                            new_brush: last_new,
                            description: last_desc,
                            ..
                        },
                        Action::ModifyBuilderBrush {
                            new_brush,
                            description,
                            ..
                        },
                    ) => {
                        *last_new = new_brush.clone();
                        *last_desc = description.clone();
                        self.redo_stack.clear();
                        self.coalescing_active = is_interacting;
                        return;
                    }
                    _ => {}
                }
            }
        }
        if self.undo_stack.len() >= 100 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear();
        self.coalescing_active = is_interacting;
    }

    pub fn end_interaction(&mut self) {
        self.coalescing_active = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

