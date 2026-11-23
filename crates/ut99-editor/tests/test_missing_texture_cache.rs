use ut99_editor::{EditorState, renderer::{Renderer,pipeline::MAX_DIFFUSE_TEXTURE_LAYERS},texture_manager::TextureManager};
use ut99_format::{UtxPackage,UtxTexture};
fn texture(name:&str,group:&str,pixel:[u8;4])->UtxTexture{UtxTexture{name:name.into(),group:group.into(),width:1,height:1,format:1,masked:false,rgba_pixels:pixel.to_vec()}}
fn setup()->(wgpu::Device,wgpu::Queue,Renderer,EditorState){
 let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::PRIMARY,..Default::default()});
 let adapter=pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions{compatible_surface:None,force_fallback_adapter:false,power_preference:wgpu::PowerPreference::LowPower})).expect("graphics adapter");
 let (device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{label:Some("missing texture cache regression"),required_features:wgpu::Features::empty(),required_limits:wgpu::Limits::default(),memory_hints:Default::default()},None)).unwrap();
 let renderer=Renderer::new(&device,wgpu::TextureFormat::Bgra8UnormSrgb,64,64);let mut state=EditorState::default();state.texture_manager=TextureManager::default();(device,queue,renderer,state)
}
#[test]
#[ignore="requires a graphics adapter; run with --ignored"]
fn missing_and_invalid_textures_are_retried_after_import_or_repair(){
 let (device,queue,mut renderer,mut state)=setup();
 renderer.texture_array=device.create_texture(&wgpu::TextureDescriptor{label:None,size:renderer.texture_array.size(),mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:renderer.texture_array.format(),usage:renderer.texture_array.usage()|wgpu::TextureUsages::COPY_SRC,view_formats:&[]});
 for name in ["Audit.Walls.Later","AUDIT.WALLS.LATER","Audit.Walls.Later"]{assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,name),(0,256.0,256.0));}
 assert!(renderer.uploaded_texture_layers.is_empty());assert_eq!(renderer.next_texture_layer,1);
 state.texture_manager.upsert_texture("Audit",texture("Later","Walls",[0,255,0,255]));
 let result=renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Walls.Later");assert_ne!(result.0,0);assert_eq!((result.1,result.2),(1.0,1.0));
 assert_eq!(read_pixel(&device,&queue,&renderer.texture_array,result.0),[0,255,0,255]);
 let mut broken=texture("Broken","Walls",[255;4]);broken.rgba_pixels.clear();state.texture_manager.upsert_texture("Audit",broken);
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Walls.Broken").0,0);
 state.texture_manager.upsert_texture("Audit",texture("Broken","Walls",[255,0,0,255]));
 let layer=renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Walls.Broken").0;assert_ne!(layer,0);assert_eq!(read_pixel(&device,&queue,&renderer.texture_array,layer),[255,0,0,255]);
}
#[test]
#[ignore="requires a graphics adapter; run with --ignored"]
fn ambiguous_names_and_group_misses_are_retried_after_namespace_changes(){
 let (_device,queue,mut renderer,mut state)=setup();
 for package in ["First","Second"]{state.texture_manager.insert_package(package.into(),UtxPackage{package_name:package.into(),textures:vec![texture("Panel","Walls",[255;4])]});}
 for _ in 0..3{assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Panel").0,0);}
 state.texture_manager.unload_package("Second");assert_ne!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Panel").0,0);
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"First.Floors.Panel").0,0);
 state.texture_manager.upsert_texture("First",texture("Panel","Floors",[0,255,0,255]));
 assert_ne!(renderer.resolve_or_upload_texture_layer(&queue,&state,"First.Floors.Panel").0,0);
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"First.Panel").0,0);
}
#[test]
#[ignore="requires a graphics adapter; run with --ignored"]
fn atlas_capacity_fallback_does_not_poison_future_lookups(){
 let (_device,queue,mut renderer,mut state)=setup();state.texture_manager.upsert_texture("Audit",texture("Valid","Walls",[255;4]));
 // Synchronize the renderer's namespace revision before simulating full capacity.
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Missing").0,0);
 renderer.next_texture_layer=MAX_DIFFUSE_TEXTURE_LAYERS;
 assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Walls.Valid").0,0);
 renderer.next_texture_layer=1;
 assert_ne!(renderer.resolve_or_upload_texture_layer(&queue,&state,"Audit.Walls.Valid").0,0);
}
fn read_pixel(device:&wgpu::Device,queue:&wgpu::Queue,texture:&wgpu::Texture,layer:u32)->[u8;4]{
 let buffer=device.create_buffer(&wgpu::BufferDescriptor{label:None,size:256,usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});let mut encoder=device.create_command_encoder(&Default::default());
 encoder.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo{texture,mip_level:0,origin:wgpu::Origin3d{x:0,y:0,z:layer},aspect:wgpu::TextureAspect::All},wgpu::TexelCopyBufferInfo{buffer:&buffer,layout:wgpu::TexelCopyBufferLayout{offset:0,bytes_per_row:Some(256),rows_per_image:Some(1)}},wgpu::Extent3d{width:1,height:1,depth_or_array_layers:1});queue.submit(Some(encoder.finish()));
 let(tx,rx)=std::sync::mpsc::channel();buffer.slice(..).map_async(wgpu::MapMode::Read,move|result|{tx.send(result).unwrap();});device.poll(wgpu::Maintain::Wait);rx.recv().unwrap().unwrap();
 let data=buffer.slice(..).get_mapped_range();let pixel=data[..4].try_into().unwrap();drop(data);buffer.unmap();pixel
}
#[test]
#[ignore="manual GPU-backed lookup benchmark; run with --ignored --nocapture --test-threads=1"]
fn benchmark_repeated_missing_references(){
 use std::{hint::black_box,time::Instant};let (_device,queue,mut renderer,mut state)=setup();
 state.texture_manager.insert_package("AuditPack".into(),UtxPackage{package_name:"AuditPack".into(),textures:(0..2000).map(|i|texture(&format!("Panel{i:05}"),"Walls",[255;4])).collect()});
 let names:Vec<_>=(0..16).map(|i|format!("AuditPack.Walls.Panel9{i:04}")).collect();
 let start=Instant::now();for name in &names{assert_eq!(renderer.resolve_or_upload_texture_layer(&queue,&state,name),(0,256.0,256.0));}println!("first 16 missing references: {:.3}ms",start.elapsed().as_secs_f64()*1000.0);
 for count in [1000,6000]{let mut times=Vec::new();for _ in 0..5{let start=Instant::now();for _ in 0..5{for i in 0..count{black_box(renderer.resolve_or_upload_texture_layer(&queue,black_box(&state),black_box(&names[i%16])));}}times.push(start.elapsed().as_secs_f64()*1000.0/5.0);}times.sort_by(f64::total_cmp);println!("requests={count}, missing_names=16, loaded_textures=2000, integrated_cache={:.3}ms",times[2]);}
 assert!(renderer.uploaded_texture_layers.is_empty());assert_eq!(renderer.next_texture_layer,1);
}
