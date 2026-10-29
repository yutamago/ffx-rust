//! M5 stage 1, list `phyre-render-b`: three D3D11-facing methods: the
//! morph manager's 16-byte record list insert, a dynamic buffer's
//! upload through `Map`/`Unmap`, and a full-screen draw's state set-up
//! (the vertex buffers, strides and offsets in its frame).

/// `Phyre_PhyreMalloc_0042fb80(size, align)`, `Phyre_PhyreFree` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The render scope lock (thiscall on a frame word), the device holder
/// made on first use, its context word (`+0x70`), `0x00585b50(array; i)`
/// (thiscall, element `i`), `crt_memcpy_thunk` (cdecl).
pub const SCOPE_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LEAVE: u32 = 0x0046_3e90;
pub const HOLDER_GUARD: u32 = 0x00c9_628c;
pub const HOLDER: u32 = 0x00c9_4f00;
pub const HOLDER_CTOR: u32 = 0x004f_ee70;
pub const HOLDER_DTOR: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const HOLDER_CONTEXT: u32 = 0x00c9_4f70;
pub const ELEMENT: u32 = 0x0058_5b50;
pub const MEMCPY: u32 = 0x0094_9256;
/// `PRendererD3D11_mapPrimitiveTopology(p)` (cdecl),
/// `PShaderPassStateD3D11_bind(state; renderer)` (thiscall).
pub const MAP_TOPOLOGY: u32 = 0x0058_ab60;
pub const PASS_STATE_BIND: u32 = 0x0058_c6c0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// COM method `off` of `obj` with `args` after it.
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + off);
        let mut words = [0u32; 8];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()]) // call width: 7
    }

    /// `0x006ab950(list, key, a, b, c)` (stdcall): nothing when a record
    /// with `key` exists; else `{key, a, b, c}` in the first record whose
    /// key is 0, or appended to a grown buffer (`PhyreMalloc`, the old one
    /// freed when owned; 0xd when it fails); 0.
    pub fn insert(h: &mut impl Host, list: u32, rec: [u32; 4]) -> u32 {
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let data: u32 = h.read(list + 4);
        for i in 0..n {
            if h.read::<u32>(data.wrapping_add(16 * i)) == rec[0] {
                return 0;
            }
        }
        for i in 0..n {
            let at = data.wrapping_add(16 * i);
            if h.read::<u32>(at) == 0 {
                h.write(at, rec);
                return 0;
            }
        }
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_add(1) << 4, 4]);
        if buf == 0 {
            return 0xd;
        }
        let n = h.read::<u32>(list) & 0x7fff_ffff;
        let mut src: u32 = h.read(list + 4);
        for i in 0..n {
            let d = buf.wrapping_add(16 * i);
            if d != 0 {
                let v: [u32; 4] = h.read(src);
                h.write(d, v);
                src = src.wrapping_add(16);
            }
        }
        let last = buf.wrapping_add(n << 4);
        if last != 0 {
            h.write(last, rec);
        }
        let old: u32 = h.read(list + 4);
        let word: u32 = h.read(list);
        if old != buf && word & 0x8000_0000 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(list + 4, buf);
        h.write(list, n.wrapping_add(1));
        0
    }

    /// `0x005937a0(buffer; i)`: element `i` of the `PSharray` at `+0xc`
    /// (16 bytes) with pending data (`+8`) uploaded under the render scope
    /// lock (`ebp - 0x14`): the context (`ebp - 0x10`) maps its resource
    /// (`WRITE_DISCARD` (4) for a dynamic buffer, `+0x24` == 2, else
    /// `WRITE` (2)) into the zeroed mapping at `ebp - 0x20`, the
    /// `[+0x20]` bytes at `+4` copied in, unmapped, `+8` cleared; 9 when
    /// the map fails or gives no pointer, else 0.
    pub fn upload(h: &mut impl Host, ebp: u32, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
        let data = sharray(h, this + 0xc, count);
        if h.read::<u32>(data.wrapping_add(16 * i).wrapping_add(8)) == 0 {
            return 0;
        }
        let lock = ebp - 0x14;
        this_call(h, SCOPE_ENTER, lock, &[]);
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            this_call(h, HOLDER_CTOR, holder, &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
        h.write(ebp - 0x10, ctx);
        let mapped = ebp - 0x20;
        h.write(mapped, [0u32; 3]);
        let kind = if h.read::<u32>(this + 0x24) == 2 {
            4
        } else {
            2
        };
        let count = h.read::<u32>(this + 0xc) & 0x7fff_ffff;
        let data = sharray(h, this + 0xc, count);
        let res: u32 = h.read(data.wrapping_add(16 * i));
        let hr = com(h, ctx, 0x38, &[res, 0, kind, 0, mapped]);
        let dst: u32 = h.read(mapped);
        if (hr as i32) < 0 || dst == 0 {
            this_call(h, SCOPE_LEAVE, lock, &[]);
            return 9;
        }
        let arr = this + 0xc;
        let e = this_call(h, ELEMENT, arr, &[i]);
        let src: u32 = h.read(e + 4);
        let size: u32 = h.read(this + 0x20);
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, size]);
        let e = this_call(h, ELEMENT, arr, &[i]);
        let res: u32 = h.read(e);
        com(h, ctx, 0x3c, &[res, 0]);
        let e = this_call(h, ELEMENT, arr, &[i]);
        h.write(e + 8, 0u32);
        this_call(h, SCOPE_LEAVE, lock, &[]);
        0
    }

    fn sharray(h: &mut impl Host, at: u32, count: u32) -> u32 {
        if count > 1 {
            h.read(at + 4)
        } else if count != 0 {
            at + 4
        } else {
            0
        }
    }

    /// `0x00592e90(quad; renderer, primitive, slot, n)`: 5 for a primitive
    /// with no topology; else the pass state bound and on the renderer's
    /// context: hull/domain shaders cleared (`+0x114`, `+0x5c`), the input
    /// layout (`+0x44`), the vertex shader (`+0x2c`), the pixel shader
    /// (`+0x24`, `+0x54` when `+0x58` is set, else `+0x50`), the
    /// constant buffer `+0x48` to both stages (`+0x1c`, `+0x40`), the
    /// sampler and resource (`+0x20`, `+0x28`), the three vertex buffers
    /// of `slot` with their strides and offsets in the frame (`+0x48`),
    /// the topology (`+0x60`), the draw of `n` (`+0x34`); 0.
    pub fn draw_setup(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        r: u32,
        primitive: u32,
        slot: u32,
        n: u32,
    ) -> u32 {
        h.write(ebp - 0x20, slot);
        let topology = h.call_sym(MAP_TOPOLOGY, Conv::Cdecl, &[primitive]);
        h.write(ebp - 0x24, topology);
        if topology == 0 {
            return 5;
        }
        this_call(h, PASS_STATE_BIND, this + 0x60, &[r]);
        let ctx = |h: &mut _| -> u32 { Mem::read(h, r + 0x70) };
        let c = ctx(h);
        com(h, c, 0x114, &[0, 0, 0]);
        let c = ctx(h);
        com(h, c, 0x5c, &[0, 0, 0]);
        let c = ctx(h);
        let layout: u32 = h.read(this + 0x44);
        com(h, c, 0x44, &[layout]);
        let c = ctx(h);
        let vs: u32 = h.read(this + 0x4c);
        com(h, c, 0x2c, &[vs, 0, 0]);
        let ps: u32 = if h.read::<u32>(this + 0x58) != 0 {
            h.read(this + 0x54)
        } else {
            h.read(this + 0x50)
        };
        let c = ctx(h);
        com(h, c, 0x24, &[ps, 0, 0]);
        for (off, at) in [(0x1c, 0x48), (0x40, 0x48), (0x20, 0x58), (0x28, 0x5c)] {
            let c = ctx(h);
            com(h, c, off, &[0, 1, this + at]);
        }
        let vb: u32 = h.read(this.wrapping_add(slot.wrapping_mul(4)));
        let (bufs, strides, offsets) = (ebp - 0x30, ebp - 0x1c, ebp - 0x10);
        h.write(bufs, [vb; 3]);
        h.write(strides, [0x28u32; 3]);
        h.write(offsets, [0u32, 0x10, 0x20]);
        let c = ctx(h);
        com(h, c, 0x48, &[0, 3, bufs, strides, offsets]);
        let c = ctx(h);
        com(h, c, 0x60, &[topology]);
        let c = ctx(h);
        com(h, c, 0x34, &[n, 0]);
        0
    }
}
