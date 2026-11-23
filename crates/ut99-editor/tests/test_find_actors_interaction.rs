use glam::Vec3;
use ut99_editor::{EditorState,editor_state::FindActorCategory};
fn frame(ctx:&egui::Context,state:&mut EditorState,events:Vec<egui::Event>)->egui::FullOutput {
 let size=ctx.data(|data|data.get_temp::<egui::Vec2>(egui::Id::new("test_find_size"))).unwrap_or(egui::vec2(1280.0,900.0));
 let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,size)),events,time:Some(ctx.input(|i|i.time)+0.1),..Default::default()};
 ctx.run(input,|ctx|{
  ut99_editor::gui::find_actors_dialog::render_find_actors_dialog(ctx,state);
  if ctx.data(|data|data.get_temp::<bool>(egui::Id::new("test_other_window"))).unwrap_or(false){
   let mut text=ctx.data(|data|data.get_temp::<String>(egui::Id::new("test_other_text"))).unwrap_or_default();
   egui::Window::new("Other editor").order(egui::Order::Foreground).show(ctx,|ui|{
    ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::new("other_search"))).request_focus();
   });
   ctx.data_mut(|data|data.insert_temp(egui::Id::new("test_other_text"),text));
  }
 })
}
fn settle(ctx:&egui::Context,state:&mut EditorState)->egui::FullOutput {let mut output=frame(ctx,state,vec![]);for _ in 0..12{output=frame(ctx,state,vec![]);}output}
fn key(ctx:&egui::Context,state:&mut EditorState,key:egui::Key){for pressed in [true,false]{frame(ctx,state,vec![egui::Event::Key{key,physical_key:None,pressed,repeat:false,modifiers:egui::Modifiers::NONE}]);}}
fn setup()->(egui::Context,EditorState){
 let ctx=egui::Context::default();let mut state=EditorState::default();state.new_map();
 state.loaded_unr.as_mut().unwrap().actors=(0..3).map(|i|{let mut actor=ut99_format::ActorRecord::default();actor.name=format!("Audit{i}");actor.class_name=if i==2{"Trigger"}else{"Light"}.into();actor.location=Vec3::new(i as f32*1000.0,1000.0,2000.0);actor}).collect();
 state.selected_actor_index=Some(2);state.show_find_actors_dialog=true;settle(&ctx,&mut state);(ctx,state)
}
fn click(ctx:&egui::Context,state:&mut EditorState,label:&str){
 let output=settle(ctx,state);let pos=output.shapes.iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{(text.galley.text()==label).then_some(egui::Rect::from_min_size(text.pos,text.galley.size()).intersect(shape.clip_rect).center())}else{None}).last().unwrap_or_else(||panic!("Missing control {label}"));
 frame(ctx,state,vec![egui::Event::PointerMoved(pos)]);
 for pressed in [true,false]{frame(ctx,state,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}
}
#[test]
fn opening_accepts_text_immediately_and_enter_uses_the_filtered_result(){
 let (ctx,mut state)=setup();let camera=state.camera.position;
 frame(&ctx,&mut state,vec![egui::Event::Text("Audit1".into())]);
 assert_eq!(state.find_actors_query,"Audit1");assert_eq!(state.camera.position,camera);assert_eq!(state.selected_actor_index,Some(2));
 key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,Some(1));assert_ne!(state.camera.position,camera);
}
#[test]
fn arrows_and_single_click_only_highlight_until_confirmation(){
 let (ctx,mut state)=setup();let camera=state.camera.position;
 key(&ctx,&mut state,egui::Key::ArrowUp);assert_eq!(state.camera.position,camera);assert_eq!(state.selected_actor_index,Some(2));
 key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,Some(1));
 let camera=state.camera.position;click(&ctx,&mut state,"Audit0");assert_eq!(state.selected_actor_index,Some(1));assert_eq!(state.camera.position,camera);
 settle(&ctx,&mut state);
 key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,Some(0));
}
#[test]
fn empty_results_escape_and_reopen_do_not_change_scene_selection(){
 let (ctx,mut state)=setup();let camera=state.camera.position;
 state.find_actors_query="no-such-actor".into();settle(&ctx,&mut state);key(&ctx,&mut state,egui::Key::Enter);
 assert_eq!(state.selected_actor_index,Some(2));assert_eq!(state.camera.position,camera);
 key(&ctx,&mut state,egui::Key::Escape);assert!(!state.show_find_actors_dialog);
 state.find_actors_query.clear();state.show_find_actors_dialog=true;settle(&ctx,&mut state);
 frame(&ctx,&mut state,vec![egui::Event::Text("Audit0".into())]);assert_eq!(state.find_actors_query,"Audit0");
 assert_eq!(state.selected_actor_index,Some(2));assert_eq!(state.camera.position,camera);
}
#[test]
fn category_changes_and_removed_actors_cannot_confirm_an_obsolete_index(){
 let (ctx,mut state)=setup();state.find_actors_category=FindActorCategory::Lights;settle(&ctx,&mut state);
 key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,Some(0));
 key(&ctx,&mut state,egui::Key::ArrowDown);
 state.loaded_unr.as_mut().unwrap().actors.remove(0);state.selected_actor_index=None;
 settle(&ctx,&mut state);key(&ctx,&mut state,egui::Key::Enter);
 assert_eq!(state.selected_actor_index,Some(0));assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].name,"Audit1");
 state.map_path=Some("Other.unr".into());state.loaded_unr.as_mut().unwrap().actors.clear();settle(&ctx,&mut state);
 let camera=state.camera.position;state.selected_actor_index=None;key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,None);assert_eq!(state.camera.position,camera);
}
#[test]
fn explicit_buttons_double_click_and_batch_selection_still_work(){
 let (ctx,mut state)=setup();state.find_actors_query="Audit0".into();settle(&ctx,&mut state);
 click(&ctx,&mut state,"🎯 Focus");assert_eq!(state.selected_actor_index,Some(0));
 state.find_actors_query="Audit1".into();settle(&ctx,&mut state);
 click(&ctx,&mut state,"⚙ Props");assert!(state.show_actor_properties);assert_eq!(state.selected_actor_index,Some(1));
 state.show_actor_properties=false;state.find_actors_query="Audit2".into();settle(&ctx,&mut state);
 // Send a direct pair within the double-click interval.
 let output=settle(&ctx,&mut state);let pos=output.shapes.iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{(text.galley.text()=="Audit2" && shape.clip_rect.contains_rect(egui::Rect::from_min_size(text.pos,text.galley.size()))).then_some(egui::Rect::from_min_size(text.pos,text.galley.size()).center())}else{None}).last().unwrap();
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos)]);
 for _ in 0..2{for pressed in [true,false]{frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}}
 assert_eq!(state.selected_actor_index,Some(2));
 state.find_actors_query.clear();state.find_actors_category=FindActorCategory::Lights;settle(&ctx,&mut state);
 let camera=state.camera.position;click(&ctx,&mut state,"⭐ Select All Matches");assert!(state.is_actor_selected(0)&&state.is_actor_selected(1));assert!(!state.is_actor_selected(2));assert_eq!(state.camera.position,camera);
 click(&ctx,&mut state,"Deselect All");assert!(state.selected_actor_index.is_none());
}
#[test]
fn background_find_window_does_not_handle_another_editors_keys_or_steal_focus(){
 let (ctx,mut state)=setup();let camera=state.camera.position;
 ctx.data_mut(|data|data.insert_temp(egui::Id::new("test_other_window"),true));settle(&ctx,&mut state);
 frame(&ctx,&mut state,vec![egui::Event::Text("other".into())]);
 key(&ctx,&mut state,egui::Key::ArrowUp);key(&ctx,&mut state,egui::Key::Enter);key(&ctx,&mut state,egui::Key::Escape);
 assert!(state.show_find_actors_dialog);assert_eq!(state.find_actors_query,"");assert_eq!(state.selected_actor_index,Some(2));assert_eq!(state.camera.position,camera);
 assert_eq!(ctx.data(|data|data.get_temp::<String>(egui::Id::new("test_other_text"))).as_deref(),Some("other"));
}
#[test]
fn typing_and_confirmation_work_in_compact_window_with_larger_text(){
 let (ctx,mut state)=setup();ctx.data_mut(|data|data.insert_temp(egui::Id::new("test_find_size"),egui::vec2(800.0,600.0)));
 let mut style=(*ctx.style()).clone();for font in style.text_styles.values_mut(){font.size*=1.3;}ctx.set_style(style);
 settle(&ctx,&mut state);frame(&ctx,&mut state,vec![egui::Event::Text("Audit0".into())]);
 assert_eq!(state.find_actors_query,"Audit0");key(&ctx,&mut state,egui::Key::Enter);assert_eq!(state.selected_actor_index,Some(0));
}
fn visible_names(output:&egui::FullOutput)->Vec<String>{
 output.shapes.iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{
  (text.galley.text().starts_with("Audit")&&shape.clip_rect.contains_rect(egui::Rect::from_min_size(text.pos,text.galley.size()))).then_some(text.galley.text().to_string())
 }else{None}).collect()
}
#[test]
fn distant_keyboard_results_are_visible_and_manual_scroll_is_preserved(){
 let (ctx,mut state)=setup();let sample=state.loaded_unr.as_ref().unwrap().actors[0].clone();
 state.loaded_unr.as_mut().unwrap().actors=(0..60).map(|i|{let mut actor=sample.clone();actor.name=format!("Audit{i:03}");actor}).collect();
 let camera=state.camera.position;settle(&ctx,&mut state);
 for _ in 0..25{key(&ctx,&mut state,egui::Key::ArrowDown);}
 let output=settle(&ctx,&mut state);let before=visible_names(&output);assert!(before.iter().any(|name|name=="Audit025"));
 assert_eq!(state.selected_actor_index,Some(2));assert_eq!(state.camera.position,camera);
 let pos=output.shapes.iter().find_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{(text.galley.text()=="Audit025").then_some(text.pos+egui::vec2(10.0,6.0))}else{None}).unwrap();
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos)]);
 frame(&ctx,&mut state,vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-400.0),modifiers:egui::Modifiers::NONE}]);
 let after=visible_names(&settle(&ctx,&mut state));assert!(!after.is_empty());assert_ne!(after[0],before[0]);
 let later=visible_names(&settle(&ctx,&mut state));assert_eq!(after,later);assert_eq!(state.camera.position,camera);
}
