use crate::{
    gfx::{
        buffer::{VertexFormat, VertexInfo},
        device::Device,
    },
    gfx_backend::GlesBackend,
};
use gfx::{
    buffer::Buffer,
    pipeline::{CompareMode, DepthStencil, Pipeline},
};
use glam::{vec3, Mat4, Quat, Vec3};
use oden_plugin_rs::{
    register_plugin, DrawParams, GuiParams, InitParams, OdenPlugin, ShutdownParams, UpdateParams,
};
use rand::Rng;
use std::f32::consts::PI;

mod gfx;
mod gfx_backend;

const VERT: &str = r#"
    #version 310 es
    #define MAX_INSTANCES 1000
    layout(location = 0) in vec3 a_pos;
    layout(location = 1) in vec3 a_color;

    layout(location = 0) out vec3 v_color;
    
    layout(std140, binding = 0) uniform Locals {
        mat4 u_p[MAX_INSTANCES];
    };

    layout(std140, binding = 1) uniform DrawLocals {
        mat4 u_mv;
    };

    void main() {
        v_color = a_color;
        gl_Position = u_mv * u_p[gl_InstanceID] * vec4(a_pos, 1.0);
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

struct State {
    device: Device<GlesBackend>,
    pipeline: Pipeline,
    vbo: Buffer,
    uniform_buffer: Buffer,
    draw_uniform_buffer: Buffer,

    angle: f32,
    offsets: Vec<(f32, f32, f32, f32)>,
}

impl OdenPlugin for State {
    fn init(api: &InitParams) -> Self {
        let mut device = Device::new(GlesBackend::new(api.gl_loader()).unwrap());

        let vertex_info = VertexInfo::new()
            .attr(0, VertexFormat::Float32x3)
            .attr(1, VertexFormat::Float32x3);

        let pipeline = device
            .create_pipeline()
            .from(VERT, FRAG)
            .with_vertex_info(&vertex_info)
            .with_depth_stencil(DepthStencil {
                write: true,
                compare: CompareMode::Less,
            })
            .build()
            .unwrap();

        #[rustfmt::skip]
        let vertices = [
            0.0,    0.5,  0.0,   1.0, 0.2, 0.3,
            0.433, -0.25, 0.0,   0.1, 1.0, 0.3,
            -0.433, -0.25, 0.0,   0.1, 0.2, 1.0,
        ];

        let vbo = device
            .create_vertex_buffer()
            .with_info(&vertex_info)
            .with_data(&vertices)
            .build()
            .unwrap();

        let uniform_buffer = device.create_uniform_buffer(0, "Locals").build().unwrap();
        let draw_uniform_buffer = device
            .create_uniform_buffer(1, "DrawLocals")
            .build()
            .unwrap();

        let mut offsets = Vec::new();

        for _i in 0..1000 {
            offsets.push((
                rand::thread_rng().gen::<f32>() * 2.0 * PI,
                rand::thread_rng().gen::<f32>() * 2.0 - 1.0,
                rand::thread_rng().gen::<f32>() * 2.0 - 1.0,
                rand::thread_rng().gen::<f32>() * 0.05,
            ));
        }

        State {
            device,
            pipeline,
            vbo,
            uniform_buffer,
            draw_uniform_buffer,
            angle: 0.0,
            offsets,
        }
    }

    fn shutdown(self, _api: &ShutdownParams) {}

    fn update(&mut self, _api: &UpdateParams) {
        self.device.clean();

        let mut mvps = Vec::new();
        mvps.reserve(self.offsets.len() * 16);

        for offset in &self.offsets {
            let transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.1),
                Quat::from_rotation_z(self.angle + offset.0),
                vec3(offset.1, offset.2, -1.0 + offset.3),
            );

            mvps.extend_from_slice(&transform.to_cols_array());
        }

        self.angle += 0.005;

        self.device.set_buffer_data(&self.uniform_buffer, &mvps);
    }

    fn draw(&mut self, api: &DrawParams) {
        let proj = Mat4::from(api.proj_matrix()) * Mat4::from(api.world_matrix());

        let viewport = api.viewport();
        let target_size = api.target_size();

        self.device.set_size(target_size.0, target_size.1);
        self.device
            .set_buffer_data(&self.draw_uniform_buffer, &proj.to_cols_array());

        let mut encoder = self.device.create_command_encoder();
        encoder.begin(None);
        encoder.set_viewport(
            viewport.0 as f32,
            viewport.1 as f32,
            viewport.2 as f32,
            viewport.3 as f32,
        );
        encoder.set_pipeline(&self.pipeline);
        encoder.bind_buffer(&self.vbo);
        encoder.bind_buffer(&self.uniform_buffer);
        encoder.bind_buffer(&self.draw_uniform_buffer);
        encoder.draw_instanced(0, 3, self.offsets.len() as i32);
        encoder.end();

        self.device.render(encoder.commands());
    }

    fn gui(&mut self, _api: &GuiParams) {}
}

register_plugin!(
    "Example Rendering",
    "f8af6c02-f226-457d-b93e-98d5cec6e5f8",
    State
);
