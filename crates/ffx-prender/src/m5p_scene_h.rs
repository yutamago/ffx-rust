//! M5 stage 1, list `scene`, batch 8: the render pass array's copy
//! construction, the rendering utility's resource teardown and the
//! animation utility's time range. Conventions as in
//! [`crate::m5p_scene_b`]; floats on the x87 (compares as `fcom`
//! reports them); frames as in [`crate::m5p_scene_f`].
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::PHYRE_FREE;
pub use crate::m5p_scene_c::{CONSTANT_BUFFER_DTOR, PHYRE_MALLOC};

/// The copies of a render pass's four arrays (cdecl `(dst, src, n)`) and
/// their element sizes.
pub const PASS_ARRAYS: [(u32, u32); 4] = [
    (0x004d_8a30, 0x20),
    (0x004d_8bd0, 0x20),
    (0x004d_89e0, 0x14),
    (0x0044_a470, 4),
];
/// The rendering utility's globals `0x004fdca0` tears down: its lock,
/// a tagged array (count, buffer), the scratch buffer (`u16` flag,
/// buffer, word) and the constant buffers (count, buffer of 0x3c-byte
/// `PConstantBuffer`s).
pub const RENDER_LOCK: u32 = 0x00ca_852c;
pub const TAGGED: u32 = 0x00ca_8510;
pub const SCRATCH: [u32; 3] = [0x00ca_8518, 0x00ca_851c, 0x00ca_8520];
pub const CONSTANT_BUFFERS: u32 = 0x00ca_8524;
/// `Phyre_PCriticalSectionBase_release_00431950` (thiscall).
pub const LOCK_RELEASE: u32 = 0x0043_1950;
/// `PArray_releaseTaggedElements_004ab4d0(items, n)` (cdecl).
pub const RELEASE_TAGGED: u32 = 0x004a_b4d0;
/// `PConstantBufferNVN_isBound` (thiscall, `al`) and
/// `PConstantBufferD3D11_releaseEntries` (thiscall).
pub const CB_IS_BOUND: u32 = 0x0059_5f70;
pub const CB_RELEASE: u32 = 0x0059_c540;
/// `0x0050acf0` (thiscall on a clip, `(&start, &end)`, `ret 8`): a clip's
/// time range; 0 when it has one.
pub const CLIP_RANGE: u32 = 0x0050_acf0;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        CB_IS_BOUND, CB_RELEASE, CLIP_RANGE, CONSTANT_BUFFER_DTOR, CONSTANT_BUFFERS, Conv, Host,
        LOCK_RELEASE, Mem, PASS_ARRAYS, PHYRE_FREE, PHYRE_MALLOC, RELEASE_TAGGED, RENDER_LOCK,
        SCRATCH, TAGGED, x87,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    /// `0x004e1ae0` (cdecl `(dst, src, n)`): `n` render passes (0x28 bytes)
    /// copy-constructed: the first word, then each of the four arrays
    /// (count, buffer) as a fresh buffer of its elements (none for an
    /// empty one; a failed allocation leaves count and buffer 0), the next
    /// count cleared before each, and the flag byte at `+0x24`. A null
    /// destination slot is skipped without using up a source pass.
    pub fn render_passes_copy(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut d = dst;
        let mut s = src;
        for _ in 0..n {
            if d != 0 {
                let w: u32 = h.read(s);
                h.write(d, w);
                h.write(d + 4, 0u32);
                for (k, &(copy, size)) in PASS_ARRAYS.iter().enumerate() {
                    let k = u32::try_from(k).unwrap_or(0);
                    let count: u32 = h.read(s + 4 + 8 * k);
                    let count = count & !EXTERNAL;
                    let data: u32 = h.read(s + 8 + 8 * k);
                    let mut buf = 0;
                    if count != 0 {
                        buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[count.wrapping_mul(size), 4]);
                        if buf != 0 {
                            h.call_sym(copy, Conv::Cdecl, &[buf, data, count]);
                        }
                    }
                    h.write(d + 8 + 8 * k, buf);
                    h.write(d + 4 + 8 * k, if buf == 0 { 0 } else { count });
                    if k < 3 {
                        h.write(d + 0xc + 8 * k, 0u32);
                    }
                }
                let flag: u8 = h.read(s + 0x24);
                h.write(d + 0x24, flag);
                s = s.wrapping_add(0x28);
            }
            d = d.wrapping_add(0x28);
        }
    }

    /// `0x004fdca0` (cdecl): the rendering utility's resources released:
    /// the lock, the tagged array, every bound constant buffer's entries,
    /// the scratch buffer, the constant buffers destroyed and freed. Its
    /// `eax`: the last free's, or what the constant buffers' header leaves
    /// (0 for none, the count for a null buffer, 0xfe for an external one).
    pub fn render_resources_release(h: &mut impl Host) -> u32 {
        let lock = h.rebase(RENDER_LOCK);
        h.call_sym(LOCK_RELEASE, Conv::Thiscall(lock), &[]);
        let tagged = h.rebase(TAGGED);
        let count: u32 = h.read(tagged);
        if count != 0 {
            let items: u32 = h.read(tagged + 4);
            if items != 0 && count & EXTERNAL == 0 {
                h.call_sym(RELEASE_TAGGED, Conv::Cdecl, &[items, count & !EXTERNAL]);
            }
            h.write(tagged + 4, 0u32);
            h.write(tagged, 0u32);
        }
        let cbs = h.rebase(CONSTANT_BUFFERS);
        let mut i = 0u32;
        loop {
            let n: u32 = h.read(cbs);
            if i >= n & !EXTERNAL {
                break;
            }
            let base: u32 = h.read(cbs + 4);
            let cb = base.wrapping_add(i.wrapping_mul(0x3c));
            let bound = h.call_sym(CB_IS_BOUND, Conv::Thiscall(cb), &[]);
            if bound & 0xff != 0 {
                h.call_sym(CB_RELEASE, Conv::Thiscall(cb), &[]);
            }
            i += 1;
        }
        let flag = h.rebase(SCRATCH[0]);
        let buffer = h.rebase(SCRATCH[1]);
        let word = h.rebase(SCRATCH[2]);
        h.write(flag, 0u16);
        h.write(word, 0u32);
        let p: u32 = h.read(buffer);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        h.write(buffer, 0u32);
        let header: u32 = h.read(cbs);
        if header == 0 {
            return 0;
        }
        let mut eax = header;
        let base: u32 = h.read(cbs + 4);
        if base != 0 {
            eax = (header >> 31) ^ 0xff;
            if header & EXTERNAL == 0 {
                let mut p = base;
                for _ in 0..header & !EXTERNAL {
                    h.call_sym(CONSTANT_BUFFER_DTOR, Conv::Thiscall(p), &[]);
                    p = p.wrapping_add(0x3c);
                }
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[base]);
            }
        }
        h.write(cbs + 4, 0u32);
        h.write(cbs, 0u32);
        eax
    }

    /// The x87 condition bits of comparing `a` with `b` (`fcom`): C0, C2,
    /// C3 as bits 0, 2, 6 of `ah`.
    fn fcom(a: f64, b: f64) -> u8 {
        if a.is_nan() || b.is_nan() {
            0x45
        } else if a < b {
            0x01
        } else if a > b {
            0x00
        } else {
            0x40
        }
    }

    fn ld(m: &impl Mem, at: u32) -> f64 {
        let bits: u32 = m.read(at);
        x87::load(bits)
    }

    /// `*at = min(v, *at)` as the original keeps `v` (`v <= *at`, and `v`
    /// for a NaN; else `*at`), stored as `f32`.
    fn keep_low(m: &mut impl Mem, at: u32, v: f64) {
        let cur = x87::load(x87::store(ld(m, at)));
        let r = if fcom(v, cur) & 0x41 != 0 { v } else { cur };
        m.write(at, x87::store(r));
    }

    /// `*at = max(v, *at)` as the original keeps `v` (`v >= *at`, and `v`
    /// for a NaN; else `*at`), stored as `f32`.
    fn keep_high(m: &mut impl Mem, at: u32, v: f64) {
        let cur = x87::load(x87::store(ld(m, at)));
        let c = fcom(v, cur) & 0x05;
        // `jp`: an even count of the two bits keeps v
        let r = if c.count_ones().is_multiple_of(2) {
            v
        } else {
            cur
        };
        m.write(at, x87::store(r));
    }

    /// `0x00516a20` (thiscall `(&start, &end)`, `ret 8`): the animation's
    /// time range: the union of its clips' ranges (`+8`, count `+4`; each
    /// clip's controller `+0x24`, 7 when one is missing; the first clip's
    /// range into `start`/`end`, the others' through the frame, `ebp-4`
    /// and `ebp-8`), widened by its own range (`+0x14`, `+0x18`) unless
    /// that is reversed; a clip's error returned as is. 7 for no clips and
    /// a reversed own range; else 0.
    pub fn time_range(h: &mut impl Host, this: u32, start: u32, end: u32, frame: u32) -> u32 {
        let lo = frame + 4;
        let hi = frame;
        h.write(lo, 0.0f32);
        h.write(hi, 0.0f32);
        let count: u32 = h.read(this + 4);
        let n = count & !EXTERNAL;
        if n == 0 {
            let c = fcom(ld(h, this + 0x18), ld(h, this + 0x14));
            if (c & 0x05).is_power_of_two() {
                return 7;
            }
        } else {
            let clips: u32 = h.read(this + 8);
            let first: u32 = h.read(clips);
            let ctrl: u32 = h.read(first + 0x24);
            if ctrl == 0 {
                return 7;
            }
            let r = h.call_sym(CLIP_RANGE, Conv::Thiscall(ctrl), &[start, end]);
            if r != 0 {
                return r;
            }
            for i in 1..n {
                let clip: u32 = h.read(clips.wrapping_add(4 * i));
                let ctrl: u32 = h.read(clip + 0x24);
                if ctrl == 0 {
                    return 7;
                }
                let r = h.call_sym(CLIP_RANGE, Conv::Thiscall(ctrl), &[lo, hi]);
                if r != 0 {
                    return r;
                }
                let a = ld(h, lo);
                keep_low(h, start, a);
                let b = ld(h, hi);
                keep_high(h, end, b);
            }
        }
        let c = fcom(ld(h, this + 0x18), ld(h, this + 0x14));
        if c & 0x01 != 0 {
            return 0;
        }
        let a = x87::store(ld(h, this + 0x14));
        let b = x87::store(ld(h, this + 0x18));
        h.write(lo, a);
        h.write(hi, b);
        if n == 0 {
            h.write(start, a);
            h.write(end, b);
            return 0;
        }
        keep_low(h, start, x87::load(a));
        keep_high(h, end, x87::load(b));
        0
    }
}
