//! List `p5-core`, continuation round: the thread pools' and the
//! preprocess ring buffer's singletons, two `PClassDescriptorAbstract<T>`
//! constructors, `PShadowCaster`'s constructor and descriptor callbacks,
//! `PInputAction`'s destructor.
//!
//! The class registrations of this list (`PString::Bind`, the
//! `PSharray<T>` / `PArray<T *>` declarations) need no code here: their
//! tables run on h1's, h2's and p2's interpreters (the shims).
//!
//! The originals' SEH frames only unwind half-built statics; dropped.

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The CRT's `atexit(f)` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;

/// `PThreadPool::GetSingleton`'s pointer, the worker count and the
/// second constructor word it reads, the work-stealing pool (static, its
/// guard word, constructor `(this; count, word)` and `atexit`
/// destructor).
pub const POOL: u32 = 0x00c9_1ba8;
pub const WORKERS: u32 = 0x00c0_b950;
pub const WORKERS_ARG: u32 = 0x00c0_b95c;
pub const STEALING: u32 = 0x00c9_1bb0;
pub const STEALING_GUARD: u32 = 0x00c9_1c10;
pub const STEALING_CTOR: u32 = 0x0057_d100;
pub const STEALING_DTOR: u32 = 0x00af_a590;
/// `PThreadPoolImmediate::GetSingleton` (cdecl `()`).
pub const IMMEDIATE_GET: u32 = 0x0057_cf50;
/// Its pointer, static, guard, `PThreadPool` constructor `(this; 0, 0)`,
/// vtable and `atexit` destructor.
pub const IMMEDIATE: u32 = 0x00cb_3424;
pub const IMMEDIATE_STATIC: u32 = 0x00cb_3428;
pub const IMMEDIATE_GUARD: u32 = 0x00cb_3444;
pub const THREAD_POOL_CTOR: u32 = 0x0044_cbe0;
pub const IMMEDIATE_VTABLE: u32 = 0x00b3_13b4;
pub const IMMEDIATE_DTOR: u32 = 0x00b0_3630;
/// `PPreprocessRingBuffer::GetSingleton`'s static, guard, critical
/// section constructor (`PCriticalSectionSimple`, at the static's start)
/// and `atexit` destructor.
pub const RING: u32 = 0x00cb_d980;
pub const RING_GUARD: u32 = 0x00cb_d9b0;
pub const CRITICAL_SECTION_CTOR: u32 = 0x0043_1690;
pub const RING_DTOR: u32 = 0x00b0_4930;

/// `PShadowCaster`'s constructor's constants: the word at `+4`, the float
/// at `+0x40` (an image constant) and the sampler state's constructor
/// (`+0x44`); `PShadowCaster::destructInstance`'s sampler state
/// destructor.
pub const SHADOW_CASTER_4: u32 = 0x00c1_58b0;
pub const SHADOW_CASTER_40: u32 = 0x00b1_f2ac;
pub const SAMPLER_CTOR: u32 = 0x0058_3b20;
pub const SAMPLER_DTOR: u32 = 0x0058_5230;
pub const SHADOW_CASTER_CTOR: u32 = 0x004c_9500;

/// `PInputAction`'s vtable.
pub const INPUT_ACTION_VTABLE: u32 = 0x00b7_2080;

/// `setLightMatrix`'s callees: `PArray_Data` (thiscall `(array)`), the
/// shader's parameter lookup (thiscall `(shader; name)`), the CRT's
/// `memcpy`, and the two parameter names.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const MEMCPY: u32 = 0x0094_9256;
pub const LIGHT_PARAMETER_B: u32 = 0x00b4_8148;
pub const LIGHT_PARAMETER_A: u32 = 0x00b4_8154;

/// What a `shr eax, 0x1f; not al` leaves in `eax` for the word `w`.
const fn owned_test(w: u32) -> u32 {
    (w >> 31) ^ 0xff
}

/// `mov al, 1` over `eax`.
const fn al_one(eax: u32) -> u32 {
    eax & !0xff | 1
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ARRAY_DATA, ATEXIT, CRITICAL_SECTION_CTOR, Conv, FIND_PARAMETER, Host, IMMEDIATE,
        IMMEDIATE_DTOR, IMMEDIATE_GET, IMMEDIATE_GUARD, IMMEDIATE_STATIC, IMMEDIATE_VTABLE,
        INPUT_ACTION_VTABLE, LIGHT_PARAMETER_A, LIGHT_PARAMETER_B, MEMCPY, PHYRE_FREE, POOL, RING,
        RING_DTOR, RING_GUARD, SAMPLER_CTOR, SAMPLER_DTOR, SHADOW_CASTER_4, SHADOW_CASTER_40,
        SHADOW_CASTER_CTOR, STEALING, STEALING_CTOR, STEALING_DTOR, STEALING_GUARD,
        THREAD_POOL_CTOR, WORKERS, WORKERS_ARG, al_one, owned_test,
    };

    /// Sets bit 0 of the guard word at `guard` (an image address) and
    /// says whether it was clear.
    fn first_time(h: &mut impl Host, guard: u32) -> bool {
        let g = h.rebase(guard);
        let w: u32 = h.read(g);
        if w & 1 != 0 {
            return false;
        }
        h.write(g, w | 1);
        true
    }

    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// `PThreadPool::GetSingleton()` (`0x0044cd30`): the pool, made on
    /// first use: the immediate pool for one worker or none, else the
    /// static work-stealing pool (built once: `(count, [0x00c0b95c])`).
    pub fn thread_pool(h: &mut impl Host) -> u32 {
        let pool = h.rebase(POOL);
        let p: u32 = h.read(pool);
        if p != 0 {
            return p;
        }
        let n: u32 = h.read(h.rebase(WORKERS));
        if n <= 1 {
            let p = h.call_sym(IMMEDIATE_GET, Conv::Cdecl, &[]);
            h.write(pool, p);
            return p;
        }
        let s = h.rebase(STEALING);
        if first_time(h, STEALING_GUARD) {
            let w: u32 = h.read(h.rebase(WORKERS_ARG));
            h.call_sym(STEALING_CTOR, Conv::Thiscall(s), &[n, w]);
            at_exit(h, STEALING_DTOR);
        }
        h.write(pool, s);
        s
    }

    /// `PThreadPoolImmediate::GetSingleton()` (`0x0057cf50`): the pool;
    /// on first use the static (built once: `PThreadPool(0, 0)`, its
    /// vtable, `atexit`).
    pub fn immediate_pool(h: &mut impl Host) -> u32 {
        let at = h.rebase(IMMEDIATE);
        let p: u32 = h.read(at);
        if p != 0 {
            return p;
        }
        let s = h.rebase(IMMEDIATE_STATIC);
        if first_time(h, IMMEDIATE_GUARD) {
            h.call_sym(THREAD_POOL_CTOR, Conv::Thiscall(s), &[0, 0]);
            let vt = h.rebase(IMMEDIATE_VTABLE);
            h.write(s, vt);
            at_exit(h, IMMEDIATE_DTOR);
        }
        h.write(at, s);
        s
    }

    /// `PPreprocessRingBuffer::GetSingleton()` (`0x005cd010`): the static
    /// ring, built once (its critical section, six words 0 at `+0x18`,
    /// `atexit`).
    pub fn ring_buffer(h: &mut impl Host) -> u32 {
        let r = h.rebase(RING);
        if first_time(h, RING_GUARD) {
            h.call_sym(CRITICAL_SECTION_CTOR, Conv::Thiscall(r), &[]);
            for i in 0..6 {
                h.write(r + 0x18 + 4 * i, 0u32);
            }
            at_exit(h, RING_DTOR);
        }
        r
    }

    /// `PShadowCaster::PShadowCaster(this)` (`0x004c9500`) -> `this`:
    /// `+0`, `+8`, `+0xc` 0, `+4` = `0x00c158b0`, the float `+0x40` from
    /// the image (through `fld`/`fstp`), the sampler state at `+0x44`;
    /// then, unless bit 7 of `+0x60` is set, its bytes `+0x44..+0x47` =
    /// 1, 1, 2, 2 and bit 6 of `+0x60` (each test reads `+0x60` again).
    pub fn shadow_caster_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        let w = h.rebase(SHADOW_CASTER_4);
        h.write(this + 4, w);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        let f: f32 = h.read(h.rebase(SHADOW_CASTER_40));
        h.write(this + 0x40, ffx_bridge::x87::round_trip(f));
        h.call_sym(SAMPLER_CTOR, Conv::Thiscall(this + 0x44), &[]);
        for (i, v) in (0u32..).zip([1u8, 1, 2, 2]) {
            if h.read::<u32>(this + 0x60) & 0x80 == 0 {
                h.write(this + 0x44 + i, v);
            }
        }
        let flags: u32 = h.read(this + 0x60);
        if flags & 0x80 == 0 {
            h.write(this + 0x60, flags | 0x40);
        }
        this
    }

    /// `PShadowCaster`'s `constructInstance(p)` (`0x004cc1f0`, stdcall):
    /// the constructor on `p` when set; `al` = 1 (over `p` from the
    /// constructor; for a null `p` the rest of `eax` is the original's
    /// frame address, here 0).
    pub fn shadow_caster_construct(h: &mut impl Host, p: u32) -> u32 {
        if p == 0 {
            return 1;
        }
        let eax = h.call_sym(SHADOW_CASTER_CTOR, Conv::Thiscall(p), &[]);
        al_one(eax)
    }

    /// `PShadowCaster`'s `destructInstance(p)` (`0x004cc2e0`, stdcall):
    /// the sampler state's destructor (`+0x44`), the owned array at `+8`
    /// freed, both its words 0; `al` = 1 over the `eax` that leaves.
    pub fn shadow_caster_destruct(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(SAMPLER_DTOR, Conv::Thiscall(p + 0x44), &[]);
        let count: u32 = h.read(p + 8);
        let mut eax = owned_test(count);
        if eax & 1 != 0 {
            let data: u32 = h.read(p + 0xc);
            eax = data;
            if data != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(p + 0xc, 0u32);
        h.write(p + 8, 0u32);
        al_one(eax)
    }

    /// `PInputAction`'s destructor `(this)` (`0x009e2330`): its vtable;
    /// each of the `[+8] & 0x7fffffff` elements at `[+0xc]` that is set
    /// deleted through its vtable slot 0 (`(1)`); the array freed when it
    /// has a count, data and is owned, both words 0; freed again when
    /// owned (now always: its words are 0); the byte `+4`'s word freed
    /// when bit 0 of `+4` is clear. `eax` as the original leaves it.
    pub fn input_action_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(INPUT_ACTION_VTABLE);
        h.write(this, vt);
        let mut i = 0;
        while i < h.read::<u32>(this + 8) & 0x7fff_ffff {
            let els: u32 = h.read(this + 0xc);
            let e: u32 = h.read(els.wrapping_add(4 * i));
            if e != 0 {
                let evt: u32 = h.read(e);
                let f: u32 = h.read(evt);
                h.call(f, Conv::Thiscall(e), &[1]);
            }
            i += 1;
        }
        let count: u32 = h.read(this + 8);
        if count != 0 {
            let data: u32 = h.read(this + 0xc);
            if data != 0 && owned_test(count) & 1 != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + 0xc, 0u32);
            h.write(this + 8, 0u32);
        }
        let count: u32 = h.read(this + 8);
        let mut eax = owned_test(count);
        if eax & 1 != 0 {
            let data: u32 = h.read(this + 0xc);
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this + 0xc, 0u32);
        h.write(this + 8, 0u32);
        let b: u8 = h.read(this + 4);
        eax = eax & !0xff | u32::from(!b);
        if !b & 1 != 0 {
            let w: u32 = h.read(this + 4);
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[w]);
        }
        eax
    }

    /// One parameter of `setLightMatrix`: `name`'s entry in `shader`
    /// (`0x0056cbf0`, thiscall), when found and `src` is set: `u16` offset
    /// `+8` and size `+0xa & 0x1fff`; copied from `src` into the buffer
    /// (`buf`, its first word the buffer's size) at the offset when the
    /// range fits and the offset is past the size word.
    fn light_parameter(h: &mut impl Host, shader: u32, buf: u32, name: u32, src: u32) {
        let name = h.rebase(name);
        let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if p == 0 || src == 0 {
            return;
        }
        let off = u32::from(h.read::<u16>(p + 8));
        let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
        if off + size <= h.read::<u32>(buf) && off >= 4 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[buf.wrapping_add(off), src, size]);
        }
    }

    /// `Phyre_CharacterLighting_setLightMatrix(this; node, a, b)`
    /// (`0x006aabf0`, thiscall, `ret 0xc`, no result): when the byte
    /// `+0x1cc` is set, for each shader entry `{shader, buffer}` of the
    /// node's array (`[node + 0x10]`, its elements through
    /// `PArray_Data`), the parameter `0x00b48148` from `b`, then
    /// `0x00b48154` from `a`.
    pub fn set_light_matrix(h: &mut impl Host, this: u32, node: u32, a: u32, b: u32) {
        if h.read::<u8>(this + 0x1cc) == 0 {
            return;
        }
        let arr: u32 = h.read(node + 0x10);
        let n = h.read::<u32>(arr) & 0x7fff_ffff;
        for i in 0..n {
            let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(arr), &[]);
            let e: u32 = h.read(data.wrapping_add(4 * i));
            let shader: u32 = h.read(e);
            let buf: u32 = h.read(e + 4);
            light_parameter(h, shader, buf, LIGHT_PARAMETER_B, b);
            light_parameter(h, shader, buf, LIGHT_PARAMETER_A, a);
        }
    }

    /// `PMatrix4_Transpose_004f93f0(out, m)` (cdecl) -> `out`: all of `m`
    /// read first (`out` may be `m`), then `out[r][c] = m[c][r]`, each
    /// float through `fld`/`fstp` (a signalling NaN comes out quiet).
    pub fn transpose(m: &mut impl ffx_bridge::Mem, out: u32, src: u32) -> u32 {
        let mut v = [0f32; 16];
        for (i, x) in (0u32..).zip(v.iter_mut()) {
            *x = m.read(src + 4 * i);
        }
        for r in 0..4u32 {
            for c in 0..4u32 {
                let x = v[(c * 4 + r) as usize];
                m.write(out + 4 * (r * 4 + c), ffx_bridge::x87::round_trip(x));
            }
        }
        out
    }
}

// The registrations' tables (shims: crates/ffx-hook/src/ported/p5_core_c.rs).

/// `0x0044_54a0` `Phyre_PString_Bind` for `ffx_prender::h1_bind` (read off
/// the machine code by a copy of h1's `gen.py`, `~/ffx-work/m4/port/p2c`).
pub const P5_PSTRING_BIND: ffx_prender::h1_bind::Bind = {
    use ffx_prender::h1_bind::{Bind, Step, Ty, Val};
    Bind {
        d: 0x00c9_1048,
        steps: &[
            Step::Guard(
                0x00c9_1120,
                0x1,
                &[
                    Step::Array {
                        member: 0x00c9_10e0,
                        ty: Ty::Call(0x0043_7c90),
                        name: 0x00b0_f77c,
                        words: [0x0, 0x0, 0x2, 0x1],
                    },
                    Step::Store(0x00c9_10e0, Val::Addr(0x00b0_f750)),
                    Step::Store(0x00c9_1114, Val::Addr(0x0044_5ae0)),
                    Step::Store(0x00c9_1118, Val::Imm(0x0)),
                    Step::Store(0x00c9_111c, Val::Addr(0x0044_5d90)),
                    Step::AtExit(0x00af_a390),
                ],
            ),
            Step::Or(0x00c9_10f4, 0x10),
            Step::Store(0x00c9_1118, Val::Addr(0x0043_2fd0)),
            Step::Store(0x00c9_110c, Val::Imm(0x2)),
            Step::Guard(
                0x00c9_1120,
                0x2,
                &[
                    Step::Class {
                        member: 0x00c9_1124,
                        name: 0x00b0_f788,
                        flags: 0x0,
                    },
                    Step::AtExit(0x00af_a350),
                ],
            ),
            Step::Store(0x00c9_113c, Val::Addr(0x00b0_f7e8)),
            Step::Store(0x00c9_1140, Val::Addr(0x0043_2f40)),
            Step::Store(0x00c9_1138, Val::Addr(0x00c9_113c)),
        ],
    }
};

/// `0x0044_b3b0` `PSharray<PString>`'s declaration for
/// `ffx_panim::p2_declare::bridge::declare_sharray` (its `m_u` flag word
/// 0).
pub const P5_PSTRING_SHARRAY: ffx_panim::p2_declare::Sharray = ffx_panim::p2_declare::Sharray {
    desc: 0x00c9_1a28,
    ctor: 0x0044_ac10,
    vtable: 0x00b1_00fc,
    name: 0x00c9_1b78,
    element: 0x00c9_1048,
    flag: 0,
    member_vtable: 0x00b1_0140,
    member_3c: 0x0044_ca80,
    member_38: 0x0044_c530,
    atexit: [0x00af_a540, 0x00af_a580, 0x00af_a560, 0x00af_a530],
};

/// `0x0054_ecb0` `PSharray<PTriggerReceiverComponent>`'s declaration.
pub const P5_TRIGGER_RECEIVER_SHARRAY: ffx_panim::p2_declare::Sharray =
    ffx_panim::p2_declare::Sharray {
        desc: 0x00ca_ef78,
        ctor: 0x0054_ddd0,
        vtable: 0x00b2_c954,
        name: 0x00ca_f1a4,
        element: 0x00ca_f388,
        flag: 2,
        member_vtable: 0x00b2_c998,
        member_3c: 0x0055_0750,
        member_38: 0x0055_02b0,
        atexit: [0x00b0_1e50, 0x00b0_1f90, 0x00b0_1f30, 0x00b0_1e40],
    };

/// `0x00a3_3680` `PArray<PInputMap *>`'s declaration for
/// `ffx_prender::declare::bridge::declare`.
pub const P5_INPUT_MAP_ARRAY: ffx_prender::declare::Declare = ffx_prender::declare::Declare {
    desc: 0x01a8_56b0,
    ctor: 0x00a3_2ef0,
    vtable: 0x00b7_a294,
    name: 0x01a8_57fc,
    element: ffx_prender::declare::Element::Descriptor(0x0194_1910),
    flags: 0x2,
    member_vtable: 0x00b7_a2d8,
    set_storage: 0x00a3_47b0,
    atexit: [0x00b0_b860, 0x00b0_b880, 0x00b0_b850, 0x00b0_b890],
};

/// `0x0045_2e80` `PClassDescriptorAbstract<PCamera>`'s constructor for
/// `ffx_prender::declare::bridge::abstract_ctor`.
pub const P5_PCAMERA_ABSTRACT: ffx_prender::declare::Abstract = ffx_prender::declare::Abstract {
    name: ffx_prender::declare::Name::Image(0x00b1_0b8c),
    size: 0x14c,
    vtable: 0x00b1_0b04,
    default_buffer: 0x00c9_2cf0,
    validation_buffer: 0x00c9_2e40,
    write_mask_buffer: 0x00c9_2f90,
};

/// `0x004c_9160` `PClassDescriptorAbstract<PShadowCaster::PShadowSplit>`'s
/// constructor.
pub const P5_PSHADOWSPLIT_ABSTRACT: ffx_prender::declare::Abstract =
    ffx_prender::declare::Abstract {
        name: ffx_prender::declare::Name::Image(0x00b1_ed54),
        size: 0xe0,
        vtable: 0x00b1_ecbc,
        default_buffer: 0x00ca_48f0,
        validation_buffer: 0x00ca_49d0,
        write_mask_buffer: 0x00ca_4ab0,
    };
