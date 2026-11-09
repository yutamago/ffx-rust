//! Round 32's ports (magic-r32): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Where a copy's operands sit: `(start, delta)` runs over the model's
/// offsets (the last run starting at or below an offset applies), and
/// single offsets moved out of order (`(model, copy)`).
struct Shift {
    runs: &'static [(u32, i32)],
    moved: &'static [(u32, u32)],
}

impl Shift {
    /// The copy's offset of the model's operand `o`.
    fn at(&self, o: u32) -> u32 {
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
fn append_last<G: Game>(
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
