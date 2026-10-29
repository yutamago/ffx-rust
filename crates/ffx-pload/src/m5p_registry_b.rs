//! M5 stage 1, list phyre-registry-load-b: the type system's own code
//! that clusters reach (the serialization kernels, the conversion driver,
//! type and descriptor helpers).
//!
//! # The data-conversion kernels (`0x00473e10`..`0x00475a80`, `0x00476c20`)
//!
//! When a cluster stores a member in another primitive type than the
//! running class (`PClassDataMember`), the loader converts it element by
//! element with a kernel from the conversion registry: each registration
//! (`0x0046e910` and its neighbours, one per pair) puts a 0x18-byte node
//! `{links x3, to type, from type, kernel}` into the registry `0x00c0e2f0`
//! (`GetType<To>` at `+0xc`, `GetType<From>` at `+0x10`, the kernel at
//! `+0x14`). The conversion driver (`0x0047c610`) calls the mapped kernel
//! as `kernel(src, dst, &counts, entry)` (cdecl, `add esp, 0x10` after it,
//! `eax` unused):
//!
//! - `counts`: `{count, src stride, dst stride, direction (byte at +0xc)}`;
//! - `entry`: the mapping entry; `[[entry+4]+8]` (`A`, the stored member's
//!   offset) and `[[entry+8]+0x24]` (`B`, the runtime member's offset).
//!   With the direction byte set the source is `src + A` and the
//!   destination `dst + B`, else the other way round.
//!
//! Each kernel is one C++ conversion `To(From)`, unrolled by four in some
//! copies (the order of reads and writes per element is the same). The
//! generic port [`bridge::kernel`] runs any pair of [`KERNELS`]:
//!
//! - from a float (`fld`): to `bool` `x != 0` (NaN is true); to `PChar`,
//!   `PInt16`, `PInt32` the low bytes of `_ftol2_sse` (`0x009497e0`,
//!   [`ffx_bridge::x87::ftol`]); to `PUInt8`/`PUInt16` the low bytes of a
//!   truncating `fistp dword`; to `unsigned int` the low dword of a
//!   truncating `fistp qword`; to `PInt64` `_ftol2` (`0x00949816`, the
//!   64-bit truncation); to `PUInt64` and `PUInt128` MSVC's unsigned
//!   conversion (`x - 2^63` through `_ftol2` plus `2^63` when
//!   `x >= 2^63`; `PUInt128`'s upper eight bytes 0); to `float` `fstp
//!   dword`, to `double` `fstp qword`;
//! - from `PUInt128` (its low eight bytes as a signed `PInt64`), `PInt64`,
//!   `PUInt64`: integers take the low bytes (8-byte copies dword by
//!   dword, read and write interleaved; `PUInt128` from a 64-bit integer
//!   is zero-extended), `bool` is `!= 0`, floats come from `fild qword`
//!   (one rounding), except `PUInt64`'s, which add the sign bit as `2^63`
//!   on the x87 (rounded to 53 bits) before the store (`float` rounds
//!   twice);
//! - from `bool`: zero-extended (`movzx`, `fild dword`), `bool` to `bool`
//!   is `!= 0`, `PChar`/`PUInt8` copy the byte;
//! - `PChar` to `PChar` (`0x00476c20`, the driver's default for one-byte
//!   members): the byte.

// The x87's integer-to-float conversions round, as these casts do.
#![allow(clippy::cast_precision_loss)]

use ffx_bridge::{Mem, x87};

/// The source type of a conversion kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum From {
    /// `float`.
    F32,
    /// `double`.
    F64,
    /// `PUInt128`: read as its low eight bytes (a signed `PInt64` for
    /// floats).
    Q,
    /// `PInt64`.
    I64,
    /// `PUInt64`.
    U64,
    /// `bool`.
    Bool,
    /// `PChar`.
    Char,
}

/// The destination type of a conversion kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum To {
    /// `PChar`.
    Char,
    /// `PUInt8`.
    U8,
    /// `PInt16`.
    I16,
    /// `PUInt16`.
    U16,
    /// `PInt32`.
    I32,
    /// `unsigned int`.
    U32,
    /// `float`.
    F32,
    /// `double`.
    F64,
    /// `PUInt128`.
    Q,
    /// `PInt64`.
    I64,
    /// `PUInt64`.
    U64,
    /// `bool`.
    Bool,
}

/// The conversion kernels of this list: `(address, from, to)`. The
/// registration functions' `GetType<To>` / `GetType<From>` calls name the
/// pair (`0x0046e910` + 0xc0 per kernel, `0x00476c20` from the driver).
pub const KERNELS: &[(u32, From, To)] = &[
    (0x0047_3e10, From::F32, To::Bool),
    (0x0047_3f00, From::F64, To::Char),
    (0x0047_3f60, From::F64, To::U8),
    (0x0047_3fd0, From::F64, To::I16),
    (0x0047_4030, From::F64, To::U16),
    (0x0047_40a0, From::F64, To::I32),
    (0x0047_4100, From::F64, To::U32),
    (0x0047_4170, From::F64, To::F32),
    (0x0047_4210, From::F64, To::F64),
    (0x0047_42b0, From::F64, To::Q),
    (0x0047_4360, From::F64, To::I64),
    (0x0047_43c0, From::F64, To::U64),
    (0x0047_4460, From::F64, To::Bool),
    (0x0047_4550, From::Q, To::Char),
    (0x0047_45a0, From::Q, To::U8),
    (0x0047_45f0, From::Q, To::I16),
    (0x0047_4650, From::Q, To::U16),
    (0x0047_46b0, From::Q, To::I32),
    (0x0047_4700, From::Q, To::U32),
    (0x0047_4750, From::Q, To::F32),
    (0x0047_47f0, From::Q, To::F64),
    (0x0047_4890, From::Q, To::Q),
    (0x0047_4910, From::Q, To::I64),
    (0x0047_4970, From::Q, To::U64),
    (0x0047_49d0, From::Q, To::Bool),
    (0x0047_4a30, From::I64, To::Char),
    (0x0047_4a80, From::I64, To::U8),
    (0x0047_4ad0, From::I64, To::I16),
    (0x0047_4b30, From::I64, To::U16),
    (0x0047_4b90, From::I64, To::I32),
    (0x0047_4be0, From::I64, To::U32),
    (0x0047_4c30, From::I64, To::F32),
    (0x0047_4cd0, From::I64, To::F64),
    (0x0047_4d70, From::I64, To::Q),
    (0x0047_4de0, From::I64, To::I64),
    (0x0047_4e40, From::I64, To::U64),
    (0x0047_4ea0, From::I64, To::Bool),
    (0x0047_4f00, From::U64, To::Char),
    (0x0047_4f50, From::U64, To::U8),
    (0x0047_4fa0, From::U64, To::I16),
    (0x0047_5000, From::U64, To::U16),
    (0x0047_5060, From::U64, To::I32),
    (0x0047_50b0, From::U64, To::U32),
    (0x0047_5100, From::U64, To::F32),
    (0x0047_5290, From::U64, To::F64),
    (0x0047_5420, From::U64, To::Q),
    (0x0047_5490, From::U64, To::I64),
    (0x0047_54f0, From::U64, To::U64),
    (0x0047_5550, From::U64, To::Bool),
    (0x0047_55b0, From::Bool, To::Char),
    (0x0047_5600, From::Bool, To::U8),
    (0x0047_5650, From::Bool, To::I16),
    (0x0047_56b0, From::Bool, To::U16),
    (0x0047_5710, From::Bool, To::I32),
    (0x0047_5770, From::Bool, To::U32),
    (0x0047_57d0, From::Bool, To::F32),
    (0x0047_5890, From::Bool, To::F64),
    (0x0047_5950, From::Bool, To::Q),
    (0x0047_59c0, From::Bool, To::I64),
    (0x0047_5a20, From::Bool, To::U64),
    (0x0047_5a80, From::Bool, To::Bool),
    (0x0047_6c20, From::Char, To::Char),
];

/// The `(from, to)` pair of kernel `f` (a compile error when `f` is not in
/// [`KERNELS`]).
#[must_use]
pub const fn kernel_of(f: u32) -> (From, To) {
    let mut i = 0;
    while i < KERNELS.len() {
        if KERNELS[i].0 == f {
            return (KERNELS[i].1, KERNELS[i].2);
        }
        i += 1;
    }
    panic!("not a kernel of this list");
}

/// `2^63`, the constant at `0x00b13bd8` of the unsigned conversions.
const TWO_63: f64 = 9_223_372_036_854_775_808.0;
/// The x87's 64-bit integer indefinite.
const INDEFINITE_64: u64 = 0x8000_0000_0000_0000;

/// A truncating `fistp dword` (control word `| 0xc00`): `0x80000000` for
/// NaN and anything outside `i32`.
#[must_use]
pub fn fist32(x: f64) -> u32 {
    if x.is_nan() || x >= 2_147_483_648.0 || x <= -2_147_483_649.0 {
        0x8000_0000
    } else {
        (x.trunc() as i32).cast_unsigned()
    }
}

/// A truncating `fistp qword`, and `_ftol2` (`0x00949816`, which rounds
/// with `fistp qword` and corrects toward zero): the integer indefinite for
/// NaN and anything outside `i64`.
#[must_use]
pub fn fist64(x: f64) -> u64 {
    if x.is_nan() || !(-TWO_63..TWO_63).contains(&x) {
        INDEFINITE_64
    } else {
        (x.trunc() as i64).cast_unsigned()
    }
}

/// MSVC's float to `unsigned __int64` (the kernels' inline sequence): below
/// `2^63` (or NaN, which compares unordered) straight through `_ftol2`;
/// else `x - 2^63` through `_ftol2`, plus `2^63` when that difference is
/// below `2^63`.
#[must_use]
pub fn ftou64(x: f64) -> u64 {
    if x.is_nan() || x < TWO_63 {
        fist64(x)
    } else {
        let y = x - TWO_63;
        let v = fist64(y);
        if y < TWO_63 {
            v.wrapping_add(INDEFINITE_64)
        } else {
            v
        }
    }
}

/// `fld qword` / `fstp qword`: the double unchanged, a signalling NaN
/// quieted (as the CPU does).
#[must_use]
pub const fn quiet64(bits: u64) -> u64 {
    let exp = (bits >> 52) & 0x7ff;
    let frac = bits & 0x000f_ffff_ffff_ffff;
    if exp == 0x7ff && frac != 0 {
        bits | 0x0008_0000_0000_0000
    } else {
        bits
    }
}

pub mod bridge {
    use super::{From, Mem, To, fist32, fist64, ftou64, quiet64, x87};

    /// Writes the low `n` bytes of `v` at `at`.
    fn put(m: &mut impl Mem, at: u32, v: u64, n: usize) {
        let b = v.to_le_bytes();
        m.write_bytes(at, &b[..n]);
    }

    /// The bytes a 64-bit integer type gives an integer destination, in the
    /// original's order (8-byte copies read and write dword by dword).
    fn int_to(m: &mut impl Mem, src: u32, dst: u32, to: To) {
        match to {
            To::Char | To::U8 => {
                let b: u8 = m.read(src);
                m.write(dst, b);
            }
            To::I16 | To::U16 => {
                let w: u16 = m.read(src);
                m.write(dst, w);
            }
            To::I32 | To::U32 => {
                let w: u32 = m.read(src);
                m.write(dst, w);
            }
            To::I64 | To::U64 => {
                let lo: u32 = m.read(src);
                m.write(dst, lo);
                let hi: u32 = m.read(src.wrapping_add(4));
                m.write(dst.wrapping_add(4), hi);
            }
            To::Q | To::F32 | To::F64 | To::Bool => {}
        }
    }

    /// One element.
    fn element(m: &mut impl Mem, from: From, to: To, src: u32, dst: u32) {
        match from {
            From::F32 | From::F64 => {
                let x = if from == From::F32 {
                    x87::load(m.read(src))
                } else {
                    f64::from_bits(m.read::<u64>(src))
                };
                match to {
                    To::Bool => m.write(dst, u8::from(x != 0.0)),
                    To::Char => {
                        let v = x87::ftol(m, x);
                        put(m, dst, u64::from(v), 1);
                    }
                    To::I16 => {
                        let v = x87::ftol(m, x);
                        put(m, dst, u64::from(v), 2);
                    }
                    To::I32 => {
                        let v = x87::ftol(m, x);
                        put(m, dst, u64::from(v), 4);
                    }
                    To::U8 => put(m, dst, u64::from(fist32(x)), 1),
                    To::U16 => put(m, dst, u64::from(fist32(x)), 2),
                    To::U32 => put(m, dst, fist64(x), 4),
                    To::I64 => put(m, dst, fist64(x), 8),
                    To::U64 => put(m, dst, ftou64(x), 8),
                    To::Q => {
                        put(m, dst, ftou64(x), 8);
                        put(m, dst.wrapping_add(8), 0, 8);
                    }
                    To::F32 => m.write(dst, x87::store(x)),
                    To::F64 => {
                        let bits: u64 = m.read(src);
                        // `fld dword` then `fstp qword` never occurs here.
                        m.write(dst, quiet64(bits));
                    }
                }
            }
            From::Q | From::I64 | From::U64 => {
                let lo: u64 = m.read(src);
                match to {
                    To::Bool => m.write(dst, u8::from(lo != 0)),
                    To::F32 => {
                        let f = if from == From::U64 {
                            (lo as f64) as f32
                        } else {
                            lo.cast_signed() as f32
                        };
                        m.write(dst, f.to_bits());
                    }
                    To::F64 => {
                        let f = if from == From::U64 {
                            lo as f64
                        } else {
                            lo.cast_signed() as f64
                        };
                        m.write(dst, f.to_bits());
                    }
                    To::Q => {
                        if from == From::Q {
                            let mut b = [0u8; 16];
                            m.read_bytes(src, &mut b);
                            m.write_bytes(dst, &b);
                        } else {
                            put(m, dst, lo, 8);
                            put(m, dst.wrapping_add(8), 0, 8);
                        }
                    }
                    _ => int_to(m, src, dst, to),
                }
            }
            From::Bool | From::Char => {
                let b: u8 = m.read(src);
                match to {
                    To::Char | To::U8 => m.write(dst, b),
                    To::I16 | To::U16 => m.write(dst, u16::from(b)),
                    To::I32 | To::U32 => m.write(dst, u32::from(b)),
                    To::I64 | To::U64 => put(m, dst, u64::from(b), 8),
                    To::Q => {
                        put(m, dst, u64::from(b), 8);
                        put(m, dst.wrapping_add(8), 0, 8);
                    }
                    To::F32 => m.write(dst, f32::from(b).to_bits()),
                    To::F64 => m.write(dst, f64::from(b).to_bits()),
                    To::Bool => m.write(dst, u8::from(b != 0)),
                }
            }
        }
    }

    /// A conversion kernel `(src, dst, &counts, entry)` (cdecl) for the
    /// pair `(from, to)` ([`super::KERNELS`]): `count` elements, each
    /// stride added after its element (wrapping), the offsets picked by the
    /// direction byte.
    pub fn kernel(
        m: &mut impl Mem,
        (from, to): (From, To),
        src: u32,
        dst: u32,
        counts: u32,
        entry: u32,
    ) {
        let stored: u32 = m.read(entry.wrapping_add(4));
        let runtime: u32 = m.read(entry.wrapping_add(8));
        let a: u32 = m.read(stored.wrapping_add(8));
        let b: u32 = m.read(runtime.wrapping_add(0x24));
        let forward: u8 = m.read(counts.wrapping_add(0xc));
        let (sa, da) = if forward != 0 { (a, b) } else { (b, a) };
        let count: u32 = m.read(counts);
        let src_stride: u32 = m.read(counts.wrapping_add(4));
        let dst_stride: u32 = m.read(counts.wrapping_add(8));
        let mut s = src.wrapping_add(sa);
        let mut d = dst.wrapping_add(da);
        for _ in 0..count {
            element(m, from, to, s, d);
            s = s.wrapping_add(src_stride);
            d = d.wrapping_add(dst_stride);
        }
    }
}

/// The small type-system helpers of the list (batch 2): one port per
/// shape, the per-copy constants in tables by the copy's address.
pub mod helpers {
    use crate::m5p_tmpl_fortype::Lazy;

    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Phyre_PhyreMalloc(size, align)` (cdecl).
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `Phyre_PClassDescriptor_Destruct` (thiscall).
    pub const DESCRIPTOR_DESTRUCT: u32 = 0x0043_b3e0;
    /// `Phyre_PClassDataMember_dtor` (thiscall).
    pub const DATA_MEMBER_DTOR: u32 = 0x0057_5470;
    /// `PhyreScene_newPoolMeshInstance_006595d0` (thiscall, `ret 8`).
    pub const NEW_POOL_MESH_INSTANCE: u32 = 0x0065_95d0;
    /// `g_PhyreScene`.
    pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;

    /// `PArray<T>::swapEndianness` helpers that build their static
    /// descriptor on first use (`PClassDescriptorForType<PArray<T>>`, the
    /// fortype lists' `SWAP_LAZY` shape): the copy's constants.
    pub const LAZY: &[(u32, Lazy)] = &[
        (
            0x0049_98a0,
            Lazy {
                guard: 0x00c9_addc,
                desc: 0x00c9_ad48,
                ctor: 0x0049_9b00,
                flags: 0x00c9_add8,
                vtable: 0x00b1_6f6c,
                at_exit: 0x00af_cd40,
                flag: 1,
            },
        ),
        (
            0x0049_9950,
            Lazy {
                guard: 0x00c9_addc,
                desc: 0x00c9_ad48,
                ctor: 0x0049_9b00,
                flags: 0x00c9_add8,
                vtable: 0x00b1_6f6c,
                at_exit: 0x00af_cd40,
                flag: 0,
            },
        ),
        (
            0x004c_8ce0,
            Lazy {
                guard: 0x00ca_4714,
                desc: 0x00ca_4680,
                ctor: 0x004c_9020,
                flags: 0x00ca_4710,
                vtable: 0x00b1_f11c,
                at_exit: 0x00af_e5f0,
                flag: 1,
            },
        ),
        (
            0x004c_8db0,
            Lazy {
                guard: 0x00ca_4714,
                desc: 0x00ca_4680,
                ctor: 0x004c_9020,
                flags: 0x00ca_4710,
                vtable: 0x00b1_f11c,
                at_exit: 0x00af_e5f0,
                flag: 0,
            },
        ),
        (
            0x004c_ef50,
            Lazy {
                guard: 0x00ca_4d2c,
                desc: 0x00ca_4c98,
                ctor: 0x004c_f1d0,
                flags: 0x00ca_4d28,
                vtable: 0x00b1_f57c,
                at_exit: 0x00af_e6c0,
                flag: 1,
            },
        ),
        (
            0x004c_f000,
            Lazy {
                guard: 0x00ca_4d2c,
                desc: 0x00ca_4c98,
                ctor: 0x004c_f1d0,
                flags: 0x00ca_4d28,
                vtable: 0x00b1_f57c,
                at_exit: 0x00af_e6c0,
                flag: 0,
            },
        ),
        (
            0x0050_93a0,
            Lazy {
                guard: 0x00ca_9b44,
                desc: 0x00ca_9ab0,
                ctor: 0x0050_9630,
                flags: 0x00ca_9b40,
                vtable: 0x00b2_57e4,
                at_exit: 0x00b0_0130,
                flag: 1,
            },
        ),
        (
            0x0050_9450,
            Lazy {
                guard: 0x00ca_9b44,
                desc: 0x00ca_9ab0,
                ctor: 0x0050_9630,
                flags: 0x00ca_9b40,
                vtable: 0x00b2_57e4,
                at_exit: 0x00b0_0130,
                flag: 0,
            },
        ),
        (
            0x0051_8550,
            Lazy {
                guard: 0x00ca_b20c,
                desc: 0x00ca_b178,
                ctor: 0x0051_8cd0,
                flags: 0x00ca_b208,
                vtable: 0x00b2_75e4,
                at_exit: 0x00b0_08f0,
                flag: 1,
            },
        ),
        (
            0x0051_85e0,
            Lazy {
                guard: 0x00ca_b0ec,
                desc: 0x00ca_b058,
                ctor: 0x0051_8d70,
                flags: 0x00ca_b0e8,
                vtable: 0x00b2_74dc,
                at_exit: 0x00b0_0910,
                flag: 1,
            },
        ),
        (
            0x0051_8670,
            Lazy {
                guard: 0x00ca_b32c,
                desc: 0x00ca_b298,
                ctor: 0x0051_8e10,
                flags: 0x00ca_b328,
                vtable: 0x00b2_76ec,
                at_exit: 0x00b0_0930,
                flag: 1,
            },
        ),
        (
            0x0051_8720,
            Lazy {
                guard: 0x00ca_b20c,
                desc: 0x00ca_b178,
                ctor: 0x0051_8cd0,
                flags: 0x00ca_b208,
                vtable: 0x00b2_75e4,
                at_exit: 0x00b0_08f0,
                flag: 0,
            },
        ),
        (
            0x0051_87b0,
            Lazy {
                guard: 0x00ca_b0ec,
                desc: 0x00ca_b058,
                ctor: 0x0051_8d70,
                flags: 0x00ca_b0e8,
                vtable: 0x00b2_74dc,
                at_exit: 0x00b0_0910,
                flag: 0,
            },
        ),
        (
            0x0051_8840,
            Lazy {
                guard: 0x00ca_b32c,
                desc: 0x00ca_b298,
                ctor: 0x0051_8e10,
                flags: 0x00ca_b328,
                vtable: 0x00b2_76ec,
                at_exit: 0x00b0_0930,
                flag: 0,
            },
        ),
        (
            0x0052_7240,
            Lazy {
                guard: 0x00ca_c36c,
                desc: 0x00ca_c2d8,
                ctor: 0x0052_7690,
                flags: 0x00ca_c368,
                vtable: 0x00b2_92d4,
                at_exit: 0x00b0_0d80,
                flag: 0,
            },
        ),
        (
            0x0054_6d20,
            Lazy {
                guard: 0x00ca_eb3c,
                desc: 0x00ca_eaa8,
                ctor: 0x0054_7020,
                flags: 0x00ca_eb38,
                vtable: 0x00b2_c2fc,
                at_exit: 0x00b0_1b50,
                flag: 1,
            },
        ),
        (
            0x0054_6de0,
            Lazy {
                guard: 0x00ca_eb3c,
                desc: 0x00ca_eaa8,
                ctor: 0x0054_7020,
                flags: 0x00ca_eb38,
                vtable: 0x00b2_c2fc,
                at_exit: 0x00b0_1b50,
                flag: 0,
            },
        ),
        (
            0x0054_dac0,
            Lazy {
                guard: 0x00ca_f00c,
                desc: 0x00ca_ef78,
                ctor: 0x0054_ddd0,
                flags: 0x00ca_f008,
                vtable: 0x00b2_c954,
                at_exit: 0x00b0_1e50,
                flag: 1,
            },
        ),
        (
            0x0054_db70,
            Lazy {
                guard: 0x00ca_f00c,
                desc: 0x00ca_ef78,
                ctor: 0x0054_ddd0,
                flags: 0x00ca_f008,
                vtable: 0x00b2_c954,
                at_exit: 0x00b0_1e50,
                flag: 0,
            },
        ),
        (
            0x005a_1bd0,
            Lazy {
                guard: 0x00cb_afe4,
                desc: 0x00cb_af50,
                ctor: 0x005a_1f80,
                flags: 0x00cb_afe0,
                vtable: 0x00b3_4cfc,
                at_exit: 0x00b0_3e20,
                flag: 0,
            },
        ),
        (
            0x005a_1c60,
            Lazy {
                guard: 0x00cb_aec4,
                desc: 0x00cb_ae30,
                ctor: 0x005a_2020,
                flags: 0x00cb_aec0,
                vtable: 0x00b3_4bf4,
                at_exit: 0x00b0_3e40,
                flag: 0,
            },
        ),
        (
            0x005b_0580,
            Lazy {
                guard: 0x00cb_bdb4,
                desc: 0x00cb_bd20,
                ctor: 0x005b_0910,
                flags: 0x00cb_bdb0,
                vtable: 0x00b3_6274,
                at_exit: 0x00b0_4200,
                flag: 0,
            },
        ),
    ];

    /// Descriptor destructor bodies `(this)`: `[this] = vtable`, then a
    /// tail jump to [`DESCRIPTOR_DESTRUCT`]; its `eax`. `(copy, vtable)`.
    pub const DESTRUCT: &[(u32, u32)] = &[
        (0x0048_f3d0, 0x00b1_58d4),
        (0x0048_f3e0, 0x00b1_57e4),
        (0x004a_1720, 0x00b1_826c),
        (0x004a_1730, 0x00b1_836c),
        (0x004a_1740, 0x00b1_846c),
        (0x004a_1750, 0x00b1_856c),
        (0x004a_1760, 0x00b1_867c),
        (0x004c_f430, 0x00b1_f4ec),
        (0x004c_f440, 0x00b1_f2dc),
        (0x0050_98c0, 0x00b2_5754),
        (0x0050_98d0, 0x00b2_552c),
    ];

    /// What a scalar deleting destructor runs between its vtable store and
    /// the free.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Body {
        /// Nothing.
        None,
        /// A destructor, thiscall on `this`.
        This(u32),
        /// A function called with `this` (cdecl).
        Arg(u32),
    }

    /// A scalar deleting destructor `(this; flags)` (`ret 4`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Delete {
        /// The vtable stored first.
        pub vtable: u32,
        /// The body.
        pub body: Body,
        /// The free when `flags & 1` (cdecl).
        pub free: u32,
    }

    /// The scalar deleting destructors of the list: `(copy, how)`. The
    /// enum types' (`PTypeEnum<T>`: unregister from the type list) and the
    /// visitors' and data members'.
    pub const DELETE: &[(u32, Delete)] = &[
        (
            0x0047_9f10,
            Delete {
                vtable: 0x00b1_3468,
                body: Body::None,
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x0047_9f40,
            Delete {
                vtable: 0x00b1_3484,
                body: Body::None,
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x0047_9f70,
            Delete {
                vtable: 0x00b1_3484,
                body: Body::None,
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x0057_5f80,
            Delete {
                vtable: 0x00b3_0a60,
                body: Body::This(DATA_MEMBER_DTOR),
                free: PHYRE_FREE,
            },
        ),
        (
            0x0057_5fb0,
            Delete {
                vtable: 0x00b3_0a60,
                body: Body::This(DATA_MEMBER_DTOR),
                free: PHYRE_FREE,
            },
        ),
        (
            0x005c_be10,
            Delete {
                vtable: 0x00b3_8558,
                body: Body::Arg(0x005c_bf40),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x005c_c270,
            Delete {
                vtable: 0x00b3_85d0,
                body: Body::Arg(0x005c_c3a0),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x009f_48c0,
            Delete {
                vtable: 0x00b7_3da4,
                body: Body::Arg(0x009f_49f0),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x00a2_de70,
            Delete {
                vtable: 0x00b7_93e0,
                body: Body::Arg(0x00a2_dfa0),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x00a2_e2e0,
            Delete {
                vtable: 0x00b7_947c,
                body: Body::Arg(0x00a2_e410),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x00a2_e750,
            Delete {
                vtable: 0x00b7_96fc,
                body: Body::Arg(0x00a2_e880),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x00a2_ebc0,
            Delete {
                vtable: 0x00b7_9b98,
                body: Body::Arg(0x00a2_ecf0),
                free: OPERATOR_DELETE,
            },
        ),
        (
            0x00a3_2530,
            Delete {
                vtable: 0x00b7_9fb4,
                body: Body::Arg(0x00a3_2660),
                free: OPERATOR_DELETE,
            },
        ),
    ];

    /// The enum types' unregister `(type)` (cdecl): unlinks it from the
    /// singly linked list (link at `+8`) whose head is the global;
    /// `(copy, head)`.
    pub const UNLINK: &[(u32, u32)] = &[
        (0x0049_e920, 0x00c9_aef8),
        (0x005c_bf40, 0x00cb_d908),
        (0x005c_c3a0, 0x00cb_d944),
        (0x009f_49f0, 0x0194_359c),
        (0x00a2_dfa0, 0x01a8_5318),
        (0x00a2_e410, 0x01a8_5354),
        (0x00a2_e880, 0x01a8_5390),
        (0x00a2_ecf0, 0x01a8_53cc),
        (0x00a3_2660, 0x01a8_5578),
    ];

    /// The types' `(object, &name, &size)` (cdecl): the name at `object +
    /// offset` and its length plus one; 0. `(copy, offset)`.
    pub const NAME: &[(u32, u32)] = &[
        (0x0047_fd20, 0),
        (0x004c_0f80, 0),
        (0x0050_7aa0, 0),
        (0x005c_beb0, 4),
        (0x005c_c310, 4),
        (0x009f_4960, 4),
        (0x00a2_df10, 4),
        (0x00a2_e380, 4),
        (0x00a2_e7f0, 4),
        (0x00a2_ec60, 4),
        (0x00a3_25d0, 4),
    ];

    /// The enum types' create callbacks `(out, name)` (cdecl): `*out =
    /// find(name)`; 0 when found, else 0x13. `(copy, find)`.
    pub const CREATE: &[(u32, u32)] = &[
        (0x005c_bf20, 0x005c_be40),
        (0x005c_c380, 0x005c_c2a0),
        (0x009f_49d0, 0x009f_48f0),
        (0x00a2_df80, 0x00a2_dea0),
        (0x00a2_e3f0, 0x00a2_e310),
        (0x00a2_e860, 0x00a2_e780),
        (0x00a2_ecd0, 0x00a2_ebf0),
        (0x00a3_2640, 0x00a3_2560),
    ];

    /// `PArray<T>` resets `(this)` (thiscall): the buffer freed unless
    /// borrowed (count bit 31) or, for all but the first, null; both words
    /// cleared. `(copy, null check)`.
    pub const RESET: &[(u32, bool)] = &[
        (0x0047_8d60, false),
        (0x0047_8d90, true),
        (0x0047_8dc0, true),
        (0x0047_8e50, true),
        (0x0050_9830, true),
        (0x0057_7950, true),
        (0x0057_7980, true),
        (0x0057_79b0, true),
        (0x0057_7a40, true),
        (0x0057_7a70, true),
        (0x0057_7aa0, true),
        (0x0057_7ad0, true),
        (0x009f_d4b0, true),
    ];

    /// `PSharray<T>` resets `(this)` (thiscall): unless borrowed, the
    /// element release `(data, count & 0x7fffffff)` (cdecl); both words
    /// cleared. `(copy, release)`.
    pub const RELEASE: &[(u32, u32)] = &[(0x004a_8300, 0x004a_b510), (0x004e_a0d0, 0x004e_c310)];

    /// The one-word constants `(copy, value)` of methods that ignore their
    /// arguments (`mov eax, k; ret n`).
    pub const CONSTANT: &[(u32, u32)] = &[
        (0x0057_6110, 0x17),
        (0x0057_6120, 0x17),
        (0x0067_5a40, 0x18),
    ];

    /// The entry of `f` in `table`.
    #[must_use]
    pub const fn of<T: Copy>(table: &[(u32, T)], f: u32) -> T {
        let mut i = 0;
        while i < table.len() {
            if table[i].0 == f {
                return table[i].1;
            }
            i += 1;
        }
        panic!("not in the table");
    }

    pub mod bridge {
        use super::{
            Body, DESCRIPTOR_DESTRUCT, Delete, NEW_POOL_MESH_INSTANCE, OPERATOR_DELETE, PHYRE_FREE,
            PHYRE_MALLOC, PHYRE_SCENE,
        };
        use ffx_graphic::host::{Conv, Host};

        /// A descriptor destructor body ([`super::DESTRUCT`]).
        pub fn destruct(h: &mut impl Host, this: u32, vtable: u32) -> u32 {
            let vt = h.rebase(vtable);
            h.write(this, vt);
            h.call_sym(DESCRIPTOR_DESTRUCT, Conv::Thiscall(this), &[])
        }

        /// A scalar deleting destructor ([`super::DELETE`]); `this`.
        pub fn delete(h: &mut impl Host, d: &Delete, this: u32, flags: u32) -> u32 {
            let vt = h.rebase(d.vtable);
            h.write(this, vt);
            match d.body {
                Body::None => {}
                Body::This(f) => {
                    h.call_sym(f, Conv::Thiscall(this), &[]);
                }
                Body::Arg(f) => {
                    h.call_sym(f, Conv::Cdecl, &[this]);
                }
            }
            if flags & 1 != 0 {
                h.call_sym(d.free, Conv::Cdecl, &[this]);
            }
            this
        }

        /// An enum type's unregister ([`super::UNLINK`]): 0.
        pub fn unlink(h: &mut impl Host, head: u32, ty: u32) -> u32 {
            let mut link = h.rebase(head);
            let mut node: u32 = h.read(link);
            let want = ty.wrapping_add(8);
            while node != 0 {
                if node.wrapping_add(8) == want {
                    let next: u32 = h.read(node.wrapping_add(8));
                    h.write(link, next);
                    h.write(node.wrapping_add(8), 0u32);
                    return 0;
                }
                link = node.wrapping_add(8);
                node = h.read(link);
            }
            0
        }

        /// A type's `(object, &name, &size)` ([`super::NAME`]): 0.
        pub fn name(
            h: &mut impl Host,
            offset: u32,
            object: u32,
            name_out: u32,
            size_out: u32,
        ) -> u32 {
            let name: u32 = h.read(object.wrapping_add(offset));
            let mut len = 0u32;
            while h.read::<u8>(name.wrapping_add(len)) != 0 {
                len = len.wrapping_add(1);
            }
            h.write(size_out, len.wrapping_add(1));
            h.write(name_out, name);
            0
        }

        /// A `PSharray<T>` reset ([`super::RELEASE`]).
        pub fn release(h: &mut impl Host, release: u32, this: u32) {
            let count: u32 = h.read(this);
            if count >> 31 == 0 {
                let data: u32 = h.read(this.wrapping_add(4));
                h.call_sym(release, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
            }
            h.write(this, 0u32);
            h.write(this.wrapping_add(4), 0u32);
        }

        /// `0x0047a650(p)` (cdecl): `PhyreFree(p)` unless `p` is 0; its
        /// `eax`, or 0.
        pub fn free_some(h: &mut impl Host, p: u32) -> u32 {
            if p == 0 {
                0
            } else {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p])
            }
        }

        /// `0x004a1770(this)`: `PhyreFree([this+4])` unless bit 0 of its low
        /// byte is set (a tagged block).
        pub fn free_untagged(h: &mut impl Host, this: u32) {
            let p: u32 = h.read(this.wrapping_add(4));
            if p & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
        }

        /// `0x00575460(this)`: `PhyreFree([this+0xc])` when bit 0 of the
        /// byte at `+0x10` (owned) is set.
        pub fn free_owned(h: &mut impl Host, this: u32) {
            let owned: u8 = h.read(this.wrapping_add(0x10));
            if owned & 1 != 0 {
                let p: u32 = h.read(this.wrapping_add(0xc));
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
        }

        /// The word at `this + offset` (`0x00576040`).
        pub fn word_at(h: &mut impl Host, this: u32, offset: u32) -> u32 {
            h.read(this.wrapping_add(offset))
        }

        /// `0x00479910(this)`: `[this] = vtable` (0x00b13484).
        pub fn set_vtable(h: &mut impl Host, this: u32, vtable: u32) {
            let vt = h.rebase(vtable);
            h.write(this, vt);
        }

        /// `0x005a0270(this)`: the vtable 0x00b344b0, the list node at
        /// `+0x2c` unlinked (`{next, prev}`) and pointed at itself, then the
        /// base vtable 0x00b0e568; `eax` = the old `prev`.
        pub fn named_semantic_dtor(h: &mut impl Host, this: u32, vtables: [u32; 2]) -> u32 {
            let vt = h.rebase(vtables[0]);
            h.write(this, vt);
            let node = this.wrapping_add(0x2c);
            let next: u32 = h.read(node);
            let prev: u32 = h.read(this.wrapping_add(0x30));
            h.write(next.wrapping_add(4), prev);
            h.write(prev, next);
            h.write(node, node);
            h.write(node.wrapping_add(4), node);
            let vt = h.rebase(vtables[1]);
            h.write(this, vt);
            prev
        }

        /// `0x00575630(this; flags)` (`ret 4`): `PIClassDataMemberDynamicType`'s
        /// deleting destructor: the vtable 0x00b309cc, the owner's
        /// (`+4`) `+0x28` cleared, `operator delete` when `flags & 1`; `this`.
        pub fn dynamic_type_delete(h: &mut impl Host, this: u32, flags: u32, vtable: u32) -> u32 {
            let owner: u32 = h.read(this.wrapping_add(4));
            let vt = h.rebase(vtable);
            h.write(this, vt);
            h.write(owner.wrapping_add(0x28), 0u32);
            if flags & 1 != 0 {
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
            }
            this
        }

        /// The member's address in an object (`thiscall(member; object)`,
        /// `ret 4`, `0x00576000`, `0x00576020`): `object + [member+0x24] -
        /// [[member+0xc]+0x88]`.
        pub fn member_address(h: &mut impl Host, this: u32, object: u32) -> u32 {
            let owner: u32 = h.read(this.wrapping_add(0xc));
            let offset: u32 = h.read(this.wrapping_add(0x24));
            let base: u32 = h.read(owner.wrapping_add(0x88));
            offset.wrapping_sub(base).wrapping_add(object)
        }

        /// `0x005756b0` / `0x005756f0` `(member; a, object)` (`ret 8`): the
        /// member's type (`+0x18`) runs its virtual `slots[1]` (byte `+0x14`
        /// bit 1 set) or `slots[0]` with `(a, the member's address in
        /// object)`; its `eax`.
        pub fn member_virtual(
            h: &mut impl Host,
            slots: [u32; 2],
            this: u32,
            a: u32,
            object: u32,
        ) -> u32 {
            let q = member_address(h, this, object);
            let ty: u32 = h.read(this.wrapping_add(0x18));
            let flags: u8 = h.read(this.wrapping_add(0x14));
            let slot = if flags & 2 != 0 { slots[1] } else { slots[0] };
            h.call_virtual(ty, slot, &[a, q])
        }

        /// `0x00672b30(this; mesh, existing)` (`ret 8`): `existing`, or
        /// when it is 0 the scene's pool instance
        /// (`PhyreScene_newPoolMeshInstance(g_PhyreScene; this, mesh)`).
        pub fn pool_instance(h: &mut impl Host, this: u32, mesh: u32, existing: u32) -> u32 {
            if existing != 0 {
                return existing;
            }
            let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
            h.call_sym(NEW_POOL_MESH_INSTANCE, Conv::Thiscall(scene), &[this, mesh])
        }

        /// An array's storage for `n` words (`n <= 1`: the inline word at
        /// `this + 4`, else `PhyreMalloc(4n, 4)`) with `count` words of
        /// `src` copied in (none when the allocation failed); the storage.
        /// `0x004d0470(this; n, src)` copies `n`, `0x004d04d0(this; n, src,
        /// count)` `count` words.
        pub fn alloc_copy(h: &mut impl Host, this: u32, n: u32, src: u32, count: u32) -> u32 {
            let buf = if n > 1 {
                h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_shl(2), 4])
            } else {
                this.wrapping_add(4)
            };
            if buf != 0 {
                let (mut p, mut s) = (buf, src);
                for _ in 0..count {
                    if p != 0 {
                        let w: u32 = h.read(s);
                        h.write(p, w);
                        s = s.wrapping_add(4);
                    }
                    p = p.wrapping_add(4);
                }
            }
            buf
        }
    }
}

/// Batch 3: the shared arrays, red-black tree lookups, data-member and
/// visitor methods, animation slot filters, small float helpers.
pub mod more {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Phyre_PClassDataMember_vftable`.
    pub const DATA_MEMBER_VTABLE: u32 = 0x00b3_09d8;
    /// The data member's `+0x1c` list destructor (thiscall).
    pub const MEMBER_LIST_DTOR: u32 = 0x0057_58e0;
    /// `Phyre_PTypeDispenser_GetType_unsigned_int` (cdecl).
    pub const GET_TYPE_U32: u32 = 0x0043_7fc0;
    /// `Phyre_PClassDescriptor_applyDefaultBufferWithValidation` (thiscall,
    /// `(p, n)`).
    pub const APPLY_DEFAULT_BUFFER: u32 = 0x0043_c160;
    /// `PSharray<PUInt32>::setToExternalBuffer` (`0x004d0b00`, this list).
    pub const SHARRAY_SET: u32 = 0x004d_0b00;
    /// `PSharray<PUInt32>` storage (`0x004d04d0`, this list).
    pub const SHARRAY_ALLOC_COPY: u32 = 0x004d_04d0;
    /// The descriptor method `0x0043ca50` (thiscall, `(a, b)`) the physics
    /// helpers forward to.
    pub const DESCRIPTOR_PAIR: u32 = 0x0043_ca50;
    /// The `1.0` (double) the `FFXApplication` axis helpers use.
    pub const ONE: u32 = 0x00b9_2248;

    /// The physics descriptors' pair forwarders `(a, b)`: `(copy,
    /// descriptor)`.
    pub const PAIR: &[(u32, u32)] = &[
        (0x005d_7980, 0x00cb_e8a8),
        (0x005d_79c0, 0x00cb_eba0),
        (0x005d_7a40, 0x00cb_f488),
    ];
    /// The array copy constructors `(dst, src, n)`: `(copy, words per
    /// element)`.
    pub const COPY: &[(u32, u32)] = &[(0x005d_78b0, 9), (0x005d_78e0, 10), (0x005d_7910, 10)];

    /// The storage of a `PSharray` (`+0` count, bit 31 borrowed; one
    /// element inline at `+4`, more through the pointer there), 0 when
    /// empty.
    fn storage(h: &mut impl Host, this: u32, word: u32) -> u32 {
        let n = word & 0x7fff_ffff;
        if n > 1 {
            h.read(this.wrapping_add(4))
        } else if n != 0 {
            this.wrapping_add(4)
        } else {
            0
        }
    }

    pub mod bridge {
        use super::{
            APPLY_DEFAULT_BUFFER, Conv, DATA_MEMBER_VTABLE, DESCRIPTOR_PAIR, GET_TYPE_U32, Host,
            MEMBER_LIST_DTOR, ONE, PHYRE_FREE, SHARRAY_ALLOC_COPY, SHARRAY_SET, storage, x87,
        };

        /// `Phyre_PClassDataMember`'s scalar deleting destructor
        /// `(this; flags)` (`0x00575510`, `0x005755a0`): the vtable; the
        /// owner (`+0xc`) unlinks it (virtual slot 9) unless its list at
        /// `+4` is empty; the `+0x1c` list destructor; the owned name
        /// (`+0x10`, byte `+0x14` bit 0) freed; `this` freed when
        /// `flags & 1`; `this`.
        pub fn member_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
            let vt = h.rebase(DATA_MEMBER_VTABLE);
            h.write(this, vt);
            let list = this.wrapping_add(4);
            let first: u32 = h.read(list);
            if first != list {
                let owner: u32 = h.read(this.wrapping_add(0xc));
                h.call_virtual(owner, 9, &[this]);
            }
            h.call_sym(
                MEMBER_LIST_DTOR,
                Conv::Thiscall(this.wrapping_add(0x1c)),
                &[],
            );
            let owned: u8 = h.read(this.wrapping_add(0x14));
            if owned & 1 != 0 {
                let name: u32 = h.read(this.wrapping_add(0x10));
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
            }
            if flags & 1 != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
            }
            this
        }

        /// `PSharray<PUInt32>::setToExternalBuffer(this; data, count)`
        /// (`0x004d0b00`, `ret 8`): 5 for a negative count; one element is
        /// copied inline (count word 1), else `+4 = data` and the count
        /// marked borrowed; the old owned, non-null buffer (neither inline
        /// nor `data`) freed; 0.
        pub fn sharray_set(h: &mut impl Host, this: u32, data: u32, count: u32) -> u32 {
            if count.cast_signed() < 0 {
                return 5;
            }
            let word: u32 = h.read(this);
            let old = storage(h, this, word);
            let inline = this.wrapping_add(4);
            let (target, new_word) = if count == 1 {
                let first: u32 = h.read(data);
                h.write(inline, first);
                (inline, 1)
            } else {
                (data, count | 0x8000_0000)
            };
            if old != target && old != inline {
                let now: u32 = h.read(this);
                if now >> 31 == 0 && old != 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
            }
            if target != inline {
                h.write(inline, target);
            }
            h.write(this, new_word);
            0
        }

        /// `PSharray<PUInt32>::resize(this; n)` (`0x004d0970`, `ret 4`): 0
        /// at once when the count word is `n`; else new storage holding
        /// the first `min(n, count)` words (`0x004d04d0`), 0xd when that
        /// fails for `n > 0`; the old owned, non-null buffer freed unless
        /// inline or reused; the count word `n`; 0.
        pub fn sharray_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
            let word: u32 = h.read(this);
            if n == word {
                return 0;
            }
            let count = word & 0x7fff_ffff;
            let old = storage(h, this, word);
            let keep = n.min(count);
            let buf = h.call_sym(SHARRAY_ALLOC_COPY, Conv::Thiscall(this), &[n, old, keep]);
            if buf == 0 && n != 0 {
                return 0xd;
            }
            let inline = this.wrapping_add(4);
            if old != buf && old != inline {
                let now: u32 = h.read(this);
                if now >> 31 == 0 && old != 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
            }
            if buf != inline {
                h.write(inline, buf);
            }
            h.write(this, n);
            0
        }

        /// `PClassDataMemberPhyreSharray<PUInt32>`'s buffer set `(this;
        /// data, n)` (`0x004d0ba0`, `ret 8`): `GetType<unsigned int>()`'s
        /// class descriptor (its virtual slot 1); when `data` is the
        /// inline word, `n` is 1 and there is a descriptor, its
        /// `applyDefaultBufferWithValidation(data, 1)`, else
        /// `setToExternalBuffer(data, n)`; 0.
        pub fn sharray_member_set(h: &mut impl Host, this: u32, data: u32, n: u32) -> u32 {
            let inline = this.wrapping_add(4);
            let ty = h.call_sym(GET_TYPE_U32, Conv::Cdecl, &[]);
            let desc = h.call_virtual(ty, 1, &[]);
            if data == inline && n == 1 && desc != 0 {
                h.call_sym(APPLY_DEFAULT_BUFFER, Conv::Thiscall(desc), &[inline, n]);
            } else {
                h.call_sym(SHARRAY_SET, Conv::Thiscall(this), &[data, n]);
            }
            0
        }

        /// `PDeleteChildrenVisitor`'s visit `(this; objects, count,
        /// member, flags)` (`0x004d7510`, `ret 0x10`): the member's class
        /// descriptor (virtual slot 1); unless it is 0 or `flags` bit 1 is
        /// set, its `destroyInstance` (slot 11) on each of the `count`
        /// objects from `objects + [desc+0x8c]`, `[desc+0x1c]` apart. The
        /// `eax` it leaves: 0 without a descriptor, the descriptor when
        /// skipped, the stride for no objects, else the last call's.
        pub fn delete_children(
            h: &mut impl Host,
            objects: u32,
            count: u32,
            member: u32,
            flags: u32,
        ) -> u32 {
            let desc = h.call_virtual(member, 1, &[]);
            if desc == 0 || (flags >> 1) & 1 != 0 {
                return desc;
            }
            let stride: u32 = h.read(desc.wrapping_add(0x1c));
            let base: u32 = h.read(desc.wrapping_add(0x8c));
            let mut p = base.wrapping_add(objects);
            let mut eax = stride;
            for _ in 0..count {
                eax = h.call_virtual(desc, 11, &[p]);
                p = p.wrapping_add(stride);
            }
            eax
        }

        /// `PSharray<T*>::indexOf(this; &value, start)` (`0x00518a60`,
        /// `ret 8`): the first index from `start` holding `value`, or -1.
        pub fn index_of(h: &mut impl Host, this: u32, value: u32, start: u32) -> u32 {
            let word: u32 = h.read(this);
            let n = word & 0x7fff_ffff;
            if start >= n {
                return u32::MAX;
            }
            let v: u32 = h.read(value);
            let mut i = start;
            while i < n {
                let data = storage(h, this, word);
                let e: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
                if e == v {
                    return i;
                }
                i += 1;
            }
            u32::MAX
        }

        /// `PSharray<PString>::indexOf(this; &name, start)` (`0x0054dcd0`,
        /// `ret 8`): the first index from `start` whose string (a pointer
        /// with bit 0 a tag) is `name` or equal to it, or -1.
        pub fn name_index(h: &mut impl Host, this: u32, name: u32, start: u32) -> u32 {
            let word: u32 = h.read(this);
            let n = word & 0x7fff_ffff;
            if start >= n {
                return u32::MAX;
            }
            let want: u32 = h.read(name);
            let data: u32 = h.read(this.wrapping_add(4));
            let mut i = start;
            while i < n {
                let e: u32 = h.read(data.wrapping_add(i.wrapping_mul(4)));
                let s = e.wrapping_sub(e & 1);
                if s == want {
                    return i;
                }
                if s != 0 && want != 0 && same_string(h, s, want) {
                    return i;
                }
                i += 1;
            }
            u32::MAX
        }

        /// The inline `strcmp(a, b) == 0`.
        fn same_string(h: &mut impl Host, a: u32, b: u32) -> bool {
            let mut k = 0u32;
            loop {
                let x: u8 = h.read(a.wrapping_add(k));
                let y: u8 = h.read(b.wrapping_add(k));
                if x != y {
                    return false;
                }
                if x == 0 {
                    return true;
                }
                k = k.wrapping_add(1);
            }
        }

        /// A tree node's key (`+0xc`; the original reads address 0 for a
        /// null node).
        fn key_of(h: &mut impl Host, node: u32) -> u32 {
            h.read(if node == 0 { 0 } else { node.wrapping_add(0xc) })
        }

        /// The red-black tree's find `(tree; &key)` (`0x005cc560`, `ret
        /// 4`): from the root (`tree+0xc`, the tree itself the null leaf)
        /// left (`+0`) for a larger node key (`+0xc`), right (`+4`) for a
        /// smaller one (unsigned); the node, or 0.
        pub fn tree_find(h: &mut impl Host, tree: u32, key: u32) -> u32 {
            let k: u32 = h.read(key);
            let mut node: u32 = h.read(tree.wrapping_add(0xc));
            while node != tree {
                let nk = key_of(h, node);
                if nk == k {
                    return node;
                }
                let side = if nk > k { 0 } else { 4 };
                node = h.read(node.wrapping_add(side));
            }
            0
        }

        /// The red-black tree's insert `(tree; node)` (`0x005cc5e0`, `ret
        /// 4`): the node's `+8` gets bit 0 set (added), both children the
        /// leaf; the search as [`tree_find`] by `node+0xc`: an existing
        /// node with that key is returned (the new one is left so);
        /// else it hangs below the last node (`+8` = the parent with bit 0
        /// kept, the tree's `+0xc` slot for an empty tree), 0.
        pub fn tree_insert(h: &mut impl Host, tree: u32, node: u32) -> u32 {
            let flags: u32 = h.read(node.wrapping_add(8));
            let bit = flags & 1;
            h.write(node.wrapping_add(8), flags.wrapping_add(1 - bit));
            h.write(node, tree);
            h.write(node.wrapping_add(4), tree);
            let mut parent = tree.wrapping_add(0xc);
            let mut side = 0u32;
            let mut cur: u32 = h.read(parent);
            if cur != tree {
                let k: u32 = h.read(node.wrapping_add(0xc));
                loop {
                    parent = cur;
                    let nk = key_of(h, cur);
                    if nk == k {
                        return cur;
                    }
                    side = u32::from(nk < k);
                    cur = h.read(cur.wrapping_add(4 * side));
                    if cur == tree {
                        break;
                    }
                }
            }
            let flags: u8 = h.read(node.wrapping_add(8));
            h.write(
                node.wrapping_add(8),
                u32::from(flags & 1).wrapping_add(parent),
            );
            h.write(parent.wrapping_add(4 * side), node);
            0
        }

        /// `PAnimationSlotFilter`'s combined test `(this; object, x)`
        /// (`0x0052ae40`, `ret 8`): `object` slot 1 `(this, x)`; then the
        /// child filter (`+0x14`) slot 3 `(object, this)` when there is
        /// one; then `object` slot 3 `(this, x)`. A false `al` stops
        /// (`al = 0` over that call's `eax`); else `al = 1` over the last.
        pub fn slot_filter(h: &mut impl Host, this: u32, object: u32, x: u32) -> u32 {
            let r = h.call_virtual(object, 1, &[this, x]);
            if r as u8 == 0 {
                return r & !0xff;
            }
            let child: u32 = h.read(this.wrapping_add(0x14));
            if child != 0 {
                let r = h.call_virtual(child, 3, &[object, this]);
                if r as u8 == 0 {
                    return r & !0xff;
                }
            }
            let r = h.call_virtual(object, 3, &[this, x]);
            r & !0xff | 1
        }

        /// The slot filter's copy `(&dst, &&src, _, &indices, _, &range)`
        /// (`0x00529e60`, cdecl): `range = {_, count, base}`; the sorted
        /// index list (self-relative at `*indices`: a count, then the
        /// indices) skipped up to `base`; of the `count` 32-byte keys from
        /// `base` on, the listed ones are skipped, the others copied (four
        /// floats through the x87, four words); both sides advance; 0.
        pub fn slot_copy(h: &mut impl Host, dst: u32, src: u32, indices: u32, range: u32) -> u32 {
            let mut d: u32 = h.read(dst);
            let s0: u32 = h.read(src);
            let mut s: u32 = h.read(s0);
            let rel: u32 = h.read(indices);
            let list = rel.wrapping_add(indices);
            let count: u32 = h.read(range.wrapping_add(4));
            let base: u32 = h.read(range.wrapping_add(8));
            let n: u32 = h.read(list);
            let mut idx = list.wrapping_add(4);
            let end = idx.wrapping_add(n.wrapping_mul(4));
            while idx < end && h.read::<u32>(idx) < base {
                idx = idx.wrapping_add(4);
            }
            for i in 0..count {
                if idx < end && base.wrapping_add(i) == h.read::<u32>(idx) {
                    idx = idx.wrapping_add(4);
                } else {
                    for k in 0..4 {
                        let f: u32 = h.read(s.wrapping_add(4 * k));
                        h.write(d.wrapping_add(4 * k), x87::store(x87::load(f)));
                    }
                    for k in 4..8 {
                        let w: u32 = h.read(s.wrapping_add(4 * k));
                        h.write(d.wrapping_add(4 * k), w);
                    }
                }
                d = d.wrapping_add(0x20);
                s = s.wrapping_add(0x20);
            }
            0
        }

        /// `0x0047bf50(n, p)` (cdecl): byte-swaps `n` words at `p` in place.
        pub fn swap_words(h: &mut impl Host, n: u32, p: u32) {
            let mut q = p;
            for _ in 0..n {
                let w: u32 = h.read(q);
                h.write(q, w.swap_bytes());
                q = q.wrapping_add(4);
            }
        }

        /// `PNamedSemanticDescriptor`'s scalar deleting destructor `(this;
        /// flags)` (`0x005a02a0`): the destructor body
        /// ([`crate::m5p_registry_b::helpers::bridge::named_semantic_dtor`]),
        /// `PhyreFree(this)` when `flags & 1`; `this`.
        pub fn named_semantic_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
            crate::m5p_registry_b::helpers::bridge::named_semantic_dtor(
                h,
                this,
                [0x00b3_44b0, 0x00b0_e568],
            );
            if flags & 1 != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
            }
            this
        }

        /// The `FFXApplication` axis helpers `(&xy)` (cdecl, a float in
        /// `ST0`): the float at `xy + offset`, `-(v + 1)` (`negate`) or
        /// `v - 1`, stored as a float (`0x00a2f740`: x, negated;
        /// `0x00a2f760`: y, negated; `0x00a2f780`: x; `0x00a2f7a0`: y).
        pub fn axis(h: &mut impl Host, xy: u32, offset: u32, negate: bool) -> f32 {
            let v = x87::load(h.read(xy.wrapping_add(offset)));
            let one = f64::from_bits(h.read(h.rebase(ONE)));
            let r = if negate {
                -x87::add(v, one)
            } else {
                x87::sub(v, one)
            };
            f32::from_bits(x87::store(r))
        }

        /// The physics descriptors' forwarders `(a, b)` ([`super::PAIR`]):
        /// `0x0043ca50(desc; a ? a + 4 : 0, b ? b + 4 : 0)`; its `eax`.
        pub fn pair(h: &mut impl Host, desc: u32, a: u32, b: u32) -> u32 {
            let b4 = if b == 0 { 0 } else { b.wrapping_add(4) };
            let a4 = if a == 0 { 0 } else { a.wrapping_add(4) };
            let d = h.rebase(desc);
            h.call_sym(DESCRIPTOR_PAIR, Conv::Thiscall(d), &[a4, b4])
        }

        /// The array copy constructors `(dst, src, n)` ([`super::COPY`]):
        /// each non-null destination slot gets the next source element
        /// (`words` words; the source advances only then).
        pub fn copy_construct(h: &mut impl Host, words: u32, dst: u32, src: u32, n: u32) {
            let (mut d, mut s) = (dst, src);
            for _ in 0..n {
                if d != 0 {
                    for k in 0..words {
                        let w: u32 = h.read(s.wrapping_add(4 * k));
                        h.write(d.wrapping_add(4 * k), w);
                    }
                    s = s.wrapping_add(4 * words);
                }
                d = d.wrapping_add(4 * words);
            }
        }

        /// The LOD blend `(&weights, n, &arrays, &out)` (`0x005cb940`,
        /// cdecl): `out[k] = ((a0[k] w0 + a1[k] w1) + a2[k] w2) + a3[k] w3`
        /// on the x87 (53 bits), stored as floats, for `n` values (`n` 0
        /// runs 2^32 times, as the original's `dec; jne`).
        pub fn blend4(h: &mut impl Host, weights: u32, n: u32, arrays: u32, out: u32) {
            let w: [f64; 4] = core::array::from_fn(|i| {
                x87::load(x87::store(x87::load(
                    h.read(weights.wrapping_add(4 * i as u32)),
                )))
            });
            let a: [u32; 4] = core::array::from_fn(|i| h.read(arrays.wrapping_add(4 * i as u32)));
            let mut o: u32 = h.read(out);
            let mut k = 0u32;
            let mut left = n;
            loop {
                let v: [f64; 4] = core::array::from_fn(|i| {
                    x87::load(h.read(a[i].wrapping_add(k.wrapping_mul(4))))
                });
                let s = x87::add(x87::mul(v[0], w[0]), x87::mul(v[1], w[1]));
                let s = x87::add(s, x87::mul(v[2], w[2]));
                let s = x87::add(s, x87::mul(v[3], w[3]));
                h.write(o, x87::store(s));
                o = o.wrapping_add(4);
                k = k.wrapping_add(1);
                left = left.wrapping_sub(1);
                if left == 0 {
                    break;
                }
            }
        }
    }
}

/// Batch 4: tree destructors, dynamic-mesh instances, the memory-copy
/// conversion kernel, the matrix scale, the script user-data check, and
/// `FFXApplication` teardown helpers.
pub mod rest {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Phyre_PhyreMalloc(size, align)` (cdecl).
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// The CRT's `memcpy` thunk (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// The member's element count getter (`[this+0x34]`, thiscall).
    pub const MEMBER_COUNT: u32 = 0x0057_6050;
    /// `Phyre_PCluster_allocateInstanceList(cluster; n, class, 0)`.
    pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
    /// The red-black tree insert (`0x005cc5e0`, this list) and its
    /// post-insert fix-up (`Phyre_PRedBlackTree_postInsertFinalize`).
    pub const TREE_INSERT: [u32; 2] = [0x005c_c5e0, 0x0057_4980];
    /// `PSharray` storage `(this; count, data)` (thiscall, `ret 8`; the `0x004d0470` shape) -> the new storage.
    pub const SHARRAY_COPY: u32 = 0x005a_6aa0;
    /// The Lua calls of `0x00570d50`: `lua_touserdata`, `lua_getmetatable`,
    /// `lua_getfield`, `lua_rawequal`, `lua_settop` (cdecl).
    pub const LUA: [u32; 5] = [
        0x0094_c1b0,
        0x0094_b0d0,
        0x0094_b030,
        0x0094_b8b0,
        0x0094_bdc0,
    ];
    /// The metatable name `0x00570d50` looks up in the registry.
    pub const META_NAME: u32 = 0x00b3_07bc;
    /// `ClusterManager_GetInstance`, `MapHasActive_0069b890`,
    /// `ClusterManager_releasePCluster`.
    pub const CLUSTERS: [u32; 3] = [0x0069_a1a0, 0x0069_b890, 0x0069_bdb0];
    /// The three cluster ids `0x006dfdd0` releases.
    pub const CLUSTER_IDS: [u32; 3] = [0x00cd_edf8, 0x00cd_edfc, 0x00cd_ee00];
    /// `IggyMenu_dtor_IggyMenu` (thiscall).
    pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;
    /// `Sg_SetMuteSound` (cdecl).
    pub const SET_MUTE_SOUND: u32 = 0x0082_1c60;
    /// `MsBattleCheck`, `MsMuteBattleSubTitle` (cdecl).
    pub const SUBTITLE: [u32; 2] = [0x0078_0c90, 0x0078_5990];
    /// The menu globals: `0x006f72a0`'s menu, two words and a byte;
    /// `0x006f7820`'s table of seven menus; the subtitle state words.
    pub const MENU: u32 = 0x00ce_82c0;
    pub const MENU_WORDS: [u32; 2] = [0x00ce_82b8, 0x00ce_82b4];
    pub const MENU_BYTE: u32 = 0x00ce_82bc;
    pub const MENU_TABLE: u32 = 0x00ce_82e4;
    pub const SUBTITLE_MUTED: u32 = 0x00ce_82bd;
    /// The radial-line instance's six getters (thiscall on its mesh's
    /// `+0x1c`), stored at `out + 0x10`..`+0x24`.
    pub const RADIAL_GETTERS: [u32; 6] = [
        0x006f_7bf0,
        0x006f_7bb0,
        0x006f_7be0,
        0x006f_7bc0,
        0x006f_7c00,
        0x006f_7bd0,
    ];

    /// The dynamic meshes' instance factories `(this; cluster, existing)`:
    /// `(copy, [instance class, constructor])`.
    pub const NEW_INSTANCE: &[(u32, [u32; 2])] = &[
        (0x005a_df70, [0x00ca_8020, 0x004f_b3c0]),
        (0x0067_44c0, [0x00cc_bce0, 0x0067_4bf0]),
        (0x0067_6400, [0x00cc_bdb8, 0x0067_5510]),
        (0x0067_7a60, [0x00cc_bf88, 0x0067_6b70]),
    ];
    /// `PRedBlackTreePair` destructors (`Phyre_PRedBlackTree_clear` twice).
    pub const TREE_PAIR_DTOR: &[u32] = &[0x0047_9000, 0x0057_7b00, 0x0057_7b60, 0x0057_7bc0];

    pub mod bridge {
        use super::{
            ALLOCATE_INSTANCE_LIST, CLUSTER_IDS, CLUSTERS, Conv, Host, IGGY_MENU_DTOR, LUA,
            MEMBER_COUNT, MEMCPY, MENU, MENU_BYTE, MENU_TABLE, MENU_WORDS, META_NAME,
            OPERATOR_DELETE, PHYRE_FREE, PHYRE_MALLOC, RADIAL_GETTERS, SET_MUTE_SOUND,
            SHARRAY_COPY, SUBTITLE, SUBTITLE_MUTED, TREE_INSERT, x87,
        };

        /// A dynamic mesh's instance factory `(this; cluster, existing)`
        /// (`ret 8`; `slots`: its two argument slots, which it overwrites
        /// with the new instance): `existing` when set; else a one-entry
        /// instance list of `class` in `cluster`, its instance (`+0x34`)
        /// constructed for `this` (`ctor`, its `eax`); 0 when either is
        /// missing.
        pub fn new_instance(
            h: &mut impl Host,
            [class, ctor]: [u32; 2],
            slots: u32,
            this: u32,
            cluster: u32,
            existing: u32,
        ) -> u32 {
            if existing != 0 {
                return existing;
            }
            let c = h.rebase(class);
            let list = h.call_sym(ALLOCATE_INSTANCE_LIST, Conv::Thiscall(cluster), &[1, c, 0]);
            if list == 0 {
                return 0;
            }
            let inst: u32 = h.read(list.wrapping_add(0x34));
            h.write(slots.wrapping_add(4), inst);
            if inst == 0 {
                return 0;
            }
            h.write(slots, inst);
            h.call_sym(ctor, Conv::Thiscall(inst), &[this])
        }

        /// The memory-copy conversion kernel `(src, dst, &counts, entry)`
        /// (`0x0047a510`, cdecl; the default for members of the same type):
        /// elements of the stored member's size (`[[entry+4]+0xc]`; for an
        /// array of the runtime member's class with a fixed count
        /// (`[desc+0x30] == 1`), that size times the larger of the stored
        /// count `[[entry+4]+0x14]` and the member's), offsets and
        /// direction as the other kernels ([`super::super::bridge::kernel`]).
        pub fn copy_kernel(h: &mut impl Host, src: u32, dst: u32, counts: u32, entry: u32) {
            let stored: u32 = h.read(entry.wrapping_add(4));
            let runtime: u32 = h.read(entry.wrapping_add(8));
            let mut size: u32 = h.read(stored.wrapping_add(0xc));
            let b: u32 = h.read(runtime.wrapping_add(0x24));
            let a: u32 = h.read(stored.wrapping_add(8));
            let forward: u8 = h.read(counts.wrapping_add(0xc));
            let (sa, da) = if forward != 0 { (a, b) } else { (b, a) };
            let src_stride: u32 = h.read(counts.wrapping_add(4));
            let dst_stride: u32 = h.read(counts.wrapping_add(8));
            let count: u32 = h.read(counts);
            let desc = h.call_virtual(runtime, 1, &[]);
            if desc != 0 && h.read::<u32>(desc.wrapping_add(0x30)) == 1 {
                let stored: u32 = h.read(entry.wrapping_add(4));
                let n: u32 = h.read(stored.wrapping_add(0x14));
                let m = h.call_sym(MEMBER_COUNT, Conv::Thiscall(runtime), &[]);
                size = n.max(m).wrapping_mul(size);
            }
            let mut s = src.wrapping_add(sa);
            let mut d = dst.wrapping_add(da);
            for _ in 0..count {
                h.call_sym(MEMCPY, Conv::Cdecl, &[d, s, size]);
                s = s.wrapping_add(src_stride);
                d = d.wrapping_add(dst_stride);
            }
        }

        /// The script component's matrix scale `(this; &out, s)`
        /// (`0x00570c00`, `ret 8`): `out[i] = this[i] * s`, each product
        /// stored as a float (all sixteen before any store); `out`.
        pub fn scale_matrix(h: &mut impl Host, this: u32, out: u32, s: u32) -> u32 {
            let s = x87::load(s);
            let m: [u32; 16] = core::array::from_fn(|i| {
                let v = x87::load(h.read(this.wrapping_add(4 * i as u32)));
                x87::store(x87::mul(s, v))
            });
            for (i, v) in m.iter().enumerate() {
                h.write(out.wrapping_add(4 * i as u32), x87::store(x87::load(*v)));
            }
            out
        }

        /// `0x00570d50(L, idx)` (cdecl): the user data at `idx` when its
        /// metatable is the registry's `META_NAME` one (`lua_rawequal`),
        /// else 0 (the stack restored by `lua_settop(L, -3)` after the
        /// compare).
        pub fn test_user_data(h: &mut impl Host, l: u32, idx: u32) -> u32 {
            let p = h.call_sym(LUA[0], Conv::Cdecl, &[l, idx]);
            if p == 0 {
                return 0;
            }
            if h.call_sym(LUA[1], Conv::Cdecl, &[l, idx]) == 0 {
                return 0;
            }
            let name = h.rebase(META_NAME);
            h.call_sym(LUA[2], Conv::Cdecl, &[l, 0xfff0_b9d8, name]);
            let same = h.call_sym(LUA[3], Conv::Cdecl, &[l, u32::MAX, 0xffff_fffe]) != 0;
            h.call_sym(LUA[4], Conv::Cdecl, &[l, 0xffff_fffd]);
            if same { p } else { 0 }
        }

        /// `PSharray` copy constructor `(this; &src)` (`0x005a2380`, `ret
        /// 4`): count word 0, storage copied by `0x005a6aa0(this; count, data)`;
        /// the old storage (none, as the count word was just cleared) freed
        /// as in the other setters; the count; `+4` the storage unless
        /// inline; `this`.
        pub fn sharray_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
            h.write(this, 0u32);
            let sw: u32 = h.read(src);
            let n = sw & 0x7fff_ffff;
            let data = if n > 1 {
                h.read(src.wrapping_add(4))
            } else if n != 0 {
                src.wrapping_add(4)
            } else {
                0
            };
            let buf = h.call_sym(SHARRAY_COPY, Conv::Thiscall(this), &[n, data]);
            let word: u32 = h.read(this);
            let cnt = word & 0x7fff_ffff;
            let inline = this.wrapping_add(4);
            let old = if cnt > 1 {
                h.read(inline)
            } else if cnt != 0 {
                inline
            } else {
                0
            };
            if old != buf && old != inline && word >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(this, n);
            if buf != inline {
                h.write(inline, buf);
            }
            this
        }

        /// The animation slot map's constructor `(this; &src)`
        /// (`0x005cc670`, `ret 4`): `[this] = src`, the tree at `+8` empty
        /// (its six words the header); `n = [src+0x10]` nodes of 16 bytes
        /// (`PhyreMalloc(16n, 4)` at `+4`), each keyed by the entry's
        /// object's `+0xc` (`<< 24`, 0 without one) or its `+8` and
        /// inserted (the fix-up when new); `this`.
        pub fn slot_map(h: &mut impl Host, this: u32, src: u32) -> u32 {
            h.write(this, src);
            let tree = this.wrapping_add(8);
            for off in [0u32, 4, 8, 0x14, 0x10, 0xc] {
                h.write(tree.wrapping_add(off), tree);
            }
            let n = h.read::<u32>(src.wrapping_add(0x10)) & 0x7fff_ffff;
            let entries: u32 = h.read(src.wrapping_add(0x14));
            let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_shl(4), 4]);
            h.write(this.wrapping_add(4), buf);
            if buf == 0 {
                return this;
            }
            for i in 0..n {
                let base: u32 = h.read(this.wrapping_add(4));
                let node = base.wrapping_add(16 * i);
                h.write(node, node);
                h.write(node.wrapping_add(4), node);
                h.write(node.wrapping_add(8), node);
                let e = entries.wrapping_add(16 * i);
                let obj: u32 = h.read(e);
                let hi: u32 = if obj == 0 {
                    0
                } else {
                    h.read(obj.wrapping_add(0xc))
                };
                let lo: u32 = h.read(e.wrapping_add(8));
                h.write(node.wrapping_add(0xc), (hi << 24) | lo);
                if h.call_sym(TREE_INSERT[0], Conv::Thiscall(tree), &[node]) == 0 {
                    h.call_sym(TREE_INSERT[1], Conv::Thiscall(tree), &[node]);
                }
            }
            this
        }

        /// `RadialLineDynamicMeshInstance`'s preprocess data `(this; out, _)`
        /// (`0x00675a90`, `ret 8`): the six getters on the mesh's
        /// (`[this+4]`) `+0x1c` into `out + 0x10`..; 0.
        pub fn radial_preprocess(h: &mut impl Host, this: u32, out: u32) -> u32 {
            let mesh: u32 = h.read(this.wrapping_add(4));
            let o: u32 = h.read(mesh.wrapping_add(0x1c));
            for (i, &g) in RADIAL_GETTERS.iter().enumerate() {
                let v = h.call_sym(g, Conv::Thiscall(o), &[]);
                h.write(out.wrapping_add(0x10 + 4 * i as u32), v);
            }
            0
        }

        /// `0x006dfdd0()`: each of the three cluster ids that the cluster
        /// manager has active is released.
        pub fn release_clusters(h: &mut impl Host) {
            for g in CLUSTER_IDS {
                let at = h.rebase(g);
                let id: u32 = h.read(at);
                if id == 0 {
                    continue;
                }
                let m = h.call_sym(CLUSTERS[0], Conv::Cdecl, &[]);
                if h.call_sym(CLUSTERS[1], Conv::Thiscall(m), &[id]) as u8 == 0 {
                    continue;
                }
                let id: u32 = h.read(at);
                let m = h.call_sym(CLUSTERS[0], Conv::Cdecl, &[]);
                h.call_sym(CLUSTERS[2], Conv::Thiscall(m), &[id]);
            }
        }

        /// The menu delete: the Iggy menu's destructor, then `operator
        /// delete`.
        fn delete_menu(h: &mut impl Host, p: u32) -> u32 {
            h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(p), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p])
        }

        /// `0x006f72a0()`: the menu at `MENU` deleted and cleared,
        /// `Sg_SetMuteSound(0)`, the two words and the byte cleared;
        /// `Sg_SetMuteSound`'s `eax`.
        pub fn close_menu(h: &mut impl Host) -> u32 {
            let at = h.rebase(MENU);
            let p: u32 = h.read(at);
            if p != 0 {
                delete_menu(h, p);
            }
            h.write(at, 0u32);
            let r = h.call_sym(SET_MUTE_SOUND, Conv::Cdecl, &[0]);
            for w in MENU_WORDS {
                let a = h.rebase(w);
                h.write(a, 0u32);
            }
            let b = h.rebase(MENU_BYTE);
            h.write(b, 0u8);
            r
        }

        /// `0x006f7820()`: the seven menus of the table at `MENU_TABLE`
        /// (last first) and the table deleted; the global cleared.
        pub fn close_menus(h: &mut impl Host) {
            let at = h.rebase(MENU_TABLE);
            let t: u32 = h.read(at);
            if t != 0 {
                for i in (0..7u32).rev() {
                    let p: u32 = h.read(t.wrapping_add(4 * i));
                    if p != 0 {
                        delete_menu(h, p);
                    }
                }
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[t]);
            }
            h.write(at, 0u32);
        }

        /// `SpdCtrl_UpdateSubtitleMute_006f7230()`: in a battle
        /// (`MsBattleCheck`), subtitles muted once while the word
        /// `0x00ce82b4` is set and unmuted once when it is clear; outside
        /// battle unmuted once. The `eax`: the mute call's, else
        /// `MsBattleCheck`'s.
        pub fn subtitle_mute(h: &mut impl Host) -> u32 {
            let battle = h.call_sym(SUBTITLE[0], Conv::Cdecl, &[]);
            let muted_at = h.rebase(SUBTITLE_MUTED);
            let muted: u8 = h.read(muted_at);
            let want = battle != 0 && h.read::<u32>(h.rebase(MENU_WORDS[1])) != 0;
            if want == (muted != 0) {
                return battle;
            }
            h.write(muted_at, u8::from(want));
            h.call_sym(SUBTITLE[1], Conv::Cdecl, &[u32::from(want)])
        }

        /// `PBitmapFontCharInfo`'s array constructor `(p, n)` (`0x00a2d1a0`,
        /// cdecl): each 0x30-byte element (the slot at 0 skipped) zeroed:
        /// three words, a byte at `+0x2c`, eight float zeros.
        pub fn font_chars(h: &mut impl Host, p: u32, n: u32) {
            let mut e = p;
            for _ in 0..n {
                if e != 0 {
                    for off in [0x14u32, 0x18] {
                        h.write(e.wrapping_add(off), 0u32);
                    }
                    for off in [0u32, 4, 8] {
                        h.write(e.wrapping_add(off), 0u32);
                    }
                    h.write(e.wrapping_add(0x2c), 0u8);
                    for off in [0x10u32, 0xc, 0x20, 0x1c, 0x28, 0x24] {
                        h.write(e.wrapping_add(off), 0u32);
                    }
                }
                e = e.wrapping_add(0x30);
            }
        }
    }
}

/// Batch 5: the namespace block's byte swaps, the mapping entries'
/// cached flags, a stream helper, the D3D11 constant-buffer binder and the
/// bounding-box min/max.
pub mod swap {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The namespace block swaps: the header (`0x0047f880`, thiscall,
    /// eight words), the class records (`0x0047bc60(n, p)`, nine words
    /// each), the member records (`0x0047be20(n, p)`, six words each), the
    /// name offsets (`0x0047bf50(n, p)`, one word each).
    pub const HEADER: u32 = 0x0047_f880;
    pub const CLASSES: u32 = 0x0047_bc60;
    pub const MEMBERS: u32 = 0x0047_be20;
    pub const WORDS: u32 = 0x0047_bf50;
    /// `Phyre_PSerialization_PStreamReader_readExact` (thiscall, `ret 8`).
    pub const READ_EXACT: u32 = 0x0057_dd30;
    /// The namespace block swap (`0x0047d680`, this list).
    pub const SWAP_BLOCK: u32 = 0x0047_d680;
    /// The buffer getter `0x00593f90(array; i)` (thiscall, `ret 4`).
    pub const BUFFER_AT: u32 = 0x0059_3f90;

    pub mod bridge {
        use super::{
            BUFFER_AT, CLASSES, Conv, HEADER, Host, MEMBERS, READ_EXACT, SWAP_BLOCK, WORDS, x87,
        };

        /// Byte-swaps `n` words at `p` in place.
        pub fn swap_words(h: &mut impl Host, n: u32, p: u32) {
            let mut q = p;
            for _ in 0..n {
                let w: u32 = h.read(q);
                h.write(q, w.swap_bytes());
                q = q.wrapping_add(4);
            }
        }

        /// `n` records of `words` words at `p` swapped (`0x0047bc60`: 9,
        /// `0x0047be20`: 6; `n` 0 does nothing).
        pub fn swap_records(h: &mut impl Host, words: u32, n: u32, p: u32) {
            swap_words(h, n.wrapping_mul(words), p);
        }

        /// `0x0047d680(block)` (thiscall): the namespace block swapped: its
        /// 8-word header, then `[+0xc]` class records after the `[+8]`
        /// name offsets (`+0x20`), `[+0x10]` member records after those,
        /// the offsets themselves; the last call's `eax` is not used.
        pub fn swap_block(h: &mut impl Host, block: u32) {
            h.call_sym(HEADER, Conv::Thiscall(block), &[]);
            let names: u32 = h.read(block.wrapping_add(8));
            let classes: u32 = h.read(block.wrapping_add(0xc));
            let at = block.wrapping_add(names.wrapping_add(8).wrapping_mul(4));
            h.call_sym(CLASSES, Conv::Cdecl, &[classes, at]);
            let classes: u32 = h.read(block.wrapping_add(0xc));
            let names: u32 = h.read(block.wrapping_add(8));
            let members: u32 = h.read(block.wrapping_add(0x10));
            let at = block.wrapping_add(
                classes
                    .wrapping_mul(9)
                    .wrapping_add(8)
                    .wrapping_add(names)
                    .wrapping_mul(4),
            );
            h.call_sym(MEMBERS, Conv::Cdecl, &[members, at]);
            let names: u32 = h.read(block.wrapping_add(8));
            h.call_sym(WORDS, Conv::Cdecl, &[names, block.wrapping_add(0x20)]);
        }

        /// The mapping entry's cached "needs conversion" flags
        /// (`0x0047dc20(entry)`, thiscall): `[+0x1c]` unless -1; else
        /// `[+0x14]` or-ed with every child's (the list from `[+0]`,
        /// linked by `+0`) and every member's class entry (`[+0x10]`, 32
        /// bytes per member of the class `[+4]`'s `+0xc`; the member `+8`
        /// without bit 1 of its `+0x14` byte, its entry `+0x14`), cached.
        pub fn needs_conversion(h: &mut impl Host, entry: u32) -> u32 {
            let cached: u32 = h.read(entry.wrapping_add(0x1c));
            if cached != u32::MAX {
                return cached;
            }
            let mut acc: u32 = h.read(entry.wrapping_add(0x14));
            let mut child: u32 = h.read(entry);
            while child != 0 {
                acc |= needs_conversion(h, child);
                child = h.read(child);
            }
            let class: u32 = h.read(entry.wrapping_add(4));
            if class != 0 {
                let n: u32 = h.read(class.wrapping_add(0xc));
                for i in 0..n {
                    let members: u32 = h.read(entry.wrapping_add(0x10));
                    let e = members.wrapping_add(i.wrapping_mul(0x20));
                    let sub: u32 = h.read(e.wrapping_add(0x14));
                    let member: u32 = h.read(e.wrapping_add(8));
                    if member != 0 && h.read::<u8>(member.wrapping_add(0x14)) & 2 == 0 && sub != 0 {
                        acc |= needs_conversion(h, sub);
                    }
                }
            }
            h.write(entry.wrapping_add(0x1c), acc);
            acc
        }

        /// `0x00579010(reader, swap, block, size)` (cdecl): `readExact(block,
        /// size)` on `reader` when both are set (else 0), then the block
        /// swapped (`0x0047d680`) when it is set and the byte `swap` is;
        /// the read's result.
        pub fn read_block(h: &mut impl Host, reader: u32, swap: u32, block: u32, size: u32) -> u32 {
            let r = if size != 0 && block != 0 {
                h.call_sym(READ_EXACT, Conv::Thiscall(reader), &[block, size])
            } else {
                0
            };
            if block != 0 && swap & 0xff != 0 {
                h.call_sym(SWAP_BLOCK, Conv::Thiscall(block), &[]);
            }
            r
        }

        /// The constant buffer bind `(this; array, slot, index)`
        /// (`0x00598d00`, `ret 0xc`; `slots`: its argument slots, whose
        /// addresses it hands the context): 5 without a context (`+0x70`)
        /// or for an index past the array's count (`[array+0x14]`, one at
        /// least) or without a buffer there; else the context's method `+0x110`
        /// (stdcall, `this` first: `(ctx, 0, 1, &buffer, &slot)`) with the
        /// buffer, then again with a null buffer (the index slot cleared);
        /// 0.
        pub fn bind_buffer(
            h: &mut impl Host,
            slots: u32,
            this: u32,
            array: u32,
            index: u32,
        ) -> u32 {
            let ctx: u32 = h.read(this.wrapping_add(0x70));
            if ctx == 0 {
                return 5;
            }
            let mut n = h.read::<u32>(array.wrapping_add(0x14)) & 0x7fff_ffff;
            if n == 0 {
                n = 1;
            }
            if index >= n {
                return 5;
            }
            if h.call_sym(BUFFER_AT, Conv::Thiscall(array), &[index]) == 0 {
                return 5;
            }
            let buffer = h.call_sym(BUFFER_AT, Conv::Thiscall(array), &[index]);
            h.write(slots, buffer);
            let vt: u32 = h.read(ctx);
            let f: u32 = h.read(vt.wrapping_add(0x110));
            h.call(f, Conv::Stdcall, &[ctx, 0, 1, slots, slots.wrapping_add(4)]);
            let ctx: u32 = h.read(this.wrapping_add(0x70));
            h.write(slots.wrapping_add(8), 0u32);
            let vt: u32 = h.read(ctx);
            let f: u32 = h.read(vt.wrapping_add(0x110));
            h.call(
                f,
                Conv::Stdcall,
                &[ctx, 0, 1, slots.wrapping_add(8), slots.wrapping_add(4)],
            );
            0
        }

        /// The bounding boxes' component max (`0x00a1b560`) / min
        /// (`0x00a1b620`) `(&out, &a, &b)` (cdecl): per component `b` when
        /// `b >= a` (max) / `b <= a` (min) or unordered, else `a`; floats
        /// through the x87; `&out`.
        pub fn min_max(h: &mut impl Host, max: bool, out: u32, a: u32, b: u32) -> u32 {
            let mut r = [0u32; 4];
            for (k, slot) in r.iter_mut().enumerate() {
                let off = 4 * k as u32;
                let x = x87::load(h.read(a.wrapping_add(off)));
                let y = x87::load(h.read(b.wrapping_add(off)));
                let pick_b = x.is_nan() || y.is_nan() || if max { y >= x } else { y <= x };
                *slot = x87::store(if pick_b { y } else { x });
            }
            for (k, v) in r.iter().enumerate() {
                h.write(out.wrapping_add(4 * k as u32), *v);
            }
            out
        }
    }
}

/// Batch 6: the namespace mapping's conversion set-up and the conversion
/// driver.
pub mod mapping {
    use ffx_graphic::host::{Conv, Host};

    /// The mapping's current generation (`0x0047c080` compares the
    /// mapping's `+0x1c` with it).
    pub const GENERATION: u32 = 0x00c9_75e8;
    /// The kernels: no conversion (`0x0047a670`), the memory copy
    /// (`0x0047a510`), the one-byte copy (`0x00476c20`).
    pub const NO_KERNEL: u32 = 0x0047_a670;
    pub const COPY_KERNEL: u32 = 0x0047_a510;
    pub const BYTE_KERNEL: u32 = 0x0047_6c20;
    /// The registry lookup `(from, to)` (cdecl) -> a kernel or 0.
    pub const FIND_KERNEL: u32 = 0x0047_b630;
    /// The member's element count (`[member+0x34]`, thiscall).
    pub const MEMBER_COUNT: u32 = 0x0057_6050;
    /// The conversion driver (`0x0047c610`, this list).
    pub const DRIVER: u32 = 0x0047_c610;

    /// A member's element size as the copies see it: 4 for a pointer
    /// member (`+0x14` bit 1) or a class array stored by reference (its
    /// descriptor kind `+0x30` 2 without `+0x14` bit 6), else its type's
    /// size (`[[member+0x18]+0x1c]`).
    fn element_size(h: &mut impl Host, member: u32) -> u32 {
        let ty: u32 = h.read(member.wrapping_add(0x18));
        let size: u32 = h.read(ty.wrapping_add(0x1c));
        let desc = h.call_virtual(member, 1, &[]);
        let flags: u32 = h.read(member.wrapping_add(0x14));
        let by_ref = desc != 0 && h.read::<u32>(desc.wrapping_add(0x30)) == 2;
        let pointer = (!(flags >> 6) & u32::from(by_ref)) | (flags >> 1);
        if pointer & 1 != 0 { 4 } else { size }
    }

    pub mod bridge {
        use super::{
            BYTE_KERNEL, COPY_KERNEL, Conv, DRIVER, FIND_KERNEL, GENERATION, Host, MEMBER_COUNT,
            NO_KERNEL, element_size,
        };
        use crate::m5p_registry_b::swap::bridge::needs_conversion;

        /// `0x0047c080(mapping)` (thiscall): when the mapping's `+0x1c` is
        /// the current generation, every entry's `+0x1c` gets it; else
        /// every entry's flags cache is reset (-1) and recomputed
        /// ([`needs_conversion`]).
        pub fn update_flags(h: &mut impl Host, mapping: u32) {
            let n = h.read::<u32>(mapping.wrapping_add(0xc)) & 0x7fff_ffff;
            let g = h.rebase(GENERATION);
            let generation: u32 = h.read(g);
            if h.read::<u32>(mapping.wrapping_add(0x1c)) == generation {
                for i in 0..n {
                    let entries: u32 = h.read(mapping.wrapping_add(0x10));
                    let v: u32 = h.read(g);
                    h.write(entries.wrapping_add(32 * i + 0x1c), v);
                }
                return;
            }
            for i in 0..n {
                let entries: u32 = h.read(mapping.wrapping_add(0x10));
                h.write(entries.wrapping_add(32 * i + 0x1c), u32::MAX);
            }
            for i in 0..n {
                let entries: u32 = h.read(mapping.wrapping_add(0x10));
                needs_conversion(h, entries.wrapping_add(32 * i));
            }
        }

        /// `0x0047e180(entry; to_runtime)` (thiscall, `ret 4`): the next
        /// entry (`[+0]`) first, then a kernel for each member of the
        /// class (`[+4]`'s `+0xc` members, 32-byte records at `[+0x10]`,
        /// the kernel at `+0x1c`): none without a runtime member (`+8`);
        /// for different types (`[rt+0x18]` vs `+0x10`) the registry's
        /// `(stored, runtime)` pair (swapped unless `to_runtime`), none
        /// when missing; for the same type none when the element sizes
        /// differ or a fixed-count class array's counts do, else the
        /// memory copy, or the one-byte copy for one byte of a plain type.
        pub fn assign_kernels(h: &mut impl Host, entry: u32, to_runtime: u32) {
            let next: u32 = h.read(entry);
            if next != 0 {
                assign_kernels(h, next, to_runtime);
            }
            let class: u32 = h.read(entry.wrapping_add(4));
            if class == 0 {
                return;
            }
            let n: u32 = h.read(class.wrapping_add(0xc));
            let members: u32 = h.read(entry.wrapping_add(0x10));
            let none = h.rebase(NO_KERNEL);
            for i in 0..n {
                let e = members.wrapping_add(32 * i);
                let slot = e.wrapping_add(0x1c);
                let rt: u32 = h.read(e.wrapping_add(8));
                if rt == 0 {
                    h.write(slot, none);
                    continue;
                }
                let st: u32 = h.read(e.wrapping_add(4));
                let size: u32 = h.read(st.wrapping_add(0xc));
                let rtype: u32 = h.read(rt.wrapping_add(0x18));
                let stype: u32 = h.read(e.wrapping_add(0x10));
                let flags = h.read::<u32>(rt.wrapping_add(0x14)) & !1;
                let desc = h.call_virtual(rt, 1, &[]);
                let kind: u32 = if desc == 0 {
                    0
                } else {
                    h.read(desc.wrapping_add(0x30))
                };
                let fixed = if kind == 1 { rt } else { 0 };
                if rtype != stype {
                    let (a, b) = if to_runtime & 0xff != 0 {
                        (stype, rtype)
                    } else {
                        (rtype, stype)
                    };
                    let k = h.call_sym(FIND_KERNEL, Conv::Cdecl, &[a, b]);
                    h.write(slot, if k == 0 { none } else { k });
                    continue;
                }
                if element_size(h, rt) != size {
                    h.write(slot, none);
                    continue;
                }
                let st_offset: u32 = h.read(st.wrapping_add(8));
                let st_flags: u32 = h.read(st.wrapping_add(0x10));
                if h.read::<u32>(rt.wrapping_add(0x24)) == st_offset
                    && flags == st_flags
                    && fixed != 0
                {
                    let count: u32 = h.read(st.wrapping_add(0x14));
                    if h.call_sym(MEMBER_COUNT, Conv::Thiscall(fixed), &[]) != count {
                        h.write(slot, none);
                        continue;
                    }
                }
                let k = if desc == 0 && size.wrapping_sub(1) == 0 {
                    BYTE_KERNEL
                } else {
                    COPY_KERNEL
                };
                let k = h.rebase(k);
                h.write(slot, k);
            }
        }

        /// One member's conversion in the driver: the children's mapping
        /// recursively per element for a nested class, else its kernel.
        fn convert_member(h: &mut impl Host, e: u32, counts: u32, src: u32, dst: u32, dir: bool) {
            let rt: u32 = h.read(e.wrapping_add(8));
            let st: u32 = h.read(e.wrapping_add(4));
            if rt == 0 || st == 0 {
                return;
            }
            let sub: u32 = h.read(e.wrapping_add(0x10));
            if sub == 0 {
                return;
            }
            let rtype: u32 = h.read(rt.wrapping_add(0x18));
            let t1 = h.call_virtual(rtype, 1, &[]);
            let t2 = h.call_virtual(sub, 1, &[]);
            if t1 != t2 {
                return;
            }
            let flags: u32 = h.read(rt.wrapping_add(0x14));
            let ptr_bit = (flags >> 1) & 1;
            let by_value = (flags >> 6) & 1;
            let desc = h.call_virtual(rt, 1, &[]);
            let kind: u32 = if desc == 0 {
                0
            } else {
                h.read(desc.wrapping_add(0x30))
            };
            let as_pointer = (u32::from(kind == 2) & u32::from(by_value == 0)) | ptr_bit != 0;
            if dir && as_pointer {
                return;
            }
            let count = if kind == 1 {
                h.call_sym(MEMBER_COUNT, Conv::Thiscall(rt), &[])
            } else {
                1
            };
            let child: u32 = h.read(e.wrapping_add(0x14));
            if child == 0 || as_pointer {
                let mut k: u32 = h.read(e.wrapping_add(0x1c));
                if k == 0 {
                    let mut size = element_size(h, rt);
                    k = h.rebase(COPY_KERNEL);
                    if desc == 0 {
                        size = size.wrapping_sub(1);
                        if size == 0 {
                            k = h.rebase(BYTE_KERNEL);
                        }
                    }
                }
                h.call(k, Conv::Cdecl, &[src, dst, counts, e]);
                return;
            }
            let size = element_size(h, rt);
            let st_offset: u32 = h.read(st.wrapping_add(8));
            let rt_offset: u32 = h.read(rt.wrapping_add(0x24));
            let (so, d_o) = if dir {
                (st_offset, rt_offset)
            } else {
                (rt_offset, st_offset)
            };
            let mut s = src.wrapping_add(so);
            let delta = dst.wrapping_add(d_o).wrapping_sub(s);
            for _ in 0..count {
                h.call_sym(
                    DRIVER,
                    Conv::Thiscall(child),
                    &[counts, s, s.wrapping_add(delta)],
                );
                s = s.wrapping_add(size);
            }
        }

        /// The conversion driver `(mapping; &counts, src, dst)`
        /// (`0x0047c610`, `ret 0xc`; `counts` as the kernels', its byte
        /// `+0xc` the direction): for this mapping and each base class's
        /// (the chain from `[+0]`, skipping a base whose class `+8` is the
        /// header's `+0x40` ... kept), every member entry with both members
        /// and a matching sub-type converts ([`convert_member`]); `src` and
        /// `dst` then advance by the stored (`[m8+0x84]`) and runtime
        /// (`[m4+0x10]`) base offsets. `eax`: the last `dst` advance.
        pub fn driver(h: &mut impl Host, mapping: u32, counts: u32, src: u32, dst: u32) -> u32 {
            let dir = h.read::<u8>(counts.wrapping_add(0xc)) != 0;
            let mut map = mapping;
            let mut m8: u32 = h.read(map.wrapping_add(8));
            let mut m4: u32 = h.read(map.wrapping_add(4));
            let (mut src, mut dst) = (src, dst);
            let mut process = true;
            loop {
                if process {
                    let entries: u32 = h.read(map.wrapping_add(0x10));
                    let n: u32 = if m4 == 0 {
                        0
                    } else {
                        h.read(m4.wrapping_add(0xc))
                    };
                    let end = entries.wrapping_add(n.wrapping_shl(5));
                    let mut e = entries;
                    while e < end {
                        convert_member(h, e, counts, src, dst, dir);
                        e = e.wrapping_add(0x20);
                    }
                }
                let next: u32 = h.read(map);
                let skip = next != 0
                    && m8 != 0
                    && h.read::<u32>(next.wrapping_add(8)) != h.read::<u32>(m8.wrapping_add(0x40));
                let a: u32 = if m8 == 0 {
                    0
                } else {
                    h.read(m8.wrapping_add(0x84))
                };
                let c: u32 = if m4 == 0 {
                    0
                } else {
                    h.read(m4.wrapping_add(0x10))
                };
                let (sa, da) = if dir { (c, a) } else { (a, c) };
                src = src.wrapping_add(sa);
                dst = dst.wrapping_add(da);
                if next == 0 {
                    return da;
                }
                map = next;
                m4 = h.read(map.wrapping_add(4));
                m8 = h.read(map.wrapping_add(8));
                process = !skip;
            }
        }
    }
}

/// Batch 7: the bounding-box out-codes, the bone matrix palettes and the
/// polygon clipper of the application layer.
pub mod geometry {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `PMatrix4_Inverse_004cc0c0(out, m)` (cdecl) -> `out`.
    pub const INVERSE: u32 = 0x004c_c0c0;
    /// `PMatrix4_Copy_004c9430(this; m)` (thiscall, `ret 4`).
    pub const COPY: u32 = 0x004c_9430;
    /// `PMatrix4_MulThis_004c9da0(a; out, b)` (thiscall, `ret 8`) -> `out`.
    pub const MUL: u32 = 0x004c_9da0;
    /// The clipper's edge intersection `(out, a, b, distance)` (cdecl) ->
    /// `out`.
    pub const INTERSECT: u32 = 0x00a2_f560;

    pub mod bridge {
        use super::{COPY, Conv, Host, INTERSECT, INVERSE, MUL, x87};

        /// The point's out-code against a box `(&min, &max, &p)`
        /// (`0x005a4180`, cdecl): per axis `k`, bit `2k` when `min[k] >
        /// p[k]`, bit `2k + 1` when `max[k] < p[k]` (false when unordered).
        pub fn outcode(h: &mut impl Host, min: u32, max: u32, p: u32) -> u32 {
            let mut code = 0;
            for k in 0..3u32 {
                let v = x87::load(h.read(p.wrapping_add(4 * k)));
                let lo = x87::load(h.read(min.wrapping_add(4 * k)));
                let hi = x87::load(h.read(max.wrapping_add(4 * k)));
                if lo > v {
                    code |= 1 << (2 * k);
                }
                if hi < v {
                    code |= 2 << (2 * k);
                }
            }
            code
        }

        /// Copies the 16 floats of the matrix at `m` to `out` (through the
        /// x87).
        fn store_matrix(h: &mut impl Host, out: u32, m: u32) {
            for k in 0..16u32 {
                let f: u32 = h.read(m.wrapping_add(4 * k));
                h.write(out.wrapping_add(4 * k), x87::store(x87::load(f)));
            }
        }

        /// The skinned bone palette `(out, &&skin)` (`0x005c8830`, cdecl;
        /// `frame`: 0x40 bytes the matrix products go through, as the
        /// original's locals): the size `64n` (`n = [[skin+0x1c]+8]`), and
        /// with `out`, `out[i] = mats[lo] * bones[hi]` for each pair `{lo,
        /// hi}` (u16, `[[skin+0x1c]+0xc]`), the matrices `[skin+0x14]` and
        /// `[[[skin+0x10]]+0xc]`.
        pub fn palette(h: &mut impl Host, frame: u32, out: u32, skin: u32) -> u32 {
            let s: u32 = h.read(skin);
            let d: u32 = h.read(s.wrapping_add(0x1c));
            let n = h.read::<u32>(d.wrapping_add(8)) & 0x7fff_ffff;
            let size = n.wrapping_shl(6);
            if out == 0 {
                return size;
            }
            let b0: u32 = h.read(s.wrapping_add(0x10));
            let b1: u32 = h.read(b0);
            let bones: u32 = h.read(b1.wrapping_add(0xc));
            let pairs: u32 = h.read(d.wrapping_add(0xc));
            let mats: u32 = h.read(s.wrapping_add(0x14));
            for i in 0..n {
                let lo: u16 = h.read(pairs.wrapping_add(4 * i));
                let hi: u16 = h.read(pairs.wrapping_add(4 * i + 2));
                let a = mats.wrapping_add(u32::from(lo) << 6);
                let b = bones.wrapping_add(u32::from(hi) << 6);
                let r = h.call_sym(MUL, Conv::Thiscall(a), &[frame, b]);
                store_matrix(h, out.wrapping_add(64 * i), r);
            }
            size
        }

        /// The palette relative to the first matrix `(out, &&skin)`
        /// (`0x005c8700`; `frame`: 0xc8 bytes, the original's three local
        /// matrices at `+0`, `+0x40`, `+0x80`): as [`palette`] with
        /// `out[i] = inverse(mats[-1]) * mats[lo] * bones[hi]`.
        pub fn palette_relative(h: &mut impl Host, frame: u32, out: u32, skin: u32) -> u32 {
            let s: u32 = h.read(skin);
            let d: u32 = h.read(s.wrapping_add(0x1c));
            let n = h.read::<u32>(d.wrapping_add(8)) & 0x7fff_ffff;
            let size = n.wrapping_shl(6);
            if out == 0 {
                return size;
            }
            let b0: u32 = h.read(s.wrapping_add(0x10));
            let b1: u32 = h.read(b0);
            let bones: u32 = h.read(b1.wrapping_add(0xc));
            let pairs: u32 = h.read(d.wrapping_add(0xc));
            let mats: u32 = h.read(s.wrapping_add(0x14));
            let (m0, m1, inv) = (frame, frame.wrapping_add(0x40), frame.wrapping_add(0x80));
            let r = h.call_sym(INVERSE, Conv::Cdecl, &[m1, mats.wrapping_sub(0x40)]);
            h.call_sym(COPY, Conv::Thiscall(inv), &[r]);
            for i in 0..n {
                let lo: u16 = h.read(pairs.wrapping_add(4 * i));
                let hi: u16 = h.read(pairs.wrapping_add(4 * i + 2));
                let a = mats.wrapping_add(u32::from(lo) << 6);
                let b = bones.wrapping_add(u32::from(hi) << 6);
                let r1 = h.call_sym(MUL, Conv::Thiscall(inv), &[m0, a]);
                let r2 = h.call_sym(MUL, Conv::Thiscall(r1), &[m1, b]);
                store_matrix(h, out.wrapping_add(64 * i), r2);
            }
            size
        }

        /// Appends the 16-byte vertex at `v` (four floats through the x87)
        /// to `out`, counting in `*count`.
        fn emit(h: &mut impl Host, out: u32, count: u32, v: u32) {
            let c: u32 = h.read(count);
            let dst = out.wrapping_add(c.wrapping_shl(4));
            h.write(count, c.wrapping_add(1));
            for k in 0..4u32 {
                let f: u32 = h.read(v.wrapping_add(4 * k));
                h.write(dst.wrapping_add(4 * k), x87::store(x87::load(f)));
            }
        }

        /// The polygon clip against one plane `(in, n, out, &count,
        /// distance)` (`0x00a2f5e0`, cdecl; `distance(&v)` a cdecl callback
        /// returning a float; `frame`: the original's 16-byte local the
        /// intersection goes through): Sutherland-Hodgman, a vertex kept
        /// unless its distance is above 0 (NaN keeps it), each edge that
        /// crosses the plane cut by `0x00a2f560`; `out`.
        pub fn clip(
            h: &mut impl Host,
            frame: u32,
            input: u32,
            n: u32,
            out: u32,
            count: u32,
            distance: u32,
        ) -> u32 {
            h.write(count, 0u32);
            if n == 0 {
                return out;
            }
            let mut prev = input.wrapping_add(n.wrapping_sub(1).wrapping_shl(4));
            let mut prev_out = h.call_st0_f64(distance, Conv::Cdecl, &[prev]) > 0.0;
            if !prev_out {
                emit(h, out, count, prev);
            }
            let mut cur = input;
            for _ in 0..n {
                let cur_out = h.call_st0_f64(distance, Conv::Cdecl, &[cur]) > 0.0;
                if prev_out == cur_out {
                    if !cur_out {
                        emit(h, out, count, cur);
                    }
                } else {
                    let c: u32 = h.read(count);
                    let dst = out.wrapping_add(c.wrapping_shl(4));
                    h.write(count, c.wrapping_add(1));
                    let (a, b) = if prev_out { (cur, prev) } else { (prev, cur) };
                    let r = h.call_sym(INTERSECT, Conv::Cdecl, &[frame, a, b, distance]);
                    for k in 0..4u32 {
                        let f: u32 = h.read(r.wrapping_add(4 * k));
                        h.write(dst.wrapping_add(4 * k), x87::store(x87::load(f)));
                    }
                    if prev_out {
                        emit(h, out, count, cur);
                    }
                }
                prev = cur;
                cur = cur.wrapping_add(0x10);
                prev_out = cur_out;
            }
            out
        }
    }
}

/// Batch 8: the point-in-box test, the element conversion entry point and
/// the physics world's contact arrays.
pub mod contacts {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PClassDescriptor_getDefaultBuffer` (thiscall).
    pub const DEFAULT_BUFFER: u32 = 0x0043_cb80;
    /// The CRT's `memcpy` thunk (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// The conversion driver (`0x0047c610`, this list).
    pub const DRIVER: u32 = 0x0047_c610;
    /// `Phyre_PhyreMalloc(size, align)` / `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// The second contact array's element constructor `(p, n)` (cdecl).
    pub const CONSTRUCT_40: u32 = 0x005f_0950;
    /// The contact append (`0x005f36d0`, this list) and the two resizes.
    pub const APPEND: u32 = 0x005f_36d0;
    pub const RESIZE: [u32; 2] = [0x0060_2900, 0x0060_29a0];

    /// The `PArray<T>` resizes `(this; n)`: `(copy, element words, copy
    /// constructor, default constructor of the new tail)`.
    pub const RESIZES: &[(u32, u32, u32, u32)] = &[
        (0x0060_2900, 9, 0x005d_78b0, 0),
        (0x0060_29a0, 10, 0x005d_78e0, CONSTRUCT_40),
        (0x0060_2a60, 10, 0x005d_7910, 0),
    ];

    /// The [`RESIZES`] row of `f` (a compile error when missing).
    #[must_use]
    pub const fn resize_of(f: u32) -> (u32, u32, u32, u32) {
        let mut i = 0;
        while i < RESIZES.len() {
            if RESIZES[i].0 == f {
                return RESIZES[i];
            }
            i += 1;
        }
        panic!("not a resize of this list");
    }

    pub mod bridge {
        use super::{
            Conv, DEFAULT_BUFFER, DRIVER, Host, MEMCPY, PHYRE_FREE, PHYRE_MALLOC, RESIZE, x87,
        };

        /// The point-in-box test `(&p, &min, &max)` (`0x00a2fc90`, cdecl):
        /// 1 unless a component of `p` is above `max` or below `min`
        /// (ordered compares: NaN is inside), else 0.
        pub fn inside(h: &mut impl Host, p: u32, min: u32, max: u32) -> u32 {
            let mut out = false;
            for k in 0..3u32 {
                let v = x87::load(h.read(p.wrapping_add(4 * k)));
                let lo = x87::load(h.read(min.wrapping_add(4 * k)));
                let hi = x87::load(h.read(max.wrapping_add(4 * k)));
                out |= v > hi || v < lo;
            }
            u32::from(!out)
        }

        /// Converts `n` stored elements `(entry, src, dst, n)`
        /// (`0x005788f0`, cdecl; `frame`: the 16-byte counts the original
        /// builds in its frame): `dst` filled with the runtime class's
        /// default buffer (`[entry+8]`, its size `+0x1c`) when it has one,
        /// then the driver on `entry` with `{n, stored stride ([[entry+4]+4]
        /// & 0x0fffffff), runtime size, direction 1}`, the pointers moved
        /// from the stored base (`[st+0x14]` to `[st+0x18]`) and the runtime
        /// one (`[rt+0x88]` to `[rt+0x8c]`); the driver's `eax`.
        pub fn convert_elements(
            h: &mut impl Host,
            frame: u32,
            entry: u32,
            src: u32,
            dst: u32,
            n: u32,
        ) -> u32 {
            let rt: u32 = h.read(entry.wrapping_add(8));
            let st: u32 = h.read(entry.wrapping_add(4));
            let size: u32 = h.read(rt.wrapping_add(0x1c));
            let buf = h.call_sym(DEFAULT_BUFFER, Conv::Thiscall(rt), &[]);
            if buf != 0 {
                let end = n.wrapping_mul(size).wrapping_add(dst);
                let mut p = dst;
                while p < end {
                    h.call_sym(MEMCPY, Conv::Cdecl, &[p, buf, size]);
                    p = p.wrapping_add(size);
                }
            }
            let st_to: u32 = h.read(st.wrapping_add(0x18));
            let st_from: u32 = h.read(st.wrapping_add(0x14));
            let rt_to: u32 = h.read(rt.wrapping_add(0x8c));
            let rt_from: u32 = h.read(rt.wrapping_add(0x88));
            let stride = h.read::<u32>(st.wrapping_add(4)) & 0x0fff_ffff;
            h.write(frame, n);
            h.write(frame.wrapping_add(4), stride);
            h.write(frame.wrapping_add(8), size);
            h.write(frame.wrapping_add(0xc), 1u8);
            let s = src.wrapping_sub(st_from).wrapping_add(st_to);
            let d = dst.wrapping_sub(rt_from).wrapping_add(rt_to);
            h.call_sym(DRIVER, Conv::Thiscall(entry), &[frame, s, d])
        }

        /// A `PArray<T>` resize `(this; n)` ([`super::RESIZES`]): 0 when the
        /// count word is `n`; new storage of `n` elements (0xd when it
        /// fails) holding the first `min(n, count)` (`copy(new, old, k)`)
        /// and, with a constructor, the rest default-built; the old
        /// owned, non-null storage freed unless reused; 0.
        pub fn resize(
            h: &mut impl Host,
            words: u32,
            copy: u32,
            ctor: u32,
            this: u32,
            n: u32,
        ) -> u32 {
            let word: u32 = h.read(this);
            if n == word {
                return 0;
            }
            let keep = n.min(word & 0x7fff_ffff);
            let old: u32 = h.read(this.wrapping_add(4));
            let mut buf = 0;
            if n != 0 {
                buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4 * words), 4]);
                if buf == 0 {
                    return 0xd;
                }
                h.call_sym(copy, Conv::Cdecl, &[buf, old, keep]);
                if ctor != 0 {
                    let tail = buf.wrapping_add(keep.wrapping_mul(4 * words));
                    h.call_sym(ctor, Conv::Cdecl, &[tail, n.wrapping_sub(keep)]);
                }
            }
            let now: u32 = h.read(this.wrapping_add(4));
            let w: u32 = h.read(this);
            if now != buf && w >> 31 == 0 && now != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[now]);
            }
            h.write(this.wrapping_add(4), buf);
            h.write(this, n);
            0
        }

        /// The physics world's contact append `(this; &a, &b, &index, c)`
        /// (`0x005f36d0`, `ret 0x10`): both arrays grown to `2cap + 1`
        /// when full (`[+0x3c]` vs `[+0x90]`; `+0x80` and `+0x88`); the
        /// record at the end of `+0x84` (36 bytes): `a` (3 floats), `b` at
        /// `+0x10`, `c` at `+0x20`; `*index` its index; the count bumped; 0.
        pub fn append(h: &mut impl Host, this: u32, a: u32, b: u32, index: u32, c: u32) -> u32 {
            let cap: u32 = h.read(this.wrapping_add(0x90));
            if h.read::<u32>(this.wrapping_add(0x3c)) >= cap {
                let cap = cap.wrapping_mul(2).wrapping_add(1);
                h.write(this.wrapping_add(0x90), cap);
                h.call_sym(RESIZE[0], Conv::Thiscall(this.wrapping_add(0x80)), &[cap]);
                let cap: u32 = h.read(this.wrapping_add(0x90));
                h.call_sym(RESIZE[1], Conv::Thiscall(this.wrapping_add(0x88)), &[cap]);
            }
            let count: u32 = h.read(this.wrapping_add(0x3c));
            h.write(index, count);
            for (base, from) in [(0u32, a), (0x10, b)] {
                let i: u32 = h.read(index);
                let data: u32 = h.read(this.wrapping_add(0x84));
                let rec = data.wrapping_add(i.wrapping_mul(36)).wrapping_add(base);
                for k in 0..3u32 {
                    let f: u32 = h.read(from.wrapping_add(4 * k));
                    h.write(rec.wrapping_add(4 * k), x87::store(x87::load(f)));
                }
            }
            let i: u32 = h.read(index);
            let data: u32 = h.read(this.wrapping_add(0x84));
            h.write(data.wrapping_add(i.wrapping_mul(36)).wrapping_add(0x20), c);
            let n: u32 = h.read(this.wrapping_add(0x3c));
            h.write(this.wrapping_add(0x3c), n.wrapping_add(1));
            0
        }

        /// `0x005f31e0(this; &a, &b, c)` (`ret 0xc`; `frame`: its local
        /// index): the append's index, or -1 when it fails.
        pub fn add_contact(
            h: &mut impl Host,
            frame: u32,
            this: u32,
            a: u32,
            b: u32,
            c: u32,
        ) -> u32 {
            h.write(frame, 0u32);
            let r = h.call_sym(super::APPEND, Conv::Thiscall(this), &[a, b, frame, c]);
            if r == 0 { h.read(frame) } else { u32::MAX }
        }
    }
}

/// Batch 9: the skin modifier's CPU kernel for four bones per vertex.
pub mod skin {
    use ffx_bridge::x87;
    use ffx_graphic::host::Host;
    use ffx_magic::sb_sse;

    /// Four `f32` lanes.
    type Lanes = [f32; 4];

    /// `mulps a, b` lane by lane.
    fn mul4(a: Lanes, b: Lanes) -> Lanes {
        core::array::from_fn(|i| sb_sse::mul(a[i], b[i]))
    }

    /// `addps a, b` lane by lane.
    fn add4(a: Lanes, b: Lanes) -> Lanes {
        core::array::from_fn(|i| sb_sse::add(a[i], b[i]))
    }

    /// One lane value in all four (`movss` + `shufps 0`).
    fn splat(x: f32) -> Lanes {
        [x; 4]
    }

    fn lanes(h: &impl Host, at: u32) -> Lanes {
        core::array::from_fn(|i| f32::from_bits(h.read(at.wrapping_add(4 * i as u32))))
    }

    fn put4(h: &mut impl Host, at: u32, v: Lanes) {
        for (i, x) in v.iter().enumerate() {
            h.write(at.wrapping_add(4 * i as u32), x.to_bits());
        }
    }

    fn scalar(h: &impl Host, at: u32) -> f32 {
        f32::from_bits(h.read(at))
    }

    /// A bone matrix's column `j` (16 bytes at `bone + 16j`).
    fn column(h: &impl Host, bone: u32, j: u32) -> Lanes {
        lanes(h, bone.wrapping_add(16 * j))
    }

    /// The x87 tail's `((c0 x + c1 y) + c2 z) [+ c3]` for component `k`
    /// of the bone at `m`, stored as a float.
    fn transform_x87(h: &impl Host, m: u32, k: u32, v: [f64; 3], translate: bool) -> f64 {
        let c = |j: u32| x87::load(h.read(m.wrapping_add(16 * j + 4 * k)));
        let s = x87::add(x87::mul(c(0), v[0]), x87::mul(c(1), v[1]));
        let s = x87::add(s, x87::mul(c(2), v[2]));
        let s = if translate { x87::add(s, c(3)) } else { s };
        x87::load(x87::store(s))
    }

    pub mod bridge {
        use super::{Host, add4, column, mul4, put4, scalar, splat, transform_x87, x87};

        /// The four-bone skinning kernel `(bones, n, &src, &dst)`
        /// (`0x005c8930`, cdecl; `src = {positions, normals, weights,
        /// indices}`, `dst = {positions, normals}`, 12-byte vectors, four
        /// weights and four byte indices per vertex, 64-byte bone matrices
        /// by column): the normal through bone 0's 3x3, the position
        /// through each bone weighted, summed. All vertices but a last one
        /// when `n` is a multiple of 4 run on SSE and store 16 bytes each
        /// (the fourth lane spills into the next vector); that last one
        /// runs on the x87 and stores 12.
        /// The positions-only four-bone kernel `(bones, n, &src, &dst)`
        /// (`0x005c94f0`, cdecl; `src = {positions, weights, indices}`,
        /// `dst = {positions}`): as [`skin4`] without normals, every bone
        /// `((c0 x + c3) + c1 y) + c2 z` on SSE.
        pub fn skin4_positions(h: &mut impl Host, bones: u32, n: u32, src: u32, dst: u32) {
            if n == 0 {
                return;
            }
            let mut pos: u32 = h.read(src);
            let mut w: u32 = h.read(src.wrapping_add(4));
            let mut idx: u32 = h.read(src.wrapping_add(8));
            let mut pout: u32 = h.read(dst);
            let simd = if n & 3 != 0 { n } else { n - 1 };
            for _ in 0..simd {
                let x = splat(scalar(h, pos));
                let y = splat(scalar(h, pos.wrapping_add(4)));
                let z = splat(scalar(h, pos.wrapping_add(8)));
                let mut acc = [0.0f32; 4];
                for k in [1u32, 0, 2, 3] {
                    let i: u8 = h.read(idx.wrapping_add(k));
                    let b = bones.wrapping_add(u32::from(i) << 6);
                    let p = add4(
                        add4(
                            add4(mul4(column(h, b, 0), x), column(h, b, 3)),
                            mul4(column(h, b, 1), y),
                        ),
                        mul4(column(h, b, 2), z),
                    );
                    let pw = mul4(p, splat(scalar(h, w.wrapping_add(4 * k))));
                    acc = if k == 1 { pw } else { add4(acc, pw) };
                }
                put4(h, pout, acc);
                pos = pos.wrapping_add(0xc);
                pout = pout.wrapping_add(0xc);
                w = w.wrapping_add(0x10);
                idx = idx.wrapping_add(4);
            }
            if simd == n {
                return;
            }
            let p: [f64; 3] =
                core::array::from_fn(|k| x87::load(h.read(pos.wrapping_add(4 * k as u32))));
            let mut acc: [f64; 3] = [0.0; 3];
            for k in 0..4u32 {
                let b = bones.wrapping_add(u32::from(h.read::<u8>(idx.wrapping_add(k))) << 6);
                let r: [f64; 3] = core::array::from_fn(|c| transform_x87(h, b, c as u32, p, true));
                let wk = x87::load(h.read(w.wrapping_add(4 * k)));
                for c in 0..3 {
                    let t = x87::load(x87::store(x87::mul(wk, r[c])));
                    acc[c] = if k == 0 {
                        t
                    } else {
                        x87::load(x87::store(x87::add(acc[c], t)))
                    };
                }
            }
            for (c, v) in acc.iter().enumerate() {
                h.write(pout.wrapping_add(4 * c as u32), x87::store(*v));
            }
        }

        pub fn skin4(h: &mut impl Host, bones: u32, n: u32, src: u32, dst: u32) {
            if n == 0 {
                return;
            }
            let mut pos: u32 = h.read(src);
            let mut nrm: u32 = h.read(src.wrapping_add(4));
            let mut w: u32 = h.read(src.wrapping_add(8));
            let mut idx: u32 = h.read(src.wrapping_add(0xc));
            let mut pout: u32 = h.read(dst);
            let mut nout: u32 = h.read(dst.wrapping_add(4));
            let simd = if n & 3 != 0 { n } else { n - 1 };
            for _ in 0..simd {
                let ny = scalar(h, nrm.wrapping_add(4));
                let nx = scalar(h, nrm);
                let b0 = bones.wrapping_add(u32::from(h.read::<u8>(idx)) << 6);
                let px = scalar(h, pos);
                let py = scalar(h, pos.wrapping_add(4));
                let nz = scalar(h, nrm.wrapping_add(8));
                // the normal: bone 0's rotation
                let xn = mul4(splat(nx), column(h, b0, 0));
                let yn = mul4(splat(ny), column(h, b0, 1));
                let normal = add4(add4(yn, xn), mul4(splat(nz), column(h, b0, 2)));
                put4(h, nout, normal);
                let b1 = bones.wrapping_add(u32::from(h.read::<u8>(idx.wrapping_add(1))) << 6);
                let pz = scalar(h, pos.wrapping_add(8));
                let (x, y, z) = (splat(px), splat(py), splat(pz));
                // bone 1: ((c0 x + c3) + c1 y) + c2 z, weighted
                let p1 = add4(
                    add4(
                        add4(mul4(column(h, b1, 0), x), column(h, b1, 3)),
                        mul4(column(h, b1, 1), y),
                    ),
                    mul4(column(h, b1, 2), z),
                );
                let w1 = splat(scalar(h, w.wrapping_add(4)));
                let mut acc = mul4(p1, w1);
                let b2 = bones.wrapping_add(u32::from(h.read::<u8>(idx.wrapping_add(2))) << 6);
                let b3 = bones.wrapping_add(u32::from(h.read::<u8>(idx.wrapping_add(3))) << 6);
                // bone 0: ((c3 + x c0) + y c1) + z c2, weighted
                let p0 = add4(
                    add4(
                        add4(column(h, b0, 3), mul4(x, column(h, b0, 0))),
                        mul4(y, column(h, b0, 1)),
                    ),
                    mul4(z, column(h, b0, 2)),
                );
                acc = add4(acc, mul4(p0, splat(scalar(h, w))));
                // bones 2 and 3: ((c0 x + c3) + c1 y) + c2 z, weighted
                for (b, k) in [(b2, 2u32), (b3, 3)] {
                    let c1y = mul4(column(h, b, 1), y);
                    let p = add4(
                        add4(add4(mul4(column(h, b, 0), x), column(h, b, 3)), c1y),
                        mul4(column(h, b, 2), z),
                    );
                    acc = add4(acc, mul4(p, splat(scalar(h, w.wrapping_add(4 * k)))));
                }
                put4(h, pout, acc);
                pos = pos.wrapping_add(0xc);
                nrm = nrm.wrapping_add(0xc);
                pout = pout.wrapping_add(0xc);
                nout = nout.wrapping_add(0xc);
                w = w.wrapping_add(0x10);
                idx = idx.wrapping_add(4);
            }
            if simd == n {
                return;
            }
            // the last vertex on the x87
            let p: [f64; 3] =
                core::array::from_fn(|k| x87::load(h.read(pos.wrapping_add(4 * k as u32))));
            let mut acc: [f64; 3] = [0.0; 3];
            for k in 0..4u32 {
                let b = bones.wrapping_add(u32::from(h.read::<u8>(idx.wrapping_add(k))) << 6);
                let r: [f64; 3] = core::array::from_fn(|c| transform_x87(h, b, c as u32, p, true));
                let wk = x87::load(h.read(w.wrapping_add(4 * k)));
                for c in 0..3 {
                    let t = x87::load(x87::store(x87::mul(wk, r[c])));
                    acc[c] = if k == 0 {
                        t
                    } else {
                        x87::load(x87::store(x87::add(acc[c], t)))
                    };
                }
            }
            for (c, v) in acc.iter().enumerate() {
                h.write(pout.wrapping_add(4 * c as u32), x87::store(*v));
            }
            let nv: [f64; 3] =
                core::array::from_fn(|k| x87::load(h.read(nrm.wrapping_add(4 * k as u32))));
            let b0 = bones.wrapping_add(u32::from(h.read::<u8>(idx)) << 6);
            let r: [f64; 3] = core::array::from_fn(|c| transform_x87(h, b0, c as u32, nv, false));
            for (c, v) in r.iter().enumerate() {
                h.write(nout.wrapping_add(4 * c as u32), x87::store(*v));
            }
        }
    }
}
