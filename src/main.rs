use crate::{
    gfx::{
        buffer::{VertexFormat, VertexInfo},
        color::Color,
        device::Device,
        pipeline::ClearOptions,
    },
    gfx_backend::GlesBackend,
};
use glam::Mat4;
use rand::Rng;
use std::f32::consts::PI;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

mod gfx;
mod gfx_backend;

const VERT: &str = r#"
    #version 310 es
    #define MAX_INSTANCES 1000
    layout(location = 0) in vec3 a_pos;
    layout(location = 1) in vec3 a_color;

    layout(location = 0) out vec3 v_color;
    
    layout(std140, binding = 0) uniform Locals {
        mat4 u_mvp[MAX_INSTANCES];
    };

    void main() {
        v_color = a_color;
        gl_Position = u_mvp[gl_InstanceID] * vec4(a_pos, 1.0);
    }
"#;

const FRAG: &str = r#"
    #version 310 es
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

    let mut device = Device::new(GlesBackend::new(&window).unwrap());

    let clear_options = ClearOptions::color(Color::new(0.1, 0.2, 0.3, 1.0));

    let vertex_info = VertexInfo::new()
        .attr(0, VertexFormat::Float32x3)
        .attr(1, VertexFormat::Float32x3);

    let pipeline = device
        .create_pipeline()
        .from(VERT, FRAG)
        .with_vertex_info(&vertex_info)
        .build()
        .unwrap();

    #[rustfmt::skip]
    let vertices = [
         0.0,    0.5,  -1.0,   1.0, 0.2, 0.3,
         0.433, -0.25, -1.0,   0.1, 1.0, 0.3,
        -0.433, -0.25, -1.0,   0.1, 0.2, 1.0,
    ];

    let vbo = device
        .create_vertex_buffer()
        .with_info(&vertex_info)
        .with_data(&vertices)
        .build()
        .unwrap();

    let uniform_buffer = device.create_uniform_buffer(0, "Locals").build().unwrap();

    let mut angle = 0.0;

    let mut offsets = Vec::<f32>::new();

    for _i in 0..100 {
        offsets.push(rand::thread_rng().gen::<f32>() * 2.0 * PI);
    }

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                let mut mvps = Vec::new();

                let mut encoder = device.create_command_encoder();

                let proj = Mat4::perspective_rh_gl(
                    PI / 2.0,
                    device.size().0 as f32 / device.size().1 as f32,
                    0.01,
                    1000.0,
                );

                for offset in &offsets {
                    let rot = Mat4::from_rotation_z(angle + offset);

                    mvps.extend_from_slice(&(proj * rot).to_cols_array());
                }

                angle += 0.005;

                device.set_buffer_data(&uniform_buffer, &mvps);

                encoder.begin(Some(&clear_options));
                encoder.set_pipeline(&pipeline);
                encoder.bind_buffer(&vbo);
                encoder.bind_buffer(&uniform_buffer);
                encoder.draw(0, 3);
                encoder.draw_instanced(0, 3, mvps.len() as i32);
                encoder.end();

                device.render(encoder.commands());

                device.swap_buffers();

                device.clean();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent { event, window_id } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    if size.width > 0 && size.height > 0 {
                        device.set_size(size.width as i32, size.height as i32);
                    }
                }
                winit::event::WindowEvent::CloseRequested => {
                    if window_id == window.id() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            },
            _ => (),
        }
    });
}
