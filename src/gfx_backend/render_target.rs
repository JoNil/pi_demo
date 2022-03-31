use super::{
    clear,
    gl::{self},
    texture::{create_texture, InnerTexture},
    Context,
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
        context: &Context,
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
    pub fn clean(&self, _context: &Context) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo as *const _);
            if let Some(tex) = self.depth_texture {
                gl::DeleteTextures(1, &tex as *const _);
            }
        }
    }

    #[inline]
    pub fn bind(&self, _context: &Context) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
        }
    }
}

unsafe fn create_fbo(
    context: &Context,
    texture: u32,
    depth_info: Option<DepthInfo>,
) -> Result<(u32, Option<u32>), String> {
    let mut fbo = 0;
    gl::GenFramebuffers(1, &mut fbo as *mut _);
    gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
    gl::FramebufferTexture2D(
        gl::FRAMEBUFFER,
        gl::COLOR_ATTACHMENT0,
        gl::TEXTURE_2D,
        texture,
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

    let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
    if status != gl::FRAMEBUFFER_COMPLETE {
        return Err(
            "Cannot create a render target because the frambuffer is incomplete...".to_string(),
        );
    }

    // transparent clear to avoid weird visual glitches
    clear(context, &Some(Color::TRANSPARENT), &None, &None);

    gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    Ok((fbo, depth_texture))
}

struct DepthInfo {
    width: i32,
    height: i32,
}
