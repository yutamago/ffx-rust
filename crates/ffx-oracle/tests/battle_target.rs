//! `MsSetTargetDir` and the chain under it (`ffx_battle::target`) against
//! the original, as in `battle.rs`: actors of random bytes with fake
//! characters and a position file on the image alone and on both
//! snapshots. The C runtime's trigonometry is the shared stand-ins
//! (`battle_common::crt`); script's and menu's callees run as the
//! originals in both oracles.
//!
//! The move functions leave their caller's saved registers in VU register
//! words (`ffx_battle::target`): the Rust gets the harness's
//! ([`HARNESS`]) for the top-level calls, the chain's own inside it; the
//! words holding a local's address are copied ([`local_words`]).

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::crt::{f80_to_f32, patch, stand_in};
use battle_common::world::{ACTOR, ACTORS, World, world};
use battle_common::*;
use ffx_battle::target::bridge as t;
use ffx_oracle::{Conv, Oracle};

fn call(m: &mut Oracle, sym: u32, args: &[u32]) -> ffx_oracle::Ret {
    let at = m.rebase(sym);
    m.call(at, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{sym:#x} {args:x?}: {e}"))
}

fn host() -> t::Host<'static> {
    fn spline(m: &mut t::Dm<'_>, out: u32, points: u32, n: u32, x: u32) {
        // the output is a frame local: through scratch in the oracle
        let o = oracle(m);
        let s = SCRATCH.with(std::cell::Cell::get);
        call(o, 0x0088_a4a0, &[s, points, n, x]);
        let v = o.read(s, 16).unwrap();
        o.write(s, &[0; 16]).unwrap();
        let w: [u32; 4] =
            std::array::from_fn(|k| u32::from_le_bytes(v[4 * k..4 * k + 4].try_into().unwrap()));
        ffx_bridge::Mem::write(m, out, w);
    }
    fn total(m: &mut t::Dm<'_>) -> u32 {
        call(oracle(m), 0x0086_c510, &[]).eax
    }
    fn top(m: &mut t::Dm<'_>) -> u32 {
        call(oracle(m), 0x0086_c1a0, &[]).eax
    }
    fn is_bind(m: &mut t::Dm<'_>, i: u32) -> u32 {
        call(oracle(m), 0x0086_d950, &[i]).eax
    }
    fn dir(m: &mut t::Dm<'_>, i: u32) -> u32 {
        f80_to_f32(call(oracle(m), 0x0086_a6e0, &[i]).st0_raw.unwrap())
    }
    fn pos(m: &mut t::Dm<'_>, i: u32, x: u32, y: u32, z: u32) {
        // the outputs may be frame locals: through scratch in the oracle
        let o = oracle(m);
        let s = SCRATCH.with(std::cell::Cell::get);
        call(o, 0x0086_b1e0, &[i, s, s + 4, s + 8]);
        let v = o.read(s, 12).unwrap();
        o.write(s, &[0; 12]).unwrap();
        let w: Vec<u32> = v
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        ffx_bridge::Mem::write(m, x, w[0]);
        ffx_bridge::Mem::write(m, y, w[1]);
        ffx_bridge::Mem::write(m, z, w[2]);
    }
    t::Host {
        trig: &stand_in,
        spline: &spline,
        atel_total: &total,
        atel_top: &top,
        atel_is_bind: &is_bind,
        atel_dir: &dir,
        atel_pos: &pos,
    }
}

thread_local! {
    /// 16 bytes in both oracles for the callbacks' outputs (cleared after).
    static SCRATCH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static ORACLE: std::cell::Cell<*mut Oracle> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// The oracle under the [`t::Dm`] the callbacks get (set by [`dm`]).
fn oracle(_: &mut t::Dm<'_>) -> &'static mut Oracle {
    // SAFETY: set to the oracle the running call borrows; the callbacks
    // only run inside that call, on this thread, and touch memory outside
    // the frames only.
    unsafe { &mut *ORACLE.with(std::cell::Cell::get) }
}

fn dm<R>(b: &mut Oracle, f: impl FnOnce(&mut t::Dm<'_>) -> R) -> R {
    ORACLE.with(|c| c.set(std::ptr::from_mut(b)));
    let r = f(&mut t::Dm::new(b));
    ORACLE.with(|c| c.set(std::ptr::null_mut()));
    r
}

/// A float: rarely NaN (the emulator gives `st0`'s NaN where the x87 gives
/// the one of larger significand, so two NaNs must not meet).
fn float(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => no_nan(rng.next()),
        1 => 0,
        _ => (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 37.0).to_bits(),
    }
}

/// A word that is no NaN (and so no signalling one).
fn no_nan(w: u32) -> u32 {
    if w & 0x7f80_0000 == 0x7f80_0000 && w & 0x7f_ffff != 0 {
        w & 0xff7f_ffff
    } else {
        w
    }
}

fn floats(rng: &mut Xorshift, n: usize) -> Vec<u8> {
    (0..n).flat_map(|_| float(rng).to_le_bytes()).collect()
}

struct Fakes {
    chars: u32,
    matrix: u32,
    file: u32,
}

fn fakes(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> Fakes {
    let chars = scratch(a, b, 0x3000);
    let matrix = scratch(a, b, 0x40);
    let file = scratch(a, b, 0x800);
    let _ = rng;
    Fakes {
        chars,
        matrix,
        file,
    }
}

/// A position file of one group (not flat), every table of 4 positions.
fn file(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, f: &Fakes) {
    let mut v = floats(rng, 0x200);
    v[0] = 0;
    v[1] = 1;
    for c in [2, 3, 4, 5, 6, 9] {
        v[c] = 4;
    }
    // each table its own positions
    for (k, o) in [0x10usize, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x2c]
        .into_iter()
        .enumerate()
    {
        let at = 0x100 + 0x40 * u32::try_from(k).unwrap();
        v[o..o + 4].copy_from_slice(&at.to_le_bytes());
    }
    poke(a, b, f.file, &v);
    set(a, b, ffx_battle::pos::bridge::POS_FILE, f.file);
}

/// Random actors whose positions are always written (characters, no
/// rides, battle positions the file has), no move kind 12 (it
/// asks its targets' move positions, which could recurse forever).
fn dress(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, f: &Fakes) {
    let v = floats(rng, 0xc00);
    poke(a, b, f.chars, &v);
    let m = floats(rng, 0x10);
    poke(a, b, f.matrix, &m);
    for c in 0..3 {
        let p = if rng.below(2) == 0 { 0 } else { f.matrix };
        poke(a, b, f.chars + c * 0x1000 + 0x10c, &p.to_le_bytes());
        // a rotation Ch_GetRot's wrap ends on (huge ones loop for ever)
        let r = (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 97.0).to_bits();
        poke(a, b, f.chars + c * 0x1000 + 0x158, &r.to_le_bytes());
    }
    for i in 0..ACTORS {
        let p = w.actor(i);
        // random words without NaNs (see float)
        let words: Vec<u8> = (0..ACTOR / 4)
            .flat_map(|_| no_nan(rng.next()).to_le_bytes())
            .collect();
        poke(a, b, p, &words);
        let chr = f.chars + rng.below(3) * 0x1000;
        poke(a, b, p, &chr.to_le_bytes());
        poke(a, b, p + 0xc, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        poke(a, b, p + 0x14, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        poke(a, b, p + 0x416, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        poke(a, b, p + 0xded, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        poke(a, b, p + 0x43f, &[0xff]);
        // a direction index into the direction pairs (a wild one reads
        // other fields, the 0xff above making NaNs that meet)
        poke(a, b, p + 0x511, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(
            a,
            b,
            p + 0x4fc,
            &[0, 3, u8::try_from(rng.below(4)).unwrap()],
        );
        poke(a, b, p + 0x6d4, &[if rng.below(3) == 0 { 1 } else { 0xff }]);
        poke(a, b, p + 0x6dd, &[0]);
        // move kinds whose target MsGetMoveTargetPos writes (with 16 and
        // above 0x12 MsGetChrMovePos moves towards stack garbage), and 7
        let kind = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 17, 18]
            [rng.below(16) as usize]
            | if rng.below(3) == 0 { 0x80 } else { 0 };
        poke(a, b, p + 0x415, &[kind]);
        for o in [0x411, 0x417, 0x418, 0x425, 0x505, 0x510, 0x34] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
        poke(a, b, p + 0x40c, &[u8::try_from(rng.below(30)).unwrap()]);
        for o in [0x452, 0x454, 0x456, 0x458, 0x45a] {
            let v = u16::try_from(rng.below(12)).unwrap();
            poke(a, b, p + o, &v.to_le_bytes());
        }
        poke(
            a,
            b,
            p + 0x454,
            &u16::try_from(rng.below(4)).unwrap().to_le_bytes(),
        );
        for (o, n) in [
            (0x390, 4),
            (0x3a4, 2),
            (0x3b0, 4),
            (0x3c0, 4),
            (0x3d0, 4),
            (0x45c, 4),
            (0x488, 1),
            (0x498, 1),
            (0x49c, 1),
            (0x4a0, 2),
            (0x4e8, 2),
            (0x514, 8),
            (0x534, 3),
            (0x53c, 1),
            (0xf70, 1),
        ] {
            poke(a, b, p + o, &floats(rng, n));
        }
        let targets = match rng.below(3) {
            0 => 1 << i,
            1 => rng.next() & 0x7fff_ffff,
            _ => 1 << rng.below(ACTORS),
        };
        poke(a, b, p + 0xdc0, &u32::to_le_bytes(targets));
    }
    // the move splines: 3-8 points each (of 2 or fewer at the ends,
    // TkMtCalcSpline writes nothing and the original adds stack garbage)
    let sp = t::MOVE_SPLINES.ptr(b).addr;
    for k in 0..4 {
        let at = sp + k * 0x210;
        poke(a, b, at, &(3 + rng.below(6)).to_le_bytes());
        poke(a, b, at + 0x10, &floats(rng, 0x80));
    }
    set(a, b, t::MOVE_FRAME, rng.range(-2, 40));
    set(
        a,
        b,
        t::SUMMONER,
        i8::try_from(rng.range(-1, 0x12)).unwrap(),
    );
    set(a, b, t::CAMERA_KO_HEIGHT, u8::from(rng.below(2) == 0));
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        i8::from(rng.below(3) == 0),
    );
}

/// The harness's callee-saved registers, as the originals called from it
/// see them (the oracle's `EBX`, `ESI`, `EDI`).
const HARNESS: t::Regs = t::Regs {
    edi: 0x3333_3333,
    esi: 0x2222_2222,
    ebx: 0x1111_1111,
};

/// The VU words that hold a caller's `edi` when it is the address of a
/// local (the original's stack, the Rust's frames): `VF_B.y` and `.z`
/// (the move's `MsMulVector`, `MsGetChrInfo`'s `MsDivVector`), `VF2.z`
/// (`MsVectorNormal2`'s scale), `VF4.z` (`MsVectorPolar2Rect`'s radius).
/// Copied from `a`; the rest compares.
fn local_words(a: &mut Oracle, b: &mut Oracle) {
    let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    let vb = ffx_battle::vector::bridge::VF_B.ptr(a).addr;
    for at in [vb + 4, vb + 8, m + 0x28, m + 0x48] {
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32, script: bool) {
    let h = host();
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    let vals = floats(rng, 0x20);
    poke(a, b, out, &vals);
    twin(
        "MsGetChrBottomPos",
        a,
        b,
        0x0079_4830,
        &[i, out],
        ALL,
        |m| t::ms_get_chr_bottom_pos(m, &stand_in, i, out).cast_unsigned(),
    );
    twin(
        "MsGetChrCameraPos",
        a,
        b,
        0x0079_4ab0,
        &[i, out + 0x10],
        ALL,
        |m| t::ms_get_chr_camera_pos(m, &stand_in, i, out + 0x10).cast_unsigned(),
    );
    same("positions", a, b);
    let d = float(rng);
    let ra = call(a, 0x0079_58c0, &[i, d]);
    let rb = t::ms_get_chr_near_dir(b, i, d).to_bits();
    assert_eq!(f80_to_f32(ra.st0_raw.unwrap()), rb, "MsGetChrNearDir");
    let angle = float(rng);
    twin(
        "MsTurnMoveVector",
        a,
        b,
        0x007a_b190,
        &[i, actor, out, angle],
        0,
        |m| {
            dm(m, |m| t::ms_turn_move_vector(m, &h, i, actor, out, angle));
            0
        },
    );
    local_words(a, b);
    same("MsTurnMoveVector", a, b);
    // one actor's property, kind 12 included (inside the chain)
    let kind = rng.below(15);
    let args = [
        i,
        kind,
        rng.below(4),
        float(rng),
        if rng.below(4) == 0 { 0 } else { out },
    ];
    twin("MsGetChrInfoOne", a, b, 0x0079_4060, &args, ALL, |m| {
        dm(m, |m| {
            t::ms_get_chr_info_one(m, &h, args[0], args[1], args[2], args[3], args[4])
        })
    });
    local_words(a, b);
    same("MsGetChrInfoOne", a, b);
    // a set's, script actors' (positions only: the others add stack garbage)
    let bits = [rng.next() & 0x7fff_ffff, 1 << i, 0][rng.below(3) as usize];
    // script actors only where the script runs (a snapshot)
    let kind = if script && rng.below(6) == 0 {
        0x100 | [1, 8, 9][rng.below(3) as usize]
    } else {
        rng.below(14)
    };
    let sel = [
        (-21i32).cast_unsigned(),
        (-5i32).cast_unsigned(),
        (-2i32).cast_unsigned(),
        rng.below(4),
    ][rng.below(4) as usize];
    let args = [
        kind,
        bits,
        if kind & 0x100 != 0 { sel } else { rng.below(4) },
        out,
    ];
    twin("MsGetChrInfo", a, b, 0x0079_5170, &args, ALL, |m| {
        dm(m, |m| {
            t::ms_get_chr_info(
                m,
                &h,
                args[0],
                args[1],
                args[2],
                args[3],
                Some([HARNESS.edi, HARNESS.ebx]),
            )
        })
    });
    local_words(a, b);
    same("MsGetChrInfo", a, b);
    let args = [
        i,
        rng.below(14),
        out,
        out + 0x10,
        if rng.below(3) == 0 { 0 } else { out + 0x14 },
    ];
    twin("MsGetChrTargetInfo", a, b, 0x007a_a050, &args, ALL, |m| {
        dm(m, |m| {
            t::ms_get_chr_target_info(m, &h, i, args[1], out, out + 0x10, args[4]).cast_unsigned()
        })
    });
    local_words(a, b);
    same("MsGetChrTargetInfo", a, b);
    // the move functions from outside the chain
    let mkind = rng.below(0x14) | if rng.below(4) == 0 { 0x80 } else { 0 };
    let done = if rng.below(4) == 0 { 0 } else { out + 0x30 };
    let args = [i, actor, mkind, out, done];
    let ra = call(a, 0x007a_9ac0, &args);
    let rb = dm(b, |m| {
        t::ms_get_move_target_pos(m, &h, i, actor, mkind, out, done, Some(HARNESS))
    });
    assert_eq!(
        f80_to_f32(ra.st0_raw.unwrap()),
        rb,
        "MsGetMoveTargetPos {args:x?}"
    );
    local_words(a, b);
    same("MsGetMoveTargetPos", a, b);
    let n = rng.range(-2, 40).cast_unsigned();
    let speed = float(rng);
    let args = [actor, n, speed, out + 0x10, out];
    twin("MsGetChrMoveAccelPos", a, b, 0x007a_ac40, &args, ALL, |m| {
        dm(m, |m| {
            t::ms_get_chr_move_accel_pos(
                m,
                &stand_in,
                actor,
                n,
                speed,
                out + 0x10,
                out,
                Some(HARNESS),
            )
        })
    });
    local_words(a, b);
    same("MsGetChrMoveAccelPos", a, b);
    twin("MsGetChrMovePos", a, b, 0x007a_93e0, &[i, out], 0, |m| {
        dm(m, |m| t::ms_get_chr_move_pos(m, &h, i, out, Some(HARNESS)));
        0
    });
    local_words(a, b);
    same("MsGetChrMovePos", a, b);
    // Rikku's Mix: a cue entry of items
    let cue = out + 0x40;
    let mut c = vec![0u8; 0x40];
    c[3] = u8::try_from(rng.below(3)).unwrap();
    for k in 0..4usize {
        let id: u16 = match rng.below(5) {
            0 => 0xff,
            1 => [0x311e, 0x3001][rng.below(2) as usize],
            _ => u16::try_from(0x2000 + rng.below(0x40)).unwrap(),
        };
        let at = 8 + (k / 2) * 16 + (k % 2) * 2;
        c[at..at + 2].copy_from_slice(&id.to_le_bytes());
    }
    poke(a, b, cue, &c);
    for o in [0xdfd, 0x717, 0x435] {
        poke(a, b, actor + o, &[u8::from(rng.below(4) == 0)]);
    }
    twin(
        "MsSetRikkuLimit",
        a,
        b,
        0x007b_0520,
        &[i, actor, cue],
        ALL,
        |m| dm(m, |m| t::ms_set_rikku_limit(m, &h, i, actor, cue)),
    );
    local_words(a, b);
    same("MsSetRikkuLimit", a, b);
    let targets =
        [rng.next() & 0x7fff_ffff, 1 << i, 0, 1 << rng.below(ACTORS)][rng.below(4) as usize];
    twin("MsSetTargetDir", a, b, 0x0079_69f0, &[i, targets], 0, |m| {
        dm(m, |m| t::ms_set_target_dir(m, &h, i, targets));
        0
    });
    local_words(a, b);
    same("MsSetTargetDir", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    patch(a);
    patch(b);
    let w = world(a, b, rng, synthetic);
    let f = fakes(a, b, rng);
    if synthetic {
        // a mix table (MsGetExcelData's layout) of 0x40 records of 0x80
        // bytes: item IDs
        let mut t = vec![0u8; 0x20 + 0x40 * 0x80];
        t[0] = 1;
        t[10..12].copy_from_slice(&0x3fu16.to_le_bytes());
        t[12..14].copy_from_slice(&0x80u16.to_le_bytes());
        t[16..20].copy_from_slice(&0x20u32.to_le_bytes());
        for k in 0..0x40 * 0x40 {
            let id = u16::try_from(
                [0x2000 + rng.below(0x40), 0x3000 + rng.below(0x40)][rng.below(2) as usize],
            )
            .unwrap();
            t[0x20 + 2 * k..0x22 + 2 * k].copy_from_slice(&id.to_le_bytes());
        }
        let at = t::MIX_TABLE.ptr(b).addr;
        poke(a, b, at, &t);
    }
    // what a learned mix refreshes (MsSetSaveCommand): 0x140 commands
    // and the menu lists
    if w.synthetic {
        let c = battle_common::world::fake_records(a, b, rng, 0x140, 0x60);
        set(a, b, ffx_kernel::bridge::COMMAND, c);
        let m = scratch(a, b, 0x5400);
        set(a, b, ffx_battle::misc::bridge::ITEM_MEM2, m);
    }
    let out = scratch(a, b, 0x80);
    let s = scratch(a, b, 0x10);
    SCRATCH.with(|c| c.set(s));
    for _ in 0..400 {
        file(a, b, rng, &f);
        dress(a, b, rng, &w, &f);
        calls(a, b, rng, &w, out, !synthetic);
    }
}

#[test]
fn targets_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7a46_0001), true);
}

#[test]
fn targets_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7a46_0002), false);
}
