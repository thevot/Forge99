//! Teleporter Destination, Network Link & Routing Studio
//!
//! Provides topological analysis, diagnostic auditing, exit vector calculation,
//! and automated link pairing for Unreal Engine 1 teleporter networks.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical severity classification for teleporter network diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeleportDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Category of teleporter network diagnostic issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeleportDiagnosticCategory {
    /// URL specifies a destination tag that does not exist in any teleporter in the map.
    BrokenUrl,
    /// Teleporter defines a Tag but no other teleporter has a URL pointing to it.
    OrphanDestination,
    /// Direct ping-pong loop (A -> B -> A) or cyclic chain with zero delay/offset.
    InfiniteLoopHazard,
    /// Destination exit is dangerously close (< 64 uu) to another teleporter or PlayerStart.
    TelefragHazard,
    /// Destination exit imparts zero velocity and zero yaw redirection.
    ImmobileExit,
}

/// A single diagnostic finding for a teleporter node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleportDiagnostic {
    pub severity: TeleportDiagnosticSeverity,
    pub node_id: usize,
    pub category: TeleportDiagnosticCategory,
    pub message: String,
}

/// A teleporter node representing an in-map `Teleporter` actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleporterNode {
    pub id: usize,
    pub name: String,
    pub tag: String,
    pub url: String,
    pub location: Vec3,
    pub rotation: Vec3, // Pitch, Yaw, Roll in degrees
    pub b_changes_velocity: bool,
    pub target_velocity: Vec3,
    pub b_changes_yaw: bool,
    pub b_enabled: bool,
}

impl TeleporterNode {
    pub fn new(id: usize, name: impl Into<String>, tag: impl Into<String>, url: impl Into<String>, location: Vec3) -> Self {
        Self {
            id,
            name: name.into(),
            tag: tag.into(),
            url: url.into(),
            location,
            rotation: Vec3::new(0.0, 0.0, 0.0),
            b_changes_velocity: false,
            target_velocity: Vec3::new(0.0, 0.0, 0.0),
            b_changes_yaw: true,
            b_enabled: true,
        }
    }

    /// Computes the forward facing vector from the node's rotation yaw.
    pub fn forward_direction(&self) -> Vec3 {
        let yaw_rad = self.rotation.y.to_radians();
        let pitch_rad = self.rotation.x.to_radians();
        let cp = pitch_rad.cos();
        Vec3::new(cp * yaw_rad.cos(), cp * yaw_rad.sin(), pitch_rad.sin()).normalize()
    }
}

/// A resolved directional or bidirectional link between two teleporters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeleportLink {
    pub source_id: usize,
    pub dest_id: usize,
    pub source_tag: String,
    pub dest_tag: String,
    pub distance: f32,
    pub is_bidirectional: bool,
    pub exit_velocity: Vec3,
    pub exit_direction: Vec3,
}

/// Topological graph representing the entire teleporter network in a level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeleportNetworkGraph {
    pub nodes: Vec<TeleporterNode>,
    pub links: Vec<TeleportLink>,
    pub diagnostics: Vec<TeleportDiagnostic>,
}

impl TeleportNetworkGraph {
    /// Builds the network graph from teleporter nodes and known player spawn locations.
    pub fn build(nodes: Vec<TeleporterNode>, player_starts: &[Vec3]) -> Self {
        let mut links = Vec::new();
        let mut diagnostics = Vec::new();

        // 1. Resolve Links
        for (i, source) in nodes.iter().enumerate() {
            if !source.b_enabled {
                continue;
            }

            let trimmed_url = source.url.trim();
            if trimmed_url.is_empty() {
                // Not necessarily an error: can be a receive-only teleporter
                continue;
            }

            // In UT99, URL can be a level name or actor tag. In-map teleporters match destination Tag (case-insensitive).
            let mut matches = Vec::new();
            for (j, candidate) in nodes.iter().enumerate() {
                if i != j && candidate.tag.eq_ignore_ascii_case(trimmed_url) {
                    matches.push((j, candidate));
                }
            }

            if matches.is_empty() {
                diagnostics.push(TeleportDiagnostic {
                    severity: TeleportDiagnosticSeverity::Error,
                    node_id: source.id,
                    category: TeleportDiagnosticCategory::BrokenUrl,
                    message: format!(
                        "Teleporter '{}' has URL '{}' matching no destination Tag in level.",
                        source.name, source.url
                    ),
                });
            } else {
                for (_dest_idx, dest) in matches {
                    let diff = dest.location - source.location;
                    let distance = diff.length();

                    // Calculate exit direction & velocity
                    let exit_dir = if dest.b_changes_yaw {
                        dest.forward_direction()
                    } else {
                        // Keeps incoming direction, default forward
                        source.forward_direction()
                    };

                    let exit_vel = if dest.b_changes_velocity {
                        dest.target_velocity
                    } else {
                        // Preserves speed, mapped along exit_dir
                        let default_speed = 400.0;
                        exit_dir * default_speed
                    };

                    // Check if candidate also points back to source (bidirectional pair)
                    let points_back = dest.url.trim().eq_ignore_ascii_case(&source.tag) && !source.tag.is_empty();

                    // Infinite loop hazard check: A -> B -> A where distance is small (< 96 uu)
                    if points_back && distance < 96.0 {
                        diagnostics.push(TeleportDiagnostic {
                            severity: TeleportDiagnosticSeverity::Error,
                            node_id: source.id,
                            category: TeleportDiagnosticCategory::InfiniteLoopHazard,
                            message: format!(
                                "Critical Loop: Teleporter '{}' and '{}' link to each other within {:.1} uu (infinite ping-pong crash hazard).",
                                source.name, dest.name, distance
                            ),
                        });
                    }

                    links.push(TeleportLink {
                        source_id: source.id,
                        dest_id: dest.id,
                        source_tag: source.tag.clone(),
                        dest_tag: dest.tag.clone(),
                        distance,
                        is_bidirectional: points_back,
                        exit_velocity: exit_vel,
                        exit_direction: exit_dir,
                    });
                }
            }
        }

        // 2. Orphan Destination Audit
        for node in &nodes {
            if !node.tag.trim().is_empty() {
                let has_incoming = links.iter().any(|link| link.dest_id == node.id);
                if !has_incoming {
                    diagnostics.push(TeleportDiagnostic {
                        severity: TeleportDiagnosticSeverity::Info,
                        node_id: node.id,
                        category: TeleportDiagnosticCategory::OrphanDestination,
                        message: format!(
                            "Teleporter '{}' has Tag '{}' but no other teleporter targets it.",
                            node.name, node.tag
                        ),
                    });
                }
            }
        }

        // 3. Telefrag Hazards Audit
        // Check destination exit proximity to other teleporters and player spawns
        let telefrag_radius: f32 = 64.0;
        for link in &links {
            if let Some(dest_node) = nodes.iter().find(|n| n.id == link.dest_id) {
                // Check proximity to player starts
                for (spawn_idx, spawn_pos) in player_starts.iter().enumerate() {
                    let d = (dest_node.location - *spawn_pos).length();
                    if d < telefrag_radius {
                        diagnostics.push(TeleportDiagnostic {
                            severity: TeleportDiagnosticSeverity::Warning,
                            node_id: dest_node.id,
                            category: TeleportDiagnosticCategory::TelefragHazard,
                            message: format!(
                                "Telefrag Hazard: Destination '{}' is {:.1} uu from PlayerStart #{} (radius < {:.0} uu).",
                                dest_node.name, d, spawn_idx, telefrag_radius
                            ),
                        });
                    }
                }

                // Check proximity to other teleporter entrances (exit on top of another teleporter)
                for other in &nodes {
                    if other.id != dest_node.id {
                        let d = (dest_node.location - other.location).length();
                        if d < telefrag_radius {
                            diagnostics.push(TeleportDiagnostic {
                                severity: TeleportDiagnosticSeverity::Warning,
                                node_id: dest_node.id,
                                category: TeleportDiagnosticCategory::TelefragHazard,
                                message: format!(
                                    "Exit Contention: Destination '{}' overlaps teleporter '{}' at {:.1} uu.",
                                    dest_node.name, other.name, d
                                ),
                            });
                        }
                    }
                }

                // Check immobile exit
                if dest_node.b_changes_velocity && dest_node.target_velocity.length() < 1.0 {
                    diagnostics.push(TeleportDiagnostic {
                        severity: TeleportDiagnosticSeverity::Info,
                        node_id: dest_node.id,
                        category: TeleportDiagnosticCategory::ImmobileExit,
                        message: format!(
                            "Destination '{}' sets target velocity to zero (players stall on exit).",
                            dest_node.name
                        ),
                    });
                }
            }
        }

        Self {
            nodes,
            links,
            diagnostics,
        }
    }

    /// Computes the exact exit launch velocity and facing for a given source teleporter.
    pub fn calculate_exit_trajectory(&self, source_id: usize, incoming_speed: f32) -> Option<(Vec3, Vec3)> {
        let link = self.links.iter().find(|l| l.source_id == source_id)?;
        let dest = self.nodes.iter().find(|n| n.id == link.dest_id)?;

        let exit_dir = if dest.b_changes_yaw {
            dest.forward_direction()
        } else {
            let src = self.nodes.iter().find(|n| n.id == source_id)?;
            src.forward_direction()
        };

        let exit_vel = if dest.b_changes_velocity {
            dest.target_velocity
        } else {
            exit_dir * incoming_speed.max(300.0)
        };

        Some((exit_vel, exit_dir))
    }

    /// Generates a perfectly coordinated bidirectional pair of teleporters.
    pub fn generate_paired_link(
        pos_a: Vec3,
        yaw_a: f32,
        pos_b: Vec3,
        yaw_b: f32,
        tag_prefix: &str,
        next_id_start: usize,
    ) -> (TeleporterNode, TeleporterNode) {
        let tag_a = format!("{}_PortalA", tag_prefix);
        let tag_b = format!("{}_PortalB", tag_prefix);

        let mut node_a = TeleporterNode::new(
            next_id_start,
            format!("Teleporter_{}", tag_a),
            &tag_a,
            &tag_b,
            pos_a,
        );
        node_a.rotation.y = yaw_a;
        node_a.b_changes_velocity = true;
        let forward_a = node_a.forward_direction();
        node_a.target_velocity = forward_a * 500.0 + Vec3::new(0.0, 0.0, 150.0); // Slight upward pop

        let mut node_b = TeleporterNode::new(
            next_id_start + 1,
            format!("Teleporter_{}", tag_b),
            &tag_b,
            &tag_a,
            pos_b,
        );
        node_b.rotation.y = yaw_b;
        node_b.b_changes_velocity = true;
        let forward_b = node_b.forward_direction();
        node_b.target_velocity = forward_b * 500.0 + Vec3::new(0.0, 0.0, 150.0);

        (node_a, node_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teleporter_forward_direction() {
        let mut node = TeleporterNode::new(0, "TestTele", "TagA", "TagB", Vec3::ZERO);
        node.rotation.y = 0.0; // Along +X
        let fwd0 = node.forward_direction();
        assert!((fwd0.x - 1.0).abs() < 1e-4);
        assert!(fwd0.y.abs() < 1e-4);

        node.rotation.y = 90.0; // Along +Y
        let fwd90 = node.forward_direction();
        assert!(fwd90.x.abs() < 1e-4);
        assert!((fwd90.y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_network_graph_resolution_and_broken_urls() {
        let mut nodes = Vec::new();
        // Node 0 -> Node 1 (OK)
        nodes.push(TeleporterNode::new(0, "TeleA", "TagA", "TagB", Vec3::new(0.0, 0.0, 0.0)));
        // Node 1 -> Node 0 (OK, Bidirectional)
        nodes.push(TeleporterNode::new(1, "TeleB", "TagB", "TagA", Vec3::new(1000.0, 0.0, 0.0)));
        // Node 2 -> Broken Tag (Error)
        nodes.push(TeleporterNode::new(2, "TeleBroken", "TagC", "DoesNotExist", Vec3::new(500.0, 500.0, 0.0)));

        let graph = TeleportNetworkGraph::build(nodes, &[]);
        assert_eq!(graph.links.len(), 2);
        assert!(graph.links[0].is_bidirectional);
        assert!(graph.links[1].is_bidirectional);

        let broken_errors: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.category == TeleportDiagnosticCategory::BrokenUrl)
            .collect();
        assert_eq!(broken_errors.len(), 1);
        assert_eq!(broken_errors[0].node_id, 2);
    }

    #[test]
    fn test_infinite_loop_and_telefrag_hazard_detection() {
        let mut nodes = Vec::new();
        // Two teleporters 50 uu apart linking to each other -> Infinite loop & overlap hazard!
        nodes.push(TeleporterNode::new(0, "TrapA", "LoopA", "LoopB", Vec3::new(100.0, 100.0, 0.0)));
        nodes.push(TeleporterNode::new(1, "TrapB", "LoopB", "LoopA", Vec3::new(130.0, 100.0, 0.0)));

        let player_starts = vec![Vec3::new(135.0, 100.0, 0.0)]; // Very close to TrapB

        let graph = TeleportNetworkGraph::build(nodes, &player_starts);

        let loop_hazards: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.category == TeleportDiagnosticCategory::InfiniteLoopHazard)
            .collect();
        assert!(!loop_hazards.is_empty());

        let telefrags: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.category == TeleportDiagnosticCategory::TelefragHazard)
            .collect();
        assert!(!telefrags.is_empty());
    }

    #[test]
    fn test_generate_paired_link() {
        let (node_a, node_b) = TeleportNetworkGraph::generate_paired_link(
            Vec3::new(0.0, 0.0, 0.0),
            0.0,
            Vec3::new(2000.0, 1000.0, 200.0),
            180.0,
            "BaseHub",
            10,
        );

        assert_eq!(node_a.id, 10);
        assert_eq!(node_b.id, 11);
        assert_eq!(node_a.tag, "BaseHub_PortalA");
        assert_eq!(node_a.url, "BaseHub_PortalB");
        assert_eq!(node_b.tag, "BaseHub_PortalB");
        assert_eq!(node_b.url, "BaseHub_PortalA");
        assert!(node_a.b_changes_velocity);
        assert!(node_b.b_changes_velocity);
    }
}
