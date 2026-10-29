//! M5 stage 1, `PhyreEngine` list `render-a`, batch 4: the scene render
//! pass records (copies of their shader, string and pass arrays, the
//! copy-and-initialise resizes), the vertex-format decoders of
//! `PPhysicsMesh` (half floats, 11/11/10 normals, the format sizes),
//! the static teardown lists and a few small classes.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::{PHYRE_FREE, PHYRE_MALLOC_ALIGNED};
use crate::m5p_render_a3::MEMCPY;

/// The objects `FUN_00432430` hands to `0x0047fe60` (cdecl), in order.
pub const RELEASE_ALL: &[u32] = &[
    0x00c9_85b4,
    0x00c9_85c0,
    0x00c9_85cc,
    0x00c9_85d8,
    0x00c9_85e4,
    0x00c9_85f0,
    0x00c9_85fc,
    0x00c9_8608,
    0x00c9_8614,
    0x00c9_8620,
    0x00c9_862c,
    0x00c9_8644,
    0x00c9_8638,
    0x00c9_8650,
    0x00c9_865c,
    0x00c9_8668,
    0x00c9_8674,
    0x00c9_8680,
    0x00c9_868c,
    0x00c9_8698,
    0x00c9_86a4,
    0x00c9_86b0,
    0x00c9_86c8,
    0x00c9_86d4,
    0x00c9_86e0,
    0x00c9_86ec,
];

/// The list items `FUN_00504040` unlinks (`List_unlinkForward_005c9920`),
/// in order, until one returns non-zero.
pub const UNLINK_ALL: &[u32] = &[
    0x00cb_d428,
    0x00cb_d4a8,
    0x00cb_d528,
    0x00cb_d5a8,
    0x00cb_d628,
    0x00cb_d6a8,
    0x00cb_d728,
    0x00cb_d7a8,
    0x00cb_d828,
    0x00c2_9498,
    0x00c2_94d8,
    0x00c2_9518,
    0x00c2_9558,
    0x00c2_9598,
];

/// The copy-and-initialise resizes `(this; n)` (`ret 4`): (address,
/// element size, copy `(dst, src, count)`, initialiser `(dst, count)`,
/// the old block's release `(data, count)` or 0 for `Phyre_PhyreFree`
/// of a non-null block).
pub const COPY_INIT_RESIZES: &[(u32, u32, u32, u32, u32)] = &[
    (0x004e_1090, 20, 0x004d_89e0, 0x004d_f180, 0),
    (0x004e_11e0, 32, 0x004d_8bd0, 0x004d_f240, 0x004d_d770),
    (0x0056_e300, 20, 0x0056_5110, 0x0056_bab0, 0x0056_a700),
];

/// The descriptor objects `FUN_0055b490` tests the class chain for.
pub const MATRIX_SOURCE_CLASSES: [u32; 3] = [0x00c9_2090, 0x00ca_7228, 0x00ca_91f8];

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

    /// A `PString` duplicated: its text (the word less the tag bit 0)
    /// copied into `PhyreMallocAligned(len + 1, 2)` by `memcpy`; 0 for no
    /// text or no memory.
    fn dup_string(h: &mut impl Host, word: u32) -> u32 {
        let text = word - (word & 1);
        if text == 0 {
            return 0;
        }
        let mut n = 0u32;
        while h.read::<u8>(text.wrapping_add(n)) != 0 {
            n += 1;
        }
        let len = n + 1;
        let block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[len, 2]);
        if block != 0 {
            cdecl(h, MEMCPY, &[block, text, len]);
        }
        block
    }

    /// `FUN_00432430()` -> 0: `0x0047fe60(o)` for each [`RELEASE_ALL`].
    pub fn release_all(h: &mut impl Host) -> u32 {
        for &o in RELEASE_ALL {
            let o = h.rebase(o);
            cdecl(h, 0x0047_fe60, &[o]);
        }
        0
    }

    /// `FUN_00504040()`: [`UNLINK_ALL`] unlinked in order until one
    /// returns non-zero; the last `eax`.
    pub fn unlink_all(h: &mut impl Host) -> u32 {
        let mut r = 0;
        for &o in UNLINK_ALL {
            let o = h.rebase(o);
            r = cdecl(h, 0x005c_9920, &[o]);
            if r != 0 {
                break;
            }
        }
        r
    }

    /// One array of a pass record: `[count, data]` at `dst + off` from
    /// `src + off`, the elements copied into a fresh block by `copy`.
    fn copy_array(h: &mut impl Host, dst: u32, src: u32, off: u32, size: u32, copy: u32) {
        h.write(dst + off, 0u32);
        let count = h.read::<u32>(src + off) & 0x7fff_ffff;
        let data: u32 = h.read(src + off + 4);
        let mut block = 0;
        if count != 0 {
            block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[count.wrapping_mul(size), 4]);
            if block != 0 {
                cdecl(h, copy, &[block, data, count]);
            }
        }
        h.write(dst + off + 4, block);
        h.write(dst + off, if block == 0 { 0 } else { count });
    }

    /// `FUN_004d8a30(dst, src, count)` (cdecl): 32-byte scene render pass
    /// records: the word `+0`, the shaders (`+4`, 0x254-byte elements,
    /// `PhyreCopyConstruct254_004baed0`), two record arrays (`+0xc` and
    /// `+0x14`, 16 bytes, `PShaderRecord_copyArray_004a3530` / `_004a3620`),
    /// the word `+0x1c`; a destination at 0 skipped.
    pub fn pass_records_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(32 * i);
            if d == 0 {
                continue;
            }
            let w: u32 = h.read(s);
            h.write(d, w);
            copy_array(h, d, s, 4, 0x254, 0x004b_aed0);
            copy_array(h, d, s, 0xc, 16, 0x004a_3530);
            copy_array(h, d, s, 0x14, 16, 0x004a_3620);
            let w: u32 = h.read(s + 0x1c);
            h.write(d + 0x1c, w);
            s = s.wrapping_add(32);
        }
    }

    /// `FUN_004d8bd0(dst, src, count)` (cdecl): 32-byte records of four
    /// `PString`s (duplicated) and four words; a destination at 0
    /// skipped.
    pub fn pass_infos_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(32 * i);
            if d == 0 {
                continue;
            }
            for k in 0..4 {
                let w: u32 = h.read(s + 4 * k);
                let p = dup_string(h, w);
                h.write(d + 4 * k, p);
            }
            for k in 4..8 {
                let w: u32 = h.read(s + 4 * k);
                h.write(d + 4 * k, w);
            }
            s = s.wrapping_add(32);
        }
    }

    /// `FUN_004e19c0(dst, src, count)` (cdecl): 8-byte records of two
    /// `PString`s, duplicated; a destination at 0 skipped.
    pub fn string_pairs_copy(h: &mut impl Host, dst: u32, src: u32, count: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(8 * i);
            if d == 0 {
                continue;
            }
            let w: u32 = h.read(s);
            let p = dup_string(h, w);
            h.write(d, p);
            let w: u32 = h.read(s + 4);
            let p = dup_string(h, w);
            h.write(d + 4, p);
            s = s.wrapping_add(8);
        }
    }

    /// A [`COPY_INIT_RESIZES`] resize `(this; n)` -> 0 or 0xd: nothing when
    /// `n` equals the header word; a block of `n` elements (none for 0)
    /// with the kept ones copied and the new ones initialised, the old
    /// owned block released (not the new one; with `release` 0 freed
    /// when not null), `[n, block]`.
    pub fn copy_init_resize(
        h: &mut impl Host,
        this: u32,
        n: u32,
        size: u32,
        copy: u32,
        init: u32,
        release: u32,
    ) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & 0x7fff_ffff;
        let keep = count.min(n);
        let data: u32 = h.read(this + 4);
        let mut block = 0;
        if n != 0 {
            block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(size), 4]);
            if block == 0 {
                return 0xd;
            }
            cdecl(h, copy, &[block, data, keep]);
            cdecl(h, init, &[block + keep * size, n - keep]);
        }
        let old: u32 = h.read(this + 4);
        let header: u32 = h.read(this);
        if old != block && header & 0x8000_0000 == 0 {
            if release != 0 {
                cdecl(h, release, &[old, header & 0x7fff_ffff]);
            } else if old != 0 {
                cdecl(h, PHYRE_FREE, &[old]);
            }
        }
        h.write(this + 4, block);
        h.write(this, n);
        0
    }

    /// `FUN_004fef30(this)` -> this: four 16-byte zero quads
    /// (`__ehvec_ctor` 0x0094966a with `PZeroQuad_init`, destructor
    /// 0x005b3640), a fifth at `+0x40`, the 16-bit word `+0x50` cleared.
    pub fn quads_ctor(h: &mut impl Host, this: u32) -> u32 {
        let init = h.rebase(0x005b_3620);
        let fini = h.rebase(0x005b_3640);
        h.call_sym(0x0094_966a, Conv::Stdcall, &[this, 0x10, 4, init, fini]);
        this_call(h, 0x005b_3620, this + 0x40, &[]);
        h.write(this + 0x50, 0u16);
        this
    }

    /// `FUN_0049a470(h)` (cdecl, the low 16 bits) -> ST0: a half float
    /// widened: zero, a denormal (`mant * K`, K the f32 at `0x00b170ac`, or
    /// `0x00b170e0` for a negative one, rounded to f32), infinity or NaN
    /// (the fraction 1 when the half's is not 0), or a normal number.
    pub fn half_to_float(h: &mut impl Host, half: u32) -> f32 {
        let x = half & 0xffff;
        let sign = x >> 15;
        let exp = (x >> 10) & 0x1f;
        let mant = x & 0x3ff;
        let bits = match exp {
            0 if mant != 0 => {
                let k_at = h.rebase(if sign != 0 { 0x00b1_70e0 } else { 0x00b1_70ac });
                let k: u32 = h.read(k_at);
                let f = x87::store(f64::from(mant));
                return f32::from_bits(x87::store(x87::mul(x87::load(k), x87::load(f))));
            }
            0 => sign << 31,
            0x1f => (sign << 31) | (0xff << 23) | u32::from(mant != 0),
            _ => (sign << 31) | ((exp + 0x70) << 23) | (mant << 13),
        };
        f32::from_bits(bits)
    }

    /// `FUN_0049bbe0(this; px, py, pz)` (`ret 0xc`) -> `pz`: the packed
    /// normal `*this` (x 11 bits, y 11 bits, z 10 bits, signed) scaled:
    /// x and y over the f64 at `0x00b170d0`, z over the one at
    /// `0x00b170c8`, each rounded to f32.
    pub fn unpack_normal(h: &mut impl Host, this: u32, px: u32, py: u32, pz: u32) -> u32 {
        let w: u32 = h.read(this);
        let y = ((w << 10) as i32) >> 21;
        let z = (w as i32) >> 22;
        let x = ((w << 21) as i32) >> 21;
        let a = h.rebase(0x00b1_70d0);
        let d1 = f64::from_bits(h.read(a));
        let a = h.rebase(0x00b1_70c8);
        let d2 = f64::from_bits(h.read(a));
        h.write(px, x87::store(x87::div(f64::from(x), d1)));
        h.write(py, x87::store(x87::div(f64::from(y), d1)));
        h.write(pz, x87::store(x87::div(f64::from(z), d2)));
        pz
    }

    /// `FUN_0055bb40(format)` (cdecl): the byte size of a vertex element
    /// format (the jump table at `0x0055bbd0` through the index bytes at
    /// `0x0055bc04`): 0 for 0..3, 48, 49 and past 50; `format & !3` for
    /// 4..47; 0x33 for 50.
    #[must_use]
    pub fn format_size(format: u32) -> u32 {
        match format {
            4..=47 => format & !3,
            50 => 0x33,
            _ => 0,
        }
    }

    /// `FUN_0049bec0(this; arr)` (`ret 4`): a stream of format 2 returns
    /// its data `+0x14` as it is; otherwise `arr` (none: 0) sized to 3
    /// words per element (`PArray_resizeDiscard_0049bfc0`, a failure: 0)
    /// and filled by `FUN_0049a520(data, [+0x14], format, [+0xc])`; its
    /// block.
    pub fn decode_stream(h: &mut impl Host, this: u32, arr: u32) -> u32 {
        let count: u32 = h.read(this + 0xc);
        let format: u8 = h.read(this + 0x18);
        if format == 2 {
            return h.read(this + 0x14);
        }
        if arr == 0 {
            return 0;
        }
        if this_call(h, 0x0049_bfc0, arr, &[count.wrapping_mul(3)]) != 0 {
            return 0;
        }
        let data: u32 = h.read(arr + 4);
        let src: u32 = h.read(this + 0x14);
        let count: u32 = h.read(this + 0xc);
        let format: u8 = h.read(this + 0x18);
        cdecl(h, 0x0049_a520, &[data, src, u32::from(format), count]);
        data
    }

    /// `FUN_00567590(this)`: the owned array at `+0xc` reset (freed when
    /// not null), then the `PString` `+0` freed unless tagged.
    pub fn animation_info_char_dtor(h: &mut impl Host, this: u32) {
        crate::m5p_render_a::bridge::reset_owned_nonnull(h, this, 0xc);
        let s: u32 = h.read(this);
        if s & 1 == 0 {
            cdecl(h, PHYRE_FREE, &[s]);
        }
    }

    /// Whether the class chain from `desc` (parents at `+0x40`) holds
    /// `class`.
    fn is_a(h: &mut impl Host, desc: u32, class: u32) -> bool {
        let class = h.rebase(class);
        let mut d = desc;
        while d != 0 {
            if d == class {
                return true;
            }
            d = h.read(d + 0x40);
        }
        false
    }

    /// `FUN_0055b490(this; out)` (`ret 4`) -> out: the world matrix of the
    /// object `+0x60` by its class (`+0x64`): a plain matrix holder copies
    /// it (`0x00451080(out; m)`), a node asks `0x004f71e0(node; out,
    /// [+0x5c])`, a scene node (`+0x60` - 4) computes it
    /// (`FUN_00506660(that; out)`); otherwise the identity
    /// (`0x00451bb0(out)`).
    pub fn object_matrix(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let desc: u32 = h.read(this + 0x64);
        let obj: u32 = h.read(this + 0x60);
        if is_a(h, desc, MATRIX_SOURCE_CLASSES[0]) && obj != 0 {
            this_call(h, 0x0045_1080, out, &[obj]);
            return out;
        }
        if is_a(h, desc, MATRIX_SOURCE_CLASSES[1]) && obj != 0 {
            let k: u32 = h.read(this + 0x5c);
            this_call(h, 0x004f_71e0, obj, &[out, k]);
            return out;
        }
        if is_a(h, desc, MATRIX_SOURCE_CLASSES[2]) && obj != 0 && obj.wrapping_sub(4) != 0 {
            this_call(h, 0x0050_6660, obj - 4, &[out]);
            return out;
        }
        cdecl(h, 0x0045_1bb0, &[out]);
        out
    }
}
