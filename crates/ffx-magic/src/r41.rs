//! Round 41's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::r32::Shift;
use crate::r36::SparksLayout;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn put<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// A child's handler run on the frame cursor `cur` (the child, the pass
/// byte at `+5`); returns its `eax`.
fn handler<G: Game>(g: &mut G, cur: u32, child: u32, pass: u8) -> u32 {
    g.write::<u32>(cur, child);
    g.write::<u8>(cur + 5, pass);
    let m = g.read::<u32>(g.read::<u32>(child).wrapping_add(0x1c));
    g.call(m, cur, &[cur]).eax
}

/// `magic_0374`'s glow visit (0x10001290, internal: `ecx` = the group,
/// reached through [`glow_thunk_0374`]; an `ebp` /GS frame). The pass walk
/// ([`walk`](crate::nodes::walk)); when it ran, the owner `N`'s children
/// `+0x6c` and `+0x70` get their handlers on the frame cursor `ebp -
/// 0x4c`. With `N + 0x28` above `K`: slot 231 `(M, G, child2 + 0x9c)`
/// fills the matrix `M` at `ebp - 0x44`, slot 245 `(ebp - 0x48)`, and a
/// sprite record at `ebp - 0x80` (`M`, `child1 + 0x1c`, `N + 0x1c`, the
/// mode word 0x44, the halfword 0x100, the zoom `N + 0x5c`, the packet
/// `*G_P + 8`) becomes a GS packet ([`sprite_packet`](crate::s5_misc::sprite_packet));
/// slot 563 `(P, (end - P) >> 4, 0, 0)` sends it and `*G_P` = the end.
/// Then the zoom eases: with `N + 0x68` set, `z += (N[0x60] - z) *
/// N[0x54]`; else, while the clock `N + 0x58` is below `K'`, `z +=
/// N[0x64] * N[0x54]` and the clock steps by `N[0x54]`, and once the clock
/// was at `K'` or `N + 0x50` is, `z = N[0x60]`.
///
/// The sprite record's words the original never writes (`ebp - 0x64`, its
/// texture bit, and `ebp - 0x5c`, its texture number) are junk reads: the
/// port writes 0 there (brief 2026-10-24); the corners `sprite_packet`
/// takes from its own frame (the halfword's high byte is set) are 0 there.
///
/// Returns the last `eax`: the walk's, the last handler's or slot 563's,
/// or `N + 0x60` when it was copied.
pub fn glow_0374<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    let (mut eax, walked) = crate::nodes::walk(g, this, this, cookie);
    let Some(n) = walked else {
        return eax;
    };
    let pass = g.read::<u8>(this + 5);
    let c1 = g.read::<u32>(n + 0x6c);
    handler(g, at(0x4c), c1, pass);
    let c2 = g.read::<u32>(n + 0x70);
    eax = handler(g, at(0x4c), c2, pass);
    let k = f32_at(g, w(0x99));
    if f32_at(g, n + 0x28) > k {
        let c2 = g.read::<u32>(n + 0x70);
        // call width: 3
        let t = slot_target(g, w(0xb6), 231);
        g.call(t, 0, &[at(0x44), w(0xb0), c2 + 0x9c]);
        // call width: 1
        let t = slot_target(g, w(0xc7), 245);
        g.call(t, 0, &[at(0x48)]);
        let p = g.read::<u32>(w(0xd5));
        let zoom = g.read::<u32>(n + 0x5c);
        g.write::<u32>(at(0x80), at(0x44));
        let c1 = g.read::<u32>(n + 0x6c);
        g.write::<u32>(at(0x7c), c1 + 0x1c);
        g.write::<u32>(at(0x54), p + 8);
        g.write::<u32>(at(0x60), 0x44);
        g.write::<u16>(at(0x50), 0x100);
        g.write::<u32>(at(0x78), n + 0x1c);
        g.write::<u32>(at(0x58), zoom);
        // junk read: [ebp-0x64], [ebp-0x5c] never written; 0 (brief 2026-10-24)
        g.write::<u32>(at(0x64), 0);
        g.write::<u32>(at(0x5c), 0);
        crate::s5_misc::sprite_packet(g, img, img.rel32_target(f + 0x10f), at(0x80));
        let end = g.read::<u32>(at(0x54));
        // call width: 4
        let t = slot_target(g, w(0x123), 563);
        eax = g.call(t, 0, &[p, end.wrapping_sub(p) >> 4, 0, 0]).eax;
        g.write::<u32>(w(0x135), end);
    }
    if g.read::<u32>(n + 0x68) != 0 {
        let z = f32_at(g, n + 0x5c);
        let z = add(mul(sub(f32_at(g, n + 0x60), z), f32_at(g, n + 0x54)), z);
        put(g, n + 0x5c, z);
        return eax;
    }
    let clock = f32_at(g, n + 0x58);
    let k = f32_at(g, w(0x172));
    // comiss clock, K; jae: ordered and not below
    if clock < k || clock.is_nan() || k.is_nan() {
        let step = f32_at(g, n + 0x54);
        let z = add(mul(f32_at(g, n + 0x64), step), f32_at(g, n + 0x5c));
        put(g, n + 0x58, add(clock, step));
        put(g, n + 0x5c, z);
        // comiss N[0x50], K; jb: below or unordered keeps the zoom
        let e = f32_at(g, n + 0x50);
        if e < k || e.is_nan() || k.is_nan() {
            return eax;
        }
    }
    eax = g.read::<u32>(n + 0x60);
    g.write::<u32>(n + 0x5c, eax);
    eax
}

/// [`glow_0374`] (the `rel32`s: the sprite packet, the cookie checks).
pub const GLOW_0374: Sig = Sig {
    len: 442,
    hash: 0x7078_c8cf_6a85_a2fc,
    calls: &[
        (0x10f, Callee::Sig(&crate::s5_misc::SPRITE_PACKET)),
        (0x161, Callee::Any),
        (0x1b2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0374`'s thunk over [`glow_0374`] (0x10001460, 12 bytes: `ecx =
/// [ebp + 8]` and a tail jump; cdecl `(group)`).
pub fn glow_thunk_0374<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let grp = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    Ret::eax(glow_0374(g, img, target, grp, fr.esp))
}

/// [`glow_thunk_0374`].
pub const GLOW_THUNK_0374: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(0x8, Callee::Sig(&GLOW_0374))],
    imports: &[],
};

/// `magic_0374`'s sparks (0x10001a80, 1399 bytes): `magic_0295`'s
/// [`sparks_0295`](crate::r36::sparks_0295) drawing through slot 231 with
/// the global at operand `+0x425` pushed between its pointers (the
/// operands from `0x426` on 5 bytes later), and leaving with no sparks
/// out through [`leave`](crate::sa_misc::leave) on the node itself.
pub fn sparks_0374<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    const LAYOUT: SparksLayout = SparksLayout {
        shift: Shift {
            runs: &[(0x426, 5)],
            moved: &[(0x560, 0x55f)],
        },
        draw: 231,
        extra: Some(0x425),
        plain: true,
    };
    crate::r36::sparks_with(g, img, fr, &LAYOUT)
}

/// [`sparks_0374`].
pub const SPARKS_0374: Sig = Sig {
    len: 1399,
    hash: 0xb9dc_cf18_29b0_308b,
    calls: &[
        (0xe8, Callee::Sig(&crate::sa_misc::RING_RESERVE)),
        (0x49e, Callee::Sig(&crate::s7_misc::STREAM_BLOCK)),
        (0x54a, Callee::Any),
        (0x55f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x56f, Callee::Any),
    ],
    imports: &[],
};

/// `[w(a)] = w(v)` for each pair of operand offsets of `f` (a global
/// address and the word stored there), in order.
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, pairs: &[(u32, u32)]) {
    for &(a, v) in pairs {
        g.write::<u32>(img.word(f + a), img.word(f + v));
    }
}

/// `[w(a)] = v` for each operand offset `a` of `f` (a global) and value.
fn sets<G: Game>(g: &mut G, img: &Image, f: u32, pairs: &[(u32, u32)]) {
    for &(a, v) in pairs {
        g.write::<u32>(img.word(f + a), v);
    }
}

/// The frame words from `at` on.
fn words<G: Game>(g: &mut G, at: u32, ws: &[u32]) {
    for (i, &v) in (0u32..).zip(ws) {
        g.write::<u32>(at + 4 * i, v);
    }
}

/// Copies 16 bytes from `from` to `to` (two `movq` pairs).
fn copy16<G: Game>(g: &mut G, from: u32, to: u32) {
    for i in 0..4 {
        let v = g.read::<u32>(from + 4 * i);
        g.write::<u32>(to + 4 * i, v);
    }
}

/// The task's own node (`*t`) taken out of its ring; returns its
/// previous link as read last.
fn ring_out<G: Game>(g: &mut G, t: u32) -> u32 {
    let node = g.read::<u32>(t);
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev + 8, next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    prev
}

/// [`node_insert`](crate::nodes::node_insert) on the group at `grp`
/// (the `rel32` at `f + op`); returns its `eax`.
fn insert_at<G: Game>(g: &mut G, img: &Image, f: u32, op: u32, grp: u32) -> u32 {
    let fr = Frame {
        at: img.rel32_target(f + op),
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fr).eax
}

/// [`free_then_unlink`](crate::nodes::free_then_unlink) on the group at
/// `grp` (the `rel32` at `f + op`); returns its `eax`.
fn free_at<G: Game>(g: &mut G, img: &Image, f: u32, op: u32, grp: u32) -> u32 {
    let fr = Frame {
        at: img.rel32_target(f + op),
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::free_then_unlink(g, img, &fr).eax
}

/// An effect handle given back when its flag byte (at `flags`) lacks bit
/// 2: the game's current word `**G_CUR` = the word at `word` (operand
/// `cur` names `G_CUR`), then slot 713 `(arg)` (the table at operand
/// `table`). Returns slot 713's `eax`, or `eax` with the bit set.
fn give_back_713<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    flags: u32,
    (cur, word, table): (u32, u32, u32),
    arg: u32,
    eax: u32,
) -> u32 {
    if g.read::<u8>(flags) & 4 != 0 {
        return eax;
    }
    let p = g.read::<u32>(img.word(f + cur));
    let v = g.read::<u32>(word);
    g.write::<u32>(p, v);
    let t = slot_target(g, img.word(f + table), 713);
    // call width: 1
    g.call(t, 0, &[arg]).eax
}

/// `magic_0374`'s effect task (0x100079d0, 852 bytes; a task callback,
/// cdecl `(t)`, `R = *(t + 4)`; a frame aligned to 8, `L` its `esp`
/// after the pushes). Each call steps the floats `R + 0x1c` and `+0x20`
/// by `K`, then by the state `R + 0x18`:
/// - 0 (then 10): the light nodes appended ([`node_append`](crate::sb_misc::node_append))
///   and lit (three 1.0 and 0.01 words), the group `L + 8` inserted
///   ([`node_insert`](crate::nodes::node_insert); its `+0x14` the node
///   the append leaves in `ecx`), the particle record `L + 8` started
///   ([`particle_node`](crate::sb_nodes::particle_node)); the count `R +
///   0x28` 1, the index `R + 0x24` 0, state 1000;
/// - 1000: once the count reaches the index's mark (a table of words),
///   one of 7 rows (`i` the index, rows of 0x2c bytes): two light nodes
///   and a model node ([`node_append`](crate::sa_misc::node_append)) with
///   their 16-byte colours, the height `slot 705 - K`, the group inserted
///   and the row's particle started; after the 7th, state 1001;
/// - 1001: at the count 0x23, the main particle freed
///   ([`free_then_unlink`](crate::nodes::free_then_unlink)) and given
///   back (slot 713), state 10000;
/// - 10000: the task's node taken out of its ring.
///
/// The count `R + 0x28` steps on every call that does not reset it.
/// Returns the last callee's `eax`, the node's link (10000) or the state.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn effect_task_0374<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let l = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let at = |o: u32| l.wrapping_add(o);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    let x = add(f32_at(g, r + 0x1c), f32_at(g, w(0x19)));
    let state = g.read::<u32>(r + 0x18);
    put(g, r + 0x1c, x);
    let x = add(f32_at(g, r + 0x20), f32_at(g, w(0x2e)));
    put(g, r + 0x20, x);
    let step = |g: &mut G| {
        let c = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, c);
    };
    match state {
        0 | 0xa => {
            if state == 0 {
                g.write::<u32>(r + 0x18, 0xa);
            }
            let (list, node) = (w(0x61), w(0x66));
            crate::sb_misc::node_append(g, img, call(0x6b), node, list, state);
            // ecx after the append: the node
            g.write::<u32>(at(0x1c), node);
            sets(
                g,
                img,
                f,
                &[
                    (0x79, 0x3f80_0000),
                    (0x83, 0x3f80_0000),
                    (0x8d, 0x3f80_0000),
                    (0x97, 0x3c23_d70a),
                ],
            );
            words(g, at(8), &[w(0xa3), list, w(0xaf), w(0xb7), w(0xbf)]);
            insert_at(g, img, f, 0xc4, at(8));
            let h = g.read::<u32>(w(0xc9));
            words(g, at(8), &[w(0xd5), w(0xdd), w(0xe5), h, w(0xf1), 3]);
            stores(g, img, f, &[(0xff, 0x103)]);
            let eax = crate::sb_nodes::particle_node(g, img, call(0x108), at(8));
            g.write::<u32>(r + 0x28, 1);
            g.write::<u32>(r + 0x18, 0x3e8);
            g.write::<u32>(r + 0x24, 0);
            Ret::eax(eax)
        }
        0x3e8 => {
            let i = g.read::<u32>(r + 0x24);
            let c = g.read::<u32>(r + 0x28);
            if c < g.read::<u32>(w(0x132).wrapping_add(4 * i)) {
                step(g);
                return Ret::eax(c);
            }
            let list = w(0x146);
            let node = w(0x141).wrapping_add(i.wrapping_mul(0x2c));
            crate::sb_misc::node_append(g, img, call(0x14b), node, list, c);
            let i = g.read::<u32>(r + 0x24);
            copy16(
                g,
                w(0x15b).wrapping_add(i << 4),
                w(0x161).wrapping_add(i.wrapping_mul(0x2c)),
            );
            let i = g.read::<u32>(r + 0x24);
            let src = w(0x15b).wrapping_add(i << 4);
            let node = w(0x17f).wrapping_add(i.wrapping_mul(0x2c));
            crate::sb_misc::node_append(g, img, call(0x184), node, list, src);
            let i = g.read::<u32>(r + 0x24);
            copy16(
                g,
                w(0x194).wrapping_add(i << 4),
                w(0x19a).wrapping_add(i.wrapping_mul(0x2c)),
            );
            let tb = slot_target(g, w(0x1ac), 705);
            // call width: 0
            let y = crate::m0018::st0_f32(g.call(tb, 0, &[]).st0);
            let row = g.read::<u32>(r + 0x24).wrapping_mul(0x2c);
            put(g, w(0x1de).wrapping_add(row), sub(y, f32_at(g, w(0x1d1))));
            let row = g.read::<u32>(r + 0x24).wrapping_mul(0x2c);
            let parent = w(0x1d6);
            let node = w(0x1ea).wrapping_add(row);
            crate::sa_misc::node_append(g, img, call(0x1ef), node, parent, row);
            let i = g.read::<u32>(r + 0x24);
            copy16(
                g,
                w(0x1ff).wrapping_add(i << 4),
                w(0x205).wrapping_add(i.wrapping_mul(0x2c)),
            );
            let i = g.read::<u32>(r + 0x24);
            let row = i.wrapping_mul(0x2c);
            words(
                g,
                at(8),
                &[
                    w(0x22a).wrapping_add(i.wrapping_mul(0x14c)),
                    parent,
                    w(0x258),
                    w(0x234).wrapping_add(row),
                    w(0x23e).wrapping_add(row),
                    w(0x248).wrapping_add(row),
                ],
            );
            insert_at(g, img, f, 0x261, at(8));
            let i = g.read::<u32>(r + 0x24);
            let rec = w(0x277).wrapping_add(i << 8);
            let h = g.read::<u32>(w(0x285));
            let part = g.read::<u32>(w(0x2a4).wrapping_add(4 * i));
            words(
                g,
                at(8),
                &[
                    rec,
                    w(0x291),
                    w(0x27c).wrapping_add(i.wrapping_mul(0x14c)),
                    h,
                    w(0x29d),
                    part,
                ],
            );
            g.write::<u32>(rec, w(0x2b2));
            let eax = crate::sb_nodes::particle_node(g, img, call(0x2b7), at(8));
            let i = g.read::<u32>(r + 0x24).wrapping_add(1);
            g.write::<u32>(r + 0x24, i);
            if i.cast_signed() >= 7 {
                let s = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, s);
            }
            step(g);
            Ret::eax(eax)
        }
        0x3e9 => {
            if g.read::<i32>(r + 0x28) < 0x23 {
                step(g);
                return Ret::eax(state);
            }
            g.write::<u32>(at(4), w(0x311));
            let eax = free_at(g, img, f, 0x316, at(4));
            let ops = (0x324, w(0x32a), 0x331);
            let eax = give_back_713(g, img, f, w(0x31c), ops, w(0x336), eax);
            g.write::<u32>(r + 0x18, 0x2710);
            step(g);
            Ret::eax(eax)
        }
        0x2710 => {
            let eax = ring_out(g, t);
            step(g);
            Ret::eax(eax)
        }
        _ => {
            step(g);
            Ret::eax(state)
        }
    }
}

/// [`effect_task_0374`] (the `rel32`s: the nodes).
pub const EFFECT_TASK_0374: Sig = Sig {
    len: 852,
    hash: 0x20fb_b105_a21d_9f9d,
    calls: &[
        (0x6b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xc4, Callee::Sig(&crate::nodes::INSERT)),
        (0x108, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x14b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x184, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1ef, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x261, Callee::Sig(&crate::nodes::INSERT)),
        (0x2b7, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x316, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
    ],
    imports: &[],
};

/// `magic_0374`'s summon task (0x100074b0, 1304 bytes; a task callback,
/// cdecl `(t)`, `R = *(t + 4)`; a frame aligned to 8, `L` its `esp` after
/// the pushes). By the state `R + 0xf4`, with the count `R + 0xf8`:
/// - 0: the model node `R + 0x18` ([`model_node_init`](crate::sa_summon::model_node_init),
///   the group `L + 0x10` = `(R + 0x18, P, 0, 1)` and the node's vtable)
///   and the node `R + 0xb0` under it ([`node_init`](crate::m0003::node_init));
///   the flag byte `R + 0xf0` 1, the count 1, state 10;
/// - 10: from the count 4 on, state 11; 11: at the count 0x50 the light
///   nodes appended ([`node_append`](crate::sb_misc::node_append)) and
///   lit, slot 325 `(V, R + 0xcc)`, the height `slot 705 - K`, the group
///   inserted ([`node_insert`](crate::nodes::node_insert)) and the
///   particle started ([`particle_node`](crate::sb_nodes::particle_node)),
///   state 12;
/// - 12: once slot 705 is no more than `R + 0xd0`, the effect's 8
///   particles freed ([`free_then_unlink`](crate::nodes::free_then_unlink))
///   and given back (slot 713 unless a flag byte has bit 2), two light
///   nodes appended as the overlays inline it and lit, slot 325 again,
///   the glow `G = G_Y * K + G`, the group inserted and two particles
///   started (the second with its place and speed words); state 10000;
/// - 10000: the task's node taken out of its ring.
///
/// The count steps on every call that does not set it. Returns the last
/// callee's `eax`, the node's link (10000) or the state.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_task_0374<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let l = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let at = |o: u32| l.wrapping_add(o);
    let t = fr.arg(g, 0);
    let r = g.read::<u32>(t + 4);
    let state = g.read::<u32>(r + 0xf4);
    let step = |g: &mut G| {
        let c = g.read::<u32>(r + 0xf8).wrapping_add(1);
        g.write::<u32>(r + 0xf8, c);
    };
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let eax = match state {
        0 => {
            let node = r + 0x18;
            words(g, at(0x10), &[node, w(0x178), 0]);
            g.write::<u8>(at(0x1c), 1);
            g.write::<u32>(node, w(0x18b));
            crate::sa_summon::model_node_init(g, img, call(0x190), at(0x10));
            words(g, at(0x10), &[r + 0xb0, w(0x1a6), node]);
            let eax = crate::m0003::node_init(g, img, call(0x1af), at(0x10));
            g.write::<u8>(r + 0xf0, 1);
            g.write::<u32>(r + 0xf8, 1);
            g.write::<u32>(r + 0xf4, 0xa);
            return Ret::eax(eax);
        }
        0xa | 0xb => {
            if state == 0xa {
                if g.read::<u32>(r + 0xf8) < 4 {
                    step(g);
                    return Ret::eax(state);
                }
                g.write::<u32>(r + 0xf4, 0xb);
            }
            if g.read::<u32>(r + 0xf8) < 0x50 {
                step(g);
                return Ret::eax(0xb);
            }
            let list = w(0x56);
            let e = crate::sb_misc::node_append(g, img, call(0x60), w(0x5b), list, 0xb);
            sets(
                g,
                img,
                f,
                &[
                    (0x6b, 0x3f80_0000),
                    (0x75, 0x3f80_0000),
                    (0x7f, 0x3f80_0000),
                    (0x89, 0x3d75_c290),
                ],
            );
            crate::sb_misc::node_append(g, img, call(0x92), w(0x65), list, e);
            // call width: 2
            g.call(slot(g, 0x9e, 325), 0, &[w(0xa3), r + 0xcc]);
            // call width: 0
            let y = crate::m0018::st0_f32(g.call(slot(g, 0xb0, 705), 0, &[]).st0);
            put(g, w(0xf1), sub(y, f32_at(g, w(0xcd))));
            words(
                g,
                at(0x10),
                &[w(0xd9), w(0xe1), w(0xe9), w(0xf9), w(0x101), w(0x109)],
            );
            insert_at(g, img, f, 0x10e, at(0x10));
            let h = g.read::<u32>(w(0x113));
            words(g, at(0x10), &[w(0x11f), w(0x127), w(0x12f), h, w(0x13b), 2]);
            stores(g, img, f, &[(0x149, 0x14d)]);
            let eax = crate::sb_nodes::particle_node(g, img, call(0x152), at(0x10));
            g.write::<u32>(r + 0xf4, 0xc);
            eax
        }
        0xc => {
            // call width: 0
            let out = g.call(slot(g, 0x213, 705), 0, &[]);
            let y = crate::m0018::st0_f32(out.st0);
            // comiss y, R[0xd0]; ja: ordered and above
            if y > f32_at(g, r + 0xd0) {
                step(g);
                return Ret::eax(out.eax);
            }
            g.write::<u32>(at(0xc), w(0x23e));
            let eax = free_at(g, img, f, 0x243, at(0xc));
            let ops = (0x251, w(0x257), 0x25e);
            give_back_713(g, img, f, w(0x249), ops, w(0x263), eax);
            // the other 7 particles, 0x100 bytes apart (`e` = the record
            // + 0x50)
            for k in 0..7u32 {
                let e = w(0x273).wrapping_add(0x100 * k);
                g.write::<u32>(at(0xc), e.wrapping_sub(0x50));
                let eax = free_at(g, img, f, 0x28c, at(0xc));
                give_back_713(g, img, f, e + 0x98, (0x29a, e, 0x2a7), e + 0x10, eax);
            }
            // the first light node appended as the overlays inline it
            let last = g.read::<u32>(w(0x2c0));
            g.write::<u16>(w(0x2c7), 0xff);
            stores(
                g,
                img,
                f,
                &[
                    (0x2cf, 0x2d3),
                    (0x2d9, 0x2dd),
                    (0x2e3, 0x2e7),
                    (0x2ed, 0x2f1),
                ],
            );
            g.write::<u32>(w(0x2f6), last);
            if last == w(0x2fb) {
                stores(g, img, f, &[(0x303, 0x307)]);
            } else {
                g.write::<u32>(last + 4, w(0x310));
            }
            let count = g.read::<u16>(w(0x316)).wrapping_add(1);
            stores(g, img, f, &[(0x323, 0x327)]);
            sets(
                g,
                img,
                f,
                &[
                    (0x32d, 0x3f80_0000),
                    (0x337, 0x3f80_0000),
                    (0x341, 0x3f80_0000),
                    (0x34b, 0x3dcc_cccd),
                ],
            );
            // the second one
            g.write::<u16>(w(0x356), 0xff);
            stores(
                g,
                img,
                f,
                &[
                    (0x35e, 0x362),
                    (0x368, 0x36c),
                    (0x372, 0x376),
                    (0x37c, 0x380),
                    (0x386, 0x38a),
                ],
            );
            if w(0x31b) == w(0x390) {
                stores(g, img, f, &[(0x398, 0x39c)]);
            } else {
                stores(g, img, f, &[(0x3a4, 0x3a8)]);
            }
            g.write::<u16>(w(0x3b0), count.wrapping_add(1));
            stores(g, img, f, &[(0x3c2, 0x3c6), (0x3cc, 0x3d0)]);
            // call width: 2
            g.call(slot(g, 0x3bc, 325), 0, &[w(0x3db), r + 0xcc]);
            let x = mul(f32_at(g, w(0x3e5)), f32_at(g, w(0x3ed)));
            let x = add(x, f32_at(g, w(0x3fc)));
            put(g, w(0x424), x);
            words(
                g,
                at(0x10),
                &[w(0x404), w(0x40c), w(0x414), w(0x41c), w(0x42c), w(0x434)],
            );
            insert_at(g, img, f, 0x439, at(0x10));
            let h = g.read::<u32>(w(0x43e));
            words(g, at(0x10), &[w(0x44a), w(0x452), w(0x45a), h, w(0x466), 0]);
            stores(g, img, f, &[(0x474, 0x478)]);
            crate::sb_nodes::particle_node(g, img, call(0x47d), at(0x10));
            let h = g.read::<u32>(w(0x482));
            sets(
                g,
                img,
                f,
                &[
                    (0x48c, 0xc2cb_09da),
                    (0x496, 0xc30c_0000),
                    (0x4a0, 0x42ef_95a6),
                    (0x4aa, 0),
                    (0x4b4, 0xffff_7e00),
                    (0x4be, 0),
                ],
            );
            words(g, at(0x10), &[w(0x4ca), w(0x4d2), w(0x4da), h, w(0x4e6), 9]);
            stores(g, img, f, &[(0x4f4, 0x4f8)]);
            let eax = crate::sb_nodes::particle_node(g, img, call(0x4fd), at(0x10));
            g.write::<u32>(r + 0xf4, 0x2710);
            eax
        }
        0x2710 => ring_out(g, t),
        _ => state,
    };
    step(g);
    Ret::eax(eax)
}

/// [`summon_task_0374`] (the `rel32`s: the nodes).
pub const SUMMON_TASK_0374: Sig = Sig {
    len: 1304,
    hash: 0x1910_3417_93bf_5cd1,
    calls: &[
        (0x60, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x92, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x10e, Callee::Sig(&crate::nodes::INSERT)),
        (0x152, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x190, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x1af, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x243, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x28c, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x439, Callee::Sig(&crate::nodes::INSERT)),
        (0x47d, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x4fd, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A pool node for the task callback `cb` (operand `cb_op`; [`take`](crate::sa_pool::take)
/// at the `rel32` `call`) linked after `prev` (or first, the head global
/// at operand `head_op`, when the list `listed` was empty), its flag byte
/// `+0x18` set to `flag` when given. Returns the node.
#[allow(clippy::too_many_arguments)] // the original's operands
fn chain_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, cb_op, head_op): (u32, u32, u32),
    flag: Option<u8>,
    prev: Option<u32>,
) -> u32 {
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), img.word(f + cb_op));
    if let Some(b) = flag {
        g.write::<u8>(node + 0x18, b);
    }
    g.write::<u32>(node + 4, 0);
    if let Some(p) = prev {
        g.write::<u32>(node, p);
        g.write::<u32>(p + 4, node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(img.word(f + head_op), node);
    }
    node
}

/// `magic_0271`'s start-up (0x10001460, 835 bytes; internal, no
/// arguments, run by its load step [`load_0271`]; a /GS frame aligned to
/// 8, `E` = its `esp` after `sub esp, 0x34`): `*G_H` = slot 719 `(D, 8,
/// 0)` with four globals set to their immediates; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object started at
/// scale `K` ([`start_object_at`](crate::s4_misc::start_object_at),
/// index 0, owner 0x1e); the point `G_P` = slot 262 of the caster, `w`
/// 1.0; the yaw `G_Y = K - slot 333 (*G_A)` wrapped once and the callback
/// global; the caster's point `P1` (`E + 0x10`, `w` 1.0) and the target's
/// `P2` (`E + 0x20`, the table word of the target's byte, `w` 1.0) by
/// slot 262; `S + 0x1f24` = `K - atan2(P1.z - P2.z, P1.x - P2.x)` wrapped
/// once (`S` = `*G_S`), `S + 0x1f28..0x1f30` = 0x32, 8, 0, the fade
/// `S + 0x1e80` ([`fade_task`](crate::s3_misc::fade_task), `edx` 0x3c,
/// two constants, the word 0) with 0.7 at `+0x10`/`+0x18`; slot 271
/// `(4)`, slot 220 `(slot 719 (D', 6, 0), 0x40, 0x100)`, the block
/// ([`me_block`](crate::sa_misc::me_block)); three pool nodes chained on
/// the task list (flag bytes 1, -, 2), the last the list's tail.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0271<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    // the three register pushes
    let e1 = e.wrapping_sub(0xc);
    stores(
        g,
        img,
        f,
        &[(0x2d, 0x31), (0x37, 0x3b), (0x41, 0x45), (0x4b, 0x4f)],
    );
    // call width: 3
    let h = g.call(slot(g, 0x15, 719), 0, &[w(0x27), 8, 0]).eax;
    g.write::<u32>(w(0x59), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x5e));
    let at = img.rel32_target(f + 0x7a);
    crate::s4_misc::start_object_at(g, img, at, w(0x6e), k(g, 0x66), 0, 0x1e, 0);
    let chr = g.read::<u32>(w(0x9a));
    // call width: 4
    g.call(slot(g, 0x7f, 262), 0, &[chr, w(0x94), w(0x8f), w(0x84)]);
    let a = g.read::<u32>(w(0xa7));
    g.write::<u32>(w(0xad), 0x3f80_0000);
    // call width: 1
    let t = crate::m0018::st0_f32(g.call(slot(g, 0xa1, 333), 0, &[a]).st0);
    let yaw = crate::m0018::wrap(sub(k(g, 0xc5), t), k(g, 0xdd), k(g, 0xed), k(g, 0xd3));
    let s = g.read::<u32>(w(0xfc));
    let chr = g.read::<u32>(w(0x116));
    g.write::<f32>(w(0x11e), yaw);
    g.write::<u32>(w(0x124), w(0x128));
    let (p1, p2) = (e + 0x10, e + 0x20);
    // call width: 4
    g.call(slot(g, 0x110, 262), 0, &[chr, p1, p1 + 4, p1 + 8]);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x135))));
    g.write::<u32>(p1 + 0xc, 0x3f80_0000);
    let who = g.read::<u32>(w(0x156).wrapping_add(4 * b));
    // call width: 4
    g.call(slot(g, 0x15b, 262), 0, &[who, p2, p2 + 4, p2 + 8]);
    let dz = sub(f32_at(g, p1 + 8), f32_at(g, p2 + 8));
    let dx = sub(f32_at(g, p1), f32_at(g, p2));
    g.write::<u32>(p2 + 0xc, 0x3f80_0000);
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    // comiss + jbe each way; the two steps are two operands
    let v = sub(k(g, 0x1b7), a);
    let v = if v > k(g, 0x1c6) {
        sub(v, k(g, 0x1d0))
    } else if k(g, 0x1da) > v {
        add(v, k(g, 0x1e7))
    } else {
        v
    };
    g.write::<f32>(s + 0x1f24, v);
    g.write::<u32>(s + 0x1f28, 0x32);
    g.write::<u32>(s + 0x1f2c, 8);
    g.write::<u32>(s + 0x1f30, 0);
    // push 0; call: the fade's word
    g.write::<u32>(e1.wrapping_sub(4), 0);
    let fade = Frame {
        at: img.rel32_target(f + 0x22f),
        esp: e1.wrapping_sub(8),
        eax: 0,
        ecx: s + 0x1e80,
        edx: 0x3c,
        xmm: [
            0,
            crate::Xmm::f32(k(g, 0x1ff)).reg(),
            crate::Xmm::f32(k(g, 0x1ef)).reg(),
            0,
        ],
    };
    crate::s3_misc::fade_task(g, img, &fade);
    g.write::<u32>(s + 0x1e90, 0x3f33_3333);
    g.write::<u32>(s + 0x1e98, 0x3f33_3333);
    // call width: 1
    g.call(slot(g, 0x234, 271), 0, &[4]);
    // call width: 3
    let h = g.call(slot(g, 0x257, 719), 0, &[w(0x266), 6, 0]).eax;
    // call width: 3
    g.call(slot(g, 0x275, 220), 0, &[h, 0x40, 0x100]);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x287), w(0x282));
    let listed = g.read::<u32>(w(0x29a)) != 0;
    let first = if listed {
        let last = g.read::<u32>(w(0x2b1));
        chain_node(g, img, f, (0x294, 0x28f, 0), Some(1), Some(last))
    } else {
        chain_node(g, img, f, (0x294, 0x28f, 0x2c6), Some(1), None)
    };
    g.write::<u32>(first + 8, first);
    // `ebx`: the head as read, or the first node when it was empty
    let second = chain_node(g, img, f, (0x2d3, 0x2cb, 0x2f5), None, Some(first));
    g.write::<u32>(second + 8, second);
    let third = chain_node(g, img, f, (0x302, 0x2fa, 0x323), Some(2), Some(second));
    g.write::<u32>(third + 8, third);
    g.write::<u32>(w(0x332), third);
    Ret::eax(0)
}

/// [`start_up_0271`] (the `rel32`s: the callees, the cookie check).
pub const START_UP_0271: Sig = Sig {
    len: 835,
    hash: 0x8767_ed66_2844_6c85,
    calls: &[
        (0x5e, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7a, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1a5, Callee::Import("_CIatan2")),
        (0x22f, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x287, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x294, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x2d3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x302, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x33b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0271`'s load step (0x100012b0, the 118-byte shape) running
/// [`start_up_0271`].
pub fn load_0271<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0271)
}

/// [`load_0271`].
pub const LOAD_0271: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0271))],
    imports: &[],
};

/// `magic_0271`'s trail (0x100021c0, 1987 bytes; internal: `ecx` = the
/// trail `T`, `edx` = a matrix `M`, the stack `(P)`; a /GS frame aligned
/// to 16, `L(x)` = its `esp` after `sub esp, 0x158` plus `x`).
///
/// The draw's set-up: slots 723 `(G)` and 724 `(L(0x110))`, the pad bit
/// (slot 245 into `L(0x24)`: bit 0 picks the packet word `0xa8020000 |
/// 0xd00`), two constant vectors (`L(0x38)`, `L(0x58)`), two handles of
/// slot 942 (`L(0x48)`, `L(0x50)`), slot 230 `(L(0xd0), L(0x110))` and
/// slot 229 `(L(0x58), L(0xd0), L(0x58))`, the packet words `L(0x64..0x7c)`
/// (0, the 64-bit tag, two pairs of `*G_C`'s words), `*G_PKT = L(0xd0)`,
/// `G_PKT2 = 0`. `P` goes into the ring of 32 points (`T + 0xc + 16 i`,
/// `i = T[0x224]`, its flag byte `T + 0x22c + i` 1, `i` steps on), and the
/// animation steps ([`anim_step`](crate::sb_common::anim_step)).
///
/// Then for each of the `n = T[0x228]` segments `j`, walking back from
/// the newest point with the spacing `T[0x21c]`: while the distance `acc`
/// is not below the last segment's length `d`, the walk moves one point
/// back (`acc -= d`; it stops at an unset flag byte or after 32 points),
/// `L(0xc0)` = slot 234 `(L(0xc0), older, newer)` and `d` its length
/// (`sqrt` of `dy dy + dx dx + dz dz`); a length equal to `K` skips the
/// segment. Else the seed `s = (s * 0x80d + 7) & 0xffff` (from `T[0x220]`),
/// the colour bytes `G_COL..+3` = the two colours `T + 0x20c`/`+0x210`
/// mixed by `j / n` (signed divisions), the point `Q` (`L(0x98)`) the
/// newer point, or `older + D acc / d` (slot 248) with `acc += T[0x21c]`;
/// the width `(K' - (j + 1) / n) T[0x214] + T[0x218] (j + 1) / n`; the
/// matrix `L(0xd0)` = `M`'s 3x3 scaled by it (slot 235 first), turned by
/// `(s - 0x7fff) K1 K2` (slot 330) and moved to `Q` (slot 229 into
/// `L(0x100)`, copied to `L(0x28)`); with an alpha byte, the sprite's
/// frame (slot 941's table: `T[8] + s % count`, wrapped) linked
/// ([`link_part`](crate::sa_misc::link_part) `(G_REC, (T[0x224] << 16) +
/// j)`), slot 942 of the packet's `+0x10` (`L(0x4c)`), the sizes `L(0x80)`,
/// `L(0x84)` = width `K3`, slot 942 `(((T[0x224] + 0x27100) << 8) + j)`
/// (`L(0x88)`), slots 603 and 604 `(L(0x28))` with the game lock 1.
///
/// Returns the last `eax`: [`anim_step`](crate::sb_common::anim_step)'s
/// with no segments, 0x1f at an unset flag byte, the count left when the
/// walk ran out, `lahf`'s mark over it after a skipped segment, slot
/// 229's without an alpha, else the lock's address.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn trail_0271<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let l = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x158);
    let at = |o: u32| l.wrapping_add(o);
    let t = fr.ecx;
    let p = fr.arg(g, 0);
    g.write::<u32>(at(0x20), fr.edx);
    // call width: 1
    g.call(slot(g, 0x1b, 723), 0, &[w(0x28)]);
    // call width: 1
    g.call(slot(g, 0x40, 724), 0, &[at(0x110)]);
    // call width: 1
    g.call(slot(g, 0x55, 245), 0, &[at(0x24)]);
    let bit = if g.read::<u8>(at(0x24)) & 1 != 0 {
        0xd00
    } else {
        0
    };
    copy16(g, w(0x68), at(0x38));
    // call width: 1
    let h = g.call(slot(g, 0x72, 942), 0, &[w(0x89)]).eax;
    g.write::<u32>(at(0x48), h);
    // call width: 1
    let h = g.call(slot(g, 0x9a, 942), 0, &[w(0xa8)]).eax;
    g.write::<u32>(at(0x50), h);
    copy16(g, w(0xb1), at(0x58));
    // call width: 2
    g.call(slot(g, 0xcd, 230), 0, &[at(0xd0), at(0x110)]);
    for o in [0x10c, 0x108, 0x104, 0x100] {
        g.write::<u32>(at(o), 0);
    }
    // call width: 3
    g.call(slot(g, 0xe0, 229), 0, &[at(0x58), at(0xd0), at(0x58)]);
    let c = g.read::<u32>(w(0x12c));
    g.write::<u32>(at(0x68), bit | 0xa802_0000);
    g.write::<u32>(at(0x64), 0);
    g.write::<u32>(at(0x6c), 2);
    for (to, from) in [(0x70u32, 0x48u32), (0x74, 0x4c), (0x78, 0x38), (0x7c, 0x3c)] {
        let v = g.read::<u32>(c.wrapping_add(from));
        g.write::<u32>(at(to), v);
    }
    g.write::<u32>(w(0x17e), at(0xd0));
    g.write::<u32>(w(0x186), 0);
    let i = g.read::<u32>(t + 0x224);
    copy16(g, p, t + 0xc + 16 * i);
    let i = g.read::<u32>(t + 0x224);
    g.write::<u8>(t + 0x22c + i, 1);
    let mut cur = g.read::<u32>(t + 0x224);
    let mut seed = g.read::<u32>(t + 0x220);
    let next = cur.wrapping_add(1);
    g.write::<u32>(t + 0x224, if next.cast_signed() >= 0x20 { 0 } else { next });
    let mut left = 0x20u32;
    let mut d = 0.0f32;
    put(g, at(4), 0.0);
    let mut eax = crate::sb_common::anim_step(g, img, img.rel32_target(f + 0x1fb), t);
    let mut j = 0u32;
    while j.cast_signed() < g.read::<i32>(t + 0x228) {
        let acc = f32_at(g, at(4));
        // comiss acc, d; jb: below or unordered goes straight on
        let mut skip = false;
        if !(acc < d || acc.is_nan() || d.is_nan()) {
            let prev = cur.wrapping_sub(1);
            let pw = if prev.cast_signed() < 0 { 0x1f } else { prev };
            put(g, at(4), sub(acc, d));
            if g.read::<u8>(t + 0x22c + pw) != 1 {
                return 0x1f;
            }
            copy16(g, t + 0xc + 16 * cur, at(0xa8));
            // call width: 3
            g.call(
                slot(g, 0x2b2, 234),
                0,
                &[at(0xc0), t + 0xc + 16 * pw, at(0xa8)],
            );
            let (dx, dy, dz) = (
                f32_at(g, at(0xc0)),
                f32_at(g, at(0xc4)),
                f32_at(g, at(0xc8)),
            );
            let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
            // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
            #[allow(clippy::cast_possible_truncation)]
            let len = f64::from(sq).sqrt() as f32;
            d = len;
            cur = pw;
            left = left.wrapping_sub(1);
            put(g, at(0x14), d);
            if left.cast_signed() <= 0 {
                return left;
            }
            // ucomiss d, K; lahf; test ah, 0x44; jnp: equal (ordered)
            #[allow(clippy::float_cmp)] // the original's exact test
            let equal = d == k(g, 0x334);
            if equal {
                eax = (left & !0xff00) | 0x4200;
                skip = true;
            }
        }
        if !skip {
            seed = (seed.wrapping_mul(0x80d).wrapping_add(7)) & 0xffff;
            g.write::<u32>(at(0xc), seed);
            let n = g.read::<i32>(t + 0x228);
            let ji = j.cast_signed();
            for (q, op) in (0u32..).zip([0x383u32, 0x3af, 0x3db, 0x40a]) {
                let a = i32::from(g.read::<u8>(t + 0x20c + q));
                let b = i32::from(g.read::<u8>(t + 0x210 + q));
                let v = a
                    .wrapping_mul(n.wrapping_sub(ji))
                    .wrapping_add(b.wrapping_mul(ji))
                    .checked_div(n)
                    .unwrap_or(0);
                // `mov [G], al`: the quotient's low byte
                g.write::<u8>(w(op), v.to_le_bytes()[0]);
            }
            let acc = f32_at(g, at(4));
            // comiss acc, d; jb: below or unordered moves along `D`
            if acc < d || acc.is_nan() || d.is_nan() {
                let r = div(acc, d);
                put(g, at(0x98), mul(f32_at(g, at(0xc0)), r));
                put(g, at(0x9c), mul(f32_at(g, at(0xc4)), r));
                put(g, at(0xa0), mul(f32_at(g, at(0xc8)), r));
                // call width: 3
                g.call(slot(g, 0x480, 248), 0, &[at(0x98), at(0xa8), at(0x98)]);
                let a = add(f32_at(g, t + 0x21c), f32_at(g, at(4)));
                put(g, at(4), a);
            } else {
                copy16(g, at(0xa8), at(0x98));
            }
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps, rounded as the original
            let s = div(
                (j.wrapping_add(1)).cast_signed() as f32,
                g.read::<i32>(t + 0x228) as f32,
            );
            let width = add(
                mul(sub(k(g, 0x4cc), s), f32_at(g, t + 0x214)),
                mul(f32_at(g, t + 0x218), s),
            );
            g.write::<u32>(at(0xa4), 0x3f80_0000);
            put(g, at(0x8), width);
            // call width: 1
            g.call(slot(g, 0x4f5, 235), 0, &[at(0xd0)]);
            let m = g.read::<u32>(at(0x20));
            for (to, from) in [
                (0xd0u32, 0u32),
                (0xd4, 4),
                (0xd8, 8),
                (0xe0, 0x10),
                (0xe4, 0x14),
                (0xe8, 0x18),
                (0xf0, 0x20),
                (0xf4, 0x24),
                (0xf8, 0x28),
            ] {
                let v = f32_at(g, m.wrapping_add(from));
                let v = if from == 0x10 {
                    mul(width, v)
                } else {
                    mul(v, width)
                };
                put(g, at(to), v);
            }
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps, rounded as the original
            let turn = mul(
                mul(
                    seed.wrapping_add(0xffff_8001).cast_signed() as f32,
                    k(g, 0x5db),
                ),
                k(g, 0x5e3),
            );
            // call width: 3
            g.call(
                slot(g, 0x5ef, 330),
                0,
                &[at(0xd0), at(0xd0), turn.to_bits()],
            );
            // call width: 3
            eax = g
                .call(slot(g, 0x617, 229), 0, &[at(0x100), at(0x110), at(0x98)])
                .eax;
            copy16(g, at(0x100), at(0x28));
            if g.read::<u8>(w(0x640)) != 0 {
                let key = g.read::<u32>(t);
                // call width: 1
                let tb = g.call(slot(g, 0x652, 941), 0, &[key]).eax;
                let count = i32::from(g.read::<i16>(tb + 6));
                let r = seed.cast_signed().checked_rem(count).unwrap_or(0);
                let mut frame = i32::from(g.read::<i16>(t + 8)).wrapping_add(r);
                g.write::<u32>(at(0x10), frame.cast_unsigned());
                // call width: 1
                let tb = g.call(slot(g, 0x670, 941), 0, &[key]).eax;
                if frame >= i32::from(g.read::<i16>(tb + 6)) {
                    // call width: 1
                    let tb = g.call(slot(g, 0x695, 941), 0, &[key]).eax;
                    frame = frame.wrapping_sub(i32::from(g.read::<i16>(tb + 6)));
                    g.write::<u32>(at(0x10), frame.cast_unsigned());
                }
                // call width: 1
                let tb = g.call(slot(g, 0x6af, 941), 0, &[key]).eax;
                let entry = tb.wrapping_add(frame.wrapping_add(2).cast_unsigned().wrapping_mul(8));
                let idx = (g.read::<u32>(t + 0x224) << 16).wrapping_add(j);
                // the link's two words: `push idx; push G_REC`
                let sp = l.wrapping_sub(8);
                g.write::<u32>(sp.wrapping_sub(4), idx);
                g.write::<u32>(sp.wrapping_sub(8), w(0x6d7));
                let link = Frame {
                    at: img.rel32_target(f + 0x6de),
                    esp: sp.wrapping_sub(0xc),
                    ecx: t,
                    edx: entry,
                    ..Frame::default()
                };
                crate::sa_misc::link_part(g, img, &link);
                let pk = g.read::<u32>(w(0x6e3)).wrapping_add(0x10);
                // call width: 1
                let h = g.call(slot(g, 0x6ec, 942), 0, &[pk]).eax;
                g.write::<u32>(at(0x4c), h);
                let size = mul(f32_at(g, at(0x8)), k(g, 0x702));
                let key2 = ((g.read::<u32>(t + 0x224).wrapping_add(0x27100)) << 8).wrapping_add(j);
                put(g, at(0x80), size);
                put(g, at(0x84), size);
                // call width: 1
                let h = g.call(slot(g, 0x728, 942), 0, &[key2]).eax;
                g.write::<u32>(at(0x88), h);
                let lock = g.read::<u32>(w(0x745));
                let saved = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                // call width: 1
                g.call(slot(g, 0x75a, 603), 0, &[at(0x28)]);
                let lock = g.read::<u32>(w(0x767));
                g.write::<u32>(lock, 1);
                // call width: 1
                g.call(slot(g, 0x77a, 604), 0, &[at(0x28)]);
                let lock = g.read::<u32>(w(0x787));
                g.write::<u32>(lock, saved);
                eax = lock;
            }
        }
        j += 1;
    }
    eax
}

/// [`trail_0271`] (the `rel32`s: the animation step, `sqrt`, the part's
/// link, the cookie check).
pub const TRAIL_0271: Sig = Sig {
    len: 1987,
    hash: 0xc7df_4ae6_7c7c_c5a4,
    calls: &[
        (0x1fb, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x2fd, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x6de, Callee::Sig(&crate::sa_misc::LINK_PART)),
        (0x7bb, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0271`'s curve task (0x10001e80, 576 bytes; a pool node's
/// callback, cdecl `(node)`; a /GS `ebp` frame): the record `R` =
/// `*G_S + 0x3d0 * node[0x14]`. Its count `R + 0x3bc` steps down to 0;
/// `t` = count / `K` (0 at or below 0), `u = K' - t`, and the point
/// `X = u³ R[0x30] + 3 u² t R[0x20] + 3 u t² R[0x10] + t³ R[0]` (each
/// axis, `w` 1.0, at `ebp - 0x14`); the matrix `ebp - 0x54` the unit
/// (slot 235) with 0.12 on its diagonal. At the count 0: the object `R +
/// 0x40` started at scale `K''` ([`start_object_at`](crate::s4_misc::start_object_at),
/// index 2), `R + 0x110` = `R + 0x30`'s four words, `R + 0x104` = the
/// state's yaw `S + 0x1f24`, the count -1. With the count above 0, or
/// while the trail's time `R + 0x398` lasts (counted down), the trail
/// `R + 0x170` is drawn ([`trail_0271`]); else the node goes back to the
/// pool ([`give_back`](crate::sa_pool::give_back)). Returns the trail's
/// or the give-back's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn curve_task_0271<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let node = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x11));
    let r = g
        .read::<u32>(node + 0x14)
        .wrapping_mul(0x3d0)
        .wrapping_add(s);
    let c = g.read::<i32>(r + 0x3bc);
    if c > 0 {
        g.write::<i32>(r + 0x3bc, c - 1);
    }
    let c = g.read::<i32>(r + 0x3bc);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps, rounded as the original
    let t = if c > 0 {
        div(c as f32, k(g, 0x4f))
    } else {
        0.0
    };
    let u = sub(k(g, 0x5c), t);
    let three = k(g, 0x64);
    let tt = mul(t, t);
    let axis = |g: &G, o: u32| {
        let p = |i: u32| f32_at(g, r + 0x10 * i + o);
        let b = mul(mul(mul(mul(u, t), u), three), p(2));
        let a = mul(mul(mul(u, u), u), p(3));
        let c = mul(mul(mul(tt, u), three), p(1));
        let d = mul(mul(tt, t), p(0));
        add(add(add(b, a), c), d)
    };
    let x = axis(g, 0);
    put(g, at(0x14), x);
    let y = axis(g, 4);
    put(g, at(0x10), y);
    let z = axis(g, 8);
    put(g, at(0xc), z);
    g.write::<u32>(at(8), 0x3f80_0000);
    // call width: 1
    g.call(slot_target(g, w(0x9c), 235), 0, &[at(0x54)]);
    for o in [0x2c, 0x40, 0x54] {
        g.write::<u32>(at(o), 0x3df5_c28f);
    }
    let c = g.read::<i32>(r + 0x3bc);
    if c <= 0 {
        if c == 0 {
            let start = img.rel32_target(f + 0x1b1);
            crate::s4_misc::start_object_at(g, img, start, r + 0x40, k(g, 0x1a2), 2, 0, 0);
            for i in 0..4 {
                let v = g.read::<u32>(r + 0x30 + 4 * i);
                g.write::<u32>(r + 0x110 + 4 * i, v);
            }
            let yaw = g.read::<u32>(s + 0x1f24);
            g.write::<u32>(r + 0x104, yaw);
            let c = g.read::<i32>(r + 0x3bc).wrapping_sub(1);
            g.write::<i32>(r + 0x3bc, c);
        }
        let d = g.read::<i32>(r + 0x398);
        if d <= 0 {
            let back = img.rel32_target(f + 0x22b);
            return Ret::eax(crate::sa_pool::give_back(g, img, back, node));
        }
        g.write::<i32>(r + 0x398, d - 1);
    }
    // push &X; call: the trail's word
    let sp = ebp.wrapping_sub(0x5c + 0xc);
    g.write::<u32>(sp.wrapping_sub(4), at(0x14));
    let trail = Frame {
        at: img.rel32_target(f + 0x210),
        esp: sp.wrapping_sub(8),
        ecx: r + 0x170,
        edx: at(0x54),
        ..Frame::default()
    };
    Ret::eax(trail_0271(g, img, &trail))
}

/// [`curve_task_0271`] (the `rel32`s: the object start, the trail, the
/// give-back, the cookie checks).
pub const CURVE_TASK_0271: Sig = Sig {
    len: 576,
    hash: 0x9cb8_51f7_c0bb_81b4,
    calls: &[
        (0x1b1, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x210, Callee::Sig(&TRAIL_0271)),
        (0x220, Callee::Any),
        (0x22b, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x238, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0370`'s sprite frame (0x10002810, 996 bytes; internal: `ecx` =
/// the record, `edx` = the draw record `D`, `xmm3` = the time step, the
/// stack `(id, part)`): [`sprite_frame`](crate::o6_fam_pairs5::sprite_frame)
/// with `D` in place of the DLL's draw globals (`D + 0x10` the matrix,
/// `D + 0x1c` the light word, the colour bytes, the id) and its operands
/// moved.
pub fn sprite_frame_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    const LAYOUT: crate::o6_fam_pairs5::FrameLayout = crate::o6_fam_pairs5::FrameLayout {
        shift: Shift {
            runs: &[
                (0xa, 0),
                (0x29, 8),
                (0x1a5, -12),
                (0x1b4, 2),
                (0x282, -2),
                (0x2f8, 0),
                (0x373, -3),
                (0x394, -9),
                (0x3a8, -1),
            ],
            moved: &[(0x19d, 0x191)],
        },
        desc: Some((0x1a0, 0x1a7)),
    };
    crate::o6_fam_pairs5::sprite_frame_with(g, img, fr, &LAYOUT)
}

/// [`sprite_frame_0370`] (the `rel32`s: the cookie checks, the printf
/// wrapper).
pub const SPRITE_FRAME_0370: Sig = Sig {
    len: 996,
    hash: 0xe808_061e_8a4d_d7b7,
    calls: &[
        (0x330, Callee::Any),
        (0x3bc, Callee::Any),
        (0x3ca, Callee::Sig(&crate::opu::DEBUG_PRINTF)),
        (0x3dc, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0370`'s block frames (0x10002f80, 173 bytes; internal: `ecx` =
/// the block `B`, `edx` = the draw record `D`, `xmm3` = the time step `dt`;
/// an `ebp` /GS frame: `dt` at `ebp - 0x28`, `D` at `-0x20`, `B` at
/// `-0x24`, `x` at `-0x1c`, `L` at `-0x18`): as
/// [`block_sprite_frames`](crate::s9_misc::block_sprite_frames), each live
/// record's frame drawn by [`sprite_frame_0370`] with `D` and `dt` handed
/// on. Returns `x`.
pub fn block_frames_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let s = |o: u32| ebp.wrapping_sub(o);
    let (dt_at, rec_at, b_at, x_at, l) = (s(0x28), s(0x20), s(0x24), s(0x1c), s(0x18));
    let call = s(0x40);
    let b = fr.ecx;
    let dt = fr.xmm_f32(3).to_bits();
    let mut x = g.read::<u32>(w(0x11));
    let base = g.read::<u32>(b.wrapping_add(4));
    g.write::<u32>(dt_at, dt);
    g.write::<u32>(rec_at, fr.edx);
    g.write::<u32>(b_at, b);
    g.write::<u32>(x_at, x);
    let frame_at = img.rel32_target(f + 0x7e);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(b) {
        let rec = base.wrapping_add(i.wrapping_mul(0x94));
        if g.read::<u32>(rec) != 0 {
            let p = if g.read::<u32>(b.wrapping_add(0x18)) == 0 {
                rec.wrapping_add(4)
            } else {
                // call width: 3
                let t = slot_target(g, w(0x4a), 229);
                g.call(t, 0, &[l, b.wrapping_add(0x1c), rec.wrapping_add(4)]);
                l
            };
            // call width: 2
            let t = slot_target(g, w(0x5e), 325);
            g.call(t, 0, &[rec.wrapping_add(0x44), p]);
            g.write::<u32>(call + 4, x);
            g.write::<u32>(call + 8, i);
            let sub = Frame {
                at: frame_at,
                esp: call,
                eax: 0,
                ecx: rec.wrapping_add(0x30),
                edx: g.read::<u32>(rec_at),
                xmm: [0, 0, 0, u128::from(dt)],
            };
            x = sprite_frame_0370(g, img, &sub).eax;
            g.write::<u32>(x_at, x);
        }
        i += 1;
    }
    Ret::eax(x)
}

/// [`block_frames_0370`] (the `rel32`s: the sprite frame, the cookie
/// check).
pub const BLOCK_FRAMES_0370: Sig = Sig {
    len: 173,
    hash: 0xda2f_71e0_ae10_512e,
    calls: &[(0x7e, Callee::Sig(&SPRITE_FRAME_0370)), (0xa5, Callee::Any)],
    imports: &[],
};

/// `slot 324 & 0x7fff` as a float (`cvtdq2ps`), through the table global
/// at operand `op` of `f`.
fn rnd15<G: Game>(g: &mut G, img: &Image, f: u32, op: u32) -> f32 {
    let t = slot_target(g, img.word(f + op), 324);
    // call width: 0
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    #[allow(clippy::cast_precision_loss)] // below 2^15: exact
    let x = r as f32;
    x
}

/// `magic_0370`'s spark (0x100011a0, 373 bytes; internal: `ecx` = a
/// sprite pool, `edx` = its resource, the stack `(p, kind)`): a sprite
/// taken ([`sprite_take`](crate::s4_misc::sprite_take); none: 0), its
/// times `+0x28`/`+0x24` 0x10, its place `p`'s three words, the speeds
/// `+0x14..+0x1c` = `((r / K - K') K'' K''')` of three randoms, `+0x58` =
/// slot 403 `(r / K * K4)` (ST0), `+0x54` 0.1875, `+0x68` 1.0, and the
/// colour steps `+0x5c..+0x64` (0x18, 7, 6; with `kind` 0x3c, 0x6e,
/// 0x5a). Returns slot 403's `eax`.
pub fn spark_0370<G: Game>(g: &mut G, img: &Image, f: u32, fr: &Frame) -> u32 {
    let k = |g: &G, o: u32| f32_at(g, img.word(f + o));
    let take = Frame {
        at: img.rel32_target(f + 5),
        esp: fr.esp.wrapping_sub(8),
        ecx: fr.ecx,
        edx: fr.edx,
        ..Frame::default()
    };
    let s = crate::s4_misc::sprite_take(g, img, &take).eax;
    if s == 0 {
        return 0;
    }
    let (p, kind) = (fr.arg(g, 0), fr.arg(g, 1));
    g.write::<u32>(s + 0x28, 0x10);
    g.write::<u32>(s + 0x24, 0x10);
    for i in 0..3 {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 4 + 4 * i, v);
    }
    for (i, (t, a, b, c, d)) in (0u32..).zip([
        (0x36u32, 0x57u32, 0x5fu32, 0x67u32, 0x6fu32),
        (0x4f, 0x95, 0x9d, 0xa5, 0xad),
        (0x8d, 0xd3, 0xdb, 0xe3, 0xeb),
    ]) {
        let r = rnd15(g, img, f, t);
        let v = mul(mul(sub(div(r, k(g, a)), k(g, b)), k(g, c)), k(g, d));
        put(g, s + 0x14 + 4 * i, v);
    }
    let r = rnd15(g, img, f, 0xcb);
    let x = mul(div(r, k(g, 0x111)), k(g, 0x11f));
    let t = slot_target(g, img.word(f + 0x109), 403);
    // call width: 1
    let out = g.call(t, 0, &[x.to_bits()]);
    put(g, s + 0x58, crate::m0018::st0_f32(out.st0));
    g.write::<u32>(s + 0x54, 0x3e40_0000);
    g.write::<u32>(s + 0x68, 0x3f80_0000);
    let steps = if kind == 0 {
        [0x18, 7, 6]
    } else {
        [0x3c, 0x6e, 0x5a]
    };
    for (i, v) in (0u32..).zip(steps) {
        g.write::<u32>(s + 0x5c + 4 * i, v);
    }
    out.eax
}

/// [`spark_0370`] (the `rel32`: the sprite take).
pub const SPARK_0370: Sig = Sig {
    len: 373,
    hash: 0xc660_2291_a0fa_6480,
    calls: &[(0x5, Callee::Sig(&crate::s4_misc::SPRITE_TAKE))],
    imports: &[],
};

/// `magic_0370`'s sprite reset (0x10001320, 242 bytes; no arguments; an
/// `ebp` frame): for the two draw records (0x30 bytes at `G_D`), the
/// handle `G_H[i]` = slot 719 `(*G_R, 4, i)`, the record cleared
/// (`memset`) with its `u16 +2` 0x80; then for each sprite system (0x5c
/// bytes from `G_S - 8` to `G_END`): cleared, its count 500, its sprites
/// the next pointer of the table `G_T`, `+8`/`+0x10`/`+0x18` 0, and its
/// 500 sprites (0x94 bytes) cleared. Returns the table's pointer past the
/// last.
pub fn sprite_reset_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    for i in 0..2u32 {
        let slot = slot_target(g, w(0x1e), 719);
        let who = g.read::<u32>(w(0x26));
        // call width: 3
        let h = g.call(slot, 0, &[who, 4, i]).eax;
        let rec = w(0x41).wrapping_add(i * 0x30);
        g.write::<u32>(w(0x4b).wrapping_add(4 * i), h);
        g.write_bytes(rec, &[0; 0x30]); // memset(rec, 0, 0x30)
        g.write::<u16>(w(0x60).wrapping_add(i * 0x30), 0x80);
    }
    let mut tp = w(0x6a);
    let mut b = w(0x72);
    loop {
        let mut sp = g.read::<u32>(tp);
        let head = b.wrapping_sub(8);
        g.write_bytes(head, &[0; 0x5c]); // memset(head, 0, 0x5c)
        g.write::<u32>(head, 0x1f4);
        g.write::<u32>(b.wrapping_sub(4), sp);
        for o in [0u32, 8, 0x10] {
            g.write::<u32>(b + o, 0);
        }
        for _ in 0..0x1f4 {
            g.write_bytes(sp, &[0; 0x94]); // memset(sp, 0, 0x94)
            g.write::<u32>(sp, 0);
            sp = sp.wrapping_add(0x94);
        }
        for o in [0u32, 8, 0x10] {
            g.write::<u32>(b + o, 0);
        }
        tp = tp.wrapping_add(4);
        b = b.wrapping_add(0x5c);
        if b.cast_signed() >= w(0xe5).cast_signed() {
            return Ret::eax(tp);
        }
    }
}

/// [`sprite_reset_0370`] (the `rel32`s: `memset`).
pub const SPRITE_RESET_0370: Sig = Sig {
    len: 242,
    hash: 0x0ace_e80a_59a4_8eb6,
    calls: &[
        (0x50, Callee::Import("memset")),
        (0x81, Callee::Import("memset")),
        (0xb9, Callee::Import("memset")),
    ],
    imports: &[],
};

/// `magic_0370`'s sprite update (0x10001420, 288 bytes; no arguments; a
/// /GS frame aligned to 8, `P` at its `esp + 4`): `P` = slot 237 `(*G_C,
/// 0x3c, 0, P)`; with `(r / K - K') K''` above 0 a spark of kind 0 from
/// the first pool ([`spark_0370`], resource `*G_R0`), with `r' / K` below
/// `K'''` one of kind 1 from the second (`*G_R1`); then each of the 2 sprite
/// systems (0x5c bytes) steps ([`sprites_step`](crate::s6_misc::sprites_step))
/// and draws its frames with its draw record ([`block_frames_0370`],
/// `dt` 0), `*G_X` its answer. Returns 1.
pub fn sprite_update_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let p = e + 4;
    let who = g.read::<u32>(w(0x2c));
    // call width: 4
    g.call(slot_target(g, w(0x1c), 237), 0, &[who, 0x3c, 0, p]);
    let spark = |g: &mut G, (pool, res, kind): (u32, u32, u32)| {
        let sp = e.wrapping_sub(0x10);
        g.write::<u32>(sp, p);
        g.write::<u32>(sp + 4, kind);
        let fr2 = Frame {
            at: 0,
            esp: sp.wrapping_sub(4),
            ecx: w(pool),
            edx: g.read::<u32>(w(res)),
            ..Frame::default()
        };
        let at = img.rel32_target(f + if kind == 0 { 0x81 } else { 0xc9 });
        spark_0370(g, img, at, &fr2);
    };
    let x = mul(
        sub(div(rnd15(g, img, f, 0x33), k(g, 0x55)), k(g, 0x5d)),
        k(g, 0x65),
    );
    // comiss x, 0; jbe: only above 0 (ordered)
    if x > 0.0 {
        spark(g, (0x7c, 0x70, 0));
    }
    let y = div(rnd15(g, img, f, 0x89), k(g, 0xad));
    // comiss K, y; jbe: only when K is above
    if k(g, 0xa5) > y {
        spark(g, (0xc4, 0xb8, 1));
    }
    let (mut sys, mut rec) = (w(0xd1), w(0xd6));
    let step = img.rel32_target(f + 0xe3);
    let frames = img.rel32_target(f + 0xf0);
    loop {
        let st = Frame {
            at: step,
            esp: e.wrapping_sub(0xc),
            ecx: sys,
            ..Frame::default()
        };
        crate::s6_misc::sprites_step(g, img, &st);
        let bf = Frame {
            at: frames,
            esp: e.wrapping_sub(0x10),
            ecx: sys,
            edx: rec,
            ..Frame::default()
        };
        let x = block_frames_0370(g, img, &bf).eax;
        rec = rec.wrapping_add(0x30);
        sys = sys.wrapping_add(0x5c);
        g.write::<u32>(w(0xfe), x);
        if rec.cast_signed() >= w(0x104).cast_signed() {
            return Ret::eax(1);
        }
    }
}

/// [`sprite_update_0370`] (the `rel32`s: the sparks, the step, the
/// frames, the cookie check).
pub const SPRITE_UPDATE_0370: Sig = Sig {
    len: 288,
    hash: 0x7963_f43c_40ac_8cba,
    calls: &[
        (0x81, Callee::Sig(&SPARK_0370)),
        (0xc9, Callee::Sig(&SPARK_0370)),
        (0xe3, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0xf0, Callee::Sig(&BLOCK_FRAMES_0370)),
        (0x118, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0370`'s draw (0x100015c0, 301 bytes; no arguments, no frame):
/// `**G_W` = the scene word; slots 723 `(*G_A)`, 724 `(*G_B)`, 258
/// `(M)`, 231 `(*G_M1, *G_B, *G_M2)`, 230 `(*G_V, *G_M1)`; the part's
/// data `(M, 0x30)` checked by slot 712 with the game lock 1 (the scene
/// word set again): 0 ends with 1; then slot 261 `(1)` non-zero ends with
/// 0; else slot 713 `(M)` and the part restarted ([`start_part`](crate::opu_parts::start_part)
/// with `M`, 0x1000, the program of `*G_P`'s table, 1). Returns 1.
pub fn draw_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let r = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let scene = |g: &mut G, op: u32, v: u32| {
        let p = g.read::<u32>(w(op));
        g.write::<u32>(p, w(v));
    };
    let a = r(g, 0x8);
    scene(g, 0x1, 0xe);
    // call width: 1
    g.call(slot(g, 0x13, 723), 0, &[a]);
    let b = r(g, 0x26);
    // call width: 1
    g.call(slot(g, 0x20, 724), 0, &[b]);
    // call width: 1
    g.call(slot(g, 0x33, 258), 0, &[w(0x38)]);
    let (m2, b, m1) = (r(g, 0x46), r(g, 0x51), r(g, 0x5d));
    // call width: 3
    g.call(slot(g, 0x4b, 231), 0, &[m1, b, m2]);
    let (m1, v) = (r(g, 0x65), r(g, 0x70));
    // call width: 2
    g.call(slot(g, 0x6a, 230), 0, &[v, m1]);
    scene(g, 0x7d, 0x85);
    let lock = r(g, 0x8a);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let ok = g.call(slot(g, 0x9c, 712), 0, &[w(0x8f), 0x30]).eax;
    let lock = r(g, 0xaa);
    g.write::<u32>(lock, saved);
    if ok == 0 {
        return Ret::eax(1);
    }
    // call width: 1
    if g.call(slot(g, 0xb9, 261), 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    scene(g, 0xd2, 0xdd);
    // call width: 1
    g.call(slot(g, 0xe2, 713), 0, &[w(0xd7)]);
    scene(g, 0xef, 0xf5);
    let d = r(g, 0xfa);
    let key = g.read::<u32>(d + 0x10);
    // call width: 1
    let tb = g.call(slot(g, 0x102, 941), 0, &[key]).eax;
    let program = g.read::<u32>(tb + 8);
    let at = img.rel32_target(f + 0x120);
    crate::opu_parts::start_part(g, img, at, w(0x11b), 0x1000, program, 1);
    Ret::eax(1)
}

/// [`draw_0370`] (the `rel32`: the part start).
pub const DRAW_0370: Sig = Sig {
    len: 301,
    hash: 0xa697_d2a0_2c66_3051,
    calls: &[(0x120, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// `magic_0370`'s start (0x10001a70, 579 bytes; cdecl `(a)`): `G_A = a`;
/// a 16 MiB buffer (slot 803 `(0x1000000, 0x10)`) cut into four globals
/// (0x80000 apart, the last twice and `+0x12110`); the resource name set
/// and loaded (slots 718, 222 `(*G_N, 0, 1000, 0x2000)`); the part `*G_P`
/// = slot 719 `(*G_N, 8, 0)`, set up by slot 711 `(P, D, 0)` with the
/// game lock 1 and slot 710 `(env, P, buf, 0x80000)`; three tables of
/// slot 941 (the part's `+0x14`, `+0x18`, `+0x1c`); three 0x110-byte
/// draw blocks cleared (`memset`) and given their vtable and flag words;
/// `G_X` = slot 260 `(0x17)`, `G_Y` = slot 260 `(0x14)`. Returns 1.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_0370<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    g.write::<u32>(w(0x7), fr.arg(g, 0));
    // call width: 2
    let buf = g.call(slot(g, 0xc, 803), 0, &[0x0100_0000, 0x10]).eax;
    g.write::<u32>(w(0x21), buf);
    let mut x = buf;
    for o in [0x2bu32, 0x35, 0x3f] {
        x = x.wrapping_add(0x80000);
        g.write::<u32>(w(o), x);
    }
    g.write::<u32>(w(0x44), x);
    g.write::<u32>(w(0x4e), x.wrapping_add(0x12110));
    stores(g, img, f, &[(0x64, 0x68)]);
    // call width: 1
    g.call(slot(g, 0x53, 718), 0, &[w(0x58)]);
    let n = g.read::<u32>(w(0x87));
    // call width: 4
    g.call(slot(g, 0x6f, 222), 0, &[n, 0, 0x3e8, 0x2000]);
    let n = g.read::<u32>(w(0x9e));
    // call width: 3
    let h = g.call(slot(g, 0x8e, 719), 0, &[n, 8, 0]).eax;
    let lock = g.read::<u32>(w(0xa6));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xaf), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(slot(g, 0xba, 711), 0, &[h, w(0xb4), 0]);
    let lock = g.read::<u32>(w(0xcd));
    g.write::<u32>(lock, saved);
    let (p, b) = (g.read::<u32>(w(0xe0)), g.read::<u32>(w(0xd8)));
    // call width: 4
    g.call(slot(g, 0xe5, 710), 0, &[w(0xea), p, b, 0x80000]);
    for (src, off, tb, dst) in [
        (0xf7u32, 0x14u32, 0x102u32, 0x10fu32),
        (0x114, 0x18, 0x11c, 0x129),
        (0x12e, 0x1c, 0x136, 0x14f),
    ] {
        let part = g.read::<u32>(w(src));
        let key = g.read::<u32>(part + off);
        // call width: 1
        let t = g.call(slot(g, tb, 941), 0, &[key]).eax;
        g.write::<u32>(w(dst), t);
    }
    g.write_bytes(w(0x14a), &[0; 0x110]); // memset(G_D0, 0, 0x110)
    stores(g, img, f, &[(0x15a, 0x15e)]);
    sets(g, img, f, &[(0x164, 1), (0x16e, 1), (0x184, 1)]);
    g.write_bytes(w(0x17e), &[0; 0x110]); // memset(G_D1, 0, 0x110)
    stores(g, img, f, &[(0x19f, 0x1a3)]);
    sets(
        g,
        img,
        f,
        &[(0x1a9, 3), (0x1b3, 3), (0x1bd, 1), (0x1c7, 1), (0x1d1, 1)],
    );
    g.write_bytes(w(0x199), &[0; 0x110]); // memset(G_D2, 0, 0x110)
    stores(g, img, f, &[(0x1e5, 0x1e9)]);
    sets(g, img, f, &[(0x1ef, 2), (0x1f9, 2), (0x203, 1), (0x20d, 1)]);
    // call width: 1
    let x = g.call(slot(g, 0x1df, 260), 0, &[0x17]).eax;
    g.write::<u32>(w(0x220), x);
    // call width: 1
    let y = g.call(slot(g, 0x225, 260), 0, &[0x14]).eax;
    g.write::<u32>(w(0x237), y);
    Ret::eax(1)
}

/// [`start_0370`] (the `rel32`s: `memset`).
pub const START_0370: Sig = Sig {
    len: 579,
    hash: 0xebb7_e0a5_baef_313e,
    calls: &[
        (0x154, Callee::Import("memset")),
        (0x18d, Callee::Import("memset")),
        (0x1da, Callee::Import("memset")),
    ],
    imports: &[],
};

/// `**G_SCENE` = the scene word, both operands of `f` (the pointer global
/// at `ptr`, the immediate at `imm`).
fn scene<G: Game>(g: &mut G, img: &Image, f: u32, ptr: u32, imm: u32) {
    let p = g.read::<u32>(img.word(f + ptr));
    g.write::<u32>(p, img.word(f + imm));
}

/// The view slots `magic_0376`'s part callbacks share: slot 258 `(p)`,
/// slot 231 `(*G_M1, *G_B, *G_M2)`, slot 230 `(*G_V, *G_M1)` (the
/// operands `(t258, t231, m2, b, m1, t230, m1', v)`).
fn view_0376<G: Game>(g: &mut G, img: &Image, f: u32, p: u32, ops: [u32; 8]) {
    let w = |o: u32| img.word(f + o);
    let [t258, t231, m2, b, m1, t230, m1b, v] = ops;
    // call width: 1
    g.call(slot_target(g, w(t258), 258), 0, &[p]);
    let (m2, b, m1) = (
        g.read::<u32>(w(m2)),
        g.read::<u32>(w(b)),
        g.read::<u32>(w(m1)),
    );
    // call width: 3
    g.call(slot_target(g, w(t231), 231), 0, &[m1, b, m2]);
    let (m1, v) = (g.read::<u32>(w(m1b)), g.read::<u32>(w(v)));
    // call width: 2
    g.call(slot_target(g, w(t230), 230), 0, &[v, m1]);
}

/// The part's data checked by slot 712 `(p, 0x30)` (table operand `t`)
/// with the game lock `**G_LOCK` (operand `lock`) set to 1, the scene word
/// stored (`ptr`, `imm`) after the first 1, the lock read back and put
/// back after the call (operand `lock2`). Returns slot 712's `eax` and the
/// lock's address after it.
fn check_712<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: u32,
    (lock, ptr, imm, t, lock2): (u32, u32, u32, u32, u32),
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let l = g.read::<u32>(w(lock));
    let sp = g.read::<u32>(w(ptr));
    g.write::<u32>(l, 1);
    g.write::<u32>(sp, w(imm));
    let saved = g.read::<u32>(l);
    g.write::<u32>(l, 1);
    // call width: 2
    let ok = g.call(slot_target(g, w(t), 712), 0, &[p, 0x30]).eax;
    let l2 = g.read::<u32>(w(lock2));
    g.write::<u32>(l2, saved);
    (ok, l2)
}

/// A part restarted from the program table of `*G_PART` (slot 941 of its
/// `+0x10`, the program at `+off`; [`start_part`](crate::opu_parts::start_part)
/// `(p, 0x1000, program, 1)`), after the scene word. Operands: the scene
/// pointer and immediate, the part global, the slot table, the `rel32`.
/// Returns the part start's `eax`.
fn restart_0376<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: u32,
    (ptr, imm, part, t, call): (u32, u32, u32, u32, u32),
    off: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    scene(g, img, f, ptr, imm);
    let d = g.read::<u32>(w(part));
    let key = g.read::<u32>(d + 0x10);
    // call width: 1
    let tb = g.call(slot_target(g, w(t), 941), 0, &[key]).eax;
    let program = g.read::<u32>(tb.wrapping_add(off));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + call), p, 0x1000, program, 1)
}

/// `magic_0376`'s glow part callback (0x10001260, 378 bytes; cdecl `(p)`;
/// an `ebp` /GS frame, `V` at `ebp - 0x14`, `U` at `ebp - 0x24`): `s =
/// (*G_T - 0x10a) / K`; `V` = slot 234 of the two points of the first or
/// the second pair (by whether `p` lies in the first 0x80 bytes of the
/// part table), `U = V s`, slot 248 `(p + 0x40, A, U)`; the view slots;
/// the part's data checked (slot 712) with the lock cleared after.
/// Returns 0 when slot 712 answers, else 1.
pub fn glow_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (v, u) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let p = fr.arg(g, 0);
    let t = g.read::<u32>(w(0x11)).wrapping_add(0xffff_fef6);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps, rounded as the original
    let s = div(t.cast_signed() as f32, f32_at(g, w(0x26)));
    let first = p.wrapping_sub(w(0x31)) & 0xffff_ff80 == 0;
    let (a, b, c, tb) = if first {
        (w(0x45), w(0x4a), w(0x61), 0x50)
    } else {
        (w(0x68), w(0x6d), w(0x84), 0x73)
    };
    // call width: 3
    g.call(slot_target(g, w(tb), 234), 0, &[v, b, a]);
    for i in 0..3 {
        put(g, u + 4 * i, mul(f32_at(g, v + 4 * i), s));
    }
    // call width: 3
    g.call(slot_target(g, w(0x9b), 248), 0, &[p + 0x40, c, u]);
    view_0376(
        g,
        img,
        f,
        p,
        [0xc9, 0xe0, 0xdb, 0xe6, 0xf2, 0xff, 0xfa, 0x105],
    );
    let (ok, l2) = check_712(g, img, f, p, (0x113, 0x118, 0x124, 0x129, 0x142));
    g.write::<u32>(l2, 0);
    Ret::eax(u32::from(ok == 0))
}

/// [`glow_0376`] (the `rel32`s: the cookie checks).
pub const GLOW_0376: Sig = Sig {
    len: 378,
    hash: 0x28a1_9738_353f_f016,
    calls: &[(0x15f, Callee::Any), (0x172, Callee::Any)],
    imports: &[],
};

/// `magic_0376`'s ring part callback (0x10001480, 301 bytes; cdecl
/// `(p)`): slot 237 `(*G_C, 11, 0, p + 0x40)`; the view slots; with the
/// clock `*G_T` in 0xa8..0xb1 the result is 1 at once; at 0xb2 the part
/// restarts (program `+0x38`) with its scale `+0x60..+0x68` 0.12 and
/// `+0x6c` 1.0; then the part's data is checked (slot 712) and the lock
/// cleared. Returns 0 when slot 712 answers, else 1.
pub fn ring_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x1c));
    // call width: 4
    g.call(slot_target(g, w(0xc), 237), 0, &[c, 0xb, 0, p + 0x40]);
    view_0376(
        g,
        img,
        f,
        p,
        [0x23, 0x37, 0x32, 0x3d, 0x49, 0x56, 0x51, 0x5c],
    );
    let clock = g.read::<u32>(w(0x6a));
    if clock.wrapping_sub(0xa8) <= 9 {
        return Ret::eax(1);
    }
    if clock == 0xb2 {
        restart_0376(g, img, f, p, (0x89, 0x8f, 0x94, 0x9c, 0xb7), 0x38);
        for o in [0x60u32, 0x64, 0x68] {
            g.write::<u32>(p + o, 0x3dfc_80ef);
        }
        g.write::<u32>(p + 0x6c, 0x3f80_0000);
    }
    let (ok, l2) = check_712(g, img, f, p, (0xdc, 0xe1, 0xed, 0xf2, 0x10c));
    g.write::<u32>(l2, 0);
    Ret::eax(u32::from(ok == 0))
}

/// [`ring_0376`] (the `rel32`: the part start).
pub const RING_0376: Sig = Sig {
    len: 301,
    hash: 0x57b8_d886_d564_05b5,
    calls: &[(0xb7, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// `magic_0376`'s pillar set-up (0x100015b0, 462 bytes; cdecl `(p)`, `i`
/// = `(p - G_TABLE) >> 7`): parts 0..3 restart with their programs
/// (`+0x1c`, `+0x20`, `+0x24`, `+0x1c`; counters `G_N0..3` cleared) at
/// their places `(x, -122, z)` with `+0x4c` 1.0; every part gets `+0x50..
/// +0x58` 0, `+0x5c` 1, `+0x6c` 1.0 and the scale `r / K * K' + K''`
/// (`+0x60..+0x68`). Returns the random's 15 bits.
pub fn pillar_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = fr.arg(g, 0);
    let i = p.wrapping_sub(w(0xa)).cast_signed() >> 7;
    // (the counter cleared, the scene operands, part, table, rel32,
    // program offset, x, z)
    let case = match i {
        0 => Some((
            0x16,
            (0x1b, 0x21, 0x26, 0x2e, 0x49),
            0x1c,
            0xc214_39ae,
            0x425d_ac50,
        )),
        1 => Some((
            0x6c,
            (0x66, 0x76, 0x7b, 0x83, 0x9e),
            0x20,
            0x4253_c651,
            0x42d8_d628,
        )),
        2 => Some((
            0xc1,
            (0xbb, 0xcb, 0xd0, 0xd8, 0xf3),
            0x24,
            0x4233_c651,
            0x4255_ac50,
        )),
        3 => Some((
            0x113,
            (0x10d, 0x11d, 0x122, 0x12a, 0x145),
            0x1c,
            0x421f_c651,
            0x42fa_d628,
        )),
        _ => None,
    };
    if let Some((n, ops, off, x, z)) = case {
        if i == 0 {
            g.write::<u32>(w(n), 0);
            restart_0376(g, img, f, p, ops, off);
        } else {
            let sp = g.read::<u32>(w(ops.0));
            g.write::<u32>(w(n), 0);
            g.write::<u32>(sp, w(ops.1));
            let d = g.read::<u32>(w(ops.2));
            let key = g.read::<u32>(d + 0x10);
            // call width: 1
            let tb = g.call(slot_target(g, w(ops.3), 941), 0, &[key]).eax;
            let program = g.read::<u32>(tb.wrapping_add(off));
            let at = img.rel32_target(f + ops.4);
            crate::opu_parts::start_part(g, img, at, p, 0x1000, program, 1);
        }
        g.write::<u32>(p + 0x40, x);
        g.write::<u32>(p + 0x48, z);
        g.write::<u32>(p + 0x4c, 0x3f80_0000);
        g.write::<u32>(p + 0x44, 0xc2f4_0000);
    }
    for o in [0x50u32, 0x54, 0x58] {
        g.write::<u32>(p + o, 0);
    }
    g.write::<u32>(p + 0x5c, 1);
    let t = slot_target(g, w(0x169), 324);
    // call width: 0
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    g.write::<u32>(p + 0x6c, 0x3f80_0000);
    #[allow(clippy::cast_precision_loss)] // below 2^15: exact
    let v = add(
        mul(div(r as f32, f32_at(g, w(0x1a8))), f32_at(g, w(0x1b0))),
        f32_at(g, w(0x1b8)),
    );
    for o in [0x60u32, 0x64, 0x68] {
        put(g, p + o, v);
    }
    Ret::eax(r)
}

/// [`pillar_0376`] (the `rel32`s: the part starts).
pub const PILLAR_0376: Sig = Sig {
    len: 462,
    hash: 0x1fd5_80d1_c3c6_7d1f,
    calls: &[
        (0x49, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x9e, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xf3, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x145, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// `magic_0376`'s pillar step (0x10001780, 348 bytes; cdecl `(p)`, `i` =
/// `(p - G_TABLE) >> 7`): parts 0..3 count their frames (`G_Ni`, `c` =
/// 0x20 - the count before); the fade `max(c * 10, -60)` for a negative
/// `c` (or `p`'s word outside the four), else 0; the view slots, slot 918
/// `(fade)`, the part's data checked (slot 712); then slot 918 `(0)` and
/// the lock cleared. Returns 0 when slot 712 answers, else 1.
pub fn pillar_step_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = fr.arg(g, 0);
    let i = p.wrapping_sub(w(0xb)).cast_signed() >> 7;
    // the counter's two operands (read, write)
    let counter = match i {
        0 => Some((0x17u32, 0x24u32)),
        1 => Some((0x30, 0x3d)),
        2 => Some((0x49, 0x56)),
        3 => Some((0x62, 0x6f)),
        _ => None,
    };
    let c = if let Some((rd, wr)) = counter {
        let n = g.read::<u32>(w(rd));
        g.write::<u32>(w(wr), n.wrapping_add(1));
        0x20u32.wrapping_sub(n)
    } else {
        p
    };
    let fade = if c.cast_signed() < 0 {
        c.cast_signed().wrapping_mul(10).max(-0x3c).cast_unsigned()
    } else {
        0
    };
    view_0376(
        g,
        img,
        f,
        p,
        [0x90, 0xa4, 0x9f, 0xaa, 0xb6, 0xc3, 0xbe, 0xc9],
    );
    // call width: 1
    g.call(slot_target(g, w(0xd6), 918), 0, &[fade]);
    let (ok, l2) = check_712(g, img, f, p, (0xe5, 0xea, 0xf6, 0xfb, 0x114));
    let t = slot_target(g, w(0x122), 918);
    if ok != 0 {
        // call width: 1
        g.call(t, 0, &[0]);
        let l = g.read::<u32>(w(0x133));
        g.write::<u32>(l, 0);
        return Ret::eax(0);
    }
    g.write::<u32>(l2, 0);
    // call width: 1
    g.call(t, 0, &[0]);
    Ret::eax(1)
}

/// [`pillar_step_0376`].
pub const PILLAR_STEP_0376: Sig = Sig {
    len: 348,
    hash: 0x06fe_134a_69cd_6ace,
    calls: &[],
    imports: &[],
};

/// `magic_0376`'s orb set-up (0x100018e0, 278 bytes; cdecl `(p)`, `i` =
/// `(p - G_TABLE) >> 7`): parts 0 and 1 restart (programs `+0x18`,
/// `+0x28`) with their places, `+0x54` and scales, then `+0x6c` 1.0,
/// `+0x5c` 1, `+0x58`/`+0x50` 0, `+0x4c` 1.0. Returns 1.
pub fn orb_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = fr.arg(g, 0);
    let i = p.wrapping_sub(w(0xa)).cast_signed() >> 7;
    let set = |g: &mut G, words: &[(u32, u32)]| {
        for &(o, v) in words {
            g.write::<u32>(p + o, v);
        }
    };
    match i {
        0 => {
            restart_0376(g, img, f, p, (0x16, 0x1c, 0x21, 0x29, 0x44), 0x18);
            set(
                g,
                &[
                    (0x40, 0xc1ed_c318),
                    (0x44, 0xc324_0000),
                    (0x48, 0x41a1_884c),
                    (0x54, 0),
                    (0x60, 0x3e20_403a),
                    (0x64, 0x3e20_403a),
                    (0x68, 0x3e20_403a),
                ],
            );
        }
        1 => {
            restart_0376(g, img, f, p, (0x85, 0x8b, 0x90, 0x98, 0xb3), 0x28);
            set(
                g,
                &[
                    (0x40, 0xbfdc_3183),
                    (0x44, 0xc2f4_0000),
                    (0x48, 0x4230_c426),
                    (0x54, 0x5000),
                    (0x60, 0x3dcf_dc16),
                    (0x64, 0x3dcf_dc16),
                    (0x68, 0x3dcf_dc16),
                ],
            );
        }
        _ => return Ret::eax(1),
    }
    set(
        g,
        &[
            (0x6c, 0x3f80_0000),
            (0x5c, 1),
            (0x58, 0),
            (0x50, 0),
            (0x4c, 0x3f80_0000),
        ],
    );
    Ret::eax(1)
}

/// [`orb_0376`] (the `rel32`s: the part starts).
pub const ORB_0376: Sig = Sig {
    len: 278,
    hash: 0x01c9_16aa_b4bc_950a,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xb3, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// `magic_0376`'s beam set-up (0x10002090, 348 bytes; cdecl `(p)`, `i` =
/// `(p - G_TABLE) >> 7`): parts 0..2 restart (programs `+0x8`, `+0xc`,
/// `+0x10`) at their places, part 0 with the scale 0.2955 and the others
/// 0.5805 (`+0x60..+0x68`); then `+0x4c` 1.0, `+0x50..+0x58` 0, `+0x5c`
/// 1, `+0x6c` 1.0. Returns the part start's `eax`, or `i`.
pub fn beam_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = fr.arg(g, 0);
    let i = p.wrapping_sub(w(0xa)).cast_signed() >> 7;
    let set = |g: &mut G, words: &[(u32, u32)]| {
        for &(o, v) in words {
            g.write::<u32>(p + o, v);
        }
    };
    let eax = match i {
        0 => {
            let eax = restart_0376(g, img, f, p, (0x16, 0x1c, 0x21, 0x29, 0x44), 0x8);
            set(
                g,
                &[
                    (0x40, 0xc33d_b863),
                    (0x44, 0xc284_0000),
                    (0x48, 0xc1b6_77b4),
                    (0x60, 0x3e97_4ba5),
                    (0x64, 0x3e97_4ba5),
                    (0x68, 0x3e97_4ba5),
                ],
            );
            eax
        }
        1 | 2 => {
            let (ops, off, place) = if i == 1 {
                (
                    (0x7d, 0x83, 0x88, 0x90, 0xab),
                    0xc,
                    [0xc2ef_70c6, 0xc228_0000, 0x41d1_884c],
                )
            } else {
                (
                    (0xd0, 0xd6, 0xdb, 0xe3, 0xfe),
                    0x10,
                    [0xc32f_b863, 0xc286_0000, 0x4240_c426],
                )
            };
            let eax = restart_0376(g, img, f, p, ops, off);
            set(g, &[(0x40, place[0]), (0x44, place[1]), (0x48, place[2])]);
            set(
                g,
                &[
                    (0x68, 0x3f14_9b95),
                    (0x64, 0x3f14_9b95),
                    (0x60, 0x3f14_9b95),
                ],
            );
            eax
        }
        _ => return Ret::eax(i.cast_unsigned()),
    };
    set(
        g,
        &[
            (0x4c, 0x3f80_0000),
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x6c, 0x3f80_0000),
        ],
    );
    Ret::eax(eax)
}

/// [`beam_0376`] (the `rel32`s: the part starts).
pub const BEAM_0376: Sig = Sig {
    len: 348,
    hash: 0xfe7e_2d33_86c2_40fb,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xab, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xfe, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// `magic_0376`'s spark (0x10001aa0, 507 bytes; internal: `ecx` = a
/// sprite system, the stack `(P)`): a sprite of it taken
/// ([`sprite_take`](crate::s4_misc::sprite_take) with the resource
/// `*G_R`; none: 0), its time `+0x28` 0x20, its place `P` plus
/// `(r / K - K1) K2 K3` per axis, its speeds `(r / K - K1) K2 K4`,
/// `+0x54` = `r / K * K1 + K5`, `+0x5c..+0x64` 0x12, 0x13, 0x10.
/// Returns the sprite.
pub fn spark_0376<G: Game>(g: &mut G, img: &Image, f: u32, fr: &Frame) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let take = Frame {
        at: img.rel32_target(f + 0xc),
        esp: fr.esp.wrapping_sub(0xc),
        ecx: fr.ecx,
        edx: g.read::<u32>(w(0x6)),
        ..Frame::default()
    };
    let s = crate::s4_misc::sprite_take(g, img, &take).eax;
    if s == 0 {
        return 0;
    }
    g.write::<u32>(s + 0x28, 0x20);
    let p = fr.arg(g, 0);
    for (i, (t, a, b, c, d)) in (0u32..).zip([
        (0x1cu32, 0x43u32, 0x50u32, 0x58u32, 0x60u32),
        (0x48, 0x8a, 0x92, 0x9a, 0xa2),
        (0x82, 0xcd, 0xd5, 0xdd, 0xe5),
    ]) {
        let r = rnd15(g, img, f, t);
        let v = mul(mul(sub(div(r, k(g, a)), k(g, b)), k(g, c)), k(g, d));
        put(g, s + 4 + 4 * i, add(v, f32_at(g, p + 4 * i)));
    }
    for (i, (t, a, b, c, d)) in (0u32..).zip([
        (0xc5u32, 0x110u32, 0x118u32, 0x120u32, 0x128u32),
        (0x108, 0x14e, 0x156, 0x15e, 0x166),
        (0x146, 0x18c, 0x194, 0x19c, 0x1a4),
    ]) {
        let r = rnd15(g, img, f, t);
        let v = mul(mul(sub(div(r, k(g, a)), k(g, b)), k(g, c)), k(g, d));
        put(g, s + 0x14 + 4 * i, v);
    }
    let r = rnd15(g, img, f, 0x184);
    let v = add(mul(div(r, k(g, 0x1c5)), k(g, 0x1cd)), k(g, 0x1d5));
    put(g, s + 0x54, v);
    for (i, v) in (0u32..).zip([0x12u32, 0x13, 0x10]) {
        g.write::<u32>(s + 0x5c + 4 * i, v);
    }
    s
}

/// [`spark_0376`] (the `rel32`: the sprite take).
pub const SPARK_0376: Sig = Sig {
    len: 507,
    hash: 0xeeee_1782_639c_eccb,
    calls: &[(0xc, Callee::Sig(&crate::s4_misc::SPRITE_TAKE))],
    imports: &[],
};

/// The sprites' colour words from the table below `G_C` (`+0x68` =
/// `G_C[-sp[0x28]]`, through `conv`) for the live sprites of `sys`.
fn sprite_colours<G: Game>(g: &mut G, sys: u32, table: u32, conv: impl Fn(u32) -> u32) {
    let mut sp = g.read::<u32>(sys + 4);
    let mut i = 0i32;
    while i < g.read::<i32>(sys) {
        if g.read::<u32>(sp) != 0 {
            let at = table.wrapping_sub(g.read::<u32>(sp + 0x28) << 2);
            let v = conv(g.read::<u32>(at));
            g.write::<u32>(sp + 0x68, v);
        }
        i += 1;
        sp = sp.wrapping_add(0x94);
    }
}

/// The sprites stepped ([`sprites_step`](crate::s6_misc::sprites_step))
/// and drawn ([`block_sprite_frames`](crate::s9_misc::block_sprite_frames),
/// `dt` in `xmm3`), the draw's answer into `*G_X` (operand `x`).
#[allow(clippy::too_many_arguments)] // the original's operands
fn sprites_draw<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    sys: u32,
    esp: u32,
    (step, frames, x): (u32, u32, u32),
    dt: f32,
) {
    let st = Frame {
        at: img.rel32_target(f + step),
        esp: esp.wrapping_sub(4),
        ecx: sys,
        ..Frame::default()
    };
    crate::s6_misc::sprites_step(g, img, &st);
    let bf = Frame {
        at: img.rel32_target(f + frames),
        esp: esp.wrapping_sub(8),
        ecx: sys,
        xmm: [0, 0, 0, crate::Xmm::f32(dt).reg()],
        ..Frame::default()
    };
    let r = crate::s9_misc::block_sprite_frames(g, img, &bf).eax;
    g.write::<u32>(img.word(f + x), r);
}

/// `magic_0376`'s embers (0x10001f20, 198 bytes; cdecl `(sys)`; an `ebp`
/// /GS frame, `P` at `ebp - 0x14`): `P` = slot 237 `(*G_C, 10, 0, P)`;
/// every fifth clock tick below 0x235 a spark ([`spark_0376`]); the live
/// sprites' colours from the table; the sprites stepped and drawn (`dt`
/// `K`). Returns 1.
pub fn embers_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let pt = ebp.wrapping_sub(0x14);
    let sys = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x2a));
    // call width: 4
    g.call(slot_target(g, w(0x1a), 237), 0, &[c, 0xa, 0, pt]);
    let clock = g.read::<i32>(w(0x32));
    let regs = ebp.wrapping_sub(0x14 + 8);
    if clock < 0x235 && clock % 5 == 0 {
        g.write::<u32>(regs.wrapping_sub(4), pt);
        let sp = Frame {
            esp: regs.wrapping_sub(8),
            ecx: sys,
            ..Frame::default()
        };
        spark_0376(g, img, img.rel32_target(f + 0x5f), &sp);
    }
    sprite_colours(g, sys, w(0x7c), |v| v);
    let dt = f32_at(g, w(0x9d));
    sprites_draw(g, img, f, sys, regs, (0x95, 0xa5, 0xb0), dt);
    Ret::eax(1)
}

/// [`embers_0376`] (the `rel32`s: the spark, the step, the frames, the
/// cookie check).
pub const EMBERS_0376: Sig = Sig {
    len: 198,
    hash: 0xa3a1_7c52_1020_6acb,
    calls: &[
        (0x5f, Callee::Sig(&SPARK_0376)),
        (0x95, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0xa5, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES_B)),
        (0xbe, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0376`'s burst (0x10001d00, 437 bytes; cdecl `(sys)`; an `ebp`
/// /GS frame, `P` at `ebp - 0x14`): `P` = slot 237 `(*G_C, 0x85, 0, P)`;
/// while the clock is below 100 a spark ([`spark_0376`]) with its `+0x54`
/// times `K`, its place `P + (r / K1 - K) K2 K2` per axis and its colour
/// steps 0x46, 0x50, 0x5a; the live sprites' colours from the table times
/// a double; the sprites stepped and drawn (`dt` 0). Returns 1.
pub fn burst_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32_at(g, w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let pt = ebp.wrapping_sub(0x14);
    let sys = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x2d));
    // call width: 4
    g.call(slot_target(g, w(0x1a), 237), 0, &[c, 0x85, 0, pt]);
    let regs = ebp.wrapping_sub(0x14 + 8);
    if g.read::<i32>(w(0x38)) < 0x64 {
        g.write::<u32>(regs.wrapping_sub(4), pt);
        let sp = Frame {
            esp: regs.wrapping_sub(8),
            ecx: sys,
            ..Frame::default()
        };
        let s = spark_0376(g, img, img.rel32_target(f + 0x4a), &sp);
        put(g, s + 0x54, mul(f32_at(g, s + 0x54), k(g, 0x62)));
        for (i, (t, a, b, c, d)) in (0u32..).zip([
            (0x50u32, 0x88u32, 0x90u32, 0x98u32, 0xa0u32),
            (0x80, 0xcb, 0xd3, 0xdb, 0xe3),
            (0xc3, 0x109, 0x111, 0x119, 0x121),
        ]) {
            let r = rnd15(g, img, f, t);
            let v = mul(mul(sub(div(r, k(g, a)), k(g, b)), k(g, c)), k(g, d));
            put(g, s + 4 + 4 * i, add(v, f32_at(g, pt + 4 * i)));
        }
        for (i, v) in (0u32..).zip([0x46u32, 0x50, 0x5a]) {
            g.write::<u32>(s + 0x5c + 4 * i, v);
        }
    }
    let scale = g.read::<f64>(w(0x151));
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    sprite_colours(g, sys, w(0x161), |v| {
        ((f64::from(f32::from_bits(v)) * scale) as f32).to_bits()
    });
    sprites_draw(g, img, f, sys, regs, (0x189, 0x194, 0x19f), 0.0);
    Ret::eax(1)
}

/// [`burst_0376`] (the `rel32`s: the spark, the step, the frames, the
/// cookie check).
pub const BURST_0376: Sig = Sig {
    len: 437,
    hash: 0xf566_1f67_2dc5_2882,
    calls: &[
        (0x4a, Callee::Sig(&SPARK_0376)),
        (0x189, Callee::Sig(&crate::s6_misc::SPRITES_STEP)),
        (0x194, Callee::Sig(&crate::s9_misc::BLOCK_SPRITE_FRAMES_B)),
        (0x1ad, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0376`'s start (0x10002330, 774 bytes; cdecl `(a)`): a 16 MiB
/// buffer (slot 803) cut into globals (0x100000, 0x100000, 0x80000 apart,
/// the last twice and `+0x39d0`); `G_A = a`; the resource set and loaded
/// (slots 718, 222); the part `*G_P` = slot 719 `(*G_N, 8, 0)` set up by
/// slot 711 with the game lock 1 and slot 710; three tables of slot 941;
/// the draw record (0x30 bytes, `u16 +2` 0x80) and four 0x110-byte
/// blocks cleared (`memset`) and given their vtables and flag words;
/// `G_X` = slot 260 `(0)`, `G_Y` = slot 260 `(0x17)`. Returns 1.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_0376<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // call width: 2
    let buf = g.call(slot(g, 0x4, 803), 0, &[0x0100_0000, 0x10]).eax;
    g.write::<u32>(w(0x19), buf);
    let x = buf.wrapping_add(0x10_0000);
    g.write::<u32>(w(0x23), x);
    let x = x.wrapping_add(0x10_0000);
    g.write::<u32>(w(0x2d), x);
    let x = x.wrapping_add(0x8_0000);
    g.write::<u32>(w(0x37), x);
    g.write::<u32>(w(0x3c), x);
    g.write::<u32>(w(0x46), x.wrapping_add(0x39d0));
    g.write::<u32>(w(0x4e), fr.arg(g, 0));
    stores(g, img, f, &[(0x64, 0x68)]);
    // call width: 1
    g.call(slot(g, 0x53, 718), 0, &[w(0x58)]);
    let n = g.read::<u32>(w(0x87));
    // call width: 4
    g.call(slot(g, 0x6f, 222), 0, &[n, 0, 0x3e8, 0x2000]);
    let n = g.read::<u32>(w(0x9e));
    // call width: 3
    let h = g.call(slot(g, 0x8e, 719), 0, &[n, 8, 0]).eax;
    let lock = g.read::<u32>(w(0xa6));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xaf), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(slot(g, 0xba, 711), 0, &[h, w(0xb4), 0]);
    let lock = g.read::<u32>(w(0xcd));
    let b = g.read::<u32>(w(0xd8));
    g.write::<u32>(lock, saved);
    let p = g.read::<u32>(w(0xe0));
    // call width: 4
    g.call(slot(g, 0xe5, 710), 0, &[w(0xea), p, b, 0x10_0000]);
    for (src, off, tb, dst) in [
        (0xf7u32, 0x14u32, 0x102u32, 0x10fu32),
        (0x114, 0x18, 0x11c, 0x129),
        (0x12e, 0x1c, 0x136, 0x14c),
    ] {
        let part = g.read::<u32>(w(src));
        let key = g.read::<u32>(part + off);
        // call width: 1
        let t = g.call(slot(g, tb, 941), 0, &[key]).eax;
        g.write::<u32>(w(dst), t);
    }
    g.write_bytes(w(0x147), &[0; 0x30]); // memset(G_D, 0, 0x30)
    g.write::<u16>(w(0x15c), 0x80);
    g.write_bytes(w(0x168), &[0; 0x110]); // memset(G_B0, 0, 0x110)
    stores(g, img, f, &[(0x17f, 0x183)]);
    sets(
        g,
        img,
        f,
        &[
            (0x189, 8),
            (0x193, 8),
            (0x19d, 1),
            (0x1a7, 1),
            (0x1b1, 1),
            (0x1bb, 1),
            (0x1c5, 1),
            (0x1cf, 1),
            (0x1d9, 1),
            (0x1e3, 1),
        ],
    );
    g.write_bytes(w(0x179), &[0; 0x110]); // memset(G_B1, 0, 0x110)
    stores(g, img, f, &[(0x1fe, 0x202)]);
    sets(g, img, f, &[(0x208, 1), (0x212, 1), (0x21c, 1)]);
    g.write_bytes(w(0x1f8), &[0; 0x110]); // memset(G_B2, 0, 0x110)
    stores(g, img, f, &[(0x237, 0x23b)]);
    sets(g, img, f, &[(0x241, 1), (0x24b, 1), (0x255, 1)]);
    g.write_bytes(w(0x231), &[0; 0x110]); // memset(G_B3, 0, 0x110)
    stores(g, img, f, &[(0x26c, 0x270)]);
    sets(
        g,
        img,
        f,
        &[
            (0x276, 8),
            (0x280, 8),
            (0x28a, 1),
            (0x294, 1),
            (0x29e, 1),
            (0x2a8, 1),
            (0x2b2, 1),
            (0x2bc, 1),
            (0x2c6, 1),
            (0x2d0, 1),
        ],
    );
    // call width: 1
    let x = g.call(slot(g, 0x263, 260), 0, &[0]).eax;
    g.write::<u32>(w(0x2e3), x);
    // call width: 1
    let y = g.call(slot(g, 0x2e8, 260), 0, &[0x17]).eax;
    g.write::<u32>(w(0x2fa), y);
    Ret::eax(1)
}

/// [`start_0376`] (the `rel32`s: `memset`).
pub const START_0376: Sig = Sig {
    len: 774,
    hash: 0x25b9_7b71_5090_1ee6,
    calls: &[
        (0x151, Callee::Import("memset")),
        (0x16d, Callee::Import("memset")),
        (0x1ec, Callee::Import("memset")),
        (0x225, Callee::Import("memset")),
        (0x25e, Callee::Import("memset")),
    ],
    imports: &[],
};
