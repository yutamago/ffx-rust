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
    /// 0x20 bytes in both oracles for a script signal block (cleared after).
    static BLOCK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
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
    poke(a, b, actor, &fake);
    twin("MsResetChrPos", a, b, 0x0078_d680, &[i, actor], ALL, |m| {
        st::ms_reset_chr_pos(m, &stand_in, i, actor)
    });
    same("MsResetChrPos", a, b);
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
    set(a, b, ac::AEON_PLAIN2, u8::from(rng.below(4) == 0));
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
fn writes(a: &mut Oracle, b: &mut Oracle, sym: u32, i: u32, probe: u32) -> bool {
    use ffx_battle::pos::bridge as p;
    poke(a, b, probe, &[0xff; 16]);
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
    let out = scratch(a, b, 0x100);
    let s = scratch(a, b, 0x10);
    SCRATCH.with(|c| c.set(s));
    let block = scratch(a, b, 0x20);
    BLOCK.with(|c| c.set(block));
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
    for _ in 0..400 {
        file(a, b, rng, &f);
        dress(a, b, rng, &w, &f);
        calls(a, b, rng, &w, out, !synthetic);
        // on the dressed actors (status and actions change their fields)
        near(a, b, rng, &w, out);
        status(a, b, rng, &w, out);
        actions(a, b, rng, &w, out);
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
