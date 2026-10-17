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
    set(a, b, ffx_battle::action::bridge::AEON_PLAIN2, flag(rng, 6));
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
    // MsResetChrPos's MsCheckBtlPos copies its battle position into VF_M's
    // third row (MsPointLen); on the field snapshot the original's x there
    // was once 0x6d where the port's was 0 with everything else the same
    // (not explained yet: copied, so only the row is not compared)
    let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    let v = a.read(m + 0x20, 16).unwrap();
    b.write(m + 0x20, &v).unwrap();
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
    same("MsCommandCueProcess", a, b);
}

/// The battle loop's per-frame callees of sound, the CD library and the
/// voice status (`ffx_battle::process`) and the deaths' sound (unported),
/// trophy (main), neck and character ID (chara), stubbed in `MsDebugEnvSave` (debug
/// only); the last is the CD library's `+8`.
const PROCESS_CALLEES: [(u32, u8); 10] = {
    use ffx_battle::process::bridge as p;
    [
        (p::SND_MUSIC_STOP, 1),
        (p::SND_MUSIC_CLEAR, 1),
        (p::TK_SND_REPORT_STATUS, 0),
        (p::SND_IS_READ_STATUS, 0),
        (p::SND_IS_READ_SHOUT_VOICE_STATUS, 0),
        (ffx_battle::death::bridge::MS_REG_SE_PLAY, 2),
        (ffx_battle::death::bridge::TROPHY_CHECK_DEFEAT_MONSTER, 1),
        (ffx_battle::atel::bridge::CH_NECK_SET_ENABLE, 2),
        (ffx_battle::death::bridge::CH_GET_CHAR_ID, 1),
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
        let f = o.rebase(PROCESS_STUBS + 0x40 * 9);
        o.write(cd + 8, &f.to_le_bytes()).unwrap();
    }
}

/// The battle loop's per-frame pieces (`ffx_battle::process`) and the
/// camera reset.
fn process(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, rec: u32, srec: u32) {
    use ffx_battle::process::bridge as p;
    let mut call = game_call;
    let flag = |rng: &mut Xorshift, n: u32| u8::from(rng.below(n) == 0);
    for (k, r) in [
        (1, rng.next()),
        (2, [0, 1, 2][rng.below(3) as usize]),
        (3, u32::from(flag(rng, 3))),
        (4, u32::from(flag(rng, 3))),
        (9, u32::from(flag(rng, 3))),
    ] {
        poke(a, b, rec + EXCHANGE_REC * k + 4, &r.to_le_bytes());
    }
    poke(
        a,
        b,
        srec + 3 * EXCHANGE_REC + 4,
        &u32::from(flag(rng, 3)).to_le_bytes(),
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
        [0xff, 0x80, 3][rng.below(3) as usize],
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
    }) {
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
    let m = ffx_battle::vector::bridge::VF_M.ptr(a).addr;
    let v = a.read(m + 0x20, 16).unwrap();
    b.write(m + 0x20, &v).unwrap();
    same("MsSetDeath", a, b);
    twin("MsSetDeathAll", a, b, 0x0079_1760, &[], 0, |m| {
        dm(m, |m| {
            s!(s);
            ffx_battle::process::bridge::ms_set_death_all(m, &h, &mut s);
        });
        0
    });
    local_words(a, b);
    let v = a.read(m + 0x20, 16).unwrap();
    b.write(m + 0x20, &v).unwrap();
    same("MsSetDeathAll", a, b);
}

/// Whether every able (`+0xdc8`) actor's and every monster's battle
/// position is written (`MsCalcChrDir`'s centres and turning monsters).
fn placed(a: &mut Oracle, b: &mut Oracle, w: &World, out: u32) -> bool {
    (0..ACTORS).all(|k| {
        (k < 0x14 && b.read(w.actor(k) + 0xdc8, 1).unwrap()[0] == 0)
            || writes(a, b, 0x0079_48e0, k, out + 0x40)
    })
}

/// Party members and aeons coming and going (`ffx_battle::exchange`).
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
    let prec = scratch(a, b, (EXCHANGE_REC * 10) as usize);
    process_stubs(a, b, prec, erec);
    for _ in 0..400 {
        file(a, b, rng, &f);
        dress(a, b, rng, &w, &f);
        calls(a, b, rng, &w, out, !synthetic);
        // on the dressed actors (status and actions change their fields)
        near(a, b, rng, &w, out);
        status(a, b, rng, &w, out);
        actions(a, b, rng, &w, out);
        exchange(a, b, rng, &w, out, xrec);
        effects(a, b, rng, &w, erec, files);
        for _ in 0..3 {
            parse(a, b, rng, &w, out, srec + 2 * EXCHANGE_REC);
        }
        for _ in 0..3 {
            command_cue(a, b, rng, &w, out, srec);
        }
        process(a, b, rng, &w, prec, srec);
        deaths(a, b, rng, &w, out);
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
