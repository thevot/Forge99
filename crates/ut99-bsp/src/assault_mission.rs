// crates/ut99-bsp/src/assault_mission.rs
//
// Assault (AS) "Mission Graph" planner for the Map Wizard.
//
// Derived from a forensic study of the 8 official UT99 Assault maps (AS-Frigate, Guardia, HiSpeed,
// Mazon, OceanFloor, Overlord, Rook, Tutorial): 3-5 `FortStandard` objectives per map, completed by
// shooting a target, activating a switch (`bTriggerOnly`), touching a zone, or destroying a chain of
// targets joined by a `Counter`. Each completed objective fires an `Event` that opens the next area
// (`Mover` door / bridge), so attackers progress sequentially from their respawn to the final fort.
//
// The planner works on a simplified room graph (`MissionNode`) so it stays independent of the
// wizard's private `RoomNode` and is fully unit-testable.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Simplified axis-aligned room/corridor volume of the wizard's room graph.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionNode {
    pub center: Vec3,
    pub size: Vec3,
    pub is_corridor: bool,
}

/// How an Assault objective is completed (mirrors the four families seen in official maps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssaultObjectiveKind {
    /// Shoot a destructible `FortStandard` (AS-Tutorial fuse, AS-Rook chains).
    Destroy,
    /// Walk up to a console and press its button (`Trigger` proximity -> `bTriggerOnly` fort).
    SwitchTouch,
    /// Shoot a lever/button (`TeamTrigger` with `TT_Shoot`) (AS-Tutorial, AS-Guardia switches).
    SwitchShoot,
    /// Reach and hold a zone (`bTriggerOnly` fort + large radius) (AS-Guardia "Lava Bridge").
    TouchZone,
    /// Destroy two targets joined by a `Counter(NumToCount=2)` (AS-Rook winch chains).
    Chain,
}

impl AssaultObjectiveKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Destroy => "Destroy Target",
            Self::SwitchTouch => "Activate Console Switch",
            Self::SwitchShoot => "Shoot Lever / Button",
            Self::TouchZone => "Secure Zone",
            Self::Chain => "Destroy Chain Pair",
        }
    }

    pub fn is_switch(&self) -> bool {
        matches!(self, Self::SwitchTouch | Self::SwitchShoot)
    }
}

/// One planned objective in mission order.
#[derive(Debug, Clone, PartialEq)]
pub struct AssaultObjectivePlan {
    pub index: usize,
    pub kind: AssaultObjectiveKind,
    /// Index into the node slice passed to [`plan_assault_mission`].
    pub node_idx: usize,
    pub fort_name: String,
    pub fort_tag: String,
    pub destroyed_message: String,
    /// Corridor node sealed by a door until this objective is completed (`None` = ungated).
    pub gate_node: Option<usize>,
    pub gate_tag: Option<String>,
    pub is_final: bool,
}

/// Complete deterministic mission plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AssaultMissionPlan {
    pub start_node: usize,
    pub objectives: Vec<AssaultObjectivePlan>,
    /// Non-corridor nodes where defenders respawn (behind the first gate).
    pub defender_nodes: Vec<usize>,
    /// BFS depth from `start_node` (all gates open); `None` = unreachable.
    pub depth: Vec<Option<u32>>,
}

/// Deterministic SplitMix64 (local copy; keeps this module free of wizard internals).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() as usize) % items.len()]
    }
}

const DESTROY_NAMES: &[&str] = &[
    "Blast Door Generator",
    "Power Coupler",
    "Shield Capacitor",
    "Fuel Tank",
    "Signal Relay",
    "Plasma Conduit",
];
const SWITCH_NAMES: &[&str] = &[
    "Gate Control Switch",
    "Bridge Lever",
    "Security Override",
    "Lift Control Terminal",
    "Door Access Panel",
];
const ZONE_NAMES: &[&str] = &["Inner Sanctum", "Command Deck", "Reactor Hall", "Outer Courtyard"];
const CHAIN_NAMES: &[&str] = &["Support Cable", "Winch Chain", "Coolant Valve", "Anchor Bolt"];

/// AABB adjacency (overlapping or touching within `eps` in XY, overlapping in Z).
pub fn node_adjacency(nodes: &[MissionNode]) -> Vec<Vec<usize>> {
    let eps = 8.0_f32;
    let mut adj = vec![Vec::new(); nodes.len()];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let a = &nodes[i];
            let b = &nodes[j];
            let dx = (a.center.x - b.center.x).abs() - (a.size.x + b.size.x) * 0.5;
            let dy = (a.center.y - b.center.y).abs() - (a.size.y + b.size.y) * 0.5;
            let dz = (a.center.z - b.center.z).abs() - (a.size.z + b.size.z) * 0.5;
            if dx <= eps && dy <= eps && dz < 0.0 {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }
    adj
}

/// Nodes reachable from `start` when the nodes flagged in `closed` are sealed.
pub fn reachable_from(adj: &[Vec<usize>], start: usize, closed: &[bool]) -> Vec<bool> {
    let mut seen = vec![false; adj.len()];
    if start >= adj.len() || closed.get(start).copied().unwrap_or(false) {
        return seen;
    }
    let mut stack = vec![start];
    seen[start] = true;
    while let Some(u) = stack.pop() {
        for &v in &adj[u] {
            if !seen[v] && !closed.get(v).copied().unwrap_or(false) {
                seen[v] = true;
                stack.push(v);
            }
        }
    }
    seen
}

fn bfs_depth(adj: &[Vec<usize>], start: usize) -> Vec<Option<u32>> {
    let mut depth = vec![None; adj.len()];
    if start >= adj.len() {
        return depth;
    }
    depth[start] = Some(0);
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(u) = queue.pop_front() {
        let d = depth[u].unwrap_or(0);
        for &v in &adj[u] {
            if depth[v].is_none() {
                depth[v] = Some(d + 1);
                queue.push_back(v);
            }
        }
    }
    depth
}

/// Plans a sequential Assault mission over the room graph.
///
/// * `count` objectives (clamped to `1..=5` and to the number of reachable rooms).
/// * Objectives are ordered by BFS depth from the attacker start room; the deepest is the final fort.
/// * Between objective `i` and `i+1` a corridor is chosen as a *gate* only if sealing it alone
///   disconnects objective `i+1` from the start while keeping objective `i` reachable (true min-cut),
///   so the mission can never soft-lock; otherwise that step stays ungated.
/// * At least one switch objective is guaranteed when `count >= 3`.
pub fn plan_assault_mission(nodes: &[MissionNode], count: usize, seed: u64) -> AssaultMissionPlan {
    let mut plan = AssaultMissionPlan::default();
    let adj = node_adjacency(nodes);
    // Select the start room among non-corridor rooms that maximizes the longest BFS reach
    // (i.e. peripheral entry point / landing), breaking ties by lower degree (dead-end) then lowest index.
    let start = (0..nodes.len())
        .filter(|&i| !nodes[i].is_corridor)
        .max_by_key(|&i| {
            let d = bfs_depth(&adj, i);
            let max_room_d = (0..nodes.len())
                .filter(|&j| !nodes[j].is_corridor)
                .filter_map(|j| d[j])
                .max()
                .unwrap_or(0);
            (max_room_d, std::cmp::Reverse(adj[i].len()), std::cmp::Reverse(i))
        });
    let Some(start) = start else {
        return plan;
    };
    plan.start_node = start;
    plan.depth = bfs_depth(&adj, start);

    let mut rooms: Vec<usize> = (0..nodes.len())
        .filter(|&i| !nodes[i].is_corridor && i != start && plan.depth[i].is_some())
        .collect();
    rooms.sort_by_key(|&i| (plan.depth[i].unwrap_or(0), i));

    // Objective 0 may live in the start room only when the map has no other room.
    let mut chosen: Vec<usize> = Vec::new();
    let want = count.clamp(1, 5);
    if rooms.is_empty() {
        chosen.push(start);
    } else {
        let n = want.min(rooms.len());
        if n == 1 {
            chosen.push(*rooms.last().unwrap());
        } else {
            for k in 0..n {
                let idx = (k * (rooms.len() - 1) + (n - 1) / 2) / (n - 1);
                let node = rooms[idx.min(rooms.len() - 1)];
                if !chosen.contains(&node) {
                    chosen.push(node);
                }
            }
            if let Some(&deepest) = rooms.last() {
                if !chosen.contains(&deepest) {
                    chosen.pop();
                    chosen.push(deepest);
                }
            }
        }
    }
    chosen.sort_by_key(|&i| (plan.depth[i].unwrap_or(0), i));
    chosen.dedup();

    let mut rng = Rng::new(seed ^ 0xA55A_17F0_0D0D_C0DE);
    let total = chosen.len();
    let mut kinds: Vec<AssaultObjectiveKind> = (0..total)
        .map(|i| {
            let is_final = i + 1 == total;
            let pool: &[AssaultObjectiveKind] = if is_final {
                &[
                    AssaultObjectiveKind::Destroy,
                    AssaultObjectiveKind::SwitchTouch,
                    AssaultObjectiveKind::SwitchShoot,
                    AssaultObjectiveKind::TouchZone,
                ]
            } else {
                &[
                    AssaultObjectiveKind::Destroy,
                    AssaultObjectiveKind::SwitchTouch,
                    AssaultObjectiveKind::SwitchShoot,
                    AssaultObjectiveKind::TouchZone,
                    AssaultObjectiveKind::Chain,
                    AssaultObjectiveKind::Destroy,
                ]
            };
            *rng.pick(pool)
        })
        .collect();
    if total >= 3 && !kinds.iter().any(|k| k.is_switch()) {
        kinds[1] = AssaultObjectiveKind::SwitchTouch;
    }

    for (i, &node_idx) in chosen.iter().enumerate() {
        let kind = kinds[i];
        let names = match kind {
            AssaultObjectiveKind::Destroy => DESTROY_NAMES,
            AssaultObjectiveKind::SwitchTouch | AssaultObjectiveKind::SwitchShoot => SWITCH_NAMES,
            AssaultObjectiveKind::TouchZone => ZONE_NAMES,
            AssaultObjectiveKind::Chain => CHAIN_NAMES,
        };
        let fort_name = (*rng.pick(names)).to_string();
        let destroyed_message = match kind {
            AssaultObjectiveKind::Destroy => "was destroyed!",
            AssaultObjectiveKind::SwitchTouch | AssaultObjectiveKind::SwitchShoot => {
                "was activated!"
            }
            AssaultObjectiveKind::TouchZone => "has been secured!",
            AssaultObjectiveKind::Chain => "has been severed!",
        }
        .to_string();
        plan.objectives.push(AssaultObjectivePlan {
            index: i,
            kind,
            node_idx,
            fort_name,
            fort_tag: format!("AS_Obj{i}"),
            destroyed_message,
            gate_node: None,
            gate_tag: None,
            is_final: i + 1 == total,
        });
    }

    // Gates: strict min-cut corridors between consecutive objectives.
    let mut used_gate = vec![false; nodes.len()];
    for i in 0..plan.objectives.len().saturating_sub(1) {
        let here = plan.objectives[i].node_idx;
        let next = plan.objectives[i + 1].node_idx;
        let mut best: Option<(u32, usize)> = None;
        for c in 0..nodes.len() {
            if !nodes[c].is_corridor || used_gate[c] || plan.depth[c].is_none() {
                continue;
            }
            let mut closed = vec![false; nodes.len()];
            closed[c] = true;
            let reach = reachable_from(&adj, start, &closed);
            if reach[here] && !reach[next] {
                // Prefer the gate closest (in depth) to the objective that unlocks it.
                let dist = plan.depth[c]
                    .unwrap_or(0)
                    .abs_diff(plan.depth[here].unwrap_or(0));
                if best.map_or(true, |(bd, _)| dist < bd) {
                    best = Some((dist, c));
                }
            }
        }
        if let Some((_, c)) = best {
            used_gate[c] = true;
            plan.objectives[i].gate_node = Some(c);
            plan.objectives[i].gate_tag = Some(format!("AS_Gate{i}"));
        }
    }

    // Defenders respawn in the region unlocked by the first gate (or the deepest rooms if ungated).
    let gates: Vec<usize> = plan.objectives.iter().filter_map(|o| o.gate_node).collect();
    let mut defenders: Vec<usize> = Vec::new();
    if let Some(&g0) = gates.first() {
        let mut closed_all = vec![false; nodes.len()];
        for &g in &gates {
            closed_all[g] = true;
        }
        let before = reachable_from(&adj, start, &closed_all);
        let mut open_first = closed_all.clone();
        open_first[g0] = false;
        let after = reachable_from(&adj, start, &open_first);
        for i in 0..nodes.len() {
            if !nodes[i].is_corridor && after[i] && !before[i] {
                defenders.push(i);
            }
        }
    }
    if defenders.is_empty() {
        defenders = rooms.iter().rev().take(2).copied().collect();
        if defenders.is_empty() {
            defenders.push(start);
        }
    }
    defenders.truncate(3);
    plan.defender_nodes = defenders;
    plan
}

/// Auto-generated mission briefing (`AssaultInfo.ObjDesc[i]`), one line per objective.
pub fn briefing_lines(plan: &AssaultMissionPlan) -> Vec<String> {
    plan.objectives
        .iter()
        .map(|o| {
            let unlock = if o.gate_node.is_some() {
                " This opens the way deeper into the base."
            } else {
                ""
            };
            match o.kind {
                AssaultObjectiveKind::Destroy => {
                    format!("Destroy the {} to break the enemy defenses.{unlock}", o.fort_name)
                }
                AssaultObjectiveKind::SwitchTouch => format!(
                    "Reach the control console and press the {} to unlock the next section.{unlock}",
                    o.fort_name
                ),
                AssaultObjectiveKind::SwitchShoot => format!(
                    "Shoot the {} to activate it and unlock the next section.{unlock}",
                    o.fort_name
                ),
                AssaultObjectiveKind::TouchZone => {
                    format!("Fight your way into the {} and secure it.{unlock}", o.fort_name)
                }
                AssaultObjectiveKind::Chain => format!(
                    "Destroy both {} anchors; once they are cut the passage opens.{unlock}",
                    o.fort_name
                ),
            }
        })
        .collect()
}

/// Structural invariant check used by tests and the generator's self-audit. Returns human-readable
/// errors (empty = valid).
pub fn audit_assault_plan(nodes: &[MissionNode], plan: &AssaultMissionPlan) -> Vec<String> {
    let mut errors = Vec::new();
    if plan.objectives.is_empty() {
        errors.push("mission has no objectives".to_string());
        return errors;
    }
    if plan.objectives.iter().filter(|o| o.is_final).count() != 1 {
        errors.push("mission must have exactly one final objective".to_string());
    }
    if !plan.objectives.last().map_or(false, |o| o.is_final) {
        errors.push("final objective must be last".to_string());
    }
    let adj = node_adjacency(nodes);
    let gates: Vec<usize> = plan.objectives.iter().filter_map(|o| o.gate_node).collect();
    let mut tags = std::collections::HashSet::new();
    for o in &plan.objectives {
        if !tags.insert(o.fort_tag.clone()) {
            errors.push(format!("duplicate fort tag {}", o.fort_tag));
        }
    }
    // Sequential reachability: objective i is reachable once gates g_0..g_{i-1} are open and not before.
    for (i, o) in plan.objectives.iter().enumerate() {
        let mut closed = vec![false; nodes.len()];
        for (j, other) in plan.objectives.iter().enumerate() {
            if j >= i {
                if let Some(g) = other.gate_node {
                    closed[g] = true;
                }
            }
        }
        let reach = reachable_from(&adj, plan.start_node, &closed);
        if !reach[o.node_idx] {
            errors.push(format!("objective {i} unreachable with previous gates open"));
        }
        if i > 0 {
            let mut all_closed = vec![false; nodes.len()];
            for &g in &gates {
                all_closed[g] = true;
            }
            if plan.objectives[i - 1].gate_node.is_some() {
                let r = reachable_from(&adj, plan.start_node, &all_closed);
                if r[o.node_idx] {
                    errors.push(format!("objective {i} reachable before its gate opens"));
                }
            }
        }
    }
    // Defenders must not respawn on the attackers' side of the first gate.
    if let Some(&g0) = gates.first() {
        let mut closed = vec![false; nodes.len()];
        closed[g0] = true;
        let reach = reachable_from(&adj, plan.start_node, &closed);
        for &d in &plan.defender_nodes {
            if reach[d] {
                errors.push("defender spawn room reachable by attackers before the first gate".to_string());
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(x: f32, y: f32, w: f32, h: f32) -> MissionNode {
        MissionNode {
            center: Vec3::new(x, y, 192.0),
            size: Vec3::new(w, h, 384.0),
            is_corridor: false,
        }
    }
    fn corridor(x: f32, y: f32, w: f32, h: f32) -> MissionNode {
        MissionNode {
            is_corridor: true,
            ..room(x, y, w, h)
        }
    }

    /// Linear chain: R0 - c - R1 - c - R2 - c - R3
    fn linear_nodes() -> Vec<MissionNode> {
        vec![
            room(0.0, 0.0, 512.0, 512.0),
            corridor(0.0, 512.0, 256.0, 512.0),
            room(0.0, 1024.0, 512.0, 512.0),
            corridor(0.0, 1536.0, 256.0, 512.0),
            room(0.0, 2048.0, 512.0, 512.0),
            corridor(0.0, 2560.0, 256.0, 512.0),
            room(0.0, 3072.0, 512.0, 512.0),
        ]
    }

    #[test]
    fn test_linear_mission_gates_and_order() {
        let nodes = linear_nodes();
        let plan = plan_assault_mission(&nodes, 3, 7);
        assert_eq!(plan.objectives.len(), 3);
        assert!(plan.objectives.last().unwrap().is_final);
        assert_eq!(plan.objectives.iter().filter(|o| o.is_final).count(), 1);
        assert!(plan.objectives.iter().any(|o| o.kind.is_switch()));
        let depths: Vec<_> = plan
            .objectives
            .iter()
            .map(|o| plan.depth[o.node_idx].unwrap())
            .collect();
        assert!(depths.windows(2).all(|w| w[0] <= w[1]));
        // linear graph => every non-final objective is gated
        assert!(plan.objectives[..2].iter().all(|o| o.gate_node.is_some()));
        assert_eq!(audit_assault_plan(&nodes, &plan), Vec::<String>::new());
    }

    #[test]
    fn test_loop_without_bridge_corridor_stays_ungated() {
        // Two rooms joined by two parallel corridors: no single corridor is a min-cut.
        let nodes = vec![
            room(0.0, 0.0, 512.0, 512.0),
            corridor(-128.0, 512.0, 128.0, 512.0),
            corridor(128.0, 512.0, 128.0, 512.0),
            room(0.0, 1024.0, 512.0, 512.0),
        ];
        let plan = plan_assault_mission(&nodes, 2, 3);
        assert!(plan.objectives.iter().all(|o| o.gate_node.is_none()));
        assert!(audit_assault_plan(&nodes, &plan).is_empty());
    }

    #[test]
    fn test_plan_is_deterministic_and_briefing_matches() {
        let nodes = linear_nodes();
        let a = plan_assault_mission(&nodes, 4, 99);
        let b = plan_assault_mission(&nodes, 4, 99);
        assert_eq!(a, b);
        assert_eq!(briefing_lines(&a).len(), a.objectives.len());
    }
}
