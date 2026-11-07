//! Round 13 (magic-r13): small own functions and parameter families
//! (`~/ffx-work/m5/magic/r4/pfamilies.tsv`), one port per family reading
//! its parameters from each member's code.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's task listed (internal: `ecx` = the object `O`; family
/// P055, `magic_0610` 0x10004670 and `magic_0673` 0x100043a0, 112 bytes):
/// `O + 0x108` and `+0x104` = the copy's time (the immediates `+0x10`,
/// `+0x1a`), `+0x10c` = the immediate `+0x24`, u16 `+0x126` = 0; a pool
/// node ([`take`](crate::sa_pool::take), the owner `+0x06`) gets `+0x14 =
/// O` and is appended to the task list (first `+0x36`, last `+0x4a`).
/// Returns the node.
pub fn object_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let o = fr.ecx;
    g.write::<u32>(o.wrapping_add(0x108), w(0x10));
    g.write::<u32>(o.wrapping_add(0x104), w(0x1a));
    g.write::<u32>(o.wrapping_add(0x10c), w(0x24));
    g.write::<u16>(o.wrapping_add(0x126), 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x30), w(0x06));
    let first = g.read::<u32>(w(0x36));
    g.write::<u32>(node.wrapping_add(0x14), o);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x63), node);
        g.write::<u32>(w(0x68), node);
    } else {
        let last = g.read::<u32>(w(0x4a));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(w(0x54), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`object_task`] in `magic_0610` (0x10004670; the `rel32`: the pool's
/// take).
pub const OBJECT_TASK_0610: Sig = Sig {
    len: 112,
    hash: 0x7311_c261_c09a_0e5c,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`object_task`] in `magic_0673` (0x100043a0, its time 0x82).
pub const OBJECT_TASK_0673: Sig = Sig {
    len: 112,
    hash: 0x9280_7162_a54d_d6b8,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The three object lists run and, when all of them and the scene word
/// are done, the sound stopped (a pool task, no arguments; family P057,
/// `magic_0304` 0x10001560 and `magic_0568` 0x10001500, 100 bytes): the
/// lists `L1`, `L2`, `L3` (operands `+0x10`, `+0x1c`, `+0x2a`) run
/// ([`host_objects_run`](crate::s9_misc::host_objects_run), `edx` = `*G_T`
/// (`+0x02`) for the first, 0 after); when each answers 1 and `S[D]` is 1
/// (`S = *G_S`, `+0x0b`; `D` the displacement `+0x44`), slot 214 `(*G_SND)`
/// (`+0x51`). Returns the last call's `eax`.
pub fn lists_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x0b));
    // push ebx; push esi; push edi: each list's run sees `esp` below them
    let esp = fr.esp.wrapping_sub(0x10);
    let run = |g: &mut G, call: u32, list: u32, edx: u32| {
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: list,
            edx,
            ..*fr
        };
        crate::s9_misc::host_objects_run(g, img, &callee).eax
    };
    let t = g.read::<u32>(w(0x02));
    let a = run(g, 0x15, w(0x10), t);
    let b = run(g, 0x23, w(0x1c), 0);
    let c = run(g, 0x31, w(0x2a), 0);
    if a != 1 || b != a || c != a || g.read::<u32>(s.wrapping_add(w(0x44))) != a {
        return Ret::eax(c);
    }
    let snd = g.read::<u32>(w(0x51));
    let t = slot_target(g, w(0x4b), 0x358 / 4);
    // call width: 1
    Ret::eax(g.call(t, 0, &[snd]).eax)
}

/// [`lists_done`] in `magic_0304` (0x10001560; the `rel32`s: the lists'
/// run three times).
pub const LISTS_DONE_0304: Sig = Sig {
    len: 100,
    hash: 0xb612_480d_33ec_af79,
    calls: &[
        (0x15, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x23, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x31, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
    ],
    imports: &[],
};

/// [`lists_done`] in `magic_0568` (0x10001500, its scene word at
/// `+0x16c8`).
pub const LISTS_DONE_0568: Sig = Sig {
    len: 100,
    hash: 0xd369_f253_2ddd_70ca,
    calls: &[
        (0x15, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x23, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x31, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
    ],
    imports: &[],
};

/// The scripts' step (a pool task, cdecl `(node)`; family P049,
/// `magic_0596` 0x10006900 and `magic_0304` 0x10001490, 203 bytes): with
/// `R = *G_R` (`+0x07`), the script block `B` (`+0x10`, 0x30 bytes) is
/// cleared (`memset`), its halfword `+0x21` gets bit 7 and its word
/// `+0x33` = `*G_S` (`+0x26`); both scripts run
/// ([`script_run`](crate::sb_misc::script_run), `ecx` `+0x2e`, `+0x3d`).
/// While the first runs (0): `R`'s four words at `+0x4e` .. (the copy's
/// displacements `+0x4e`, `+0x5a`, `+0x66`, `+0x72`) go to `+0x54`, `+0x60`,
/// `+0x6c`, `+0x7d`; the two motion scripts run
/// ([`motion_script`](crate::s4_misc::motion_script), `ecx` `+0x77`,
/// `+0x87`) and the camera rigs' step follows (a tail jump,
/// [`camera_rigs_step`](crate::s8_misc::camera_rigs_step)). Once it is
/// done: `R[+0xa0]` = 1, a held node `*G_N` (`+0x9a`) goes back to the
/// pool ([`give_back`](crate::sa_pool::give_back), then `G_N` = 0), the
/// parts are registered ([`register_parts`](crate::sa_misc::register_parts),
/// `eax` the last call's) and the task's node goes back (a tail jump).
pub fn scripts_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let r = g.read::<u32>(w(0x07));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let flags = g.read::<u16>(w(0x21)) | 0x80;
    g.write::<u16>(w(0x21), flags);
    let s = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), s);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    let second = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first == 0 {
        for (from, to) in [(0x4eu32, 0x54u32), (0x5a, 0x60), (0x66, 0x6c), (0x72, 0x7d)] {
            let v = g.read::<u32>(r.wrapping_add(w(from)));
            g.write::<u32>(r.wrapping_add(w(to)), v);
        }
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x82), w(0x77));
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x8c), w(0x87));
        // pop edi; pop esi; pop ebp; jmp: the caller's stack as it came
        let callee = Frame {
            at: img.rel32_target(f + 0x94),
            ..*fr
        };
        return crate::s8_misc::camera_rigs_step(g, img, &callee);
    }
    let held = g.read::<u32>(w(0x9a));
    g.write::<u32>(r.wrapping_add(w(0xa0)), 1);
    let mut eax = second;
    if held != 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0xad), held);
        g.write::<u32>(w(0xb3), 0);
    }
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0xbc), eax);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0xc7),
        node,
    ))
}

/// [`scripts_step`] in `magic_0596` (0x10006900; the `rel32`s: `memset`,
/// the scripts, the motion scripts, the rigs' step, the give-backs and
/// the parts' registration).
pub const SCRIPTS_STEP_0596: Sig = Sig {
    len: 203,
    hash: 0xa5f7_2545_93c2_e5a2,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x82, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x8c, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x94, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0xad, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xbc, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0xc7, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`scripts_step`] in `magic_0304` (0x10001490; its record's words
/// 0x1448 on).
pub const SCRIPTS_STEP_0304: Sig = Sig {
    len: 203,
    hash: 0x307e_8f40_e2df_99e7,
    ..SCRIPTS_STEP_0596
};

/// Two object lists run and, when both are done, the task moved on
/// (`magic_0244` 0x10001560, 71 bytes, a pool task, cdecl `(task)`): the
/// lists `L1`, `L2` (operands `+0x0b`, `+0x1b`) run
/// ([`host_objects_run`](crate::s9_misc::host_objects_run), `edx` =
/// `*G_A`, `*G_B` (`+0x05`, `+0x16`)); when both answer 1 the task's
/// state `+0x14` = 3 and its node's callback (`task[8] + 0x10`) = the
/// immediate `+0x41`. Returns the node then, else the second run's `eax`.
pub fn lists_two_done_0244<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; push esi: each run sees `esp` below them
    let esp = fr.esp.wrapping_sub(0xc);
    let run = |g: &mut G, call: u32, list: u32, edx: u32| {
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: list,
            edx,
            ..*fr
        };
        crate::s9_misc::host_objects_run(g, img, &callee).eax
    };
    let a = g.read::<u32>(w(0x05));
    let first = run(g, 0x10, w(0x0b), a);
    let b = g.read::<u32>(w(0x16));
    let second = run(g, 0x22, w(0x1b), b);
    if first != 1 || second != 1 {
        return Ret::eax(second);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let node = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(node.wrapping_add(0x10), w(0x41));
    Ret::eax(node)
}

/// [`lists_two_done_0244`] (`magic_0244` 0x10001560; the `rel32`s: the
/// lists' run twice).
pub const LISTS_TWO_DONE_0244: Sig = Sig {
    len: 71,
    hash: 0x26e8_3764_c6ed_231d,
    calls: &[
        (0x10, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x22, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
    ],
    imports: &[],
};

/// The scripts' step with the motion step (a pool task, cdecl `(node)`;
/// family P080, `magic_0165` 0x100016b0 and `magic_0606` 0x10006930, 163
/// bytes): as [`scripts_step`] (the block `+0x10` cleared, bit 7 of
/// `+0x21`, `+0x33` = `*G_S` `+0x26`, both scripts run, `ecx` `+0x2e`,
/// `+0x3d`); while the first runs: `R = *G_R` (`+0x07`)'s four words at
/// the displacements `+0x4e`, `+0x5a`, `+0x66`, `+0x72` go to `+0x54`,
/// `+0x60`, `+0x6c`, `+0x78`, the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step)) and the rigs' step
/// follows (a tail jump); once done, `R[+0x8b]` = 1, the actors are
/// restored ([`actors_restored`](crate::s4_small::actors_restored)) and
/// the task's node goes back ([`give_back`](crate::sa_pool::give_back), a
/// tail jump).
pub fn scripts_step_motion<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let r = g.read::<u32>(w(0x07));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let flags = g.read::<u16>(w(0x21)) | 0x80;
    g.write::<u16>(w(0x21), flags);
    let s = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), s);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first == 0 {
        for (from, to) in [(0x4eu32, 0x54u32), (0x5a, 0x60), (0x66, 0x6c), (0x72, 0x78)] {
            let v = g.read::<u32>(r.wrapping_add(w(from)));
            g.write::<u32>(r.wrapping_add(w(to)), v);
        }
        crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7d));
        // pop edi; pop esi; pop ebp; jmp: the caller's stack as it came
        let callee = Frame {
            at: img.rel32_target(f + 0x85),
            ..*fr
        };
        return crate::s8_misc::camera_rigs_step(g, img, &callee);
    }
    g.write::<u32>(r.wrapping_add(w(0x8b)), 1);
    // push ebp; push esi; push edi; call
    let restore = Frame {
        at: img.rel32_target(f + 0x94),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    crate::s4_small::actors_restored(g, img, &restore);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x9f),
        node,
    ))
}

/// [`scripts_step_motion`] in `magic_0165` (0x100016b0; the `rel32`s:
/// `memset`, the scripts, the motion step, the rigs' step, the actors'
/// restore and the give-back).
pub const SCRIPTS_STEP_MOTION_0165: Sig = Sig {
    len: 163,
    hash: 0x7bbf_b577_54e1_c57d,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x85, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x94, Callee::Sig(&crate::s4_small::ACTORS_RESTORED)),
        (0x9f, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`scripts_step_motion`] in `magic_0606` (0x10006930).
pub const SCRIPTS_STEP_MOTION_0606: Sig = Sig {
    hash: 0x8d38_7f1f_777d_f4fa,
    ..SCRIPTS_STEP_MOTION_0165
};

/// The script's step (a pool task, cdecl `(node)`; family P082,
/// `magic_0307` 0x10001510 and `magic_0605` 0x10006e60, 142 bytes): with
/// `S = *G_S` (`+0x06`), the block `+0x0f` cleared (`memset`, 0x30
/// bytes), bit 7 of `+0x20` set, `+0x2a` = `*(+0x25)`, `+0x37` = `*(+0x2f)`;
/// the DLL's script runs ([`run`](crate::sa_script::run)). Still running
/// (0): the motion script ([`motion_script`](crate::s4_misc::motion_script),
/// `ecx` `+0x45`) and the rigs' step (a tail jump). Done: unless `S[+0x57]`
/// is 1 it returns the run's `eax`; else `S[+0x66]` = 1, a held node
/// `*G_N` (`+0x60`) goes back to the pool (then `G_N` = 0) and the task's
/// node goes back (a tail jump).
pub fn script_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x06));
    g.write_bytes(w(0x0f), &[0u8; 0x30]);
    let flags = g.read::<u16>(w(0x20)) | 0x80;
    g.write::<u16>(w(0x20), flags);
    let v = g.read::<u32>(w(0x25));
    g.write::<u32>(w(0x2a), v);
    let v = g.read::<u32>(w(0x2f));
    g.write::<u32>(w(0x37), v);
    let r = crate::sa_script::run(g, img, img.rel32_target(f + 0x3c));
    if r == 0 {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x4a), w(0x45));
        // pop esi; pop ebp; jmp: the caller's stack as it came
        let callee = Frame {
            at: img.rel32_target(f + 0x51),
            ..*fr
        };
        return crate::s8_misc::camera_rigs_step(g, img, &callee);
    }
    if g.read::<u32>(s.wrapping_add(w(0x57))) != 1 {
        return Ret::eax(r);
    }
    let held = g.read::<u32>(w(0x60));
    g.write::<u32>(s.wrapping_add(w(0x66)), 1);
    if held != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x73), held);
        g.write::<u32>(w(0x79), 0);
    }
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x87),
        node,
    ))
}

/// [`script_step`] in `magic_0307` (0x10001510; the `rel32`s: `memset`,
/// the script, the motion script, the rigs' step and the give-backs).
pub const SCRIPT_STEP_0307: Sig = Sig {
    len: 142,
    hash: 0x74fd_c0f0_fb12_15b5,
    calls: &[
        (0x14, Callee::Import("memset")),
        (0x3c, Callee::Sig(&crate::sa_script::RUN)),
        (0x4a, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x51, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x73, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x87, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`script_step`] in `magic_0605` (0x10006e60).
pub const SCRIPT_STEP_0605: Sig = Sig {
    hash: 0x75b9_b87d_0597_4ae5,
    ..SCRIPT_STEP_0307
};
