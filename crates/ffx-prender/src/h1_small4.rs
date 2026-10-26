//! `PhyreEngine` render methods, round 3 batch 3 (M4 port, h1-render-a):
//! the geometry and rendering `initialize` sequences, two command-ring
//! submits, two more destructors, the texture format description and the
//! parameter buffers' resource set-up.

use ffx_graphic::host::{Conv, Host};

use crate::basic::FREE;
use crate::class_slots::REGISTER_TYPE;
use crate::h1_small3::{RELEASE_TAGGED_WORDS, Rel};

/// One call of an `initialize` sequence (cdecl, its `ffx.sym` address).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `f()`.
    Call(u32),
    /// `RegisterType(f())`.
    Register(u32),
    /// `f(arg)`, `arg` an image address.
    CallWith(u32, u32),
}

/// `PUtilityGeometry::initialize`'s calls (`0x004322c0`, read from its
/// machine code): the `Bind` functions and type getters, then the 26
/// semantic registrations (`0x0047fdb0` of each descriptor).
pub const GEOMETRY_STEPS: &[Step] = &[
    Step::Call(0x0048_1f80),
    Step::Call(0x0048_5470),
    Step::Call(0x0048_1f60),
    Step::Call(0x0048_5450),
    Step::Call(0x0048_4300),
    Step::Call(0x0048_aa00),
    Step::Call(0x0049_a180),
    Step::Call(0x0049_07c0),
    Step::Call(0x0049_0250),
    Step::Call(0x0048_7ca0),
    Step::Call(0x0048_7990),
    Step::Call(0x0048_7970),
    Step::Call(0x0048_0510),
    Step::Register(0x0047_fae0),
    Step::Call(0x0049_70e0),
    Step::Call(0x0049_6ee0),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85b4),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85c0),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85cc),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85d8),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85e4),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85f0),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_85fc),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8608),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8614),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8620),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_862c),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8644),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8638),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8650),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_865c),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8668),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8674),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8680),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_868c),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_8698),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86a4),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86b0),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86c8),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86d4),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86e0),
    Step::CallWith(SEMANTIC_REGISTER, 0x00c9_86ec),
];

/// `0x0047fdb0(descriptor)` (cdecl): a vertex semantic's registration.
pub const SEMANTIC_REGISTER: u32 = 0x0047_fdb0;

/// `PUtilityRendering::initialize`'s calls after its three start-up
/// objects (`0x00432862`..`0x00432a0e`, read from its machine code).
pub const RENDERING_STEPS: &[Step] = &[
    Step::Call(0x004d_65b0),
    Step::Call(0x004d_6470),
    Step::Call(0x0049_fe10),
    Step::Call(0x0049_fdf0),
    Step::Call(0x004f_d620),
    Step::Call(0x004d_7e90),
    Step::Call(0x004d_80a0),
    Step::Call(0x004c_f7c0),
    Step::Call(0x004e_b2d0),
    Step::Call(0x004d_36e0),
    Step::Call(0x004f_4800),
    Step::Call(0x004f_38a0),
    Step::Call(0x0050_1110),
    Step::Call(0x004e_4270),
    Step::Call(0x004d_ca40),
    Step::Register(0x004d_9950),
    Step::Call(0x004d_cdd0),
    Step::Call(0x004d_c870),
    Step::Register(0x004d_0e20),
    Step::Call(0x004b_6350),
    Step::Call(0x004b_6870),
    Step::Call(0x004b_6330),
    Step::Call(0x004b_c360),
    Step::Register(0x004c_0b10),
    Step::Call(0x004c_1fb0),
    Step::Call(0x004c_3400),
    Step::Call(0x004c_33e0),
    Step::Call(0x004c_4c40),
    Step::Call(0x004c_4c20),
    Step::Call(0x004c_6500),
    Step::Call(0x004c_64e0),
    Step::Call(0x004b_ed60),
    Step::Call(0x004b_ed40),
    Step::Call(0x004a_1b40),
    Step::Call(0x004a_1be0),
    Step::Call(0x004a_1c80),
    Step::Call(0x004a_1d20),
    Step::Call(0x004a_1e50),
    Step::Call(0x004b_6680),
    Step::Call(0x004b_6580),
    Step::Call(0x004b_6770),
    Step::Call(0x004a_a3f0),
    Step::Call(0x004a_a630),
    Step::Call(0x004a_a4f0),
    Step::Call(0x004a_a590),
    Step::Call(0x004a_a760),
    Step::Call(0x004a_a350),
    Step::Call(0x004a_a410),
    Step::Call(0x004a_a130),
    Step::Call(0x004a_9de0),
    Step::Call(0x004a_a000),
    Step::Call(0x004a_a800),
    Step::Call(0x004a_9ec0),
    Step::Call(0x004a_9f60),
    Step::Call(0x004a_a210),
    Step::Call(0x004a_a2b0),
    Step::Call(0x004a_aa70),
    Step::Call(0x004a_a8c0),
    Step::Call(0x004a_a8a0),
    Step::Call(0x004a_ae60),
    Step::Call(0x004a_9da0),
    Step::Call(0x004a_9dc0),
    Step::Call(0x004a_9d80),
    Step::Call(0x004a_ac50),
    Step::Call(0x004f_a540),
    Step::Call(0x004f_b8c0),
    Step::Call(0x004f_fa30),
    Step::Call(0x004c_7b70),
    Step::Call(0x004c_7b50),
    Step::Register(0x004c_8a70),
    Step::Call(0x004c_ad80),
    Step::Call(0x004c_aba0),
    Step::Call(0x0049_d8c0),
    Step::Register(0x0049_d050),
    Step::Call(0x0050_2100),
    Step::Call(0x0050_20e0),
    Step::Call(0x0050_34b0),
    Step::Call(0x0050_3490),
    Step::Call(0x004f_4700),
];

/// `PUtilityRendering::initialize`'s start-up objects (virtual method 1
/// each, while the byte [`STARTED`] is clear).
pub const STARTUP_OBJECTS: [u32; 3] = [0x00c0_a33c, 0x00c1_a300, 0x00c1_a350];
/// The byte that stops the start-up objects.
pub const STARTED: u32 = 0x00c0_a09c;
/// The rendering initialize's last steps, each while `eax` stays 0: the
/// texture formats' set-up, `0x004fd9e0(0x20)`, `0x004ff020()`, and
/// (reached by a tail jump) `0x004e62c0()`.
pub const TEXTURE_FORMATS_INIT: u32 = 0x004c_1050;
pub const CONTEXT_INIT: u32 = 0x004f_d9e0;
pub const CONTEXT_INIT_ARG: u32 = 0x20;
pub const DEVICE_INIT: u32 = 0x004f_f020;
pub const LAST_INIT: u32 = 0x004e_62c0;

/// A command ring's submit (`0x00434c50` and `0x00434cd0`): thiscall
/// callees on the ring, its field offsets and the state byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ring {
    /// `al` non-zero: no preparation.
    pub ready: u32,
    /// The preparation: a non-zero `eax` is returned.
    pub prepare: u32,
    /// The write index (`+index` + 1, wrapping at the count).
    pub index: u32,
    /// The slot array's count word and data (a `PSharray`).
    pub count: u32,
    pub data: u32,
    /// `submit(slot, x)`.
    pub submit: u32,
    /// The byte set to 2 when the submit gives 0.
    pub state: u32,
}

/// The rings, by address.
pub const RINGS: &[(u32, Ring)] = &[
    (
        0x0043_4c50,
        Ring {
            ready: 0x0056_2da0,
            prepare: 0x0056_1a20,
            index: 0x10,
            count: 0x18,
            data: 0x1c,
            submit: 0x0056_3290,
            state: 0x14,
        },
    ),
    (
        0x0043_4cd0,
        Ring {
            ready: 0x0056_2db0,
            prepare: 0x0056_1b70,
            index: 0x10,
            count: 0x14,
            data: 0x18,
            submit: 0x0056_3900,
            state: 0xe,
        },
    ),
];

/// The ring `f` (compile time).
#[must_use]
pub const fn ring_of(f: u32) -> Ring {
    let mut i = 0;
    while i < RINGS.len() {
        if RINGS[i].0 == f {
            return RINGS[i].1;
        }
        i += 1;
    }
    panic!("not a ring of h1_small4")
}

/// `0x00430030(ms)` (cdecl): the wait while a slot is busy.
pub const SLEEP: u32 = 0x0043_0030;

/// `PMeshSegmentEntries_destroy_00434210(data, n)` (cdecl).
pub const DESTROY_ENTRIES: u32 = 0x0043_4210;

/// The mesh segment's destructor `0x00433af0`: its arrays, in order.
pub const SEGMENT_ARRAYS: [(u32, u32, Rel); 7] = [
    (0x30, 0x34, Rel::FreeShared),
    (0x28, 0x2c, Rel::Free),
    (0x20, 0x24, Rel::Elements(RELEASE_TAGGED_WORDS)),
    (0x18, 0x1c, Rel::Free),
    (0x10, 0x14, Rel::Free),
    (0x8, 0xc, Rel::Free),
    (0x0, 0x4, Rel::Elements(DESTROY_ENTRIES)),
];

/// The texture formats' element bits and flag bits 9..14 (`0x004c0c00`'s
/// jump table, formats 0..0x2e).
pub const FORMAT_BITS: [(u32, u32); 47] = [
    (0x8, 0xe00),
    (0x8, 0x1000),
    (0x10, 0x1e00),
    (0x10, 0x600),
    (0x10, 0xe00),
    (0x10, 0x1000),
    (0x20, 0x1e00),
    (0x20, 0x600),
    (0x10, 0x1e00),
    (0x10, 0x1e00),
    (0x10, 0xe00),
    (0x20, 0x1e00),
    (0x20, 0x1e00),
    (0x20, 0x1e00),
    (0x40, 0x1e00),
    (0x4, 0x1e00),
    (0x8, 0x1e00),
    (0x8, 0x1e00),
    (0x4, 0x200),
    (0x8, 0x1e00),
    (0x4, 0xe00),
    (0x8, 0x1e00),
    (0x8, 0x1e00),
    (0x0, 0x0),
    (0x0, 0x0),
    (0x0, 0x0),
    (0x0, 0x0),
    (0x4, 0xe00),
    (0x40, 0x1e00),
    (0x80, 0x1e00),
    (0x10, 0x200),
    (0x10, 0xe00),
    (0x20, 0x200),
    (0x20, 0xe00),
    (0x20, 0x600),
    (0x20, 0x1e00),
    (0x40, 0x600),
    (0x40, 0x1e00),
    (0x10, 0x2000),
    (0x20, 0x2000),
    (0x20, 0x6000),
    (0x20, 0x2000),
    (0x20, 0x1e00),
    (0x20, 0x1e00),
    (0xc, 0x1e00),
    (0x20, 0x1e00),
    (0x20, 0x1e00),
];
/// Formats with flag 1, 2 and 4.
pub const FORMAT_FLAG1: [u32; 9] = [0xf, 0x10, 0x11, 0x14, 0x15, 0x16, 0x12, 0x13, 0x1b];
pub const FORMAT_FLAG2: [u32; 4] = [0x26, 0x27, 0x28, 0x29];
pub const FORMAT_FLAG4: [u32; 10] = [0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x1c, 0x24, 0x25, 0x1d];

/// `0x0045cd60`'s callees and statics: the block allocator (cdecl
/// `(n, tag)`), the resource lock (`take`/`give`, thiscall), the resource
/// set (made once: six words pointing at itself), its find and insert
/// (thiscall, one word).
pub const BLOCK_ALLOC: u32 = 0x0045_d520;
pub const BLOCK_TAG: u32 = 0x7b5;
pub const RESOURCE_LOCK: u32 = 0x00c9_40c4;
pub const LOCK_TAKE: u32 = 0x0043_1920;
pub const LOCK_GIVE: u32 = 0x0043_1a80;
pub const RESOURCE_SET_GUARD: u32 = 0x00c9_4264;
pub const RESOURCE_SET: u32 = 0x00c9_424c;
pub const RESOURCE_SET_ATEXIT: u32 = 0x00af_af70;
pub const RESOURCE_SET_FIND: u32 = 0x0045_b9d0;
pub const RESOURCE_SET_INSERT: u32 = 0x0057_4980;
pub const ATEXIT: u32 = 0x0094_9207;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, BLOCK_ALLOC, BLOCK_TAG, CONTEXT_INIT, CONTEXT_INIT_ARG, Conv, DESTROY_ENTRIES,
        DEVICE_INIT, FORMAT_BITS, FORMAT_FLAG1, FORMAT_FLAG2, FORMAT_FLAG4, FREE, Host, LAST_INIT,
        LOCK_GIVE, LOCK_TAKE, REGISTER_TYPE, RELEASE_TAGGED_WORDS, RESOURCE_LOCK, RESOURCE_SET,
        RESOURCE_SET_ATEXIT, RESOURCE_SET_FIND, RESOURCE_SET_GUARD, RESOURCE_SET_INSERT, Rel, Ring,
        SEGMENT_ARRAYS, SLEEP, STARTED, STARTUP_OBJECTS, Step, TEXTURE_FORMATS_INIT,
    };
    use crate::h1_small3::bridge::{release, release_all};

    /// The steps in order.
    fn run(h: &mut impl Host, steps: &[Step]) {
        for &step in steps {
            match step {
                Step::Call(f) => {
                    h.call_sym(f, Conv::Cdecl, &[]);
                }
                Step::Register(f) => {
                    let t = h.call_sym(f, Conv::Cdecl, &[]);
                    h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[t]);
                }
                Step::CallWith(f, arg) => {
                    let arg = h.rebase(arg);
                    h.call_sym(f, Conv::Cdecl, &[arg]);
                }
            }
        }
    }

    /// `PUtilityGeometry::initialize()` (`0x004322c0`, cdecl) -> 0: the
    /// steps ([`super::GEOMETRY_STEPS`]).
    pub fn geometry_initialize(h: &mut impl Host, steps: &[Step]) -> u32 {
        run(h, steps);
        0
    }

    /// `PUtilityRendering::initialize()` (`0x00432820`, cdecl): the
    /// start-up objects' method 1 while [`STARTED`] stays 0, the steps
    /// ([`super::RENDERING_STEPS`]), then the first non-zero `eax` of the
    /// texture formats' set-up, `0x004fd9e0(0x20)`, `0x004ff020()` and
    /// `0x004e62c0()` (a tail jump in the original).
    pub fn rendering_initialize(h: &mut impl Host, steps: &[Step]) -> u32 {
        let started = h.rebase(STARTED);
        for obj in STARTUP_OBJECTS {
            if h.read::<u8>(started) != 0 {
                break;
            }
            let obj = h.rebase(obj);
            h.call_virtual(obj, 1, &[]);
        }
        run(h, steps);
        let eax = h.call_sym(TEXTURE_FORMATS_INIT, Conv::Cdecl, &[]);
        if eax != 0 {
            return eax;
        }
        let eax = h.call_sym(CONTEXT_INIT, Conv::Cdecl, &[CONTEXT_INIT_ARG]);
        if eax != 0 {
            return eax;
        }
        let eax = h.call_sym(DEVICE_INIT, Conv::Cdecl, &[]);
        if eax != 0 {
            return eax;
        }
        h.call_sym(LAST_INIT, Conv::Cdecl, &[])
    }

    /// A ring submit of [`super::RINGS`] (thiscall `(this, x)`, `ret 4`):
    /// unless `ready` (`al`) says so, `prepare`'s non-zero `eax` is
    /// returned; the next slot (index + 1, 0 at the count; 20 bytes each,
    /// inline for a single slot) is waited on while its `+0x10` is
    /// positive (`0x00430030(1)`), then `submit(slot, x)`; a 0 result sets
    /// the state byte to 2. `eax` = the submit's.
    pub fn ring_submit(h: &mut impl Host, k: &Ring, this: u32, x: u32) -> u32 {
        let ready = h.call_sym(k.ready, Conv::Thiscall(this), &[]) as u8;
        if ready == 0 {
            let eax = h.call_sym(k.prepare, Conv::Thiscall(this), &[]);
            if eax != 0 {
                return eax;
            }
        }
        let next = h.read::<u32>(this + k.index).wrapping_add(1);
        let count = h.read::<u32>(this + k.count) & 0x7fff_ffff;
        let slot = if next == count { 0 } else { next };
        let base = if count > 1 {
            h.read(this + k.data)
        } else if count != 0 {
            this + k.data
        } else {
            0
        };
        let busy = base.wrapping_add(slot.wrapping_mul(20)).wrapping_add(0x10);
        while h.read::<i32>(busy) > 0 {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
        }
        let eax = h.call_sym(k.submit, Conv::Thiscall(this), &[slot, x]);
        if eax == 0 {
            h.write(this + k.state, 2u8);
        }
        eax
    }

    /// `0x00433af0(this)` (thiscall): the mesh segment's arrays released
    /// ([`SEGMENT_ARRAYS`]); the last release's `eax`.
    pub fn segment_dtor(h: &mut impl Host, this: u32) -> u32 {
        release_all(h, this, &SEGMENT_ARRAYS)
    }

    /// A release gated on a non-zero count word: the block (non-null,
    /// owned) released as `how`, then both words cleared.
    fn release_if_counted(h: &mut impl Host, this: u32, count_at: u32, data_at: u32, how: Rel) {
        let count: u32 = h.read(this + count_at);
        if count == 0 {
            return;
        }
        let data: u32 = h.read(this + data_at);
        if data != 0 && count & 0x8000_0000 == 0 {
            release(h, this, count_at, data_at, how);
        }
        h.write(this + data_at, 0u32);
        h.write(this + count_at, 0u32);
    }

    /// `0x00495a10(this)` (thiscall): five arrays released when counted
    /// (entries `+0`/`+4` by `PMeshSegmentEntries_destroy`, blocks `+8`,
    /// `+0x18`, `+0x28`, tagged words `+0x20`), then the `PSharray`
    /// `+0x30` (its single element inline at `+0x34`): an owned outside
    /// block freed, the count cleared. `eax`: 0 without a count, the
    /// count when the element is inline, 0xfe when not owned, else
    /// `PhyreFree`'s.
    pub fn arrays_dtor(h: &mut impl Host, this: u32) -> u32 {
        release_if_counted(h, this, 0, 4, Rel::Elements(DESTROY_ENTRIES));
        release_if_counted(h, this, 8, 0xc, Rel::Free);
        release_if_counted(h, this, 0x18, 0x1c, Rel::Free);
        release_if_counted(h, this, 0x20, 0x24, Rel::Elements(RELEASE_TAGGED_WORDS));
        release_if_counted(h, this, 0x28, 0x2c, Rel::Free);
        let count: u32 = h.read(this + 0x30);
        if count == 0 {
            return 0;
        }
        let n = count & 0x7fff_ffff;
        let inline = this + 0x34;
        let block = if n > 1 {
            h.read(inline)
        } else if n != 0 {
            inline
        } else {
            0
        };
        let eax = if block == inline {
            count
        } else if count & 0x8000_0000 != 0 {
            0xfe
        } else {
            h.call_sym(FREE, Conv::Cdecl, &[block])
        };
        h.write(this + 0x30, 0u32);
        eax
    }

    /// `0x004c0c00(this, a, format, flags)` (thiscall, `ret 0xc`) ->
    /// `this`: `[a, 0, format, bits, flags']` with `bits` and the flag
    /// bits 9..14 from [`FORMAT_BITS`] (0 past 0x2e), `flags'` = `flags`
    /// with bits 0..2 and 9..14 replaced, plus bit 0, 1 or 2 for the
    /// formats of [`FORMAT_FLAG1`], [`FORMAT_FLAG2`], [`FORMAT_FLAG4`].
    pub fn format_desc(h: &mut impl Host, this: u32, a: u32, format: u32, flags: u32) -> u32 {
        h.write(this, a);
        h.write(this + 4, 0u32);
        h.write(this + 8, format);
        let (bits, high) = FORMAT_BITS.get(format as usize).copied().unwrap_or((0, 0));
        h.write(this + 0xc, bits);
        let mut f = (flags & 0xffff_81f8) | high;
        if FORMAT_FLAG1.contains(&format) {
            f |= 1;
        }
        if FORMAT_FLAG2.contains(&format) {
            f |= 2;
        }
        if FORMAT_FLAG4.contains(&format) {
            f |= 4;
        }
        h.write(this + 0x10, f);
        this
    }

    /// `0x0045cd60(buffer)` (cdecl) -> 0, 0x12 (a resource already in
    /// the set) or 0x15 (an entry without units): each of the buffer's
    /// `+0x3c` entries (from `+0x30`, stride `[[+0x2c]+0x1c]`) gets
    /// `+0x24` = the buffer's `+0x28`, its list head `+4` emptied, a block
    /// of its `+0x18` (rounded down to even) units (`0x0045d520(n,
    /// 0x7b5)`) in `+0x10`/`+0x14`, and is added to the resource set
    /// (made once) under the resource lock.
    pub fn register_resources(h: &mut impl Host, buffer: u32) -> u32 {
        let layout: u32 = h.read(buffer + 0x2c);
        let count: u32 = h.read(buffer + 0x3c);
        let stride: u32 = h.read(layout + 0x1c);
        let mut e: u32 = h.read(buffer + 0x30);
        let value: u32 = h.read(buffer + 0x28);
        let lock = h.rebase(RESOURCE_LOCK);
        let set = h.rebase(RESOURCE_SET);
        for _ in 0..count {
            h.write(e + 0x24, value);
            let units: u32 = h.read(e + 0x18);
            let n = units - (units & 1);
            if n == 0 {
                return 0x15;
            }
            let head = e + 4;
            h.write(head, head);
            h.write(head + 4, head);
            h.write(head + 8, head);
            let block = h.call_sym(BLOCK_ALLOC, Conv::Cdecl, &[n, BLOCK_TAG]);
            h.write(e + 0x10, block);
            h.write(e + 0x14, n);
            h.call_sym(LOCK_TAKE, Conv::Thiscall(lock), &[]);
            let guard = h.rebase(RESOURCE_SET_GUARD);
            let flags: u32 = h.read(guard);
            if flags & 1 == 0 {
                h.write(guard, flags | 1);
                for k in 0..6 {
                    h.write(set + 4 * k, set);
                }
                let f = h.rebase(RESOURCE_SET_ATEXIT);
                h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
            }
            let found = h.call_sym(RESOURCE_SET_FIND, Conv::Thiscall(set), &[e]);
            if found == 0 {
                h.call_sym(RESOURCE_SET_INSERT, Conv::Thiscall(set), &[head]);
            }
            h.call_sym(LOCK_GIVE, Conv::Thiscall(lock), &[]);
            if found != 0 {
                return 0x12;
            }
            e = e.wrapping_add(stride);
        }
        0
    }
}
