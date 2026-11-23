//! Clearance audit for generated modules.
//!
//! For each procedural case, every additive/semisolid brush and every spawn/pickup actor is matched with
//! the carved room box that contains it, and the headroom between its top and the room ceiling is
//! computed. Findings are aggregated by module family so a rules version can be compared before/after.
//!
//! Usage: FORGE99_GEN_VERSION=3 cargo run --release -p ut99-bsp --example module_clearance

use std::collections::BTreeMap;

use ut99_bsp::{generate_valid_ut99_map, Brush, CsgOperation, MapWizardConfig, WizardGameMode, WizardLayoutStrategy, WizardMapScale};

fn bounds(b: &Brush) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &b.polys {
        for v in &p.vertices {
            let w = *v + b.location;
            for (i, c) in [w.x, w.y, w.z].into_iter().enumerate() {
                lo[i] = lo[i].min(c);
                hi[i] = hi[i].max(c);
            }
        }
    }
    (lo, hi)
}

/// Strip room/index suffixes so "ProcChamber_4_Foo_2" and "ProcChamber_7_Foo_0" fall in one family.
fn family(name: &str) -> String {
    let mut parts: Vec<&str> = name.split('_').filter(|p| !p.chars().all(|c| c.is_ascii_digit())).collect();
    parts.retain(|p| !p.starts_with("Proc") && !p.starts_with("Room") && !p.starts_with("Corridor"));
    if parts.is_empty() {
        name.to_string()
    } else {
        parts.join("_")
    }
}

fn main() {
    let ver: u32 = std::env::var("FORGE99_GEN_VERSION").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let mut poke: BTreeMap<String, (usize, f32)> = BTreeMap::new();
    let mut tight: BTreeMap<String, (usize, f32)> = BTreeMap::new();
    let mut actor_tight: BTreeMap<String, (usize, f32)> = BTreeMap::new();
    let mut total_brushes = 0usize;

    for mode in [WizardGameMode::DeathMatch, WizardGameMode::CaptureTheFlag, WizardGameMode::Domination, WizardGameMode::Assault] {
        for scale in WizardMapScale::all() {
            for seed in [1u64, 2, 3, 5, 8, 13, 21, 34, 7001, 7013, 7019, 7027] {
                let mut cfg = MapWizardConfig::default();
                cfg.game_mode = mode;
                cfg.scale = *scale;
                cfg.seed = seed;
                cfg.sync_mode_conventions();
                cfg.auto_run_build_and_audit = false;
                cfg.generator_version = ver;
                cfg.layout_strategy = WizardLayoutStrategy::ProceduralSeed;
                let map = generate_valid_ut99_map(&cfg);
                let rooms: Vec<([f32; 3], [f32; 3])> =
                    map.brushes.iter().filter(|b| b.csg_oper == CsgOperation::Subtract).map(bounds).collect();
                let room_at = |x: f32, y: f32, z: f32| {
                    rooms
                        .iter()
                        .filter(|(lo, hi)| x > lo[0] && x < hi[0] && y > lo[1] && y < hi[1] && z > lo[2] - 1.0 && z < hi[2] + 1.0)
                        .min_by(|a, b| ((a.1[2] - a.0[2]) as i32).cmp(&((b.1[2] - b.0[2]) as i32)))
                        .copied()
                };
                for b in map.brushes.iter().filter(|b| b.csg_oper == CsgOperation::Add) {
                    total_brushes += 1;
                    let (lo, hi) = bounds(b);
                    let (cx, cy) = ((lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5);
                    let Some((rlo, rhi)) = room_at(cx, cy, lo[2] + 2.0) else { continue };
                    let head = rhi[2] - hi[2];
                    let fam = family(&b.name);
                    if std::env::var("CLEAR_DEBUG").is_ok() && b.name.starts_with("TransitStair_") && head < 128.0 {
                        println!("  {} seed={seed} {mode:?} {scale:?} stairtop={:.0} room=[{:.0},{:.0}] head={head:.0}", b.name, hi[2], rlo[2], rhi[2]);
                    }
                    if head < -8.0 && hi[2] - lo[2] < (rhi[2] - rlo[2]) {
                        let e = poke.entry(fam.clone()).or_default();
                        e.0 += 1;
                        e.1 = e.1.min(head);
                    }
                    // Platforms the player stands on need ~128 UU of headroom above the top face.
                    let walk = ["Balcony", "Catwalk", "Deck", "Bridge", "Stair", "Ramp", "Lift", "Platform", "Pedestal"];
                    if walk.iter().any(|w| b.name.contains(w)) && head >= -8.0 && head < 128.0 {
                        let e = tight.entry(fam).or_insert((0, f32::MAX));
                        e.0 += 1;
                        e.1 = e.1.min(head);
                    }
                }
                for a in &map.actors {
                    let interesting = a.class_name == "PlayerStart"
                        || a.class_name.contains("Flag")
                        || a.class_name.contains("Jump")
                        || a.class_name == "Kicker"
                        || a.class_name.starts_with("Weapon")
                        || a.class_name == "Translocator";
                    if !interesting {
                        continue;
                    }
                    if let Some((_, rhi)) = room_at(a.location.x, a.location.y, a.location.z) {
                        let head = rhi[2] - a.location.z;
                        if head < 96.0 {
                            let e = actor_tight.entry(a.class_name.clone()).or_insert((0, f32::MAX));
                            e.0 += 1;
                            e.1 = e.1.min(head);
                        }
                    }
                }
            }
        }
    }
    println!("rules v{ver}: {total_brushes} additive brushes audited");
    println!("-- brushes poking through the ceiling (family: count, worst overshoot)");
    for (k, (n, w)) in &poke {
        println!("  {k}: {n}, {w:.0}");
    }
    println!("-- walkable modules with < 128 UU headroom (family: count, worst headroom)");
    for (k, (n, w)) in &tight {
        println!("  {k}: {n}, {w:.0}");
    }
    println!("-- actors with < 96 UU above (class: count, worst)");
    for (k, (n, w)) in &actor_tight {
        println!("  {k}: {n}, {w:.0}");
    }
}
