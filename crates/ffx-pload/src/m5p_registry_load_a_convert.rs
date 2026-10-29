//! The serialization's data-conversion kernels (M5 stage 1, list
//! `phyre-registry-load-a`): `0x004719d0`..`0x00473d60`.
//!
//! When a cluster stores a member in another primitive type than the
//! running class, the instance-list conversion (`0x0047dxxx`) looks up a
//! kernel by (destination type, source type) in the tree at `0x00c0e2f0`
//! (`0x0047b630`, which registers them all the first time through the
//! `0x0046xxxx` registrars: `GetType<dst>`, `GetType<src>`, the kernel) and
//! calls it for each member array. Every kernel is
//! `cdecl(src, dst, desc, ctx)`, no result:
//!
//! - `desc`: `+0` the element count (`u32`), `+4` the source stride, `+8`
//!   the destination stride, `+0xc` a direction flag (byte);
//! - `ctx`: `+4` a member whose `+8` is one offset, `+8` a member whose
//!   `+0x24` is the other. With the flag set the source starts at
//!   `src + [[ctx+4]+8]` and the destination at `dst + [[ctx+8]+0x24]`;
//!   without it the two offsets swap.
//!
//! Then `count` times: read one source element, convert, store, step both
//! pointers by their strides. The kernels unroll by four where they
//! convert through the x87 and keep counters in their own argument slots
//! (`[ebp+8]`, `[ebp+0x10]`, `[ebp+0x14]`): scratch that no caller reads
//! (the one caller calls through a pointer and pops its words).
//!
//! The conversions (one [`Elem`] per kernel, by address in [`KERNELS`]):
//! integers are widened to 32 bits (`movsx`/`movzx`) and their low bytes
//! stored, or sign-extended to 64 bits (`cdq`; `PUInt128` adds 8 zero
//! bytes); `bool` is `value != 0`; integers to floats go through `fild`
//! (`unsigned int` adds 2^32 when negative), so the value is exact and
//! rounds once at the store; floats to integers use `_ftol2_sse`
//! (`0x009497e0`) for the signed types and `fistp` with the rounding
//! control set to chop for the unsigned ones; `PUInt64`/`PUInt128` from
//! `float` is the compiler's unsigned conversion through `_ftol2`
//! (`0x00949816`) around 2^63.

use ffx_bridge::{Mem, x87};

/// How a kernel reads its source element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src {
    /// `char` (`movsx` byte).
    I8,
    /// `PUInt8` (`movzx` byte).
    U8,
    /// `PInt16` (`movsx` word).
    I16,
    /// `PUInt16` (`movzx` word).
    U16,
    /// `PInt32` (dword, sign-extended where widened).
    I32,
    /// `unsigned int` (dword, zero-extended where widened).
    U32,
}

/// How a kernel stores an integer source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dst {
    /// The low byte.
    B8,
    /// The low 16 bits.
    B16,
    /// The low 32 bits.
    B32,
    /// 64 bits (`PInt64`/`PUInt64`).
    B64,
    /// 64 bits and 8 zero bytes (`PUInt128`).
    B128,
}

/// Float to integer conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ftoi {
    /// `_ftol2_sse` (`0x009497e0`): [`x87::ftol`], the low `n` bytes of `eax`.
    Ftol(u8),
    /// `fistp dword` with chop: toward zero, `0x80000000` outside `i32`
    /// and for NaN; the low `n` bytes.
    Chop32(u8),
    /// `fistp qword` with chop, its low dword: [`x87::ftol2`].
    Chop64Low,
    /// `_ftol2` (`0x00949816`): 64 bits.
    Ftol64,
    /// The unsigned 64-bit conversion around 2^63 (`0x00473d60`); 8 zero
    /// bytes more when `true` (`PUInt128`, `0x00473c40`).
    Unsigned64(bool),
}

/// One kernel's element conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elem {
    /// An integer widened by its type, stored at a width.
    Int(Src, Dst),
    /// `bool`: one byte, `value != 0` (the source read at its width).
    NonZero(Src),
    /// To `float` (`fild`, `fstp dword`).
    ToF32(Src),
    /// To `double` (`fild`, `fstp qword`).
    ToF64(Src),
    /// `float` to `float` (`fld`, `fstp dword`).
    F32ToF32,
    /// `float` to `double`.
    F32ToF64,
    /// `float` to an integer.
    FromF32(Ftoi),
}

use Dst::{B8, B16, B32, B64, B128};
use Elem::{F32ToF32, F32ToF64, FromF32, Int, NonZero, ToF32, ToF64};
use Ftoi::{Chop32, Chop64Low, Ftol, Ftol64, Unsigned64};
use Src::{I8, I16, I32, U8, U16, U32};

/// Every kernel by address: the destination and source types (named by
/// their `GetType<T>` in the registrar) and its conversion.
pub const KERNELS: &[(u32, &str, &str, Elem)] = &[
    (0x0047_19d0, "PChar", "PChar", Int(U8, B8)),
    (0x0047_1a20, "PUInt8", "PChar", Int(U8, B8)),
    (0x0047_1a70, "PInt16", "PChar", Int(I8, B16)),
    (0x0047_1ad0, "PUInt16", "PChar", Int(I8, B16)),
    (0x0047_1b30, "PInt32", "PChar", Int(I8, B32)),
    (0x0047_1b90, "PUInt32", "PChar", Int(I8, B32)),
    (0x0047_1bf0, "Float", "PChar", ToF32(I8)),
    (0x0047_1cb0, "Double", "PChar", ToF64(I8)),
    (0x0047_1d70, "PUInt128", "PChar", Int(I8, B128)),
    (0x0047_1de0, "PInt64", "PChar", Int(I8, B64)),
    (0x0047_1e40, "PUInt64", "PChar", Int(I8, B64)),
    (0x0047_1ea0, "Bool", "PChar", NonZero(U8)),
    (0x0047_1f00, "PChar", "PUInt8", Int(U8, B8)),
    (0x0047_1f50, "PUInt8", "PUInt8", Int(U8, B8)),
    (0x0047_1fa0, "PInt16", "PUInt8", Int(U8, B16)),
    (0x0047_2000, "PUInt16", "PUInt8", Int(U8, B16)),
    (0x0047_2060, "PInt32", "PUInt8", Int(U8, B32)),
    (0x0047_20c0, "PUInt32", "PUInt8", Int(U8, B32)),
    (0x0047_2120, "Float", "PUInt8", ToF32(U8)),
    (0x0047_21e0, "Double", "PUInt8", ToF64(U8)),
    (0x0047_22a0, "PUInt128", "PUInt8", Int(U8, B128)),
    (0x0047_2310, "PInt64", "PUInt8", Int(U8, B64)),
    (0x0047_2370, "PUInt64", "PUInt8", Int(U8, B64)),
    (0x0047_23d0, "Bool", "PUInt8", NonZero(U8)),
    (0x0047_2430, "PChar", "PInt16", Int(U8, B8)),
    (0x0047_2480, "PUInt8", "PInt16", Int(U8, B8)),
    (0x0047_24d0, "PInt16", "PInt16", Int(U16, B16)),
    (0x0047_2530, "PUInt16", "PInt16", Int(U16, B16)),
    (0x0047_2590, "PInt32", "PInt16", Int(I16, B32)),
    (0x0047_25f0, "PUInt32", "PInt16", Int(I16, B32)),
    (0x0047_2650, "Float", "PInt16", ToF32(I16)),
    (0x0047_2710, "Double", "PInt16", ToF64(I16)),
    (0x0047_27d0, "PUInt128", "PInt16", Int(I16, B128)),
    (0x0047_2840, "PInt64", "PInt16", Int(I16, B64)),
    (0x0047_28a0, "PUInt64", "PInt16", Int(I16, B64)),
    (0x0047_2900, "Bool", "PInt16", NonZero(U16)),
    (0x0047_2960, "PChar", "PUInt16", Int(U8, B8)),
    (0x0047_29b0, "PUInt8", "PUInt16", Int(U8, B8)),
    (0x0047_2a00, "PInt16", "PUInt16", Int(U16, B16)),
    (0x0047_2a60, "PUInt16", "PUInt16", Int(U16, B16)),
    (0x0047_2ac0, "PInt32", "PUInt16", Int(U16, B32)),
    (0x0047_2b20, "PUInt32", "PUInt16", Int(U16, B32)),
    (0x0047_2b80, "Float", "PUInt16", ToF32(U16)),
    (0x0047_2c40, "Double", "PUInt16", ToF64(U16)),
    (0x0047_2d00, "PUInt128", "PUInt16", Int(U16, B128)),
    (0x0047_2d70, "PInt64", "PUInt16", Int(U16, B64)),
    (0x0047_2dd0, "PUInt64", "PUInt16", Int(U16, B64)),
    (0x0047_2e30, "Bool", "PUInt16", NonZero(U16)),
    (0x0047_2e90, "PChar", "PInt32", Int(U8, B8)),
    (0x0047_2ee0, "PUInt8", "PInt32", Int(U8, B8)),
    (0x0047_2f30, "PInt16", "PInt32", Int(U16, B16)),
    (0x0047_2f90, "PUInt16", "PInt32", Int(U16, B16)),
    (0x0047_2ff0, "PInt32", "PInt32", Int(U32, B32)),
    (0x0047_3040, "PUInt32", "PInt32", Int(U32, B32)),
    (0x0047_3090, "Float", "PInt32", ToF32(I32)),
    (0x0047_3130, "Double", "PInt32", ToF64(I32)),
    (0x0047_31d0, "PUInt128", "PInt32", Int(I32, B128)),
    (0x0047_3240, "PInt64", "PInt32", Int(I32, B64)),
    (0x0047_32a0, "PUInt64", "PInt32", Int(I32, B64)),
    (0x0047_3300, "Bool", "PInt32", NonZero(U32)),
    (0x0047_3360, "PChar", "PUInt32", Int(U8, B8)),
    (0x0047_33b0, "PUInt8", "PUInt32", Int(U8, B8)),
    (0x0047_3400, "PInt16", "PUInt32", Int(U16, B16)),
    (0x0047_3460, "PUInt16", "PUInt32", Int(U16, B16)),
    (0x0047_34c0, "PInt32", "PUInt32", Int(U32, B32)),
    (0x0047_3510, "PUInt32", "PUInt32", Int(U32, B32)),
    (0x0047_3560, "Float", "PUInt32", ToF32(U32)),
    (0x0047_3630, "Double", "PUInt32", ToF64(U32)),
    (0x0047_3700, "PUInt128", "PUInt32", Int(U32, B128)),
    (0x0047_3770, "PInt64", "PUInt32", Int(U32, B64)),
    (0x0047_37d0, "PUInt64", "PUInt32", Int(U32, B64)),
    (0x0047_3830, "Bool", "PUInt32", NonZero(U32)),
    (0x0047_3890, "PChar", "Float", FromF32(Ftol(1))),
    (0x0047_38f0, "PUInt8", "Float", FromF32(Chop32(1))),
    (0x0047_3960, "PInt16", "Float", FromF32(Ftol(2))),
    (0x0047_39c0, "PUInt16", "Float", FromF32(Chop32(2))),
    (0x0047_3a30, "PInt32", "Float", FromF32(Ftol(4))),
    (0x0047_3a90, "PUInt32", "Float", FromF32(Chop64Low)),
    (0x0047_3b00, "Float", "Float", F32ToF32),
    (0x0047_3ba0, "Double", "Float", F32ToF64),
    (0x0047_3c40, "PUInt128", "Float", FromF32(Unsigned64(true))),
    (0x0047_3d00, "PInt64", "Float", FromF32(Ftol64)),
    (0x0047_3d60, "PUInt64", "Float", FromF32(Unsigned64(false))),
];

/// The kernels whose last instruction to write `eax` steps the source
/// pointer (`add eax, esi` in the loop, `mov eax, [ebp+8]; add eax, ecx`
/// before it): they return where the source ends.
pub const EAX_SOURCE_END: [u32; 4] = [0x0047_3090, 0x0047_3130, 0x0047_3b00, 0x0047_3ba0];
/// The kernels that keep the destination pointer in `eax` the same way.
pub const EAX_DEST_END: [u32; 3] = [0x0047_3700, 0x0047_3770, 0x0047_37d0];

/// The kernel at `addr`.
#[must_use]
pub fn kernel(addr: u32) -> Option<Elem> {
    KERNELS.iter().find(|k| k.0 == addr).map(|k| k.3)
}

/// Pure parts.
pub mod core {
    /// 2^63 as the kernels' `float` constant (`0x00b13be0`).
    const TWO_63: f64 = 9_223_372_036_854_775_808.0;

    /// `_ftol2` (`0x00949816`): `fistp qword` at round-to-nearest, then a
    /// correction toward zero; NaN and anything outside `i64` give the
    /// integer indefinite `0x8000000000000000`.
    #[must_use]
    pub fn ftol2_64(x: f64) -> u64 {
        if x.is_nan() || x >= TWO_63 || x < -TWO_63 {
            0x8000_0000_0000_0000
        } else {
            #[allow(clippy::cast_possible_truncation)] // in range, checked
            let v = x.trunc() as i64;
            v.cast_unsigned()
        }
    }

    /// The compiler's `float` to `unsigned __int64` (`0x00473d60`): below
    /// 2^63 (or NaN) `_ftol2`; else `_ftol2(x - 2^63)` with bit 63 added
    /// when the difference is below 2^63 (rounded to `float` first, exact
    /// there); otherwise (2^64 and up, infinity) `_ftol2` of the
    /// difference alone.
    #[must_use]
    pub fn f32_to_u64(x: f64) -> u64 {
        if x.is_nan() || x < TWO_63 {
            return ftol2_64(x);
        }
        let d = ffx_bridge::x87::load(ffx_bridge::x87::store(x - TWO_63));
        if d.is_nan() || d >= TWO_63 {
            ftol2_64(d)
        } else {
            ftol2_64(d).wrapping_add(0x8000_0000_0000_0000)
        }
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{B8, B16, B32, B64, B128, Elem, Ftoi, Mem, Src, core, x87};

    /// A source element widened to 64 bits the way the kernels widen it.
    fn read_int(m: &impl Mem, src: Src, at: u32) -> i64 {
        match src {
            Src::I8 => i64::from(m.read::<i8>(at)),
            Src::U8 => i64::from(m.read::<u8>(at)),
            Src::I16 => i64::from(m.read::<i16>(at)),
            Src::U16 => i64::from(m.read::<u16>(at)),
            Src::I32 => i64::from(m.read::<i32>(at)),
            Src::U32 => i64::from(m.read::<u32>(at)),
        }
    }

    /// Stores the low `n` bytes of `v`.
    fn store_low(m: &mut impl Mem, at: u32, v: u64, n: usize) {
        m.write_bytes(at, &v.to_le_bytes()[..n]);
    }

    /// One element.
    fn convert_one(m: &mut impl Mem, elem: Elem, s: u32, d: u32) {
        match elem {
            Elem::Int(src, dst) => {
                let v = read_int(m, src, s).cast_unsigned();
                match dst {
                    B8 => store_low(m, d, v, 1),
                    B16 => store_low(m, d, v, 2),
                    B32 => store_low(m, d, v, 4),
                    B64 => store_low(m, d, v, 8),
                    B128 => {
                        store_low(m, d, v, 8);
                        m.write(d + 8, 0u64);
                    }
                }
            }
            Elem::NonZero(src) => {
                let v = read_int(m, src, s);
                m.write(d, u8::from(v != 0));
            }
            Elem::ToF32(src) => {
                // exact in f64 (at most 32 bits), one rounding at the store
                #[allow(clippy::cast_precision_loss)]
                let x = read_int(m, src, s) as f64;
                m.write(d, x87::store(x));
            }
            Elem::ToF64(src) => {
                #[allow(clippy::cast_precision_loss)]
                let x = read_int(m, src, s) as f64;
                m.write(d, x);
            }
            Elem::F32ToF32 => {
                let x = x87::load(m.read(s));
                m.write(d, x87::store(x));
            }
            Elem::F32ToF64 => {
                let x = x87::load(m.read(s));
                m.write(d, x.to_bits());
            }
            Elem::FromF32(how) => {
                let x = x87::load(m.read(s));
                match how {
                    Ftoi::Ftol(n) => {
                        let v = x87::ftol(m, x);
                        store_low(m, d, u64::from(v), usize::from(n));
                    }
                    Ftoi::Chop32(n) => {
                        store_low(m, d, u64::from(x87::cvttsd2si(x)), usize::from(n));
                    }
                    Ftoi::Chop64Low => m.write(d, x87::ftol2(x)),
                    Ftoi::Ftol64 => m.write(d, core::ftol2_64(x)),
                    Ftoi::Unsigned64(wide) => {
                        m.write(d, core::f32_to_u64(x));
                        if wide {
                            m.write(d + 8, 0u64);
                        }
                    }
                }
            }
        }
    }

    /// A kernel `cdecl(src, dst, desc, ctx)` (module docs): `count`
    /// elements converted by `elem`. Returns where the source and the
    /// destination pointers end (past the last element): some kernels
    /// leave one of them in `eax` ([`super::EAX_SOURCE_END`],
    /// [`super::EAX_DEST_END`]).
    pub fn convert(
        m: &mut impl Mem,
        elem: Elem,
        src: u32,
        dst: u32,
        desc: u32,
        ctx: u32,
    ) -> (u32, u32) {
        let a: u32 = m.read(ctx + 4);
        let a: u32 = m.read(a + 8);
        let b: u32 = m.read(ctx + 8);
        let b: u32 = m.read(b + 0x24);
        let forward: u8 = m.read(desc + 0xc);
        let (so, dof) = if forward != 0 { (a, b) } else { (b, a) };
        let count: u32 = m.read(desc);
        let ss: u32 = m.read(desc + 4);
        let ds: u32 = m.read(desc + 8);
        let mut s = src.wrapping_add(so);
        let mut d = dst.wrapping_add(dof);
        for _ in 0..count {
            convert_one(m, elem, s, d);
            s = s.wrapping_add(ss);
            d = d.wrapping_add(ds);
        }
        (s, d)
    }
}
