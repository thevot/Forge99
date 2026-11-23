use glam::Vec3;
use ut99_editor::{CompilerBridge, EditorState};
use ut99_format::{ActorRecord, UnrLevel};
mod common;

#[test]
fn clearing_paths_roundtrips_without_deleting_inventory_actors() {
    let dir = common::TestDir::new();
    let mut state = EditorState::default();
    state.new_map();
    let mut item = ActorRecord::default();
    item.name = "TestShock".into();
    item.class_name = "ShockRifle".into();
    item.location = Vec3::new(64.0, 0.0, 64.0);
    let mut node = ActorRecord::default();
    node.name = "TestPath".into();
    node.class_name = "PathNode".into();
    node.location = Vec3::new(192.0, 0.0, 64.0);
    state.loaded_unr.as_mut().unwrap().actors.extend([item, node]);
    state.build_ai_paths();
    assert!(!state.nav_graph.reach_specs.is_empty(), "fixture needs estimated paths");

    let before_path = dir.path().join("DM-BeforeClear.unr");
    CompilerBridge::save_unr_native(&state, &before_path).unwrap();
    let before = UnrLevel::open(&before_path).unwrap();
    assert!(!before.reach_specs.is_empty(), "fixture must serialize reachspecs");

    let actor_snapshot = state.loaded_unr.as_ref().unwrap().actors.clone();
    state.clear_ai_paths();
    let cleared_path = dir.path().join("DM-AfterClear.unr");
    CompilerBridge::save_unr_native(&state, &cleared_path).unwrap();
    let cleared = UnrLevel::open(&cleared_path).unwrap();
    assert!(cleared.reach_specs.is_empty(), "cleared package must contain no reachspecs");
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors, actor_snapshot);
    for class in ["ShockRifle", "InventorySpot", "PathNode"] {
        assert_eq!(before.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case(class)).count(),
            cleared.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case(class)).count(), "actor class {class}");
    }
    let before_item = before.actors.iter().find(|a| a.name == "TestShock").unwrap();
    let cleared_item = cleared.actors.iter().find(|a| a.name == "TestShock").unwrap();
    assert!(before_item.properties.keys().any(|key| key.eq_ignore_ascii_case("myMarker")), "fixture must serialize the pickup reference");
    assert!(matches!(before_item.properties.get("myMarker").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index != 0), "myMarker must be a non-null object reference, got {:?}", before_item.properties.get("myMarker"));
    assert_eq!(before_item.properties.get("myMarker"), cleared_item.properties.get("myMarker"));
    let before_spot = before.actors.iter().find(|a| a.class_name.eq_ignore_ascii_case("InventorySpot")).unwrap();
    let cleared_spot = cleared.actors.iter().find(|a| a.name == before_spot.name).unwrap();
    assert!(before_spot.properties.keys().any(|key| key.eq_ignore_ascii_case("markedItem")), "fixture must serialize the reciprocal reference");
    assert!(matches!(before_spot.properties.get("markedItem").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index != 0), "markedItem must be a non-null object reference, got {:?}", before_spot.properties.get("markedItem"));
    assert!(matches!(before_item.properties.get("myMarker").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index == before_spot.export_index as i32 + 1));
    assert!(matches!(before_spot.properties.get("markedItem").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index == before_item.export_index as i32 + 1));
    assert_eq!(before_spot.properties.get("markedItem"), cleared_spot.properties.get("markedItem"));

    let reopened_path = dir.path().join("DM-Resaved.unr");
    before.save(&reopened_path).unwrap();
    let resaved = UnrLevel::open(reopened_path).unwrap();
    let item = resaved.actors.iter().find(|a| a.name == "TestShock").unwrap();
    let spot = resaved.actors.iter().find(|a| a.name == before_spot.name).unwrap();
    assert!(matches!(item.properties.get("myMarker").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index == spot.export_index as i32 + 1));
    assert!(matches!(spot.properties.get("markedItem").map(|p| &p.value), Some(ut99_format::unr::tagged_property::TaggedValue::Object(index)) if *index == item.export_index as i32 + 1));

    state.undo();
    let undo_path = dir.path().join("DM-UndoClear.unr");
    CompilerBridge::save_unr_native(&state, &undo_path).unwrap();
    let restored = UnrLevel::open(&undo_path).unwrap();
    assert_eq!(restored.reach_specs.len(), before.reach_specs.len());
    state.redo();
    assert!(state.nav_graph.reach_specs.is_empty());
}

#[test]
fn unresolved_symbolic_inventory_reference_is_not_fabricated() {
    use ut99_format::unr::tagged_property::{TaggedProperty, TaggedValue};
    let dir = common::TestDir::new();
    let mut state = EditorState::default();
    state.new_map();
    let mut item = ActorRecord::default();
    item.name = "TestUnresolved".into();
    item.class_name = "ShockRifle".into();
    item.properties.insert("myMarker".into(), TaggedProperty {
        name: "myMarker".into(), array_index: 0, value: TaggedValue::Name("MissingSpot".into()),
    });
    state.loaded_unr.as_mut().unwrap().actors.push(item);
    let path = dir.path().join("DM-Unresolved.unr");
    CompilerBridge::save_unr_native(&state, &path).unwrap();
    let loaded = UnrLevel::open(path).unwrap();
    let saved = loaded.actors.iter().find(|a| a.name == "TestUnresolved").unwrap();
    assert_eq!(saved.properties.get("myMarker").map(|p| &p.value), Some(&TaggedValue::Name("MissingSpot".into())));
}
