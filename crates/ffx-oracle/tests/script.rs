//! The Atel VM accessors (`ffx_script::vm::bridge`) against the original,
//! on two oracles started alike from a snapshot of the running game: one
//! runs `FFX.exe`'s code, the other the Rust bridge on the same memory;
//! results and every written byte must agree. The workers, stacks and
//! variables are the snapshot's own (the field snapshot has the salvage
//! ship's event running). Needs `FFX_GAME_DIR` and `FFX_SNAPSHOT`; skipped
//! otherwise.
//!
//! Float results come back in `ST0`, which the oracle reads as an `f64`
//! (`fst qword`, NaN payloads kept): they compare bit for bit, NaNs
//! included. `_ftol2_sse` pops each one.

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
        ffx_oracle::f80_to_f64(&raw)
    }
    fn call_intrinsic2(&mut self, addr: u32, a: f64, b: f64) -> f64 {
        let at = self.0.rebase(TRAMPOLINE);
        let (x, y) = (a.to_bits(), b.to_bits());
        #[allow(clippy::cast_possible_truncation)] // the two halves
        let args = [x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32, addr];
        let r = self.0.call(at, Conv::Cdecl, &args).unwrap();
        let raw = r.st0_raw.expect("the intrinsic leaves ST0");
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        ffx_oracle::f80_to_f64(&raw)
    }
    fn call_intrinsic1(&mut self, addr: u32, a: f64) -> f64 {
        let at = self.0.rebase(TRAMPOLINE);
        let x = a.to_bits();
        #[allow(clippy::cast_possible_truncation)] // the two halves
        let args = [x as u32, (x >> 32) as u32, addr];
        let r = self.0.call(at + 19, Conv::Cdecl, &args).unwrap();
        let raw = r.st0_raw.expect("the intrinsic leaves ST0");
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        ffx_oracle::f80_to_f64(&raw)
    }
    fn scratch(&mut self) -> u32 {
        // the bottom of the emulated stack: no call reaches that deep, and
        // the comparisons leave the stack out, as the originals' locals are
        self.0.stack_range().start + 0x100
    }
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

/// Whether an `ST0` result is the bridge's `f32` bits, bit for bit (NaN
/// payloads and signs included, as `fld dword` widens them).
fn same_float(st0: f64, bits: u32) -> bool {
    ffx_oracle::same_float(st0, f64::from(f32::from_bits(bits)))
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
    settle(a, b);
}

/// Forgets the pages both sides wrote so far, once a diff passed: every
/// page left is equal on both sides or differs only where the diff's filter
/// (which looks at the page's own bytes) lets it, so it gives the same answer
/// again until a side writes the page, which marks it once more. Without it
/// each diff re-read every page any earlier case of the pair wrote.
fn settle(a: &mut Oracle, b: &mut Oracle) {
    a.clear_dirty();
    b.clear_dirty();
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
    settle(a, b);
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

/// The instruction budget of a [`run_pair`] call. Some natives and
/// programs wait on a loading thread, a critical section or a semaphore
/// that no thread of the emulator ever releases (`Native_0258_Poll` through
/// `TOSwapInternationalEnvExec`'s file reads, `sa_read_callback`,
/// `graphicProcessPs2DataLoadingCallback`, `task_create_in_group`): such a
/// call spins until the budget runs out, then the case counts as stopped.
/// At the oracle's default of 200M instructions each spin took 5-20 s,
/// most of the natives tests' time. The calls that return need far less:
/// with this budget the whole file stops exactly the cases it stopped with
/// the default, on the image and both snapshots (2026-10-21; the natives
/// of `native_mods` all return within 200k on the field snapshot). A call
/// that ran out of it would stop its case as a spin does, so keep it far
/// above the longest call that returns.
const RUN_BUDGET: usize = 20_000_000;

/// A pair of oracles on the snapshot with the run lock stubbed (the mutex
/// is a Windows object the emulator does not have; both sides call the
/// same stubs).
fn run_pair(pages: &std::sync::Arc<Pages>, exe: &std::path::Path) -> (Oracle, Oracle) {
    let mut pair = (
        Oracle::from_snapshot(exe, pages).unwrap(),
        Oracle::from_snapshot(exe, pages).unwrap(),
    );
    pair.0.set_max_instructions(RUN_BUDGET);
    pair.1.set_max_instructions(RUN_BUDGET);
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

const UNINIT: [Uninit; 9] = [
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
    // MsGetMoveTargetDist (0x007ab540, behind Atel_BtlMoveTargetDist and
    // so 0x700c/0x7085/0x7053) copies the position MsGetChrPos leaves at
    // [ebp-0x14] into the actor's +0x390; for MsGetChr's spare record (the
    // 0xcd-filled event workers' ids) nothing writes it. Same site and
    // stub as MsGetTargetDist's: [ebp-0x28..-0x8], nine words.
    Uninit {
        site: 0x007a_b546,
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
            let x = ffx_oracle::f80_to_f64(&raw);
            assert!(ffx_oracle::same_float(x, rb), "{what}: ST0 {x} vs {rb}");
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
/// (an import with no stub) is skipped and counted. Shard `shard` takes its
/// share of the ctrl works ([`CTRL_SHARDS`]).
#[allow(clippy::too_many_lines)] // the loop over every native and its checks
fn natives_match(groups: &[&[(u32, bool)]], rng_seed: u32, shard: usize) {
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
    let (mut compared, mut stops, mut mine) = (0, 0, 0);
    let mut seen = std::collections::BTreeSet::new();
    let random = groups
        .iter()
        .flat_map(|l| l.iter())
        .filter(|(s, _)| !R20_UNINIT.contains(s))
        .count();
    for (k, c) in ctrls.into_iter().enumerate() {
        let mut pair = None;
        let n = {
            let (a, b) = pair.get_or_insert_with(|| fresh_run_pair(&pages, &exe, c));
            actors(a, b).len().min(12)
        };
        if k % CTRL_SHARDS != shard {
            // another shard's ctrl work: only its random draws (6
            // arguments of 2 draws per native and actor)
            skip_draws(&mut rng, n * random * 6 * 2);
            continue;
        }
        mine += 1;
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
                        let x = ffx_oracle::f80_to_f64(&raw);
                        assert!(ffx_oracle::same_float(x, rb), "{what}: ST0 {x} vs {rb}");
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
        "shard {shard}: {compared} native calls compared ({stops} stopped in the original); never completed in this shard: {}",
        never.join(" ")
    );
    assert!(mine == 0 || compared > 0, "nothing ran");
}

/// The tests that loop over the ctrl works in use run as this many shards
/// (`<name>::shard0` ..): shard `k` takes ctrl works `k`, `k + CTRL_SHARDS`,
/// .. (the battle snapshot has three, the field snapshot one), and skips
/// the random draws of the others, so each case sees the same arguments
/// and the same state as in one whole run, and the shards together run
/// every case once.
const CTRL_SHARDS: usize = 3;

/// Advances `rng` by `n` draws (each `below`/`next` is one).
fn skip_draws(rng: &mut Xorshift, n: usize) {
    for _ in 0..n {
        rng.next();
    }
}

/// [`natives_match`] split by module so the test threads share the work,
/// each module in [`CTRL_SHARDS`] shards (`natives_match_the_original::moves`
/// still selects all of `moves`).
mod natives_match_the_original {
    use ffx_script::vm::bridge as m;

    macro_rules! shards {
        ($name:ident, $groups:expr, $seed:expr) => {
            mod $name {
                use super::m;
                #[test]
                fn shard0() {
                    super::super::natives_match($groups, $seed, 0);
                }
                #[test]
                fn shard1() {
                    super::super::natives_match($groups, $seed, 1);
                }
                #[test]
                fn shard2() {
                    super::super::natives_match($groups, $seed, 2);
                }
            }
        };
    }

    shards!(native, &[&m::native::PORTED], 0x5eed_0005);
    shards!(
        map_cam_msg,
        &[
            &m::native_map::PORTED,
            &m::native_cam::PORTED,
            &m::native_msg::PORTED,
        ],
        0x5eed_0105
    );
    shards!(moves, &[&m::native_move::PORTED], 0x5eed_0205);
    shards!(
        btl,
        &[&m::native_btl::PORTED, &m::native_btl2::PORTED],
        0x5eed_0305
    );
    shards!(
        sys_chr,
        &[&m::native_sys::PORTED, &m::native_chr::PORTED],
        0x5eed_0405
    );
    shards!(mods, &[&m::native_mods::PORTED], 0x5eed_0505);
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
/// tested on chosen arguments only; and `0xb008`'s start, which leaves `EAX` as it was
/// (the dispatch ignores it) and writes nothing.
const R20_UNINIT: [u32; 4] = [0x0085_fd80, 0x0086_0000, 0x0085_8290, 0x0076_eb00];

/// [`round19_natives`] in [`CTRL_SHARDS`] shards.
mod round19_natives_match_the_original {
    #[test]
    fn shard0() {
        super::round19_natives(0);
    }
    #[test]
    fn shard1() {
        super::round19_natives(1);
    }
    #[test]
    fn shard2() {
        super::round19_natives(2);
    }
}

/// Round 19's natives (`native_mods`) on every actor of every ctrl work in
/// use, each with several random argument stacks: integers small and
/// large, floats in and out of the angle range; results and memory. Shard
/// `shard` takes its share of the ctrl works ([`CTRL_SHARDS`]).
#[allow(clippy::too_many_lines)] // the random loop and the targeted cases
fn round19_natives(shard: usize) {
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
    let (mut compared, mut stops, mut mine) = (0, 0, 0);
    let random = PORTED
        .iter()
        .filter(|(s, _)| !R20_UNINIT.contains(s))
        .count();
    for (k, c) in ctrls.into_iter().enumerate() {
        let mut pair = Some(r19_pair(&pages, &exe, c));
        let n = {
            let (a, b) = pair.as_mut().unwrap();
            actors(a, b).len().min(8)
        };
        if k % CTRL_SHARDS != shard {
            // another shard's ctrl work: only its random draws (3 rounds of
            // 8 arguments of 2 draws per native and actor), so the rest
            // see the stream they see in one run
            skip_draws(&mut rng, n * random * 3 * 8 * 2);
            continue;
        }
        mine += 1;
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
            // 0xc059's command-menu panel bit set and cleared on commands
            // of both halves (0x3000-0x305f, 0x3060 on)
            for i in [0x3000u32, 0x3015, 0x3061] {
                for on in [0u32, 1, 0] {
                    let push = [(stack::INT, i), (stack::INT, on)];
                    native_case(a, b, w, 0x0087_82a0, &push, &format!("0xc059 {i} {on}"));
                }
            }
            compared += 23;
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
                            let x = ffx_oracle::f80_to_f64(&raw);
                            assert!(ffx_oracle::same_float(x, rb), "{what}: ST0 {x} vs {rb}");
                        }
                        None => panic!("{what}: not in find"),
                    }
                    native_no_diff(a, b, &what);
                    compared += 1;
                }
            }
        }
    }
    eprintln!(
        "shard {shard}: {compared} round-19 native calls compared ({stops} stopped in the original)"
    );
    assert!(mine == 0 || compared > 0, "nothing ran");
}

// ---- script round 22: the VM's helpers ----

/// `len` bytes allocated at the same address in both oracles.
fn alloc_both(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let at = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), at);
    at
}

/// `len` random bytes at `at` in both oracles.
fn fill_both(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32, len: u32) {
    for k in 0..len {
        #[allow(clippy::cast_possible_truncation)] // a random byte
        poke(a, b, at + k, rng.next() as u8);
    }
}

/// The original at `sym` against the bridge `f`: `EAX` (when `eax`) and
/// every byte written.
fn r22_case(
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    eax: bool,
    what: &str,
    f: impl FnOnce(&mut Run) -> u32,
) {
    clear_stacks(a, b);
    let ra = orig(a, sym, args);
    let rb = f(&mut Run(b));
    if eax {
        assert_eq!(ra, rb, "{what}: EAX");
    }
    native_no_diff(a, b, what);
}

/// A worker of type `kind` (header, a `0x400`-byte work area, an empty
/// stack, no battle actor) allocated in both oracles.
fn r22_worker(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, kind: u8) -> u32 {
    let hdr = alloc_both(a, b, 0x40);
    poke(a, b, hdr, kind);
    let w = alloc_both(a, b, 0x400);
    // random bytes from +0x80 (the free lists, the line table), the rest
    // zero: an empty stack's pop reports the level's PC through the
    // levels and the block (`+4`)
    fill_both(a, b, rng, w + 0x80, 0x40);
    poke(a, b, w, hdr);
    let block = alloc_both(a, b, 0x40);
    poke(a, b, w + 4, block);
    poke(a, b, w + worker::LEVEL, 0u8);
    poke(a, b, w + worker::STACK, 0i32);
    poke(a, b, w + worker::BATTLE_ID, 0xffu8);
    w
}

/// The bind masks, sprites, voice ids, name converters, line tables and
/// the angle wrap copy on one pair of oracles (image or snapshot).
#[allow(clippy::too_many_lines)] // one block per helper
fn r22_helpers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    use ffx_script::vm::bridge::helpers as hp;
    let mut n = 0;
    // Atel_ClearBindMask: random masks, bits inside, past the end, negative
    let masks = a.rebase(0x0132_6b90);
    for round in 0..48_u32 {
        // random masks have every bit somewhere: mostly a few bits only, so
        // the union shows each word
        if round % 4 == 0 {
            fill_both(a, b, rng, masks, 0x50);
        } else {
            for k in 0..0x14 {
                poke(a, b, masks + 4 * k, 0u32);
            }
            for _ in 0..rng.below(4) {
                let at = masks + 4 * rng.below(0x14);
                let v = Mem::read::<u32>(b, at) | 1 << rng.below(32);
                poke(a, b, at, v);
            }
        }
        let k = match round {
            0 => 0,
            1 => 1,
            2 => 31,
            3 => 32,
            4 => 0x1ff,
            5 => 0x200,
            6 => 0x7fff_ffff,
            7 => u32::MAX,
            8 => (-31_i32).cast_unsigned(),
            9 => (-33_i32).cast_unsigned(),
            10 => (-64_i32).cast_unsigned(),
            _ => rng.below(0x240),
        };
        r22_case(
            a,
            b,
            0x0086_3650,
            &[k],
            k.cast_signed() < 0x200,
            &format!("Atel_ClearBindMask({k:#x})"),
            |r| hp::atel_clear_bind_mask(r, k),
        );
        n += 1;
    }
    // the sprite setters, slots and values in and out of range
    let slots = [0, 3, 0x7f, 0x80, 1000, u32::MAX, (-5_i32).cast_unsigned()];
    let vals = [0, 1, 0x80, 0xff, 0x100, u32::MAX, 0x8000_0000, 0x7fff_ffff];
    for round in 0..40_u32 {
        let pick = |rng: &mut Xorshift, t: &[u32]| {
            if rng.below(4) == 0 {
                rng.next()
            } else {
                t[rng.below(u32::try_from(t.len()).unwrap()) as usize]
            }
        };
        let i = if round < 7 {
            slots[round as usize]
        } else {
            pick(rng, &slots)
        };
        let v: [u32; 3] = std::array::from_fn(|_| pick(rng, &vals));
        r22_case(
            a,
            b,
            0x0087_4ad0,
            &[i, v[0]],
            true,
            &format!("Atel_SetSpriteAlpha({i:#x}, {:#x})", v[0]),
            |r| hp::atel_set_sprite_alpha(r, i, v[0]),
        );
        r22_case(
            a,
            b,
            0x0087_49f0,
            &[i, v[0], v[1], v[2]],
            true,
            &format!("Atel_SetSpriteColor({i:#x}, {v:x?})"),
            |r| hp::atel_set_sprite_color(r, i, v[0], v[1], v[2]),
        );
        n += 2;
    }
    // Atel_GetVoiceStreamParams around the 21 threshold and at random
    let out = alloc_both(a, b, 8);
    for round in 0..40_u32 {
        let id = match round {
            0..=5 => {
                let q = [0, 20, 21, 99, 100, 104][round as usize] * 10000 + rng.below(10000);
                q << 12 | rng.below(0x1000)
            }
            _ => rng.next(),
        };
        let (pa, pb) = if round % 2 == 0 {
            (out, out + 4)
        } else {
            (out + 4, out)
        };
        fill_both(a, b, rng, out, 8);
        r22_case(
            a,
            b,
            0x0088_5ba0,
            &[id, pa, pb],
            true,
            &format!("Atel_GetVoiceStreamParams({id:#x})"),
            |r| hp::atel_get_voice_stream_params(r, id, pa, pb),
        );
        n += 1;
    }
    // the name converters: every byte once, random names, in place and
    // one byte down
    let buf = alloc_both(a, b, 0x220);
    for round in 0..24_u32 {
        let len = if round < 2 { 0xff } else { rng.below(24) };
        for k in 0..len {
            #[allow(clippy::cast_possible_truncation)] // bytes 1..=255
            let c = if round < 2 { k + 1 } else { rng.below(255) + 1 } as u8;
            poke(a, b, buf + 0x10 + k, c);
        }
        poke(a, b, buf + 0x10 + len, 0u8);
        let src = buf + 0x10;
        let dst = match round % 3 {
            0 => buf + 0x110,
            1 => src,
            _ => src - 1,
        };
        for (sym, name) in [
            (0x0087_5220, "Atel_NameToLang1"),
            (0x0087_5370, "Atel_NameFromLang1"),
        ] {
            r22_case(
                a,
                b,
                sym,
                &[src, dst],
                true,
                &format!("{name} round {round}"),
                |r| {
                    if sym == 0x0087_5220 {
                        hp::atel_name_to_lang1(r, src, dst)
                    } else {
                        hp::atel_name_from_lang1(r, src, dst)
                    }
                },
            );
            n += 1;
        }
    }
    // the line tables on synthetic workers of every type: no table, a
    // table of its own, and two tables laid over the worker (node 0 on the
    // free list's head, slot 1's key on it), where the order of the writes
    // shows; the free list empty or not
    let names = [
        "Atel_ResetLineTable",
        "Atel_FreeLineSignals",
        "Atel_SetLineTable",
    ];
    for (f, name) in names.into_iter().enumerate() {
        for kind in [0u8, 4, 5, 6, 7] {
            for layout in 0..4 {
                for listed in [false, true] {
                    let w = r22_worker(a, b, rng, kind);
                    let table = alloc_both(a, b, 0x1e0);
                    fill_both(a, b, rng, table, 0x1e0);
                    let t = [0, table, w + 0x48, w + 0x80][layout];
                    // Atel_SetLineTable with a table of its own installs it
                    let (pre, arg) = if f == 2 && layout == 1 {
                        (0, t)
                    } else {
                        (t, t)
                    };
                    poke(a, b, w + 0x2e4, pre);
                    let node = alloc_both(a, b, 0x18);
                    fill_both(a, b, rng, node, 0x18);
                    poke(a, b, w + 0x88, if listed { node } else { 0 });
                    let what = format!("{name} type {kind} layout {layout} listed {listed}");
                    let sym = [0x0086_d110, 0x0086_d1c0, 0x0086_d190][f];
                    let args = [w, arg];
                    let args = if f == 2 { &args[..] } else { &args[..1] };
                    r22_case(a, b, sym, args, true, &what, |r| match f {
                        0 => hp::atel_reset_line_table(r, w),
                        1 => hp::atel_free_line_signals(r, w),
                        _ => hp::atel_set_line_table(r, w, arg),
                    });
                    n += 1;
                }
            }
        }
    }
    // Angle_Wrap_00a7ae20 against Angle_Wrap's port
    // the original's 3.141592 (0x00b582f0, as f32)
    let pi = f32::from_bits(0x4049_0fd8);
    let mut xs = vec![
        0.0,
        -0.0,
        pi,
        -pi,
        f32::from_bits(pi.to_bits() + 1),
        f32::from_bits((-pi).to_bits() + 1),
        core::f32::consts::PI,
        -core::f32::consts::PI,
        6.283_184,
        -6.283_184,
        1000.0,
        -9999.5,
        f32::NAN,
        -f32::NAN,
    ];
    for _ in 0..40 {
        #[allow(clippy::cast_precision_loss)] // a random angle
        xs.push((rng.next() as f32 / u32::MAX as f32 - 0.5) * 80.0);
    }
    for x in xs {
        clear_stacks(a, b);
        let st0 = orig_float(a, 0x00a7_ae20, &[x.to_bits()]);
        let r = ffx_chara::chr::wrap_angle(x);
        assert!(
            same_float(st0, r.to_bits()),
            "Angle_Wrap_00a7ae20({x}): {st0} vs {r}"
        );
        no_diff(a, b, "Angle_Wrap_00a7ae20");
        n += 1;
    }
    n
}

/// `Atel_BtlMoveTargetDist` on worker `w` with `push` on its stack.
fn r22_move(a: &mut Oracle, b: &mut Oracle, w: u32, push: &[(u8, u32)], g: u32, what: &str) {
    let s = w + worker::STACK;
    let lvl = i32::from(Mem::read::<u8>(b, w + worker::LEVEL));
    let args = v::script_worker_level(b, w, lvl) + 0x2c;
    poke(a, b, s, 0i32);
    for &(k, x) in push {
        push_both(a, b, s, k, x);
    }
    r22_case(a, b, 0x007a_9150, &[w, args, s, g], true, what, |r| {
        ffx_script::vm::bridge::native_btl::atel_btl_move_target_dist(r, w, args, s, g)
    });
}

/// `MsGetMoveTargetPos` kinds (`ffx_battle::target`) the move tests use.
/// Kinds 2 and 10 (the last two) read other actors through the actor's
/// target fields, garbage in `MsGetChr`'s spare record (ids from 0x5d up,
/// the snapshots' 0xcd-filled event workers), where the original stops.
const KINDS: [u32; 9] = [0, 1, 3, 6, 13, 14, 0x46, 2, 10];

/// Eight move arguments as pushed (kind, speed, n, accel, brake, a, b, c),
/// `n` as given; a float slot sometimes holds an integer and the other way
/// round (the pops convert).
fn r22_move_args(rng: &mut Xorshift, n: u32, spare: bool) -> Vec<(u8, u32)> {
    #[allow(clippy::cast_precision_loss)] // a random value
    let f = |rng: &mut Xorshift, k: f32| (rng.next() as f32 / u32::MAX as f32 - 0.3) * k;
    let mut v = vec![
        // move kinds on the actor's own positions (1 battle position, 3
        // escape, 6 its position, 13/14 on the ground; 0x40 the speed from
        // +0x460); 2 and 10 read its targets and actor +0x416
        (
            stack::INT,
            KINDS[rng.below(if spare { 7 } else { 9 }) as usize],
        ),
        (stack::FLOAT, f(rng, 8.0).to_bits()),
        (stack::INT, n),
        (stack::INT, rng.below(0x90)),
        (stack::INT, rng.below(0x90)),
        (stack::FLOAT, f(rng, 10.0).to_bits()),
        (stack::FLOAT, f(rng, 10.0).to_bits()),
        (stack::FLOAT, f(rng, 10.0).to_bits()),
    ];
    if rng.below(4) == 0 {
        let i = rng.below(8) as usize;
        v[i].0 = if v[i].0 == stack::INT {
            stack::FLOAT
        } else {
            stack::INT
        };
        if v[i].0 == stack::INT {
            v[i].1 = rng.below(40);
        }
    }
    v
}

/// Round 22's helpers against the original: the image alone, then the
/// snapshot's state (`FFX_SNAPSHOT`), where `Atel_BtlMoveTargetDist` also
/// runs on every actor of every ctrl work in use (battle actors in the
/// battle snapshot).
#[test]
fn round22_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0022);
    let mut compared = 0;
    // the image: the helpers, and the move set-up without a battle actor
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        compared += r22_helpers(&mut a, &mut b, &mut rng);
        for round in 0..12_u32 {
            let w = r22_worker(&mut a, &mut b, &mut rng, 0);
            let n = if round % 2 == 0 { 3 } else { 0 };
            let push = r22_move_args(&mut rng, n, false);
            let take = if round % 4 == 3 { 5 } else { 8 };
            let what = format!("Atel_BtlMoveTargetDist image round {round}");
            r22_move(&mut a, &mut b, w, &push[..take], 0, &what);
            compared += 1;
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        assert!(compared > 0);
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    {
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        compared += r22_helpers(&mut a, &mut b, &mut rng);
    }
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut battle = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let ws = actors(&mut a, &mut b);
        for (i, &w) in ws.iter().enumerate().take(16) {
            let id = Mem::read::<u8>(&b, w + worker::BATTLE_ID);
            for round in 0..4_u32 {
                let n = [1, 12, 0, (-3_i32).cast_unsigned()][round as usize];
                let push = r22_move_args(&mut rng, n, id != 0xff && id >= 0x5d);
                let g = [0.0f32, -0.5, 1.25, 0.0][round as usize].to_bits();
                let what = format!(
                    "Atel_BtlMoveTargetDist ctrl {c:#x} worker {i} (actor {id:#x}) round {round}"
                );
                r22_move(&mut a, &mut b, w, &push, g, &what);
                compared += 1;
                if id != 0xff && n.cast_signed() > 0 {
                    battle += 1;
                }
            }
            // the line tables on the real worker
            let what = format!("ctrl {c:#x} worker {i}");
            r22_case(&mut a, &mut b, 0x0086_d1c0, &[w], true, &what, |r| {
                hp::atel_free_line_signals(r, w)
            });
            r22_case(&mut a, &mut b, 0x0086_d110, &[w], true, &what, |r| {
                hp::atel_reset_line_table(r, w)
            });
            compared += 2;
        }
    }
    eprintln!("{compared} round-22 cases compared ({battle} moves on a battle actor)");
}

/// A worker of type `kind` with room for every move and turn block
/// (`0x1000` bytes): each level block's move (`+0x44`) and turn (`+0x48`)
/// pointer is one of the worker's own blocks, every block's mode (`+2`)
/// random, idle or busy.
fn r23_worker(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, kind: u8) -> u32 {
    let hdr = alloc_both(a, b, 0x40);
    poke(a, b, hdr, kind);
    let w = alloc_both(a, b, 0x1000);
    poke(a, b, w, hdr);
    poke(a, b, w + worker::LEVEL, u8::try_from(rng.below(9)).unwrap());
    for i in 0..13 {
        let modes = [0, 0x7f, 0x80, 0x1f, 0x20, 0x4000, rng.next() & 0xffff];
        let m = modes[rng.below(7) as usize];
        poke(a, b, w + 0x5ba + i * 0x4c, u16::try_from(m).unwrap());
        let m = modes[rng.below(7) as usize];
        poke(a, b, w + 0x866 + i * 0x44, u16::try_from(m).unwrap());
    }
    for l in 0..9 {
        let blk = w + 0x12c + l * 0x4c;
        poke(a, b, blk + 0x44, w + 0x5b8 + rng.below(13) * 0x4c);
        poke(a, b, blk + 0x48, w + 0x864 + rng.below(13) * 0x44);
    }
    w
}

/// A mask and mode for the block helpers: the shared and `0x20`/`0x100`
/// bits in every combination, random bits besides.
fn r23_mask_mode(rng: &mut Xorshift, round: u32) -> (u32, u32) {
    let mask = match round % 5 {
        0 => 0,
        1 => u32::MAX,
        2 => 1 << rng.below(32),
        _ => rng.next(),
    };
    let flags = [0, 0x20, 0x100, 0x120][(round % 4) as usize];
    let mode = rng.next()
        & !0x120
        & if round.is_multiple_of(3) {
            0xffff
        } else {
            u32::MAX
        }
        | flags;
    (mask, mode)
}

/// The four block helpers on worker `w`: share at `level`, start at the
/// worker's own level.
fn r23_blocks(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: u32,
    level: u32,
    round: u32,
    what: &str,
) {
    use ffx_script::vm::bridge::helpers as hp;
    let (mask, mode) = r23_mask_mode(rng, round);
    let param = rng.next();
    let what = format!("{what} level {level:#x} mask {mask:#x} mode {mode:#x}");
    r22_case(
        a,
        b,
        0x0087_0440,
        &[w, level, mask, mode],
        true,
        &format!("Atel_ShareMoveBlock {what}"),
        |r| hp::atel_share_move_block(r, w, level, mask, mode),
    );
    r22_case(
        a,
        b,
        0x0087_1090,
        &[w, level, mask, mode],
        true,
        &format!("Atel_ShareTurnBlock {what}"),
        |r| hp::atel_share_turn_block(r, w, level, mask, mode),
    );
    r22_case(
        a,
        b,
        0x0087_03e0,
        &[w, mask, mode, param],
        true,
        &format!("Atel_StartMoveBlock {what}"),
        |r| hp::atel_start_move_block(r, w, mask, mode, param),
    );
    r22_case(
        a,
        b,
        0x0087_1030,
        &[w, mask, mode, param],
        true,
        &format!("Atel_StartTurnBlock {what}"),
        |r| hp::atel_start_turn_block(r, w, mask, mode, param),
    );
}

/// `Atel_AddSepLoad` on random lists: counts from -1 to 9, ids already
/// listed or not, every group bit against random selected groups.
fn r23_sep_loads(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) -> u32 {
    use ffx_script::vm::bridge::helpers as hp;
    let count = a.rebase(0x0132_5b78);
    let list = a.rebase(0x0132_7030);
    let groups = a.rebase(0x0132_6bdc);
    for round in 0..rounds {
        let n = [0, 1, 7, 8, 9, -1, 3][(round % 7) as usize];
        if round % 3 != 2 {
            poke(a, b, count, n);
        }
        for i in 0..8 {
            poke(a, b, list + i * 8, rng.below(6));
            poke(a, b, list + i * 8 + 4, rng.next());
        }
        poke(a, b, groups, u8::try_from(rng.below(256)).unwrap());
        let id = if round % 2 == 0 {
            rng.below(8)
        } else {
            rng.next()
        };
        let group = match round % 4 {
            0 => 0,
            1 => 1 << (16 + rng.below(7)),
            2 => rng.next() & 0x7f_0000,
            _ => rng.next(),
        };
        let flags = group | rng.next() & 0xffff;
        r22_case(
            a,
            b,
            0x0086_e250,
            &[id, flags],
            true,
            &format!("Atel_AddSepLoad({id:#x}, {flags:#x}) round {round}"),
            |r| hp::atel_add_sep_load(r, id, flags),
        );
    }
    rounds
}

/// Round 23's helpers against the original: the move/turn block setters
/// on synthetic workers of every type and level (image), then on every
/// actor of every ctrl work in use (`FFX_SNAPSHOT`); `Atel_AddSepLoad` on
/// both.
#[test]
fn round23_helpers_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0023);
    let mut compared = 0;
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        compared += r23_sep_loads(&mut a, &mut b, &mut rng, 200);
        for round in 0..240_u32 {
            let kind = [0, 1, 4, 5, 6, 7][(round % 6) as usize];
            let w = r23_worker(&mut a, &mut b, &mut rng, kind);
            let level = match round % 8 {
                0 => u32::MAX,
                1 => 12,
                2 => 9,
                _ => rng.below(9),
            };
            r23_blocks(
                &mut a,
                &mut b,
                &mut rng,
                w,
                level,
                round,
                &format!("image type {kind} round {round}"),
            );
            compared += 4;
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    {
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        compared += r23_sep_loads(&mut a, &mut b, &mut rng, 60);
    }
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut real = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let ws = actors(&mut a, &mut b);
        for (i, &w) in ws.iter().enumerate().take(24) {
            // only workers with move and turn blocks: every level's
            // pointers into its own blocks (the 0x100 mode reads them)
            let own = (0..9).all(|l| {
                let at = w + 0x12c + l * 0x4c;
                let mv = Mem::read::<u32>(&b, at + 0x44).wrapping_sub(w + 0x5b8);
                let tn = Mem::read::<u32>(&b, at + 0x48).wrapping_sub(w + 0x864);
                mv < 9 * 0x4c && tn < 9 * 0x44
            });
            if !own {
                continue;
            }
            real += 1;
            for round in 0..6_u32 {
                let level = if round == 0 {
                    u32::from(Mem::read::<u8>(&b, w + worker::LEVEL))
                } else {
                    rng.below(9)
                };
                r23_blocks(
                    &mut a,
                    &mut b,
                    &mut rng,
                    w,
                    level,
                    round,
                    &format!("ctrl {c:#x} worker {i} round {round}"),
                );
                compared += 4;
            }
        }
    }
    eprintln!("{compared} round-23 cases compared ({real} real workers)");
    assert!(real > 0, "no worker with move blocks");
}

/// `AtelSetBindObject` and `AtelSetPos3d` on worker `w` as it is, with
/// random floats in its position block and random arguments.
fn r23_pos(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32, round: u32, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let kind = Mem::read::<u8>(b, Mem::read::<u32>(b, w));
    if kind != 4 {
        let p = w + if kind.wrapping_sub(5) < 2 {
            0x284
        } else {
            0x558
        };
        for k in 0..0x10 {
            if rng.below(3) == 0 {
                poke(a, b, p + 4 * k, stored_value(rng));
            }
        }
        let frame = run::NativeHost::scratch(&mut Run(b));
        // yiSetRotTransGroup's EAX is a pointer into its own frame (the
        // matrix work): equal unless both are stack addresses
        let what = format!("AtelSetBindObject {what}");
        clear_stacks(a, b);
        let ra = orig(a, 0x0086_f4c0, &[w]);
        let rb = hp::atel_set_bind_object(&mut Run(b), w, frame);
        let stack = a.stack_range();
        assert!(
            ra == rb || stack.contains(&ra) && stack.contains(&rb),
            "{what}: EAX {ra:#x} vs {rb:#x}"
        );
        native_no_diff(a, b, &what);
    }
    let pos = [stored_value(rng), stored_value(rng), stored_value(rng)];
    let snap = [0, 1, 0, u32::MAX][(round % 4) as usize];
    let frame = run::NativeHost::scratch(&mut Run(b));
    r22_case(
        a,
        b,
        0x0087_0b20,
        &[w, pos[0], pos[1], pos[2], snap],
        true,
        &format!("AtelSetPos3d {what} pos {pos:x?} snap {snap}"),
        |r| hp::atel_set_pos_3d(r, w, pos, snap, frame),
    );
}

/// A ctrl work for the image: `g_ScriptContext` pointed at a zeroed one
/// whose player worker id (`+0xa`) is `player`.
fn r23_ctx(a: &mut Oracle, b: &mut Oracle, player: u16) {
    let c = alloc_both(a, b, 0x300);
    poke(a, b, c + 0xa, player);
    let at = a.rebase(0x0132_6b28);
    poke(a, b, at, c);
}

/// Round 23 b: `AtelSetBindObject` and `AtelSetPos3d` on synthetic
/// workers of every binding (the image), then on every actor of every ctrl
/// work in use, with `+0x34` bit 4 both ways (`FFX_SNAPSHOT`).
#[test]
#[allow(clippy::too_many_lines)] // the image part, then the snapshot part
fn round23b_helpers_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_023b);
    let mut compared = 0;
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r23_ctx(&mut a, &mut b, 3);
        for round in 0..96_u32 {
            let kind = [0, 1, 5, 6, 4, 7][(round % 6) as usize];
            let w = r23_worker(&mut a, &mut b, &mut rng, kind);
            // +0xaa: none, the character (none set), the camera, the map
            // objects (none loaded on the image)
            let bind = [0u8, 1, 2, 3, 4, 5][(round / 6 % 6) as usize];
            poke(&mut a, &mut b, w + 0xaa, bind);
            poke(
                &mut a,
                &mut b,
                w + 0xa8,
                u16::try_from(rng.below(4)).unwrap(),
            );
            poke(&mut a, &mut b, w + 0x9c, 0u32);
            poke(&mut a, &mut b, w + 0x3c, rng.below(2));
            // the player, one only its low byte matches, another
            let player = [3u16, 0x103, 7][(round % 3) as usize];
            poke(&mut a, &mut b, w + 0x2e, player);
            poke(
                &mut a,
                &mut b,
                w + 0x34,
                u8::try_from(rng.below(256)).unwrap(),
            );
            let ctx = Mem::read::<u32>(&b, b.rebase(0x0132_6b28));
            for k in 0..8 {
                poke(&mut a, &mut b, ctx + 0x218 + 4 * k, stored_value(&mut rng));
            }
            r23_pos(
                &mut a,
                &mut b,
                &mut rng,
                w,
                round,
                &format!("image type {kind} bind {bind} round {round}"),
            );
            compared += 2;
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut binds = [0u32; 8];
    let mut players = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let ws = actors(&mut a, &mut b);
        for (i, &w) in ws.iter().enumerate().take(24) {
            let bind = Mem::read::<u8>(&b, w + 0xaa);
            binds[usize::from(bind.min(7))] += 1;
            let ctx = Mem::read::<u32>(&b, b.rebase(0x0132_6b28));
            let chr = Mem::read::<u32>(&b, w + 0x9c);
            let player = Mem::read::<u16>(&b, w + 0x2e) == Mem::read::<u16>(&b, ctx + 0xa);
            for round in 0..4_u32 {
                if player && chr != 0 {
                    // the character's encounter (bits 7-8) and dictionary
                    // (bits 11-12) ids, different from each other
                    players += 1;
                    let pid = Mem::read::<u32>(&b, chr + 0x828) & !0x1980;
                    let pid = pid | (round & 3) << 7 | ((round + 1) & 3) << 11;
                    poke(&mut a, &mut b, chr + 0x828, pid);
                }
                if round == 3 && bind.wrapping_sub(4) < 2 {
                    // an object id only its low byte matches
                    let id = Mem::read::<u16>(&b, w + 0xa8);
                    poke(&mut a, &mut b, w + 0xa8, id | 0x100);
                }
                let f = Mem::read::<u8>(&b, w + 0x34);
                poke(
                    &mut a,
                    &mut b,
                    w + 0x34,
                    if round < 2 { f | 0x10 } else { f & !0x10 },
                );
                r23_pos(
                    &mut a,
                    &mut b,
                    &mut rng,
                    w,
                    round,
                    &format!("ctrl {c:#x} worker {i} bind {bind} round {round}"),
                );
                compared += 2;
                if player && chr != 0 && round == 2 {
                    // the player without a character: dictionary index 0
                    poke(&mut a, &mut b, w + 0x9c, 0u32);
                    let what = format!("ctrl {c:#x} worker {i} without its character");
                    r23_pos(&mut a, &mut b, &mut rng, w, round, &what);
                    poke(&mut a, &mut b, w + 0x9c, chr);
                    compared += 2;
                }
            }
        }
    }
    eprintln!("{compared} round-23b cases compared (bindings {binds:?}, {players} on the player)");
    assert!(players > 0, "no player worker with a character");
}

/// `Atel_QueueSepLoads` on random load lists against random separates
/// tables (ids shared or not, pending or not, every fade flag); headers
/// without a table too.
fn r23_sep_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rounds: u32) {
    use ffx_script::vm::bridge::helpers as hp;
    let loads = a.rebase(0x0132_5b78);
    let list = a.rebase(0x0132_7030);
    for round in 0..rounds {
        let hdr = alloc_both(a, b, 0x200);
        let n = [0, 1, 3, 8, -1][(round % 5) as usize];
        match round % 7 {
            // no table offset (+0xc of the header itself then holds one)
            0 => {
                poke(a, b, hdr + 0xc, 0x80u32);
                poke(a, b, hdr + 0x80, 8u32);
                for j in 0..8 {
                    poke(a, b, hdr + 0x84 + j * 8, j);
                }
            }
            1 => {
                // a header without the table's +0xc (and a count at +0)
                poke(a, b, hdr + 0x2c, 0x40u32);
                poke(a, b, hdr, 2u32);
            }
            _ => {
                poke(a, b, hdr + 0x2c, 0x40u32);
                poke(a, b, hdr + 0x4c, 0x80u32);
                poke(a, b, hdr + 0x80, n);
                for j in 0..8 {
                    poke(a, b, hdr + 0x84 + j * 8, rng.below(10));
                    poke(a, b, hdr + 0x88 + j * 8, rng.next());
                }
            }
        }
        // 9 reaches past the list's 8 entries (no caller lets it)
        poke(a, b, loads, [0, 1, 4, 8, -2, 9][(round % 6) as usize]);
        for i in 0..9 {
            let flags =
                rng.next() & !0xe000 | [0, 0x8000, 0xa000, 0xc000, 0xe000][rng.below(5) as usize];
            poke(a, b, list + i * 8, rng.below(10));
            poke(a, b, list + i * 8 + 4, flags);
        }
        r22_case(
            a,
            b,
            0x0086_38c0,
            &[hdr],
            true,
            &format!("Atel_QueueSepLoads round {round}"),
            |r| hp::atel_queue_sep_loads(r, hdr),
        );
    }
}

/// `Atel_UcRestore` and `Atel_UcResume` with the ctrl work's flag bytes
/// (`+0`, `+1`, `+3`) in every combination that matters.
fn r23_uc(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let ctx = Mem::read::<u32>(b, b.rebase(0x0132_6b28));
    // every combination of the bits that matter, random bits besides
    for round in 0..48_u32 {
        let r = |rng: &mut Xorshift| u8::try_from(rng.below(256)).unwrap();
        let f0 = r(rng) & !0x22 | [0, 2, 0x20, 0x22][(round % 4) as usize];
        let f1 = r(rng) & !4 | [0, 4][(round / 4 % 2) as usize];
        let f3 = r(rng) & !0x84 | [0, 4, 0x80][(round / 8 % 3) as usize];
        poke(a, b, ctx, f0);
        poke(a, b, ctx + 1, f1);
        poke(a, b, ctx + 3, f3);
        let what = format!("{what} flags {f0:#x} {f1:#x} {f3:#x}");
        let (f, name) = if round / 24 == 0 {
            (0x0086_a130, "Atel_UcResume")
        } else {
            (0x0086_a0e0, "Atel_UcRestore")
        };
        r22_case(a, b, f, &[], false, &format!("{name} {what}"), |r| {
            if f == 0x0086_a130 {
                hp::atel_uc_resume(r);
            } else {
                hp::atel_uc_restore(r);
            }
            0
        });
    }
}

/// Round 23 c: the ctrl work's control helpers, the separates queue, the
/// rotation getter, the character direction sync and the bind effect
/// switch: synthetic state on the image, then the snapshot's workers and
/// characters (`FFX_SNAPSHOT`).
#[test]
#[allow(clippy::too_many_lines)] // the image part, then the snapshot part
fn round23c_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_023c);
    let dir_of = |r: &mut Run, c: u32| {
        let f = r.0.rebase(0x0082_ab80);
        r.0.call_f64(f, &[c]).unwrap()
    };
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r23_ctx(&mut a, &mut b, 3);
        r23_uc(&mut a, &mut b, &mut rng, "image");
        r23_sep_queue(&mut a, &mut b, &mut rng, 140);
        for round in 0..24_u32 {
            let kind = [0, 1, 5, 6][(round % 4) as usize];
            let w = r23_worker(&mut a, &mut b, &mut rng, kind);
            let chr = alloc_both(&mut a, &mut b, 0x1000);
            fill_both(&mut a, &mut b, &mut rng, chr, 0x1000);
            poke(
                &mut a,
                &mut b,
                w + 0x9c,
                if round % 3 == 2 { 0 } else { chr },
            );
            poke(
                &mut a,
                &mut b,
                w + 0xaa,
                [1u8, 1, 0, 2][(round / 4 % 4) as usize],
            );
            let what = format!("Atel_SyncChrDir image type {kind} round {round}");
            r22_case(&mut a, &mut b, 0x0086_9df0, &[w], false, &what, |r| {
                hp::atel_sync_chr_dir(r, w, dir_of);
                0
            });
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    {
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        r23_sep_queue(&mut a, &mut b, &mut rng, 40);
    }
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut rot, mut chars) = (0, 0);
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        r23_uc(&mut a, &mut b, &mut rng, &format!("ctrl {c:#x}"));
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let ws = actors(&mut a, &mut b);
        for (i, &w) in ws.iter().enumerate().take(24) {
            let kind = Mem::read::<u8>(&b, Mem::read::<u32>(&b, w));
            if kind == 4 {
                continue;
            }
            let k = u32::try_from(i).unwrap();
            let st0 = orig_float(&mut a, 0x0086_a6e0, &[k]);
            let rb = hp::atel_get_worker_rot_y(&mut Run(&mut b), k);
            assert!(
                same_float(st0, rb.to_bits()),
                "Atel_GetWorkerRotY ctrl {c:#x} worker {i}"
            );
            rot += 1;
            let what = format!("Atel_SyncChrDir ctrl {c:#x} worker {i}");
            r22_case(&mut a, &mut b, 0x0086_9df0, &[w], false, &what, |r| {
                hp::atel_sync_chr_dir(r, w, dir_of);
                0
            });
            if Mem::read::<u8>(&b, w + 0xaa) == 1 && Mem::read::<u32>(&b, w + 0x9c) != 0 {
                chars += 1;
                for kind in 0..4 {
                    let what = format!("AtelBindEff2On ctrl {c:#x} worker {i} kind {kind}");
                    r22_case(&mut a, &mut b, 0x0086_2700, &[w, kind], true, &what, |r| {
                        hp::atel_bind_eff2_on(r, w, kind)
                    });
                }
            }
        }
    }
    eprintln!("round 23c: {rot} rotations, {chars} characters");
    assert!(rot > 0 && chars > 0);
}

/// Random values in the fields the round-23 d accessors read and write,
/// on worker `w` of both oracles.
fn r23d_fields(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32) {
    #[allow(clippy::cast_possible_truncation)] // random bytes and halves
    {
        poke(a, b, w + 0x32, rng.next() as u8);
        poke(a, b, w + 0x33, rng.next() as u8);
        poke(a, b, w + 0x38, rng.next() as u16);
        poke(a, b, w + 0xa8, rng.next() as u16);
        poke(a, b, w + 0xaa, [0u8, 1, 1, 2, 4][rng.below(5) as usize]);
        poke(a, b, w + 0xb14, rng.next() as u16);
    }
    poke(a, b, w + 0xa0, rng.next());
    for k in 0..4 {
        poke(a, b, w + 0xaf4 + k * 4, rng.next());
        poke(a, b, w + 0xb04 + k * 2, rng.next() & 0xffff);
    }
}

/// Every round-23 d accessor on the event's worker `i` (ctrl work 0) and
/// the current ctrl work's worker `i`, the work-size pair on the current
/// one's worker `i` (its header's size and work offset prepared when
/// `prep`).
#[allow(clippy::too_many_lines)] // one case per accessor
fn r23d_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, i: u32, prep: bool, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    type Get = fn(&mut Run, u32) -> u32;
    type Get2 = fn(&mut Run, u32, u32) -> u32;
    type Set = fn(&mut Run, u32, u32, u32) -> u32;
    let gets: [(u32, &str, Get); 8] = [
        (
            0x0086_b1b0,
            "AtelGetEventActorLinkWork",
            |r: &mut Run, i| hp::atel_get_event_actor_link_work(r, i),
        ),
        (0x0086_b370, "AtelGetEventBindIdFromID", |r: &mut Run, i| {
            hp::atel_get_event_bind_id_from_id(r, i)
        }),
        (0x0086_b3d0, "AtelGetEventCharIDFromID", |r: &mut Run, i| {
            hp::atel_get_event_char_id_from_id(r, i)
        }),
        (0x0086_b320, "AtelGetEventActorType", |r: &mut Run, i| {
            hp::atel_get_event_actor_type(r, i)
        }),
        (0x0086_ba40, "AtelGetHideStatusFromID", |r: &mut Run, i| {
            hp::atel_get_hide_status_from_id(r, i)
        }),
        (
            0x0086_bbf0,
            "AtelGetKeepBattlePartyFromID",
            |r: &mut Run, i| hp::atel_get_keep_battle_party_from_id(r, i),
        ),
        (0x0086_c3f0, "AtelGetRequestLevel", |r: &mut Run, i| {
            hp::atel_get_request_level(r, i)
        }),
        (0x0086_c160, "AtelGetMyWorkSize", |r: &mut Run, i| {
            hp::atel_get_my_work_size(r, i)
        }),
    ];
    for (f, name, port) in gets {
        r22_case(a, b, f, &[i], true, &format!("{name} {what}"), |r| {
            port(r, i)
        });
    }
    let gets2: [(u32, &str, Get2); 4] = [
        (
            0x0086_b400,
            "AtelGetEventElemCharIDFromID",
            |r: &mut Run, i, k| hp::atel_get_event_elem_char_id_from_id(r, i, k),
        ),
        (
            0x0086_b440,
            "AtelGetEventElemHandleFromID",
            |r: &mut Run, i, k| hp::atel_get_event_elem_handle_from_id(r, i, k),
        ),
        (
            0x0086_b480,
            "AtelGetEventElemPosFromID",
            |r: &mut Run, i, k| hp::atel_get_event_elem_pos_from_id(r, i, k),
        ),
        (
            0x0086_b4c0,
            "AtelGetEventElemStatus",
            |r: &mut Run, i, k| hp::atel_get_event_elem_status(r, i, k),
        ),
    ];
    for k in [0, 1, rng.below(4), rng.below(40)] {
        for (f, name, port) in gets2 {
            if f != 0x0086_b4c0 && k > 3 {
                continue;
            }
            let what = format!("{name} {what} element {k}");
            r22_case(a, b, f, &[i, k], true, &what, |r| port(r, i, k));
        }
    }
    let sets: [(u32, &str, Set); 4] = [
        (
            0x0086_fde0,
            "AtelSetEventElemHandleFromID",
            |r: &mut Run, i, k, v| hp::atel_set_event_elem_handle_from_id(r, i, k, v),
        ),
        (
            0x0086_fe20,
            "AtelSetEventElemPosFromID",
            |r: &mut Run, i, k, v| hp::atel_set_event_elem_pos_from_id(r, i, k, v),
        ),
        (
            0x0086_fda0,
            "AtelSetEventElemCharIDFromID",
            |r: &mut Run, i, k, v| hp::atel_set_event_elem_char_id_from_id(r, i, k, v),
        ),
        (
            0x0086_fe60,
            "AtelSetEventElemStatus",
            |r: &mut Run, i, k, v| hp::atel_set_event_elem_status(r, i, k, v),
        ),
    ];
    for k in [0, 1, rng.below(4), rng.below(40), 15, 16] {
        for (f, name, port) in sets {
            if f != 0x0086_fe60 && k > 3 {
                continue;
            }
            for v in [0, 1, rng.next()] {
                let what = format!("{name} {what} element {k} value {v:#x}");
                r22_case(a, b, f, &[i, k, v], true, &what, |r| port(r, i, k, v));
            }
        }
    }
    for v in [0, rng.next()] {
        let what = format!("AtelSetEventActorLinkWork {what} {v:#x}");
        r22_case(a, b, 0x0086_fc40, &[i, v], true, &what, |r| {
            hp::atel_set_event_actor_link_work(r, i, v)
        });
        let what = format!("AtelSetEventCharHandleFromID {what} {v:#x}");
        r22_case(a, b, 0x0086_fd60, &[i, v], true, &what, |r| {
            hp::atel_set_event_char_handle_from_id(r, i, v)
        });
    }
    // the work size and the work clear: the header's size small, zero or
    // negative and its work offset set or not, with an own work or none
    let w = v::atel_get_basic_work(b, i.cast_signed());
    let hdr = Mem::read::<u32>(b, w);
    let bin = Mem::read::<u32>(b, w + 4);
    for round in 0..if prep { 14_u32 } else { 4 } {
        if prep {
            // sizes small, zero, negative (-0x21 rounds towards zero), with
            // and without the work offset; the last two rounds put the
            // binary at -0x10 so binary + offset is 0 (no clear)
            let size = [0x13_i32, 0, -5, 0x40, -0x21, 0x21][(round % 6) as usize];
            poke(a, b, hdr + 0x10, size);
            poke(a, b, hdr + 0x2c, if round < 6 { 0u32 } else { 0x10 });
            poke(
                a,
                b,
                w + 4,
                if round >= 12 {
                    0u32.wrapping_sub(0x10)
                } else {
                    bin
                },
            );
        }
        let what = format!("AtelGetMyWorkSize {what} round {round}");
        r22_case(a, b, 0x0086_c160, &[i], true, &what, |r| {
            hp::atel_get_my_work_size(r, i)
        });
        let own = if round % 3 == 0 || round >= 12 {
            0
        } else {
            let at = alloc_both(a, b, 0x80);
            fill_both(a, b, rng, at, 0x80);
            at
        };
        let what = format!("AtelSetMyWorkAdrs {what} round {round} own {own:#x}");
        r22_case(a, b, 0x0087_0870, &[i, own], true, &what, |r| {
            hp::atel_set_my_work_adrs(r, i, own)
        });
    }
    if prep {
        poke(a, b, w + 4, bin);
    }
}

/// A synthetic ctrl work at `c` with `n` workers of every type (headers
/// with small work sizes, binaries) and random fields; returns its
/// workers.
fn r23d_ctrl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, c: u32, n: u16) -> Vec<u32> {
    let work = alloc_both(a, b, usize::from(n) * 0xb58);
    fill_both(a, b, rng, work, u32::from(n) * 0xb58);
    poke(a, b, c + ctrl::ACTORS, n);
    poke(a, b, c + ctrl::WORKERS, n);
    for k in [ctrl::CLASS_5B8, ctrl::CLASS_558, ctrl::CLASS_2E8] {
        poke(a, b, c + k, 0u16);
    }
    poke(a, b, c + ctrl::ID, u16::try_from(rng.below(7)).unwrap());
    poke(a, b, c + ctrl::WORK, work);
    (0..u32::from(n))
        .map(|i| {
            let w = work + i * 0xb58;
            let hdr = alloc_both(a, b, 0x40);
            fill_both(a, b, rng, hdr, 0x40);
            poke(a, b, hdr, u8::try_from(i % 8).unwrap());
            poke(a, b, hdr + 0x10, rng.below(0x40));
            poke(a, b, hdr + 0x2c, [0, 0x10][(i % 2) as usize]);
            let bin = alloc_both(a, b, 0x80);
            fill_both(a, b, rng, bin, 0x80);
            poke(a, b, w, hdr);
            poke(a, b, w + 4, bin);
            r23d_fields(a, b, rng, w);
            w
        })
        .collect()
}

/// Round 23 d (this machine): the event workers' link, binding, character
/// and element accessors (`MsBtlBridge*`'s), the hide/party/request-level
/// getters, the work size pair and `AtelGetElemId`: synthetic ctrl works 0
/// and 1 on the image (indices past the end too), then every ctrl work in
/// use on the snapshot (`FFX_SNAPSHOT`) against the event's own workers.
#[test]
fn round23d_event_accessors_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_023d);
    let cache = 0x0132_70e4;
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let works = a.rebase(0x0132_5ba0);
        let context = a.rebase(0x0132_6b28);
        r23d_ctrl(&mut a, &mut b, &mut rng, works, 5);
        r23d_ctrl(&mut a, &mut b, &mut rng, works + ctrl::SIZE, 3);
        // ctrl work 0 with fewer actors than workers: the type getter
        // checks the actor count
        poke(&mut a, &mut b, works + ctrl::ACTORS, 3u16);
        for i in 0..6 {
            r22_case(
                &mut a,
                &mut b,
                0x0086_b320,
                &[i],
                true,
                "AtelGetEventActorType 3 of 5",
                |r| hp::atel_get_event_actor_type(r, i),
            );
        }
        poke(&mut a, &mut b, works + ctrl::ACTORS, 5u16);
        for current in [works, works + ctrl::SIZE] {
            poke(&mut a, &mut b, context, current);
            for i in [0, 1, 2, 4, 5, 9, u32::MAX] {
                let c = a.rebase(cache);
                poke(&mut a, &mut b, c, 0u32);
                let what = format!("image ctrl {current:#x} worker {i}");
                r23d_cases(&mut a, &mut b, &mut rng, i, true, &what);
            }
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut compared = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let works = a.rebase(0x0132_5ba0);
        let n = u32::from(Mem::read::<u16>(&b, works + ctrl::WORKERS)).min(16);
        let mine = u32::from(Mem::read::<u16>(&b, c + ctrl::WORKERS));
        for i in (0..n.min(mine)).chain([n, u32::MAX]) {
            let what = format!("ctrl {c:#x} worker {i}");
            r23d_cases(&mut a, &mut b, &mut rng, i, false, &what);
            let ew = {
                let ctx = a.rebase(0x0132_6b28);
                poke(&mut a, &mut b, ctx, works);
                let w = v::atel_get_basic_work(&mut b, i.cast_signed());
                orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
                poke(&mut a, &mut b, ctx, c);
                w
            };
            r23d_fields(&mut a, &mut b, &mut rng, ew);
            r23d_cases(
                &mut a,
                &mut b,
                &mut rng,
                i,
                false,
                &format!("{what} random"),
            );
            compared += 1;
        }
        for chr in [0, 1, 7, 17] {
            for kind in [0, 1, 2] {
                let what = format!("AtelGetElemId ctrl {c:#x} chr {chr} kind {kind}");
                r22_case(
                    &mut a,
                    &mut b,
                    0x0086_b150,
                    &[chr, kind],
                    true,
                    &what,
                    |r| hp::atel_get_elem_id(r, chr, kind),
                );
            }
        }
    }
    eprintln!("round 23d: {compared} snapshot workers");
    assert!(compared > 0);
}

/// The save-record setters of round 23 e on one pair (image or snapshot):
/// random record bytes first, so every set and clear shows.
#[allow(clippy::too_many_lines)] // one block per setter
fn r23e_setters(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    type Set = fn(&mut Run, u32) -> u32;
    let rec = orig(a, 0x0088_c830, &[]);
    assert_eq!(orig(b, 0x0088_c830, &[]), rec);
    let flags = orig(a, 0x0078_5250, &[]);
    let menu = orig(a, 0x0078_5290, &[]);
    orig(b, 0x0078_5250, &[]);
    orig(b, 0x0078_5290, &[]);
    for round in 0..8_u32 {
        fill_both(a, b, rng, rec, 0x100);
        let v = [0, 1, 2, 0x8000_0000, 0x1_0000, rng.next(), 0xffff, u32::MAX][round as usize];
        let sets: [(u32, &str, Set); 9] = [
            (0x0087_15f0, "AtelSetWaterBattleEffect", |r: &mut Run, v| {
                hp::atel_set_water_battle_effect(r, v)
            }),
            (0x0086_f060, "Atel_SetSaveEventD1", |r: &mut Run, v| {
                hp::atel_set_save_event_d1(r, v)
            }),
            (0x0086_f430, "Atel_SetSaveEvent13Bit1", |r: &mut Run, v| {
                hp::atel_set_save_event_13_bit1(r, v)
            }),
            (0x0087_0140, "AtelSetForcePlaceId", |r: &mut Run, v| {
                hp::atel_set_force_place_id(r, v)
            }),
            (
                0x0087_0ed0,
                "Atel_SetSaveDicIndexValue",
                |r: &mut Run, v| hp::atel_set_save_event_word(r, 0xe, v),
            ),
            (0x0087_3dd0, "Atel_SetSaveEventE8", |r: &mut Run, v| {
                hp::atel_set_save_event_word(r, 0xe8, v)
            }),
            (0x0087_0fa0, "Atel_SetSaveEventEA", |r: &mut Run, v| {
                hp::atel_set_save_event_word(r, 0xea, v)
            }),
            (0x0087_4030, "Atel_SetSaveEventE4", |r: &mut Run, v| {
                hp::atel_set_save_event_e4(r, v)
            }),
            (0x0087_4000, "Atel_JumpToEntrance", |r: &mut Run, v| {
                hp::atel_jump_to_entrance(r, v)
            }),
        ];
        for (f, name, port) in sets {
            let what = format!("{name} {what} {v:#x}");
            r22_case(a, b, f, &[v], true, &what, |r| port(r, v));
        }
        // the help flags: negative, in range, past the 0x80 bytes
        fill_both(a, b, rng, flags, 0x80);
        for i in [
            u32::MAX - 4,
            0,
            7,
            8,
            0x3ff,
            0x400,
            0x1388,
            rng.below(0x400),
        ] {
            for on in [0, 1, 7] {
                let what = format!("AtelSetHelpEventFlag {what} {i:#x} {on}");
                r22_case(a, b, 0x0087_01f0, &[i, on], true, &what, |r| {
                    hp::atel_set_help_event_flag(r, i, on)
                });
            }
        }
        // the Dolby, mono and vibration bits both ways
        let word = rng.next() & !0x91 | [0, 1, 0x10, 0x11, 0x80, 0x81, 0x90, 0x91][round as usize];
        poke(a, b, menu, word);
        for on in [0, 1, 0x100] {
            let what = format!("AtelSetDolby {what} menu {word:#x} {on}");
            r22_case(a, b, 0x0086_f9e0, &[on], true, &what, |r| {
                hp::atel_set_dolby(r, on)
            });
        }
    }
    fill_both(a, b, rng, rec + 0x1e0, 0x1020);
    poke(a, b, a.rebase(0x0132_5b78), rng.next());
    poke(a, b, a.rebase(0x0132_6bdc), rng.next());
    r22_case(
        a,
        b,
        0x0086_d4e0,
        &[],
        true,
        &format!("AtelInitSaveWork {what}"),
        |r| hp::atel_init_save_work(r),
    );
}

/// Round 23 e (this machine): the save event record setters behind the
/// natives, the help flags, `AtelSetDolby`, `AtelInitSaveWork`, the entrance
/// jump (image and snapshots), and the two map loads (`AtelMapLoadNoCache`,
/// `AtelMapLoad`'s code, and `AtelMapLoadBtl`) on every ctrl work in use
/// (`FFX_SNAPSHOT`).
#[test]
fn round23e_save_setters_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_023e);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r23e_setters(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut loads, mut stops) = (0, 0);
    for c in ctrls {
        let mut p = fresh_run_pair(&pages, &exe, c);
        r23e_setters(&mut p.0, &mut p.1, &mut rng, &format!("ctrl {c:#x}"));
        let mut p = fresh_run_pair(&pages, &exe, c);
        for map in [0u32, u32::MAX, 1] {
            for (f, name) in [
                (0x0086_dd30, "AtelMapLoadNoCache"),
                (0x0086_dcd0, "AtelMapLoadBtl"),
            ] {
                let what = format!("{name} {map} ctrl {c:#x}");
                let ok = r19_case(&pages, &exe, c, &mut p, f, &[map], true, &what, |r| {
                    if f == 0x0086_dd30 {
                        hp::atel_map_load(r, map)
                    } else {
                        hp::atel_map_load_btl(r, map)
                    }
                });
                if ok {
                    loads += 1;
                } else {
                    stops += 1;
                }
            }
        }
    }
    eprintln!("round 23e: {loads} map loads compared, {stops} stopped");
    assert!(loads > 0);
}

/// Worker `w`'s two position blocks (`+0x284`, `+0x558`, 0x40 bytes each)
/// as floats without signalling NaNs (quiet NaNs, infinities and
/// denormals among them).
fn r23f_positions(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32) {
    for base in [0x284, 0x558] {
        for k in 0..16 {
            let v = if rng.below(4) == 0 {
                stored_value(rng)
            } else {
                (f32::from(u16::try_from(rng.below(2000)).unwrap()) - 1000.0).to_bits()
            };
            poke(a, b, w + base + k * 4, v);
        }
    }
}

/// A signal queue of `n` entries on worker `w` (`+0x80`): kinds 0..=3
/// (`+0xf`), random level bytes (`+0xe`).
fn r23f_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32, n: u32) {
    let mut next = 0u32;
    for _ in 0..n {
        let e = alloc_both(a, b, 0x20);
        fill_both(a, b, rng, e, 0x20);
        poke(a, b, e, next);
        poke(a, b, e + 0xf, u8::try_from(rng.below(4)).unwrap());
        next = e;
    }
    poke(a, b, w + 0x80, next);
}

/// The round-23 f helpers on the current ctrl work's worker `i` (worker
/// `w`): the three position getters, `Event_SetPlayer`, `AtelIsBind`, the
/// signal level test, the `+0x34` bits and the level vectors.
fn r23f_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, i: u32, w: u32, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    type Pos = fn(&mut Run, u32, u32, u32, u32) -> u32;
    let out = alloc_both(a, b, 0x20);
    let gets: [(u32, &str, Pos); 3] = [
        (
            0x0086_a720,
            "AtelGetActorStartPos",
            |r: &mut Run, i, x, y, z| hp::atel_get_actor_start_pos(r, i, x, y, z),
        ),
        (
            0x0086_a390,
            "AtelGetActorEndPos",
            |r: &mut Run, i, x, y, z| hp::atel_get_actor_end_pos(r, i, x, y, z),
        ),
        (
            0x0086_a2f0,
            "Script_GetWorkerPos",
            |r: &mut Run, i, x, y, z| hp::script_get_worker_pos(r, i, x, y, z),
        ),
    ];
    // separate outs, then outs over the source block (the order shows)
    let src = Mem::read::<u32>(b, w);
    let t = Mem::read::<u8>(b, src);
    let block = match t {
        5 | 6 => w + 0x284,
        _ => w + 0x558,
    };
    for (f, name, port) in gets {
        for (x, y, z) in [
            (out, out + 4, out + 8),
            (block + 4, block, block + 0x30),
            (block + 8, block + 0x34, block + 4),
        ] {
            r23f_positions(a, b, rng, w);
            let what = format!("{name} {what} out {x:#x}");
            r22_case(a, b, f, &[i, x, y, z], true, &what, |r| port(r, i, x, y, z));
        }
    }
    r23f_positions(a, b, rng, w);
    #[allow(clippy::cast_possible_truncation)] // random bytes
    poke(a, b, w + 0xaa, [0u8, 1, 1, 2][rng.below(4) as usize]);
    poke(a, b, w + 0x9c, [0, rng.next()][rng.below(2) as usize]);
    r22_case(
        a,
        b,
        0x0086_d950,
        &[i],
        true,
        &format!("AtelIsBind {what}"),
        |r| hp::atel_is_bind(r, i),
    );
    for n in [0, 1, 4] {
        r23f_queue(a, b, rng, w, n);
        for level in [u32::MAX, 0, 3, 8, 15, 16] {
            let what = format!("Atel_HasSignalFromLevel {what} {n} entries level {level}");
            r22_case(a, b, 0x0086_3520, &[i, level], true, &what, |r| {
                hp::atel_has_signal_from_level(r, i, level)
            });
        }
    }
    for bits in [
        0u16,
        0x0400,
        0x1400,
        0x1c00,
        u16::try_from(rng.below(0x1_0000)).unwrap(),
    ] {
        poke(a, b, w + 0x34, bits);
        let what = format!("Atel_GetWorkerBits10_12 {what} {bits:#x}");
        r22_case(a, b, 0x0086_a440, &[i], true, &what, |r| {
            hp::atel_get_worker_bits10_12(r, i)
        });
    }
    if t != 4 {
        let (va, vc) = (stored_value(rng), stored_value(rng));
        let what = format!("Atel_SetLevelVectorsXZ {what}");
        r22_case(a, b, 0x0086_f750, &[w, va, vc], true, &what, |r| {
            hp::atel_set_level_vectors_xz(r, w, va, vc)
        });
        for id in [i, 0xffff, 0x1_ffff] {
            let what = format!("Event_SetPlayer {what} id {id:#x}");
            r22_case(a, b, 0x0086_f6a0, &[id], true, &what, |r| {
                hp::event_set_player(r, id)
            });
        }
    }
}

/// The effect polls with the event effect state (`0x01133364`, `i8`) at
/// every value the jump tables tell apart (state 3 starts the effect).
fn r23f_polls(a: &mut Oracle, b: &mut Oracle, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let stat = a.rebase(0x0113_3364);
    // the effect's handler table (0x01133360) with a stand-in that only
    // returns (NkzVersion), so starting it runs no effect code
    let table = alloc_both(a, b, 4);
    let stand_in = a.rebase(0x00a5_cf00);
    poke(a, b, table, stand_in);
    let at = a.rebase(0x0113_3360);
    poke(a, b, at, table);
    for s in [-1i8, 0, 1, 2, 3, 4, 5, 6, 7, 0x7f] {
        poke(a, b, stat, s);
        poke(a, b, a.rebase(0x0132_6b58), 0x1234_u32);
        poke(a, b, a.rebase(0x0132_6b80), 0x11_u8);
        let what = format!("{what} state {s}");
        r22_case(
            a,
            b,
            0x0086_33a0,
            &[],
            true,
            &format!("Atel_EffectIdlePoll {what}"),
            |r| hp::atel_effect_idle_poll(r),
        );
        r22_case(
            a,
            b,
            0x0086_3350,
            &[],
            true,
            &format!("Atel_EffectEndPoll {what}"),
            |r| hp::atel_effect_end_poll(r),
        );
        r22_case(
            a,
            b,
            0x0086_33e0,
            &[7],
            true,
            &format!("Atel_EffectStartPoll {what}"),
            |r| hp::atel_effect_start_poll(r, 7),
        );
    }
}

/// Round 23 f (this machine): worker positions (`AtelGetActorStartPos`,
/// `AtelGetActorEndPos`, `Script_GetWorkerPos`, `Event_SetPlayer`), the bind
/// and signal tests, the `+0x34` bits, the level vectors, the effect polls,
/// `AtelRecoverPreBgPage`, `AtelExecReturn2` and the `AtelGetCharHandleFromBindPC`
/// copy: synthetic ctrl works of every worker type on the image, then the
/// workers of every ctrl work in use (`FFX_SNAPSHOT`).
#[test]
#[allow(clippy::too_many_lines)] // the image part, then the snapshot part
fn round23f_positions_and_polls_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_023f);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let works = a.rebase(0x0132_5ba0);
        let context = a.rebase(0x0132_6b28);
        let ws = r23d_ctrl(&mut a, &mut b, &mut rng, works, 8);
        poke(&mut a, &mut b, context, works);
        for (i, &w) in (0u32..).zip(&ws) {
            if i % 8 == 4 {
                continue; // type 4 reads address 0
            }
            let cache = a.rebase(0x0132_70e4);
            poke(&mut a, &mut b, cache, 0u32);
            r23f_cases(&mut a, &mut b, &mut rng, i, w, &format!("image worker {i}"));
        }
        r23f_polls(&mut a, &mut b, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut compared, mut stops) = (0, 0);
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let n = u32::from(Mem::read::<u16>(&b, c + ctrl::WORKERS)).min(16);
        for i in 0..n {
            let w = v::atel_get_basic_work(&mut b, i.cast_signed());
            orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
            if Mem::read::<u8>(&b, Mem::read::<u32>(&b, w)) == 4 {
                continue;
            }
            r23f_cases(
                &mut a,
                &mut b,
                &mut rng,
                i,
                w,
                &format!("ctrl {c:#x} worker {i}"),
            );
            compared += 1;
        }
        let mut p = fresh_run_pair(&pages, &exe, c);
        for chr in [0, 1, 2, 7, 0x12, u32::MAX] {
            let what = format!("AtelGetCharHandleFromBindPC2 ctrl {c:#x} chr {chr:#x}");
            if !r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_aee0,
                &[chr],
                true,
                &what,
                |r| hp::atel_get_char_handle_from_bind_pc(r, chr),
            ) {
                stops += 1;
            }
        }
        // the pre-BG page bytes both ways
        for (x, y) in [(0x80u8, 0xffu8), (3, 0x80), (0x80, 2), (1, 0)] {
            let at = p.0.rebase(0x0132_711d);
            poke(&mut p.0, &mut p.1, at, x);
            poke(&mut p.0, &mut p.1, at + 1, y);
            let what = format!("AtelRecoverPreBgPage ctrl {c:#x} {x:#x} {y:#x}");
            if !r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0087_3cc0,
                &[],
                false,
                &what,
                |r| {
                    hp::atel_recover_pre_bg_page(r);
                    0
                },
            ) {
                stops += 1;
            }
        }
        // AtelExecReturn2 with and without a request level, as the battle
        // calls it (i, 0, 0)
        for i in 0..n.min(6) {
            let w = v::atel_get_basic_work(&mut p.1, i.cast_signed());
            orig(&mut p.0, ATEL_GET_BASIC_WORK, &[i]);
            for lvl in [0u8, 1, 3] {
                poke(&mut p.0, &mut p.1, w + 0x32, lvl);
                let what = format!("AtelExecReturn2 ctrl {c:#x} worker {i} level {lvl}");
                if !r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0086_9350,
                    &[i, 0, 0],
                    false,
                    &what,
                    |r| {
                        hp::atel_exec_return2(r, i, 0, 0);
                        0
                    },
                ) {
                    stops += 1;
                }
            }
        }
        let mut p = fresh_run_pair(&pages, &exe, c);
        r23f_polls(&mut p.0, &mut p.1, &format!("ctrl {c:#x}"));
    }
    eprintln!("round 23f: {compared} snapshot workers, {stops} stopped");
    assert!(compared > 0);
}

/// An event header at a fresh address with its sound table set as `mode`
/// says: 0 none, 1 an offset that wraps to address 0, 2 a table without
/// a foot table and a music word `music`, 3 a table with both.
fn r23g_header(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, mode: u32, music: u32) -> u32 {
    let hdr = alloc_both(a, b, 0x200);
    fill_both(a, b, rng, hdr, 0x200);
    let t = match mode {
        0 => 0,
        1 => 0u32.wrapping_sub(hdr),
        _ => 0x80,
    };
    poke(a, b, hdr + 0x2c, t);
    if mode >= 2 {
        poke(
            a,
            b,
            hdr + 0x80 + 0x2c,
            if mode == 3 { 0x100u32 } else { 0 },
        );
        poke(a, b, hdr + 0x80 + 0x38, music);
    }
    hdr
}

/// Round 23 g's set-up helpers that need no file on one pair (image or
/// snapshot); `w` worker `i` of the current ctrl work.
#[allow(clippy::too_many_lines)] // one block per helper
fn r23g_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, i: u32, w: u32, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    // the debug work: counts 0, 1, inside, 8, past, huge and negative
    for n in [0u32, 1, 3, 8, 9, 0x8000_0000, u32::MAX] {
        let src = alloc_both(a, b, 0x80);
        fill_both(a, b, rng, src, 0x80);
        poke(a, b, src, n);
        let what = format!("AtelInitSourceCodeDebugWork {what} count {n:#x}");
        r22_case(a, b, 0x0086_d670, &[i, src], false, &what, |r| {
            hp::atel_init_source_code_debug_work(r, i, src);
            0
        });
    }
    r22_case(
        a,
        b,
        0x0086_d670,
        &[i, 0],
        false,
        &format!("AtelInitSourceCodeDebugWork {what} none"),
        |r| {
            hp::atel_init_source_code_debug_work(r, i, 0);
            0
        },
    );
    let flag = a.rebase(0x0132_5ba1);
    for f in [0u8, 0x80, 0x7f, 0xff] {
        poke(a, b, flag, f);
        r22_case(
            a,
            b,
            0x0086_61a0,
            &[],
            false,
            &format!("AtelEventRemoveGuideMap {what} {f:#x}"),
            |r| {
                hp::atel_event_remove_guide_map(r);
                0
            },
        );
        poke(a, b, flag, f);
        r22_case(
            a,
            b,
            0x0086_6490,
            &[],
            true,
            &format!("AtelEventSetupGuideMap {what} {f:#x}"),
            |r| hp::atel_event_setup_guide_map(r),
        );
    }
    for mode in 0..4 {
        let hdr = r23g_header(a, b, rng, mode, 7);
        let what = format!("{what} header mode {mode}");
        r22_case(
            a,
            b,
            0x0086_6210,
            &[hdr],
            true,
            &format!("AtelEventSetUpFoot3d {what}"),
            |r| hp::atel_event_set_up_foot_3d(r, hdr),
        );
        // a music word that is not negative: nothing read
        r22_case(
            a,
            b,
            0x0086_5a60,
            &[hdr],
            true,
            &format!("AtelEventLoadMusic {what}"),
            |r| hp::atel_event_load_music(r, hdr),
        );
    }
    fill_both(a, b, rng, a.rebase(0x00c0_a004), 0x10);
    fill_both(a, b, rng, a.rebase(0x0132_8278), 0x10);
    r22_case(
        a,
        b,
        0x0087_7420,
        &[],
        true,
        &format!("AtelInitCalcSound {what}"),
        |r| hp::atel_init_calc_sound(r),
    );
    fill_both(a, b, rng, w + 0x40, 0x48);
    r22_case(
        a,
        b,
        0x0086_d260,
        &[i],
        true,
        &format!("AtelInitInternalActorWork {what}"),
        |r| hp::atel_init_internal_actor_work(r, i),
    );
    let rec = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    for flags in [0, 0x80, rng.next()] {
        fill_both(a, b, rng, rec + 0xd8, 0x10);
        let movie = rng.below(0x40);
        let what = format!("AtelSetOnMemoryMovieFileWait {what} {movie} {flags:#x}");
        r22_case(a, b, 0x0087_3f70, &[movie, flags], true, &what, |r| {
            hp::atel_set_on_memory_movie_file_wait(r, movie, flags)
        });
    }
    poke(a, b, a.rebase(0x0132_6b82), 1u8);
    r22_case(
        a,
        b,
        0x0086_e700,
        &[],
        true,
        &format!("AtelRecoverFromMenuCallback {what}"),
        |r| hp::atel_recover_from_menu_callback(r),
    );
    // the guide map angle: ctrl work 0 in use or not, with its block or none
    let c0 = a.rebase(0x0132_5ba0);
    let (f0, blk) = (Mem::read::<u8>(b, c0), Mem::read::<u32>(b, c0 + 0x48));
    for (on, with) in [(true, true), (false, true), (true, false)] {
        poke(a, b, c0, if on { f0 | 1 } else { f0 & !1 });
        poke(a, b, c0 + 0x48, if with { blk } else { 0 });
        for k in [0, 1, 5] {
            clear_stacks(a, b);
            let st0 = orig_float(a, 0x0086_b9a0, &[k]);
            let rb = hp::atel_get_guide_map_ang_ex(&Run(b), k);
            assert!(
                same_float(st0, rb.to_bits()),
                "AtelGetGuideMapAngEx {what} on {on} block {with} {k}: {st0} {rb}"
            );
            native_no_diff(a, b, "AtelGetGuideMapAngEx");
        }
    }
    poke(a, b, c0, f0);
    poke(a, b, c0 + 0x48, blk);
}

/// Round 23 g (this machine): the event set-up helpers (debug work,
/// guide map flag, foot table, music load, calc sound, internal actor
/// work, movie wait, menu return, guide map angle): a synthetic ctrl work
/// on the image, then every ctrl work in use (`FFX_SNAPSHOT`), where a
/// negative music word also reads the music.
#[test]
fn round23g_setup_helpers_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0240);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let works = a.rebase(0x0132_5ba0);
        let context = a.rebase(0x0132_6b28);
        let ws = r23d_ctrl(&mut a, &mut b, &mut rng, works, 3);
        poke(&mut a, &mut b, context, works);
        // an event block whose expansion (+0x2c -> +0x40) has a float
        // record (+0x3c -> +0x80), floats without signalling NaNs
        let blk = alloc_both(&mut a, &mut b, 0x100);
        fill_both(&mut a, &mut b, &mut rng, blk, 0x100);
        poke(&mut a, &mut b, blk + 0x2c, 0x40u32);
        poke(&mut a, &mut b, blk + 0x40 + 0x3c, 0x80u32);
        for k in 0..8 {
            let v = stored_value(&mut rng);
            poke(&mut a, &mut b, blk + 0x80 + 0x44 + k * 4, v);
        }
        poke(&mut a, &mut b, works + 0x48, blk);
        // the Phyre scene (0x00ccb9d8) the movie wait stores into
        let scene = alloc_both(&mut a, &mut b, 0x1100);
        let at = a.rebase(0x00cc_b9d8);
        poke(&mut a, &mut b, at, scene);
        for (i, &w) in (0u32..).zip(&ws) {
            r23g_cases(&mut a, &mut b, &mut rng, i, w, &format!("image worker {i}"));
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut reads, mut stops) = (0, 0);
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let w = v::atel_get_basic_work(&mut b, 0);
        orig(&mut a, ATEL_GET_BASIC_WORK, &[0]);
        r23g_cases(&mut a, &mut b, &mut rng, 0, w, &format!("ctrl {c:#x}"));
        let mut p = fresh_run_pair(&pages, &exe, c);
        for music in [0x8000_0005u32, 0xffff_0001] {
            let hdr = r23g_header(&mut p.0, &mut p.1, &mut rng, 3, music);
            let what = format!("AtelEventLoadMusic ctrl {c:#x} music {music:#x}");
            if r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_5a60,
                &[hdr],
                true,
                &what,
                |r| hp::atel_event_load_music(r, hdr),
            ) {
                reads += 1;
            } else {
                stops += 1;
            }
        }
    }
    eprintln!("round 23g: {reads} music reads compared, {stops} stopped");
}

/// Worker `w`'s four FP slots (`+0xb18`): ids free, free with bit 15, 3,
/// 5 with a position, 7, bones random, positions without signalling NaNs.
fn r23h_slots(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32) {
    let ids = [0u16, 0x8000, 3, 0x8005, 7, 3];
    for k in 0..4 {
        let s = w + 0xb18 + k * 0x10;
        poke(a, b, s, ids[rng.below(6) as usize]);
        poke(a, b, s + 2, u16::try_from(rng.below(0x40)).unwrap());
        for j in 0..3 {
            poke(a, b, s + 4 + j * 4, stored_value(rng));
        }
    }
}

/// The four FP helpers on worker `w` with binding `bind` and model
/// `model` (`+0xaa`, `+0x9c`), fresh slots before each case.
fn r23h_cases(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: u32,
    bind: u8,
    model: u32,
    what: &str,
) {
    use ffx_script::vm::bridge::helpers as hp;
    poke(a, b, w + 0xaa, bind);
    poke(a, b, w + 0x9c, model);
    let what = format!("{what} binding {bind} model {model:#x}");
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    for fp in [3, 9, 0x1_0003] {
        let bone = rng.below(0x40);
        r23h_slots(a, b, rng, w);
        r22_case(
            a,
            b,
            0x0086_2800,
            &[w, fp, bone],
            false,
            &format!("AtelBindFp {what} fp {fp:#x}"),
            |r| {
                hp::atel_bind_fp(r, w, fp, bone);
                0
            },
        );
    }
    for fp in [0, 3, 5, 0x8003] {
        r23h_slots(a, b, rng, w);
        r22_case(
            a,
            b,
            0x0086_28a0,
            &[w, fp],
            false,
            &format!("AtelBindFpOff {what} fp {fp:#x}"),
            |r| {
                hp::atel_bind_fp_off(r, w, fp);
                0
            },
        );
    }
    for fp in [3, 5, 9] {
        r23h_slots(a, b, rng, w);
        let pos = [stored_value(rng), stored_value(rng), stored_value(rng)];
        let what = format!("AtelBindFpPos {what} fp {fp}");
        r22_case(
            a,
            b,
            0x0086_2920,
            &[w, fp, pos[0], pos[1], pos[2]],
            false,
            &what,
            |r| {
                let f = frame(r);
                hp::atel_bind_fp_pos(r, w, fp, pos, f);
                0
            },
        );
    }
    for round in 0..3 {
        r23h_slots(a, b, rng, w);
        r22_case(
            a,
            b,
            0x0086_e4c0,
            &[w],
            false,
            &format!("AtelRecoverFPFromBattle {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_recover_fp_from_battle(r, w, f);
                0
            },
        );
    }
}

/// Round 23 h (this machine): the FP bindings (`AtelBindFp`,
/// `AtelBindFpOff`, `AtelBindFpPos`, `AtelRecoverFPFromBattle`) on synthetic
/// workers of types 1 and 0 with every binding (character binding without a
/// model; the image), then on the snapshots' type-1 workers with their own
/// character models and the map bindings (`FFX_SNAPSHOT`).
#[test]
fn round23h_fp_bindings_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0241);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        for t in [1u8, 0, 2] {
            let w = r22_worker(&mut a, &mut b, &mut rng, t);
            let big = alloc_both(&mut a, &mut b, 0xc00);
            let hdr = Mem::read::<u32>(&b, w);
            poke(&mut a, &mut b, big, hdr);
            for bind in [0u8, 1, 2, 4, 5] {
                r23h_cases(
                    &mut a,
                    &mut b,
                    &mut rng,
                    big,
                    bind,
                    0,
                    &format!("image type {t}"),
                );
            }
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut chars = 0;
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let n = u32::from(Mem::read::<u16>(&b, c + ctrl::WORKERS)).min(16);
        for i in 0..n {
            let w = v::atel_get_basic_work(&mut b, i.cast_signed());
            orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
            if Mem::read::<u8>(&b, Mem::read::<u32>(&b, w)) != 1 {
                continue;
            }
            let (bind, model) = (
                Mem::read::<u8>(&b, w + 0xaa),
                Mem::read::<u32>(&b, w + 0x9c),
            );
            let what = format!("ctrl {c:#x} worker {i}");
            if bind == 1 && model != 0 {
                r23h_cases(&mut a, &mut b, &mut rng, w, 1, model, &what);
                chars += 1;
            }
            for bind in [4u8, 5, 3] {
                r23h_cases(&mut a, &mut b, &mut rng, w, bind, model, &what);
            }
            // a character binding without a model
            r23h_cases(&mut a, &mut b, &mut rng, w, 1, 0, &what);
        }
    }
    eprintln!("round 23h: {chars} character bindings");
}

/// A header for `AtelGetMapIndex` (`mode`: 0 no entrance table, 1 no
/// sound table, 2 no map record, 3 a record without a language table, 4
/// with one), entrances of random maps and language words.
fn r23i_map_header(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, mode: u32) -> u32 {
    let hdr = alloc_both(a, b, 0x400);
    fill_both(a, b, rng, hdr, 0x400);
    poke(a, b, hdr + 4, if mode == 0 { 0u32 } else { 0x100 });
    poke(a, b, hdr + 0x2c, if mode == 1 { 0u32 } else { 0x40 });
    poke(a, b, hdr + 0x40 + 0x3c, if mode == 2 { 0u32 } else { 0x80 });
    poke(a, b, hdr + 0x80 + 0xc, if mode == 3 { 0u32 } else { 0xc0 });
    hdr
}

/// Round 23 i on one pair (image or snapshot): the radius tests on worker
/// `w`, the map index, the event music and the map camera request.
#[allow(clippy::too_many_lines)] // one block per helper
fn r23i_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: u32, cam_to: u32, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    for round in 0..12 {
        r23f_positions(a, b, rng, w);
        let r = [0x4120_0000, 0, stored_value(rng)][round % 3];
        // points near the worker's block point (some inside the radius)
        let t = Mem::read::<u8>(b, Mem::read::<u32>(b, w));
        let block = if t == 5 || t == 6 {
            w + 0x284
        } else {
            w + 0x558
        };
        let mut near = |k: u32| {
            let c = f32::from_bits(Mem::read::<u32>(b, block + k * 4));
            let d = f32::from(u16::try_from(rng.below(16)).unwrap()) - 8.0;
            if c.is_finite() {
                (c + d).to_bits()
            } else {
                d.to_bits()
            }
        };
        let pos = [near(0), near(1), near(2)];
        let args = [r, w, pos[0], pos[1], pos[2]];
        r22_case(
            a,
            b,
            0x0086_34b0,
            &args,
            true,
            &format!("Atel_InRadiusXZ {what} {round}"),
            |rr| hp::atel_in_radius_xz(rr, r, w, pos[0], pos[2]),
        );
        r22_case(
            a,
            b,
            0x0086_3440,
            &args,
            true,
            &format!("Atel_InRadiusXYZ {what} {round}"),
            |rr| hp::atel_in_radius_xyz(rr, r, w, pos),
        );
    }
    let lang = a.rebase(0x0133_0830);
    for mode in 0..5 {
        let hdr = r23i_map_header(a, b, rng, mode);
        for l in [-3i8, 0, 5, 15, 16, 0x7f] {
            poke(a, b, lang, l);
            for i in [0, 1, 3] {
                let what = format!("AtelGetMapIndex {what} mode {mode} lang {l} entrance {i}");
                r22_case(a, b, 0x0086_bd90, &[hdr, i], true, &what, |r| {
                    hp::atel_get_map_index(r, hdr, i)
                });
            }
        }
    }
    // the event music without a sound call: no music word, a positive
    // one, the music already playing, a fade with nothing playing
    let now = a.rebase(0x0132_ff5c);
    for (mode, music, playing) in [
        (0, 0x8003_0005u32, 5u32),
        (2, 5, 7),
        (3, 0x8003_0005, 5),
        (3, 0x8003_0105, u32::MAX),
    ] {
        let hdr = r23g_header(a, b, rng, mode, music);
        poke(a, b, now, playing);
        let what = format!("AtelEventPlayMusic {what} music {music:#x} playing {playing:#x}");
        r22_case(a, b, 0x0086_5ac0, &[hdr], true, &what, |r| {
            hp::atel_event_play_music(r, hdr)
        });
    }
    // the map camera: no worker, a table (index inside and past the end),
    // no table (the default record), camera 0 and not
    let jump = orig(a, 0x0078_5240, &[]);
    orig(b, 0x0078_5240, &[]);
    for round in 0..12_u32 {
        let set = alloc_both(a, b, 0x200);
        fill_both(a, b, rng, set, 0x200);
        poke(
            a,
            b,
            set + 0x18,
            if round == 0 {
                0xffffu16
            } else {
                u16::try_from(cam_to).unwrap()
            },
        );
        poke(a, b, set + 4, if round % 4 == 3 { 0u32 } else { 0x40 });
        poke(
            a,
            b,
            set + 8,
            [0x60u32, 0x100, 0x41, 0][(round % 4) as usize],
        );
        poke(a, b, jump + 0xc, u8::try_from(round / 4 % 3).unwrap());
        let cam: i16 = if round == 11 {
            0
        } else {
            i16::try_from(rng.below(30)).unwrap() - 3
        };
        for e in 0..6 {
            poke(
                a,
                b,
                set + 0x40 + e * 32 + 6,
                if cam == 0 {
                    0
                } else {
                    cam + i16::try_from(e).unwrap()
                },
            );
        }
        let def = a.rebase(0x0132_5b56);
        poke(a, b, def, cam);
        let what = format!("AtelRequestMapCamera {what} round {round}");
        r22_case(a, b, 0x0086_ed80, &[set], true, &what, |r| {
            hp::atel_request_map_camera(r, set)
        });
    }
}

/// Round 23 i (this machine): `Atel_InRadiusXZ`/`XYZ`, `AtelGetMapIndex`,
/// `AtelEventPlayMusic` and `AtelRequestMapCamera`: a synthetic ctrl work on
/// the image, then every ctrl work in use (`FFX_SNAPSHOT`).
#[test]
fn round23i_radius_music_camera_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0242);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let works = a.rebase(0x0132_5ba0);
        let context = a.rebase(0x0132_6b28);
        let ws = r23d_ctrl(&mut a, &mut b, &mut rng, works, 8);
        poke(&mut a, &mut b, context, works);
        for (i, &w) in (0u32..).zip(&ws) {
            if i % 8 == 4 {
                continue;
            }
            // signals need a free node list; the image has none, so the
            // camera request goes to a receiver past the end (worker 0)
            r23i_cases(
                &mut a,
                &mut b,
                &mut rng,
                w,
                0xffff,
                &format!("image worker {i}"),
            );
        }
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let (mut compared, mut music_calls, mut stops) = (0, 0, 0);
    for c in ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let n = u32::from(Mem::read::<u16>(&b, c + ctrl::WORKERS)).min(6);
        for i in 0..n {
            let w = v::atel_get_basic_work(&mut b, i.cast_signed());
            orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
            if Mem::read::<u8>(&b, Mem::read::<u32>(&b, w)) == 4 {
                continue;
            }
            r23i_cases(
                &mut a,
                &mut b,
                &mut rng,
                w,
                i,
                &format!("ctrl {c:#x} worker {i}"),
            );
            compared += 1;
        }
        // the music calls: other or no music playing, numbers below and
        // from 0x100 (an original that stops starts the pair again)
        let mut p = fresh_run_pair(&pages, &exe, c);
        for music in [
            0x8003_0005u32,
            0x8003_0105,
            0xffff_ffff,
            0x8000_00ff,
            0x8003_0100,
        ] {
            for playing in [u32::MAX, 5, 7, 0x105, 0, 0x100] {
                let hdr = r23g_header(&mut p.0, &mut p.1, &mut rng, 3, music);
                let now = p.0.rebase(0x0132_ff5c);
                poke(&mut p.0, &mut p.1, now, playing);
                let what =
                    format!("AtelEventPlayMusic ctrl {c:#x} music {music:#x} playing {playing:#x}");
                if r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0086_5ac0,
                    &[hdr],
                    true,
                    &what,
                    |r| hp::atel_event_play_music(r, hdr),
                ) {
                    music_calls += 1;
                } else {
                    stops += 1;
                }
            }
        }
    }
    eprintln!("round 23i: {compared} snapshot workers, {music_calls} music calls, {stops} stopped");
    assert!(compared > 0);
}

/// Replaces each `(f, n)` in both oracles with a stub that logs `f` and
/// its first `n` stack words into a shared buffer and returns the log's
/// new end (so a dropped or reordered call, a changed argument or a
/// result the caller ignores shows in the memory comparison). Returns the
/// log's cursor cell.
fn r24_log_stubs(a: &mut Oracle, b: &mut Oracle, fs: &[(u32, u8)]) -> u32 {
    let log = alloc_both(a, b, 0x8000);
    let cell = alloc_both(a, b, 4);
    poke(a, b, cell, log);
    for &(f, n) in fs {
        let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &cell.to_le_bytes()].concat(); // mov ecx, [cell]
        code.extend([&[0xc7, 0x01][..], &f.to_le_bytes()].concat()); // mov [ecx], f
        for k in 0..n {
            code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 + 4 * k]); // [ecx+4+4k] = arg k
        }
        code.extend([0x83, 0xc1, 4 + 4 * n]); // add ecx, 4 + 4n
        code.extend([&[0x89, 0x0d][..], &cell.to_le_bytes()].concat()); // mov [cell], ecx
        code.extend([0x89, 0xc8, 0xc3]); // mov eax, ecx; ret
        let at = a.alloc_code(&code).unwrap();
        assert_eq!(b.alloc_code(&code).unwrap(), at);
        for o in [&mut *a, &mut *b] {
            let entry = o.rebase(f);
            let rel = at.wrapping_sub(entry + 5);
            o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
                .unwrap();
        }
    }
    cell
}

/// `TOGetFFXLang`'s manager (`0x00cded58`): a fresh one on the image, the
/// snapshot's own otherwise; returns where its language word is.
fn r24_lang_word(a: &mut Oracle, b: &mut Oracle) -> u32 {
    let at = a.rebase(0x00cd_ed58);
    let mut m = Mem::read::<u32>(b, at);
    if m == 0 {
        m = alloc_both(a, b, 0x40);
        poke(a, b, at, m);
    }
    m + 4
}

/// Round 24 a on one pair: `getEventLocTexListPath` for event names with
/// and without `bltz`/`sins05`/the five tutorial names, every language
/// 0..=12 and map `0x79` or not.
fn r24a_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) -> u32 {
    use ffx_script::vm::bridge::helpers as hp;
    let lang = r24_lang_word(a, b);
    // fiosUnifyFilename's stream prefix (none on the image) and its
    // overflow message (Phyre_PhyrePrintf)
    let prefix = a.rebase(0x00cc_9698);
    if Mem::read::<u32>(b, prefix) == 0 {
        let s = alloc_both(a, b, 0x10);
        for (k, &c) in (0u32..).zip(b"data/\0") {
            poke(a, b, s + k, c);
        }
        poke(a, b, prefix, s);
    }
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(0x0043_52f0);
        o.stub_return(at, 0).unwrap();
    }
    let name_at = a.rebase(0x023c_bc60);
    let map = a.rebase(0x012f_bc40);
    let table = a.rebase(0x00c5_2768);
    let saved: Vec<u32> = (0..12)
        .map(|k| Mem::read::<u32>(b, table + 4 * k))
        .collect();
    let mut real = vec![];
    for k in 0..0x40 {
        let c = Mem::read::<u8>(b, name_at + k);
        if c == 0 {
            break;
        }
        real.push(c);
    }
    // long, but short enough for the 256-byte locals (the game ends on
    // a longer one: fiosUnifyFilename's sprintf_s)
    let long_name = "ab".repeat(75);
    let mut names: Vec<Vec<u8>> = [
        "",
        "bltz0000",
        "bl/bltz0005",
        "bl/bltz0006",
        "bl/bltz0200",
        "zz/bltz0201",
        "bltz0201",
        "bl/bltz0100",
        "bltz",
        "ss/sins0500",
        "sins05",
        "ss/sins0400",
        "bj/bjyt0000",
        "x/bltz0000/bltz0005",
        long_name.as_str(),
    ]
    .iter()
    .map(|s| s.as_bytes().to_vec())
    .collect();
    names.push(real);
    let mut n = 0;
    for (i, name) in names.iter().enumerate() {
        let mut z = name.clone();
        z.push(0);
        Mem::write_bytes(a, name_at, &z);
        Mem::write_bytes(b, name_at, &z);
        for l in 0..=12u32 {
            for m79 in [false, true] {
                // the prefix table back as it was (language 11 on map 0x79
                // moves its word 11)
                for (k, &w) in (0u32..).zip(&saved) {
                    poke(a, b, table + 4 * k, w);
                }
                if l % 2 == 1 {
                    // a prefix of its own for every language (the image's
                    // share "" between 0, 1 and 6..)
                    for k in 0..12u32 {
                        let p = alloc_both(a, b, 8);
                        for (j, &c) in (0u32..).zip(format!("L{k}/\0").as_bytes()) {
                            poke(a, b, p + j, c);
                        }
                        poke(a, b, table + 4 * k, p);
                    }
                }
                poke(a, b, lang, l);
                poke(a, b, map, if m79 { 0x79u32 } else { 0x41 + rng.below(4) });
                let out = alloc_both(a, b, 0x300);
                fill_both(a, b, rng, out, 0x40);
                let what = format!("getEventLocTexListPath {what} name {i} lang {l} map79 {m79}");
                r22_case(a, b, 0x0087_4c50, &[out], true, &what, |r| {
                    let frame = r.0.stack_range().start + 0x100;
                    hp::get_event_loc_tex_list_path(r, out, frame)
                });
                n += 1;
            }
        }
    }
    n
}

/// Round 24 a (this machine): `getEventLocTexListPath` on the image, then
/// on the snapshot (`FFX_SNAPSHOT`), with its own event name among the
/// names.
#[test]
fn round24a_tex_list_path_matches_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0243);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let n = r24a_cases(&mut a, &mut b, &mut rng, "image");
        assert!(n > 0);
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    let n = r24a_cases(&mut a, &mut b, &mut rng, "snapshot");
    eprintln!("round 24a: {n} snapshot calls");
}

/// Round 24 a on one pair: `AtelStopSoundAll` as it stands,
/// `AtelGetHyoukaMenuEnable` over ctrl work 0's flag bytes, the overlay
/// status and every callee result (stubbed alike on both sides), and
/// `0x008630c0` with ctrl work 0's run bit on and off and the worker
/// cache cold, warm and pointing elsewhere.
#[allow(clippy::too_many_lines)] // one block per helper
fn r24a_more(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str, synthetic: bool) {
    use ffx_script::vm::bridge::helpers as hp;
    // the image has no sound system (SndSepStop reads through 0)
    if !synthetic {
        r22_case(
            a,
            b,
            0x0087_1890,
            &[],
            true,
            &format!("AtelStopSoundAll {what}"),
            |r| hp::atel_stop_sound_all(r),
        );
    }
    // then logging stand-ins for every callee, on both
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_6eb0, 1),
            (0x0088_7f00, 1),
            (0x0088_7b60, 0),
            (0x0088_6a60, 1),
            (0x0088_6800, 1),
            (0x0067_a230, 0),
            (0x0067_a030, 0),
            (0x0088_72e0, 0),
        ],
    );
    r22_case(
        a,
        b,
        0x0087_1890,
        &[],
        true,
        &format!("AtelStopSoundAll {what} logged"),
        |r| hp::atel_stop_sound_all(r),
    );
    let works = a.rebase(0x0132_5ba0);
    let context = a.rebase(0x0132_6b28);
    let overlay = a.rebase(0x0132_6b80);
    let saved: Vec<u8> = (0..4).map(|k| Mem::read::<u8>(b, works + k)).collect();
    let callees = [
        0x0088_d6d0u32,
        0x0078_0c90,
        0x0067_ab50,
        0x0088_6650,
        0x0076_f060,
    ];
    for round in 0..160u32 {
        // flags: mostly the enabling pattern with one bit flipped
        let mut f = [0x13u8, 0x10, 0x00, 0x00];
        match round % 4 {
            0 => {}
            1 => {
                let bit = rng.below(32);
                f[(bit / 8) as usize] ^= 1 << (bit % 8);
            }
            _ => {
                for x in &mut f {
                    *x = u8::try_from(rng.below(256)).unwrap();
                }
            }
        }
        for (k, &x) in (0u32..).zip(&f) {
            poke(a, b, works + k, x);
        }
        poke(
            a,
            b,
            overlay,
            u8::try_from([0, 1, 2, 0][(round / 4 % 4) as usize]).unwrap(),
        );
        let other = if round % 3 == 0 { works } else { works + 0x238 };
        poke(a, b, context, other);
        // one callee says yes in a third of the rounds (each in turn)
        let yes = if round % 3 == 2 {
            Some(callees[(round / 3 % 5) as usize])
        } else {
            None
        };
        for f in callees {
            let v = u32::from(yes == Some(f)) * [1, 0x100, 0xffff_ffff][(round % 3) as usize];
            for o in [&mut *a, &mut *b] {
                let at = o.rebase(f);
                o.stub_return(at, v).unwrap();
            }
        }
        r22_case(
            a,
            b,
            0x0086_bae0,
            &[],
            true,
            &format!("AtelGetHyoukaMenuEnable {what} round {round}"),
            |r| hp::atel_get_hyouka_menu_enable(r),
        );
    }
    for (k, &x) in (0u32..).zip(&saved) {
        poke(a, b, works + k, x);
    }
    let cache = a.rebase(0x0132_70e4);
    let cache_ctrl = a.rebase(0x0132_70dc);
    let cache_index = a.rebase(0x0132_70e0);
    for round in 0..8u32 {
        if synthetic {
            let n = [0u16, 1, 5, 9][(round % 4) as usize];
            r23d_ctrl(a, b, rng, works, n);
            if round == 6 {
                // fewer workers than actors: the rest wrap to worker 0
                poke(a, b, works + ctrl::WORKERS, 2u16);
            }
        }
        let bit = u8::from(round % 4 != 3);
        let f = Mem::read::<u8>(b, works);
        poke(a, b, works, f & !1 | bit);
        let id = Mem::read::<u16>(b, works + ctrl::ID);
        match round % 3 {
            0 => poke(a, b, cache, 0u32),
            1 => {
                // warm for index 0 of this ctrl work: a stand-in worker
                let fake = alloc_both(a, b, 0x40);
                poke(a, b, cache, fake);
                poke(a, b, cache_ctrl, id);
                poke(a, b, cache_index, 0u16);
            }
            _ => {
                poke(a, b, cache_ctrl, id.wrapping_add(1));
                poke(a, b, cache_index, 1u16);
            }
        }
        r22_case(
            a,
            b,
            0x0086_30c0,
            &[],
            bit == 1,
            &format!("Atel_SetRunInBattle {what} round {round}"),
            |r| hp::atel_set_run_in_battle(r),
        );
    }
}

/// Round 24 a (this machine), the rest: `AtelStopSoundAll`,
/// `AtelGetHyoukaMenuEnable`, `0x008630c0` on the image (a synthetic ctrl
/// work 0) and on the snapshot (its own).
#[test]
fn round24a_sound_menu_battle_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0244);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r24a_more(&mut a, &mut b, &mut rng, "image", true);
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    let n = Mem::read::<u16>(&b, b.rebase(0x0132_5ba0) + ctrl::ACTORS);
    eprintln!("round 24a: snapshot ctrl work 0 has {n} actors");
    r24a_more(&mut a, &mut b, &mut rng, "snapshot", false);
}

/// Round 24 b on one pair: `0x0086d510` on synthetic workers of every
/// type (an empty free list or one with a node) and on `real` workers,
/// `0x00863c90` on banks in and past the two, `0x00866520` over event
/// separate tables, modes, groups and masks with its two sound calls
/// logged.
#[allow(clippy::too_many_lines)] // one block per helper
fn r24b_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, real: &[u32], what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let mut workers = real.to_vec();
    for kind in 0..8u8 {
        let hdr = alloc_both(a, b, 0x40);
        poke(a, b, hdr, kind);
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w, 0x600);
        poke(a, b, w, hdr);
        if kind % 2 == 0 {
            poke(a, b, w + 0x88, 0u32);
        } else {
            let node = alloc_both(a, b, 0x18);
            poke(a, b, w + 0x88, node);
        }
        workers.push(w);
    }
    for (i, &w) in workers.iter().enumerate() {
        if i < real.len() {
            // AtelInitBasicWork empties the list first (a snapshot's
            // unused workers hold the heap's fill there)
            poke(a, b, w + 0x88, 0u32);
        }
        r22_case(
            a,
            b,
            0x0086_d510,
            &[w],
            true,
            &format!("Atel_InitFreeSignals {what} worker {i}"),
            |r| hp::atel_init_free_signals(r, w),
        );
    }
    let status = a.rebase(0x0132_6b82);
    for bank in [0u32, 1, 0xff, 2, 0x101, 0xffff_ff00] {
        poke(a, b, status, u8::try_from(rng.below(256)).unwrap());
        r22_case(
            a,
            b,
            0x0086_3c90,
            &[bank],
            true,
            &format!("Atel_ResetMsgWindowBank {what} bank {bank:#x}"),
            |r| hp::atel_reset_msg_window_bank(r, bank),
        );
    }
    // the separates: logging stand-ins for the two sound calls
    r24_log_stubs(a, b, &[(0x0088_6dc0, 8), (0x0088_6db0, 0)]);
    let header = a.rebase(0x0132_5be8);
    let mask = a.rebase(0x0132_6bdc);
    let list = a.rebase(0x0132_7030);
    for round in 0..240u32 {
        let hdr = alloc_both(a, b, 0x400);
        fill_both(a, b, rng, hdr, 0x40);
        let n = [0i32, 1, 3, 8, 12, 17, 20, -2][(round % 8) as usize];
        // +0x2c -> +0xc -> (n, then (id, flags) pairs), all from the header
        poke(a, b, hdr + 0x2c, if round % 31 == 30 { 0u32 } else { 0x40 });
        poke(
            a,
            b,
            hdr + 0x40 + 0xc,
            if round % 37 == 36 { 0u32 } else { 0x80 },
        );
        poke(a, b, hdr + 0x80, n);
        for i in 0..20u32 {
            let id = match rng.below(8) {
                0 => rng.next(),
                1 => u32::MAX - rng.below(4),
                _ => rng.below(12),
            };
            let low = rng.below(16);
            let groups = match rng.below(4) {
                0 => 0,
                1 => 1 << (16 + rng.below(8)),
                2 => rng.next() & 0x7f_0000,
                _ => rng.next(),
            };
            poke(a, b, hdr + 0x84 + i * 8, id);
            poke(a, b, hdr + 0x88 + i * 8, groups & !0xf | low);
        }
        poke(a, b, header, hdr);
        poke(a, b, mask, u8::try_from(rng.below(256)).unwrap());
        for i in 0..8 {
            poke(a, b, list + i * 8, rng.below(12));
            poke(a, b, list + i * 8 + 4, rng.next());
        }
        let mode = [0u32, 1, 0x100][(round % 3) as usize];
        let group = match round / 3 % 5 {
            // negative: -1, or one whose low five bits name a group
            0 if round % 2 == 0 => u32::MAX,
            0 => 0x8000_0000 | rng.below(8),
            1 => rng.below(8),
            2 => 8 + rng.below(24),
            3 => 32 + rng.below(8),
            _ => 0,
        };
        let p0 = rng.next();
        r22_case(
            a,
            b,
            0x0086_6520,
            &[p0, mode, group],
            true,
            &format!("Atel_LoadEventSeps {what} round {round} mode {mode:#x} group {group:#x}"),
            |r| hp::atel_load_event_seps(r, mode, group),
        );
    }
}

/// Round 24 b (this machine): `0x0086d510`, `0x00863c90`, `0x00866520`
/// on the image, then on the snapshot (`FFX_SNAPSHOT`) with every worker
/// of every ctrl work in use among the workers.
#[test]
fn round24b_signals_windows_seps_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0245);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r24b_cases(&mut a, &mut b, &mut rng, &[], "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    let context = a.rebase(0x0132_6b28);
    let mut real = vec![];
    for c in ctrls {
        poke(&mut a, &mut b, context, c);
        let n = Mem::read::<u16>(&b, c + ctrl::WORKERS);
        for i in 0..u32::from(n) {
            real.push(v::atel_get_basic_work(&mut b, i.cast_signed()));
            orig(&mut a, ATEL_GET_BASIC_WORK, &[i]);
        }
    }
    eprintln!("round 24b: {} snapshot workers", real.len());
    r24b_cases(&mut a, &mut b, &mut rng, &real, "snapshot");
}

/// `f` replaced in both oracles by a stand-in returning a countdown: the
/// cell's value, decremented while not 0 (n, n-1, ..., 1, then 0 for good);
/// the word after the cell counts the calls. Returns the cell.
fn r24_countdown_stub(a: &mut Oracle, b: &mut Oracle, f: u32) -> u32 {
    let cell = alloc_both(a, b, 8);
    #[rustfmt::skip]
    let code = [
        &[0xff, 0x05][..], &(cell + 4).to_le_bytes(), // inc dword [cell + 4]
        &[0xa1], &cell.to_le_bytes(),            // mov eax, [cell]
        &[0x85, 0xc0, 0x74, 0x06],               // test eax, eax; jz ret
        &[0xff, 0x0d], &cell.to_le_bytes(),      // dec dword [cell]
        &[0xc3],                                 // ret
    ]
    .concat();
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    cell
}

/// Round 24 c, the movie, menu and sound helpers on one pair (their callees
/// stubbed alike on both sides).
#[allow(clippy::too_many_lines)] // one block per helper
fn r24c_stubbed(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let pic = a.rebase(0x0112_a00c);
    // destroyMoviePicture and 0x0076ed10
    r24_log_stubs(a, b, &[(0x0063_df30, 0)]);
    for on in [0u32, 1, 0x100] {
        poke(a, b, pic, on);
        poke(a, b, a.rebase(0x0112_a008), 7u32);
        r22_case(
            a,
            b,
            0x0076_ece0,
            &[],
            on != 0,
            &format!("destroyMoviePicture {what} {on:#x}"),
            |r| hp::destroy_movie_picture(r),
        );
    }
    for k in 0..32u32 {
        for (bit, f) in [0x0064_1cf0u32, 0x0064_1ce0, 0x0076_f0c0]
            .into_iter()
            .enumerate()
        {
            let v = if k >> bit & 1 != 0 {
                [1, 0x100, u32::MAX][(k % 3) as usize]
            } else {
                0
            };
            for o in [&mut *a, &mut *b] {
                let at = o.rebase(f);
                o.stub_return(at, v).unwrap();
            }
        }
        poke(a, b, pic, (k >> 3 & 3) * 0x80);
        r22_case(
            a,
            b,
            0x0076_ed10,
            &[],
            true,
            &format!("Movie_IsBusy {what} {k}"),
            |r| hp::movie_is_busy(r),
        );
    }
    // the save screen's card poll: every step, TkMcGetInfo 0 and not
    let step = a.rebase(0x0186_626b);
    let info = a.rebase(0x0186_626c);
    let toggles = a.rebase(0x0186_626a);
    for k in 0..48u32 {
        for o in [&mut *a, &mut *b] {
            let at = o.rebase(0x0088_82c0);
            o.stub_return(at, [0, 1, 0x55][(k % 3) as usize]).unwrap();
        }
        poke(
            a,
            b,
            step,
            u8::try_from([0, 1, 2, 3, 0xff, 2][(k / 3 % 6) as usize]).unwrap(),
        );
        poke(a, b, info, [2u32, 3, 0, 0x102][(k / 18 % 4) as usize]);
        poke(a, b, toggles, u8::try_from(k % 2 * (1 + k % 5)).unwrap());
        poke(a, b, a.rebase(0x0186_6268), 0x77u8);
        r22_case(
            a,
            b,
            0x008b_3d20,
            &[],
            true,
            &format!("TkMenuSaveLoad_PollCard {what} {k}"),
            |r| hp::tk_menu_save_load_poll_card(r),
        );
    }
    // and its arguments, through a logging stand-in (a non-zero result)
    r24_log_stubs(a, b, &[(0x0088_82c0, 4)]);
    for st in [0u8, 1] {
        poke(a, b, step, st);
        r22_case(
            a,
            b,
            0x008b_3d20,
            &[],
            true,
            &format!("TkMenuSaveLoad_PollCard {what} logged {st}"),
            |r| hp::tk_menu_save_load_poll_card(r),
        );
    }
    // 0x00871780: the transfer wait, the queue, the group offsets
    let sync = r24_countdown_stub(a, b, 0x0088_7910);
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_7f00, 1),
            (0x0088_7b80, 0),
            (0x0086_38c0, 1),
            (0x0088_6f30, 2),
            (0x0088_6f00, 2),
        ],
    );
    let mask = a.rebase(0x0132_6bdc);
    for round in 0..160u32 {
        poke(a, b, sync, rng.below(4));
        poke(a, b, mask, u8::try_from(rng.below(256)).unwrap());
        let data = alloc_both(a, b, 0x80);
        let head = match round % 6 {
            0 => 0x4000_0000 | rng.below(0x100),
            1 => 0xc000_0000 | rng.below(0x100),
            2 => rng.below(0x100),
            3 => 0x8000_0000 | rng.below(0x100),
            _ => 0x4000_0000 | rng.next() & 0x3fff_ffff,
        };
        poke(a, b, data, head);
        for k in 1..12u32 {
            let size = if rng.below(5) == 0 {
                0
            } else {
                rng.below(0x1000)
            };
            poke(a, b, data + 4 * k, size);
        }
        let d = if round % 23 == 22 { 0 } else { data };
        let group = match round % 4 {
            0 => u32::MAX - rng.below(3),
            1 => rng.below(3),
            2 => rng.below(10),
            _ => 0x8000_0000 | rng.below(4),
        };
        let (hdr, sep_only, flag) = (rng.next(), rng.below(2) * rng.next(), rng.below(2));
        r22_case(
            a,
            b,
            0x0087_1780,
            &[hdr, d, sep_only, group, flag],
            true,
            &format!("Atel_TransSepGroup {what} round {round} group {group:#x} head {head:#x}"),
            |r| hp::atel_trans_sep_group(r, hdr, d, sep_only, group, flag),
        );
    }
}

/// Round 24 c on one pair with ctrl work 0 as it stands (the image's made
/// up by the caller): `0x00878140` over variable words of every storage
/// and slots in and past 0..=7, `0x0086b270` with every out-pointer
/// combination, `AtelGetGuideMapActorInfo` over the marker table and past
/// it (`synthetic`: a table of the test's own).
#[allow(clippy::too_many_lines)] // one block per helper
fn r24c_workers(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    synthetic: bool,
    what: &str,
) -> u32 {
    use ffx_script::vm::bridge::helpers as hp;
    let works = a.rebase(0x0132_5ba0);
    let context = a.rebase(0x0132_6b28);
    poke(a, b, context, works);
    let n = u32::from(Mem::read::<u16>(b, works + ctrl::WORKERS));
    let ws: Vec<u32> = (0..n)
        .map(|i| {
            orig(a, ATEL_GET_BASIC_WORK, &[i]);
            v::atel_get_basic_work(b, i.cast_signed())
        })
        .collect();
    let kind = |o: &Oracle, w: u32| Mem::read::<u8>(o, Mem::read::<u32>(o, w));
    let mut calls = 0;
    // 0x00878140
    let stk = alloc_both(a, b, 0x400);
    for round in 0..64u32 {
        let w = ws[(round % n) as usize];
        if kind(b, w) == 4 {
            continue;
        }
        let storage = round % 8;
        // storage 3 through the ctrl work's callback only on the snapshot
        if storage == 3 && synthetic {
            continue;
        }
        let word = if round % 9 == 8 {
            0
        } else {
            (rng.below(7) << 28) | (storage << 25) | rng.below(0x100)
        };
        let slot = [0u32, 3, 7, 8, u32::MAX, 0x8000_0000][(round % 6) as usize];
        poke(a, b, stk + stack::DEPTH, 0i32);
        push_both(a, b, stk, stack::INT, slot);
        push_both(a, b, stk, stack::INT, word);
        let args = rng.next();
        r22_case(
            a,
            b,
            0x0087_8140,
            &[w, args, stk],
            true,
            &format!("Atel_SetVarSlot {what} round {round} word {word:#x} slot {slot:#x}"),
            |r| hp::atel_set_var_slot(r, w, stk),
        );
        calls += 1;
    }
    // 0x0086b270
    let out = alloc_both(a, b, 0x20);
    let player = a.rebase(0x0132_6b2c);
    for i in [0u32, 1, 2, 3, 5, n, u32::MAX] {
        let target = if i < n { i } else { 0 };
        poke(a, b, player, 0u32);
        if kind(b, ws[target as usize]) == 4 {
            continue;
        }
        for (x, y, z) in [
            (out, out + 4, out + 8),
            (0, out, 0),
            (out + 8, 0, out),
            (0, 0, 0),
        ] {
            poke(a, b, context, works + 0x238);
            r22_case(
                a,
                b,
                0x0086_b270,
                &[i, x, y, z],
                true,
                &format!("Atel_GetEventActorRot {what} {i:#x}"),
                |r| hp::atel_get_event_actor_rot(r, i, x, y, z),
            );
            calls += 1;
        }
    }
    // AtelGetGuideMapActorInfo
    let block_at = works + 0x48;
    if synthetic {
        let blk = alloc_both(a, b, 0x400);
        fill_both(a, b, rng, blk, 0x400);
        poke(a, b, blk + 0x2c, 0x40u32);
        poke(a, b, blk + 0x40 + 0x18, 0x100u32);
        poke(a, b, blk + 0x100, 6i32);
        for e in 0..6u32 {
            let id = [0u16, 1, 2, 3, 5, 6][e as usize];
            let marker = if e == 2 { 0x8000_1234 } else { rng.next() };
            poke(a, b, blk + 0x104 + e * 8, marker);
            poke(a, b, blk + 0x108 + e * 8, id);
        }
        poke(a, b, block_at, blk);
        for (i, &w) in (0u32..).zip(&ws) {
            // the position worker is the next one (not the table's)
            poke(a, b, w + 0x2e, u16::try_from((i + 1) % n).unwrap());
            if i == 3 {
                poke(a, b, w + 0xbc, 0x8000_0042u32);
            }
        }
    }
    let info = alloc_both(a, b, 0x40);
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    for round in 0..24u32 {
        let flags = Mem::read::<u8>(b, works);
        poke(
            a,
            b,
            works,
            if round % 8 == 7 {
                flags & !1
            } else {
                flags | 1
            },
        );
        let f2 = Mem::read::<u8>(b, works + 2);
        poke(
            a,
            b,
            works + 2,
            if round % 2 == 0 { f2 | 0x80 } else { f2 & 0x7f },
        );
        let i = [0u32, 1, 2, 3, 4, 5, 6, 40, u32::MAX][(round % 9) as usize];
        let o = if round % 11 == 10 { 0 } else { info };
        // the table's worker must have a position block
        poke(a, b, context, works);
        let blk = Mem::read::<u32>(b, block_at);
        if blk != 0 && o != 0 {
            let id = hp::atel_get_bin_hdr_table18_b(&Run(b), blk, i);
            let w = v::atel_get_basic_work(b, id.cast_signed());
            orig(a, ATEL_GET_BASIC_WORK, &[id]);
            let k = u32::from(Mem::read::<u16>(b, w + 0x2e));
            let kw = v::atel_get_basic_work(b, k.cast_signed());
            orig(a, ATEL_GET_BASIC_WORK, &[k]);
            if kind(b, w) == 4 || kind(b, kw) == 4 {
                continue;
            }
        }
        fill_both(a, b, rng, info, 0x40);
        poke(a, b, context, works + 0x238);
        r22_case(
            a,
            b,
            0x0086_b740,
            &[i, o],
            true,
            &format!("AtelGetGuideMapActorInfo {what} round {round} i {i:#x}"),
            |r| {
                let f = frame(r);
                hp::atel_get_guide_map_actor_info(r, i, o, f)
            },
        );
        calls += 1;
    }
    calls
}

/// Round 24 c (this machine): destroyMoviePicture, `0x0076ed10`,
/// `0x008b3d20`, `0x00871780`, `0x00878140`, `0x0086b270` and
/// `AtelGetGuideMapActorInfo` on the image (a synthetic ctrl work 0) and
/// on the snapshot (its own ctrl work 0).
#[test]
fn round24c_movie_menu_seps_markers_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0246);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r24c_stubbed(&mut a, &mut b, &mut rng, "image");
        let works = a.rebase(0x0132_5ba0);
        r23d_ctrl(&mut a, &mut b, &mut rng, works, 8);
        let n = r24c_workers(&mut a, &mut b, &mut rng, true, "image");
        assert!(n > 0);
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r24c_stubbed(&mut a, &mut b, &mut rng, "snapshot");
    let n = r24c_workers(&mut a, &mut b, &mut rng, false, "snapshot");
    eprintln!("round 24c: {n} snapshot worker calls");
}

/// `f` replaced in both oracles by a stand-in that logs `f` and its first
/// `n` stack words (as [`r24_log_stubs`]) and returns `values` in turn
/// (then whatever follows them: zeros).
fn r24_seq_stub(a: &mut Oracle, b: &mut Oracle, f: u32, n: u8, values: &[u32]) {
    let log = alloc_both(a, b, 0x8000);
    let cell = alloc_both(a, b, 4);
    poke(a, b, cell, log);
    let seq = alloc_both(a, b, 4 * values.len() + 0x100);
    for (k, &v) in (0u32..).zip(values) {
        poke(a, b, seq + 4 * k, v);
    }
    let vcell = alloc_both(a, b, 4);
    poke(a, b, vcell, seq);
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &cell.to_le_bytes()].concat();
    code.extend([&[0xc7, 0x01][..], &f.to_le_bytes()].concat());
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 + 4 * k]);
    }
    code.extend([0x83, 0xc1, 4 + 4 * n]);
    code.extend([&[0x89, 0x0d][..], &cell.to_le_bytes()].concat());
    code.extend([&[0x8b, 0x15][..], &vcell.to_le_bytes()].concat()); // mov edx, [vcell]
    code.extend([0x8b, 0x02, 0x83, 0xc2, 0x04]); // mov eax, [edx]; add edx, 4
    code.extend([&[0x89, 0x15][..], &vcell.to_le_bytes(), &[0xc3]].concat()); // mov [vcell], edx; ret
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
}

/// A separates block for `0x00866520`'s callers: a script block whose
/// `+0x2c` -> `+0xc` table lists `n` random `(id, flags)` pairs.
fn r24_sep_block(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: i32) -> u32 {
    let blk = alloc_both(a, b, 0x200);
    fill_both(a, b, rng, blk, 0x40);
    poke(a, b, blk + 0x2c, 0x40u32);
    poke(a, b, blk + 0x40 + 0xc, 0x80u32);
    poke(a, b, blk + 0x80, n);
    for i in 0..12u32 {
        poke(a, b, blk + 0x84 + i * 8, rng.below(12));
        poke(a, b, blk + 0x88 + i * 8, rng.next() & 0x7f_000f);
    }
    blk
}

/// Round 24 d on one pair, the stand-in part: the separate wrappers, the
/// menu entry, the vibration volume, the camera variable getters (their
/// callees logged); `ws` are workers of the current ctrl work.
#[allow(clippy::too_many_lines)] // one block per helper
fn r24d_stubbed(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, ws: &[u32], what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let works = a.rebase(0x0132_5ba0);
    let context = a.rebase(0x0132_6b28);
    let sync = r24_countdown_stub(a, b, 0x0088_7910);
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_7f00, 1),
            (0x0088_7b80, 0),
            (0x0086_38c0, 1),
            (0x0088_6f30, 2),
            (0x0088_6f00, 2),
            (0x0088_7780, 0),
            (0x0088_6dc0, 8),
            (0x0088_6db0, 0),
            (0x0091_9d70, 1),
            (0x0087_5f60, 2),
            (0x0078_8b90, 0),
            (0x008c_a540, 0),
            (0x0087_7460, 1),
            (0x0087_7450, 1),
            (0x007b_a780, 3),
            (0x007b_d420, 5),
            (0x007b_d4b0, 5),
        ],
    );
    let data = alloc_both(a, b, 0x80);
    for round in 0..24u32 {
        poke(a, b, sync, rng.below(3));
        poke(a, b, data, 0x4000_0000 | rng.below(0x100));
        for k in 1..12u32 {
            poke(a, b, data + 4 * k, rng.below(0x800));
        }
        let d = if round % 7 == 6 { 0 } else { data };
        let group = [u32::MAX, 0, 1, 2, 5, 0x8000_0001][(round % 6) as usize];
        let hdr = rng.next();
        poke(a, b, context, works + 0x238 * (round % 3));
        r22_case(
            a,
            b,
            0x0087_1750,
            &[hdr, d, group],
            true,
            &format!("AtelSndTransSepSPKDataGroup {what} {round}"),
            |r| hp::atel_snd_trans_sep_spk_data_group(r, hdr, d, group),
        );
        let sep_only = [0u32, 1, 2][(round % 3) as usize];
        poke(a, b, works + 0x204, rng.next());
        r22_case(
            a,
            b,
            0x0086_6430,
            &[hdr, d, sep_only],
            true,
            &format!("AtelEventSetUpSep {what} {round}"),
            |r| hp::atel_event_set_up_sep(r, hdr, d, sep_only),
        );
        // AtelEventRemoveSep: a block or none
        let blk = if round % 5 == 4 {
            0
        } else {
            r24_sep_block(a, b, rng, [0, 3, 12][(round % 3) as usize])
        };
        poke(a, b, works + 0x48, blk);
        poke(a, b, context, works + 0x238);
        let mode = round % 2;
        r22_case(
            a,
            b,
            0x0086_61d0,
            &[mode],
            true,
            &format!("AtelEventRemoveSep {what} {round}"),
            |r| hp::atel_event_remove_sep(r, mode),
        );
    }
    r22_case(
        a,
        b,
        0x0086_55e0,
        &[],
        true,
        &format!("AtelEnterMenuCallback {what}"),
        |r| hp::atel_enter_menu_callback(r),
    );
    // the vibration volume: pad states and channel lengths near the ends
    let levels: Vec<u32> = (0..0x400).map(|_| rng.below(0x300)).collect();
    r24_seq_stub(a, b, 0x0087_5c20, 2, &levels);
    let pads = a.rebase(0x0132_8150);
    for round in 0..40u32 {
        poke(
            a,
            b,
            a.rebase(0x0132_8272),
            [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2][(round % 20) as usize],
        );
        poke(
            a,
            b,
            a.rebase(0x0132_8274),
            u16::try_from(rng.below(0x200)).unwrap(),
        );
        for p in 0..8u32 {
            let rec = pads + p * 0x24;
            poke(a, b, rec + 0x22, u8::try_from(rng.below(4)).unwrap());
            for ch in [6u32, 0x16] {
                let len = [0x200u16, 0xfffe, 0x1ff, 0][rng.below(4) as usize];
                poke(a, b, rec + ch, len);
            }
        }
        r22_case(
            a,
            b,
            0x0087_5b40,
            &[],
            // off: the original leaves its caller's EAX
            round % 10 != 9,
            &format!("Vibrate_UpdateVolume {what} {round}"),
            |r| hp::vibrate_update_volume(r),
        );
    }
    // the camera variable getters
    let stk = alloc_both(a, b, 0x400);
    // the event map camera: ctrl work 0's block +0x18 (the first worker's
    // id in half the rounds)
    let blk = alloc_both(a, b, 0x40);
    poke(a, b, works + 0x48, blk);
    for (round, &w) in (0u32..).cycle().zip(ws.iter().cycle()).take(24) {
        if Mem::read::<u8>(b, Mem::read::<u32>(b, w)) == 4 {
            continue;
        }
        poke(a, b, context, works);
        poke(a, b, stk + stack::DEPTH, 0i32);
        for _ in 0..3 {
            let word = ([0u32, 1, 5][rng.below(3) as usize] << 25) | rng.below(0x40);
            push_both(a, b, stk, stack::INT, word);
        }
        // the camera word: set, or 0 to look the camera up
        let cam = w + worker::CAMERA;
        poke(a, b, cam, if round % 2 == 0 { 0 } else { rng.next() });
        let id = Mem::read::<u16>(b, w + worker::ID);
        poke(a, b, blk + 0x18, if round % 4 < 2 { id } else { id ^ 1 });
        let side = round % 3;
        for (sym, get) in [(0x007b_a3c0u32, 0x007b_d420u32), (0x007b_a460, 0x007b_d4b0)] {
            poke(a, b, stk + stack::DEPTH, 3i32);
            let args = rng.next();
            r22_case(
                a,
                b,
                sym,
                &[w, args, stk, side],
                true,
                &format!("cam vars {sym:#x} {what} {round} id {id}"),
                |r| hp::atel_cam_get_vars(r, get, w, stk, side),
            );
        }
    }
}

/// Round 24 d on one pair: `0x0086f940` from every worker of the current
/// ctrl work to a few others (`ws`).
fn r24d_level_target(a: &mut Oracle, b: &mut Oracle, ws: &[u32], what: &str) -> u32 {
    use ffx_script::vm::bridge::helpers as hp;
    let n = u32::try_from(ws.len()).unwrap();
    let mut calls = 0;
    for (k, &w) in (0u32..).zip(ws) {
        // a type 4 worker has no position block (Atel_SetLevelVectors
        // reads through 0)
        if Mem::read::<u8>(b, Mem::read::<u32>(b, w)) == 4 {
            continue;
        }
        // a snapshot's unused workers hold the heap's fill in their level
        // blocks
        let t = Mem::read::<u8>(b, Mem::read::<u32>(b, w));
        let l = u32::from(Mem::read::<u8>(b, w + 0x32)).min(if t == 5 || t == 6 { 1 } else { 8 });
        if Mem::read::<u32>(b, w + 0x170 + l * 0x4c) == 0xcdcd_cdcd {
            continue;
        }
        for i in [0, k, (k + 1) % n, n, u32::MAX] {
            let target = ws[if i < n { i as usize } else { 0 }];
            if Mem::read::<u8>(b, Mem::read::<u32>(b, target)) == 4 {
                continue;
            }
            r22_case(
                a,
                b,
                0x0086_f940,
                &[w, i],
                true,
                &format!("Atel_SetLevelTarget {what} worker {k} -> {i:#x}"),
                |r| hp::atel_set_level_target(r, w, i),
            );
            calls += 1;
        }
    }
    calls
}

/// Round 24 d (this machine): the separate wrappers, the menu entry, the
/// vibration volume, the camera variable getters and `0x0086f940` on the
/// image (a synthetic ctrl work 0) and the snapshot (ctrl work 0 as it
/// stands); `0x0086e8d0` on every ctrl work in use of the snapshot (the
/// script runs' set-up: signals need the snapshot's free nodes), with
/// battle-end tables of the test's own.
#[test]
#[allow(clippy::too_many_lines)] // one block per part
fn round24d_seps_menu_vibrate_cam_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0247);
    let workers = |a: &mut Oracle, b: &mut Oracle| -> Vec<u32> {
        let works = a.rebase(0x0132_5ba0);
        let context = a.rebase(0x0132_6b28);
        poke(a, b, context, works);
        let n = u32::from(Mem::read::<u16>(b, works + ctrl::WORKERS));
        (0..n)
            .map(|i| {
                orig(a, ATEL_GET_BASIC_WORK, &[i]);
                v::atel_get_basic_work(b, i.cast_signed())
            })
            .collect()
    };
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        let works = a.rebase(0x0132_5ba0);
        r23d_ctrl(&mut a, &mut b, &mut rng, works, 8);
        let ws = workers(&mut a, &mut b);
        // turn blocks of their own for the level helper
        for &w in &ws {
            for l in 0..9u32 {
                let blk = alloc_both(&mut a, &mut b, 0x40);
                poke(&mut a, &mut b, w + 0x170 + l * 0x4c, blk);
            }
        }
        assert!(r24d_level_target(&mut a, &mut b, &ws, "image") > 0);
        r24d_stubbed(&mut a, &mut b, &mut rng, &ws, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut signalled = 0;
    for &c in &ctrls {
        let (mut a, mut b) = fresh_run_pair(&pages, &exe, c);
        let n = u32::from(Mem::read::<u16>(&b, c + ctrl::WORKERS)).min(12);
        let blk = Mem::read::<u32>(&b, c + 0x48);
        if blk == 0 {
            continue;
        }
        let save = Mem::read::<u32>(&b, blk + 0x24);
        // the table: 6 entries of tags 1..3, workers inside, one 0xffff
        let tab = alloc_both(&mut a, &mut b, 0x80);
        poke(&mut a, &mut b, tab, 6u32);
        for e in 0..6u32 {
            let worker = if e == 4 {
                0xffff
            } else {
                u16::try_from(e % n.max(1)).unwrap()
            };
            poke(&mut a, &mut b, tab + 4 + e * 16, 1 + e % 3);
            poke(&mut a, &mut b, tab + 8 + e * 16, worker);
            poke(
                &mut a,
                &mut b,
                tab + 10 + e * 16,
                u16::try_from(1 + e % 2).unwrap(),
            );
        }
        // AtelGetBattleEndTagAlways: g_SaveData +0xe8
        let always = a.rebase(0x0112_ca90 + 0xe8);
        let was = Mem::read::<u16>(&b, always);
        for (k, tag) in [1u32, 2, 3, 9, 1, 9].into_iter().enumerate() {
            // the table (relative to the block), none, the snapshot's own;
            // the last with every entry (the save's "always")
            let t = match k {
                3 => 0,
                4 => save,
                _ => tab.wrapping_sub(blk),
            };
            poke(&mut a, &mut b, blk + 0x24, t);
            poke(&mut a, &mut b, always, u16::from(k == 5));
            let what = format!("Atel_RequestBattleEndEntries ctrl {c:#x} tag {tag} table {t:#x}");
            let mut p = (a, b);
            if r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0086_e8d0,
                &[tag],
                true,
                &what,
                |r| hp::atel_request_battle_end_entries(r, tag),
            ) {
                signalled += 1;
            }
            (a, b) = p;
        }
        poke(&mut a, &mut b, blk + 0x24, save);
        poke(&mut a, &mut b, always, was);
        // no script block
        poke(&mut a, &mut b, c + 0x48, 0u32);
        let mut p = (a, b);
        r19_case(
            &pages,
            &exe,
            c,
            &mut p,
            0x0086_e8d0,
            &[1],
            true,
            "Atel_RequestBattleEndEntries no block",
            |r| hp::atel_request_battle_end_entries(r, 1),
        );
    }
    eprintln!("round 24d: {signalled} battle-end calls");
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    let ws = workers(&mut a, &mut b);
    eprintln!("round 24d: {} snapshot workers", ws.len());
    r24d_level_target(&mut a, &mut b, &ws, "snapshot");
    r24d_stubbed(&mut a, &mut b, &mut rng, &ws, "snapshot");
}

/// Round 24 e on one pair: the save-load helpers, the movie state, the
/// battle-end effect queue, the separate fade-out, the water effects and
/// the save event bits (their sound, menu and effect callees logged).
#[allow(clippy::too_many_lines)] // one block per helper
fn r24e_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let wins: Vec<u32> = (0..8)
        .map(|_| {
            let w = alloc_both(a, b, 0x80);
            fill_both(a, b, rng, w, 0x80);
            w
        })
        .collect();
    r24_seq_stub(a, b, 0x008a_a1a0, 0, &wins);
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_8480, 2),
            (0x008a_ab00, 1),
            (0x0088_6f90, 2),
            (0x0072_9570, 1),
            (0x0072_9700, 1),
            (0x0072_95b0, 1),
        ],
    );
    for slot in [0u32, 1, 3] {
        r22_case(
            a,
            b,
            0x008b_3cd0,
            &[slot],
            true,
            &format!("TkMenuSaveLoad_ClearLoadPos {what} {slot}"),
            |r| hp::tk_menu_save_load_clear_load_pos(r, slot),
        );
    }
    for k in 0..3u32 {
        r22_case(
            a,
            b,
            0x008b_3c20,
            &[],
            true,
            &format!("TkMenuSaveLoad_OpenWindow {what} {k}"),
            |r| hp::tk_menu_save_load_open_window(r),
        );
        let arg = rng.next();
        r22_case(
            a,
            b,
            0x008b_3c70,
            &[arg],
            true,
            &format!("TkMenuSaveLoad_OpenChoice {what} {k}"),
            |r| hp::tk_menu_save_load_open_choice(r, arg),
        );
    }
    let bufs = a.rebase(0x0132_7110);
    fill_both(a, b, rng, bufs, 12);
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    fill_both(a, b, rng, save + 0xdc, 12);
    r22_case(
        a,
        b,
        0x0087_3c00,
        &[],
        true,
        &format!("Atel_InitMovieState {what}"),
        |r| hp::atel_init_movie_state(r),
    );
    for (k, kind) in [0x121u32, 0x120, 0, 0x8000_0121, 0x121, 7]
        .into_iter()
        .enumerate()
    {
        poke(a, b, a.rebase(0x0132_6b58), 0u32);
        let v = rng.next();
        r22_case(
            a,
            b,
            0x0086_1550,
            &[kind, v],
            true,
            &format!("Atel_QueueBattleEndEffect {what} {k}"),
            |r| hp::atel_queue_battle_end_effect(r, kind, v),
        );
    }
    let count = a.rebase(0x0132_707c);
    for n in [-1i32, 0, 1, 3, 7] {
        fill_both(a, b, rng, count + 4, 0x40);
        poke(a, b, count, n);
        r22_case(
            a,
            b,
            0x0086_8460,
            &[],
            n > 0,
            &format!("Atel_FadeOutQueuedSeps {what} {n}"),
            |r| hp::atel_fade_out_queued_seps(r),
        );
    }
    let ids = a.rebase(0x0132_6cf0);
    for round in 0..12u32 {
        let mut any = false;
        for k in 0..4u32 {
            let id = if rng.below(3) == 0 {
                u32::MAX - rng.below(3)
            } else {
                rng.below(0x40)
            };
            any |= id.cast_signed() >= 0;
            poke(a, b, ids + k * 4, id);
        }
        let on = [0u32, 1, 0x100][(round % 3) as usize];
        r22_case(
            a,
            b,
            0x0086_82a0,
            &[on],
            any,
            &format!("Atel_ShowWaterFps {what} {round}"),
            |r| hp::atel_show_water_fps(r, on),
        );
    }
    for n in [-40i32, -33, -32, -1, 0, 5, 31, 32, 63, 64, 100] {
        let n = n.cast_unsigned();
        r22_case(
            a,
            b,
            0x0087_00e0,
            &[n],
            true,
            &format!("Atel_SetSaveEventBit {what} {n:#x}"),
            |r| hp::atel_set_save_event_bit(r, n),
        );
    }
}

/// Round 24 e (this machine): the helpers of [`r24e_cases`] on the image
/// and the snapshot, then `0x008764a0` on every ctrl work in use (the
/// script runs' set-up, the owner lookup stubbed).
#[test]
fn round24e_save_movie_effects_match_the_original() {
    use ffx_script::vm::bridge::helpers as hp;
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0248);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r24e_cases(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    {
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        r24e_cases(&mut a, &mut b, &mut rng, "snapshot");
    }
    let ctrls = in_use(&Oracle::from_snapshot(&exe, &pages).unwrap());
    let mut calls = 0;
    for &c in &ctrls {
        let mut p = fresh_run_pair(&pages, &exe, c);
        // the real owner lookup (it scans ctrl work 0): a bound worker's
        // owner, and one nobody has
        let works = p.0.rebase(0x0132_5ba0);
        let n0 = u32::from(Mem::read::<u16>(&p.1, works + 0xc));
        let mut owners = vec![0x1234_5678u32];
        let context = p.0.rebase(0x0132_6b28);
        let here = Mem::read::<u32>(&p.1, context);
        poke(&mut p.0, &mut p.1, context, works);
        for i in 0..n0 {
            let w = v::atel_get_basic_work(&mut p.1, i.cast_signed());
            orig(&mut p.0, ATEL_GET_BASIC_WORK, &[i]);
            if Mem::read::<u8>(&p.1, w + 0xaa) == 1 {
                owners.push(Mem::read::<u32>(&p.1, w + 0x9c));
            }
        }
        poke(&mut p.0, &mut p.1, context, here);
        for &owner in owners.iter().take(4) {
            let what = format!("Atel_RequestOwnerTrigger ctrl {c:#x} owner {owner:#x}");
            if r19_case(
                &pages,
                &exe,
                c,
                &mut p,
                0x0087_64a0,
                &[owner, 1],
                true,
                &what,
                |r| hp::atel_request_owner_trigger(r, owner, 1),
            ) {
                calls += 1;
            }
        }
        for found in [u32::MAX, 0, 1, 2, 3] {
            for o in [&mut p.0, &mut p.1] {
                let at = o.rebase(0x0087_6440);
                o.stub_return(at, found).unwrap();
            }
            for kind in [0u32, 1, 2] {
                let owner = rng.next();
                let what =
                    format!("Atel_RequestOwnerTrigger ctrl {c:#x} found {found:#x} kind {kind}");
                if r19_case(
                    &pages,
                    &exe,
                    c,
                    &mut p,
                    0x0087_64a0,
                    &[owner, kind],
                    true,
                    &what,
                    |r| hp::atel_request_owner_trigger(r, owner, kind),
                ) {
                    calls += 1;
                }
            }
        }
    }
    eprintln!("round 24e: {calls} owner trigger calls");
    assert!(calls > 0);
}

// ---- script round 25 ----

/// `f` in both oracles returns `v` (`stub_return`).
fn r25_ret(a: &mut Oracle, b: &mut Oracle, f: u32, v: u32) {
    for o in [a, b] {
        let at = o.rebase(f);
        o.stub_return(at, v).unwrap();
    }
}

/// Replaces `f` in both oracles with a stub that copies the string its
/// first argument points at (at most 64 bytes, its NUL included) into a
/// fresh buffer and returns `0x77`: what a callee handed a pointer to a
/// frame local saw (bytes after the NUL are frame leftovers). Returns
/// the buffer.
fn r25_copy_stub(a: &mut Oracle, b: &mut Oracle, f: u32) -> u32 {
    let buf = alloc_both(a, b, 0x40);
    let mut code = vec![0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0xbf]; // push esi; push edi; mov esi,[esp+0xc]; mov edi,
    code.extend(buf.to_le_bytes());
    // mov ecx,64; l: lodsb; stosb; test al,al; loopnz l; pop edi; pop esi
    code.extend([
        0xb9, 0x40, 0, 0, 0, 0xac, 0xaa, 0x84, 0xc0, 0xe0, 0xfa, 0x5f, 0x5e,
    ]);
    code.extend([0xb8, 0x77, 0, 0, 0, 0xc3]); // mov eax,0x77; ret
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    buf
}

/// `fiosUnifyFilename`'s stream prefix (none on the image) and its
/// overflow message stubbed, as in round 24 a.
fn r25_fios_prefix(a: &mut Oracle, b: &mut Oracle) {
    let prefix = a.rebase(0x00cc_9698);
    if Mem::read::<u32>(b, prefix) == 0 {
        let s = alloc_both(a, b, 0x10);
        for (k, &c) in (0u32..).zip(b"data/\0") {
            poke(a, b, s + k, c);
        }
        poke(a, b, prefix, s);
    }
    r25_ret(a, b, 0x0043_52f0, 0);
}

/// Replaces `f` in both oracles with `fld dword [cell]; ret`, the cell
/// holding `bits`; returns the cell (a float stand-in).
fn r25_float_stub(a: &mut Oracle, b: &mut Oracle, f: u32, bits: u32) -> u32 {
    let cell = alloc_both(a, b, 4);
    poke(a, b, cell, bits);
    let code: Vec<u8> = [&[0xd9, 0x05][..], &cell.to_le_bytes(), &[0xc3]].concat();
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    cell
}

/// An event header in both oracles whose separates table (`+0x2c` ->
/// `+0xc`) holds `n` entries with ids below 8 and random flags (bit 2
/// mostly set).
fn r25_sep_header(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, n: i32) -> u32 {
    let hdr = alloc_both(a, b, 0x400);
    poke(a, b, hdr + 0x2c, 0x100u32);
    poke(a, b, hdr + 0x10c, 0x200u32);
    poke(a, b, hdr + 0x200, n);
    for i in 0..n.clamp(0, 0x18).cast_unsigned() {
        let mut flags = rng.next() & 0x1ffe;
        if rng.below(4) != 0 {
            flags |= 2;
        }
        if rng.below(2) == 0 {
            flags &= !0x330;
        }
        poke(a, b, hdr + 0x204 + i * 8, rng.below(8));
        poke(a, b, hdr + 0x208 + i * 8, flags);
    }
    hdr
}

/// Round 25 a on one pair: the soft reset, the shape clear, the
/// background separates, the worker position, the battle set-up and the
/// footstep table (callees logged or stubbed).
#[allow(clippy::too_many_lines)] // one block per function
fn r25a_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    // AtelExecSoftWareReset
    let works = a.rebase(0x0132_5ba0);
    let mode = a.rebase(0x0132_7108);
    r25_ret(a, b, 0x0087_2550, 0x5050);
    for round in 0..64u32 {
        // every condition allowing the reset, then (but in the first
        // rounds) one of the nine broken, or everything random
        let broken = round / 4 % 11;
        for (off, keep) in [(0u32, 1u8), (3, 0x83), (0x210, 0x42)] {
            let r = rng.next().to_le_bytes()[0];
            let v = if broken == 10 { r } else { r & !keep };
            poke(a, b, works + off, v);
        }
        if broken != 10 {
            let f = Mem::read::<u8>(b, works);
            poke(a, b, works, f | u8::from(broken != 1));
            let f = Mem::read::<u8>(b, works + 0x210);
            let f = (f | 0x40) & !2;
            let f = match broken {
                5 => f | 2,
                7 => f & !0x40,
                _ => f,
            };
            poke(a, b, works + 0x210, f);
            let f = match broken {
                3 => 1 << (round % 2),
                4 => 0x80,
                _ => 0,
            };
            poke(a, b, works + 3, f);
        }
        poke(a, b, mode, [0u32, 1, 2, 0][(round % 4) as usize]);
        r25_ret(
            a,
            b,
            0x0086_d940,
            u32::from(broken == 2 || (broken == 10 && rng.below(2) == 0)),
        );
        let map = if broken == 6 {
            0x15c
        } else {
            rng.below(0x200) | 0x200
        };
        r25_ret(a, b, 0x0086_c1c0, map);
        r25_ret(a, b, 0x0088_7b20, u32::from(broken == 8));
        r25_ret(a, b, 0x0088_7b10, u32::from(broken == 9));
        r25_ret(a, b, 0x0087_2350, rng.below(2) * 7);
        r22_case(
            a,
            b,
            0x0087_38d0,
            &[],
            true,
            &format!("AtelExecSoftWareReset {what} {round}"),
            |r| hp::atel_exec_soft_ware_reset(r),
        );
    }
    // AtelClearShape
    r24_log_stubs(a, b, &[(0x00a4_46a0, 1), (0x0064_5280, 2)]);
    let seen = r25_copy_stub(a, b, 0x0063_9330);
    let sprites = a.rebase(0x0132_7148);
    let lang = r24_lang_word(a, b);
    r25_fios_prefix(a, b);
    for (k, loaded) in [0u32, 1, 7].into_iter().enumerate() {
        fill_both(a, b, rng, sprites, 0x80 * 0x1c);
        fill_both(a, b, rng, seen, 0x40);
        poke(a, b, lang, rng.below(13));
        poke(a, b, a.rebase(0x0132_7140), loaded);
        fill_both(a, b, rng, a.rebase(0x0132_7144), 4);
        fill_both(a, b, rng, a.rebase(0x0132_7f48), 8);
        fill_both(a, b, rng, a.rebase(0x023c_bc5c), 4);
        r22_case(
            a,
            b,
            0x0087_42b0,
            &[],
            true,
            &format!("AtelClearShape {what} {k}"),
            |r| {
                let frame = r.0.stack_range().start + 0x100;
                hp::atel_clear_shape(r, frame)
            },
        );
    }
    // Atel_TranslateBGSeps, AtelEventKickBGSep
    r24_log_stubs(
        a,
        b,
        &[(0x0088_6ff0, 3), (0x0088_6e50, 3), (0x0088_6f90, 2)],
    );
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    let hdr_cell = works + 0x48;
    let old_hdr = Mem::read::<u32>(b, hdr_cell);
    let list = a.rebase(0x0132_7030);
    let loads = a.rebase(0x0132_5b78);
    let queue = a.rebase(0x0132_707c);
    for round in 0..24u32 {
        let n = [0i32, -2, 1, 5, 12, 24][(round % 6) as usize];
        let hdr = r25_sep_header(a, b, rng, n);
        poke(a, b, save + 0x28, u8::from(rng.below(2) == 0));
        let f = Mem::read::<u8>(b, works);
        poke(a, b, works, (f & !1) | u8::from(round % 5 != 0));
        poke(a, b, hdr_cell, hdr);
        let volume = rng.next();
        let frames = [0u32, 1, 0x7f, 0x100, 0x8000_0001, rng.next()][(round % 6) as usize];
        let all = round % 3;
        r22_case(
            a,
            b,
            0x0086_1020,
            &[volume, frames, all],
            true,
            &format!("Atel_TranslateBGSeps {what} {round}"),
            |r| hp::atel_translate_bg_seps(r, volume, frames, all),
        );
        poke(
            a,
            b,
            loads,
            rng.below(9).cast_signed() - i32::from(round % 7 == 0),
        );
        for k in 0..8u32 {
            poke(a, b, list + k * 8, rng.below(8));
            poke(a, b, list + k * 8 + 4, rng.next() & 0x8000);
        }
        poke(a, b, queue, rng.below(4).cast_signed());
        fill_both(a, b, rng, queue + 4, 0x40);
        let fade = [0u32, 1, 0x100][(round % 3) as usize];
        r22_case(
            a,
            b,
            0x0086_58d0,
            &[hdr, fade],
            false,
            &format!("AtelEventKickBGSep {what} {round}"),
            |r| {
                hp::atel_event_kick_bg_sep(r, hdr, fade);
                0
            },
        );
    }
    poke(a, b, hdr_cell, old_hdr);
    // AtelEventRegistFootInfo
    r24_log_stubs(a, b, &[(0x0088_70e0, 4)]);
    for round in 0..8u32 {
        let hdr = alloc_both(a, b, 0x1000);
        poke(a, b, hdr + 0x2c, if round == 0 { 0 } else { 0x100u32 });
        // what reads at the header itself would find: tables at +0x1c..
        // and offsets at +0 to a count of 1 at +0x800
        for off in [0x1cu32, 0x20, 0x30, 0x34] {
            poke(a, b, hdr + off, 0x200u32);
        }
        for party in 0..16u32 {
            poke(a, b, hdr + party * 2, 0x800u16);
        }
        poke(a, b, hdr + 0x800, 1i32);
        fill_both(a, b, rng, hdr + 0x804, 0x10);
        for (k, off) in [0x1cu32, 0x20, 0x30, 0x34].into_iter().enumerate() {
            let tab = 0x200 + 0x380 * u32::try_from(k).unwrap();
            poke(
                a,
                b,
                hdr + 0x100 + off,
                if rng.below(5) == 0 { 0 } else { tab },
            );
            for party in 0..16u32 {
                let at = 0x40 + party * 0x30;
                let e = if rng.below(4) == 0 {
                    0u16
                } else {
                    u16::try_from(at).unwrap()
                };
                poke(a, b, hdr + tab + party * 2, e);
                let n = [0i32, 1, 2, -1][rng.below(4) as usize];
                poke(a, b, hdr + tab + at, n);
                fill_both(a, b, rng, hdr + tab + at + 4, 0x20);
            }
        }
        r22_case(
            a,
            b,
            0x0086_5b80,
            &[hdr],
            true,
            &format!("AtelEventRegistFootInfo {what} {round}"),
            |r| hp::atel_event_regist_foot_info(r, hdr),
        );
    }
}

/// Round 25 a: `Atel_SetWorkerPosXZ_00870970` on synthetic workers and a
/// synthetic ctrl work (its character callees logged).
fn r25a_worker_pos(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0082_b440, 3),
            (0x0082_4650, 1),
            (0x0083_d820, 1),
            (0x0083_d7a0, 1),
        ],
    );
    let ctx_cell = a.rebase(0x0132_6b28);
    let old = Mem::read::<u32>(b, ctx_cell);
    let ctx = alloc_both(a, b, 0x240);
    fill_both(a, b, rng, ctx, 0x240);
    poke(a, b, ctx_cell, ctx);
    let floats = [
        0f32,
        1.5,
        -3.25,
        1e30,
        -0.0,
        f32::from_bits(0x7fc0_1234),
        7e-40,
    ];
    for round in 0..40u32 {
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w, 0x600);
        let kind = alloc_both(a, b, 4);
        poke(a, b, kind, [5u8, 6, 0, 7, 0xff][(round % 5) as usize]);
        poke(a, b, w, kind);
        poke(a, b, w + 0xaa, u8::from(rng.below(3) != 0));
        poke(
            a,
            b,
            w + 0x9c,
            if rng.below(4) == 0 { 0 } else { rng.next() | 1 },
        );
        let player = Mem::read::<u16>(b, ctx + 0xa);
        if rng.below(2) == 0 {
            poke(a, b, w + 0x2e, player);
        }
        let x = floats[rng.below(7) as usize].to_bits();
        let z = floats[rng.below(7) as usize].to_bits();
        let keep = rng.below(2) * (1 + rng.below(3));
        r22_case(
            a,
            b,
            0x0087_0970,
            &[w, x, z, keep],
            false,
            &format!("Atel_SetWorkerPosXZ {what} {round}"),
            |r| {
                hp::atel_set_worker_pos_xz(r, w, x, z, keep);
                0
            },
        );
    }
    poke(a, b, ctx_cell, old);
}

/// Round 25 a: `AtelExecCtrlEventInit_008677a0` on the current ctrl work
/// (`Script_Run`, the pad, the signals and the effect init stubbed), each
/// round with other workers already initialised.
fn r25a_event_init(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0078_1530, 0),
            (0x0086_41e0, 3),
            (0x0086_53f0, 2),
            (0x007f_cc20, 0),
        ],
    );
    let (c, _) = ctx(b);
    let n = u32::from(Mem::read::<u16>(b, c + 0xc));
    let pressed = a.rebase(0x0132_70d0);
    let level = a.rebase(0x0132_6b30);
    for round in 0..16u32 {
        for i in 0..n {
            let w = v::atel_get_basic_work(b, i.cast_signed());
            let f = Mem::read::<u8>(b, w + 0x34);
            let done = round % 4 == 3 || rng.below(4) != 0;
            poke(a, b, w + 0x34, if done { f | 0x10 } else { f & !0x10 });
        }
        // the eight message windows of bank 0: open or not, busy or not
        poke(a, b, a.rebase(0x0132_6b82), 0u8);
        for k in 0..8u32 {
            let win = a.rebase(0x0132_6d84) + k * 0x2c;
            let open = k > 0 || rng.below(4) != 0;
            poke(
                a,
                b,
                win,
                if open {
                    1u16 + u16::from(rng.below(2) == 0)
                } else {
                    0
                },
            );
            poke(a, b, win + 2, u8::from(k > 0 && rng.below(3) == 0));
            poke(
                a,
                b,
                win + 9,
                [0u8, 0x10, 0x22, 0x02][rng.below(4) as usize],
            );
        }
        let pads: Vec<u32> = (0..2).map(|_| rng.next() & 0x0003_00ff).collect();
        r24_seq_stub(a, b, 0x0088_8c40, 3, &pads);
        poke(a, b, pressed, rng.next().to_le_bytes()[0]);
        fill_both(a, b, rng, a.rebase(0x0132_6b2c), 4);
        let f = Mem::read::<u8>(b, c);
        poke(a, b, c, f ^ (rng.next().to_le_bytes()[0] & 0xc));
        poke(a, b, level, rng.below(4).cast_signed() - 1);
        let battle = round % 2;
        r22_case(
            a,
            b,
            0x0086_77a0,
            &[c, battle],
            false,
            &format!("AtelExecCtrlEventInit {what} {round} ({n} workers)"),
            |r| {
                hp::atel_exec_ctrl_event_init(r, c, battle);
                0
            },
        );
    }
}

/// Round 25 a: `AtelEventSetUpForBattleMusic` with its callees logged,
/// ctrl work 0's bound workers given effects and type `0x5001`.
#[allow(clippy::too_many_lines)] // the set-up of each round
fn r25a_battle_music(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_7b50, 1),
            (0x0087_5f60, 2),
            (0x0088_6c40, 0),
            (0x0088_5e50, 0),
            (0x0090_9c60, 0),
            (0x0082_ab90, 1),
            (0x0082_abc0, 1),
            (0x0082_8b30, 1),
            (0x0072_9570, 1),
            (0x0091_9c60, 0),
            (0x0082_dad0, 1),
        ],
    );
    let works = a.rebase(0x0132_5ba0);
    let ctx_cell = a.rebase(0x0132_6b28);
    let here = Mem::read::<u32>(b, ctx_cell);
    let n = u32::from(Mem::read::<u16>(b, works + 0xc));
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    let others: Vec<u32> = in_use(b).into_iter().filter(|&c| c != works).collect();
    for round in 0..24u32 {
        poke(a, b, ctx_cell, works);
        let ws: Vec<u32> = (0..n)
            .map(|i| {
                orig(a, ATEL_GET_BASIC_WORK, &[i]);
                v::atel_get_basic_work(b, i.cast_signed())
            })
            .collect();
        for (i, &w) in ws.iter().enumerate() {
            // only the 0xb58-byte workers (the first class) have FP slots
            let big = ws.get(i + 1).is_some_and(|&next| next == w + 0xb58);
            if big && rng.below(3) == 0 {
                poke(a, b, w + 0xa8, 0x5001u16);
                fill_both(a, b, rng, w + 0xb18, 0x40);
                // a free slot, with and without bit 15
                poke(
                    a,
                    b,
                    w + 0xb18 + 0x10 * rng.below(4),
                    [0u16, 0x8000][rng.below(2) as usize],
                );
            }
            if rng.below(4) == 0 {
                poke(a, b, w + 0x9c, 0u32);
            }
            fill_both(a, b, rng, w + 0x38, 1);
        }
        // ctrl work 0 ends with +3 bit 0x80 set (no UC); another one may
        // get it back
        let c = match round % 4 {
            0 => others
                .get(round as usize / 4 % others.len().max(1))
                .copied()
                .unwrap_or(works + 0x238),
            2 => works,
            _ => here,
        };
        poke(a, b, ctx_cell, c);
        for off in [0u32, 1, 3] {
            fill_both(a, b, rng, c + off, 1);
            fill_both(a, b, rng, works + off, 1);
        }
        if round % 4 == 0 {
            // the UC comes back on, or misses one condition
            let miss = round / 4 % 6;
            let f = Mem::read::<u8>(b, c) & !0x22;
            poke(
                a,
                b,
                c,
                f | [0x20u8, 0x02, 0x22, 0x22, 0x22, 0][miss as usize],
            );
            let f = Mem::read::<u8>(b, c + 1) & !4;
            poke(a, b, c + 1, f | if miss == 2 { 4 } else { 0 });
            let f = Mem::read::<u8>(b, c + 3) & !0x84;
            poke(a, b, c + 3, f | [0u8, 0, 0, 0x04, 0x80, 0][miss as usize]);
        }
        fill_both(a, b, rng, a.rebase(0x0132_6b48), 4);
        fill_both(a, b, rng, a.rebase(0x0132_6b80), 1);
        fill_both(a, b, rng, save + 0xe4, 4);
        let st = a.rebase(0x0132_6b48);
        if round % 3 == 0 {
            let v = Mem::read::<u32>(b, st);
            poke(a, b, st, v | 0x1_0000);
            let v = Mem::read::<u32>(b, save + 0xe4);
            poke(a, b, save + 0xe4, v & !0x1_0000);
        }
        r25_ret(
            a,
            b,
            0x0078_2a10,
            if round % 3 == 0 { 0 } else { rng.below(3) },
        );
        r22_case(
            a,
            b,
            0x0086_6240,
            &[],
            false,
            &format!("AtelEventSetUpForBattleMusic {what} {round} ({n} workers)"),
            |r| {
                hp::atel_event_set_up_for_battle_music(r);
                0
            },
        );
    }
    poke(a, b, ctx_cell, here);
}

#[test]
fn round25a_reset_shapes_seps_setup_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0251);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r25a_cases(&mut a, &mut b, &mut rng, "image");
        r25a_worker_pos(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let pair = || {
        (
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        )
    };
    let (mut a, mut b) = pair();
    r25a_cases(&mut a, &mut b, &mut rng, "snapshot");
    r25a_worker_pos(&mut a, &mut b, &mut rng, "snapshot");
    let (mut a, mut b) = pair();
    r25a_event_init(&mut a, &mut b, &mut rng, "snapshot");
    let (mut a, mut b) = pair();
    r25a_battle_music(&mut a, &mut b, &mut rng, "snapshot");
}

/// Floats for the round 25 cases: zeros, halves, large, tiny, a quiet
/// NaN, infinities.
const R25_FLOATS: [u32; 14] = [
    0,
    0x8000_0000,
    0x3f00_0000,
    0xbfc0_0000,
    0x4f00_0000,
    0xcf00_0001,
    0x4700_0000,
    0x0000_0001,
    0x7fc0_1234,
    0x7f80_0000,
    0xff80_0000,
    0x4049_0fdb,
    0x4b00_0001,
    0xcb00_0003,
];

/// Round 25 b on one pair: the small helpers (callees logged or stubbed).
#[allow(clippy::too_many_lines)] // one block per function
fn r25b_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_8480, 2),
            (0x0088_9490, 3),
            (0x0088_94b0, 3),
            (0x0082_aea0, 3),
            (0x0082_b190, 2),
        ],
    );
    r22_case(
        a,
        b,
        0x008b_3cf0,
        &[],
        true,
        &format!("AtelAllClearCallbackQue {what}"),
        |r| hp::atel_all_clear_callback_que(r),
    );
    // AtelExecVibrate
    let vib = a.rebase(0x0132_8270);
    for round in 0..12u32 {
        fill_both(a, b, rng, vib, 2);
        poke(a, b, vib + 2, u8::from(round % 3 == 0));
        let update = [0u32, 1, 2, 3][(round % 4) as usize];
        r22_case(
            a,
            b,
            0x0087_39a0,
            &[update],
            true,
            &format!("AtelExecVibrate {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_exec_vibrate(r, update, f)
            },
        );
    }
    // Atel_SetChrAltParts
    for (k, (chr, on)) in [
        (0u32, 1u32),
        (0x1234, 0),
        (0x5678, 1),
        (0x9abc, 0x100),
        (0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        r22_case(
            a,
            b,
            0x0087_20e0,
            &[chr, on],
            false,
            &format!("Atel_SetChrAltParts {what} {k}"),
            |r| {
                hp::atel_set_chr_alt_parts(r, chr, on);
                0
            },
        );
    }
    // Atel_ApplyChrDir, Atel_SyncChrRot
    let rot = r25_float_stub(a, b, 0x0082_ace0, 0);
    let fcall = |r: &mut Run, f: u32, args: &[u32]| {
        let at = r.0.rebase(f);
        r.0.call_f64(at, args).unwrap()
    };
    for round in 0..30u32 {
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w, 0x600);
        let kind = alloc_both(a, b, 4);
        poke(a, b, kind, [5u8, 6, 0, 7, 0xff][(round % 5) as usize]);
        poke(a, b, w, kind);
        poke(a, b, w + 0xaa, [1u8, 1, 0, 2][rng.below(4) as usize]);
        poke(
            a,
            b,
            w + 0x9c,
            if rng.below(4) == 0 { 0 } else { rng.next() | 1 },
        );
        for off in [0x284u32, 0x558] {
            poke(a, b, w + off + 0x20, R25_FLOATS[rng.below(12) as usize]);
        }
        // Math_WrapAngle steps by 2 pi: no infinities or huge angles
        let angles = [
            0u32,
            0x8000_0000,
            0x3f00_0000,
            0xbfc0_0000,
            0x42c8_0000,
            0x40e0_0000,
            0x4049_0fdb,
            0xc1a0_0000,
            0x7fc0_1234,
        ];
        poke(a, b, rot, angles[rng.below(9) as usize]);
        r22_case(
            a,
            b,
            0x0087_0360,
            &[w],
            false,
            &format!("Atel_ApplyChrDir {what} {round}"),
            |r| {
                hp::atel_apply_chr_dir(r, w);
                0
            },
        );
        r22_case(
            a,
            b,
            0x0086_9f90,
            &[w],
            false,
            &format!("Atel_SyncChrRot {what} {round}"),
            |r| {
                hp::atel_sync_chr_rot(r, w, fcall);
                0
            },
        );
    }
    // Atel_SetSpriteScale
    for (k, i) in [
        0u32,
        5,
        0x7f,
        0x80,
        0xffff_ffff,
        0x8000_0000,
        0x7fff_ffff,
        64,
    ]
    .into_iter()
    .enumerate()
    {
        for _ in 0..4 {
            let x = R25_FLOATS[rng.below(14) as usize];
            let y = R25_FLOATS[rng.below(14) as usize];
            r22_case(
                a,
                b,
                0x0087_4a80,
                &[i, x, y],
                false,
                &format!("Atel_SetSpriteScale {what} {k} {x:#x} {y:#x}"),
                |r| {
                    hp::atel_set_sprite_scale(r, i, x, y);
                    0
                },
            );
        }
    }
    // Atel_SaveLights, Atel_RestoreLights
    // the light setters reach the scene, which the image lacks; the
    // setters logged (their result is the renderer's)
    r24_log_stubs(a, b, &[(0x0083_6310, 2), (0x0083_6430, 2)]);
    let saved = a.rebase(0x0132_6d00);
    for k in 0..if what == "image" { 0 } else { 3 } {
        fill_both(a, b, rng, saved, 0x70);
        for j in 0..0x1c {
            poke(a, b, saved + j * 4, R25_FLOATS[rng.below(8) as usize]);
        }
        r22_case(
            a,
            b,
            0x0086_e770,
            &[],
            true,
            &format!("Atel_RestoreLights {what} {k}"),
            |r| hp::atel_restore_lights(r),
        );
        fill_both(a, b, rng, saved, 0x70);
        r22_case(
            a,
            b,
            0x0086_db20,
            &[],
            true,
            &format!("Atel_SaveLights {what} {k}"),
            |r| hp::atel_save_lights(r),
        );
    }
    // Atel_StepSpriteAnim
    for round in 0..40u32 {
        let data = alloc_both(a, b, 0x100);
        let n = [1i16, 3, 6, 0, -1][rng.below(5) as usize];
        poke(a, b, data + 4, n);
        for f in 0..8u32 {
            poke(
                a,
                b,
                data + f * 8 + 0x12,
                [0x100i16, 0x300, 0, -5, 0x7fff][rng.below(5) as usize],
            );
            poke(a, b, data + f * 8 + 0x14, rng.next().to_le_bytes()[0]);
        }
        let s = alloc_both(a, b, 0x1c);
        fill_both(a, b, rng, s, 0x1c);
        poke(a, b, s + 0x14, data);
        poke(a, b, s + 0xc, u16::try_from(rng.below(7)).unwrap());
        poke(
            a,
            b,
            s + 0x18,
            [0u32, 0x100, 0x200, 0xffff_ff00, 0x7fff_ff00][rng.below(5) as usize],
        );
        r22_case(
            a,
            b,
            0x0087_4bf0,
            &[s],
            false,
            &format!("Atel_StepSpriteAnim {what} {round}"),
            |r| {
                hp::atel_step_sprite_anim(r, s);
                0
            },
        );
    }
    // Atel_LoadEventTexList
    r25_fios_prefix(a, b);
    let seen = r25_copy_stub(a, b, 0x0063_9310);
    let lang = r24_lang_word(a, b);
    for (k, on) in [0u32, 1, 0xffff_ffff].into_iter().enumerate() {
        fill_both(a, b, rng, seen, 0x40);
        fill_both(a, b, rng, a.rebase(0x0132_7140), 8);
        poke(a, b, lang, rng.below(13));
        let (x, y) = (rng.next(), rng.next());
        r22_case(
            a,
            b,
            0x0087_4b70,
            &[on, x, y],
            false,
            &format!("Atel_LoadEventTexList {what} {k}"),
            |r| {
                let f = frame(r);
                hp::atel_load_event_tex_list(r, on, x, y, f);
                0
            },
        );
    }
}

/// Round 25 b: `Atel_ResetTextLanguage_0086ece0` with its callees stubbed:
/// the language word before and after, the build's mode and how many
/// tries the swap takes.
fn r25b_language(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0078_5ed0, 1),
            (0x008b_0920, 1),
            (0x0088_7f00, 1),
            (0x0076_cfd0, 0),
        ],
    );
    for round in 0..16u32 {
        let was = rng.below(2);
        let after = if round % 4 == 0 {
            was
        } else {
            was ^ (1 + rng.below(2) * 2)
        };
        r24_seq_stub(a, b, 0x0078_51d0, 0, &[was, after, rng.next()]);
        r25_ret(a, b, 0x0078_26a0, [0u32, 1, 0x100][(round % 3) as usize]);
        let tries = round / 4 % 3;
        let mut exec = vec![0u32; tries as usize];
        exec.push(1 + rng.below(3));
        r24_seq_stub(a, b, 0x008b_0340, 0, &exec);
        r22_case(
            a,
            b,
            0x0086_ece0,
            &[],
            false,
            &format!("Atel_ResetTextLanguage {what} {round}"),
            |r| {
                hp::atel_reset_text_language(r);
                0
            },
        );
    }
}

#[test]
fn round25b_small_helpers_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0252);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r25b_cases(&mut a, &mut b, &mut rng, "image");
        r25b_language(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r25b_cases(&mut a, &mut b, &mut rng, "snapshot");
    r25b_language(&mut a, &mut b, &mut rng, "snapshot");
}

/// Replaces `f` in both oracles with a stub that stores three words from
/// cells into the addresses its three arguments hold; returns the first
/// cell (the others follow).
fn r25_out3_stub(a: &mut Oracle, b: &mut Oracle, f: u32) -> u32 {
    let cells = alloc_both(a, b, 12);
    let mut code = vec![];
    for k in 0..3u8 {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x8b, 0x0d]); // mov eax,[esp+4+4k]; mov ecx,
        code.extend((cells + u32::from(k) * 4).to_le_bytes());
        code.extend([0x89, 0x08]); // mov [eax],ecx
    }
    code.push(0xc3);
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    cells
}

/// Round 25 c on one pair: the separates fade, the sprite steps, the
/// guide map draw, the party mask, the bar segment, the battle-end map
/// start and the movie texture (callees logged or stubbed).
#[allow(clippy::too_many_lines)] // one block per function
fn r25c_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    r24_log_stubs(
        a,
        b,
        &[
            (0x0088_6ff0, 3),
            (0x0091_9d00, 1),
            (0x0084_84c0, 4),
            (0x0084_8680, 2),
            (0x0090_9b90, 0),
            (0x0088_d680, 0),
            (0x0091_9da0, 1),
            (0x0072_9a10, 1),
            (0x0090_a2b0, 1),
            (0x0091_9cd0, 0),
            (0x0091_9a40, 0),
            (0x0064_3aa0, 2),
            (0x0087_3f70, 2),
        ],
    );
    let works = a.rebase(0x0132_5ba0);
    // AtelBGSepFade
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    let old_hdr = Mem::read::<u32>(b, works + 0x48);
    for round in 0..8u32 {
        let hdr = r25_sep_header(a, b, rng, [3i32, 0, 9, 16][(round % 4) as usize]);
        poke(a, b, works + 0x48, hdr);
        let f = Mem::read::<u8>(b, works);
        poke(a, b, works, f | 1);
        poke(a, b, save + 0x28, u8::from(rng.below(2) == 0));
        let (volume, frames) = (rng.next(), rng.below(0x200));
        r22_case(
            a,
            b,
            0x0086_1000,
            &[volume, frames],
            true,
            &format!("AtelBGSepFade {what} {round}"),
            |r| hp::atel_bg_sep_fade(r, volume, frames),
        );
    }
    poke(a, b, works + 0x48, old_hdr);
    // Atel_StepSprites
    let sprites = a.rebase(0x0132_7148);
    let anim = alloc_both(a, b, 0x100);
    poke(a, b, anim + 4, 4i16);
    for f in 0..8u32 {
        poke(
            a,
            b,
            anim + f * 8 + 0x12,
            [0x100i16, 0x300, 0][rng.below(3) as usize],
        );
        poke(a, b, anim + f * 8 + 0x14, rng.next().to_le_bytes()[0]);
    }
    for round in 0..8u32 {
        for i in 0..0x80u32 {
            let s = sprites + i * 0x1c;
            poke(a, b, s, u8::from(rng.below(3) == 0));
            poke(a, b, s + 0xc, u16::try_from(rng.below(4)).unwrap());
            poke(a, b, s + 0x14, anim);
            poke(a, b, s + 0x18, rng.below(4) * 0x100);
        }
        poke(a, b, a.rebase(0x0132_7144), u32::from(round % 4 != 1));
        poke(a, b, a.rebase(0x0132_7f4c), u32::from(round % 4 != 2));
        r25_ret(a, b, 0x0086_d940, u32::from(round % 4 == 3));
        r22_case(
            a,
            b,
            0x0087_43e0,
            &[],
            false,
            &format!("Atel_StepSprites {what} {round}"),
            |r| {
                hp::atel_step_sprites(r);
                0
            },
        );
    }
    // AtelDrawSemiTrans
    let menu = alloc_both(a, b, 4);
    r25_ret(a, b, 0x0078_5290, menu);
    for round in 0..40u32 {
        let f = Mem::read::<u8>(b, works);
        poke(a, b, works, (f & !1) | u8::from(round % 8 != 0));
        fill_both(a, b, rng, works + 3, 1);
        poke(
            a,
            b,
            works + 0x1f4,
            [0u32, 2, 3, 4, 0xffff_ffff, 0x8000_0000][rng.below(6) as usize],
        );
        fill_both(a, b, rng, menu, 1);
        r25_ret(a, b, 0x0086_d940, u32::from(round % 8 == 1));
        r22_case(
            a,
            b,
            0x0087_2ca0,
            &[],
            false,
            &format!("AtelDrawSemiTrans {what} {round}"),
            |r| {
                hp::atel_draw_semi_trans(r);
                0
            },
        );
    }
    // Atel_SetPartyMask
    let members = r25_out3_stub(a, b, 0x0078_52f0);
    for round in 0..10u32 {
        for k in 0..3 {
            let v = [0xffu32, 0, 3, 7, 31, 33, 0x100][rng.below(7) as usize];
            poke(a, b, members + k * 4, v);
        }
        fill_both(a, b, rng, a.rebase(0x0132_6b7c), 4);
        r22_case(
            a,
            b,
            0x0086_23b0,
            &[],
            false,
            &format!("Atel_SetPartyMask {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_set_party_mask(r, f);
                0
            },
        );
    }
    // Native_DrawBarSegment
    for round in 0..20u32 {
        let pos = alloc_both(a, b, 8);
        fill_both(a, b, rng, pos, 8);
        let seg = alloc_both(a, b, 8);
        fill_both(a, b, rng, seg, 8);
        poke(a, b, seg + 4, [0u8, 1, 2, 0][(round % 4) as usize]);
        r24_seq_stub(a, b, 0x0082_0580, 1, &[rng.below(0x40) + 1]);
        poke(
            a,
            b,
            a.rebase(0x0132_59f0),
            rng.below(0x400).cast_signed() - 0x100,
        );
        poke(
            a,
            b,
            a.rebase(0x0132_59e8),
            [1i32, 7, -3, 0x100][rng.below(4) as usize],
        );
        poke(
            a,
            b,
            a.rebase(0x0132_59f4),
            rng.below(0x200).cast_signed() - 0x80,
        );
        fill_both(a, b, rng, a.rebase(0x0132_59f8), 4);
        r22_case(
            a,
            b,
            0x0085_5a30,
            &[pos, seg],
            true,
            &format!("Native_DrawBarSegment {what} {round}"),
            |r| hp::native_draw_bar_segment(r, pos, seg),
        );
    }
    // Atel_BattleEndMapStart
    for round in 0..8u32 {
        let map = rng.below(4);
        r25_ret(
            a,
            b,
            0x0088_d690,
            if round % 2 == 0 { map } else { map + 1 },
        );
        fill_both(a, b, rng, a.rebase(0x0132_5b88), 4);
        let g = Mem::read::<u8>(b, works + 1);
        poke(a, b, works + 1, g ^ (rng.next().to_le_bytes()[0] & 0x80));
        r22_case(
            a,
            b,
            0x0086_15b0,
            &[map],
            true,
            &format!("Atel_BattleEndMapStart {what} {round}"),
            |r| hp::atel_battle_end_map_start(r, map),
        );
    }
    // Atel_SetMovieTexture
    for round in 0..24u32 {
        poke(
            a,
            b,
            save + 0xe0,
            [0u32, 1, 2, 3, 5, 6, 0xffff_ffff][rng.below(7) as usize],
        );
        let id = rng.below(3);
        let flags = [0u32, 0x80, 0x81][rng.below(3) as usize];
        poke(a, b, save + 0xd8, rng.below(3));
        poke(a, b, save + 0xdc, [0u32, 0x80, 0x81][rng.below(3) as usize]);
        r22_case(
            a,
            b,
            0x0087_3eb0,
            &[id, flags],
            true,
            &format!("Atel_SetMovieTexture {what} {round}"),
            |r| hp::atel_set_movie_texture(r, id, flags),
        );
    }
}

/// Round 25 c: `Atel_RegistVibrateSignal_00876590` over the workers of
/// ctrl work 0 (the curve and the signal stubbed and logged).
fn r25c_vibrate_signal(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(a, b, &[(0x0086_ec40, 5)]);
    let works = a.rebase(0x0132_5ba0);
    let n = u32::from(Mem::read::<u16>(b, works + 0xc)).min(6);
    let ctx_cell = a.rebase(0x0132_6b28);
    let here = Mem::read::<u32>(b, ctx_cell);
    let mut cases = 0;
    for i in 0..n {
        poke(a, b, ctx_cell, works);
        let w = v::atel_get_basic_work(b, i.cast_signed());
        orig(a, ATEL_GET_BASIC_WORK, &[i]);
        poke(a, b, ctx_cell, here);
        for k in [0u32, 1, 2, 3, 5, 7, 8, 9] {
            fill_both(a, b, rng, w + 0xab, 1);
            let curves = [0xffffu32, rng.below(0x40), rng.below(0x40)];
            r24_seq_stub(a, b, 0x0087_63e0, 3, &curves[rng.below(3) as usize..]);
            let from = rng.next();
            // the ctrl work it comes back to: the snapshot's, or another
            poke(
                a,
                b,
                ctx_cell,
                if k % 2 == 1 { works + 0x238 } else { here },
            );
            r22_case(
                a,
                b,
                0x0087_6590,
                &[from, i, k],
                true,
                &format!("Atel_RegistVibrateSignal {what} {i} {k}"),
                |r| hp::atel_regist_vibrate_signal(r, from, i, k),
            );
            cases += 1;
        }
    }
    eprintln!("round 25c: {cases} vibrate signal cases ({what})");
}

#[test]
fn round25c_fade_sprites_guide_party_bar_movie_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0253);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r25c_cases(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r25c_cases(&mut a, &mut b, &mut rng, "snapshot");
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r25c_vibrate_signal(&mut a, &mut b, &mut rng, "snapshot");
}

/// Replaces `f(side, out)` in both oracles with a stub that stores a
/// cell's word at `out + 4` (`MsCameraGetPosNow`'s y), keeps `side` in
/// the next word and returns 0; returns the cell.
fn r25_cam_y_stub(a: &mut Oracle, b: &mut Oracle, f: u32) -> u32 {
    let cell = alloc_both(a, b, 8);
    let code: Vec<u8> = [
        &[0x8b, 0x44, 0x24, 0x04, 0xa3][..], // mov eax,[esp+4]; mov [cell+4],eax (the side)
        &(cell + 4).to_le_bytes(),
        &[0x8b, 0x44, 0x24, 0x08, 0x8b, 0x0d], // mov eax,[esp+8]; mov ecx,
        &cell.to_le_bytes(),
        &[0x89, 0x48, 0x04, 0x31, 0xc0, 0xc3], // mov [eax+4],ecx; xor eax,eax; ret
    ]
    .concat();
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    cell
}

/// Finite floats for matrices and positions (no NaNs: the inverse would
/// spread them, a case of its own).
const R25_FINITE: [u32; 8] = [
    0,
    0x3f80_0000,
    0xbf80_0000,
    0x4120_0000,
    0x3e80_0000,
    0xc2c8_0000,
    0x4049_0fdb,
    0x8000_0000,
];

/// Round 25 d on one pair: the turn to a point, the sprite show, the water
/// status, the pad history, the event path language (callees logged or
/// stubbed).
#[allow(clippy::too_many_lines)] // one block per function
fn r25d_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    // Atel_TurnToPoint
    let yaw_pitch = |r: &mut Run, args: &[u32]| {
        let f = r.0.rebase(0x0088_b4d0);
        r.0.call(f, Conv::Cdecl, args).unwrap().eax
    };
    for round in 0..24u32 {
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w + 4, 0x5fc);
        let kind = alloc_both(a, b, 4);
        poke(a, b, kind, [5u8, 6, 0, 7][(round % 4) as usize]);
        poke(a, b, w, kind);
        poke(
            a,
            b,
            w + 0x32,
            [0u8, 1, 2, 5, 8, 9, 0xff][rng.below(7) as usize],
        );
        for l in 0..9 {
            let t = alloc_both(a, b, 0x30);
            fill_both(a, b, rng, t, 0x30);
            poke(a, b, w + 0x174 + l * 0x4c, t);
        }
        for off in [0x284u32, 0x558] {
            for k in 0..3 {
                poke(a, b, w + off + k * 4, R25_FINITE[rng.below(8) as usize]);
            }
        }
        let point = [0u32; 3].map(|_| R25_FINITE[rng.below(8) as usize]);
        r22_case(
            a,
            b,
            0x0086_f7f0,
            &[w, point[0], point[1], point[2]],
            false,
            &format!("Atel_TurnToPoint {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_turn_to_point(r, w, point, f, yaw_pitch);
                0
            },
        );
    }
    // Atel_ShowSprite
    r24_log_stubs(a, b, &[(0x007e_cf50, 3), (0x00a4_4700, 1)]);
    r25_fios_prefix(a, b);
    let seen = r25_copy_stub(a, b, 0x0063_9310);
    let lang = r24_lang_word(a, b);
    for (k, i) in [0u32, 3, 0x7f, 0x80, 0xffff_fff0, 0x7fff_ffff]
        .into_iter()
        .enumerate()
    {
        fill_both(a, b, rng, seen, 0x40);
        poke(a, b, a.rebase(0x023c_bc5c), u32::from(k % 3 != 0));
        poke(a, b, a.rebase(0x0132_7140), u32::from(k % 2 == 0));
        poke(a, b, lang, rng.below(13));
        let tex = rng.next();
        let s = a.rebase(0x0132_7148) + i.cast_signed().clamp(0, 0x7f).cast_unsigned() * 0x1c;
        fill_both(a, b, rng, s, 0x1c);
        r22_case(
            a,
            b,
            0x0087_48a0,
            &[i, tex],
            false,
            &format!("Atel_ShowSprite {what} {k}"),
            |r| {
                let f = frame(r);
                hp::atel_show_sprite(r, i, tex, f);
                0
            },
        );
    }
    // AtelUpdateWaterStatus
    let cam_y = r25_cam_y_stub(a, b, 0x007b_d470);
    let works = a.rebase(0x0132_5ba0);
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    let ctx_cell = a.rebase(0x0132_6b28);
    let levels = [0u32, 0x3f80_0000, 0xbf80_0000, 0x4120_0000, 0x7fc0_0000];
    for round in 0..40u32 {
        r25_ret(a, b, 0x0078_0c90, [1u32, 0, 0, 2][(round % 4) as usize]);
        r25_ret(
            a,
            b,
            0x0090_8dd0,
            [0u32, 1, 3, 0xffff_ffff, 2][rng.below(5) as usize],
        );
        fill_both(a, b, rng, save + 0x29, 1);
        if rng.below(2) == 0 {
            poke(a, b, save + 0x29, 0u8);
        }
        fill_both(a, b, rng, works + 2, 1);
        poke(a, b, works + 0x208, levels[rng.below(5) as usize]);
        poke(a, b, cam_y, levels[rng.below(5) as usize]);
        fill_both(a, b, rng, a.rebase(0x0132_6b83), 1);
        if round % 3 == 0 {
            poke(a, b, ctx_cell, works + 0x238);
        }
        r22_case(
            a,
            b,
            0x0087_1e50,
            &[],
            false,
            &format!("AtelUpdateWaterStatus {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_update_water_status(r, f);
                0
            },
        );
    }
    // AtelExec_Internal_871d10
    for round in 0..8u32 {
        r25_ret(
            a,
            b,
            0x0088_7dc0,
            [0u32, 1, 2, 3, 0xffff_ffff][rng.below(5) as usize],
        );
        for f in [
            0x0088_8d70,
            0x0088_8e50,
            0x0088_8e30,
            0x0088_8e80,
            0x0088_8ea0,
        ] {
            let vals: Vec<u32> = (0..8).map(|_| rng.next()).collect();
            r24_seq_stub(a, b, f, 3, &vals);
        }
        fill_both(a, b, rng, a.rebase(0x0132_70c0), 0x18);
        r22_case(
            a,
            b,
            0x0087_1d70,
            &[],
            false,
            &format!("AtelExec_Internal_871d10 {what} {round}"),
            |r| {
                hp::atel_exec_internal_871d10(r);
                0
            },
        );
    }
    // Atel_SetEventPathLang
    r24_log_stubs(a, b, &[(0x00a4_2f30, 1), (0x00a4_3220, 2)]);
    for l in (0..20u32).chain([0xffff_ffff, 0x100]) {
        r25_ret(a, b, 0x0088_7d00, l);
        fill_both(a, b, rng, a.rebase(0x0132_85b4), 4);
        let to = rng.next();
        r22_case(
            a,
            b,
            0x0087_9430,
            &[to],
            false,
            &format!("Atel_SetEventPathLang {what} {l}"),
            |r| {
                hp::atel_set_event_path_lang(r, to);
                0
            },
        );
    }
}

/// Round 25 d on the snapshot's workers: the vibration signal with a
/// curve, the listener matrix, the affection and the variable slots.
#[allow(clippy::too_many_lines)] // one block per function
fn r25d_workers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    let works = a.rebase(0x0132_5ba0);
    let ctx_cell = a.rebase(0x0132_6b28);
    let here = Mem::read::<u32>(b, ctx_cell);
    let n = u32::from(Mem::read::<u16>(b, works + 0xc)).min(6);
    // Atel_RegistVibrateSignalWith
    r24_log_stubs(a, b, &[(0x0086_e990, 5)]);
    for i in 0..n {
        poke(a, b, ctx_cell, works);
        let w = v::atel_get_basic_work(b, i.cast_signed());
        orig(a, ATEL_GET_BASIC_WORK, &[i]);
        for k in [0u32, 1, 3, 7, 8] {
            fill_both(a, b, rng, w + 0xab, 1);
            let curve = [0xffff_ffffu32, 0xffff, 3, 0x8000_0000, 0x10, 0][rng.below(6) as usize];
            r24_seq_stub(
                a,
                b,
                0x0087_63e0,
                3,
                &[[0xffffu32, 5][rng.below(2) as usize]],
            );
            poke(
                a,
                b,
                ctx_cell,
                if k % 2 == 1 { works + 0x238 } else { here },
            );
            let from = rng.next();
            r22_case(
                a,
                b,
                0x0087_66d0,
                &[from, i, k, curve],
                true,
                &format!("Atel_RegistVibrateSignalWith {what} {i} {k}"),
                |r| hp::atel_regist_vibrate_signal_with(r, from, i, k, curve),
            );
        }
    }
    // AtelSetSndCameraPos
    r24_log_stubs(a, b, &[(0x007c_0070, 1)]);
    let cam = a.rebase(0x0132_82c8);
    for round in 0..8u32 {
        for k in 0..16 {
            poke(a, b, cam + k * 4, R25_FINITE[rng.below(8) as usize]);
        }
        for k in [0u32, 5, 10, 15] {
            poke(a, b, cam + k * 4, 0x3f80_0000u32);
        }
        let player = if round % 3 == 0 || n == 0 {
            0xffffu16
        } else {
            u16::try_from(rng.below(n)).unwrap()
        };
        poke(a, b, works + 0xa, player);
        poke(
            a,
            b,
            ctx_cell,
            if round % 2 == 1 { works + 0x238 } else { here },
        );
        r22_case(
            a,
            b,
            0x0087_7470,
            &[],
            false,
            &format!("AtelSetSndCameraPos {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_set_snd_camera_pos(r, f);
                0
            },
        );
    }
    // Atel_AddLoveParam
    poke(a, b, ctx_cell, here);
    let c = here;
    let save = orig(a, 0x0088_c830, &[]);
    orig(b, 0x0088_c830, &[]);
    let cn = u32::from(Mem::read::<u16>(b, c + 0xc)).min(6);
    let mut cases = 0;
    for round in 0..96u32 {
        if cn == 0 {
            break;
        }
        let i = rng.below(cn);
        let w = v::atel_get_basic_work(b, i.cast_signed());
        orig(a, ATEL_GET_BASIC_WORK, &[i]);
        poke(
            a,
            b,
            w + 0xaa,
            [0u8, 1, 1, 1, 2, 1, 1, 1][(round % 8) as usize],
        );
        if rng.below(6) == 0 {
            poke(a, b, w + 0x9c, 0u32);
        } else if Mem::read::<u32>(b, w + 0x9c) == 0 {
            poke(a, b, w + 0x9c, 0x1234u32);
        }
        poke(
            a,
            b,
            w + 0x40,
            [0u32, 1, 3, 7, 0, 5, 8, 0xffff_ffff][(round % 8) as usize],
        );
        let f = Mem::read::<u8>(b, c + 2);
        poke(
            a,
            b,
            c + 2,
            if round % 5 == 0 { f & !0x40 } else { f | 0x40 },
        );
        fill_both(a, b, rng, c + 4, 4);
        poke(
            a,
            b,
            a.rebase(0x0132_6b90),
            [0u32, 0, 0, 0x2, 0xff, 0x80][rng.below(6) as usize],
        );
        r25_ret(a, b, 0x0078_4f60, [1u32, 0, 1, 2][rng.below(4) as usize]);
        r25_ret(a, b, 0x0078_4f90, [0u32, 1, 1, 2][rng.below(4) as usize]);
        r25_ret(
            a,
            b,
            0x0088_d660,
            [0u32, 5, 31, 32, 63, 639, 640, 0xffff_ffff, 0xffff_ffc0][if round % 6 == 5 {
                8
            } else {
                rng.below(9) as usize
            }],
        );
        fill_both(a, b, rng, save + 0x2c, 0x20);
        fill_both(a, b, rng, save + 0x4c, 0x50);
        let skip = u32::from(round % 7 == 3);
        r22_case(
            a,
            b,
            0x0086_5650,
            &[i, skip],
            false,
            &format!("Atel_AddLoveParam {what} {round}"),
            |r| {
                hp::atel_add_love_param(r, i, skip);
                0
            },
        );
        cases += 1;
    }
    // Atel_FindVarSlot: worker 0's script header replaced by one of ours,
    // the reference aimed at an entry's first, last, past-the-end or a
    // middle element
    if cn > 0 {
        let w = v::atel_get_basic_work(b, 0);
        orig(a, ATEL_GET_BASIC_WORK, &[0]);
        let sizes = [1u32, 1, 2, 2, 4, 4, 4, 1];
        for round in 0..64u32 {
            let hdr = alloc_both(a, b, 0x400);
            let level = rng.below(3);
            poke(a, b, w + 0x2e, u16::try_from(level).unwrap());
            poke(a, b, w + 4, hdr);
            poke(a, b, hdr + 0x38 + level * 4, 0x100u32);
            let ne = [1u16, 2, 4, 8][rng.below(4) as usize];
            poke(a, b, hdr + 0x102, ne);
            poke(a, b, hdr + 0x114, 0x200u32);
            let storage = [0u32, 0x0600_0000, 0x0200_0000, 0x0e00_0000][(round % 4) as usize];
            let mut ents = vec![];
            for e in 0..8u32 {
                let kind = rng.below(8);
                // a base just below bit 24 now and then: the element
                // index masks bits 24-27 off both
                let base = if round % 8 == 7 {
                    storage | 0x00ff_fff0 | (rng.below(4) * 4)
                } else {
                    storage | (rng.below(0x40) * 4)
                };
                let count = 1 + rng.below(7);
                poke(a, b, hdr + 0x200 + e * 8, (kind << 28) | base);
                poke(a, b, hdr + 0x204 + e * 8, u16::try_from(count).unwrap());
                ents.push((kind, base, count));
            }
            if rng.below(3) != 0 {
                let wc = alloc_both(a, b, 0x100);
                fill_both(a, b, rng, wc + 0x60, 0x40);
                for k in 0..3u32 {
                    poke(
                        a,
                        b,
                        wc + 0x60 + k * 0x10 + 0xc,
                        u16::try_from(rng.below(5)).unwrap(),
                    );
                }
                poke(a, b, wc + 0x54, 0x60u32);
                poke(a, b, w + 8, wc);
            } else {
                poke(a, b, w + 8, 0u32);
            }
            let rec = alloc_both(a, b, 8);
            fill_both(a, b, rng, rec, 2);
            poke(a, b, rec + 2, 0u16);
            let ti = rng.below(u32::from(ne));
            let t = ents[ti as usize];
            // the actor's limit right at the entry now and then
            if Mem::read::<u32>(b, w + 8) != 0 && rng.below(2) == 0 {
                let wc = Mem::read::<u32>(b, w + 8);
                poke(
                    a,
                    b,
                    wc + 0x60 + level * 0x10 + 0xc,
                    u16::try_from(ti).unwrap(),
                );
            }
            let span = t.2 * sizes[t.0 as usize];
            let at = [
                0,
                span - 1,
                span,
                rng.below(span),
                span + 1,
                rng.below(span),
            ][rng.below(6) as usize];
            let v = if round % 9 == 8 {
                (rng.below(8) << 28) | storage | rng.below(0x140)
            } else {
                (t.0 << 28) | (t.1 + at)
            };
            poke(a, b, rec + 4, v);
            r22_case(
                a,
                b,
                0x0087_bc90,
                &[rec],
                false,
                &format!("Atel_FindVarSlot {what} {round}"),
                |r| {
                    hp::atel_find_var_slot(r, rec);
                    0
                },
            );
        }
    }
    eprintln!("round 25d: {cases} affection cases ({what})");
}

#[test]
fn round25d_turn_sprite_water_pad_lang_love_vars_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0254);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r25d_cases(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r25d_cases(&mut a, &mut b, &mut rng, "snapshot");
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    r25d_workers(&mut a, &mut b, &mut rng, "snapshot");
}

/// Code in both oracles that logs `tag` and its first `n` stack words
/// into a fresh log and returns the log's new end (a function pointer for
/// the original's indirect calls; [`r24_log_stubs`] without the patch).
/// Returns the code's address.
fn r25_log_fn(a: &mut Oracle, b: &mut Oracle, tag: u32, n: u8) -> u32 {
    let log = alloc_both(a, b, 0x1000);
    let cell = alloc_both(a, b, 4);
    poke(a, b, cell, log);
    let mut code: Vec<u8> = [&[0x8b, 0x0d][..], &cell.to_le_bytes()].concat();
    code.extend([&[0xc7, 0x01][..], &tag.to_le_bytes()].concat());
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 + 4 * k]);
    }
    code.extend([0x83, 0xc1, 4 + 4 * n]);
    code.extend([&[0x89, 0x0d][..], &cell.to_le_bytes()].concat());
    code.extend([0x89, 0xc8, 0xc3]);
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    at
}

/// Replaces `f` in both oracles with a stub that keeps its first `args`
/// (at most 3) stack words after the cells, stores `n` words from cells at the
/// address in its stack argument `arg` (0-based) and returns 0; returns
/// the first cell.
fn r25_out_stub(a: &mut Oracle, b: &mut Oracle, f: u32, args: u8, arg: u8, n: u8) -> u32 {
    let cells = alloc_both(a, b, 4 * usize::from(n) + 12);
    let mut code = vec![];
    for k in 0..args {
        // mov eax,[esp+4+4k]; mov [cells+4n+4k],eax (what the caller passed)
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0xa3]);
        code.extend((cells + 4 * u32::from(n + k)).to_le_bytes());
    }
    code.extend([0x8b, 0x44, 0x24, 4 + 4 * arg]); // mov eax,[esp+4+4arg]
    for k in 0..n {
        code.extend([0x8b, 0x0d]); // mov ecx,[cell k]
        code.extend((cells + u32::from(k) * 4).to_le_bytes());
        code.extend([0x89, 0x48, 4 * k]); // mov [eax+4k],ecx
    }
    code.extend([0x31, 0xc0, 0xc3]);
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    cells
}

/// Replaces `f` in both oracles with a stub that keeps its first three
/// stack words and the `n` bytes its argument `arg` points at in a fresh
/// buffer (words, then the bytes) and returns 0; returns the buffer.
fn r25_copy_raw_stub(a: &mut Oracle, b: &mut Oracle, f: u32, arg: u8, n: u8) -> u32 {
    let buf = alloc_both(a, b, 12 + usize::from(n));
    let mut code = vec![];
    for k in 0..3u8 {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0xa3]);
        code.extend((buf + 4 * u32::from(k)).to_le_bytes());
    }
    code.extend([0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c + 4 * arg, 0xbf]); // push esi; push edi; mov esi,[esp+0xc+4arg]; mov edi,
    code.extend((buf + 12).to_le_bytes());
    code.extend([0xb9, n, 0, 0, 0, 0xf3, 0xa4, 0x5f, 0x5e, 0x31, 0xc0, 0xc3]); // mov ecx,n; rep movsb; pop edi; pop esi; xor eax,eax; ret
    let at = a.alloc_code(&code).unwrap();
    assert_eq!(b.alloc_code(&code).unwrap(), at);
    for o in [&mut *a, &mut *b] {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[&[0xe9][..], &rel.to_le_bytes()].concat())
            .unwrap();
    }
    buf
}

/// Round 25 e on one pair: the turn end, the stream seek, the game-over
/// jump and the separates kick (callees logged or stubbed).
#[allow(clippy::too_many_lines)] // one block per function
fn r25e_cases(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    r24_log_stubs(a, b, &[(0x0082_b520, 2), (0x0086_3fe0, 2)]);
    // Atel_SyncTurnEnd
    let turn_floats = [0u32, 0x3f80_0000, 0x8000_0000, 0x7fc0_0000, 0x4000_0000];
    for round in 0..60u32 {
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w + 4, 0x5fc);
        let kind = alloc_both(a, b, 4);
        poke(a, b, kind, [5u8, 6, 0, 7][(round % 4) as usize]);
        poke(a, b, w, kind);
        poke(a, b, w + 0xaa, u8::from(rng.below(3) != 0));
        poke(
            a,
            b,
            w + 0x9c,
            if rng.below(4) == 0 { 0 } else { rng.next() | 1 },
        );
        let t = alloc_both(a, b, 0x40);
        fill_both(a, b, rng, t, 0x40);
        let mode = [3u16, 4, 5, 0, 1, 6, 0x1f, 0x13, 0x15, 0x14][rng.below(10) as usize]
            | [0u16, 0x80, 0x40, 0xc0, 0x4000, 0x1000][rng.below(6) as usize];
        poke(
            a,
            b,
            t + 2,
            mode | if rng.below(2) == 0 { 0x20u16 } else { 0 },
        );
        for off in [8u32, 0xc, 0x20, 0x24, 0x28] {
            poke(a, b, t + off, turn_floats[rng.below(5) as usize]);
        }
        for base in [0x284u32, 0x558] {
            for off in [0x30u32, 0x34, 0x38] {
                poke(a, b, w + base + off, turn_floats[rng.below(5) as usize]);
            }
        }
        // the direction reached now and then
        let p = if matches!(round % 4, 0 | 1) {
            w + 0x284
        } else {
            w + 0x558
        };
        if round % 2 == 0 {
            let d = Mem::read::<u32>(b, p + 0x34);
            poke(a, b, t + 0x20, d);
        }
        if round % 4 == 0 {
            let d = Mem::read::<u32>(b, p + 0x30);
            poke(a, b, t + 0x24, d);
        }
        let lvl = alloc_both(a, b, 0x4c);
        poke(a, b, lvl + 0x48, t);
        r22_case(
            a,
            b,
            0x0087_0dc0,
            &[w, lvl],
            false,
            &format!("Atel_SyncTurnEnd {what} {round}"),
            |r| {
                hp::atel_sync_turn_end(r, w, lvl);
                0
            },
        );
    }
    // Atel_StreamSeek
    let name = r25_out_stub(a, b, 0x0088_7870, 2, 1, 4);
    let cd = alloc_both(a, b, 0x20);
    let select = r25_log_fn(a, b, 0x5e1, 1);
    let seek = r25_log_fn(a, b, 0x5ee, 1);
    poke(a, b, cd + 0x10, select);
    poke(a, b, cd + 4, seek);
    r25_ret(a, b, 0x0076_d0d0, cd);
    for round in 0..12u32 {
        for (k, word) in [*b"bgm_", *b"0123", *b"ab\0x", [0u8; 4]]
            .into_iter()
            .enumerate()
        {
            poke(
                a,
                b,
                name + 4 * u32::try_from(k).unwrap(),
                u32::from_le_bytes(word),
            );
        }
        if round % 3 == 0 {
            poke(a, b, name + 8, u32::from_le_bytes(*b"zz\0\0"));
        }
        r24_seq_stub(a, b, 0x0088_7790, 1, &[rng.next()]);
        let start = if round % 4 == 1 {
            0
        } else {
            alloc_both(a, b, 4)
        };
        let count = if round % 4 == 2 {
            0
        } else {
            alloc_both(a, b, 4)
        };
        if count != 0 {
            poke(a, b, count, rng.below(5));
        }
        let sector = alloc_both(a, b, 4);
        poke(a, b, sector, rng.below(0x40));
        let id = rng.next();
        let pos = if round % 3 == 0 { u32::MAX } else { rng.next() };
        fill_both(a, b, rng, a.rebase(0x0133_0098), 0x48);
        r22_case(
            a,
            b,
            0x0088_5bf0,
            &[start, count, sector, id, pos],
            true,
            &format!("Atel_StreamSeek {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_stream_seek(r, start, count, sector, id, pos, f)
            },
        );
    }
    // Atel_KickBGSepsOnFlag
    r24_log_stubs(
        a,
        b,
        &[(0x0088_6eb0, 1), (0x0088_6e50, 3), (0x0088_6ff0, 3)],
    );
    let ctx_cell = a.rebase(0x0132_6b28);
    let ctx = alloc_both(a, b, 0x240);
    let here = Mem::read::<u32>(b, ctx_cell);
    poke(a, b, ctx_cell, ctx);
    for round in 0..24u32 {
        let hdr = r25_sep_header(a, b, rng, [0i32, 3, 9, 20][(round % 4) as usize]);
        for i in 0..20u32 {
            if rng.below(2) == 0 {
                let f = Mem::read::<u32>(b, hdr + 0x208 + i * 8);
                poke(
                    a,
                    b,
                    hdr + 0x208 + i * 8,
                    f | [0x100u32, 0x200, 0x80, 0x40][rng.below(4) as usize],
                );
            }
        }
        poke(a, b, ctx + 2, [8u8, 0, 8, 0xff][rng.below(4) as usize]);
        poke(
            a,
            b,
            ctx + 0x1f4,
            [3u32, 4, 0, 0xffff_ffff][rng.below(4) as usize],
        );
        let on = [0u32, 1, 0x100][(round % 3) as usize];
        r22_case(
            a,
            b,
            0x0086_5780,
            &[hdr, on],
            false,
            &format!("Atel_KickBGSepsOnFlag {what} {round}"),
            |r| {
                hp::atel_kick_bg_seps_on_flag(r, hdr, on);
                0
            },
        );
    }
    poke(a, b, ctx_cell, here);
}

/// Round 25 e: `AtelJumpGameOver` with every callee outside the script
/// module logged (the language swap stubbed as in round 25 b).
fn r25e_game_over(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    r24_log_stubs(
        a,
        b,
        &[
            (0x008a_de50, 0),
            (0x0079_8b60, 0),
            (0x0087_62f0, 0),
            (0x0087_5f60, 2),
            (0x0088_69e0, 1),
            (0x0088_6af0, 2),
            (0x0088_6a60, 1),
            (0x0088_6800, 1),
            (0x0088_6db0, 0),
            (0x0088_5ec0, 1),
            (0x0082_cb60, 1),
            (0x0082_bee0, 1),
            (0x0082_cc70, 0),
            (0x0090_8450, 0),
            (0x008a_dee0, 0),
            (0x0078_5ed0, 1),
            (0x008b_0920, 1),
            (0x0088_7f00, 1),
            (0x0076_cfd0, 0),
            (0x0088_6eb0, 1),
            (0x0088_6f90, 2),
            (0x0088_6dd0, 1),
            (0x0088_7780, 0),
        ],
    );
    let works = a.rebase(0x0132_5ba0);
    let music = a.rebase(0x0132_7130);
    for round in 0..6u32 {
        r24_seq_stub(a, b, 0x0078_51d0, 0, &[0, rng.below(2), 1]);
        r25_ret(a, b, 0x0078_26a0, rng.below(2));
        r24_seq_stub(a, b, 0x008b_0340, 0, &[0, 1]);
        r25_ret(
            a,
            b,
            0x0088_63f0,
            [0xffff_ffffu32, 3, 0x8a][(round % 3) as usize],
        );
        poke(a, b, music, [0xffff_ffffu32, 7, 0][(round % 3) as usize]);
        fill_both(a, b, rng, a.rebase(0x0132_6bdc), 1);
        fill_both(a, b, rng, a.rebase(0x0132_5b98), 8);
        fill_both(a, b, rng, works, 4);
        fill_both(a, b, rng, works + 0x210, 4);
        let ctx_cell = a.rebase(0x0132_6b28);
        if round % 2 == 1 {
            poke(a, b, ctx_cell, works + 0x238);
        }
        r22_case(
            a,
            b,
            0x0086_da10,
            &[],
            false,
            &format!("AtelJumpGameOver {what} {round}"),
            |r| {
                hp::atel_jump_game_over(r);
                0
            },
        );
    }
}

/// Round 25 e on the snapshot's workers: the turn to a worker, the
/// character's attachment point into variables, the polar move.
#[allow(clippy::too_many_lines)] // one block per function
fn r25e_workers(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, what: &str) {
    use ffx_script::vm::bridge::helpers as hp;
    let frame = |r: &Run| r.0.stack_range().start + 0x100;
    let works = a.rebase(0x0132_5ba0);
    let ctx_cell = a.rebase(0x0132_6b28);
    poke(a, b, ctx_cell, works);
    let n = u32::from(Mem::read::<u16>(b, works + ctrl::WORKERS));
    let ws: Vec<u32> = (0..n)
        .map(|i| {
            orig(a, ATEL_GET_BASIC_WORK, &[i]);
            v::atel_get_basic_work(b, i.cast_signed())
        })
        .collect();
    let kind = |o: &Oracle, w: u32| Mem::read::<u8>(o, Mem::read::<u32>(o, w));
    // Atel_TurnToWorker
    let yaw_pitch = |r: &mut Run, args: &[u32]| {
        let f = r.0.rebase(0x0088_b4d0);
        r.0.call(f, Conv::Cdecl, args).unwrap().eax
    };
    let mut turns = 0;
    for round in 0..24u32 {
        let w = alloc_both(a, b, 0x600);
        fill_both(a, b, rng, w + 4, 0x5fc);
        let k = alloc_both(a, b, 4);
        poke(a, b, k, [5u8, 6, 0, 7][(round % 4) as usize]);
        poke(a, b, w, k);
        poke(a, b, w + 0x32, [0u8, 1, 2, 8, 9][rng.below(5) as usize]);
        for l in 0..9 {
            let t = alloc_both(a, b, 0x30);
            fill_both(a, b, rng, t, 0x30);
            poke(a, b, w + 0x174 + l * 0x4c, t);
        }
        for off in [0x284u32, 0x558] {
            for j in 0..3 {
                poke(a, b, w + off + j * 4, R25_FINITE[rng.below(8) as usize]);
            }
        }
        let target = rng.below(n);
        if kind(b, ws[target as usize]) == 4 {
            continue;
        }
        r22_case(
            a,
            b,
            0x0086_f8a0,
            &[w, target],
            false,
            &format!("Atel_TurnToWorker {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_turn_to_worker(r, w, target, f, yaw_pitch);
                0
            },
        );
        turns += 1;
    }
    // Native_GetChrOptPos
    let opt = r25_out_stub(a, b, 0x0083_54f0, 3, 2, 4);
    let stk = alloc_both(a, b, 0x400);
    let mut opts = 0;
    for round in 0..32u32 {
        let w = ws[(round % n) as usize];
        if kind(b, w) == 4 {
            continue;
        }
        for k in 0..4 {
            poke(a, b, opt + k * 4, R25_FINITE[rng.below(8) as usize]);
        }
        let target = ws[rng.below(n) as usize];
        if rng.below(4) == 0 {
            poke(a, b, target + 0x9c, 0u32);
        } else {
            // a character of our own (the field snapshot has workers whose
            // handle is the debug heap's 0xcdcdcdcd)
            let chr = alloc_both(a, b, 0x700);
            fill_both(a, b, rng, chr + 0x6b4, 0x10);
            poke(a, b, target + 0x9c, chr);
        }
        let mode = [0u32, 0, 1, 2][(round % 4) as usize];
        poke(a, b, stk + stack::DEPTH, 0i32);
        let ti = ws.iter().position(|&x| x == target).unwrap_or(0);
        push_both(a, b, stk, stack::INT, u32::try_from(ti).unwrap());
        if mode == 0 {
            push_both(a, b, stk, stack::INT, rng.below(4));
        }
        // variables whose storage the snapshot has (a null storage would
        // put the store near address 0)
        for _ in 0..3 {
            let mut word = 0;
            for _ in 0..8 {
                // (storage 5 is the worker itself: a store there could
                // land on its character handle)
                let storage = [0u32, 1, 2, 4][rng.below(4) as usize];
                let cand = (rng.below(7) << 28) | (storage << 25) | (rng.below(0x40) * 4);
                let at = v::atel_get_real_var_adrs(&mut Run(b), w, cand);
                if at >= 0x0040_0000 {
                    word = cand;
                    break;
                }
            }
            push_both(a, b, stk, stack::INT, word);
        }
        r22_case(
            a,
            b,
            0x00a7_a980,
            &[w, 0, stk, mode],
            true,
            &format!("Native_GetChrOptPos {what} {round}"),
            |r| {
                let f = frame(r);
                hp::native_get_chr_opt_pos(r, w, stk, mode, f)
            },
        );
        opts += 1;
    }
    // Atel_ApplyPolarMove
    r24_log_stubs(a, b, &[(0x0086_f4c0, 1)]);
    r25_copy_raw_stub(a, b, 0x007b_f940, 2, 16);
    let mut moves = 0;
    for round in 0..32u32 {
        let w = ws[(round % n) as usize];
        if kind(b, w) == 4 {
            continue;
        }
        poke(
            a,
            b,
            w + 0xaa,
            [2u8, 3, 4, 5, 1, 6, 2, 2][(round % 8) as usize],
        );
        let p = if matches!(kind(b, w), 5 | 6) {
            w + 0x284
        } else {
            w + 0x558
        };
        for off in [0u32, 4, 8, 0xc, 0x20, 0x24] {
            poke(a, b, p + off, R25_FINITE[rng.below(8) as usize]);
        }
        poke(a, b, w + 0x32, u8::try_from(rng.below(3)).unwrap());
        let lv = v::script_worker_level(b, w, i32::from(Mem::read::<u8>(b, w + 0x32)));
        let lv0 = v::script_worker_level(b, w, 0);
        for off in [0x44u32, 0x48] {
            // the current level's blocks, and level 0's the other way
            let on = rng.below(2) == 0;
            for (l, set) in [(lv0, !on), (lv, on)] {
                let t = Mem::read::<u32>(b, l + off);
                if t >= 0x0040_0000 {
                    poke(a, b, t + 2, if set { 3u16 } else { 0 });
                }
            }
        }
        r22_case(
            a,
            b,
            0x0086_29c0,
            &[w],
            false,
            &format!("Atel_ApplyPolarMove {what} {round}"),
            |r| {
                let f = frame(r);
                hp::atel_apply_polar_move(r, w, f);
                0
            },
        );
        moves += 1;
    }
    eprintln!("round 25e: {turns} turns, {opts} attachment points, {moves} polar moves ({what})");
}

#[test]
fn round25e_turns_stream_game_over_seps_match_the_original() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut rng = Xorshift(0x5eed_0255);
    {
        let mut a = Oracle::load(&exe).unwrap();
        let mut b = Oracle::load(&exe).unwrap();
        r25e_cases(&mut a, &mut b, &mut rng, "image");
    }
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; the snapshot part skipped");
        return;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    let pair = || {
        (
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        )
    };
    let (mut a, mut b) = pair();
    r25e_cases(&mut a, &mut b, &mut rng, "snapshot");
    let (mut a, mut b) = pair();
    r25e_game_over(&mut a, &mut b, &mut rng, "snapshot");
    let (mut a, mut b) = pair();
    r25e_workers(&mut a, &mut b, &mut rng, "snapshot");
}
