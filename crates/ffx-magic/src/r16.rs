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
}
