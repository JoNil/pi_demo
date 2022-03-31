use super::{
    gl,
    pipeline::{InnerPipeline, VertexAttributes},
    Context,
};
use std::ffi::{c_void, CString};

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
    pub fn new(_context: &Context, kind: Kind, dynamic: bool) -> Result<Self, String> {
        let mut buffer = 0;
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
    pub fn bind(&mut self, context: &Context, pipeline_id: Option<u64>) {
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
                    gl::BindBufferBase(gl::UNIFORM_BUFFER, *slot, self.buffer);
                }
                _ => {}
            }
        }
    }

    #[inline]
    pub fn update(&mut self, _context: &Context, data: &[u8]) {
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

    pub fn bind_ubo_block(&mut self, _context: &Context, pipeline: &InnerPipeline) {
        self.block_binded = true;

        if let Kind::Uniform(slot, name) = &self.kind {
            unsafe {
                let name = CString::new(name.clone()).unwrap();

                let index = gl::GetUniformBlockIndex(pipeline.program, name.as_ptr());

                if index != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(pipeline.program, index, *slot);
                }
            }
        }
    }

    #[inline(always)]
    pub fn clean(self, _context: &Context) {
        unsafe {
            gl::DeleteBuffers(1, &self.buffer as *const _);
        }
    }
}
