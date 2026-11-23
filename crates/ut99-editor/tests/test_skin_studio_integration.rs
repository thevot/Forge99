//! Integration tests for UT99 Skin & Character Authoring Studio in Forge99

use ut99_bsp::skin_studio::{
    audit_skin_project, generate_int_registration_manifest, get_builtin_skin_models,
    SkinAuditSeverity, SkinFaceEntry, SkinTexture,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_editor_state_skin_studio_initialization() {
    let state = EditorState::default();
    assert!(!state.show_skin_studio_dialog);
    assert_eq!(state.skin_studio_settings.active_tab, 0);
    assert_eq!(state.skin_studio_project.selected_model_index, 0);
    assert_eq!(state.skin_studio_project.package_name, "SoldierSkins_Custom");
    assert_eq!(state.skin_studio_project.skin_id, "cust");
    assert_eq!(state.skin_studio_project.skin_description, "Custom Soldier");
    assert_eq!(state.skin_studio_project.faces.len(), 1);
    assert!(state.skin_studio_project.chest_base.is_some());
    assert!(state.skin_studio_project.legs_base.is_some());
    assert!(state.skin_studio_project.arms_base.is_some());
}

#[test]
fn test_skin_studio_model_catalog_and_face_pipeline() {
    let models = get_builtin_skin_models();
    assert!(models.len() >= 10);

    // Verify Male Soldier
    let soldier = &models[0];
    assert_eq!(soldier.display_name, "Male Soldier");
    assert_eq!(soldier.model_prefix, "Soldier");
    assert_eq!(soldier.mesh_package, "Botpack.Soldier");

    // Test creating a new face with custom dimensions and cropping talk texture
    let face_tex = SkinTexture::create_solid("Face_Sarge".to_string(), 256, 256, [210, 160, 120, 255]);
    let face_entry = SkinFaceEntry::new("Sarge".to_string(), "Sarge".to_string(), face_tex);

    assert_eq!(face_entry.talk_texture.width, 64);
    assert_eq!(face_entry.talk_texture.height, 64);
    assert!(face_entry.talk_texture.palette.is_some());
}

#[test]
fn test_skin_studio_audit_and_int_emission() {
    let mut state = EditorState::default();
    state.skin_studio_project.package_name = "SoldierSkins_Steel".to_string();
    state.skin_studio_project.skin_id = "stel".to_string();
    state.skin_studio_project.skin_description = "Steel Commando".to_string();

    let report = audit_skin_project(&state.skin_studio_project);
    assert!(report.iter().any(|r| r.severity == SkinAuditSeverity::Info));

    let manifest = generate_int_registration_manifest(&state.skin_studio_project);
    assert!(manifest.contains("[public]"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel1,Class=Texture,Description=\"Steel Commando\")"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel2,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel3,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel4,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel1T_0,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel1T_1,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel2T_2,Class=Texture)"));
    assert!(manifest.contains("Object=(Name=SoldierSkins_Steel.stel2T_3,Class=Texture)"));
}

#[test]
fn test_skin_studio_dialog_rendering_all_tabs() {
    let mut state = EditorState::default();
    state.show_skin_studio_dialog = true;

    let ctx = egui::Context::default();

    for tab in 0..=5 {
        state.skin_studio_settings.active_tab = tab;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
        });
    }
}

#[test]
fn test_skin_studio_texture_cache_stability_and_zero_churn() {
    let mut state = EditorState::default();
    state.show_skin_studio_dialog = true;
    state.skin_studio_settings.active_tab = 0; // Live preview with all swatches & 3D character viewport

    let ctx = egui::Context::default();

    // Frame 1: Textures are allocated and cached
    let output1 = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    assert!(!state.skin_studio_settings.skin_texture_cache.0.is_empty(), "Cache should contain uploaded textures");
    assert!(!output1.textures_delta.set.is_empty(), "Frame 1 should upload textures");

    // Frame 2: Textures are reused from cache, so NO textures should be freed!
    let output2 = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    assert!(output2.textures_delta.free.is_empty(), "No textures should be freed on frame 2 (zero churn)");
    assert!(output2.textures_delta.set.is_empty(), "No new textures should be uploaded on identical frame 2");

    // Frame 3: Same stability test
    let output3 = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    assert!(output3.textures_delta.free.is_empty(), "Frame 3 should have 0 freed textures");
    assert!(output3.textures_delta.set.is_empty(), "Frame 3 should have 0 set textures");

    // Switch to Tab 2 (Textures & 4 Teams) and verify stability
    state.skin_studio_settings.active_tab = 2;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    let output_tab2_subsequent = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    assert!(output_tab2_subsequent.textures_delta.free.is_empty(), "Tab 2 should also not free textures per frame");

    // Switch to Tab 3 (Faces & TalkTextures) and verify stability
    state.skin_studio_settings.active_tab = 3;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    let output_tab3_subsequent = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::skin_studio_dialog::show_skin_studio_dialog(ctx, &mut state);
    });
    assert!(output_tab3_subsequent.textures_delta.free.is_empty(), "Tab 3 should also not free textures per frame");
}
