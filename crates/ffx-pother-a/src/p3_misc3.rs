//! Start-up and teardown code of the third continuation round: the engine's
//! init (`PhyreInit`), the Win32 timer's, `PhyrePrintf`, the timer
//! component's and the spline's destructors, the timer component's create
//! callback, the scheduler's clear, a script's array release and the class
//! view member walk's offset step.
//!
//! Exception frames and stack cookies are left out: nothing on these
//! paths throws, and the cookie only guards the frame.

use crate::{cdecl, free, thiscall, with_al};
use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The imports these call, through their IAT slots.
pub const IAT_TIME_BEGIN_PERIOD: u32 = 0x00b0_c704;
pub const IAT_SLEEP: u32 = 0x00b0_c100;
pub const IAT_TIME_GET_TIME: u32 = 0x00b0_c70c;
pub const IAT_QUERY_PERFORMANCE_FREQUENCY: u32 = 0x00b0_c0e8;
pub const IAT_QUERY_PERFORMANCE_COUNTER: u32 = 0x00b0_c0e4;
pub const IAT_GET_MODULE_HANDLE_A: u32 = 0x00b0_c114;
pub const IAT_GET_PROC_ADDRESS: u32 = 0x00b0_c118;
pub const IAT_VSNPRINTF_S: u32 = 0x00b0_c384;

/// `Phyre_PTimerWin32_Initialize`'s state: the two "API works" bytes
/// (`timeBeginPeriod` returned `TIMERR_NOERROR`,
/// `QueryPerformanceFrequency` returned 1), the
/// start `timeGetTime`, the start counter's milliseconds (low word), the
/// `GetSystemTimePreciseAsFileTime` pointer, the start time in seconds
/// (`f64`, counter-based and `timeGetTime`-based), the start counter and
/// the frequency (`i64`), the counter period (`f64`) and three zeroed
/// `f64`s.
pub const TIMER_PERIOD_OK: u32 = 0x00c9_0040;
pub const TIMER_HAS_COUNTER: u32 = 0x00c9_0041;
pub const TIMER_START_TICKS: u32 = 0x00c9_0044;
pub const TIMER_START_MS: u32 = 0x00c9_0048;
pub const TIMER_PRECISE_TIME: u32 = 0x00c9_004c;
pub const TIMER_START_COMBINED: u32 = 0x00c9_0050;
pub const TIMER_START_SECONDS: u32 = 0x00c9_0058;
pub const TIMER_ZERO_A: u32 = 0x00c9_0060;
pub const TIMER_COUNTER: u32 = 0x00c9_0068;
pub const TIMER_FREQUENCY: u32 = 0x00c9_0070;
pub const TIMER_PERIOD: u32 = 0x00c9_0078;
pub const TIMER_ZERO_B: u32 = 0x00c9_0080;
pub const TIMER_ZERO_C: u32 = 0x00c9_0088;
/// The `f64` constants: 1000, 2^-32, 2^32.
pub const F64_THOUSAND: u32 = 0x00b0_dba0;
pub const F64_TWO_POW_M32: u32 = 0x00b0_db98;
pub const F64_TWO_POW_32: u32 = 0x00b0_dbb0;
/// `"kernel32.dll"` and `"GetSystemTimePreciseAsFileTime"`.
pub const KERNEL32: u32 = 0x00b0_db80;
pub const PRECISE_TIME_NAME: u32 = 0x00b0_db60;

/// `PhyreInit`'s callees and state.
pub const PHYRE_INITIALIZED: u32 = 0x00c9_01a0;
pub const MEMORY_ALLOCATOR_INIT: u32 = 0x0046_7930;
/// The `Bind`s it calls first, in order.
pub const INIT_BINDS: [u32; 24] = [
    0x0045_d600, // PAssetReference_InitializeAssetTree
    0x0043_5610,
    0x0043_e000,
    0x0043_a1e0,
    0x0044_6490,
    0x0043_b8b0,
    0x0044_7da0,
    0x0043_ffe0,
    0x0044_2e30,
    0x0044_f120,
    0x0044_d880,
    0x0045_14a0,
    0x0045_37d0,
    0x0045_4070,
    0x0045_6540,
    0x0045_55e0,
    0x0044_54a0,
    0x0045_c500,
    0x0045_c890,
    0x0044_43f0,
    0x0044_b2b0,
    0x0045_80c0,
    0x0045_aa60,
    0x0045_e950,
];
/// The class descriptor getters whose results it registers, in order
/// (`PTimer`, `Vector2`, `Vector3`, `Vector4`, `Quat`, `Point3`, `Matrix3`,
/// `Matrix4`, `PMatrix4`, `PMatrix4x3`).
pub const INIT_TYPES: [u32; 10] = [
    0x0043_0340,
    0x0045_fb40,
    0x0045_fd90,
    0x0046_0070,
    0x0045_f7c0,
    0x0045_f4e0,
    0x0045_f210,
    0x0045_f300,
    0x0045_f3f0,
    0x0046_2aa0,
];
/// The init list (`{vtable, next}`; slot 1 inits, non-zero fails).
pub const INIT_LIST: u32 = 0x00c9_01a8;
pub const SHADER_PASS_INIT_ALL: u32 = 0x0046_4400;
pub const TIMER_INITIALIZE: u32 = 0x0042_fe10;
pub const TIMER_GET_TIME: u32 = 0x0042_fd80;
/// `g_AppStartTime` (`f64`).
pub const APP_START_TIME: u32 = 0x00c9_0278;
pub const THREAD_POOL_SET_STACK_SIZE: u32 = 0x0044_ce20;
pub const STORE_GLOBAL_C0B958: u32 = 0x0044_ce00;
pub const STORE_GLOBAL_C0B95C: u32 = 0x0044_cdf0;
pub const THREAD_POOL_GET_SINGLETON: u32 = 0x0044_cd30;
pub const REGISTER_DEFAULT_TYPES: u32 = 0x0043_9510;
pub const PBASE_GET_CLASS_DESCRIPTOR: u32 = 0x0043_5740;
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
pub const NAMESPACE_INIT_GLOBAL: u32 = 0x0043_e440;
pub const SORT_CLASS_DESCRIPTORS: u32 = 0x0043_be70;
pub const PHYRE_EXIT: u32 = 0x0043_0d60;
pub const ATEXIT: u32 = 0x0094_9207;

/// `PhyrePrintf`'s buffer (`[ebp-0x4004]`, 0x4000 bytes).
pub const PRINTF_BUFFER: u32 = 0x4000;

/// The timer component's vtable and its callees.
pub const TIMER_COMPONENT_VFTABLE: u32 = 0x00b2_d6d0;
pub const SCRIPT_RELEASE_5D5450: u32 = 0x005d_5450;
pub const SCRIPTABLE_DTOR: u32 = 0x0055_2c60;
pub const SCRIPTABLE_CTOR: u32 = 0x0055_2c00;
/// The timer component's class descriptor (the scriptable ctor's).
pub const TIMER_COMPONENT_DESCRIPTOR: u32 = 0x00ca_fb70;
/// `PSpline`'s vtable.
pub const SPLINE_VFTABLE: u32 = 0x00b2_c158;

/// The scheduler's callees.
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const DRAIN_AND_CLEAR: u32 = 0x005d_1880;
pub const FREE_PAIR_ARRAY: u32 = 0x005c_fa60;
/// `PArray_releaseTaggedWords_00434360` (cdecl `(data, count)`).
pub const RELEASE_TAGGED_WORDS: u32 = 0x0043_4360;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APP_START_TIME, ATEXIT, Conv, DRAIN_AND_CLEAR, F64_THOUSAND, F64_TWO_POW_32,
        F64_TWO_POW_M32, FREE_LIST_DEALLOCATE, FREE_LIST_RESET, FREE_PAIR_ARRAY, Host,
        IAT_GET_MODULE_HANDLE_A, IAT_GET_PROC_ADDRESS, IAT_QUERY_PERFORMANCE_COUNTER,
        IAT_QUERY_PERFORMANCE_FREQUENCY, IAT_SLEEP, IAT_TIME_BEGIN_PERIOD, IAT_TIME_GET_TIME,
        IAT_VSNPRINTF_S, INIT_BINDS, INIT_LIST, INIT_TYPES, KERNEL32, MEMORY_ALLOCATOR_INIT,
        NAMESPACE_INIT_GLOBAL, PBASE_GET_CLASS_DESCRIPTOR, PHYRE_EXIT, PHYRE_INITIALIZED,
        PRECISE_TIME_NAME, PRINTF_BUFFER, REGISTER_DEFAULT_TYPES, REGISTER_TYPE,
        RELEASE_TAGGED_WORDS, SCRIPT_RELEASE_5D5450, SCRIPTABLE_CTOR, SCRIPTABLE_DTOR,
        SHADER_PASS_INIT_ALL, SORT_CLASS_DESCRIPTORS, SPLINE_VFTABLE, STORE_GLOBAL_C0B95C,
        STORE_GLOBAL_C0B958, THREAD_POOL_GET_SINGLETON, THREAD_POOL_SET_STACK_SIZE,
        TIMER_COMPONENT_DESCRIPTOR, TIMER_COMPONENT_VFTABLE, TIMER_COUNTER, TIMER_FREQUENCY,
        TIMER_GET_TIME, TIMER_HAS_COUNTER, TIMER_INITIALIZE, TIMER_PERIOD, TIMER_PERIOD_OK,
        TIMER_PRECISE_TIME, TIMER_START_COMBINED, TIMER_START_MS, TIMER_START_SECONDS,
        TIMER_START_TICKS, TIMER_ZERO_A, TIMER_ZERO_B, TIMER_ZERO_C, cdecl, free, thiscall,
        with_al, x87,
    };

    /// Calls the import whose IAT slot is `iat` (`stdcall`).
    fn import(h: &mut impl Host, iat: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(iat));
        h.call(f, Conv::Stdcall, args)
    }

    /// `fild qword` and the `fstp qword` after it.
    fn load_i64(h: &impl Host, a: u32) -> f64 {
        let v: u64 = h.read(a);
        #[allow(clippy::cast_precision_loss)] // the x87 rounds the same at 53 bits
        let x = v.cast_signed() as f64;
        x
    }

    /// `fild dword` of `v` and the unsigned fix-up after it (`+ 2^32` when
    /// negative).
    fn unsigned(v: u32, two_pow_32: f64) -> f64 {
        let x = f64::from(v.cast_signed());
        if v.cast_signed() < 0 {
            x87::add(x, two_pow_32)
        } else {
            x
        }
    }

    /// `Phyre_PTimerWin32_Initialize` (cdecl `()`): 1 ms timer period
    /// (`timeBeginPeriod(5)`), the start tick, the performance counter's
    /// frequency, period and start in seconds and milliseconds, the start
    /// time combined (`ms * 2^32 + ticks`), the precise time function.
    /// Returns 0x17 when neither API works, else 0.
    pub fn timer_initialize(h: &mut impl Host) -> u32 {
        let r = import(h, IAT_TIME_BEGIN_PERIOD, &[5]);
        h.write(h.rebase(TIMER_PERIOD_OK), u8::from(r == 0));
        import(h, IAT_SLEEP, &[0]);
        let ticks = import(h, IAT_TIME_GET_TIME, &[]);
        let counter = h.rebase(TIMER_COUNTER);
        let frequency = h.rebase(TIMER_FREQUENCY);
        h.write(h.rebase(TIMER_START_TICKS), ticks);
        h.write(counter, 1u32);
        h.write(counter + 4, 0u32);
        h.write(frequency, 1u32);
        h.write(frequency + 4, 0u32);
        let ok = import(h, IAT_QUERY_PERFORMANCE_FREQUENCY, &[frequency]);
        let f = load_i64(h, frequency);
        h.write(h.rebase(TIMER_HAS_COUNTER), u8::from(ok == 1));
        let period = x87::div(1.0, f);
        h.write(h.rebase(TIMER_PERIOD), period.to_bits());
        import(h, IAT_QUERY_PERFORMANCE_COUNTER, &[counter]);
        let c = load_i64(h, counter);
        let seconds = x87::mul(c, period);
        h.write(h.rebase(TIMER_START_SECONDS), seconds.to_bits());
        let thousand = f64::from_bits(h.read(h.rebase(F64_THOUSAND)));
        let two_m32 = f64::from_bits(h.read(h.rebase(F64_TWO_POW_M32)));
        let two_32 = f64::from_bits(h.read(h.rebase(F64_TWO_POW_32)));
        // `fistp qword` toward zero, the low word kept
        let ms = x87::ftol2(x87::mul(x87::mul(seconds, thousand), two_m32));
        h.write(h.rebase(TIMER_START_MS), ms);
        let high = x87::mul(unsigned(ms, two_32), two_32);
        let combined = x87::add(high, unsigned(ticks, two_32));
        h.write(h.rebase(TIMER_START_COMBINED), combined.to_bits());
        for z in [TIMER_ZERO_B, TIMER_ZERO_C, TIMER_ZERO_A] {
            h.write(h.rebase(z), 0u64);
        }
        let name = h.rebase(PRECISE_TIME_NAME);
        let kernel32 = h.rebase(KERNEL32);
        let module = import(h, IAT_GET_MODULE_HANDLE_A, &[kernel32]);
        let precise = import(h, IAT_GET_PROC_ADDRESS, &[module, name]);
        h.write(h.rebase(TIMER_PRECISE_TIME), precise);
        let period_ok: u8 = h.read(h.rebase(TIMER_PERIOD_OK));
        let has: u8 = h.read(h.rebase(TIMER_HAS_COUNTER));
        if period_ok == 0 && has == 0 { 0x17 } else { 0 }
    }

    /// `Phyre_PhyreInit` (cdecl `()`): once, the allocator, the core
    /// classes' `Bind`s and types, the init list's slot 1, the shader
    /// passes, the timer (and the start time), the thread pool, the
    /// default types, `PBase`, the global class descriptors (sorted),
    /// `atexit(PhyreExit)`. The first non-zero step result is returned,
    /// else 0.
    pub fn phyre_init(h: &mut impl Host) -> u32 {
        let done = h.rebase(PHYRE_INITIALIZED);
        if h.read::<u8>(done) != 0 {
            return 0;
        }
        let r = cdecl(h, MEMORY_ALLOCATOR_INIT, &[]);
        if r != 0 {
            return r;
        }
        for f in INIT_BINDS {
            cdecl(h, f, &[]);
        }
        for g in INIT_TYPES {
            let t = cdecl(h, g, &[]);
            cdecl(h, REGISTER_TYPE, &[t]);
        }
        let mut p: u32 = h.read(h.rebase(INIT_LIST));
        while p != 0 {
            let r = h.call_virtual(p, 1, &[]);
            if r != 0 {
                return r;
            }
            p = h.read(p + 4);
        }
        for f in [SHADER_PASS_INIT_ALL, TIMER_INITIALIZE] {
            let r = cdecl(h, f, &[]);
            if r != 0 {
                return r;
            }
        }
        let t = h.call_sym_st0_f64(TIMER_GET_TIME, Conv::Cdecl, &[]);
        h.write(h.rebase(APP_START_TIME), t.to_bits());
        cdecl(h, THREAD_POOL_SET_STACK_SIZE, &[0x40000]);
        cdecl(h, STORE_GLOBAL_C0B958, &[0x40000]);
        cdecl(h, STORE_GLOBAL_C0B95C, &[0x800]);
        let pool = cdecl(h, THREAD_POOL_GET_SINGLETON, &[]);
        let r = h.call_virtual(pool, 2, &[]);
        if r != 0 {
            return r;
        }
        cdecl(h, REGISTER_DEFAULT_TYPES, &[]);
        let d = cdecl(h, PBASE_GET_CLASS_DESCRIPTOR, &[]);
        cdecl(h, REGISTER_TYPE, &[d]);
        cdecl(h, NAMESPACE_INIT_GLOBAL, &[]);
        let r = cdecl(h, SORT_CLASS_DESCRIPTORS, &[]);
        if r != 0 {
            return r;
        }
        let exit = h.rebase(PHYRE_EXIT);
        cdecl(h, ATEXIT, &[exit]);
        h.write(done, 1u8);
        0
    }

    /// `Phyre_PhyrePrintf` (cdecl `(level, format, ...)`): the message
    /// formatted into `buffer` (the original's `[ebp-0x4004]`,
    /// [`PRINTF_BUFFER`] bytes) by `_vsnprintf_s(buffer, 0x4000,
    /// _TRUNCATE, format, va)` with `va` the address of the variable
    /// arguments, then the TTY callback's dispatch; its `eax`.
    pub fn phyre_printf(h: &mut impl Host, level: u32, format: u32, va: u32, buffer: u32) -> u32 {
        let f: u32 = h.read(h.rebase(IAT_VSNPRINTF_S));
        h.call(
            f,
            Conv::Cdecl,
            &[buffer, PRINTF_BUFFER, u32::MAX, format, va],
        );
        crate::p3_misc::bridge::tty_dispatch(h, level, buffer)
    }

    /// `0x005553e0` (thiscall `(component, flags)`, `ret 4`): the timer
    /// component's scalar deleting destructor: its vtable, its script
    /// handle released (`0x005d5450`) when it owns one, the scriptable
    /// component's destructor, `PhyreFree` with flag 1. Returns `this`.
    pub fn timer_component_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(TIMER_COMPONENT_VFTABLE);
        h.write(this, vt);
        let handle: u32 = h.read(this + 0x1c);
        if handle != 0 && h.read::<u8>(this + 0x28) != 0 {
            cdecl(h, SCRIPT_RELEASE_5D5450, &[handle]);
            h.write(this + 0x28, 0u8);
            h.write(this + 0x1c, 0u32);
        }
        thiscall(h, SCRIPTABLE_DTOR, this, &[]);
        if flags & 1 != 0 {
            free(h, this);
        }
        this
    }

    /// `0x00555bc0` (stdcall `(p)`, the timer component's create
    /// callback): unless `p` is null, the scriptable component's
    /// constructor with the timer component's descriptor, two zero
    /// floats, the vtable and the owned byte cleared. `al` = 1.
    pub fn timer_component_create(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let d = h.rebase(TIMER_COMPONENT_DESCRIPTOR);
            thiscall(h, SCRIPTABLE_CTOR, p, &[d]);
            h.write(p + 0x20, 0u32);
            let vt = h.rebase(TIMER_COMPONENT_VFTABLE);
            h.write(p, vt);
            h.write(p + 0x24, 0u32);
            h.write(p + 0x28, 0u8);
        }
        1
    }

    /// One array `{count | 0x80000000 when not owned, data}` of `this`
    /// released as the spline's destructor does: when the count word is
    /// non-zero, the data freed when owned, then both words cleared.
    fn release_array(h: &mut impl Host, this: u32, count: u32, data: u32) {
        let n: u32 = h.read(this + count);
        if n != 0 {
            let p: u32 = h.read(this + data);
            if p != 0 && n >> 31 == 0 {
                free(h, p);
            }
            h.write(this + data, 0u32);
            h.write(this + count, 0u32);
        }
    }

    /// `0x005472c0` (thiscall `(spline)`): `PSpline`'s destructor body:
    /// its vtable, every member cleared (two zero floats), then its three
    /// arrays released (empty by then). Returns `this`.
    pub fn spline_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(SPLINE_VFTABLE);
        h.write(this, vt);
        for off in [4, 8, 0xc, 0x10, 0x1c, 0x14, 0x20] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x18, 0u8);
        h.write(this + 0x24, 0u32);
        h.write(this + 0x28, 0u32);
        release_array(h, this, 4, 8);
        release_array(h, this, 0x24, 0x28);
        release_array(h, this, 0xc, 0x10);
        this
    }

    /// Unlinks and frees every node of the singly linked list whose head
    /// is at `head` (`{next, .., +0x10 id}`): each found from the head,
    /// unlinked, its id set to -1 and handed back to the free list at
    /// `this + 4`.
    fn drain(h: &mut impl Host, this: u32, head: u32) {
        loop {
            let node: u32 = h.read(head);
            if node == 0 {
                return;
            }
            // the link that points at `node`
            let mut link = head;
            let mut cur: u32 = h.read(link);
            while cur != 0 && cur != node {
                link = cur;
                cur = h.read(cur);
            }
            if cur != 0 {
                let next: u32 = h.read(cur);
                h.write(link, next);
                h.write(cur, 0u32);
            }
            h.write(node + 0x10, u32::MAX);
            thiscall(h, FREE_LIST_DEALLOCATE, this + 4, &[node]);
        }
    }

    /// `0x005d18f0` (thiscall `(scheduler)`): `PScheduler`'s clear: the
    /// lists at `+0x24` and `+0x1c` drained into the free list at `+4`,
    /// the free list reset, the queue at `+0x50` drained, the arrays at
    /// `+0x34` (`PhyreFree`) and `+0x28` (`0x005cfa60`, the pairs)
    /// released.
    pub fn scheduler_clear(h: &mut impl Host, this: u32) {
        drain(h, this, this + 0x24);
        drain(h, this, this + 0x1c);
        thiscall(h, FREE_LIST_RESET, this + 4, &[]);
        thiscall(h, DRAIN_AND_CLEAR, this + 0x50, &[]);
        let n: u32 = h.read(this + 0x34);
        if n != 0 {
            let p: u32 = h.read(this + 0x38);
            if p != 0 && n >> 31 == 0 {
                free(h, p);
            }
            h.write(this + 0x38, 0u32);
            h.write(this + 0x34, 0u32);
        }
        let n: u32 = h.read(this + 0x28);
        if n != 0 {
            let p: u32 = h.read(this + 0x2c);
            if p != 0 && n >> 31 == 0 {
                cdecl(h, FREE_PAIR_ARRAY, &[p, n & 0x7fff_ffff]);
            }
            h.write(this + 0x2c, 0u32);
            h.write(this + 0x28, 0u32);
        }
    }

    /// `0x005d4770` (thiscall `(script)`): the four tagged pointers at
    /// `+0x1c`, `+0x18`, `+0x14`, `+0x10` freed when their bit 0 is clear,
    /// the tagged-word array `{count, data}` at `+8` released when owned,
    /// the array at `+0` freed when owned; both cleared. `eax` as the
    /// original leaves it (from the last test or `PhyreFree`).
    pub fn script_release(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x1c, 0x18, 0x14, 0x10] {
            let p: u32 = h.read(this + off);
            if p & 1 == 0 {
                free(h, p);
            }
        }
        let n: u32 = h.read(this + 8);
        if n >> 31 == 0 {
            let data: u32 = h.read(this + 0xc);
            cdecl(h, RELEASE_TAGGED_WORDS, &[data, n & 0x7fff_ffff]);
        }
        h.write(this + 0xc, 0u32);
        h.write(this + 8, 0u32);
        let n: u32 = h.read(this);
        // `shr eax, 31; not al`
        let mut eax = with_al(n >> 31, !((n >> 31) as u8));
        if n >> 31 == 0 {
            eax = h.read(this + 4);
            if eax != 0 {
                eax = free(h, eax);
            }
        }
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x0047ed80` (thiscall `(cursor, base, &offset)`, `ret 8`): the
    /// class view member walk's step: the member's object (slot 1 of the
    /// view at `+8`) and its kind (`+0x30`); unless the member is skipped
    /// (flag bit 1, or a kind-2 object whose slot 4 with `base` is not
    /// 1, when flag bit 6 asks), the offset of the nested data:
    /// `*offset` grows by the member's offset delta and the result is the
    /// nested object's address. 0 when skipped or there is no nested view.
    pub fn member_step(h: &mut impl Host, this: u32, base: u32, offset: u32) -> u32 {
        let d: u32 = h.read(this);
        let view: u32 = h.read(this + 8);
        let a: u32 = h.read(d + 8);
        let member: u32 = h.read(this + 4);
        let flags: u32 = h.read(member + 0x10);
        let b: u32 = h.read(member + 8);
        let dm: u32 = h.read(d + 4);
        let e: u32 = h.read(dm + 0x14);
        let obj = h.call_virtual(view, 1, &[]);
        let kind: u32 = if obj == 0 { 0 } else { h.read(obj + 0x30) };
        let extra: u32 = h.read(view + 0x24);
        let nested: u32 = h.read(this + 0x14);
        let checked = (flags >> 6) & 1 != 0 && obj != 0 && h.call_virtual(obj, 4, &[base]) == 1;
        let skip = (flags >> 1) & 1 != 0 || (kind == 2 && !checked);
        if nested == 0 || skip {
            return 0;
        }
        let q: u32 = h.read(nested + 8);
        let c: u32 = h.read(nested + 4);
        let t: u32 = if q == 0 { 0 } else { h.read(q + 0x88) };
        let delta = e.wrapping_sub(h.read(c + 0x14)).wrapping_sub(b);
        let a88: u32 = h.read(a + 0x88);
        let o: u32 = h.read(offset);
        h.write(offset, o.wrapping_add(delta));
        t.wrapping_sub(a88).wrapping_add(extra).wrapping_add(base)
    }
}
