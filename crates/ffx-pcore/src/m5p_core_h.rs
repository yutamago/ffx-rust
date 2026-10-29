//! M5 stage 1, list `phyre-core`, batch 8: a texture level's upload to
//! the GPU (`0x0059d8f0`).
//!
//! Under the render scope lock (`PRenderScopeLock`, an object in the
//! caller's frame), the level's surfaces (`+0x24`, a `PSharray` of
//! 28-byte entries: the D3D11 resource, the data at `+0x10`) are copied
//! mip by mip with the device context's `UpdateSubresource` (vtable
//! `+0xc0`), the row pitch from the format (`[+0]`: bits per pixel `+0xc`,
//! block-compressed when bit 0 of `+0x10` is set) and the size (`+0x1c`,
//! `+0x20`, halved per mip, at least 1). Textures flagged `0x10000` in
//! `+0x68` call the context's method `+0x3c (resource, mip)` per mip
//! instead. The context is the renderer's (`[0x00c94f70]`, inside the
//! lazily built static at `0x00c94f00`).

/// `Phyre_PSharray_element28_00585c30`'s port: the 28-byte entry.
pub const ENTRY: u32 = 0x1c;
/// The render scope lock's enter and leave (thiscall on the frame
/// object).
pub const LOCK_ENTER: u32 = 0x0046_3d50;
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The renderer static, its guard, constructor and `atexit` destructor;
/// the device context word inside it; the CRT's `atexit`.
pub const RENDERER: u32 = 0x00c9_4f00;
pub const RENDERER_GUARD: u32 = 0x00c9_628c;
pub const RENDERER_CTOR: u32 = 0x004f_ee70;
pub const RENDERER_DTOR: u32 = 0x00af_b710;
pub const CONTEXT: u32 = 0x00c9_4f70;
pub const ATEXIT: u32 = 0x0094_9207;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// A `PSharray`'s storage.
    fn storage(h: &impl Host, at: u32) -> u32 {
        match h.read::<u32>(at) & 0x7fff_ffff {
            0 => 0,
            1 => at + 4,
            _ => h.read(at + 4),
        }
    }

    /// The bytes of a `w` x `h` block of format `fmt` (block-compressed
    /// sizes rounded up to 4), the rows' pitch when `rows` is 1.
    fn bytes(h: &impl Host, fmt: u32, w: u32, rows: u32, pitch_only: bool) -> u32 {
        let bpp: u32 = h.read(fmt + 0xc);
        if h.read::<u8>(fmt + 0x10) & 1 != 0 {
            let w = w.wrapping_add(3) & !3;
            let r = if pitch_only {
                4
            } else {
                rows.wrapping_add(3) & !3
            };
            bpp.wrapping_mul(w).wrapping_mul(r) >> 3
        } else {
            let r = if pitch_only { 1 } else { rows };
            bpp.wrapping_mul(w).wrapping_mul(r) >> 3
        }
    }

    /// `0x0059d8f0(texture; level, unused)` -> 0, 3 for a level past the
    /// surfaces (at least one counts), 0x15 for one without a resource.
    /// The scope lock lives in the frame word at `lock`.
    pub fn upload(h: &mut impl Host, this: u32, level: u32, lock: u32) -> u32 {
        let surfaces = this + 0x24;
        let mut n = h.read::<u32>(surfaces) & 0x7fff_ffff;
        if n == 0 {
            n = 1;
        }
        if level >= n {
            return 3;
        }
        let entry = storage(h, surfaces).wrapping_add(level.wrapping_mul(ENTRY));
        if h.read::<u32>(entry) == 0 {
            return 0x15;
        }
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let guard = h.rebase(RENDERER_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let r = h.rebase(RENDERER);
            h.call_sym(RENDERER_CTOR, Conv::Thiscall(r), &[]);
            let dtor = h.rebase(RENDERER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let ctx: u32 = h.read(h.rebase(CONTEXT));
        let mips = if (h.read::<u32>(this + 0x14) >> 1) & 1 != 0 {
            0
        } else {
            h.read(this + 0xc)
        };
        let off = level.wrapping_mul(ENTRY);
        if h.read::<u32>(this + 0x68) & 0x1_0000 != 0 {
            let mut i = 0u32;
            loop {
                let res: u32 = h.read(storage(h, surfaces).wrapping_add(off));
                let vt: u32 = h.read(ctx);
                let f: u32 = h.read(vt.wrapping_add(0x3c));
                h.call(f, Conv::Stdcall, &[ctx, res, i]);
                i = i.wrapping_add(1);
                if i > mips {
                    break;
                }
            }
        } else {
            let entry = storage(h, surfaces).wrapping_add(off);
            let mut data: u32 = h.read(entry.wrapping_add(0x10));
            let mut w: u32 = h.read(this + 0x1c);
            let mut ht: u32 = h.read(this + 0x20);
            let mut i = 0u32;
            loop {
                let ww = if w == 0 { h.read(this + 0x1c) } else { w };
                let fmt: u32 = h.read(this);
                let pitch = bytes(h, fmt, ww, 1, true);
                let res: u32 = h.read(storage(h, surfaces).wrapping_add(off));
                let vt: u32 = h.read(ctx);
                let f: u32 = h.read(vt.wrapping_add(0xc0));
                h.call(f, Conv::Stdcall, &[ctx, res, i, 0, data, pitch, 0]);
                let a = if w == 0 { h.read(this + 0x1c) } else { w };
                let b = if ht == 0 { h.read(this + 0x20) } else { ht };
                let fmt: u32 = h.read(this);
                data = data.wrapping_add(bytes(h, fmt, a, b, false));
                w >>= 1;
                if w <= 1 {
                    w = 1;
                }
                ht >>= 1;
                if ht <= 1 {
                    ht = 1;
                }
                i = i.wrapping_add(1);
                if i > mips {
                    break;
                }
            }
        }
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        0
    }
}
