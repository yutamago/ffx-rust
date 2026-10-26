//! h2-render-b round 4, batch 2: the mesh-instance constructor and its
//! array setup, the D3D11 shader and buffer constructors, vertex/index
//! buffer and shader-resource-view creation on the render thread's
//! device, the constant-buffer unmap, a parameter-slot fix-up, a
//! destructor and a `Bind()` table.
//!
//! COM methods stay calls through the object's vtable (`stdcall`, the
//! object first), `ecx` as the original leaves it.

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// `memset` (cdecl `(dst, byte, n)`).
const MEMSET: u32 = 0x0094_964c;
/// `PhyreMalloc(size, align)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
/// `PhyrePrintf(level, format, ...)`.
const PRINTF: u32 = 0x0043_52f0;
/// The scoped lock's constructor and destructor (thiscall on the lock).
const LOCK: u32 = 0x0046_3d50;
const UNLOCK: u32 = 0x0046_3e90;
/// The render-thread device holder's guard word and the holder.
const HOLDER_GUARD: u32 = 0x00c9_628c;
const HOLDER: u32 = 0x00c9_4f00;
/// The holder's device (`HOLDER + 0x6c`) and device context (`+ 0x70`).
const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
const HOLDER_CONTEXT: u32 = 0x00c9_4f70;

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
    h.call(f, Conv::Thiscall(ecx), &words) // call width: 4 (three arguments at most)
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

/// Writes the six words of a `D3D11_BUFFER_DESC` at `at`.
fn desc(h: &mut impl Host, at: u32, words: [u32; 6]) {
    for (i, w) in words.into_iter().enumerate() {
        h.write(at + 4 * i as u32, w);
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BORROWED, COUNT, Conv, HOLDER_CONTEXT, HOLDER_DEVICE, Host, LOCK, MALLOC, MEMSET,
        PHYRE_FREE, PRINTF, UNLOCK, com, desc, holder, storage,
    };

    /// `0x004f27e0(this; mesh)`: the mesh instance's words (the mesh at
    /// `+0`, `+0x10 = mesh + 0x30`, `+0x80 = -1`, the rest of
    /// `+4..+0x7c` zero, `fldz` stores at `+0x64`/`+0x68`), then
    /// `0x004f8b90(this; mesh)`; returns `this`.
    pub fn mesh_instance_ctor(h: &mut impl Host, this: u32, mesh: u32) -> u32 {
        h.write(this, mesh);
        for off in (4..=0x50).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x10, mesh.wrapping_add(0x30));
        for off in (0x64..=0x7c).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x80, u32::MAX);
        h.call_sym(0x004f_8b90, Conv::Thiscall(this), &[mesh]);
        this
    }

    /// `0x004f8b90(this; mesh)`: the instance's two arrays sized to the
    /// mesh's. The 0x24-byte segments at `+0x24` (`mesh +0` count) built
    /// by `0x004f6380(data, n)` in new storage, the old ones destroyed
    /// (`0x004f4fc0(data, n)`) when owned; the 0x40-byte matrices at
    /// `+8` (`mesh +0x18` count), the old storage freed when owned; each
    /// matrix copied from the mesh's (`mesh +0x1c`) by `0x00453340`.
    /// The mesh at `+0`, `+0x10 = mesh + 0x30`; 0, or 0xd when an
    /// allocation fails.
    pub fn size_arrays(h: &mut impl Host, this: u32, mesh: u32) -> u32 {
        let n = h.read::<u32>(mesh) & COUNT;
        let cur: u32 = h.read(this + 0x24);
        if n != cur & COUNT && n != cur {
            let mut data = 0;
            if n != 0 {
                data = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(36), 4]);
                if data == 0 {
                    return 0xd;
                }
                h.call_sym(0x004f_6380, Conv::Cdecl, &[data, n]);
            }
            let old: u32 = h.read(this + 0x28);
            let w: u32 = h.read(this + 0x24);
            if old != data && w & BORROWED == 0 {
                h.call_sym(0x004f_4fc0, Conv::Cdecl, &[old, w & COUNT]);
            }
            h.write(this + 0x28, data);
            h.write(this + 0x24, n);
        }
        let m = h.read::<u32>(mesh + 0x18) & COUNT;
        let cur: u32 = h.read(this + 8);
        if m != cur & COUNT && m != cur {
            let mut data = 0;
            if m != 0 {
                data = h.call_sym(MALLOC, Conv::Cdecl, &[m << 6, 4]);
                if data == 0 {
                    return 0xd;
                }
            }
            let old: u32 = h.read(this + 0xc);
            let w: u32 = h.read(this + 8);
            if old != data && w & BORROWED == 0 && old != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(this + 0xc, data);
            h.write(this + 8, m);
        }
        let dst: u32 = h.read(this + 0xc);
        let src: u32 = h.read(mesh + 0x1c);
        if dst != 0 && src != 0 {
            for k in 0..m {
                let off = k << 6;
                h.call_sym(
                    0x0045_3340,
                    Conv::Thiscall(dst.wrapping_add(off)),
                    &[src.wrapping_add(off)],
                );
            }
        }
        h.write(this, mesh);
        h.write(this + 0x10, mesh.wrapping_add(0x30));
        0
    }

    /// `0x005954f0(this; device, size, size2)`: the dynamic buffer pairs
    /// (`this + 0/4` of `size`, 0x100000 for 0, vertex; `this + 8/0xc`
    /// of `size2 + 0x10`, 0x40000 for 0, index; CPU write) created by
    /// `CreateBuffer` when missing, from the frame's descriptions; the
    /// sizes at `+0x18`/`+0x1c`, then the CPU copies at `+0x20`/`+0x24`
    /// (16-aligned `PhyreMalloc`) when missing. 9 when a creation fails,
    /// else 0. `frame`: the original's `ebp - 0x38` (`this`, then the
    /// two descriptions at `+4` and `+0x1c`).
    pub fn create_dynamic_buffers(
        h: &mut impl Host,
        this: u32,
        dev: u32,
        size: u32,
        size2: u32,
        frame: u32,
    ) -> u32 {
        let size = if size == 0 { 0x10_0000 } else { size };
        let size2 = if size2 == 0 { 0x4_0000 } else { size2 };
        h.write(frame, this);
        h.write(this + 0x18, size);
        h.write(this + 0x1c, size2);
        let (d1, d2) = (frame + 0x1c, frame + 4);
        desc(h, d1, [size, 2, 1, 0x1_0000, 0, 0]);
        desc(h, d2, [size2.wrapping_add(0x10), 2, 2, 0x1_0000, 0, 0]);
        for k in 0..2 {
            let p = this + 4 * k;
            if h.read::<u32>(p) == 0 && (com(h, d1, dev, 3, &[d1, 0, p]) as i32) < 0 {
                return 9;
            }
            if h.read::<u32>(p + 8) == 0 && (com(h, d2, dev, 3, &[d2, 0, p + 8]) as i32) < 0 {
                return 9;
            }
        }
        if h.read::<u32>(this + 0x20) == 0 {
            let n: u32 = h.read(this + 0x18);
            let p = h.call_sym(MALLOC, Conv::Cdecl, &[n, 0x10]);
            h.write(this + 0x20, p);
        }
        if h.read::<u32>(this + 0x24) == 0 {
            let n = h.read::<u32>(this + 0x1c).wrapping_add(0x10);
            let p = h.call_sym(MALLOC, Conv::Cdecl, &[n, 0x10]);
            h.write(this + 0x24, p);
        }
        0
    }

    /// `0x00583f20(this)`: the D3D11 shader's words: `+0..+0x18` and
    /// `+0x4dc` zero, the 0x200 bytes at `+0x1c` and 0x40 at `+0x21c`
    /// cleared (`memset`), `+0x25c..+0x298` zero, `+0x29c..+0x4d8` all 4;
    /// returns `this`.
    pub fn shader_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in (0..=0x18).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x4dc, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x1c, 0, 0x200]);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x21c, 0, 0x40]);
        for off in (0x25c..=0x298).step_by(4) {
            h.write(this + off, 0u32);
        }
        for off in (0x29c..=0x4d8).step_by(4) {
            h.write(this + off, 4u32);
        }
        this
    }

    /// `0x004e4ec0(obj)` (cdecl): the byte `0x00ca5fe8` set; for each of
    /// the `+0x3c` parameter blocks at `+0x30` (stride `[+0x2c] +0x1c`):
    /// with no slots (`0x00ca5fec` zero) the slot table at block `+0x14`
    /// freed and reset to an empty borrowed one; else 7 without the
    /// slot memory `+0x38`, otherwise each block's table set to the next
    /// `n` words of it (borrowed, the old one freed when owned and apart),
    /// cleared, and each 0x28-byte parameter of the block's `+0xc` array
    /// whose index (`[param] +0xc`) is not -1 put in its slot. Returns 0.
    pub fn assign_slots(h: &mut impl Host, obj: u32) -> u32 {
        let flag = h.rebase(0x00ca_5fe8);
        h.write(flag, 1u8);
        let slots = h.rebase(0x00ca_5fec);
        let ty: u32 = h.read(obj + 0x2c);
        let count: u32 = h.read(obj + 0x3c);
        let stride: u32 = h.read(ty + 0x1c);
        let mut block: u32 = h.read(obj + 0x30);
        let mut n: u32 = h.read(slots);
        if n == 0 {
            for _ in 0..count {
                let p: u32 = h.read(block + 0x18);
                if p != 0 && h.read::<u32>(block + 0x14) & BORROWED == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
                }
                h.write(block + 0x18, 0u32);
                h.write(block + 0x14, BORROWED);
                block = block.wrapping_add(stride);
            }
            return 0;
        }
        let mut cursor: u32 = h.read(obj + 0x38);
        if cursor == 0 {
            return 7;
        }
        for _ in 0..count {
            if (n as i32) >= 0 {
                let p: u32 = h.read(block + 0x18);
                if p != cursor && h.read::<u32>(block + 0x14) & BORROWED == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
                }
                h.write(block + 0x18, cursor);
                h.write(block + 0x14, n | BORROWED);
                n = h.read(slots);
            }
            let mut i = 0u32;
            while i < n {
                h.write(cursor, 0u32);
                n = h.read(slots);
                i += 1;
                cursor = cursor.wrapping_add(4);
            }
            let m = h.read::<u32>(block + 0xc) & COUNT;
            for k in 0..m {
                let data: u32 = h.read(block + 0x10);
                let param = data.wrapping_add(0x28 * k);
                let p: u32 = h.read(param);
                let idx: u32 = h.read(p.wrapping_add(0xc));
                if idx != u32::MAX {
                    let table: u32 = h.read(block + 0x18);
                    h.write(table.wrapping_add(idx.wrapping_mul(4)), param);
                }
            }
            n = h.read(slots);
            block = block.wrapping_add(stride);
        }
        0
    }

    /// The owned array at `p + off` (count word, data after it) freed
    /// (no null check) and cleared.
    fn free_owned(h: &mut impl Host, p: u32, off: u32) {
        if h.read::<u32>(p + off) & BORROWED == 0 {
            let d: u32 = h.read(p + off + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[d]);
        }
        h.write(p + off + 4, 0u32);
        h.write(p + off, 0u32);
    }

    /// A `PString` word at `at` freed unless tagged static (bit 0).
    fn free_name(h: &mut impl Host, at: u32) {
        let w: u32 = h.read(at);
        if w & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
        }
    }

    /// `0x004ea2d0(this)`: the name at `+0x34`, the array at `+0x2c`
    /// (`0x004ec310(data, n)`), the owned arrays at `+0x24`, `+0x1c`,
    /// `+0x14`, `+0xc` and the name at `+8` released.
    pub fn destroy_4ea2d0(h: &mut impl Host, this: u32) {
        free_name(h, this + 0x34);
        let n: u32 = h.read(this + 0x2c);
        if n & BORROWED == 0 {
            let d: u32 = h.read(this + 0x30);
            h.call_sym(0x004e_c310, Conv::Cdecl, &[d, n & COUNT]);
        }
        h.write(this + 0x30, 0u32);
        h.write(this + 0x2c, 0u32);
        for off in [0x24, 0x1c, 0x14, 0xc] {
            free_owned(h, this, off);
        }
        free_name(h, this + 8);
    }

    /// `0x00561a20(this)` (vertex) / `0x00561b70(this)` (index): without
    /// a width (`this + width`) a warning (`msg`) and 0; else under the
    /// scoped lock each of the 20-byte entries of the `PSharray` at
    /// `this + array` gets a dynamic buffer of that width (`bind` flags,
    /// CPU write) from the render thread's device; bit 0 of
    /// `this + flag` set and 0, or 9 when a creation fails. `frame`: the
    /// original's `ebp - 0x34` (the count, the lock, the device, the
    /// description).
    pub fn create_stream_buffers(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        layout: (u32, u32, u32, u32, u32),
    ) -> u32 {
        let (width_off, array, bind, flag, msg) = layout;
        let width: u32 = h.read(this + width_off);
        if width == 0 {
            let msg = h.rebase(msg);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg]);
            return 0;
        }
        let lock = frame + 4;
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        holder(h);
        let dev: u32 = h.read(h.rebase(HOLDER_DEVICE));
        let d = frame + 0xc;
        desc(h, d, [width, 2, bind, 0x1_0000, 0, 0]);
        h.write(frame + 8, dev);
        let n = h.read::<u32>(this + array) & COUNT;
        h.write(frame, n);
        let mut result = 0;
        for i in 0..n {
            let s = storage(h, this + array);
            let vt: u32 = h.read(dev);
            let r = com(h, vt, dev, 3, &[d, 0, s.wrapping_add(20 * i)]);
            if (r as i32) < 0 {
                result = 9;
                break;
            }
        }
        if result == 0 {
            let f: u32 = h.read(this + flag);
            h.write(this + flag, f | 1);
        }
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        result
    }

    /// `0x00564ab0(this; i, _)`: entry `i` (20 bytes) of the `PSharray`
    /// at `+0x18`: when it holds a buffer, its `GetDesc` into the frame,
    /// then under the scoped lock the render thread's context unmaps it:
    /// the staging copy (word 3, via `0x0055e0f0(array; i)`) for a buffer
    /// without CPU read access that has one, else the buffer. Returns 0.
    /// `frame`: the original's `ebp - 0x30` (`i`, the lock, the
    /// description).
    pub fn unmap_stream(h: &mut impl Host, this: u32, i: u32, frame: u32) -> u32 {
        let arr = this + 0x18;
        h.write(frame, i);
        let at = i.wrapping_mul(20);
        let s = storage(h, arr);
        let buf: u32 = h.read(s.wrapping_add(at));
        if buf == 0 {
            return 0;
        }
        let vt: u32 = h.read(buf);
        com(h, vt, buf, 10, &[frame + 8]);
        let lock = frame + 4;
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        holder(h);
        let s = storage(h, arr);
        if h.read::<u32>(frame + 0x14) & 0x2_0000 == 0
            && h.read::<u32>(s.wrapping_add(at + 0xc)) != 0
        {
            let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let e = h.call_sym(0x0055_e0f0, Conv::Thiscall(arr), &[i]);
            let staging: u32 = h.read(e.wrapping_add(0xc));
            com(h, arr, ctx, 15, &[staging, 0]);
        } else {
            let ctx: u32 = h.read(h.rebase(HOLDER_CONTEXT));
            let s = storage(h, arr);
            let b: u32 = h.read(s.wrapping_add(at));
            com(h, ctx, ctx, 15, &[b, 0]);
        }
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        0
    }

    /// `0x0058f710(this)`: for a texture with bit 3 of `+0x64`, under the
    /// scoped lock a shader-resource view for each of
    /// `0x00593bb0(this)` 0x1c-byte entries of the `PSharray` at `+0x24`
    /// (`CreateShaderResourceView(entry +0, desc, entry +8)` on the
    /// render thread's device; the description: the format `[+0] +0x20`,
    /// dimension 4, or 6 when `+0x58` is not 1, the mip count `+0x4c`).
    /// 9 when one fails, else 0. `frame`: the original's `ebp - 0x30`
    /// (the description, the lock at `+0x18`, the index, the device).
    pub fn create_views(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u8>(this + 0x64) & 8 == 0 {
            return 0;
        }
        let lock = frame + 0x18;
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        holder(h);
        let dev: u32 = h.read(h.rebase(HOLDER_DEVICE));
        h.write(frame + 0x20, dev);
        let res: u32 = h.read(this);
        let format: u32 = h.read(res + 0x20);
        let dim = if h.read::<u32>(this + 0x58) == 1 {
            4
        } else {
            6
        };
        let mips: u32 = h.read(this + 0x4c);
        desc(h, frame, [format, dim, 0, mips, 0, 0]);
        h.write(frame + 0x1c, 0u32);
        let mut i = 0u32;
        let mut result = 0;
        while i < h.call_sym(0x0059_3bb0, Conv::Thiscall(this), &[]) {
            let s = storage(h, this + 0x24);
            let off = 0x1c * i;
            let r: u32 = h.read(s.wrapping_add(off));
            let vt: u32 = h.read(dev);
            let hr = com(h, vt, dev, 7, &[r, frame, s.wrapping_add(off + 8)]);
            if (hr as i32) < 0 {
                result = 9;
                break;
            }
            i += 1;
            h.write(frame + 0x1c, i);
        }
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        result
    }
}
