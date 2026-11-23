// Physical placement audit of wizard-generated maps, measured on the BSP built by the REAL UT99 editor
// engine (`ucc exec`): every actor must rest exactly on its supporting surface (no floating medkits, no
// fans buried in the floor), must not intersect walls/pillars, and every PlayerStart must have a floor.
//
// Requires FORGE99_TEST_UT_DIR to point to a dedicated isolated UT99 installation.

use std::path::PathBuf;

use ut99_bsp::class_profiles::{class_profile, is_navigation_point, Mount};
use ut99_bsp::{WizardArchetype, WizardGameMode, WizardLayoutStrategy, WizardRoomShape};
use ut99_editor::compiler::CompilerBridge;
use ut99_editor::editor_state::EditorState;
use ut99_format::UnrLevel;
mod common;

struct Scenario {
    label: &'static str,
    mode: WizardGameMode,
    layout: WizardLayoutStrategy,
    archetype: WizardArchetype,
    room_shape: WizardRoomShape,
    seed: u64,
}

#[test]
fn unsafe_mirrored_pickup_is_reported() {
    let mut cfg = ut99_bsp::MapWizardConfig::default();
    cfg.game_mode = WizardGameMode::CaptureTheFlag;
    cfg.archetype = WizardArchetype::CentralArenaMultiTier;
    cfg.layout_strategy = WizardLayoutStrategy::CanonicalArchetype;
    cfg.room_shape = WizardRoomShape::Rectangular;
    cfg.seed = 1;
    cfg.sync_mode_conventions();
    let map = ut99_bsp::generate_valid_ut99_map(&cfg);
    assert!(map.warnings.iter().any(|warning| warning.contains("MedBox_Blue_0")), "warnings: {:?}", map.warnings);
}

fn build_scenario(sc: &Scenario, ut_dir: &std::path::Path, out: &PathBuf) -> (UnrLevel, Vec<ut99_bsp::Brush>) {
    let mut state = EditorState::default();
    state.map_wizard_config.game_mode = sc.mode;
    state.map_wizard_config.layout_strategy = sc.layout;
    state.map_wizard_config.archetype = sc.archetype;
    state.map_wizard_config.room_shape = sc.room_shape;
    state.map_wizard_config.seed = sc.seed;
    state.map_wizard_config.sync_mode_conventions();
    state.map_wizard_config.auto_run_build_and_audit = false;
    if let Some(version) = std::env::var("FORGE99_TEST_GEN_VERSION").ok().and_then(|v| v.parse().ok()) {
        state.map_wizard_config.generator_version = version;
    }
    state.generate_map_from_wizard();
    CompilerBridge::build_unr_with_engine(&state, ut_dir, out)
        .unwrap_or_else(|e| panic!("[{}] engine build failed: {e}", sc.label));
    let level = UnrLevel::open(out).unwrap_or_else(|e| panic!("[{}] cannot reopen built map: {e}", sc.label));
    (level, state.world_brushes.to_vec())
}

#[test]
fn wizard_maps_have_physically_valid_actor_placement_on_the_engine_bsp() {
    let Some(ut_dir) = std::env::var_os("FORGE99_TEST_UT_DIR").map(PathBuf::from) else {
        eprintln!("Set FORGE99_TEST_UT_DIR to an isolated UT installation to run engine placement audit");
        return;
    };
    let temp_dir = common::TestDir::new();
    let scenarios = [
        Scenario { label: "complex-dm", mode: WizardGameMode::DeathMatch, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::InterconnectedComplex, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "arena-ctf", mode: WizardGameMode::CaptureTheFlag, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::CentralArenaMultiTier, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "twin-ctf", mode: WizardGameMode::CaptureTheFlag, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::SymmetricalTwinBases, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "orbital-dom", mode: WizardGameMode::Domination, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::OrbitalOutpostSkybox, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "complex-assault", mode: WizardGameMode::Assault, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::InterconnectedComplex, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "canyon-team", mode: WizardGameMode::TeamDeathMatch, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::CanyonCrestCitadels, room_shape: WizardRoomShape::Rectangular, seed: 1 },
        Scenario { label: "tower-octagon-dm", mode: WizardGameMode::DeathMatch, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::VerticalTowerShaft, room_shape: WizardRoomShape::Octagonal, seed: 1 },
        Scenario { label: "threeway-dom", mode: WizardGameMode::Domination, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::ThreeWayRingArena, room_shape: WizardRoomShape::Mixed, seed: 1 },
        Scenario { label: "fortress-assault", mode: WizardGameMode::Assault, layout: WizardLayoutStrategy::CanonicalArchetype, archetype: WizardArchetype::FortressSiegeLine, room_shape: WizardRoomShape::Chamfered, seed: 1 },
        Scenario { label: "proc-1", mode: WizardGameMode::DeathMatch, layout: WizardLayoutStrategy::ProceduralSeed, archetype: WizardArchetype::InterconnectedComplex, room_shape: WizardRoomShape::Rectangular, seed: 1234 },
        Scenario { label: "proc-2", mode: WizardGameMode::TeamDeathMatch, layout: WizardLayoutStrategy::ProceduralSeed, archetype: WizardArchetype::InterconnectedComplex, room_shape: WizardRoomShape::Mixed, seed: 98765 },
    ];

    let mut failures: Vec<String> = Vec::new();
    let filter = std::env::var("FORGE99_TEST_SCENARIO").ok();
    let mut executed = 0;
    for sc in &scenarios {
        if filter.as_deref().is_some_and(|wanted| wanted != sc.label) { continue; }
        executed += 1;
        let out = temp_dir.path().join(format!("forge99_placement_{}.unr", sc.label));
        let (lvl, brushes) = build_scenario(sc, &ut_dir, &out);
        let _ = std::fs::remove_file(&out);
        let bsp = lvl
            .world_bsp
            .as_ref()
            .unwrap_or_else(|| panic!("[{}] engine map has no compiled world BSP", sc.label));

        let mut starts = 0;
        for a in &lvl.actors {
            let cls = a.class_name.rsplit('.').next().unwrap_or(&a.class_name).to_string();
            let low = cls.to_ascii_lowercase();
            if matches!(low.as_str(), "levelinfo" | "brush" | "mover" | "inventoryspot") {
                continue;
            }
            if let Some(prop) = a.properties.get("bAutoBuilt") {
                if let ut99_format::TaggedValue::Bool(true) = prop.value {
                    continue;
                }
            }
            let prof = class_profile(&cls);
            if prof.mount == Mount::Fixed {
                continue;
            }
            let loc = a.location;
            let tag = format!("[{}] {}@({:.0},{:.0},{:.0})", sc.label, a.name, loc.x, loc.y, loc.z);

            if bsp.point_is_solid(loc) {
                failures.push(format!("{tag}: centre is inside solid geometry"));
                continue;
            }
            if low == "playerstart" {
                starts += 1;
            }
            if is_navigation_point(&cls) {
                if lvl.floor_distance(loc, 120.0, 1.0).is_none() {
                    failures.push(format!("{tag}: navigation point has no floor within 120uu"));
                }
                continue;
            }
            if !bsp.cylinder_is_free(loc, (prof.radius - 2.0).max(4.0), prof.height) {
                failures.push(format!("{tag}: collision cylinder (r={}, h={}) intersects solid", prof.radius, prof.height));
            }
            match prof.mount {
                Mount::Floor => match lvl.floor_distance(loc, 200.0, 0.5) {
                    None => failures.push(format!("{tag}: no floor below (floating)")),
                    Some(d) if (d - prof.rest_offset).abs() > 3.0 => {
                        failures.push(format!(
                            "{tag}: rests {d:.1}uu above the floor, expected {:.1}",
                            prof.rest_offset
                        ));
                    }
                    _ => {}
                },
                Mount::Ceiling => match bsp.ceiling_distance(loc, 300.0, 0.5) {
                    None => failures.push(format!("{tag}: no ceiling above (floating)")),
                    Some(d) if (d - prof.rest_offset).abs() > 3.0 => {
                        // UCC's world BSP may omit the collision volume of a thin
                        // semisolid visual panel. Accept only a panel whose lower
                        // face supports the fixture and whose upper face coincides
                        // with the measured structural ceiling.
                        let supported_by_panel = brushes.iter().any(|brush| {
                            if brush.csg_oper != ut99_bsp::CsgOperation::Add
                                || !brush.polys.iter().any(|p| p.flags & ut99_bsp::PF_SEMISOLID != 0) {
                                return false;
                            }
                            ut99_bsp::placement::ConvexVolume::from_brush(brush)
                                .and_then(|v| v.vertical_span(loc.x, loc.y))
                                .is_some_and(|((lo, _), (hi, _))| {
                                    hi - lo <= 4.5 && (lo - loc.z - prof.rest_offset).abs() <= 1.0
                                        && (hi - loc.z - d).abs() <= 1.0
                                })
                        });
                        if !supported_by_panel {
                            failures.push(format!("{tag}: hangs {d:.1}uu below the ceiling, expected {:.1}", prof.rest_offset));
                        }
                    }
                    _ => {}
                },
                Mount::Fixed => {}
            }
        }
        assert!(starts >= 4, "[{}] expected at least 4 PlayerStarts, found {starts}", sc.label);
    }

    assert!(executed > 0, "no engine scenario matched FORGE99_TEST_SCENARIO");

    assert!(
        failures.is_empty(),
        "{} placement problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
