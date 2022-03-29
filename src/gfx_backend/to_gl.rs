use crate::gfx::{
    buffer::{BufferUsage, VertexFormat},
    pipeline::{
        BlendFactor, BlendOperation, CompareMode, CullMode, DrawPrimitive, DrawType, StencilAction,
    },
    texture::TextureFilter,
};

use super::gl;

pub trait ToGl {
    fn to_gl(&self) -> u32;
}

pub trait ToOptionalGl {
    fn to_gl(&self) -> Option<u32>;
}

impl ToGl for StencilAction {
    fn to_gl(&self) -> u32 {
        match self {
            StencilAction::Keep => gl::KEEP,
            StencilAction::Zero => gl::ZERO,
            StencilAction::Replace => gl::REPLACE,
            StencilAction::Increment => gl::INCR,
            StencilAction::IncrementWrap => gl::INCR_WRAP,
            StencilAction::Decrement => gl::DECR,
            StencilAction::DecrementWrap => gl::DECR_WRAP,
            StencilAction::Invert => gl::INVERT,
        }
    }
}

impl ToGl for BlendOperation {
    fn to_gl(&self) -> u32 {
        match self {
            BlendOperation::Add => gl::FUNC_ADD,
            BlendOperation::Subtract => gl::FUNC_SUBTRACT,
            BlendOperation::ReverseSubtract => gl::FUNC_REVERSE_SUBTRACT,
            BlendOperation::Max => gl::MAX,
            BlendOperation::Min => gl::MIN,
        }
    }
}

impl ToGl for BlendFactor {
    fn to_gl(&self) -> u32 {
        match self {
            BlendFactor::Zero => gl::ZERO,
            BlendFactor::One => gl::ONE,
            BlendFactor::SourceAlpha => gl::SRC_ALPHA,
            BlendFactor::SourceColor => gl::SRC_COLOR,
            BlendFactor::InverseSourceAlpha => gl::ONE_MINUS_SRC_ALPHA,
            BlendFactor::InverseSourceColor => gl::ONE_MINUS_SRC_COLOR,
            BlendFactor::DestinationAlpha => gl::DST_ALPHA,
            BlendFactor::DestinationColor => gl::SRC_COLOR,
            BlendFactor::InverseDestinationAlpha => gl::ONE_MINUS_DST_ALPHA,
            BlendFactor::InverseDestinationColor => gl::ONE_MINUS_DST_COLOR,
        }
    }
}

impl ToOptionalGl for CompareMode {
    fn to_gl(&self) -> Option<u32> {
        Some(match self {
            None => return Option::None,
            CompareMode::Less => gl::LESS,
            CompareMode::Equal => gl::EQUAL,
            CompareMode::LEqual => gl::LEQUAL,
            CompareMode::Greater => gl::GREATER,
            CompareMode::NotEqual => gl::NOTEQUAL,
            CompareMode::GEqual => gl::GEQUAL,
            CompareMode::Always => gl::ALWAYS,
        })
    }
}

impl ToOptionalGl for CullMode {
    fn to_gl(&self) -> Option<u32> {
        Some(match self {
            None => return Option::None,
            CullMode::Front => gl::FRONT,
            CullMode::Back => gl::BACK,
        })
    }
}

impl ToGl for DrawType {
    fn to_gl(&self) -> u32 {
        match self {
            DrawType::Static => gl::STATIC_DRAW,
            DrawType::Dynamic => gl::DYNAMIC_DRAW,
        }
    }
}

impl ToGl for BufferUsage {
    fn to_gl(&self) -> u32 {
        match self {
            BufferUsage::Vertex => gl::ARRAY_BUFFER,
            BufferUsage::Index => gl::ELEMENT_ARRAY_BUFFER,
            BufferUsage::Uniform(_) => gl::UNIFORM_BUFFER,
        }
    }
}

impl ToGl for VertexFormat {
    fn to_gl(&self) -> u32 {
        match &self {
            VertexFormat::UInt8
            | VertexFormat::UInt8x2
            | VertexFormat::UInt8x3
            | VertexFormat::UInt8x4 => gl::UNSIGNED_BYTE,
            _ => gl::FLOAT,
        }
    }
}

impl ToGl for TextureFilter {
    fn to_gl(&self) -> u32 {
        match self {
            TextureFilter::Linear => gl::LINEAR,
            TextureFilter::Nearest => gl::NEAREST,
        }
    }
}

impl ToGl for DrawPrimitive {
    fn to_gl(&self) -> u32 {
        match self {
            DrawPrimitive::Triangles => gl::TRIANGLES,
            DrawPrimitive::TriangleStrip => gl::TRIANGLE_STRIP,
            DrawPrimitive::Lines => gl::LINES,
            DrawPrimitive::LineStrip => gl::LINE_STRIP,
        }
    }
}
