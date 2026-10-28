//! M5 stage 1, list `script`: the debug natives no earlier round ported.
//! Most do something only in debug mode (`TkIsDebug`, `0x00887d10`, which
//! the shipped game answers with 0); the prints go to the empty
//! `dbgPrintf` (`0x0062f500`), so the ports leave them out but keep the
//! pops and the lookups the original makes for them.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use super::native::Native;
use super::run::NativeHost;
use super::{atel_get_battle_id_adrs2, atel_pop_stack_float, atel_pop_stack_integer};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call_n(f, args)
}

/// `TkIsDebug()` (`0x00887d10`).
const TK_IS_DEBUG: u32 = 0x0088_7d10;

fn debug(h: &mut impl NativeHost) -> bool {
    call(h, TK_IS_DEBUG, &[]) != 0
}

/// `Native_b002_DebugPrint` (`0x0076ea30`): a movie warning through
/// `dbgPrintf`; 0.
pub fn native_b002<H: NativeHost>(_h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    0
}

/// `Native_70b1_DebugPrint` (`0x007a4a60`): `(v)`: prints `v` and the
/// worker's battle actor byte (`*AtelGetBattleIdAdrs2(w)`, read as the
/// original reads it); 0.
pub fn native_70b1<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = atel_get_battle_id_adrs2(w);
    let _ = h.read::<u8>(id);
    pop(h, s);
    0
}

/// `Native_705f_DebugPrint` (`0x007a4c30`): `(f)`: as `0x70b1` with a float.
pub fn native_705f<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let id = atel_get_battle_id_adrs2(w);
    let _ = h.read::<u8>(id);
    atel_pop_stack_float(h, s);
    0
}

/// `Native_00f8_DebugVarMemset` (`0x00858520`): `(var, byte, count)`
/// (popped `count`, `byte`, `var`): `memset(AtelGetRealVarAdrs(w, var),
/// byte, count)` in debug mode (the address is looked up either way);
/// `count`.
pub fn native_00f8<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let count = pop(h, s);
    let byte = pop(h, s);
    let var = pop(h, s);
    let at = call(h, 0x0086_c2e0, &[w, var]);
    if debug(h) {
        // the CRT's memset (library code: stays the original's)
        call(h, 0x0094_964c, &[at, byte, count]);
    }
    count
}

/// `Native_c006_DebugClearGlobal01301804` (`0x008785e0`): `0x0083a570()`
/// in debug mode; 1.
pub fn native_c006<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if debug(h) {
        call(h, 0x0083_a570, &[]);
    }
    1
}

/// `Native_c027_DebugCollectBits` (`0x00878d20`): in debug mode
/// `TOSetCollectBit(-1)`, then `0x0088e780(0xa004 + i)` for i 0..0x1a; 0.
pub fn native_c027<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if debug(h) {
        call(h, 0x0090_6b30, &[u32::MAX]);
        for i in 0..0x1a {
            call(h, 0x0088_e780, &[0xa004 + i]);
        }
    }
    0
}

/// `Native_c028_ImportantSetAll` (`0x00878d60`): in debug mode
/// `TkMsImportantSet(0xa004 + i)` for i 0..0x1a; 1.
pub fn native_c028<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    if debug(h) {
        for i in 0..0x1a {
            call(h, 0x0088_e790, &[0xa004 + i]);
        }
    }
    1
}

/// `Native_c03b_FullItem` (`0x008790b0`): `MsDebugFullItem()`; 1.
pub fn native_c03b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0078_49d0, &[]);
    1
}

/// `Native_c03c_FullCommand` (`0x008790c0`): `MsDebugFullCommand()`; 1.
pub fn native_c03c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0078_48b0, &[]);
    1
}

/// `Native_4038_MemDump` (`0x00a77ab0`): `Sg_MemDump()`; 0.
pub fn native_4038<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x0083_bb50, &[]);
    0
}

/// `Native_401e_SgTest` (`0x00a782c0`): `(v)`: `Sg_Test(v)` (`0x008467b0`),
/// its `EAX`.
pub fn native_401e<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    call(h, 0x0084_67b0, &[v])
}

/// `Native_c034_EnableSetTexture` (`0x00878f20`): `(v)` (dropped):
/// `Ch_EnableSetTexture(1)` in debug mode; 1.
pub fn native_c034<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    pop(h, s);
    if debug(h) {
        call(h, 0x0083_0330, &[1]);
    }
    1
}

/// `Native_c022_EnableDrawGui` (`0x00878b00`): `(v)`: in debug mode
/// `Sg_EnableDrawGui(v)`'s `EAX`, else 1.
pub fn native_c022<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    if debug(h) {
        return call(h, 0x0082_0420, &[v]);
    }
    1
}

/// The Rust version of the native at `sym`, if this module has it.
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    use Native::Int;
    Some(match sym {
        0x0076_ea30 => Int(native_b002),
        0x007a_4a60 => Int(native_70b1),
        0x007a_4c30 => Int(native_705f),
        0x0085_8520 => Int(native_00f8),
        0x0087_85e0 => Int(native_c006),
        0x0087_8d20 => Int(native_c027),
        0x0087_8d60 => Int(native_c028),
        0x0087_90b0 => Int(native_c03b),
        0x0087_90c0 => Int(native_c03c),
        0x00a7_7ab0 => Int(native_4038),
        0x00a7_82c0 => Int(native_401e),
        0x0087_8f20 => Int(native_c034),
        0x0087_8b00 => Int(native_c022),
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 13] = [
    (0x0076_ea30, false),
    (0x007a_4a60, false),
    (0x007a_4c30, false),
    (0x0085_8520, false),
    (0x0087_85e0, false),
    (0x0087_8d20, false),
    (0x0087_8d60, false),
    (0x0087_90b0, false),
    (0x0087_90c0, false),
    (0x00a7_7ab0, false),
    (0x00a7_82c0, false),
    (0x0087_8f20, false),
    (0x0087_8b00, false),
];
