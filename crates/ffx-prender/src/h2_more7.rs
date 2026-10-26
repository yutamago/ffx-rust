//! h2-render-b round 4, batch 3: the mesh-instance destructor, the
//! vertex/index buffers' release and staging unmap, the input-assembler
//! vertex-buffer bind.
//!
//! COM methods stay calls through the object's vtable (`stdcall`, the
//! object first), `ecx` as the original leaves it.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// `PhyrePrintf(level, format, ...)`.
const PRINTF: u32 = 0x0043_52f0;
/// The scoped lock's constructor and destructor (thiscall on the lock).
const LOCK: u32 = 0x0046_3d50;
const UNLOCK: u32 = 0x0046_3e90;
/// The render-thread device holder's guard word and the holder.
const HOLDER_GUARD: u32 = 0x00c9_628c;
const HOLDER: u32 = 0x00c9_4f00;
/// The holder's device context (`HOLDER + 0x70`).
const HOLDER_CONTEXT: u32 = 0x00c9_4f70;
/// `PSharray` element 20-byte getter `(array; i)` (thiscall).
const ELEMENT20: u32 = 0x0055_e0f0;

/// The storage of the `PSharray` at `array` (count word read now).
fn storage(h: &mut impl Host, array: u32) -> u32 {
    match h.read::<u32>(array) & COUNT {
        0 => 0,
        1 => array + 4,
        _ => h.read(array + 4),
    }
}

/// A COM method: `slot` (word index) of `obj`'s vtable, `stdcall` with
/// `obj` first, `ecx` as the original leaves it.
fn com(h: &mut impl Host, ecx: u32, obj: u32, slot: u32, args: &[u32]) -> u32 {
    let vt: u32 = h.read(obj);
    let f: u32 = h.read(vt + 4 * slot);
    let mut words = Vec::with_capacity(args.len() + 1);
    words.push(obj);
    words.extend_from_slice(args);
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 7 (six arguments at most)
}

/// The holder of the render thread's device, built on first use (the
/// guard's bit 0; its destructor registered with `atexit`).
fn holder(h: &mut impl Host) {
    let g = h.rebase(HOLDER_GUARD);
    let v: u32 = h.read(g);
    if v & 1 == 0 {
        h.write(g, v | 1);
        let holder = h.rebase(HOLDER);
        h.call_sym(0x004f_ee70, Conv::Thiscall(holder), &[]);
        let dtor = h.rebase(0x00af_b710);
        h.call_sym(0x0094_9207, Conv::Cdecl, &[dtor]);
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BORROWED, COUNT, Conv, ELEMENT20, HOLDER_CONTEXT, Host, LOCK, PHYRE_FREE, PRINTF, UNLOCK,
        com, holder, storage, x87,
    };

    /// The array of owned constant buffers at `this + off` (count, data):
    /// each taken out (nulled), destroyed (`PConstantBuffer::~PConstantBuffer`)
    /// and freed.
    fn delete_buffers(h: &mut impl Host, this: u32, off: u32) {
        let mut i = 0u32;
        while i < h.read::<u32>(this + off) & COUNT {
            let data: u32 = h.read(this + off + 4);
            let at = data.wrapping_add(4 * i);
            let p: u32 = h.read(at);
            h.write(at, 0u32);
            if p != 0 {
                h.call_sym(0x0049_fa10, Conv::Thiscall(p), &[]);
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
            i += 1;
        }
    }

    /// The owned array at `p + off` freed (no null check) and cleared.
    fn free_owned(h: &mut impl Host, p: u32, off: u32) {
        if h.read::<u32>(p + off) & BORROWED == 0 {
            let d: u32 = h.read(p + off + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
        }
        h.write(p + off + 4, 0u32);
        h.write(p + off, 0u32);
    }

    /// `PMeshInstance::~PMeshInstance(this)`: the constant buffers of the
    /// arrays at `+0x70` and `+0x78` deleted, both arrays freed, the name
    /// at `+0x2c` freed unless static, the segments at `+0x24`
    /// (`0x004f4fc0(data, n)`) and the matrices at `+8` released.
    pub fn mesh_instance_dtor(h: &mut impl Host, this: u32) {
        delete_buffers(h, this, 0x70);
        delete_buffers(h, this, 0x78);
        free_owned(h, this, 0x78);
        free_owned(h, this, 0x70);
        let name: u32 = h.read(this + 0x2c);
        if name & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
        }
        let n: u32 = h.read(this + 0x24);
        if n & BORROWED == 0 {
            let d: u32 = h.read(this + 0x28);
            h.call_sym(0x004f_4fc0, Conv::Cdecl, &[d, n & COUNT]);
        }
        h.write(this + 0x28, 0u32);
        h.write(this + 0x24, 0u32);
        if h.read::<u32>(this + 8) & BORROWED == 0 {
            let d: u32 = h.read(this + 0xc);
            if d != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
            }
        }
        h.write(this + 0xc, 0u32);
        h.write(this + 8, 0u32);
    }

    /// `0x005646d0(this)` (vertex) / `0x005648c0(this)` (index): unless
    /// created (bit 0 of `this + flag`) a warning (`msg`) and 0; else the
    /// spin wait (`spin(this)`), then under the scoped lock (the frame
    /// word, `this` from the prologue's `push ecx`) the buffers of each 20-byte entry of the `PSharray` at
    /// `this + array` (words 1, 0 and 3) released and nulled, the flag's
    /// bit 0 cleared. Returns 0.
    pub fn release_streams(
        h: &mut impl Host,
        this: u32,
        lock: u32,
        layout: (u32, u32, u32, u32),
    ) -> u32 {
        let (spin, array, flag, msg) = layout;
        // the lock's slot is the `push ecx` of the prologue: it starts as `this`
        h.write(lock, this);
        if h.read::<u8>(this + flag) & 1 == 0 {
            let msg = h.rebase(msg);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
            return 0;
        }
        h.call_sym(spin, Conv::Thiscall(this), &[]);
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        let n = h.read::<u32>(this + array) & COUNT;
        for k in 0..n {
            for word in [4, 0, 0xc] {
                let at = 20 * k + word;
                let s = storage(h, this + array);
                let obj: u32 = h.read(s.wrapping_add(at));
                if obj != 0 {
                    let vt: u32 = h.read(obj);
                    com(h, vt, obj, 2, &[]);
                    let s = storage(h, this + array);
                    h.write(s.wrapping_add(at), 0u32);
                }
            }
        }
        let f: u32 = h.read(this + flag);
        h.write(this + flag, f & !1);
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        0
    }

    /// `0x00564cb0(this; i, _)`: entry `i` (20 bytes) of the `PSharray`
    /// at `+0x18`: when it holds a buffer, under the scoped lock its
    /// `GetDesc`; without CPU write access and with a staging copy (word
    /// 3) the copy is unmapped and copied into the buffer
    /// (`CopyResource`), else the buffer is unmapped. Returns 0. `frame`:
    /// the original's `ebp - 0x34` (the lock, the array, `i`, the
    /// description).
    pub fn unmap_copy(h: &mut impl Host, this: u32, i: u32, frame: u32) -> u32 {
        let arr = this + 0x18;
        h.write(frame + 8, i);
        h.write(frame + 4, arr);
        let at = i.wrapping_mul(20);
        let s = storage(h, arr);
        if h.read::<u32>(s.wrapping_add(at)) == 0 {
            return 0;
        }
        h.call_sym(LOCK, Conv::Thiscall(frame), &[]);
        holder(h);
        let s = storage(h, arr);
        let buf: u32 = h.read(s.wrapping_add(at));
        let vt: u32 = h.read(buf);
        com(h, vt, buf, 10, &[frame + 0xc]);
        let s = storage(h, arr);
        if h.read::<u32>(frame + 0x18) & 0x1_0000 == 0
            && h.read::<u32>(s.wrapping_add(at + 0xc)) != 0
        {
            let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let e = h.call_sym(ELEMENT20, Conv::Thiscall(arr), &[i]);
            let staging: u32 = h.read(e.wrapping_add(0xc));
            com(h, arr, ctx, 15, &[staging, 0]);
            let dst = h.call_sym(ELEMENT20, Conv::Thiscall(arr), &[i]);
            let src = h.call_sym(ELEMENT20, Conv::Thiscall(arr), &[i]);
            let staging: u32 = h.read(src.wrapping_add(0xc));
            let target: u32 = h.read(dst);
            com(h, staging, ctx, 47, &[target, staging]);
        } else {
            let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let s = storage(h, arr);
            let b: u32 = h.read(s.wrapping_add(at));
            com(h, ctx, ctx, 15, &[b, 0]);
        }
        h.call_sym(UNLOCK, Conv::Thiscall(frame), &[]);
        0
    }

    /// `0x0059b140(this; &device, program)`: `al = 0` without the input
    /// layout `+0x4e8`; `al = 1` when the program has no streams
    /// (`+0x2c`) or no layout (`+0x14`). Else for each of the layout's
    /// streams (half-word offsets into the streams) a bound data block
    /// (`0x00562da0`, else `al = 0`): its buffer (`0x00593af0(block;
    /// [stream + 8])`), stride (`[block]`) and offset (`[[stream + 4]] +
    /// [block + 0x30]`) into the frame's arrays of 32 (the rest
    /// cleared); `IASetVertexBuffers(0, 16 at feature level 0xa000 else
    /// 32, ...)` and `IASetInputLayout`. `al = 1`. `frame`: the
    /// original's `ebp - 0x194` (buffers; the count, device, streams and
    /// `this` at `+0x80..+0x8c`; offsets at `+0x90`, strides at
    /// `+0x110`).
    pub fn bind_vertex_streams(
        h: &mut impl Host,
        this: u32,
        dev_ptr: u32,
        program: u32,
        frame: u32,
    ) -> u8 {
        let (buffers, offsets, strides) = (frame, frame + 0x90, frame + 0x110);
        h.write(frame + 0x8c, this);
        if h.read::<u32>(this + 0x4e8) == 0 {
            return 0;
        }
        let base: u32 = h.read(program + 0x2c);
        let layout: u32 = h.read(program + 0x14);
        h.write(frame + 0x88, base);
        if base == 0 || layout == 0 {
            return 1;
        }
        let mut offs: u32 = h.read(layout + 0x14);
        let n = h.read::<u32>(layout + 0x10) & COUNT;
        let dev: u32 = h.read(dev_ptr);
        h.write(frame + 0x84, dev);
        h.write(frame + 0x80, n);
        for i in 0..n {
            let stream = base.wrapping_add(u32::from(h.read::<u16>(offs)));
            let block: u32 = h.read(stream);
            if h.call_sym(0x0056_2da0, Conv::Thiscall(block), &[]) as u8 == 0 {
                return 0;
            }
            let block: u32 = h.read(stream);
            let w: u32 = h.read(stream + 8);
            let b = h.call_sym(0x0059_3af0, Conv::Thiscall(block), &[w]);
            h.write(buffers.wrapping_add(4 * i), b);
            let block: u32 = h.read(stream);
            let stride: u32 = h.read(block);
            h.write(strides.wrapping_add(4 * i), stride);
            let p: u32 = h.read(stream + 4);
            let block: u32 = h.read(stream);
            let off = h.read::<u32>(p).wrapping_add(h.read(block + 0x30));
            h.write(offsets.wrapping_add(4 * i), off);
            offs = offs.wrapping_add(2);
        }
        for i in n..32 {
            h.write(offsets + 4 * i, 0u32);
            h.write(strides + 4 * i, 0u32);
            h.write(buffers + 4 * i, 0u32);
        }
        let ctx: u32 = h.read(dev + 0x70);
        let count = if h.read::<u32>(dev + 0xb4) == 0xa000 {
            0x10
        } else {
            0x20
        };
        com(h, ctx, ctx, 18, &[0, count, buffers, strides, offsets]);
        let ctx: u32 = h.read(dev + 0x70);
        let il: u32 = h.read(this + 0x4e8);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 17, &[il]);
        1
    }

    /// A float copied through the x87 (`fld`/`fstp`: a signalling NaN
    /// comes out quiet).
    fn fcopy(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    /// `0x0058e200(this; device, flags, colour, depth, stencil, target)`:
    /// the full-screen clear by drawing. Nothing for no flags. Colour
    /// (bits 0/1): the four colour floats and four copies of the depth
    /// into the frame and copied (32 bytes) into the constant buffer
    /// `+4` mapped write-discard. Stencil (bit 3): the depth-stencil
    /// state of slot `flags & 7` (0x17c bytes each) set to write the
    /// stencil value, its views released (`0x0059c950`) and rebuilt
    /// (`0x0058d4e0`). Then that slot's pass state bound
    /// (`PShaderPassStateD3D11_bind`), the compute and geometry stages
    /// cleared, the clear shaders (`+0` layout, `+8` vertex, pixel
    /// `[+8 + 4 target]` or `+0xc`) and constant buffer set, no vertex
    /// buffers, triangle lists, `Draw(3, 0)`. Returns 0. `frame`: the
    /// original's `ebp - 0x30` (the constants, the map, `this` at `+0x2c`).
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn clear_by_draw(
        h: &mut impl Host,
        this: u32,
        dev: u32,
        flags: u32,
        colour: u32,
        depth: u32,
        stencil: u32,
        target: u32,
        frame: u32,
    ) -> u32 {
        h.write(frame + 0x2c, this);
        if flags == 0 {
            return 0;
        }
        if flags & 3 != 0 {
            let ctx: u32 = h.read(dev + 0x70);
            for k in 0..4 {
                let c: u32 = h.read(colour + 4 * k);
                h.write(frame + 4 * k, fcopy(c));
            }
            let d = fcopy(depth);
            for k in 4..8 {
                h.write(frame + 4 * k, d);
            }
            for k in 8..11 {
                h.write(frame + 4 * k, 0u32);
            }
            let cb: u32 = h.read(this + 4);
            com(h, ctx, ctx, 14, &[cb, 0, 4, 0, frame + 0x20]);
            let p: u32 = h.read(frame + 0x20);
            let mut block = [0u8; 0x20];
            h.read_bytes(frame, &mut block);
            h.write_bytes(p, &block);
            let ctx: u32 = h.read(dev + 0x70);
            let vt: u32 = h.read(ctx);
            let cb: u32 = h.read(this + 4);
            com(h, vt, ctx, 15, &[cb, 0]);
        }
        let slot = this.wrapping_add((flags & 7) * 0x17c);
        if flags & 8 != 0 {
            h.write(slot + 0x1a4, stencil as u8);
            h.write(slot + 0x64, 1u32);
            h.write(slot + 0x68, 0xffffu16);
            h.write(slot + 0x78, 8u32);
            for off in [0x6c, 0x70, 0x74] {
                h.write(slot + off, 3u32);
            }
            h.write(slot + 0x88, 8u32);
            for off in [0x7c, 0x80, 0x84] {
                h.write(slot + off, 3u32);
            }
            h.call_sym(0x0059_c950, Conv::Thiscall(slot + 0x2c), &[]);
            h.call_sym(0x0058_d4e0, Conv::Thiscall(slot + 0x2c), &[]);
        }
        h.call_sym(0x0058_c6c0, Conv::Thiscall(slot + 0x2c), &[dev]);
        let ctx: u32 = h.read(dev + 0x70);
        let ps: u32 = if target != 0 {
            h.read(this.wrapping_add(target.wrapping_mul(4)).wrapping_add(8))
        } else {
            h.read(this + 0xc)
        };
        let layout: u32 = h.read(this);
        let vs: u32 = h.read(this + 8);
        let vt: u32 = h.read(ctx);
        com(h, vt, ctx, 69, &[0, 0, 0]);
        com(h, vt, ctx, 23, &[0, 0, 0]);
        com(h, vt, ctx, 17, &[layout]);
        com(h, vt, ctx, 11, &[vs, 0, 0]);
        com(h, vt, ctx, 9, &[ps, 0, 0]);
        com(h, ctx, ctx, 7, &[0, 1, this + 4]);
        com(h, vt, ctx, 16, &[0, 1, this + 4]);
        com(h, vt, ctx, 18, &[0, 0, 0, 0, 0]);
        com(h, vt, ctx, 24, &[4]);
        com(h, vt, ctx, 13, &[3, 0]);
        0
    }

    /// `0x0058e3d0(this; flags)`: the renderer's clear. Nothing for no
    /// flags. With the full viewport (`+0x132c/+0x1330` equal to
    /// `+0x1334/+0x1338`, no offset at `+0x1324/+0x1328`): bit 0 clears
    /// each bound render target (`+0x2b8`, `+0x2fc` of them) to the
    /// colour `+0x37c`, bits 1/3 the depth-stencil view `+0x2d8` (depth
    /// `+0x38c`, stencil byte `+0x390`). Else the clear by drawing
    /// (`0x0058e200(this + 0x6ec; ...)`, the colour in the frame).
    /// `al = 1`. `frame`: the original's `ebp - 0x10`.
    pub fn clear(h: &mut impl Host, this: u32, flags: u32, frame: u32) -> u8 {
        if flags == 0 {
            return 1;
        }
        let full = h.read::<u32>(this + 0x132c) == h.read::<u32>(this + 0x1334)
            && h.read::<u32>(this + 0x1330) == h.read::<u32>(this + 0x1338)
            && h.read::<u32>(this + 0x1324) == 0
            && h.read::<u32>(this + 0x1328) == 0;
        if !full {
            for k in 0..4 {
                let c: u32 = h.read(this + 0x37c + 4 * k);
                h.write(frame + 4 * k, fcopy(c));
            }
            let n: u32 = h.read(this + 0x2fc);
            let stencil = u32::from(h.read::<u8>(this + 0x390));
            let depth = fcopy(h.read(this + 0x38c));
            h.call_sym(
                0x0058_e200,
                Conv::Thiscall(this + 0x6ec),
                &[this, flags, frame, depth, stencil, n],
            );
            return 1;
        }
        if flags & 1 != 0 {
            let mut i = 0u32;
            while i < h.read::<u32>(this + 0x2fc) {
                let rt: u32 = h.read(this + 0x2b8 + 4 * i);
                if rt != 0 {
                    let ctx: u32 = h.read(this + 0x70);
                    com(h, ctx, ctx, 50, &[rt, this + 0x37c]);
                }
                i += 1;
            }
        }
        let dsv: u32 = h.read(this + 0x2d8);
        if dsv != 0 && flags & 0xa != 0 {
            let stencil = u32::from(h.read::<u8>(this + 0x390));
            let depth = fcopy(h.read(this + 0x38c));
            let ctx: u32 = h.read(this + 0x70);
            let which = ((flags >> 1) & 4 | flags & 2) >> 1;
            com(h, ctx, ctx, 53, &[dsv, which, depth, stencil]);
        }
        1
    }
}
