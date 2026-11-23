//! Deterministic baseline of the Map Wizard.
//!
//! Runs a fixed matrix of configurations through `generate_valid_ut99_map` (no UCC, no disk
//! writes besides the report) and records size/telemetry metrics plus a content hash, so any later
//! generator change can be diffed against `baseline_v<version>.json`.
//!
//! Usage: cargo run --release -p ut99-bsp --example wizard_baseline -- [out.json]

use std::collections::BTreeMap;
use std::time::Instant;

use serde_json::{json, Value};
use ut99_bsp::{
    generate_valid_ut99_map, CsgOperation, IssueSeverity, LevelDoctor, MapWizardConfig, WizardArchetype, WizardGameMode,
    WizardLayoutStrategy, WizardMapScale,
};

const KNOWN_SEEDS: [u64; 10] = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
const UNSEEN_SEEDS: [u64; 10] = [7001, 7013, 7019, 7027, 7039, 7043, 7057, 7069, 7079, 7103];

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= *b as u64;
        *h = h.wrapping_mul(0x100_0000_01b3);
    }
}

fn run_case(label: String, cfg: &MapWizardConfig) -> Value {
    let t = Instant::now();
    let map = generate_valid_ut99_map(cfg);
    let ms = t.elapsed().as_secs_f64() * 1000.0;

    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let (mut sub, mut add) = (0usize, 0usize);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for b in &map.brushes {
        match b.csg_oper {
            CsgOperation::Subtract => sub += 1,
            CsgOperation::Add => add += 1,
            _ => {}
        }
        fnv(&mut h, b.name.as_bytes());
        for p in &b.polys {
            for v in &p.vertices {
                let w = *v + b.location;
                for (i, c) in [w.x, w.y, w.z].into_iter().enumerate() {
                    fnv(&mut h, &c.to_bits().to_le_bytes());
                    if b.csg_oper == CsgOperation::Subtract {
                        lo[i] = lo[i].min(c);
                        hi[i] = hi[i].max(c);
                    }
                }
            }
        }
    }
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &map.actors {
        fnv(&mut h, a.class_name.as_bytes());
        for c in [a.location.x, a.location.y, a.location.z] {
            fnv(&mut h, &c.to_bits().to_le_bytes());
        }
        *classes.entry(a.class_name.as_str()).or_default() += 1;
    }
    let mut issues = LevelDoctor::audit_brushes(&map.brushes);
    issues.extend(LevelDoctor::audit_zone_portals(&map.brushes));
    let doctor_errors = issues.iter().filter(|i| i.severity == IssueSeverity::Error).count();
    let doctor_warnings = issues.len() - doctor_errors;
    let extent = |i: usize| if hi[i] >= lo[i] { hi[i] - lo[i] } else { 0.0 };
    json!({
        "case": label,
        "hash": format!("{h:016x}"),
        "brushes": map.brushes.len(), "subtractive": sub, "additive": add,
        "actors": map.actors.len(), "movers": map.movers.len(), "warnings": map.warnings.len(), "doctor_errors": doctor_errors, "doctor_warnings": doctor_warnings,
        "doctor_samples": issues.iter().take(3).map(|i| format!("{:?} {} {}: {}", i.severity, i.category, i.object_name, i.description)).collect::<Vec<_>>(),
        "extent_xyz": [extent(0), extent(1), extent(2)],
        "est": {
            "brushes": map.preview.estimated_brushes, "lights": map.preview.estimated_lights,
            "spawns": map.preview.estimated_spawns, "path_nodes": map.preview.estimated_path_nodes,
            "weapons": map.preview.estimated_weapons_pickups,
        },
        "classes": classes,
        "gen_ms": (ms * 10.0).round() / 10.0,
    })
}

fn base_cfg(mode: WizardGameMode, scale: WizardMapScale, seed: u64) -> MapWizardConfig {
    let mut cfg = MapWizardConfig::default();
    cfg.game_mode = mode;
    cfg.scale = scale;
    cfg.seed = seed;
    cfg.sync_mode_conventions();
    cfg.auto_run_build_and_audit = false;
    if let Some(v) = std::env::var("FORGE99_GEN_VERSION").ok().and_then(|v| v.parse().ok()) {
        cfg.generator_version = v;
    }
    cfg
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "baseline.json".into());
    let modes = [
        WizardGameMode::DeathMatch,
        WizardGameMode::CaptureTheFlag,
        WizardGameMode::Domination,
        WizardGameMode::Assault,
    ];
    let mut cases = Vec::new();

    for mode in modes {
        // Procedural layouts: scales x (10 known + 10 unseen seeds).
        for scale in WizardMapScale::all() {
            for (set, seeds) in [("known", &KNOWN_SEEDS), ("unseen", &UNSEEN_SEEDS)] {
                for &seed in seeds {
                    let mut cfg = base_cfg(mode, *scale, seed);
                    cfg.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
                    let label = format!("{mode:?}/proc/{scale:?}/{set}/{seed}");
                    let mut v = run_case(label, &cfg);
                    v["set"] = json!(set);
                    cases.push(v);
                }
            }
        }
        // Canonical archetypes: Standard scale, seeds 1 and 2.
        for arch in WizardArchetype::all() {
            for seed in [1u64, 2] {
                let mut cfg = base_cfg(mode, WizardMapScale::Standard, seed);
                cfg.layout_strategy = WizardLayoutStrategy::CanonicalArchetype;
                cfg.archetype = *arch;
                let label = format!("{mode:?}/{arch:?}/Standard/known/{seed}");
                let mut v = run_case(label, &cfg);
                v["set"] = json!("known");
                cases.push(v);
            }
        }
    }

    // Determinism check: the same configuration generated twice must hash identically.
    let mut nondeterministic = Vec::new();
    for mode in modes {
        let mut cfg = base_cfg(mode, WizardMapScale::Standard, 1);
        cfg.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
        let a = run_case("a".into(), &cfg)["hash"].clone();
        let b = run_case("b".into(), &cfg)["hash"].clone();
        if a != b {
            nondeterministic.push(format!("{mode:?}"));
        }
    }

    let report = json!({
        "generator_version": env!("CARGO_PKG_VERSION"),
        "rules_version": std::env::var("FORGE99_GEN_VERSION").unwrap_or_else(|_| "1".into()),
        "case_count": cases.len(),
        "determinism_failures": nondeterministic.len(),
        "cases": cases,
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("{} cases -> {out} (determinism failures: {})", report["case_count"], nondeterministic.len());
}
