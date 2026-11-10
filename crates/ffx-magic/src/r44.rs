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
