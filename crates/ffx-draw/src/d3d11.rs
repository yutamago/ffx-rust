//! The Direct3D 11 backend, on a device and immediate context it is given
//! (in the game: the game's own, the context unwrapped from the graphics
//! layer's proxy so that its hooks don't see these calls).
//!
//! It makes no output of its own: render targets come from the host
//! ([`D3d11::import_view`]) or are inputs made from a description.
//! [`Backend::begin`] saves everything the backend can change on the
//! context and [`Backend::end`] puts it back, so the host's next calls see
//! the state it set itself.

// D3D11's enums are i32 newtypes of the u32 numbers the descriptions keep;
// handles, slot counts and sizes fit in u32.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_FLAG, D3D11_BLEND, D3D11_BLEND_DESC, D3D11_BLEND_OP, D3D11_BOX, D3D11_BUFFER_DESC,
    D3D11_COMPARISON_FUNC, D3D11_CPU_ACCESS_READ, D3D11_CULL_MODE, D3D11_DEPTH_STENCIL_DESC,
    D3D11_DEPTH_STENCIL_VIEW_DESC, D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_WRITE_MASK,
    D3D11_FILL_MODE, D3D11_FILTER, D3D11_INPUT_CLASSIFICATION, D3D11_INPUT_ELEMENT_DESC,
    D3D11_MAP_READ, D3D11_MAP_WRITE_DISCARD, D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC,
    D3D11_RENDER_TARGET_BLEND_DESC, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_SAMPLER_DESC,
    D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_STENCIL_OP, D3D11_SUBRESOURCE_DATA,
    D3D11_TEXTURE_ADDRESS_MODE, D3D11_TEXTURE2D_DESC, D3D11_USAGE, D3D11_USAGE_STAGING,
    D3D11_VIEWPORT, ID3D11BlendState, ID3D11Buffer, ID3D11DepthStencilState,
    ID3D11DepthStencilView, ID3D11Device, ID3D11DeviceContext, ID3D11DeviceContext1,
    ID3D11GeometryShader, ID3D11InputLayout, ID3D11PixelShader, ID3D11RasterizerState,
    ID3D11RenderTargetView, ID3D11Resource, ID3D11SamplerState, ID3D11ShaderResourceView,
    ID3D11Texture2D, ID3D11UnorderedAccessView, ID3D11VertexShader,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC};
use windows::core::{BOOL, Interface, PCSTR};

use crate::backend::{
    Backend, Blend, Buffer, DepthState, Error, InitData, Layout, Raster, Readback, Result, Sampler,
    Shader, Texture, View,
};
use crate::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
    TextureDesc, USAGE_DYNAMIC, ViewDesc, ViewKind,
};
use crate::state::{
    CB_SLOTS, CbBinding, DrawCall, DrawState, IndexBinding, RT_SLOTS, SAMPLER_SLOTS, SRV_SLOTS,
    StageBindings, VB_SLOTS, VertexBinding,
};

enum ShaderObj {
    Vertex(ID3D11VertexShader),
    Pixel(ID3D11PixelShader),
    Geometry(ID3D11GeometryShader),
}

enum ViewObj {
    Srv(ID3D11ShaderResourceView),
    Rtv(ID3D11RenderTargetView),
    Dsv(ID3D11DepthStencilView),
}

struct BufferObj {
    buffer: ID3D11Buffer,
    desc: BufferDesc,
    /// What was last written through the backend (for the replayed
    /// stream: constant buffer contents as the draw saw them).
    written: Vec<u8>,
}

/// Everything [`D3d11::draw`] can change, as the context had it.
#[derive(Default)]
struct Saved {
    layout: Option<ID3D11InputLayout>,
    topology: D3D_PRIMITIVE_TOPOLOGY,
    vb: [Option<ID3D11Buffer>; VB_SLOTS],
    vb_strides: [u32; VB_SLOTS],
    vb_offsets: [u32; VB_SLOTS],
    ib: Option<ID3D11Buffer>,
    ib_format: DXGI_FORMAT,
    ib_offset: u32,
    vs: Option<ID3D11VertexShader>,
    ps: Option<ID3D11PixelShader>,
    gs: Option<ID3D11GeometryShader>,
    vs_cb: SavedCbs,
    ps_cb: SavedCbs,
    vs_srv: [Option<ID3D11ShaderResourceView>; SRV_SLOTS],
    ps_srv: [Option<ID3D11ShaderResourceView>; SRV_SLOTS],
    vs_samplers: [Option<ID3D11SamplerState>; SAMPLER_SLOTS],
    ps_samplers: [Option<ID3D11SamplerState>; SAMPLER_SLOTS],
    raster: Option<ID3D11RasterizerState>,
    viewports: Vec<D3D11_VIEWPORT>,
    scissors: Vec<RECT>,
    rtv: [Option<ID3D11RenderTargetView>; RT_SLOTS],
    dsv: Option<ID3D11DepthStencilView>,
    uav: [Option<ID3D11UnorderedAccessView>; RT_SLOTS],
    blend: Option<ID3D11BlendState>,
    blend_factor: [f32; 4],
    sample_mask: u32,
    depth: Option<ID3D11DepthStencilState>,
    stencil_ref: u32,
}

#[derive(Default)]
struct SavedCbs {
    buffers: [Option<ID3D11Buffer>; CB_SLOTS],
    first: [u32; CB_SLOTS],
    count: [u32; CB_SLOTS],
}

/// One set of objects: those the backend made (kept for its lifetime), or
/// those it imported from the host (kept until [`D3d11::release_imports`]).
#[derive(Default)]
struct Store {
    shaders: Vec<ShaderObj>,
    layouts: Vec<ID3D11InputLayout>,
    blends: Vec<ID3D11BlendState>,
    depths: Vec<ID3D11DepthStencilState>,
    rasters: Vec<ID3D11RasterizerState>,
    samplers: Vec<ID3D11SamplerState>,
    buffers: Vec<BufferObj>,
    textures: Vec<(ID3D11Texture2D, TextureDesc)>,
    /// A view and the texture behind it (`None`: not a 2D texture).
    views: Vec<(ViewObj, Option<Texture>)>,
    /// Raw object address → handle, for reading the bound state back.
    known: HashMap<usize, u32>,
}

/// The handle bit of imported objects: their index in the import store.
const IMPORTED: u32 = 1 << 31;

/// What a host object is, for [`D3d11::import`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Import {
    Shader(Stage),
    Layout,
    Blend,
    DepthState,
    Raster,
    Sampler,
    Buffer,
    View(ViewKind),
}

/// A constant buffer the context has bound: the raw buffer and its window
/// in 16-byte constants (`count` 0: the whole buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundCb {
    pub buffer: usize,
    pub first: u32,
    pub count: u32,
}

/// [`D3d11::import_bound`]'s result.
#[derive(Debug, Clone)]
pub struct BoundNow {
    /// Everything but the constant buffers, in imported handles.
    pub state: DrawState,
    /// Per stage (vertex, pixel) and slot.
    pub cbs: [[Option<BoundCb>; CB_SLOTS]; 2],
    /// Raw resources the draw reads: vertex and index buffers, the
    /// resources behind the shader resource views.
    pub reads: Vec<usize>,
}

/// The Direct3D 11 backend.
pub struct D3d11 {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    context1: Option<ID3D11DeviceContext1>,
    own: Store,
    imported: Store,
    /// What the last draw applied (`None`: unknown, set everything).
    applied: Option<DrawState>,
    saved: Option<Box<Saved>>,
}

fn err(what: &str, e: &windows::core::Error) -> Error {
    Error(format!("{what}: {e}"))
}

fn raw<T: Interface>(o: &T) -> usize {
    o.as_raw() as usize
}

fn bool32(b: bool) -> BOOL {
    BOOL::from(b)
}

impl D3d11 {
    /// A backend on `device` and `context` (taking a reference on each).
    ///
    /// # Safety
    /// `device` must be a live `ID3D11Device` and `context` its immediate
    /// context (the real one, not a proxy), used only on the thread that
    /// owns it while the backend draws.
    #[must_use]
    pub unsafe fn borrow(device: *mut c_void, context: *mut c_void) -> Option<Self> {
        // SAFETY: the caller's live COM objects; cloning takes a reference.
        let (device, context) = unsafe {
            (
                ID3D11Device::from_raw_borrowed(&device)?.clone(),
                ID3D11DeviceContext::from_raw_borrowed(&context)?.clone(),
            )
        };
        let context1 = context.cast::<ID3D11DeviceContext1>().ok();
        Some(Self {
            device,
            context,
            context1,
            own: Store::default(),
            imported: Store::default(),
            applied: None,
            saved: None,
        })
    }

    /// Brings in one of the host's views (a render target, depth buffer or
    /// shader resource), and the texture behind it.
    ///
    /// # Safety
    /// `view` must be a live view of `kind` on this backend's device.
    pub unsafe fn import_view(
        &mut self,
        view: *mut c_void,
        kind: ViewKind,
    ) -> Option<(View, Texture)> {
        // SAFETY: the caller's live view; cloning takes a reference.
        let (obj, resource) = unsafe {
            match kind {
                ViewKind::ShaderResource => {
                    let v = ID3D11ShaderResourceView::from_raw_borrowed(&view)?.clone();
                    let r = v.GetResource().ok()?;
                    (ViewObj::Srv(v), r)
                }
                ViewKind::RenderTarget => {
                    let v = ID3D11RenderTargetView::from_raw_borrowed(&view)?.clone();
                    let r = v.GetResource().ok()?;
                    (ViewObj::Rtv(v), r)
                }
                ViewKind::DepthStencil => {
                    let v = ID3D11DepthStencilView::from_raw_borrowed(&view)?.clone();
                    let r = v.GetResource().ok()?;
                    (ViewObj::Dsv(v), r)
                }
            }
        };
        let tex: ID3D11Texture2D = resource.cast().ok()?;
        let mut d = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: a live texture.
        unsafe { tex.GetDesc(&raw mut d) };
        let desc = texture_desc_of(&d);
        let t = match self.raw_handle(raw(&tex)) {
            Some(h) => Texture(h),
            None => self.add_texture(true, tex, desc),
        };
        Some((self.add_view(true, obj, Some(t)), t))
    }

    /// Brings in one of the host's objects, taking a reference on it until
    /// [`D3d11::release_imports`]; the handle's number for the type `kind`
    /// names (`Shader(n)`, `Buffer(n)`, ...). An object the backend knows
    /// already (made or imported) gives its handle again. Views of any
    /// resource can be imported this way; [`Backend::view_texture`] is
    /// `None` for those not of a 2D texture.
    ///
    /// # Safety
    /// `object` must be a live object of `kind` on this backend's device.
    #[allow(clippy::too_many_lines)] // one arm per kind
    pub unsafe fn import(&mut self, object: *mut c_void, kind: Import) -> Option<u32> {
        if object.is_null() {
            return None;
        }
        let p = object as usize;
        if let Some(h) = self.raw_handle(p) {
            return Some(h);
        }
        // SAFETY: the caller's live object of that kind; cloning takes a
        // reference.
        unsafe {
            Some(match kind {
                Import::Shader(Stage::Vertex) => {
                    let o = ID3D11VertexShader::from_raw_borrowed(&object)?.clone();
                    push(
                        &mut self.imported,
                        IMPORTED,
                        p,
                        |s| &mut s.shaders,
                        ShaderObj::Vertex(o),
                    )
                }
                Import::Shader(Stage::Pixel) => {
                    let o = ID3D11PixelShader::from_raw_borrowed(&object)?.clone();
                    push(
                        &mut self.imported,
                        IMPORTED,
                        p,
                        |s| &mut s.shaders,
                        ShaderObj::Pixel(o),
                    )
                }
                Import::Shader(Stage::Geometry) => {
                    let o = ID3D11GeometryShader::from_raw_borrowed(&object)?.clone();
                    push(
                        &mut self.imported,
                        IMPORTED,
                        p,
                        |s| &mut s.shaders,
                        ShaderObj::Geometry(o),
                    )
                }
                Import::Layout => {
                    let o = ID3D11InputLayout::from_raw_borrowed(&object)?.clone();
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.layouts, o)
                }
                Import::Blend => {
                    let o = ID3D11BlendState::from_raw_borrowed(&object)?.clone();
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.blends, o)
                }
                Import::DepthState => {
                    let o = ID3D11DepthStencilState::from_raw_borrowed(&object)?.clone();
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.depths, o)
                }
                Import::Raster => {
                    let o = ID3D11RasterizerState::from_raw_borrowed(&object)?.clone();
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.rasters, o)
                }
                Import::Sampler => {
                    let o = ID3D11SamplerState::from_raw_borrowed(&object)?.clone();
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.samplers, o)
                }
                Import::Buffer => {
                    let o = ID3D11Buffer::from_raw_borrowed(&object)?.clone();
                    let mut d = D3D11_BUFFER_DESC::default();
                    o.GetDesc(&raw mut d);
                    let desc = BufferDesc {
                        size: d.ByteWidth,
                        usage: d.Usage.0 as u32,
                        bind: d.BindFlags,
                        cpu_access: d.CPUAccessFlags,
                        misc: d.MiscFlags,
                        stride: d.StructureByteStride,
                    };
                    let obj = BufferObj {
                        buffer: o,
                        desc,
                        written: Vec::new(),
                    };
                    push(&mut self.imported, IMPORTED, p, |s| &mut s.buffers, obj)
                }
                Import::View(kind) => {
                    let (obj, resource) = match kind {
                        ViewKind::ShaderResource => {
                            let v = ID3D11ShaderResourceView::from_raw_borrowed(&object)?.clone();
                            let r = v.GetResource().ok();
                            (ViewObj::Srv(v), r)
                        }
                        ViewKind::RenderTarget => {
                            let v = ID3D11RenderTargetView::from_raw_borrowed(&object)?.clone();
                            let r = v.GetResource().ok();
                            (ViewObj::Rtv(v), r)
                        }
                        ViewKind::DepthStencil => {
                            let v = ID3D11DepthStencilView::from_raw_borrowed(&object)?.clone();
                            let r = v.GetResource().ok();
                            (ViewObj::Dsv(v), r)
                        }
                    };
                    let tex = resource.and_then(|r| r.cast::<ID3D11Texture2D>().ok());
                    let t = tex.map(|tex| match self.raw_handle(raw(&tex)) {
                        Some(h) => Texture(h),
                        None => {
                            let mut d = D3D11_TEXTURE2D_DESC::default();
                            tex.GetDesc(&raw mut d);
                            self.add_texture(true, tex, texture_desc_of(&d))
                        }
                    });
                    self.add_view(true, obj, t).0
                }
            })
        }
    }

    /// Drops the references [`D3d11::import`] and [`D3d11::import_view`]
    /// took. Their handles are invalid afterwards (and may be given out
    /// again); call it outside `begin`..`end`.
    pub fn release_imports(&mut self) {
        self.imported = Store::default();
        self.applied = None;
    }

    /// Objects imported since the last release.
    #[must_use]
    pub fn imports(&self) -> usize {
        self.imported.known.len()
    }

    /// The description of a texture.
    #[must_use]
    pub fn texture_desc(&self, t: Texture) -> Option<TextureDesc> {
        self.texture_obj(t).map(|x| x.1)
    }

    /// The bytes last written to a buffer through the backend.
    #[must_use]
    pub fn written(&self, b: Buffer) -> Option<&[u8]> {
        self.buffer_obj(b).map(|x| x.written.as_slice())
    }

    /// The store a handle number is in, and its index there.
    fn part(&self, h: u32) -> (&Store, usize) {
        if h & IMPORTED == 0 {
            (&self.own, h as usize)
        } else {
            (&self.imported, (h & !IMPORTED) as usize)
        }
    }

    fn shader_obj(&self, h: Shader) -> Option<&ShaderObj> {
        let (s, i) = self.part(h.0);
        s.shaders.get(i)
    }

    fn layout_obj(&self, h: Layout) -> Option<&ID3D11InputLayout> {
        let (s, i) = self.part(h.0);
        s.layouts.get(i)
    }

    fn blend_obj(&self, h: Blend) -> Option<&ID3D11BlendState> {
        let (s, i) = self.part(h.0);
        s.blends.get(i)
    }

    fn depth_obj(&self, h: DepthState) -> Option<&ID3D11DepthStencilState> {
        let (s, i) = self.part(h.0);
        s.depths.get(i)
    }

    fn raster_obj(&self, h: Raster) -> Option<&ID3D11RasterizerState> {
        let (s, i) = self.part(h.0);
        s.rasters.get(i)
    }

    fn sampler_obj(&self, h: Sampler) -> Option<&ID3D11SamplerState> {
        let (s, i) = self.part(h.0);
        s.samplers.get(i)
    }

    fn buffer_obj(&self, h: Buffer) -> Option<&BufferObj> {
        let (s, i) = self.part(h.0);
        s.buffers.get(i)
    }

    fn texture_obj(&self, h: Texture) -> Option<&(ID3D11Texture2D, TextureDesc)> {
        let (s, i) = self.part(h.0);
        s.textures.get(i)
    }

    fn view_obj(&self, h: View) -> Option<&(ViewObj, Option<Texture>)> {
        let (s, i) = self.part(h.0);
        s.views.get(i)
    }

    fn add_texture(&mut self, imported: bool, tex: ID3D11Texture2D, desc: TextureDesc) -> Texture {
        let p = raw(&tex);
        let (store, bit) = self.store_mut(imported);
        Texture(push(store, bit, p, |s| &mut s.textures, (tex, desc)))
    }

    fn add_view(&mut self, imported: bool, obj: ViewObj, t: Option<Texture>) -> View {
        let p = match &obj {
            ViewObj::Srv(v) => raw(v),
            ViewObj::Rtv(v) => raw(v),
            ViewObj::Dsv(v) => raw(v),
        };
        let (store, bit) = self.store_mut(imported);
        View(push(store, bit, p, |s| &mut s.views, (obj, t)))
    }

    fn store_mut(&mut self, imported: bool) -> (&mut Store, u32) {
        if imported {
            (&mut self.imported, IMPORTED)
        } else {
            (&mut self.own, 0)
        }
    }

    fn srv(&self, v: Option<View>) -> Option<ID3D11ShaderResourceView> {
        match self.view_obj(v?) {
            Some((ViewObj::Srv(x), _)) => Some(x.clone()),
            _ => None,
        }
    }

    fn rtv(&self, v: Option<View>) -> Option<ID3D11RenderTargetView> {
        match self.view_obj(v?) {
            Some((ViewObj::Rtv(x), _)) => Some(x.clone()),
            _ => None,
        }
    }

    fn dsv(&self, v: Option<View>) -> Option<ID3D11DepthStencilView> {
        match self.view_obj(v?) {
            Some((ViewObj::Dsv(x), _)) => Some(x.clone()),
            _ => None,
        }
    }

    fn buffer(&self, b: Option<Buffer>) -> Option<&ID3D11Buffer> {
        self.buffer_obj(b?).map(|x| &x.buffer)
    }

    fn raw_handle(&self, p: usize) -> Option<u32> {
        self.own
            .known
            .get(&p)
            .or_else(|| self.imported.known.get(&p))
            .copied()
    }

    fn handle<T: Interface>(&self, o: Option<&T>) -> Option<u32> {
        self.raw_handle(raw(o?))
    }

    /// Applies the parts of `s` that differ from what the last draw set.
    #[allow(clippy::too_many_lines)] // one block per piece of state
    fn apply(&mut self, s: &DrawState) {
        let last = self.applied.take();
        let changed = |f: &dyn Fn(&DrawState) -> bool| last.as_ref().is_none_or(|l| !f(l));
        let c = &self.context;
        // SAFETY: live objects of this device, on the context's thread.
        unsafe {
            if changed(&|l| l.topology == s.topology) {
                c.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY(s.topology as i32));
            }
            if changed(&|l| l.layout == s.layout) {
                let l = s.layout.and_then(|l| self.layout_obj(l));
                c.IASetInputLayout(l);
            }
            if changed(&|l| l.vertex == s.vertex) {
                let mut bufs: [Option<ID3D11Buffer>; VB_SLOTS] = Default::default();
                let mut strides = [0u32; VB_SLOTS];
                let mut offsets = [0u32; VB_SLOTS];
                for (i, v) in s.vertex.iter().enumerate() {
                    if let Some(v) = v {
                        bufs[i] = self.buffer(Some(v.buffer)).cloned();
                        strides[i] = v.stride;
                        offsets[i] = v.offset;
                    }
                }
                c.IASetVertexBuffers(
                    0,
                    VB_SLOTS as u32,
                    Some(bufs.as_ptr()),
                    Some(strides.as_ptr()),
                    Some(offsets.as_ptr()),
                );
            }
            if changed(&|l| l.index == s.index) {
                match s.index {
                    Some(i) => c.IASetIndexBuffer(
                        self.buffer(Some(i.buffer)),
                        DXGI_FORMAT(i.format as i32),
                        i.offset,
                    ),
                    None => c.IASetIndexBuffer(None::<&ID3D11Buffer>, DXGI_FORMAT(0), 0),
                }
            }
            if changed(&|l| l.vs.shader == s.vs.shader) {
                let sh = match s.vs.shader.and_then(|h| self.shader_obj(h)) {
                    Some(ShaderObj::Vertex(x)) => Some(x),
                    _ => None,
                };
                c.VSSetShader(sh, None);
            }
            if changed(&|l| l.ps.shader == s.ps.shader) {
                let sh = match s.ps.shader.and_then(|h| self.shader_obj(h)) {
                    Some(ShaderObj::Pixel(x)) => Some(x),
                    _ => None,
                };
                c.PSSetShader(sh, None);
            }
            if changed(&|l| l.gs == s.gs) {
                let sh = match s.gs.and_then(|h| self.shader_obj(h)) {
                    Some(ShaderObj::Geometry(x)) => Some(x),
                    _ => None,
                };
                c.GSSetShader(sh, None);
            }
            for (stage, st, prev) in [
                (Stage::Vertex, &s.vs, last.as_ref().map(|l| &l.vs)),
                (Stage::Pixel, &s.ps, last.as_ref().map(|l| &l.ps)),
            ] {
                self.apply_stage(stage, st, prev);
            }
            if changed(&|l| l.raster == s.raster) {
                c.RSSetState(s.raster.and_then(|r| self.raster_obj(r)));
            }
            if changed(&|l| l.viewport.map(f32::to_bits) == s.viewport.map(f32::to_bits)) {
                let v = s.viewport;
                c.RSSetViewports(Some(&[D3D11_VIEWPORT {
                    TopLeftX: v[0],
                    TopLeftY: v[1],
                    Width: v[2],
                    Height: v[3],
                    MinDepth: v[4],
                    MaxDepth: v[5],
                }]));
            }
            if changed(&|l| l.scissor == s.scissor) {
                let r = s.scissor;
                c.RSSetScissorRects(Some(&[RECT {
                    left: r[0],
                    top: r[1],
                    right: r[2],
                    bottom: r[3],
                }]));
            }
            if changed(&|l| l.targets == s.targets && l.depth_target == s.depth_target) {
                let mut rtvs: [Option<ID3D11RenderTargetView>; RT_SLOTS] = Default::default();
                let mut n = 0;
                for (i, v) in s.targets.iter().enumerate() {
                    rtvs[i] = self.rtv(*v);
                    if v.is_some() {
                        n = i + 1;
                    }
                }
                let dsv = self.dsv(s.depth_target);
                c.OMSetRenderTargets(Some(&rtvs[..n]), dsv.as_ref());
            }
            if changed(&|l| {
                l.blend == s.blend
                    && l.blend_factor.map(f32::to_bits) == s.blend_factor.map(f32::to_bits)
                    && l.sample_mask == s.sample_mask
            }) {
                c.OMSetBlendState(
                    s.blend.and_then(|b| self.blend_obj(b)),
                    Some(&s.blend_factor),
                    s.sample_mask,
                );
            }
            if changed(&|l| l.depth == s.depth && l.stencil_ref == s.stencil_ref) {
                c.OMSetDepthStencilState(s.depth.and_then(|d| self.depth_obj(d)), s.stencil_ref);
            }
        }
        self.applied = Some(*s);
    }

    fn apply_stage(&self, stage: Stage, s: &StageBindings, last: Option<&StageBindings>) {
        let c = &self.context;
        // SAFETY: live objects of this device, on the context's thread.
        unsafe {
            if last.is_none_or(|l| l.cbs != s.cbs) {
                let mut bufs: [Option<ID3D11Buffer>; CB_SLOTS] = Default::default();
                let mut first = [0u32; CB_SLOTS];
                let mut count = [0u32; CB_SLOTS];
                let mut ranged = false;
                for (i, b) in s.cbs.iter().enumerate() {
                    if let Some(b) = b {
                        bufs[i] = self.buffer(Some(b.buffer)).cloned();
                        first[i] = b.first;
                        count[i] = b.count;
                        ranged |= b.count != 0;
                    }
                }
                match (stage, ranged, &self.context1) {
                    (Stage::Vertex, true, Some(c1)) => c1.VSSetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(bufs.as_ptr()),
                        Some(first.as_ptr()),
                        Some(count.as_ptr()),
                    ),
                    (Stage::Pixel, true, Some(c1)) => c1.PSSetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(bufs.as_ptr()),
                        Some(first.as_ptr()),
                        Some(count.as_ptr()),
                    ),
                    (Stage::Vertex, ..) => c.VSSetConstantBuffers(0, Some(&bufs)),
                    (Stage::Pixel, ..) => c.PSSetConstantBuffers(0, Some(&bufs)),
                    (Stage::Geometry, ..) => {}
                }
            }
            if last.is_none_or(|l| l.srvs != s.srvs) {
                let mut views: [Option<ID3D11ShaderResourceView>; SRV_SLOTS] = Default::default();
                for (i, v) in s.srvs.iter().enumerate() {
                    views[i] = self.srv(*v);
                }
                match stage {
                    Stage::Vertex => c.VSSetShaderResources(0, Some(&views)),
                    Stage::Pixel => c.PSSetShaderResources(0, Some(&views)),
                    Stage::Geometry => {}
                }
            }
            if last.is_none_or(|l| l.samplers != s.samplers) {
                let mut sm: [Option<ID3D11SamplerState>; SAMPLER_SLOTS] = Default::default();
                for (i, v) in s.samplers.iter().enumerate() {
                    sm[i] = v.and_then(|v| self.sampler_obj(v)).cloned();
                }
                match stage {
                    Stage::Vertex => c.VSSetSamplers(0, Some(&sm)),
                    Stage::Pixel => c.PSSetSamplers(0, Some(&sm)),
                    Stage::Geometry => {}
                }
            }
        }
    }

    /// What the context has bound now, every object imported
    /// ([`D3d11::import`]): the state for drawing the host's draw again
    /// later. Constant buffers are left out of the state; they come back
    /// raw in [`BoundNow::cbs`] with their windows. Read with the `Get*`
    /// calls, so only live objects are taken.
    #[allow(
        clippy::too_many_lines,
        clippy::needless_range_loop,
        clippy::many_single_char_names
    )] // one block per piece of state; parallel arrays; as in `bound`
    pub fn import_bound(&mut self) -> BoundNow {
        fn ptr<T: Interface>(o: Option<&T>) -> *mut c_void {
            o.map_or(std::ptr::null_mut(), Interface::as_raw)
        }
        let c = self.context.clone();
        let c1 = self.context1.clone();
        let mut s = DrawState::default();
        let mut reads = Vec::new();
        let mut cbs = [[None; CB_SLOTS]; 2];
        // SAFETY: Get* calls with arrays of the documented sizes; every
        // object they return is alive while its reference is held here,
        // which is when it is imported.
        unsafe {
            s.topology = c.IAGetPrimitiveTopology().0 as u32;
            let l = c.IAGetInputLayout().ok();
            s.layout = self.import(ptr(l.as_ref()), Import::Layout).map(Layout);
            let mut vb: [Option<ID3D11Buffer>; VB_SLOTS] = Default::default();
            let mut strides = [0u32; VB_SLOTS];
            let mut offsets = [0u32; VB_SLOTS];
            c.IAGetVertexBuffers(
                0,
                VB_SLOTS as u32,
                Some(vb.as_mut_ptr()),
                Some(strides.as_mut_ptr()),
                Some(offsets.as_mut_ptr()),
            );
            for i in 0..VB_SLOTS {
                let p = ptr(vb[i].as_ref());
                if !p.is_null() {
                    reads.push(p as usize);
                }
                s.vertex[i] = self.import(p, Import::Buffer).map(|h| VertexBinding {
                    buffer: Buffer(h),
                    stride: strides[i],
                    offset: offsets[i],
                });
            }
            let mut ib = None;
            let mut fmt = DXGI_FORMAT(0);
            let mut off = 0;
            c.IAGetIndexBuffer(Some(&raw mut ib), Some(&raw mut fmt), Some(&raw mut off));
            let p = ptr(ib.as_ref());
            if !p.is_null() {
                reads.push(p as usize);
            }
            s.index = self.import(p, Import::Buffer).map(|h| IndexBinding {
                buffer: Buffer(h),
                format: fmt.0 as u32,
                offset: off,
            });
            let mut vs = None;
            c.VSGetShader(&raw mut vs, None, None);
            s.vs.shader = self
                .import(ptr(vs.as_ref()), Import::Shader(Stage::Vertex))
                .map(Shader);
            let mut ps = None;
            c.PSGetShader(&raw mut ps, None, None);
            s.ps.shader = self
                .import(ptr(ps.as_ref()), Import::Shader(Stage::Pixel))
                .map(Shader);
            let mut gs = None;
            c.GSGetShader(&raw mut gs, None, None);
            s.gs = self
                .import(ptr(gs.as_ref()), Import::Shader(Stage::Geometry))
                .map(Shader);
            for (k, stage) in [Stage::Vertex, Stage::Pixel].into_iter().enumerate() {
                let mut cb: [Option<ID3D11Buffer>; CB_SLOTS] = Default::default();
                let mut first = [0u32; CB_SLOTS];
                let mut count = [0u32; CB_SLOTS];
                match (stage, &c1) {
                    (Stage::Vertex, Some(c1)) => c1.VSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cb.as_mut_ptr()),
                        Some(first.as_mut_ptr()),
                        Some(count.as_mut_ptr()),
                    ),
                    (Stage::Pixel, Some(c1)) => c1.PSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cb.as_mut_ptr()),
                        Some(first.as_mut_ptr()),
                        Some(count.as_mut_ptr()),
                    ),
                    (Stage::Vertex, None) => c.VSGetConstantBuffers(0, Some(&mut cb)),
                    _ => c.PSGetConstantBuffers(0, Some(&mut cb)),
                }
                for i in 0..CB_SLOTS {
                    if let Some(b) = &cb[i] {
                        let mut d = D3D11_BUFFER_DESC::default();
                        b.GetDesc(&raw mut d);
                        // The `…1` getters give a whole buffer as its size
                        // in constants.
                        let whole = first[i] == 0 && count[i] * 16 == d.ByteWidth;
                        cbs[k][i] = Some(BoundCb {
                            buffer: raw(b),
                            first: if whole { 0 } else { first[i] },
                            count: if whole { 0 } else { count[i] },
                        });
                    }
                }
                let mut srvs: [Option<ID3D11ShaderResourceView>; SRV_SLOTS] = Default::default();
                let mut sm: [Option<ID3D11SamplerState>; SAMPLER_SLOTS] = Default::default();
                if stage == Stage::Vertex {
                    c.VSGetShaderResources(0, Some(&mut srvs));
                    c.VSGetSamplers(0, Some(&mut sm));
                } else {
                    c.PSGetShaderResources(0, Some(&mut srvs));
                    c.PSGetSamplers(0, Some(&mut sm));
                }
                let mut st = StageBindings::default();
                for i in 0..SRV_SLOTS {
                    if let Some(v) = &srvs[i]
                        && let Ok(r) = v.GetResource()
                    {
                        reads.push(raw(&r));
                    }
                    st.srvs[i] = self
                        .import(
                            ptr(srvs[i].as_ref()),
                            Import::View(ViewKind::ShaderResource),
                        )
                        .map(View);
                }
                for i in 0..SAMPLER_SLOTS {
                    st.samplers[i] = self
                        .import(ptr(sm[i].as_ref()), Import::Sampler)
                        .map(Sampler);
                }
                let at = if stage == Stage::Vertex {
                    &mut s.vs
                } else {
                    &mut s.ps
                };
                at.srvs = st.srvs;
                at.samplers = st.samplers;
            }
            let r = c.RSGetState().ok();
            s.raster = self.import(ptr(r.as_ref()), Import::Raster).map(Raster);
            let mut n = 1u32;
            let mut vp = [D3D11_VIEWPORT::default()];
            c.RSGetViewports(&raw mut n, Some(vp.as_mut_ptr()));
            let v = vp[0];
            s.viewport = [
                v.TopLeftX, v.TopLeftY, v.Width, v.Height, v.MinDepth, v.MaxDepth,
            ];
            let mut n = 1u32;
            let mut sc = [RECT::default()];
            c.RSGetScissorRects(&raw mut n, Some(sc.as_mut_ptr()));
            s.scissor = [sc[0].left, sc[0].top, sc[0].right, sc[0].bottom];
            let mut rtvs: [Option<ID3D11RenderTargetView>; RT_SLOTS] = Default::default();
            let mut dsv = None;
            c.OMGetRenderTargets(Some(&mut rtvs), Some(&raw mut dsv));
            for i in 0..RT_SLOTS {
                s.targets[i] = self
                    .import(ptr(rtvs[i].as_ref()), Import::View(ViewKind::RenderTarget))
                    .map(View);
            }
            s.depth_target = self
                .import(ptr(dsv.as_ref()), Import::View(ViewKind::DepthStencil))
                .map(View);
            let mut blend = None;
            c.OMGetBlendState(
                Some(&raw mut blend),
                Some(&mut s.blend_factor),
                Some(&raw mut s.sample_mask),
            );
            s.blend = self.import(ptr(blend.as_ref()), Import::Blend).map(Blend);
            let mut depth = None;
            c.OMGetDepthStencilState(Some(&raw mut depth), Some(&raw mut s.stencil_ref));
            s.depth = self
                .import(ptr(depth.as_ref()), Import::DepthState)
                .map(DepthState);
        }
        BoundNow {
            state: s,
            cbs,
            reads,
        }
    }

    /// What the context has bound now, in this backend's handles (objects
    /// it does not know come out as `None`). With [`D3d11::apply`] in
    /// between, this is what the API was really given: the replayed
    /// stream is built from it.
    #[must_use]
    #[allow(clippy::too_many_lines, clippy::needless_range_loop)] // one block per piece of state; parallel arrays
    pub fn bound(&self) -> DrawState {
        let c = &self.context;
        let mut s = DrawState::default();
        // SAFETY: Get* calls with arrays of the documented sizes; the
        // returned references are released when the options drop.
        unsafe {
            s.topology = c.IAGetPrimitiveTopology().0 as u32;
            s.layout = c
                .IAGetInputLayout()
                .ok()
                .and_then(|l| self.handle(Some(&l)))
                .map(Layout);
            let mut vb: [Option<ID3D11Buffer>; VB_SLOTS] = Default::default();
            let mut strides = [0u32; VB_SLOTS];
            let mut offsets = [0u32; VB_SLOTS];
            c.IAGetVertexBuffers(
                0,
                VB_SLOTS as u32,
                Some(vb.as_mut_ptr()),
                Some(strides.as_mut_ptr()),
                Some(offsets.as_mut_ptr()),
            );
            for i in 0..VB_SLOTS {
                s.vertex[i] = self.handle(vb[i].as_ref()).map(|h| VertexBinding {
                    buffer: Buffer(h),
                    stride: strides[i],
                    offset: offsets[i],
                });
            }
            let mut ib = None;
            let mut fmt = DXGI_FORMAT(0);
            let mut off = 0;
            c.IAGetIndexBuffer(Some(&raw mut ib), Some(&raw mut fmt), Some(&raw mut off));
            s.index = self.handle(ib.as_ref()).map(|h| IndexBinding {
                buffer: Buffer(h),
                format: fmt.0 as u32,
                offset: off,
            });
            let mut vs = None;
            c.VSGetShader(&raw mut vs, None, None);
            s.vs.shader = self.handle(vs.as_ref()).map(Shader);
            let mut ps = None;
            c.PSGetShader(&raw mut ps, None, None);
            s.ps.shader = self.handle(ps.as_ref()).map(Shader);
            let mut gs = None;
            c.GSGetShader(&raw mut gs, None, None);
            s.gs = self.handle(gs.as_ref()).map(Shader);
            for (stage, st) in [(Stage::Vertex, &mut s.vs), (Stage::Pixel, &mut s.ps)] {
                let mut cbs: [Option<ID3D11Buffer>; CB_SLOTS] = Default::default();
                let mut first = [0u32; CB_SLOTS];
                let mut count = [0u32; CB_SLOTS];
                match (stage, &self.context1) {
                    (Stage::Vertex, Some(c1)) => c1.VSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.as_mut_ptr()),
                        Some(first.as_mut_ptr()),
                        Some(count.as_mut_ptr()),
                    ),
                    (Stage::Pixel, Some(c1)) => c1.PSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.as_mut_ptr()),
                        Some(first.as_mut_ptr()),
                        Some(count.as_mut_ptr()),
                    ),
                    (Stage::Vertex, None) => c.VSGetConstantBuffers(0, Some(&mut cbs)),
                    _ => c.PSGetConstantBuffers(0, Some(&mut cbs)),
                }
                for i in 0..CB_SLOTS {
                    st.cbs[i] = self.handle(cbs[i].as_ref()).map(|h| {
                        let size = self.buffer_obj(Buffer(h)).map_or(0, |b| b.desc.size);
                        // The `…1` getters report a whole buffer as 0 for
                        // its size in constants.
                        let whole = first[i] == 0 && count[i] * 16 == size;
                        CbBinding {
                            buffer: Buffer(h),
                            first: if whole { 0 } else { first[i] },
                            count: if whole { 0 } else { count[i] },
                        }
                    });
                }
                let mut srvs: [Option<ID3D11ShaderResourceView>; SRV_SLOTS] = Default::default();
                let mut sm: [Option<ID3D11SamplerState>; SAMPLER_SLOTS] = Default::default();
                if stage == Stage::Vertex {
                    c.VSGetShaderResources(0, Some(&mut srvs));
                    c.VSGetSamplers(0, Some(&mut sm));
                } else {
                    c.PSGetShaderResources(0, Some(&mut srvs));
                    c.PSGetSamplers(0, Some(&mut sm));
                }
                for i in 0..SRV_SLOTS {
                    st.srvs[i] = self.handle(srvs[i].as_ref()).map(View);
                }
                for i in 0..SAMPLER_SLOTS {
                    st.samplers[i] = self.handle(sm[i].as_ref()).map(Sampler);
                }
            }
            s.raster = c
                .RSGetState()
                .ok()
                .and_then(|r| self.handle(Some(&r)))
                .map(Raster);
            let mut n = 1u32;
            let mut vp = [D3D11_VIEWPORT::default()];
            c.RSGetViewports(&raw mut n, Some(vp.as_mut_ptr()));
            let v = vp[0];
            s.viewport = [
                v.TopLeftX, v.TopLeftY, v.Width, v.Height, v.MinDepth, v.MaxDepth,
            ];
            let mut n = 1u32;
            let mut sc = [RECT::default()];
            c.RSGetScissorRects(&raw mut n, Some(sc.as_mut_ptr()));
            s.scissor = [sc[0].left, sc[0].top, sc[0].right, sc[0].bottom];
            let mut rtvs: [Option<ID3D11RenderTargetView>; RT_SLOTS] = Default::default();
            let mut dsv = None;
            c.OMGetRenderTargets(Some(&mut rtvs), Some(&raw mut dsv));
            for i in 0..RT_SLOTS {
                s.targets[i] = self.handle(rtvs[i].as_ref()).map(View);
            }
            s.depth_target = self.handle(dsv.as_ref()).map(View);
            let mut blend = None;
            c.OMGetBlendState(
                Some(&raw mut blend),
                Some(&mut s.blend_factor),
                Some(&raw mut s.sample_mask),
            );
            s.blend = self.handle(blend.as_ref()).map(Blend);
            let mut depth = None;
            c.OMGetDepthStencilState(Some(&raw mut depth), Some(&raw mut s.stencil_ref));
            s.depth = self.handle(depth.as_ref()).map(DepthState);
        }
        s
    }

    fn save(&self) -> Saved {
        let c = &self.context;
        let mut s = Saved::default();
        // SAFETY: Get* calls with arrays of the documented sizes.
        unsafe {
            s.layout = c.IAGetInputLayout().ok();
            s.topology = c.IAGetPrimitiveTopology();
            c.IAGetVertexBuffers(
                0,
                VB_SLOTS as u32,
                Some(s.vb.as_mut_ptr()),
                Some(s.vb_strides.as_mut_ptr()),
                Some(s.vb_offsets.as_mut_ptr()),
            );
            c.IAGetIndexBuffer(
                Some(&raw mut s.ib),
                Some(&raw mut s.ib_format),
                Some(&raw mut s.ib_offset),
            );
            c.VSGetShader(&raw mut s.vs, None, None);
            c.PSGetShader(&raw mut s.ps, None, None);
            c.GSGetShader(&raw mut s.gs, None, None);
            for (stage, cbs) in [(Stage::Vertex, &mut s.vs_cb), (Stage::Pixel, &mut s.ps_cb)] {
                match (stage, &self.context1) {
                    (Stage::Vertex, Some(c1)) => c1.VSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.buffers.as_mut_ptr()),
                        Some(cbs.first.as_mut_ptr()),
                        Some(cbs.count.as_mut_ptr()),
                    ),
                    (Stage::Pixel, Some(c1)) => c1.PSGetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.buffers.as_mut_ptr()),
                        Some(cbs.first.as_mut_ptr()),
                        Some(cbs.count.as_mut_ptr()),
                    ),
                    (Stage::Vertex, None) => c.VSGetConstantBuffers(0, Some(&mut cbs.buffers)),
                    _ => c.PSGetConstantBuffers(0, Some(&mut cbs.buffers)),
                }
            }
            c.VSGetShaderResources(0, Some(&mut s.vs_srv));
            c.PSGetShaderResources(0, Some(&mut s.ps_srv));
            c.VSGetSamplers(0, Some(&mut s.vs_samplers));
            c.PSGetSamplers(0, Some(&mut s.ps_samplers));
            s.raster = c.RSGetState().ok();
            let mut n = 16u32;
            s.viewports = vec![D3D11_VIEWPORT::default(); 16];
            c.RSGetViewports(&raw mut n, Some(s.viewports.as_mut_ptr()));
            s.viewports.truncate(n as usize);
            let mut n = 16u32;
            s.scissors = vec![RECT::default(); 16];
            c.RSGetScissorRects(&raw mut n, Some(s.scissors.as_mut_ptr()));
            s.scissors.truncate(n as usize);
            c.OMGetRenderTargetsAndUnorderedAccessViews(
                Some(&mut s.rtv),
                Some(&raw mut s.dsv),
                0,
                None,
            );
            // UAV slots follow the render targets in use.
            let used = s.rtv.iter().rposition(Option::is_some).map_or(0, |i| i + 1);
            if used < RT_SLOTS {
                c.OMGetRenderTargetsAndUnorderedAccessViews(
                    None,
                    None,
                    used as u32,
                    Some(&mut s.uav[used..]),
                );
            }
            c.OMGetBlendState(
                Some(&raw mut s.blend),
                Some(&mut s.blend_factor),
                Some(&raw mut s.sample_mask),
            );
            c.OMGetDepthStencilState(Some(&raw mut s.depth), Some(&raw mut s.stencil_ref));
        }
        s
    }

    fn restore(&self, s: &Saved) {
        let c = &self.context;
        // SAFETY: the context's own objects as it had them bound.
        unsafe {
            c.IASetInputLayout(s.layout.as_ref());
            c.IASetPrimitiveTopology(s.topology);
            c.IASetVertexBuffers(
                0,
                VB_SLOTS as u32,
                Some(s.vb.as_ptr()),
                Some(s.vb_strides.as_ptr()),
                Some(s.vb_offsets.as_ptr()),
            );
            c.IASetIndexBuffer(s.ib.as_ref(), s.ib_format, s.ib_offset);
            c.VSSetShader(s.vs.as_ref(), None);
            c.PSSetShader(s.ps.as_ref(), None);
            c.GSSetShader(s.gs.as_ref(), None);
            // Targets before shader resources: binding a target unbinds
            // any shader resource view of the same texture.
            let used = s.rtv.iter().rposition(Option::is_some).map_or(0, |i| i + 1);
            if s.uav.iter().any(Option::is_some) {
                c.OMSetRenderTargetsAndUnorderedAccessViews(
                    Some(&s.rtv[..used]),
                    s.dsv.as_ref(),
                    used as u32,
                    (RT_SLOTS - used) as u32,
                    Some(s.uav[used..].as_ptr()),
                    None,
                );
            } else {
                c.OMSetRenderTargets(Some(&s.rtv[..used]), s.dsv.as_ref());
            }
            for (stage, cbs) in [(Stage::Vertex, &s.vs_cb), (Stage::Pixel, &s.ps_cb)] {
                match (stage, &self.context1) {
                    (Stage::Vertex, Some(c1)) => c1.VSSetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.buffers.as_ptr()),
                        Some(cbs.first.as_ptr()),
                        Some(cbs.count.as_ptr()),
                    ),
                    (Stage::Pixel, Some(c1)) => c1.PSSetConstantBuffers1(
                        0,
                        CB_SLOTS as u32,
                        Some(cbs.buffers.as_ptr()),
                        Some(cbs.first.as_ptr()),
                        Some(cbs.count.as_ptr()),
                    ),
                    (Stage::Vertex, None) => c.VSSetConstantBuffers(0, Some(&cbs.buffers)),
                    _ => c.PSSetConstantBuffers(0, Some(&cbs.buffers)),
                }
            }
            c.VSSetShaderResources(0, Some(&s.vs_srv));
            c.PSSetShaderResources(0, Some(&s.ps_srv));
            c.VSSetSamplers(0, Some(&s.vs_samplers));
            c.PSSetSamplers(0, Some(&s.ps_samplers));
            c.RSSetState(s.raster.as_ref());
            c.RSSetViewports(Some(&s.viewports));
            c.RSSetScissorRects(Some(&s.scissors));
            c.OMSetBlendState(s.blend.as_ref(), Some(&s.blend_factor), s.sample_mask);
            c.OMSetDepthStencilState(s.depth.as_ref(), s.stencil_ref);
        }
    }
}

/// Adds `obj` to one of `store`'s lists (`list`) and to its known
/// objects; its handle number (`bit` marks imports).
fn push<T>(
    store: &mut Store,
    bit: u32,
    raw: usize,
    list: fn(&mut Store) -> &mut Vec<T>,
    obj: T,
) -> u32 {
    let v = list(store);
    let h = v.len() as u32 | bit;
    v.push(obj);
    store.known.insert(raw, h);
    h
}

fn texture_desc_of(d: &D3D11_TEXTURE2D_DESC) -> TextureDesc {
    TextureDesc {
        width: d.Width,
        height: d.Height,
        mips: d.MipLevels,
        array_size: d.ArraySize,
        format: d.Format.0 as u32,
        samples: d.SampleDesc.Count,
        quality: d.SampleDesc.Quality,
        usage: d.Usage.0 as u32,
        bind: d.BindFlags,
        cpu_access: d.CPUAccessFlags,
        misc: d.MiscFlags,
    }
}

fn texture_desc(d: &TextureDesc) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: d.width,
        Height: d.height,
        MipLevels: d.mips,
        ArraySize: d.array_size,
        Format: DXGI_FORMAT(d.format as i32),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: d.samples,
            Quality: d.quality,
        },
        Usage: D3D11_USAGE(d.usage as i32),
        BindFlags: d.bind,
        CPUAccessFlags: d.cpu_access,
        MiscFlags: d.misc,
    }
}

/// Whether a DXGI format is block-compressed (BC1–BC7).
fn block_compressed(format: u32) -> bool {
    matches!(format, 70..=84 | 94..=99)
}

impl Backend for D3d11 {
    fn begin(&mut self) {
        self.saved = Some(Box::new(self.save()));
        self.applied = None;
    }

    fn end(&mut self) {
        if let Some(s) = self.saved.take() {
            self.restore(&s);
        }
        self.applied = None;
    }

    fn create_shader(&mut self, stage: Stage, dxbc: &[u8]) -> Result<Shader> {
        let d = &self.device;
        // SAFETY: the device's documented calls; the bytecode is copied.
        let obj = unsafe {
            match stage {
                Stage::Vertex => {
                    let mut s = None;
                    d.CreateVertexShader(dxbc, None, Some(&raw mut s))
                        .map_err(|e| err("CreateVertexShader", &e))?;
                    ShaderObj::Vertex(s.ok_or(Error("no vertex shader".into()))?)
                }
                Stage::Pixel => {
                    let mut s = None;
                    d.CreatePixelShader(dxbc, None, Some(&raw mut s))
                        .map_err(|e| err("CreatePixelShader", &e))?;
                    ShaderObj::Pixel(s.ok_or(Error("no pixel shader".into()))?)
                }
                Stage::Geometry => {
                    let mut s = None;
                    d.CreateGeometryShader(dxbc, None, Some(&raw mut s))
                        .map_err(|e| err("CreateGeometryShader", &e))?;
                    ShaderObj::Geometry(s.ok_or(Error("no geometry shader".into()))?)
                }
            }
        };
        let p = match &obj {
            ShaderObj::Vertex(x) => raw(x),
            ShaderObj::Pixel(x) => raw(x),
            ShaderObj::Geometry(x) => raw(x),
        };
        Ok(Shader(push(&mut self.own, 0, p, |s| &mut s.shaders, obj)))
    }

    fn create_layout(&mut self, elements: &[InputElement], vs_dxbc: &[u8]) -> Result<Layout> {
        let names: Vec<Vec<u8>> = elements
            .iter()
            .map(|e| {
                let mut n = e.semantic.as_bytes().to_vec();
                n.push(0);
                n
            })
            .collect();
        let descs: Vec<D3D11_INPUT_ELEMENT_DESC> = elements
            .iter()
            .zip(&names)
            .map(|(e, n)| D3D11_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(n.as_ptr()),
                SemanticIndex: e.index,
                Format: DXGI_FORMAT(e.format as i32),
                InputSlot: e.slot,
                AlignedByteOffset: e.offset,
                InputSlotClass: D3D11_INPUT_CLASSIFICATION(i32::from(e.per_instance)),
                InstanceDataStepRate: e.step,
            })
            .collect();
        let mut l = None;
        // SAFETY: `names` outlives the call.
        unsafe {
            self.device
                .CreateInputLayout(&descs, vs_dxbc, Some(&raw mut l))
                .map_err(|e| err("CreateInputLayout", &e))?;
        }
        let l = l.ok_or(Error("no input layout".into()))?;
        let p = raw(&l);
        Ok(Layout(push(&mut self.own, 0, p, |s| &mut s.layouts, l)))
    }

    fn create_blend(&mut self, d: &BlendDesc) -> Result<Blend> {
        let mut desc = D3D11_BLEND_DESC {
            AlphaToCoverageEnable: bool32(d.alpha_to_coverage),
            IndependentBlendEnable: bool32(d.independent),
            ..D3D11_BLEND_DESC::default()
        };
        for (o, t) in desc.RenderTarget.iter_mut().zip(&d.targets) {
            *o = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: bool32(t.enable),
                SrcBlend: D3D11_BLEND(t.src as i32),
                DestBlend: D3D11_BLEND(t.dst as i32),
                BlendOp: D3D11_BLEND_OP(t.op as i32),
                SrcBlendAlpha: D3D11_BLEND(t.src_alpha as i32),
                DestBlendAlpha: D3D11_BLEND(t.dst_alpha as i32),
                BlendOpAlpha: D3D11_BLEND_OP(t.op_alpha as i32),
                RenderTargetWriteMask: t.write_mask,
            };
        }
        let mut s = None;
        // SAFETY: a full description.
        unsafe {
            self.device
                .CreateBlendState(&raw const desc, Some(&raw mut s))
                .map_err(|e| err("CreateBlendState", &e))?;
        }
        let s = s.ok_or(Error("no blend state".into()))?;
        let p = raw(&s);
        Ok(Blend(push(&mut self.own, 0, p, |st| &mut st.blends, s)))
    }

    fn create_depth_state(&mut self, d: &DepthStencilDesc) -> Result<DepthState> {
        let face = |f: &crate::desc::StencilFace| D3D11_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D11_STENCIL_OP(f.fail as i32),
            StencilDepthFailOp: D3D11_STENCIL_OP(f.depth_fail as i32),
            StencilPassOp: D3D11_STENCIL_OP(f.pass as i32),
            StencilFunc: D3D11_COMPARISON_FUNC(f.func as i32),
        };
        let desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: bool32(d.depth_enable),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK(d.depth_write as i32),
            DepthFunc: D3D11_COMPARISON_FUNC(d.depth_func as i32),
            StencilEnable: bool32(d.stencil_enable),
            StencilReadMask: d.stencil_read_mask,
            StencilWriteMask: d.stencil_write_mask,
            FrontFace: face(&d.front),
            BackFace: face(&d.back),
        };
        let mut s = None;
        // SAFETY: a full description.
        unsafe {
            self.device
                .CreateDepthStencilState(&raw const desc, Some(&raw mut s))
                .map_err(|e| err("CreateDepthStencilState", &e))?;
        }
        let s = s.ok_or(Error("no depth-stencil state".into()))?;
        let p = raw(&s);
        Ok(DepthState(push(
            &mut self.own,
            0,
            p,
            |st| &mut st.depths,
            s,
        )))
    }

    fn create_raster(&mut self, d: &RasterDesc) -> Result<Raster> {
        let desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_MODE(d.fill as i32),
            CullMode: D3D11_CULL_MODE(d.cull as i32),
            FrontCounterClockwise: bool32(d.front_ccw),
            DepthBias: d.depth_bias,
            DepthBiasClamp: f32::from_bits(d.depth_bias_clamp),
            SlopeScaledDepthBias: f32::from_bits(d.slope_scaled_depth_bias),
            DepthClipEnable: bool32(d.depth_clip),
            ScissorEnable: bool32(d.scissor),
            MultisampleEnable: bool32(d.multisample),
            AntialiasedLineEnable: bool32(d.antialiased_line),
        };
        let mut s = None;
        // SAFETY: a full description.
        unsafe {
            self.device
                .CreateRasterizerState(&raw const desc, Some(&raw mut s))
                .map_err(|e| err("CreateRasterizerState", &e))?;
        }
        let s = s.ok_or(Error("no rasterizer state".into()))?;
        let p = raw(&s);
        Ok(Raster(push(&mut self.own, 0, p, |st| &mut st.rasters, s)))
    }

    fn create_sampler(&mut self, d: &SamplerDesc) -> Result<Sampler> {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER(d.filter as i32),
            AddressU: D3D11_TEXTURE_ADDRESS_MODE(d.address[0] as i32),
            AddressV: D3D11_TEXTURE_ADDRESS_MODE(d.address[1] as i32),
            AddressW: D3D11_TEXTURE_ADDRESS_MODE(d.address[2] as i32),
            MipLODBias: f32::from_bits(d.mip_lod_bias),
            MaxAnisotropy: d.max_anisotropy,
            ComparisonFunc: D3D11_COMPARISON_FUNC(d.comparison as i32),
            BorderColor: d.border.map(f32::from_bits),
            MinLOD: f32::from_bits(d.min_lod),
            MaxLOD: f32::from_bits(d.max_lod),
        };
        let mut s = None;
        // SAFETY: a full description.
        unsafe {
            self.device
                .CreateSamplerState(&raw const desc, Some(&raw mut s))
                .map_err(|e| err("CreateSamplerState", &e))?;
        }
        let s = s.ok_or(Error("no sampler state".into()))?;
        let p = raw(&s);
        Ok(Sampler(push(&mut self.own, 0, p, |st| &mut st.samplers, s)))
    }

    fn create_buffer(&mut self, desc: &BufferDesc, data: Option<&[u8]>) -> Result<Buffer> {
        let d = D3D11_BUFFER_DESC {
            ByteWidth: desc.size,
            Usage: D3D11_USAGE(desc.usage as i32),
            BindFlags: desc.bind,
            CPUAccessFlags: desc.cpu_access,
            MiscFlags: desc.misc,
            StructureByteStride: desc.stride,
        };
        let init = data.map(|data| D3D11_SUBRESOURCE_DATA {
            pSysMem: data.as_ptr().cast(),
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        });
        let mut b = None;
        // SAFETY: `data` (if any) is the buffer's size and outlives the
        // call.
        unsafe {
            self.device
                .CreateBuffer(
                    &raw const d,
                    init.as_ref().map(std::ptr::from_ref),
                    Some(&raw mut b),
                )
                .map_err(|e| err("CreateBuffer", &e))?;
        }
        let b = b.ok_or(Error("no buffer".into()))?;
        let p = raw(&b);
        let obj = BufferObj {
            buffer: b,
            desc: *desc,
            written: data.map(<[u8]>::to_vec).unwrap_or_default(),
        };
        Ok(Buffer(push(&mut self.own, 0, p, |s| &mut s.buffers, obj)))
    }

    fn create_texture(&mut self, desc: &TextureDesc, init: &[InitData]) -> Result<Texture> {
        let d = texture_desc(desc);
        let subs: Vec<D3D11_SUBRESOURCE_DATA> = init
            .iter()
            .map(|s| D3D11_SUBRESOURCE_DATA {
                pSysMem: s.data.as_ptr().cast(),
                SysMemPitch: s.row_pitch,
                SysMemSlicePitch: 0,
            })
            .collect();
        let mut t = None;
        // SAFETY: one entry per subresource, each outliving the call.
        unsafe {
            self.device
                .CreateTexture2D(
                    &raw const d,
                    (!subs.is_empty()).then_some(subs.as_ptr()),
                    Some(&raw mut t),
                )
                .map_err(|e| err("CreateTexture2D", &e))?;
        }
        let t = t.ok_or(Error("no texture".into()))?;
        Ok(self.add_texture(false, t, *desc))
    }

    fn create_view(
        &mut self,
        texture: Texture,
        kind: ViewKind,
        desc: Option<&ViewDesc>,
    ) -> Result<View> {
        let tex = self
            .texture_obj(texture)
            .map(|t| t.0.clone())
            .ok_or(Error("no such texture".into()))?;
        let words = desc.map(ViewDesc::words);
        // SAFETY: the view descriptions are a format, a dimension and a
        // union of u32 fields: 24 bytes for shader resource and
        // depth-stencil views, 20 for render target views, the layout of
        // `words`.
        let obj = unsafe {
            match kind {
                ViewKind::ShaderResource => {
                    let mut v = None;
                    let d = words.map(|w| {
                        std::ptr::read_unaligned(
                            w.as_ptr().cast::<D3D11_SHADER_RESOURCE_VIEW_DESC>(),
                        )
                    });
                    self.device
                        .CreateShaderResourceView(
                            &tex,
                            d.as_ref().map(std::ptr::from_ref),
                            Some(&raw mut v),
                        )
                        .map_err(|e| err("CreateShaderResourceView", &e))?;
                    ViewObj::Srv(v.ok_or(Error("no view".into()))?)
                }
                ViewKind::RenderTarget => {
                    let mut v = None;
                    let d = words.map(|w| {
                        std::ptr::read_unaligned(w.as_ptr().cast::<D3D11_RENDER_TARGET_VIEW_DESC>())
                    });
                    self.device
                        .CreateRenderTargetView(
                            &tex,
                            d.as_ref().map(std::ptr::from_ref),
                            Some(&raw mut v),
                        )
                        .map_err(|e| err("CreateRenderTargetView", &e))?;
                    ViewObj::Rtv(v.ok_or(Error("no view".into()))?)
                }
                ViewKind::DepthStencil => {
                    let mut v = None;
                    let d = words.map(|w| {
                        std::ptr::read_unaligned(w.as_ptr().cast::<D3D11_DEPTH_STENCIL_VIEW_DESC>())
                    });
                    self.device
                        .CreateDepthStencilView(
                            &tex,
                            d.as_ref().map(std::ptr::from_ref),
                            Some(&raw mut v),
                        )
                        .map_err(|e| err("CreateDepthStencilView", &e))?;
                    ViewObj::Dsv(v.ok_or(Error("no view".into()))?)
                }
            }
        };
        Ok(self.add_view(false, obj, Some(texture)))
    }

    fn write_buffer(&mut self, buffer: Buffer, offset: u32, data: &[u8]) {
        let b = if buffer.0 & IMPORTED == 0 {
            self.own.buffers.get_mut(buffer.0 as usize)
        } else {
            self.imported
                .buffers
                .get_mut((buffer.0 & !IMPORTED) as usize)
        };
        let Some(b) = b else {
            return;
        };
        let size = b.desc.size as usize;
        let from = (offset as usize).min(size);
        let to = (from + data.len()).min(size);
        let data = &data[..to - from];
        if b.written.len() != size {
            b.written.resize(size, 0);
        }
        b.written[from..to].copy_from_slice(data);
        // SAFETY: mapped memory of `size` bytes; written within it.
        unsafe {
            if b.desc.usage == USAGE_DYNAMIC {
                let mut m = D3D11_MAPPED_SUBRESOURCE::default();
                if self
                    .context
                    .Map(&b.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&raw mut m))
                    .is_ok()
                    && !m.pData.is_null()
                {
                    // A discard loses the rest: write the whole buffer.
                    std::ptr::copy_nonoverlapping(b.written.as_ptr(), m.pData.cast::<u8>(), size);
                    self.context.Unmap(&b.buffer, 0);
                }
            } else {
                let region = D3D11_BOX {
                    left: from as u32,
                    top: 0,
                    front: 0,
                    right: to as u32,
                    bottom: 1,
                    back: 1,
                };
                self.context.UpdateSubresource(
                    &b.buffer,
                    0,
                    Some(&raw const region),
                    data.as_ptr().cast(),
                    0,
                    0,
                );
            }
        }
    }

    fn clear_target(&mut self, view: View, color: [f32; 4]) {
        if let Some(v) = self.rtv(Some(view)) {
            // SAFETY: a live view of this device.
            unsafe { self.context.ClearRenderTargetView(&v, &color) };
        }
    }

    fn clear_depth(&mut self, view: View, flags: u32, depth: f32, stencil: u8) {
        if let Some(v) = self.dsv(Some(view)) {
            // SAFETY: a live view of this device.
            unsafe {
                self.context
                    .ClearDepthStencilView(&v, flags, depth, stencil);
            }
        }
    }

    fn copy_texture(&mut self, dst: Texture, src: Texture) {
        if let (Some(d), Some(s)) = (self.texture_obj(dst), self.texture_obj(src)) {
            // SAFETY: live textures of this device.
            unsafe { self.context.CopyResource(&d.0, &s.0) };
        }
    }

    fn draw(&mut self, state: &DrawState, call: DrawCall) {
        self.apply(state);
        let c = &self.context;
        // SAFETY: the state just applied.
        unsafe {
            match call {
                DrawCall::Indexed { count, start, base } => c.DrawIndexed(count, start, base),
                DrawCall::Plain { count, start } => c.Draw(count, start),
                DrawCall::IndexedInstanced {
                    count,
                    instances,
                    start,
                    base,
                    first_instance,
                } => c.DrawIndexedInstanced(count, instances, start, base, first_instance),
                DrawCall::Instanced {
                    count,
                    instances,
                    start,
                    first_instance,
                } => c.DrawInstanced(count, instances, start, first_instance),
            }
        }
    }

    fn view_texture(&self, view: View) -> Option<Texture> {
        self.view_obj(view).and_then(|v| v.1)
    }

    fn read_texture(&mut self, texture: Texture) -> Result<Readback> {
        let (tex, desc) = self
            .texture_obj(texture)
            .cloned()
            .ok_or(Error("no such texture".into()))?;
        if desc.samples > 1 {
            return Err(Error("multisampled".into()));
        }
        let mut sd = texture_desc(&desc);
        sd.MipLevels = 1;
        sd.ArraySize = 1;
        sd.Usage = D3D11_USAGE_STAGING;
        sd.BindFlags = D3D11_BIND_FLAG(0).0 as u32;
        sd.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        sd.MiscFlags = 0;
        let mut staging: Option<ID3D11Texture2D> = None;
        // SAFETY: a staging copy of mip 0, slice 0, mapped for reading.
        unsafe {
            self.device
                .CreateTexture2D(&raw const sd, None, Some(&raw mut staging))
                .map_err(|e| err("CreateTexture2D (staging)", &e))?;
            let staging = staging.ok_or(Error("no staging texture".into()))?;
            let dst: ID3D11Resource = staging.cast().map_err(|e| err("cast", &e))?;
            let src: ID3D11Resource = tex.cast().map_err(|e| err("cast", &e))?;
            self.context
                .CopySubresourceRegion(&dst, 0, 0, 0, 0, &src, 0, None);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(&dst, 0, D3D11_MAP_READ, 0, Some(&raw mut m))
                .map_err(|e| err("Map (read)", &e))?;
            let rows = if block_compressed(desc.format) {
                desc.height.div_ceil(4)
            } else {
                desc.height
            };
            let data =
                std::slice::from_raw_parts(m.pData.cast::<u8>(), (m.RowPitch * rows) as usize)
                    .to_vec();
            self.context.Unmap(&dst, 0);
            Ok(Readback {
                width: desc.width,
                height: desc.height,
                format: desc.format,
                row_pitch: m.RowPitch,
                rows,
                data,
            })
        }
    }
}
