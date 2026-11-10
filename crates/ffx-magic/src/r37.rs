//! Round 37's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, in tested pieces.

// `comiss` with `jb`/`jbe`: an unordered compare takes one side
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::r25::abs_stores;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// [`camera_task_0570`]'s code offset, frame and record: `base` is `E -
/// 0xc` (the aligned frame `E` below the three pushes, so `[esp + x]` is
/// `base + x` with nothing pushed), `r` the task's record.
struct Ctx {
    f: u32,
    base: u32,
    r: u32,
}

impl Ctx {
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    fn k<G: Game>(&self, g: &G, img: &Image, o: u32) -> f32 {
        g.read::<f32>(self.w(img, o))
    }

    fn target(&self, img: &Image, o: u32) -> u32 {
        img.rel32_target(self.f + o)
    }

    /// Slot `n` through the imports pointer the instruction at operand
    /// `op` reads.
    fn slot<G: Game>(&self, g: &G, img: &Image, op: u32, n: u32) -> u32 {
        slot_target(g, self.w(img, op), n)
    }

    fn rf<G: Game>(&self, g: &G, o: u32) -> f32 {
        g.read::<f32>(self.r.wrapping_add(o))
    }

    fn wf<G: Game>(&self, g: &mut G, o: u32, x: f32) {
        g.write::<f32>(self.r.wrapping_add(o), x);
    }

    /// The state `R + 0x18` up by one.
    fn up<G: Game>(&self, g: &mut G) {
        let s = g.read::<u32>(self.r + 0x18).wrapping_add(1);
        g.write::<u32>(self.r + 0x18, s);
    }

    /// The count `R + 0x38` (the wait every state tests).
    fn count<G: Game>(&self, g: &G) -> i32 {
        g.read::<i32>(self.r + 0x38)
    }

    fn reset<G: Game>(&self, g: &mut G) {
        g.write::<u32>(self.r + 0x38, 0);
    }

    /// The vector `v` into the local `V = E + 0x10` (`base + 0x1c`), then
    /// slot 229 `(V, M, V)` (`M` operand `m`, the imports pointer operand
    /// `imp`). Returns slot 229's `eax`.
    fn vec229<G: Game>(&self, g: &mut G, img: &Image, (imp, m): (u32, u32), v: [u32; 4]) -> u32 {
        let p = self.base + 0x1c;
        for (i, x) in (0u32..).zip(v) {
            g.write::<u32>(p + 4 * i, x);
        }
        // call width: 3
        g.call(self.slot(g, img, imp, 229), 0, &[p, self.w(img, m), p])
            .eax
    }

    /// The four words of `V` into the globals the operands `ops` name.
    fn copy4<G: Game>(&self, g: &mut G, img: &Image, ops: [u32; 4]) {
        for (i, o) in (0u32..).zip(ops) {
            let x = g.read::<u32>(self.base + 0x1c + 4 * i);
            g.write::<u32>(self.w(img, o), x);
        }
    }

    /// The light set (the call at `c`) on `V` with the radius at operand
    /// `k`. Returns its `eax`.
    fn light<G: Game>(&self, g: &mut G, img: &Image, c: u32, k: u32) -> u32 {
        let r = u128::from(g.read::<u32>(self.w(img, k)));
        let lf = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(4),
            edx: self.base + 0x1c,
            xmm: [0, 0, r, 0],
            ..Frame::default()
        };
        crate::s4_small::light_set(g, img, &lf).eax
    }

    /// The move set-up (the call at `c`) of the node at operand `node`
    /// with `xmm2`/`xmm3` the floats at operands `k2`/`k3`, `esp` `d`
    /// below `base`. Returns `eax` as it came.
    fn setup<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node): (u32, u32),
        (k2, k3): (u32, u32),
        d: u32,
        eax: u32,
    ) -> u32 {
        let x2 = u128::from(g.read::<u32>(self.w(img, k2)));
        let x3 = u128::from(g.read::<u32>(self.w(img, k3)));
        let mf = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(d + 4),
            ecx: self.w(img, node),
            eax,
            xmm: [0, 0, x2, x3],
            ..Frame::default()
        };
        crate::s4_small::move_setup(g, img, &mf).eax
    }

    /// The move's ends of the DLL's object (the call at `c`): `from` =
    /// `V`, `to` = `E + 0x40` (`base + 0x4c`), pushed under the word
    /// `push ecx` leaves (never read). Returns its `eax`.
    fn ends<G: Game>(&self, g: &mut G, img: &Image, c: u32) -> u32 {
        let to = self.base + 0x4c;
        g.write::<u32>(self.base.wrapping_sub(8), to);
        // junk read: [ebp-0x10] (`to`'s z, E + 0x48) is never written; 0
        // (brief 2026-10-24)
        g.write::<u32>(to + 8, 0);
        let ef = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(0xc),
            edx: self.base + 0x1c,
            ..Frame::default()
        };
        crate::s6_misc::move_ends_of(g, img, &ef).eax
    }

    /// The axes' record `[O, steps, sizes, mode]` at `E + 0x20` (`base +
    /// 0x2c`; `O` operand `obj`, the steps `E + 0x30`, the sizes `V`)
    /// with the steps `(0, 0, z)` and the sizes `(0, 0, s)`, then the
    /// signed axes (the call at `c`, `esp` `d` below `base`). Returns its
    /// `eax`.
    fn axes<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, obj): (u32, u32),
        (z, s, mode): (u32, u32, u32),
        d: u32,
    ) -> u32 {
        let b = self.base;
        for (o, v) in [
            (0x3cu32, 0),
            (0x1c, 0),
            (0x40, 0),
            (0x20, 0),
            (0x44, z),
            (0x24, s),
            (0x2c, self.w(img, obj)),
            (0x34, b + 0x1c),
            (0x38, mode),
        ] {
            g.write::<u32>(b + o, v);
        }
        let af = Frame {
            at: self.target(img, c),
            esp: b.wrapping_sub(d + 4),
            ecx: b + 0x2c,
            ..Frame::default()
        };
        crate::s5_misc::signed_axes(g, img, &af).eax
    }
}

/// `magic_0570`'s camera task (0x10009b30, 6312 bytes with its tables; a
/// task callback `(A)`, cdecl; an aligned /GS frame `E` of 0x54 bytes
/// below three pushes). The record `R = A[4]` keeps the state `R + 0x18`,
/// a float `R + 0x1c`, the counts `R + 0x34`/`R + 0x38` (both up at every
/// exit; `R + 0x38` is the wait) and a step `R + 0x3c`. Three dispatch
/// tables (a byte table for states up to 0xc9, then 0x384..0x387 and
/// 0x3e9..0x3ec). By the state:
/// - 0, 0xb4, 0x320, 0xa0, 0x2c0: on by one (0xa0 resets the wait);
/// - 0x96, 0x97: the summon's model node and motion, nodes and bones, the
///   sound node, the camera's vectors (slot 229), slot 296 for each byte
///   slot 288 lists; then the camera's second vector and the static link
///   (two states on);
/// - 0x98: the fade byte `(70 - wait) / K * K'`, on at 0;
/// - 0xa1, 0xc8, 0xc9: the root node and its start, the camera; the
///   nodes, the axes and the move's ends; the light's fall;
/// - 0xaa..0xac: the light set, the camera's place and its climb;
/// - 0x190..0x192, 0x19a..0x19d: the move globals, the light's growth and
///   the steps; the static node; the axes and the move's ends;
/// - 0x2bc, 0x2be, 0x3e8: the axes and the camera's vectors (slots
///   229/248, the two vectors);
/// - 0x384..0x387, 0x3e9..0x3ec: the static node, the move set-ups, the
///   lights, the camera's way;
/// - 0x2710: the nodes leave, the lists unlink, the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn camera_task_0570<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; sub esp, 0x54; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x54);
    g.write::<u32>(e + 0x50, g.read::<u32>(w(0xa)) ^ e);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let c = Ctx {
        f,
        base: e.wrapping_sub(0xc),
        r,
    };
    let state = g.read::<u32>(r + 0x18);
    let set = |g: &mut G, s: u32| g.write::<u32>(r + 0x18, s);
    let eax = if state <= 0xc9 {
        let tb = img.word(w(0x41) + state) & 0xff;
        match state {
            0 | 0xb4 => {
                set(g, state + 1);
                state + 1
            }
            0x96 => {
                start_0570(g, img, &c, state);
                c.up(g);
                c.reset(g);
                second_0570(g, img, &c)
            }
            0x97 => second_0570(g, img, &c),
            0x98 => fade_0570(g, img, &c),
            0xa0 => {
                c.reset(g);
                set(g, state + 1);
                state + 1
            }
            0xa1 => {
                if c.count(g) < 0x19 {
                    tb
                } else {
                    root_0570(g, img, &c)
                }
            }
            0xc8 => nodes_0570(g, img, &c),
            0xc9 => {
                let x = sub(c.k(g, img, 0x749), c.rf(g, 0x1c));
                g.write::<f32>(w(0x756), x);
                let y = mul(c.rf(g, 0x1c), c.k(g, img, 0x763));
                c.wf(g, 0x1c, y);
                if c.k(g, img, 0x770) >= c.k(g, img, 0x777) {
                    abs_stores(g, img, f, 0x781, 0x78b);
                    c.up(g);
                }
                tb
            }
            0xaa => {
                let eax = light_0570(g, img, &c);
                c.up(g);
                climb_0570(g, img, &c);
                eax
            }
            0xab => {
                c.up(g);
                climb_0570(g, img, &c);
                tb
            }
            0xac => {
                climb_0570(g, img, &c);
                tb
            }
            _ => tb,
        }
    } else if state < 0x190 {
        state
    } else if state <= 0x192 {
        if state == 0x190 {
            c.reset(g);
            set(g, 0x191);
        }
        if state <= 0x191 && c.count(g) >= 4 {
            move_globals_0570(g, img, &c);
            c.up(g);
        }
        grow_0570(g, img, &c)
    } else if state < 0x19a {
        state - 0x192
    } else if state == 0x19a {
        static_0570(g, img, &c, state)
    } else if state == 0x19b {
        if c.count(g) < 0xa {
            0
        } else {
            let eax = ends_axes_0570(g, img, &c, state);
            abs_stores(g, img, f, 0xcd7, 0xce8);
            c.up(g);
            eax
        }
    } else if state <= 0x19d {
        if state == 0x19c && c.count(g) >= 0x19 {
            g.write::<u32>(r + 0x1c, 0);
            g.write::<u32>(r + 0x3c, 0x10);
            set(g, 0x19d);
        }
        step_0570(g, img, &c, (0xc06, 0xc0e, 0xc1b, 0xc2c));
        let s = g.read::<u32>(r + 0x3c);
        let x = g.read::<u32>(w(0xc39)).wrapping_add(s);
        g.write::<u32>(w(0xc39), x);
        s
    } else if state < 0x2bc {
        state - 0x19d
    } else if state == 0x2bc {
        turn_0570(g, img, &c, state)
    } else if state == 0x2be {
        away_0570(g, img, &c)
    } else if state == 0x2c0 || state == 0x320 {
        set(g, state + 1);
        state + 1
    } else if state < 0x320 {
        state.wrapping_sub(0x2c0)
    } else if state < 0x3e8 {
        match state.wrapping_sub(0x384) {
            0 => slide_0570(g, img, &c, state),
            1 => {
                let x = sub(c.k(g, img, 0x11fb), c.k(g, img, 0x1203));
                g.write::<f32>(w(0x120b), x);
                if c.count(g) < 0x46 {
                    1
                } else {
                    near_0570(g, img, &c)
                }
            }
            2 => {
                let x = add(c.k(g, img, 0x1396), c.rf(g, 0x1c));
                g.write::<f32>(w(0x13a3), x);
                let n = c.count(g);
                let y = add(c.rf(g, 0x1c), c.k(g, img, 0x13b4));
                c.wf(g, 0x1c, y);
                if n < 0x14 {
                    2
                } else {
                    let v = [0xc0e0_0000, 0xc220_0000, 0xc28c_0000, ONE];
                    c.vec229(g, img, (0x13cf, 0x13c9), v);
                    let eax = c.light(g, img, 0x140b, 0x13ff);
                    c.up(g);
                    eax
                }
            }
            3 => {
                if c.count(g) < 0x41 {
                    3
                } else {
                    let v = [0xc0e0_0000, 0xc1a0_0000, 0xc28c_0000, ONE];
                    c.vec229(g, img, (0x142d, 0x1427), v);
                    let eax = c.light(g, img, 0x1469, 0x145d);
                    abs_stores(g, img, f, 0x1482, 0x148c);
                    let eax = c.setup(g, img, (0x148d, 0x147e), (0x1479, 0x1471), 0, eax);
                    c.up(g);
                    eax
                }
            }
            x => x,
        }
    } else if state == 0x3e8 {
        sights_0570(g, img, &c)
    } else if state < 0x2710 {
        let i = state - 0x3e9;
        if i > 3 {
            i
        } else {
            sweep_0570(g, img, &c, state)
        }
    } else if state == 0x2710 {
        let eax = crate::sa_misc::leave(g, w(0x1745), 0x2710);
        let eax = crate::l3_common::list_unlink_current(g, img, c.target(img, 0x174f), eax);
        crate::l3_common::list_unlink_current(g, img, c.target(img, 0x1754), eax);
        crate::h6_setup::unlink_task(g, task)
    } else {
        state
    };
    for o in [0x34u32, 0x38] {
        let x = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, x);
    }
    Ret::eax(eax)
}

/// [`camera_task_0570`]'s state 0x96 up to its first step: the model node
/// with the summon's byte, slot 292 and the part's slot 255 on `V`, slot
/// 666 unless the flag, the motion's state, appends, the model's place,
/// the time left, an owned node and its bones, the model state, a second
/// owned node and the summon's bones, slots 211 and 3, the small node, the
/// sound node, the camera's first vectors (slot 229) and slot 296 for
/// every byte slot 288 lists but the DLL's own, the last's and 0xff.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let base = c.base;
    let below = base.wrapping_sub(4);
    let grp = base + 0x10;
    let p = base + 0x1c;
    // push ecx (the state, unread); push the summon's byte
    let b = u32::from(g.read::<u8>(w(0x5a)));
    g.write::<u32>(base.wrapping_sub(4), state);
    g.write::<u32>(base.wrapping_sub(8), b);
    let mf = Frame {
        at: call(0x6b),
        esp: base.wrapping_sub(0xc),
        ecx: w(0x66),
        edx: w(0x61),
        ..Frame::default()
    };
    crate::s4_misc::model_node_with(g, img, &mf);
    // call width: 1 (the model node's words stay under it)
    let eax = g.call(c.slot(g, img, 0x75, 292), 0, &[p]).eax;
    crate::m0070::part_call(g, img, call(0x8e), 255, [eax, w(0x89), p]);
    if g.read::<u8>(w(0x94)) & 1 == 0 {
        let m = gw(g, 0xb0);
        // call width: 2
        g.call(c.slot(g, img, 0x9c, 666), 0, &[m, 0]);
    }
    // the motion's cursor `[E + 4]`: the node, the mark byte
    let mark = g.read::<u8>(w(0xbc));
    g.write::<u32>(grp, w(0xc8));
    g.write::<u8>(grp + 5, mark);
    let sf = Frame {
        at: call(0xd1),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    let eax = crate::s4_summon::motion_state(g, img, &sf).eax;
    crate::sb_misc::node_append(g, img, call(0xe0), w(0xdb), w(0xd6), eax);
    let eax = crate::m0070::model_position(g, img, call(0xea), w(0xe5));
    abs_stores(g, img, f, 0xf8, 0x102);
    crate::sa_misc::node_append(g, img, call(0x103), w(0xf4), w(0xef), eax);
    g.write::<u32>(grp, w(0x10f));
    g.write::<u32>(grp + 4, w(0x117));
    crate::m0042::time_left(g, img, call(0x11c), grp, below);
    // four pushes: the owned node's words (they stay to 0x23b)
    let sp = base.wrapping_sub(0x10);
    for (i, o) in (0u32..).zip([0x130u32, 0x12b, 0x126, 0x121]) {
        g.write::<u32>(sp + 4 * i, w(o));
    }
    let of = Frame {
        at: call(0x13f),
        esp: sp.wrapping_sub(4),
        ecx: w(0x13a),
        edx: w(0x135),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // the bones of the owned node (`ecx` as the append leaves it)
    let bf = Frame {
        at: call(0x144),
        esp: sp.wrapping_sub(4),
        ecx: w(0x13a),
        ..Frame::default()
    };
    crate::s4_small::bone_rate(g, img, &bf);
    let eax = crate::sb_misc::node_append(g, img, call(0x153), w(0x14e), w(0x149), 0);
    abs_stores(g, img, f, 0x15c, 0x184);
    crate::sb_misc::node_append(g, img, call(0x185), w(0x158), w(0x149), eax);
    // the model state's record `[E + 4]` (`[esp + 0x20]` four words down)
    g.write::<u32>(grp, w(0x191));
    g.write::<u32>(grp + 4, w(0x199));
    let eax = crate::m0070::model_state(g, img, call(0x19e), grp);
    crate::sa_misc::node_append(g, img, call(0x1ad), w(0x1a8), w(0x1a3), eax);
    // four more pushes (the third `ecx`, the node just appended)
    let sp2 = base.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([w(0x1bd), w(0x1b8), w(0x1a8), w(0x1b2)]) {
        g.write::<u32>(sp2 + 4 * i, v);
    }
    abs_stores(g, img, f, 0x1c6, 0x1e4);
    let of = Frame {
        at: call(0x1e5),
        esp: sp2.wrapping_sub(4),
        ecx: w(0x1c2),
        edx: w(0x1a3),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // the bones' cursor `[E + 4]`: the owned node, the mark byte
    let mark = g.read::<u8>(w(0x1ec));
    g.write::<u32>(grp, w(0x1c2));
    g.write::<u8>(grp + 5, mark);
    let bf = Frame {
        at: call(0x1fd),
        esp: sp2.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    let m = gw(g, 0x20d);
    // call width: 2
    g.call(c.slot(g, img, 0x202, 211), 0, &[m, w(0x207)]);
    let fl = g.read::<u16>(w(0x221)) | 0x10;
    g.write::<u16>(w(0x221), fl);
    let m = gw(g, 0x228);
    abs_stores(g, img, f, 0x22c, 0x236);
    // call width: 1
    g.call(c.slot(g, img, 0x21a, 3), 0, &[m]);
    // push the small node's word
    g.write::<u32>(below, w(0x244));
    crate::m0070::node_init_small(g, img, call(0x24e), w(0x249), w(0x23f), w(0x244));
    abs_stores(g, img, f, 0x259, 0x263);
    // the sound node `[N, parent, 700.0]` (the parent `edx` as the node
    // set-up leaves it)
    for (i, v) in (0u32..).zip([w(0x267), w(0x23f), 0x442f_0000]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x277, 0x281);
    crate::o6_fam_node::node_reset_sound(g, img, call(0x282), grp);
    abs_stores(g, img, f, 0x286, 0x28d);
    abs_stores(g, img, f, 0x29d, 0x2b1);
    // the camera's first vectors: `E + 0x20` through slot 229, both copies
    let v = base + 0x2c;
    for (i, x) in (0u32..).zip([0, 0xc180_0000, 0xc080_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(c.slot(g, img, 0x299, 229), 0, &[v, w(0x293), v]);
    for (i, (a, b)) in (0u32..).zip([
        (0x2e3u32, 0x2ebu32),
        (0x2f9, 0x301),
        (0x31c, 0x324),
        (0x332, 0x33a),
    ]) {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(w(a), x);
        g.write::<u32>(w(b), x);
    }
    abs_stores(g, img, f, 0x33e, 0x37a);
    for (i, x) in (0u32..).zip([0x4120_0000, 0, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // slot 296 for each byte slot 288 lists (the count at `E + 4`)
    let n_at = grp;
    // call width: 2
    let list = g.call(c.slot(g, img, 0x314, 288), 0, &[0x10, n_at]).eax;
    let mut n = g.read::<i32>(n_at);
    let mut i = 0i32;
    let mut at = list;
    while i < n {
        let b = g.read::<u8>(at);
        if b != g.read::<u8>(w(0x3b4)) && u32::from(b) != gw(g, 0x3bf) && b != 0xff {
            // call width: 2
            g.call(c.slot(g, img, 0x3ce, 296), 0, &[u32::from(b), 0]);
            n = g.read::<i32>(n_at);
        }
        i += 1;
        at = at.wrapping_add(1);
    }
}

/// [`camera_task_0570`]'s state 0x97 (and 0x96's end): the camera's second
/// vector (slot 229) into its globals, the static link, two flags; the
/// state on, the wait reset. Returns the static link's `eax`.
fn second_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    c.vec229(g, img, (0x3fd, 0x3f7), [0, 0xbf80_0000, 0xc248_0000, ONE]);
    c.copy4(g, img, [0x433, 0x441, 0x44f, 0x460]);
    abs_stores(g, img, f, 0x464, 0x475);
    let lf = Frame {
        at: c.target(img, 0x476),
        esp: c.base.wrapping_sub(4),
        ..Frame::default()
    };
    let eax = crate::s4_small::static_link_last_word(g, img, &lf).eax;
    abs_stores(g, img, f, 0x47a, 0x495);
    c.up(g);
    c.reset(g);
    eax
}

/// [`camera_task_0570`]'s state 0x98: the fade byte `(70 - wait) / K *
/// K'` (truncated; its low byte kept), on when it is 0. Returns the
/// truncated value.
fn fade_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let n = 0x46i32.wrapping_sub(c.count(g));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let x = mul(div(n as f32, c.k(g, img, 0x4b7)), c.k(g, img, 0x4bf));
    let v = cvtt(x).cast_unsigned();
    g.write::<u8>(c.w(img, 0x4c8), v as u8);
    if v as u8 == 0 {
        c.up(g);
    }
    v
}

/// [`camera_task_0570`]'s state 0xa1 past its wait: the root node and its
/// start (`xmm1` the rate), the camera's vector (slot 229) into its
/// globals, two flags; state 0xc8. Returns slot 229's `eax`.
fn root_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let node = c.w(img, 0x4f9);
    crate::m0070::node_init_root(g, img, c.target(img, 0x4fe), node);
    let rate = u128::from(g.read::<u32>(c.w(img, 0x506)));
    abs_stores(g, img, f, 0x50a, 0x55a);
    let nf = Frame {
        at: c.target(img, 0x55b),
        esp: c.base.wrapping_sub(4),
        ecx: node,
        xmm: [0, rate, 0, 0],
        ..Frame::default()
    };
    crate::sa_misc::node_start(g, img, &nf);
    let v = [0xc234_0000, 0xc1f0_0000, 0xc22c_0000, ONE];
    let eax = c.vec229(g, img, (0x56b, 0x565), v);
    c.copy4(g, img, [0x5a1, 0x5af, 0x5bd, 0x5cb]);
    abs_stores(g, img, f, 0x5cf, 0x5e0);
    g.write::<u32>(c.r + 0x18, 0xc8);
    eax
}

/// [`camera_task_0570`]'s state 0xc8: two appends, an owned node, the
/// small node under it, its word kept, the axes `(0, 0, 0xe000)` /
/// `(0, 0, 0.98)` and the move's ends `(160, 0, -) -> (0.98, 0, -)`; on,
/// `R + 0x1c` 3.0. Returns the move's ends' `eax`.
fn nodes_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let base = c.base;
    let list = w(0x5f0);
    crate::sa_misc::node_append(g, img, call(0x5fa), w(0x5f5), list, 0);
    abs_stores(g, img, f, 0x603, 0x621);
    crate::sb_misc::node_append(g, img, call(0x622), w(0x5ff), list, 0);
    // four pushes (the third `ecx`, the node just appended)
    let sp = base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x632), w(0x5ff), w(0x62c), w(0x627)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x63b, 0x663);
    let owned = w(0x637);
    let of = Frame {
        at: call(0x664),
        esp: sp.wrapping_sub(4),
        ecx: owned,
        edx: list,
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // push ecx (the owned node, as the append leaves it): the small node's
    // word
    g.write::<u32>(sp.wrapping_sub(4), owned);
    abs_stores(g, img, f, 0x66e, 0x675);
    let small = w(0x66a);
    crate::m0070::node_init_small(g, img, call(0x676), small, list, owned);
    g.write::<u32>(base + 0x30, base + 0x3c);
    g.write::<u32>(w(0x684), small);
    abs_stores(g, img, f, 0x693, 0x6bb);
    c.axes(g, img, (0x700, 0x6ef), (0xe000, 0x3f7a_e148, 0), 0);
    let p = base + 0x1c;
    for (o, v) in [(0u32, 0x4320_0000), (4, 0), (0x30, 0x3f7a_e148), (0x34, 0)] {
        g.write::<u32>(p + o, v);
    }
    let eax = c.ends(g, img, 0x72f);
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0x4040_0000);
    eax
}

/// [`camera_task_0570`]'s state 0xaa: the camera's place (slot 229) and
/// the light set, the move globals, the camera's way `(-2000, 0, 0)`; on,
/// `R + 0x1c` -100.0. Returns the light set's `eax`.
fn light_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    c.vec229(g, img, (0x79f, 0x799), [0, 0xc160_0000, 0, ONE]);
    let eax = c.light(g, img, 0x7db, 0x7cf);
    abs_stores(g, img, f, 0x7df, 0x865);
    let p = c.base + 0x1c;
    for (i, v) in (0u32..).zip([0xc4fa_0000, 0, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0x87d, 0x8b9);
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0xc2c8_0000);
    eax
}

/// [`camera_task_0570`]'s states 0xab/0xac: the step `+0x60` on, the
/// camera's way `(R + 0x1c, 0, 0)` into its globals, `R + 0x1c` up by
/// `K - K'` and the light's level down by `K''`.
fn climb_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let x = g.read::<u32>(w(0x8c8)).wrapping_add(0x60);
    g.write::<u32>(w(0x8c8), x);
    let v = c.rf(g, 0x1c);
    let p = c.base + 0x1c;
    g.write::<f32>(p, v);
    g.write::<f32>(w(0x8dc), v);
    g.write::<f32>(w(0x8e4), v);
    g.write::<u32>(p + 4, 0);
    g.write::<u32>(p + 8, 0);
    abs_stores(g, img, f, 0x8f8, 0x920);
    let y = sub(add(c.rf(g, 0x1c), c.k(g, img, 0x929)), c.k(g, img, 0x931));
    c.wf(g, 0x1c, y);
    let z = sub(c.k(g, img, 0x93e), c.k(g, img, 0x946));
    g.write::<f32>(w(0x94e), z);
}

/// [`camera_task_0570`]'s state 0x191 past its wait: the move globals
/// (the way `to - from`, the flags' bit 1 cleared and bit 0 set, the
/// rates). Returns the flags.
fn move_globals_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let a = sub(k(g, 0x972), k(g, 0x97a));
    let fl = (g.read::<u32>(w(0x97f)) & !2) | 1;
    let b = sub(0.0, k(g, 0x98a));
    g.write::<f32>(w(0x995), a);
    let x = sub(k(g, 0x99d), k(g, 0x9a5));
    abs_stores(g, img, f, 0x9ac, 0x9ca);
    g.write::<f32>(w(0x9ce), x);
    let y = sub(k(g, 0x9d6), k(g, 0x9de));
    g.write::<f32>(w(0x9e6), b);
    abs_stores(g, img, f, 0x9ea, 0x9fe);
    g.write::<u32>(w(0x9ff), fl);
    g.write::<f32>(w(0xa07), y);
    let z = sub(k(g, 0xa0f), k(g, 0xa17));
    g.write::<f32>(w(0xa1f), z);
    fl
}

/// A light's growth: `R + 0x1c` up by `K` (operand `ka`); while `K'`
/// (`kb`) is above the level `L` (`kl`), `L` (`ks`) becomes `R + 0x1c +
/// L`; then the step `R + 0x3c` up by 8.
fn step_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, (ka, kb, kl, ks): (u32, u32, u32, u32)) {
    let x = add(c.rf(g, 0x1c), c.k(g, img, ka));
    let limit = c.k(g, img, kb);
    c.wf(g, 0x1c, x);
    let level = c.k(g, img, kl);
    if limit > level {
        g.write::<f32>(c.w(img, ks), add(x, level));
    }
    let s = g.read::<u32>(c.r + 0x3c).wrapping_add(8);
    g.write::<u32>(c.r + 0x3c, s);
}

/// [`camera_task_0570`]'s state 0x192 (and 0x191's end): the light's
/// growth, then the move's `x` down by the step while above -0x800 and
/// its `y` up by it while below 0x5800. Returns the last `y` read or
/// stored.
fn grow_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let w = |o: u32| c.w(img, o);
    step_0570(g, img, c, (0xa2f, 0xa37, 0xa44, 0xa55));
    let x = g.read::<i32>(w(0xa5e));
    if x > -0x800 {
        let s = g.read::<i32>(c.r + 0x3c);
        g.write::<i32>(w(0xa6d), x.wrapping_sub(s));
    }
    let y = g.read::<i32>(w(0xa72));
    if y >= 0x5800 {
        return y.cast_unsigned();
    }
    let y = y.wrapping_add(g.read::<i32>(c.r + 0x3c));
    g.write::<i32>(w(0xa85), y);
    y.cast_unsigned()
}

/// [`camera_task_0570`]'s state 0x19a: the static node (one stack word,
/// over the state `push ecx` leaves), the move globals (the flags' bit 1
/// cleared and bit 0 set, `K - 0.0`); on, the wait reset. Returns the
/// flags.
fn static_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let f = c.f;
    let base = c.base;
    g.write::<u32>(base.wrapping_sub(4), state);
    g.write::<u32>(base.wrapping_sub(8), c.w(img, 0xac5));
    let sf = Frame {
        at: c.target(img, 0xacf),
        esp: base.wrapping_sub(0xc),
        ecx: c.w(img, 0xaca),
        ..Frame::default()
    };
    crate::s4_misc::static_node(g, img, &sf);
    let fl = (g.read::<u32>(c.w(img, 0xad4)) & !2) | 1;
    let x = sub(c.k(g, img, 0xadc), 0.0);
    abs_stores(g, img, f, 0xaed, 0xb83);
    g.write::<f32>(c.w(img, 0xb87), x);
    abs_stores(g, img, f, 0xb8b, 0xba9);
    g.write::<u32>(c.w(img, 0xbaa), fl);
    c.up(g);
    c.reset(g);
    fl
}

/// [`camera_task_0570`]'s state 0x19b past its wait: the move's ends
/// `(-200, -20, -) -> (0.98, 0.98, -)` (over the state `push ecx`
/// leaves), then the axes `(0, 0, 0x2000)` / `(0, 0, 0.025)` in mode 1.
/// Returns the axes' `eax`.
fn ends_axes_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let base = c.base;
    g.write::<u32>(base.wrapping_sub(4), state);
    let p = base + 0x1c;
    for (o, v) in [
        (0u32, 0xc348_0000),
        (4, 0xc1a0_0000),
        (0x30, 0x3f7a_e148),
        (0x34, 0x3f7a_e148),
    ] {
        g.write::<u32>(p + o, v);
    }
    // junk read: [esp+0x24] (`from`'s z, E + 0x18) is never written before
    // the move's ends read it; 0 (brief 2026-10-24)
    g.write::<u32>(p + 8, 0);
    c.ends(g, img, 0xc77);
    g.write::<u32>(base + 0x30, base + 0x3c);
    c.axes(g, img, (0xcd3, 0xcc2), (0x2000, 0x3ccc_cccd, 1), 0)
}

/// [`camera_task_0570`]'s state 0x2bc: the move's ends `(-200, -20, -) ->
/// (0, 0, -)` and the axes `(0, 0, 0x2000)` / `(0, 0, 0)` in mode 1 (both
/// over the state `push ecx` leaves), the flags (bit 0 set, bits 1 and 3
/// cleared), the camera's place and aim (slots 229 and 248 twice, into
/// its globals and the two vectors), its way `(60, -10, 0)`; on. Returns
/// the two vectors' `eax`.
fn turn_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let base = c.base;
    g.write::<u32>(base.wrapping_sub(4), state);
    let p = base + 0x1c;
    for (o, v) in [(0u32, 0xc348_0000), (4, 0xc1a0_0000), (0x30, 0), (0x34, 0)] {
        g.write::<u32>(p + o, v);
    }
    // junk read: [esp+0x24] (`from`'s z, E + 0x18) is never written before
    // the move's ends read it; 0 (brief 2026-10-24)
    g.write::<u32>(p + 8, 0);
    c.ends(g, img, 0xd1b);
    g.write::<u32>(base + 0x30, base + 0x3c);
    // the move's ends' two words stay under the axes
    c.axes(g, img, (0xd74, 0xd63), (0x2000, 0, 1), 8);
    let fl = (g.read::<u32>(w(0xd79)) | 1) & !0xa;
    g.write::<u32>(w(0xd84), fl);
    abs_stores(g, img, f, 0xd98, 0xda9);
    c.vec229(
        g,
        img,
        (0xd94, 0xd8e),
        [0x41f0_0000, 0xc110_0000, 0xc1c8_0000, 0],
    );
    // call width: 3
    g.call(c.slot(g, img, 0xde0, 248), 0, &[p, w(0xdda), p]);
    c.copy4(g, img, [0xdf6, 0xe0c, 0xe1a, 0xe33]);
    abs_stores(g, img, f, 0xe37, 0xe5c);
    c.vec229(g, img, (0xe2b, 0xe25), [0, 0x4020_0000, 0, 0]);
    // call width: 3
    let eax = g.call(c.slot(g, img, 0xe93, 248), 0, &[p, w(0xe8d), p]).eax;
    let tf = Frame {
        at: c.target(img, 0xea7),
        esp: base.wrapping_sub(4),
        edx: p,
        eax,
        ..Frame::default()
    };
    let eax = crate::s4_small::two_vectors(g, img, &tf).eax;
    for (i, v) in (0u32..).zip([0x4270_0000, 0xc120_0000, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0xec3, 0xeff);
    c.up(g);
    eax
}

/// [`camera_task_0570`]'s state 0x2be: the camera's place and aim (slots
/// 229 and 248) into its globals, the light words (`L - K`), the angle
/// word 0; on. Returns 0.
fn away_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let p = c.base + 0x1c;
    c.vec229(g, img, (0xf2e, 0xf28), [0x4120_0000, 0, 0xc28c_0000, 0]);
    // call width: 3
    g.call(c.slot(g, img, 0xf69, 248), 0, &[p, w(0xf63), p]);
    c.copy4(g, img, [0xf7f, 0xf8d, 0xf9b, 0xfa9]);
    let x = sub(c.k(g, img, 0xfb1), c.k(g, img, 0xfb9));
    abs_stores(g, img, f, 0xfc2, 0xff1);
    g.write::<f32>(w(0xff5), x);
    g.write::<u16>(w(0xffb), 0);
    c.up(g);
    0
}

/// [`camera_task_0570`]'s state 0x384: the static node (over the state
/// `push ecx` leaves), the move globals and set-up, the camera's place
/// (slot 229) and the light set, the move globals, the move's ends `(0,
/// 0, -) -> (0, 0, -)`, the light words; on, the wait reset. Returns 0.
fn slide_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let base = c.base;
    g.write::<u32>(base.wrapping_sub(4), state);
    g.write::<u32>(base.wrapping_sub(8), w(0x102f));
    let sf = Frame {
        at: c.target(img, 0x1039),
        esp: base.wrapping_sub(0xc),
        ecx: w(0x1034),
        ..Frame::default()
    };
    // the static node keeps `ecx`
    let eax = crate::s4_misc::static_node(g, img, &sf).eax;
    abs_stores(g, img, f, 0x1048, 0x10ac);
    c.setup(g, img, (0x10ad, 0x1034), (0x1041, 0x1041), 8, eax);
    let v = [0xc000_0000, 0xc1a0_0000, 0xc28c_0000, ONE];
    c.vec229(g, img, (0x10bd, 0x10b7), v);
    c.light(g, img, 0x10f9, 0x10ed);
    abs_stores(g, img, f, 0x1107, 0x1186);
    let p = base + 0x1c;
    for (o, v) in [(0u32, 0), (4, 0), (0x30, 0), (0x34, 0)] {
        g.write::<u32>(p + o, v);
    }
    c.ends(g, img, 0x11a7);
    let x = sub(c.k(g, img, 0x11af), c.k(g, img, 0x11b7));
    abs_stores(g, img, f, 0x11bd, 0x11db);
    g.write::<f32>(w(0x11df), x);
    g.write::<u16>(w(0x11e5), 0);
    c.up(g);
    c.reset(g);
    0
}

/// [`camera_task_0570`]'s state 0x385 past its wait: the camera's place
/// (slot 229) and the light set, the light words, the move globals (the
/// angles -pi/8, the word 0xf800) and set-up; on, `R + 0x1c` 0, the wait
/// reset. Returns the set-up's `eax` (0xfffff800).
fn near_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let v = [0xc140_0000, 0xc1c8_0000, 0xc28c_0000, ONE];
    c.vec229(g, img, (0x1225, 0x121f), v);
    c.light(g, img, 0x1261, 0x1255);
    let x = sub(c.k(g, img, 0x1269), c.k(g, img, 0x1271));
    g.write::<u16>(w(0x1289), 0);
    abs_stores(g, img, f, 0x1297, 0x1319);
    g.write::<f32>(w(0x131d), x);
    abs_stores(g, img, f, 0x1321, 0x1335);
    g.write::<u16>(w(0x1337), 0xf800);
    abs_stores(g, img, f, 0x133b, 0x1377);
    let eax = c.setup(g, img, (0x1378, 0x1293), (0x1281, 0x1279), 0, 0xffff_f800);
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0);
    c.reset(g);
    eax
}

/// [`camera_task_0570`]'s state 0x3e8: the flags (bit 0 set, bits 1 and 3
/// cleared), the camera's place (slot 229 of the DLL's vector) into both
/// copies, slot 325, the move globals, the camera's way `(160, 0, 0)`;
/// on, the wait reset, `R + 0x1c` 0.05. Returns slot 325's `eax`.
fn sights_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let p = c.base + 0x1c;
    let fl = (g.read::<u32>(w(0x149a)) | 1) & !0xa;
    g.write::<u32>(w(0x14a5), fl);
    abs_stores(g, img, f, 0x14bd, 0x14c7);
    // call width: 3
    g.call(c.slot(g, img, 0x14b9, 229), 0, &[p, w(0x14b3), w(0x14aa)]);
    for (i, (a, b)) in (0u32..).zip([(0x14deu32, 0x14e6u32), (0x14f4, 0x14fc), (0x150d, 0x1515)]) {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(w(a), x);
        g.write::<u32>(w(b), x);
    }
    let x = g.read::<u32>(p + 0xc);
    abs_stores(g, img, f, 0x151f, 0x1530);
    g.write::<u32>(w(0x1534), x);
    g.write::<u32>(w(0x153c), x);
    // call width: 2
    let eax = g
        .call(c.slot(g, img, 0x14d6, 325), 0, &[w(0x154c), w(0x1547)])
        .eax;
    abs_stores(g, img, f, 0x1552, 0x15b3);
    for (i, v) in (0u32..).zip([0x4320_0000, 0, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0x15cb, 0x1607);
    c.up(g);
    c.reset(g);
    g.write::<u32>(c.r + 0x1c, 0x3d4c_cccd);
    eax
}

/// [`camera_task_0570`]'s states 0x3e9..0x3ec (`xmm3` the float at
/// `+0x1643` in all): 0x3e9 on into 0x3ea; 0x3ea past its wait the move
/// set-up, on; 0x3eb `R + 0x1c` up by `xmm3`, the light's place moved, on
/// past its wait; 0x3ec slot 325, the camera's place (slot 229, `z` up by
/// `K`, `y` down by `R + 0x1c`) and the two vectors. Returns the two
/// vectors' `eax`.
fn sweep_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // `eax`: 0x3ea (0x3e9's step) or 1 (the dispatch's)
    let eax = if state == 0x3e9 {
        g.write::<u32>(c.r + 0x18, 0x3ea);
        0x3ea
    } else {
        state - 0x3e9
    };
    if state <= 0x3ea && c.count(g) >= 0xa {
        abs_stores(g, img, f, 0x1667, 0x167b);
        c.setup(g, img, (0x167c, 0x1663), (0x165e, 0x1643), 0, eax);
        c.up(g);
    }
    if state <= 0x3eb {
        let x = add(c.rf(g, 0x1c), k(g, 0x1643));
        c.wf(g, 0x1c, x);
        for (a, kk, b, plus) in [
            (0x1695u32, 0x169du32, 0x16a5u32, true),
            (0x16ad, 0x16b5, 0x16bd, true),
            (0x16c5, 0x16cd, 0x16d5, false),
        ] {
            let y = if plus {
                add(k(g, a), k(g, kk))
            } else {
                sub(k(g, a), k(g, kk))
            };
            g.write::<f32>(w(b), y);
        }
        if c.count(g) >= 0x1e {
            c.up(g);
        }
    }
    let p = c.base + 0x1c;
    // call width: 2
    g.call(c.slot(g, img, 0x16ed, 325), 0, &[p, w(0x16e7)]);
    let z = add(g.read::<f32>(p + 8), k(g, 0x1703));
    g.write::<f32>(p + 8, z);
    let y = sub(g.read::<f32>(p + 4), c.rf(g, 0x1c));
    g.write::<f32>(p + 4, y);
    // call width: 3
    let eax = g
        .call(c.slot(g, img, 0x1713, 229), 0, &[p, w(0x170d), p])
        .eax;
    let tf = Frame {
        at: c.target(img, 0x173e),
        esp: c.base.wrapping_sub(4),
        edx: p,
        eax,
        ..Frame::default()
    };
    crate::s4_small::two_vectors(g, img, &tf).eax
}

/// [`camera_task_0570`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const CAMERA_TASK_0570: Sig = Sig {
    len: 6312,
    hash: 0x2480_1371_830c_ad4d,
    calls: &[
        (0x6b, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x8e, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0xd1, Callee::Sig(&crate::s4_summon::MOTION_STATE)),
        (0xe0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xea, Callee::Sig(&crate::m0070::MODEL_POSITION)),
        (0x103, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x11c, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x13f, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x144, Callee::Sig(&crate::s4_small::BONE_RATE)),
        (0x153, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x185, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x19e, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x1ad, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x1e5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1fd, Callee::Sig(&crate::sa_summon::BONE)),
        (0x24e, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x282, Callee::Any),
        (0x476, Callee::Sig(&crate::s4_small::STATIC_LINK_LAST_WORD)),
        (0x4fe, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x55b, Callee::Sig(&crate::sa_misc::NODE_START)),
        (0x5fa, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x622, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x664, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x676, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x700, Callee::Sig(&crate::s5_misc::SIGNED_AXES)),
        (0x72f, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0x7db, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0xacf, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0xc77, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0xcd3, Callee::Sig(&crate::s5_misc::SIGNED_AXES)),
        (0xd1b, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0xd74, Callee::Sig(&crate::s5_misc::SIGNED_AXES)),
        (0xea7, Callee::Sig(&crate::s4_small::TWO_VECTORS)),
        (0x1039, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x10ad, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x10f9, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x11a7, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0x1261, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x1378, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x140b, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x1469, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x148d, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x167c, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x173e, Callee::Sig(&crate::s4_small::TWO_VECTORS)),
        (0x174a, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x174f, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x1754, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x1782, Callee::Any),
    ],
    imports: &[],
};
