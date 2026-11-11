//! Round 32's ports (magic-r32): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Where a copy's operands sit: `(start, delta)` runs over the model's
/// offsets (the last run starting at or below an offset applies), and
/// single offsets moved out of order (`(model, copy)`).
pub(crate) struct Shift {
    pub(crate) runs: &'static [(u32, i32)],
    pub(crate) moved: &'static [(u32, u32)],
}

impl Shift {
    /// The copy's offset of the model's operand `o`.
    pub(crate) fn at(&self, o: u32) -> u32 {
        if let Some(&(_, c)) = self.moved.iter().find(|&&(m, _)| m == o) {
            return c;
        }
        let d = self
            .runs
            .iter()
            .rev()
            .find(|&&(s, _)| s <= o)
            .map_or(0, |&(_, d)| d);
        o.wrapping_add_signed(d)
    }
}

/// `node` goes last on the task list as the overlays inline it (operands
/// in `f`'s code: the head `head` tested, the tail `tail` read, the head's
/// store `head_w` when the list was empty, the tail's store `tail_w`).
/// Returns `eax` after it: the old tail, or `eax` as it was. (The same
/// steps as `r31`'s private helper.)
pub(crate) fn append_last<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    [head, tail, head_w, tail_w]: [u32; 4],
    eax: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let listed = g.read::<u32>(w(head)) != 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    let mut eax = eax;
    if listed {
        let last = g.read::<u32>(w(tail));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        eax = last;
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(head_w), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(tail_w), node);
    eax
}

/// What a copy of `r31::spin_actors_0605` changes besides its operands'
/// places: the bases of the records `R`, the objects `B` and the shake
/// blocks `H` in `S`, and the object start's part number.
struct ActorsLayout {
    shift: Shift,
    rec: u32,
    obj: u32,
    shake: u32,
    part: u32,
}

/// `magic_0307` 0x10007000 against `magic_0605` 0x1000a540: `R = S +
/// 0x1000`, `B = S`, `H = S + 0x23a4`, part 13; `*G_N` is read first
/// (operand `+0x11`), the object pointer kept as `B + 0xd8` (an `add edx,
/// -0xd8` before the start, other store lengths after it).
const ACTORS_0307: ActorsLayout = ActorsLayout {
    shift: Shift {
        runs: &[
            (0x10, 5),
            (0x27, -9),
            (0x49, 0),
            (0x1c9, 3),
            (0x1d4, 0),
            (0x1f1, 6),
            (0x221, 3),
            (0x22e, -1),
            (0x23b, -4),
            (0x25c, 2),
        ],
        moved: &[(0x23, 0x11)],
    },
    rec: 0x1000,
    obj: 0,
    shake: 0x23a4,
    part: 13,
};

/// `magic_0307`'s actors' set-up for the spin (0x10007000, 1083 bytes; a
/// table callback, no arguments; a /GS frame): the steps of
/// `r31::spin_actors_0605` (`magic_0605` 0x1000a540) on 0307's records
/// ([`ACTORS_0307`]). With `S = *G_S` and `n = *G_N`, for each record `k
/// < n` (`R = S + 0x1000 + 0xd4 k`, `Q = R + 0x84`), its actor's object
/// `O`, the object `B = S + 0x130 k` and the shake block `H = S + 0x23a4 +
/// 0x30 k`: the actor's place (slot 262) and height (slot 12, ST0), `Q`'s
/// place, scales and turns (slots 330 / 328 / 329 after the record's
/// scale), `B` started (part 13, the part callback) and placed, `H` set
/// up with a shake task node put last on the task list; then the caster's
/// colour outside a battle (slots 217, 255), slots 211 / 213, `*G_E = 1`
/// and the others shown, whose `eax` it returns.
pub fn spin_actors_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    spin_actors_with(g, img, fr, &ACTORS_0307)
}

/// [`spin_actors_0307`] (the `rel32`s: the actor scale, the object start,
/// the others shown, `__security_check_cookie`; `rand`).
pub const SPIN_ACTORS_0307: Sig = Sig {
    len: 1083,
    hash: 0x4f30_68da_9fe6_fccc,
    calls: &[
        (0xff, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x210, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x429, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0x433, Callee::Any),
    ],
    imports: &[(0x373, "rand")],
};

#[allow(clippy::too_many_lines)] // the original's steps, one by one
fn spin_actors_with<G: Game>(g: &mut G, img: &Image, fr: &Frame, lay: &ActorsLayout) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let at = |o: u32| lay.shift.at(o);
    let w = |o: u32| img.word(f + at(o));
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let (px, py, pz, vec) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
        ebp.wrapping_sub(0x24),
    );
    let s = g.read::<u32>(w(0x13));
    let n = g.read::<u32>(w(0x23)).cast_signed();
    let mut v = 0x20u32;
    // `k` counts up from 0: the inlined "actor -1 is `*G_SELF`" branch
    // never runs
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(lay.rec + 0xd4 * k);
        let q = r.wrapping_add(0x84);
        let b = s.wrapping_add(lay.obj + 0x130 * k);
        let h = s.wrapping_add(lay.shake + 0x30 * k);
        g.write::<u32>(r.wrapping_add(0x14), 1);
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x64)).wrapping_add(k)));
        let o = g.read::<u32>(w(0x6f).wrapping_add(4 * i));
        let mut st = Pushes::default();
        st.push_all(&[pz, py, px, o]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x84, 262), 0);
        st.push(o);
        // call width: 12 (the most `Pushes::words` holds)
        let hy = st0_f32(st.call(g, t(g, 0x91, 12), 0).st0);
        let x = rf(g, px);
        g.write::<f32>(q + 0xc, x);
        let y = sub(rf(g, q + 0x10), c(g, 0xb6));
        for o in [0xffff_fffcu32, 0, 4] {
            g.write::<u32>(q.wrapping_add(o), 0);
        }
        g.write::<u32>(q + 0x1c, w(0xd1));
        g.write::<f32>(q + 0x10, y);
        let z = rf(g, pz);
        g.write::<f32>(q + 0x14, z);
        g.write::<u32>(q + 0x20, w(0xe7));
        g.write::<u32>(q + 0x24, w(0xee));
        let one = w(0xfa);
        let mat = r.wrapping_add(0x20);
        crate::sa_misc::actor_scale(g, img, img.rel32_target(f + at(0xff)), k, mat);
        st.pop(4);
        for (o, table, slot) in [
            (4u32, 0x10fu32, 330u32),
            (0xffff_fffc, 0x12e, 328),
            (0, 0x14c, 329),
        ] {
            st.set_top(rf(g, q.wrapping_add(o)).to_bits());
            st.push_all(&[mat, mat]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, table, slot), 0);
            st.pop(2);
        }
        for (dst, src) in [(0x34u32, 0xcu32), (0x30, 0x10), (0x2c, 0x14)] {
            let x = g.read::<u32>(q + src);
            g.write::<u32>(q.wrapping_sub(dst), x);
        }
        g.write::<u32>(q + 0x34, 0);
        g.write::<u32>(q + 0x38, 0);
        let d = sub(rf(g, q + 0x10), hy);
        for o in [0x24u32, 0x1c, 0x18, 0x14] {
            g.write::<u32>(q.wrapping_sub(o), 0);
        }
        let d = mul(d, c(g, 0x1af));
        g.write::<u32>(q.wrapping_sub(0x10), 0);
        // xorps with the sign mask
        let d = d.to_bits() ^ g.read::<u32>(w(0x1bd));
        g.write::<u32>(q.wrapping_sub(0x20), d);
        let scale = c(g, 0x1cd);
        g.write::<u32>(q.wrapping_sub(0xc), 0);
        g.write::<u32>(q.wrapping_sub(8), 0);
        g.write::<u32>(q + 0x40, v);
        let owner = v.wrapping_add(0x10);
        g.write::<u32>(q + 0x3c, 0x10);
        g.write::<u32>(q + 0x48, 0);
        g.write::<u32>(q + 0x4c, 0);
        let so = img.rel32_target(f + at(0x20a));
        crate::s4_misc::start_object_at(g, img, so, b, scale, lay.part, owner, w(0x1e3));
        g.write::<f32>(b + 0xd0, rf(g, px));
        g.write::<f32>(b + 0xd8, rf(g, pz));
        g.write::<u32>(b + 0xdc, one);
        g.write::<f32>(b + 0xd4, sub(hy, c(g, 0x244)));
        g.write::<u32>(b + 0x120, k);
        g.write::<u32>(h, 0);
        g.write::<u32>(h + 0x1c, 0);
        g.write::<u32>(h + 0x20, w(0x26e));
        g.write::<u32>(h + 0x24, 0);
        g.write::<u32>(h + 0x28, 10);
        g.write::<u16>(h + 0x2c, 0);
        let ops = [0x287, 0x293, 0x2b7, 0x2bf, 0x2f0, 0x2fb].map(at);
        let (node, eax) = crate::r27::pool_take_0587(g, img, f, ops, 0);
        g.write::<u8>(node.wrapping_add(0x18), 1);
        g.write::<u32>(node.wrapping_add(0x14), h);
        append_last(g, img, f, node, [0x301, 0x317, 0x32a, 0x368].map(at), eax);
        for (o, op) in [
            (4u32, 0x337u32),
            (8, 0x33e),
            (0xc, 0x345),
            (0x10, 0x34c),
            (0x14, 0x353),
            (0x18, 0x35a),
        ] {
            g.write::<u32>(h + o, w(op));
        }
        let fl = g.read::<u16>(h + 0x2c);
        g.write::<u16>(h + 0x2c, fl | 3);
        g.write::<u32>(h + 0x24, owner);
        let r = g.rand().cast_signed();
        v = v.wrapping_add(1).wrapping_add((r % 10).cast_unsigned());
    }
    let me = g.read::<u32>(w(0x3a8));
    if g.read::<u32>(w(0x3a1)) == 0 {
        let a = g.read::<u32>(w(0x3b0));
        if a.cast_signed() < 0x20 {
            // call width: 2
            g.call(t(g, 0x3be, 217), 0, &[a, vec]);
            // call width: 4 (slot 217's words stay on the stack)
            g.call(t(g, 0x3d0, 255), 0, &[a, vec, a, vec]);
        }
    }
    // call width: 2
    g.call(t(g, 0x3e1, 211), 0, &[me, 0]);
    if g.read::<u16>(w(0x3f6)) == 0 {
        let x = g.read::<u32>(w(0x408));
        // call width: 2
        g.call(t(g, 0x400, 213), 0, &[x, 0]);
    }
    g.write::<u32>(w(0x41e), 1);
    let show = img.rel32_target(f + at(0x427));
    let eax = crate::sb_vm::show_others(g, img, show, ebp.wrapping_sub(0x40), 1);
    Ret::eax(eax)
}

/// One of the sparks' two records: the operand offsets in the code (a
/// slot's table operand, then the `mov [esp], imm32` value's), the slot
/// of `+0x14` and what differs at the end (as `r31::sparks_0605`).
struct Spark {
    take: u32,
    /// `+0x30`'s source: the global's operand and the word's offset in
    /// the table it points at
    part: (u32, u32),
    s235: u32,
    turn: (u32, u32),
    s329: u32,
    far: (u32, u32),
    near: (u32, u32),
    s229: u32,
    x: (u32, u32),
    y: (u32, u32, u32, u32),
    z: (u32, u32),
    vx: (u32, u32, u32),
    w14: (u32, u32, u16),
    vz: (u32, u32, u32),
    yaw: (u32, u32),
    words: (u32, u32),
    size: (u32, u32, u32, u32),
    second: bool,
}

/// `magic_0307`'s two records (the first's part word at `+0`, the
/// second's at `+4`).
const SPARK_RECORDS_0307: [Spark; 2] = [
    Spark {
        take: 0x41,
        part: (0x51, 0),
        s235: 0x6c,
        turn: (0x83, 0x8a),
        s329: 0xa9,
        far: (0xb6, 0xc6),
        near: (0xd0, 0xd7),
        s229: 0xfd,
        x: (0x114, 0x132),
        y: (0x145, 0x14c, 0x17c, 0x184),
        z: (0x16d, 0x174),
        vx: (0x1af, 0x1a5, 0x1c7),
        w14: (0x1cc, 0x1d3, 404),
        vz: (0x1ed, 0x1f4, 0x20c),
        yaw: (0x211, 0x218),
        words: (0x240, 0x24e),
        size: (0x232, 0x25b, 0x26d, 0x28d),
        second: false,
    },
    Spark {
        take: 0x2ac,
        part: (0x2bc, 4),
        s235: 0x2d8,
        turn: (0x2ef, 0x2f6),
        s329: 0x315,
        far: (0x322, 0x332),
        near: (0x33c, 0x343),
        s229: 0x369,
        x: (0x380, 0x39e),
        y: (0x3b1, 0x3b8, 0x3e8, 0x3f0),
        z: (0x3d9, 0x3e0),
        vx: (0x41b, 0x411, 0x433),
        w14: (0x438, 0x43f, 314),
        vz: (0x459, 0x460, 0x478),
        yaw: (0x47d, 0x484),
        words: (0x4ac, 0x4ba),
        size: (0x49e, 0x4c7, 0x4d9, 0x4fa),
        second: true,
    },
];

/// `magic_0307`'s sparks (0x10007440, 1342 bytes; a table callback, no
/// arguments; a /GS frame): `r31::sparks_0605` (`magic_0605` 0x1000a980)
/// with three changes. Each record's `+0x30` is a word of the table `*G_P`
/// points at (`+0` for the first record, `+4` for the second) where 0605
/// calls slot 719; the height is `S[0xc64] - r404(10) + C - C'`; and `z`
/// adds `r + z` (0605: `z + r`). The actors' mean `P`
/// ([`mean_position`](crate::sb_misc::mean_position), its height then
/// 0), then three times two records of the 0x74-byte pool
/// ([`pool_take`](crate::sb_misc::pool_take); the first that fails ends
/// it, `eax` 0): frame -1, time 0x200, playing; a unit `M` (slot 235)
/// turned by slot 314 `(pi)` (slot 329); `V` = slot 229
/// `(V, M, (0, 0, a + b, 1))` with `a`, `b` slot 314 `(100)`, `(50)`; the place, speed,
/// spin, colour words and size from slot 314 / 404 draws; the second
/// record also an alpha byte, 0x2828 and `+0x48 = r`. Returns slot 314's
/// last `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn sparks_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let s = g.read::<u32>(w(0x15));
    let (out, vec, m, v) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x74),
        ebp.wrapping_sub(0x34),
    );
    g.write::<u32>(vec, 0);
    g.write::<u32>(vec + 4, 0);
    g.write::<u32>(vec + 0xc, w(0x2d));
    let mean = img.rel32_target(f + 0x32);
    crate::sb_misc::mean_position(g, img, mean, ebp.wrapping_sub(0x8c), out);
    g.write::<u32>(out + 4, 0);
    // a slot answering in ST0, its one word the immediate at `imm`
    let st0 = |g: &mut G, (table, imm): (u32, u32), slot: u32| {
        // call width: 1
        let out = g.call(t(g, table, slot), 0, &[w(imm)]);
        (st0_f32(out.st0), out.eax)
    };
    let mut eax = 0;
    for _ in 0..3 {
        for h in &SPARK_RECORDS_0307 {
            let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + h.take));
            if p == 0 {
                return Ret::eax(0);
            }
            let (table, at) = h.part;
            let part = g.read::<u32>(g.read::<u32>(w(table)).wrapping_add(at));
            g.write::<u16>(p + 0x38, 0xffff);
            g.write::<u16>(p + 0x3a, 0x200);
            g.write::<u32>(p + 0x30, part);
            g.write::<u32>(p + 0x34, 1);
            // call width: 1
            g.call(t(g, h.s235, 235), 0, &[m]);
            let (a, _) = st0(g, h.turn, 314);
            // call width: 3
            g.call(t(g, h.s329, 329), 0, &[m, m, a.to_bits()]);
            let (far, _) = st0(g, h.far, 314);
            let (near, _) = st0(g, h.near, 314);
            g.write::<f32>(vec + 8, add(near, far));
            // call width: 3
            g.call(t(g, h.s229, 229), 0, &[v, m, vec]);
            for o in [0u32, 4, 8, 0xc] {
                let x = g.read::<u32>(out + o);
                g.write::<u32>(p + o, x);
            }
            let (r, _) = st0(g, h.x, 314);
            let x = add(r, g.read::<f32>(p));
            g.write::<f32>(p, x);
            let (yt, yi, c1, c2) = h.y;
            let (r, _) = st0(g, (yt, yi), 404);
            let y = sub(
                add(sub(g.read::<f32>(s.wrapping_add(0xc64)), r), c(g, c1)),
                c(g, c2),
            );
            g.write::<f32>(p + 4, y);
            let (r, _) = st0(g, h.z, 314);
            let z = add(r, g.read::<f32>(p + 8));
            g.write::<f32>(p + 8, z);
            let (vt, vi, k) = h.vx;
            let (r, _) = st0(g, (vt, vi), 314);
            let sx = add(mul(g.read::<f32>(v), c(g, k)), r);
            g.write::<f32>(p + 0x10, sx);
            let (wt, wi, slot) = h.w14;
            let (r, _) = st0(g, (wt, wi), u32::from(slot));
            g.write::<f32>(p + 0x14, r);
            let (zt, zi, k) = h.vz;
            let (r, _) = st0(g, (zt, zi), 314);
            let sz = add(mul(g.read::<f32>(v + 8), c(g, k)), r);
            g.write::<f32>(p + 0x18, sz);
            let (r, _) = st0(g, h.yaw, 314);
            g.write::<f32>(p + 0x1c, r);
            g.write::<u32>(p + 0x20, 0);
            g.write::<u32>(p + 0x24, w(h.words.0));
            g.write::<u32>(p + 0x28, 0);
            g.write::<u32>(p + 0x2c, w(h.words.1));
            let (st, si, ca, cm) = h.size;
            let (r, e) = st0(g, (st, si), 314);
            eax = e;
            let size = mul(add(r, c(g, ca)), c(g, cm));
            if h.second {
                g.write::<u32>(p + 0x4c, 0x50);
                g.write::<u32>(p + 0x50, 0);
                g.write::<u16>(p + 0x68, 0x2828);
                g.write::<u8>(p + 0x6a, 0x28);
                g.write::<u32>(p + 0x60, 0);
                g.write::<f32>(p + 0x3c, size);
                let (r, e) = st0(g, (0x4de, 0x517), 314);
                eax = e;
                g.write::<f32>(p + 0x48, r);
            } else {
                g.write::<u32>(p + 0x40, 0);
                g.write::<u32>(p + 0x58, 0x62);
                g.write::<u32>(p + 0x5c, 0);
                g.write::<u32>(p + 0x4c, 0x62);
                g.write::<u32>(p + 0x50, 0);
                g.write::<f32>(p + 0x3c, size);
                g.write::<u32>(p + 0x60, 0);
            }
        }
    }
    Ret::eax(eax)
}

/// [`sparks_0307`] (the `rel32`s: the mean position, the pool take twice,
/// `__security_check_cookie`).
pub const SPARKS_0307: Sig = Sig {
    len: 1342,
    hash: 0x96b8_4f6d_bd7b_56e1,
    calls: &[
        (0x32, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x41, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x2ac, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x536, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0307`'s spinning pieces start (0x10003c70, 1118 bytes; a table
/// callback, no arguments; an aligned /GS frame): `r31::spin_start_0605`
/// (`magic_0605` 0x100087f0) on 0307's records (`R = S + 0x1000 + 0xd4 k`,
/// `P = R + 0x94`, the centre at `S + 0xc50`), with its own lift. For
/// each record `k < *G_N` and its actor's object `O`:
/// - `R + 0x14` 1; slot 262 `(O, &x, &y, &z)` (w 1.0); slot 12 `(O)`'s
///   height `h` (ST0) and one slot 314 `(5.0)` `r`; `P = y` and
///   `u = ((y - h) + (r + C)) / D`;
/// - the flat distance `d` from the centre (the height difference `S.y -
///   S.y`) to `P + 0x24`, `P + 0x28` (the largest kept); the yaw `K -
///   atan2(dz, dx)` (wrapped once) to `P + 0x1c`, `P + 0x20` a constant;
///   nine words from `P - 0x34` 0 but `P - 0x30 = u C1` and `P - 0x20 = u
///   C2 / D`; slot 333 `(O)`'s angle `K - a` wrapped to `P - 0x10`;
///   zeros, three scales 1.0 and the flags `0x3c, 8, 0, 1, 0` at `P +
///   0x2c..0x3c`;
/// - the actor's mode ([`actor_mode`](crate::sa_script::actor_mode),
///   `ecx = k`, `edx = R + 0x20`) and slot 528 `(LIST[k], 0)`.
///
/// Then, when the largest distance is above `L`, each record's `P + 0x28`
/// times `L / d`. A task node with the callback `0307`'s spin pieces
/// (`r31::spin_pieces_0605`'s copy) is taken from the free ring and goes
/// last on the task list, also kept at `S + 0x2860`. Returns 0xffff or the
/// list's old tail.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn spin_start_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::wrap_once;
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0x44; three pushes
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x44);
    let cookie = g.read::<u32>(w(0xa)) ^ esp0;
    g.write::<u32>(esp0 + 0x40, cookie);
    let b = esp0.wrapping_sub(0xc);
    let (px, py, pz) = (b + 0x3c, b + 0x40, b + 0x44);
    let s = g.read::<u32>(w(0x1c));
    let n = g.read::<u32>(w(0x15)).cast_signed();
    let mut far = 0.0f32;
    // `k` counts up from 0: the inlined "actor -1 is `*G_SELF`" branches
    // (operands `+0x4d`, `+0x91`, `+0x263`, `+0x31f`) never run
    let obj = |g: &G, k: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x54)).wrapping_add(k)));
        g.read::<u32>(w(0x5f).wrapping_add(4 * i))
    };
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(0x1000 + 0xd4 * k);
        let p = r.wrapping_add(0x94);
        g.write::<u32>(r.wrapping_add(0x14), 1);
        let o = obj(g, k);
        // call width: 4
        g.call(t(g, 0x74, 262), 0, &[o, px, py, pz]);
        g.write::<u32>(b + 0x48, 0x3f80_0000);
        let o = obj(g, k);
        // call width: 1
        let h = st0_f32(g.call(t(g, 0xa9, 12), 0, &[o]).st0);
        let y = rf(g, py);
        g.write::<f32>(p, y);
        // call width: 1 (`mov [esp], 5.0` over slot 12's word)
        let r5 = st0_f32(g.call(t(g, 0xbd, 314), 0, &[0x40a0_0000]).st0);
        let lift = add(sub(rf(g, py), h), add(r5, c(g, 0xee)));
        let sy = rf(g, s.wrapping_add(0xc54));
        g.write::<f32>(py, sy);
        let dy = sub(sy, sy);
        let z = rf(g, pz);
        let dz = sub(z, rf(g, s.wrapping_add(0xc58)));
        let u = div(lift, c(g, 0x127));
        let x = rf(g, px);
        let dx = sub(x, rf(g, s.wrapping_add(0xc50)));
        let flat = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        #[allow(clippy::cast_possible_truncation)]
        let d = f64::from(flat).sqrt() as f32;
        if d > far {
            far = d;
        }
        g.write::<f32>(p + 0x24, d);
        g.write::<f32>(p + 0x28, d);
        let dz = sub(z, rf(g, s.wrapping_add(0xc58)));
        let dx = sub(x, rf(g, s.wrapping_add(0xc50)));
        // `_CIatan2` on x87 (`ST1` = dz), rounded once
        #[allow(clippy::cast_possible_truncation)]
        let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = sub(c(g, 0x1c1), a);
        let yaw = wrap_once(g, img, f, yaw, [0x1d0, 0x1da, 0x1e4, 0x1f1]);
        g.write::<f32>(p + 0x1c, yaw);
        let up = mul(u, c(g, 0x20f));
        let side = div(mul(u, c(g, 0x207)), c(g, 0x217));
        g.write::<u32>(p + 0x20, w(0x21e));
        for i in 0..9u32 {
            g.write::<u32>(p.wrapping_sub(0x34 - 4 * i), 0);
        }
        g.write::<f32>(p.wrapping_sub(0x30), up);
        g.write::<f32>(p.wrapping_sub(0x20), side);
        let o = obj(g, k);
        // call width: 1
        let v = st0_f32(g.call(t(g, 0x27b, 333), 0, &[o]).st0);
        let a2 = sub(c(g, 0x28f), v);
        let a2 = wrap_once(g, img, f, a2, [0x29f, 0x2a9, 0x2b3, 0x2c0]);
        g.write::<f32>(p.wrapping_sub(0x10), a2);
        g.write::<u32>(p.wrapping_sub(0xc), 0);
        g.write::<u32>(p.wrapping_sub(8), 0);
        for (o, at) in [(0xcu32, 0x2dau32), (0x10, 0x2e1), (0x14, 0x2e8)] {
            g.write::<u32>(p + o, w(at));
        }
        for (o, v) in [
            (0x2cu32, 0x3cu32),
            (0x30, 8),
            (0x34, 0),
            (0x38, 1),
            (0x3c, 0),
        ] {
            g.write::<u32>(p + o, v);
        }
        let mode = Frame {
            at: img.rel32_target(f + 0x315),
            esp: b.wrapping_sub(4),
            ecx: k,
            edx: r.wrapping_add(0x20),
            ..*fr
        };
        crate::sa_script::actor_mode(g, img, &mode);
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x326)).wrapping_add(k)));
        // call width: 2
        g.call(t(g, 0x332, 528), 0, &[i, 0]);
    }
    let lim = c(g, 0x356);
    if n > 0 && far > lim {
        let q = div(lim, far);
        let mut i = 0u32;
        while i.cast_signed() < g.read::<u32>(w(0x398)).cast_signed() {
            let at = s.wrapping_add(0x10bc + 0xd4 * i);
            let x = mul(q, rf(g, at));
            g.write::<f32>(at, x);
            i += 1;
        }
    }
    let (node, eax) =
        crate::r27::pool_take_0587(g, img, f, [0x3a1, 0x3b0, 0x3cd, 0x3d5, 0x406, 0x411], 0);
    let eax = append_last(g, img, f, node, [0x417, 0x426, 0x439, 0x451], eax);
    g.write::<u32>(s.wrapping_add(0x2860), node);
    Ret::eax(eax)
}

/// [`spin_start_0307`] (the `rel32`s: `_libm_sse2_sqrt_precise`,
/// `_CIatan2`, the actor's mode, `__security_check_cookie`).
pub const SPIN_START_0307: Sig = Sig {
    len: 1118,
    hash: 0xc868_94a5_f616_bae6,
    calls: &[
        (0x159, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1af, Callee::Import("_CIatan2")),
        (0x315, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x456, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0307`'s object aimed from `P` to `Q` (0x10003000, 842 bytes;
/// internal: `ecx` = the object `O`, `edx` = `P`, `xmm3` the offset's
/// length `L`, the stack word `Q`; an 8-aligned /GS frame):
/// `r29::aim_object_0563` (`magic_0563` 0x10002af0, the same sums in
/// other registers) with another start at the end: the pitch `atan2(d.y,
/// sqrt(d.z^2 + d.x^2))` and the yaw `K - atan2(d.z, d.x)` wrapped once;
/// `V = (L, 0, 0, 1)` through `M` (slots 235, 328, 329, 229); `O + 0x130`
/// = `P`, `+0x140` = `P + d / K1 + V`, `+0x150` = `d / K1 K2 + P - V`
/// (`w` 1.0 each), `+0x160` = `Q`, `+0x170` = `P`, `+0x180..` = `(0,
/// 1/30, 0x1e)`; `O` started ([`start_object_at`](crate::s4_misc::start_object_at),
/// part 1, owners 0, scale `K3`) and placed at `P`. Returns `P.w`'s word.
#[allow(clippy::too_many_arguments)] // the original's registers and stack
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn aim_object_0307<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    obj: u32,
    p: u32,
    len: f32,
) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let q = g.read::<u32>(esp + 4);
    // push ebp; and esp, -8; sub esp, 0x74; three pushes
    let b0 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x80);
    let (m, v) = (b0 + 0x38, b0 + 0x28);
    let dy = sub(rf(g, q + 4), rf(g, p + 4));
    let dx = sub(rf(g, q), rf(g, p));
    let dz = sub(rf(g, q + 8), rf(g, p + 8));
    let flat = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let root = f64::from(flat).sqrt();
    // `_CIatan2` on x87 (`ST1` = y), each result rounded once
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), root) as f32;
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = sub(k(g, 0xc3), ang);
    // comiss; jbe: unordered keeps the value
    if yaw > k(g, 0xd2) {
        yaw = sub(yaw, k(g, 0xe2));
    } else if k(g, 0xec) > yaw {
        yaw = add(yaw, k(g, 0xf9));
    }
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x109, 235), 0);
    st.set_top(pitch.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x12b, 328), 0);
    st.pop(2);
    st.set_top(yaw.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x14c, 329), 0);
    for (i, x) in (0u32..).zip([len.to_bits(), 0, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    st.push_all(&[v, m, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x16e, 229), 0);
    let k1 = k(g, 0x1a4);
    let (px, py, pz) = (rf(g, p), rf(g, p + 4), rf(g, p + 8));
    let sy = div(sub(rf(g, q + 4), py), k1);
    let sx = div(sub(rf(g, q), px), k1);
    let sz = div(sub(rf(g, q + 8), pz), k1);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0x130 + 4 * i, x);
    }
    let (vx, vy, vz) = (rf(g, v), rf(g, v + 4), rf(g, v + 8));
    g.write::<f32>(obj + 0x140, add(add(rf(g, p), sx), vx));
    g.write::<f32>(obj + 0x144, add(add(rf(g, p + 4), sy), vy));
    g.write::<u32>(obj + 0x14c, 0x3f80_0000);
    g.write::<f32>(obj + 0x148, add(add(sz, rf(g, p + 8)), vz));
    let k2 = k(g, 0x243);
    g.write::<f32>(obj + 0x150, sub(add(mul(sx, k2), rf(g, p)), vx));
    g.write::<f32>(obj + 0x154, sub(add(mul(sy, k2), rf(g, p + 4)), vy));
    g.write::<f32>(obj + 0x158, sub(add(mul(sz, k2), rf(g, p + 8)), vz));
    g.write::<u32>(obj + 0x15c, 0x3f80_0000);
    for i in 0..4 {
        let x = g.read::<u32>(q + 4 * i);
        g.write::<u32>(obj + 0x160 + 4 * i, x);
    }
    let scale = k(g, 0x2a4);
    g.write::<u32>(obj + 0x180, 0);
    g.write::<u32>(obj + 0x184, 0x3d08_8889);
    g.write::<u32>(obj + 0x188, 0x1e);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0x170 + 4 * i, x);
    }
    let so = img.rel32_target(f + 0x30b);
    crate::s4_misc::start_object_at(g, img, so, obj, scale, 1, 0, 0);
    let mut eax = 0;
    for i in 0..4 {
        eax = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, eax);
    }
    eax
}

/// [`aim_object_0307`] (the `rel32`s: the square root, `_CIatan2` twice,
/// the object start, `__security_check_cookie`).
pub const AIM_OBJECT_0307: Sig = Sig {
    len: 842,
    hash: 0xfeb6_0dff_1795_36fd,
    calls: &[
        (0x6f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x82, Callee::Import("_CIatan2")),
        (0xb1, Callee::Import("_CIatan2")),
        (0x30b, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x342, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0307`'s two aimed objects (0x10003350, 355 bytes; cdecl, no
/// arguments; the point `P` at `ebp - 0x14`, `S = *G_S`, the target `Q =
/// S + 0xc40`): for the bone points 0x11 and 0x37 of `*G_O` (slot 237 into
/// `P`, `w` 1.0) the objects `S + 0xce0` and `S + 0xe70` aimed from `P` to
/// `Q` ([`aim_object_0307`], lengths `K1`, `K2`); then two pool nodes
/// ([`take`](crate::sa_pool::take), owners `CB1`, `CB2`) on the task list,
/// the second (kind 1) with the fader `S + 0x236c` reset (`(0, 60, 0, 2,
/// 14, 14)`, u16 `+0x18` 0) and made the list's tail. Returns the second
/// node. (`r29::aim_objects_0563`'s steps with `Q` taken as it is.)
pub fn aim_objects_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let p = ebp.wrapping_sub(0x14);
    let s = g.read::<u32>(w(0x13));
    g.write::<u32>(ebp.wrapping_sub(0x18), s);
    let q = s.wrapping_add(0xc40);
    // (bone, slot table operand, actor operand, object, length operand,
    // call, the callee's return address below the entry)
    for (bone, ts, ta, o, kl, c, ret) in [
        (
            0x11u32, 0x1eu32, 0x2eu32, 0xce0u32, 0x3bu32, 0x57u32, 0x40u32,
        ),
        (0x37, 0x60, 0x70, 0xe70, 0x7a, 0x90, 0x54),
    ] {
        let actor = g.read::<u32>(w(ta));
        // call width: 4
        g.call(t(g, ts, 237), 0, &[actor, bone, 0, p]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        // the words under the call: Q, then slot 237's four
        let esp = fr.esp.wrapping_sub(ret);
        for (i, x) in (1u32..).zip([q, actor, bone, 0, p]) {
            g.write::<u32>(esp + 4 * i, x);
        }
        aim_object_0307(g, img, call(c), esp, s.wrapping_add(o), p, k(g, kl));
    }
    let n1 = crate::sa_pool::take(g, img, call(0x9d), w(0x98));
    let mut head = g.read::<u32>(w(0xa3));
    g.write::<u32>(n1.wrapping_add(4), 0);
    if head == 0 {
        head = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0xcb), n1);
    } else {
        let last = g.read::<u32>(w(0xb6));
        g.write::<u32>(n1, last);
        g.write::<u32>(last.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    let c = s.wrapping_add(0x236c);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x3c),
        (8, 0),
        (0xc, 2),
        (0x14, 0xe),
        (0x10, 0xe),
    ] {
        g.write::<u32>(c + o, v);
    }
    g.write::<u16>(c + 0x18, 0);
    let n2 = crate::sa_pool::take(g, img, call(0x110), w(0xde));
    g.write::<u8>(n2.wrapping_add(0x18), 1);
    g.write::<u32>(n2.wrapping_add(0x14), c);
    g.write::<u32>(n2.wrapping_add(4), 0);
    g.write::<u32>(w(0x123), n2);
    if head == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x152), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    Ret::eax(n2)
}

/// [`aim_objects_0307`] (the `rel32`s: the aims, the pool take twice,
/// `__security_check_cookie` twice).
pub const AIM_OBJECTS_0307: Sig = Sig {
    len: 355,
    hash: 0x857e_89b3_11c8_6b25,
    calls: &[
        (0x57, Callee::Sig(&AIM_OBJECT_0307)),
        (0x90, Callee::Sig(&AIM_OBJECT_0307)),
        (0x9d, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x110, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x13c, Callee::Any),
        (0x15b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0307`'s start-up (0x10007a30, 570 bytes; no arguments, run by
/// its load step [`load_0307`]; no frame): `r29::start_up_0605`'s steps
/// (slot 718 `(P0)`; slot 719 `(P, 8, 0)` into `G_H` with the statics'
/// stores; the part set-up, the facing and places, the turned start; slot
/// 271 `(4)`; the script block's words; two task nodes, kinds 1 and 2, the
/// records block after the first), then slot 520 `()`'s battle id `b`
/// moves the five corner globals `X0..X4`: by `+K1` each when `b` is
/// 0x24e0004, 0x24e0006 or in group 0x1eb; `X0 - K1`, `X1 - K2`, `X3 +
/// K2`, `X4 + K1` for another id of group 0x24e. Returns 0.
pub fn start_up_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r13_start::append_kind;
    use crate::r25::abs_stores;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    g.call(t(g, 0x1, 718), 0, &[w(0x6)]);
    let s719 = t(g, 0x13, 719);
    abs_stores(g, img, f, 0x26, 0x58);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x22), 8, 0]).eax;
    g.write::<u32>(w(0x5e), h);
    // slot 718's word stays; push BUF: the return address at `E - 0xc`
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, w(0x6));
    g.write::<u32>(esp + 4, w(0x63));
    crate::sb_misc::part_setup(g, img, call(0x68), esp);
    crate::r29::facing_places_0605(g, img, call(0x6d), esp);
    let st = Frame {
        at: call(0x72),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &st);
    // call width: 3 (BUF and slot 718's word under it)
    g.call(t(g, 0x77, 271), 0, &[4, w(0x63), w(0x6)]);
    abs_stores(g, img, f, 0x8d, 0xb1);
    let n1 = crate::sa_pool::take(g, img, call(0xb2), w(0x89));
    append_kind(g, img, f, n1, 1, [0xb8, 0xcc, 0xeb, 0xde, 0xeb]);
    crate::sa_misc::records_init(g, img, call(0xf0), w(0xe3));
    let n2 = crate::sa_pool::take(g, img, call(0xfa), w(0xf5));
    let listed = g.read::<u32>(w(0x100)) != 0;
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x114));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x126), n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x12e), n2);
    // call width: 0
    let b = g.call(t(g, 0x133, 520), 0, &[]).eax;
    // a global `X` (operand `x`) moved by the constant at operand `k`
    let step = |g: &mut G, x: u32, k: u32, up: bool| {
        let (v, d) = (g.read::<f32>(w(x)), g.read::<f32>(w(k)));
        g.write::<f32>(w(x), if up { add(v, d) } else { sub(v, d) });
    };
    if b == 0x24e_0004 || b == 0x24e_0006 || b & 0xffff_0000 == 0x1eb_0000 {
        for x in [0x1d7u32, 0x1eb, 0x1ff, 0x213, 0x227] {
            step(g, x, 0x1cf, true);
        }
    } else if b & 0xffff_0000 == 0x24e_0000 {
        step(g, 0x16c, 0x174, false);
        step(g, 0x186, 0x18e, false);
        step(g, 0x19e, 0x1a6, true);
        step(g, 0x1b6, 0x1be, true);
    }
    Ret::eax(0)
}

/// [`start_up_0307`].
pub const START_UP_0307: Sig = Sig {
    len: 570,
    hash: 0xbdef_87f4_5cd8_c879,
    calls: &[
        (0x68, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x6d, Callee::Sig(&crate::r29::FACING_PLACES_0307)),
        (0x72, Callee::Sig(&crate::o4_effect::START_TURNED_0307_2430)),
        (0xb2, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf0, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xfa, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0307`'s load step (0x100012f0, the 118-byte shape) running
/// [`start_up_0307`].
pub fn load_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0307)
}

/// [`load_0307`].
pub const LOAD_0307: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0307))],
    imports: &[],
};

/// `magic_0307`'s bone point pushed out by a turned offset (0x100044c0,
/// 240 bytes; internal: `ecx` = the point `P`; a /GS `ebp` frame, the
/// matrix `M` at `ebp - 0x64`, the offset `V` at `ebp - 0x14`, the result
/// at `ebp - 0x24`): slot 237 `(*G_O, 0x5c, 0, P)` gives the bone point,
/// `P.w` 1.0; `M` the identity (slot 235) turned by `S + 0xc20` (slot 328)
/// and `S + 0xc24` (slot 329); `V = (0, -2, -2, 1)` through `M` (slot 229)
/// added to `P` (slot 248), `P.w` 1.0 again. Each call sees the words the
/// earlier ones left. Returns slot 248's `eax`. `esp` is the entry's (the
/// return address).
pub fn bone_offset_0307<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, p: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let (m, v, out) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
    );
    let s = g.read::<u32>(w(0x1e));
    let actor = g.read::<u32>(w(0x2c));
    let mut st = Pushes::default();
    st.push_all(&[p, 0, 0x5c, actor]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x11, 237), 0);
    g.write::<u32>(p.wrapping_add(0xc), 0x3f80_0000);
    for (i, x) in (0u32..).zip([0, 0xc000_0000, 0xc000_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x37, 235), 0);
    // `add esp, 0x10`: `P` is left on top for the turns' angles
    st.pop(4);
    st.set_top(g.read::<u32>(s.wrapping_add(0xc20)));
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x75, 328), 0);
    st.pop(2);
    st.set_top(g.read::<u32>(s.wrapping_add(0xc24)));
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x97, 329), 0);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    st.push_all(&[v, m, out]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0xb0, 229), 0);
    st.push_all(&[out, p, p]);
    // call width: 12 (the most `Pushes::words` holds)
    let eax = st.call(g, t(g, 0xc8, 248), 0).eax;
    g.write::<u32>(p.wrapping_add(0xc), 0x3f80_0000);
    eax
}

/// [`bone_offset_0307`] (`__security_check_cookie`).
pub const BONE_OFFSET_0307: Sig = Sig {
    len: 240,
    hash: 0x3eb5_4881_7e78_0f1c,
    calls: &[(0xe8, Callee::Any)],
    imports: &[],
};

/// `magic_0307`'s bone offset for a late object (0x100045b0, 39 bytes;
/// cdecl `(O)`): an object whose `+0x118` is above 0x3a gets `+0x104` 0
/// (`eax` as it came); else its point `O + 0xd0` goes through
/// [`bone_offset_0307`] (a tail jump on the same stack), whose `eax` it
/// returns.
pub fn late_bone_offset_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = fr.arg(g, 0);
    if g.read::<u32>(o.wrapping_add(0x118)).cast_signed() > 0x3a {
        g.write::<u32>(o.wrapping_add(0x104), 0);
        return Ret::eax(fr.eax);
    }
    let f = img.rel32_target(fr.at + 0x23);
    Ret::eax(bone_offset_0307(g, img, f, fr.esp, o.wrapping_add(0xd0)))
}

/// [`late_bone_offset_0307`].
pub const LATE_BONE_OFFSET_0307: Sig = Sig {
    len: 39,
    hash: 0x8256_666b_c9e3_e3f5,
    calls: &[(0x23, Callee::Sig(&BONE_OFFSET_0307))],
    imports: &[],
};

/// `magic_0307`'s object steered by the swing record (0x100045e0, 320
/// bytes; cdecl `(O)`; `S = *G_S`, the record `R = S + 0x16a0`): while the
/// count `n = R[0x20]` is above 0, the blend `t = n / D` gives the scale
/// `x = (C - t) K + t K'` (kept in the argument's slot), the count drops,
/// and when it reaches 0 slot 233 `(R, R, 10.0)` runs and the fader `S +
/// 0x236c` restarts (`(0, 60, 0, 0)`, its node by
/// [`object_node_edx`](crate::o4_more2::object_node_edx) with 0x14); the
/// point `O + 0xd0` goes through [`bone_offset_0307`]. With no count the
/// scale is `K`. `O + 0x60..0x6c` = the scale; `R + 0x10..` += `R`'s
/// first three words, and `O + 0xd0..` += those. Returns the count it
/// read (none left) or the bone offset's `eax`.
pub fn swing_object_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x6));
    let r = s.wrapping_add(0x16a0);
    let o = fr.arg(g, 0);
    let n = g.read::<u32>(r + 0x20);
    let (scale, eax) = if n.cast_signed() > 0 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let t = div(n.cast_signed() as f32, c(g, 0x36));
        let left = n.wrapping_sub(1);
        let a = mul(t, c(g, 0x43));
        g.write::<u32>(r + 0x20, left);
        let x = add(mul(sub(c(g, 0x2e), t), c(g, 0x4e)), a);
        // `movss [ebp + 8], xmm1`: the argument's slot keeps it
        g.write::<f32>(fr.esp.wrapping_add(4), x);
        if left == 0 {
            // call width: 3 (`push ecx` then `mov [esp], 10.0`)
            g.call(slot_target(g, w(0x5e), 233), 0, &[r, r, 0x4120_0000]);
            let c = s.wrapping_add(0x236c);
            for (k, v) in [(0u32, 0u32), (4, 0x3c), (8, 0), (0xc, 0)] {
                g.write::<u32>(c + k, v);
            }
            let node = img.rel32_target(f + 0x9e);
            crate::o4_more2::object_node_edx(g, img, node, c, 0x14);
        }
        let bo = img.rel32_target(f + 0xa9);
        // the return address: `esp` after four pushes (`ebp`, three registers)
        let esp = fr.esp.wrapping_sub(0x14);
        let eax = bone_offset_0307(g, img, bo, esp, o.wrapping_add(0xd0));
        (g.read::<f32>(fr.esp.wrapping_add(4)), eax)
    } else {
        (c(g, 0xb8), n)
    };
    for k in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<f32>(o.wrapping_add(k), scale);
    }
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    g.write::<f32>(r + 0x10, add(rf(g, r), rf(g, r + 0x10)));
    g.write::<f32>(r + 0x14, add(rf(g, r + 0x14), rf(g, r + 4)));
    g.write::<f32>(r + 0x18, add(rf(g, r + 8), rf(g, r + 0x18)));
    let p = o.wrapping_add(0xd0);
    g.write::<f32>(p, add(rf(g, r + 0x10), rf(g, p)));
    g.write::<f32>(p + 4, add(rf(g, p + 4), rf(g, r + 0x14)));
    g.write::<f32>(p + 8, add(rf(g, r + 0x18), rf(g, p + 8)));
    Ret::eax(eax)
}

/// [`swing_object_0307`] (the `rel32`s: the node, the bone offset).
pub const SWING_OBJECT_0307: Sig = Sig {
    len: 320,
    hash: 0x24a2_d247_3dbd_0106,
    calls: &[
        (
            0x9e,
            Callee::Sig(&crate::o4_more2::OBJECT_NODE_EDX_0307_B480),
        ),
        (0xa9, Callee::Sig(&BONE_OFFSET_0307)),
    ],
    imports: &[],
};

/// `magic_0307`'s two late objects started (0x10004720, 361 bytes; no
/// arguments; `S = *G_S`): the objects `A` (operand `+0x18`) and `B`
/// (`+0x47`) started ([`start_object_at`](crate::s4_misc::start_object_at):
/// part 8, scale `K`; part 9, owner 0xf, scale 0), each given the turn
/// `S + 0xc20`, `S + 0xc24` at `+0xc0`/`+0xc4` and its step callback at
/// `+0x104` ([`late_bone_offset_0307`], [`swing_object_0307`]); the swing
/// record `R = S + 0x16a0`: `(0, 0, -125, 0)` through slot 229 `(R, S +
/// 0xbe0, R)`, `+0x10..0x1c` 0, count 0x2b; the fader `F = S + 0x2350`
/// reset (`(0, 40, 60, 0)`, `+0x10`/`+0x14` 0x2b, u16 `+0x18` 0) with a
/// pool node ([`take`](crate::sa_pool::take), kind 1) last on the task
/// list. Returns the node.
pub fn late_objects_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0xc));
    let k = g.read::<f32>(w(0x4));
    let so = call(0x1d);
    crate::s4_misc::start_object_at(g, img, so, w(0x18), k, 8, 0, 0);
    let turn = |g: &mut G, (x, y): (u32, u32)| {
        let a = g.read::<u32>(s.wrapping_add(0xc20));
        g.write::<u32>(w(x), a);
        let b = g.read::<u32>(s.wrapping_add(0xc24));
        g.write::<u32>(w(y), b);
    };
    turn(g, (0x30, 0x4f));
    g.write::<u32>(w(0x55), w(0x59));
    crate::s4_misc::start_object_at(g, img, call(0x5e), w(0x47), 0.0, 9, 0xf, 0);
    turn(g, (0x74, 0x84));
    g.write::<u32>(w(0x8a), w(0x8e));
    let r = s.wrapping_add(0x16a0);
    for (i, x) in (0u32..).zip([0, 0, 0xc2fa_0000, 0]) {
        g.write::<u32>(r + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x9b, 229), 0, &[r, s.wrapping_add(0xbe0), r]);
    for o in [0x10u32, 0x14, 0x18, 0x1c] {
        g.write::<u32>(r + o, 0);
    }
    g.write::<u32>(r + 0x20, 0x2b);
    let c = s.wrapping_add(0x2350);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x28),
        (8, 0x3c),
        (0xc, 0),
        (0x14, 0x2b),
        (0x10, 0x2b),
    ] {
        g.write::<u32>(c + o, v);
    }
    g.write::<u16>(c + 0x18, 0);
    let node = crate::sa_pool::take(g, img, call(0x124), w(0xf2));
    let listed = g.read::<u32>(w(0x12a)) != 0;
    g.write::<u32>(node.wrapping_add(0x14), c);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(0x143));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(w(0x14d), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x15c), node);
        g.write::<u32>(w(0x161), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`late_objects_0307`] (the `rel32`s: the object start twice, the pool
/// take).
pub const LATE_OBJECTS_0307: Sig = Sig {
    len: 361,
    hash: 0xbe52_9a7f_bbc2_7562,
    calls: &[
        (0x1d, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x5e, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x124, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0307`'s aimed emitter start (0x100050f0, 294 bytes; no
/// arguments; `S = *G_S`): the emitter `E = S + 0x1e70` started
/// ([`start_object_at`](crate::s4_misc::start_object_at), part 9, scale
/// `K`), placed at `S + 0xc80`, turned by `S + 0xc20`/`S + 0xc24`, its
/// target `S + 0xc60`, the record words `+0x378` (operand `+0xb1`), `+0x380`
/// (the step callback) and zeros; aimed
/// ([`aimed_emitter_angles`](crate::s7_misc::aimed_emitter_angles), speed
/// `K2`); then each actor record's three scales (`S + 0x10a0 + 0xd4 k`)
/// 0.75. Returns the emitter's node with no records, else the record
/// pointer past the last.
pub fn emitter_start_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0xc));
    let e = s.wrapping_add(0x1e70);
    let k = g.read::<f32>(w(0x4));
    crate::s4_misc::start_object_at(g, img, call(0x20), e, k, 9, 0, 0);
    let copy = |g: &mut G, from: u32, to: u32| {
        let x = g.read::<u32>(s.wrapping_add(from));
        g.write::<u32>(e.wrapping_add(to), x);
    };
    for i in 0..4u32 {
        copy(g, 0xc80 + 4 * i, 0xd0 + 4 * i);
    }
    copy(g, 0xc20, 0xc0);
    copy(g, 0xc24, 0xc4);
    for i in 0..4u32 {
        copy(g, 0xc60 + 4 * i, 0x358 + 4 * i);
    }
    g.write::<u32>(e + 0x378, w(0xb1));
    g.write::<u32>(e + 0x37c, 0);
    g.write::<u32>(e + 0x380, w(0xc5));
    g.write::<u32>(e + 0x38c, 0);
    g.write::<u32>(e + 0x390, 0);
    g.write::<u16>(e + 0x394, 0);
    // ecx = E, xmm1 = the speed; the call's return address after two pushes
    let aim = Frame {
        at: call(0xe5),
        esp: fr.esp.wrapping_sub(0xc),
        eax: 0,
        ecx: e,
        edx: 0,
        xmm: [0, u128::from(g.read::<u32>(w(0x6a))), 0, 0],
    };
    let mut eax = crate::s7_misc::aimed_emitter_angles(g, img, &aim);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0xed)).cast_signed() {
        let at = s.wrapping_add(0x10a4 + 0xd4 * i);
        g.write::<u32>(at + 4, 0x3f40_0000);
        g.write::<u32>(at, 0x3f40_0000);
        g.write::<u32>(at - 4, 0x3f40_0000);
        i += 1;
        eax = at + 0xd4;
    }
    Ret::eax(eax)
}

/// [`emitter_start_0307`] (the `rel32`s: the object start, the aimed
/// emitter).
pub const EMITTER_START_0307: Sig = Sig {
    len: 294,
    hash: 0x1dd9_a093_cb35_fde2,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0xe5, Callee::Sig(&crate::s7_misc::AIMED_EMITTER_ANGLES)),
    ],
    imports: &[],
};

/// Where a copy of `magic_0307`'s three-node start keeps its operands:
/// the takes' `rel32`s and owners, the "listed" tests, the list's tail
/// read, the head stores, the last tail store; the fader and shake blocks
/// in `S`.
struct Chain {
    takes: [u32; 3],
    owners: [u32; 3],
    listed: [u32; 3],
    tail: u32,
    heads: [u32; 3],
    last: u32,
    fader: u32,
    shake: u32,
}

/// [`scene_start_0307`]'s operands.
const CHAIN_34C0: Chain = Chain {
    takes: [0x6e, 0xde, 0x148],
    owners: [0xe, 0xac, 0x11d],
    listed: [0x74, 0xe4, 0x14e],
    tail: 0x89,
    heads: [0x9c, 0x10a, 0x171],
    last: 0x1bc,
    fader: 0x2350,
    shake: 0x23a4,
};

/// `magic_0307`'s three task nodes as its starts inline them (the code at
/// `f`, `S` and the record `a`; operands in `ch`): a pool node
/// ([`take`](crate::sa_pool::take)) for `a` after the list's tail; the
/// fader `F = S + ch.fader` (`(0, 100, 0, 3)`, 100, 100) with a kind-1
/// node after it; the shake block `H = S + ch.shake` (`+0x20` 1/48,
/// `+0x28` 0x30) with a kind-1 node after that, `H + 4..` = `(4, 2, 0,
/// 4, 2, 0)`, flags `| 3`. The list's tail word is written only for the
/// last node. Returns the last node.
fn record_chain<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, a: u32, ch: &Chain) -> u32 {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    // a node after `prev` (or the list's head when the list is empty)
    let chain = |g: &mut G, node: u32, listed: bool, prev: u32, head_w: u32| {
        g.write::<u32>(node.wrapping_add(4), 0);
        if listed {
            g.write::<u32>(node, prev);
            g.write::<u32>(prev.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(head_w), node);
        }
        g.write::<u32>(node.wrapping_add(8), node);
    };
    let n1 = crate::sa_pool::take(g, img, call(ch.takes[0]), w(ch.owners[0]));
    let listed = g.read::<u32>(w(ch.listed[0])) != 0;
    g.write::<u32>(n1.wrapping_add(0x14), a);
    let tail = if listed { g.read::<u32>(w(ch.tail)) } else { 0 };
    chain(g, n1, listed, tail, ch.heads[0]);
    let c = s.wrapping_add(ch.fader);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x64),
        (8, 0),
        (0xc, 3),
        (0x14, 0x64),
        (0x10, 0x64),
    ] {
        g.write::<u32>(c + o, v);
    }
    g.write::<u16>(c + 0x18, 0);
    let n2 = crate::sa_pool::take(g, img, call(ch.takes[1]), w(ch.owners[1]));
    let listed = g.read::<u32>(w(ch.listed[1])) != 0;
    g.write::<u8>(n2.wrapping_add(0x18), 1);
    g.write::<u32>(n2.wrapping_add(0x14), c);
    chain(g, n2, listed, n1, ch.heads[1]);
    let h = s.wrapping_add(ch.shake);
    g.write::<u32>(h, 0);
    g.write::<u32>(h + 0x1c, 0);
    g.write::<u32>(h + 0x20, 0x3caa_aaab);
    g.write::<u32>(h + 0x24, 0);
    g.write::<u32>(h + 0x28, 0x30);
    g.write::<u16>(h + 0x2c, 0);
    let n3 = crate::sa_pool::take(g, img, call(ch.takes[2]), w(ch.owners[2]));
    let listed = g.read::<u32>(w(ch.listed[2])) != 0;
    g.write::<u8>(n3.wrapping_add(0x18), 1);
    g.write::<u32>(n3.wrapping_add(0x14), h);
    chain(g, n3, listed, n2, ch.heads[2]);
    for (o, v) in [
        (4u32, 0x4080_0000u32),
        (8, 0x4000_0000),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4000_0000),
        (0x18, 0),
    ] {
        g.write::<u32>(h + o, v);
    }
    let fl = g.read::<u16>(h + 0x2c);
    g.write::<u16>(h + 0x2c, fl | 3);
    g.write::<u32>(w(ch.last), n3);
    n3
}

/// `magic_0307`'s scene start (0x100034c0, 683 bytes; no arguments; `S =
/// *G_S`): the record `A = S + 0x2210` (colour bytes and words, `+0x8c`
/// / `+0x90` 0x18, `+0x98` 0x48), the fader `F = S + 0x2350` (`(0, 100,
/// 0, 3)`, 100, 100) and the shake block `H = S + 0x23a4` (`+0x20` 1/48,
/// `+0x28` 0x30), each with a pool node ([`take`](crate::sa_pool::take);
/// `F`'s and `H`'s kind 1) chained after the one before (the list's tail
/// word is written only for the last); `H + 4..` = `(4, 2, 0, 4, 2, 0)`,
/// flags `| 3`. Then the objects `O1` (operand `+0x1b7`; part 6, scale
/// `K1`) and `O2` (`+0x228`; part 12, owner 1, scale `K2`) started
/// ([`start_object_at`](crate::s4_misc::start_object_at)), each placed at
/// `S + 0xc50` with its `y` less `C1` / `C2` and `+0xc4` = `S + 0x2868`.
/// Returns `O2` (the start's `eax`).
pub fn scene_start_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x9));
    let a = s.wrapping_add(0x2210);
    g.write::<u32>(a + 0x81, 0xff80_ffff);
    g.write::<u8>(a + 0x80, 0xff);
    g.write::<u16>(a + 0x85, 0xffff);
    g.write::<u8>(a + 0x87, 0);
    g.write::<u32>(a + 0x88, 0);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x18);
    g.write::<u32>(a + 0x8c, 0x18);
    g.write::<u32>(a + 0x94, 0);
    record_chain(g, img, f, s, a, &CHAIN_34C0);
    // (object, start, part, owner, scale; the place's operands, `y`'s
    // constant, `+0xc4`'s)
    let objs = [
        (
            0x1b7u32,
            0x1c1u32,
            6u32,
            0u32,
            0x179u32,
            [0x1d9u32, 0x1e9, 0x204, 0x217],
            0x1f9u32,
            0x230u32,
        ),
        (
            0x228,
            0x23d,
            0xc,
            1,
            0x1d1,
            [0x24d, 0x25d, 0x278, 0x289],
            0x26d,
            0x29a,
        ),
    ];
    let mut eax = 0;
    for (o, st, part, owner, k, place, cy, turn) in objs {
        let scale = g.read::<f32>(w(k));
        eax = crate::s4_misc::start_object_at(g, img, call(st), w(o), scale, part, owner, 0);
        for (i, &op) in (0u32..).zip(&place) {
            let x = g.read::<u32>(s.wrapping_add(0xc50 + 4 * i));
            g.write::<u32>(w(op), x);
        }
        let y = sub(g.read::<f32>(s.wrapping_add(0xc54)), g.read::<f32>(w(cy)));
        let t = g.read::<u32>(s.wrapping_add(0x2868));
        g.write::<u32>(w(turn), t);
        g.write::<f32>(w(place[1]), y);
    }
    Ret::eax(eax)
}

/// [`scene_start_0307`] (the `rel32`s: the pool take three times, the
/// object start twice).
pub const SCENE_START_0307: Sig = Sig {
    len: 683,
    hash: 0x3eb8_4fe3_c794_6cdc,
    calls: &[
        (0x6e, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xde, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x148, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1c1, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x23d, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};

/// [`emitter_step_0307`]'s operands.
const CHAIN_4DE0: Chain = Chain {
    takes: [0x61, 0xd1, 0x13b],
    owners: [0xe, 0x9f, 0x110],
    listed: [0x67, 0xd7, 0x141],
    tail: 0x7c,
    heads: [0x8f, 0xfd, 0x164],
    last: 0x1a5,
    fader: 0x2388,
    shake: 0x23d4,
};

/// `magic_0307`'s emitter arrival (0x10004de0, 782 bytes; cdecl `(E)`, the
/// emitter's step callback set by [`emitter_start_0307`]; `S = *G_S`): the
/// record `A = S + 0x22b0` (colour words, `+0x8c`/`+0x90` 0x14, `+0x98`
/// 0x48) and the three task nodes ([`record_chain`]: fader `S + 0x2388`,
/// shake `S + 0x23d4`); the emitter `O` (operand `+0x19b`) taken off the
/// host's list ([`list_remove`](crate::l5_objhost::list_remove)) and, when
/// it was there, `**G_P = W`, slot 713 `(O)` and bit 15 of its `+0x124`
/// cleared; the objects `O1` (`+0x1ee`, part 5, scale `K1`) and `O2`
/// (`+0x255`, part 3, scale `K2`) started
/// ([`start_object_at`](crate::s4_misc::start_object_at)) at `E + 0x358`,
/// `+0xc4` = `S + 0x2868`, their step callbacks at `+0x104`; `O2 + 0x70`
/// a code word and `+0x74` 0x1000; each actor record's `S + 0x10c8 + 0xd4
/// k` 1. Returns 0x1000 with no records, else the record pointer past the
/// last.
pub fn emitter_step_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x9));
    let a = s.wrapping_add(0x22b0);
    g.write::<u32>(a + 0x80, 0x80ff_ffff);
    g.write::<u32>(a + 0x84, 0x0020_20ff);
    g.write::<u32>(a + 0x88, 0);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x14);
    g.write::<u32>(a + 0x8c, 0x14);
    g.write::<u32>(a + 0x94, 0);
    record_chain(g, img, f, s, a, &CHAIN_4DE0);
    let o = w(0x19b);
    if crate::l5_objhost::list_remove(g, img, call(0x1aa), w(0x1a0), o) != 0 {
        let p = g.read::<u32>(w(0x1b3));
        g.write::<u32>(p, w(0x1be));
        // call width: 1
        g.call(t(g, 0x1c3, 713), 0, &[w(0x1b8)]);
        let fl = g.read::<u16>(w(0x1da));
        g.write::<u16>(w(0x1da), fl & 0x7fff);
    }
    let e = fr.arg(g, 0);
    // (object, start, part, scale; the place's operands, `+0xc4`'s)
    let objs = [
        (
            0x1eeu32,
            0x1f3u32,
            5u32,
            0x1e2u32,
            [0x20eu32, 0x221, 0x233, 0x244],
            0x25du32,
        ),
        (0x255, 0x26c, 3, 0x1fe, [0x27c, 0x28c, 0x29c, 0x2ac], 0x2c6),
    ];
    for (i, (ob, st, part, k, place, turn)) in objs.into_iter().enumerate() {
        let scale = g.read::<f32>(w(k));
        crate::s4_misc::start_object_at(g, img, call(st), w(ob), scale, part, 0, 0);
        for (j, &op) in (0u32..).zip(&place) {
            let x = g.read::<u32>(e.wrapping_add(0x358 + 4 * j));
            g.write::<u32>(w(op), x);
        }
        let x = g.read::<u32>(s.wrapping_add(0x2868));
        g.write::<u32>(w(turn), x);
        if i == 0 {
            g.write::<u32>(w(0x263), w(0x267));
        }
    }
    g.write::<u32>(w(0x2cc), w(0x2d0));
    g.write::<u32>(w(0x2d6), w(0x2da));
    g.write::<u16>(w(0x2e0), 0x1000);
    let mut eax = 0x1000;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0x2e6)).cast_signed() {
        let at = s.wrapping_add(0x10c8 + 0xd4 * i);
        g.write::<u32>(at, 1);
        i += 1;
        eax = at + 0xd4;
    }
    Ret::eax(eax)
}

/// [`emitter_step_0307`] (the `rel32`s: the pool take three times, the
/// list removal, the object start twice).
pub const EMITTER_STEP_0307: Sig = Sig {
    len: 782,
    hash: 0x05c8_ce28_f608_d1a8,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xd1, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x13b, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1aa, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x1f3, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x26c, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};

/// Where a copy of `magic_0307`'s model matrix block keeps its operands:
/// the slot tables of 235, 277, 15, 16, 17, 330, 328 and 329, the actor
/// word pushed for 277 and loaded for the others, the scale `K`, and
/// whether slot 17's factor comes first (`a M[0x28]`).
struct MatrixOps {
    tables: [u32; 8],
    actor: (u32, u32),
    k: u32,
    first17: bool,
}

/// [`summon_start_0307`]'s block.
const MATRIX_55E0: MatrixOps = MatrixOps {
    tables: [0x69, 0x87, 0xaa, 0xd5, 0xf1, 0x101, 0x13a, 0x159],
    actor: (0x8d, 0xbe),
    k: 0xa5,
    first17: true,
};

/// `magic_0307`'s model matrix as its starts inline it (`M` pushed on
/// `st` already): `M` a unit (slot 235), its diagonal slot 277
/// `(*G_O)`'s scale times `K`, each then times slots 15, 16, 17 `(*G_O)`
/// (ST0), turned by `T.z`, `T.x`, `T.y` (slots 330, 328, 329; `T` the
/// three angles at `turn`). Leaves slot 329's three words on `st`.
fn model_matrix<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    st: &mut Pushes,
    (m, turn): (u32, u32),
    ops: &MatrixOps,
) {
    use crate::sb_sse::mul;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, i: usize, n: u32| slot_target(g, w(ops.tables[i]), n);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0, 235), 0);
    st.push(g.read::<u32>(w(ops.actor.0)));
    // call width: 12 (the most `Pushes::words` holds)
    let x = mul(
        st0_f32(st.call(g, t(g, 1, 277), 0).st0),
        g.read::<f32>(w(ops.k)),
    );
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, x);
    }
    let o = g.read::<u32>(w(ops.actor.1));
    for (i, slot, at) in [(2usize, 15u32, 0u32), (3, 16, 0x14), (4, 17, 0x28)] {
        st.push(o);
        // call width: 12 (the most `Pushes::words` holds)
        let r = st0_f32(st.call(g, t(g, i, slot), 0).st0);
        let v = g.read::<f32>(m + at);
        let x = if slot == 17 && ops.first17 {
            mul(r, v)
        } else {
            mul(v, r)
        };
        g.write::<f32>(m + at, x);
    }
    st.pop(4);
    for (i, slot, at) in [(5usize, 330u32, 8u32), (6, 328, 0), (7, 329, 4)] {
        st.set_top(g.read::<u32>(turn + at));
        st.push_all(&[m, m]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, i, slot), 0);
        if slot != 329 {
            st.pop(2);
        }
    }
}

/// `magic_0307`'s summon start (0x100055e0, 1135 bytes; cdecl, no
/// arguments; `S = *G_S`): `r29::summon_start_0563`'s steps on 0307's
/// blocks with the model matrix written out. The turn
/// `T = (0, S[0x2868] - K wrapped once, 0)` into `S + 0xc20`; the matrix `M = S + 0xbe0` a
/// unit (slot 235), its diagonal slot 277 `(*G_O)`'s scale times `K0`,
/// each then times slots 15, 16, 17 `(*G_O)` (the last as `a M[0x28]`),
/// turned by `T.z`, `T.x`, `T.y` (slots 330, 328, 329); `S + 0xc10` =
/// `S + 0xc90`; the object `O` (operand `+0x1b2`) started
/// ([`start_object_at`](crate::s4_misc::start_object_at), part 2, owner
/// 0x11, scale `K1`) at `S + 0xc90`, its `y` less `K2`, turned by `T`,
/// `+0xe0` 10 and `+0xe8` 30; the lights `S + 0x23a4` (`(2, 1, 0)` twice,
/// 1/16, 0x10, 15) and `S + 0x23d4` (`(1.5, 0, 1)` twice, 0x3c2237c3,
/// 0x65, 0x7c) and the sprite fan record `S + 0x2598`, each with a pool
/// node ([`take`](crate::sa_pool::take), the lights' kind 1) on the task
/// list; a fourth node with the word 0x2e000a made the list's tail.
/// Returns the fourth node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_start_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x9));
    let yaw = g.read::<u32>(s + 0x2868);
    g.write::<u32>(s + 0xc20, 0);
    g.write::<u32>(s + 0xc24, yaw);
    g.write::<u32>(s + 0xc28, 0);
    let mut a = sub(g.read::<f32>(s + 0xc24), k(g, 0x3c));
    // comiss; jbe: unordered keeps the value
    if a > k(g, 0x43) {
        a = sub(a, k(g, 0x4d));
    } else if k(g, 0x57) > a {
        a = add(a, k(g, 0x64));
    }
    let m = s + 0xbe0;
    let mut st = Pushes::default();
    st.push(m);
    g.write::<f32>(s + 0xc24, a);
    model_matrix(g, img, f, &mut st, (m, s + 0xc20), &MATRIX_55E0);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0xc90 + 4 * i);
        g.write::<u32>(s + 0xc10 + 4 * i, x);
    }
    let obj = w(0x1b2);
    crate::s4_misc::start_object_at(g, img, call(0x1b7), obj, k(g, 0x179), 2, 0x11, 0);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0xc90 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(s + 0xc94), k(g, 0x1e7));
    for (o, from) in [(0xc0u32, 0xc20u32), (0xc4, 0xc24)] {
        let x = g.read::<u32>(s + from);
        g.write::<u32>(obj + o, x);
    }
    g.write::<f32>(obj + 0xd4, y);
    g.write::<u32>(obj + 0xe0, 0x4120_0000);
    g.write::<u32>(obj + 0xe8, 0x41f0_0000);
    // a pool node on the task list after `prev` (the head's operand, the
    // store of an empty list's head)
    let append = |g: &mut G, prev: u32, node: u32, listed: bool, first: u32| {
        if listed {
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(first), node);
        }
        g.write::<u32>(node + 8, node);
    };
    // the two lights
    let mut prev = 0;
    for (l, (step, time, count, flag), (c, owner, head, last, first)) in [
        (
            0x23a4u32,
            (
                (0x4000_0000u32, 0x3f80_0000u32, 0u32),
                0x3d80_0000u32,
                0x10u32,
                0xfu32,
            ),
            (0x276u32, 0x24bu32, 0x27cu32, 0x295u32, 0x2a8u32),
        ),
        (
            0x23d4,
            ((0x3fc0_0000, 0, 0x3f80_0000), 0x3c22_37c3, 0x65, 0x7c),
            (0x31c, 0x2f1, 0x322, 0, 0x348),
        ),
    ] {
        let r = s + l;
        for (o, v) in [
            (0u32, 0u32),
            (0x1c, 0),
            (0x20, time),
            (0x24, 0),
            (0x28, count),
        ] {
            g.write::<u32>(r + o, v);
        }
        g.write::<u16>(r + 0x2c, 0);
        let node = crate::sa_pool::take(g, img, call(c), w(owner));
        let listed = g.read::<u32>(w(head)) != 0;
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 0x14, r);
        g.write::<u32>(node + 4, 0);
        if last != 0 && listed {
            // the first appends after the list's tail
            prev = g.read::<u32>(w(last));
        }
        append(g, prev, node, listed, first);
        let (x, y, z) = step;
        for (o, v) in [(4u32, x), (8, y), (0xc, z), (0x10, x), (0x14, y), (0x18, z)] {
            g.write::<u32>(r + o, v);
        }
        let fl = g.read::<u16>(r + 0x2c) | 3;
        g.write::<u16>(r + 0x2c, fl);
        g.write::<u32>(r + 0x24, flag);
        prev = node;
    }
    // the sprite fan record
    let e = s + 0x2598;
    for (o, v) in [
        (0xf0u32, 0x4080_8080u32),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0x7c),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
        (0x108, 0x50),
        (0x104, 0x50),
        (0x10c, 8),
    ] {
        g.write::<u32>(e + o, v);
    }
    let n3 = crate::sa_pool::take(g, img, call(0x3f8), w(0x38f));
    let listed = g.read::<u32>(w(0x3fe)) != 0;
    g.write::<u32>(n3 + 0x14, e);
    g.write::<u32>(n3 + 4, 0);
    append(g, prev, n3, listed, 0x420);
    let n4 = crate::sa_pool::take(g, img, call(0x42d), w(0x425));
    let listed = g.read::<u32>(w(0x433)) != 0;
    g.write::<u32>(n4 + 0x14, 0x002e_000a);
    g.write::<u32>(n4 + 4, 0);
    g.write::<u32>(w(0x449), n4);
    append(g, n3, n4, listed, 0x463);
    Ret::eax(n4)
}

/// [`summon_start_0307`] (the `rel32`s: the object start, the pool take
/// four times).
pub const SUMMON_START_0307: Sig = Sig {
    len: 1135,
    hash: 0x5aec_6b7c_3860_a450,
    calls: &[
        (0x1b7, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x276, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x31c, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3f8, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x42d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`summon_turn_0307`]'s block.
const MATRIX_40D0: MatrixOps = MatrixOps {
    tables: [0x135, 0x15d, 0x182, 0x1af, 0x1cd, 0x1ec, 0x218, 0x237],
    actor: (0x163, 0x196),
    k: 0x17d,
    first17: false,
};

/// `magic_0307`'s summon turned to its place (0x100040d0, 1005 bytes;
/// cdecl, no arguments; an 8-aligned /GS frame: `B` = the aligned `esp`
/// after `sub esp, 0x6c`, the vector `V` at `B + 0x14`, the matrix `N` at
/// `B + 0x24`; `S = *G_S`). With `d = S[0xc60] - S[0xc70]` the pitch
/// `atan2(d.y, sqrt(d.z^2 + d.x^2))` and the yaw `K - atan2(d.z, d.x)`
/// (wrapped once) go to `S + 0xc20`/`+0xc24` and are then replaced: the
/// turn is `(-0.419, S[0x2868], 0)`. The model matrix `M = S + 0xbe0`
/// ([`model_matrix`]); `S + 0xc10` = `S + 0xc70`; `V = (0, 0, 120, 1)`
/// through `N` (a unit turned by the same angles; slots 235, 330, 328,
/// 329, 229) added to `S + 0xc70` into `S + 0xc80` (slot 248, `w` 1.0);
/// `S[0xc14]` less `C`; the block `S + 0xca0..0xcbc` = `(0, 0.5, 0, 0, 0,
/// 0.01, 0, 0)`; slot 543 `(0x28)`, whose `eax` it returns. Each call sees
/// the words the earlier ones left.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_turn_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::wrap_once;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0x6c
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x6c);
    let cookie = g.read::<u32>(w(0xa)) ^ b;
    g.write::<u32>(b + 0x68, cookie);
    let (v, n) = (b + 0x14, b + 0x24);
    let s = g.read::<u32>(w(0x17));
    let dy = sub(rf(g, s + 0xc64), rf(g, s + 0xc74));
    let dx = sub(rf(g, s + 0xc60), rf(g, s + 0xc70));
    let dz = sub(rf(g, s + 0xc68), rf(g, s + 0xc78));
    let flat = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let root = f64::from(flat).sqrt();
    // `_CIatan2` on x87 (`ST1` = y), each result rounded once
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), root) as f32;
    g.write::<f32>(s + 0xc20, pitch);
    #[allow(clippy::cast_possible_truncation)]
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = sub(g.read::<f32>(w(0xe2)), a);
    let yaw = wrap_once(g, img, f, yaw, [0xf1, 0xfb, 0x105, 0x112]);
    g.write::<f32>(s + 0xc24, yaw);
    g.write::<u32>(s + 0xc28, 0);
    let turn = g.read::<u32>(s + 0x2868);
    g.write::<u32>(s + 0xc24, turn);
    let m = s + 0xbe0;
    g.write::<u32>(s + 0xc20, 0xbed6_7750);
    g.write::<u32>(s + 0xc28, 0);
    let mut st = Pushes::default();
    st.push(m);
    model_matrix(g, img, f, &mut st, (m, s + 0xc20), &MATRIX_40D0);
    let p = s + 0xc70;
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0xc10 + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([0, 0, 0x42f0_0000, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    st.push(n);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x299, 235), 0);
    st.pop(3);
    for (table, slot, at) in [
        (0x2ccu32, 330u32, 0xc28u32),
        (0x2ef, 328, 0xc20),
        (0x312, 329, 0xc24),
    ] {
        st.set_top(g.read::<u32>(s + at));
        st.push_all(&[n, n]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, table, slot), 0);
        if slot != 329 {
            st.pop(2);
        }
    }
    st.push_all(&[v, n, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x32e, 229), 0);
    st.push_all(&[v, p, s + 0xc80]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x348, 248), 0);
    g.write::<u32>(s + 0xc8c, 0x3f80_0000);
    let y = sub(rf(g, s + 0xc14), g.read::<f32>(w(0x36f)));
    g.write::<f32>(s + 0xc14, y);
    for (i, x) in (0u32..).zip([0, 0x3f00_0000, 0, 0, 0, 0x3c23_d70a, 0, 0]) {
        g.write::<u32>(s + 0xca0 + 4 * i, x);
    }
    st.push(0x28);
    // call width: 12 (the most `Pushes::words` holds)
    let eax = st.call(g, t(g, 0x355, 543), 0).eax;
    Ret::eax(eax)
}

/// [`summon_turn_0307`] (the `rel32`s: the square root, `_CIatan2`
/// twice, `__security_check_cookie`).
pub const SUMMON_TURN_0307: Sig = Sig {
    len: 1005,
    hash: 0x8482_3a6b_3cbc_f1ba,
    calls: &[
        (0x7c, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x8f, Callee::Import("_CIatan2")),
        (0xd0, Callee::Import("_CIatan2")),
        (0x3e5, Callee::Any),
    ],
    imports: &[],
};

/// One of [`burst_step_0307`]'s two record kinds: the operands of its
/// calls in the code (`(table, value)`: a slot table and the float it is
/// given) and what differs.
struct Burst {
    take: u32,
    /// the part word: the table global's operand and the word's offset
    part: (u32, u32),
    rise: u32,
    spin: u32,
    s329: u32,
    /// `V.z`: slot 314 `(80)`, or slot 404 `(40)` plus a constant
    depth: (u32, u32, u32, Option<u32>),
    s229: u32,
    vx: (u32, u32, u32),
    vy: (u32, u32),
    vz: (u32, u32, u32),
    yaw: u32,
    colour: u32,
    size: (u32, u32, u32, u32),
    count: u32,
}

const BURSTS_0307: [Burst; 2] = [
    Burst {
        take: 0x71,
        part: (0x81, 0),
        rise: 0xc5,
        spin: 0xed,
        s329: 0x124,
        depth: (0x131, 314, 0x42a0_0000, None),
        s229: 0x157,
        vx: (0x16d, 0x3e99_999a, 0x1a5),
        vy: (0x1aa, 0x3e99_999a),
        vz: (0x1ce, 0x3e99_999a, 0x1f0),
        yaw: 0x1f5,
        colour: 0x3d4c_cccd,
        size: (0x219, 0x3e4c_cccd, 0x25a, 0x274),
        count: 8,
    },
    Burst {
        take: 0x2b6,
        part: (0x2c6, 0x14),
        rise: 0x30b,
        spin: 0x333,
        s329: 0x36a,
        depth: (0x377, 404, 0x4220_0000, Some(0x39f)),
        s229: 0x3b0,
        vx: (0x3cb, 0x3e4c_cccd, 0x403),
        vy: (0x408, 0x3e4c_cccd),
        vz: (0x42c, 0x3e4c_cccd, 0x44e),
        yaw: 0x453,
        colour: 0x3ca3_d70a,
        size: (0x477, 0x3e4c_cccd, 0x4b8, 0x4d2),
        count: 2,
    },
];

/// `magic_0307`'s late object's spark bursts (0x10004890, 1287 bytes;
/// cdecl `(O)`, the object's step callback; a /GS `ebp` frame: the
/// matrices `M` at `ebp - 0xa4` and `N` at `ebp - 0x64`, `V` at `ebp -
/// 0x24`, the turned `U` at `ebp - 0x14`). An object whose `+0x118` is above 0x14 gets `+0x104` 0 and
/// returns the cookie word. Else `M` a unit (slot 235), `V = (0, 0, -,
/// 1)`, and up to eight records of the 0x74-byte pool
/// ([`pool_take`](crate::sb_misc::pool_take); the first that fails ends
/// the kind, `eax` 0): the part word `*G_P[0]`, frame -1, time 0x200,
/// playing; at `O + 0xd0` raised by slot 314 `(100)`; `N` = `M` turned by
/// slot 314 `(pi)` (slot 329), `V.z` slot 314 `(80)`, `U` = slot 229 `(U, N, V)`
/// added to `x`/`z`; the speed `(U.x K + r, r', U.z K + r'')`, spin, the
/// colour words, the size `(r + C) K'`, the counts. Then, for an object
/// whose `+0x118` is at most 10, up to two more with `*G_P[5]`, `V.z` slot
/// 404 `(40)` plus a constant and their own constants. Returns slot 314's
/// last `eax` (or 0).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn burst_step_0307<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let o = fr.arg(g, 0);
    let age = |g: &G| g.read::<u32>(o.wrapping_add(0x118)).cast_signed();
    if age(g) > 0x14 {
        g.write::<u32>(o.wrapping_add(0x104), 0);
        return Ret::eax(cookie);
    }
    let (m, n, v, u) = (
        ebp.wrapping_sub(0xa4),
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    // call width: 1
    g.call(t(g, 0x42, 235), 0, &[m]);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    // a slot answering in ST0 (rounded to f32) for one float word
    let st0 = |g: &mut G, table: u32, slot: u32, x: u32| {
        // call width: 1
        let out = g.call(t(g, table, slot), 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    let mut eax = 0;
    for (kind, b) in BURSTS_0307.iter().enumerate() {
        if kind == 1 && age(g) > 0xa {
            break;
        }
        for _ in 0..b.count {
            let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + b.take));
            if p == 0 {
                eax = 0;
                break;
            }
            let (table, at) = b.part;
            let part = g.read::<u32>(g.read::<u32>(w(table)).wrapping_add(at));
            g.write::<u16>(p + 0x38, 0xffff);
            g.write::<u32>(p + 0x30, part);
            g.write::<u32>(p + 0x34, 1);
            g.write::<u16>(p + 0x3a, 0x200);
            for i in 0..4u32 {
                let x = g.read::<u32>(o.wrapping_add(0xd0 + 4 * i));
                g.write::<u32>(p + 4 * i, x);
            }
            let (r, _) = st0(g, b.rise, 314, 0x42c8_0000);
            g.write::<f32>(p + 4, add(g.read::<f32>(p + 4), r));
            let (a, _) = st0(g, b.spin, 314, 0x4049_0fdb);
            // call width: 3
            g.call(t(g, b.s329, 329), 0, &[n, m, a.to_bits()]);
            let (table, slot, x, plus) = b.depth;
            let (r, _) = st0(g, table, slot, x);
            let z = match plus {
                Some(k) => add(r, c(g, k)),
                None => r,
            };
            g.write::<f32>(v + 8, z);
            // call width: 3
            g.call(t(g, b.s229, 229), 0, &[u, n, v]);
            let (ux, uz) = (g.read::<f32>(u), g.read::<f32>(u + 8));
            g.write::<f32>(p, add(g.read::<f32>(p), ux));
            g.write::<f32>(p + 8, add(g.read::<f32>(p + 8), uz));
            let (table, x, k) = b.vx;
            let (r, _) = st0(g, table, 314, x);
            g.write::<f32>(p + 0x10, add(mul(ux, c(g, k)), r));
            let (r, _) = st0(g, b.vy.0, 314, b.vy.1);
            g.write::<f32>(p + 0x14, r);
            let (table, x, k) = b.vz;
            let (r, _) = st0(g, table, 314, x);
            g.write::<f32>(p + 0x18, add(mul(uz, c(g, k)), r));
            let (r, _) = st0(g, b.yaw, 314, 0x4049_0fdb);
            g.write::<f32>(p + 0x1c, r);
            g.write::<u32>(p + 0x20, 0);
            g.write::<u32>(p + 0x24, b.colour);
            g.write::<u32>(p + 0x28, 0);
            g.write::<u32>(p + 0x2c, 0x3f80_0000);
            let (table, x, ka, km) = b.size;
            let (r, e) = st0(g, table, 314, x);
            eax = e;
            let size = mul(add(r, c(g, ka)), c(g, km));
            if kind == 0 {
                g.write::<u32>(p + 0x40, 0x3f00_0000);
                g.write::<u32>(p + 0x58, 0x40);
                g.write::<u32>(p + 0x5c, 0);
                g.write::<u32>(p + 0x4c, 0x40);
                g.write::<f32>(p + 0x3c, size);
                g.write::<u32>(p + 0x50, 0x40);
            } else {
                g.write::<u32>(p + 0x4c, 0x18);
                g.write::<u32>(p + 0x50, 0xc);
                g.write::<u32>(p + 0x54, 0xffff_fff4);
                g.write::<f32>(p + 0x3c, size);
            }
            g.write::<u8>(p + 0x6b, 0);
            g.write::<u32>(p + 0x60, 0);
        }
    }
    Ret::eax(eax)
}

/// [`burst_step_0307`] (the `rel32`s: `__security_check_cookie` twice,
/// the pool take twice).
pub const BURST_STEP_0307: Sig = Sig {
    len: 1287,
    hash: 0xd28f_5a87_8ea6_ddba,
    calls: &[
        (0x31, Callee::Any),
        (0x71, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x2b6, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x4ff, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0055`'s start-up (0x10001340, 989 bytes; internal, no arguments,
/// run by its load step [`load_0055`]; an 8-aligned /GS frame, `B` = the
/// aligned `esp` after `sub esp, 0x90` and two pushes: `Q` at `B + 0x10`,
/// the turned offset `O` at `B + 0x20`, `P` at `B + 0x30`, `V` at `B +
/// 0x40`, `M` at `B + 0x50`; `A = *G_A`, `T = TAB[*I]`):
/// - two globals, `*G_C` = slot 719 `(NAME, 8, 0)`, the part set-up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - `P` = slot 262 `(A, ..)`, `Q` = slot 262 `(T, ..)` with `Q.y = P.y`
///   (`magic_0067`'s copy keeps `Q.y`); the yaw
///   `K - atan2(Q.z - P.z, Q.x - P.x)` (`_CIatan2`, x87) wrapped once; the height `h` = slot 251
///   `(*I)` outside a battle with `*I` below 0x20, else slot 17 `(T)`
///   times slot 19 `(T)` (ST0 each);
/// - `M` from slot 235 turned by slots 328 `(M, M, 0)` and 329 `(M, M,
///   yaw)`; `Q += M (0, 0, h + C, 1)` (slots 229, 248); `n = trunc(|P - Q|
///   K2)` (the root in double);
/// - the camera path task ([`camera_path_task`](crate::s7_misc::camera_path_task),
///   `(Q, 1.1, 1.0, n)`, scale `K3`);
/// - the actor block (`TAB2[i] - 7 + n` at least 1, slot 413 or 215
///   `(*G_X)` by `*G_B`, -1, 0, 1, the u16 flag by `*G_B`);
/// - a task node ([`take`](crate::sa_pool::take), owner `CB`) last on the
///   task list.
///
/// Returns 0.
pub fn start_up_0055<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_55(g, img, fr, true)
}

/// [`start_up_0055`] in `magic_0067` (0x10003a60, 977 bytes): `Q.y` kept
/// (the two moves gone, every later operand 12 bytes earlier).
pub fn start_up_0067<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_up_55(g, img, fr, false)
}

#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_up_55<G: Game>(g: &mut G, img: &Image, fr: &Frame, level: bool) -> Ret {
    use crate::sb_sse::{add, cvtt, mul, sub};
    let f = fr.at;
    let at = |o: u32| if level || o < 0xa8 { o } else { o - 0xc };
    let w = |o: u32| img.word(f + at(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + at(o));
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let one = 0x3f80_0000u32;
    // push ebp; and esp, -8; sub esp, 0x90; two pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x90);
    let cookie = g.read::<u32>(w(0xd)) ^ e;
    g.write::<u32>(e + 0x8c, cookie);
    let b = e.wrapping_sub(8);
    let (q, o, p, v, m) = (b + 0x10, b + 0x20, b + 0x30, b + 0x40, b + 0x50);
    let s719 = t(g, 0x1b, 719);
    g.write::<u32>(w(0x32), w(0x36));
    g.write::<u32>(w(0x3c), w(0x40));
    // call width: 3
    let c = g.call(s719, 0, &[w(0x2c), 8, 0]).eax;
    g.write::<u32>(w(0x4a), c);
    crate::l5_objhost::part_init(g, img, call(0x4f));
    let a = g.read::<u32>(w(0x69));
    // call width: 4
    g.call(t(g, 0x63, 262), 0, &[a, p, p + 4, p + 8]);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x76))));
    g.write::<u32>(p + 0xc, one);
    let tt = g.read::<u32>(w(0x97).wrapping_add(4 * i));
    // call width: 4
    g.call(t(g, 0x9c, 262), 0, &[tt, q, q + 4, q + 8]);
    if level {
        let py = g.read::<u32>(p + 4);
        g.write::<u32>(q + 4, py);
    }
    let dz = sub(rd(g, q + 8), rd(g, p + 8));
    g.write::<u32>(q + 0xc, one);
    let dx = sub(rd(g, q), rd(g, p));
    // `_CIatan2(dz, dx)`, one rounding (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = crate::ob_model::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x104), ang),
        [0x113, 0x123, 0x12d, 0x13a].map(at),
    );
    g.write::<f32>(b + 8, yaw);
    let battle = g.read::<u32>(w(0x146)) != 0;
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x14d))));
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let h = if !battle && i < 0x20 {
        // call width: 1
        let x = st0_f32(g.call(t(g, 0x16d, 251), 0, &[i]).st0);
        g.write::<f32>(b + 0xc, x);
        x
    } else {
        let tt = g.read::<u32>(w(0x18e).wrapping_add(4 * i));
        // call width: 1
        let a19 = st0_f32(g.call(t(g, 0x193, 19), 0, &[tt]).st0);
        g.write::<f32>(o, a19);
        // call width: 1
        let a17 = st0_f32(g.call(t(g, 0x1a2, 17), 0, &[tt]).st0);
        g.write::<f32>(b + 0xc, a17);
        mul(a17, a19)
    };
    g.write::<f32>(v + 8, add(h, k(g, 0x1c3)));
    // call width: 1
    g.call(t(g, 0x1cd, 235), 0, &[m]);
    // call width: 3
    g.call(t(g, 0x1ed, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(t(g, 0x20e, 329), 0, &[m, m, yaw.to_bits()]);
    g.write::<u32>(v + 0xc, one);
    // call width: 3
    g.call(t(g, 0x22a, 229), 0, &[o, m, v]);
    // call width: 3
    g.call(t(g, 0x24a, 248), 0, &[q, q, o]);
    g.write::<u32>(q + 0xc, one);
    // `subps` P - Q lane by lane, y first in the sum
    let d = [0u32, 4, 8].map(|j| sub(rd(g, p + j), rd(g, q + j)));
    let sum = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
    #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
    let root = f64::from(sum).sqrt() as f32;
    let base = g.read::<u32>(w(0x2ac).wrapping_add(4 * g.read::<u32>(w(0x29d))));
    let n = cvtt(mul(root, k(g, 0x2bc))).cast_unsigned();
    // the path task's `(Q, 1.1, 1.0, n)`, `xmm3` = `K3`
    for (j, x) in (0u32..).zip([q, 0x3f8c_cccd, one, n]) {
        g.write::<u32>(b.wrapping_sub(0x10) + 4 * j, x);
    }
    let path = Frame {
        at: call(0x2d9),
        esp: b.wrapping_sub(0x14),
        eax: q,
        ecx: 0,
        edx: 0,
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x2a5)).reg()],
    };
    crate::s7_misc::camera_path_task(g, img, &path);
    let count = base.wrapping_sub(7).wrapping_add(n);
    g.write::<u32>(w(0x2e7), base);
    let count = if count.cast_signed() <= 0 { 1 } else { count };
    g.write::<u32>(w(0x2f7), 1);
    g.write::<u32>(w(0x300), count);
    g.write::<u32>(w(0x306), 0);
    let battle = g.read::<u32>(w(0x310)) != 0;
    let x = g.read::<u32>(w(0x316));
    g.write::<u32>(w(0x31b), x);
    g.write::<u32>(w(0x327), 1);
    g.write::<u32>(w(0x32d), w(0x331));
    g.write::<u32>(w(0x337), u32::MAX);
    // call width: 1
    let r = g
        .call(t(g, 0x321, if battle { 413 } else { 215 }), 0, &[x])
        .eax;
    let battle = g.read::<u32>(w(0x356)) != 0;
    g.write::<u32>(w(0x35c), r);
    g.write::<u32>(w(0x362), u32::MAX);
    g.write::<u32>(w(0x36c), 0);
    g.write::<u32>(w(0x376), 1);
    if battle {
        g.write::<u16>(w(0x389), 1);
    } else {
        g.write::<u16>(w(0x380), 0);
    }
    let node = crate::sa_pool::take(g, img, call(0x393), w(0x38e));
    let listed = g.read::<u32>(w(0x399)) != 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(0x3a9));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3bb), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x3cc), node);
    Ret::eax(0)
}

/// [`start_up_0055`] (the `rel32`s: the part set-up, `_CIatan2`, the
/// square root, the camera path task, the pool take,
/// `__security_check_cookie`).
pub const START_UP_0055: Sig = Sig {
    len: 989,
    hash: 0xe24e_f1e6_6f86_aa46,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xf2, Callee::Import("_CIatan2")),
        (0x298, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2d9, Callee::Sig(&crate::s7_misc::CAMERA_PATH_TASK)),
        (0x393, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3d5, Callee::Any),
    ],
    imports: &[],
};

/// [`start_up_0067`].
pub const START_UP_0067: Sig = Sig {
    len: 977,
    hash: 0x3129_3e76_20fe_6b65,
    calls: &[
        (0x4f, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xe6, Callee::Import("_CIatan2")),
        (0x28c, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2cd, Callee::Sig(&crate::s7_misc::CAMERA_PATH_TASK)),
        (0x387, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3c9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0055`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0055`].
pub fn load_0055<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0055)
}

/// [`load_0055`].
pub const LOAD_0055: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0055))],
    imports: &[],
};

/// `magic_0067`'s load step (0x10003930, the 118-byte shape) running
/// [`start_up_0067`].
pub fn load_0067<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0067)
}

/// [`load_0067`].
pub const LOAD_0067: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0067))],
    imports: &[],
};

/// `magic_0067` 0x100040c0's operands against `magic_0055` 0x100019a0's
/// (the search's operands have none).
const PATH_0067: Shift = Shift {
    runs: &[],
    moved: &[
        (0x6e, 0x2c),
        (0x74, 0x32),
        (0x87, 0x3f),
        (0xa8, 0x60),
        (0xad, 0x65),
        (0xf7, 0xaf),
        (0x109, 0xc1),
        (0x119, 0xd1),
        (0x120, 0xd8),
        (0x136, 0xee),
        (0x1b9, 0x107),
        (0x1d7, 0x125),
        (0x1f0, 0x13e),
        (0x1f8, 0x146),
        (0x20c, 0x15a),
        (0x21a, 0x168),
        (0x222, 0x170),
        (0x232, 0x180),
        (0x237, 0x185),
        (0x24b, 0x199),
        (0x25b, 0x1a9),
        (0x265, 0x1b3),
        (0x26f, 0x1bd),
        (0x27c, 0x1ca),
        (0x283, 0x1dc),
        (0x290, 0x1d2),
        (0x298, 0x1d7),
        (0x2ad, 0x1ec),
        (0x32c, 0x26b),
        (0x33e, 0x27d),
        (0x34d, 0x28c),
        (0x357, 0x296),
        (0x361, 0x2a0),
        (0x36e, 0x2ad),
        (0x3b1, 0x2f5),
        (0x3b7, 0x2fb),
        (0x3bd, 0x301),
        (0x3c1, 0x305),
        (0x3c7, 0x30b),
        (0x3d3, 0x317),
        (0x3d9, 0x31d),
        (0x3df, 0x323),
        (0x3e9, 0x32d),
        (0x408, 0x34c),
        (0x413, 0x357),
        (0x419, 0x35d),
        (0x423, 0x367),
        (0x42d, 0x371),
        (0x437, 0x37b),
        (0x440, 0x384),
        (0x446, 0x38a),
        (0x44b, 0x38f),
        (0x452, 0x396),
        (0x45b, 0x39f),
        (0x46e, 0x3b2),
        (0x48b, 0x3cf),
        (0x493, 0x3d7),
        (0x4c4, 0x408),
        (0x4cf, 0x413),
        (0x4d5, 0x419),
        (0x4e8, 0x42c),
        (0x4fb, 0x43f),
        (0x50d, 0x450),
    ],
};

/// `magic_0055`'s camera path points (0x100019a0, 1309 bytes; internal,
/// no arguments, called by [`steps_0055`]; an 8-aligned /GS frame, `B` =
/// the aligned `esp` after `sub esp, 0xa4` and three pushes: `Q` at `B +
/// 0x28`, `P` at `B + 0x38`, `U` at `B + 0x48`, `M` at `B + 0x68`; `A =
/// *G_A`, `T = TAB[*I]`):
/// - slot 390 `(*I)` searched in the table `L` (to its -1): `found`, kept
///   in `*G_F`;
/// - `P` = slot 262 `(A, ..)`, `Q` = slot 262 `(T, ..)`; the yaw `K -
///   atan2(P.z - Q.z, P.x - Q.x)` wrapped once, replaced (when found) by
///   `K - slot 333 (T)` wrapped;
/// - `M` from slot 235 turned by slot 329 `(M, M, yaw)`; `Q` copied to the
///   first record `R0`, its `w` = `K - slot 333 (T)` wrapped;
/// - five records `R1..R5` (16 bytes from `R0 + 0x10`): `U` = slot 229
///   `(U, M, V_k)` (`V` the table `V1`, or `V2` when found), the record
///   `Q + U` with its `w` the yaw `K - atan2(dz, dx)` from the record
///   before (wrapped);
/// - the actor block (slot 413 or 215 `(*I)` by `*G_B`, -1, 0, 1, the u16
///   flag by `*G_B`, 60 and 0 at two more globals);
/// - [`task_start`](crate::s3_misc::task_start) `(R, 1)`; a node off the
///   free ring ([`pool_take_0587`](crate::r27::pool_take_0587), the step
///   `CB`, kind 1) last on the task list.
///
/// Returns the list's old tail (0xffff when it was empty).
/// `magic_0067` 0x100040c0 is the same code without the search
/// ([`path_points_0067`]).
pub fn path_points_0055<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    path_points_with(g, img, f, esp, None)
}

/// [`path_points_0055`] in `magic_0067` (0x100040c0, 1120 bytes): no
/// search (never found), the operands at [`PATH_0067`]'s places.
pub fn path_points_0067<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    path_points_with(g, img, f, esp, Some(&PATH_0067))
}

#[allow(clippy::too_many_lines)] // the original's steps, in order
fn path_points_with<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    copy: Option<&Shift>,
) -> u32 {
    use crate::sb_sse::{add, sub};
    let at = |o: u32| copy.map_or(o, |c| c.at(o));
    let w = |o: u32| img.word(f + at(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let one = 0x3f80_0000u32;
    let wrap = |g: &G, x: f32, [hi, turn, lo, turn2]: [u32; 4]| {
        // comiss; jbe: unordered keeps the value
        if x > k(g, hi) {
            sub(x, k(g, turn))
        } else if k(g, lo) > x {
            add(x, k(g, turn2))
        } else {
            x
        }
    };
    // push ebp; and esp, -8; sub esp, 0xa4 and three pushes (0067: 0xa8
    // and two): the cookie at `B + 0xac` either way, xored with the `esp`
    // after the `sub`
    let b = (esp.wrapping_sub(4) & !7).wrapping_sub(0xb0);
    let e = if copy.is_none() { b + 0xc } else { b + 8 };
    let cookie = g.read::<u32>(w(0xd)) ^ e;
    g.write::<u32>(b + 0xac, cookie);
    let (q, p, u, m) = (b + 0x28, b + 0x38, b + 0x48, b + 0x68);
    let idx = |g: &G, op: u32| u32::from(g.read::<u8>(g.read::<u32>(w(op))));
    let mut found = false;
    if copy.is_none() {
        let i = idx(g, 0x1b);
        // call width: 1
        let r = g.call(t(g, 0x27, 390), 0, &[i]).eax;
        let mut at = w(0x3f);
        loop {
            let x = g.read::<u32>(at);
            if x == u32::MAX {
                break;
            }
            if x == r {
                found = true;
                break;
            }
            at = at.wrapping_add(4);
        }
        g.write::<u32>(w(0x80), u32::from(found));
    }
    let a = g.read::<u32>(w(0x74));
    // call width: 4
    g.call(t(g, 0x6e, 262), 0, &[a, p, p + 4, p + 8]);
    let i = idx(g, 0x87);
    g.write::<u32>(p + 0xc, one);
    let tt = g.read::<u32>(w(0xa8).wrapping_add(4 * i));
    // call width: 4
    g.call(t(g, 0xad, 262), 0, &[tt, q, q + 4, q + 8]);
    let dz = sub(rd(g, p + 8), rd(g, q + 8));
    g.write::<u32>(q + 0xc, one);
    let dx = sub(rd(g, p), rd(g, q));
    // `_CIatan2(dz, dx)`, one rounding (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = wrap(g, sub(k(g, 0x109), ang), [0x120, 0x119, 0x136, 0x119]);
    g.write::<f32>(b + 0xc, yaw);
    if found {
        let tt = g.read::<u32>(w(0x159).wrapping_add(4 * idx(g, 0x14f)));
        // call width: 1
        let v = st0_f32(g.call(t(g, 0x15e, 333), 0, &[tt]).st0);
        yaw = wrap(g, sub(k(g, 0x172), v), [0x182, 0x192, 0x19c, 0x1a9]);
        g.write::<f32>(b + 0xc, yaw);
    }
    // call width: 1
    g.call(t(g, 0x1b9, 235), 0, &[m]);
    // call width: 3
    g.call(t(g, 0x1d7, 329), 0, &[m, m, yaw.to_bits()]);
    let (qx, qy, qz, qw) = (
        g.read::<u32>(q),
        g.read::<u32>(q + 4),
        g.read::<u32>(q + 8),
        g.read::<u32>(q + 0xc),
    );
    g.write::<u32>(w(0x1f8), qx);
    g.write::<u32>(w(0x20c), qy);
    g.write::<u32>(w(0x21a), qz);
    g.write::<u32>(w(0x222), qw);
    let mut prev = (f32::from_bits(qx), f32::from_bits(qz));
    let tt = g.read::<u32>(w(0x232).wrapping_add(4 * idx(g, 0x1f0)));
    // call width: 1
    let v = st0_f32(g.call(t(g, 0x237, 333), 0, &[tt]).st0);
    let yaw3 = wrap(g, sub(k(g, 0x24b), v), [0x25b, 0x265, 0x26f, 0x27c]);
    g.write::<f32>(w(0x290), yaw3);
    let mut vk = if found { w(0x288) } else { w(0x283) };
    let mut r = w(0x298);
    for _ in 0..5 {
        r = r.wrapping_add(0x10);
        // call width: 3
        g.call(t(g, 0x2ad, 229), 0, &[u, m, vk]);
        let x = add(rd(g, u), rd(g, q));
        g.write::<f32>(r - 8, x);
        let y = add(rd(g, u + 4), rd(g, q + 4));
        g.write::<f32>(r - 4, y);
        let z = add(rd(g, u + 8), rd(g, q + 8));
        g.write::<f32>(r, z);
        g.write::<u32>(r + 4, qw);
        let dz = sub(z, prev.1);
        let dx = sub(x, prev.0);
        // `_CIatan2(dz, dx)`, one rounding
        #[allow(clippy::cast_possible_truncation)]
        let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = wrap(g, sub(k(g, 0x33e), ang), [0x34d, 0x357, 0x361, 0x36e]);
        g.write::<f32>(r + 4, yaw);
        vk = vk.wrapping_add(0x10);
        prev = (x, z);
    }
    // the actor block
    let battle = g.read::<u32>(w(0x3b1)) != 0;
    g.write::<u32>(w(0x3bd), w(0x3c1));
    g.write::<u32>(w(0x3c7), 0);
    let i = idx(g, 0x3b7);
    g.write::<u32>(w(0x3d3), i);
    g.write::<u32>(w(0x3df), 0x3c);
    g.write::<u32>(w(0x3e9), 0);
    // call width: 1
    let r = g
        .call(t(g, 0x3d9, if battle { 413 } else { 215 }), 0, &[i])
        .eax;
    let battle = g.read::<u32>(w(0x408)) != 0;
    g.write::<u32>(w(0x413), r);
    g.write::<u32>(w(0x419), u32::MAX);
    g.write::<u32>(w(0x423), 0);
    g.write::<u32>(w(0x42d), 1);
    if battle {
        g.write::<u16>(w(0x440), 1);
    } else {
        g.write::<u16>(w(0x437), 0);
    }
    // `task_start(R, 1)`: its return address below the two words
    let ts = b.wrapping_sub(0xc);
    g.write::<u32>(ts + 4, w(0x446));
    g.write::<u32>(ts + 8, 1);
    let start = Frame {
        at: img.rel32_target(f + at(0x44b)),
        esp: ts,
        eax: 0,
        ecx: 1,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s3_misc::task_start(g, img, &start);
    g.write::<u32>(w(0x45b), 0);
    let ops = [0x452, 0x46e, 0x48b, 0x493, 0x4c4, 0x4cf].map(at);
    let (node, eax) = crate::r27::pool_take_0587(g, img, f, ops, 0);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    append_last(g, img, f, node, [0x4d5, 0x4e8, 0x4fb, 0x50d].map(at), eax)
}

/// [`path_points_0055`] (the `rel32`s: `_CIatan2` twice, the task start,
/// `__security_check_cookie`).
pub const PATH_POINTS_0055: Sig = Sig {
    len: 1309,
    hash: 0xbd02_0621_d0a4_0901,
    calls: &[
        (0xf7, Callee::Import("_CIatan2")),
        (0x32c, Callee::Import("_CIatan2")),
        (0x44b, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x515, Callee::Any),
    ],
    imports: &[],
};

/// [`path_points_0067`].
pub const PATH_POINTS_0067: Sig = Sig {
    len: 1120,
    hash: 0x242c_2f9e_5d49_5afb,
    calls: &[
        (0xaf, Callee::Import("_CIatan2")),
        (0x26b, Callee::Import("_CIatan2")),
        (0x38f, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x458, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0055`'s camera path step (0x10001720, 547 bytes; cdecl `(N)`,
/// the start-up's task node; `ebp - 4` a local): while the first count
/// `*G_C1` is above 0 it counts down, and on reaching 0 the actor block
/// (`*G_X`, slot 413 or 215 by `*G_B`, -1, 0, 1, the u16 flag), the
/// object `O` started ([`start_object_anim_first`](crate::s4_misc::start_object_anim_first),
/// the first part, value `TAB[*G_X]`, scale `K`) and turned by `K' - slot 333
/// (*G_A)` wrapped, its record words set and the second count `*G_C2` =
/// `TAB2[*G_X] + TAB[*G_X]`. Else the second count counts down, and on
/// reaching 0 the path points run ([`path_points_0055`]) when bit 8 of
/// `*G_F` is set, else slot 239 `(*G_X, -5, 0)` and `*G_D = 1`. Then the
/// two motion scripts (`R1`, `R2`,
/// [`motion_script`](crate::s4_misc::motion_script)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)) run; when the
/// objects answer 1, `*G_D` is 1 and `R1`'s script 1, the node's `+0x14`
/// = 3 and its owner's step `+0x10` = `CB`. Returns the objects' answer
/// (1 past the first test), or the owner.
pub fn steps_0055<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    steps_with(g, img, fr, path_points_0055)
}

/// [`steps_0055`] in `magic_0067` (0x10003e40, the same code calling
/// [`path_points_0067`]).
pub fn steps_0067<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    steps_with(g, img, fr, path_points_0067)
}

#[allow(clippy::too_many_lines)] // the original's steps, in order
fn steps_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    path: fn(&mut G, &Image, u32, u32) -> u32,
) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let c1 = g.read::<u32>(w(0x5));
    if c1.cast_signed() > 0 {
        let c1 = c1.wrapping_sub(1);
        g.write::<u32>(w(0x14), c1);
        if c1 == 0 {
            let battle = g.read::<u32>(w(0x20)) != 0;
            let x = g.read::<u32>(w(0x26));
            g.write::<u32>(w(0x2b), x);
            g.write::<u32>(w(0x37), w(0x3b));
            g.write::<u32>(w(0x41), u32::MAX);
            // call width: 1
            let r = g
                .call(t(g, 0x31, if battle { 413 } else { 215 }), 0, &[x])
                .eax;
            let battle = g.read::<u32>(w(0x60)) != 0;
            g.write::<u32>(w(0x6b), r);
            g.write::<u32>(w(0x71), u32::MAX);
            g.write::<u32>(w(0x7b), 0);
            g.write::<u32>(w(0x85), 1);
            if battle {
                g.write::<u16>(w(0x98), 1);
            } else {
                g.write::<u16>(w(0x8f), 0);
            }
            let i = g.read::<u32>(w(0x9d));
            let v = g.read::<u32>(w(0xac).wrapping_add(4 * i));
            crate::s4_misc::start_object_anim_first(g, img, call(0xbb), w(0xb4), k(g, 0xa5), v);
            let a = g.read::<u32>(w(0xc6));
            // call width: 1
            let r = st0_f32(g.call(t(g, 0xc0, 333), 0, &[a]).st0);
            g.write::<f32>(ebp.wrapping_sub(4), r);
            let mut yaw = sub(k(g, 0xd9), r);
            // comiss; jbe: unordered keeps the value
            if yaw > k(g, 0xe8) {
                yaw = sub(yaw, k(g, 0xf2));
            } else if k(g, 0xfc) > yaw {
                yaw = add(yaw, k(g, 0x109));
            }
            g.write::<u16>(w(0x111), 0);
            g.write::<f32>(w(0x11e), yaw);
            let i = g.read::<u32>(w(0x116));
            let c2 = g.read::<u32>(w(0x125).wrapping_add(4 * i)).wrapping_add(v);
            g.write::<u32>(w(0x12d), w(0x131));
            for o in [0x137u32, 0x141, 0x14b, 0x155] {
                g.write::<u32>(w(o), 0);
            }
            g.write::<u32>(w(0x15f), w(0x163));
            g.write::<u32>(w(0x169), w(0x16d));
            g.write::<u32>(w(0x173), 0);
            g.write::<u32>(w(0x17d), 0);
            g.write::<u32>(w(0x187), w(0x18b));
            g.write::<u32>(w(0x190), c2);
        }
    } else {
        let c2 = g.read::<u32>(w(0x197));
        if c2.cast_signed() > 0 {
            let c2 = c2.wrapping_sub(1);
            g.write::<u32>(w(0x1a1), c2);
            if c2 == 0 {
                if g.read::<u32>(w(0x1a9)) & 0x100 != 0 {
                    // the call's return address after three pushes
                    path(g, img, call(0x1b4), fr.esp.wrapping_sub(0x10));
                } else {
                    let x = g.read::<u32>(w(0x1cb));
                    // call width: 3
                    g.call(t(g, 0x1bb, 239), 0, &[x, 0xffff_fffb, 0]);
                    g.write::<u32>(w(0x1d6), 1);
                }
            }
        }
    }
    let r1 = crate::s4_misc::motion_script(g, img, call(0x1e4), w(0x1df));
    crate::s4_misc::motion_script(g, img, call(0x1f0), w(0x1e9));
    // the call's return address after three pushes
    let objs = crate::l5_objhost::run_objects(g, img, call(0x1f5), fr.esp.wrapping_sub(0x10));
    if objs != 1 || g.read::<u32>(w(0x200)) != 1 || r1 != 1 {
        return Ret::eax(objs);
    }
    let n = fr.arg(g, 0);
    g.write::<u32>(n.wrapping_add(0x14), 3);
    let owner = g.read::<u32>(n.wrapping_add(8));
    g.write::<u32>(owner.wrapping_add(0x10), w(0x21a));
    Ret::eax(owner)
}

/// [`steps_0055`] (the `rel32`s: the object start, the path points, the
/// motion script twice, the objects).
pub const STEPS_0055: Sig = Sig {
    len: 547,
    hash: 0x1a50_d9c6_c5fe_8c19,
    calls: &[
        (0xbb, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM_FIRST)),
        (0x1b4, Callee::Sig(&PATH_POINTS_0055)),
        (0x1e4, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1f0, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1f5, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`steps_0067`].
pub const STEPS_0067: Sig = Sig {
    len: 547,
    hash: 0x1a50_d9c6_c5fe_8c19,
    calls: &[
        (0xbb, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM_FIRST)),
        (0x1b4, Callee::Sig(&PATH_POINTS_0067)),
        (0x1e4, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1f0, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1f5, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// `magic_0162`'s effect end (0x10002a80, 734 bytes; internal, no
/// arguments, called by [`host_start_end_0162`]; a cookie frame, `ebp =
/// E - 4`, the positions at `ebp - 0x14` and `ebp - 0x24`):
/// `r23::effect_end_0598`'s steps (its operands from `+0x34` on 0x3f bytes
/// later) after slot 384 of the third selected character too and slots 25
/// `(0x3011, 1)`, 24 `(0x3011, 0)`, and the resource parts registered at
/// the end ([`register_resource_parts`](crate::s4_small::register_resource_parts)),
/// whose `eax` it returns.
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn effect_end_0162<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::ob_model::{Who, who};
    let at = |o: u32| if o < 0x34 { o } else { o + 0x3f };
    let w = |o: u32| img.word(f + at(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    for (sel, n, table, slot) in [(0x11u32, 1u32, 0x1eu32, 0x23u32), (0x30, 2, 0x3b, 0x40)] {
        let c = who(g, img, f, Who::Sel { sel, at: n, table });
        // call width: 1
        g.call(slot_target(g, img.word(f + slot), 384), 0, &[c]);
    }
    for (o, slot, id, on) in [
        (0x4du32, 25u32, 0x3011u32, 1u32),
        (0x5e, 24, 0x3011, 0),
        (0x6f, 25, 0x3013, 1),
        (0x80, 24, 0x3013, 0),
    ] {
        // call width: 2
        g.call(slot_target(g, img.word(f + o), slot), 0, &[id, on]);
    }
    let selp = g.read::<u32>(w(0x53));
    let k = u32::from(g.read::<u8>(selp));
    let caster = g.read::<u32>(w(0x67).wrapping_add(4 * k));
    crate::r23::reset_one(
        g,
        img,
        f,
        (at(0x5c), at(0x7a), at(0x8c), at(0x9d)),
        caster,
        |g| u32::from(g.read::<u8>(selp)),
        ebp.wrapping_sub(0x14),
    );
    let summon = g.read::<u32>(w(0xae));
    crate::r23::reset_one(
        g,
        img,
        f,
        (at(0xb7), at(0xce), at(0xe0), at(0xf1)),
        summon,
        |g| g.read::<u32>(w(0xc0)),
        ebp.wrapping_sub(0x24),
    );
    if g.read::<u16>(w(0x106)) == 0 {
        let a = g.read::<u32>(w(0x118));
        // call width: 2
        g.call(t(g, 0x110, 213), 0, &[a, 0]);
    }
    let wait_b = g.read::<u16>(w(0x12a));
    g.write::<u32>(w(0x131), 1);
    if wait_b == 0 {
        let b = g.read::<u32>(w(0x144));
        // call width: 2
        g.call(t(g, 0x13c, 213), 0, &[b, 0]);
    }
    let off = g.read::<u32>(w(0x155));
    let selp = g.read::<u32>(w(0x15b));
    g.write::<u32>(w(0x161), 1);
    let c = u32::from(g.read::<u8>(selp));
    if off == 0 && c < 0x20 {
        // call width: 2
        g.call(t(g, 0x174, 296), 0, &[c, 1]);
    } else {
        let chr = g.read::<u32>(w(0x18b).wrapping_add(4 * c));
        // call width: 2
        g.call(t(g, 0x190, 257), 0, &[chr, 0]);
    }
    let off = g.read::<u32>(w(0x1a1)) != 0;
    let i = if off { 0 } else { g.read::<u32>(w(0x1a9)) };
    if !off && i.cast_signed() < 0x20 {
        // call width: 2
        g.call(t(g, 0x1b6, 296), 0, &[i, 1]);
    } else {
        let summon = g.read::<u32>(w(0x1cb));
        // call width: 2
        g.call(t(g, 0x1c3, 257), 0, &[summon, 0]);
    }
    let summon = g.read::<u32>(w(0x1e9));
    // call width: 2
    g.call(t(g, 0x1d8, 399), 0, &[summon, 0]);
    let summon = g.read::<u32>(w(0x1f8));
    // call width: 4
    g.call(t(g, 0x1f0, 8), 0, &[summon, 1, summon, 0]);
    // the slot 399 and slot 8 words under the call: the return address at
    // `E - 0x3c`
    crate::sa_script::each_actor(
        g,
        img,
        img.rel32_target(f + at(0x207)),
        1,
        esp.wrapping_sub(0x3c),
    );
    // call width: 2
    g.call(t(g, 0x20c, 371), 0, &[0, 0]);
    // call width: 2
    g.call(t(g, 0x21d, 371), 0, &[1, 0]);
    // call width: 1
    g.call(t(g, 0x22e, 543), 0, &[0]);
    let selp = g.read::<u32>(w(0x23d));
    let c = u32::from(g.read::<u8>(selp));
    // call width: 2
    let mut eax = g.call(t(g, 0x248, 219), 0, &[c, 1]).eax;
    if eax != 0 {
        // call width: 2
        eax = g.call(t(g, 0x25f, 257), 0, &[eax, 0]).eax;
    }
    let node = g.read::<u32>(w(0x270));
    if node != 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + at(0x279)), node);
        g.write::<u32>(w(0x27f), 0);
    }
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + at(0x288)), eax);
    // its return address after `sub esp, 0x24` and two pushes
    let res = Frame {
        at: img.rel32_target(f + 0x2cc),
        esp: esp.wrapping_sub(0x34),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s4_small::register_resource_parts(g, img, &res).eax
}

/// [`effect_end_0162`] (the `rel32`s as `r23::EFFECT_END_0598`'s and the
/// resource parts).
pub const EFFECT_END_0162: Sig = Sig {
    len: 734,
    hash: 0x7908_6998_a20b_d25c,
    calls: &[
        (0x246, Callee::Sig(&crate::sa_script::EACH_ACTOR)),
        (0x2b8, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x2c7, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (
            0x2cc,
            Callee::Sig(&crate::s4_small::REGISTER_RESOURCE_PARTS),
        ),
        (0x2d6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162` 0x10001260 (309 bytes; a table entry, no arguments):
/// `r23::host_start_end_0598`'s code running [`effect_end_0162`].
pub fn host_start_end_0162<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    crate::sa_misc::host_start_calls(g, img, f);
    let t = slot_target(g, img.word(f + 0x106), 549);
    // call width: 1
    if g.call(t, 0, &[1]).eax != 0 {
        let t = slot_target(g, img.word(f + 0x11c), 550);
        // call width: 1
        g.call(t, 0, &[1]);
    }
    // the return address at `E - 4`
    effect_end_0162(g, img, img.rel32_target(f + 0x12e), fr.esp.wrapping_sub(4));
    Ret::eax(0)
}

/// [`host_start_end_0162`].
pub const HOST_START_END_0162: Sig = Sig {
    len: 309,
    hash: 0x7322_6dfb_363d_5347,
    calls: &[(0x12e, Callee::Sig(&EFFECT_END_0162))],
    imports: &[],
};

/// `magic_0263`'s height from a distance (0x100024d0, 241 bytes; `ecx` =
/// the record `R = [obj, out, B, e]`, an 8-aligned /GS frame, the locals
/// from `L` after `sub esp, 0x30`): `r21::height_0262` with the second
/// point given straight to slot 234 (no second slot 217). Nothing when
/// bit 0 of the object's `+0x90` is set (the cookie word left). Else with
/// the table `T = *G_T`: `c = T[(e + 0x4000) >> 4 & 0xfff]`, `q = T[e >>
/// 4 & 0xfff] / c`; slot 217 `(obj[0x50], A)` (`A = L + 0x1c`), slot 234
/// `(D, A, B)` (`D = L + 0xc`); `R + 4` = `|D| / c * q - D.y`.
pub fn height_0263<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rec = fr.ecx;
    let l = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let obj = g.read::<u32>(rec);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        return Ret::eax(g.read::<u32>(w(0xa)) ^ l);
    }
    let e = g.read::<i32>(rec + 0xc);
    let tab = g.read::<u32>(w(0x2c));
    let at = |i: i32| tab.wrapping_add(4 * (i.cast_unsigned() & 0xfff));
    let c = g.read::<f32>(at(e.wrapping_add(0x4000) >> 4));
    let q = div(g.read::<f32>(at(e >> 4)), c);
    let (a, d) = (l + 0x1c, l + 0xc);
    let model = g.read::<u32>(obj + 0x50);
    // call width: 2
    g.call(t(g, 0x5b, 217), 0, &[model, a]);
    let b = g.read::<u32>(rec + 8);
    // call width: 5 (slot 217's words stay)
    g.call(t(g, 0x84, 234), 0, &[d, a, b, model, a]);
    let [x, y, z] = [0u32, 4, 8].map(|o| g.read::<f32>(d + o));
    let s = add(add(mul(y, y), mul(x, x)), mul(z, z));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let r = f64::from(s).sqrt() as f32;
    let h = sub(mul(div(r, c), q), y);
    g.write::<f32>(rec + 4, h);
    Ret::eax(0)
}

/// [`height_0263`] (the `rel32`s: the square root and the cookie check).
pub const HEIGHT_0263: Sig = Sig {
    len: 241,
    hash: 0x0d1a_ae79_308b_80de,
    calls: &[
        (0xc1, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xe9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0263`'s centre of its objects (0x100061c0, 338 bytes; internal:
/// `ecx` = the record `R = [S, out, X]`; a cookie frame, `ebp = E - 4`, the
/// point `P` at `ebp - 0x24`, `Q` at `ebp - 0x14`): `P` = slot 325 `(P,
/// X)`; for each of the `n = S[0x4dc]` objects (their words from `S +
/// 0x6c`, 0x98 apart) `Q` = slot 531 `(word, Q)` less `P` (kept in `Q`),
/// summed; `out = P + sum (C / n)` (lane by lane; with no objects `0 x (C
/// / n)`, NaN for `n = 0`), `w` 1.0. Returns `out`.
pub fn centre_0263<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let r = fr.ecx;
    let (p, q) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    g.write::<u32>(ebp.wrapping_sub(0x34), r);
    let x = g.read::<u32>(r + 8);
    let s = g.read::<u32>(r);
    // call width: 2
    g.call(t(g, 0x21, 325), 0, &[p, x]);
    let mut sum = [0.0f32; 3];
    let n = g.read::<u32>(s + 0x4dc).cast_signed();
    let mut obj = s + 0x6c;
    for _ in 0..n.max(0) {
        let word = g.read::<u32>(obj);
        // call width: 2
        g.call(t(g, 0x65, 531), 0, &[word, q]);
        let dy = sub(rd(g, q + 4), rd(g, p + 4));
        let dz = sub(rd(g, q + 8), rd(g, p + 8));
        let dx = sub(rd(g, q), rd(g, p));
        g.write::<f32>(q + 4, dy);
        g.write::<f32>(q + 8, dz);
        g.write::<f32>(q, dx);
        sum = [add(dx, sum[0]), add(dy, sum[1]), add(dz, sum[2])];
        obj = obj.wrapping_add(0x98);
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let k = div(g.read::<f32>(w(0xee)), n as f32);
    let [ax, ay, az] = sum.map(|v| mul(v, k));
    g.write::<f32>(q + 4, ay);
    g.write::<f32>(q + 8, az);
    let out = g.read::<u32>(r + 4);
    g.write::<f32>(out, add(rd(g, p), ax));
    g.write::<f32>(out + 4, add(rd(g, p + 4), ay));
    g.write::<f32>(out + 8, add(rd(g, p + 8), az));
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    Ret::eax(out)
}

/// [`centre_0263`] (the `rel32`: the cookie check).
pub const CENTRE_0263: Sig = Sig {
    len: 338,
    hash: 0x68bf_fcc6_bc50_e75a,
    calls: &[(0x14a, Callee::Any)],
    imports: &[],
};

/// `magic_0284`'s landing check and puff (0x10001d90, 332 bytes; cdecl
/// `(E)`): `s7_misc::landing_puff`'s checks and move, with the object
/// taken from the 0x140-byte pool ([`pool_take_140`](crate::sb_misc::pool_take_140);
/// none: no start) and started ([`start_object_at`](crate::s4_misc::start_object_at),
/// part 0, scale `K5`) at the point; `E + 0x26c` = 0 and the smoke puff
/// ([`smoke_puff`](crate::s6_misc::smoke_puff), a tail jump with `ecx` =
/// `E`), whose `eax` it returns. Outside the box nothing (the entry's
/// `eax`).
#[allow(clippy::neg_cmp_op_on_partial_ord)] // `comiss`/`jbe`: unordered fails the test
pub fn landing_take_0284<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.arg(g, 0);
    let x = g.read::<f32>(e + 0xc);
    // `comiss`; `jbe`: unordered fails each test
    if !(x > k(g, 0xf) && k(g, 0x1d) > x) {
        return Ret::eax(fr.eax);
    }
    if !(g.read::<f32>(e + 0x14) > k(g, 0x32)) {
        return Ret::eax(fr.eax);
    }
    let y = g.read::<f32>(e + 0x10);
    if !(y >= 0.0) {
        return Ret::eax(fr.eax);
    }
    let fl = g.read::<u16>(e + 0x284) | 2;
    g.write::<u16>(e + 0x284, fl);
    let v = g.read::<f32>(e + 0x260);
    // `ucomiss`/`lahf`/`jnp`: only an ordered 0 skips the move
    if v != 0.0 {
        let neg = f32::from_bits(div(sub(y, v), v).to_bits() ^ g.read::<u32>(w(0x81)));
        let back = sub(k(g, 0x7a), neg);
        let dx = mul(g.read::<f32>(e + 0x25c), back);
        let nx = sub(g.read::<f32>(e + 0xc), dx);
        g.write::<f32>(e + 0xc, nx);
        let dz = mul(g.read::<f32>(e + 0x264), back);
        let nz = sub(g.read::<f32>(e + 0x14), dz);
        g.write::<f32>(e + 0x14, nz);
    }
    g.write::<u32>(e + 0x10, 0);
    for off in [0u32, 4, 8, 0xc] {
        let p = g.read::<u32>(e + 0xc + off);
        g.write::<u32>(e + 0x24c + off, p);
    }
    let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0xe2));
    if obj != 0 {
        let start = img.rel32_target(f + 0xfe);
        crate::s4_misc::start_object_at(g, img, start, obj, k(g, 0xf0), 0, 0, 0);
        for off in [0u32, 4, 8, 0xc] {
            let p = g.read::<u32>(e + 0x24c + off);
            g.write::<u32>(obj + 0xd0 + off, p);
        }
    }
    g.write::<u32>(e + 0x26c, 0);
    let puff = Frame {
        at: img.rel32_target(f + 0x145),
        ecx: e,
        ..*fr
    };
    Ret::eax(crate::s6_misc::smoke_puff(g, img, &puff))
}

/// [`landing_take_0284`] (the `rel32`s: the pool take, the object start,
/// the smoke puff).
pub const LANDING_TAKE_0284: Sig = Sig {
    len: 332,
    hash: 0xd6d9_331c_f476_9ed2,
    calls: &[
        (0xe2, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0xfe, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x145, Callee::Sig(&crate::s6_misc::SMOKE_PUFF)),
    ],
    imports: &[],
};
