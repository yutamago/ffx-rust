//! The Windows back end of [`crate::gfx`]: the real Direct3D 11 device,
//! every call forwarded with the game's arguments marshalled out of the
//! arena (descriptions copied into the API's structures, contents as
//! byte slices, objects mapped from their game handles to the real
//! interfaces). Nothing here hands a host pointer to the game, so it
//! builds the same for i686 (stage 3) and `x86_64` (stage 4).
//!
//! On Windows the window is real (`crate::win_forward`): the device is
//! made with its swap chain on the game's window
//! (`D3D11CreateDeviceAndSwapChain`, the game's description), the back
//! buffer is the swap chain's buffer 0 and `Present` presents. Without a
//! window (the description names none) the device has no swap chain: the
//! back buffer is an offscreen texture and `Present` flushes.

// The methods' arguments keep their short `d3d11.h` roles (`c` the
// context, `d` the device, `r` a resource, `v` views, `s` states).
#![allow(clippy::many_single_char_names)]

use std::collections::HashMap;
use std::ffi::CString;

use ffx_gfx::StateKind;
use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_PRIMITIVE_TOPOLOGY,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BLEND_DESC, D3D11_BOX, D3D11_BUFFER_DESC, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_STENCIL_VIEW_DESC,
    D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_INSTANCE_DATA, D3D11_INPUT_PER_VERTEX_DATA,
    D3D11_MAP_READ, D3D11_MAP_WRITE_DISCARD, D3D11_MAPPED_SUBRESOURCE, D3D11_QUERY_DESC,
    D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_SAMPLER_DESC, D3D11_SDK_VERSION,
    D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE1D_DESC,
    D3D11_TEXTURE2D_DESC, D3D11_TEXTURE3D_DESC, D3D11_UNORDERED_ACCESS_VIEW_DESC,
    D3D11_USAGE_STAGING, D3D11_VIEWPORT, D3D11CreateDevice, D3D11CreateDeviceAndSwapChain,
    ID3D11Asynchronous, ID3D11BlendState, ID3D11Buffer, ID3D11ComputeShader,
    ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device, ID3D11DeviceContext,
    ID3D11DeviceContext1, ID3D11DomainShader, ID3D11GeometryShader, ID3D11HullShader,
    ID3D11InputLayout, ID3D11PixelShader, ID3D11Query, ID3D11RasterizerState,
    ID3D11RenderTargetView, ID3D11Resource, ID3D11SamplerState, ID3D11ShaderResourceView,
    ID3D11Texture2D, ID3D11UnorderedAccessView, ID3D11VertexShader,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_MODE_DESC, DXGI_MODE_SCALING, DXGI_MODE_SCANLINE_ORDER, DXGI_RATIONAL,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT, DXGI_USAGE, IDXGISwapChain,
};
use windows::core::{IUnknown, Interface, PCSTR};

use crate::com::{E_FAIL, E_INVALIDARG, S_FALSE, S_OK};
use crate::com_methods::class;
use crate::gfx::{Backend, Create, Op, SwapChainDesc};

/// A failed call's `HRESULT` as the game sees it.
fn hr(e: &windows::core::Error) -> u32 {
    u32::from_ne_bytes(e.code().0.to_ne_bytes())
}

/// Reads a D3D11 description from its bytes (`T` is a plain `repr(C)`
/// structure of 32-bit fields: the same layout on i686 and `x86_64`).
fn read<T: Copy>(b: &[u8]) -> Option<T> {
    (b.len() >= std::mem::size_of::<T>())
        // SAFETY: `b` holds at least `size_of::<T>()` bytes and `T` is
        // plain data with no invalid bit patterns.
        .then(|| unsafe { std::ptr::read_unaligned(b.as_ptr().cast::<T>()) })
}

fn words_bytes(w: &[u32]) -> Vec<u8> {
    w.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// One real object and what the back end needs to know of it.
struct Object {
    unknown: IUnknown,
    /// `D3D11_USAGE` of a buffer or texture.
    usage: u32,
}

/// The real device.
#[derive(Default)]
pub struct D3d11 {
    device: Option<ID3D11Device>,
    context: Option<ID3D11DeviceContext>,
    context1: Option<ID3D11DeviceContext1>,
    objects: HashMap<u32, Object>,
    /// The swap chain on the game's window (`None`: offscreen).
    swap_chain: Option<IDXGISwapChain>,
    /// The back buffer's handle in the layer.
    back_buffer: u32,
}

// SAFETY: the device is free-threaded, and the layer calls the back end
// (the immediate context included) under its one lock
// (`crate::gfx::Slot`), so the context is never used by two threads at
// once.
unsafe impl Send for D3d11 {}

impl D3d11 {
    fn get<T: Interface>(&self, id: u32) -> Option<T> {
        if id == 0 {
            return None;
        }
        self.objects
            .get(&id)
            .and_then(|o| o.unknown.cast::<T>().ok())
    }

    fn list<T: Interface>(&self, ids: &[u32]) -> Vec<Option<T>> {
        ids.iter().map(|&i| self.get::<T>(i)).collect()
    }

    fn keep(&mut self, id: u32, unknown: Option<impl Interface>, usage: u32) -> u32 {
        match unknown.and_then(|u| u.cast::<IUnknown>().ok()) {
            Some(unknown) => {
                self.objects.insert(id, Object { unknown, usage });
                S_OK
            }
            None => E_FAIL,
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per object kind
    fn make(&mut self, id: u32, what: &Create<'_>) -> windows::core::Result<u32> {
        let Some(d) = self.device.clone() else {
            return Ok(E_FAIL);
        };
        // SAFETY: every pointer handed to the device points at a
        // description or data owned by this frame that outlives the call.
        unsafe {
            match what {
                Create::Buffer { desc, init } => {
                    let bd: D3D11_BUFFER_DESC = read(&words_bytes(desc)).unwrap_or_default();
                    let sub = init.map(|b| D3D11_SUBRESOURCE_DATA {
                        pSysMem: b.as_ptr().cast(),
                        SysMemPitch: 0,
                        SysMemSlicePitch: 0,
                    });
                    let mut out: Option<ID3D11Buffer> = None;
                    d.CreateBuffer(
                        &raw const bd,
                        sub.as_ref().map(std::ptr::from_ref),
                        Some(&raw mut out),
                    )?;
                    Ok(self.keep(id, out, desc[1]))
                }
                Create::Texture { dims, desc, init } => {
                    let subs: Vec<D3D11_SUBRESOURCE_DATA> = init
                        .iter()
                        .map(|(row, slice, data)| D3D11_SUBRESOURCE_DATA {
                            pSysMem: data.as_ptr().cast(),
                            SysMemPitch: *row,
                            SysMemSlicePitch: *slice,
                        })
                        .collect();
                    let init = (!subs.is_empty()).then_some(subs.as_ptr());
                    let bytes = words_bytes(desc);
                    match dims {
                        1 => {
                            let td: D3D11_TEXTURE1D_DESC = read(&bytes).unwrap_or_default();
                            let mut out = None;
                            d.CreateTexture1D(&raw const td, init, Some(&raw mut out))?;
                            Ok(self.keep(id, out, desc[4]))
                        }
                        2 => {
                            let td: D3D11_TEXTURE2D_DESC = read(&bytes).unwrap_or_default();
                            let mut out = None;
                            d.CreateTexture2D(&raw const td, init, Some(&raw mut out))?;
                            Ok(self.keep(id, out, desc[7]))
                        }
                        _ => {
                            let td: D3D11_TEXTURE3D_DESC = read(&bytes).unwrap_or_default();
                            let mut out = None;
                            d.CreateTexture3D(&raw const td, init, Some(&raw mut out))?;
                            Ok(self.keep(id, out, desc[5]))
                        }
                    }
                }
                Create::View {
                    class: c,
                    resource,
                    desc,
                } => {
                    let Some(r) = self.get::<ID3D11Resource>(*resource) else {
                        return Ok(E_INVALIDARG);
                    };
                    let desc = desc.unwrap_or(&[]);
                    match *c {
                        class::SRV => {
                            let vd = read::<D3D11_SHADER_RESOURCE_VIEW_DESC>(desc);
                            let mut out: Option<ID3D11ShaderResourceView> = None;
                            d.CreateShaderResourceView(
                                &r,
                                vd.as_ref().map(std::ptr::from_ref),
                                Some(&raw mut out),
                            )?;
                            Ok(self.keep(id, out, 0))
                        }
                        class::RTV => {
                            let vd = read::<D3D11_RENDER_TARGET_VIEW_DESC>(desc);
                            let mut out: Option<ID3D11RenderTargetView> = None;
                            d.CreateRenderTargetView(
                                &r,
                                vd.as_ref().map(std::ptr::from_ref),
                                Some(&raw mut out),
                            )?;
                            Ok(self.keep(id, out, 0))
                        }
                        class::DSV => {
                            let vd = read::<D3D11_DEPTH_STENCIL_VIEW_DESC>(desc);
                            let mut out: Option<ID3D11DepthStencilView> = None;
                            d.CreateDepthStencilView(
                                &r,
                                vd.as_ref().map(std::ptr::from_ref),
                                Some(&raw mut out),
                            )?;
                            Ok(self.keep(id, out, 0))
                        }
                        _ => {
                            let vd = read::<D3D11_UNORDERED_ACCESS_VIEW_DESC>(desc);
                            let mut out: Option<ID3D11UnorderedAccessView> = None;
                            d.CreateUnorderedAccessView(
                                &r,
                                vd.as_ref().map(std::ptr::from_ref),
                                Some(&raw mut out),
                            )?;
                            Ok(self.keep(id, out, 0))
                        }
                    }
                }
                Create::Shader { stage, bytecode } => match stage {
                    b'v' => {
                        let mut out: Option<ID3D11VertexShader> = None;
                        d.CreateVertexShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    b'p' => {
                        let mut out: Option<ID3D11PixelShader> = None;
                        d.CreatePixelShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    b'g' => {
                        let mut out: Option<ID3D11GeometryShader> = None;
                        d.CreateGeometryShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    b'h' => {
                        let mut out: Option<ID3D11HullShader> = None;
                        d.CreateHullShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    b'd' => {
                        let mut out: Option<ID3D11DomainShader> = None;
                        d.CreateDomainShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    _ => {
                        let mut out: Option<ID3D11ComputeShader> = None;
                        d.CreateComputeShader(bytecode, None, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                },
                Create::Layout { elements, bytecode } => {
                    let names: Vec<CString> = elements
                        .iter()
                        .map(|e| CString::new(e.semantic.clone()).unwrap_or_default())
                        .collect();
                    let descs: Vec<D3D11_INPUT_ELEMENT_DESC> = elements
                        .iter()
                        .zip(&names)
                        .map(|(e, n)| D3D11_INPUT_ELEMENT_DESC {
                            SemanticName: PCSTR(n.as_ptr().cast()),
                            SemanticIndex: e.index,
                            Format: DXGI_FORMAT(i32::from_ne_bytes(e.format.to_ne_bytes())),
                            InputSlot: e.slot,
                            AlignedByteOffset: e.offset,
                            InputSlotClass: if e.per_instance {
                                D3D11_INPUT_PER_INSTANCE_DATA
                            } else {
                                D3D11_INPUT_PER_VERTEX_DATA
                            },
                            InstanceDataStepRate: e.step,
                        })
                        .collect();
                    let mut out: Option<ID3D11InputLayout> = None;
                    d.CreateInputLayout(&descs, bytecode, Some(&raw mut out))?;
                    Ok(self.keep(id, out, 0))
                }
                Create::State { kind, desc } => match kind {
                    StateKind::Blend => {
                        let sd: D3D11_BLEND_DESC = read(desc).unwrap_or_default();
                        let mut out: Option<ID3D11BlendState> = None;
                        d.CreateBlendState(&raw const sd, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    StateKind::Depth => {
                        let sd: D3D11_DEPTH_STENCIL_DESC = read(desc).unwrap_or_default();
                        let mut out: Option<ID3D11DepthStencilState> = None;
                        d.CreateDepthStencilState(&raw const sd, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    StateKind::Raster => {
                        let sd: D3D11_RASTERIZER_DESC = read(desc).unwrap_or_default();
                        let mut out: Option<ID3D11RasterizerState> = None;
                        d.CreateRasterizerState(&raw const sd, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                    StateKind::Sampler => {
                        let sd: D3D11_SAMPLER_DESC = read(desc).unwrap_or_default();
                        let mut out: Option<ID3D11SamplerState> = None;
                        d.CreateSamplerState(&raw const sd, Some(&raw mut out))?;
                        Ok(self.keep(id, out, 0))
                    }
                },
                Create::Query(q) => {
                    let qd: D3D11_QUERY_DESC = read(&words_bytes(q)).unwrap_or_default();
                    let mut out: Option<ID3D11Query> = None;
                    d.CreateQuery(&raw const qd, Some(&raw mut out))?;
                    Ok(self.keep(id, out, 0))
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per call
    fn run(&self, op: &Op<'_>) {
        let Some(c) = self.context.as_ref() else {
            return;
        };
        let n = |s: &[u32]| u32::try_from(s.len()).unwrap_or(0);
        // SAFETY: every slice and description outlives its call; the
        // objects are the device's own.
        unsafe {
            match op {
                Op::InputLayout(l) => {
                    c.IASetInputLayout(self.get::<ID3D11InputLayout>(*l).as_ref());
                }
                Op::VertexBuffers(start, b, s, o) => {
                    let bufs = self.list::<ID3D11Buffer>(b);
                    c.IASetVertexBuffers(
                        *start,
                        n(b),
                        Some(bufs.as_ptr()),
                        Some(s.as_ptr()),
                        Some(o.as_ptr()),
                    );
                }
                Op::IndexBuffer(b, f, o) => c.IASetIndexBuffer(
                    self.get::<ID3D11Buffer>(*b).as_ref(),
                    DXGI_FORMAT(i32::from_ne_bytes(f.to_ne_bytes())),
                    *o,
                ),
                Op::Topology(t) => {
                    c.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY(i32::from_ne_bytes(
                        t.to_ne_bytes(),
                    )));
                }
                Op::Shader(stage, s) => match stage {
                    b'v' => c.VSSetShader(self.get::<ID3D11VertexShader>(*s).as_ref(), None),
                    b'p' => c.PSSetShader(self.get::<ID3D11PixelShader>(*s).as_ref(), None),
                    b'g' => c.GSSetShader(self.get::<ID3D11GeometryShader>(*s).as_ref(), None),
                    b'h' => c.HSSetShader(self.get::<ID3D11HullShader>(*s).as_ref(), None),
                    b'd' => c.DSSetShader(self.get::<ID3D11DomainShader>(*s).as_ref(), None),
                    _ => c.CSSetShader(self.get::<ID3D11ComputeShader>(*s).as_ref(), None),
                },
                Op::ConstantBuffers(stage, start, b, ranges) => {
                    let bufs = self.list::<ID3D11Buffer>(b);
                    match (ranges, self.context1.as_ref()) {
                        (Some((f, k)), Some(c1)) => {
                            let (bp, fp, kp) =
                                (Some(bufs.as_ptr()), Some(f.as_ptr()), Some(k.as_ptr()));
                            match stage {
                                b'v' => c1.VSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                                b'p' => c1.PSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                                b'g' => c1.GSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                                b'h' => c1.HSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                                b'd' => c1.DSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                                _ => c1.CSSetConstantBuffers1(*start, n(b), bp, fp, kp),
                            }
                        }
                        _ => match stage {
                            b'v' => c.VSSetConstantBuffers(*start, Some(&bufs)),
                            b'p' => c.PSSetConstantBuffers(*start, Some(&bufs)),
                            b'g' => c.GSSetConstantBuffers(*start, Some(&bufs)),
                            b'h' => c.HSSetConstantBuffers(*start, Some(&bufs)),
                            b'd' => c.DSSetConstantBuffers(*start, Some(&bufs)),
                            _ => c.CSSetConstantBuffers(*start, Some(&bufs)),
                        },
                    }
                }
                Op::Resources(stage, start, v) => {
                    let views = self.list::<ID3D11ShaderResourceView>(v);
                    match stage {
                        b'v' => c.VSSetShaderResources(*start, Some(&views)),
                        b'p' => c.PSSetShaderResources(*start, Some(&views)),
                        b'g' => c.GSSetShaderResources(*start, Some(&views)),
                        b'h' => c.HSSetShaderResources(*start, Some(&views)),
                        b'd' => c.DSSetShaderResources(*start, Some(&views)),
                        _ => c.CSSetShaderResources(*start, Some(&views)),
                    }
                }
                Op::Samplers(stage, start, v) => {
                    let s = self.list::<ID3D11SamplerState>(v);
                    match stage {
                        b'v' => c.VSSetSamplers(*start, Some(&s)),
                        b'p' => c.PSSetSamplers(*start, Some(&s)),
                        b'g' => c.GSSetSamplers(*start, Some(&s)),
                        b'h' => c.HSSetSamplers(*start, Some(&s)),
                        b'd' => c.DSSetSamplers(*start, Some(&s)),
                        _ => c.CSSetSamplers(*start, Some(&s)),
                    }
                }
                Op::Targets(v, dsv) => {
                    let rtv = self.list::<ID3D11RenderTargetView>(v);
                    c.OMSetRenderTargets(
                        Some(&rtv),
                        self.get::<ID3D11DepthStencilView>(*dsv).as_ref(),
                    );
                }
                Op::Blend(s, factor, mask) => {
                    c.OMSetBlendState(
                        self.get::<ID3D11BlendState>(*s).as_ref(),
                        Some(factor),
                        *mask,
                    );
                }
                Op::Depth(s, r) => {
                    c.OMSetDepthStencilState(self.get::<ID3D11DepthStencilState>(*s).as_ref(), *r);
                }
                Op::Raster(s) => c.RSSetState(self.get::<ID3D11RasterizerState>(*s).as_ref()),
                Op::Viewports(v) => {
                    let vp: Vec<D3D11_VIEWPORT> = v
                        .iter()
                        .map(|x| D3D11_VIEWPORT {
                            TopLeftX: x[0],
                            TopLeftY: x[1],
                            Width: x[2],
                            Height: x[3],
                            MinDepth: x[4],
                            MaxDepth: x[5],
                        })
                        .collect();
                    c.RSSetViewports(Some(&vp));
                }
                Op::Scissors(v) => {
                    let r: Vec<RECT> = v
                        .iter()
                        .map(|x| RECT {
                            left: x[0],
                            top: x[1],
                            right: x[2],
                            bottom: x[3],
                        })
                        .collect();
                    c.RSSetScissorRects(Some(&r));
                }
                Op::Draw(kind, p) => {
                    use ffx_gfx::DrawKind as K;
                    let base = i32::from_ne_bytes(p[2].to_ne_bytes());
                    match kind {
                        K::Plain => c.Draw(p[0], p[1]),
                        K::Indexed => c.DrawIndexed(p[0], p[1], base),
                        K::Instanced => c.DrawInstanced(p[0], p[1], p[2], p[3]),
                        K::IndexedInstanced => {
                            c.DrawIndexedInstanced(
                                p[0],
                                p[1],
                                p[2],
                                i32::from_ne_bytes(p[3].to_ne_bytes()),
                                p[4],
                            );
                        }
                    }
                }
                Op::ClearTarget(v, color) => {
                    if let Some(v) = self.get::<ID3D11RenderTargetView>(*v) {
                        c.ClearRenderTargetView(&v, color);
                    }
                }
                Op::ClearDepth(v, flags, depth, stencil) => {
                    if let Some(v) = self.get::<ID3D11DepthStencilView>(*v) {
                        c.ClearDepthStencilView(&v, *flags, *depth, *stencil);
                    }
                }
                Op::Copy(dst, src) => {
                    if let (Some(d), Some(s)) = (
                        self.get::<ID3D11Resource>(*dst),
                        self.get::<ID3D11Resource>(*src),
                    ) {
                        c.CopyResource(&d, &s);
                    }
                }
                Op::Resolve(w) => {
                    if let (Some(d), Some(s)) = (
                        self.get::<ID3D11Resource>(w[0]),
                        self.get::<ID3D11Resource>(w[2]),
                    ) {
                        c.ResolveSubresource(
                            &d,
                            w[1],
                            &s,
                            w[3],
                            DXGI_FORMAT(i32::from_ne_bytes(w[4].to_ne_bytes())),
                        );
                    }
                }
                Op::CopyRegion(w, bx) => {
                    if let (Some(d), Some(s)) = (
                        self.get::<ID3D11Resource>(w[0]),
                        self.get::<ID3D11Resource>(w[5]),
                    ) {
                        let b = bx.map(|b| D3D11_BOX {
                            left: b[0],
                            top: b[1],
                            front: b[2],
                            right: b[3],
                            bottom: b[4],
                            back: b[5],
                        });
                        c.CopySubresourceRegion(
                            &d,
                            w[1],
                            w[2],
                            w[3],
                            w[4],
                            &s,
                            w[6],
                            b.as_ref().map(std::ptr::from_ref),
                        );
                    }
                }
                Op::Update(dst, sub, bx, data, row, depth) => {
                    if let Some(d) = self.get::<ID3D11Resource>(*dst) {
                        let b = bx.map(|b| D3D11_BOX {
                            left: b[0],
                            top: b[1],
                            front: b[2],
                            right: b[3],
                            bottom: b[4],
                            back: b[5],
                        });
                        c.UpdateSubresource(
                            &d,
                            *sub,
                            b.as_ref().map(std::ptr::from_ref),
                            data.as_ptr().cast(),
                            *row,
                            *depth,
                        );
                    }
                }
                Op::Unmapped(id, sub, row, depth, data) => {
                    self.write(c, *id, *sub, *row, *depth, data);
                }
                Op::Begin(q) => {
                    if let Some(q) = self.get::<ID3D11Asynchronous>(*q) {
                        c.Begin(&q);
                    }
                }
                Op::End(q) => {
                    if let Some(q) = self.get::<ID3D11Asynchronous>(*q) {
                        c.End(&q);
                    }
                }
                Op::GenerateMips(v) => {
                    if let Some(v) = self.get::<ID3D11ShaderResourceView>(*v) {
                        c.GenerateMips(&v);
                    }
                }
                Op::ClearState => c.ClearState(),
                Op::Flush => c.Flush(),
            }
        }
    }

    /// The CPU's writes to a mapped subresource reach the resource: a
    /// discarding map of a dynamic resource, else `UpdateSubresource`.
    fn write(&self, c: &ID3D11DeviceContext, id: u32, sub: u32, row: u32, depth: u32, data: &[u8]) {
        let Some(o) = self.objects.get(&id) else {
            return;
        };
        let Ok(r) = o.unknown.cast::<ID3D11Resource>() else {
            return;
        };
        // SAFETY: the mapped pointer is valid for the subresource until
        // `Unmap`; rows are copied at the smaller of the two pitches.
        unsafe {
            if o.usage == 2 {
                let mut m = D3D11_MAPPED_SUBRESOURCE::default();
                if c.Map(&r, sub, D3D11_MAP_WRITE_DISCARD, 0, Some(&raw mut m))
                    .is_ok()
                    && !m.pData.is_null()
                {
                    copy_pitched(data, row, m.pData.cast::<u8>(), m.RowPitch);
                    c.Unmap(&r, sub);
                }
            } else {
                c.UpdateSubresource(&r, sub, None, data.as_ptr().cast(), row, depth);
            }
        }
    }
}

/// Copies `src` (rows of `src_pitch`) to `dst` (rows of `dst_pitch`).
///
/// # Safety
/// `dst` must hold `src.len() / src_pitch` rows of `dst_pitch` bytes.
unsafe fn copy_pitched(src: &[u8], src_pitch: u32, dst: *mut u8, dst_pitch: u32) {
    if src_pitch == 0 || dst_pitch == 0 || src_pitch == dst_pitch {
        // SAFETY: the caller's.
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len()) };
        return;
    }
    let width = src_pitch.min(dst_pitch) as usize;
    for (i, row) in src.chunks(src_pitch as usize).enumerate() {
        let n = width.min(row.len());
        // SAFETY: the caller's: row `i` of `dst` holds `dst_pitch` bytes.
        unsafe { std::ptr::copy_nonoverlapping(row.as_ptr(), dst.add(i * dst_pitch as usize), n) };
    }
}

impl Backend for D3d11 {
    fn name(&self) -> &'static str {
        "Direct3D 11"
    }

    fn create_device(&mut self, desc: &SwapChainDesc, flags: u32) -> u32 {
        let mut device = None;
        let mut context = None;
        let mut level = D3D_FEATURE_LEVEL::default();
        let w = desc.0;
        let r = if desc.window() == 0 {
            // SAFETY: the out-pointers are locals.
            unsafe {
                D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_FLAG(flags),
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&raw mut device),
                    Some(&raw mut level),
                    Some(&raw mut context),
                )
            }
        } else {
            // the game's description, field by field (DXGI_SWAP_CHAIN_DESC)
            #[allow(clippy::cast_possible_wrap)] // enums and BOOL
            let sd = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC {
                    Width: w[0],
                    Height: w[1],
                    RefreshRate: DXGI_RATIONAL {
                        Numerator: w[2],
                        Denominator: w[3],
                    },
                    Format: DXGI_FORMAT(w[4] as i32),
                    ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER(w[5] as i32),
                    Scaling: DXGI_MODE_SCALING(w[6] as i32),
                },
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: w[7],
                    Quality: w[8],
                },
                BufferUsage: DXGI_USAGE(w[9]),
                BufferCount: w[10],
                OutputWindow: HWND(w[11] as usize as *mut _),
                Windowed: windows::core::BOOL(w[12] as i32),
                SwapEffect: DXGI_SWAP_EFFECT(w[13] as i32),
                Flags: w[14],
            };
            let mut chain = None;
            // SAFETY: the description and the out-pointers are locals.
            let r = unsafe {
                D3D11CreateDeviceAndSwapChain(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_FLAG(flags),
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&raw const sd),
                    Some(&raw mut chain),
                    Some(&raw mut device),
                    Some(&raw mut level),
                    Some(&raw mut context),
                )
            };
            self.swap_chain = chain;
            r
        };
        if let Err(e) = r {
            return hr(&e);
        }
        self.context1 = context
            .as_ref()
            .and_then(|c: &ID3D11DeviceContext| c.cast().ok());
        self.device = device;
        self.context = context;
        S_OK
    }

    fn back_buffer(&mut self, id: u32) -> u32 {
        if self.objects.contains_key(&id) {
            return S_OK;
        }
        self.back_buffer = id;
        if let Some(chain) = self.swap_chain.as_ref() {
            // SAFETY: buffer 0 of our own swap chain.
            return match unsafe { chain.GetBuffer::<ID3D11Texture2D>(0) } {
                Ok(t) => self.keep(id, Some(t), 0),
                Err(e) => hr(&e),
            };
        }
        // no window yet: an offscreen texture stands for the back buffer
        let desc = D3D11_TEXTURE2D_DESC {
            Width: 1920,
            Height: 1080,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT(28),
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BindFlags: 0x28,
            ..Default::default()
        };
        let Some(d) = self.device.clone() else {
            return E_FAIL;
        };
        let mut out: Option<ID3D11Texture2D> = None;
        // SAFETY: the description and the out-pointer are locals.
        match unsafe { d.CreateTexture2D(&raw const desc, None, Some(&raw mut out)) } {
            Ok(()) => self.keep(id, out, 0),
            Err(e) => hr(&e),
        }
    }

    fn create(&mut self, id: u32, what: &Create<'_>) -> u32 {
        self.make(id, what).unwrap_or_else(|e| hr(&e))
    }

    fn release(&mut self, id: u32) {
        self.objects.remove(&id);
    }

    fn op(&mut self, op: &Op<'_>) {
        self.run(op);
    }

    fn read(&mut self, id: u32, sub: u32, row: u32, out: &mut [u8]) -> bool {
        let (Some(c), Some(r)) = (self.context.as_ref(), self.get::<ID3D11Resource>(id)) else {
            return false;
        };
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: the mapped pointer is valid until `Unmap`; at most
        // `out.len()` bytes are read, row by row at the smaller pitch.
        unsafe {
            if c.Map(&r, sub, D3D11_MAP_READ, 0, Some(&raw mut m)).is_err() || m.pData.is_null() {
                return false;
            }
            let src = m.pData.cast::<u8>().cast_const();
            let pitch = if m.RowPitch == 0 { row } else { m.RowPitch } as usize;
            let row = row.max(1) as usize;
            for (i, chunk) in out.chunks_mut(row).enumerate() {
                let n = chunk.len().min(pitch);
                std::ptr::copy_nonoverlapping(src.add(i * pitch), chunk.as_mut_ptr(), n);
            }
            c.Unmap(&r, sub);
        }
        true
    }

    fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32 {
        let (Some(c), Some(q)) = (self.context.as_ref(), self.get::<ID3D11Asynchronous>(id)) else {
            return E_FAIL;
        };
        let size = u32::try_from(out.len()).unwrap_or(0);
        // SAFETY: `out` holds `size` bytes; the raw slot keeps `S_FALSE`
        // (not ready), which the wrapper folds into `Ok`.
        let r = unsafe {
            (Interface::vtable(c).GetData)(c.as_raw(), q.as_raw(), out.as_mut_ptr().cast(), size, 0)
        };
        match r.0 {
            0 => S_OK,
            1 => S_FALSE,
            e => u32::from_ne_bytes(e.to_ne_bytes()),
        }
    }

    fn present(&mut self, sync: u32, flags: u32) -> u32 {
        if let Some(chain) = self.swap_chain.as_ref() {
            // SAFETY: a plain call on our own swap chain.
            let r = unsafe { chain.Present(sync, DXGI_PRESENT(flags)) };
            return u32::from_ne_bytes(r.0.to_ne_bytes());
        }
        if let Some(c) = self.context.as_ref() {
            // SAFETY: a plain call on the device's own context.
            unsafe { c.Flush() };
        }
        S_OK
    }

    fn back_buffer_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        let (d, c) = (self.device.clone()?, self.context.clone()?);
        let bb = self.get::<ID3D11Texture2D>(self.back_buffer)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: a copy of the back buffer into a staging texture of its
        // description, mapped for reading; at most `w * 4` bytes a row are
        // read, `h` rows.
        unsafe {
            bb.GetDesc(&raw mut desc);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;
            let mut staging: Option<ID3D11Texture2D> = None;
            d.CreateTexture2D(&raw const desc, None, Some(&raw mut staging))
                .ok()?;
            let staging = staging?;
            c.CopyResource(&staging, &bb);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            c.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&raw mut m))
                .ok()?;
            let (w, h) = (desc.Width, desc.Height);
            let row = (w * 4) as usize;
            let mut out = vec![0u8; row * h as usize];
            let src = m.pData.cast::<u8>().cast_const();
            for y in 0..h as usize {
                std::ptr::copy_nonoverlapping(
                    src.add(y * m.RowPitch as usize),
                    out.as_mut_ptr().add(y * row),
                    row,
                );
            }
            c.Unmap(&staging, 0);
            // R8G8B8A8 (28, 29) to BGRA; B8G8R8A8 (87, 91) as is
            if matches!(desc.Format.0, 27..=32) {
                for px in out.as_chunks_mut::<4>().0 {
                    px.swap(0, 2);
                }
            }
            Some((w, h, out))
        }
    }
}
