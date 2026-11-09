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

/// `magic_0519`'s camera tracks (0x10002320, 644 bytes; internal, `ecx`
/// = the script record `R`): [`camera_tracks`](crate::s6_misc::camera_tracks)
/// with the record in `ecx` and its frame count at `R + 0x28` (`d = R +
/// 0x28 - (i16) *G_T`, `K1` the constant at `+0x04`), the stores in
/// another order. Track 1: `R[0]`, the rate `K1 / (d / R[4])`, 0, `d`,
/// `R[4]`, kind 3 (without `R + 0x10`) or `R[0]`, `R + 0x10`, `K1 / d`,
/// `d`, 0, `R[4]`, kind 4; track 2 likewise from `R[8]`, `R[0xc]`; track
/// 3 from `R + 0x14` by `(R + 0x18 - R + 0x14) / d`, 0, kind 1, or the
/// record `R + 0x14`, `R + 0x10`, 0, `K1 / d`, kinds 2 and 3 and `2d`.
/// Then the angle track from `a = R + 0x1c` to `b = R + 0x20` (the end
/// the short way round), `d` twice and on. Returns `d`.
#[allow(clippy::too_many_lines)] // the original's tracks, word by word
pub fn camera_tracks_0519<G: Game>(g: &mut G, img: &Image, f: u32, r: u32) -> u32 {
    use crate::sb_sse::{add, div};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let one = k(g, 0x4);
    let base = i32::from(g.read::<i16>(w(0xc)));
    let rd = |g: &G, o: u32| g.read::<u32>(r.wrapping_add(o));
    let left = |g: &G| rd(g, 0x28).wrapping_sub(base.cast_unsigned());
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let fl = |x: u32| x.cast_signed() as f32;
    let set = |g: &mut G, o: u32, v: u32| g.write::<u32>(w(o), v);
    let flag = rd(g, 0x10);
    let r0 = rd(g, 0);
    let eax = if flag == 0 {
        let d = left(g);
        let n = rd(g, 4);
        set(g, 0x3b, r0);
        let rate = div(one, div(fl(d), fl(n)));
        set(g, 0x4c, n);
        g.write::<f32>(w(0x54), rate);
        set(g, 0x5a, 0);
        set(g, 0x64, d);
        set(g, 0x6a, 3);
        let d = left(g);
        let c = rd(g, 0xc);
        let x = div(fl(d), fl(c));
        set(g, 0x90, rd(g, 8));
        set(g, 0x9a, 0);
        g.write::<f32>(w(0xa6), div(one, x));
        set(g, 0xac, d);
        set(g, 0xb2, c);
        set(g, 0xb8, 3);
        let a = g.read::<f32>(r.wrapping_add(0x14));
        let d = left(g);
        let b = g.read::<f32>(r.wrapping_add(0x18));
        g.write::<f32>(w(0xd7), a);
        set(g, 0xe4, 0);
        set(g, 0xf2, 1);
        g.write::<f32>(w(0xfe), div(sub(b, a), fl(d)));
        d
    } else {
        let d = left(g);
        let n = rd(g, 4);
        set(g, 0x11e, r0);
        g.write::<f32>(w(0x126), div(one, fl(d)));
        set(g, 0x12c, flag);
        set(g, 0x132, d);
        set(g, 0x138, 0);
        set(g, 0x142, n);
        set(g, 0x148, 4);
        let d = left(g);
        let (c, fl10, e8) = (rd(g, 0xc), rd(g, 0x10), rd(g, 8));
        set(g, 0x16d, e8);
        set(g, 0x173, c);
        set(g, 0x179, fl10);
        set(g, 0x17f, 0);
        g.write::<f32>(w(0x18b), div(one, fl(d)));
        set(g, 0x191, d);
        set(g, 0x197, 4);
        let d = left(g);
        let fl10 = rd(g, 0x10);
        set(g, 0x1b6, r.wrapping_add(0x14));
        set(g, 0x1bc, fl10);
        set(g, 0x1c2, 0);
        g.write::<f32>(w(0x1ce), div(one, fl(d)));
        set(g, 0x1d7, 2);
        set(g, 0x1e1, 3);
        d.wrapping_mul(2)
    };
    set(g, 0x1ea, eax);
    let (a, b) = (
        g.read::<f32>(r.wrapping_add(0x1c)),
        g.read::<f32>(r.wrapping_add(0x20)),
    );
    let d = left(g);
    // comiss: each test false when unordered
    let end = if 0.0 > a {
        let kk = k(g, 0x211);
        if b > 0.0 && sub(b, a) > kk {
            sub(k(g, 0x225), sub(kk, b))
        } else {
            b
        }
    } else {
        let kk = k(g, 0x23c);
        if 0.0 > b && sub(a, b) > kk {
            add(add(b, kk), kk)
        } else {
            b
        }
    };
    g.write::<f32>(w(0x25b), end);
    g.write::<f32>(w(0x263), a);
    g.write::<f32>(w(0x26b), b);
    set(g, 0x270, d);
    set(g, 0x275, d);
    set(g, 0x27b, 1);
    d
}

/// [`camera_tracks_0519`] (644 bytes, 1 overlay).
pub const CAMERA_TRACKS_0519: Sig = Sig {
    len: 644,
    hash: 0xb1b0_e40d_6d39_d463,
    calls: &[],
    imports: &[],
};

/// `magic_0519`'s camera step (0x100025b0, 69 bytes; internal, no
/// arguments): the u16 frame `*G_T` steps on; with a record `R = *G_R`,
/// the count `G_N` steps, and at the record's last frame (`R + 0x24` =
/// the old frame) the record count `G_C` steps, `*G_R = R + 0x2c`, `G_N`
/// 0 and the next record's tracks start ([`camera_tracks_0519`], a tail
/// jump). Returns that `d`, else `eax` with its low half the old frame.
pub fn camera_step_0519<G: Game>(g: &mut G, img: &Image, f: u32, eax: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = g.read::<u16>(w(0x2));
    let r = g.read::<u32>(w(0x8));
    g.write::<u16>(w(0x13), t.wrapping_add(1));
    let eax = (eax & !0xffff) | u32::from(t.wrapping_add(1));
    if r == 0 {
        return eax;
    }
    let n = g.read::<u32>(w(0x1d)).wrapping_add(1);
    g.write::<u32>(w(0x1d), n);
    if i32::from(t.cast_signed()).cast_unsigned() != g.read::<u32>(r + 0x24) {
        return eax;
    }
    let c = g.read::<u32>(w(0x28)).wrapping_add(1);
    g.write::<u32>(w(0x28), c);
    let next = r + 0x2c;
    g.write::<u32>(w(0x31), next);
    g.write::<u32>(w(0x37), 0);
    camera_tracks_0519(g, img, img.rel32_target(f + 0x40), next)
}

/// [`camera_step_0519`] (69 bytes, 1 overlay).
pub const CAMERA_STEP_0519: Sig = Sig {
    len: 69,
    hash: 0x1071_4f60_b028_2ab9,
    calls: &[(0x40, Callee::Sig(&CAMERA_TRACKS_0519))],
    imports: &[],
};

/// `magic_0519`'s parts step (0x100027a0, 678 bytes; internal, `ecx` = a
/// block `B`, called twice by [`frame_0519`]; a cookie frame, `ebp = E -
/// 4`, a matrix at `ebp - 0x54`, a point at `ebp - 0x14`): each of the 12
/// parts `P = B + 0xa0 i` with its u16 `+0x4e` set runs by its mode byte
/// `+0x10`:
/// - 0: its point `P + 0x70` (`+0x60` 1.0) from its bone `P[0x14]`
///   (slot 237 `(B[0], bone, 0, P + 0x70)`; with a second bone `P[0x18]`
///   the middle of both (`(a + b) K` with slot 237's point) plus the
///   offset `P + 0x1c..`; without one plus the offset; without the first
///   the offset alone), then refreshed
///   ([`part_refresh_b`](crate::sa_misc::part_refresh_b) of `P + 0x30`);
/// - 1: the bone's matrix (slot 236 `(B[0], bone, M)`), its three rows
///   scaled by `P[0x90] K'` into the DLL's matrix `*G_M` (slot 233), its
///   fourth row copied, moved by the offset (slot 229 `(M3, M, P +
///   0x1c)`), `w` 1.0; slots 231 `(*G_A, *G_B, M)` and 230 `(*G_C, *G_D)`,
///   `**G_P` = `ITEM`, slot 712 `(P + 0x30, 0x30)` with `**G_LOCK` 1
///   around it; when it answers non-zero the part's `+0x4e` is cleared;
/// - 2: its point the offset, `w` 1.0, the part dropped
///   ([`part_drop`](crate::s4_small::part_drop) of `P + 0x30`).
///
/// Returns the last `eax` (each step's, the mode less 2 for another
/// mode, the cookie `__security_cookie ^ ebp` when no part ran).
#[allow(clippy::too_many_lines)] // the original's three modes, in order
pub fn parts_step_0519<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, b: u32) -> u32 {
    use crate::sb_sse::{add, mul};
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let mut eax = g.read::<u32>(w(0x7)) ^ ebp;
    let (m, pt) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    // push ebx, esi, edi: the callees' return address at E - 0x70
    let callee_esp = esp.wrapping_sub(0x70);
    for i in 0..12u32 {
        let p = b + 0x70 + 0xa0 * i;
        if g.read::<u16>(p - 0x22) == 0 {
            continue;
        }
        let mode = u32::from(g.read::<u8>(p - 0x60));
        let r0 = g.read::<u32>(b);
        let copy_offset = |g: &mut G| {
            for k in 0..3 {
                let x = g.read::<u32>(p - 0x54 + 4 * k);
                g.write::<u32>(p + 4 * k, x);
            }
        };
        match mode {
            0 => {
                let bone = g.read::<u32>(p - 0x5c);
                if bone == 0xffff_ffff {
                    copy_offset(g);
                    eax = g.read::<u32>(p - 0x4c);
                } else {
                    g.call(t(g, 0x1c7, 237), 0, &[r0, bone, 0, p]);
                    let bone2 = g.read::<u32>(p - 0x58);
                    eax = bone2;
                    if bone2 != 0xffff_ffff {
                        let r0 = g.read::<u32>(b);
                        eax = g.call(t(g, 0x1e8, 237), 0, &[r0, bone2, 0, pt]).eax;
                        let k = g.read::<f32>(w(0x203));
                        for c in 0..3 {
                            let v = add(g.read::<f32>(p + 4 * c), g.read::<f32>(pt + 4 * c));
                            g.write::<f32>(p + 4 * c, mul(v, k));
                        }
                    }
                    for c in 0..3 {
                        let v = add(g.read::<f32>(p - 0x54 + 4 * c), g.read::<f32>(p + 4 * c));
                        g.write::<f32>(p + 4 * c, v);
                    }
                }
                g.write::<u32>(p + 0xc, ONE);
                eax = crate::sa_misc::part_refresh_b(
                    g,
                    img,
                    img.rel32_target(f + 0x281),
                    p - 0x40,
                    eax,
                );
            }
            1 => {
                let bone = g.read::<u32>(p - 0x5c);
                g.call(t(g, 0x6f, 236), 0, &[r0, bone, m]);
                let s = mul(g.read::<f32>(p + 0x20), g.read::<f32>(w(0x86)));
                for (r, op, so) in [(0u32, 0x9du32, 0x97u32), (1, 0xc0, 0xc9), (2, 0xe7, 0xf0)] {
                    let dst = g.read::<u32>(w(op)) + 0x10 * r;
                    g.call(t(g, so, 233), 0, &[dst, m + 0x10 * r, s.to_bits()]);
                }
                let gm = g.read::<u32>(w(0xfe));
                for k in 0..4 {
                    let x = g.read::<u32>(m + 0x30 + 4 * k);
                    g.write::<u32>(gm + 0x30 + 4 * k, x);
                }
                g.call(t(g, 0x10f, 229), 0, &[gm + 0x30, gm, p - 0x54]);
                let gm = g.read::<u32>(w(0x12c));
                g.write::<u32>(gm + 0x3c, ONE);
                let (a2, b2) = (g.read::<u32>(w(0x145)), g.read::<u32>(w(0x133)));
                g.call(t(g, 0x13f, 231), 0, &[a2, b2, gm]);
                let (c2, d2) = (g.read::<u32>(w(0x15e)), g.read::<u32>(w(0x153)));
                g.call(t(g, 0x158, 230), 0, &[c2, d2]);
                let gp = g.read::<u32>(w(0x16b));
                g.write::<u32>(gp, w(0x173));
                let lock = g.read::<u32>(w(0x178));
                let saved = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                eax = g.call(t(g, 0x189, 712), 0, &[p - 0x40, 0x30]).eax;
                let lock = g.read::<u32>(w(0x197));
                g.write::<u32>(lock, saved);
                if eax != 0 {
                    eax = 0;
                    g.write::<u16>(p - 0x22, 0);
                }
            }
            2 => {
                copy_offset(g);
                g.write::<u32>(p + 0xc, ONE);
                let df = Frame {
                    at: img.rel32_target(f + 0x5e),
                    esp: callee_esp,
                    ecx: p - 0x40,
                    eax: g.read::<u32>(p - 0x4c),
                    ..Frame::default()
                };
                eax = crate::s4_small::part_drop(g, img, &df).eax;
            }
            _ => eax = mode - 2,
        }
    }
    eax
}

/// [`parts_step_0519`] (678 bytes, 1 overlay).
pub const PARTS_STEP_0519: Sig = Sig {
    len: 678,
    hash: 0x84fc_81de_9581_8897,
    calls: &[
        (0x5e, Callee::Sig(&crate::s4_small::PART_DROP)),
        (0x281, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        // __security_check_cookie
        (0x29e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0519`'s frame (0x10004020, 252 bytes; a table entry, no
/// arguments; an 8-aligned frame): while `*G_ON` is set, `**G_Z` = slot
/// 253 answers 0; slots 723 `(*G_S)` and 724 `(*G_C)`; `**G_P` = `ITEM`;
/// the two part blocks stepped ([`parts_step_0519`]); by the timer `n =
/// *G_T`: frames 0x11f..0x13b fade the model
/// ([`fade`](crate::m0519::fade) of `n - 0x11e`), else the effect timer
/// `e = *G_E - 0xa0` sets the colour filter
/// ([`filter`](crate::m0519::filter)) at `e 0x70 / 40` while below 0x28
/// and `(0x50 - e) 0x70 / 40` up to 0x50; unless `**G_Z` the count `G_N`
/// steps. Returns 0.
pub fn frame_0519<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    if g.read::<u32>(w(0x8)) == 0 {
        return Ret::eax(0);
    }
    let r = g.call(t(g, 0x14, 253), 0, &[]).eax;
    let z = g.read::<u32>(w(0x22));
    let s = g.read::<u32>(w(0x28));
    g.write::<u32>(z, u32::from(r == 0));
    g.call(t(g, 0x34, 723), 0, &[s]);
    let c = g.read::<u32>(w(0x50));
    g.call(t(g, 0x41, 724), 0, &[c]);
    let p = g.read::<u32>(w(0x57));
    g.write::<u32>(p, w(0x65));
    // push ebp; and esp, -8: the callees' return address 4 below it
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(4);
    for (call, blk) in [(0x6au32, 0x5fu32), (0x74, 0x6f)] {
        parts_step_0519(g, img, img.rel32_target(f + call), esp, w(blk));
    }
    let n = g.read::<u32>(w(0x7a));
    if n.wrapping_sub(0x11e) <= 0x1d {
        let k = n.wrapping_sub(0x11e);
        if k.cast_signed() > 0 {
            crate::m0519::fade(g, img, img.rel32_target(f + 0x94), k);
        }
    } else {
        let e = g.read::<i32>(w(0x9c));
        if e >= 0xa0 {
            let d = e - 0xa0;
            let level = if d < 0x28 {
                Some(d * 0x70 / 40)
            } else if d < 0x50 {
                Some((0x50 - d) * 0x70 / 40)
            } else {
                None
            };
            if let Some(level) = level {
                let ff = Frame {
                    at: img.rel32_target(f + 0xe2),
                    esp,
                    ecx: level.cast_unsigned(),
                    ..Frame::default()
                };
                crate::m0519::filter(g, img, &ff);
            }
        }
    }
    let z = g.read::<u32>(w(0xe7));
    if g.read::<u32>(z) == 0 {
        let n = g.read::<u32>(w(0xf2)).wrapping_add(1);
        g.write::<u32>(w(0xf2), n);
    }
    Ret::eax(0)
}

/// [`frame_0519`] (252 bytes, 1 overlay).
pub const FRAME_0519: Sig = Sig {
    len: 252,
    hash: 0x9698_253b_c590_6c94,
    calls: &[
        (0x6a, Callee::Sig(&PARTS_STEP_0519)),
        (0x74, Callee::Sig(&PARTS_STEP_0519)),
        (0x94, Callee::Sig(&crate::m0519::FADE)),
        (0xe2, Callee::Sig(&crate::m0519::FILTER)),
    ],
    imports: &[],
};

/// `magic_0568`'s objects started (0x10002e00, 553 bytes; a table entry,
/// no arguments): with the state `S = *G_STATE`, the object `O = S +
/// 0x850` started for the host `H` ([`start_host_object`](crate::s4_small::start_host_object),
/// scale `K`, `(2, 0x15)`) at the summon's position `S + 0x1438` with its
/// `y` less `D` and its yaw `S + 0x16bc`; `S + 0x142c` = `S + 0x143c`;
/// the timer block `T = S + 0x1474` (0, 40, 0, 50, 1, 1, a halfword 0) and
/// the record `R = S + 0x14c0` (scales 1, 1, 0, 4, 4, 0, count 0x16, flags
/// 3, `+0x20` 0.04) each on a task node (kind 1, callbacks `CB1`, `CB2`)
/// appended to the task list (the second after the first, the list's
/// last set once, to the second); the object `S + 0x1560`'s colour and
/// fields and its pool node ([`object_node`](crate::s8_misc::object_node));
/// the fade task of `S + 0x14f0`
/// ([`fade_task`](crate::s3_misc::fade_task), 0x10 frames, the stack word
/// 0xb4). Returns the fade's node.
#[allow(clippy::too_many_lines)] // one straight sequence of steps
pub fn objects_start_0568<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let s = g.read::<u32>(w(0xb));
    let obj = s + 0x850;
    // push ebx, esi, edi; sub esp, 8; push 0x15; push 2: the return
    // address at E - 0x20
    let esp = fr.esp.wrapping_sub(0x20);
    g.write::<u32>(esp + 4, 2);
    g.write::<u32>(esp + 8, 0x15);
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x4)));
    let hf = Frame {
        at: img.rel32_target(f + 0x26),
        esp,
        ecx: w(0x21),
        edx: obj,
        xmm,
        ..*fr
    };
    crate::s4_small::start_host_object(g, img, &hf);
    for o in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(s + 0x1438 + o);
        g.write::<u32>(obj + 0xd0 + o, v);
    }
    let y = sub(g.read::<f32>(obj + 0xd4), k(g, 0x6c));
    let yaw = g.read::<u32>(s + 0x16bc);
    g.write::<u32>(obj + 0xc4, yaw);
    g.write::<f32>(obj + 0xd4, y);
    let v = g.read::<u32>(s + 0x143c);
    g.write::<u32>(s + 0x142c, v);
    let timer = s + 0x1474;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x28),
        (8, 0),
        (0xc, 0x32),
        (0x14, 1),
        (0x10, 1),
    ] {
        g.write::<u32>(timer + o, v);
    }
    g.write::<u16>(timer + 0x18, 0);
    let n1 = crate::sa_pool::take(g, img, img.rel32_target(f + 0xc8), w(0x7a));
    let first = g.read::<u32>(w(0xce));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, timer);
    g.write::<u32>(n1 + 4, 0);
    if first == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0xfa), n1);
    } else {
        let last = g.read::<u32>(w(0xe7));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let r = s + 0x14c0;
    for (o, v) in [
        (0u32, 0u32),
        (0x1c, 0),
        (0x20, 0x3d23_d70a),
        (0x24, 0),
        (0x28, 0x19),
    ] {
        g.write::<u32>(r + o, v);
    }
    g.write::<u16>(r + 0x2c, 0);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x135), w(0x10a));
    let first = g.read::<u32>(w(0x13b));
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, r);
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x15e), n2);
    } else {
        // the list's last is the first node (its last word not stored yet)
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    for (o, v) in [
        (4u32, ONE),
        (8, ONE),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4080_0000),
        (0x18, 0),
    ] {
        g.write::<u32>(r + o, v);
    }
    let fl = g.read::<u16>(r + 0x2c);
    g.write::<u16>(r + 0x2c, fl | 3);
    g.write::<u32>(r + 0x24, 0x16);
    g.write::<u32>(w(0x1a2), n2);
    let o2 = s + 0x1560;
    g.write::<u32>(o2 + 0xf0, 0x4080_8080);
    g.write::<u16>(o2 + 0x124, 2);
    for (o, v) in [
        (0x11cu32, u32::MAX),
        (0x120, 0x5c),
        (0x110, 0x76),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
    ] {
        g.write::<u32>(o2 + o, v);
    }
    // the pushes undone: the return address at E - 0x10
    let esp = fr.esp.wrapping_sub(0x10);
    let of = Frame {
        at: img.rel32_target(f + 0x1f9),
        esp,
        ecx: o2,
        ..Frame::default()
    };
    crate::s8_misc::object_node(g, img, &of);
    // push 0xb4 under the return address
    let esp = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(esp + 4, 0xb4);
    let ff = Frame {
        at: img.rel32_target(f + 0x21e),
        esp,
        ecx: s + 0x14f0,
        edx: 0x10,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x209))),
            u128::from(g.read::<u32>(w(0x201))),
            0,
        ],
        ..Frame::default()
    };
    Ret::eax(crate::s3_misc::fade_task(g, img, &ff))
}

/// [`objects_start_0568`] (553 bytes, 1 overlay).
pub const OBJECTS_START_0568: Sig = Sig {
    len: 553,
    hash: 0xdb6c_ea77_eda5_bcd8,
    calls: &[
        (0x26, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
        (0xc8, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x135, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1f9, Callee::Sig(&crate::s8_misc::OBJECT_NODE)),
        (0x21e, Callee::Sig(&crate::s3_misc::FADE_TASK)),
    ],
    imports: &[],
};

/// A task node (kind 1, the pool take at the `rel32` `call`, owner `CB`
/// at `cb`) for the block `blk`, linked after the list's last as the
/// overlay inlines it: the list's first at `first` (an operand) decides;
/// an empty list gets it as its first (`head`), else it goes after
/// `prev` (the last word at the operand `last`, or the node just linked).
/// The list's last word itself is left to the caller. Returns the node.
fn chain_node_0568<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, cb, first, head): (u32, u32, u32, u32),
    prev: Result<u32, u32>,
    blk: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
    g.write::<u8>(n + 0x18, 1);
    let listed = g.read::<u32>(w(first)) != 0;
    g.write::<u32>(n + 0x14, blk);
    g.write::<u32>(n + 4, 0);
    if listed {
        let last = match prev {
            Ok(node) => node,
            Err(op) => g.read::<u32>(w(op)),
        };
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    } else {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(head), n);
    }
    g.write::<u32>(n + 8, n);
    n
}

/// `magic_0568`'s second objects start (0x10001920, 771 bytes; a table
/// entry, no arguments; an `ebp` frame with the state `S = *G_STATE` in
/// its local): `S + 0x16c0` 0; the objects `S + 0x260` and `S + 0x130`
/// started for the host `H`
/// ([`start_host_object`](crate::s4_small::start_host_object), scales
/// `K1`, `K2`, `(2, 0x32)` and `(1, 0)`) at the point `S + 0x1448` with
/// the yaw `S + 0x16bc`, their `y` less `D1` and `D2`; the second's block
/// `S + 0x1688` (two callbacks `+0x20`, `+0x24`, the rest 0) into its
/// `+0x110`, `+0xe0` = -0.7, a fade task of `S + 0x14f0`
/// ([`fade_task`](crate::s3_misc::fade_task), 0x1e frames, the stack
/// word 0); two timer blocks `S + 0x1458` (0, 80, 0, 35, 28, 28) and `S +
/// 0x1474` (0, 80, 40, 102, 76, 76) and the record `S + 0x1490` (scales
/// 1, 1, 0, 4, 4, 0, count 0x66, flags 3, `+0x20` 0.01315, `+0x28` 0x4c)
/// on task nodes chained onto the task list ([`chain_node_0568`]), the
/// list's last the third. Returns the third node.
#[allow(clippy::too_many_lines)] // one straight sequence of steps
pub fn objects_start2_0568<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let s = g.read::<u32>(w(0x11));
    // push ebp; push ecx (the local S); push ebx, esi, edi; sub esp, 8;
    // two pushes: the return address at E - 0x28
    let esp = fr.esp.wrapping_sub(0x28);
    g.write::<u32>(fr.esp.wrapping_sub(8), s);
    g.write::<u32>(s + 0x16c0, 0);
    let start = |g: &mut G, call: u32, host: u32, obj: u32, scale: u32, args: [u32; 2]| {
        g.write::<u32>(esp + 4, args[0]);
        g.write::<u32>(esp + 8, args[1]);
        let mut xmm = fr.xmm;
        xmm[3] = u128::from(g.read::<u32>(w(scale)));
        let hf = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: w(host),
            edx: obj,
            xmm,
            ..*fr
        };
        crate::s4_small::start_host_object(g, img, &hf);
    };
    let o1 = s + 0x260;
    start(g, 0x37, 0x25, o1, 0x8, [2, 0x32]);
    for o in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(s + 0x1448 + o);
        g.write::<u32>(o1 + 0xd0 + o, v);
    }
    let y = sub(g.read::<f32>(o1 + 0xd4), k(g, 0x7f));
    let yaw = g.read::<u32>(s + 0x16bc);
    g.write::<u32>(o1 + 0xc4, yaw);
    g.write::<f32>(o1 + 0xd4, y);
    let o2 = s + 0x130;
    start(g, 0xac, 0xa7, o2, 0x45, [1, 0]);
    for o in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(s + 0x1448 + o);
        g.write::<u32>(o2 + 0xd0 + o, v);
    }
    let b = s + 0x1688;
    for o in [0x18u32, 0x14, 0x10, 0x1c] {
        g.write::<u32>(b + o, 0);
    }
    g.write::<u16>(b + 0x30, 0);
    g.write::<u32>(b + 0x20, w(0x11b));
    g.write::<u32>(b + 0x24, w(0x122));
    g.write::<u32>(b + 0x28, 0);
    g.write::<u32>(b + 0x2c, 0);
    g.write::<u32>(o2 + 0x110, b);
    let y = sub(g.read::<f32>(o2 + 0xd4), k(g, 0xc6));
    let yaw = g.read::<u32>(s + 0x16bc);
    g.write::<u32>(o2 + 0xc4, yaw);
    g.write::<f32>(o2 + 0xd4, y);
    g.write::<u32>(o2 + 0xe0, 0xbf33_3333);
    // push ecx (0) over the second start's words: the return address at
    // E - 0x2c
    let fesp = fr.esp.wrapping_sub(0x2c);
    g.write::<u32>(fesp + 4, 0);
    let ff = Frame {
        at: img.rel32_target(f + 0x171),
        esp: fesp,
        ecx: s + 0x14f0,
        edx: 0x1e,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0xe6))),
            u128::from(g.read::<u32>(w(0xc6))),
            0,
        ],
        ..Frame::default()
    };
    crate::s3_misc::fade_task(g, img, &ff);
    let t1 = s + 0x1458;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0),
        (0xc, 0x23),
        (0x14, 0x1c),
        (0x10, 0x1c),
    ] {
        g.write::<u32>(t1 + o, v);
    }
    g.write::<u16>(t1 + 0x18, 0);
    let n1 = chain_node_0568(g, img, f, (0x1b3, 0x181, 0x1bf, 0x1e5), Err(0x1d2), t1);
    let t2 = s + 0x1474;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0x28),
        (0xc, 0x66),
        (0x14, 0x4c),
        (0x10, 0x4c),
    ] {
        g.write::<u32>(t2 + o, v);
    }
    g.write::<u16>(t2 + 0x18, 0);
    let n2 = chain_node_0568(g, img, f, (0x227, 0x1f5, 0x22d, 0x253), Ok(n1), t2);
    let r = s + 0x1490;
    for (o, v) in [
        (0u32, 0u32),
        (0x1c, 0),
        (0x20, 0x3c57_9436),
        (0x24, 0),
        (0x28, 0x4c),
    ] {
        g.write::<u32>(r + o, v);
    }
    g.write::<u16>(r + 0x2c, 0);
    let n3 = chain_node_0568(g, img, f, (0x291, 0x266, 0x297, 0x2ba), Ok(n2), r);
    for (o, v) in [
        (4u32, ONE),
        (8, ONE),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4080_0000),
        (0x18, 0),
    ] {
        g.write::<u32>(r + o, v);
    }
    let fl = g.read::<u16>(r + 0x2c);
    g.write::<u16>(r + 0x2c, fl | 3);
    g.write::<u32>(r + 0x24, 0x66);
    g.write::<u32>(w(0x2fa), n3);
    Ret::eax(n3)
}

/// [`objects_start2_0568`] (771 bytes, 1 overlay).
pub const OBJECTS_START2_0568: Sig = Sig {
    len: 771,
    hash: 0xc005_cf39_41df_94f2,
    calls: &[
        (0x37, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
        (0xac, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
        (0x171, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x1b3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x227, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x291, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0612`'s placement (0x100073d0, 779 bytes; internal, no
/// arguments, called by [`start_up_0612`]; an 8-aligned frame: `B =
/// ((E - 4) & !7) - 0x80`, `P = B - 8` after two pushes): with the state
/// `S = *G_STATE`, the caster `*G_A`'s position (slot 262 into `P +
/// 0x10..0x18`) into `S + 0x2b20`, `w` 1.0; the scene point `S + 0x2b30`
/// ([`scene_point`](crate::s6_misc::scene_point)) picks the character
/// (none: `*G_A`, else `G_CHRS[(*G_SEL)[i]]`) whose slot 12 value goes
/// into `S + 0x2b34`; `ang = A - atan2(S[0x2b38] - z, S[0x2b30] - x)`
/// (`_CIatan2` on doubles, rounded once; wrapped once); the frame `S +
/// 0x2b00` (`0, ang, 0`, the stale word, the position); the part shown
/// ([`part_toggle`](crate::s4_misc::part_toggle) of `S + 0x2ac0`) and
/// its matrix scaled and turned by `(0, ang, 0)`
/// ([`scale_rotate`](crate::sa_misc::scale_rotate)); `S + 0x3ba4` = `ang`;
/// the camera point `S + 0x2b50` = the position moved by `(0, 0, -60)`
/// turned by X 0 and Y `ang` (slots 235, 328, 329 into `P + 0x40`, slot
/// 229 into `P + 0x20`, slot 248 adds it), `w` 1.0; `S + 0x3ba0`, `+0x3ba8`
/// 0, a flag set; the scene light
/// ([`scene_light`](crate::s5_misc::scene_light)) and the part centres
/// ([`part_centres`](crate::s6_misc::part_centres)). Returns the last
/// call's `eax`.
pub fn place_0612<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    place_with_0612(g, img, f, esp, true)
}

/// [`place_0612`] with the scene light or without it (`magic_0563`'s
/// copy, 0x100014f0, calls only the part centres).
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub(crate) fn place_with_0612<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    light: bool,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let p = (esp.wrapping_sub(4) & !7).wrapping_sub(0x88);
    let s = g.read::<u32>(w(0x1b));
    let (px, py, pz, pw) = (p + 0x10, p + 0x14, p + 0x18, p + 0x1c);
    let chr = g.read::<u32>(w(0x35));
    g.call(t(g, 0x2f, 262), 0, &[chr, px, py, pz]);
    for (off, src) in [(0x2b20u32, px), (0x2b24, py), (0x2b28, pz)] {
        let v = g.read::<u32>(src);
        g.write::<u32>(s + off, v);
    }
    g.write::<u32>(pw, ONE);
    g.write::<u32>(s + 0x2b2c, ONE);
    // the return address at P - 4
    let cf = Frame {
        at: img.rel32_target(f + 0x87),
        esp: p.wrapping_sub(4),
        ecx: s + 0x2b30,
        ..Frame::default()
    };
    let i = crate::s6_misc::scene_point(g, img, &cf);
    let who = if i == 0xffff_ffff {
        g.read::<u32>(w(0x93))
    } else {
        let sel = g.read::<u32>(w(0x9a));
        let c = u32::from(g.read::<u8>(sel.wrapping_add(i)));
        g.read::<u32>(w(0xa5).wrapping_add(4 * c))
    };
    let h = st0_f32(g.call(t(g, 0xab, 12), 0, &[who]).st0);
    g.write::<f32>(s + 0x2b34, h);
    let dz = sub(g.read::<f32>(s + 0x2b38), g.read::<f32>(pz));
    let dx = sub(g.read::<f32>(s + 0x2b30), g.read::<f32>(px));
    // `_CIatan2` on x87 (the two differences as doubles), rounded once
    #[allow(clippy::cast_possible_truncation)] // fstp qword; cvtpd2ps
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let ang = wrap_once(g, img, f, sub(k(g, 0x106), a), [0x115, 0x125, 0x12f, 0x13c]);
    // junk read: [P+0x2c] (the turn vector's `w`) never written; 0 (brief
    // 2026-10-24)
    g.write::<u32>(s + 0x2b0c, 0);
    g.write::<u32>(s + 0x2b00, 0);
    g.write::<f32>(s + 0x2b04, ang);
    g.write::<u32>(s + 0x2b08, 0);
    for (i, src) in (0u32..).zip([px, py, pz, pw]) {
        let v = g.read::<u32>(src);
        g.write::<u32>(s + 0x2b10 + 4 * i, v);
    }
    let v = p + 0x20;
    g.write::<f32>(v + 4, ang);
    g.write::<u32>(v + 8, 0);
    g.write::<u32>(v, 0);
    let m = s + 0x2ac0;
    let pf = Frame {
        at: img.rel32_target(f + 0x1c5),
        esp: p.wrapping_sub(4),
        edx: m,
        ..Frame::default()
    };
    crate::s4_misc::part_toggle(g, img, &pf);
    crate::sa_misc::scale_rotate(g, img, img.rel32_target(f + 0x1d5), m, v);
    g.write::<f32>(s + 0x3ba4, ang);
    let cam = s + 0x2b50;
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x2b20 + 4 * i);
        g.write::<u32>(cam + 4 * i, x);
    }
    let (mat, d) = (p + 0x40, p + 0x30);
    for (i, x) in (0u32..).zip([0u32, 0, 0xc270_0000, ONE]) {
        g.write::<u32>(d + 4 * i, x);
    }
    g.call(t(g, 0x223, 235), 0, &[mat]);
    g.call(t(g, 0x260, 328), 0, &[mat, mat, 0]);
    g.call(t(g, 0x281, 329), 0, &[mat, mat, ang.to_bits()]);
    g.write::<u32>(d + 0xc, ONE);
    g.call(t(g, 0x29d, 229), 0, &[v, mat, d]);
    g.call(t(g, 0x2b7, 248), 0, &[cam, cam, v]);
    g.write::<u32>(cam + 0xc, ONE);
    g.write::<u32>(s + 0x3ba0, 0);
    g.write::<u32>(s + 0x3ba8, 0);
    g.write::<u32>(w(0x2e5), 1);
    let lf = Frame {
        at: img.rel32_target(f + 0x2ee),
        esp: p.wrapping_sub(4),
        ..Frame::default()
    };
    if !light {
        // (the part centres where the scene light is)
        return crate::s6_misc::part_centres(g, img, &lf).eax;
    }
    crate::s5_misc::scene_light(g, img, &lf);
    let cf = Frame {
        at: img.rel32_target(f + 0x2f3),
        ..lf
    };
    crate::s6_misc::part_centres(g, img, &cf).eax
}

/// [`place_0612`] (779 bytes, 1 overlay).
pub const PLACE_0612: Sig = Sig {
    len: 779,
    hash: 0x414d_fed5_e393_8967,
    calls: &[
        (0x87, Callee::Sig(&crate::s6_misc::SCENE_POINT)),
        (0xf4, Callee::Import("_CIatan2")),
        (0x1c5, Callee::Sig(&crate::s4_misc::PART_TOGGLE)),
        (0x1d5, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0x2ee, Callee::Sig(&crate::s5_misc::SCENE_LIGHT)),
        (0x2f3, Callee::Sig(&crate::s6_misc::PART_CENTRES)),
        // __security_check_cookie
        (0x303, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0612`'s start-up (0x1000a120, 397 bytes; no arguments, called
/// by its load step): slot 718 `(P0)`; four globals from immediates (the
/// state among them); slot 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`part_setup`](crate::sb_misc::part_setup) `(BUF)`), the placement
/// ([`place_0612`]), the effect reset and scaled
/// ([`reset_and_scale`](crate::s4_small::reset_and_scale)); slot 271
/// `(4)`; the two script blocks' callbacks, words, halfwords and bytes;
/// two task nodes (kinds 1 and 2) appended to the task list, the records
/// set up ([`records_init`](crate::sa_misc::records_init)) between them;
/// when slot 520 answers 0x24a0000, `G_X` = `G_Y` = `G_X - K`. Returns 0.
pub fn start_up_0612<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_with_0612(g, img, fr, place_0612)
}

/// [`start_up_0612`] with the DLL's own placement given (`magic_0563`'s
/// copy places without the part centres).
pub(crate) fn start_up_with_0612<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    place: fn(&mut G, &Image, u32, u32) -> u32,
) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    g.call(t(g, 0x1, 718), 0, &[w(0x6)]);
    g.write::<u32>(w(0x28), 0);
    for o in [0x30u32, 0x3a, 0x44, 0x4e] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    // call width: 4 (slot 718's word under it)
    let h = g.call(t(g, 0x13, 719), 0, &[w(0x22), 8, 0, w(0x6)]).eax;
    g.write::<u32>(w(0x5e), h);
    // push BUF over slot 718's word: the return address at E - 0xc
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, w(0x6));
    g.write::<u32>(esp + 4, w(0x63));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x68), esp);
    place(g, img, img.rel32_target(f + 0x6d), esp);
    let rf = Frame {
        at: img.rel32_target(f + 0x72),
        esp,
        ..Frame::default()
    };
    crate::s4_small::reset_and_scale(g, img, &rf);
    // call width: 3 (BUF and slot 718's word under it)
    g.call(t(g, 0x77, 271), 0, &[4, w(0x63), w(0x6)]);
    g.write::<u32>(w(0x8f), w(0x93));
    g.write::<u32>(w(0x99), 0);
    g.write::<u16>(w(0xa4), 0);
    g.write::<u8>(w(0xac), 1);
    g.write::<u32>(w(0xb3), w(0xb7));
    g.write::<u32>(w(0xbd), 0);
    g.write::<u16>(w(0xc8), 0);
    g.write::<u8>(w(0xd0), 1);
    for (call, cb, kind, ops, then) in [
        (
            0xd6u32,
            0x89u32,
            1u8,
            [0xdcu32, 0xf0, 0x10f, 0x102, 0x10f],
            true,
        ),
        (0x11e, 0x119, 2, [0x124, 0x138, 0x152, 0x14a, 0x152], false),
    ] {
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
        g.write::<u8>(node.wrapping_add(0x18), kind);
        g.write::<u32>(node.wrapping_add(4), 0);
        crate::r12h_start::append(g, img, f, node, ops);
        if then {
            crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x114), w(0x107));
        }
    }
    if g.call(t(g, 0x157, 520), 0, &[]).eax == 0x024a_0000 {
        let x = sub(g.read::<f32>(w(0x16e)), g.read::<f32>(w(0x176)));
        g.write::<f32>(w(0x17e), x);
        g.write::<f32>(w(0x186), x);
    }
    Ret::eax(0)
}

/// [`start_up_0612`] (397 bytes, 1 overlay).
pub const START_UP_0612: Sig = Sig {
    len: 397,
    hash: 0x9ec0_3b7e_8844_b09c,
    calls: &[
        (0x68, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x6d, Callee::Sig(&PLACE_0612)),
        (0x72, Callee::Sig(&crate::s4_small::RESET_AND_SCALE_563)),
        (0xd6, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x114, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0x11e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0612`'s load step (0x100071d0, the 118-byte shape) running
/// [`start_up_0612`].
pub fn load_0612<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0612)
}

/// [`load_0612`].
pub const LOAD_0612: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0612))],
    imports: &[],
};

/// `magic_0603`'s summon object set-up (0x1000c5c0, 514 bytes; internal,
/// no arguments; an `ebp` frame with a two-word group at `ebp - 8`): the
/// DLL's object `O` joins the DLL's ring at its end (vtable, step, `+0x14`
/// 0, the ring's flag bit 0 cleared, `O + 0x50` 0); the node `N` joins
/// the static list (pass byte 0xff, an empty child list, after the list's
/// last: the list's first when it was empty, the count up), `N + 0x1c` 1,
/// a flag 1; the colour node `C` (vtable set) over the group `(C, P)`
/// ([`colour_node`](crate::s3_misc::colour_node)); then its fades at 0.5
/// (`C + 0xac` bit 0 cleared, `K - x` steps and the halves). Returns the
/// colour node's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of stores
pub fn summon_setup_0603<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    const HALF: u32 = 0x3f00_0000;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let set = |g: &mut G, dst: u32, src: u32| g.write::<u32>(w(dst), w(src));
    // the ring: the old last, the object's words
    let last = g.read::<u32>(w(0x4));
    g.write::<u32>(w(0x9), last);
    set(g, 0xf, 0x13);
    set(g, 0x19, 0x1d);
    g.write::<u32>(w(0x23), 0);
    set(g, 0x2d, 0x31);
    g.write::<u32>(last + 4, w(0x38));
    let list_last = g.read::<u32>(w(0x3d));
    let fl = g.read::<u32>(w(0x43));
    g.write::<u32>(w(0x43), fl & !1);
    set(g, 0x4d, 0x51);
    g.write::<u32>(w(0x57), 0);
    // the static node
    set(g, 0x61, 0x65);
    g.write::<u16>(w(0x6c), 0xff);
    set(g, 0x74, 0x78);
    set(g, 0x7e, 0x82);
    set(g, 0x88, 0x8c);
    set(g, 0x92, 0x96);
    g.write::<u32>(w(0x9b), list_last);
    if list_last == w(0xa0) {
        set(g, 0xa8, 0xac);
    } else {
        g.write::<u32>(list_last + 4, w(0xb5));
    }
    let n = g.read::<u16>(w(0xbc)).wrapping_add(1);
    g.write::<u16>(w(0xbc), n);
    set(g, 0xc5, 0xc9);
    g.write::<u8>(w(0xcf), 1);
    g.write::<u32>(w(0xd6), 1);
    let ebp = esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(8);
    g.write::<u32>(group, w(0xe1));
    g.write::<u32>(group + 4, w(0xe8));
    set(g, 0xee, 0xf2);
    let cf = Frame {
        at: img.rel32_target(f + 0xf7),
        esp: ebp.wrapping_sub(0xc),
        ecx: group,
        ..Frame::default()
    };
    let eax = crate::s3_misc::colour_node(g, img, &cf);
    // `and [C + 0xac], ~1`: the colour node has just set that word to 1
    let fl = g.read::<u32>(w(0x110));
    g.write::<u32>(w(0x110), fl & !1);
    let c = k(g, 0xff);
    for dst in [0x117u32, 0x121, 0x12b, 0x148, 0x152, 0x15c, 0x166] {
        g.write::<u32>(w(dst), HALF);
    }
    g.write::<u32>(w(0x183), ONE);
    g.write::<u32>(w(0x18d), 0);
    for (dst, src) in [
        (0x137u32, 0x10au32),
        (0x172, 0x142),
        (0x199, 0x17d),
        (0x1ac, 0x1a4),
        (0x1bf, 0x1b7),
        (0x1da, 0x1ca),
        (0x1f2, 0x1d2),
    ] {
        let v = sub(c, k(g, src));
        g.write::<f32>(w(dst), v);
    }
    let v = sub(k(g, 0x1e2), k(g, 0x1ea));
    g.write::<f32>(w(0x1fa), v);
    eax
}

/// [`summon_setup_0603`] (514 bytes, 1 overlay).
pub const SUMMON_SETUP_0603: Sig = Sig {
    len: 514,
    hash: 0x9381_4dfc_f5f5_fe8c,
    calls: &[(0xf7, Callee::Sig(&crate::s3_misc::COLOUR_NODE))],
    imports: &[],
};

/// `magic_0573`'s summon object set-up (0x10007b70, 654 bytes; internal:
/// `ecx` = the object `O`, `edx` = the summon `S`, the stack `(A, B)`; a
/// cookie frame, `ebp = E - 4`). The object joins the DLL's ring
/// ([`ring_join`](crate::h6_setup::ring_join)); its node `O + 0x5c` and
/// the bone `O + 0xe0` join the static list
/// ([`static_child`](crate::h6_setup::static_child)); `S`'s model state
/// into `O + 0x78` ([`model_state`](crate::m0070::model_state)); `O +
/// 0x104`, `+0xfc` 0, `O + 0x100` = `(K - y) K1 / K2` truncated (`y` slot
/// 333 of `S[0x54]` with bit 0 of `S + 0x90`, else slot 601 `(S[0x50],
/// &y)`); the particle node `O + 0x10c`
/// ([`node_setup`](crate::s5_misc::node_setup), the group `(node, LIST,
/// G, O + 0xe0, O + 0x5c, S)`); `O + 0x254` 1; slot 210 `(A[0x54], M)`
/// into a local matrix, slot 229 `(O + 0x29c, M, (500, -2000, -16200,
/// 1))`, `O + 0x2a0` += `K3`; slot 211 `(S[0x54], O + 0x1a8)`; `S + 0x90`
/// bit 4 set, `S + 0x4c` = the node; `O + 0x38` = `B`, `+0x30` = `S`,
/// `+0x34` = `A`, `+0x28`, `+0x1c`, `+0x18` 0. Returns `A`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0573<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::h6_setup::{ring_join, static_child};
    use crate::sb_sse::{add, div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (obj, s) = (fr.ecx, fr.edx);
    let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    g.write::<u32>(l(0x64), a);
    ring_join(g, img, f, obj, [0x26, 0x2d, 0x35, 0x3f, 0x44, 0x50, 0x57]);
    let c = obj + 0x5c;
    static_child(
        g,
        img,
        f,
        c,
        [0x6a, 0x71, 0x76, 0x7e, 0x88, 0x90, 0x9c, 0xa2],
    );
    g.write::<u32>(c, w(0xa8));
    g.write::<u32>(l(0x68), s);
    let me = l(0x60);
    g.write::<u32>(me, s);
    g.write::<u32>(me + 4, obj + 0x78);
    crate::m0070::model_state(g, img, img.rel32_target(f + 0xb9), me);
    let bone = obj + 0xe0;
    g.write::<u32>(bone, w(0xc5));
    static_child(
        g,
        img,
        f,
        bone,
        [0xd8, 0xdf, 0xe4, 0xec, 0xf1, 0xf9, 0x105, 0x10b],
    );
    g.write::<u32>(obj + 0x104, 0);
    g.write::<u32>(obj + 0xfc, 0);
    let y = l(0x5c);
    let v = if g.read::<u8>(s + 0x90) & 1 != 0 {
        let r = st0_f32(g.call(t(g, 0x12d, 333), 0, &[g.read::<u32>(s + 0x54)]).st0);
        g.write::<f32>(y, r);
        sub(k(g, 0x143), r)
    } else {
        // call width: 2
        g.call(t(g, 0x156, 601), 0, &[g.read::<u32>(s + 0x50), y]);
        sub(k(g, 0x169), g.read::<f32>(y))
    };
    let v = div(mul(v, k(g, 0x179)), k(g, 0x18a));
    g.write::<u32>(obj + 0x100, crate::s4_summon::trunc(v));
    let node = obj + 0x10c;
    g.write::<u32>(l(0x74), bone);
    let group = l(0x80);
    for (i, x) in (0u32..).zip([node, w(0x1a7), w(0x1ae), bone, c, s]) {
        g.write::<u32>(group + 4 * i, x);
    }
    g.write::<u32>(node, w(0x1ba));
    let nf = Frame {
        at: img.rel32_target(f + 0x1bf),
        esp: fr.esp.wrapping_sub(0x98),
        ecx: group,
        ..Frame::default()
    };
    crate::s5_misc::node_setup(g, img, &nf);
    g.write::<u8>(obj + 0x254, 1);
    let m = l(0x58);
    // call width: 2
    g.call(t(g, 0x1d5, 210), 0, &[g.read::<u32>(a + 0x54), m]);
    let p = l(0x14);
    for (i, x) in (0u32..).zip([0x43fa_0000u32, 0xc4fa_0000, 0xc67d_2001, ONE]) {
        g.write::<u32>(p + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x1f1, 229), 0, &[obj + 0x29c, m, p]);
    let x = add(g.read::<f32>(obj + 0x2a0), k(g, 0x225));
    g.write::<f32>(obj + 0x2a0, x);
    // call width: 2
    g.call(t(g, 0x234, 211), 0, &[g.read::<u32>(s + 0x54), node + 0x9c]);
    let fl = g.read::<u16>(s + 0x90);
    g.write::<u16>(s + 0x90, fl | 0x10);
    g.write::<u32>(s + 0x4c, node);
    g.write::<u32>(obj + 0x38, b);
    g.write::<u32>(obj + 0x30, s);
    g.write::<u32>(obj + 0x28, 0);
    g.write::<u32>(obj + 0x34, a);
    g.write::<u32>(obj + 0x1c, 0);
    g.write::<u32>(obj + 0x18, 0);
    a
}

/// [`summon_setup_0573`] (654 bytes, 1 overlay).
pub const SUMMON_SETUP_0573: Sig = Sig {
    len: 654,
    hash: 0xb849_94a8_d0e9_dc89,
    calls: &[
        (0xb9, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x1bf, Callee::Sig(&crate::s5_misc::NODE_SETUP)),
        // __security_check_cookie
        (0x286, Callee::Any),
    ],
    imports: &[],
};

/// A ring walked with a cursor in the caller's frame (`cursor`: the
/// object, byte `+5` the pass): the object's handler `[[obj] + 0x1c]`,
/// cdecl `(cursor)`, `ecx` the cursor, with `under` the words the
/// original's frame already holds above it. Returns those words with the
/// cursor pushed.
fn visit_with<G: Game>(
    g: &mut G,
    cursor: u32,
    obj: u32,
    pass: u8,
    mut under: Vec<u32>,
) -> Vec<u32> {
    g.write::<u32>(cursor, obj);
    g.write::<u8>(cursor + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
    under.insert(0, cursor);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(handler, cursor, &under);
    under
}

/// `magic_0573`'s bone update (0x100037a0, 1016 bytes; internal, `ecx` =
/// the cursor, called by [`thunk_bone_0573`] and the DLL's own code; a
/// cookie frame, `ebp = E - 4`: cursors at `ebp - 0xb4`, `- 0xbc`, `-
/// 0xa4`, the matrix `M` at `ebp - 0x98`, the vector `V` at `ebp - 0x14`,
/// the matrix `L` at `ebp - 0x58`). The node `n`, its parent `p = n +
/// 0x128` (read first). The walk (its `eax` the cookie `__security_cookie
/// ^ ebp`); when the cursor moved: the rings of `n + 0x140`, `+0x144` and
/// the parent (when there is one) visited; the DLL's function table (by
/// `n + 0x148`, `ecx` the index) turns `M` towards the target `n[0x144] +
/// 0x1c`; slot 231 `(n + 0x5c, n + 0x150, M)`; slot 233 interpolates `V`
/// towards the `+0x140` object by its rate times `n + 0x1a4`, and the
/// node's three vectors by `V`; slot 325 `(n + 0x4c, n + 0x190)`; slots
/// 668, 231, 231, 229 compose the node's matrices with the parent's; `n +
/// 0xcc..` += the parent's. With the byte `n + 0x149` set: `n + 0x1a4` =
/// `n + 0x1a8`, the new parent `q = n + 0x14c` visited, slot 327 `(M, q +
/// 0x9c)`, `L` = slot 231 `(L, p + 0xdc, n + 0x150)` then `(L, L, M)`, its
/// rows normalised into `n + 0x150..` (slot 232), slot 229 `(n + 0x190, M,
/// n + 0xcc)`, slot 233 `(n + 0x190, n + 0x190, n + 0x1a4)`, `n + 0x19c`
/// 1.0, the parent `q` and the byte 0. Then the node's object shown
/// ([`show`](crate::m0573::show) of `n + 0x1ac`). Returns its `eax`, or
/// the walk's.
pub fn bone_0573<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    bone_with(g, img, f, esp, this, true)
}

/// [`bone_0573`]'s steps, with the object shown at the end (`show`) or not
/// (`magic_0577`'s copy, [`crate::r27::bone_0577`]: it returns the last
/// slot call's `eax`, or the new parent).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub(crate) fn bone_with<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    this: u32,
    show: bool,
) -> u32 {
    use crate::sb_sse::{add, mul};
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let n = g.read::<u32>(this);
    let p = g.read::<u32>(n + 0x128);
    g.write::<u32>(l(0xac), this);
    g.write::<u32>(l(0xa8), n);
    g.write::<u32>(l(0x9c), p);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    let (eax, moved) = crate::nodes::walk(g, this, this, cookie);
    if moved.is_none() {
        return eax;
    }
    let pass = |g: &G| g.read::<u8>(this + 5);
    let mut under = Vec::new();
    for (cur, off) in [(l(0xb4), 0x140u32), (l(0xbc), 0x144)] {
        let obj = g.read::<u32>(n + off);
        let ps = pass(g);
        under = visit_with(g, cur, obj, ps, under);
    }
    if p != 0 {
        let ps = pass(g);
        visit_with(g, l(0xa4), p, ps, Vec::new());
    }
    let m = l(0x98);
    let target = g.read::<u32>(n + 0x144).wrapping_add(0x1c);
    let which = g.read::<u8>(n + 0x148);
    let func = g.read::<u32>(w(0x103).wrapping_add(4 * u32::from(which)));
    g.call(func, u32::from(which), &[m, target]);
    // call width: 5 (the table call's words stay on the stack)
    g.call(t(g, 0x118, 231), 0, &[n + 0x5c, n + 0x150, m, m, target]);
    let src = g.read::<u32>(n + 0x140);
    let rate = mul(g.read::<f32>(src + 0x28), g.read::<f32>(n + 0x1a4));
    let v = l(0x14);
    g.call(t(g, 0x14c, 233), 0, &[v, src + 0x1c, rate.to_bits()]);
    for (i, (a, b, op)) in [
        (0x1cu32, 0x5cu32, 0x15eu32),
        (0x2c, 0x6c, 0x18d),
        (0x3c, 0x7c, 0x1af),
    ]
    .into_iter()
    .enumerate()
    {
        let x = g.read::<u32>(v + 4 * i as u32);
        g.call(t(g, op, 233), 0, &[n + a, n + b, x]);
    }
    let x = g.read::<u32>(v + 8);
    // (the last slot 233's words stay under it)
    let mut st = vec![n + 0x4c, n + 0x190, n + 0x3c, n + 0x7c, x];
    g.call(t(g, 0x1c3, 325), 0, &st);
    let src = g.read::<u32>(n + 0x140).wrapping_add(0x1c);
    let mut eax = 0;
    for (words, op, slot) in [
        ([n + 0x12c, p + 0x12c, src], 0x1f7u32, 668u32),
        ([n + 0xdc, p + 0xdc, n + 0x5c], 0x21e, 231),
        ([n + 0x9c, p + 0x9c, n + 0x1c], 0x240, 231),
        ([n + 0xcc, p + 0xdc, n + 0x4c], 0x255, 229),
    ] {
        st.splice(0..0, words);
        st.truncate(12);
        // call width: 12 (the earlier calls' words stay on the stack)
        eax = g.call(t(g, op, slot), 0, &st).eax;
    }
    let c = n + 0xcc;
    // addss: the first operand's NaN wins, so the order is kept
    let x0 = add(g.read::<f32>(c), g.read::<f32>(p + 0xcc));
    g.write::<f32>(c, x0);
    for i in [4u32, 8] {
        let x = add(g.read::<f32>(p + 0xcc + i), g.read::<f32>(c + i));
        g.write::<f32>(c + i, x);
    }
    if g.read::<u8>(n + 0x149) != 0 {
        let s = g.read::<u32>(n + 0x1a8);
        g.write::<u32>(n + 0x1a4, s);
        let q = g.read::<u32>(n + 0x14c);
        let ps = g.read::<u8>(g.read::<u32>(l(0xac)) + 5);
        let mut st = visit_with(g, l(0xa4), q, ps, Vec::new());
        let lm = l(0x58);
        let q = g.read::<u32>(n + 0x14c);
        for (words, op, slot) in [
            (vec![m, q + 0x9c], 0x2f5u32, 327u32),
            (vec![lm, p + 0xdc, n + 0x150], 0x314, 231),
            (vec![lm, lm, m], 0x32d, 231),
            (vec![n + 0x150, lm], 0x33e, 232),
            (vec![n + 0x160, l(0x48)], 0x357, 232),
            (vec![n + 0x170, l(0x38)], 0x36f, 232),
            (vec![n + 0x190, m, n + 0xcc], 0x38a, 229),
        ] {
            st.splice(0..0, words);
            st.truncate(12);
            // call width: 12 (the earlier calls' words stay on the stack)
            g.call(t(g, op, slot), 0, &st);
        }
        let s = g.read::<u32>(n + 0x1a4);
        g.call(t(g, 0x3a6, 233), 0, &[n + 0x190, n + 0x190, s]);
        g.write::<u32>(n + 0x19c, ONE);
        let q = g.read::<u32>(n + 0x14c);
        g.write::<u32>(n + 0x128, q);
        g.write::<u8>(n + 0x149, 0);
        eax = q;
    }
    if !show {
        return eax;
    }
    let obj = g.read::<u32>(n + 0x1ac);
    crate::m0573::show(g, img, img.rel32_target(f + 0x3e3), obj)
}

/// [`bone_0573`] (1,016 bytes, 1 overlay).
pub const BONE_0573: Sig = Sig {
    len: 1016,
    hash: 0xdcf7_d8a7_6df0_eaba,
    calls: &[
        (0x3e3, Callee::Sig(&crate::m0573::SHOW)),
        // __security_check_cookie
        (0x3f0, Callee::Any),
    ],
    imports: &[],
};

/// The thunk over [`bone_0573`] (0x10003ba0, 15 bytes; a vtable's `+0x1c`
/// method, cdecl `(cursor)`; `push ecx` then the call: the target's
/// return address at `E - 0xc`).
pub fn thunk_bone_0573<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let cursor = fr.arg(g, 0);
    let esp = fr.esp.wrapping_sub(0xc);
    // push ebp; push ecx: the entry's `ecx` in the thunk's frame
    g.write::<u32>(fr.esp.wrapping_sub(8), fr.ecx);
    Ret::eax(bone_0573(g, img, img.rel32_target(fr.at + 8), esp, cursor))
}

/// [`thunk_bone_0573`] (15 bytes, 1 overlay).
pub const THUNK_BONE_0573: Sig = Sig {
    len: 15,
    hash: 0xd192_63e1_ae73_f895,
    calls: &[(8, Callee::Sig(&BONE_0573))],
    imports: &[],
};

/// `magic_0613`'s motion task (0x10009310, 482 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame with the
/// model-id pair at `A = ((E - 4) & !7) - 8`). By the state `R + 0x18`
/// (a state set here runs at once): 10 and 0x32 step on; 11 and 31 wait
/// for the object `O = R + 0x1c`'s motion (slot 20 of `O[0x54]` with bit
/// 0 of `O + 0x90`, else slot 411 of `O[0x50]`) and go to 20 and 40; 20,
/// 30, 40 and 60 give `O` the values 0xb9, 0xba, 0xbb, 0xbd
/// ([`set_value`](crate::m0042::set_value)) and step on; 61 waits for
/// `O`'s model id ([`model_id`](crate::sa_misc::model_id)), gives it 0xbc
/// and steps on with `R + 0x30` 1; 10000 unlinks the task node. Every way
/// ends with `R + 0x2c` and `R + 0x30` up (a step also the state).
/// Returns the last `eax`.
pub fn motion_task_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let bump = |g: &mut G, o: u32| {
        let v = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, v);
    };
    // the end of every way: the counts up (a step: the state too)
    let end = |g: &mut G, step: bool, eax: u32| {
        if step {
            bump(g, 0x18);
        }
        bump(g, 0x2c);
        bump(g, 0x30);
        Ret::eax(eax)
    };
    let set = |g: &mut G, call: u32, v: u32| {
        let obj = g.read::<u32>(r + 0x1c);
        crate::m0042::set_value(g, img, img.rel32_target(f + call), obj, v)
    };
    // waits for the object's motion: its answer
    let motion = |g: &mut G, o20: u32, o411: u32| {
        let obj = g.read::<u32>(r + 0x1c);
        if g.read::<u8>(obj + 0x90) & 1 != 0 {
            g.call(t(g, o20, 20), 0, &[g.read::<u32>(obj + 0x54)]).eax
        } else {
            g.call(t(g, o411, 411), 0, &[g.read::<u32>(obj + 0x50)]).eax
        }
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            10 => return end(g, true, 0),
            0x32 => return end(g, true, 0x32),
            11 | 31 => {
                let (o20, o411, next) = if state == 11 {
                    (0x50u32, 0x5du32, 0x14u32)
                } else {
                    (0x8d, 0x9a, 0x28)
                };
                let eax = motion(g, o20, o411);
                if eax == 0 {
                    return end(g, false, 0);
                }
                g.write::<u32>(r + 0x18, next);
            }
            20 => {
                let eax = set(g, 0x102, 0xb9);
                return end(g, true, eax);
            }
            30 => {
                let eax = set(g, 0x11e, 0xba);
                return end(g, true, eax);
            }
            0x28 => {
                let eax = set(g, 0x13a, 0xbb);
                return end(g, true, eax);
            }
            0x3c => {
                let eax = set(g, 0xe6, 0xbd);
                return end(g, true, eax);
            }
            0x3d => {
                let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
                let obj = g.read::<u32>(r + 0x1c);
                g.write::<u32>(a, obj);
                let id = crate::sa_misc::model_id(g, img, img.rel32_target(f + 0x159), a);
                if id == 0 {
                    return end(g, false, 0);
                }
                let eax = set(g, 0x16d, 0xbc);
                bump(g, 0x18);
                bump(g, 0x2c);
                g.write::<u32>(r + 0x30, 1);
                return Ret::eax(eax);
            }
            0x2710 => {
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return end(g, false, prev);
            }
            // `sub eax, 10` and the byte table: another state below 0x28
            // leaves its table index (5) or the state less 10
            s if (10..0x20).contains(&s) => return end(g, false, 5),
            s if s < 0x28 => return end(g, false, s.wrapping_sub(10)),
            s => return end(g, false, s),
        }
    }
}

/// [`motion_task_0613`] (482 bytes, 1 overlay).
pub const MOTION_TASK_0613: Sig = Sig {
    len: 482,
    hash: 0x47bc_946b_6d14_6b91,
    calls: &[
        (0xe6, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x102, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x11e, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x13a, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x159, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x16d, Callee::Sig(&crate::m0042::SET_VALUE)),
    ],
    imports: &[],
};

/// The `mov dword [G], imm32` stores (10 bytes each) at the instruction
/// offsets `at`: `G` (`+2`) = the immediate (`+6`).
pub(crate) fn stores_0613<G: Game>(g: &mut G, img: &Image, f: u32, at: &[u32]) {
    for &o in at {
        g.write::<u32>(img.word(f + o + 2), img.word(f + o + 6));
    }
}

/// `magic_0613`'s start-up (0x1000b860, 704 bytes; cdecl `(actor)`; an
/// 8-aligned frame): the targets of the actor
/// ([`targets_init_c`](crate::s4_summon::targets_init_c)); the DLL's two
/// objects and four static nodes initialised (vtables, links to
/// themselves, the pass halfwords 0x1ff/0xff); slot 718 `(P)`, slot 719
/// `(P, 8, 0)` into `G_H`, `pppInitData(h, BUF, 0)` (slot 711, the game
/// lock `**G_LOCK` 1 around it), slot 710 `(OUT, *G_H, DST, 0x100000)`,
/// slots 222 and 221 `(P, 0, 1000, 2)` (the earlier calls' words under
/// them); the first object joins the second's ring and the second the
/// first's (vtable, step, `+0x14` itself, the ring's last and sentinel,
/// the flag bit 0 cleared), their `+0x28`/`+0x18`, `+0x34` 0 and `+0x18`
/// 0x19a. Returns 0.
#[allow(clippy::too_many_lines)] // one straight sequence of stores
pub fn start_up_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    start_up_head_0613(g, img, fr);
    // the first object joins the second's ring
    let last = g.read::<u32>(w(0x1fe));
    g.write::<u32>(w(0x203), last);
    stores_0613(g, img, f, &[0x207, 0x211, 0x21b, 0x225]);
    g.write::<u32>(last + 4, w(0x232));
    let last = g.read::<u32>(w(0x237));
    let fl = g.read::<u32>(w(0x23d));
    g.write::<u32>(w(0x23d), fl & !1);
    g.write::<u32>(w(0x243), last);
    stores_0613(
        g,
        img,
        f,
        &[0x247, 0x251, 0x25b, 0x265, 0x26f, 0x279, 0x283],
    );
    // and the second the first's
    g.write::<u32>(last + 4, w(0x290));
    let fl = g.read::<u32>(w(0x296));
    g.write::<u32>(w(0x296), fl & !1);
    stores_0613(g, img, f, &[0x29b, 0x2a5, 0x2af]);
    Ret::eax(0)
}

/// [`start_up_0613`]'s first 0x1fa bytes, which `magic_0609`'s start-up
/// ([`crate::r27::start_up_0609`]) shares: everything up to slots 222 and
/// 221.
pub(crate) fn start_up_head_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; push ecx; push esi: the return address 4
    // below
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    crate::s4_summon::targets_init_c(g, img, img.rel32_target(f + 0xc), esp, actor);
    stores_0613(
        g,
        img,
        f,
        &[0x15, 0x1f, 0x29, 0x33, 0x3d, 0x47, 0x51, 0x5b, 0x65, 0x6f],
    );
    // the static nodes: vtable, pass halfword, links to themselves, no
    // parent (the halfword stores `66 c7 05 [G] imm16`)
    for (dw, hw, rest) in [
        (0x79u32, 0x83u32, [0x8cu32, 0x96, 0xa0]),
        (0xaa, 0xb4, [0xbd, 0xc7, 0xd1]),
        (0xdb, 0xe5, [0xee, 0xf8, 0x102]),
        (0x10c, 0x116, [0x11f, 0x129, 0x133]),
    ] {
        stores_0613(g, img, f, &[dw]);
        g.write::<u16>(w(hw + 3), w(hw + 7) as u16);
        stores_0613(g, img, f, &rest);
    }
    stores_0613(g, img, f, &[0x148]);
    g.call(t(g, 0x11, 718), 0, &[w(0x144)]);
    let h = g.call(t(g, 0x155, 719), 0, &[w(0x167), 8, 0]).eax;
    let lock = g.read::<u32>(w(0x16f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x17b), h);
    g.write::<u32>(lock, 1);
    g.call(t(g, 0x186, 711), 0, &[h, w(0x180), 0]);
    let lock = g.read::<u32>(w(0x199));
    g.write::<u32>(lock, saved);
    let hh = g.read::<u32>(w(0x1b9));
    let mut st = vec![w(0x1be), hh, w(0x1b3), 0x10_0000];
    // call width: 4
    g.call(t(g, 0x1a3, 710), 0, &st);
    for (tab, slot, p) in [(0x1c5u32, 222u32, 0x1d9u32), (0x1e0, 221, 0x1f4)] {
        st.splice(0..0, [w(p), 0, 0x3e8, 2]);
        // call width: 12 (the earlier calls' words stay on the stack)
        g.call(t(g, tab, slot), 0, &st);
    }
}

/// [`start_up_0613`] (704 bytes, 1 overlay).
pub const START_UP_0613: Sig = Sig {
    len: 704,
    hash: 0x207b_4495_c3a2_31fb,
    calls: &[(0xc, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};
