use super::{
    buffer::{
        Buffer, BufferUsage, IndexBufferBuilder, UniformBufferBuilder, VertexAttr,
        VertexBufferBuilder, VertexStepMode,
    },
    commands::Commands,
    limits::Limits,
    pipeline::{Pipeline, PipelineBuilder, PipelineOptions},
    render_texture::{RenderTexture, RenderTextureBuilder},
    renderer::Renderer,
    shader::ShaderSource,
    texture::{
        Texture, TextureBuilder, TextureInfo, TextureRead, TextureReader, TextureUpdate,
        TextureUpdater,
    },
};
use std::sync::{Arc, RwLock};

/// Device resource ID, used to know which resource was dropped
#[derive(Debug)]
pub enum ResourceId {
    Buffer(u64),
    Texture(u64),
    Pipeline(u64),
    RenderTexture(u64),
}

/// Represents a the implementation graphics backend like glow, wgpu or another
pub trait DeviceBackend {
    /// Returns the name of the api used (like webgl, wgpu, etc...)
    fn api_name(&self) -> &str;

    /// Return the device limits
    fn limits(&self) -> Limits {
        Default::default()
    }

    /// Create a new pipeline and returns the id
    fn create_pipeline(
        &mut self,
        vertex_source: &[u8],
        fragment_source: &[u8],
        vertex_attrs: &[VertexAttr],
        options: PipelineOptions,
    ) -> Result<u64, String>;

    /// Create a new vertex buffer object and returns the id
    fn create_vertex_buffer(
        &mut self,
        attrs: &[VertexAttr],
        step_mode: VertexStepMode,
    ) -> Result<u64, String>;

    /// Create a new index buffer object and returns the id
    fn create_index_buffer(&mut self) -> Result<u64, String>;

    /// Create a new uniform buffer and returns the id
    fn create_uniform_buffer(&mut self, slot: u32, name: &str) -> Result<u64, String>;

    /// Upload to the GPU the buffer data slice
    fn set_buffer_data(&mut self, buffer: u64, data: &[u8]);

    /// Create a new renderer using the size of the graphics
    fn render(&mut self, commands: &[Commands], target: Option<u64>);

    /// Clean all the dropped resources
    fn clean(&mut self, to_clean: &[ResourceId]);

    /// Sets the render size
    fn set_size(&mut self, width: i32, height: i32);

    /// Sets the screen dpi
    fn set_dpi(&mut self, scale_factor: f64);

    /// Create a new texture and returns the id
    fn create_texture(&mut self, info: &TextureInfo) -> Result<u64, String>;

    /// Create a new render target and returns the id
    fn create_render_texture(&mut self, texture_id: u64, info: &TextureInfo)
        -> Result<u64, String>;

    /// Update texture data
    fn update_texture(&mut self, texture: u64, opts: &TextureUpdate) -> Result<(), String>;

    /// Read texture pixels
    fn read_pixels(
        &mut self,
        texture: u64,
        bytes: &mut [u8],
        opts: &TextureRead,
    ) -> Result<(), String>;
}

/// Helper to drop resources on the backend
/// Like pipelines, textures, buffers
#[derive(Debug, Default)]
pub(crate) struct DropManager {
    dropped: RwLock<Vec<ResourceId>>,
}

impl DropManager {
    pub fn push(&self, id: ResourceId) {
        self.dropped.write().unwrap().push(id);
    }

    pub fn clean(&self) {
        self.dropped.write().unwrap().clear();
    }
}

pub struct Device {
    size: (i32, i32),
    dpi: f64,
    backend: Box<dyn DeviceBackend>,
    drop_manager: Arc<DropManager>,
}

impl Device {
    pub fn new(backend: Box<dyn DeviceBackend>) -> Result<Self, String> {
        Ok(Self {
            backend,
            size: (1, 1),
            dpi: 1.0,
            drop_manager: Arc::new(Default::default()),
        })
    }

    #[inline]
    pub fn limits(&self) -> Limits {
        self.backend.limits()
    }

    #[inline]
    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    #[inline]
    pub fn set_size(&mut self, width: i32, height: i32) {
        self.size = (width, height);
        self.backend.set_size(width, height);
    }

    #[inline]
    pub fn dpi(&self) -> f64 {
        self.dpi
    }

    #[inline]
    pub fn set_dpi(&mut self, scale_factor: f64) {
        self.dpi = scale_factor;
        self.backend.set_dpi(scale_factor);
    }

    #[inline]
    pub fn api_name(&self) -> &str {
        self.backend.api_name()
    }

    #[inline]
    pub fn create_renderer(&self) -> Renderer {
        Renderer::new(self.size.0, self.size.1)
    }

    /// Creates a Pipeline builder
    #[inline]
    pub fn create_pipeline(&mut self) -> PipelineBuilder {
        PipelineBuilder::new(self)
    }

    /// Creates a texture builder
    #[inline]
    pub fn create_texture(&mut self) -> TextureBuilder {
        TextureBuilder::new(self)
    }

    /// Creates a render texture builder
    #[inline]
    pub fn create_render_texture(&mut self, width: i32, height: i32) -> RenderTextureBuilder {
        RenderTextureBuilder::new(self, width, height)
    }

    /// Creates a vertex buffer builder
    #[inline]
    pub fn create_vertex_buffer(&mut self) -> VertexBufferBuilder {
        VertexBufferBuilder::new(self)
    }

    /// Creates a index buffer builder
    #[inline]
    pub fn create_index_buffer(&mut self) -> IndexBufferBuilder {
        IndexBufferBuilder::new(self)
    }

    /// Creates a uniform buffer builder
    #[inline]
    pub fn create_uniform_buffer(&mut self, slot: u32, name: &str) -> UniformBufferBuilder {
        UniformBufferBuilder::new(self, slot, name)
    }

    /// Update the texture data
    #[inline]
    pub fn update_texture<'a>(&'a mut self, texture: &'a mut Texture) -> TextureUpdater {
        TextureUpdater::new(self, texture)
    }

    /// Read pixels from a texture
    #[inline]
    pub fn read_pixels<'a>(&'a mut self, texture: &'a Texture) -> TextureReader {
        TextureReader::new(self, texture)
    }

    #[inline]
    pub(crate) fn inner_create_pipeline_from_raw(
        &mut self,
        vertex_source: &[u8],
        fragment_source: &[u8],
        vertex_attrs: &[VertexAttr],
        options: PipelineOptions,
    ) -> Result<Pipeline, String> {
        let stride = vertex_attrs
            .iter()
            .fold(0, |acc, data| acc + data.format.bytes()) as usize;

        let id = self.backend.create_pipeline(
            vertex_source,
            fragment_source,
            vertex_attrs,
            options.clone(),
        )?;

        Ok(Pipeline::new(
            id,
            stride,
            options,
            self.drop_manager.clone(),
        ))
    }

    #[inline]
    pub(crate) fn inner_create_pipeline(
        &mut self,
        vertex_source: &ShaderSource,
        fragment_source: &ShaderSource,
        vertex_attrs: &[VertexAttr],
        options: PipelineOptions,
    ) -> Result<Pipeline, String> {
        let api = self.backend.api_name();
        let vertex = vertex_source
            .get_source(api)
            .ok_or(format!("Vertex shader for api '{}' not available.", api))?;
        let fragment = fragment_source
            .get_source(api)
            .ok_or(format!("Fragment shader for api '{}' not available.", api))?;
        self.inner_create_pipeline_from_raw(vertex, fragment, vertex_attrs, options)
    }

    #[inline(always)]
    pub(crate) fn inner_create_vertex_buffer(
        &mut self,
        data: Option<&[f32]>,
        attrs: &[VertexAttr],
        step_mode: VertexStepMode,
    ) -> Result<Buffer, String> {
        let id = self.backend.create_vertex_buffer(attrs, step_mode)?;

        let buffer = Buffer::new(id, BufferUsage::Vertex, None, self.drop_manager.clone());

        if let Some(d) = data {
            self.set_buffer_data(&buffer, d);
        }

        Ok(buffer)
    }

    #[inline]
    pub(crate) fn inner_create_index_buffer(
        &mut self,
        data: Option<&[u32]>,
    ) -> Result<Buffer, String> {
        let id = self.backend.create_index_buffer()?;

        let buffer = Buffer::new(id, BufferUsage::Index, None, self.drop_manager.clone());

        if let Some(d) = data {
            self.set_buffer_data(&buffer, d);
        }
        Ok(buffer)
    }

    #[inline]
    pub(crate) fn inner_create_uniform_buffer(
        &mut self,
        slot: u32,
        name: &str,
        data: Option<&[f32]>,
    ) -> Result<Buffer, String> {
        //debug_assert!(current_pipeline.is_some()) //pipeline should be already binded
        let id = self.backend.create_uniform_buffer(slot, name)?;
        let buffer = Buffer::new(
            id,
            BufferUsage::Uniform(slot),
            None,
            self.drop_manager.clone(),
        );

        if let Some(d) = data {
            self.set_buffer_data(&buffer, d);
        }

        Ok(buffer)
    }

    #[inline]
    pub(crate) fn inner_create_texture(&mut self, info: TextureInfo) -> Result<Texture, String> {
        let id = self.backend.create_texture(&info)?;
        Ok(Texture::new(id, info, self.drop_manager.clone()))
    }

    #[inline]
    pub(crate) fn inner_create_render_texture(
        &mut self,
        info: TextureInfo,
    ) -> Result<RenderTexture, String> {
        let tex_id = self.backend.create_texture(&info)?;

        let id = self.backend.create_render_texture(tex_id, &info)?;
        let texture = Texture::new(tex_id, info, self.drop_manager.clone());
        Ok(RenderTexture::new(id, texture, self.drop_manager.clone()))
    }

    #[inline]
    pub fn render(&mut self, commands: &[Commands]) {
        self.backend.render(commands, None);
    }

    #[inline]
    pub fn render_to(&mut self, target: &RenderTexture, commands: &[Commands]) {
        self.backend.render(commands, Some(target.id()));
    }

    #[inline]
    pub(crate) fn inner_update_texture(
        &mut self,
        texture: &mut Texture,
        opts: &TextureUpdate,
    ) -> Result<(), String> {
        self.backend.update_texture(texture.id(), opts)
    }

    #[inline]
    pub(crate) fn inner_read_pixels(
        &mut self,
        texture: &Texture,
        bytes: &mut [u8],
        opts: &TextureRead,
    ) -> Result<(), String> {
        self.backend.read_pixels(texture.id(), bytes, opts)
    }

    #[inline]
    pub fn clean(&mut self) {
        if self.drop_manager.dropped.read().unwrap().is_empty() {
            return;
        }

        self.backend
            .clean(&self.drop_manager.dropped.read().unwrap());
        self.drop_manager.clean();
    }

    #[inline]
    pub fn set_buffer_data<T: BufferDataType>(&mut self, buffer: &Buffer, data: &[T]) {
        self.backend
            .set_buffer_data(buffer.id(), bytemuck::cast_slice(data));
    }
}

pub trait BufferDataType: bytemuck::Pod {}
impl BufferDataType for u32 {}
impl BufferDataType for f32 {}