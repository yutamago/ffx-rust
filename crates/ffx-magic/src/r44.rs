//! Round 44's ports (own rows of the DLLs below `magic_0250`): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
use crate::r31::child_of;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
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
