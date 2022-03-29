use crate::gfx::{
    buffer::{VertexAttr, VertexStepMode},
    pipeline::{BlendMode, CompareMode, PipelineOptions, StencilAction, StencilOptions},
};

use super::{
    egl::EGLContext,
    gl,
    to_gl::{ToGl, ToOptionalGl},
};

pub(crate) struct InnerPipeline {
    pub vertex: u32,
    pub fragment: u32,
    pub program: u32,
    pub vao: u32,
    pub uniform_locations: Vec<u32>,
}

#[inline]
pub(crate) fn get_inner_attrs(attrs: &[VertexAttr]) -> (i32, Vec<InnerAttr>) {
    let mut stride = 0;
    let attrs = attrs
        .iter()
        .map(|attr| {
            let inner_attr = InnerAttr::from(attr, stride);
            stride += attr.format.bytes();
            inner_attr
        })
        .collect::<Vec<_>>();

    (stride, attrs)
}

impl InnerPipeline {
    #[inline(always)]
    pub fn new(
        context: &EGLContext,
        vertex_source: &str,
        fragment_source: &str,
        attrs: &[VertexAttr],
    ) -> Result<Self, String> {
        let (stride, attrs) = get_inner_attrs(attrs);

        create_pipeline(context, vertex_source, fragment_source, stride, attrs)
    }

    #[inline(always)]
    pub fn clean(self, context: &EGLContext) {
        clean_pipeline(context, self);
    }

    #[inline(always)]
    pub fn bind(&self, context: &EGLContext, options: &PipelineOptions) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::UseProgram(self.program);

            set_stencil(context, options);
            set_depth_stencil(context, options);
            set_color_mask(context, options);
            set_culling(context, options);
            set_blend_mode(context, options);
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VertexAttributes {
    pub stride: i32,
    attrs: Vec<InnerAttr>,
    vertex_step_mode: VertexStepMode,
}

impl VertexAttributes {
    pub fn new(stride: i32, attrs: Vec<InnerAttr>, vertex_step_mode: VertexStepMode) -> Self {
        Self {
            stride,
            attrs,
            vertex_step_mode,
        }
    }

    pub unsafe fn enable(&self, context: &EGLContext) {
        let step_mode = match self.vertex_step_mode {
            VertexStepMode::Vertex => 0,
            VertexStepMode::Instance => 1,
        };

        self.attrs
            .iter()
            .for_each(|attr| attr.enable(context, self.stride, step_mode));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InnerAttr {
    pub location: u32,
    pub size: i32,
    pub data_type: u32,
    pub normalized: bool,
    pub offset: i32,
}

impl InnerAttr {
    #[inline(always)]
    fn from(attr: &VertexAttr, offset: i32) -> InnerAttr {
        Self {
            location: attr.location,
            size: attr.format.size(),
            data_type: attr.format.to_gl(),
            normalized: attr.format.normalized(),
            offset,
        }
    }

    #[inline(always)]
    unsafe fn enable(&self, _context: &EGLContext, stride: i32, vertex_step_mode: u32) {
        gl::EnableVertexAttribArray(self.location);
        gl::VertexAttribPointer(
            self.location,
            self.size,
            self.data_type,
            self.normalized as u8,
            stride,
            self.offset as *const _,
        );
        gl::VertexAttribDivisor(self.location, vertex_step_mode);
    }
}

#[inline(always)]
unsafe fn set_stencil(_context: &EGLContext, options: &PipelineOptions) {
    if should_disable_stencil(&options.stencil) {
        gl::Disable(gl::STENCIL_TEST);
    } else if let Some(opts) = options.stencil {
        gl::Enable(gl::STENCIL_TEST);
        gl::StencilMask(opts.write_mask);
        gl::StencilOp(
            opts.stencil_fail.to_gl(),
            opts.depth_fail.to_gl(),
            opts.pass.to_gl(),
        );
        gl::StencilFunc(
            opts.compare.to_gl().unwrap_or(gl::ALWAYS),
            opts.reference as _,
            opts.read_mask,
        );
    }
}

#[inline(always)]
unsafe fn set_depth_stencil(_context: &EGLContext, options: &PipelineOptions) {
    match options.depth_stencil.compare.to_gl() {
        Some(d) => {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(d);
        }
        _ => gl::Disable(gl::DEPTH_TEST),
    }

    gl::DepthMask(options.depth_stencil.write as _);
}

#[inline(always)]
unsafe fn set_color_mask(_context: &EGLContext, options: &PipelineOptions) {
    gl::ColorMask(
        options.color_mask.r as _,
        options.color_mask.g as _,
        options.color_mask.b as _,
        options.color_mask.a as _,
    );
}

#[inline(always)]
unsafe fn set_culling(_context: &EGLContext, options: &PipelineOptions) {
    match options.cull_mode.to_gl() {
        Some(mode) => {
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(mode);
        }
        _ => gl::Disable(gl::CULL_FACE),
    }
}

#[inline(always)]
unsafe fn set_blend_mode(_context: &EGLContext, options: &PipelineOptions) {
    match (options.color_blend, options.alpha_blend) {
        (Some(cbm), None) => {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(cbm.src.to_gl(), cbm.dst.to_gl());
            gl::BlendEquation(cbm.op.to_gl());
        }
        (Some(cbm), Some(abm)) => {
            gl::Enable(gl::BLEND);
            gl::BlendFuncSeparate(
                cbm.src.to_gl(),
                cbm.dst.to_gl(),
                abm.src.to_gl(),
                abm.dst.to_gl(),
            );
            gl::BlendEquationSeparate(cbm.op.to_gl(), abm.op.to_gl());
        }
        (None, Some(abm)) => {
            let cbm = BlendMode::NORMAL;
            gl::Enable(gl::BLEND);
            gl::BlendFuncSeparate(
                cbm.src.to_gl(),
                cbm.dst.to_gl(),
                abm.src.to_gl(),
                abm.dst.to_gl(),
            );
            gl::BlendEquationSeparate(cbm.op.to_gl(), abm.op.to_gl());
        }
        (None, None) => {
            gl::Disable(gl::BLEND);
        }
    }
}

#[inline(always)]
fn clean_pipeline(_context: &EGLContext, pip: InnerPipeline) {
    let InnerPipeline {
        vertex,
        fragment,
        program,
        vao,
        ..
    } = pip;

    unsafe {
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
        gl::DeleteProgram(program);
        gl::DeleteVertexArrays(1, &vao as *const _);
    }
}

#[inline(always)]
fn create_pipeline(
    context: &EGLContext,
    vertex_source: &str,
    fragment_source: &str,
    _stride: i32,
    _attrs: Vec<InnerAttr>,
) -> Result<InnerPipeline, String> {
    let vertex = create_shader(context, gl::VERTEX_SHADER, vertex_source)?;
    let fragment = create_shader(context, gl::FRAGMENT_SHADER, fragment_source)?;
    let program = create_program(context, vertex, fragment)?;

    let uniform_locations = unsafe {
        let mut count = 0;
        gl::GetProgramiv(program, gl::ACTIVE_UNIFORMS, &mut count);

        let mut uniform_max_size = 0;
        gl::GetProgramiv(
            program,
            gl::ACTIVE_UNIFORM_MAX_LENGTH,
            &mut uniform_max_size,
        );

        (0..count)
            .into_iter()
            .filter_map(|index| {
                let mut name = String::with_capacity(uniform_max_size as usize);
                name.extend(std::iter::repeat('\0').take(uniform_max_size as usize));
                let mut length = 0;
                let mut size = 0;
                let mut utype = 0;
                gl::GetActiveUniform(
                    program,
                    index as _,
                    uniform_max_size,
                    &mut length,
                    &mut size,
                    &mut utype,
                    name.as_ptr() as *mut _,
                );
                name.truncate(length as usize);

                match gl::GetUniformLocation(program, name.as_ptr() as *const _) {
                    0 => {
                        // inform about uniforms outside of blocks that are missing
                        if !name.contains("") {
                            eprintln!("Cannot get uniform location for: {}", name);
                        }
                        None
                    }
                    loc => Some(loc as _),
                }
            })
            .collect::<Vec<_>>()
    };

    let vao = unsafe {
        let mut vao = 0;
        gl::GenVertexArrays(1, &mut vao as *mut _);
        gl::BindVertexArray(vao);
        vao
    };

    Ok(InnerPipeline {
        vertex,
        fragment,
        program,
        vao,
        uniform_locations,
    })
}

#[inline(always)]
fn create_shader(_context: &EGLContext, typ: u32, source: &str) -> Result<u32, String> {
    unsafe {
        let shader = gl::CreateShader(typ);
        gl::ShaderSource(
            shader,
            1,
            &(source.as_ptr() as *const _),
            &(source.len() as _),
        );
        gl::CompileShader(shader);

        let mut status = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status as *mut _);
        if status == 1 {
            return Ok(shader);
        }

        let err = {
            let mut length = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut length as *mut _);
            if length > 0 {
                let mut log = String::with_capacity(length as usize);
                log.extend(std::iter::repeat('\0').take(length as usize));
                gl::GetShaderInfoLog(shader, length, &mut length, (&log[..]).as_ptr() as *mut _);
                log.truncate(length as usize);
                log
            } else {
                String::from("")
            }
        };
        gl::DeleteShader(shader);

        let typ_name = match typ {
            gl::VERTEX_SHADER => "vertex".to_string(),
            gl::FRAGMENT_SHADER => "fragment".to_string(),
            _ => format!("unknown type ({})", typ),
        };

        Err(format!(
            "{} with {} shader: \n--\n{}\n--\n",
            err, typ_name, source
        ))
    }
}

#[inline(always)]
fn create_program(_context: &EGLContext, vertex: u32, fragment: u32) -> Result<u32, String> {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);

        let mut status = 0;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
        if status == 1 {
            return Ok(program);
        }

        let err = {
            let mut length = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut length);
            if length > 0 {
                let mut log = String::with_capacity(length as usize);
                log.extend(std::iter::repeat('\0').take(length as usize));
                gl::GetProgramInfoLog(program, length, &mut length, (&log[..]).as_ptr() as *mut _);
                log.truncate(length as usize);
                log
            } else {
                String::from("")
            }
        };
        gl::DeleteProgram(program);
        Err(err)
    }
}

#[inline(always)]
fn should_disable_stencil(stencil: &Option<StencilOptions>) -> bool {
    match stencil {
        Some(stencil) => {
            stencil.compare == CompareMode::Always
                && stencil.stencil_fail == StencilAction::Keep
                && stencil.depth_fail == StencilAction::Keep
                && stencil.pass == StencilAction::Keep
        }
        None => true,
    }
}
