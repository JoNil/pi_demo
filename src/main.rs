use gfx_backend::{egl, gl};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{run_return::EventLoopExtRunReturn, unix::WindowExtUnix},
    window::WindowBuilder,
};

use crate::{
    gfx::{
        buffer::{VertexFormat, VertexInfo},
        color::Color,
        device::Device,
        pipeline::ClearOptions,
    },
    gfx_backend::GlesBackend,
};

mod gfx;
mod gfx_backend;

static CONFIG_ATTRIBS: &[i32] = &[
    egl::EGL_RED_SIZE,
    8,
    egl::EGL_GREEN_SIZE,
    8,
    egl::EGL_BLUE_SIZE,
    8,
    egl::EGL_DEPTH_SIZE,
    8,
    egl::EGL_RENDERABLE_TYPE,
    egl::EGL_OPENGL_ES3_BIT,
    egl::EGL_NONE,
];

static CONTEXT_ATTRIBS: &[i32] = &[egl::EGL_CONTEXT_CLIENT_VERSION, 3, egl::EGL_NONE];

const VERT: &'static str = r#"
    #version 330 es
    layout(location = 0) in vec2 a_pos;
    layout(location = 1) in vec3 a_color;

    layout(location = 0) out vec3 v_color;

    void main() {
        v_color = a_color;
        gl_Position = vec4(a_pos - 0.5, 0.0, 1.0);
    }
"#;

const FRAG: &'static str = r#"
    #version 330 es
    precision mediump float;

    layout(location = 0) in vec3 v_color;
    layout(location = 0) out vec4 color;

    void main() {
        color = vec4(v_color, 1.0);
    }
"#;

fn main() {
    let display = egl::get_display(egl::EGL_DEFAULT_DISPLAY).unwrap();

    dbg!(display);

    let mut major = 0;
    let mut minor = 0;

    assert!(egl::initialize(display, &mut major, &mut minor));

    dbg!(major, minor);

    assert!(egl::bind_api(egl::EGL_OPENGL_ES_API));

    let config = egl::choose_config(display, CONFIG_ATTRIBS, 1).unwrap();

    dbg!(config);

    let context =
        egl::create_context(display, config, egl::EGL_NO_CONTEXT, CONTEXT_ATTRIBS).unwrap();

    dbg!(context);

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let surface =
        egl::create_window_surface(display, config, window.xlib_window().unwrap() as _, &[])
            .unwrap();

    assert!(egl::make_current(display, surface, surface, context));

    gl::load_with(|s| egl::get_proc_address(s) as _);

    let backend = GlesBackend::new();

    let device = Device::new(backend);

    let clear_options = ClearOptions::color(Color::new(0.1, 0.2, 0.3, 1.0));

    let vertex_info = VertexInfo::new()
        .attr(0, VertexFormat::Float32x2)
        .attr(1, VertexFormat::Float32x3);

    let pipeline = device
        .create_pipeline()
        .from(&VERT, &FRAG)
        .with_vertex_info(&vertex_info)
        .build()
        .unwrap();

    #[rustfmt::skip]
    let vertices = [
        0.5, 1.0,   1.0, 0.2, 0.3,
        0.0, 0.0,   0.1, 1.0, 0.3,
        1.0, 0.0,   0.1, 0.2, 1.0,
    ];

    let vbo = device
        .create_vertex_buffer()
        .with_info(&vertex_info)
        .with_data(&vertices)
        .build()
        .unwrap();

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                let mut renderer = device.create_renderer();

                renderer.begin(Some(&clear_options));
                renderer.set_pipeline(&pipeline);
                renderer.bind_buffer(&vbo);
                renderer.draw(0, 3);
                renderer.end();

                device.render(renderer.commands());

                egl::swap_buffers(display, surface);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });

    assert!(egl::destroy_surface(display, surface));
    assert!(egl::destroy_context(display, context));
    assert!(egl::terminate(display));
}
