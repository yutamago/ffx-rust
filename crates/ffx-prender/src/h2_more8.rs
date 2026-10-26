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
        CLEAR_32, COLOR_FORMAT, CREATE_BUFFERS, CREATE_DEVICE, CREATE_DEVICE_AND_SWAP_CHAIN,
        CREATE_DYNAMIC_BUFFERS, CREATE_VIEWS, Conv, DEPTH_FORMAT, FAILED, Host, IID_A, IID_B,
        IID_BACK_BUFFER, IID_FACTORY, INIT_A, INIT_B, LOCK_RELEASE, MALLOC_ALIGNED, MEMSET,
        MUTEX_LOCK, PHYRE_FREE, READER_CLOSE, READER_CTOR, READER_DTOR, REGISTER_TEXTURE,
        RELEASE_VIEWS, RELOAD_TEXTURES, SET_DIMENSIONS, SET_RANGE, SET_TEXTURE, SET_VIEWPORT,
        SET_VIEWPORT_RECT, SET_WORD_AT, SETTINGS, SOURCE_CTOR, TREE_NEXT, UPDATE_MAX_LEVEL, com,
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
}
