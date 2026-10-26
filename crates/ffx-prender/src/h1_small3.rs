//! Small `PhyreEngine` methods, round 3 (M4 port, h1-render-a): array
//! destructors, element constructors and destructors, array resizes, the
//! world's class-list set-up, a recursive lock's release, the render
//! interface binder's scope, a shader pass's resource unbinding, the
//! nested-class visitor's step, and the context switch factory.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::basic::{FREE, MALLOC};

/// The count word's "not owned" flag.
const EXTERNAL: u32 = 0x8000_0000;

/// How a destructor releases one array (count word, data word).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rel {
    /// `PhyreFree(data)` unless `data` is 0.
    Free,
    /// `f(data, count)` (cdecl): destroys the elements and frees the block.
    Elements(u32),
    /// `PhyreFree(data)` when the count is more than 1 (a `PSharray`'s
    /// single element lives in the header), `data` not checked.
    FreeShared,
}

/// A destructor of up to two arrays: the vtable stored first (0: none),
/// the arrays in order (count offset, data offset, how), and a last
/// `thiscall` on `this + 4` (0: none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dtor {
    pub vtable: u32,
    pub arrays: [(u32, u32, Rel); 2],
    pub tail: u32,
}

/// The destructors, by address.
pub const DTORS: &[(u32, Dtor)] = &[
    (
        0x0048_4160,
        Dtor {
            vtable: 0,
            arrays: [(0x10, 0x14, Rel::Free), (0x8, 0xc, Rel::Free)],
            tail: 0,
        },
    ),
    (
        0x0049_69f0,
        Dtor {
            vtable: 0x00b1_6868,
            arrays: [(0x10, 0x14, Rel::Free), (0x8, 0xc, Rel::Free)],
            tail: 0,
        },
    ),
    (
        0x0049_9dc0,
        Dtor {
            vtable: 0,
            arrays: [(0x10, 0x14, Rel::Free), (0x4, 0x8, Rel::Free)],
            tail: 0,
        },
    ),
    (
        0x004a_8860,
        Dtor {
            vtable: 0,
            arrays: [
                (0x8, 0xc, Rel::Elements(RELEASE_TAGGED_16)),
                (0x0, 0x4, Rel::Elements(RELEASE_TAGGED_16B)),
            ],
            tail: 0,
        },
    ),
    (
        0x004b_bf50,
        Dtor {
            vtable: 0,
            arrays: [
                (0x14, 0x18, Rel::Elements(RELEASE_TAGGED_16B)),
                (0xc, 0x10, Rel::Elements(RELEASE_TAGGED_16)),
            ],
            tail: RELEASE_SHADER_PASSES,
        },
    ),
];

/// `PArray_releaseTaggedElements_004ab4d0(data, count)` (cdecl).
pub const RELEASE_TAGGED_16: u32 = 0x004a_b4d0;
/// `PArray_releaseTaggedElements_004ab510(data, count)` (cdecl).
pub const RELEASE_TAGGED_16B: u32 = 0x004a_b510;
/// `PArray_releaseTaggedWords_00434360(data, count)` (cdecl).
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;
/// `PArray_releaseShaderPasses_004bbe40(this)` (thiscall).
pub const RELEASE_SHADER_PASSES: u32 = 0x004b_be40;

/// The destructor `f` (compile time).
#[must_use]
pub const fn dtor_of(f: u32) -> Dtor {
    let mut i = 0;
    while i < DTORS.len() {
        if DTORS[i].0 == f {
            return DTORS[i].1;
        }
        i += 1;
    }
    panic!("not a destructor of h1_small3")
}

/// The constructor of a 0x6c-byte entry's member at `+0x30` (thiscall).
pub const ENTRY6C_MEMBER_CTOR: u32 = 0x0055_dd30;
/// Its destructor (thiscall).
pub const ENTRY6C_MEMBER_DTOR: u32 = 0x0055_e060;
/// The destructor of a 0x40-byte element (thiscall).
pub const ELEMENT40_DTOR: u32 = 0x0055_dfa0;

/// An array resize: element size, copy (cdecl `(dst, src, n)`) and
/// default constructor (cdecl `(data, n)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resize {
    pub size: u32,
    pub copy: u32,
    pub init: u32,
}

/// The resizes, by address.
pub const RESIZES: &[(u32, Resize)] = &[
    (
        0x0044_3d20,
        Resize {
            size: 20,
            copy: 0x0044_2290,
            init: 0x0044_3150,
        },
    ),
    (
        0x0048_3710,
        Resize {
            size: 12,
            copy: 0x0048_0bb0,
            init: 0x0048_2d70,
        },
    ),
];

/// The resize `f` (compile time).
#[must_use]
pub const fn resize_of(f: u32) -> Resize {
    let mut i = 0;
    while i < RESIZES.len() {
        if RESIZES[i].0 == f {
            return RESIZES[i].1;
        }
        i += 1;
    }
    panic!("not a resize of h1_small3")
}

/// The number of world class entries (`PWorld::initialize` makes one
/// more).
pub const WORLD_CLASS_COUNT: u32 = 0x00c9_075c;
/// `Phyre_PNamespace_GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;

/// `0x0042fd00()` (cdecl): the current thread's id.
pub const THREAD_ID: u32 = 0x0042_fd00;
/// The `InterlockedCompareExchange` import slot.
pub const INTERLOCKED_CAS: u32 = 0x00b0_c12c;
/// `Phyre_PEventFlagWin32_signal(this)` (thiscall).
pub const EVENT_SIGNAL: u32 = 0x0043_0200;

/// The render interface lock's guard word, object, constructor and
/// `atexit` destructor.
pub const LOCK_GUARD: u32 = 0x00c9_4efc;
pub const LOCK: u32 = 0x00c9_4ee4;
pub const LOCK_CTOR: u32 = 0x0043_15e0;
pub const LOCK_ATEXIT: u32 = 0x00af_b700;
/// `0x00431840(lock)` (thiscall): takes the lock.
pub const LOCK_TAKE: u32 = 0x0043_1840;
/// `0x004317d0(lock)` (thiscall): the lock's depth.
pub const LOCK_DEPTH: u32 = 0x0043_17d0;
/// The render interface's guard word, object, constructor and `atexit`
/// destructor.
pub const INTERFACE_GUARD: u32 = 0x00c9_628c;
pub const INTERFACE: u32 = 0x00c9_4f00;
pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const INTERFACE_ATEXIT: u32 = 0x00af_b710;
/// `PRenderInterface::bindToCurrentThread(interface)` (thiscall).
pub const INTERFACE_BIND: u32 = 0x0058_de10;
/// `0x0059d2e0(interface)` (thiscall): the unbind.
pub const INTERFACE_UNBIND: u32 = 0x0059_d2e0;
/// `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `0x004319d0(lock)` (thiscall): releases the lock.
pub const LOCK_RELEASE: u32 = 0x0043_19d0;

/// The shader pass's resource-table callees (thiscall on the pass):
/// `(a, b, slot)` twice for slots 0..3, then `(a, b)`, `(a)`, `(a)`.
pub const PASS_UNBIND: [u32; 5] = [
    0x0059_8a20,
    0x0059_8a80,
    0x0059_8a70,
    0x0059_8a30,
    0x0059_8a60,
];

/// `0x0047d2f0(this, class)` and `0x0047d3e0(this, entry, &offset, rem,
/// x)` (thiscall on the visitor's `+8` object).
pub const NESTED_FIND: u32 = 0x0047_d2f0;
pub const NESTED_VISIT: u32 = 0x0047_d3e0;

/// The parameter buffer's `destroyInstance` helpers: `0x0059e530()`
/// (cdecl, a static getter) and the local visitor's vtable.
pub const PARAMETER_BUFFER_HELPER: u32 = 0x0059_e530;
pub const BUFFER_VISITOR_VTABLE: u32 = 0x00b1_fac0;

/// The context-switch factory's callees and free lists.
pub const FIND_BY_NAME: u32 = 0x004d_37e0;
pub const PRINTF: u32 = 0x0043_52f0;
pub const NEW_SWITCH_FORMAT: u32 = 0x00b1_fa50;
/// Free lists `(head, block size)`: the switches and their names.
pub const SWITCH_LIST: (u32, u32) = (0x00ca_4edc, 0x00ca_4ee4);
pub const NAME_LIST: (u32, u32) = (0x00ca_4ef4, 0x00ca_4efc);
pub const ALLOCATE_BLOCK: u32 = 0x0049_c8d0;
pub const MEMCPY: u32 = 0x0094_9256;
pub const SWITCH_CTOR: u32 = 0x004d_2e50;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ALLOCATE_BLOCK, ATEXIT, BUFFER_VISITOR_VTABLE, Conv, Dtor, ELEMENT40_DTOR,
        ENTRY6C_MEMBER_CTOR, ENTRY6C_MEMBER_DTOR, EVENT_SIGNAL, EXTERNAL, FIND_BY_NAME, FREE,
        GLOBAL_NAMESPACE, Host, INTERFACE, INTERFACE_ATEXIT, INTERFACE_BIND, INTERFACE_CTOR,
        INTERFACE_GUARD, INTERFACE_UNBIND, INTERLOCKED_CAS, LOCK, LOCK_ATEXIT, LOCK_CTOR,
        LOCK_DEPTH, LOCK_GUARD, LOCK_RELEASE, LOCK_TAKE, MALLOC, MEMCPY, Mem, NAME_LIST,
        NESTED_FIND, NESTED_VISIT, NEW_SWITCH_FORMAT, PARAMETER_BUFFER_HELPER, PASS_UNBIND, PRINTF,
        RELEASE_TAGGED_16, RELEASE_TAGGED_WORDS, Rel, Resize, SWITCH_CTOR, SWITCH_LIST, THREAD_ID,
        WORLD_CLASS_COUNT,
    };

    /// One array of `this` released, then its data and count words cleared; `eax`
    /// as the original leaves it: the callee's, 0 for a null block, or
    /// 0xfe when the block is not owned.
    pub fn release(h: &mut impl Host, this: u32, count_at: u32, data_at: u32, how: Rel) -> u32 {
        let count: u32 = h.read(this + count_at);
        let eax = if count & EXTERNAL == 0 {
            let data: u32 = h.read(this + data_at);
            match how {
                Rel::Free if data == 0 => 0,
                // `not al` of 0 left in `eax` when the element is inline
                Rel::FreeShared if count & !EXTERNAL <= 1 => 0xff,
                Rel::Free | Rel::FreeShared => h.call_sym(FREE, Conv::Cdecl, &[data]),
                Rel::Elements(f) => h.call_sym(f, Conv::Cdecl, &[data, count & !EXTERNAL]),
            }
        } else {
            0xfe
        };
        h.write(this + data_at, 0u32);
        h.write(this + count_at, 0u32);
        eax
    }

    /// The arrays `(count offset, data offset, how)` of `this` released in
    /// order, as [`release`]; the last one's `eax` (0 for none).
    pub fn release_all(h: &mut impl Host, this: u32, arrays: &[(u32, u32, Rel)]) -> u32 {
        let mut eax = 0;
        for &(count, data, how) in arrays {
            eax = release(h, this, count, data, how);
        }
        eax
    }

    /// A destructor of [`super::DTORS`] (thiscall on `this`): the vtable,
    /// the arrays in order, then the tail call; `eax` = the last call's
    /// (or the last array's, as [`release`]).
    pub fn dtor(h: &mut impl Host, k: &Dtor, this: u32) -> u32 {
        if k.vtable != 0 {
            let vtable = h.rebase(k.vtable);
            h.write(this, vtable);
        }
        let mut eax = release_all(h, this, &k.arrays);
        if k.tail != 0 {
            eax = h.call_sym(k.tail, Conv::Thiscall(this + 4), &[]);
        }
        eax
    }

    /// `0x004a8910(this)` (thiscall): `+0x10` = 0; the tagged block at
    /// `+4` freed unless bit 0 is set, `+4` = 0; the array at `+8`/`+0xc`
    /// released (`PArray_releaseTaggedWords`); then `+4` again, now 0:
    /// `PhyreFree(0)`, whose `eax` is returned.
    pub fn tagged_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x10, 0u32);
        let block: u32 = h.read(this + 4);
        if block & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        h.write(this + 4, 0u32);
        let mut eax = release(h, this, 8, 0xc, Rel::Elements(RELEASE_TAGGED_WORDS));
        let block: u32 = h.read(this + 4);
        if block & 1 == 0 {
            eax = h.call_sym(FREE, Conv::Cdecl, &[block]);
        } else {
            // `mov al, [this+4]; not al` left in `eax`
            eax = (eax & !0xff) | (!block & 0xff);
        }
        eax
    }

    /// The default fields of a 0x6c-byte entry, without its `+0x30`
    /// member.
    fn entry6c_fields(m: &mut impl Mem, e: u32) {
        m.write(e, 0u32);
        m.write(e + 4, u32::MAX);
        m.write(e + 8, 0u32);
        m.write(e + 0xc, 0u32);
        m.write(e + 0x10, 6u32);
        m.write(e + 0x14, 0u8);
        m.write(e + 0x18, 0u32);
        m.write(e + 0x1c, 0u32);
        m.write(e + 0x20, u32::MAX);
        m.write(e + 0x24, u32::MAX);
        m.write(e + 0x28, 0u32);
        m.write(e + 0x2c, 0u32);
    }

    /// `0x004337d0(this)` (thiscall) -> `this`: a 0x6c-byte entry's
    /// defaults, then its `+0x30` member's constructor.
    pub fn entry6c_ctor(h: &mut impl Host, this: u32) -> u32 {
        entry6c_fields(h, this);
        h.call_sym(ENTRY6C_MEMBER_CTOR, Conv::Thiscall(this + 0x30), &[]);
        this
    }

    /// `0x00434430(data, n)` (cdecl): `n` entries as [`entry6c_ctor`]
    /// (each unless its address is 0). The second result: the last value
    /// of the `n` argument slot (the last entry's address), when `n` is
    /// not 0.
    pub fn entry6c_construct(h: &mut impl Host, data: u32, n: u32) -> Option<u32> {
        let mut slot = None;
        for i in 0..n {
            let e = i.wrapping_mul(0x6c).wrapping_add(data);
            slot = Some(e);
            if e != 0 {
                entry6c_fields(h, e);
                h.call_sym(ENTRY6C_MEMBER_CTOR, Conv::Thiscall(e + 0x30), &[]);
            }
        }
        slot
    }

    /// `0x00434210(data, n)` (cdecl) -> `PhyreFree`'s `eax` (0 for a null
    /// `data`): each of the `n` 0x6c-byte entries has its `+0x30` member
    /// destroyed, its 0x40-byte element array at `+0x28`/`+0x2c` and its
    /// block at `+8`/`+0xc` released; then `data` is freed. The `n`
    /// argument slot ends at 0 when `n` is not 0.
    pub fn entry6c_destroy(h: &mut impl Host, data: u32, n: u32) -> u32 {
        if data == 0 {
            return 0;
        }
        for i in 0..n {
            let e = data.wrapping_add(i.wrapping_mul(0x6c));
            h.call_sym(ENTRY6C_MEMBER_DTOR, Conv::Thiscall(e + 0x30), &[]);
            let count: u32 = h.read(e + 0x28);
            if count & EXTERNAL == 0 {
                let block: u32 = h.read(e + 0x2c);
                if block != 0 {
                    for k in 0..count & !EXTERNAL {
                        h.call_sym(ELEMENT40_DTOR, Conv::Thiscall(block + 0x40 * k), &[]);
                    }
                    h.call_sym(FREE, Conv::Cdecl, &[block]);
                }
            }
            h.write(e + 0x2c, 0u32);
            h.write(e + 0x28, 0u32);
            let count: u32 = h.read(e + 8);
            if count & EXTERNAL == 0 {
                let block: u32 = h.read(e + 0xc);
                if block != 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[block]);
                }
            }
            h.write(e + 0xc, 0u32);
            h.write(e + 8, 0u32);
        }
        h.call_sym(FREE, Conv::Cdecl, &[data])
    }

    /// `0x004d6180(this, p)` (thiscall, `ret 4`) -> `this`: `+4` = `p`,
    /// `+0` = `p`'s u16 at `+0x30`; then the array `+8`/`+0xc` (both 0
    /// first) takes `p`'s data `+0x24` as external with `p`'s count
    /// `+0x20`, releasing an owned old block other than it.
    pub fn view_ctor(h: &mut impl Host, this: u32, p: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, p);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        let kind: u16 = h.read(p + 0x30);
        h.write(this, u32::from(kind));
        let p: u32 = h.read(this + 4);
        let count = h.read::<u32>(p + 0x20) & !EXTERNAL;
        let data: u32 = h.read(p + 0x24);
        let old: u32 = h.read(this + 0xc);
        if old != data {
            let old_count: u32 = h.read(this + 8);
            if old_count & EXTERNAL == 0 {
                h.call_sym(
                    RELEASE_TAGGED_16,
                    Conv::Cdecl,
                    &[old, old_count & !EXTERNAL],
                );
            }
        }
        h.write(this + 0xc, data);
        h.write(this + 8, count | EXTERNAL);
        this
    }

    /// `0x004319d0(lock)` (thiscall): a recursive lock's release. When
    /// the current thread owns it (`+0x10`) and the depth (`+0x14`) drops
    /// to 0, `InterlockedCompareExchange(+0x10, 0, thread)`; then, while
    /// waiters (`+0xc`, signed) are positive, its event (`+4`) signalled.
    /// `eax`: the last call's, else the thread id.
    pub fn lock_release(h: &mut impl Host, this: u32) -> u32 {
        let owner = this + 0x10;
        let mut eax = h.call_sym(THREAD_ID, Conv::Cdecl, &[]);
        if h.read::<u32>(owner) == eax {
            let depth = h.read::<u32>(owner + 4).wrapping_sub(1);
            h.write(owner + 4, depth);
            if depth == 0 {
                let cas: u32 = h.read(h.rebase(INTERLOCKED_CAS));
                eax = h.call(cas, Conv::Stdcall, &[owner, 0, eax]);
            }
        }
        if h.read::<i32>(this + 0xc) > 0 {
            eax = h.call_sym(EVENT_SIGNAL, Conv::Thiscall(this + 4), &[]);
        }
        eax
    }

    /// A guarded static: constructed once (bit 0 of `guard`) with its
    /// destructor registered.
    fn static_once(h: &mut impl Host, guard: u32, obj: u32, ctor: u32, atexit: u32) {
        let guard = h.rebase(guard);
        let flags: u32 = h.read(guard);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            let obj = h.rebase(obj);
            h.call_sym(ctor, Conv::Thiscall(obj), &[]);
            let f = h.rebase(atexit);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
    }

    /// The render interface (made once) bound with `f`, when the lock's
    /// depth is 1.
    fn interface_if_outermost(h: &mut impl Host, f: u32) {
        static_once(h, LOCK_GUARD, LOCK, LOCK_CTOR, LOCK_ATEXIT);
        let lock = h.rebase(LOCK);
        if h.call_sym(LOCK_DEPTH, Conv::Thiscall(lock), &[]) == 1 {
            static_once(
                h,
                INTERFACE_GUARD,
                INTERFACE,
                INTERFACE_CTOR,
                INTERFACE_ATEXIT,
            );
            let interface = h.rebase(INTERFACE);
            h.call_sym(f, Conv::Thiscall(interface), &[]);
        }
    }

    /// `0x00463d50(this)` (thiscall) -> `this`: the render lock (made
    /// once) stored in `+0` and taken; the outermost taker binds the
    /// render interface to its thread.
    pub fn render_scope_enter(h: &mut impl Host, this: u32) -> u32 {
        static_once(h, LOCK_GUARD, LOCK, LOCK_CTOR, LOCK_ATEXIT);
        let lock = h.rebase(LOCK);
        h.write(this, lock);
        h.call_sym(LOCK_TAKE, Conv::Thiscall(lock), &[]);
        interface_if_outermost(h, INTERFACE_BIND);
        this
    }

    /// `0x00463e90(this)` (thiscall): the outermost holder unbinds the
    /// render interface; then the lock in `+0` is released; its `eax`.
    pub fn render_scope_leave(h: &mut impl Host, this: u32) -> u32 {
        interface_if_outermost(h, INTERFACE_UNBIND);
        let lock: u32 = h.read(this);
        h.call_sym(LOCK_RELEASE, Conv::Thiscall(lock), &[])
    }

    /// An array resize of [`super::RESIZES`] (thiscall on the array: `+0`
    /// count with the "not owned" flag, `+4` data; `ret 4`): nothing when
    /// `n` equals the count word. Else a new block of `n` elements (none
    /// for 0; `eax` = 0xd when the allocation fails), the first
    /// `min(n, count)` copied and the rest default-constructed, the old
    /// block freed when owned, non-null and other than the new one; the
    /// array takes the block and `n`. `eax` = 0.
    ///
    /// The second result: what the original leaves in its `n` argument
    /// slot (the new block), when it gets that far.
    pub fn resize(h: &mut impl Host, k: &Resize, this: u32, n: u32) -> (u32, Option<u32>) {
        let word: u32 = h.read(this);
        if n == word {
            return (0, None);
        }
        let keep = (word & !EXTERNAL).min(n);
        let old: u32 = h.read(this + 4);
        let mut block = 0;
        if n != 0 {
            block = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(k.size), 4]);
            if block == 0 {
                return (0xd, Some(0));
            }
            h.call_sym(k.copy, Conv::Cdecl, &[block, old, keep]);
            let rest = block.wrapping_add(keep.wrapping_mul(k.size));
            h.call_sym(k.init, Conv::Cdecl, &[rest, n - keep]);
        }
        let old: u32 = h.read(this + 4);
        let word: u32 = h.read(this);
        if old != block && word & EXTERNAL == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, block);
        h.write(this, n);
        (0, Some(block))
    }

    /// `PWorld::initialize(this)` (thiscall) -> 0, or the resize's error:
    /// the class entry list (`+8`, 20-byte entries) resized to one more
    /// than [`WORLD_CLASS_COUNT`] ([`resize`] of `0x00443d20`), entries
    /// 1.. linked before entry 0; then every class of the global
    /// namespace's list (`+8`, linked at `+0x2c`) with an index (`+0x80`,
    /// non-negative) gets its depth (the number of `+0x40` parent links)
    /// in its entry's `+0x10`.
    pub fn world_initialize(h: &mut impl Host, k: &Resize, this: u32) -> u32 {
        let count: u32 = h.read(h.rebase(WORLD_CLASS_COUNT));
        let (eax, _) = resize(h, k, this + 8, count.wrapping_add(1));
        if eax != 0 {
            return eax;
        }
        for i in 1..=count {
            let head: u32 = h.read(this + 0xc);
            let last: u32 = h.read(head + 4);
            let e = head.wrapping_add(0x14 * i);
            h.write(e, head);
            h.write(e + 4, last);
            h.write(last, e);
            h.write(head + 4, e);
        }
        let namespace = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        let end = namespace + 8;
        let mut node: u32 = h.read(end);
        while node != end && node != 0 && node != 0x2c {
            let class = node - 0x2c;
            let index: u32 = h.read(class + 0x80);
            if index & EXTERNAL == 0 {
                let mut depth = 0u32;
                let mut parent: u32 = h.read(class + 0x40);
                while parent != 0 {
                    parent = h.read(parent + 0x40);
                    depth = depth.wrapping_add(1);
                }
                let entries: u32 = h.read(this + 0xc);
                h.write(entries.wrapping_add(index.wrapping_mul(20)) + 0x10, depth);
            }
            node = h.read(class + 0x2c);
        }
        0
    }

    /// `destroyInstance(descriptor, p)` of `PParameterBuffer`
    /// (`0x004d7190`, thiscall, `ret 4`): a visitor in `frame` (the
    /// original's `ebp-0x14`: vtable, `0x0059e530()`'s result) handed
    /// with `p` to the descriptor's virtual method 15; then `p`'s array
    /// `+8`/`+0xc` released when owned (`PArray_releaseTaggedElements`) and
    /// cleared. `al` = 1, over the release's `eax`.
    pub fn destroy_parameter_buffer(h: &mut impl Host, frame: u32, this: u32, p: u32) -> u32 {
        let helper = h.call_sym(PARAMETER_BUFFER_HELPER, Conv::Cdecl, &[]);
        h.write(frame + 4, helper);
        let vtable = h.rebase(BUFFER_VISITOR_VTABLE);
        h.write(frame, vtable);
        h.call_virtual(this, 15, &[p, frame]);
        let count: u32 = h.read(p + 8);
        let mut eax = 1;
        if count & EXTERNAL == 0 {
            let data: u32 = h.read(p + 0xc);
            eax = h.call_sym(RELEASE_TAGGED_16, Conv::Cdecl, &[data, count & !EXTERNAL]);
        }
        h.write(p + 0xc, 0u32);
        h.write(p + 8, 0u32);
        (eax & !0xff) | 1
    }

    /// `0x004ba540(this, a, b)` (thiscall, `ret 8`): the shader pass's
    /// resource tables unbound: `(a, b, slot)` with the first two callees
    /// for slots 0..3, then `(a, b)`, `(a)`, `(a)`; stops at the first
    /// non-zero `eax`, which it returns.
    pub fn unbind_pass(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let [one, two, pair, single, last] = PASS_UNBIND;
        for slot in 0..4 {
            for f in [one, two] {
                let eax = h.call_sym(f, Conv::Thiscall(this), &[a, b, slot]);
                if eax != 0 {
                    return eax;
                }
            }
        }
        let eax = h.call_sym(pair, Conv::Thiscall(this), &[a, b]);
        if eax != 0 {
            return eax;
        }
        let eax = h.call_sym(single, Conv::Thiscall(this), &[a]);
        if eax != 0 {
            return eax;
        }
        h.call_sym(last, Conv::Thiscall(this), &[a])
    }

    /// `PFindNestedClassVisitor` slot 1 (`0x0047fa10`, thiscall `(this,
    /// base, count, object, flags)`, `ret 0x10`): unless the visitor has
    /// found something (`+0x18`) or `flags` bit 1 is set, the object's
    /// class (virtual method 1); the visitor's target (`+0xc` + `+0x10`)
    /// relative to `base` plus the class's offset (`+0x8c`), or, when the
    /// class has an entry in the visitor's `+8` table, that entry's
    /// offset (`+0x18`) and element size (`+4`, low 28 bits; else the
    /// class's `+0x1c`). Inside `count` elements, `+0x1c` = the entry
    /// visit's `eax` and `+0x18` = the offset it leaves in `slot` (the
    /// original's `flags` argument slot, passed by address).
    pub fn visit_nested(
        h: &mut impl Host,
        slot: u32,
        this: u32,
        base: u32,
        count: u32,
        object: u32,
        flags: u32,
    ) {
        if h.read::<u32>(this + 0x18) != 0 || flags & 2 != 0 {
            return;
        }
        let class = h.call_virtual(object, 1, &[]);
        if class == 0 {
            return;
        }
        let table: u32 = h.read(this + 8);
        let entry = h.call_sym(NESTED_FIND, Conv::Thiscall(table), &[class]);
        let info = if entry == 0 { 0 } else { h.read(entry + 4) };
        let class_offset: u32 = h.read(class + 0x8c);
        let mut offset = base.wrapping_add(class_offset);
        h.write(slot, offset);
        let target = h
            .read::<u32>(this + 0x10)
            .wrapping_add(h.read(this + 0xc))
            .wrapping_sub(offset);
        let start = if info == 0 {
            class_offset
        } else {
            h.read(info + 0x18)
        };
        let class_size: u32 = h.read(class + 0x1c);
        let size = if info == 0 {
            class_size
        } else {
            h.read::<u32>(info + 4) & 0x0fff_ffff
        };
        let at = start.wrapping_add(target) as i32;
        if at < 0 || at >= size.wrapping_mul(count) as i32 {
            return;
        }
        // `size` is not 0 here: 0 elements fit nothing
        let (Some(index), Some(rem)) = (target.checked_div(size), target.checked_rem(size)) else {
            return;
        };
        let x: u32 = h.read(this + 0x14);
        let table: u32 = h.read(this + 8);
        offset = offset.wrapping_add(index.wrapping_mul(class_size));
        h.write(slot, offset);
        let eax = h.call_sym(NESTED_VISIT, Conv::Thiscall(table), &[entry, slot, rem, x]);
        h.write(this + 0x1c, eax);
        let offset: u32 = h.read(slot);
        h.write(this + 0x18, offset);
    }

    /// One block popped off the free list `(head, size)`, a new block
    /// allocated first when it is empty; 0 when none.
    fn pop_free(h: &mut impl Host, list: (u32, u32)) -> u32 {
        let head = h.rebase(list.0);
        let mut block: u32 = h.read(head);
        if block == 0 {
            let size: u32 = h.read(h.rebase(list.1));
            h.call_sym(ALLOCATE_BLOCK, Conv::Thiscall(head), &[size]);
            block = h.read(head);
            if block == 0 {
                return 0;
            }
        }
        let next: u32 = h.read(block);
        h.write(head, next);
        block
    }

    /// `0x004d3830(name)` (cdecl): the context switch of that name
    /// (`PhyreFindByName`), or, for a new name, a switch made from the
    /// free lists after a `PhyrePrintf(1, ...)`: the name copied with its
    /// terminator, `0x004d2e50(switch, 0, copy)`'s `eax`. A missing block
    /// gives the switch block itself (or 0).
    pub fn switch_for_name(h: &mut impl Host, name: u32) -> u32 {
        let found = h.call_sym(FIND_BY_NAME, Conv::Cdecl, &[name]);
        if name == 0 || found != 0 {
            return found;
        }
        let format = h.rebase(NEW_SWITCH_FORMAT);
        h.call_sym(PRINTF, Conv::Cdecl, &[1, format, name]);
        let switch = pop_free(h, SWITCH_LIST);
        let copy = pop_free(h, NAME_LIST);
        if switch == 0 || copy == 0 {
            return switch;
        }
        let mut len = 0u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len + 1]);
        h.call_sym(SWITCH_CTOR, Conv::Thiscall(switch), &[0, copy])
    }
}
