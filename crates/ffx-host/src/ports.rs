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

/// `zlib_uncompress(dst, &dst_len, src, src_len)` (`0x00405e20`):
/// `ffx_core::zlib`, as the hook's hand-written replacement; `*dst_len` is
/// updated only on success. The loads' hot spot under Unicorn (every
/// compressed block of `FFX_Data.vbf`).
fn zlib_uncompress(e: &mut Exec, args: &Args) -> Ret {
    let [dst, dst_len, src, src_len] = [0, 1, 2, 3].map(|i| args.arg(Conv::Cdecl, i));
    let mut mem = e.mem();
    let capacity: u32 = mem.read(dst_len);
    let mut input = vec![0u8; src_len as usize];
    mem.read_bytes(src, &mut input);
    let mut out = vec![0u8; capacity as usize];
    match ffx_core::zlib::uncompress(&mut out, &input) {
        Ok(written) => {
            mem.write_bytes(dst, &out[..written]);
            mem.write(dst_len, u32::try_from(written).unwrap_or(u32::MAX));
            Ret::eax(0)
        }
        Err(code) => Ret::eax(code.cast_unsigned()),
    }
}

/// A hand-written port.
pub type HandPort = fn(&mut Exec, &Args) -> Ret;

/// The hand-registered ports: address, `ffx.sym` name, convention, stack
/// words, port.
pub const HAND: [(u32, &str, Conv, usize, HandPort); 3] = [
    (
        0x0040_5e20,
        "zlib_uncompress",
        Conv::Cdecl,
        4,
        zlib_uncompress,
    ),
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

/// A comma list from the environment, or from the file it names as
/// `@path` (comma or line separated); `None`: unset or empty.
pub(crate) fn names(var: &str) -> Option<Vec<String>> {
    let mut v = std::env::var(var).ok()?;
    if let Some(path) = v.strip_prefix('@') {
        v = std::fs::read_to_string(path).ok()?.replace('\n', ",");
    }
    let l: Vec<String> = v
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect();
    (!l.is_empty()).then_some(l)
}

/// The `ported!` entries alone (the held ones left out). As the hook's
/// `FFX_HOOK_DISABLE`/`FFX_HOOK_ONLY`, `FFX_HOST_DISABLE=<names>` leaves
/// the named entries out and `FFX_HOST_ONLY=<names>` keeps only those
/// (comma lists of `ffx.sym` names, or `@file`; for A/B runs and
/// bisecting).
#[must_use]
pub fn ported() -> Table {
    let mut t = Table::new();
    let (hooks, _) = ffx_host_ports::table_hooks();
    let disable = names("FFX_HOST_DISABLE").unwrap_or_default();
    let only = names("FFX_HOST_ONLY");
    for h in hooks {
        if disable.iter().any(|n| n == h.name)
            || only
                .as_ref()
                .is_some_and(|o| !o.iter().any(|n| n == h.name))
        {
            continue;
        }
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
/// entries over them, then the clock's ports and wrappers over both
/// (`crate::clock::register`; a wrapper over a `ported!` entry calls that
/// port as its original), then the C runtime helpers (`crate::crt`).
#[must_use]
pub fn table() -> Table {
    let mut t = hand();
    for (addr, e) in ported().entries() {
        t.register(addr, e);
    }
    crate::clock::register(&mut t);
    crate::crt::register(&mut t);
    crate::crt_start::register(&mut t);
    t
}
