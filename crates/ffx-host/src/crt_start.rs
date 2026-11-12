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

/// The entry point's, the C initializers' and `Game_Main`'s table entries
/// (`FFX_HOST_DISABLE=<names>` leaves the named ones to the emulator in
/// the dev-unicorn build, as for the `ported!` entries).
pub fn register(t: &mut Table) {
    let disable = crate::ports::names("FFX_HOST_DISABLE").unwrap_or_default();
    let mut t = Filtered { t, disable };
    let t = &mut t;
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
    t.register(
        GAME_MAIN,
        Entry {
            name: "Game_Main",
            conv: Conv::Stdcall,
            words: 4,
            port: Port::Exec(game_main),
        },
    );
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

// ---- WinMain: `Game_Main` (`0x0062ee50`) ----

mod gm {
    //! `Game_Main`'s callees and data.
    pub const GET_THREAD_LOCALE: u32 = 0x00b0_c1b4;
    pub const SET_THREAD_UI_LANGUAGE: u32 = 0x00b0_c1b8;
    pub const SET_UNHANDLED_FILTER: u32 = 0x00b0_c190;
    pub const SH_GET_FOLDER_PATH_W: u32 = 0x00b0_c628;
    pub const CREATE_MUTEX_W: u32 = 0x00b0_c198;
    pub const GET_LAST_ERROR: u32 = 0x00b0_c030;
    pub const SYS_FREE_STRING: u32 = 0x00b0_c61c;
    pub const SYS_ALLOC_STRING_LEN: u32 = 0x00b0_c620;
    pub const MESSAGE_BOX_W: u32 = 0x00b0_c684;
    pub const CLOSE_HANDLE: u32 = 0x00b0_c0dc;
    pub const GET_COMMAND_LINE_W: u32 = 0x00b0_c180;
    pub const COMMAND_LINE_TO_ARGV_W: u32 = 0x00b0_c62c;
    pub const WIDE_CHAR_TO_MULTI_BYTE: u32 = 0x00b0_c1b0;
    pub const LOCAL_FREE: u32 = 0x00b0_c1a8;
    pub const PEEK_MESSAGE_A: u32 = 0x00b0_c6a0;
    pub const POST_MESSAGE_A: u32 = 0x00b0_c6ac;
    pub const TRANSLATE_MESSAGE: u32 = 0x00b0_c690;
    pub const DISPATCH_MESSAGE_A: u32 = 0x00b0_c694;
    pub const STEAM_SHUTDOWN: u32 = 0x00b0_c8b0;
    /// The crash filter, the documents folder maker, the settings
    /// (constructor, load, dtor, the thread language), the ATL string.
    pub const CRASH_FILTER: u32 = 0x0062_68e0;
    pub const CREATE_DOCUMENTS: u32 = 0x0062_ac90;
    pub const SETTINGS_CTOR: u32 = 0x0040_1880;
    pub const SETTINGS_LOAD: u32 = 0x0040_1cb0;
    pub const SETTINGS_DTOR: u32 = 0x0040_1920;
    pub const SETTINGS_LANGUAGE: u32 = 0x0062_cad0;
    pub const ATL_STRING: u32 = 0x0061_b270;
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const NEW_ARRAY: u32 = 0x0063_04c0;
    pub const DELETE_ARRAY: u32 = 0x0063_0540;
    pub const MEMSET: u32 = 0x0094_964c;
    /// The application's start-up and frame, the renderer sync, the
    /// logging, the exit, Steam.
    pub const SET_ANISOTROPY: u32 = 0x0058_de30;
    pub const PARSE_COMMAND_LINE: u32 = 0x0062_adc0;
    pub const STRLEN: u32 = 0x0060_f930;
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    pub const STEAM_INIT: u32 = 0x0062_3470;
    pub const APP_STARTUP: u32 = 0x0062_9a40;
    pub const APP_SYNC: u32 = 0x0062_d3d0;
    pub const TTY_WRITE: u32 = 0x0064_8850;
    pub const PHYRE_EXIT: u32 = 0x0043_0d60;
    pub const GET_TIME: u32 = 0x0042_fd80;
    pub const APP_FRAME: u32 = 0x0062_e340;
    pub const RENDER_SYNC: u32 = 0x005b_c3f0;
    /// Data: the folder's suffix (0x7a bytes), the mutex name, the
    /// caption, the log formats; the settings, the application, its
    /// window; the frame limiter's words and constants.
    pub const FOLDER_SUFFIX: u32 = 0x00b4_0eb8;
    pub const MUTEX_NAME: u32 = 0x00b4_0f38;
    pub const CAPTION: u32 = 0x00b3_f9fc;
    pub const FMT_USAGE: u32 = 0x00b4_0dc0;
    pub const FMT_USAGE2: u32 = 0x00b4_0e24;
    pub const FMT_EMPTY: u32 = 0x00b7_cae0;
    pub const FMT_SYNC: u32 = 0x00b4_0cc4;
    pub const FMT_BYE: u32 = 0x00b4_0f84;
    pub const SETTINGS: u32 = 0x022f_b504;
    pub const APP: u32 = 0x00cc_9cd8;
    pub const HWND: u32 = 0x00cc_9ce8;
    pub const WINDOWED_2: u32 = 0x00cc_9cd1;
    pub const QUIT_ARMED: u32 = 0x00c3_0f90;
    pub const START_TIME: u32 = 0x00c9_0278;
    pub const LIMITER_LAST: u32 = 0x00cc_9d00;
    pub const VSYNCS: u32 = 0x00c3_0e88;
    pub const VSYNC_SECONDS: u32 = 0x00b4_12d8;
    pub const VSYNC_RATE: u32 = 0x00b4_1300;
    pub const MINUS_THOUSAND: u32 = 0x00b4_1328;
}

fn cdecl(e: &mut Exec, f: u32, words: &[u32]) -> Option<Ret> {
    let args = Args {
        stack: words.to_vec(),
        ..Args::default()
    };
    e.call(f, Conv::Cdecl, &args).ok()
}

fn this(e: &mut Exec, f: u32, ecx: u32, words: &[u32]) -> Option<Ret> {
    let args = Args {
        ecx,
        stack: words.to_vec(),
        ..Args::default()
    };
    e.call(f, Conv::Thiscall, &args).ok()
}

fn f64_at(e: &Exec, at: u32) -> f64 {
    e.mem().read(at)
}

/// `[VSYNCS]` as `fild` and the unsigned fix-up make it.
fn vsyncs(e: &Exec) -> f64 {
    f64::from(word(e, gm::VSYNCS))
}

/// The argument words' UTF-8 copies (`new[]` blocks, NULL for an empty
/// one), `argc` read from `argc_at`.
fn utf8_argv(e: &mut Exec, wide: u32, argc_at: u32) -> Option<u32> {
    let argc = word(e, argc_at);
    let bytes = argc.saturating_mul(4);
    let arr = cdecl(e, gm::NEW_ARRAY, &[bytes])?.eax;
    cdecl(e, gm::MEMSET, &[arr, 0, argc.wrapping_mul(4)])?;
    for i in 0..argc.cast_signed().max(0).cast_unsigned() {
        let w = word(e, wide.wrapping_add(4 * i));
        let mut len = 0u32;
        while e.mem().read::<u16>(w.wrapping_add(2 * len)) != 0 {
            len += 1;
        }
        let n = import(
            e,
            gm::WIDE_CHAR_TO_MULTI_BYTE,
            Conv::Stdcall,
            &[0, 0, w, len, 0, 0, 0, 0],
        )?
        .eax;
        if n.cast_signed() > 0 {
            let p = cdecl(e, gm::NEW_ARRAY, &[n.wrapping_add(1)])?.eax;
            e.mem().write(arr.wrapping_add(4 * i), p);
            import(
                e,
                gm::WIDE_CHAR_TO_MULTI_BYTE,
                Conv::Stdcall,
                &[0, 0, w, len, p, n, 0, 0],
            )?;
            e.mem().write(p.wrapping_add(n), 0u8);
        }
    }
    Some(arr)
}

/// The UTF-8 argument copies and their array freed (`delete[]`).
fn free_argv(e: &mut Exec, arr: u32, argc_at: u32) -> Option<()> {
    let mut i = 0u32;
    while i.cast_signed() < word(e, argc_at).cast_signed() {
        let p = word(e, arr.wrapping_add(4 * i));
        cdecl(e, gm::DELETE_ARRAY, &[p])?;
        i += 1;
    }
    cdecl(e, gm::DELETE_ARRAY, &[arr])?;
    Some(())
}

/// `Game_Main(hInstance, hPrev, cmdline, show)` (`0x0062ee50`, stdcall,
/// `ret 0x10`; `WinMain`), as a host function (not in the 32-bit hook: the
/// hook keeps the original's loop). In the original's order: the thread's
/// UI language, the crash filter, the documents folder and the settings
/// (`GameSetting.ini` under it), the single-instance mutex (a message box
/// and 1 when another runs), the UTF-8 command line, the frame limiter's
/// start, the application's set-up from the settings, the command-line
/// help, Steam, `PApplication_Startup` (its failure: the sync log, 1),
/// the message loop with the frame limiter (`docs/frame-timing.md`; the
/// limiter's sleep is `crate::clock`'s, as the original's `Sleep` at
/// `0x0062f3bb` is there), then the shutdown (the renderer's sync, the
/// sync log, `PhyreExit` or the goodbye line, the arguments freed,
/// `SteamAPI_Shutdown`, the settings deleted). Its exit code.
fn game_main(e: &mut Exec, args: &Args) -> Ret {
    game_main_run(e, args).unwrap_or(Ret::eax(0))
}

#[allow(
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::similar_names,
    clippy::many_single_char_names
)] // the original's one function, its names
fn game_main_run(e: &mut Exec, args: &Args) -> Option<Ret> {
    // the original's frame (`ebp` = its first stack word - 8), on the game
    // stack below the caller's
    let first = if args.slots == 0 {
        let esp = e.reg(crate::exec::RegisterX86::ESP).ok()?;
        esp.wrapping_sub(0x40) & !0xf
    } else {
        args.slots
    };
    let ebp = first.wrapping_sub(8);
    let (folder, argc_at, flag_at, local, msg) = (
        ebp - 0x404,
        ebp - 0x408,
        ebp - 0x409,
        ebp - 0x424,
        ebp - 0x444,
    );
    let lang = import(e, gm::GET_THREAD_LOCALE, Conv::Stdcall, &[])?.eax;
    import(e, gm::SET_THREAD_UI_LANGUAGE, Conv::Stdcall, &[lang])?;
    import(
        e,
        gm::SET_UNHANDLED_FILTER,
        Conv::Stdcall,
        &[gm::CRASH_FILTER],
    )?;
    e.mem().write(argc_at, 0u32);
    cdecl(e, gm::CREATE_DOCUMENTS, &[])?;
    import(
        e,
        gm::SH_GET_FOLDER_PATH_W,
        Conv::Stdcall,
        &[0, 5, 0, 0, folder],
    )?;
    let mut end = folder;
    while e.mem().read::<u16>(end) != 0 {
        end += 2;
    }
    let mut suffix = vec![0u8; 0x7a];
    e.mem().read_bytes(gm::FOLDER_SUFFIX, &mut suffix);
    e.mem().write_bytes(end, &suffix);
    let block = cdecl(e, gm::OPERATOR_NEW, &[0x63c])?.eax;
    let settings = if block == 0 {
        0
    } else {
        this(e, gm::SETTINGS_CTOR, block, &[folder])?.eax
    };
    e.mem().write(gm::SETTINGS, settings);
    this(e, gm::SETTINGS_LOAD, settings, &[])?;
    cdecl(e, gm::SETTINGS_LANGUAGE, &[])?;
    let mutex = import(
        e,
        gm::CREATE_MUTEX_W,
        Conv::Stdcall,
        &[0, 1, gm::MUTEX_NAME],
    )?
    .eax;
    if import(e, gm::GET_LAST_ERROR, Conv::Stdcall, &[])?.eax == 0xb7 {
        // another instance: its message, the mutex closed, 1
        import(e, gm::SYS_FREE_STRING, Conv::Stdcall, &[0])?;
        let s = cdecl(e, gm::ATL_STRING, &[0x7835])?.eax;
        let text = if s == 0 {
            0
        } else {
            let n = u32::from(e.mem().read::<u16>(s));
            import(e, gm::SYS_ALLOC_STRING_LEN, Conv::Stdcall, &[s + 2, n])?.eax
        };
        import(
            e,
            gm::MESSAGE_BOX_W,
            Conv::Stdcall,
            &[0, text, gm::CAPTION, 0x10],
        )?;
        import(e, gm::CLOSE_HANDLE, Conv::Stdcall, &[mutex])?;
        import(e, gm::SYS_FREE_STRING, Conv::Stdcall, &[text])?;
        return Some(Ret::eax(1));
    }
    let line = import(e, gm::GET_COMMAND_LINE_W, Conv::Stdcall, &[])?.eax;
    let wide = import(
        e,
        gm::COMMAND_LINE_TO_ARGV_W,
        Conv::Stdcall,
        &[line, argc_at],
    )?
    .eax;
    if wide == 0 {
        return Some(Ret::eax(0));
    }
    let argv = utf8_argv(e, wide, argc_at)?;
    import(e, gm::LOCAL_FREE, Conv::Stdcall, &[wide])?;
    // the limiter's start: vsyncs / the rate
    let start = vsyncs(e) / f64_at(e, gm::VSYNC_RATE);
    e.mem().write(gm::LIMITER_LAST, start);
    let app = word(e, gm::APP);
    let vt = word(e, app);
    let f = word(e, vt + 4);
    this(e, f, app, &[])?;
    let s = word(e, gm::SETTINGS);
    let app = word(e, gm::APP);
    let base = app + 0x26c;
    let h: u16 = e.mem().read(s + 6);
    e.mem()
        .write(base, i32::from(h.cast_signed()).cast_unsigned());
    let w: u16 = e.mem().read(s + 4);
    e.mem()
        .write(base + 4, i32::from(w.cast_signed()).cast_unsigned());
    let v = word(e, s + 0x38);
    e.mem().write(base + 0x18, v);
    let v = u8::from(word(e, s + 0x28) != 0);
    e.mem().write(base + 0x1c, v);
    let mode = word(e, s + 8);
    e.mem().write(base + 0x2c, u8::from(mode == 1));
    e.mem().write(gm::WINDOWED_2, u8::from(mode == 2));
    // fild, fstp dword, fld, ftol2: the int through a float
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let aniso = (word(e, s + 0x2c).cast_signed() as f32) as i64 as u32;
    cdecl(e, gm::SET_ANISOTROPY, &[aniso])?;
    e.mem().write(flag_at, 0u8);
    let argc = word(e, argc_at);
    if argc.cast_signed() > 1 {
        this(e, gm::PARSE_COMMAND_LINE, flag_at, &[argc, argv, base])?;
        if e.mem().read::<u8>(flag_at) != 0 {
            let a0 = word(e, argv);
            let n = cdecl(e, gm::STRLEN, &[a0])?.eax.wrapping_add(1);
            cdecl(e, gm::PHYRE_PRINTF, &[2, gm::FMT_USAGE, n])?;
            cdecl(e, gm::PHYRE_PRINTF, &[2, gm::FMT_USAGE2])?;
            cdecl(e, gm::PHYRE_PRINTF, &[2, gm::FMT_EMPTY])?;
            free_argv(e, argv, argc_at)?;
            return Some(Ret::eax(0));
        }
    }
    cdecl(e, gm::STEAM_INIT, &[])?;
    e.mem().write(local, base);
    e.mem().write(local + 4, 0u32);
    let argc = word(e, argc_at);
    if this(e, gm::APP_STARTUP, local, &[argv, argc])?.eax != 0 {
        let r = this(e, gm::APP_SYNC, local, &[])?.eax;
        cdecl(e, gm::TTY_WRITE, &[0xa, gm::FMT_SYNC, r])?;
        if r == 0 {
            cdecl(e, gm::PHYRE_EXIT, &[])?;
        }
        free_argv(e, argv, argc_at)?;
        return Some(Ret::eax(1));
    }
    // the message loop
    e.mem().write(msg + 4, 0u32);
    import(e, gm::PEEK_MESSAGE_A, Conv::Stdcall, &[msg, 0, 0, 0, 0])?;
    while word(e, msg + 4) != 0x12 {
        let got = import(e, gm::PEEK_MESSAGE_A, Conv::Stdcall, &[msg, 0, 0, 0, 1])?.eax;
        let keep = if word(e, msg + 4) == 0x12 {
            0
        } else {
            u32::MAX
        };
        let armed = word(e, gm::QUIT_ARMED) & keep;
        e.mem().write(gm::QUIT_ARMED, armed);
        if got != 0 {
            import(e, gm::TRANSLATE_MESSAGE, Conv::Stdcall, &[msg])?;
            import(e, gm::DISPATCH_MESSAGE_A, Conv::Stdcall, &[msg])?;
            continue;
        }
        let now = cdecl(e, gm::GET_TIME, &[])?.st0.unwrap_or(0.0);
        let t = now - f64_at(e, gm::START_TIME);
        let interval = vsyncs(e) * f64_at(e, gm::VSYNC_SECONDS);
        let over = (t - f64_at(e, gm::LIMITER_LAST)) - interval;
        // fcom: `over` kept unless it is above the interval (NaN: kept)
        let m = if over > interval { interval } else { over };
        if m >= 0.0 {
            e.mem().write(gm::LIMITER_LAST, t - m);
            let mut post = this(e, gm::APP_FRAME, local, &[])?.eax != 0;
            if !post {
                let app = word(e, gm::APP);
                if e.mem().read::<u8>(app + 0x39) != 0 {
                    post = true;
                } else {
                    let f = word(e, app + 0x268);
                    post = this(e, f, app, &[])?.eax != 0;
                }
            }
            if post {
                let hwnd = word(e, gm::HWND);
                import(e, gm::POST_MESSAGE_A, Conv::Stdcall, &[hwnd, 0x10, 0, 0])?;
            }
        } else {
            // fmul by -1000.0, fistp with truncation, the low word
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let ms = (m * f64_at(e, gm::MINUS_THOUSAND)) as i64 as u32;
            crate::clock::limiter(e, ms);
        }
    }
    // the shutdown
    let app = word(e, gm::APP);
    if app != 0 {
        let at = app + 0x124;
        let vt = word(e, at);
        let f = word(e, vt + 8);
        this(e, f, at, &[])?;
        let app = word(e, gm::APP);
        this(e, gm::RENDER_SYNC, app + 0x124, &[])?;
    }
    let r = this(e, gm::APP_SYNC, local, &[])?.eax;
    cdecl(e, gm::TTY_WRITE, &[0xa, gm::FMT_SYNC, r])?;
    if r == 0 {
        cdecl(e, gm::PHYRE_EXIT, &[])?;
    } else {
        cdecl(e, gm::PHYRE_PRINTF, &[3, gm::FMT_BYE])?;
    }
    free_argv(e, argv, argc_at)?;
    import(e, gm::STEAM_SHUTDOWN, Conv::Cdecl, &[])?;
    let s = word(e, gm::SETTINGS);
    if s != 0 {
        this(e, gm::SETTINGS_DTOR, s, &[])?;
        cdecl(e, gm::OPERATOR_DELETE, &[s])?;
        e.mem().write(gm::SETTINGS, 0u32);
    }
    Some(Ret::eax(0))
}

/// A table that leaves out the entries `FFX_HOST_DISABLE` names.
struct Filtered<'a> {
    t: &'a mut Table,
    disable: Vec<String>,
}

impl Filtered<'_> {
    fn register(&mut self, addr: u32, e: Entry) {
        if !self.disable.iter().any(|n| n == e.name) {
            self.t.register(addr, e);
        }
    }
}
