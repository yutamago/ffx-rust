//! Round 19's ports (magic-r19): the summon state machines and the own
//! rows of overlays below `magic_0450` with only a few functions left,
//! written out for their DLLs.

use crate::r11_fades::{stores, zeros};
use crate::runtime::slot_target;
use crate::sb_misc::node_append;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `dst = K - src` (`movaps xmm0, xmm1; subss`) for each `(src, dst)`
/// operand pair, `K` the float given.
fn k_minus<G: Game>(g: &mut G, img: &Image, f: u32, k: f32, pairs: &[(u32, u32)]) {
    for &(src, dst) in pairs {
        let v = sub(k, g.read::<f32>(img.word(f + src)));
        g.write::<f32>(img.word(f + dst), v);
    }
}

/// `or dword [G], 1` with `G` at the operand `op` of `f`.
fn or1<G: Game>(g: &mut G, img: &Image, f: u32, op: u32) {
    let at = img.word(f + op);
    let v = g.read::<u32>(at) | 1;
    g.write::<u32>(at, v);
}

/// The task's node `*task` leaves its ring (next's `+8` the old prev,
/// prev's `+4` the next, the node linked to itself). Returns the next as
/// read the second time.
fn ring_leave<G: Game>(g: &mut G, task: u32) -> u32 {
    let node = g.read::<u32>(task);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    next
}

/// A particle node and its effect node as the summon tasks call them
/// (`push b; sub esp, 8; push node; call`, twice, the return address at
/// `esp`): [`particle_node_with`](crate::s3_nodes::particle_node_with)
/// with `ecx` the operand `pc`, the flag byte at the operand `byte` set
/// before it, then [`effect_node_b`](crate::s4_fam446::effect_node_b)
/// with `ecx` the operand `ec`; each gets the node at the operand of its
/// own push. Returns the effect node's `eax`.
fn particle_effect<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    esp: u32,
    (pc, byte, pn, pb, pcall): (u32, u32, u32, u32, u32),
    (ec, en, eb, ecall): (u32, u32, u32, u32),
) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(esp + 0x10, pb);
    g.write::<u32>(esp + 4, w(pn));
    g.write::<u8>(w(byte), 1);
    let p = Frame {
        at: img.rel32_target(f + pcall),
        esp,
        ecx: w(pc),
        ..*fr
    };
    crate::s3_nodes::particle_node_with(g, img, &p);
    g.write::<u32>(esp + 0x10, eb);
    g.write::<u32>(esp + 4, w(en));
    let e = Frame {
        at: img.rel32_target(f + ecall),
        esp,
        ecx: w(ec),
        ..*fr
    };
    crate::s4_fam446::effect_node_b(g, img, &e).eax
}

/// `magic_0343`'s summon task (0x10006ad0, 1845 bytes; cdecl `(task)`, the
/// record `R = task[1]`; an 8-aligned frame `A`, the group `L` at `S +
/// 0xc`, `S = A - 0x28` after the pushes). By the state `R + 0x18`:
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)
///   `[NODE, LIST, *G_A, 0, 1]`), the summon's model node
///   ([`model_node_init`](crate::sa_summon::model_node_init) `[NODE2,
///   LIST, *G_B, 0]`), the colour node
///   ([`colour_node`](crate::s3_misc::colour_node)), the static model's
///   flags ([`static_flag_slots`](crate::o4_misc::static_flag_slots));
///   state 10;
/// - 10: the camera block's `K - x` differences and its 0.5 scales, the
///   node `C1` appended, slot 237 `(*G_C, 1, 0, C1 + 0x1c)`, `C2`
///   appended (colour 1, 1, 1, 0.26), the model node
///   ([`node_insert`](crate::nodes::node_insert)), its particle and
///   effect nodes; state 30;
/// - 30: from a count of 170, slot 228 `(*G_B, 0x4a)` and slot 222
///   (the sound); state 60;
/// - 60: unless slot 313 `(2)` answers, two more nodes (slot 237 through
///   [`slot237_arg`](crate::o4_tiny::slot237_arg)), a model node, its
///   particle and effect nodes; state 50;
/// - 50: from a count of 280, [`vector_set`](crate::o4_misc::vector_set)
///   with `(0, 30, 0, 0)`; state 51;
/// - 51: from a count of 330, a node at the model's position
///   ([`model_position`](crate::m0070::model_position), `y` less `K3`),
///   another, the model node
///   ([`insert_with`](crate::sa_misc::insert_with)), its particle and
///   effect nodes; state 52;
/// - 52: from a count of 90, the camera block again (1.0 scales, flags
///   bit 0 off and bit 2 on); state 53;
/// - 53: from a count of 120, the character's fade
///   ([`chr_fade`](crate::l3_common::chr_fade) `[NODE2, 0.0, 30]`); state
///   10000;
/// - 10000: from a count of 30, the current list node unlinked
///   ([`list_unlink_current`](crate::l3_common::list_unlink_current)), the
///   model set up ([`model_setup`](crate::sa_summon::model_setup)), the
///   task's node off its ring.
///
/// The states that move on (but 0, 52 and 10000) reset the count
/// `R + 0x2c`; every exit counts `R + 0x28` and `R + 0x2c`. Returns the
/// last call's `eax`, the state, the flags word, 0 or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0343<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x20; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let l = s + 0xc;
    // the calls with four pushed words: the return address at S - 0x14
    let esp4 = s.wrapping_sub(0x14);
    let state = g.read::<u32>(r + 0x18);
    let count = g.read::<i32>(r + 0x2c);
    let set_l = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(l + 4 * i, v);
        }
    };
    let at = |o: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    // (eax, whether the count `+0x2c` restarts)
    let (eax, restart) = match state {
        0 => {
            let a = g.read::<u32>(w(0x283));
            set_l(g, &[w(0x28f), w(0x297), a, 0]);
            g.write::<u8>(l + 0x10, 1);
            stores(g, img, f, &[0x2ac]);
            crate::s3_summon::model_nodes(g, img, &at(0x2b7));
            let b = g.read::<u32>(w(0x2bc));
            set_l(g, &[w(0x2c8), w(0x2d0), b]);
            g.write::<u8>(l + 0xc, 0);
            stores(g, img, f, &[0x2dd]);
            crate::sa_summon::model_node_init(g, img, call(0x2e8), l);
            set_l(g, &[w(0x2f4), w(0x2fc)]);
            stores(g, img, f, &[0x300]);
            crate::s3_misc::colour_node(g, img, &at(0x30b));
            let eax = crate::o4_misc::static_flag_slots(g, img, &at(0x310)).eax;
            g.write::<u32>(r + 0x18, 0xa);
            (eax, false)
        }
        10 => {
            let k = kf(g, 0x91);
            let flags = g.read::<u32>(w(0xa2)) & !1;
            g.write::<u32>(w(0xa2), flags);
            stores(g, img, f, &[0xb1]);
            k_minus(g, img, f, k, &[(0x9c, 0xbf)]);
            stores(g, img, f, &[0xce, 0xd8, 0xe2, 0xec]);
            k_minus(g, img, f, k, &[(0xca, 0xfa)]);
            stores(g, img, f, &[0x109, 0x113, 0x11d, 0x127]);
            k_minus(
                g,
                img,
                f,
                k,
                &[(0x105, 0x135), (0x140, 0x148), (0x153, 0x15b)],
            );
            let x = sub(k, kf(g, 0x166));
            let y = sub(k, kf(g, 0x16e));
            g.write::<f32>(w(0x176), x);
            let z = sub(kf(g, 0x17e), kf(g, 0x186));
            g.write::<f32>(w(0x18e), y);
            g.write::<f32>(w(0x196), z);
            node_append(g, img, call(0x19b), w(0xad), w(0xa8), state);
            let c = g.read::<u32>(w(0x1b5));
            // call width: 4
            let e = g
                .call(slot_target(g, w(0x1a0), 0x3b4 / 4), 0, &[c, 1, 0, w(0x1a5)])
                .eax;
            stores(g, img, f, &[0x1c8]);
            node_append(g, img, call(0x1d3), w(0x1c4), w(0x1bf), e);
            g.write::<u32>(s + 0x20, w(0x1c4));
            stores(g, img, f, &[0x1df, 0x1e9, 0x1f3, 0x1fd]);
            set_l(g, &[w(0x20b), w(0x1bf), w(0x217), w(0x21f), w(0x227)]);
            crate::nodes::node_insert(g, img, &at(0x22c));
            let eax = particle_effect(
                g,
                img,
                fr,
                esp4,
                (0x236, 0x241, 0x23b, 1, 0x247),
                (0x24f, 0x259, 0, 0x25e),
            );
            or1(g, img, f, 0x267);
            zeros(g, img, f, &[0x26c]);
            g.write::<u32>(r + 0x18, 0x1e);
            (eax, true)
        }
        0x1e => {
            if count < 0xaa {
                (state, false)
            } else {
                let tab = w(0x52);
                let b = g.read::<u32>(w(0x5a));
                g.call(slot_target(g, tab, 0x390 / 4), 0, &[b, 0x4a]);
                // call width: 6
                let eax = g
                    .call(
                        slot_target(g, w(0x67), 0x378 / 4),
                        0,
                        &[w(0x78), 8, 0xa, 2, b, 0x4a],
                    )
                    .eax;
                g.write::<u32>(r + 0x18, 0x3c);
                (eax, true)
            }
        }
        0x3c => {
            if count < 0 {
                (state, false)
            } else {
                let e = g.call(slot_target(g, w(0x623), 0x4e4 / 4), 0, &[2]).eax;
                if e != 0 {
                    (e, false)
                } else {
                    let e = node_append(g, img, call(0x647), w(0x642), w(0x63d), e);
                    // push P: the return address at S - 8
                    g.write::<u32>(s - 4, w(0x64c));
                    let a = Frame {
                        at: call(0x651),
                        esp: s - 8,
                        eax: e,
                        ..*fr
                    };
                    let e = crate::o4_tiny::slot237_arg(g, img, &a).eax;
                    stores(g, img, f, &[0x662]);
                    node_append(g, img, call(0x66d), w(0x65e), w(0x659), e);
                    g.write::<u32>(s + 0x20, w(0x65e));
                    stores(g, img, f, &[0x679, 0x683, 0x68d, 0x697]);
                    set_l(g, &[w(0x6a5), w(0x659), w(0x6b1), w(0x6b9), w(0x6c1)]);
                    crate::nodes::node_insert(g, img, &at(0x6c6));
                    let eax = particle_effect(
                        g,
                        img,
                        fr,
                        esp4,
                        (0x6d0, 0x6db, 0x6d5, 0, 0x6e1),
                        (0x6e9, 0x6f3, 1, 0x6f8),
                    );
                    or1(g, img, f, 0x701);
                    zeros(g, img, f, &[0x706]);
                    g.write::<u32>(r + 0x18, 0x32);
                    (eax, true)
                }
            }
        }
        0x32 => {
            if g.read::<i32>(r + 0x28) < 0x118 {
                (state, false)
            } else {
                set_l(g, &[0, 0x41f0_0000, 0, 0]);
                let v = Frame {
                    at: call(0x35a),
                    esp: s - 4,
                    eax: state,
                    edx: l,
                    xmm: [0, 0, u128::from(g.read::<u32>(w(0x331))), 0],
                    ..*fr
                };
                let eax = crate::o4_misc::vector_set(g, img, &v).eax;
                g.write::<u32>(r + 0x18, state + 1);
                (eax, true)
            }
        }
        0x33 => {
            if g.read::<i32>(r + 0x28) < 0x14a {
                (0, false)
            } else {
                node_append(g, img, call(0x4c0), w(0x4bb), w(0x4b6), 0);
                let e = crate::m0070::model_position(g, img, call(0x4ca), w(0x4c5));
                let y = sub(kf(g, 0x4d2), kf(g, 0x4da));
                stores(g, img, f, &[0x4e8]);
                g.write::<f32>(w(0x4f6), y);
                node_append(g, img, call(0x4fb), w(0x4e4), w(0x4df), e);
                for (i, v) in (1u32..).zip([w(0x50b), w(0x506), w(0x501), w(0x4e4)]) {
                    g.write::<u32>(esp4 + 4 * i, v);
                }
                stores(g, img, f, &[0x514, 0x51e, 0x528, 0x532]);
                let iw = Frame {
                    at: call(0x53d),
                    esp: esp4,
                    ecx: w(0x510),
                    edx: w(0x4df),
                    ..*fr
                };
                crate::sa_misc::insert_with(g, img, &iw);
                let eax = particle_effect(
                    g,
                    img,
                    fr,
                    esp4,
                    (0x545, 0x550, 0x556, 2, 0x55b),
                    (0x563, 0x56d, 2, 0x572),
                );
                or1(g, img, f, 0x578);
                zeros(g, img, f, &[0x580]);
                g.write::<u32>(r + 0x18, state + 1);
                (eax, true)
            }
        }
        0x34 => {
            if count < 0x5a {
                (0, false)
            } else {
                let k = kf(g, 0x393);
                let x = sub(k, kf(g, 0x39e));
                g.write::<u32>(w(0x3a3), 0);
                let flags = g.read::<u32>(w(0x3a8)) & !1 | 4;
                g.write::<f32>(w(0x3b6), x);
                let x = sub(k, kf(g, 0x3c1));
                stores(g, img, f, &[0x3c5, 0x3cf, 0x3d9, 0x3e3]);
                g.write::<f32>(w(0x3f1), x);
                let x = sub(k, kf(g, 0x3fc));
                stores(g, img, f, &[0x400, 0x40a, 0x414, 0x41e]);
                g.write::<f32>(w(0x42c), x);
                let x = sub(k, kf(g, 0x437));
                g.write::<u32>(w(0x43c), flags);
                stores(g, img, f, &[0x440]);
                g.write::<f32>(w(0x44e), x);
                k_minus(g, img, f, k, &[(0x459, 0x461)]);
                let x = sub(k, kf(g, 0x46c));
                let y = sub(k, kf(g, 0x474));
                g.write::<f32>(w(0x47c), x);
                let z = sub(kf(g, 0x484), kf(g, 0x48c));
                g.write::<f32>(w(0x494), y);
                g.write::<f32>(w(0x49c), z);
                g.write::<u32>(r + 0x18, state + 1);
                (flags, false)
            }
        }
        0x35 => {
            if count < 0x78 {
                (state, false)
            } else {
                set_l(g, &[w(0x5a4), 0, 0x1e]);
                let eax = crate::l3_common::chr_fade(g, img, call(0x5b9), l);
                g.write::<u32>(r + 0x18, 0x2710);
                (eax, true)
            }
        }
        0x2710 => {
            if count < 0x1e {
                (state, false)
            } else {
                crate::l3_common::list_unlink_current(g, img, call(0x5e4), state);
                g.write::<u32>(l, w(0x5f0));
                crate::sa_summon::model_setup(g, img, &at(0x5f5));
                (ring_leave(g, task), false)
            }
        }
        _ => (state, false),
    };
    if restart {
        g.write::<u32>(r + 0x2c, 0);
    }
    for o in [0x28u32, 0x2c] {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    }
    Ret::eax(eax)
}

/// [`summon_task_0343`] (1845 bytes, 1 overlay).
pub const SUMMON_TASK_0343: Sig = Sig {
    len: 1845,
    hash: 0xacb2_f406_90b1_c4eb,
    calls: &[
        (0x19b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1d3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x22c, Callee::Sig(&crate::nodes::INSERT)),
        (0x247, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x25e, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x2b7, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x2e8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x30b, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (
            0x310,
            Callee::Sig(&crate::o4_misc::STATIC_FLAG_SLOTS_0343_6900),
        ),
        (0x35a, Callee::Sig(&crate::o4_misc::VECTOR_SET_0343_69E0)),
        (0x4c0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4ca, Callee::Sig(&crate::m0070::MODEL_POSITION)),
        (0x4fb, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x53d, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x55b, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x572, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x5b9, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x5e4, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x5f5, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x647, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x651, Callee::Sig(&crate::o4_tiny::SLOT237_ARG_0343_6970)),
        (0x66d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6c6, Callee::Sig(&crate::nodes::INSERT)),
        (0x6e1, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x6f8, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x72d, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0202`'s summon task (0x10006f70, 2024 bytes with its jump table;
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `A`, `S =
/// A - 0x80` after the pushes: the group `L` at `S + 0xc`, the group
/// `L2` at `S + 0x44`, the vectors `V` at `S + 0x5c` and `W` at `S +
/// 0x6c`, the distance at `S + 0x30`). By the state `R + 0x18`:
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)
///   `[NODE, LIST, *G_A, 0, 1]`), the summon's model node
///   ([`model_node_init`](crate::sa_summon::model_node_init) `[OBJ, LIST,
///   *G_B, 0]`), a node appended at the object's position
///   ([`model_position`](crate::sa_misc::model_position), `w` 1.0) and its
///   effect ([`effect_ctor_arg2`](crate::o4_ctor::effect_ctor_arg2)) with
///   its flag set and word cleared; state 10;
/// - 10: the object's flag step ([`flag_slots`](crate::o4_fam2::flag_slots));
///   state 11;
/// - 11: from a count of 26, a node at the object's facing
///   ([`facing_int`](crate::sa_misc::facing_int)), two model nodes
///   ([`node_insert`](crate::nodes::node_insert)) with their nodes, a
///   particle node, every target's flag `R + 0x30 + 4i` set, a node
///   appended ([`summon_append`](crate::r17::summon_append)) and a third
///   model node; state 12;
/// - 12: state 13, the counts 0 and 3; then as 13;
/// - 13: each target `i` still flagged whose position (slot 262, its
///   ground slot 12, less `P`, slot 232 on it and on `Q`) is within `K`
///   (slot 394): the nodes `B_i` (its heading,
///   [`heading_to`](crate::s4_small::heading_to), `+0x20` less 0x2000),
///   `C_i` (at the target's position less `K2`) and `D_i` (colour 1, 1,
///   1, 0.7), the model node `M_i`, the particle node `P_i`
///   ([`particle_node`](crate::sb_nodes::particle_node)), slot 239
///   `(*G_C, id, 0)`, the flag cleared; with no flag left the next state;
///   a word less 0x1000;
/// - 14: slot 20 `(*G_D)` with the flag byte's bit 0, else slot 411
///   `(*G_E)`; when it answers, state 10000 at once;
/// - 10000: the object leaves its parent
///   ([`leave`](crate::sa_misc::leave)), the model set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the task's node off
///   its ring.
///
/// Every exit counts `R + 0x2c` (10 and 11 restart it at 1). Returns the
/// last call's `eax`, the target count, the state (less 11 from 11 on), 0
/// or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0202<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r17::summon_append;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x74; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x80);
    let l = s + 0xc;
    let tab = w(0x5c);
    let bump = |g: &mut G, o: u32| {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    };
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let on = |o: u32, ecx: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx,
        ..*fr
    };
    let mut state = g.read::<u32>(r + 0x18);
    if state == 14 {
        let e = if g.read::<u8>(w(0x56)) & 1 != 0 {
            let d = g.read::<u32>(w(0x64));
            g.call(slot_target(g, tab, 0x50 / 4), 0, &[d]).eax
        } else {
            let e = g.read::<u32>(w(0x6f));
            g.call(slot_target(g, tab, 0x66c / 4), 0, &[e]).eax
        };
        if e == 0 {
            bump(g, 0x2c);
            return Ret::eax(0);
        }
        g.write::<u32>(r + 0x18, 0x2710);
        state = 0x2710;
    }
    match state {
        0 => {
            let a = g.read::<u32>(w(0x6dd));
            set(g, l, &[w(0x6e9), w(0x6f1), a, 0]);
            g.write::<u8>(l + 0x10, 1);
            stores(g, img, f, &[0x706]);
            crate::s3_summon::model_nodes(g, img, &on(0x711, l));
            let b = g.read::<u32>(w(0x716));
            set(g, l, &[w(0x722), w(0x72a), b]);
            g.write::<u8>(l + 0xc, 0);
            stores(g, img, f, &[0x737]);
            crate::sa_summon::model_node_init(g, img, call(0x742), l);
            node_append(g, img, call(0x751), w(0x74c), w(0x747), 0);
            crate::sa_misc::model_position(g, img, call(0x760), w(0x75b), w(0x756));
            stores(g, img, f, &[0x767]);
            // sub esp, 8; push N; push ecx (slot 12's left-over, never
            // read: 0 here): the return address at S - 0x14
            let esp = s.wrapping_sub(0x14);
            set(g, esp + 4, &[0, w(0x772)]);
            let ec = Frame {
                at: call(0x778),
                esp,
                ..*fr
            };
            let eax = crate::o4_ctor::effect_ctor_arg2(g, img, &ec).eax;
            or1(g, img, f, 0x77e);
            zeros(g, img, f, &[0x783]);
            bump(g, 0x2c);
            g.write::<u32>(r + 0x18, 0xa);
            Ret::eax(eax)
        }
        10 => {
            let eax = crate::o4_fam2::flag_slots(g, img, call(0x7b2), w(0x7ad));
            bump(g, 0x18);
            g.write::<u32>(r + 0x2c, 1);
            Ret::eax(eax)
        }
        11 => {
            if g.read::<i32>(r + 0x2c) < 0x1a {
                bump(g, 0x2c);
                return Ret::eax(0);
            }
            crate::sa_misc::node_append(g, img, call(0xa9), w(0xa4), w(0x9f), 0);
            crate::sa_misc::facing_int(g, img, call(0xb8), w(0xb3), w(0xae), s - 4);
            set(
                g,
                l,
                &[w(0xc4), w(0xcc), w(0xd4), w(0xdc), w(0xe4), w(0xec)],
            );
            crate::nodes::node_insert(g, img, &on(0xf1, l));
            node_append(g, img, call(0x100), w(0xfb), w(0xf6), 0);
            zeros(g, img, f, &[0x109]);
            stores(g, img, f, &[0x113, 0x11d, 0x127]);
            node_append(g, img, call(0x132), w(0x105), w(0xf6), 0);
            g.write::<u32>(s + 0x20, w(0x105));
            stores(g, img, f, &[0x13e, 0x148, 0x152, 0x15c]);
            set(g, l, &[w(0x16a), w(0xf6), w(0x176), w(0x17e), w(0x186)]);
            crate::nodes::node_insert(g, img, &on(0x18b, l));
            // push 0; sub esp, 8; push N: the return address at S - 0x14
            let esp = s.wrapping_sub(0x14);
            g.write::<u32>(esp + 0x10, 0);
            g.write::<u32>(esp + 4, w(0x19a));
            g.write::<u8>(w(0x1a0), 1);
            let pn = Frame {
                at: call(0x1a6),
                esp,
                ecx: w(0x195),
                ..*fr
            };
            crate::s3_nodes::particle_node_with(g, img, &pn);
            let mut i = 0u32;
            while i.cast_signed() < g.read::<i32>(w(0x1b1)) {
                g.write::<u32>(r + 0x30 + 4 * i, 1);
                i += 1;
            }
            summon_append(
                g,
                img,
                f,
                w(0x1d9),
                [
                    0x204, 0x20e, 0x1d3, 0x1d3, 0x218, 0x220, 0x234, 0x23e, 0x1dd,
                ],
            );
            zeros(g, img, f, &[0x246, 0x250, 0x25a]);
            set(
                g,
                l,
                &[w(0x268), w(0x270), w(0x278), w(0x280), w(0x288), w(0x290)],
            );
            let eax = crate::nodes::node_insert(g, img, &on(0x295, l)).eax;
            g.write::<u8>(w(0x29b), 1);
            bump(g, 0x18);
            g.write::<u32>(r + 0x2c, 1);
            Ret::eax(eax)
        }
        12 | 13 => {
            let mut eax = state - 0xb;
            if state == 12 {
                bump(g, 0x18);
                g.write::<u32>(r + 0x28, 0);
                g.write::<u32>(r + 0x2c, 3);
            }
            let n = |g: &G| g.read::<i32>(w(0x2b8));
            let mut acc = 0u32;
            if n(g) > 0 {
                let mut i = 0u32;
                loop {
                    acc |= target_0202(g, img, fr, s, i);
                    i += 1;
                    eax = i;
                    if i.cast_signed() >= n(g) {
                        break;
                    }
                }
            }
            if acc == 0 {
                bump(g, 0x18);
            }
            let at = w(0x66b);
            let x = g.read::<u32>(at).wrapping_sub(0x1000);
            g.write::<u32>(at, x);
            bump(g, 0x2c);
            Ret::eax(eax)
        }
        0x2710 => {
            let node = w(0x689);
            crate::sa_misc::leave(g, node, 0x2710);
            g.write::<u32>(s + 0x30, node);
            crate::sa_summon::model_setup(g, img, &on(0x69b, s + 0x30));
            let next = ring_leave(g, task);
            bump(g, 0x2c);
            Ret::eax(next)
        }
        _ => {
            bump(g, 0x2c);
            Ret::eax(if (0xb..0x2710).contains(&state) {
                state - 0xb
            } else {
                state
            })
        }
    }
}

/// One target of [`summon_task_0202`]'s state 13 (`i`; `S` the task's
/// frame): its position `V` (slot 262 and the ground, slot 12), less `P`
/// (slot 234), slot 232 on `V` and on `(W, Q)`; while its flag `R +
/// 0x30 + 4i` is set and slot 394's distance `(V, W)` is within `K`, its
/// nodes, model node and particle node, slot 239, the flag cleared.
/// Returns the flag as it is after.
#[allow(clippy::too_many_lines)] // one target's steps, in order
fn target_0202<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32, i: u32) -> u32 {
    use crate::m0018::st0_f32;
    use crate::r17::summon_append;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let r = g.read::<u32>(fr.arg(g, 0) + 4);
    let (l, l2, v, wv, d) = (s + 0xc, s + 0x44, s + 0x5c, s + 0x6c, s + 0x30);
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let edi = i.wrapping_mul(0x2c);
    let t = w(0x2d7).wrapping_add(i.wrapping_mul(0x98));
    let m = w(0x2df).wrapping_add(i.wrapping_mul(0x14c));
    let p = w(0x2e7).wrapping_add(i << 8);
    let flag = r + 0x30 + 4 * i;
    let actor = g.read::<u32>(t);
    // call width: 4
    g.call(
        slot_target(g, w(0x305), 0x418 / 4),
        0,
        &[actor, v, v + 4, v + 8],
    );
    let actor = g.read::<u32>(t);
    let ground = st0_f32(g.call(slot_target(g, w(0x314), 0x30 / 4), 0, &[actor]).st0);
    g.write::<f32>(v + 4, ground);
    g.call(slot_target(g, w(0x332), 0x3a8 / 4), 0, &[v, v, w(0x32b)]);
    g.call(slot_target(g, w(0x348), 0x3a0 / 4), 0, &[v, v]);
    // call width: 4
    g.call(
        slot_target(g, w(0x35f), 0x3a0 / 4),
        0,
        &[wv, w(0x359), v, v],
    );
    if g.read::<u32>(flag) == 0 {
        return 0;
    }
    let dist = st0_f32(g.call(slot_target(g, w(0x382), 0x628 / 4), 0, &[v, wv]).st0);
    g.write::<f32>(d, dist);
    // comiss K, d; jb: on when K >= d (and ordered)
    if !matches!(
        kf(g, 0x396).partial_cmp(&dist),
        Some(core::cmp::Ordering::Greater | core::cmp::Ordering::Equal)
    ) {
        return g.read::<u32>(flag);
    }
    let b = w(0x3aa).wrapping_add(edi);
    summon_append(
        g,
        img,
        f,
        b,
        [
            0x3cf, 0x3d9, 0x3de, 0x3e9, 0x3ee, 0x3f6, 0x402, 0x419, 0x3b9,
        ],
    );
    set(
        g,
        l,
        &[t.wrapping_sub(0x54), w(0x40f).wrapping_add(edi), w(0x425)],
    );
    let ht = Frame {
        at: call(0x42a),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    crate::s4_small::heading_to(g, img, &ht);
    let at = w(0x430).wrapping_add(edi);
    let x = g.read::<u32>(at).wrapping_sub(0x2000);
    g.write::<u32>(at, x);
    let c = w(0x43a).wrapping_add(edi);
    summon_append(
        g,
        img,
        f,
        c,
        [
            0x459, 0x463, 0x468, 0x473, 0x478, 0x480, 0x48c, 0x498, 0x4ae,
        ],
    );
    let cy = w(0x492).wrapping_add(edi);
    let actor = g.read::<u32>(t);
    // call width: 4
    g.call(
        slot_target(g, w(0x4b5), 0x418 / 4),
        0,
        &[actor, w(0x4a3).wrapping_add(edi), cy, cy + 4],
    );
    let actor = g.read::<u32>(t);
    let y = st0_f32(g.call(slot_target(g, w(0x4c8), 0x30 / 4), 0, &[actor]).st0);
    g.write::<f32>(cy, y);
    let y = sub(g.read::<f32>(cy), kf(g, 0x4db));
    g.write::<u32>(w(0x4e1).wrapping_add(edi), 0x3f80_0000);
    g.write::<f32>(cy, y);
    let dn = w(0x4eb).wrapping_add(edi);
    summon_append(
        g,
        img,
        f,
        dn,
        [
            0x511, 0x51b, 0x520, 0x52b, 0x530, 0x538, 0x544, 0x54e, 0x554,
        ],
    );
    for (o, v) in [
        (0x568u32, 0x3f80_0000u32),
        (0x572, 0x3f80_0000),
        (0x57c, 0x3f80_0000),
        (0x586, 0x3f33_3333),
    ] {
        g.write::<u32>(w(o).wrapping_add(edi), v);
    }
    set(g, l2, &[m, w(0x596), w(0x59e), c, b, dn]);
    let ins = Frame {
        at: call(0x5ab),
        esp: s - 4,
        ecx: l2,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &ins);
    let o = g.read::<u32>(w(0x5b4));
    g.write::<u8>(m + 0x148, 1);
    set(g, l2, &[p, w(0x5d1), m, o, w(0x5e1), 1]);
    g.write::<u32>(p, w(0x5c5));
    crate::sb_nodes::particle_node(g, img, call(0x5ee), l2);
    let id = g.read::<u32>(t.wrapping_sub(4));
    let h = g.read::<u32>(w(0x608));
    g.call(slot_target(g, w(0x5f7), 0x3bc / 4), 0, &[h, id, 0]);
    g.write::<u32>(flag, 0);
    0
}

/// [`summon_task_0202`] (2024 bytes, 1 overlay).
pub const SUMMON_TASK_0202: Sig = Sig {
    len: 2024,
    hash: 0xf20c_8adb_680a_79cf,
    calls: &[
        (0xa9, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xb8, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0xf1, Callee::Sig(&crate::nodes::INSERT)),
        (0x100, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x132, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x18b, Callee::Sig(&crate::nodes::INSERT)),
        (0x1a6, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x295, Callee::Sig(&crate::nodes::INSERT)),
        (0x42a, Callee::Sig(&crate::s4_small::HEADING_TO)),
        (0x5ab, Callee::Sig(&crate::nodes::INSERT)),
        (0x5ee, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x680, Callee::Any),
        (0x68e, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x69b, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x6c6, Callee::Any),
        (0x711, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x742, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x751, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x760, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (
            0x778,
            Callee::Sig(&crate::o4_ctor::EFFECT_CTOR_ARG2_0202_6F10),
        ),
        (0x7a4, Callee::Any),
        (0x7b2, Callee::Sig(&crate::o4_fam2::FLAG_SLOTS_0202_6D90)),
        (0x7cd, Callee::Any),
    ],
    imports: &[],
};

/// `mov byte [G], imm8` at each instruction offset of `ats` (operand
/// `+2`, immediate `+6`).
fn byte_stores<G: Game>(g: &mut G, img: &Image, f: u32, ats: &[u32]) {
    for &at in ats {
        let v = img.word(f + at + 6).to_le_bytes()[0];
        g.write::<u8>(img.word(f + at + 2), v);
    }
}

/// `magic_0144`'s summon task (0x10006780, 2527 bytes with its tables;
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `A`, the
/// group `L` at `S + 8`, `S = A - 0x20`). `R + 0x1c` and `+0x20` grow by
/// `K0`, the count `R + 0x28` by one, then by the state `R + 0x18` (a
/// byte table and a jump table; the waits set the next state and run it
/// in the same call):
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)
///   `[NODE, LIST, *G_A, 0, 1]`), the colour node
///   ([`colour_node`](crate::s3_misc::colour_node)), the camera block's
///   flag and scales; then as 10;
/// - 10: three nodes appended inline (the first with the target's record
///   `R + 0x24`), the particle node
///   ([`particle_node`](crate::sb_nodes::particle_node), program 3), a
///   fourth node with slot 719's handle;
/// - 20, 30, 40: a node (20 and 30 through
///   [`node_init`](crate::m0003::node_init) on the target's record), a
///   node appended, the model node ([`node_insert`](crate::nodes::node_insert))
///   and its particle node (programs 0, 1, 2);
/// - 11, 21, 31: from a count of 20, 25, 30 the states 20, 30, 40;
/// - 41: from a count of 25 the camera block's `K - x` and differences,
///   then 10000;
/// - 42: from a count of 65, 10000;
/// - 10000: the task's node leaves its ring.
///
/// 10, 20, 30 and 40 go on to the next state and restart the count at 1;
/// the waits and other states count once more. Returns the last call's
/// `eax`, the byte table's index, the state or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0144<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r17::summon_append_abs;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x1c; push esi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let l = s + 8;
    let set_l = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(l + 4 * i, v);
        }
    };
    let on = |o: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    let bump = |g: &mut G, o: u32| {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    };
    let x = add(g.read::<f32>(r + 0x1c), kf(g, 0x19));
    let mut state = g.read::<u32>(r + 0x18);
    bump(g, 0x28);
    g.write::<f32>(r + 0x1c, x);
    let y = add(g.read::<f32>(r + 0x20), kf(g, 0x31));
    g.write::<f32>(r + 0x20, y);
    // the record of the target `R + 0x24` (stride 0x98) from the operand `o`
    let target = |g: &G, o: u32| {
        g.read::<u32>(r + 0x24)
            .wrapping_mul(0x98)
            .wrapping_add(w(o))
    };
    // a model node, its particle node (the tail the three share: L's
    // words 0, 2 and 5 given, then 1, 3, 4)
    let tail = |g: &mut G, (l0, l2, l5): (u32, u32, u32), vt: u32| {
        set_l(g, &[w(l0)]);
        g.write::<u32>(l + 8, w(l2));
        g.write::<u32>(l + 0x14, l5);
        stores(g, img, f, &[vt]);
        let e = g.read::<u32>(w(0x6f9));
        g.write::<u32>(l + 4, w(0x705));
        g.write::<u32>(l + 0xc, e);
        g.write::<u32>(l + 0x10, w(0x711));
        let eax = crate::sb_nodes::particle_node(g, img, call(0x716), l);
        bump(g, 0x18);
        g.write::<u32>(r + 0x28, 1);
        Ret::eax(eax)
    };
    loop {
        if state == 0x2710 {
            let next = ring_leave(g, task);
            bump(g, 0x28);
            return Ret::eax(next);
        }
        let case = if state <= 0x2a {
            img.word(w(0x62).wrapping_add(state)) & 0xff
        } else {
            0xa
        };
        let eax = if state <= 0x2a { case } else { state };
        let wait = |g: &G, n: i32| g.read::<i32>(r + 0x28) < n;
        state = match case {
            0 | 1 => {
                if case == 0 {
                    let a = g.read::<u32>(w(0x1e5));
                    set_l(g, &[w(0x1f1), w(0x1f9), a, 0]);
                    g.write::<u8>(l + 0x10, 1);
                    stores(g, img, f, &[0x20e]);
                    crate::s3_summon::model_nodes(g, img, &on(0x219));
                    set_l(g, &[w(0x225), w(0x22d)]);
                    stores(g, img, f, &[0x231]);
                    crate::s3_misc::colour_node(g, img, &on(0x23c));
                    or1(g, img, f, 0x242);
                    stores(
                        g,
                        img,
                        f,
                        &[
                            0x247, 0x251, 0x25b, 0x265, 0x26f, 0x279, 0x283, 0x28d, 0x297,
                        ],
                    );
                    zeros(g, img, f, &[0x2a1]);
                    g.write::<u32>(r + 0x18, 0xa);
                }
                return nodes_0144(g, img, fr, r, l, target(g, 0x2c2));
            }
            2 | 4 | 6 => {
                let (n, next) = [(0x14, 0x14), (0x19, 0x1e), (0x1e, 0x28)][case as usize / 2 - 1];
                if wait(g, n) {
                    bump(g, 0x28);
                    return Ret::eax(eax);
                }
                g.write::<u32>(r + 0x18, next);
                next
            }
            3 | 5 => {
                let (rec, n, ins, byte, flag) = if case == 3 {
                    (0x5cc, 0x5d8, 0x5e9, 0x5f2, 0x5f9)
                } else {
                    (0x736, 0x742, 0x753, 0x75c, 0x763)
                };
                set_l(g, &[w(n), w(n + 8), target(g, rec)]);
                crate::m0003::node_init(g, img, call(ins), l);
                let o = if case == 3 { 0x5ee } else { 0x758 };
                let last = g.read::<u32>(w(o));
                byte_stores(g, img, f, &[byte]);
                stores(g, img, f, &[flag]);
                if case == 3 {
                    summon_append_abs(
                        g,
                        img,
                        f,
                        last,
                        (
                            0x603,
                            &[0x60c, 0x616, 0x620, 0x62a],
                            0x635,
                            0x63a,
                            0x640,
                            0x64f,
                        ),
                        Some(0x656),
                        &[0x65e, 0x668, 0x672, 0x67c, 0x686, 0x690],
                    );
                    set_l(
                        g,
                        &[w(0x69e), w(0x6a6), w(0x6ae), w(0x6b6), w(0x6be), w(0x6c6)],
                    );
                    crate::nodes::node_insert(g, img, &on(0x6cb));
                    byte_stores(g, img, f, &[0x6cf]);
                    return tail(g, (0x6da, 0x6e2, 0), 0x6ee);
                }
                summon_append_abs(
                    g,
                    img,
                    f,
                    last,
                    (
                        0x76d,
                        &[0x776, 0x780, 0x78a, 0x794],
                        0x79f,
                        0x7a4,
                        0x7aa,
                        0x7b9,
                    ),
                    Some(0x7c0),
                    &[0x7c8, 0x7d2, 0x7dc, 0x7e6, 0x7f0, 0x7fa],
                );
                set_l(
                    g,
                    &[w(0x808), w(0x810), w(0x818), w(0x820), w(0x828), w(0x830)],
                );
                crate::nodes::node_insert(g, img, &on(0x835));
                byte_stores(g, img, f, &[0x839]);
                return tail(g, (0x844, 0x84c, 1), 0x858);
            }
            7 => {
                let last = g.read::<u32>(w(0x868));
                summon_append_abs(
                    g,
                    img,
                    f,
                    last,
                    (
                        0x86c,
                        &[0x875, 0x87f, 0x889, 0x893],
                        0x89e,
                        0x8a3,
                        0x8a9,
                        0x8b8,
                    ),
                    Some(0x8bf),
                    &[0x8c7, 0x8d1, 0x8db, 0x8e5, 0x8ef, 0x8f9],
                );
                set_l(
                    g,
                    &[w(0x907), w(0x90f), w(0x917), w(0x91f), w(0x927), w(0x92f)],
                );
                crate::nodes::node_insert(g, img, &on(0x934));
                byte_stores(g, img, f, &[0x938]);
                return tail(g, (0x943, 0x94b, 2), 0x957);
            }
            8 => {
                if wait(g, 0x19) {
                    bump(g, 0x28);
                    return Ret::eax(eax);
                }
                lights_0144(g, img, f);
                g.write::<u32>(r + 0x18, 0x2710);
                0x2710
            }
            9 => {
                if wait(g, 0x41) {
                    bump(g, 0x28);
                    return Ret::eax(eax);
                }
                g.write::<u32>(r + 0x18, 0x2710);
                0x2710
            }
            _ => {
                bump(g, 0x28);
                return Ret::eax(eax);
            }
        };
    }
}

/// [`summon_task_0144`]'s state 41: the camera block (`K` the float at
/// `+0x49`): its flags bit 0 off and bit 2 on, 1.0 scales, `K - x` for
/// four words and four differences.
fn lights_0144<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let dif = |g: &mut G, (a, b, d): (u32, u32, u32)| {
        let v = sub(kf(g, a), kf(g, b));
        g.write::<f32>(w(d), v);
    };
    let k = kf(g, 0x49);
    let x = sub(k, kf(g, 0xc0));
    let flags = g.read::<u32>(w(0xc5)) & !1 | 4;
    stores(g, img, f, &[0xcf]);
    g.write::<f32>(w(0xdd), x);
    let v = sub(kf(g, 0xe5), kf(g, 0xed));
    stores(g, img, f, &[0xf1, 0xfb, 0x105, 0x10f]);
    g.write::<f32>(w(0x11d), v);
    let x = sub(k, kf(g, 0x128));
    zeros(g, img, f, &[0x12c]);
    g.write::<u32>(w(0x137), flags);
    stores(g, img, f, &[0x13b]);
    g.write::<f32>(w(0x149), x);
    dif(g, (0x151, 0x159, 0x161));
    k_minus(g, img, f, k, &[(0x16c, 0x174)]);
    dif(g, (0x17c, 0x184, 0x18c));
    k_minus(g, img, f, k, &[(0x197, 0x19f)]);
    dif(g, (0x1a7, 0x1af, 0x1b7));
}

/// [`summon_task_0144`]'s state 10 (`rec` the target's record): the
/// nodes `N1` (the record at `+0x38`), `N2` and `N3` appended to the
/// list one after another (the count read once, stored once), the
/// particle node (program 3), `N4` appended with slot 719's handle;
/// state 11, the count 1. Returns slot 719's `eax`.
fn nodes_0144<G: Game>(g: &mut G, img: &Image, fr: &Frame, r: u32, l: u32, rec: u32) -> Ret {
    use crate::r17::summon_append_abs;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let last = g.read::<u32>(w(0x2b6));
    summon_append_abs(
        g,
        img,
        f,
        last,
        (
            0x2d0,
            &[0x2c6, 0x2d9, 0x2e3, 0x2ed, 0x2f7],
            0x302,
            0x307,
            0x30d,
            0x31c,
        ),
        None,
        &[],
    );
    let count = g.read::<u16>(w(0x322));
    g.write::<u32>(w(0x328), rec);
    stores(g, img, f, &[0x333, 0x33d]);
    zeros(g, img, f, &[0x347, 0x351]);
    byte_stores(g, img, f, &[0x35b]);
    stores(g, img, f, &[0x362]);
    // N2 after N1, N3 after N2: each compared with the list as the last
    for (flag, pre, prev, cmp, first, link) in [
        (
            0x36cu32,
            [0x375u32, 0x37f, 0x389, 0x393, 0x39d],
            0x32d,
            0x3a9,
            0x3af,
            0x3bb,
        ),
        (
            0x408,
            [0x411, 0x41b, 0x425, 0x42f, 0x439],
            0x3c6,
            0x445,
            0x44b,
            0x457,
        ),
    ] {
        if flag == 0x408 {
            stores(g, img, f, &[0x3fe]);
        }
        g.write::<u16>(w(flag + 3), 0xff);
        stores(g, img, f, &pre);
        stores(g, img, f, &[if w(prev) == w(cmp) { first } else { link }]);
        if flag == 0x36c {
            stores(g, img, f, &[0x3cc, 0x3d6, 0x3e0, 0x3ea, 0x3f4]);
        }
    }
    g.write::<u16>(w(0x465), count.wrapping_add(3));
    let e = g.read::<u32>(w(0x46a));
    stores(g, img, f, &[0x472, 0x47c, 0x486, 0x490, 0x49a]);
    byte_stores(g, img, f, &[0x4a4]);
    for (i, v) in (0u32..).zip([w(0x4af), w(0x4b7), w(0x4bf), e, w(0x4cb), 3]) {
        g.write::<u32>(l + 4 * i, v);
    }
    stores(g, img, f, &[0x4d7]);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x4e2), l);
    let last = g.read::<u32>(w(0x4e7));
    summon_append_abs(
        g,
        img,
        f,
        last,
        (
            0x4eb,
            &[0x4f4, 0x4fe, 0x508, 0x512],
            0x51d,
            0x522,
            0x528,
            0x537,
        ),
        Some(0x543),
        &[0x549],
    );
    let tab = w(0x53c);
    let h = g
        .call(slot_target(g, tab, 0xb3c / 4), 0, &[w(0x55c), 6, 0])
        .eax;
    g.write::<u32>(w(0x563), h);
    stores(g, img, f, &[0x567, 0x571, 0x57b, 0x585, 0x58f]);
    zeros(g, img, f, &[0x599]);
    stores(g, img, f, &[0x5a3]);
    let n = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, n);
    g.write::<u32>(r + 0x28, 1);
    Ret::eax(h)
}

/// [`summon_task_0144`] (2527 bytes, 1 overlay).
pub const SUMMON_TASK_0144: Sig = Sig {
    len: 2527,
    hash: 0x9745_7dbb_4650_cbb0,
    calls: &[
        (0x219, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x23c, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x4e2, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5e9, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x6cb, Callee::Sig(&crate::nodes::INSERT)),
        (0x716, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x753, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x835, Callee::Sig(&crate::nodes::INSERT)),
        (0x934, Callee::Sig(&crate::nodes::INSERT)),
    ],
    imports: &[],
};

/// `magic_0207`'s summon task (0x10006b00, 2637 bytes with its tables;
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `A`, the
/// group `L` at `S + 0xc`, `S = A - 0x20`). By the state `R + 0x18` (a
/// byte table and a jump table; 11, 25 and 32 set the next state once
/// slot 411 `(*G)` answers and run it in the same call):
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)),
///   the summon's model node
///   ([`model_node_init`](crate::sa_summon::model_node_init)), a node
///   ([`node_init`](crate::m0003::node_init)), the colour node and two
///   effect nodes ([`effect_node_a`](crate::s4_fam446::effect_node_a));
///   state 10;
/// - 10 and 30: slot 228 `(*G, 0xb2)` / `(*G, 0xb6)`, then slot 701 with
///   eleven words (`2, .., 10.0, 1.0` / `1, .., 0.0, 1.0`);
/// - 20: five nodes appended, two big nodes
///   ([`node_init_big_table`](crate::m0070::node_init_big_table)) with
///   slot 215's handles, slot 228 `(*G, 0xb3)`;
/// - 21: from a count of 3, four particle nodes
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with)) and two
///   copies ([`copy_node`](crate::m0207::copy_node));
/// - 22: once slot 411 answers, the camera block (`K - x`, 0.5 scales),
///   six nodes appended, two model nodes
///   ([`insert_with`](crate::sa_misc::insert_with)) and their particle
///   nodes, slot 228 `(*G, 0xb4)`;
/// - 23: from a count of 4, a static child
///   ([`static_child`](crate::sa_misc::static_child)), a node, a model
///   node and its particle node, slot 239 `(*G, -5, 0)`;
/// - 24: from a count of 16, the camera block again (1.0 scales, flags
///   bit 0 off and bit 2 on), slot 228 `(*G, 0xb5)`;
/// - 31: once slot 411 answers, slot 228 `(*G, 0xb7)`;
/// - 10000: slot 213 `(*G, 0)`, the task's node off its ring.
///
/// Every exit counts `R + 0x2c` (20, 22 and 23 restart it at 1); the
/// states that act move on to the next. Returns the last call's `eax`,
/// the byte table's index, the state, 0 or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0207<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x18; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let l = s + 0xc;
    // the calls with four pushed words: the return address at S - 0x14
    let esp4 = s.wrapping_sub(0x14);
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let bump = |g: &mut G, o: u32| {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    };
    let slot = |g: &mut G, tab: u32, at: u32, words: &[u32]| {
        g.call(slot_target(g, w(tab), at / 4), 0, words).eax
    };
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let on = |o: u32, ecx: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx,
        ..*fr
    };
    // push b; sub esp, 8; push node: a particle node with `ecx` (operands)
    let pnw = |g: &mut G, (ecx, node, b, at): (u32, u32, u32, u32)| {
        g.write::<u32>(esp4 + 0x10, b);
        g.write::<u32>(esp4 + 4, w(node));
        let p = Frame {
            at: call(at),
            esp: esp4,
            ecx: w(ecx),
            ..*fr
        };
        crate::s3_nodes::particle_node_with(g, img, &p)
    };
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        if state == 0x2710 {
            let h = gw(g, 0x9c3);
            slot(g, 0x9bb, 0x354, &[h, 0]);
            let next = ring_leave(g, task);
            bump(g, 0x2c);
            return Ret::eax(next);
        }
        let case = if state <= 0x20 {
            img.word(w(0x32).wrapping_add(state)) & 0xff
        } else {
            12
        };
        let idle = if state <= 0x20 { case } else { state };
        let eax = match case {
            0 => {
                let a = gw(g, 0xc8);
                set(g, l, &[w(0xd4), w(0xdc), a, 0]);
                g.write::<u8>(l + 0x10, 1);
                stores(g, img, f, &[0xf1]);
                crate::s3_summon::model_nodes(g, img, &on(0xfc, l));
                let b = gw(g, 0x101);
                set(g, l, &[w(0x10d), w(0x115), b]);
                g.write::<u8>(l + 0xc, 0);
                stores(g, img, f, &[0x122]);
                crate::sa_summon::model_node_init(g, img, call(0x12d), l);
                set(g, l, &[w(0x139), w(0x141), w(0x149)]);
                crate::m0003::node_init(g, img, call(0x14e), l);
                byte_stores(g, img, f, &[0x156]);
                zeros(g, img, f, &[0x15d]);
                set(g, l, &[w(0x16b), w(0x173)]);
                stores(g, img, f, &[0x177]);
                crate::s3_misc::colour_node(g, img, &on(0x182, l));
                // push b; push ecx; push N; push ecx (the left-over `ecx`
                // words are never read: 0 here)
                let mut eax = 0;
                for (b, n, ecx, at) in [
                    (0u32, 0x18au32, 0x190u32, 0x195u32),
                    (1, 0x1a0, 0x1a6, 0x1ab),
                ] {
                    set(g, esp4 + 4, &[0, w(n), 0, b]);
                    let e = Frame {
                        at: call(at),
                        esp: esp4,
                        ecx: w(ecx),
                        ..*fr
                    };
                    eax = crate::s4_fam446::effect_node_a(g, img, &e).eax;
                }
                or1(g, img, f, 0x1b1);
                zeros(g, img, f, &[0x1b6]);
                bump(g, 0x2c);
                g.write::<u32>(r + 0x18, 0xa);
                return Ret::eax(eax);
            }
            1 | 9 => {
                let (tab, gop, id, two, x) = if case == 1 {
                    (0x1d4, 0x1df, 0xb2, (0x21b, 2), 0x4120_0000)
                } else {
                    (0x927, 0x932, 0xb6, (0x96e, 1), 0)
                };
                let h = gw(g, gop);
                let e = slot(g, tab, 0x390, &[h, id]);
                let a = gw(g, two.0);
                let b = gw(g, 0x228);
                // call width: 11
                let eax = slot(
                    g,
                    0x222,
                    0xaf4,
                    &[b, two.1, a, 0, e, 0, 0, x, 0x3f80_0000, 0, 0],
                );
                bump(g, 0x18);
                bump(g, 0x2c);
                return Ret::eax(eax);
            }
            2 | 8 | 11 => {
                let (tab, gop, next) = match case {
                    2 => (0x3e, 0x44, 0x14),
                    8 => (0x65, 0x6b, 0x1e),
                    _ => (0x8c, 0x92, 0x2710),
                };
                let h = gw(g, gop);
                if slot(g, tab, 0x66c, &[h]) == 0 {
                    bump(g, 0x2c);
                    return Ret::eax(0);
                }
                g.write::<u32>(r + 0x18, next);
                state = next;
                continue;
            }
            3 => {
                or1(g, img, f, 0x245);
                zeros(g, img, f, &[0x254]);
                let list = w(0x24b);
                crate::sa_misc::node_append(g, img, call(0x25f), w(0x250), list, 0);
                zeros(g, img, f, &[0x268]);
                stores(g, img, f, &[0x272]);
                zeros(g, img, f, &[0x27c]);
                node_append(g, img, call(0x287), w(0x264), list, 0);
                stores(g, img, f, &[0x290, 0x29a, 0x2a4, 0x2ae]);
                node_append(g, img, call(0x2b9), w(0x28c), list, 0);
                g.write::<u32>(l + 8, w(0x28c));
                stores(g, img, f, &[0x2c5]);
                zeros(g, img, f, &[0x2cf]);
                stores(g, img, f, &[0x2d9, 0x2e3]);
                set(g, l, &[w(0x2f1), list]);
                set(g, l + 0xc, &[w(0x2fd), w(0x305)]);
                crate::m0070::node_init_big_table(g, img, call(0x30a), l);
                let h = gw(g, 0x315);
                byte_stores(g, img, f, &[0x319]);
                let e = slot(g, 0x30f, 0x35c, &[h]);
                g.write::<u32>(w(0x336), e);
                stores(g, img, f, &[0x33a]);
                node_append(g, img, call(0x345), w(0x331), w(0x32c), e);
                g.write::<u32>(l + 8, w(0x331));
                stores(g, img, f, &[0x351]);
                zeros(g, img, f, &[0x35b]);
                stores(g, img, f, &[0x365, 0x36f]);
                set(g, l, &[w(0x37d), w(0x32c)]);
                set(g, l + 0xc, &[w(0x389), w(0x391)]);
                crate::m0070::node_init_big_table(g, img, call(0x396), l);
                let h = gw(g, 0x3a1);
                byte_stores(g, img, f, &[0x3a5]);
                let e = slot(g, 0x39b, 0x35c, &[h]);
                g.write::<u32>(w(0x3b8), e);
                let h = gw(g, 0x3c8);
                stores(g, img, f, &[0x3cc]);
                let eax = slot(g, 0x3bd, 0x390, &[h, 0xb3]);
                bump(g, 0x18);
                g.write::<u32>(r + 0x2c, 1);
                return Ret::eax(eax);
            }
            4 => {
                if g.read::<i32>(r + 0x2c) < 3 {
                    idle
                } else {
                    let copy = |g: &mut G, (arg, ecx, at): (u32, u32, u32)| {
                        g.write::<u32>(esp4, w(arg));
                        let c = Frame {
                            at: call(at),
                            esp: esp4 - 4,
                            ecx: w(ecx),
                            ..*fr
                        };
                        crate::m0207::copy_node(g, img, &c);
                    };
                    pnw(g, (0x404, 0x409, 0, 0x40e));
                    copy(g, (0x413, 0x418, 0x41d));
                    pnw(g, (0x425, 0x42f, 3, 0x434));
                    pnw(g, (0x43c, 0x446, 0, 0x44b));
                    copy(g, (0x450, 0x455, 0x45a));
                    let eax = pnw(g, (0x462, 0x46c, 3, 0x471));
                    bump(g, 0x18);
                    bump(g, 0x2c);
                    return Ret::eax(eax);
                }
            }
            5 => {
                let h = gw(g, 0x48b);
                if slot(g, 0x485, 0x66c, &[h]) == 0 {
                    0
                } else {
                    let eax = nodes_0207(g, img, fr, s);
                    bump(g, 0x18);
                    g.write::<u32>(r + 0x2c, 1);
                    return Ret::eax(eax);
                }
            }
            6 => {
                if g.read::<i32>(r + 0x2c) < 4 {
                    idle
                } else {
                    crate::sa_misc::static_child(g, img, call(0x735), w(0x730));
                    let b = g.read::<u8>(w(0x73a));
                    g.write::<u8>(w(0x749), b);
                    node_append(g, img, call(0x74e), w(0x744), w(0x73f), 0);
                    set(g, esp4 + 4, &[w(0x75e), w(0x759), w(0x754), w(0x744)]);
                    stores(g, img, f, &[0x767, 0x771, 0x77b, 0x785]);
                    let iw = Frame {
                        at: call(0x790),
                        esp: esp4,
                        ecx: w(0x763),
                        edx: w(0x73f),
                        ..*fr
                    };
                    crate::sa_misc::insert_with(g, img, &iw);
                    byte_stores(g, img, f, &[0x7a1]);
                    pnw(g, (0x798, 0x7a9, 2, 0x7ae));
                    let h = gw(g, 0x7c3);
                    let eax = slot(g, 0x7b3, 0x3bc, &[h, 0xffff_fffb, 0]);
                    bump(g, 0x18);
                    g.write::<u32>(r + 0x2c, 1);
                    return Ret::eax(eax);
                }
            }
            7 => {
                if g.read::<i32>(r + 0x2c) < 0x10 {
                    idle
                } else {
                    let h = lights_0207(g, img, f);
                    let eax = slot(g, 0x820, 0x390, &[h, 0xb5]);
                    bump(g, 0x18);
                    bump(g, 0x2c);
                    return Ret::eax(eax);
                }
            }
            10 => {
                let h = gw(g, 0x980);
                if slot(g, 0x97a, 0x66c, &[h]) == 0 {
                    0
                } else {
                    let h = gw(g, 0x99f);
                    let eax = slot(g, 0x994, 0x390, &[h, 0xb7]);
                    bump(g, 0x18);
                    bump(g, 0x2c);
                    return Ret::eax(eax);
                }
            }
            _ => idle,
        };
        bump(g, 0x2c);
        return Ret::eax(eax);
    }
}

/// [`summon_task_0207`]'s state 24 camera block (`K` the float at
/// `+0x7ed`): flags bit 0 off and bit 2 on, `K - x`, 1.0 scales. Returns
/// `*G` as the original pushes it for slot 228 in the middle.
fn lights_0207<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let k = kf(g, 0x7ed);
    let x = sub(k, kf(g, 0x7f8));
    let flags = g.read::<u32>(w(0x7fd)) & !1 | 4;
    g.write::<u32>(w(0x808), flags);
    g.write::<f32>(w(0x810), x);
    let x = sub(k, kf(g, 0x81b));
    let h = g.read::<u32>(w(0x82b));
    stores(g, img, f, &[0x82f]);
    g.write::<f32>(w(0x83d), x);
    let x = sub(k, kf(g, 0x848));
    stores(g, img, f, &[0x84c, 0x856, 0x860, 0x86a]);
    g.write::<f32>(w(0x878), x);
    let x = sub(k, kf(g, 0x883));
    stores(g, img, f, &[0x887, 0x891, 0x89b]);
    zeros(g, img, f, &[0x8a5]);
    g.write::<f32>(w(0x8b3), x);
    let x = sub(k, kf(g, 0x8be));
    stores(g, img, f, &[0x8c2]);
    g.write::<f32>(w(0x8d0), x);
    let x = sub(k, kf(g, 0x8db));
    let y = sub(k, kf(g, 0x8e3));
    g.write::<f32>(w(0x8eb), x);
    let z = sub(kf(g, 0x8f3), kf(g, 0x8fb));
    g.write::<f32>(w(0x903), y);
    g.write::<f32>(w(0x90b), z);
    h
}

/// [`summon_task_0207`]'s state 22 after slot 411 (`S` the task's frame):
/// the camera block (`K - x`, 0.5 scales, flags bit 0 off), six nodes
/// appended, two model nodes ([`insert_with`](crate::sa_misc::insert_with))
/// with their particle nodes, slot 228 `(*G, 0xb4)`. Returns its `eax`.
fn nodes_0207<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let esp4 = s.wrapping_sub(0x14);
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let pnw = |g: &mut G, (ecx, node, b, at): (u32, u32, u32, u32)| {
        g.write::<u32>(esp4 + 0x10, b);
        g.write::<u32>(esp4 + 4, w(node));
        let p = Frame {
            at: call(at),
            esp: esp4,
            ecx: w(ecx),
            ..*fr
        };
        crate::s3_nodes::particle_node_with(g, img, &p)
    };
    let k = kf(g, 0x4a6);
    let x = sub(k, kf(g, 0x4b1));
    let flags = g.read::<u32>(w(0x4b7)) & !1;
    g.write::<u32>(w(0x4b7), flags);
    stores(g, img, f, &[0x4c6]);
    g.write::<f32>(w(0x4d4), x);
    let x = sub(k, kf(g, 0x4df));
    stores(g, img, f, &[0x4e3, 0x4ed, 0x4f7, 0x501]);
    g.write::<f32>(w(0x50f), x);
    let x = sub(k, kf(g, 0x51a));
    stores(g, img, f, &[0x51e, 0x528, 0x532]);
    zeros(g, img, f, &[0x53c]);
    g.write::<f32>(w(0x54a), x);
    k_minus(g, img, f, k, &[(0x555, 0x55d), (0x568, 0x570)]);
    let x = sub(k, kf(g, 0x57b));
    let y = sub(k, kf(g, 0x583));
    g.write::<f32>(w(0x58b), x);
    let z = sub(kf(g, 0x593), kf(g, 0x59b));
    g.write::<f32>(w(0x5a3), y);
    g.write::<f32>(w(0x5ab), z);
    let list = w(0x4bd);
    node_append(g, img, call(0x5b0), w(0x4c2), list, 0);
    stores(g, img, f, &[0x5b9, 0x5c3, 0x5cd, 0x5d7]);
    crate::sa_misc::node_append(g, img, call(0x5e2), w(0x5b5), list, 0);
    zeros(g, img, f, &[0x5eb, 0x5f5]);
    stores(g, img, f, &[0x5ff]);
    node_append(g, img, call(0x60a), w(0x5e7), list, 0);
    set(g, esp4 + 4, &[w(0x61a), w(0x5e7), w(0x614), w(0x60f)]);
    stores(g, img, f, &[0x623, 0x62d]);
    zeros(g, img, f, &[0x637]);
    stores(g, img, f, &[0x641]);
    let iw = Frame {
        at: call(0x64c),
        esp: esp4,
        ecx: w(0x61f),
        edx: list,
        ..*fr
    };
    crate::sa_misc::insert_with(g, img, &iw);
    pnw(g, (0x654, 0x65e, 1, 0x663));
    let list2 = w(0x668);
    crate::sa_misc::node_append(g, img, call(0x672), w(0x66d), list2, 0);
    zeros(g, img, f, &[0x67b, 0x685]);
    stores(g, img, f, &[0x68f]);
    node_append(g, img, call(0x69a), w(0x677), list2, 0);
    // pushed on top of the particle node's words: the return address at
    // S - 0x24
    let esp8 = s.wrapping_sub(0x24);
    set(g, esp8 + 4, &[w(0x6d2), w(0x677), w(0x6a4), w(0x69f)]);
    stores(g, img, f, &[0x6a9, 0x6b3]);
    zeros(g, img, f, &[0x6bd]);
    stores(g, img, f, &[0x6c7]);
    let iw = Frame {
        at: call(0x6dc),
        esp: esp8,
        ecx: w(0x6d7),
        edx: list2,
        ..*fr
    };
    crate::sa_misc::insert_with(g, img, &iw);
    pnw(g, (0x6e4, 0x6ee, 1, 0x6f3));
    let h = g.read::<u32>(w(0x703));
    g.call(slot_target(g, w(0x6f8), 0x390 / 4), 0, &[h, 0xb4])
        .eax
}

/// [`summon_task_0207`] (2637 bytes, 1 overlay).
pub const SUMMON_TASK_0207: Sig = Sig {
    len: 2637,
    hash: 0x2516_105c_feab_9618,
    calls: &[
        (0xfc, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x12d, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x14e, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x182, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x195, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x1ab, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x25f, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x287, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2b9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x30a, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x345, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x396, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x40e, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x41d, Callee::Sig(&crate::m0207::COPY_NODE)),
        (0x434, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x44b, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x45a, Callee::Sig(&crate::m0207::COPY_NODE)),
        (0x471, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x5b0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5e2, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x60a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x64c, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x663, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x672, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x69a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6dc, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x6f3, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x735, Callee::Sig(&crate::sa_misc::STATIC_CHILD)),
        (0x74e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x790, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x7ae, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
    ],
    imports: &[],
};

/// `magic_0106`'s summon task (0x10006c30, 2337 bytes with its tables;
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `A`, `S =
/// A - 0x30` after the pushes: the group `L` at `S + 0x18`, a pair at `S +
/// 0x10`). By the state `R + 0x18` (a byte table and a jump table; 0, 10,
/// 21, 31, 42 and 52 set a state and run it in the same call):
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)),
///   a node at the targets' centre ([`centre`](crate::m0577::centre)), an
///   effect node ([`effect_node`](crate::sb_nodes::effect_node), its
///   vtable after) with its flags, the colour node, eight 1.0 words, the
///   root node ([`node_init_root`](crate::m0070::node_init_root)); then
///   as 40;
/// - 10: as 40;
/// - 20: a linked node ([`link_node_38_one`](crate::s4_small::link_node_38_one)),
///   a node appended, the model node ([`node_insert`](crate::nodes::node_insert)),
///   its particle node, an effect node;
/// - 30: two effect nodes, a particle node, the root node's colours and
///   its start ([`node_start`](crate::sa_misc::node_start));
/// - 40: the particle node (program 2);
/// - 41: from a count of 10, the camera block cleared (`0 - x`), then
///   for each of slot 288's target bytes that is no target, slot 541
///   `(id, 0.0, 15)` and the id kept in `R + 0x30` (count `R + 0x70`); the
///   flag `R + 0x74` set;
/// - 42: at the count 1 [`locked_878`](crate::s4_small::locked_878)
///   (`cl` 1); from 40, slot 296 `(id, 0)` for each kept id; state 20;
/// - 21, 31: from 122, 90 the states 30, 50 (31 with slot 239 `(*G, -5,
///   0)`);
/// - 50: on to 51;
/// - 51: from 15, slots 296 `(id, 1)` and 541 `(id, 1.0, 15)` per id;
/// - 52: from 15, the camera block (`K - x`, 1.0 scales), the flag
///   cleared, [`locked_878`](crate::s4_small::locked_878) (`cl` 0); state
///   10000;
/// - 10000: the task's node off its ring.
///
/// The states that act go to the next with the count 0; every exit calls
/// slot 845 while the flag `R + 0x74` is set and counts `R + 0x2c`.
/// Returns that slot's or the last call's `eax`, the byte table's index,
/// the state, the id count, 0 or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0106<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x24; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let l = s + 0x18;
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let on = |o: u32, ecx: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx,
        ..*fr
    };
    // the record's set-up then slot 719's effect node, its vtable after
    let effect = |g: &mut G, words: [u32; 5], b: u32, at: u32, vt: u32| {
        set(g, l, &words);
        g.write::<u32>(l + 0x14, b);
        crate::sb_nodes::effect_node(g, img, call(at), l);
        let node = g.read::<u32>(l);
        g.write::<u32>(node, w(vt));
        node
    };
    let mut state = g.read::<u32>(r + 0x18);
    let eax = loop {
        if state == 0x2710 {
            break ring_leave(g, task);
        }
        let case = if state <= 0x34 {
            img.word(w(0x3a).wrapping_add(state)) & 0xff
        } else {
            12
        };
        let idle = if state <= 0x34 { case } else { state };
        let count = g.read::<i32>(r + 0x2c);
        let next = |g: &mut G| {
            let n = g.read::<u32>(r + 0x18).wrapping_add(1);
            g.write::<u32>(r + 0x18, n);
            g.write::<u32>(r + 0x2c, 0);
        };
        let n_ids = |g: &G| u32::from(g.read::<u16>(r + 0x70));
        state = match case {
            0 | 1 => {
                if case == 0 {
                    start_0106(g, img, fr, s);
                    g.write::<u32>(r + 0x74, 0);
                    g.write::<u32>(r + 0x18, 0xa);
                }
                g.write::<u32>(r + 0x18, 0x28);
                0x28
            }
            2 => {
                g.write::<u32>(s - 4, w(0x579));
                let ln = Frame {
                    at: call(0x583),
                    esp: s - 8,
                    ecx: w(0x57e),
                    ..*fr
                };
                crate::s4_small::link_node_38_one(g, img, &ln);
                zeros(g, img, f, &[0x594]);
                stores(g, img, f, &[0x59e]);
                zeros(g, img, f, &[0x5a8]);
                node_append(g, img, call(0x5b3), w(0x590), w(0x58b), 0);
                g.write::<u32>(l + 0x14, w(0x590));
                stores(g, img, f, &[0x5bf, 0x5c9, 0x5d3, 0x5dd]);
                set(g, l, &[w(0x5eb), w(0x58b), w(0x5f7), w(0x5ff), w(0x607)]);
                crate::nodes::node_insert(g, img, &on(0x60c, l));
                let o = g.read::<u32>(w(0x611));
                set(g, l, &[w(0x61d), w(0x625), w(0x62d), o, w(0x639), 1]);
                stores(g, img, f, &[0x645]);
                crate::sb_nodes::particle_node(g, img, call(0x650), l);
                let node = effect(
                    g,
                    [w(0x65c), w(0x664), w(0x66c), w(0x674), w(0x67c)],
                    2,
                    0x689,
                    0x693,
                );
                or1(g, img, f, 0x699);
                zeros(g, img, f, &[0x69e]);
                next(g);
                break node;
            }
            4 => {
                effect(
                    g,
                    [w(0x6bf), w(0x6c7), w(0x6cf), w(0x6d7), w(0x6df)],
                    1,
                    0x6ec,
                    0x6fa,
                );
                or1(g, img, f, 0x700);
                zeros(g, img, f, &[0x705]);
                effect(
                    g,
                    [w(0x713), w(0x71b), w(0x723), w(0x72b), w(0x733)],
                    3,
                    0x740,
                    0x74e,
                );
                let o = g.read::<u32>(w(0x753));
                or1(g, img, f, 0x759);
                zeros(g, img, f, &[0x75e]);
                set(g, l, &[w(0x76c), w(0x774), w(0x77c), o, w(0x788), 0]);
                stores(g, img, f, &[0x794]);
                crate::sb_nodes::particle_node(g, img, call(0x79f), l);
                stores(
                    g,
                    img,
                    f,
                    &[0x7b0, 0x7ba, 0x7c4, 0x7ce, 0x7d8, 0x7e2, 0x7ec, 0x7f6],
                );
                let ns = Frame {
                    at: call(0x801),
                    esp: s - 4,
                    ecx: w(0x7ac),
                    xmm: [0, u128::from(g.read::<u32>(w(0x7a7))), 0, 0],
                    ..*fr
                };
                let eax = crate::sa_misc::node_start(g, img, &ns);
                next(g);
                break eax;
            }
            6 => {
                let o = g.read::<u32>(w(0x374));
                set(g, l, &[w(0x380), w(0x388), w(0x390), o, w(0x39c), 2]);
                stores(g, img, f, &[0x3a8]);
                let eax = crate::sb_nodes::particle_node(g, img, call(0x3b3), l);
                next(g);
                break eax;
            }
            7 => {
                if count < 0xa {
                    break idle;
                }
                let eax = ids_0106(g, img, fr, s);
                next(g);
                g.write::<u32>(r + 0x74, 1);
                break eax;
            }
            8 => {
                let mut eax = idle;
                if count == 1 {
                    // mov cl, 1: the rest of ecx is still the task
                    let lk = Frame {
                        ecx: task & !0xff | 1,
                        ..on(0x199, 0)
                    };
                    eax = crate::s4_small::locked_878(g, img, &lk).eax;
                }
                if count < 0x28 {
                    break eax;
                }
                let mut i = 0;
                while i < n_ids(g) {
                    let id = u32::from(g.read::<u16>(r + 0x30 + 2 * i));
                    g.call(slot_target(g, w(0x1bb), 0x4a0 / 4), 0, &[id, 0]);
                    i += 1;
                }
                g.write::<u32>(r + 0x18, 0x14);
                0x14
            }
            3 | 5 => {
                let (n, to) = if case == 3 {
                    (0x7a, 0x1e)
                } else {
                    (0x5a, 0x32)
                };
                if count < n {
                    break idle;
                }
                if case == 5 {
                    let h = g.read::<u32>(w(0x213));
                    g.call(slot_target(g, w(0x203), 0x3bc / 4), 0, &[h, 0xffff_fffb, 0]);
                }
                g.write::<u32>(r + 0x18, to);
                to
            }
            9 => {
                next(g);
                break idle;
            }
            10 => {
                if count < 0xf {
                    break idle;
                }
                let mut i = 0;
                while i < n_ids(g) {
                    let id = u32::from(g.read::<u16>(r + 0x30 + 2 * i));
                    g.call(slot_target(g, w(0x837), 0x4a0 / 4), 0, &[id, 1]);
                    let id = u32::from(g.read::<u16>(r + 0x30 + 2 * i));
                    g.call(
                        slot_target(g, w(0x855), 0x874 / 4),
                        0,
                        &[id, 0x3f80_0000, 0xf],
                    );
                    i += 1;
                }
                let eax = if i == 0 { 0 } else { n_ids(g) };
                next(g);
                break eax;
            }
            11 => {
                if count < 0xf {
                    break idle;
                }
                lights_0106(g, img, f, g.read::<f32>(w(0x24)));
                g.write::<u32>(r + 0x74, 0);
                let lk = Frame {
                    ecx: task & !0xff,
                    ..on(0x34d, 0)
                };
                crate::s4_small::locked_878(g, img, &lk);
                g.write::<u32>(r + 0x18, 0x2710);
                0x2710
            }
            _ => break idle,
        };
    };
    let eax = if g.read::<u32>(r + 0x74) == 0 {
        eax
    } else {
        g.call(slot_target(g, w(0x8a0), 0xd34 / 4), 0, &[]).eax
    };
    let n = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, n);
    Ret::eax(eax)
}

/// [`summon_task_0106`]'s state 0 (`S` the task's frame): the model
/// nodes, a node at the targets' centre, an effect node, the colour node
/// and the root node.
fn start_0106<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let (l, l2) = (s + 0x18, s + 0x10);
    let set = |g: &mut G, at: u32, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(at + 4 * i, v);
        }
    };
    let on = |o: u32, ecx: u32, edx: u32| Frame {
        at: call(o),
        esp: s - 4,
        ecx,
        edx,
        ..*fr
    };
    let a = g.read::<u32>(w(0x46));
    set(g, l, &[w(0x52), w(0x5a), a, 0]);
    g.write::<u8>(l + 0x10, 1);
    stores(g, img, f, &[0x6f]);
    crate::s3_summon::model_nodes(g, img, &on(0x7a, l, 0));
    node_append(g, img, call(0x89), w(0x84), w(0x7f), 0);
    crate::m0577::centre(g, img, &on(0x98, w(0x93), w(0x8e)));
    set(g, l, &[w(0xa4), w(0xac), w(0xb4), w(0xbc), w(0xc4), 0]);
    crate::sb_nodes::effect_node(g, img, call(0xd1), l);
    let node = g.read::<u32>(l);
    g.write::<u32>(node, w(0xdf));
    let flags = g.read::<u32>(w(0xe4)) | 7;
    zeros(g, img, f, &[0xee]);
    g.write::<u32>(w(0xf9), flags);
    set(g, l2, &[w(0x101), w(0x109)]);
    stores(g, img, f, &[0x10d]);
    crate::s3_misc::colour_node(g, img, &on(0x118, l2, 0));
    stores(
        g,
        img,
        f,
        &[0x121, 0x12b, 0x135, 0x13f, 0x149, 0x153, 0x15d, 0x167],
    );
    crate::m0070::node_init_root(g, img, call(0x172), w(0x11d));
}

/// [`summon_task_0106`]'s state 41 (`S` the task's frame): the camera
/// block cleared (`0 - x`, scales 0), the id count `R + 0x70` 0, slot 288
/// `(16, &n)` gives `n` target bytes; each that no target `+0x0` (stride
/// 0x98, `*G_N` of them) holds gets slot 541 `(id, 0.0, 15)` and is kept
/// in `R + 0x30`. Returns `n`, or slot 288's `eax` for none.
fn ids_0106<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let r = g.read::<u32>(fr.arg(g, 0) + 4);
    let x = sub(0.0, kf(g, 0x3da));
    let flags = g.read::<u32>(w(0x3e0)) & !1;
    g.write::<u32>(w(0x3e0), flags);
    zeros(g, img, f, &[0x3e7, 0x3f1]);
    g.write::<f32>(w(0x3ff), x);
    let x = sub(0.0, kf(g, 0x40a));
    zeros(g, img, f, &[0x40e, 0x418, 0x422, 0x42c]);
    g.write::<f32>(w(0x43a), x);
    let x = sub(0.0, kf(g, 0x445));
    zeros(g, img, f, &[0x449]);
    stores(g, img, f, &[0x453]);
    zeros(g, img, f, &[0x45d]);
    g.write::<f32>(w(0x46b), x);
    k_minus(g, img, f, 0.0, &[(0x476, 0x47e), (0x489, 0x491)]);
    let x = sub(0.0, kf(g, 0x49c));
    let y = sub(0.0, kf(g, 0x4a4));
    g.write::<f32>(w(0x4ac), x);
    let z = sub(kf(g, 0x4b4), kf(g, 0x4bc));
    g.write::<f32>(w(0x4c4), y);
    g.write::<f32>(w(0x4cc), z);
    g.write::<u16>(r + 0x70, 0);
    let n_at = s + 0x10;
    let list = g
        .call(slot_target(g, w(0x4da), 0x480 / 4), 0, &[0x10, n_at])
        .eax;
    let n = g.read::<i32>(n_at);
    if n <= 0 {
        return list;
    }
    for i in 0..n.cast_unsigned() {
        let id = u32::from(g.read::<u8>(list.wrapping_add(i)));
        let targets = g.read::<i32>(w(0x4fe));
        let known = (0..targets.max(0).cast_unsigned())
            .any(|j| g.read::<u32>(w(0x50c).wrapping_add(j * 0x98)) == id);
        if known {
            continue;
        }
        g.call(slot_target(g, w(0x52d), 0x874 / 4), 0, &[id, 0, 0xf]);
        let k = g.read::<u16>(r + 0x70);
        let id = g.read::<u8>(list.wrapping_add(i));
        g.write::<u16>(r + 0x30 + 2 * u32::from(k), u16::from(id));
        g.write::<u16>(r + 0x70, k.wrapping_add(1));
    }
    n.cast_unsigned()
}

/// [`summon_task_0106`]'s state 52 camera block (`K` given): flags bit 0
/// off and bit 2 on, `K - x` and differences, 1.0 scales.
fn lights_0106<G: Game>(g: &mut G, img: &Image, f: u32, k: f32) {
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let x = sub(k, kf(g, 0x239));
    let flags = g.read::<u32>(w(0x23e)) & !1 | 4;
    stores(g, img, f, &[0x248]);
    g.write::<f32>(w(0x256), x);
    let x = sub(k, kf(g, 0x261));
    stores(g, img, f, &[0x265, 0x26f, 0x279, 0x283]);
    g.write::<f32>(w(0x291), x);
    let x = sub(k, kf(g, 0x29c));
    stores(g, img, f, &[0x2a0, 0x2aa, 0x2b4]);
    zeros(g, img, f, &[0x2be]);
    g.write::<f32>(w(0x2cc), x);
    let x = sub(k, kf(g, 0x2d7));
    g.write::<u32>(w(0x2dc), flags);
    stores(g, img, f, &[0x2e0]);
    g.write::<f32>(w(0x2f0), x);
    k_minus(g, img, f, k, &[(0x2fb, 0x303), (0x30e, 0x316)]);
    let v = sub(kf(g, 0x31e), kf(g, 0x326));
    g.write::<f32>(w(0x32e), v);
    k_minus(g, img, f, k, &[(0x339, 0x341)]);
}

/// [`summon_task_0106`] (2337 bytes, 1 overlay).
pub const SUMMON_TASK_0106: Sig = Sig {
    len: 2337,
    hash: 0x060f_a44f_34d4_5d3b,
    calls: &[
        (0x7a, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x89, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x98, Callee::Sig(&crate::m0577::CENTRE)),
        (0xd1, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x118, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x172, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x199, Callee::Sig(&crate::s4_small::LOCKED_878)),
        (0x34d, Callee::Sig(&crate::s4_small::LOCKED_878)),
        (0x3b3, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x583, Callee::Sig(&crate::s4_small::LINK_NODE_38_ONE)),
        (0x5b3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x60c, Callee::Sig(&crate::nodes::INSERT)),
        (0x650, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x689, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x6ec, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x740, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x79f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x801, Callee::Sig(&crate::sa_misc::NODE_START)),
    ],
    imports: &[],
};
