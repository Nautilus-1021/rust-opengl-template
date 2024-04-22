use std::{
    mem::size_of,
    num::NonZeroU32,
    sync::{
        mpsc::{channel, SendError, Sender},
        Arc
    },
    thread::{self, JoinHandle}
};

use glow::{HasContext, ARRAY_BUFFER, COLOR_BUFFER_BIT, FLOAT, FRAGMENT_SHADER, RENDERER, SHADING_LANGUAGE_VERSION, STATIC_DRAW, TRIANGLES, VERTEX_SHADER};
use winit::window::Window;
use glutin_winit::GlWindow;
use glutin::{
    config::Config,
    context::{NotCurrentContext, NotCurrentGlContext},
    display::{Display, GlDisplay},
    surface::{GlSurface, SwapInterval}
};

use crate::util;

const VERTEX_SHADER_SOURCE: &str = "
#version 330 core

layout (location = 0) in vec2 aPos;
layout (location = 1) in vec3 aColor;

out vec3 bColor;

void main() {
    bColor = aColor;
    gl_Position = vec4(aPos, 0.0, 1.0);
}";

const FRAGMENT_SHADER_SOURCE: &str = "
#version 330 core
out vec4 FragColor;

in vec3 bColor;

void main() {
    FragColor = vec4(bColor, 1.0);
}";

pub enum ThreadMessage {
    Exit,
    Resize(i32, i32)
}

pub struct ThreadedRenderer {
    channel_sender: Sender<ThreadMessage>,
    th_handle: JoinHandle<()>
}

impl ThreadedRenderer {
    pub fn new(gl_display: Display, gl_config: Config, wnd: Arc<Window>, not_current_context: NotCurrentContext) -> Self {
        let (th_channel_sender, th_channel_receiver) = channel();

        let th_handle = thread::Builder::new().name("Render Thread".to_owned()).spawn(move || {
            let attrs = wnd.build_surface_attributes(Default::default());
            let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs).unwrap() };

            let wnd_context = not_current_context.make_current(&gl_surface).unwrap();
            
            if let Err(msg) = gl_surface.set_swap_interval(&wnd_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())) {
                eprintln!("Error when setting vsync: {msg}");
            }

            let context = unsafe { glow::Context::from_loader_function_cstr(|name| { gl_display.get_proc_address(name) }) };

            let hardware = unsafe { context.get_parameter_string(RENDERER) };
            println!("[Glow] Running on {hardware}");

            let version = context.version();
            println!("[Glow] OpenGL version: {}.{}\n[Glow] Driver: {}", version.major, version.minor, version.vendor_info);

            let glsl_version = unsafe { context.get_parameter_string(SHADING_LANGUAGE_VERSION) };
            println!("[Glow] GLSL version: {glsl_version}");

            unsafe {
                context.clear_color(1.0, 1.0, 1.0, 1.0);
            }

            let triangle_buf = [
                0.5, -0.5,  1.0, 0.0, 0.0,
                -0.5, -0.5, 0.0, 1.0, 0.0,
                0.0, 0.5,   0.0, 0.0, 1.0_f32
            ];

            let (triangle_vao, shader_program) = unsafe {
                let triangle_vao = context.create_vertex_array().unwrap();
                context.bind_vertex_array(Some(triangle_vao));

                let triangle_vbo = context.create_buffer().unwrap();
                context.bind_buffer(ARRAY_BUFFER, Some(triangle_vbo));
                context.buffer_data_u8_slice(ARRAY_BUFFER, util::to_raw_data(&triangle_buf), STATIC_DRAW);

                context.vertex_attrib_pointer_f32(0, 2, FLOAT, false, (size_of::<f32>() * 5) as i32, 0);
                context.enable_vertex_attrib_array(0);

                context.vertex_attrib_pointer_f32(1, 3, FLOAT, false, (size_of::<f32>() * 5) as i32, (size_of::<f32>() * 2) as i32);
                context.enable_vertex_attrib_array(1);


                let vert_shader = context.create_shader(VERTEX_SHADER).unwrap();
                context.shader_source(vert_shader, VERTEX_SHADER_SOURCE);
                context.compile_shader(vert_shader);

                if !context.get_shader_compile_status(vert_shader) {
                    panic!("Error when compiling vertex shader: {}", context.get_shader_info_log(vert_shader))
                }

                let frag_shader = context.create_shader(FRAGMENT_SHADER).unwrap();
                context.shader_source(frag_shader, FRAGMENT_SHADER_SOURCE);
                context.compile_shader(frag_shader);

                if !context.get_shader_compile_status(frag_shader) {
                    panic!("Error when compiling fragment shader: {}", context.get_shader_info_log(frag_shader))
                }

                let shader_program = context.create_program().unwrap();
                context.attach_shader(shader_program, vert_shader);
                context.attach_shader(shader_program, frag_shader);
                context.link_program(shader_program);

                if !context.get_program_link_status(shader_program) {
                    panic!("Error when linking shader program: {}", context.get_program_info_log(shader_program))
                }

                context.delete_shader(vert_shader);
                context.delete_shader(frag_shader);

                (triangle_vao, shader_program)
            };

            'render: loop {
                for message in th_channel_receiver.try_iter() {
                    match message {
                        ThreadMessage::Exit => {
                            break 'render;
                        }
                        ThreadMessage::Resize(new_width, new_height) => {
                            gl_surface.resize(&wnd_context, NonZeroU32::new(new_width as u32).unwrap(), NonZeroU32::new(new_height as u32).unwrap());
                            unsafe {
                                context.viewport(0, 0, new_width, new_height);
                            }
                        }
                    }
                }

                unsafe {
                    context.clear(COLOR_BUFFER_BIT);

                    context.use_program(Some(shader_program));
                    context.bind_vertex_array(Some(triangle_vao));

                    context.draw_arrays(TRIANGLES, 0, 3);

                    let _ = gl_surface.swap_buffers(&wnd_context);

                    wnd.request_redraw();
                }
            }
        }).unwrap();

        Self { channel_sender: th_channel_sender, th_handle }
    }

    pub fn stop(self) {
        let _ = self.channel_sender.send(ThreadMessage::Exit);
        self.th_handle.join().unwrap();
    }

    pub fn send_message(&self, message: ThreadMessage) -> Result<(), SendError<ThreadMessage>> {
        self.channel_sender.send(message)
    }
}
