//! `FFX.exe`'s statically linked C runtime helpers that ports call, as
//! host functions in the function table (docs/host.md, "The C runtime
//! DLLs"): the compiler's array construction and destruction loops and the
//! `atexit` registration. Each ran in Unicorn for every call a port made
//! (gate run 1: 52 k calls each of the array loops, 19 k of `onexit`).
//!
//! The compiler's 64-bit arithmetic helpers that take their operands on
//! the stack (`__allmul`, `__alldiv`, `__aulldiv`, `__aullrem`: two
//! `i64`/`u64` words each, `ret 0x10`, the result in `edx:eax`) are host
//! functions only in the build without Unicorn ([`register_arith`]): there
//! a call from Rust has nothing else to run, while with Unicorn an entry
//! would stop the emulator at every call the game's own code makes. The
//! shifts (`__allshl`, `__aullshr`: `edx:eax` and `cl`) and `_ftol2`
//! (`ST0`) take register operands a call from Rust cannot pass
//! ([`crate::table::Args`] has no `eax`, no x87 stack); the ports compute
//! them inline, and nothing in Rust calls them.
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

/// `__allmul` (`0x00949f80`).
pub const ALLMUL: u32 = 0x0094_9f80;
/// `__alldiv` (`0x00949ea0`).
pub const ALLDIV: u32 = 0x0094_9ea0;
/// `__aulldiv` (`0x00949e30`).
pub const AULLDIV: u32 = 0x0094_9e30;
/// `__aullrem` (`0x00949db0`).
pub const AULLREM: u32 = 0x0094_9db0;

/// The two 64-bit operands: words 0..2 and 2..4, low word first.
fn operands(args: &Args) -> (u64, u64) {
    let w = |i| u64::from(a(args, i));
    (w(1) << 32 | w(0), w(3) << 32 | w(2))
}

/// `__allmul(a, b)`: the low 64 bits of the product (signed and unsigned
/// alike).
fn allmul(_: &mut Exec, args: &Args) -> Ret {
    let (x, y) = operands(args);
    Ret::u64(x.wrapping_mul(y))
}

/// `__alldiv(a, b)`: the signed quotient, toward zero. The original
/// divides the magnitudes and negates for unlike signs, so `i64::MIN / -1`
/// is `i64::MIN` (wrapping). A zero divisor faults in the original (a
/// divide error); here it gives 0.
fn alldiv(_: &mut Exec, args: &Args) -> Ret {
    let (x, y) = operands(args);
    let q = if y == 0 {
        0
    } else {
        x.cast_signed().wrapping_div(y.cast_signed())
    };
    Ret::u64(q.cast_unsigned())
}

/// `__aulldiv(a, b)`: the unsigned quotient (0 for a zero divisor, where
/// the original faults).
fn aulldiv(_: &mut Exec, args: &Args) -> Ret {
    let (x, y) = operands(args);
    Ret::u64(x.checked_div(y).unwrap_or(0))
}

/// `__aullrem(a, b)`: the unsigned remainder (0 for a zero divisor, where
/// the original faults).
fn aullrem(_: &mut Exec, args: &Args) -> Ret {
    let (x, y) = operands(args);
    Ret::u64(x.checked_rem(y).unwrap_or(0))
}

/// Registers the 64-bit arithmetic helpers (stdcall-like: 4 words, `ret
/// 0x10`). [`register`] does in the build without Unicorn; the tests
/// register them over a Unicorn table to compare with the originals.
pub fn register_arith(t: &mut Table) {
    for (addr, name, port) in [
        (ALLMUL, "__allmul", allmul as fn(&mut Exec, &Args) -> Ret),
        (ALLDIV, "alldiv", alldiv),
        (AULLDIV, "aulldiv", aulldiv),
        (AULLREM, "__aullrem", aullrem),
    ] {
        t.register(
            addr,
            Entry {
                name,
                conv: Conv::Stdcall,
                words: 4,
                port: Port::Exec(port),
            },
        );
    }
}

/// Registers the helpers (over the `ported!` entries; none exist there).
pub fn register(t: &mut Table) {
    #[cfg(not(feature = "dev-unicorn"))]
    register_arith(t);
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
