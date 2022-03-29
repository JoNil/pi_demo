use std::ptr;

use super::{egl::EGLContext, gl, to_gl::ToGl};
use crate::gfx::texture::{TextureFormat, TextureInfo};

pub type TextureKey = u32;

pub(crate) struct InnerTexture {
    pub texture: TextureKey,
    pub size: (i32, i32),
}

impl InnerTexture {
    pub fn new(context: &EGLContext, info: &TextureInfo) -> Result<Self, String> {
        let texture = unsafe { create_texture(context, info)? };
        let size = (info.width, info.height);
        Ok(Self { texture, size })
    }

    pub fn bind(&self, _context: &EGLContext, slot: u32, location: &u32) {
        unsafe {
            gl::ActiveTexture(gl_slot(slot).unwrap());
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::Uniform1i(*location as _, slot as _);
        }
    }

    #[inline(always)]
    pub fn clean(self, _context: &EGLContext) {
        unsafe {
            gl::DeleteTextures(1, &self.texture as *const _);
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
    _context: &EGLContext,
    info: &TextureInfo,
) -> Result<TextureKey, String> {
    let mut texture = 0;
    gl::GenTextures(1, &mut texture as *mut _);

    let bytes_per_pixel = info.bytes_per_pixel();
    if bytes_per_pixel != 4 {
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, bytes_per_pixel as _);
    }

    gl::BindTexture(gl::TEXTURE_2D, texture);

    gl::TexParameteri(
        gl::TEXTURE_2D,
        gl::TEXTURE_MAG_FILTER,
        info.mag_filter.to_gl() as _,
    );
    gl::TexParameteri(
        gl::TEXTURE_2D,
        gl::TEXTURE_MIN_FILTER,
        info.min_filter.to_gl() as _,
    );
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

    let depth = TextureFormat::Depth16 == info.format;
    let mut data = info.bytes.as_deref();
    let mut typ = gl::UNSIGNED_BYTE;
    let mut format = texture_format(&info.format);
    if depth {
        format = gl::DEPTH_COMPONENT;
        typ = gl::UNSIGNED_SHORT;
        data = None;

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::TEXTURE_2D,
            texture,
            0,
        );
    }

    let mut c_data = ptr::null();
    if let Some(data) = data {
        c_data = data.as_ptr();
    }

    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        texture_internal_format(&info.format) as _,
        info.width,
        info.height,
        0,
        format,
        typ,
        c_data as *const _,
    );

    gl::BindTexture(gl::TEXTURE_2D, 0);

    Ok(texture)
}

pub(crate) fn texture_format(tf: &TextureFormat) -> u32 {
    match tf {
        TextureFormat::Rgba32 => gl::RGBA,
        TextureFormat::R8 => gl::RED,
        TextureFormat::Depth16 => gl::DEPTH_COMPONENT16,
    }
}

pub(crate) fn texture_internal_format(tf: &TextureFormat) -> u32 {
    match tf {
        TextureFormat::R8 => gl::R8,
        _ => texture_format(tf),
    }
}
