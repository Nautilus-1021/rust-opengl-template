use std::sync::Arc;

use winit::{
    error::EventLoopError,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder
};
use glutin_winit::DisplayBuilder;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, GlProfile, Version},
    display::{GetGlDisplay, GlDisplay}
};
use raw_window_handle::HasRawWindowHandle;

mod renderer;
use renderer::ThreadMessage;
pub mod util;

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new().unwrap();
    let wnd_builder = WindowBuilder::new().with_title("Rust OpenGL");

    let (wnd, gl_config) = DisplayBuilder::new().with_window_builder(Some(wnd_builder)).build(&event_loop, ConfigTemplateBuilder::new(), |mut configs| { configs.next().unwrap() }).unwrap();

    let wnd = Arc::new(wnd.unwrap());

    let rwh = wnd.raw_window_handle();

    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new().with_profile(GlProfile::Core).with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3)))).build(Some(rwh));

    let not_current_context = unsafe { gl_display.create_context(&gl_config, &context_attributes).unwrap() };

    let mut renderer = Some(renderer::ThreadedRenderer::new(gl_display.clone(), gl_config.clone(), wnd.clone(), not_current_context));

    event_loop.run(move |event, window_target| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(size) => {
                    if size.width != 0 && size.height != 0 {
                        renderer.as_ref().unwrap().send_message(ThreadMessage::Resize(size.width as i32, size.height as i32)).unwrap();
                    }
                }
                WindowEvent::CloseRequested => {
                    window_target.exit();
                }
                _ => ()
            }
            Event::LoopExiting => {
                renderer.take().unwrap().stop();
            }
            _ => ()
        }
    })
}
