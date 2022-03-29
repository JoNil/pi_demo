use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
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
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let backend = GlesBackend::new(&window);

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
                let mut encoder = device.create_command_encoder();

                encoder.begin(Some(&clear_options));
                encoder.set_pipeline(&pipeline);
                encoder.bind_buffer(&vbo);
                encoder.draw(0, 3);
                encoder.end();

                device.render(encoder.commands());

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
}
