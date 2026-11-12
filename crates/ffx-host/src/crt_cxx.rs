//! `MSVCR110`'s C++ runtime and error exits in Rust (docs/host.md, "The
//! C runtime DLLs", m5-wp5c): `__RTDynamicCast` over the image's RTTI,
//! the `std::exception`/`bad_cast` objects, and the functions whose DLL
//! code ends the process or unwinds frames (`_purecall`, `terminate`,
//! `_CxxThrowException`, `longjmp`, the SEH handlers, `_wassert`). Without
//! the emulator nothing can unwind a Rust port's frames, so those stop the
//! thread with what the DLL would have reported, where the DLL would have
//! ended the process or run a handler.
//!
//! `__RTDynamicCast` is kept as a Rust side with the DLLs loaded too
//! (`tests/crt_dll.rs` compares it with the DLL's); the rest are Rust sides
//! only without the DLLs.

#![allow(clippy::many_single_char_names)] // the C names

use ffx_bridge::Mem;

use crate::crt_dll::dll_export;
use crate::exec::{Exec, Stop, Trap};
use crate::imports::{ImportFn, read_str};
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// Kept with the DLLs.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] =
    &[(CRT, "__RTDynamicCast", Conv::Cdecl, 5, rt_dynamic_cast)];

/// `exception(const exception &)`, `bad_cast(const char *)`,
/// `bad_cast(const bad_cast &)`, `~bad_cast()`, `~type_info()`.
const EXCEPTION_COPY: &str = "??0exception@std@@QAE@ABV01@@Z";
const BAD_CAST_NEW: &str = "??0bad_cast@std@@QAE@PBD@Z";
const BAD_CAST_COPY: &str = "??0bad_cast@std@@QAE@ABV01@@Z";
const BAD_CAST_DTOR: &str = "??1bad_cast@std@@UAE@XZ";
const TYPE_INFO_DTOR: &str = "??1type_info@@UAE@XZ";

/// Without the DLLs.
pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "_purecall", Conv::Cdecl, 0, purecall),
    (CRT, "?terminate@@YAXXZ", Conv::Cdecl, 0, terminate),
    (CRT, "_CxxThrowException", Conv::Stdcall, 2, throw),
    (CRT, "__CxxFrameHandler3", Conv::Cdecl, 4, seh_handler),
    (CRT, "_except_handler4_common", Conv::Cdecl, 6, seh_handler),
    (CRT, "_XcptFilter", Conv::Cdecl, 2, seh_handler),
    (CRT, "__crtUnhandledException", Conv::Cdecl, 1, unhandled),
    (
        CRT,
        "__crtTerminateProcess",
        Conv::Cdecl,
        1,
        terminate_process,
    ),
    (CRT, "_invoke_watson", Conv::Cdecl, 5, watson),
    (CRT, "_wassert", Conv::Cdecl, 3, wassert),
    (CRT, "_setjmp3", Conv::Cdecl, 2, setjmp3),
    (CRT, "longjmp", Conv::Cdecl, 2, longjmp),
    (CRT, EXCEPTION_COPY, Conv::Thiscall, 1, exception_copy),
    (CRT, BAD_CAST_NEW, Conv::Thiscall, 1, bad_cast_new),
    (CRT, BAD_CAST_COPY, Conv::Thiscall, 1, bad_cast_copy),
    (CRT, BAD_CAST_DTOR, Conv::Thiscall, 0, exception_dtor),
    (CRT, TYPE_INFO_DTOR, Conv::Thiscall, 0, type_info_dtor),
];

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn stop(e: &mut Exec, why: String) -> Ret {
    e.end(Stop::Failed(why, 0));
    Ret::VOID
}

/// `_purecall`: the CRT's `R6025 pure virtual function call`, then the
/// process ends.
fn purecall(e: &mut Exec, _: &Args) -> Ret {
    stop(
        e,
        format!("{CRT}!_purecall: R6025 pure virtual function call"),
    )
}

fn terminate(e: &mut Exec, _: &Args) -> Ret {
    stop(e, format!("{CRT}!terminate() (abort)"))
}

/// `_CxxThrowException(object, throw info)`: the exception's type (the
/// first catchable type's `TypeDescriptor` name) in the stop. No handler
/// can run: the frames to unwind are the ports'.
fn throw(e: &mut Exec, args: &Args) -> Ret {
    let info = a(args, 1);
    let mem = e.mem();
    let name = if info == 0 {
        "a rethrow".to_string()
    } else {
        // ThrowInfo { attributes, pmfnUnwind, pForwardCompat, pCatchableTypeArray }
        let array: u32 = mem.read(info + 0xc);
        let first: u32 = mem.read(array + 4);
        let td: u32 = mem.read(first + 4);
        read_str(&mem, td + 8, false)
    };
    stop(
        e,
        format!(
            "{CRT}!_CxxThrowException: a C++ exception of type {name} (no handler without the emulator)"
        ),
    )
}

/// The language-specific SEH handlers and the CRT start-up's filter:
/// only an exception dispatch calls them, which runs emulated code.
fn seh_handler(e: &mut Exec, _: &Args) -> Ret {
    stop(
        e,
        format!("{CRT}: an SEH handler or filter called without the emulator"),
    )
}

fn unhandled(e: &mut Exec, _: &Args) -> Ret {
    stop(
        e,
        format!("{CRT}!__crtUnhandledException: an unhandled exception"),
    )
}

/// `__crtTerminateProcess(code)`: the process ends with `code`.
fn terminate_process(e: &mut Exec, args: &Args) -> Ret {
    e.end(Stop::Milestone(format!(
        "__crtTerminateProcess({:#x})",
        a(args, 0)
    )));
    Ret::VOID
}

fn watson(e: &mut Exec, _: &Args) -> Ret {
    stop(
        e,
        format!("{CRT}!_invoke_watson: an invalid parameter (the CRT ends the process)"),
    )
}

/// `_wassert(message, file, line)` (wide strings): the CRT's assertion
/// report, then `abort`.
fn wassert(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let message = read_str(&mem, a(args, 0), true);
    let file = read_str(&mem, a(args, 1), true);
    let line = a(args, 2);
    stop(
        e,
        format!("Assertion failed: {message}, file {file}, line {line}"),
    )
}

/// `_setjmp3(buffer, count, ...)`: the first return, 0. The buffer is
/// left alone: a `longjmp` back into it can't be done (below).
fn setjmp3(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

/// `longjmp(buffer, value)`: the error exits of libvpx and Lua. Unwinding
/// to the `setjmp` caller's frame would leave the Rust frames in between:
/// the thread stops instead.
fn longjmp(e: &mut Exec, args: &Args) -> Ret {
    stop(
        e,
        format!(
            "{CRT}!longjmp({:#x}, {}) without the emulator",
            a(args, 0),
            a(args, 1)
        ),
    )
}

// ------------------------------------------------- exception objects

/// `std::exception`'s layout (VS2012): the vtable, `_Mywhat`, `_Mydofree`.
const MYWHAT: u32 = 4;
const MYDOFREE: u32 = 8;

/// The vtable the no-DLL `std::exception`s get: the scalar deleting
/// destructor and `what()` as traps.
fn exception_vtable(e: &mut Exec) -> u32 {
    if let Some(v) = crate::crt_os::cell(e, "std::exception vtable") {
        return v;
    }
    let mut code = vec![0xcc_u8, 0xcc];
    code.resize(16, 0xcc);
    let Ok(at) = e.alloc_code(&code) else {
        return 0;
    };
    e.shared().add_trap(
        at,
        Trap {
            run: exception_deleting_dtor,
            tag: 0,
            conv: Conv::Thiscall,
            words: 1,
        },
    );
    e.shared().add_trap(
        at + 1,
        Trap {
            run: exception_what,
            tag: 0,
            conv: Conv::Thiscall,
            words: 0,
        },
    );
    let table = e.alloc(8);
    let mut mem = e.mem();
    mem.write(table, at);
    mem.write(table + 4, at + 1);
    crate::crt_os::set_cell(e, "std::exception vtable", table);
    table
}

/// `exception::_Copy_str`: a copy of `what` on the heap (`_Mydofree`
/// set), or none for NULL.
fn copy_what(e: &mut Exec, this: u32, what: u32) {
    let text = {
        let mem = e.mem();
        let mut v = Vec::new();
        let mut p = what;
        while what != 0 && v.len() < 0x1_0000 {
            let b: u8 = mem.read(p);
            v.push(b);
            if b == 0 {
                break;
            }
            p = p.wrapping_add(1);
        }
        v
    };
    let (copy, free) = if what == 0 {
        (0, 0u8)
    } else {
        let at = e.alloc(u32::try_from(text.len()).unwrap_or(0));
        e.mem().write_bytes(at, &text);
        (at, 1)
    };
    let mut mem = e.mem();
    mem.write(this + MYWHAT, copy);
    mem.write(this + MYDOFREE, free);
}

/// `exception(const exception &other)`: `this`.
fn exception_copy(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let other = a(args, 0);
    let vt = exception_vtable(e);
    let (what, free): (u32, u8) = {
        let mem = e.mem();
        (mem.read(other + MYWHAT), mem.read(other + MYDOFREE))
    };
    e.mem().write(this, vt);
    if free == 0 {
        let mut mem = e.mem();
        mem.write(this + MYWHAT, what);
        mem.write(this + MYDOFREE, 0u8);
    } else {
        copy_what(e, this, what);
    }
    Ret::eax(this)
}

/// `bad_cast(const char *what)`: `this`.
fn bad_cast_new(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let vt = exception_vtable(e);
    e.mem().write(this, vt);
    copy_what(e, this, a(args, 0));
    Ret::eax(this)
}

fn bad_cast_copy(e: &mut Exec, args: &Args) -> Ret {
    exception_copy(e, args)
}

/// `~exception()`: the copied string freed.
fn exception_dtor(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let (what, free): (u32, u8) = {
        let mem = e.mem();
        (mem.read(this + MYWHAT), mem.read(this + MYDOFREE))
    };
    if free != 0 {
        e.release(what);
    }
    Ret::VOID
}

fn exception_deleting_dtor(e: &mut Exec, _: u32, args: &Args) -> Ret {
    let this = args.ecx;
    exception_dtor(e, args);
    if a(args, 0) & 1 != 0 {
        e.release(this);
    }
    Ret::eax(this)
}

/// `what()`: `_Mywhat`, or "Unknown exception".
fn exception_what(e: &mut Exec, _: u32, args: &Args) -> Ret {
    let what: u32 = e.mem().read(args.ecx + MYWHAT);
    if what != 0 {
        return Ret::eax(what);
    }
    let text = e.alloc(20);
    e.mem().write_bytes(text, b"Unknown exception\0");
    Ret::eax(text)
}

/// `~type_info()`: the undecorated name's cache (`_M_data`, `+4`) freed.
fn type_info_dtor(e: &mut Exec, args: &Args) -> Ret {
    let this = args.ecx;
    let data: u32 = e.mem().read(this + 4);
    if data != 0 {
        e.release(data);
        e.mem().write(this + 4, 0u32);
    }
    Ret::VOID
}

// ------------------------------------------------------- dynamic_cast

/// `ClassHierarchyDescriptor` attributes and `BaseClassDescriptor`'s
/// (`BCD_PRIVORPROTBASE`).
const CHD_MULTINH: u32 = 1;
const BCD_PRIVORPROTBASE: u32 = 4;

/// Whether two `TypeDescriptor`s name the same type (the same address or
/// the same decorated name at `+8`).
fn same_type(mem: &impl Mem, x: u32, y: u32) -> bool {
    if x == y {
        return true;
    }
    let name = |t: u32| {
        let mut v = Vec::new();
        let mut p = t + 8;
        loop {
            let b: u8 = mem.read(p);
            if b == 0 || v.len() > 4096 {
                return v;
            }
            v.push(b);
            p += 1;
        }
    };
    name(x) == name(y)
}

/// `PMDtoOffset(object, pmd)`: `mdisp`, plus for a virtual base `pdisp`
/// and the virtual base table's entry.
fn pmd_offset(mem: &impl Mem, object: u32, pmd: u32) -> u32 {
    let mdisp: u32 = mem.read(pmd);
    let pdisp: u32 = mem.read(pmd + 4);
    let vdisp: u32 = mem.read(pmd + 8);
    let mut off = 0u32;
    if pdisp.cast_signed() >= 0 {
        let vbtable: u32 = mem.read(object.wrapping_add(pdisp));
        off = pdisp.wrapping_add(mem.read(vbtable.wrapping_add(vdisp)));
    }
    off.wrapping_add(mdisp)
}

/// `__RTDynamicCast(p, vf_delta, source, target, is_reference)` (the
/// DLL's `0x10099e7c`): the complete object from `p`'s
/// `CompleteObjectLocator`, then for a single-inheritance hierarchy
/// (`FindSITargetTypeInstance` `0x10099bd7`) the first base descriptor of
/// the target type, if the source type follows it in the base array with
/// no private or protected base before it; the object adjusted by its
/// `PMD`.
/// NULL in, NULL out; no such base: NULL (a reference cast throws
/// `bad_cast`: the DLL, or the stop). Multiple and virtual inheritance
/// hierarchies (their source-subobject rules): the DLL, or (no DLL) the
/// same search.
fn rt_dynamic_cast(e: &mut Exec, args: &Args) -> Ret {
    let (p, vf_delta, source, target, is_ref) =
        (a(args, 0), a(args, 1), a(args, 2), a(args, 3), a(args, 4));
    if p == 0 {
        return Ret::eax(0);
    }
    let found = {
        let mem = e.mem();
        let vtable: u32 = mem.read(p);
        let col: u32 = mem.read(vtable.wrapping_sub(4));
        let offset: u32 = mem.read(col + 4);
        let cd_offset: u32 = mem.read(col + 8);
        let mut complete = p.wrapping_sub(offset);
        if cd_offset != 0 {
            complete = complete.wrapping_add(mem.read(p.wrapping_sub(cd_offset)));
        }
        let _ = vf_delta;
        let chd: u32 = mem.read(col + 0x10);
        let attributes: u32 = mem.read(chd + 4);
        if attributes & CHD_MULTINH != 0 && e.shared().dll(CRT).is_some() {
            None
        } else {
            let count = mem.read::<u32>(chd + 8).min(4096);
            let array: u32 = mem.read(chd + 0xc);
            let bcd_at = |i: u32| -> u32 { mem.read(array + 4 * i) };
            let found = (0..count).find(|&i| same_type(&mem, mem.read(bcd_at(i)), target));
            let base = found.and_then(|i| {
                for j in i + 1..count {
                    let b = bcd_at(j);
                    if mem.read::<u32>(b + 0x14) & BCD_PRIVORPROTBASE != 0 {
                        return None;
                    }
                    if same_type(&mem, mem.read(b), source) {
                        return Some(bcd_at(i));
                    }
                }
                None
            });
            Some(base.map_or(0, |b| {
                complete.wrapping_add(pmd_offset(&mem, complete, b + 8))
            }))
        }
    };
    match found {
        Some(0) if is_ref != 0 => {
            if e.shared().dll(CRT).is_some() {
                return dll_export(e, CRT, "__RTDynamicCast", Conv::Cdecl, args);
            }
            stop(
                e,
                format!("{CRT}!__RTDynamicCast: bad_cast (Bad dynamic_cast!)"),
            )
        }
        Some(r) => Ret::eax(r),
        None => dll_export(e, CRT, "__RTDynamicCast", Conv::Cdecl, args),
    }
}
