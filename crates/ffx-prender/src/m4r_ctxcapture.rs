//! `PRendererBase::generateContextParameterCaptureBuffer` (`0x005b6cf0`,
//! list p6-render-c; `m4-rest`): the per-context shader parameters of one
//! draw (lights, shadow casters, the mesh instance's data blocks) written
//! into a parameter capture buffer.
//!
//! `thiscall(renderer; ctx, lights, extra)`, `ret 0xc` -> the buffer, or 0
//! when the ring (`renderer +0x1c`) has no room. `ctx`: `+0xc` an index
//! into `[+0x10] +0x20`'s 12-byte array (semantic `0xe3`), `+0x1c` the
//! mesh instance (its data blocks: semantics `0xde..0xe2`), `+0x28` the
//! node's matrix block, `+0x6c` the parameter layout (`+0xc` the count,
//! bit 31 a flag; `+0x10` 16-byte entries: `+0` a u16 count, `+2` the
//! semantic `0xc0..0xe3`, `+3` the format (`0x30` a 3x4), `+8` the u16
//! offset, `+0xa` the size (13 bits) and the light slot (top 3 bits);
//! `+0x1c` the buffer's size). `lights`: eight light pointers by slot, the
//! first four also by index with their count at `+0x10`. `extra` is added
//! to the size the ring allocates.
//!
//! An entry whose slot holds a light takes its value from the light (its
//! `+0x10` transform, or the identity's 3x4 without one; `+0x14` its
//! shadow caster; `+0x1c`/`+0x20` the cone's angles; `+0x24` its
//! intensity; `+0x28`/`+0x2c` its range), the others from the context or
//! defaults. Each value is copied to `buffer + offset` when it fits and
//! the offset is at least 4; the skin-like matrix arrays (`0xd7`) are
//! written in place.
//!
//! The frame: the bridge takes the original's `ebp` and keeps every local
//! where the original does, temporaries included: a value's size can
//! reach past the block it is copied from (the shim's frame starts zeroed;
//! junk read: a word the original never wrote before such a copy reads 0
//! there, brief 2026-10-24).

use ffx_bridge::x87::{self, Reg};
use ffx_graphic::host::{Conv, Host};

/// `PMatrix4_toWorldMatrix3x4` (thiscall `(dst, m)`, `ret 4`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
/// A segmented ring's `allocate` (thiscall `(ring, size)`, `ret 4`).
pub const ALLOCATE: u32 = 0x0061_3e10;
/// `Vectormath_Vector4_scaleTo_004f30d0` (thiscall `(v, out, s)`, `ret 8`)
/// -> out.
pub const SCALE_TO: u32 = 0x004f_30d0;
/// `PLight_isRangeInfinite_005b8d80` (thiscall `(light)`) -> `al`.
pub const RANGE_INFINITE: u32 = 0x005b_8d80;
/// `Phyre_maxFloat_005126e0` (cdecl `(a, b)` floats) -> `ST0`.
pub const MAX_FLOAT: u32 = 0x0051_26e0;
/// `_CIcos`'s import thunk.
pub const CI_COS: u32 = 0x0094_98c2;
/// `PMatrix4_Rotate3_004ca140` (thiscall `(m, out, v)`, `ret 8`) -> out.
pub const ROTATE3: u32 = 0x004c_a140;
/// `Vector3_normalize_004525a0` (cdecl `(out, v)`) -> out.
pub const NORMALIZE: u32 = 0x0045_25a0;
/// `PMatrix4_TransformPoint3_004c9eb0` (thiscall `(m, out, p)`, `ret 8`)
/// -> out.
pub const TRANSFORM_POINT: u32 = 0x004c_9eb0;
/// `PMatrix4_Copy_00453340` (thiscall `(out, m)`, `ret 4`).
pub const COPY: u32 = 0x0045_3340;
/// `PMatrix4_Copy_004c9430` (thiscall `(out, m)`, `ret 4`).
pub const COPY4: u32 = 0x004c_9430;
/// `PMatrix4_identity_00454cf0` (cdecl `(out)`) -> out.
pub const IDENTITY: u32 = 0x0045_4cf0;
/// `Phyre_PRendering_bufferSlice_005b8b00` (thiscall `(buffer, entry)`,
/// `ret 4`) -> the entry's place in the buffer.
pub const SLICE: u32 = 0x005b_8b00;
/// `PShadowCaster_splitDistances_005b8c70` (thiscall `(caster, out)`,
/// `ret 4`) -> out.
pub const SPLIT_DISTANCES: u32 = 0x005b_8c70;
/// `PRendererBase_lightAttenuation_005b5ef0` (cdecl `(&a, &b, &c, &d,
/// light)`).
pub const ATTENUATION: u32 = 0x005b_5ef0;
/// `PArray12_at_005a7770` (thiscall `(array, index)`, `ret 4`) -> the
/// element's address.
pub const ARRAY12_AT: u32 = 0x005a_7770;
/// `PMeshInstance_findDataBlock_00489150` (thiscall `(instance, &out,
/// semantic)`, `ret 8`) -> nonzero when found.
pub const FIND_DATA_BLOCK: u32 = 0x0048_9150;
/// `Phyre_PhyrePrintf` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// The data-block semantics of `0xde..0xe1` (image addresses the lookup
/// compares).
pub const BLOCK_SEMANTICS: [u32; 4] = [0x00c9_8620, 0x00c9_862c, 0x00c9_8650, 0x00c9_8638];
/// The "no shadow map for this split" warning's once-flag (byte).
pub const NO_MAP_WARNED: u32 = 0x00cb_bf5d;
/// The warnings' texts.
pub const SPLIT_WARNING: u32 = 0x00b3_66e0;
pub const NO_MAP_WARNING: u32 = 0x00b3_6734;
pub const NO_BLOCK_WARNING: u32 = 0x00b3_6770;
pub const NO_INDEX_BLOCK_WARNING: u32 = 0x00b3_67b8;
/// The constants: the cone's epsilon (double), the range's floor and the
/// cone's floor (floats), the infinite range's four values.
pub const CONE_EPSILON: u32 = 0x00b3_68a8;
pub const RANGE_FLOOR: u32 = 0x00b3_68a0;
pub const CONE_FLOOR: u32 = 0x00b3_68a4;
pub const INFINITE_RANGE: [u32; 2] = [0x00b9_232c, 0x00b1_0f50];
pub const INFINITE_SCALE: u32 = 0x00b3_68b8;
/// The frame's size below `ebp`.
pub const FRAME: u32 = 0x3ec;

const ONE: u32 = 0x3f80_0000;

/// A float copied through the x87 stack (`fld dword`, `fstp dword`): a
/// signalling NaN comes out quiet.
#[must_use]
pub fn through_x87(bits: u32) -> u32 {
    x87::store(x87::load(bits))
}

/// The 3x4 a 4x4 `m` stores (its rows 0..2, read down the columns): `m0 m1
/// m2 m4 | m5 m6 m8 m9 | m10 m12 m13 m14`.
#[must_use]
pub fn rows_3x4(m: &[u32; 16]) -> [u32; 12] {
    [
        m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10], m[12], m[13], m[14],
    ]
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        ALLOCATE, ARRAY12_AT, ATTENUATION, BLOCK_SEMANTICS, CI_COS, CONE_EPSILON, CONE_FLOOR, COPY,
        COPY4, Conv, FIND_DATA_BLOCK, Host, IDENTITY, INFINITE_RANGE, INFINITE_SCALE, MAX_FLOAT,
        NO_BLOCK_WARNING, NO_INDEX_BLOCK_WARNING, NO_MAP_WARNED, NO_MAP_WARNING, NORMALIZE, ONE,
        PRINTF, RANGE_FLOOR, RANGE_INFINITE, ROTATE3, Reg, SCALE_TO, SLICE, SPLIT_DISTANCES,
        SPLIT_WARNING, TO_WORLD_3X4, TRANSFORM_POINT, rows_3x4, through_x87, x87,
    };

    /// The frame at the original's `ebp`.
    struct Frame(u32);

    impl Frame {
        fn at(&self, off: u32) -> u32 {
            self.0.wrapping_sub(off)
        }
        fn get(&self, h: &impl Host, off: u32) -> u32 {
            h.read(self.at(off))
        }
        fn set(&self, h: &mut impl Host, off: u32, w: u32) {
            h.write(self.at(off), w);
        }
        /// `M`, the 4x4 at `ebp - 0xc0` (identity at the start; the copies
        /// land there).
        fn matrix(&self, h: &impl Host) -> [u32; 16] {
            let mut m = [0; 16];
            for (k, w) in m.iter_mut().enumerate() {
                *w = self.get(h, 0xc0 - 4 * k as u32);
            }
            m
        }
    }

    // the frame's blocks
    /// The value vector (`ebp - 0x60`, four floats; kept across entries).
    const VALUE: u32 = 0x60;
    /// The attenuation vectors (four lights each): `a`, `b` (starts at
    /// ones), `c`, `d` (ones).
    const ATT_A: u32 = 0x24;
    const ATT_B: u32 = 0x44;
    const ATT_C: u32 = 0x34;
    const ATT_D: u32 = 0x14;
    /// `M`.
    const MATRIX: u32 = 0xc0;
    /// The shadow-map record `{0x42, width, caster + 0x44, map}`.
    const SHADOW_MAP: u32 = 0xe0;
    /// The data-block record `{0xde, 0, 0, block}`.
    const DATA_BLOCK: u32 = 0x148;
    /// The index-block record `{0xe2, 0, 0, block + 0x30}`.
    const INDEX_BLOCK: u32 = 0x15c;
    /// The identity's world 3x4 (the transform of a light without one).
    const WORLD: u32 = 0x3ac;
    /// The "attenuation computed" byte.
    const ATT_DONE: u32 = 0x61;

    /// What every entry reads.
    struct Ctx {
        renderer: u32,
        buffer: u32,
        /// `fld qword [0x00b368a8]`, kept on the x87 stack across entries.
        epsilon: f64,
    }

    fn set4(h: &mut impl Host, f: &Frame, off: u32, w: [u32; 4]) {
        for (k, x) in w.into_iter().enumerate() {
            f.set(h, off - 4 * k as u32, x);
        }
    }

    /// `v` through `ebp - 0x48`, then into all four lanes of the value.
    fn splat(h: &mut impl Host, f: &Frame, v: u32) -> u32 {
        let v = through_x87(v);
        f.set(h, 0x48, v);
        set4(h, f, VALUE, [v; 4]);
        VALUE
    }

    /// Three words at `p` through the x87 into `tmp`, then into the
    /// value's xyz, `w` its last lane.
    fn xyz_value(h: &mut impl Host, f: &Frame, p: u32, tmp: u32, w: u32) -> u32 {
        for k in 0..3 {
            let x = through_x87(h.read(p + 4 * k));
            f.set(h, tmp - 4 * k, x);
        }
        for k in 0..3 {
            let x = f.get(h, tmp - 4 * k);
            f.set(h, VALUE - 4 * k, x);
        }
        f.set(h, VALUE - 0xc, w);
        VALUE
    }

    /// Three words of the transform at `+from` through the x87 into the
    /// frame at `ebp - to`.
    fn take3(h: &mut impl Host, f: &Frame, transform: u32, from: u32, to: u32) {
        for k in 0..3 {
            let x = through_x87(h.read(transform + from + 4 * k));
            f.set(h, to - 4 * k, x);
        }
    }

    fn copy3(h: &mut impl Host, f: &Frame, from: u32, to: u32) {
        for k in 0..3 {
            let x = f.get(h, from - 4 * k);
            f.set(h, to - 4 * k, x);
        }
    }

    fn cos(h: &mut impl Host, bits: u32) -> u32 {
        let addr = h.rebase(CI_COS);
        h.call_intrinsic(addr, x87::load(bits)).to_bits()
    }

    fn max_float(h: &mut impl Host, a: u32, b: u32) -> f64 {
        let r = h.call_sym_st0(MAX_FLOAT, Conv::Cdecl, &[a, b]);
        x87::load(r.to_bits())
    }

    fn printf(h: &mut impl Host, text: u32, args: &[u32]) {
        let text = h.rebase(text);
        let mut all = vec![1, text];
        all.extend_from_slice(args);
        h.call_sym(PRINTF, Conv::Cdecl, &all);
    }

    /// The four lights' attenuation vectors (`0xcf..0xd2`), computed on
    /// the first entry that asks; the vector's block.
    fn attenuation(h: &mut impl Host, f: &Frame, semantic: u32) -> u32 {
        if h.read::<u8>(f.at(ATT_DONE)) == 0 {
            let lights = f.get(h, 0x74);
            // the vectors' distances from the lights table, kept in the frame
            for (off, block) in [(0x68, ATT_A), (0x50, ATT_C), (0x70, ATT_B)] {
                f.set(h, off, f.at(block).wrapping_sub(lights));
            }
            for i in 0..4 {
                let count: u32 = h.read(f.get(h, 0x74) + 0x10);
                if count < i + 1 {
                    continue;
                }
                let light: u32 = h.read(lights + 4 * i);
                if light != 0 {
                    let args = [
                        f.at(ATT_A) + 4 * i,
                        f.at(ATT_B) + 4 * i,
                        f.at(ATT_C) + 4 * i,
                        f.at(ATT_D) + 4 * i,
                        light,
                    ];
                    h.call_sym(ATTENUATION, Conv::Cdecl, &args);
                }
            }
            h.write(f.at(ATT_DONE), 1_u8);
        }
        match semantic.wrapping_sub(0xcf) {
            0 => ATT_A,
            1 => ATT_B,
            2 => ATT_C,
            3 => ATT_D,
            _ => VALUE,
        }
    }

    /// The shared `0xe3`: element `[ctx + 0xc]` of `[ctx + 0x10] + 0x20`'s
    /// array, or 1.0 without one, in all four lanes.
    fn context_scalar(h: &mut impl Host, f: &Frame) -> u32 {
        let holder = f.get(h, 0xc4);
        f.set(h, 0x48, ONE);
        let array: u32 = h.read(holder + 0x20);
        if array != 0 {
            let index: u32 = h.read(f.get(h, 0x80) + 0xc);
            let p = h.call_sym(ARRAY12_AT, Conv::Thiscall(array), &[index]);
            let v = through_x87(h.read(p));
            f.set(h, 0x48, v);
        }
        let v = f.get(h, 0x48);
        set4(h, f, VALUE, [v; 4]);
        VALUE
    }

    /// The cone (`0xc8`): `[inner, max(outer - inner, 0.001) + inner,
    /// cos inner, cos inner - max(cos inner - cos outer, 0.001)]`.
    fn cone(h: &mut impl Host, f: &Frame, light: u32) -> u32 {
        let a = through_x87(h.read(light + 0x1c));
        f.set(h, 0x6c, a);
        let b = through_x87(h.read(light + 0x20));
        f.set(h, 0x70, b);
        let ca = cos(h, a);
        f.set(h, 0x48, ca);
        let floor = through_x87(h.read(h.rebase(CONE_FLOOR)));
        let span = Reg::load(f32::from_bits(b))
            .sub_m32(f32::from_bits(a))
            .store()
            .to_bits();
        f.set(h, 0x50, span);
        let m1 = max_float(h, span, floor);
        let outer = Reg::constant(m1)
            .add_m32(f32::from_bits(a))
            .store()
            .to_bits();
        f.set(h, 0x68, outer);
        let ca64 = x87::load(ca);
        h.write(f.at(0x1fc), ca64.to_bits());
        let cb = cos(h, b);
        f.set(h, 0x50, cb);
        let d = x87::store(x87::sub(ca64, x87::load(cb)));
        f.set(h, 0x50, d);
        let m2 = max_float(h, d, floor);
        let low = x87::store(x87::sub(ca64, m2));
        f.set(h, 0x50, low);
        set4(h, f, VALUE, [a, outer, ca, low]);
        VALUE
    }

    /// The range (`0xc3`): `[near, far, 1/max(far - near, 1e-6), -near/
    /// ...]`, or the infinite range's constants.
    fn range(h: &mut impl Host, f: &Frame, light: u32) -> u32 {
        let infinite = h.call_sym(RANGE_INFINITE, Conv::Thiscall(light), &[]);
        if infinite & 0xff != 0 {
            let mut w = [0_u32; 4];
            w[0] = through_x87(h.read(h.rebase(INFINITE_RANGE[0])));
            w[1] = through_x87(h.read(h.rebase(INFINITE_RANGE[1])));
            w[3] = through_x87(h.read(h.rebase(INFINITE_SCALE)));
            set4(h, f, VALUE, w);
            return VALUE;
        }
        let floor = through_x87(h.read(h.rebase(RANGE_FLOOR)));
        let near: u32 = h.read(light + 0x28);
        let far: u32 = h.read(light + 0x2c);
        let span = Reg::load(f32::from_bits(far))
            .sub_m32(f32::from_bits(near))
            .store()
            .to_bits();
        f.set(h, 0x48, span);
        let m = max_float(h, span, floor);
        let inv = x87::store(x87::div(1.0, m));
        f.set(h, 0x48, inv);
        let near = through_x87(near);
        f.set(h, 0x1f4, near);
        f.set(h, 0x1f0, through_x87(far));
        let scaled = x87::store(x87::mul(-x87::load(near), x87::load(inv)));
        f.set(h, 0x1e8, scaled);
        let far = f.get(h, 0x1f0);
        set4(h, f, VALUE, [near, far, inv, scaled]);
        VALUE
    }

    /// The 3x4 rows of `M` into the frame at `ebp - 0x26c` (`0xd3..0xd6`
    /// with format `0x30`), through the original's temporaries.
    fn matrix_rows(h: &mut impl Host, f: &Frame) -> u32 {
        let m = f.matrix(h).map(through_x87);
        f.set(h, 0x2e0, m[4]);
        set4(h, f, 0x26c, [m[0], m[1], m[2], m[4]]);
        for (off, x) in [
            (0x164, m[9]),
            (0x48, m[9]),
            (0xf0, m[8]),
            (0x50, m[8]),
            (0x120, m[6]),
            (0x68, m[6]),
            (0xf8, m[5]),
        ] {
            f.set(h, off, x);
        }
        set4(h, f, 0x25c, [m[5], m[6], m[8], m[9]]);
        for (off, x) in [
            (0x138, m[14]),
            (0x48, m[14]),
            (0x100, m[13]),
            (0x50, m[13]),
            (0x128, m[12]),
            (0x68, m[12]),
            (0x108, m[10]),
        ] {
            f.set(h, off, x);
        }
        set4(h, f, 0x24c, [m[10], m[12], m[13], m[14]]);
        0x26c
    }

    fn write_rows(h: &mut impl Host, out: u32, m: &[u32; 16]) {
        for (k, x) in rows_3x4(m).into_iter().enumerate() {
            h.write(out + 4 * k as u32, x);
        }
    }

    /// The caster's split matrices (`0xd7` with a light): `[entry]` of
    /// them (at most the caster's count) written in place, as 3x4s
    /// (format `0x30`) or 4x4s.
    fn split_matrices(h: &mut impl Host, f: &Frame, c: &Ctx, light: u32, entry: u32) {
        let caster: u32 = h.read(light + 0x14);
        f.set(h, 0x48, caster);
        let count = u32::from(h.read::<u16>(entry)).min(h.read::<u32>(caster + 8) & 0x7fff_ffff);
        let format = h.read::<u8>(entry + 3);
        let out = h.call_sym(SLICE, Conv::Thiscall(c.buffer), &[entry]);
        for i in 0..count {
            let splits: u32 = h.read(f.get(h, 0x48) + 0xc);
            let src = splits.wrapping_add(0x80).wrapping_add(i * 0xe0);
            if format == 0x30 {
                h.call_sym(COPY, Conv::Thiscall(f.at(MATRIX)), &[src]);
                let m = f.matrix(h).map(through_x87);
                for (off, x) in [
                    (0x104, m[9]),
                    (0x70, m[9]),
                    (0x10c, m[8]),
                    (0x50, m[8]),
                    (0x114, m[6]),
                    (0x68, m[6]),
                    (0x11c, m[5]),
                    (0x124, m[14]),
                    (0x70, m[14]),
                    (0x12c, m[13]),
                    (0x50, m[13]),
                    (0x134, m[12]),
                    (0x68, m[12]),
                    (0x14c, m[10]),
                ] {
                    f.set(h, off, x);
                }
                write_rows(h, out.wrapping_add(i * 0x30), &m);
            } else {
                h.call_sym(COPY4, Conv::Thiscall(f.at(0x23c)), &[src]);
                let dst = out.wrapping_add(i << 6);
                for k in 0..16 {
                    let x = through_x87(f.get(h, 0x23c - 4 * k));
                    h.write(dst + 4 * k, x);
                }
            }
        }
    }

    /// The same without a light: `[entry]` identities.
    fn identities(h: &mut impl Host, f: &Frame, c: &Ctx, entry: u32) {
        let count = u32::from(h.read::<u16>(entry));
        let format = h.read::<u8>(entry + 3);
        let out = h.call_sym(SLICE, Conv::Thiscall(c.buffer), &[entry]);
        if format == 0x30 {
            let id = h.call_sym(IDENTITY, Conv::Cdecl, &[f.at(0x23c)]);
            h.call_sym(COPY, Conv::Thiscall(f.at(MATRIX)), &[id]);
            if count == 0 {
                return;
            }
            let m = f.matrix(h).map(through_x87);
            for (off, x) in [
                (0x160, m[9]),
                (0x68, m[9]),
                (0x110, m[8]),
                (0x50, m[8]),
                (0x130, m[6]),
                (0x48, m[6]),
                (0x118, m[5]),
                (0x2cc, m[5]),
                (0xe8, m[14]),
                (0x4c, m[14]),
                (0xec, m[13]),
                (0x6c, m[13]),
                (0xf4, m[12]),
                (0x70, m[12]),
                (0xfc, m[10]),
                (0x2dc, m[10]),
            ] {
                f.set(h, off, x);
            }
            for i in 0..count {
                write_rows(h, out.wrapping_add(i * 0x30), &m);
            }
        } else {
            for i in 0..count {
                let dst = out.wrapping_add(i << 6);
                for k in 0..16 {
                    h.write(dst + 4 * k, if k % 5 == 0 { ONE } else { 0 });
                }
            }
        }
    }

    /// The light's shadow caster's split `index` (`0xd3..0xd6`): its
    /// matrix as a 4x4, or as the 3x4 for format `0x30`; a split the
    /// caster lacks warns and leaves the value.
    fn split_matrix(h: &mut impl Host, f: &Frame, light: u32, entry: u32, index: u32) -> u32 {
        let caster: u32 = h.read(light + 0x14);
        if caster == 0 {
            return VALUE;
        }
        let count = h.read::<u32>(caster + 8) & 0x7fff_ffff;
        if index >= count {
            printf(h, SPLIT_WARNING, &[index, count]);
            return VALUE;
        }
        let splits: u32 = h.read(caster + 0xc);
        let src = splits.wrapping_add(0x80).wrapping_add(index * 0xe0);
        h.call_sym(COPY, Conv::Thiscall(f.at(MATRIX)), &[src]);
        if h.read::<u8>(entry + 3) != 0x30 {
            return MATRIX;
        }
        matrix_rows(h, f)
    }

    /// The caster's shadow map `index` (`0xd9..0xdc`) into the record.
    fn shadow_map(h: &mut impl Host, f: &Frame, light: u32, index: u32) -> u32 {
        let caster: u32 = h.read(light + 0x14);
        if caster == 0 {
            return VALUE;
        }
        let count = h.read::<u32>(caster + 8) & 0x7fff_ffff;
        let mut map = 0;
        if index < count {
            let splits: u32 = h.read(caster + 0xc);
            let target: u32 = h.read(splits.wrapping_add(index * 0xe0).wrapping_add(0xc0));
            if target == 0 {
                let warned = h.rebase(NO_MAP_WARNED);
                if h.read::<u8>(warned) == 0 {
                    h.write(warned, 1_u8);
                    printf(h, NO_MAP_WARNING, &[]);
                }
            } else if h.read::<u8>(target + 4) == 0 {
                map = h.read(target);
            }
        }
        let width = if map == 0 { 0 } else { h.read(map + 8) };
        f.set(h, SHADOW_MAP - 4, width);
        f.set(h, SHADOW_MAP - 0xc, map);
        f.set(h, SHADOW_MAP - 8, caster.wrapping_add(0x44));
        SHADOW_MAP
    }

    /// The instance's data block for `0xde..0xe1` into the record; `warn`
    /// (without a light) prints when it is missing.
    fn data_block(h: &mut impl Host, f: &Frame, semantic: u32, warn: bool) -> u32 {
        let out = if warn { 0x6c } else { 0x48 };
        f.set(h, out, 0);
        let instance: u32 = h.read(f.get(h, 0x80) + 0x1c);
        let mut block = 0;
        if instance != 0 {
            let key = h.rebase(BLOCK_SEMANTICS[(semantic - 0xde) as usize]);
            let found = h.call_sym(FIND_DATA_BLOCK, Conv::Thiscall(instance), &[f.at(out), key]);
            if found == 0 {
                if warn {
                    f.set(h, out, 0);
                }
            } else {
                block = f.get(h, out);
            }
        }
        if warn && block == 0 {
            printf(h, NO_BLOCK_WARNING, &[]);
            block = f.get(h, out);
        }
        f.set(h, DATA_BLOCK - 0xc, block);
        DATA_BLOCK
    }

    /// The instance's index block (`0xe2`) into the record.
    fn index_block(h: &mut impl Host, f: &Frame, warn: bool) -> u32 {
        let instance: u32 = h.read(f.get(h, 0x80) + 0x1c);
        let block = if instance == 0 {
            0
        } else {
            instance.wrapping_add(0x30)
        };
        if warn && block == 0 {
            printf(h, NO_INDEX_BLOCK_WARNING, &[]);
        }
        f.set(h, INDEX_BLOCK - 0xc, block);
        INDEX_BLOCK
    }

    /// An entry whose slot holds `light`: the block its value is copied
    /// from, or `None` (written in place).
    fn lit(h: &mut impl Host, f: &Frame, c: &Ctx, light: u32, entry: u32) -> Option<u32> {
        let mut transform: u32 = h.read(light + 0x10);
        if transform == 0 {
            transform = f.at(WORLD);
        }
        let semantic = u32::from(h.read::<u8>(entry + 2));
        f.set(h, 0x6c, semantic);
        if semantic == 0xc0 {
            // the light object itself
            return Some(light);
        }
        let node = f.get(h, 0xcc);
        let world_lights = c.renderer.wrapping_add(0xa0);
        lit_value(
            h,
            f,
            c,
            light,
            entry,
            semantic,
            transform,
            node,
            world_lights,
        )
        .map(|off| f.at(off))
    }

    #[allow(clippy::too_many_arguments)]
    fn lit_value(
        h: &mut impl Host,
        f: &Frame,
        c: &Ctx,
        light: u32,
        entry: u32,
        semantic: u32,
        transform: u32,
        node: u32,
        world_lights: u32,
    ) -> Option<u32> {
        match semantic {
            0xc1 => {
                let v: u32 = h.read(light + 0x24);
                Some(splat(h, f, v))
            }
            0xc2 => {
                let s = through_x87(h.read(light + 0x24));
                let v = h.call_sym(SCALE_TO, Conv::Thiscall(light), &[f.at(0x34c), s]);
                let mut w = [0; 4];
                for (k, x) in w.iter_mut().enumerate() {
                    *x = through_x87(h.read(v + 4 * k as u32));
                }
                set4(h, f, VALUE, w);
                Some(VALUE)
            }
            0xc3 => Some(range(h, f, light)),
            0xc4..=0xc7 => {
                let x: u32 = h.read(light + if semantic & 1 == 0 { 0x1c } else { 0x20 });
                let d = Reg::constant(c.epsilon);
                let v = if semantic & 1 == 0 {
                    d.subr_m32(f32::from_bits(x))
                } else {
                    d.add_m32(f32::from_bits(x))
                };
                let mut v = v.store().to_bits();
                if semantic >= 0xc6 {
                    f.set(h, 0x48, v);
                    v = cos(h, v);
                }
                Some(splat(h, f, v))
            }
            0xc8 => Some(cone(h, f, light)),
            0xc9 | 0xcd => {
                // the transform's direction rotated, normalized
                let (this, input, out, norm, tmp) = if semantic == 0xc9 {
                    (node, 0x27c, 0x35c, 0x31c, 0x1d4)
                } else {
                    (world_lights, 0x2bc, 0x32c, 0x33c, 0x194)
                };
                take3(h, f, transform, 0x10, input);
                let p = h.call_sym(ROTATE3, Conv::Thiscall(this), &[f.at(out), f.at(input)]);
                let n = h.call_sym(NORMALIZE, Conv::Cdecl, &[f.at(norm), p]);
                Some(xyz_value(h, f, n, tmp, 0))
            }
            0xca | 0xce => {
                // the transform's position transformed
                let (this, first, input, out, tmp) = if semantic == 0xca {
                    (node, 0x174, 0x28c, 0x37c, 0x1b4)
                } else {
                    (world_lights, 0x1a4, 0x2ac, 0x2fc, 0x184)
                };
                take3(h, f, transform, 0x20, first);
                copy3(h, f, first, input);
                let p = h.call_sym(
                    TRANSFORM_POINT,
                    Conv::Thiscall(this),
                    &[f.at(out), f.at(input)],
                );
                Some(xyz_value(h, f, p, tmp, ONE))
            }
            0xcb => {
                take3(h, f, transform, 0x10, 0x29c);
                let n = h.call_sym(NORMALIZE, Conv::Cdecl, &[f.at(0x36c), f.at(0x29c)]);
                Some(xyz_value(h, f, n, 0x1c4, 0))
            }
            0xcc => {
                take3(h, f, transform, 0x20, 0x1e4);
                copy3(h, f, 0x1e4, VALUE);
                f.set(h, VALUE - 0xc, ONE);
                Some(VALUE)
            }
            0xcf..=0xd2 => Some(attenuation(h, f, semantic)),
            0xd3..=0xd6 => Some(split_matrix(h, f, light, entry, semantic - 0xd3)),
            0xd7 => {
                split_matrices(h, f, c, light, entry);
                None
            }
            0xd8 => {
                let caster: u32 = h.read(light + 0x14);
                if caster != 0 {
                    let p = h.call_sym(SPLIT_DISTANCES, Conv::Thiscall(caster), &[f.at(0x30c)]);
                    let mut w = [0; 4];
                    for (k, x) in w.iter_mut().enumerate() {
                        *x = through_x87(h.read(p + 4 * k as u32));
                    }
                    set4(h, f, VALUE, w);
                }
                Some(VALUE)
            }
            0xd9..=0xdc => Some(shadow_map(h, f, light, semantic - 0xd9)),
            0xde..=0xe1 => Some(data_block(h, f, semantic, false)),
            0xe2 => Some(index_block(h, f, false)),
            0xe3 => Some(context_scalar(h, f)),
            // 0xdd and the rest: the value as the last entry left it
            _ => Some(VALUE),
        }
    }

    /// An entry whose slot holds no light.
    fn unlit(h: &mut impl Host, f: &Frame, c: &Ctx, entry: u32) -> Option<u32> {
        let semantic = u32::from(h.read::<u8>(entry + 2));
        f.set(h, 0x6c, semantic.wrapping_sub(0xcf));
        match semantic {
            0xcf..=0xd2 => Some(attenuation(h, f, semantic)),
            0xd3..=0xd6 => {
                let id = h.call_sym(IDENTITY, Conv::Cdecl, &[f.at(0x3ec)]);
                h.call_sym(COPY, Conv::Thiscall(f.at(MATRIX)), &[id]);
                Some(MATRIX)
            }
            0xd7 => {
                identities(h, f, c, entry);
                None
            }
            0xd9..=0xdc => {
                f.set(h, SHADOW_MAP - 0xc, 0);
                f.set(h, SHADOW_MAP - 4, 0);
                Some(SHADOW_MAP)
            }
            0xde..=0xe1 => Some(data_block(h, f, semantic, true)),
            0xe2 => Some(index_block(h, f, true)),
            0xe3 => Some(context_scalar(h, f)),
            _ => {
                set4(h, f, VALUE, [0; 4]);
                Some(VALUE)
            }
        }
        .map(|off| f.at(off))
    }

    /// `generateContextParameterCaptureBuffer(renderer; ctx, lights,
    /// extra)` with the original's frame at `ebp`: see the module.
    pub fn generate(
        h: &mut impl Host,
        renderer: u32,
        ctx: u32,
        lights: u32,
        extra: u32,
        ebp: u32,
    ) -> u32 {
        let f = &Frame(ebp);
        // M = identity
        for k in 0..16 {
            f.set(h, MATRIX - 4 * k, if k % 5 == 0 { ONE } else { 0 });
        }
        f.set(h, 0x74, lights);
        let holder: u32 = h.read(ctx + 0x10);
        f.set(h, 0xc4, holder);
        let layout: u32 = h.read(ctx + 0x6c);
        f.set(h, 0xcc, ctx.wrapping_add(0x28));
        f.set(h, 0x80, ctx);
        f.set(h, 0xc8, layout);
        h.call_sym(TO_WORLD_3X4, Conv::Thiscall(f.at(WORLD)), &[f.at(MATRIX)]);
        let size: u32 = h.read(layout + 0x1c);
        let buffer = h.call_sym(
            ALLOCATE,
            Conv::Thiscall(renderer.wrapping_add(0x1c)),
            &[extra.wrapping_add(size)],
        );
        f.set(h, 0x7c, buffer);
        set4(h, f, VALUE, [0; 4]);
        set4(h, f, SHADOW_MAP, [0x42, 0, 0, 0]);
        set4(h, f, DATA_BLOCK, [0xde, 0, 0, 0]);
        set4(h, f, INDEX_BLOCK, [0xe2, 0, 0, 0]);
        if buffer == 0 {
            return 0;
        }
        set4(h, f, ATT_A, [0; 4]);
        h.write(f.at(ATT_DONE), 0_u8);
        h.write(buffer, size);
        set4(h, f, ATT_B, [ONE; 4]);
        let count = h.read::<u32>(layout + 0xc) & 0x7fff_ffff;
        f.set(h, 0xe4, count);
        set4(h, f, ATT_D, [ONE; 4]);
        f.set(h, 0x78, renderer.wrapping_add(0xa0));
        f.set(h, 0xd0, 0);
        set4(h, f, ATT_C, [0; 4]);
        let epsilon = f64::from_bits(h.read(h.rebase(CONE_EPSILON)));
        let c = Ctx {
            renderer,
            buffer,
            epsilon,
        };
        let mut i = 0_u32;
        while i < f.get(h, 0xe4) {
            let entries: u32 = h.read(f.get(h, 0xc8) + 0x10);
            let entry = entries.wrapping_add(i << 4);
            f.set(h, 0x48, entry);
            f.set(h, 0x4c, f.at(VALUE));
            let slot = u32::from(h.read::<u16>(entry + 0xa) >> 13);
            let light: u32 = h.read(lights.wrapping_add(4 * slot));
            let source = if light == 0 {
                unlit(h, f, &c, entry)
            } else {
                lit(h, f, &c, light, entry)
            };
            if let Some(src) = source {
                let size = u32::from(h.read::<u16>(entry + 0xa) & 0x1fff);
                let offset = u32::from(h.read::<u16>(entry + 8));
                let room: u32 = h.read(buffer);
                if offset + size <= room && offset >= 4 {
                    // memcpy (the CRT's, through 0x00949256)
                    let mut chunk = [0_u8; 0x40];
                    for at in (0..size).step_by(0x40) {
                        let part = &mut chunk[..(size - at).min(0x40) as usize];
                        h.read_bytes(src.wrapping_add(at), part);
                        h.write_bytes(buffer.wrapping_add(offset + at), part);
                    }
                }
            }
            i += 1;
            f.set(h, 0xd0, i);
        }
        buffer
    }
}
