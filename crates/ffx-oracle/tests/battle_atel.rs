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

/// Actors shown and hidden (`ffx_battle::visible`) and the camera script
/// request; `op_et_bindeff` (vfx) is a recording stub in both.
fn visible(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    block: u32,
    chars: u32,
    worker: u32,
) {
    use ffx_battle::visible::bridge as vi;
    let mut call = script_call;
    for i in 0..ACTORS {
        let p = w.actor(i);
        if w.synthetic {
            let c = if rng.below(4) == 0 {
                0
            } else {
                chars + 0x1000 * rng.below(3)
            };
            poke(a, b, p, &c.to_le_bytes());
        }
        let req: Vec<u8> = (0..4).map(|_| u8::from(rng.below(5) != 0)).collect();
        poke(a, b, p + 0x16, &req);
        let bound = if rng.below(6) == 0 {
            u8::try_from(rng.below(ACTORS)).unwrap()
        } else {
            0xff
        };
        // no cycles: an actor is bound only to a later one
        let bound = if u32::from(bound) > i { bound } else { 0xff };
        poke(a, b, p + 0x13, &[bound]);
        for o in [0x1d, 0x36, 0x440] {
            poke(a, b, p + o, &[u8::from(rng.below(4) == 0)]);
        }
        let on = if rng.below(4) == 0 {
            u8::try_from(rng.below(ACTORS)).unwrap()
        } else {
            0xff
        };
        let on = if u32::from(on) < i { on } else { 0xff };
        poke(a, b, p + 0x43f, &[on]);
        let shadow: Vec<u8> = (0..8)
            .map(|_| [0xff, 0, 1, 2, rng.byte()][rng.below(5) as usize])
            .collect();
        poke(a, b, p + 0x25, &shadow);
        fill(a, b, rng, p + 0x32, 2);
    }
    if w.synthetic {
        let top = ffx_battle::actor::bridge::WEAPON_TOP.get(b);
        for k in 0..2 * ACTORS {
            let r = top + k * ffx_battle::actor::WEAPON_SIZE;
            let c = if rng.below(3) == 0 {
                chars + 0x1000 * rng.below(3)
            } else {
                0
            };
            poke(a, b, r, &c.to_le_bytes());
            let shadow: Vec<u8> = (0..8)
                .map(|_| [0xff, 0, 1, rng.byte()][rng.below(4) as usize])
                .collect();
            poke(a, b, r + 0x25, &shadow);
        }
    }
    set(a, b, vi::HIDE_MASK, rng.below(4));
    set(a, b, vi::HIDE_LATCH, rng.below(4));
    set(a, b, vi::HIDE_QUEUE_LAST, rng.range(0, 8));
    set(a, b, vi::CAMERA_HIDES, u8::from(rng.below(2) == 0));
    let i = rng.below(ACTORS);
    let (k, v) = (rng.below(4), [0, 1, rng.next()][rng.below(3) as usize]);
    twin(
        "msSetChrVisibleReq",
        a,
        b,
        0x0079_7080,
        &[i, k, v],
        ALL,
        |m| vi::ms_set_chr_visible_req(m, &mut call, i, k, v),
    );
    same("msSetChrVisibleReq", a, b);
    // its two requests by name, and running away
    let (j, v) = (rng.below(ACTORS), [0, 1, rng.next()][rng.below(3) as usize]);
    let (what, sym) = [
        ("MsSetChrVisible", 0x0079_6660),
        ("MsSetChrVisibleAppear", 0x0079_66a0),
    ][rng.below(2) as usize];
    twin(what, a, b, sym, &[j, v], ALL, |m| {
        if sym == 0x0079_6660 {
            vi::ms_set_chr_visible(m, &mut call, j, v)
        } else {
            vi::ms_set_chr_visible_appear(m, &mut call, j, v)
        }
    });
    same(what, a, b);
    let away = [0, 1, rng.next()][rng.below(3) as usize];
    if rng.below(4) == 0 {
        twin("MsSetEscapeFlagAll", a, b, 0x0078_de80, &[away], 0, |m| {
            ffx_battle::chr_damage::bridge::ms_set_escape_flag_all(m, &mut call, away);
            0
        });
        same("MsSetEscapeFlagAll", a, b);
    } else {
        let p = w.actor(j);
        twin(
            "MsSetEscapeFlag",
            a,
            b,
            0x0078_de40,
            &[j, p, away],
            0,
            |m| {
                ffx_battle::chr_damage::bridge::ms_set_escape_flag(m, &mut call, j, p, away);
                0
            },
        );
        same("MsSetEscapeFlag", a, b);
    }
    let p = w.actor(rng.below(ACTORS));
    let k = rng.range(-2, 5);
    let v = rng.next();
    twin(
        "msSetChrShadowReq",
        a,
        b,
        0x0079_7020,
        &[p, k.cast_unsigned(), v],
        ALL,
        |m| vi::ms_set_chr_shadow_req(m, p, k, v.to_le_bytes()[0]),
    );
    same("msSetChrShadowReq", a, b);
    let v = u32::from(rng.below(2) == 0);
    if rng.below(4) == 0 {
        twin(
            "MsSetChrVisibleCameraAll",
            a,
            b,
            0x0079_67c0,
            &[v],
            ALL,
            |m| vi::ms_set_chr_visible_camera_all(m, &mut call, v),
        );
    } else {
        twin(
            "MsSetChrVisibleCamera",
            a,
            b,
            0x0079_67a0,
            &[i, v],
            ALL,
            |m| vi::ms_set_chr_visible_camera(m, &mut call, i, v),
        );
    }
    same("MsSetChrVisibleCamera", a, b);
    twin("MsReturnCameraVisible", a, b, 0x0079_5c70, &[], 0, |m| {
        vi::ms_return_camera_visible(m, &mut call);
        0
    });
    same("MsReturnCameraVisible", a, b);
    // the camera script request
    set(a, b, at::CAMERA_LOCK, u8::from(rng.below(5) == 0));
    set(a, b, at::CAMERA_STATE, u8::try_from(rng.below(4)).unwrap());
    set(
        a,
        b,
        ffx_battle::minor::bridge::EFFECT_CAMERA_MODE,
        u8::from(rng.below(4) == 0),
    );
    set(a, b, vi::CAMERA_HIDES, u8::from(rng.below(2) == 0));
    queue(a, b, rng);
    let req = |rng: &mut Xorshift| [0xffff, rng.below(8), rng.below(0x48)][rng.below(3) as usize];
    let monster = [0xff, rng.below(ACTORS), 0x14 + rng.below(8)][rng.below(3) as usize];
    let args = [
        rng.below(0x48),
        monster,
        req(rng),
        req(rng),
        rng.below(2),
        rng.below(8),
        if rng.below(2) == 0 { 0 } else { worker },
        0,
    ];
    twin("MsAtelRequestCamera", a, b, 0x0079_7bc0, &args, ALL, |m| {
        script!(s, block);
        at::ms_atel_request_camera(
            m, &mut s, args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
        )
        .cast_unsigned()
    });
    same("MsAtelRequestCamera", a, b);
    // no bindings left for the other tests' positions
    for i in 0..ACTORS {
        poke(a, b, w.actor(i) + 0x13, &[0xff]);
        poke(a, b, w.actor(i) + 0x43f, &[0xff]);
    }
}

/// The command parser's smaller callees: the magic camera request,
/// escaping, overdrive types, the music volume and the title window
/// (sound's and system's functions recording stubs in both).
fn parse_callees(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    w: &World,
    block: u32,
    worker: u32,
) {
    use ffx_battle::exec::bridge as e;
    let mut call = script_call;
    set(a, b, at::CAMERA_LOCK, u8::from(rng.below(5) == 0));
    set(a, b, at::CAMERA_STATE, u8::try_from(rng.below(4)).unwrap());
    set(
        a,
        b,
        ffx_battle::minor::bridge::EFFECT_CAMERA_MODE,
        u8::from(rng.below(4) == 0),
    );
    queue(a, b, rng);
    let req = |rng: &mut Xorshift| [0xffff, rng.below(8), rng.below(0x48)][rng.below(3) as usize];
    let id = [0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0][rng.below(7) as usize]
        + rng.below(0x20);
    let args = [
        rng.below(8),
        [0xff, rng.below(ACTORS), 0x14 + rng.below(8)][rng.below(3) as usize],
        id,
        req(rng),
        req(rng),
        rng.below(2),
        rng.below(8),
        if rng.below(2) == 0 { 0 } else { worker },
        0,
    ];
    twin(
        "MsAtelRequestMagicCamera",
        a,
        b,
        0x0079_8000,
        &args,
        ALL,
        |m| {
            script!(s, block);
            at::ms_atel_request_magic_camera(
                m, &mut s, args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
                args[8],
            )
            .cast_unsigned()
        },
    );
    same("MsAtelRequestMagicCamera", a, b);
    // overdrive types: the counters in ply_save
    set(
        a,
        b,
        ffx_battle::save::bridge::STEAL_RULE,
        u8::from(rng.below(5) == 0),
    );
    let ply = ffx_battle::save::bridge::PLY_SAVE.ptr(b).addr;
    for c in 0..7 {
        let r = ply + 0x94 * c;
        let counts: Vec<u8> = (0..0x11)
            .flat_map(|_| {
                [0u16, 1, 2, 0xffff, u16::try_from(rng.below(30)).unwrap()][rng.below(5) as usize]
                    .to_le_bytes()
            })
            .collect();
        poke(a, b, r + 0x60, &counts);
        poke(a, b, r + 0x88, &rng.next().to_le_bytes());
    }
    for c in 0..ACTORS {
        let p = w.actor(c);
        poke(a, b, p + 0x6f0, &(rng.next() & rng.next()).to_le_bytes());
        for o in [0xdc8, 0xdcc, 0xdce, 0x41c] {
            poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
        }
        poke(
            a,
            b,
            p + 0x5bb,
            &[[0xc, 0, rng.byte()][rng.below(3) as usize]],
        );
    }
    let (i, k, force) = (rng.below(9), rng.below(0x12), rng.below(2));
    twin(
        "MsLimitTypeExpGet",
        a,
        b,
        0x007b_10c0,
        &[i, k, force],
        ALL,
        |m| e::ms_limit_type_exp_get(m, i, k, force),
    );
    same("MsLimitTypeExpGet", a, b);
    let i = rng.below(8);
    let actor = w.actor(i);
    twin(
        "MsLimitTypeEscapeCheck",
        a,
        b,
        0x007b_1090,
        &[i, actor],
        ALL,
        |m| e::ms_limit_type_escape_check(m, i, actor),
    );
    same("MsLimitTypeEscapeCheck", a, b);
    let party: Vec<u8> = (0..7)
        .map(|_| {
            if rng.below(4) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(8)).unwrap()
            }
        })
        .collect();
    poke(
        a,
        b,
        ffx_battle::save::bridge::BATTLE_PARTY.ptr(b).addr,
        &party,
    );
    queue(a, b, rng);
    let kept: Vec<Vec<u8>> = (0..ACTORS)
        .map(|k| b.read(w.actor(k) + 0x4fc, 3).unwrap())
        .collect();
    twin("MsEscapeAll", a, b, 0x007a_da60, &[], 0, |m| {
        script!(s, block);
        at::ms_escape_all(m, &mut s);
        0
    });
    same("MsEscapeAll", a, b);
    // the battle position kinds back (the defense test's positions must be
    // written; battle_pos.rs)
    for (k, v) in kept.iter().enumerate() {
        poke(a, b, w.actor(u32::try_from(k).unwrap()) + 0x4fc, v);
    }
    let (vol, frames) = (rng.below(0x80), rng.below(60));
    twin(
        "MsMusicVolume",
        a,
        b,
        0x007a_0090,
        &[vol, frames],
        ALL,
        |m| e::ms_music_volume(m, &mut call, vol, frames),
    );
    same("MsMusicVolume", a, b);
    let (k, pos) = (rng.below(8), rng.next());
    twin(
        "MsMenuOpenTitleWindow",
        a,
        b,
        0x0078_fea0,
        &[k, pos],
        ALL,
        |m| e::ms_menu_open_title_window(m, &mut call, k, pos),
    );
    same("MsMenuOpenTitleWindow", a, b);
    // effect files: a table of our own where there is none
    let t = e::EFFECT_FILES.get(b);
    if !readable(a, t, 0x802) {
        let v: Vec<u8> = {
            let mut n = 0u16;
            (0..0x401)
                .flat_map(|_| {
                    n = n.wrapping_add(u16::try_from(rng.below(5)).unwrap());
                    n.to_le_bytes()
                })
                .collect()
        };
        let at = scratch(a, b, 0x802);
        poke(a, b, at, &v);
        set(a, b, e::EFFECT_FILES, at);
    }
    let (n, k) = (rng.range(-1, 0x402), rng.range(-2, 6));
    twin(
        "MsEffectFileNumber",
        a,
        b,
        0x0078_7a70,
        &[n.cast_unsigned(), k.cast_unsigned()],
        ALL,
        |m| e::ms_effect_file_number(m, n, k).cast_unsigned(),
    );
    // sound effect slots: some ids shared between actors
    let ids = [0, 0, 0x1234, 0x5678, rng.next()];
    for c in 0..ACTORS {
        let v: Vec<u8> = (0..32)
            .flat_map(|_| {
                let id = if rng.below(4) == 0 {
                    rng.next()
                } else {
                    ids[rng.below(5) as usize]
                };
                id.to_le_bytes()
            })
            .collect();
        poke(a, b, w.actor(c) + 0xeac, &v);
        poke(a, b, w.actor(c) + 0xc, &[u8::try_from(c).unwrap()]);
    }
    let i = rng.below(ACTORS);
    let shared = rng.below(2);
    twin(
        "MsSEPchrClear",
        a,
        b,
        0x007a_0400,
        &[w.actor(i), shared],
        0,
        |m| {
            e::ms_sep_chr_clear(m, &mut call, w.actor(i), shared);
            0
        },
    );
    same("MsSEPchrClear", a, b);
    // aeons' command table
    let i = 8 + rng.below(10);
    let (mo, va) = if rng.below(4) == 0 {
        (0, 0)
    } else {
        (w.out + 0x40, w.out + 0x44)
    };
    let args = [
        i,
        [0x3100, rng.below(0x40) + 0x3000][rng.below(2) as usize],
        rng.below(4),
        mo,
        va,
    ];
    set(a, b, ffx_battle::save::bridge::MENU, rng.next());
    // commands used (18 characters x 0x16 flag words)
    fill(
        a,
        b,
        rng,
        ffx_battle::save::bridge::COMMAND_USE.ptr(b).addr,
        0x12 * 0x2c,
    );
    twin("MsGetAeonCommand", a, b, 0x007a_f060, &args, ALL, |m| {
        e::ms_get_aeon_command(m, args[0], args[1], args[2], args[3], args[4])
    });
    same("MsGetAeonCommand", a, b);
}

/// Actors turned, placed, coming back, running away and leaving their
/// effects' motions (`ffx_battle::chr_damage`), on the characters
/// [`visible`] set.
fn comebacks(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, w: &World, block: u32, pos: u32) {
    use ffx_battle::chr_damage::bridge as cd;
    let mut call = script_call;
    let i = [rng.below(ACTORS), rng.below(0x40)][rng.below(4).min(1) as usize];
    let dir = battle_common::world::quiet(rng.next());
    twin("MsSetChrDirF", a, b, 0x0079_5e20, &[i, dir], ALL, |m| {
        cd::ms_set_chr_dir_f(m, i, dir).cast_unsigned()
    });
    same("MsSetChrDirF", a, b);
    let v: Vec<u8> = (0..4)
        .flat_map(|_| battle_common::world::quiet(rng.next()).to_le_bytes())
        .collect();
    poke(a, b, pos, &v);
    let p = w.actor(i % ACTORS);
    poke(a, b, p + 0x34, &[u8::from(rng.below(2) == 0)]);
    // only real characters (a battle snapshot's) have a cloth to reset:
    // the fake ones go
    let chr = u32::from_le_bytes(b.read(p, 4).unwrap().try_into().unwrap());
    if w.synthetic || !readable(a, chr, 0x1000) {
        poke(a, b, p, &[0; 4]);
    }
    twin("MsSetChrPosReset", a, b, 0x0079_6220, &[i, pos], ALL, |m| {
        cd::ms_set_chr_pos_reset(m, i, pos).cast_unsigned()
    });
    same("MsSetChrPosReset", a, b);
    // coming back (or blown away)
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    poke(a, b, p + 0x5d0, &rng.range(-3, 3).to_le_bytes());
    poke(a, b, p + 0x617, &[u8::from(rng.below(3) == 0)]);
    let weight = battle_common::world::quiet(rng.next());
    poke(a, b, p + 0x4dc, &weight.to_le_bytes());
    let full = rng.below(2);
    if rng.below(2) == 0 {
        twin("MsRelife", a, b, 0x0078_d530, &[i, p, full], 0, |m| {
            cd::ms_relife(m, &mut call, i, p, full);
            0
        });
        same("MsRelife", a, b);
    } else {
        twin("MsBlowProcess", a, b, 0x0078_91b0, &[i, p], ALL, |m| {
            cd::ms_blow_process(m, &mut call, i, p)
        });
        same("MsBlowProcess", a, b);
    }
    // an effect letting go of a motion; a stone shattering
    let i = rng.below(ACTORS);
    let p = w.actor(i);
    for o in [0x436, 0x450] {
        poke(a, b, p + o, &[u8::from(rng.below(3) == 0)]);
    }
    let kind = [0, 1, 3, 4, 5, 6, rng.next()][rng.below(7) as usize];
    queue(a, b, rng);
    twin("MsEffectEndMotion", a, b, 0x0078_7950, &[i, kind], 0, |m| {
        script!(s, block);
        cd::ms_effect_end_motion(m, &mut s, i, kind);
        0
    });
    same("MsEffectEndMotion", a, b);
    queue(a, b, rng);
    twin("MsStoneMove", a, b, 0x007a_2750, &[i], 0, |m| {
        script!(s, block);
        cd::ms_stone_move(m, &mut s, i);
        0
    });
    same("MsStoneMove", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) {
    let w = world(a, b, rng, synthetic);
    let rec = scratch(a, b, 0xa0);
    for o in [&mut *a, &mut *b] {
        patch(o);
        let at = o.rebase(at::SG_BLUR);
        o.stub_return(at, 0).unwrap();
        script_stubs(o, rec);
        recording_stub(o, 0x007f_b290, STUBS + 0x40, rec + 0x20, 1, 0);
        // SndGetNowPlayMusic, SndMusicVolumeTranslate, TOBtlOpenSignWindow
        recording_stub(o, 0x0088_63f0, STUBS + 0x80, rec + 0x40, 0, 0x2d);
        recording_stub(o, 0x0088_6b40, STUBS + 0xc0, rec + 0x50, 3, 7);
        recording_stub(o, 0x0089_8c60, STUBS + 0x100, rec + 0x70, 1, 0);
        // SndSepClear
        recording_stub(o, 0x0088_6d80, STUBS + 0x140, rec + 0x80, 1, 0);
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
    let chars = scratch(a, b, 0x3000);
    let pos = scratch(a, b, 0x10);
    let words: Vec<u8> = (0..0xc00)
        .flat_map(|_| battle_common::world::quiet(rng.next()).to_le_bytes())
        .collect();
    poke(a, b, chars, &words);
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
        visible(a, b, rng, &w, block, chars, worker);
        comebacks(a, b, rng, &w, block, pos);
        // the sign window opens now and then
        let open = rng.below(2);
        poke(a, b, rec + 0x78, &open.to_le_bytes());
        parse_callees(a, b, rng, &w, block, worker);
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
