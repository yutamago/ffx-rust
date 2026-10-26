//! h2-render-b continuation round: the D3D11 renderer's device and swap
//! chain creation (`PRendererD3D11_createDeviceAndSwapChain`).
//!
//! COM methods stay calls through the object's vtable (`stdcall`, the
//! object first) and the `d3d11.dll` imports calls through their thunks,
//! `ecx` as the original leaves it. The two imports take ten and twelve
//! words: they are `stdcall`, passed as a `thiscall` (the callee pops the
//! same words, and ignores `ecx`) because the hosts pass at most eight
//! words to a plain `stdcall`.

use ffx_graphic::host::{Conv, Host};

/// `D3D11CreateDevice`'s import thunk (stdcall, ten words).
const CREATE_DEVICE: u32 = 0x0096_838e;
/// `D3D11CreateDeviceAndSwapChain`'s import thunk (stdcall, twelve words).
const CREATE_DEVICE_AND_SWAP_CHAIN: u32 = 0x0096_8394;
/// `memset` (the CRT import thunk, cdecl).
const MEMSET: u32 = 0x0094_964c;
/// `g_Settings` (a pointer to the game's settings block).
const SETTINGS: u32 = 0x022f_b504;
/// `SetWordAt_00402a00(this; i, v)`: `this[i] = v`.
const SET_WORD_AT: u32 = 0x0040_2a00;
/// The interface IDs and textures the creation passes.
const IID_A: u32 = 0x00b3_1450;
const IID_B: u32 = 0x00b3_1460;
const IID_BACK_BUFFER: u32 = 0x00b3_1470;
const IID_FACTORY: u32 = 0x00b3_1480;
const COLOR_FORMAT: u32 = 0x00ca_310c;
const DEPTH_FORMAT: u32 = 0x00ca_347c;
/// `initA`/`initB`'s shader blobs (a length word, then the code), the
/// input semantics, the debug-name GUID and names, the record finisher.
const VS_A: u32 = 0x00c2_e2e0;
const PS_A: u32 = 0x00c2_e68c;
const PS_B: u32 = 0x00c2_e944;
const VS_B: u32 = 0x00c2_eccc;
const PS_B_BLOBS: [u32; 8] = [
    0x00c2_efb8,
    0x00c2_f210,
    0x00c2_f4a4,
    0x00c2_f774,
    0x00c2_fa80,
    0x00c2_fdc8,
    0x00c3_014c,
    0x00c3_050c,
];
const POSITION: u32 = 0x00b3_3c0c;
const TEXCOORD: u32 = 0x00b3_3c18;
const NAME_GUID: u32 = 0x00b6_ae3c;
const NAME_A: u32 = 0x00b3_3c24;
const NAME_B: u32 = 0x00b3_3c3c;
const FINISH_RECORD: u32 = 0x0058_d4e0;
/// Engine methods it calls (thiscall).
const SET_TEXTURE: u32 = 0x0058_fb90;
const CREATE_VIEWS: u32 = 0x0059_1460;
const SET_DIMENSIONS: u32 = 0x004c_40c0;
const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
const SET_VIEWPORT: u32 = 0x0059_bbf0;
const SET_RANGE: u32 = 0x005b_3660;
const INIT_A: u32 = 0x0059_4f70;
const INIT_B: u32 = 0x0059_5200;
const CREATE_DYNAMIC_BUFFERS: u32 = 0x0059_54f0;
const CREATE_BUFFERS: u32 = 0x0059_5620;
const REGISTER_TEXTURE: u32 = 0x0058_cf80;

/// `PRendererD3D11_setViewport_0058c790(this; x, y, w, h)`.
const SET_VIEWPORT_RECT: u32 = 0x0058_c790;
/// `PRenderTargetD3D11_releaseViews_0059c650` (thiscall).
const RELEASE_VIEWS: u32 = 0x0059_c650;

/// The texture-file list's iterator step `next(list; node)` (thiscall).
const TREE_NEXT: u32 = 0x0057_51d0;
const MUTEX_LOCK: u32 = 0x0043_1840;
const LOCK_RELEASE: u32 = 0x0043_19d0;
const MALLOC_ALIGNED: u32 = 0x0042_fb80;
const PHYRE_FREE: u32 = 0x0042_fb20;
const CLEAR_32: u32 = 0x0046_3d10;
const READER_CTOR: u32 = 0x0057_66d0;
const READER_CLOSE: u32 = 0x0057_6930;
const READER_DTOR: u32 = 0x0057_67c0;
/// `FUN_006080e0(obj; word, 0)` (thiscall on a local).
const SOURCE_CTOR: u32 = 0x0060_80e0;
const RELOAD_TEXTURES: u32 = 0x0046_6da0;

/// `createViews`' callees: the placeholder texture's descriptor, the
/// device-object makers of 2D, volume and cube textures, the resource
/// getters, the render scope lock and the renderer context.
const PLACEHOLDER: u32 = 0x00ca_34cc;
const MAKE_2D: u32 = 0x0058_fc20;
const MAKE_3D: u32 = 0x0059_03a0;
const MAKE_CUBE: u32 = 0x0059_0800;
const ELEMENT_FIELD: u32 = 0x0059_4060;
const RESOURCE_3D: u32 = 0x0059_3eb0;
const RESOURCE_CUBE: u32 = 0x0059_3f00;
const SCOPE_ENTER: u32 = 0x0046_3d50;
const SCOPE_LEAVE: u32 = 0x0046_3e90;
const RENDERER_CONTEXT: u32 = 0x0046_49d0;

/// The renderer constructor's callees.
const RENDERER_BASE_CTOR: u32 = 0x004f_ee80;
const TEXTURE_2D_CTOR: u32 = 0x004c_2f90;
const ZERO_ITEMS: u32 = 0x0058_b070;
const TEXTURE_DATA_CTOR: u32 = 0x004c_78a0;
const ZERO6_CTOR: u32 = 0x005b_3580;
const VECTOR_CTOR: u32 = 0x0094_966a;
const ZERO_QUAD: u32 = 0x005b_3620;
const SURFACE_DTOR: u32 = 0x005b_3640;
const SURFACE_CTOR: u32 = 0x005b_35c0;
const STATE_A_CTOR: u32 = 0x0058_3320;
const STATE_BLOCK_CTOR: u32 = 0x0058_3e20;
const STATE_CACHE_CTOR: u32 = 0x0058_33b0;
const SAMPLER_CTOR: u32 = 0x004b_e870;
const FREE_LIST_CTOR: u32 = 0x0049_c480;
const FREE_LIST_TAG: u32 = 0x00b1_3bc8;
const EVENT_OWNER_CTOR: u32 = 0x0043_15e0;
const TREE_REMOVE: u32 = 0x0057_5000;
const FREE_LIST_RESET: u32 = 0x0049_cc10;
const CRITICAL_SECTION_INIT: u32 = 0x0043_17f0;

/// `initWindow`'s defaults, `GetClientRect`'s import word and the
/// creation it calls.
const DEFAULT_A: u32 = 0x00b1_0f38;
const DEFAULT_B: u32 = 0x00b3_4254;
const DEFAULT_C: u32 = 0x00b9_2194;
const GET_CLIENT_RECT: u32 = 0x00b0_c680;
const CREATE_DEVICE_AND_SWAP_CHAIN_FN: u32 = 0x0059_5800;
/// `findContextVariant`'s switch count, key builder and comparison.
const SWITCH_COUNT: u32 = 0x00ca_4e20;
const BUILD_KEY: u32 = 0x004e_f3f0;
const COMPARE_WORDS: u32 = 0x004e_ea10;

/// What the creation returns on a failed D3D11 call.
const FAILED: u32 = 9;

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt + 4 * slot);
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 4 (three arguments at most)
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BUILD_KEY, COMPARE_WORDS, CREATE_DEVICE_AND_SWAP_CHAIN_FN, DEFAULT_A, DEFAULT_B, DEFAULT_C,
        GET_CLIENT_RECT, SWITCH_COUNT,
    };
    use super::{
        CLEAR_32, COLOR_FORMAT, CREATE_BUFFERS, CREATE_DEVICE, CREATE_DEVICE_AND_SWAP_CHAIN,
        CREATE_DYNAMIC_BUFFERS, CREATE_VIEWS, Conv, DEPTH_FORMAT, FAILED, Host, IID_A, IID_B,
        IID_BACK_BUFFER, IID_FACTORY, INIT_A, INIT_B, LOCK_RELEASE, MALLOC_ALIGNED, MEMSET,
        MUTEX_LOCK, PHYRE_FREE, READER_CLOSE, READER_CTOR, READER_DTOR, REGISTER_TEXTURE,
        RELEASE_VIEWS, RELOAD_TEXTURES, SET_DIMENSIONS, SET_RANGE, SET_TEXTURE, SET_VIEWPORT,
        SET_VIEWPORT_RECT, SET_WORD_AT, SETTINGS, SOURCE_CTOR, TREE_NEXT, UPDATE_MAX_LEVEL, com,
    };
    use super::{
        CRITICAL_SECTION_INIT, EVENT_OWNER_CTOR, FREE_LIST_CTOR, FREE_LIST_RESET, FREE_LIST_TAG,
        RENDERER_BASE_CTOR, SAMPLER_CTOR, STATE_A_CTOR, STATE_BLOCK_CTOR, STATE_CACHE_CTOR,
        SURFACE_CTOR, SURFACE_DTOR, TEXTURE_2D_CTOR, TEXTURE_DATA_CTOR, TREE_REMOVE, VECTOR_CTOR,
        ZERO_ITEMS, ZERO_QUAD, ZERO6_CTOR,
    };
    use super::{
        ELEMENT_FIELD, MAKE_2D, MAKE_3D, MAKE_CUBE, PLACEHOLDER, RENDERER_CONTEXT, RESOURCE_3D,
        RESOURCE_CUBE, SCOPE_ENTER, SCOPE_LEAVE,
    };
    use super::{
        FINISH_RECORD, NAME_A, NAME_B, NAME_GUID, POSITION, PS_A, PS_B, PS_B_BLOBS, TEXCOORD, VS_A,
        VS_B,
    };

    /// The bytes of `setTextureQuality`'s frame below its exception
    /// registration (`ebp - 0x34` up to `ebp - 0xc`).
    pub const QUALITY_FRAME: u32 = 0x28;

    /// `PRenderInterface::setTextureQuality(this; q)` (`0x0058e000`,
    /// thiscall, ret 4): when `q` differs from `+0x1340`, stores it and,
    /// under the lock at `+0x1374`, reloads the textures of every file in
    /// the list at `+0x1344` (a tree: its first node is found from
    /// `+0x1350` down the nodes' word 0, the next by `0x005751d0`): per
    /// file a 32-byte loader context (`PhyreMalloc`, zeroed), a file
    /// reader on the node's name (`+0x10`), a source object on its word
    /// `+0x18`, `reloadClusterTextures`, then the context freed and the
    /// reader closed and destroyed. Returns 0. `frame` is the original's
    /// `ebp - 0x34` (the reader and the source object live there).
    pub fn set_texture_quality(h: &mut impl Host, this: u32, q: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x34 - off;
        if h.read::<u32>(this + 0x1340) == q {
            return 0;
        }
        h.write(this + 0x1340, q);
        let lock = this + 0x1374;
        h.write(local(0x14), lock);
        h.call_sym(MUTEX_LOCK, Conv::Thiscall(lock), &[]);
        let list = this + 0x1344;
        // the first node: `+0xc` itself when it is the list or null, else
        // down word 0 to the node before the list
        let mut node: u32 = h.read(list + 0xc);
        if node != list && node != 0 {
            loop {
                let next: u32 = h.read(node);
                if next == list {
                    break;
                }
                node = next;
            }
        }
        if node != list {
            let mut next = h.call_sym(TREE_NEXT, Conv::Thiscall(list), &[node]);
            let reader = local(0x34);
            let source = local(0x20);
            loop {
                let file = node + 0x10;
                let ctx = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x20, 4]);
                h.write(local(0x10), ctx);
                h.write(local(0x18), ctx);
                if ctx != 0 {
                    h.call_sym(CLEAR_32, Conv::Thiscall(ctx), &[]);
                }
                h.write(local(0x10), ctx);
                let name: u32 = h.read(file);
                h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[name]);
                let word: u32 = h.read(file + 8);
                h.call_sym(SOURCE_CTOR, Conv::Thiscall(source), &[word, 0]);
                let ctx_now: u32 = h.read(local(0x10));
                h.call_sym(RELOAD_TEXTURES, Conv::Thiscall(ctx_now), &[reader, file]);
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[ctx]);
                h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
                node = next;
                if next != list {
                    next = h.call_sym(TREE_NEXT, Conv::Thiscall(list), &[next]);
                }
                h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
                if node == list {
                    break;
                }
            }
        }
        let lock: u32 = h.read(local(0x14));
        h.call_sym(LOCK_RELEASE, Conv::Thiscall(lock), &[]);
        0
    }

    /// The bytes of `resizeBuffers`'s frame (`ebp - 0xc` up to `ebp`).
    pub const RESIZE_FRAME: u32 = 0xc;

    /// `PRendererD3D11::resizeBuffers(this; width, height)`
    /// (`0x00599210`, thiscall, ret 8): with a swap chain (`+0x74`),
    /// unbinds the render targets, releases the back and depth buffers'
    /// views, `ResizeBuffers` (count `+0xa0`, format `+0x88`, flags 2);
    /// when that succeeds, wraps the new back buffer as at creation (the
    /// sample count `+0x94`), resizes the depth buffer, sets the
    /// viewport, the ranges and the description's size (`+0x78`); then
    /// binds the render target again. Without a back buffer it stops
    /// before the bind. Returns the last call's `eax` (0 when the back
    /// buffer is missing); without a swap chain the original returns the
    /// `eax` it was called with, which its callers drop (here 0).
    /// `frame` is the original's `ebp - 0xc`.
    pub fn resize_buffers(
        h: &mut impl Host,
        this: u32,
        width: u32,
        height: u32,
        frame: u32,
    ) -> u32 {
        let local = |off: u32| frame + 0xc - off;
        if h.read::<u32>(this + 0x74) == 0 {
            return 0;
        }
        let context: u32 = h.read(this + 0x70);
        com(h, this, context, 33, &[0, 0, 0]);
        h.call_sym(RELEASE_VIEWS, Conv::Thiscall(this + 0x1a0), &[]);
        h.call_sym(RELEASE_VIEWS, Conv::Thiscall(this + 0x1c8), &[]);
        let chain: u32 = h.read(this + 0x74);
        let count: u32 = h.read(this + 0xa0);
        let format: u32 = h.read(this + 0x88);
        let vt: u32 = h.read(chain);
        if com(h, vt, chain, 13, &[count, width, height, format, 2]) == 0 {
            let samples: u32 = h.read(this + 0x94);
            let buffer = local(0x4);
            h.write(buffer, 0u32);
            h.write(local(0x8), samples);
            let chain: u32 = h.read(this + 0x74);
            let iid = h.rebase(IID_BACK_BUFFER);
            let vt: u32 = h.read(chain);
            com(h, vt, chain, 9, &[0, iid, buffer]);
            let texture: u32 = h.read(buffer);
            if texture == 0 {
                return 0;
            }
            let low = if samples.wrapping_sub(1) != 0 {
                h.read::<u8>(local(0x8))
            } else {
                0
            };
            h.write(this + 0x1a5, low);
            let color = h.rebase(COLOR_FORMAT);
            h.call_sym(
                SET_TEXTURE,
                Conv::Thiscall(this + 0xb8),
                &[texture, color, 0, 0],
            );
            h.call_sym(CREATE_VIEWS, Conv::Thiscall(this + 0x1a0), &[]);
            h.write(this + 0x1cd, low);
            let depth = h.rebase(DEPTH_FORMAT);
            h.call_sym(
                SET_DIMENSIONS,
                Conv::Thiscall(this + 0x12c),
                &[width, height, depth, 0],
            );
            let v: u32 = h.read(this + 0x140);
            h.write(this + 0x140, v & !2);
            h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(this + 0x12c), &[]);
            h.call_sym(CREATE_VIEWS, Conv::Thiscall(this + 0x1c8), &[]);
            h.write(this + 0x1334, width);
            h.write(this + 0x1338, height);
            h.write(this + 0x1324, 0u32);
            h.write(this + 0x1328, 0u32);
            h.write(this + 0x132c, width);
            h.write(this + 0x1330, height);
            h.call_sym(
                SET_VIEWPORT_RECT,
                Conv::Thiscall(this),
                &[0, 0, width, height],
            );
            h.call_sym(
                SET_RANGE,
                Conv::Thiscall(this + 0x1f0),
                &[width, height, color],
            );
            h.call_sym(
                SET_RANGE,
                Conv::Thiscall(this + 0x20c),
                &[width, height, depth],
            );
            h.write(this + 0x78, width);
            h.write(this + 0x7c, height);
        }
        let view_arg: u32 = h.read(this + 0x1dc);
        let view: u32 = h.read(this + 0x1b0);
        h.write(local(0xc), view);
        let context: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(context);
        com(h, vt, context, 33, &[1, local(0xc), view_arg])
    }

    /// The bytes of `createDeviceAndSwapChain`'s frame below the cookie
    /// (`ebp - 0x40` up to `ebp - 4`).
    pub const CREATE_FRAME: u32 = 0x3c;

    /// `PRendererD3D11::createDeviceAndSwapChain(this; width, height,
    /// samples, windowed, flag, hwnd)` (`0x00595800`, thiscall, ret 0x18):
    /// probes `D3D11CreateDevice` for feature level 10.0 or better (10.0
    /// with the settings' words `+0x38` and `+0x48` both set turns the
    /// settings word 0xe off), fills the swap-chain description at
    /// `+0x78` (`samples` clamped to 1..=16), creates the device, context
    /// and swap chain (`+0x6c`, `+0x70`, `+0x74`), wraps the back buffer
    /// (`+0xb8`) and the depth buffer (`+0x12c`) and their views, sets the
    /// viewport and ranges, creates the renderer's buffers, a state object
    /// on 11.0 devices, binds the render target, registers the depth
    /// texture and tells the factory not to take Alt+Enter. Returns 0, 9
    /// when a D3D11 call fails, or the failing engine call's result.
    /// `frame` is the original's `ebp - 0x40` (the callees get its
    /// locals' addresses).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn create_device_and_swap_chain(
        h: &mut impl Host,
        this: u32,
        width: u32,
        height: u32,
        samples: u32,
        windowed: u8,
        flag: u8,
        hwnd: u32,
        frame: u32,
    ) -> u32 {
        // the locals at `ebp - off`
        let local = |off: u32| frame + 0x40 - off;
        h.write(local(0x38), hwnd);
        let samples = samples.clamp(1, 0x10);
        // the feature levels asked for: 11.0, 10.1, 10.0
        h.write(local(0x10), 0xb000u32);
        h.write(local(0xc), 0xa100u32);
        h.write(local(0x8), 0xa000u32);
        let level = local(0x2c);
        h.write(level, 0x9100u32);
        let create = h.rebase(CREATE_DEVICE);
        let r = h.call(
            create,
            Conv::Thiscall(this),
            &[0, 1, 0, 0, 0, 0, 7, 0, level, 0],
        );
        if (r as i32) < 0 || h.read::<i32>(level) < 0xa000 {
            return FAILED;
        }
        if h.read::<u32>(level) == 0xa000 {
            let settings: u32 = h.read(h.rebase(SETTINGS));
            if h.read::<u32>(settings + 0x38) != 0 && h.read::<u32>(settings + 0x48) != 0 {
                h.call_sym(SET_WORD_AT, Conv::Thiscall(settings), &[0xe, 0]);
            }
        }
        // the swap-chain description
        let desc = this + 0x78;
        h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x3c]);
        h.write(desc, width);
        h.write(desc + 4, height);
        h.write(desc + 0x2c, hwnd);
        h.write(desc + 0x1c, samples);
        h.write(desc + 0x28, 1u32);
        h.write(desc + 0x30, u32::from(windowed != 0));
        h.write(desc + 0x10, 0x1cu32);
        h.write(desc + 8, 0x3cu32);
        h.write(desc + 0xc, 1u32);
        h.write(desc + 0x24, 0x20u32);
        h.write(desc + 0x20, 0u32);
        h.write(desc + 0x34, 0u32);
        h.write(desc + 0x38, 2u32);
        let create = h.rebase(CREATE_DEVICE_AND_SWAP_CHAIN);
        let r = h.call(
            create,
            Conv::Thiscall(level),
            &[
                0,
                1,
                0,
                0,
                local(0x10),
                3,
                7,
                desc,
                this + 0x74,
                this + 0x6c,
                level,
                this + 0x70,
            ],
        );
        if r != 0 || h.read::<u32>(this + 0x70) == 0 {
            return FAILED;
        }
        let device: u32 = h.read(this + 0x6c);
        if device == 0 || h.read::<u32>(this + 0x74) == 0 {
            return FAILED;
        }
        let got: u32 = h.read(level);
        h.write(this + 0xb4, got);
        // an interface of the device and one of that (released)
        let out = local(0x34);
        h.write(out, 0u32);
        let iid = h.rebase(IID_A);
        if com(h, device, device, 0, &[iid, out]) as i32 >= 0 {
            let q: u32 = h.read(out);
            let out2 = local(0x30);
            h.write(out2, 0u32);
            let iid = h.rebase(IID_B);
            let vt: u32 = h.read(q);
            com(h, vt, q, 0, &[iid, out2]);
            let q: u32 = h.read(out);
            let vt: u32 = h.read(q);
            com(h, vt, q, 2, &[]);
        }
        // the back buffer
        let chain: u32 = h.read(this + 0x74);
        let buffer = local(0x3c);
        h.write(buffer, 0u32);
        let iid = h.rebase(IID_BACK_BUFFER);
        let vt: u32 = h.read(chain);
        com(h, vt, chain, 9, &[0, iid, buffer]);
        let texture: u32 = h.read(buffer);
        if texture == 0 {
            return FAILED;
        }
        let v: u32 = h.read(this + 0x1a8);
        h.write(this + 0x1a8, v & !1);
        // `samples` when more than one, else 0, in the low byte of
        // `-(samples - 1)`
        let m = samples.wrapping_sub(1);
        let low = if m != 0 { samples & 0xff } else { 0 };
        let multi = (m.wrapping_neg() & !0xff) | low;
        h.write(this + 0x1a5, low as u8);
        h.write(local(0x30), multi);
        let color = h.rebase(COLOR_FORMAT);
        let r = h.call_sym(
            SET_TEXTURE,
            Conv::Thiscall(this + 0xb8),
            &[texture, color, 0, 0],
        );
        if r != 0 {
            return r;
        }
        let r = h.call_sym(CREATE_VIEWS, Conv::Thiscall(this + 0x1a0), &[]);
        if r != 0 {
            return r;
        }
        let multi: u32 = h.read(local(0x30));
        let v: u32 = h.read(this + 0x1d0);
        h.write(this + 0x1d0, v & !1);
        h.write(this + 0x1cd, multi as u8);
        let depth = h.rebase(DEPTH_FORMAT);
        h.call_sym(
            SET_DIMENSIONS,
            Conv::Thiscall(this + 0x12c),
            &[width, height, depth, 0],
        );
        let v: u32 = h.read(this + 0x140);
        h.write(this + 0x140, v & !2);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(this + 0x12c), &[]);
        let r = h.call_sym(CREATE_VIEWS, Conv::Thiscall(this + 0x1c8), &[]);
        if r != 0 {
            return r;
        }
        h.write(this + 0x1334, width);
        h.write(this + 0x1338, height);
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(this), &[0, 0, width, height]);
        h.call_sym(
            SET_RANGE,
            Conv::Thiscall(this + 0x1f0),
            &[width, height, color],
        );
        h.call_sym(
            SET_RANGE,
            Conv::Thiscall(this + 0x20c),
            &[width, height, depth],
        );
        let r = h.call_sym(INIT_A, Conv::Thiscall(this + 0x394), &[this]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(INIT_B, Conv::Thiscall(this + 0x6ec), &[this]);
        if r != 0 {
            return r;
        }
        let words: [u32; 3] = [
            h.read(this + 0x6c),
            h.read(this + 0x5c),
            h.read(this + 0x60),
        ];
        h.write(this + 0x131c, flag);
        let r = h.call_sym(CREATE_DYNAMIC_BUFFERS, Conv::Thiscall(this + 0x320), &words);
        if r != 0 {
            return r;
        }
        let words: [u32; 3] = [
            h.read(this + 0x6c),
            h.read(this + 0x64),
            h.read(this + 0x68),
        ];
        let r = h.call_sym(
            CREATE_BUFFERS,
            Conv::Thiscall(this + 0x348),
            &[words[0], words[1], words[2], 0, 0, 0],
        );
        if r != 0 {
            return r;
        }
        if h.read::<i32>(this + 0xb4) >= 0xb000 {
            // a 24-byte description: words 0x10, 0, 0, 0, 0x10, 4
            let d = local(0x28);
            for (k, w) in [0x10u32, 0, 0, 0, 0x10, 4].into_iter().enumerate() {
                h.write(d + 4 * k as u32, w);
            }
            let device: u32 = h.read(this + 0x6c);
            com(h, device, device, 3, &[d, 0, this + 0x378]);
        }
        let view_arg: u32 = h.read(this + 0x1dc);
        let view: u32 = h.read(this + 0x1b0);
        h.write(local(0x40), view);
        let context: u32 = h.read(this + 0x70);
        let vt: u32 = h.read(context);
        com(h, vt, context, 33, &[1, local(0x40), view_arg]);
        let r = h.call_sym(REGISTER_TEXTURE, Conv::Thiscall(this + 0x12f8), &[]);
        if r != 0 {
            return r;
        }
        let chain: u32 = h.read(this + 0x74);
        let iid = h.rebase(IID_FACTORY);
        if com(h, chain, chain, 6, &[iid, this + 0x133c]) as i32 >= 0 {
            let factory: u32 = h.read(this + 0x133c);
            let vt: u32 = h.read(factory);
            let hwnd: u32 = h.read(local(0x38));
            com(h, vt, factory, 8, &[hwnd, 2]);
        }
        0
    }

    /// The bytes of `createViews`' frame below its exception registration
    /// (`ebp - 0x50` up to `ebp - 0xc`).
    pub const VIEWS_FRAME: u32 = 0x44;

    /// The first word of a texture's storage array (`count` at `t + at`,
    /// the inline word or the storage pointer at `t + at + 4`): the
    /// storage's address, 0 when empty (the original then reads address
    /// 0, as this does).
    fn first_slot(h: &mut impl Host, t: u32, at: u32) -> u32 {
        match h.read::<u32>(t + at) & 0x7fff_ffff {
            0 => 0,
            1 => t + at + 4,
            _ => h.read(t + at + 4),
        }
    }

    /// The texture's device resource: the storage array's first word
    /// (none when empty), or the element getter's when `pick` and it has
    /// one.
    fn resource(h: &mut impl Host, t: u32, pick: bool) -> u32 {
        let mut res = match h.read::<u32>(t + 0x24) & 0x7fff_ffff {
            0 => 0,
            1 => h.read(t + 0x28),
            _ => {
                let p: u32 = h.read(t + 0x28);
                h.read(p)
            }
        };
        if pick && h.call_sym(ELEMENT_FIELD, Conv::Thiscall(t), &[0]) != 0 {
            res = h.call_sym(ELEMENT_FIELD, Conv::Thiscall(t), &[0]);
        }
        res
    }

    /// Makes sure the texture's device object exists (`made` its flag
    /// word): when its storage holds no data, `make(t; 0, 1, w, 0)` with
    /// `w` the format's bit 10 (a byte in the frame word at `ebp - 0x20`,
    /// whose upper bytes the original never writes) or 0.
    fn ensure(h: &mut impl Host, t: u32, made: u32, storage: u32, make: u32, word: Option<u32>) {
        if h.read::<u32>(t + made) != 0 {
            return;
        }
        let p = first_slot(h, t, storage);
        if h.read::<u32>(p) == 0 {
            let w = if let Some(at) = word {
                let flags: u32 = h.read(t + 0x14);
                h.write(at, ((flags >> 10) & 1) as u8);
                // junk read: [ebp-0x1f..0x1d] never written; 0 (brief 2026-10-24)
                h.read(at)
            } else {
                0
            };
            h.call_sym(make, Conv::Thiscall(t), &[0, 1, w, 0]);
        }
        h.write(t + made, 1u32);
    }

    /// A per-face view array (`count` at `v + at`, storage at `+4`) sized
    /// to six: a new zeroed 24-byte block (the old storage freed when
    /// owned) unless it holds six already or the allocation fails.
    fn six_faces(h: &mut impl Host, v: u32, at: u32) {
        if h.read::<u32>(v + at) == 6 {
            return;
        }
        let m = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x18, 4]);
        if m == 0 {
            return;
        }
        for k in 0..6 {
            h.write(m + 4 * k, 0u32);
        }
        let old: u32 = h.read(v + at + 4);
        if old != m && h.read::<u32>(v + at) >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(v + at + 4, m);
        h.write(v + at, 6u32);
    }

    /// `PRenderTargetD3D11::createViews(this)` (`0x00591460`, thiscall):
    /// the views of the target's texture (`[this]`, of kind byte `+4`:
    /// 0 a 2D texture, 1 a volume, 2 a cube), created once under the
    /// render scope lock: a depth-stencil view (`+0x14`, when the format's
    /// flags `+0x10` have bit 1) or a render-target view (`+0x10`) of the
    /// whole texture, multisampled when `+5` is set (2D) or the cube's
    /// `+0x4c` is not 1; a cube also gets one view per face (the arrays
    /// `+0x18`/`+0x1c` and `+0x20`/`+0x24`). The texture's device object
    /// is made first when missing. Returns 0, 7 for the placeholder
    /// texture (`0x00ca34cc`), 9 when a creation fails or for a depth
    /// volume. `frame` is the original's `ebp - 0x50` (the descriptions,
    /// the locks and the out words live there).
    #[allow(clippy::too_many_lines)]
    pub fn create_views(h: &mut impl Host, v: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x50 - off;
        let kind: u8 = h.read(v + 4);
        let t: u32 = h.read(v);
        if kind == 2 {
            h.write(local(0x20), t);
        }
        if t == 0 || kind > 2 {
            return 0;
        }
        let placeholder = h.rebase(PLACEHOLDER);
        if h.read::<u32>(t) == placeholder {
            return 7;
        }
        match kind {
            0 => {
                ensure(h, t, 0x70, 0x24, MAKE_2D, Some(local(0x20)));
                let d: u32 = h.read(t);
                h.write(local(0x14), d);
                let multi = h.read::<u8>(v + 5) != 0;
                let pick = h.read::<u8>(v + 8) & 1 != 0;
                let depth = (h.read::<u32>(d + 0x10) >> 1) & 1 != 0;
                let slot = if depth { v + 0x14 } else { v + 0x10 };
                h.write(local(0x24), slot);
                if h.read::<u32>(slot) != 0 {
                    return 0;
                }
                let lock = local(0x10);
                h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
                let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
                let device: u32 = h.read(ctx + 0x6c);
                // the description: depth-stencil at `ebp - 0x50` (format,
                // dimension 3 or 5, flags, slice), render-target at
                // `ebp - 0x4c` (format, dimension 4 or 6)
                let (desc, method) = if depth {
                    for off in [0x50, 0x4c, 0x44, 0x48, 0x40, 0x3c] {
                        h.write(local(off), 0u32);
                    }
                    (local(0x50), 10)
                } else {
                    for off in [0x4c, 0x48, 0x44, 0x40, 0x3c] {
                        h.write(local(off), 0u32);
                    }
                    (local(0x4c), 9)
                };
                h.write(local(0x18), device);
                let format: u32 = h.read(d + 0x1c);
                h.write(desc, format);
                h.write(local(0x44), 0u32);
                let dim = u32::from(multi) * 2 + if depth { 3 } else { 4 };
                h.write(desc + 4, dim);
                let res = resource(h, t, pick);
                let ok = if res == 0 {
                    true
                } else {
                    let vt: u32 = h.read(device);
                    com(h, vt, device, method, &[res, desc, slot]) as i32 >= 0
                };
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                if ok { 0 } else { FAILED }
            }
            1 => {
                ensure(h, t, 0x64, 0x28, MAKE_3D, Some(local(0x20)));
                let d: u32 = h.read(t);
                if (h.read::<u32>(d + 0x10) >> 1) & 1 != 0 {
                    return FAILED;
                }
                let slot = v + 0x10;
                h.write(local(0x24), slot);
                if h.read::<u32>(slot) != 0 {
                    return 0;
                }
                let lock = local(0x10);
                h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
                let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
                let device: u32 = h.read(ctx + 0x6c);
                let desc = local(0x4c);
                for off in [0x4c, 0x48, 0x44, 0x40, 0x3c] {
                    h.write(local(off), 0u32);
                }
                let format: u32 = h.read(d + 0x1c);
                h.write(desc, format);
                let depth_count: u32 = h.read(t + 0x48);
                h.write(local(0x3c), depth_count);
                h.write(local(0x48), 8u32);
                h.write(local(0x44), 0u32);
                h.write(local(0x40), 0u32);
                let vt: u32 = h.read(device);
                let f: u32 = h.read(vt + 4 * 9);
                let res = h.call_sym(RESOURCE_3D, Conv::Thiscall(t), &[0]);
                // `ecx` as the getter leaves it (the method ignores it)
                let r = h.call(f, Conv::Thiscall(t), &[device, res, desc, slot]);
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                if r as i32 >= 0 { 0 } else { FAILED }
            }
            _ => {
                ensure(h, t, 0x64, 0x20, MAKE_CUBE, None);
                let d: u32 = h.read(t);
                let depth = (h.read::<u32>(d + 0x10) >> 1) & 1 != 0;
                let multi = h.read::<u32>(t + 0x4c) != 1;
                // depth: lock `ebp - 0x14`, description `ebp - 0x50`,
                // device word `ebp - 0x10`, out word `ebp - 0x1c`, the
                // array `+0x20`; else `ebp - 0x18`, `ebp - 0x38`,
                // `ebp - 0x14`, `ebp - 0x24`, `+0x18`
                let (slot, lock, desc, dev_at, out, faces, method, base) = if depth {
                    (
                        v + 0x14,
                        local(0x14),
                        local(0x50),
                        local(0x10),
                        local(0x1c),
                        0x20,
                        10,
                        3,
                    )
                } else {
                    (
                        v + 0x10,
                        local(0x18),
                        local(0x38),
                        local(0x14),
                        local(0x24),
                        0x18,
                        9,
                        4,
                    )
                };
                if h.read::<u32>(slot) != 0 {
                    return 0;
                }
                h.call_sym(SCOPE_ENTER, Conv::Thiscall(lock), &[]);
                let ctx = h.call_sym(RENDERER_CONTEXT, Conv::Cdecl, &[]);
                let device: u32 = h.read(ctx + 0x6c);
                if depth {
                    for off in [0x50, 0x4c, 0x44, 0x40, 0x3c, 0x48] {
                        h.write(local(off), 0u32);
                    }
                } else {
                    for k in 0..5 {
                        h.write(desc + 4 * k, 0u32);
                    }
                }
                let format: u32 = h.read(d + 0x1c);
                h.write(desc, format);
                // the array description: first slice, size 6
                h.write(desc + 8 + if depth { 4 } else { 0 }, 0u32);
                h.write(desc + 0xc + if depth { 4 } else { 0 }, 0u32);
                h.write(desc + 0x10 + if depth { 4 } else { 0 }, 6u32);
                h.write(dev_at, device);
                h.write(desc + 4, u32::from(multi) * 2 + base);
                let first = desc + 0xc + if depth { 4 } else { 0 };
                let vt: u32 = h.read(device);
                let f: u32 = h.read(vt + 4 * method);
                let res = h.call_sym(RESOURCE_CUBE, Conv::Thiscall(t), &[0]);
                let mut ok = h.call(f, Conv::Thiscall(t), &[device, res, desc, slot]) as i32 >= 0;
                if ok {
                    six_faces(h, v, faces);
                    for i in 0..6u32 {
                        let device: u32 = h.read(dev_at);
                        let t: u32 = h.read(local(0x20));
                        h.write(first, i);
                        h.write(first + 4, 1u32);
                        let vt: u32 = h.read(device);
                        let f: u32 = h.read(vt + 4 * method);
                        h.write(out, 0u32);
                        let res = h.call_sym(RESOURCE_CUBE, Conv::Thiscall(t), &[0]);
                        let r = h.call(f, Conv::Thiscall(t), &[device, res, desc, out]);
                        let view: u32 = h.read(out);
                        if (r as i32) < 0 || view == 0 {
                            ok = false;
                            break;
                        }
                        let array: u32 = h.read(v + faces + 4);
                        h.write(array + 4 * i, view);
                    }
                }
                h.call_sym(SCOPE_LEAVE, Conv::Thiscall(lock), &[]);
                if ok { 0 } else { FAILED }
            }
        }
    }

    /// `PRendererD3D11::PRendererD3D11(this)` (`0x00583580`, thiscall):
    /// the base (`0x004fee80`), the device words cleared, the back and
    /// depth buffer textures (`+0xb8`, `+0x12c`: `PTexture2DBase` with
    /// one inline level, their 44-byte blocks cleared), their views
    /// (`+0x1a0`, `+0x1c8`), two ranges (`+0x1f0`, `+0x20c`), eight
    /// render surfaces (`+0x228`) and one more (`+0x2a8`), the bound
    /// state words, the state caches (`+0x394`, `+0x570`, `+0x6ec`), the
    /// sampler (`+0x12f8`, its default filter bytes unless locked), the
    /// texture-file list (`+0x1344`, emptied) with its free list and the
    /// critical section (`+0x1374`). Returns `this`.
    #[allow(clippy::too_many_lines)]
    pub fn renderer_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(RENDERER_BASE_CTOR, Conv::Thiscall(this), &[]);
        for off in [0x6c, 0x70, 0x74] {
            h.write(this + off, 0u32);
        }
        // the two textures: one inline level each (the `!= 0` tests of
        // the original's inline construction kept)
        for tex in [this + 0xb8, this + 0x12c] {
            h.call_sym(TEXTURE_2D_CTOR, Conv::Thiscall(tex), &[]);
            let levels = tex + 0x28;
            h.write(tex + 0x24, 0u32);
            if levels != 0 {
                h.call_sym(ZERO_ITEMS, Conv::Cdecl, &[levels, 1]);
                h.write(tex + 0x24, 1u32);
            }
            h.write(tex + 0x70, 0u32);
            h.call_sym(MEMSET, Conv::Cdecl, &[tex + 0x44, 0, 0x2c]);
        }
        for (views, tex) in [(this + 0x1a0, this + 0xb8), (this + 0x1c8, this + 0x12c)] {
            h.call_sym(TEXTURE_DATA_CTOR, Conv::Thiscall(views), &[tex]);
            for k in 0..6 {
                h.write(views + 0x10 + 4 * k, 0u32);
            }
        }
        h.call_sym(ZERO6_CTOR, Conv::Thiscall(this + 0x1f0), &[]);
        h.call_sym(ZERO6_CTOR, Conv::Thiscall(this + 0x20c), &[]);
        let surfaces = this + 0x228;
        let ctor = h.rebase(ZERO_QUAD);
        let dtor = h.rebase(SURFACE_DTOR);
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[surfaces, 0x10, 8, ctor, dtor]);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surfaces + 0x80), &[0, 0]);
        for off in [0xb0, 0xd4, 0xd8] {
            h.write(surfaces + off, 0u32);
        }
        for off in [
            0x31c, 0x330, 0x334, 0x338, 0x33c, 0x320, 0x324, 0x328, 0x32c, 0x340, 0x344, 0x358,
            0x35c, 0x360, 0x364, 0x348, 0x34c, 0x350, 0x354, 0x368, 0x36c, 0x370, 0x374,
        ] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x38c, 1.0f32);
        h.write(this + 0x378, 0u32);
        h.write(this + 0x390, 0u32);
        h.call_sym(STATE_A_CTOR, Conv::Thiscall(this + 0x394), &[]);
        h.call_sym(STATE_BLOCK_CTOR, Conv::Thiscall(this + 0x570), &[]);
        h.call_sym(STATE_CACHE_CTOR, Conv::Thiscall(this + 0x6ec), &[]);
        let sampler = this + 0x12f8;
        h.call_sym(SAMPLER_CTOR, Conv::Thiscall(sampler), &[]);
        h.write(sampler + 0x20, 0u32);
        h.write(this + 0x131c, 0u8);
        for k in 0..9 {
            h.write(this + 0x1320 + 4 * k, 0u32);
        }
        // the list: an empty tree header (all six words itself)
        let list = this + 0x1344;
        for k in 0..6 {
            h.write(list + 4 * k, list);
        }
        let tag = h.rebase(FREE_LIST_TAG);
        h.call_sym(
            FREE_LIST_CTOR,
            Conv::Thiscall(list + 0x18),
            &[0x38, 8, 4, tag],
        );
        h.call_sym(EVENT_OWNER_CTOR, Conv::Thiscall(this + 0x1374), &[]);
        // the sampler's default filter bytes, unless its flags' bit 7
        for (k, v) in [0u8, 0, 2, 2, 2].into_iter().enumerate() {
            if (h.read::<u32>(sampler + 0x1c) >> 7) & 1 == 0 {
                h.write(sampler + k as u32, v);
            }
        }
        loop {
            let node: u32 = h.read(list + 0xc);
            if node == list || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(list), &[node]);
        }
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(list + 0x18), &[]);
        h.call_sym(CRITICAL_SECTION_INIT, Conv::Thiscall(this + 0x1374), &[]);
        this
    }

    /// The bytes of `initWindow`'s frame below the cookie (`ebp - 0x14`
    /// up to `ebp - 4`: the client rectangle).
    pub const WINDOW_FRAME: u32 = 0x10;

    /// `PRendererD3D11::initWindow(this; info)` (`0x00595750`, thiscall,
    /// ret 4): 1 without `info`; else the renderer's four default floats
    /// (`+0x37c`, `+0x380` from `0x00b10f38`, `+0x384` 0.8, `+0x388`
    /// 0.5), `info` at `+0x1320`, `GetClientRect(info.hwnd)`, and
    /// `createDeviceAndSwapChain(width, height, info + 4, 1, info + 9
    /// (byte), hwnd)`, whose result it returns. `frame` is the original's
    /// `ebp - 0x14`.
    pub fn init_window(h: &mut impl Host, this: u32, info: u32, frame: u32) -> u32 {
        if info == 0 {
            return 1;
        }
        // f32 copies through the FPU (the constants are no signalling
        // NaNs)
        let a: u32 = h.read(h.rebase(DEFAULT_A));
        h.write(this + 0x37c, a);
        h.write(this + 0x380, a);
        let b: u32 = h.read(h.rebase(DEFAULT_B));
        h.write(this + 0x384, b);
        h.write(this + 0x1320, info);
        let c: u32 = h.read(h.rebase(DEFAULT_C));
        h.write(this + 0x388, c);
        let rect = frame;
        let hwnd: u32 = h.read(info);
        let f: u32 = h.read(h.rebase(GET_CLIENT_RECT));
        // stdcall, `ecx` the rectangle as the original leaves it
        h.call(f, Conv::Thiscall(rect), &[hwnd, rect]);
        let info: u32 = h.read(this + 0x1320);
        let [left, top, right, bottom]: [u32; 4] = [
            h.read(rect),
            h.read(rect + 4),
            h.read(rect + 8),
            h.read(rect + 0xc),
        ];
        let hwnd: u32 = h.read(info);
        let flag: u8 = h.read(info + 9);
        let size: u32 = h.read(info + 4);
        h.call_sym(
            CREATE_DEVICE_AND_SWAP_CHAIN_FN,
            Conv::Thiscall(this),
            &[
                right.wrapping_sub(left),
                bottom.wrapping_sub(top),
                size,
                1,
                u32::from(flag),
                hwnd,
            ],
        )
    }

    /// The bytes of `findContextVariant`'s frame below the cookie
    /// (`ebp - 0x14` up to `ebp - 4`).
    pub const VARIANT_FRAME: u32 = 0x10;

    /// The words `findContextVariant` takes for its key buffer (an
    /// `alloca` of the switch count `0x00ca4e20` in words).
    pub fn variant_key_words(h: &mut impl Host) -> u32 {
        h.read(h.rebase(SWITCH_COUNT))
    }

    /// `PEffect::findContextVariant(this; a0..a4)` (`0x004ef440`,
    /// thiscall, ret 0x14): -1 without context variants (`+0x2c`); else
    /// builds the node's packed switch key into `buf` (`0x004ef3f0`,
    /// which advances the cursor at `ebp - 0xc`) and binary-searches the
    /// variants' 8-byte entries at `+0x30` with `0x004eea10` (the key
    /// and its word count): the index found, or -1. The search narrows
    /// to `lo = max(lo + 1, mid)` / `hi = min(hi - 1, mid)`, signed.
    /// `buf` is the original's `alloca` (16-aligned, below its saved
    /// registers), `frame` its `ebp - 0x14`.
    pub fn find_context_variant(
        h: &mut impl Host,
        this: u32,
        args: [u32; 5],
        buf: u32,
        frame: u32,
    ) -> u32 {
        let local = |off: u32| frame + 0x14 - off;
        let n = h.read::<u32>(this + 0x2c) & 0x7fff_ffff;
        if n == 0 {
            return u32::MAX;
        }
        h.write(local(0x8), buf);
        let cursor = local(0xc);
        h.write(cursor, buf);
        let [a0, a1, a2, a3, a4] = args;
        h.call_sym(
            BUILD_KEY,
            Conv::Thiscall(this),
            &[a0, a1, cursor, a2, a3, a4],
        );
        let end: u32 = h.read(cursor);
        let buf: u32 = h.read(local(0x8));
        let count = ((end.wrapping_sub(buf) as i32) >> 2) as u32;
        h.write(local(0x10), count);
        let data: u32 = h.read(this + 0x30);
        h.write(local(0x14), data);
        let mut lo: i32 = 0;
        let mut hi = n as i32 - 1;
        loop {
            let mid = (hi.wrapping_add(lo)) >> 1;
            let entry = data.wrapping_add((mid as u32).wrapping_mul(8));
            let r = h.call_sym(COMPARE_WORDS, Conv::Thiscall(entry), &[buf, count]) as i32;
            if r == 0 {
                return mid as u32;
            }
            if r < 0 {
                lo = (lo + 1).max(mid);
            } else {
                hi = (hi - 1).min(mid);
            }
            if lo > hi {
                return u32::MAX;
            }
        }
    }

    /// A shader creation `slot` (12 vertex, 15 pixel) of `device` from
    /// the blob whose length word is at `blob` (the code follows it; the
    /// length counts two more), into `out`; `ecx` as the original leaves
    /// it.
    fn create_shader(
        h: &mut impl Host,
        ecx: u32,
        device: u32,
        slot: u32,
        blob: u32,
        out: u32,
    ) -> u32 {
        let blob = h.rebase(blob);
        let len: u32 = h.read(blob);
        com(
            h,
            ecx,
            device,
            slot,
            &[blob + 4, len.wrapping_add(0xffff_fffe), 0, out],
        )
    }

    /// An input element (7 words) at `at`: semantic name, index, format,
    /// slot, the rest 0.
    fn input_element(h: &mut impl Host, at: u32, name: u32, index: u32, format: u32, slot: u32) {
        let name = h.rebase(name);
        for (k, w) in [name, index, format, slot, 0, 0, 0].into_iter().enumerate() {
            h.write(at + 4 * k as u32, w);
        }
    }

    /// The bytes of `initA`'s frame below the cookie (`ebp - 0x90` up to
    /// `ebp - 4`).
    pub const INIT_A_FRAME: u32 = 0x8c;

    /// `PRendererD3D11::initA(this; renderer)` (`0x00594f70`, thiscall,
    /// ret 4; `this` = renderer `+0x394`): sixteen dynamic vertex
    /// buffers of 0x28000 bytes (`+0`..`+0x3c`), a vertex shader
    /// (`+0x4c`) and two pixel shaders (`+0x50`, `+0x54`) from the
    /// image's blobs, an input layout of three elements (`+0x44`), an
    /// 80-byte constant buffer (`+0x48`, named). Returns 0, 9 when a
    /// buffer, shader or layout fails, 5 when the constant buffer does.
    /// `frame` is the original's `ebp - 0x90`.
    pub fn init_a(h: &mut impl Host, this: u32, renderer: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x90 - off;
        h.write(local(0x90), renderer);
        let device: u32 = h.read(renderer + 0x6c);
        h.write(local(0x8c), this);
        let desc = local(0x88);
        for (k, w) in [0x28000u32, 2, 1, 0x10000, 0, 0].into_iter().enumerate() {
            h.write(desc + 4 * k as u32, w);
        }
        for i in 0..16 {
            if (com(h, desc, device, 3, &[desc, 0, this + 4 * i]) as i32) < 0 {
                return FAILED;
            }
        }
        let this: u32 = h.read(local(0x8c));
        let renderer: u32 = h.read(local(0x90));
        h.write(this + 0x40, 0u32);
        for (slot, blob, out) in [
            (12, VS_A, this + 0x4c),
            (15, PS_A, this + 0x50),
            (15, PS_B, this + 0x54),
        ] {
            let device: u32 = h.read(renderer + 0x6c);
            let vt: u32 = h.read(device);
            if (create_shader(h, vt, device, slot, blob, out) as i32) < 0 || h.read::<u32>(out) == 0
            {
                return FAILED;
            }
        }
        let blob = h.rebase(VS_A);
        let len: u32 = h.read(blob);
        let device: u32 = h.read(renderer + 0x6c);
        let elements = local(0x58);
        input_element(h, elements, POSITION, 0, 2, 0);
        input_element(h, elements + 0x1c, TEXCOORD, 0, 2, 1);
        input_element(h, elements + 0x38, TEXCOORD, 1, 0x10, 2);
        let out = this + 0x44;
        let vt: u32 = h.read(device);
        let r = com(
            h,
            vt,
            device,
            11,
            &[elements, 3, blob + 4, len.wrapping_add(0xffff_fffe), out],
        );
        if (r as i32) < 0 || h.read::<u32>(out) == 0 {
            return FAILED;
        }
        let device: u32 = h.read(renderer + 0x6c);
        let desc = local(0x70);
        for (k, w) in [0x50u32, 0, 4, 0, 0, 0].into_iter().enumerate() {
            h.write(desc + 4 * k as u32, w);
        }
        let out = this + 0x48;
        if (com(h, device, device, 3, &[desc, 0, out]) as i32) < 0 {
            return 5;
        }
        let buffer: u32 = h.read(out);
        if buffer == 0 {
            return 5;
        }
        let guid = h.rebase(NAME_GUID);
        let name = h.rebase(NAME_A);
        com(h, buffer, buffer, 5, &[guid, 0x17, name]);
        0
    }

    /// The bytes of `initB`'s frame below the cookie (`ebp - 0x60` up to
    /// `ebp - 4`).
    pub const INIT_B_FRAME: u32 = 0x5c;

    /// `PRendererD3D11::initB(this; renderer)` (`0x00595200`, thiscall,
    /// ret 4; `this` = renderer `+0x6ec`): a vertex shader (`+8`) and
    /// eight pixel shaders (`+0xc`..`+0x28`, all created before any is
    /// checked), an input layout of no elements (`+0`; the original
    /// passes a count of 0), a 32-byte dynamic constant buffer (`+4`,
    /// named), then eight 0x17c-byte state records from `+0x2c` (byte
    /// `+0x84` 0 or 0xf by bit 0, word `+0x5c` bit 1) each finished by
    /// `0x0058d4e0`. Returns 0, 9 when a shader or the layout fails, 5
    /// when the buffer does, or the record call's result. `frame` is the
    /// original's `ebp - 0x60`.
    pub fn init_b(h: &mut impl Host, this: u32, renderer: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x60 - off;
        let device: u32 = h.read(renderer + 0x6c);
        h.write(local(0x44), this);
        let r = create_shader(h, this, device, 12, VS_B, this + 8);
        if (r as i32) < 0 || h.read::<u32>(this + 8) == 0 {
            return FAILED;
        }
        for (k, blob) in (0u32..).zip(PS_B_BLOBS) {
            let vt: u32 = h.read(device);
            let r = create_shader(h, vt, device, 15, blob, this + 0xc + 4 * k);
            h.write(local(0x24 - 4 * k), r);
        }
        for k in 0..8 {
            if h.read::<i32>(local(0x24 - 4 * k)) < 0 || h.read::<u32>(this + 0xc + 4 * k) == 0 {
                return FAILED;
            }
        }
        let blob = h.rebase(VS_B);
        let len: u32 = h.read(blob);
        let element = local(0x60);
        input_element(h, element, POSITION, 0, 2, 0);
        let vt: u32 = h.read(device);
        let r = com(
            h,
            vt,
            device,
            11,
            &[element, 0, blob + 4, len.wrapping_add(0xffff_fffe), this],
        );
        if (r as i32) < 0 || h.read::<u32>(this) == 0 {
            return FAILED;
        }
        let desc = local(0x3c);
        for (k, w) in [0x20u32, 2, 4, 0x10000, 0, 0].into_iter().enumerate() {
            h.write(desc + 4 * k as u32, w);
        }
        let out = this + 4;
        if (com(h, desc, device, 3, &[desc, 0, out]) as i32) < 0 {
            return 5;
        }
        let buffer: u32 = h.read(out);
        if buffer == 0 {
            return 5;
        }
        let guid = h.rebase(NAME_GUID);
        let name = h.rebase(NAME_B);
        com(h, buffer, buffer, 5, &[guid, 0x17, name]);
        let this: u32 = h.read(local(0x44));
        for k in 0..8u32 {
            let rec = this + 0x60 + 0x17c * k;
            h.write(rec + 0x50, if k & 1 != 0 { 0xfu8 } else { 0 });
            h.write(rec - 8, 1u32);
            h.write(rec, 8u32);
            h.write(rec - 4, (k >> 1) & 1);
            h.write(rec + 4, 0u32);
            h.write(rec + 8, 0xffffu16);
            h.write(rec + 0x18, 8u32);
            h.write(rec + 0xc, 1u32);
            h.write(rec + 0x10, 1u32);
            h.write(rec + 0x14, 1u32);
            h.write(rec + 0x28, 8u32);
            h.write(rec + 0x1c, 1u32);
            h.write(rec + 0x20, 1u32);
            h.write(rec + 0x24, 1u32);
            let r = h.call_sym(FINISH_RECORD, Conv::Thiscall(rec - 0x34), &[]);
            if r != 0 {
                return r;
            }
        }
        0
    }
}
