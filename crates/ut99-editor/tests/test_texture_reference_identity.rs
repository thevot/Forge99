use ut99_editor::texture_manager::TextureManager;
use ut99_format::{UtxPackage,UtxTexture};
mod common;
fn tex(group:&str,color:[u8;4],width:u32)->UtxTexture {
 UtxTexture{name:"Panel".into(),group:group.into(),width,height:1,format:1,masked:false,rgba_pixels:color.repeat(width as usize)}
}
fn manager()->TextureManager {
 let mut m=TextureManager::default();
 m.insert_package("AuditPack".into(),UtxPackage{package_name:"AuditPack".into(),textures:vec![tex("Walls",[255,0,0,255],1),tex("Floors",[0,255,0,255],2),tex("Nested.Detail",[0,0,255,255],3)]}); m
}
#[test]
fn explicit_references_respect_package_group_case_and_delimiters() {
 let m=manager();
 for name in ["AuditPack.Floors.Panel","auditpack.floors.panel","Texture'AuditPack.Floors.Panel'","\"AuditPack.Floors.Panel\""] {
  assert_eq!(m.find_loaded_texture(name).unwrap().width,2,"{name}");
 }
 assert_eq!(m.find_loaded_texture("AuditPack.Nested.Detail.Panel").unwrap().width,3);
 for name in ["MissingPack.Panel","AuditPack.Missing.Panel","AuditPack..Panel","None","Texture\'","Texture\'\'","\"\""] { assert!(m.find_loaded_texture(name).is_none(),"{name}"); }
 assert_eq!(m.sample_texture_rgba("AuditPack.Floors.Panel",0.0,0.0),Some([0.0,1.0,0.0,1.0]));
}
#[test]
fn ambiguous_short_names_do_not_depend_on_package_insertion_order() {
 for reverse in [false,true] {
  let mut m=manager(); assert!(m.find_loaded_texture("Panel").is_none()); assert!(m.find_loaded_texture("AuditPack.Panel").is_none());
  let mut packages=vec!["First","Second"]; if reverse {packages.reverse();}
  for p in packages { m.insert_package(p.into(),UtxPackage{package_name:p.into(),textures:vec![tex("",[1,2,3,255],1)]}); }
  assert!(m.find_loaded_texture("Panel").is_none());
  assert_eq!(m.find_loaded_texture("First.Panel").unwrap().group,"");
 }
}
#[test]
fn unique_legacy_names_and_mylevel_remain_supported() {
 let mut m=TextureManager::default(); m.insert_package("MyLevel".into(),UtxPackage{package_name:"MyLevel".into(),textures:vec![tex("Walls",[1,2,3,255],1)]});
 for name in ["Panel","MyLevel.Panel","MyLevel.Walls.Panel"] {assert!(m.find_loaded_texture(name).is_some());}
 assert_eq!(m.get_texture_dimensions_by_ref("Texture'MyLevel.Walls.Panel'"),Some((1,1)));
}
#[test]
fn thumbnail_keys_distinguish_groups_and_share_case_aliases() {
 let mut m=manager();let ctx=egui::Context::default();
 let walls=m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Walls.Panel").unwrap();
 let floors=m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Floors.Panel").unwrap();
 assert_ne!(walls.id(),floors.id());
 assert_eq!(floors.id(),m.get_or_create_egui_texture_by_ref(&ctx,"texture'auditpack.floors.panel'").unwrap().id());
 assert_eq!(m.cached_thumbnail_count(),2); assert_eq!(m.thumbnails_memory_usage_bytes(),12);
 assert!(m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Panel").is_none());
 assert_eq!(m.get_texture_dimensions_by_ref("AuditPack.Floors.Panel"),Some((2,1)));
}
#[test]
fn updates_conversion_and_eviction_preserve_other_groups() {
 let mut m=manager();let ctx=egui::Context::default();
 let wall=m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Walls.Panel").unwrap().id();
 let floor=m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Floors.Panel").unwrap().id();
 m.upsert_texture("auditpack",tex("floors",[255,255,0,255],2));
 assert_eq!(m.get_package("AuditPack").unwrap().textures.len(),3);
 assert_eq!(m.cached_thumbnail_count(),1);assert_eq!(m.thumbnails_memory_usage_bytes(),4);
 assert_eq!(wall,m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Walls.Panel").unwrap().id());
 assert_ne!(floor,m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Floors.Panel").unwrap().id());
 m.convert_texture_format("AUDITPACK","Floors.Panel",0).unwrap();
 assert_eq!(m.get_texture("AuditPack","Walls.Panel").unwrap().format,1);
 assert_eq!(m.get_texture("AuditPack","Floors.Panel").unwrap().format,0);
 assert_eq!(m.cached_thumbnail_count(),1);assert_eq!(m.thumbnails_memory_usage_bytes(),4);
 m.unload_package("auditpack");assert_eq!(m.cached_thumbnail_count(),0);assert_eq!(m.thumbnails_memory_usage_bytes(),0);
 m.insert_package("auditpack".into(),UtxPackage{package_name:"AuditPack".into(),textures:vec![tex("Walls",[255,0,0,255],1)]});
 assert!(m.get_or_create_egui_texture_by_ref(&ctx,"AuditPack.Walls.Panel").is_some());
 assert_eq!(m.available_packages().iter().filter(|name|name.eq_ignore_ascii_case("auditpack")).count(),1);
}
#[test]
fn exporting_a_selected_group_writes_its_pixels() {
 let m=manager(); let dir=common::TestDir::new(); let path=dir.path().join("selected.png");
 m.export_texture_to_file("AuditPack","Floors.Panel",&path).unwrap();
 let image=image::open(&path).unwrap().to_rgba8();assert_eq!(image.dimensions(),(2,1));assert_eq!(image.get_pixel(0,0).0,[0,255,0,255]);
 assert!(m.export_texture_to_file("AuditPack","Panel",&path).is_err());
}
#[test]
fn ungrouped_loader_sentinel_and_package_replacement_keep_cache_coherent() {
 let mut m=TextureManager::default();let ctx=egui::Context::default();
 m.insert_package("MyLevel".into(),UtxPackage{package_name:"MyLevel".into(),textures:vec![tex("None",[1,2,3,255],1)]});
 assert_eq!(m.resolve_texture_reference("MyLevel.Panel").as_deref(),Some("MyLevel.Panel"));
 let _ = m.get_or_create_egui_texture_by_ref(&ctx,"MyLevel.Panel").unwrap();
 let mut meta=ut99_editor::texture_manager::TextureSurfaceMetadata::default();meta.friction=0.5;
 m.set_texture_metadata("MYLEVEL.PANEL",meta);
 assert_eq!(m.get_texture_metadata("Texture'MyLevel.Panel'").friction,0.5);
 m.insert_package("mylevel".into(),UtxPackage{package_name:"MyLevel".into(),textures:vec![tex("",[4,5,6,255],2)]});
 assert_eq!(m.cached_thumbnail_count(),0);assert_eq!(m.thumbnails_memory_usage_bytes(),0);
 assert_eq!(m.loaded_package_count(),1);
 assert_eq!(m.get_texture_dimensions_by_ref("MyLevel.Panel"),Some((2,1)));
}
fn browser_frame(ctx:&egui::Context,state:&mut ut99_editor::EditorState,events:Vec<egui::Event>)->egui::FullOutput {
 let input=egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1280.0,1000.0))),events,time:Some(ctx.input(|i|i.time)+0.2),..Default::default()};
 ctx.run(input,|ctx|ut99_editor::gui::texture_browser::render_texture_browser(ctx,state))
}
#[test]
fn browser_selection_application_and_in_use_filter_preserve_group_identity() {
 let mut state=ut99_editor::EditorState::default();state.new_map();state.texture_manager=manager();
 state.show_texture_browser=true;state.selected_texture_package="AuditPack".into();state.selected_texture_group="Floors".into();
 let ctx=egui::Context::default();browser_frame(&ctx,&mut state,vec![]);
 let output=browser_frame(&ctx,&mut state,vec![]);
 let pos=output.shapes.iter().find_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape {
  (text.galley.text()=="Panel").then_some(text.pos+egui::vec2(25.0,-45.0))
 }else{None}).expect("visible floor texture");
 for pressed in [true,false] {browser_frame(&ctx,&mut state,vec![egui::Event::PointerMoved(pos),egui::Event::PointerButton{pos,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]);}
 assert_eq!(state.active_texture.as_deref(),Some("AuditPack.Floors.Panel"));
 state.select_surface(0,0,false);state.apply_active_texture_to_current_selection();
 assert_eq!(state.world_brushes[0].polys[0].texture,"AuditPack.Floors.Panel");
 // A foreign package's short-name collision must not appear in this package's In Use view.
 for brush in &mut state.world_brushes { for poly in &mut brush.polys {poly.texture="MissingPack.Panel".into();} }
 state.selected_texture_group="In Use".into();
 let output=browser_frame(&ctx,&mut state,vec![]);
 let labels:Vec<&str>=output.shapes.iter().filter_map(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{Some(text.galley.text())}else{None}).collect();
 assert!(labels.iter().any(|label|label.contains("0 texture(s)")),"{labels:?}");
 state.world_brushes[0].polys[0].texture="AuditPack.Floors.Panel".into();
 let output=browser_frame(&ctx,&mut state,vec![]);
 assert!(output.shapes.iter().any(|shape|if let egui::epaint::Shape::Text(text)=&shape.shape{text.galley.text().contains("1 texture(s)")}else{false}));
}
#[test]
#[ignore = "requires a graphics adapter; run with --ignored"]
fn viewport_uploads_correct_group_and_refreshes_after_namespace_changes() {
 let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::PRIMARY,..Default::default()});
 let adapter=pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions{compatible_surface:None,force_fallback_adapter:false,power_preference:wgpu::PowerPreference::LowPower})).expect("graphics adapter");
 println!("GPU: {:?}",adapter.get_info());
 let (device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{label:Some("texture identity regression"),required_features:wgpu::Features::empty(),required_limits:wgpu::Limits::default(),memory_hints:Default::default()},None)).unwrap();
 let mut renderer=ut99_editor::renderer::Renderer::new(&device,wgpu::TextureFormat::Bgra8UnormSrgb,64,64);
 // Give the upload target readback permission in this test only; no draw calls
 // or production texture usage flags are changed.
 renderer.texture_array=device.create_texture(&wgpu::TextureDescriptor {
  label:Some("readable upload test target"),size:renderer.texture_array.size(),mip_level_count:1,
  sample_count:1,dimension:wgpu::TextureDimension::D2,format:renderer.texture_array.format(),
  usage:renderer.texture_array.usage()|wgpu::TextureUsages::COPY_SRC,view_formats:&[],
 });
 let mut state=ut99_editor::EditorState::default();state.texture_manager=manager();
 let (layer,width,_)=renderer.resolve_or_upload_texture_layer(&queue,&state,"AuditPack.Floors.Panel");
 assert_ne!(layer,0);assert_eq!(width,2.0);
 assert_eq!(read_gpu_pixel(&device,&queue,&renderer.texture_array,layer),[0,255,0,255]);
 state.texture_manager.upsert_texture("AuditPack",tex("Floors",[0,0,255,255],2));
 let (layer,_,_)=renderer.resolve_or_upload_texture_layer(&queue,&state,"AuditPack.Floors.Panel");
 assert_eq!(read_gpu_pixel(&device,&queue,&renderer.texture_array,layer),[0,0,255,255]);
 state.texture_manager=TextureManager::default();
 state.texture_manager.insert_package("First".into(),UtxPackage{package_name:"First".into(),textures:vec![tex("",[255,0,0,255],1)]});
 assert_ne!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Panel").0,0);
 state.texture_manager.insert_package("Second".into(),UtxPackage{package_name:"Second".into(),textures:vec![tex("",[0,255,0,255],1)]});
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Panel").0,0);
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Missing.Panel").0,0);
}
fn read_gpu_pixel(device:&wgpu::Device,queue:&wgpu::Queue,texture:&wgpu::Texture,layer:u32)->[u8;4] {
 let buffer=device.create_buffer(&wgpu::BufferDescriptor{label:None,size:256,usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
 let mut encoder=device.create_command_encoder(&Default::default());
 encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo{texture,mip_level:0,origin:wgpu::Origin3d{x:0,y:0,z:layer},aspect:wgpu::TextureAspect::All},wgpu::TexelCopyBufferInfo{buffer:&buffer,layout:wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(256),rows_per_image:Some(1)}},wgpu::Extent3d{width:1,height:1,depth_or_array_layers:1});
 queue.submit(Some(encoder.finish()));
 let (tx,rx)=std::sync::mpsc::channel();buffer.slice(..).map_async(wgpu::MapMode::Read,move|result|{tx.send(result).unwrap();});
 device.poll(wgpu::Maintain::Wait);rx.recv().unwrap().unwrap();
 let bytes=buffer.slice(..).get_mapped_range();let pixel=bytes[..4].try_into().unwrap();drop(bytes);buffer.unmap();pixel
}
