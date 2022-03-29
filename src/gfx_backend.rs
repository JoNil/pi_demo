pub mod egl;
pub mod gl;

pub struct GlesBackend {}

impl GlesBackend {
    pub fn new() -> Self {
        Self {}
    }
}
