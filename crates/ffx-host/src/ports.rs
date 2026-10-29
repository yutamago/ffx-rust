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

/// The hand-registered ports: address, `ffx.sym` name, convention, stack
/// words, port.
pub const HAND: [(u32, &str, Conv, usize, Port); 3] = [
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
    crate::clock::register(&mut t);
    t
}
