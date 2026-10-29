//! M5 stage 1, `PhyreEngine` list `render-a`, batch 8:
//! `PTexture3DD3D11::create`, the 3D texture's D3D11 textures (the
//! description, one subresource per mip, `CreateTexture3D` per view and
//! its `GetDesc`), then its views.

use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a8::{
    HOLDER, HOLDER_CTOR, HOLDER_DEVICE, HOLDER_DTOR, HOLDER_GUARD, SCOPE_ENTER, SCOPE_LEAVE,
    VIEW_COUNT,
};

/// `PTexture3DD3D11_createViews_0058f880(this)`.
pub const CREATE_VIEWS: u32 = 0x0058_f880;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The bytes of the frame `create` hands out (`ebp - 0x1e0` up to the
    /// cookie at `ebp - 0x10`: the lock, the description and the
    /// subresources).
    pub const CREATE_FRAME: u32 = 0x1d0;
    /// The subresource entries its frame holds (from `ebp - 0x190` up to
    /// the cookie, 12 bytes each; the original has no check, more would
    /// overwrite its cookie and end the process).
    pub const SUBRESOURCES: u32 = 32;

    /// A subresource size: `bits * w * h / 8` (`w`, `h` rounded up to 4
    /// for block-compressed formats).
    fn size(bits: u32, w: u32, hh: u32, block: bool) -> u32 {
        let (w, hh) = if block {
            (w.wrapping_add(3) & !3, hh.wrapping_add(3) & !3)
        } else {
            (w, hh)
        };
        bits.wrapping_mul(w).wrapping_mul(hh) >> 3
    }

    /// `PTexture3DD3D11_create_005903a0(this; data, views, _, skip)` (`ret
    /// 0x10`) -> 0 or a status. `dynamic` = no sRGB bind (bit 8 of
    /// `+0x14`) and (`skip`, or no `data`); 5 at once for `skip` with the
    /// sRGB bind. Under the lock (`ebp - 0x1d8`) with the device holder
    /// (0x17 without a device): the `D3D11_TEXTURE3D_DESC` at `ebp -
    /// 0x1b4` (sizes `+0x1c..+0x24`, mips `+0xc` + 1, or 1 when `+0x10` is
    /// 0 and `+0x14` has no bit 1; the format `[+0] + 0x18`; usage 2 and
    /// CPU write 0x10000 when dynamic with one mip; bind 8 for `views`,
    /// with the sRGB bind 0x40 or 0x20 by bit 1 of `[+0] + 0x10`), one
    /// subresource per mip from `data` on at `ebp - 0x190` (row and slice
    /// pitches from `[+0] + 0xc` bits per texel, the sizes halving and 0
    /// standing for the full size, block formats rounded to 4 and rows of
    /// 4); `CreateTexture3D` (device slot 6, the subresources only with
    /// `data`; 9 for anything but 0) into each 20-byte element of the
    /// sharray `+0x28` (`0x00593bc0` of them, asked again after each),
    /// each one's `GetDesc` (slot 10) into `+0x40`. After the lock, with
    /// `views`, `PTexture3DD3D11_createViews`'s status when not 0. `frame`
    /// is the original's `ebp - 0x1e0`.
    #[allow(clippy::too_many_lines)]
    pub fn create(h: &mut impl Host, this: u32, data: u32, views: u8, skip: u8, frame: u32) -> u32 {
        let local = |off: u32| frame + 0x1e0 - off;
        let srgb_bind = (h.read::<u32>(this + 0x14) >> 8) & 1 != 0;
        if skip != 0 && srgb_bind {
            return 5;
        }
        let mut dynamic = !srgb_bind && (skip != 0 || data == 0);
        let base: u32 = h.read(this);
        let lock = local(0x1d8);
        this_call(h, SCOPE_ENTER, lock, &[]);
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            this_call(h, HOLDER_CTOR, holder, &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            h.call_sym(0x0094_9207, Conv::Cdecl, &[dtor]);
        }
        let dev_at = h.rebase(HOLDER_DEVICE);
        let dev: u32 = h.read(dev_at);
        if dev == 0 {
            this_call(h, SCOPE_LEAVE, lock, &[]);
            return 0x17;
        }
        let w: u32 = h.read(this + 0x1c);
        let hh: u32 = h.read(this + 0x20);
        let d: u32 = h.read(this + 0x24);
        let mut mips = h.read::<u32>(this + 0xc).wrapping_add(1);
        if h.read::<u32>(this + 0x10) == 0 && (h.read::<u32>(this + 0x14) >> 1) & 1 == 0 {
            mips = 1;
        }
        let format: u32 = h.read(base + 0x18);
        if dynamic {
            dynamic = mips <= 1;
        }
        let mut bind = if views != 0 { 8u32 } else { 0 };
        if srgb_bind {
            bind |= if (h.read::<u32>(base + 0x10) >> 1) & 1 != 0 {
                0x40
            } else {
                0x20
            };
        }
        let usage = if dynamic { 2u32 } else { 0 };
        let cpu = if dynamic { 0x1_0000u32 } else { 0 };
        let desc = local(0x1b4);
        for (k, v) in (0u32..).zip([w, hh, d, mips, format, usage, bind, cpu, 0]) {
            h.write(desc + 4 * k, v);
        }
        // the subresources
        let subs = local(0x190);
        let mut cursor = data;
        let (mut wc, mut hc, mut dc) = (w, hh, d);
        // past 32 the original overwrites its cookie; the cursor is not used after
        for entry in 0..mips.min(SUBRESOURCES) {
            let block = h.read::<u8>(base + 0x10) & 1 != 0;
            let bits: u32 = h.read(base + 0xc);
            let fw = if wc == 0 { w } else { wc };
            let fh = if hc == 0 { hh } else { hc };
            let fd = if dc == 0 { d } else { dc };
            let total = size(bits, fw, fh, block).wrapping_mul(fd);
            let (row_w, rows) = if block {
                (fw.wrapping_add(3) & !3, 4u32)
            } else {
                (fw, 1)
            };
            let pitch = bits.wrapping_mul(row_w).wrapping_mul(rows) >> 3;
            let slice = size(bits, fw, fh, block);
            {
                let e = subs + 0xc * entry;
                h.write(e, cursor);
                h.write(e + 4, pitch);
                h.write(e + 8, slice);
            }
            cursor = cursor.wrapping_add(total);
            wc >>= 1;
            hc >>= 1;
            dc >>= 1;
        }
        let init = if data != 0 { subs } else { 0 };
        let mut i = 0u32;
        let mut off = 0u32;
        while i < this_call(h, VIEW_COUNT, this, &[]) {
            let n = h.read::<u32>(this + 0x28) & 0x7fff_ffff;
            let textures = if n > 1 {
                h.read(this + 0x2c)
            } else if n != 0 {
                this + 0x2c
            } else {
                0
            };
            let vt: u32 = h.read(dev);
            let f: u32 = h.read(vt + 0x18);
            let hr = h.call(
                f,
                Conv::Stdcall,
                &[dev, desc, init, textures.wrapping_add(off)],
            );
            if hr != 0 {
                this_call(h, SCOPE_LEAVE, lock, &[]);
                return 9;
            }
            let n = h.read::<u32>(this + 0x28) & 0x7fff_ffff;
            let textures = if n > 1 {
                h.read(this + 0x2c)
            } else if n != 0 {
                this + 0x2c
            } else {
                0
            };
            let tex: u32 = h.read(textures.wrapping_add(off));
            let vt: u32 = h.read(tex);
            let f: u32 = h.read(vt + 0x28);
            h.call(f, Conv::Stdcall, &[tex, this + 0x40]);
            i += 1;
            off = off.wrapping_add(0x14);
        }
        this_call(h, SCOPE_LEAVE, lock, &[]);
        if views != 0 {
            let r = this_call(h, CREATE_VIEWS, this, &[]);
            if r != 0 {
                return r;
            }
        }
        0
    }
}
