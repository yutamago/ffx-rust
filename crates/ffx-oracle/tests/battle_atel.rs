//! The battle's script requests (`ffx_battle::atel`: `MsAtelRequestExe`,
//! `MsSetMotion` and the handler lookups) against the original, as in
//! `battle.rs`: the shared battle world, random request queues, owners and
//! handlers, on the image alone and on both snapshots (whose battle has
//! live scripts). The script calls run as the originals in the Rust's
//! oracle, but for the script code itself (`AtelDecodeSignal`,
//! `AtelDecodeCommand`: the same recording stubs in both,
//! `AtelGetRequestLevel` the same random level); `Sg_Blur` (main) is the
//! same stub in both.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::crt::{patch, stand_in};
use battle_common::world::{ACTORS, World, world};
use battle_common::*;
use ffx_battle::atel::bridge as at;
use ffx_battle::state::bridge as st;
use ffx_oracle::{Conv, Oracle};

/// The signal block's scratch in both oracles, and the original's
/// `AtelRegistSignal2`.
const REGIST_SIGNAL2: u32 = 0x0086_e9e0;

fn script_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

fn signal(block: u32) -> impl FnMut(&mut Oracle, [u32; 8]) -> u32 {
    move |m, words| {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        m.write(block, &bytes).unwrap();
        let at = m.rebase(REGIST_SIGNAL2);
        let r = m.call(at, Conv::Cdecl, &[block]).unwrap().eax;
        m.write(block, &[0; 0x20]).unwrap();
        r
    }
}

/// A random request queue: kinds 2-4 (sometimes others), owners and
/// handlers mostly plausible.
fn queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let n = rng.range(-1, 17);
    set(a, b, st::ATEL_COUNT, n);
    for e in 0..16 {
        let kind = match rng.below(8) {
            0 => rng.below(7),
            1 | 2 => 3,
            3 => 4,
            _ => 2,
        };
        let who = match kind {
            2 => rng.below(3),
            3 => rng.below(ACTORS),
            _ => rng.below(2),
        };
        let which = match rng.below(4) {
            0 => rng.below(0x48),
            1 => 0x40,
            _ => rng.below(8),
        };
        let k = rng.range(-1, 8).cast_unsigned();
        let r: Vec<u8> = [kind, who, which, k]
            .iter()
            .flat_map(|v| u16::try_from(v & 0xffff).unwrap().to_le_bytes())
            .collect();
        poke(a, b, st::ATEL_REQUEST.ptr(b).addr + 8 * e, &r);
    }
}

/// A [`at::Script`] onto the originals in the Rust's oracle.
macro_rules! script {
    ($s:ident, $block:expr) => {
        let mut call = script_call;
        let mut sig = signal($block);
        let mut off = |m: &mut Oracle, e| ffx_vfx::et::bridge::op_et_eff_off_signal(m, e);
        let mut $s = at::Script {
            call: &mut call,
            signal: &mut sig,
            off: &mut off,
        };
    };
}

fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, block: u32, worker: u32) {
    let kind = [2, 2, 3, 4, rng.below(6)][rng.below(5) as usize];
    let i = match kind {
        2 => rng.below(3),
        3 => rng.below(ACTORS + 1),
        _ => rng.below(2),
    };
    let n = match rng.below(4) {
        0 => rng.below(0x48),
        1 => rng.below(ACTORS) + 5,
        _ => rng.below(8),
    };
    let out = w.out;
    twin(
        "MsAtelGetHandler",
        a,
        b,
        0x0079_8590,
        &[kind, i, n, out],
        ALL,
        |m| at::ms_atel_get_handler(m, kind, i, n.cast_signed(), out),
    );
    same("MsAtelGetHandler", a, b);
    // the work of script controls 3 and 4 only as the game set it: no
    // faked actor or camera handlers past the lookup
    if !w.synthetic && matches!(kind, 3 | 4) {
        set(a, b, at::CHR_SCRIPTS, 0);
        set(a, b, ffx_battle::minor::bridge::ATEL_CAM_ON, 0);
    }
    queue(a, b, rng);
    // a worker of kind's execution space now and then
    fill(a, b, rng, worker, 0x100);
    if rng.below(2) == 0 {
        poke(a, b, worker + 0xae, &[u8::try_from(kind & 0xf).unwrap()]);
    }
    if rng.below(2) == 0 {
        let (h, base) = at::atel_get_handler(b, kind, i, n.cast_signed());
        if h != 0 {
            let wk = u32::from(b.read(h, 1).unwrap()[0]) + base.unwrap_or(0);
            poke(
                a,
                b,
                worker + 0x2e,
                &u16::try_from(wk & 0xffff).unwrap().to_le_bytes(),
            );
        }
    }
    let actor = if rng.below(3) == 0 { worker } else { 0 };
    let flags = rng.below(8);
    let o = if rng.below(4) == 0 { 0 } else { out };
    let args = [kind, i, n, flags, actor, o];
    twin("MsAtelRequestExe", a, b, 0x0079_7d50, &args, ALL, |m| {
        script!(s, block);
        at::ms_atel_request_exe(m, &mut s, kind, i, n.cast_signed(), flags, actor, o)
            .cast_unsigned()
    });
    same("MsAtelRequestExe", a, b);
    let bonus = rng.byte();
    twin(
        "MsAtelCameraPriority",
        a,
        b,
        0x0079_8760,
        &[kind, i, n, u32::from(bonus)],
        0,
        |m| {
            script!(s, block);
            at::ms_atel_camera_priority(m, &mut s, kind, i, n.cast_signed(), bonus);
            0
        },
    );
    same("MsAtelCameraPriority", a, b);
    twin(
        "MsAtelParseSysTag",
        a,
        b,
        0x0079_79d0,
        &[kind, i, n],
        ALL,
        |m| {
            script!(s, block);
            at::ms_atel_parse_sys_tag(m, &mut s, kind, i, n.cast_signed()).cast_unsigned()
        },
    );
    same("MsAtelParseSysTag", a, b);
    // a motion
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    for at in [0x39, 0x3fa, 0x414, 0x429, 0x432, 0x448, 0x50d, 0xdf2] {
        fill(a, b, rng, p + at, 1);
    }
    let motion = match rng.below(5) {
        0 => rng.next(),
        _ => rng.below(0x60),
    };
    let wait = rng.below(2) * rng.next();
    let x = rng.next();
    queue(a, b, rng);
    let flags = rng.below(8);
    let args = [i, motion, wait, x, flags, actor, o];
    twin("MsSetMotion", a, b, 0x007a_b360, &args, ALL, |m| {
        script!(s, block);
        at::ms_set_motion(
            m,
            &mut s,
            i,
            motion.cast_signed(),
            wait,
            x.to_le_bytes()[0],
            flags,
            actor,
            o,
        )
    });
    same("MsSetMotion", a, b);
    // the motions around it
    let p = w.actor(rng.below(ACTORS));
    poke(a, b, p + 0x6d1, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
    queue(a, b, rng);
    twin("MsPaymentProcessStart", a, b, 0x007a_fa80, &[p], 0, |m| {
        script!(s, block);
        at::ms_payment_process_start(m, &mut s, p);
        0
    });
    same("MsPaymentProcessStart", a, b);
    let i = rng.below(ACTORS);
    let voice = match rng.below(4) {
        0 => rng.next(),
        _ => rng.below(0x20),
    };
    let flags = rng.below(8);
    queue(a, b, rng);
    twin(
        "MsSetVoiceCheck",
        a,
        b,
        0x007a_07b0,
        &[i, voice, flags, actor, o],
        ALL,
        |m| {
            script!(s, block);
            at::ms_set_voice_check(m, &mut s, i, voice.cast_signed(), flags, actor, o)
        },
    );
    same("MsSetVoiceCheck", a, b);
    // the head targets (the neck is chara's: actors without a character
    // on the image)
    let p = w.actor(rng.below(ACTORS));
    for at in [0x438, 0x439, 0xdce] {
        let v = if rng.below(2) == 0 {
            0xff
        } else {
            rng.below(3)
        };
        poke(a, b, p + at, &[u8::try_from(v).unwrap()]);
    }
    if w.synthetic {
        poke(a, b, p, &[0; 4]);
    }
    twin("msResetHeadTarget", a, b, 0x007b_1f10, &[p], ALL, |m| {
        at::ms_reset_head_target(m, &mut script_call, p)
    });
    same("msResetHeadTarget", a, b);
    // a command's targets taking their motion
    let chr = rng.below(ACTORS);
    let cue = w.out + 0x40;
    fill(a, b, rng, cue, 0x48);
    let k = rng.below(4);
    let id: u16 = if rng.below(8) == 0 {
        0xff
    } else {
        u16::try_from(0x3000 + rng.below(0x40)).unwrap()
    };
    poke(
        a,
        b,
        cue + 8 + 16 * k,
        &[id.to_le_bytes(), [0xff, 0]].concat(),
    );
    poke(
        a,
        b,
        cue + 0x10 + 16 * k,
        &(rng.next() & 0x7fff_ffff).to_le_bytes(),
    );
    for j in 0..ACTORS {
        let q = w.actor(j);
        poke(a, b, q + 0x422, &[u8::try_from(rng.below(ACTORS)).unwrap()]);
        for at in [0x4ff, 0xdcc, 0x438] {
            let v = if rng.below(2) == 0 { 0 } else { rng.byte() };
            poke(a, b, q + at, &[v]);
        }
        if w.synthetic {
            poke(a, b, q, &[0; 4]);
        }
    }
    queue(a, b, rng);
    let far = if rng.below(4) == 0 { 0 } else { w.out + 0x90 };
    twin(
        "MsSetTargetsMotion",
        a,
        b,
        0x0079_1fa0,
        &[chr, cue, k, far],
        ALL,
        |m| {
            script!(s, block);
            at::ms_set_targets_motion(m, &mut s, chr, cue, k, far).cast_unsigned()
        },
    );
    same("MsSetTargetsMotion", a, b);
}

/// What `MsDefenseStartProcess` gets for its caller's `edi`, `esi`, `ebx`:
/// the oracle harness's.
const HARNESS: ffx_battle::target::bridge::Regs = ffx_battle::target::bridge::Regs {
    edi: 0x3333_3333,
    esi: 0x2222_2222,
    ebx: 0x1111_1111,
};

/// Defenders stepping in (a battle's live actors: positions the game
/// set): some able actors (`+0xdc8`) defend another from a third.
fn defense(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, block: u32) {
    for i in 0..ACTORS {
        let p = w.actor(i);
        let (from, to) = if w.synthetic || rng.below(4) != 0 {
            (rng.byte(), 0xff)
        } else {
            (
                u8::try_from(rng.below(ACTORS)).unwrap(),
                u8::try_from(rng.below(ACTORS)).unwrap(),
            )
        };
        poke(a, b, p + 0x6d3, &[from, to]);
        poke(a, b, p + 0xdc8, &[u8::from(rng.below(3) != 0)]);
        fill(a, b, rng, p + 0x411, 1);
        fill(a, b, rng, p + 0x616, 2);
    }
    queue(a, b, rng);
    twin("MsDefenseStartProcess", a, b, 0x007a_f920, &[], 0, |m| {
        script!(s, block);
        at::ms_defense_start_process(m, &mut s, &stand_in, Some(HARNESS));
        0
    });
    same("MsDefenseStartProcess", a, b);
}

/// Handler owners: actors' files and the script camera's as one of the
/// battle's script files (the same handler layout), the switches random,
/// work bases small.
fn owners(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World) {
    let records = at::SCRIPT_RECORDS.ptr(b).addr;
    let file = u32::from_le_bytes(b.read(records + 0x10, 4).unwrap().try_into().unwrap());
    set(a, b, at::CHR_SCRIPTS, u8::from(rng.below(2) == 0));
    set(
        a,
        b,
        ffx_battle::minor::bridge::ATEL_CAM_ON,
        u8::from(rng.below(2) == 0),
    );
    let cam = if rng.below(4) == 0 { 0 } else { file };
    set(a, b, ffx_battle::minor::bridge::ATEL_CAM, [rng.next(), cam]);
    for i in 0..ACTORS {
        let p = w.actor(i);
        let f = if file == 0 || rng.below(3) == 0 {
            0
        } else {
            file
        };
        poke(a, b, p + 0xf7c, &f.to_le_bytes());
        poke(a, b, p + 0x48, &rng.below(2).to_le_bytes());
        poke(a, b, p + 0xdf6, &[u8::try_from(rng.below(2)).unwrap()]);
        fill(a, b, rng, p + 0xdf5, 1);
    }
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0x20);
    for o in [&mut *a, &mut *b] {
        patch(o);
        let at = o.rebase(at::SG_BLUR);
        o.stub_return(at, 0).unwrap();
        script_stubs(o, rec);
    }
    if w.synthetic {
        // our own actors: no monster files; no battle scripts (a field
        // snapshot keeps the last battle's freed ones)
        for i in 0..ACTORS {
            poke(a, b, w.actor(i) + 0xf7c, &[0; 4]);
        }
        let records = at::SCRIPT_RECORDS.ptr(b).addr;
        poke(a, b, records + 0x10, &[0; 4]);
        poke(a, b, records + 0x30, &[0; 4]);
        let flag = scratch(a, b, 4);
        set(a, b, st::VOICE_EVENT, flag);
    }
    let block = scratch(a, b, 0x20);
    let worker = scratch(a, b, 0x100);
    for _ in 0..1500 {
        let level = rng.below(7);
        for o in [&mut *a, &mut *b] {
            let at = o.rebase(at::ATEL_GET_REQUEST_LEVEL);
            o.stub_return(at, level).unwrap();
        }
        let flag = st::VOICE_EVENT.get(b);
        if readable(a, flag, 1) {
            poke(a, b, flag, &[u8::from(rng.below(2) == 0)]);
        }
        if !w.synthetic {
            owners(a, b, rng, &w);
        }
        calls(a, b, rng, &w, block, worker);
        defense(a, b, rng, &w, block);
    }
}

#[test]
fn script_requests_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xa7e1_0001), true);
}

#[test]
fn script_requests_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0xa7e1_0002), false);
}
