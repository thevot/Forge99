//! Read-only inspection of inventory navigation references in selected maps.
//! Usage: cargo run -p ut99-editor --example inspect_inventory_markers -- map.unr [...]
use std::{env, path::PathBuf};
use ut99_format::{ActorRecord, UnrLevel};

fn reference(actor: &ActorRecord, name: &str) -> Option<serde_json::Value> {
    let property = actor.properties.iter().find(|(key, _)| key.eq_ignore_ascii_case(name));
    let path = actor.object_paths.iter().find(|(key, _)| key.eq_ignore_ascii_case(name));
    if property.is_none() && path.is_none() { return None; }
    Some(serde_json::json!({
        "value": property.map(|(_, p)| format!("{:?}", p.value)),
        "resolved_path": path.map(|(_, p)| p),
    }))
}

fn main() -> anyhow::Result<()> {
    let mut records = Vec::new();
    for path in env::args_os().skip(1).map(PathBuf::from) {
        let record = match UnrLevel::open(&path) {
            Ok(level) => {
                let actors: Vec<_> = level.actors.iter().filter_map(|actor| {
                    let marker = reference(actor, "myMarker");
                    let item = reference(actor, "markedItem");
                    if marker.is_none() && item.is_none() && !actor.class_name.eq_ignore_ascii_case("InventorySpot") {
                        return None;
                    }
                    Some(serde_json::json!({
                        "name": actor.name, "class": actor.class_name,
                        "location": [actor.location.x, actor.location.y, actor.location.z],
                        "my_marker": marker, "marked_item": item,
                    }))
                }).collect();
                let read_issues = level.read_issues.clone();
                let mut state = ut99_editor::EditorState::default();
                state.loaded_unr = Some(level);
                state.run_level_doctor_audit();
                let link_issues: Vec<_> = state.map_audit_issues.iter()
                    .filter(|issue| issue.category == "InventoryLinks")
                    .map(|issue| serde_json::json!({"actor": issue.object_name, "description": issue.description}))
                    .collect();
                serde_json::json!({"file": path, "status": "parsed", "read_issues": read_issues, "actors": actors, "inventory_link_issues": link_issues})
            },
            Err(error) => serde_json::json!({"file": path, "status": "error", "error": format!("{error:#}")}),
        };
        records.push(record);
    }
    println!("{}", serde_json::to_string_pretty(&records)?);
    Ok(())
}
