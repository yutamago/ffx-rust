//! p6-render-c, continuation round, batch 8: more copies of shapes other
//! lists ported (the `PArray<T>` descriptors' constructors and
//! declarations of [`crate::declare`], the occluder cursor's `skipEmpty`),
//! the input sources' semantic checks and constructors, and the render
//! delegation helper's per-flag runs. Callees stay original and are
//! called at their addresses through [`Host`].

use ffx_graphic::host::{Conv, Host};

use crate::declare::{Abstract, Declare, Element, Name};

/// The copies of `PClassDescriptorAbstract<PArray<T>>`'s constructor
/// ([`crate::declare::bridge::abstract_ctor`]): (original, constants).
pub const ABSTRACT: &[(u32, Abstract)] = &[
    (0x005a_9db0, abs(0x00cb_b974, 0x00b3_57b4, 0x00cb_b9e0)),
    (0x005a_9e50, abs(0x00cb_b96c, 0x00b3_55a4, 0x00cb_b980)),
    (0x005a_9ef0, abs(0x00cb_b970, 0x00b3_56ac, 0x00cb_b9b0)),
    (0x005b_0910, abs(0x00cb_bf24, 0x00b3_622c, 0x00cb_bf30)),
    (0x005b_e710, abs(0x00cb_d258, 0x00b3_7e7c, 0x00cb_d320)),
    (0x005b_e7b0, abs(0x00cb_d254, 0x00b3_7d74, 0x00cb_d2f0)),
    (0x005b_e850, abs(0x00cb_d250, 0x00b3_7c6c, 0x00cb_d260)),
    (0x009f_96d0, abs(0x0198_4c20, 0x00b7_6d5c, 0x0198_4c40)),
    (0x00a2_b780, abs(0x01a8_52e4, 0x00b7_9224, 0x01a8_52f0)),
];

/// Every copy here names its class through a global, records 8 bytes and
/// keeps its three buffers 16 bytes apart.
const fn abs(name: u32, vtable: u32, default_buffer: u32) -> Abstract {
    Abstract {
        name: Name::Global(name),
        size: 8,
        vtable,
        default_buffer,
        validation_buffer: default_buffer + 0x10,
        write_mask_buffer: default_buffer + 0x20,
    }
}

/// The copies of the `PArray<T>` descriptor's `DeclareClassDescriptor`
/// ([`crate::declare::bridge::declare`]): (original, constants).
pub const DECLARE: &[(u32, Declare)] = &[
    (
        0x005a_b580,
        Declare {
            desc: 0x00cb_b760,
            ctor: 0x005a_9db0,
            vtable: 0x00b3_57fc,
            name: 0x00cb_b974,
            element: Element::Getter(0x0043_7d30),
            flags: 2,
            member_vtable: 0x00b3_5840,
            set_storage: 0x005a_f8d0,
            atexit: [0x00b0_4050, 0x00b0_40b0, 0x00b0_4020, 0x00b0_40e0],
        },
    ),
    (
        0x005a_b790,
        Declare {
            desc: 0x00cb_b520,
            ctor: 0x005a_9e50,
            vtable: 0x00b3_55ec,
            name: 0x00cb_b96c,
            element: Element::Descriptor(0x00cb_b268),
            flags: 0,
            member_vtable: 0x00b3_5630,
            set_storage: 0x005a_f920,
            atexit: [0x00b0_4070, 0x00b0_40c0, 0x00b0_4030, 0x00b0_40f0],
        },
    ),
    (
        0x005a_b9a0,
        Declare {
            desc: 0x00cb_b640,
            ctor: 0x005a_9ef0,
            vtable: 0x00b3_56f4,
            name: 0x00cb_b970,
            element: Element::Descriptor(0x00cb_bff8),
            flags: 0,
            member_vtable: 0x00b3_5738,
            set_storage: 0x005a_f970,
            atexit: [0x00b0_4090, 0x00b0_40d0, 0x00b0_4040, 0x00b0_4100],
        },
    ),
    (
        0x005b_17c0,
        Declare {
            desc: 0x00cb_bd20,
            ctor: 0x005b_0910,
            vtable: 0x00b3_6274,
            name: 0x00cb_bf24,
            element: Element::Getter(0x0043_5740),
            flags: 2,
            member_vtable: 0x00b3_62b8,
            set_storage: 0x005b_33c0,
            atexit: [0x00b0_4200, 0x00b0_4220, 0x00b0_41f0, 0x00b0_4240],
        },
    ),
    (
        0x005c_18c0,
        Declare {
            desc: 0x00cb_ce60,
            ctor: 0x005b_e710,
            vtable: 0x00b3_7ec4,
            name: 0x00cb_d258,
            element: Element::Descriptor(0x00cb_c128),
            flags: 0,
            member_vtable: 0x00b3_7f08,
            set_storage: 0x005c_79c0,
            atexit: [0x00b0_4400, 0x00b0_4470, 0x00b0_43d0, 0x00b0_44d0],
        },
    ),
    (
        0x005c_1ad0,
        Declare {
            desc: 0x00cb_cd40,
            ctor: 0x005b_e7b0,
            vtable: 0x00b3_7dbc,
            name: 0x00cb_d254,
            element: Element::Descriptor(0x00cb_c090),
            flags: 0,
            member_vtable: 0x00b3_7e00,
            set_storage: 0x005c_7a10,
            atexit: [0x00b0_4420, 0x00b0_4480, 0x00b0_43e0, 0x00b0_44e0],
        },
    ),
    (
        0x005c_1ce0,
        Declare {
            desc: 0x00cb_cc20,
            ctor: 0x005b_e850,
            vtable: 0x00b3_7cb4,
            name: 0x00cb_d250,
            element: Element::Descriptor(0x00cb_bf60),
            flags: 0,
            member_vtable: 0x00b3_7cf8,
            set_storage: 0x005c_7a60,
            atexit: [0x00b0_4440, 0x00b0_4490, 0x00b0_43f0, 0x00b0_44f0],
        },
    ),
    (
        0x00a0_74c0,
        Declare {
            desc: 0x0194_5010,
            ctor: 0x009f_96d0,
            vtable: 0x00b7_6da4,
            name: 0x0198_4c20,
            element: Element::Descriptor(0x0194_35e8),
            flags: 2,
            member_vtable: 0x00b7_6de8,
            set_storage: 0x00a2_3be0,
            atexit: [0x00b0_8da0, 0x00b0_8ed0, 0x00b0_8d90, 0x00b0_8ef0],
        },
    ),
    (
        0x00a2_c6c0,
        Declare {
            desc: 0x01a8_50b0,
            ctor: 0x00a2_b780,
            vtable: 0x00b7_926c,
            name: 0x01a8_52e4,
            element: Element::Descriptor(0x01a8_4c88),
            flags: 0,
            member_vtable: 0x00b7_92b0,
            set_storage: 0x00a2_db70,
            atexit: [0x00b0_90e0, 0x00b0_9150, 0x00b0_90d0, 0x00b0_9160],
        },
    ),
];

/// The [`ABSTRACT`] constants of the copy at `addr` (a compile error when
/// there is none: the shims look theirs up in a `const` block).
#[must_use]
pub const fn abstract_of(addr: u32) -> Abstract {
    let mut i = 0;
    while i < ABSTRACT.len() {
        if ABSTRACT[i].0 == addr {
            return ABSTRACT[i].1;
        }
        i += 1;
    }
    panic!("no such copy")
}

/// The [`DECLARE`] constants of the copy at `addr`, as [`abstract_of`].
#[must_use]
pub const fn declare_of(addr: u32) -> Declare {
    let mut i = 0;
    while i < DECLARE.len() {
        if DECLARE[i].0 == addr {
            return DECLARE[i].1;
        }
        i += 1;
    }
    panic!("no such copy")
}

/// The input sources' semantic-check methods (thiscall on the source,
/// no stack words): (original, the semantics' rule, the default semantic
/// object stored over a rejected one).
pub const SEMANTIC_CHECKS: &[(u32, Rule, u32)] = &[
    (0x009f_0350, Rule::AtMost(0x1a), 0x00c6_b884),
    (0x009f_03c0, Rule::AtLeast(0x1c), 0x00c6_bc94),
];

/// The mouse-delta sources' checks ([`bridge::check_mouse_delta`]).
pub const MOUSE_DELTA_CHECKS: &[u32] = &[0x009f_0470, 0x009f_04f0];

/// The semantic object a mouse source falls back to (its first slot,
/// `+0x20` / `+0x1c`).
pub const MOUSE_DEFAULT: u32 = 0x00c6_c3cc;
/// The semantic object a mouse-delta source's second slot falls back to.
pub const MOUSE_DELTA_DEFAULT: u32 = 0x00c6_bc94;

/// Which semantic kinds (virtual slot 1 of the semantic object, signed)
/// a check keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Kinds up to this one.
    AtMost(i32),
    /// Kinds from this one up.
    AtLeast(i32),
}

impl Rule {
    #[must_use]
    pub const fn keeps(self, kind: u32) -> bool {
        match self {
            Rule::AtMost(n) => kind as i32 <= n,
            Rule::AtLeast(n) => kind as i32 >= n,
        }
    }
}

/// The input sources' `constructInstance` copies that end in a semantic
/// check: (original, [`+4`, `+8`, vtable], the check).
pub const CONSTRUCT_CHECKED: &[(u32, [u32; 3], u32)] = &[
    (
        0x009e_b0d0,
        [0x0194_0c00, 0x00c6_8234, 0x00b7_0b28],
        0x009f_0350,
    ),
    (
        0x009e_b160,
        [0x0194_0b68, 0x00c6_8228, 0x00b7_0a30],
        0x009f_03c0,
    ),
    (
        0x009e_b500,
        [0x0194_0dc8, 0x00c6_8258, 0x00b7_0e28],
        0x009f_0470,
    ),
    (
        0x009e_b590,
        [0x0194_0e60, 0x00c6_8264, 0x00b7_0f28],
        0x009f_04f0,
    ),
];

/// `PInputSourceMouseButton`'s `constructInstance` (`0x009eb470`):
/// [`+4`, `+8`, vtable].
pub const CONSTRUCT_MOUSE_BUTTON: [u32; 3] = [0x0194_0d30, 0x00c6_824c, 0x00b7_0d28];

/// The render delegation helper's runs (thiscall on the helper): each
/// runs the delegations whose flags (`+0xc`) have this bit.
pub const DELEGATION_RUNS: &[(u32, u32)] = &[
    (0x006a_d2d0, 0x200),
    (0x006a_d380, 0x800),
    (0x006a_d430, 0x4000),
    (0x006a_d4e0, 0x4),
    (0x006a_d600, 0x400),
    (0x006a_d6b0, 0x80),
    (0x006a_d760, 0x100),
    (0x006a_d810, 0x40),
    (0x006a_d8c0, 0x10),
    (0x006a_d970, 0x20),
];

/// The flag of the run at `addr` ([`DELEGATION_RUNS`]), as
/// [`abstract_of`].
#[must_use]
pub const fn delegation_flag(addr: u32) -> u32 {
    let mut i = 0;
    while i < DELEGATION_RUNS.len() {
        if DELEGATION_RUNS[i].0 == addr {
            return DELEGATION_RUNS[i].1;
        }
        i += 1;
    }
    panic!("no such run")
}

/// The helper's mutex handle (`.bss`).
pub const DELEGATION_MUTEX: u32 = 0x00cc_caa0;
/// The import slots of `WaitForSingleObject` and `ReleaseMutex` (stdcall).
pub const WAIT_FOR_SINGLE_OBJECT: u32 = crate::glue::WAIT_FOR_SINGLE_OBJECT;
pub const RELEASE_MUTEX: u32 = crate::glue::RELEASE_MUTEX;
/// `RenderDelegationHelper_setDataToFreeList(this, data)` (thiscall).
pub const SET_DATA_TO_FREE_LIST: u32 = 0x006a_e1a0;
/// The delegation array's compaction ([`bridge::compact_delegations`]).
pub const COMPACT: u32 = 0x006a_d590;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// The helper's free-list set-up `0x006adc90(this)` (thiscall).
pub const DELEGATION_FREE_LIST_INIT: u32 = 0x006a_dc90;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        COMPACT, Conv, DELEGATION_FREE_LIST_INIT, DELEGATION_MUTEX, Host, MOUSE_DEFAULT,
        MOUSE_DELTA_DEFAULT, OPERATOR_DELETE, RELEASE_MUTEX, Rule, SET_DATA_TO_FREE_LIST,
        WAIT_FOR_SINGLE_OBJECT,
    };
    use crate::p6_descriptor::bridge::construct_input;

    /// A semantic object's kind: virtual slot 1 (thiscall), 0 for none.
    fn kind(h: &mut impl Host, semantic: u32) -> u32 {
        if semantic == 0 {
            0
        } else {
            h.call_virtual(semantic, 1, &[])
        }
    }

    /// The kinds of both semantic slots stored (`+0x18` from `+0x20`,
    /// `+0x1c` from `+0x24`); `eax` is the last kind read (the first
    /// slot's when the second is empty).
    fn store_kinds(h: &mut impl Host, source: u32) -> u32 {
        let first: u32 = h.read(source + 0x20);
        let a = kind(h, first);
        let second: u32 = h.read(source + 0x24);
        h.write(source + 0x18, a);
        if second == 0 {
            h.write(source + 0x1c, 0u32);
            return a;
        }
        let b = h.call_virtual(second, 1, &[]);
        h.write(source + 0x1c, b);
        b
    }

    /// The joypad-button and key sources' semantic check
    /// (`0x009f0350`, `0x009f03c0`; thiscall): each slot (`+0x20`,
    /// `+0x24`) whose kind (0 when empty) `rule` rejects gets `default`;
    /// then [`store_kinds`].
    pub fn check_semantics(h: &mut impl Host, source: u32, rule: Rule, default: u32) -> u32 {
        let first: u32 = h.read(source + 0x20);
        let a = kind(h, first);
        let second: u32 = h.read(source + 0x24);
        let b = kind(h, second);
        let default = h.rebase(default);
        if !rule.keeps(a) {
            h.write(source + 0x20, default);
        }
        if !rule.keeps(b) {
            h.write(source + 0x24, default);
        }
        store_kinds(h, source)
    }

    /// Kinds 0xb, 0xc and 0xd: the mouse's.
    fn mouse_kind(k: u32) -> bool {
        matches!(k, 0xb..=0xd)
    }

    /// The mouse-delta sources' semantic check (`0x009f0470`,
    /// `0x009f04f0`, the same code; thiscall): an empty first slot or one
    /// of another kind gets [`MOUSE_DEFAULT`]; an empty second slot gets
    /// [`MOUSE_DELTA_DEFAULT`], and so does a set one when the FIRST
    /// slot's kind (read again, as the original does) is not the mouse's;
    /// then [`store_kinds`].
    pub fn check_mouse_delta(h: &mut impl Host, source: u32) -> u32 {
        let first: u32 = h.read(source + 0x20);
        if first == 0 || !mouse_kind(h.call_virtual(first, 1, &[])) {
            let d = h.rebase(MOUSE_DEFAULT);
            h.write(source + 0x20, d);
        }
        let reject = if h.read::<u32>(source + 0x24) == 0 {
            true
        } else {
            let first: u32 = h.read(source + 0x20);
            !mouse_kind(h.call_virtual(first, 1, &[]))
        };
        if reject {
            let d = h.rebase(MOUSE_DELTA_DEFAULT);
            h.write(source + 0x24, d);
        }
        store_kinds(h, source)
    }

    /// `constructInstance(obj)` of an input source with two semantic
    /// slots (thiscall on the descriptor, `ret 4`; in an SEH frame,
    /// which nothing here throws through): as
    /// [`crate::p6_descriptor::bridge::construct_input`], both slots
    /// (`+0x20`, `+0x24`) empty, then the source's check (`check`,
    /// thiscall on `obj`), when `obj` is set. `al = 1`; the rest of `eax`
    /// is the check's, or a stack address for a null `obj`, which no
    /// caller reads.
    pub fn construct_checked(h: &mut impl Host, obj: u32, fields: [u32; 3], check: u32) -> u8 {
        if obj != 0 {
            construct_input(h, obj, fields);
            h.write(obj.wrapping_add(0x20), 0u32);
            h.write(obj.wrapping_add(0x24), 0u32);
            h.call_sym(check, Conv::Thiscall(obj), &[]);
        }
        1
    }

    /// `PInputSourceMouseButton`'s `constructInstance(obj)`
    /// (`0x009eb470`): as [`crate::p6_descriptor::bridge::construct_input`],
    /// then the semantic slot `+0x1c` = [`MOUSE_DEFAULT`] and `+0x18` its
    /// kind, when `obj` is set. `al = 1`, as [`construct_checked`].
    pub fn construct_mouse_button(h: &mut impl Host, obj: u32, fields: [u32; 3]) -> u8 {
        if obj != 0 {
            construct_input(h, obj, fields);
            let d = h.rebase(MOUSE_DEFAULT);
            h.write(obj.wrapping_add(0x1c), d);
            let k = h.call_virtual(d, 1, &[]);
            h.write(obj.wrapping_add(0x18), k);
        }
        1
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    fn entry_at(h: &mut impl Host, helper: u32, i: u32) -> u32 {
        let entries: u32 = h.read(helper.wrapping_add(4));
        h.read(entries.wrapping_add(i.wrapping_mul(4)))
    }

    /// The render delegation helper's run of the delegations flagged
    /// `flag` (thiscall on the helper, a `PArray` of entries: count
    /// `+0`, low 31 bits; data `+4`), under the mutex
    /// [`DELEGATION_MUTEX`]: each entry with `flag` in its flags (`+0xc`)
    /// runs its callback (`+0x10`, cdecl, with `+0`); a one-shot entry
    /// (flag 1, of the entry in the slot after the callback) then goes
    /// back to the free list, or when that fails (negative) is deleted
    /// with its buffer (`+4`; `+8` cleared first); either way its slot is
    /// cleared and the array compacted, the index staying put. Returns
    /// `ReleaseMutex`'s result.
    pub fn run_delegations(h: &mut impl Host, helper: u32, flag: u32) -> u32 {
        let mutex: u32 = h.read(h.rebase(DELEGATION_MUTEX));
        import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, u32::MAX]);
        let mut i = 0u32;
        while i < h.read::<u32>(helper) & 0x7fff_ffff {
            let entry = entry_at(h, helper, i);
            if entry != 0 && h.read::<u32>(entry.wrapping_add(0xc)) & flag != 0 {
                let arg: u32 = h.read(entry);
                let callback: u32 = h.read(entry.wrapping_add(0x10));
                h.call(callback, Conv::Cdecl, &[arg]);
                let entry = entry_at(h, helper, i);
                if h.read::<u8>(entry.wrapping_add(0xc)) & 1 != 0 {
                    let r = h.call_sym(SET_DATA_TO_FREE_LIST, Conv::Thiscall(helper), &[entry]);
                    if (r as i32) < 0 {
                        let dead = entry_at(h, helper, i);
                        if dead != 0 {
                            let buffer: u32 = h.read(dead.wrapping_add(4));
                            if buffer != 0 {
                                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buffer]);
                            }
                            h.write(dead.wrapping_add(8), 0u32);
                            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[dead]);
                        }
                    }
                    let entries: u32 = h.read(helper.wrapping_add(4));
                    h.write(entries.wrapping_add(i.wrapping_mul(4)), 0u32);
                    i = i.wrapping_sub(1);
                    h.call_sym(COMPACT, Conv::Thiscall(helper), &[]);
                }
            }
            i = i.wrapping_add(1);
        }
        let mutex: u32 = h.read(h.rebase(DELEGATION_MUTEX));
        import(h, RELEASE_MUTEX, &[mutex])
    }

    /// The delegation array's compaction (`0x006ad590`, thiscall): the
    /// entries before the last set one move down over the empty slots,
    /// in order; each move clears the slot it vacates at the end. The
    /// last entry itself is never moved: with it at index 0 or no entry
    /// set, nothing changes. `eax` is left over (callers reload it).
    pub fn compact_delegations(h: &mut impl Host, helper: u32) {
        let n = h.read::<u32>(helper) & 0x7fff_ffff;
        let data: u32 = h.read(helper.wrapping_add(4));
        let mut last: i32 = -1;
        let mut k = n as i32 - 1;
        while k >= 0 {
            if h.read::<u32>(data.wrapping_add(4 * k as u32)) != 0 {
                last = k;
                break;
            }
            k -= 1;
        }
        let mut i: i32 = 0;
        if last <= 0 {
            return;
        }
        loop {
            let data: u32 = h.read(helper.wrapping_add(4));
            if h.read::<u32>(data.wrapping_add(4 * i as u32)) == 0 {
                let mut j = i;
                while j < last {
                    let data: u32 = h.read(helper.wrapping_add(4));
                    let at = data.wrapping_add(4 * j as u32);
                    let next: u32 = h.read(at.wrapping_add(4));
                    h.write(at, next);
                    j += 1;
                }
                let data: u32 = h.read(helper.wrapping_add(4));
                last -= 1;
                h.write(data.wrapping_add(4 * last as u32).wrapping_add(4), 0u32);
                i -= 1;
            }
            i += 1;
            if i >= last {
                return;
            }
        }
    }

    /// The render delegation helper's constructor (`0x006ad0d0`,
    /// thiscall; in an SEH frame nothing throws through): an empty array
    /// (`+0`, `+4`), then the free list's set-up
    /// ([`super::DELEGATION_FREE_LIST_INIT`]). Returns the helper.
    pub fn delegation_helper_ctor(h: &mut impl Host, helper: u32) -> u32 {
        h.write(helper, 0u32);
        h.write(helper.wrapping_add(4), 0u32);
        h.call_sym(DELEGATION_FREE_LIST_INIT, Conv::Thiscall(helper), &[]);
        helper
    }
}
