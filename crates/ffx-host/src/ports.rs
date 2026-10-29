//! The ports this build registers in the function table
//! (docs/host.md, "One source of truth").
//!
//! The target is the hook's own `ported!` lists, compiled a second time
//! with a host-side `ported!` that registers each shim here instead of
//! patching (docs/host.md lists what that needs first). Until then this
//! file registers ports by hand through their portable bridges, which is
//! enough to prove the Rust and Unicorn paths in both directions.

use ffx_bridge::Mem;

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `Path_ToLowerAscii(s)` (`0x00607d50`): `ffx_core::path`, as the hook's
/// hand-written replacement; returns the address of the NUL (or `s` for
/// NULL).
fn path_to_lower_ascii(e: &mut Exec, args: &Args) -> Ret {
    let s = args.arg(Conv::Cdecl, 0);
    if s == 0 {
        return Ret::eax(0);
    }
    let mut mem = e.mem();
    let mut bytes = Vec::new();
    let mut p = s;
    loop {
        let b: u8 = mem.read(p);
        if b == 0 {
            break;
        }
        bytes.push(b);
        p += 1;
    }
    ffx_core::path::to_lower_ascii(&mut bytes);
    mem.write_bytes(s, &bytes);
    Ret::eax(p)
}

/// `MsCheckRange(x, lo, hi)` (`0x0079a0c0`): `ffx_kernel`.
fn ms_check_range(_: &mut Exec, args: &Args) -> Ret {
    let [x, lo, hi] = [0, 1, 2].map(|i| args.arg(Conv::Cdecl, i).cast_signed());
    Ret::eax(ffx_kernel::bridge::ms_check_range(x, lo, hi).cast_unsigned())
}

/// The hand-registered ports: address, `ffx.sym` name, convention, stack
/// words, port.
pub const HAND: [(u32, &str, Conv, usize, Port); 2] = [
    (
        0x0060_7d50,
        "Path_ToLowerAscii",
        Conv::Cdecl,
        1,
        path_to_lower_ascii,
    ),
    (0x0079_a0c0, "MsCheckRange", Conv::Cdecl, 3, ms_check_range),
];

/// Every port of this build.
#[must_use]
pub fn table() -> Table {
    let mut t = Table::new();
    for (addr, name, conv, words, port) in HAND {
        t.register(
            addr,
            Entry {
                name,
                conv,
                words,
                port,
            },
        );
    }
    t
}
