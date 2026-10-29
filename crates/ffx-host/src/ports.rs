//! The ports this build registers in the function table
//! (docs/host.md, "One source of truth").
//!
//! The hook's own `ported!` lists, compiled a second time by
//! `ffx-host-ports` (each line an entry: name, address, convention, stack
//! words and the shim), but the shims `tools/host-ports.py` holds back
//! (they hand the game host addresses). Two hand-written ports through
//! their portable bridges stay as the executor's own check of both
//! directions; the `ported!` entries at the same addresses replace them in
//! [`table`].

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

/// A hand-written port.
pub type HandPort = fn(&mut Exec, &Args) -> Ret;

/// The hand-registered ports: address, `ffx.sym` name, convention, stack
/// words, port.
pub const HAND: [(u32, &str, Conv, usize, HandPort); 2] = [
    (
        0x0060_7d50,
        "Path_ToLowerAscii",
        Conv::Cdecl,
        1,
        path_to_lower_ascii,
    ),
    (0x0079_a0c0, "MsCheckRange", Conv::Cdecl, 3, ms_check_range),
];

/// The hand-registered ports alone.
#[must_use]
pub fn hand() -> Table {
    let mut t = Table::new();
    for (addr, name, conv, words, port) in HAND {
        t.register(
            addr,
            Entry {
                name,
                conv,
                words,
                port: Port::Exec(port),
            },
        );
    }
    t
}

/// The `ported!` entries alone (the held ones left out).
#[must_use]
pub fn ported() -> Table {
    let mut t = Table::new();
    let (hooks, _) = ffx_host_ports::table_hooks();
    for h in hooks {
        t.register(
            h.address,
            Entry {
                name: h.name,
                conv: h.conv,
                words: h.words,
                port: Port::Routed(h.run),
            },
        );
    }
    t
}

/// Every port of this build: the hand-written ones, then the `ported!`
/// entries over them.
#[must_use]
pub fn table() -> Table {
    let mut t = hand();
    for (addr, e) in ported().entries() {
        t.register(addr, e);
    }
    t
}
