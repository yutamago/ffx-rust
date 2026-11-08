//! Round 23's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::ob_model::{
    Angle, ModelOps, RootOps, Who, root_matrix, scale_model, st0_f32, wrap_once,
};
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0598`'s placement of the caster and the summon (0x10006cf0,
/// 1919 bytes; no arguments, `esp` the entry stack pointer; a cookie
/// frame, `ebp = E - 4`): [`place_pair`](crate::m0596::place_pair)'s
/// steps with the state `S = *G_STATE` 0x520 bytes further on, no stale
/// word (`+0x91c`/`+0x93c` 0, `+0x92c`/`+0x94c` the local `w` 1.0 at `ebp -
/// 8`), the summon's frame without the height, `pos` at `ebp - 0x34`.
/// The caster (slot 262 into `ebp - 0x14..-0xc`, kept in `S + 0x970`)
/// faces `ang = A - Ch_GetRot` (slot 333, wrapped once); the frames `S +
/// 0x910` and `+0x930` get the angle and the position; the caster's
/// model and the summon's (`*G_SUMMON`, battle index `*G_INDEX`) are
/// placed ([`root_matrix`], [`scale_model`]) into `S + 0x850` and `+0x890`
/// (copied to `+0x880` and `+0x8c0`); slot 238 `(index, 1)` shows the
/// summon, slot 296 `(index, 0)` hides the caster (`*G_OFF` 0, index
/// below 32) or slot 257 `(summon, 1)`; slot 8 `(summon, 0)`; `S + 0xd18`
/// = `ang`. Two camera points, `S + 0x980` and `+0x990`: the position
/// moved by `(0, 0, 15)` and `(0, 0, 24)` turned by X 0 and Y `ang`
/// (slots 235, 328, 329 into the matrices at `ebp - 0xb4` and `ebp -
/// 0x74`, slot 229 into `ebp - 0x34`, slot 248 adds it), `w` 1.0; `S +
/// 0xd3c` 0 and three flags set. Returns the second slot 248's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn place_pair_0598<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let chr = crate::ob_model::who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x14,
            at: 0,
            table: 0x33,
        },
    );
    let s = g.read::<u32>(w(0x20));
    let (px, py, pz) = (l(0x14), l(0x10), l(0xc));
    let mut stack = vec![chr, px, py, pz];
    g.call(t(g, 0x38, 262), 0, &stack);
    for (off, src) in [(0x970u32, px), (0x974, py), (0x978, pz)] {
        let v = g.read::<u32>(src);
        g.write::<u32>(s + off, v);
    }
    g.write::<u32>(s + 0x97c, ONE);
    g.write::<u32>(l(8), ONE);
    let chr2 = crate::ob_model::who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x76,
            at: 0,
            table: 0x87,
        },
    );
    stack.insert(0, chr2);
    // call width: 5 (the slot 262 words stay on the stack)
    let rot = st0_f32(g.call(t(g, 0x8c, 333), 0, &stack).st0);
    let x = sub(g.read::<f32>(w(0xa2)), rot);
    let ang = wrap_once(g, img, f, x, [0xb4, 0xc6, 0xd0, 0xdd]);
    // the two frames: 0, the angle, 0, 0, then the position and `w`
    let frame = |g: &mut G, at: u32| {
        g.write::<u32>(at, 0);
        g.write::<f32>(at + 4, ang);
        g.write::<u32>(at + 8, 0);
        g.write::<u32>(at + 0xc, 0);
        for (i, src) in (0u32..).zip([px, py, pz, l(8)]) {
            let v = g.read::<u32>(src);
            g.write::<u32>(at + 0x10 + 4 * i, v);
        }
    };
    let copy = |g: &mut G, dst: u32, src: u32| {
        for i in 0..4 {
            let v = g.read::<u32>(src + 4 * i);
            g.write::<u32>(dst + 4 * i, v);
        }
    };
    let turns = [Angle::Bits(0), Angle::Bits(0), Angle::Bits(ang.to_bits())];
    frame(g, s + 0x910);
    let selp = g.read::<u32>(w(0x145));
    let k = u32::from(g.read::<u8>(selp));
    let chr = g.read::<u32>(w(0x155).wrapping_add(4 * k));
    let pos = l(0x34);
    let m = s.wrapping_add(0x850);
    let root_ops = RootOps {
        s210: 0x166,
        off: 0x179,
        s217: 0x18d,
        s255: 0x19f,
        s211: 0x1b8,
    };
    root_matrix(g, img, f, &root_ops, (chr, m, pos), |g| {
        u32::from(g.read::<u8>(selp))
    });
    let model = ModelOps {
        s235: 0x1c5,
        c277: Who::Sel {
            sel: 0x1d3,
            at: 0,
            table: 0x1dd,
        },
        s277: 0x1e2,
        k: 0x200,
        c: Who::Sel {
            sel: 0x213,
            at: 0,
            table: 0x21d,
        },
        s15: 0x222,
        s16: 0x23f,
        s17: 0x261,
        s330: 0x284,
        s328: 0x2a2,
        s329: 0x2c3,
    };
    scale_model(g, img, f, &model, m, &[chr, m], turns);
    copy(g, s + 0x880, s + 0x920);
    frame(g, s + 0x930);
    let summon = g.read::<u32>(w(0x36f));
    let m = s.wrapping_add(0x890);
    let root_ops = RootOps {
        s210: 0x386,
        off: 0x399,
        s217: 0x3b0,
        s255: 0x3c2,
        s211: 0x3db,
    };
    root_matrix(g, img, f, &root_ops, (summon, m, pos), |g| {
        g.read::<u32>(w(0x3a2))
    });
    let model = ModelOps {
        s235: 0x3e8,
        c277: Who::Word(0x3fc),
        s277: 0x3f6,
        k: 0x41a,
        c: Who::Word(0x433),
        s15: 0x41f,
        s16: 0x450,
        s17: 0x45f,
        s330: 0x495,
        s328: 0x4b3,
        s329: 0x4d4,
    };
    scale_model(g, img, f, &model, m, &[summon, m], turns);
    copy(g, s + 0x8c0, s + 0x940);
    let i = g.read::<u32>(w(0x523));
    // the second turn's three words stay on the stack
    g.call(t(g, 0x51b, 238), 0, &[i, 1, m, m, ang.to_bits()]);
    let off = g.read::<u32>(w(0x534)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x53c)) };
    if !off && i.cast_signed() < 0x20 {
        g.call(t(g, 0x549, 296), 0, &[i, 0]);
    } else {
        let summon = g.read::<u32>(w(0x55e));
        g.call(t(g, 0x556, 257), 0, &[summon, 1]);
    }
    let summon = g.read::<u32>(w(0x579));
    g.call(t(g, 0x56b, 8), 0, &[summon, 0]);
    g.write::<f32>(s + 0xd18, ang);
    // the camera points: (cam, matrix, z, slot operands 235/328/329/229/248)
    let v = l(0x24);
    let mut eax = 0;
    for (cam, mat, z, ops) in [
        (
            0x980u32,
            l(0xb4),
            0x4170_0000u32,
            [0x5cdu32, 0x601, 0x626, 0x642, 0x65a],
        ),
        (
            0x990,
            l(0x74),
            0x41c0_0000,
            [0x6aa, 0x6db, 0x6fd, 0x716, 0x72e],
        ),
    ] {
        let cam = s + cam;
        copy(g, cam, s + 0x920);
        g.write::<u32>(v, 0);
        g.write::<u32>(v + 4, 0);
        g.write::<u32>(v + 8, z);
        g.call(t(g, ops[0], 235), 0, &[mat]);
        g.call(t(g, ops[1], 328), 0, &[mat, mat, 0]);
        g.call(t(g, ops[2], 329), 0, &[mat, mat, ang.to_bits()]);
        g.write::<u32>(v + 0xc, ONE);
        g.call(t(g, ops[3], 229), 0, &[pos, mat, v]);
        eax = g.call(t(g, ops[4], 248), 0, &[cam, cam, pos]).eax;
        g.write::<u32>(cam + 0xc, ONE);
    }
    g.write::<u32>(s + 0xd3c, 0);
    for o in [0x759u32, 0x763, 0x76d] {
        g.write::<u32>(w(o), 1);
    }
    eax
}

/// [`place_pair_0598`] (1,919 bytes, 1 overlay).
pub const PLACE_PAIR_0598: Sig = Sig {
    len: 1919,
    hash: 0xdae9_8feb_fed1_d85e,
    // __security_check_cookie
    calls: &[(0x777, Callee::Any)],
    imports: &[],
};

/// `magic_0598`'s second start (0x100085c0, 326 bytes, no arguments):
/// [`start2`](crate::m0596::start2) instruction for instruction, with
/// the family's part set-up ([`part_setup`](crate::sb_misc::part_setup))
/// and its own placement ([`place_pair_0598`]).
pub fn start2_0598<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0596::start2_with(
        g,
        img,
        fr,
        |g, img, at, esp| {
            crate::sb_misc::part_setup(g, img, at, esp);
        },
        |g, img, at, esp| {
            place_pair_0598(g, img, at, esp);
        },
    )
}

/// [`start2_0598`] (326 bytes, 1 overlay; `m0596::START2`'s bytes).
pub const START2_0598: Sig = Sig {
    len: 326,
    hash: 0xb0df_13a1_ef72_7e74,
    calls: &[
        (0x4c, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x51, Callee::Sig(&PLACE_PAIR_0598)),
        (0x56, Callee::Sig(&crate::o4_effect::START_TURNED_0598_7470)),
        (0xba, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf8, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x102, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0598`'s load step (0x10006a60, 176 bytes; a table entry):
/// [`cue_start`](crate::m0591::cue_start)'s copy that runs
/// [`start2_0598`] where the others set up their targets.
pub fn cue_start_0598<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0591::cue_start_with(g, img, fr, |g, img, fr| {
        start2_0598(g, img, fr);
    })
}

/// [`cue_start_0598`] (176 bytes, 1 overlay).
pub const CUE_START_0598: Sig = Sig {
    len: 176,
    hash: 0x0c88_f772_d650_fe60,
    calls: &[(0x95, Callee::Sig(&START2_0598))],
    imports: &[],
};

/// A character moved to its own position (the original's `G_OFF` check):
/// unless `*G_OFF` (operand `off`) is set, the battle index `i` below 32
/// gets slot 217 `(i, pos)` and slot 255 `(i, pos)`; then slot 211 `(chr,
/// 0)`.
fn reset_one<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (off, s217, s255, s211): (u32, u32, u32, u32),
    chr: u32,
    index: impl FnOnce(&G) -> u32,
    pos: u32,
) {
    let w = |o: u32| img.word(f + o);
    if g.read::<u32>(w(off)) == 0 {
        let i = index(g);
        if i.cast_signed() < 0x20 {
            g.call(slot_target(g, w(s217), 217), 0, &[i, pos]);
            g.call(slot_target(g, w(s255), 255), 0, &[i, pos]);
        }
    }
    g.call(slot_target(g, w(s211), 211), 0, &[chr, 0]);
}

/// `magic_0598`'s effect end (0x10007500, 666 bytes; internal, no
/// arguments, called by [`host_start_end_0598`]; a cookie frame,
/// `ebp = E - 4`, the positions at `ebp - 0x14` and `ebp - 0x24`). Slot
/// 384 of the second selected character; slots 25 `(0x3013, 1)` and 24
/// `(0x3013, 0)`; the caster (the first selected) and the summon
/// `*G_SUMMON` back at their own places ([`reset_one`]); slot 213 `(*G_A,
/// 0)` while the u16 `*G_WA` is 0 and `(*G_B, 0)` while `*G_WB` is 0, the
/// flags `G_F1`, `G_F2` 1; the caster shown (slot 296 `(i, 1)` for an
/// index below 32, the battle on, else slot 257 `(chr, 0)`), the summon
/// likewise (slot 296 `(*G_INDEX, 1)` or 257 `(summon, 0)`); slots 399
/// `(summon, 0)`, 8 `(summon, 1)`; the party's list run
/// ([`each_actor`](crate::sa_script::each_actor), `ecx` 1); slots 371
/// `(0, 0)`, `(1, 0)`, 543 `(0)`; slot 219 `(caster index, 1)`, a
/// character it answers hidden (slot 257 `(c, 0)`); the node `*G_N` back
/// to the pool ([`give_back`](crate::sa_pool::give_back)) and cleared;
/// the parts registered
/// ([`register_parts`](crate::sa_misc::register_parts) with the `eax`
/// left: the give-back's, else slot 257's or 0). Returns what that
/// leaves.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn effect_end_0598<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let second = crate::ob_model::who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x11,
            at: 1,
            table: 0x1e,
        },
    );
    g.call(t(g, 0x23, 384), 0, &[second]);
    g.call(t(g, 0x30, 25), 0, &[0x3013, 1, second]);
    g.call(t(g, 0x41, 24), 0, &[0x3013, 0]);
    let selp = g.read::<u32>(w(0x53));
    let k = u32::from(g.read::<u8>(selp));
    let caster = g.read::<u32>(w(0x67).wrapping_add(4 * k));
    reset_one(
        g,
        img,
        f,
        (0x5c, 0x7a, 0x8c, 0x9d),
        caster,
        |g| u32::from(g.read::<u8>(selp)),
        ebp.wrapping_sub(0x14),
    );
    let summon = g.read::<u32>(w(0xae));
    reset_one(
        g,
        img,
        f,
        (0xb7, 0xce, 0xe0, 0xf1),
        summon,
        |g| g.read::<u32>(w(0xc0)),
        ebp.wrapping_sub(0x24),
    );
    if g.read::<u16>(w(0x106)) == 0 {
        let a = g.read::<u32>(w(0x118));
        g.call(t(g, 0x110, 213), 0, &[a, 0]);
    }
    let wait_b = g.read::<u16>(w(0x12a));
    g.write::<u32>(w(0x131), 1);
    if wait_b == 0 {
        let b = g.read::<u32>(w(0x144));
        g.call(t(g, 0x13c, 213), 0, &[b, 0]);
    }
    let off = g.read::<u32>(w(0x155));
    let selp = g.read::<u32>(w(0x15b));
    g.write::<u32>(w(0x161), 1);
    let c = u32::from(g.read::<u8>(selp));
    if off == 0 && c < 0x20 {
        g.call(t(g, 0x174, 296), 0, &[c, 1]);
    } else {
        let chr = g.read::<u32>(w(0x18b).wrapping_add(4 * c));
        g.call(t(g, 0x190, 257), 0, &[chr, 0]);
    }
    let off = g.read::<u32>(w(0x1a1)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x1a9)) };
    if !off && i.cast_signed() < 0x20 {
        g.call(t(g, 0x1b6, 296), 0, &[i, 1]);
    } else {
        let summon = g.read::<u32>(w(0x1cb));
        g.call(t(g, 0x1c3, 257), 0, &[summon, 0]);
    }
    let summon = g.read::<u32>(w(0x1e9));
    g.call(t(g, 0x1d8, 399), 0, &[summon, 0]);
    let summon = g.read::<u32>(w(0x1f8));
    g.call(t(g, 0x1f0, 8), 0, &[summon, 1, summon, 0]);
    // the slot 399 and slot 8 words under the call: the return address at
    // `E - 0x3c`
    crate::sa_script::each_actor(
        g,
        img,
        img.rel32_target(f + 0x207),
        1,
        esp.wrapping_sub(0x3c),
    );
    g.call(t(g, 0x20c, 371), 0, &[0, 0]);
    g.call(t(g, 0x21d, 371), 0, &[1, 0]);
    g.call(t(g, 0x22e, 543), 0, &[0]);
    let selp = g.read::<u32>(w(0x23d));
    let c = u32::from(g.read::<u8>(selp));
    let mut eax = g.call(t(g, 0x248, 219), 0, &[c, 1]).eax;
    if eax != 0 {
        eax = g.call(t(g, 0x25f, 257), 0, &[eax, 0]).eax;
    }
    let node = g.read::<u32>(w(0x270));
    if node != 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x279), node);
        g.write::<u32>(w(0x27f), 0);
    }
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x288), eax)
}

/// [`effect_end_0598`] (666 bytes, 1 overlay).
pub const EFFECT_END_0598: Sig = Sig {
    len: 666,
    hash: 0x4b63_28cd_ec64_9f42,
    calls: &[
        (0x207, Callee::Sig(&crate::sa_script::EACH_ACTOR)),
        (0x279, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x288, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        // __security_check_cookie
        (0x292, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0598` 0x10006920 (309 bytes; a table entry, no arguments):
/// [`host_start_d`](crate::sa_misc::host_start_d) whose end runs the
/// effect's end ([`effect_end_0598`]) whatever slot 549 answers. Returns
/// 0.
pub fn host_start_end_0598<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    crate::sa_misc::host_start_calls(g, img, f);
    let t = slot_target(g, img.word(f + 0x106), 549);
    if g.call(t, 0, &[1]).eax != 0 {
        let t = slot_target(g, img.word(f + 0x11c), 550);
        g.call(t, 0, &[1]);
    }
    // the return address at `E - 4`
    effect_end_0598(g, img, img.rel32_target(f + 0x12e), fr.esp.wrapping_sub(4));
    Ret::eax(0)
}

/// [`host_start_end_0598`] (309 bytes, 1 overlay).
pub const HOST_START_END_0598: Sig = Sig {
    len: 309,
    hash: 0x7322_6dfb_363d_5347,
    calls: &[(0x12e, Callee::Sig(&EFFECT_END_0598))],
    imports: &[],
};

/// `magic_0598`'s turn (0x100081b0, 966 bytes; a table entry, no
/// arguments; a cookie frame, `ebp = E - 4`): with the state `S =
/// *G_STATE`, its yaw `S + 0x914` steps by `K` (wrapped once); the camera
/// point `S + 0x920` is the point `(25, 0, 15)` turned by X 0 and Y `S +
/// 0xd18` (slots 235, 328, 329 into the matrix at `ebp - 0x64`, slot 229
/// into `ebp - 0x24`, slot 248 adds it), `w` 1.0; the caster's model
/// matrix `S + 0x850` ([`scale_model`], the first selected character,
/// turned by `S + 0x918`, `+0x910`, `+0x914`); `S + 0x950..` = `S +
/// 0x910..` (eight words); the second character's model `S + 0x8d0`
/// turned by `S + 0x958`, `+0x950`, `+0x954`; then a fade task
/// ([`fade_task`](crate::s3_misc::fade_task) of `S + 0xbb0`, 0x1e frames,
/// the stack word 0x55, `xmm2`/`xmm1` the DLL's constants). Returns the
/// task's node.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn turn_0598<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let s = g.read::<u32>(w(0x14));
    let yaw = add(g.read::<f32>(s + 0x914), k(g, 0x25));
    let yaw = wrap_once(g, img, f, yaw, [0x2f, 0x39, 0x43, 0x50]);
    g.write::<f32>(s + 0x914, yaw);
    let a = g.read::<u32>(s + 0xd18);
    let (m, v, out) = (l(0x64), l(0x14), l(0x24));
    for (i, x) in (0u32..).zip([0x41c8_0000u32, 0, 0x4170_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let cam = s + 0x920;
    g.call(t(g, 0x59, 235), 0, &[m]);
    g.call(t(g, 0xa9, 328), 0, &[m, m, 0]);
    g.call(t(g, 0xc8, 329), 0, &[m, m, a]);
    g.write::<u32>(v + 0xc, ONE);
    g.call(t(g, 0xe1, 229), 0, &[out, m, v]);
    g.call(t(g, 0xf9, 248), 0, &[cam, cam, out]);
    g.write::<u32>(cam + 0xc, ONE);
    let caster = ModelOps {
        s235: 0x108,
        c277: Who::Sel {
            sel: 0x123,
            at: 0,
            table: 0x12d,
        },
        s277: 0x132,
        k: 0x14a,
        c: Who::Sel {
            sel: 0x15d,
            at: 0,
            table: 0x167,
        },
        s15: 0x16c,
        s16: 0x183,
        s17: 0x19f,
        s330: 0x1bf,
        s328: 0x1eb,
        s329: 0x20a,
    };
    let turns = |o: u32| [Angle::At(s + o + 8), Angle::At(s + o), Angle::At(s + o + 4)];
    scale_model(g, img, f, &caster, s + 0x850, &[], turns(0x910));
    for i in 0..8 {
        let x = g.read::<u32>(s + 0x910 + 4 * i);
        g.write::<u32>(s + 0x950 + 4 * i, x);
    }
    let second = ModelOps {
        s235: 0x27d,
        c277: Who::Sel {
            sel: 0x291,
            at: 1,
            table: 0x29c,
        },
        s277: 0x2a1,
        k: 0x2b9,
        c: Who::Sel {
            sel: 0x2cc,
            at: 1,
            table: 0x2d7,
        },
        s15: 0x2dc,
        s16: 0x2f3,
        s17: 0x30f,
        s330: 0x32f,
        s328: 0x35b,
        s329: 0x37a,
    };
    let m2 = s + 0x8d0;
    scale_model(g, img, f, &second, m2, &[], turns(0x950));
    // push 0x55 over slot 329's three words: the return address at E - 0x90
    let esp = fr.esp.wrapping_sub(0x90);
    let a = g.read::<u32>(s + 0x954);
    for (i, x) in (1u32..).zip([0x55, m2, m2, a]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x394)));
    xmm[1] = u128::from(g.read::<u32>(w(0x39c)));
    let fade = Frame {
        at: img.rel32_target(f + 0x3ae),
        esp,
        ecx: s + 0xbb0,
        edx: 0x1e,
        xmm,
        ..*fr
    };
    Ret::eax(crate::s3_misc::fade_task(g, img, &fade))
}

/// [`turn_0598`] (966 bytes, 1 overlay).
pub const TURN_0598: Sig = Sig {
    len: 966,
    hash: 0x7f24_4198_e5f8_bbbc,
    calls: &[
        (0x3ae, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        // __security_check_cookie
        (0x3be, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0598`'s objects started (0x100078c0, 1389 bytes; a table entry,
/// no arguments; a cookie frame, `ebp = E - 4`): with the state `S =
/// *G_STATE`, three effect objects `O = S + 0x4c0 + 0x130 j`
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// each with its scale constant in `xmm3`), each at the camera point `S +
/// 0x980` (`+0xd0..`) with its `y` less `K`; their yaw `+0xc4` = `S +
/// 0xd18` (the second's plus `K'` and the third's less it, wrapped once;
/// those two also get `+0xc8` = +-pi/4 and `+0xe0` = +-15). `S + 0x940..`
/// = `S + 0x990..`; the second character's model `S + 0x8d0`
/// ([`root_matrix`], [`scale_model`] turned by `S + 0x958`, `+0x950`,
/// `+0x954` after `S + 0x950..` = `S + 0x930..`, eight words), `S +
/// 0x900..` = `S + 0x960..`; slot 399 `(chr, 1)`; the record `R = S +
/// 0x9a0` (colour bytes 0x80, sizes 0x18, the u16 `+0x98` 0x48) on a task
/// node ([`take`](crate::sa_pool::take), callback `CB`) appended to the
/// task list; the sprite fan `S + 0xbe8` (colour 0x30808080, u16 `+0x124`
/// 4, `+0xfc` 0.05) started
/// ([`sprite_fan_start`](crate::s6_misc::sprite_fan_start)); slots 371
/// `(0, 0)` and `(1, 0)`. Returns the last call's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of steps
pub fn objects_start_0598<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let s = g.read::<u32>(w(0x1d));
    let copy = |g: &mut G, dst: u32, src: u32, n: u32| {
        for i in 0..n {
            let v = g.read::<u32>(src + 4 * i);
            g.write::<u32>(dst + 4 * i, v);
        }
    };
    let start = |g: &mut G, call: u32, obj: u32, scale: u32| {
        let mut xmm = fr.xmm;
        xmm[3] = u128::from(g.read::<u32>(w(scale)));
        let of = Frame {
            at: img.rel32_target(f + call),
            esp: fr.esp.wrapping_sub(0x3c),
            edx: obj,
            xmm,
            ..*fr
        };
        crate::o6_fam_object::start_object_first(g, img, of.at, &of);
    };
    let o1 = s + 0x4c0;
    start(g, 0x2d, o1, 0x14);
    copy(g, o1 + 0xd0, s + 0x980, 4);
    let y = sub(g.read::<f32>(o1 + 0xd4), k(g, 0x75));
    let a = g.read::<u32>(s + 0xd18);
    g.write::<u32>(o1 + 0xc4, a);
    g.write::<f32>(o1 + 0xd4, y);
    let o2 = o1 + 0x130;
    start(g, 0x93, o2, 0x3b);
    copy(g, o2 + 0xd0, s + 0x980, 4);
    let x = add(g.read::<f32>(s + 0xd18), k(g, 0xdb));
    let x = wrap_once(g, img, f, x, [0xe5, 0xa1, 0xf5, 0xa1]);
    g.write::<f32>(o2 + 0xc4, x);
    let y = sub(g.read::<f32>(o2 + 0xd4), k(g, 0x11e));
    g.write::<u32>(o2 + 0xc8, 0x3f49_0fdb);
    g.write::<u32>(o2 + 0xe0, 0x4170_0000);
    g.write::<f32>(o2 + 0xd4, y);
    let o3 = o2 + 0x130;
    start(g, 0x147, o3, 0x106);
    copy(g, o3 + 0xd0, s + 0x980, 4);
    let x = sub(g.read::<f32>(s + 0xd18), k(g, 0x187));
    let x = wrap_once(g, img, f, x, [0x191, 0x19b, 0x1a5, 0x1b2]);
    g.write::<f32>(o3 + 0xc4, x);
    let y = sub(g.read::<f32>(o3 + 0xd4), k(g, 0x1ca));
    g.write::<u32>(o3 + 0xc8, 0xbf49_0fdb);
    g.write::<u32>(o3 + 0xe0, 0xc170_0000);
    g.write::<f32>(o3 + 0xd4, y);
    copy(g, s + 0x940, s + 0x990, 4);
    let m = s.wrapping_add(0x8d0);
    let selp = g.read::<u32>(w(0x221));
    let i = u32::from(g.read::<u8>(selp.wrapping_add(1)));
    let chr = g.read::<u32>(w(0x22c).wrapping_add(4 * i));
    let root_ops = RootOps {
        s210: 0x23a,
        off: 0x24d,
        s217: 0x25e,
        s255: 0x270,
        s211: 0x286,
    };
    root_matrix(
        g,
        img,
        f,
        &root_ops,
        (chr, m, ebp.wrapping_sub(0x14)),
        |_| i,
    );
    copy(g, s + 0x950, s + 0x930, 8);
    let model = ModelOps {
        s235: 0x2f3,
        c277: Who::Sel {
            sel: 0x301,
            at: 1,
            table: 0x30c,
        },
        s277: 0x311,
        k: 0x329,
        c: Who::Sel {
            sel: 0x33c,
            at: 1,
            table: 0x347,
        },
        s15: 0x34c,
        s16: 0x363,
        s17: 0x372,
        s330: 0x39c,
        s328: 0x3c8,
        s329: 0x3ea,
    };
    let turns = [
        Angle::At(s + 0x958),
        Angle::At(s + 0x950),
        Angle::At(s + 0x954),
    ];
    scale_model(g, img, f, &model, m, &[chr, m], turns);
    copy(g, s + 0x900, s + 0x960, 4);
    let c = crate::ob_model::who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x42e,
            at: 1,
            table: 0x43b,
        },
    );
    g.call(t(g, 0x440, 399), 0, &[c, 1]);
    let r = s + 0x9a0;
    g.write::<u32>(r + 0x81, 0x8080_8080);
    g.write::<u8>(r + 0x80, 0x80);
    g.write::<u16>(r + 0x85, 0x8080);
    g.write::<u8>(r + 0x87, 0);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x18);
    g.write::<u32>(r + 0x8c, 0x18);
    g.write::<u32>(r + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x4ad), w(0x456));
    let last = g.read::<u32>(w(0x4b3));
    g.write::<u32>(node + 0x14, r);
    g.write::<u32>(node + 4, 0);
    if last == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x4d8), node);
    } else {
        let tail = g.read::<u32>(w(0x4c6));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x4e6), node);
    let fan = s + 0xbe8;
    g.write::<u32>(fan + 0xf0, 0x3080_8080);
    g.write::<u16>(fan + 0x124, 4);
    g.write::<u32>(fan + 0x120, 0);
    g.write::<u32>(fan + 0x110, 0);
    g.write::<u32>(fan + 0xf8, 0);
    g.write::<u32>(fan + 0xfc, 0x3d4c_cccd);
    g.write::<u32>(fan + 0x100, 0);
    let ff = Frame {
        at: img.rel32_target(f + 0x533),
        esp: fr.esp.wrapping_sub(0x2c),
        ecx: fan,
        ..*fr
    };
    crate::s6_misc::sprite_fan_start(g, img, &ff);
    g.call(t(g, 0x538, 371), 0, &[0, 0]);
    Ret::eax(g.call(t(g, 0x549, 371), 0, &[1, 0]).eax)
}

/// [`objects_start_0598`] (1,389 bytes, 1 overlay).
pub const OBJECTS_START_0598: Sig = Sig {
    len: 1389,
    hash: 0x43a4_77e7_9f3d_f303,
    calls: &[
        (
            0x2d,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
        ),
        (
            0x93,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
        ),
        (
            0x147,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
        ),
        (0x4ad, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x533, Callee::Sig(&crate::s6_misc::SPRITE_FAN_START)),
        // __security_check_cookie
        (0x565, Callee::Any),
    ],
    imports: &[],
};
