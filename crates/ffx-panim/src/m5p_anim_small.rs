//! The small functions of list `phyre-anim` (M5 stage 1, batch 1): the
//! template copies the animation classes' descriptors and arrays share,
//! and the controllers' one-line virtual methods.
//!
//! - `PClassDescriptorForType<T>` destructors: the copy's vtable, then a
//!   tail jump to `PClassDescriptor::Destruct` ([`TAIL_DTORS`],
//!   [`bridge::tail_dtor`]);
//! - `PArray<T>`'s lazily built swap helpers ([`SWAP_LAZY`], the generic
//!   port of list tmpl-fortype, `ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy`);
//! - `PArray<T>` destructors and `setBorrowed`: release the elements the
//!   array owns (top bit of the count clear), then zero both words;
//! - scalar deleting destructors of the channel-target binders, the
//!   hierarchy visitors and the thread-pool jobs;
//! - getters, size queries and constant virtual methods.
//!
//! Everything else (`PhyreFree`, `operator delete`, the type registry,
//! callees outside the list) stays a call to the original. Every port
//! returns the `eax` the original leaves where its callers can read it.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

pub use ffx_pload::m5p_tmpl_fortype::Lazy;

/// `Phyre::PClassDescriptor::Destruct` (thiscall), the tail-jump target of
/// [`TAIL_DTORS`].
pub const DESTRUCT: u32 = 0x0043_b3e0;
/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Phyre::PString::Assign(text)` (thiscall, `ret 4`).
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `Phyre::PRedBlackTree::clear()` (thiscall, no stack words).
pub const TREE_CLEAR: u32 = 0x0057_4760;
/// The script layer's object push `(L, object, class)` (cdecl).
pub const PUSH_OBJECT: u32 = 0x0043_6e80;
/// `Phyre::PScripting::PAsyncProcessHeader::process()` (thiscall).
pub const ASYNC_PROCESS: u32 = 0x0061_7f30;
/// `PThreadPoolJob::~PThreadPoolJob()` (thiscall; stores its vtable).
pub const THREAD_POOL_JOB_DTOR: u32 = 0x005c_d3c0;
/// `PFreelistAllocatedThreadPoolJob`'s `operator delete` (cdecl, a bare
/// `ret`: the freelist keeps the job).
pub const FREELIST_DELETE: u32 = 0x005c_d3e0;
/// `PTimeController`'s constant time (`f32`).
pub const TIME_CONSTANT: u32 = 0x00ca_b918;

/// The vtable `PAnimationChannelTargetBind::~PAnimationChannelTargetBind`
/// (`0x005cdb70`) stores.
pub const TARGET_BIND_VTABLE: u32 = 0x00b2_a31c;
/// The vtable `PAnimationHierarchyVisitContext::~PAnimationHierarchyVisitContext`
/// (`0x00521230`) stores.
pub const VISIT_CONTEXT_VTABLE: u32 = 0x00b2_80ac;
/// `PAnimationHierarchyNode`'s vtable.
pub const HIERARCHY_NODE_VTABLE: u32 = 0x00b2_8150;

/// The top bit of a `PArray` count: the elements are not the array's.
pub const NOT_OWNED: u32 = 0x8000_0000;

include!("m5p_anim_table.rs.in");

/// The word of copy `f` in `table` (a compile error when `f` is missing).
#[must_use]
pub const fn word_of(table: &[(u32, u32)], f: u32) -> u32 {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`SWAP_LAZY`] entry of copy `f`.
#[must_use]
pub const fn lazy_of(f: u32) -> Lazy {
    let mut i = 0;
    while i < SWAP_LAZY.len() {
        if SWAP_LAZY[i].0 == f {
            return SWAP_LAZY[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// What a scalar deleting destructor runs before the free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// The destructor is a vtable store (inlined, or one of this list's).
    Vtable(u32),
    /// An original destructor (`thiscall(this)`).
    Call(u32),
}

/// A scalar deleting destructor: what it destructs with and frees with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delete {
    pub base: Base,
    /// The cdecl `(p)` free.
    pub free: u32,
}

/// The deleting destructors (`thiscall(this, flags)` -> `this`).
pub const DELETES: &[(u32, Delete)] = &[
    // PAnimationHierarchyNode: its vtable, PhyreFree.
    (
        0x0052_12a0,
        Delete {
            base: Base::Vtable(HIERARCHY_NODE_VTABLE),
            free: PHYRE_FREE,
        },
    ),
    // PAnimationHierarchyVisitContext.
    (
        0x0052_12d0,
        Delete {
            base: Base::Vtable(VISIT_CONTEXT_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    // The channel-target binders: ~PAnimationChannelTargetBind (0x005cdb70).
    (
        0x0053_5620,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_5650,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_5680,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_56b0,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_56e0,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_5710,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_5740,
        Delete {
            base: Base::Vtable(TARGET_BIND_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    // The hierarchy visitors: ~PAnimationHierarchyVisitContext (0x00521230).
    (
        0x0053_5770,
        Delete {
            base: Base::Vtable(VISIT_CONTEXT_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0053_57a0,
        Delete {
            base: Base::Vtable(VISIT_CONTEXT_VTABLE),
            free: OPERATOR_DELETE,
        },
    ),
    // The thread-pool jobs.
    (
        0x0053_5830,
        Delete {
            base: Base::Call(THREAD_POOL_JOB_DTOR),
            free: FREELIST_DELETE,
        },
    ),
    (
        0x0053_5860,
        Delete {
            base: Base::Call(THREAD_POOL_JOB_DTOR),
            free: OPERATOR_DELETE,
        },
    ),
];

/// The [`DELETES`] entry of copy `f`.
#[must_use]
pub const fn delete_of(f: u32) -> Delete {
    let mut i = 0;
    while i < DELETES.len() {
        if DELETES[i].0 == f {
            return DELETES[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The `PArray<T>` destructors that free owned elements as they are
/// (`0x00510150` and its copies).
pub const RELEASES: &[u32] = &[
    0x0051_0150,
    0x0053_4790,
    0x0053_47c0,
    0x0053_47f0,
    0x0053_4850,
    0x0053_4880,
    0x0061_71e0,
];

/// The `PArray<T>` destructors that run each owned element's virtual
/// destructor first: `(copy, element size)`.
pub const RELEASES_VIRTUAL: &[(u32, u32)] = &[
    (0x0053_4550, 0xc),
    (0x0053_45b0, 8),
    (0x0053_4610, 0xc),
    (0x0053_4670, 8),
    (0x0053_46d0, 8),
    (0x0053_4730, 8),
];

/// The `PArray<T>` destructors that hand owned elements to a cdecl
/// `(els, count)` helper: `(copy, helper)`.
pub const RELEASES_WITH: &[(u32, u32)] = &[
    // Phyre_PAnimation_PAnimationChannelTarget_freeArray
    (0x0051_9180, 0x0051_a890),
    // Phyre_DeleteArray4c_00540b80
    (0x0053_e9e0, 0x0054_0b80),
];

/// `PArray<T>::setBorrowed` copies (`Phyre_PArray_float_setBorrowed`'s
/// shape, `crate::array::bridge::set_borrowed` with `Old::Free`).
pub const SET_BORROWED: &[u32] = &[
    0x0052_0e00,
    0x0053_c810,
    0x0053_c860,
    0x0053_c8b0,
    0x0053_c900,
];

/// The virtual methods that return a constant (`mov eax, imm; ret`):
/// `(copy, value, whether it is an image address)`.
pub const CONSTANTS: &[(u32, u32, bool)] = &[
    (0x0052_4cb0, 1, false),
    (0x0052_4cd0, 0xc, false),
    (0x0052_4d10, 0x0052_4be0, true),
    (0x0052_6390, 0x0052_58d0, true),
    (0x0052_a4f0, 0x0052_9e60, true),
    (0x0052_a510, 1, false),
    (0x0052_e280, 0x0052_e140, true),
    (0x0052_f770, 0x0052_f600, true),
    (0x0053_06b0, 0x0053_0580, true),
    (0x0053_06e0, 1, false),
    (0x0053_15e0, 1, false),
];

/// The [`CONSTANTS`] entry of copy `f`.
#[must_use]
pub const fn constant_of(f: u32) -> (u32, bool) {
    let mut i = 0;
    while i < CONSTANTS.len() {
        if CONSTANTS[i].0 == f {
            return (CONSTANTS[i].1, CONSTANTS[i].2);
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The per-batch scratch size queries (`thiscall()`): `(copy, the count's
/// place, the bytes added before the 16-byte rounding)`. The size is
/// `((count * 4 + extra) & !0xf) + 0x10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    /// `[this + off] & 0x7fffffff`.
    Own(u32),
    /// `[[this + 8] + 0x10] & 0x7fffffff`.
    Source,
}

/// The size queries.
pub const SIZES: &[(u32, Count, u32)] = &[
    (0x0052_a4d0, Count::Source, 0x13),
    (0x0052_e260, Count::Own(0x14), 0xf),
    (0x0053_0690, Count::Source, 0x1f),
];

/// The [`SIZES`] entry of copy `f`.
#[must_use]
pub const fn size_of(f: u32) -> (Count, u32) {
    let mut i = 0;
    while i < SIZES.len() {
        if SIZES[i].0 == f {
            return (SIZES[i].1, SIZES[i].2);
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Checks that `f` is in `table`.
pub const fn listed(table: &[u32], f: u32) {
    let mut i = 0;
    while i < table.len() {
        if table[i] == f {
            return;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// What `shr eax, 31; not al` leaves of an owned/not-owned count when the
/// release finds nothing it owns: `0xfe`.
const NOT_OWNED_EAX: u32 = 0xfe;

pub mod bridge {
    use super::{
        ASYNC_PROCESS, Base, Conv, Count, Delete, Host, Lazy, Mem, NOT_OWNED, NOT_OWNED_EAX,
        PHYRE_FREE, PUSH_OBJECT, STRING_ASSIGN, TIME_CONSTANT, TREE_CLEAR,
    };

    /// A [`super::TAIL_DTORS`] copy: `vtable` at `this`, then
    /// `PClassDescriptor::Destruct` (thiscall on `this`), whose `eax` it
    /// returns.
    pub fn tail_dtor(h: &mut impl Host, vtable: u32, this: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(super::DESTRUCT, Conv::Thiscall(this), &[])
    }

    /// A destructor that only stores its class's vtable (`0x005cdb70`,
    /// `0x00521230`); `eax` stays the caller's.
    pub fn set_vtable(m: &mut impl Mem, this: u32, vtable: u32) {
        let vt = m.rebase(vtable);
        m.write(this, vt);
    }

    /// A [`super::DELETES`] copy (`thiscall(this, flags)`): the base
    /// destructor, `free(this)` when `flags & 1`; returns `this`.
    pub fn delete(h: &mut impl Host, d: Delete, this: u32, flags: u32) -> u32 {
        match d.base {
            Base::Vtable(vt) => set_vtable(h, this, vt),
            Base::Call(f) => {
                h.call_sym(f, Conv::Thiscall(this), &[]);
            }
        }
        if flags & 1 != 0 {
            h.call_sym(d.free, Conv::Cdecl, &[this]);
        }
        this
    }

    /// A [`super::SWAP_LAZY`] copy: the tmpl-fortype generic port.
    pub fn swap_lazy(h: &mut impl Host, l: &Lazy, p: u32) -> u32 {
        ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy(h, l, p)
    }

    /// A [`super::RELEASES`] copy (`thiscall`, `PArray<T>::~PArray`):
    /// `PhyreFree` of the elements when the array owns them and they are
    /// not null, then count and pointer zero. `eax`: `0xfe` when not
    /// owned, else the null pointer or `PhyreFree`'s.
    pub fn release(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = NOT_OWNED_EAX;
        if count & NOT_OWNED == 0 {
            let els: u32 = h.read(this + 4);
            eax = els;
            if els != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[els]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// A [`super::RELEASES_WITH`] copy: owned elements go to
    /// `helper(els, count)` (null or not), then count and pointer zero.
    /// `eax`: `0xfe` when not owned, else the helper's.
    pub fn release_with(h: &mut impl Host, helper: u32, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = NOT_OWNED_EAX;
        if count & NOT_OWNED == 0 {
            let els: u32 = h.read(this + 4);
            eax = h.call_sym(helper, Conv::Cdecl, &[els, count & !NOT_OWNED]);
        }
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// A [`super::RELEASES_VIRTUAL`] copy: when the array owns non-null
    /// elements, each one's virtual slot 0 (the scalar deleting
    /// destructor) with flags 0, then `PhyreFree` of the array; count and
    /// pointer zero. `eax` as [`release`].
    pub fn release_virtual(h: &mut impl Host, stride: u32, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let mut eax = NOT_OWNED_EAX;
        if count & NOT_OWNED == 0 {
            let els: u32 = h.read(this + 4);
            eax = els;
            if els != 0 {
                let mut el = els;
                for _ in 0..count & !NOT_OWNED {
                    h.call_virtual(el, 0, &[0]);
                    el = el.wrapping_add(stride);
                }
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[els]);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// A [`super::CONSTANTS`] copy: the value, rebased when it is an
    /// image address.
    pub fn constant(m: &impl Mem, (value, address): (u32, bool)) -> u32 {
        if address { m.rebase(value) } else { value }
    }

    /// A [`super::SIZES`] copy (`thiscall`).
    pub fn size(m: &impl Mem, (count, extra): (Count, u32), this: u32) -> u32 {
        let n: u32 = match count {
            Count::Own(off) => m.read(this.wrapping_add(off)),
            Count::Source => {
                let src: u32 = m.read(this.wrapping_add(8));
                m.read(src.wrapping_add(0x10))
            }
        };
        ((n & !NOT_OWNED).wrapping_mul(4).wrapping_add(extra) & !0xf).wrapping_add(0x10)
    }

    /// `[this + off] & 0x7fffffff` (`thiscall`; `0x0052cc80`: `+0xc`,
    /// `0x00616a20`: `+0x100`): an array's count.
    pub fn count_at(m: &impl Mem, this: u32, off: u32) -> u32 {
        m.read::<u32>(this.wrapping_add(off)) & !NOT_OWNED
    }

    /// `[this + off]` (`0x0052a500`: `+0x14`, `0x00616aa0`: `+0x118`).
    pub fn word_at(m: &impl Mem, this: u32, off: u32) -> u32 {
        m.read(this.wrapping_add(off))
    }

    /// `0x00616eb0` (`thiscall(v)`): `[this + 0x118] = v`; `eax = v`.
    pub fn set_word_118(m: &mut impl Mem, this: u32, v: u32) -> u32 {
        m.write(this.wrapping_add(0x118), v);
        v
    }

    /// `0x00616d50` (`thiscall`): `[this + 0x118] != -1`.
    pub fn has_word_118(m: &impl Mem, this: u32) -> u32 {
        u32::from(m.read::<u32>(this.wrapping_add(0x118)) != u32::MAX)
    }

    /// `Phyre_PAnimation_PAnimationController_setTimeController`
    /// (`0x005265f0`, `thiscall(tc)`): `[this + 0x10] = tc`; 0.
    pub fn set_time_controller(m: &mut impl Mem, this: u32, tc: u32) -> u32 {
        m.write(this.wrapping_add(0x10), tc);
        0
    }

    /// `0x0052cc50` (`thiscall(i)`): source `i` of a blender (`[this +
    /// 0x10]` array, count at `+0xc`), or 0 past the end.
    pub fn source_at(m: &impl Mem, this: u32, i: u32) -> u32 {
        if i < count_at(m, this, 0xc) {
            let els: u32 = m.read(this.wrapping_add(0x10));
            m.read(els.wrapping_add(i.wrapping_mul(4)))
        } else {
            0
        }
    }

    /// `0x005111c0`, `0x00432f50` (`thiscall(i)`): word `i` of the array at `[this + 8]`.
    pub fn element_8(m: &impl Mem, this: u32, i: u32) -> u32 {
        let els: u32 = m.read(this.wrapping_add(8));
        m.read(els.wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x00432f70` (`thiscall(i)`): the address of element `i` (`0x34`
    /// bytes each) of the array at `[this + 0x10]`.
    pub fn element_34(m: &impl Mem, this: u32, i: u32) -> u32 {
        let els: u32 = m.read(this.wrapping_add(0x10));
        i.wrapping_mul(0x34).wrapping_add(els)
    }

    /// `0x005306c0` (`thiscall(i)`): `[this + 0xc]` for `i == 0`, else 0.
    pub fn first_source(m: &impl Mem, this: u32, i: u32) -> u32 {
        if i == 0 {
            m.read(this.wrapping_add(0xc))
        } else {
            0
        }
    }

    /// `Phyre_PAnimation_PAnimationChannel_getBatchSortKey` (`0x0050e730`,
    /// `thiscall`): `[this + 0x20]`, or'ed with `[[this + 0x1c] + 0xc] <<
    /// 2` when `[this + 0x1c]` is set.
    pub fn batch_sort_key(m: &impl Mem, this: u32) -> u32 {
        let key: u32 = m.read(this.wrapping_add(0x20));
        let p: u32 = m.read(this.wrapping_add(0x1c));
        if p == 0 {
            key
        } else {
            key | m.read::<u32>(p.wrapping_add(0xc)) << 2
        }
    }

    /// `0x0045de80` (`thiscall(out)`): `out[0] = [this + 0x1c]`, `out[1] =
    /// [this + 0x20]`; `eax = out`.
    pub fn copy_pair(m: &mut impl Mem, this: u32, out: u32) -> u32 {
        let a: u32 = m.read(this.wrapping_add(0x1c));
        let b: u32 = m.read(this.wrapping_add(0x20));
        m.write(out, a);
        m.write(out.wrapping_add(4), b);
        out
    }

    /// `0x00526250` (`thiscall`): `([[this + 8] + 0x10] & 0x7fffffff) * 4`,
    /// 0 without a source.
    pub fn source_bytes(m: &impl Mem, this: u32) -> u32 {
        let src: u32 = m.read(this.wrapping_add(8));
        if src == 0 {
            0
        } else {
            (m.read::<u32>(src.wrapping_add(0x10)) & !NOT_OWNED) << 2
        }
    }

    /// `0x00510f60` (cdecl `(a, b)`): the order of two modules by their
    /// word at `+0x14` (unsigned): -1 when `b`'s is greater, 1 when it is
    /// less, 0 when equal.
    pub fn compare_modules(m: &impl Mem, a: u32, b: u32) -> u32 {
        let key_a: u32 = m.read(a.wrapping_add(0x14));
        let key_b: u32 = m.read(b.wrapping_add(0x14));
        if key_b > key_a {
            u32::MAX
        } else {
            u32::from(key_b < key_a)
        }
    }

    /// `0x005cea70` (`thiscall`): each process header from `[this + 4]`
    /// along `+8` runs `PAsyncProcessHeader::process`; the first non-zero
    /// result, or 0.
    pub fn process_all(h: &mut impl Host, this: u32) -> u32 {
        let mut p: u32 = h.read(this.wrapping_add(4));
        while p != 0 {
            let r = h.call_sym(ASYNC_PROCESS, Conv::Thiscall(p), &[]);
            if r != 0 {
                return r;
            }
            p = h.read(p.wrapping_add(8));
        }
        0
    }

    /// `0x005074c0`, `0x00517220` (`thiscall(text)`): `PString::Assign` on
    /// the string at `this + off` (a tail jump); its `eax`.
    pub fn assign_string(h: &mut impl Host, this: u32, off: u32, text: u32) -> u32 {
        h.call_sym(
            STRING_ASSIGN,
            Conv::Thiscall(this.wrapping_add(off)),
            &[text],
        )
    }

    /// `0x005ce190` (`thiscall`): `PRedBlackTree::clear` on the tree at
    /// `this + 0xc` (a tail jump); its `eax`.
    pub fn clear_tree(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this.wrapping_add(0xc)), &[])
    }

    /// `0x00524bb0` (cdecl `(L, event, class)`): pushes the event's
    /// object (`event + 4`, or null for a null event) through the script
    /// layer's object push (a tail jump); its `eax`.
    pub fn push_event(h: &mut impl Host, l: u32, event: u32, class: u32) -> u32 {
        let object = if event == 0 { 0 } else { event.wrapping_add(4) };
        h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, object, class])
    }

    /// `0x005cdab0` / `0x005cdac0` (`thiscall(a, b)`, the node counter's
    /// visits): `[this + off] += 1`; `al = 1`.
    pub fn count_visit(m: &mut impl Mem, this: u32, off: u32) -> u8 {
        let at = this.wrapping_add(off);
        let n: u32 = m.read(at);
        m.write(at, n.wrapping_add(1));
        1
    }

    /// `PTimeController`'s constant time (`0x00522110`): the `f32` at
    /// [`TIME_CONSTANT`], left in `ST0`.
    pub fn time_constant(m: &impl Mem) -> f32 {
        let at = m.rebase(TIME_CONSTANT);
        f32::from_bits(m.read(at))
    }
}
