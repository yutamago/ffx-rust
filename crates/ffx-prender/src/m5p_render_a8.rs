//! M5 stage 1, `PhyreEngine` list `render-a`, batch 7: the sprite
//! collection's rotation setter (`_CIsin` / `_CIcos`), the 3D texture's
//! shader-resource views and the sprite collection's flush (the records
//! copied into the mapped vertex buffer, sorted by depth, their bounds
//! handed on).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::f32_move;
use crate::m5p_render_a3::MEMCPY;

/// The CRT intrinsics' import thunks.
pub const CI_SIN: u32 = 0x0094_98c8;
pub const CI_COS: u32 = 0x0094_98c2;
/// The scoped lock's enter and leave (thiscall on the lock).
pub const SCOPE_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LEAVE: u32 = 0x0046_3e90;
/// The D3D11 device holder: guard, object, constructor, atexit
/// destructor, the device word.
pub const HOLDER_GUARD: u32 = 0x00c9_628c;
pub const HOLDER: u32 = 0x00c9_4f00;
pub const HOLDER_CTOR: u32 = 0x004f_ee70;
pub const HOLDER_DTOR: u32 = 0x00af_b710;
pub const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
/// `0x00593bc0(texture)`: the view count of a 3D texture.
pub const VIEW_COUNT: u32 = 0x0059_3bc0;
/// The CRT's `qsort` import slot.
pub const QSORT: u32 = 0x00b0_c3ac;
/// The f32 bounds' starting values (`+FLT_MAX`, `-FLT_MAX`).
pub const BOUND_LOW: u32 = 0x00b1_7504;
pub const BOUND_HIGH: u32 = 0x00b1_f2d0;
/// The f64 degrees-to-radians factor.
pub const DEG_TO_RAD: u32 = 0x00b9_2220;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `FUN_0056e730(this; frame, angle)` (`ret 8`) -> 0, 9 or 5: the frame
    /// record's `+8` = sin(angle), `+0xc` = cos(angle) (each the CRT
    /// intrinsic's result stored to f32), `+0x2c` = angle.
    pub fn sprite_rotation(h: &mut impl Host, this: u32, frame: u32, angle: u32) -> u32 {
        let records: u32 = h.read(this + 0xb8);
        if records == 0 {
            return 9;
        }
        let order: u32 = h.read(this + 0xc8);
        let idx: u32 = h.read(order.wrapping_add(frame.wrapping_mul(4)));
        if idx >= h.read::<u32>(this + 0x20) {
            return 5;
        }
        let rec = records.wrapping_add(idx.wrapping_mul(48));
        let x = x87::load(angle);
        let sin = h.rebase(CI_SIN);
        let s = h.call_intrinsic(sin, x).to_bits();
        h.write(rec + 8, f32_move(s));
        let cos = h.rebase(CI_COS);
        let c = h.call_intrinsic(cos, x).to_bits();
        h.write(rec + 0xc, f32_move(c));
        h.write(rec + 0x2c, f32_move(angle));
        0
    }

    /// The device holder, built once.
    fn holder(h: &mut impl Host) {
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            this_call(h, HOLDER_CTOR, holder, &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            cdecl(h, 0x0094_9207, &[dtor]);
        }
    }

    /// The bytes of `FUN_0058f880`'s frame it hands out (`ebp - 0x30` up
    /// to `ebp - 0x10`: the view description and the lock).
    pub const VIEWS_FRAME: u32 = 0x20;

    /// `FUN_0058f880(this)` -> 0, or 9 when a view fails: for a texture
    /// with bit 3 of `+0x58`, under the lock (`ebp - 0x18`) and with the
    /// device holder built, one `CreateShaderResourceView` (device slot
    /// 7) per view (`0x00593bc0`, asked again after each) of the 20-byte
    /// sharray at `+0x28` (the resource word, the view at `+4`), with the
    /// description at `ebp - 0x30`: the format `[+0] + 0x20`, dimension 8
    /// (3D), mip 0, the mip count `+0x4c`. `frame` is the original's
    /// `ebp - 0x30`.
    pub fn create_views(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        if h.read::<u8>(this + 0x58) & 8 == 0 {
            return 0;
        }
        let lock = frame + 0x18;
        this_call(h, SCOPE_ENTER, lock, &[]);
        holder(h);
        let dev_at = h.rebase(HOLDER_DEVICE);
        let dev: u32 = h.read(dev_at);
        let desc = frame;
        let base: u32 = h.read(this);
        let format: u32 = h.read(base + 0x20);
        let mips: u32 = h.read(this + 0x4c);
        for (k, v) in (0u32..).zip([format, 8, 0, mips, 0, 0]) {
            h.write(desc + 4 * k, v);
        }
        let mut i = 0u32;
        let mut off = 0u32;
        while i < this_call(h, VIEW_COUNT, this, &[]) {
            let n = h.read::<u32>(this + 0x28) & 0x7fff_ffff;
            let views = if n > 1 {
                h.read(this + 0x2c)
            } else if n != 0 {
                this + 0x2c
            } else {
                0
            };
            let resource: u32 = h.read(views.wrapping_add(off));
            let vt: u32 = h.read(dev);
            let f: u32 = h.read(vt + 0x1c);
            let hr = h.call(
                f,
                Conv::Stdcall,
                &[dev, resource, desc, views.wrapping_add(off + 4)],
            );
            if hr & 0x8000_0000 != 0 {
                this_call(h, SCOPE_LEAVE, lock, &[]);
                return 9;
            }
            off = off.wrapping_add(0x14);
            i += 1;
        }
        this_call(h, SCOPE_LEAVE, lock, &[]);
        0
    }

    /// The minimum's keep (`fld bound; fld value; fcom st(1); test ah,
    /// 0x41`): the value unless it is above the bound (equal or unordered
    /// keep the value).
    fn min_keep(x: f64, b: f64) -> f64 {
        if x > b { b } else { x }
    }

    /// `FUN_0056ee50(this)` -> 0 or the vertex buffer's status: for a
    /// collection with records (`+0xb8`): the vertex buffer `+0x50`
    /// mapped (`0x00434c50(vb; &data)`, a non-zero status returned), the
    /// `+0x20` 48-byte records copied in (`memcpy`), sorted by depth when
    /// the byte `+4` is set (`qsort` with `PSprite_compareDepth`), the
    /// buffer's next slot unmapped (as `PBuffer_unmapNextSlot`, its
    /// status returned when not 0); the bounds of the records (`x - w`,
    /// `x + w`, `y - h`, `y + h` with `w`, `h` the floats `+0x20`/`+0x24`
    /// times the f64 at `0x00b92220`, the depth `+0x28`; from `+FLT_MAX`
    /// and `-FLT_MAX`) handed to the bounds owner `[[this] + 0x1c]`
    /// (`0x00500bd0(owner; &min, &max)`, min at `ebp - 0x2c`, max at
    /// `ebp - 0x48`); then no records (`+0xb8` = 0) and the buffer's count `+4`
    /// = `+0x20`. `frame` is the original's `ebp - 0x48`.
    pub fn flush(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let at = |n: u32| frame + 0x48 - n;
        if h.read::<u32>(this + 0xb8) == 0 {
            return 0;
        }
        let vb: u32 = h.read(this + 0x50);
        let r = this_call(h, 0x0043_4c50, vb, &[at(0x1c)]);
        if r != 0 {
            return r;
        }
        let count: u32 = h.read(this + 0x20);
        let records: u32 = h.read(this + 0xb8);
        let data: u32 = h.read(at(0x1c));
        cdecl(h, MEMCPY, &[data, records, count.wrapping_mul(48)]);
        if h.read::<u8>(this + 4) != 0 {
            let count: u32 = h.read(this + 0x20);
            let data: u32 = h.read(at(0x1c));
            let cmp = h.rebase(0x0056_c110);
            let qs = h.rebase(QSORT);
            let qs: u32 = h.read(qs);
            h.call(qs, Conv::Cdecl, &[data, count, 0x30, cmp]);
        }
        let next = h.read::<u32>(vb + 0x10).wrapping_add(1);
        let n = h.read::<u32>(vb + 0x18) & 0x7fff_ffff;
        let slot = if next == n { 0 } else { next };
        let r = this_call(h, 0x0056_4cb0, vb, &[slot, at(0x1c)]);
        if r != 0 {
            return r;
        }
        h.write(vb + 0x10, slot);
        h.write(vb + 0x14, 0u8);
        let count: u32 = h.read(this + 0x20);
        if count != 0 {
            let lo_at = h.rebase(BOUND_LOW);
            let lo = x87::load(h.read(lo_at));
            let hi_at = h.rebase(BOUND_HIGH);
            let hi = x87::load(h.read(hi_at));
            let k_at = h.rebase(DEG_TO_RAD);
            let k = f64::from_bits(h.read(k_at));
            let mut min = [x87::store(lo); 3];
            let mut max = [x87::store(hi); 3];
            let mut rec: u32 = h.read(this + 0xb8);
            for _ in 0..count {
                let x = x87::load(h.read(rec));
                let y = x87::load(h.read(rec + 4));
                let w = x87::mul(x87::load(h.read(rec + 0x20)), k);
                let hh = x87::mul(x87::load(h.read(rec + 0x24)), k);
                let x0 = x87::store(x87::sub(x, w));
                let y0 = x87::store(x87::sub(y, hh));
                let x1 = x87::store(x87::add(w, x87::load(h.read(rec))));
                let y1 = x87::store(x87::add(hh, x87::load(h.read(rec + 4))));
                let z = f32_move(h.read(rec + 0x28));
                min[0] = x87::store(min_keep(x87::load(x0), x87::load(min[0])));
                min[1] = x87::store(min_keep(x87::load(y0), x87::load(min[1])));
                min[2] = x87::store(min_keep(x87::load(z), x87::load(min[2])));
                max[0] = x87::store(max_keep(x87::load(x1), x87::load(max[0])));
                max[1] = x87::store(max_keep(x87::load(y1), x87::load(max[1])));
                // maximum z: `fld bound; fcom st(1)` (the bound against
                // the value), `test ah, 0x41`
                max[2] = x87::store(max_z(x87::load(z), x87::load(max[2])));
                rec = rec.wrapping_add(0x30);
            }
            let base: u32 = h.read(this);
            let owner: u32 = h.read(base + 0x1c);
            if owner != 0 {
                // max at ebp-0x48.., min at ebp-0x2c..
                for (i, v) in (0u32..).zip(max) {
                    h.write(at(0x48) + 4 * i, v);
                }
                for (i, v) in (0u32..).zip(min) {
                    h.write(at(0x2c) + 4 * i, v);
                }
                this_call(h, 0x0050_0bd0, owner, &[at(0x2c), at(0x48)]);
            }
        }
        let vb: u32 = h.read(this + 0x50);
        let count: u32 = h.read(this + 0x20);
        h.write(this + 0xb8, 0u32);
        h.write(vb + 4, count);
        0
    }

    /// The maximum's keep for x and y (`fld bound; fld value; fcom
    /// st(1); test ah, 5; jp`): the value unless it is below the bound
    /// (equal or unordered keep the value).
    fn max_keep(x: f64, b: f64) -> f64 {
        if x < b { b } else { x }
    }

    /// The maximum's keep for z (`fld value` under `fld bound`; `fcom
    /// st(1)` compares the bound with the value; `test ah, 0x41`): the
    /// bound when it is above the value, else the value.
    fn max_z(z: f64, b: f64) -> f64 {
        if b > z { b } else { z }
    }
}
