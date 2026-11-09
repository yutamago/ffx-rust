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

/// `magic_0514`'s draw visit (0x10003220, 763 bytes; internal, `ecx` = the
/// cursor, reached by the tail jump of [`thunk_visit_draw_0514`]; a
/// 16-aligned frame: `A = ((E - 4) & !0xf) - 0x148`, the child cursor at
/// `A`, the record `A + 8` (0x5c bytes), the colour `A + 0x80`, slot 325's
/// vector `A + 0xc0`, slot 231's matrix `A + 0x100`). The walk
/// ([`walk`](crate::nodes::walk), each method with the owner in `ecx`);
/// after it, the owner `O`'s u16 count `+0xaa` drops, and once it was not
/// above 0: with `O + 0x7f` set, its two children `+0x6c`, `+0x70` are
/// visited through their `+0x1c` method (the cursor at `A` with the pass
/// byte), slots 355 and 354 `(h, -1)` of `O[0x68] + 0x54`, slot 325 `(A +
/// 0xc0, O[0x70] + 0x1c)` and slot 231 `(A + 0x100, M, O[0x6c] + 0x9c)`,
/// and slot 567 draws the record `(O[0x78], A + 8, O[0x6c] + 0x9c, 0, 0)`
/// (its answer into `G_H`); a set `0x10` flag at `O + 0xa4` then makes
/// the owner leave its parent ([`leave`](crate::sa_misc::leave)) and runs
/// [`three_slots_m`](crate::o4_tiny4::three_slots_m) on it. Returns the
/// last `eax`: the walk's, the count less one, slot 567's or the slots'.
#[allow(clippy::too_many_lines)] // one straight sequence of steps
pub fn visit_draw_0514<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, cursor: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let a = (esp.wrapping_sub(4) & !0xf).wrapping_sub(0x148);
    // the cookie (`__security_cookie ^ esp`) is in `eax` when the walk
    // starts: the stop's answer keeps its upper bytes
    let cookie = g.read::<u32>(w(0xd)) ^ a;
    let owner = g.read::<u32>(cursor);
    let (eax, tail) = crate::nodes::walk(g, cursor, owner, cookie);
    let Some(o) = tail else {
        return eax;
    };
    let n = g.read::<u16>(o + 0xaa);
    let mut eax = u32::from(n).wrapping_sub(1);
    g.write::<u16>(o + 0xaa, n.wrapping_sub(1));
    if n.cast_signed() > 0 {
        return eax;
    }
    if g.read::<u8>(o + 0x7f) != 0 {
        for off in [0x6cu32, 0x70] {
            let pass = g.read::<u8>(cursor + 5);
            let child = g.read::<u32>(o + off);
            g.write::<u32>(a, child);
            g.write::<u8>(a + 5, pass);
            let method = g.read::<u32>(g.read::<u32>(child) + 0x1c);
            g.call(method, a, &[a]);
        }
        for (op, slot) in [(0xd5u32, 355u32), (0xea, 354)] {
            let h = g.read::<u32>(g.read::<u32>(o + 0x68) + 0x54);
            g.call(t(g, op, slot), 0, &[h, 0xffff_ffff]);
        }
        let colour = a + 0x80;
        for (i, x) in (0u32..).zip([0x437f_0000u32, 0x4340_0000, 0x4300_0000, ONE]) {
            g.write::<u32>(colour + 4 * i, x);
        }
        let v = a + 0xc0;
        let p = g.read::<u32>(o + 0x70) + 0x1c;
        g.call(t(g, 0x132, 325), 0, &[v, p]);
        let rec = a + 8;
        g.write::<u16>(rec + 8, 0x44);
        let gp = g.read::<u32>(w(0x149));
        g.write::<u32>(v + 0xc, 0x3d0f_5c29);
        for i in 0..12 {
            g.write::<u32>(a + 0x90 + 4 * i, 0);
        }
        g.write::<u32>(rec, 0x5c);
        g.write::<u32>(rec + 0x1c, 0);
        g.write::<u32>(rec + 0x38, 0);
        g.write::<u32>(rec + 0x3c, 0);
        let x = g.read::<u32>(gp);
        g.write::<u32>(rec + 4, x);
        for off in [0xcu32, 0x10, 0x14] {
            g.write::<u32>(rec + off, 0);
        }
        let x = g.read::<u32>(o + 0x7c);
        g.write::<u32>(rec + 0x18, x);
        g.write::<u32>(rec + 0x40, 0);
        let m = a + 0x100;
        let src = g.read::<u32>(o + 0x6c) + 0x9c;
        g.call(t(g, 0x240, 231), 0, &[m, w(0x23a), src]);
        g.write::<u32>(rec + 0x20, m);
        g.write::<u32>(rec + 0x24, colour);
        g.write::<u32>(rec + 0x28, v);
        g.write::<u32>(rec + 0x44, 0);
        g.write::<u32>(rec + 0x2c, m);
        let b = g.read::<u8>(o + 0x80);
        g.write::<u8>(rec + 0x4c, b);
        let b = g.read::<u8>(o + 0x81);
        g.write::<u8>(rec + 0x4d, b);
        let h = g.read::<u32>(w(0x2a0));
        g.write::<u32>(rec + 0x34, h);
        let src = g.read::<u32>(o + 0x6c) + 0x9c;
        let model = g.read::<u32>(o + 0x78);
        eax = g.call(t(g, 0x2bb, 567), 0, &[model, rec, src, 0, 0]).eax;
        g.write::<u32>(w(0x2ce), eax);
    }
    if g.read::<u8>(o + 0xa4) & 0x10 != 0 {
        eax = crate::sa_misc::leave(g, o, eax);
        // the owner stays in `ecx` (the leave never writes it)
        let tf = Frame {
            at: img.rel32_target(f + 0x2e3),
            esp: esp.wrapping_sub(4),
            ecx: o,
            eax,
            ..Frame::default()
        };
        eax = crate::o4_tiny4::three_slots_m(g, img, &tf).eax;
    }
    eax
}

/// [`visit_draw_0514`] (763 bytes, 1 overlay).
pub const VISIT_DRAW_0514: Sig = Sig {
    len: 763,
    hash: 0x8e44_6153_ef3a_24db,
    calls: &[
        (0x2de, Callee::Sig(&crate::sa_misc::LEAVE)),
        (
            0x2e3,
            Callee::Sig(&crate::o4_tiny4::THREE_SLOTS_M_0514_3180),
        ),
        // __security_check_cookie
        (0x2f3, Callee::Any),
    ],
    imports: &[],
};

/// The thunk over [`visit_draw_0514`] (0x10003570, 12 bytes; a vtable's
/// `+0x1c` method, cdecl `(cursor)`): `ecx` = the cursor, then the tail
/// jump with the entry stack as it was.
pub fn thunk_visit_draw_0514<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let cursor = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    Ret::eax(visit_draw_0514(g, img, target, fr.esp, cursor))
}

/// [`thunk_visit_draw_0514`] (12 bytes, 1 overlay).
pub const THUNK_VISIT_DRAW_0514: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT_DRAW_0514))],
    imports: &[],
};

/// The two frames of a placement: `0, ang, 0, 0` then the position words
/// at `pos` (`x`, `y`, `z`, `w`).
fn pair_frame<G: Game>(g: &mut G, at: u32, ang: f32, pos: [u32; 4]) {
    g.write::<u32>(at, 0);
    g.write::<f32>(at + 4, ang);
    g.write::<u32>(at + 8, 0);
    g.write::<u32>(at + 0xc, 0);
    for (i, src) in (0u32..).zip(pos) {
        let v = g.read::<u32>(src);
        g.write::<u32>(at + 0x10 + 4 * i, v);
    }
}

/// Four words copied from `src` to `dst`.
fn copy4<G: Game>(g: &mut G, dst: u32, src: u32) {
    for i in 0..4 {
        let v = g.read::<u32>(src + 4 * i);
        g.write::<u32>(dst + 4 * i, v);
    }
}

/// `magic_0600`'s placement of the caster and the summon (0x10006870,
/// 1385 bytes; no arguments, `esp` the entry stack pointer; a cookie
/// frame, `ebp = E - 4`, `pos` at `ebp - 0x24`): [`place_pair_0598`]'s
/// first half with the state `S = *G_STATE` 0x354 bytes further on (the
/// position `S + 0xcc4`, the frames `S + 0xc44`, `+0xc64`, the models
/// `S + 0xac4`, `+0xb04` copied to `+0xaf4`, `+0xb34`); slot 238
/// `(index, 1)` shows the summon, slot 296 `(index, 0)` hides the caster
/// (`*G_OFF` 0, index below 32) or slot 257 `(summon, 1)`; `S + 0xeb4` =
/// `ang`, `+0xeac`, `+0xeb0`, `+0xec8` 0 and two flags set. Returns slot
/// 296's or 257's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn place_pair_0600<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let sel = |sel: u32, table: u32| Who::Sel { sel, at: 0, table };
    let chr = crate::ob_model::who(g, img, f, sel(0x11, 0x2e));
    let s = g.read::<u32>(w(0x1b));
    let (px, py, pz) = (l(0x14), l(0x10), l(0xc));
    let mut stack = vec![chr, px, py, pz];
    g.call(t(g, 0x33, 262), 0, &stack);
    for (off, src) in [(0xcc4u32, px), (0xcc8, py), (0xccc, pz)] {
        let v = g.read::<u32>(src);
        g.write::<u32>(s + off, v);
    }
    g.write::<u32>(s + 0xcd0, ONE);
    g.write::<u32>(l(8), ONE);
    let chr2 = crate::ob_model::who(g, img, f, sel(0x71, 0x82));
    stack.insert(0, chr2);
    // call width: 5 (the slot 262 words stay on the stack)
    let rot = st0_f32(g.call(t(g, 0x87, 333), 0, &stack).st0);
    let x = sub(g.read::<f32>(w(0x9a)), rot);
    let ang = wrap_once(g, img, f, x, [0xa9, 0xb8, 0xc2, 0xcf]);
    let pos4 = [px, py, pz, l(8)];
    let turns = [Angle::Bits(0), Angle::Bits(0), Angle::Bits(ang.to_bits())];
    pair_frame(g, s + 0xc44, ang, pos4);
    let selp = g.read::<u32>(w(0x134));
    let k = u32::from(g.read::<u8>(selp));
    let chr = g.read::<u32>(w(0x145).wrapping_add(4 * k));
    let pos = l(0x24);
    let m = s.wrapping_add(0xac4);
    let root_ops = RootOps {
        s210: 0x154,
        off: 0x167,
        s217: 0x17b,
        s255: 0x18d,
        s211: 0x1a3,
    };
    root_matrix(g, img, f, &root_ops, (chr, m, pos), |g| {
        u32::from(g.read::<u8>(selp))
    });
    let model = ModelOps {
        s235: 0x1b0,
        c277: sel(0x1be, 0x1c8),
        s277: 0x1cd,
        k: 0x1e5,
        c: sel(0x1f8, 0x202),
        s15: 0x207,
        s16: 0x21e,
        s17: 0x23a,
        s330: 0x257,
        s328: 0x275,
        s329: 0x293,
    };
    scale_model(g, img, f, &model, m, &[chr, m], turns);
    copy4(g, s + 0xaf4, s + 0xc54);
    pair_frame(g, s + 0xc64, ang, pos4);
    let summon = g.read::<u32>(w(0x33c));
    let m = s.wrapping_add(0xb04);
    let root_ops = RootOps {
        s210: 0x350,
        off: 0x363,
        s217: 0x37a,
        s255: 0x38c,
        s211: 0x3a2,
    };
    root_matrix(g, img, f, &root_ops, (summon, m, pos), |g| {
        g.read::<u32>(w(0x36c))
    });
    let model = ModelOps {
        s235: 0x3af,
        c277: Who::Word(0x3c3),
        s277: 0x3bd,
        k: 0x3db,
        c: Who::Word(0x3f4),
        s15: 0x3e0,
        s16: 0x40b,
        s17: 0x41a,
        s330: 0x444,
        s328: 0x462,
        s329: 0x480,
    };
    scale_model(g, img, f, &model, m, &[summon, m], turns);
    copy4(g, s + 0xb34, s + 0xc74);
    let i = g.read::<u32>(w(0x4cf));
    // the second turn's three words stay on the stack
    g.call(t(g, 0x4c7, 238), 0, &[i, 1, m, m, ang.to_bits()]);
    let off = g.read::<u32>(w(0x4e1)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x4ea)) };
    let eax = if !off && i.cast_signed() < 0x20 {
        g.call(t(g, 0x4f7, 296), 0, &[i, 0]).eax
    } else {
        let summon = g.read::<u32>(w(0x50c));
        g.call(t(g, 0x504, 257), 0, &[summon, 1]).eax
    };
    g.write::<f32>(s + 0xeb4, ang);
    for o in [0xeacu32, 0xeb0, 0xec8] {
        g.write::<u32>(s + o, 0);
    }
    for o in [0x54du32, 0x557] {
        g.write::<u32>(w(o), 1);
    }
    eax
}

/// [`place_pair_0600`] (1,385 bytes, 1 overlay).
pub const PLACE_PAIR_0600: Sig = Sig {
    len: 1385,
    hash: 0x8c33_f8fa_fab8_1504,
    // __security_check_cookie
    calls: &[(0x561, Callee::Any)],
    imports: &[],
};

/// `magic_0600`'s start (0x10006530, 417 bytes; no arguments, called by
/// [`cue_start_0600`]): the byte `G_T` 0x1e and four globals from
/// immediates (the state `*G_STATE` among them); slot 215 `(0x1e)` into
/// `G_A`, `G_N` 2, slot 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`part_setup`](crate::sb_misc::part_setup) `(BUF)`), the pair placed
/// ([`place_pair_0600`]), the effect reset and scaled
/// ([`reset_and_scale`](crate::s4_small::reset_and_scale)); the two
/// script blocks' callbacks, words, halfwords and bytes; slot 271 `(4)`;
/// the 16 records `0x140` bytes apart cleared at their first word (`G_B`,
/// `G_C` 0, `G_D` 16); two task nodes (kinds 1 and 2) appended to the
/// task list, the records set up
/// ([`records_init_400`](crate::sa_misc::records_init_400)) between them.
/// Returns 0.
pub fn start_0600<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    g.write::<u8>(w(0x7), 0x1e);
    g.write::<u32>(w(0x16), 0);
    for o in [0x1eu32, 0x28, 0x32] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    let a = g.call(t(g, 0x1, 215), 0, &[0x1e]).eax;
    g.write::<u32>(w(0x3f), a);
    g.write::<u32>(w(0x4c), 2);
    // call width: 4 (slot 215's word under it)
    let h = g.call(t(g, 0x44, 719), 0, &[w(0x5d), 8, 0, 0x1e]).eax;
    g.write::<u32>(w(0x67), h);
    // push BUF over slot 215's word: the return address at E - 0xc
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, 0x1e);
    g.write::<u32>(esp + 4, w(0x6c));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x71), esp);
    place_pair_0600(g, img, img.rel32_target(f + 0x76), esp);
    let rf = Frame {
        at: img.rel32_target(f + 0x7b),
        esp,
        ..Frame::default()
    };
    crate::s4_small::reset_and_scale(g, img, &rf);
    g.write::<u32>(w(0x86), w(0x8a));
    g.write::<u32>(w(0x90), 0);
    g.write::<u16>(w(0x9b), 0);
    g.write::<u8>(w(0xa3), 1);
    g.write::<u32>(w(0xaa), w(0xae));
    g.write::<u32>(w(0xb4), 0);
    g.write::<u16>(w(0xbf), 0);
    g.write::<u8>(w(0xc7), 1);
    // call width: 3 (BUF and slot 215's word under it)
    g.call(t(g, 0x80, 271), 0, &[4, w(0x6c), 0x1e]);
    g.write::<u32>(w(0xdb), w(0xdf));
    let recs = w(0xe4);
    for i in 0..16 {
        g.write::<u32>(recs + 0x140 * i, 0);
    }
    g.write::<u32>(w(0x101), 0);
    g.write::<u32>(w(0x10c), 0x10);
    for (call, cb, kind, ops, then) in [
        (
            0x115u32,
            0x106u32,
            1u8,
            [0x11bu32, 0x12f, 0x14e, 0x141, 0x14e],
            true,
        ),
        (0x15d, 0x158, 2, [0x163, 0x177, 0x184, 0x192, 0x19a], false),
    ] {
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
        g.write::<u8>(node.wrapping_add(0x18), kind);
        g.write::<u32>(node.wrapping_add(4), 0);
        crate::r12h_start::append(g, img, f, node, ops);
        if then {
            crate::sa_misc::records_init_400(g, img, img.rel32_target(f + 0x153), w(0x146));
        }
    }
    Ret::eax(0)
}

/// [`start_0600`] (417 bytes, 1 overlay).
pub const START_0600: Sig = Sig {
    len: 417,
    hash: 0x8e1a_d695_bb20_f87f,
    calls: &[
        (0x71, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x76, Callee::Sig(&PLACE_PAIR_0600)),
        (0x7b, Callee::Sig(&crate::s4_small::RESET_AND_SCALE_430)),
        (0x115, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x153, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x15d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0600`'s load step (0x10006400, 176 bytes; a table entry):
/// [`cue_start`](crate::m0591::cue_start)'s copy that runs
/// [`start_0600`] where the others set up their targets.
pub fn cue_start_0600<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0591::cue_start_with(g, img, fr, |g, img, fr| {
        start_0600(g, img, fr);
    })
}

/// [`cue_start_0600`] (176 bytes, 1 overlay).
pub const CUE_START_0600: Sig = Sig {
    len: 176,
    hash: 0x986e_8935_7b33_91de,
    calls: &[(0x95, Callee::Sig(&START_0600))],
    imports: &[],
};

/// `magic_0600`'s effect end (0x10006e80, 614 bytes; internal, no
/// arguments, called by [`task_0600`]; a cookie frame, `ebp = E - 4`, the
/// positions at `ebp - 0x14` and `ebp - 0x24`): slot 384 of the third
/// selected character; slots 25 `(0x3016, 1)` and 24 `(0x3016, 0)`; the
/// caster (the first selected), the summon `*G_SUMMON` and the second
/// selected character back at their own places ([`reset_one`]); slot 213
/// `(*G_A, 0)` while the u16 `*G_WA` is 0, the flag `G_F` 1; the caster
/// shown (slot 296 `(i, 1)` for an index below 32, the battle on, else
/// slot 257 `(chr, 0)`); the party's list run
/// ([`each_actor`](crate::sa_script::each_actor), `ecx` 1); the summon
/// shown likewise (slot 296 `(*G_INDEX, 1)` or 257 `(summon, 0)`); slots
/// 371 `(0, 0)`, `(1, 0)`; the node `*G_N` back to the pool
/// ([`give_back`](crate::sa_pool::give_back)) and cleared; the objects
/// freed ([`objects_freed`](crate::s6_misc::objects_freed)) and the parts
/// registered ([`register_parts`](crate::sa_misc::register_parts) with
/// the free's `eax`). Returns what that leaves.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn effect_end_0600<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let third = crate::ob_model::who(
        g,
        img,
        f,
        Who::Sel {
            sel: 0x11,
            at: 2,
            table: 0x1e,
        },
    );
    g.call(t(g, 0x23, 384), 0, &[third]);
    g.call(t(g, 0x30, 25), 0, &[0x3016, 1, third]);
    g.call(t(g, 0x41, 24), 0, &[0x3016, 0]);
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
    let selp = g.read::<u32>(w(0x101));
    let k = u32::from(g.read::<u8>(selp.wrapping_add(1)));
    let second = g.read::<u32>(w(0x116).wrapping_add(4 * k));
    reset_one(
        g,
        img,
        f,
        (0x10a, 0x126, 0x138, 0x149),
        second,
        |_| k,
        ebp.wrapping_sub(0x14),
    );
    if g.read::<u16>(w(0x15e)) == 0 {
        let a = g.read::<u32>(w(0x170));
        g.call(t(g, 0x168, 213), 0, &[a, 0]);
    }
    let off = g.read::<u32>(w(0x181));
    let selp = g.read::<u32>(w(0x187));
    g.write::<u32>(w(0x18d), 1);
    let c = u32::from(g.read::<u8>(selp));
    if off == 0 && c < 0x20 {
        g.call(t(g, 0x1a0, 296), 0, &[c, 1]);
    } else {
        let chr = g.read::<u32>(w(0x1b7).wrapping_add(4 * c));
        g.call(t(g, 0x1bc, 257), 0, &[chr, 0]);
    }
    // the return address at `E - 0x2c`
    crate::sa_script::each_actor(
        g,
        img,
        img.rel32_target(f + 0x1d1),
        1,
        esp.wrapping_sub(0x2c),
    );
    let off = g.read::<u32>(w(0x1d7)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x1df)) };
    if !off && i.cast_signed() < 0x20 {
        g.call(t(g, 0x1ec, 296), 0, &[i, 1]);
    } else {
        let summon = g.read::<u32>(w(0x201));
        g.call(t(g, 0x1f9, 257), 0, &[summon, 0]);
    }
    g.call(t(g, 0x20e, 371), 0, &[0, 0]);
    g.call(t(g, 0x222, 371), 0, &[1, 0]);
    let node = g.read::<u32>(w(0x234));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x240), node);
        g.write::<u32>(w(0x246), 0);
    }
    let of = Frame {
        at: img.rel32_target(f + 0x24f),
        esp: esp.wrapping_sub(0x2c),
        ..Frame::default()
    };
    let eax = crate::s6_misc::objects_freed(g, img, &of).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x254), eax)
}

/// [`effect_end_0600`] (614 bytes, 1 overlay).
pub const EFFECT_END_0600: Sig = Sig {
    len: 614,
    hash: 0xefd5_52cc_e592_98df,
    calls: &[
        (0x1d1, Callee::Sig(&crate::sa_script::EACH_ACTOR)),
        (0x240, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x24f, Callee::Sig(&crate::s6_misc::OBJECTS_FREED)),
        (0x254, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        // __security_check_cookie
        (0x25e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0600`'s per-frame task (0x100066e0, 326 bytes; cdecl `(node)`,
/// pushed by [`start_0600`]): [`task`](crate::m0598::task)'s steps with
/// four points copied (`S + 0xc54`, `+0xc74`, `+0xc94`, `+0xcb4` to `S +
/// 0xaf4`, `+0xb34`, `+0xb74`, `+0xbb4`) and two motion scripts; once
/// the first script is done, `S + 0xec8` = 1, the effect ends
/// ([`effect_end_0600`]) and the node goes back to the pool (a tail
/// jump). Returns the tail jump's answer.
pub fn task_0600<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), v);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first != 0 {
        g.write::<u32>(s + 0xec8, 1);
        // push ebp, esi, edi: the return address at E - 0x10
        effect_end_0600(
            g,
            img,
            img.rel32_target(f + 0x137),
            fr.esp.wrapping_sub(0x10),
        );
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x142),
            node,
        ));
    }
    for (from, to) in [
        (0xc54u32, 0xaf4u32),
        (0xc74, 0xb34),
        (0xc94, 0xb74),
        (0xcb4, 0xbb4),
    ] {
        copy4(g, s + to, s + from);
    }
    for (op, call) in [(0x10bu32, 0x116u32), (0x11b, 0x120)] {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + call), w(op));
    }
    let rigs = Frame {
        at: img.rel32_target(f + 0x128),
        ..*fr
    };
    crate::s8_misc::camera_rigs_step(g, img, &rigs)
}

/// [`task_0600`] (326 bytes, 1 overlay).
pub const TASK_0600: Sig = Sig {
    len: 326,
    hash: 0x75f7_3732_cc8d_d5e3,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x116, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x120, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x128, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x137, Callee::Sig(&EFFECT_END_0600)),
        (0x142, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0568`'s placement (0x10001570, 740 bytes; internal, no
/// arguments, called by [`start_up_0568`]; a cookie frame,
/// `ebp = E - 4`): with the state `S = *G_STATE`, the summon
/// `*G_SUMMON`'s position (slot 262 into `ebp - 0x14..-0xc`) into
/// `S + 0x1438`, `w` 1.0; `ang = A - Ch_GetRot(summon)` (slot 333,
/// wrapped once); the frame `S + 0x1418`
/// (`0, ang, 0`, the stale word, the position, `w`); the summon's part
/// shown ([`part_toggle`](crate::s4_misc::part_toggle) of `S + 0x13d8`)
/// and its matrix scaled and turned by `(0, ang, 0)`
/// ([`scale_rotate`](crate::sa_misc::scale_rotate)); `S + 0x1408..` = the
/// position; slot 238 `(index, 1)`; slot 296 `(index, 0)` (`*G_OFF` 0, the
/// index below 32) or slot 257 `(summon, 1)`; `S + 0x16bc` = `ang`; the
/// camera point `S + 0x1448` = the position moved by `(0, 0, -5)` turned
/// by X 0 and Y `ang` (slots 235, 328, 329 into `ebp - 0x84`, slot 229
/// into `ebp - 0x44`, slot 248 adds it), `w` 1.0; `S + 0x16c8` 0, a flag
/// set. Returns slot 248's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn place_0568<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let s = g.read::<u32>(w(0x17));
    let (px, py, pz) = (l(0x14), l(0x10), l(0xc));
    let summon = g.read::<u32>(w(0x2e));
    g.call(t(g, 0x28, 262), 0, &[summon, px, py, pz]);
    for (off, src) in [(0x1438u32, px), (0x143c, py), (0x1440, pz)] {
        let v = g.read::<u32>(src);
        g.write::<u32>(s + off, v);
    }
    g.write::<u32>(s + 0x1444, ONE);
    g.write::<u32>(l(8), ONE);
    let v = l(0x24);
    g.write::<u32>(v, 0);
    let summon2 = g.read::<u32>(w(0x72));
    // call width: 5 (the slot 262 words stay on the stack)
    let r = g.call(t(g, 0x40, 333), 0, &[summon2, summon, px, py, pz]);
    let x = sub(g.read::<f32>(w(0x96)), st0_f32(r.st0));
    let ang = wrap_once(g, img, f, x, [0xa8, 0xb2, 0xbc, 0xc9]);
    // junk read: [ebp-0x18] never written; 0 (brief 2026-10-24)
    g.write::<u32>(s + 0x1424, 0);
    g.write::<u32>(s + 0x1418, 0);
    g.write::<f32>(s + 0x141c, ang);
    g.write::<u32>(s + 0x1420, 0);
    for (i, src) in (0u32..).zip([px, py, pz, l(8)]) {
        let x = g.read::<u32>(src);
        g.write::<u32>(s + 0x1428 + 4 * i, x);
    }
    g.write::<f32>(v + 4, ang);
    g.write::<u32>(v + 8, 0);
    let m = s + 0x13d8;
    // the return address at `E - 0x98`
    let pf = Frame {
        at: img.rel32_target(f + 0x13d),
        esp: esp.wrapping_sub(0x98),
        edx: m,
        ..Frame::default()
    };
    crate::s4_misc::part_toggle(g, img, &pf);
    crate::sa_misc::scale_rotate(g, img, img.rel32_target(f + 0x14c), m, v);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x1428 + 4 * i);
        g.write::<u32>(s + 0x1408 + 4 * i, x);
    }
    let i = g.read::<u32>(w(0x189));
    // the vector's pointer stays under it
    g.call(t(g, 0x181, 238), 0, &[i, 1, v]);
    let off = g.read::<u32>(w(0x19a)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x1a2)) };
    if !off && i.cast_signed() < 0x20 {
        g.call(t(g, 0x1af, 296), 0, &[i, 0]);
    } else {
        let summon = g.read::<u32>(w(0x1c4));
        g.call(t(g, 0x1bc, 257), 0, &[summon, 1]);
    }
    g.write::<f32>(s + 0x16bc, ang);
    let cam = s + 0x1448;
    for (i, src) in (0u32..).zip([px, py, pz, l(8)]) {
        let x = g.read::<u32>(src);
        g.write::<u32>(cam + 4 * i, x);
    }
    let (mat, d, out) = (l(0x84), l(0x34), l(0x44));
    g.write::<u32>(d, 0);
    g.write::<u32>(d + 4, 0);
    g.write::<u32>(d + 8, 0xc0a0_0000);
    g.call(t(g, 0x209, 235), 0, &[mat]);
    let a0 = g.read::<u32>(v);
    g.call(t(g, 0x252, 328), 0, &[mat, mat, a0]);
    let a1 = g.read::<u32>(v + 4);
    g.call(t(g, 0x274, 329), 0, &[mat, mat, a1]);
    g.write::<u32>(d + 0xc, ONE);
    g.call(t(g, 0x290, 229), 0, &[out, mat, d]);
    let eax = g.call(t(g, 0x2a8, 248), 0, &[cam, cam, out]).eax;
    g.write::<u32>(cam + 0xc, ONE);
    g.write::<u32>(s + 0x16c8, 0);
    g.write::<u32>(w(0x2d2), 1);
    eax
}

/// [`place_0568`] (740 bytes, 1 overlay).
pub const PLACE_0568: Sig = Sig {
    len: 740,
    hash: 0x72f9_a91d_ba53_6bff,
    calls: &[
        (0x13d, Callee::Sig(&crate::s4_misc::PART_TOGGLE)),
        (0x14c, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        // __security_check_cookie
        (0x2dc, Callee::Any),
    ],
    imports: &[],
};

/// One of [`start_up_0568`]'s three particle loads: slot 719 `(P, 8, k)`
/// into `G_H`, `pppInitData(h, BUF, 0)` (slot 711, the game lock
/// `**G_LOCK` 1 around it), slot 710 `(OUT, *G_H, DST, size)`, four of the
/// DLL's words cleared. `ops` are the operand offsets: slot 719's table,
/// `P`, `G_H`, the lock, slot 711's table, `BUF`, the lock again, slot
/// 710's table, `size`, `DST`, `*G_H`, `OUT`, the four words.
fn part_load_0568<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 16], k: u32) {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // (each call's words go above the earlier calls' words, which the
    // callees never read)
    let h = g.call(t(g, ops[0], 719), 0, &[w(ops[1]), 8, k]).eax;
    g.write::<u32>(w(ops[2]), h);
    let lock = g.read::<u32>(w(ops[3]));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    g.call(t(g, ops[4], 711), 0, &[h, w(ops[5]), 0]);
    let lock = g.read::<u32>(w(ops[6]));
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(ops[10]));
    // call width: 4
    g.call(t(g, ops[7], 710), 0, &[w(ops[11]), h, w(ops[9]), w(ops[8])]);
    for &o in &ops[12..16] {
        g.write::<u32>(w(o), 0);
    }
}

/// `magic_0568`'s start-up (0x10003030, 720 bytes; no arguments, called
/// by its load step): four globals from immediates (the state `*G_STATE`
/// among them); three particle loads ([`part_load_0568`], `k` 0, 1, 2);
/// the placement ([`place_0568`]); the effect's turned matrix
/// ([`start_turned`](crate::o4_effect::start_turned)); the two script
/// blocks' callbacks, words, halfwords and bytes; slot 271 `(4)`; two
/// task nodes (kinds 1 and 2) appended to the task list, the records set
/// up ([`records_init_400`](crate::sa_misc::records_init_400)) between
/// them. Returns 0.
pub fn start_up_0568<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    g.write::<u32>(w(0x17), 0);
    for o in [0x1fu32, 0x29, 0x33] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    let loads: [([u32; 16], u32); 3] = [
        (
            [
                0x1, 0x11, 0x48, 0x41, 0x55, 0x4f, 0x68, 0x74, 0x6d, 0x79, 0x7f, 0x8a, 0x99, 0xa3,
                0xad, 0xb7,
            ],
            0,
        ),
        (
            [
                0x91, 0xc8, 0xd7, 0xd0, 0xe4, 0xde, 0xf7, 0x101, 0x106, 0x111, 0x117, 0x11c, 0x12b,
                0x135, 0x13f, 0x149,
            ],
            1,
        ),
        (
            [
                0x123, 0x15a, 0x169, 0x162, 0x176, 0x170, 0x189, 0x195, 0x18e, 0x19a, 0x1a0, 0x1ab,
                0x1b3, 0x1bd, 0x1c7, 0x1d1,
            ],
            2,
        ),
    ];
    for (ops, k) in loads {
        part_load_0568(g, img, f, ops, k);
    }
    // push esi and the loads' last words: the return address at E - 0x40
    let esp = fr.esp.wrapping_sub(0x40);
    place_0568(g, img, img.rel32_target(f + 0x1da), esp);
    let tf = Frame {
        at: img.rel32_target(f + 0x1df),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &tf);
    g.write::<u32>(w(0x1ea), w(0x1ee));
    g.write::<u32>(w(0x1f4), 0);
    g.write::<u16>(w(0x1ff), 0);
    g.write::<u8>(w(0x207), 1);
    g.write::<u32>(w(0x20e), w(0x212));
    g.write::<u32>(w(0x218), 0);
    g.write::<u16>(w(0x223), 0);
    g.write::<u8>(w(0x22b), 1);
    g.call(t(g, 0x1e4, 271), 0, &[4]);
    for (call, cb, kind, ops, then) in [
        (
            0x243u32,
            0x23eu32,
            1u8,
            [0x249u32, 0x25e, 0x27d, 0x270, 0x27d],
            true,
        ),
        (0x28c, 0x287, 2, [0x292, 0x2a6, 0x2b3, 0x2c1, 0x2c9], false),
    ] {
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
        g.write::<u8>(node.wrapping_add(0x18), kind);
        g.write::<u32>(node.wrapping_add(4), 0);
        crate::r12h_start::append(g, img, f, node, ops);
        if then {
            crate::sa_misc::records_init_400(g, img, img.rel32_target(f + 0x282), w(0x275));
        }
    }
    Ret::eax(0)
}

/// [`start_up_0568`] (720 bytes, 1 overlay).
pub const START_UP_0568: Sig = Sig {
    len: 720,
    hash: 0x25d3_b276_2a87_a118,
    calls: &[
        (0x1da, Callee::Sig(&PLACE_0568)),
        (
            0x1df,
            Callee::Sig(&crate::o4_effect::START_TURNED_0568_1860),
        ),
        (0x243, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x282, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x28c, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0568`'s load step (0x10001340, the 118-byte shape) running
/// [`start_up_0568`].
pub fn load_0568<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0568)
}

/// [`load_0568`].
pub const LOAD_0568: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0568))],
    imports: &[],
};
