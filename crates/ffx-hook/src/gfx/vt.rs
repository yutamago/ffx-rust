//! Vtable slots of the D3D11 interfaces the graphics layer hooks (the
//! declaration order in `d3d11.h`).

/// `ID3D11Device`.
pub mod device {
    pub const CREATE_BUFFER: usize = 3;
    pub const CREATE_TEXTURE_2D: usize = 5;
    pub const CREATE_SHADER_RESOURCE_VIEW: usize = 7;
    pub const CREATE_RENDER_TARGET_VIEW: usize = 9;
    pub const CREATE_DEPTH_STENCIL_VIEW: usize = 10;
    pub const CREATE_INPUT_LAYOUT: usize = 11;
    pub const CREATE_VERTEX_SHADER: usize = 12;
    pub const CREATE_GEOMETRY_SHADER: usize = 13;
    pub const CREATE_PIXEL_SHADER: usize = 15;
    pub const CREATE_COMPUTE_SHADER: usize = 18;
    pub const CREATE_BLEND_STATE: usize = 20;
    pub const CREATE_DEPTH_STENCIL_STATE: usize = 21;
    pub const CREATE_RASTERIZER_STATE: usize = 22;
    pub const CREATE_SAMPLER_STATE: usize = 23;
    pub const CREATE_DEFERRED_CONTEXT: usize = 27;
    pub const GET_IMMEDIATE_CONTEXT: usize = 40;
}

/// `ID3D11DeviceContext`.
pub mod context {
    pub const VS_SET_CONSTANT_BUFFERS: usize = 7;
    pub const PS_SET_SHADER_RESOURCES: usize = 8;
    pub const PS_SET_SHADER: usize = 9;
    pub const PS_SET_SAMPLERS: usize = 10;
    pub const VS_SET_SHADER: usize = 11;
    pub const DRAW_INDEXED: usize = 12;
    pub const DRAW: usize = 13;
    pub const MAP: usize = 14;
    pub const UNMAP: usize = 15;
    pub const PS_SET_CONSTANT_BUFFERS: usize = 16;
    pub const IA_SET_INPUT_LAYOUT: usize = 17;
    pub const IA_SET_VERTEX_BUFFERS: usize = 18;
    pub const IA_SET_INDEX_BUFFER: usize = 19;
    pub const DRAW_INDEXED_INSTANCED: usize = 20;
    pub const DRAW_INSTANCED: usize = 21;
    pub const GS_SET_SHADER: usize = 23;
    pub const IA_SET_PRIMITIVE_TOPOLOGY: usize = 24;
    pub const VS_SET_SHADER_RESOURCES: usize = 25;
    pub const VS_SET_SAMPLERS: usize = 26;
    pub const OM_SET_RENDER_TARGETS: usize = 33;
    pub const OM_SET_RENDER_TARGETS_AND_UAVS: usize = 34;
    pub const OM_SET_BLEND_STATE: usize = 35;
    pub const OM_SET_DEPTH_STENCIL_STATE: usize = 36;
    pub const DISPATCH: usize = 41;
    pub const RS_SET_STATE: usize = 43;
    pub const RS_SET_VIEWPORTS: usize = 44;
    pub const RS_SET_SCISSOR_RECTS: usize = 45;
    pub const COPY_SUBRESOURCE_REGION: usize = 46;
    pub const COPY_RESOURCE: usize = 47;
    pub const UPDATE_SUBRESOURCE: usize = 48;
    pub const CLEAR_RENDER_TARGET_VIEW: usize = 50;
    pub const CLEAR_DEPTH_STENCIL_VIEW: usize = 53;
    pub const GENERATE_MIPS: usize = 54;
    pub const RESOLVE_SUBRESOURCE: usize = 57;
    pub const CS_SET_SHADER: usize = 69;
    pub const EXECUTE_COMMAND_LIST: usize = 58;
    // ID3D11DeviceContext1
    pub const COPY_SUBRESOURCE_REGION_1: usize = 115;
    pub const UPDATE_SUBRESOURCE_1: usize = 116;
    pub const VS_SET_CONSTANT_BUFFERS_1: usize = 119;
    pub const PS_SET_CONSTANT_BUFFERS_1: usize = 123;
}

/// `IUnknown::Release`.
pub const RELEASE: usize = 2;
/// `ID3D11DeviceChild::GetDevice`.
pub const GET_DEVICE: usize = 3;
/// `ID3D11View::GetResource`.
pub const VIEW_GET_RESOURCE: usize = 7;
/// `ID3D11Texture2D::GetDesc` (also `ID3D11Buffer::GetDesc`).
pub const RESOURCE_GET_DESC: usize = 10;
/// `IDXGISwapChain::Present`.
pub const PRESENT: usize = 8;
/// `IDXGISwapChain::GetBuffer`.
pub const GET_BUFFER: usize = 9;
