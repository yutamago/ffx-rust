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

// ---- round 4 (menu-battle-script 2): the host-file and capture natives ----

/// `MSVCR110` `sprintf`'s import slot.
const SPRINTF_IAT: u32 = 0x00b0_c3dc;

/// `Native_c057_CopyToSaveRam` (`0x008781f0`): `(event)`: the event's
/// save data (`event + 0x40`, 0x68c0 bytes) copied over the save RAM
/// (`0x0112ca90`, the CRT's `memcpy`), then `0x00a44640()`; 1.
pub fn native_c057<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let p = pop(h, s);
    let ram = h.rebase(0x0112_ca90);
    call(h, 0x0094_9256, &[ram, p.wrapping_add(0x40), 0x68c0]);
    call(h, 0x00a4_4640, &[]);
    1
}

/// `Native_c056_CdRead` (`0x00878230`): `(file, sector, buf)` (popped
/// `buf`, `sector`, `file`): `TkCdRead(file, sector, buf)`; 1.
pub fn native_c056<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let buf = pop(h, s);
    let sector = pop(h, s);
    let file = pop(h, s);
    call(h, 0x0088_d140, &[file, sector, buf]);
    1
}

/// `Native_c05b_DebugPs2String` / `Native_c05c_...` (`0x00878330` /
/// `0x008783c0`): `(v)`: `sprintf(buf, fmt, worker u16 +0x2e, v)` (the
/// two formats `0x00b5c0f4` / `0x00b5c100`), `AtelPs2DebugString2(buf)`;
/// `v`. The buffer is the original's `ebp - 0x24` (the host's scratch).
fn debug_ps2_string<H: NativeHost>(h: &mut H, w: u32, s: u32, fmt: u32) -> u32 {
    let v = pop(h, s);
    let buf = h.scratch();
    let id = u32::from(h.read::<u16>(w.wrapping_add(0x2e)));
    let fmt = h.rebase(fmt);
    let f: u32 = h.read(h.rebase(SPRINTF_IAT));
    h.call_n(f, &[buf, fmt, id, v]);
    call(h, 0x0087_3c80, &[buf]);
    v
}

pub fn native_c05b<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    debug_ps2_string(h, w, s, 0x00b5_c0f4)
}

pub fn native_c05c<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    debug_ps2_string(h, w, s, 0x00b5_c100)
}

/// `Native_c000_SaveEventToHost` / `Native_c001_SaveEventFromHost`
/// (`0x00878420` / `0x00878490`): in debug mode the save event
/// (`TkGetSaveEventAddress`, 0x21ec bytes) written to / read from the
/// host file `0x00c52dc0` (`AtelPCCreate` + `AtelPCWrite` / `AtelPCOpen`
/// + `AtelPCRead`, then `AtelPCClose`); 1 when it opened, else 0.
fn save_event_host<H: NativeHost>(h: &mut H, write: bool) -> u32 {
    if !debug(h) {
        return 0;
    }
    let ev = call(h, 0x0088_c830, &[]);
    let name = h.rebase(0x00c5_2dc0);
    let fd = call(h, if write { 0x0087_9c60 } else { 0x0087_9cf0 }, &[name]);
    if fd.cast_signed() < 0 {
        return 0;
    }
    call(
        h,
        if write { 0x0087_9e50 } else { 0x0087_9de0 },
        &[fd, ev, 0x21ec],
    );
    call(h, 0x0087_9c30, &[fd]);
    1
}

pub fn native_c000<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    save_event_host(h, true)
}

pub fn native_c001<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    save_event_host(h, false)
}

/// `Native_c004_CaptureEnableKari` (`0x00878570`): `(mode, on)` (popped
/// `on`, `mode`): `Sg_CaptureEnable_Kari(mode, on)` in debug mode; 1.
pub fn native_c004<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    let mode = pop(h, s);
    if debug(h) {
        call(h, 0x0083_a4f0, &[mode, on]);
    }
    1
}

/// `Native_c005_CaptureEnableKariOn` (`0x008785b0`): `(mode)`:
/// `Sg_CaptureEnable_KariOn(mode)` in debug mode; 1.
pub fn native_c005<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let mode = pop(h, s);
    if debug(h) {
        call(h, 0x0083_a580, &[mode]);
    }
    1
}

/// `Native_c00c_LoadFieldParticle` / `Native_c00b_LoadBtlFieldParticle`
/// (`0x00878650` / `0x00878690`): `(n)`: in debug mode
/// `yiLoadFieldParticleNo(n)` / `yiLoadBattleFieldParticleNo(n)`'s `eax`
/// (after an empty `dbgPrintf`), else 1.
pub fn native_c00c<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    if debug(h) {
        return call(h, 0x0090_92e0, &[n]);
    }
    1
}

pub fn native_c00b<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let n = pop(h, s);
    if debug(h) {
        return call(h, 0x0090_9120, &[n]);
    }
    1
}

/// `Native_c015_SepTransFromHost` (`0x008787d0`): `(var)`: in debug mode
/// the host file named by the variable (`AtelGetRealVarAdrs`) read into
/// a 64-byte aligned `TkMalloc` block and handed to `SndSepTrans`, the
/// block freed; 1.
pub fn native_c015<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let name = call(h, 0x0086_c2e0, &[w, v]);
    if !debug(h) {
        return 1;
    }
    let fd = call(h, 0x0087_9cf0, &[name]);
    if fd.cast_signed() < 0 {
        return 1;
    }
    let n = call(h, 0x0087_9e10, &[fd]);
    let m = call(h, 0x0088_e650, &[n.wrapping_add(0x40)]);
    let p = call(h, 0x0088_d120, &[m, 0x40]);
    call(h, 0x0087_9de0, &[fd, p, n]);
    call(h, 0x0087_9c30, &[fd]);
    call(h, 0x0088_6ee0, &[p, n]);
    call(h, 0x0088_d520, &[m]);
    1
}

/// `Native_c018_SqsSetProjectBase` (`0x008788a0`): `(var)`: in debug mode
/// `SqsSetProjectBase(AtelGetRealVarAdrs(w, var))`'s `eax`, else 1.
pub fn native_c018<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    let p = call(h, 0x0086_c2e0, &[w, v]);
    if debug(h) {
        return call(h, 0x0081_eab0, &[p]);
    }
    1
}

/// `Native_c02a_OutputEncountStart` (`0x00878e10`): `(on)`: in debug mode
/// `TkSetOutputEncount(on)` and, still in debug mode with `on`,
/// `AtelOutputEncountStart()`; `on`.
pub fn native_c02a<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    let on = pop(h, s);
    if debug(h) {
        call(h, 0x0088_ebe0, &[on]);
        if debug(h) && on != 0 {
            call(h, 0x0088_5a80, &[]);
        }
    }
    on
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
        0x0087_81f0 => Int(native_c057),
        0x0087_8230 => Int(native_c056),
        0x0087_8330 => Int(native_c05b),
        0x0087_83c0 => Int(native_c05c),
        0x0087_8420 => Int(native_c000),
        0x0087_8490 => Int(native_c001),
        0x0087_8570 => Int(native_c004),
        0x0087_85b0 => Int(native_c005),
        0x0087_8650 => Int(native_c00c),
        0x0087_8690 => Int(native_c00b),
        0x0087_87d0 => Int(native_c015),
        0x0087_88a0 => Int(native_c018),
        0x0087_8e10 => Int(native_c02a),
        _ => return None,
    })
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 26] = [
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
    (0x0087_81f0, false),
    (0x0087_8230, false),
    (0x0087_8330, false),
    (0x0087_83c0, false),
    (0x0087_8420, false),
    (0x0087_8490, false),
    (0x0087_8570, false),
    (0x0087_85b0, false),
    (0x0087_8650, false),
    (0x0087_8690, false),
    (0x0087_87d0, false),
    (0x0087_88a0, false),
    (0x0087_8e10, false),
];
