use ut99_editor::{EditorApp, app::EditorEvent};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Forge99 — 64-bit Level Editor for Unreal Tournament 99...");

    let event_loop = EventLoop::<EditorEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = EditorApp::default();
    app.install_repaint_callback(event_loop.create_proxy());
    event_loop.run_app(&mut app)?;

    Ok(())
}
