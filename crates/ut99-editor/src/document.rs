use crate::compiler::CompilerBridge;
use crate::editor_state::{EditorMover, EditorState};
use ut99_bsp::{Brush, CsgOperation, PF_NOT_SOLID, PF_SEMISOLID};
use ut99_format::ActorRecord;

/// Summary metrics of a [`LevelDocument`] (Deliverable F.1 SSOT).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentSummary {
    pub total_brushes: usize,
    pub add_brushes: usize,
    pub subtract_brushes: usize,
    pub semisolid_brushes: usize,
    pub nonsolid_brushes: usize,
    pub total_polygons: usize,
    pub movers_count: usize,
    pub actors_count: usize,
    pub lights_count: usize,
    pub nav_points_count: usize,
}

/// Unified Single Source of Truth (`SSOT`) representation of an Unreal Tournament 99 level
/// (`Deliverable F.1` — `LevelDocument`), bridging `.t3d`, `.unr`, `world_brushes`, `movers`, and `actors`.
#[derive(Debug, Clone, Default)]
pub struct LevelDocument {
    pub world_brushes: Vec<Brush>,
    pub movers: Vec<EditorMover>,
    pub static_meshes: Vec<Brush>,
    pub actors: Vec<ActorRecord>,
    pub reach_specs_count: usize,
}

impl LevelDocument {
    /// Captures the authoritative [`LevelDocument`] state from an [`EditorState`].
    pub fn from_editor_state(state: &EditorState) -> Self {
        let actors = state
            .loaded_unr
            .as_ref()
            .map(|u| {
                u.actors
                    .iter()
                    .filter(|a| !a.class_name.eq_ignore_ascii_case("Brush"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Self {
            world_brushes: state.world_brushes.to_vec(),
            movers: state.movers.to_vec(),
            static_meshes: state.static_meshes.to_vec(),
            actors,
            reach_specs_count: state.nav_graph.reach_specs.len(),
        }
    }

    /// Synchronizes an [`EditorState`] from this [`LevelDocument`], ensuring `world_brushes`,
    /// `movers`, `static_meshes`, and `loaded_unr` are 100% consistent.
    pub fn apply_to_editor_state(&self, state: &mut EditorState) {
        state.world_brushes = self.world_brushes.clone().into();
        state.movers = self.movers.clone().into();
        state.static_meshes = self.static_meshes.clone().into();

        let mut unr = state.loaded_unr.take().unwrap_or_default();
        unr.actors = self.actors.clone().into();
        unr.sync_actor_views();
        state.loaded_unr = Some(unr);
        state.map_instance_id = crate::editor_state::next_map_instance_id();
        state.map_is_dirty = true;
    }

    /// Computes a structural summary of all geometry, movers, lights, and navigation points in the document.
    pub fn summary(&self) -> DocumentSummary {
        let mut s = DocumentSummary {
            total_brushes: self.world_brushes.len(),
            movers_count: self.movers.len(),
            actors_count: self.actors.len(),
            ..Default::default()
        };

        for b in &self.world_brushes {
            s.total_polygons += b.polys.len();
            let is_semi = b.polys.iter().any(|p| (p.flags & PF_SEMISOLID) != 0);
            let is_non = b.polys.iter().any(|p| (p.flags & PF_NOT_SOLID) != 0);
            match b.csg_oper {
                CsgOperation::Subtract => s.subtract_brushes += 1,
                CsgOperation::Add => {
                    if is_semi {
                        s.semisolid_brushes += 1;
                    } else if is_non {
                        s.nonsolid_brushes += 1;
                    } else {
                        s.add_brushes += 1;
                    }
                }
                _ => {}
            }
        }

        for a in &self.actors {
            if a.class_name.contains("Light") {
                s.lights_count += 1;
            }
            if a.class_name.contains("PathNode")
                || a.class_name.contains("PlayerStart")
                || a.class_name.contains("InventorySpot")
                || a.class_name.contains("Lift")
                || a.class_name.contains("Teleporter")
            {
                s.nav_points_count += 1;
            }
        }

        s
    }

    /// Serializes this [`LevelDocument`] to a lossless UnrealEd 2.0 `.t3d` string.
    pub fn to_t3d_string(&self, builder_brush: &Brush) -> String {
        let mut temp_state = EditorState::default();
        self.apply_to_editor_state(&mut temp_state);
        temp_state.builder_brush = builder_brush.clone();
        let t3d_map = CompilerBridge::export_t3d_map(&temp_state);
        let mut buf = Vec::new();
        let _ = ut99_format::write_t3d(&mut buf, &t3d_map);
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Parses an UnrealEd 2.0 `.t3d` map string into a [`LevelDocument`].
    pub fn from_t3d_str(t3d_text: &str) -> Result<Self, String> {
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join(format!("_rea_doc_{}.t3d", std::process::id()));
        std::fs::write(&tmp_path, t3d_text).map_err(|e| e.to_string())?;
        let mut state = EditorState::default();
        let res = state.load_t3d(tmp_path.to_str().unwrap_or_default());
        let _ = std::fs::remove_file(&tmp_path);
        res.map_err(|e| e.to_string())?;
        Ok(Self::from_editor_state(&state))
    }
}
