//! Round 44's ports (own rows of the DLLs below `magic_0250`): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
use crate::r31::child_of;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// The operands of the summon set-ups' model loop ([`model_nodes`]): the
/// offsets in the function of each instruction's operand, and the frame
/// and block places that differ between the copies.
struct ModelOps {
    /// the first model `S + base`, the next `stride` further on
    base: u32,
    stride: u32,
    /// the bone node `S + bone`
    bone: u32,
    /// the matrix slot 327 filled, `ebp - m`; slot 601's height, `ebp - h`
    m: u32,
    h: u32,
    /// vtables of the nodes `A`, `B`, `C`, `D`
    vt: [u32; 4],
    /// slots 262, 229, 290, 277, 333, 601, 211 (their `g_MagicImports` loads)
    slots: [u32; 7],
    /// the constants: 277's scale, 333's and 601's minuends, the tilt's
    /// factor and divisor
    k: [u32; 5],
}

/// The summon set-ups' last loop (`magic_0155`, `magic_0164`; the r31
/// set-ups of 0609 and 0613 have it inline): for each model `k`
/// (`O = S + base + stride k`): node `A = O + 0x210` (slot 262 of
/// `O[0x54]` into `A + 0x1c..`, w 1.0, slot 229 through `M` into
/// `S + 0x2c + 0x10 k`), node `B = A + 0x2c` (slot 290
/// `(O[0x50], B + 0x1c)`, `B + 0x28` = slot 277 `(O[0x54])` K), node
/// `C = O + 0x1e4` (`+0x24`, `+0x1c` 0, the tilt `+0x20 = (K - h) K' / K''`
/// truncated, `h` slot 333 `(O[0x54])` with the flag `O + 0x90` bit 0,
/// else slot 601 `(O[0x50], &h)`), node `D = O + 0x98`; `O + 0x1d4`,
/// `+0x1d8`, `+0x1dc` = `A`, `B`, `C`, `O + 0x1c0` = the bone, byte
/// `O + 0x1e0` 0; slot 211 `(O[0x54], O + 0x134)`, flag bit 4 of
/// `O + 0x90`, `O + 0x4c = D`. Returns `S + count` (the count's
/// address, what the original leaves in `eax`) when a model ran, else
/// `eax`.
#[allow(clippy::too_many_arguments)] // the loop's inputs, as the original has them
fn model_nodes<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ebp: u32,
    grp: u32,
    s: u32,
    count_at: u32,
    ops: &ModelOps,
    mut eax: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let [s262, s229, s290, s277, s333, s601, s211] = ops.slots;
    let parent = |g: &G| g.read::<u32>(grp.wrapping_add(4));
    let count = |g: &G| g.read::<u32>(count_at).cast_signed();
    let m = ebp.wrapping_sub(ops.m);
    let bone = s.wrapping_add(ops.bone);
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = s.wrapping_add(ops.base + ops.stride * i);
        let ob = |off: u32| o.wrapping_add(off);
        let (na, nb, nc, nd) = (ob(0x210), ob(0x23c), ob(0x1e4), ob(0x98));
        let p = parent(g);
        child_of(g, na, p);
        let m54 = g.read::<u32>(ob(0x54));
        let pos = na.wrapping_add(0x1c);
        g.write::<u32>(na, w(ops.vt[0]));
        let mut st = Pushes::default();
        st.push_all(&[pos.wrapping_add(8), pos.wrapping_add(4), pos, m54]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, s262, 262), 0);
        let out = s.wrapping_add(0x2c + 0x10 * i);
        g.write::<u32>(na.wrapping_add(0x28), ONE);
        st.push_all(&[pos, m, out]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, s229, 229), 0);
        let p = parent(g);
        child_of(g, nb, p);
        let m50 = g.read::<u32>(ob(0x50));
        let mut st = Pushes::default();
        st.push_all(&[nb.wrapping_add(0x1c), m50]);
        g.write::<u32>(nb, w(ops.vt[1]));
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, s290, 290), 0);
        st.push(g.read::<u32>(ob(0x54)));
        // call width: 12 (the most `Pushes::words` holds)
        let v = st0_f32(st.call(g, t(g, s277, 277), 0).st0);
        g.write::<f32>(nb.wrapping_add(0x28), mul(v, k(g, ops.k[0])));
        g.write::<u32>(nc, w(ops.vt[2]));
        let p = parent(g);
        child_of(g, nc, p);
        g.write::<u32>(nc.wrapping_add(0x24), 0);
        g.write::<u32>(nc.wrapping_add(0x1c), 0);
        let h = if g.read::<u8>(ob(0x90)) & 1 != 0 {
            let m54 = g.read::<u32>(ob(0x54));
            // call width: 1
            let h = st0_f32(g.call(t(g, s333, 333), 0, &[m54]).st0);
            sub(k(g, ops.k[1]), h)
        } else {
            let m50 = g.read::<u32>(ob(0x50));
            let hp = ebp.wrapping_sub(ops.h);
            // call width: 2
            g.call(t(g, s601, 601), 0, &[m50, hp]);
            sub(k(g, ops.k[2]), g.read::<f32>(hp))
        };
        let tilt = div(mul(h, k(g, ops.k[3])), k(g, ops.k[4]));
        g.write::<u32>(nc.wrapping_add(0x20), cvtt(tilt).cast_unsigned());
        g.write::<u32>(nd, w(ops.vt[3]));
        let p = parent(g);
        child_of(g, nd, p);
        g.write::<u32>(ob(0x1dc), nc);
        g.write::<u32>(ob(0x1d8), nb);
        g.write::<u32>(ob(0x1c0), bone);
        g.write::<u32>(ob(0x1d4), na);
        g.write::<u8>(ob(0x1e0), 0);
        let m54 = g.read::<u32>(ob(0x54));
        // call width: 2
        g.call(t(g, s211, 211), 0, &[m54, ob(0x134)]);
        let fl = g.read::<u16>(ob(0x90));
        g.write::<u16>(ob(0x90), fl | 0x10);
        g.write::<u32>(ob(0x4c), nd);
        eax = count_at;
        i += 1;
    }
    eax
}

/// The bone node of a summon set-up: `S + bone` gets its vtable (operand
/// `vt`), is made a child of `P`, holds the three nodes (`+0x13c` the
/// mean place, `+0x140` the scale, `+0x144` the turn), the DLL's object
/// (operand `obj`) at `+0x128` and byte `+0x148` 0, and is set up
/// ([`bone_plain`](crate::sa_summon::bone_plain)) through the holder at
/// `ebp - holder` (mark byte 0), `esp` at the call `ebp - esp`.
#[allow(clippy::too_many_arguments)] // the node's operands
fn bone_node<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    ebp: u32,
    p: u32,
    bone: u32,
    nodes: [u32; 3],
    ops: [u32; 5],
) {
    let [vt, obj, call, holder, esp] = ops;
    let f = fr.at;
    g.write::<u32>(bone, img.word(f + vt));
    child_of(g, bone, p);
    let [mean, scale, turn] = nodes;
    g.write::<u32>(bone + 0x144, turn);
    g.write::<u32>(bone + 0x13c, mean);
    g.write::<u32>(bone + 0x140, scale);
    g.write::<u32>(bone + 0x128, img.word(f + obj));
    g.write::<u8>(bone + 0x148, 0);
    let holder = ebp.wrapping_sub(holder);
    g.write::<u32>(holder, bone);
    g.write::<u8>(holder + 5, 0);
    let bf = Frame {
        at: img.rel32_target(f + call),
        esp: ebp.wrapping_sub(esp),
        ecx: holder,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bf);
}

/// `magic_0155`'s summon set-up (0x100075b0, 1715 bytes; internal, `ecx`
/// = the group `G` (`[S, P, a, b, flags byte]`); a /GS frame): as
/// [`summon_setup_0609`](crate::r31::summon_setup_0609) (the block's
/// words, models 0x278 apart, slot 262 twice and slot 12 per model node,
/// the mean place of slot 366 without the flagged models) with the bone
/// set up once and the models' last node `D` not set up as a bone
/// ([`model_nodes`]). Returns `S + 0x163c`, or slot 327's `eax` with no
/// models.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let grp = fr.ecx;
    let parent = |g: &G| g.read::<u32>(grp.wrapping_add(4));
    let s = g.read::<u32>(grp);
    let p = parent(g);
    child_of(g, s, p);
    let s = g.read::<u32>(grp);
    let at = |o: u32| s.wrapping_add(o);
    let x = g.read::<u32>(grp.wrapping_add(0xc));
    g.write::<u32>(at(0x1648), x);
    let x = g.read::<u32>(grp.wrapping_add(8));
    g.write::<u32>(at(0x164c), x);
    let fl = g.read::<u8>(grp.wrapping_add(0x10)) | 2;
    g.write::<u8>(at(0x1650), fl);
    let a = g.read::<u32>(at(0x164c));
    // call width: 2
    let objs = g.call(t(g, 0x84, 218), 0, &[a, at(0x163c)]).eax;
    g.write::<u32>(at(0x1644), objs);
    let count = |g: &G| g.read::<u32>(at(0x163c)).cast_signed();
    let model = |i: u32| at(0xac + 0x278 * i);
    // the model nodes
    let args = ebp.wrapping_sub(0x8c);
    let init = img.rel32_target(f + 0xf0);
    let (v1, v2) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let node = model(i);
        let p = parent(g);
        let byte = g.read::<u8>(g.read::<u32>(at(0x1644)).wrapping_add(i));
        g.write::<u32>(args, node);
        g.write::<u32>(args + 4, p);
        g.write::<u32>(args + 8, u32::from(byte));
        g.write::<u8>(args + 0xc, 0);
        g.write::<u32>(node, w(0xeb));
        crate::sa_summon::model_node_init(g, img, init, args);
        for (v, op) in [(v1, 0x101u32), (v2, 0x11d)] {
            let m = g.read::<u32>(node + 0x54);
            // call width: 4
            g.call(t(g, op, 262), 0, &[m, v, v + 4, v + 8]);
        }
        let m = g.read::<u32>(node + 0x54);
        // call width: 1
        let h = st0_f32(g.call(t(g, 0x12d, 12), 0, &[m]).st0);
        g.write::<f32>(v2 + 4, h);
        let d = sub(rf(g, v1 + 4), h);
        g.write::<u16>(node + 0x26e, 0);
        g.write::<f32>(node + 0x274, d);
        i += 1;
    }
    // the mean place
    let mean = at(0x15b8);
    let p = parent(g);
    child_of(g, mean, p);
    g.write::<u32>(mean, w(0x1b5));
    for o in [0u32, 4, 8] {
        g.write::<u32>(v2 + o, 0);
    }
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = model(i);
        if g.read::<u8>(o + 0x90) & 1 == 0 {
            let m = g.read::<u32>(o + 0x50);
            // call width: 2
            g.call(t(g, 0x1eb, 366), 0, &[m, v1]);
        }
        for j in 0..3u32 {
            let x = add(rf(g, v2 + 4 * j), rf(g, v1 + 4 * j));
            g.write::<f32>(v2 + 4 * j, x);
        }
        i += 1;
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let q = div(k(g, 0x24b), count(g) as f32);
    for j in 0..3u32 {
        g.write::<f32>(at(0x15d4 + 4 * j), mul(rf(g, v2 + 4 * j), q));
    }
    g.write::<u32>(at(0x15e0), ONE);
    let scale = at(0x15e4);
    let p = parent(g);
    child_of(g, scale, p);
    g.write::<u32>(scale, w(0x2cb));
    for o in [0x1600u32, 0x1604, 0x1608, 0x160c] {
        g.write::<u32>(at(o), ONE);
    }
    let turn = at(0x1610);
    let p = parent(g);
    g.write::<u32>(turn, w(0x305));
    child_of(g, turn, p);
    for o in [0x162cu32, 0x1630, 0x1634] {
        g.write::<u32>(at(o), 0);
    }
    let p = parent(g);
    bone_node(
        g,
        img,
        fr,
        ebp,
        p,
        at(0x146c),
        [mean, scale, turn],
        [0x37c, 0x3d9, 0x3ef, 0x84, 0xa4],
    );
    // call width: 2
    let eax = g
        .call(t(g, 0x3ff, 327), 0, &[ebp.wrapping_sub(0x68), at(0x1508)])
        .eax;
    let ops = ModelOps {
        base: 0xac,
        stride: 0x278,
        bone: 0x146c,
        m: 0x68,
        h: 0x80,
        vt: [0x47c, 0x4f4, 0x539, 0x600],
        slots: [0x473, 0x495, 0x4ee, 0x507, 0x587, 0x5b3, 0x65c],
        k: [0x525, 0x5a0, 0x5c9, 0x5d9, 0x5e1],
    };
    Ret::eax(model_nodes(g, img, f, ebp, grp, s, at(0x163c), &ops, eax))
}

/// [`summon_setup_0155`] (the `rel32`s: the model node set-up, the bone,
/// `__security_check_cookie`).
pub const SUMMON_SETUP_0155: Sig = Sig {
    len: 1715,
    hash: 0x7855_cfd9_0f6e_cc74,
    calls: &[
        (0xf0, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x3ef, Callee::Sig(&crate::sa_summon::BONE)),
        (0x6ab, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0164`'s summon set-up (0x100075f0, 1780 bytes; internal, `ecx`
/// = the group `G` (`[S, P, a, b, flags byte]`); a /GS frame): as
/// [`summon_setup_0613`](crate::r31::summon_setup_0613) (models 0x274
/// apart from `S + 0xbc`) but the mean place: for each model `O`, slot 390
/// `(O[0x50])`; when it answers 0x1084, slot 268 `(O[0x54], 0x11, S +
/// 0x15c4)` gives the place itself (`S + 0x15d0` = 1.0, slot 325 `(S +
/// 0xac, S + 0x15c4)`, flag bit 3 of `S + 0x1640`) and the loop ends;
/// otherwise slot 268 `(O[0x54], 0x11, &v)` is added up and the sums times
/// `K / n` go to `S + 0x15c4..` (w 1.0). Then the scale, turn and bone
/// nodes and [`model_nodes`]. Returns `S + 0x162c`, or slot 327's `eax`
/// with no models.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let grp = fr.ecx;
    let parent = |g: &G| g.read::<u32>(grp.wrapping_add(4));
    let s = g.read::<u32>(grp);
    let p = parent(g);
    child_of(g, s, p);
    let s = g.read::<u32>(grp);
    let at = |o: u32| s.wrapping_add(o);
    let x = g.read::<u32>(grp.wrapping_add(0xc));
    g.write::<u32>(at(0x1638), x);
    let x = g.read::<u32>(grp.wrapping_add(8));
    g.write::<u32>(at(0x163c), x);
    let fl = g.read::<u8>(grp.wrapping_add(0x10)) | 2;
    g.write::<u8>(at(0x1640), fl);
    let a = g.read::<u32>(at(0x163c));
    // call width: 2
    let objs = g.call(t(g, 0x82, 218), 0, &[a, at(0x162c)]).eax;
    g.write::<u32>(at(0x1634), objs);
    let count = |g: &G| g.read::<u32>(at(0x162c)).cast_signed();
    let model = |i: u32| at(0xbc + 0x274 * i);
    // the model nodes
    let args = ebp.wrapping_sub(0x7c);
    let init = img.rel32_target(f + 0xd7);
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let node = model(i);
        let p = parent(g);
        let byte = g.read::<u8>(g.read::<u32>(at(0x1634)).wrapping_add(i));
        g.write::<u32>(args, node);
        g.write::<u32>(args + 4, p);
        g.write::<u32>(args + 8, u32::from(byte));
        g.write::<u8>(args + 0xc, 0);
        g.write::<u32>(node, w(0xd2));
        crate::sa_summon::model_node_init(g, img, init, args);
        g.write::<u16>(node + 0x26e, 0);
        i += 1;
    }
    // the mean place
    let mean = at(0x15a8);
    let p = parent(g);
    child_of(g, mean, p);
    g.write::<u32>(mean, w(0x142));
    let (mut sx, mut sy, mut sz) = (0f32, 0f32, 0f32);
    let v = ebp.wrapping_sub(0x14);
    let mut kq = 0x137;
    let mut own = false;
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = model(i);
        let m50 = g.read::<u32>(o + 0x50);
        // call width: 1
        let e = g.call(t(g, 0x173, 390), 0, &[m50]).eax;
        if e == 0x1084 {
            own = true;
            break;
        }
        let m54 = g.read::<u32>(o + 0x54);
        // call width: 3
        g.call(t(g, 0x19a, 268), 0, &[m54, 0x11, v]);
        sx = add(g.read::<f32>(v), sx);
        sy = add(g.read::<f32>(v + 4), sy);
        g.write::<u32>(v + 0xc, ONE);
        sz = add(g.read::<f32>(v + 8), sz);
        kq = 0x20a;
        i += 1;
    }
    let place = at(0x15c4);
    if own {
        let m54 = g.read::<u32>(model(i) + 0x54);
        // call width: 3
        g.call(t(g, 0x296, 268), 0, &[m54, 0x11, place]);
        g.write::<u32>(place + 0xc, ONE);
        g.write::<u32>(at(0x15d0), ONE);
        // call width: 2
        g.call(t(g, 0x2ae, 325), 0, &[at(0xac), place]);
        let fl = g.read::<u8>(at(0x1640));
        g.write::<u8>(at(0x1640), fl | 8);
    } else {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let q = div(k(g, kq), count(g) as f32);
        g.write::<f32>(place, mul(sx, q));
        g.write::<f32>(place + 4, mul(sy, q));
        g.write::<f32>(place + 8, mul(sz, q));
        g.write::<u32>(at(0x15d0), ONE);
    }
    let scale = at(0x15d4);
    let p = parent(g);
    child_of(g, scale, p);
    g.write::<u32>(scale, w(0x2e9));
    for o in [0x15f0u32, 0x15f4, 0x15f8, 0x15fc] {
        g.write::<u32>(at(o), ONE);
    }
    let turn = at(0x1600);
    let p = parent(g);
    g.write::<u32>(turn, w(0x323));
    child_of(g, turn, p);
    for o in [0x161cu32, 0x1620, 0x1624] {
        g.write::<u32>(at(o), 0);
    }
    let p = parent(g);
    bone_node(
        g,
        img,
        fr,
        ebp,
        p,
        at(0x145c),
        [mean, scale, turn],
        [0x39b, 0x425, 0x435, 0x74, 0x94],
    );
    // call width: 2
    let eax = g
        .call(t(g, 0x445, 327), 0, &[ebp.wrapping_sub(0x58), at(0x14f8)])
        .eax;
    let ops = ModelOps {
        base: 0xbc,
        stride: 0x274,
        bone: 0x145c,
        m: 0x58,
        h: 0x6c,
        vt: [0x4bd, 0x535, 0x57a, 0x641],
        slots: [0x4b4, 0x4d6, 0x52f, 0x548, 0x5c8, 0x5f4, 0x69a],
        k: [0x566, 0x5e1, 0x60a, 0x61a, 0x622],
    };
    Ret::eax(model_nodes(g, img, f, ebp, grp, s, at(0x162c), &ops, eax))
}

/// [`summon_setup_0164`] (the `rel32`s: the model node set-up, the bone,
/// `__security_check_cookie`).
pub const SUMMON_SETUP_0164: Sig = Sig {
    len: 1780,
    hash: 0x7385_92db_54fd_1dc9,
    calls: &[
        (0xd7, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x435, Callee::Sig(&crate::sa_summon::BONE)),
        (0x6ec, Callee::Any),
    ],
    imports: &[],
};

/// The summon start of `magic_0155` (0x10008eb0) and `magic_0164`
/// (0x10008e40), 61 bytes, internal (cdecl, no arguments; a frame): the
/// group `G = ebp - 0x14` = `[S, P, *A, 0, byte 0]` (the operands at
/// `+0x11`, `+0x18`, `+0x7`), `S`'s vtable stored (the word at `+0x2c`
/// gets the operand at `+0x30`) and the DLL's set-up (`setup`, the call
/// at `+0x34`) run on it. Returns the set-up's `eax`.
fn summon_start_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    setup: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let a = g.read::<u32>(w(0x7));
    let grp = ebp.wrapping_sub(0x14);
    g.write::<u32>(grp, w(0x11));
    g.write::<u32>(grp + 4, w(0x18));
    g.write::<u32>(grp + 8, a);
    g.write::<u32>(grp + 0xc, 0);
    g.write::<u8>(grp + 0x10, 0);
    g.write::<u32>(w(0x2c), w(0x30));
    let sf = Frame {
        at: img.rel32_target(f + 0x35),
        esp: ebp.wrapping_sub(0x18),
        ecx: grp,
        ..*fr
    };
    Ret::eax(setup(g, img, &sf).eax)
}

/// `magic_0155`'s summon start (0x10008eb0): [`summon_start_with`]
/// [`summon_setup_0155`].
pub fn summon_start_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_start_with(g, img, fr, summon_setup_0155)
}

/// `magic_0164`'s summon start (0x10008e40): [`summon_start_with`]
/// [`summon_setup_0164`].
pub fn summon_start_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_start_with(g, img, fr, summon_setup_0164)
}

/// [`summon_start_0155`] (the `rel32`: the set-up).
pub const SUMMON_START_0155: Sig = Sig {
    len: 61,
    hash: 0xf970_deea_f7a3_d994,
    calls: &[(0x35, Callee::Sig(&SUMMON_SETUP_0155))],
    imports: &[],
};

/// [`summon_start_0164`] (the `rel32`: the set-up).
pub const SUMMON_START_0164: Sig = Sig {
    len: 61,
    hash: 0xf970_deea_f7a3_d994,
    calls: &[(0x35, Callee::Sig(&SUMMON_SETUP_0164))],
    imports: &[],
};

/// `magic_0155`'s throw task (0x10009570, 900 bytes; a task callback,
/// cdecl `(task)`, the record `R = task[1]`, `S = R + 0x1c`'s word; an
/// 8-aligned frame, `B` = `esp` after the three pushes): the steps of
/// [`throw_task_0609`](crate::r29::throw_task_0609) with the node appends
/// called ([`node_append`](crate::sa_misc::node_append)) and the particle
/// and effect nodes through their wrappers
/// ([`particle_node_with`](crate::s3_nodes::particle_node_with),
/// [`effect_node_with`](crate::s3_nodes::effect_node_with)). By the state
/// `R + 0x18` (a jump table for 10..21):
/// - 10: flag bit 1 of `S + 0x1650`, the state 11 and `R + 0x30` 0; then
///   as 11;
/// - 11: `S + 0x15d8` less `K`; the state steps on at `R + 0x30 >= 40`;
/// - 12: `S + 0x15d8` less `K'`, `S + 0x162c` and `+0x1630` less 0x200;
///   the state steps on at `R + 0x30 >= 130`;
/// - 13: `S + 0x162c` and `+0x1630` less 0x200;
/// - 20: the particles thrown again
///   ([`particles_rethrow`](crate::s5_misc::particles_rethrow) `(S, 0,
///   0.5)`), the state 21 and the target mask `R + 0x34` 7; then as 21;
/// - 21: for each of the `S + 0x163c` targets `i` whose mask bit is set
///   and whose `S + 0x100 + 0x278 i + 0x218` (u16) is not 0: the static
///   node `N_i` (0x2c bytes from `D`) under `P` at the target's point
///   (slot 262 of its handle; `y` slot 12's less `K`, `w` 1.0), the sprite
///   node `N_i + 0x160` under `P` (scales 1.0, slot 251 times `K2`), the
///   owned node `A_i` (0x14c bytes from `G_A`), the particle node `Q_i`
///   (0x100 bytes from `G_Q`, index 1), the effect node `R + 0x38 + 0x38
///   i` (its flag set, `+4` 0), slot 719's handle to slot 306 `(7, 2, h)`,
///   the bit cleared; with the mask empty the state steps on;
/// - 30: flag bit 1 of `S + 0x1650`, the state 31 and `R + 0x30` 0;
/// - 31: the state steps on at `R + 0x30 >= 20`;
/// - 10000: the task node unlinked.
///
/// Every way ends with `R + 0x2c` and `R + 0x30` up. Returns the
/// last `eax`: the state (less 10 below 30), the rethrow's, the
/// last target's mask word (shifted or with its bit's complement), the
/// node's old `+4`.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn throw_task_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let st = g.read::<u32>(r + 0x18);
    let s = g.read::<u32>(r + 0x1c);
    // push ebp; and esp, -8; sub esp, 0x2c; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let bump = |g: &mut G, o: u32| {
        let v = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, v);
    };
    let end = |g: &mut G, eax: u32| {
        bump(g, 0x2c);
        bump(g, 0x30);
        Ret::eax(eax)
    };
    // the step at a count: the state on when `R + 0x30 >= n`
    let step_at = |g: &mut G, n: u32, eax: u32| {
        if g.read::<i32>(r + 0x30) >= n.cast_signed() {
            bump(g, 0x18);
        }
        end(g, eax)
    };
    let less = |g: &mut G, o: u32, op: u32| {
        let x = sub(g.read::<f32>(s + o), k(g, op));
        g.write::<f32>(s + o, x);
    };
    let flag_on = |g: &mut G| {
        let b = g.read::<u8>(s + 0x1650) | 2;
        g.write::<u8>(s + 0x1650, b);
        bump(g, 0x18);
        g.write::<u32>(r + 0x30, 0);
    };
    let back = |g: &mut G| {
        for o in [0x162cu32, 0x1630] {
            let v = g.read::<u32>(s + o).wrapping_sub(0x200);
            g.write::<u32>(s + o, v);
        }
    };
    let mut eax = if st >= 30 { st } else { st.wrapping_sub(10) };
    match st {
        10 | 11 => {
            if st == 10 {
                flag_on(g);
            }
            less(g, 0x15d8, 0x5b);
            return step_at(g, 0x28, eax);
        }
        12 => {
            less(g, 0x15d8, 0x7c);
            back(g);
            return step_at(g, 0x82, eax);
        }
        13 => {
            back(g);
            return end(g, eax);
        }
        30 | 31 => {
            if st == 30 {
                flag_on(g);
            }
            return step_at(g, 0x14, eax);
        }
        10000 => {
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            return end(g, next);
        }
        20 | 21 => {}
        _ => return end(g, eax),
    }
    if st == 20 {
        let rec = b0 + 0x2c;
        for (i, v) in (0u32..).zip([s, 0, 0x3f00_0000]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        let rt = Frame {
            at: call(0xe2),
            esp: b0.wrapping_sub(4),
            ecx: rec,
            ..*fr
        };
        eax = crate::s5_misc::particles_rethrow(g, img, &rt).eax;
        bump(g, 0x18);
        g.write::<u32>(r + 0x34, 7);
    }
    let mut i = 0u32;
    let (mut qa, mut qb, mut qc, mut d, mut e) =
        (w(0x11c), w(0x114), r + 0x64, w(0x106), s + 0x100);
    while i.cast_signed() < g.read::<i32>(s + 0x163c) {
        let mask = g.read::<u32>(r + 0x34);
        eax = (mask.cast_signed() >> (i & 31)).cast_unsigned();
        if eax & 1 != 0 && g.read::<u16>(e + 0x218) != 0 {
            // the static node at the target's point
            let n = d - 0x20;
            eax = crate::sb_misc::node_append(g, img, call(0x15a), n, w(0x150), eax);
            let h = g.read::<u32>(e);
            // call width: 4
            g.call(t(g, 0x16c, 262), 0, &[h, d - 4, d, d + 4]);
            // call width: 1
            let y = st0_f32(g.call(t(g, 0x17b, 12), 0, &[h]).st0);
            g.write::<f32>(d, sub(y, k(g, 0x198)));
            g.write::<u32>(d + 8, ONE);
            // the sprite node
            let c = d + 0x140;
            crate::sb_misc::node_append(g, img, call(0x1b8), c, w(0x1a6), eax);
            for o in [0x15cu32, 0x160, 0x164] {
                g.write::<u32>(d + o, ONE);
            }
            let x = g.read::<u32>(e.wrapping_sub(4));
            // call width: 1
            let rr = st0_f32(g.call(t(g, 0x1e6, 251), 0, &[x]).st0);
            g.write::<f32>(d + 0x168, mul(rr, k(g, 0x200)));
            // the model node, owned
            let call_esp = b0.wrapping_sub(0x14);
            for (j, v) in (1u32..).zip([w(0x21c), n, w(0x216), c]) {
                g.write::<u32>(call_esp + 4 * j, v);
            }
            let na = Frame {
                at: call(0x22b),
                esp: call_esp,
                ecx: qa,
                edx: w(0x20c),
                ..*fr
            };
            crate::sa_misc::node_append_owned(g, img, &na);
            // its particle node (`push 1; sub esp, 8; push A_i`)
            g.write::<u32>(call_esp + 4, qa);
            g.write::<u32>(call_esp + 0x10, 1);
            let pn = Frame {
                at: call(0x23d),
                esp: call_esp,
                ecx: qb,
                ..*fr
            };
            crate::s3_nodes::particle_node_with(g, img, &pn);
            // the effect node (`push 4; push ecx; push word; push A_i`: the
            // third word, what the particle node leaves in `ecx`, is not
            // read)
            g.write::<u32>(call_esp + 4, qa);
            g.write::<u32>(call_esp + 8, w(0x248));
            g.write::<u32>(call_esp + 0x10, 4);
            let en = Frame {
                at: call(0x255),
                esp: call_esp,
                ecx: qc - 0x2c,
                ..*fr
            };
            crate::s3_nodes::effect_node_with(g, img, &en);
            let fl = g.read::<u32>(qc) | 1;
            g.write::<u32>(qc, fl);
            g.write::<u32>(qc + 4, 0);
            // call width: 3
            let hh = g.call(t(g, 0x25a, 719), 0, &[w(0x273), 5, 0]).eax;
            // call width: 3
            g.call(t(g, 0x27e, 306), 0, &[7, 2, hh]);
            eax = !(1u32 << (i & 31));
            g.write::<u32>(r + 0x34, g.read::<u32>(r + 0x34) & eax);
        }
        qa += 0x14c;
        qb += 0x100;
        qc += 0x38;
        i += 1;
        e += 0x278;
        d += 0x2c;
    }
    if g.read::<u32>(r + 0x34) == 0 {
        bump(g, 0x18);
    }
    end(g, eax)
}

/// [`throw_task_0155`] (the `rel32`s: the rethrow, the node appends, the
/// owned append, the particle and effect nodes).
pub const THROW_TASK_0155: Sig = Sig {
    len: 900,
    hash: 0x5dc1_e306_4d9b_b526,
    calls: &[
        (0xe2, Callee::Sig(&crate::s5_misc::PARTICLES_RETHROW)),
        (0x15a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1b8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x22b, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x23d, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x255, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
    ],
    imports: &[],
};

/// One of [`frame_0184`]'s four part blocks, its operands `d` bytes after
/// the first block's (`+0x15a`): the scale `K` (the immediates of the four
/// stores and the `[esp]` word) into four globals, the matrix `M` (the
/// word at `+0x178`) rows 0..2 scaled from three vectors (slot 233 `(M +
/// 0x10 r, v_r, s_r)`, `s_0` the immediate, `s_1`, `s_2` two of the
/// globals read back), row 3 four globals, slot 231 `(A, B, M)`, slot 230
/// `(C, D)`, the screen word's store, then the part (slot 712 `(part,
/// 0x30)` with the lock word set to 1 and put back); when it ran, the
/// block's u16 flag 0.
fn part_block_0184<G: Game>(g: &mut G, img: &Image, f: u32, d: u32) {
    let w = |o: u32| img.word(f + d + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    for o in [0x166u32, 0x182, 0x18c, 0x196] {
        g.write::<u32>(w(o - 4), w(o));
    }
    let k0 = w(0x16d);
    let m = gw(g, 0x178);
    // call width: 3
    g.call(t(g, 0x15b, 233), 0, &[m, w(0x172), k0]);
    let s1 = gw(g, 0x1a6);
    let m = gw(g, 0x1ab);
    // call width: 3
    g.call(t(g, 0x1c1, 233), 0, &[m + 0x10, w(0x1bb), s1]);
    let s2 = gw(g, 0x1d1);
    let m = gw(g, 0x1d6);
    // call width: 3
    g.call(t(g, 0x1ec, 233), 0, &[m + 0x20, w(0x1e6), s2]);
    let m = gw(g, 0x1f9);
    for (i, o) in (0u32..).zip([0x201u32, 0x20e, 0x21b, 0x228]) {
        let x = gw(g, o);
        g.write::<u32>(m + 0x30 + 4 * i, x);
    }
    let (a, b) = (gw(g, 0x23f), gw(g, 0x22f));
    // call width: 3
    g.call(t(g, 0x239, 231), 0, &[a, b, m]);
    let (c, e) = (gw(g, 0x258), gw(g, 0x24d));
    // call width: 2
    g.call(t(g, 0x252, 230), 0, &[c, e]);
    g.write::<u32>(gw(g, 0x265), w(0x26d));
    let lock = gw(g, 0x272);
    let old = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let ran = g.call(t(g, 0x284, 712), 0, &[w(0x277), 0x30]).eax;
    let lock = gw(g, 0x292);
    g.write::<u32>(lock, old);
    if ran != 0 {
        g.write::<u16>(w(0x2a3), 0);
    }
}

/// `magic_0184`'s frame (0x10002300, 1897 bytes; a table entry, cdecl, no
/// arguments, no frame). Nothing while the word at `+0x2` is 0. With the
/// frame count `F` (the word at `+0xe`):
/// - at `F` = 0x83 and 0xd3, two sounds each (slot 220 `(id, 0x40,
///   0x100)`);
/// - from `F` = 0x83 (or 0xdd, 0xf1): the three handles of slot 941
///   (globals), slots 723 and 724 (the screen and camera matrices), the
///   screen word; then the part blocks ([`part_block_0184`]): the first
///   from 0x83, the second from 0xd8, the third from 0xdd, the fourth from
///   0xf1, each only while its u16 flag is set; from 0xf1 also slot 230
///   and the sprites ([`sprites_0184`](crate::r40::sprites_0184));
/// - from 0x11e, when slot 240 `(motion)` answers, slots 213 `(m, 0)`,
///   `(motion, 4)` and 214 end the motions.
///
/// `F` counts up. Returns 0.
pub fn frame_0184<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    if gw(g, 0x2) == 0 {
        return Ret::eax(0);
    }
    let sound = |g: &mut G, slot: u32, id: u32| {
        let id = gw(g, id);
        // call width: 3
        g.call(t(g, slot, 220), 0, &[id, 0x40, 0x100]);
    };
    // `eax` keeps the frame count; each sound branch reads it again
    let mut fc = gw(g, 0xe);
    if fc == 0x83 {
        sound(g, 0x1a, 0x2d);
        sound(g, 0x34, 0x47);
        fc = gw(g, 0x4e);
    }
    if fc == 0xd3 {
        sound(g, 0x5d, 0x70);
        sound(g, 0x77, 0x8a);
        fc = gw(g, 0x91);
    }
    let mut fc = fc.cast_signed();
    let (a, b, c) = (
        fc.wrapping_sub(0x83),
        fc.wrapping_sub(0xdd),
        fc.wrapping_sub(0xf1),
    );
    if a >= 0 || b >= 0 || c >= 0 {
        for (slot, arg, out) in [
            (0xbeu32, 0xc4u32, 0xd7u32),
            (0xdc, 0xd2, 0xef),
            (0xf4, 0xea, 0x107),
        ] {
            let x = gw(g, arg);
            // call width: 1
            let h = g.call(t(g, slot, 941), 0, &[x]).eax;
            g.write::<u32>(w(out), h);
        }
        let x = gw(g, 0x102);
        g.write::<u32>(gw(g, 0x10c), w(0x112));
        // call width: 1
        g.call(t(g, 0x117, 723), 0, &[x]);
        let x = gw(g, 0x12a);
        // call width: 1
        g.call(t(g, 0x124, 724), 0, &[x]);
        g.write::<u32>(gw(g, 0x137), w(0x140));
        if a >= 0 && g.read::<u16>(w(0x14f)) != 0 {
            part_block_0184(g, img, f, 0);
        }
        if b >= -5 && g.read::<u16>(w(0x2b3)) != 0 {
            part_block_0184(g, img, f, 0x164);
        }
        if b >= 0 && g.read::<u16>(w(0x416)) != 0 {
            part_block_0184(g, img, f, 0x2c7);
        }
        if c >= 0 {
            if g.read::<u16>(w(0x579)) != 0 {
                part_block_0184(g, img, f, 0x42a);
            }
            let (x, y) = (gw(g, 0x6de), gw(g, 0x6d3));
            // call width: 2
            g.call(t(g, 0x6d8, 230), 0, &[x, y]);
            // `esp` at the call: three pushes and the return address
            let sp = img.rel32_target(f + 0x6ee);
            crate::r40::sprites_0184(g, img, sp, fr.esp.wrapping_sub(0x10));
        }
        fc = gw(g, 0x6f3).cast_signed();
    }
    if fc < 0x11e {
        g.write::<u32>(w(0x762), fc.cast_unsigned().wrapping_add(1));
        return Ret::eax(0);
    }
    let m = gw(g, 0x708);
    // call width: 1
    if g.call(t(g, 0x702, 240), 0, &[m]).eax != 0 {
        let x = gw(g, 0x724);
        // call width: 2
        g.call(t(g, 0x71c, 213), 0, &[x, 0]);
        let x = gw(g, 0x739);
        // call width: 2
        g.call(t(g, 0x731, 213), 0, &[x, 4]);
        let x = gw(g, 0x74c);
        // call width: 1
        g.call(t(g, 0x746, 214), 0, &[x]);
    }
    let n = gw(g, 0x75c).wrapping_add(1);
    g.write::<u32>(w(0x762), n);
    Ret::eax(0)
}

/// [`frame_0184`] (the `rel32`: the sprites).
pub const FRAME_0184: Sig = Sig {
    len: 1897,
    hash: 0x87ec_454d_2184_e5cd,
    calls: &[(0x6ee, Callee::Sig(&crate::r40::SPRITES_0184))],
    imports: &[],
};

/// `magic_0184`'s motion step (0x10001990, 2403 bytes; a table entry,
/// cdecl, no arguments; a /GS frame, `ebp` = `E - 4`). Returns 0. Waits
/// for the effect (slot 225 answering 0, then the gate `+0x15` counted
/// up), sets it ready once (slot 226 `(m, 0)` and the word at `+0x5e`),
/// waits for its motion level (slot 227 at least 1, then `+0x6d` counted
/// up); at frame count `F` 0x47 the motion 0xb8 (slot 228), at 0x10e the
/// motion end (slot 213 `(m, 0)`). With `a = F - 0x83`, `b = F - 0xdd`,
/// `c = F - 0xf1`:
/// - `a >= 0`: the centre of two bones (slot 237 into `ebp - 0x44` and
///   `ebp - 0x24`, the sum times a double, `w` 1.0), slot 236, three rows
///   scaled by 100 (slot 233); the first part's rotation (slots 244, 243,
///   244 and 231 on `ebp - 0x84`) and the centre copied, then the
///   second's (slot 244, 231), each while its u16 flag is set;
/// - `b == 0`: slot 238 `(m, 1)`, the start `P0` = the centre, the end
///   `P1 = Q K2 + P0 K1` (`Q` four globals), the direction (slots 234 and
///   232 with its `y` 0, 233 by 20 into `*G_D`), slot 235, the target
///   `P1` less `K` in `y`, slot 228 `(m, 0x22)`;
/// - `0 <= b < 20`: `t = b / K + K'`, the place `P0 (1 - t) + P1 t` with
///   `y = (P1.y - P0.y) t^2 + P0.y` (slot 235 first, the motion 0x49 at `b`
///   = 19), the offset `(0, h K, 0, 1)` turned by slot 243's matrix (the
///   angle the i16 global, slot 231) through slot 229, the model's matrix
///   place `(P + v)` with `y` raised by `(1 - t^2) h` (slot 211);
/// - `c` in `0..=19`: `u = c / K`, `s = |T[(u^2 K'') >> 4 & 0xfff]|` (the
///   table at `*G_T`), slot 230, `x = (1 - u)^2`: the place `P1 x + Q (1 -
///   x)` (w too), the scatter ([`scatter`](crate::m0184::scatter)) at the
///   place plus `G_O`, spread `K`, scale `((1 - u) + D) D'`, then `y`
///   less `x s K'''`; the damage at `c` = 10 (slot 239 `(m, -5, 6)`);
/// - `c` = 20: slot 211 `(m', 0)` and the motion 0x4a (slot 228).
///
/// Every way past the waits adds 0xf000 to the u16 angle.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn motion_0184<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let copy = |g: &mut G, from: u32, to: u32| {
        let x = gw(g, from);
        g.write::<u32>(w(to), x);
    };
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let rf = |g: &G, o: u32| g.read::<f32>(lf(o));
    let cookie = gw(g, 0xa) ^ ebp;
    g.write::<u32>(lf(4), cookie);
    if gw(g, 0x15) == 0 {
        let m = gw(g, 0x23);
        // call width: 1
        if g.call(t(g, 0x1d, 225), 0, &[m]).eax != 0 {
            return Ret::eax(0);
        }
        let n = gw(g, 0x3c).wrapping_add(1);
        g.write::<u32>(w(0x3c), n);
    }
    if gw(g, 0x42) == 0 {
        let m = gw(g, 0x52);
        g.write::<u32>(w(0x5e), 1);
        // call width: 2
        g.call(t(g, 0x4a, 226), 0, &[m, 0]);
    }
    if gw(g, 0x6d) == 0 {
        let m = gw(g, 0x7b);
        // call width: 1
        if g.call(t(g, 0x75, 227), 0, &[m]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        let n = gw(g, 0x95).wrapping_add(1);
        g.write::<u32>(w(0x95), n);
    }
    let mut fc = gw(g, 0x9b);
    if fc == 0x47 || fc == 0x10e {
        let m = gw(g, 0xcd);
        if fc == 0x47 {
            // call width: 2
            g.call(t(g, 0xa5, 228), 0, &[m, 0xb8]);
        } else {
            // call width: 2
            g.call(t(g, 0xbf, 213), 0, &[m, 0]);
        }
        fc = gw(g, 0xd5);
    }
    let fc = fc.cast_signed();
    let (a, b, c) = (
        fc.wrapping_sub(0x83),
        fc.wrapping_sub(0xdd),
        fc.wrapping_sub(0xf1),
    );
    if a < 0 && b < 0 && c < 0 {
        return end_0184(g, img, f);
    }
    let r = lf(0x84);
    if a >= 0 {
        let (v1, v2) = (lf(0x44), lf(0x24));
        let m = gw(g, 0x121);
        // call width: 4
        g.call(t(g, 0x10e, 237), 0, &[m, 0xa8, 0, v1]);
        let m = gw(g, 0x13f);
        // call width: 4
        g.call(t(g, 0x12c, 237), 0, &[m, 0xeb, 0, v2]);
        let d = g.read::<f64>(w(0x153));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let centre = |g: &G, i: u32| {
            let x = add(g.read::<f32>(v1 + 4 * i), g.read::<f32>(v2 + 4 * i));
            (f64::from(x) * d) as f32
        };
        g.write::<u32>(w(0x172), ONE);
        let x = centre(g, 0);
        put(g, 0x182, x);
        let x = centre(g, 1);
        put(g, 0x19f, x);
        let x = centre(g, 2);
        put(g, 0x1bc, x);
        let m = gw(g, 0x168);
        // call width: 3
        g.call(t(g, 0x158, 236), 0, &[m, 0x2d, w(0x15d)]);
        for (slot, row, v) in [
            (0x1c9u32, 0x1e3u32, 0x1deu32),
            (0x1ea, 0x204, 0x1ff),
            (0x20b, 0x225, 0x220),
        ] {
            // call width: 3
            g.call(t(g, slot, 233), 0, &[w(row), w(v), 0x42c8_0000]);
        }
        if g.read::<u16>(w(0x231)) != 0 {
            // call width: 2
            g.call(t(g, 0x249, 244), 0, &[r, 0x4000]);
            // call width: 3
            g.call(t(g, 0x25d, 231), 0, &[w(0x26d), w(0x262), r]);
            // call width: 2
            g.call(t(g, 0x280, 243), 0, &[r, 0xffff_c000]);
            // call width: 3
            g.call(t(g, 0x294, 231), 0, &[w(0x2a4), w(0x299), r]);
            // call width: 2
            g.call(t(g, 0x2b7, 244), 0, &[r, 0x8000]);
            // call width: 3
            g.call(t(g, 0x2cb, 231), 0, &[w(0x2db), w(0x2d0), r]);
            for (from, to) in [
                (0x2e5u32, 0x2edu32),
                (0x2f5, 0x2fd),
                (0x305, 0x30d),
                (0x315, 0x320),
            ] {
                copy(g, from, to);
            }
        }
        if g.read::<u16>(w(0x327)) != 0 {
            // call width: 2
            g.call(t(g, 0x33b, 244), 0, &[r, 0x4000]);
            // call width: 3
            g.call(t(g, 0x34f, 231), 0, &[w(0x35f), w(0x354), r]);
            for (from, to) in [
                (0x369u32, 0x371u32),
                (0x379, 0x381),
                (0x389, 0x391),
                (0x399, 0x3a4),
            ] {
                copy(g, from, to);
            }
        }
    }
    if b == 0 {
        let m = gw(g, 0x3b9);
        // call width: 2
        g.call(t(g, 0x3b1, 238), 0, &[m, 1]);
        let (k2, k1) = (k(g, 0x3d9), k(g, 0x3e1));
        let (p0x, p0y, p0z, p0w) = (k(g, 0x3d1), k(g, 0x3c9), k(g, 0x3e9), k(g, 0x401));
        put(g, 0x3f1, p0y);
        let x = mul(k(g, 0x3f9), k2);
        put(g, 0x40d, p0x);
        put(g, 0x419, p0z);
        let x = add(x, mul(p0x, k1));
        put(g, 0x429, p0w);
        put(g, 0x43a, x);
        let x = add(mul(k(g, 0x442), k2), mul(p0z, k1));
        put(g, 0x460, x);
        let x = add(mul(k(g, 0x468), k2), mul(p0w, k1));
        put(g, 0x478, x);
        copy(g, 0x480, 0x488);
        let v = lf(0x34);
        // call width: 3
        g.call(t(g, 0x458, 234), 0, &[v, w(0x44e), w(0x432)]);
        g.write::<u32>(lf(0x30), 0);
        // call width: 2
        g.call(t(g, 0x49a, 232), 0, &[v, v]);
        // call width: 3
        g.call(t(g, 0x4bc, 233), 0, &[w(0x4c1), v, 0x41a0_0000]);
        // call width: 1
        g.call(t(g, 0x4ce, 235), 0, &[w(0x4d3)]);
        copy(g, 0x4e3, 0x4f0);
        copy(g, 0x4f8, 0x500);
        let y = sub(k(g, 0x508), k(g, 0x510));
        let m = gw(g, 0x518);
        g.write::<u32>(w(0x51e), ONE);
        put(g, 0x52a, y);
        // call width: 2
        g.call(t(g, 0x4e8, 228), 0, &[m, 0x22]);
    }
    if (0..0x14).contains(&b) {
        let tt = add(div(cvt(b.cast_unsigned()), k(g, 0x55c)), k(g, 0x56a));
        // call width: 1
        g.call(t(g, 0x554, 235), 0, &[w(0x56f)]);
        let one_t = sub(k(g, 0x589), tt);
        g.write::<u32>(w(0x5aa), ONE);
        let x = add(mul(k(g, 0x591), one_t), mul(k(g, 0x599), tt));
        let t2 = mul(tt, tt);
        put(g, 0x5ce, x);
        let z = add(mul(k(g, 0x5d6), one_t), mul(k(g, 0x5be), tt));
        put(g, 0x5ee, z);
        let y = add(mul(sub(k(g, 0x5f6), k(g, 0x5fe)), t2), k(g, 0x60a));
        put(g, 0x612, y);
        if b == 0x13 {
            let m = gw(g, 0x624);
            // call width: 2
            g.call(t(g, 0x61c, 228), 0, &[m, 0x49]);
        }
        let h = i32::from(g.read::<i16>(w(0x636))).cast_unsigned();
        let r2 = lf(0xc4);
        // call width: 2
        g.call(t(g, 0x643, 243), 0, &[r2, h]);
        // call width: 3
        g.call(t(g, 0x657, 231), 0, &[w(0x667), w(0x65c), r2]);
        let up = mul(k(g, 0x671), k(g, 0x679));
        let v = lf(0x14);
        g.write::<u32>(lf(0xc), 0);
        g.write::<u32>(v, 0);
        g.write::<f32>(lf(0x10), up);
        g.write::<u32>(lf(8), ONE);
        // call width: 3
        g.call(t(g, 0x688, 229), 0, &[v, w(0x682), v]);
        let x = add(k(g, 0x6b2), rf(g, 0x14));
        let vy = sub(rf(g, 0x10), k(g, 0x6c4));
        let lift = sub(k(g, 0x6cc), t2);
        put(g, 0x6e1, x);
        let y = add(mul(lift, k(g, 0x6f5)), add(k(g, 0x6e9), vy));
        let z = add(k(g, 0x70c), rf(g, 0xc));
        g.write::<f32>(lf(0x10), vy);
        g.write::<u32>(w(0x71c), ONE);
        put(g, 0x728, y);
        put(g, 0x730, z);
        let m = gw(g, 0x700);
        // call width: 2
        g.call(t(g, 0x6d9, 211), 0, &[m, w(0x6fa)]);
    }
    if c.cast_unsigned() <= 0x13 {
        let u = div(cvt(c.cast_unsigned()), k(g, 0x758));
        let table = gw(g, 0x750);
        let i = (cvtt(mul(mul(u, u), k(g, 0x772))) >> 4) & 0xfff;
        let s = g.read::<f32>(table.wrapping_add(4 * i.cast_unsigned()));
        // `comiss 0, s; jbe`: negated when below 0 (the sign mask's word)
        let s = if 0.0 > s {
            f32::from_bits(s.to_bits() ^ gw(g, 0x798))
        } else {
            s
        };
        // call width: 2
        g.call(t(g, 0x7a5, 230), 0, &[w(0x7b5), w(0x7aa)]);
        let k7 = k(g, 0x7bf);
        let v = sub(k7, u);
        let x2 = mul(v, v);
        let rest = sub(k7, x2);
        let px = add(mul(k(g, 0x7cf), rest), mul(k(g, 0x7c7), x2));
        let py = add(mul(k(g, 0x7d7), rest), mul(k(g, 0x823), x2));
        let pz = add(mul(k(g, 0x7ea), rest), mul(k(g, 0x837), x2));
        let pw = add(mul(k(g, 0x7f2), rest), mul(k(g, 0x84b), x2));
        put(g, 0x857, px);
        put(g, 0x85f, pz);
        let base = lf(0x24);
        put(g, 0x877, py);
        g.write::<f32>(base + 4, py);
        put(g, 0x884, pw);
        let spread = k(g, 0x88c);
        g.write::<f32>(base, add(k(g, 0x86b), px));
        g.write::<f32>(base + 8, add(k(g, 0x899), pz));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let scale = ((f64::from(v) + g.read::<f64>(w(0x8b1))) * g.read::<f64>(w(0x8b9))) as f32;
        let mut xmm = fr.xmm;
        xmm[1] = u128::from(spread.to_bits());
        xmm[2] = u128::from(scale.to_bits());
        let sf = Frame {
            at: img.rel32_target(f + 0x8c2),
            esp: lf(0xd8),
            ecx: base,
            xmm,
            ..*fr
        };
        crate::m0184::scatter(g, img, &sf);
        let y = sub(k(g, 0x8da), mul(mul(x2, s), k(g, 0x8e2)));
        put(g, 0x8ee, y);
        if c == 0xa {
            let m = gw(g, 0x908);
            // call width: 3
            g.call(t(g, 0x8f8, 239), 0, &[m, 0xffff_fffb, 6]);
        }
    } else if c == 0x14 {
        let m = gw(g, 0x921);
        // call width: 2
        g.call(t(g, 0x919, 211), 0, &[m, 0]);
        let m = gw(g, 0x936);
        // call width: 2
        g.call(t(g, 0x92e, 228), 0, &[m, 0x4a]);
    }
    end_0184(g, img, f)
}

/// [`motion_0184`]'s end: the u16 angle (the word at `+0x94d`) plus
/// 0xf000. Returns 0.
fn end_0184<G: Game>(g: &mut G, img: &Image, f: u32) -> Ret {
    let at = img.word(f + 0x94d);
    let x = g.read::<u16>(at).wrapping_add(0xf000);
    g.write::<u16>(at, x);
    Ret::eax(0)
}

/// [`motion_0184`] (the `rel32`s: the scatter, `__security_check_cookie`).
pub const MOTION_0184: Sig = Sig {
    len: 2403,
    hash: 0x5b5a_9adb_a123_d5e4,
    calls: &[
        (0x8c2, Callee::Sig(&crate::m0184::SCATTER)),
        (0x95b, Callee::Any),
    ],
    imports: &[],
};

/// A node off the free ring (inlined in the start-ups): with `free`
/// nodes left (the u16 the operand `count` names), the node `n` (the
/// head, given) leaves the ring (its neighbours joined), the head word
/// (operand `head`) becomes `n`'s next (0 for the last), the node is
/// cleared (`+0x8` itself, `+0x10` the step `cb`, byte `+0x18` 0) and the
/// count stored. Returns the node and the new head; with no free node
/// the original goes on with node 0 (it faults), the port too.
fn ring_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    free: &mut u16,
    (head, cb, count): (u32, u32, u32),
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    if *free == 0 {
        return (0, n);
    }
    let prev = g.read::<u32>(n);
    let next = g.read::<u32>(n + 4);
    g.write::<u32>(prev + 4, next);
    let next = g.read::<u32>(n + 4);
    let prev = g.read::<u32>(n);
    g.write::<u32>(next, prev);
    let new_head = if *free > 1 { g.read::<u32>(n + 4) } else { 0 };
    g.write::<u32>(w(head), new_head);
    *free = free.wrapping_sub(1);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, w(cb));
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(w(count), *free);
    (n, new_head)
}

/// The node `n` appended to the task list: `+0x4` 0; with no first node
/// (`first` 0) `n` is the first (`+0` 0, the operand `to_first`'s word),
/// else after `last` (`+0` = `last`, `last + 4` = `n`); `+0x8` itself.
fn task_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    first: u32,
    last: u32,
    to_first: u32,
) {
    g.write::<u32>(n + 4, 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(img.word(f + to_first), n);
    } else {
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    }
    g.write::<u32>(n + 8, n);
}

/// `magic_0072`'s start-up (0x10001460, 2201 bytes; internal, no
/// arguments, run by its load step; an 8-aligned /GS frame, `S` = `esp`
/// after the three pushes, `T = TAB[byte[i]]` the targets):
/// - three globals, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init)); object 0
///   ([`start_object_at`](crate::s4_misc::start_object_at) `(0, 0x1d, -,
///   0)`);
/// - the summon's place (slot 262 of `*G_A` into three globals, `y` from
///   slot 268 `(*G_A, 6, ..)`), a block of globals (a callback, a
///   record), the centre `C` (slot 262 of `*G_A` at `S + 0x28`, `y` from
///   slot 268, `w` 1);
/// - the targets' farthest distance from `C` (slots 262 and 268 per
///   target, `y` from the second; `_libm_sse2_sqrt_precise`), stored, and
///   `n = d / K` (truncated);
/// - per target `i`: object 1 ([`start_object_at`](crate::s4_misc::start_object_at)
///   `(1, rand % 10 + n + 0x49, -, B)`) at the target (slot 262 of its
///   handle into the object's vector, `y` from slot 268, `w` 1), the
///   target's byte at `+0x4c`, the yaw `K - atan2(C.z - z, C.x - x)`
///   (`_CIatan2`) wrapped once into `[K', K'']`, and the target's record
///   (0x34 bytes) cleared and linked;
/// - two blocks of globals and two nodes off the free ring
///   ([`ring_take`]; the second from the head the first left), marked 1
///   and appended to the task list ([`task_append`]), slot 271 `(4)`;
///   the camera words (slot 413 or 215 of `*G_X` by a flag), a third node
///   for the sprite pool ([`pool_setup`](crate::m0072::pool_setup)) and a
///   fourth (marked 2, the frame step 0x10001d30).
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0072<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x84; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x84);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x80, cookie);
    let s = b0.wrapping_sub(0xc);
    let rf = |g: &G, o: u32| g.read::<f32>(s + o);
    for (to, v) in [(0x33u32, 0x37u32), (0x3d, 0x41), (0x47, 0x4b)] {
        g.write::<u32>(w(to), w(v));
    }
    // call width: 3
    let c = g.call(t(g, 0x1b, 719), 0, &[w(0x2d), 8, 0]).eax;
    g.write::<u32>(w(0x55), c);
    crate::l5_objhost::part_init(g, img, call(0x5a));
    let start =
        |g: &mut G, (at, obj, scale): (u32, u32, f32), (index, owner, b): (u32, u32, u32)| {
            crate::s4_misc::start_object_at(g, img, call(at), obj, scale, index, owner, b);
        };
    let scale = k(g, 0x62);
    start(g, (0x76, w(0x6a), scale), (0, 0x1d, 0));
    // the summon's place
    let a = gw(g, 0x81);
    // call width: 4
    g.call(t(g, 0x7b, 262), 0, &[a, w(0x96), w(0x91), w(0x8c)]);
    // call width: 3
    g.call(t(g, 0xa3, 268), 0, &[a, 6, s + 0x48]);
    let y = rf(g, 0x4c);
    let a = gw(g, 0xba);
    g.write::<u16>(w(0xc2), 0);
    g.write::<f32>(w(0xe1), y);
    g.write::<u32>(w(0xe7), ONE);
    for (to, v) in [
        (0xf1u32, w(0xf5)),
        (0xfb, 0),
        (0x105, 0),
        (0x10f, 0),
        (0x119, 0),
        (0x123, w(0x127)),
        (0x12d, w(0x131)),
        (0x137, 0),
        (0x141, 0),
        (0x14b, w(0x14f)),
    ] {
        g.write::<u32>(w(to), v);
    }
    // the centre C at S + 0x28
    let cv = s + 0x28;
    // call width: 4
    g.call(t(g, 0xd9, 262), 0, &[a, cv, cv + 4, cv + 8]);
    // call width: 3
    g.call(t(g, 0x162, 268), 0, &[a, 6, s + 0x6c]);
    let y = rf(g, 0x70);
    let count = gw(g, 0x17c);
    g.write::<f32>(cv + 4, y);
    g.write::<u32>(cv + 0xc, ONE);
    g.write::<f32>(s + 0x10, 0.0);
    g.write::<u32>(s + 0x14, count);
    // a target's handle (`i` = -1 never comes)
    let target = |g: &G, i: u32, (bytes, tab): (u32, u32)| {
        let b = g.read::<u8>(gw(g, bytes).wrapping_add(i));
        g.read::<u32>(w(tab).wrapping_add(4 * u32::from(b)))
    };
    let mut far = 0f32;
    let mut i = 0u32;
    while i.cast_signed() < count.cast_signed() {
        let h = target(g, i, (0x1b6, 0x1c1));
        let p = s + 0x38;
        // call width: 4
        g.call(t(g, 0x1d5, 262), 0, &[h, p, p + 4, p + 8]);
        // call width: 3
        g.call(t(g, 0x1e8, 268), 0, &[h, 6, s + 0x5c]);
        let y = rf(g, 0x60);
        g.write::<f32>(p + 4, y);
        g.write::<u32>(p + 0xc, ONE);
        let d = [0u32, 4, 8].map(|o| sub(g.read::<f32>(cv + o), g.read::<f32>(p + o)));
        let sq = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let dist = f64::from(sq).sqrt() as f32;
        if dist > far {
            far = dist;
            g.write::<f32>(s + 0x10, far);
        }
        i += 1;
    }
    g.write::<f32>(w(0x271), far);
    let n = cvtt(div(far, k(g, 0x279)));
    g.write::<i32>(w(0x284), n);
    let base = n.wrapping_add(0x49);
    g.write::<i32>(s + 0x10, base);
    let mut i = 0u32;
    let mut v = w(0x298);
    while i.cast_signed() < count.cast_signed() {
        // `rand() % 10` (`idiv` by `i`'s 10: `ebx` is 10 on every turn)
        let r = g.rand().cast_signed() % 10;
        let scale = k(g, 0x2b3);
        start(
            g,
            (0x2c5, v - 0xd4, scale),
            (1, r.wrapping_add(base).cast_unsigned(), w(0x2a1)),
        );
        let h = target(g, i, (0x2da, 0x2e5));
        // call width: 4
        g.call(t(g, 0x2ee, 262), 0, &[h, v - 4, v, v + 4]);
        // call width: 3
        g.call(t(g, 0x309, 268), 0, &[h, 6, s + 0x7c]);
        let y = rf(g, 0x80);
        g.write::<f32>(v, y);
        g.write::<u32>(v + 8, ONE);
        let b = g.read::<u8>(gw(g, 0x33c).wrapping_add(i));
        g.write::<u32>(v + 0x4c, u32::from(b));
        let dz = sub(rf(g, 0x30), g.read::<f32>(v + 4));
        let dx = sub(rf(g, 0x28), g.read::<f32>(v - 4));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let mut yaw = sub(k(g, 0x38a), ang);
        if yaw > k(g, 0x399) {
            yaw = sub(yaw, k(g, 0x3a3));
        } else if k(g, 0x3ad) > yaw {
            yaw = add(yaw, k(g, 0x3ba));
        }
        let rec = w(0x3ca).wrapping_add(i.wrapping_mul(0x34));
        g.write::<f32>(v - 0x10, yaw);
        for o in [0x18u32, 0x14, 0x10, 0x1c] {
            g.write::<u32>(rec + o, 0);
        }
        g.write::<u32>(v + 0x3c, rec);
        g.write::<u32>(rec + 0x20, w(0x3f9));
        g.write::<u32>(rec + 0x24, w(0x400));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<u16>(rec + 0x30, 0);
        i += 1;
        v += 0x130;
    }
    // the light blocks and two nodes off the free ring
    let mut free = g.read::<u16>(w(0x426));
    let put = |g: &mut G, list: &[(u32, u32)]| {
        for &(to, v) in list {
            g.write::<u32>(w(to), v);
        }
    };
    put(
        g,
        &[
            (0x42c, ONE),
            (0x436, ONE),
            (0x440, ONE),
            (0x44a, ONE),
            (0x454, 0x3f00_0000),
            (0x45e, 0x3f00_0000),
            (0x468, 0x3f00_0000),
            (0x472, ONE),
            (0x47c, 0),
            (0x486, 0),
            (0x490, 0x20),
            (0x49a, 0x20),
        ],
    );
    let head = gw(g, 0x4b8);
    let (n1, head) = ring_take(g, img, f, head, &mut free, (0x4dd, 0x502, 0x50d));
    let first = gw(g, 0x513);
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, w(0x51e));
    let last = gw(g, 0x52e);
    task_append(g, img, f, n1, first, last, 0x543);
    let first = if first == 0 { n1 } else { first };
    put(
        g,
        &[
            (0x54c, 0x3f00_0000),
            (0x556, ONE),
            (0x560, 0x3f00_0000),
            (0x56a, 0x3f00_0000),
            (0x574, 0x3f00_0000),
            (0x57e, ONE),
            (0x588, ONE),
            (0x592, ONE),
            (0x59c, ONE),
            (0x5a6, ONE),
            (0x5b0, 0x94),
            (0x5ba, 0),
            (0x5c4, 0x20),
            (0x5ce, 0x20),
        ],
    );
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let (n2, _) = ring_take(g, img, f, head, &mut free, (0x5f8, 0x631, 0x63c));
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x647));
    task_append(g, img, f, n2, first, n1, 0x665);
    g.write::<u32>(w(0x67b), n2);
    // call width: 1
    g.call(t(g, 0x66a, 271), 0, &[4]);
    // the camera words
    let x = gw(g, 0x682);
    let flag = gw(g, 0x68b);
    g.write::<u32>(w(0x691), x);
    put(
        g,
        &[
            (0x69d, 0x1d),
            (0x6a7, 0x59),
            (0x6b1, 0x1e),
            (0x6bb, w(0x6bf)),
            (0x6c5, u32::MAX),
        ],
    );
    let slot = if flag == 0 {
        t(g, 0x697, 215)
    } else {
        t(g, 0x697, 413)
    };
    // call width: 1
    let c = g.call(slot, 0, &[x]).eax;
    let flag = gw(g, 0x6e4);
    put(g, &[(0x6ef, c), (0x6f5, u32::MAX), (0x6ff, 0), (0x709, 1)]);
    if flag == 0 {
        g.write::<u16>(w(0x713), 0);
    } else {
        g.write::<u16>(w(0x71c), 1);
    }
    // the sprite pool's node and the frame's node
    let mut free = g.read::<u16>(w(0x723));
    let head = gw(g, 0x737);
    let (n3, _) = ring_take(g, img, f, head, &mut free, (0x754, 0x78d, 0x798));
    let first = gw(g, 0x79e);
    g.write::<u8>(n3 + 0x18, 1);
    let last = gw(g, 0x7b1);
    task_append(g, img, f, n3, first, last, 0x7c4);
    g.write::<u32>(w(0x7d2), n3);
    crate::m0072::pool_setup(g, img, call(0x7d7), w(0x7c9));
    let mut free = g.read::<u16>(w(0x7de));
    let head = gw(g, 0x7ed);
    let (n4, _) = ring_take(g, img, f, head, &mut free, (0x80a, 0x83e, 0x849));
    let first = gw(g, 0x84f);
    g.write::<u8>(n4 + 0x18, 2);
    let last = gw(g, 0x862);
    task_append(g, img, f, n4, first, last, 0x875);
    g.write::<u32>(w(0x88a), n4);
    Ret::eax(0)
}

/// [`start_up_0072`] (the `rel32`s: the slot-719 global's part set-up,
/// the object starts, `_libm_sse2_sqrt_precise`, `_CIatan2`, the sprite
/// pool, `__security_check_cookie`; `rand` through the IAT).
pub const START_UP_0072: Sig = Sig {
    len: 2201,
    hash: 0x085c_4a76_3980_ed7a,
    calls: &[
        (0x5a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x76, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x245, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2c5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x378, Callee::Import("_CIatan2")),
        (0x7d7, Callee::Sig(&crate::m0072::POOL_SETUP)),
        (0x891, Callee::Any),
    ],
    imports: &[(0x2a8, "rand")],
};

/// `magic_0072`'s load step (0x100012b0, 118 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step))
/// running [`start_up_0072`].
pub fn load_0072<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0072)
}

/// [`load_0072`] (the 118-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0072: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0072))],
    imports: &[],
};

/// One of [`flames_0072`]'s sprite records `R` (taken by
/// [`pool_take`](crate::m0072::pool_take); none ends the burst): the
/// point and `w` at `+0xc..+0x18` (the frame's four words), the handle
/// `h` at `+0` (slot 719 `(D, 9, 0)` for the first kind, else the given
/// word), `+4` 1, the offset `(0, 0, r + K, 0)` at `+0x1c` (`r` slot 314
/// `(lift)`) turned by the matrix `N` (slot 229), `+0x28` slot 314
/// `(pi)`, the colour `(0, g, 0, 1)` at `+0x2c`, `+0x44` the count as a
/// float, the u16s `+0x8` 0xffff and `+0xa` 0x200; for the first kind
/// byte `+0x3e` 0x60 and byte `+0x3f` `rand() % 65 + 32`; byte `+0x43`
/// 0, `+0x5c` 8, the size `+0x4c = (slot 404 (s) + K') K''` and the
/// speed `+0x50..+0x58`. `ops` = the operands `[314's load, K, 229's
/// load, the second 314's load, 404's load, K', K'']`. Returns the record
/// and slot 404's `eax`, or `None` when no record was free.
#[allow(clippy::too_many_arguments)] // the record's inputs
fn flame_record<G: Game>(
    g: &mut G,
    img: &Image,
    (f, ebp): (u32, u32),
    handle: Option<u32>,
    (lift, green, size): (u32, u32, u32),
    ops: [u32; 7],
    speed: [u32; 3],
    take: u32,
) -> Option<(u32, u32)> {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let rec = crate::m0072::pool_take(g, img, take);
    if rec == 0 {
        return None;
    }
    let h = match handle {
        Some(h) => h,
        // call width: 3
        None => g.call(t(g, 0x322, 719), 0, &[w(0x331), 9, 0]).eax,
    };
    for (o, from) in [(0xcu32, 0xa8u32), (0x10, 0xb0), (0x14, 0xac), (0x18, 0xa4)] {
        let x = g.read::<u32>(lf(from));
        g.write::<u32>(rec + o, x);
    }
    let v = rec + 0x1c;
    g.write::<u32>(v, 0);
    g.write::<u32>(rec + 0x20, 0);
    // call width: 1
    let r = st0_f32(g.call(t(g, ops[0], 314), 0, &[lift]).st0);
    g.write::<f32>(rec + 0x24, add(r, k(g, ops[1])));
    g.write::<u32>(rec + 0x28, 0);
    // call width: 3
    g.call(t(g, ops[2], 229), 0, &[v, lf(0x54), v]);
    // call width: 1
    let a = st0_f32(g.call(t(g, ops[3], 314), 0, &[0x4049_0fdb]).st0);
    g.write::<f32>(rec + 0x28, a);
    let n = cvt(g.read::<u32>(lf(0xa0)));
    for (o, x) in [(0x2cu32, 0u32), (0x30, green), (0x34, 0), (0x38, ONE)] {
        g.write::<u32>(rec + o, x);
    }
    g.write::<f32>(rec + 0x44, n);
    g.write::<u16>(rec + 8, 0xffff);
    g.write::<u32>(rec, h);
    g.write::<u32>(rec + 4, 1);
    g.write::<u16>(rec + 0xa, 0x200);
    if handle.is_none() {
        g.write::<u8>(rec + 0x3e, 0x60);
        let b = (g.rand().cast_signed() % 0x41).to_le_bytes()[0];
        g.write::<u8>(rec + 0x3f, b.wrapping_add(0x20));
    }
    g.write::<u8>(rec + 0x43, 0);
    g.write::<u32>(rec + 0x5c, 8);
    // call width: 1
    let out = g.call(t(g, ops[4], 404), 0, &[size]);
    let s = mul(add(st0_f32(out.st0), k(g, ops[5])), k(g, ops[6]));
    for (o, x) in [(0x50u32, speed[0]), (0x54, speed[1]), (0x58, speed[2])] {
        g.write::<u32>(rec + o, x);
    }
    g.write::<f32>(rec + 0x4c, s);
    Some((rec, out.eax))
}

/// `magic_0072`'s flame burst (0x10001e80, 1928 bytes; internal, no
/// arguments; a /GS frame, `ebp` = `E - 4`). While the wait `*G_W` is
/// positive it counts down (returns it); then, when the burst count
/// `*G_N` is 0, the second wait `*G_V` counts down the same way (returns
/// it, or its value when not positive). Otherwise `*G_N` counts down and
/// a burst starts: the matrix `M` (`ebp - 0x94`, slot 235) turned by two
/// random angles (slot 314 less `K`, wrapped once; slots 328 and 329),
/// the summon's matrix `N` (slot 236 `(*G_A, 0x10)`) scaled by `K / s`
/// (`s` slot 277) in its three rows, `N = M N` (231), `M = N` (230), the
/// offset `(0, 0, 7, 1)` through `M` (229) plus `N`'s row 3 the point;
/// then twice the pair of sprite records ([`flame_record`]: the first
/// kind with slot 719's handle and the random byte, the second with the
/// handle `G_H[1]`), and a last record (the handle `G_H[2]`, green 0.2,
/// `+0x64` 1). Returns the last slot 404's `eax`, or 0 when the pool ran
/// out.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn flames_0072<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let cookie = gw(g, 0xa) ^ ebp;
    g.write::<u32>(lf(4), cookie);
    let wait = gw(g, 0x14).cast_signed();
    if wait > 0 {
        let x = wait.wrapping_sub(1).cast_unsigned();
        g.write::<u32>(w(0x1e), x);
        return Ret::eax(x);
    }
    let n = gw(g, 0x31);
    if n == 0 {
        let v = gw(g, 0x3a);
        if v.cast_signed() <= 0 {
            return Ret::eax(v);
        }
        let x = v.wrapping_sub(1);
        g.write::<u32>(w(0x48), x);
        return Ret::eax(x);
    }
    g.write::<u32>(w(0x5c), n.wrapping_sub(1));
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x61, 314), 0, &[0x3e20_d97c]).st0);
    let mut a = sub(r, k(g, 0x87));
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x8c, 314), 0, &[0x3fc9_0fdb]).st0);
    let mut b = sub(r, k(g, 0xb9));
    let m = lf(0x94);
    // call width: 1
    g.call(t(g, 0xc5, 235), 0, &[m]);
    let tw = k(g, 0xe5);
    if a > k(g, 0xef) {
        a = sub(a, tw);
    } else if k(g, 0xff) > a {
        a = add(a, tw);
    }
    // call width: 3
    g.call(t(g, 0x11b, 328), 0, &[m, m, a.to_bits()]);
    if b > k(g, 0x135) {
        b = sub(b, k(g, 0x13f));
    } else if k(g, 0x149) > b {
        b = add(b, k(g, 0x156));
    }
    // call width: 3
    g.call(t(g, 0x169, 329), 0, &[m, m, b.to_bits()]);
    let nm = lf(0x54);
    let a_h = gw(g, 0x182);
    // call width: 3
    g.call(t(g, 0x17a, 236), 0, &[a_h, 0x10, nm]);
    let a_h = gw(g, 0x195);
    // call width: 1
    let s = st0_f32(g.call(t(g, 0x18f, 277), 0, &[a_h]).st0);
    let q = div(k(g, 0x1ab), s);
    for i in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
        let x = mul(g.read::<f32>(nm + 4 * i), q);
        g.write::<f32>(nm + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x1e9, 231), 0, &[nm, nm, m]);
    let v = lf(0x14);
    for (i, x) in (0u32..).zip([0u32, 0, 0x40e0_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 2
    g.call(t(g, 0x25a, 230), 0, &[m, nm]);
    for o in [0x5cu32, 0x60, 0x64] {
        g.write::<u32>(lf(o), 0);
    }
    // call width: 3
    g.call(t(g, 0x292, 229), 0, &[v, m, v]);
    let y0 = g.read::<u32>(lf(0x18));
    g.write::<u32>(lf(0xa4), y0);
    let d = gw(g, 0x2bc);
    g.write::<u32>(lf(0xa0), d);
    g.write::<u32>(lf(0x9c), 0);
    for (to, a, b) in [
        (0xa8u32, 0x24u32, 0x14u32),
        (0xb0, 0x20, 0x10),
        (0xac, 0x1c, 0xc),
    ] {
        let x = add(g.read::<f32>(lf(a)), g.read::<f32>(lf(b)));
        g.write::<f32>(lf(to), x);
    }
    let take = |o: u32| img.rel32_target(f + o);
    let fe = (f, ebp);
    let words = |g: &G, o: u32, i: u32| g.read::<u32>(gw(g, o) + 4 * i);
    for turn in 0..2u32 {
        if flame_record(
            g,
            img,
            fe,
            None,
            (0x3fc0_0000, 0, ONE),
            [0x369, 0x3a6, 0x3b0, 0x3ca, 0x43b, 0x471, 0x491],
            [0xc1a0_0000, 0xc0c0_0000, 0],
            take(0x312),
        )
        .is_none()
        {
            return Ret::eax(0);
        }
        let h = words(g, 0x4ab, 1);
        if flame_record(
            g,
            img,
            fe,
            Some(h),
            (ONE, 0, ONE),
            [0x4bb, 0x51d, 0x527, 0x541, 0x599, 0x5d8, 0x5f2],
            [0xc1a0_0000, 0xc0c0_0000, 0],
            take(0x49b),
        )
        .is_none()
        {
            return Ret::eax(0);
        }
        g.write::<u32>(lf(0x9c), turn + 1);
    }
    let h = words(g, 0x622, 2);
    let Some((rec, eax)) = flame_record(
        g,
        img,
        fe,
        Some(h),
        (ONE, 0x3e4c_cccd, 0x3f00_0000),
        [0x632, 0x694, 0x69e, 0x6b8, 0x70e, 0x74a, 0x76e],
        [0, 0x447a_0000, 0],
        take(0x612),
    ) else {
        return Ret::eax(0);
    };
    g.write::<u32>(rec + 0x64, 1);
    Ret::eax(eax)
}

/// [`flames_0072`] (the `rel32`s: `__security_check_cookie` three times,
/// the pool take three times; `rand` through the IAT).
pub const FLAMES_0072: Sig = Sig {
    len: 1928,
    hash: 0x94f7_b61b_8192_69cf,
    calls: &[
        (0x28, Callee::Any),
        (0x52, Callee::Any),
        (0x312, Callee::Sig(&crate::m0072::POOL_TAKE)),
        (0x49b, Callee::Sig(&crate::m0072::POOL_TAKE)),
        (0x612, Callee::Sig(&crate::m0072::POOL_TAKE)),
        (0x780, Callee::Any),
    ],
    imports: &[(0x42e, "rand")],
};

/// `magic_0072`'s frame step (0x10001d30, 314 bytes; a task step, cdecl
/// `(task)`; an 8-aligned frame): the flame burst ([`flames_0072`]), the
/// objects ([`run_objects`](crate::l5_objhost::run_objects)) and the
/// motion script ([`motion_step`](crate::s3_motion::motion_step)); when
/// both give 1 and `*G_F` is 0: for each of `*G_N` handles `i`, three
/// sounds (slot 772 `(h, i)` of slot 719 `(D, 9, 0)`'s handle, then of
/// `G_H[1]` and `G_H[2]`, each played by slot 779 `(lo, hi)` with the
/// lock word set to 1 and put back), the task's state 3 and its record's
/// `+0x10` the next step. Returns the record, else the motion's result.
pub fn frame_0072<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; push esi; push edi; the calls
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    let ff = Frame {
        at: call(0x9),
        esp,
        ..*fr
    };
    flames_0072(g, img, &ff);
    let objs = crate::l5_objhost::run_objects(g, img, call(0xe), esp);
    let mv = crate::s3_motion::motion_step(g, img, call(0x15));
    if mv != 1 || objs != 1 || gw(g, 0x2c) != 0 {
        return Ret::eax(mv);
    }
    if gw(g, 0x3b).cast_signed() > 0 {
        let mut i = 0u32;
        loop {
            // call width: 3
            let h0 = g.call(t(g, 0x51, 719), 0, &[w(0x60), 9, 0]).eax;
            // (the handle's source, slot 772's load, the lock's global
            // before and after, slot 779's load)
            for (src, s772, lock_at, s779, back) in [
                (None, 0x6cu32, 0x7au32, 0x83u32, 0x96u32),
                (Some((0x9e, 4)), 0xa6, 0xb4, 0xbd, 0xd0),
                (Some((0xd8, 8)), 0xe0, 0xee, 0xf7, 0x10a),
            ] {
                let h = match src {
                    None => h0,
                    Some((o, d)) => g.read::<u32>(gw(g, o) + d),
                };
                // call width: 2
                let out = g.call(t(g, s772, 772), 0, &[h, i]);
                let lock = gw(g, lock_at);
                let old = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                // call width: 2
                g.call(t(g, s779, 779), 0, &[out.eax, out.edx]);
                let lock = gw(g, back);
                g.write::<u32>(lock, old);
            }
            i += 1;
            if i.cast_signed() >= gw(g, 0x116).cast_signed() {
                break;
            }
        }
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x130));
    Ret::eax(rec)
}

/// [`frame_0072`] (the `rel32`s: the flames, the objects, the motion).
pub const FRAME_0072: Sig = Sig {
    len: 314,
    hash: 0x56a2_4e0a_75d9_5d4c,
    calls: &[
        (0x9, Callee::Sig(&FLAMES_0072)),
        (0xe, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x15, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// `magic_0009`'s start-up (0x10001340, 1464 bytes; internal, no
/// arguments, run by its load step; an 8-aligned /GS frame, `S` = `esp`
/// after the three pushes):
/// - four globals, `*G_C` = slot 719 `(D, 8, 0)`; the part set-up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - the target `T` (`TAB[*sel]`): `A` = slot 268 `(T, 0x11)` at `S +
///   0x30` (after slot 262 there) with `y` less `K`, `w` 1; the summon
///   `B` (slot 262 then 268 of `*G_B` at `S + 0x40`, `y` less `K`, `w`
///   1);
/// - `d = A - B`: the pitch `atan2(dy, sqrt(dx^2 + dz^2))` (doubles, the
///   `x87` `_CIatan2`), the yaw `K - atan2(dz, dx)` wrapped once;
/// - `A` moved by the matrix of slot 235 turned by the pitch and the yaw
///   (slots 328, 329) applied to `(0, 0, 6, 1)` (slots 229, 248), the
///   first part object ([`part_object`](crate::m0009::part_object) `(1,
///   0)`), `A` into four globals; the yaw plus `K'` wrapped into a
///   global, the target's byte, the second part object `(0, 0x12)`, `B`
///   into four globals, a handle word and a block of globals; `B` moved
///   by the same turn applied to `(0, 0, 3, 1)`;
/// - the effect block set up ([`setup`](crate::m0009::setup) `(A, B,
///   (pitch, yaw, 0))`), three nodes off the pool
///   ([`take`](crate::sa_pool::take): the frame task, marked 1; the
///   trail task; the objects' frame, marked 2) at the task list's end.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0009<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0xac; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xac);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0xa8, cookie);
    let s = b0.wrapping_sub(0xc);
    let rf = |g: &G, o: u32| g.read::<f32>(s + o);
    let wf = |g: &mut G, o: u32, x: f32| g.write::<f32>(s + o, x);
    for (to, v) in [(0x33u32, 0x37u32), (0x3d, 0x41), (0x47, 0x4b), (0x51, 0x55)] {
        g.write::<u32>(w(to), w(v));
    }
    // call width: 3
    let c = g.call(t(g, 0x1b, 719), 0, &[w(0x2d), 8, 0]).eax;
    g.write::<u32>(w(0x5f), c);
    crate::l5_objhost::part_init(g, img, call(0x64));
    // the target A at S + 0x30
    let b = g.read::<u8>(gw(g, 0x69));
    let tt = g.read::<u32>(w(0x73).wrapping_add(4 * u32::from(b)));
    let a = s + 0x30;
    // call width: 4
    g.call(t(g, 0x87, 262), 0, &[tt, a, a + 4, a + 8]);
    // call width: 3
    g.call(t(g, 0x9a, 268), 0, &[tt, 0x11, a]);
    let ya = sub(rf(g, 0x34), k(g, 0xb3));
    // the summon B at S + 0x40
    let bh = gw(g, 0xb9);
    let bv = s + 0x40;
    g.write::<u32>(a + 0xc, ONE);
    wf(g, 0x34, ya);
    // call width: 4
    g.call(t(g, 0xcd, 262), 0, &[bh, bv, bv + 4, bv + 8]);
    // call width: 3
    g.call(t(g, 0xee, 268), 0, &[bh, 0x11, bv]);
    let yb = sub(rf(g, 0x44), k(g, 0x10a));
    let dz = sub(rf(g, 0x38), rf(g, 0x48));
    let dx = sub(rf(g, 0x30), rf(g, 0x40));
    let dy = sub(ya, yb);
    wf(g, 0x44, yb);
    g.write::<u32>(bv + 0xc, ONE);
    wf(g, 0x1c, dx);
    let flat = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let r = f64::from(flat).sqrt();
    let pitch64 = g.atan2(f64::from(dy), r);
    g.write::<f64>(s + 0x10, pitch64);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let pitch = pitch64 as f32;
    wf(g, 0x20, pitch);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = sub(k(g, 0x1d9), ang);
    let tw = k(g, 0x1e9);
    if yaw > k(g, 0x1f0) {
        yaw = sub(yaw, tw);
    } else if k(g, 0x206) > yaw {
        yaw = add(yaw, tw);
    }
    wf(g, 0xc, yaw);
    // the turn applied to (0, 0, z, 1), the point moved (slot 248)
    let m = s + 0x70;
    let v = s + 0x60;
    let out = s + 0x50;
    let turned = |g: &mut G, p: u32, z: u32, ops: [u32; 5]| {
        // call width: 1
        g.call(t(g, ops[0], 235), 0, &[m]);
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let pitch = g.read::<f64>(s + 0x10) as f32;
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[m, m, pitch.to_bits()]);
        let yaw = rf(g, 0xc);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[m, m, yaw.to_bits()]);
        g.write::<u32>(v + 8, z);
        g.write::<u32>(v + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[out, m, v]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[p, p, out]);
    };
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 8, 0x40c0_0000);
    wf(g, 0x24, yaw);
    g.write::<u32>(s + 0x28, 0);
    turned(g, a, 0x40c0_0000, [0x21f, 0x267, 0x288, 0x2a7, 0x2ca]);
    g.write::<u32>(a + 0xc, ONE);
    let obj = |g: &mut G, at: u32, (o, scale): (u32, u32), (index, owner): (u32, u32)| {
        let sc = gw(g, scale);
        crate::m0009::part_object(g, img, call(at), w(o), sc, index, owner);
    };
    obj(g, 0x2f3, (0x2e2, 0x2da), (1, 0));
    for (i, to) in (0u32..).zip([0x301u32, 0x30f, 0x31d, 0x32b]) {
        let x = g.read::<u32>(a + 4 * i);
        g.write::<u32>(w(to), x);
    }
    let mut yaw2 = add(rf(g, 0xc), k(g, 0x339));
    if yaw2 > k(g, 0x343) {
        yaw2 = sub(yaw2, k(g, 0x34d));
    } else if k(g, 0x357) > yaw2 {
        yaw2 = add(yaw2, k(g, 0x364));
    }
    let byte = g.read::<u8>(gw(g, 0x369));
    g.write::<f32>(w(0x37c), yaw2);
    g.write::<u32>(w(0x38d), u32::from(byte));
    obj(g, 0x392, (0x388, 0x371), (0, 0x12));
    let hw = gw(g, 0x39d);
    for (i, to) in (0u32..).zip([0x3a5u32, 0x3b3, 0x3c8, 0x3e3]) {
        let x = g.read::<u32>(bv + 4 * i);
        g.write::<u32>(w(to), x);
    }
    g.write::<u32>(w(0x3be), hw);
    g.write::<u16>(w(0x3d4), 0);
    let x = rf(g, 0xc);
    g.write::<f32>(w(0x3f7), x);
    for (to, x) in [
        (0x3fdu32, 0u32),
        (0x407, 0),
        (0x411, 0),
        (0x41b, 0),
        (0x425, w(0x429)),
        (0x42f, w(0x433)),
        (0x439, 0),
        (0x443, 0),
    ] {
        g.write::<u32>(w(to), x);
    }
    turned(g, bv, 0x4040_0000, [0x3ef, 0x474, 0x495, 0x4b4, 0x4d7]);
    g.write::<u32>(bv + 0xc, ONE);
    crate::m0009::setup(g, img, call(0x4f9), a, bv, s + 0x20);
    // three nodes off the pool at the task list's end
    let take = |g: &mut G, o: u32, owner: u32| crate::sa_pool::take(g, img, call(o), w(owner));
    let n1 = take(g, 0x506, 0x501);
    let mut first = gw(g, 0x50c);
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x538), first);
    } else {
        let last = gw(g, 0x523);
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let n2 = take(g, 0x545, 0x53d);
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        first = n2;
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x567), first);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    let n3 = take(g, 0x574, 0x56c);
    g.write::<u8>(n3 + 0x18, 2);
    g.write::<u32>(n3 + 4, 0);
    if first == 0 {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x595), n3);
    } else {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2 + 4, n3);
    }
    g.write::<u32>(n3 + 8, n3);
    g.write::<u32>(w(0x5a7), n3);
    Ret::eax(0)
}

/// [`start_up_0009`] (the `rel32`s: the part set-up, `sqrt`, `_CIatan2`
/// twice, the part objects, the effect set-up, the pool takes,
/// `__security_check_cookie`).
pub const START_UP_0009: Sig = Sig {
    len: 1464,
    hash: 0xda80_412b_36f2_4009,
    calls: &[
        (0x64, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x175, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x188, Callee::Import("_CIatan2")),
        (0x1c7, Callee::Import("_CIatan2")),
        (0x2f3, Callee::Sig(&crate::m0009::PART_OBJECT)),
        (0x392, Callee::Sig(&crate::m0009::PART_OBJECT)),
        (0x4f9, Callee::Sig(&crate::m0009::SETUP)),
        (0x506, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x545, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x574, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x5b0, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0009`'s load step (0x10001210, 119 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step), the
/// 119-byte form) running [`start_up_0009`].
pub fn load_0009<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::One, start_up_0009)
}

/// [`load_0009`] (the 119-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0009: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&START_UP_0009))],
    imports: &[],
};

/// `magic_0009`'s trail task (0x10001c70, 2038 bytes; a task callback,
/// cdecl `(task)`; an 8-aligned /GS frame, `S` = `esp` after the three
/// pushes; `R = *G_FX`, the effect block). The count `R + 0x9b8` steps
/// down; `u = n / K` and `u' = (n + 1) / K` place two points between
/// `R + 0x40` and `R + 0x50` (slot 326) lowered by the arc `(1 - u) u K1
/// K2` (the other terms times 0); their difference gives the pitch
/// `atan2(dy, sqrt(dx^2 + dz^2))` and the yaw `K - atan2(dz, dx)`
/// (`_CIatan2`, wrapped once) of the matrix `M1` (slots 235, 330 by 0,
/// 328, 329); the height of the arc's next step `Q = (0, h, 0, 1)`; the
/// screen and camera matrices (723, 724) and a small scale matrix `M4`
/// (235, 0.05).
/// - With the count still running: for each of the four 0x250-byte
///   records `T` from `R + 0x70`: its spin `T + 0x24c` plus `K` wrapped
///   once (slots 235, 330), `M5 = M5 M1` (231), the point `M5 Q` plus the
///   first point through `R`'s matrix (229 twice) plus `R + 0x60`, into
///   `T + 0xc` (and `w`) and the frame; `*G_W = T[0x1c]`; the sprite
///   matrix `M6` scaled by `T + 0x20` placed at the camera's point
///   (229) and turned into the screen (231); the record's animation frame
///   (slot 941's table, the timer `T + 0xa` down by 0x200, the frame
///   `T + 8` on, looping on the entry's flag 0x80 or holding at the last);
///   the part linked ([`link_part`](crate::sa_misc::link_part)) and the
///   trail drawn ([`trail_draw`](crate::s4_misc::trail_draw)). Then the
///   last point into the object's four words and its script
///   ([`obj_script`](crate::l5_objhost::obj_script)). Returns the
///   script's flag bit 15.
/// - Done: while the first record's `+0x248` is positive, each record's
///   is halved and its trail drawn (returns the last draw's `eax`); else
///   the task goes back to the pool ([`give_back`](crate::sa_pool::give_back)).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn trail_task_0009<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x1e4; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1e4);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x1e0, cookie);
    let s = b0.wrapping_sub(0xc);
    let at = |o: u32| s.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(s.wrapping_add(o));
    let wf = |g: &mut G, o: u32, x: f32| g.write::<f32>(s.wrapping_add(o), x);
    let task = fr.arg(g, 0);
    let r = gw(g, 0x20);
    g.write::<u32>(at(0x20), task);
    g.write::<u32>(at(0x1c), r);
    let count = r + 0x9b8;
    let c = g.read::<i32>(count);
    if c > 0 {
        g.write::<i32>(count, c - 1);
    }
    let c = g.read::<u32>(count);
    let u = if c == 0 { 0.0 } else { div(cvt(c), k(g, 0x54)) };
    wf(g, 0x18, u);
    let (a0, a1) = (r + 0x40, r + 0x50);
    // call width: 4
    g.call(t(g, 0x71, 326), 0, &[at(0x58), a0, a1, u.to_bits()]);
    let one_u = sub(k(g, 0x8d), u);
    let uu = mul(u, u);
    let h1 = add(
        add(
            mul(mul(mul(one_u, u), k(g, 0xb1)), k(g, 0xc0)),
            mul(mul(one_u, one_u), 0.0),
        ),
        mul(uu, 0.0),
    );
    wf(g, 0x34, one_u);
    wf(g, 0x24, uu);
    let y1 = sub(rf(g, 0x5c), h1);
    wf(g, 0x5c, y1);
    // (the count read again after the call)
    let u2 = div(cvt(g.read::<u32>(count).wrapping_add(1)), k(g, 0x105));
    wf(g, 0xc, u2);
    // call width: 4
    g.call(t(g, 0x112, 326), 0, &[at(0x48), a0, a1, u2.to_bits()]);
    let one_u2 = sub(k(g, 0x12e), u2);
    let h2 = add(
        add(
            mul(mul(mul(one_u2, u2), k(g, 0x14b)), k(g, 0x157)),
            mul(mul(one_u2, one_u2), 0.0),
        ),
        mul(mul(u2, u2), 0.0),
    );
    let dz = sub(rf(g, 0x60), rf(g, 0x50));
    let dx = sub(rf(g, 0x58), rf(g, 0x48));
    let y2 = sub(rf(g, 0x4c), h2);
    wf(g, 0xc, dz);
    wf(g, 0x30, dx);
    let dy = sub(rf(g, 0x5c), y2);
    wf(g, 0x4c, y2);
    let flat = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let rr = f64::from(flat).sqrt();
    g.write::<f64>(at(0x10), f64::from(dy));
    let pitch = g.atan2(f64::from(dy), rr);
    g.write::<f64>(at(0x28), pitch);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = sub(k(g, 0x21c), ang);
    let tw = k(g, 0x22c);
    if yaw > k(g, 0x233) {
        yaw = sub(yaw, tw);
    } else if k(g, 0x249) > yaw {
        yaw = add(yaw, tw);
    }
    wf(g, 0xc, yaw);
    let m1 = at(0xa8);
    // call width: 1
    g.call(t(g, 0x265, 235), 0, &[m1]);
    // call width: 3
    g.call(t(g, 0x282, 330), 0, &[m1, m1, 0]);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let p32 = pitch as f32;
    // call width: 3
    g.call(t(g, 0x2aa, 328), 0, &[m1, m1, p32.to_bits()]);
    // call width: 3
    g.call(t(g, 0x2ce, 329), 0, &[m1, m1, yaw.to_bits()]);
    // the arc's next height
    let kk = k(g, 0x2e4);
    let x2 = mul(one_u, one_u);
    let x5 = mul(one_u, u);
    let h3 = mul(mul(x5, x2), kk);
    let h3 = add(mul(h3, k(g, 0x322)), mul(mul(x2, x2), 0.0));
    let h3 = add(h3, mul(mul(mul(uu, x2), kk), k(g, 0x357)));
    let h3 = add(h3, mul(mul(mul(uu, x5), kk), k(g, 0x376)));
    let h3 = add(h3, mul(mul(uu, uu), 0.0));
    let q = at(0x48);
    for (o, x) in [(0u32, 0u32), (8, 0), (0xc, ONE)] {
        g.write::<u32>(q + o, x);
    }
    g.write::<f32>(q + 4, h3);
    let (m2, m3, m4) = (at(0x1a8), at(0x168), at(0xe8));
    // call width: 1
    g.call(t(g, 0x328, 723), 0, &[m2]);
    // call width: 1
    g.call(t(g, 0x39c, 724), 0, &[m3]);
    // call width: 1
    g.call(t(g, 0x3b4, 235), 0, &[m4]);
    let ended = g.read::<u32>(count) == 0;
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(m4 + o, 0x3d4c_cccd);
    }
    if ended {
        let mut half = r + 0x70 + 0x248;
        if g.read::<i32>(half) <= 0 {
            let node = g.read::<u32>(at(0x20));
            let eax = crate::sa_pool::give_back(g, img, call(0x7dd), node);
            return Ret::eax(eax);
        }
        let mut eax = 0;
        for _ in 0..4 {
            let v = g.read::<i32>(half) / 2;
            g.write::<i32>(half, v);
            g.write::<u32>(at(0) - 4, half - 0x23c);
            let tf = Frame {
                at: call(0x7b3),
                esp: at(0) - 8,
                ecx: half - 0x224,
                edx: m4,
                ..*fr
            };
            eax = crate::s4_misc::trail_draw(g, img, &tf);
            half += 0x250;
        }
        return Ret::eax(eax);
    }
    let (m5, m6, p) = (at(0x128), at(0x68), at(0x38));
    g.write::<u32>(w(0x3fb), m6);
    g.write::<u32>(w(0x401), 0);
    for i in 0..4u32 {
        let rec = r + 0x70 + 0x250 * i;
        let mut spin = add(g.read::<f32>(rec + 0x24c), k(g, 0x415));
        if spin > k(g, 0x41c) {
            spin = sub(spin, k(g, 0x426));
        } else if k(g, 0x430) > spin {
            spin = add(spin, k(g, 0x43d));
        }
        g.write::<f32>(rec + 0x24c, spin);
        // call width: 1
        g.call(t(g, 0x44a, 235), 0, &[m5]);
        let spin = g.read::<u32>(rec + 0x24c);
        // call width: 3
        g.call(t(g, 0x475, 330), 0, &[m5, m5, spin]);
        // call width: 3
        g.call(t(g, 0x49d, 231), 0, &[m5, m1, m5]);
        // call width: 3
        g.call(t(g, 0x4bf, 229), 0, &[p, m5, q]);
        for o in [0u32, 4, 8] {
            let x = add(g.read::<f32>(p + o), rf(g, 0x58 + o));
            g.write::<f32>(p + o, x);
        }
        // call width: 3
        g.call(t(g, 0x4f4, 229), 0, &[p, r, p]);
        let pw = g.read::<u32>(r + 0x6c);
        let pos = [0u32, 4, 8].map(|o| add(g.read::<f32>(p + o), g.read::<f32>(r + 0x60 + o)));
        g.write::<u32>(rec + 0x18, pw);
        for (o, x) in (0u32..).zip(pos) {
            g.write::<f32>(rec + 0xc + 4 * o, x);
        }
        let x = g.read::<u32>(rec + 0x1c);
        g.write::<u32>(w(0x558), x);
        for (o, x) in (0u32..).zip(pos) {
            g.write::<f32>(p + 4 * o, x);
        }
        g.write::<u32>(p + 0xc, pw);
        // call width: 1
        g.call(t(g, 0x565, 235), 0, &[m6]);
        let sc = g.read::<f32>(rec + 0x20);
        for j in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
            let x = mul(g.read::<f32>(m6 + 4 * j), sc);
            g.write::<f32>(m6 + 4 * j, x);
        }
        // call width: 3
        g.call(t(g, 0x5e0, 229), 0, &[m6 + 0x30, m3, rec + 0xc]);
        // call width: 3
        g.call(t(g, 0x679, 231), 0, &[m6, m2, m6]);
        let x = g.read::<u32>(rec);
        // call width: 1
        let tbl = g.call(t(g, 0x686, 941), 0, &[x]).eax;
        let fi = g.read::<u16>(rec + 8);
        let entry_of = |fi: u16| {
            tbl.wrapping_add(
                (i32::from(fi.cast_signed()) + 2)
                    .cast_unsigned()
                    .wrapping_mul(8),
            )
        };
        let mut entry = entry_of(fi);
        if g.read::<u32>(rec + 4) != 0 {
            let timer = g.read::<u16>(rec + 0xa).wrapping_sub(0x200);
            g.write::<u16>(rec + 0xa, timer);
            if timer.cast_signed() <= 0 {
                let next = fi.wrapping_add(1);
                g.write::<u16>(rec + 8, next);
                let reload = if next.cast_signed() < g.read::<i16>(tbl + 6) {
                    true
                } else if g.read::<u8>(entry + 4) & 0x80 != 0 {
                    g.write::<u16>(rec + 8, 0);
                    true
                } else {
                    g.write::<u16>(rec + 8, fi);
                    g.write::<u32>(rec + 4, 0);
                    false
                };
                if reload {
                    entry = entry_of(g.read::<u16>(rec + 8));
                    let x = g.read::<u16>(entry + 2);
                    g.write::<u16>(rec + 0xa, x);
                }
            }
        }
        // the part linked: `edx` the entry, stack `(owner, i)`
        g.write::<u32>(at(0) - 8, w(0x6f9));
        g.write::<u32>(at(0) - 4, i);
        let lf = Frame {
            at: call(0x700),
            esp: at(0) - 0xc,
            ecx: rec,
            edx: entry,
            ..*fr
        };
        crate::sa_misc::link_part(g, img, &lf);
        g.write::<u32>(at(0) - 0xc, rec + 0xc);
        let tf = Frame {
            at: call(0x713),
            esp: at(0) - 0x10,
            ecx: rec + 0x24,
            edx: m4,
            ..*fr
        };
        crate::s4_misc::trail_draw(g, img, &tf);
    }
    for (o, to) in [(0u32, 0x738u32), (4, 0x746), (8, 0x754), (0xc, 0x767)] {
        let x = g.read::<u32>(p + o);
        g.write::<u32>(w(to), x);
    }
    let obj = w(0x75f);
    crate::l5_objhost::obj_script(g, img, call(0x76c), obj);
    // (its `eax`: bit 15 of the object's flags)
    Ret::eax(u32::from(g.read::<u16>(obj + 0x30) >> 15))
}

/// [`trail_task_0009`] (the `rel32`s: `sqrt`, `_CIatan2` twice, the part
/// link, the trail draw twice, the script, the give-back,
/// `__security_check_cookie` three times).
pub const TRAIL_TASK_0009: Sig = Sig {
    len: 2038,
    hash: 0x24cb_5289_965f_41fc,
    calls: &[
        (0x1c8, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1db, Callee::Import("_CIatan2")),
        (0x20a, Callee::Import("_CIatan2")),
        (0x700, Callee::Sig(&crate::sa_misc::LINK_PART)),
        (0x713, Callee::Sig(&crate::s4_misc::TRAIL_DRAW_0009)),
        (0x76c, Callee::Sig(&crate::l5_objhost::OBJ_SCRIPT)),
        (0x77d, Callee::Any),
        (0x7b3, Callee::Sig(&crate::s4_misc::TRAIL_DRAW_0009)),
        (0x7d0, Callee::Any),
        (0x7dd, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x7ee, Callee::Any),
    ],
    imports: &[],
};

/// [`stage_task_0150`]'s ending (shared by states 6 and 101), three
/// independent steps by the timer `t`: past `K1` the glow `*G_G` drops
/// by the step `*G_S` (which shrinks by `m`); past `K2` the flash `*G_F`
/// shrinks; past `K3` the node leaves its parent
/// ([`leave`](crate::sa_misc::leave)) and the task node is unlinked
/// (returns its old next). `ks` the three bounds' operands, `ops` =
/// `[the step's load, the glow's load, its store, the step's store, the
/// flash's load, its factor, its store]`. Returns `eax` as the original
/// leaves it.
#[allow(clippy::too_many_arguments)] // the shared tail's operands
fn stage_end_0150<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (t, m): (f32, f32),
    ks: [u32; 3],
    ops: [u32; 7],
    task: u32,
    eax: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // `comiss t, K; jb`: below or unordered skips the step
    let past = |g: &G, o: u32| !(t < k(g, o) || t.is_nan());
    if past(g, ks[0]) {
        let step = k(g, ops[0]);
        let glow = sub(k(g, ops[1]), step);
        g.write::<f32>(w(ops[2]), glow);
        g.write::<f32>(w(ops[3]), mul(step, m));
    }
    if past(g, ks[1]) {
        let x = mul(k(g, ops[4]), k(g, ops[5]));
        g.write::<f32>(w(ops[6]), x);
    }
    if !past(g, ks[2]) {
        return eax;
    }
    crate::sa_misc::leave(g, w(0xfda), eax);
    // `task` is the frame word holding the task: its node is `[[task]]`
    let node = g.read::<u32>(g.read::<u32>(task));
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node + 8, node);
    next
}

/// The flash of [`stage_task_0150`] (states 6 and 101, operands `d`
/// bytes apart): when the timer reached `K` and the move is armed
/// (`*G_M`), seven colour words 1.0 and the move started
/// ([`move_start`](crate::sa_misc::move_start) `(N, 0, 0.02, 1)` on the
/// frame), the flag cleared; the glow byte `(A + B) K'` truncated; slot
/// 250 `(0, (slot 307 - K'') *G_R)`; `*G_R` and `B` shrink by `m`. Returns
/// slot 250's `eax`.
fn stage_flash_0150<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, d: u32) -> u32 {
    let w = |o: u32| img.word(f + o.wrapping_add(d));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    if g.read::<u32>(w(0x8d6)) != 0 {
        for o in [0x8e3u32, 0x8ed, 0x8f7, 0x901, 0x90b, 0x915, 0x91f] {
            g.write::<u32>(w(o), ONE);
        }
        let rec = s + 0x34;
        for (i, x) in (0u32..).zip([w(0x92b), 0, 0x3ca3_d70a, ONE]) {
            g.write::<u32>(rec + 4 * i, x);
        }
        crate::sa_misc::move_start(g, rec);
        g.write::<u32>(w(0x94e), 0);
    }
    let x = mul(add(k(g, 0x95a), k(g, 0x962)), k(g, 0x96a));
    g.write::<u8>(w(0x973), cvtt(x).to_le_bytes()[0]);
    // call width: 0
    let r = st0_f32(g.call(t(g, 0x978, 307), 0, &[]).st0);
    let v = mul(sub(r, k(g, 0x992)), k(g, 0x9a8));
    // call width: 2
    let eax = g.call(t(g, 0x997, 250), 0, &[0, v.to_bits()]).eax;
    let m = k(g, 0x9c7);
    let x = mul(k(g, 0x9bf), m);
    g.write::<f32>(w(0x9de), x);
    let x = mul(k(g, 0x9e6), m);
    g.write::<f32>(w(0x9f2), x);
    eax
}

/// `magic_0150`'s stage task (0x1000bac0, 4258 bytes with its tables; a
/// task callback `(task)`, cdecl; an 8-aligned /GS frame, `S` = `esp`
/// after the two pushes). The timer `T` grows by `K` each run; by the
/// state (a byte table up to 0x65, the rest the end):
/// - 5: the counter `C` steps the light words (`A`, `B` less 100) and the
///   fade `C K + K0`; from 12 on, with `*G_Q` = 1 the state turns 100
///   and runs as 100; else the summon's direction (slot 232), the node
///   joins its list ([`node_append`](crate::sb_misc::node_append)), the
///   place (slot 325) moved back along it by `K`, its height, a sound
///   (286), the camera's mode (slot 520: two offsets by kind; `*G_Q` 2
///   lowers it), the second direction (232) times 20 (233), the state on
///   and five words;
/// - 0: the camera tree ([`camera_tree`](crate::s5_misc::camera_tree)),
///   a sound, slot 211, the motion 0xb9 (228), on; then as 1;
/// - 1: the place (229 from `d = K / *G_D`), the height from slot 289,
///   the mode 0xd4 lowering it, the offset (229), slot 289 of 294's
///   answer (1 for 0xff), slot 255 unless the flag, slot 3, on;
/// - 2: `x` up by `K`; on once `T` reaches `K'`;
/// - 3: the light set-up (eight colour words, the scales, the flags, `A`
///   0x1c78, `B` 0x400, `C` 0), slots 531 and 255 unless the flag; then
///   as 4;
/// - 4: the fade and the light words down, `B` less 130, `A` less 100;
///   on (glow byte 0x40, 0.9) once `T` reaches `K`;
/// - 6: the place moved by the step (233 by 0.75), a sound by `1 - g`,
///   `g` shrinks; the flash and the end ([`stage_flash_0150`],
///   [`stage_end_0150`]);
/// - 100: the direction (232), the node joins its list, the place
///   (325), a sound, five factors by the summon's kind (`*G_K`
///   0x1088/0x1089/0x108c, else by slot 535) moving the place, its
///   height, the side vector (232, 233, 248), the step (232, 233), a
///   sound; on, four words; then as 101;
/// - 101: the place back by the step (233 by 0.95); the flash and the
///   end.
///
/// Returns `eax` as the original leaves it (the byte table's index, the
/// state, the last call's).
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn stage_task_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    // push ebp; and esp, -8; sub esp, 0x40; two pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x40);
    let cookie = gw(g, 0xa) ^ b0;
    g.write::<u32>(b0 + 0x3c, cookie);
    let s = b0.wrapping_sub(8);
    let at = |o: u32| s.wrapping_add(o);
    let task = fr.arg(g, 0);
    g.write::<u32>(at(0x1c), task);
    let tm = add(k(g, 0x18), k(g, 0x23));
    let mut state = gw(g, 0x29);
    put(g, 0x39, tm);
    let up = |g: &mut G, o: u32| {
        let x = gw(g, o).wrapping_add(1);
        g.write::<u32>(w(o), x);
    };
    if state > 0x65 {
        return Ret::eax(state);
    }
    let k4 = k(g, 0x4a);
    let idx = img.word(w(0x63).wrapping_add(state)) & 0xff;
    let mut target = img.word(w(0x6a).wrapping_add(4 * idx)).wrapping_sub(f);
    let eax = loop {
        match target {
            // state 5
            0x6e => {
                let c = gw(g, 0x5c);
                let x = add(mul(cvt(c), k(g, 0x7b)), k4);
                let (b, a) = (
                    gw(g, 0x56).wrapping_sub(0x64),
                    gw(g, 0x50).wrapping_sub(0x64),
                );
                g.write::<u32>(w(0x8c), b);
                g.write::<u32>(w(0x92), a);
                g.write::<u32>(w(0x98), c.wrapping_add(1));
                put(g, 0xa0, x);
                if c.cast_signed() < 0xc {
                    break c;
                }
                if gw(g, 0xae) == 1 {
                    g.write::<u32>(w(0xbc), 0x64);
                    state = 0x64;
                    let idx = img.word(w(0x63).wrapping_add(state)) & 0xff;
                    target = img.word(w(0x6a).wrapping_add(4 * idx)).wrapping_sub(f);
                    continue;
                }
                break stage_dir_0150(g, img, f, s);
            }
            // state 0
            0x33d => {
                let args = at(0x1c);
                g.write::<u32>(args, w(0x345));
                g.write::<u32>(args + 4, w(0x34d));
                let cf = Frame {
                    at: img.rel32_target(f + 0x352),
                    esp: s.wrapping_sub(4),
                    ecx: args,
                    ..*fr
                };
                crate::s5_misc::camera_tree(g, img, &cf);
                let node = g.read::<u32>(args);
                g.write::<u32>(node, w(0x35d));
                let h = gw(g, 0x36f);
                // call width: 2
                g.call(t(g, 0x362, 286), 0, &[h, 0x4416_0000]);
                let x = gw(g, 0x387);
                // call width: 2
                g.call(t(g, 0x37c, 211), 0, &[x, w(0x381)]);
                let x = gw(g, 0x39f);
                // call width: 2
                g.call(t(g, 0x394, 228), 0, &[x, 0xb9]);
                up(g, 0x3b0);
                // (then as state 1)
                target = 0x3b4;
            }
            // state 1
            0x3b4 => break stage_place_0150(g, img, f, s),
            // state 2
            0x572 => {
                let x = add(k(g, 0x57d), k(g, 0x585));
                put(g, 0x58d, x);
                if !(tm < k(g, 0x575) || tm.is_nan()) {
                    g.write::<u32>(w(0x59a), state + 1);
                }
                break idx;
            }
            // state 3 (then as 4)
            0x5af => {
                let eax = stage_light_0150(g, img, f, s, state);
                break stage_fade_0150(g, img, f, eax);
            }
            // state 4
            0x79a => break stage_fade_0150(g, img, f, idx),
            // state 6
            0x815 => {
                let x = add(k(g, 0x819), k(g, 0x821));
                g.write::<u8>(w(0x82d), 0);
                put(g, 0x83d, x);
                let z = add(k(g, 0x845), k(g, 0x84d));
                g.write::<u32>(w(0x853), ONE);
                put(g, 0x869, z);
                // call width: 3
                g.call(t(g, 0x826, 233), 0, &[w(0x861), w(0x85c), 0x3f40_0000]);
                let x = sub(k(g, 0x879), k(g, 0x881));
                let h = gw(g, 0x89a);
                // call width: 2
                let mut eax = g.call(t(g, 0x886, 285), 0, &[h, x.to_bits()]).eax;
                let m = k(g, 0x8ac);
                let gl = mul(k(g, 0x8a4), m);
                let tm2 = k(g, 0x8b4);
                put(g, 0x8ca, gl);
                let (mut tt, mut mm) = (tm2, m);
                if !(tm2 < k(g, 0x8c2) || tm2.is_nan()) {
                    eax = stage_flash_0150(g, img, f, s, 0);
                    tt = k(g, 0x9cf);
                    mm = k(g, 0x9c7);
                }
                let end = stage_end_0150(
                    g,
                    img,
                    f,
                    (tt, mm),
                    [0x9f9, 0xa2a, 0xa4b],
                    [0xa03, 0xa0b, 0xa1b, 0xa23, 0xa34, 0xa3c, 0xa44],
                    at(0x1c),
                    eax,
                );
                break end;
            }
            // state 100 (then as 101)
            0xa54 => {
                // (its `eax` is the next state's first call's)
                stage_kind_0150(g, img, f, s);
                target = 0xddc;
            }
            // state 101
            0xddc => {
                let x = sub(k(g, 0xde0), k(g, 0xde8));
                g.write::<u8>(w(0xdf4), 0);
                put(g, 0xe04, x);
                let z = sub(k(g, 0xe0c), k(g, 0xe14));
                g.write::<u32>(w(0xe1a), ONE);
                put(g, 0xe30, z);
                // call width: 3
                let mut eax = g
                    .call(t(g, 0xded, 233), 0, &[w(0xe28), w(0xe23), 0x3f73_3333])
                    .eax;
                let (mut tt, mut mm) = (k(g, 0xe40), k(g, 0xe48));
                if !(tt < k(g, 0xe52) || tt.is_nan()) {
                    eax = stage_flash_0150(g, img, f, s, 0x588);
                    tt = k(g, 0xf57);
                    mm = k(g, 0xf4f);
                }
                let end = stage_end_0150(
                    g,
                    img,
                    f,
                    (tt, mm),
                    [0xf81, 0xfb2, 0xfd3],
                    [0xf8b, 0xf93, 0xfa3, 0xfab, 0xfbc, 0xfc4, 0xfcc],
                    at(0x1c),
                    eax,
                );
                break end;
            }
            _ => break idx,
        }
    };
    Ret::eax(eax)
}

/// [`stage_task_0150`]'s state 5 past its count (`*G_Q` not 1): the
/// summon's direction `D` (slot 232 into `S + 0x24`), the node joins its
/// list ([`node_append`](crate::sb_misc::node_append)), the place copied
/// (slot 325), a word, a sound (slot 286 `(h, 600)`), flags and words,
/// the place again (325) moved back along `D` by `K` and its height
/// lowered; slot 520's mode moves it once more (0x24a0000 and 0x2470000
/// by their factors), `*G_Q` 2 lowers the height again; the step (232)
/// times 20 (233); the state on and five words. Returns slot 233's
/// `eax`.
fn stage_dir_0150<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let d = s + 0x24;
    // call width: 2
    let eax = g.call(t(g, 0xcf, 232), 0, &[d, w(0xc9)]).eax;
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xe6), w(0xe1), w(0xdc), eax);
    // call width: 2
    g.call(t(g, 0xeb, 325), 0, &[w(0xfb), w(0xf0)]);
    let x = gw(g, 0x105);
    g.write::<u32>(w(0x115), x);
    let h = gw(g, 0x128);
    // call width: 2
    g.call(t(g, 0x10a, 286), 0, &[h, 0x4416_0000]);
    let b = g.read::<u8>(w(0x135)) | 1;
    g.write::<u8>(w(0x135), b);
    g.write::<u32>(w(0x13c), w(0x140));
    g.write::<u32>(w(0x146), ONE);
    g.write::<u32>(w(0x150), w(0x154));
    // call width: 2
    g.call(t(g, 0x12f, 325), 0, &[w(0x164), w(0x15f)]);
    let (nx, nz) = (g.read::<f32>(d), g.read::<f32>(d + 8));
    let x = sub(k(g, 0x17c), mul(nx, k(g, 0x174)));
    put(g, 0x19e, x);
    let z = sub(k(g, 0x1a6), mul(nz, k(g, 0x193)));
    put(g, 0x1b2, z);
    let y = sub(k(g, 0x1ba), k(g, 0x1c2));
    put(g, 0x1ca, y);
    // call width: 0
    let mode = g.call(t(g, 0x181, 520), 0, &[]).eax;
    let y = if mode == 0x24a_0000 {
        let k2 = k(g, 0x1da);
        let x = sub(k(g, 0x1f6), mul(nx, k2));
        put(g, 0x210, x);
        let z = sub(k(g, 0x218), mul(nz, k2));
        put(g, 0x224, z);
        let y = sub(k(g, 0x22c), k(g, 0x234));
        put(g, 0x29c, y);
        y
    } else if mode == 0x247_0000 {
        let x = sub(k(g, 0x256), mul(nx, k(g, 0x24e)));
        put(g, 0x270, x);
        let z = sub(k(g, 0x278), mul(nz, k(g, 0x268)));
        put(g, 0x284, z);
        let y = sub(k(g, 0x28c), k(g, 0x294));
        put(g, 0x29c, y);
        y
    } else {
        k(g, 0x23e)
    };
    if gw(g, 0x2a2) == 2 {
        put(g, 0x2b5, sub(y, k(g, 0x2ad)));
    }
    // call width: 2
    g.call(t(g, 0x2ba, 232), 0, &[w(0x2ca), w(0x2bf)]);
    // call width: 3
    let eax = g
        .call(t(g, 0x2d1, 233), 0, &[w(0x2eb), w(0x2e6), 0x41a0_0000])
        .eax;
    let st = gw(g, 0x2f6).wrapping_add(1);
    g.write::<u32>(w(0x2f6), st);
    for (o, x) in [
        (0x2fcu32, 0x3ea0_d97cu32),
        (0x306, ONE),
        (0x310, 0x41f0_0000),
        (0x31a, 0x3f00_0000),
        (0x324, 0x3f00_0000),
    ] {
        g.write::<u32>(w(o), x);
    }
    eax
}

/// [`stage_task_0150`]'s state 1: `d = K / *G_D`, the place (slot 229
/// of `(d K1, 0, d K2, 1)` through the matrix into the summon's place),
/// its height from slot 289 `(h)` less `K` (mode 0xd4 lowers it), the
/// offset `(d K3, 289's, d K4, 1)` at `S + 0x24` (slot 229 in place; the
/// second slot 289 of slot 294's answer, 1 for 0xff, its `y`), slot 255
/// unless the flag, slot 3; the state on. Returns slot 3's `eax`.
fn stage_place_0150<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let d = div(k(g, 0x3b8), k(g, 0x3c0));
    let b = g.read::<u8>(w(0x3c6)) | 1;
    g.write::<u8>(w(0x3c6), b);
    let v1 = s + 0x34;
    g.write::<f32>(v1, mul(d, k(g, 0x3d2)));
    g.write::<u32>(w(0x3f3), w(0x3f7));
    g.write::<f32>(s + 8, d);
    g.write::<u32>(v1 + 4, 0);
    g.write::<f32>(v1 + 8, mul(d, k(g, 0x3ed)));
    g.write::<u32>(v1 + 0xc, ONE);
    // call width: 3
    g.call(t(g, 0x3dc, 229), 0, &[w(0x407), w(0x402), v1]);
    let h = gw(g, 0x439);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x42a, 289), 0, &[h]).st0);
    g.write::<f32>(s + 0x1c, r);
    put(g, 0x45d, sub(r, k(g, 0x44d)));
    // call width: 0
    if g.call(t(g, 0x452, 520), 0, &[]).eax == 0xd4_0000 {
        put(g, 0x484, sub(k(g, 0x474), k(g, 0x47c)));
    }
    let d = g.read::<f32>(s + 8);
    let v2 = s + 0x24;
    g.write::<f32>(v2, mul(d, k(g, 0x49a)));
    g.write::<u32>(v2 + 4, 0);
    g.write::<f32>(v2 + 8, mul(d, k(g, 0x4a2)));
    g.write::<u32>(v2 + 0xc, ONE);
    // call width: 3
    g.call(t(g, 0x4ad, 229), 0, &[v2, w(0x4a7), v2]);
    let h = gw(g, 0x4e5);
    // call width: 1
    let r = g.call(t(g, 0x4d6, 294), 0, &[h]).eax;
    let out = if r == 0xff {
        // call width: 1
        g.call(t(g, 0x4f4, 289), 0, &[1])
    } else {
        let h = gw(g, 0x500);
        // call width: 1
        let r = g.call(t(g, 0x4f4, 294), 0, &[h]).eax;
        // call width: 1
        g.call(t(g, 0x511, 289), 0, &[r])
    };
    g.write::<f32>(v2 + 4, st0_f32(out.st0));
    if g.read::<u8>(w(0x526)) & 1 == 0 {
        let x = gw(g, 0x539);
        // call width: 2
        g.call(t(g, 0x533, 255), 0, &[x, v2]);
    }
    let x = gw(g, 0x54f);
    // call width: 1
    let eax = g.call(t(g, 0x549, 3), 0, &[x]).eax;
    let st = gw(g, 0x55d).wrapping_add(1);
    g.write::<u32>(w(0x55d), st);
    eax
}

/// [`stage_task_0150`]'s state 3: the light set up (the colour terms `K -
/// c` of eight words, four and three halves, the scale `K' / *G_D K''`,
/// the flags cleared, `A` 0x1c78, `B` 0x400, the fade `K4`, `C` 0, the
/// state on), slot 531 and slot 255 `(h, S + 0x34)` each unless its flag
/// word's bit 0. Returns `eax` as the original leaves it (the flag word
/// in its low half over the last call's).
fn stage_light_0150<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, state: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let k1 = k(g, 0x5b3);
    let scale = mul(div(k(g, 0x5bb), k(g, 0x5c3)), k(g, 0x5d6));
    let x = sub(k1, k(g, 0x5ce));
    let flag = g.read::<u16>(w(0x5dc));
    let fl = gw(g, 0x5e2) & !1;
    g.write::<u32>(w(0x5e2), fl);
    put(g, 0x5eb, x);
    let x = sub(k1, k(g, 0x5f6));
    let b = g.read::<u8>(w(0x5fc)) & 0xfc;
    g.write::<u8>(w(0x5fc), b);
    put(g, 0x610, x);
    let x = sub(k1, k(g, 0x61b));
    for o in [0x621u32, 0x62b, 0x635, 0x63f] {
        g.write::<u32>(w(o), 0x3f00_0000);
    }
    put(g, 0x64b, x);
    let x = sub(k1, k(g, 0x656));
    for o in [0x65cu32, 0x666, 0x670] {
        g.write::<u32>(w(o), 0x3f00_0000);
    }
    g.write::<u32>(w(0x67a), 0x3ca3_d70a);
    put(g, 0x686, x);
    let x = sub(k1, k(g, 0x691));
    g.write::<u32>(w(0x697), 0);
    g.write::<u32>(w(0x6a1), w(0x6a5));
    g.write::<u32>(w(0x6ab), 0xc140_0000);
    put(g, 0x6b7, scale);
    put(g, 0x6bf, x);
    let x = sub(k1, k(g, 0x6ca));
    let x2 = sub(k1, k(g, 0x6d2));
    g.write::<u32>(w(0x6d8), 0x1c78);
    g.write::<u32>(w(0x6de), 0x400);
    put(g, 0x6e6, k(g, 0x4a));
    put(g, 0x6ee, x);
    let x = sub(k(g, 0x6f6), k(g, 0x6fe));
    put(g, 0x706, x2);
    g.write::<u32>(w(0x70c), 0);
    g.write::<u32>(w(0x716), state.wrapping_add(1));
    put(g, 0x71e, x);
    // `mov ax, [flags]`: the word over the index left in `eax`
    let mut eax = u32::from(flag);
    if flag & 1 == 0 {
        let h = gw(g, 0x732);
        // call width: 2
        let r = g.call(t(g, 0x72c, 531), 0, &[h, s + 0x34]).eax;
        let flag = g.read::<u16>(w(0x740));
        eax = (r & 0xffff_0000) | u32::from(flag);
        if flag & 1 == 0 {
            let h = gw(g, 0x771);
            // call width: 2
            eax = g.call(t(g, 0x76b, 255), 0, &[h, s + 0x34]).eax;
        }
    }
    eax
}

/// [`stage_task_0150`]'s state 4 (and 3's end): the fade less `K`, `B`
/// less 130, `A` less 100, the light's height less `K'`; once the timer
/// reaches `K''` the state on, the glow byte 0x40 and 0.9. Returns `eax`
/// as given.
fn stage_fade_0150<G: Game>(g: &mut G, img: &Image, f: u32, eax: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let tm = k(g, 0x39);
    let (b, a) = (
        gw(g, 0x56).wrapping_sub(0x82),
        gw(g, 0x50).wrapping_sub(0x64),
    );
    put(g, 0x7be, sub(k(g, 0x79e), k(g, 0x7a6)));
    put(g, 0x7e2, sub(k(g, 0x7c6), k(g, 0x7ce)));
    g.write::<u32>(w(0x7d4), b);
    g.write::<u32>(w(0x7da), a);
    if tm < k(g, 0x7b6) || tm.is_nan() {
        return eax;
    }
    let st = gw(g, 0x29).wrapping_add(1);
    g.write::<u8>(w(0x7ef), 0x40);
    g.write::<u32>(w(0x7f6), 0x3f66_6666);
    g.write::<u32>(w(0x800), st);
    eax
}

/// [`stage_task_0150`]'s state 100: the summon's direction `D` (232),
/// the node joins its list, the place copied (325), a sound (286 `(h,
/// 800)`), flags and words; five factors `p8, p14, p10, pc, p18` by the
/// summon's kind (`*G_K` 0x1088, 0x1089, 0x108c; else by slot 535 of the
/// byte: 0, 2 or another); the place copied again (325), moved along `D`
/// by `p14`, its height less `p10`; the side `(D.z, 0, -D.x, 0)` of the
/// direction again (232) by `p18` (233) added (248); the step (232)
/// scaled by `pc` into the move and by `p8` (233); a sound (285); the
/// state on and four words. Returns slot 285's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn stage_kind_0150<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    let d = s + 0x24;
    // call width: 2
    let eax = g.call(t(g, 0xa5f, 232), 0, &[d, w(0xa59)]).eax;
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xa76), w(0xa71), w(0xa6c), eax);
    // call width: 2
    g.call(t(g, 0xa7b, 325), 0, &[w(0xa8b), w(0xa80)]);
    let h = gw(g, 0xaa8);
    // call width: 2
    g.call(t(g, 0xa92, 286), 0, &[h, 0x4448_0000]);
    let kind = gw(g, 0xaaf);
    let b = g.read::<u8>(w(0xab5)) | 1;
    g.write::<u8>(w(0xab5), b);
    g.write::<u32>(w(0xabf), w(0xac3));
    g.write::<u32>(w(0xac9), ONE);
    g.write::<u32>(w(0xad3), w(0xad7));
    let ks = |g: &G, ops: [u32; 5]| ops.map(|o| k(g, o));
    let [p8, p14, p10, pc, p18] = match kind {
        0x1088 => ks(g, [0xbe6, 0xbf4, 0xc02, 0xc10, 0xc1e]),
        0x1089 => ks(g, [0xba4, 0xbb2, 0xbc0, 0xbce, 0xbdc]),
        0x108c => ks(g, [0xb8c, 0xb9a, 0xc02, 0xc10, 0xc1e]),
        _ => {
            let byte = u32::from(g.read::<u8>(w(0xaf9)));
            // call width: 1
            let r = g.call(t(g, 0xaff, 535), 0, &[byte]).eax;
            match r {
                0 => ks(g, [0xb0f, 0xb1d, 0xb82, 0xb67, 0xb75]),
                2 => ks(g, [0xb0f, 0xb1d, 0xb59, 0xb67, 0xb75]),
                _ => ks(g, [0xb0f, 0xb1d, 0xb38, 0xb38, 0xb4c]),
            }
        }
    };
    for (o, x) in [(8u32, p8), (0xc, pc), (0x10, p10), (0x14, p14), (0x18, p18)] {
        g.write::<f32>(s + o, x);
    }
    // call width: 2
    g.call(t(g, 0xc23, 325), 0, &[w(0xc33), w(0xc28)]);
    let (nx, nz) = (g.read::<f32>(d), g.read::<f32>(d + 8));
    put(g, 0xc66, add(mul(nx, p14), k(g, 0xc58)));
    put(g, 0xc82, add(mul(nz, p14), k(g, 0xc7a)));
    put(g, 0xc98, sub(k(g, 0xc8a), p10));
    // call width: 2
    g.call(t(g, 0xc5e, 232), 0, &[d, w(0xc50)]);
    let (nx, nz) = (g.read::<u32>(d), g.read::<f32>(d + 8));
    g.write::<f32>(d, nz);
    g.write::<u32>(d + 8, nx ^ gw(g, 0xcb3));
    g.write::<u32>(d + 4, 0);
    g.write::<u32>(d + 0xc, 0);
    // call width: 3
    g.call(t(g, 0xcd2, 233), 0, &[d, d, p18.to_bits()]);
    // call width: 3
    g.call(t(g, 0xcfd, 248), 0, &[w(0xd0d), w(0xd02), d]);
    // call width: 2
    g.call(t(g, 0xd14, 232), 0, &[w(0xd27), w(0xd22)]);
    put(g, 0xd4f, add(mul(k(g, 0xd31), pc), k(g, 0xd47)));
    put(g, 0xd6d, add(mul(k(g, 0xd57), pc), k(g, 0xd65)));
    // call width: 3
    g.call(t(g, 0xd3c, 233), 0, &[w(0xd88), w(0xd83), p8.to_bits()]);
    let h = gw(g, 0xda5);
    // call width: 2
    let eax = g.call(t(g, 0xd8f, 285), 0, &[h, 0x3e71_463b]).eax;
    let st = gw(g, 0xdb0).wrapping_add(1);
    g.write::<u32>(w(0xdb0), st);
    for (o, x) in [
        (0xdb6u32, ONE),
        (0xdc0, 0x4270_0000),
        (0xdca, 0x3f00_0000),
        (0xdd4, 0x3f00_0000),
    ] {
        g.write::<u32>(w(o), x);
    }
    eax
}

/// [`stage_task_0150`] (the `rel32`s: the node appends, the camera tree,
/// the move starts, the leave, `__security_check_cookie` five times).
pub const STAGE_TASK_0150: Sig = Sig {
    len: 4258,
    hash: 0x5b24_011f_10d3_bc4f,
    calls: &[
        (0xe6, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x335, Callee::Any),
        (0x352, Callee::Sig(&crate::s5_misc::CAMERA_TREE)),
        (0x56a, Callee::Any),
        (0x5a7, Callee::Any),
        (0x80d, Callee::Any),
        (0x948, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0xa76, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xed0, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0xfdf, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x100a, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0071`'s node sparks (0x10001430, 1585 bytes; thiscall, `ecx` =
/// the group `C`, through its thunk 0x10001b40; a /GS frame aligned to
/// 16, `S` = `esp` after the two pushes): as
/// [`node_sparks`](crate::s9_misc::node_sparks) with its own spawn and
/// counters. The walk ([`walk`](crate::nodes::walk), `ecx` the owner);
/// nothing more when it stops. With the owner `N`:
/// - while `K0 > N+0x50`: the wait `N+0x58` counts down; at 0 slot 324's
///   answer `r` sets the next wait `(r & 3) + 1` and a request of `(r >> 2
///   & 0x1f) + 0x10` sparks off `N`'s ring
///   ([`ring_take`](crate::m0070::ring_take)); each record `R`: `f` = slot
///   307 less `K`, the kind `R+0x24` (slot 324's low two bits 0: colour
///   0x80808080 and `R+0x30` 0, else 0x40808080 and the answer), the size
///   `R+0x20 = ((f K1 + K2) K3) K4 + K5` (by kind), alive, the speed from
///   three more slot 307 draws, slot 325 `(R, N+0x38)` and `y` less `K6`;
/// - else with no sparks counted (`N+0x54`) the node is released
///   ([`node_release`](crate::s6_misc::node_release)).
///
/// Then the draw as `node_sparks`' (the render state at `S+0x30`, its
/// matrix `S+0xa8`; the local matrix `S+0x68` turned in place by the
/// record's angle, slot 230 into the render matrix), the counter `N+0x54`
/// and `N+0x50` up by `K`. Returns the render state's `+0x14`, the
/// release's `eax` or the walk's.
#[allow(clippy::too_many_lines)] // the original's three phases, in order
pub fn node_sparks_0071<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let s0 = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0xe8);
    let p = s0.wrapping_sub(8);
    let cookie = g.read::<u32>(w(0xd)) ^ s0;
    g.write::<u32>(s0 + 0xe4, cookie);
    let (eax, owner) = crate::nodes::walk(g, fr.ecx, g.read::<u32>(fr.ecx), cookie);
    let Some(n) = owner else {
        return Ret::eax(eax);
    };
    let rf = |g: &G, o: u32| g.read::<f32>(n.wrapping_add(o));
    let draw = if k(g, 0x76) > rf(g, 0x50) {
        let wait = g.read::<u32>(n + 0x58);
        g.write::<u32>(n + 0x58, wait.wrapping_sub(1));
        if wait == 0 {
            // call width: 0
            let r = g.call(t(g, 0x9a, 324), 0, &[]).eax;
            g.write::<u32>(n + 0x58, (r & 3) + 1);
            let count = ((r.cast_signed() >> 2) & 0x1f) + 0x10;
            g.write::<u32>(p + 0x1c, n);
            g.write::<u16>(p + 0x20, count.cast_unsigned() as u16);
            crate::m0070::ring_take(g, p + 0x1c);
            g.write::<u32>(p + 0x18, n + 0x38);
            for j in 0..2u32 {
                let mut rec = g.read::<u32>(p + 0x28 + 4 * j);
                let cnt = g.read::<u16>(p + 0x22 + 2 * j);
                for _ in 0..cnt {
                    g.write::<u32>(rec + 0x10, 0);
                    // call width: 0
                    let fv = sub(st0_f32(g.call(t(g, 0x106, 307), 0, &[]).st0), k(g, 0x120));
                    // call width: 0
                    let r = g.call(t(g, 0x125, 324), 0, &[]).eax;
                    // `test al, 3; sete`: the low two bits clear
                    let kind = u32::from(r.trailing_zeros() >= 2);
                    g.write::<u16>(rec + 0x12, r as u16);
                    g.write::<u32>(rec + 0x24, kind);
                    let x = if kind != 0 {
                        g.write::<u32>(rec + 0x28, 0x8080_8080);
                        g.write::<u32>(rec + 0x30, 0);
                        add(mul(fv, k(g, 0x156)), k(g, 0x16c))
                    } else {
                        let x = add(mul(fv, k(g, 0x176)), k(g, 0x181));
                        g.write::<u32>(rec + 0x28, 0x4080_8080);
                        g.write::<u32>(rec + 0x30, r & 0xffff);
                        x
                    };
                    let x = mul(x, k(g, 0x193));
                    g.write::<u8>(rec + 0x34, 1);
                    let x = mul(x, k(g, 0x1a4));
                    g.write::<u32>(rec + 0x2c, 0);
                    g.write::<f32>(rec + 0x20, add(x, k(g, 0x1b3)));
                    // call width: 0
                    let v = sub(st0_f32(g.call(t(g, 0x198, 307), 0, &[]).st0), k(g, 0x1d2));
                    g.write::<f32>(rec + 0x14, sub(k(g, 0x1da), mul(v, k(g, 0x1e7))));
                    // call width: 0
                    let v = sub(st0_f32(g.call(t(g, 0x1df, 307), 0, &[]).st0), k(g, 0x20a));
                    g.write::<f32>(rec + 0x18, mul(v, k(g, 0x217)));
                    // call width: 0
                    let v = sub(st0_f32(g.call(t(g, 0x20f, 307), 0, &[]).st0), k(g, 0x236));
                    g.write::<f32>(rec + 0x1c, sub(k(g, 0x23e), mul(v, k(g, 0x24f))));
                    // call width: 2
                    g.call(t(g, 0x243, 325), 0, &[rec, n + 0x38]);
                    let y = sub(g.read::<f32>(rec + 4), k(g, 0x26e));
                    g.write::<f32>(rec + 4, y);
                    rec = rec.wrapping_add(0x40);
                }
            }
        }
        true
    } else {
        g.read::<u32>(n + 0x54) != 0
    };
    if !draw {
        g.write::<u32>(p + 8, n);
        let release = Frame {
            at: img.rel32_target(f + 0x619),
            esp: p.wrapping_sub(4),
            eax: 0,
            ecx: p + 8,
            edx: 0,
            xmm: [0; 4],
        };
        return Ret::eax(crate::s6_misc::node_release(g, img, &release));
    }
    // the draw
    let l = |g: &G, o: u32| g.read::<f32>(p + o);
    let m = p + 0xa8;
    g.write::<u32>(p + 0x4c, m);
    g.write::<u32>(p + 0x40, m);
    g.write::<u16>(p + 0x32, 0x80);
    let z = g.read::<u32>(w(0x2b8));
    g.write::<u32>(p + 0x44, z);
    for o in [
        0x6cu32, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x94, 0x38,
    ] {
        g.write::<u32>(p + o, 0);
    }
    g.write::<u8>(p + 0x48, 0);
    g.write::<u32>(n + 0x54, 0);
    g.write::<u32>(p + 0xc, n);
    let step_at = img.rel32_target(f + 0x535);
    for j in 0..2u32 {
        let mut rec = g.read::<u32>(n + 0x24 + 4 * j);
        let counts = n + 0x2c + 2 * j;
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(counts)) {
            if g.read::<u8>(rec + 0x34) != 0 && g.read::<u8>(rec + 0x2b) != 0 {
                // call width: 3
                g.call(t(g, 0x377, 229), 0, &[p + 0x98, w(0x371), rec]);
                let x3 = mul(g.read::<f32>(rec + 0x20), g.read::<f32>(rec + 0x2c));
                let tb = g.read::<u32>(w(0x38f));
                let (m1, m2, m3, m4, m6, m7) = (
                    l(g, 0x6c),
                    l(g, 0x70),
                    l(g, 0x74),
                    l(g, 0x78),
                    l(g, 0x80),
                    l(g, 0x84),
                );
                for o in [0x7cu32, 0x68, 0x90] {
                    g.write::<f32>(p + o, x3);
                }
                let a = g.read::<u32>(rec + 0x30);
                let sine = |g: &G, a: u32| g.read::<f32>(tb.wrapping_add(4 * ((a >> 4) & 0xfff)));
                let (sn, cs) = (sine(g, a), sine(g, a.wrapping_add(0x4000)));
                g.write::<f32>(p + 0x14, sn);
                let row = [
                    (0x68u32, add(mul(cs, x3), mul(sn, m4))),
                    (0x6c, add(mul(sn, x3), mul(cs, m1))),
                    (0x78, sub(mul(cs, m4), mul(x3, sn))),
                    (0x7c, sub(mul(cs, x3), mul(m1, sn))),
                    (0x70, add(mul(sn, m6), mul(cs, m2))),
                    (0x80, sub(mul(cs, m6), mul(m2, sn))),
                    (0x74, add(mul(sn, m7), mul(cs, m3))),
                    (0x84, sub(mul(cs, m7), mul(m3, sn))),
                ];
                for (o, v) in row {
                    g.write::<f32>(p + o, v);
                }
                // call width: 2
                g.call(t(g, 0x43b, 230), 0, &[p + 0xa8, p + 0x68]);
                let fade = g.read::<f32>(rec + 0x2c);
                let top = k(g, 0x4f2);
                if top > fade {
                    let v = add(fade, k(g, 0x502));
                    g.write::<f32>(rec + 0x2c, v);
                    if v > top {
                        g.write::<u32>(rec + 0x2c, 0x4080_0000);
                    }
                }
                let colour = g.read::<u32>(rec + 0x28);
                g.write::<u32>(p + 0x34, colour);
                let kind = g.read::<u32>(rec + 0x24);
                let q = g.read::<u32>(kind.wrapping_mul(4).wrapping_add(n + 0x48));
                g.write::<u32>(p.wrapping_sub(0xc), p + 0x30);
                let step = Frame {
                    at: step_at,
                    esp: p.wrapping_sub(0x10),
                    eax: 0,
                    ecx: rec + 0x10,
                    edx: q,
                    xmm: [0; 4],
                };
                let live = crate::s6_misc::motion_sequence_step(g, img, &step);
                g.write::<u8>(rec + 0x34, live as u8);
                let (vx, x0, vy, vz) = (
                    g.read::<f32>(rec + 0x14),
                    g.read::<f32>(rec),
                    g.read::<f32>(rec + 0x18),
                    g.read::<f32>(rec + 0x1c),
                );
                let b = g.read::<u8>(rec + 0x2b).wrapping_add(0xfc);
                g.write::<u8>(rec + 0x2b, b);
                g.write::<f32>(rec, add(x0, vx));
                let y = add(g.read::<f32>(rec + 4), vy);
                g.write::<f32>(rec + 0x14, mul(vx, k(g, 0x563)));
                g.write::<f32>(rec + 4, y);
                let zz = add(g.read::<f32>(rec + 8), vz);
                g.write::<f32>(rec + 0x18, sub(vy, k(g, 0x57b)));
                g.write::<f32>(rec + 8, zz);
                g.write::<f32>(rec + 0x1c, mul(vz, k(g, 0x596)));
                let c = g.read::<u32>(n + 0x54).wrapping_add(1);
                g.write::<u32>(n + 0x54, c);
            }
            i += 1;
            rec = rec.wrapping_add(0x40);
        }
    }
    let x = add(rf(g, 0x50), k(g, 0x5e0));
    let z = g.read::<u32>(p + 0x44);
    g.write::<u32>(w(0x5e9), z);
    g.write::<f32>(n + 0x50, x);
    Ret::eax(z)
}

/// [`node_sparks_0071`] (the `rel32`s: the ring take, the sequence step,
/// `__security_check_cookie` twice, the release).
pub const NODE_SPARKS_0071: Sig = Sig {
    len: 1585,
    hash: 0xfd77_4dae_5fce_76f7,
    calls: &[
        (0xca, Callee::Sig(&crate::m0070::RING_TAKE)),
        (0x535, Callee::Sig(&crate::s6_misc::MOTION_SEQUENCE_STEP)),
        (0x5fe, Callee::Any),
        (0x619, Callee::Sig(&crate::s6_misc::NODE_RELEASE)),
        (0x629, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0071`'s 12-byte thunk over [`node_sparks_0071`] (0x10001b40:
/// `ecx` = the first stack word, then a jump with the caller's stack).
pub fn jmp_node_sparks_0071<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let sf = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx,
        ..*fr
    };
    node_sparks_0071(g, img, &sf)
}

/// [`jmp_node_sparks_0071`].
pub const JMP_NODE_SPARKS_0071: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&NODE_SPARKS_0071))],
    imports: &[],
};

/// A node appended under the parent `P` as `magic_0042`'s scene inlines
/// it: the store run `run` (the node's pass halfword, child list, parent
/// and link), `+8` = the old last `last` (the `mov [G], eax` at operand
/// `lb`), then `P`'s first (the store at `eq`) when `last` is `P`
/// (operand `cmp`), else `last + 4` = the node (the immediate at `ll`).
fn append_0042<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    last: u32,
    run: (u32, u32),
    [lb, cmp, eq, ll]: [u32; 4],
) {
    let w = |o: u32| img.word(f + o);
    crate::r25::abs_stores(g, img, f, run.0, run.1);
    g.write::<u32>(w(lb), last);
    if last == w(cmp) {
        crate::r25::abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(ll));
    }
}

/// A node appended as [`append_0042`] when the old last is a known node
/// (its `+8` among the store run, the address compared from `ecx`): `P`'s
/// first (the store at `eq`) when `last` is `P` (operand `cmp`), else the
/// store at `ne` (`last + 4`).
fn append_known_0042<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    last: u32,
    run: (u32, u32),
    [cmp, eq, ne]: [u32; 3],
) {
    crate::r25::abs_stores(g, img, f, run.0, run.1);
    let at = if last == img.word(f + cmp) { eq } else { ne };
    crate::r25::abs_stores(g, img, f, at, at + 10);
}

/// `magic_0042`'s scene set-up (0x1000a610, 2637 bytes; a table entry,
/// cdecl `(task)`; an 8-aligned frame, `S` = `esp` after the push): the
/// task kept, the static nodes ([`static_nodes_init`](crate::sb_misc::static_nodes_init)),
/// slots 718 twice, the part (slot 719's handle, 711 under the lock word,
/// 710), slots 222 and 221; the scene's nodes under the DLL's parent `P`
/// one after the other (the store runs in the code; [`append_0042`]):
/// the summon's model node ([`model_node_init`](crate::sa_summon::model_node_init))
/// and its place (slot 262), a copy (325) lowered by `K`, a node placed
/// by slot 262, one given slot 10, a pair holding the place's words, the
/// model state ([`model_state`](crate::m0070::model_state)), the time
/// left ([`time_left`](crate::m0042::time_left)), a node set up with the
/// handle ([`node_init`](crate::m0042::node_init)), three more copying the
/// time, a copy (325), three with the scaled height; then the DLL's
/// start-up ([`start_up`](crate::m0042::start_up)) and the static link
/// ([`static_node_link_b`](crate::s4_small::static_node_link_b)).
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn scene_0042<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x14; push esi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let task = fr.arg(g, 0);
    g.write::<u32>(w(0xe), task);
    crate::sb_misc::static_nodes_init(g, img, call(0x13), task);
    g.write::<u32>(w(0x29), w(0x2d));
    // call width: 1
    g.call(t(g, 0x18, 718), 0, &[w(0x1d)]);
    // call width: 1
    g.call(t(g, 0x34, 718), 0, &[w(0x42)]);
    // call width: 3
    let h = g.call(t(g, 0x49, 719), 0, &[w(0x5b), 8, 0]).eax;
    let lock = gw(g, 0x63);
    let old = g.read::<u32>(lock);
    g.write::<u32>(w(0x6f), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x7a, 711), 0, &[h, w(0x74), 0]);
    let lock = gw(g, 0x8d);
    g.write::<u32>(lock, old);
    let x = gw(g, 0xad);
    // call width: 4
    g.call(t(g, 0x97, 710), 0, &[w(0xb2), x, w(0xa7), 0x10_0000]);
    // call width: 4
    g.call(t(g, 0xb9, 222), 0, &[w(0xca), 0, 0xb, 2]);
    // call width: 4
    g.call(t(g, 0xd1, 221), 0, &[w(0xe5), 0, 0x3e8, 2]);
    abs_stores(g, img, f, 0xef, 0x183);
    // the summon's model node
    let args = s + 8;
    for (i, v) in (0u32..).zip([w(0x187), w(0x18f), 0x14]) {
        g.write::<u32>(args + 4 * i, v);
    }
    g.write::<u8>(args + 0xc, 1);
    abs_stores(g, img, f, 0x1a0, 0x1aa);
    crate::sa_summon::model_node_init(g, img, call(0x1ab), args);
    let x = gw(g, 0x1c5);
    // call width: 4
    g.call(t(g, 0x1bf, 262), 0, &[x, w(0x1ba), w(0x1b5), w(0x1b0)]);
    // A: a node placed by a copy
    let last = gw(g, 0x1d2);
    abs_stores(g, img, f, 0x1d9, 0x1e3);
    append_0042(
        g,
        img,
        f,
        last,
        (0x1e3, 0x214),
        [0x215, 0x21a, 0x220, 0x22f],
    );
    let n = g.read::<u16>(w(0x23b)).wrapping_add(1);
    g.write::<u16>(w(0x23b), n);
    abs_stores(g, img, f, 0x23f, 0x253);
    // call width: 2
    g.call(t(g, 0x234, 325), 0, &[w(0x25f), w(0x25a)]);
    let y = sub(g.read::<f32>(w(0x269)), g.read::<f32>(w(0x271)));
    // B: a node at slot 262's place
    let last = gw(g, 0x276);
    abs_stores(g, img, f, 0x27d, 0x290);
    g.write::<f32>(w(0x294), y);
    append_0042(
        g,
        img,
        f,
        last,
        (0x298, 0x2b6),
        [0x2b7, 0x2bc, 0x2c2, 0x2d1],
    );
    let n = g.read::<u16>(w(0x2dd)).wrapping_add(1);
    g.write::<u16>(w(0x2dd), n);
    let x = gw(g, 0x2f2);
    abs_stores(g, img, f, 0x2f6, 0x30a);
    // call width: 4
    g.call(t(g, 0x2d6, 262), 0, &[x, w(0x2ec), w(0x2e7), w(0x2e2)]);
    // C: a node given slot 10
    let last = gw(g, 0x313);
    append_0042(
        g,
        img,
        f,
        last,
        (0x31a, 0x35f),
        [0x360, 0x365, 0x36b, 0x37a],
    );
    let n = g.read::<u16>(w(0x386)).wrapping_add(1);
    g.write::<u16>(w(0x386), n);
    let x = gw(g, 0x38e);
    abs_stores(g, img, f, 0x392, 0x3cb);
    // call width: 2
    g.call(t(g, 0x37f, 10), 0, &[x, 0]);
    // D and E: the place's words and the model state
    let last = gw(g, 0x3d1);
    append_0042(
        g,
        img,
        f,
        last,
        (0x3d8, 0x409),
        [0x40a, 0x40f, 0x415, 0x424],
    );
    let lo = g.read::<u64>(w(0x42c));
    let mut count = g.read::<u16>(w(0x432));
    g.write::<u64>(w(0x43a), lo);
    let hi = g.read::<u64>(w(0x442));
    let d = w(0x447);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x44d, 0x457);
    g.write::<u64>(w(0x45b), hi);
    append_known_0042(g, img, f, d, (0x45f, 0x49a), [0x49c, 0x4a2, 0x4ae]);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x4be, 0x4c8);
    g.write::<u16>(w(0x4ca), count);
    abs_stores(g, img, f, 0x4ce, 0x4d8);
    g.write::<u32>(args, w(0x4dc));
    g.write::<u32>(args + 4, w(0x4e4));
    crate::m0070::model_state(g, img, call(0x4e9), args);
    // F: the time left
    let last = gw(g, 0x4ee);
    append_0042(
        g,
        img,
        f,
        last,
        (0x4f2, 0x52d),
        [0x52e, 0x533, 0x539, 0x548],
    );
    let n = g.read::<u16>(w(0x54f)).wrapping_add(1);
    g.write::<u16>(w(0x54f), n);
    abs_stores(g, img, f, 0x557, 0x561);
    g.write::<u32>(args, w(0x565));
    g.write::<u32>(args + 4, w(0x56d));
    crate::m0042::time_left(g, img, call(0x572), args, s.wrapping_sub(4));
    // G: a node set up with the handle
    let last = gw(g, 0x577);
    append_0042(
        g,
        img,
        f,
        last,
        (0x57b, 0x5b6),
        [0x5b7, 0x5bc, 0x5c2, 0x5d1],
    );
    let hh = gw(g, 0x5d6);
    let n = g.read::<u16>(w(0x5dd)).wrapping_add(1);
    g.write::<u16>(w(0x5dd), n);
    abs_stores(g, img, f, 0x5e5, 0x61e);
    for (i, v) in (0u32..).zip([w(0x622), w(0x62a), hh, 0x6b]) {
        g.write::<u32>(args + 4 * i, v);
    }
    crate::m0042::node_init(g, img, call(0x63b), args);
    // H, I, J, K: the time copied, the lists
    let last = gw(g, 0x640);
    append_0042(
        g,
        img,
        f,
        last,
        (0x644, 0x67f),
        [0x680, 0x685, 0x68b, 0x69a],
    );
    let x = gw(g, 0x69f);
    let mut count = g.read::<u16>(w(0x6a6));
    g.write::<u32>(w(0x6ab), x);
    let x = gw(g, 0x6b0);
    g.write::<u32>(w(0x6b5), x);
    let x = gw(g, 0x6ba);
    g.write::<u32>(w(0x6bf), x);
    let hn = w(0x6c4);
    count = count.wrapping_add(1);
    append_known_0042(g, img, f, hn, (0x6ca, 0x70f), [0x710, 0x716, 0x722]);
    let i_n = w(0x72d);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x733, 0x762);
    append_known_0042(g, img, f, i_n, (0x762, 0x7a7), [0x7a8, 0x7ae, 0x7ba]);
    let j_n = w(0x7c5);
    count = count.wrapping_add(1);
    append_known_0042(g, img, f, j_n, (0x7cb, 0x824), [0x825, 0x82b, 0x837]);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x848, 0x852);
    g.write::<u16>(w(0x855), count);
    abs_stores(g, img, f, 0x859, 0x863);
    // call width: 2
    g.call(t(g, 0x842, 325), 0, &[w(0x86f), w(0x86a)]);
    // L, M, N: the scaled height
    let last = gw(g, 0x876);
    append_0042(
        g,
        img,
        f,
        last,
        (0x87d, 0x8ae),
        [0x8af, 0x8b4, 0x8ba, 0x8c9],
    );
    let x = mul(g.read::<f32>(w(0x8d1)), g.read::<f32>(w(0x8d9)));
    let mut count = g.read::<u16>(w(0x8df));
    let l_n = w(0x8e4);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x8ea, 0x8fe);
    g.write::<f32>(w(0x902), x);
    append_known_0042(g, img, f, l_n, (0x906, 0x955), [0x957, 0x95d, 0x969]);
    let m_n = w(0x974);
    count = count.wrapping_add(1);
    append_known_0042(g, img, f, m_n, (0x97a, 0x9dd), [0x9df, 0x9e5, 0x9f1]);
    count = count.wrapping_add(1);
    abs_stores(g, img, f, 0x9fd, 0xa07);
    g.write::<u16>(w(0xa09), count);
    abs_stores(g, img, f, 0xa0d, 0xa3c);
    // the start-up and the static link
    let sf = Frame {
        at: call(0xa3d),
        esp: s.wrapping_sub(4),
        ..*fr
    };
    crate::m0042::start_up(g, img, &sf);
    let lf = Frame {
        at: call(0xa42),
        esp: s.wrapping_sub(4),
        ..*fr
    };
    crate::s4_small::static_node_link_b(g, img, &lf);
    Ret::eax(0)
}

/// [`scene_0042`] (the `rel32`s: the static nodes, the model node, the
/// model state, the time left, the node set-up, the start-up, the static
/// link).
pub const SCENE_0042: Sig = Sig {
    len: 2637,
    hash: 0xf187_f675_a775_38ea,
    calls: &[
        (0x13, Callee::Sig(&crate::sb_misc::STATIC_NODES_INIT)),
        (0x1ab, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x4e9, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x572, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x63b, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0xa3d, Callee::Sig(&crate::m0042::START_UP)),
        (0xa42, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_B)),
    ],
    imports: &[],
};

/// `magic_0042`'s camera task (0x100085b0, 2692 bytes with its tables; a
/// task callback `(task)`, cdecl; an 8-aligned frame, `S` = `esp` after
/// the two pushes). The record `R = task[1]`: `R + 0x18` and `+0x1c`
/// grow by `K`, the count `R + 0x20` counts up; the state is the global
/// `*G_S` (a byte table up to 0x6f, then 10000 and the rest):
/// - 0: the camera words, the differences ([`global_differences`](crate::s7_misc::global_differences),
///   `edx` 0, `xmm2` `K`), on, `R + 0x20` 0; 1 and 111: waits on the
///   count; 2 and 21: the camera moves; 81: on;
/// - 10 (then as 11): slots 557/560 (the flag 4), the camera, slot 285
///   `(h, 0)`, the move's words; 11: the node appended
///   ([`node_append`](crate::sb_misc::node_append)), the camera's target
///   `R + 0x178` (returned), its height and depth by `R + 0x28`, on (10
///   two states on);
/// - 20: slots 285/286, the camera, on, byte 0x70; 60: the effect node
///   ([`effect_node_with`](crate::s3_nodes::effect_node_with)), flags,
///   slots 719/306, the node leaves ([`leave`](crate::sa_misc::leave)),
///   slots 285/286, the owned node ([`node_append_owned`](crate::sa_misc::node_append_owned)),
///   the camera, on; 61/62: flags and colour words, on; 70: the camera, on;
///   80: the move ([`move_start`](crate::sa_misc::move_start) on the
///   frame), the list unlinked ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   the camera, on; 90: the camera, flags, colours, on; 91: slot 285,
///   on; 92: the depth on, the fade byte from `R + 0x1c` past `K`, on;
///   100: the camera by `R + 0x28`, then as 101 with the shake (slots
///   307/250 through [`float_307`](crate::s4_misc::float_307) and
///   [`call_250`](crate::s4_small::call_250)) while the byte is above 2,
///   else slot 250 `(0, 0)` and on; 101: the count `R + 0x310` to 128,
///   on; 110: slots 286 and 250, the camera, on;
/// - 10000: the task node unlinked.
///
/// Returns `eax` as the original leaves it (the byte table's index, the
/// last call's, a truncated byte).
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn camera_task_0042<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let rf = |g: &G, o: u32| g.read::<f32>(r + o);
    let wf = |g: &mut G, o: u32, x: f32| g.write::<f32>(r + o, x);
    let wr = |g: &mut G, o: u32, x: u32| g.write::<u32>(r + o, x);
    // push ebp; and esp, -8; sub esp, 0x10; two pushes
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let x = add(rf(g, 0x18), k(g, 0x1a));
    let n = g.read::<u32>(r + 0x20).wrapping_add(1);
    g.write::<u32>(r + 0x20, n);
    let st = gw(g, 0x23);
    wf(g, 0x18, x);
    let x = add(rf(g, 0x1c), k(g, 0x38));
    wf(g, 0x1c, x);
    let set_st = |g: &mut G, o: u32, v: u32| g.write::<u32>(w(o), v);
    let up_st = |g: &mut G, o: u32| {
        let v = gw(g, o).wrapping_add(1);
        g.write::<u32>(w(o), v);
    };
    if st.cast_signed() > 0x2710 || (st != 0x2710 && st > 0x6f) {
        return Ret::eax(fr.eax);
    }
    if st == 0x2710 {
        let node = g.read::<u32>(task);
        let next = g.read::<u32>(node + 4);
        let prev = g.read::<u32>(node + 8);
        g.write::<u32>(next + 8, prev);
        let prev = g.read::<u32>(node + 8);
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, next);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        return Ret::eax(next);
    }
    let idx = img.word(w(0x5f).wrapping_add(st)) & 0xff;
    let target = img.word(w(0x6e).wrapping_add(4 * idx)).wrapping_sub(f);
    // the camera's target `*G_T`, height, depth (the state 11 tail)
    let tail_11 = |g: &mut G, x2: f32, st: u32| {
        let x0 = k(g, 0x36c);
        let d = sub(x2, rf(g, 0x28));
        g.write::<u32>(w(0x37a), r + 0x178);
        set_st(g, 0x386, st);
        wf(g, 0x33c, sub(x0, mul(d, k(g, 0x38e))));
        let d = sub(x2, rf(g, 0x28));
        wf(g, 0x340, add(mul(d, k(g, 0x3a7)), k(g, 0x3af)));
        let x = mul(rf(g, 0x28), k(g, 0x3c4));
        g.write::<u8>(r + 0x390, 0x55);
        wf(g, 0x28, x);
        // `lea eax, [esi + 0x178]` for the target's store
        r + 0x178
    };
    let append_11 = |g: &mut G, eax: u32| {
        crate::sb_misc::node_append(g, img, call(0x364), r + 0x2f0, r + 0x2c4, eax)
    };
    // the count `R + 0x310` to 128 (state 101)
    let count_101 = |g: &mut G, st: u32, eax: u32| {
        let c = g.read::<u32>(r + 0x310).wrapping_add(1);
        g.write::<u32>(r + 0x310, c);
        if c.cast_signed() >= 0x80 {
            set_st(g, 0x4e5, st.wrapping_add(1));
        }
        eax
    };
    let eax = match target {
        // state 0
        0x72 => {
            let x2 = k(g, 0x76);
            abs_stores(g, img, f, 0x7c, 0xe0);
            let df = Frame {
                at: call(0xe1),
                esp: s.wrapping_sub(4),
                eax: idx,
                edx: 0,
                xmm: [0, 0, u128::from(x2.to_bits()), 0],
                ..*fr
            };
            let eax = crate::s7_misc::global_differences(g, img, &df);
            set_st(g, 0xe8, st.wrapping_add(1));
            wr(g, 0x20, 0);
            eax
        }
        // state 1
        0xf5 => {
            if n >= 0x28 {
                set_st(g, 0x101, st.wrapping_add(1));
            }
            idx
        }
        // state 81
        0xfe => {
            set_st(g, 0x101, st.wrapping_add(1));
            idx
        }
        // state 2
        0x10b => {
            let x = sub(rf(g, 0x368), k(g, 0x117));
            wf(g, 0x368, x);
            idx
        }
        // state 90
        0x129 => {
            abs_stores(g, img, f, 0x129, 0x133);
            for (o, v) in [(0x364u32, 0), (0x368, 0x47af_c800), (0x36c, 0)] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x151, 0x15b);
            for (o, v) in [(0x338u32, 0), (0x33c, 0xc3fa_0000), (0x340, 0xc20c_0000)] {
                wr(g, o, v);
            }
            let fl = gw(g, 0x17b) | 1;
            g.write::<u32>(w(0x17b), fl);
            abs_stores(g, img, f, 0x181, 0x1e5);
            set_st(g, 0x1e7, st.wrapping_add(1));
            idx
        }
        // state 91
        0x1f1 => {
            abs_stores(g, img, f, 0x1f7, 0x201);
            wr(g, 0x1c, 0);
            let h = gw(g, 0x217);
            // call width: 2
            let eax = g.call(t(g, 0x1f2, 285), 0, &[h, 0x3ef1_463b]).eax;
            up_st(g, 0x222);
            wr(g, 0x394, 0x3f73_3333);
            eax
        }
        // state 92
        0x236 => {
            let x = add(rf(g, 0x33c), k(g, 0x242));
            let x2 = k(g, 0x24a);
            wf(g, 0x33c, x);
            let x1 = rf(g, 0x1c);
            if x2 > x1 {
                idx
            } else {
                let b = cvtt(mul(sub(x1, x2), k(g, 0x276))).cast_unsigned();
                g.write::<u8>(r + 0x390, b.to_le_bytes()[0]);
                if !(x1 < k(g, 0x267) || x1.is_nan()) {
                    set_st(g, 0x28d, st.wrapping_add(1));
                }
                b
            }
        }
        // state 10 (then as 11)
        0x297 => {
            // call width: 0
            let e = g.call(t(g, 0x298, 557), 0, &[]).eax;
            // call width: 1
            g.call(t(g, 0x2a9, 560), 0, &[e | 4]);
            abs_stores(g, img, f, 0x2ba, 0x2c4);
            for (o, v) in [(0x364u32, 0), (0x368, 0xc0a0_0000), (0x36c, 0)] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x2e2, 0x2ec);
            wr(g, 0x338, 0);
            abs_stores(g, img, f, 0x2f6, 0x300);
            let h = gw(g, 0x30f);
            // call width: 2
            let eax = g.call(t(g, 0x2b6, 285), 0, &[h, 0]).eax;
            let st = gw(g, 0x317);
            let x2 = k(g, 0x31f);
            for (o, v) in [
                (0x2e4u32, 0x1800u32),
                (0x28, ONE),
                (0x24, 0x41f0_0000),
                (0x1c, 0),
            ] {
                wr(g, o, v);
            }
            g.write::<u8>(r + 0x390, 1);
            wr(g, 0x394, ONE);
            append_11(g, eax);
            tail_11(g, x2, st.wrapping_add(2))
        }
        // state 11
        0x357 => {
            // (the table enters after state 10's `inc edi`)
            append_11(g, idx);
            tail_11(g, k(g, 0x67), st.wrapping_add(1))
        }
        // state 100 (then as 101)
        0x3da => {
            let x2 = k(g, 0x3de);
            let d = sub(x2, rf(g, 0x28));
            wf(g, 0x33c, sub(k(g, 0x3e6), mul(d, k(g, 0x3f6))));
            let d = sub(x2, rf(g, 0x28));
            wf(g, 0x340, add(mul(d, k(g, 0x40f)), k(g, 0x417)));
            let x = mul(rf(g, 0x28), k(g, 0x42c));
            wf(g, 0x28, x);
            let c = g.read::<u32>(r + 0x310).wrapping_add(1);
            g.write::<u32>(r + 0x310, c);
            if g.read::<u8>(r + 0x390) > 2 {
                let ff = Frame {
                    at: call(0x445),
                    esp: s.wrapping_sub(4),
                    ..*fr
                };
                let v = match crate::s4_misc::float_307(g, img, &ff).xmm0 {
                    Some(crate::Xmm::F32(x)) => f32::from_bits(x),
                    _ => 0.0,
                };
                let x = mul(sub(v, k(g, 0x44d)), rf(g, 0x24));
                let cf = Frame {
                    at: call(0x45a),
                    esp: s.wrapping_sub(4),
                    xmm: [u128::from(x.to_bits()), u128::from(x.to_bits()), 0, 0],
                    ..*fr
                };
                crate::s4_small::call_250(g, img, &cf);
                let b = cvt(u32::from(g.read::<u8>(r + 0x390)));
                let b = cvtt(mul(b, k(g, 0x470))).cast_unsigned();
                let x = mul(rf(g, 0x24), k(g, 0x481));
                g.write::<u8>(r + 0x390, b.to_le_bytes()[0]);
                wf(g, 0x24, x);
                b
            } else {
                // call width: 2
                let eax = g.call(t(g, 0x497, 250), 0, &[0, 0]).eax;
                let st = gw(g, 0x4b7).wrapping_add(1);
                g.write::<u8>(r + 0x390, 1);
                set_st(g, 0x4c8, st);
                count_101(g, st, eax)
            }
        }
        // state 101
        0x4cc => count_101(g, st, idx),
        // state 20
        0x4ef => {
            let h = gw(g, 0x504);
            // call width: 2
            g.call(t(g, 0x4f0, 285), 0, &[h, 0x3e20_d97c]);
            let h = gw(g, 0x521);
            // call width: 2
            let eax = g.call(t(g, 0x50b, 286), 0, &[h, 0x43c8_0000]).eax;
            abs_stores(g, img, f, 0x527, 0x531);
            for (o, v) in [(0x364u32, 0xc190_0000u32), (0x368, 0xc25c_0000), (0x36c, 0)] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x54f, 0x559);
            for (o, v) in [
                (0x338u32, 0x4190_0000u32),
                (0x33c, 0xc248_0000),
                (0x340, 0x4288_0000),
            ] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x577, 0x581);
            up_st(g, 0x586);
            g.write::<u8>(r + 0x390, 0x70);
            wr(g, 0x398, 1);
            eax
        }
        // state 21
        0x5a1 => {
            let x = sub(rf(g, 0x364), k(g, 0x5ad));
            wf(g, 0x364, x);
            let x = add(rf(g, 0x338), k(g, 0x5c5));
            wf(g, 0x338, x);
            idx
        }
        // state 60
        0x5d7 => {
            let sp = s.wrapping_sub(0x10);
            for (i, v) in (0u32..).zip([w(0x5e0), w(0x5db), n, 3]) {
                g.write::<u32>(sp + 4 * i, v);
            }
            let ef = Frame {
                at: call(0x5ea),
                esp: sp.wrapping_sub(4),
                ecx: w(0x5e5),
                ..*fr
            };
            crate::s3_nodes::effect_node_with(g, img, &ef);
            let fl = gw(g, 0x5ef) | 7;
            g.write::<u32>(w(0x5fa), fl);
            abs_stores(g, img, f, 0x605, 0x60f);
            // call width: 3
            let h = g.call(t(g, 0x5ff, 719), 0, &[w(0x618), 5, 0]).eax;
            // call width: 3
            let eax = g.call(t(g, 0x623, 306), 0, &[7, 2, h]).eax;
            crate::sa_misc::leave(g, r + 0x374, eax);
            let h = gw(g, 0x656);
            // call width: 2
            g.call(t(g, 0x642, 285), 0, &[h, 0]);
            let h = gw(g, 0x673);
            // call width: 2
            g.call(t(g, 0x65d, 286), 0, &[h, 0x4480_0000]);
            // the owned node: four words under the 286 call's
            let sp = s.wrapping_sub(0x18);
            for (i, v) in (0u32..).zip([w(0x68c), w(0x684), w(0x67f), w(0x67a)]) {
                g.write::<u32>(sp + 4 * i, v);
            }
            let of = Frame {
                at: call(0x696),
                esp: sp.wrapping_sub(4),
                ecx: r + 0x2c,
                edx: w(0x691),
                ..*fr
            };
            crate::sa_misc::node_append_owned(g, img, &of);
            g.write::<u32>(w(0x69c), r + 0x2c);
            for o in [0x364u32, 0x368, 0x36c] {
                wr(g, o, 0);
            }
            wr(g, 0x2bc, w(0x6c4));
            g.write::<u32>(w(0x6cf), r + 0x178);
            up_st(g, 0x6d8);
            for (o, v) in [
                (0x338u32, 0x4270_0000u32),
                (0x33c, 0xc2b4_0000),
                (0x340, 0xc248_0000),
            ] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x6fa, 0x704);
            // `lea eax, [esi + 0x178]` for the target's store
            r + 0x178
        }
        // states 61 and 62
        0x70a | 0x70b => {
            let fl = gw(g, 0x70d) | 3;
            g.write::<u32>(w(0x70d), fl);
            abs_stores(g, img, f, 0x713, 0x792);
            let d = if target == 0x70a { 2 } else { 1 };
            set_st(g, 0x794, st.wrapping_add(d));
            idx
        }
        // state 70
        0x79e => {
            for (o, v) in [
                (0x368u32, 0u32),
                (0x338, 0x41f0_0000),
                (0x33c, 0xc1a0_0000),
                (0x340, 0xc2a0_0000),
            ] {
                wr(g, o, v);
            }
            set_st(g, 0x7c9, st.wrapping_add(1));
            idx
        }
        // state 80
        0x7d3 => {
            let args = s + 8;
            abs_stores(g, img, f, 0x7d7, 0x827);
            for (i, v) in (0u32..).zip([w(0x82b), 0, 0x3da3_d70a, ONE]) {
                g.write::<u32>(args + 4 * i, v);
            }
            let eax = crate::sa_misc::move_start(g, args);
            let fl = gw(g, 0x84e) & !1;
            g.write::<u32>(w(0x84e), fl);
            let eax = crate::l3_common::list_unlink_current(g, img, call(0x854), eax);
            abs_stores(g, img, f, 0x858, 0x862);
            for (o, v) in [(0x364u32, 0u32), (0x368, 0xc2b4_0000), (0x36c, 0)] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x880, 0x88a);
            for (o, v) in [
                (0x338u32, 0x42a0_0000u32),
                (0x33c, 0x4248_0000),
                (0x340, 0xc2c8_0000),
            ] {
                wr(g, o, v);
            }
            set_st(g, 0x8ab, st.wrapping_add(1));
            eax
        }
        // state 110
        0x8b5 => {
            let h = gw(g, 0x8ca);
            // call width: 2
            g.call(t(g, 0x8b6, 286), 0, &[h, 0x4480_0000]);
            // call width: 2
            let eax = g.call(t(g, 0x8d1, 250), 0, &[0, 0]).eax;
            abs_stores(g, img, f, 0x8ec, 0x8f6);
            for (o, v) in [(0x364u32, 0u32), (0x368, 0xc28c_0000), (0x36c, 0)] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x914, 0x91e);
            for (o, v) in [
                (0x338u32, 0x4220_0000u32),
                (0x33c, 0xc2f0_0000),
                (0x340, 0xc248_0000),
            ] {
                wr(g, o, v);
            }
            abs_stores(g, img, f, 0x93c, 0x946);
            up_st(g, 0x94b);
            wr(g, 0x28, ONE);
            wr(g, 0x20, 0);
            eax
        }
        // state 111
        0x963 => {
            if n >= 0xf {
                let x = sub(mul(rf(g, 0x28), k(g, 0x971)), k(g, 0x979));
                wf(g, 0x340, x);
                let x = mul(rf(g, 0x28), k(g, 0x98e));
                wf(g, 0x28, x);
            }
            idx
        }
        _ => idx,
    };
    Ret::eax(eax)
}

/// [`camera_task_0042`] (the `rel32`s: the differences, the node append,
/// the slot 307 float, slot 250, the effect node, the leave, the owned
/// append, the move start, the list unlink).
pub const CAMERA_TASK_0042: Sig = Sig {
    len: 2692,
    hash: 0xdb1c_f51a_48f5_0a1a,
    calls: &[
        (0xe1, Callee::Sig(&crate::s7_misc::GLOBAL_DIFFERENCES)),
        (0x364, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x445, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x45a, Callee::Sig(&crate::s4_small::CALL_250)),
        (0x5ea, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x63d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x696, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x848, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x854, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
    ],
    imports: &[],
};

/// [`summon_task_0164`]'s (and `summon_task_0155`'s) state 100: the model node
/// ([`model_node_with`](crate::s4_misc::model_node_with)), the static
/// link, two appends, the model's place, the time left, the owned node,
/// the value 0xb8, the summon start ([`summon_start_0164`]), and slot 296
/// for each byte slot 288 lists but the DLL's own, the models' and those
/// of kinds 0x106a/0x10f8/0x10fa (slot 390). Then the state on.
///
/// `magic_0155`'s copy (`kind` 155) also links the static node
/// ([`static_node_link_b`](crate::s4_small::static_node_link_b)) after
/// the start, which moves the later operands 5 and then 10 bytes on, and
/// its models are 0x278 bytes apart.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_begin<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32, st: u32, kind: u16) {
    let f = fr.at;
    let late = |o: u32| match (kind, o) {
        (155, 0x108..0x146) => o + 5,
        (155, 0x146..) => o + 0xa,
        _ => o,
    };
    let w = |o: u32| img.word(f + late(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + late(o));
    let r = g.read::<u32>(s + 0x10);
    let x = gw(g, 0x5e);
    g.write::<u32>(s.wrapping_sub(8), x);
    g.write::<u32>(s.wrapping_sub(4), st);
    let mf = Frame {
        at: call(0x6d),
        esp: s.wrapping_sub(0xc),
        ecx: w(0x68),
        edx: w(0x63),
        ..*fr
    };
    crate::s4_misc::model_node_with(g, img, &mf);
    let lf = Frame {
        at: call(0x77),
        esp: s.wrapping_sub(0xc),
        edx: w(0x72),
        ..*fr
    };
    let eax = crate::s4_small::static_node_link(g, img, &lf).eax;
    crate::sb_misc::node_append(g, img, call(0x86), w(0x81), w(0x7c), eax);
    let eax = crate::sa_misc::model_position(g, img, call(0x95), w(0x90), w(0x8b));
    g.write::<u32>(w(0xa5), ONE);
    crate::sa_misc::node_append(g, img, call(0xae), w(0x9f), w(0x9a), eax);
    let rec = s + 0x1c;
    g.write::<u32>(rec, w(0xba));
    g.write::<u32>(rec + 4, w(0xc2));
    crate::m0042::time_left(g, img, call(0xc7), rec, s.wrapping_sub(0xc));
    let sp = s.wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([w(0xdb), w(0xd6), w(0xd1), w(0xcc)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let of = Frame {
        at: call(0xea),
        esp: sp.wrapping_sub(4),
        ecx: w(0xe5),
        edx: w(0xe0),
        ..*fr
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    crate::m0042::set_value(g, img, call(0xf9), w(0xf4), 0xb8);
    let sf = Frame {
        at: call(0x101),
        esp: s.wrapping_sub(0x10),
        ..*fr
    };
    if kind == 155 {
        summon_start_0155(g, img, &sf);
        let lf = Frame {
            at: img.rel32_target(f + 0x109),
            esp: s.wrapping_sub(4),
            ..*fr
        };
        crate::s4_small::static_node_link_b(g, img, &lf);
    } else {
        summon_start_0164(g, img, &sf);
    }
    let count_at = s + 0x14;
    let stride = if kind == 155 { 0x278 } else { 0x274 };
    // call width: 2
    let list = g.call(t(g, 0x10e, 288), 0, &[0x10, count_at]).eax;
    let count = g.read::<i32>(count_at);
    let mut i = 0i32;
    let mut bytes = list;
    while i < count {
        let b = u32::from(g.read::<u8>(bytes));
        let skip = b == gw(g, 0x135)
            || {
                let n = gw(g, 0x182).cast_signed();
                (0..n)
                    .any(|j| g.read::<u32>(w(0x142).wrapping_add(stride * j.cast_unsigned())) == b)
            }
            || {
                // call width: 1
                let kind = g.call(t(g, 0x155, 390), 0, &[b]).eax;
                matches!(kind, 0x106a | 0x10f8 | 0x10fa)
            };
        if !skip {
            let b = g.read::<u8>(bytes);
            let cf = Frame {
                at: call(0x17c),
                esp: s.wrapping_sub(4),
                ecx: u32::from(b),
                ..*fr
            };
            crate::s4_small::call_296(g, img, &cf);
        }
        i += 1;
        bytes = bytes.wrapping_add(1);
    }
    let st = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, st);
}

/// [`summon_task_0164`]'s state 101: two appends, slot 520's camera mode
/// into the turn, the owned node, two small nodes with the place's words,
/// the reset node ([`node_reset_sound`](crate::o6_fam_node::node_reset_sound))
/// and the light's words; the state on and the wait 0. Returns the reset's
/// `eax`.
fn summon_nodes_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) -> u32 {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let r = g.read::<u32>(s + 0x10);
    let edx = w(0x196);
    crate::sb_misc::node_append(g, img, call(0x1a0), w(0x19b), edx, 0);
    abs_stores(g, img, f, 0x1a9, 0x1d1);
    crate::sa_misc::node_append(g, img, call(0x1d2), w(0x1a5), edx, 0);
    abs_stores(g, img, f, 0x1db, 0x1e5);
    // call width: 0
    let mode = g.call(t(g, 0x1d7, 520), 0, &[]).eax;
    let turn = if mode == 0x24f_000a {
        0xffff_d000
    } else {
        0xffff_e000
    };
    g.write::<u32>(w(0x20b), turn);
    abs_stores(g, img, f, 0x223, 0x22d);
    let sp = s.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x215), w(0x210), w(0x205), w(0x1ee)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let parent = w(0x21a);
    let of = Frame {
        at: call(0x22e),
        esp: sp.wrapping_sub(4),
        ecx: w(0x21f),
        edx: parent,
        ..*fr
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    crate::m0070::node_init_small(g, img, call(0x23d), w(0x238), parent, w(0x233));
    for (from, to) in [(0x245u32, 0x24du32), (0x255, 0x260)] {
        let x = gw(g, from);
        g.write::<u32>(w(to), x);
    }
    let x = gw(g, 0x268);
    g.write::<u32>(w(0x27a), x);
    crate::m0070::node_init_small(g, img, call(0x27f), w(0x272), parent, w(0x26d));
    let rec = s + 0x14;
    abs_stores(g, img, f, 0x28a, 0x2a8);
    g.write::<u32>(rec, w(0x2ac));
    g.write::<u32>(rec + 4, parent);
    g.write::<u32>(rec + 8, 0x4416_0000);
    abs_stores(g, img, f, 0x2bc, 0x2c6);
    let eax = crate::o6_fam_node::node_reset_sound(g, img, call(0x2c7), rec);
    abs_stores(g, img, f, 0x2cb, 0x2f0);
    let st = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, st);
    g.write::<u32>(r + 0x38, 0);
    eax
}

/// `magic_0164`'s summon task (0x10009760, 4102 bytes with its tables; a
/// task callback `(task)`, cdecl; an 8-aligned /GS frame, `S` = `esp`
/// after the three pushes, `S + 0x10` = `R`). The record `R = task[1]`
/// keeps the state `R + 0x18`, floats `R + 0x1c..0x2c`, the counts
/// `R + 0x34`/`R + 0x38` (both up at every exit; `+0x38` the wait) and a
/// frame count `R + 0x3c`. By the state (a byte table up to 0xc9, then
/// 400..402, 410..412, 700, 10000):
/// - 0, 104 (past its wait), 150, 160, 180: on (some run the next);
/// - 100 then 101 then 102: [`summon_begin`], [`summon_nodes_0164`],
///   the wait of 16 and the light's start;
/// - 103: the light down; 105: the static node and the scene's colours
///   and camera words; 106: the camera shake (slot 307);
/// - 151, 152: the camera's words, its frames ([`frames_time`](crate::s4_small::frames_time));
///   161: on; 200/201: the camera's start by slot 520 and its fall;
///   181: the camera's climb;
/// - 400: the camera by slot 520; 401/402: the fade, slot 325 when the
///   flag; 410: the camera reset, slot 325, the frames; 411/412: the
///   light down until the wait of 40; 700: slots 325, 262, 12, 327, 229,
///   the camera by slot 520 (twice) and the frames;
/// - 10000: the node leaves, the list unlinks, the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn summon_task_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    // push ebp; and esp, -8; sub esp, 0x6c; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x6c);
    let cookie = gw(g, 0xa) ^ b0;
    g.write::<u32>(b0 + 0x68, cookie);
    let s = b0.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(s + 0x10, r);
    let rf = |g: &G, o: u32| g.read::<f32>(r + o);
    let wf = |g: &mut G, o: u32, x: f32| g.write::<f32>(r + o, x);
    let wr = |g: &mut G, o: u32, x: u32| g.write::<u32>(r + o, x);
    let rd = |g: &G, o: u32| g.read::<u32>(r + o);
    let up = |g: &mut G| {
        let v = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, v);
    };
    let st = rd(g, 0x18);
    // the camera's words `(x, y, 0)` at `S + 0x20` and their four copies
    let cam = |g: &mut G, (x, y): (f32, f32), ops: [u32; 6]| {
        g.write::<f32>(s + 0x20, x);
        g.write::<f32>(s + 0x24, y);
        g.write::<u32>(s + 0x28, 0);
        put(g, ops[0], x);
        put(g, ops[1], x);
        put(g, ops[2], y);
        put(g, ops[3], y);
        g.write::<u32>(w(ops[4]), 0);
        g.write::<u32>(w(ops[5]), 0);
    };
    // the fall of the camera's words by `R + 0x28`/`+0x2c`
    let fall = |g: &mut G, ops: [u32; 6]| {
        let x = sub(k(g, ops[0]), rf(g, 0x28));
        put(g, ops[1], x);
        let x = sub(k(g, ops[2]), rf(g, 0x2c));
        put(g, ops[3], x);
        let x = sub(k(g, ops[4]), rf(g, 0x2c));
        put(g, ops[5], x);
    };
    let frames = |g: &mut G, at: u32, edx: u32, eax: u32| {
        let ff = Frame {
            at: call(at),
            esp: s.wrapping_sub(4),
            eax,
            edx,
            ..*fr
        };
        crate::s4_small::frames_time(g, img, &ff).eax
    };
    let mut eax = cookie;
    let mut step = false;
    if st > 0x190 {
        match st {
            0x191 | 0x192 => {
                if st == 0x191 {
                    g.write::<u8>(w(0xc68), 0);
                    g.write::<u32>(w(0xc6f), 0x3f66_6666);
                    up(g);
                }
                let on = g.read::<u8>(w(0xc7c)) & 1 != 0;
                let x = add(k(g, 0xc85), k(g, 0xc8d));
                put(g, 0xc95, x);
                if on {
                    g.write::<u32>(w(0xca2), w(0xca6));
                    // call width: 2
                    eax = g.call(t(g, 0xc9c, 325), 0, &[w(0xcb6), w(0xcb1)]).eax;
                    g.write::<u32>(w(0xcc1), w(0xcc5));
                    step = true;
                }
            }
            0x19a => {
                g.write::<u8>(w(0xb4b), 0);
                g.write::<u32>(w(0xb51), w(0xb55));
                // call width: 2
                g.call(t(g, 0xb45, 325), 0, &[w(0xb65), w(0xb60)]);
                let x = sub(k(g, 0xb6f), k(g, 0xb77));
                abs_stores(g, img, f, 0xb82, 0xbdc);
                for o in [0x20u32, 0x24, 0x28] {
                    g.write::<u32>(s + o, 0);
                }
                abs_stores(g, img, f, 0xbf4, 0xc4e);
                put(g, 0xc52, x);
                g.write::<u16>(w(0xc58), 0);
                eax = frames(g, 0xc5d, 0, 0);
                step = true;
            }
            0x19b | 0x19c => {
                if st == 0x19b {
                    eax = st + 1;
                    wr(g, 0x38, 0);
                    wr(g, 0x18, eax);
                }
                let x = sub(k(g, 0xcdc), k(g, 0xce4));
                put(g, 0xcec, x);
                step = g.read::<i32>(r + 0x38) >= 0x28;
            }
            0x2710 => {
                let e = crate::sa_misc::leave(g, w(0xd2a), eax);
                crate::l3_common::list_unlink_current(g, img, call(0xd34), e);
                let node = g.read::<u32>(task);
                let next = g.read::<u32>(node + 4);
                let prev = g.read::<u32>(node + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(node + 8);
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                eax = next;
            }
            0x2bc => {
                g.write::<u8>(w(0xd5b), 0);
                abs_stores(g, img, f, 0xd60, 0xd6a);
                // call width: 2
                g.call(t(g, 0xd55, 325), 0, &[w(0xd76), w(0xd71)]);
                let h = gw(g, 0xd92);
                // call width: 6 (325's words stay under it)
                g.call(
                    t(g, 0xd8c, 262),
                    0,
                    &[h, s + 0x20, s + 0x24, s + 0x28, w(0xd76), w(0xd71)],
                );
                let h = gw(g, 0xda5);
                // call width: 1
                let y = st0_f32(g.call(t(g, 0xd9f, 12), 0, &[h]).st0);
                // `fstp [esp + 0x40]` five words down: over the place's `y`
                g.write::<f32>(s + 0x24, y);
                let x = sub(y, k(g, 0xdbc));
                put(g, 0xdd6, x);
                let m = s + 0x30;
                // call width: 2
                g.call(t(g, 0xdce, 327), 0, &[m, w(0xdc8)]);
                // call width: 3
                g.call(t(g, 0xded, 229), 0, &[w(0xdf2), m, w(0xde3)]);
                abs_stores(g, img, f, 0xe03, 0xe53);
                // call width: 0
                let mode = g.call(t(g, 0xdff, 520), 0, &[]).eax;
                match mode {
                    0x237_0000 => {
                        abs_stores(g, img, f, 0xeab, 0xec9);
                    }
                    0x238_0000 => {
                        abs_stores(g, img, f, 0xe9f, 0xea9);
                        abs_stores(g, img, f, 0xeb5, 0xec9);
                    }
                    _ => {
                        abs_stores(g, img, f, 0xe6c, 0xe80);
                        if mode == 0x24f_000a {
                            abs_stores(g, img, f, 0xe93, 0xe9d);
                        } else {
                            abs_stores(g, img, f, 0xe87, 0xe91);
                        }
                    }
                }
                // call width: 0
                let mode = g.call(t(g, 0xeca, 520), 0, &[]).eax;
                let edx = if mode == 0x238_0000 { 0x800 } else { 0x200 };
                eax = frames(g, 0xee8, edx, mode);
                step = true;
            }
            _ => {}
        }
    } else if st == 0x190 {
        abs_stores(g, img, f, 0x9f4, 0xa12);
        // call width: 0
        let mode = g.call(t(g, 0x9f0, 520), 0, &[]).eax;
        let x0 = if mode == 0x24b_0005 || mode == 0x24f_000a {
            let x = sub(k(g, 0xa52), k(g, 0xa5a));
            abs_stores(g, img, f, 0xa5e, 0xa72);
            x
        } else {
            let x = sub(k(g, 0xa2c), k(g, 0xa34));
            abs_stores(g, img, f, 0xa38, 0xa4c);
            x
        };
        let x1 = mul(mul(0.0, k(g, 0xa7b)), k(g, 0xa91));
        g.write::<u16>(w(0xa81), 0);
        put(g, 0xa89, x0);
        g.write::<u32>(w(0xa96), 0);
        g.write::<u32>(w(0xa9c), ONE);
        for o in [0xaa5u32, 0xaaa, 0xaaf] {
            g.write::<u32>(w(o), 0);
        }
        g.write::<u32>(w(0xab5), 0xc160_0000);
        g.write::<u32>(w(0xabe), 0);
        for o in [0x20u32, 0x24, 0x28] {
            g.write::<u32>(s + o, 0);
        }
        for o in [0xacfu32, 0xad4, 0xad9, 0xade, 0xae3, 0xae8] {
            g.write::<u32>(w(o), 0);
        }
        put(g, 0xaf0, x1);
        put(g, 0xaf8, x1);
        g.write::<u16>(w(0xafe), 0);
        g.write::<u32>(w(0xb03), 0);
        g.write::<u32>(w(0xb09), w(0xb0d));
        eax = 0;
        step = true;
    } else if st <= 0xc9 {
        let idx = img.word(w(0x45).wrapping_add(st)) & 0xff;
        let mut target = img.word(w(0x4c).wrapping_add(4 * idx)).wrapping_sub(f);
        eax = idx;
        loop {
            match target {
                // state 0
                0x50 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                }
                // states 100 and 101 (then 102)
                0x5b | 0x195 => {
                    if target == 0x5b {
                        summon_begin(g, img, fr, s, st, 164);
                    }
                    eax = summon_nodes_0164(g, img, fr, s);
                    target = 0x2fa;
                    continue;
                }
                // state 102
                0x2fa => {
                    if g.read::<i32>(r + 0x38) >= 0x10 {
                        let x = k(g, 0x308);
                        wf(g, 0x1c, x);
                        wr(g, 0x24, 0x4120_0000);
                        wr(g, 0x20, 0x4120_0000);
                        wr(g, 0x38, 0);
                        step = true;
                    }
                }
                // state 103
                0x32b => {
                    let x = sub(k(g, 0x32f), k(g, 0x337));
                    put(g, 0x33f, x);
                    let x = add(sub(rf(g, 0x1c), rf(g, 0x20)), rf(g, 0x24));
                    put(g, 0x356, x);
                    let late = g.read::<i32>(r + 0x38) >= 0x14;
                    let x = mul(rf(g, 0x24), k(g, 0x367));
                    wf(g, 0x24, x);
                    if late {
                        wr(g, 0x38, 0);
                        step = true;
                    }
                }
                // state 104
                0x382 => {
                    if g.read::<i32>(r + 0x38) >= 0xa {
                        eax = st + 1;
                        wr(g, 0x38, 0);
                        wr(g, 0x18, eax);
                    }
                }
                // state 105
                0x39e => {
                    g.write::<u32>(s.wrapping_sub(8), w(0x3a0));
                    g.write::<u32>(s.wrapping_sub(4), st);
                    let sf = Frame {
                        at: call(0x3aa),
                        esp: s.wrapping_sub(0xc),
                        ecx: w(0x3a5),
                        ..*fr
                    };
                    crate::s4_misc::static_node(g, img, &sf);
                    let fl = gw(g, 0x3af) & !2;
                    let x1 = sub(0.0, k(g, 0x3ba));
                    abs_stores(g, img, f, 0x3c4, 0x45a);
                    put(g, 0x45e, x1);
                    abs_stores(g, img, f, 0x462, 0x480);
                    eax = fl | 1;
                    g.write::<u32>(w(0x484), eax);
                    abs_stores(g, img, f, 0x488, 0x500);
                    wr(g, 0x1c, 0x4270_0000);
                    wr(g, 0x20, ONE);
                    let y = rf(g, 0x1c);
                    cam(g, (0.0, y), [0x52b, 0x535, 0x541, 0x549, 0x54f, 0x559]);
                    step = true;
                }
                // state 106
                0x566 => {
                    g.write::<u32>(s + 0x20, 0);
                    // call width: 0
                    let out = g.call(t(g, 0x567, 307), 0, &[]);
                    eax = out.eax;
                    let v = st0_f32(out.st0);
                    g.write::<f32>(s + 0x10, v);
                    let y = add(
                        mul(mul(sub(v, k(g, 0x589)), k(g, 0x597)), rf(g, 0x20)),
                        rf(g, 0x1c),
                    );
                    let x0 = g.read::<f32>(s + 0x20);
                    put(g, 0x5a9, x0);
                    put(g, 0x5b1, x0);
                    g.write::<u32>(s + 0x28, 0);
                    g.write::<u32>(w(0x5bf), 0);
                    g.write::<f32>(s + 0x24, y);
                    put(g, 0x5d1, y);
                    put(g, 0x5d9, y);
                    g.write::<u32>(w(0x5df), 0);
                    let x = mul(rf(g, 0x20), k(g, 0x5f0));
                    wf(g, 0x20, x);
                }
                // state 150 (then 151, 152)
                0x5fe => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    target = 0x604;
                    continue;
                }
                // state 151 (then 152)
                0x604 => {
                    g.write::<u8>(w(0x606), 1);
                    abs_stores(g, img, f, 0x60b, 0x683);
                    wr(g, 0x24, 0);
                    let y = rf(g, 0x24);
                    cam(g, (0.0, y), [0x6a7, 0x6b1, 0x6bd, 0x6c5, 0x6cb, 0x6d5]);
                    up(g);
                    wr(g, 0x3c, 0);
                    target = 0x6e7;
                    continue;
                }
                // state 152
                0x6e7 => {
                    let x = sub(k(g, 0x6eb), k(g, 0x6f3));
                    let v = gw(g, 0x6f9).wrapping_sub(0x14);
                    g.write::<u32>(w(0x6f9), v);
                    let v = gw(g, 0x700).wrapping_add(0x80);
                    g.write::<u32>(w(0x700), v);
                    put(g, 0x709, x);
                    let n = rd(g, 0x3c).wrapping_sub(0x10);
                    wr(g, 0x3c, n);
                    eax = frames(g, 0x715, n, eax);
                    let y = rf(g, 0x24);
                    cam(g, (0.0, y), [0x736, 0x740, 0x74c, 0x754, 0x75a, 0x764]);
                }
                // state 160
                0x771 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    step = true;
                }
                // state 200
                0x77c => {
                    abs_stores(g, img, f, 0x77c, 0x7ea);
                    wr(g, 0x3c, 0x3e8);
                    wr(g, 0x1c, 0x4348_0000);
                    wr(g, 0x20, 0x42a0_0000);
                    cam(g, (0.0, 100.0), [0x819, 0x823, 0x82d, 0x837, 0x841, 0x84b]);
                    let n = rd(g, 0x3c);
                    frames(g, 0x857, n, eax);
                    let x = sub(k(g, 0x85f), k(g, 0x867));
                    g.write::<u16>(w(0x86f), 0);
                    abs_stores(g, img, f, 0x878, 0x896);
                    put(g, 0x89a, x);
                    // call width: 0
                    eax = g.call(t(g, 0x874, 520), 0, &[]).eax;
                    let (y, a, b) = match eax {
                        0x24b_0005 => (0x8fe, 0x3f00_0000, 0x3e19_999a),
                        0x24f_000a => (0x8da, 0x3ecc_cccd, 0x3e19_999a),
                        _ => (0x8b6, 0x3f4c_cccd, 0),
                    };
                    abs_stores(g, img, f, y - 2, y + 8);
                    wr(g, 0x2c, a);
                    wr(g, 0x28, b);
                    wr(g, 0x38, 0);
                    step = true;
                }
                // state 201
                0x920 => fall(g, [0x924, 0x931, 0x939, 0x946, 0x94e, 0x95b]),
                // state 180 (then 181)
                0x964 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    target = 0x96a;
                    continue;
                }
                // state 181
                0x96a => {
                    let x0 = sub(rf(g, 0x20), k(g, 0x978));
                    let n = rd(g, 0x3c).wrapping_sub(0x10);
                    wr(g, 0x3c, n);
                    let x1 = add(rf(g, 0x1c), k(g, 0x984));
                    wf(g, 0x20, x0);
                    wf(g, 0x1c, x1);
                    g.write::<f32>(s + 0x20, x1);
                    let y = rf(g, 0x20);
                    g.write::<f32>(s + 0x24, y);
                    g.write::<u32>(s + 0x28, 0);
                    fall(g, [0x9a7, 0x9bc, 0x9c4, 0x9d1, 0x9d9, 0x9e6]);
                }
                // state 161
                0xcf6 => step = true,
                _ => {}
            }
            break;
        }
    }
    if step {
        up(g);
    }
    for o in [0x34u32, 0x38] {
        let v = rd(g, o).wrapping_add(1);
        wr(g, o, v);
    }
    Ret::eax(eax)
}

/// [`summon_task_0164`] (the `rel32`s: the model node, the static link,
/// the appends, the model's place, the time left, the owned appends, the
/// value, the summon start, slot 296, the small nodes, the reset, the
/// static node, the frames (four), `__security_check_cookie`, the leave
/// and the list unlink).
pub const SUMMON_TASK_0164: Sig = Sig {
    len: 4102,
    hash: 0x82ee_8156_2c2d_7bef,
    calls: &[
        (0x6d, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x77, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK)),
        (0x86, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x95, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (0xae, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xc7, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0xea, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xf9, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x101, Callee::Sig(&SUMMON_START_0164)),
        (0x17c, Callee::Sig(&crate::s4_small::CALL_296)),
        (0x1a0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1d2, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x22e, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x23d, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x27f, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (
            0x2c7,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0155_2FC0),
        ),
        (0x3aa, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x715, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0x857, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0xc5d, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0xd09, Callee::Any),
        (0xd2f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xd34, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xee8, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
    ],
    imports: &[],
};

/// `magic_0155`'s summon state 101 (as [`summon_nodes_0164`] without slot
/// 520: the turn's words are constants; the owned node takes the second
/// append's node, left in `ecx`, as its third word and the first append's
/// parent, left in `edx`, as its parent). Returns the reset's `eax`.
fn summon_nodes_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) -> u32 {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let r = g.read::<u32>(s + 0x10);
    let edx = w(0x1a0);
    crate::sb_misc::node_append(g, img, call(0x1aa), w(0x1a5), edx, 0);
    let ecx = w(0x1af);
    abs_stores(g, img, f, 0x1b3, 0x1db);
    crate::sa_misc::node_append(g, img, call(0x1dc), ecx, edx, 0);
    let sp = s.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x1ec), w(0x1e7), ecx, w(0x1e1)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x1f5, 0x213);
    let of = Frame {
        at: call(0x214),
        esp: sp.wrapping_sub(4),
        ecx: w(0x1f1),
        edx,
        ..*fr
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    crate::m0070::node_init_small(g, img, call(0x223), w(0x21e), edx, w(0x219));
    for (from, to) in [(0x22bu32, 0x233u32), (0x23b, 0x246)] {
        let x = gw(g, from);
        g.write::<u32>(w(to), x);
    }
    let x = gw(g, 0x24e);
    g.write::<u32>(w(0x260), x);
    crate::m0070::node_init_small(g, img, call(0x265), w(0x258), edx, w(0x253));
    let rec = s + 0x14;
    abs_stores(g, img, f, 0x270, 0x28e);
    g.write::<u32>(rec, w(0x292));
    g.write::<u32>(rec + 4, edx);
    g.write::<u32>(rec + 8, 0x4416_0000);
    abs_stores(g, img, f, 0x2a2, 0x2ac);
    let eax = crate::o6_fam_node::node_reset_sound(g, img, call(0x2ad), rec);
    abs_stores(g, img, f, 0x2b1, 0x2d6);
    let st = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, st);
    g.write::<u32>(r + 0x38, 0);
    eax
}

/// `magic_0155`'s summon task (0x10009ae0, 3746 bytes with its tables; a
/// task callback `(task)`, cdecl): [`summon_task_0164`]'s states with the
/// same byte table, these differing: 100 links the static node after the
/// start ([`summon_begin`] kind 155); 101 has fixed turn words
/// ([`summon_nodes_0155`]); 181 also runs 201; 200 sets the camera by slot
/// 520's answer less 0x16b0000 (65 or 80 high), its frames and the fade,
/// the wait 0; 201 steps two words (one down, one up by 0x48); 400 sets
/// its words with no slot call; 700 ends with the camera's place words.
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn summon_task_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let put = |g: &mut G, o: u32, x: f32| g.write::<f32>(w(o), x);
    // push ebp; and esp, -8; sub esp, 0x6c; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x6c);
    let cookie = gw(g, 0xa) ^ b0;
    g.write::<u32>(b0 + 0x68, cookie);
    let s = b0.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(s + 0x10, r);
    let rf = |g: &G, o: u32| g.read::<f32>(r + o);
    let wf = |g: &mut G, o: u32, x: f32| g.write::<f32>(r + o, x);
    let wr = |g: &mut G, o: u32, x: u32| g.write::<u32>(r + o, x);
    let rd = |g: &G, o: u32| g.read::<u32>(r + o);
    let up = |g: &mut G| {
        let v = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, v);
    };
    let st = rd(g, 0x18);
    let cam = |g: &mut G, (x, y): (f32, f32), ops: [u32; 6]| {
        g.write::<f32>(s + 0x20, x);
        g.write::<f32>(s + 0x24, y);
        g.write::<u32>(s + 0x28, 0);
        put(g, ops[0], x);
        put(g, ops[1], x);
        put(g, ops[2], y);
        put(g, ops[3], y);
        g.write::<u32>(w(ops[4]), 0);
        g.write::<u32>(w(ops[5]), 0);
    };
    let frames = |g: &mut G, at: u32, edx: u32, eax: u32| {
        let ff = Frame {
            at: call(at),
            esp: s.wrapping_sub(4),
            eax,
            edx,
            ..*fr
        };
        crate::s4_small::frames_time(g, img, &ff).eax
    };
    // state 201's two words (181 runs it too)
    let words_201 = |g: &mut G| {
        let v = gw(g, 0x92b).wrapping_sub(1);
        g.write::<u32>(w(0x92b), v);
        let v = gw(g, 0x931).wrapping_add(0x48);
        g.write::<u32>(w(0x931), v);
    };
    let mut eax = cookie;
    let mut step = false;
    if st > 0x190 {
        match st {
            0x191 | 0x192 => {
                if st == 0x191 {
                    g.write::<u8>(w(0xbb6), 0);
                    abs_stores(g, img, f, 0xbbb, 0xbc5);
                    up(g);
                }
                let on = g.read::<u8>(w(0xbca)) & 1 != 0;
                let x = add(k(g, 0xbd3), k(g, 0xbdb));
                put(g, 0xbe3, x);
                if on {
                    abs_stores(g, img, f, 0xbee, 0xbf8);
                    // call width: 2
                    eax = g.call(t(g, 0xbea, 325), 0, &[w(0xc04), w(0xbff)]).eax;
                    abs_stores(g, img, f, 0xc0d, 0xc17);
                    step = true;
                }
            }
            0x19a => {
                g.write::<u8>(w(0xa99), 0);
                abs_stores(g, img, f, 0xa9d, 0xaa7);
                // call width: 2
                g.call(t(g, 0xa93, 325), 0, &[w(0xab3), w(0xaae)]);
                let x = sub(k(g, 0xabd), k(g, 0xac5));
                abs_stores(g, img, f, 0xad0, 0xb2a);
                for o in [0x20u32, 0x24, 0x28] {
                    g.write::<u32>(s + o, 0);
                }
                abs_stores(g, img, f, 0xb42, 0xb9c);
                put(g, 0xba0, x);
                g.write::<u16>(w(0xba6), 0);
                eax = frames(g, 0xbab, 0, 0);
                step = true;
            }
            0x19b | 0x19c => {
                if st == 0x19b {
                    eax = st + 1;
                    wr(g, 0x38, 0);
                    wr(g, 0x18, eax);
                }
                let x = sub(k(g, 0xc2a), k(g, 0xc32));
                put(g, 0xc3a, x);
                step = g.read::<i32>(r + 0x38) >= 0x28;
            }
            0x2710 => {
                let e = crate::sa_misc::leave(g, w(0xc78), eax);
                crate::l3_common::list_unlink_current(g, img, call(0xc82), e);
                let node = g.read::<u32>(task);
                let next = g.read::<u32>(node + 4);
                let prev = g.read::<u32>(node + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(node + 8);
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                eax = next;
            }
            0x2bc => {
                g.write::<u8>(w(0xca9), 0);
                abs_stores(g, img, f, 0xcae, 0xcb8);
                // call width: 2
                g.call(t(g, 0xca3, 325), 0, &[w(0xcc4), w(0xcbf)]);
                let h = gw(g, 0xce0);
                // call width: 6 (325's words stay under it)
                g.call(
                    t(g, 0xcda, 262),
                    0,
                    &[h, s + 0x20, s + 0x24, s + 0x28, w(0xcc4), w(0xcbf)],
                );
                let h = gw(g, 0xcf3);
                // call width: 1
                let y = st0_f32(g.call(t(g, 0xced, 12), 0, &[h]).st0);
                // `fstp [esp + 0x40]` five words down: over the place's `y`
                g.write::<f32>(s + 0x24, y);
                let x = sub(y, k(g, 0xd0a));
                put(g, 0xd24, x);
                let m = s + 0x30;
                // call width: 2
                g.call(t(g, 0xd1c, 327), 0, &[m, w(0xd16)]);
                // call width: 3
                eax = g.call(t(g, 0xd3b, 229), 0, &[w(0xd40), m, w(0xd31)]).eax;
                abs_stores(g, img, f, 0xd4f, 0xd8b);
                step = true;
            }
            _ => {}
        }
    } else if st == 0x190 {
        let x0 = sub(k(g, 0x93f), k(g, 0x947));
        let x1 = mul(mul(0.0, k(g, 0x952)), k(g, 0x966));
        abs_stores(g, img, f, 0x958, 0x962);
        abs_stores(g, img, f, 0x96a, 0x9ba);
        for o in [0x20u32, 0x24, 0x28] {
            g.write::<u32>(s + o, 0);
        }
        abs_stores(g, img, f, 0x9d2, 0xa2c);
        put(g, 0xa30, x0);
        g.write::<u16>(w(0xa36), 0);
        put(g, 0xa3e, x1);
        put(g, 0xa46, x1);
        g.write::<u16>(w(0xa4c), 0);
        g.write::<u32>(w(0xa51), 0);
        abs_stores(g, img, f, 0xa55, 0xa5f);
        eax = 0;
        step = true;
    } else if st <= 0xc9 {
        let idx = img.word(w(0x45).wrapping_add(st)) & 0xff;
        let mut target = img.word(w(0x4c).wrapping_add(4 * idx)).wrapping_sub(f);
        eax = idx;
        loop {
            match target {
                // state 0
                0x50 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                }
                // states 100 and 101 (then 102)
                0x5b | 0x19f => {
                    if target == 0x5b {
                        summon_begin(g, img, fr, s, st, 155);
                    }
                    eax = summon_nodes_0155(g, img, fr, s);
                    target = 0x2e0;
                    continue;
                }
                // state 102
                0x2e0 => {
                    if g.read::<i32>(r + 0x38) >= 0x10 {
                        let x = k(g, 0x2ee);
                        wf(g, 0x1c, x);
                        wr(g, 0x24, 0x4120_0000);
                        wr(g, 0x20, 0x4120_0000);
                        wr(g, 0x38, 0);
                        step = true;
                    }
                }
                // state 103
                0x311 => {
                    let x = sub(k(g, 0x315), k(g, 0x31d));
                    put(g, 0x325, x);
                    let x = add(sub(rf(g, 0x1c), rf(g, 0x20)), rf(g, 0x24));
                    put(g, 0x33c, x);
                    let late = g.read::<i32>(r + 0x38) >= 0x14;
                    let x = mul(rf(g, 0x24), k(g, 0x34d));
                    wf(g, 0x24, x);
                    if late {
                        wr(g, 0x38, 0);
                        step = true;
                    }
                }
                // state 104
                0x368 => {
                    if g.read::<i32>(r + 0x38) >= 0xa {
                        eax = st + 1;
                        wr(g, 0x38, 0);
                        wr(g, 0x18, eax);
                    }
                }
                // state 105
                0x384 => {
                    g.write::<u32>(s.wrapping_sub(8), w(0x386));
                    g.write::<u32>(s.wrapping_sub(4), st);
                    let sf = Frame {
                        at: call(0x390),
                        esp: s.wrapping_sub(0xc),
                        ecx: w(0x38b),
                        ..*fr
                    };
                    crate::s4_misc::static_node(g, img, &sf);
                    let fl = gw(g, 0x395) & !2;
                    let x1 = sub(0.0, k(g, 0x3a0));
                    abs_stores(g, img, f, 0x3aa, 0x440);
                    put(g, 0x444, x1);
                    abs_stores(g, img, f, 0x448, 0x466);
                    eax = fl | 1;
                    g.write::<u32>(w(0x46a), eax);
                    abs_stores(g, img, f, 0x46e, 0x4e6);
                    wr(g, 0x1c, 0x4270_0000);
                    wr(g, 0x20, ONE);
                    let y = rf(g, 0x1c);
                    cam(g, (0.0, y), [0x511, 0x51b, 0x527, 0x52f, 0x535, 0x53f]);
                    step = true;
                }
                // state 106
                0x54c => {
                    g.write::<u32>(s + 0x20, 0);
                    // call width: 0
                    let out = g.call(t(g, 0x54d, 307), 0, &[]);
                    eax = out.eax;
                    let v = st0_f32(out.st0);
                    g.write::<f32>(s + 0x10, v);
                    let y = add(
                        mul(mul(sub(v, k(g, 0x56f)), k(g, 0x57d)), rf(g, 0x20)),
                        rf(g, 0x1c),
                    );
                    let x0 = g.read::<f32>(s + 0x20);
                    put(g, 0x58f, x0);
                    put(g, 0x597, x0);
                    g.write::<u32>(s + 0x28, 0);
                    g.write::<u32>(w(0x5a5), 0);
                    g.write::<f32>(s + 0x24, y);
                    put(g, 0x5b7, y);
                    put(g, 0x5bf, y);
                    g.write::<u32>(w(0x5c5), 0);
                    let x = mul(rf(g, 0x20), k(g, 0x5d6));
                    wf(g, 0x20, x);
                }
                // state 150 (then 151, 152)
                0x5e4 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    target = 0x5ea;
                    continue;
                }
                // state 151 (then 152)
                0x5ea => {
                    g.write::<u8>(w(0x5ec), 1);
                    abs_stores(g, img, f, 0x5f1, 0x669);
                    wr(g, 0x24, 0);
                    let y = rf(g, 0x24);
                    cam(g, (0.0, y), [0x68d, 0x697, 0x6a3, 0x6ab, 0x6b1, 0x6bb]);
                    up(g);
                    wr(g, 0x3c, 0);
                    target = 0x6cd;
                    continue;
                }
                // state 152
                0x6cd => {
                    let x = sub(k(g, 0x6d1), k(g, 0x6d9));
                    let v = gw(g, 0x6df).wrapping_sub(0x14);
                    g.write::<u32>(w(0x6df), v);
                    let v = gw(g, 0x6e6).wrapping_add(0x80);
                    g.write::<u32>(w(0x6e6), v);
                    put(g, 0x6ef, x);
                    let n = rd(g, 0x3c).wrapping_sub(0x10);
                    wr(g, 0x3c, n);
                    eax = frames(g, 0x6fb, n, eax);
                    let y = rf(g, 0x24);
                    cam(g, (0.0, y), [0x71c, 0x726, 0x732, 0x73a, 0x740, 0x74a]);
                }
                // state 160
                0x757 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    step = true;
                }
                // state 200
                0x762 => {
                    abs_stores(g, img, f, 0x767, 0x7c1);
                    // call width: 0
                    let mode = g
                        .call(t(g, 0x763, 520), 0, &[])
                        .eax
                        .wrapping_add(0xfe95_0000);
                    abs_stores(g, img, f, 0x7ce, 0x7ec);
                    if mode > 3 {
                        abs_stores(g, img, f, 0x7f1, 0x7fb);
                    }
                    wr(g, 0x3c, 0x3e8);
                    wr(g, 0x1c, 0x4348_0000);
                    wr(g, 0x20, 0x4220_0000);
                    cam(g, (0.0, 100.0), [0x82a, 0x834, 0x83e, 0x848, 0x852, 0x85c]);
                    let n = rd(g, 0x3c);
                    frames(g, 0x868, n, mode);
                    let x = sub(k(g, 0x870), k(g, 0x878));
                    abs_stores(g, img, f, 0x87e, 0x89c);
                    put(g, 0x8a0, x);
                    g.write::<u16>(w(0x8a6), 0);
                    wr(g, 0x38, 0);
                    eax = 0;
                    step = true;
                }
                // state 201
                0x929 => words_201(g),
                // state 180 (then 181)
                0x8b2 => {
                    eax = st + 1;
                    wr(g, 0x18, eax);
                    target = 0x8b8;
                    continue;
                }
                // state 181 (then 201's words)
                0x8b8 => {
                    let x0 = sub(rf(g, 0x20), k(g, 0x8c6));
                    let x1 = add(rf(g, 0x1c), k(g, 0x8ce));
                    let n = rd(g, 0x3c).wrapping_sub(0x10);
                    wr(g, 0x3c, n);
                    wf(g, 0x20, x0);
                    wf(g, 0x1c, x1);
                    g.write::<f32>(s + 0x20, x1);
                    let y = rf(g, 0x20);
                    g.write::<f32>(s + 0x24, y);
                    let x = sub(k(g, 0x8f5), k(g, 0x8fd));
                    g.write::<u32>(s + 0x28, 0);
                    put(g, 0x90d, x);
                    let x = sub(k(g, 0x915), k(g, 0x91d));
                    put(g, 0x925, x);
                    words_201(g);
                }
                // state 161
                0xc44 => step = true,
                _ => {}
            }
            break;
        }
    }
    if step {
        up(g);
    }
    for o in [0x34u32, 0x38] {
        let v = rd(g, o).wrapping_add(1);
        wr(g, o, v);
    }
    Ret::eax(eax)
}

/// [`summon_task_0155`] (the `rel32`s as [`SUMMON_TASK_0164`]'s with the
/// static link (b) after the summon start).
pub const SUMMON_TASK_0155: Sig = Sig {
    len: 3746,
    hash: 0x7064_c89f_0e76_038c,
    calls: &[
        (0x6d, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x77, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK)),
        (0x86, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x95, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (0xae, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xc7, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0xea, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xf9, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x101, Callee::Sig(&SUMMON_START_0155)),
        (0x109, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_B)),
        (0x186, Callee::Sig(&crate::s4_small::CALL_296)),
        (0x1aa, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1dc, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x214, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x223, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x265, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (
            0x2ad,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0155_2FC0),
        ),
        (0x390, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x6fb, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0x868, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0xbab, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0xc57, Callee::Any),
        (0xc7d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xc82, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
    ],
    imports: &[],
};
