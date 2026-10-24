//! What one draw has bound: the pipeline (shaders, layout, fixed-function
//! states) and the resources per slot, as plain values a backend applies.

use crate::backend::{Blend, Buffer, DepthState, Layout, Raster, Sampler, Shader, View};

/// Constant buffer slots per stage (`D3D11_COMMONSHADER_CONSTANT_BUFFER_API_SLOT_COUNT`).
pub const CB_SLOTS: usize = 14;
/// Shader resource and sampler slots kept per stage.
pub const SRV_SLOTS: usize = 16;
pub const SAMPLER_SLOTS: usize = 16;
/// Vertex buffer slots kept.
pub const VB_SLOTS: usize = 16;
/// Simultaneous render targets.
pub const RT_SLOTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VertexBinding {
    pub buffer: Buffer,
    pub stride: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexBinding {
    pub buffer: Buffer,
    /// DXGI format: 57 `R16_UINT`, 42 `R32_UINT`.
    pub format: u32,
    pub offset: u32,
}

/// A constant buffer bound to a slot: the window from `first` for `count`
/// 16-byte constants (`count` 0: the whole buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CbBinding {
    pub buffer: Buffer,
    pub first: u32,
    pub count: u32,
}

/// One shader stage's resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StageBindings {
    pub shader: Option<Shader>,
    pub cbs: [Option<CbBinding>; CB_SLOTS],
    pub srvs: [Option<View>; SRV_SLOTS],
    pub samplers: [Option<Sampler>; SAMPLER_SLOTS],
}

/// Everything a draw sees. `None` is the API's default (a null state).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DrawState {
    /// `D3D11_PRIMITIVE_TOPOLOGY`.
    pub topology: u32,
    pub layout: Option<Layout>,
    pub vertex: [Option<VertexBinding>; VB_SLOTS],
    pub index: Option<IndexBinding>,
    pub vs: StageBindings,
    pub ps: StageBindings,
    /// Only the shader: geometry shaders (the capture records none) would
    /// need their own resources.
    pub gs: Option<Shader>,
    pub blend: Option<Blend>,
    pub blend_factor: [f32; 4],
    pub sample_mask: u32,
    pub depth: Option<DepthState>,
    pub stencil_ref: u32,
    pub raster: Option<Raster>,
    /// `D3D11_VIEWPORT`: x, y, width, height, min depth, max depth.
    pub viewport: [f32; 6],
    /// Left, top, right, bottom.
    pub scissor: [i32; 4],
    pub targets: [Option<View>; RT_SLOTS],
    pub depth_target: Option<View>,
}

/// The draw call itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawCall {
    Indexed {
        count: u32,
        start: u32,
        base: i32,
    },
    Plain {
        count: u32,
        start: u32,
    },
    IndexedInstanced {
        count: u32,
        instances: u32,
        start: u32,
        base: i32,
        first_instance: u32,
    },
    Instanced {
        count: u32,
        instances: u32,
        start: u32,
        first_instance: u32,
    },
}
