use glam::{Mat4, Vec3};
use std::f32::consts::PI;

pub struct Camera {
    pos: Vec3,
}

impl Camera {
    pub fn new() -> Self {
        Self { pos: Vec3::ZERO }
    }

    pub fn update(&mut self, size: (i32, i32)) -> Mat4 {
        let proj = Mat4::perspective_rh_gl(PI / 2.0, size.0 as f32 / size.1 as f32, 0.01, 1000.0);

        proj
    }
}
