use ut99_bsp::{
    build_prefab_brush, build_prefab_polygons, ArchStyle, ArchitecturalPrefabConfig,
    ArchitecturalPrefabPreset, ArchitecturalPrefabType, ClassicPillarConfig, CsgOperation,
    LinearStairsConfig, PillarStyle, SpiralStairsConfig, VaultedArchConfig,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_linear_and_spiral_stairs_synthesis() {
    let mut linear_cfg = ArchitecturalPrefabConfig::default();
    linear_cfg.prefab_type = ArchitecturalPrefabType::LinearStairs;
    linear_cfg.linear_stairs = LinearStairsConfig {
        step_count: 8,
        step_width: 256.0,
        step_height: 16.0,
        step_depth: 32.0,
        solid_underside: true,
    };

    let linear_polys = build_prefab_polygons(&linear_cfg);
    assert_eq!(linear_polys.len(), 8 * 6, "8 convex boxes must generate 48 polygons");
    for p in &linear_polys {
        assert_eq!(p.vertices.len(), 4);
    }

    let mut spiral_cfg = ArchitecturalPrefabConfig::default();
    spiral_cfg.prefab_type = ArchitecturalPrefabType::SpiralStairs;
    spiral_cfg.spiral_stairs = SpiralStairsConfig {
        step_count: 12,
        inner_radius: 32.0,
        outer_radius: 180.0,
        step_height: 16.0,
        total_angle_degrees: 360.0,
        has_central_pillar: true,
    };

    let spiral_polys = build_prefab_polygons(&spiral_cfg);
    assert!(!spiral_polys.is_empty(), "Spiral staircase must generate geometry");
    assert!(spiral_polys.len() >= 12 * 6, "Must contain at least 12 step wedges");
}

#[test]
fn test_vaulted_arch_and_keystones_generation() {
    let mut roman_cfg = ArchitecturalPrefabConfig::default();
    roman_cfg.prefab_type = ArchitecturalPrefabType::VaultedArch;
    roman_cfg.vaulted_arch = VaultedArchConfig {
        arch_style: ArchStyle::RoundRoman,
        outer_radius: 256.0,
        inner_radius: 192.0,
        thickness_depth: 64.0,
        segments: 10,
    };

    let roman_polys = build_prefab_polygons(&roman_cfg);
    assert_eq!(roman_polys.len(), 10 * 6, "10 keystone wedges must yield 60 polygons");

    let mut gothic_cfg = ArchitecturalPrefabConfig::default();
    gothic_cfg.prefab_type = ArchitecturalPrefabType::VaultedArch;
    gothic_cfg.vaulted_arch = VaultedArchConfig {
        arch_style: ArchStyle::GothicPointed,
        outer_radius: 256.0,
        inner_radius: 192.0,
        thickness_depth: 64.0,
        segments: 12,
    };

    let gothic_polys = build_prefab_polygons(&gothic_cfg);
    assert_eq!(gothic_polys.len(), 12 * 6, "12 pointed gothic wedges must yield 72 polygons");
}

#[test]
fn test_classical_pillar_and_entasis_geometry() {
    let mut pillar_cfg = ArchitecturalPrefabConfig::default();
    pillar_cfg.prefab_type = ArchitecturalPrefabType::ClassicPillar;
    pillar_cfg.classic_pillar = ClassicPillarConfig {
        pillar_style: PillarStyle::CylindricalFluted,
        height: 384.0,
        radius: 48.0,
        base_height: 32.0,
        capital_height: 32.0,
        entasis_factor: 1.15,
        sides: 8,
    };

    let brush = build_prefab_brush("TestPillar", CsgOperation::Add, &pillar_cfg);
    assert_eq!(brush.name, "TestPillar");
    assert_eq!(brush.csg_oper, CsgOperation::Add);
    assert!(!brush.polys.is_empty(), "Pillar brush must have non-empty polygons");
}

#[test]
fn test_architectural_studio_presets_and_bake_workflow() {
    let mut state = EditorState::default();

    // 1. Apply TournamentArenaStairs preset
    state.apply_architectural_preset(ArchitecturalPrefabPreset::TournamentArenaStairs);
    assert_eq!(state.architectural_settings.config.prefab_type, ArchitecturalPrefabType::LinearStairs);
    assert_eq!(state.architectural_settings.config.linear_stairs.step_count, 8);

    // 2. Synthesize to Builder Brush
    state.synthesize_prefab_to_builder();
    assert_eq!(state.builder_brush.polys.len(), 48);

    // 3. Bake to World Brush
    let initial_count = state.world_brushes.len();
    let baked_idx = state.bake_prefab_to_world(CsgOperation::Add);
    assert_eq!(baked_idx, initial_count);
    assert_eq!(state.world_brushes.len(), initial_count + 1);

    // 4. Undo and Redo
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_count, "Undo must remove baked brush");

    state.redo();
    assert_eq!(state.world_brushes.len(), initial_count + 1, "Redo must restore baked brush");
}
