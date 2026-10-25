//! `PhyreEngine`'s class-descriptor callbacks (`PClassDescriptorForType<T>`,
//! `...Abstract<T>`, `...Concrete<T>` vtables): the per-type construct,
//! destroy and fix-up slots the serializer and the asset loader call
//! through the descriptor's vtable (M4 port round 1, list p6-render-c).
//!
//! The compiler stamped one copy per type, so each shape below stands for
//! many originals (their addresses are the tables in [`family`]). All are
//! `thiscall(descriptor, instance)` with `ret 4`; none reads the
//! descriptor. The originals return `al = 1` (`mov al, 1` over whatever
//! `eax` held: a callee's result, the instance pointer) or `eax = 0`;
//! their callers (`0x0043c55c`, `0x0043c577`, `0x0043cf1b`, the
//! serializer's `call [eax+0x2c]`) ignore the result, but the bridges
//! return the whole of `eax` as the original leaves it.
//!
//! A `PArray<T>` is `{count, data}`: bit 31 of `count` marks a buffer the
//! array does not own (a view into a loaded cluster), which is never
//! freed.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreFree(ptr)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// `al = 1` over `eax`'s upper bytes, as `mov al, 1` leaves it.
#[must_use]
pub const fn al_one(eax: u32) -> u32 {
    (eax & !0xff) | 1
}

pub mod bridge {
    //! The originals over game memory.

    use super::{Conv, Host, PHYRE_FREE, al_one};

    /// `destroyInstance(obj)` of a polymorphic type (38 bytes, 52 copies):
    /// the object starts 4 bytes before `obj` (its vtable; 0 for a null
    /// `obj`, which faults on the vtable read as the original does), and
    /// gets its virtual destructor (slot 0) with flags 0, no free.
    pub fn destroy_virtual(h: &mut impl Host, obj: u32) -> u32 {
        let this = if obj == 0 { 0 } else { obj.wrapping_sub(4) };
        al_one(h.call_virtual(this, 0, &[0]))
    }

    /// `destroyInstance(obj)` of a type with a plain destructor (17 bytes):
    /// `dtor(obj)` (thiscall, no stack words).
    pub fn destroy_with(h: &mut impl Host, dtor: u32, obj: u32) -> u32 {
        al_one(h.call_sym(dtor, Conv::Thiscall(obj), &[]))
    }

    /// `destroyInstance(arr)` of a `PArray` (49 and 54 bytes) at `arr + at`:
    /// an owned buffer (`count` bit 31 clear) goes to `PhyreFree`, a null
    /// one too unless `skip_null` (the 54-byte copies test it); then
    /// `data` and `count` are zeroed, in that order.
    pub fn array_free(h: &mut impl Host, arr: u32, at: u32, skip_null: bool) -> u32 {
        let a = arr.wrapping_add(at);
        let count: u32 = h.read(a);
        let mut eax = count >> 31;
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(a.wrapping_add(4));
            if skip_null && data == 0 {
                eax = 0;
            } else {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(a.wrapping_add(4), 0u32);
        h.write(a, 0u32);
        al_one(eax)
    }

    /// `destroyInstance(arr)` of a `PArray` whose elements need their own
    /// destroy (58 bytes): `free(data, count & 0x7fffffff)` (cdecl) for an
    /// owned buffer, then `data` and `count` zeroed.
    pub fn array_free_with(h: &mut impl Host, free: u32, arr: u32) -> u32 {
        let count: u32 = h.read(arr);
        let mut eax = count >> 31;
        if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(arr.wrapping_add(4));
            eax = h.call_sym(free, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
        }
        h.write(arr.wrapping_add(4), 0u32);
        h.write(arr, 0u32);
        al_one(eax)
    }

    /// `constructInstance(arr)` of a `PArray` (or a type that starts with
    /// two zero words; 29 bytes): both words zeroed when `arr` is set.
    pub fn array_init(h: &mut impl Host, arr: u32) -> u32 {
        if arr != 0 {
            h.write(arr, 0u32);
            h.write(arr.wrapping_add(4), 0u32);
        }
        al_one(arr)
    }
}

/// The originals of each shape, by address (with the callee the shape
/// takes, where it takes one). The shims in `ffx-hook` hold one entry
/// each; `tests/p6_render_c.rs` runs every address here against its
/// bridge and checks the shims name the same bridge.
pub mod family {
    /// [`super::bridge::destroy_virtual`].
    pub const DESTROY_VIRTUAL: &[u32] = &[
        0x005a_e0b0,
        0x0062_7060,
        0x0067_2b50,
        0x009e_c0f0,
        0x009e_c170,
        0x009e_c1a0,
        0x009e_c1d0,
        0x009e_c200,
        0x009e_c230,
        0x009e_c260,
        0x009e_c290,
        0x009e_c2c0,
        0x009e_c2f0,
        0x009e_c320,
        0x009e_c350,
        0x009e_c380,
        0x009e_c3b0,
        0x009e_c3e0,
        0x009e_c410,
        0x009e_c440,
        0x009e_c470,
        0x009e_c4a0,
        0x009e_c4d0,
        0x009e_c500,
        0x009e_c530,
        0x009e_c560,
        0x00a1_0720,
        0x00a1_0750,
        0x00a1_0780,
        0x00a1_07b0,
        0x00a1_07e0,
        0x00a1_0810,
        0x00a1_0840,
        0x00a1_0870,
        0x00a1_08a0,
        0x00a1_08d0,
        0x00a1_0900,
        0x00a1_0930,
        0x00a1_0960,
        0x00a1_0b10,
        0x00a1_0b40,
        0x00a1_0b70,
        0x00a1_0bd0,
        0x00a1_0c00,
        0x00a1_0c30,
        0x00a1_0c60,
        0x00a1_0cb0,
        0x00a1_0ce0,
        0x00a1_0d10,
        0x00a1_0d40,
        0x00a1_0d70,
        0x00a1_0da0,
    ];
    /// `mov al, 1; ret 4`.
    pub const TRUE1: &[u32] = &[
        0x005a_e0e0,
        0x005b_2a20,
        0x005b_2a90,
        0x005b_2aa0,
        0x005c_5bf0,
        0x005c_5c50,
        0x005c_5ca0,
        0x005c_5de0,
        0x005c_5e00,
        0x005c_5e10,
        0x005c_5e20,
        0x005c_5e30,
        0x005c_5e40,
        0x0062_7090,
    ];
    /// `xor eax, eax; ret 4`.
    pub const ZERO1: &[u32] = &[
        0x005a_e2d0,
        0x005a_e2e0,
        0x005b_2ac0,
        0x005c_5ee0,
        0x005c_5ef0,
        0x005c_5f00,
        0x005c_5f10,
        0x005c_5f50,
        0x005c_5f60,
    ];
    /// [`super::bridge::destroy_with`]: (original, destructor).
    pub const DESTROY_WITH: &[(u32, u32)] = &[
        (0x005b_2a70, 0x005b_0e40),
        (0x005c_5dc0, 0x005b_f7d0),
        (0x009e_c120, 0x009e_2420),
        (0x00a1_0c90, 0x009f_e790),
    ];
    /// [`super::bridge::array_free`] at 0, null buffers freed too.
    pub const ARRAY_FREE: &[u32] = &[
        0x005a_dff0,
        0x005b_2a30,
        0x009e_c070,
        0x009e_c0b0,
        0x00a1_06e0,
    ];
    /// [`super::bridge::array_free`] at 0, null buffers skipped.
    pub const ARRAY_FREE_NN: &[u32] = &[0x005a_e030, 0x005a_e070, 0x005c_5d00, 0x005c_5d40];
    /// [`super::bridge::array_free`] at `+0xc`, null buffers skipped.
    pub const ARRAY_FREE_NN_C: &[u32] = &[0x005c_5d80];
    /// [`super::bridge::array_free_with`]: (original, the elements' free).
    pub const ARRAY_FREE_WITH: &[(u32, u32)] = &[(0x005c_5cc0, 0x005c_1ef0)];
    /// [`super::bridge::array_init`].
    pub const ARRAY_INIT: &[u32] = &[
        0x005a_da10,
        0x005a_da30,
        0x005a_da50,
        0x005b_2990,
        0x005c_5af0,
        0x005c_5b10,
        0x005c_5b30,
        0x005c_5c60,
        0x009e_afd0,
        0x009e_aff0,
        0x009e_b040,
        0x00a0_f020,
    ];
}
