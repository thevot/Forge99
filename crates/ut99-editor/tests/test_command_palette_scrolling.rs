use ut99_editor::EditorState;
fn frame(ctx:&egui::Context,state:&mut EditorState,events:Vec<egui::Event>)->egui::FullOutput {
 let modifiers=events.iter().find_map(|event|if let egui::Event::Key{modifiers,..}=event{Some(*modifiers)}else{None}).unwrap_or_default();
 let size=ctx.data(|data|data.get_temp::<egui::Vec2>(egui::Id::new("test_screen_size"))).unwrap_or(egui::vec2(1280.0,900.0));
 let input=egui::RawInput{modifiers,screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,size)),events,time:Some(ctx.input(|i|i.time)+0.05),..Default::default()};
 ctx.run(input,|ctx|ut99_editor::gui::command_palette::render_command_palette(ctx,state))
}
fn labels(output:&egui::FullOutput)->Vec<(String,egui::Rect)> {
 output.shapes.iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape {
  let label=text.galley.text();let rect=egui::Rect::from_min_size(text.pos,text.galley.size());
  (label.contains('[')&&label.contains(']')&&shape.clip_rect.intersects(rect)).then_some((label.to_string(),rect.intersect(shape.clip_rect)))
 }else{None}).collect()
}
fn settle(ctx:&egui::Context,state:&mut EditorState)->egui::FullOutput {
 let mut output=frame(ctx,state,vec![]);for _ in 0..25{output=frame(ctx,state,vec![]);}output
}
fn setup()->(egui::Context,EditorState) {
 let ctx=egui::Context::default();let mut state=EditorState::default();state.show_command_palette=true;settle(&ctx,&mut state);(ctx,state)
}
fn key(ctx:&egui::Context,state:&mut EditorState,key:egui::Key,shift:bool) {
 for pressed in [true,false] {frame(ctx,state,vec![egui::Event::Key{key,physical_key:None,pressed,repeat:false,modifiers:egui::Modifiers{shift,..Default::default()}}]);}
}
fn scroll(ctx:&egui::Context,state:&mut EditorState) {
 frame(ctx,state,vec![egui::Event::PointerMoved(egui::pos2(640.0,400.0))]);settle(ctx,state);
 frame(ctx,state,vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-500.0),modifiers:egui::Modifiers::NONE}]);
}
#[test]
fn manual_scroll_survives_idle_frames() {
 let (ctx,mut state)=setup();let before=labels(&settle(&ctx,&mut state))[0].0.clone();
 scroll(&ctx,&mut state);let after=labels(&settle(&ctx,&mut state));
 assert_ne!(after[0].0,before,"idle frames must not undo manual scrolling");
 let later=labels(&settle(&ctx,&mut state));assert_eq!(later[0].0,after[0].0);
}
#[test]
fn hovering_a_visible_row_does_not_move_the_list() {
 let (ctx,mut state)=setup();let before=labels(&settle(&ctx,&mut state));
 let pos=before[3].1.left_top()+egui::vec2(12.0,6.0);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos)]);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos+egui::vec2(1.0,0.0))]);
 assert!(state.command_palette_selected_index>0,"pointer must hover a noninitial command");
 let after=labels(&settle(&ctx,&mut state));assert_eq!(before[0].0.trim_start_matches('▶').trim(),after[0].0.trim_start_matches('▶').trim());
}
#[test]
fn keyboard_navigation_reveals_selection_after_manual_scroll() {
 let (ctx,mut state)=setup();scroll(&ctx,&mut state);settle(&ctx,&mut state);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(egui::pos2(10.0,10.0))]);
 for _ in 0..20 {key(&ctx,&mut state,egui::Key::ArrowDown,false);}
 let visible=labels(&settle(&ctx,&mut state));assert_eq!(state.command_palette_selected_index,20);
 assert!(visible.iter().any(|(label,_)|label.starts_with('▶')));
 key(&ctx,&mut state,egui::Key::Tab,true);assert_eq!(state.command_palette_selected_index,19);
 key(&ctx,&mut state,egui::Key::Tab,false);assert_eq!(state.command_palette_selected_index,20);
}
#[test]
fn query_changes_reset_selection_enter_runs_selected_command_and_escape_cancels() {
 let (ctx,mut state)=setup();scroll(&ctx,&mut state);settle(&ctx,&mut state);
 frame(&ctx,&mut state,vec![egui::Event::Text("camera bookmarks manager".into())]);
 let output=settle(&ctx,&mut state);assert_eq!(state.command_palette_selected_index,0);
 assert!(labels(&output).iter().any(|(label,_)|label.contains("Camera Bookmarks Manager")));
 key(&ctx,&mut state,egui::Key::Enter,false);assert!(state.show_camera_bookmarks_dialog);assert!(!state.show_command_palette);
 assert_eq!(state.recent_palette_commands.first().map(String::as_str),Some("win_bookmarks"));
 state.show_command_palette=true;settle(&ctx,&mut state);
 key(&ctx,&mut state,egui::Key::Escape,false);assert!(!state.show_command_palette);assert!(state.command_palette_search.is_empty());
}
#[test]
fn empty_results_enter_is_safe_and_reopening_reveals_initial_selection() {
 let (ctx,mut state)=setup();scroll(&ctx,&mut state);settle(&ctx,&mut state);
 state.command_palette_search="zzzzzzzzzzzz-no-such-command".into();settle(&ctx,&mut state);
 let recent=state.recent_palette_commands.clone();key(&ctx,&mut state,egui::Key::Enter,false);
 assert!(state.show_command_palette);assert_eq!(state.recent_palette_commands,recent);
 key(&ctx,&mut state,egui::Key::Escape,false);state.show_command_palette=true;
 let output=settle(&ctx,&mut state);assert_eq!(state.command_palette_selected_index,0);
 assert!(labels(&output).iter().any(|(label,_)|label.starts_with('▶')));
}

#[test]
fn clicking_a_result_executes_that_command() {
 let (ctx,mut state)=setup();state.command_palette_search="engine budget profiler".into();
 let output=settle(&ctx,&mut state);
 let pos=labels(&output).into_iter().find(|(label,_)|label.contains("Open Engine Budget & Profiler")).unwrap().1.left_top()+egui::vec2(12.0,6.0);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos)]);
 for pressed in [true,false] {frame(&ctx,&mut state,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}
 assert!(state.show_map_budget_dialog);assert!(!state.show_command_palette);
 assert_eq!(state.recent_palette_commands.first().map(String::as_str),Some("win_budget_profiler"));
}
#[test]
fn compact_window_and_larger_text_keep_manual_and_keyboard_navigation() {
 let ctx=egui::Context::default();ctx.data_mut(|data|data.insert_temp(egui::Id::new("test_screen_size"),egui::vec2(800.0,600.0)));
 let mut style=(*ctx.style()).clone();
 for font in style.text_styles.values_mut(){font.size*=1.35;}ctx.set_style(style);
 let mut state=EditorState::default();state.show_command_palette=true;
 let before=labels(&settle(&ctx,&mut state));let pos=before[1].1.left_top()+egui::vec2(12.0,6.0);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos)]);settle(&ctx,&mut state);
 frame(&ctx,&mut state,vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:egui::vec2(0.0,-450.0),modifiers:egui::Modifiers::NONE}]);
 let after=labels(&settle(&ctx,&mut state));assert_ne!(before[0].0,after[0].0);
 frame(&ctx,&mut state,vec![egui::Event::PointerMoved(egui::pos2(5.0,5.0))]);
 key(&ctx,&mut state,egui::Key::ArrowDown,false);
 let visible=labels(&settle(&ctx,&mut state));assert!(visible.iter().any(|(label,_)|label.starts_with('▶')));
}
