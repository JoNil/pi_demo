use super::{egl::EGLContext, gl, pipeline::{VertexAttributes, InnerPipeline}};
use std::ffi::c_void;

pub(crate) enum Kind {
    Vertex(VertexAttributes),
    Index,
    Uniform(u32, String),
}

pub(crate) struct InnerBuffer {
    buffer: u32,

    pub block_binded: bool,

    gpu_buff_size: usize,
    draw_usage: u32,
    draw_target: u32,
    pub(crate) kind: Kind,
    last_pipeline: Option<u64>,
}

impl InnerBuffer {
    #[allow(unused_variables)] // ubo is used only on wasm32 builds
    pub fn new(context: &EGLContext, kind: Kind, dynamic: bool) -> Result<Self, String> {
        let buffer = 0;
        unsafe {
            gl::GenBuffers(1, &mut buffer);
        }

        let draw_usage = if dynamic {
            gl::DYNAMIC_DRAW
        } else {
            gl::STATIC_DRAW
        };

        let draw_target = match &kind {
            Kind::Vertex(_) => gl::ARRAY_BUFFER,
            Kind::Index => gl::ELEMENT_ARRAY_BUFFER,
            Kind::Uniform(_, _) => gl::UNIFORM_BUFFER,
        };

        Ok(InnerBuffer {
            buffer,

            block_binded: false,

            gpu_buff_size: 0,
            draw_usage,
            draw_target,
            kind,
            last_pipeline: None,
        })
    }

    #[inline]
    pub fn bind(&mut self, context: &EGLContext, pipeline_id: Option<u64>) {
        let pipeline_changed = pipeline_id.is_some() && pipeline_id != self.last_pipeline;
        if pipeline_changed {
            self.last_pipeline = pipeline_id;
        };

        unsafe {
            gl::BindBuffer(self.draw_target, self.buffer);

            match &self.kind {
                Kind::Vertex(attrs) => {
                    if pipeline_changed {
                        attrs.enable(context);
                    }
                }
                Kind::Uniform(slot, _) => {
                    gl.bind_buffer_base(gl::UNIFORM_BUFFER, *slot, Some(self.buffer));
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn update(&mut self, gl: &EGLContext, data: &[u8]) {
        let needs_alloc = self.gpu_buff_size != data.len();

        unsafe {
            if needs_alloc {
                gl::BufferData(
                    self.draw_target,
                    data.len() as isize,
                    data.as_ptr() as *const c_void,
                    self.draw_usage,
                );
            } else {
                gl::BufferSubData(
                    self.draw_target,
                    0,
                    data.len() as isize,
                    data.as_ptr() as *const c_void,
                );
            }
        }
    }

    pub fn bind_ubo_block(&mut self, context: &EGLContext, pipeline: &InnerPipeline) {
        self.block_binded = true;

        if let Kind::Uniform(slot, name) = &self.kind {
            unsafe {
                if let Some(index) = gl.get_uniform_block_index(pipeline.program, name) {
                    gl.uniform_block_binding(pipeline.program, index, *slot as _);
                }
            }
        }
    }

    #[inline(always)]
    pub fn clean(self, context: &EGLContext) {
        unsafe {
            gl.delete_buffer(self.buffer);
        }
    }
}
