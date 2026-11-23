//! Test Suite for Milestone //! UnrealScript Class Source Viewer & DefaultProperties Live Inspector.

use std::fs;
use ut99_editor::editor_state::EditorState;
use ut99_editor::unrealscript::get_class_metadata;
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_unrealscript_metadata_and_source_decompilation() {
    // 1. Test canonical Actor class
    let actor_meta = get_class_metadata("Actor", None, None);
    assert_eq!(actor_meta.name, "Actor");
    assert_eq!(actor_meta.package, "Engine");
    assert_eq!(actor_meta.super_class, "Object");
    assert!(!actor_meta.variables.is_empty());
    assert!(!actor_meta.functions.is_empty());

    let actor_src = actor_meta.to_uc_source();
    assert!(actor_src.contains("class Actor extends Object"));
    assert!(actor_src.contains("var(Display)"));
    assert!(actor_src.contains("var(Collision)"));
    assert!(actor_src.contains("defaultproperties"));
    assert!(actor_src.contains("bHidden=True") || actor_src.contains("bHidden=true") || actor_src.contains("DrawType="));

    // 2. Test Light class
    let light_meta = get_class_metadata("Light", Some("Actor"), Some("Engine"));
    assert_eq!(light_meta.name, "Light");
    assert_eq!(light_meta.super_class, "Actor");
    assert_eq!(light_meta.package, "Engine");

    let light_src = light_meta.to_uc_source();
    assert!(light_src.contains("class Light extends Actor"));
    assert!(light_src.contains("LightBrightness"));
    assert!(light_src.contains("LightRadius"));
    assert!(light_src.contains("LightType=LT_Steady"));

    // 3. Test ShockRifle class
    let shock_meta = get_class_metadata("ShockRifle", Some("TournamentWeapon"), Some("Botpack"));
    assert_eq!(shock_meta.name, "ShockRifle");
    assert_eq!(shock_meta.super_class, "TournamentWeapon");
    assert_eq!(shock_meta.package, "Botpack");

    let shock_src = shock_meta.to_uc_source();
    assert!(shock_src.contains("class ShockRifle extends TournamentWeapon"));
    assert!(shock_src.contains("Fire("));
    assert!(shock_src.contains("AltFire("));
    assert!(shock_src.contains("PickupAmmoCount=20"));

    // 4. Test ZoneInfo class
    let zone_meta = get_class_metadata("ZoneInfo", Some("Info"), Some("Engine"));
    assert_eq!(zone_meta.name, "ZoneInfo");
    let zone_src = zone_meta.to_uc_source();
    assert!(zone_src.contains("class ZoneInfo extends Info"));
    assert!(zone_src.contains("ZoneVelocity"));
    assert!(zone_src.contains("bWaterZone=False"));

    // 5. Test Fallback for unknown / custom class
    let custom_meta = get_class_metadata("MyCustomMutator", None, None);
    assert_eq!(custom_meta.name, "MyCustomMutator");
    let custom_src = custom_meta.to_uc_source();
    assert!(custom_src.contains("class MyCustomMutator extends Actor"));
    assert!(custom_src.contains("defaultproperties"));
}

#[test]
fn test_unrealscript_viewer_state_and_live_edits() {
    let mut state = EditorState::default();

    // Open viewer for class "Spotlight"
    state.open_uc_viewer_for_class("Spotlight");
    assert!(state.show_uc_viewer_dialog);
    assert_eq!(state.uc_viewer_class, "Spotlight");
    assert!(!state.uc_editable_defaults.is_empty());
    assert!(state.uc_source_cache.contains("class Spotlight extends Light"));

    // Check initial default property for Cone
    let initial_cone = state
        .uc_editable_defaults
        .iter()
        .find(|(k, _)| k == "LightCone")
        .map(|(_, v)| v.clone())
        .expect("Spotlight should have LightCone");
    assert_eq!(initial_cone, "32");

    // Live update property
    state.update_class_default_property("LightCone", "64");

    // Verify it updated in editable defaults
    let updated_cone = state
        .uc_editable_defaults
        .iter()
        .find(|(k, _)| k == "LightCone")
        .map(|(_, v)| v.clone())
        .unwrap();
    assert_eq!(updated_cone, "64");

    // Verify source cache was dynamically recompiled
    assert!(state.uc_source_cache.contains("LightCone=64"));
}

#[test]
fn test_unrealscript_propagation_to_actors_with_undo_redo() {
    let mut state = EditorState::default();

    // Setup loaded UNR level
    let mut level = UnrLevel::default();

    // Add 2 Light actors and 1 PathNode actor
    let mut light1 = ActorRecord::default();
    light1.class_name = "Light".to_string();
    light1.tag = "Light_Test1".to_string();

    let mut light2 = ActorRecord::default();
    light2.class_name = "Light".to_string();
    light2.tag = "Light_Test2".to_string();

    let mut pathnode = ActorRecord::default();
    pathnode.class_name = "PathNode".to_string();
    pathnode.tag = "PathNode_Test".to_string();

    level.actors.push(light1);
    level.actors.push(light2);
    level.actors.push(pathnode);

    state.loaded_unr = Some(level);

    // Open viewer for Light and modify LightBrightness and LightHue
    state.open_uc_viewer_for_class("Light");
    state.update_class_default_property("LightBrightness", "190");
    state.update_class_default_property("LightHue", "45");

    // Propagate default properties to all Light actors in the map
    let modified = state.propagate_default_properties_to_actors("Light");
    assert_eq!(modified, 2, "Should have updated exactly 2 Light actors");

    // Verify the actors have the modified properties
    {
        let unr = state.loaded_unr.as_ref().unwrap();
        let l1 = unr.actors.iter().find(|a| a.tag == "Light_Test1").unwrap();
        assert_eq!(l1.light_brightness, 190);
        assert_eq!(l1.light_hue, 45);
        assert_eq!(
            l1.properties.get("LightBrightness").map(|p| match &p.value {
                ut99_format::TaggedValue::Str(s) => s.as_str(),
                _ => "",
            }),
            Some("190")
        );
        assert_eq!(
            l1.properties.get("LightHue").map(|p| match &p.value {
                ut99_format::TaggedValue::Str(s) => s.as_str(),
                _ => "",
            }),
            Some("45")
        );

        let pn = unr.actors.iter().find(|a| a.tag == "PathNode_Test").unwrap();
        assert!(!pn.properties.contains_key("LightBrightness"));
    }

    // Test Undo
    state.undo();
    {
        let unr = state.loaded_unr.as_ref().unwrap();
        let l1 = unr.actors.iter().find(|a| a.tag == "Light_Test1").unwrap();
        assert!(!l1.properties.contains_key("LightBrightness"));
        assert!(!l1.properties.contains_key("LightHue"));
        assert_eq!(l1.light_brightness, 64); // restored to baseline
    }

    // Test Redo
    state.redo();
    {
        let unr = state.loaded_unr.as_ref().unwrap();
        let l1 = unr.actors.iter().find(|a| a.tag == "Light_Test1").unwrap();
        assert_eq!(
            l1.properties.get("LightBrightness").map(|p| match &p.value {
                ut99_format::TaggedValue::Str(s) => s.as_str(),
                _ => "",
            }),
            Some("190")
        );
        assert_eq!(l1.light_brightness, 190);
    }
}

#[test]
fn test_unrealscript_export_active_uc_file() {
    let mut state = EditorState::default();
    state.open_uc_viewer_for_class("ShockRifle");

    let tmp_dir = std::env::temp_dir().join(format!(
        "forge99_uc_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_dir).expect("Failed to create tempdir");

    let export_res = state.export_active_uc_file(&tmp_dir);
    assert!(export_res.is_ok(), "Exporting .uc file should succeed");

    let exported_path = export_res.unwrap();
    assert!(exported_path.exists(), "Exported file must exist on disk");
    assert_eq!(exported_path.file_name().unwrap(), "ShockRifle.uc");

    let content = fs::read_to_string(&exported_path).expect("Failed to read exported .uc");
    assert!(content.contains("class ShockRifle extends TournamentWeapon"));
    assert!(content.contains("defaultproperties"));
    assert_eq!(content, state.uc_source_cache);

    // Clean up
    let _ = fs::remove_dir_all(&tmp_dir);
}
