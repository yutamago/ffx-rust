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

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};
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

/// `EAX` of the original at `f` (an `ffx.sym` address).
fn orig(a: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    a.call(a.rebase(f), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x}{args:x?}: {e}"))
        .eax
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

fn no_diff(a: &mut Oracle, b: &mut Oracle, what: &str) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
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
                    assert_eq!(
                        ra,
                        v::atel_fetch_stack_integer(&mut b, pop, s),
                        "{what}: fetch"
                    );
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
