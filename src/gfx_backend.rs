use self::{
    buffer::{InnerBuffer, Kind},
    pipeline::{get_inner_attrs, InnerPipeline, VertexAttributes},
    render_target::InnerRenderTexture,
    texture::{texture_format, InnerTexture},
    to_gl::ToGl,
};
use crate::{
    gfx::{
        buffer::{VertexAttr, VertexStepMode},
        color::Color,
        commands::Commands,
        device::{DeviceBackend, ResourceId},
        limits::Limits,
        pipeline::{DrawPrimitive, PipelineOptions},
        texture::{TextureInfo, TextureRead, TextureUpdate},
    },
    gfx_backend::gl::types::GLint,
};
use std::collections::HashMap;
use winit::window::Window;

#[cfg(target_os = "linux")]
use egl::{EGLContext, EGLDisplay, EGLSurface};

#[cfg(target_os = "linux")]
use winit::platform::unix::WindowExtUnix;

mod buffer;
pub mod gl;
mod pipeline;
mod render_target;
mod texture;
mod to_gl;

#[cfg(target_os = "linux")]
pub mod egl;

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
static CONTEXT_ATTRIBS: &[i32] = &[egl::EGL_CONTEXT_CLIENT_VERSION, 3, egl::EGL_NONE];

#[cfg(target_os = "linux")]
type Context = EGLContext;

#[cfg(target_os = "windows")]
type Context = raw_gl_context::GlContext;

pub struct GlesBackend {
    #[cfg(target_os = "linux")]
    display: EGLDisplay,
    #[cfg(target_os = "linux")]
    context: EGLContext,
    #[cfg(target_os = "linux")]
    surface: EGLSurface,

    #[cfg(target_os = "windows")]
    context: raw_gl_context::GlContext,

    buffer_count: u64,
    texture_count: u64,
    pipeline_count: u64,
    render_target_count: u64,
    size: (i32, i32),
    dpi: f32,
    pipelines: HashMap<u64, InnerPipeline>,
    buffers: HashMap<u64, InnerBuffer>,
    textures: HashMap<u64, InnerTexture>,
    render_targets: HashMap<u64, InnerRenderTexture>,
    using_indices: bool,
    current_pipeline: u64,
    limits: Limits,
    current_uniforms: Vec<u32>,
}

impl GlesBackend {
    pub fn new(window: &Window) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        let (display, context, surface) = {
            let display =
                egl::get_display(egl::EGL_DEFAULT_DISPLAY).ok_or("Faild to get egl display")?;

            let mut major = 0;
            let mut minor = 0;

            egl::initialize(display, &mut major, &mut minor)
                .then(|| ())
                .ok_or("Failed to initialize egl")?;

            egl::bind_api(egl::EGL_OPENGL_ES_API)
                .then(|| ())
                .ok_or("Failed to bind api")?;

            let config =
                egl::choose_config(display, CONFIG_ATTRIBS, 1).ok_or("Failed to choose config")?;

            let context =
                egl::create_context(display, config, egl::EGL_NO_CONTEXT, CONTEXT_ATTRIBS)
                    .ok_or("Failed to create context")?;

            let window = window.xlib_window().ok_or("Failed to get window")?;

            let surface = egl::create_window_surface(display, config, window as _, &[])
                .ok_or("Failed to create surface")?;

            egl::make_current(display, surface, surface, context)
                .then(|| ())
                .ok_or("Failed to make the context current")?;

            gl::load_with(|s| egl::get_proc_address(s) as _);

            (display, context, surface)
        };

        #[cfg(target_os = "windows")]
        let context = {
            let context =
                raw_gl_context::GlContext::create(&window, raw_gl_context::GlConfig::default())
                    .unwrap();

            context.make_current();

            gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

            context
        };

        let mut limits = Limits::default();
        unsafe {
            gl::GetIntegerv(
                gl::MAX_TEXTURE_SIZE,
                &mut limits.max_texture_size as *mut _ as *mut GLint,
            );
            gl::GetIntegerv(
                gl::MAX_UNIFORM_BLOCK_SIZE,
                &mut limits.max_uniform_blocks as *mut _ as *mut GLint,
            );
        }

        Ok(Self {
            #[cfg(target_os = "linux")]
            display,
            #[cfg(target_os = "linux")]
            context,
            #[cfg(target_os = "linux")]
            surface,

            #[cfg(target_os = "windows")]
            context,

            pipeline_count: 0,
            buffer_count: 0,
            texture_count: 0,
            render_target_count: 0,
            size: (0, 0),
            dpi: 1.0,
            pipelines: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            render_targets: HashMap::new(),
            using_indices: false,
            current_pipeline: 0,
            limits,
            current_uniforms: vec![],
        })
    }
}

#[cfg(target_os = "linux")]
impl Drop for GlesBackend {
    fn drop(&mut self) {
        assert!(egl::destroy_surface(self.display, self.surface));
        assert!(egl::destroy_context(self.display, self.context));
        assert!(egl::terminate(self.display));
    }
}

impl GlesBackend {
    #[inline(always)]
    fn clear(&self, color: &Option<Color>, depth: &Option<f32>, stencil: &Option<i32>) {
        clear(&self.context, color, depth, stencil);
    }

    fn begin(
        &self,
        target: Option<u64>,
        color: &Option<Color>,
        depth: &Option<f32>,
        stencil: &Option<i32>,
    ) {
        let render_target = match target {
            Some(id) => self.render_targets.get(&id),
            _ => None,
        };

        let (width, height, dpi) = match render_target {
            Some(rt) => {
                rt.bind(&self.context);
                (rt.size.0, rt.size.1, 1.0)
            }
            None => {
                unsafe {
                    gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                }
                (self.size.0, self.size.1, self.dpi)
            }
        };

        self.viewport(0.0, 0.0, width as _, height as _, dpi);

        self.clear(color, depth, stencil);
    }

    #[inline]
    fn viewport(&self, x: f32, y: f32, width: f32, height: f32, dpi: f32) {
        let ww = width * dpi;
        let hh = height * dpi;

        unsafe {
            gl::Viewport(x as _, y as _, ww as _, hh as _);
        }
    }

    #[inline]
    fn scissors(&self, x: f32, y: f32, width: f32, height: f32, dpi: f32) {
        let canvas_height = ((self.size.1 - (height + y) as i32) as f32 * dpi) as i32;
        let x = x * dpi;
        let width = width * dpi;
        let height = height * dpi;

        unsafe {
            gl::Enable(gl::SCISSOR_TEST);
            gl::Scissor(x as _, canvas_height, width as _, height as _);
        }
    }

    fn end(&mut self) {
        unsafe {
            gl::Disable(gl::SCISSOR_TEST);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        self.using_indices = false;
    }

    fn clean_pipeline(&mut self, id: u64) {
        if let Some(pip) = self.pipelines.remove(&id) {
            pip.clean(&self.context);
        }
    }

    fn set_pipeline(&mut self, id: u64, options: &PipelineOptions) {
        if let Some(pip) = self.pipelines.get(&id) {
            pip.bind(&self.context, options);
            self.using_indices = false;
            self.current_pipeline = id;
            self.current_uniforms = pip.uniform_locations.clone();
        }
    }

    fn bind_buffer(&mut self, id: u64) {
        if let Some(buffer) = self.buffers.get_mut(&id) {
            match &buffer.kind {
                Kind::Index => {
                    self.using_indices = true;
                }
                Kind::Uniform(_slot, _name) => {
                    if !buffer.block_binded {
                        buffer.bind_ubo_block(
                            &self.context,
                            self.pipelines.get(&self.current_pipeline).as_ref().unwrap(),
                        );
                    }
                }
                _ => {}
            }

            buffer.bind(&self.context, Some(self.current_pipeline));
        }
    }

    fn bind_texture(&mut self, id: u64, slot: u32, location: u32) {
        if let Some(texture) = self.textures.get(&id) {
            texture.bind(&self.context, slot, self.get_uniform_loc(&location));
        }
    }

    #[inline(always)]
    fn get_uniform_loc<'a>(&'a self, location: &'a u32) -> &'a u32 {
        &self.current_uniforms[*location as usize]
    }

    fn clean_buffer(&mut self, id: u64) {
        if let Some(buffer) = self.buffers.remove(&id) {
            buffer.clean(&self.context);
        }
    }

    fn clean_texture(&mut self, id: u64) {
        if let Some(texture) = self.textures.remove(&id) {
            texture.clean(&self.context);
        }
    }

    fn clean_render_target(&mut self, id: u64) {
        if let Some(rt) = self.render_targets.remove(&id) {
            rt.clean(&self.context);
        }
    }

    fn draw(&mut self, primitive: &DrawPrimitive, offset: i32, count: i32) {
        unsafe {
            if self.using_indices {
                gl::DrawElements(
                    primitive.to_gl(),
                    count,
                    gl::UNSIGNED_INT,
                    (offset * 4) as *const _,
                );
            } else {
                gl::DrawArrays(primitive.to_gl(), offset, count);
            }
        }
    }
    fn draw_instanced(&mut self, primitive: &DrawPrimitive, offset: i32, count: i32, length: i32) {
        unsafe {
            if self.using_indices {
                gl::DrawElementsInstanced(
                    primitive.to_gl(),
                    count,
                    gl::UNSIGNED_INT,
                    offset as *const _,
                    length,
                );
            } else {
                gl::DrawArraysInstanced(primitive.to_gl(), offset, count, length);
            }
        }
    }
}

impl DeviceBackend for GlesBackend {
    fn limits(&self) -> Limits {
        self.limits
    }

    fn create_pipeline(
        &mut self,
        vertex_source: &[u8],
        fragment_source: &[u8],
        vertex_attrs: &[VertexAttr],
        options: PipelineOptions,
    ) -> Result<u64, String> {
        let vertex_source = std::str::from_utf8(vertex_source).map_err(|e| e.to_string())?;
        let fragment_source = std::str::from_utf8(fragment_source).map_err(|e| e.to_string())?;

        let inner_pipeline =
            InnerPipeline::new(&self.context, vertex_source, fragment_source, vertex_attrs)?;
        inner_pipeline.bind(&self.context, &options);

        self.pipeline_count += 1;
        self.pipelines.insert(self.pipeline_count, inner_pipeline);

        self.set_pipeline(self.pipeline_count, &options);
        Ok(self.pipeline_count)
    }

    fn create_vertex_buffer(
        &mut self,
        attrs: &[VertexAttr],
        step_mode: VertexStepMode,
    ) -> Result<u64, String> {
        let (stride, inner_attrs) = get_inner_attrs(attrs);
        let kind = Kind::Vertex(VertexAttributes::new(stride, inner_attrs, step_mode));
        let mut inner_buffer = InnerBuffer::new(&self.context, kind, true)?;
        inner_buffer.bind(&self.context, Some(self.current_pipeline));
        self.buffer_count += 1;
        self.buffers.insert(self.buffer_count, inner_buffer);
        Ok(self.buffer_count)
    }

    fn create_index_buffer(&mut self) -> Result<u64, String> {
        let mut inner_buffer = InnerBuffer::new(&self.context, Kind::Index, true)?;
        inner_buffer.bind(&self.context, Some(self.current_pipeline));
        self.buffer_count += 1;
        self.buffers.insert(self.buffer_count, inner_buffer);
        Ok(self.buffer_count)
    }

    fn create_uniform_buffer(&mut self, slot: u32, name: &str) -> Result<u64, String> {
        let mut inner_buffer =
            InnerBuffer::new(&self.context, Kind::Uniform(slot, name.to_string()), true)?;
        inner_buffer.bind(&self.context, Some(self.current_pipeline));
        self.buffer_count += 1;
        self.buffers.insert(self.buffer_count, inner_buffer);
        Ok(self.buffer_count)
    }

    fn set_buffer_data(&mut self, id: u64, data: &[u8]) {
        if let Some(buffer) = self.buffers.get_mut(&id) {
            buffer.bind(&self.context, None);
            buffer.update(&self.context, data);
        }
    }

    fn render(&mut self, commands: &[Commands], target: Option<u64>) {
        commands.iter().for_each(|cmd| {
            use Commands::*;

            match cmd {
                Begin {
                    color,
                    depth,
                    stencil,
                } => self.begin(target, color, depth, stencil),
                End => self.end(),
                Pipeline { id, options } => self.set_pipeline(*id, options),
                BindBuffer { id } => self.bind_buffer(*id),
                Draw {
                    primitive,
                    offset,
                    count,
                } => self.draw(primitive, *offset, *count),
                DrawInstanced {
                    primitive,
                    offset,
                    count,
                    length,
                } => self.draw_instanced(primitive, *offset, *count, *length),
                BindTexture { id, slot, location } => self.bind_texture(*id, *slot, *location),
                Size { width, height } => self.set_size(*width, *height),
                Viewport {
                    x,
                    y,
                    width,
                    height,
                } => self.viewport(*x, *y, *width, *height, self.dpi),
                Scissors {
                    x,
                    y,
                    width,
                    height,
                } => self.scissors(*x, *y, *width, *height, self.dpi),
            }
        });
    }

    fn clean(&mut self, to_clean: &[ResourceId]) {
        to_clean.iter().for_each(|res| match &res {
            ResourceId::Pipeline(id) => self.clean_pipeline(*id),
            ResourceId::Buffer(id) => self.clean_buffer(*id),
            ResourceId::Texture(id) => self.clean_texture(*id),
            ResourceId::RenderTexture(id) => self.clean_render_target(*id),
        })
    }

    fn set_size(&mut self, width: i32, height: i32) {
        self.size = (width, height);
    }

    fn set_dpi(&mut self, scale_factor: f64) {
        self.dpi = scale_factor as _;
    }

    fn create_texture(&mut self, info: &TextureInfo) -> Result<u64, String> {
        let inner_texture = InnerTexture::new(&self.context, info)?;
        self.texture_count += 1;
        self.textures.insert(self.texture_count, inner_texture);
        Ok(self.texture_count)
    }

    fn create_render_texture(
        &mut self,
        texture_id: u64,
        info: &TextureInfo,
    ) -> Result<u64, String> {
        let texture = self.textures.get(&texture_id).ok_or(format!(
            "Error creating render target: texture id '{}' not found.",
            texture_id
        ))?;

        let inner_rt = InnerRenderTexture::new(&self.context, texture, info)?;
        self.render_target_count += 1;
        self.render_targets
            .insert(self.render_target_count, inner_rt);
        Ok(self.render_target_count)
    }

    fn update_texture(&mut self, texture: u64, opts: &TextureUpdate) -> Result<(), String> {
        match self.textures.get(&texture) {
            Some(texture) => {
                unsafe {
                    gl::BindTexture(gl::TEXTURE_2D, texture.texture);
                    gl::TexSubImage2D(
                        gl::TEXTURE_2D,
                        0,
                        opts.x_offset,
                        opts.y_offset,
                        opts.width,
                        opts.height,
                        texture_format(&opts.format), // 3d texture needs another value?
                        gl::UNSIGNED_BYTE,            // todo UNSIGNED SHORT FOR DEPTH (3d) TEXTURES
                        opts.bytes.as_ptr() as *const _,
                    );
                    // todo unbind texture?
                    Ok(())
                }
            }
            _ => Err("Invalid texture id".to_string()),
        }
    }

    fn read_pixels(
        &mut self,
        texture: u64,
        bytes: &mut [u8],
        opts: &TextureRead,
    ) -> Result<(), String> {
        match self.textures.get(&texture) {
            Some(texture) => unsafe {
                let mut fbo = 0;
                gl::GenFramebuffers(1, &mut fbo as *mut _);
                gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    texture.texture,
                    0,
                );

                let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
                let can_read = status == gl::FRAMEBUFFER_COMPLETE;

                let clean = || {
                    gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                    gl::DeleteFramebuffers(1, &fbo as *const _);
                };

                if can_read {
                    gl::ReadPixels(
                        opts.x_offset,
                        opts.y_offset,
                        opts.width,
                        opts.height,
                        texture_format(&opts.format),
                        gl::UNSIGNED_BYTE,
                        bytes.as_mut_ptr() as *mut _,
                    );
                    clean();
                    Ok(())
                } else {
                    clean();
                    Err("Framebuffer incomplete...".to_string())
                }
            },
            None => Err("Invalid texture id".to_string()),
        }
    }

    fn swap_buffers(&mut self) {
        #[cfg(target_os = "linux")]
        egl::swap_buffers(self.display, self.surface);

        #[cfg(target_os = "windows")]
        self.context.swap_buffers();
    }
}

#[inline]
pub(crate) fn clear(
    _context: &Context,
    color: &Option<Color>,
    depth: &Option<f32>,
    stencil: &Option<i32>,
) {
    let mut mask = 0;
    unsafe {
        if let Some(color) = color {
            mask |= gl::COLOR_BUFFER_BIT;
            gl::ClearColor(color.r, color.g, color.b, color.a);
        }

        if let Some(depth) = *depth {
            mask |= gl::DEPTH_BUFFER_BIT;
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(1);
            gl::ClearDepthf(depth);
        }

        if let Some(stencil) = *stencil {
            mask |= gl::STENCIL_BUFFER_BIT;
            gl::Enable(gl::STENCIL_TEST);
            gl::StencilMask(0xff);
            gl::ClearStencil(stencil);
        }

        if mask != 0 {
            gl::Clear(mask);
        }
    }
}
