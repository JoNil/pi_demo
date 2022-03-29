use super::{
    egl::EGLContext,
    gl::{self},
    texture::InnerTexture, clear,
};
use crate::gfx::{
    color::Color,
    texture::{TextureFilter, TextureFormat, TextureInfo},
};

pub(crate) struct InnerRenderTexture {
    fbo: u32,
    depth_texture: Option<u32>,
    pub size: (i32, i32),
}

impl InnerRenderTexture {
    pub fn new(
        context: &EGLContext,
        texture: &InnerTexture,
        info: &TextureInfo,
    ) -> Result<Self, String> {
        let width = info.width;
        let height = info.height;
        let depth_info = if info.depth {
            Some(DepthInfo { width, height })
        } else {
            None
        };

        let (fbo, depth_texture) = unsafe { create_fbo(context, texture.texture, depth_info)? };
        let size = texture.size;
        Ok(Self {
            fbo,
            depth_texture,
            size,
        })
    }

    #[inline(always)]
    pub fn clean(&self, context: &EGLContext) {
        unsafe {
            gl.delete_framebuffer(self.fbo);
            if let Some(tex) = self.depth_texture {
                gl.delete_texture(tex);
            }
        }
    }

    #[inline]
    pub fn bind(&self, context: &EGLContext) {
        unsafe {
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(self.fbo));
        }
    }
}

unsafe fn create_fbo(
    context: &EGLContext,
    texture: u32,
    depth_info: Option<DepthInfo>,
) -> Result<(u32, Option<u32>), String> {
    let fbo = gl.create_framebuffer()?;
    gl.bind_framebuffer(gl::FRAMEBUFFER, Some(fbo));
    gl.framebuffer_texture_2d(
        gl::FRAMEBUFFER,
        gl::COLOR_ATTACHMENT0,
        gl::TEXTURE_2D,
        Some(texture),
        0,
    );

    let depth_texture = match depth_info {
        Some(info) => Some(create_texture(
            context,
            &TextureInfo {
                width: info.width,
                height: info.height,
                format: TextureFormat::Depth16,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
                ..Default::default()
            },
        )?),
        _ => None,
    };

    let status = gl.check_framebuffer_status(gl::FRAMEBUFFER);
    if status != gl::FRAMEBUFFER_COMPLETE {
        return Err(
            "Cannot create a render target because the frambuffer is incomplete...".to_string(),
        );
    }

    // transparent clear to avoid weird visual glitches
    clear(context, &Some(Color::TRANSPARENT), &None, &None);

    gl.bind_framebuffer(gl::FRAMEBUFFER, None);
    Ok((fbo, depth_texture))
}

struct DepthInfo {
    width: i32,
    height: i32,
}
