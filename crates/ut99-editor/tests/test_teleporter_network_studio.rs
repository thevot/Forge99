use glam::Vec3;
use ut99_bsp::{
    TeleportDiagnosticCategory, TeleportDiagnosticSeverity, TeleportNetworkGraph, TeleporterNode,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_teleporter_graph_resolution_and_diagnostics() {
    let mut nodes = Vec::new();

    // Node 0 -> Node 1 (Bidirectional pair)
    nodes.push(TeleporterNode::new(0, "Tele_BlueBase", "BluePortal", "RedPortal", Vec3::new(0.0, 0.0, 50.0)));
    nodes.push(TeleporterNode::new(1, "Tele_RedBase", "RedPortal", "BluePortal", Vec3::new(2000.0, 0.0, 50.0)));

    // Node 2 -> Broken URL
    nodes.push(TeleporterNode::new(2, "Tele_Broken", "BrokenGate", "NonExistentTag", Vec3::new(1000.0, 1000.0, 50.0)));

    // Node 3 -> Orphan destination (has Tag, but nothing routes to it)
    nodes.push(TeleporterNode::new(3, "Tele_Orphan", "UnlinkedDestination", "", Vec3::new(-1000.0, 0.0, 50.0)));

    let graph = TeleportNetworkGraph::build(nodes, &[]);

    assert_eq!(graph.links.len(), 2, "Must resolve the two bidirectional links");
    assert!(graph.links[0].is_bidirectional);
    assert!(graph.links[1].is_bidirectional);

    // Verify BrokenUrl diagnostic
    let broken: Vec<_> = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::BrokenUrl)
        .collect();
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].node_id, 2);
    assert_eq!(broken[0].severity, TeleportDiagnosticSeverity::Error);

    // Verify Orphan diagnostics (Node 2 and Node 3 both define tags that have no incoming links)
    let orphans: Vec<_> = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::OrphanDestination)
        .collect();
    assert_eq!(orphans.len(), 2);
    assert!(orphans.iter().any(|d| d.node_id == 3));
    assert!(orphans.iter().any(|d| d.node_id == 2));
}

#[test]
fn test_infinite_loop_and_telefrag_detection() {
    let mut nodes = Vec::new();
    // Two teleporters placed 40 uu apart pointing directly at each other -> Critical Infinite Ping-Pong Hazard
    nodes.push(TeleporterNode::new(0, "LoopA", "TagA", "TagB", Vec3::new(100.0, 100.0, 0.0)));
    nodes.push(TeleporterNode::new(1, "LoopB", "TagB", "TagA", Vec3::new(140.0, 100.0, 0.0)));

    let player_spawns = vec![Vec3::new(145.0, 100.0, 0.0)]; // Close to LoopB (telefrag danger)

    let graph = TeleportNetworkGraph::build(nodes, &player_spawns);

    let loops: Vec<_> = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::InfiniteLoopHazard)
        .collect();
    assert!(!loops.is_empty(), "Must detect infinite ping-pong loop hazard between adjacent teleporters");

    let telefrags: Vec<_> = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::TelefragHazard)
        .collect();
    assert!(!telefrags.is_empty(), "Must detect telefrag proximity hazard to PlayerStart");
}

#[test]
fn test_exit_velocity_and_direction_calculation() {
    let mut nodes = Vec::new();
    let mut src = TeleporterNode::new(0, "Src", "SrcTag", "DstTag", Vec3::new(0.0, 0.0, 0.0));
    src.rotation.y = 0.0; // Forward along +X

    let mut dst = TeleporterNode::new(1, "Dst", "DstTag", "", Vec3::new(1000.0, 0.0, 0.0));
    dst.rotation.y = 90.0; // Forward along +Y
    dst.b_changes_velocity = true;
    dst.target_velocity = Vec3::new(0.0, 800.0, 250.0);
    dst.b_changes_yaw = true;

    nodes.push(src);
    nodes.push(dst);

    let graph = TeleportNetworkGraph::build(nodes, &[]);
    let (vel, dir) = graph.calculate_exit_trajectory(0, 400.0).expect("Exit trajectory resolved");

    assert_eq!(vel, Vec3::new(0.0, 800.0, 250.0), "Must apply target velocity from destination");
    assert!((dir.x).abs() < 1e-3);
    assert!((dir.y - 1.0).abs() < 1e-3, "Exit direction must face yaw +Y (90°)");
}

#[test]
fn test_editor_pair_creation_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(500.0, 500.0, 100.0);
    state.teleporter_studio_settings.auto_pair_prefix = "CTFFlagPortal".to_string();

    // 1. Create paired teleporters
    let (id_a, id_b) = state.create_paired_teleporters_at_pivot(1000.0);

    let unr = state.loaded_unr.as_ref().expect("UNR level present");
    assert!(unr.actors.len() >= 2);
    let actor_a = &unr.actors[id_a];
    let actor_b = &unr.actors[id_b];

    assert_eq!(actor_a.tag, "CTFFlagPortal_PortalA");
    assert_eq!(actor_b.tag, "CTFFlagPortal_PortalB");
    assert!(actor_a.properties.contains_key("URL"));
    assert!(actor_b.properties.contains_key("URL"));

    // 2. Network graph collected from editor state
    let graph = state.collect_teleporter_network();
    assert!(graph.links.iter().any(|l| l.source_id == id_a && l.dest_id == id_b));

    // 3. Test Undo
    assert!(state.history.can_undo());
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().unwrap();
    assert!(unr_after_undo.actors.len() < id_b);

    // 4. Test Redo
    assert!(state.history.can_redo());
    state.redo();
    let unr_after_redo = state.loaded_unr.as_ref().unwrap();
    assert!(unr_after_redo.actors.len() >= 2);
    assert_eq!(unr_after_redo.actors[id_a].tag, "CTFFlagPortal_PortalA");
    assert_eq!(unr_after_redo.actors[id_b].tag, "CTFFlagPortal_PortalB");

    // 5. Test Property Update and Undo
    state.update_teleporter_properties(id_a, "CustomTagA", "CustomUrlB", true, Vec3::new(600.0, 0.0, 200.0), true);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[id_a].tag, "CustomTagA");

    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[id_a].tag, "CTFFlagPortal_PortalA", "Undo must revert teleporter tag to original");

    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[id_a].tag, "CustomTagA", "Redo must re-apply updated teleporter tag");
}
