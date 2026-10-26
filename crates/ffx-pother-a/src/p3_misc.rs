//! Small functions of the list's second round: constructors and
//! destructors (the fly controller, the async process buffer, the scripted
//! component, the named semantic descriptor), create/destroy callbacks, the
//! TTY callback, a comparator, `cosf`/`sinf`, a vector normalisation and a
//! hashed random float.
//!
//! Exception frames and stack cookies are left out: nothing on these
//! paths throws.

use crate::{cdecl, free, thiscall, with_al};
use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PWorldMatrixFlyController_vftable`.
pub const FLY_CONTROLLER_VFTABLE: u32 = 0x00b3_9348;
/// `PWorldMatrixController_setVftable` (thiscall `(this)`), the fly
/// controller's destructor's tail.
pub const CONTROLLER_DTOR: u32 = 0x0061_a730;
/// `0x0061a690` (thiscall `(this)`): the controller base's constructor.
pub const CONTROLLER_CTOR: u32 = 0x0061_a690;
/// The CRT's `_CIcos` and `_CIsin` import thunks (argument and result in
/// `ST0`).
pub const CI_COS: u32 = 0x0094_98c2;
pub const CI_SIN: u32 = 0x0094_98c8;
/// The import slots of `printf`, `IsDebuggerPresent`, `DebugBreak`.
pub const IAT_PRINTF: u32 = 0x00b0_c380;
pub const IAT_IS_DEBUGGER_PRESENT: u32 = 0x00b0_c130;
pub const IAT_DEBUG_BREAK: u32 = 0x00b0_c134;
/// The TTY callback's format string (`printf(format, message)`).
pub const TTY_FORMAT: u32 = 0x00b0_e160;
/// The TTY callback object, its vtable, the pointer to it and the two once
/// flags `0x00435390` keeps.
pub const TTY_OBJECT: u32 = 0x00c9_02d8;
pub const TTY_VFTABLE: u32 = 0x00b0_e158;
pub const TTY_POINTER: u32 = 0x00c9_02e0;
pub const TTY_ONCE: u32 = 0x00c9_02e4;
pub const TTY_OBJECT_ONCE: u32 = 0x00c9_02dc;
/// The TTY object's `atexit` destructor.
pub const TTY_ATEXIT: u32 = 0x00af_9e20;
/// `atexit` (cdecl `(f)`).
pub const ATEXIT: u32 = 0x0094_9207;
/// The time stamp `0x005ce630` compares against.
pub const DRAIN_NOW: u32 = 0x00cb_d9c0;
/// `0x0047d3e0` (thiscall `(this, class, &object, offset, 0)`, `ret 0x10`).
pub const FIND_MEMBER_AT: u32 = 0x0047_d3e0;
/// `Phyre_PGameplay_PScriptableComponent_PScriptableComponent` (thiscall
/// `(this, descriptor)`).
pub const SCRIPTABLE_INIT: u32 = 0x0055_2c00;
/// `Phyre_PGameplay_PScriptedComponent_vftable`.
pub const SCRIPTED_VFTABLE: u32 = 0x00b2_d410;
/// `Phyre_PGameplay_PScriptableComponent_vftable`.
pub const SCRIPTABLE_VFTABLE: u32 = 0x00b2_d200;
/// `Phyre_PBase_GetClassDescriptor` (cdecl `()`).
pub const PBASE_GET_DESCRIPTOR: u32 = 0x0043_5740;
/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(descriptor)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `PUtilityGameplay::initialize`'s calls in order (cdecl `()`), and
/// whether their `eax` is then registered (`RegisterType`).
pub const GAMEPLAY_INIT: [(u32, bool); 14] = [
    (0x0054_5480, false),
    (0x0054_4d70, true),
    (0x0054_7ef0, false),
    (0x0055_64a0, false),
    (0x0054_e7f0, false),
    (0x0054_ce90, true),
    (0x0055_1920, false),
    (0x0055_2410, false),
    (0x0055_0d50, false),
    (0x0055_5470, false),
    (0x0055_2db0, false),
    (0x0055_3d50, false),
    (0x0055_76a0, false),
    (0x0055_9ad0, false),
];
/// `0x005a29d0` (thiscall `(element)`): the 0x24-byte element's destructor.
pub const ELEMENT_DTOR: u32 = 0x005a_29d0;
/// `Phyre_PScripting_PAsyncProcessBuffer_vftable`,
/// `Phyre_PScripting_PAsyncProcessBatch_vftable`.
pub const ASYNC_BUFFER_VFTABLE: u32 = 0x00b3_897c;
pub const ASYNC_BATCH_VFTABLE: u32 = 0x00b3_8970;
/// `0x005cea90` (thiscall `(owner, batch)`).
pub const ASYNC_DETACH: u32 = 0x005c_ea90;
/// `Phyre_PCriticalSectionSimple_PCriticalSectionSimple` (thiscall).
pub const CRITICAL_SECTION_INIT: u32 = 0x0043_1690;
/// `Phyre_PCriticalSectionBase_release_00431950` (thiscall).
pub const CRITICAL_SECTION_RELEASE: u32 = 0x0043_1950;
/// `Phyre_PhyreOS_Sleep` (cdecl `(ms)`).
pub const SLEEP: u32 = 0x0043_0030;
/// The two constants `0x005f0ce0` reads: 2^32 (`f32`, added to a negative
/// hash) and the scale (`f64`).
pub const HASH_UNSIGNED: u32 = 0x00b1_1f14;
pub const HASH_SCALE: u32 = 0x00b0_db98;
/// `0x0044d6a0` (thiscall `(ref, descriptor)`), `0x0044d730` (thiscall
/// `(ref)`).
pub const REF_INIT: u32 = 0x0044_d6a0;
pub const REF_RELEASE: u32 = 0x0044_d730;
/// `Phyre_PClassDataMember_vftable`.
pub const DATA_MEMBER_VFTABLE: u32 = 0x00b3_09d8;
/// `Phyre_PList_clear_005758e0` (thiscall `(list)`).
pub const LIST_CLEAR: u32 = 0x0057_58e0;
/// `Phyre_PNamedSemanticDescriptor_vftable` and the head of the list the
/// constructor links into (`{first, ..}`, nodes `{next, prev}`).
pub const NAMED_SEMANTIC_VFTABLE: u32 = 0x00b3_44b0;
pub const NAMED_SEMANTIC_LIST: u32 = 0x00c2_6b74;
/// The descriptors `0x00556910` and `0x00552200` initialise their
/// references with, and the word `0x0054e0f0` stores first.
pub const TIMER_REF_DESCRIPTOR: u32 = 0x00ca_fd60;
pub const RECEIVER_REF_DESCRIPTOR: u32 = 0x00ca_f540;
pub const LOCATOR_WORD: u32 = 0x00c2_1360;
/// `PMatrix4_toWorldMatrix3x4` (thiscall `(out, &matrix)`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
/// The bytes of the identity-matrix frame `0x0054e0f0` and `0x00552200`
/// pass.
pub const MATRIX_FRAME: u32 = 0x40;

/// `1.0f`.
const ONE: u32 = 0x3f80_0000;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ASYNC_BATCH_VFTABLE, ASYNC_BUFFER_VFTABLE, ASYNC_DETACH, ATEXIT, CI_COS, CI_SIN,
        CONTROLLER_CTOR, CONTROLLER_DTOR, CRITICAL_SECTION_INIT, CRITICAL_SECTION_RELEASE, Conv,
        DATA_MEMBER_VFTABLE, DRAIN_NOW, ELEMENT_DTOR, FIND_MEMBER_AT, FLY_CONTROLLER_VFTABLE,
        GAMEPLAY_INIT, HASH_SCALE, HASH_UNSIGNED, Host, IAT_DEBUG_BREAK, IAT_IS_DEBUGGER_PRESENT,
        IAT_PRINTF, LIST_CLEAR, LOCATOR_WORD, NAMED_SEMANTIC_LIST, NAMED_SEMANTIC_VFTABLE, ONE,
        PBASE_GET_DESCRIPTOR, RECEIVER_REF_DESCRIPTOR, REF_INIT, REF_RELEASE, REGISTER_TYPE,
        SCRIPTABLE_INIT, SCRIPTABLE_VFTABLE, SCRIPTED_VFTABLE, SLEEP, TIMER_REF_DESCRIPTOR,
        TO_WORLD_3X4, TTY_ATEXIT, TTY_FORMAT, TTY_OBJECT, TTY_OBJECT_ONCE, TTY_ONCE, TTY_POINTER,
        TTY_VFTABLE, cdecl, free, thiscall, with_al, x87,
    };

    /// `0x005d6d10` (thiscall `(controller)`): the fly controller's
    /// destructor: its vtable, then the base's (a tail jump); that `eax`.
    pub fn fly_controller_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(FLY_CONTROLLER_VFTABLE);
        h.write(this, vt);
        thiscall(h, CONTROLLER_DTOR, this, &[])
    }

    /// `0x005d6cc0` (thiscall `(controller)`): the fly controller's
    /// constructor: the base, the vtable, then `+0x34..+0x70` (one row
    /// pattern of 1.0 and 0.0); `this`.
    pub fn fly_controller_ctor(h: &mut impl Host, this: u32) -> u32 {
        thiscall(h, CONTROLLER_CTOR, this, &[]);
        let vt = h.rebase(FLY_CONTROLLER_VFTABLE);
        h.write(this, vt);
        // the stores in the original's order
        for (off, v) in [
            (0x34, ONE),
            (0x38, 0),
            (0x3c, 0),
            (0x40, 0),
            (0x44, 0),
            (0x4c, 0),
            (0x50, 0),
            (0x48, ONE),
            (0x54, 0),
            (0x58, 0),
            (0x60, 0),
            (0x5c, ONE),
            (0x64, 0),
            (0x68, 0),
            (0x6c, 0),
            (0x70, ONE),
        ] {
            h.write(this + off, v);
        }
        this
    }

    /// `cosf` / `sinf` (`0x004ceac0` / `0x004ceae0`, cdecl `(x)`): the
    /// CRT intrinsic on `x`, stored as `f32` and loaded back into `ST0`.
    pub fn cosf(h: &mut impl Host, x: u32) -> f32 {
        let f = h.rebase(CI_COS);
        h.call_intrinsic(f, x87::load(x))
    }

    /// See [`cosf`].
    pub fn sinf(h: &mut impl Host, x: u32) -> f32 {
        let f = h.rebase(CI_SIN);
        h.call_intrinsic(f, x87::load(x))
    }

    /// `0x005d3dd0` (destroy callback, stdcall `(p)`): frees the word at
    /// `p` unless its bit 0 is set; `al = 1`.
    pub fn destroy_free_word(h: &mut impl Host, p: u32) -> u8 {
        let w: u32 = h.read(p);
        if w & 1 == 0 {
            free(h, w);
        }
        1
    }

    /// `0x005d0640` (destroy callback, stdcall `(p)`): virtual slot 2 of
    /// the object at `p - 4` (address 0's object for a null `p`: a fault in
    /// both) with 0; `al = 1`.
    pub fn destroy_virtual2(h: &mut impl Host, p: u32) -> u8 {
        let obj = if p == 0 { 0 } else { p - 4 };
        h.call_virtual(obj, 2, &[0]);
        1
    }

    /// `0x004354f0` (`PTTYCallback`'s slot 1, `(level, message)`, `ret 8`,
    /// `this` unused): `printf(format, message)`; at level 4 with a
    /// debugger, `DebugBreak()`. `eax` as the original leaves it.
    pub fn tty_print(h: &mut impl Host, level: u32, message: u32) -> u32 {
        let printf: u32 = h.read(h.rebase(IAT_PRINTF));
        let format = h.rebase(TTY_FORMAT);
        let mut eax = h.call(printf, Conv::Cdecl, &[format, message]);
        if level == 4 {
            let is_debugger: u32 = h.read(h.rebase(IAT_IS_DEBUGGER_PRESENT));
            eax = h.call(is_debugger, Conv::Stdcall, &[]);
            if eax != 0 {
                let debug_break: u32 = h.read(h.rebase(IAT_DEBUG_BREAK));
                eax = h.call(debug_break, Conv::Stdcall, &[]);
            }
        }
        eax
    }

    /// `0x00435390` (cdecl `(level, message)`): the TTY callback (made on
    /// the first call: its vtable and `atexit` destructor once, then the
    /// pointer to it), its slot 1 with both; that `eax`.
    pub fn tty_dispatch(h: &mut impl Host, level: u32, message: u32) -> u32 {
        let once = h.rebase(TTY_ONCE);
        let flags: u32 = h.read(once);
        let obj = if flags & 1 == 0 {
            h.write(once, flags | 1);
            let made = h.rebase(TTY_OBJECT_ONCE);
            let f2: u32 = h.read(made);
            if f2 & 1 == 0 {
                let dtor = h.rebase(TTY_ATEXIT);
                h.write(made, f2 | 1);
                let vt = h.rebase(TTY_VFTABLE);
                h.write(h.rebase(TTY_OBJECT), vt);
                cdecl(h, ATEXIT, &[dtor]);
            }
            let obj = h.rebase(TTY_OBJECT);
            h.write(h.rebase(TTY_POINTER), obj);
            obj
        } else {
            h.read(h.rebase(TTY_POINTER))
        };
        h.call_virtual(obj, 1, &[level, message])
    }

    /// `0x005a0820` (cdecl `(&a, &b)`, a sort comparator): by the `u16` at
    /// `+2` descending, then the `i32` at `+8` descending: -1, 0 or 1.
    pub fn compare(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let pa: u32 = h.read(a);
        let pb: u32 = h.read(b);
        let x: u16 = h.read(pa + 2);
        let y: u16 = h.read(pb + 2);
        if x > y {
            return u32::MAX;
        }
        if x < y {
            return 1;
        }
        let i: i32 = h.read(pa + 8);
        let j: i32 = h.read(pb + 8);
        if i > j { u32::MAX } else { u32::from(i < j) }
    }

    /// `0x005ce630` (cdecl `(end, &cursor)`): advances the cursor (nodes
    /// `{next, stamp}`) past nodes whose stamp is 0 or earlier than the
    /// global's (signed difference), stopping at `end`; 0 when it moved, else
    /// 0xd.
    pub fn drain(h: &mut impl Host, end: u32, cursor: u32) -> u32 {
        let now: u32 = h.read(h.rebase(DRAIN_NOW));
        let mut moved = false;
        if h.read::<u32>(cursor) != end {
            loop {
                let n: u32 = h.read(cursor);
                let stamp: u32 = h.read(n + 4);
                if stamp != 0 && (stamp.wrapping_sub(now) as i32) >= 0 {
                    break;
                }
                let next: u32 = h.read(n);
                h.write(cursor, next);
                moved = true;
                if next == end {
                    break;
                }
            }
        }
        if moved { 0 } else { 0xd }
    }

    /// `0x0047d5f0` `Class_FindMemberByOffset` (thiscall `(this, class,
    /// &object, offset, &element)`, `ret 0x10`; `this` goes on to
    /// `0x0047d3e0`): the member `0x0047d3e0(class, &object, offset, 0)`
    /// finds, and the element index `(member class +0x14 - member +8 -
    /// object now + object before + offset) / 4` (unsigned) into
    /// `element`; the member or 0.
    pub fn find_member_by_offset(
        h: &mut impl Host,
        this: u32,
        class: u32,
        object: u32,
        offset: u32,
        element: u32,
    ) -> u32 {
        let before: u32 = h.read(object);
        let f = h.rebase(FIND_MEMBER_AT);
        let m = h.call(f, Conv::Thiscall(this), &[class, object, offset, 0]);
        if m == 0 {
            return 0;
        }
        let a: u32 = h.read(m);
        let b: u32 = h.read(m + 4);
        let a4: u32 = h.read(a + 4);
        let base: u32 = h.read(a4 + 0x14);
        let start: u32 = h.read(b + 8);
        let now: u32 = h.read(object);
        let i = base
            .wrapping_sub(start)
            .wrapping_sub(now)
            .wrapping_add(before)
            .wrapping_add(offset);
        h.write(element, i >> 2);
        m
    }

    /// `0x00553980` (thiscall `(this, descriptor)`): `PScriptedComponent`'s
    /// constructor: the scriptable base, the vtable, its fields; `this`.
    pub fn scripted_ctor(h: &mut impl Host, this: u32, descriptor: u32) -> u32 {
        thiscall(h, SCRIPTABLE_INIT, this, &[descriptor]);
        let vt = h.rebase(SCRIPTED_VFTABLE);
        h.write(this, vt);
        for off in [0x20, 0x24, 0x14, 0x18, 0x1c] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x28, 1u32);
        h.write(this + 0x2c, 0u16);
        h.write(this + 0x2e, 0u8);
        this
    }

    /// `0x005d5000` (create callback, stdcall `(p)`): eight zero words, a
    /// zero byte at `+0x20`, 1 at `+0x24`; `al = 1` over `p`.
    pub fn create_zeroed(h: &mut impl Host, p: u32) -> u32 {
        if p != 0 {
            for i in 0..8 {
                h.write(p + 4 * i, 0u32);
            }
            h.write(p + 0x20, 0u8);
            h.write(p + 0x24, 1u32);
        }
        with_al(p, 1)
    }

    /// `0x005d3860` (thiscall `(this)`): `{0, 0, PBase's descriptor}`;
    /// `this`.
    pub fn base_ref_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        let d = cdecl(h, PBASE_GET_DESCRIPTOR, &[]);
        h.write(this + 8, d);
        this
    }

    /// `0x005d3d60` (create callback, stdcall `(p)`): [`base_ref_ctor`]
    /// unless null; `al = 1`.
    pub fn create_base_ref(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            base_ref_ctor(h, p);
        }
        1
    }

    /// `0x004335b0` `PUtilityGameplay::initialize()`: the gameplay
    /// classes' `Bind`s and two registrations ([`super::GAMEPLAY_INIT`]);
    /// 0.
    pub fn gameplay_initialize(h: &mut impl Host) -> u32 {
        for &(f, reg) in &GAMEPLAY_INIT {
            let r = cdecl(h, f, &[]);
            if reg {
                cdecl(h, REGISTER_TYPE, &[r]);
            }
        }
        0
    }

    /// `0x005a2700` (thiscall `(array)`): unless the count word's bit 31
    /// is set (not owned), each 0x24-byte element's destructor and the
    /// buffer freed; then both words 0. `eax` as the original leaves it.
    pub fn array_release(h: &mut impl Host, this: u32) -> u32 {
        let w: u32 = h.read(this);
        // `shr eax, 31; not al`
        let mut eax = with_al(w >> 31, !((w >> 31) as u8));
        if w >> 31 == 0 {
            let data: u32 = h.read(this + 4);
            eax = data;
            if data != 0 {
                let mut e = data;
                for _ in 0..(w & 0x7fff_ffff) {
                    thiscall(h, ELEMENT_DTOR, e, &[]);
                    e = e.wrapping_add(0x24);
                }
                eax = free(h, data);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x005746b0` (cdecl `(out, &v)`): a 4x4 identity with `v` (x, y, z,
    /// each through `fld`/`fstp`) as the translation row; `out`.
    pub fn translation(h: &mut impl Host, out: u32, v: u32) -> u32 {
        let t = [0, 4, 8].map(|i| x87::store(x87::load(h.read(v + i))));
        for (off, x) in [
            (0x00, ONE),
            (0x04, 0),
            (0x08, 0),
            (0x0c, 0),
            (0x10, 0),
            (0x18, 0),
            (0x1c, 0),
            (0x14, ONE),
            (0x20, 0),
            (0x24, 0),
            (0x2c, 0),
            (0x28, ONE),
            (0x30, t[0]),
            (0x34, t[1]),
            (0x38, t[2]),
            (0x3c, ONE),
        ] {
            h.write(out + off, x);
        }
        out
    }

    /// `0x005ce550` (thiscall `(buffer)`): `PAsyncProcessBuffer`'s
    /// destructor: unlinks every node of its list (`+0x44`, nodes `{.., +0xc
    /// next, +0x10 owner}`) it owns, then the batch's vtable and the batch
    /// detached from its owner (`+0x10`) when it has one.
    pub fn async_buffer_dtor(h: &mut impl Host, this: u32) {
        let mut n: u32 = h.read(this + 0x44);
        let vt = h.rebase(ASYNC_BUFFER_VFTABLE);
        h.write(this, vt);
        while n != 0 {
            let next: u32 = h.read(n + 0xc);
            if h.read::<u32>(n + 0x10) == this && h.read::<u32>(this + 0x44) != 0 {
                let mut link = this + 0x44;
                loop {
                    let c: u32 = h.read(link);
                    if c == n {
                        h.write(link, next);
                        h.write(n + 0xc, 0u32);
                        h.write(n + 0x10, 0u32);
                        break;
                    }
                    if h.read::<u32>(c + 0xc) == 0 {
                        break;
                    }
                    link = c + 0xc;
                }
            }
            n = next;
        }
        let vt = h.rebase(ASYNC_BATCH_VFTABLE);
        h.write(this, vt);
        let owner: u32 = h.read(this + 0x10);
        if owner != 0 {
            thiscall(h, ASYNC_DETACH, owner, &[this]);
        }
    }

    /// `0x005ce4b0` (thiscall `(buffer)`): `PAsyncProcessBuffer`'s
    /// constructor; `this`.
    pub fn async_buffer_ctor(h: &mut impl Host, this: u32) -> u32 {
        thiscall(h, CRITICAL_SECTION_INIT, this + 0x14, &[]);
        for off in [0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40] {
            h.write(this + off, 0u32);
        }
        h.write(this + 8, this + 4);
        h.write(this + 4, 0u32);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        let vt = h.rebase(ASYNC_BUFFER_VFTABLE);
        h.write(this, vt);
        h.write(this + 0x44, 0u32);
        this
    }

    /// `0x005f0ce0` (cdecl `(seed, lo, hi)`): a hash of `seed` as an
    /// unsigned float (stored as `f32`) `r`, then `lo + (hi - lo) * r *
    /// scale`, stored as `f32` and returned in `ST0`.
    pub fn hashed_float(h: &mut impl Host, seed: u32, lo: u32, hi: u32) -> f32 {
        const GOLDEN: u32 = 0x9e37_79b9;
        let a = (seed ^ 0x00bc_602f).wrapping_mul(GOLDEN);
        let b = ((a >> 26) ^ (a << 6) ^ a).wrapping_mul(GOLDEN);
        let c = ((b >> 12) ^ (b << 5)).wrapping_add(b);
        let mut r = f64::from(c as i32);
        if (c as i32) < 0 {
            r = x87::add(r, x87::load(h.read(h.rebase(HASH_UNSIGNED))));
        }
        let r = x87::load(x87::store(r));
        let (lo, hi) = (x87::load(lo), x87::load(hi));
        let at = h.rebase(HASH_SCALE);
        let (low, high): (u32, u32) = (h.read(at), h.read(at + 4));
        let scale = f64::from_bits(u64::from(high) << 32 | u64::from(low));
        let t = x87::mul(x87::mul(x87::sub(hi, lo), r), scale);
        f32::from_bits(x87::store(x87::add(lo, t)))
    }

    /// `0x005d1880` (thiscall `(this)`): releases the critical section,
    /// drains the list at `+0x24` through the cursor at `+0x28` (sleeping 1
    /// ms between tries until it moves or reaches the end), then clears
    /// `+0x18..+0x28`; 0.
    pub fn drain_and_clear(h: &mut impl Host, this: u32) -> u32 {
        thiscall(h, CRITICAL_SECTION_RELEASE, this, &[]);
        let cursor = this + 0x28;
        let end: u32 = h.read(this + 0x24);
        if drain(h, end, cursor) == 0 {
            loop {
                let end: u32 = h.read(this + 0x24);
                if end == h.read::<u32>(cursor) {
                    break;
                }
                cdecl(h, SLEEP, &[1]);
                let end: u32 = h.read(this + 0x24);
                if drain(h, end, cursor) != 0 {
                    break;
                }
            }
        }
        h.write(cursor, 0u32);
        h.write(this + 0x24, 0u32);
        h.write(this + 0x18, 0u32);
        h.write(this + 0x1c, 0u32);
        h.write(this + 0x20, 0u32);
        0
    }

    /// `0x0055af50` (destroy callback, stdcall `(p)`): frees the word at
    /// `+0x54` unless its bit 0 is set, releases the reference at `p`;
    /// `al = 1`.
    pub fn destroy_ref_free54(h: &mut impl Host, p: u32) -> u8 {
        let w: u32 = h.read(p + 0x54);
        if w & 1 == 0 {
            free(h, w);
        }
        thiscall(h, REF_RELEASE, p, &[]);
        1
    }

    /// `0x00575470` (thiscall `(member)`): `PClassDataMember`'s destructor:
    /// the vtable; when its list (`+4`) is not empty, the owner's (`+0xc`)
    /// slot 9 with it; the list at `+0x1c` cleared; the buffer at `+0x10`
    /// freed when bit 0 of `+0x14` is set. `eax` as the original leaves it.
    pub fn data_member_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(DATA_MEMBER_VFTABLE);
        h.write(this, vt);
        if h.read::<u32>(this + 4) != this + 4 {
            let owner: u32 = h.read(this + 0xc);
            h.call_virtual(owner, 9, &[this]);
        }
        let mut eax = thiscall(h, LIST_CLEAR, this + 0x1c, &[]);
        if h.read::<u8>(this + 0x14) & 1 != 0 {
            let p: u32 = h.read(this + 0x10);
            eax = free(h, p);
        }
        eax
    }

    /// `0x005744a0` (cdecl `(out, &v)`): the 4-vector `v` over its length
    /// (the sum of squares and the length each stored as `f32`, then
    /// `1 / length`); `out`. `v` is read again as the original does, after
    /// each store (they may overlap).
    pub fn normalize4(h: &mut impl Host, out: u32, v: u32) -> u32 {
        fn ld(h: &impl Host, at: u32) -> f64 {
            x87::load(h.read(at))
        }
        fn f32r(x: f64) -> f64 {
            x87::load(x87::store(x))
        }
        let y = ld(h, v + 4);
        let x = ld(h, v);
        let z = ld(h, v + 8);
        let w = ld(h, v + 0xc);
        let mut s = f32r(x87::mul(x, x));
        s = f32r(x87::add(s, x87::mul(y, y)));
        s = f32r(x87::add(s, x87::mul(z, z)));
        s = f32r(x87::add(s, x87::mul(w, w)));
        let len = f32r(x87::sqrt(s));
        let inv = f32r(x87::div(1.0, len));
        let x = ld(h, v);
        h.write(out, x87::store(x87::mul(x, inv)));
        let y = ld(h, v + 4);
        h.write(out + 4, x87::store(x87::mul(inv, y)));
        let z = ld(h, v + 8);
        h.write(out + 8, x87::store(x87::mul(z, inv)));
        let w = ld(h, v + 0xc);
        h.write(out + 0xc, x87::store(x87::mul(inv, w)));
        out
    }

    /// `0x005a0200` `PNamedSemanticDescriptor::PNamedSemanticDescriptor`
    /// (thiscall `(this, a, b, c, d, e)`, `ret 0x14`): an empty tree at
    /// `+4`, the five words at `+0x18`, the vtable, the node at `+0x2c`
    /// linked first into the global list; `this`.
    pub fn named_semantic_ctor(h: &mut impl Host, this: u32, args: [u32; 5]) -> u32 {
        let t = this + 4;
        h.write(t, t);
        h.write(t + 4, t);
        h.write(t + 8, t);
        h.write(t + 0xc, 0u32);
        h.write(t + 0x10, 0u32);
        for (i, a) in (0u32..).zip(args) {
            h.write(this + 0x18 + 4 * i, a);
        }
        let node = this + 0x2c;
        h.write(node, node);
        h.write(node + 4, node);
        let vt = h.rebase(NAMED_SEMANTIC_VFTABLE);
        h.write(this, vt);
        let head = h.rebase(NAMED_SEMANTIC_LIST);
        let first: u32 = h.read(head);
        h.write(node, first);
        h.write(node + 4, head);
        h.write(head, node);
        h.write(first + 4, node);
        this
    }

    /// `0x00552c60` (thiscall `(component)`): `PScriptableComponent`'s
    /// destructor: the vtable, the word at `+0x10` freed unless its bit 0
    /// is set, the reference at `+4` released (its `eax`).
    pub fn scriptable_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(SCRIPTABLE_VFTABLE);
        h.write(this, vt);
        let w: u32 = h.read(this + 0x10);
        if w & 1 == 0 {
            free(h, w);
        }
        thiscall(h, REF_RELEASE, this + 4, &[])
    }

    /// `0x00556910` (create callback, stdcall `(p)`): unless null, the
    /// reference with the timer's descriptor, then `+0x10 = 1.0`, `+0xc =
    /// 0`, `+0x18 = 1` (byte), `+0x14 = 0.0`; `al = 1`.
    pub fn create_timer(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let d = h.rebase(TIMER_REF_DESCRIPTOR);
            thiscall(h, REF_INIT, p, &[d]);
            h.write(p + 0x10, ONE);
            h.write(p + 0xc, 0u32);
            h.write(p + 0x18, 1u8);
            h.write(p + 0x14, 0u32);
        }
        1
    }

    /// The identity matrix the originals build in their frame for
    /// `PMatrix4_toWorldMatrix3x4`, in their store order.
    fn identity(h: &mut impl Host, frame: u32) {
        for (off, x) in [
            (0x00, ONE),
            (0x04, 0),
            (0x08, 0),
            (0x0c, 0),
            (0x10, 0),
            (0x18, 0),
            (0x1c, 0),
            (0x20, 0),
            (0x24, 0),
            (0x2c, 0),
            (0x30, 0),
            (0x34, 0),
            (0x38, 0),
            (0x14, ONE),
            (0x28, ONE),
            (0x3c, ONE),
        ] {
            h.write(frame + off, x);
        }
    }

    /// `0x0054e0f0` (thiscall `(this)`): `{locator word, 0, 0, 0}`, the
    /// world matrix at `+0x10` from an identity in `frame`
    /// ([`super::MATRIX_FRAME`] bytes), `+0x40 = 1` (byte); `this`.
    pub fn locator_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let w = h.rebase(LOCATOR_WORD);
        h.write(this, w);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        identity(h, frame);
        thiscall(h, TO_WORLD_3X4, this + 0x10, &[frame]);
        h.write(this + 0x40, 1u8);
        this
    }

    /// `0x00552200` (thiscall `(this)`): the reference with the receiver
    /// descriptor, the world matrix at `+0xc` from an identity in `frame`,
    /// eight zero words at `+0x3c`; `this`.
    pub fn receiver_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let d = h.rebase(RECEIVER_REF_DESCRIPTOR);
        thiscall(h, REF_INIT, this, &[d]);
        identity(h, frame);
        thiscall(h, TO_WORLD_3X4, this + 0xc, &[frame]);
        for i in 0..8 {
            h.write(this + 0x3c + 4 * i, 0u32);
        }
        this
    }
}
