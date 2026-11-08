//! Round 16's own ports (own rows of `magic_0450` and up): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code
//! written out where no ported kin exists.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`staged_volley_task`](crate::s7_misc::staged_volley_task) in
/// `magic_0621` (0x10006e30; `magic_0677`'s immediates; the throw is
/// [`model_object_setup`](crate::s7_misc::model_object_setup)'s 908-byte
/// copy, [`MODEL_OBJECT_SETUP_0621`](crate::s7_misc::MODEL_OBJECT_SETUP_0621)).
pub const STAGED_VOLLEY_TASK_0621: Sig = Sig {
    len: 898,
    hash: 0xda8f_c44e_a2e3_39fe,
    calls: &[
        (0x65, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x85, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0xd8, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x26d, Callee::Sig(&crate::s7_misc::MODEL_OBJECT_SETUP_0621)),
        (0x36e, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
    ],
    imports: &[],
};

/// `magic_0480`'s next object at its record (0x100043e0, cdecl `(S)`; a
/// cookie frame, `ebp` = `esp - 4`, 331 bytes): the object `O = B +
/// 0x130 S[+0x2c]` (`B` the operand `+0x29`) started at the scale `*K`
/// (`+0x14`; [`start_object_first`](crate::o6_fam_object::start_object_first),
/// frameless); slot 262 `(*G_A, O+0xd0, O+0xd4, O+0xd8)`, `O+0xdc` 1.0,
/// `O+0xd4 -= *K2`, `O+0xe8` -4.0; the yaw `*K3 - slot 333 (*G_A)` (ST0)
/// brought back once by `T` when above `H` or below `Lo` into `O+0xc4`;
/// `S[+0x2c]` one more. Then record 2 of the DLL's table set twice
/// ([`record_copy`](crate::o4_fam3::record_copy), `edx` 2, which it
/// keeps): from the local `(1.396, 0, -, -)`, three light words set
/// (`+0x101` ..), and from `(0, 0, 0, -)`. Returns the second copy's
/// last word.
pub fn object_records_0480<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    let s = fr.arg(g, 0);
    let obj = g
        .read::<u32>(s + 0x2c)
        .wrapping_mul(0x130)
        .wrapping_add(w(0x29));
    // push ebx, esi, edi; sub esp, 0x10; call (the start reads no stack)
    let at = img.rel32_target(f + 0x33);
    let call = Frame {
        at,
        esp: e.wrapping_sub(0x3c),
        edx: obj,
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x14)))],
        ..*fr
    };
    crate::o6_fam_object::start_object_first(g, img, at, &call);
    let p = obj.wrapping_add(0xd0);
    let mut st = Pushes::default();
    st.push_all(&[p + 8, p + 4, p, g.read::<u32>(w(0x4d))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x46), 0x418 / 4), fr.ecx);
    st.push(g.read::<u32>(w(0x60)));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let v = sub(g.read::<f32>(obj + 0xd4), k(g, 0x77));
    g.write::<u32>(obj + 0xe8, 0xc080_0000);
    g.write::<f32>(obj + 0xd4, v);
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x5a), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let mut yaw = sub(k(g, 0x9c), t);
    // `comiss` + `jbe`: an unordered yaw stays
    if yaw > k(g, 0xab) {
        yaw = sub(yaw, k(g, 0xb5));
    } else if k(g, 0xbf) > yaw {
        yaw = add(yaw, k(g, 0xcc));
    }
    g.write::<f32>(obj + 0xc4, yaw);
    let n = g.read::<u32>(s + 0x2c).wrapping_add(1);
    g.write::<u32>(s + 0x2c, n);
    // the local at `ebp - 0x14`
    let src = e.wrapping_sub(0x18);
    g.write::<u32>(src, 0x3fb2_b8c0);
    g.write::<u32>(src + 4, 0);
    // junk read: [ebp-0xc], [ebp-0x8] never written; 0 (brief 2026-10-24)
    g.write::<u32>(src + 8, 0);
    g.write::<u32>(src + 0xc, 0);
    crate::o4_fam3::record_copy(g, img, img.rel32_target(f + 0xf4), 2, src);
    crate::r11_fades::stores(g, img, f, &[0xff, 0x109, 0x113]);
    for o in [0u32, 4, 8] {
        g.write::<u32>(src + o, 0);
    }
    // junk read: [ebp-0x8] never written; 0 (brief 2026-10-24)
    let eax = crate::o4_fam3::record_copy(g, img, img.rel32_target(f + 0x133), 2, src);
    Ret::eax(eax)
}

/// [`object_records_0480`] (the `rel32`s: the object start, the two
/// record copies and the cookie check).
pub const OBJECT_RECORDS_0480: Sig = Sig {
    len: 331,
    hash: 0x96ac_9f5d_94a3_b8bd,
    calls: &[
        (
            0x33,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
        ),
        (0xf4, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2880)),
        (0x133, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2910)),
        (0x143, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0474`'s start-up (0x100013e0, 935 bytes, run by its load
/// step; cdecl, no arguments; a cookie frame, `ebp` = `esp - 4`): two
/// words set, slot 719 `(PART, 8, 0)` into `G_H` (`+0x3e`), the part set
/// up ([`part_init`](crate::l5_objhost::part_init)); the first object
/// (`+0x53`, scale `*K` `+0x4b`) started
/// ([`start_object_at`](crate::s4_misc::start_object_at), index and
/// owners 0); slot 262 `(*G_A, P, P+4, P+8)` (the immediates `+0x79`,
/// `+0x74`, `+0x69`), `P+0xc` 1.0, the yaw `*K1 - slot 333 (*G_A)` (ST0)
/// brought back once by `T` (`+0xb6`) when above `H` or below `Lo` into
/// `+0xe1`; `*Y -= *K2` (`+0xe9`), a halfword 0 and twelve words set.
/// The kind byte `*(*G_K)` (`+0xfe`): for kinds 8..17 the second object
/// (`+0x182`) gets its colour bytes and counts and a timed start
/// ([`timed_start`](crate::m0474::timed_start)), scale `*K3` (`+0x17d`),
/// else scale `*K4` (`+0x1c5`); then the third object (`+0x1d5`, index
/// 1, owner 0x3c, `owner_b` the callback `+0x1cb`) starts; with the
/// battle actor `A = *(G_T + 4 *(*G_K))`, slot 262 `(A, ..)` and slot 268
/// `(A, 6, &local)` (the local's `y` into `+0x22c`, `+0x232` 1.0), the
/// second yaw (slot 333 `(A)`) into `+0x2ac`; eight words, a halfword
/// and a word set; two scale tasks
/// ([`scale_task`](crate::s4_misc::scale_task): `ecx` `+0x2a4`, scales
/// `*K5`, `*K6`, count 0; `ecx` `+0x323`, `*K6`, `*K5`, count 0x50);
/// slot 271 `(4)`; the actor block of slot 535's answer
/// ([`me_block`](crate::sa_misc::me_block)); a pool node
/// ([`take`](crate::sa_pool::take), owner `+0x35e`) appended to the task
/// list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn start_up_0474<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    let wrap = |g: &G, y: f32, (hi, t, lo, t2): (u32, u32, u32, u32)| {
        // `comiss` + `jbe`: an unordered yaw stays
        if y > k(g, hi) {
            sub(y, k(g, t))
        } else if k(g, lo) > y {
            add(y, k(g, t2))
        } else {
            y
        }
    };
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x20)]);
    stores(g, img, f, &[0x24, 0x2e]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot_target(g, w(0x11), 0xb3c / 4), fr.ecx).eax;
    g.write::<u32>(w(0x3e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    let at = img.rel32_target(f + 0x5f);
    crate::s4_misc::start_object_at(g, img, at, w(0x53), k(g, 0x4b), 0, 0, 0);
    // push 0; push ecx; push 0; push 0: the start's words stay (the calls
    // see its index and owner; the `ecx` word under them is not passed)
    let mut st = Pushes::default();
    st.push_all(&[0, 0]);
    st.push_all(&[w(0x69), w(0x74), w(0x79), g.read::<u32>(w(0x7f))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x64), 0x418 / 4), fr.ecx);
    st.push(g.read::<u32>(w(0x8c)));
    g.write::<u32>(w(0x92), w(0x96));
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x86), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let yaw = wrap(g, sub(k(g, 0xa9), t), (0xc0, 0xb6, 0xd0, 0xb6));
    g.write::<f32>(w(0xe1), yaw);
    let y = sub(g.read::<f32>(w(0xe9)), k(g, 0xf1));
    g.write::<u16>(w(0xf9), 0);
    let kind_at = g.read::<u32>(w(0xfe));
    stores(g, img, f, &[0x102]);
    g.write::<f32>(w(0x110), y);
    stores(
        g,
        img,
        f,
        &[
            0x114, 0x11e, 0x128, 0x132, 0x13c, 0x146, 0x150, 0x15a, 0x164,
        ],
    );
    let kind = u32::from(g.read::<u8>(kind_at));
    let scale = if kind.wrapping_sub(8) <= 9 {
        g.write::<u32>(w(0x188), w(0x18c));
        g.write::<u8>(w(0x192), w(0x196) as u8);
        g.write::<u16>(w(0x19a), w(0x19e) as u16);
        g.write::<u8>(w(0x1a2), w(0x1a6) as u8);
        g.write::<u32>(w(0x1a9), w(0x1ad));
        g.write::<u16>(w(0x1b4), w(0x1b8) as u16);
        crate::m0474::timed_start(g, img, img.rel32_target(f + 0x1bb), w(0x182));
        k(g, 0x17d)
    } else {
        k(g, 0x1c5)
    };
    let at = img.rel32_target(f + 0x1da);
    crate::s4_misc::start_object_at(g, img, at, w(0x1d5), scale, 1, 0x3c, w(0x1cb));
    // push esi; push CB; push ecx; push 0x3c; push 1: the start's words
    // stay (its index and owner passed on; `ecx` and `esi` under them not)
    let mut st = Pushes::default();
    st.push_all(&[0x3c, 1]);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1df))));
    let a = g.read::<u32>(b.wrapping_mul(4).wrapping_add(w(0x1f3)));
    st.push_all(&[w(0x1e4), w(0x1ec), w(0x1fd), a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x1f8), 0x418 / 4), fr.ecx);
    let local = e.wrapping_sub(0x18);
    st.push_all(&[local, 6, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x20f), 0x430 / 4), fr.ecx);
    let py = g.read::<u32>(local + 4);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x224))));
    g.write::<u32>(w(0x22c), py);
    g.write::<u32>(w(0x232), w(0x236));
    st.push(g.read::<u32>(b.wrapping_mul(4).wrapping_add(w(0x240))));
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x245), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let yaw = wrap(g, sub(k(g, 0x258), t), (0x268, 0x272, 0x27c, 0x289));
    let (k5, k6) = (g.read::<u32>(w(0x291)), g.read::<u32>(w(0x299)));
    g.write::<f32>(w(0x2ac), yaw);
    stores(
        g,
        img,
        f,
        &[0x2b0, 0x2ba, 0x2c4, 0x2ce, 0x2d8, 0x2e2, 0x2ec, 0x2f6],
    );
    g.write::<u16>(w(0x302), 0);
    stores(g, img, f, &[0x306]);
    // push 0; call: the count on the stack
    g.write::<u32>(e.wrapping_sub(0x20), 0);
    let task = |at: u32, esp: u32, ecx: u32, x1: u32, x2: u32| Frame {
        at: img.rel32_target(f + at),
        esp,
        ecx,
        xmm: [0, u128::from(x1), u128::from(x2), 0],
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &task(0x311, e.wrapping_sub(0x24), w(0x2a4), k5, k6));
    let k6b = g.read::<u32>(w(0x319));
    g.write::<u32>(e.wrapping_sub(0x24), 0x50);
    crate::s4_misc::scale_task(
        g,
        img,
        &task(0x328, e.wrapping_sub(0x28), w(0x323), k6b, k5),
    );
    let mut st = Pushes::default();
    st.push_all(&[0, 0x50, 4]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x32d), 0x43c / 4), fr.ecx);
    st.push(g.read::<u32>(w(0x342)));
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, slot_target(g, w(0x33c), 0x85c / 4), fr.ecx).eax;
    let a = g.read::<u32>(r.wrapping_mul(4).wrapping_add(w(0x351)));
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x356), a);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x363), w(0x35e));
    let any = g.read::<u32>(w(0x369));
    g.write::<u32>(node.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x38b), node);
    } else {
        let tail = g.read::<u32>(w(0x379));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x396), node);
    Ret::eax(0)
}

/// [`start_up_0474`] (the `rel32`s: the part's set-up, the object starts,
/// the timed start, the scale tasks, the actor block, the pool's take
/// and the cookie check).
pub const START_UP_0474: Sig = Sig {
    len: 935,
    hash: 0xdbad_e41d_6007_2bd5,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1bb, Callee::Sig(&crate::m0474::TIMED_START)),
        (0x1da, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x311, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x328, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x356, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x363, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x39f, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_setup_task`](crate::s7_misc::summon_setup_task) in
/// `magic_0683` (0x10006a20, 525 bytes: no model check).
pub const SUMMON_SETUP_TASK_0683: Sig = Sig {
    len: 525,
    hash: 0xf739_d6b9_e180_f294,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x6c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x77, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x82, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x8d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x98, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xa3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xed, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x130, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x19d, Callee::Sig(&crate::nodes::INSERT)),
        (0x1db, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0683`'s summon object (internal: `ecx` = `O`, `edx` = `R`,
/// `xmm2` = `s`; an aligned frame, `P` = `((esp - 4) & !7) - 0x30` after
/// the saved registers, the set-up record at `P+0x18`; 779 bytes): as
/// [`summon_object_0685`](crate::s9_misc::summon_object_0685) on the
/// larger object: `O` joins the task ring, `B = O+0x220` and `C =
/// O+0x278` go on the node list, `C`'s colour `(1, 1, 1, s K1 K2)`, the
/// node `N = O+0x2d0` inserted; `B`'s point slot 268 `(R[0x54], 0x11,
/// O+0x23c)` (`w` 1.0); the particle `O+0x20` (program index 2), the
/// effect node `E = O+0x57c`; `O+0x240`, `O+0x23c` and `O+0x244` moved
/// by a random step each (slot 307, `(r - K3) K4 - K5`); `O+0x578 = R`,
/// `+0x56c = s`, `+0x568`, `+0x570`, `+0x574` 0. Returns `R`.
pub fn summon_object_0683<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let (o, r) = (fr.ecx, fr.edx);
    let s = fr.xmm_f32(2);
    let p = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x15));
    g.write::<u32>(o + 0x10, w(0x1c));
    g.write::<u32>(o + 0x14, o);
    let last = g.read::<u32>(w(0x24));
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x2e));
    let last = g.read::<u32>(w(0x33));
    g.write::<u32>(last.wrapping_add(4), o);
    let fl = g.read::<u32>(w(0x42)) & !1;
    g.write::<u32>(w(0x42), fl);
    g.write::<u32>(w(0x49), o);
    let (b, c) = (o + 0x220, o + 0x278);
    let ops_b = [0x5c, 0x63, 0x68, 0x70, 0x80, 0x88, 0x94, 0x9a, 0xa0];
    crate::s9_misc::list_node(g, img, f, b, ops_b);
    g.write::<f32>(p + 0xc, s);
    g.write::<u32>(p + 0x10, r);
    let ops_c = [0xb9, 0xc0, 0xc5, 0xcd, 0xd2, 0xda, 0xe6, 0xec, 0xf2];
    crate::s9_misc::list_node(g, img, f, c, ops_c);
    let rec = p + 0x18;
    let n = o + 0x2d0;
    for at in [0x294u32, 0x298, 0x29c] {
        g.write::<u32>(o + at, 0x3f80_0000);
    }
    for (i, v) in (0u32..).zip([n, w(0x12a), w(0x132), b, w(0x13a), c]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let insert = Frame {
        at: img.rel32_target(f + 0x143),
        esp: p.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &insert);
    let h = g.read::<u32>(r.wrapping_add(0x54));
    let pt = o + 0x23c;
    // call width: 3
    g.call(slot_target(g, w(0x158), 0x430 / 4), 0, &[h, 0x11, pt]);
    g.write::<u32>(pt + 0xc, 0x3f80_0000);
    let step = |g: &mut G, tab: u32, (k1, k2, k3): (u32, u32, u32)| {
        let x = g.call(slot_target(g, w(tab), 0x4cc / 4), 0, &[]);
        let x = sub(crate::m0018::st0_f32(x.st0), k(g, k1));
        sub(mul(x, k(g, k2)), k(g, k3))
    };
    let x = step(g, 0x165, (0x189, 0x1a5, 0x1b5));
    let d = g.read::<u32>(w(0x18e));
    let q = o + 0x20;
    for (i, v) in (0u32..).zip([q, w(0x19d), n, d, w(0x1bd), 2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let v = add(x, g.read::<f32>(o + 0x240));
    g.write::<f32>(o + 0x240, v);
    g.write::<f32>(o + 0x2a0, mul(mul(s, k(g, 0x1e3)), k(g, 0x1eb)));
    g.write::<u32>(q, w(0x1f9));
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x202), rec);
    let e = o + 0x57c;
    for (i, v) in (0u32..).zip([e, w(0x21e), w(0x226), b, w(0x232), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x23f), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x249));
    let fl = g.read::<u32>(e + 0x2c) | 1;
    g.write::<u32>(e + 0x2c, fl);
    g.write::<u32>(e + 0x30, 0);
    let x = step(g, 0x24e, (0x273, 0x280, 0x288));
    let v = add(x, g.read::<f32>(pt));
    g.write::<f32>(pt, v);
    let x = step(g, 0x278, (0x2aa, 0x2b2, 0x2ba));
    g.write::<u32>(o + 0x578, r);
    let v = add(x, g.read::<f32>(o + 0x244));
    for z in [0x568u32, 0x570, 0x574] {
        g.write::<u32>(o + z, 0);
    }
    g.write::<f32>(o + 0x244, v);
    g.write::<f32>(o + 0x56c, s);
    Ret::eax(r)
}

/// [`summon_object_0683`] (the `rel32`s: the node insert, the particle
/// node, the effect node).
pub const SUMMON_OBJECT_0683: Sig = Sig {
    len: 779,
    hash: 0x9c30_fd5d_f1d6_e29e,
    calls: &[
        (0x143, Callee::Sig(&crate::nodes::INSERT)),
        (0x202, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x23f, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// [`staged_volley_task`](crate::s7_misc::staged_volley_task) in
/// `magic_0683` (0x10006f40; `magic_0682`'s immediates; the throw is
/// [`summon_object_0683`]).
pub const STAGED_VOLLEY_TASK_0683: Sig = Sig {
    len: 898,
    hash: 0xab9b_cfcb_bd84_3ae4,
    calls: &[
        (0x65, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x85, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0xd8, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x26d, Callee::Sig(&SUMMON_OBJECT_0683)),
        (0x36e, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
    ],
    imports: &[],
};

/// The 12-byte `ecx` thunk ([`ecx_jmp`](crate::sl_wrap::ecx_jmp)) in
/// `magic_0613` (0x10004a20) over its mean visit
/// ([`MEAN_VISIT_0613`](crate::m0164::MEAN_VISIT_0613)).
pub const JMP_MEAN_VISIT_0613: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&crate::m0164::MEAN_VISIT_0613))],
    imports: &[],
};

/// A ring node of the pool's (count u16 at the operand `pop.0`, free
/// head `node`) taken as the original inlines it
/// ([`pool_pop`](crate::r13_misc::pool_pop), the operands `pop`) and
/// appended to the task list (`first` the head word tested, `first_w`
/// written when it was empty; `tail` the node to link after otherwise:
/// `None` reads the tail word `last`). `rec`: the node's byte `+0x18` 1
/// and `+0x14` the immediate at that operand. With none free the node is
/// 0 and the original writes through it (not run). Returns the node and
/// the new free head.
fn ring_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    pop: (u32, u32, u32, u32, u32),
    rec: Option<u32>,
    (first, first_w, tail, last): (u32, u32, Option<u32>, u32),
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let (n, head) = if g.read::<u16>(w(pop.0)) == 0 {
        (0, 0)
    } else {
        (node, crate::r13_misc::pool_pop(g, img, f, node, pop).1)
    };
    let any = g.read::<u32>(w(first));
    if let Some(r) = rec {
        g.write::<u8>(n.wrapping_add(0x18), 1);
        g.write::<u32>(n.wrapping_add(0x14), w(r));
    }
    g.write::<u32>(n.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(first_w), n);
    } else {
        let t = tail.unwrap_or_else(|| g.read::<u32>(w(last)));
        g.write::<u32>(n, t);
        g.write::<u32>(t.wrapping_add(4), n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    (n, head)
}

/// A summon's start-up with dealt actors (run by its load step, the
/// [`summon_load_objects`](crate::s7_misc::summon_load_objects) shape;
/// cdecl, no arguments; a cookie frame, `ebp` = `esp - 4`): three words
/// set (`+0x26` ..), slot 719 `(PART, 8, 0)` into `G_H` (`+0x4a`; two of
/// its words stay), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)), the actors dealt into
/// the bytes at `ebp - 0x14` ([`deal`](crate::sb_misc::deal)). The yaw
/// `K - slot 333 (*G_X)` (ST0) is brought back once by `T` when above
/// `H` or below `Lo`. For each of the `*G_N` objects `i` (`O = B + 0x130
/// i`, `B` the operand `+0xd5` less 0xd4; the dealt byte `d`, signed):
/// slot 404 `(0.2)` (the immediate `+0xf7`, ST0 `r`), the object started
/// at the scale `(r + K1) K2`
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// the stack `(-, owner, -, 0.2)`); the actor `A` = `*G_X` for `d` = -1,
/// else `*(G_T + 4 G_B[d])`; slots 262 `(A, O+0xd0, O+0xd4, O+0xd8)` and
/// 268 `(A, 0x11, O+0xd0)`, `O+0xdc` 1.0, `O+0xc4` the yaw, `O+0x120` the
/// actor's number (`*G_X2` for -1, else `G_B[d]`), the owner one step
/// more (imm8 `+0x1a1`: 0x14 or 0x2d), the record `E = E0 + 0x34 i`
/// cleared (callbacks `+0x1d3`, `+0x1da`) with `O+0x110 = E`, `O+0x108`
/// the operand `+0x1ef`. Then the light words (`mov [G], imm` from
/// `+0x215`), a ring node (record `+0x332`) appended to the task list,
/// more light words with the last owner `+ 0x14` among them, a second
/// node (record `+0x461`) after it, slot 271 `(4)` and a third node
/// without a record. Returns 0.
///
/// `magic_0619` 0x100031a0 (1,362 bytes) and `magic_0681` 0x10003220
/// (1,382 bytes: one more light word in the second block, and one after
/// the second node).
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn dealt_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // `mov [G], eax` (0xa3) at +0x3c1 in `magic_0619`'s layout, 10 bytes
    // later in `magic_0681`'s
    let d = if img.word(f + 0x3c1) & 0xff == 0xa3 {
        0
    } else {
        10
    };
    let e = fr.esp;
    let dealt = e.wrapping_sub(0x18);
    stores(g, img, f, &[0x26, 0x30, 0x3a]);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x22)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot_target(g, w(0x11), 0xb3c / 4), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x4a), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4f));
    // two words of slot 719's, ebx, esi and the cookie frame above
    let at = img.rel32_target(f + 0x57);
    crate::sb_misc::deal(g, img, at, e.wrapping_sub(0x40), dealt);
    st.push(g.read::<u32>(w(0x62)));
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x5c), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let mut yaw = sub(k(g, 0x7a), t);
    // `comiss` + `jbe`: an unordered yaw stays
    if yaw > k(g, 0x89) {
        yaw = sub(yaw, k(g, 0x98));
    } else if k(g, 0xa2) > yaw {
        yaw = add(yaw, k(g, 0xaf));
    }
    g.write::<u32>(w(0xbd), w(0xc1));
    let mut owner = 0u32;
    let mut obj = w(0xd5);
    let mut rec = w(0xda);
    let actor = |g: &G, b: i32| {
        if b == -1 {
            g.read::<u32>(w(0x12f))
        } else {
            let l = g.read::<u32>(w(0x136));
            let n = g.read::<u8>(l.wrapping_add(b.cast_unsigned()));
            g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(0x141)))
        }
    };
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0xca)) {
        let b = i32::from(g.read::<i8>(dealt.wrapping_add(i)));
        // push ecx; mov [esp], 0.2 (edi saved above it)
        let mut s = Pushes::default();
        s.push(w(0xf7));
        // call width: 12 (Pushes caps the words at 12)
        let r = s.call(g, slot_target(g, w(0xe6), 0x650 / 4), fr.ecx);
        let r = crate::m0018::st0_f32(r.st0);
        let scale = mul(add(r, k(g, 0x10d)), k(g, 0x11c));
        // push ecx; push owner; push ecx; call: the words over the 0.2
        // (the two `ecx` words are not read)
        let esp = e.wrapping_sub(0x4c);
        g.write::<u32>(esp + 4, 0);
        g.write::<u32>(esp + 8, owner);
        g.write::<u32>(esp + 0xc, 0);
        g.write::<u32>(esp + 0x10, w(0xf7));
        let at = img.rel32_target(f + 0x121);
        let call = Frame {
            at,
            esp,
            edx: obj.wrapping_sub(0xd4),
            xmm: [0, 0, 0, u128::from(scale.to_bits())],
            ..*fr
        };
        // `magic_0619`'s start is round 4's copy, which takes no value
        if img.matches(at, &crate::s4_misc::START_OBJECT_FIRST) {
            crate::s4_misc::start_object_first(g, img, at, call.edx, scale, owner);
        } else {
            crate::o6_fam_object::start_object_first(g, img, at, &call);
        }
        let a = actor(g, b);
        let p = obj.wrapping_sub(4);
        let mut s = Pushes::default();
        s.push_all(&[obj + 4, obj, p, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x14a), 0x418 / 4), fr.ecx);
        s.push_all(&[p, 0x11, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x160), 0x430 / 4), fr.ecx);
        g.write::<u32>(obj + 8, 0x3f80_0000);
        g.write::<f32>(obj.wrapping_sub(0x10), yaw);
        let n = if b == -1 {
            g.read::<u32>(w(0x18f))
        } else {
            let l = g.read::<u32>(w(0x196));
            u32::from(g.read::<u8>(l.wrapping_add(b.cast_unsigned())))
        };
        owner = owner.wrapping_add(w(0x1a1) & 0xff);
        g.write::<u32>(obj + 0x4c, n);
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
        g.write::<u32>(rec + 0xc, w(0x1d3));
        g.write::<u32>(rec + 0x10, w(0x1da));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        g.write::<u32>(obj + 0x34, w(0x1ef));
        obj = obj.wrapping_add(0x130);
        rec = rec.wrapping_add(0x34);
        i += 1;
    }
    dealt_tail(g, img, fr, f, owner, d)
}

/// The dealt start-ups' tail (`fb` the base its operands are read from:
/// the function in `magic_0619`'s layout, 0xb5 bytes later in
/// `magic_0675`'s; `d` the extra light word's shift): the light words,
/// a ring node (record `+0x332`) appended to the task list, more light
/// words with the last owner `+ 0x14` among them, a second node (record
/// `+0x461`) after it, slot 271 `(4)` and a third node without a record.
/// Returns 0.
fn dealt_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame, fb: u32, owner: u32, d: u32) -> Ret {
    use crate::r11_fades::stores;
    let f = fb;
    let w = |o: u32| img.word(f + o);
    stores(
        g,
        img,
        f,
        &[0x215, 0x21f, 0x229, 0x233, 0x23d, 0x247, 0x251],
    );
    stores(
        g,
        img,
        f,
        &[0x25b, 0x265, 0x26f, 0x279, 0x283, 0x28d, 0x297],
    );
    stores(g, img, f, &[0x2a1, 0x2ab]);
    let node = g.read::<u32>(w(0x2c6));
    let (first, head) = ring_append(
        g,
        img,
        f,
        node,
        (0x211, 0x2f0, 0x2f0, 0x315, 0x320),
        Some(0x332),
        (0x326, 0x353, None, 0x340),
    );
    for i in 0..(10 + d / 10) {
        stores(g, img, f, &[0x35d + 10 * i]);
    }
    g.write::<u32>(w(0x3c2 + d), owner.wrapping_add(0x14));
    stores(g, img, f, &[0x3c6 + d, 0x3d0 + d, 0x3da + d]);
    let (second, _) = ring_append(
        g,
        img,
        f,
        head,
        (0x211, 0x40b + d, 0x413 + d, 0x444 + d, 0x44f + d),
        Some(0x461 + d),
        (0x455 + d, 0x47e + d, Some(first), 0),
    );
    if d != 0 {
        stores(g, img, f, &[0x494]);
    }
    g.write::<u32>(w(0x494 + 2 * d), second);
    let mut s = Pushes::default();
    s.push(4);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x483 + d), 0x43c / 4), fr.ecx);
    let e3 = 2 * d;
    let node = g.read::<u32>(w(0x4af + e3));
    let (third, _) = ring_append(
        g,
        img,
        f,
        node,
        (0x49d + e3, 0x4cc + e3, 0x4d4 + e3, 0x500 + e3, 0x50b + e3),
        None,
        (0x511 + e3, 0x535 + e3, None, 0x522 + e3),
    );
    g.write::<u32>(w(0x543 + e3), third);
    Ret::eax(0)
}

/// [`dealt_start`] in `magic_0619` (0x100031a0; the `rel32`s: the part's
/// set-up, the deal, the object start and the cookie check).
pub const DEALT_START_0619: Sig = Sig {
    len: 1362,
    hash: 0xc384_f773_832b_1cb4,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x57, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x121, Callee::Sig(&crate::s4_misc::START_OBJECT_FIRST)),
        (0x54a, Callee::Any),
    ],
    imports: &[],
};

/// [`dealt_start`] in `magic_0681` (0x10003220).
pub const DEALT_START_0681: Sig = Sig {
    len: 1382,
    hash: 0x0189_717c_d704_1303,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x57, Callee::Sig(&crate::sb_misc::DEAL)),
        (
            0x121,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0074_1C70),
        ),
        (0x55e, Callee::Any),
    ],
    imports: &[],
};

/// [`aim_records`](crate::s7_misc::aim_records) in `magic_0480`
/// (0x100023d0, 912 bytes: the object in the DLL's data; the `rel32`s:
/// the square root's and `_CIatan2`'s thunks and the cookie check).
pub const AIM_RECORDS_0480: Sig = Sig {
    len: 912,
    hash: 0x468b_b416_d5f3_bebd,
    calls: &[
        (0xda, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xed, Callee::Import("_CIatan2")),
        (0x12c, Callee::Import("_CIatan2")),
        (0x388, Callee::Any),
    ],
    imports: &[],
};

/// [`start_up_0369`](crate::m0367::start_up_0369) in `magic_0480`
/// (0x10004630, 194 bytes: the aim records and a pool node).
pub const START_UP_0480: Sig = Sig {
    len: 194,
    hash: 0x623d_7811_f26b_70af,
    calls: &[
        (0x29, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x4c, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x78, Callee::Sig(&AIM_RECORDS_0480)),
        (0x82, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0494`'s start-up with dealt actors (0x10002550, 770 bytes, run
/// by its load step; cdecl, no arguments; a cookie frame): as
/// [`dealt_start`] without the yaw (slot 333 `(*G_X)`'s ST0 dropped), the
/// owner 15 a step (`esi`), no `O+0xc4`/`O+0x108` stores; then the actor
/// block inlined: `*G_O` = the last owner `n` (`owner - 15`), `*G_CB` =
/// the callback `+0x1c1` when `n` > 0 else `+0x1c6`, `-1`, `*G_X2`, the
/// model (slot 413 `(*G_X2)` in a battle, `*G_BT` `+0x1cf`, else slot
/// 215), -1, 0, 1 and the halfword battle flag (1 or 0); one ring node
/// (callback `+0x2b0`) appended to the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn dealt_start_0494<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    let dealt = e.wrapping_sub(0x18);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x22)]);
    stores(g, img, f, &[0x26, 0x30]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot_target(g, w(0x11), 0xb3c / 4), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x40), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x45));
    // two words of slot 719's, esi, edi and the cookie frame above
    let at = img.rel32_target(f + 0x4d);
    crate::sb_misc::deal(g, img, at, e.wrapping_sub(0x3c), dealt);
    st.push(g.read::<u32>(w(0x58)));
    // fstp st(0): the answer is dropped
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x52), 0x534 / 4), fr.ecx);
    let mut owner = 0u32;
    let mut obj = w(0x81);
    let mut rec = w(0x86);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x75)) {
        let b = i32::from(g.read::<i8>(dealt.wrapping_add(i)));
        // push ecx; mov [esp], 0.2 (ebx saved above it)
        let mut s = Pushes::default();
        s.push(w(0xa7));
        // call width: 12 (Pushes caps the words at 12)
        let r = s.call(g, slot_target(g, w(0x96), 0x650 / 4), fr.ecx);
        let r = crate::m0018::st0_f32(r.st0);
        let scale = mul(add(r, k(g, 0xbd)), k(g, 0xcc));
        // push ecx; push owner; push ecx; call: the words over the 0.2
        // (the two `ecx` words are not read)
        let esp = e.wrapping_sub(0x48);
        g.write::<u32>(esp + 4, 0);
        g.write::<u32>(esp + 8, owner);
        g.write::<u32>(esp + 0xc, 0);
        g.write::<u32>(esp + 0x10, w(0xa7));
        let at = img.rel32_target(f + 0xd1);
        let call = Frame {
            at,
            esp,
            edx: obj.wrapping_sub(0xd4),
            xmm: [0, 0, 0, u128::from(scale.to_bits())],
            ..*fr
        };
        crate::o6_fam_object::start_object_first(g, img, at, &call);
        let a = if b == -1 {
            g.read::<u32>(w(0xdf))
        } else {
            let l = g.read::<u32>(w(0xe6));
            let n = g.read::<u8>(l.wrapping_add(b.cast_unsigned()));
            g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(0xf1)))
        };
        let p = obj.wrapping_sub(4);
        let mut s = Pushes::default();
        s.push_all(&[obj + 4, obj, p, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0xfa), 0x418 / 4), fr.ecx);
        s.push_all(&[p, 0x11, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(0x110), 0x430 / 4), fr.ecx);
        g.write::<u32>(obj + 8, 0x3f80_0000);
        let n = if b == -1 {
            g.read::<u32>(w(0x127))
        } else {
            let l = g.read::<u32>(w(0x13f));
            u32::from(g.read::<u8>(l.wrapping_add(b.cast_unsigned())))
        };
        g.write::<u32>(obj + 0x4c, n);
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
        g.write::<u32>(rec + 0xc, w(0x17b));
        g.write::<u32>(rec + 0x10, w(0x182));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        owner = owner.wrapping_add(0xf);
        obj = obj.wrapping_add(0x130);
        rec = rec.wrapping_add(0x34);
        i += 1;
    }
    let x2 = g.read::<u32>(w(0x1b7));
    let n = owner.wrapping_sub(0xf);
    let cb = if n.cast_signed() > 0 {
        w(0x1c1)
    } else {
        w(0x1c6)
    };
    let battle = g.read::<u32>(w(0x1cf)) != 0;
    g.write::<u32>(w(0x1d5), n);
    g.write::<u32>(w(0x1e0), cb);
    stores(g, img, f, &[0x1e4]);
    g.write::<u32>(w(0x1f0), x2);
    let s = if battle { 0x674 / 4 } else { 0x35c / 4 };
    let mut st = Pushes::default();
    st.push(x2);
    // call width: 12 (Pushes caps the words at 12)
    let m = st.call(g, slot_target(g, w(0x1da), s), fr.ecx).eax;
    g.write::<u32>(w(0x217), m);
    stores(g, img, f, &[0x21b, 0x225]);
    g.write::<u32>(w(0x231), 1);
    g.write::<u16>(w(0x23b), u16::from(battle));
    let node = g.read::<u32>(w(0x25a));
    let (node, _) = ring_append(
        g,
        img,
        f,
        node,
        (0x24b, 0x277, 0x27f, 0x2b0, 0x2bb),
        None,
        (0x2c1, 0x2e5, None, 0x2d2),
    );
    g.write::<u32>(w(0x2f3), node);
    Ret::eax(0)
}

/// [`dealt_start_0494`] (the `rel32`s: the part's set-up, the deal, the
/// object start and the cookie check).
pub const DEALT_START_0494: Sig = Sig {
    len: 770,
    hash: 0x5473_ad96_52cd_5af3,
    calls: &[
        (0x45, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4d, Callee::Sig(&crate::sb_misc::DEAL)),
        (
            0xd1,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0074_1C70),
        ),
        (0x2fa, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0666`'s set-up (0x10002770, cdecl `(a)`, 623 bytes): `*G_A` =
/// `a`, three words set (`+0x1c` ..), slot 718 `(NAME)`, slot 222
/// `(*G_N, 0, 1000, 0x2000)`, slot 719 `(*G_N, 8, 0)` into `G_H`;
/// `pppInitData(H, PROG, 0)` (slot 711) with the game flag `*G_L` 1
/// around it, slot 710 `(BUF, H, *G_E, 0x80000)`; the programs of `H`'s
/// words `+0x14`, `+0x18`, `+0x1c` (slot 941) into three words; then
/// three 0x110-byte blocks cleared (`memset`) and set: the first's
/// callback, counts 9 and 1s (`+0x132` .. `+0x1a0`), the second's
/// callback and 14s and its 14 entries' first words 1 (8 bytes apart, to
/// the operand `+0x1ea`), the third's callback, 3s and 1s; slot 260
/// `(0)` and `(1)` into two words. Returns 1.
pub fn setup_0666<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x07), a);
    let t = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot / 4);
    let mut st = Pushes::default();
    st.push(w(0x18));
    stores(g, img, f, &[0x1c, 0x26, 0x30]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x0c, 0xb38), fr.ecx);
    st.push_all(&[0x2000, 0x3e8, 0, g.read::<u32>(w(0x55))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x3d, 0x378), fr.ecx);
    st.push_all(&[0, 8, g.read::<u32>(w(0x6c))]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t(g, 0x5c, 0xb3c), fr.ecx).eax;
    let lock = g.read::<u32>(w(0x74));
    let held = g.read::<u32>(lock);
    g.write::<u32>(w(0x7d), h);
    st.push_all(&[0, w(0x82), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x88, 0xb1c), fr.ecx);
    let lock = g.read::<u32>(w(0x9b));
    st.push(0x80000);
    st.push(g.read::<u32>(w(0xa6)));
    g.write::<u32>(lock, held);
    st.push(g.read::<u32>(w(0xae)));
    st.push(w(0xb8));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xb3, 0xb18), fr.ecx);
    st.push(g.read::<u32>(g.read::<u32>(w(0xc5)).wrapping_add(0x14)));
    // call width: 12 (Pushes caps the words at 12)
    let p = st.call(g, t(g, 0xcd, 0xeb4), fr.ecx).eax;
    g.write::<u32>(w(0xda), p);
    let mut st = Pushes::default();
    st.push(g.read::<u32>(g.read::<u32>(w(0xdf)).wrapping_add(0x18)));
    // call width: 12 (Pushes caps the words at 12)
    let p = st.call(g, t(g, 0xea, 0xeb4), fr.ecx).eax;
    g.write::<u32>(w(0xf7), p);
    st.push(g.read::<u32>(g.read::<u32>(w(0xfc)).wrapping_add(0x1c)));
    // call width: 12 (Pushes caps the words at 12)
    let p = st.call(g, t(g, 0x104, 0xeb4), fr.ecx).eax;
    g.write::<u32>(w(0x11d), p);
    // memset(BLOCK, 0, 0x110), three times
    g.write_bytes(w(0x118), &[0u8; 0x110]);
    stores(
        g,
        img,
        f,
        &[0x132, 0x13c, 0x146, 0x150, 0x15a, 0x164, 0x16e],
    );
    stores(g, img, f, &[0x178, 0x182, 0x18c, 0x196, 0x1a0]);
    g.write_bytes(w(0x12e), &[0u8; 0x110]);
    stores(g, img, f, &[0x1b2, 0x1bc, 0x1c6]);
    let mut at = w(0x1d1);
    loop {
        g.write::<u32>(at, 1);
        at = at.wrapping_add(8);
        if at.cast_signed() >= w(0x1ea).cast_signed() {
            break;
        }
    }
    g.write_bytes(w(0x1f8), &[0u8; 0x110]);
    stores(g, img, f, &[0x206, 0x210, 0x21a, 0x224, 0x22e, 0x238]);
    // the third memset's words stay under the two calls
    let mut st = Pushes::default();
    st.push_all(&[0x110, 0, w(0x1f8), 0]);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x202, 0x410), fr.ecx).eax;
    g.write::<u32>(w(0x24d), r);
    st.push(1);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x252, 0x410), fr.ecx).eax;
    g.write::<u32>(w(0x261), r);
    Ret::eax(1)
}

/// [`setup_0666`] (the `rel32`s: `memset` three times).
pub const SETUP_0666: Sig = Sig {
    len: 623,
    hash: 0xbd36_2c66_9a8b_5c8f,
    calls: &[
        (0x122, Callee::Import("memset")),
        (0x1ab, Callee::Import("memset")),
        (0x1fd, Callee::Import("memset")),
    ],
    imports: &[],
};

/// `magic_0666`'s object placement (0x10001ca0, cdecl `(O)`, 991 bytes;
/// a cookie frame, `L(o)` = `ebp - o`, the matrices locals the slots
/// fill): slot 236 `(*G_M, n, M)` (`n` 100 when `O` lies in the first 0x80
/// bytes of the DLL's table `+0x1a`, else 7), its three rows normalised
/// (slot 232), the second row (else the first) copied to `V` (slot 325)
/// and `V.x`, `V.z` scaled by `K` (`V.y` 0); the axes from `V` and
/// `(0, 1, 0, 1)` (slot 232, two slot 266 crosses, slot 232 twice) into
/// the matrix `R = L(0x84)` (rows `L(0x24)`, `L(0x44)`, `L(0x34)`, `w` 0,
/// the last row slot 325 of `L(0xa4)` with `y` -0.01, `w` 1.0); `R` times
/// `O+0x40` (slot 229) into `L(0xe4)`, whose last row goes to `R`'s; `R`
/// scaled by `O+0x60` (slot 235's identity, slot 231), slots 230 and 231
/// on the game's matrices; slot 712 `(O, 0x30)` under the game flag;
/// when it answers, slot 713 `(O)` and the particle started on `O`
/// ([`start_part`](crate::opu_parts::start_part), the program of slot
/// 941's table `+0x28`). Returns 1.
#[allow(clippy::too_many_lines)] // the original's slot calls, in order
pub fn place_0666<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let t = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot / 4);
    let o = fr.arg(g, 0);
    let near = o.wrapping_sub(w(0x1a)) & 0xffff_ff80 == 0;
    // push edi (not passed on)
    let mut st = Pushes::default();
    st.push(l(0xd4));
    let (n, m, rows) = if near {
        (0x64, 0x3b, [0x4a, 0x5f, 0x74])
    } else {
        (7, 0x8c, [0x9b, 0xb0, 0xc5])
    };
    st.push_all(&[n, g.read::<u32>(w(m))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x2b, 0x3b0), fr.ecx);
    for (row, at) in [0xd4u32, 0xc4, 0xb4].into_iter().zip(rows) {
        st.push_all(&[l(row), l(row)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, at, 0x3a0), fr.ecx);
    }
    let src = if near { l(0xc4) } else { l(0xd4) };
    st.push_all(&[src, l(0x14)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xdd, 0x514), fr.ecx);
    let x = mul(g.read::<f32>(l(0x14)), k(g, 0xf2));
    g.write::<f32>(l(0x14), x);
    let z = mul(g.read::<f32>(l(0xc)), k(g, 0x10a));
    // push esi (not passed on)
    let mut st = Pushes::default();
    st.push_all(&[l(0x14), l(0x34)]);
    g.write::<f32>(l(0xc), z);
    g.write::<u32>(l(0x10), 0);
    for (at, v) in [
        (0x94u32, 0u32),
        (0x90, 0x3f80_0000),
        (0x8c, 0),
        (0x88, 0x3f80_0000),
    ] {
        g.write::<u32>(l(at), v);
    }
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x115, 0x3a0), fr.ecx);
    st.push_all(&[l(0x34), l(0x94), l(0x24)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x165, 0x428), fr.ecx);
    st.push_all(&[l(0x24), l(0x34), l(0x44)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x17e, 0x428), fr.ecx);
    for (v, at) in [(0x24u32, 0x190u32), (0x44, 0x1a2)] {
        st.push_all(&[l(v), l(v)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, at, 0x3a0), fr.ecx);
    }
    // the matrix `R`: the axes as rows, `w` 0
    for (from, to) in [(0x24u32, 0x84u32), (0x44, 0x74), (0x34, 0x64)] {
        for c in [0u32, 4, 8] {
            let v = g.read::<u32>(l(from - c));
            g.write::<u32>(l(to - c), v);
        }
    }
    for at in [0x78u32, 0x68, 0x58, 0x54, 0x50, 0x4c] {
        g.write::<u32>(l(at), 0);
    }
    g.write::<u32>(l(0x48), 0x3f80_0000);
    // junk read: L(0xa4)..L(0x98) never written; 0 (brief 2026-10-24)
    for c in [0u32, 4, 8, 0xc] {
        g.write::<u32>(l(0xa4 - c), 0);
    }
    st.push_all(&[l(0xa4), l(0x54)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x248, 0x514), fr.ecx);
    st.push_all(&[o.wrapping_add(0x40), l(0x84), l(0xe4)]);
    g.write::<u32>(l(0x50), w(0x26e));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x267, 0x394), fr.ecx);
    let mut st = Pushes::default();
    st.push_all(&[l(0xe4), l(0x54)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x289, 0x514), fr.ecx);
    let sc = g.read::<u32>(o.wrapping_add(0x60));
    st.push(l(0x124));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x2a2, 0x3ac), fr.ecx);
    for at in [0xfcu32, 0x110, 0x124] {
        g.write::<u32>(l(at), sc);
    }
    st.push_all(&[l(0x124), l(0x84), l(0x84)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x2ce, 0x39c), fr.ecx);
    st.push_all(&[l(0x84), g.read::<u32>(w(0x300))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x2fa, 0x398), fr.ecx);
    st.push_all(&[
        g.read::<u32>(w(0x30e)),
        g.read::<u32>(w(0x319)),
        g.read::<u32>(w(0x325)),
    ]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x313, 0x39c), fr.ecx);
    st.push_all(&[g.read::<u32>(w(0x32d)), g.read::<u32>(w(0x338))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x332, 0x398), fr.ecx);
    st.push(0x30);
    let cur = g.read::<u32>(w(0x345));
    g.write::<u32>(cur, w(0x34d));
    let lock = g.read::<u32>(w(0x352));
    st.push(o);
    let held = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x360, 0xb20), fr.ecx).eax;
    let lock = g.read::<u32>(w(0x36e));
    g.write::<u32>(lock, held);
    if r != 0 {
        let cur = g.read::<u32>(w(0x37d));
        g.write::<u32>(cur, w(0x384));
        let mut st = Pushes::default();
        st.push(o);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x389, 0xb24), fr.ecx);
        let cur = g.read::<u32>(w(0x396));
        g.write::<u32>(cur, w(0x39c));
        let h = g.read::<u32>(w(0x3a1));
        st.push(g.read::<u32>(h.wrapping_add(0x10)));
        // call width: 12 (Pushes caps the words at 12)
        let e = st.call(g, t(g, 0x3a9, 0xeb4), fr.ecx).eax;
        let program = g.read::<u32>(e.wrapping_add(0x28));
        let at = img.rel32_target(f + 0x3c4);
        crate::opu_parts::start_part(g, img, at, o, 0x1000, program, 1);
    }
    Ret::eax(1)
}

/// [`place_0666`] (the `rel32`s: the particle start and the cookie
/// check).
pub const PLACE_0666: Sig = Sig {
    len: 991,
    hash: 0x859f_443e_c253_702f,
    calls: &[
        (0x3c4, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x3d7, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0661`'s per-frame step (0x10001f50, an overlay table entry,
/// no arguments, 141 bytes): 0 while `*G_ON` is 0. Else `**G_C` = `CB`,
/// slots 723 `(*G_A)` and 724 `(*G_B)`, `**G_D = *G_E`, the cues
/// ([`cues`](crate::m0661::cues)) and slot 261 `(0x1c)`: when it answers,
/// slot 214 `(*G_S)`, else `*G_N` one more. Returns 1.
pub fn frame_0661<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot / 4);
    if g.read::<u32>(w(0x02)) == 0 {
        return Ret::eax(0);
    }
    let cur = g.read::<u32>(w(0x0d));
    let mut st = Pushes::default();
    st.push(g.read::<u32>(w(0x13)));
    g.write::<u32>(cur, w(0x19));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x1e, 0xb4c), fr.ecx);
    st.push(g.read::<u32>(w(0x31)));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x2b, 0xb50), fr.ecx);
    let d = g.read::<u32>(w(0x3f));
    let v = g.read::<u32>(w(0x44));
    g.write::<u32>(d, v);
    // two words pushed; call
    let cues = Frame {
        at: img.rel32_target(f + 0x4b),
        esp: fr.esp.wrapping_sub(0xc),
        ..*fr
    };
    crate::m0661::cues(g, img, &cues);
    st.push(0x1c);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x50, 0x414), fr.ecx).eax;
    if r == 0 {
        let n = g.read::<u32>(w(0x83)).wrapping_add(1);
        g.write::<u32>(w(0x83), n);
    } else {
        let mut st = Pushes::default();
        st.push(g.read::<u32>(w(0x6c)));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x66, 0x358), fr.ecx);
    }
    Ret::eax(1)
}

/// [`frame_0661`] (the `rel32`: the cues).
pub const FRAME_0661: Sig = Sig {
    len: 141,
    hash: 0xa348_d10a_6f2e_0150,
    calls: &[(0x4b, Callee::Sig(&crate::m0661::CUES))],
    imports: &[],
};

/// `magic_0661`'s set-up (0x10001ce0, cdecl `(a)`, 424 bytes): slot 803
/// `(0x80000, 0x10)`'s buffer into `G_B` and its next three 0x80000-byte
/// parts into three words; `*G_A = a`; `*G_N` its immediate, slot 718
/// `(NAME)`, slot 222 `(*G_N, 0, 1000, 0x2000)`, slot 719 `(*G_N, 8, 0)`
/// into `G_H`; `pppInitData(H, PROG, 0)` (slot 711) with the game flag
/// `*G_L` 1 around it, slot 710 `(BUF, H, *G_B, 0x80000)` (each call
/// seeing the earlier words); the programs of `H`'s words `+0x14`,
/// `+0x18`, `+0x1c` (slot 941) into three words. Then 14 timelines (0x110
/// bytes from the operand `+0x13e` less 4) cleared (`memset`), each with
/// its count `n` (table `+0x152`) at `+4` and `+8`, its word `+0x10c`
/// (table `+0x168`) and its first `n` events' first words 1 (8 bytes
/// apart). Returns 1.
pub fn setup_0661<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot / 4);
    let mut st = Pushes::default();
    st.push_all(&[0x10, 0x80000]);
    // call width: 12 (Pushes caps the words at 12)
    let buf = st.call(g, t(g, 0x04, 0xc8c), fr.ecx).eax;
    for (k, at) in (0u32..).zip([0x1bu32, 0x25, 0x2f, 0x39]) {
        g.write::<u32>(w(at), buf.wrapping_add(0x80000 * k));
    }
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x41), a);
    st.push(w(0x4b));
    g.write::<u32>(w(0x57), w(0x5b));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x46, 0xb38), fr.ecx);
    st.push_all(&[0x2000, 0x3e8, 0, g.read::<u32>(w(0x7a))]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x62, 0x378), fr.ecx);
    st.push_all(&[0, 8, g.read::<u32>(w(0x91))]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t(g, 0x81, 0xb3c), fr.ecx).eax;
    let lock = g.read::<u32>(w(0x99));
    let held = g.read::<u32>(lock);
    g.write::<u32>(w(0xa2), h);
    st.push_all(&[0, w(0xa7), h]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xad, 0xb1c), fr.ecx);
    let lock = g.read::<u32>(w(0xc0));
    st.push(0x80000);
    st.push(g.read::<u32>(w(0xcb)));
    g.write::<u32>(lock, held);
    st.push(g.read::<u32>(w(0xd3)));
    st.push(w(0xdd));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xd8, 0xb18), fr.ecx);
    let mut st = Pushes::default();
    for (h_at, tab, field, out) in [
        (0xeau32, 0xf5u32, 0x14u32, 0x102u32),
        (0x107, 0x10f, 0x18, 0x11c),
        (0x121, 0x129, 0x1c, 0x139),
    ] {
        let h = g.read::<u32>(w(h_at));
        st.push(g.read::<u32>(h.wrapping_add(field)));
        // call width: 12 (Pushes caps the words at 12)
        let p = st.call(g, t(g, tab, 0xeb4), fr.ecx).eax;
        g.write::<u32>(w(out), p);
    }
    let mut tl = w(0x13e);
    let mut k = 0u32;
    loop {
        let n = g.read::<u32>(k.wrapping_add(w(0x152)));
        // memset(T - 4, 0, 0x110)
        g.write_bytes(tl.wrapping_sub(4), &[0u8; 0x110]);
        let x = g.read::<u32>(k.wrapping_add(w(0x168)));
        g.write::<u32>(tl + 0x108, x);
        g.write::<u32>(tl, n);
        g.write::<u32>(tl + 4, n);
        let mut e = tl + 8;
        for _ in 0..n.cast_signed().max(0) {
            g.write::<u32>(e, 1);
            e = e.wrapping_add(8);
        }
        tl = tl.wrapping_add(0x110);
        k += 4;
        if tl.cast_signed() >= w(0x198).cast_signed() {
            break;
        }
    }
    Ret::eax(1)
}

/// [`setup_0661`] (the `rel32`: `memset`).
pub const SETUP_0661: Sig = Sig {
    len: 424,
    hash: 0x2bf9_63fb_50a2_cbdf,
    calls: &[(0x162, Callee::Import("memset"))],
    imports: &[],
};

/// `magic_0661`'s screen texture set-up (0x10001000, internal: `ecx` =
/// the record `S`; a cookie frame, `L(o)` = `ebp - o`, 963 bytes): slot
/// 719 `(*G_N, 2, 8)` gives `T`; two sizes set (`+0x42`, `+0x4c`), slot
/// 235 `(A)` and `(B)`, slot 230 `(C, S)`; the draw record's words (the
/// buffers, counts and colours from `+0x7e` on, two halfwords) and slot
/// 825 `(*G_X)` (`E`). With `T`'s 64-bit word `v`: the widths `wx = 1 <<
/// ((v >> 26) & 15)`, `wy = 1 << ((v >> 30) & 15)` (16 bits); under the
/// game flag slot 885 `(v & 0x3fff, (v >> 20) & 0x3f, 0, 0, wx, wy,
/// L(0x104))`. When it answers: the offsets `((x % (wx << 16)) >> 4) /
/// wx` and the same for `y` (signed, `*G_X`/`*G_Y`), times `K`; the four
/// colour sums (a signed and an unsigned byte each) clamped to `0..Kmax`
/// (`comiss` + `jbe`: unordered stays) times `K2` into `L(0x114)`..; under
/// the game flag slot 879 `(E, L(0x104), x, y, L(0x114), 1.0, D, wx, wy,
/// 0)`. Returns the flag pointer (0 when slot 885 does not answer).
#[allow(clippy::too_many_lines)] // the original's record, in order
pub fn texture_0661<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot / 4);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let rec = fr.ecx;
    let mut st = Pushes::default();
    st.push_all(&[8, 2, g.read::<u32>(w(0x27))]);
    // call width: 12 (Pushes caps the words at 12)
    let tex = st.call(g, t(g, 0x14, 0xb3c), fr.ecx).eax;
    st.push(w(0x36));
    stores(g, img, f, &[0x42, 0x4c]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x31, 0x3ac), fr.ecx);
    st.push(w(0x5f));
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x5a, 0x3ac), fr.ecx);
    st.push_all(&[rec, w(0x78)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x6c, 0x398), fr.ecx);
    let a = g.read::<u32>(w(0x7f));
    let mut edx = g.read::<u32>(w(0x85));
    st.push(g.read::<u32>(w(0x8b)));
    g.write::<u32>(w(0x90), a);
    let v = g.read::<u32>(w(0x95));
    g.write::<u32>(w(0x9a), v);
    let sum = g.read::<u32>(w(0x9f)).wrapping_add(g.read::<u32>(w(0xa5)));
    g.write::<u32>(w(0xab), edx);
    edx = edx.wrapping_add(sum);
    stores(g, img, f, &[0xb6, 0xc0, 0xca, 0xd4, 0xde, 0xe8, 0xf2, 0xfc]);
    g.write::<u16>(w(0x109), w(0x10d) as u16);
    g.write::<u16>(w(0x112), w(0x116) as u16);
    g.write::<u32>(w(0x120), edx);
    // call width: 12 (Pushes caps the words at 12)
    let e = st.call(g, t(g, 0xb2, 0xce4), fr.ecx).eax;
    let v = u64::from(g.read::<u32>(tex)) | (u64::from(g.read::<u32>(tex + 4)) << 32);
    let wx = u32::from(1u16 << ((v >> 26) & 15));
    let wy = u32::from(1u16 << ((v >> 30) & 15));
    let flag = g.read::<u32>(w(0x15d));
    let held = g.read::<u32>(flag);
    g.write::<u32>(flag, 1);
    let mut st = Pushes::default();
    st.push_all(&[
        l(0x104),
        wy,
        wx,
        0,
        0,
        ((v >> 20) & 0x3f) as u32,
        (v & 0x3fff) as u32,
    ]);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x19a, 0xdd4), fr.ecx).eax;
    let flag = g.read::<u32>(w(0x1b2));
    g.write::<u32>(flag, held);
    if r == 0 {
        return Ret::eax(0);
    }
    // `(n % (d << 16)) >> 4` (arithmetic) divided by `d`, signed
    let offset = |n: u32, d: u32| {
        let m = n.cast_signed().wrapping_rem((d << 16).cast_signed()) >> 4;
        m.wrapping_div(d.cast_signed()).cast_unsigned()
    };
    let x = mul(cvt(offset(g.read::<u32>(w(0x1ca)), wx)), k(g, 0x204));
    let y = mul(cvt(offset(g.read::<u32>(w(0x1f1)), wy)), k(g, 0x231));
    let byte_sum = |g: &G, sb: u32, ub: u32| {
        let s8 = i32::from(g.read::<i8>(w(sb))).cast_unsigned();
        let u8v = u32::from(g.read::<u8>(w(ub)));
        add(cvt(s8), cvt(u8v))
    };
    let hi = k(g, 0x2ae);
    let k2 = k(g, 0x2c8);
    for (n, (sb, ub)) in (0u32..).zip([
        (0x217u32, 0x222u32),
        (0x238, 0x251),
        (0x260, 0x274),
        (0x283, 0x294),
    ]) {
        let c = byte_sum(g, sb, ub);
        // `comiss` + `jbe`: an unordered sum stays
        let c = if 0.0 > c {
            0.0
        } else if c > hi {
            hi
        } else {
            c
        };
        g.write::<f32>(l(0x114 - 4 * n), mul(c, k2));
    }
    let flag = g.read::<u32>(w(0x324));
    g.write::<u32>(flag, 1);
    let mut st = Pushes::default();
    st.push_all(&[
        0,
        cvt(wy).to_bits(),
        cvt(wx).to_bits(),
        w(0x35b),
        w(0x369),
        l(0x114),
        y.to_bits(),
        x.to_bits(),
        l(0x104),
        e,
    ]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x384, 0xdbc), fr.ecx);
    let flag = g.read::<u32>(w(0x3a3));
    g.write::<u32>(flag, held);
    Ret::eax(flag)
}

/// [`texture_0661`] (the `rel32`: the cookie check).
pub const TEXTURE_0661: Sig = Sig {
    len: 963,
    hash: 0x45a5_63ec_3f5c_770e,
    calls: &[(0x3bb, Callee::Any)],
    imports: &[],
};

/// `magic_0661`'s screen texture entry (0x10001530, an overlay table
/// entry, no arguments; a cookie frame, 69 bytes): slot 210 `(*G_S,
/// L(0x44))` fills the local record, which [`texture_0661`] takes.
/// Returns 1.
pub fn texture_entry_0661<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let local = ebp.wrapping_sub(0x44);
    let s = g.read::<u32>(w(0x1b));
    // call width: 2
    g.call(slot_target(g, w(0x15), 0x348 / 4), fr.ecx, &[s, local]);
    let callee = Frame {
        at: img.rel32_target(f + 0x2e),
        esp: local.wrapping_sub(4),
        ecx: local,
        ..*fr
    };
    texture_0661(g, img, &callee);
    Ret::eax(1)
}

/// [`texture_entry_0661`] (the `rel32`s: the texture set-up and the
/// cookie check).
pub const TEXTURE_ENTRY_0661: Sig = Sig {
    len: 69,
    hash: 0x6e4f_32e4_0744_5c23,
    calls: &[(0x2e, Callee::Sig(&TEXTURE_0661)), (0x3d, Callee::Any)],
    imports: &[],
};

/// `magic_0489`'s sprite spawn (0x10001000, internal: `ecx` = the
/// sprite pool, cdecl `(pos, vel)`; an `ebp` frame, 508 bytes): a free
/// sprite `S` ([`sprite_take`](crate::s4_misc::sprite_take), the resource
/// `*G_R`; none: returns 0). With `u(k) = (slot 324 & 0x7fff) / K - K1`
/// (a fresh draw each, `K`/`K1` read at each one's operands): the speed
/// `s = u K2 + K3`, `S+0x28` 16, `vel * s` into `S+0x14..+0x1c` (after the
/// position); the position `S+0x4..+0xc` = `pos + u K2 D` (in `f64`, one
/// rounding); `S+0x54` = `(r / K) K4 + D2` (`f64`, `r` the fifth draw),
/// `S+0x5c..+0x64` 0x80 and `S+0x68` 1.0. Returns the last draw `& 0x7fff`.
#[allow(clippy::similar_names)] // the original's three axes
pub fn spawn_0489<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let kd = |g: &G, o: u32| f64::from_bits(g.read::<u64>(w(o)));
    let take = Frame {
        at: img.rel32_target(f + 0xe),
        esp: fr.esp.wrapping_sub(0x18),
        edx: g.read::<u32>(w(0x05)),
        ..*fr
    };
    let sp = crate::s4_misc::sprite_take(g, img, &take).eax;
    if sp == 0 {
        return Ret::eax(0);
    }
    let draw = |g: &mut G, tab: u32| {
        // call width: 0
        g.call(slot_target(g, w(tab), 0x510 / 4), fr.ecx, &[]).eax & 0x7fff
    };
    let vel = fr.arg(g, 1);
    let r = draw(g, 0x1e);
    let u = sub(div(cvt(r), k(g, 0x3e)), k(g, 0x4a));
    g.write::<u32>(sp + 0x28, 0x10);
    let speed = add(mul(u, k(g, 0x59)), k(g, 0x61));
    let v: [f32; 3] = core::array::from_fn(|c| mul(g.read::<f32>(vel + 4 * c as u32), speed));
    let pos = fr.arg(g, 0);
    for (c, (tab, k1, k2, k3, d)) in [
        (0x82u32, 0xaau32, 0xb7u32, 0xbfu32, 0xceu32),
        (0xaf, 0xff, 0x107, 0x10f, 0x11f),
        (0xf7, 0x150, 0x158, 0x160, 0x170),
    ]
    .into_iter()
    .enumerate()
    {
        let r = draw(g, tab);
        let u = mul(sub(div(cvt(r), k(g, k1)), k(g, k2)), k(g, k3));
        let p = f64::from(g.read::<f32>(pos + 4 * c as u32));
        let x = (f64::from(u) * kd(g, d) + p) as f32;
        g.write::<f32>(sp + 4 + 4 * c as u32, x);
    }
    for (c, x) in v.into_iter().enumerate() {
        g.write::<f32>(sp + 0x14 + 4 * c as u32, x);
    }
    let r = draw(g, 0x148);
    for at in [0x5cu32, 0x60, 0x64] {
        g.write::<u32>(sp + at, 0x80);
    }
    g.write::<u32>(sp + 0x68, 0x3f80_0000);
    let x = mul(div(cvt(r), k(g, 0x1c1)), k(g, 0x1d0));
    g.write::<f32>(sp + 0x54, (f64::from(x) + kd(g, 0x1ea)) as f32);
    Ret::eax(r)
}

/// [`spawn_0489`] (the `rel32`: the sprite take).
pub const SPAWN_0489: Sig = Sig {
    len: 508,
    hash: 0x48fe_d859_4294_f9c0,
    calls: &[(0xe, Callee::Sig(&crate::s4_misc::SPRITE_TAKE))],
    imports: &[],
};

/// `magic_0489`'s second sprite spawn (0x10001430, internal: `ecx` = the
/// pool, cdecl `(pos, vel)`; 587 bytes): as [`spawn_0489`] with `u =
/// (slot 324 & 0x7fff) / K - K1`: `S+0x28`, `+0x24` 0x18 and `+0x2c` 12;
/// the speed `(u K2) K3 + K3`; the position `pos + u K2` (`f32`); `S+0x58`
/// = slot 403 `((r / K) K4)` (ST0); seven colour words; `S+0x8c` = `((r'
/// / K) K5 + K6) K7` and `S+0x90` that times `K2`. Returns the last draw
/// `& 0x7fff`.
#[allow(clippy::similar_names)] // the original's three axes
pub fn spawn2_0489<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let take = Frame {
        at: img.rel32_target(f + 0xe),
        esp: fr.esp.wrapping_sub(0x18),
        edx: g.read::<u32>(w(0x05)),
        ..*fr
    };
    let sp = crate::s4_misc::sprite_take(g, img, &take).eax;
    if sp == 0 {
        return Ret::eax(0);
    }
    let draw = |g: &mut G, tab: u32| {
        // call width: 0
        g.call(slot_target(g, w(tab), 0x510 / 4), fr.ecx, &[]).eax & 0x7fff
    };
    let vel = fr.arg(g, 1);
    let r = draw(g, 0x1e);
    let u = sub(div(cvt(r), k(g, 0x3e)), k(g, 0x4a));
    for (at, v) in [(0x28u32, 0x18u32), (0x24, 0x18), (0x2c, 0xc)] {
        g.write::<u32>(sp + at, v);
    }
    let speed = add(mul(mul(u, k(g, 0x67)), k(g, 0x6f)), k(g, 0x77));
    let v: [f32; 3] = core::array::from_fn(|c| mul(g.read::<f32>(vel + 4 * c as u32), speed));
    let pos = fr.arg(g, 0);
    for (c, (tab, k1, k2, k3)) in [
        (0x98u32, 0xc0u32, 0xcdu32, 0xd5u32),
        (0xc5, 0xff, 0x107, 0x10f),
        (0xf7, 0x13a, 0x142, 0x14a),
    ]
    .into_iter()
    .enumerate()
    {
        let r = draw(g, tab);
        let u = mul(sub(div(cvt(r), k(g, k1)), k(g, k2)), k(g, k3));
        let x = add(u, g.read::<f32>(pos + 4 * c as u32));
        g.write::<f32>(sp + 4 + 4 * c as u32, x);
    }
    for (c, x) in v.into_iter().enumerate() {
        g.write::<f32>(sp + 0x14 + 4 * c as u32, x);
    }
    let r = draw(g, 0x132);
    let a = mul(div(cvt(r), k(g, 0x193)), k(g, 0x1a1));
    // push ecx; movss [esp], a; call
    let t = g.call(slot_target(g, w(0x18b), 0x64c / 4), fr.ecx, &[a.to_bits()]);
    g.write::<f32>(sp + 0x58, crate::m0018::st0_f32(t.st0));
    for (at, v) in [
        (0x6cu32, 0x81u32),
        (0x70, 0x86),
        (0x74, 0x8d),
        (0x78, 0x8),
        (0x7c, 0x71),
        (0x80, 0x84),
        (0x84, 0xa7),
        (0x88, 0),
    ] {
        g.write::<u32>(sp + at, v);
    }
    let r = draw(g, 0x1bb);
    let x = mul(
        add(mul(div(cvt(r), k(g, 0x212)), k(g, 0x21a)), k(g, 0x222)),
        k(g, 0x22a),
    );
    g.write::<f32>(sp + 0x8c, x);
    g.write::<f32>(sp + 0x90, mul(x, k(g, 0x23a)));
    Ret::eax(r)
}

/// [`spawn2_0489`] (the `rel32`: the sprite take).
pub const SPAWN2_0489: Sig = Sig {
    len: 587,
    hash: 0x1904_fcbb_2186_644f,
    calls: &[(0xe, Callee::Sig(&crate::s4_misc::SPRITE_TAKE))],
    imports: &[],
};

/// `magic_0489`'s spray step (0x10001260, an overlay table entry, cdecl
/// `(pool)`; a /GS frame aligned to 8, `B` = `((esp - 4) & !7) - 0x38`
/// after the saved registers, 450 bytes): slot 237 `(*G_A, 0, 0, P)`
/// puts the point `P = B+0x24`; the way `d = G_T - G_F` (three globals
/// each), its length `l` (the `f64` square root of `dy dy + dx dx + dz
/// dz`, rounded) and the velocity `V = B+0x14` = `d (K / l) K2` (`d K2`
/// when `l` is 0; `ucomiss` + `lahf`/`jnp`: an unordered `l` divides);
/// five sprites spawned at `P` with `V` ([`spawn_0489`], `ecx` the pool,
/// their words left on the stack); each live sprite's `+0x68` = the
/// table word `T[-(+0x28)]` (operand `+0x17c`); the sprites stepped
/// ([`sprites_step`](crate::s6_misc::sprites_step)) and their blocks'
/// frames ([`block_sprite_frames`](crate::s9_misc::block_sprite_frames),
/// `dt` 0) into `*G_R`. Returns 1.
pub fn spray_0489<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let pool = fr.arg(g, 0);
    let (pos, vel) = (b + 0x24, b + 0x14);
    let a = g.read::<u32>(w(0x2f));
    // call width: 4
    g.call(slot_target(g, w(0x1f), 0x3b4 / 4), fr.ecx, &[a, 0, 0, pos]);
    let dx = sub(k(g, 0x39), k(g, 0x41));
    let dy = sub(k(g, 0x49), k(g, 0x51));
    let dz = sub(k(g, 0x59), k(g, 0x61));
    let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    // `_libm_sse2_sqrt_precise`, then `cvtsd2ss`
    let l = f64::from(sq).sqrt() as f32;
    let (mut x, mut y, mut z) = (dx, dy, dz);
    // `ucomiss l, 0`: `jnp` takes only the ordered equal case
    if l != 0.0 || l.is_nan() {
        let q = div(k(g, 0xc6), l);
        (x, y, z) = (mul(dx, q), mul(dy, q), mul(dz, q));
    }
    let k2 = k(g, 0xec);
    for (c, v) in [x, y, z].into_iter().enumerate() {
        g.write::<f32>(vel + 4 * c as u32, mul(v, k2));
    }
    for (n, at) in (1u32..).zip([0x11bu32, 0x12c, 0x13d, 0x14e, 0x15f]) {
        // push V; push P (the earlier calls' words stay); call
        let esp = b.wrapping_sub(8 * n);
        g.write::<u32>(esp, pos);
        g.write::<u32>(esp + 4, vel);
        let spawn = Frame {
            at: img.rel32_target(f + at),
            esp: esp.wrapping_sub(4),
            ecx: pool,
            ..*fr
        };
        spawn_0489(g, img, &spawn);
    }
    let n = g.read::<i32>(pool);
    let mut sp = g.read::<u32>(pool + 4);
    for _ in 0..n.max(0) {
        if g.read::<u32>(sp) != 0 {
            let i = g.read::<u32>(sp + 0x28);
            let v = g.read::<u32>(w(0x17c).wrapping_sub(i.wrapping_mul(4)));
            g.write::<u32>(sp + 0x68, v);
        }
        sp = sp.wrapping_add(0x94);
    }
    let step = Frame {
        at: img.rel32_target(f + 0x195),
        esp: b.wrapping_sub(4),
        ecx: pool,
        ..*fr
    };
    crate::s6_misc::sprites_step(g, img, &step);
    // push ecx; xorps xmm3, xmm3; call
    let frames = Frame {
        at: img.rel32_target(f + 0x1a0),
        esp: b.wrapping_sub(8),
        ecx: pool,
        xmm: [0, 0, 0, 0],
        ..*fr
    };
    let r = crate::s9_misc::block_sprite_frames(g, img, &frames).eax;
    g.write::<u32>(w(0x1ac), r);
    Ret::eax(1)
}

/// [`spray_0489`] (the `rel32`s: the square root's thunk, five spawns,
/// the sprites' step, their frames and the cookie check).
pub const SPRAY_0489: Sig = Sig {
    len: 450,
    hash: 0x053a_931f_0479_d46b,
    calls: &[
        (0x9f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x11b, Callee::Sig(&SPAWN_0489)),
        (0x12c, Callee::Sig(&SPAWN_0489)),
        (0x13d, Callee::Sig(&SPAWN_0489)),
        (0x14e, Callee::Sig(&SPAWN_0489)),
        (0x15f, Callee::Sig(&SPAWN_0489)),
        (0x195, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x1a0, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES)),
        (0x1ba, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0489`'s second spray step (0x100016e0, an overlay table
/// entry, cdecl `(pool)`; the frame of [`spray_0489`], 509 bytes): the
/// point `P` (slot 237) and the way `d` normalised in place (`K / l`,
/// not when `l` is 0); while slot 261 `(2)` is 0 `P += d K1`; `V = d
/// K2`; when slot 261 `(2)` answers, two sprites spawned
/// ([`spawn2_0489`]) and their words popped; a third spawned; the
/// sprites stepped and their blocks' frames (`dt` 0) into `*G_R`.
/// Returns 1.
pub fn spray2_0489<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let pool = fr.arg(g, 0);
    let (pos, vel) = (b + 0x24, b + 0x14);
    let a = g.read::<u32>(w(0x2e));
    // call width: 4
    g.call(slot_target(g, w(0x1e), 0x3b4 / 4), fr.ecx, &[a, 0, 0, pos]);
    let mut d = [
        sub(k(g, 0x38), k(g, 0x40)),
        sub(k(g, 0x48), k(g, 0x50)),
        sub(k(g, 0x58), k(g, 0x60)),
    ];
    let sq = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
    // `_libm_sse2_sqrt_precise`, then `cvtsd2ss`
    let l = f64::from(sq).sqrt() as f32;
    // `ucomiss l, 0`: `jnp` takes only the ordered equal case
    if l != 0.0 || l.is_nan() {
        let q = div(k(g, 0xb9), l);
        d = d.map(|x| mul(x, q));
    }
    // call width: 1
    let r = g.call(slot_target(g, w(0xf2), 0x414 / 4), fr.ecx, &[2]).eax;
    if r == 0 {
        let k1 = k(g, 0x11d);
        for (c, x) in d.into_iter().enumerate() {
            let at = pos + 4 * c as u32;
            let v = add(mul(x, k1), g.read::<f32>(at));
            g.write::<f32>(at, v);
        }
    }
    let k2 = k(g, 0x15e);
    for (c, x) in d.into_iter().enumerate() {
        g.write::<f32>(vel + 4 * c as u32, mul(x, k2));
    }
    // call width: 1
    let r = g
        .call(slot_target(g, w(0x163), 0x414 / 4), fr.ecx, &[2])
        .eax;
    let spawn = |g: &mut G, at: u32, esp: u32| {
        g.write::<u32>(esp, pos);
        g.write::<u32>(esp + 4, vel);
        let callee = Frame {
            at: img.rel32_target(f + at),
            esp: esp.wrapping_sub(4),
            ecx: pool,
            ..*fr
        };
        spawn2_0489(g, img, &callee);
    };
    if r != 0 {
        spawn(g, 0x1a3, b.wrapping_sub(8));
        spawn(g, 0x1b4, b.wrapping_sub(0x10));
    }
    spawn(g, 0x1c8, b.wrapping_sub(8));
    let step = Frame {
        at: img.rel32_target(f + 0x1cf),
        esp: b.wrapping_sub(0xc),
        ecx: pool,
        ..*fr
    };
    crate::s6_misc::sprites_step(g, img, &step);
    // add esp, 4 (`V` stays); xorps xmm3, xmm3; call
    let frames = Frame {
        at: img.rel32_target(f + 0x1dc),
        esp: b.wrapping_sub(8),
        ecx: pool,
        xmm: [0, 0, 0, 0],
        ..*fr
    };
    let r = crate::s9_misc::block_sprite_frames(g, img, &frames).eax;
    g.write::<u32>(w(0x1e8), r);
    Ret::eax(1)
}

/// [`spray2_0489`] (the `rel32`s: the square root's thunk, three spawns,
/// the sprites' step, their frames and the cookie check).
pub const SPRAY2_0489: Sig = Sig {
    len: 509,
    hash: 0x4856_2c7b_755e_f0d0,
    calls: &[
        (0x9e, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1a3, Callee::Sig(&SPAWN2_0489)),
        (0x1b4, Callee::Sig(&SPAWN2_0489)),
        (0x1c8, Callee::Sig(&SPAWN2_0489)),
        (0x1cf, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x1dc, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES)),
        (0x1f5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0450`'s caster object start (0x10005b40, internal, no
/// arguments used; a cookie frame, 689 bytes): with the block `B = *G_B`
/// the object `O = B+0x260` started
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `*K`,
/// index 2, owners 0) at `B+0x1c10`, `O+0xd4 -= K1`, its yaw `O+0xc4` =
/// `B+0x1ee0`; the angles `B+0x1b90..` `(0, yaw, 0, 0)`; the matrix `M =
/// B+0x1b50` (slot 235's identity) scaled by slot 277 `(*G_S)` times `K2`
/// and by slots 15, 16, 17 `(*G_T)` on its diagonal, turned by slots 330,
/// 328, 329 (the angles, each `movss [esp]` over the earlier call's last
/// word), `B+0x1b80` and `B+0x1ba0` the point. Then slot 210 `(*G_T, M)`
/// (with `M` 0, a block at -0x1b50, slots 217 and 255 `(a, L(0x14))`
/// for the actor `a` below 0x20 when `*G_F` is 0), slot 211 `(*G_T, M)`
/// and a scale task
/// ([`scale_task`](crate::s4_misc::scale_task): `ecx` `B+0x1dd0`, scales
/// `K3` and `K4`, count 0). Returns the task's `eax`.
pub fn caster_start_0450<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, slot: u32| slot_target(g, w(0xa7), slot / 4);
    let st0 = |o: crate::Out| crate::m0018::st0_f32(o.st0);
    let e = fr.esp;
    let b = g.read::<u32>(w(0x1b));
    let o = b.wrapping_add(0x260);
    let at = img.rel32_target(f + 0x31);
    crate::s4_misc::start_object_at(g, img, at, o, k(g, 0x14), 2, 0, 0);
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(b.wrapping_add(0x1c10 + c));
        g.write::<u32>(o + 0xd0 + c, v);
    }
    let y = sub(g.read::<f32>(o + 0xd4), k(g, 0x77));
    let yaw = g.read::<u32>(b.wrapping_add(0x1ee0));
    g.write::<u32>(o + 0xc4, yaw);
    let m = b.wrapping_add(0x1b50);
    g.write::<f32>(o + 0xd4, y);
    g.write::<u32>(b.wrapping_add(0x1b90), 0);
    let yaw = g.read::<u32>(b.wrapping_add(0x1ee0));
    g.write::<u32>(b.wrapping_add(0x1b94), yaw);
    g.write::<u32>(b.wrapping_add(0x1b98), 0);
    g.write::<u32>(b.wrapping_add(0x1b9c), 0);
    // the start's `(2, 0, ecx, 0)` stay under the calls (the `ecx` word
    // and below not passed on)
    let mut st = Pushes::default();
    st.push_all(&[0, 2, m]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x3ac), fr.ecx);
    st.push(g.read::<u32>(w(0xce)));
    // call width: 12 (Pushes caps the words at 12)
    let sc = mul(
        st0(st.call(g, slot_target(g, w(0xc8), 0x454 / 4), fr.ecx)),
        k(g, 0xe6),
    );
    for c in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + c, sc);
    }
    let tt = g.read::<u32>(w(0xff));
    st.push(tt);
    // call width: 12 (Pushes caps the words at 12)
    let a = st0(st.call(g, slot_target(g, w(0xeb), 0x3c / 4), fr.ecx));
    let v = mul(g.read::<f32>(m), a);
    st.push(tt);
    g.write::<f32>(m, v);
    // call width: 12 (Pushes caps the words at 12)
    let bb = st0(st.call(g, slot_target(g, w(0x116), 0x40 / 4), fr.ecx));
    let v = mul(bb, g.read::<f32>(m + 0x14));
    st.push(tt);
    g.write::<f32>(m + 0x14, v);
    // call width: 12 (Pushes caps the words at 12)
    let c = st0(st.call(g, slot_target(g, w(0x125), 0x44 / 4), fr.ecx));
    let v = mul(g.read::<f32>(m + 0x28), c);
    g.write::<f32>(m + 0x28, v);
    // add esp, 0x20: the start's first word stays, each angle written over it
    for (slot, ang, tab) in [
        (0x528u32, 0x1b98u32, 0x14fu32),
        (0x520, 0x1b90, 0x183),
        (0x524, 0x1b94, 0x19a),
    ] {
        let a = g.read::<u32>(b.wrapping_add(ang));
        let mut st = Pushes::default();
        st.push_all(&[a, m, m]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot_target(g, w(tab), slot / 4), fr.ecx);
    }
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(b.wrapping_add(0x1c10 + c));
        g.write::<u32>(b.wrapping_add(0x1b80 + c), v);
    }
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(b.wrapping_add(0x1c10 + c));
        g.write::<u32>(b.wrapping_add(0x1ba0 + c), v);
    }
    let tt = g.read::<u32>(w(0x211));
    if m != 0 {
        // call width: 2
        g.call(slot_target(g, w(0x222), 0x348 / 4), fr.ecx, &[tt, m]);
    } else if g.read::<u32>(w(0x235)) == 0 {
        let actor = g.read::<u32>(w(0x23e));
        if actor.cast_signed() < 0x20 {
            let local = e.wrapping_sub(0x18);
            // call width: 2
            g.call(slot_target(g, w(0x24c), 0x364 / 4), fr.ecx, &[actor, local]);
            // call width: 2 (the first call's words under it)
            g.call(
                slot_target(g, w(0x25e), 0x3fc / 4),
                fr.ecx,
                &[actor, local, actor, local],
            );
        }
    }
    // call width: 2
    g.call(slot_target(g, w(0x274), 0x34c / 4), fr.ecx, &[tt, m]);
    // push 0; call: the count on the stack over slot 211's words
    g.write::<u32>(e.wrapping_sub(0x34), 0);
    g.write::<u32>(e.wrapping_sub(0x30), tt);
    g.write::<u32>(e.wrapping_sub(0x2c), m);
    let task = Frame {
        at: img.rel32_target(f + 0x299),
        esp: e.wrapping_sub(0x38),
        ecx: b.wrapping_add(0x1dd0),
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x28c))),
            u128::from(g.read::<u32>(w(0x284))),
            0,
        ],
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &task)
}

/// [`caster_start_0450`] (the `rel32`s: the object start, the scale task
/// and the cookie check).
pub const CASTER_START_0450: Sig = Sig {
    len: 689,
    hash: 0x69ac_e0c7_7dda_877b,
    calls: &[
        (0x31, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x299, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x2a9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0450`'s caster glide step (0x10007d50, an overlay table entry,
/// no arguments; an `ebp` frame, 609 bytes): with the block `B = *G_B`
/// and its counter `n = B+0x1ef8`: nothing while `n` is at most 0
/// (returns `n`); else `n` one less, the eased fraction `s`
/// ([`ease_sample`](crate::sb_misc::ease_sample) of the keys `+0x39`, 8
/// of them, stride 1, at `(n / K) K1`) mixes the points `B+0x1c10` and
/// `B+0x1c30` into `B+0x1ba0` (`w` 1.0); the yaw
/// ([`ease_angle`](crate::sb_misc::ease_angle), 12 keys at `+0xcd`, at
/// `(n / K) K2`) wrapped once into `B+0x1b90`; the matrix `M = B+0x1b50`
/// rebuilt as in [`caster_start_0450`] (slots 235, 277, 15-17, 330, 328,
/// 329). Once `n` is 0 the scale task
/// ([`scale_task`](crate::s4_misc::scale_task): `ecx` `B+0x1e08`, scales
/// `K3`, `K4`, count 0) starts and its `eax` is returned, else slot
/// 329's.
#[allow(clippy::too_many_lines)] // the original's calls, in order
pub fn glide_0450<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let st0 = |o: crate::Out| crate::m0018::st0_f32(o.st0);
    let e = fr.esp;
    let b = g.read::<u32>(w(0x07));
    let n = g.read::<u32>(b.wrapping_add(0x1ef8));
    if n.cast_signed() <= 0 {
        return Ret::eax(n);
    }
    let n = n.wrapping_sub(1);
    let t = mul(div(cvt(n), k(g, 0x27)), k(g, 0x34));
    g.write::<u32>(b.wrapping_add(0x1ef8), n);
    // push 1 (the stride); call
    g.write::<u32>(e.wrapping_sub(0x10), 1);
    let at = img.rel32_target(f + 0x44);
    let s = crate::sb_misc::ease_sample(g, img, at, e.wrapping_sub(0x14), w(0x39), 8, t);
    let km = sub(k(g, 0x4c), s);
    let p = |g: &G, o: u32| g.read::<f32>(b.wrapping_add(o));
    let x = add(mul(p(g, 0x1c30), s), mul(p(g, 0x1c10), km));
    let y = add(mul(p(g, 0x1c34), s), mul(p(g, 0x1c14), km));
    let z = add(mul(p(g, 0x1c38), s), mul(p(g, 0x1c18), km));
    g.write::<f32>(b.wrapping_add(0x1ba0), x);
    g.write::<f32>(b.wrapping_add(0x1ba4), y);
    g.write::<u32>(b.wrapping_add(0x1bac), 0x3f80_0000);
    g.write::<u32>(e.wrapping_sub(0x14), 1);
    g.write::<f32>(b.wrapping_add(0x1ba8), z);
    let n = g.read::<u32>(b.wrapping_add(0x1ef8));
    let t = mul(div(cvt(n), k(g, 0xe8)), k(g, 0xf0));
    let at = img.rel32_target(f + 0xf5);
    let mut a = crate::sb_misc::ease_angle(g, img, at, e.wrapping_sub(0x18), w(0xcd), 0xc, t);
    // `comiss` + `jbe`: an unordered yaw stays
    if a > k(g, 0x102) {
        a = sub(a, k(g, 0x10c));
    } else if k(g, 0x116) > a {
        a = add(a, k(g, 0x123));
    }
    g.write::<f32>(b.wrapping_add(0x1b90), a);
    let m = b.wrapping_add(0x1b50);
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x128), 0x3ac / 4), fr.ecx);
    st.push(g.read::<u32>(w(0x14c)));
    let sc = st0(st.call(g, slot_target(g, w(0x146), 0x454 / 4), fr.ecx));
    let sc = mul(sc, k(g, 0x164));
    for c in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + c, sc);
    }
    let tt = g.read::<u32>(w(0x17d));
    st.push(tt);
    // call width: 12 (Pushes caps the words at 12)
    let a1 = st0(st.call(g, slot_target(g, w(0x169), 0x3c / 4), fr.ecx));
    let v = mul(g.read::<f32>(m), a1);
    st.push(tt);
    g.write::<f32>(m, v);
    // call width: 12 (Pushes caps the words at 12)
    let a2 = st0(st.call(g, slot_target(g, w(0x194), 0x40 / 4), fr.ecx));
    let v = mul(a2, g.read::<f32>(m + 0x14));
    st.push(tt);
    g.write::<f32>(m + 0x14, v);
    // call width: 12 (Pushes caps the words at 12)
    let a3 = st0(st.call(g, slot_target(g, w(0x1a3), 0x44 / 4), fr.ecx));
    let v = mul(g.read::<f32>(m + 0x28), a3);
    g.write::<f32>(m + 0x28, v);
    // add esp, 0x10: `M`'s word stays, each angle written over it
    let mut r = 0;
    for (slot, ang, tab) in [
        (0x528u32, 0x1b98u32, 0x1cdu32),
        (0x520, 0x1b90, 0x1f9),
        (0x524, 0x1b94, 0x218),
    ] {
        let a = g.read::<u32>(b.wrapping_add(ang));
        let mut st = Pushes::default();
        st.push_all(&[a, m, m]);
        // call width: 12 (Pushes caps the words at 12)
        r = st.call(g, slot_target(g, w(tab), slot / 4), fr.ecx).eax;
    }
    if g.read::<u32>(b.wrapping_add(0x1ef8)) != 0 {
        return Ret::eax(r);
    }
    // push 0; call
    g.write::<u32>(e.wrapping_sub(0x10), 0);
    let task = Frame {
        at: img.rel32_target(f + 0x255),
        esp: e.wrapping_sub(0x14),
        ecx: b.wrapping_add(0x1e08),
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x248))),
            u128::from(g.read::<u32>(w(0x240))),
            0,
        ],
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &task)
}

/// [`glide_0450`] (the `rel32`s: the eased sample, the eased angle and
/// the scale task).
pub const GLIDE_0450: Sig = Sig {
    len: 609,
    hash: 0xd4e0_7508_e773_b2d6,
    calls: &[
        (0x44, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0xf5, Callee::Sig(&crate::sb_misc::EASE_ANGLE)),
        (0x255, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
    ],
    imports: &[],
};

/// `magic_0450`'s caster rise step (0x10007980, an overlay table entry,
/// no arguments, 703 bytes): with the block `B = *G_B` and its counter
/// `n = B+0x1ef0`: while `n` is above 0 the sparks
/// ([`sparks_p034`](crate::r11_misc::sparks_p034)) and `n` one less, the
/// fraction `s = n / K` (else `s` = `K0`, 1.0); the point `B+0x1c00` =
/// `B+0x1c30 s + B+0x1c40 (K0 - s)` (`w` 1.0). Unless `n` is now 0 that
/// is all (returns the sparks' `eax`, or the entry's). Else the object
/// `O = B+0x130` leaves the DLL's list
/// ([`list_remove`](crate::l5_objhost::list_remove); when it was there,
/// `*G_C = CB` and slot 713 `(O)`), its halfword `+0x124` loses bit 15;
/// the object `O2 = B+0x390` starts at `B+0x1c40`
/// ([`start_object_at`](crate::s4_misc::start_object_at): index 3,
/// `owner_b` the callback `+0x10d`, scale `*K2`), its yaw `B+0x1ee0`; the
/// record `R = B+0x1c60` gets its colour bytes and counts; a ring node
/// (callback `+0x222`, `+0x14 = R`) goes last on the task list; the actor
/// `a = (*G_A)[1]`: slot 296 `(a, 0)` outside a battle while `a` is below
/// 0x20, else slot 257 `(*(G_T + 4 a), 1)`; `n` one less. Returns the
/// last slot's `eax`.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn rise_0450<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0x0b));
    let at = |o: u32| b.wrapping_add(o);
    let mut eax = fr.eax;
    let (s, one) = if g.read::<i32>(at(0x1ef0)) > 0 {
        // push edi; call
        let sparks = Frame {
            at: img.rel32_target(f + 0x19),
            esp: fr.esp.wrapping_sub(8),
            ..*fr
        };
        eax = crate::r11_misc::sparks_p034(g, img, &sparks).eax;
        let n = g.read::<u32>(at(0x1ef0)).wrapping_sub(1);
        g.write::<u32>(at(0x1ef0), n);
        (div(cvt(n), k(g, 0x3a)), k(g, 0x2f))
    } else {
        (k(g, 0x04), k(g, 0x04))
    };
    let km = sub(one, s);
    let p = |g: &G, o: u32| g.read::<f32>(at(o));
    let x = add(mul(p(g, 0x1c30), s), mul(km, p(g, 0x1c40)));
    let y = add(mul(p(g, 0x1c34), s), mul(km, p(g, 0x1c44)));
    let z = add(mul(p(g, 0x1c38), s), mul(km, p(g, 0x1c48)));
    g.write::<f32>(at(0x1c00), x);
    g.write::<f32>(at(0x1c04), y);
    g.write::<u32>(at(0x1c0c), 0x3f80_0000);
    g.write::<f32>(at(0x1c08), z);
    if g.read::<u32>(at(0x1ef0)) != 0 {
        return Ret::eax(eax);
    }
    let o = at(0x130);
    let removed = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0xd4), w(0xcf), o);
    if removed != 0 {
        let c = g.read::<u32>(w(0xdd));
        g.write::<u32>(c, w(0xe4));
        // call width: 1 (ebx, esi saved under it)
        g.call(slot_target(g, w(0xe9), 0xb24 / 4), fr.ecx, &[o]);
    }
    let h = g.read::<u16>(o + 0x124) & 0x7fff;
    g.write::<u16>(o + 0x124, h);
    let o2 = at(0x390);
    let start = img.rel32_target(f + 0x11f);
    crate::s4_misc::start_object_at(g, img, start, o2, k(g, 0x108), 3, 0, w(0x10d));
    let count = g.read::<u16>(w(0x12c));
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(at(0x1c40 + c));
        g.write::<u32>(o2 + 0xd0 + c, v);
    }
    let yaw = g.read::<u32>(at(0x1ee0));
    g.write::<u32>(o2 + 0xc4, yaw);
    let r = at(0x1c60);
    g.write::<u32>(r + 0x81, 0xff80_ffff);
    g.write::<u8>(r + 0x80, 0xff);
    g.write::<u16>(r + 0x85, 0xffff);
    g.write::<u8>(r + 0x87, 0);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x14);
    g.write::<u32>(r + 0x8c, 0x14);
    g.write::<u32>(r + 0x94, 0);
    let node = if count == 0 {
        // none free: the original writes through 0 (not run)
        0
    } else {
        let node = g.read::<u32>(w(0x1cc));
        crate::r13_misc::pool_pop(g, img, f, node, (0x12c, 0x1e9, 0x1f1, 0x222, 0x22d));
        node
    };
    let any = g.read::<u32>(w(0x233));
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u32>(node.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x25a), node);
    } else {
        let tail = g.read::<u32>(w(0x247));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    let battle = g.read::<u32>(w(0x263)) != 0;
    let actor = g.read::<u32>(w(0x26a));
    g.write::<u32>(w(0x270), node);
    let a = u32::from(g.read::<u8>(actor.wrapping_add(1)));
    let eax = if !battle && a.cast_signed() < 0x20 {
        // call width: 2
        g.call(slot_target(g, w(0x283), 0x4a0 / 4), fr.ecx, &[a, 0])
            .eax
    } else {
        let m = g.read::<u32>(a.wrapping_mul(4).wrapping_add(w(0x2a3)));
        // call width: 2
        g.call(slot_target(g, w(0x2a8), 0x404 / 4), fr.ecx, &[m, 1])
            .eax
    };
    let n = g.read::<u32>(at(0x1ef0)).wrapping_sub(1);
    g.write::<u32>(at(0x1ef0), n);
    Ret::eax(eax)
}

/// [`rise_0450`] (the `rel32`s: the sparks, the list removal and the
/// object start).
pub const RISE_0450: Sig = Sig {
    len: 703,
    hash: 0x9e39_c15a_70c4_1946,
    calls: &[
        (0x19, Callee::Sig(&crate::r11_misc::SPARKS_0450)),
        (0xd4, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x11f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};

/// `magic_0450`'s caster launch (0x10007100, an overlay table entry, no
/// arguments; a /GS frame aligned to 8, `S` = `((esp - 4) & !7) - 0x90`
/// after the saved registers, its locals the slots fill; 1,390 bytes):
/// with the block `B = *G_B`: the object `O = B+0x5f0` leaves the DLL's
/// list ([`list_remove`](crate::l5_objhost::list_remove); when it was
/// there `*G_C = CB`, slot 713 `(O)` and bit 15 of `O+0x124` cleared);
/// the actor `A = *(G_T + 4 (*G_K)[0])`'s point (slots 262, 268 `(A, 16,
/// S+0x18)`) into `B+0x1c40` (`w` 1.0); the local matrix `S+0x48` (slot
/// 235, turned by slots 328 `(0)` and 329 `(B+0x1ee0)`) moves `(0, -25,
/// 80, 1)` (slot 229) and the point is added (slot 248) into `B+0x1c30`
/// (`w` 1.0); the matrix `M = B+0x1bb0` rebuilt as in
/// [`caster_start_0450`] for the actor `(*G_K)[1]` (slots 235, 277, 15-17,
/// 330, 328, 329) with the angles `B+0x1bf0 = (0, yaw, 0, 0)`; the point
/// into `B+0x1be0` and `B+0x1c00`; slots 210 and 211 `(T, M)` (`T` the
/// second actor's; with `M` 0 the other way as in [`caster_start_0450`]);
/// the object `B+0x130` started
/// ([`start_object_at`](crate::s4_misc::start_object_at), index 1, scale
/// `*K`); the way `d = B+0x1c40 - B+0x1c30`: the pitch `atan2(dy,
/// sqrt(dx dx + dz dz))` and the yaw `K1 - atan2(dz, dx)` (wrapped once
/// by `T2` past `H` or below `Lo`; `_CIatan2`'s `f64`s rounded) into
/// `B+0x1f0`/`+0x1f4` and `B+0x1c50`/`+0x1c54`, `B+0x234` the callback
/// `+0x524`, `B+0x1ef0` 12 and `+0x1ef4` 1. Returns 0 (the original
/// leaves `_CIatan2`'s `eax`, junk).
#[allow(clippy::too_many_lines)] // the original's calls, in order
pub fn launch_0450<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let st0 = |o: crate::Out| crate::m0018::st0_f32(o.st0);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x90);
    let l = |o: u32| s.wrapping_add(o);
    let b = g.read::<u32>(w(0x1f));
    let at = |o: u32| b.wrapping_add(o);
    let o = at(0x5f0);
    let removed = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x31), w(0x24), o);
    if removed != 0 {
        let c = g.read::<u32>(w(0x3a));
        g.write::<u32>(c, w(0x41));
        // call width: 1
        g.call(slot_target(g, w(0x46), 0xb24 / 4), fr.ecx, &[o]);
        let h = g.read::<u16>(o + 0x124) & 0x7fff;
        g.write::<u16>(o + 0x124, h);
    }
    let actor = |g: &G, sel: u32, k1: u32, tab: u32| {
        let n = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(k1)));
        g.read::<u32>(n.wrapping_mul(4).wrapping_add(w(tab)))
    };
    let a = actor(g, 0x62, 0, 0x6c);
    let mut st = Pushes::default();
    st.push_all(&[l(0x20), l(0x1c), l(0x18), a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x80), 0x418 / 4), fr.ecx);
    st.push_all(&[l(0x18), 0x10, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x93), 0x430 / 4), fr.ecx);
    for c in [0u32, 4, 8] {
        let v = g.read::<u32>(l(0x18 + c));
        g.write::<u32>(at(0x1c40 + c), v);
    }
    g.write::<u32>(at(0x1c4c), 0x3f80_0000);
    let y = g.read::<u32>(at(0x1ba4));
    st.push(l(0x48));
    g.write::<u32>(l(0x1c), y);
    let yaw = g.read::<u32>(at(0x1ee0));
    for (o2, v) in [
        (0x24u32, 0x3f80_0000u32),
        (0x28, 0),
        (0x2c, 0xc1c8_0000),
        (0x30, 0x42a0_0000),
        (0x34, 0x3f80_0000),
    ] {
        g.write::<u32>(l(o2), v);
    }
    g.write::<u32>(l(0x10), yaw);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0xe4), 0x3ac / 4), fr.ecx);
    // add esp, 0x1c: the 262 call's first word stays, written over
    for (slot, word, tab) in [(0x520u32, 0u32, 0x13du32), (0x524, yaw, 0x15e)] {
        let mut st = Pushes::default();
        st.push_all(&[word, l(0x48), l(0x48)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot_target(g, w(tab), slot / 4), fr.ecx);
    }
    // (slot 329's words stay under it)
    let mut st = Pushes::default();
    st.push_all(&[yaw, l(0x48), l(0x48)]);
    st.push_all(&[l(0x28), l(0x48), l(0x38)]);
    g.write::<u32>(l(0x34), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x17a), 0x394 / 4), fr.ecx);
    st.push_all(&[l(0x38), l(0x18), l(0x18)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x19a), 0x3e0 / 4), fr.ecx);
    let v = g.read::<u32>(l(0x18));
    g.write::<u32>(at(0x1c30), v);
    let v = g.read::<u32>(l(0x1c));
    g.write::<u32>(l(0x24), 0x3f80_0000);
    g.write::<u32>(at(0x1c34), v);
    let v = g.read::<u32>(l(0x20));
    g.write::<u32>(at(0x1c38), v);
    g.write::<u32>(at(0x1c3c), 0x3f80_0000);
    let yaw = g.read::<u32>(at(0x1ee0));
    g.write::<u32>(at(0x1bf4), yaw);
    for o2 in [0x1bf0u32, 0x1bf8, 0x1bfc] {
        g.write::<u32>(at(o2), 0);
    }
    let m = at(0x1bb0);
    st.push(m);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x1ef), 0x3ac / 4), fr.ecx);
    st.push(actor(g, 0x221, 1, 0x22c));
    // call width: 12 (Pushes caps the words at 12)
    let sc = mul(
        st0(st.call(g, slot_target(g, w(0x231), 0x454 / 4), fr.ecx)),
        k(g, 0x24b),
    );
    for c in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + c, sc);
    }
    let tt = actor(g, 0x25e, 1, 0x269);
    st.push(tt);
    // call width: 12 (Pushes caps the words at 12)
    let a1 = st0(st.call(g, slot_target(g, w(0x26e), 0x3c / 4), fr.ecx));
    let v = mul(g.read::<f32>(m), a1);
    st.push(tt);
    g.write::<f32>(m, v);
    // call width: 12 (Pushes caps the words at 12)
    let a2 = st0(st.call(g, slot_target(g, w(0x287), 0x40 / 4), fr.ecx));
    let v = mul(g.read::<f32>(m + 0x14), a2);
    st.push(tt);
    g.write::<f32>(m + 0x14, v);
    // call width: 12 (Pushes caps the words at 12)
    let a3 = st0(st.call(g, slot_target(g, w(0x2a5), 0x44 / 4), fr.ecx));
    let v = mul(a3, g.read::<f32>(m + 0x28));
    g.write::<f32>(m + 0x28, v);
    // add esp, 0x34: the 262 call's first word stays, each angle over it
    for (slot, ang, tab) in [
        (0x528u32, 0x1bf8u32, 0x2b5u32),
        (0x520, 0x1bf0, 0x2f0),
        (0x524, 0x1bf4, 0x30f),
    ] {
        let a = g.read::<u32>(at(ang));
        let mut st = Pushes::default();
        st.push_all(&[a, m, m]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot_target(g, w(tab), slot / 4), fr.ecx);
    }
    let v: [u32; 4] = core::array::from_fn(|c| g.read::<u32>(l(0x18 + 4 * c as u32)));
    for (c, x) in (0u32..).zip(v) {
        g.write::<u32>(at(0x1be0 + 4 * c), x);
    }
    for (c, x) in (0u32..).zip(v) {
        g.write::<u32>(at(0x1c00 + 4 * c), x);
    }
    let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x381)).wrapping_add(1)));
    let t2 = g.read::<u32>(sel.wrapping_mul(4).wrapping_add(w(0x38c)));
    g.write::<u32>(l(0x10), t2);
    if m != 0 {
        // call width: 2
        g.call(slot_target(g, w(0x39b), 0x348 / 4), fr.ecx, &[t2, m]);
    } else if g.read::<u32>(w(0x3ae)) == 0 && sel.cast_signed() < 0x20 {
        // call width: 2
        g.call(slot_target(g, w(0x3c0), 0x364 / 4), fr.ecx, &[sel, l(0x38)]);
        // call width: 4 (the first call's words under it)
        g.call(
            slot_target(g, w(0x3d3), 0x3fc / 4),
            fr.ecx,
            &[sel, l(0x38), sel, l(0x38)],
        );
    }
    // call width: 2
    g.call(slot_target(g, w(0x3ea), 0x34c / 4), fr.ecx, &[t2, m]);
    let start = img.rel32_target(f + 0x40f);
    crate::s4_misc::start_object_at(g, img, start, at(0x130), k(g, 0x3fa), 1, 0, 0);
    let p = |g: &G, o2: u32| g.read::<f32>(at(o2));
    let dy = sub(p(g, 0x1c44), p(g, 0x1c34));
    let dx = sub(p(g, 0x1c40), p(g, 0x1c30));
    let dz = sub(p(g, 0x1c48), p(g, 0x1c38));
    let sq = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise`, then `_CIatan2` on x87
    let h = f64::from(sq).sqrt();
    let pitch = g.atan2(f64::from(dy), h);
    let turn = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = sub(k(g, 0x4c9), turn);
    // `comiss` + `jbe`: an unordered yaw stays
    if yaw > k(g, 0x4d8) {
        yaw = sub(yaw, k(g, 0x4e2));
    } else if k(g, 0x4ec) > yaw {
        yaw = add(yaw, k(g, 0x4f9));
    }
    let pitch = pitch as f32;
    g.write::<f32>(at(0x1f4), yaw);
    g.write::<f32>(at(0x1f0), pitch);
    g.write::<u32>(at(0x234), w(0x524));
    g.write::<f32>(at(0x1c50), pitch);
    g.write::<f32>(at(0x1c54), yaw);
    g.write::<u32>(at(0x1c58), 0);
    g.write::<u32>(at(0x1c5c), 0);
    g.write::<u32>(at(0x1ef0), 0xc);
    g.write::<u32>(at(0x1ef4), 1);
    Ret::eax(0)
}

/// [`launch_0450`] (the `rel32`s: the list removal, the object start, the
/// square root's and `_CIatan2`'s thunks and the cookie check).
pub const LAUNCH_0450: Sig = Sig {
    len: 1390,
    hash: 0xd31d_0583_1ec3_2501,
    calls: &[
        (0x31, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x40f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x475, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x488, Callee::Import("_CIatan2")),
        (0x4b7, Callee::Import("_CIatan2")),
        (0x566, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0675`'s start-up with dealt actors and their kinds
/// (0x100013e0, 1,543 bytes; the frame of [`dealt_start`]): its head
/// (the words, slot 719, the part, the deal, the yaw) and two more words
/// set (`+0xbb`, `+0xc5`); for each object `i` (`O = B + 0x130 i`, `B`
/// the operand `+0xdf` less 0xd4, the record `E` from `+0xe4`): slot 404
/// `(0.2)`, the scale `(r + K1) K2`; the dealt actor's kind
/// ([`actor_kind_check`](crate::sa_misc::actor_kind_check)); kind 0: the
/// object started with index 0
/// ([`start_object_first`](crate::o6_fam_object::start_object_first), the
/// stack `(0, owner)`), the actor's point (slots 262, 268) and its height
/// `O+0xd4` = slot 12 `(A)` less `K3`, `O+0x108` the operand `+0x1cd`;
/// else index 1, the point and `O+0x108` = `+0x234`; then as
/// [`dealt_start`]'s loop (yaw, actor number, owner one step on, the
/// record) and its tail ([`dealt_tail`], operands 0xb5 bytes on).
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn dealt_kind_start_0675<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    let dealt = e.wrapping_sub(0x18);
    stores(g, img, f, &[0x26, 0x30, 0x3a]);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x22)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot_target(g, w(0x11), 0xb3c / 4), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x4a), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x4f));
    let at = img.rel32_target(f + 0x57);
    crate::sb_misc::deal(g, img, at, e.wrapping_sub(0x40), dealt);
    st.push(g.read::<u32>(w(0x62)));
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x5c), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let mut yaw = sub(k(g, 0x7a), t);
    // `comiss` + `jbe`: an unordered yaw stays
    if yaw > k(g, 0x89) {
        yaw = sub(yaw, k(g, 0x98));
    } else if k(g, 0xa2) > yaw {
        yaw = add(yaw, k(g, 0xaf));
    }
    stores(g, img, f, &[0xbb, 0xc5]);
    let mut owner = 0u32;
    let mut obj = w(0xdf);
    let mut rec = w(0xe4);
    let actor = |g: &G, b: i32, none: u32, list: u32, table: u32| {
        if b == -1 {
            g.read::<u32>(w(none))
        } else {
            let l = g.read::<u32>(w(list));
            let n = g.read::<u8>(l.wrapping_add(b.cast_unsigned()));
            g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(table)))
        }
    };
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0xd4)) {
        let b = i32::from(g.read::<i8>(dealt.wrapping_add(i)));
        // push ecx; mov [esp], 0.2; call; add esp, 4
        let mut s = Pushes::default();
        s.push(w(0x107));
        // call width: 12 (Pushes caps the words at 12)
        let r = s.call(g, slot_target(g, w(0xf6), 0x650 / 4), fr.ecx);
        let r = crate::m0018::st0_f32(r.st0);
        let scale = mul(add(r, k(g, 0x11c)), k(g, 0x126));
        let kind = crate::sa_misc::actor_kind_check(
            g,
            img,
            img.rel32_target(f + 0x130),
            b.cast_unsigned(),
        );
        // sub esp, 8; push owner; push kind (0) or 1; call
        let esp = e.wrapping_sub(0x4c);
        g.write::<u32>(esp + 4, u32::from(kind != 0));
        g.write::<u32>(esp + 8, owner);
        let o = obj.wrapping_sub(0xd4);
        let p = obj.wrapping_sub(4);
        let mut s = Pushes::default();
        if kind == 0 {
            let at = img.rel32_target(f + 0x14f);
            let call = Frame {
                at,
                esp,
                edx: o,
                xmm: [0, 0, 0, u128::from(scale.to_bits())],
                ..*fr
            };
            crate::o6_fam_object::start_object_first(g, img, at, &call);
            let a = actor(g, b, 0x15d, 0x164, 0x16f);
            s.push_all(&[obj + 4, obj, p, a]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x178), 0x418 / 4), fr.ecx);
            s.push_all(&[p, 0x11, a]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x18e), 0x430 / 4), fr.ecx);
            s.push(a);
            // call width: 12 (Pushes caps the words at 12)
            let hgt =
                crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x1a2), 0x30 / 4), fr.ecx).st0);
            let y = sub(hgt, k(g, 0x1b8));
            g.write::<u32>(obj + 8, 0x3f80_0000);
            g.write::<f32>(obj, y);
            g.write::<u32>(obj + 0x34, w(0x1cd));
        } else {
            let at = img.rel32_target(f + 0x1d6);
            let call = Frame {
                at,
                esp,
                edx: o,
                xmm: [0, 0, 0, u128::from(scale.to_bits())],
                ..*fr
            };
            crate::o6_fam_object::start_object_first(g, img, at, &call);
            let a = actor(g, b, 0x1e4, 0x1eb, 0x1f6);
            s.push_all(&[obj + 4, obj, p, a]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x1ff), 0x418 / 4), fr.ecx);
            s.push_all(&[p, 0x11, a]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, slot_target(g, w(0x215), 0x430 / 4), fr.ecx);
            g.write::<u32>(obj + 8, 0x3f80_0000);
            g.write::<u32>(obj + 0x34, w(0x234));
        }
        g.write::<f32>(obj.wrapping_sub(0x10), yaw);
        let n = if b == -1 {
            g.read::<u32>(w(0x24b))
        } else {
            let l = g.read::<u32>(w(0x252));
            u32::from(g.read::<u8>(l.wrapping_add(b.cast_unsigned())))
        };
        owner = owner.wrapping_add(w(0x25d) & 0xff);
        g.write::<u32>(obj + 0x4c, n);
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
        g.write::<u32>(rec + 0xc, w(0x28f));
        g.write::<u32>(rec + 0x10, w(0x296));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        obj = obj.wrapping_add(0x130);
        rec = rec.wrapping_add(0x34);
        i += 1;
    }
    dealt_tail(g, img, fr, f + 0xb5, owner, 0)
}

/// [`dealt_kind_start_0675`] (the `rel32`s: the part's set-up, the deal,
/// the kind check, the two object starts and the cookie check).
pub const DEALT_KIND_START_0675: Sig = Sig {
    len: 1543,
    hash: 0x8bf8_1582_0c73_b866,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x57, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x130, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (
            0x14f,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0126_22A0),
        ),
        (
            0x1d6,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0126_22A0),
        ),
        (0x5ff, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0492`'s start-up with dealt actors, their kinds and a node
/// each (0x10002a30, 1,911 bytes; a cookie frame, `ebp` = `esp - 4`):
/// as [`dealt_kind_start_0675`]'s head (two words set) and loop (no
/// `O+0x108`; `O+0xdc` 1.0 on both ways), the owner 0x3c a step, and per
/// object the record `R = R0 + 0x34 i` (`R+0x14` = owner + 12, sizes,
/// the dealt number, three 1.0s) with a ring node (`+0x14 = R - 8`)
/// appended to the task list, `*G_R = R - 8`. Then the actor block
/// inlined as [`dealt_start_0494`]'s (last owner `n` = owner - 0x3c),
/// the light words, two ring nodes (records `+0x53d`, `+0x671`; two light
/// words overwritten after each), slot 271 `(4)` and a third node (byte
/// `+0x18` 2). Returns 0.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn dealt_nodes_start_0492<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.esp;
    let dealt = e.wrapping_sub(0x18);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x20)]);
    stores(g, img, f, &[0x24, 0x2e]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot_target(g, w(0x11), 0xb3c / 4), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x3e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    let at = img.rel32_target(f + 0x4b);
    crate::sb_misc::deal(g, img, at, e.wrapping_sub(0x3c), dealt);
    st.push(g.read::<u32>(w(0x56)));
    // call width: 12 (Pushes caps the words at 12)
    let t = st.call(g, slot_target(g, w(0x50), 0x534 / 4), fr.ecx);
    let t = crate::m0018::st0_f32(t.st0);
    let mut yaw = sub(k(g, 0x70), t);
    // `comiss` + `jbe`: an unordered yaw stays
    if yaw > k(g, 0x7f) {
        yaw = sub(yaw, k(g, 0x8e));
    } else if k(g, 0x98) > yaw {
        yaw = add(yaw, k(g, 0xa5));
    }
    let actor = |g: &G, b: i32, none: u32, list: u32, table: u32| {
        if b == -1 {
            g.read::<u32>(w(none))
        } else {
            let l = g.read::<u32>(w(list));
            let n = g.read::<u8>(l.wrapping_add(b.cast_unsigned()));
            g.read::<u32>(u32::from(n).wrapping_mul(4).wrapping_add(w(table)))
        }
    };
    let mut owner = 0u32;
    let mut obj = w(0xcd);
    let mut r = w(0xc8);
    let mut rec = w(0xd4);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0xbd)) {
        let b = i32::from(g.read::<i8>(dealt.wrapping_add(i)));
        // push ecx; mov [esp], 0.2; call; add esp, 4
        let mut s = Pushes::default();
        s.push(w(0xf7));
        // call width: 12 (Pushes caps the words at 12)
        let rr = s.call(g, slot_target(g, w(0xe6), 0x650 / 4), fr.ecx);
        let rr = crate::m0018::st0_f32(rr.st0);
        let scale = mul(add(rr, k(g, 0x10c)), k(g, 0x116));
        let kind = crate::sa_misc::actor_kind_check(
            g,
            img,
            img.rel32_target(f + 0x120),
            b.cast_unsigned(),
        );
        // sub esp, 8; push owner; push kind (0) or 1; call
        let esp = e.wrapping_sub(0x50);
        g.write::<u32>(esp + 4, u32::from(kind != 0));
        g.write::<u32>(esp + 8, owner);
        let o = obj.wrapping_sub(0xd4);
        let p = obj.wrapping_sub(4);
        let (start_at, ops) = if kind == 0 {
            (0x13b, (0x149, 0x150, 0x15b, 0x164, 0x17a))
        } else {
            (0x1b4, (0x1c2, 0x1c9, 0x1d4, 0x1dd, 0x1f3))
        };
        let at = img.rel32_target(f + start_at);
        let call = Frame {
            at,
            esp,
            edx: o,
            xmm: [0, 0, 0, u128::from(scale.to_bits())],
            ..*fr
        };
        crate::o6_fam_object::start_object_first(g, img, at, &call);
        let a = actor(g, b, ops.0, ops.1, ops.2);
        let mut s = Pushes::default();
        s.push_all(&[obj + 4, obj, p, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(ops.3), 0x418 / 4), fr.ecx);
        s.push_all(&[p, 0x11, a]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(ops.4), 0x430 / 4), fr.ecx);
        if kind == 0 {
            s.push(a);
            // call width: 12 (Pushes caps the words at 12)
            let hgt =
                crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x18e), 0x30 / 4), fr.ecx).st0);
            g.write::<f32>(obj, sub(hgt, k(g, 0x1a4)));
        }
        g.write::<u32>(obj + 8, 0x3f80_0000);
        g.write::<f32>(obj.wrapping_sub(0x10), yaw);
        let n = if b == -1 {
            g.read::<u32>(w(0x222))
        } else {
            let l = g.read::<u32>(w(0x229));
            u32::from(g.read::<u8>(l.wrapping_add(b.cast_unsigned())))
        };
        g.write::<u32>(obj + 0x4c, n);
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(rec + 0xc, w(0x25b));
        g.write::<u32>(rec + 0x10, w(0x262));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        g.write::<u32>(obj + 0x3c, rec.wrapping_sub(0x14));
        // the object's record and its ring node
        g.write::<u32>(r + 0x14, owner.wrapping_add(0xc));
        for (o, v) in [
            (0xffff_fff8u32, 0x3f00_0000u32),
            (0xffff_fffc, 0x3fa6_6666),
            (0, 0x3f00_0000),
            (0x10, 0x20),
            (0x18, 0x10),
            (0x1c, 0x3a),
            (0x20, b.cast_unsigned()),
            (4, 0x3f80_0000),
            (8, 0x3f80_0000),
            (0xc, 0x3f80_0000),
        ] {
            g.write::<u32>(r.wrapping_add(o), v);
        }
        g.write::<u16>(r + 0x28, 0);
        let node = if g.read::<u16>(w(0x2cd)) == 0 {
            // none free: the original writes through 0 (not run)
            0
        } else {
            let node = g.read::<u32>(w(0x2d9));
            crate::r13_misc::pool_pop(g, img, f, node, (0x2cd, 0x2fd, 0x305, 0x336, 0x341));
            node
        };
        let any = g.read::<u32>(w(0x347));
        let rr8 = r.wrapping_sub(8);
        g.write::<u32>(node.wrapping_add(0x14), rr8);
        g.write::<u32>(node.wrapping_add(4), 0);
        if any == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x36f), node);
        } else {
            let tail = g.read::<u32>(w(0x35c));
            g.write::<u32>(node, tail);
            g.write::<u32>(tail.wrapping_add(4), node);
        }
        g.write::<u32>(w(0x378), rr8);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x394), node);
        owner = owner.wrapping_add(0x3c);
        obj = obj.wrapping_add(0x130);
        r = r.wrapping_add(0x34);
        rec = rec.wrapping_add(0x34);
        i += 1;
    }
    // the actor block (`n` the last owner)
    let n = owner.wrapping_sub(0x3c);
    g.write::<u32>(w(0x3bc), n);
    let x2 = g.read::<u32>(w(0x3c1));
    let cb = if n.cast_signed() > 0 {
        w(0x3c6)
    } else {
        w(0x3cb)
    };
    let battle = g.read::<u32>(w(0x3d4)) != 0;
    g.write::<u32>(w(0x3da), x2);
    g.write::<u32>(w(0x3e6), cb);
    stores(g, img, f, &[0x3ea]);
    let mut st = Pushes::default();
    st.push(x2);
    let slot = if battle { 0x674 / 4 } else { 0x35c / 4 };
    // call width: 12 (Pushes caps the words at 12)
    let m = st.call(g, slot_target(g, w(0x3e0), slot), fr.ecx).eax;
    g.write::<u32>(w(0x416), m);
    stores(g, img, f, &[0x41a, 0x424]);
    g.write::<u32>(w(0x430), 1);
    g.write::<u16>(w(0x43a), u16::from(battle));
    // the light words
    let count = g.read::<u16>(w(0x44a));
    stores(
        g,
        img,
        f,
        &[0x44e, 0x458, 0x462, 0x46c, 0x476, 0x480, 0x48a],
    );
    stores(g, img, f, &[0x494, 0x49e, 0x4a8, 0x4b2, 0x4bc]);
    let (n1, head) = if count == 0 {
        (0, 0)
    } else {
        let node = g.read::<u32>(w(0x4d7));
        let (_, head) =
            crate::r13_misc::pool_pop(g, img, f, node, (0x44a, 0x4fc, 0x4fc, 0x521, 0x52c));
        (node, head)
    };
    let mut first = g.read::<u32>(w(0x532));
    g.write::<u8>(n1.wrapping_add(0x18), 1);
    g.write::<u32>(n1.wrapping_add(0x14), w(0x53d));
    g.write::<u32>(n1.wrapping_add(4), 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x562), n1);
    } else {
        let tail = g.read::<u32>(w(0x54d));
        g.write::<u32>(n1, tail);
        g.write::<u32>(tail.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    stores(
        g,
        img,
        f,
        &[0x56f, 0x579, 0x583, 0x58d, 0x597, 0x5a1, 0x5ab],
    );
    stores(g, img, f, &[0x5b5, 0x5bf, 0x5c9, 0x5d3]);
    g.write::<u32>(w(0x5de), owner.wrapping_add(0x14));
    stores(g, img, f, &[0x5e2, 0x5ec, 0x5f6]);
    let n2 = if g.read::<u16>(w(0x44a)) == 0 {
        0
    } else {
        crate::r13_misc::pool_pop(g, img, f, head, (0x44a, 0x622, 0x62a, 0x65b, 0x666));
        head
    };
    g.write::<u8>(n2.wrapping_add(0x18), 1);
    g.write::<u32>(n2.wrapping_add(0x14), w(0x671));
    g.write::<u32>(n2.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x68f), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    stores(g, img, f, &[0x69b]);
    g.write::<u32>(w(0x6af), n2);
    let mut st = Pushes::default();
    st.push(4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot_target(g, w(0x694), 0x43c / 4), fr.ecx);
    let n3 = if g.read::<u16>(w(0x6b8)) == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x6ca));
        crate::r13_misc::pool_pop(g, img, f, node, (0x6b8, 0x6e7, 0x6ef, 0x720, 0x72b));
        node
    };
    let any = g.read::<u32>(w(0x731));
    g.write::<u8>(n3.wrapping_add(0x18), 2);
    g.write::<u32>(n3.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x75a), n3);
    } else {
        let tail = g.read::<u32>(w(0x747));
        g.write::<u32>(n3, tail);
        g.write::<u32>(tail.wrapping_add(4), n3);
    }
    g.write::<u32>(n3.wrapping_add(8), n3);
    g.write::<u32>(w(0x768), n3);
    Ret::eax(0)
}

/// [`dealt_nodes_start_0492`] (the `rel32`s: the part's set-up, the deal,
/// the kind check, the two object starts and the cookie check).
pub const DEALT_NODES_START_0492: Sig = Sig {
    len: 1911,
    hash: 0x8737_7bb9_1456_66ac,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4b, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x120, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (
            0x13b,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0492_1E90),
        ),
        (
            0x1b4,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0492_1E90),
        ),
        (0x76f, Callee::Any),
    ],
    imports: &[],
};

/// The load steps ([`load_step`](crate::r9_load::load_step)) whose
/// start-up is a round 16 port: `name => SIG, shape, start-up port, its
/// Sig`.
macro_rules! loads {
    ($($f:ident => $sig:ident, $shape:ident, $start:path, $ssig:path;)*) => {
        $(
            #[doc = concat!("The load step running [`", stringify!($start), "`].")]
            pub fn $f<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
                crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::$shape, $start)
            }

            #[doc = concat!("[`", stringify!($f), "`].")]
            pub const $sig: Sig = Sig {
                len: match crate::r9_load::Shape::$shape {
                    crate::r9_load::Shape::Zero => 118,
                    crate::r9_load::Shape::One => 119,
                },
                hash: match crate::r9_load::Shape::$shape {
                    crate::r9_load::Shape::Zero => 0xf93b_1968_cd8e_f184,
                    crate::r9_load::Shape::One => 0x2da6_1889_6875_2369,
                },
                calls: &[(
                    match crate::r9_load::Shape::$shape {
                        crate::r9_load::Shape::Zero => 0x5b,
                        crate::r9_load::Shape::One => 0x5c,
                    },
                    Callee::Sig(&$ssig),
                )],
                imports: &[],
            };
        )*
    };
}

loads! {
    load_0619 => LOAD_0619, One, dealt_start, DEALT_START_0619;
    load_0681 => LOAD_0681, One, dealt_start, DEALT_START_0681;
    load_0480 => LOAD_0480, Zero, crate::m0367::start_up_0369, START_UP_0480;
    load_0474 => LOAD_0474, Zero, start_up_0474, START_UP_0474;
    load_0494 => LOAD_0494, Zero, dealt_start_0494, DEALT_START_0494;
    load_0675 => LOAD_0675, One, dealt_kind_start_0675, DEALT_KIND_START_0675;
    load_0492 => LOAD_0492, Zero, dealt_nodes_start_0492, DEALT_NODES_START_0492;
}
