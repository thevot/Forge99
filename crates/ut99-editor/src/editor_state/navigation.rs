use super::*;

impl EditorState {
    pub fn rebuild_navigation_paths(&mut self) {
        self.nav_graph = NavGraph::new();

        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if actor.class_name.contains("PathNode")
                    || actor.class_name.contains("PlayerStart")
                    || actor.class_name.contains("Inventory")
                    || actor.class_name.contains("Lift")
                    || actor.class_name.contains("Teleporter")
                {
                    self.nav_graph.add_node(&actor.name, actor.location, &actor.class_name);
                }
            }
        }

        // If no actors found in unr, use demo path nodes from map center
        if self.nav_graph.nodes.is_empty() {
            let p0 = self.nav_graph.add_node("PlayerStart0", Vec3::new(-256.0, 0.0, 32.0), "PlayerStart");
            let p1 = self.nav_graph.add_node("PathNode0", Vec3::new(0.0, 0.0, 32.0), "PathNode");
            let p2 = self.nav_graph.add_node("InventorySpot0", Vec3::new(256.0, 0.0, 32.0), "InventorySpot");
            let p3 = self.nav_graph.add_node("PathNode1", Vec3::new(256.0, 256.0, 32.0), "PathNode");
            self.nav_graph.add_reach_spec(p0, p1, ut99_bsp::R_WALK);
            self.nav_graph.add_reach_spec(p1, p0, ut99_bsp::R_WALK);
            self.nav_graph.add_reach_spec(p1, p2, ut99_bsp::R_WALK);
            self.nav_graph.add_reach_spec(p2, p1, ut99_bsp::R_WALK);
            self.nav_graph.add_reach_spec(p2, p3, ut99_bsp::R_WALK);
            self.nav_graph.add_reach_spec(p3, p2, ut99_bsp::R_WALK);
        } else {
            self.nav_graph.build_paths(1000.0, 64.0, 320.0);
        }

        self.log(format!("Rebuilt Navigation Network: {} nodes, {} reach specs.", self.nav_graph.nodes.len(), self.nav_graph.reach_specs.len()));
    }

    pub fn start_bot_simulation(&mut self, start_node: usize, target_node: usize) -> bool {
        if let Some(path) = self.nav_graph.find_shortest_path(start_node, target_node) {
            let start_loc = self.nav_graph.nodes[start_node].location;
            let mut bot = BotSimulator::new("SimBot_Xan", start_loc);
            bot.set_path(path, &self.nav_graph);
            self.bot_simulator = Some(bot);
            self.log(format!("Bot AI Simulation started from node #{} to node #{}.", start_node, target_node));
            true
        } else {
            self.log(format!("Bot AI: No valid reachability path found from node #{} to node #{}.", start_node, target_node));
            false
        }
    }

    pub fn step_bot_simulation(&mut self, dt: f32) {
        if let Some(ref mut bot) = self.bot_simulator {
            bot.update(dt, &self.nav_graph);
        }
    }

    // --- Fase 6: Level Doctor & Diagnostics ---

}
