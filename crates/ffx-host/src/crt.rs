//! `FFX.exe`'s statically linked C runtime helpers that ports call, as
//! host functions in the function table (docs/host.md, "The C runtime
//! DLLs"): the compiler's array construction and destruction loops and the
//! `atexit` registration. Each ran in Unicorn for every call a port made
//! (gate run 1: 52 k calls each of the array loops, 19 k of `onexit`).
//!
//! The rest of that family (`security_check_cookie`, `alloca_probe*`,
//! `ftol2_sse`, `SEH_prolog4`/`SEH_epilog4`) only emulated code calls: a
//! port's own frame needs none of them. They stay in Unicorn with their
//! callers, since an entry would only add an emulator stop to every call.

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `eh_vector_constructor_iterator` (`0x0094966a`).
pub const EH_VECTOR_CTOR: u32 = 0x0094_966a;
/// `eh_vector_destructor_iterator` (`0x009496d2`).
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
/// `onexit` (`0x00949162`).
pub const ONEXIT: u32 = 0x0094_9162;
/// `atexit` (`0x00949207`).
pub const ATEXIT: u32 = 0x0094_9207;

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

/// Calls the `thiscall` `f` on `this` (no stack words); `false` when the
/// thread stopped.
fn call_this(e: &mut Exec, f: u32, this: u32) -> bool {
    let args = Args {
        ecx: this,
        ..Args::default()
    };
    e.call(f, Conv::Thiscall, &args).is_ok()
}

/// `eh_vector_constructor_iterator(ptr, size, count, ctor, dtor)`, stdcall
/// (`ret 0x14`): `ctor` (thiscall) on each of the `count` elements from
/// `ptr` up, `size` bytes apart (a signed count: none for `count <= 0`).
/// The original's SEH frame destroys the elements built so far with `dtor`
/// when a constructor throws (`ArrayUnwind`); no constructor the ports hand
/// it throws, and a stop ends the loop. Returns 1 in `eax`, as the
/// original leaves it.
fn eh_vector_ctor(e: &mut Exec, args: &Args) -> Ret {
    let (ptr, size, count, ctor) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let mut p = ptr;
    for _ in 0..count.cast_signed().max(0) {
        if !call_this(e, ctor, p) {
            return Ret::eax(0);
        }
        p = p.wrapping_add(size);
    }
    Ret::eax(1)
}

/// `eh_vector_destructor_iterator(ptr, size, count, dtor)`, stdcall
/// (`ret 0x10`): `dtor` (thiscall) on each of the `count` elements, last
/// first (`count` signed: none for `count <= 0`). Returns 1 in `eax`.
fn eh_vector_dtor(e: &mut Exec, args: &Args) -> Ret {
    let (ptr, size, count, dtor) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let mut p = ptr.wrapping_add(size.wrapping_mul(count));
    for _ in 0..count.cast_signed().max(0) {
        p = p.wrapping_sub(size);
        if !call_this(e, dtor, p) {
            return Ret::eax(0);
        }
    }
    Ret::eax(1)
}

/// `onexit(f)`, cdecl: the image's `atexit` table (`0x025d61ac`/`b0`,
/// encoded pointers) through `__dllonexit`, or `_onexit` once it is gone
/// (-1). Both imports are the run's own here (`imports::onexit`: `f` to
/// [`crate::exec::Shared::at_exit`], `f` back) and the pointer coding is
/// the identity, so the table's words come back unchanged: `f` is
/// registered and returned.
fn onexit(e: &mut Exec, args: &Args) -> Ret {
    let f = a(args, 0);
    if let Ok(mut l) = e.shared().at_exit.lock() {
        l.push(f);
    }
    Ret::eax(f)
}

/// `atexit(f)`, cdecl: 0 when [`onexit`] registered it (a non-zero
/// result), else -1.
fn atexit(e: &mut Exec, args: &Args) -> Ret {
    let r = onexit(e, args).eax;
    Ret::eax(if r == 0 { u32::MAX } else { 0 })
}

/// Registers the helpers (over the `ported!` entries; none exist there).
pub fn register(t: &mut Table) {
    let mut put = |addr, name, conv, words, port| {
        t.register(
            addr,
            Entry {
                name,
                conv,
                words,
                port: Port::Exec(port),
            },
        );
    };
    put(
        EH_VECTOR_CTOR,
        "eh_vector_constructor_iterator",
        Conv::Stdcall,
        5,
        eh_vector_ctor,
    );
    put(
        EH_VECTOR_DTOR,
        "eh_vector_destructor_iterator",
        Conv::Stdcall,
        4,
        eh_vector_dtor,
    );
    put(ONEXIT, "onexit", Conv::Cdecl, 1, onexit);
    put(ATEXIT, "atexit", Conv::Cdecl, 1, atexit);
}
