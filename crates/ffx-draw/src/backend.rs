//! The backend trait: the few things a graphics API must do for the
//! renderer. Only Direct3D 11 implements it (`d3d11`); Vulkan or wgpu can
//! come behind it later (M6) without the layers above changing.
//!
//! A backend owns its objects and hands out plain handles. It never makes
//! an output of its own: render targets are either made through it from a
//! description (inputs the frame needs) or brought in from the host (the
//! game's own targets, `D3d11::import_view`).

use crate::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
    TextureDesc, ViewDesc, ViewKind,
};
use crate::state::{DrawCall, DrawState};

macro_rules! handle {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
    )*};
}

handle!(
    /// A shader (any stage).
    Shader,
    /// An input layout.
    Layout,
    /// A blend state.
    Blend,
    /// A depth-stencil state.
    DepthState,
    /// A rasterizer state.
    Raster,
    /// A sampler state.
    Sampler,
    /// A buffer.
    Buffer,
    /// A 2D texture (render targets and depth buffers too).
    Texture,
    /// A view of a texture: shader resource, render target or depth-stencil.
    View,
);

/// A backend error, with what was being done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// One subresource's initial contents (`D3D11_SUBRESOURCE_DATA` of a 2D
/// texture: rows `row_pitch` bytes apart).
#[derive(Debug, Clone, Copy)]
pub struct InitData<'a> {
    pub row_pitch: u32,
    pub data: &'a [u8],
}

/// Mip 0, slice 0 of a texture as the GPU holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readback {
    pub width: u32,
    pub height: u32,
    /// DXGI format.
    pub format: u32,
    pub row_pitch: u32,
    /// Rows of `row_pitch` bytes (block rows for compressed formats).
    pub rows: u32,
    pub data: Vec<u8>,
}

/// What the renderer needs from a graphics API.
pub trait Backend {
    /// From here to [`Backend::end`] the backend may change the context's
    /// state; `end` puts back what was bound at `begin` (the host's own
    /// draws then go on as if nothing happened).
    fn begin(&mut self);
    fn end(&mut self);

    /// A shader from its DXBC.
    ///
    /// # Errors
    /// When the API refuses the bytecode.
    fn create_shader(&mut self, stage: Stage, dxbc: &[u8]) -> Result<Shader>;
    /// An input layout for `elements`, checked against the input signature
    /// of the vertex shader `vs_dxbc`.
    ///
    /// # Errors
    /// When the API refuses it.
    fn create_layout(&mut self, elements: &[InputElement], vs_dxbc: &[u8]) -> Result<Layout>;
    /// # Errors
    /// When the API refuses the description.
    fn create_blend(&mut self, desc: &BlendDesc) -> Result<Blend>;
    /// # Errors
    /// When the API refuses the description.
    fn create_depth_state(&mut self, desc: &DepthStencilDesc) -> Result<DepthState>;
    /// # Errors
    /// When the API refuses the description.
    fn create_raster(&mut self, desc: &RasterDesc) -> Result<Raster>;
    /// # Errors
    /// When the API refuses the description.
    fn create_sampler(&mut self, desc: &SamplerDesc) -> Result<Sampler>;
    /// A buffer, with its contents when given (`data.len()` must be the
    /// size).
    ///
    /// # Errors
    /// When the API refuses it.
    fn create_buffer(&mut self, desc: &BufferDesc, data: Option<&[u8]>) -> Result<Buffer>;
    /// A texture with each subresource's contents (mips of slice 0, then
    /// of slice 1, ...), or none.
    ///
    /// # Errors
    /// When the API refuses it.
    fn create_texture(&mut self, desc: &TextureDesc, init: &[InitData]) -> Result<Texture>;
    /// A view of `texture` (`None`: the whole resource as created).
    ///
    /// # Errors
    /// When the API refuses it.
    fn create_view(
        &mut self,
        texture: Texture,
        kind: ViewKind,
        desc: Option<&ViewDesc>,
    ) -> Result<View>;

    /// New contents for `buffer` from `offset` (dynamic buffers are
    /// replaced whole: `offset` 0 and the full size).
    fn write_buffer(&mut self, buffer: Buffer, offset: u32, data: &[u8]);
    fn clear_target(&mut self, view: View, color: [f32; 4]);
    /// `flags`: 1 depth, 2 stencil.
    fn clear_depth(&mut self, view: View, flags: u32, depth: f32, stencil: u8);
    fn copy_texture(&mut self, dst: Texture, src: Texture);
    /// One draw with everything it needs bound.
    fn draw(&mut self, state: &DrawState, call: DrawCall);

    /// The texture a view shows.
    fn view_texture(&self, view: View) -> Option<Texture>;
    /// # Errors
    /// When the texture can't be read back (multisampled, or the API
    /// fails).
    fn read_texture(&mut self, texture: Texture) -> Result<Readback>;
}
