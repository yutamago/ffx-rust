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
