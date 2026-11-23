use glam::Vec3;
use ut99_editor::{EditorState, history::Action, editor_state::PendingDirtyAction};
mod common;
fn moved() -> (EditorState, Vec3, Vec3) {
    let mut state=EditorState::default(); state.new_map();
    let old=state.world_brushes[0].location;
    let new=old+Vec3::new(64.0,0.0,0.0);
    state.world_brushes[0].location=new;
    state.history.push(Action::MoveBrush{brush_index:0,old_location:old,new_location:new});
    (state,old,new)
}
#[test]
fn undo_and_redo_after_real_save_mark_the_document_modified() {
    let dir=common::TestDir::new(); let path=dir.path().join("Saved.t3d");
    let (mut state,old,new)=moved();
    state.save_t3d(&path).unwrap(); assert!(!state.map_is_dirty);
    state.undo(); assert_eq!(state.world_brushes[0].location,old); assert!(state.map_is_dirty);
    assert_eq!(state.history.redo_stack.len(),1);
    state.save_t3d(&path).unwrap(); assert!(!state.map_is_dirty);
    state.redo(); assert_eq!(state.world_brushes[0].location,new); assert!(state.map_is_dirty);
    assert_eq!(state.history.undo_stack.len(),1);
    state.save_t3d(&path).unwrap(); assert!(!state.map_is_dirty);
}
#[test]
fn empty_history_and_invalid_or_identical_edits_preserve_dirty_state() {
    for dirty in [false,true] {
        let mut state=EditorState::default(); state.new_map(); state.map_is_dirty=dirty;
        state.undo(); state.redo(); assert_eq!(state.map_is_dirty,dirty);
        for index in [0,usize::MAX] {
            let current=state.world_brushes[0].location;
            let action=Action::MoveBrush{brush_index:index,old_location:current,new_location:current};
            state.apply_undo_action(&action); state.apply_redo_action(&action);
            assert_eq!(state.map_is_dirty,dirty);
        }
        let invalid=Action::UpdateLightEffects{actor_index:usize::MAX,old_actor:Default::default(),new_actor:Default::default(),description:"missing actor".into()};
        state.apply_undo_action(&invalid); state.apply_redo_action(&invalid);
        assert_eq!(state.map_is_dirty,dirty);
    }
}
#[test]
fn builder_only_and_empty_compound_actions_do_not_dirty_the_document() {
    let mut state=EditorState::default(); state.new_map();
    let old=state.builder_brush.clone(); let mut new=old.clone(); new.location.x+=32.0;
    let actions=vec![Action::ModifyBuilderBrush{old_brush:old,new_brush:new,description:"builder".into()}];
    state.history.push(Action::Compound{actions,description:"tools".into()});
    state.undo(); state.redo(); assert!(!state.map_is_dirty);
    let empty=Action::Compound{actions:vec![],description:"empty".into()};
    state.apply_undo_action(&empty); state.apply_redo_action(&empty); assert!(!state.map_is_dirty);
}
#[test]
fn mixed_compound_and_actor_transform_actions_mark_changes() {
    let (mut state,_,_)=moved(); let move_action=state.history.undo_stack.pop().unwrap();
    let builder=state.builder_brush.clone();
    state.history.push(Action::Compound{actions:vec![move_action,Action::ModifyBuilderBrush{old_brush:builder.clone(),new_brush:builder,description:"builder".into()}],description:"mixed".into()});
    state.map_is_dirty=false; state.undo(); assert!(state.map_is_dirty);
    state.map_is_dirty=false; state.redo(); assert!(state.map_is_dirty);
    let actor=ut99_format::ActorRecord::default();
    state.loaded_unr.as_mut().unwrap().actors.push(actor);
    let index=state.loaded_unr.as_ref().unwrap().actors.len()-1;
    let action=Action::MoveActor{actor_index:index,old_location:Vec3::X,new_location:Vec3::Y};
    state.map_is_dirty=false; state.apply_undo_action(&action); assert!(state.map_is_dirty);
    state.map_is_dirty=false; state.apply_redo_action(&action); assert!(state.map_is_dirty);
}
fn frame(ctx: &egui::Context, state: &mut EditorState, events: Vec<egui::Event>) -> egui::FullOutput {
    let input=egui::RawInput {
        screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1280.0,900.0))),
        events, time:Some(ctx.input(|i| i.time)+0.2), ..Default::default()
    };
    ctx.run(input, |ctx| ut99_editor::gui::render_ui(ctx,state))
}
fn click(ctx: &egui::Context, state: &mut EditorState, label: &str) {
    let output=frame(ctx,state,vec![]);
    let pos=output.shapes.iter().find_map(|shape| {
        if let egui::epaint::Shape::Text(text)=&shape.shape {
            if text.galley.text()==label {
                return Some(egui::Rect::from_min_size(text.pos,text.galley.size()).intersect(shape.clip_rect).center());
            }
        }
        None
    }).unwrap_or_else(||panic!("Missing visible command: {label}"));
    for pressed in [true,false] {
        frame(ctx,state,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton {
            pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE,
        }]);
    }
}
#[test]
fn undo_after_save_restores_open_new_and_exit_confirmation_and_cancel_preserves_history() {
    let dir=common::TestDir::new();
    for command in ["Open", "New Map", "Exit"] {
        let (mut state,_,_)=moved(); state.save_t3d(dir.path().join("Saved.t3d")).unwrap(); state.undo();
        state.save_feedback=None;
        let ctx=egui::Context::default();
        frame(&ctx,&mut state,vec![]);
        if command=="Open" { state.prompt_or_open_map_path(dir.path().join("Other.unr")); }
        else { click(&ctx,&mut state,"File"); click(&ctx,&mut state,command); }
        assert!(state.show_unsaved_exit_modal);
        assert!(state.map_is_dirty);
        assert!(matches!((&state.pending_dirty_action,command),
            (Some(PendingDirtyAction::OpenMapPath(_)),"Open") |
            (Some(PendingDirtyAction::NewMap),"New Map") |
            (Some(PendingDirtyAction::ExitApp),"Exit")));
        let before=state.world_brushes.clone();
        frame(&ctx,&mut state,vec![]);
        click(&ctx,&mut state,"Cancel");
        assert!(!state.show_unsaved_exit_modal && state.pending_dirty_action.is_none());
        assert_eq!(state.world_brushes,before); assert_eq!(state.history.redo_stack.len(),1);
        assert!(!state.confirm_exit_without_saving);
    }
}
#[test]
fn builder_commits_and_studio_previews_preserve_clean_state() {
    let mut state=EditorState::default(); state.new_map();
    let old=state.builder_brush.clone(); let mut brush=old.clone(); brush.location.x+=64.0;
    let action=Action::CommitLoftedBrush{brush_index:None,brush,is_builder_brush:true,old_builder_brush:Some(old),description:"Builder only".into()};
    state.apply_redo_action(&action); state.apply_undo_action(&action); assert!(!state.map_is_dirty);
    let config=state.water_surface_studio_settings.active_config.clone();
    let preset=state.water_surface_studio_settings.selected_preset;
    let action=Action::UpdateWaterSurfaceConfig{old_config:config.clone(),new_config:config,old_preset:preset,new_preset:preset,description:"preview".into()};
    state.apply_undo_action(&action); state.apply_redo_action(&action); assert!(!state.map_is_dirty);
}
#[test]
fn rotations_scales_textures_and_structural_edits_track_real_changes() {
    let mut state=EditorState::default(); state.new_map();
    let actions=vec![
        Action::RotateBrush{brush_index:0,old_rotation:Vec3::ZERO,new_rotation:Vec3::Z*90.0},
        Action::ScaleBrush{brush_index:0,old_scale:Vec3::ONE,new_scale:Vec3::splat(2.0)},
        Action::SetSurfaceTexture{brush_index:0,poly_index:0,old_texture:"Old.Texture".into(),new_texture:"New.Texture".into()},
    ];
    for action in actions {
        state.map_is_dirty=false; state.apply_redo_action(&action); assert!(state.map_is_dirty);
        state.map_is_dirty=false; state.apply_redo_action(&action); assert!(!state.map_is_dirty);
        state.apply_undo_action(&action); assert!(state.map_is_dirty);
    }
    let action=Action::DeleteBrush{index:0,brush:state.world_brushes[0].clone()};
    state.map_is_dirty=false; let count=state.world_brushes.len();
    state.apply_redo_action(&action); assert!(state.map_is_dirty); assert_eq!(state.world_brushes.len(),count-1);
    state.map_is_dirty=false; state.apply_undo_action(&action); assert!(state.map_is_dirty); assert_eq!(state.world_brushes.len(),count);
}
