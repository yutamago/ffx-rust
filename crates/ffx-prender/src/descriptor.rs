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

    /// `constructInstance(obj)` of a plain type (43 and 50 bytes): its
    /// first `words` words zeroed when `obj` is set.
    pub fn construct_zeroed(h: &mut impl Host, obj: u32, words: u32) -> u32 {
        if obj != 0 {
            for i in 0..words {
                h.write(obj.wrapping_add(4 * i), 0u32);
            }
        }
        al_one(obj)
    }

    /// `constructInstance(obj)` of a type starting with two unset
    /// handles (29 bytes): `+0` and `+4` = -1 when `obj` is set.
    pub fn construct_unset_pair(h: &mut impl Host, obj: u32) -> u32 {
        if obj != 0 {
            h.write(obj, u32::MAX);
            h.write(obj.wrapping_add(4), u32::MAX);
        }
        al_one(obj)
    }

    /// `constructInstance(obj)` of `PModifierNetworkInstanceInput` (43
    /// bytes): `+0`, `+4` = -1, `+0xc`, `+0x10` = 0 (`+8` stays) when `obj`
    /// is set.
    pub fn construct_unset_pair_zeroed(h: &mut impl Host, obj: u32) -> u32 {
        if obj != 0 {
            h.write(obj, u32::MAX);
            h.write(obj.wrapping_add(4), u32::MAX);
            h.write(obj.wrapping_add(0xc), 0u32);
            h.write(obj.wrapping_add(0x10), 0u32);
        }
        al_one(obj)
    }

    /// `constructInstance(obj)` of a polymorphic type with an inline
    /// constructor (43 and 49 bytes): the vtable (an image address), three
    /// zero words, and the `u16` at `+0x10` when given, if `obj` is set.
    pub fn construct_vtable_zeroed(
        h: &mut impl Host,
        obj: u32,
        vtable: u32,
        half: Option<u16>,
    ) -> u32 {
        if obj != 0 {
            let vtable = h.rebase(vtable);
            h.write(obj, vtable);
            for i in 1..4 {
                h.write(obj.wrapping_add(4 * i), 0u32);
            }
            if let Some(v) = half {
                h.write(obj.wrapping_add(0x10), v);
            }
        }
        al_one(obj)
    }

    /// `constructInstance(obj)` of the motion and touch input sources (52
    /// bytes, 16 copies): `fields` are the image addresses stored at `+4`,
    /// `+8` and `+0` (the vtable); `+0xc` = 0, the byte at `+0x10` = 0,
    /// `+0x14` = 1.0 (`fld1; fstp dword`), when `obj` is set.
    pub fn construct_input(h: &mut impl Host, obj: u32, fields: [u32; 3]) -> u32 {
        if obj != 0 {
            let [a, b, vtable] = fields.map(|f| h.rebase(f));
            h.write(obj.wrapping_add(4), a);
            h.write(obj.wrapping_add(0x14), 1.0f32);
            h.write(obj.wrapping_add(8), b);
            h.write(obj.wrapping_add(0xc), 0u32);
            h.write(obj.wrapping_add(0x10), 0u8);
            h.write(obj, vtable);
        }
        al_one(obj)
    }

    /// `constructInstance(obj)` through a constructor (21, 76, 86 and 93
    /// bytes): `ctor(obj)` (thiscall, no stack words; the larger copies
    /// open an SEH frame around it, which nothing here throws through),
    /// then the derived class's vtable at `+0` and a word at `+4` where
    /// given (image addresses), when `obj` is set. Returns `al = 1`: the
    /// rest of `eax` is the constructor's, or for a null `obj` the
    /// caller's (21 bytes) or a stack address (the SEH copies), which no
    /// caller reads.
    pub fn construct_with(
        h: &mut impl Host,
        obj: u32,
        ctor: u32,
        vtable: Option<u32>,
        word4: Option<u32>,
    ) -> u8 {
        if obj != 0 {
            h.call_sym(ctor, Conv::Thiscall(obj), &[]);
            if let Some(v) = vtable {
                let v = h.rebase(v);
                h.write(obj, v);
            }
            if let Some(w) = word4 {
                let w = h.rebase(w);
                h.write(obj.wrapping_add(4), w);
            }
        }
        1
    }

    /// A scalar deleting destructor, vtable slot 0 (33, 34 and 39 bytes):
    /// the class's vtable at `this` where given, then the base destructor
    /// `dtor(this)` (thiscall) where given, then `PhyreFree(this)` when
    /// bit 0 of `flags` (its low byte is tested) is set. Returns `this`.
    pub fn deleting_dtor(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        vtable: Option<u32>,
        dtor: Option<u32>,
    ) -> u32 {
        if let Some(v) = vtable {
            let v = h.rebase(v);
            h.write(this, v);
        }
        if let Some(d) = dtor {
            h.call_sym(d, Conv::Thiscall(this), &[]);
        }
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PClassDataMemberArrayDynamic<T>` slot `+0x20` (34 bytes, 7
    /// copies): the member's callback at `+0x3c` as `callback(obj, a, b)`
    /// (thiscall), or 0x15 without one.
    pub fn member_call(h: &mut impl Host, member: u32, obj: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(member.wrapping_add(0x3c));
        if f == 0 {
            return 0x15;
        }
        h.call(f, Conv::Thiscall(obj), &[a, b])
    }

    /// `PClassDataMemberArrayDynamic<T>` slot `+0x18` (43 bytes, 2
    /// copies): the member's callback at `+0x38` as `callback(obj)` (`ecx`
    /// = `obj`, `edx` = `member`, no stack words), or without one the word
    /// at `obj + member[+0x24] - member[+0xc][+0x88]` (the array's count
    /// in the instance).
    pub fn member_get(h: &mut impl Host, member: u32, obj: u32) -> u32 {
        let f: u32 = h.read(member.wrapping_add(0x38));
        if f != 0 {
            return h.call(f, Conv::Fastcall, &[obj, member]);
        }
        let class: u32 = h.read(member.wrapping_add(0xc));
        let offset: u32 = h.read(member.wrapping_add(0x24));
        let base: u32 = h.read(class.wrapping_add(0x88));
        h.read(offset.wrapping_sub(base).wrapping_add(obj))
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
    /// [`super::bridge::construct_zeroed`]: (original, words).
    pub const CONSTRUCT_ZEROED: &[(u32, u32)] = &[
        (0x005a_db00, 4),
        (0x00a0_f9b0, 4),
        (0x005b_29b0, 5),
        (0x005c_5b50, 5),
        (0x005c_5bb0, 5),
    ];
    /// [`super::bridge::construct_unset_pair`].
    pub const CONSTRUCT_UNSET_PAIR: &[u32] = &[0x005c_5c80];
    /// [`super::bridge::construct_unset_pair_zeroed`].
    pub const CONSTRUCT_UNSET_PAIR_ZEROED: &[u32] = &[0x005b_29f0];
    /// [`super::bridge::construct_vtable_zeroed`]: (original, vtable, the `u16` at `+0x10`).
    pub const CONSTRUCT_VTABLE_ZEROED: &[(u32, u32, Option<u16>)] = &[
        (0x009e_b010, 0x00b7_2080, None),
        (0x00a0_f970, 0x00b7_4058, Some(0x100)),
    ];
    /// [`super::bridge::construct_input`]: (original, [`+4`, `+8`, vtable]).
    pub const CONSTRUCT_INPUT: &[(u32, [u32; 3])] = &[
        (0x009e_b1f0, [0x0194_14e8, 0x00c6_82e8, 0x00b7_1a38]),
        (0x009e_b230, [0x0194_1580, 0x00c6_82f4, 0x00b7_1b40]),
        (0x009e_b270, [0x0194_1618, 0x00c6_8300, 0x00b7_1c48]),
        (0x009e_b2b0, [0x0194_16b0, 0x00c6_830c, 0x00b7_1d50]),
        (0x009e_b2f0, [0x0194_1748, 0x00c6_8318, 0x00b7_1e60]),
        (0x009e_b330, [0x0194_17e0, 0x00c6_8324, 0x00b7_1f70]),
        (0x009e_b370, [0x0194_1450, 0x00c6_82dc, 0x00b7_1938]),
        (0x009e_b3b0, [0x0194_1288, 0x00c6_82b8, 0x00b7_1638]),
        (0x009e_b3f0, [0x0194_1320, 0x00c6_82c4, 0x00b7_1738]),
        (0x009e_b430, [0x0194_13b8, 0x00c6_82d0, 0x00b7_1838]),
        (0x009e_b620, [0x0194_1028, 0x00c6_8270, 0x00b7_1438]),
        (0x009e_b660, [0x0194_10c0, 0x00c6_827c, 0x00b7_1538]),
        (0x009e_b6a0, [0x0194_0f90, 0x00c6_82a0, 0x00b7_1128]),
        (0x009e_b6e0, [0x0194_0ef8, 0x00c6_82ac, 0x00b7_1028]),
        (0x009e_b720, [0x0194_1158, 0x00c6_8288, 0x00b7_1228]),
        (0x009e_b760, [0x0194_11f0, 0x00c6_8294, 0x00b7_1330]),
    ];
    /// [`super::bridge::construct_with`]: (original, constructor, vtable, the word at `+4`).
    pub const CONSTRUCT_WITH: &[(u32, u32, Option<u32>, Option<u32>)] = &[
        (0x005c_5b90, 0x005b_f340, None, None),
        (0x0062_6c20, 0x0062_0590, None, None),
        (0x00a0_f0a0, 0x009f_b830, None, None),
        (0x00a0_f0f0, 0x009f_bb30, None, None),
        (0x00a0_f1d0, 0x009f_bd90, None, None),
        (0x00a0_f300, 0x009f_c160, None, None),
        (0x00a0_f3b0, 0x009f_c2b0, None, None),
        (0x00a0_f400, 0x009f_c310, None, None),
        (0x00a0_f450, 0x009f_c480, None, None),
        (0x00a0_f4a0, 0x009f_c4e0, None, None),
        (0x00a0_f750, 0x009f_c7e0, None, None),
        (0x00a0_f8c0, 0x009f_cbb0, None, None),
        (0x00a0_fb60, 0x009f_d080, None, None),
        (0x0067_2ad0, 0x0049_68c0, Some(0x00b4_3ed8), None),
        (0x00a0_f350, 0x009f_c160, Some(0x00b7_5b88), None),
        (0x00a0_f7a0, 0x009f_c7e0, Some(0x00b7_4fb8), None),
        (0x00a0_fbb0, 0x009f_d080, Some(0x00b7_5e98), None),
        (
            0x00a0_f040,
            0x009f_bb30,
            Some(0x00b7_4380),
            Some(0x0194_3718),
        ),
        (
            0x00a0_f2a0,
            0x009f_c160,
            Some(0x00b7_5c88),
            Some(0x0194_45a8),
        ),
        (
            0x00a0_f6f0,
            0x009f_c7e0,
            Some(0x00b7_50b8),
            Some(0x0194_3e88),
        ),
        (
            0x00a0_fb00,
            0x009f_d080,
            Some(0x00b7_5fa8),
            Some(0x0194_4770),
        ),
    ];
    /// [`super::bridge::deleting_dtor`]: (original, vtable, base destructor).
    pub const DELETING_DTOR: &[(u32, Option<u32>, Option<u32>)] = &[
        (0x005a_ade0, None, Some(0x005a_a4b0)),
        (0x00a0_26a0, None, Some(0x009f_dc00)),
        (0x00a0_2770, None, Some(0x009f_dc00)),
        (0x00a0_2820, None, Some(0x009f_dd30)),
        (0x00a0_29f0, None, Some(0x009f_df60)),
        (0x00a0_2a20, None, Some(0x009f_df60)),
        (0x00a0_2a50, None, Some(0x009f_df60)),
        (0x00a0_2a80, None, Some(0x009f_e0d0)),
        (0x00a0_2ab0, None, Some(0x009f_e020)),
        (0x00a0_2ae0, None, Some(0x009f_e0d0)),
        (0x00a0_2b10, None, Some(0x009f_e0d0)),
        (0x00a0_2d00, None, Some(0x009f_e2a0)),
        (0x00a0_2d30, None, Some(0x009f_e2a0)),
        (0x00a0_2d60, None, Some(0x009f_e2a0)),
        (0x00a0_2f10, None, Some(0x009f_e640)),
        (0x00a0_30b0, None, Some(0x009f_e890)),
        (0x00a0_30e0, None, Some(0x009f_e890)),
        (0x00a0_3110, None, Some(0x009f_e890)),
        (0x0062_1ba0, Some(0x00b3_ff50), None),
        (0x00a0_2fc0, Some(0x00b7_4058), None),
        (0x00a0_3020, Some(0x00b7_4058), None),
        (0x00a0_3050, Some(0x00b7_4058), None),
        (0x00a0_3080, Some(0x00b7_4058), None),
        (0x0067_2760, Some(0x00b4_3ed8), Some(0x0049_69f0)),
    ];
    /// [`super::bridge::member_call`].
    pub const MEMBER_CALL: &[u32] = &[
        0x005a_f9c0,
        0x005a_f9f0,
        0x005a_fa20,
        0x005b_3410,
        0x005c_7ab0,
        0x005c_7ae0,
        0x005c_7b10,
    ];
    /// [`super::bridge::member_get`].
    pub const MEMBER_GET: &[u32] = &[0x005a_e3a0, 0x005b_2b40];
}
