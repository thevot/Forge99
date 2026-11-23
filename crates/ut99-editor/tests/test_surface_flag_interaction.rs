use ut99_editor::EditorState;
use ut99_bsp::{PF_UNLIT,PF_MASKED};
fn state()->EditorState{let mut s=EditorState::default();s.new_map();s.history=Default::default();s.map_is_dirty=false;s}
fn frame(ctx:&egui::Context,state:&mut EditorState,events:Vec<egui::Event>)->egui::FullOutput{
 let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1600.0,2400.0))),events,time:Some(ctx.input(|i|i.time)+0.1),..Default::default()};
 ctx.run(input,|ctx|ut99_editor::gui::surface_properties::render_surface_properties_dialog(ctx,state))
}
fn settle(ctx:&egui::Context,state:&mut EditorState)->egui::FullOutput{let mut out=frame(ctx,state,vec![]);for _ in 0..8{out=frame(ctx,state,vec![]);}out}
fn click(ctx:&egui::Context,state:&mut EditorState,label:&str){
 let out=settle(ctx,state);let pos=out.shapes.iter().find_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{(text.galley.text()==label).then_some(egui::Rect::from_min_size(text.pos,text.galley.size()).center())}else{None}).unwrap_or_else(||panic!("missing {label}"));
 frame(ctx,state,vec![egui::Event::PointerMoved(pos)]);for pressed in [true,false]{frame(ctx,state,vec![egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}
}
#[test]
fn mixed_ui_click_enables_all_then_clears_all_and_preserves_other_bits(){
 let ctx=egui::Context::default();let mut s=state();s.show_surface_properties=true;
 s.world_brushes[0].polys[0].flags=PF_UNLIT|PF_MASKED;s.world_brushes[0].polys[1].flags=PF_MASKED;
 s.selected_surfaces.extend([(0,0),(0,1)]);let untouched=s.world_brushes[0].polys[2].clone();
 click(&ctx,&mut s,"≈ Unlit");assert_eq!(s.world_brushes[0].polys[0].flags,PF_UNLIT|PF_MASKED);assert_eq!(s.world_brushes[0].polys[1].flags,PF_UNLIT|PF_MASKED);assert!(s.map_is_dirty);
 click(&ctx,&mut s,"Unlit");assert_eq!(s.world_brushes[0].polys[0].flags,PF_MASKED);assert_eq!(s.world_brushes[0].polys[1].flags,PF_MASKED);assert_eq!(s.world_brushes[0].polys[2],untouched);
 s.undo();assert_eq!(s.world_brushes[0].polys[1].flags,PF_UNLIT|PF_MASKED);s.undo();assert_eq!(s.world_brushes[0].polys[0].flags,PF_UNLIT|PF_MASKED);assert_eq!(s.world_brushes[0].polys[1].flags,PF_MASKED);
 s.redo();assert_eq!(s.world_brushes[0].polys[1].flags,PF_UNLIT|PF_MASKED);
}
#[test]
fn multiple_brushes_are_one_undo_step(){
 let mut s=state();let other=s.world_brushes.len();s.world_brushes.push(s.world_brushes[0].clone());
 s.selected_surfaces.extend([(0,0),(other,0)]);s.world_brushes[0].polys[0].flags=0;s.world_brushes[other].polys[0].flags=PF_MASKED;
 let before=s.world_brushes.clone();assert_eq!(s.set_selected_surfaces_flags(PF_UNLIT,0),2);assert_eq!(s.history.undo_stack.len(),1);assert!(s.map_is_dirty);
 s.undo();assert_eq!(s.world_brushes,before);s.redo();assert_eq!(s.world_brushes[0].polys[0].flags,PF_UNLIT);assert_eq!(s.world_brushes[other].polys[0].flags,PF_UNLIT|PF_MASKED);
}
#[test]
fn no_changes_and_invalid_targets_do_not_dirty_or_record_history(){
 let mut s=state();s.selected_surfaces.insert((0,0));s.world_brushes[0].polys[0].flags=PF_UNLIT;
 assert_eq!(s.set_selected_surfaces_flags(PF_UNLIT,0),0);assert!(!s.map_is_dirty);assert!(s.history.undo_stack.is_empty());
 s.selected_surfaces.clear();s.selected_surfaces.extend([(usize::MAX,0),(0,usize::MAX)]);let builder=s.builder_brush.clone();let brushes=s.world_brushes.clone();
 assert_eq!(s.set_selected_surfaces_flags(PF_UNLIT,0),0);assert!(!s.map_is_dirty);assert!(s.history.undo_stack.is_empty());assert_eq!(s.builder_brush,builder);assert_eq!(s.world_brushes,brushes);
}
#[test]
fn builder_flags_are_undoable_without_dirtying_the_map(){
 let mut s=state();s.selected_brush_index=None;s.selected_surfaces.clear();let old=s.builder_brush.clone();let map=s.world_brushes.clone();
 s.set_selected_surfaces_flags(PF_UNLIT,0);assert!(!s.map_is_dirty);assert_eq!(s.history.undo_stack.len(),1);assert!(s.builder_brush.polys.iter().all(|p|p.flags&PF_UNLIT!=0));
 s.undo();assert_eq!(s.builder_brush,old);assert!(!s.map_is_dirty);s.redo();assert!(s.builder_brush.polys.iter().all(|p|p.flags&PF_UNLIT!=0));assert!(!s.map_is_dirty);assert_eq!(s.world_brushes,map);
}
#[test]
fn brush_summary_reads_all_faces_and_ui_uniforms_the_whole_brush(){
 let ctx=egui::Context::default();let mut s=state();s.selected_brush_index=Some(0);s.show_surface_properties=true;
 for poly in &mut s.world_brushes[0].polys{poly.flags=PF_MASKED;}s.world_brushes[0].polys[1].flags|=PF_UNLIT;
 click(&ctx,&mut s,"≈ Unlit");assert!(s.world_brushes[0].polys.iter().all(|p|p.flags==PF_MASKED|PF_UNLIT));
}
#[test]
fn invalid_ui_targets_are_disabled_and_do_not_fall_back_to_builder(){
 let ctx=egui::Context::default();let mut s=state();s.show_surface_properties=true;s.selected_surfaces.insert((usize::MAX,usize::MAX));
 let before=s.builder_brush.clone();assert_eq!(s.selected_surface_flags_summary(),(0,0,0));click(&ctx,&mut s,"Unlit");
 assert_eq!(s.builder_brush,before);assert!(!s.map_is_dirty);assert!(s.history.undo_stack.is_empty());
 s.selected_surfaces.clear();s.selected_brush_index=Some(usize::MAX);assert_eq!(s.selected_surface_flags_summary(),(0,0,0));assert_eq!(s.set_selected_surfaces_flags(PF_UNLIT,0),0);
}
#[test]
fn builder_summary_covers_all_faces_and_noop_preserves_redo(){
 let mut s=state();s.selected_brush_index=None;for poly in &mut s.builder_brush.polys{poly.flags=0;}s.builder_brush.polys[0].flags=PF_UNLIT;
 assert_eq!(s.selected_surface_flags_summary(),(0,PF_UNLIT,s.builder_brush.polys.len()));
 s.builder_brush.polys.clear();assert_eq!(s.selected_surface_flags_summary(),(0,0,0));assert_eq!(s.set_selected_surfaces_flags(PF_UNLIT,0),0);assert!(s.history.undo_stack.is_empty());
 s.selected_surfaces.insert((0,0));s.world_brushes[0].polys[0].flags=PF_MASKED;
 s.set_selected_surfaces_flags(PF_UNLIT,0);s.undo();s.map_is_dirty=false;
 assert_eq!(s.history.redo_stack.len(),1);assert_eq!(s.set_selected_surfaces_flags(PF_MASKED,0),0);assert_eq!(s.history.redo_stack.len(),1);assert!(s.history.undo_stack.is_empty());assert!(!s.map_is_dirty);
}
#[test]
fn console_clear_wins_and_legacy_xor_keeps_its_semantics(){
 let mut s=state();s.selected_surfaces.extend([(0,0),(0,1)]);s.world_brushes[0].polys[0].flags=0;s.world_brushes[0].polys[1].flags=0;
 let command=format!("POLY SET SETFLAGS={} CLEARFLAGS={}",PF_UNLIT|PF_MASKED,PF_UNLIT);
 let result=s.execute_unrealed_command(&command);assert!(result.contains("2 surface(s) updated"));
 assert_eq!(s.world_brushes[0].polys[0].flags,PF_MASKED);assert_eq!(s.world_brushes[0].polys[1].flags,PF_MASKED);assert_eq!(s.history.undo_stack.len(),1);
 s.undo();assert_eq!(s.world_brushes[0].polys[0].flags,0);assert_eq!(s.world_brushes[0].polys[1].flags,0);
 s.world_brushes[0].polys[0].flags=PF_UNLIT;s.toggle_selected_brush_poly_flag(PF_UNLIT);
 assert_eq!(s.world_brushes[0].polys[0].flags,0);assert_eq!(s.world_brushes[0].polys[1].flags,PF_UNLIT);
}
