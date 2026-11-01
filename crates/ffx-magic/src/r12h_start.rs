//! Round 12 (own rows from `magic_0450` up): the two-object effect start
//! of `magic_0469`, `0470`, `0551`, `0554` and `0590`, called by their
//! cue start ([`crate::m0638::cue_start`]). Each copy differs in its
//! globals and immediates only; the ports read them from the code
//! (`w(off)` = the word at `+off`).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The `f32` at the address in the operand at `f + op`.
pub(crate) fn konst<G: Game>(g: &G, img: &Image, f: u32, op: u32) -> f32 {
    g.read::<f32>(img.word(f + op))
}

/// `x` wrapped once (`comiss` + `jbe`, an unordered `x` stays): `x - T`
/// above `HI`, `x + T2` below `LO`; `ops` are the operand offsets of
/// `HI`, `T`, `LO`, `T2`.
pub(crate) fn wrap<G: Game>(g: &G, img: &Image, f: u32, x: f32, ops: [u32; 4]) -> f32 {
    let k = |op: u32| konst(g, img, f, op);
    if x > k(ops[0]) {
        sub(x, k(ops[1]))
    } else if k(ops[2]) > x {
        add(x, k(ops[3]))
    } else {
        x
    }
}

/// The 32-bit stores `mov dword [G], imm32` (10 bytes each) at the
/// offsets `at`: `G` (`+2`) = the immediate (`+6`).
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, at: &[u32]) {
    for &o in at {
        g.write::<u32>(img.word(f + o + 2), img.word(f + o + 6));
    }
}

/// Slot 333 (the actor's angle, ST0 stored as `f32`; its table global
/// at operand `ops.0`, the words `p`) taken from the constant at operand
/// `ops.1`: `K0 - angle`.
pub(crate) fn angle_from<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: &Pushes,
    ops: (u32, u32),
) -> f32 {
    let t = slot_target(g, img.word(f + ops.0), 333);
    // call width: 12 (Pushes caps the words at 12)
    let a = crate::ob_model::st0_f32(p.call(g, t, 0).st0);
    sub(konst(g, img, f, ops.1), a)
}

/// The free-list node `node` appended to the list `G_FIRST`/`G_LAST`
/// (operands at `ops`: `G_FIRST` read, `G_LAST` read, `G_LAST` written,
/// `G_FIRST` written when empty, `G_LAST` written when empty): its `+4`
/// 0; after the last node, or the first with `+0` 0; `+8` itself.
pub(crate) fn append<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 5]) {
    let w = |off: u32| img.word(f + off);
    let first = g.read::<u32>(w(ops[0]));
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(ops[3]), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(ops[4]), node);
    } else {
        let last = g.read::<u32>(w(ops[1]));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(ops[2]), node);
    }
}

/// The effect's two objects started (`magic_0469` 0x10001340, no
/// arguments, `ebp` frame with one local): two buffer globals from
/// immediates; slot 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`crate::l5_objhost::part_init`]); object 1 started
/// ([`crate::s4_misc::start_object_owners`], `edx` = `O1`, scale `K`,
/// `(0, 0x46, 0x82, CB)`); slot 262 `(*G_A, X, Y, Z)` (the actor's point
/// into three globals), `W` = 1.0; the angle `K0 - slot 333 (*G_A)`
/// wrapped once into `O1`'s angle, `Y += DY`, two more floats; object
/// 2's record (eight words and a halfword) and object 2 started (`(1, 0,
/// 0, 0)`); the angle again into `O2`'s, its callback word and record;
/// the actor block ([`crate::sa_misc::me_block`] `(B)`); a free-list node
/// ([`crate::m0032::pop_node`]) appended to the task list. Each slot
/// call sees the words the earlier calls left. Returns 0.
///
/// `magic_0469` and `magic_0551` (664 bytes each; one float immediate
/// differs).
pub fn objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    p.push_all(&[0, 8, w(0x14)]);
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    let t = slot_target(g, w(0x5), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, t, 0).eax;
    p.pop(1);
    g.write::<u32>(w(0x32), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    let scale = konst(g, img, f, 0x3f);
    p.push_all(&[w(0x44), 0x82, 0x46, 0]);
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + 0x57),
        w(0x52),
        scale,
        0,
        0x46,
        (0x82, w(0x44)),
    );
    let actor = g.read::<u32>(w(0x77));
    p.push_all(&[w(0x61), w(0x6c), w(0x71), actor]);
    let t = slot_target(g, w(0x5c), 262);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    g.write::<u32>(w(0x8a), w(0x8e));
    p.push(g.read::<u32>(w(0x84)));
    let a = angle_from(g, img, f, &p, (0x7e, 0xa1));
    let a = wrap(g, img, f, a, [0xb8, 0xae, 0xc8, 0xae]);
    p.pop(11);
    let scale = konst(g, img, f, 0xd9);
    g.write::<f32>(w(0xe3), a);
    let y = add(g.read::<f32>(w(0xeb)), konst(g, img, f, 0xf3));
    p.push_all(&[0, 0, 0, 1]);
    g.write::<f32>(w(0x105), y);
    stores(
        g,
        img,
        f,
        &[
            0x109, 0x113, 0x11d, 0x127, 0x131, 0x13b, 0x145, 0x14f, 0x159, 0x163,
        ],
    );
    g.write::<u16>(w(0x16f), 0);
    g.write::<u32>(w(0x175), w(0x179));
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + 0x17e),
        w(0xfd),
        scale,
        1,
        0,
        (0, 0),
    );
    p.push(g.read::<u32>(w(0x189)));
    let a = angle_from(g, img, f, &p, (0x183, 0x19c));
    let a = wrap(g, img, f, a, [0x1ab, 0x1b5, 0x1bf, 0x1cc]);
    g.write::<f32>(w(0x1db), a);
    g.write::<u32>(w(0x1e1), w(0x1e5));
    stores(
        g,
        img,
        f,
        &[0x1e9, 0x1f3, 0x1fd, 0x207, 0x211, 0x21b, 0x225, 0x22f],
    );
    g.write::<u16>(w(0x23b), 0);
    g.write::<u32>(w(0x241), w(0x245));
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x24a), w(0x1d3));
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x252));
    append(g, img, f, node, [0x258, 0x268, 0x275, 0x286, 0x28e]);
    Ret::eax(0)
}

/// [`objects_start`]'s callees.
const OBJECTS_START_CALLS: &[(u32, Callee)] = &[
    (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
    (0x57, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
    (0x17e, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
    (0x24a, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
    (0x252, Callee::Sig(&crate::m0032::POP_NODE)),
];

/// [`objects_start`] in `magic_0469` (0x10001340).
pub const OBJECTS_START_0469: Sig = Sig {
    len: 664,
    hash: 0x427b_a124_eac2_abb2,
    calls: OBJECTS_START_CALLS,
    imports: &[],
};

/// [`objects_start`] in `magic_0551` (0x10001340).
pub const OBJECTS_START_0551: Sig = Sig {
    len: 664,
    hash: 0x8a12_6ade_333a_6232,
    calls: OBJECTS_START_CALLS,
    imports: &[],
};

/// The operand layout of [`objects_start_b`]'s copies: the code after
/// object 2's float stores sits `shift` bytes back (`magic_0590` has no
/// second float store there), and `Y` moves by `+DY` or `-DY`.
struct LayoutB {
    shift: u32,
    up: bool,
}

/// The effect's two objects started, second shape (`magic_0470`
/// 0x10001370, no arguments, `ebp` frame with one local): as
/// [`objects_start`], but the angle (`K0 - slot 333 (*G_A)` wrapped
/// twice, `+DA` between: the local) is taken before object 1 starts and
/// stored after slot 262; after object 2 a halfword of its record is
/// cleared before the second angle, and slot 271 `(4)` takes the actor
/// block's place.
fn objects_start_b<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &LayoutB) -> Ret {
    let f = fr.at;
    // operands after the second float store (+0x142) move back by `shift`
    let o = |x: u32| if x > 0x142 { x - l.shift } else { x };
    let w = |off: u32| img.word(f + o(off));
    let mut p = Pushes::default();
    p.push_all(&[0, 8, w(0x14)]);
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    let t = slot_target(g, w(0x5), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, t, 0).eax;
    p.pop(1);
    g.write::<u32>(w(0x32), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    p.push(g.read::<u32>(w(0x42)));
    let a = angle_from(g, img, f, &p, (0x3c, 0x55));
    p.pop(3);
    let (t1, lo) = (konst(g, img, f, 0x6a), konst(g, img, f, 0x72));
    let hi = konst(g, img, f, 0x62);
    let once = |x: f32| {
        if x > hi {
            sub(x, t1)
        } else if lo > x {
            add(x, t1)
        } else {
            x
        }
    };
    let a = once(add(once(a), konst(g, img, f, 0x91)));
    let scale = konst(g, img, f, 0xb7);
    p.push_all(&[w(0xbc), 0x82, 0x46, 0]);
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + 0xcf),
        w(0xca),
        scale,
        0,
        0x46,
        (0x82, w(0xbc)),
    );
    let actor = g.read::<u32>(w(0xef));
    p.push_all(&[w(0xd9), w(0xe4), w(0xe9), actor]);
    let t = slot_target(g, w(0xd4), 262);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    let scale = konst(g, img, f, 0xfe);
    g.write::<f32>(w(0x108), a);
    let (y, dy) = (g.read::<f32>(w(0x110)), konst(g, img, f, 0x118));
    let y = if l.up { add(y, dy) } else { sub(y, dy) };
    p.push_all(&[0, 0, 0, 1]);
    g.write::<u32>(w(0x128), w(0x12c));
    g.write::<f32>(w(0x134), y);
    g.write::<u32>(w(0x13a), w(0x13e));
    if l.shift == 0 {
        g.write::<u32>(w(0x144), w(0x148));
    }
    let at: Vec<u32> = [0x14c, 0x156, 0x160, 0x16a, 0x174, 0x17e, 0x188, 0x192]
        .iter()
        .map(|&x| o(x))
        .collect();
    stores(g, img, f, &at);
    g.write::<u16>(w(0x19e), 0);
    g.write::<u32>(w(0x1a4), w(0x1a8));
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + o(0x1ad)),
        w(0x122),
        scale,
        1,
        0,
        (0, 0),
    );
    p.push(g.read::<u32>(w(0x1b8)));
    let a = angle_from(g, img, f, &p, (o(0x1b2), o(0x1cb)));
    let a = wrap(g, img, f, a, [o(0x1da), o(0x1e4), o(0x1ee), o(0x1fb)]);
    g.write::<u16>(w(0x203), 0);
    g.write::<f32>(w(0x210), a);
    g.write::<u32>(w(0x216), w(0x21a));
    let at: Vec<u32> = [0x21e, 0x228, 0x232, 0x23c, 0x246, 0x250, 0x25a, 0x264]
        .iter()
        .map(|&x| o(x))
        .collect();
    stores(g, img, f, &at);
    g.write::<u32>(w(0x270), w(0x274));
    let t = slot_target(g, w(0x208), 271);
    // call width: 1
    g.call(t, 0, &[4]);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + o(0x286)));
    let ops = [0x28c, 0x29c, 0x2a9, 0x2ba, 0x2c2].map(o);
    append(g, img, f, node, ops);
    Ret::eax(0)
}

/// [`objects_start_b`] as `magic_0470` and `magic_0554` have it (716
/// bytes, `Y + DY`).
pub fn objects_start_b_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    objects_start_b(g, img, fr, &LayoutB { shift: 0, up: true })
}

/// [`objects_start_b`] as `magic_0590` has it (706 bytes, `Y - DY`, no
/// second float store).
pub fn objects_start_b_down<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    objects_start_b(
        g,
        img,
        fr,
        &LayoutB {
            shift: 10,
            up: false,
        },
    )
}

/// [`objects_start_b_up`]'s callees.
const OBJECTS_START_B_UP_CALLS: &[(u32, Callee)] = &[
    (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
    (0xcf, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
    (0x1ad, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
    (0x286, Callee::Sig(&crate::m0032::POP_NODE)),
];

/// [`objects_start_b_up`] in `magic_0470` (0x10001370).
pub const OBJECTS_START_B_0470: Sig = Sig {
    len: 716,
    hash: 0xb34b_c065_eba2_0416,
    calls: OBJECTS_START_B_UP_CALLS,
    imports: &[],
};

/// [`objects_start_b_up`] in `magic_0554` (0x10001370).
pub const OBJECTS_START_B_0554: Sig = Sig {
    len: 716,
    hash: 0x9d06_eff6_2c07_0296,
    calls: OBJECTS_START_B_UP_CALLS,
    imports: &[],
};

/// [`objects_start_b_down`] in `magic_0590` (0x10001370).
pub const OBJECTS_START_B_0590: Sig = Sig {
    len: 706,
    hash: 0x6b36_47d3_72ef_1be0,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xcf, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x1a3, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x27c, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// [`crate::s3_misc::task_start`] at `target` with its stack word `arg`
/// (written where the original's is: the callee's `esp` is `esp`, the
/// word above its return address).
pub(crate) fn task_start<G: Game>(g: &mut G, img: &Image, target: u32, esp: u32, arg: u32) -> u32 {
    g.write::<u32>(esp.wrapping_add(4), arg);
    let callee = Frame {
        at: target,
        esp,
        ..Frame::default()
    };
    crate::s3_misc::task_start(g, img, &callee)
}
