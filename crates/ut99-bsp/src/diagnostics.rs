use crate::brush::Brush;
use crate::navigation::NavGraph;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapIssue {
    pub severity: IssueSeverity,
    pub category: String, // "Geometry", "Texture", "Actor", "Lighting", "Navigation"
    pub description: String,
    pub object_name: String,
    pub location: Option<Vec3>,
    pub auto_fixable: bool,
}

/// Pre-Build CSG Solidity & Convexity Diagnostics for the Red Builder Brush.
#[derive(Debug, Clone, PartialEq)]
pub struct BuilderBrushDiagnostics {
    pub is_convex: bool,
    pub is_watertight: bool,
    pub has_degenerate_polys: bool,
    pub reflex_vertex_count: usize,
    pub coplanar_split_risk: bool,
    pub summary_message: String,
}

/// Level Doctor: Performs comprehensive diagnostic auditing of the map.
pub struct LevelDoctor;

impl LevelDoctor {
    /// Pre-build CSG safety audit evaluating Red Builder Brush convexity, watertightness,
    /// degenerate faces, and coplanar split risk in real time before Add/Subtract CSG.
    pub fn audit_builder_brush(brush: &Brush) -> BuilderBrushDiagnostics {
        let world_polys = brush.to_world_polys();
        if world_polys.is_empty() {
            return BuilderBrushDiagnostics {
                is_convex: true,
                is_watertight: false,
                has_degenerate_polys: false,
                reflex_vertex_count: 0,
                coplanar_split_risk: false,
                summary_message: "Builder Brush has 0 polygons.".to_string(),
            };
        }

        let mut degenerate = false;
        for poly in &world_polys {
            if poly.vertices.len() < 3 {
                degenerate = true;
                break;
            }
            let cross = (poly.vertices[1] - poly.vertices[0]).cross(poly.vertices[2] - poly.vertices[0]);
            if cross.length_squared() < 1e-4 {
                degenerate = true;
                break;
            }
        }

        let eps = 0.5f32;
        let mut reflex_count = 0;
        for f in &world_polys {
            for other in &world_polys {
                for &v in &other.vertices {
                    if f.plane.dist_to_point(v) > eps {
                        reflex_count += 1;
                    }
                }
            }
        }
        let is_convex = reflex_count == 0;

        // Check watertightness via manifold edge matching
        let mut edge_counts: std::collections::HashMap<((i32, i32, i32), (i32, i32, i32)), usize> = std::collections::HashMap::new();
        let quant = |v: Vec3| -> (i32, i32, i32) {
            ((v.x * 10.0).round() as i32, (v.y * 10.0).round() as i32, (v.z * 10.0).round() as i32)
        };
        for poly in &world_polys {
            let n = poly.vertices.len();
            for i in 0..n {
                let q0 = quant(poly.vertices[i]);
                let q1 = quant(poly.vertices[(i + 1) % n]);
                if q0 == q1 {
                    continue;
                }
                let edge = if q0 < q1 { (q0, q1) } else { (q1, q0) };
                *edge_counts.entry(edge).or_insert(0) += 1;
            }
        }
        let is_watertight = !edge_counts.is_empty() && edge_counts.values().all(|&c| c == 2);

        let coplanar_split_risk = !is_convex || degenerate;

        let summary_message = if degenerate {
            "⚠️ Degenerate faces detected in Builder Brush (zero-area or <3 vertices).".to_string()
        } else if !is_convex {
            format!("⚠️ Non-convex Builder Brush ({} reflex violations). Risk of BSP holes and coplanar cuts if added/subtracted directly. Decompose or use Intersect/Deintersect.", reflex_count)
        } else if !is_watertight {
            "⚠️ Builder Brush is not watertight (open boundary edges detected).".to_string()
        } else {
            "✅ Builder Brush is strictly convex and watertight (Safe for CSG).".to_string()
        };

        BuilderBrushDiagnostics {
            is_convex,
            is_watertight,
            has_degenerate_polys: degenerate,
            reflex_vertex_count: reflex_count,
            coplanar_split_risk,
            summary_message,
        }
    }
    /// Audits all brushes for geometry, bounds, and texture defects
    pub fn audit_brushes(brushes: &[Brush]) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        for (b_idx, brush) in brushes.iter().enumerate() {
            let brush_label = if brush.name.is_empty() {
                format!("Brush_{}", b_idx)
            } else {
                brush.name.clone()
            };

            if brush.polys.is_empty() {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "Geometry".to_string(),
                    description: format!("Brush '{}' contains 0 polygons", brush_label),
                    object_name: brush_label.clone(),
                    location: Some(brush.location),
                    auto_fixable: false,
                });
                continue;
            }

            for (p_idx, poly) in brush.polys.iter().enumerate() {
                // 1. Degenerate vertex count
                if poly.vertices.len() < 3 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Error,
                        category: "Geometry".to_string(),
                        description: format!("Polygon #{} in '{}' has only {} vertices (< 3)", p_idx, brush_label, poly.vertices.len()),
                        object_name: brush_label.clone(),
                        location: poly.vertices.first().copied().or(Some(brush.location)),
                        auto_fixable: true,
                    });
                    continue;
                }

                // 2. Degenerate area
                let area = compute_poly_area(&poly.vertices);
                if area < 0.1 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Geometry".to_string(),
                        description: format!("Polygon #{} in '{}' has near-zero surface area ({:.2} sq units)", p_idx, brush_label, area),
                        object_name: brush_label.clone(),
                        location: Some(poly.vertices[0]),
                        auto_fixable: true,
                    });
                }

                // 3. Out-of-bounds coordinates (Unreal safe limit is +/- 65,536)
                for &v in &poly.vertices {
                    let world_v = v + brush.location;
                    if world_v.x.abs() > 65000.0 || world_v.y.abs() > 65000.0 || world_v.z.abs() > 65000.0 {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Error,
                            category: "Geometry".to_string(),
                            description: format!("Polygon vertex in '{}' exceeds engine grid bounds ({:.0}, {:.0}, {:.0})", brush_label, world_v.x, world_v.y, world_v.z),
                            object_name: brush_label.clone(),
                            location: Some(world_v),
                            auto_fixable: false,
                        });
                        break;
                    }
                }

                // 4. Missing / Empty texture
                if poly.texture.trim().is_empty() {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Texture".to_string(),
                        description: format!("Polygon #{} in '{}' has no texture assigned (Null surface)", p_idx, brush_label),
                        object_name: brush_label.clone(),
                        location: Some(poly.vertices[0]),
                        auto_fixable: true,
                    });
                }

                // 5. Non-planar polygon check (Deliverable E.6 Zero-HOM Pre-Build Validator)
                if poly.vertices.len() >= 4 && poly.plane.normal.length_squared() > 0.5 {
                    let mut max_dev = 0.0_f32;
                    for &v in &poly.vertices {
                        let dev = (v.dot(poly.plane.normal) - poly.plane.dist).abs();
                        if dev > max_dev {
                            max_dev = dev;
                        }
                    }
                    if max_dev > 0.05 {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Warning,
                            category: "Geometry".to_string(),
                            description: format!(
                                "Polygon #{} in '{}' is non-planar (deviation: {:.2} uu) - risk of BSP hole / HOM",
                                p_idx, brush_label, max_dev
                            ),
                            object_name: brush_label.clone(),
                            location: Some(poly.vertices[0] + brush.location),
                            auto_fixable: true,
                        });
                    }
                }
            }

            // 6. Semi-Solid intersecting Portal or subsequent Subtract brush (Deliverable E.6 Zero-HOM check)
            let is_semisolid = brush.csg_oper == crate::CsgOperation::Add
                && brush.polys.iter().any(|p| (p.flags & (crate::PF_SEMISOLID | crate::PF_LEGACY_SEMISOLID)) != 0);
            if is_semisolid {
                let (ss_min, ss_max) = brush.world_bounds();
                for (other_idx, other) in brushes.iter().enumerate() {
                    if other_idx == b_idx {
                        continue;
                    }
                    let (o_min, o_max) = other.world_bounds();
                    let overlaps = ss_min.x < o_max.x
                        && ss_max.x > o_min.x
                        && ss_min.y < o_max.y
                        && ss_max.y > o_min.y
                        && ss_min.z < o_max.z
                        && ss_max.z > o_min.z;
                    if !overlaps {
                        continue;
                    }
                    let has_portal = other.polys.iter().any(|p| (p.flags & crate::PF_PORTAL) != 0);
                    if has_portal {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Error,
                            category: "Geometry".to_string(),
                            description: format!(
                                "Semi-Solid brush '{}' intersects Zone Portal brush '{}' - guaranteed BSP HOM / zone leak!",
                                brush_label, other.name
                            ),
                            object_name: brush_label.clone(),
                            location: Some(brush.location),
                            auto_fixable: false,
                        });
                    } else if other.csg_oper == crate::CsgOperation::Subtract && other_idx > b_idx {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Warning,
                            category: "Geometry".to_string(),
                            description: format!(
                                "Semi-Solid brush '{}' is cut by later Subtract brush '{}' - risk of BSP HOM / invisible collision!",
                                brush_label, other.name
                            ),
                            object_name: brush_label.clone(),
                            location: Some(brush.location),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }

        issues
    }

    /// Audits actor tags and events for unlinked triggers and movers
    pub fn audit_actor_linkages(actors: &[(String, String, String, Vec3)]) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        let mut tag_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for (_name, _class_name, tag_or_event, _loc) in actors {
            let trimmed = tag_or_event.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("None") {
                *tag_counts.entry(trimmed.to_ascii_lowercase()).or_insert(0) += 1;
            }
        }

        for (name, class_name, tag_or_event, loc) in actors {
            let cls_low = class_name.to_ascii_lowercase();
            let trimmed = tag_or_event.trim();
            let is_empty = trimmed.is_empty() || trimmed.eq_ignore_ascii_case("None");
            if cls_low.contains("trigger") {
                if is_empty {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "ActorLinkage".to_string(),
                        description: format!(
                            "Trigger '{}' ({}) has no Event/Tag linkage assigned and will not activate any target.",
                            name, class_name
                        ),
                        object_name: name.clone(),
                        location: Some(*loc),
                        auto_fixable: false,
                    });
                } else if tag_counts.get(&trimmed.to_ascii_lowercase()).copied().unwrap_or(0) < 2 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "ActorLinkage".to_string(),
                        description: format!(
                            "Trigger '{}' ({}) references Event/Tag '{}' which is not matched by any target Actor.",
                            name, class_name, trimmed
                        ),
                        object_name: name.clone(),
                        location: Some(*loc),
                        auto_fixable: false,
                    });
                }
            }
        }

        issues
    }

    /// Audits bidirectional `Event -> Tag` actor linkages across `(name, class_name, tag, event, location)` records.
    pub fn audit_actor_event_tag_links(
        actors: &[(String, String, String, String, Vec3)],
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        let mut all_tags = std::collections::HashSet::new();
        let mut all_events = std::collections::HashSet::new();

        for (_name, class_name, tag, event, _loc) in actors {
            let t = tag.trim();
            if !t.is_empty() && !t.eq_ignore_ascii_case("None") && !t.eq_ignore_ascii_case(class_name) {
                all_tags.insert(t.to_ascii_lowercase());
            }
            let e = event.trim();
            if !e.is_empty() && !e.eq_ignore_ascii_case("None") {
                all_events.insert(e.to_ascii_lowercase());
            }
        }

        for (name, class_name, tag, event, loc) in actors {
            let cls_low = class_name.to_ascii_lowercase();
            let t = tag.trim();
            let e = event.trim();
            let has_event = !e.is_empty() && !e.eq_ignore_ascii_case("None");
            let has_custom_tag =
                !t.is_empty() && !t.eq_ignore_ascii_case("None") && !t.eq_ignore_ascii_case(class_name);

            if has_event && !all_tags.contains(&e.to_ascii_lowercase()) {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "ActorLinkage".to_string(),
                    description: format!(
                        "Actor '{}' ({}) fires Event '{}' which matches no Actor Tag in the level (orphaned Event).",
                        name, class_name, e
                    ),
                    object_name: name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            } else if cls_low.contains("trigger") && !cls_low.contains("triggered") && !has_event {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "ActorLinkage".to_string(),
                    description: format!(
                        "Trigger '{}' ({}) has no Event assigned and will do nothing when triggered.",
                        name, class_name
                    ),
                    object_name: name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            }

            if (cls_low.contains("specialevent")
                || cls_low.contains("dispatcher")
                || cls_low.contains("counter")
                || cls_low.contains("triggered"))
                && has_custom_tag
                && !all_events.contains(&t.to_ascii_lowercase())
            {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "ActorLinkage".to_string(),
                    description: format!(
                        "Triggered actor '{}' ({}) waits for Tag '{}', but no Actor fires matching Event '{}'.",
                        name, class_name, t, t
                    ),
                    object_name: name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            }
        }

        issues
    }

    /// Audits AI Navigation graph for orphaned or stranded nodes
    pub fn audit_navigation_graph(graph: &NavGraph) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        let mut incoming_counts = vec![0usize; graph.nodes.len()];
        let mut outgoing_counts = vec![0usize; graph.nodes.len()];

        for spec in &graph.reach_specs {
            if spec.from < outgoing_counts.len() {
                outgoing_counts[spec.from] += 1;
            }
            if spec.to < incoming_counts.len() {
                incoming_counts[spec.to] += 1;
            }
        }

        for (idx, node) in graph.nodes.iter().enumerate() {
            let inc = incoming_counts[idx];
            let out = outgoing_counts[idx];

            if inc == 0 && out == 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "Navigation".to_string(),
                    description: format!("Navigation node '{}' ({}) is completely isolated (0 connections)", node.name, node.node_type),
                    object_name: node.name.clone(),
                    location: Some(node.location),
                    auto_fixable: false,
                });
            } else if inc == 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "Navigation".to_string(),
                    description: format!("Navigation node '{}' has no incoming paths (bots cannot navigate TO here)", node.name),
                    object_name: node.name.clone(),
                    location: Some(node.location),
                    auto_fixable: false,
                });
            } else if out == 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "Navigation".to_string(),
                    description: format!("Navigation node '{}' is a dead-end with no outgoing paths", node.name),
                    object_name: node.name.clone(),
                    location: Some(node.location),
                    auto_fixable: false,
                });
            }
        }

        // Detect groups that have internal links but no link at all to the rest
        // of the graph. This is weak connectivity only: directed reachability
        // and the engine's actual movement rules require additional checks.
        let mut neighbours = vec![Vec::new(); graph.nodes.len()];
        for spec in &graph.reach_specs {
            if spec.from < neighbours.len() && spec.to < neighbours.len() {
                neighbours[spec.from].push(spec.to);
                neighbours[spec.to].push(spec.from);
            }
        }
        let mut visited = vec![false; graph.nodes.len()];
        let mut components = Vec::new();
        for start in 0..graph.nodes.len() {
            if visited[start] || neighbours[start].is_empty() { continue; }
            let mut stack = vec![start];
            visited[start] = true;
            let mut count = 0;
            while let Some(node) = stack.pop() {
                count += 1;
                for &next in &neighbours[node] {
                    if !visited[next] {
                        visited[next] = true;
                        stack.push(next);
                    }
                }
            }
            components.push((start, count));
        }
        if components.len() > 1 {
            for &(start, count) in &components {
                let node = &graph.nodes[start];
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "Navigation".to_string(),
                    description: format!("Navigation network has {} disconnected groups; this group has {} nodes", components.len(), count),
                    object_name: node.name.clone(),
                    location: Some(node.location),
                    auto_fixable: false,
                });
            }
        }

        // Use the graph's directed edges, not straight-line distance or the
        // undirected component check above. This reports a marker only when it
        // is unreachable from every spawn represented in this graph.
        let mut outgoing = vec![Vec::new(); graph.nodes.len()];
        for spec in &graph.reach_specs {
            if spec.from < outgoing.len() && spec.to < outgoing.len() {
                outgoing[spec.from].push(spec.to);
            }
        }
        let mut reached = vec![false; graph.nodes.len()];
        let mut stack = Vec::new();
        for (idx, node) in graph.nodes.iter().enumerate() {
            if node.node_type.rsplit('.').next().unwrap_or("").eq_ignore_ascii_case("PlayerStart") {
                reached[idx] = true;
                stack.push(idx);
            }
        }
        let has_spawns = !stack.is_empty();
        while let Some(node) = stack.pop() {
            for &next in &outgoing[node] {
                if !reached[next] {
                    reached[next] = true;
                    stack.push(next);
                }
            }
        }
        if has_spawns {
            for (idx, node) in graph.nodes.iter().enumerate() {
                if !reached[idx]
                    && node.node_type.rsplit('.').next().unwrap_or("").eq_ignore_ascii_case("InventorySpot")
                {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Navigation".to_string(),
                        description: format!("Inventory marker '{}' is unreachable from every PlayerStart in the estimated directed navigation graph", node.name),
                        object_name: node.name.clone(),
                        location: Some(node.location),
                        auto_fixable: false,
                    });
                }
            }
        }

        issues
    }

    /// Auto-fixes brush issues: strips degenerate/zero-area polygons and applies default texture
    pub fn autofix_brush(brush: &mut Brush, default_texture: &str) -> usize {
        let mut fixes = 0;

        brush.polys.retain(|poly| {
            if poly.vertices.len() < 3 {
                fixes += 1;
                return false;
            }
            if compute_poly_area(&poly.vertices) < 0.1 {
                fixes += 1;
                return false;
            }
            true
        });

        for poly in &mut brush.polys {
            if poly.texture.trim().is_empty() {
                poly.texture = default_texture.to_string();
                fixes += 1;
            }
            if poly.vertices.len() >= 4 && poly.plane.normal.length_squared() > 0.5 {
                let n = poly.plane.normal.normalize_or_zero();
                let mut fixed_plane = false;
                for v in &mut poly.vertices {
                    let dev = v.dot(n) - poly.plane.dist;
                    if dev.abs() > 0.05 {
                        *v -= n * dev;
                        fixed_plane = true;
                    }
                }
                if fixed_plane {
                    fixes += 1;
                }
            }
        }

        fixes
    }

    /// Audits actor placements (player starts, lights, pickups) for gameplay readiness
    pub fn audit_actor_records(actors: &[(String, String, Vec3, u8, u8)]) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        let player_starts: Vec<_> = actors.iter().filter(|(_, c, ..)| c.contains("PlayerStart")).collect();
        if player_starts.is_empty() {
            issues.push(MapIssue {
                severity: IssueSeverity::Error,
                category: "Actor".to_string(),
                description: "Level contains 0 PlayerStart actors! Players cannot spawn in the level.".to_string(),
                object_name: "World".to_string(),
                location: None,
                auto_fixable: false,
            });
        }

        // Check overlapping player starts
        for i in 0..player_starts.len() {
            for j in (i + 1)..player_starts.len() {
                if player_starts[i].2.distance(player_starts[j].2) < 32.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Actor".to_string(),
                        description: format!("PlayerStart '{}' overlaps '{}' at distance < 32u (telefrag risk).", player_starts[i].0, player_starts[j].0),
                        object_name: player_starts[i].0.clone(),
                        location: Some(player_starts[i].2),
                        auto_fixable: false,
                    });
                }
            }
        }

        // Check lights with 0 radius or brightness
        for (name, class, loc, brightness, radius) in actors {
            if class.contains("Light") && (*brightness == 0 || *radius == 0) {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "Lighting".to_string(),
                    description: format!("Light actor '{}' has 0 brightness or 0 radius (inactive light).", name),
                    object_name: name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            }
        }

        issues
    }

    /// Audits all Zone Portals (polygons with PF_PORTAL) for integrity, non-planarity, and boundary closure
    pub fn audit_zone_portals(brushes: &[Brush]) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        let mut portal_count = 0;

        for (b_idx, brush) in brushes.iter().enumerate() {
            let brush_label = if brush.name.is_empty() {
                format!("Brush_{}", b_idx)
            } else {
                brush.name.clone()
            };

            for (p_idx, poly) in brush.polys.iter().enumerate() {
                if (poly.flags & crate::PF_PORTAL) != 0 {
                    portal_count += 1;
                    let poly_loc = poly.vertices.first().copied().unwrap_or(Vec3::ZERO) + brush.location;

                    // 1. Minimum vertices
                    if poly.vertices.len() < 3 {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Error,
                            category: "ZonePortals".to_string(),
                            description: format!("Zone Portal #{} in '{}' has invalid vertex count (< 3)", p_idx, brush_label),
                            object_name: brush_label.clone(),
                            location: Some(poly_loc),
                            auto_fixable: false,
                        });
                        continue;
                    }

                    // 2. Non-zero area
                    let area = compute_poly_area(&poly.vertices);
                    if area < 1.0 {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Error,
                            category: "ZonePortals".to_string(),
                            description: format!("Zone Portal #{} in '{}' has near-zero area ({:.1} sq units) - will fail to divide zones", p_idx, brush_label, area),
                            object_name: brush_label.clone(),
                            location: Some(poly_loc),
                            auto_fixable: false,
                        });
                    }

                    // 3. Planarity check
                    let normal = poly.plane.normal;
                    let dist = poly.plane.dist;
                    let mut max_plane_dist: f32 = 0.0;
                    for &v in &poly.vertices {
                        let d = (v.dot(normal) - dist).abs();
                        if d > max_plane_dist {
                            max_plane_dist = d;
                        }
                    }
                    if max_plane_dist > 0.5 {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Warning,
                            category: "ZonePortals".to_string(),
                            description: format!("Zone Portal #{} in '{}' is non-planar (deviation: {:.2} uu) - risk of BSP zone leaks", p_idx, brush_label, max_plane_dist),
                            object_name: brush_label.clone(),
                            location: Some(poly_loc),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }

        if portal_count > 0 {
            issues.push(MapIssue {
                severity: IssueSeverity::Info,
                category: "ZonePortals".to_string(),
                description: format!("Detected {} zone portal polygon(s) active in the map.", portal_count),
                object_name: "World".to_string(),
                location: None,
                auto_fixable: false,
            });
        }

        issues
    }

    /// Audits map architecture and navigation for UT99 tournament gameplay metrics:
    /// - Minimum corridor width (38 uu; player diameter is 34 uu)
    /// - Minimum vertical door/ceiling height (80 uu; standing player height is 78 uu)
    /// - Lethal/high drop hazards (falls > 400 uu damage; > 800 uu fatal)
    /// - Embedded or snagged actors inside solid brush geometry
    pub fn audit_gameplay_clearance(
        brushes: &[Brush],
        nav_graph: &NavGraph,
        actors: &[(String, String, Vec3)],
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        // 1. Navigation reachspec corridor width and drop height
        for spec in &nav_graph.reach_specs {
            if spec.from >= nav_graph.nodes.len() || spec.to >= nav_graph.nodes.len() {
                continue;
            }
            let from_node = &nav_graph.nodes[spec.from];
            let to_node = &nav_graph.nodes[spec.to];

            // A. Narrow corridor / bottleneck check (< 38 uu)
            let is_inventory = from_node.node_type.to_ascii_lowercase().contains("inventory")
                || to_node.node_type.to_ascii_lowercase().contains("inventory")
                || from_node.name.to_ascii_lowercase().contains("inventory")
                || to_node.name.to_ascii_lowercase().contains("inventory");

            if !is_inventory && spec.distance < 38.0 && spec.distance > 0.1 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "GameplayClearance".to_string(),
                    description: format!(
                        "Corridor bottleneck: Reachability path between '{}' and '{}' has clearance/length {:.1} uu (< 38 uu, UT99 player diameter is 34 uu) - risk of player/bot jams.",
                        from_node.name, to_node.name, spec.distance
                    ),
                    object_name: from_node.name.clone(),
                    location: Some(from_node.location),
                    auto_fixable: false,
                });
            }

            // B. High drop & lethal fall hazards (skip non-physical locomotion like teleporters or lifts)
            let is_special = (spec.reach_flags & crate::navigation::R_SPECIAL) != 0
                || from_node.node_type.to_ascii_lowercase().contains("teleporter")
                || to_node.node_type.to_ascii_lowercase().contains("teleporter")
                || from_node.node_type.to_ascii_lowercase().contains("lift")
                || to_node.node_type.to_ascii_lowercase().contains("lift");

            if !is_special {
                let drop_z = from_node.location.z - to_node.location.z;
                if drop_z >= 800.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Error,
                        category: "GameplayHazard".to_string(),
                        description: format!(
                            "Lethal drop hazard: Fall of {:.0} uu from '{}' to '{}' exceeds fatal impact threshold (800 uu) without jump boots or water dampener.",
                            drop_z, from_node.name, to_node.name
                        ),
                        object_name: from_node.name.clone(),
                        location: Some(from_node.location),
                        auto_fixable: false,
                    });
                } else if drop_z >= 400.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "GameplayHazard".to_string(),
                        description: format!(
                            "High drop hazard: Fall of {:.0} uu from '{}' to '{}' inflicts significant falling damage (> 400 uu).",
                            drop_z, from_node.name, to_node.name
                        ),
                        object_name: from_node.name.clone(),
                        location: Some(from_node.location),
                        auto_fixable: false,
                    });
                }
            }

            // C. Low ceiling / doorway clearance check (< 80 uu)
            let mid_pt = (from_node.location + to_node.location) * 0.5;
            for brush in brushes {
                let b_low = brush.name.to_ascii_lowercase();
                if b_low == "worldshell" || b_low == "world_shell" || b_low.contains("rib") || b_low.contains("baseboard") {
                    continue;
                }
                let is_solid = (brush.csg_oper == crate::CsgOperation::Add
                    || brush.polys.iter().any(|p| (p.flags & (crate::poly::PF_SEMISOLID | crate::poly::PF_LEGACY_SEMISOLID)) != 0))
                    && !brush.polys.iter().any(|p| (p.flags & crate::poly::PF_NOT_SOLID) != 0);
                if is_solid {
                    let (min_b, max_b) = brush.world_bounds();
                    if mid_pt.x >= min_b.x && mid_pt.x <= max_b.x && mid_pt.y >= min_b.y && mid_pt.y <= max_b.y {
                        let clearance = min_b.z - mid_pt.z;
                        if clearance > 0.0 && clearance < 80.0 {
                            issues.push(MapIssue {
                                severity: IssueSeverity::Warning,
                                category: "GameplayClearance".to_string(),
                                description: format!(
                                    "Low doorway/ceiling clearance: Overhead brush '{}' is only {:.1} uu above path between '{}' and '{}' (< 80 uu, standing player is 78 uu).",
                                    brush.name, clearance, from_node.name, to_node.name
                                ),
                                object_name: brush.name.clone(),
                                location: Some(mid_pt),
                                auto_fixable: false,
                            });
                        }
                    }
                }
            }
        }

        // 2. Embedded / Solid Geometry Snags for critical actors
        for (act_name, act_class, loc) in actors {
            let is_gameplay_actor = act_class.contains("PlayerStart")
                || act_class.contains("PathNode")
                || act_class.contains("InventorySpot")
                || act_class.contains("FlagBase")
                || act_class.contains("ControlPoint")
                || act_class.contains("FortStandard")
                || act_class.contains("Weapon")
                || act_class.contains("Ammo")
                || act_class.contains("Pickup");

            if !is_gameplay_actor {
                continue;
            }

            // CTF FlagBase specific clearance: ensure the spawned CTFFlag cylinder (R=48, H=30)
            // does not encroach any solid detail brush (such as backplates, columns, or overhead roofs)
            if act_class.contains("FlagBase") {
                for brush in brushes {
                    let is_solid = (brush.csg_oper == crate::CsgOperation::Add
                        || brush.polys.iter().any(|p| (p.flags & crate::poly::PF_SEMISOLID) != 0))
                        && !brush.polys.iter().any(|p| (p.flags & crate::poly::PF_NOT_SOLID) != 0);
                    // Exclude the supporting base floor pedestals underneath the flag
                    let is_pedestal_base = brush.name.contains("CtfDaisBase")
                        || brush.name.contains("CtfDaisTop")
                        || brush.name.contains("CtfShrineBase")
                        || brush.name.contains("CtfBunkerBase")
                        || brush.name.contains("CtfCanopyBase");
                    if is_solid && !is_pedestal_base {
                        let (min_b, max_b) = brush.world_bounds();
                        let cyl_r = 48.0;
                        let cyl_h = 30.0;
                        if loc.x + cyl_r > min_b.x && loc.x - cyl_r < max_b.x
                            && loc.y + cyl_r > min_b.y && loc.y - cyl_r < max_b.y
                            && loc.z + cyl_h > min_b.z && loc.z - cyl_h < max_b.z
                        {
                            issues.push(MapIssue {
                                severity: IssueSeverity::Error,
                                category: "FlagClearance".to_string(),
                                description: format!(
                                    "FlagBase '{}' has its flag collision cylinder (R=48, H=30) encroaching solid brush '{}' - will cause Spawn(class'RedFlag'/'CTFFlag') to fail with Accessed None in UT99!",
                                    act_name, brush.name
                                ),
                                object_name: act_name.clone(),
                                location: Some(*loc),
                                auto_fixable: false,
                            });
                        }
                    }
                }
            }

            // PlayerStart room cavity clearance: verify PlayerStart is located inside an authentic
            // subtracted room/corridor cavity and not in solid bedrock/void outside the level.
            if act_class.contains("PlayerStart") {
                let sub_brushes: Vec<&Brush> = brushes
                    .iter()
                    .filter(|b| {
                        b.csg_oper == crate::CsgOperation::Subtract
                            && !b.name.eq_ignore_ascii_case("worldshell")
                            && !b.name.eq_ignore_ascii_case("world_shell")
                    })
                    .collect();
                if !sub_brushes.is_empty() {
                    let in_any_cavity = sub_brushes.iter().any(|b| {
                        let (min_b, max_b) = b.world_bounds();
                        loc.x >= min_b.x - 4.0 && loc.x <= max_b.x + 4.0
                            && loc.y >= min_b.y - 4.0 && loc.y <= max_b.y + 4.0
                            && loc.z >= min_b.z - 4.0 && loc.z <= max_b.z + 4.0
                    });
                    if !in_any_cavity {
                        issues.push(MapIssue {
                            severity: IssueSeverity::Error,
                            category: "SpawnClearance".to_string(),
                            description: format!(
                                "Critical spawn failure: PlayerStart '{}' is placed outside all room cavities in solid bedrock/void at ({:.1}, {:.1}, {:.1}) - will cause 'Failed to spawn player actor' crash in UT99!",
                                act_name, loc.x, loc.y, loc.z
                            ),
                            object_name: act_name.clone(),
                            location: Some(*loc),
                            auto_fixable: false,
                        });
                    }
                }
            }

            for brush in brushes {
                let b_name_lower = brush.name.to_ascii_lowercase();
                if b_name_lower == "worldshell" || b_name_lower == "world_shell" {
                    continue;
                }
                let is_solid = (brush.csg_oper == crate::CsgOperation::Add
                    || brush.polys.iter().any(|p| (p.flags & (crate::poly::PF_SEMISOLID | crate::poly::PF_LEGACY_SEMISOLID)) != 0))
                    && !brush.polys.iter().any(|p| (p.flags & crate::poly::PF_NOT_SOLID) != 0);
                if is_solid {
                    let (min_b, max_b) = brush.world_bounds();
                    let margin = 4.0;
                    if loc.x >= min_b.x + margin && loc.x <= max_b.x - margin
                        && loc.y >= min_b.y + margin && loc.y <= max_b.y - margin
                        && loc.z >= min_b.z + margin && loc.z <= max_b.z - margin
                    {
                        let world_polys = brush.to_world_polys();
                        let inside_convex = !world_polys.is_empty()
                            && world_polys
                                .iter()
                                .all(|p| p.plane.dist_to_point(*loc) <= -margin);
                        if inside_convex {
                            issues.push(MapIssue {
                                severity: IssueSeverity::Error,
                                category: "ActorClearance".to_string(),
                                description: format!(
                                    "Actor '{}' ({}) is embedded completely inside solid brush '{}' - risk of collision snag or failed spawn.",
                                    act_name, act_class, brush.name
                                ),
                                object_name: act_name.clone(),
                                location: Some(*loc),
                                auto_fixable: false,
                            });
                        }
                    }
                }
            }
        }

        issues
    }

    /// Audits teleporter actors to ensure active portals are visible in-game (`VisibleTeleporter`
    /// or accompanied by a `UTTeleEffect` / `UTTeleportEffect` or portal archway).
    pub fn audit_teleporter_visibility(
        actors: &[(String, String, Vec3)],
        brushes: &[Brush],
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        for (act_name, act_class, loc) in actors {
            let short = act_class.rsplit('.').next().unwrap_or(act_class);
            if short.eq_ignore_ascii_case("Teleporter") {
                let has_nearby_effect = actors.iter().any(|(_, other_cls, other_loc)| {
                    let o_short = other_cls.rsplit('.').next().unwrap_or(other_cls).to_ascii_lowercase();
                    (o_short.contains("teleeffect") || o_short.contains("teleporteffect"))
                        && other_loc.distance(*loc) <= 96.0
                });
                let has_portal_brush = brushes.iter().any(|b| {
                    let b_low = b.name.to_ascii_lowercase();
                    (b_low.contains("tele") || b_low.contains("portal") || b_low.contains("arch"))
                        && b.location.distance(*loc) <= 160.0
                });

                if !has_nearby_effect && !has_portal_brush {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "TeleporterVisibility".to_string(),
                        description: format!(
                            "Teleporter '{}' uses base class '{}' (bHidden=True in UT99) without a VisibleTeleporter vortex or portal arch brush — portal will be invisible in-game.",
                            act_name, act_class
                        ),
                        object_name: act_name.clone(),
                        location: Some(*loc),
                        auto_fixable: true,
                    });
                }
            }
        }

        issues
    }

    /// Audits Domination ControlPoint setup (checks for required point count, unique PointNames,
    /// absence of proximity spam < 450 UU, triangle asymmetry, and player spawn standoff).
    pub fn audit_domination_setup(
        control_points: &[(String, String, Vec3)], // (name, point_name, location)
        player_starts: &[Vec3],
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        if control_points.is_empty() {
            return issues;
        }

        if control_points.len() < 2 {
            issues.push(MapIssue {
                severity: IssueSeverity::Error,
                category: "Objectives".to_string(),
                description: format!(
                    "Domination requires at least 2 ControlPoints (found {}). Tournament standard is 3.",
                    control_points.len()
                ),
                object_name: control_points[0].0.clone(),
                location: Some(control_points[0].2),
                auto_fixable: false,
            });
        }

        // 1. Unique and non-empty PointName
        let mut seen_names = std::collections::HashSet::new();
        for (actor_name, pt_name, loc) in control_points {
            if pt_name.trim().is_empty() || pt_name == "<none>" {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "Objectives".to_string(),
                    description: format!(
                        "ControlPoint '{}' has empty or missing PointName property.",
                        actor_name
                    ),
                    object_name: actor_name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            } else if !seen_names.insert(pt_name.clone()) {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "Objectives".to_string(),
                    description: format!(
                        "Duplicate ControlPoint PointName '{}' found on '{}'.",
                        pt_name, actor_name
                    ),
                    object_name: actor_name.clone(),
                    location: Some(*loc),
                    auto_fixable: false,
                });
            }
        }

        // 2. Proximity Spam check (distance < 450 UU)
        for i in 0..control_points.len() {
            for j in (i + 1)..control_points.len() {
                let dist = control_points[i].2.distance(control_points[j].2);
                if dist < 450.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Error,
                        category: "Objectives".to_string(),
                        description: format!(
                            "Critical ControlPoint proximity spam: '{}' is only {:.1} UU from '{}' (minimum 450 UU).",
                            control_points[i].1, dist, control_points[j].1
                        ),
                        object_name: control_points[i].0.clone(),
                        location: Some(control_points[i].2),
                        auto_fixable: false,
                    });
                }
            }
        }

        // 3. Triangle asymmetry check (if 3 points)
        if control_points.len() == 3 {
            let d_ab = control_points[0].2.distance(control_points[1].2);
            let d_bc = control_points[1].2.distance(control_points[2].2);
            let d_ca = control_points[2].2.distance(control_points[0].2);
            let perim = d_ab + d_bc + d_ca;
            let asymmetry = (d_ab - d_bc).abs() + (d_bc - d_ca).abs() + (d_ca - d_ab).abs();
            let asym_ratio = (asymmetry / perim.max(1.0)) * 100.0;

            if asym_ratio > 38.0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "GameplayBalance".to_string(),
                    description: format!(
                        "Severe Domination triangle asymmetry ({:.1}%). May cause degenerate 2-point camping dominance.",
                        asym_ratio
                    ),
                    object_name: control_points[0].0.clone(),
                    location: Some(control_points[0].2),
                    auto_fixable: false,
                });
            }
        }

        // 4. PlayerStart standoff check (< 360 UU)
        for ps_loc in player_starts {
            for (actor_name, pt_name, cp_loc) in control_points {
                let dist = ps_loc.distance(*cp_loc);
                if dist < 360.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Spawns".to_string(),
                        description: format!(
                            "PlayerStart is only {:.1} UU from ControlPoint '{}' ('{}'). Standoff distance should be >= 400 UU.",
                            dist, pt_name, actor_name
                        ),
                        object_name: actor_name.clone(),
                        location: Some(*ps_loc),
                        auto_fixable: false,
                    });
                }
            }
        }

        issues
    }

    /// Validates Capture The Flag (CTF) tournament balance, team parity, and flag base geometry.
    pub fn audit_ctf_setup(
        flag_bases: &[(String, u8, Vec3)], // (actor_name, team, location)
        player_starts: &[(String, u8, Vec3)], // (actor_name, team, location)
        alternate_paths: &[(u8, f32, bool, Vec3)], // (team, weight, return_only, location)
        defense_points: &[(String, u8, bool, Vec3)], // (actor_name, team, is_sniping, location)
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        if flag_bases.is_empty() {
            return issues;
        }

        // 1. Exactly 1 Red FlagBase (Team 0) and 1 Blue FlagBase (Team 1)
        let red_flags: Vec<_> = flag_bases.iter().filter(|(_, t, _)| *t == 0).collect();
        let blue_flags: Vec<_> = flag_bases.iter().filter(|(_, t, _)| *t == 1).collect();

        if red_flags.is_empty() {
            issues.push(MapIssue {
                severity: IssueSeverity::Error,
                category: "Objectives".to_string(),
                description: "Missing Red FlagBase (Team = 0). CTF requires a Red team flag base.".to_string(),
                object_name: "FlagBase".to_string(),
                location: None,
                auto_fixable: false,
            });
        }
        if blue_flags.is_empty() {
            issues.push(MapIssue {
                severity: IssueSeverity::Error,
                category: "Objectives".to_string(),
                description: "Missing Blue FlagBase (Team = 1). CTF requires a Blue team flag base.".to_string(),
                object_name: "FlagBase".to_string(),
                location: None,
                auto_fixable: false,
            });
        }
        if red_flags.len() > 1 {
            issues.push(MapIssue {
                severity: IssueSeverity::Warning,
                category: "Objectives".to_string(),
                description: format!("Multiple Red FlagBases found ({}). Standard CTF requires exactly 1 per team.", red_flags.len()),
                object_name: red_flags[0].0.clone(),
                location: Some(red_flags[0].2),
                auto_fixable: false,
            });
        }
        if blue_flags.len() > 1 {
            issues.push(MapIssue {
                severity: IssueSeverity::Warning,
                category: "Objectives".to_string(),
                description: format!("Multiple Blue FlagBases found ({}). Standard CTF requires exactly 1 per team.", blue_flags.len()),
                object_name: blue_flags[0].0.clone(),
                location: Some(blue_flags[0].2),
                auto_fixable: false,
            });
        }

        // 2. Flag-to-Flag Minimum Distance
        if let (Some(r), Some(b)) = (red_flags.first(), blue_flags.first()) {
            let dist = r.2.distance(b.2);
            if dist < 2200.0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameplayBalance".to_string(),
                    description: format!(
                        "FlagBases are critically close: only {:.1} UU apart (minimum 2200 UU to prevent instant flag capture spam).",
                        dist
                    ),
                    object_name: r.0.clone(),
                    location: Some(r.2),
                    auto_fixable: false,
                });
            }
        }

        // 3. PlayerStart Team Counts & Parity
        let red_spawns = player_starts.iter().filter(|(_, t, _)| *t == 0).count();
        let blue_spawns = player_starts.iter().filter(|(_, t, _)| *t == 1).count();

        if red_spawns < 4 || blue_spawns < 4 {
            issues.push(MapIssue {
                severity: IssueSeverity::Error,
                category: "Spawns".to_string(),
                description: format!(
                    "Insufficient PlayerStarts: found {} Red and {} Blue spawns (minimum 4 per team).",
                    red_spawns, blue_spawns
                ),
                object_name: "PlayerStart".to_string(),
                location: None,
                auto_fixable: false,
            });
        } else if (red_spawns as i32 - blue_spawns as i32).abs() > 1 {
            issues.push(MapIssue {
                severity: IssueSeverity::Warning,
                category: "GameplayBalance".to_string(),
                description: format!(
                    "Asymmetric PlayerStart count between teams: {} Red vs {} Blue.",
                    red_spawns, blue_spawns
                ),
                object_name: "PlayerStart".to_string(),
                location: None,
                auto_fixable: false,
            });
        }

        // 4. Standoff Proximity Checks
        for (ps_name, ps_team, ps_loc) in player_starts {
            for (fb_name, fb_team, fb_loc) in flag_bases {
                let dist = ps_loc.distance(*fb_loc);
                if ps_team == fb_team && dist < 320.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Spawns".to_string(),
                        description: format!(
                            "PlayerStart '{}' is only {:.1} UU from own FlagBase '{}' (standoff distance should be >= 350 UU).",
                            ps_name, dist, fb_name
                        ),
                        object_name: ps_name.clone(),
                        location: Some(*ps_loc),
                        auto_fixable: false,
                    });
                } else if ps_team != fb_team && dist < 1800.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "Spawns".to_string(),
                        description: format!(
                            "PlayerStart '{}' (Team {}) is only {:.1} UU from enemy FlagBase '{}' (standoff distance should be >= 2000 UU).",
                            ps_name, ps_team, dist, fb_name
                        ),
                        object_name: ps_name.clone(),
                        location: Some(*ps_loc),
                        auto_fixable: false,
                    });
                }
            }
        }

        // 5. Bot AI Navigation & DefensePoints
        if alternate_paths.is_empty() {
            issues.push(MapIssue {
                severity: IssueSeverity::Warning,
                category: "BotNavigation".to_string(),
                description: "No AlternatePath actors found; bot flag carriers may take predictable single-file routes.".to_string(),
                object_name: "AlternatePath".to_string(),
                location: None,
                auto_fixable: false,
            });
        }
        if defense_points.is_empty() {
            issues.push(MapIssue {
                severity: IssueSeverity::Warning,
                category: "BotNavigation".to_string(),
                description: "No DefensePoint actors found for tactical bot base defense.".to_string(),
                object_name: "DefensePoint".to_string(),
                location: None,
                auto_fixable: false,
            });
        }

        issues
    }

    /// Audits competitive tournament playability, spawn safety, weapon proximity, and combat flow.
    pub fn audit_tournament_playability(
        player_starts: &[Vec3],
        weapons: &[(String, Vec3)], // (class_name, location)
        ammo_boxes: &[(String, Vec3)], // (class_name, location)
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();

        // 1. Telefrag Risk (< 180 UU between any two PlayerStarts)
        for i in 0..player_starts.len() {
            for j in (i + 1)..player_starts.len() {
                let dist = player_starts[i].distance(player_starts[j]);
                if dist < 180.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Error,
                        category: "Spawns".to_string(),
                        description: format!(
                            "Critical telefrag hazard: PlayerStart #{} is only {:.1} UU from PlayerStart #{} (minimum safe clearance: 200 UU).",
                            i, dist, j
                        ),
                        object_name: format!("PlayerStart_{i}"),
                        location: Some(player_starts[i]),
                        auto_fixable: true,
                    });
                }
            }
        }

        // 2. Weapon Starvation Risk (PlayerStart > 850 UU from nearest weapon)
        if !weapons.is_empty() {
            for (ps_idx, ps_loc) in player_starts.iter().enumerate() {
                let nearest_weapon_dist = weapons
                    .iter()
                    .map(|(_, w_loc)| ps_loc.distance(*w_loc))
                    .fold(f32::INFINITY, f32::min);

                if nearest_weapon_dist > 850.0 {
                    issues.push(MapIssue {
                        severity: IssueSeverity::Warning,
                        category: "GameplayBalance".to_string(),
                        description: format!(
                            "Weapon starvation risk: PlayerStart #{} is {:.1} UU from nearest weapon ({:.1}s unarmed sprint).",
                            ps_idx, nearest_weapon_dist, nearest_weapon_dist / 400.0
                        ),
                        object_name: format!("PlayerStart_{ps_idx}"),
                        location: Some(*ps_loc),
                        auto_fixable: true,
                    });
                }
            }
        }

        // 3. Orphan Weapon Risk (Weapon > 400 UU from any ammo)
        for (w_name, w_loc) in weapons {
            let short_wep = w_name.rsplit('.').next().unwrap_or(w_name);
            if short_wep.eq_ignore_ascii_case("ChainSaw")
                || short_wep.eq_ignore_ascii_case("WarHeadLauncher")
                || short_wep.eq_ignore_ascii_case("Translocator")
            {
                continue;
            }
            let has_near_ammo = ammo_boxes.iter().any(|(_, a_loc)| w_loc.distance(*a_loc) <= 400.0);
            if !has_near_ammo && !ammo_boxes.is_empty() {
                issues.push(MapIssue {
                    severity: IssueSeverity::Warning,
                    category: "GameplayBalance".to_string(),
                    description: format!(
                        "Orphan weapon warning: Weapon '{}' has no ammunition boxes within 400 UU.",
                        short_wep
                    ),
                    object_name: short_wep.to_string(),
                    location: Some(*w_loc),
                    auto_fixable: true,
                });
            }
        }

        issues
    }

    /// Audits consistency between map name prefix (e.g. "DM-", "DOM-", "CTF-", "AS-")
    /// and the actual game setup (DefaultGameType, ControlPoints, FlagBases, FortStandards).
    /// Detects cases where a map named "DM-*" contains Domination ControlPoints or specifies Botpack.Domination.
    pub fn audit_mode_prefix_coherence(
        map_name: &str,
        game_type: Option<&str>,
        control_points_count: usize,
        flag_bases_count: usize,
        fort_standards_count: usize,
    ) -> Vec<MapIssue> {
        let mut issues = Vec::new();
        let upper = map_name.to_ascii_uppercase();
        let clean_game_type = game_type.unwrap_or("").trim();

        let is_dm = upper.starts_with("DM-")
            || upper.starts_with("DM_")
            || (upper.starts_with("DM") && upper.chars().nth(2).map_or(false, |c| !c.is_alphanumeric()));

        if is_dm {
            if clean_game_type.contains("Domination") {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Map prefix is DeathMatch ('{}') but LevelInfo.DefaultGameType is '{}' (Domination). The engine will launch Domination instead of DeathMatch.",
                        map_name, clean_game_type
                    ),
                    object_name: "LevelInfo".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            } else if clean_game_type.contains("CTFGame") {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Map prefix is DeathMatch ('{}') but LevelInfo.DefaultGameType is '{}' (Capture the Flag).",
                        map_name, clean_game_type
                    ),
                    object_name: "LevelInfo".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            } else if clean_game_type.contains("Assault") {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Map prefix is DeathMatch ('{}') but LevelInfo.DefaultGameType is '{}' (Assault).",
                        map_name, clean_game_type
                    ),
                    object_name: "LevelInfo".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }

            if control_points_count > 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "DeathMatch map ('{}') contains {} Domination ControlPoint actor(s). These belong exclusively in DOM- maps.",
                        map_name, control_points_count
                    ),
                    object_name: "ControlPoint".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
            if flag_bases_count > 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "DeathMatch map ('{}') contains {} FlagBase actor(s). These belong exclusively in CTF- maps.",
                        map_name, flag_bases_count
                    ),
                    object_name: "FlagBase".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
            if fort_standards_count > 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "DeathMatch map ('{}') contains {} FortStandard actor(s). These belong exclusively in AS- maps.",
                        map_name, fort_standards_count
                    ),
                    object_name: "FortStandard".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
        } else if upper.starts_with("DOM-") || upper.starts_with("DOM_") {
            if !clean_game_type.is_empty() && !clean_game_type.contains("Domination") {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Map prefix is Domination ('{}') but LevelInfo.DefaultGameType is '{}' instead of Botpack.Domination.",
                        map_name, clean_game_type
                    ),
                    object_name: "LevelInfo".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
            if control_points_count == 0 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Domination map ('{}') has no ControlPoint actors (standard requires 3).",
                        map_name
                    ),
                    object_name: "ControlPoint".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
        } else if upper.starts_with("CTF-") || upper.starts_with("CTF_") {
            if !clean_game_type.is_empty() && !clean_game_type.contains("CTFGame") {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Map prefix is CTF ('{}') but LevelInfo.DefaultGameType is '{}' instead of Botpack.CTFGame.",
                        map_name, clean_game_type
                    ),
                    object_name: "LevelInfo".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
            if flag_bases_count < 2 {
                issues.push(MapIssue {
                    severity: IssueSeverity::Error,
                    category: "GameModeMismatch".to_string(),
                    description: format!(
                        "Capture the Flag map ('{}') found {} FlagBase actors (requires 2: Red and Blue).",
                        map_name, flag_bases_count
                    ),
                    object_name: "FlagBase".to_string(),
                    location: None,
                    auto_fixable: false,
                });
            }
        }

        issues
    }
}

/// Computes the 3D polygon surface area
fn compute_poly_area(vertices: &[Vec3]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }
    let p0 = vertices[0];
    let mut total = Vec3::ZERO;
    for i in 1..vertices.len() - 1 {
        let e1 = vertices[i] - p0;
        let e2 = vertices[i + 1] - p0;
        total += e1.cross(e2);
    }
    total.length() * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::Polygon;
    use crate::CsgOperation;

    #[test]
    fn test_diagnostics_audit_degenerate_brush() {
        let mut brush = Brush::new("BadBrush", CsgOperation::Add);
        // Valid poly with texture
        let p_valid = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
        ]).unwrap().with_texture("Indus1.Indus_B01");
        brush.polys.push(p_valid);

        // Valid poly without texture
        let p_untextured = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::new(100.0, 0.0, 50.0),
            Vec3::new(100.0, 100.0, 50.0),
        ]).unwrap();
        brush.polys.push(p_untextured);

        // Degenerate zero-area poly (collinear points)
        let p_degen = Polygon::new(vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(20.0, 0.0, 0.0),
        ]).unwrap_or_else(|| {
            // Force degenerate Polygon directly
            Polygon {
                vertices: vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0), Vec3::new(20.0, 0.0, 0.0)],
                plane: crate::math::Plane { normal: Vec3::Z, dist: 0.0 },
                texture: String::new(),
                flags: 0,
                base: Vec3::ZERO,
                texture_u: Vec3::X,
                texture_v: Vec3::Y,
                pan_u: 0,
                pan_v: 0,
                vertex_colors: Vec::new(),
                lightmap: None,
            }
        });
        brush.polys.push(p_degen);

        let issues = LevelDoctor::audit_brushes(&[brush.clone()]);
        assert!(!issues.is_empty(), "Should report issues for degenerate poly and missing texture");
        assert!(issues.iter().any(|i| i.category == "Geometry"));
        assert!(issues.iter().any(|i| i.category == "Texture"));

        // Auto-fix
        let fixes = LevelDoctor::autofix_brush(&mut brush, "Indus1.DefaultWall");
        assert_eq!(fixes, 2, "Should fix exactly 1 degenerate poly and 1 missing texture");
        assert_eq!(brush.polys.len(), 2, "Degenerate poly must be removed, leaving 2 valid polys");
        assert_eq!(brush.polys[0].texture, "Indus1.Indus_B01");
        assert_eq!(brush.polys[1].texture, "Indus1.DefaultWall");
    }

    #[test]
    fn test_diagnostics_audit_navigation_orphans() {
        let mut graph = NavGraph::new();
        let n0 = graph.add_node("Connected0", Vec3::new(0.0, 0.0, 0.0), "PathNode");
        let n1 = graph.add_node("Connected1", Vec3::new(200.0, 0.0, 0.0), "PathNode");
        let _isolated = graph.add_node("StrandedNode", Vec3::new(1000.0, 0.0, 0.0), "PathNode");

        graph.add_reach_spec(n0, n1, 1);
        graph.add_reach_spec(n1, n0, 1);

        let issues = LevelDoctor::audit_navigation_graph(&graph);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, IssueSeverity::Error);
        assert!(issues[0].description.contains("StrandedNode"));
    }

    #[test]
    fn navigation_audit_reports_disconnected_linked_groups() {
        let mut graph = NavGraph::new();
        for i in 0..4 {
            graph.add_node(format!("node{i}"), Vec3::new(i as f32 * 100.0, 0.0, 0.0), "PathNode");
        }
        for (a, b) in [(0, 1), (1, 0), (2, 3), (3, 2)] {
            graph.add_reach_spec(a, b, crate::navigation::R_WALK);
        }
        let issues = LevelDoctor::audit_navigation_graph(&graph);
        assert_eq!(issues.iter().filter(|i| i.description.contains("disconnected groups")).count(), 2);
        graph.add_reach_spec(1, 2, crate::navigation::R_WALK);
        assert!(!LevelDoctor::audit_navigation_graph(&graph).iter().any(|i| i.description.contains("disconnected groups")));
    }

    #[test]
    fn navigation_audit_respects_direction_and_all_spawn_sources() {
        let mut graph = NavGraph::new();
        let spawn = graph.add_node("spawn", Vec3::ZERO, "Engine.PlayerStart");
        let other = graph.add_node("other", Vec3::X, "PathNode");
        let item = graph.add_node("item", Vec3::Y, "Botpack.InventorySpot");
        graph.add_reach_spec(spawn, other, crate::navigation::R_WALK);
        graph.add_reach_spec(other, spawn, crate::navigation::R_WALK);
        graph.add_reach_spec(item, other, crate::navigation::R_WALK);
        let report = LevelDoctor::audit_navigation_graph(&graph);
        assert!(report.iter().any(|i| i.object_name == "item" && i.description.contains("unreachable from every PlayerStart")));
        assert!(!report.iter().any(|i| i.description.contains("disconnected groups")));

        let second = graph.add_node("second", Vec3::Z, "PlayerStart");
        graph.add_reach_spec(second, item, crate::navigation::R_WALK);
        let report = LevelDoctor::audit_navigation_graph(&graph);
        assert!(!report.iter().any(|i| i.object_name == "item" && i.description.contains("unreachable from every PlayerStart")));
    }

    #[test]
    fn navigation_audit_does_not_infer_item_unreachability_without_spawns() {
        let mut graph = NavGraph::new();
        graph.add_node("item", Vec3::ZERO, "InventorySpot");
        let report = LevelDoctor::audit_navigation_graph(&graph);
        assert!(!report.iter().any(|i| i.description.contains("unreachable from every PlayerStart")));
    }

    #[test]
    fn test_diagnostics_audit_gameplay_clearance() {
        let mut graph = NavGraph::new();
        let n0 = graph.add_node("NodeA", Vec3::new(0.0, 0.0, 1000.0), "PathNode");
        let n1 = graph.add_node("NodeB", Vec3::new(20.0, 0.0, 100.0), "PathNode"); // drop of 900 uu, dist 20 uu

        graph.reach_specs.push(crate::navigation::NavReachSpec {
            from: n0,
            to: n1,
            distance: 20.0, // bottleneck < 38
            reach_flags: 1,
        });

        let mut solid_brush = crate::build_cube("SolidPillar", CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
        solid_brush.location = Vec3::new(500.0, 500.0, 0.0);

        let actors = vec![
            ("PlayerStart1".to_string(), "Botpack.PlayerStart".to_string(), Vec3::new(500.0, 500.0, 0.0)),
        ];

        let issues = LevelDoctor::audit_gameplay_clearance(&[solid_brush], &graph, &actors);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|i| i.category == "GameplayClearance" && i.description.contains("bottleneck")));
        assert!(issues.iter().any(|i| i.category == "GameplayHazard" && i.description.contains("Lethal drop")));
        assert!(issues.iter().any(|i| i.category == "ActorClearance" && i.description.contains("embedded")));
    }

    #[test]
    fn test_audit_builder_brush_convexity_and_solidity() {
        // 1. Convex watertight cube
        let cube = crate::build_cube("ConvexCube", CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
        let diag = LevelDoctor::audit_builder_brush(&cube);
        assert!(diag.is_convex, "Cube must be recognized as strictly convex");
        assert!(diag.is_watertight, "Cube must be watertight");
        assert!(!diag.has_degenerate_polys);
        assert_eq!(diag.reflex_vertex_count, 0);
        assert!(!diag.coplanar_split_risk);
        assert!(diag.summary_message.contains("Safe for CSG"));

        // 2. Concave L-shaped brush
        let mut concave = crate::build_cube("ConcaveBase", CsgOperation::Add, Vec3::new(256.0, 256.0, 256.0));
        // Add an extra polygon that creates a reflex indentation
        let p_reflex = Polygon::new(vec![
            Vec3::new(100.0, 100.0, 400.0),
            Vec3::new(200.0, 100.0, 400.0),
            Vec3::new(200.0, 200.0, 400.0),
        ]).unwrap();
        concave.polys.push(p_reflex);
        let diag_concave = LevelDoctor::audit_builder_brush(&concave);
        assert!(!diag_concave.is_convex, "Concave brush must be flagged as non-convex");
        assert!(diag_concave.reflex_vertex_count > 0);
        assert!(diag_concave.coplanar_split_risk);
        assert!(diag_concave.summary_message.contains("Non-convex Builder Brush"));
    }

    #[test]
    fn test_audit_mode_prefix_coherence() {
        // DM map with Domination GameType and ControlPoints
        let issues = LevelDoctor::audit_mode_prefix_coherence(
            "DM-GothicArena",
            Some("Botpack.Domination"),
            3,
            0,
            0,
        );
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|i| i.description.contains("LevelInfo.DefaultGameType is 'Botpack.Domination'")));
        assert!(issues.iter().any(|i| i.description.contains("contains 3 Domination ControlPoint actor(s)")));

        // Clean DM map
        let clean_dm = LevelDoctor::audit_mode_prefix_coherence(
            "DM-Hyperborea",
            Some("Botpack.DeathMatchPlus"),
            0,
            0,
            0,
        );
        assert!(clean_dm.is_empty());

        // DOM map without ControlPoints
        let dom_empty = LevelDoctor::audit_mode_prefix_coherence(
            "DOM-CinderSpire",
            Some("Botpack.Domination"),
            0,
            0,
            0,
        );
        assert_eq!(dom_empty.len(), 1);
        assert!(dom_empty[0].description.contains("has no ControlPoint actors"));
    }

    #[test]
    fn test_audit_tournament_playability() {
        // Hazardous setup: 2 PlayerStarts too close (< 180 UU), 1 PlayerStart far from weapons (> 850 UU), 1 orphan weapon
        let player_starts = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0), // Telefrag risk (< 180)
            Vec3::new(2000.0, 2000.0, 0.0), // Starvation risk (> 850)
        ];
        let weapons = vec![
            ("Botpack.ShockRifle".to_string(), Vec3::new(400.0, 0.0, 0.0)),
            ("Botpack.UT_FlakCannon".to_string(), Vec3::new(800.0, 800.0, 0.0)), // Orphan weapon
        ];
        let ammo = vec![
            ("Botpack.ShockCore".to_string(), Vec3::new(450.0, 0.0, 0.0)), // Within 400 of ShockRifle
        ];

        let issues = LevelDoctor::audit_tournament_playability(&player_starts, &weapons, &ammo);
        assert_eq!(issues.len(), 3);
        assert!(issues.iter().any(|i| i.category == "Spawns" && i.description.contains("telefrag")));
        assert!(issues.iter().any(|i| i.category == "GameplayBalance" && i.description.contains("Weapon starvation")));
        assert!(issues.iter().any(|i| i.category == "GameplayBalance" && i.description.contains("Orphan weapon")));

        // Clean setup
        let clean_spawns = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(500.0, 0.0, 0.0),
        ];
        let clean_weapons = vec![
            ("Botpack.ShockRifle".to_string(), Vec3::new(250.0, 0.0, 0.0)),
        ];
        let clean_ammo = vec![
            ("Botpack.ShockCore".to_string(), Vec3::new(300.0, 0.0, 0.0)),
        ];
        let clean_issues = LevelDoctor::audit_tournament_playability(&clean_spawns, &clean_weapons, &clean_ammo);
        assert!(clean_issues.is_empty(), "Clean playability setup must produce 0 issues");
    }
}
