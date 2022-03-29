use super::{egl::EGLContext, gl};
use crate::gfx::texture::{TextureFormat, TextureInfo};

pub type TextureKey = u32;

pub(crate) struct InnerTexture {
    pub texture: TextureKey,
    pub size: (i32, i32),
    pub is_srgba: bool,
}

impl InnerTexture {
    pub fn new(context: &EGLContext, info: &TextureInfo) -> Result<Self, String> {
        let texture = unsafe { create_texture(context, info)? };
        let size = (info.width, info.height);
        let is_srgba = info.format == TextureFormat::SRgba8;
        Ok(Self {
            texture,
            size,
            is_srgba,
        })
    }

    pub fn bind(&self, context: &EGLContext, slot: u32, location: &u32) {
        unsafe {
            gl.active_texture(gl_slot(slot).unwrap());
            gl.bind_texture(gl::TEXTURE_2D, Some(self.texture));
            gl.uniform_1_i32(Some(location), slot as _);
        }
    }

    #[inline(always)]
    pub fn clean(self, context: &EGLContext) {
        unsafe {
            gl.delete_texture(self.texture);
        }
    }
}

#[inline]
fn gl_slot(slot: u32) -> Result<u32, String> {
    Ok(match slot {
        0 => gl::TEXTURE0,
        1 => gl::TEXTURE1,
        2 => gl::TEXTURE2,
        3 => gl::TEXTURE3,
        4 => gl::TEXTURE4,
        5 => gl::TEXTURE5,
        6 => gl::TEXTURE6,
        7 => gl::TEXTURE7,
        _ => return Err(format!("Unsupported texture slot '{}'", slot)),
    })
}

pub(crate) unsafe fn create_texture(
    context: &EGLContext,
    info: &TextureInfo,
) -> Result<TextureKey, String> {
    let texture = gl.create_texture()?;

    let bytes_per_pixel = info.bytes_per_pixel();
    if bytes_per_pixel != 4 {
        gl.pixel_store_i32(gl::UNPACK_ALIGNMENT, bytes_per_pixel as _);
    }

    gl.bind_texture(gl::TEXTURE_2D, Some(texture));

    gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);

    gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

    gl.tex_parameter_i32(
        gl::TEXTURE_2D,
        gl::TEXTURE_MAG_FILTER,
        info.mag_filter.to_glow() as _,
    );
    gl.tex_parameter_i32(
        gl::TEXTURE_2D,
        gl::TEXTURE_MIN_FILTER,
        info.min_filter.to_glow() as _,
    );
    gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
    gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

    let depth = TextureFormat::Depth16 == info.format;
    let mut data = info.bytes.as_deref();
    let mut typ = gl::UNSIGNED_BYTE;
    let mut format = texture_format(&info.format);
    if depth {
        format = gl::DEPTH_COMPONENT;
        typ = gl::UNSIGNED_SHORT;
        data = None;

        gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
        gl.tex_parameter_i32(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);

        gl.framebuffer_texture_2d(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::TEXTURE_2D,
            Some(texture),
            0,
        );
    }

    gl.tex_image_2d(
        gl::TEXTURE_2D,
        0,
        texture_internal_format(&info.format) as _,
        info.width,
        info.height,
        0,
        format,
        typ,
        data,
    );

    //TODO mipmaps? gl.generate_mipmap(gl::TEXTURE_2D);
    gl.bind_texture(gl::TEXTURE_2D, None);
    Ok(texture)
}

pub(crate) fn texture_format(tf: &TextureFormat) -> u32 {
    match tf {
        TextureFormat::Rgba32 => gl::RGBA,
        TextureFormat::R8 => gl::RED,
        TextureFormat::Depth16 => gl::DEPTH_COMPONENT16,
        TextureFormat::SRgba8 => gl::RGBA,
    }
}

pub(crate) fn texture_internal_format(tf: &TextureFormat) -> u32 {
    match tf {
        TextureFormat::R8 => gl::R8,
        TextureFormat::SRgba8 => gl::SRGB8_ALPHA8,
        _ => texture_format(tf),
    }
}
