//! The Atel VM accessors (`ffx_script::vm::bridge`) against the original,
//! on two oracles started alike from a snapshot of the running game: one
//! runs `FFX.exe`'s code, the other the Rust bridge on the same memory;
//! results and every written byte must agree. The workers, stacks and
//! variables are the snapshot's own (the field snapshot has the salvage
//! ship's event running). Needs `FFX_GAME_DIR` and `FFX_SNAPSHOT`; skipped
//! otherwise.
//!
//! Float results come back in `ST0`, which the oracle reads as an `f64`:
//! they compare exactly, NaNs only as NaNs (their bits are checked where
//! the code stores them). `_ftol2_sse` pops each one.

// a and b are the two oracles (original, bridge) throughout; w, l, s a worker,
// its level and its stack
#![allow(clippy::many_single_char_names)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_script::vm::bridge::run;
use ffx_script::vm::bridge::signal;
use ffx_script::vm::{self, bridge as v, ctrl, stack, worker};

const ATEL_GET_BASIC_WORK: u32 = 0x0086_a830;
const ATEL_GET_ACTOR_TYPE: u32 = 0x0086_a7b0;
const ATEL_GET_ACTOR_ID: u32 = 0x0086_a5d0;
const ATEL_GET_CAMERA_WORK_ADRS: u32 = 0x0086_ae50;
const ATEL_GET_BATTLE_ID_ADRS2: u32 = 0x0086_a990;
const ATEL_GET_CHAR_HANDLE: u32 = 0x0086_ae80;
const ATEL_GET_POS_WORK_ADRS: u32 = 0x0086_c2b0;
const SCRIPT_WORKER_LEVEL: u32 = 0x0087_2650;
const ATEL_GET_ACTOR_NUMBER: u32 = 0x0086_a650;
const ATEL_GET_NAMESPACE_NUMBER: u32 = 0x0086_c190;
const ATEL_GET_INDEX_WORK: u32 = 0x0086_bb80;
const ATEL_GET_BIN_HDR_EXPAND_ADD_CHR: u32 = 0x0086_aa20;
const ATEL_GET_ACTOR_BUFFER_SIZE: u32 = 0x0086_a290;
const ATEL_GET_CTRL_WORK: u32 = 0x0086_afa0;
const ATEL_GET_CUR_CTRL_WORK: u32 = 0x0086_aff0;
const ATEL_CHANGE_CURRENT_CTRL: u32 = 0x0086_2d30;
const ATEL_SET_CURRENT_CTRL: u32 = 0x0086_f730;
const ATEL_GET_CTRLWORK_ACTOR_TOTAL: u32 = 0x0086_afe0;
const ATEL_GET_NOW_CTRL_TOP_ID: u32 = 0x0086_c1a0;
const ATEL_GET_TOTAL_ACTOR_NUMBER2: u32 = 0x0086_c510;
const ATEL_GET_EVENT_ACTOR_TOTAL: u32 = 0x0086_b300;
const ATEL_GET_EVENT_MAP_CAMERA_ID: u32 = 0x0086_b5d0;
const ATEL_REMOVE_CTRL_WORK: u32 = 0x0086_ed40;
const ATEL_GET_MES_WIN_WORK: u32 = 0x0086_be90;

const ATEL_PUSH_STACK_INTEGER: u32 = 0x0086_e3f0;
const ATEL_PUSH_STACK_FLOAT: u32 = 0x0086_e3c0;
const ATEL_POP_STACK_INTEGER: u32 = 0x0086_df00;
const ATEL_POP_STACK_FLOAT: u32 = 0x0086_de50;
const ATEL_FETCH_STACK_INTEGER: u32 = 0x0086_df40;

const ATEL_GET_REAL_VAR_ADRS: u32 = 0x0086_c2e0;
const ATEL_GET_VALUE_ADRS: u32 = 0x0086_c570;
const ATEL_GET_VALUE_FLOAT: u32 = 0x0086_c610;
const ATEL_GET_VALUE_INC_VALUE: u32 = 0x0086_c6e0;
const ATEL_GET_VALUE_INTEGER: u32 = 0x0086_c730;
const ATEL_GET_VAR_ADRS: u32 = 0x0086_c7d0;
const ATEL_GET_VAR_ARRAY_MAX: u32 = 0x0086_c7f0;
const ATEL_GET_VAR_ARRAY_NUMBER: u32 = 0x0086_c810;
const ATEL_GET_VAR_ARRAY_RANGE: u32 = 0x0086_c830;
const ATEL_GET_VAR_INFO_ADRS: u32 = 0x0086_c850;
const ATEL_GET_VAR_TYPE: u32 = 0x0086_c870;
const ATEL_GET_VERSION: u32 = 0x0086_c890;
const ATEL_SET_VALUE_FLOAT: u32 = 0x0087_12e0;
const ATEL_SET_VALUE_INTEGER: u32 = 0x0087_1400;
const SCRIPT_PUSH_VAR: u32 = 0x0086_e440;
const SCRIPT_PUSH_VAR_ELEM: u32 = 0x0086_e100;
const SCRIPT_STORE_VAR: u32 = 0x0086_dfb0;
const SCRIPT_PUSH_VAR_REF: u32 = 0x0086_e160;

/// `_ftol2_sse`: pops `ST0` into `EAX`.
const FTOL2: u32 = 0x0094_97e0;

fn setup() -> Option<(Oracle, Oracle)> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

/// The bridge's view of the oracle: its memory, and its emulator for the
/// callbacks the VM makes into original code.
struct Run<'a>(&'a mut Oracle);

impl Mem for Run<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl v::Host for Run<'_> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.0.call(addr, Conv::Cdecl, args).unwrap().eax
    }
}

impl run::NativeHost for Run<'_> {
    fn call_float(&mut self, addr: u32, args: &[u32]) -> f64 {
        let r = self.0.call(addr, Conv::Cdecl, args).unwrap();
        let raw = r.st0_raw.expect("a float native leaves ST0");
        // pop it, as the original's fstp does
        let ftol = self.0.rebase(FTOL2);
        self.0.call(ftol, Conv::Cdecl, &[]).unwrap();
        f80_to_f64(raw)
    }
    fn call_intrinsic2(&mut self, addr: u32, a: f64, b: f64) -> f64 {
        let at = self.0.rebase(TRAMPOLINE);
        let (x, y) = (a.to_bits(), b.to_bits());
        #[allow(clippy::cast_possible_truncation)] // the two halves
        let args = [x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32, addr];
        let r = self.0.call(at, Conv::Cdecl, &args).unwrap();
        let raw = r.st0_raw.expect("the intrinsic leaves ST0");
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        f80_to_f64(raw)
    }
    fn call_intrinsic1(&mut self, addr: u32, a: f64) -> f64 {
        let at = self.0.rebase(TRAMPOLINE);
        let x = a.to_bits();
        #[allow(clippy::cast_possible_truncation)] // the two halves
        let args = [x as u32, (x >> 32) as u32, addr];
        let r = self.0.call(at + 19, Conv::Cdecl, &args).unwrap();
        let raw = r.st0_raw.expect("the intrinsic leaves ST0");
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        f80_to_f64(raw)
    }
    fn scratch(&mut self) -> u32 {
        // the bottom of the emulated stack: no call reaches that deep, and
        // the comparisons leave the stack out, as the originals' locals are
        self.0.stack_range().start + 0x100
    }
}

/// An 80-bit x87 value as `fstp m64` stores it, NaN payloads kept (the
/// values natives leave are already rounded to 53 bits).
fn f80_to_f64(raw: [u8; 10]) -> f64 {
    let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    let sign = u64::from(se >> 15) << 63;
    if se & 0x7fff == 0x7fff && mant << 1 != 0 {
        // NaN: quiet, the top 52 fraction bits kept
        return f64::from_bits(sign | 0x7ff8_0000_0000_0000 | (mant << 1) >> 12);
    }
    let x = if se.trailing_zeros() >= 15 && mant == 0 {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)] // exact for 53-bit values
        let m = mant as f64;
        m * 2f64.powi(i32::from(se & 0x7fff) - 16383 - 63)
    };
    f64::from_bits(sign | x.to_bits())
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

/// `EAX` of the original at `f` (an `ffx.sym` address). Fails when the
/// original leaves a float in `ST0`: that is a result its callers read, so
/// the port must return it (a void port of a float function breaks them).
fn orig(a: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    let r = a
        .call(a.rebase(f), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x}{args:x?}: {e}"));
    assert!(
        r.st0_raw.is_none(),
        "{f:#x}{args:x?}: the original returns {:?} in ST0",
        r.st0
    );
    r.eax
}

/// `ST0` of the original at `f`, popped.
fn orig_float(a: &mut Oracle, f: u32, args: &[u32]) -> f64 {
    let r = a
        .call(a.rebase(f), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x}{args:x?}: {e}"));
    let x = r.st0.unwrap_or_else(|| panic!("{f:#x}: nothing in ST0"));
    orig(a, FTOL2, &[]);
    x
}

/// Whether an `ST0` result is the bridge's `f32` bits.
fn same_float(st0: f64, bits: u32) -> bool {
    let f = f32::from_bits(bits);
    if f.is_nan() {
        st0.is_nan()
    } else {
        st0.to_bits() == f64::from(f).to_bits()
    }
}

/// Every byte written must agree, except inside words that hold an address
/// in the emulated stack on both sides: original code the bridge calls
/// (a native) runs on a shallower stack than when `Script_Run` calls it,
/// so a pointer to one of its locals it leaves behind differs by the depth
/// (the game's Rust frames differ in size from the original's as well).
fn no_diff(a: &mut Oracle, b: &mut Oracle, what: &str) {
    let stack = a.stack_range();
    assert_eq!(stack, b.stack_range());
    let d: Vec<_> = a
        .diff(b)
        .unwrap()
        .into_iter()
        .filter(|&(at, _, _)| {
            let w = at & !3;
            !(stack.contains(&Mem::read::<u32>(a, w)) && stack.contains(&Mem::read::<u32>(b, w)))
        })
        .collect();
    assert_eq!(d, [], "{what}: memory");
}

/// Zeroes the top 64 KiB of both oracles' stacks, where the frames of the
/// next call go: a native called by the test and the same native's helper
/// called by the Rust version sit at different depths, so a word original
/// code reads before writing would otherwise hold different leftovers
/// (`0x007bcb80` reads a spline position and tangent that
/// `TkMtCalcSpline` leaves unwritten for short paths, for one).
fn clear_stacks(a: &mut Oracle, b: &mut Oracle) {
    let r = a.stack_range();
    let zero = vec![0u8; 0x1_0000];
    for o in [a, b] {
        Mem::write_bytes(o, r.end - 0x1_0000, &zero);
    }
}

/// The emulated VU0 registers (`g_Vu0_Q` `0x00c8f788` and the VF registers
/// from `0x00c8f790`): battle helpers load vectors with an unwritten `w`
/// into them (`0x007a9150`, behind `0x700c`/`0x7085`), whose stack
/// leftovers differ between a native the test calls and the same helper
/// called from Rust, one frame higher. They are scratch registers that
/// every use reloads.
const EE_GPR: std::ops::Range<u32> = 0x00c8_f788..0x00c8_f890;

/// [`no_diff`], leaving out [`EE_GPR`].
fn native_no_diff(a: &mut Oracle, b: &mut Oracle, what: &str) {
    let stack = a.stack_range();
    let ee = a.rebase(EE_GPR.start)..a.rebase(EE_GPR.end);
    let d: Vec<_> = a
        .diff(b)
        .unwrap()
        .into_iter()
        .filter(|&(at, _, _)| {
            let w = at & !3;
            let on_stack =
                stack.contains(&Mem::read::<u32>(a, w)) && stack.contains(&Mem::read::<u32>(b, w));
            !(ee.contains(&at) || on_stack)
        })
        .collect();
    assert_eq!(d, [], "{what}: memory");
}

/// Writes `value` at `addr` in both oracles.
fn poke<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, addr: u32, value: T) {
    Mem::write(a, addr, value);
    Mem::write(b, addr, value);
}

/// The current ctrl work and its worker count (`WORKERS`).
fn ctx(o: &Oracle) -> (u32, u32) {
    let c = v::CONTEXT.get(o);
    (c, u32::from(o.read::<u16>(c + ctrl::WORKERS)))
}

/// Every worker of the current ctrl work, from both sides alike.
fn real_workers(a: &mut Oracle, b: &mut Oracle) -> Vec<u32> {
    let (_, n) = ctx(b);
    (0..n)
        .map(|i| {
            let w = v::atel_get_basic_work(b, i.cast_signed());
            assert_eq!(orig(a, ATEL_GET_BASIC_WORK, &[i]), w, "worker {i}");
            w
        })
        .collect()
}

#[test]
fn workers_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        check_workers(&mut a, &mut b);
    }
}

fn check_workers(a: &mut Oracle, b: &mut Oracle) {
    let (c, n) = ctx(b);
    eprintln!("ctrl work {c:#x}: {n} workers");
    let n = n.cast_signed();
    let mut indices: Vec<i32> = (-3..n + 5).collect();
    indices.extend([
        0x7fff,
        0x8000,
        0xffff,
        0x1_0000,
        0x1_0005,
        -0x8000,
        -0x8001,
        i32::MIN,
    ]);
    for round in 0..3 {
        for &i in &indices {
            let arg = i.cast_unsigned();
            let ra = orig(a, ATEL_GET_BASIC_WORK, &[arg]);
            assert_eq!(ra, v::atel_get_basic_work(b, i), "AtelGetBasicWork({i})");
            // again: the cache
            let ra = orig(a, ATEL_GET_BASIC_WORK, &[arg]);
            assert_eq!(
                ra,
                v::atel_get_basic_work(b, i),
                "AtelGetBasicWork({i}) cached"
            );
            let ra = orig(a, ATEL_GET_ACTOR_TYPE, &[arg]);
            assert_eq!(ra, v::atel_get_actor_type(b, i), "AtelGetActorType({i})");
        }
        no_diff(a, b, "AtelGetBasicWork");
        // round 1: a stale cache from another ctrl number; round 2: none
        if round == 0 {
            let id = Mem::read::<u16>(b, c + ctrl::ID);
            poke(a, b, c + ctrl::ID, id ^ 0x8001);
        } else {
            let id = Mem::read::<u16>(b, c + ctrl::ID);
            poke(a, b, c + ctrl::ID, id ^ 0x8001);
            let cache = a.rebase(v::CACHE_WORK.sym);
            poke(a, b, cache, 0u32);
        }
    }
    for w in real_workers(a, b) {
        for (f, rust) in [
            (ATEL_GET_ACTOR_ID, v::atel_get_actor_id(b, w)),
            (ATEL_GET_CAMERA_WORK_ADRS, v::atel_get_camera_work_adrs(w)),
            (ATEL_GET_BATTLE_ID_ADRS2, v::atel_get_battle_id_adrs2(w)),
            (ATEL_GET_CHAR_HANDLE, v::atel_get_char_handle(b, w)),
            (ATEL_GET_POS_WORK_ADRS, v::atel_get_pos_work_adrs(b, w)),
        ] {
            assert_eq!(orig(a, f, &[w]), rust, "{f:#x}({w:#x})");
        }
        for l in -2i32..12 {
            let ra = orig(a, SCRIPT_WORKER_LEVEL, &[w, l.cast_unsigned()]);
            assert_eq!(
                ra,
                v::script_worker_level(b, w, l),
                "Script_WorkerLevel({w:#x}, {l})"
            );
        }
    }
    // the block the workers run
    let w0 = v::atel_get_basic_work(b, 0);
    orig(a, ATEL_GET_BASIC_WORK, &[0]);
    let blk = Mem::read::<u32>(b, w0 + worker::BLOCK);
    let own = Mem::read::<u32>(b, c + ctrl::BLOCK);
    eprintln!("ctrl work {c:#x}: block {blk:#x}, +0x48 {own:#x}");
    assert!(own == 0 || own == blk, "ctrl +0x48: the block or null");
    for (f, rust) in [
        (ATEL_GET_ACTOR_NUMBER, v::atel_get_actor_number(b, blk)),
        (
            ATEL_GET_NAMESPACE_NUMBER,
            v::atel_get_namespace_number(b, blk),
        ),
        (
            ATEL_GET_BIN_HDR_EXPAND_ADD_CHR,
            v::atel_get_bin_hdr_expand_add_chr(b, blk),
        ),
        (
            ATEL_GET_ACTOR_BUFFER_SIZE,
            v::atel_get_actor_buffer_size(b, blk),
        ),
    ] {
        assert_eq!(orig(a, f, &[blk]), rust, "{f:#x}({blk:#x})");
    }
    for i in 0..v::atel_get_namespace_number(b, blk) {
        let ra = orig(a, ATEL_GET_INDEX_WORK, &[blk, i]);
        assert_eq!(
            ra,
            v::atel_get_index_work(b, blk, i),
            "AtelGetIndexWork({i})"
        );
    }
    no_diff(a, b, "worker accessors");
}

#[test]
fn ctrl_works_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let (c, _) = ctx(&b);
    for i in [0, 1, 6, 7, 100, u32::MAX] {
        assert_eq!(
            orig(&mut a, ATEL_GET_CTRL_WORK, &[i]),
            v::atel_get_ctrl_work(&b, i),
            "AtelGetCtrlWork({i})"
        );
    }
    for i in 0..ctrl::COUNT {
        let cw = v::atel_get_ctrl_work(&b, i);
        assert_eq!(
            orig(&mut a, ATEL_GET_CTRLWORK_ACTOR_TOTAL, &[cw]),
            v::atel_get_ctrlwork_actor_total(&b, cw),
            "AtelGetCtrlworkActorTotal({i})"
        );
    }
    let ctrl0 = v::atel_get_ctrl_work(&b, 0);
    let top = b.read::<u16>(c + ctrl::TOP_ID);
    let flags = b.read::<u8>(ctrl0 + ctrl::FLAGS);
    for (top_id, flag) in [
        (top, flags),
        (0xffff, flags ^ 1),
        (0x7fff, flags),
        (top, flags ^ 1),
    ] {
        poke(&mut a, &mut b, c + ctrl::TOP_ID, top_id);
        poke(&mut a, &mut b, ctrl0 + ctrl::FLAGS, flag);
        for (f, rust) in [
            (ATEL_GET_CUR_CTRL_WORK, v::atel_get_cur_ctrl_work(&b)),
            (ATEL_GET_NOW_CTRL_TOP_ID, v::atel_get_now_ctrl_top_id(&b)),
            (
                ATEL_GET_TOTAL_ACTOR_NUMBER2,
                v::atel_get_total_actor_number2(&b),
            ),
            (
                ATEL_GET_EVENT_ACTOR_TOTAL,
                v::atel_get_event_actor_total(&b),
            ),
            (
                ATEL_GET_EVENT_MAP_CAMERA_ID,
                v::atel_get_event_map_camera_id(&b),
            ),
        ] {
            assert_eq!(orig(&mut a, f, &[]), rust, "{f:#x} (top {top_id:#x})");
        }
    }
    for i in [3, 0, 6] {
        let ra = orig(&mut a, ATEL_CHANGE_CURRENT_CTRL, &[i]);
        assert_eq!(
            ra,
            v::atel_change_current_ctrl(&mut b, i),
            "AtelChangeCurrentCtrl({i})"
        );
    }
    let ra = orig(&mut a, ATEL_SET_CURRENT_CTRL, &[c]);
    assert_eq!(
        ra,
        v::atel_set_current_ctrl(&mut b, c),
        "AtelSetCurrentCtrl"
    );
    no_diff(&mut a, &mut b, "current ctrl");
    // a free ctrl work, then the event's own (a copy of the flags survives in `a`/`b` alike)
    for i in [4, 0] {
        orig(&mut a, ATEL_REMOVE_CTRL_WORK, &[i]);
        v::atel_remove_ctrl_work(&mut b, i);
        no_diff(&mut a, &mut b, "AtelRemoveCtrlWork");
    }
    let bank_at = a.rebase(v::MENU_EXEC_STATUS.sym);
    for bank in [0i8, 1, -1, 3] {
        poke(&mut a, &mut b, bank_at, bank);
        for i in -3i32..12 {
            assert_eq!(
                orig(&mut a, ATEL_GET_MES_WIN_WORK, &[i.cast_unsigned()]),
                v::atel_get_mes_win_work(&b, i),
                "AtelGetMesWinWork({i}), bank {bank}"
            );
        }
    }
    no_diff(&mut a, &mut b, "ctrl works");
}

/// A slot kind, either.
fn any_kind(rng: &mut Xorshift) -> u8 {
    if rng.below(2) == 0 {
        stack::INT
    } else {
        stack::FLOAT
    }
}

/// A value for a stack slot: integers, floats and float edge cases.
#[allow(clippy::cast_precision_loss)] // any float will do
fn slot_value(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => rng.below(200).wrapping_sub(100),
        1 => (rng.below(2000) as f32 - 1000.0).to_bits() ^ (rng.below(2) << 31),
        2 => [
            0x7f80_0001, // signalling NaN
            0x7fc0_0000,
            0xffc0_0000,
            0x7f80_0000,
            0xff80_0000,
            0x4f00_0000, // 2^31
            0xcf00_0000, // -2^31
            0x4f00_0001,
            0x8000_0000,
            0x0000_0001, // denormal
        ][rng.below(10) as usize],
        3 => (rng.next() as f32 / 3.0).to_bits(),
        _ => rng.next(),
    }
}

fn is_snan(bits: u32) -> bool {
    bits & 0x7fc0_0000 == 0x7f80_0000 && bits & 0x003f_ffff != 0
}

/// A value that gets stored as a float: like [`slot_value`], without
/// signalling NaNs. The x87 quiets them on `fld` (the game masks the
/// invalid exception) and the bridge does too
/// (`ffx_script::vm::through_x87`), but Unicorn's x87 keeps them as they
/// are, so the two sides would differ on them by the emulator's fault.
fn stored_value(rng: &mut Xorshift) -> u32 {
    let x = slot_value(rng);
    if is_snan(x) { x | 0x0040_0000 } else { x }
}

#[test]
fn stacks_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    // a stack of our own with room around it, and the workers' own
    let mine = a.alloc(0x200).unwrap();
    assert_eq!(b.alloc(0x200).unwrap(), mine);
    let mine = mine + 0x80;
    let mut stacks = vec![mine];
    // the actors' (the last workers have a 0x30-byte work, no stack)
    let (c, _) = ctx(&b);
    let actors = usize::from(b.read::<u16>(c + ctrl::ACTORS));
    stacks.extend(
        real_workers(&mut a, &mut b)[..actors]
            .iter()
            .map(|w| w + worker::STACK),
    );
    let mut rng = Xorshift(0x1234_5678);
    for (k, &s) in stacks.iter().enumerate() {
        // our stack starts anywhere, including just past either end
        if s == mine {
            let d = rng.below(24).cast_signed() - 2;
            poke(&mut a, &mut b, s + stack::DEPTH, d);
        }
        for step in 0..400 {
            let what = format!("stack {k} step {step}");
            let value = slot_value(&mut rng);
            match rng.below(6) {
                0 => {
                    orig(&mut a, ATEL_PUSH_STACK_INTEGER, &[0, s, value]);
                    v::atel_push_stack_integer(&mut b, s, value);
                }
                1 => {
                    orig(&mut a, ATEL_PUSH_STACK_FLOAT, &[0, s, value]);
                    v::atel_push_stack_float(&mut b, s, value);
                }
                2 => {
                    let ra = orig(&mut a, ATEL_POP_STACK_INTEGER, &[0, s]);
                    assert_eq!(ra, v::atel_pop_stack_integer(&mut b, s), "{what}: pop int");
                }
                3 => {
                    let pop = rng.below(2);
                    let ra = orig(&mut a, ATEL_FETCH_STACK_INTEGER, &[0, pop, s]);
                    assert_eq!(ra, v::script_top_int(&mut b, pop, s), "{what}: fetch");
                }
                _ => {
                    // the worker argument is read only for the error report
                    let w = s - worker::STACK;
                    let w = if s == mine {
                        stacks[1] - worker::STACK
                    } else {
                        w
                    };
                    let ra = orig_float(&mut a, ATEL_POP_STACK_FLOAT, &[w, s]);
                    let rb = v::atel_pop_stack_float(&mut b, s);
                    assert!(same_float(ra, rb), "{what}: pop float {ra} vs {rb:#x}");
                }
            }
            if step % 50 == 0 {
                no_diff(&mut a, &mut b, &what);
            }
        }
        no_diff(&mut a, &mut b, &format!("stack {k}"));
    }
}

#[test]
fn ftol_matches_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let s = a.alloc(0x80).unwrap();
    assert_eq!(b.alloc(0x80).unwrap(), s);
    let mut rng = Xorshift(0x0bad_cafe);
    for i in 0..3000 {
        let bits = if i < 1000 {
            slot_value(&mut rng)
        } else {
            rng.next()
        };
        // a float slot popped as an integer goes through _ftol2_sse
        for o in [&mut a, &mut b] {
            Mem::write(o, s + stack::DEPTH, 1i32);
            Mem::write(o, s + stack::VALUES, bits);
            Mem::write(o, s + stack::KINDS, stack::FLOAT);
        }
        let ra = orig(&mut a, ATEL_POP_STACK_INTEGER, &[0, s]);
        assert_eq!(ra, vm::ftol(f32::from_bits(bits)), "ftol({bits:#x})");
        assert_eq!(
            ra,
            v::atel_pop_stack_integer(&mut b, s),
            "pop int {bits:#x}"
        );
    }
    no_diff(&mut a, &mut b, "ftol");
}

/// Pushes `value` as `kind` on both sides (the push itself is checked
/// above).
fn push_both(a: &mut Oracle, b: &mut Oracle, s: u32, kind: u8, value: u32) {
    let f = if kind == stack::FLOAT {
        ATEL_PUSH_STACK_FLOAT
    } else {
        ATEL_PUSH_STACK_INTEGER
    };
    orig(a, f, &[0, s, value]);
    if kind == stack::FLOAT {
        v::atel_push_stack_float(b, s, value);
    } else {
        v::atel_push_stack_integer(b, s, value);
    }
}

/// Every accessor on variable `var` of worker `w`; `values` also reads and
/// writes its elements (only where its storage is mapped).
fn check_var(
    a: &mut Oracle,
    b: &mut Oracle,
    w: u32,
    var: u32,
    stk: u32,
    rng: &mut Xorshift,
    values: bool,
) {
    let what = format!("worker {w:#x} var {var}");
    for (f, rust) in [
        (ATEL_GET_VAR_INFO_ADRS, v::atel_get_var_info_adrs(b, w, var)),
        (ATEL_GET_VAR_TYPE, v::atel_get_var_type(b, w, var)),
        (ATEL_GET_VAR_ARRAY_MAX, v::atel_get_var_array_max(b, w, var)),
        (
            ATEL_GET_VAR_ARRAY_NUMBER,
            v::atel_get_var_array_max(b, w, var),
        ),
        (
            ATEL_GET_VAR_ARRAY_RANGE,
            v::atel_get_var_array_range(b, w, var),
        ),
    ] {
        assert_eq!(orig(a, f, &[w, var]), rust, "{what}: {f:#x}");
    }
    let ra = orig(a, ATEL_GET_VAR_ADRS, &[w, var]);
    assert_eq!(
        ra,
        v::atel_get_var_adrs(&mut Run(b), w, var),
        "{what}: AtelGetVarAdrs"
    );
    let len = v::atel_get_var_array_max(b, w, var).cast_signed();
    for index in [-1, 0, 1, len - 1, len, len + 3, i32::MIN, i32::MAX] {
        let arg = index.cast_unsigned();
        let ra = orig(a, ATEL_GET_VALUE_ADRS, &[w, var, arg]);
        let at = v::atel_get_value_adrs(&mut Run(b), w, var, index);
        assert_eq!(ra, at, "{what}[{index}]: AtelGetValueAdrs");
        // the element must be there (and the next 4 bytes, for the widest)
        if !values || a.read(at, 4).is_err() {
            continue;
        }
        let ra = orig(a, ATEL_GET_VALUE_INTEGER, &[w, var, arg]);
        let rb = v::atel_get_value_integer(&mut Run(b), w, var, index);
        assert_eq!(ra, rb, "{what}[{index}]: AtelGetValueInteger");
        let ra = orig_float(a, ATEL_GET_VALUE_FLOAT, &[w, var, arg]);
        let rb = v::atel_get_value_float(&mut Run(b), w, var, index);
        assert!(
            same_float(ra, rb),
            "{what}[{index}]: AtelGetValueFloat {ra} vs {rb:#x}"
        );
        // a signalling NaN in an f32 element would come back quiet only on
        // hardware: give the emulator a quiet one
        if v::atel_get_var_type(b, w, var) == vm::var::F32 {
            let bits = Mem::read::<u32>(b, at);
            if is_snan(bits) {
                poke(a, b, at, bits | 0x0040_0000);
            }
        }
        // pushes, with the stack anywhere from empty to full
        let depth = rng.below(21).cast_signed();
        poke(a, b, stk + stack::DEPTH, depth);
        orig(a, SCRIPT_PUSH_VAR, &[w, stk, var, arg]);
        v::script_push_var(&mut Run(b), w, stk, var, index);
        push_both(a, b, stk, stack::INT, arg);
        orig(a, SCRIPT_PUSH_VAR_ELEM, &[w, stk, var]);
        v::script_push_var_elem(&mut Run(b), w, stk, var);
        push_both(a, b, stk, any_kind(rng), arg);
        orig(a, SCRIPT_PUSH_VAR_REF, &[w, stk, var]);
        v::script_push_var_ref(b, w, stk, var);
        no_diff(a, b, &format!("{what}[{index}]: pushes"));
        // stores, then the original value back
        let old = v::atel_get_value_integer(&mut Run(b), w, var, index);
        let old_bits = v::atel_get_value_float(&mut Run(b), w, var, index);
        for _ in 0..4 {
            let x = stored_value(rng);
            let clamp = rng.below(2);
            orig(a, ATEL_SET_VALUE_INTEGER, &[w, var, arg, x, clamp]);
            v::atel_set_value_integer(&mut Run(b), w, var, index, x.cast_signed(), clamp);
            let x = stored_value(rng);
            orig(a, ATEL_SET_VALUE_FLOAT, &[w, var, arg, x, clamp]);
            v::atel_set_value_float(&mut Run(b), w, var, index, x, clamp);
            no_diff(a, b, &format!("{what}[{index}]: AtelSetValue* {x:#x}"));
            for mode in [0, 1, 0x80, 0x81] {
                poke(a, b, stk + stack::DEPTH, rng.below(4).cast_signed());
                push_both(a, b, stk, stack::INT, arg);
                push_both(a, b, stk, any_kind(rng), stored_value(rng));
                let ra = orig(a, SCRIPT_STORE_VAR, &[w, stk, var, mode]);
                assert_eq!(ra, v::script_store_var(&mut Run(b), w, stk, var, mode));
                no_diff(
                    a,
                    b,
                    &format!("{what}[{index}]: Script_StoreVar mode {mode:#x}"),
                );
            }
        }
        if v::atel_get_var_type(b, w, var) == vm::var::F32 {
            orig(a, ATEL_SET_VALUE_FLOAT, &[w, var, arg, old_bits, 0]);
            v::atel_set_value_float(&mut Run(b), w, var, index, old_bits, 0);
        } else {
            orig(a, ATEL_SET_VALUE_INTEGER, &[w, var, arg, old, 0]);
            v::atel_set_value_integer(&mut Run(b), w, var, index, old.cast_signed(), 0);
        }
    }
    no_diff(a, b, &what);
}

/// The actors of the current ctrl work.
fn actors(a: &mut Oracle, b: &mut Oracle) -> Vec<u32> {
    let (c, _) = ctx(b);
    let n = usize::from(Mem::read::<u16>(b, c + ctrl::ACTORS));
    real_workers(a, b)[..n].to_vec()
}

#[test]
fn variables_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let s = a.alloc(0x80).unwrap();
    assert_eq!(b.alloc(0x80).unwrap(), s);
    let mut rng = Xorshift(0x5eed_0001);
    let mut count = 0;
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            let h = b.read::<u32>(w + worker::HEADER);
            for var in 0..u32::from(b.read::<u16>(h + 2)) {
                check_var(&mut a, &mut b, w, var, s, &mut rng, true);
                count += 1;
            }
        }
        eprintln!("ctrl work {c:#x}: {count} variables so far");
    }
    assert!(count > 0, "no variables in the snapshot");
}

/// The ctrl works in use, the snapshot's current one first.
fn in_use(b: &Oracle) -> Vec<u32> {
    let (cur, _) = ctx(b);
    let mut all = vec![cur];
    for i in 0..ctrl::COUNT {
        let c = v::atel_get_ctrl_work(b, i);
        if c != cur
            && b.read::<u8>(c + ctrl::FLAGS) & 1 != 0
            && b.read::<u16>(c + ctrl::ACTORS) != 0
        {
            all.push(c);
        }
    }
    all
}

/// Whether the current ctrl work is one of `g_AtelCtrlWorks`.
fn ctx_ok(b: &Oracle) -> bool {
    let c = v::CONTEXT.get(b);
    (0..ctrl::COUNT).any(|i| v::atel_get_ctrl_work(b, i) == c)
}

/// Makes ctrl work `c` current on both sides, the worker cache dropped.
fn make_current(a: &mut Oracle, b: &mut Oracle, c: u32) {
    let (at, cache) = (a.rebase(v::CONTEXT.sym), a.rebase(v::CACHE_WORK.sym));
    poke(a, b, at, c);
    poke(a, b, cache, 0u32);
}

#[test]
fn every_storage_and_type_matches_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let stk = a.alloc(0x80).unwrap();
    assert_eq!(b.alloc(0x80).unwrap(), stk);
    let mut rng = Xorshift(0x5eed_0002);
    let (c, _) = ctx(&b);
    let w = actors(&mut a, &mut b)[0];
    let desc = {
        let hdr = b.read::<u32>(w + worker::HEADER);
        b.read::<u32>(hdr + vm::header::VARS)
            .wrapping_add(b.read::<u32>(w + worker::BLOCK))
    };
    // storage 3 without and with the ctrl work's callback (the original
    // AtelGetRealVarAdrs itself: the offset has storage 0)
    let callback = a.rebase(ATEL_GET_REAL_VAR_ADRS);
    for cb in [0, callback] {
        poke(&mut a, &mut b, c + ctrl::VAR_ADDRESS, cb);
        for storage in 0..8u32 {
            for kind in 0..16u32 {
                for off in [0u32, 4, 0x11, 0x7f_fff0, 0xff_ffff] {
                    let word = kind << 28 | storage << 25 | off;
                    let ra = orig(&mut a, ATEL_GET_REAL_VAR_ADRS, &[w, word]);
                    let rb = v::atel_get_real_var_adrs(&mut Run(&mut b), w, word);
                    assert_eq!(ra, rb, "AtelGetRealVarAdrs({word:#x}) callback {cb:#x}");
                    // as variable 0, three elements long
                    poke(&mut a, &mut b, desc, word);
                    poke(&mut a, &mut b, desc + 4, 3u16);
                    // values only near the start of the areas (storage 1 is null in the field)
                    let values = off < 0x20 && storage != 1 && storage != 7;
                    check_var(&mut a, &mut b, w, 0, stk, &mut rng, values);
                }
            }
        }
    }
    for kind in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16, u32::MAX] {
        assert_eq!(
            orig(&mut a, ATEL_GET_VALUE_INC_VALUE, &[kind]),
            v::atel_get_value_inc_value(kind),
            "AtelGetValueIncValue({kind})"
        );
    }
    assert_eq!(orig(&mut a, ATEL_GET_VERSION, &[]), v::atel_get_version(&b));
    no_diff(&mut a, &mut b, "storages");
}

/// One small accessor: its address, and the bridge doing the same to `b`.
type Small = (u32, &'static str, fn(&mut Oracle, &[u32]) -> u32);

/// The small global accessors, with how many arguments each takes.
#[allow(clippy::too_many_lines)] // a table
fn smalls() -> Vec<(Small, usize)> {
    use ffx_script::vm::bridge::globals as g;
    let all: Vec<(Small, usize)> = vec![
        (
            (0x0087_3b00, "AtelGetMusicNumber", |b, _| {
                g::atel_get_music_number(b)
            }),
            0,
        ),
        (
            (0x0087_3d00, "AtelRegistMusicNumber", |b, x| {
                g::atel_regist_music_number(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0086_b030, "AtelGetEffectFlag", |b, x| {
                g::atel_get_effect_flag(b, x[0])
            }),
            1,
        ),
        (
            (0x0086_fa40, "AtelSetEffectFlag", |b, x| {
                g::atel_set_effect_flag(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0086_0fd0, "AtelAllClearEffectFlag", |b, _| {
                g::atel_all_clear_effect_flag(b);
                0
            }),
            0,
        ),
        (
            (0x0086_b050, "AtelGetEffectParam", |b, x| {
                g::atel_get_effect_param(b, x[0])
            }),
            1,
        ),
        (
            (0x0086_3880, "AtelClearEffectParam", |b, _| {
                g::atel_clear_effect_param(b);
                0
            }),
            0,
        ),
        (
            (0x0087_7450, "AtelSetSnd3dVolumeDiffRatio", |b, x| {
                g::atel_set_snd3d_volume_diff_ratio(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_7460, "AtelSetSnd3dVolumeTotalRatio", |b, x| {
                g::atel_set_snd3d_volume_total_ratio(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_7330, "AtelClearSnd3dWorkAll", |b, _| {
                g::atel_clear_snd3d_work_all(b);
                0
            }),
            0,
        ),
        (
            (0x0086_e820, "AtelRegistAlbhedDic", |b, x| {
                g::atel_regist_albhed_dic(b, x[0], x[1]);
                0
            }),
            2,
        ),
        (
            (0x0087_0130, "AtelSetForceBattleMapId", |b, x| {
                g::atel_set_force_battle_map_id(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_15e0, "AtelSetVibrateStreamExt", |b, x| {
                g::atel_set_vibrate_stream_ext(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_5000, "AtelCheckCallbackQue", |b, x| {
                g::atel_check_callback_que(b, x[0])
            }),
            1,
        ),
        (
            (0x0087_5180, "AtelIsCallbackQue", |b, _| {
                g::atel_is_callback_que(b)
            }),
            0,
        ),
        (
            (0x0086_fc20, "AtelSetEncountBattleEndStatus", |b, x| {
                g::atel_set_encount_battle_end_status(b, x[0], x[1], x[2]);
                0
            }),
            3,
        ),
        (
            (0x0086_cff0, "AtelInitBattleEndStatus", |b, _| {
                g::atel_init_battle_end_status(b);
                0
            }),
            0,
        ),
        (
            (0x0086_f3a0, "AtelSetBattleEndKeepRecoverType", |b, x| {
                g::atel_set_battle_end_keep_recover_type(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0086_c4d0, "AtelGetStatusWinFadeOut", |b, _| {
                g::atel_get_status_win_fade_out(b)
            }),
            0,
        ),
        (
            (0x0087_0280, "AtelSetMCFPInWater", |b, x| {
                g::atel_set_mcfp_in_water(b, x[0], x[1], x[2], x[3]);
                0
            }),
            4,
        ),
        (
            (0x0087_0f60, "AtelSetShopRatio", |b, x| {
                g::atel_set_shop_ratio(b, x[0]);
                0
            }),
            1,
        ),
        (
            (0x0086_5890, "AtelEventGetMotionIndex", |_, x| {
                g::atel_event_get_motion_index(x[0], x[1])
            }),
            2,
        ),
        (
            (0x0087_2b90, "AtelClearPreBgPage", |b, _| {
                g::atel_clear_pre_bg_page(b);
                0
            }),
            0,
        ),
        (
            (0x0087_3aa0, "AtelGetEventMode", |b, _| {
                g::atel_get_event_mode(b)
            }),
            0,
        ),
        (
            (0x0087_2b70, "AtelClearPauseFlag", |b, _| {
                g::atel_clear_pause_flag(b);
                0
            }),
            0,
        ),
        (
            (0x0086_e840, "AtelRegistBattleEndMovieFadeout", |b, x| {
                g::atel_regist_battle_end_movie_fadeout(b, x[0], x[1], x[2]);
                0
            }),
            3,
        ),
        (
            (0x0086_d4c0, "AtelInitMotionRequest", |b, _| {
                g::atel_init_motion_request(b);
                0
            }),
            0,
        ),
        (
            (0x0086_0a90, "getMegasAdding", |b, _| g::get_megas_adding(b)),
            0,
        ),
        (
            (0x0087_2b50, "AtelCheckMenuOtakaraFlag", |b, _| {
                g::atel_check_menu_otakara_flag(b)
            }),
            0,
        ),
        (
            (0x0086_be50, "AtelGetMenuMask", |b, _| {
                g::atel_get_menu_mask(b)
            }),
            0,
        ),
        (
            (0x0086_3620, "AtelClearAllMenuMask", |b, _| {
                g::atel_clear_all_menu_mask(b);
                0
            }),
            0,
        ),
        (
            (0x0087_3ac0, "AtelGetExpandFTCAdrs", |b, _| {
                g::atel_get_expand_ftc_adrs(b)
            }),
            0,
        ),
        (
            (0x0086_3f90, "AtelClearNotAuronPatchFlag", |b, _| {
                g::atel_clear_not_auron_patch_flag(b);
                0
            }),
            0,
        ),
        (
            (0x0086_3ae0, "AtelClearLoveParamDisable", |b, _| {
                g::atel_clear_love_param_disable(b);
                0
            }),
            0,
        ),
        (
            (0x0086_58b0, "AtelEventGetNowCtrlTopId", |b, _| {
                g::atel_event_get_now_ctrl_top_id(b)
            }),
            0,
        ),
        (
            (0x0086_b000, "AtelGetCurrentActorTotal", |b, _| {
                g::atel_get_current_actor_total(b)
            }),
            0,
        ),
        (
            (0x0086_afc0, "AtelGetCtrlWorkStatus", |b, x| {
                g::atel_get_ctrl_work_status(b, x[0])
            }),
            1,
        ),
        (
            (0x0086_b710, "AtelGetGideMapActorNumber", |b, _| {
                g::atel_get_gide_map_actor_number(b)
            }),
            0,
        ),
        (
            (0x0087_02e0, "AtelSetMenuExecStatus", |b, x| {
                g::atel_set_menu_exec_status(b, x[0].to_le_bytes()[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_3e60, "AtelSetMapIdPage", |b, x| {
                g::atel_set_map_id_page(b, x[0].to_le_bytes()[0]);
                0
            }),
            1,
        ),
        (
            (0x0087_0900, "AtelSetOverlayStatus", |b, x| {
                g::atel_set_overlay_status(b, x[0].to_le_bytes()[0]);
                0
            }),
            1,
        ),
    ];
    all
}

/// Arguments kept to the tables' sizes (indices into small tables), so
/// both sides read memory that is there.
fn masked(name: &str, x: u32) -> u32 {
    match name {
        "AtelGetEffectParam" => x & 3,
        "AtelCheckCallbackQue" => x & 7,
        "AtelGetCtrlWorkStatus" => x % 7,
        _ => x,
    }
}

#[test]
fn globals_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0003);
    for round in 0..40 {
        for ((f, name, rust), n) in smalls() {
            let args: Vec<u32> = (0..n)
                .map(|_| masked(name, stored_value(&mut rng)))
                .collect();
            let ra = orig(&mut a, f, &args);
            let rb = rust(&mut b, &args);
            // the setters return whatever EAX held; compare the getters only
            if !name.contains("Set")
                && !name.contains("Regist")
                && !name.contains("Clear")
                && !name.contains("Init")
            {
                assert_eq!(ra, rb, "{name}{args:x?} round {round}");
            }
            no_diff(&mut a, &mut b, name);
        }
    }
    // the worker one, on every actor
    for w in actors(&mut a, &mut b) {
        assert_eq!(
            orig(&mut a, 0x0086_a420, &[w]),
            ffx_script::vm::bridge::globals::atel_get_actor_execspace_id(&b, w),
            "AtelGetActorExecspaceID({w:#x})"
        );
    }
    // strings: overlapping and not
    let buf = a.alloc(0x100).unwrap();
    assert_eq!(b.alloc(0x100).unwrap(), buf);
    for (dst, src, text) in [
        (0, 0x80, &b"hello"[..]),
        (0, 3, b"ab"),
        (0x10, 0x12, b"xyzw"),
        (0x40, 0x40, b""),
    ] {
        for (f, cpy) in [(0x0087_1950u32, false), (0x0087_1980, true)] {
            for o in [&mut a, &mut b] {
                let mut init = vec![0u8; 0x100];
                init[..3].copy_from_slice(b"ab\0");
                init[0x80..0x80 + text.len()].copy_from_slice(text);
                init[src..src + text.len()].copy_from_slice(text);
                Mem::write_bytes(o, buf, &init);
            }
            let (d, s) = (
                buf + u32::try_from(dst).unwrap(),
                buf + u32::try_from(src).unwrap(),
            );
            let ra = orig(&mut a, f, &[d, s]);
            let rb = if cpy {
                ffx_script::vm::bridge::globals::atel_strcpy(&mut b, d, s)
            } else {
                ffx_script::vm::bridge::globals::atel_strcat(&mut b, d, s)
            };
            assert_eq!(ra, rb, "{f:#x}({dst:#x}, {src:#x})");
            no_diff(&mut a, &mut b, "strings");
        }
    }
}

/// A getter of the second batch: address, name, the bridge.
type Getter = (u32, &'static str, fn(&mut Oracle) -> u32);

/// The second batch of global accessors: the pad words, the battle end
/// and keep-play bits, the save data fields, the sound bytes.
#[allow(clippy::too_many_lines)] // a table
fn getters() -> Vec<Getter> {
    use ffx_script::vm::bridge::globals as g;
    vec![
        (0x0086_af30, "AtelGetControlerT", |b| {
            u32::from(g::atel_get_controler_t(b))
        }),
        (0x0086_af50, "AtelGetControlerRept", |b| {
            u32::from(g::atel_get_controler_rept(b))
        }),
        (0x0086_af60, "AtelGetControlerHist1c", |b| {
            u32::from(g::atel_get_controler_hist1c(b))
        }),
        (0x0086_af70, "AtelGetControler", |b| {
            u32::from(g::atel_get_controler(b))
        }),
        (0x0086_af80, "AtelGetControlerHist08", |b| {
            u32::from(g::atel_get_controler_hist08(b))
        }),
        (0x0086_c200, "AtelGetOverlayStatus", |b| {
            g::atel_get_overlay_status(b)
        }),
        (0x0087_3b80, "AtelGetVoicePan", |b| g::atel_get_voice_pan(b)),
        (0x0087_3b90, "AtelGetVagStreamVolume", |b| {
            g::atel_get_vag_stream_volume(b)
        }),
        (0x0086_a9b0, "AtelGetBattleStatus", |b| {
            g::atel_get_battle_status(b)
        }),
        (0x0086_bc30, "AtelGetKeepPlayStatusWhiteInOut", |b| {
            g::atel_get_keep_play_status_white_in_out(b)
        }),
        (0x0086_bc60, "AtelGetKeepPlayStatusNoStopFanfare", |b| {
            g::atel_get_keep_play_status_no_stop_fanfare(b)
        }),
        (0x0086_bc10, "AtelIsKeepPlayStatusClear12", |b| {
            g::atel_is_keep_play_status_clear12(b)
        }),
        (0x0086_bc20, "AtelIsKeepPlayStatusClear8", |b| {
            g::atel_is_keep_play_status_clear8(b)
        }),
        (0x0086_bc50, "AtelIsKeepPlayStatusClear11", |b| {
            g::atel_is_keep_play_status_clear11(b)
        }),
        (0x0086_c280, "AtelIsBattleEndStatusClear8", |b| {
            g::atel_is_battle_end_status_clear8(b)
        }),
        (0x0086_c290, "AtelIsBattleEndStatusClear9", |b| {
            g::atel_is_battle_end_status_clear9(b)
        }),
        (0x0086_a9a0, "AtelGetBattleSceneGroup", |b| {
            g::atel_get_battle_scene_group(b)
        }),
        (0x0086_a9c0, "AtelGetBattleStatusDummyEnc", |b| {
            g::atel_get_battle_status_dummy_enc(b)
        }),
        (0x0086_a9d0, "AtelGetBattleStatusSys", |b| {
            g::atel_get_battle_status_sys(b)
        }),
        (0x0086_d9d0, "AtelGetSave20", |b| g::atel_get_save20(b)),
        (0x0087_3b10, "AtelGetOnMemoryMovieFileNo", |b| {
            g::atel_get_on_memory_movie_file_no(b)
        }),
        (0x0087_3b20, "AtelGetSaveD8", |b| g::atel_get_save_d8(b)),
        (0x0087_3b30, "AtelGetSaveDC", |b| g::atel_get_save_dc(b)),
        (0x0087_3b40, "AtelGetRandomEncountStatus", |b| {
            g::atel_get_random_encount_status(b)
        }),
        (0x0086_c470, "AtelGetScenarioFlag", |b| {
            g::atel_get_scenario_flag(b)
        }),
        (0x0086_a7e0, "AtelGetAlbhedRikku", |b| {
            g::atel_get_albhed_rikku(b)
        }),
        (0x0086_c4b0, "AtelGetSphereMonitor", |b| {
            g::atel_get_sphere_monitor(b)
        }),
        (0x0086_c8a0, "AtelGetWaterBattleEffect", |b| {
            g::atel_get_water_battle_effect(b)
        }),
        (0x0087_3a90, "AtelGetBattleEndTagAlways", |b| {
            g::atel_get_battle_end_tag_always(b)
        }),
        (0x0087_3ae0, "AtelGetMapMarkerX", |b| {
            g::atel_get_map_marker_x(b)
        }),
        (0x0087_3af0, "AtelGetMapMarkerY", |b| {
            g::atel_get_map_marker_y(b)
        }),
    ]
}

/// A setter of the second batch: address, name, the bridge with the
/// argument.
type Setter = (u32, &'static str, fn(&mut Oracle, u32));

fn setters() -> Vec<Setter> {
    use ffx_script::vm::bridge::globals as g;
    vec![
        (0x0087_3e50, "AtelSetHalfTurnAngle", |b, x| {
            g::atel_set_half_turn_angle(b, x);
        }),
        (0x0087_3d10, "AtelSetVoicePan", |b, x| {
            g::atel_set_voice_pan(b, x.to_le_bytes()[0]);
        }),
        (0x0087_3d20, "AtelSetVagStreamVolume", |b, x| {
            g::atel_set_vag_stream_volume(b, x.to_le_bytes()[0]);
        }),
        (0x0087_3cf0, "AtelRegistExpandFTCAdrs", |b, x| {
            g::atel_regist_expand_ftc_adrs(b, x);
        }),
        (0x0087_3e40, "AtelSetEventMode", |b, x| {
            g::atel_set_event_mode(b, x.to_le_bytes()[0]);
        }),
        (0x0086_0f20, "setMegasAdding", |b, x| {
            g::set_megas_adding(b, x);
        }),
        (0x0086_fa80, "AtelSetEffectState", |b, x| {
            g::atel_set_effect_state(b, x);
        }),
        (0x0086_f3c0, "AtelSetBattleSceneGroup", |b, x| {
            g::atel_set_battle_scene_group(b, x.to_le_bytes()[0]);
        }),
        (0x0087_00c0, "AtelSetMapFadeType", |b, x| {
            g::atel_set_map_fade_type(b, x.to_le_bytes()[0]);
        }),
        (0x0087_00d0, "AtelSetMapFadeFrames", |b, x| {
            g::atel_set_map_fade_frames(b, x.to_le_bytes()[0]);
        }),
        (0x0087_3e90, "AtelSetOnMemoryMovie", |b, x| {
            g::atel_set_on_memory_movie(b, x);
        }),
        (0x0087_72e0, "AtelClearCameraMatrixFlag", |b, _| {
            g::atel_clear_camera_matrix_flag(b);
        }),
        (0x0086_38b0, "AtelClearForcePlaceId", |b, _| {
            g::atel_clear_force_place_id(b);
        }),
    ]
}

#[test]
fn more_globals_match_the_original() {
    use ffx_script::vm::bridge::globals as g;
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0004);
    let save = a.rebase(g::SAVE_DATA.sym);
    let pads = a.rebase(g::PAD_HELD.sym);
    let status = a.rebase(g::BATTLE_END_STATUS.sym);
    for round in 0..40 {
        // random state where the getters read
        if round > 0 {
            for off in (0..0x100).step_by(4).chain([0xbec]) {
                poke(&mut a, &mut b, save + off, rng.next());
            }
            for i in 0..6 {
                poke(&mut a, &mut b, pads + 4 * i, rng.next());
            }
            for i in 0..2 {
                poke(&mut a, &mut b, status + 4 * i, rng.next());
            }
            let fs = a.rebase(0x0132_5ba0) + vm::ctrl::FIELD_STATE;
            poke(&mut a, &mut b, fs, rng.next());
        }
        for (f, name, set) in setters() {
            let x = stored_value(&mut rng);
            orig(&mut a, f, &[x]);
            set(&mut b, x);
            no_diff(&mut a, &mut b, name);
        }
        for (f, name, get) in getters() {
            let ra = orig(&mut a, f, &[]);
            let rb = get(&mut b);
            // the pad getters load AX only: the rest of EAX is the caller's
            let ra = if name.contains("Controler") {
                ra & 0xffff
            } else {
                ra
            };
            assert_eq!(ra, rb, "{name} round {round}");
            no_diff(&mut a, &mut b, name);
        }
        let st0 = orig_float(&mut a, 0x0087_3ad0, &[]);
        assert!(
            same_float(st0, g::atel_get_half_turn_angle(&b)),
            "AtelGetHalfTurnAngle round {round}"
        );
        for _ in 0..4 {
            let (i, x) = (rng.below(4), rng.next());
            orig(&mut a, 0x0086_fa60, &[i, x]);
            g::atel_set_effect_param(&mut b, i, x);
            no_diff(&mut a, &mut b, "AtelSetEffectParam");
        }
    }
    // the ones that call original code
    for set in [true, false] {
        let (f, rb) = if set {
            (0x0087_3e80, g::atel_set_menu_otakara_flag(&mut Run(&mut b)))
        } else {
            (
                0x0087_2b60,
                g::atel_clear_menu_otakara_flag(&mut Run(&mut b)),
            )
        };
        assert_eq!(orig(&mut a, f, &[]), rb, "{f:#x}");
        no_diff(&mut a, &mut b, "menu otakara flag");
    }
    for i in 0..20i32 {
        assert_eq!(
            orig(&mut a, 0x0086_ae00, &[i.cast_unsigned()]),
            g::atel_get_bind_pc_from_id(&mut b, i),
            "AtelGetBindPCFromID({i})"
        );
        no_diff(&mut a, &mut b, "AtelGetBindPCFromID");
    }
    // natives' table entries, in range and past it
    for _ in 0..200 {
        let id = rng.below(16) << 12 | rng.below(0x1000);
        assert_eq!(
            orig(&mut a, 0x0087_7770, &[id]),
            run::script_native_entry(&b, id),
            "Script_NativeEntry({id:#x})"
        );
    }
}

#[test]
fn atel_init_actor_matches_the_original() {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    for c in ctrls {
        // the callbacks take the run lock and reach CRT imports
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let ra = orig(&mut a, 0x0086_cae0, &[]);
        let rb = ffx_script::vm::bridge::globals::atel_init_actor(&mut Run(&mut b));
        assert_eq!(ra, rb, "AtelInitActor on {c:#x}");
        no_diff(&mut a, &mut b, "AtelInitActor");
    }
}

const SCRIPT_RUN: u32 = 0x0086_41e0;

/// A pair of oracles on the snapshot with the run lock stubbed (the mutex
/// is a Windows object the emulator does not have; both sides call the
/// same stubs).
fn run_pair(pages: &std::sync::Arc<Pages>, exe: &std::path::Path) -> (Oracle, Oracle) {
    let mut pair = (
        Oracle::from_snapshot(exe, pages).unwrap(),
        Oracle::from_snapshot(exe, pages).unwrap(),
    );
    let image = std::fs::read(exe).unwrap();
    let pe = goblin::pe::PE::parse(&image).unwrap();
    let mut sizes: Vec<usize> = CRT.iter().map(|(_, c)| c.len()).collect();
    sizes.extend(UNINIT.iter().map(|u| u.stub.len()));
    let caves = caves(&image, &pe, &sizes);
    assert_eq!(caves.len(), sizes.len(), "room for every stub");
    for o in [&mut pair.0, &mut pair.1] {
        // the run lock, and REQ's debug hook (0x0067a9b0: it spins on a
        // sound thread flag, forever in the emulator in the battle)
        for f in [run::MUTEX_LOCK, run::MUTEX_UNLOCK, signal::REQ_HOOK] {
            let at = o.rebase(f);
            o.stub_return(at, 0).unwrap();
        }
        // CRT imports the oracle has no stub for, written into int3
        // padding of the image (executable there), the IAT pointed at them
        for (&(name, code), &cave) in CRT.iter().zip(&caves) {
            let at = o.image_base() + cave;
            Mem::write_bytes(o, at, code);
            for imp in pe.imports.iter().filter(|i| i.name == name) {
                let slot = o.image_base() + u32::try_from(imp.offset).unwrap();
                Mem::write(o, slot, at);
            }
        }
        for &(at, bytes) in &ZEROED {
            let at = o.rebase(at);
            Mem::write_bytes(o, at, bytes);
        }
        // the stack words natives read before writing: a call at each
        // site into a stub that zeroes them, then does what the site did
        for (u, &cave) in UNINIT.iter().zip(&caves[CRT.len()..]) {
            let at = o.image_base() + cave;
            let mut stub = u.stub.to_vec();
            if let Some((off, sym)) = u.fixup {
                stub[off..off + 4].copy_from_slice(&o.rebase(sym).to_le_bytes());
            }
            Mem::write_bytes(o, at, &stub);
            let site = o.rebase(u.site);
            let mut call = vec![0xe8];
            call.extend_from_slice(&at.wrapping_sub(site + 5).to_le_bytes());
            call.resize(u.site_len, 0x90);
            Mem::write_bytes(o, site, &call);
        }
    }
    pair
}

/// A stack word original code reads before anything writes it: what it
/// holds is whatever earlier calls left there, which differs between
/// `Script_Run` and the bridge's calls (another depth, other callers). Both
/// oracles zero it through a stub called in place of `site_len` bytes at
/// `site`; the stub ends by doing what those bytes did.
struct Uninit {
    site: u32,
    site_len: usize,
    stub: &'static [u8],
    /// An absolute address to rebase into the stub: offset, `ffx.sym`
    /// address.
    fixup: Option<(usize, u32)>,
}

/// In place: the three loads with which `AtelRegistSignalMain`
/// (`0x0086ea00`), while the current ctrl work has header tables (`+0x1f8`,
/// battle ctrl works 2 and 3), takes a signal's header words from the
/// request; every caller leaves those words unwritten on its stack, and the
/// Rust callers pass 0 ([`ffx_script::vm::bridge::signal::Request::new`]).
/// `mov ecx, [esi+0x14]`, `mov eax, [esi+0x18]`, `mov esi, [esi+0x1c]`
/// become `xor reg, reg; nop`.
const ZEROED: [(u32, &[u8]); 3] = [
    (0x0086_ea84, &[0x31, 0xc9, 0x90]),
    (0x0086_ea87, &[0x31, 0xc0, 0x90]),
    (0x0086_ea8a, &[0x31, 0xf6, 0x90]),
];

const UNINIT: [Uninit; 8] = [
    // 0x007bb6a0 (Native_CamSetPos and its "ref" twin) hands Cam_SetPos a
    // vector of three popped floats whose w ([ebp-8]) it never writes;
    // Cam_SetPos copies it into the camera and the VU0 registers. Site: the
    // prologue's cookie load. Stub: mov dword [ebp-8], 0; mov eax,
    // [__security_cookie]; ret
    Uninit {
        site: 0x007b_b6a6,
        site_len: 5,
        stub: &[
            0xc7, 0x45, 0xf8, 0x00, 0x00, 0x00, 0x00, 0xa1, 0, 0, 0, 0, 0xc3,
        ],
        fixup: Some((8, 0x00c6_13d8)),
    },
    // 0x007bab90 (Native_CamRefSetTarget's) does the same with the vector
    // at [ebp-0x14] it hands 0x007beeb0: w at [ebp-8]. Same site and stub.
    Uninit {
        site: 0x007b_ab96,
        site_len: 5,
        stub: &[
            0xc7, 0x45, 0xf8, 0x00, 0x00, 0x00, 0x00, 0xa1, 0, 0, 0, 0, 0xc3,
        ],
        fixup: Some((8, 0x00c6_13d8)),
    },
    // MsGetTargetDist (0x007a9a40, behind Native_702e) takes the target
    // and actor positions into [ebp-0x28..-0x8] and reads words of them
    // the callees leave unwritten. Same site; the stub zeroes the nine
    // words: push edi; lea edi, [ebp-0x28]; xor eax, eax; push ecx;
    // mov ecx, 9; rep stosd; pop ecx; pop edi; mov eax, [cookie]; ret
    Uninit {
        site: 0x007a_9a46,
        site_len: 5,
        stub: &[
            0x57, 0x8d, 0x7d, 0xd8, 0x31, 0xc0, 0x51, 0xb9, 0x09, 0, 0, 0, 0xf3, 0xab, 0x59, 0x5f,
            0xa1, 0, 0, 0, 0, 0xc3,
        ],
        fixup: Some((17, 0x00c6_13d8)),
    },
    // Native_702c (0x007a45c0) reads the pair 0x00794d20 fills at
    // [ebp-0x34]/[ebp-0x30] (unwritten for some actors) into the actor's
    // +0x490/+0x494; the stub zeroes [ebp-0x54..-0x8], 19 words, the same
    // way as the one above.
    Uninit {
        site: 0x007a_45c6,
        site_len: 5,
        stub: &[
            0x57, 0x8d, 0x7d, 0xac, 0x31, 0xc0, 0x51, 0xb9, 0x13, 0, 0, 0, 0xf3, 0xab, 0x59, 0x5f,
            0xa1, 0, 0, 0, 0, 0xc3,
        ],
        fixup: Some((17, 0x00c6_13d8)),
    },
    // MsSetChrDirTarget (0x00795eb0, behind Native_7003/7006 and others)
    // stores the angle 0x00794d20 leaves at [ebp-0x18] into the actor's
    // +0x490, unwritten for some targets; the stub zeroes [ebp-0x1c..-0x8],
    // 6 words.
    Uninit {
        site: 0x0079_5eb6,
        site_len: 5,
        stub: &[
            0x57, 0x8d, 0x7d, 0xe4, 0x31, 0xc0, 0x51, 0xb9, 0x06, 0, 0, 0, 0xf3, 0xab, 0x59, 0x5f,
            0xa1, 0, 0, 0, 0, 0xc3,
        ],
        fixup: Some((17, 0x00c6_13d8)),
    },
    // 0x007a8720 (MsGetTargetDir, behind Native_708f) reads the angle
    // 0x00794d20 leaves at [ebp-4] (unwritten for some actors), and
    // [ebp-8]. Site: sub esp, 8; push edi; push dword [ebp+8]. Stub: pop
    // eax; mov dword [ebp-4], 0; mov dword [ebp-8], 0; then those three;
    // jmp eax (EAX is free until the call that follows)
    Uninit {
        site: 0x007a_8723,
        site_len: 7,
        stub: &[
            0x58, 0xc7, 0x45, 0xfc, 0, 0, 0, 0, 0xc7, 0x45, 0xf8, 0, 0, 0, 0, 0x83, 0xec, 0x08,
            0x57, 0xff, 0x75, 0x08, 0xff, 0xe0,
        ],
        fixup: None,
    },
    // 0x007a87f0 (the turn steps behind Native_702c) reads the direction
    // MsGetChrDir leaves at [ebp-8] and the angle at [ebp-4], unwritten
    // for some actors. Site: sub esp, 8; lea eax, [ebp-8]. Stub: pop ecx;
    // mov dword [ebp-4], 0; mov dword [ebp-8], 0; those two; jmp ecx
    Uninit {
        site: 0x007a_87f3,
        site_len: 6,
        stub: &[
            0x59, 0xc7, 0x45, 0xfc, 0, 0, 0, 0, 0xc7, 0x45, 0xf8, 0, 0, 0, 0, 0x83, 0xec, 0x08,
            0x8d, 0x45, 0xf8, 0xff, 0xe1,
        ],
        fixup: None,
    },
    // Light_Set (0x00836520, Native_LightSet's) hands Sg_LightSetColor and
    // Sg_LightSetDir a vector at [ebp-0x1c] whose w ([ebp-0x10]) it never
    // writes. Site: sub esp, 0x1c; fild dword [ebp+0xc]. Stub: mov dword
    // [ebp-0x10], 0; pop eax; sub esp, 0x1c; fild dword [ebp+0xc]; jmp eax
    // (EAX is free until the lea that follows)
    Uninit {
        site: 0x0083_6523,
        site_len: 6,
        stub: &[
            0xc7, 0x45, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x58, 0x83, 0xec, 0x1c, 0xdb, 0x45, 0x0c,
            0xff, 0xe0,
        ],
        fixup: None,
    },
];

/// x86 for the imports natives reach that the oracle has no stub for: CRT
/// functions (cdecl) and two Win32 waits (stdcall), the same on both sides.
const CRT: [(&str, &[u8]); 12] = [
    // fld1; fpatan; ret
    ("_CIatan", &[0xd9, 0xe8, 0xd9, 0xf3, 0xc3]),
    // the _CI* helpers take and return ST0 (ST1 too for atan2): one x87
    // instruction each
    ("_CIcos", &[0xd9, 0xff, 0xc3]),
    ("_CIsin", &[0xd9, 0xfe, 0xc3]),
    ("_CIsqrt", &[0xd9, 0xfa, 0xc3]),
    ("_CIatan2", &[0xd9, 0xf3, 0xc3]),
    // fptan; fstp st0; ret
    ("_CItan", &[0xd9, 0xf2, 0xdd, 0xd8, 0xc3]),
    // _isnan(double): fld qword [esp+4]; fucomip st0, st0; fstp st0;
    // setp al; movzx eax, al; ret
    (
        "_isnan",
        &[
            0xdd, 0x44, 0x24, 0x04, 0xdf, 0xe8, 0xdd, 0xd8, 0x0f, 0x9a, 0xc0, 0x0f, 0xb6, 0xc0,
            0xc3,
        ],
    ),
    // _finite(double): mov eax, [esp+8]; and eax, 0x7ff00000; cmp eax,
    // 0x7ff00000; setne al; movzx eax, al; ret
    (
        "_finite",
        &[
            0x8b, 0x44, 0x24, 0x08, 0x25, 0x00, 0x00, 0xf0, 0x7f, 0x3d, 0x00, 0x00, 0xf0, 0x7f,
            0x0f, 0x95, 0xc0, 0x0f, 0xb6, 0xc0, 0xc3,
        ],
    ),
    // strstr(hay, needle), naive: the first match or null
    (
        "strstr",
        &[
            0x56, 0x57, 0x8b, 0x4c, 0x24, 0x0c, 0x8b, 0x54, 0x24,
            0x10, // save; ecx hay, edx needle
            0x89, 0xce, 0x89, 0xd7, // outer: esi = ecx, edi = edx
            0x8a, 0x07, 0x84, 0xc0, 0x74, 0x10, // inner: al = *edi; end of needle: found
            0x3a, 0x06, 0x75, 0x04, 0x46, 0x47, 0xeb,
            0xf2, // mismatch: next; else step, inner
            0x80, 0x39, 0x00, 0x74, 0x08, 0x41, 0xeb,
            0xe6, // next: end of hay: none; ecx++, outer
            0x89, 0xc8, 0x5f, 0x5e, 0xc3, // found: eax = ecx
            0x31, 0xc0, 0x5f, 0x5e, 0xc3, // none: eax = 0
        ],
    ),
    // printf(fmt, ...): the debug lines of the natives; 0 (xor eax, eax; ret)
    ("printf", &[0x31, 0xc0, 0xc3]),
    // WaitForSingleObject(handle, ms): WAIT_OBJECT_0 (xor eax, eax; ret 8)
    ("WaitForSingleObject", &[0x31, 0xc0, 0xc2, 0x08, 0x00]),
    // ReleaseMutex(handle): TRUE (mov eax, 1; ret 4)
    (
        "ReleaseMutex",
        &[0xb8, 0x01, 0x00, 0x00, 0x00, 0xc2, 0x04, 0x00],
    ),
];

/// Room in `.text` for each of `sizes`: runs of `int3` padding at least two
/// bytes longer, as image offsets one byte in (so a stub never touches the
/// code around it).
fn caves(image: &[u8], pe: &goblin::pe::PE, sizes: &[usize]) -> Vec<u32> {
    let text = pe
        .sections
        .iter()
        .find(|s| s.name().ok() == Some(".text"))
        .expect(".text");
    let raw = text.pointer_to_raw_data as usize;
    let data = &image[raw..raw + text.size_of_raw_data as usize];
    let mut runs = Vec::new();
    let mut start = None;
    for (i, &b) in data.iter().enumerate() {
        if b == 0xcc {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            runs.push((s, i - s));
        }
    }
    let mut used = vec![false; runs.len()];
    sizes
        .iter()
        .filter_map(|&n| {
            let k = (0..runs.len()).find(|&k| !used[k] && runs[k].1 >= n + 2)?;
            used[k] = true;
            Some(text.virtual_address + u32::try_from(runs[k].0 + 1).unwrap())
        })
        .collect()
}

/// Runs every actor of each ctrl work in use through `Script_Run` and the
/// bridge alike, `rounds` times (as frames would), comparing the result and
/// all memory after each worker. A run the original cannot finish in the
/// emulator (a native reaching an import it has no stub for) ends that ctrl
/// work's sequence; the count of compared runs is reported.
fn run_workers(mode: u32, callbacks: u32, rounds: u32) {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = {
        let o = Oracle::from_snapshot(&exe, &pages).unwrap();
        in_use(&o)
    };
    let mut compared = 0;
    let mut executed = [0u64; 0x80];
    for c in ctrls {
        let (mut a, mut b) = run_pair(&pages, &exe);
        make_current(&mut a, &mut b, c);
        let workers = actors(&mut a, &mut b);
        let mut stopped = None;
        'rounds: for round in 0..rounds {
            for (i, &w) in workers.iter().enumerate() {
                let what = format!("ctrl {c:#x} round {round} worker {i} ({w:#x})");
                let ra = match a.call(a.rebase(SCRIPT_RUN), Conv::Cdecl, &[w, mode, callbacks]) {
                    Ok(r) => r.eax,
                    Err(e) => {
                        stopped = Some(format!("{what}: the original stops: {e}"));
                        break 'rounds;
                    }
                };
                let rb = run::script_run_traced(
                    &mut Run(&mut b),
                    &mut run::OriginalNatives,
                    w,
                    mode,
                    callbacks,
                    |op, _| executed[usize::from(op)] += 1,
                );
                assert_eq!(ra, rb, "{what}: result");
                no_diff(&mut a, &mut b, &what);
                compared += 1;
            }
        }
        eprintln!(
            "ctrl work {c:#x}: {} workers; {}",
            workers.len(),
            stopped.unwrap_or_else(|| "all rounds".into())
        );
    }
    let ops: Vec<String> = (0..0x80)
        .filter(|&o| executed[o] != 0)
        .map(|o| format!("{o:02x}:{}", executed[o]))
        .collect();
    eprintln!(
        "{compared} runs compared, {} instructions; opcodes {}",
        executed.iter().sum::<u64>(),
        ops.join(" ")
    );
    assert!(compared > 0, "nothing ran");
}

#[test]
fn script_run_matches_the_original() {
    run_workers(0, 1, 8);
}

#[test]
fn script_run_without_callbacks_matches_the_original() {
    run_workers(0, 0, 4);
}

#[test]
fn script_run_stepping_matches_the_original() {
    run_workers(1, 1, 6);
}

/// Starts worker `w` at entry `e` on both sides, as a request would: the
/// running level's PC at the entry, no wait, no call, an empty stack, speed
/// 1 (so the run goes on past empty stacks until a native or an end stops
/// it), the run flag set.
fn start_entry(a: &mut Oracle, b: &mut Oracle, w: u32, e: u32) {
    let hdr = Mem::read::<u32>(b, w + worker::HEADER);
    let blk = Mem::read::<u32>(b, w + worker::BLOCK);
    let code = blk.wrapping_add(Mem::read::<u32>(b, blk + vm::block::CODE));
    let entries = blk.wrapping_add(Mem::read::<u32>(b, hdr + 0x20));
    let pc = code.wrapping_add(Mem::read::<u32>(b, entries + e * 4));
    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
    let l = v::script_worker_level(b, w, lvl);
    poke(a, b, l + vm::level::PC, pc);
    poke(a, b, l + 0x1d, [0u8; 3]);
    poke(a, b, w + worker::STACK, 0i32);
    poke(a, b, w + 0x94, 1f32.to_bits());
    poke(a, b, w + 0x98, 0u32);
    let flags = Mem::read::<u16>(b, w + worker::FLAGS);
    poke(a, b, w + worker::FLAGS, flags | 2);
}

/// Every entry point (up to `max_entries` a worker) of every actor of each
/// ctrl work in use, run `frames` times from its start on both sides.
fn run_entries(max_entries: u32, frames: u32) {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = {
        let o = Oracle::from_snapshot(&exe, &pages).unwrap();
        in_use(&o)
    };
    let (mut compared, mut stops) = (0, 0);
    let mut executed = [0u64; 0x80];
    for c in ctrls {
        let mut pair = None;
        let n = {
            let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
            actors(a, b).len()
        };
        for i in 0..n {
            for e in 0..max_entries {
                let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
                let w = v::atel_get_basic_work(b, i32::try_from(i).unwrap());
                assert_eq!(
                    orig(a, ATEL_GET_BASIC_WORK, &[u32::try_from(i).unwrap()]),
                    w
                );
                let hdr = Mem::read::<u32>(b, w + worker::HEADER);
                if e >= u32::from(Mem::read::<u16>(b, hdr + 8)) {
                    break;
                }
                start_entry(a, b, w, e);
                for frame in 0..frames {
                    let what = format!("ctrl {c:#x} worker {i} entry {e} frame {frame}");
                    let ra = match a.call(a.rebase(SCRIPT_RUN), Conv::Cdecl, &[w, 0, 1]) {
                        Ok(r) => r.eax,
                        Err(err) => {
                            eprintln!("{what}: the original stops: {err}");
                            stops += 1;
                            pair = None;
                            break;
                        }
                    };
                    let rb = run::script_run_traced(
                        &mut Run(b),
                        &mut run::OriginalNatives,
                        w,
                        0,
                        1,
                        |op, _| executed[usize::from(op)] += 1,
                    );
                    assert_eq!(ra, rb, "{what}: result");
                    no_diff(a, b, &what);
                    compared += 1;
                }
                // the next entry starts from this state, which both sides share
            }
        }
    }
    let ops: Vec<String> = (0..0x80)
        .filter(|&o| executed[o] != 0)
        .map(|o| format!("{o:02x}:{}", executed[o]))
        .collect();
    eprintln!(
        "{compared} runs compared ({stops} stopped in the original), {} instructions, {} opcodes: {}",
        executed.iter().sum::<u64>(),
        ops.len(),
        ops.join(" ")
    );
    assert!(compared > 0, "nothing ran");
}

/// [`run_pair`] with ctrl work `c` current and the game speed at 1.
fn fresh_run_pair(
    pages: &std::sync::Arc<Pages>,
    exe: &std::path::Path,
    c: u32,
) -> (Oracle, Oracle) {
    let (mut a, mut b) = run_pair(pages, exe);
    make_current(&mut a, &mut b, c);
    let speed = a.rebase(run::GAME_SPEED.sym);
    poke(&mut a, &mut b, speed, 1f32.to_bits());
    let at = a.rebase(TRAMPOLINE);
    for o in [&mut a, &mut b] {
        Mem::write_bytes(o, at, &TRAMPOLINE_CODE);
    }
    (a, b)
}

/// `graphicFontDraw`'s body (`0x00640660`), which no script code calls:
/// `fld qword [esp+4]; fld qword [esp+0xc]; call [esp+0x14]; ret` (a
/// two-operand intrinsic with its operands in ST1 and ST0), then at `+16`
/// `fstp st(0); ret`, and at `+19` the one-operand form `fld qword
/// [esp+4]; call [esp+0xc]; ret`. Written into both runs, so the memory
/// diff leaves it alone.
const TRAMPOLINE: u32 = 0x0064_0660;
const TRAMPOLINE_CODE: [u8; 28] = [
    0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3, 0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3,
];

#[test]
fn script_entries_match_the_original() {
    run_entries(6, 6);
}

/// A random program for worker `w` (its own variables, constants and
/// labels), keeping its stack between empty and full, ended by one of the
/// ways a run stops.
#[allow(clippy::too_many_lines)] // one arm per kind of instruction
fn program(rng: &mut Xorshift, b: &Oracle, w: u32) -> Vec<u8> {
    let hdr = b.read::<u32>(w + worker::HEADER);
    let count = |off: u32| u32::from(b.read::<u16>(hdr + off)).max(1);
    let (vars, ints, floats, labels) = (count(2), count(4), count(6), count(0x0a));
    let actors = u32::from(b.read::<u16>(ctx(b).0 + ctrl::ACTORS)).max(1);
    let mut code = Vec::new();
    let long = |code: &mut Vec<u8>, op: u8, x: u32| {
        code.push(op | 0x80);
        code.extend_from_slice(&x.to_le_bytes()[..2]);
    };
    let mut depth = 0i32;
    for _ in 0..=rng.below(40) {
        let any = rng.below(1 << 16);
        match rng.below(12) {
            // pushes
            0 | 1 => {
                let imm = [0, 1, 2, 3, 31, 32, 0xffff, 0x8000, 0x7fff, any][rng.below(10) as usize];
                long(&mut code, 0x2e, imm);
            }
            2 if depth < 18 => {
                let op = [
                    0x28, 0x29, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x70, 0x74, 0x26, 0x2b,
                ][rng.below(11) as usize];
                if op == 0x2b && depth == 0 {
                    continue;
                }
                code.push(op);
            }
            3 => match rng.below(4) {
                0 => long(&mut code, 0x2d, rng.below(ints)),
                1 => long(&mut code, 0x2f, rng.below(floats)),
                2 => long(&mut code, 0x75, rng.below(8)),
                _ => long(&mut code, 0x1f, rng.below(vars)),
            },
            // binary and unary operators
            4..=6 if depth >= 2 => {
                let op = [
                    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                    0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
                ][rng.below(24) as usize];
                code.push(op);
                depth -= 1;
                continue;
            }
            7 if depth >= 1 => code.push([0x19, 0x1a, 0x1c][rng.below(3) as usize]),
            // pops and stores
            8 if depth >= 1 => {
                match rng.below(4) {
                    0 => code.push(
                        [0x2a, 0x2c, 0x25, 0x59, 0x5c, 0x5d, 0x63, 0x66][rng.below(8) as usize],
                    ),
                    1 => long(
                        &mut code,
                        [0x20, 0x21][rng.below(2) as usize],
                        rng.below(vars),
                    ),
                    2 => {
                        // index on top: an element push or a reference
                        long(
                            &mut code,
                            [0x22, 0x27][rng.below(2) as usize],
                            rng.below(vars),
                        );
                        continue;
                    }
                    _ if depth >= 2 => {
                        long(
                            &mut code,
                            [0x23, 0x24][rng.below(2) as usize],
                            rng.below(vars),
                        );
                        depth -= 2;
                        continue;
                    }
                    _ => continue,
                }
                depth -= 1;
                continue;
            }
            // the calls into other code (party requests, waits, requests, 79,
            // 7a), with arguments that are in range: party slots 0..2 (the
            // original's party lookup reads below its own array for a
            // negative slot, stack garbage that differs with the call
            // depth), workers that exist, small levels and entries
            9 if depth < 15 => {
                const OPS: [u8; 25] = [
                    0x39, 0x3a, 0x3b, 0x79, 0x77, 0x78, 0x7a, 0x36, 0x37, 0x38, 0x45, 0x46, 0x47,
                    0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53,
                ];
                let op = OPS[rng.below(25) as usize];
                let args: Vec<u32> = match op {
                    // level, party slot, entry
                    0x39..=0x3b => vec![rng.below(9), rng.below(3), rng.below(4)],
                    // worker, entry / party slot, entry
                    0x77 => vec![rng.below(actors), rng.below(4)],
                    0x78 => vec![rng.below(3), rng.below(4)],
                    // worker, level, entry
                    0x36..=0x38 | 0x45..=0x53 => {
                        vec![rng.below(actors), rng.below(9), rng.below(4)]
                    }
                    0x79 => vec![rng.below(4), rng.below(4), rng.below(4)],
                    _ => vec![rng.below(4), rng.below(4)],
                };
                for &x in &args {
                    long(&mut code, 0x2e, x);
                }
                code.push(op);
                // 39-3b and 7a push their result
                if matches!(op, 0x39..=0x3b | 0x7a) {
                    depth += 1;
                }
                continue;
            }
            _ => continue,
        }
        depth += 1;
    }
    // how it ends, with an empty stack (an end opcode leaves the PC, so
    // with values left the loop would run it again and again)
    code.resize(code.len() + usize::try_from(depth.max(0)).unwrap(), 0x2a);
    match rng.below(9) {
        1 => code.push(0x3e),
        2 => {
            long(&mut code, 0x2e, 1);
            code.push([0x3d, 0x3f][rng.below(2) as usize]);
        }
        3 => code.push(0x54),
        4 | 5 => {
            if rng.below(2) == 0 {
                long(&mut code, 0x2e, rng.below(2));
            }
            long(
                &mut code,
                [0x30, 0x31, 0x32, 0x55, 0x56, 0x57][rng.below(6) as usize],
                rng.below(labels),
            );
            code.push(0x3c);
        }
        _ => code.push(0x3c),
    }
    code
}

#[test]
fn script_programs_match_the_original() {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    // the ctrl work with the most actors (battle: ctrl work 2, whose
    // header tables send signals through their own path)
    let c = {
        let o = Oracle::from_snapshot(&exe, &pages).unwrap();
        in_use(&o)
            .into_iter()
            .max_by_key(|&c| Mem::read::<u16>(&o, c + ctrl::ACTORS))
            .unwrap()
    };
    let mut rng = Xorshift(0x5eed_0004);
    let (mut compared, mut stops) = (0, 0);
    let mut executed = [0u64; 0x80];
    let mut pair = None;
    for k in 0..400 {
        let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
        let workers = actors(a, b);
        let w = workers[rng.below(u32::try_from(workers.len()).unwrap()) as usize];
        let code = program(&mut rng, b, w);
        let at = a.alloc_bytes(&code).unwrap();
        assert_eq!(b.alloc_bytes(&code).unwrap(), at);
        start_entry(a, b, w, 0);
        // runs in a battle too (bit 2: past MsBattleCheck)
        let flags = Mem::read::<u16>(b, w + worker::FLAGS);
        poke(a, b, w + worker::FLAGS, flags | 4);
        let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
        let l = v::script_worker_level(b, w, lvl);
        poke(a, b, l + vm::level::PC, at);
        // statement by statement, on, or on with the accumulator over 1
        let speed = [0.0f32, 1.0, 2.5][rng.below(3) as usize];
        poke(a, b, w + 0x94, speed.to_bits());
        let mode = u32::from(rng.below(8) == 0);
        // ATEL_ONLY=k: program k alone, on the snapshot as it is
        if let Some(only) = std::env::var("ATEL_ONLY")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            && only != k
        {
            continue;
        }
        let what = format!("program {k} on {w:#x}, mode {mode}: {code:02x?}");
        if std::env::var_os("ATEL_TRACE").is_some() {
            eprintln!("{what}");
        }
        for frame in 0..3 {
            let t = std::time::Instant::now();
            let ra = match a.call(a.rebase(SCRIPT_RUN), Conv::Cdecl, &[w, mode, 1]) {
                Ok(r) => r.eax,
                Err(e) => {
                    if std::env::var_os("ATEL_TRACE").is_some() {
                        eprintln!(
                            "  frame {frame}: the original stops after {:?}: {e}",
                            t.elapsed()
                        );
                    }
                    stops += 1;
                    pair = None;
                    break;
                }
            };
            if std::env::var_os("ATEL_TRACE").is_some() {
                eprintln!("  frame {frame}: original {:?}", t.elapsed());
            }
            let rb = run::script_run_traced(
                &mut Run(b),
                &mut run::OriginalNatives,
                w,
                mode,
                1,
                |op, _| {
                    executed[usize::from(op)] += 1;
                },
            );
            assert_eq!(ra, rb, "{what} frame {frame}: result");
            no_diff(a, b, &format!("{what} frame {frame}"));
            compared += 1;
        }
    }
    let ops: Vec<String> = (0..0x80)
        .filter(|&o| executed[o] != 0)
        .map(|o| format!("{o:02x}"))
        .collect();
    eprintln!(
        "{compared} runs compared ({stops} stopped in the original), {} instructions, {} opcodes: {}",
        executed.iter().sum::<u64>(),
        ops.len(),
        ops.join(" ")
    );
    assert!(compared > 0, "nothing ran");
}

/// Runs `code` on the first actor `frames` times on both sides, after
/// `setup` (given the program's address); returns the opcodes executed.
fn run_code(
    code: &[u8],
    mode: u32,
    frames: u32,
    setup: impl Fn(&mut Oracle, &mut Oracle, u32),
) -> Option<Vec<u8>> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let c = ctx(&Oracle::from_snapshot(&exe, &pages).unwrap()).0;
    let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
    let w = actors(&mut a, &mut b)[0];
    let at = a.alloc_bytes(code).unwrap();
    assert_eq!(b.alloc_bytes(code).unwrap(), at);
    start_entry(&mut a, &mut b, w, 0);
    let lvl = i32::from(Mem::read::<u8>(&b, w + worker::LEVEL));
    let l = v::script_worker_level(&b, w, lvl);
    poke(&mut a, &mut b, l + vm::level::PC, at);
    setup(&mut a, &mut b, at);
    let mut ops = Vec::new();
    for frame in 0..frames {
        let what = format!("{code:02x?} mode {mode} frame {frame}");
        let ra = orig(&mut a, SCRIPT_RUN, &[w, mode, 1]);
        let rb = run::script_run_traced(
            &mut Run(&mut b),
            &mut run::OriginalNatives,
            w,
            mode,
            1,
            |op, _| {
                ops.push(op);
            },
        );
        assert_eq!(ra, rb, "{what}: result");
        no_diff(&mut a, &mut b, &what);
    }
    Some(ops)
}

#[test]
fn invalid_opcodes_match_the_original() {
    // with an empty stack (else the loop never ends, in the original too):
    // the PC stays and the run gives up after 0x10000 tries
    for op in [0x1b, 0x41, 0x44, 0x7b, 0x7f, 0x9b, 0xc1, 0xff] {
        if run_code(&[op, 0x3c], 0, 2, |_, _, _| {}).is_none() {
            eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
            return;
        }
    }
}

#[test]
fn breakpoints_match_the_original() {
    // a breakpoint replacing "pushimm 5" (0xae 05 00) with 0x40, then
    // "pop A; end": entry 2 of the ctrl work's table at +0x64 set to it
    let code = [0x40, 0x05, 0x00, 0x2a, 0x3c];
    for (armed, mode, saved) in [
        (0u8, 0, 0xae_u8),
        (1, 0, 0xae),
        (0, 1, 0xae),
        (1, 1, 0xae),
        (1, 0, 0x7c),
        (1, 0, 0x40),
    ] {
        let ops = run_code(&code, mode, 3, |a, b, at| {
            let c = ctx(b).0;
            for e in 0..4u32 {
                let t = c + 0x64 + e * 0x10;
                poke(a, b, t, if e == 2 { 0 } else { -1i32 });
                poke(a, b, t + 8, at);
                poke(a, b, t + 0xe, [saved, armed]);
            }
        });
        let Some(ops) = ops else {
            eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
            return;
        };
        eprintln!("armed {armed} mode {mode} saved {saved:#x}: ran {ops:02x?}");
    }
}

#[test]
fn interpreter_helpers_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let stk = a.alloc(0x80).unwrap();
    assert_eq!(b.alloc(0x80).unwrap(), stk);
    let mut rng = Xorshift(0x5eed_0005);
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            let lvl = i32::from(Mem::read::<u8>(&b, w + worker::LEVEL));
            let l = v::script_worker_level(&b, w, lvl);
            let pc = Mem::read::<u32>(&b, l + vm::level::PC);
            assert_eq!(
                orig(&mut a, 0x0086_9d60, &[w, l]),
                run::atel_fetch_command(&b, l),
                "AtelFetchCommand {w:#x}"
            );
            for long in [0, 0x80, 1] {
                assert_eq!(
                    orig(&mut a, 0x0086_9da0, &[long, pc]),
                    run::atel_fetch_command_main(&b, long, pc),
                    "AtelFetchCommandMain {w:#x}"
                );
            }
            // jumps to each label, outside a call and inside one to another worker
            let hdr = Mem::read::<u32>(&b, w + worker::HEADER);
            let labels = u32::from(Mem::read::<u16>(&b, hdr + 0x0a));
            let saved: [u8; 0x20] = Mem::read(&b, l);
            for depth in [0u8, 1, 2] {
                poke(&mut a, &mut b, l + vm::level::CALL_DEPTH, depth);
                poke(&mut a, &mut b, l + 0x10, [0u16, 0]);
                for label in 0..labels.min(8) {
                    orig(&mut a, 0x0087_26f0, &[w, l, label]);
                    run::script_jump(&mut b, w, l, label);
                    no_diff(
                        &mut a,
                        &mut b,
                        &format!("Script_Jump {w:#x} {label} depth {depth}"),
                    );
                }
            }
            poke(&mut a, &mut b, l, saved);
            poke(&mut a, &mut b, l + vm::level::PC, pc);
        }
    }
    // stacks: the top two kinds, and the top as an integer, depth from -2 to 20
    for _ in 0..2000 {
        let depth = rng.below(23).cast_signed() - 2;
        let kinds: Vec<u8> = (0..24)
            .map(|_| u8::try_from(rng.below(3)).unwrap())
            .collect();
        let values: Vec<u32> = (0..19).map(|_| stored_value(&mut rng)).collect();
        for o in [&mut a, &mut b] {
            Mem::write(o, stk + stack::DEPTH, depth);
            Mem::write_bytes(o, stk + 0x50, &kinds);
            for (i, &x) in (0..).zip(&values) {
                Mem::write(o, stk + stack::VALUES + i * 4, x);
            }
        }
        assert_eq!(
            orig(&mut a, 0x0086_d9e0, &[0, stk]),
            run::script_top_two_ints(&b, stk),
            "Script_TopTwoInts depth {depth}"
        );
        assert_eq!(
            orig(&mut a, 0x0086_a000, &[0, stk]),
            run::atel_fetch_stack_integer(&mut b, stk),
            "AtelFetchStackInteger depth {depth}"
        );
        no_diff(&mut a, &mut b, "stack helpers");
    }
}

/// A hand-built worker in an image-only oracle: one ctrl work (the first,
/// made current) with one actor, its block, header, variables, constants,
/// labels and code, run through the original `Script_Run` and the Rust one
/// frame by frame, the result and all memory compared. Checks the
/// interpreter without a snapshot: the prologue, fetch and dispatch,
/// arithmetic, variables, constants, registers, jumps, the float
/// accumulator and the end (`AtelExecReturn` and `AtelDecodeSignal`).
#[test]
#[allow(clippy::too_many_lines)] // the hand-built block
fn synthetic_script_matches_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    // the block: header at +0x40, tables from +0x80, code at +0x100
    let mut block = vec![0u8; 0x180];
    let put =
        |buf: &mut Vec<u8>, at: usize, v: u32| buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(&mut block, 0x30, 0x100); // code
    put(&mut block, 0x38, 0x40); // header 0
    let h = 0x40;
    block[h] = 0; // type 0: a full actor
    block[h + 2..h + 4].copy_from_slice(&2u16.to_le_bytes()); // variables
    block[h + 4..h + 6].copy_from_slice(&2u16.to_le_bytes()); // ints
    block[h + 6..h + 8].copy_from_slice(&2u16.to_le_bytes()); // floats
    block[h + 0xa..h + 0xc].copy_from_slice(&1u16.to_le_bytes()); // labels
    put(&mut block, h + 0x14, 0x80); // variable descriptors
    put(&mut block, h + 0x18, 0x90); // int constants
    put(&mut block, h + 0x1c, 0x98); // float constants
    put(&mut block, h + 0x20, 0xa0); // entry points
    put(&mut block, h + 0x24, 0xa8); // labels
    // var 0: i32 and var 1: f32, both in the worker (storage 5)
    put(&mut block, 0x80, 5 << 28 | 5 << 25);
    put(&mut block, 0x84, 1);
    put(&mut block, 0x88, 6 << 28 | 5 << 25 | 4);
    put(&mut block, 0x8c, 1);
    put(&mut block, 0x90, 0xffff_fff0);
    put(&mut block, 0x94, 7);
    put(&mut block, 0x98, 2.5f32.to_bits());
    put(&mut block, 0x9c, (-0.75f32).to_bits());
    put(&mut block, 0xa0, 0); // entry 0
    put(&mut block, 0xa4, 0x60); // entry 1
    put(&mut block, 0xa8, 6); // label 0: the loop
    let code: &[u8] = &[
        0xae, 0x05, 0x00, // pushimm 5
        0xa0, 0x00, 0x00, // pop var0
        // loop: var0 -= 1; var1 = var0 * 2.5 + -0.75; A = var0 > 0
        0x9f, 0x00, 0x00, 0xae, 0x01, 0x00, 0x15, 0xa0, 0x00, 0x00, //
        0x9f, 0x00, 0x00, 0xaf, 0x00, 0x00, 0x16, 0xaf, 0x01, 0x00, 0x14, 0xa0, 0x01, 0x00, //
        0xad, 0x00, 0x00, 0xad, 0x01, 0x00, 0x14, 0x2c, // B = -16 + 7
        0x9f, 0x00, 0x00, 0xae, 0x00, 0x00, 0x0a, //
        0xd6, 0x00, 0x00, // pop A, jump to the loop while A != 0
        0x9f, 0x01, 0x00, 0x25, // the float accumulator from var1
        0x28, 0x29, 0x08, 0x2a, // A = B >u A
        // REQ: worker 0 (itself), level 1, entry 1; the result into A
        0xae, 0x01, 0x00, 0xae, 0x00, 0x00, 0xae, 0x01, 0x00, 0x36, 0x2a, //
        0x3c, // end: AtelExecReturn starts the request
    ];
    block[0x100..0x100 + code.len()].copy_from_slice(code);
    // entry 1, run at level 1 from the signal: var0 = 3, end
    block[0x160..0x167].copy_from_slice(&[0xae, 0x03, 0x00, 0xa0, 0x00, 0x00, 0x3c]);
    let blk = a.alloc_bytes(&block).unwrap();
    assert_eq!(b.alloc_bytes(&block).unwrap(), blk);
    let w = a.alloc(0xb58).unwrap();
    assert_eq!(b.alloc(0xb58).unwrap(), w);
    // four free signal nodes
    let nodes = a.alloc(4 * 0x18).unwrap();
    assert_eq!(b.alloc(4 * 0x18).unwrap(), nodes);
    let c = a.rebase(v::CTRL_WORKS.sym);
    for o in [&mut a, &mut b] {
        for f in [run::MUTEX_LOCK, run::MUTEX_UNLOCK, signal::REQ_HOOK] {
            let at = o.rebase(f);
            o.stub_return(at, 0).unwrap();
        }
        let free = w + signal::sigwork::FREE;
        Mem::write(o, free, nodes);
        for i in 0..4 {
            let n = nodes + i * 0x18;
            Mem::write(o, n, if i < 3 { n + 0x18 } else { 0 });
            Mem::write(o, n + 4, if i == 0 { free } else { n - 0x18 });
        }
        let m = o.rebase(run::RUN_MUTEX.sym);
        Mem::write(o, m, 0u32);
        let speed = o.rebase(run::GAME_SPEED.sym);
        Mem::write(o, speed, 1f32.to_bits());
        let ctx = o.rebase(v::CONTEXT.sym);
        Mem::write(o, ctx, c);
        Mem::write(o, c + ctrl::FLAGS, 1u8);
        Mem::write(o, c + ctrl::ACTORS, 1u16);
        Mem::write(o, c + ctrl::WORKERS, 1u16);
        Mem::write(o, c + ctrl::WORK, w);
        Mem::write(o, w + worker::HEADER, blk + 0x40);
        Mem::write(o, w + worker::BLOCK, blk);
        Mem::write(o, w + worker::FLAGS, 2u16 | 4 | 8);
        Mem::write(o, w + 0x94, 1f32.to_bits());
        Mem::write(o, w + worker::LEVELS + vm::level::PC, blk + 0x100);
    }
    let mut ops = 0;
    for frame in 0..4 {
        let ra = orig(&mut a, SCRIPT_RUN, &[w, 0, 0]);
        let rb = run::script_run_traced(
            &mut Run(&mut b),
            &mut run::OriginalNatives,
            w,
            0,
            0,
            |_, _| ops += 1,
        );
        assert_eq!(ra, rb, "frame {frame}");
        no_diff(&mut a, &mut b, &format!("frame {frame}"));
    }
    assert!(ops > 40, "the program ran ({ops} instructions)");
    assert_eq!(
        Mem::read::<i32>(&b, w + 0x48),
        3,
        "var0 counted down, then the request ran entry 1"
    );
}

/// The native paths the random arguments of [`natives_match_the_original`]
/// do not reach: `Native_MsgSetPos`'s three moved windows, the voice waits'
/// countdowns and debug counting, `Native_MsgClose` on each window state,
/// `Native_CamSetScreenDepth` in debug mode.
#[test]
#[allow(clippy::too_many_lines)] // one block per case family
fn native_special_cases_match_the_original() {
    use ffx_script::vm::bridge::{native, native_msg};
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let c = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap())[0];
    let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
    let (a, b) = (&mut a, &mut b);
    let w = v::atel_get_basic_work(b, 0);
    orig(a, ATEL_GET_BASIC_WORK, &[0]);
    let s = w + worker::STACK;
    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
    let args = v::script_worker_level(b, w, lvl) + 0x2c;
    let set = |a: &mut Oracle, b: &mut Oracle, sym: u32, x: u32| {
        let at = b.rebase(sym);
        poke(a, b, at, x);
    };
    let run = |a: &mut Oracle, b: &mut Oracle, sym: u32, push: &[(u8, u32)], what: &str| {
        poke(a, b, s, 0i32);
        for &(k, x) in push {
            push_both(a, b, s, k, x);
        }
        clear_stacks(a, b);
        let ra = a
            .call(a.rebase(sym), Conv::Cdecl, &[w, args, s])
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        let Some(native::Native::Int(f)) = native::find::<Run>(sym) else {
            panic!("{what}: not an integer native");
        };
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original returns {:?} in ST0",
            ra.st0
        );
        let rb = f(&mut Run(b), w, args, s);
        assert_eq!(ra.eax, rb, "{what}: EAX");
        native_no_diff(a, b, what);
    };
    let int = |x: u32| (stack::INT, x);
    let map = native_msg::CURRENT_MAP_NO.sym;
    for (m, x, y) in [
        (0x17, 0x1e6, 0x104),
        (0x17, 0x1e6, 0x105),
        (0xab, 0x100, 0x14f),
        (0xd4, 0x16, 0xdc),
        (0xd4, 0x17, 0xdc),
        (0x01, 0x1e6, 0x104),
    ] {
        set(a, b, map, m);
        run(
            a,
            b,
            0x0085_80c0,
            &[int(1), int(x), int(y), int(2)],
            &format!("MsgSetPos map {m:#x} ({x:#x}, {y:#x})"),
        );
    }
    let debug = native::DEBUG_MODE.sym;
    for mode in [0, 1] {
        set(a, b, debug, mode);
        for (count, flag, left) in [
            (0, 1, 2),
            (3, 1, 1),
            (0, 1, 0),
            (5, 1, -1i32),
            (0, 0, 0),
            (2, 0, 0),
        ] {
            for (sym, base) in [(0x0085_e1d0, 0x0132_5acc_u32), (0x0085_e4a0, 0x0132_5afc)] {
                let (cnt, fl, lf) = if base == 0x0132_5acc {
                    (0x0132_5acc, 0x0132_5ae4, 0x0132_5ae0)
                } else {
                    (0x0132_5afc, 0x0132_5aec, 0x0132_5ae8)
                };
                set(a, b, cnt, count);
                set(a, b, fl, flag);
                set(a, b, lf, left.cast_unsigned());
                for i in 0..3 {
                    run(
                        a,
                        b,
                        sym,
                        &[],
                        &format!("voice {sym:#x} debug {mode} ({count}, {flag}, {left}) call {i}"),
                    );
                }
            }
        }
        run(
            a,
            b,
            0x007b_90b0,
            &[(stack::FLOAT, 404.75f32.to_bits())],
            &format!("CamSetScreenDepth debug {mode}"),
        );
    }
    set(a, b, debug, 0);
    for st in [0u16, 1, 2, 3, 4] {
        // out of range only where nothing is closed: TOCloseMessageWindow
        // gets the window number unclamped
        let far: &[i32] = if st == 0 || st == 3 {
            &[0, 3, 9]
        } else {
            &[0, 3]
        };
        for &i in far {
            let win = v::atel_get_mes_win_work(b, i.min(7));
            poke(a, b, win + 0x14, st);
            run(
                a,
                b,
                0x0085_9060,
                &[int(i.cast_unsigned())],
                &format!("MsgClose {i} state {st}"),
            );
        }
    }
}

/// Calls native `sym` on both sides with `push` on the worker's stack and
/// compares the result (`EAX`, or `ST0` for a float handler) and memory.
fn native_case(a: &mut Oracle, b: &mut Oracle, w: u32, sym: u32, push: &[(u8, u32)], what: &str) {
    use ffx_script::vm::bridge::native::{self, Native};
    let s = w + worker::STACK;
    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
    let args = v::script_worker_level(b, w, lvl) + 0x2c;
    poke(a, b, s, 0i32);
    for &(k, x) in push {
        push_both(a, b, s, k, x);
    }
    clear_stacks(a, b);
    let ra = a
        .call(a.rebase(sym), Conv::Cdecl, &[w, args, s])
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    match native::find::<Run>(sym) {
        Some(Native::Int(f)) => {
            assert!(
                ra.st0_raw.is_none(),
                "{what}: the original returns {:?} in ST0",
                ra.st0
            );
            let rb = f(&mut Run(b), w, args, s);
            assert_eq!(ra.eax, rb, "{what}: EAX");
        }
        Some(Native::Float(f)) => {
            let raw = ra.st0_raw.expect("ST0");
            orig(a, FTOL2, &[]);
            let rb = f(&mut Run(b), w, args, s);
            let x = f80_to_f64(raw);
            assert!(
                x.to_bits() == rb.to_bits() || (x.is_nan() && rb.is_nan()),
                "{what}: ST0 {x} vs {rb}"
            );
        }
        None => panic!("{what}: not in find"),
    }
    native_no_diff(a, b, what);
}

/// The move and turn starts on every mode with set-up blocks, the message
/// opens and waits on each window state, `Msg_ShowSetup`, the frame wait
/// counted by time, the camera paths from the battle lists, `Native_CamWait`
/// in debug mode and `Native_BtlRandom` with its latch.
#[test]
#[allow(clippy::too_many_lines)] // one block per native family
fn move_and_message_cases_match_the_original() {
    use ffx_script::vm::bridge::{native, native_cam, native_move};
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let c = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap())[0];
    let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
    let (a, b) = (&mut a, &mut b);
    let n = actors(a, b).len().min(4);
    eprintln!("{n} actors of ctrl work {c:#x}");
    let int = |x: u32| (stack::INT, x);
    let flt = |x: f32| (stack::FLOAT, x.to_bits());
    let set = |a: &mut Oracle, b: &mut Oracle, sym: u32, x: u32| {
        let at = b.rebase(sym);
        poke(a, b, at, x);
    };
    for i in 0..n {
        let w = v::atel_get_basic_work(b, i32::try_from(i).unwrap());
        orig(a, ATEL_GET_BASIC_WORK, &[u32::try_from(i).unwrap()]);
        let lvl = u32::from(Mem::read::<u8>(b, w + worker::LEVEL));
        let mb = w + 0x5b8 + lvl * 0x4c;
        let tb = Mem::read::<u32>(
            b,
            v::script_worker_level(b, w, i32::try_from(lvl).unwrap()) + 0x48,
        );
        let fields: [(f32, f32, f32, f32, f32); 5] = [
            (3.0, 1.0, 2.0, 0.5, 0.0),
            (-4.0, 6.0, 1.5, 0.0, 1.0),
            (1.0, 1.0, -2.5, 4.0, 0.0),
            (10.0, -2.0, 0.25, -3.0, 2.0),
            (-1.0, -7.5, -1.0, 8.0, 0.0),
        ];
        for (k, &(p2c, p20, p18, pc, p38)) in fields.iter().enumerate() {
            for mode in [5u32, 6, 7, 8, 9, 0x809, 1, 0x10005] {
                for (o, x) in [
                    (0x2c, p2c),
                    (0x20, p20),
                    (0x18, p18),
                    (0xc, pc),
                    (0x38, p38),
                ] {
                    poke(a, b, mb + o, x.to_bits());
                }
                native_case(
                    a,
                    b,
                    w,
                    0x0085_6c30,
                    &[int(1), int(mode), int(2)],
                    &format!("MoveStart worker {i} mode {mode:#x} fields {k}"),
                );
            }
            if tb != 0 {
                for mode in [2u32, 0x802, 0x402, 0xc02, 1, 3, 4, 5] {
                    for (o, x) in [(0x2c, p2c * 40.0), (0x10, p20), (0xc, pc)] {
                        poke(a, b, tb + o, x.to_bits());
                    }
                    native_case(
                        a,
                        b,
                        w,
                        0x0085_7520,
                        &[int(1), int(mode), int(2)],
                        &format!("TurnStart worker {i} mode {mode:#x} fields {k}"),
                    );
                }
            }
        }
        for sym in [
            0x0085_a5c0,
            0x0085_a2a0,
            0x0085_a480,
            0x0085_7f40,
            0x0085_8020,
        ] {
            native_case(
                a,
                b,
                w,
                sym,
                &[flt(2.5)],
                &format!("speed {sym:#x} worker {i}"),
            );
        }
        native_case(
            a,
            b,
            w,
            0x0085_7180,
            &[flt(37.75)],
            &format!("SetTalkRange worker {i}"),
        );
        native_case(
            a,
            b,
            w,
            0x0087_7b70,
            &[flt(2.0)],
            &format!("Sqrt worker {i}"),
        );
        native_case(
            a,
            b,
            w,
            0x0085_8970,
            &[flt(5.0), flt(1.0), flt(-3.0)],
            &format!("AngleToPoint worker {i}"),
        );
    }
    let w = v::atel_get_basic_work(b, 0);
    orig(a, ATEL_GET_BASIC_WORK, &[0]);
    // the frame wait counted by time
    set(a, b, native_move::WAIT_FRAMES_BY_TIME.sym, 1);
    native_case(a, b, w, 0x0085_c4e0, &[int(10)], "WaitFrames start");
    for k in 0..3 {
        native_case(
            a,
            b,
            w,
            0x0085_c6b0,
            &[],
            &format!("WaitFrames poll by time {k}"),
        );
    }
    set(a, b, native_move::WAIT_FRAMES_BY_TIME.sym, 0);
    // message opens and waits on each state
    for mode in [0u32, 2, 4, 8, 0x40, 66] {
        native_case(
            a,
            b,
            w,
            0x0085_7870,
            &[int(1), int(0)],
            "MsgSetText before MsgOpen",
        );
        native_case(
            a,
            b,
            w,
            0x0085_8b50,
            &[int(1), int(mode)],
            &format!("MsgOpen mode {mode}"),
        );
    }
    for st in [0u16, 1, 2, 3, 4, 5] {
        let win = v::atel_get_mes_win_work(b, 1);
        for (start, poll, end) in [
            (0x0085_abd0, 0x0085_ae90, 0x0085_b6d0),
            (0x0085_b7c0, 0x0085_b980, 0x0085_ba80),
            (0x0085_9710, 0x0085_9c50, 0x0085_a6f0),
        ] {
            let push: &[(u8, u32)] = if start == 0x0085_abd0 {
                &[int(1), int(1)]
            } else if start == 0x0085_9710 {
                &[int(7), int(9), int(1)]
            } else {
                &[int(1)]
            };
            native_case(
                a,
                b,
                w,
                start,
                push,
                &format!("wait start {start:#x} state {st}"),
            );
            poke(a, b, win + 0x14, st);
            native_case(
                a,
                b,
                w,
                poll,
                &[],
                &format!("wait poll {poll:#x} state {st}"),
            );
            native_case(a, b, w, end, &[], &format!("wait end {end:#x} state {st}"));
        }
    }
    // the battle flag bytes: set every selector with an odd and an even
    // value, read each back
    for k in (0u32..=27).filter(|k| ![0, 10, 11, 21, 22].contains(k)) {
        for v in [0x35u32, 0x82] {
            native_case(
                a,
                b,
                w,
                0x007a_81f0,
                &[int(k), int(v)],
                &format!("707d k {k} v {v:#x}"),
            );
            for r in 0u32..=27 {
                native_case(
                    a,
                    b,
                    w,
                    0x007a_27e0,
                    &[int(r)],
                    &format!("707e k {r} after {k}"),
                );
            }
        }
    }
    // the 0x0267 choice poll on each state
    for st in [0u16, 1, 2, 3, 4] {
        let win = v::atel_get_mes_win_work(b, 1);
        native_case(a, b, w, 0x0085_9710, &[int(1)], "0x0267 start");
        poke(a, b, win + 0x14, st);
        native_case(
            a,
            b,
            w,
            0x0085_a540,
            &[],
            &format!("0x0267 poll state {st}"),
        );
        native_case(
            a,
            b,
            w,
            0x0085_a620,
            &[],
            &format!("0x0267 result state {st}"),
        );
    }
    // the voice standby and start, armed and not, in and out of debug mode
    for mode in [0u32, 1] {
        set(a, b, native::DEBUG_MODE.sym, mode);
        for (cnt, left, hold) in [(0u32, 2u32, 0u32), (3, 0, 0), (0, 0, 2)] {
            for c in [0x0132_5acc_u32, 0x0132_5af4, 0x0132_5af8] {
                set(a, b, c, cnt);
            }
            set(a, b, 0x012f_b808, hold);
            for (start, poll, end, f, l) in [
                (
                    0x0085_cd00u32,
                    0x0085_cec0u32,
                    0x0085_d360u32,
                    0x0132_5ad4u32,
                    0x0132_5ad0u32,
                ),
                (
                    0x0085_d6b0,
                    0x0085_d9d0,
                    0x0085_de80,
                    0x0132_5adc,
                    0x0132_5ad8,
                ),
            ] {
                native_case(
                    a,
                    b,
                    w,
                    start,
                    &[int(7)],
                    &format!("voice start {start:#x} debug {mode}"),
                );
                for armed in [1u32, 0] {
                    set(a, b, f, armed);
                    set(a, b, l, left);
                    for k in 0..3 {
                        native_case(
                            a,
                            b,
                            w,
                            poll,
                            &[],
                            &format!(
                                "voice poll {poll:#x} debug {mode} armed {armed} left {left} hold {hold} call {k}"
                            ),
                        );
                    }
                }
                native_case(
                    a,
                    b,
                    w,
                    end,
                    &[],
                    &format!("voice end {end:#x} debug {mode}"),
                );
            }
        }
    }
    set(a, b, native::DEBUG_MODE.sym, 0);
    set(a, b, 0x012f_b808, 0);
    native_case(
        a,
        b,
        w,
        0x0086_02a0,
        &[int(2), int(3), int(1), int(0), int(40), int(60), int(4)],
        "MsgShowWait start",
    );
    // camera paths from the battle lists
    for (n0, n1) in [(0u32, 0u32), (3, 2), (5, 7)] {
        set(a, b, native_cam::BTL_SPLINES.sym, n0);
        set(a, b, native_cam::BTL_SPLINES.sym + 4, n1);
        for list in [0u32, 1] {
            native_case(
                a,
                b,
                w,
                0x007b_a020,
                &[int(list), int(4)],
                &format!("Cam_6059 list {list} ({n0}, {n1})"),
            );
            native_case(
                a,
                b,
                w,
                0x007b_a100,
                &[int(list), flt(0.5), int(4)],
                &format!("Cam_605a list {list} ({n0}, {n1})"),
            );
            native_case(
                a,
                b,
                w,
                0x007b_a210,
                &[int(list), int(4)],
                &format!("Cam_605b list {list} ({n0}, {n1})"),
            );
        }
    }
    for mode in [0, 1] {
        set(a, b, native::DEBUG_MODE.sym, mode);
        for frames in [0u32, 2] {
            set(a, b, native_cam::CAM_WAIT_FRAMES.sym, frames);
            native_case(
                a,
                b,
                w,
                0x007b_9be0,
                &[],
                &format!("CamWait debug {mode} frames {frames}"),
            );
        }
    }
    set(a, b, native::DEBUG_MODE.sym, 0);
    for latch in [0, 1] {
        set(a, b, native_cam::BTL_RANDOM_LATCH.sym, latch);
        native_case(
            a,
            b,
            w,
            0x007b_89a0,
            &[int(3)],
            &format!("BtlRandom latch {latch}"),
        );
        native_case(
            a,
            b,
            w,
            0x007b_9b40,
            &[],
            &format!("CamMoveFinish latch {latch}"),
        );
    }
}

/// Start handlers that are a bare `ret`: `Native_ChrWaitMotion_501e`,
/// `Native_MusicWaitLoaded`, `Native_00dc`, `Native_FadeWait` and
/// the starts of `0xb009` and `0xb00b`.
const BARE_RET: [u32; 6] = [
    0x00a7_9370,
    0x0085_9d70,
    0x0085_ee00,
    0x00a7_7ee0,
    0x0076_eb30,
    0x0076_ec20,
];

/// The Rust natives of `groups` against the original on each actor of each
/// ctrl work in use, with small integer and float arguments on the worker's
/// stack
/// (the same on both sides): the result (`EAX`, or `ST0` for the float
/// ones) and all memory. A call the original cannot finish in the emulator
/// (an import with no stub) is skipped and counted.
#[allow(clippy::too_many_lines)] // the loop over every native and its checks
fn natives_match(groups: &[&[(u32, bool)]], rng_seed: u32) {
    use ffx_script::vm::bridge::native::{self, Native};
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut rng = Xorshift(rng_seed);
    let (mut compared, mut stops) = (0, 0);
    let mut seen = std::collections::BTreeSet::new();
    for c in ctrls {
        let mut pair = None;
        let n = {
            let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
            actors(a, b).len().min(12)
        };
        for i in 0..n {
            for &(sym, float) in groups.iter().flat_map(|l| l.iter()) {
                // tested on chosen arguments in round19_natives_match_the_original
                if R20_UNINIT.contains(&sym) {
                    continue;
                }
                // a native that ran alike on both sides may still have made
                // a ctrl work from a garbage-filled worker's words current
                // (battle.snap's worker 5 of 0xf46010 is 0xcd fill): start
                // again then
                if pair.as_ref().is_some_and(|(_, b)| !ctx_ok(b)) {
                    pair = None;
                }
                let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
                let w = v::atel_get_basic_work(b, i32::try_from(i).unwrap());
                orig(a, ATEL_GET_BASIC_WORK, &[u32::try_from(i).unwrap()]);
                let s = w + worker::STACK;
                poke(a, b, s, 0i32);
                for _ in 0..6 {
                    let (kind, val) = if rng.below(3) == 0 {
                        #[allow(clippy::cast_precision_loss)] // small
                        let f = (rng.below(400) as f32 - 200.0) / 4.0;
                        (stack::FLOAT, f.to_bits())
                    } else {
                        (stack::INT, rng.below(4))
                    };
                    push_both(a, b, s, kind, val);
                }
                let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
                let args = v::script_worker_level(b, w, lvl) + 0x2c;
                let what = format!("native {sym:#x} on worker {i} of {c:#x}");
                clear_stacks(a, b);
                // ECX = 0 at the call: some natives make their out local
                // with `push ecx` (`0x70be`'s `MsGetChrDir(i, &out)` writes
                // nothing without a model), and the ports zero it
                a.call(a.rebase(BARE_RET[1]), Conv::Thiscall(0), &[])
                    .unwrap();
                let ra = match a.call(a.rebase(sym), Conv::Cdecl, &[w, args, s]) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("{what}: the original stops: {e}");
                        stops += 1;
                        pair = None;
                        continue;
                    }
                };
                match native::find::<Run>(sym) {
                    Some(Native::Int(f)) => {
                        assert!(!float, "{what}");
                        assert!(
                            ra.st0_raw.is_none(),
                            "{what}: the original returns {:?} in ST0",
                            ra.st0
                        );
                        let rb = f(&mut Run(b), w, args, s);
                        // bare rets: EAX is the caller's (and the
                        // dispatch ignores a start's)
                        if !BARE_RET.contains(&sym) {
                            assert_eq!(ra.eax, rb, "{what}: EAX");
                        }
                    }
                    Some(Native::Float(f)) => {
                        assert!(float, "{what}");
                        let raw = ra.st0_raw.expect("ST0");
                        orig(a, FTOL2, &[]);
                        let rb = f(&mut Run(b), w, args, s);
                        let x = f80_to_f64(raw);
                        assert!(
                            x.to_bits() == rb.to_bits() || (x.is_nan() && rb.is_nan()),
                            "{what}: ST0 {x} vs {rb}"
                        );
                    }
                    None => panic!("{what}: not in find"),
                }
                native_no_diff(a, b, &what);
                compared += 1;
                seen.insert(sym);
            }
        }
    }
    let never: Vec<String> = groups
        .iter()
        .flat_map(|l| l.iter())
        .filter(|(sym, _)| !seen.contains(sym))
        .map(|(sym, _)| format!("{sym:#x}"))
        .collect();
    eprintln!(
        "{compared} native calls compared ({stops} stopped in the original); never completed here: {}",
        never.join(" ")
    );
    assert!(compared > 0, "nothing ran");
}

/// [`natives_match`] split by module so the test threads share the work.
mod natives_match_the_original {
    use ffx_script::vm::bridge as m;

    #[test]
    fn native() {
        super::natives_match(&[&m::native::PORTED], 0x5eed_0005);
    }
    #[test]
    fn map_cam_msg() {
        super::natives_match(
            &[
                &m::native_map::PORTED,
                &m::native_cam::PORTED,
                &m::native_msg::PORTED,
            ],
            0x5eed_0105,
        );
    }
    #[test]
    fn moves() {
        super::natives_match(&[&m::native_move::PORTED], 0x5eed_0205);
    }
    #[test]
    fn btl() {
        super::natives_match(
            &[&m::native_btl::PORTED, &m::native_btl2::PORTED],
            0x5eed_0305,
        );
    }
    #[test]
    fn sys_chr() {
        super::natives_match(
            &[&m::native_sys::PORTED, &m::native_chr::PORTED],
            0x5eed_0405,
        );
    }
    #[test]
    fn mods() {
        super::natives_match(&[&m::native_mods::PORTED], 0x5eed_0505);
    }
}

/// The non-native VM helpers of round 13 against the original, on every
/// actor of every ctrl work in use and on random global state: results
/// (`EAX`) and all memory.
#[test]
#[allow(clippy::too_many_lines)] // one block per helper
fn vm_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0013);
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            let what = format!("worker {w:#x} of {c:#x}");
            // workers without turn blocks (their slots unset, 0xcdcdcdcd)
            // fault in the original
            let turns = (0..9).all(|l| {
                let p = Mem::read::<u32>(&b, w + 0x174 + l * 0x4c);
                (w..w + 0x1000).contains(&p)
            });
            for level in [u32::MAX, 0, 1, 3, 8, 9].into_iter().filter(|_| turns) {
                let ra = orig(&mut a, 0x0086_3fe0, &[w, level]);
                let rb = hp::script_stop_turn(&mut b, w, level);
                assert_eq!(ra, rb, "Script_StopTurn {what} {level}");
                no_diff(&mut a, &mut b, &format!("Script_StopTurn {what}"));
            }
            for on in [1, 0, 7] {
                let ra = orig(&mut a, 0x0087_12a0, &[w, on]);
                let rb = hp::atel_set_worker_flag_200(&mut b, w, on);
                assert_eq!(ra, rb, "0x008712a0 {what}");
                no_diff(&mut a, &mut b, "0x008712a0");
            }
            if turns && {
                let h = Mem::read::<u32>(&b, w + worker::HEADER);
                Mem::read::<u8>(&b, h) != 4
            } {
                let v: Vec<u32> = (0..3).map(|_| stored_value(&mut rng)).collect();
                let ra = orig(&mut a, 0x0086_f780, &[w, v[0], v[1], v[2]]);
                let rb = hp::atel_set_level_vectors(&mut b, w, v[0], v[1], v[2]);
                assert_eq!(ra, rb, "0x0086f780 {what}");
                no_diff(&mut a, &mut b, "0x0086f780");
            }
        }
        // the header tables of the ctrl work's script block (+0x48)
        let bin = Mem::read::<u32>(&b, c + 0x48);
        // a ctrl work without a block (0) faults in the original
        if bin != 0 {
            let what = format!("ctrl {c:#x}");
            assert_eq!(
                orig(&mut a, 0x0086_ac60, &[bin]),
                hp::atel_bin_table_count(&b, bin),
                "0x0086ac60 {what}"
            );
            for i in 0..4 {
                assert_eq!(
                    orig(&mut a, 0x0086_ac30, &[bin, i]),
                    hp::atel_bin_table_entry_a(&b, bin, i),
                    "0x0086ac30 {what} {i}"
                );
                assert_eq!(
                    orig(&mut a, 0x0086_ac90, &[bin, i]),
                    hp::atel_bin_table_entry_b(&b, bin, i),
                    "0x0086ac90 {what} {i}"
                );
            }
        }
    }
    let ctx = a.rebase(0x0132_6b28);
    let ctrl0 = Mem::read::<u32>(&b, ctx);
    let bank = a.rebase(0x0132_6b82);
    let windows = a.rebase(0x0132_6d84);
    let masks = a.rebase(0x0132_6b94);
    for round in 0..60 {
        if round > 0 {
            for off in (0..0x2c * 24).step_by(4) {
                let v = if rng.below(2) == 0 {
                    rng.below(3)
                } else {
                    rng.next()
                };
                poke(&mut a, &mut b, windows + off, v);
            }
            poke(&mut a, &mut b, bank, u8::try_from(rng.below(3)).unwrap());
            poke(&mut a, &mut b, ctrl0, rng.next());
            for i in 0..19 {
                poke(&mut a, &mut b, masks + 4 * i, rng.next());
            }
        }
        for i in [u32::MAX, 0, 3, 7, 9, 15, 16] {
            assert_eq!(
                orig(&mut a, 0x0087_73f0, &[i]),
                hp::atel_get_snd3d_work(&b, i),
                "AtelGetSnd3dWork {i}"
            );
            for k in 0..8 {
                assert_eq!(
                    orig(&mut a, 0x0087_6860, &[k, i & 7]),
                    hp::atel_sound_table_get(&b, k, i & 7),
                    "AtelSoundTableGet {k} {i}"
                );
            }
            assert_eq!(
                orig(&mut a, 0x0086_be60, &[i]) & 0xffff,
                hp::atel_get_window_status(&b, i),
                "0x0086be60 {i} round {round}"
            );
        }
        assert_eq!(
            orig(&mut a, 0x0086_c8b0, &[]),
            hp::atel_windows_busy(&b),
            "0x0086c8b0 round {round}"
        );
        for strict in [0, 1] {
            assert_eq!(
                orig(&mut a, 0x0086_c530, &[strict]),
                hp::atel_ctrl_running(&b, strict),
                "0x0086c530 {strict} round {round}"
            );
        }
        assert_eq!(
            orig(&mut a, 0x0086_0f50, &[]),
            hp::atel_all_clear_bind_mask(&mut b),
            "AtelAllClearBindMask"
        );
        no_diff(&mut a, &mut b, &format!("globals round {round}"));
    }
}

/// Round 14's VM helpers against the original: the worker setters on
/// every actor, the script-block getters on every ctrl work in use, the
/// flag, sprite and record setters on random state; results and memory.
#[test]
#[allow(clippy::too_many_lines)] // one block per helper family
fn more_vm_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0014);
    // a float that is not a signalling NaN (the emulator keeps those)
    let f32v = |rng: &mut Xorshift| loop {
        let v = stored_value(rng);
        let f = f32::from_bits(v);
        if !f.is_nan() || v & 0x0040_0000 != 0 {
            return v;
        }
    };
    let level_setters: [(u32, u32); 4] = [
        (0x0086_f040, 0x5ec),
        (0x0087_15c0, 0x5c8),
        (0x0087_1610, 0x5c4),
        (0x0087_1630, 0x5c0),
    ];
    let st0_setters: [(u32, u32, bool); 6] = [
        (0x0087_1500, 0x5f8, false),
        (0x0087_1520, 0x5f4, false),
        (0x0087_1540, 0x880, true),
        (0x0087_1560, 0x87c, true),
        (0x0087_1580, 0x874, true),
        (0x0087_15a0, 0x878, true),
    ];
    let all: [(u32, u32, u32); 7] = [
        (0x0086_f0d0, 0x5f0, 0x4c),
        (0x0086_f170, 0x5f8, 0x4c),
        (0x0086_f1c0, 0x5f4, 0x4c),
        (0x0086_f350, 0x5c8, 0x4c),
        (0x0086_f210, 0x880, 0x44),
        (0x0086_f260, 0x87c, 0x44),
        (0x0086_f2b0, 0x874, 0x44),
    ];
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            let what = format!("worker {w:#x}");
            for on in [1, 0, 3] {
                let ra = orig(&mut a, 0x0087_0fe0, &[w, on]);
                assert_eq!(
                    ra,
                    hp::atel_set_worker_flag36_8(&mut b, w, on),
                    "0x00870fe0 {what}"
                );
            }
            for i in 0..3 {
                let v = f32v(&mut rng);
                let ra = orig(&mut a, 0x0087_0d10, &[w, i, v]);
                assert_eq!(
                    ra,
                    hp::atel_set_level_f74(&mut b, w, i, v),
                    "0x00870d10 {what}"
                );
            }
            for (f, off) in level_setters {
                let v = f32v(&mut rng);
                let ra = orig(&mut a, f, &[w, v]);
                assert_eq!(
                    ra,
                    hp::atel_set_cur_level_f32(&mut b, w, off, v),
                    "{f:#x} {what}"
                );
            }
            for (f, off, turn) in st0_setters {
                let (i, v) = (rng.below(9), f32v(&mut rng));
                let ra = orig_float(&mut a, f, &[w, i, v]);
                let rb = if turn {
                    hp::atel_set_turn_f32(&mut b, w, i, off, v)
                } else {
                    hp::atel_set_level_f32(&mut b, w, i, off, v)
                };
                assert!(same_float(ra, ffx_bridge::x87::store(rb)), "{f:#x} {what}");
            }
            for (f, off, step) in all {
                let v = f32v(&mut rng);
                let ra = orig(&mut a, f, &[w, v]);
                assert_eq!(
                    ra,
                    hp::atel_set_all_f32(&mut b, w, off, step, v),
                    "{f:#x} {what}"
                );
            }
            let v = f32v(&mut rng);
            let ra = orig_float(&mut a, 0x0086_f300, &[w, v]);
            hp::atel_set_all_f32(&mut b, w, 0x878, 0x44, v);
            assert!(same_float(ra, v), "0x0086f300 {what}");
            no_diff(&mut a, &mut b, &format!("worker setters {what}"));
        }
        let bin = Mem::read::<u32>(&b, c + 0x48);
        if bin != 0 {
            let what = format!("ctrl {c:#x}");
            assert_eq!(
                orig(&mut a, 0x0086_adc0, &[bin]),
                hp::atel_get_bin_hdr_expand_sp_flags(&b, bin),
                "0x0086adc0 {what}"
            );
            assert_eq!(
                orig(&mut a, 0x0086_acc0, &[bin]),
                hp::atel_get_bin_hdr_expand_24(&b, bin),
                "0x0086acc0 {what}"
            );
            assert_eq!(
                orig(&mut a, 0x0086_aa70, &[bin]),
                hp::atel_get_bin_hdr_expand_name(&b, bin),
                "0x0086aa70 {what}"
            );
            assert_eq!(
                orig(&mut a, 0x0086_ad20, &[bin]),
                hp::atel_get_bin_hdr_expand_sound_event(&b, bin),
                "0x0086ad20 {what}"
            );
            for i in 0..4 {
                assert_eq!(
                    orig(&mut a, 0x0086_aaf0, &[bin, i]),
                    hp::atel_get_bin_hdr_table18_a(&b, bin, i),
                    "0x0086aaf0 {what}"
                );
                assert_eq!(
                    orig(&mut a, 0x0086_aac0, &[bin, i]),
                    hp::atel_get_bin_hdr_table18_b(&b, bin, i),
                    "0x0086aac0 {what}"
                );
            }
        }
        // the +8 block's string table, where +8 is a block (or 0)
        let e = Mem::read::<u32>(&b, c + 8);
        if e == 0 || (e.trailing_zeros() >= 2 && (0x0040_0000..0x7000_0000).contains(&e)) {
            let t = if e == 0 {
                1
            } else {
                Mem::read::<u32>(&b, e + 0x44)
            };
            if t != 0 {
                for i in 0..2 {
                    assert_eq!(
                        orig(&mut a, 0x0088_58c0, &[c, i]),
                        hp::atel_get_block_string(&b, c, i),
                        "0x008858c0 ctrl {c:#x}"
                    );
                }
            }
        }
    }
    assert_eq!(
        orig(&mut a, 0x0086_ae20, &[]),
        hp::atel_get_bindp_list(&b),
        "AtelGetBindpList"
    );
    let strs = a.alloc(0x40).unwrap();
    assert_eq!(b.alloc(0x40).unwrap(), strs);
    for round in 0..40 {
        for at in [
            0x0132_6b70,
            0x0132_6b74,
            0x0132_6b90,
            0x0132_5db0,
            0x00c5_273c,
        ] {
            let p = a.rebase(at);
            poke(&mut a, &mut b, p, rng.next());
        }
        let g = a.rebase(0x0132_5ba3);
        poke(&mut a, &mut b, g, u8::try_from(rng.below(256)).unwrap());
        let n = rng.below(40);
        let ra = orig(&mut a, 0x0086_3860, &[n]);
        assert_eq!(ra, hp::atel_clear_effect_flag(&mut b, n), "0x00863860");
        let ra = orig(&mut a, 0x0087_02f0, &[n]);
        assert_eq!(ra, hp::atel_set_effect_flag2(&mut b, n), "0x008702f0");
        let n = rng.below(40);
        let ra = orig(&mut a, 0x0086_3af0, &[n]);
        assert_eq!(ra, hp::atel_clear_effect_flag2(&mut b, n), "0x00863af0");
        let k = rng.below(12);
        let on = rng.below(2);
        let ra = orig(&mut a, 0x0087_0250, &[k, on]);
        let rb = hp::atel_set_love_param_disable(&mut b, k, on);
        if k < 8 {
            assert_eq!(ra, rb, "AtelSetLoveParamDisable {k}");
        }
        for (f, set) in [
            (
                0x0087_0160,
                hp::atel_set_guide_map_draw_bliz_flag as fn(&mut Oracle, u32) -> u32,
            ),
            (0x0087_0190, hp::atel_set_guide_map_draw_flag),
            (0x0087_01c0, hp::atel_set_guide_map_force_draw_flag),
            (0x0086_f3d0, hp::atel_set_ctrl_flag_1325db0),
        ] {
            let on = rng.below(3);
            assert_eq!(orig(&mut a, f, &[on]), set(&mut b, on), "{f:#x}");
        }
        let m = rng.next();
        assert_eq!(
            orig(&mut a, 0x0086_2d50, &[m]),
            hp::atel_get_battle_map_id(&b, m),
            "0x00862d50"
        );
        for i in [u32::MAX, 0, 5, 127, 128] {
            assert_eq!(
                orig(&mut a, 0x0087_47b0, &[i]),
                hp::atel_get_sprite(&b, i),
                "0x008747b0 {i}"
            );
            assert_eq!(
                orig(&mut a, 0x0087_47f0, &[i]),
                hp::atel_clear_sprite(&mut b, i),
                "0x008747f0 {i}"
            );
            let on = rng.below(3);
            assert_eq!(
                orig(&mut a, 0x0087_4970, &[i, on]),
                hp::atel_set_sprite_flag(&mut b, i, on),
                "0x00874970 {i}"
            );
            let v = rng.next();
            assert_eq!(
                orig(&mut a, 0x0087_49b0, &[i, v]),
                hp::atel_set_sprite_13(&mut b, i, v),
                "0x008749b0 {i}"
            );
            let (x, y) = (rng.next(), rng.next());
            assert_eq!(
                orig(&mut a, 0x0087_4b30, &[i, x, y]),
                hp::atel_set_sprite_pos(&mut b, i, x, y),
                "0x00874b30 {i}"
            );
        }
        for i in [u32::MAX, 0, 9, 15, 16] {
            assert_eq!(
                orig(&mut a, 0x0087_72f0, &[i]),
                hp::atel_clear_snd3d_work(&mut b, i),
                "AtelClearSnd3dWork {i}"
            );
        }
        assert_eq!(
            orig(&mut a, 0x0087_4fb0, &[]),
            hp::atel_clear_records_1327f50(&mut b),
            "0x00874fb0"
        );
        // strings: a run of one character, then others
        let ch = b'a' + u8::try_from(rng.below(3)).unwrap();
        let mut s: Vec<u8> = (0..rng.below(20))
            .map(|_| {
                if rng.below(2) == 0 {
                    ch
                } else {
                    b'a' + u8::try_from(rng.below(3)).unwrap()
                }
            })
            .collect();
        s.push(0);
        for o in [&mut a, &mut b] {
            Mem::write_bytes(o, strs, &s);
        }
        assert_eq!(
            orig(&mut a, 0x0087_19c0, &[strs]),
            hp::atel_strlen(&b, strs),
            "0x008719c0"
        );
        let n = rng.below(6).wrapping_sub(1);
        let ra = orig(&mut a, 0x0087_7610, &[strs, n, u32::from(ch)]);
        assert_eq!(
            ra,
            hp::atel_str_trim_leading(&mut b, strs, n, u32::from(ch)),
            "0x00877610 {s:?} {n}"
        );
        no_diff(&mut a, &mut b, &format!("globals round {round}"));
    }
    let ra = orig_float(&mut a, 0x007a_3ac0, &[]);
    assert!(
        same_float(ra, ffx_bridge::x87::store(hp::atel_get_battle_count_f(&b))),
        "0x007a3ac0"
    );
}

/// Round 15's VM helpers against the original: results and memory.
#[test]
fn round15_vm_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0015);
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            let turns = (0..9).all(|l| {
                let p = Mem::read::<u32>(&b, w + 0x174 + l * 0x4c);
                (w..w + 0x1000).contains(&p)
            });
            let h = Mem::read::<u32>(&b, w + worker::HEADER);
            if turns && Mem::read::<u8>(&b, h) != 4 {
                for mask in [7, 1, 2, 4, 0] {
                    let ra = orig(&mut a, 0x0086_db80, &[w, mask]);
                    assert_eq!(
                        ra,
                        hp::atel_copy_turn_angles(&mut b, w, mask),
                        "0x0086db80 {w:#x}"
                    );
                }
                no_diff(&mut a, &mut b, &format!("0x0086db80 {w:#x}"));
            }
        }
        let ra = orig(&mut a, 0x0087_19e0, &[]);
        assert_eq!(ra, hp::atel_uc_off(&mut Run(&mut b)), "AtelUcOff {c:#x}");
        no_diff(&mut a, &mut b, &format!("AtelUcOff {c:#x}"));
    }
    assert_eq!(
        orig(&mut a, 0x0086_c410, &[]),
        hp::atel_get_save_dic(&mut Run(&mut b)),
        "AtelGetSaveDic"
    );
    // dictionaries 0 and 1 (higher ones are unset in the snapshots)
    for d in 0..2 {
        for i in [0, 1, 5, 40, u32::MAX] {
            let ra = orig(&mut a, 0x0086_c430, &[i, d]);
            assert_eq!(
                ra,
                hp::atel_get_save_dic_name(&mut Run(&mut b), i, d),
                "AtelGetSaveDicName {i} {d}"
            );
        }
    }
    no_diff(&mut a, &mut b, "AtelGetSaveDicName");
    let dst = a.alloc(0x20).unwrap();
    assert_eq!(b.alloc(0x20).unwrap(), dst);
    for _ in 0..200 {
        let v = if rng.below(2) == 0 {
            rng.below(100_000)
        } else {
            rng.next()
        };
        let ra = orig(&mut a, 0x0087_7520, &[v, dst, 0x30]);
        assert_eq!(
            ra,
            hp::atel_put_digits(&mut b, v, dst, 0x30),
            "0x00877520 {v}"
        );
        no_diff(&mut a, &mut b, &format!("0x00877520 {v}"));
    }
    // AtelRequireCtrlWork on a ctrl work not in use, with and without a
    // block (a used one's)
    let used = in_use(&b);
    let blk = Mem::read::<u32>(&b, used[0] + 0x48);
    let free = (0..7)
        .find(|&i| !used.contains(&(a.rebase(0x0132_5ba0) + i * 0x238)))
        .unwrap();
    for block in [blk, 0] {
        let (x, y) = (rng.next(), rng.next());
        let ra = orig(&mut a, 0x0086_ee10, &[free, block, x, y]);
        assert_eq!(
            ra,
            hp::atel_require_ctrl_work(&mut b, free, block, x, y),
            "AtelRequireCtrlWork"
        );
        no_diff(&mut a, &mut b, "AtelRequireCtrlWork");
    }
    let tables = a.rebase(0x0132_8558);
    let saved: [u32; 16] = Mem::read(&b, tables);
    for g in 0..16 {
        let t = if rng.below(2) == 0 { 0 } else { rng.next() };
        assert_eq!(
            orig(&mut a, 0x0087_7880, &[g, t]),
            hp::atel_set_up_call_func(&mut b, g, t),
            "AtelSetUpCallFunc"
        );
    }
    no_diff(&mut a, &mut b, "AtelSetUpCallFunc");
    poke(&mut a, &mut b, tables, saved);
}

/// Round 16's VM helpers against the original: results and memory.
#[test]
#[allow(clippy::too_many_lines)] // one block per helper
fn round16_vm_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let mut rng = Xorshift(0x5eed_0016);
    for c in in_use(&b) {
        make_current(&mut a, &mut b, c);
        for w in actors(&mut a, &mut b) {
            for key in [u32::MAX, 0, 1, 2, 5] {
                assert_eq!(
                    orig(&mut a, 0x0086_32b0, &[w, key]),
                    hp::script_line_actor_side(&b, w, key),
                    "Script_LineActorSide {w:#x} {key}"
                );
            }
        }
        // Native_PlayMotionCommon on each model, as model 0x66 too (its
        // 0x013008e0 flag), in every mode, from the worker's stack
        for w in actors(&mut a, &mut b) {
            let m = v::atel_get_char_handle(&b, w);
            if m == 0 {
                continue;
            }
            let id = Mem::read::<u16>(&b, m);
            let s = w + worker::STACK;
            for (mode, model) in [(0, id), (1, 0x66), (2, id), (3, 0x66)] {
                poke(&mut a, &mut b, m, model);
                poke(&mut a, &mut b, s, 0i32);
                for x in [0x0002_1092, 0, 1, 0, u32::MAX] {
                    push_both(&mut a, &mut b, s, stack::INT, x);
                }
                clear_stacks(&mut a, &mut b);
                let ra = orig(&mut a, 0x00a7_aae0, &[w, s, mode, 0]);
                let rb = ffx_script::vm::bridge::native_chr::native_play_motion_common(
                    &mut Run(&mut b),
                    w,
                    s,
                    mode,
                    0,
                );
                assert_eq!(ra, rb, "Native_PlayMotionCommon {w:#x} {mode}");
                no_diff(
                    &mut a,
                    &mut b,
                    &format!("Native_PlayMotionCommon {w:#x} {mode}"),
                );
            }
            poke(&mut a, &mut b, m, id);
        }
        let blk = Mem::read::<u32>(&b, c + 0x48);
        if blk != 0 {
            assert_eq!(
                orig(&mut a, 0x0086_c0c0, &[blk]),
                hp::atel_get_my_buffer_size(&b, blk),
                "AtelGetMyBufferSize {c:#x}"
            );
            for i in 0..4 {
                let ra = orig_float(&mut a, 0x0086_ab80, &[blk, i]);
                let rb = hp::atel_get_bin_hdr_float(&b, blk, i);
                assert!(
                    same_float(ra, ffx_bridge::x87::store(rb)),
                    "0x0086ab80 {c:#x} {i}"
                );
            }
        }
    }
    let s = a.alloc(0x40).unwrap();
    assert_eq!(b.alloc(0x40).unwrap(), s);
    let digits = b"0123456789abcdefABCDEFxg$ ";
    for _ in 0..300 {
        let mut text: Vec<u8> = match rng.below(3) {
            0 => b"0x".to_vec(),
            1 => b"$".to_vec(),
            _ => Vec::new(),
        };
        for _ in 0..rng.below(12) {
            text.push(digits[usize::try_from(rng.below(26)).unwrap()]);
        }
        text.push(0);
        for o in [&mut a, &mut b] {
            Mem::write_bytes(o, s, &text);
        }
        assert_eq!(
            orig(&mut a, 0x0087_7660, &[s]),
            hp::atel_parse_hex(&b, s),
            "0x00877660 {text:?}"
        );
    }
    for _ in 0..40 {
        let n = if rng.below(4) == 0 {
            rng.below(0x200)
        } else {
            rng.below(0x240) - 0x20
        };
        let ra = orig(&mut a, 0x0086_f460, &[n]);
        let rb = hp::atel_set_bind_mask(&mut b, n);
        if n.cast_signed() < 0x200 {
            assert_eq!(ra, rb, "0x0086f460 {n}");
        }
        no_diff(&mut a, &mut b, &format!("0x0086f460 {n}"));
    }
    let win = a.rebase(0x0132_6d70);
    for k in (0..16 * 0x2c).step_by(4) {
        poke(&mut a, &mut b, win + k, rng.next());
    }
    assert_eq!(
        orig(&mut a, 0x0086_3b10, &[]),
        hp::atel_clear_mes_win_work(&mut b),
        "AtelClearMesWinWork"
    );
    no_diff(&mut a, &mut b, "AtelClearMesWinWork");
    // the callback queues: register named callbacks, then run them; a
    // callback that returns non-zero (AtelGetSnd3dWork) is done, one
    // that returns 0 (0x008719c0 on an empty string) stays
    let done = a.rebase(0x0087_73f0);
    let wait = a.rebase(0x0087_19c0);
    let que = a.rebase(0x0132_7f50);
    for q in 0..2 {
        let zero = [0u8; 0x100];
        for o in [&mut a, &mut b] {
            Mem::write_bytes(o, que + q * 0x100, &zero);
        }
        for k in 0..17 {
            let name: Vec<u8> = (0..rng.below(10))
                .map(|_| b'a' + u8::try_from(rng.below(26)).unwrap())
                .chain([0])
                .collect();
            for o in [&mut a, &mut b] {
                Mem::write_bytes(o, s, &name);
            }
            let f = if k % 3 == 2 { wait } else { done };
            let ra = orig(&mut a, 0x0087_51b0, &[q, f, s]);
            assert_eq!(
                ra,
                hp::atel_regist_callback_que2(&mut b, q, f, s),
                "AtelRegistCallbackQue2 {q} {k}"
            );
            no_diff(&mut a, &mut b, &format!("AtelRegistCallbackQue2 {q} {k}"));
        }
        for k in 0..20 {
            let ra = orig(&mut a, 0x0087_50b0, &[q]);
            let rb = hp::atel_exec_callback_que(&mut Run(&mut b), q);
            assert_eq!(ra, rb, "AtelExecCallbackQue {q} {k}");
            no_diff(&mut a, &mut b, &format!("AtelExecCallbackQue {q} {k}"));
            if k % 4 == 3 {
                // let a waiting callback finish: its word non-zero
                poke(&mut a, &mut b, que + q * 0x100 + 4, 0x41_u32);
            }
        }
    }
}

/// The ctrl work's default start callback (`0x008683e0`) on every ctrl
/// work in use: each worker runs until its init is done (Rust's
/// `Script_Run` against the original), then the signals are decoded.
#[test]
fn ctrl_start_matches_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut compared = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        // A worker Script_Run skips (+0x34 bit 1 clear; a field worker in
        // battle: bit 2 clear, +0x36 bit 6 clear and MsBattleCheck set; the
        // ctrl's mode 1, or mode 2 without +0x36 bit 15)
        // never sets its init bit, and the callback's loop would spin on
        // it for ever, in the game as here: a snapshot's ctrl work is past
        // its start. Such workers get the init bit in both runs.
        let in_battle = orig(&mut b, run::MS_BATTLE_CHECK, &[]) != 0;
        orig(&mut a, run::MS_BATTLE_CHECK, &[]);
        let mode = Mem::read::<u8>(&b, c + ctrl::MODE) & 3;
        for i in 0..u32::from(Mem::read::<u16>(&b, c + 0xc)) {
            let w = v::atel_get_basic_work(&mut Run(&mut b), i.cast_signed());
            orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
            let f = Mem::read::<u16>(&b, w + worker::FLAGS);
            let f2 = Mem::read::<u16>(&b, w + worker::FLAGS2);
            if mode == 1
                || (mode == 2 && f2 & 0x8000 == 0)
                || f & 2 == 0
                || (f & 4 == 0 && f2 & 0x40 == 0 && in_battle)
            {
                poke(&mut a, &mut b, w + worker::FLAGS, f | 0x10);
            }
        }
        clear_stacks(&mut a, &mut b);
        let ra = match a.call(a.rebase(0x0086_83e0), Conv::Cdecl, &[c]) {
            Ok(r) => r.eax,
            Err(e) => {
                eprintln!("ctrl {c:#x}: the original stops: {e}");
                continue;
            }
        };
        let rb = hp::atel_ctrl_start(&mut Run(&mut b), c);
        assert_eq!(ra, rb, "0x008683e0 {c:#x}");
        native_no_diff(&mut a, &mut b, &format!("0x008683e0 {c:#x}"));
        compared += 1;
    }
    eprintln!("{compared} ctrl works started");
}

/// Round 18's natives on the inputs random small arguments miss: the
/// command-record natives `0x70d6`/`0x70e7` with commands that have a
/// record, `0x70ee`'s thresholds, the counters, item slots and flags with
/// prepared globals, on every actor of every ctrl work in use.
#[test]
#[allow(clippy::too_many_lines)] // one block per native
fn round18_natives_match_the_original() {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let int = |v: u32| (stack::INT, v);
    let mut compared = 0;
    let mut records = 0;
    let mut models = 0;
    for c in ctrls {
        let mut pair = fresh_run_pair(&pages, &exe, c);
        let ws = {
            let (a, b) = (&mut pair.0, &mut pair.1);
            actors(a, b)
        };
        for &w in ws.iter().take(12) {
            // the command records: try each command on the original first
            for cmd in [
                0x3000, 0x3001, 0x3003, 0x3010, 0x2000, 0x2001, 0x1000, 0x4000,
            ] {
                for (sym, push) in [
                    (0x007a_80b0, vec![int(0), int(0), int(cmd)]),
                    (0x007a_8680, vec![int(cmd)]),
                ] {
                    let (a, b) = (&mut pair.0, &mut pair.1);
                    let s = w + worker::STACK;
                    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
                    let args = v::script_worker_level(b, w, lvl) + 0x2c;
                    poke(a, b, s, 0i32);
                    for &(k, x) in &push {
                        push_both(a, b, s, k, x);
                    }
                    clear_stacks(a, b);
                    if a.call(a.rebase(sym), Conv::Cdecl, &[w, args, s]).is_err() {
                        // no record for this command: the original reads
                        // through null, as the port would
                        pair = fresh_run_pair(&pages, &exe, c);
                        continue;
                    }
                    // the original ran: start again from the same state
                    pair = fresh_run_pair(&pages, &exe, c);
                    let (a, b) = (&mut pair.0, &mut pair.1);
                    native_case(a, b, w, sym, &push, &format!("{sym:#x} command {cmd:#x}"));
                    records += 1;
                }
            }
            let (a, b) = (&mut pair.0, &mut pair.1);
            // 0x70ee's thresholds on a + b
            for sum in [
                -5i32, 0, 0x1f, 0x20, 0x2f, 0x30, 0x3e, 0x3f, 0x4f, 0x50, 0x7fff,
            ] {
                let push = [int(sum.cast_unsigned()), int(0)];
                native_case(a, b, w, 0x007a_7b90, &push, &format!("0x70ee {sum}"));
                compared += 1;
            }
            // counters and flags with prepared bytes
            for v in [3u8, 1, 0, 0xff, 0x80] {
                poke(a, b, a.rebase(0x0112_ca0d), v);
                native_case(a, b, w, 0x007a_2c10, &[], &format!("0x710c {v}"));
                poke(a, b, a.rebase(0x0112_c9fc), v);
                native_case(
                    a,
                    b,
                    w,
                    0x007a_73d0,
                    &[int(u32::from(v))],
                    &format!("0x70a0 {v}"),
                );
                poke(a, b, a.rebase(0x0112_ca10), v);
                native_case(a, b, w, 0x007a_3a80, &[], &format!("0x710b {v}"));
                poke(a, b, a.rebase(0x0113_63ec), v & 0x1f);
                native_case(a, b, w, 0x007a_3560, &[int(0x1234)], &format!("0x70f7 {v}"));
                poke(a, b, a.rebase(0x0112_c9e4), v);
                native_case(a, b, w, 0x007a_2eb0, &[], &format!("0x709a {v}"));
                compared += 5;
            }
            // 0x70df: the pointer and the flag
            // an i16 inside the worker (both copies are equal there)
            let rec = w + 0x500;
            poke(a, b, rec, 0x0012_i16);
            for (flag, ptr, v) in [
                (0u8, rec, 0x12_0000u32),
                (1, 0, 0x12_0000),
                (1, rec, 0x12_0000),
                (1, rec, 0x13_0000),
                (1, rec, 0xffff_0000),
            ] {
                poke(a, b, a.rebase(0x0112_a8e0), flag);
                poke(a, b, a.rebase(0x0112_a9f0), ptr);
                native_case(
                    a,
                    b,
                    w,
                    0x007a_4ee0,
                    &[int(v)],
                    &format!("0x70df {flag} {ptr:#x} {v:#x}"),
                );
                compared += 1;
            }
            // the item slots: one free in the middle, then none
            // (only where g_BtlItemMem2 holds a mapped table: the original
            // 0x70f1 is tried first)
            let id = u32::from(Mem::read::<u8>(b, v::atel_get_battle_id_adrs2(w)));
            let (s0, args0) = (w + worker::STACK, 0);
            let mapped = id != 0xff
                && a.call(a.rebase(0x007a_2d30), Conv::Cdecl, &[w, args0, s0])
                    .is_ok();
            pair = fresh_run_pair(&pages, &exe, c);
            let (a, b) = (&mut pair.0, &mut pair.1);
            if mapped {
                let base = Mem::read::<u32>(b, b.rebase(0x0231_0cd8)) + id * 0x478;
                for k in 0..20 {
                    poke(a, b, base + 2 * k, if k == 7 { 0xff_u16 } else { 0x10_u16 });
                }
                native_case(a, b, w, 0x007a_2e60, &[int(0x42)], "0x70f2 slot 7");
                native_case(a, b, w, 0x007a_2e60, &[int(0x43)], "0x70f2 full");
                native_case(a, b, w, 0x007a_2d30, &[], "0x70f1");
                compared += 3;
            }
            // the model natives with inexact values where they read them:
            // 0x705c's scale (+0x3a4 times the model's +0x4f4) and 0x7092's
            // height (the model's +0x16c), on the actor selecting itself
            let sel = 0xffff_fff3_u32;
            if id != 0xff && orig(b, 0x0079_47f0, &[id, sel, 0, 1]) == id {
                orig(a, 0x0079_47f0, &[id, sel, 0, 1]);
                let chr = orig(b, 0x0079_4020, &[id]);
                orig(a, 0x0079_4020, &[id]);
                let m = Mem::read::<u32>(b, chr);
                if m != 0 {
                    poke(a, b, chr + 0x3a4, 1.3f32.to_bits());
                    poke(a, b, chr + 0x3a8, 2.7f32.to_bits());
                    poke(a, b, m + 0x4f4, 1.1f32.to_bits());
                    poke(a, b, m + 0x16c, 1.1f32.to_bits());
                    native_case(a, b, w, 0x007a_2df0, &[int(sel)], "0x705c scale");
                    native_case(a, b, w, 0x007a_5f10, &[int(sel)], "0x7092 height");
                    models += 2;
                }
            }
            // the map-part natives on parts that exist (0x0091a3d0 counts
            // them)
            let parts = orig(b, 0x0091_a3d0, &[]).min(4);
            orig(a, 0x0091_a3d0, &[]);
            for i in 0..parts {
                let f = (stack::FLOAT, 1.25f32.to_bits());
                native_case(
                    a,
                    b,
                    w,
                    0x0091_c280,
                    &[int(i), f],
                    &format!("0x8054 part {i}"),
                );
                native_case(
                    a,
                    b,
                    w,
                    0x0091_c230,
                    &[int(i), f],
                    &format!("0x8053 part {i}"),
                );
                compared += 2;
            }
            // 0x7126 on a negative index (an actor without parts reads through
            // null in the original)
            native_case(a, b, w, 0x007a_8550, &[int(u32::MAX)], "0x7126 -1");
            compared += 1;
        }
    }
    eprintln!(
        "{compared} round-18 cases compared, {records} with a command record, {models} with a model"
    );
    assert!(compared > 0, "nothing ran");
}

/// One round-19 case: the original at `sym` with `args`, then the bridge
/// `f` on the other side; `EAX` compared when `eax` is set, every byte
/// written always. An original that stops (reading through a null table
/// the snapshot leaves unset) skips the case and starts the pair again.
#[allow(clippy::too_many_arguments)] // the pair, its source and the case
fn r19_case(
    pages: &std::sync::Arc<Pages>,
    exe: &std::path::Path,
    c: u32,
    pair: &mut (Oracle, Oracle),
    sym: u32,
    args: &[u32],
    eax: bool,
    what: &str,
    f: impl FnOnce(&mut Run) -> u32,
) -> bool {
    let (a, b) = (&mut pair.0, &mut pair.1);
    clear_stacks(a, b);
    let ra = match a.call(a.rebase(sym), Conv::Cdecl, args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{what}: the original stops: {e}");
            *pair = fresh_run_pair(pages, exe, c);
            return false;
        }
    };
    assert!(ra.st0_raw.is_none(), "{what}: the original returns ST0");
    let rb = f(&mut Run(b));
    if eax {
        assert_eq!(ra.eax, rb, "{what}: EAX");
    }
    native_no_diff(a, b, what);
    true
}

/// Round 19's setup functions (map jumps, player control, move blocks,
/// entrances, the event's workers, ctrl work defaults, triggers) against
/// the original on every ctrl work in use, with prepared flags where the
/// snapshot's leave a path cold.
#[test]
#[allow(clippy::too_many_lines)] // one block per function
fn round19_setup_matches_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut compared, mut stops) = (0, 0);
    let mut tally = |ok: bool| {
        if ok {
            compared += 1;
        } else {
            stops += 1;
        }
    };
    for c in ctrls {
        let mut p = fresh_run_pair(&pages, &exe, c);
        let n = u32::from(Mem::read::<u16>(&p.1, c + 0xc)).min(12);
        let out = p.0.alloc(0x10).unwrap();
        assert_eq!(p.1.alloc(0x10).unwrap(), out);
        for i in 0..n {
            let w = v::atel_get_basic_work(&mut p.1, i.cast_signed());
            orig(&mut p.0, ATEL_GET_BASIC_WORK, &[i]);
            let what = |f: &str| format!("{f} worker {i} of {c:#x}");
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_a970,
                &[i],
                true,
                &what("0x0086a970"),
                |r| hp::atel_get_battle_id_adrs(r, i),
            ));
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_b3a0,
                &[i],
                true,
                &what("0x0086b3a0"),
                |r| hp::atel_get_event_char_handle_from_id(r, i),
            ));
            for (x, y, z) in [(out, out + 4, out + 8), (0, out, 0), (out + 8, 0, out)] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0086_b1e0,
                    &[i, x, y, z],
                    true,
                    &what("0x0086b1e0"),
                    |r| hp::atel_get_event_actor_pos(r, i, x, y, z),
                ));
            }
            for mode in [0, 1] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0087_03b0,
                    &[w, mode],
                    true,
                    &what(&format!("0x008703b0 {mode}")),
                    |r| hp::atel_set_move_hokan_mode(r, w, mode),
                ));
            }
            // the move blocks, where every level's points into the worker
            let blocks = (0..9).all(|l| {
                let q = Mem::read::<u32>(&p.1, w + 0x170 + l * 0x4c);
                (w..w + 0x1000).contains(&q)
            });
            if blocks {
                for l in [u32::MAX, 0, 1, 3, 8, 12] {
                    // with and without the block turning
                    for turn in [false, true] {
                        let lv = if l == u32::MAX {
                            u32::from(Mem::read::<u8>(&p.1, w + 0x32))
                        } else {
                            l
                        };
                        let lv = lv.min(8);
                        let blk = Mem::read::<u32>(&p.1, w + 0x170 + lv * 0x4c);
                        if turn {
                            let f = Mem::read::<u16>(&p.1, blk + 2);
                            poke(&mut p.0, &mut p.1, blk + 2, f | 0x4000);
                        }
                        // every level sharing the block (on the second
                        // pass), distinct values in it, others in the
                        // blocks the levels get
                        if turn {
                            for j in 0..9 {
                                poke(&mut p.0, &mut p.1, w + 0x170 + j * 0x4c, blk);
                            }
                        }
                        for (k, off) in [0x28, 0x2c, 0x30, 0x38, 0x44, 0x48, 8, 0xc]
                            .into_iter()
                            .enumerate()
                        {
                            #[allow(clippy::cast_precision_loss)] // small
                            let x = (k as f32 + 1.5) * 3.25;
                            poke(&mut p.0, &mut p.1, blk + off, x.to_bits());
                            for j in 0..9 {
                                let own = w + 0x5b8 + j * 0x4c;
                                if own != blk {
                                    poke(&mut p.0, &mut p.1, own + off, (-x).to_bits());
                                }
                            }
                        }
                        tally(r19_case(
                            &pages,
                            &exe,
                            c,
                            &mut p,
                            0x0086_3e10,
                            &[w, l],
                            true,
                            &what(&format!("0x00863e10 level {l} turn {turn}")),
                            |r| hp::script_stop_move(r, w, l),
                        ));
                    }
                }
            }
            // every bound-effect bit set, so each kind clears its own
            let f = Mem::read::<u16>(&p.1, w + 0x38);
            poke(&mut p.0, &mut p.1, w + 0x38, f | 0xe0);
            for k in [1, 0, 2, 3] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0086_2680,
                    &[w, k],
                    true,
                    &what(&format!("0x00862680 {k}")),
                    |r| hp::atel_bind_eff2_off_some(r, w, k),
                ));
            }
            for e in [u32::MAX, 0, 1, 3, 100] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0086_c020,
                    &[w, e],
                    true,
                    &what(&format!("0x0086c020 {e}")),
                    |r| hp::event_get_entrance_at(r, w, e),
                ));
            }
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_bfe0,
                &[w],
                true,
                &what("Event_GetEntrance"),
                |r| hp::event_get_entrance(r, w),
            ));
            // a trigger worker with every trigger enabled, then none
            for bits in [0xffu8, 0] {
                poke(&mut p.0, &mut p.1, w + 0xab, bits);
                for k in [0, 1, 2, 5] {
                    tally(r19_case(
                        &pages,
                        &exe,
                        c,
                        &mut p,
                        0x0087_64f0,
                        &[0xffff, i, k],
                        true,
                        &what(&format!("0x008764f0 {bits:#x} {k}")),
                        |r| hp::atel_request_trigger(r, 0xffff, i, k),
                    ));
                }
            }
            for l in [u32::MAX, 0, 2] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0087_1730,
                    &[i, l],
                    false,
                    &what(&format!("0x00871730 {l}")),
                    |r| {
                        hp::atel_skip_req_level2(r, i, l);
                        0
                    },
                ));
            }
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_41c0,
                &[i, 0],
                true,
                &what("0x008641c0"),
                |r| hp::atel_decode_command(r, i, 0),
            ));
        }
        let what = |f: &str| format!("{f} on {c:#x}");
        // player control: as it is, then with a control bit on every
        // type-1 worker, then gated by the ctrl work's flags
        tally(r19_case(
            &pages,
            &exe,
            c,
            &mut p,
            0x0087_1ab0,
            &[],
            false,
            &what("AtelUcOn"),
            |r| {
                hp::atel_uc_on(r);
                0
            },
        ));
        for i in 0..n {
            let w = v::atel_get_basic_work(&mut p.1, i.cast_signed());
            orig(&mut p.0, ATEL_GET_BASIC_WORK, &[i]);
            for (off, bit) in [(0x34, 0x200u16), (0x36, 8)] {
                let f = Mem::read::<u16>(&p.1, w + off);
                poke(&mut p.0, &mut p.1, w + off, f | bit);
            }
        }
        for (off, v) in [(0, 0u8), (0, 0x20), (1, 4), (3, 0x80)] {
            let f = Mem::read::<u8>(&p.1, c + off);
            let v = if v == 0 { f } else { f | v };
            poke(&mut p.0, &mut p.1, c + off, v);
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0087_1ab0,
                &[],
                false,
                &what(&format!("AtelUcOn +{off} {v:#x}")),
                |r| {
                    hp::atel_uc_on(r);
                    0
                },
            ));
            poke(&mut p.0, &mut p.1, c + off, f);
        }
        // the last two flag bytes told apart (an index past the end reads
        // the last)
        let fl = orig(&mut p.1, 0x0078_5250, &[]);
        orig(&mut p.0, 0x0078_5250, &[]);
        poke(&mut p.0, &mut p.1, fl + 0x7e, 0u8);
        poke(&mut p.0, &mut p.1, fl + 0x7f, 0xffu8);
        for i in [0u32, 1, 7, 8, 9, 100, 1023, 1024, 5000, u32::MAX - 2] {
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_b9e0,
                &[i],
                true,
                &what(&format!("0x0086b9e0 {i}")),
                |r| hp::atel_get_help_event_flag(r, i),
            ));
        }
        for i in [0u32, 1, 7, 14, 15, 20] {
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0087_3b50,
                &[i],
                true,
                &what(&format!("0x00873b50 {i}")),
                |r| hp::atel_get_summon_name_for_final_battle_selector(r, i),
            ));
        }
        for chr in [0u32, 1, 2, 5, 8, 0xff] {
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_ae90,
                &[chr],
                true,
                &what(&format!("0x0086ae90 {chr}")),
                |r| hp::atel_get_char_handle_from_bind_pc(r, chr),
            ));
        }
        let list = p.1.rebase(0x00c8_6708);
        let ids: Vec<u32> = (0..4)
            .map(|k| u32::from(Mem::read::<u16>(&p.1, list + 8 * k + 2 + 2 * (k & 1))))
            .collect();
        for mode in [0, 1, 2, 3] {
            for &id in &ids {
                for min in [0, 1, 3, u32::MAX] {
                    tally(r19_case(
                        &pages,
                        &exe,
                        c,
                        &mut p,
                        0x0087_2aa0,
                        &[mode, id, min],
                        true,
                        &what(&format!("0x00872aa0 {mode} {id} {min}")),
                        |r| hp::atel_check_capt_monster(r, mode, id, min),
                    ));
                }
            }
        }
        // the dictionary index: as it is, then a table (bit 15)
        let blk = Mem::read::<u32>(&p.1, p.1.rebase(0x0132_5be8));
        for table in [false, true] {
            if table && blk != 0 {
                poke(&mut p.0, &mut p.1, blk + 0x1e, 0x8003_u16);
                let t = Mem::read::<u32>(&p.1, blk + 0x28).wrapping_add(blk);
                for k in 0..5u16 {
                    poke(&mut p.0, &mut p.1, t + 2 * u32::from(k), 0x100 + k);
                }
            }
            for i in [0u32, 1, 2, 3, 9, u32::MAX] {
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0087_0ef0,
                    &[i],
                    true,
                    &what(&format!("0x00870ef0 {table} {i}")),
                    |r| hp::atel_set_save_dic_index(r, i),
                ));
            }
        }
        // ctrl work defaults: in use as they are, then with callbacks
        // cleared
        for i in 0..3u32 {
            let cw = v::atel_get_ctrl_work(&p.1, i);
            for clear in [false, true] {
                if clear {
                    for off in [0x1c, 0x20, 0x2c, 0x4c, 0x58, 0x5c, 0x60] {
                        if off != 0x1c || i == 1 {
                            poke(&mut p.0, &mut p.1, cw + off, 0u32);
                        }
                    }
                }
                tally(r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0087_1190,
                    &[i],
                    true,
                    &what(&format!("0x00871190 {i} {clear}")),
                    |r| hp::atel_set_up_ctrl_work(r, i),
                ));
            }
        }
        for map in [0u32, u32::MAX, 1] {
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_dca0,
                &[map],
                true,
                &what(&format!("AtelMapLoad {map}")),
                |r| hp::atel_map_load(r, map),
            ));
        }
        // the map jumps: without the jump bit, then with it and each fade
        let e = orig(&mut p.1, 0x0088_c830, &[]);
        orig(&mut p.0, 0x0088_c830, &[]);
        for (bit, own, kind, map) in [
            (false, 0, 0u8, 0x109u16),
            (true, 0, 0, 0x109),
            (true, 0, 0, 0x10),
            (true, 1, 0, 0x10),
            (true, 1, 1, 0x10),
            (true, 1, 2, 0x10),
        ] {
            let f = Mem::read::<u8>(&p.1, c);
            poke(&mut p.0, &mut p.1, c, if bit { f | 0x80 } else { f & 0x7f });
            poke(&mut p.0, &mut p.1, e + 0x11, kind);
            poke(&mut p.0, &mut p.1, e + 0x12, 7u8);
            poke(&mut p.0, &mut p.1, e + 2, map);
            let ctx = p.1.rebase(v::CONTEXT.sym);
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_ff40,
                &[u32::from(map), 1, own],
                true,
                &what(&format!("AtelSetEventJump2 {bit} {own} {kind}")),
                |r| hp::atel_set_event_jump2(r, u32::from(map), 1, own),
            ));
            poke(&mut p.0, &mut p.1, ctx, c);
            let f = Mem::read::<u8>(&p.1, c);
            poke(&mut p.0, &mut p.1, c, if bit { f | 0x80 } else { f & 0x7f });
            tally(r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_fec0,
                &[u32::from(map), 1],
                true,
                &what(&format!("AtelSetEventJump {bit} {map:#x}")),
                |r| hp::atel_set_event_jump(r, u32::from(map), 1),
            ));
            poke(&mut p.0, &mut p.1, ctx, c);
        }
    }
    eprintln!("{compared} round-19 setup cases compared ({stops} stopped in the original)");
    assert!(compared > 0, "nothing ran");
}

/// [`fresh_run_pair`] with four point lights (`g_SgPLights` `0x013008cc`,
/// count `0x013008d0`; the snapshots have none), so the light natives get
/// past their table.
fn r19_pair(pages: &std::sync::Arc<Pages>, exe: &std::path::Path, c: u32) -> (Oracle, Oracle) {
    let (mut a, mut b) = fresh_run_pair(pages, exe, c);
    let t = a.alloc(4 * 0x2c).unwrap();
    assert_eq!(b.alloc(4 * 0x2c).unwrap(), t);
    for k in 0..4 * 0x2c_u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte pattern
        poke(&mut a, &mut b, t + k, (k * 7 + 3) as u8 & 0x3f);
    }
    let (at, n) = (a.rebase(0x0130_08cc), a.rebase(0x0130_08d0));
    poke(&mut a, &mut b, at, t);
    poke(&mut a, &mut b, n, 4u32);
    (a, b)
}

/// Natives that can read an uninitialised local of the original
/// (`0x0099` and `0x009c` with no points, `0x00f7` with a negative slot),
/// tested on chosen arguments only; `0x7076`, whose `MsSetBallSplineInit`
/// and `MsSetBallSpline` copy callee locals left unwritten (`MsGetChrPos`
/// without an actor, `Ch_OptposGet` without the element) into the actor
/// and spline records, so the two stacks' leftovers differ; and `0xb008`'s start, which leaves `EAX` as it was
/// (the dispatch ignores it) and writes nothing.
const R20_UNINIT: [u32; 5] = [
    0x0085_fd80,
    0x0086_0000,
    0x0085_8290,
    0x007a_56f0,
    0x0076_eb00,
];

/// Round 19's natives (`native_mods`) on every actor of every ctrl work in
/// use, each with several random argument stacks: integers small and
/// large, floats in and out of the angle range; results and memory.
#[test]
#[allow(clippy::too_many_lines)] // the random loop and the targeted cases
fn round19_natives_match_the_original() {
    use ffx_script::vm::bridge::native::{self, Native};
    use ffx_script::vm::bridge::native_mods::PORTED;
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut rng = Xorshift(0x5eed_0019);
    let (mut compared, mut stops) = (0, 0);
    for c in ctrls {
        let mut pair = Some(r19_pair(&pages, &exe, c));
        let n = {
            let (a, b) = pair.as_mut().unwrap();
            actors(a, b).len().min(8)
        };
        // 0x100c on integers past f32's 24 bits, with and without SSE2:
        // the product's rounding to f32 shows in the x87 truncation's low
        // word (SSE2's gives 0x80000000 for all of them)
        {
            let (a, b) = pair.get_or_insert_with(|| r19_pair(&pages, &exe, c));
            let w = actors(a, b)[0];
            let sse = a.rebase(ffx_bridge::x87::SSE2_AVAILABLE.sym);
            let had = Mem::read::<u32>(b, sse);
            for flag in [had, 0] {
                poke(a, b, sse, flag);
                for x in [0x0100_0001u32, 0x0123_4567, 0x7fff_ffff, 3] {
                    let what = format!("0x100c {x:#x} sse {flag}");
                    native_case(a, b, w, 0x0087_78f0, &[(stack::INT, x)], &what);
                    compared += 1;
                }
            }
            poke(a, b, sse, had);
        }
        // the natives that write through popped pointers, with real ones,
        // and the light-direction ones on lights that exist
        {
            let (a, b) = pair.get_or_insert_with(|| r19_pair(&pages, &exe, c));
            let w = actors(a, b)[0];
            let out = a.alloc(0x20).unwrap();
            assert_eq!(b.alloc(0x20).unwrap(), out);
            let int = |v: u32| (stack::INT, v);
            let f = |v: f32| (stack::FLOAT, v.to_bits());
            let ptrs = [int(out), int(out + 4), int(out + 8)];
            native_case(a, b, w, 0x00a7_7a70, &ptrs, "0x4037");
            // worker 0's model's id bits apart (0x502f/0x5031 read +0x828)
            let w0 = v::atel_get_basic_work(b, 0);
            orig(a, ATEL_GET_BASIC_WORK, &[0]);
            let m0 = Mem::read::<u32>(b, w0 + 0x9c);
            if m0 != 0 {
                poke(a, b, m0 + 0x828, 0x0001_8035_u32);
                native_case(a, b, w, 0x00a7_9d60, &[int(0)], "0x502f");
                native_case(a, b, w, 0x00a7_9e00, &[int(0)], "0x5031");
                compared += 2;
            }
            for i in 0..4 {
                let mut push = vec![int(i)];
                push.extend(ptrs);
                native_case(a, b, w, 0x00a7_84c0, &push, &format!("0x4027 {i}"));
                native_case(a, b, w, 0x00a7_8560, &push, &format!("0x4029 {i}"));
                let dir = [int(i), f(0.25), f(-1.5), f(3.0)];
                native_case(a, b, w, 0x00a7_7f70, &dir, &format!("0x4011 {i}"));
                native_case(a, b, w, 0x00a7_7fd0, &dir, &format!("0x4012 {i}"));
                compared += 4;
            }
            compared += 1;
        }
        // group 0's spline positions (0x0099, 0x009c) on a three-point
        // table: with no points their out vector stays the original's
        // uninitialised local, so the random loop leaves them out
        {
            let (a, b) = pair.get_or_insert_with(|| r19_pair(&pages, &exe, c));
            let w = actors(a, b)[0];
            let word = 0x100;
            let pv = v::atel_get_real_var_adrs(&mut Run(b), w, word);
            poke(a, b, pv, 3u32);
            for k in 0..12u32 {
                #[allow(clippy::cast_precision_loss)] // small
                let x = (k as f32 - 5.0) * 1.25;
                poke(a, b, pv + 0x10 + 4 * k, x.to_bits());
            }
            for sym in [0x0085_fd80, 0x0086_0000] {
                for n in [0u32, 1, 2] {
                    let push = [(stack::INT, word), (stack::INT, n)];
                    native_case(a, b, w, sym, &push, &format!("{sym:#x} n {n}"));
                    compared += 1;
                }
            }
            // 0x00f7's start: AtelGetPartyWorker reads below its locals
            // for a negative slot
            for slot in 0..4 {
                for n in [0u32, 2] {
                    let push = [(stack::INT, slot), (stack::INT, n)];
                    native_case(a, b, w, 0x0085_8290, &push, &format!("0x00f7 {slot} {n}"));
                    compared += 1;
                }
            }
            // 0x013a's choice window: open window 1, then the poll on each
            // state and the result
            for st in [0u16, 1, 2, 3, 4, 5] {
                let open = [1, 0, 0, 0, 0, 0, 0, 0].map(|x| (stack::INT, x));
                native_case(a, b, w, 0x0085_fb30, &open, &format!("0x013a start {st}"));
                let win = v::atel_get_mes_win_work(b, 1);
                poke(a, b, win + 0x14, st);
                native_case(a, b, w, 0x0085_fcd0, &[], &format!("0x013a poll {st}"));
                native_case(a, b, w, 0x0085_fd40, &[], &format!("0x013a result {st}"));
                compared += 3;
            }
            pair = None;
        }
        // group 0's flag and table natives on the values the snapshots
        // lack: flag words with every bit, the level-2 turn block, the
        // party slots, a bit index past the save block's set, each
        // MsEventEffectGetStat result
        {
            let (mut a, mut b) = r19_pair(&pages, &exe, c);
            let (a, b) = (&mut a, &mut b);
            let w = actors(a, b)[0];
            let int = |v: u32| (stack::INT, v);
            let me = u32::from(Mem::read::<u16>(b, w + 0x2e));
            poke(a, b, w + 0x36, 0xffff_u16);
            native_case(a, b, w, 0x0085_bae0, &[int(me)], "0x0118 all bits");
            let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
            let blk = Mem::read::<u32>(b, v::script_worker_level(b, w, lvl) + 0x48);
            // battle.snap's workers leave the block pointer 0xcdcdcdcd
            if blk >= 0x1_0000 && blk != 0xcdcd_cdcd {
                for x in [0x10_u16, 0x20, 0x01] {
                    poke(a, b, blk + 2, x);
                    native_case(a, b, w, 0x0085_a130, &[], &format!("0x00c3 {x:#x}"));
                }
            }
            for slot in [0u32, 1, 2, 0xff] {
                poke(a, b, w + 0x40, slot);
                native_case(a, b, w, 0x0085_bbe0, &[], &format!("0x00cd slot {slot}"));
            }
            for i in [639u32, 640, 641] {
                native_case(a, b, w, 0x0085_eb30, &[int(i)], &format!("0x0238 {i}"));
            }
            poke(a, b, w + worker::LEVEL, 2u8);
            let f = (stack::FLOAT, 1.5f32.to_bits());
            native_case(a, b, w, 0x0085_9510, &[f], "0x0022 level 2");
            for r in 0..8 {
                for o in [&mut *a, &mut *b] {
                    let at = o.rebase(0x0078_8bd0);
                    o.stub_return(at, r).unwrap();
                }
                native_case(a, b, w, 0x0085_f070, &[], &format!("0x00dd {r}"));
            }
            compared += 19;
        }
        // group 7's battle-global natives on values the snapshots lack: the
        // 0x70f8 list (count 3, read index 1) and its 0x70f6 reset, the
        // first 0x007b19d0 record with distinct bytes +0/+1, the pad
        // orientation, 0x7081's state byte
        {
            let (mut a, mut b) = r19_pair(&pages, &exe, c);
            let (a, b) = (&mut a, &mut b);
            let w = actors(a, b)[0];
            let (n, i) = (a.rebase(0x0113_63ec), a.rebase(0x0113_63ed));
            poke(a, b, n, 3u8);
            poke(a, b, i, 1u8);
            for k in 0..4 {
                native_case(a, b, w, 0x007a_36e0, &[], &format!("0x70f8 {k}"));
            }
            poke(a, b, i, 1u8);
            native_case(a, b, w, 0x007a_34e0, &[], "0x70f6");
            native_case(a, b, w, 0x007a_36e0, &[], "0x70f8 after 0x70f6");
            let r = orig(a, 0x007b_19d0, &[0]);
            assert_eq!(r, orig(b, 0x007b_19d0, &[0]));
            poke(a, b, r, 5u8);
            poke(a, b, r + 1, 9u8);
            native_case(a, b, w, 0x007a_3c00, &[], "0x710e");
            native_case(a, b, w, 0x007a_3c90, &[], "0x710f");
            for v in [5u32, 6, 0xff] {
                native_case(
                    a,
                    b,
                    w,
                    0x007a_7340,
                    &[(stack::INT, v)],
                    &format!("0x7099 {v}"),
                );
            }
            let st = a.rebase(0x0112_c022);
            for x in [0u8, 1, 2] {
                poke(a, b, st, x);
                native_case(a, b, w, 0x007a_5d60, &[], &format!("0x7081 poll {x}"));
            }
            compared += 14;
        }
        // 0x7076 with its three spline callees stubbed: its own record
        // writes and the order around them
        {
            let (mut a, mut b) = r19_pair(&pages, &exe, c);
            let (a, b) = (&mut a, &mut b);
            for o in [&mut *a, &mut *b] {
                for f in [0x007b_7c80, 0x007b_78c0, 0x007a_b010] {
                    let at = o.rebase(f);
                    o.stub_return(at, 0).unwrap();
                }
            }
            for w in actors(a, b) {
                if Mem::read::<u8>(b, v::atel_get_battle_id_adrs2(w)) == 0xff {
                    continue;
                }
                let push = [
                    (stack::INT, 0x1234),
                    (stack::FLOAT, 0.5f32.to_bits()),
                    (stack::INT, 1),
                ];
                native_case(a, b, w, 0x007a_56f0, &push, "0x7076 stubbed");
                compared += 1;
            }
        }
        // the polls without keep on a read that reports done (0): 5 for
        // 0x5062, 1 for 0x5064 (the reads stubbed on both sides)
        {
            let (mut a, mut b) = r19_pair(&pages, &exe, c);
            let w = actors(&mut a, &mut b)[0];
            let lvl = i32::from(Mem::read::<u8>(&b, w + worker::LEVEL));
            let args = v::script_worker_level(&b, w, lvl) + 0x2c;
            for o in [&mut a, &mut b] {
                for f in [0x0083_6a50, 0x0082_a040] {
                    let at = o.rebase(f);
                    o.stub_return(at, 0).unwrap();
                }
            }
            for (sym, keep) in [
                (0x00a7_9070, 0u32),
                (0x00a7_9290, 0),
                (0x00a7_9070, 1),
                (0x00a7_9290, 1),
            ] {
                for (off, x) in [(0, 7u32), (4, 2), (8, keep), (0xc, 0x55)] {
                    poke(&mut a, &mut b, args + off, x);
                }
                native_case(
                    &mut a,
                    &mut b,
                    w,
                    sym,
                    &[],
                    &format!("poll {sym:#x} keep {keep}"),
                );
                compared += 1;
            }
        }
        for i in 0..n {
            for &(sym, float) in &PORTED {
                if R20_UNINIT.contains(&sym) {
                    continue;
                }
                for round in 0..3 {
                    if pair.as_ref().is_some_and(|(_, b)| !ctx_ok(b)) {
                        pair = None;
                    }
                    let (a, b) = pair.get_or_insert_with(|| r19_pair(&pages, &exe, c));
                    let w = v::atel_get_basic_work(b, i32::try_from(i).unwrap());
                    orig(a, ATEL_GET_BASIC_WORK, &[u32::try_from(i).unwrap()]);
                    let s = w + worker::STACK;
                    poke(a, b, s, 0i32);
                    for _ in 0..8 {
                        let (kind, val) = match rng.below(4) {
                            0 => {
                                #[allow(clippy::cast_precision_loss)] // small
                                let f = (rng.below(4000) as f32 - 2000.0) / 300.0;
                                (stack::FLOAT, f.to_bits())
                            }
                            1 => (stack::INT, rng.next() & 0xffff),
                            _ => (stack::INT, rng.below(4)),
                        };
                        push_both(a, b, s, kind, val);
                    }
                    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
                    let args = v::script_worker_level(b, w, lvl) + 0x2c;
                    let what = format!("native {sym:#x} round {round} on worker {i} of {c:#x}");
                    clear_stacks(a, b);
                    a.call(a.rebase(BARE_RET[1]), Conv::Thiscall(0), &[])
                        .unwrap();
                    let ra = match a.call(a.rebase(sym), Conv::Cdecl, &[w, args, s]) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("{what}: the original stops: {e}");
                            stops += 1;
                            pair = None;
                            continue;
                        }
                    };
                    match native::find::<Run>(sym) {
                        Some(Native::Int(f)) => {
                            assert!(!float, "{what}");
                            assert!(ra.st0_raw.is_none(), "{what}: the original returns ST0");
                            let rb = f(&mut Run(b), w, args, s);
                            assert_eq!(ra.eax, rb, "{what}: EAX");
                        }
                        Some(Native::Float(f)) => {
                            assert!(float, "{what}");
                            let raw = ra.st0_raw.expect("ST0");
                            orig(a, FTOL2, &[]);
                            let rb = f(&mut Run(b), w, args, s);
                            let x = f80_to_f64(raw);
                            assert!(
                                x.to_bits() == rb.to_bits() || (x.is_nan() && rb.is_nan()),
                                "{what}: ST0 {x} vs {rb}"
                            );
                        }
                        None => panic!("{what}: not in find"),
                    }
                    native_no_diff(a, b, &what);
                    compared += 1;
                }
            }
        }
    }
    eprintln!("{compared} round-19 native calls compared ({stops} stopped in the original)");
    assert!(compared > 0, "nothing ran");
}
