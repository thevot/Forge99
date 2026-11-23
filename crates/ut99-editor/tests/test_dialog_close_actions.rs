use ut99_editor::EditorState;
#[derive(Clone,Copy,Debug)]enum Dialog{Replace,Special,Ctf,Zone}
fn show(s:&mut EditorState,d:Dialog,value:bool){match d{Dialog::Replace=>s.show_replace_textures_dialog=value,Dialog::Special=>s.show_add_special_dialog=value,Dialog::Ctf=>s.show_ctf_symmetry_dialog=value,Dialog::Zone=>s.show_zone_seal_dialog=value}}
fn open(s:&EditorState,d:Dialog)->bool{match d{Dialog::Replace=>s.show_replace_textures_dialog,Dialog::Special=>s.show_add_special_dialog,Dialog::Ctf=>s.show_ctf_symmetry_dialog,Dialog::Zone=>s.show_zone_seal_dialog}}
fn title(d:Dialog)->&'static str{match d{Dialog::Replace=>"Find & Replace Textures (UnrealEd 2.0 Parity)",Dialog::Special=>"Add Special Brush - UnrealEd 2 Paradigm",Dialog::Ctf=>"Symmetrical CTF Base Generator",Dialog::Zone=>"Zone Boundary Leak Detector & Seal Tester"}}
fn frame(ctx:&egui::Context,s:&mut EditorState,d:Dialog,events:Vec<egui::Event>)->egui::FullOutput{
 let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1600.0,1600.0))),events,time:Some(ctx.input(|i|i.time)+0.1),..Default::default()};
 ctx.run(input,|ctx|match d{Dialog::Replace=>ut99_editor::gui::replace_textures_dialog::render_replace_textures_dialog(ctx,s),Dialog::Special=>ut99_editor::gui::special_brush_dialog::render_special_brush_dialog(ctx,s),Dialog::Ctf=>ut99_editor::gui::ctf_symmetry_dialog::show_ctf_symmetry_dialog(ctx,s),Dialog::Zone=>ut99_editor::gui::zone_seal_dialog::show_zone_seal_dialog(ctx,s)})
}
fn settle(ctx:&egui::Context,s:&mut EditorState,d:Dialog)->egui::FullOutput{let mut out=frame(ctx,s,d,vec![]);for _ in 0..8{out=frame(ctx,s,d,vec![]);}out}
fn click_pos(ctx:&egui::Context,s:&mut EditorState,d:Dialog,pos:egui::Pos2){frame(ctx,s,d,vec![egui::Event::PointerMoved(pos)]);for pressed in [true,false]{frame(ctx,s,d,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}}
fn click(ctx:&egui::Context,s:&mut EditorState,d:Dialog,label:&str){
 let output=settle(ctx,s,d);let pos=output.shapes.iter().find_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{let rect=egui::Rect::from_min_size(text.pos,text.galley.size());(text.galley.text()==label&&shape.clip_rect.contains_rect(rect)).then_some(rect.center())}else{None}).unwrap_or_else(||panic!("Visible button missing: {label} in {d:?}"));click_pos(ctx,s,d,pos);
}
fn setup(d:Dialog)->(egui::Context,EditorState){let ctx=egui::Context::default();let mut s=EditorState::default();s.new_map();s.history=Default::default();s.map_is_dirty=false;show(&mut s,d,true);settle(&ctx,&mut s,d);(ctx,s)}
fn check_close(d:Dialog,label:&str){
 let (ctx,mut s)=setup(d);let brushes=s.world_brushes.clone();let builder=s.builder_brush.clone();let actors=s.loaded_unr.as_ref().unwrap().actors.clone();
 click(&ctx,&mut s,d,label);assert!(!open(&s,d),"{d:?} did not close");settle(&ctx,&mut s,d);assert!(!open(&s,d));
 assert_eq!(s.world_brushes,brushes);assert_eq!(s.builder_brush,builder);assert_eq!(s.loaded_unr.as_ref().unwrap().actors,actors);assert!(s.history.undo_stack.is_empty());assert!(!s.map_is_dirty);
 show(&mut s,d,true);settle(&ctx,&mut s,d);assert!(open(&s,d));click(&ctx,&mut s,d,label);assert!(!open(&s,d));
}
#[test]fn replace_close_is_effective(){check_close(Dialog::Replace,"Close");}
#[test]fn special_cancel_is_effective(){check_close(Dialog::Special,"Cancel");}
#[test]fn ctf_close_is_effective(){check_close(Dialog::Ctf,"✖ Close");}
#[test]fn zone_close_is_effective(){check_close(Dialog::Zone,"✖ Close");}
#[test]
fn confirming_special_brush_adds_once_and_closes(){
 let d=Dialog::Special;let (ctx,mut s)=setup(d);let before=s.world_brushes.len();click(&ctx,&mut s,d,"Add Special Brush");
 assert!(!open(&s,d));assert_eq!(s.world_brushes.len(),before+1);assert_eq!(s.history.undo_stack.len(),1);
 settle(&ctx,&mut s,d);assert_eq!(s.world_brushes.len(),before+1);
 show(&mut s,d,true);settle(&ctx,&mut s,d);assert!(open(&s,d));assert_eq!(s.world_brushes.len(),before+1);
 click(&ctx,&mut s,d,"Cancel");s.undo();assert_eq!(s.world_brushes.len(),before);
}
#[test]
fn titlebar_x_closes_every_affected_dialog(){
 for d in [Dialog::Replace,Dialog::Special,Dialog::Ctf,Dialog::Zone]{
  let (ctx,mut s)=setup(d);let before=s.world_brushes.clone();let output=settle(&ctx,&mut s,d);
  let rect=ctx.memory(|memory|memory.area_rect(egui::Id::new(title(d)))).expect("window area");
  // Use the observed diagonal strokes of the actual titlebar close icon.
  let pos=output.shapes.iter().find_map(|shape|if let egui::epaint::Shape::LineSegment{points,..}=&shape.shape{
   let delta=points[1]-points[0];let center=points[0]+delta*0.5;
   (delta.x.abs()>4.0&&(delta.x.abs()-delta.y.abs()).abs()<1.0&&center.x>rect.right()-40.0&&center.y<rect.top()+35.0).then_some(center)
  }else{None}).unwrap_or_else(||panic!("Close icon not found in {d:?}: {rect:?}"));
  click_pos(&ctx,&mut s,d,pos);assert!(!open(&s,d),"{d:?}");settle(&ctx,&mut s,d);assert!(!open(&s,d));assert_eq!(s.world_brushes,before);assert!(s.history.undo_stack.is_empty());assert!(!s.map_is_dirty);
 }
}
#[test]
fn closing_does_not_reset_live_tool_settings(){
 for (d,label) in [(Dialog::Replace,"Close"),(Dialog::Special,"Cancel"),(Dialog::Ctf,"✖ Close"),(Dialog::Zone,"✖ Close")]{
  let (ctx,mut s)=setup(d);
  s.replace_texture_find="Audit.Wall".into();s.replace_texture_with="Audit.Floor".into();s.special_brush_flags=42;
  s.ctf_symmetry_config.center=glam::Vec3::new(128.0,256.0,512.0);s.ctf_symmetry_config.prefix_filter=Some("Audit".into());s.show_zone_leaks_visual=true;
  let before=s.world_brushes.clone();click(&ctx,&mut s,d,label);assert!(!open(&s,d));
  assert_eq!(s.replace_texture_find,"Audit.Wall");assert_eq!(s.replace_texture_with,"Audit.Floor");assert_eq!(s.special_brush_flags,42);
  assert_eq!(s.ctf_symmetry_config.center,glam::Vec3::new(128.0,256.0,512.0));assert_eq!(s.ctf_symmetry_config.prefix_filter.as_deref(),Some("Audit"));assert!(s.show_zone_leaks_visual);
  assert_eq!(s.world_brushes,before);assert!(s.history.undo_stack.is_empty());assert!(!s.map_is_dirty);
 }
}
