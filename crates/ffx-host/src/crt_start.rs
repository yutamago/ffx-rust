//! `FFX.exe`'s entry point, the CRT's `WinMainCRTStartup` (`0x009493c1`:
//! `__security_init_cookie` `0x0094a224`, then `__tmainCRTStartup`
//! `0x009493cb`), as a host function in the function table (m5-wp5b,
//! 2026-11-12): the no-Unicorn build stops at the first address without a
//! Rust side, and this is the first. Its machine code, in order:
//!
//! - the stack cookie: kept when it already differs from the default
//!   `0xbb40e64e` with a high half set; else `GetSystemTimeAsFileTime`'s
//!   two halves, `GetCurrentThreadId`, `GetTickCount64`'s low half and
//!   `QueryPerformanceCounter`'s two halves xored with the address of the
//!   local they meet in (the original's, from the entry's stack), the default and
//!   a zero high half avoided as the CRT does; `[0x00c613dc]` its
//!   complement;
//! - `__crtGetShowWindowMode()`;
//! - the native start-up lock (`0x025d61a0`: this thread's stack base,
//!   released again before `WinMain`) and state (`0x025d61a4`): `_initterm_e`
//!   over the C initializers (`0x00b0d86c`..`0x00b0d880`; a non-zero
//!   result returns 255), `_initterm` over the C++ ones
//!   (`0x00b0c8c0`..`0x00b0d868`), state 2;
//! - the dynamic TLS callback (`0x025d61a8`): only called when
//!   `_IsNonwritableInCurrentImage` holds for its slot, which lies in
//!   `.data` (writable): never;
//! - the command line (`*_acmdln`) past the program name (quotes toggle,
//!   `_ismbblead` bytes take two) and the spaces after it;
//! - `WinMain` (`Game_Main`, stdcall `(0x00400000, 0, cmdline, show)`),
//!   its result into `0x0193e9c8`, then `exit` with it (the managed-app
//!   word `0x0193e9e0` is 0: the `_cexit` path is not taken).
//!
//! The SEH frame's filter (`_XcptFilter`) is the original's only for
//! exceptions that reach the entry; the executor ends the run for those.

use ffx_bridge::Mem;

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// The entry point.
pub const ENTRY: u32 = 0x0094_93c1;

/// `__security_cookie` and its complement.
const COOKIE: u32 = 0x00c6_13d8;
const COOKIE_COMPLEMENT: u32 = 0x00c6_13dc;
const DEFAULT_COOKIE: u32 = 0xbb40_e64e;
/// The import slots the start-up calls through.
const GET_SYSTEM_TIME_AS_FILE_TIME: u32 = 0x00b0_c040;
const GET_TICK_COUNT64: u32 = 0x00b0_c044;
const GET_CURRENT_THREAD_ID: u32 = 0x00b0_c108;
const QUERY_PERFORMANCE_COUNTER: u32 = 0x00b0_c0e4;
const CRT_GET_SHOW_WINDOW_MODE: u32 = 0x00b0_c4d4;
const AMSG_EXIT: u32 = 0x00b0_c4d8;
const INITTERM_E: u32 = 0x00b0_c4fc;
const INITTERM: u32 = 0x00b0_c500;
const ISMBBLEAD: u32 = 0x00b0_c4f0;
const ACMDLN: u32 = 0x00b0_c504;
const EXIT: u32 = 0x00b0_c4e4;
const CEXIT: u32 = 0x00b0_c4ec;
/// The start-up state, the managed-app word, the result, the "already
/// initialised" flag.
const NATIVE_STARTUP_STATE: u32 = 0x025d_61a4;
/// The native start-up lock (`__native_startup_lock`).
const STARTUP_LOCK: u32 = 0x025d_61a0;
const MANAGED_APP: u32 = 0x0193_e9e0;
const MAIN_RESULT: u32 = 0x0193_e9c8;
const NESTED: u32 = 0x0193_e9cc;
/// The initializer tables.
const XI_A: u32 = 0x00b0_d86c;
const XI_Z: u32 = 0x00b0_d880;
const XC_A: u32 = 0x00b0_c8c0;
const XC_Z: u32 = 0x00b0_d868;
/// `WinMain`.
const GAME_MAIN: u32 = 0x0062_ee50;
const IMAGE_BASE: u32 = 0x0040_0000;

/// Calls the import in `slot` with `words`.
fn import(e: &mut Exec, slot: u32, conv: Conv, words: &[u32]) -> Option<Ret> {
    let f: u32 = e.mem().read(slot);
    let args = Args {
        stack: words.to_vec(),
        ..Args::default()
    };
    e.call(f, conv, &args).ok()
}

/// `__security_init_cookie` (`0x0094a224`) with its frame pointer at
/// `ebp`: the file time at `ebp - 0xc`, the counter at `ebp - 0x14`, the
/// local at `ebp - 4` (its address xored in).
fn init_cookie(e: &mut Exec, ebp: u32) -> Option<()> {
    let cookie: u32 = e.mem().read(COOKIE);
    if cookie != DEFAULT_COOKIE && cookie & 0xffff_0000 != 0 {
        e.mem().write(COOKIE_COMPLEMENT, !cookie);
        return Some(());
    }
    let (ft, pc, local) = (ebp - 0xc, ebp - 0x14, ebp - 4);
    e.mem().write(ft, 0u64);
    import(e, GET_SYSTEM_TIME_AS_FILE_TIME, Conv::Stdcall, &[ft])?;
    let mem = e.mem();
    let mut v = mem.read::<u32>(ft + 4) ^ mem.read::<u32>(ft);
    v ^= import(e, GET_CURRENT_THREAD_ID, Conv::Stdcall, &[])?.eax;
    v ^= import(e, GET_TICK_COUNT64, Conv::Stdcall, &[])?.eax;
    import(e, QUERY_PERFORMANCE_COUNTER, Conv::Stdcall, &[pc])?;
    let mem = e.mem();
    let mut c = mem.read::<u32>(pc + 4) ^ mem.read::<u32>(pc) ^ v ^ local;
    if c == DEFAULT_COOKIE {
        c = DEFAULT_COOKIE + 1;
    } else if c & 0xffff_0000 == 0 {
        c |= (c | 0x4711) << 16;
    }
    let mut mem = e.mem();
    mem.write(COOKIE, c);
    mem.write(COOKIE_COMPLEMENT, !c);
    Some(())
}

/// The command line past the program name and the spaces after it.
fn command_line(e: &mut Exec) -> Option<u32> {
    let var: u32 = e.mem().read(ACMDLN);
    let mut p: u32 = e.mem().read(var);
    let mut quoted = false;
    loop {
        let c: u8 = e.mem().read(p);
        if c <= b' ' && (c == 0 || !quoted) {
            break;
        }
        if c == b'"' {
            quoted = !quoted;
        }
        if import(e, ISMBBLEAD, Conv::Cdecl, &[u32::from(c)])?.eax != 0 {
            p = p.wrapping_add(1);
        }
        p = p.wrapping_add(1);
    }
    loop {
        let c: u8 = e.mem().read(p);
        if c == 0 || c > b' ' {
            return Some(p);
        }
        p = p.wrapping_add(1);
    }
}

/// `WinMainCRTStartup()` (`0x009493c1`): see the module docs. The process
/// exit code (`exit` ends the run before it returns).
fn entry(e: &mut Exec, args: &Args) -> Ret {
    run(e, args).unwrap_or(Ret::eax(0))
}

fn run(e: &mut Exec, args: &Args) -> Option<Ret> {
    // `__security_init_cookie`'s frame where the original has it (its
    // locals are xored into the cookie by address): 0x0c below the entry's
    // first stack word (the return address, the call's, the saved `ebp`),
    // or, called from Rust (the thread's start), below where `Exec::call`
    // would have put that word for the emulated original (0x40 below `esp`,
    // 16-aligned); on the game stack, so no heap block moves
    let first = if args.slots == 0 {
        let esp = e.reg(crate::exec::RegisterX86::ESP).ok()?;
        esp.wrapping_sub(0x40) & !0xf
    } else {
        args.slots
    };
    init_cookie(e, first.wrapping_sub(0xc))?;
    let show = import(e, CRT_GET_SHOW_WINDOW_MODE, Conv::Cdecl, &[])?.eax & 0xffff;
    // the native start-up lock: this thread's stack base (`[TEB + 4]`),
    // taken with `lock cmpxchg` against 0; already held by this thread: a
    // nested start-up (the lock is then not released below)
    let fiber: u32 = e.mem().read(e.teb + 4);
    let held: u32 = e.mem().read(STARTUP_LOCK);
    let nested = held == fiber && held != 0;
    if held == 0 {
        e.mem().write(STARTUP_LOCK, fiber);
    }
    let state: u32 = e.mem().read(NATIVE_STARTUP_STATE);
    if state == 1 {
        import(e, AMSG_EXIT, Conv::Cdecl, &[0x1f])?;
    } else if state == 0 {
        e.mem().write(NATIVE_STARTUP_STATE, 1u32);
        if import(e, INITTERM_E, Conv::Cdecl, &[XI_A, XI_Z])?.eax != 0 {
            return Some(Ret::eax(0xff));
        }
    } else {
        e.mem().write(NESTED, 1u32);
    }
    if e.mem().read::<u32>(NATIVE_STARTUP_STATE) == 1 {
        import(e, INITTERM, Conv::Cdecl, &[XC_A, XC_Z])?;
        e.mem().write(NATIVE_STARTUP_STATE, 2u32);
    }
    if !nested {
        e.mem().write(STARTUP_LOCK, 0u32);
    }
    let cmdline = command_line(e)?;
    let args = Args {
        stack: vec![IMAGE_BASE, 0, cmdline, show],
        ..Args::default()
    };
    let r = e.call(GAME_MAIN, Conv::Stdcall, &args).ok()?.eax;
    e.mem().write(MAIN_RESULT, r);
    if e.mem().read::<u32>(MANAGED_APP) == 0 {
        import(e, EXIT, Conv::Cdecl, &[r])?;
    }
    if e.mem().read::<u32>(NESTED) == 0 {
        import(e, CEXIT, Conv::Cdecl, &[])?;
    }
    Some(Ret::eax(e.mem().read::<u32>(MAIN_RESULT)))
}

/// The entry point's and the C initializers' table entries.
pub fn register(t: &mut Table) {
    for (addr, name, port) in [
        (
            ENTRY,
            "WinMainCRTStartup",
            entry as fn(&mut Exec, &Args) -> Ret,
        ),
        (0x0094_92b6, "pre_c_init", pre_c_init),
        (0x0094_9126, "onexitinit", onexit_init),
        (0x0094_a374, "__isa_available_init", isa_init),
        (
            0x0094_a091,
            "setUnhandledExceptionFilter_0094a091",
            set_unhandled_filter,
        ),
        (0x0094_936f, "pre_cpp_init", pre_cpp_init),
        (RTC_TERMINATE, "_RTC_Terminate", rtc_terminate),
    ] {
        t.register(
            addr,
            Entry {
                name,
                conv: Conv::Cdecl,
                words: 0,
                port: Port::Exec(port),
            },
        );
    }
}

// ---- the C initializers `_initterm_e` runs (`0x00b0d870`..) ----

const SET_APP_TYPE: u32 = 0x00b0_c4e0;
const ENCODE_POINTER: u32 = 0x00b0_c034;
const FMODE_SLOT: u32 = 0x00b0_c508;
const COMMODE_SLOT: u32 = 0x00b0_c50c;
const SETUSERMATHERR: u32 = 0x00b0_c4f8;
const CONFIGTHREADLOCALE: u32 = 0x00b0_c4f4;
const CALLOC_CRT: u32 = 0x00b0_c4c0;
const CONTROLFP_S: u32 = 0x00b0_c52c;
const INVOKE_WATSON: u32 = 0x00b0_c528;
const SET_UNHANDLED_FILTER: u32 = 0x00b0_c524;
const IS_PROCESSOR_FEATURE_PRESENT: u32 = 0x00b0_c03c;
/// `__onexitbegin`/`__onexitend` (encoded).
const ONEXIT_END: u32 = 0x025d_61ac;
const ONEXIT_BEGIN: u32 = 0x025d_61b0;
/// The image's `_fmode`/`_commode` values the CRT copies out, the
/// `_matherr` hook flag, the thread-locale setting.
const IMAGE_FMODE: u32 = 0x0193_ed1c;
const IMAGE_COMMODE: u32 = 0x0193_ed10;
const USERMATHERR_SET: u32 = 0x00c6_13e0;
const THREAD_LOCALE: u32 = 0x00c6_13e4;
/// `_matherr`'s default (returns 0) and the unhandled-exception filter.
const MATHERR: u32 = 0x0094_a2be;
const UNHANDLED_FILTER: u32 = 0x0094_a050;
/// The RTC initializer table (empty in this build: begin = end).
const RTC_A: u32 = 0x00bc_b0f4;
const RTC_Z: u32 = 0x00bc_b0f4;
/// `__isa_available`, `__favor`'s flags, the CPU feature word.
const ISA_AVAILABLE: u32 = 0x0193_ed20;
const ISA_FLAGS: u32 = 0x00c6_13ec;
const CPU_FEATURES: u32 = 0x0193_ed24;

fn word(e: &Exec, at: u32) -> u32 {
    e.mem().read(at)
}

/// `pre_c_init` (`0x009492b6`): the managed-app test of the image's PE
/// header (a CLR directory: not `FFX.exe`) into `0x0193e9e0`,
/// `__set_app_type(2)` (GUI), the `atexit` table's ends `EncodePointer(-1)`,
/// `_fmode`/`_commode` copied into the CRT's variables, the (empty) RTC
/// initializers, `__setusermatherr(_matherr)` unless set, the default
/// precision (`_controlfp_s(0, 0x10000, 0x30000)`, `_invoke_watson` on
/// failure), `_configthreadlocale(-1)` when asked. 0.
fn pre_c_init(e: &mut Exec, _: &Args) -> Ret {
    pre_c_init_run(e).unwrap_or(Ret::eax(0))
}

fn pre_c_init_run(e: &mut Exec) -> Option<Ret> {
    let base = IMAGE_BASE;
    let managed = (|| {
        if e.mem().read::<u16>(base) != 0x5a4d {
            return 0;
        }
        let pe = base.wrapping_add(word(e, base + 0x3c));
        if word(e, pe) != 0x4550 || e.mem().read::<u16>(pe + 0x18) != 0x10b {
            return 0;
        }
        if word(e, pe + 0x74) <= 0xe {
            return 0;
        }
        u32::from(word(e, pe + 0xe8) != 0)
    })();
    e.mem().write(MANAGED_APP, managed);
    import(e, SET_APP_TYPE, Conv::Cdecl, &[2])?;
    let end = import(e, ENCODE_POINTER, Conv::Stdcall, &[u32::MAX])?.eax;
    e.mem().write(ONEXIT_END, end);
    e.mem().write(ONEXIT_BEGIN, end);
    for (slot, value) in [(FMODE_SLOT, IMAGE_FMODE), (COMMODE_SLOT, IMAGE_COMMODE)] {
        let var = word(e, slot);
        let v = word(e, value);
        e.mem().write(var, v);
    }
    let mut at = RTC_A;
    while at < RTC_Z {
        let f = word(e, at);
        if f != 0 {
            e.call(f, Conv::Cdecl, &Args::default()).ok()?;
        }
        at += 4;
    }
    if word(e, USERMATHERR_SET) == 0 {
        import(e, SETUSERMATHERR, Conv::Cdecl, &[MATHERR])?;
    }
    if import(e, CONTROLFP_S, Conv::Cdecl, &[0, 0x1_0000, 0x3_0000])?.eax != 0 {
        import(e, INVOKE_WATSON, Conv::Cdecl, &[0, 0, 0, 0, 0])?;
    }
    if word(e, THREAD_LOCALE) == u32::MAX {
        import(e, CONFIGTHREADLOCALE, Conv::Cdecl, &[u32::MAX])?;
    }
    Some(Ret::eax(0))
}

/// `onexitinit` (`0x00949126`): unless the `atexit` table exists (it does
/// after [`pre_c_init`]), 32 entries from `_calloc_crt`, encoded into
/// both ends; 0, or 0x18 (`_RT_ONEXIT`) for no memory.
fn onexit_init(e: &mut Exec, _: &Args) -> Ret {
    (|| {
        if word(e, ONEXIT_BEGIN) != 0 {
            return Some(Ret::eax(0));
        }
        let p = import(e, CALLOC_CRT, Conv::Cdecl, &[0x20, 4])?.eax;
        let enc = import(e, ENCODE_POINTER, Conv::Stdcall, &[p])?.eax;
        e.mem().write(ONEXIT_BEGIN, enc);
        e.mem().write(ONEXIT_END, enc);
        if p == 0 {
            return Some(Ret::eax(0x18));
        }
        e.mem().write(p, 0u32);
        Some(Ret::eax(0))
    })()
    .unwrap_or(Ret::eax(0))
}

/// `cpuid(leaf, 0)` of the machine the host runs on (`eax`, `ebx`, `ecx`,
/// `edx`); zeros elsewhere.
fn cpuid(leaf: u32) -> [u32; 4] {
    #[cfg(target_arch = "x86_64")]
    {
        let r = std::arch::x86_64::__cpuid_count(leaf, 0);
        [r.eax, r.ebx, r.ecx, r.edx]
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = leaf;
        [0; 4]
    }
}

/// `__isa_available_init` (`0x0094a374`): with SSE2
/// (`IsProcessorFeaturePresent(10)`), `__isa_available` 1, 2 with SSE4.2,
/// 3 with AVX, the `__favor` flags 1|2(|4)(|8); the feature word's bit 1
/// for ERMS (`cpuid` 7) and bit 0 for the Atom models of a `GenuineIntel`
/// processor. From the processor the host runs on (Unicorn's was
/// Unicorn's). 0.
fn isa_init(e: &mut Exec, _: &Args) -> Ret {
    e.mem().write(ISA_AVAILABLE, 0u32);
    let flags = word(e, ISA_FLAGS) | 1;
    e.mem().write(ISA_FLAGS, flags);
    let Some(sse2) = import(e, IS_PROCESSOR_FEATURE_PRESENT, Conv::Stdcall, &[10]) else {
        return Ret::eax(0);
    };
    if sse2.eax == 0 {
        return Ret::eax(0);
    }
    e.mem().write(ISA_AVAILABLE, 1u32);
    let one = cpuid(1);
    let mut flags = word(e, ISA_FLAGS) | 2;
    e.mem().write(ISA_FLAGS, flags);
    if one[2] & 0x10_0000 != 0 {
        flags |= 4;
        e.mem().write(ISA_AVAILABLE, 2u32);
        e.mem().write(ISA_FLAGS, flags);
    }
    if one[2] & 0x1000_0000 != 0 {
        flags |= 8;
        e.mem().write(ISA_AVAILABLE, 3u32);
        e.mem().write(ISA_FLAGS, flags);
    }
    let seven = cpuid(7);
    let mut features = word(e, CPU_FEATURES);
    if seven[1] & 0x200 != 0 {
        features |= 2;
        e.mem().write(CPU_FEATURES, features);
    }
    let zero = cpuid(0);
    if zero[1] == 0x756e_6547 && zero[3] == 0x4965_6e69 && zero[2] == 0x6c65_746e {
        let model = cpuid(1)[0] & 0x0fff_3ff0;
        if [0x106c0, 0x20660, 0x20670, 0x30650, 0x30660, 0x30670].contains(&model) {
            features |= 1;
            e.mem().write(CPU_FEATURES, features);
        }
    }
    Ret::eax(0)
}

/// `0x0094a091`: `__crtSetUnhandledExceptionFilter(0x0094a050)`; 0.
fn set_unhandled_filter(e: &mut Exec, _: &Args) -> Ret {
    import(e, SET_UNHANDLED_FILTER, Conv::Cdecl, &[UNHANDLED_FILTER]);
    Ret::eax(0)
}

// ---- the first C++ initializer `_initterm` runs (`0x00b0c8c4`) ----

const ATEXIT_FN: u32 = 0x0094_9207;
const GETMAINARGS: u32 = 0x00b0_c4dc;
/// `_RTC_Terminate` (`0x0094a2e8`; its table `0x00bcb0fc`.. is empty).
const RTC_TERMINATE: u32 = 0x0094_a2e8;
const RTC_T_A: u32 = 0x00bc_b0fc;
const RTC_T_Z: u32 = 0x00bc_b0fc;
/// `argc`, `argv`, `envp`, the `__getmainargs` result, `_startupinfo`,
/// `_dowildcard` and `_newmode`.
const ARGC: u32 = 0x0193_e9d0;
const ARGV: u32 = 0x0193_e9d4;
const ENVP: u32 = 0x0193_e9d8;
const ARGRET: u32 = 0x0193_e9dc;
const STARTINFO: u32 = 0x0193_e9e4;
const DOWILDCARD: u32 = 0x0193_ed14;
const NEWMODE: u32 = 0x0193_ed18;

/// `pre_cpp_init` (`0x0094936f`): `atexit(_RTC_Terminate)`, the start-up
/// info's new mode, `__getmainargs(&argc, &argv, &envp, _dowildcard,
/// &startinfo)` (its result kept; `_amsg_exit(8)` when negative). Its
/// `eax`: the result.
fn pre_cpp_init(e: &mut Exec, _: &Args) -> Ret {
    (|| {
        let args = Args {
            stack: vec![RTC_TERMINATE],
            ..Args::default()
        };
        e.call(ATEXIT_FN, Conv::Cdecl, &args).ok()?;
        let newmode = word(e, NEWMODE);
        e.mem().write(STARTINFO, newmode);
        let wild = word(e, DOWILDCARD);
        let r = import(
            e,
            GETMAINARGS,
            Conv::Cdecl,
            &[ARGC, ARGV, ENVP, wild, STARTINFO],
        )?
        .eax;
        e.mem().write(ARGRET, r);
        if r.cast_signed() < 0 {
            import(e, AMSG_EXIT, Conv::Cdecl, &[8])?;
        }
        Some(Ret::eax(r))
    })()
    .unwrap_or(Ret::eax(0))
}

/// `_RTC_Terminate` (`0x0094a2e8`): the RTC terminators (none in this
/// build) called.
fn rtc_terminate(e: &mut Exec, _: &Args) -> Ret {
    let mut at = RTC_T_A;
    while at < RTC_T_Z {
        let f = word(e, at);
        if f != 0 && e.call(f, Conv::Cdecl, &Args::default()).is_err() {
            break;
        }
        at += 4;
    }
    Ret::eax(at)
}
