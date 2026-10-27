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
        spline_via(m, 0x0088_a4a0, out, points, n, x);
    }
    fn tangent(m: &mut t::Dm<'_>, out: u32, points: u32, n: u32, x: u32) {
        spline_via(m, 0x0088_b3b0, out, points, n, x);
    }
    fn spline3(m: &mut t::Dm<'_>, out: u32, points: u32, n: u32, x: u32) {
        spline_via(m, 0x0088_a4d0, out, points, n, x);
    }
    fn spline3_tangent(m: &mut t::Dm<'_>, out: u32, points: u32, n: u32, x: u32) {
        spline_via(m, 0x0088_aba0, out, points, n, x);
    }
    fn near_point(m: &mut t::Dm<'_>, a: [u32; 8]) -> u32 {
        // the outputs are frame locals: through scratch in the oracle
        let o = oracle(m);
        let s = SCRATCH.with(std::cell::Cell::get);
        let r = call(
            o,
            0x00a4_4120,
            &[a[0], a[1], a[2], a[3], s, s + 4, s + 8, a[7]],
        );
        let v = o.read(s, 12).unwrap();
        o.write(s, &[0; 12]).unwrap();
        for k in 0..3 {
            let w = u32::from_le_bytes(v[4 * k..4 * k + 4].try_into().unwrap());
            ffx_bridge::Mem::write(m, a[4 + k], w);
        }
        f80_to_f32(r.st0_raw.unwrap())
    }
    fn neck_enable(m: &mut t::Dm<'_>, chr: u32, on: u32) {
        call(oracle(m), 0x0083_4f60, &[chr, on]);
    }
    fn neck_target(m: &mut t::Dm<'_>, chr: u32, to: u32) {
        call(oracle(m), 0x0083_5050, &[chr, to]);
    }
    fn spline_via(m: &mut t::Dm<'_>, f: u32, out: u32, points: u32, n: u32, x: u32) {
        // the output is a frame local: through scratch in the oracle,
        // holding its words first (some counts write nothing)
        let w: [u32; 4] = ffx_bridge::Mem::read(m, out);
        let o = oracle(m);
        let s = SCRATCH.with(std::cell::Cell::get);
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        o.write(s, &bytes).unwrap();
        call(o, f, &[s, points, n, x]);
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
        spline_tangent: &tangent,
        spline3: &spline3,
        spline3_tangent: &spline3_tangent,
        near_point: &near_point,
        cos: &cos_stand_in,
        neck_enable: &neck_enable,
        neck_target: &neck_target,
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
    /// The round 28 sector checks' own stream (the later checks keep
    /// theirs).
    static DIVIDE_RNG: std::cell::Cell<u32> = const { std::cell::Cell::new(0x28d1_0001) };
    /// 0x20 bytes in both oracles for a script signal block (cleared after).
    static BLOCK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// 0x80 bytes in both oracles for the target lists' frame words and
    /// count (round 32).
    static LISTS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// The battle snapshot's character pointers, before the actors are
    /// randomized (`MsAttachWeapon` binds to their real models).
    static LIVE_CHRS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
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
///
/// `MsGetDirMode`'s position (`MsGetChrPos` into a local) leaves its `w`
/// unwritten: the original's stack garbage reaches the `w` words of
/// `VF_B`'s neighbour, `VF0` and `VF2` through `MsVectorRect2Polar`, so
/// those are copied too.
/// [`battle_common::same`] but for the emulated a0 (`0x00c8f548`),
/// copied from `a`: `op_3d_vol_pan_get` (a sound's pan, `MsSEP3Dregist`)
/// leaves the address of its caller's volume there, a stack local in the
/// original and the sound node's `+4` in the port (as `tests/vfx.rs`).
fn same(what: &str, a: &mut Oracle, b: &mut Oracle) {
    let a0 = ffx_battle::vector::bridge::VF_M.ptr(a).addr - 0x288;
    let w = a.read(a0, 4).unwrap();
    b.write(a0, &w).unwrap();
    battle_common::same(what, a, b);
}

fn local_words(a: &mut Oracle, b: &mut Oracle) {
    let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    let vb = ffx_battle::vector::bridge::VF_B.ptr(a).addr;
    for at in [
        vb + 4,
        vb + 8,
        m + 0x28,
        m + 0x48,
        vb - 4,
        m + 0xc,
        m + 0x2c,
    ] {
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
}

/// The sector checks' facing local without a model: 5.0 (beyond pi,
/// so `MsNearRag` turns it) in the original's whole stack and the Rust's
/// two words at `local`.
fn facing_stack(a: &mut Oracle, b: &mut Oracle, local: u32) {
    let r = a.stack_range();
    let five: Vec<u8> = (r.start..r.end)
        .step_by(4)
        .flat_map(|_| 5.0f32.to_le_bytes())
        .collect();
    a.write(r.start, &five).unwrap();
    b.write(local, &five[..8]).unwrap();
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
    // the turn's sectors (round 28): the facing's local 5.0 in both
    // ([`facing_stack`]: an actor without a model leaves it as it was),
    // sector counts of either sign (never 0: the original's divide
    // faults), points apart, together or a hair apart, a battle direction
    // of the slots around +0x514
    let mut dr = Xorshift(DIVIDE_RNG.with(std::cell::Cell::get));
    let slot = a.read(actor + 0x511, 1).unwrap();
    let local = a.stack_range().end - 0x200;
    facing_stack(a, b, local);
    // (and one an f32 cannot hold)
    let n = [
        1,
        2,
        4,
        8,
        0x10,
        1 + dr.below(100),
        (-3i32).cast_unsigned(),
        0x0100_0003,
    ][dr.below(8) as usize];
    let mode = [0, 1, dr.next()][dr.below(3) as usize];
    poke(
        a,
        b,
        actor + 0x511,
        &[[0u8, 1, 2, 3, 0xff][dr.below(5) as usize]],
    );
    twin(
        "MsGetBasicDivide",
        a,
        b,
        0x007a_8720,
        &[i, n, mode],
        ALL,
        |m| ffx_battle::body::bridge::ms_get_basic_divide(m, i, n, mode, local),
    );
    same("MsGetBasicDivide", a, b);
    let (from, to) = (out + 0x20, out + 0x30);
    match dr.below(6) {
        0 => {
            let p = a.read(from, 16).unwrap();
            poke(a, b, to, &p);
        }
        // a tiny but nonzero distance
        1 => {
            let mut p = a.read(from, 16).unwrap();
            let x = f32::from_le_bytes(p[..4].try_into().unwrap());
            p[..4].copy_from_slice(&(x + 1e-4 * x.abs().max(1.0)).to_le_bytes());
            poke(a, b, to, &p);
        }
        _ => {}
    }
    facing_stack(a, b, local);
    twin(
        "MsGetDirDivide",
        a,
        b,
        0x007a_87f0,
        &[from, to, i, n, mode],
        ALL,
        |m| ffx_battle::body::bridge::ms_get_dir_divide(m, &stand_in, from, to, i, n, mode, local),
    );
    same("MsGetDirDivide", a, b);
    poke(a, b, actor + 0x511, &slot);
    DIVIDE_RNG.with(|c| c.set(dr.0));
    let d = float(rng);
    let ra = call(a, 0x0079_58c0, &[i, d]);
    let rb = t::ms_get_chr_target_dir(b, i, d).to_bits();
    assert_eq!(f80_to_f32(ra.st0_raw.unwrap()), rb, "MsGetChrTargetDir");
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

/// The game's code under a [`t::Dm`] (script, vfx and chara callees).
/// [`game_call`] on an oracle itself.
fn game_call_o(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

fn game_call(m: &mut t::Dm<'_>, f: u32, args: &[u32]) -> u32 {
    oracle(m)
        .call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

/// `AtelRegistSignal2` with the block in [`BLOCK`].
fn game_signal(m: &mut t::Dm<'_>, words: [u32; 8]) -> u32 {
    let block = BLOCK.with(std::cell::Cell::get);
    let o = oracle(m);
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    o.write(block, &bytes).unwrap();
    let at = o.rebase(0x0086_e9e0);
    let r = o.call(at, Conv::Cdecl, &[block]).unwrap().eax;
    o.write(block, &[0; 0x20]).unwrap();
    r
}

fn eff_off(m: &mut t::Dm<'_>, e: u32) {
    ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
}

/// Directions and the status chain (`ffx_battle::status`): the direction
/// modes, the status effects, and the motions statuses and hits play.
fn status(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::status::bridge as st;
    let h = host();
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    for (o, n) in [(0x490, 2), (0x4f4, 1), (0x3e0, 4)] {
        poke(a, b, actor + o, &floats(rng, n));
    }
    poke(a, b, actor + 0x42b, &[u8::try_from(rng.below(15)).unwrap()]);
    let t = if rng.below(2) == 0 {
        0xff
    } else {
        u8::try_from(rng.below(ACTORS)).unwrap()
    };
    poke(a, b, actor + 0x431, &[t]);
    for o in [0x42c, 0x42d, 0x41b] {
        poke(a, b, actor + o, &[u8::from(rng.below(3) == 0)]);
    }
    set(
        a,
        b,
        t::COMMAND_ACTOR,
        u8::try_from(rng.below(ACTORS)).unwrap(),
    );
    let d = float(rng);
    ftwin("MsDirTurn", a, b, 0x007a_9650, &[actor, d], |m| {
        t::ms_dir_turn(m, actor, d)
    });
    same("MsDirTurn", a, b);
    let vec = out + 0x60;
    poke(a, b, vec, &floats(rng, 4));
    let follow = rng.below(2);
    let (mo, po) = if rng.below(4) == 0 {
        (0, 0)
    } else {
        (out + 0x70, out + 0x74)
    };
    let args = [i, actor, vec, follow, mo, po];
    ftwin("MsGetDirMode", a, b, 0x007a_9710, &args, |m| {
        dm(m, |m| {
            t::ms_get_dir_mode(m, &h, i, actor, vec, follow, mo, po)
        })
    });
    local_words(a, b);
    same("MsGetDirMode", a, b);
    let args = [
        i,
        rng.below(15),
        rng.below(2),
        rng.next(),
        rng.range(-2, 4).cast_unsigned(),
        rng.range(-2, 4).cast_unsigned(),
    ];
    twin("MsSetChrDirTarget", a, b, 0x0079_5eb0, &args, ALL, |m| {
        dm(m, |m| {
            t::ms_set_chr_dir_target(m, &h, args[0], args[1], args[2], args[3], args[4], args[5])
        })
        .cast_unsigned()
    });
    local_words(a, b);
    same("MsSetChrDirTarget", a, b);
    // status effects (op_et_eff and op_et_astatus_effect_change stubbed in
    // both; the effect's end is vfx's port against the original)
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    for o in [0x606, 0x616] {
        poke(
            a,
            b,
            actor + o,
            &u16::try_from(rng.below(0x10000)).unwrap().to_le_bytes(),
        );
    }
    let counts: Vec<u8> = (0..13)
        .map(|_| [0, 1, 2, 0xfe, 0xff][rng.below(5) as usize])
        .collect();
    poke(a, b, actor + 0x608, &counts);
    for o in [0xdcd, 0xdce, 0x613, 0x614, 0xdcb, 0x63c, 0xdc8] {
        poke(a, b, actor + o, &[u8::from(rng.below(3) == 0)]);
    }
    poke(a, b, actor + 0x63e, &[u8::try_from(rng.below(2)).unwrap()]);
    poke(a, b, actor + 0x63f, &[u8::try_from(rng.below(2)).unwrap()]);
    let slots: Vec<u8> = (0..24)
        .flat_map(|_| if rng.below(2) == 0 { 0 } else { rng.next() }.to_le_bytes())
        .collect();
    poke(a, b, actor + 0xe30, &slots);
    poke(a, b, actor + 0x6fc, &rng.below(0x40).to_le_bytes());
    let rec = st::STATUS_COUNT_EFFECTS.ptr(b).at(rng.below(13)).addr;
    let on = rng.below(3);
    twin(
        "MsStatusEffect",
        a,
        b,
        0x0079_ef40,
        &[i, actor, rec, on, actor + 0xe30],
        0,
        |m| {
            dm(m, |m| {
                st::ms_status_effect(m, &mut call, &mut off, i, actor, rec, on, actor + 0xe30);
            });
            0
        },
    );
    same("MsStatusEffect", a, b);
    twin("MsStatusEffectCheck", a, b, 0x0079_eff0, &[i], 0, |m| {
        dm(m, |m| st::ms_status_effect_check(m, &mut call, &mut off, i));
        0
    });
    same("MsStatusEffectCheck", a, b);
    twin("MsGetStatusLook", a, b, 0x0079_e7c0, &[i], ALL, |m| {
        st::ms_get_status_look(m, i)
    });
    twin(
        "MsCheckAutoStatus",
        a,
        b,
        0x007a_f260,
        &[i, actor],
        ALL,
        |m| {
            dm(m, |m| {
                st::ms_check_auto_status(m, &mut call, &mut off, i, actor)
            })
        },
    );
    same("MsCheckAutoStatus", a, b);
    if rng.below(8) == 0 {
        twin(
            "MsCheckStatusAfterInput",
            a,
            b,
            0x007a_f4b0,
            &[],
            ALL,
            |m| {
                dm(m, |m| {
                    st::ms_check_status_after_input(m, &mut call, &mut off)
                })
            },
        );
        same("MsCheckStatusAfterInput", a, b);
    }
    // the motions: movers in range, the weapon on the battle snapshot's
    // real models
    // actor i moves itself; others are never put back (MsCheckBtlPos
    // compares the original's stack garbage when a battle position is not
    // written, battle_pos.rs): actor i only when its position is
    for k in 0..ACTORS {
        let q = w.actor(k);
        poke(a, b, q + 0x422, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        poke(a, b, q + 0x43f, &[0]);
        // no monster files (the random words would be read as handlers)
        poke(a, b, q + 0xf7c, &[0; 4]);
    }
    poke(a, b, actor + 0x422, &[u8::try_from(i).unwrap()]);
    let probe = out + 0x40;
    poke(a, b, probe, &[0xff; 16]);
    twin("MsGetChrBtlPos", a, b, 0x0079_48e0, &[i, probe], ALL, |m| {
        ffx_battle::pos::bridge::ms_get_chr_btl_pos(m, &stand_in, i, probe).cast_unsigned()
    });
    same("MsGetChrBtlPos", a, b);
    let written = b.read(probe, 16).unwrap().iter().all(|&x| x != 0xff);
    poke(a, b, actor + 0x43f, &[if written { 0xff } else { 0 }]);
    for o in [0x436, 0x4b5, 0x6da, 0x41a, 0x43e, 0x426, 0x4ac] {
        poke(a, b, actor + o, &[u8::from(rng.below(3) == 0)]);
    }
    poke(
        a,
        b,
        actor + 0x4b6,
        &[u8::from(rng.below(2) == 0), u8::from(rng.below(2) == 0)],
    );
    let live = LIVE_CHRS.with(|c| c.borrow().get(i as usize).copied());
    let fake = b.read(actor, 4).unwrap();
    if let Some(c) = live {
        poke(a, b, actor, &c.to_le_bytes());
    }
    // actor 6's arm (its +4 7: the model part a weapon replaces)
    let six = w.actor(6);
    let kind6 = if rng.below(2) == 0 { 7 } else { rng.below(9) };
    poke(a, b, six + 4, &kind6.to_le_bytes());
    let hide = rng.below(3);
    let who = [6, 6, rng.below(ACTORS)][rng.below(3) as usize];
    twin("MsHideArm", a, b, 0x0079_5c00, &[who, hide], 0, |m| {
        dm(m, |m| st::ms_hide_arm(m, &mut call, who, hide));
        0
    });
    same("MsHideArm", a, b);
    let k = rng.below(2);
    twin("MsAttachWeapon", a, b, 0x0079_3330, &[i, k], ALL, |m| {
        dm(m, |m| st::ms_attach_weapon(m, &h, &mut call, i, k))
    });
    local_words(a, b);
    same("MsAttachWeapon", a, b);
    // and let go again (round 27), at times the other weapon
    let k = if rng.below(3) == 0 { 1 - k } else { k };
    twin("MsDetachWeapon", a, b, 0x0079_3f10, &[i, k], ALL, |m| {
        dm(m, |m| st::ms_detach_weapon(m, &mut call, i, k))
    });
    local_words(a, b);
    same("MsDetachWeapon", a, b);
    poke(a, b, actor, &fake);
    // and on actors whose weapon has a character (the snapshot's party),
    // on their real models, ready to let go (+0x4b5)
    for _ in 0..3 {
        let j = rng.below(ACTORS);
        let k = rng.below(2);
        let wpn = ffx_battle::actor::bridge::ms_get_weapon(b, j, k.cast_signed());
        if b.read(wpn, 4).unwrap() == [0; 4] {
            continue;
        }
        let pj = w.actor(j);
        let fake = b.read(pj, 4).unwrap();
        if let Some(c) = LIVE_CHRS.with(|c| c.borrow().get(j as usize).copied()) {
            poke(a, b, pj, &c.to_le_bytes());
        }
        poke(
            a,
            b,
            pj + 0x4b5,
            &[1, u8::from(rng.below(2) == 0), u8::from(rng.below(2) == 0)],
        );
        if j == 6 && rng.below(2) == 0 {
            poke(a, b, pj + 4, &7u32.to_le_bytes());
        }
        twin("MsDetachWeapon", a, b, 0x0079_3f10, &[j, k], ALL, |m| {
            dm(m, |m| st::ms_detach_weapon(m, &mut call, j, k))
        });
        local_words(a, b);
        same("MsDetachWeapon", a, b);
        poke(a, b, pj, &fake);
    }
    // actor 6's arm shown again (+4 7): no snapshot gives it a weapon
    // character, so it lends one of the party's weapon models and, when it
    // has no live character, a party member's
    let donor = (0..ACTORS * 2).find_map(|n| {
        let wpn = ffx_battle::actor::bridge::ms_get_weapon(b, n / 2, (n % 2).cast_signed());
        let c = b.read(wpn, 4).unwrap();
        (c != [0; 4] && n / 2 != 6).then_some(c)
    });
    let lives: Vec<u32> =
        LIVE_CHRS.with(|c| c.borrow().iter().copied().filter(|&c| c != 0).collect());
    if let (Some(wchr), Some(&any)) = (donor, lives.first()) {
        // its own draw: the stream the later steps get stays as it was
        let k = Xorshift(i ^ 0x28d6).below(2);
        let six = w.actor(6);
        let wpn = ffx_battle::actor::bridge::ms_get_weapon(b, 6, k.cast_signed());
        let (fake, kept) = (b.read(six, 8).unwrap(), b.read(wpn, 4).unwrap());
        let flags = b.read(six + 0x4b5, 3).unwrap();
        // the lent model's place, turn and binding are put back after (its
        // owner still holds it)
        let lent = u32::from_le_bytes(wchr.clone().try_into().unwrap());
        let model = b.read(lent, 0x200).unwrap();
        let own = LIVE_CHRS
            .with(|c| c.borrow().get(6).copied())
            .filter(|&c| c != 0);
        poke(a, b, six, &own.unwrap_or(any).to_le_bytes());
        poke(a, b, six + 4, &7u32.to_le_bytes());
        poke(a, b, six + 0x4b5, &[1, 0, 0]);
        poke(a, b, wpn, &wchr);
        twin("MsDetachWeapon", a, b, 0x0079_3f10, &[6, k], ALL, |m| {
            dm(m, |m| st::ms_detach_weapon(m, &mut call, 6, k))
        });
        local_words(a, b);
        same("MsDetachWeapon", a, b);
        poke(a, b, six, &fake);
        poke(a, b, six + 0x4b5, &flags);
        poke(a, b, lent, &model);
        poke(a, b, wpn, &kept);
    }
    twin("MsResetChrPos", a, b, 0x0078_d680, &[i, actor], ALL, |m| {
        st::ms_reset_chr_pos(m, &stand_in, i, actor)
    });
    same("MsResetChrPos", a, b);
    // the hit motion may attach its weapon (MsAttachWeapon): on the
    // snapshot's real character, as above
    if let Some(c) = live {
        poke(a, b, actor, &c.to_le_bytes());
    }
    let kind = rng.below(18);
    let flags = rng.below(8);
    twin(
        "MsDamageSetMotion",
        a,
        b,
        0x0078_ca20,
        &[i, kind, flags],
        0,
        |m| {
            dm(m, |m| {
                let mut s = Script {
                    call: &mut call,
                    signal: &mut signal,
                    off: &mut off,
                };
                st::ms_damage_set_motion(m, &h, &mut s, i, kind, flags);
            });
            0
        },
    );
    local_words(a, b);
    same("MsDamageSetMotion", a, b);
    twin(
        "MsCheckStatusAfterAction",
        a,
        b,
        0x007a_f390,
        &[i, actor],
        ALL,
        |m| {
            dm(m, |m| {
                let mut s = Script {
                    call: &mut call,
                    signal: &mut signal,
                    off: &mut off,
                };
                st::ms_check_status_after_action(m, &h, &mut s, i, actor)
            })
        },
    );
    local_words(a, b);
    same("MsCheckStatusAfterAction", a, b);
    poke(a, b, actor, &fake);
}

/// Random input and command cues, the actors' cue fields in range.
fn cues(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    use ffx_battle::state::bridge as st;
    let n = rng.range(-1, 0x40);
    set(a, b, st::INPUT_CUE_COUNT, i8::try_from(n).unwrap());
    let n = rng.range(-1, 0x30);
    set(a, b, st::COMMAND_CUE_COUNT, i8::try_from(n).unwrap());
    let input: Vec<u8> = (0..0x40 * 8)
        .map(|k| match k % 8 {
            0 => u8::try_from(rng.below(ACTORS + 2)).unwrap(),
            1 => [0, 0, 1, rng.byte()][rng.below(4) as usize],
            _ => rng.byte(),
        })
        .collect();
    poke(a, b, st::INPUT_CUE.ptr(b).addr, &input);
    let commands: Vec<u8> = (0..0x30 * st::CUE_SIZE)
        .map(|k| match k % st::CUE_SIZE {
            0 => u8::try_from(rng.below(ACTORS + 2)).unwrap(),
            1 => [0, 1, rng.byte()][rng.below(3) as usize],
            _ => rng.byte(),
        })
        .collect();
    poke(a, b, st::COMMAND_CUE.ptr(b).addr, &commands);
    set(
        a,
        b,
        ffx_battle::action::bridge::SET_COMMAND_FLAG,
        rng.byte(),
    );
    for i in 0..ACTORS {
        let p = w.actor(i);
        let own = |rng: &mut Xorshift, n: u32| {
            if rng.below(4) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(n)).unwrap()
            }
        };
        let f = [own(rng, 0x40), own(rng, 0x30), rng.byte(), rng.byte()];
        poke(a, b, p + 0xde4, &f);
        poke(a, b, p + 0xde8, &[u8::try_from(rng.below(5)).unwrap()]);
        poke(a, b, p + 0x5ac, &[u8::try_from(rng.below(0x40)).unwrap()]);
        for o in [0xdf1, 0xdf8, 0x613, 0x614, 0xdcc, 0xdc8] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
    }
}

/// The action requests and the cue entries they make
/// (`ffx_battle::action`).
fn actions(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::action::bridge as ac;
    use ffx_battle::atel::bridge::Script;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    cues(a, b, rng, w);
    for k in 0..ACTORS {
        poke(a, b, w.actor(k) + 0x43f, &[0]);
        poke(a, b, w.actor(k) + 0xf7c, &[0; 4]);
    }
    // a cue entry of the action request's form
    let cue = out + 0x20;
    fill(a, b, rng, cue, 0x48);
    let chr = rng.below(ACTORS + 1);
    poke(a, b, cue, &[u8::try_from(chr).unwrap()]);
    let flag = rng.range(-1, 3).cast_unsigned();
    let gil = rng.range(-5, 9000);
    let args = [cue, gil.cast_unsigned(), flag];
    twin("MsSetBtlCommand", a, b, 0x0079_2d60, &args, ALL, |m| {
        dm(m, |m| {
            s!(s);
            ac::ms_set_btl_command(m, &h, &mut s, cue, gil, flag).cast_unsigned()
        })
    });
    local_words(a, b);
    same("MsSetBtlCommand", a, b);
    cues(a, b, rng, w);
    let prio = [0, 0x10, 0x25, rng.byte().into()][rng.below(4) as usize];
    let after = rng.below(ACTORS + 2);
    let args = [cue, prio, flag, after];
    twin("MsInsertBtlCommand", a, b, 0x0079_29b0, &args, ALL, |m| {
        dm(m, |m| {
            s!(s);
            ac::ms_insert_btl_command(m, &h, &mut s, cue, prio, flag, after).cast_unsigned()
        })
    });
    local_words(a, b);
    same("MsInsertBtlCommand", a, b);
    // an input entry ending
    cues(a, b, rng, w);
    let chr = rng.below(ACTORS);
    let n =
        u32::try_from(i32::from(ffx_battle::state::bridge::INPUT_CUE_COUNT.get(b)).max(1)).unwrap();
    let k = if rng.below(6) == 0 {
        0xff
    } else {
        rng.below(n)
    };
    if k != 0xff && rng.below(4) != 0 {
        poke(
            a,
            b,
            ffx_battle::state::bridge::INPUT_CUE.ptr(b).addr + 8 * k,
            &[u8::try_from(chr).unwrap()],
        );
        if rng.below(2) == 0 {
            poke(a, b, w.actor(chr) + 0xde4, &[u8::try_from(k).unwrap()]);
        }
    }
    let (last, keep) = (rng.below(2), rng.below(2));
    twin(
        "MsDeleteInputCue",
        a,
        b,
        0x007b_20e0,
        &[chr, k, last, keep],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                ac::ms_delete_input_cue(m, &h, &mut s, chr, k, last, keep)
            })
        },
    );
    local_words(a, b);
    same("MsDeleteInputCue", a, b);
    // action scripts (the request block random, which only a script
    // fills for real)
    cues(a, b, rng, w);
    fill(
        a,
        b,
        rng,
        ffx_battle::exec::bridge::ACTION_REQUEST_BLOCK.ptr(b).addr,
        0x48,
    );
    set(a, b, ffx_battle::exec::bridge::ACTION_REQUEST, rng.below(2));
    set(
        a,
        b,
        ffx_battle::exec::bridge::ACTION_REQUEST_STEP,
        rng.below(0x40),
    );
    let i = [rng.below(ACTORS), 0xe + rng.below(4), rng.next()][rng.below(3) as usize];
    let who = if rng.below(3) == 0 {
        0xff
    } else {
        rng.below(ACTORS)
    };
    let n = [rng.range(-2, 12), rng.range(0, 0x50)][rng.below(2) as usize];
    let mode = [0, 1, 2, u32::MAX][rng.below(4) as usize];
    let o = if rng.below(3) == 0 { out + 0x80 } else { 0 };
    let args = [i, who, n.cast_unsigned(), rng.below(2), mode, o];
    twin("MsActionRequest", a, b, 0x007a_ceb0, &args, ALL, |m| {
        dm(m, |m| {
            s!(s);
            ac::ms_action_request(m, &h, &mut s, i, who, n, args[3], mode, o).cast_unsigned()
        })
    });
    local_words(a, b);
    same("MsActionRequest", a, b);
    let bits = [rng.next() & 0x7fff_ffff, 1 << rng.below(ACTORS), 0][rng.below(3) as usize];
    let mode = [0, 1, 2][rng.below(3) as usize];
    let n = rng.range(-1, 12);
    let args = [bits, who, rng.next(), rng.next(), n.cast_unsigned(), mode];
    twin("MsActionTarget", a, b, 0x007a_d0c0, &args, ALL, |m| {
        dm(m, |m| {
            s!(s);
            ac::ms_action_target(m, &h, &mut s, bits, who, n, mode)
        })
    });
    local_words(a, b);
    same("MsActionTarget", a, b);
    // aeons' summon cues
    let i = 0xe + rng.below(5);
    let actor = w.actor(i);
    poke(a, b, cue + 2, &[0]);
    for k in 0..2 {
        let id: u16 = [0x3057, 0x3123, 0x3124, 0x312b, 0x312c, 0x3000][rng.below(6) as usize];
        poke(a, b, cue + 8 + 2 * k, &id.to_le_bytes());
    }
    let x = u32::from(rng.below(5) == 0);
    twin(
        "MsCheckSummon",
        a,
        b,
        0x007b_2980,
        &[i, actor, x, cue],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                ac::ms_check_summon(m, &h, &mut s, i, actor, x, cue).cast_unsigned()
            })
        },
    );
    local_words(a, b);
    same("MsCheckSummon", a, b);
    // all of an actor's input entries
    cues(a, b, rng, w);
    let chr = rng.below(ACTORS);
    let keep = rng.below(2);
    twin(
        "MsCleanInputCue",
        a,
        b,
        0x007b_20b0,
        &[chr, keep],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                ac::ms_clean_input_cue(m, &h, &mut s, chr, keep)
            })
        },
    );
    local_words(a, b);
    same("MsCleanInputCue", a, b);
    // an aeon's command (its table, the kernel's command 0x3100)
    cues(a, b, rng, w);
    let i = 8 + rng.below(10);
    let target = rng.below(ACTORS);
    let kind = rng.below(4);
    set(a, b, ac::AEON_PLAIN, u8::from(rng.below(4) == 0));
    set(a, b, ac::NO_MAGIC_EFFECT, u8::from(rng.below(4) == 0));
    set(a, b, ffx_battle::save::bridge::MENU, rng.next());
    poke(a, b, w.actor(i) + 0x3f9, &[u8::from(rng.below(2) == 0)]);
    twin(
        "MsInsertChrCommand",
        a,
        b,
        0x007a_e1a0,
        &[i, target, kind],
        ALL,
        |m| ac::ms_insert_chr_command(m, i, target, kind),
    );
    same("MsInsertChrCommand", a, b);
}

/// Whether `sym` (`MsGetChrPos` or `MsGetChrBtlPos`) writes actor `i`'s
/// position (both oracles call it on `probe`): where it does not, the
/// originals use their stack garbage.
///
/// A position the original cannot read (its position data pointing out
/// of the snapshot) counts as not written.
fn writes(a: &mut Oracle, b: &mut Oracle, sym: u32, i: u32, probe: u32) -> bool {
    use ffx_battle::pos::bridge as p;
    poke(a, b, probe, &[0xff; 16]);
    // (the original in both: the same pages mapped, the same state)
    let ra = a.call(a.rebase(sym), Conv::Cdecl, &[i, probe]).is_ok();
    let rb = b.call(b.rebase(sym), Conv::Cdecl, &[i, probe]).is_ok();
    poke(a, b, probe, &[0xff; 16]);
    if !(ra && rb) {
        // the fault's write left an unmapped page dirty in both (the two
        // were the same before the probe)
        a.clear_dirty();
        b.clear_dirty();
        return false;
    }
    twin("position probe", a, b, sym, &[i, probe], ALL, |m| {
        if sym == 0x0079_56e0 {
            p::ms_get_chr_pos(m, &stand_in, i, probe).cast_unsigned()
        } else {
            p::ms_get_chr_btl_pos(m, &stand_in, i, probe).cast_unsigned()
        }
    });
    same("position probe", a, b);
    b.read(probe, 12).unwrap().iter().all(|&x| x != 0xff)
}

/// The escape direction and the walk-up check (`ffx_battle::target`).
fn near(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    let h = host();
    // nobody stands on another (status and actions leave 0 there)
    for k in 0..ACTORS {
        poke(a, b, w.actor(k) + 0x43f, &[0xff]);
    }
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    // the escape position's y is never written: the original's stack
    // garbage reaches VF2.y and VF3.y through MsVectorRect2Polar (tiny next
    // to x and z, which must be written)
    let probe = out + 0x40;
    poke(a, b, probe, &[0xff; 16]);
    twin(
        "MsGetChrEscapePos",
        a,
        b,
        0x0079_4ec0,
        &[i, probe],
        ALL,
        |m| {
            let trig = &stand_in;
            ffx_battle::flow::bridge::ms_get_chr_escape_pos(m, i, probe, |m, a, index, kk, out| {
                ffx_battle::pos::bridge::ms_get_summoner_pos(
                    m,
                    trig,
                    a,
                    index.cast_unsigned(),
                    kk.cast_unsigned(),
                    out,
                );
            })
            .cast_unsigned()
        },
    );
    same("MsGetChrEscapePos", a, b);
    let e = b.read(probe, 12).unwrap();
    let escape = e[0..4] != [0xff; 4] && e[8..12] != [0xff; 4];
    if escape && writes(a, b, 0x0079_56e0, i, probe) {
        ftwin("MsGetEscapeDir", a, b, 0x007a_99b0, &[i], |m| {
            dm(m, |m| t::ms_get_escape_dir(m, &h, i))
        });
        let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
        for at in [m + 0x24, m + 0x34] {
            let v = a.read(at, 4).unwrap();
            b.write(at, &v).unwrap();
        }
        local_words(a, b);
    }
    same("MsGetEscapeDir", a, b);
    let walk = if rng.below(4) == 0 { 0 } else { 0x1000 };
    poke(
        a,
        b,
        actor + 0x4ac,
        &(walk | (rng.next() & !0x1000)).to_le_bytes(),
    );
    for o in [0x4b4, 0x44b] {
        poke(a, b, actor + o, &[u8::from(rng.below(3) == 0)]);
    }
    poke(a, b, actor + 0x40d, &[u8::try_from(rng.below(30)).unwrap()]);
    if !writes(a, b, 0x0079_56e0, i, probe) {
        return;
    }
    let mut targets =
        [1 << rng.below(ACTORS), 1 << i, rng.next() & 0x7fff_ffff, 0][rng.below(4) as usize];
    for j in 0..ACTORS {
        if targets >> j & 1 != 0 && !writes(a, b, 0x0079_48e0, j, probe) {
            targets &= !(1 << j);
        }
    }
    // an actor at its battle position, targeting itself: its move
    // target is where it stands (the "near" case)
    if rng.below(3) == 0 && writes(a, b, 0x0079_48e0, i, probe) {
        targets = 1 << i;
        poke(a, b, actor + 0x4b4, &[0]);
        twin("MsSetChrPos", a, b, 0x0079_6150, &[i, probe], ALL, |m| {
            ffx_battle::body::bridge::ms_set_chr_pos(m, i, probe).cast_unsigned()
        });
        same("MsSetChrPos", a, b);
    }
    poke(a, b, actor + 0xdc0, &targets.to_le_bytes());
    twin(
        "MsCheckTargetsNear",
        a,
        b,
        0x007a_f0f0,
        &[i, actor, targets],
        ALL,
        |m| dm(m, |m| t::ms_check_targets_near(m, &h, i, actor, targets)),
    );
    local_words(a, b);
    same("MsCheckTargetsNear", a, b);
    twin(
        "MsCheckChrNear",
        a,
        b,
        0x007a_f230,
        &[i, actor],
        0xff,
        |m| dm(m, |m| t::ms_check_chr_near(m, &h, i, actor)),
    );
    local_words(a, b);
    same("MsCheckChrNear", a, b);
}

/// Where the exchange chain's other-module callees go as logging stubs
/// (`chr_texanim_set_old`, unused in battle), and their records.
const EXCHANGE_STUBS: u32 = 0x0078_03f0;
const EXCHANGE_REC: u32 = 0xd0;

/// The exchange chain's callees of chara, vfx, sound and unported battle
/// code (`ffx_battle::exchange`) with their argument counts.
const EXCHANGE_CALLEES: [(u32, u8); 12] = {
    use ffx_battle::exchange::bridge as x;
    [
        (x::CH_DATA_READ, 1),
        (x::CH_DATA_READ_SYNC, 1),
        (x::CHN_READ_SYSTEM_MGRP, 2),
        (x::CH_MGRP_READ, 2),
        (x::CHN_READ_SYSTEM_MGRP_SYNC, 2),
        (x::MS_BTL_READ_PLAYER_MODEL_PROCESS, 1),
        (x::MS_BTL_READ_PLAYER_PROCESS, 2),
        (x::MS_BTL_READ_MONSTER_PROCESS, 2),
        (x::MS_CD_READ_WEAPON, 3),
        (x::MS_BTL_WEAPON_REGIST, 3),
        (x::OP_ET_WEAPON_FREE, 1),
        (ffx_battle::exec::bridge::SND_SEP_CLEAR, 1),
    ]
};

/// Copies the VU registers from `a` (a path through the originals' stack
/// garbage) and `len` bytes at `at`.
fn copy_garbage(a: &mut Oracle, b: &mut Oracle, at: u32, len: usize) {
    let vu = ffx_battle::vector::bridge::VU_Q.ptr(a).addr & !0xf;
    let end = ffx_battle::vector::bridge::VF_OUT.ptr(a).addr + 0x10;
    let v = a.read(vu, (end - vu) as usize).unwrap();
    b.write(vu, &v).unwrap();
    let v = a.read(at, len).unwrap();
    b.write(at, &v).unwrap();
}

/// A random read queue the manager can finish (no kind 1 to start, no
/// kind above 3, no state above 2: those spin in the original).
fn read_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32) {
    use ffx_battle::misc::bridge as mi;
    let mut q = vec![0u8; 32 * 12];
    for e in q.chunks_mut(12) {
        for x in e.iter_mut() {
            *x = rng.byte();
        }
        let kind = [0, 2, 3, 1][rng.below(4) as usize];
        e[0] = kind;
        e[1] = u8::try_from(rng.below(ACTORS)).unwrap();
        e[2] = u8::try_from(if kind == 1 {
            1 + rng.below(2)
        } else {
            rng.below(3)
        })
        .unwrap();
        e[3] = u8::try_from(rng.below(12)).unwrap();
        e[4] = u8::try_from(rng.below(ACTORS)).unwrap();
        if rng.below(4) == 0 {
            e[6..8].copy_from_slice(&[0, 0]);
        }
    }
    poke(a, b, mi::READ_CUE.ptr(b).addr, &q);
    set(
        a,
        b,
        mi::READ_CUE_INDEX,
        u8::try_from(rng.below(32)).unwrap(),
    );
    let n = [rng.below(4), rng.below(0x21), 0x20, 0][rng.below(4) as usize];
    set(
        a,
        b,
        ffx_battle::state::bridge::READ_CUE_LEVEL,
        u8::try_from(n).unwrap(),
    );
    // the loads finished or not, the weapon read done or not
    for k in [1, 4, 8] {
        let r = u32::from(rng.below(3) == 0);
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
}

/// `MsCalcChrDir`'s `MsVectorRect2Polar` leaves the address of its
/// local in `VF_M.z` (`+8`): copied from `a`.
fn rect_words(a: &mut Oracle, b: &mut Oracle) {
    let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    let w = a.read(m + 8, 4).unwrap();
    b.write(m + 8, &w).unwrap();
}

/// The effect read's callees of system, vfx, chara and the CD library
/// (`ffx_battle::parse`), stubbed in `tex_anim_pos_set_old` (8) and
/// `tex_anim_no_set_old` (4), with their argument counts; the CD
/// library's `+0x10` is `FUN_00780750` (the third).
const EFFECT_CALLEES: [(u32, u8); 12] = {
    use ffx_battle::parse::bridge as p;
    [
        (p::MS_BATTLE_ERROR, 1),
        (p::GET_CD, 0),
        (0x0078_0750, 1),
        (p::MAGIC_FILE_SET_MAGIC_ID, 1),
        (p::MAGIC_FILE_LOAD, 0),
        (p::MAGIC_FILE_IS_PRX_PRELOADED, 1),
        (p::MAGIC_FILE_START, 0),
        (p::MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE, 1),
        (p::MAGIC_FILE_CLEANUP, 0),
        (p::IS_LOAD_TEXTURE, 0),
        (p::GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE, 0),
        (p::CH_GET_ID, 1),
    ]
};

fn effect_stub_at(k: u32) -> u32 {
    if k < 8 {
        0x0078_08f0 + 0x40 * k
    } else {
        0x0078_07c0 + 0x40 * (k - 8)
    }
}

/// The stubs of [`EFFECT_CALLEES`] in both oracles, records from `rec`
/// ([`EXCHANGE_REC`] each), and the CD library object.
fn effect_stubs(a: &mut Oracle, b: &mut Oracle, rec: u32) {
    let cd = scratch(a, b, 0x20);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(EFFECT_CALLEES) {
            logging_stub(o, f, effect_stub_at(k), rec + EXCHANGE_REC * k, n, 0);
        }
        let f = o.rebase(0x0078_0750);
        o.write(cd + 0x10, &f.to_le_bytes()).unwrap();
        o.write(rec + EXCHANGE_REC + 4, &cd.to_le_bytes()).unwrap();
    }
}

/// The magic effect reads (`ffx_battle::parse`).
fn effects(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32, files: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::parse::bridge as p;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_STAT,
        i8::try_from(flag(rng, 8)).unwrap(),
    );
    let stone = [i8::try_from(flag(rng, 6)).unwrap(), 0];
    set(a, b, ffx_battle::state::bridge::STONE_EFFECT, stone);
    set(a, b, p::NO_MAGIC_FILE, flag(rng, 4));
    set(
        a,
        b,
        ffx_battle::action::bridge::NO_MAGIC_EFFECT,
        flag(rng, 6),
    );
    set(a, b, p::EFFECT_PRELOADED, u32::from(flag(rng, 3)));
    set(a, b, p::EFFECT_READ_CHR, rng.below(ACTORS));
    // the effect file table, effect 0x15e at times
    set(a, b, ffx_battle::exec::bridge::EFFECT_FILES, files);
    let mut t = vec![0u8; 0x802];
    for k in 0..0x401 {
        let v = u16::try_from(k * 2 + rng.below(3)).unwrap();
        t[2 * k as usize..2 * k as usize + 2].copy_from_slice(&v.to_le_bytes());
    }
    let id = [rng.below(0x400), 0x84, rng.below(0x40), 0][rng.below(4) as usize];
    if rng.below(3) == 0 && id < 0x3ff {
        t[2 * id as usize..2 * id as usize + 4].copy_from_slice(&[0x5e, 1, 0x5f, 1]);
    }
    poke(a, b, files, &t);
    for (k, r) in [
        (4, [0, 5, u32::MAX][rng.below(3) as usize]),
        (5, u32::from(flag(rng, 2))),
        (6, u32::from(flag(rng, 2))),
        (7, rng.next()),
        (8, rng.next()),
        (9, u32::from(flag(rng, 2))),
        (10, [0, 1, 3][rng.below(3) as usize]),
        (11, [0x108f, rng.below(0x2000)][rng.below(2) as usize]),
    ] {
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    let i = [rng.below(ACTORS), rng.next()][rng.below(4).min(1) as usize];
    let actor = w.actor(i & 0xff);
    if (i & 0xff) < ACTORS {
        poke(a, b, actor + 0xdfb, &[[0, 0, 0, 1][rng.below(4) as usize]]);
    }
    twin("MsEffectRead", a, b, 0x0078_7f20, &[i, id], ALL, |m| {
        dm(m, |m| {
            s!(s);
            p::ms_effect_read(m, &mut s, i, id).cast_unsigned()
        })
    });
    same("MsEffectRead", a, b);
    twin("MsEffectReadComplete", a, b, 0x0078_8110, &[], ALL, |m| {
        dm(m, |m| {
            s!(s);
            p::ms_effect_read_complete(m, &mut s)
        })
    });
    same("MsEffectReadComplete", a, b);
}

/// A command parsed (`MsParseCommand`): a cue entry of a random actor,
/// list and command, every actor placed (the moves and directions).
fn parse(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32, title: u32) {
    use ffx_battle::atel::bridge::Script;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    set(a, b, ffx_battle::state::bridge::INPUT_CUE_COUNT, 0);
    set(a, b, ffx_battle::state::bridge::COMMAND_CUE_COUNT, 0);
    if !placed(a, b, w, out) {
        return;
    }
    let cue = out + 0x80;
    fill(a, b, rng, cue, 0x48);
    let i = rng.below(ACTORS);
    let sub = rng.below(3);
    poke(a, b, cue, &[u8::try_from(i).unwrap()]);
    poke(a, b, cue + 2, &[u8::try_from(sub).unwrap()]);
    let ids = [
        0x3002, 0x3003, 0x3004, 0x3005, 0x3018, 0x3021, 0x3022, 0x3055, 0x3056, 0x3117, 0x30ff,
        0x3129, 0x302a, 0x3025, 0x3026, 0x3029,
    ];
    let id = [ids[rng.below(16) as usize], 0x3000 + rng.below(0x140)][rng.below(3).min(1) as usize];
    let list = cue + 8 + sub * 0x10;
    poke(a, b, list, &u16::try_from(id).unwrap().to_le_bytes());
    // its kind: mostly one with a motion (the fake records' are random)
    let (rec, _) = ffx_kernel::bridge::rom_strings(b, ffx_kernel::bridge::COMMAND, id);
    if rec != 0 && rng.below(3) != 0 {
        poke(
            a,
            b,
            rec + 0x15,
            &[u8::try_from(1 + rng.below(0x1e)).unwrap()],
        );
    }
    // the title window opens or not
    poke(a, b, title + 4, &u32::from(rng.below(2) == 0).to_le_bytes());
    // the second word: an actor to swap in or an aeon
    let other = [rng.below(ACTORS), 8 + rng.below(10)][rng.below(2) as usize];
    poke(a, b, list + 2, &u16::try_from(other).unwrap().to_le_bytes());
    let actor = w.actor(i);
    // the effect motions kept (an actor that can act, +0x15, +0xdce) or
    // cleared
    for o in [0x15, 0xdce] {
        poke(a, b, actor + o, &[u8::from(rng.below(2) == 0)]);
    }
    for o in [0xdfb, 0xdea] {
        poke(
            a,
            b,
            actor + o,
            &[[0, 0, 1, rng.byte()][rng.below(4) as usize]],
        );
    }
    // targets among the placed actors (MsSetTargetDir turns towards them;
    // g_BtlMotionBusy and the action take them): monsters, the actor itself
    let targets = [
        (rng.next() & 0xff) << 0x14,
        1 << (0x14 + rng.below(8)),
        1 << i,
        0,
    ][rng.below(4) as usize];
    poke(a, b, actor + 0xdc0, &targets.to_le_bytes());
    let needed: &[u32] = if [0x3117, 0x30ff].contains(&id) {
        &[other, 0xf, 0x10, 0x11]
    } else {
        &[]
    };
    // (MsSetTargetDir reads the current positions, MsGetChrPos)
    let current = (0..ACTORS)
        .filter(|&j| j == i || targets >> j & 1 != 0)
        .all(|j| writes(a, b, 0x0079_56e0, j, out + 0x40));
    if !current
        || !writes(a, b, 0x0079_48e0, i, out + 0x40)
        || !needed
            .iter()
            .all(|&k| writes(a, b, 0x0079_48e0, k, out + 0x40))
    {
        return;
    }
    twin("MsParseCommand", a, b, 0x007a_e370, &[cue], ALL, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            ffx_battle::parse::bridge::ms_parse_command(m, &h, &mut s, cue).cast_unsigned()
        })
    });
    local_words(a, b);
    rect_words(a, b);
    unplaced_rows(a, b, out);
    same("MsParseCommand", a, b);
}

/// The command cue's first entry moving on (`MsCommandCueProcess`):
/// random waits, states and commands, every actor placed.
fn command_cue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32, srec: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::parse::bridge as p;
    use ffx_battle::state::bridge as st;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    cues(a, b, rng, w);
    set(a, b, st::INPUT_CUE_COUNT, 0);
    if !placed(a, b, w, out) {
        return;
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    // the defense, payment and readiness states able to finish
    for j in 0..ACTORS {
        let q = w.actor(j);
        poke(
            a,
            b,
            q + 0x6d4,
            &[
                [0xff, 0xff, 0xff, u8::try_from(0x14 + rng.below(8)).unwrap()]
                    [rng.below(4) as usize],
            ],
        );
        // a guard's attacker and the one it guards are monsters (placed:
        // MsDefenseStartProcess reads both battle positions)
        poke(
            a,
            b,
            q + 0x6d3,
            &[u8::try_from(0x14 + rng.below(8)).unwrap()],
        );
        for o in [0x508, 0x432, 0x40a, 0x40b] {
            poke(a, b, q + o, &[flag(rng, 2)]);
        }
    }
    set(a, b, p::PARTY_LEAVES, flag(rng, 25));
    set(
        a,
        b,
        ffx_battle::minor::bridge::NUMBERS_KIND12,
        flag(rng, 25),
    );
    set(
        a,
        b,
        st::STONE_EFFECT,
        [i8::try_from(flag(rng, 25)).unwrap(), 0],
    );
    set(a, b, p::COMMAND_HOLD, flag(rng, 25));
    set(a, b, st::MESSAGE_CUE_COUNT, flag(rng, 25));
    set(
        a,
        b,
        p::COMMAND_WAIT,
        [-1, -1, -1, -1, -1, -1, -1, 0][rng.below(8) as usize],
    );
    set(a, b, p::COMMAND_CAMERA, flag(rng, 3));
    set(a, b, p::COMMAND_SINGLE, u32::from(flag(rng, 3)));
    set(a, b, ffx_battle::stat::bridge::GUARD_TOOK, flag(rng, 3));
    // (the system message's own window calls stay out: status 0)
    for (k, r) in [(3, u32::from(flag(rng, 25))), (4, 0)] {
        poke(a, b, srec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    set(
        a,
        b,
        st::COMMAND_CUE_COUNT,
        [1, 2, 0][rng.below(3) as usize],
    );
    let cue = st::COMMAND_CUE.ptr(b).addr;
    let i = rng.below(ACTORS);
    let k = rng.below(2);
    let state = [0, 0, 0, 1, 2, 3, 4, 5, 6, 7][rng.below(10) as usize];
    poke(
        a,
        b,
        cue,
        &[
            u8::try_from(i).unwrap(),
            u8::try_from(rng.below(2)).unwrap(),
            u8::try_from(k).unwrap(),
            [0, 1, 2, 1][rng.below(4) as usize],
            state,
            rng.byte(),
            [0, 1, 4][rng.below(3) as usize],
            [0x30, rng.byte()][rng.below(2) as usize],
        ],
    );
    let ids = [0x3002, 0x3003, 0x3018, 0x3021, 0x3025, 0x302a, 0x3056];
    for j in 0..2 {
        let id = [ids[rng.below(7) as usize], 0x3000 + rng.below(0x110)][rng.below(2) as usize];
        let id = if id == 0x311e { 0x3000 } else { id };
        poke(
            a,
            b,
            cue + 8 + 0x10 * j,
            &u16::try_from(id).unwrap().to_le_bytes(),
        );
        poke(
            a,
            b,
            cue + 0xa + 0x10 * j,
            &[u8::try_from(rng.below(ACTORS)).unwrap(), 0],
        );
        let t = [1 << rng.below(ACTORS), 0, rng.next() & 0x7fff_ffff][rng.below(3) as usize];
        poke(a, b, cue + 0x10 * (j + 1), &t.to_le_bytes());
    }
    let actor = w.actor(i);
    for o in [0x433, 0x50e, 0x608, 0x6e0, 0x717, 0x508, 0x436, 0xdf4] {
        poke(a, b, actor + o, &[flag(rng, 12)]);
    }
    // the payment's motion actor (Yojimbo's, +0x6d1): any byte (MsSetMotion
    // on a bad index matches the original's)
    poke(a, b, actor + 0x6d1, &[rng.byte()]);
    // MP enough at times (MsCalcMagic)
    if rng.below(2) == 0 {
        poke(a, b, actor + 0x5d4, &0x7fffu32.to_le_bytes());
        poke(a, b, actor + 0x6de, &[0]);
    }
    poke(a, b, actor + 0xdfb, &[0]);
    twin("MsCommandCueProcess", a, b, 0x0079_2210, &[], ALL, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            p::ms_command_cue_process(m, &h, &mut s)
        })
    });
    local_words(a, b);
    rect_words(a, b);
    unplaced_rows(a, b, out);
    same("MsCommandCueProcess", a, b);
}

/// The battle loop's per-frame callees of sound, the CD library and the
/// voice status (`ffx_battle::process`), the sound effects (`SndSepPlay`),
/// trophy (main), neck and character ID (chara), the battle effects'
/// release (vfx, the heap), stubbed in `MsDebugEnvSave` (debug
/// only); the last is the CD library's `+8`.
const PROCESS_CALLEES: [(u32, u8); 13] = {
    use ffx_battle::process::bridge as p;
    [
        (p::SND_MUSIC_STOP, 1),
        (p::SND_MUSIC_CLEAR, 1),
        (p::TK_SND_REPORT_STATUS, 0),
        (p::SND_IS_READ_STATUS, 0),
        (p::SND_IS_READ_SHOUT_VOICE_STATUS, 0),
        (ffx_battle::after_damage::bridge::SND_SEP_PLAY, 3),
        (ffx_battle::death::bridge::TROPHY_CHECK_DEFEAT_MONSTER, 1),
        (ffx_battle::atel::bridge::CH_NECK_SET_ENABLE, 2),
        (ffx_battle::death::bridge::CH_GET_CHAR_ID, 1),
        (p::OP_ET_BATTLE_EFFECT_FREE, 0),
        (p::USER_FREE, 1),
        (p::MAGIC_FILE_GET_MAGIC_ID, 0),
        (0, 1),
    ]
};
const PROCESS_STUBS: u32 = 0x007c_d800;

/// The stubs of [`PROCESS_CALLEES`] in both oracles, records from `rec`;
/// the CD library object (the effect stubs' `GetCD` result) gets the last
/// as its `+8`.
fn process_stubs(a: &mut Oracle, b: &mut Oracle, rec: u32, erec: u32) {
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(PROCESS_CALLEES) {
            logging_stub(o, f, PROCESS_STUBS + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        let cd = u32::from_le_bytes(
            o.read(erec + EXCHANGE_REC + 4, 4)
                .unwrap()
                .try_into()
                .unwrap(),
        );
        let f = o.rebase(PROCESS_STUBS + 0x40 * 12);
        o.write(cd + 8, &f.to_le_bytes()).unwrap();
    }
}

/// The battle loop's per-frame pieces (`ffx_battle::process`) and the
/// camera reset.
fn process(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    rec: u32,
    srec: u32,
    out: u32,
) {
    use ffx_battle::process::bridge as p;
    let mut call = game_call;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    for (k, r) in [
        (1, rng.next()),
        (2, [0, 0, 0, 1, 2][rng.below(5) as usize]),
        (3, u32::from(flag(rng, 6))),
        (4, u32::from(flag(rng, 6))),
        (11, [0x1ad, 0, 5][rng.below(3) as usize]),
        (12, u32::from(flag(rng, 6))),
    ] {
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    poke(
        a,
        b,
        srec + 3 * EXCHANGE_REC + 4,
        &u32::from(flag(rng, 6)).to_le_bytes(),
    );
    twin("MsNowMusicStop", a, b, 0x007a_00b0, &[], ALL, |m| {
        dm(m, |m| p::ms_now_music_stop(m, &mut call))
    });
    same("MsNowMusicStop", a, b);
    for i in 0..ACTORS {
        poke(a, b, w.actor(i) + 0x3d, &[flag(rng, 3)]);
    }
    twin("MsResetNormalCalc", a, b, 0x0079_5c40, &[], ALL, |m| {
        p::ms_reset_normal_calc(m)
    });
    same("MsResetNormalCalc", a, b);
    set(a, b, p::PAUSE, flag(rng, 6));
    set(a, b, p::HIT_MOTION, flag(rng, 6));
    set(
        a,
        b,
        ffx_battle::action::bridge::TURN_END_ACTOR,
        [0xff, 0x80, 3, 0][rng.below(4) as usize],
    );
    set(
        a,
        b,
        ffx_battle::state::bridge::MESSAGE_CUE_COUNT,
        flag(rng, 6),
    );
    set(a, b, ffx_battle::atel::bridge::CAMERA_STATE, flag(rng, 6));
    set(a, b, ffx_battle::flow::bridge::LIMIT_LEARNT, flag(rng, 6));
    set(
        a,
        b,
        ffx_battle::state::bridge::STONE_EFFECT,
        [
            i8::try_from(flag(rng, 6)).unwrap(),
            i8::try_from(flag(rng, 6)).unwrap(),
        ],
    );
    set(a, b, ffx_battle::parse::bridge::COMMAND_HOLD, flag(rng, 6));
    twin("MsBattleCanProgress", a, b, 0x0079_1190, &[], ALL, |m| {
        p::ms_battle_can_progress(m).cast_unsigned()
    });
    let any = rng.below(2);
    twin("msVoiceCheckStanby", a, b, 0x007b_2c10, &[any], ALL, |m| {
        dm(m, |m| {
            p::ms_voice_check_stanby(m, &mut call, any).cast_unsigned()
        })
    });
    same("msVoiceCheckStanby", a, b);
    set(
        a,
        b,
        ffx_battle::state::bridge::STATUS,
        [1, 1, 0, 0x21][rng.below(4) as usize],
    );
    set(a, b, p::RESET_WAIT, u32::from(flag(rng, 6)));
    set(a, b, p::RESET_WAIT2, flag(rng, 6));
    // mostly idle: no read queued, no sound transfer, no actor in a motion
    // phase (+0xdfb)
    if rng.below(4) != 0 {
        set(a, b, ffx_battle::state::bridge::READ_CUE_LEVEL, 0);
        set(a, b, ffx_battle::state::bridge::SEP_TRANS, 0);
        for j in 0..ACTORS {
            poke(a, b, w.actor(j) + 0xdfb, &[0]);
        }
    }
    twin("MsCheckSoftResetOk", a, b, 0x0079_2e80, &[], ALL, |m| {
        dm(m, |m| {
            p::ms_check_soft_reset_ok(m, &mut call).cast_unsigned()
        })
    });
    same("MsCheckSoftResetOk", a, b);
    let cam = ffx_battle::camera::bridge::DEBUG_CAMERA_SLEEP;
    let v = [cam.get(b), rng.below(2), rng.next()][rng.below(3) as usize];
    twin("MsDebugCameraSleep", a, b, 0x0079_21c0, &[v], 0, |m| {
        ffx_battle::camera::bridge::ms_debug_camera_sleep(m, v);
        0
    });
    same("MsDebugCameraSleep", a, b);
    // actors moving back to their places (the positions MsCheckBtlPos reads)
    for j in 0..ACTORS {
        let q = w.actor(j);
        poke(a, b, q + 0x34, &[flag(rng, 2)]);
    }
    let i = rng.below(ACTORS);
    let placed_now = |a: &mut Oracle, b: &mut Oracle, j: u32| {
        writes(a, b, 0x0079_56e0, j, out + 0x40) && writes(a, b, 0x0079_48e0, j, out + 0x40)
    };
    if placed_now(a, b, i) {
        let actor = w.actor(i);
        let h = host();
        twin("MsMotionPosCheck", a, b, 0x007a_a100, &[i, actor], 0, |m| {
            dm(m, |m| p::ms_motion_pos_check(m, &h, i, actor));
            0
        });
        same("MsMotionPosCheck", a, b);
    }
    if (0..ACTORS).all(|j| {
        ffx_battle::flow::bridge::ms_move_check_flag(b, w.actor(j)) == 0 || placed_now(a, b, j)
    }) {
        let h = host();
        twin("MsMotionPosCheckAll", a, b, 0x007a_a140, &[], 0, |m| {
            dm(m, |m| p::ms_motion_pos_check_all(m, &h));
            0
        });
        same("MsMotionPosCheckAll", a, b);
    }
    // overdrive modes to announce
    // (half the time nobody acting: the announcements run)
    let quiet = rng.below(2) == 0;
    for j in 0..ACTORS {
        let v = if quiet { 0 } else { flag(rng, 12) };
        poke(a, b, w.actor(j) + 0xdf0, &[v]);
    }
    twin("MsCountChrActing", a, b, 0x007b_0ca0, &[], ALL, |m| {
        p::ms_count_chr_acting(m)
    });
    set(a, b, p::LIMIT_TYPE_CUE, [1, 1, 0][rng.below(3) as usize]);
    set(
        a,
        b,
        ffx_battle::flow::bridge::LIMIT_LEARNT,
        flag(rng, 4) ^ 1,
    );
    let id = [0, 0, u16::try_from(0x3000 + rng.below(0x100)).unwrap()][rng.below(3) as usize];
    set(a, b, ffx_battle::flow::bridge::LIMIT_LEARNT_ID, id);
    set(a, b, ffx_battle::atel::bridge::CAMERA_STATE, flag(rng, 4));
    set(a, b, ffx_battle::state::bridge::MESSAGE_CUE_COUNT, 0);
    let save = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    for c in 0..0x12u32 {
        let r = save + c * 0x94;
        for n in 0..0x14 {
            poke(
                a,
                b,
                r + 0x60 + 2 * n,
                &u16::from(flag(rng, 4) ^ 1).to_le_bytes(),
            );
        }
        poke(a, b, r + 0x88, &(rng.next() & rng.next()).to_le_bytes());
    }
    twin("MsLimitTypeProcess", a, b, 0x007b_1180, &[], 0, |m| {
        dm(m, |m| p::ms_limit_type_process(m, &mut call));
        0
    });
    same("MsLimitTypeProcess", a, b);
    let buf = [0, rng.next()][rng.below(2) as usize];
    set(a, b, ffx_battle::misc::bridge::ET_EFFECT_READ, buf);
    twin("MsEtEffectFree", a, b, 0x0079_e630, &[], 0, |m| {
        dm(m, |m| p::ms_et_effect_free(m, &mut call));
        0
    });
    same("MsEtEffectFree", a, b);
    // the turn order
    set(
        a,
        b,
        ffx_battle::state::bridge::INPUT_CUE_COUNT,
        i8::from(rng.below(4) == 0),
    );
    set(a, b, p::ORDER_HOLD, flag(rng, 8));
    set(a, b, p::IDLE_ROUNDS, i8::try_from(rng.below(4)).unwrap());
    set(
        a,
        b,
        p::IDLE_ROUNDS_LIMIT,
        i8::try_from(rng.below(4)).unwrap(),
    );
    set(a, b, p::STATUS_SHORT, flag(rng, 3));
    let cmd = [0x312c, 0x3022, 0x3000, 0x3001][rng.below(4) as usize];
    set(a, b, ffx_battle::parse::bridge::PARSED_COMMAND, cmd);
    set(
        a,
        b,
        ffx_battle::state::bridge::TITLE_WINDOW,
        [i8::try_from(rng.below(3)).unwrap() - 1, 0],
    );
    // a round where nobody's wait is over (the idle rounds)
    let idle = rng.below(3) == 0;
    for j in 0..ACTORS {
        let q = w.actor(j);
        for o in [0xde6, 0xdcc] {
            poke(a, b, q + o, &[flag(rng, 6)]);
        }
        poke(a, b, q + 0xdc8, &[flag(rng, 4) ^ 1]);
        poke(a, b, q + 0xdd6, &[flag(rng, 4) ^ 1]);
        poke(
            a,
            b,
            q + 0x65c,
            &[if idle {
                [1, 2, 3, rng.byte() | 1][rng.below(4) as usize]
            } else {
                [0, 0, 1, 2, rng.byte()][rng.below(5) as usize]
            }],
        );
        poke(
            a,
            b,
            q + 0x6d2,
            &[[0xff, 0xfe, rng.byte()][rng.below(3) as usize]],
        );
        let s606 = b.read(q + 0x606, 1).unwrap()[0]
            & !(4 * flag(rng, 1).min(1) * u8::from(rng.below(5) != 0));
        poke(a, b, q + 0x606, &[s606]);
        poke(a, b, q + 0x5ac, &[rng.byte()]);
        for o in [0x608, 0x609, 0x60a, 0x614] {
            poke(a, b, q + o, &[flag(rng, 5)]);
        }
        poke(
            a,
            b,
            q + 0x5bb,
            &[[0xd, 0xe, 0xf, 0x10, rng.byte()][rng.below(5) as usize]],
        );
        // (no AP-linked gauge: its next-AP table is not loaded here;
        // MsLimitUp has its own test) and an ordinary gauge rate
        let abil = b.read(q + 0x6be, 1).unwrap()[0] & !8;
        poke(a, b, q + 0x6be, &[abil]);
        let rate = (f32::from(i16::try_from(rng.range(1, 3000)).unwrap()) / 100.0).to_bits();
        poke(a, b, q + 0x70c, &rate.to_le_bytes());
    }
    let n = [0, 1, 3, 8, rng.below(0x20)][rng.below(5) as usize];
    set(
        a,
        b,
        ffx_battle::state::bridge::ORDER_COUNT,
        i32::try_from(n).unwrap(),
    );
    let list: Vec<u8> = (0..0x20)
        .map(|_| u8::try_from(rng.below(ACTORS)).unwrap())
        .collect();
    poke(a, b, ffx_battle::state::bridge::ORDERS.ptr(b).addr, &list);
    twin("MsOrderCalc", a, b, 0x0078_d3e0, &[out + 0x40], ALL, |m| {
        p::ms_order_calc(m, out + 0x40)
    });
    same("MsOrderCalc", a, b);
    let i = rng.below(9);
    let actor = w.actor(i);
    twin(
        "MsLimitTypeTurnCheck",
        a,
        b,
        0x007b_13c0,
        &[i, actor],
        ALL,
        |m| p::ms_limit_type_turn_check(m, i, actor),
    );
    same("MsLimitTypeTurnCheck", a, b);
    twin("MsBattleProcess_Order", a, b, 0x0079_0fb0, &[], ALL, |m| {
        dm(m, |m| p::ms_battle_process_order(m, &mut call)).cast_unsigned()
    });
    same("MsBattleProcess_Order", a, b);
    if rng.below(8) == 0 {
        twin("MsCameraInit", a, b, 0x007b_d5d0, &[], 0, |m| {
            ffx_battle::camera::bridge::ms_camera_init(m);
            0
        });
        same("MsCameraInit", a, b);
    }
}

/// Actors going down (`ffx_battle::death`): their death checks, effects
/// and motions, the stone effect queue, and every actor's each frame.
fn deaths(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::death::bridge as d;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    set(a, b, ffx_battle::state::bridge::INPUT_CUE_COUNT, 0);
    set(a, b, ffx_battle::state::bridge::COMMAND_CUE_COUNT, 0);
    for i in 0..ACTORS {
        let p = w.actor(i);
        poke(a, b, p + 0xdcc, &[[0, 1, 1, 2][rng.below(4) as usize]]);
        poke(
            a,
            b,
            p + 0xf5e,
            &[[0, 0, 2, 4, rng.byte()][rng.below(5) as usize]],
        );
        for o in [0x3f9, 0x508, 0xdcd, 0xdce, 0xdd0, 0x3f7] {
            poke(a, b, p + o, &[flag(rng, 3)]);
        }
        poke(
            a,
            b,
            p + 0x449,
            &[[0, 1, 6, 2, rng.byte()][rng.below(5) as usize]],
        );
        poke(a, b, p + 0x3f5, &[u8::try_from(rng.below(10)).unwrap()]);
        poke(
            a,
            b,
            p + 0x701,
            &[[0, 3, rng.byte()][rng.below(3) as usize]],
        );
    }
    let i = [rng.below(ACTORS), 0xff, rng.byte().into()][rng.below(4).min(2) as usize];
    let actor = w.actor(i & 0xff);
    if i < ACTORS {
        twin(
            "MsCheckDeathMotion",
            a,
            b,
            0x0078_c4c0,
            &[i, actor],
            ALL,
            |m| d::ms_check_death_motion(m, i, actor),
        );
    }
    let k = rng.below(10);
    twin("MsDeathEffectStart", a, b, 0x0079_e3d0, &[i, k], ALL, |m| {
        dm(m, |m| {
            s!(s);
            d::ms_death_effect_start(m, &mut s, i, k)
        })
    });
    same("MsDeathEffectStart", a, b);
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin(
        "MsDeathTerminate",
        a,
        b,
        0x0078_cca0,
        &[i, actor],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                d::ms_death_terminate(m, &mut s, i, actor)
            })
        },
    );
    same("MsDeathTerminate", a, b);
    if writes(a, b, 0x0079_56e0, i, out + 0x40) {
        twin(
            "MsSetDeathProcess",
            a,
            b,
            0x0078_ddc0,
            &[i, actor],
            0,
            |m| {
                dm(m, |m| {
                    s!(s);
                    d::ms_set_death_process(m, &h, &mut s, i, actor);
                });
                0
            },
        );
        same("MsSetDeathProcess", a, b);
    }
    // the stone queue
    let n = rng.below(5);
    set(
        a,
        b,
        ffx_battle::state::bridge::STONE_EFFECT,
        [
            i8::try_from(n).unwrap(),
            i8::try_from(rng.below(4)).unwrap(),
        ],
    );
    let q: Vec<u8> = (0..8)
        .flat_map(|_| [rng.below(ACTORS), 0xff, 0x40][rng.below(4).min(2) as usize].to_le_bytes())
        .collect();
    poke(a, b, ffx_battle::minor::bridge::STONES.ptr(b).addr, &q);
    poke(
        a,
        b,
        ffx_battle::misc::bridge::MESSAGE_CUE_INIT.ptr(b).addr,
        &[flag(rng, 3) ^ 1],
    );
    twin("MsStoneEffectProcess", a, b, 0x007a_2690, &[], ALL, |m| {
        dm(m, |m| {
            s!(s);
            d::ms_stone_effect_process(m, &mut s)
        })
    });
    same("MsStoneEffectProcess", a, b);
    // whole deaths (the positions MsResetChrPos and the camera read)
    let dying: Vec<u32> = (0..ACTORS)
        .filter(|&j| b.read(w.actor(j) + 0xdcc, 1).unwrap()[0] == 1)
        .collect();
    if !dying.iter().all(|&j| {
        writes(a, b, 0x0079_56e0, j, out + 0x40) && writes(a, b, 0x0079_48e0, j, out + 0x40)
    }) || !placed(a, b, w, out)
    {
        return;
    }
    let j = dying.first().copied().unwrap_or(0);
    let flags = rng.below(3);
    twin("MsSetDeath", a, b, 0x0078_dc30, &[j, flags], ALL, |m| {
        dm(m, |m| {
            s!(s);
            d::ms_set_death(m, &h, &mut s, j, flags)
        })
    });
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsSetDeath", a, b);
    twin("MsSetDeathAll", a, b, 0x0079_1760, &[], 0, |m| {
        dm(m, |m| {
            s!(s);
            ffx_battle::process::bridge::ms_set_death_all(m, &h, &mut s);
        });
        0
    });
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsSetDeathAll", a, b);
}

/// The game's functions an actor's turn start, a death and the damage
/// step call, stubbed: `TOSetBtlActiveChr` (menu), `MsGetItem` (not
/// ported), `forceAutoBattleOff` and `TOBtlCloseSignWindow` (system),
/// `Sg_EnFocus` (graphics), `op_et_prayeff_send_read_through` (vfx); the
/// first four arguments logged.
const INPUT_CALLEES: [(u32, u8); 6] = [
    (ffx_battle::input::bridge::TO_SET_BTL_ACTIVE_CHR, 1),
    (ffx_battle::death::bridge::MS_GET_ITEM, 4),
    (ffx_battle::death::bridge::FORCE_AUTO_BATTLE_OFF, 0),
    (ffx_battle::death::bridge::TO_BTL_CLOSE_SIGN_WINDOW, 0),
    (ffx_battle::chr_damage::bridge::SG_EN_FOCUS, 1),
    (
        ffx_battle::chr_damage::bridge::OP_ET_PRAYEFF_SEND_READ_THROUGH,
        1,
    ),
];
const INPUT_STUBS: u32 = PROCESS_STUBS + 0x40 * 16;

/// The disposal callees `ffx_battle::damage_end` goes through (chara's
/// model and data, `Sg_FreeA`, the effect, its stream and file), logging
/// stubs after the input ones.
const FREE_CALLEES: [(u32, u8); 8] = {
    use ffx_battle::damage_end::bridge as de;
    [
        (de::CH_MODEL_PURGE_PACKET, 2),
        (de::CH_DISPOSE, 1),
        (de::CHN_DISPOSE_SYSTEM_MGRP, 2),
        (de::CHN_DATA_DISPOSE, 2),
        (de::SG_FREE_A, 2),
        (de::OP_OVL_EFFECT_FREE, 0),
        (de::SND_STOP_BATTLE_STREAM, 0),
        (de::MAGIC_FILE_STOP, 0),
    ]
};
const FREE_STUBS: u32 = PROCESS_STUBS + 0x40 * 24;

/// The battle number `MsBattleGetNum` gives: a field record of `id` and a
/// group byte `g` at `rec`, in battle.
fn battle_num(a: &mut Oracle, b: &mut Oracle, rec: u32, id: i16, g: u8) {
    use ffx_battle::state::bridge as st;
    let mut r = vec![0u8; 16];
    r[..2].copy_from_slice(&id.to_le_bytes());
    r[14] = g;
    poke(a, b, rec, &r);
    set(a, b, st::STATUS, 1);
    set(a, b, st::FIELD_RECORDS, rec);
    set(a, b, st::FIELD_INDEX, 0);
    set(a, b, st::SCENE_GROUP, rec + 14);
}

/// A random halfword.
fn half16(rng: &mut Xorshift) -> u16 {
    u16::try_from(rng.below(0x1_0000)).unwrap()
}

thread_local! {
    /// [`turns`]' own random stream (its state).
    static TURNS_RNG: std::cell::Cell<u32> = const { std::cell::Cell::new(0x7a46_1212) };
}

/// [`turns`] with the actors and the globals it sets put back after (its
/// statuses and HP send the other checks down the originals' stack
/// garbage).
fn input(a: &mut Oracle, b: &mut Oracle, w: &World, out: u32, rec: u32) {
    use ffx_battle::state::bridge as st;
    let keep: Vec<(u32, usize)> = vec![
        (w.actor(0), (ACTORS * battle_common::world::ACTOR) as usize),
        (st::UNDERWATER.ptr(b).addr, 1),
        (ffx_battle::atel::bridge::CAMERA_STATE.ptr(b).addr, 1),
        (ffx_battle::parse::bridge::COMMAND_FLAG29.ptr(b).addr, 1),
        (ffx_battle::parse::bridge::COMMAND_3029_COUNT.ptr(b).addr, 1),
        (st::STATUS.ptr(b).addr, 1),
        (st::FIELD_RECORDS.ptr(b).addr, 4),
        (st::FIELD_INDEX.ptr(b).addr, 2),
        (st::SCENE_GROUP.ptr(b).addr, 4),
        (ffx_battle::actor::bridge::MON_TOP.ptr(b).addr, 4),
        (ffx_battle::menu::bridge::PARTY.ptr(b).addr, 7),
        (ffx_battle::minor::bridge::DIR_FREE.ptr(b).addr, 1),
        (ffx_battle::menu::bridge::END_FORCED.ptr(b).addr, 1),
        (ffx_battle::actor::bridge::CHR_SPARE.ptr(b).addr + 0x436, 1),
    ];
    let saved: Vec<Vec<u8>> = keep.iter().map(|&(at, n)| b.read(at, n).unwrap()).collect();
    // its own random stream: the other checks see the same cases with or
    // without it
    let mut own = TURNS_RNG.with(|c| {
        let mut r = Xorshift(c.get());
        let next = Xorshift(r.next());
        c.set(r.0);
        next
    });
    turns(a, b, &mut own, w, out, rec);
    for (&(at, _), v) in keep.iter().zip(&saved) {
        poke(a, b, at, v);
    }
}

/// An actor's turn start (`ffx_battle::input`): poses, damage numbers,
/// HP loss, Regen, Doom, Provoke, Berserk, the menu.
fn turns(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32, rec: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::death::bridge as d;
    use ffx_battle::input::bridge as ip;
    use ffx_battle::minor::bridge as mn;
    use ffx_battle::state::bridge as st;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    // the battle (MsBattleGetNum: the scripted fights MsSubHP knows)
    let (id, g) = [
        (0x159, 0),
        (0x20, 1),
        (0x209, 0),
        (0x20, 0),
        (
            i16::try_from(rng.range(-0x8000, 0x8000)).unwrap(),
            rng.byte(),
        ),
    ][rng.below(5) as usize];
    battle_num(a, b, rec + 0x5e0, id, g);
    // the monsters (MsGetMon) are the actors from 0x14
    set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    set(a, b, st::INPUT_CUE_COUNT, 0);
    set(a, b, st::COMMAND_CUE_COUNT, 0);
    set(a, b, st::UNDERWATER, i8::try_from(flag(rng, 4)).unwrap());
    for i in 0..ACTORS {
        let p = w.actor(i);
        let max = [1, 100, 1000, rng.range(1, 99_999)][rng.below(4) as usize];
        let hp =
            [0, max, max / 2, max / 4, max / 10, rng.range(-5, max + 5)][rng.below(6) as usize];
        poke(a, b, p + 0x594, &max.to_le_bytes());
        poke(a, b, p + 0x5d0, &hp.to_le_bytes());
        let mp_max = rng.range(0, 999);
        poke(a, b, p + 0x598, &mp_max.to_le_bytes());
        poke(a, b, p + 0x5d4, &rng.range(0, mp_max + 1).to_le_bytes());
        for o in [
            0x450, 0x609, 0x60a, 0x612, 0xddb, 0x716, 0xdf7, 0xdf3, 0x5cb,
        ] {
            poke(a, b, p + o, &[flag(rng, 4)]);
        }
        // the 13 status counts (1 to 0xfd count down)
        let counts: Vec<u8> = (0..13)
            .map(|_| [0, 0, 0, 1, 2, 0xfd, 0xfe, 0xff, rng.byte()][rng.below(9) as usize])
            .collect();
        poke(a, b, p + 0x608, &counts);
        // mostly one the cursor may pick (MsGetCursorTarget: +0xdd9, no
        // event +0xddd, near +0x4ff, there +0xdcd)
        if rng.below(3) != 0 {
            for (o, v) in [
                (0xdd9, 1),
                (0xddd, flag(rng, 4) ^ 1),
                (0x4ff, 0),
                (0xdcd, 0),
            ] {
                poke(a, b, p + o, &[v]);
            }
            poke(a, b, p + 0xdde, &0xffu16.to_le_bytes());
        }
        // the overdrive mode in use (Comrade 7, Slayer 8, Hero 9) and the
        // Hero's strength +0x6f4
        poke(
            a,
            b,
            p + 0x5bb,
            &[[7, 8, 9, rng.byte()][rng.below(4) as usize]],
        );
        let own = [rng.range(0, 3000), rng.next().cast_signed()][rng.below(2) as usize];
        poke(a, b, p + 0x6f4, &own.to_le_bytes());
        // few groups (+0x504 & 7: Provoke keeps the provoker's group)
        let g = b.read(p + 0x504, 1).unwrap()[0] & !7 | u8::try_from(rng.below(2)).unwrap();
        poke(a, b, p + 0x504, &[g]);
        // each moves itself (MsDamageSetMotion's +0x422: the positions
        // checked are its own)
        poke(a, b, p + 0x422, &[u8::try_from(i).unwrap()]);
        // an automatic actor's +0xdf3 (sign-extended)
        poke(
            a,
            b,
            p + 0xdf3,
            &[[0, 1, 0x80, rng.byte()][rng.below(4) as usize]],
        );
        poke(a, b, p + 0xdcc, &[[0, 0, 0, 1, 2][rng.below(5) as usize]]);
        poke(a, b, p + 0xdc8, &[flag(rng, 5) ^ 1]);
        poke(a, b, p + 0x6d2, &[rng.byte()]);
        poke(a, b, p + 0x5c8, &[u8::try_from(rng.below(3)).unwrap()]);
        poke(a, b, p + 0x5c4, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        let id =
            [0, 0x3001, 0x3000, 0x3022, 0x3057, 0x3000 + rng.below(0x80)][rng.below(6) as usize];
        let id = u16::try_from(id).unwrap();
        poke(a, b, p + 0x6c6, &id.to_le_bytes());
        poke(
            a,
            b,
            p + 0x591,
            &[[0x11, 0, rng.byte()][rng.below(3) as usize]],
        );
        let st6: u16 =
            [0, 2, 0x100, 0x200, 0x300, 0x400, 0x500, half16(rng)][rng.below(8) as usize];
        poke(a, b, p + 0x606, &st6.to_le_bytes());
        let s16: u16 = [0, 0x4000, 0x800, 0x2040, 0x10c0, half16(rng)][rng.below(6) as usize];
        poke(a, b, p + 0x616, &s16.to_le_bytes());
        let s2e: u16 = [0, 0xffff, half16(rng)][rng.below(3) as usize];
        poke(a, b, p + 0x62e, &s2e.to_le_bytes());
        // no AP-linked gauge (its table is not loaded here)
        let abil = b.read(p + 0x6be, 1).unwrap()[0] & !8;
        poke(a, b, p + 0x6be, &[abil]);
    }
    // a pose
    let i = rng.below(ACTORS);
    let level = [u32::MAX, u32::MAX, u32::MAX, rng.below(4)][rng.below(4) as usize];
    twin("MsSetChrWeak", a, b, 0x0078_d7f0, &[i, level], ALL, |m| {
        ip::ms_set_chr_weak(m, i, level)
    });
    same("MsSetChrWeak", a, b);
    // the damage numbers: a pool of 0x80 records, some taken
    let pool = rec + 0x800;
    let mut recs = vec![0u8; 0x80 * 0x14];
    let full = rng.below(6) == 0;
    for r in recs.chunks_mut(0x14) {
        if full || rng.below(3) == 0 {
            r[0] = 1 + rng.byte() % 0x80;
        }
    }
    poke(a, b, pool, &recs);
    set(a, b, ffx_battle::misc::bridge::NUMBERS, pool);
    set(a, b, mn::NUMBER_TAIL, out + 0x0c);
    let n = [
        rng.range(-20, 20),
        rng.range(0, 99_999),
        rng.next().cast_signed(),
        i32::MIN,
    ][rng.below(4) as usize];
    let digits = [0, out + 0x10][rng.below(2) as usize];
    twin(
        "MsNumberDigits",
        a,
        b,
        0x0079_fc00,
        &[n.cast_unsigned(), digits],
        ALL,
        |m| ip::ms_number_digits(m, n, digits).cast_unsigned(),
    );
    same("MsNumberDigits", a, b);
    let args: Vec<u32> = (0..10).map(|_| rng.next()).collect();
    twin("MsNumberPut", a, b, 0x0079_fc60, &args, ALL, |m| {
        let x = |k: usize| args[k].to_le_bytes()[0];
        let y = |k: usize| u16::from_le_bytes([args[k].to_le_bytes()[0], args[k].to_le_bytes()[1]]);
        ip::ms_number_put(
            m,
            x(0),
            x(1),
            x(2),
            y(3),
            y(4),
            x(5),
            x(6),
            x(7),
            x(8),
            x(9),
        )
    });
    same("MsNumberPut", a, b);
    for g in [mn::NUMBERS_KIND1, mn::NUMBERS_KIND12, mn::NUMBERS_SHOWN] {
        set(a, b, g, 0);
    }
    let kind = rng.below(7);
    let value = [rng.range(-9999, 9999), 0, rng.next().cast_signed()][rng.below(3) as usize];
    let other = rng.range(-3, 3);
    let flags = [0, 1, 2, 3][rng.below(4) as usize];
    let style = [0, 1, 2, 3, 0x81, 0x82][rng.below(6) as usize];
    let args = [
        i,
        kind,
        value.cast_unsigned(),
        other.cast_unsigned(),
        0,
        flags,
        style,
    ];
    twin("MsNumberRegist", a, b, 0x0079_fa00, &args, ALL, |m| {
        ip::ms_number_regist(m, i, kind, value, other, 0, flags, style).cast_unsigned()
    });
    same("MsNumberRegist", a, b);
    // HP loss in the scripted battles
    set(a, b, ip::KEEP_ALIVE, u32::from(flag(rng, 2)));
    set(a, b, ip::DEBUG_KILL, u32::from(flag(rng, 6)));
    set(
        a,
        b,
        ffx_battle::parse::bridge::COMMAND_3029_COUNT,
        flag(rng, 2),
    );
    set(
        a,
        b,
        ffx_battle::damage::bridge::DEBUG_NO_DAMAGE_MONSTER,
        flag(rng, 6),
    );
    set(
        a,
        b,
        ffx_battle::damage::bridge::DEBUG_NO_DAMAGE_PARTY,
        flag(rng, 6),
    );
    let i = [0x14, 0x15, 0x17, rng.below(ACTORS)][rng.below(4) as usize];
    let actor = w.actor(i);
    if rng.below(3) == 0 {
        let max: i32 = [24_000, 48_000][rng.below(2) as usize];
        poke(a, b, actor + 0x594, &max.to_le_bytes());
    }
    poke(a, b, actor + 4, &rng.range(-1, 10).to_le_bytes());
    let hp = i32::from_le_bytes(b.read(actor + 0x5d0, 4).unwrap().try_into().unwrap());
    let dmg = [rng.range(-500, 30_000), rng.range(0, 100), hp][rng.below(3) as usize];
    let args = [
        i,
        actor,
        dmg.cast_unsigned(),
        rng.next(),
        0,
        rng.below(4),
        1,
    ];
    twin("MsSubHP", a, b, 0x0078_e230, &args, 0, |m| {
        ip::ms_sub_hp(m, i, actor, dmg, args[3].cast_signed(), 0, args[5], 1);
        0
    });
    same("MsSubHP", a, b);
    // an automatic actor
    let i = [0xe, rng.below(ACTORS)][rng.below(2) as usize];
    let actor = w.actor(i);
    twin(
        "MsCheckAutoInput",
        a,
        b,
        0x0079_2120,
        &[i, actor],
        ALL,
        |m| ip::ms_check_auto_input(m, i, actor).cast_unsigned(),
    );
    same("MsCheckAutoInput", a, b);
    // the forced commands (a block of 0x48 bytes)
    let cmd = out + 0x20;
    for (k, what, sym) in [
        (0, "MsSetDeathSentence", 0x0079_9cd0),
        (1, "MsSetBerserkCommand", 0x0079_9ac0),
        (2, "MsSetProvokingCommand", 0x0079_9d50),
    ] {
        fill(a, b, rng, cmd, 0x48);
        let i = rng.below(ACTORS);
        let actor = w.actor(i);
        let c = if rng.below(8) == 0 { 0 } else { cmd };
        let flag = rng.below(3);
        let side = rng.below(2);
        let args = [i, actor, c, flag, side];
        // the forced command's record: its target kind (+0x1a bits 2-3)
        // and the cursor's event bits at random (put back after)
        let id = u32::from_le_bytes(
            [b.read(actor + 0x6c6, 2).unwrap(), vec![0, 0]]
                .concat()
                .try_into()
                .unwrap(),
        );
        let rec = ffx_battle::stat::bridge::ms_get_com_data(b, id, 0);
        let old = if readable(a, rec + 0x1a, 2) {
            let v = b.read(rec + 0x1a, 2).unwrap();
            let f = u16::from_le_bytes([v[0], v[1]]) & !0xc0c
                | (u16::try_from(rng.below(4)).unwrap() << 2)
                | 0xc00;
            poke(a, b, rec + 0x1a, &f.to_le_bytes());
            Some(v)
        } else {
            None
        };
        twin(what, a, b, sym, &args, ALL, |m| {
            dm(m, |m| {
                s!(s);
                match k {
                    0 => ip::ms_set_death_sentence(m, &h, &mut s, i, actor, c, flag),
                    1 => ip::ms_set_berserk_command(m, &h, &mut s, i, actor, c, flag, side)
                        .cast_unsigned(),
                    _ => ip::ms_set_provoking_command(m, &h, &mut s, i, actor, c, flag)
                        .cast_unsigned(),
                }
            })
        });
        local_words(a, b);
        same(what, a, b);
        if let Some(v) = old {
            poke(a, b, rec + 0x1a, &v);
        }
    }
    // the cheat (its standing motion reads the actor's positions)
    let i = rng.below(ACTORS);
    if writes(a, b, 0x0079_56e0, i, out + 0x40) && writes(a, b, 0x0079_48e0, i, out + 0x40) {
        set(a, b, ip::CHEAT_POWERUP, 1);
        twin("MsCheatPlayerPowerup", a, b, 0x0078_4720, &[i], 0, |m| {
            dm(m, |m| {
                s!(s);
                ip::ms_cheat_player_powerup(m, &h, &mut s, i);
            });
            0
        });
        local_words(a, b);
        same("MsCheatPlayerPowerup", a, b);
    }
    // the overdrive modes of a death: kills either way between the party
    // (0-6) and the monsters, now and then any two
    for _ in 0..4 {
        let (i, j) = match rng.below(3) {
            0 => (rng.below(7), 0x14 + rng.below(8)),
            1 => (0x14 + rng.below(8), rng.below(7)),
            _ => (rng.below(ACTORS), rng.below(ACTORS)),
        };
        let args = [i, w.actor(i), j, w.actor(j)];
        twin(
            "MsLimitTypeDeathCheck",
            a,
            b,
            0x007b_0f80,
            &args,
            ALL,
            |m| d::ms_limit_type_death_check(m, i, args[1], j, args[3]),
        );
        same("MsLimitTypeDeathCheck", a, b);
    }
    // Auto-Life and Auto-Phoenix (an item in the bag at times)
    let (i, killer) = (rng.below(ACTORS), rng.below(ACTORS));
    let actor = w.actor(i);
    for q in 0..ACTORS {
        let bits = [0u16, 0x1000, half16(rng)][rng.below(3) as usize];
        poke(a, b, w.actor(q) + 0x6bc, &bits.to_le_bytes());
    }
    let status = [0, 0x200, u32::from(half16(rng))][rng.below(3) as usize];
    twin(
        "MsAutoRelifeProcess",
        a,
        b,
        0x007b_2910,
        &[i, actor, killer, status],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                d::ms_auto_relife_process(m, &h, &mut s, i, actor, killer, status).cast_unsigned()
            })
        },
    );
    local_words(a, b);
    same("MsAutoRelifeProcess", a, b);
    twin(
        "MsAutoPhenixProcess",
        a,
        b,
        0x007b_27b0,
        &[i, actor, killer],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                d::ms_auto_phenix_process(m, &h, &mut s, i, actor, killer)
            })
        },
    );
    local_words(a, b);
    same("MsAutoPhenixProcess", a, b);
    // deaths by damage (three; the spoils and auto-battle through the stubs)
    for _ in 0..3 {
        set(a, b, d::NO_OVERKILL, flag(rng, 3));
        let i = rng.below(ACTORS);
        let j = [
            rng.below(ACTORS),
            8 + rng.below(7),
            0xf + rng.below(3),
            0x14 + rng.below(8),
        ][rng.below(4) as usize];
        let v = w.actor(j);
        poke(a, b, v + 0x617, &[flag(rng, 4)]);
        // mostly out of HP (else only a removal kills)
        let hp = [0, -5, 0, 1, rng.range(-100, 100)][rng.below(5) as usize];
        poke(a, b, v + 0x5d0, &hp.to_le_bytes());
        poke(a, b, v + 0xdcc, &[flag(rng, 5)]);
        poke(a, b, v + 0x10, &[flag(rng, 6) ^ 1]);
        poke(a, b, v + 0x701, &[[0, 0, 1, 3][rng.below(4) as usize]]);
        for o in [0xdcd, 0xdce, 0xdd0, 0x6d0, 0x3f5] {
            poke(a, b, v + o, &[flag(rng, 3)]);
        }
        poke(a, b, v + 0xf60, &rng.range(-2, 3).to_le_bytes());
        let no_reward = u32::from(flag(rng, 3));
        let flags = rng.below(3);
        let dying_ok =
            writes(a, b, 0x0079_56e0, j, out + 0x40) && writes(a, b, 0x0079_48e0, j, out + 0x40);
        if dying_ok && placed(a, b, w, out) {
            twin(
                "MsDamageCheckDeath",
                a,
                b,
                0x0078_c740,
                &[i, j, no_reward, flags],
                ALL,
                |m| {
                    dm(m, |m| {
                        s!(s);
                        d::ms_damage_check_death(m, &h, &mut s, i, j, no_reward, flags)
                    })
                },
            );
            local_words(a, b);
            unplaced_rows(a, b, out);
            same("MsDamageCheckDeath", a, b);
        }
    }
    // the damage step's pieces (ffx_battle::chr_damage)
    {
        use ffx_battle::chr_damage::bridge as cd;
        twin("MsScreenFocusResetAll", a, b, 0x007b_bda0, &[], 0, |m| {
            dm(m, |m| {
                cd::ms_screen_focus_reset_all(m, &mut call);
            });
            0
        });
        same("MsScreenFocusResetAll", a, b);
        let party: Vec<u8> = (0..7)
            .map(|_| [0xff, u8::try_from(rng.below(ACTORS)).unwrap()][rng.below(2) as usize])
            .collect();
        poke(a, b, ffx_battle::menu::bridge::PARTY.ptr(b).addr, &party);
        // a free slot (0xff) names the spare record: its +0x436 does not
        // count
        let spare = ffx_battle::actor::bridge::CHR_SPARE.ptr(b).addr;
        poke(a, b, spare + 0x436, &[flag(rng, 2)]);
        for q in 0..ACTORS {
            let p = w.actor(q);
            for o in [0x436, 0x41c, 0xde7, 0x508, 0x432, 0x41d, 0xe13] {
                poke(
                    a,
                    b,
                    p + o,
                    &[flag(rng, 3) ^ u8::from(o == 0x41c || o == 0x41d)],
                );
            }
            let hits: Vec<u8> = (0..4)
                .map(|_| u8::try_from(rng.below(3)).unwrap())
                .collect();
            poke(a, b, p + 0x774, &hits);
            let hits: Vec<u8> = (0..4)
                .map(|_| u8::try_from(rng.below(3)).unwrap())
                .collect();
            poke(a, b, p + 0xa4c, &hits);
            // (kind 8 ends actors 0x10 and 0x11 first: on them it recurses
            // for good, in the original too)
            let kind = [0, 8, rng.byte()][rng.below(3) as usize];
            let kind = if (0x10..0x12).contains(&q) && kind == 8 {
                0
            } else {
                kind
            };
            poke(a, b, p + 0xdea, &[kind]);
            let e = [0, rng.next()][rng.below(2) as usize];
            poke(a, b, p + 0xea0, &e.to_le_bytes());
        }
        twin("MsCheckPartyDying", a, b, 0x0079_2ee0, &[], ALL, |m| {
            cd::ms_check_party_dying(m).cast_unsigned()
        });
        let i = rng.below(ACTORS);
        let end = [i, 0xff, rng.below(ACTORS)][rng.below(3) as usize];
        set(
            a,
            b,
            ffx_battle::action::bridge::TURN_END_ACTOR,
            u8::try_from(end).unwrap(),
        );
        twin(
            "MsCheckRestMotion",
            a,
            b,
            0x0079_1780,
            &[i, w.actor(i)],
            ALL,
            |m| cd::ms_check_rest_motion(m, i, w.actor(i)),
        );
        let (x, y) = (rng.below(3), rng.below(3));
        twin(
            "MsCheckHitPending",
            a,
            b,
            0x0078_ee80,
            &[x, y, i],
            ALL,
            |m| cd::ms_check_hit_pending(m, x, y, i),
        );
        let cams: Vec<u8> = (0..8)
            // (a camera ID's low byte the camera, one of three)
            .flat_map(|_| {
                [0, (rng.next() & !0xff) | rng.below(3)][rng.below(2) as usize].to_le_bytes()
            })
            .collect();
        poke(a, b, mn::EFFECT_CAMERA_BUF.ptr(b).addr, &cams);
        set(
            a,
            b,
            ffx_battle::parse::bridge::COMMAND_STARTED,
            flag(rng, 2),
        );
        let keep = rng.below(2);
        twin("MsEffectCameraEnd", a, b, 0x0078_7760, &[keep], 0, |m| {
            dm(m, |m| {
                s!(s);
                cd::ms_effect_camera_end(m, &mut s, keep);
            });
            0
        });
        local_words(a, b);
        same("MsEffectCameraEnd", a, b);
        set(a, b, mn::NUMBERS_KIND1, flag(rng, 4));
        set(a, b, cd::COMMAND_HOLD2, u32::from(flag(rng, 4)));
        let cmd = [0x3056, 0x3103, 0x4086, 0x3001, rng.below(0x8000)][rng.below(5) as usize];
        set(a, b, ffx_battle::parse::bridge::PARSED_COMMAND, cmd);
        let title = [0, 1, 0x2d, 0x2e, -1, rng.byte().cast_signed()][rng.below(6) as usize];
        set(a, b, st::TITLE_WINDOW, [title, 0]);
        twin(
            "MsCheckCommandEnd",
            a,
            b,
            0x0079_1690,
            &[w.actor(i)],
            ALL,
            |m| cd::ms_check_command_end(m, w.actor(i)),
        );
        same("MsCheckCommandEnd", a, b);
        let j = [0x10, 0x11, 0xff, rng.below(ACTORS)][rng.below(4) as usize];
        twin("MsMagicEffectEnd", a, b, 0x0079_e830, &[j], ALL, |m| {
            dm(m, |m| {
                cd::ms_magic_effect_end(m, &mut call, j).cast_unsigned()
            })
        });
        same("MsMagicEffectEnd", a, b);
        // a command's end: everyone put back, the petrified that ran away
        // out of place shattering (their positions read)
        for q in 0..ACTORS {
            for o in [0xdce, 0xdcf] {
                poke(a, b, w.actor(q) + o, &[flag(rng, 3)]);
            }
        }
        set(a, b, mn::DIR_FREE, flag(rng, 4));
        let all = rng.below(2);
        let out_ok = (0..ACTORS).all(|q| {
            let f = b.read(w.actor(q) + 0xdce, 2).unwrap();
            f[0] == 0
                || f[1] != 0
                || (writes(a, b, 0x0079_56e0, q, out + 0x40)
                    && writes(a, b, 0x0079_48e0, q, out + 0x40))
        });
        if out_ok && placed(a, b, w, out) {
            twin(
                "MsCommandReturnChrStatusAll",
                a,
                b,
                0x0079_3e00,
                &[all],
                0,
                |m| {
                    dm(m, |m| {
                        s!(s);
                        cd::ms_command_return_chr_status_all(m, &h, &mut s, all);
                    });
                    0
                },
            );
            local_words(a, b);
            unplaced_rows(a, b, out);
            same("MsCommandReturnChrStatusAll", a, b);
        }
        // Poison at a turn's end (the hit motion reads the positions)
        let i = rng.below(ACTORS);
        let end = [i, 0xff, rng.below(ACTORS)][rng.below(3) as usize];
        set(
            a,
            b,
            ffx_battle::action::bridge::TURN_END_ACTOR,
            u8::try_from(end).unwrap(),
        );
        poke(a, b, w.actor(i) + 0x5ba, &[rng.byte()]);
        if writes(a, b, 0x0079_56e0, i, out + 0x40)
            && writes(a, b, 0x0079_48e0, i, out + 0x40)
            && placed(a, b, w, out)
        {
            twin(
                "MsPoisonProcess",
                a,
                b,
                0x007a_fab0,
                &[i, w.actor(i)],
                0,
                |m| {
                    dm(m, |m| {
                        s!(s);
                        cd::ms_poison_process(m, &h, &mut s, i, w.actor(i));
                    });
                    0
                },
            );
            local_words(a, b);
            unplaced_rows(a, b, out);
            same("MsPoisonProcess", a, b);
        }
    }
    // what a hit leaves behind (ffx_battle::after_damage)
    {
        use ffx_battle::after_damage::bridge as ad;
        let i = rng.below(ACTORS);
        let p = w.actor(i);
        let n = [
            rng.range(-500, 500),
            rng.range(-20, 300),
            rng.next().cast_signed(),
        ][rng.below(3) as usize];
        let flags = rng.below(8);
        let style = [0, 1, 2, 0x81][rng.below(4) as usize];
        let args = [i, p, n.cast_unsigned(), rng.next(), 0, flags, style];
        twin("MsSubMP", a, b, 0x0078_e3b0, &args, 0, |m| {
            ad::ms_sub_mp(m, i, p, n, args[3].cast_signed(), 0, flags, style);
            0
        });
        same("MsSubMP", a, b);
        let args = [i, p, n.cast_unsigned(), 0, flags, style];
        twin("MsSubCTB", a, b, 0x0078_e1e0, &args, 0, |m| {
            ad::ms_sub_ctb(m, i, p, n, 0, flags, style);
            0
        });
        same("MsSubCTB", a, b);
        let i = rng.below(ACTORS);
        let p = w.actor(i);
        poke(a, b, p + 0x606, &[flag(rng, 2)]);
        twin("MsAliveProcess", a, b, 0x0078_9160, &[i, p], ALL, |m| {
            dm(m, |m| {
                s!(s);
                ad::ms_alive_process(m, &mut s, i, p)
            })
        });
        local_words(a, b);
        same("MsAliveProcess", a, b);
        let (by, helper) = (rng.below(ACTORS), w.actor(rng.below(ACTORS)));
        poke(a, b, p + 0x446, &[[0, 1, 2][rng.below(3) as usize]]);
        // mostly a helper free to act (MsAutoCommandStatusCheck)
        if rng.below(3) != 0 {
            for (o, v) in [
                (0xdce, 0u8),
                (0x606, 0),
                (0x607, 0),
                (0x608, 0),
                (0x617, 0),
                (0xdc8, 1),
            ] {
                poke(a, b, helper + o, &[v]);
            }
        }
        twin(
            "MsAutoRelifeProcess",
            a,
            b,
            0x0078_d8d0,
            &[i, p, by, helper],
            ALL,
            |m| {
                dm(m, |m| {
                    s!(s);
                    ad::ms_auto_relife_command(m, &h, &mut s, i, p, by, helper)
                })
            },
        );
        local_words(a, b);
        same("MsAutoRelifeProcess", a, b);
        let got = rng.below(2);
        let (what, sym) = [
            ("MsSetStealEffect", 0x0079_ed00),
            ("MsSetStealGillEffect", 0x0079_ed20),
            ("MsSetChrEffectNext", 0x0079_ebe0),
        ][rng.below(3) as usize];
        let y = rng.next();
        twin(what, a, b, sym, &[i, got, y], 0, |m| {
            dm(m, |m| {
                let mut c = game_call;
                match sym {
                    0x0079_ed00 => ad::ms_set_steal_effect(m, &mut c, i, got),
                    0x0079_ed20 => ad::ms_set_steal_gill_effect(m, &mut c, i, got),
                    _ => {
                        ad::ms_set_chr_effect_next(m, &mut c, i, got, y);
                    }
                }
            });
            0
        });
        same(what, a, b);
    }
    // sounds where the actors are, stone, warded statuses
    {
        use ffx_battle::after_damage::bridge as ad;
        let nodes = b.rebase(ad::SEP_3D_NODES);
        let mut area = vec![0u8; 0x200];
        for k in 0..16 {
            area[0x20 * k] = flag(rng, 2);
            area[0x20 * k + 8..0x20 * k + 12]
                .copy_from_slice(&(0x8000_0000 | rng.below(4)).to_le_bytes());
        }
        // a list of some of them (at times the last linked to itself)
        let n = rng.below(5);
        let list = ffx_battle::minor::bridge::SEP_3D.ptr(b).addr;
        let mut head = 0u32;
        for k in 0..n {
            let at = nodes + 0x20 * k;
            if k == 0 {
                head = at;
            } else {
                let o = (0x20 * (k - 1) + 0xc) as usize;
                area[o..o + 4].copy_from_slice(&at.to_le_bytes());
            }
        }
        if n > 0 && rng.below(4) == 0 {
            let o = (0x20 * (n - 1) + 0xc) as usize;
            area[o..o + 4].copy_from_slice(&(nodes + 0x20 * (n - 1)).to_le_bytes());
        }
        poke(a, b, nodes, &area);
        poke(a, b, list, &head.to_le_bytes());
        set(a, b, ad::SEP_3D_OFF, flag(rng, 8));
        set(
            a,
            b,
            ad::LAST_SE,
            [0, 0x8000_0059, 0x8000_0001][rng.below(3) as usize],
        );
        let i = [rng.below(ACTORS), 0xff][rng.below(4).min(1) as usize];
        let se = [0, 0x59, 1, 0x8000_0002 | rng.below(2)][rng.below(4) as usize];
        let kind = [0, 1, 2, 3, 4, 0x103, 5][rng.below(7) as usize];
        let posv = out + 0x60;
        poke(a, b, posv, &floats(rng, 4));
        let pos = [0, posv][rng.below(2) as usize];
        let once = rng.below(2);
        let placed_i = i == 0xff
            || (writes(a, b, 0x0079_56e0, i, out + 0x40)
                && writes(a, b, 0x0079_48e0, i, out + 0x40));
        if placed_i {
            match rng.below(5) {
                0 => {
                    let args = [kind, i, se, once, pos];
                    twin("MsSEP3Dregist", a, b, 0x007a_0a30, &args, ALL, |m| {
                        let mut c = game_call_o;
                        ad::ms_sep_3d_regist(m, &stand_in, &mut c, kind, i, se, once, pos)
                    });
                }
                1 => {
                    twin("MsRegSEplay", a, b, 0x007a_0100, &[i, se], ALL, |m| {
                        let mut c = game_call_o;
                        ad::ms_reg_se_play(m, &stand_in, &mut c, i, se)
                    });
                    // MsRegSEplay3: the same unless hasted (the sound just
                    // played forgotten, mostly)
                    if rng.below(4) != 0 {
                        set(a, b, ad::LAST_SE, 0);
                    }
                    let p = ffx_battle::actor::bridge::ms_get_chr(b, i & 0xff);
                    poke(a, b, p + 0x613, &[u8::from(rng.below(2) == 0)]);
                    twin("MsRegSEplay3", a, b, 0x007a_0170, &[i, se], ALL, |m| {
                        let mut c = game_call_o;
                        ad::ms_reg_se_play3(m, &stand_in, &mut c, i, se)
                    });
                }
                2 => twin("MsRegSEplay2", a, b, 0x007a_0140, &[i, se], ALL, |m| {
                    let mut c = game_call_o;
                    ad::ms_reg_se_play2(m, &stand_in, &mut c, i, se)
                }),
                3 if i != 0xff => {
                    let v = out + 0x80;
                    let top = rng.below(2) == 0;
                    let sym = if top { 0x0079_5980 } else { 0x0079_4c20 };
                    twin("MsGetChrTopPos/CenterPos", a, b, sym, &[i, v], ALL, |m| {
                        if top {
                            t::ms_get_chr_top_pos(m, &stand_in, i, v).cast_unsigned()
                        } else {
                            t::ms_get_chr_center_pos(m, &stand_in, i, v).cast_unsigned()
                        }
                    });
                }
                _ => {
                    let ii = i % ACTORS;
                    let p = w.actor(ii);
                    poke(
                        a,
                        b,
                        p + 0x606,
                        &[[0, 4, rng.byte()][rng.below(3) as usize]],
                    );
                    twin("MsStoneProcess", a, b, 0x0078_e150, &[ii, p], ALL, |m| {
                        let mut c = game_call_o;
                        ad::ms_stone_process(m, &stand_in, &mut c, ii, p)
                    });
                }
            }
            same("sounds", a, b);
        }
        let bits = rng.next() & 0xfff8 | [0, 0x8000][rng.below(2) as usize];
        let j = rng.below(ACTORS);
        twin(
            "MsStatusDefenseEffect",
            a,
            b,
            0x0079_ee20,
            &[0, j, bits],
            ALL,
            |m| {
                let mut c = game_call_o;
                ad::ms_status_defense_effect(m, &mut c, 0, j, bits)
            },
        );
        same("MsStatusDefenseEffect", a, b);
    }
    // the overdrive modes of a hit, Auto-Med, Auto-Potion and the whole
    // AfterDamageProcess
    {
        use ffx_battle::after_damage::bridge as ad;
        use ffx_battle::save::bridge as sv;
        let side = |rng: &mut Xorshift| match rng.below(3) {
            0 => rng.below(7),
            1 => 0x14 + rng.below(8),
            _ => rng.below(ACTORS),
        };
        for q in 0..ACTORS {
            let p = w.actor(q);
            let mode = [0u8, 1, 2, 3, 4, 5, 6, 10, 0x13, rng.byte()][rng.below(10) as usize];
            poke(a, b, p + 0x5bb, &[mode]);
            // a strength to divide by
            poke(a, b, p + 0x6f4, &rng.range(1, 3000).to_le_bytes());
            poke(a, b, p + 0x594, &rng.range(1, 99_999).to_le_bytes());
        }
        // the cure items in the bag at times
        let ids = sv::ITEM_IDS.ptr(b).addr;
        let counts = sv::ITEM_COUNTS.ptr(b).addr;
        for (slot, item) in [
            0x2000u16, 0x2001, 0x2002, 0x200a, 0x200c, 0x200d, 0x200e, 0x200f,
        ]
        .into_iter()
        .enumerate()
        {
            let slot = u32::try_from(slot).unwrap();
            poke(a, b, ids + 2 * slot, &item.to_le_bytes());
            poke(a, b, counts + slot, &[[0, 0, 1, 5][rng.below(4) as usize]]);
        }
        set(a, b, sv::USE_ITEM, 0);
        // party and monsters either way, the modes in use the branches read
        for _ in 0..6 {
            let (i, j) = match rng.below(3) {
                0 => (rng.below(7), 0x14 + rng.below(8)),
                1 => (0x14 + rng.below(8), rng.below(7)),
                _ => (side(rng), side(rng)),
            };
            for q in [i, j] {
                let p = w.actor(q);
                let mode = [0u8, 0, 1, 2, 3, 3, 4, 5, 6, 10, 0x13, 0x13, rng.byte()]
                    [rng.below(13) as usize];
                poke(a, b, p + 0x5bb, &[mode]);
                // a strength the hit outdoes at times (mode 0's cap)
                poke(
                    a,
                    b,
                    p + 0x6f4,
                    &[rng.range(1, 300), rng.range(1, 3000)][rng.below(2) as usize].to_le_bytes(),
                );
                // at times a gauge that fills (MsLimitUp: not KO'd or
                // petrified, empty, no cap in sight)
                if rng.below(2) == 0 {
                    for (o, v) in [
                        (0xdcc, 0u8),
                        (0xdce, 0),
                        (0x5bc, 0),
                        (0x5bd, 0xff),
                        (0x617, 0),
                    ] {
                        poke(a, b, p + o, &[v]);
                    }
                }
            }
            let (by, actor) = (w.actor(i), w.actor(j));
            let hp = [
                rng.range(-500, 500),
                rng.range(1, 5000),
                -rng.range(1, 5000),
                0,
            ][rng.below(4) as usize];
            let raw = rng.range(0, 5000);
            let crit = rng.below(2);
            let args = [
                i,
                by,
                j,
                actor,
                hp.cast_unsigned(),
                raw.cast_unsigned(),
                crit,
            ];
            twin(
                "MsLimitTypeDamageCheck",
                a,
                b,
                0x007b_0d50,
                &args,
                ALL,
                |m| ad::ms_limit_type_damage_check(m, i, by, j, actor, hp, raw, crit),
            );
            same("MsLimitTypeDamageCheck", a, b);
            let n = [0, 1, i32::from(rng.byte())][rng.below(3) as usize];
            let bits = rng.below(4);
            let args = [i, by, j, actor, n.cast_unsigned(), bits];
            twin(
                "MsLimitTypeStatusCheck",
                a,
                b,
                0x007b_12c0,
                &args,
                ALL,
                |m| ad::ms_limit_type_status_check(m, i, by, j, actor, n, bits),
            );
            same("MsLimitTypeStatusCheck", a, b);
        }
        // Auto-Med and Auto-Potion: mostly free to act
        let (i, by) = (rng.below(ACTORS), rng.below(ACTORS));
        let actor = w.actor(i);
        if rng.below(3) != 0 {
            for (o, v) in [(0xdce, 0u8), (0x607, 0), (0x608, 0), (0x617, 0), (0xdc8, 1)] {
                poke(a, b, actor + o, &[v]);
            }
        }
        let st = [0u8, 2, 8, 0xa, rng.byte() & !4][rng.below(5) as usize];
        poke(a, b, actor + 0x606, &[st]);
        // HP about half (Auto-Potion's limit)
        let max = i32::from_le_bytes(b.read(actor + 0x594, 4).unwrap().try_into().unwrap());
        let hp = [max / 2, max / 2 + 1, max / 4, max][rng.below(4) as usize];
        poke(a, b, actor + 0x5d0, &hp.to_le_bytes());
        for o in [0x609, 0x60a] {
            poke(a, b, actor + o, &[flag(rng, 2)]);
        }
        for o in [0x62a, 0x62c] {
            poke(
                a,
                b,
                actor + o,
                &[[0, 0, rng.byte()][rng.below(3) as usize]],
            );
        }
        let old: Vec<u32> = (0..4).map(|_| u32::from(flag(rng, 3))).collect();
        let args = [i, actor, by, old[0], old[1], old[2], old[3]];
        twin("MsAutoCureProcess", a, b, 0x007b_2510, &args, ALL, |m| {
            dm(m, |m| {
                s!(s);
                ad::ms_auto_cure_process(
                    m, &h, &mut s, i, actor, by, old[0], old[1], old[2], old[3],
                )
            })
        });
        local_words(a, b);
        same("MsAutoCureProcess", a, b);
        twin(
            "MsAutoPotionProcess",
            a,
            b,
            0x007b_2850,
            &[i, actor, by],
            ALL,
            |m| {
                dm(m, |m| {
                    s!(s);
                    ad::ms_auto_potion_process(m, &h, &mut s, i, actor, by)
                })
            },
        );
        local_words(a, b);
        same("MsAutoPotionProcess", a, b);
        after_damage(a, b, rng, w, out);
    }
    // Regen and the status counts before a turn (the motions of those it
    // heals read their positions)
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    let healed_ok = |a: &mut Oracle, b: &mut Oracle| {
        (0..ACTORS).all(|q| {
            let p = w.actor(q);
            let r = b.read(p, 0xdd0).unwrap();
            let hp = i32::from_le_bytes(r[0x5d0..0x5d4].try_into().unwrap());
            r[0x612] == 0
                || r[0xdc8] == 0
                || hp <= 0
                || r[0xdcc] != 0
                || r[0xdce] != 0
                || (writes(a, b, 0x0079_56e0, q, out + 0x40)
                    && writes(a, b, 0x0079_48e0, q, out + 0x40))
        })
    };
    if placed(a, b, w, out) && healed_ok(a, b) {
        twin(
            "MsCheckStatusBeforeAction",
            a,
            b,
            0x007a_f4f0,
            &[i, actor],
            ALL,
            |m| {
                dm(m, |m| {
                    s!(s);
                    ip::ms_check_status_before_action(m, &h, &mut s, i, actor)
                })
            },
        );
        local_words(a, b);
        same("MsCheckStatusBeforeAction", a, b);
    }
    // whole turn starts: a few input entries, three rounds
    for _ in 0..3 {
        whole_turn(a, b, rng, w, out, &h);
    }
}

/// One round of [`turns`]' whole turn starts.
fn whole_turn(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    h: &t::Host<'_>,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::input::bridge as ip;
    use ffx_battle::minor::bridge as mn;
    use ffx_battle::state::bridge as st;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    let n = rng.below(4);
    set(a, b, st::INPUT_CUE_COUNT, i8::try_from(n).unwrap());
    set(
        a,
        b,
        st::COMMAND_CUE_COUNT,
        i8::try_from(rng.below(3)).unwrap(),
    );
    // mostly a fight still on (MsGetBattleEndStatus 0): three party
    // members and three monsters in it, the entries theirs
    let on = rng.below(3) != 0;
    if on {
        // everyone at its own place (+0x6d4 set: MsBtlGetPos reads +0x3b0
        // for a valid group), so the motions find their positions
        let file = ffx_battle::pos::bridge::POS_FILE.get(b);
        if readable(a, file, 2) && b.read(file + 1, 1).unwrap()[0].cast_signed() > 0 {
            for q in 0..ACTORS {
                poke(a, b, w.actor(q) + 0x4fc, &[0]);
                poke(a, b, w.actor(q) + 0x6d4, &[0]);
            }
        }
        let mut party = vec![0xffu8; 7];
        party[..3].copy_from_slice(&[0, 1, 2]);
        poke(a, b, ffx_battle::menu::bridge::PARTY.ptr(b).addr, &party);
        set(a, b, ffx_battle::menu::bridge::END_FORCED, 0);
        for q in [0, 1, 2, 0x14, 0x15, 0x16] {
            let p = w.actor(q);
            for (o, v) in [(0xdd2, 1), (0xdd3, 1), (0xdc8, 1), (0xdcc, 0), (0xdce, 0)] {
                poke(a, b, p + o, &[v]);
            }
            if b.read(p + 0x5d0, 1).unwrap()[0] == 0 {
                poke(a, b, p + 0x5d0, &100i32.to_le_bytes());
            }
        }
    }
    // the turn's actors: the commands they may get
    for q in [0, 1, 2, 0x14] {
        let p = w.actor(q);
        let st6: u16 = [0, 0, 0x100, 0x200, 0x300, 0x400, 0x500][rng.below(7) as usize];
        poke(a, b, p + 0x606, &st6.to_le_bytes());
        poke(a, b, p + 0x608, &[[0, 0, 0, 1][rng.below(4) as usize]]);
        let doom: u16 = [0, 0, 0, 0x4000][rng.below(4) as usize];
        poke(a, b, p + 0x616, &doom.to_le_bytes());
        poke(a, b, p + 0x5c8, &[u8::try_from(rng.below(2)).unwrap()]);
        poke(a, b, p + 0xdf3, &[[0, 1, 0x80][rng.below(3) as usize]]);
        poke(a, b, p + 0x716, &[flag(rng, 4)]);
        // the party's members are party members (+0x591 0x11: the cheat
        // heals them)
        poke(a, b, p + 0x591, &[if q < 3 { 0x11 } else { 0 }]);
        // mostly past the turn's start actions (+0xdf7: their random
        // request blocks rewrite the statuses above)
        poke(a, b, p + 0xdf7, &[flag(rng, 4) ^ 1]);
    }
    let entries: Vec<u8> = (0..n * 8)
        .map(|k| match k % 8 {
            0 if on => [0, 1, 2, 0x14][rng.below(4) as usize],
            0 => u8::try_from(rng.below(ACTORS)).unwrap(),
            1 => [0, 0, 1, rng.byte()][rng.below(4) as usize],
            3 => flag(rng, 5),
            _ => rng.byte(),
        })
        .collect();
    poke(a, b, st::INPUT_CUE.ptr(b).addr, &entries);
    set(
        a,
        b,
        ffx_battle::parse::bridge::COMMAND_FLAG29,
        flag(rng, 8),
    );
    set(a, b, mn::NUMBERS_KIND1, flag(rng, 3));
    set(
        a,
        b,
        ffx_battle::atel::bridge::CAMERA_STATE,
        [0, 0, 0, 0, 1, 2][rng.below(6) as usize],
    );
    set(a, b, ip::AUTO_OFF, flag(rng, 3));
    set(a, b, ip::CHEAT_POWERUP, u32::from(flag(rng, 3)));
    // at times a plain menu turn with the cheat on: no forced command, not
    // automatic, hurt (the heal shows)
    if rng.below(4) == 0 {
        set(a, b, ip::CHEAT_POWERUP, 1);
        for q in [0, 1, 2, 0x14] {
            let p = w.actor(q);
            for (o, v) in [
                (0x606, 0u8),
                (0x607, 0),
                (0x608, 0),
                (0x616, 0),
                (0x617, 0),
                (0xdf3, 0),
            ] {
                poke(a, b, p + o, &[v]);
            }
            let max = i32::from_le_bytes(b.read(p + 0x594, 4).unwrap().try_into().unwrap());
            poke(a, b, p + 0x5d0, &(max / 2).max(1).to_le_bytes());
        }
    }
    // at times Regen without a Zombie (the numbers' wait, not the
    // commands')
    if rng.below(3) == 0 {
        for q in 0..ACTORS {
            let p = w.actor(q);
            let v = b.read(p + 0x606, 1).unwrap()[0] & !2;
            poke(a, b, p + 0x606, &[v]);
        }
    }
    if !placed(a, b, w, out) {
        return;
    }
    // the originals' command block starts out zero, as the port's does
    let stack = a.stack_range();
    a.write(stack.end - 0x8000, &vec![0; 0x8000]).unwrap();
    twin("MsBattleProcess_InputCue", a, b, 0x0079_2a90, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            ip::ms_battle_process_input_cue(m, h, &mut s);
        });
        0
    });
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsBattleProcess_InputCue", a, b);
}

/// `AfterDamageProcess` on hit records built at random: an attacker `i`'s
/// hit `k` on `j`, whose positions are locatable (its motions, the death
/// check and the reaction read them).
fn after_damage(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    for _ in 0..3 {
        after_damage_once(a, b, rng, w, out);
    }
}

/// One [`after_damage`] case.
fn after_damage_once(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::after_damage::bridge as ad;
    use ffx_battle::atel::bridge::Script;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    let (i, j) = (
        rng.below(ACTORS),
        [0, rng.below(ACTORS)][rng.below(3).min(1) as usize],
    );
    let k = rng.below(4);
    let actor = w.actor(j);
    // at times hurt (Auto-Potion) and clean statuses after the hit (free
    // to act: Auto-Med and Auto-Potion go on)
    let clean = rng.below(2) == 0;
    if rng.below(2) == 0 {
        let max = i32::from_le_bytes(b.read(actor + 0x594, 4).unwrap().try_into().unwrap());
        poke(
            a,
            b,
            actor + 0x5d0,
            &[max / 4, max / 2, max / 2 + 1][rng.below(3) as usize].to_le_bytes(),
        );
    }
    // no automatic Threaten (+0x62a, +0x630, +0x636; +0x618 the saved
    // statuses): a stone ending redoes the automatic statuses, which could
    // turn it on unseen within the call (its yaw is the original's garbage)
    for q in 0..ACTORS {
        let p = w.actor(q);
        for o in [0x618, 0x62a, 0x630, 0x636] {
            let v = u16::from_le_bytes(b.read(p + o, 2).unwrap().try_into().unwrap()) & !0x800;
            poke(a, b, p + o, &v.to_le_bytes());
        }
    }
    // never every shared command learnt (its achievement is Steam's)
    let sc = ffx_battle::save::bridge::SHARED_COMMANDS;
    set(a, b, sc, sc.get(b) & 0x7fff);
    for n in 0..2 {
        let rec = actor + 0x774 + 0x2d8 * n;
        let count = rng.below(4);
        let done = rng.below(count + 1);
        let mut head = [0u8; 0x18];
        head[0] = u8::try_from(done).unwrap();
        head[1] = u8::try_from(count).unwrap();
        head[2] = u8::try_from([i, i, i, rng.below(ACTORS)][rng.below(4) as usize]).unwrap();
        head[3] = u8::try_from([k, k, rng.below(4)][rng.below(3) as usize]).unwrap();
        head[4] = [0, 0, 1, 2, 3][rng.below(5) as usize];
        head[5] = [0, 1, 3, 4, rng.byte()][rng.below(5) as usize];
        head[6] = flag(rng, 2);
        head[7] = flag(rng, 4);
        head[8..10].copy_from_slice(&[0i16, 1, 3, -1][rng.below(4) as usize].to_le_bytes());
        head[0xa..0xc].copy_from_slice(
            &u16::try_from(0x2000 + rng.below(0x40))
                .unwrap()
                .to_le_bytes(),
        );
        head[0xc] = flag(rng, 4);
        head[0x10..0x14].copy_from_slice(&[0i32, 100, -5][rng.below(3) as usize].to_le_bytes());
        head[0x17] = u8::try_from(rng.below(ACTORS)).unwrap();
        poke(a, b, rec, &head);
        let st: u16 = u16::from_le_bytes(b.read(actor + 0x606, 2).unwrap().try_into().unwrap());
        for e in 0..count {
            let mut ent = [0u8; 0x2c];
            ent[0] = [1, 5, 6, 8, 2, rng.byte()][rng.below(6) as usize];
            ent[1] = [0, 1, 2][rng.below(3) as usize];
            ent[2] = rng.byte();
            ent[3] = u8::try_from(rng.below(4)).unwrap();
            ent[4] = [0, 1, rng.byte()][rng.below(3) as usize];
            ent[6] = [0, 1, 2, 0x80][rng.below(4) as usize];
            for c in &mut ent[7..0x14] {
                *c = if clean {
                    0
                } else {
                    [0, 0, 1, rng.byte()][rng.below(4) as usize]
                };
            }
            if clean {
                // Darkness, Silence (+0x60a, +0x609) and Regen (+0x612)
                for o in [8, 9, 0x11] {
                    ent[o] = flag(rng, 2);
                }
            }
            // Threaten as it was (turning it on reads the original's garbage)
            let s606 = [st, st ^ 1, st ^ 4, st ^ 9, st ^ 2, half16(rng)][rng.below(6) as usize];
            let s606 = if clean {
                [0, 2, 8, 0xa][rng.below(4) as usize]
            } else {
                s606 & !0x800
            } | st & 0x800;
            ent[0x14..0x16].copy_from_slice(&s606.to_le_bytes());
            let s616 = if clean {
                0
            } else {
                [0u16, 1, 0x200, half16(rng)][rng.below(4) as usize]
            };
            ent[0x16..0x18].copy_from_slice(&s616.to_le_bytes());
            let bits =
                u16::try_from(rng.below(8)).unwrap() | [0, 0x20, 0x8200][rng.below(3) as usize];
            ent[0x18..0x1a].copy_from_slice(&bits.to_le_bytes());
            for o in [0x1c, 0x20, 0x24, 0x28] {
                let v = [rng.range(-300, 300), rng.range(0, 5000), 0][rng.below(3) as usize];
                ent[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            poke(a, b, rec + 0x18 + 0x2c * e, &ent);
        }
    }
    for o in [0x6dc, 0x6dd, 0x612, 0x617] {
        poke(a, b, actor + o, &[flag(rng, 3)]);
    }
    let auto = [0u16, 0x400, 0x400, 0x800, 0xc00, half16(rng)][rng.below(6) as usize];
    if clean {
        poke(a, b, actor + 0xdc8, &[1]);
    }
    poke(a, b, actor + 0x6bc, &auto.to_le_bytes());
    // mostly applying entries (bit 3 shows nothing and applies none)
    let flags = rng.below(0x40) & !1 & !(u32::from(rng.below(3) != 0) << 3)
        | [0, 0x400][rng.below(2) as usize];
    let dst = out + 0xa0;
    poke(a, b, dst, &0xeeee_eeeeu32.to_le_bytes());
    // MsSetDamage at times: the hit is the command's (+2 of i's own block),
    // the selector j alone or a whole side
    let whole = rng.below(2) == 0;
    let sel = [j, j, 0xffff_fff1, 0xffff_fff2, 0xffff_ffeb][rng.below(5) as usize];
    let them: Vec<u32> = if whole && sel != j {
        (0..ACTORS).collect()
    } else {
        vec![i, j]
    };
    let ok = them.iter().all(|&q| {
        writes(a, b, 0x0079_56e0, q, out + 0x40) && writes(a, b, 0x0079_48e0, q, out + 0x40)
    });
    if !ok || !placed(a, b, w, out) {
        return;
    }
    if whole {
        let user = w.actor(i);
        poke(a, b, user + 0xde5, &[0xff]);
        let k8 = u8::try_from(k).unwrap();
        poke(
            a,
            b,
            user + 0x72c + 2,
            &[k8, [k8, k8 + 1, k8 + 2][rng.below(3) as usize]],
        );
        poke(a, b, user + 0xde9, &[flag(rng, 2) ^ 1]);
        poke(
            a,
            b,
            user + 0xde4,
            &[[0xff, 0, 1, 2][rng.below(4) as usize]],
        );
        poke(a, b, user + 0x6ca, &[flag(rng, 2)]);
        poke(a, b, user + 0xdcc, &[flag(rng, 3)]);
        let all: Vec<u32> = (0..ACTORS).collect();
        let was = threats(b, w, &all);
        twin(
            "MsSetDamage",
            a,
            b,
            0x0078_d980,
            &[i, sel, flags],
            ALL,
            |m| {
                dm(m, |m| {
                    let mut s = Script {
                        call: &mut call,
                        signal: &mut signal,
                        off: &mut off,
                    };
                    ad::ms_set_damage(m, &h, &mut s, i, sel, flags)
                })
            },
        );
        threatened(a, b, w, &all, &was);
        local_words(a, b);
        unplaced_rows(a, b, out);
        same("MsSetDamage", a, b);
        return;
    }
    let was = threats(b, w, &[j]);
    twin(
        "AfterDamageProcess",
        a,
        b,
        0x0078_f060,
        &[i, k, j, dst, flags],
        ALL,
        |m| {
            dm(m, |m| {
                let mut s = Script {
                    call: &mut call,
                    signal: &mut signal,
                    off: &mut off,
                };
                ad::after_damage_process(m, &h, &mut s, i, k, j, dst, flags)
            })
        },
    );
    threatened(a, b, w, &[j], &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("AfterDamageProcess", a, b);
}

/// Who of `them` has Threaten on (`+0x606` bit 11).
fn threats(b: &mut Oracle, w: &World, them: &[u32]) -> Vec<bool> {
    them.iter()
        .map(|&q| b.read(w.actor(q) + 0x607, 1).unwrap()[0] & 8 != 0)
        .collect()
}

/// After a call that may have turned Threaten on for one of `them` (an
/// entry restoring it after a revival cleared it; a death may clear it
/// again within the same call): that actor's first yaw (`+0x51c`) and the
/// vector unit's rows are the original's stack garbage
/// (`MsThreatProcess`), copied from `a` for each of them threatened
/// before (`was`) or after. A threatened actor's target gets its yaw from
/// the same local and may lose Threaten again in the call too: as
/// `MsThreatProcess` is the only writer of `+0x51c`, any of `them` whose
/// yaw differs is copied as well (round 27).
fn threatened(a: &mut Oracle, b: &mut Oracle, w: &World, them: &[u32], was: &[bool]) {
    let now = threats(b, w, them);
    for (k, &q) in them.iter().enumerate() {
        let at = w.actor(q) + 0x51c;
        if was[k] || now[k] || a.read(at, 4).unwrap() != b.read(at, 4).unwrap() {
            copy_garbage(a, b, at, 4);
        }
    }
}

/// After a call: when some actor's battle position cannot be located any
/// more (the call gave it a bad position record: an aeon summoned in by
/// an exchange takes the summoner's group, the spare record's when there
/// is none), `MsCheckBtlPos` left that vector's x and z alone and the
/// original's `MsPointLen` loaded its stack garbage into `VF_M`'s first
/// row (the difference) and third row (the vector): those two words are
/// copied from `a`. With every actor placed they are compared
/// (`docs/formats/battle.md`).
fn unplaced_rows(a: &mut Oracle, b: &mut Oracle, probe: u32) {
    let probe = probe + 0x40;
    let keep = a.read(probe, 16).unwrap();
    // (the probes' own register traffic, the emulated EE and VU0
    // registers, put back after)
    let regs = ffx_battle::vector::bridge::VF_M.ptr(b).addr - 0x300;
    let saved = b.read(regs, 0x400).unwrap();
    let lost = (0..ACTORS).any(|k| {
        b.write(probe, &[0xee; 16]).unwrap();
        ffx_battle::pos::bridge::ms_get_chr_btl_pos(b, &stand_in, k, probe);
        b.read(probe, 12).unwrap().contains(&0xee)
    });
    b.write(probe, &keep).unwrap();
    b.write(regs, &saved).unwrap();
    if lost {
        let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
        for at in [m, m + 0x20] {
            let v = a.read(at, 16).unwrap();
            b.write(at, &v).unwrap();
        }
    }
}

/// Whether every actor's battle position is written (`MsGetChrBtlPos`):
/// `MsCalcChrDir`'s centres, turning monsters and `MsCheckBtlPos`, which
/// any actor may reach (`MsResetChrPos`). For an actor whose position
/// record cannot be located the original leaves its vector's x and z
/// alone and `MsPointLen` subtracts its stack garbage
/// (`docs/formats/battle.md`).
fn placed(a: &mut Oracle, b: &mut Oracle, w: &World, out: u32) -> bool {
    let _ = w;
    (0..ACTORS).all(|k| writes(a, b, 0x0079_48e0, k, out + 0x40))
}

/// A short list of 3D sounds (playing 0x80000000-3, at times fading)
/// in place of the image's or snapshot's (whose nodes may be stale).
fn sep_3d_list(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let nodes = b.rebase(ffx_battle::after_damage::bridge::SEP_3D_NODES);
    let mut area = vec![0u8; 0x200];
    let n = rng.below(5);
    for k in 0..n as usize {
        let at = nodes + 0x20 * u32::try_from(k).unwrap();
        area[0x20 * k + 4] = [0, 0, 1, 3][rng.below(4) as usize];
        area[0x20 * k + 8..0x20 * k + 12]
            .copy_from_slice(&(0x8000_0000 | rng.below(4)).to_le_bytes());
        let next = if k + 1 < n as usize { at + 0x20 } else { 0 };
        area[0x20 * k + 0xc..0x20 * k + 0x10].copy_from_slice(&next.to_le_bytes());
    }
    poke(a, b, nodes, &area);
    let head = if n == 0 { 0 } else { nodes };
    poke(
        a,
        b,
        ffx_battle::minor::bridge::SEP_3D.ptr(b).addr,
        &head.to_le_bytes(),
    );
}

/// The end of a command's damage step (`ffx_battle::damage_end`): sound
/// fades, script cancels, effect frees, model disposal, the aeon placed,
/// the party back from a summon and the whole per-frame step.
fn damage_end(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::damage_end::bridge as de;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    // sound fades on a short list (playing 0x80000000-3, at times fading)
    {
        sep_3d_list(a, b, rng);
        let se = [0, 1, 2, 3, rng.next()][rng.below(5) as usize];
        let frames = rng.next();
        let i = rng.below(ACTORS);
        match rng.below(3) {
            0 => twin(
                "MsSEP3DFind",
                a,
                b,
                0x007a_0b90,
                &[se | 0x8000_0000],
                ALL,
                |m| de::ms_sep_3d_find(m, se | 0x8000_0000),
            ),
            1 => twin(
                "MsSEP3DfadeOut",
                a,
                b,
                0x007a_08a0,
                &[se | 0x8000_0000, frames],
                ALL,
                |m| de::ms_sep_3d_fade_out(m, se | 0x8000_0000, frames),
            ),
            _ => twin(
                "MsRegSEfade",
                a,
                b,
                0x007a_00d0,
                &[i, se, frames],
                ALL,
                |m| de::ms_reg_se_fade(m, i, se, frames).cast_unsigned(),
            ),
        }
        same("sound fades", a, b);
    }
    // a script handler cancelled
    let (kind, i, n) = (
        [2, 2, 3, 4][rng.below(4) as usize],
        rng.below(ACTORS),
        rng.range(-1, 0x22),
    );
    twin(
        "MsAtelRequestCancel",
        a,
        b,
        0x0079_7cd0,
        &[kind, i, n.cast_unsigned()],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                de::ms_atel_request_cancel(m, &mut s, kind, i, n).cast_unsigned()
            })
        },
    );
    same("MsAtelRequestCancel", a, b);
    // a magic effect ends (its function a logging stub)
    let i = [rng.below(ACTORS), 0xff][rng.below(6).min(1) as usize];
    if i != 0xff {
        let p = w.actor(i);
        poke(
            a,
            b,
            p + 0xdfb,
            &[[6, 6, 0, rng.byte()][rng.below(4) as usize]],
        );
        poke(
            a,
            b,
            p + 0xdfe,
            &[[2, 0, rng.byte()][rng.below(3) as usize]],
        );
        let eff = if rng.below(2) == 0 {
            0
        } else {
            let f = b.rebase(FREE_STUBS + 0x40 * 8);
            poke(a, b, out + 0xc0, &[0, f].map(u32::to_le_bytes).concat());
            out + 0xc0
        };
        poke(a, b, p + 0xf8c, &eff.to_le_bytes());
    }
    set(
        a,
        b,
        ffx_battle::parse::bridge::EFFECT_READ_STAT,
        [0, rng.next()][rng.below(2) as usize],
    );
    twin("MsEffectFree", a, b, 0x0078_7af0, &[i], 0, |m| {
        dm(m, |m| {
            s!(s);
            de::ms_effect_free(m, &mut s, i);
        });
        0
    });
    same("MsEffectFree", a, b);
    // models and weapons disposed
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    for o in [0x11, 0x12, 0x3e] {
        poke(a, b, p + o, &[flag(rng, 2)]);
    }
    for o in [4, 8, 0x48] {
        poke(
            a,
            b,
            p + o,
            &[0, rng.next() | 1][rng.below(2) as usize].to_le_bytes(),
        );
    }
    let flags = rng.below(0x10);
    match rng.below(3) {
        0 => twin(
            "MsDebugFullCommand_00796dc0",
            a,
            b,
            0x0079_6dc0,
            &[i, p, flags],
            ALL,
            |m| {
                dm(m, |m| {
                    s!(s);
                    de::ms_chr_dispose(m, &mut s, i, p, flags)
                })
            },
        ),
        1 => {
            let k = rng.below(2);
            let wp = ffx_battle::actor::bridge::ms_get_weapon(b, i, k.cast_signed());
            if wp != 0 {
                poke(a, b, wp + 0x3e, &[flag(rng, 2)]);
                for o in [4, 8] {
                    poke(
                        a,
                        b,
                        wp + o,
                        &[0, rng.next() | 1][rng.below(2) as usize].to_le_bytes(),
                    );
                }
            }
            twin(
                "MsWeaponDispose",
                a,
                b,
                0x007a_d750,
                &[i, k, flags, wp],
                0,
                |m| {
                    dm(m, |m| {
                        s!(s);
                        de::ms_weapon_dispose(m, &mut s, i, k, flags, wp);
                    });
                    0
                },
            );
        }
        _ => twin("MsBtlChrFree", a, b, 0x0079_3410, &[i, flags], 0, |m| {
            dm(m, |m| {
                s!(s);
                de::ms_btl_chr_free(m, &mut s, i, flags);
            });
            0
        }),
    }
    same("disposal", a, b);
    // the rest needs every actor placed (positions, motions)
    let all: Vec<u32> = (0..ACTORS).collect();
    if !placed(a, b, w, out)
        || !all
            .iter()
            .all(|&q| writes(a, b, 0x0079_56e0, q, out + 0x40))
    {
        return;
    }
    // no characters for these calls, back after: MsSetChrPosReset resets
    // a character's cloth, which neither the fake ones nor a snapshot's
    // (its cloth data outside the snapshot) have (battle_atel.rs tests it)
    let chrs: Vec<[u8; 4]> = all
        .iter()
        .map(|&q| b.read(w.actor(q), 4).unwrap().try_into().unwrap())
        .collect();
    for &q in &all {
        poke(a, b, w.actor(q), &[0; 4]);
    }
    damage_end_steps(a, b, rng, w, out, &all);
    for (&q, c) in all.iter().zip(&chrs) {
        poke(a, b, w.actor(q), c);
    }
}

/// The placed part of [`damage_end`]: the aeon placed and the whole step.
fn damage_end_steps(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    all: &[u32],
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::damage_end::bridge as de;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    // an aeon placed after its summon (by a party member: an aeon's
    // battle position, kind 0xd, is its summoner's)
    let i = 8 + rng.below(10);
    set(
        a,
        b,
        ffx_battle::target::bridge::SUMMONER,
        i8::try_from(rng.below(7)).unwrap(),
    );
    let party: Vec<u8> = (0..7)
        .map(|_| [0xff, u8::try_from(rng.below(0x12)).unwrap()][rng.below(2) as usize])
        .collect();
    poke(a, b, ffx_battle::ability::bridge::PARTY.ptr(b).addr, &party);
    for q in party.iter().filter(|&&q| q != 0xff) {
        // (not the Magus Sisters' 0x3009-0x300b: a sister whose position
        // fails adds the original's stack garbage, MsGetSummonerPos)
        let cmd = [0x3001u32, 0x3005, 0x3008, 0x300c, rng.next()][rng.below(5) as usize];
        poke(a, b, w.actor(u32::from(*q)) + 4, &cmd.to_le_bytes());
    }
    let was = threats(b, w, all);
    twin("MsSetChrPosAfterSummon", a, b, 0x007a_c6a0, &[i], 0, |m| {
        dm(m, |m| {
            s!(s);
            de::ms_set_chr_pos_after_summon(m, &h, &mut s, i);
        });
        0
    });
    threatened(a, b, w, all, &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsSetChrPosAfterSummon", a, b);
    // the whole step: actors with commands in hand at random states
    for q in 0..ACTORS {
        let p = w.actor(q);
        let st = [0u8, 0, 1, 4, 5, 2, 3][rng.below(7) as usize];
        poke(a, b, p + 0xdf0, &[st]);
        poke(a, b, p + 0xdf2, &[flag(rng, 3)]);
        poke(a, b, p + 0xdfb, &[[0, 0, 6, 1][rng.below(4) as usize]]);
        // kind 8 (MsMagicEffectEnd ends 0x10's and 0x11's first: never
        // theirs, which would recurse for good)
        let kind = u8::try_from(rng.below(if q == 0x10 || q == 0x11 { 8 } else { 9 })).unwrap();
        poke(a, b, p + 0xdea, &[kind]);
        // its own cue entry (the original reads one without checking)
        poke(a, b, p + 0xde5, &[u8::try_from(q).unwrap()]);
        let e = ffx_battle::state::bridge::COMMAND_CUE.ptr(b).addr
            + ffx_battle::state::bridge::CUE_SIZE * q;
        let hits = u8::try_from(rng.below(3)).unwrap();
        poke(
            a,
            b,
            e + 2,
            &[hits, hits + u8::try_from(rng.below(3)).unwrap()],
        );
        poke(a, b, p + 0xf8c, &[0; 4]);
    }
    for (g, v) in [
        (de::DYING_WAIT.ptr(b).addr, flag(rng, 3)),
        (de::CAMERA_RESET.ptr(b).addr, flag(rng, 3)),
        (de::KEEP_FOCUS.ptr(b).addr, flag(rng, 3)),
        (
            ffx_battle::parse::bridge::COMMAND_STARTED.ptr(b).addr,
            flag(rng, 2),
        ),
        (
            ffx_battle::parse::bridge::COMMAND_MUSIC_DOWN.ptr(b).addr,
            flag(rng, 2),
        ),
    ] {
        poke(a, b, g, &[v]);
    }
    set(a, b, ffx_battle::state::bridge::INPUT_CUE_COUNT, 0);
    set(a, b, ffx_battle::state::bridge::COMMAND_CUE_COUNT, 0);
    // the title window about its wait (shown past its top − 20) and no
    // magic file or command wait at times
    let top = rng.range(0x14, 0x60);
    let title = [0, top - 0x14, top - 0x13, top - 0x15, rng.range(-5, 0x60)][rng.below(5) as usize];
    poke(
        a,
        b,
        ffx_battle::exec::bridge::TITLE_WINDOW.ptr(b).addr,
        &[
            i8::try_from(title).unwrap().cast_unsigned(),
            u8::try_from(top).unwrap(),
        ],
    );
    if rng.below(2) == 0 {
        set(a, b, de::MAGIC_HANDLE, -1);
        set(a, b, ffx_battle::parse::bridge::COMMAND_WAIT, -1);
        for &q in all {
            poke(a, b, w.actor(q) + 0x434, &[flag(rng, 2)]);
        }
    }
    // the monsters (MsGetMon, for MsGetBattleEndStatus) are the actors from
    // 0x14
    set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    let was = threats(b, w, all);
    twin(
        "MsBattleProcess_ChrDamage",
        a,
        b,
        0x0079_11e0,
        &[],
        0,
        |m| {
            dm(m, |m| {
                s!(s);
                de::ms_battle_process_chr_damage(m, &h, &mut s);
            });
            0
        },
    );
    threatened(a, b, w, all, &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsBattleProcess_ChrDamage", a, b);
}

/// The battle end's callees of script, vfx, sound and the system
/// (`ffx_battle::battle_end`), logging stubs after the disposal ones; the
/// last is a function the effect tables point at.
const END_CALLEES: [(u32, u8); 16] = {
    use ffx_battle::battle_end::bridge as be;
    [
        (be::TO_ALL_CLOSE_MESSAGE_WINDOW, 0),
        (be::ATEL_IS_BATTLE_END_STATUS_CLEAR9, 0),
        (be::ATEL_IS_BATTLE_END_STATUS_CLEAR8, 0),
        (be::SND_MUSIC_VOLUME_FADE_OUT, 2),
        (be::ATEL_GET_STATUS_WIN_FADE_OUT, 0),
        (be::OP_ET_BATTLE_GENKO_COUNTER_GET, 0),
        (be::SG_FADE_OUT, 1),
        (be::ATEL_BG_SEP_FADE, 2),
        (be::OP_OVL_EFFECT_FORCED_FREE, 0),
        (be::OP_OEF2_SEP_FREE_ALL, 1),
        (be::SG_SET_KEEP_FPS, 1),
        // SndMusicRead(music): the argument is logged (a port that dropped
        // it passed with 0 here)
        (be::SND_MUSIC_READ, 1),
        (be::SND_SYNC_MUSIC_STANDBY, 1),
        (be::SND_MUSIC_REPLAY_POINT_CLEAR, 1),
        (be::SND_MUSIC_START, 1),
        (0, 1),
    ]
};
const END_STUBS: u32 = PROCESS_STUBS + 0x40 * 32;

/// The end's stubs' results: mostly 0, at times one of `odd`.
fn end_results(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, k: u32, odd: &[u32]) {
    let r = if rng.below(3) == 0 {
        odd[rng.below(u32::try_from(odd.len()).unwrap()) as usize]
    } else {
        0
    };
    poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
}

/// The battle end (`ffx_battle::battle_end`): its helpers, then the
/// `g_BtlStatus` steps from random states (every actor placed: the
/// summon return and the motions).
fn battle_end(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    recs: [u32; 4],
    files: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::battle_end::bridge as be;
    let [rec, erec, _, _] = recs;
    // the effect file table (MsEffectFileNumber)
    set(a, b, ffx_battle::exec::bridge::EFFECT_FILES, files);
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    let stub = |b: &mut Oracle, k: u32| b.rebase(END_STUBS + 0x40 * k);
    // the effect tables: the overlay table (+0x20) and the event effect's
    // table ([0]) point at the last stub
    let f = stub(b, 15);
    poke(a, b, out + 0xa0, &f.to_le_bytes());
    poke(a, b, out + 0x80 + 0x20, &f.to_le_bytes());
    poke(
        a,
        b,
        erec + 7 * EXCHANGE_REC + 4,
        &(out + 0x80).to_le_bytes(),
    );
    for k in 0..15 {
        let r = rng.next();
        end_results(a, b, rng, rec, k, &[1, 2, 3, r]);
    }
    set(a, b, ffx_battle::state::bridge::EVENT_EFFECT_B, rng.next());
    let id = [0x29f, rng.below(0x400), rng.next()][rng.below(3) as usize];
    twin("MsEffectForceFree", a, b, 0x0078_7ab0, &[id], ALL, |m| {
        dm(m, |m| be::ms_effect_force_free(m, &mut call, id))
    });
    same("MsEffectForceFree", a, b);
    twin("MsEffectSepClearAll", a, b, 0x0078_82b0, &[], ALL, |m| {
        dm(m, |m| be::ms_effect_sep_clear_all(m, &mut call))
    });
    same("MsEffectSepClearAll", a, b);
    // an event effect read: loads waiting or not, asynchronous or not
    let stat = [0i8, 0, 0, 1, 3, 4][rng.below(6) as usize];
    set(a, b, ffx_battle::state::bridge::EVENT_EFFECT_STAT, stat);
    set(a, b, be::NO_EVENT_MAGIC_FILE, flag(rng, 3));
    for (k, r) in [
        (4, [0, 5, 1, u32::MAX][rng.below(4) as usize]),
        (10, [0, 1, 3][rng.below(3) as usize]),
    ] {
        poke(a, b, erec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    let id = [rng.below(0x400), rng.below(0x40), 0][rng.below(3) as usize];
    twin("MsEventEffectRead", a, b, 0x0078_8c60, &[id], ALL, |m| {
        dm(m, |m| {
            be::ms_event_effect_read(m, &mut call, id).cast_unsigned()
        })
    });
    same("MsEventEffectRead", a, b);
    // an event effect started
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_STAT,
        [3i8, 3, 0, 4][rng.below(4) as usize],
    );
    set(a, b, ffx_battle::state::bridge::EVENT_EFFECT_A, out + 0xa0);
    set(
        a,
        b,
        be::EVENT_EFFECT_COUNT,
        [rng.below(8), rng.next()][rng.below(2) as usize],
    );
    let x = [0, rng.below(0x100), rng.next()][rng.below(3) as usize];
    twin("MsEventEffectStart", a, b, 0x0078_8d60, &[x], ALL, |m| {
        dm(m, |m| {
            be::ms_event_effect_start(m, &mut call, x).cast_unsigned()
        })
    });
    same("MsEventEffectStart", a, b);
    twin("MsSetEndDirAll", a, b, 0x0079_3290, &[], 0, |m| {
        be::ms_set_end_dir_all(m);
        0
    });
    same("MsSetEndDirAll", a, b);
    // the system script checks, with the voice event flag or not
    let voice = ffx_battle::state::bridge::VOICE_EVENT.get(b);
    if voice != 0 {
        poke(a, b, voice, &[flag(rng, 2)]);
    }
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        i8::try_from(flag(rng, 4)).unwrap(),
    );
    let v = [0, 1, 2, 3, 4, rng.next()][rng.below(6) as usize];
    twin(
        "MsAtelRequestSystemCheckExe",
        a,
        b,
        0x0079_82a0,
        &[v, 1, 0, 0],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                be::ms_atel_request_system_check_exe(m, &mut s, v, 1, 0, 0).cast_unsigned()
            })
        },
    );
    same("MsAtelRequestSystemCheckExe", a, b);
    // sounds in transfer
    let t = ffx_battle::minor::bridge::SEP_TRANS_TABLE.ptr(b).addr;
    let words: Vec<u8> = (0..0x80)
        .flat_map(|_| [0, 0, 0, rng.next()][rng.below(4) as usize].to_le_bytes())
        .collect();
    poke(a, b, t, &words);
    set(
        a,
        b,
        ffx_battle::minor::bridge::SEP_TRANS_FLAG,
        u16::try_from(rng.below(0x10000)).unwrap(),
    );
    match rng.below(3) {
        0 => twin("MsSEPbattleClear", a, b, 0x007a_0370, &[], 0, |m| {
            dm(m, |m| be::ms_sep_battle_clear(m, &mut call));
            0
        }),
        1 => {
            // the argument goes on to SndMusicRead (a tail jump): the port
            // once dropped it, and the victory fanfare never loaded. Drawn
            // from a side stream: the shared one stays as it was, so the
            // later cases keep their inputs
            let mut side = Xorshift(rng.0 ^ 0x9e37_79b9 | 1);
            let music = [0x8a, 0x8c, side.next()][side.below(3) as usize];
            twin("MsMusicReadStart", a, b, 0x0079_fff0, &[music], ALL, |m| {
                dm(m, |m| be::ms_music_read_start(m, &mut call, music))
            });
        }
        _ => {
            let music = [0x8a, 0x8c, rng.next()][rng.below(3) as usize];
            twin("MsMusicSync", a, b, 0x007a_0040, &[music], ALL, |m| {
                dm(m, |m| {
                    be::ms_music_sync(m, &mut call, music).cast_unsigned()
                })
            });
        }
    }
    same("music", a, b);
    // overdrive experience for winning
    for q in 0..7 {
        let p = w.actor(q);
        poke(a, b, p + 0xdc8, &[flag(rng, 2)]);
        poke(
            a,
            b,
            p + 0x5bb,
            &[[0xb, 0xb, rng.byte()][rng.below(3) as usize]],
        );
    }
    twin("MsLimitTypeWinCheck", a, b, 0x007b_1540, &[], ALL, |m| {
        be::ms_limit_type_win_check(m)
    });
    same("MsLimitTypeWinCheck", a, b);
    // the rest needs every actor placed for the actions and the summon
    // return; the steps without them run anyway
    let all: Vec<u32> = (0..ACTORS).collect();
    if !placed(a, b, w, out)
        || !all
            .iter()
            .all(|&q| writes(a, b, 0x0079_56e0, q, out + 0x40))
    {
        return;
    }
    let chrs: Vec<[u8; 4]> = all
        .iter()
        .map(|&q| b.read(w.actor(q), 4).unwrap().try_into().unwrap())
        .collect();
    for &q in &all {
        poke(a, b, w.actor(q), &[0; 4]);
    }
    let i = [0xe, rng.below(ACTORS)][rng.below(2) as usize];
    let was = threats(b, w, &all);
    twin("MsDeathYoujinbou", a, b, 0x007b_2ad0, &[i], ALL, |m| {
        dm(m, |m| {
            s!(s);
            be::ms_death_youjinbou(m, &h, &mut s, i).cast_unsigned()
        })
    });
    threatened(a, b, w, &all, &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsDeathYoujinbou", a, b);
    for _ in 0..20 {
        battle_end_status(a, b, rng, w, out, recs);
    }
    for (&q, c) in all.iter().zip(&chrs) {
        poke(a, b, w.actor(q), c);
    }
}

/// One `MsBattleProcess_Status_007917d0` step from a random state.
fn battle_end_status(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    recs: [u32; 4],
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::battle_end::bridge as be;
    let [rec, _, prec, srec] = recs;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    let all: Vec<u32> = (0..ACTORS).collect();
    // the state: the steps weighted alike, a few out of range
    let st = [
        1i8, 2, 2, 2, 2, 0x17, 0x18, 0x18, 0x19, 0x1a, 0x1b, 0x1b, 0x1c, 0x1d, 0x1d, 0x1e, 0x1e,
        0x1e, 0x1f, 0x1f, 0x20, 0x21, 0, 3, 0x16, -1,
    ][rng.below(26) as usize];
    set(a, b, ffx_battle::state::bridge::STATUS, st);
    // the waits: all clear half the time, else each at times
    let busy = rng.below(2) == 0;
    let wait = |rng: &mut Xorshift, n: u32| u8::from(busy && rng.below(n) == 0);
    set(
        a,
        b,
        ffx_battle::state::bridge::COMMAND_CUE_COUNT,
        i8::try_from(wait(rng, 4)).unwrap(),
    );
    set(a, b, ffx_battle::minor::bridge::NUMBERS_SHOWN, wait(rng, 4));
    set(a, b, ffx_battle::atel::bridge::CAMERA_STATE, wait(rng, 4));
    set(a, b, ffx_battle::flow::bridge::LIMIT_LEARNT, wait(rng, 4));
    set(
        a,
        b,
        ffx_battle::state::bridge::STONE_EFFECT,
        [i8::try_from(wait(rng, 4)).unwrap(), 0],
    );
    for (r, k) in [(prec, 3), (prec, 4), (srec, 3), (srec, 4)] {
        let v = if wait(rng, 4) != 0 {
            [1u32, 3][rng.below(2) as usize]
        } else {
            0
        };
        poke(a, b, r + EXCHANGE_REC * k + 4, &v.to_le_bytes());
    }
    for k in 0..15 {
        let r = rng.next();
        end_results(a, b, rng, rec, k, &[1, 2, r]);
    }
    end_results(a, b, rng, rec, 16, &[2, 2, 1]);
    // the end status checks (AtelIsBattleEndStatusClear9/8), the status
    // window's fade and the vfx counter: 0, 1 or 2 alike
    for k in [1, 2, 4, 5] {
        let r = rng.below(3);
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    // the end: forced at times; the party's and monsters' fighting fields
    set(
        a,
        b,
        ffx_battle::menu::bridge::END_FORCED,
        [0, 1, 2, 3, 4, 5, 6][rng.below(7) as usize],
    );
    set(a, b, ffx_battle::actor::bridge::MON_TOP, w.actor(0x14));
    // nobody moving half the time
    let still = rng.below(2) == 0;
    for &q in &all {
        let p = w.actor(q);
        for o in [0xdd2, 0xdd3, 0xdc8] {
            poke(a, b, p + o, &[u8::from(rng.below(4) != 0)]);
        }
        for o in [0xdcc, 0xdce] {
            poke(a, b, p + o, &[flag(rng, 4)]);
        }
        poke(
            a,
            b,
            p + 0x5d0,
            &[0, 0, 100, rng.next()][rng.below(4) as usize].to_le_bytes(),
        );
        // what the closing waits on
        for o in [
            0x15, 0x508, 0x42e, 0x415, 0x41c, 0x41d, 0x432, 0x436, 0x43a, 0x43b, 0x608,
        ] {
            let moving = matches!(o, 0x508 | 0x42e | 0x415 | 0x41c);
            let v = if matches!(o, 0x15 | 0x43b) {
                flag(rng, 2)
            } else if matches!(o, 0x43a | 0x608) {
                flag(rng, 4)
            } else if moving && still {
                0
            } else {
                flag(rng, 30)
            };
            poke(a, b, p + o, &[v]);
        }
        poke(a, b, p + 0x702, &[[1, 0, 2][rng.below(3) as usize]]);
        poke(a, b, p + 0xdee, &[rng.byte()]);
        poke(a, b, p + 0xf8c, &[0; 4]);
    }
    set(
        a,
        b,
        ffx_battle::target::bridge::SUMMONER,
        [0, i8::try_from(1 + rng.below(6)).unwrap()][rng.below(2) as usize],
    );
    set(a, b, be::NO_SUMMON_RETURN, flag(rng, 4));
    set(
        a,
        b,
        be::END_KIND,
        [2, 2, 2, 1, 3, 4, 5, 0][rng.below(8) as usize],
    );
    for g in [be::END_FADE, be::END_FADE_NEXT, be::END_FADE_FRAMES] {
        set(a, b, g, rng.byte());
    }
    set(
        a,
        b,
        be::END_FADE_OUT,
        [0, 0, 0x3c, rng.byte()][rng.below(4) as usize],
    );
    set(
        a,
        b,
        be::END_EVENT_EFFECT,
        [0, 1, 1, -1, rng.byte().cast_signed()][rng.below(5) as usize],
    );
    for g in [
        be::END_VICTORY,
        be::END_GAME_OVER,
        be::END_WAIT,
        be::END_HOLD,
    ] {
        set(
            a,
            b,
            g,
            flag(rng, 2) * [1, rng.byte()][rng.below(2) as usize],
        );
    }
    set(a, b, be::END_HOLD, flag(rng, 6));
    set(
        a,
        b,
        be::END_MUSIC,
        u16::try_from(rng.below(0x10000)).unwrap(),
    );
    set(
        a,
        b,
        be::END_EFFECT,
        u16::try_from([0x29f, rng.below(0x400), rng.below(0x10000)][rng.below(3) as usize])
            .unwrap(),
    );
    set(
        a,
        b,
        be::PAD_OLD,
        [0, 0, u32::MAX, rng.next()][rng.below(4) as usize],
    );
    set(
        a,
        b,
        be::PAD,
        [0, 0xf0, 0x10, rng.byte()][rng.below(4) as usize],
    );
    set(
        a,
        b,
        ffx_battle::action::bridge::NO_MAGIC_EFFECT,
        flag(rng, 4),
    );
    set(a, b, ffx_battle::parse::bridge::NO_MAGIC_FILE, flag(rng, 4));
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_STAT,
        [0i8, 0, 3, 5, 5, 1][rng.below(6) as usize],
    );
    set(a, b, ffx_battle::state::bridge::EVENT_EFFECT_A, out + 0xa0);
    set(a, b, ffx_battle::state::bridge::INPUT_CUE_COUNT, 0);
    sep_3d_list(a, b, rng);
    let was = threats(b, w, &all);
    twin("MsBattleProcess_Status", a, b, 0x0079_17d0, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            be::ms_battle_process_status(m, &h, &mut s);
        });
        0
    });
    threatened(a, b, w, &all, &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsBattleProcess_Status", a, b);
}

/// The battle loop's sound and event-effect callees of sound and main
/// (`ffx_battle::battle_end`), logging stubs in
/// `MsBattleProcess_DebugStatus_00792f20` (never called here); the first
/// is replaced by one answering from its argument.
const FRAME_CALLEES: [(u32, u8); 8] = {
    use ffx_battle::battle_end::bridge as be;
    [
        (be::SND_IS_SEP_PLAY, 1),
        (be::SND_SEP_STOP, 1),
        (be::SND_SEP_VOLUME_DIRECT, 2),
        (be::SND_SEP_PAN_DIRECT, 2),
        (be::SND_SYNC_WAVE_TRANS, 1),
        (be::SND_STANDBY_SEP_WAVE, 0),
        (be::SND_STANDBY_BATTLE_WAVE, 2),
        (be::SND_SEP_TRANS_SEP_ONLY, 2),
    ]
};
const FRAME_STUBS: u32 = 0x0079_2f20;

/// `op_3d_vol_pan_get` with its output through [`SCRATCH`] (the original's
/// is a frame local).
fn vol_pan(m: &mut Oracle, v: u32, vol: u32) -> (u32, u32) {
    let s = SCRATCH.with(std::cell::Cell::get);
    let pan = call(
        m,
        ffx_battle::after_damage::bridge::OP_3D_VOL_PAN_GET,
        &[v, vol, s],
    )
    .eax;
    let out = u32::from_le_bytes(m.read(s, 4).unwrap().try_into().unwrap());
    m.write(s, &[0; 4]).unwrap();
    (pan, out)
}

/// The battle loop's per-frame sound transfers, 3D sounds and event
/// effect end (`ffx_battle::battle_end`).
fn battle_frame(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    frec: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::battle_end::bridge as be;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    for k in 0..8 {
        let r = [0, 0, 1, rng.next()][rng.below(4) as usize];
        poke(a, b, frec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    // an event effect done
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_STAT,
        [6i8, 6, 0, 5][rng.below(4) as usize],
    );
    let f = b.rebase(END_STUBS + 0x40 * 15);
    poke(a, b, out + 0xc4, &f.to_le_bytes());
    set(
        a,
        b,
        ffx_battle::state::bridge::EVENT_EFFECT_A,
        [out + 0xc0, out + 0xc0, 0][rng.below(3) as usize],
    );
    set(a, b, be::KEEP_FPS, rng.byte());
    twin("MsEventEffectProcess", a, b, 0x0078_8c00, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            be::ms_event_effect_process(m, &mut s);
        });
        0
    });
    same("MsEventEffectProcess", a, b);
    // the sound transfers: records of every type, actor and state
    let q = be::SEP_TRANS_QUEUE.ptr(b).addr;
    let mut recs = vec![0u8; 16 * 12];
    for r in recs.chunks_mut(12) {
        r[0..4].copy_from_slice(&(out + rng.below(0x80)).to_le_bytes());
        r[4..8].copy_from_slice(&rng.next().to_le_bytes());
        r[8] = [0, 1, 1, 2][rng.below(4) as usize];
        r[9] = [u8::try_from(rng.below(ACTORS)).unwrap(), 0xff][rng.below(2) as usize];
        r[10] = [0, 1, 1, 2][rng.below(4) as usize];
    }
    poke(a, b, q, &recs);
    set(
        a,
        b,
        ffx_battle::state::bridge::SEP_TRANS,
        [1, 2, 1, 0, u32::MAX][rng.below(5) as usize],
    );
    set(
        a,
        b,
        ffx_battle::minor::bridge::SEP_TRANS_STATE,
        [rng.below(16), rng.next(), rng.next()],
    );
    set(
        a,
        b,
        be::SEP_TRANS_SCENE,
        [1, 1, 2, 0][rng.below(4) as usize],
    );
    set(
        a,
        b,
        ffx_battle::misc::bridge::BTL_READ3,
        [0, rng.next()][rng.below(2) as usize],
    );
    for i in 0..ACTORS {
        poke(
            a,
            b,
            w.actor(i) + 0xf2d,
            &[[1, 1, 2, 0][rng.below(4) as usize]],
        );
        poke(a, b, w.actor(i) + 0xf2c, &[flag(rng, 3)]);
    }
    poke(
        a,
        b,
        frec + EXCHANGE_REC * 4 + 4,
        &u32::from(flag(rng, 3)).to_le_bytes(),
    );
    twin("MsSEPtransProcess", a, b, 0x007a_0540, &[], ALL, |m| {
        dm(m, |m| {
            be::ms_sep_trans_process(m, &mut call).cast_unsigned()
        })
    });
    same("MsSEPtransProcess", a, b);
    // 3D sounds: delays, fades, actors (every actor placed)
    if !placed(a, b, w, out) {
        return;
    }
    let nodes = b.rebase(ffx_battle::after_damage::bridge::SEP_3D_NODES);
    let mut area = vec![0u8; 0x200];
    let n = rng.below(6);
    for k in 0..n as usize {
        let at = nodes + 0x20 * u32::try_from(k).unwrap();
        let e = &mut area[0x20 * k..0x20 * k + 0x20];
        e[0] = 1;
        e[1] = [1, 2, 3, 4, 0, 5][rng.below(6) as usize];
        e[2] = u8::try_from(rng.below(ACTORS)).unwrap();
        e[3] = [0, 0, 0, 1, 2][rng.below(5) as usize];
        let total = [0i16, 0, 5, -3, 1, 0x7fff][rng.below(6) as usize];
        let left = [1i16, 2, 5, 0, -1, 3][rng.below(6) as usize];
        e[4..6].copy_from_slice(&total.to_le_bytes());
        e[6..8].copy_from_slice(&left.to_le_bytes());
        e[8..12].copy_from_slice(&(0x8000_0000 | rng.below(4)).to_le_bytes());
        // the last links to itself at times (the walk cuts it)
        let next = if k + 1 < n as usize {
            at + 0x20
        } else if rng.below(4) == 0 {
            at
        } else {
            0
        };
        e[0xc..0x10].copy_from_slice(&next.to_le_bytes());
    }
    poke(a, b, nodes, &area);
    let head = if n == 0 { 0 } else { nodes };
    poke(
        a,
        b,
        ffx_battle::minor::bridge::SEP_3D.ptr(b).addr,
        &head.to_le_bytes(),
    );
    twin("MsSEP3Dprocess", a, b, 0x007a_0930, &[], ALL, |m| {
        be::ms_sep_3d_process(m, &stand_in, &mut game_call_o, &mut vol_pan)
    });
    local_words(a, b);
    same("MsSEP3Dprocess", a, b);
}

/// [`sep_3d_list`] with its nodes marked in use (`+0`), as the game's
/// are: `MsSEP3Dregist` takes a free node from the pool, and one still
/// linked would close the list into a loop (`MsSEP3DFind` then spins).
fn sep_3d_list_used(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    sep_3d_list(a, b, rng);
    let mut node = u32::from_le_bytes(
        b.read(ffx_battle::minor::bridge::SEP_3D.ptr(b).addr, 4)
            .unwrap()
            .try_into()
            .unwrap(),
    );
    while node != 0 {
        poke(a, b, node, &[1]);
        node = u32::from_le_bytes(b.read(node + 0xc, 4).unwrap().try_into().unwrap());
    }
}

/// `MsEffectStart`'s callees of other modules (`ffx_battle::effect`):
/// logging stubs in `0x007ce450` (unreachable code), installed around
/// [`effect_start`] and removed after (`user_malloc` and the filters are
/// other tests' originals).
const START_CALLEES: [(u32, u8); 14] = {
    use ffx_battle::effect::bridge as e;
    [
        (e::TO_BTL_INIT_TIDUS_LIMIT_WINDOW, 0),
        (e::TO_BTL_OPEN_TIDUS_LIMIT_WINDOW, 1),
        (e::TO_BTL_INIT_AURON_LIMIT_WINDOW, 0),
        (e::TO_BTL_OPEN_AURON_LIMIT_WINDOW, 1),
        (e::TO_BTL_INIT_LULU_LIMIT_WINDOW, 0),
        (e::TO_BTL_OPEN_LULU_LIMIT_WINDOW, 1),
        (e::SG_FILTER_SUB, 4),
        (e::SG_FILTER_RESET, 0),
        (e::SG_EFFECT_ROUTINE_ENABLE, 1),
        (e::ATEL_INIT_INTERNAL_ACTOR_WORK, 1),
        (e::ATEL_GET_ACTOR_BUFFER_SIZE, 1),
        (e::ATEL_GET_MY_BUFFER_SIZE, 1),
        (e::USER_MALLOC, 1),
        (ffx_battle::save_param::bridge::CH_SET_TRANSPARENT, 3),
    ]
};
const START_STUBS: u32 = 0x007c_e450;
/// `AtelBattleSetUp`'s recording stub (eight arguments, two slots) and an
/// effect's function, in `0x007ceb30` (unreachable code).
const START_STUBS2: u32 = 0x007c_eb30;

/// Installs `MsEffectStart`'s stubs (records from `rec`, [`EXCHANGE_REC`]
/// each; `AtelBattleSetUp`'s at `rec + 14 *`, the effect function's at
/// `rec + 15 *`), returning the entry bytes they replaced.
fn start_stubs(a: &mut Oracle, b: &mut Oracle, rec: u32) -> Vec<(u32, Vec<u8>)> {
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let fs = START_CALLEES
            .iter()
            .map(|&(f, _)| f)
            .chain([ffx_battle::effect::bridge::ATEL_BATTLE_SET_UP]);
        for f in fs {
            let at = o.rebase(f);
            saved.push((f, o.read(at, 5).unwrap()));
        }
        for (k, (f, n)) in (0u32..).zip(START_CALLEES) {
            logging_stub(o, f, START_STUBS + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        recording_stub(
            o,
            ffx_battle::effect::bridge::ATEL_BATTLE_SET_UP,
            START_STUBS2,
            rec + EXCHANGE_REC * 14,
            8,
            0x5e7,
        );
        logging_stub(o, 0, START_STUBS2 + 0x80, rec + EXCHANGE_REC * 15, 1, 0);
    }
    saved.truncate(saved.len() / 2);
    saved
}

fn unstub(a: &mut Oracle, b: &mut Oracle, saved: &[(u32, Vec<u8>)]) {
    for o in [&mut *a, &mut *b] {
        for (f, bytes) in saved {
            let at = o.rebase(*f);
            o.write(at, bytes).unwrap();
        }
    }
}

/// A command's effect from start to end (`ffx_battle::effect`): the motion
/// lookups, the camera script, the reflect step, the overdrive start and
/// the whole per-frame `MsEffectStart`, on actors in every state. Its own
/// random stream (`rng`), so the other tests' stay as they were.
#[allow(clippy::too_many_arguments)]
fn effect_start(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    rec: u32,
    recs: [u32; 3],
    cam: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::effect::bridge as e;
    let [prec, srec, erec] = recs;
    let saved = start_stubs(a, b, rec);
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    macro_rules! pick {
        ($rng:expr, $v:expr $(,)?) => {{
            let v: &[u32] = $v;
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let stub_ret = |a: &mut Oracle, b: &mut Oracle, r: u32, k: u32, v: u32| {
        poke(a, b, r + EXCHANGE_REC * k + 4, &v.to_le_bytes());
    };
    // the magic ID (MagicFile_GetMagicId: the process stubs' 11th) and the
    // models whose motion it stops
    stub_ret(a, b, prec, 11, pick!(rng, &[0x1b2, 0x262, 0, 0x1b3]));
    let n = [0i32, 0, 1, 2, 3, -1][rng.below(6) as usize];
    let stop = e::MOTION_STOP_CHRS.ptr(b).addr;
    let mut list = n.to_le_bytes().to_vec();
    for _ in 0..3 {
        let chr = u32::from_le_bytes(
            b.read(w.actor(rng.below(ACTORS)), 4)
                .unwrap()
                .try_into()
                .unwrap(),
        );
        list.extend(pick!(rng, &[chr, chr, rng.next()]).to_le_bytes());
    }
    poke(a, b, stop, &list);
    // motions: weapon kinds 0-6, the motion one of the kinds' at times
    for i in 0..ACTORS {
        let p = w.actor(i);
        let wk = [rng.below(7), rng.below(7)];
        poke(
            a,
            b,
            p + 0x408,
            &[u8::try_from(wk[0]).unwrap(), u8::try_from(wk[1]).unwrap()],
        );
        let m = pick!(rng, &[0x5a, 0xb2, 0x5b, 0x58, 0x29, rng.below(0x100), 0xc9]);
        poke(a, b, p + 0x3f0, &[u8::try_from(m).unwrap(), rng.byte()]);
        for o in [
            0x508, 0x436, 0x435, 0x432, 0x429, 0x413, 0xdfd, 0xe0d, 0xe14, 0xdfc,
        ] {
            poke(a, b, p + o, &[flag(rng, 3)]);
        }
        poke(
            a,
            b,
            p + 0xea4,
            &pick!(rng, &[0, 0, rng.next()]).to_le_bytes(),
        );
        // the effect's sound: one of the 3D list's at times (sep_3d_list)
        let se = pick!(rng, &[0, 1, 2, 3, rng.below(0x10000)]);
        poke(a, b, p + 0xf76, &u16::try_from(se).unwrap().to_le_bytes());
    }
    for _ in 0..4 {
        let i = rng.below(ACTORS);
        let kind = pick!(rng, &[1, 2, 3, 4, 5, 6, 0xc, 0xd, 0x82, 0, 7, 0x100]);
        let p = w.actor(i);
        twin("ms_motion_trans", a, b, 0x007a_8c80, &[p, kind], ALL, |m| {
            e::ms_motion_trans(m, &mut game_call_o, p, kind)
        });
        same("ms_motion_trans", a, b);
        twin("MsEffectMotionDone", a, b, 0x0078_7650, &[i, p], ALL, |m| {
            e::ms_effect_motion_done(m, &mut game_call_o, p)
        });
        same("MsEffectMotionDone", a, b);
        twin("MsStatusEffectReflect", a, b, 0x0079_f160, &[i], ALL, |m| {
            e::ms_status_effect_reflect(m, &mut game_call_o, i)
        });
        same("MsStatusEffectReflect", a, b);
    }
    // a camera script file (script and data offsets), manager 2 with room
    // or full, small buffers (past 0x800 MsBattleError, a stub here)
    poke(a, b, cam + 4, &[0x10, 0, 0, 0, 0x20, 0, 0, 0]);
    let managers = ffx_battle::minor::bridge::ATEL_MANAGERS.ptr(b).addr;
    for kind in 0..3 {
        let m = managers + 0x54 * kind;
        let count = pick!(rng, &[0, 1, 3, 7, 8]);
        poke(a, b, m, &count.to_le_bytes());
        let sizes = [rng.below(0x100), pick!(rng, &[0, 0x1f0, 0x7f0])];
        poke(a, b, m + 8, &sizes[0].to_le_bytes());
        poke(a, b, m + 0x10, &sizes[1].to_le_bytes());
        fill(a, b, rng, m + 0x14, 0x20);
    }
    stub_ret(a, b, rec, 10, pick!(rng, &[0x10, 0x33, 0, 0xffff_ffe1]));
    stub_ret(a, b, rec, 11, pick!(rng, &[0x10, 0x41, 0]));
    stub_ret(a, b, rec, 12, rng.next());
    let kind = pick!(rng, &[0, 1, 2, 2, 3]);
    twin("MsAtelSetUp", a, b, 0x0079_8390, &[kind], ALL, |m| {
        e::ms_atel_set_up(m, &mut game_call_o, kind)
    });
    same("MsAtelSetUp", a, b);
    let n = rng.below(4);
    twin(
        "MsAtelActorWorkInit",
        a,
        b,
        0x0079_7490,
        &[2, 0, n],
        ALL,
        |m| e::ms_atel_actor_work_init(m, &mut game_call_o, 2, 0, n),
    );
    same("MsAtelActorWorkInit", a, b);
    let m2 = managers + 0x54 * 2;
    poke(a, b, m2 + 8, &rng.below(0x100).to_le_bytes());
    poke(a, b, m2 + 0x10, &rng.below(0x100).to_le_bytes());
    let file = pick!(rng, &[cam, cam, 0]);
    twin("MsAtelCamRegist", a, b, 0x0079_7550, &[file], ALL, |m| {
        dm(m, |m| {
            s!(s);
            e::ms_atel_cam_regist(m, &mut s, file)
        })
    });
    same("MsAtelCamRegist", a, b);
    // the effects: none, or one with or without a camera script; the
    // effect's function a stub
    let eff = out + 0xe0;
    let f = b.rebase(START_STUBS2 + 0x80);
    poke(a, b, eff, &f.to_le_bytes());
    poke(a, b, eff + 0x1c, &pick!(rng, &[cam, 0]).to_le_bytes());
    let effects = [0, eff, eff];
    // commands an overdrive at times (+0x58), the kernel's or the fakes
    let reset = |a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift| {
        let m2 = managers + 0x54 * 2;
        poke(a, b, m2, &pick!(rng, &[0, 2]).to_le_bytes());
        poke(a, b, m2 + 8, &rng.below(0x100).to_le_bytes());
        poke(a, b, m2 + 0x10, &rng.below(0x100).to_le_bytes());
    };
    for i in 0..ACTORS {
        let p = w.actor(i);
        let id = 0x3000 + rng.below(0x40);
        poke(a, b, p + 0xf5c, &u16::try_from(id).unwrap().to_le_bytes());
        let (c, _) = ffx_battle::command::bridge::get_command(b, u32::MAX, 0, id);
        if c != 0 && readable(a, c + 0x58, 1) {
            let v = pick!(rng, &[0x20, 0x25, 0x30, 0x40, 0, rng.below(0x100)]);
            poke(a, b, c + 0x58, &[u8::try_from(v).unwrap()]);
        }
        poke(a, b, p + 0xf8c, &pick!(rng, &effects).to_le_bytes());
        // a cue entry: its own (0xff) mostly, the hit's targets
        let de5 = if rng.below(4) == 0 {
            rng.below(8)
        } else {
            0xff
        };
        poke(a, b, p + 0xde5, &[u8::try_from(de5).unwrap()]);
        let cue = ffx_battle::actor::bridge::ms_get_chr_command(b, i);
        let hit = rng.below(3);
        poke(a, b, cue + 2, &[u8::try_from(hit).unwrap()]);
        poke(
            a,
            b,
            cue + 5,
            &[u8::try_from(pick!(rng, &[0, 0, 1, 1, 2])).unwrap()],
        );
        let targets = pick!(
            rng,
            &[0, 1 << rng.below(ACTORS), rng.next() & 0x7fff_ffff, 7],
        );
        poke(a, b, cue + 0x14 + hit * 0x10, &targets.to_le_bytes());
    }
    set(
        a,
        b,
        ffx_battle::parse::bridge::EFFECT_READ_STAT,
        pick!(rng, &[0, 0, cam]),
    );
    for _ in 0..2 {
        reset(a, b, rng);
        let i = rng.below(ACTORS);
        let p = w.actor(i);
        twin("MsEffectCamRegist", a, b, 0x0078_8ef0, &[p], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_cam_regist(m, &mut s, p)
            })
        });
        same("MsEffectCamRegist", a, b);
        twin("MsEffectCameraEnd2", a, b, 0x0078_8f30, &[i, p], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_camera_end2(m, &mut s, i, p)
            })
        });
        same("MsEffectCameraEnd2", a, b);
        let i = pick!(rng, &[0, 2, 5, rng.below(ACTORS), 0x100 | rng.below(6)]);
        let p = w.actor(i & 0xff);
        twin("MsEffectLimitCheck", a, b, 0x007a_fc90, &[i, p], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_limit_check(m, &mut s, i, p)
            })
        });
        same("MsEffectLimitCheck", a, b);
        twin("MsEffectLimitStart", a, b, 0x007a_fd30, &[i], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_limit_start(m, &mut s, i)
            })
        });
        same("MsEffectLimitStart", a, b);
    }
    if !placed(a, b, w, out) {
        unstub(a, b, &saved);
        return;
    }
    let all: Vec<u32> = (0..ACTORS).collect();
    for _ in 0..2 {
        sep_3d_list_used(a, b, rng);
        let i = rng.below(ACTORS);
        let p = w.actor(i);
        let was = threats(b, w, &all);
        twin("MsEffectReflect", a, b, 0x0078_9030, &[i, p], 0, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_reflect(m, &h, &mut s, i, p);
            });
            0
        });
        threatened(a, b, w, &all, &was);
        local_words(a, b);
        same("MsEffectReflect", a, b);
        sep_3d_list_used(a, b, rng);
        let (x, motion, fl) = (rng.below(3), pick!(rng, &[1, 1, 0, 2]), rng.byte());
        let args = [p, x, motion, u32::from(fl)];
        let was = threats(b, w, &all);
        twin("MsEffectEnd", a, b, 0x0078_8f90, &args, ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_end(m, &h, &mut s, p, x, motion, u32::from(fl)).cast_unsigned()
            })
        });
        threatened(a, b, w, &all, &was);
        local_words(a, b);
        unplaced_rows(a, b, out);
        same("MsEffectEnd", a, b);
        sep_3d_list_used(a, b, rng);
        let i = rng.below(ACTORS);
        poke(
            a,
            b,
            w.actor(i) + 0xdfb,
            &[pick!(rng, &[4, 4, 3]).to_le_bytes()[0]],
        );
        set(
            a,
            b,
            ffx_battle::state::bridge::EVENT_EFFECT_STAT,
            [4i8, 3, 0][rng.below(3) as usize],
        );
        let id = pick!(
            rng,
            &[
                0x200 | i,
                0x200 | i,
                0x8000,
                0x8000 | rng.below(0x100),
                rng.next(),
            ],
        );
        let x = rng.below(3);
        let was = threats(b, w, &all);
        twin("MsEffectReady", a, b, 0x0078_8200, &[id, x], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_ready(m, &h, &mut s, id, x)
            })
        });
        threatened(a, b, w, &all, &was);
        local_words(a, b);
        unplaced_rows(a, b, out);
        same("MsEffectReady", a, b);
        sep_3d_list_used(a, b, rng);
        let i = rng.below(ACTORS);
        let p = w.actor(i);
        let was = threats(b, w, &all);
        twin("MsEffectBackToIdle", a, b, 0x0078_7d70, &[i, p], ALL, |m| {
            dm(m, |m| {
                s!(s);
                e::ms_effect_back_to_idle(m, &h, &mut s, i, p).cast_unsigned()
            })
        });
        threatened(a, b, w, &all, &was);
        local_words(a, b);
        unplaced_rows(a, b, out);
        same("MsEffectBackToIdle", a, b);
    }
    // the whole step: every actor in a state, the waits all clear half
    // the time
    for i in 0..ACTORS {
        let st = pick!(
            rng,
            &[
                0, 1, 1, 2, 3, 3, 3, 4, 5, 6, 7, 8, 9, 9, 10, 11, 11, 12, 13, 14, 15, 16, 0xff,
            ],
        );
        // (a weapon swap, 9, for characters and aeons only: a monster's
        // MsSetSaveParam divides by its ply_rom record's zeros)
        let st = if st == 9 && i >= 0x12 { 10 } else { st };
        let p = w.actor(i);
        poke(a, b, p + 0xdfb, &[u8::try_from(st).unwrap()]);
        // (not 8 for 0x10 and 0x11: MsMagicEffectEnd would recurse without
        // end, battle.md)
        let kind = pick!(rng, &[0, 1, 2, 3, 4, 8, 8]);
        let kind = if (0x10..0x12).contains(&i) && kind == 8 {
            1
        } else {
            kind
        };
        poke(a, b, p + 0xdea, &[u8::try_from(kind).unwrap()]);
        poke(
            a,
            b,
            p + 0xf65,
            &[u8::try_from(pick!(rng, &[0, 0, 1, 3])).unwrap()],
        );
        poke(
            a,
            b,
            p + 0xe00,
            &[u8::try_from(pick!(rng, &[0xff, 0x10, 0x30])).unwrap()],
        );
        poke(
            a,
            b,
            p + 0xe01,
            &[u8::try_from(pick!(rng, &[0xff, 0x11])).unwrap()],
        );
        if rng.below(3) != 0 {
            for o in [0x508, 0x436, 0x435, 0xdfd] {
                poke(a, b, p + o, &[0]);
            }
        }
        poke(
            a,
            b,
            p + 0xdc0,
            &pick!(rng, &[1 << i, rng.next() & 0x7fff_ffff]).to_le_bytes(),
        );
    }
    let busy = rng.below(2) == 0;
    let wait = |rng: &mut Xorshift, n: u32| u8::from(busy && rng.below(n) == 0);
    set(a, b, ffx_battle::process::bridge::RESET_WAIT2, wait(rng, 4));
    set(a, b, e::DEBUG_EFFECT_HOLD, wait(rng, 4));
    set(a, b, ffx_battle::atel::bridge::CAMERA_STATE, flag(rng, 2));
    set(a, b, e::CAMERA_ONCE, [0u8, 1, 2][rng.below(3) as usize]);
    set(
        a,
        b,
        ffx_battle::damage_end::bridge::DEBUG_CAMERA_SLEEP,
        [rng.byte() & 1, rng.byte()],
    );
    set(
        a,
        b,
        e::LIMIT_FADE,
        [0i8, 3, 6, 7, -1][rng.below(5) as usize],
    );
    stub_ret(a, b, prec, 3, u32::from(wait(rng, 4)));
    stub_ret(a, b, srec, 3, u32::from(wait(rng, 4)));
    stub_ret(a, b, srec, 4, u32::from(wait(rng, 4)));
    // the limit window up (+0xd24) or not
    for i in 0..ACTORS {
        poke(a, b, w.actor(i) + 0xd24, &[flag(rng, 2), 0]);
    }
    // an effect read (state 14) on a magic in the effect table
    set(
        a,
        b,
        ffx_battle::parse::bridge::LAST_MAGIC,
        rng.below(0x400),
    );
    let _ = erec;
    reset(a, b, rng);
    sep_3d_list_used(a, b, rng);
    let was = threats(b, w, &all);
    twin("MsEffectStart", a, b, 0x0078_8480, &[], 0, |m| {
        dm(m, |m| {
            s!(s);
            e::ms_effect_start(m, &h, &mut s);
        });
        0
    });
    threatened(a, b, w, &all, &was);
    local_words(a, b);
    unplaced_rows(a, b, out);
    same("MsEffectStart", a, b);
    unstub(a, b, &saved);
}

/// Kernel tables the saved parameters read (`ply_rom`, the auto-abilities,
/// the aeons' floors): the snapshot's, else fakes with no zero divisor.
fn save_tables(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    use battle_common::world::fake_records;
    let rom = ffx_battle::menu::bridge::PLY_ROM;
    if !readable(a, rom.get(b), 0x20) {
        let t = fake_records(a, b, rng, 0x14, 0x30);
        for k in 0..0x14 {
            for o in (0x1c..0x2a).step_by(2) {
                poke(a, b, t + 0x20 + 0x30 * k + o, &[1 + rng.byte() % 9]);
            }
        }
        set(a, b, rom, t);
    }
    let ab = ffx_kernel::bridge::A_ABILITY;
    if !readable(a, ab.get(b), 0x20) {
        let t = fake_records(a, b, rng, 0x40, 0x68);
        set(a, b, ab, t);
    }
    let grow = ffx_battle::save_param::bridge::SUM_ASSURE;
    if !readable(a, grow.get(b), 0x20) {
        let t = fake_records(a, b, rng, 0x80, 0xc);
        set(a, b, grow, t);
    }
}

/// The save areas the saved parameters change (the characters, the sphere
/// grid totals, the equipment, the battle count, the damage rates), kept
/// to be put back after.
fn save_areas(b: &mut Oracle) -> Vec<(u32, usize)> {
    use ffx_battle::save::bridge as sv;
    vec![
        (sv::PLY_SAVE.ptr(b).addr, 0x94 * 0x12),
        (ffx_battle::minor::bridge::ABILITY_MAP.ptr(b).addr, 0x1c * 8),
        (
            sv::EQUIPMENT.ptr(b).addr,
            (ffx_battle::save::EQUIPMENT_SLOTS * ffx_battle::save::EQUIPMENT_SIZE) as usize,
        ),
        (sv::BATTLE_COUNT.ptr(b).addr, 4),
        (
            ffx_battle::minor::bridge::DAMAGE_RATES.ptr(b).addr,
            4 * 0x12,
        ),
        (sv::OUT_PARTY.ptr(b).addr, 0x11),
    ]
}

/// The characters' saved parameters (`ffx_battle::save_param`): sphere
/// grid, aeon and weapon stats, the save written back at a battle's end,
/// a weapon swapped. The save is randomized for it and put back after.
fn save_params(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    out: u32,
    rec: u32,
    xrec: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::save_param::bridge as sp;
    let saved = start_stubs(a, b, rec);
    let areas = save_areas(b);
    let keep: Vec<Vec<u8>> = areas
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! pick {
        ($rng:expr, $v:expr $(,)?) => {{
            let v: &[u32] = $v;
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    save_tables(a, b, rng);
    // the characters: weapons the equipment has (or none), small or
    // limit-breaking values, joined or not
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    for i in 0..0x12 {
        let p = ply + 0x94 * i;
        fill(a, b, rng, p, 0x94);
        for o in [4, 8, 0x1c, 0x20] {
            let v = pick!(rng, &[rng.below(3000), rng.below(100), rng.next(), 0]);
            poke(a, b, p + o, &v.to_le_bytes());
        }
        for o in [0x2d, 0x2e] {
            let v = pick!(rng, &[rng.below(200), rng.below(8), 0xff]);
            poke(a, b, p + o, &[u8::try_from(v).unwrap()]);
        }
        poke(a, b, p + 0x2c, &[rng.byte() & 0x11]);
    }
    let map = ffx_battle::minor::bridge::ABILITY_MAP.ptr(b).addr;
    for k in 0..8 {
        let mut v = vec![0u8; 0x1c];
        for x in &mut v {
            *x = rng.byte();
        }
        v[0..4].copy_from_slice(&rng.below(200).to_le_bytes());
        v[4..8].copy_from_slice(&rng.below(200).to_le_bytes());
        poke(a, b, map + 0x1c * k, &v);
    }
    let count = pick!(rng, &[0, 29, 30, 300, 599, 900, rng.next()]);
    set(a, b, ffx_battle::save::bridge::BATTLE_COUNT, count);
    set(a, b, ffx_battle::minor::bridge::ENCOUNT_ON, rng.byte());
    // the aeons' records: no zero divisor
    let rom = ffx_battle::menu::bridge::PLY_ROM.get(b);
    for i in 8..0x12 {
        let (r, _) = ffx_kernel::bridge::excel_data(b, i, rom);
        for o in (0x1c..0x2a).step_by(2) {
            if b.read(r + o, 1).unwrap()[0] == 0 {
                poke(a, b, r + o, &[1]);
            }
        }
    }
    for _ in 0..3 {
        let i = pick!(rng, &[rng.below(8), rng.below(0x12), 8, 9, u32::MAX]);
        twin("MsGetAbilityMap", a, b, 0x0079_87f0, &[i], ALL, |m| {
            sp::ms_get_ability_map(m, i)
        });
        let i = rng.below(0x12);
        twin(
            "MsGetChrAbilityMap",
            a,
            b,
            0x0078_5b60,
            &[i, out],
            ALL,
            |m| sp::ms_get_chr_ability_map(m, i, out),
        );
        same("MsGetChrAbilityMap", a, b);
        let i = pick!(rng, &[5, rng.below(8), 8 + rng.below(10), rng.below(0x12)]);
        twin("MsSetSaveParam", a, b, 0x0078_60f0, &[i], ALL, |m| {
            sp::ms_set_save_param(m, i)
        });
        same("MsSetSaveParam", a, b);
        let (i, k) = (
            rng.below(0x12) | (u32::from(rng.byte()) & 0x80) << 8,
            rng.below(2),
        );
        let id = pick!(rng, &[rng.below(200), 0xff, 0x5000 | rng.below(200)]);
        twin(
            "MsEquipSaveWeapon",
            a,
            b,
            0x007a_b970,
            &[i, k, id],
            0,
            |m| {
                sp::ms_equip_save_weapon(m, i, k, id);
                0
            },
        );
        same("MsEquipSaveWeapon", a, b);
    }
    twin("MsSetSaveParamAll", a, b, 0x0078_6900, &[], ALL, |m| {
        sp::ms_set_save_param_all(m)
    });
    same("MsSetSaveParamAll", a, b);
    // the actors' weapons as the save's (MsEquipSaveWeapon reads the save's)
    for i in 0..ACTORS {
        let p = w.actor(i);
        for k in 0..2 {
            let v = pick!(rng, &[rng.below(200), 0xff]);
            poke(a, b, p + 0x592 + k, &[u8::try_from(v).unwrap()]);
        }
        let hp = pick!(rng, &[0, rng.below(5000), u32::MAX]);
        poke(a, b, p + 0x5d0, &hp.to_le_bytes());
        poke(
            a,
            b,
            p + 0x6d8,
            &[u8::try_from(pick!(rng, &[0, 0, 1, 2])).unwrap()],
        );
    }
    twin("MsSetSaveData", a, b, 0x0078_5fc0, &[], ALL, |m| {
        sp::ms_set_save_data(m)
    });
    same("MsSetSaveData", a, b);
    // a weapon swapped: the same model or another (characters and aeons:
    // a monster's MsSetSaveParam divides by its ply_rom record's zeros)
    let i = rng.below(0x12);
    let k = rng.below(2);
    let p = w.actor(i);
    poke(
        a,
        b,
        p + 0xdfb,
        &[u8::try_from(pick!(rng, &[9, 9, 8])).unwrap()],
    );
    let id = if rng.below(2) == 0 {
        u32::from(b.read(p + 0x592 + k, 1).unwrap()[0])
    } else {
        pick!(rng, &[rng.below(200), 0xff])
    };
    for o in [0x48, 0x4c] {
        poke(a, b, p + o, &pick!(rng, &[0, rng.next()]).to_le_bytes());
    }
    read_queue(a, b, rng, xrec);
    twin("MsWeaponRead", a, b, 0x007a_d820, &[i, k, id], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            sp::ms_weapon_read(m, &mut s, i, k, id);
        });
        0
    });
    same("MsWeaponRead", a, b);
    let frames = rng.below(20);
    twin(
        "MsWeaponFadeIn",
        a,
        b,
        0x007a_d570,
        &[i, k, frames],
        0,
        |m| {
            sp::ms_weapon_fade_in(m, &mut game_call_o, i, k, frames);
            0
        },
    );
    same("MsWeaponFadeIn", a, b);
    for (&(at, _), v) in areas.iter().zip(&keep) {
        poke(a, b, at, v);
    }
    unstub(a, b, &saved);
}

/// `MsMessageCueProcess`'s windows (system) and the camera move's step
/// (`0x007bcab0`, not ported here): logging stubs from the fourth slot of
/// [`START_STUBS2`], installed around [`messages_and_cameras`].
const FRAME2_CALLEES: [(u32, u8); 10] = {
    use ffx_battle::battle_end::bridge as be;
    [
        (be::TO_BTL_DRAW_STD_CHR_NAME_MESSAGE_WINDOW, 2),
        (be::TO_BTL_DRAW_FIRST_STRIKE_PLAYER_MESSAGE_WINDOW, 0),
        (be::TO_BTL_DRAW_FIRST_STRIKE_ENEMY_MESSAGE_WINDOW, 0),
        (be::TO_BTL_DRAW_GET_ITEM_MESSAGE_WINDOW, 2),
        (be::TO_BTL_DRAW_CAPTURE_MONSTER_MESSAGE_WINDOW, 2),
        (be::TO_BTL_DRAW_LEARNING_MESSAGE_WINDOW, 2),
        (be::TO_BTL_DRAW_GET_LIMIT_TYPE_MESSAGE_WINDOW, 2),
        (be::TO_BTL_DRAW_GET_MONEY_MESSAGE_WINDOW, 1),
        (be::TO_BTL_CLOSE_SIMPLE_HELP_MES, 0),
        (ffx_battle::camera::bridge::CAMERA_MOVE_STEP, 4),
    ]
};

/// The battle messages shown one after the other
/// (`MsMessageCueProcess`) and the script cameras' move reset
/// (`MsAtelResetMoveCameraID` down to `MsSetCheckChange`), on random
/// queues and cameras. Its own random stream.
fn messages_and_cameras(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    script: bool,
) {
    use ffx_battle::battle_end::bridge as be;
    use ffx_battle::camera::bridge as cam;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(FRAME2_CALLEES) {
            if saved.len() < FRAME2_CALLEES.len() {
                let at = o.rebase(f);
                saved.push((f, o.read(at, 5).unwrap()));
            }
            logging_stub(
                o,
                f,
                START_STUBS2 + 0xc0 + 0x40 * k,
                rec + EXCHANGE_REC * k,
                n,
                0,
            );
        }
    }
    // the windows: an ID or closed at once (-1), never 0 (the original
    // opens the same message again for ever)
    for k in 0..8 {
        let r = [1u32, 7, u32::MAX, 0x40][rng.below(4) as usize];
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    let cue = ffx_battle::state::bridge::MESSAGE_CUE.ptr(b).addr;
    let mut q = vec![0u8; 48];
    for e in q.chunks_mut(12) {
        for x in e.iter_mut() {
            *x = rng.byte();
        }
        e[0] = [1, 2, 3, 4, 5, 6, 7, 8, 0, 9][rng.below(10) as usize];
        e[1] = [0, 3, 10][rng.below(3) as usize];
        e[2] = [0, 0, 1, 3, 5, 10, 30][rng.below(7) as usize];
    }
    poke(a, b, cue, &q);
    set(
        a,
        b,
        ffx_battle::state::bridge::MESSAGE_CUE_COUNT,
        [0u8, 1, 2, 4, 3][rng.below(5) as usize],
    );
    set(
        a,
        b,
        ffx_battle::state::bridge::MESSAGE_CUE_START,
        rng.byte(),
    );
    set(
        a,
        b,
        be::MESSAGE_WINDOW,
        [0i32, 0, 3, -1][rng.below(4) as usize],
    );
    set(
        a,
        b,
        be::MESSAGE_TIME,
        [
            [0u8, 0],
            [1, 5],
            [4, 2],
            [3, 2],
            [2, 1],
            [rng.byte(), rng.byte()],
        ][rng.below(6) as usize],
    );
    set(
        a,
        b,
        be::PAD,
        [0u8, 0x20, 0x20, rng.byte()][rng.below(4) as usize],
    );
    set(
        a,
        b,
        be::PAD_OLD,
        [0u32, 0x20, rng.next()][rng.below(3) as usize],
    );
    sep_3d_list_used(a, b, rng);
    twin("MsMessageCueProcess", a, b, 0x0079_cdf0, &[], ALL, |m| {
        be::ms_message_cue_process(m, &stand_in, &mut game_call_o)
    });
    same("MsMessageCueProcess", a, b);
    // three cameras of random floats; slots whose requests are the
    // script cameras' at times
    let cams = cam::CAMERAS.ptr(b).addr;
    let words: Vec<u8> = (0..3 * cam::CAMERA_SIZE / 4)
        .flat_map(|_| float(rng).to_le_bytes())
        .collect();
    poke(a, b, cams, &words);
    let mut ids = Vec::new();
    for c in 0..3 {
        let base = cams + c * cam::CAMERA_SIZE;
        for o in [0x31, 0x33, 0x31 + 0x61c, 0x33 + 0x61c] {
            poke(a, b, base + o, &[u8::try_from(rng.below(3)).unwrap()]);
        }
        for o in [
            0xd84,
            0xd94,
            0x5bc,
            0x5cc,
            0x5dc,
            0x5bc + 0x61c,
            0x5cc + 0x61c,
            0x5dc + 0x61c,
        ] {
            let v = u16::from(rng.below(3) == 0);
            poke(a, b, base + o, &v.to_le_bytes());
        }
        for n in 0..4 {
            let id = c | rng.below(6) << 8 | rng.below(3) << 16;
            ids.push(id);
            let mut s = id.to_le_bytes().to_vec();
            s.extend([
                u8::from(rng.below(5) == 0),
                u8::from(rng.below(5) == 0),
                rng.byte() & 3,
            ]);
            poke(
                a,
                b,
                cam::SLOTS_AT.ptr(b).addr + c * cam::CAMERA_SIZE + 8 * n,
                &s,
            );
        }
    }
    let pick = |rng: &mut Xorshift| {
        if rng.below(4) == 0 {
            rng.next() & 0x03_0502
        } else {
            ids[rng.below(u32::try_from(ids.len()).unwrap()) as usize]
        }
    };
    let atel: Vec<u8> = (0..12).flat_map(|_| pick(rng).to_le_bytes()).collect();
    poke(
        a,
        b,
        ffx_battle::minor::bridge::ATEL_CAMERA.ptr(b).addr,
        &atel,
    );
    let id = pick(rng);
    let (side, step, mode) = (
        rng.below(2),
        rng.below(3),
        [0, 1, 2, 3][rng.below(4) as usize],
    );
    twin(
        "MsSetCheckChange",
        a,
        b,
        0x007c_0790,
        &[id, side, step, mode],
        ALL,
        |m| cam::ms_set_check_change(m, &mut game_call_o, id, side, step, mode),
    );
    same("MsSetCheckChange", a, b);
    let id = pick(rng);
    let side = rng.below(4);
    let p = cams + rng.below(3) * cam::CAMERA_SIZE + 0x80;
    twin(
        "MsCameraSetRect",
        a,
        b,
        0x007b_f940,
        &[id, side, p],
        ALL,
        |m| cam::ms_camera_set_rect(m, &mut game_call_o, id, side, p),
    );
    same("MsCameraSetRect", a, b);
    let id = pick(rng);
    let flags = [0, 1, 8, 9, rng.below(0x10)][rng.below(5) as usize];
    twin(
        "MsCameraResetMove",
        a,
        b,
        0x007b_ed60,
        &[id, flags],
        ALL,
        |m| cam::ms_camera_reset_move(m, &mut game_call_o, id, flags),
    );
    same("MsCameraResetMove", a, b);
    let kind = [0, 1, 2, 3, 4, rng.next()][rng.below(6) as usize];
    twin(
        "MsAtelResetMoveCameraID",
        a,
        b,
        0x007b_a860,
        &[kind],
        ALL,
        |m| cam::ms_atel_reset_move_camera_id(m, &mut game_call_o, kind),
    );
    same("MsAtelResetMoveCameraID", a, b);
    // the script's IDs (round 28): kept, or got from a free slot (some
    // cleared), every kind and camera and beyond
    for k in 0..12 {
        if rng.below(3) == 0 {
            let at = ffx_battle::minor::bridge::ATEL_CAMERA.ptr(b).addr + 4 * k;
            poke(a, b, at, &[0; 4]);
        }
    }
    for _ in 0..3 {
        let owner = rng.below(0x100);
        let kind = [0, 1, 2, 3, 4, rng.next()][rng.below(6) as usize];
        let c = [0, 1, 2, 3, rng.next()][rng.below(5) as usize];
        twin(
            "MsAtelGetCameraID",
            a,
            b,
            0x007b_a780,
            &[owner, kind, c],
            ALL,
            |m| cam::ms_atel_get_camera_id(m, owner, kind, c),
        );
        same("MsAtelGetCameraID", a, b);
    }
    camera_setters(a, b, rng, &pick, script);
    unstub(a, b, &saved);
}

/// The camera request setters (round 28) on [`messages_and_cameras`]'
/// cameras: requests the slots name (current or not) and others, both
/// sides and beyond, script actors only on a snapshot, the vector from a
/// camera, the aims' floats (random, zero divisors at times).
fn camera_setters(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    pick: &dyn Fn(&mut Xorshift) -> u32,
    script: bool,
) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    let mut call = game_call;
    let cams = cam::CAMERAS.ptr(b).addr;
    let actor = |rng: &mut Xorshift| -> u32 {
        if script && rng.below(4) != 0 {
            rng.below(4)
        } else {
            [0xffff, rng.next()][rng.below(2) as usize] | 0xffff
        }
    };
    let id = pick(rng);
    let side = rng.below(4);
    let (x, y) = (actor(rng), actor(rng));
    let p = cams + rng.below(3) * cam::CAMERA_SIZE + 0x80;
    twin(
        "MsCameraSetAtelFocus_007beeb0",
        a,
        b,
        0x007b_eeb0,
        &[id, side, x, y, p],
        ALL,
        |m| {
            dm(m, |m| {
                cam::ms_camera_set_atel_focus(m, &h, &mut call, id, side, x, y, p)
            })
        },
    );
    same("MsCameraSetAtelFocus_007beeb0", a, b);
    // the aims' bases (`+0x8c`): none, or the other side's position
    for c in 0..3 {
        for k in 0..2 {
            let s = cams + c * cam::CAMERA_SIZE + 0x30 + 0x61c * k;
            let base = [
                0,
                cams + c * cam::CAMERA_SIZE + 0x30 + 0x61c * (1 - k) + 0x50,
            ][rng.below(2) as usize];
            poke(a, b, s + 0x8c, &base.to_le_bytes());
        }
    }
    // the other side's channel's actors for the yaw's track
    for c in 0..3 {
        for n in 0..6 {
            for k in 0..2 {
                let at = cams + c * cam::CAMERA_SIZE + 0x70c + 0x130 * n + 0xc;
                let at = if k == 0 { at } else { at - 0x61c };
                let v: Vec<u8> = (0..2)
                    .flat_map(|_| u16::try_from(actor(rng) & 0xffff).unwrap().to_le_bytes())
                    .collect();
                poke(a, b, at, &v);
            }
        }
    }
    let id = pick(rng);
    let side = rng.below(4);
    let v = [float(rng), float(rng), float(rng)];
    twin(
        "MsCameraSetAtelYaw_007befc0",
        a,
        b,
        0x007b_efc0,
        &[id, side, v[0], v[1], v[2]],
        ALL,
        |m| {
            dm(m, |m| {
                cam::ms_camera_set_atel_yaw(m, &h, &mut call, id, side, v[0], v[1], v[2])
            })
        },
    );
    same("MsCameraSetAtelYaw_007befc0", a, b);
    let mut six = || {
        let mut v = [0u32; 6];
        for w in &mut v {
            *w = float(rng);
        }
        // zero divisors at times (the aims skip the division)
        for k in [3, 5] {
            if rng.below(3) == 0 {
                v[k] = 0;
            }
        }
        v
    };
    let v = six();
    let w = six();
    let id = pick(rng);
    let side = rng.below(4);
    zero_stack(a);
    let scratch = a.stack_range().end - 0x200;
    twin(
        "MsCameraSetAimPolar_007bf330",
        a,
        b,
        0x007b_f330,
        &[id, side, v[0], v[1], v[2], v[3], v[4], v[5]],
        ALL,
        |m| cam::ms_camera_set_aim_polar(m, &stand_in, &mut game_call_o, id, side, v, scratch),
    );
    same("MsCameraSetAimPolar_007bf330", a, b);
    let id = pick(rng);
    let side = rng.below(4);
    let short = [0, 1, rng.next()][rng.below(3) as usize];
    zero_stack(a);
    twin(
        "MsCameraSetAimPolar2_007bf420",
        a,
        b,
        0x007b_f420,
        &[id, side, short, w[0], w[1], w[2], w[3], w[4], w[5]],
        ALL,
        |m| {
            cam::ms_camera_set_aim_polar2(
                m,
                &stand_in,
                &mut game_call_o,
                id,
                side,
                short,
                w,
                scratch,
            )
        },
    );
    same("MsCameraSetAimPolar2_007bf420", a, b);
}

/// Party members and aeons coming and going (`ffx_battle::exchange`).
/// What `MsSetRamChr` reads for every actor (round 34): the kernel's
/// character records, the item memories and, for each monster, a record
/// (`+0xf80`) and data (`+0x48`, null at times) of our own where the
/// snapshot has none; `rec` holds 8 records of 0x80 bytes and 8 data
/// blocks of 0x200.
fn ram_chr_world(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32) {
    use ffx_battle::misc::bridge::{ITEM_MEM, ITEM_MEM2};
    use ffx_battle::save_param::bridge as sp;
    // only where there are none: other sections use the same tables
    // (MsSetSaveParam divides by a ply_rom byte +0x1c)
    let rom = ffx_battle::save::bridge::KERNEL_PLY_ROM;
    if !readable(a, rom.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x14, 0x30);
        for k in 0..0x14 {
            poke(a, b, t + 0x20 + 0x30 * k + 0x1c, &[1 + rng.byte() % 0x7f]);
        }
        set(a, b, rom, t);
    }
    // the item memories of all 0x1f actors (MsSetRamChr writes the
    // monsters' too): ours in `rec` but on a snapshot that has them
    for (g, n, at) in [
        (ITEM_MEM, sp::ITEM_MEM_SIZE, rec + 0x3600),
        (ITEM_MEM2, sp::ITEM_MEM2_SIZE, rec + 0x5800),
    ] {
        if w.synthetic || !readable(a, g.get(b), (ACTORS * n) as usize) {
            set(a, b, g, at);
        }
    }
    let aab = ffx_kernel::bridge::A_ABILITY;
    if !readable(a, aab.get(b), 0x20) {
        let t = battle_common::world::fake_records(a, b, rng, 0x100, 0x80);
        set(a, b, aab, t);
    }
    let mon_top = ffx_battle::actor::bridge::MON_TOP;
    if !readable(a, mon_top.get(b), 0x10) {
        set(a, b, mon_top, w.actor(0x14));
    }
    for k in 0..8 {
        let p = w.actor(0x14 + k);
        let (record, data) = (rec + 0x80 * k, rec + 0x400 + 0x200 * k);
        let real = u32::from_le_bytes(b.read(p + 0xf80, 4).unwrap().try_into().unwrap());
        if w.synthetic || !readable(a, real, 0x80) {
            fill(a, b, rng, record, 0x80);
            poke(a, b, p + 0xf80, &record.to_le_bytes());
        }
        // real data with its 0x46 item words (+0x14 their offset), or ours
        let real = u32::from_le_bytes(b.read(p + 0x48, 4).unwrap().try_into().unwrap());
        let usable = real == 0
            || (readable(a, real, 0x18) && {
                let o = u32::from_le_bytes(b.read(real + 0x14, 4).unwrap().try_into().unwrap());
                readable(a, real.wrapping_add(o), 0x118)
            });
        if w.synthetic || !usable {
            fill(a, b, rng, data, 0x200);
            poke(
                a,
                b,
                data + 0x14,
                &(0x20 + 4 * rng.below(0x20)).to_le_bytes(),
            );
            let v = if rng.below(3) == 0 { 0 } else { data };
            poke(a, b, p + 0x48, &v.to_le_bytes());
        }
    }
}

/// `MsSetChrVisibleBtl2Event`'s chara and vfx callees that the fake
/// characters can't take (`Ch_SetTransparent` and `Ch_SetHideEffect` run:
/// the visibility code under `MsRelife` calls them in Rust): logging stubs in
/// `MsDebugEnvSave` (`0x007cd800`, debug only) after [`END_STUBS`]'s,
/// installed around the call (the test's characters are fakes).
const REVIVE_CALLEES: [(u32, u8); 2] = {
    use ffx_battle::exchange::bridge as x;
    [
        (x::CH_SET_SHADE_COL, 5),
        (x::OP_CHR_STATUS_RESET_EXCEPTION_WRAP, 1),
    ]
};
const REVIVE_STUBS: u32 = END_STUBS + 0x40 * 16;

/// Actors brought back after an event (`MsSetChrVisibleBtl2Event`, round
/// 34): a few at random (at times none, or a monster), `actor` one of
/// them or not, the status reset or not; then `Mon_LoadAll`.
fn revive(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::exchange::bridge as x;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    ram_chr_world(a, b, rng, w, rec);
    let mut mask = 0u32;
    for _ in 0..rng.below(4) {
        mask |= 1 << rng.below(ACTORS);
    }
    if rng.below(8) == 0 {
        mask |= 0x8000_0000;
    }
    for k in 0..ACTORS {
        let p = w.actor(k);
        for o in [0x508, 0x608, 0x432, 0xdcc, 0x3b, 0x414] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
        poke(a, b, p + 0x43f, &[0xff]);
        poke(a, b, p + 0x13, &[0xff]);
        poke(a, b, p + 0xf7c, &[0; 4]);
        // a character at times (the fakes are only logged: REVIVE_CALLEES)
        if rng.below(4) == 0 {
            poke(a, b, p, &[0; 4]);
        }
    }
    let actor = match rng.below(3) {
        0 => 0,
        1 => w.actor(rng.below(ACTORS)),
        _ => {
            let k = (0..ACTORS).find(|&k| mask >> k & 1 != 0).unwrap_or(0);
            w.actor(k)
        }
    };
    let reset = u32::from(rng.below(2) == 0);
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(REVIVE_CALLEES) {
            let at = o.rebase(f);
            saved.push((f, o.read(at, 5).unwrap()));
            logging_stub(
                o,
                f,
                REVIVE_STUBS + 0x80 * k,
                rec + 0x2000 + 0x200 * k,
                n,
                0,
            );
        }
    }
    saved.truncate(saved.len() / 2);
    twin(
        "MsSetChrVisibleBtl2Event",
        a,
        b,
        0x007a_89c0,
        &[actor, mask, reset],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                x::ms_set_chr_visible_btl2_event(m, &mut s, actor, mask, reset)
            })
        },
    );
    unstub(a, b, &saved);
    same("MsSetChrVisibleBtl2Event", a, b);
    // the monsters' scripts queued in manager 1 (full at times) and the
    // managers set up: the script module's and the allocator's calls
    // MsEffectStart's logging stubs
    let managers = ffx_battle::minor::bridge::ATEL_MANAGERS.ptr(b).addr;
    let count = [0u32, 1, 5, 7, 8][rng.below(5) as usize];
    poke(a, b, managers + 0x54, &count.to_le_bytes());
    let none = rng.below(4) == 0;
    for k in 0..8 {
        let p = w.actor(0x14 + k);
        poke(a, b, p + 0x10, &[u8::from(rng.below(3) != 0)]);
        poke(a, b, p + 0xf78, &rng.next().to_le_bytes());
        // at times no monster with data (no scripts)
        if none {
            poke(a, b, p + 0x48, &[0; 4]);
        }
        poke(
            a,
            b,
            p + 0xe,
            &u16::try_from(rng.below(0x10000)).unwrap().to_le_bytes(),
        );
    }
    let saved = start_stubs(a, b, rec + 0x2800);
    twin("Mon_LoadAll", a, b, 0x0078_4070, &[], 0, |m| {
        dm(m, |m| {
            ffx_battle::read::bridge::mon_load_all(m, &mut game_call);
            0
        })
    });
    unstub(a, b, &saved);
    same("Mon_LoadAll", a, b);
}

fn exchange(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32, rec: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::exchange::bridge as x;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    macro_rules! s {
        ($s:ident) => {
            let mut $s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
        };
    }
    cues(a, b, rng, w);
    for k in 0..ACTORS {
        let p = w.actor(k);
        poke(a, b, p + 0x43f, &[0xff]);
        poke(a, b, p + 0x13, &[0xff]);
        poke(a, b, p + 0xf7c, &[0; 4]);
        let t = |rng: &mut Xorshift| {
            if rng.below(2) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(ACTORS)).unwrap()
            }
        };
        let (t5, t6) = (t(rng), t(rng));
        poke(a, b, p + 0x5c5, &[t5, t6]);
        let flags = u16::from_le_bytes(b.read(p + 0x606, 2).unwrap().try_into().unwrap());
        let threat = if rng.below(3) == 0 { 0x800 } else { 0 };
        poke(a, b, p + 0x606, &(flags & !0x800 | threat).to_le_bytes());
        for o in [0x10, 0x35, 0xdcd, 0xdcc, 0x512, 0xdc8, 0xdca, 0x11] {
            poke(a, b, p + o, &[u8::from(rng.below(3) != 0)]);
        }
        poke(a, b, p + 0x511, &[u8::try_from(rng.below(3)).unwrap()]);
        // free to play a motion (MsCheckSetMotion, MsThreatEndMotion)
        if rng.below(3) == 0 {
            for o in [0x508, 0x608, 0x432, 0xdcc] {
                poke(a, b, p + o, &[0]);
            }
            let f = b.read(p + 0x606, 1).unwrap()[0] & !4;
            poke(a, b, p + 0x606, &[f]);
            let f = b.read(p + 0x617, 1).unwrap()[0] & !1;
            poke(a, b, p + 0x617, &[f]);
        }
        let e = if rng.below(3) == 0 { 0 } else { rng.next() };
        poke(a, b, p + 0x4c, &e.to_le_bytes());
        for o in [0x592, 0x593] {
            let id =
                [0xff, rng.byte(), u8::try_from(rng.below(0x20)).unwrap()][rng.below(3) as usize];
            poke(a, b, p + o, &[id]);
        }
    }
    read_queue(a, b, rng, rec);
    // the leaves
    let id = [0xff, rng.below(0x100), rng.below(0x20)][rng.below(3) as usize];
    twin("MsGetSaveWeaponModel", a, b, 0x007a_bdc0, &[id], ALL, |m| {
        x::ms_get_save_weapon_model(m, id)
    });
    let list = x::BOUND_WEAPONS.ptr(b).addr;
    let id = if rng.below(2) == 0 {
        let k = rng.below(4);
        u32::from(u16::from_le_bytes(
            b.read(list + 2 * k, 2).unwrap().try_into().unwrap(),
        ))
    } else {
        rng.below(0x10000)
    };
    twin("MsCheckBoundWeapon", a, b, 0x007a_d130, &[id], ALL, |m| {
        x::ms_check_bound_weapon(m, id)
    });
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin("MsEffectWeaponFree", a, b, 0x0078_8b30, &[i], 0, |m| {
        dm(m, |m| {
            s!(s);
            x::ms_effect_weapon_free(m, &mut s, i);
            0
        })
    });
    same("MsEffectWeaponFree", a, b);
    twin(
        "MsThreatEndMotion",
        a,
        b,
        0x0078_f6a0,
        &[i, actor],
        0,
        |m| {
            dm(m, |m| {
                s!(s);
                x::ms_threat_end_motion(m, &mut s, i, actor);
                0
            })
        },
    );
    same("MsThreatEndMotion", a, b);
    // the threat, its first yaw from the original's garbage
    let threat = b.read(actor + 0x607, 1).unwrap()[0] & 8 != 0;
    let t = u32::from(b.read(actor + 0x5c5, 1).unwrap()[0]);
    let ok = writes(a, b, 0x0079_48e0, i, out + 0x40)
        && (t == 0xff || writes(a, b, 0x0079_48e0, t, out + 0x40));
    if !threat || ok {
        twin(
            "MsThreatProcess",
            a,
            b,
            0x0078_e460,
            &[i, actor],
            ALL,
            |m| {
                dm(m, |m| {
                    s!(s);
                    x::ms_threat_process(m, &h, &mut s, i, actor)
                })
            },
        );
        if threat {
            copy_garbage(a, b, actor + 0x51c, 4);
        }
        same("MsThreatProcess", a, b);
    }
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin("MsThreatOff", a, b, 0x0078_e410, &[i, actor], 0, |m| {
        dm(m, |m| {
            s!(s);
            x::ms_threat_off(m, &h, &mut s, i, actor);
            0
        })
    });
    same("MsThreatOff", a, b);
    // the read queue
    twin("MsBtlReadCueManage", a, b, 0x007a_d190, &[], 0, |m| {
        dm(m, |m| {
            s!(s);
            x::ms_btl_read_cue_manage(m, &mut s);
            0
        })
    });
    same("MsBtlReadCueManage", a, b);
    read_queue(a, b, rng, rec);
    // (a kind 1 entry spins in the original until something else starts
    // it: no caller queues one)
    let args = [
        [0, 2, 3][rng.below(3) as usize],
        rng.below(ACTORS),
        rng.below(ACTORS),
        rng.below(0x10000),
        rng.below(12),
    ];
    twin("MsBtlReadCueRegist", a, b, 0x007a_d3f0, &args, 0, |m| {
        dm(m, |m| {
            s!(s);
            x::ms_btl_read_cue_regist(m, &mut s, args[0], args[1], args[2], args[3], args[4]);
            0
        })
    });
    same("MsBtlReadCueRegist", a, b);
    read_queue(a, b, rng, rec);
    let which = rng.below(4);
    twin(
        "MsBtlReadCueRegistWeapon",
        a,
        b,
        0x007a_d470,
        &[i, which],
        0,
        |m| {
            dm(m, |m| {
                s!(s);
                x::ms_btl_read_cue_regist_weapon(m, &mut s, i, which);
                0
            })
        },
    );
    same("MsBtlReadCueRegistWeapon", a, b);
    // the directions (positions the originals write only)
    if placed(a, b, w, out) {
        set(a, b, x::CENTER_SCALE, float(rng));
        let i = rng.below(ACTORS);
        if writes(a, b, 0x0079_48e0, i, out + 0x40) {
            ftwin("MsCalcChrDir", a, b, 0x0079_3ba0, &[i, w.actor(i)], |m| {
                dm(m, |m| x::ms_calc_chr_dir(m, &h, i, Some(HARNESS)))
            });
            local_words(a, b);
            rect_words(a, b);
            same("MsCalcChrDir", a, b);
        }
        // a monster that turns
        let m = w.actor(0x14 + rng.below(8));
        poke(a, b, m + 0x10, &[1]);
        for o in [0xdcd, 0xdcc, 0x512] {
            poke(a, b, m + o, &[0]);
        }
        poke(a, b, m + 0xdc8 + 2 * rng.below(2), &[1]);
        let free = [0, 1, 0xff][rng.below(3) as usize];
        set(a, b, ffx_battle::minor::bridge::DIR_FREE, free);
        twin("MsChangeMonsterDir", a, b, 0x0079_3cf0, &[], 0, |m| {
            dm(m, |m| {
                s!(s);
                x::ms_change_monster_dir(m, &h, &mut s);
                0
            })
        });
        local_words(a, b);
        rect_words(a, b);
        same("MsChangeMonsterDir", a, b);
    }
    // the exchanges (no threats: MsThreatOff's own release only)
    read_queue(a, b, rng, rec);
    cues(a, b, rng, w);
    let party: Vec<u8> = (0..7)
        .map(|_| [0xff, u8::try_from(rng.below(0x12)).unwrap()][rng.below(2) as usize])
        .collect();
    poke(a, b, ffx_battle::menu::bridge::PARTY.ptr(b).addr, &party);
    let reserve: Vec<u8> = (0..0x11)
        .map(|_| [0xff, u8::try_from(rng.below(0x12)).unwrap()][rng.below(2) as usize])
        .collect();
    poke(
        a,
        b,
        ffx_battle::flow::bridge::PARTY_EXTRA.ptr(b).addr,
        &reserve,
    );
    let who =
        |rng: &mut Xorshift| [0xff, rng.below(0x12), rng.below(ACTORS)][rng.below(3) as usize];
    let (o, n) = (who(rng), who(rng));
    let mode = rng.range(-1, 2);
    let after = rng.below(ACTORS + 2);
    twin(
        "MsExchangeChr",
        a,
        b,
        0x007a_dae0,
        &[o, n, mode.cast_unsigned(), after],
        ALL,
        |m| {
            dm(m, |m| {
                s!(s);
                x::ms_exchange_chr(m, &h, &mut s, o, n, mode, after)
            })
        },
    );
    local_words(a, b);
    same("MsExchangeChr", a, b);
    read_queue(a, b, rng, rec);
    // no input or command entries: deleting the random ones writes through
    // their random bytes (into the positions MsCalcChrDir needs)
    set(a, b, ffx_battle::state::bridge::INPUT_CUE_COUNT, 0);
    set(a, b, ffx_battle::state::bridge::COMMAND_CUE_COUNT, 0);
    // (the monsters turn towards the party: every able actor placed, the
    // aeons coming too)
    let inn = rng.below(8);
    let summon = 8 + rng.below(10);
    if !placed(a, b, w, out)
        || ![
            summon, 0xf, 0x10, 0x11, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
        ]
        .into_iter()
        .all(|k| writes(a, b, 0x0079_48e0, k, out + 0x40))
    {
        return;
    }
    set(a, b, x::CENTER_SCALE, float(rng));
    twin(
        "MsExchangeSummon",
        a,
        b,
        0x007a_df90,
        &[inn, summon],
        0,
        |m| {
            dm(m, |m| {
                s!(s);
                x::ms_exchange_summon(m, &h, &mut s, inn, summon);
                0
            })
        },
    );
    local_words(a, b);
    rect_words(a, b);
    same("MsExchangeSummon", a, b);
    // the party back from a summon (ffx_battle::damage_end)
    {
        use ffx_battle::damage_end::bridge as de;
        let summoner = [0, 0, rng.byte().cast_signed()][rng.below(3) as usize];
        set(a, b, ffx_battle::target::bridge::SUMMONER, summoner);
        let slot = [
            0xff,
            u8::try_from(summon).unwrap(),
            u8::try_from(rng.below(0x12)).unwrap(),
        ][rng.below(3) as usize];
        set(
            a,
            b,
            ffx_battle::ability::bridge::LIST_SLOT,
            slot.cast_signed(),
        );
        set(
            a,
            b,
            ffx_battle::action::bridge::TURN_END_ACTOR,
            u8::try_from(rng.below(8)).unwrap(),
        );
        let (what, sym, arg) = [
            ("MsFreePlayer", 0x007a_e0a0, rng.below(0x12)),
            ("MsExchangePlayer", 0x007a_df10, 0),
            ("MsReturnSummon", 0x007a_ef20, rng.below(2)),
        ][rng.below(3) as usize];
        let args: &[u32] = if sym == 0x007a_df10 { &[] } else { &[arg] };
        twin(what, a, b, sym, args, 0, |m| {
            dm(m, |m| {
                s!(s);
                match sym {
                    0x007a_e0a0 => de::ms_free_player(m, &h, &mut s, arg),
                    0x007a_df10 => de::ms_exchange_player(m, &h, &mut s),
                    _ => de::ms_return_summon(m, &h, &mut s, arg),
                }
            });
            0
        });
        local_words(a, b);
        rect_words(a, b);
        same(what, a, b);
    }
}

/// Which of [`check`]'s sections run, and how often. The whole test
/// ([`Sel::ALL`]) runs every section 400 times and the teardown 200
/// times, in one random stream; a `part_*` test (round 31, [`part!`])
/// runs one section on the same set-up, so a mutation run can select the
/// sections that reach its functions (`MUT_FILTER=part_moves`) instead
/// of paying ~200 s for the whole file per mutant.
struct Sel {
    /// The sections by name; empty for all.
    only: &'static [&'static str],
    iters: u32,
    teardowns: u32,
}

impl Sel {
    const ALL: Sel = Sel {
        only: &[],
        iters: 400,
        teardowns: 200,
    };

    fn on(&self, name: &str) -> bool {
        self.only.is_empty() || self.only.contains(&name)
    }

    /// A section the whole test runs when `chance` came up: a part test
    /// runs it every time (the random draw still happens first).
    fn pick(&self, chance: bool, name: &str) -> bool {
        if self.only.is_empty() {
            chance
        } else {
            self.on(name)
        }
    }
}

/// One section of [`check`] on the image and on `FFX_SNAPSHOT`, `n`
/// rounds (the teardown `n` times), after the sections it needs (listed
/// first: what they leave behind, a section of the whole test reads; found
/// by trying them on the image and both snapshots). `effect_start` has
/// none: it needs sections later in the whole test's loop.
macro_rules! part {
    ($name:ident, $section:literal, $n:expr) => {
        part!($name, [$section], $n);
    };
    ($name:ident, [$($section:literal),+], $n:expr) => {
        mod $name {
            use super::*;

            const SEL: Sel = Sel {
                only: &[$($section),+],
                iters: $n,
                teardowns: $n,
            };

            #[test]
            fn image() {
                let Some((mut a, mut b)) = synthetic() else {
                    return;
                };
                check(&mut a, &mut b, &mut Xorshift(0x7a46_1001), true, &SEL);
            }

            #[test]
            fn snapshot() {
                let Some((mut a, mut b)) = battle_common::snapshot() else {
                    return;
                };
                check(&mut a, &mut b, &mut Xorshift(0x7a46_1002), false, &SEL);
            }
        }
    };
}

part!(part_calls, "calls", 60);
part!(part_near, "near", 60);
part!(part_status, "status", 60);
part!(part_actions, "actions", 60);
part!(part_exchange, "exchange", 60);
part!(part_effects, "effects", 60);
part!(part_parse, ["status", "effects", "parse"], 60);
part!(part_command_cue, ["calls", "status", "command_cue"], 60);
part!(part_process, "process", 60);
part!(part_deaths, ["status", "deaths"], 60);
part!(part_input, ["exchange", "input"], 60);
part!(
    part_damage_end,
    [
        "near",
        "actions",
        "exchange",
        "effects",
        "parse",
        "damage_end"
    ],
    60
);
part!(part_revive, "revive", 60);
part!(part_bridge_to_battle, "bridge_to_battle", 60);
part!(part_models_freed, "models_freed", 60);
part!(part_battle_end, ["process", "battle_end"], 60);
part!(part_battle_frame, "battle_frame", 60);
part!(part_save_params, "save_params", 60);
part!(part_messages_and_cameras, "messages_and_cameras", 60);
part!(part_moves, "moves", 100);
part!(part_move_steps, "move_steps", 60);
part!(part_ball_splines, "ball_splines", 60);
part!(part_motions, ["status", "save_params", "motions"], 60);
part!(
    part_atel_update0,
    ["status", "save_params", "motions", "atel_update0"],
    40
);
part!(part_special_stage, "special_stage", 60);
part!(part_bridge_to_event, "bridge_to_event", 40);
part!(part_setup_leaves, "setup_leaves", 40);
part!(part_exe_leaves, "exe_leaves", 40);
part!(
    part_read_leaves,
    ["exchange", "save_params", "read_leaves"],
    40
);
part!(
    part_read_manage,
    ["exchange", "save_params", "read_manage"],
    40
);
part!(part_debug_full_command, "debug_full_command", 40);
part!(part_weapon_plate, "weapon_plate", 40);
part!(part_init_leaves, "init_leaves", 40);
part!(part_save_start, "save_start", 40);
part!(part_battle_init, "battle_init", 40);
part!(part_battle_process, "battle_process", 40);
part!(part_read_set_scene, "read_set_scene", 40);
part!(part_read_monster, "read_monster", 40);
part!(part_target_dist, "target_dist", 150);
part!(part_et_effects, "et_effects", 200);
part!(part_effect_signals, "effect_signals", 200);
part!(part_cd_reads, "cd_reads", 100);
part!(part_teardown, "teardown", 100);

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool, sel: &Sel) {
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
    let out = scratch(a, b, 0x100);
    let s = scratch(a, b, 0x10);
    SCRATCH.with(|c| c.set(s));
    let block = scratch(a, b, 0x20);
    BLOCK.with(|c| c.set(block));
    let lists = scratch(a, b, 0x80);
    LISTS.with(|c| c.set(lists));
    let rec = scratch(a, b, 0x80);
    for o in [&mut *a, &mut *b] {
        script_stubs(o, rec);
        let at = o.rebase(0x0082_bee0);
        o.stub_return(at, 0).unwrap();
        // op_et_eff, op_et_astatus_effect_change, Ch_ModelSetHide: what
        // they got is compared
        for (k, (f, n)) in [(0x007f_c2e0, 4), (0x007f_aed0, 2), (0x0082_aea0, 3)]
            .into_iter()
            .enumerate()
        {
            let k = u32::try_from(k).unwrap();
            recording_stub(
                o,
                f,
                STUBS + 0x40 + 0x40 * k,
                rec + 0x20 + 0x20 * k,
                n,
                0x4000_0123,
            );
        }
    }
    if w.synthetic {
        // no battle scripts (a field snapshot keeps the last battle's freed
        // ones)
        let records = ffx_battle::atel::bridge::SCRIPT_RECORDS.ptr(b).addr;
        poke(a, b, records + 0x10, &[0; 4]);
        poke(a, b, records + 0x30, &[0; 4]);
        // the voice event flag MsSetVoiceCheck reads through a pointer
        let flag = scratch(a, b, 4);
        set(a, b, ffx_battle::state::bridge::VOICE_EVENT, flag);
    }
    // the battle snapshot's models (their actors' first words)
    let live: Vec<u32> = if w.synthetic {
        Vec::new()
    } else {
        (0..ACTORS)
            .map(|k| u32::from_le_bytes(b.read(w.actor(k), 4).unwrap().try_into().unwrap()))
            .collect()
    };
    LIVE_CHRS.with(|c| *c.borrow_mut() = live);
    let vrec = scratch(a, b, 0xe300);
    let xrec = scratch(a, b, (EXCHANGE_REC * 12) as usize);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(EXCHANGE_CALLEES) {
            logging_stub(
                o,
                f,
                EXCHANGE_STUBS + 0x40 * k,
                xrec + EXCHANGE_REC * k,
                n,
                0,
            );
        }
    }
    let erec = scratch(a, b, (EXCHANGE_REC * 12) as usize);
    effect_stubs(a, b, erec);
    let files = scratch(a, b, 0x802);
    // the music volume's sound callees (SndGetNowPlayMusic,
    // SndMusicVolumeTranslate), after the exchange stubs and inside the CD
    // call's function
    let srec = scratch(a, b, (EXCHANGE_REC * 5) as usize);
    for o in [&mut *a, &mut *b] {
        use ffx_battle::exec::bridge as e;
        logging_stub(o, e::SND_GET_NOW_PLAY_MUSIC, 0x0078_06f0, srec, 0, 0x2d);
        logging_stub(
            o,
            e::SND_MUSIC_VOLUME_TRANSLATE,
            0x0078_0760,
            srec + EXCHANGE_REC,
            3,
            7,
        );
        // TOBtlOpenSignWindow (system), MsMenuOpenTitleWindow's window
        logging_stub(o, 0x0089_8c60, 0x0078_08c0, srec + 2 * EXCHANGE_REC, 1, 0);
        // the command cue's waits: SndGetStreamState (sound) and the
        // system message's status (MsSystemMessageCheck)
        logging_stub(
            o,
            ffx_battle::parse::bridge::SND_GET_STREAM_STATE,
            STUBS + 0x100,
            srec + 3 * EXCHANGE_REC,
            0,
            0,
        );
        logging_stub(o, 0x008b_7ca0, 0x0078_03a0, srec + 4 * EXCHANGE_REC, 1, 0);
    }
    let prec = scratch(a, b, (EXCHANGE_REC * 13) as usize);
    process_stubs(a, b, prec, erec);
    // the input stubs' records, then a damage number pool
    let irec = scratch(a, b, 0x800 + 0x80 * 0x14);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(INPUT_CALLEES) {
            logging_stub(o, f, INPUT_STUBS + 0x40 * k, irec + EXCHANGE_REC * k, n, 0);
        }
    }
    // the disposal stubs' records, then a function an effect points at
    let frec = scratch(a, b, (EXCHANGE_REC * 9) as usize);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(FREE_CALLEES) {
            logging_stub(o, f, FREE_STUBS + 0x40 * k, frec + EXCHANGE_REC * k, n, 0);
        }
        logging_stub(o, 0, FREE_STUBS + 0x40 * 8, frec + EXCHANGE_REC * 8, 1, 0);
    }
    // the loop's sound stubs' records
    let grec = scratch(a, b, (EXCHANGE_REC * 8) as usize);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(FRAME_CALLEES) {
            logging_stub(o, f, FRAME_STUBS + 0x40 * k, grec + EXCHANGE_REC * k, n, 0);
        }
        // SndIsSepPlay: playing by the sound's low bit (a walk mixes
        // playing and stopped nodes)
        let at = o.rebase(FRAME_STUBS);
        o.write(at, &[0x8b, 0x44, 0x24, 0x04, 0x83, 0xe0, 0x01, 0xc3])
            .unwrap();
    }
    // the battle end's stubs' records
    let nrec = scratch(a, b, (EXCHANGE_REC * 18) as usize);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(END_CALLEES) {
            logging_stub(o, f, END_STUBS + 0x40 * k, nrec + EXCHANGE_REC * k, n, 0);
        }
        // the system message's window (TOGetWindowStatus,
        // TOCloseMessageWindow; the synthetic image has no windows), in
        // the process stubs' spare slots
        for (k, f) in [(16u32, 0x0090_6c20), (17, 0x008a_b8f0)] {
            logging_stub(
                o,
                f,
                PROCESS_STUBS + 0x40 * (k - 3),
                nrec + EXCHANGE_REC * k,
                1,
                0,
            );
        }
    }
    // MsEffectStart's stubs' records and a camera script file
    let effrec = scratch(a, b, (EXCHANGE_REC * 16) as usize);
    let cam = scratch(a, b, 0x40);
    let mrec = scratch(a, b, (EXCHANGE_REC * 10) as usize);
    let mut r15 = Xorshift(0x15e5_0001 ^ u32::from(synthetic));
    let mut r16 = Xorshift(0x16e5_0001 ^ u32::from(synthetic));
    let mut r17 = Xorshift(0x17e5_0001 ^ u32::from(synthetic));
    let mut r18m = Xorshift(0x18e5_0101 ^ u32::from(synthetic));
    let mut r19 = Xorshift(0x19e5_0001 ^ u32::from(synthetic));
    let mut r34 = Xorshift(0x34e5_0001 ^ u32::from(synthetic));
    let mut r20 = Xorshift(0x20e5_0001 ^ u32::from(synthetic));
    // a special stage's work (round 18): matrices, the sea's, angles
    let work = scratch(a, b, 0x870);
    // Ch_ModelCreatePacket's stub (round 18) and its record
    let crec = scratch(a, b, EXCHANGE_REC as usize);
    // the special stage's map callees (round 19) and the port's locals
    let srecs: Vec<u32> = SPECIAL_CALLEES
        .iter()
        .map(|(_, spec)| scratch(a, b, copying_rec_len(spec) as usize))
        .collect();
    let locals = scratch(a, b, ffx_battle::special::bridge::SPECIAL_LOCALS as usize);
    // MsBtlBridgeBtl2Event's tables and records (round 19)
    let btables = scratch(a, b, 0x40 * 8);
    let belems = scratch(a, b, 0x40 * 3);
    let brec = scratch(a, b, (EXCHANGE_REC * 15) as usize);
    // the setup leaves' stub records and heap block (round 19)
    let grec2 = scratch(a, b, (EXCHANGE_REC * 2) as usize);
    let erec2 = scratch(a, b, (EXCHANGE_REC * 27) as usize);
    let heap = scratch(a, b, 0x8a90);
    // MsBattleExe's heap (round 21): its own block for the 0x1e270 bytes of
    // actors MsBtlChrGetMem asks for. Handing it `heap` (0x8a90) let
    // MsBtlChrInit write past its end over the scratch tables after it
    // (the fake g_KernelPlyRom first), and a later MsEffectStart divided by
    // a cleared ply_rom byte.
    let actor_heap = scratch(a, b, 0x1e270);
    // MsBtlReadManage's stub records and data (round 21)
    let rmrec = scratch(a, b, (EXCHANGE_REC * READ_MANAGE_RECS) as usize);
    let rmdata = scratch(a, b, 0x100);
    let mut r21 = Xorshift(0x21e5_0001 ^ u32::from(synthetic));
    // MsDebugFullCommand's ply_data block (round 28)
    let dfc = scratch(a, b, 0x200);
    let mut r28 = Xorshift(0x28df_0001 ^ u32::from(synthetic));
    let iheap = scratch(a, b, 0x1000);
    let idata = scratch(a, b, 0x300);
    let sblock = scratch(a, b, 0x1000);
    let sdata = scratch(a, b, 0x40);
    let mut save_rng = Xorshift(0x21e5_0201 ^ u32::from(synthetic));
    let battle_init_rec = scratch(a, b, (EXCHANGE_REC * BATTLE_INIT_RECS) as usize);
    let battle_init_dest = scratch(a, b, 0x1000);
    let battle_init_data = scratch(a, b, 0x80);
    let mut binit_rng = Xorshift(0x21e5_0301 ^ u32::from(synthetic));
    let mut process_rng = Xorshift(0x21e5_0401 ^ u32::from(synthetic));
    let monster_tables = scratch(a, b, 0xc000);
    let mut monster_rng = Xorshift(0x21e5_0501 ^ u32::from(synthetic));
    // MsBtlReadSetScene's records and file (round 22)
    let scene_rec = scratch(a, b, (EXCHANGE_REC * 7) as usize);
    let scene_file = scratch(a, b, 0x200);
    let mut scene_rng = Xorshift(0x22e5_0101 ^ u32::from(synthetic));
    let mut init_rng = Xorshift(0x21e5_0101 ^ u32::from(synthetic));
    // MsSetMove and MsGetTargetDist (round 31)
    let mut r31 = Xorshift(0x31e5_0001 ^ u32::from(synthetic));
    // the op_et_eff wrappers (round 31)
    let mut et_rng = Xorshift(0x31e5_0101 ^ u32::from(synthetic));
    // the effect signals and motions (round 32): their frame words
    let mut sig_rng = Xorshift(0x32e5_0001 ^ u32::from(synthetic));
    let sig_local = scratch(a, b, 0x10);
    let sig_rec = scratch(a, b, 0x100);
    // the effect and motion reads (round 32): stub records, the CD
    // object and its methods' records, the effect file's block
    let mut cd_rng = Xorshift(0x32e5_0101 ^ u32::from(synthetic));
    let cd_rec = scratch(a, b, 0x400);
    let cd_block = scratch(a, b, 0x5_1000);
    for o in [&mut *a, &mut *b] {
        logging_stub(
            o,
            ffx_battle::free::bridge::CH_MODEL_CREATE_PACKET,
            SPECIAL_STUBS,
            crec,
            1,
            0,
        );
        for (k, ((f, spec), rec)) in (1u32..).zip(SPECIAL_CALLEES.iter().zip(&srecs)) {
            let ret = if *f == ffx_battle::special::bridge::YN_PSIM_GET_HANDLE {
                0x1234
            } else {
                0
            };
            copying_stub(o, *f, SPECIAL_STUBS + 0x80 * k, *rec, spec, ret);
        }
    }
    bridge_stubs(a, b, btables, brec);
    for _ in 0..sel.iters {
        file(a, b, rng, &f);
        dress(a, b, rng, &w, &f);
        if sel.on("calls") {
            calls(a, b, rng, &w, out, !synthetic);
        }
        // on the dressed actors (status and actions change their fields)
        if sel.on("near") {
            near(a, b, rng, &w, out);
        }
        if sel.on("status") {
            status(a, b, rng, &w, out);
        }
        if sel.on("actions") {
            actions(a, b, rng, &w, out);
        }
        if sel.on("exchange") {
            exchange(a, b, rng, &w, out, xrec);
        }
        if sel.on("effects") {
            effects(a, b, rng, &w, erec, files);
        }
        if sel.on("parse") {
            for _ in 0..3 {
                parse(a, b, rng, &w, out, srec + 2 * EXCHANGE_REC);
            }
        }
        if sel.on("command_cue") {
            for _ in 0..3 {
                command_cue(a, b, rng, &w, out, srec);
            }
        }
        if sel.on("process") {
            process(a, b, rng, &w, prec, srec, out);
        }
        if sel.on("deaths") {
            deaths(a, b, rng, &w, out);
        }
        if sel.on("input") {
            input(a, b, &w, out, irec);
        }
        if sel.on("damage_end") {
            damage_end(a, b, rng, &w, out);
        }
        // revive sets every actor up again and fakes the monster tables:
        // only its own part test runs it
        if !sel.only.is_empty() && sel.on("revive") {
            revive(a, b, &mut r34, &w, vrec);
        }
        if sel.on("models_freed") {
            models_freed(a, b, &mut r18m, &w, crec);
        }
        if sel.on("battle_end") {
            battle_end(a, b, rng, &w, out, [nrec, erec, prec, srec], files);
        }
        if sel.on("battle_frame") {
            battle_frame(a, b, rng, &w, out, grec);
        }
        if sel.on("effect_start") {
            effect_start(a, b, &mut r15, &w, out, effrec, [prec, srec, erec], cam);
        }
        if sel.on("save_params") {
            save_params(a, b, &mut r15, &w, out, effrec, xrec);
        }
        if sel.on("messages_and_cameras") {
            messages_and_cameras(a, b, &mut r15, mrec, !synthetic);
        }
        if sel.on("moves") {
            moves(a, b, &mut r16, &w, out);
        }
        let whole = r17.below(2) == 0;
        if sel.on("move_steps") {
            move_steps(a, b, &mut r17, &w, &f, whole);
        }
        if sel.on("ball_splines") {
            ball_splines(a, b, &mut r17, &w, &f, out);
        }
        if sel.on("motions") {
            motions(a, b, &mut r18m, &w, &f, work);
        }
        if sel.pick(r19.below(4) == 0, "atel_update0") {
            atel_update0(a, b, &mut r19, &w, work, locals);
        }
        if sel.on("special_stage") {
            special_stage(a, b, &mut r19, &w, work, locals);
        }
        if sel.pick(r34.below(4) == 0, "bridge_to_battle") {
            bridge_to_battle(a, b, &mut r34, &w, &f, btables, brec, belems);
        }
        if sel.pick(r19.below(6) == 0, "bridge_to_event") {
            bridge_to_event(a, b, &mut r19, &w, &f, btables, brec);
        }
        if sel.pick(r19.below(4) == 0, "setup_leaves") {
            setup_leaves(a, b, &mut r19, grec2, heap, out);
        }
        if sel.pick(r19.below(4) == 0, "exe_leaves") {
            exe_leaves(a, b, &mut r19, erec2, actor_heap, out);
        }
        if sel.pick(r20.below(4) == 0, "read_leaves") {
            read_leaves(a, b, &mut r20, &w, erec2, out);
        }
        if sel.pick(r21.below(2) == 0, "read_manage") {
            read_manage(a, b, &mut r21, &w, rmrec, effrec, rmdata, out);
        }
        if sel.pick(r28.below(4) == 0, "debug_full_command") {
            debug_full_command(a, b, &mut r28, rmrec, dfc, synthetic);
        }
        // snapshots only: the weapon's name reads kernel tables the image
        // lacks
        if !synthetic && sel.pick(r28.below(3) == 0, "weapon_plate") {
            weapon_plate(a, b, &mut r28, rmrec, dfc);
        }
        if sel.pick(init_rng.below(3) == 0, "init_leaves") {
            init_leaves(a, b, &mut init_rng, rmrec, iheap, idata);
        }
        if sel.pick(save_rng.below(4) == 0, "save_start") {
            save_start(a, b, &mut save_rng, rmrec, sblock, iheap, sdata);
        }
        if sel.pick(binit_rng.below(4) == 0, "battle_init") {
            battle_init(
                a,
                b,
                &mut binit_rng,
                battle_init_rec,
                [iheap, sblock, battle_init_dest, battle_init_data],
            );
        }
        if sel.pick(process_rng.below(2) == 0, "battle_process") {
            battle_process(
                a,
                b,
                &mut process_rng,
                &w,
                battle_init_rec,
                battle_init_data + 0x60,
            );
        }
        if sel.pick(scene_rng.below(3) == 0, "read_set_scene") {
            read_set_scene(a, b, &mut scene_rng, scene_rec, scene_file);
        }
        if sel.pick(monster_rng.below(3) == 0, "read_monster") {
            read_monster(
                a,
                b,
                &mut monster_rng,
                battle_init_rec,
                monster_tables,
                battle_init_dest,
            );
        }
        if sel.on("target_dist") {
            target_dist(a, b, &mut r31, &w, &f);
        }
        if sel.on("et_effects") {
            for _ in 0..4 {
                et_effects(a, b, &mut et_rng, rec);
            }
        }
        if sel.on("cd_reads") {
            cd_reads(a, b, &mut cd_rng, [sig_rec, cd_rec, cd_block]);
        }
        if sel.on("effect_signals") {
            for _ in 0..4 {
                effect_signals(a, b, &mut sig_rng, rec, [sig_local, sig_rec]);
            }
        }
    }
    // the teardown last (it clears the actors and the script managers)
    if sel.on("teardown") {
        let mut r18 = Xorshift(0x18e5_0001 ^ u32::from(synthetic));
        for _ in 0..sel.teardowns {
            teardown(a, b, &mut r18, work);
        }
    }
}

/// The battle's teardown leaves (`ffx_battle::free`): the heap,
/// `Sg_FreeA` and `op_et_battle_effect_free` are the logging stubs
/// ([`PROCESS_CALLEES`], [`FREE_CALLEES`]); `AtelRemoveCtrlWork` and
/// `Yn_SeaResetMatrix` run as the originals (the latter in both).
fn teardown(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, work: u32) {
    use ffx_battle::free::bridge as fr;
    let mut call = game_call;
    let ptr = |rng: &mut Xorshift| [0, rng.next() | 1, rng.next()][rng.below(3) as usize];
    for g in [
        ffx_battle::actor::bridge::CHR_TOP,
        ffx_battle::actor::bridge::MON_TOP,
        ffx_battle::actor::bridge::WEAPON_TOP,
        ffx_battle::misc::bridge::NUMBERS,
        ffx_battle::misc::bridge::EFFECT_BUFFER,
        ffx_battle::misc::bridge::ITEM_MEM,
        ffx_battle::misc::bridge::ITEM_MEM2,
        ffx_battle::misc::bridge::ET_EFFECT_READ,
        ffx_battle::parse::bridge::EFFECT_READ_STAT,
        fr::FIELD_ITEM,
        fr::FIELD_ITEM2,
    ] {
        let v = ptr(rng);
        set(a, b, g, v);
    }
    let v = ptr(rng).cast_signed();
    set(a, b, ffx_battle::state::bridge::MEM_CHECK2, v);
    let m = ffx_battle::minor::bridge::ATEL_MANAGERS.ptr(b).addr;
    for o in [4, 0xc, 0x58, 0x60] {
        poke(a, b, m + o, &ptr(rng).to_le_bytes());
    }
    set(
        a,
        b,
        ffx_battle::minor::bridge::ATEL_CAM_ON,
        u8::from(rng.below(2) == 0),
    );
    let records = ffx_battle::atel::bridge::SCRIPT_RECORDS.ptr(b).addr;
    for k in 0..4 {
        poke(a, b, records + 0x20 * k + 4, &ptr(rng).to_le_bytes());
    }
    let text = ffx_battle::flow::bridge::MENU_TEXT.ptr(b).addr;
    for k in 0..17 {
        poke(a, b, text + 12 * k, &[u8::from(rng.below(3) == 0)]);
        poke(a, b, text + 12 * k + 8, &ptr(rng).to_le_bytes());
    }
    // the special stage: its work's up vector pointer is null or a word of
    // the work itself
    let kind = [0i8, 1, 2, 3, 4, -1][rng.below(6) as usize];
    set(a, b, fr::SPECIAL_KIND, kind);
    let up = [0, work + 0x40][rng.below(2) as usize];
    poke(a, b, work + 0x810, &up.to_le_bytes());
    let w = if (1..=3).contains(&kind) || rng.below(2) == 0 {
        work
    } else {
        0
    };
    set(a, b, fr::SPECIAL_MEM, w);
    // screen 0's shake and the sea's matrix away from what the undoing
    // writes
    let screen = ffx_battle::camera::bridge::SCREENS.ptr(b).addr;
    poke(a, b, screen + 0x16c, &[1]);
    poke(a, b, screen + 0x30, &floats(rng, 1));
    let sea = ffx_field::guide::SEA_MATRIX.ptr(b).addr;
    poke(a, b, sea, &floats(rng, 16));
    for (k, (name, sym)) in (0..).zip([
        ("MsBtlChrFreeMem", 0x0079_3500),
        ("MsNumberFreeMem", 0x0079_f990),
        ("MsEtEffectStop", 0x0079_e7b0),
        ("MsEffectEnvFree", 0x0078_7a40),
        ("MsEffectBufferFree_00782750", 0x0078_2750),
        ("MsBattleSpecialFree", 0x0079_dde0),
        ("MsBtlFreeItemMem", 0x0079_9f70),
        ("MsMemCheckFree", 0x0079_9ec0),
        ("MsFieldItemFree", 0x0079_8f90),
    ]) {
        twin(name, a, b, sym, &[], 0, |m| {
            dm(m, |m| match k {
                0 => fr::ms_btl_chr_free_mem(m, &mut call),
                1 => fr::ms_number_free_mem(m, &mut call),
                2 => fr::ms_et_effect_stop(m, &mut call),
                3 => fr::ms_effect_env_free(m, &mut call),
                4 => fr::ms_effect_buffer_free(m, &mut call),
                5 => fr::ms_battle_special_free(m, &mut call),
                6 => fr::ms_btl_free_item_mem(m, &mut call),
                7 => fr::ms_mem_check_free(m, &mut call),
                _ => fr::ms_field_item_free(m, &mut call),
            });
            0
        });
        same(name, a, b);
    }
    twin("MsAtelRegistFree", a, b, 0x0079_7a90, &[], 0, |m| {
        dm(m, |m| {
            fr::ms_atel_regist_free(m, &mut call, ffx_script::vm::bridge::atel_remove_ctrl_work);
        });
        0
    });
    same("MsAtelRegistFree", a, b);
    for _ in 0..2 {
        let i = rng.below(4);
        twin(
            "MsBtlScriptRecordFree_00782640",
            a,
            b,
            0x0078_2640,
            &[i],
            0,
            |m| {
                dm(m, |m| fr::ms_btl_script_record_free(m, &mut call, i));
                0
            },
        );
        same("MsBtlScriptRecordFree_00782640", a, b);
    }
    for _ in 0..6 {
        let i = [rng.below(17), rng.below(0x14), rng.next()][rng.below(3) as usize];
        twin("MsMenuFreeText", a, b, 0x0078_fb00, &[i], ALL, |m| {
            dm(m, |m| fr::ms_menu_free_text(m, &mut call, i)).cast_unsigned()
        });
        same(&format!("MsMenuFreeText {i:#x}"), a, b);
    }
}

/// `_CIcos` on a double as the patched thunk computes it (`x²`,
/// [`battle_common::crt`]).
fn cos_stand_in(x: f64) -> f64 {
    ffx_bridge::x87::mul(x, x)
}

/// Zeroes the top of `o`'s stack, where the next original's locals go
/// (a port's frames start zeroed).
fn zero_stack(o: &mut Oracle) {
    let r = o.stack_range();
    o.write(r.start, &vec![0; (r.end - r.start) as usize])
        .unwrap();
}

/// The move step's pieces (round 16): `MsCheckNeckMotion`,
/// `MsGetMoveTargetDist`, `MsCameraMoveCalc`, on the dressed actors.
fn moves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, out: u32) {
    use ffx_battle::camera::bridge as cam;
    use ffx_battle::predict::bridge as pr;
    let h = host();
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    let motion =
        [0x1fu8, 0x20, 0x22, 0x24, 0x25, 0x47, 0x48, 0x4c, 0x4d, 0][rng.below(10) as usize];
    poke(a, b, actor + 0x3f0, &[motion]);
    twin("MsCheckNeckMotion", a, b, 0x007a_b700, &[actor], ALL, |m| {
        pr::ms_check_neck_motion(m, actor)
    });
    // MsGetMoveTargetDist (the original's locals zeroed like the Rust
    // frame: a kind MsGetMoveTargetPos writes nothing for, the summoner's
    // without one, leaves them as they were; kind 17 left out: on the
    // field snapshot the original's VU registers 0 and 2 end up holding
    // the actor and the caller's ebx where the port leaves zeros, an open
    // question for MsGetMoveTargetPos itself; 15 reads its handle's
    // optional position, which faults on these synthetic handles)
    let kind = u32::from([1u8, 2, 3, 4, 5, 6, 8, 9, 10, 11, 13, 14, 18][rng.below(13) as usize])
        | [0, 0x40, 0x80, 0xc0][rng.below(4) as usize];
    let n = [0, 1, 7, 30, 0xffff, 0x1_0005][rng.below(6) as usize];
    let accel = rng.range(-300, 300).cast_unsigned();
    let brake = rng.range(-300, 300).cast_unsigned();
    let gravity = [0, 0x8000_0000, float(rng), float(rng)][rng.below(4) as usize];
    let args = [
        i,
        kind,
        rng.below(ACTORS),
        float(rng),
        n,
        accel,
        brake,
        float(rng),
        float(rng),
        float(rng),
        gravity,
    ];
    // kind 9's turn (MsTurnMoveVector, tested in `calls`) carries its
    // frame's garbage into the VU registers: off here
    poke(a, b, actor + 0x411, &[0]);
    zero_stack(a);
    twin("MsGetMoveTargetDist", a, b, 0x007a_b540, &args, 0, |m| {
        dm(m, |m| {
            pr::ms_get_move_target_dist(
                m,
                &h,
                args[0],
                args[1],
                args[2],
                args[3],
                args[4],
                args[5],
                args[6],
                [args[7], args[8], args[9]],
                args[10],
                Some(HARNESS.ebx),
            );
        });
        0
    });
    local_words(a, b);
    same(&format!("MsGetMoveTargetDist {args:x?}"), a, b);
    // MsCameraMoveCalc on a move block in `out`
    for _ in 0..4 {
        let mv = out + 0x40;
        let small = |rng: &mut Xorshift| u16::try_from(rng.below(12)).unwrap();
        let mut words = [
            u16::try_from(rng.below(5)).unwrap(),
            0,
            small(rng),
            small(rng),
            small(rng),
            small(rng),
            small(rng),
            small(rng),
        ];
        if rng.below(6) == 0 {
            words[2] = 0xfffe;
        }
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        poke(a, b, mv, &bytes);
        poke(a, b, out, &float(rng).to_le_bytes());
        let near = rng.below(2);
        let start = float(rng);
        let args = [near, out, start, mv];
        twin("MsCameraMoveCalc", a, b, 0x007b_da80, &args, ALL, |m| {
            cam::ms_camera_move_calc(m, &cos_stand_in, near, out, start, mv)
        });
        same("MsCameraMoveCalc", a, b);
    }
}

/// Makes every NaN word in `len` bytes at `at` a number in both oracles:
/// the calls before leave NaNs in the actors and characters, and two
/// NaNs that meet resolve in the emulator unlike on the x87 (it takes
/// `st0`'s, the x87 the larger significand, with the sign by operand
/// order), so the move steps start from numbers.
fn no_nans(a: &mut Oracle, b: &mut Oracle, at: u32, len: u32) {
    let v = a.read(at, len as usize).unwrap();
    let w: Vec<u8> = v
        .chunks(4)
        .flat_map(|c| no_nan(u32::from_le_bytes(c.try_into().unwrap())).to_le_bytes())
        .collect();
    if w != v {
        poke(a, b, at, &w);
    }
}

/// `MsGetChrPredictPos` and `MsMoveChr` (round 17) on the
/// dressed actors, every move kind.
/// `whole`: `MsMoveChr`, else `MsGetChrPredictPos` actor by actor.
/// Actors the move originals can run, on top of `dress` (the calls since
/// change them): the position file and move splines again, every actor
/// with a character at a modest position and no NaN words, valid actor
/// indexes, the dress-time battle places.
fn sane_actors(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, f: &Fakes) {
    // the position file again (kind 1's battle position: the calls since
    // `dress` replace it, and a missing table leaves words of the target
    // unwritten, stack garbage in the original)
    file(a, b, rng, f);
    // the move splines again (the calls since `dress` rewrite them):
    // three of them, 3 * 0x210 being g_BtlMoveTarget, which actor 0's
    // move writes
    let sp = t::MOVE_SPLINES.ptr(b).addr;
    for k in 0..3 {
        let at = sp + k * 0x210;
        poke(a, b, at, &(3 + rng.below(6)).to_le_bytes());
        poke(a, b, at + 0x10, &floats(rng, 0x80));
    }
    // every actor with a character (one without has its position from
    // the battle position file, which leaves words of the caller's local
    // unwritten: stack garbage in the original, zeros in the port)
    let chrs: Vec<u32> = (0..ACTORS)
        .map(|i| u32::from_le_bytes(a.read(w.actor(i), 4).unwrap().try_into().unwrap()))
        .filter(|&c| c != 0)
        .collect();
    for i in 0..ACTORS {
        let p = w.actor(i);
        no_nans(a, b, p, ACTOR);
        let mut chr = u32::from_le_bytes(a.read(p, 4).unwrap().try_into().unwrap());
        if chr == 0 && !chrs.is_empty() {
            chr = chrs[rng.below(u32::try_from(chrs.len()).unwrap()) as usize];
            poke(a, b, p, &chr.to_le_bytes());
        }
        if chr != 0 && a.read(chr, 0x1000).is_ok() {
            no_nans(a, b, chr, 0x1000);
        }
        // the fields `dress` sets for positions the file has, and actor
        // indexes (the calls since change them; an index past the actors
        // writes no target, which leaves the original's stack garbage)
        poke(a, b, p + 0x43f, &[0xff]);
        for o in [0xc, 0x14, 0x416, 0xded] {
            poke(a, b, p + o, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        }
        poke(a, b, p + 0x511, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(
            a,
            b,
            p + 0x4fc,
            &[0, 3, u8::try_from(rng.below(4)).unwrap()],
        );
        poke(a, b, p + 0x6d4, &[if rng.below(3) == 0 { 1 } else { 0xff }]);
        poke(a, b, p + 0x6dd, &[0]);
    }
    // the characters at modest positions (huge ones overflow on the way)
    // and rotations
    let mut seen = Vec::new();
    for i in 0..ACTORS {
        let c = u32::from_le_bytes(a.read(w.actor(i), 4).unwrap().try_into().unwrap());
        if !seen.contains(&c) {
            seen.push(c);
            let v: Vec<u8> = (0..3)
                .flat_map(|_| {
                    (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 37.0)
                        .to_bits()
                        .to_le_bytes()
                })
                .collect();
            poke(a, b, c + ffx_chara::layout::POS, &v);
            // a rotation Ch_GetRot's wrap ends on, as `dress` sets it
            let r = (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 97.0).to_bits();
            poke(a, b, c + 0x158, &r.to_le_bytes());
        }
    }
    // the actors another faces or moves to (+0x416, +0xded) of another
    // character: two actors of one character stand at one point, and the
    // direction between them (a zero vector) reads the w words the
    // originals leave unwritten, stack garbage after an earlier step
    let chr_of = |o: &mut Oracle, k: u32| {
        u32::from_le_bytes(o.read(w.actor(k), 4).unwrap().try_into().unwrap())
    };
    for i in 0..ACTORS {
        let own = chr_of(a, i);
        let others: Vec<u32> = (0..ACTORS).filter(|&k| chr_of(a, k) != own).collect();
        if others.is_empty() {
            continue;
        }
        for o in [0x416, 0xded] {
            let k = others[rng.below(u32::try_from(others.len()).unwrap()) as usize];
            poke(a, b, w.actor(i) + o, &[u8::try_from(k).unwrap()]);
        }
    }
}

/// `MsMoveSplineRegist` and `MsSetBallSpline` (round 17 group 2) on
/// sane actors of safe move kinds (the ball's end asks its targets' move
/// positions: kinds 0x10 and above 0x12 move towards stack garbage, 12
/// recurses, and 7 runs its spline at the frame plus `g_BtlMoveFrame`,
/// past the table's points).
fn ball_splines(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    f: &Fakes,
    out: u32,
) {
    use ffx_battle::predict::bridge as pr;
    let h = host();
    sane_actors(a, b, rng, w, f);
    let kinds = [0u8, 1, 2, 3, 6, 8, 10, 11, 13, 14];
    for i in 0..ACTORS {
        let p = w.actor(i);
        let kind = kinds[rng.below(u32::try_from(kinds.len()).unwrap()) as usize];
        poke(a, b, p + 0x415, &[kind]);
        poke(a, b, p + 0x411, &[0]);
        let len = u16::try_from(1 + rng.below(40)).unwrap();
        poke(a, b, p + 0x45a, &len.to_le_bytes());
        let fr = u16::try_from(rng.below(u32::from(len) + 1)).unwrap();
        poke(a, b, p + 0x452, &fr.to_le_bytes());
        poke(a, b, p + 0x454, &u16::from(rng.below(2) == 0).to_le_bytes());
    }
    // MsMoveSplineRegist: any spline (3 is none), any count (0x20 full)
    for _ in 0..4 {
        let k = rng.below(4);
        let counts = ffx_battle::misc::bridge::MOVE_SPLINES.ptr(b).addr;
        if k < 3 {
            poke(a, b, counts + 4 * k, &rng.range(-1, 0x22).to_le_bytes());
        }
        let i = rng.below(ACTORS);
        let kind = u32::from(kinds[rng.below(u32::try_from(kinds.len()).unwrap()) as usize]);
        poke(a, b, out, &floats(rng, 4));
        let point = if rng.below(4) == 0 { 0 } else { out };
        let args = [k, i, kind, point];
        zero_stack(a);
        twin("MsMoveSplineRegist", a, b, 0x007a_b0b0, &args, 0, |m| {
            dm(m, |m| {
                pr::ms_move_spline_regist(m, &h, k, i, kind, point, Some(HARNESS.ebx));
            });
            0
        });
        local_words(a, b);
        same(&format!("MsMoveSplineRegist {args:x?}"), a, b);
        // MsMoveSpline (round 32) on the same spline and actor: speeds now
        // and then held (+0x41b); the actor's move (`+0x400..+0x4a0`) and
        // the spline's first word put back after (a spline move, kind 7,
        // with any frame count is no state the later steps take)
        let p = w.actor(i);
        let moved = a.read(p + 0x400, 0xa0).unwrap();
        let data = ffx_battle::target::bridge::MOVE_SPLINES.ptr(b).addr + 0x210 * k;
        let first = a.read(data, 4).unwrap();
        poke(a, b, p + 0x41b, &[u8::from(rng.below(3) == 0)]);
        let args = [k, i, rng.next(), float(rng), rng.next()];
        twin("MsMoveSpline", a, b, 0x007a_b010, &args, 0, |m| {
            pr::ms_move_spline(m, k, i, args[2], args[3], args[4]);
            0
        });
        same(&format!("MsMoveSpline {args:x?}"), a, b);
        poke(a, b, p + 0x400, &moved);
        poke(a, b, data, &first);
    }
    // MsSetBallSpline: thrower i, catcher +0x14, the ball state +0x420
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    let t = rng.below(ACTORS);
    poke(a, b, p + 0x14, &[u8::try_from(t).unwrap()]);
    let state = [0u8, 0, 1, 2, 3, 4, 0xff][rng.below(7) as usize];
    poke(a, b, p + 0x420, &[state]);
    let q = w.actor(t);
    let first = [t, rng.below(ACTORS), t][rng.below(3) as usize];
    let targets = match rng.below(5) {
        0 => 0,
        1 => rng.next() & 0x7fff_ffff,
        _ => 1 << first,
    };
    poke(a, b, q + 0xdc0, &targets.to_le_bytes());
    // the catcher's result on its first target: its own command (+0x72c)
    // and a result entry in that target's first buffer (+0x774), +0x18 of
    // 0, 1 (the end moved on) or -1
    poke(a, b, q + 0xde5, &[0xff]);
    let id = a.read(q + 0x72c + 2, 1).unwrap()[0];
    let buf = w.actor(first) + 0x774;
    let r = [0u8, 1, 1, 0xff][rng.below(4) as usize];
    poke(a, b, buf, &[0, 1, u8::try_from(t).unwrap(), id]);
    poke(a, b, buf + 0x18, &[r]);
    poke(a, b, w.actor(first) + 0xa4c + 2, &[0xff]);
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        i8::from(rng.below(3) == 0),
    );
    poke(a, b, pr::BALL_OFFSET.ptr(b).addr, &floats(rng, 4));
    zero_stack(a);
    twin("MsSetBallSpline", a, b, 0x007b_78c0, &[i], 0, |m| {
        dm(m, |m| pr::ms_set_ball_spline(m, &h, i, Some(HARNESS)));
        0
    });
    local_words(a, b);
    same(&format!("MsSetBallSpline {i} state {state} to {t}"), a, b);
}

fn move_steps(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    f: &Fakes,
    whole: bool,
) {
    use ffx_battle::predict::bridge as pr;
    let h = host();
    let kinds = [0u8, 7, 7, 0x10, 1, 2, 3, 6, 8, 9, 10, 11, 13, 14];
    sane_actors(a, b, rng, w, f);
    for i in 0..ACTORS {
        let p = w.actor(i);
        let kind = kinds[rng.below(u32::try_from(kinds.len()).unwrap()) as usize]
            | if rng.below(3) == 0 { 0x80 } else { 0 };
        poke(a, b, p + 0x415, &[kind]);
        // a length of at least 1 (0 makes 0/0 NaNs that meet, which the
        // emulator resolves unlike the x87)
        let len = u16::try_from(1 + rng.below(12)).unwrap();
        poke(a, b, p + 0x45a, &len.to_le_bytes());
        // the frame within the move (a spline move past its length reads
        // spline points past the table, wild in the original; steering
        // moves run to twice the length, then restart)
        let fr = if kind & 0x7f == 7 {
            rng.below(u32::from(len))
        } else {
            rng.below(2 * u32::from(len) + 2)
        };
        poke(a, b, p + 0x452, &u16::try_from(fr).unwrap().to_le_bytes());
        // spline 2 is the ball's, rebuilt each step (MsSetBallSpline): here
        // in its states that need no target positions (2, 3: the catcher's
        // optional position; above 3: none; `ball_splines` has 0 and 1)
        let sp = u16::try_from(rng.below(3)).unwrap();
        poke(a, b, p + 0x454, &sp.to_le_bytes());
        if sp == 2 {
            poke(a, b, p + 0x420, &[[2u8, 3, 4][rng.below(3) as usize]]);
        }
        // the direction modes as `status` sets them (MsGetDirMode), but
        // for 2, 7 and 9 (the targets' centre: none makes the indefinite
        // NaN, which the C runtime's stand-in meets in the emulator unlike
        // on the x87; `status` has them)
        // on the whole frame, mode 8 (towards the move's target, the
        // step's local) only for kinds that write one: 0, 7 and 0x10
        // leave it as it was, after an earlier actor's step stack garbage
        // in the original
        let mut mode = [0u8, 1, 3, 4, 5, 6, 8, 10, 11, 12, 13, 14][rng.below(12) as usize];
        if whole && mode == 8 && matches!(kind & 0x7f, 0 | 7 | 0x10) {
            mode = 3;
        }
        poke(a, b, p + 0x42b, &[mode]);
        let dt = if rng.below(2) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(ACTORS)).unwrap()
        };
        poke(a, b, p + 0x431, &[dt]);
        for o in [0x42c, 0x42d, 0x41b] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
        // angles Ch_SetDirF's wrap ends on (a huge one loops for ever)
        let angle = |rng: &mut Xorshift| {
            (f32::from(i16::try_from(rng.range(-3000, 3000)).unwrap()) / 97.0).to_bits()
        };
        for o in [0x4f4, 0x490, 0x470, 0x47c, 0x46c] {
            poke(a, b, p + o, &angle(rng).to_le_bytes());
        }
        // kind 9's turn and kind 17: see `moves`
        poke(a, b, p + 0x411, &[0]);
        poke(a, b, p + 0x419, &[u8::from(rng.below(3) == 0)]);
        poke(a, b, p + 0x42a, &[[0u8, 1, 0xff][rng.below(3) as usize]]);
        poke(a, b, p + 0x42e, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(a, b, p + 0x42f, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(a, b, p + 0x430, &[u8::try_from(rng.below(4)).unwrap()]);
        poke(a, b, p + 0x439, &[u8::try_from(rng.below(3)).unwrap()]);
        let t = if rng.below(3) == 0 {
            0xff
        } else {
            u8::try_from(rng.below(ACTORS)).unwrap()
        };
        poke(a, b, p + 0x438, &[t]);
        poke(
            a,
            b,
            p + 0x3f0,
            &[[0u8, 0x22, 0x4a, 0x30][rng.below(4) as usize]],
        );
        for o in [0x10, 0x12] {
            poke(a, b, p + o, &[u8::from(rng.below(2) == 0)]);
        }
        for o in [0x460, 0x464, 0x468, 0x474, 0x478, 0x480, 0x484, 0x4a8] {
            poke(a, b, p + o, &float(rng).to_le_bytes());
        }
    }
    if whole {
        zero_stack(a);
        twin("MsMoveChr", a, b, 0x007a_ab80, &[], 0, |m| {
            dm(m, |m| pr::ms_move_chr(m, &h));
            0
        });
        local_words(a, b);
        same("MsMoveChr", a, b);
        return;
    }
    for i in 0..ACTORS {
        let k: u8 = b.read(w.actor(i) + 0x415, 1).unwrap()[0];
        zero_stack(a);
        twin("MsGetChrPredictPos", a, b, 0x007a_a2a0, &[i], 0, |m| {
            dm(m, |m| pr::ms_get_chr_predict_pos(m, &h, i, Some(HARNESS)));
            0
        });
        local_words(a, b);
        same(&format!("MsGetChrPredictPos {i} kind {k:#x}"), a, b);
    }
}

/// `MsGetTargetDist` and `MsSetMove` (round 31) on the dressed actors:
/// every move kind and `MsGetChrInfo` property, the original's locals
/// zeroed like the Rust frames (a kind `MsGetMoveTargetPos` writes
/// nothing for leaves them as they were). Kind 15 reads its handle's
/// optional position, which faults on the synthetic handles; kind 17 is
/// left out as in [`moves`].
fn target_dist(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, f: &Fakes) {
    use ffx_battle::predict::bridge as pr;
    let h = host();
    if rng.below(2) == 0 {
        sane_actors(a, b, rng, w, f);
    }
    let i = if rng.below(8) == 0 {
        // an index MsBtlChrNumCheck refuses: no position
        [ACTORS, 0xff, 0x100 + rng.below(ACTORS)][rng.below(3) as usize]
    } else {
        rng.below(ACTORS)
    };
    let kind = [0u32, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 16, 18, 19]
        [rng.below(17) as usize]
        | [0, 0x40, 0x80, 0xc0][rng.below(4) as usize];
    let flat = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
    // kind 9's turn (MsTurnMoveVector) carries its frame's garbage into
    // the VU registers: off here, as in `moves`
    if i < ACTORS {
        poke(a, b, w.actor(i) + 0x411, &[0]);
    }
    // widths (the model's +0x4e4 times its x scale +0x3a8) whose sums
    // an f32 cannot hold
    for k in 0..ACTORS {
        let p = w.actor(k);
        for o in [0x4e4, 0x3a8] {
            let x = f32::from(i16::try_from(rng.range(1, 3000)).unwrap()) / 37.0;
            poke(a, b, p + o, &x.to_bits().to_le_bytes());
        }
    }
    let args = [i, kind, flat];
    zero_stack(a);
    ftwin("MsGetTargetDist", a, b, 0x007a_9a40, &args, |m| {
        dm(m, |m| {
            pr::ms_get_target_dist(m, &h, i, kind, flat, Some(HARNESS))
        })
    });
    local_words(a, b);
    same(&format!("MsGetTargetDist {args:x?}"), a, b);
    let info = rng.below(15);
    let args = [i, info];
    zero_stack(a);
    ftwin("MsSetMove", a, b, 0x007a_9fc0, &args, |m| {
        dm(m, |m| pr::ms_set_move(m, &h, i, info))
    });
    local_words(a, b);
    same(&format!("MsSetMove {args:x?}"), a, b);
    target_lists(a, b, rng, w, i);
}

/// The target lists (round 32): who stands on whom (`+0x43f`, mostly
/// nobody; `+0x440` carries some) for `MsGetChrLinkSet_007b7440` and
/// `MsGetTargetLinkList`, then `MsGetTargetSortList` (its `MsGetChrInfo`
/// the Rust with the original's `edi` and `ebx`); the links put back
/// after. The ports' frame words (`local`) start as the original's stack
/// (zeroed) and are dropped after.
fn target_lists(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, i: u32) {
    use ffx_battle::minor::bridge as mi;
    let h = host();
    let kept: Vec<Vec<u8>> = (0..ACTORS)
        .map(|k| a.read(w.actor(k) + 0x43f, 2).unwrap())
        .collect();
    for k in 0..ACTORS {
        // (now and then a record index past the actors, for the links'
        // whole-byte compare)
        let under = match rng.below(6) {
            0 | 1 => u8::try_from(rng.below(ACTORS + 2)).unwrap(),
            2 => u8::try_from(0x20 + rng.below(0x20)).unwrap(),
            _ => 0xff,
        };
        poke(
            a,
            b,
            w.actor(k) + 0x43f,
            &[under, u8::from(rng.below(2) == 0)],
        );
    }
    // actor i's targets (`+0xdc0`, the list without a running command)
    let bits = rng.next() & ((1u32 << ACTORS) - 1);
    poke(a, b, w.actor(i % ACTORS) + 0xdc0, &bits.to_le_bytes());
    if rng.below(4) != 0 {
        // (no running command: its cue entry's targets would win)
        poke(a, b, w.actor(i % ACTORS) + 0xde5, &[0xff]);
    }
    let local = LISTS.with(std::cell::Cell::get);
    let out = local + 0x40;
    let pick = [0, out][rng.below(4).min(1) as usize];
    let set = [0, rng.next(), 1 << rng.below(32)][rng.below(3) as usize];
    let j = [i, rng.below(ACTORS), 0x20 + rng.below(0x20)][rng.below(3) as usize];
    // a record past the actors carries some (put back after)
    let carry = ffx_battle::actor::bridge::ms_get_chr(b, j) + 0x440;
    let carried = a.read(carry, 1).unwrap();
    if j >= ACTORS {
        poke(a, b, carry, &[1]);
    }
    twin(
        "MsGetChrLinkSet_007b7440",
        a,
        b,
        0x007b_7440,
        &[j, set],
        ALL,
        |m| mi::ms_get_chr_link_set(m, j, set),
    );
    poke(a, b, carry, &carried);
    poke(a, b, local, &[0; 0x34]);
    zero_stack(a);
    twin(
        "MsGetTargetLinkList",
        a,
        b,
        0x007b_6fb0,
        &[i, pick],
        ALL,
        |m| mi::ms_get_target_link_list(m, i, pick, local),
    );
    let v = a.read(local, 0x34).unwrap();
    b.write(local, &v).unwrap();
    same(&format!("MsGetTargetLinkList {i:#x}"), a, b);
    let flip = [0, 1, rng.next()][rng.below(3) as usize];
    let args = [i, pick, flip];
    // now and then an actor's model at x NaN (one payload: the NaNs that
    // meet are the same), for the sort's unordered compares
    let nan_at = {
        // (one of i's targets, else any actor)
        let k = (0..ACTORS)
            .cycle()
            .skip(rng.below(ACTORS) as usize)
            .take(ACTORS as usize)
            .find(|&k| bits >> k & 1 != 0)
            .unwrap_or(0);
        let chr = u32::from_le_bytes(a.read(w.actor(k), 4).unwrap().try_into().unwrap());
        let at = chr.wrapping_add(ffx_chara::layout::POS);
        (rng.below(3) == 0 && readable(a, at, 4)).then(|| (at, a.read(at, 4).unwrap()))
    };
    if let Some((at, _)) = &nan_at {
        poke(a, b, *at, &0x7fc0_0000u32.to_le_bytes());
    }
    zero_stack(a);
    twin("MsGetTargetSortList", a, b, 0x007b_72c0, &args, ALL, |m| {
        dm(m, |m| {
            let mut info = |m: &mut t::Dm<'_>, x: [u32; 4]| {
                t::ms_get_chr_info(m, &h, x[0], x[1], x[2], x[3], Some([i & 0xff, HARNESS.ebx]))
            };
            mi::ms_get_target_sort_list(m, &stand_in, &mut info, args, local)
        })
    });
    let v = a.read(local, 0x34).unwrap();
    b.write(local, &v).unwrap();
    local_words(a, b);
    same(&format!("MsGetTargetSortList {args:x?}"), a, b);
    if let Some((at, v)) = nan_at {
        poke(a, b, at, &v);
    }
    for (k, v) in (0..ACTORS).zip(&kept) {
        poke(a, b, w.actor(k) + 0x43f, v);
    }
}

/// The `op_et_eff` wrappers (round 31, `ffx_battle::et_effect`) on the
/// dressed actors: `op_et_eff` is the recording stub (its last call's
/// arguments compared), its result at `rec + 0x34` positive, zero or
/// negative at random; the sounds run `MsRegSEplay` (the port in the
/// Rust, as the shims do).
fn et_effects(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32) {
    use ffx_battle::after_damage::bridge as ad;
    use ffx_battle::et_effect::bridge as et;
    // indices MsBtlChrNumCheck refuses: the spare actor (0xff) and the
    // records from 0x1f, whose fields the actor's offsets reach as well
    let i = if rng.below(5) == 0 {
        [
            0xff,
            0x1ff,
            ACTORS + rng.below(0x3e),
            0x100 + rng.below(ACTORS),
        ][rng.below(4) as usize]
    } else {
        rng.below(ACTORS)
    };
    let p = ffx_battle::actor::bridge::ms_get_chr(b, i);
    let pick =
        |rng: &mut Xorshift, v: &[u32]| v[rng.below(u32::try_from(v.len()).unwrap()) as usize];
    let result = pick(rng, &[1, 0x4000_0123, 0, 0x8000_0000, u32::MAX]);
    poke(a, b, rec + 0x34, &result.to_le_bytes());
    for o in [0xe1e, 0xe22, 0xe24, 0xe28] {
        let n = u16::try_from({
            let r = rng.below(0x400);
            pick(rng, &[0, 1, 0x7f, 0xffff, r])
        })
        .unwrap();
        poke(a, b, p + o, &n.to_le_bytes());
    }
    poke(a, b, p + 0x4c, &rng.next().to_le_bytes());
    poke(
        a,
        b,
        p + 0xe12,
        &[u8::try_from(rng.below(0x100)).unwrap(); 2],
    );
    let flags = {
        let r = rng.below(0x1_0000);
        pick(rng, &[0, 0x40, 0x2000, 0x2040, r])
    };
    poke(
        a,
        b,
        p + 0x616,
        &u16::try_from(flags).unwrap().to_le_bytes(),
    );
    poke(a, b, p + 0x5d9, &[u8::try_from(rng.below(0x100)).unwrap()]);
    poke(a, b, p + 0xdcd, &[u8::from(rng.below(4) == 0)]);
    for s in 0..4 {
        let v = if rng.below(2) == 0 { 0 } else { rng.next() };
        poke(a, b, p + 0xe90 + 4 * s, &v.to_le_bytes());
    }
    let ea0 = if rng.below(5) == 0 { rng.next() } else { 0 };
    poke(a, b, p + 0xea0, &ea0.to_le_bytes());
    for (o, n) in [(0xe0f, 5), (0x505, 3), (0x5c0, 4)] {
        let on = rng.below(n) == 0;
        poke(a, b, p + o, &[u8::from(if o == 0xe0f { !on } else { on })]);
    }
    for o in [0xe10, 0xe11] {
        let e = u8::try_from({
            let r = rng.below(0x100);
            pick(
                rng,
                &[
                    0, 0, 0x1e, 0x1f, 0x20, 0x21, 0x34, 0x36, 0x37, 0x38, 0x39, 0x1d, r,
                ],
            )
        })
        .unwrap();
        poke(a, b, p + o, &[e]);
    }
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        i8::from(rng.below(3) == 0),
    );
    let summoner = i8::try_from(pick(rng, &[0, 1, 5, ACTORS - 1])).unwrap();
    set(a, b, ffx_battle::target::bridge::SUMMONER, summoner);
    set(a, b, et::DEBUG_EFFECT_NO_OWNER, u8::from(rng.below(2) == 0));
    set(a, b, ffx_battle::status::bridge::EFFECT_OWNER, rng.next());
    let group = {
        let r = rng.next();
        pick(rng, &[0, 1, r])
    };
    let effect = rng.below(0x100);
    let mut reg_se = |m: &mut t::Dm<'_>, i: u32, se: u32| {
        ad::ms_reg_se_play(m, &stand_in, &mut game_call, i, se)
    };
    match rng.below(10) {
        0 => {
            let kind = {
                let r = rng.next();
                pick(rng, &[1, 0, 8, r])
            };
            let global = rng.below(2);
            let args = [i, group, kind, effect, global];
            twin(
                "MsSetChrEffect_0079eb10",
                a,
                b,
                0x0079_eb10,
                &args,
                ALL,
                |m| {
                    dm(m, |m| {
                        et::ms_set_chr_effect(m, &mut game_call, i, group, kind, effect, global)
                    })
                },
            );
        }
        1 => twin(
            "FUN_0079eba0",
            a,
            b,
            0x0079_eba0,
            &[i, group, effect],
            ALL,
            |m| {
                dm(m, |m| {
                    et::ms_set_chr_effect8(m, &mut game_call, i, group, effect)
                })
            },
        ),
        2 => twin(
            "FUN_0079ec40",
            a,
            b,
            0x0079_ec40,
            &[i, group, effect],
            ALL,
            |m| {
                dm(m, |m| {
                    et::ms_set_owner_effect8(m, &mut game_call, i, group, effect)
                })
            },
        ),
        3 => twin(
            "FUN_0079eca0",
            a,
            b,
            0x0079_eca0,
            &[i, group, effect],
            ALL,
            |m| {
                dm(m, |m| {
                    et::ms_set_owner_effect7(m, &mut game_call, i, group, effect)
                })
            },
        ),
        4 => twin("MsEtCriticalEffect", a, b, 0x0079_e590, &[i], ALL, |m| {
            dm(m, |m| et::ms_et_critical_effect(m, &mut game_call, i))
        }),
        5 => twin("MsEffectSetGuard", a, b, 0x0079_e510, &[i], ALL, |m| {
            dm(m, |m| {
                et::ms_effect_set_guard(m, &mut game_call, &mut reg_se, i)
            })
        }),
        6 => twin(
            "MsSetWeaponEffectAttribute",
            a,
            b,
            0x0079_ed40,
            &[i, p],
            ALL,
            |m| {
                dm(m, |m| {
                    et::ms_set_weapon_effect_attribute(m, &mut game_call, i, p)
                })
            },
        ),
        _ => {
            let summon = u32::from(rng.below(4) == 0);
            twin(
                "MsMagicEffectStart",
                a,
                b,
                0x0079_e8c0,
                &[i, summon],
                ALL,
                |m| {
                    dm(m, |m| {
                        et::ms_magic_effect_start(m, &mut game_call, &mut reg_se, i, summon)
                    })
                },
            );
        }
    }
    same(&format!("et effect {i:#x}"), a, b);
}

/// The effect signals (round 32, `ffx_battle::effect_signal`; the effect
/// motions are in `battle_leaf`'s resets) on the dressed actors: the callees the
/// ports run through the game run as the originals, `op_et_eff` is the
/// recording stub (its result at `rec + 0x34`). The damage buffers of the
/// first target hold records of actor `i`'s command now and then, with
/// the results and flags the signals test. `local` stands for the
/// original's frame words: zeroed in both before a call (the original's
/// stack too), and the Rust's copy dropped after it. The callees the
/// ports call through the game are recording stubs during the calls
/// ([`with_stubs`]): the dispatcher's four steps, and the sound and
/// `MsSetDamage` of the damage step (whose originals want a real battle).
fn effect_signals(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    [local, stub_rec]: [u32; 2],
) {
    use ffx_battle::actor::bridge as ac;
    use ffx_battle::effect_signal::bridge as es;
    let pick =
        |rng: &mut Xorshift, v: &[u32]| v[rng.below(u32::try_from(v.len()).unwrap()) as usize];
    // indices MsBtlChrNumCheck refuses now and then (as `et_effects`)
    let i = if rng.below(6) == 0 {
        [
            0xff,
            0x1ff,
            ACTORS + rng.below(0x3e),
            0x100 + rng.below(ACTORS),
        ][rng.below(4) as usize]
    } else {
        rng.below(ACTORS)
    };
    let i8 = i & 0xff;
    let p = ac::ms_get_chr(b, i8);
    let result = pick(rng, &[1, 0x4000_0123, 0, 0x8000_0000, u32::MAX]);
    poke(a, b, rec + 0x34, &result.to_le_bytes());
    // an actor's fields (not the records past them, whose offsets these
    // reach: the weapons' models); its part and owner bytes put back after
    let inside = i8 < ACTORS;
    let part_bytes = a.read(p + 0x12, 3).unwrap();
    if inside {
        let state = u8::try_from({
            let r = rng.below(0x100);
            pick(rng, &[1, 1, 4, 4, 0, 2, r])
        })
        .unwrap();
        poke(a, b, p + 0x446, &[state]);
        // a part (+0x12) points at its owner (+0x14)
        let part = u8::from(rng.below(5) == 0);
        poke(a, b, p + 0x12, &[part]);
        poke(a, b, p + 0x14, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        for o in [0x39c, 0xe1e, 0xe22] {
            poke(a, b, p + o, &rng.next().to_le_bytes());
        }
        for o in [0x429, 0x4c] {
            poke(a, b, p + o, &[u8::try_from(rng.below(0x100)).unwrap()]);
        }
        for (o, n) in [(0xe19, 3), (0xe16, 3)] {
            poke(a, b, p + o, &[u8::from(rng.below(n) != 0)]);
        }
        let own = u16::try_from({
            let r = rng.below(0x40);
            pick(rng, &[0xffff, 0x8000, 0, 2, 0xff, 0x7fff, r])
        })
        .unwrap();
        poke(a, b, p + 0xe1a, &own.to_le_bytes());
        let effect = u8::try_from({
            let r = rng.below(0x100);
            pick(rng, &[0, 0x1e, 0x36, r])
        })
        .unwrap();
        poke(a, b, p + 0xe0e, &[effect]);
    }
    // the first target's damage buffers for i's command, now and then
    let first = ffx_battle::body::bridge::battle_first_selected_actor(b, i, 0xffff_fff0, 0, 1);
    if ac::ms_btl_chr_num_check(i8) != 0 && first >= 0 && rng.below(4) != 0 {
        let cmd = ac::ms_get_chr_command(b, i8);
        let id = b.read(cmd + 2, 1).unwrap()[0];
        let t = ac::ms_get_chr(b, first.cast_unsigned());
        let next = u8::try_from(pick(rng, &[0, 0, 1])).unwrap();
        let count = u8::try_from(pick(rng, &[1, 2])).unwrap();
        for o in [0x774u32, 0xa4c] {
            poke(a, b, t + o, &[next, count, u8::try_from(i8).unwrap(), id]);
            for k in 0..2 {
                let e = t + o + 0x2c * k;
                let r = u8::try_from(pick(
                    rng,
                    &[0, 1, 0x11, 4, 6, 2, 3, 5, 0x10, 0x12, 0xff, 0x80],
                ))
                .unwrap();
                poke(a, b, e + 0x18, &[r]);
                let flags = u16::try_from({
                    let r = rng.below(0x1_0000);
                    pick(rng, &[0, 1, 4, 8, 0x100, 0x101, 0x107, 0x108, r])
                })
                .unwrap();
                poke(a, b, e + 0x32, &flags.to_le_bytes());
                poke(a, b, e + 0x1a, &[rng.byte()]);
            }
        }
    }
    poke(a, b, local, &[0; 0x10]);
    zero_stack(a);
    let signal = if rng.below(8) == 0 {
        pick(rng, &[10, 31, 32, 33, u32::MAX, 0x8000_0001])
    } else {
        rng.below(10)
    };
    let args = [i, signal, rng.next()];
    let steps = [
        (es::MS_EFFECT_SIGNAL_HIT, 2),
        (es::MS_EFFECT_SIGNAL_AVOID, 2),
        (es::MS_EFFECT_SIGNAL_DAMAGE, 3),
        (es::MS_EFFECT_SIGNAL_MOTION, 3),
    ];
    // the actors not set up, now and then (put back after)
    let top = ffx_battle::actor::bridge::CHR_TOP;
    let kept = top.get(b);
    if rng.below(10) == 0 {
        set(a, b, top, 0);
    }
    with_stubs(a, b, &steps, stub_rec, |a, b| {
        twin(
            "MsEffectSignal_007b74d0",
            a,
            b,
            0x007b_74d0,
            &args,
            ALL,
            |m| dm(m, |m| es::ms_effect_signal(m, &mut game_call, i, signal)),
        );
    });
    same(&format!("MsEffectSignal {args:x?}"), a, b);
    set(a, b, top, kept);
    // and the damage step alone, on an actor in range
    let signal = pick(rng, &[0, 0, 9, 9, 1, 5]);
    let args = [i, p, signal];
    poke(a, b, local, &[0; 0x10]);
    zero_stack(a);
    let callees = [(es::MS_CHR_SE_PLAY, 2), (es::MS_SET_DAMAGE, 3)];
    with_stubs(a, b, &callees, stub_rec, |a, b| {
        twin(
            "MsEffectSignalDamage_007b7680",
            a,
            b,
            0x007b_7680,
            &args,
            0,
            |m| {
                dm(m, |m| {
                    es::ms_effect_signal_damage(m, &mut game_call, i, p, signal, local);
                });
                0
            },
        );
    });
    // (the frame words: the original's stack, the Rust's `local`)
    let v = a.read(local, 0x10).unwrap();
    b.write(local, &v).unwrap();
    same(&format!("effect signal {i:#x}"), a, b);
    poke(a, b, p + 0x12, &part_bytes);
}

/// `MsEtEffectRead` and `MsRegMotionRead` (round 32,
/// `ffx_battle::read`): `GetCD` (its object at `cd + 0x200`, the methods
/// `+0`, `+8`, `+0x10`, `+0x14` logging stubs from `cd`), `Sg_MallocA`
/// (the block `block`), `MsBattleError`, `op_et_battle_init` and chara's
/// motion callees are recording stubs during the calls; the file size,
/// what is loaded and the stubs' results at random.
fn cd_reads(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, [rec, cd, block]: [u32; 3]) {
    use ffx_battle::read::bridge as rd;
    let obj = cd + 0x200;
    for o in [&mut *a, &mut *b] {
        for (k, (slot, n)) in (0u32..).zip([(0u32, 4u8), (8, 1), (0x10, 1), (0x14, 1)]) {
            let at = SIGNAL_STUBS + 0x400 + 0x40 * k;
            logging_stub(o, 0, at, cd + 0x80 * k, n, 0);
            let f = o.rebase(at);
            o.write(obj + slot, &f.to_le_bytes()).unwrap();
        }
    }
    let size = {
        let r = rng.below(0x6_0000);
        [
            r,
            rd::ET_EFFECT_SIZE,
            rd::ET_EFFECT_SIZE + 1,
            0x100,
            0x8000_0000 | r,
        ][rng.below(5) as usize]
    };
    // the block holds old data (the read clears it)
    for _ in 0..4 {
        let at = block + [rng.below(0x5_0ff0) & !3, 0x5_0fec][rng.below(2) as usize];
        poke(a, b, at, &rng.next().to_le_bytes());
    }
    poke(a, b, cd + 0x80 * 3 + 4, &size.to_le_bytes());
    for k in 0..3 {
        poke(a, b, cd + 0x80 * k + 4, &rng.next().to_le_bytes());
    }
    let mut call = game_call;
    let stubs = [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (rd::SG_MALLOC_A, 5),
        (rd::MS_BATTLE_ERROR, 1),
        (rd::OP_ET_BATTLE_INIT, 1),
        (rd::CHN_SYSTEM_MGRP_LOADED, 0),
        (rd::CH_REG_MOTION_GET_EXIST_MEM, 0),
        (rd::CH_REG_MOTION_ENTRY_CACHE, 1),
    ];
    let ret = |k: u32, n: u32| rec + 0x20 * k + 4 + 4 * n;
    let loaded = [0u32, 0, 1][rng.below(3) as usize];
    let exist = [0u32, 0, 1][rng.below(3) as usize];
    let init = rng.next();
    let flag = rng.byte();
    set(a, b, ffx_battle::misc::bridge::REG_MOTION_FLAG, flag);
    let effects = rng.below(2) == 0;
    with_stubs(a, b, &stubs, rec, |a, b| {
        for (k, n, v) in [
            (0, 0, obj),
            (1, 5, block),
            (3, 1, init),
            (4, 0, loaded),
            (5, 0, exist),
        ] {
            poke(a, b, ret(k, n), &v.to_le_bytes());
        }
        if effects {
            twin("MsEtEffectRead", a, b, 0x0079_e6a0, &[], ALL, |m| {
                dm(m, |m| rd::ms_et_effect_read(m, &mut call))
            });
        } else {
            twin("MsRegMotionRead", a, b, 0x0079_ccf0, &[], ALL, |m| {
                dm(m, |m| rd::ms_reg_motion_read(m, &mut call)).cast_unsigned()
            });
        }
    });
    same(&format!("cd reads {size:#x} {loaded} {exist}"), a, b);
}

/// `FUN_007ca580` (dead, 0x10a9 bytes): room for [`with_stubs`]' code.
const SIGNAL_STUBS: u32 = 0x007c_a580;

/// Runs `f` with the functions `stubs` (ffx.sym address, argument count)
/// replaced by [`recording_stub`]s in both oracles (code at
/// [`SIGNAL_STUBS`], records at `rec`, 0x20 bytes each, results 0), and
/// puts their first bytes back after.
fn with_stubs(
    a: &mut Oracle,
    b: &mut Oracle,
    stubs: &[(u32, u8)],
    rec: u32,
    f: impl FnOnce(&mut Oracle, &mut Oracle),
) {
    let saved: Vec<Vec<u8>> = stubs
        .iter()
        .map(|&(s, _)| {
            let at = a.rebase(s);
            a.read(at, 5).unwrap()
        })
        .collect();
    for o in [&mut *a, &mut *b] {
        for (k, &(s, n)) in (0u32..).zip(stubs) {
            recording_stub(o, s, SIGNAL_STUBS + 0x40 * k, rec + 0x20 * k, n, 0);
        }
    }
    f(a, b);
    for o in [&mut *a, &mut *b] {
        for (&(s, _), v) in stubs.iter().zip(&saved) {
            let at = o.rebase(s);
            o.write(at, v).unwrap();
        }
    }
}

#[test]
fn targets_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7a46_0001), true, &Sel::ALL);
}

#[test]
fn targets_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x7a46_0002), false, &Sel::ALL);
}

/// The actors' motions each frame (`ffx_battle::motion`, round 18 group
/// 2) on sane actors: their speed fields, queued motions, stance counters
/// and the touch check's conditions at random.
fn motions(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, f: &Fakes, work: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::motion::bridge as mo;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    sane_actors(a, b, rng, w, f);
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    // the fake characters' texture animations: none, or an entry of the
    // static table
    for c in 0..3 {
        let chr = f.chars + c * 0x1000;
        poke(a, b, chr + 0x6fc, &u32::from(flag(rng, 2)).to_le_bytes());
        poke(a, b, chr + 0x700, &rng.below(4).to_le_bytes());
    }
    for i in 0..ACTORS {
        let p = w.actor(i);
        for (o, n) in [
            (0x12, 6),
            (0xdce, 6),
            (0x613, 5),
            (0x614, 2),
            (0x432, 4),
            (0x508, 2),
            (0x38, 4),
            (0x412, 4),
            (0xdf0, 6),
        ] {
            poke(a, b, p + o, &[flag(rng, n)]);
        }
        poke(a, b, p + 0x2d, &[flag(rng, 2) ^ 1]);
        poke(a, b, p + 0x415, &[[0, 0, 0, 1][rng.below(4) as usize]]);
        poke(a, b, p + 0x606, &[0, [0u8, 8][rng.below(2) as usize]]);
        poke(a, b, p + 0x5c6, &[[0xffu8, 0xff, 3][rng.below(3) as usize]]);
        poke(a, b, p + 0x3f0, &[u8::try_from(rng.below(0x40)).unwrap()]);
        poke(a, b, p + 0x404, &[u8::try_from(rng.below(3)).unwrap()]);
        let mut words = [0u16; 4];
        for w in &mut words {
            *w = [
                0,
                0x100,
                0x80,
                0xffff,
                u16::try_from(rng.below(0x400)).unwrap(),
            ][rng.below(5) as usize];
        }
        // +0x3fc the base speed, +0x400 the current, +0x402 the fixed
        poke(a, b, p + 0x3fc, &words[0].to_le_bytes());
        poke(a, b, p + 0x400, &words[1].to_le_bytes());
        poke(a, b, p + 0x402, &words[2].to_le_bytes());
        poke(a, b, p + 0x406, &[u8::try_from(rng.below(0x20)).unwrap()]);
        poke(
            a,
            b,
            p + 0x3f2,
            &[[0xffu8, 0xff, 0xff, 0, 5][rng.below(5) as usize]],
        );
        poke(a, b, p + 0x427, &[u8::try_from(rng.below(5)).unwrap()]);
        poke(a, b, p + 0x428, &[u8::try_from(rng.below(4)).unwrap()]);
    }
    set(
        a,
        b,
        ffx_battle::body::bridge::MOTION_BUSY,
        rng.next() & rng.next() & rng.next(),
    );
    set(a, b, ffx_battle::atel::bridge::CAMERA_STATE, flag(rng, 3));
    set(
        a,
        b,
        ffx_battle::state::bridge::COMMAND_CUE_COUNT,
        i8::from(flag(rng, 2) == 1),
    );
    let status = [0i8, 0x1b, 3][rng.below(3) as usize];
    set(a, b, ffx_battle::state::bridge::STATUS, status);
    let i = rng.below(ACTORS);
    let actor = w.actor(i);
    twin("MsCalcMotionSpeed", a, b, 0x0078_ac20, &[actor], ALL, |m| {
        mo::ms_calc_motion_speed(m, actor)
    });
    same("MsCalcMotionSpeed", a, b);
    zero_stack(a);
    twin(
        "MsCheckChrTouch_007ab750",
        a,
        b,
        0x007a_b750,
        &[i, actor],
        ALL,
        |m| dm(m, |m| mo::ms_check_chr_touch(m, h.trig, i, actor)).cast_unsigned(),
    );
    local_words(a, b);
    same("MsCheckChrTouch_007ab750", a, b);
    zero_stack(a);
    twin("MsMotionProcess", a, b, 0x007a_a160, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            mo::ms_motion_process(m, h.trig, &mut s);
        });
        0
    });
    local_words(a, b);
    same("MsMotionProcess", a, b);
    special_matrices(a, b, rng, w, work);
}

/// The special stage's matrices (`ffx_battle::special`): the actors' in
/// the work, and on the snapshots the sea's from a live model.
fn special_matrices(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, work: u32) {
    use ffx_battle::special::bridge as sp;
    let h = host();
    let mut call = game_call;
    for i in 0..ACTORS {
        let p = w.actor(i);
        poke(a, b, p + 0x32, &[u8::from(rng.below(5) == 0)]);
        poke(a, b, p + 0x12, &[u8::from(rng.below(5) == 0)]);
        poke(a, b, p + 0x3a0, &floats(rng, 3));
    }
    let angles = work + 0x800;
    poke(a, b, angles, &floats(rng, 4));
    let mem = if rng.below(6) == 0 { 0 } else { work };
    set(a, b, ffx_battle::free::bridge::SPECIAL_MEM, mem);
    zero_stack(a);
    twin(
        "MsSpecialChrMatrix_0079e030",
        a,
        b,
        0x0079_e030,
        &[angles, 0],
        0,
        |m| {
            dm(m, |m| sp::ms_special_chr_matrix(m, h.trig, angles));
            0
        },
    );
    rot_locals(a, b);
    local_words(a, b);
    same("MsSpecialChrMatrix_0079e030", a, b);
    let live: Vec<u32> =
        LIVE_CHRS.with(|c| c.borrow().iter().copied().filter(|&c| c != 0).collect());
    if !live.is_empty() {
        let chr = live[rng.below(u32::try_from(live.len()).unwrap()) as usize];
        let id = [5, 0xe, rng.below(0x20)][rng.below(3) as usize];
        twin(
            "MsSpecialSeaMatrix_0079df90",
            a,
            b,
            0x0079_df90,
            &[chr, id, work],
            0,
            |m| {
                dm(m, |m| {
                    sp::ms_special_sea_matrix(m, &mut call, chr, id, work);
                });
                0
            },
        );
        same("MsSpecialSeaMatrix_0079df90", a, b);
    }
}

/// The rotation rows' lanes `MsRotMatrixX` (the last one called) takes
/// from its uninitialized locals, copied from `a`.
fn rot_locals(a: &mut Oracle, b: &mut Oracle) {
    for o in [0x810u32, 0x81c, 0x820, 0x82c] {
        let at = a.rebase(0x00c8_f000 + o);
        let v = a.read(at, 4).unwrap();
        b.write(at, &v).unwrap();
    }
}

/// Code no test runs (`Ms_Gui_BattleConsoleProc`'s, a debug window), for
/// rounds 18 and 19's stubs, 0x80 bytes each.
const SPECIAL_STUBS: u32 = 0x007c_6110;

/// The map callees `MsBattleSpecial` passes its locals to, stubbed by
/// [`copying_stub`] (`true`: a pointer whose four words are kept):
/// `yiSetRotTransDObj(i, rot, pos)`, `Yn_PSimSetPos(h, k, v)`,
/// `Yn_SeaSetPos(v)`, `Yn_SeaSetAng(v)`, `Yn_SeaSetUvSpd(u, v)`,
/// `Yn_PSimGetHandle_00919f00(i)` (returns 0x1234).
const SPECIAL_CALLEES: [(u32, &[bool]); 6] = {
    use ffx_battle::special::bridge as sp;
    [
        (sp::YI_SET_ROT_TRANS_DOBJ, &[false, true, true]),
        (sp::YN_PSIM_SET_POS, &[false, false, true]),
        (sp::YN_SEA_SET_POS, &[true]),
        (sp::YN_SEA_SET_ANG, &[true]),
        (sp::YN_SEA_SET_UV_SPD, &[false, false]),
        (sp::YN_PSIM_GET_HANDLE, &[false]),
    ]
};

/// Bytes of one [`copying_stub`] record ring (8 entries after a count
/// and a result word).
fn copying_rec_len(spec: &[bool]) -> u32 {
    8 + 8 * spec.iter().map(|&p| if p { 16 } else { 4 }).sum::<u32>()
}

/// Replaces `f` by a stub at `at` that counts its calls at `rec`, returns
/// the word at `rec + 4` (`ret`), and keeps each call's arguments in a
/// ring of 8 from `rec + 8`: a word, or for a `spec` `true` the four words
/// it points at (so locals compare by value, not by address).
fn copying_stub(o: &mut Oracle, f: u32, at: u32, rec: u32, spec: &[bool], ret: u32) {
    let size: u32 = spec.iter().map(|&p| if p { 16 } else { 4 }).sum();
    let mut code = vec![0x8b, 0x0d];
    code.extend(rec.to_le_bytes());
    code.extend([0xff, 0x05]);
    code.extend(rec.to_le_bytes());
    code.extend([0x83, 0xe1, 0x07, 0x69, 0xc9]);
    code.extend(size.to_le_bytes());
    code.extend([0x81, 0xc1]);
    code.extend((rec + 8).to_le_bytes());
    let mut off = 0u32;
    for (k, &ptr) in (0u8..).zip(spec) {
        if ptr {
            code.extend([0x8b, 0x54, 0x24, 4 + 4 * k]);
            for j in 0..4u8 {
                code.extend([0x8b, 0x42, 4 * j, 0x89, 0x81]);
                code.extend(off.to_le_bytes());
                off += 4;
            }
        } else {
            code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x81]);
            code.extend(off.to_le_bytes());
            off += 4;
        }
    }
    code.push(0xa1);
    code.extend((rec + 4).to_le_bytes());
    code.push(0xc3);
    assert!(code.len() <= 0x80);
    o.write(rec, &[0; 4]).unwrap();
    o.write(rec + 4, &ret.to_le_bytes()).unwrap();
    let at = o.rebase(at);
    o.write(at, &code).unwrap();
    let entry = o.rebase(f);
    let rel = at.wrapping_sub(entry + 5);
    o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
        .unwrap();
}

/// `MsChrCreatePacket` (its `Ch_ModelCreatePacket` the stub at
/// [`SPECIAL_STUBS`], records at `crec`) and, now and then,
/// `MsBtlChrFreeAll` (chara's disposals the [`FREE_CALLEES`] stubs).
fn models_freed(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, crec: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::free::bridge as fr;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    poke(a, b, p + 0x3a, &[[0u8, 1, 0x80][rng.below(3) as usize]]);
    let model = b.read(p, 4).unwrap();
    if rng.below(3) == 0 {
        poke(a, b, p, &[0; 4]);
    }
    twin("MsChrCreatePacket", a, b, 0x0079_3d90, &[p], ALL, |m| {
        dm(m, |m| fr::ms_chr_create_packet(m, &mut call, p))
    });
    poke(a, b, p, &model);
    let _ = crec;
    same("MsChrCreatePacket", a, b);
    if rng.below(8) != 0 {
        return;
    }
    for k in 0..ACTORS {
        let q = w.actor(k);
        for o in [0x12u32, 0x21, 0x22, 0x3e, 0x11] {
            poke(a, b, q + o, &[u8::from(rng.below(3) == 0)]);
        }
    }
    let extra = [0, rng.next() | 1][rng.below(2) as usize];
    set(a, b, ffx_battle::flow::bridge::CHR_EXTRA, extra);
    twin("MsBtlChrFreeAll", a, b, 0x0079_3480, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            fr::ms_btl_chr_free_all(m, &mut s);
        });
        0
    });
    same("MsBtlChrFreeAll", a, b);
}

/// `MsBattleSpecial` (round 19) on a work of random angles and counters:
/// kinds 0-4, the boat and sea models live ones on the snapshots (none on
/// the image: their optional positions need a real model); the map
/// callees are [`SPECIAL_CALLEES`]' stubs, `locals` the port's locals
/// (cleared after, as the original's stack is not compared).
fn special_stage(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    work: u32,
    locals: u32,
) {
    use ffx_battle::special::bridge as sp;
    let h = host();
    let mut call = game_call;
    let kind = [1i8, 1, 1, 2, 3, 0, 4][rng.below(7) as usize];
    set(a, b, ffx_battle::free::bridge::SPECIAL_KIND, kind);
    set(a, b, ffx_battle::free::bridge::SPECIAL_MEM, work);
    // the stage's angles and floats, the counters small
    poke(a, b, work + 0x800, &floats(rng, 0x19));
    for (o, x) in [
        (0x850, rng.range(-3, 3)),
        (0x854, rng.range(-1, 3)),
        (0x858, rng.range(-2, 4)),
        (0x85c, rng.range(-1, 4)),
        (0x860, rng.range(1, 30)),
    ] {
        poke(a, b, work + o, &x.to_le_bytes());
    }
    set(
        a,
        b,
        sp::CURRENT_MAP,
        [0x3d, 0x3d, 7][rng.below(3) as usize],
    );
    let live: Vec<u32> =
        LIVE_CHRS.with(|c| c.borrow().iter().copied().filter(|&c| c != 0).collect());
    let saved: Vec<Vec<u8>> = [0x14, 0x17]
        .iter()
        .map(|&i| b.read(w.actor(i), 4).unwrap())
        .collect();
    for i in [0x14, 0x17] {
        let m = if live.is_empty() || rng.below(4) == 0 {
            0
        } else {
            live[rng.below(u32::try_from(live.len()).unwrap()) as usize]
        };
        poke(a, b, w.actor(i), &m.to_le_bytes());
    }
    zero_stack(a);
    poke(a, b, locals, &vec![0; sp::SPECIAL_LOCALS as usize]);
    twin("MsBattleSpecial", a, b, 0x0079_d430, &[], 0, |m| {
        dm(m, |m| sp::ms_battle_special(m, &mut call, h.trig, locals));
        0
    });
    poke(a, b, locals, &vec![0; sp::SPECIAL_LOCALS as usize]);
    rot_locals(a, b);
    local_words(a, b);
    same(&format!("MsBattleSpecial kind {kind}"), a, b);
    for (i, m) in [0x14, 0x17].into_iter().zip(saved) {
        poke(a, b, w.actor(i), &m);
    }
}

/// `MsAtelUpdate0_007972e0` (round 19) on a control of no works (an image
/// has no script set up for decoding; `MsAtelUpdate1` covers the work
/// loops): the motions and the special stage between them. The original's
/// `eax` is `MsBattleSpecial`'s leftover then, not compared. Right after
/// [`motions`], on its actors.
fn atel_update0(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    work: u32,
    locals: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::special::bridge as sp;
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    // the stage's models none (their optional positions need real ones)
    let saved: Vec<Vec<u8>> = [0x14, 0x17]
        .iter()
        .map(|&i| b.read(w.actor(i), 4).unwrap())
        .collect();
    for i in [0x14, 0x17] {
        poke(a, b, w.actor(i), &[0; 4]);
    }
    let ctrl = scratch(a, b, 0x10);
    poke(a, b, ctrl, &[0; 0x10]);
    poke(a, b, ctrl + 0xe, &rng.next().to_le_bytes()[..2]);
    // kind 1 without its boat reads stack words MsMotionProcess left
    // (`special_stage` covers it)
    let kind = [0i8, 2, 3][rng.below(3) as usize];
    set(a, b, ffx_battle::free::bridge::SPECIAL_KIND, kind);
    set(a, b, ffx_battle::free::bridge::SPECIAL_MEM, work);
    zero_stack(a);
    poke(a, b, locals, &vec![0; sp::SPECIAL_LOCALS as usize]);
    twin(
        "MsAtelUpdate0_007972e0",
        a,
        b,
        0x0079_72e0,
        &[ctrl],
        0,
        |m| {
            dm(m, |m| {
                let mut s = Script {
                    call: &mut call,
                    signal: &mut signal,
                    off: &mut off,
                };
                sp::ms_atel_update0(m, &mut s, h.trig, locals, ctrl)
            });
            0
        },
    );
    poke(a, b, locals, &vec![0; sp::SPECIAL_LOCALS as usize]);
    rot_locals(a, b);
    local_words(a, b);
    same("MsAtelUpdate0_007972e0", a, b);
    for (i, m) in [0x14, 0x17].into_iter().zip(saved) {
        poke(a, b, w.actor(i), &m);
    }
}

/// `MsBtlBridgeBtl2Event`'s script getters, each a [`table_stub`] over
/// the event actors (`ELEM`: over `(e, n)` pairs), and its setters,
/// [`logging_stub`]s.
const BRIDGE_GETTERS: [u32; 8] = {
    use ffx_battle::free::bridge as fr;
    [
        fr::ATEL_GET_EVENT_BIND_ID,
        fr::ATEL_GET_EVENT_CHAR_ID,
        fr::ATEL_GET_EVENT_CHAR_HANDLE,
        fr::ATEL_GET_EVENT_ACTOR_LINK_WORK,
        fr::ATEL_GET_BIND_PC,
        fr::ATEL_GET_HIDE_STATUS,
        fr::ATEL_GET_KEEP_BATTLE_PARTY,
        fr::ATEL_GET_EVENT_ELEM_STATUS,
    ]
};
const BRIDGE_SETTERS: [(u32, u8); 6] = {
    use ffx_battle::free::bridge as fr;
    [
        (fr::ATEL_SET_EVENT_ELEM_HANDLE, 3),
        (fr::ATEL_SET_EVENT_ELEM_POS, 3),
        (fr::ATEL_SET_EVENT_ELEM_CHAR_ID, 3),
        (fr::ATEL_SET_EVENT_ELEM_STATUS, 3),
        (fr::ATEL_SET_EVENT_CHAR_HANDLE, 2),
        (fr::ATEL_SET_EVENT_ACTOR_LINK_WORK, 2),
    ]
};
/// The event actors the bridge test sets up at most.
const BRIDGE_ACTORS: u32 = 4;
/// Where the bridge stubs go (after [`SPECIAL_CALLEES`]' in
/// [`SPECIAL_STUBS`]): getters 0x20 bytes each, then the logging ones.
const BRIDGE_STUBS: u32 = SPECIAL_STUBS + 0x400;

/// Replaces `f` by a stub at `at` returning `table[arg0]` (`pair`:
/// `table[2 arg0 + arg1]`), words.
fn table_stub(o: &mut Oracle, f: u32, at: u32, table: u32, pair: bool) {
    let mut code = vec![0x8b, 0x44, 0x24, 0x04];
    if pair {
        code.extend([0x03, 0xc0, 0x03, 0x44, 0x24, 0x08]);
    }
    code.extend([0x8b, 0x04, 0x85]);
    code.extend(table.to_le_bytes());
    code.push(0xc3);
    let at = o.rebase(at);
    o.write(at, &code).unwrap();
    let entry = o.rebase(f);
    let rel = at.wrapping_sub(entry + 5);
    o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
        .unwrap();
}

/// The bridge stubs in both oracles: tables from `tables` (0x40 bytes
/// each), setter records from `rec` ([`EXCHANGE_REC`] each), then the
/// actor total's and `graphicLevelGetNameHash`'s.
fn bridge_stubs(a: &mut Oracle, b: &mut Oracle, tables: u32, rec: u32) {
    use ffx_battle::free::bridge as fr;
    for o in [&mut *a, &mut *b] {
        for (k, f) in (0u32..).zip(BRIDGE_GETTERS) {
            let pair = f == fr::ATEL_GET_EVENT_ELEM_STATUS;
            table_stub(o, f, BRIDGE_STUBS + 0x20 * k, tables + 0x40 * k, pair);
        }
        let base = BRIDGE_STUBS + 0x20 * 8;
        for (k, (f, n)) in (0u32..).zip(BRIDGE_SETTERS) {
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        logging_stub(
            o,
            fr::ATEL_GET_EVENT_ACTOR_TOTAL,
            base + 0x40 * 6,
            rec + EXCHANGE_REC * 6,
            0,
            0,
        );
        logging_stub(
            o,
            fr::GRAPHIC_LEVEL_GET_NAME_HASH,
            base + 0x40 * 7,
            rec + EXCHANGE_REC * 7,
            0,
            0,
        );
        // MsBattleFree's and MsBattleAllFree's other callees (round 19;
        // GetCD is the effect stubs' already)
        for (k, (f, n)) in (8u32..).zip([
            (fr::GRAPHIC_SET_IS_BATTLE_MAP, 1u8),
            (fr::SG_ID_SET_EFFECT_TABLE_INDEX, 1),
            (fr::TO_CLEAR_BTL_WINDOW, 0),
            (fr::GRAPHIC_MENU_UNLOAD_SPECIAL_TEXTURE, 1),
            (fr::TROPHY_MANAGER_DESTROY_INSTANCE, 0),
        ]) {
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
    }
}

/// `MsBtlBridgeBtl2Event` (round 19) on event actors made up in the
/// stubs' tables: bound or not, their bind PCs the actor they name, an
/// actor of their character ID, or none.
/// `MsBtlBridgeEvent2Btl`'s element getters (round 34): pair
/// [`table_stub`]s after [`REVIVE_STUBS`]', installed around the call.
const BRIDGE_ELEM_GETTERS: [u32; 3] = {
    use ffx_battle::atel::bridge as at;
    [
        at::ATEL_GET_EVENT_ELEM_CHAR_ID,
        at::ATEL_GET_EVENT_ELEM_HANDLE,
        at::ATEL_GET_EVENT_ELEM_POS,
    ]
};
const BRIDGE_ELEM_STUBS: u32 = REVIVE_STUBS + 0x100;

/// The event's characters handed to the battle (`MsBtlBridgeEvent2Btl`,
/// round 34) over [`bridge_stubs`]' tables: up to four event actors,
/// bound or not, with a character or not, of an actor's ID (in the
/// battle, without a character yet, at times) or not, its bound PC that
/// actor, another, none or out of range; their two elements with IDs,
/// handles and positions or zeros, onto weapon records free or taken.
/// What it changes is put back after.
#[allow(clippy::too_many_arguments)]
fn bridge_to_battle(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    f: &Fakes,
    tables: u32,
    rec: u32,
    elems: u32,
) {
    let n = rng.below(BRIDGE_ACTORS + 1);
    poke(a, b, rec + EXCHANGE_REC * 6 + 4, &n.to_le_bytes());
    let first = ffx_battle::actor::bridge::ms_get_chr(b, ACTORS);
    let regions: Vec<(u32, usize)> = vec![
        (first, (0x5d - ACTORS as usize) * 0x390),
        (w.actor(0), (ACTORS * ACTOR) as usize),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    for e in 0..BRIDGE_ACTORS {
        let k = [rng.below(0x1f), 0x1b, 0x1c][rng.below(3) as usize];
        let p = w.actor(k);
        // the actor that takes it, at times (or one out of the battle)
        if rng.below(2) == 0 {
            poke(a, b, p, &[0; 4]);
            poke(a, b, p + 0x10, &[u8::from(rng.below(4) != 0)]);
        }
        let id = if rng.below(4) == 0 {
            rng.next()
        } else {
            u32::from_le_bytes(b.read(p + 4, 4).unwrap().try_into().unwrap())
        };
        // an earlier actor that would take it too (the bound PC first)
        if k > 0 && rng.below(3) == 0 {
            let q = w.actor(rng.below(k));
            poke(a, b, q, &[0; 4]);
            poke(a, b, q + 4, &id.to_le_bytes());
            poke(a, b, q + 0x10, &[1]);
        }
        let pc = [u32::MAX, k, k, rng.below(0x1f), 0x1b, 0x1c, 0x30][rng.below(7) as usize];
        // the link work's upper bytes are the worker's (AtelGetEventActorLinkWork)
        let values = [
            [1, 1, 1, 0, 2][rng.below(5) as usize],
            id,
            [0, f.chars, f.chars + 0x1000][rng.below(3) as usize],
            rng.next(),
            pc,
        ];
        for (g, v) in (0u32..).zip(values) {
            poke(a, b, tables + 0x40 * g + 4 * e, &v.to_le_bytes());
        }
        for m in 0..2 {
            let at = 4 * (2 * e + m);
            let v = [
                [0, rng.below(0x10000), 0x1000 + rng.below(8)][rng.below(3) as usize],
                [0, f.chars + 0x2000, rng.next()][rng.below(3) as usize],
                rng.next(),
            ];
            for (g, v) in (0u32..).zip(v) {
                poke(a, b, elems + 0x40 * g + at, &v.to_le_bytes());
            }
            // the weapon record free or taken
            let wr = ffx_battle::actor::bridge::ms_get_weapon(b, k, m.cast_signed());
            let taken = [0, f.chars + 0x1000][rng.below(2) as usize];
            poke(a, b, wr, &taken.to_le_bytes());
        }
    }
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        for (k, g) in (0u32..).zip(BRIDGE_ELEM_GETTERS) {
            let at = o.rebase(g);
            saved.push((g, o.read(at, 5).unwrap()));
            table_stub(o, g, BRIDGE_ELEM_STUBS + 0x20 * k, elems + 0x40 * k, true);
        }
    }
    saved.truncate(saved.len() / 2);
    twin("MsBtlBridgeEvent2Btl", a, b, 0x0078_24e0, &[], 0, |m| {
        ffx_battle::atel::bridge::ms_btl_bridge_event2_btl(m, &mut game_call_o);
        0
    });
    unstub(a, b, &saved);
    same("MsBtlBridgeEvent2Btl", a, b);
    for (&(at, _), bytes) in regions.iter().zip(&kept) {
        poke(a, b, at, bytes);
    }
}

fn bridge_to_event(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    f: &Fakes,
    tables: u32,
    rec: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::free::bridge as fr;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let n = rng.below(BRIDGE_ACTORS + 1);
    poke(a, b, rec + EXCHANGE_REC * 6 + 4, &n.to_le_bytes());
    let hash = [0x21e8u32, 0x21e8, 7][rng.below(3) as usize];
    poke(a, b, rec + EXCHANGE_REC * 7 + 4, &hash.to_le_bytes());
    let id_of = |o: &mut Oracle, k: u32| {
        u32::from_le_bytes(o.read(w.actor(k) + 4, 4).unwrap().try_into().unwrap())
    };
    for e in 0..BRIDGE_ACTORS {
        let k = rng.below(0x1f);
        let id = if rng.below(4) == 0 {
            rng.next()
        } else {
            id_of(b, k)
        };
        let pc = [u32::MAX, k, rng.below(0x1f), 0x30][rng.below(4) as usize];
        let values = [
            [1, 1, 1, 0, 2][rng.below(5) as usize],
            id,
            [0, 0x1111, 0x2222][rng.below(3) as usize],
            rng.below(0x100),
            pc,
            [0, 0, 1, 0xffff_fffe][rng.below(4) as usize],
            u32::from(rng.below(3) == 0),
        ];
        for (g, v) in (0u32..).zip(values) {
            poke(a, b, tables + 0x40 * g + 4 * e, &v.to_le_bytes());
        }
        for m in 0..2 {
            poke(
                a,
                b,
                tables + 0x40 * 7 + 4 * (2 * e + m),
                &u32::from(rng.below(3) != 0).to_le_bytes(),
            );
        }
    }
    // what this changes, put back after (the next round's calls expect
    // `dress`'s models, records and actors)
    let first = ffx_battle::actor::bridge::ms_get_chr(b, ACTORS);
    // (and the battle's globals, which the whole teardown below clears)
    let regions: Vec<(u32, usize)> = vec![
        (f.chars, 0x3000),
        (first, (0x5d - ACTORS as usize) * 0x390),
        (w.actor(0), (ACTORS * ACTOR) as usize),
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    // the fake models without cluts or texture animations (their
    // pointers would be random), the weapon records' models fake or none
    for c in 0..3 {
        let chr = f.chars + c * 0x1000;
        poke(a, b, chr + ffx_chara::layout::CLUTS, &[0; 4]);
        // a texture animation of the static table, or none
        poke(
            a,
            b,
            chr + 0x6fc,
            &u32::from(rng.below(2) == 0).to_le_bytes(),
        );
        poke(a, b, chr + 0x700, &rng.below(4).to_le_bytes());
        // the model block Ch_SetThickness takes its default from
        poke(a, b, chr + 0x1c4, &f.chars.to_le_bytes());
    }
    for k in ACTORS..0x5d {
        let p = ffx_battle::actor::bridge::ms_get_chr(b, k);
        let m = [0, f.chars + 0x1000 * rng.below(3)][rng.below(2) as usize];
        poke(a, b, p, &m.to_le_bytes());
        poke(
            a,
            b,
            p + 4,
            &[0, rng.next()][rng.below(2) as usize].to_le_bytes(),
        );
        poke(a, b, p + 0x12, &[0]);
    }
    // in battle scene 0x02370000 at times (models handed over there stay
    // hidden on level 0x21e8)
    if rng.below(2) == 0 {
        use ffx_battle::state::bridge as st;
        poke(a, b, tables + 0x1f0, &0x237u16.to_le_bytes());
        poke(a, b, tables + 0x1f4, &[0]);
        set(a, b, st::SCENE_FIELD, tables + 0x1f0);
        set(a, b, st::SCENE_GROUP, tables + 0x1f4);
        set(a, b, st::STATUS, 1);
    }
    // the party (kept members are looked up in its first three) and the
    // actors' party slots, both small
    let party: Vec<u8> = (0..7)
        .map(|_| u8::try_from(rng.below(8)).unwrap())
        .collect();
    poke(a, b, ffx_battle::ability::bridge::PARTY.ptr(b).addr, &party);
    for k in 0..ACTORS {
        poke(
            a,
            b,
            w.actor(k) + 0xc,
            &[u8::try_from(rng.below(8)).unwrap()],
        );
    }
    for k in 0..ACTORS {
        let p = w.actor(k);
        for o in [0x12u32, 0x1b, 0x1c, 0x21, 0x22, 0x3a] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
    }
    twin("MsBtlBridgeBtl2Event", a, b, 0x0078_21f0, &[], 0, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            fr::ms_btl_bridge_btl2_event(m, &mut s);
        });
        0
    });
    same("MsBtlBridgeBtl2Event", a, b);
    // the whole teardown on what is left (in battle or not)
    if rng.below(2) == 0 {
        let status = [0i8, 1, 0x1b][rng.below(3) as usize];
        set(a, b, ffx_battle::state::bridge::STATUS, status);
        // the state the teardown clears, set
        set(a, b, ffx_battle::save::bridge::USE_ITEM, 0x2001);
        set(a, b, ffx_battle::process::bridge::LIMIT_TYPE_CUE, 1);
        set(a, b, ffx_battle::minor::bridge::ZANMATO_SEP, [7; 10]);
        // the special stage's up vector (special_stage left floats there)
        let sm = ffx_battle::free::bridge::SPECIAL_MEM.get(b);
        if sm != 0 {
            poke(a, b, sm + 0x810, &[0; 4]);
        }
        let all = rng.below(2) == 0;
        if all {
            // menu text 0 not freed: the original's clear after freeing it
            // lands on MsBattleAllFree's return address (docs/formats/battle.md)
            let text = ffx_battle::flow::bridge::MENU_TEXT.ptr(b).addr;
            poke(a, b, text + 8, &[0; 4]);
        }
        let (name, sym) = if all {
            ("MsBattleAllFree", 0x0078_0b40)
        } else {
            ("MsBattleFree", 0x0078_1590)
        };
        twin(name, a, b, sym, &[], ALL, |m| {
            dm(m, |m| {
                let mut s = Script {
                    call: &mut call,
                    signal: &mut signal,
                    off: &mut off,
                };
                if all {
                    fr::ms_battle_all_free(m, &mut s)
                } else {
                    fr::ms_battle_free(m, &mut s)
                }
            })
        });
        same(name, a, b);
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// The setup leaves (`ffx_battle::getmem`, round 19): `user_malloc` and
/// `AtelGetWaterBattleEffect` stubbed for the call (records at `rec`, the
/// heap handing out `heap` or null), then put back.
fn setup_leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, heap: u32, out: u32) {
    use ffx_battle::getmem::bridge as gm;
    let mut call = game_call;
    // the battle's globals these change, put back after
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let base = BRIDGE_STUBS + 0x20 * 8 + 0x40 * 14;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in
            (0u32..).zip([(gm::USER_MALLOC, 1), (gm::ATEL_GET_WATER_BATTLE_EFFECT, 0)])
        {
            let at = o.rebase(f);
            saved.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
    }
    // MsNumberGetMem clears what it got unchecked: a block always there
    poke(a, b, rec + 4, &heap.to_le_bytes());
    let water = u32::from(rng.below(2) == 0);
    poke(a, b, rec + EXCHANGE_REC + 4, &water.to_le_bytes());
    twin(
        "MsGetSaveConfigEnglish",
        a,
        b,
        0x0078_51d0,
        &[],
        ALL,
        |_| gm::ms_get_save_config_english(),
    );
    let numbers = [0, 0, rng.next() | 1][rng.below(3) as usize];
    set(a, b, ffx_battle::misc::bridge::NUMBERS, numbers);
    twin("MsNumberGetMem", a, b, 0x0079_f9b0, &[], 0, |m| {
        dm(m, |m| gm::ms_number_get_mem(m, &mut call));
        0
    });
    same("MsNumberGetMem", a, b);
    let got = [heap, heap, 0][rng.below(3) as usize];
    poke(a, b, rec + 4, &got.to_le_bytes());
    twin("MsBtlGetItemMem", a, b, 0x0079_9fb0, &[], 0, |m| {
        dm(m, |m| gm::ms_btl_get_item_mem(m, &mut call));
        0
    });
    same("MsBtlGetItemMem", a, b);
    let r1 = gm::SCRIPT_RECORD1.ptr(b).addr;
    fill(a, b, rng, r1, 0x20);
    let status = ffx_battle::state::bridge::STATUS.get(b);
    twin(
        "MsBtlScriptRecordShift_00782670",
        a,
        b,
        0x0078_2670,
        &[],
        0,
        |m| {
            gm::ms_btl_script_record_shift(m);
            0
        },
    );
    same("MsBtlScriptRecordShift_00782670", a, b);
    set(a, b, ffx_battle::state::bridge::STATUS, status);
    for _ in 0..3 {
        let kind = [u32::MAX, 0, 1, 2, 3, 4, rng.next()][rng.below(7) as usize];
        let o = [0, out + 0x80][rng.below(2) as usize];
        twin(
            "MsBtlEncountEffect_007829b0",
            a,
            b,
            0x0078_29b0,
            &[kind, o],
            ALL,
            |m| dm(m, |m| gm::ms_btl_encount_effect(m, &mut call, kind, o)),
        );
        same("MsBtlEncountEffect_007829b0", a, b);
    }
    for (k, (f, v)) in saved.into_iter().enumerate() {
        let o = if k < 2 { &mut *a } else { &mut *b };
        let at = o.rebase(f);
        o.write(at, &v).unwrap();
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// Returns `table[count & 3]` (the table at `rec + 8`), counting calls at
/// `rec`: `f` replaced by it at `at` unless 0 (then only the code, for a
/// function pointer).
fn counting_stub(o: &mut Oracle, f: u32, at: u32, rec: u32) {
    let mut code = vec![0xa1];
    code.extend(rec.to_le_bytes());
    code.extend([0xff, 0x05]);
    code.extend(rec.to_le_bytes());
    code.extend([0x83, 0xe0, 0x03, 0x8b, 0x04, 0x85]);
    code.extend((rec + 8).to_le_bytes());
    code.push(0xc3);
    o.write(rec, &[0; 4]).unwrap();
    let at = o.rebase(at);
    o.write(at, &code).unwrap();
    if f != 0 {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
            .unwrap();
    }
}

/// `MsBattleExe`'s leaves (round 20): the CD object (`GetCD`'s, its
/// directory, size and name methods stubs, the sizes from a table),
/// `MsGetCheckEnglish`, the effects', sound's and assertion's callees
/// stubbed for the call (records from `rec`, [`EXCHANGE_REC`] each), the
/// heap handing out `heap`; then put back with the battle's globals.
fn exe_leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, heap: u32, out: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::getmem::bridge as gm;
    use ffx_battle::state::bridge as st;
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let base = BRIDGE_STUBS + 0x20 * 8 + 0x40 * 16;
    let cdobj = out + 0x40;
    let callees = [
        (gm::GET_CD, 0u8),
        (gm::MS_GET_CHECK_ENGLISH, 0),
        (gm::YI_ASSERT, 4),
        (gm::MS_REG_SE_PLAY2, 2),
        (gm::OP_ET_ENCOUNT_EFFECT_ON, 2),
        (gm::OP_ET_BATTLE_EFFECT_INIT, 0),
        (gm::USER_MALLOC, 1),
    ];
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip(callees) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        // the CD object's methods: +0x10 directory, +0x58 name (logging),
        // +0x14 size (counting), and the event effect's function
        logging_stub(o, 0, base + 0x40 * 7, rec + EXCHANGE_REC * 7, 1, 0);
        logging_stub(
            o,
            0,
            base + 0x40 * 8,
            rec + EXCHANGE_REC * 8,
            1,
            0x00b5_4790,
        );
        counting_stub(o, 0, base + 0x40 * 9, rec + EXCHANGE_REC * 9);
        logging_stub(o, 0, base + 0x40 * 10, rec + EXCHANGE_REC * 10, 1, 0);
        for (slot, k) in [(0x10u32, 7u32), (0x58, 8), (0x14, 9), (8, 10)] {
            let f = o.rebase(base + 0x40 * k);
            o.write(cdobj + slot, &f.to_le_bytes()).unwrap();
        }
        let f = o.rebase(base + 0x40 * 10);
        o.write(out + 0xa4, &f.to_le_bytes()).unwrap();
        saved.push(mine);
    }
    poke(a, b, rec + 4, &cdobj.to_le_bytes());
    let english = [0u32, u32::MAX][rng.below(2) as usize];
    poke(a, b, rec + EXCHANGE_REC + 4, &english.to_le_bytes());
    poke(a, b, rec + EXCHANGE_REC * 6 + 4, &heap.to_le_bytes());
    let sizes: Vec<u8> = (0..4)
        .flat_map(|_| [rng.below(0x9000), rng.next()][rng.below(2) as usize].to_le_bytes())
        .collect();
    poke(a, b, rec + EXCHANGE_REC * 9 + 8, &sizes);
    // the file sizes and the index
    for _ in 0..2 {
        let (dir, dir2, file) = ([0xe, 3][rng.below(2) as usize], 0xb, rng.below(0x400));
        twin(
            "MsGetFileSize",
            a,
            b,
            0x0078_2a20,
            &[dir, dir2, file],
            ALL,
            |m| dm(m, |m| gm::ms_get_file_size(m, &mut call, dir, dir2, file)),
        );
        same("MsGetFileSize", a, b);
        let both = rng.below(2);
        twin(
            "MsBattleExeSetIndex",
            a,
            b,
            0x0078_1550,
            &[both, file],
            ALL,
            |m| dm(m, |m| gm::ms_battle_exe_set_index(m, &mut call, both, file)),
        );
        same("MsBattleExeSetIndex", a, b);
    }
    // the event effect: none, or one still running (its table's function
    // the stub at base + 10, or no table)
    set(
        a,
        b,
        st::EVENT_EFFECT_STAT,
        [0i8, 3, 1][rng.below(3) as usize],
    );
    set(a, b, st::EVENT_EFFECT_B, rng.next());
    set(
        a,
        b,
        st::EVENT_EFFECT_A,
        [0, out + 0xa0][rng.below(2) as usize],
    );
    twin("MsEventEffectFree", a, b, 0x0078_8b90, &[], ALL, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            gm::ms_event_effect_free(m, &mut s)
        })
    });
    same("MsEventEffectFree", a, b);
    twin("MsBtlChrGetMem", a, b, 0x0079_3530, &[], 0, |m| {
        dm(m, |m| gm::ms_btl_chr_get_mem(m, &mut call));
        0
    });
    same("MsBtlChrGetMem", a, b);
    // a field list of the image or snapshot (MsBtlListField clamps)
    for _ in 0..3 {
        let field = [0, rng.below(4), rng.next()][rng.below(3) as usize];
        poke(a, b, out + 0x80, &rng.range(-2, 6).to_le_bytes());
        poke(a, b, out + 0x84, &rng.range(-2, 6).to_le_bytes());
        let args = [field, out + 0x80, out + 0x84];
        if ffx_battle::misc::bridge::FIELD_TABLE2.get(b) == 0 {
            break;
        }
        twin("MsBtlListCDpos", a, b, 0x0079_d0e0, &args, ALL, |m| {
            gm::ms_btl_list_cd_pos(m, field, out + 0x80, out + 0x84)
        });
        same("MsBtlListCDpos", a, b);
    }
    set(
        a,
        b,
        ffx_battle::process::bridge::RESET_WAIT,
        [0, 0, 5][rng.below(3) as usize],
    );
    set(
        a,
        b,
        gm::ENCOUNT_EFFECTS,
        u8::try_from(rng.below(0x100)).unwrap(),
    );
    let (kind, se) = (rng.below(4), rng.below(0x100));
    twin(
        "MsEtEffectEncount",
        a,
        b,
        0x0079_e5e0,
        &[kind, se],
        ALL,
        |m| dm(m, |m| gm::ms_et_effect_encount(m, &mut call, kind, se)),
    );
    same("MsEtEffectEncount", a, b);
    set(a, b, ffx_battle::misc::bridge::ET_EFFECT_READ, rng.next());
    twin("MsEtEffectSet", a, b, 0x0079_e7a0, &[], ALL, |m| {
        dm(m, |m| gm::ms_et_effect_set(m, &mut call))
    });
    same("MsEtEffectSet", a, b);
    exe(a, b, rng, rec, out);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsBattleExe`'s other callees (round 20), stubbed for one call after
/// [`exe_leaves`]' (records from `rec + 11 *`).
const EXE_CALLEES: [(u32, u8); 16] = {
    use ffx_battle::exe::bridge as ex;
    [
        (ex::FFX_IS_DEBUG, 0),
        (ex::SS_SET_PROJECT_BASE, 1),
        (ex::SG_MEM_SET_ALIGN, 1),
        (ex::SG_BLUR, 1),
        (ex::MS_SCREEN_FOCUS_RESET_ALL, 0),
        (ex::SG_FADE_IN, 1),
        (ex::SG_SET_EXEC_FRAMES, 1),
        (ex::SG_SET_PLAYER, 1),
        (ex::SG_MALLOC_A, 4),
        (ex::SG_SET_KEEP_FPS, 1),
        (ex::ATEL_EVENT_SET_UP_FOR_BATTLE_MUSIC, 1),
        (ex::TO_INIT_BTL_WINDOW, 0),
        (ex::MS_BTL_READ_MANAGE, 0),
        (ex::MS_DEBUG_SET_SCENE, 4),
        (ex::MS_SET_ITEM_WINDOW, 0),
        (ex::MS_BTL_START_AP, 0),
    ]
};

/// `MsBattleExe` on the snapshot's field list (the image has none): from
/// an event or the field (out of battle, so the battle before is not
/// freed), debug or not.
fn exe(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, out: u32) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::exe::bridge as ex;
    if ffx_battle::misc::bridge::FIELD_TABLE2.get(b) == 0 {
        return;
    }
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let base = BRIDGE_STUBS + 0x20 * 8 + 0x40 * 27;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip(EXE_CALLEES) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * (11 + k), n, 0);
        }
        saved.push(mine);
    }
    let debug = u32::from(rng.below(3) == 0);
    poke(a, b, rec + EXCHANGE_REC * 11 + 4, &debug.to_le_bytes());
    poke(
        a,
        b,
        rec + EXCHANGE_REC * (11 + 8) + 4,
        &(out + 0xc0).to_le_bytes(),
    );
    poke(
        a,
        b,
        rec + EXCHANGE_REC * (11 + 12) + 4,
        &0x5eu32.to_le_bytes(),
    );
    set(a, b, ffx_battle::state::bridge::STATUS, 0);
    set(a, b, ffx_battle::state::bridge::EXE, 3);
    // the encounter type and no soft reset waiting (the encounter effect runs)
    set(
        a,
        b,
        ffx_battle::exe::bridge::ENCOUNT_TYPE,
        [-1i8, 0, 1, 2, 5][rng.below(5) as usize],
    );
    set(a, b, ffx_battle::process::bridge::RESET_WAIT, 0);
    let rec0 = ffx_battle::atel::bridge::SCRIPT_RECORDS.ptr(b).addr + 4;
    poke(
        a,
        b,
        rec0,
        &[0, out + 0xe0][rng.below(2) as usize].to_le_bytes(),
    );
    let args = [
        rng.below(2),
        [0, rng.below(4), rng.next()][rng.below(3) as usize],
        rng.range(-1, 5).cast_unsigned(),
        rng.range(-1, 9).cast_unsigned(),
    ];
    twin("MsBattleExe", a, b, 0x0078_1020, &args, ALL, |m| {
        dm(m, |m| {
            let mut s = Script {
                call: &mut call,
                signal: &mut signal,
                off: &mut off,
            };
            ex::ms_battle_exe(m, &mut s, args[0], args[1], args[2], args[3])
        })
    });
    same(&format!("MsBattleExe {args:x?}"), a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
}

/// `MsBtlReadManage`'s leaves (round 20): the scene read (`GetCD`'s
/// object, `Sg_MallocA` stubbed for the call, records from `rec`), the
/// loading wait, the monsters placed, the event hide flags.
fn read_leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32, out: u32) {
    use ffx_battle::read::bridge as rd;
    let h = host();
    let mut call = game_call;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
        (w.actor(0), (ACTORS * ACTOR) as usize),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let base = BRIDGE_STUBS + 0x20 * 8 + 0x40 * 43;
    let cdobj = out + 0x40;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip([
            (ffx_battle::getmem::bridge::GET_CD, 0u8),
            (rd::SG_MALLOC_A, 4),
        ]) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        for (k, n) in [(2u32, 4u8), (3, 1), (4, 1)] {
            logging_stub(o, 0, base + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        for (slot, k) in [(0u32, 2u32), (0x10, 3), (0x14, 4)] {
            let f = o.rebase(base + 0x40 * k);
            o.write(cdobj + slot, &f.to_le_bytes()).unwrap();
        }
        saved.push(mine);
    }
    poke(a, b, rec + 4, &cdobj.to_le_bytes());
    poke(a, b, rec + EXCHANGE_REC + 4, &(out + 0xc0).to_le_bytes());
    poke(a, b, rec + EXCHANGE_REC * 2 + 4, &rng.next().to_le_bytes());
    poke(
        a,
        b,
        rec + EXCHANGE_REC * 4 + 4,
        &rng.below(0x8000).to_le_bytes(),
    );
    let file = rng.below(0x200);
    twin("MsBtlReadScene", a, b, 0x0078_3d80, &[file], ALL, |m| {
        dm(m, |m| rd::ms_btl_read_scene(m, &mut call, file))
    });
    same("MsBtlReadScene", a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for k in 0..ACTORS {
        poke(a, b, w.actor(k) + 0x433, &[u8::from(rng.below(12) == 0)]);
        poke(a, b, w.actor(k) + 0x407, &[u8::from(rng.below(3) == 0)]);
    }
    twin("MsGetRamChrCTBbasic", a, b, 0x0078_4220, &[], ALL, |m| {
        rd::ms_get_ram_chr_ctb_basic(m).cast_unsigned()
    });
    same("MsGetRamChrCTBbasic", a, b);
    if placed(a, b, w, out) {
        twin("MsCalcChrDirAll", a, b, 0x0079_3cb0, &[], 0, |m| {
            dm(m, |m| rd::ms_calc_chr_dir_all(m, &h, Some(HARNESS)));
            0
        });
        local_words(a, b);
        rect_words(a, b);
        same("MsCalcChrDirAll", a, b);
        zero_stack(a);
        twin(
            "MsPlaceMonsters_00784420",
            a,
            b,
            0x0078_4420,
            &[],
            ALL,
            |m| dm(m, |m| rd::ms_place_monsters(m, &h, &mut call)),
        );
        local_words(a, b);
        same("MsPlaceMonsters_00784420", a, b);
    }
    for _ in 0..2 {
        let i = [rng.below(ACTORS), rng.below(0x100), rng.next()][rng.below(3) as usize];
        let flags = rng.below(8);
        twin(
            "MsSetChrVisibleEvent",
            a,
            b,
            0x0079_67f0,
            &[i, flags],
            ALL,
            |m| dm(m, |m| rd::ms_set_chr_visible_event(m, &mut call, i, flags)),
        );
        same("MsSetChrVisibleEvent", a, b);
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
    // the originals' stack as the later steps expect it (MsEffectStart
    // reads a local it never writes)
    zero_stack(a);
}

/// `MsBtlReadManage`'s callees of other modules and the two battle readers
/// it leaves to the game (`ffx_battle::read`), with their argument counts;
/// then the CD object's methods `+0`, `+0x10`, `+0x14`, `+0x58` (round 21).
const READ_MANAGE_CALLEES: [(u32, u8); 19] = {
    use ffx_battle::read::bridge as rd;
    [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (rd::ATEL_IS_CALLBACK_QUE, 0),
        (rd::GRAPHIC_LEVEL_GET_NAME_HASH, 0),
        (rd::GRAPHIC_REGIST_ASYNC_TASK, 2),
        (rd::TO_INIT_BATTLE_FONT_PATH, 1),
        (rd::TK_GET_LANG, 0),
        (rd::GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE, 3),
        (rd::FLUSH_CACHE, 1),
        (rd::MS_BTL_READ_SET_SCENE, 0),
        (rd::GRAPHIC_SET_IS_BATTLE_MAP, 1),
        (rd::YI_BATTLE_MAP_READ_NB, 2),
        (rd::YI_FIELD_MAP_READ_NB, 2),
        (rd::YI_GET_RENDER_STATUS, 0),
        (rd::LOAD_PHYRE_FIELD_MAP, 0),
        (rd::SQS_SET_REVERB_PLAY_SWITCH, 1),
        (rd::CH_CHECK_NB_BATTLE_CHR, 0),
        (rd::MS_BTL_READ_MONSTER, 0),
        (rd::SG_MALLOC_A, 4),
        (ffx_battle::getmem::bridge::MS_GET_CHECK_ENGLISH, 0),
    ]
};
const READ_MANAGE_STUBS: u32 = BRIDGE_STUBS + 0x20 * 8 + 0x40 * 48;
const READ_MANAGE_RECS: u32 = 23;

/// `BtlEnd_DrawWeaponPlate_008be0d0` (round 28): the plate and name draws
/// as logging stubs (in [`READ_MANAGE_STUBS`]' slots, records from `rec`),
/// the remaps as stand-ins (x kept, y doubled), up to four spoils (IDs of the
/// spoils' and other equipment at `block`, never 0xff: the original reads
/// through the null slot), held or not, equippable or not; spoils in and
/// out of range, the rows' offset or a fixed y; the spoils' slots and
/// globals put back after.
fn weapon_plate(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, block: u32) {
    use ffx_battle::battle_end::bridge::{self as be, Remap};
    let mut call = game_call_o;
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, f) in (0u32..).zip([
            be::TO_DRAW_MENU_PLATE_XYWH_TYPE,
            be::DRAW_CROSS_MENU_ICON_WEAPON_NAME,
        ]) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, READ_MANAGE_STUBS + 0x40 * k, stub(k), 4, 0);
        }
        // the remaps (their device globals are unset on the image): x as
        // it is, y doubled
        for (k, (f, code)) in (2u32..).zip([
            (0x0064_47e0u32, &[0xd9u8, 0x44, 0x24, 0x04, 0xc3][..]),
            (0x0064_4820, &[0xd9, 0x44, 0x24, 0x04, 0xd8, 0xc0, 0xc3][..]),
        ]) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            let body = o.rebase(READ_MANAGE_STUBS + 0x40 * k);
            o.write(body, code).unwrap();
            let rel = body.wrapping_sub(at + 5);
            o.write(at, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
                .unwrap();
        }
        saved.push(mine);
    }
    let spoils = ffx_battle::save::bridge::SPOILS_EQUIPMENT.ptr(b).addr;
    let kept = (
        b.read(spoils, 0x16 * 8).unwrap(),
        be::SPOIL_WEAPONS.get(b),
        be::SPOIL_WEAPON_COUNT.get(b),
        be::SPOIL_WEAPON_ROWS.get(b),
    );
    let count = rng.below(5);
    let ids: Vec<u8> = (0..4)
        .flat_map(|_| {
            let id = [
                0xb000 | rng.below(10),
                0x7000 | rng.below(8),
                rng.below(0xfe),
            ][rng.below(3) as usize];
            u16::try_from(id).unwrap().to_le_bytes()
        })
        .collect();
    poke(a, b, block, &ids);
    set(a, b, be::SPOIL_WEAPONS, block);
    set(a, b, be::SPOIL_WEAPON_COUNT, count.cast_signed());
    // (at times more rows than an f32 holds exactly: 0x100001f rounds up
    // to 0x1000020, which moves the plate a pixel)
    let rows = [rng.range(-0x3000, 0x8000), 0x0100_001f][rng.below(2) as usize];
    set(a, b, be::SPOIL_WEAPON_ROWS, rows);
    for k in 0..8 {
        let slot = ffx_battle::save::bridge::SPOILS_EQUIPMENT.ptr(b).addr + 0x16 * k;
        poke(a, b, slot + 2, &[u8::from(rng.below(4) != 0)]);
        poke(a, b, slot + 6, &[[0u8, 3, 0xff][rng.below(3) as usize]]);
    }
    let n = [0, 1, 2, 3, count, (-1i32).cast_unsigned()][rng.below(6) as usize];
    let x = rng.range(-50, 700).cast_unsigned();
    let y = rng.range(-50, 500).cast_unsigned();
    let fixed = [0, 1, rng.next()][rng.below(3) as usize];
    let remap = |r: Remap, v: f32| match r {
        Remap::X2 => v,
        Remap::Y2 => v * 2.0,
    };
    let local = a.stack_range().end - 0x200;
    twin(
        "BtlEnd_DrawWeaponPlate_008be0d0",
        a,
        b,
        0x008b_e0d0,
        &[n, x, y, fixed],
        0,
        |m| {
            be::btl_end_draw_weapon_plate(m, &remap, &mut call, n, x, y, fixed, local);
            0
        },
    );
    same("BtlEnd_DrawWeaponPlate_008be0d0", a, b);
    poke(a, b, spoils, &kept.0);
    set(a, b, be::SPOIL_WEAPONS, kept.1);
    set(a, b, be::SPOIL_WEAPON_COUNT, kept.2);
    set(a, b, be::SPOIL_WEAPON_ROWS, kept.3);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
}

/// `MsDebugFullCommand` (round 28): `GetCD`'s object, `Sg_MallocA` and
/// `Sg_FreeA` as logging stubs (in [`READ_MANAGE_STUBS`]' slots, records
/// from `rec`), the read block `block` a `ply_data` of up to two segments
/// (records 0x12, 0x13 in one, both, or neither: the first record then),
/// in a battle or not (`MsBattleCheck`'s bytes, put back after); not on
/// a snapshot without live characters (field.snap's made-up actors,
/// rebuilt as a battle's, loop the original's list sort).
fn debug_full_command(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    block: u32,
    synthetic: bool,
) {
    use ffx_battle::read::bridge as rd;
    let mut call = game_call;
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let cdobj = block + 0x1c0;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip([
            (ffx_battle::getmem::bridge::GET_CD, 0u8),
            (rd::SG_MALLOC_A, 4),
            (rd::SG_FREE_A, 2),
        ]) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, READ_MANAGE_STUBS + 0x40 * k, stub(k), n, 0);
        }
        for (k, (slot, n)) in (3u32..).zip([(0u32, 4u8), (8, 1), (0x10, 1), (0x14, 1)]) {
            logging_stub(o, 0, READ_MANAGE_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(READ_MANAGE_STUBS + 0x40 * k);
            o.write(cdobj + slot, &f.to_le_bytes()).unwrap();
        }
        saved.push(mine);
    }
    poke(a, b, stub(0) + 4, &cdobj.to_le_bytes());
    poke(a, b, stub(1) + 4, &block.to_le_bytes());
    poke(a, b, stub(6) + 4, &rng.below(0x10000).to_le_bytes());
    // the table: segments (first, last, stride, strings, data) after the
    // count, records from 0x20
    fill(a, b, rng, block, 0x1c0);
    let count = rng.below(3);
    let first = 0x12 - rng.below(2);
    let last = [0x12, 0x13, 0x20][rng.below(3) as usize];
    let mut head = [0u8; 0x20];
    head[..2].copy_from_slice(&u16::try_from(count).unwrap().to_le_bytes());
    for (n, (lo, hi)) in [(first, last), (0x13, 0x13)].into_iter().enumerate() {
        let seg = 8 + 12 * n;
        let fields = [lo, hi, 0x4a, 0x10];
        for (k, f) in fields.into_iter().enumerate() {
            head[seg + 2 * k..seg + 2 * k + 2]
                .copy_from_slice(&u16::try_from(f).unwrap().to_le_bytes());
        }
        let data = 0x20 + 0x4a * u32::try_from(n).unwrap();
        head[seg + 8..seg + 12].copy_from_slice(&data.to_le_bytes());
    }
    poke(a, b, block, &head);
    let flags = [
        ffx_battle::state::bridge::STATUS,
        ffx_battle::state::bridge::EXE,
    ];
    let kept = flags.map(|g| g.get(b));
    let live = LIVE_CHRS.with(|c| c.borrow().iter().any(|&c| c != 0));
    let battle = i8::from(rng.below(2) == 0 && (synthetic || live));
    for g in flags {
        set(a, b, g, battle);
    }
    twin("MsDebugFullCommand", a, b, 0x0078_48b0, &[], 0, |m| {
        dm(m, |m| rd::ms_debug_full_command(m, &mut call));
        0
    });
    same("MsDebugFullCommand", a, b);
    for (g, v) in flags.into_iter().zip(kept) {
        set(a, b, g, v);
    }
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
}

/// `MsBtlReadManage` (round 21): the read's state machine from each status
/// a few calls on, its callees as [`READ_MANAGE_CALLEES`]' logging stubs
/// (records from `rec`, results drawn) and `MsEffectStart`'s stubs for
/// `MsAtelSetUp` (records from `erec`); `data` holds the CD's name, the
/// formation and the scene's field and group. The battle's globals, the
/// actors and `AsyncLoadNewCharacter`'s count are put back after.
// (the picked values are small: the casts keep them)
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
fn read_manage(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    rec: u32,
    erec: u32,
    data: u32,
    out: u32,
) {
    use ffx_battle::atel::bridge::Script;
    use ffx_battle::read::bridge as rd;
    macro_rules! pick {
        ($rng:expr, $v:expr $(,)?) => {{
            let v: &[u32] = $v;
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let h = host();
    let mut call = game_call;
    let mut signal = game_signal;
    let mut off = eff_off;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
        (w.actor(0), (ACTORS * ACTOR) as usize),
        (rd::NEW_CHARACTER_LOADS.ptr(b).addr, 4),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let cdobj = data;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip(READ_MANAGE_CALLEES) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, READ_MANAGE_STUBS + 0x40 * k, stub(k), n, 0);
        }
        for (k, (slot, n)) in (19u32..).zip([(0u32, 4u8), (0x10, 1), (0x14, 1), (0x58, 1)]) {
            logging_stub(o, 0, READ_MANAGE_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(READ_MANAGE_STUBS + 0x40 * k);
            o.write(cdobj + slot, &f.to_le_bytes()).unwrap();
        }
        saved.push(mine);
    }
    let started = start_stubs(a, b, erec);
    // the CD object, the file's name, the formation (byte 1 the reverb,
    // 2 the footsteps), the scene's field and group (0x01e60046 or near)
    poke(a, b, stub(0) + 4, &cdobj.to_le_bytes());
    poke(a, b, data + 0x60, b"btl_name_01\0");
    poke(a, b, stub(22) + 4, &(data + 0x60).to_le_bytes());
    poke(a, b, stub(17) + 4, &(out + 0xc0).to_le_bytes());
    let formation = data + 0x80;
    poke(
        a,
        b,
        formation,
        &[rng.byte(), pick!(rng, &[0, 1, 1, 2]) as u8, rng.byte(), 0],
    );
    set(a, b, ffx_battle::state::bridge::VOICE_EVENT, formation);
    poke(a, b, data + 0x90, &0x01e6u16.to_le_bytes());
    poke(a, b, data + 0x94, &[pick!(rng, &[0x46, 0x46, 0x45]) as u8]);
    set(a, b, ffx_battle::state::bridge::SCENE_FIELD, data + 0x90);
    set(a, b, ffx_battle::state::bridge::SCENE_GROUP, data + 0x94);
    // the actors: present or not, their slot, model and weapons, ids; half
    // the time none still loading
    let loading = rng.below(2) == 0;
    for i in 0..ACTORS {
        poke(
            a,
            b,
            w.actor(i) + 0x433,
            &[u8::from(loading && rng.below(10) == 0)],
        );
    }
    for i in 0..0x1c {
        let p = w.actor(i);
        poke(a, b, p + 0xdc8, &[u8::from(rng.below(3) != 0)]);
        poke(a, b, p + 0xc, &[pick!(rng, &[i, i, rng.below(0x12)]) as u8]);
        poke(a, b, p + 0x24, &[u8::from(rng.below(3) != 0)]);
        let id = pick!(rng, &[0xffff, rng.below(0x400), rng.below(0x400)]) as u16;
        poke(a, b, p + 0xe, &id.to_le_bytes());
        poke(a, b, p + 0x407, &[u8::from(rng.below(3) == 0)]);
    }
    // the directions and the monsters' places need sane positions
    let sane = placed(a, b, w, out);
    let managers = ffx_battle::minor::bridge::ATEL_MANAGERS.ptr(b).addr;
    poke(a, b, managers, &pick!(rng, &[0, 1, 6, 7, 8]).to_le_bytes());
    let mut status = pick!(
        rng,
        &[
            4, 4, 4, 5, 5, 6, 7, 7, 7, 8, 9, 9, 0xa, 0xb, 0xc, 0xc, 0xd, 0xe, 0xf, 0xf, 0x10, 0x11,
            0x13, 0x13, 0x14, 0x14, 0x14, 0x15, 0x15, 0x16, 0x16, 0x16, 0x17, 0x21, 0x22, 0x22,
            0x23, 3, 0, 0xff, 0x80
        ],
    );
    for step in 0..3 {
        if step > 0 {
            status = u32::from(
                b.read(ffx_battle::state::bridge::STATUS.ptr(b).addr, 1)
                    .unwrap()[0],
            );
        }
        let to_party = |s: u32| matches!(s, 7 | 9 | 0xa | 0x22);
        if !sane && to_party(status) {
            status = 0xb;
        }
        set(a, b, ffx_battle::state::bridge::STATUS, status as u8 as i8);
        // the debug wait (on, frames), the hold, the first visit
        set(a, b, rd::DEBUG_READ_WAIT, u8::from(rng.below(5) == 0));
        set(a, b, rd::READ_WAIT, pick!(rng, &[0, 0, 1, 0x3c]) as u8);
        set(
            a,
            b,
            rd::READ_HOLD,
            pick!(rng, &[0, 0, 0, 1, 2, 0x5a, u32::MAX]).cast_signed(),
        );
        let first = rng.below(2) == 0;
        set(
            a,
            b,
            ffx_battle::save::bridge::SCENARIO,
            if first || rng.below(4) == 0 {
                0x9ce
            } else {
                0x9cd
            },
        );
        poke(
            a,
            b,
            stub(2) + 4,
            &(if first { 0x223f } else { 0x2240u32 }).to_le_bytes(),
        );
        // the script callbacks, the language, the scene's result, the
        // renderer, the map's flags
        poke(
            a,
            b,
            stub(1) + 4,
            &u32::from(rng.below(3) == 0).to_le_bytes(),
        );
        poke(
            a,
            b,
            stub(5) + 4,
            &u32::from(rng.below(2) == 0).to_le_bytes(),
        );
        let scene = if sane {
            pick!(rng, &[0xa, 0, 3])
        } else {
            pick!(rng, &[0, 3])
        };
        poke(a, b, stub(8) + 4, &scene.to_le_bytes());
        poke(
            a,
            b,
            stub(12) + 4,
            &pick!(rng, &[0, 0, 0x20, 0x40, 0x60, 0x80]).to_le_bytes(),
        );
        let map_read = if sane {
            u32::from(rng.below(2) == 0)
        } else {
            1
        };
        set(a, b, ffx_battle::misc::bridge::MAP_READ, map_read);
        set(a, b, rd::MAP_STARTED, u8::from(rng.below(2) == 0));
        set(
            a,
            b,
            ffx_battle::exe::bridge::BATTLE_MAP,
            rng.next() as u16,
        );
        set(
            a,
            b,
            rd::FILE_NUMBER,
            pick!(rng, &[0x101, 0x102, 0x103, 0x104, 0x105, rng.below(0x400)]),
        );
        set(a, b, rd::EXE_MODE, pick!(rng, &[0, 0, 1, 0xff]) as u8 as i8);
        set(a, b, ffx_battle::exe::bridge::FILE_BUFFER, out + 0x80);
        // the cues, the character loads, first strike, counts and rules
        set(
            a,
            b,
            ffx_battle::state::bridge::READ_CUE_LEVEL,
            u8::from(rng.below(2) == 0),
        );
        set(
            a,
            b,
            rd::NEW_CHARACTER_LOADS,
            pick!(rng, &[u32::MAX, 0, 1, 3]).cast_signed(),
        );
        set(
            a,
            b,
            rd::FIRST_ATTACK,
            pick!(rng, &[0, 1, 2, 2, 3]) as u8 as i8,
        );
        set(
            a,
            b,
            rd::FIRST_ATTACK_SET,
            pick!(rng, &[0xff, 0xff, 0, 1, 2]) as u8 as i8,
        );
        set(a, b, rd::FIELD_MODE_B, pick!(rng, &[0, 1, 1, 2]));
        set(a, b, rd::START_READY, rng.byte());
        set(
            a,
            b,
            ffx_battle::save::bridge::BATTLE_COUNT,
            pick!(rng, &[0, 7, 0x0fff_fffe, 0x0fff_ffff, u32::MAX]),
        );
        set(
            a,
            b,
            ffx_battle::save::bridge::STEAL_RULE,
            pick!(rng, &[0, 1, 2]) as u8,
        );
        set(a, b, rd::FIRST_SCRIPT, rng.next());
        set(a, b, rd::SCENE_SCRIPT, rng.next());
        set(
            a,
            b,
            ffx_battle::battle_end::bridge::MESSAGE_WINDOW,
            pick!(rng, &[0, 0, 1]).cast_signed(),
        );
        set(
            a,
            b,
            ffx_battle::state::bridge::SEP_TRANS,
            pick!(rng, &[0, 0, 0, 1]),
        );
        twin(
            &format!("MsBtlReadManage {status:#x}"),
            a,
            b,
            0x0078_3020,
            &[],
            0,
            |m| {
                dm(m, |m| {
                    let mut s = Script {
                        call: &mut call,
                        signal: &mut signal,
                        off: &mut off,
                    };
                    rd::ms_btl_read_manage(m, &mut s, &h);
                });
                0
            },
        );
        local_words(a, b);
        rect_words(a, b);
        same(&format!("MsBtlReadManage {status:#x}"), a, b);
    }
    unstub(a, b, &started);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
    zero_stack(a);
}

/// `MsBattleInit`'s leaves' callees of other modules (`ffx_battle::init`,
/// round 21), with their argument counts; then the CD object's `+0x10`,
/// `+0x14`, `+0` (logged with three arguments behind a stub that records
/// the request's `+0xc`).
const INIT_CALLEES: [(u32, u8); 13] = {
    use ffx_battle::init::bridge as ini;
    [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (ini::USER_MALLOC, 1),
        (ini::USER_FREE, 1),
        (ini::SG_MEM_SET_ALIGN, 1),
        (ini::GET_EXCEL_BIN_SIZE, 2),
        (ini::MS_BATTLE_ERROR, 1),
        (ini::IS_PTR32_NULL, 1),
        (ini::SET_PTR32, 1),
        (ini::GET_PTR32_FUNC, 1),
        (ffx_battle::getmem::bridge::MS_GET_CHECK_ENGLISH, 0),
        (ini::YN_PSIM_TEST, 0),
        (ini::YN_SEA_CREATE, 0),
        (ini::YI_DISPOSE_ANIM_D_OBJ, 2),
    ]
};
const INIT_STUBS: u32 = READ_MANAGE_STUBS + 0x40 * 24;

/// Code no test runs for round 22's stubs (0x40 bytes each, 83 of them):
/// `Ms_Gui_BattleChr2Proc` (`0x007de790`, 5369 bytes), a debug window's
/// procedure no code calls (the window table holds it). The area from
/// [`SPECIAL_STUBS`] to `Ms_Gui_BattleConfigProc`'s end (`0x007c86e0`) is
/// used up to [`INIT_STUBS`]` + 59`.
const GUI_STUBS: u32 = 0x007d_e790;
/// `MsBtlReadSetScene`'s callees (round 22) as logging stubs:
/// `TOFTCTransfer(font)`, `MsSetRamChrData()`, `MsBtlBridgeEvent2Btl()`,
/// `AtelEventRemoveForBattle(map)`, `Sg_SetPlayer(n)`,
/// `Sg_IdSetEffectTableIndex(i)`, `TkGetNowEventJumpMapNo()` (last, its
/// result drawn).
const SET_SCENE_CALLEES: [(u32, u8); 7] = {
    use ffx_battle::read::bridge as rd;
    [
        (rd::TO_FTC_TRANSFER, 1),
        (rd::MS_SET_RAM_CHR_DATA, 0),
        (rd::MS_BTL_BRIDGE_EVENT2BTL, 0),
        (rd::ATEL_EVENT_REMOVE_FOR_BATTLE, 1),
        (ffx_battle::exe::bridge::SG_SET_PLAYER, 1),
        (ffx_battle::free::bridge::SG_ID_SET_EFFECT_TABLE_INDEX, 1),
        (rd::TK_GET_NOW_EVENT_JUMP_MAP_NO, 0),
    ]
};

/// `MsBattleInit`'s leaves (round 21, `ffx_battle::init`): the memory
/// check, the gains record, the status effects' numbers (on a table of our
/// own and the game's), the field items, the weapon names and key items,
/// the menu texts (kernel and CD ones, the request block `data`'s), the
/// start texts and the special stage's kinds. Callees as
/// [`INIT_CALLEES`]' logging stubs (records from `rec`), the heap handing
/// out `heap` (0x1000 bytes); the globals they change put back after.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
fn init_leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, heap: u32, data: u32) {
    use ffx_battle::init::bridge as ini;
    macro_rules! pick {
        ($rng:expr, $v:expr $(,)?) => {{
            let v: &[u32] = $v;
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let mut call = game_call_o;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x00c4_2000), 0x500),
        (b.rebase(0x0231_0c00), 0x900),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let cdobj = data;
    let request = data + 0x40;
    let table = data + 0x80;
    let seen = stub(16) + 0xc0;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip(INIT_CALLEES) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, INIT_STUBS + 0x40 * k, stub(k), n, 0);
        }
        for (k, (slot, n)) in (13u32..).zip([(0x10u32, 1u8), (0x14, 1), (0, 3)]) {
            logging_stub(o, 0, INIT_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(INIT_STUBS + 0x40 * k);
            o.write(cdobj + slot, &f.to_le_bytes()).unwrap();
        }
        // the read: the request's +0xc (when there is one) to `seen`, then
        // the logging stub
        let at = o.rebase(INIT_STUBS + 0x40 * 16);
        let log = o.rebase(INIT_STUBS + 0x40 * 15);
        let mut code = vec![
            0x8b, 0x44, 0x24, 0x10, 0x85, 0xc0, 0x74, 0x08, 0x8b, 0x40, 0x0c, 0xa3,
        ];
        code.extend(seen.to_le_bytes());
        code.push(0xe9);
        code.extend(log.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
        o.write(at, &code).unwrap();
        o.write(seen, &[0; 4]).unwrap();
        o.write(cdobj, &at.to_le_bytes()).unwrap();
        saved.push(mine);
    }
    poke(a, b, stub(0) + 4, &cdobj.to_le_bytes());
    poke(a, b, stub(1) + 4, &heap.to_le_bytes());
    poke(a, b, stub(2) + 4, &rng.next().to_le_bytes());
    poke(a, b, stub(8) + 4, &table.to_le_bytes());
    fill(a, b, rng, heap, 0x100);
    // the memory check: booked below, at or past the top
    for _ in 0..3 {
        let top = pick!(rng, &[0x7_0000, 0x40, rng.below(0x8_0000)]).cast_signed();
        let booked = pick!(
            rng,
            &[
                0x40,
                0x6_ffc0,
                0x6_fff0,
                0x7_0000,
                0x7_0040,
                rng.below(0x8_0000)
            ]
        );
        set(
            a,
            b,
            ffx_battle::state::bridge::MEM_CHECK,
            booked.cast_signed(),
        );
        twin(
            "MsMemCheckSum",
            a,
            b,
            0x0079_9f20,
            &[top.cast_unsigned()],
            ALL,
            |m| ini::ms_mem_check_sum(m, &mut call, top),
        );
        same("MsMemCheckSum", a, b);
    }
    // the gains record
    fill(
        a,
        b,
        rng,
        ffx_battle::flow::bridge::COUNTS_BATTLE.ptr(b).addr,
        0x1b0,
    );
    twin("MsBtlGetInit", a, b, 0x0079_8b60, &[], ALL, |m| {
        ini::ms_btl_get_init(m)
    });
    same("MsBtlGetInit", a, b);
    let kind = pick!(rng, &[0, 7, 0xf, rng.next()]);
    twin(
        "MsBtlGetClear_007993e0",
        a,
        b,
        0x0079_93e0,
        &[table, kind],
        ALL,
        |m| ini::ms_btl_get_clear(m, table, kind),
    );
    same("MsBtlGetClear_007993e0", a, b);
    // the status effects' numbers: a table of our own (skipped entries,
    // the end at one of three zero bytes), from near the limit
    let mut t = Vec::new();
    for _ in 0..rng.below(12) {
        let e = [
            pick!(rng, &[0xff, 0, 1, rng.byte().into()]) as u8,
            pick!(rng, &[0, 0, 1, rng.byte().into()]) as u8,
            rng.byte(),
            rng.byte(),
        ];
        if e[0] == 0 && e[1] == 0 && e[2] == 0 {
            continue;
        }
        t.extend(e);
    }
    t.extend([0, 0, 0, rng.byte()]);
    poke(a, b, table, &t);
    let n = pick!(rng, &[0, 5, 0x14, 0x18, 0x19]);
    twin(
        "MsEtEffectNumber_0079f1c0",
        a,
        b,
        0x0079_f1c0,
        &[table, n],
        ALL,
        |m| ini::ms_et_effect_number(m, &mut call, table, n),
    );
    same("MsEtEffectNumber_0079f1c0", a, b);
    twin("MsEtEffectInit", a, b, 0x0079_e670, &[], ALL, |m| {
        ini::ms_et_effect_init(m, &mut call)
    });
    same("MsEtEffectInit", a, b);
    // the field items, the weapon names and key items (a table made or
    // there), sizes small
    poke(a, b, stub(15) + 4, &rng.below(0x400).to_le_bytes());
    poke(a, b, stub(14) + 4, &rng.below(0x400).to_le_bytes());
    twin("MsFieldItemRead", a, b, 0x0079_9020, &[], ALL, |m| {
        ini::ms_field_item_read(m, &mut call)
    });
    same("MsFieldItemRead", a, b);
    for (name, f) in [
        ("MsWeaponNameRead", 0x007a_2600u32),
        ("MsImportantRead", 0x0079_08c0),
    ] {
        poke(a, b, stub(4) + 4, &rng.below(0x100).to_le_bytes());
        let g = if f == 0x007a_2600 {
            ffx_battle::menu::bridge::WEAPON_NAMES
        } else {
            ffx_battle::save::bridge::KERNEL_IMPORTANT
        };
        set(a, b, g, [0, table + 0x100][rng.below(2) as usize]);
        twin(name, a, b, f, &[], ALL, |m| {
            if f == 0x007a_2600 {
                ini::ms_weapon_name_read(m, &mut call)
            } else {
                ini::ms_important_read(m, &mut call)
            }
        });
        same(name, a, b);
    }
    // the menu texts: kernel and CD ones, out of range, block made or there
    for _ in 0..4 {
        let i = pick!(
            rng,
            &[
                rng.below(0x11),
                rng.below(0x11),
                2,
                0xd,
                0xf,
                0x11,
                rng.next()
            ]
        );
        let load = rng.below(2);
        poke(a, b, stub(4) + 4, &rng.below(0x100).to_le_bytes());
        poke(
            a,
            b,
            stub(6) + 4,
            &u32::from(rng.below(2) == 0).to_le_bytes(),
        );
        poke(a, b, stub(7) + 4, &rng.next().to_le_bytes());
        poke(a, b, stub(9) + 4, &pick!(rng, &[0, 0, 1]).to_le_bytes());
        twin("MsMenuReadText", a, b, 0x0078_fef0, &[i, load], ALL, |m| {
            ini::ms_menu_read_text(m, &mut call, i, load, request)
        });
        let r = b.read(request, 0x10).unwrap();
        a.write(request, &r).unwrap();
        same(&format!("MsMenuReadText {i:#x} {load}"), a, b);
    }
    // the start list: the game's (kernel texts only) or CD texts, whose
    // size `load` decides while English
    if rng.below(2) == 0 {
        poke(a, b, b.rebase(ini::START_TEXTS), &[2, 0xd, 2, 0xff]);
        poke(a, b, stub(9) + 4, &1u32.to_le_bytes());
    }
    for _ in 0..2 {
        let k = rng.below(10);
        twin(
            "MsMenuReadStartText_00782b70",
            a,
            b,
            0x0078_2b70,
            &[k],
            ALL,
            |m| ini::ms_menu_read_start_text(m, &mut call, k, request),
        );
        let r = b.read(request, 0x10).unwrap();
        a.write(request, &r).unwrap();
        same(&format!("MsMenuReadStartText_00782b70 {k}"), a, b);
    }
    // the special stage's kinds (its work block not cleared before)
    let kind = pick!(rng, &[0, 1, 1, 2, 3, 4, u32::MAX]);
    fill(a, b, rng, heap, 0x870);
    poke(a, b, stub(12) + 4, &rng.next().to_le_bytes());
    twin(
        "MsBattleSpecialInit",
        a,
        b,
        0x0079_de60,
        &[kind],
        ALL,
        |m| ini::ms_battle_special_init(m, &mut call, kind),
    );
    same(&format!("MsBattleSpecialInit {kind}"), a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsSetSaveStartGame`'s callees of other modules and `MsWeaponNameNum`
/// (round 21), with their argument counts; then the CD object's `+0x10`,
/// `+0x14`, `+0`, `+8`.
const SAVE_START_CALLEES: [(u32, u8); 14] = {
    use ffx_battle::init::bridge as ini;
    [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (ini::DBG_PRINTF, 1),
        (ini::SG_MALLOC_A, 4),
        (ini::SG_FREE_A, 2),
        (ini::USER_MALLOC, 1),
        (ini::USER_FREE, 1),
        (ini::GET_EXCEL_BIN_SIZE, 2),
        (ini::TO_GET_FFX_LANG, 0),
        (ini::TK_GET_TOTAL_SYNC_V_COUNTER, 0),
        (ini::MS_RAND_MIZE, 1),
        (ini::MS_SET_SAVE_CONFIG_ENGLISH, 1),
        (ini::MS_WEAPON_NAME_NUM, 1),
        (ini::TK_INIT_SAVE_WORK, 0),
        (ini::EI_ABM_DATA_INIT, 1),
    ]
};
const SAVE_START_STUBS: u32 = INIT_STUBS + 0x40 * 17;

/// An `MsGetExcelData` table of one segment: `n` records of `stride` bytes
/// (random), the strings at the records.
fn excel_table(rng: &mut Xorshift, n: u16, stride: u16) -> Vec<u8> {
    let mut t = vec![0u8; 0x20];
    t[0] = 1;
    t[10..12].copy_from_slice(&(n - 1).to_le_bytes());
    t[12..14].copy_from_slice(&stride.to_le_bytes());
    t[16..20].copy_from_slice(&0x20u32.to_le_bytes());
    t.extend((0..usize::from(n) * usize::from(stride)).map(|_| rng.byte()));
    t
}

/// `MsSetSaveStartGame` (round 21): a new game's save from tables of our
/// own (the CD's blocks `block`, `ply_save` in `buf`; 0x1000 bytes each),
/// weapons ending by a repeat or the 0x23 limit, owners below 7 or not,
/// starting items, a debug build or not. Callees as
/// [`SAVE_START_CALLEES`]' logging stubs (records from `rec`, the CD object
/// at `data`); the save, the weapon names' table and the debug flag put
/// back after.
#[allow(clippy::cast_possible_truncation)]
fn save_start(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    block: u32,
    buf: u32,
    data: u32,
) {
    use ffx_battle::init::bridge as ini;
    let mut call = game_call_o;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
        (ini::FFX_IS_DEBUG.ptr(b).addr, 1),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, (f, n)) in (0u32..).zip(SAVE_START_CALLEES) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, SAVE_START_STUBS + 0x40 * k, stub(k), n, 0);
        }
        for (k, (slot, n)) in (14u32..).zip([(0x10u32, 1u8), (0x14, 1), (0, 4), (8, 1)]) {
            logging_stub(o, 0, SAVE_START_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(SAVE_START_STUBS + 0x40 * k);
            o.write(data + slot, &f.to_le_bytes()).unwrap();
        }
        saved.push(mine);
    }
    // the weapons' file in the CD's blocks (also the menu's and party's
    // first records), owners below 7 or not
    let n = [5u16, 0x22, 0x23, 0x30][rng.below(4) as usize];
    let mut wep = excel_table(rng, n, 0x16);
    for k in 0..usize::from(n) {
        wep[0x20 + 0x16 * k + 4] = [0, 3, 6, 7, 0xff][rng.below(5) as usize];
    }
    poke(a, b, block, &wep);
    // the characters' ply_save, names at small offsets
    let mut ply = excel_table(rng, 18, 0x94);
    for k in 0..18 {
        let at = 0x20 + 0x94 * k;
        ply[at..at + 2].copy_from_slice(&(rng.below(0x900) as u16).to_le_bytes());
    }
    poke(a, b, buf, &ply);
    // the weapon names (MsWeaponName's table)
    let names = battle_common::world::fake_records(a, b, rng, 0x100, 0x40);
    set(a, b, ffx_battle::menu::bridge::WEAPON_NAMES, names);
    poke(a, b, stub(0) + 4, &data.to_le_bytes());
    poke(a, b, stub(2) + 4, &block.to_le_bytes());
    poke(a, b, stub(4) + 4, &buf.to_le_bytes());
    poke(a, b, stub(6) + 4, &(ply.len() as u32).to_le_bytes());
    for k in [7u32, 8, 11, 13] {
        poke(a, b, stub(k) + 4, &rng.next().to_le_bytes());
    }
    set(a, b, ini::ALL_WEAPONS, u8::from(rng.below(2) == 0));
    set(a, b, ini::FFX_IS_DEBUG, i8::from(rng.below(3) == 0));
    // the starting items (after the party's record is copied over them,
    // the save's own: so the record's bytes), IDs 0xffff or not
    for j in 0..8 {
        let id: u16 =
            [0xffff, 0x2000 + rng.below(0x70) as u16, rng.next() as u16][rng.below(3) as usize];
        let at = 0x20 + 0x44 + 2 * j;
        if at + 2 <= wep.len() {
            wep[at..at + 2].copy_from_slice(&id.to_le_bytes());
        }
    }
    poke(a, b, block, &wep);
    twin("MsSetSaveStartGame", a, b, 0x0078_6b00, &[], ALL, |m| {
        ini::ms_set_save_start_game(m, &mut call)
    });
    same(&format!("MsSetSaveStartGame {n:#x}"), a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsBattleInit`'s own callees of the game (round 21): the inits, the
/// kernel tables' loader (a counting stub), the encounter list, the debug
/// build's party and positions, `MsBattleExe`, the scene, the event battle,
/// effects, the label, the debug scene and fills, with their logged
/// argument counts.
const BATTLE_INIT_CALLEES: [(u32, u8); 29] = {
    use ffx_battle::init::bridge as ini;
    [
        (ini::FFX_GET_ASIA_MODE, 1),
        (ini::START_INITS[0], 0),
        (ini::START_INITS[1], 0),
        (ini::START_INITS[2], 0),
        (ini::START_INITS[3], 0),
        (ini::START_INITS[4], 0),
        (ini::START_INITS[5], 0),
        (ini::START_INITS[6], 0),
        (ini::START_INITS[7], 0),
        (ini::START_INITS2[0], 0),
        (ini::START_INITS2[1], 0),
        (ini::START_INITS2[2], 0),
        (ini::START_INITS2[3], 0),
        (ini::START_INITS2[4], 0),
        (ini::MS_MEM_CHECK_INIT, 0),
        (ini::MS_BTL_LIST_INIT, 0),
        (ini::FFX_IS_DEBUG_FN, 0),
        (ini::MS_SET_SAVE_PARTY, 0),
        (ini::MS_BTL_POS_INIT, 0),
        (ini::MS_BATTLE_EXE, 4),
        (ini::MS_BTL_READ_SET_SCENE, 0),
        (ini::MS_BATTLE_INIT_AT_EVENT, 1),
        (ini::SG_EFFECT_ROUTINE_ENABLE, 1),
        (ini::MS_BATTLE_LABEL_EXE, 3),
        (ini::MS_DEBUG_SET_SCENE, 4),
        (ini::MS_DEBUG_FULL[0], 0),
        (ini::MS_DEBUG_FULL[1], 0),
        (ini::MS_DEBUG_FULL[2], 0),
        (ini::MS_DEBUG_FULL[3], 0),
    ]
};
const BATTLE_INIT_RECS: u32 = 59;

/// `MsBattleInit` (round 21) on stubs: every game callee of it and of its
/// ported leaves ([`INIT_CALLEES`], [`SAVE_START_CALLEES`] with
/// `dbgPrintf`'s first argument only, [`BATTLE_INIT_CALLEES`];
/// `Kernel_LoadTables` counting, the CD object's methods at `data` with
/// the read recording the request's `+0xc`), in `INIT_STUBS` on (both
/// earlier tests reinstall theirs). The heap hands out `heap` (holding a
/// `ply_save` table), the CD's blocks `block` (a weapons table), the
/// pointer table `dest`; the tables the leaves fill are made fresh (their
/// pointers 0) so nothing of the game's is overwritten. The battle's
/// globals put back after.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn battle_init(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    [heap, block, dest, data]: [u32; 4],
) {
    use ffx_battle::init::bridge as ini;
    let mut call = game_call_o;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x00c4_2000), 0x500),
        (b.rebase(0x0231_0c00), 0x900),
        (ini::FFX_IS_DEBUG.ptr(b).addr, 1),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let mut callees: Vec<(u32, u8)> = Vec::new();
    for (f, n) in INIT_CALLEES
        .into_iter()
        .chain(SAVE_START_CALLEES)
        .chain(BATTLE_INIT_CALLEES)
    {
        let n = if f == ini::DBG_PRINTF { 1 } else { n };
        if !callees.iter().any(|&(g, _)| g == f) {
            callees.push((f, n));
        }
    }
    let k_load = u32::try_from(callees.len()).unwrap();
    let k_cd = k_load + 1;
    assert!(k_cd + 6 <= BATTLE_INIT_RECS);
    let seen = stub(k_cd + 4) + 0xc0;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for &(f, _) in callees.iter().chain([&(ini::KERNEL_LOAD_TABLES, 0)]) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
        }
        for (k, &(f, n)) in (0u32..).zip(&callees) {
            logging_stub(o, f, INIT_STUBS + 0x40 * k, stub(k), n, 0);
        }
        counting_stub(
            o,
            ini::KERNEL_LOAD_TABLES,
            INIT_STUBS + 0x40 * k_load,
            stub(k_load),
        );
        for (k, (slot, n)) in (k_cd..).zip([(0x10u32, 1u8), (0x14, 1), (0, 3), (8, 1)]) {
            logging_stub(o, 0, INIT_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(INIT_STUBS + 0x40 * k);
            o.write(data + slot, &f.to_le_bytes()).unwrap();
        }
        // the directory stub: the size stub then returns the directory
        // << 12 (sizes by directory; 0xd000 and up read signed)
        let dir_at = o.rebase(INIT_STUBS + 0x40 * (k_cd + 5));
        let dir_log = o.rebase(INIT_STUBS + 0x40 * k_cd);
        let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0xc1, 0xe0, 0x0c, 0xa3];
        code.extend((stub(k_cd + 1) + 4).to_le_bytes());
        code.push(0xe9);
        code.extend(
            dir_log
                .wrapping_sub(dir_at + code.len() as u32 + 4)
                .to_le_bytes(),
        );
        o.write(dir_at, &code).unwrap();
        let at = o.rebase(INIT_STUBS + 0x40 * (k_cd + 4));
        let log = o.rebase(INIT_STUBS + 0x40 * (k_cd + 2));
        let mut code = vec![
            0x8b, 0x44, 0x24, 0x10, 0x85, 0xc0, 0x74, 0x08, 0x8b, 0x40, 0x0c, 0xa3,
        ];
        code.extend(seen.to_le_bytes());
        code.push(0xe9);
        code.extend(log.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
        o.write(at, &code).unwrap();
        o.write(seen, &[0; 4]).unwrap();
        o.write(data, &at.to_le_bytes()).unwrap();
        o.write(data + 0x10, &dir_at.to_le_bytes()).unwrap();
        saved.push(mine);
    }
    let ret = |f: u32| -> u32 {
        let k = callees.iter().position(|&(g, _)| g == f).unwrap();
        stub(u32::try_from(k).unwrap()) + 4
    };
    // the tables: a weapons file in the CD's blocks, ply_save in the heap
    let n = [5u16, 0x23, 0x30][rng.below(3) as usize];
    let mut wep = excel_table(rng, n, 0x16);
    for k in 0..usize::from(n) {
        wep[0x20 + 0x16 * k + 4] = [0, 6, 7][rng.below(3) as usize];
    }
    poke(a, b, block, &wep);
    let mut ply = excel_table(rng, 18, 0x94);
    for k in 0..18 {
        let at = 0x20 + 0x94 * k;
        ply[at..at + 2].copy_from_slice(&(rng.below(0x900) as u16).to_le_bytes());
    }
    poke(a, b, heap, &ply);
    poke(
        a,
        b,
        ret(ffx_battle::getmem::bridge::GET_CD),
        &data.to_le_bytes(),
    );
    poke(a, b, ret(ini::USER_MALLOC), &heap.to_le_bytes());
    poke(a, b, ret(ini::SG_MALLOC_A), &block.to_le_bytes());
    poke(a, b, ret(ini::GET_PTR32_FUNC), &dest.to_le_bytes());
    poke(
        a,
        b,
        ret(ini::GET_EXCEL_BIN_SIZE),
        &(ply.len() as u32).to_le_bytes(),
    );
    poke(
        a,
        b,
        ret(ini::IS_PTR32_NULL),
        &u32::from(rng.below(2) == 0).to_le_bytes(),
    );
    poke(a, b, stub(k_cd + 1) + 4, &rng.below(0x8000).to_le_bytes());
    for f in [
        ini::FFX_GET_ASIA_MODE,
        ini::SET_PTR32,
        ini::TO_GET_FFX_LANG,
        ini::TK_GET_TOTAL_SYNC_V_COUNTER,
        ini::MS_WEAPON_NAME_NUM,
    ] {
        poke(a, b, ret(f), &rng.next().to_le_bytes());
    }
    for f in [ini::MS_GET_CHECK_ENGLISH, ini::FFX_IS_DEBUG_FN] {
        poke(a, b, ret(f), &u32::from(rng.below(2) == 0).to_le_bytes());
    }
    // Kernel_LoadTables: none, one or three more
    let more: [u32; 4] = [[0, 0, 0, 0], [1, 0, 0, 0], [1, 1, 1, 0]][rng.below(3) as usize];
    let t: Vec<u8> = more.iter().flat_map(|w| w.to_le_bytes()).collect();
    poke(a, b, stub(k_load) + 8, &t);
    // the leaves' tables made fresh, the memory word, the toggles
    set(a, b, ffx_battle::menu::bridge::WEAPON_NAMES, 0);
    set(a, b, ffx_battle::save::bridge::KERNEL_IMPORTANT, 0);
    set(
        a,
        b,
        ini::MEM_WORD,
        [0, 0, rng.next()][rng.below(3) as usize],
    );
    set(
        a,
        b,
        ffx_battle::state::bridge::EXE,
        [0, 1, -1][rng.below(3) as usize],
    );
    set(a, b, ini::DEBUG_FULL, u8::from(rng.below(4) == 0));
    set(a, b, ini::FFX_IS_DEBUG, i8::from(rng.below(3) == 0));
    set(a, b, ini::ALL_WEAPONS, u8::from(rng.below(2) == 0));
    set(
        a,
        b,
        ffx_battle::state::bridge::MEM_CHECK,
        rng.below(0x8_0000).cast_signed(),
    );
    twin("MsBattleInit", a, b, 0x0078_1700, &[], ALL, |m| {
        ini::ms_battle_init(m, &mut call, data + 0x40)
    });
    let r = b.read(data + 0x40, 0x10).unwrap();
    a.write(data + 0x40, &r).unwrap();
    same("MsBattleInit", a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsBattleProcess`'s callees (round 21, `ffx_battle::process`), all
/// through the game, with their logged argument counts.
fn battle_process_callees() -> Vec<(u32, u8)> {
    use ffx_battle::process::bridge as p;
    let mut v = vec![
        (p::MS_BTL_READ_MANAGE, 0),
        (p::MS_DEBUG_CAMERA_SLEEP, 1),
        (p::FFX_IS_DEBUG, 0),
        (p::TK_GET_CONTROLER_HIST, 3),
        (p::MS_DEBUG_GET_FIELD_NUM, 0),
        (p::DBG_PRINTF, 3),
        (p::TO_ALL_CLEAR_MESSAGE_WINDOW, 0),
        (p::MS_BATTLE_LABEL_EXE, 3),
        (p::MS_BATTLE_EXE, 4),
        (p::MS_EVENT_EFFECT_PROCESS, 0),
        (p::TO_DEBUG_GET_PAD_PRINT_FLAG, 0),
        (p::MS_BATTLE_PROCESS_DEBUG_STATUS, 0),
        (p::MS_SEP_3D_PROCESS, 0),
        (p::MS_MENU_CLOSE_TITLE_WINDOW, 1),
        (p::MS_CHECK_PLY_BTL_POS_ALL, 0),
        (p::MS_RESET_NORMAL_CALC, 0),
        (p::MS_SEP_TRANS_PROCESS, 0),
        (p::MS_BATTLE_PROCESS_ORDER, 0),
        (p::MS_BATTLE_PROCESS_CHR_DAMAGE, 0),
        (p::MS_SET_DEATH_ALL, 0),
        (p::MS_STONE_EFFECT_PROCESS, 0),
        (p::MS_CHECK_CHR_HIT_MOTION_ALL, 0),
        (p::MS_BATTLE_PROCESS_STATUS, 0),
        (p::MS_BATTLE_PROCESS_INPUT_CUE, 0),
        (p::TK_CHECK_SOFT_WARE_RESET, 0),
        (p::MS_CHECK_SOFT_RESET_OK, 0),
        (p::OPD_EXIT_FROM_OVL, 0),
        (p::MS_NOW_MUSIC_STOP, 0),
        (p::MS_CAMERA_INIT, 0),
        (p::MS_SET_SAVE_DATA, 0),
        (p::MS_BATTLE_FREE, 0),
        (p::ATEL_BATTLE_END_CALLBACK, 4),
    ];
    v.extend(p::FRAME_STEPS.map(|f| (f, 0)));
    v.extend(p::DEBUG_RESTART_STEPS.map(|f| (f, 0)));
    for (f, args) in p::END_STEPS {
        if !v.iter().any(|&(g, _)| g == f) {
            v.push((f, u8::try_from(args.len()).unwrap()));
        }
    }
    v
}

/// `MsBattleProcess` (round 21): a frame in and out of battle, every
/// callee a logging stub (from `INIT_STUBS` on, records from `rec`;
/// results drawn: the pad with and without the debug combination, the
/// debug build, the waits), the frame's counters and holds drawn, the
/// scene's field and group at `data`. The battle's globals, the leader's
/// HP word and the draw environment flag put back after.
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
fn battle_process(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    rec: u32,
    data: u32,
) {
    use ffx_battle::process::bridge as p;
    macro_rules! pick {
        ($rng:expr, $v:expr $(,)?) => {{
            let v: &[u32] = $v;
            v[$rng.below(u32::try_from(v.len()).unwrap()) as usize]
        }};
    }
    let mut call = game_call_o;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x00c4_2000), 0x500),
        (p::DEF_DRAW_ENV2.ptr(b).addr, 1),
        (w.actor(0) + 0x5d0, 4),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let callees = battle_process_callees();
    assert!(callees.len() <= BATTLE_INIT_RECS as usize);
    let stub = |k: usize| rec + EXCHANGE_REC * u32::try_from(k).unwrap();
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, &(f, n)) in callees.iter().enumerate() {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(
                o,
                f,
                INIT_STUBS + 0x40 * u32::try_from(k).unwrap(),
                stub(k),
                n,
                0,
            );
        }
        saved.push(mine);
    }
    let ret = |f: u32| stub(callees.iter().position(|&(g, _)| g == f).unwrap()) + 4;
    // the stubs' results
    let combo = p::DEBUG_COMBO;
    let pad = pick!(
        rng,
        &[
            0,
            rng.below(0x1_0000),
            0x0f,
            0x4e,
            combo,
            combo | p::DEBUG_RESTART,
            0x880,
            0x880 | combo,
            0x880 | combo | 0x20,
            0x800
        ]
    );
    poke(
        a,
        b,
        ret(p::TK_GET_CONTROLER_HIST),
        &(pad | (rng.next() << 16)).to_le_bytes(),
    );
    for f in [
        p::FFX_IS_DEBUG,
        p::TO_DEBUG_GET_PAD_PRINT_FLAG,
        p::MS_CHECK_PLY_BTL_POS_ALL,
        p::TK_CHECK_SOFT_WARE_RESET,
        p::MS_CHECK_SOFT_RESET_OK,
    ] {
        poke(
            a,
            b,
            ret(f),
            &pick!(rng, &[0, 0, 1, u32::MAX]).to_le_bytes(),
        );
    }
    for f in [
        p::MS_SEP_3D_PROCESS,
        p::MS_DEBUG_GET_FIELD_NUM,
        p::MS_CHECK_CHR_HIT_MOTION_ALL,
    ] {
        poke(a, b, ret(f), &pick!(rng, &[0, 0, rng.next()]).to_le_bytes());
    }
    // the frame's state: status, the draw environment, counters, holds
    let status = pick!(rng, &[0, 0, 1, 1, 1, 0x21, 0x21, 4, 0x15]);
    set(a, b, ffx_battle::state::bridge::STATUS, status as u8 as i8);
    set(a, b, p::DEF_DRAW_ENV2, pick!(rng, &[1, 1, 0]) as u8 as i8);
    set(
        a,
        b,
        ffx_battle::state::bridge::EXE,
        pick!(rng, &[0, 1]) as u8 as i8,
    );
    poke(
        a,
        b,
        w.actor(0) + 0x5d0,
        &pick!(rng, &[0, 1, u32::MAX, 0x500]).to_le_bytes(),
    );
    set(a, b, p::PAUSE, pick!(rng, &[0, 0, 1, 5]) as u8);
    set(
        a,
        b,
        ffx_battle::state::bridge::TITLE_WINDOW,
        [pick!(rng, &[0, 1, 2, 0x80]) as u8 as i8, rng.byte() as i8],
    );
    set(
        a,
        b,
        ffx_battle::parse::bridge::PARTY_LEAVES,
        pick!(rng, &[0, 1]) as u8,
    );
    set(a, b, p::HIT_MOTION, pick!(rng, &[0, 0, 1]) as u8);
    for g in [
        ffx_battle::battle_end::bridge::END_FADE,
        ffx_battle::battle_end::bridge::END_FADE_OUT,
    ] {
        set(a, b, g, pick!(rng, &[0, 0, 1, 3]) as u8);
    }
    set(
        a,
        b,
        ffx_battle::battle_end::bridge::END_KIND,
        pick!(rng, &[0, 0, 1, 5, 6]) as u8 as i8,
    );
    set(a, b, p::NO_SAVE, pick!(rng, &[0, 1]) as u8);
    set(
        a,
        b,
        ffx_battle::state::bridge::MESSAGE_CUE_COUNT,
        pick!(rng, &[0, 0, 0, 1]) as u8,
    );
    set(
        a,
        b,
        ffx_battle::atel::bridge::CAMERA_STATE,
        pick!(rng, &[0, 0, 0, 1]) as u8,
    );
    set(
        a,
        b,
        ffx_battle::parse::bridge::COMMAND_HOLD,
        pick!(rng, &[0, 0, 0, 1]) as u8,
    );
    set(
        a,
        b,
        ffx_battle::flow::bridge::LIMIT_LEARNT,
        pick!(rng, &[0, 0, 0, 1]) as u8,
    );
    set(
        a,
        b,
        ffx_battle::action::bridge::TURN_END_ACTOR,
        pick!(rng, &[0xff, 0xff, 0xff, 3]) as u8,
    );
    set(a, b, ffx_battle::state::bridge::ENCOUNT_TYPE, rng.byte());
    poke(a, b, data, &(rng.next() as u16).to_le_bytes());
    poke(a, b, data + 4, &[rng.byte()]);
    set(a, b, ffx_battle::state::bridge::SCENE_FIELD, data);
    set(a, b, ffx_battle::state::bridge::SCENE_GROUP, data + 4);
    twin(
        &format!("MsBattleProcess {status:#x} {pad:#x}"),
        a,
        b,
        0x0079_0c10,
        &[],
        0,
        |m| {
            p::ms_battle_process(m, &mut call);
            0
        },
    );
    same(&format!("MsBattleProcess {status:#x} {pad:#x}"), a, b);
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsBtlReadSetScene` (round 22, `ffx_battle::read`): a battle file of
/// our own in `file` (0x200 bytes: four section offsets, random
/// sections), the file's name `system_01` or near it, its text and font
/// there or not, `MsBattleExe`'s mode 0 or not and the event's jump map
/// the battle's or not. Callees as [`SET_SCENE_CALLEES`]' logging stubs
/// (records from `rec`) in [`GUI_STUBS`]; `MsActionInit` and
/// `MsPushBtlPosAll` run as the originals and the ports. The battle's
/// globals and the stubbed entries put back after.
fn read_set_scene(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, file: u32) {
    use ffx_battle::read::bridge as rd;
    let mut call = game_call_o;
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for (k, &(f, n)) in (0u32..).zip(&SET_SCENE_CALLEES) {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
            logging_stub(o, f, GUI_STUBS + 0x40 * k, stub(k), n, 0);
        }
        saved.push(mine);
    }
    // the file: offsets of four sections inside it (some at its end)
    let mut bytes = vec![0u8; 0x200];
    for (i, v) in bytes.iter_mut().enumerate().skip(0x14) {
        *v = if i % 7 == 0 { 0 } else { rng.byte() };
    }
    bytes[..4].copy_from_slice(&[4, 0, 0, 0]);
    for k in 1..5 {
        let off = 0x14 + rng.below(0x1e8);
        bytes[4 * k..4 * k + 4].copy_from_slice(&off.to_le_bytes());
    }
    poke(a, b, file, &bytes);
    set(a, b, ffx_battle::exe::bridge::FILE_BUFFER, file);
    let names: [&[u8]; 6] = [
        b"system_01\0",
        b"system_01\0",
        b"system_0\0",
        b"system_011\0",
        b"\0",
        b"cdsp00_00\0",
    ];
    let name = names[rng.below(6) as usize];
    poke(a, b, b.rebase(rd::FILE_NAME), name);
    let text = [0, file + 0x100, rng.next() | 1][rng.below(3) as usize];
    let font = [0, rng.next() | 1][rng.below(2) as usize];
    set(a, b, rd::FILE_TEXT_DATA, text);
    set(a, b, rd::FILE_FONT_DATA, font);
    // a stale font and text table (cleared first)
    set(a, b, ffx_battle::misc::bridge::FONT, rng.next() | 1);
    set(a, b, rd::FILE_TEXT, rng.next() | 1);
    set(a, b, rd::EXE_MODE, [0, 0, 0, 1, -1][rng.below(5) as usize]);
    let music = [
        0x0010u16,
        0xfff0,
        0x8000,
        rng.word().to_le_bytes()[0].into(),
    ][rng.below(4) as usize];
    set(a, b, ffx_battle::exe::bridge::BATTLE_MAP, music);
    let jump = match rng.below(3) {
        0 => i32::from(music.cast_signed()).cast_unsigned(),
        1 => u32::from(music),
        _ => rng.next(),
    };
    poke(a, b, stub(6) + 4, &jump.to_le_bytes());
    for _ in 0..2 {
        twin(
            "MsBtlReadSetScene",
            a,
            b,
            rd::MS_BTL_READ_SET_SCENE,
            &[],
            ALL,
            |m| rd::ms_btl_read_set_scene(m, &mut call),
        );
        same("MsBtlReadSetScene", a, b);
    }
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsBtlReadMonster` (round 21, `ffx_battle::read`): the eight monster
/// slots from a drawn start, IDs shared or not, files there or not; the
/// IDs by a counting `MsGetChrID` (four of below 0x65, 0x65-0xb4, the
/// named three and above); the three kernel text tables our own at
/// `tables` (0x4000 bytes each, strings short and past the limits) or
/// none; the CD object at `data`, the request block at `data + 0x40`.
/// The other callees are logging stubs (records from `rec`). The battle's
/// globals and the monsters put back after.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn read_monster(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    rec: u32,
    tables: u32,
    data: u32,
) {
    use ffx_battle::read::bridge as rd;
    let mut call = game_call_o;
    let mon0 = ffx_battle::actor::bridge::ms_get_mon(b, 0);
    if !readable(a, mon0, (8 * ACTOR) as usize) {
        return;
    }
    let regions = [
        (b.rebase(0x0112_a800), 0xc800),
        (b.rebase(0x0231_0c00), 0x900),
        (mon0, (8 * ACTOR) as usize),
    ];
    let kept: Vec<Vec<u8>> = regions
        .iter()
        .map(|&(at, n)| b.read(at, n).unwrap())
        .collect();
    let callees: [(u32, u8); 6] = [
        (ffx_battle::getmem::bridge::GET_CD, 0),
        (rd::READ_BTL_MONSTER_NAME, 0),
        (rd::MON_ATTACH_DATA, 1),
        (rd::MON_LOAD_ALL, 0),
        (rd::SG_MALLOC_A, 4),
        (rd::MS_GET_CHR_ID, 1),
    ];
    let stub = |k: u32| rec + EXCHANGE_REC * k;
    let seen = stub(10) + 0xc0;
    let mut saved = Vec::new();
    for o in [&mut *a, &mut *b] {
        let mut mine = Vec::new();
        for &(f, _) in &callees {
            let at = o.rebase(f);
            mine.push((f, o.read(at, 5).unwrap()));
        }
        for (k, &(f, n)) in (0u32..).zip(&callees[..5]) {
            logging_stub(o, f, INIT_STUBS + 0x40 * k, stub(k), n, 0);
        }
        counting_stub(o, rd::MS_GET_CHR_ID, INIT_STUBS + 0x40 * 5, stub(5));
        for (k, (slot, n)) in (6u32..).zip([(0x10u32, 1u8), (0x14, 1), (0, 3)]) {
            logging_stub(o, 0, INIT_STUBS + 0x40 * k, stub(k), n, 0);
            let f = o.rebase(INIT_STUBS + 0x40 * k);
            o.write(data + slot, &f.to_le_bytes()).unwrap();
        }
        let at = o.rebase(INIT_STUBS + 0x40 * 10);
        let log = o.rebase(INIT_STUBS + 0x40 * 8);
        let mut code = vec![
            0x8b, 0x44, 0x24, 0x10, 0x85, 0xc0, 0x74, 0x08, 0x8b, 0x40, 0x0c, 0xa3,
        ];
        code.extend(seen.to_le_bytes());
        code.push(0xe9);
        code.extend(log.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
        o.write(at, &code).unwrap();
        o.write(seen, &[0; 4]).unwrap();
        o.write(data, &at.to_le_bytes()).unwrap();
        saved.push(mine);
    }
    poke(a, b, stub(0) + 4, &data.to_le_bytes());
    poke(a, b, stub(4) + 4, &(rng.next() | 0x10).to_le_bytes());
    let ids: Vec<u8> = [
        [0x10u32, 0x70, 0xfe, 0x200],
        [0x64, 0x65, 0xb4, 0xb5],
        [0x11c, 0x11f, 0xfe, 3],
        [
            rng.below(0x300),
            rng.below(0x300),
            rng.below(0x300),
            rng.below(0x300),
        ],
    ][rng.below(4) as usize]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    poke(a, b, stub(5) + 8, &ids);
    poke(
        a,
        b,
        stub(7) + 4,
        &[0u32, 0x100, 0x2000][rng.below(3) as usize].to_le_bytes(),
    );
    // the text tables: records for IDs 0-0x2ff whose name, help and scan
    // offsets point into a string area of short and long runs
    for (k, &g) in rd::MONSTER_TEXTS.iter().enumerate() {
        let t = tables + 0x4000 * u32::try_from(k).unwrap();
        let mut bytes = excel_table(rng, 0x300, 0xe);
        bytes.resize(0x4000, 0);
        for (i, b) in bytes.iter_mut().enumerate().skip(0x2a20) {
            // every other 0x300-byte stretch holds short strings, the
            // others one long run (past all three limits)
            let short = (i / 0x300) % 2 == 0 && rng.below(40) == 0;
            *b = if i % 0x300 == 0 || short {
                0
            } else {
                0x41 + rng.byte() % 26
            };
        }
        for r in 0..0x300usize {
            for f in [0usize, 2, 6] {
                let at = 0x20 + 0xe * r + 2 * f;
                let off = 0x2a00 + rng.below(0x1500) as u16;
                bytes[at..at + 2].copy_from_slice(&off.to_le_bytes());
            }
        }
        poke(a, b, t, &bytes);
        let p = if rng.below(5) == 0 { 0 } else { t };
        set(a, b, ffx_bridge::Global::<u32>::new(g), p);
    }
    // the slots: IDs (some the same), shared data or not
    for i in 0..8 {
        let m = mon0 + ACTOR * i;
        let id = [0xffff, 7, 7, 9, rng.below(0x100)][rng.below(5) as usize] as u16;
        poke(a, b, m + 0xe, &id.to_le_bytes());
        poke(
            a,
            b,
            m + 0x48,
            &[0, 0, rng.next() | 1][rng.below(3) as usize].to_le_bytes(),
        );
    }
    set(
        a,
        b,
        rd::NEXT_MONSTER,
        [0, 0, 0, 3, 7, 8][rng.below(6) as usize],
    );
    set(a, b, rd::FILE_KIND, [0, 0, 1, 2, 8][rng.below(5) as usize]);
    for _ in 0..3 {
        twin("MsBtlReadMonster", a, b, 0x0078_3680, &[], ALL, |m| {
            rd::ms_btl_read_monster(m, &mut call, data + 0x40)
        });
        let r = b.read(data + 0x40, 0x10).unwrap();
        a.write(data + 0x40, &r).unwrap();
        same("MsBtlReadMonster", a, b);
    }
    for (o, mine) in [&mut *a, &mut *b].into_iter().zip(saved) {
        for (f, v) in mine {
            let at = o.rebase(f);
            o.write(at, &v).unwrap();
        }
    }
    for ((at, _), v) in regions.into_iter().zip(kept) {
        poke(a, b, at, &v);
    }
}

/// `MsCamSetBtl` (round 26) on made-up camera sides and channel records
/// over the dressed actors: battle actors' sets (id bits 16-19 five), and
/// on a snapshot the script actors' too. The caller's registers are the
/// harness's ([`HARNESS`]); the set's local in `VF_B.z` is copied.
fn cam_set_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, script: bool) {
    let h = host();
    let (side, other, ch, other_ch) = (buf, buf + 0x100, buf + 0x200, buf + 0x280);
    let vector = buf + 0x300;
    for s in [side, other] {
        poke(a, b, s, &floats(rng, 0x40));
        poke(a, b, s, &[u8::from(rng.below(2) == 0)]);
        let base = if rng.below(3) == 0 { 0 } else { vector };
        poke(a, b, s + 0x8c, &base.to_le_bytes());
    }
    poke(a, b, vector, &floats(rng, 4));
    let bits_of = |rng: &mut Xorshift| match rng.below(4) {
        0 => 1 << rng.below(ACTORS),
        1 => rng.next() & 0x7fff_ffff,
        2 => 0,
        _ => 3,
    };
    let selector = |rng: &mut Xorshift| -> u16 {
        if rng.below(4) == 0 {
            0xffff
        } else if script {
            [0xffeb, 0xfffb, 0xfffe, 0, 1, 2][rng.below(6) as usize]
        } else {
            u16::try_from(rng.below(4)).unwrap()
        }
    };
    let b0 = bits_of(rng);
    for r in [ch, other_ch] {
        poke(a, b, r, &floats(rng, 0x20));
        poke(
            a,
            b,
            r,
            &[
                [0u8, 1, 2][rng.below(3) as usize],
                0,
                [0xbu8, 0xc, 0xd][rng.below(3) as usize],
                0,
                0,
                0,
                u8::from(rng.below(2) == 0),
            ],
        );
        for k in 0..2 {
            poke(a, b, r + 0x10 + 2 * k, &selector(rng).to_le_bytes());
            let bits = if rng.below(3) == 0 { b0 } else { bits_of(rng) };
            poke(a, b, r + 0x14 + 4 * k, &bits.to_le_bytes());
        }
    }
    // the other channel sometimes naming the same actors
    if rng.below(3) == 0 {
        let w = a.read(ch + 0x14, 8).unwrap();
        poke(a, b, other_ch + 0x14, &w);
    }
    set(a, b, ffx_battle::camera::bridge::EE_R0, [0; 4]);
    let actors = if !script || rng.below(2) == 0 {
        5
    } else {
        rng.below(16)
    };
    let id = actors << 16 | rng.below(0x1000);
    let call = |a: &mut Oracle, b: &mut Oracle, mode: u32| {
        let args = [id, mode, side, ch, other, other_ch];
        zero_stack(a);
        twin("MsCamSetBtl", a, b, 0x007c_58e0, &args, ALL, |m| {
            dm(m, |m| {
                ffx_battle::camera::bridge::ms_cam_set_btl(
                    m,
                    &h,
                    id,
                    mode,
                    side,
                    ch,
                    other,
                    other_ch,
                    Some(HARNESS),
                )
            })
        });
        // MsGetChrInfo's caller ebx: the set's local
        let at = ffx_battle::vector::bridge::VF_B.ptr(a).addr + 8;
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
        same("MsCamSetBtl", a, b);
    };
    // sides at the same place: a first call without a mode places this
    // side, the other side (not polar) takes its position
    if rng.below(4) == 0 {
        poke(a, b, other_ch, &[0]);
        call(a, b, 0);
        let w = a.read(side + 0x50, 12).unwrap();
        poke(a, b, other + 0x50, &w);
        // or only in x
        if rng.below(3) == 0 {
            poke(a, b, other + 0x58, &floats(rng, 1));
        }
    }
    call(a, b, rng.below(6));
}

/// Round 26's small checks: the dressed actors (as [`check`] dresses
/// them) for `rounds` rounds, `body` called `per` times on each with a
/// scratch block of 0x400 bytes.
fn dressed_check(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    synthetic: bool,
    (rounds, per): (u32, u32),
    mut body: impl FnMut(&mut Oracle, &mut Oracle, &mut Xorshift, &World, u32),
) {
    patch(a);
    patch(b);
    let w = world(a, b, rng, synthetic);
    let f = fakes(a, b, rng);
    let s = scratch(a, b, 0x10);
    SCRATCH.with(|c| c.set(s));
    let rec = scratch(a, b, 0x80);
    // SndSepPlay (sound) a logging stub, as in [`process_stubs`]
    let srec = scratch(a, b, EXCHANGE_REC as usize);
    for o in [&mut *a, &mut *b] {
        script_stubs(o, rec);
        let play = ffx_battle::after_damage::bridge::SND_SEP_PLAY;
        logging_stub(o, play, PROCESS_STUBS + 0x40 * 5, srec, 3, 0);
    }
    let buf = scratch(a, b, 0x400);
    for _ in 0..rounds {
        file(a, b, rng, &f);
        dress(a, b, rng, &w, &f);
        for _ in 0..per {
            body(a, b, rng, &w, buf);
        }
    }
}

#[test]
fn cam_set_btl_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0001),
        true,
        (150, 8),
        |a, b, rng, _, buf| {
            cam_set_btl(a, b, rng, buf, false);
        },
    );
}

#[test]
fn cam_set_btl_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0002),
        false,
        (150, 8),
        |a, b, rng, _, buf| {
            cam_set_btl(a, b, rng, buf, true);
        },
    );
}

/// `MsNumberDrawProcess` (round 26) on the dressed actors with screen
/// points near the screen and a made-up list of number characters at
/// `buf`; `TOBtlSetDamageValue` runs as the original in both.
fn number_draw(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, buf: u32) {
    use ffx_battle::input::bridge as ip;
    let status = [0x1fi8, 0x20, 0x21, 0, 5, 0x22][rng.below(6) as usize];
    set(a, b, ffx_battle::state::bridge::STATUS, status);
    // near the screen, at times on its edges
    let near = |rng: &mut Xorshift| {
        if rng.below(6) == 0 {
            [0x200, 0x201, 0x1a0, 0x1a1, 0, -1][rng.below(6) as usize]
        } else {
            rng.range(-0x30, 0x240)
        }
    };
    for i in 0..ACTORS {
        let p = w.actor(i);
        let on = |rng: &mut Xorshift| u8::from(rng.below(6) != 0);
        poke(a, b, p + 0x10, &[on(rng)]);
        poke(a, b, p + 0xdc8, &[on(rng)]);
        poke(a, b, p + 0x15, &[on(rng)]);
        let bits = if rng.below(6) == 0 { 0 } else { 0x4000 } | rng.below(0x4000);
        poke(a, b, p + 0x616, &u16::try_from(bits).unwrap().to_le_bytes());
        let hp = [0, 0x8000_0000, 0x7fc0_0000, float(rng), float(rng)][rng.below(5) as usize];
        poke(a, b, p + 0x40, &hp.to_le_bytes());
        let count = [rng.byte(), rng.byte() % 10, 0][rng.below(3) as usize];
        poke(a, b, p + 0x5c8, &[count, rng.byte()]);
        poke(a, b, p + 0xf6c, &float(rng).to_le_bytes());
        for o in [0xf3c, 0xf40, 0xf44, 0xf48, 0xf4c, 0xf50] {
            poke(a, b, p + o, &near(rng).to_le_bytes());
        }
    }
    // up to 12 characters, linked in order
    let n = rng.below(13);
    let mut list = vec![0u8; 0x14 * 12];
    for k in 0..n as usize {
        let e = &mut list[0x14 * k..0x14 * (k + 1)];
        let actor = [rng.below(ACTORS), rng.next() & 0xff][rng.below(4).min(1) as usize];
        e[0] = rng.byte();
        e[1] = [0u8, 1, 2, 3, rng.byte()][rng.below(5) as usize];
        e[2] = u8::try_from(actor).unwrap();
        // the frame rows mostly in the table, at times anywhere near it
        let wide = rng.below(4) == 0;
        e[3] = if wide {
            rng.byte()
        } else {
            u8::try_from(rng.below(8)).unwrap()
        };
        e[4] = rng.byte();
        e[5] = u8::from(rng.below(4) == 0);
        e[6] = if wide {
            rng.byte()
        } else {
            u8::try_from(rng.below(20)).unwrap()
        };
        e[7] = [u8::try_from(rng.below(20)).unwrap(), rng.byte()][rng.below(4).min(1) as usize];
        e[8..10].copy_from_slice(&i16::try_from(rng.range(-40, 40)).unwrap().to_le_bytes());
        e[10..12].copy_from_slice(&i16::try_from(rng.range(-40, 40)).unwrap().to_le_bytes());
        e[12] = rng.byte();
        e[13] = rng.byte();
        let next = if k + 1 < n as usize {
            buf + 0x14 * (u32::try_from(k).unwrap() + 1)
        } else {
            0
        };
        e[0x10..0x14].copy_from_slice(&next.to_le_bytes());
    }
    poke(a, b, buf, &list);
    let head = if n == 0 { 0 } else { buf };
    set(a, b, ffx_battle::minor::bridge::NUMBER_LIST, head);
    // the damage values' count: room, nearly full, full
    let count = [0i16, 0x3c, 0x3f][rng.below(3) as usize];
    set(a, b, ffx_bridge::Global::<i16>::new(0x0133_f108), count);
    twin("MsNumberDrawProcess", a, b, 0x0079_f660, &[], 0, |m| {
        let mut c = game_call_o;
        ip::ms_number_draw_process(m, &stand_in, &mut c);
        0
    });
    same("MsNumberDrawProcess", a, b);
}

#[test]
fn number_draw_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0101),
        true,
        (300, 2),
        number_draw,
    );
}

#[test]
fn number_draw_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0102),
        false,
        (300, 2),
        number_draw,
    );
}

/// The actors' sounds by kind (round 26): `MsChrSeGet_0079fe80` and the
/// players over it on the dressed actors, their sound fields, a table
/// entry and the scene's transferred sounds made up; the players (through
/// the ported `MsSEP3Dregist`) only for actors with a position.
fn chr_se(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, buf: u32) {
    use ffx_battle::after_damage::bridge as ad;
    // an actor, at times in a word with high bits (the players mask them)
    let i = rng.below(ACTORS)
        | if rng.below(3) == 0 {
            rng.next() & !0xff
        } else {
            0
        };
    let p = w.actor(i & 0xff);
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    poke(a, b, p + 0x40f, &[flag(rng, 4) ^ 1]);
    poke(a, b, p + 0xdcc, &[flag(rng, 3)]);
    poke(a, b, p + 0x410, &[rng.byte()]);
    poke(a, b, p + 0x613, &[flag(rng, 3)]);
    poke(a, b, p + 0x15, &[flag(rng, 5) ^ 1]);
    let hp = [0, 0x8000_0000, 0x7fc0_0000, float(rng), float(rng)][rng.below(5) as usize];
    poke(a, b, p + 0x40, &hp.to_le_bytes());
    let owner = u8::try_from(rng.below(ACTORS)).unwrap();
    poke(a, b, p + 0x14, &[owner]);
    let o = w.actor(u32::from(owner));
    poke(a, b, o + 0xf2c, &[flag(rng, 4) ^ 1]);
    let sounds: Vec<u8> = (0..0x20)
        .flat_map(|_| {
            [0, 0x8000_0001 | rng.below(4), rng.next()][rng.below(3) as usize].to_le_bytes()
        })
        .collect();
    poke(a, b, o + 0xeac, &sounds);
    // a table entry: variant bits and sources at random
    let kind = [
        rng.below(0x21),
        6,
        rng.range(-2, 0x23).cast_unsigned(),
        rng.next(),
    ][rng.below(4) as usize];
    if rng.below(2) == 0 && kind < 0x21 {
        let class = rng.below(3);
        let table = ad::SE_TABLES[class as usize].ptr(b).addr;
        let entry = (rng.below(16) << 8) | (rng.below(4) << 12) | rng.below(0x24);
        poke(
            a,
            b,
            table + 2 * kind,
            &u16::try_from(entry).unwrap().to_le_bytes(),
        );
    }
    set(
        a,
        b,
        ffx_battle::state::bridge::UNDERWATER,
        [0i8, 1, 2, -1][rng.below(4) as usize],
    );
    let trans = ffx_battle::minor::bridge::SEP_TRANS_FLAG.ptr(b).addr;
    poke(a, b, trans, &[flag(rng, 2)]);
    let table: Vec<u8> = (0..0x80)
        .flat_map(|_| [0, 0x4000_0000 | rng.below(8)][rng.below(2) as usize].to_le_bytes())
        .collect();
    poke(
        a,
        b,
        ffx_battle::minor::bridge::SEP_TRANS_TABLE.ptr(b).addr,
        &table,
    );
    set(
        a,
        b,
        ad::LAST_CHR_SE,
        [0, 0x8000_0001, 0x8000_0002][rng.below(3) as usize],
    );
    sep_3d_list(a, b, rng);
    // no actors at times (the players' first check)
    let top = ffx_battle::actor::bridge::CHR_TOP;
    let was = top.get(b);
    let op = rng.below(6);
    // the fade at times for a spare actor (low byte 0x80 on, which the
    // players' & 0xff keeps)
    let i = if op == 1 && rng.below(4) == 0 {
        (i & !0xff) | 0x80 | rng.below(0x80)
    } else {
        i
    };
    // (only the players that check it first)
    let none = matches!(op, 2 | 3 | 5) && rng.below(8) == 0;
    let probe = buf + 0x80;
    let placed = op < 2
        || none
        || i & 0xff == 0xff
        || writes(a, b, 0x0079_56e0, i & 0xff, probe) && writes(a, b, 0x0079_48e0, i & 0xff, probe);
    let op = if placed { op } else { op % 2 };
    if none {
        set(a, b, top, 0);
    }
    let frames = rng.next();
    let what = [
        "MsChrSeGet_0079fe80",
        "MsChrSeFade_0079fcf0",
        "MsChrSePlay_0079fd40",
        "MsChrSePlay2_0079fda0",
        "MsChrSePlayUnhasted_0079fdf0",
        "MsChrSePlayAlive_0079fe30",
    ][op as usize];
    let sym = [
        0x0079_fe80,
        0x0079_fcf0,
        0x0079_fd40,
        0x0079_fda0,
        0x0079_fdf0,
        0x0079_fe30,
    ][op as usize];
    let args: Vec<u32> = if op == 0 {
        vec![i & 0xff, kind]
    } else if op == 1 {
        vec![i, kind, frames]
    } else {
        vec![i, kind]
    };
    twin(what, a, b, sym, &args, ALL, |m| {
        let mut c = game_call_o;
        match op {
            0 => ad::ms_chr_se_get(m, i & 0xff, kind),
            1 => ad::ms_chr_se_fade(m, i, kind, frames).cast_unsigned(),
            2 => ad::ms_chr_se_play(m, &stand_in, &mut c, i, kind),
            3 => ad::ms_chr_se_play2(m, &stand_in, &mut c, i, kind),
            4 => ad::ms_chr_se_play_unhasted(m, &stand_in, &mut c, i, kind),
            _ => ad::ms_chr_se_play_alive(m, &stand_in, &mut c, i, kind),
        }
    });
    if none {
        set(a, b, top, was);
    }
    same(what, a, b);
}

#[test]
fn chr_se_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0201),
        true,
        (150, 8),
        chr_se,
    );
}

#[test]
fn chr_se_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    dressed_check(
        &mut a,
        &mut b,
        &mut Xorshift(0x26ca_0202),
        false,
        (150, 8),
        chr_se,
    );
}

/// `MsChrDelExtraCommand_007acb40` (round 26) on the dressed actors: their
/// extra commands and second menu lists (`g_BtlItemMem2`, here a block of
/// 31 lists, `items`) drawn from a few commands, one of them removed.
fn del_extra_command(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, items: u32) {
    set(a, b, ffx_battle::misc::bridge::ITEM_MEM2, items);
    let pool = [0x3001u16, 0x3002, 0x3003, 0xff, 0x2005];
    let pick = |rng: &mut Xorshift| pool[rng.below(5) as usize];
    for i in 0..ACTORS {
        let p = w.actor(i);
        let extra: Vec<u8> = (0..4).flat_map(|_| pick(rng).to_le_bytes()).collect();
        poke(a, b, p + 0x71a, &extra);
        poke(a, b, p + 0x6cb, &[u8::try_from(rng.below(5)).unwrap()]);
        let list: Vec<u8> = (0..8).flat_map(|_| pick(rng).to_le_bytes()).collect();
        poke(a, b, items + 0x478 * i + 0x28, &list);
    }
    // the actor set selector -4 reads (absent ones count with `any`)
    let picked = rng.next() & 0x7fff_ffff;
    set(a, b, ffx_battle::actor::bridge::CHR_SET, picked);
    let chr = rng.below(ACTORS);
    let sel = match rng.below(4) {
        0 => 0x1000 + rng.below(0x40),
        1 => rng.below(0x40),
        2 => rng.next() & 0x7fff_ffff,
        _ => rng.range(-27, 0).cast_unsigned(),
    };
    let command =
        [u32::from(pick(rng)), 0x1_3001, rng.below(0x10000)][rng.below(4).min(2) as usize];
    twin(
        "MsChrDelExtraCommand_007acb40",
        a,
        b,
        0x007a_cb40,
        &[chr, command, sel],
        ALL,
        |m| ffx_battle::ability::bridge::ms_chr_del_extra_command(m, chr, command, sel),
    );
    same("MsChrDelExtraCommand_007acb40", a, b);
    // and added (round 27): slots of both signs and past 7
    let slot = [rng.range(-2, 10), 0, 7][rng.below(3) as usize];
    twin(
        "MsChrAddExtraCommand_007ac840",
        a,
        b,
        0x007a_c840,
        &[chr, command, sel, slot.cast_unsigned()],
        ALL,
        |m| ffx_battle::ability::bridge::ms_chr_add_extra_command(m, chr, command, sel, slot),
    );
    same("MsChrAddExtraCommand_007ac840", a, b);
}

/// `MsMonShuffleVariants_007842f0` (round 26) on the dressed actors: the
/// monsters shown at random, their model owners mostly among themselves
/// (an owner's own `+0x14` itself or another).
fn mon_shuffle(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let owners = [0x14u8, 0x15, 0x14, 0x16, 0x14, 0x15, 0x1a, 0x1b];
    for k in 0x14..0x1c {
        let p = w.actor(k);
        poke(a, b, p + 0x10, &[u8::from(rng.below(6) != 0)]);
        poke(a, b, p + 0x11, &[u8::from(rng.below(6) != 0)]);
        let owner = if rng.below(4) == 0 {
            u8::try_from(rng.below(ACTORS)).unwrap()
        } else {
            owners[rng.below(8) as usize]
        };
        poke(a, b, p + 0x14, &[owner]);
    }
    twin(
        "MsMonShuffleVariants_007842f0",
        a,
        b,
        0x0078_42f0,
        &[],
        0,
        |m| {
            ffx_battle::read::bridge::ms_mon_shuffle_variants(m);
            0
        },
    );
    same("MsMonShuffleVariants_007842f0", a, b);
}

fn command_and_variants(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let items = scratch(a, b, 0x8a90);
    dressed_check(a, b, rng, synthetic, (150, 4), |a, b, rng, w, _| {
        if rng.below(2) == 0 {
            del_extra_command(a, b, rng, w, items);
        } else {
            mon_shuffle(a, b, rng, w);
        }
    });
}

#[test]
fn commands_and_variants_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    command_and_variants(&mut a, &mut b, &mut Xorshift(0x26ca_0301), true);
}

#[test]
fn commands_and_variants_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    command_and_variants(&mut a, &mut b, &mut Xorshift(0x26ca_0302), false);
}

/// `MsCameraSetBtl` (round 26) on three cameras of random floats (as
/// `messages_and_cameras` fills them) over the dressed actors: requests
/// of the cameras' slots or made up, battle actors' sets (id bits 16-19
/// five) and on a snapshot the script actors' too.
/// Three cameras of random floats at `CAMERAS` with four requests each
/// (battle-actor levels, script ones only on a snapshot; channels 0-5,
/// at times 8-15), moving flags and base vectors (`+0x8c`: none or the
/// other side's position); one of the requests (at times a made-up id).
fn camera_requests(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, script: bool) -> u32 {
    use ffx_battle::camera::bridge as cam;
    let cams = cam::CAMERAS.ptr(b).addr;
    let words: Vec<u8> = (0..3 * cam::CAMERA_SIZE / 4)
        .flat_map(|_| float(rng).to_le_bytes())
        .collect();
    poke(a, b, cams, &words);
    let mut ids = Vec::new();
    for c in 0..3 {
        let base = cams + c * cam::CAMERA_SIZE;
        for o in [0x31, 0x33, 0x31 + 0x61c, 0x33 + 0x61c] {
            poke(a, b, base + o, &[u8::try_from(rng.below(3)).unwrap()]);
        }
        // each side's base vector (+0x8c): none, or the other side's position
        for side in 0..2 {
            let s = base + 0x30 + side * 0x61c;
            let v = [0, base + 0x30 + (1 - side) * 0x61c + 0x50][rng.below(2) as usize];
            poke(a, b, s + 0x8c, &v.to_le_bytes());
        }
        for o in [
            0x5bc,
            0x5cc,
            0x5dc,
            0x5bc + 0x61c,
            0x5cc + 0x61c,
            0x5dc + 0x61c,
        ] {
            let v = u16::from(rng.below(3) == 0);
            poke(a, b, base + o, &v.to_le_bytes());
        }
        for n in 0..4 {
            // the level's low nibble 5: battle actors (script ones only
            // on a snapshot)
            let level = if !script || rng.below(2) == 0 {
                5 | rng.below(2) << 4
            } else {
                rng.below(5)
            };
            // channels 0-5, at times beyond (8-15)
            let sub = if rng.below(8) == 0 {
                8 + rng.below(8)
            } else {
                rng.below(6)
            };
            let id = c | sub << 8 | level << 16;
            ids.push(id);
            let mut s = id.to_le_bytes().to_vec();
            s.extend([
                u8::from(rng.below(5) == 0),
                u8::from(rng.below(5) == 0),
                rng.byte() & 3,
            ]);
            poke(
                a,
                b,
                cam::SLOTS_AT.ptr(b).addr + c * cam::CAMERA_SIZE + 8 * n,
                &s,
            );
        }
    }
    if rng.below(5) == 0 {
        rng.next() & 0x05_0f02 | if script { 0 } else { 0x5_0000 }
    } else {
        ids[rng.below(u32::try_from(ids.len()).unwrap()) as usize]
    }
}

/// A made-up channel's actor selector for request `id`, as
/// `MsCamSetBtl` draws them: none, script ones (only on a snapshot, for a
/// request that is no battle actor's), or battle ones.
fn cam_selector(rng: &mut Xorshift, script: bool, id: u32) -> u32 {
    if rng.below(4) == 0 {
        0xffff
    } else if script && id & 0xf_0000 != 0x5_0000 {
        [0xffff_ffeb, 0xffff_fffb, 0xffff_fffe, 0, 1, 2][rng.below(6) as usize]
    } else {
        // (the special sets only on a snapshot: some read state the
        // image alone lacks)
        match rng.below(3) {
            0 => 0x1000 + rng.below(0x40),
            2 if script => rng.range(-27, 0).cast_unsigned(),
            _ => rng.below(0x40),
        }
    }
}

fn camera_set_btl(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, script: bool) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    let id = camera_requests(a, b, rng, script);
    let selector = |rng: &mut Xorshift| cam_selector(rng, script, id);
    set(a, b, cam::EE_R0, [0; 4]);
    let side = rng.below(4);
    let chr = rng.below(ACTORS);
    let mode = rng.below(6);
    let sets = [selector(rng), selector(rng)];
    let lift = float(rng);
    let polar = [float(rng), float(rng), float(rng)];
    let args = [
        id, side, chr, mode, sets[0], sets[1], lift, polar[0], polar[1], polar[2],
    ];
    zero_stack(a);
    twin("MsCameraSetBtl", a, b, 0x007c_5c60, &args, ALL, |m| {
        dm(m, |m| {
            let mut c = game_call;
            cam::ms_camera_set_btl(m, &h, &mut c, id, side, chr, mode, sets, lift, polar)
        })
    });
    // MsGetChrInfo's caller ebx: the set's local
    let at = ffx_battle::vector::bridge::VF_B.ptr(a).addr + 8;
    let w = a.read(at, 4).unwrap();
    b.write(at, &w).unwrap();
    same("MsCameraSetBtl", a, b);
}

/// [`camera_set_btl`] on the dressed actors, `MsSetCheckChange`'s move
/// step (`CAMERA_MOVE_STEP`, its splines and filters) a logging stub as in
/// [`messages_and_cameras`].
fn camera_set_btl_check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let rec = scratch(a, b, EXCHANGE_REC as usize);
    for o in [&mut *a, &mut *b] {
        let f = ffx_battle::camera::bridge::CAMERA_MOVE_STEP;
        logging_stub(o, f, START_STUBS2 + 0xc0 + 0x40 * 9, rec, 4, 0);
    }
    dressed_check(a, b, rng, synthetic, (200, 4), |a, b, rng, _, _| {
        camera_set_btl(a, b, rng, !synthetic);
    });
}

#[test]
fn camera_set_btl_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    camera_set_btl_check(&mut a, &mut b, &mut Xorshift(0x26ca_0401), true);
}

#[test]
fn camera_set_btl_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    camera_set_btl_check(&mut a, &mut b, &mut Xorshift(0x26ca_0402), false);
}

/// `MsCameraAtelFocus_007c0560` (round 26) on a made-up side and move at
/// `buf`: script actors only on a snapshot (the image runs no script).
fn atel_focus(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, script: bool) {
    let h = host();
    let (side, mv) = (buf, buf + 0x100);
    poke(a, b, side, &floats(rng, 0x20));
    poke(a, b, mv, &floats(rng, 0x18));
    for k in 0..2 {
        let actor: u16 = if script && rng.below(4) != 0 {
            u16::try_from(rng.below(4)).unwrap()
        } else {
            0xffff
        };
        poke(a, b, mv + 0xc + 2 * k, &actor.to_le_bytes());
    }
    zero_stack(a);
    twin(
        "MsCameraAtelFocus_007c0560",
        a,
        b,
        0x007c_0560,
        &[side, mv],
        0,
        |m| {
            dm(m, |m| {
                ffx_battle::camera::bridge::ms_camera_atel_focus(m, &h, side, mv);
            });
            0
        },
    );
    same("MsCameraAtelFocus_007c0560", a, b);
}

/// The special stage's camera (round 26): `MsBattleSpecialCameraKind_0079dd70`,
/// `MsTransSpPos` and `MsSpCameraProcess` on a made-up stage work
/// (`work`, 0x870 bytes of floats) and three vectors at `buf`; the port's
/// matrix (the original's local) at `buf + 0x80`, cleared after.
fn sp_camera(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, work: u32) {
    use ffx_battle::special::bridge as sp;
    poke(a, b, work, &floats(rng, 0x870 / 4));
    set(a, b, ffx_battle::free::bridge::SPECIAL_MEM, work);
    let kind = [0i8, 1, 1, 2, 3, 4, -1][rng.below(7) as usize];
    set(a, b, ffx_battle::free::bridge::SPECIAL_KIND, kind);
    set(
        a,
        b,
        ffx_battle::process::bridge::LIMIT_TYPE_CUE,
        [0u8, 1, 1, 2][rng.below(4) as usize],
    );
    set(
        a,
        b,
        ffx_battle::misc::bridge::SPECIAL_CAMERA,
        u8::from(rng.below(3) == 0),
    );
    set(a, b, sp::SP_CAMERA_ON, u8::from(rng.below(4) != 0));
    poke(a, b, buf, &floats(rng, 12));
    let matrix = buf + 0x80;
    zero_stack(a);
    // the last rotation's (MsRotMatrixX/Z) uninitialized row lanes in VU
    // register 4 and 5 (as battle_vector.rs): x last unless inverse
    let mut last_x = true;
    match rng.below(3) {
        0 => twin(
            "MsBattleSpecialCameraKind_0079dd70",
            a,
            b,
            0x0079_dd70,
            &[],
            ALL,
            |m| sp::ms_battle_special_camera_kind(m).cast_unsigned(),
        ),
        1 => {
            let inverse = [0, 1, rng.next()][rng.below(3) as usize];
            let v = buf + 0x10 * rng.below(3);
            last_x = inverse == 0;
            twin("MsTransSpPos", a, b, 0x0079_e1b0, &[inverse, v], 0, |m| {
                sp::ms_trans_sp_pos(m, &stand_in, inverse, v, matrix);
                0
            });
        }
        _ => {
            let v = [buf, buf + 0x10, buf + 0x20];
            twin("MsSpCameraProcess", a, b, 0x0079_dfe0, &v, 0, |m| {
                sp::ms_sp_camera_process(m, &stand_in, v, matrix);
                0
            });
        }
    }
    poke(a, b, matrix, &[0; 0x40]);
    if kind == 1 {
        let free = if last_x {
            [0x810u32, 0x81c, 0x820, 0x82c]
        } else {
            [0x818, 0x81c, 0x828, 0x82c]
        };
        for o in free {
            let at = a.rebase(0x00c8_f000 + o);
            let w = a.read(at, 4).unwrap();
            b.write(at, &w).unwrap();
        }
    }
    same("the special camera", a, b);
}

/// `MsCameraAtelYaw_007c0650` and `MsCameraSplinePos_007bcd30` (round 26)
/// on a made-up side, move and other side at `buf`: script actors only on
/// a snapshot; the spline one of the dressed move splines (`MOVE_SPLINES`,
/// at times of two points or fewer, where the original's local keeps its
/// stack: zeroed).
fn camera_tracks(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, script: bool) {
    let h = host();
    let (side, mv, other) = (buf, buf + 0x100, buf + 0x200);
    poke(a, b, side, &floats(rng, 0x20));
    poke(a, b, mv, &floats(rng, 0x18));
    poke(a, b, other, &floats(rng, 0x10));
    zero_stack(a);
    if rng.below(2) == 0 {
        let track = buf + 0x300;
        for k in 0..2 {
            let actor: u16 = if script && rng.below(4) != 0 {
                u16::try_from(rng.below(4)).unwrap()
            } else {
                0xffff
            };
            poke(a, b, track + 0xc + 2 * k, &actor.to_le_bytes());
        }
        twin(
            "MsCameraAtelYaw_007c0650",
            a,
            b,
            0x007c_0650,
            &[side, mv, track],
            0,
            |m| {
                dm(m, |m| {
                    ffx_battle::camera::bridge::ms_camera_atel_yaw(m, &h, side, mv, track);
                });
                0
            },
        );
    } else {
        let sp = t::MOVE_SPLINES.ptr(b).addr + 0x210 * rng.below(4);
        if rng.below(3) == 0 {
            poke(a, b, sp, &rng.below(3).to_le_bytes());
        }
        poke(a, b, mv + 8, &sp.to_le_bytes());
        poke(a, b, mv + 0x1c, &[rng.byte()]);
        twin(
            "MsCameraSplinePos_007bcd30",
            a,
            b,
            0x007b_cd30,
            &[rng.next(), rng.next(), side, mv, other],
            0,
            |m| {
                dm(m, |m| {
                    ffx_battle::camera::bridge::ms_camera_spline_pos(m, &h, side, mv, other);
                });
                0
            },
        );
    }
    // the spline's VU words holding its output's address (a local)
    local_words(a, b);
    same("the camera tracks", a, b);
}

fn camera_leaves(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let work = scratch(a, b, 0x870);
    let buf2 = scratch(a, b, 0x400);
    dressed_check(
        a,
        b,
        rng,
        synthetic,
        (150, 4),
        |a, b, rng, _, buf| match rng.below(3) {
            0 => atel_focus(a, b, rng, buf, !synthetic),
            1 => sp_camera(a, b, rng, buf, work),
            _ => camera_tracks(a, b, rng, buf2, !synthetic),
        },
    );
}

#[test]
fn camera_leaves_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    camera_leaves(&mut a, &mut b, &mut Xorshift(0x26ca_0501), true);
}

#[test]
fn camera_leaves_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    camera_leaves(&mut a, &mut b, &mut Xorshift(0x26ca_0502), false);
}

/// A spline position: in 0..1, beyond, at the ends, or (`nan`) NaN.
fn spline_time(rng: &mut Xorshift, nan: bool) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => 1.0f32.to_bits(),
        2 => (-0.0f32).to_bits(),
        3 if nan => 0x7fc0_0000 | rng.below(0x40_0000),
        4 => (f32::from(i16::try_from(rng.range(-400, 1400)).unwrap()) / 1000.0).to_bits(),
        _ => (f32::from(u16::try_from(rng.below(1000)).unwrap()) / 1000.0).to_bits(),
    }
}

/// `MsCameraSplineMove_007bcb80` (round 27) for a request of the dressed
/// cameras ([`camera_requests`]) on a made-up side and move at `buf`, the
/// spline one of the dressed move splines as in [`camera_tracks`].
fn camera_spline_move(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32, script: bool) {
    let h = host();
    let id = camera_requests(a, b, rng, script);
    let (side, mv) = (buf, buf + 0x100);
    poke(a, b, side, &floats(rng, 0x20));
    poke(a, b, mv, &floats(rng, 0x20));
    let sp = t::MOVE_SPLINES.ptr(b).addr + 0x210 * rng.below(4);
    if rng.below(3) == 0 {
        poke(a, b, sp, &rng.below(3).to_le_bytes());
    }
    poke(a, b, mv + 8, &sp.to_le_bytes());
    poke(a, b, mv + 0x1c, &[rng.byte()]);
    let nan_t = rng.below(2) == 0;
    poke(a, b, mv + 0x68, &spline_time(rng, nan_t).to_le_bytes());
    let speed = match rng.below(6) {
        0 => 0,
        1 => (-0.0f32).to_bits(),
        2 if !nan_t => 0x7fc0_0000,
        _ => (f32::from(i16::try_from(rng.range(-100, 100)).unwrap()) / 1000.0).to_bits(),
    };
    poke(a, b, mv + 0x6c, &speed.to_le_bytes());
    let side_no = [0, 1, rng.next()][rng.below(3) as usize];
    let args = [id, side_no, rng.next(), side, mv];
    zero_stack(a);
    twin(
        "MsCameraSplineMove_007bcb80",
        a,
        b,
        0x007b_cb80,
        &args,
        0,
        |m| {
            dm(m, |m| {
                let mut c = game_call;
                ffx_battle::camera::bridge::ms_camera_spline_move(
                    m, &h, &mut c, id, side_no, side, mv,
                );
            });
            0
        },
    );
    // the splines' and MsAddPoint's VU words holding their locals' addresses
    local_words(a, b);
    same("MsCameraSplineMove_007bcb80", a, b);
}

/// The harness's `EBP` (the oracle's), as a callee reading its caller's
/// saved `ebp` sees it.
const HARNESS_EBP: u32 = 0x4444_4444;

/// A made-up camera side and channel at `buf` and `buf + 0x100` for the
/// near-point steps: up to five filters of kinds 0-6 (at times others),
/// the spline one of the dressed move splines; the side's base vector (`+0x8c`) none or the channel's start.
fn near_channel(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32) -> (u32, u32) {
    let (side, chan) = (buf, buf + 0x100);
    poke(a, b, side, &floats(rng, 0x40));
    poke(a, b, chan, &floats(rng, 0x130 / 4));
    let base = [0, chan + 0x30][rng.below(2) as usize];
    poke(a, b, side + 0x8c, &base.to_le_bytes());
    let n = u8::try_from(rng.below(6)).unwrap();
    poke(
        a,
        b,
        chan,
        &[
            u8::try_from(rng.below(3)).unwrap(),
            0,
            0,
            0,
            n,
            u8::from(rng.below(3) == 0),
        ],
    );
    for k in 0..5 {
        let kind = if rng.below(8) == 0 {
            rng.next()
        } else {
            rng.below(7)
        };
        poke(a, b, chan + 0x90 + 0x20 * k, &kind.to_le_bytes());
    }
    // (no short splines: the near-point search's spline locals would keep
    // the stack, which differs between the oracles there)
    let sp = t::MOVE_SPLINES.ptr(b).addr + 0x210 * rng.below(4);
    poke(a, b, chan + 8, &sp.to_le_bytes());
    poke(
        a,
        b,
        chan + 0x1d,
        &[u8::from(rng.below(2) == 0), u8::from(rng.below(2) == 0)],
    );
    (side, chan)
}

/// `MsCameraMoveStep_007bcab0` and `MsCameraNearPoints_007c0d40` (round
/// 27) on [`near_channel`]: the near-point search and the channel limits
/// (`0x007c0120`) run as the originals in both oracles.
fn camera_near(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, buf: u32) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    let (side, chan) = near_channel(a, b, rng, buf);
    zero_stack(a);
    if rng.below(2) == 0 {
        let other = buf + 0x300;
        poke(a, b, other, &floats(rng, 0x20));
        let speed = match rng.below(4) {
            0 => 0,
            1 => (-0.0f32).to_bits(),
            2 => 0x7fc0_0000,
            _ => float(rng),
        };
        poke(a, b, other + 0x6c, &speed.to_le_bytes());
        let step = [0, 1, 1, 2, rng.next()][rng.below(5) as usize];
        let args = [side, other, chan, step];
        twin(
            "MsCameraMoveStep_007bcab0",
            a,
            b,
            cam::CAMERA_MOVE_STEP,
            &args,
            ALL,
            |m| {
                let mut c = game_call_o;
                let tail = Some([HARNESS.esi, HARNESS.ebx, HARNESS_EBP]);
                cam::ms_camera_move_step(m, &stand_in, &mut c, side, other, chan, step, tail)
            },
        );
        // the channel limits (0x007c0120, the original in both) pass
        // MsMulVector and MsVectorGetSide vectors by value whose w words
        // are their caller's stack: the move step's in `a`, the harness's
        // in `b`
        let lim = a.read(chan + 4, 2).unwrap();
        if lim[0] != 0 && lim[1] == 0 {
            let vb = ffx_battle::vector::bridge::VF_B.ptr(a).addr;
            let vm = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
            for at in [vb + 0xc, vm + 0x1c] {
                let w = a.read(at, 4).unwrap();
                b.write(at, &w).unwrap();
            }
        }
    } else {
        let args = [rng.next(), side, chan];
        twin(
            "MsCameraNearPoints_007c0d40",
            a,
            b,
            0x007c_0d40,
            &args,
            ALL,
            |m| {
                dm(m, |m| {
                    let mut c = game_call;
                    let tail = Some([HARNESS.ebx, HARNESS.edi, HARNESS.esi]);
                    cam::ms_camera_near_points(m, &h, &mut c, side, chan, tail)
                })
            },
        );
    }
    // the near-point search's VU words holding its locals' addresses
    local_words(a, b);
    same("the camera's near points", a, b);
}

/// Round 27's camera steps on the dressed actors, `MsSetCheckChange`'s
/// move step (`CAMERA_MOVE_STEP`) a logging stub as in
/// [`camera_set_btl_check`] for [`camera_spline_move`] (installed after
/// [`camera_near`]'s runs, which call it as the original).
fn camera_steps(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let buf = scratch(a, b, 0x400);
    dressed_check(a, b, rng, synthetic, (60, 4), |a, b, rng, _, _| {
        camera_near(a, b, rng, buf);
    });
    let rec = scratch(a, b, 0x80);
    for o in [&mut *a, &mut *b] {
        let f = ffx_battle::camera::bridge::CAMERA_MOVE_STEP;
        logging_stub(o, f, START_STUBS2 + 0xc0 + 0x40 * 9, rec, 4, 0);
    }
    dressed_check(a, b, rng, synthetic, (150, 4), |a, b, rng, _, _| {
        camera_spline_move(a, b, rng, buf, !synthetic);
    });
    dressed_check(a, b, rng, synthetic, (60, 4), |a, b, rng, _, _| {
        camera_set_splines(a, b, rng, !synthetic);
    });
}

/// `MsCameraSetSpline` and `MsCameraSetSpline2` (round 28) for a request
/// of the dressed cameras ([`camera_requests`]: current or not) or
/// another, both sides and beyond, a dressed move spline or none, the
/// place and speed as [`camera_spline_move`]'s.
fn camera_set_splines(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, script: bool) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    let mut call = game_call;
    let id = camera_requests(a, b, rng, script);
    let id = [id, id, id ^ 0x100, rng.next() & 0x3_0502][rng.below(4) as usize];
    let side = [0, 1, rng.next()][rng.below(3) as usize];
    let sp = if rng.below(6) == 0 {
        0
    } else {
        t::MOVE_SPLINES.ptr(b).addr + 0x210 * rng.below(4)
    };
    let flags = rng.next();
    zero_stack(a);
    if rng.below(2) == 0 {
        let nan_t = rng.below(2) == 0;
        let t = spline_time(rng, nan_t);
        let speed = match rng.below(5) {
            0 => 0,
            1 => (-0.0f32).to_bits(),
            _ => (f32::from(i16::try_from(rng.range(-100, 100)).unwrap()) / 1000.0).to_bits(),
        };
        twin(
            "MsCameraSetSpline",
            a,
            b,
            0x007b_fca0,
            &[id, side, sp, t, speed, flags],
            ALL,
            |m| {
                dm(m, |m| {
                    cam::ms_camera_set_spline(m, &h, &mut call, id, side, sp, t, speed, flags)
                })
            },
        );
    } else {
        twin(
            "MsCameraSetSpline2",
            a,
            b,
            0x007b_fd90,
            &[id, side, sp, flags],
            ALL,
            |m| {
                dm(m, |m| {
                    cam::ms_camera_set_spline2(m, &h, &mut call, id, side, sp, flags)
                })
            },
        );
    }
    local_words(a, b);
    same("MsCameraSetSpline", a, b);
}

#[test]
fn camera_steps_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    camera_steps(&mut a, &mut b, &mut Xorshift(0x27ca_0101), true);
}

#[test]
fn camera_steps_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    camera_steps(&mut a, &mut b, &mut Xorshift(0x27ca_0102), false);
}

/// `_CIcos`/`_CIsin` on a double as the patched C runtime gives them
/// (`battle_common::crt`, as `tests/battle_shake.rs`).
fn trig64(t: ffx_battle::pos::bridge::Trig, x: f64) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    use ffx_bridge::x87::{add, mul, store};
    match t {
        Trig::Cos => store(mul(x, x)),
        Trig::Sin => store(add(x, x)),
        Trig::Atan => store(-x),
    }
}

/// A camera move lane (16 bytes): kind, curve, the accelerating,
/// holding and braking frames, the length and the frame, all short.
fn camera_lane(rng: &mut Xorshift) -> Vec<u8> {
    let small = |rng: &mut Xorshift, n: u32| u16::try_from(rng.below(n)).unwrap();
    // (length 0, an idle lane, often: all three of a side idle at times)
    let total = if rng.below(2) == 0 { 0 } else { small(rng, 24) };
    let w = [
        small(rng, 4),
        small(rng, 4),
        small(rng, 4),
        small(rng, 8),
        small(rng, 8),
        small(rng, 8),
        total,
        small(rng, u32::from(total) + 3),
    ];
    w.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A random shake record (as `tests/battle_shake.rs`): every kind and
/// none, short counts, lengths of both signs.
fn shake_record(rng: &mut Xorshift) -> Vec<u8> {
    let mut r = vec![0u8; 0x24];
    r[0] = if rng.below(3) == 0 {
        [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10][rng.below(10) as usize]
    } else {
        0
    };
    r[1] = if rng.below(3) == 0 { 0 } else { rng.byte() };
    r[2..4].copy_from_slice(&rng.next().to_le_bytes()[..2]);
    let left = i16::try_from(rng.range(-3, 6)).unwrap();
    r[4..6].copy_from_slice(&left.to_le_bytes());
    let length = match rng.below(5) {
        0 => 0i16,
        1 => i16::try_from(rng.range(-40, 0)).unwrap(),
        _ => i16::try_from(rng.range(1, 60)).unwrap(),
    };
    r[6..8].copy_from_slice(&length.to_le_bytes());
    for o in [8usize, 0xc, 0x10, 0x14, 0x18, 0x1c] {
        r[o..o + 4].copy_from_slice(&float(rng).to_le_bytes());
    }
    r[0x20] = u8::try_from(rng.below(6)).unwrap();
    r[0x21] = u8::try_from(rng.below(4)).unwrap();
    r
}

/// One camera channel (`ch`, 0x130 bytes) of request `id` for
/// `MsCameraProcess`: its polar byte, a request of every kind (and
/// none, and beyond), up to five filters, a dressed move spline, script
/// actors only on a snapshot, `MsCamSetBtl`'s selectors and their bits.
fn process_channel(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    ch: u32,
    id: u32,
    script: bool,
) {
    let polar = [0u8, 1, 2, rng.byte()][rng.below(4) as usize];
    let kind = u8::try_from(rng.below(19)).unwrap();
    let filters = u8::try_from(rng.below(6)).unwrap();
    poke(
        a,
        b,
        ch,
        &[
            polar,
            rng.byte() & 3,
            kind,
            0,
            filters,
            u8::from(rng.below(3) == 0),
        ],
    );
    for k in 0..5 {
        let kind = if rng.below(8) == 0 {
            rng.next()
        } else {
            rng.below(7)
        };
        poke(a, b, ch + 0x90 + 0x20 * k, &kind.to_le_bytes());
    }
    let sp = t::MOVE_SPLINES.ptr(b).addr + 0x210 * rng.below(4);
    poke(a, b, ch + 8, &sp.to_le_bytes());
    for k in 0..2 {
        let actor: u16 = if script && rng.below(4) != 0 {
            u16::try_from(rng.below(4)).unwrap()
        } else {
            0xffff
        };
        poke(a, b, ch + 0xc + 2 * k, &actor.to_le_bytes());
    }
    let chr = rng.below(ACTORS);
    for k in 0..2 {
        let sel = cam_selector(rng, script, id);
        poke(a, b, ch + 0x10 + 2 * k, &sel.to_le_bytes()[..2]);
        let bits = if id & 0xf_0000 == 0x5_0000 {
            ffx_battle::actor::bridge::ms_get_chr_bit(b, chr, sel, 1, 1)
        } else {
            sel
        };
        poke(a, b, ch + 0x14 + 4 * k, &bits.to_le_bytes());
    }
    poke(
        a,
        b,
        ch + 0x1d,
        &[u8::from(rng.below(2) == 0), u8::from(rng.below(2) == 0)],
    );
    // the easing frames (`+0x70`..`+0x7c`): none, -0.0, one (the eased
    // value is the target) or any
    for k in 0..4 {
        let frames = [
            0,
            (-0.0f32).to_bits(),
            1.0f32.to_bits(),
            float(rng),
            float(rng),
        ][rng.below(5) as usize];
        poke(a, b, ch + 0x70 + 4 * k, &frames.to_le_bytes());
    }
}

/// `MsCameraProcess` (round 27) on three dressed cameras
/// ([`camera_requests`], [`process_channel`] for every side's four
/// channels, short move lanes, shakes), screens on and off, the camera
/// matrices and the override at times, the special stage's camera at
/// times (its work at `work`). The Rust gets the harness's registers and
/// the original's frame address (`ebp`) for the by-value vectors' words.
fn camera_process(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, script: bool, work: u32) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    // at times one screen alone, its trailing sides' lag at the limit or
    // beyond it, no camera matrices and no special stage: the beyond case's
    // MsMulVector then writes VF_B last
    let lag = (rng.below(4) == 0).then(|| [5.0f32, 1.0][rng.below(2) as usize]);
    camera_requests(a, b, rng, script);
    let cams = cam::CAMERAS.ptr(b).addr;
    for c in 0..3 {
        let base = cams + c * cam::CAMERA_SIZE;
        // the requests' channels among the four set up below
        // (MsSetCheckChange reads the id's channel, its spline too)
        for n in 0..4 {
            let at = cam::SLOTS_AT.ptr(b).addr + c * cam::CAMERA_SIZE + 8 * n + 1;
            poke(a, b, at, &[u8::try_from(rng.below(4)).unwrap()]);
        }
        let last = [0u8, 1, 2, 3, 0xff, rng.byte()][rng.below(6) as usize];
        poke(
            a,
            b,
            base + 0xc8a,
            &[u8::from(rng.below(3) == 0), rng.byte(), last],
        );
        for o in [0xd78, 0xd88] {
            poke(a, b, base + o, &camera_lane(rng));
        }
        // at times the roll targets are the last roll (no turn to ease)
        if rng.below(4) == 0 {
            let last = a.read(base + 0xc90, 4).unwrap();
            for n in 0..4 {
                poke(a, b, base + 0xcb8 + 0x30 * n, &last);
            }
        }
        for side in 0..2 {
            let s = base + 0x30 + side * 0x61c;
            let near = |rng: &mut Xorshift| [0u8, 1, rng.byte()][rng.below(3) as usize];
            poke(
                a,
                b,
                s + 1,
                &[
                    [0u8, 1, 2][rng.below(3) as usize],
                    rng.byte() & 1,
                    u8::from(rng.below(3) == 0),
                    near(rng),
                    near(rng),
                ],
            );
            for o in [0x580, 0x590, 0x5a0] {
                poke(a, b, s + o, &camera_lane(rng));
            }
            for k in 0..3 {
                poke(a, b, s + 0x5b0 + 0x24 * k, &shake_record(rng));
            }
            for n in 0..4 {
                let id: u32 = u32::from_le_bytes(
                    a.read(cam::SLOTS_AT.ptr(b).addr + c * cam::CAMERA_SIZE + 8 * n, 4)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                );
                process_channel(a, b, rng, s + 0xc0 + 0x130 * n, id, script);
            }
            // trailing sides whose lag (no step toward the position: scale
            // 0) is exactly the limit (|(3, 4, 0)| is 5) or beyond it
            if let Some(limit) = lag {
                poke(a, b, s + 3, &[1]);
                poke(a, b, s + 8, &limit.to_bits().to_le_bytes());
                poke(a, b, s + 0xc, &[0; 4]);
                let lag: Vec<u8> = [3.0f32, 4.0, 0.0]
                    .iter()
                    .flat_map(|v| v.to_bits().to_le_bytes())
                    .collect();
                poke(a, b, s + 0x40, &lag);
                // a request on every channel (the side only moves with one)
                for n in 0..4 {
                    let at = s + 0xc0 + 0x130 * n + 2;
                    if a.read(at, 1).unwrap()[0] == 0 {
                        poke(a, b, at, &[1]);
                    }
                }
            }
        }
    }
    let screens = cam::SCREENS.ptr(b).addr;
    let alone = rng.below(3);
    for k in 0..3 {
        let on = if lag.is_some() {
            u8::from(k == alone)
        } else {
            u8::from(rng.below(4) != 0)
        };
        poke(
            a,
            b,
            screens + k * cam::SCREEN_SIZE + 0xc0,
            &[on, u8::try_from(rng.below(3)).unwrap()],
        );
    }
    set(
        a,
        b,
        cam::MATRIX_ON,
        u32::from(rng.below(6) == 0 && lag.is_none()),
    );
    set(a, b, cam::OVERRIDE_ON, u32::from(rng.below(6) == 0));
    poke(a, b, cam::MATRIX_LOOK_AT.ptr(b).addr, &floats(rng, 8));
    set(a, b, cam::OVERRIDE_DEPTH, float(rng));
    set(a, b, cam::MATRIX_ROLL, float(rng));
    set(a, b, cam::EE_R0, [0; 4]);
    // the special stage's camera at times (as [`sp_camera`])
    let special = rng.below(5) == 0 && lag.is_none();
    let kind = [0i8, 1, 1, 2, 3, 4, -1][rng.below(7) as usize];
    if special {
        poke(a, b, work, &floats(rng, 0x870 / 4));
        set(a, b, ffx_battle::free::bridge::SPECIAL_MEM, work);
        set(a, b, ffx_battle::free::bridge::SPECIAL_KIND, kind);
        set(
            a,
            b,
            ffx_battle::special::bridge::SP_CAMERA_ON,
            u8::from(rng.below(4) != 0),
        );
        set(a, b, ffx_battle::misc::bridge::SPECIAL_CAMERA, 0u8);
    }
    set(
        a,
        b,
        ffx_battle::process::bridge::LIMIT_TYPE_CUE,
        u8::from(special),
    );
    zero_stack(a);
    let frame = a.stack_range().end - 0x108;
    twin("MsCameraProcess", a, b, 0x007b_e110, &[], 0, |m| {
        dm(m, |m| {
            let mut c = game_call;
            let mut env = cam::ProcessEnv {
                trig64: &trig64,
                call: &mut c,
                regs: Some(HARNESS),
                frame: Some(frame),
            };
            cam::ms_camera_process(m, &h, &mut env);
        });
        0
    });
    // the channel limits (0x007c0120) and MsSetCheckChange's move step run
    // as the originals from the harness in `b`: their by-value vectors' w
    // words are their callers' stack (as [`camera_near`])
    let vb = ffx_battle::vector::bridge::VF_B.ptr(a).addr;
    let vm = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    for at in [vb + 0xc, vm + 0x1c] {
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
    if special && kind == 1 {
        // MsRotMatrixX/Z's uninitialized row lanes (as [`sp_camera`])
        for o in [0x810u32, 0x818, 0x81c, 0x820, 0x828, 0x82c] {
            let at = a.rebase(0x00c8_f000 + o);
            let w = a.read(at, 4).unwrap();
            b.write(at, &w).unwrap();
        }
    }
    // the VU words holding a local's address ([`local_words`]) but VF2.z
    // and VF4.z: MsCameraProcess's own vectors' words are known registers.
    // VF_B.y and .z are compared when a trailing side's limit MsMulVector
    // wrote them last (the caller's edi, esi): the lone camera's lag was
    // cut to the limit (it is no longer (3, 4, 0) and finite; a NaN step
    // toward the position skips the cut); else the move step, an original
    // run from the harness in `b`, or MsGetChrInfo's local left them
    let vb = ffx_battle::vector::bridge::VF_B.ptr(a).addr;
    let cut = lag.is_some_and(|l| l < 5.0) && {
        let c = a
            .read(screens + alone * cam::SCREEN_SIZE + 0xc1, 1)
            .unwrap()[0];
        let base = cams + u32::from(c) * cam::CAMERA_SIZE;
        (0..2).any(|side| {
            let v: Vec<u32> = a
                .read(base + 0x30 + side * 0x61c + 0x40, 12)
                .unwrap()
                .chunks(4)
                .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                .collect();
            v.iter().all(|&x| f32::from_bits(x).is_finite())
                && v[..2] != [3.0f32.to_bits(), 4.0f32.to_bits()]
        })
    };
    for at in [vb + 4, vb + 8, vb - 4, vm + 0xc, vm + 0x2c] {
        if (at == vb + 4 || at == vb + 8) && cut {
            continue;
        }
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
    same("MsCameraProcess", a, b);
}

fn camera_process_check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let work = scratch(a, b, 0x870);
    dressed_check(a, b, rng, synthetic, (100, 3), |a, b, rng, _, _| {
        camera_process(a, b, rng, !synthetic, work);
    });
}

#[test]
fn camera_process_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    camera_process_check(&mut a, &mut b, &mut Xorshift(0x27ca_0201), true);
}

#[test]
fn camera_process_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    camera_process_check(&mut a, &mut b, &mut Xorshift(0x27ca_0202), false);
}

/// `MsScreenProcess`'s callees in the renderer and other modules as
/// logging stubs (arguments, return value), in `Ms_Gui_BattleConfigProc`
/// (`0x007c6e20`, never called); `MagicFile_GetMagicId`'s and
/// `graphicLevelGetNameHash`'s results are the test's.
const SCREEN_STUBS: u32 = 0x007c_6e20;
const SCREEN_CALLEES: [(u32, u8); 11] = {
    use ffx_battle::screen::bridge as s;
    [
        (s::LEVEL_NAME_HASH, 0),
        (s::MAGIC_ID, 0),
        (s::SET_WORLD_MATRIX_SUB, 2),
        (s::SET_NEAR_PLANE, 1),
        (s::SET_SCREEN_OFFSET, 2),
        (s::SET_DTT_OFFSET, 2),
        (s::NEED_RESTORE_CAMERA, 0),
        (s::CAM_APPLY, 1),
        (s::RESTORE_SCREEN_SETUP, 0),
        (s::SG_SET_FOCUS, 2),
        (ffx_battle::chr_damage::bridge::SG_EN_FOCUS, 1),
    ]
};

/// `getNearPlane` (`0x00638e70`) as `fld dword [at]; ret` in both oracles.
fn near_plane_stub(o: &mut Oracle, at: u32) {
    let f = o.rebase(ffx_battle::screen::bridge::GET_NEAR_PLANE);
    let code = [[0xd9u8, 0x05].as_slice(), &at.to_le_bytes(), &[0xc3]].concat();
    o.write(f, &code).unwrap();
}

/// `MsScreenProcess` (round 27) on three screens of random floats (links
/// 0-4, short move lanes, shakes, focus words at times the camera's) and
/// dressed cameras, the override, the camera matrices, magic and map
/// numbers, the special levels' bounds and the near-plane state drawn;
/// the renderer's callees stubbed ([`SCREEN_STUBS`]).
fn screen_process(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, rec: u32, near: u32) {
    use ffx_battle::camera::bridge as cam;
    use ffx_battle::screen::bridge as s;
    camera_requests(a, b, rng, false);
    let cams = cam::CAMERAS.ptr(b).addr;
    let screens = cam::SCREENS.ptr(b).addr;
    for k in 0..3 {
        let base = cams + k * cam::CAMERA_SIZE;
        poke(
            a,
            b,
            base + 0xca8,
            &u32::from(rng.below(3) != 0).to_le_bytes(),
        );
        let sc = screens + k * cam::SCREEN_SIZE;
        poke(a, b, sc, &floats(rng, 0x1c0 / 4));
        poke(
            a,
            b,
            sc + 0xc0,
            &[
                u8::from(rng.below(4) != 0),
                u8::try_from(rng.below(3)).unwrap(),
                u8::try_from(rng.below(5)).unwrap(),
                u8::try_from(rng.below(5)).unwrap(),
            ],
        );
        for l in 0..8 {
            poke(a, b, sc + 0xdc + 0x10 * l, &camera_lane(rng));
        }
        for l in 0..2 {
            poke(a, b, sc + 0x16c + 0x24 * l, &shake_record(rng));
        }
        // focus words: at times the camera's (one of the three differing)
        let words: Vec<u8> = (0..3).flat_map(|_| rng.below(4).to_le_bytes()).collect();
        poke(a, b, base + 0xcac, &words);
        let mut mine = words.clone();
        if rng.below(2) == 0 {
            let w = 4 * rng.below(3) as usize;
            mine[w] ^= 1;
        }
        poke(a, b, sc + 0xd0, &mine);
    }
    set(a, b, cam::OVERRIDE_ON, u32::from(rng.below(5) == 0));
    set(a, b, cam::MATRIX_ON, u32::from(rng.below(5) == 0));
    poke(a, b, cam::OVERRIDE_ROLL.ptr(b).addr, &floats(rng, 13));
    poke(
        a,
        b,
        ffx_battle::misc::bridge::SCREEN_OFFSET.ptr(b).addr,
        &floats(rng, 2),
    );
    let size = [
        [640i32, 448],
        [512, 416],
        [rng.range(-900, 900), rng.range(-900, 900)],
        // (sizes past 2^24 round as `f32`)
        [rng.next().cast_signed(), rng.next().cast_signed()],
    ][rng.below(4) as usize];
    set(a, b, cam::DISPLAY_SIZE, size);
    set(a, b, s::NEAR_DEFAULT, u8::from(rng.below(2) == 0));
    set(
        a,
        b,
        s::GRAPHIC_MAP_ID,
        [0xd0u32, 0xd1][rng.below(2) as usize],
    );
    let sign = |rng: &mut Xorshift| -> i32 { if rng.below(4) == 0 { 5 } else { -1 } };
    set(
        a,
        b,
        ffx_battle::parse::bridge::EFFECT_READ_CLEAR,
        sign(rng).cast_unsigned(),
    );
    set(a, b, s::BTL_EVENT_EFFECT_CLEAR, sign(rng));
    poke(a, b, near, &float(rng).to_le_bytes());
    // the special levels' bounds: the eye and look-at at times far or near,
    // or with squared lengths exactly 0.01 (as f32: 1053² + 3103² over
    // 2^30) or 1e9 (31600² + 1200²)
    for o in [0x40, 0x65c] {
        let c = cams + rng.below(3) * cam::CAMERA_SIZE + o;
        let v: [f32; 4] = match rng.below(5) {
            0 => [1053.0 / 32768.0, 3103.0 / 32768.0, 0.0, 0.0],
            1 => [31600.0, 1200.0, 0.0, 0.0],
            k => {
                let scale = [0.001f32, 1.0, 1e5][k as usize - 2];
                std::array::from_fn(|_| f32::from_bits(float(rng)) * scale)
            }
        };
        let v: Vec<u8> = v.iter().flat_map(|x| x.to_bits().to_le_bytes()).collect();
        poke(a, b, c, &v);
    }
    let hash = [0x1505u32, 0x822, 0x1883, 0x77][rng.below(4) as usize];
    let magic = [0x98u32, 0x259, 0x21c, 0x5][rng.below(4) as usize];
    let restore = u32::from(rng.below(3) == 0);
    for (k, (f, _)) in (0u32..).zip(SCREEN_CALLEES) {
        let ret = match f {
            s::LEVEL_NAME_HASH => hash,
            s::MAGIC_ID => magic,
            s::NEED_RESTORE_CAMERA => restore,
            _ => 0,
        };
        let at = rec + EXCHANGE_REC * k + 4;
        poke(a, b, at, &ret.to_le_bytes());
    }
    let camera = [0i32, 1, 2, 1, 3, 4][rng.below(6) as usize];
    zero_stack(a);
    twin(
        "MsScreenProcess",
        a,
        b,
        0x007b_c110,
        &[camera.cast_unsigned()],
        ALL,
        |m| {
            let mut call = game_call_o;
            let mut fcall = |m: &mut Oracle, f: u32, args: &[u32]| {
                f80_to_f32(m.call(f, Conv::Cdecl, args).unwrap().st0_raw.unwrap())
            };
            let mut env = s::ScreenEnv {
                trig: &stand_in,
                trig64: &trig64,
                cos: &cos_stand_in,
                call: &mut call,
                fcall: &mut fcall,
            };
            s::ms_screen_process(m, &mut env, camera).cast_unsigned()
        },
    );
    // MsCameraMatrix's four stack words it never wrote (its port writes 0)
    for o in [0x7d8, 0x7dc, 0x7e8, 0x7ec] {
        let at = a.rebase(0x00c8_f000 + o);
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
    same("MsScreenProcess", a, b);
}

fn screen_process_check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let rec = scratch(a, b, EXCHANGE_REC as usize * SCREEN_CALLEES.len());
    let near = scratch(a, b, 4);
    for o in [&mut *a, &mut *b] {
        for (k, (f, n)) in (0u32..).zip(SCREEN_CALLEES) {
            logging_stub(o, f, SCREEN_STUBS + 0x40 * k, rec + EXCHANGE_REC * k, n, 0);
        }
        near_plane_stub(o, near);
    }
    dressed_check(a, b, rng, synthetic, (100, 4), |a, b, rng, _, _| {
        screen_process(a, b, rng, rec, near);
    });
}

#[test]
fn screen_process_matches_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    screen_process_check(&mut a, &mut b, &mut Xorshift(0x27ca_0301), true);
}

#[test]
fn screen_process_matches_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    screen_process_check(&mut a, &mut b, &mut Xorshift(0x27ca_0302), false);
}

/// `MsCameraLimits_007c0120` (round 35) on [`near_channel`]'s channels:
/// the line limits (kinds 0, 2) with their `+8`/`+0xc` at times 0, the
/// height limits (1, 3) with the position around their bounds, the spline
/// ones (4, 5) through the original near-point search in both. The scale's
/// other words are the harness's registers. `MsVectorGetSide`'s two `w`
/// words are its own frame's garbage in `a`: copied from `a`.
fn camera_limits(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    buf: u32,
    reach: &mut [u32; 8],
) {
    use ffx_battle::camera::bridge as cam;
    let h = host();
    let (side, chan) = near_channel(a, b, rng, buf);
    let n = 1 + rng.below(5);
    poke(a, b, chan + 4, &[u8::try_from(n).unwrap()]);
    for k in 0..n {
        let e = chan + 0x90 + 0x20 * k;
        let kind = if rng.below(10) == 0 {
            rng.next()
        } else {
            rng.below(6)
        };
        poke(a, b, e, &kind.to_le_bytes());
        reach[(kind as usize).min(7)] += 1;
        if rng.below(4) == 0 {
            poke(a, b, e + 8, &0u32.to_le_bytes());
        }
        if rng.below(4) == 0 {
            poke(
                a,
                b,
                e + 0xc,
                &[0u32, 0x7fc0_0000][rng.below(2) as usize].to_le_bytes(),
            );
        }
        // a height range around the side's y, sometimes the wrong way round
        let y = f32::from_bits(ffx_bridge::Mem::read(a, side + 0x54));
        if rng.below(8) == 0 {
            let at = e + [0x18, 0x1c][rng.below(2) as usize];
            poke(a, b, at, &0x8000_0000u32.to_le_bytes());
        } else if rng.below(2) == 0 && y.is_finite() {
            let r = f32::from(u8::try_from(rng.below(40)).unwrap()) / 4.0;
            let (lo, hi) = if rng.below(4) == 0 {
                (y + r, y - r)
            } else {
                (y - r, y + r)
            };
            poke(a, b, e + 0x18, &lo.to_bits().to_le_bytes());
            poke(a, b, e + 0x1c, &hi.to_bits().to_le_bytes());
        }
    }
    zero_stack(a);
    // `_CIatan` as `x²` here (`fmul st, st`), which the argument's 53 bits
    // reach (the shared stand-in, `-x`, rounds alike from 24)
    for o in [&mut *a, &mut *b] {
        let at = o.rebase(0x0094_9d5c);
        o.write(at, &[0xd8, 0xc8, 0xc3]).unwrap();
    }
    let square = |x: f64| ffx_bridge::x87::store(ffx_bridge::x87::mul(x, x));
    let args = [side, chan];
    twin(
        "MsCameraLimits_007c0120",
        a,
        b,
        cam::CAMERA_LIMITS,
        &args,
        ALL,
        |m| {
            dm(m, |m| {
                cam::ms_camera_limits(m, &h, &square, side, chan, Some(HARNESS))
            })
        },
    );
    let vm = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    for at in [vm + 0xc, vm + 0x1c] {
        let w = a.read(at, 4).unwrap();
        b.write(at, &w).unwrap();
    }
    same("MsCameraLimits_007c0120", a, b);
}

fn camera_limits_check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let mut reach = [0u32; 8];
    dressed_check(a, b, rng, synthetic, (60, 6), |a, b, rng, _, buf| {
        camera_limits(a, b, rng, buf, &mut reach);
    });
    eprintln!("limit kinds: {reach:?}");
}

#[test]
fn camera_limits_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    camera_limits_check(&mut a, &mut b, &mut Xorshift(0x35c0_0120), true);
}

#[test]
fn camera_limits_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    camera_limits_check(&mut a, &mut b, &mut Xorshift(0x35c0_0121), false);
}
