//! Round 46's ports (own rows of the DLLs below `magic_0250`): the DLLs'
//! own code written out, in tested pieces.

// `comiss` with `jb`/`jbe`: an unordered compare takes one side
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::r25::abs_stores;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// [`camera_task_0070`]'s code offset, frame and record: `base` is `E -
/// 0xc` (the aligned frame `E` below the three pushes, so `[esp + x]` is
/// `base + x` with nothing pushed), `r` the task's record. Its locals as
/// `magic_0570`'s camera task's, but the move's `to` sits at `base +
/// 0x3c` and the axes' steps at `base + 0x4c` (the other way round there).
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

    /// Slot 520 (no arguments) through the imports pointer at operand
    /// `op`: its answer.
    fn s520<G: Game>(&self, g: &mut G, img: &Image, op: u32) -> u32 {
        // call width: 0
        g.call(self.slot(g, img, op, 520), 0, &[]).eax
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

    /// The camera's vector `V = E + 0x10` (`base + 0x1c`).
    fn p(&self) -> u32 {
        self.base + 0x1c
    }

    /// The vector `v` into `V`, then slot 229 `(V, M, V)` (`M` operand
    /// `m`, the imports pointer operand `imp`). Returns slot 229's `eax`.
    fn vec229<G: Game>(&self, g: &mut G, img: &Image, (imp, m): (u32, u32), v: [u32; 4]) -> u32 {
        let p = self.p();
        for (i, x) in (0u32..).zip(v) {
            g.write::<u32>(p + 4 * i, x);
        }
        // call width: 3
        g.call(self.slot(g, img, imp, 229), 0, &[p, self.w(img, m), p])
            .eax
    }

    /// Slot 248 `(V, M, V)` (`M` operand `m`, the imports pointer operand
    /// `imp`). Returns its `eax`.
    fn aim248<G: Game>(&self, g: &mut G, img: &Image, (imp, m): (u32, u32)) -> u32 {
        let p = self.p();
        // call width: 3
        g.call(self.slot(g, img, imp, 248), 0, &[p, self.w(img, m), p])
            .eax
    }

    /// The four words of `V` into the globals the operands `ops` name.
    fn copy4<G: Game>(&self, g: &mut G, img: &Image, ops: [u32; 4]) {
        for (i, o) in (0u32..).zip(ops) {
            let x = g.read::<u32>(self.p() + 4 * i);
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
            edx: self.p(),
            xmm: [0, 0, r, 0],
            ..Frame::default()
        };
        crate::s4_small::light_set(g, img, &lf).eax
    }

    /// The move set-up (the call at `c`) of the node at operand `node`
    /// with `xmm2`/`xmm3` the floats at operands `k2`/`k3`. Returns its
    /// `eax` (`eax` as it came in).
    fn setup<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node): (u32, u32),
        (k2, k3): (u32, u32),
        eax: u32,
    ) -> u32 {
        let x2 = u128::from(g.read::<u32>(self.w(img, k2)));
        let x3 = u128::from(g.read::<u32>(self.w(img, k3)));
        let mf = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(4),
            ecx: self.w(img, node),
            eax,
            xmm: [0, 0, x2, x3],
            ..Frame::default()
        };
        crate::s4_small::move_setup(g, img, &mf).eax
    }

    /// The move's ends of the DLL's object (the call at `c`): `from` =
    /// `V` (`x`, `y` as given; its `z` as the frame holds it), `to` = `E +
    /// 0x30` (`base + 0x3c`, its `x`, `y` as given), pushed under the word
    /// `push ecx` leaves (never read). Returns its `eax`.
    fn ends<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        c: u32,
        (fx, fy, tx, ty): (u32, u32, u32, u32),
    ) -> u32 {
        let p = self.p();
        let to = self.base + 0x3c;
        g.write::<u32>(self.base.wrapping_sub(8), to);
        for (a, v) in [(p, fx), (p + 4, fy), (to, tx), (to + 4, ty)] {
            g.write::<u32>(a, v);
        }
        // junk read: [esp+0x4c] (`to`'s z, E + 0x38) is never written; 0
        // (brief 2026-10-24)
        g.write::<u32>(to + 8, 0);
        let ef = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(0xc),
            edx: p,
            ..Frame::default()
        };
        crate::s6_misc::move_ends_of(g, img, &ef).eax
    }

    /// The axes' record `[O, steps, sizes, mode]` at `E + 0x20` (`base +
    /// 0x2c`; `O` operand `obj`, the steps `E + 0x40`, the sizes `V`)
    /// with the steps `(0, 0, z)` and the sizes `(0, 0, s)`, then the
    /// signed axes (the call at `c`). Returns its `eax`.
    fn axes<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, obj): (u32, u32),
        (z, s, mode): (u32, u32, u32),
    ) -> u32 {
        let b = self.base;
        for (o, v) in [
            (0x30u32, b + 0x4c),
            (0x4c, 0),
            (0x1c, 0),
            (0x50, 0),
            (0x20, 0),
            (0x54, z),
            (0x24, s),
            (0x2c, self.w(img, obj)),
            (0x34, b + 0x1c),
            (0x38, mode),
        ] {
            g.write::<u32>(b + o, v);
        }
        let af = Frame {
            at: self.target(img, c),
            esp: b.wrapping_sub(4),
            ecx: b + 0x2c,
            ..Frame::default()
        };
        crate::s5_misc::signed_axes(g, img, &af).eax
    }
}

/// `magic_0070`'s camera task (0x10009e30, 6308 bytes with its tables; a
/// task callback `(A)`, cdecl; an aligned /GS frame `E` of 0x54 bytes
/// below three pushes): `magic_0570`'s camera task
/// ([`camera_task_0570`](crate::r37::camera_task_0570)) with its own
/// blocks. The record `R = A[4]` keeps the state `R + 0x18`, a float `R +
/// 0x1c`, the counts `R + 0x34`/`R + 0x38` (both up at every exit; `R +
/// 0x38` is the wait) and a step `R + 0x3c`. The same three dispatch
/// tables and states; what differs from 0570's:
/// - 0x96: slot 520's answer nudges the model's place (0x2350000 and
///   0x2360000: `x - K`, `z + K'`; 0x2370000: `z - K''`; 0x2380000: `x -
///   K`) and is the part call's `eax`; a second answer of 0x2370000 takes
///   0x4000 off a word; the summon's bones (`bone_plain` on the owned
///   node), slot 228 `(byte, 0xd8)` and the release of a handle before the
///   small node; slot 288's list is walked only when a third answer is not
///   0x2370000;
/// - 0x97: the node ease of the DLL's object after the static link (its
///   `eax` returned);
/// - 0x2bc: no move's ends and axes; 0x384: no static node;
/// - the move's `to` and the axes' steps trade places in the frame.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn camera_task_0070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
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
                start_0070(g, img, &c, state);
                c.up(g);
                c.reset(g);
                second_0070(g, img, &c)
            }
            0x97 => second_0070(g, img, &c),
            0x98 => fade_0070(g, img, &c),
            0xa0 => {
                c.reset(g);
                set(g, state + 1);
                state + 1
            }
            0xa1 => {
                if c.count(g) < 0x19 {
                    tb
                } else {
                    root_0070(g, img, &c)
                }
            }
            0xc8 => nodes_0070(g, img, &c),
            0xc9 => {
                let x = sub(c.k(g, img, 0x833), c.rf(g, 0x1c));
                g.write::<f32>(w(0x840), x);
                let y = mul(c.rf(g, 0x1c), c.k(g, img, 0x84d));
                c.wf(g, 0x1c, y);
                if c.k(g, img, 0x85a) >= c.k(g, img, 0x861) {
                    abs_stores(g, img, f, 0x86b, 0x875);
                    c.up(g);
                }
                tb
            }
            0xaa => {
                let eax = light_0070(g, img, &c);
                c.up(g);
                climb_0070(g, img, &c);
                eax
            }
            0xab => {
                c.up(g);
                climb_0070(g, img, &c);
                tb
            }
            0xac => {
                climb_0070(g, img, &c);
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
            move_globals_0070(g, img, &c);
            c.up(g);
        }
        grow_0070(g, img, &c)
    } else if state < 0x19a {
        state - 0x192
    } else if state == 0x19a {
        static_0070(g, img, &c, state)
    } else if state == 0x19b {
        if c.count(g) < 0xa {
            0
        } else {
            let eax = ends_axes_0070(g, img, &c, state);
            abs_stores(g, img, f, 0xdc1, 0xdd2);
            c.up(g);
            eax
        }
    } else if state <= 0x19d {
        if state == 0x19c && c.count(g) >= 0x19 {
            g.write::<u32>(r + 0x1c, 0);
            g.write::<u32>(r + 0x3c, 0x10);
            set(g, 0x19d);
        }
        step_0070(g, img, &c, (0xcf0, 0xcf8, 0xd05, 0xd16));
        let s = g.read::<u32>(r + 0x3c);
        let x = g.read::<u32>(w(0xd23)).wrapping_add(s);
        g.write::<u32>(w(0xd23), x);
        s
    } else if state < 0x2bc {
        state - 0x19d
    } else if state == 0x2bc {
        turn_0070(g, img, &c)
    } else if state == 0x2be {
        away_0070(g, img, &c)
    } else if state == 0x2c0 || state == 0x320 {
        set(g, state + 1);
        state + 1
    } else if state < 0x320 {
        state.wrapping_sub(0x2c0)
    } else if state < 0x3e8 {
        match state.wrapping_sub(0x384) {
            0 => slide_0070(g, img, &c),
            1 => {
                let x = sub(c.k(g, img, 0x11f8), c.k(g, img, 0x1200));
                g.write::<f32>(w(0x1208), x);
                if c.count(g) < 0x46 {
                    1
                } else {
                    near_0070(g, img, &c)
                }
            }
            2 => {
                // (`R + 0x1c` first here: 0570 adds the other way round)
                let x = add(c.rf(g, 0x1c), c.k(g, img, 0x1398));
                g.write::<f32>(w(0x13a0), x);
                let n = c.count(g);
                let y = add(c.rf(g, 0x1c), c.k(g, img, 0x13b1));
                c.wf(g, 0x1c, y);
                if n < 0x14 {
                    2
                } else {
                    let v = [0xc0e0_0000, 0xc220_0000, 0xc28c_0000, ONE];
                    c.vec229(g, img, (0x13cc, 0x13c6), v);
                    let eax = c.light(g, img, 0x1408, 0x13fc);
                    c.up(g);
                    eax
                }
            }
            3 => {
                if c.count(g) < 0x41 {
                    3
                } else {
                    let v = [0xc0e0_0000, 0xc1a0_0000, 0xc28c_0000, ONE];
                    c.vec229(g, img, (0x142a, 0x1424), v);
                    let eax = c.light(g, img, 0x1466, 0x145a);
                    abs_stores(g, img, f, 0x147f, 0x1489);
                    let eax = c.setup(g, img, (0x148a, 0x147b), (0x1476, 0x146e), eax);
                    c.up(g);
                    eax
                }
            }
            x => x,
        }
    } else if state == 0x3e8 {
        sights_0070(g, img, &c)
    } else if state < 0x2710 {
        let i = state - 0x3e9;
        if i > 3 {
            i
        } else {
            sweep_0070(g, img, &c, state)
        }
    } else if state == 0x2710 {
        let eax = crate::sa_misc::leave(g, w(0x1742), 0x2710);
        let eax = crate::l3_common::list_unlink_current(g, img, c.target(img, 0x174c), eax);
        crate::l3_common::list_unlink_current(g, img, c.target(img, 0x1751), eax);
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

/// [`camera_task_0070`]'s state 0x96 up to its first step: the model node
/// with the summon's byte, slot 292 on `V` and slot 520's nudge, the
/// part's slot 255, slot 666 unless the flag, the motion's state, appends,
/// the model's place, the time left, slot 520's cut, an owned node and its
/// bones, the model state, a second owned node and the summon's bones,
/// slots 211 and 3, slot 228 and the release, the small node, the sound
/// node, the camera's first vectors (slot 229) and, unless slot 520
/// answers 0x2370000, slot 296 for every byte slot 288 lists but the DLL's
/// own, the last's and 0xff.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let base = c.base;
    let below = base.wrapping_sub(4);
    let grp = base + 0x10;
    let p = c.p();
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
    g.call(c.slot(g, img, 0x75, 292), 0, &[p]);
    // slot 520's answer nudges the place `V`
    let a = c.s520(g, img, 0x82);
    match a {
        0x0235_0000 | 0x0236_0000 => {
            let x = sub(g.read::<f32>(p), c.k(g, img, 0xb2));
            g.write::<f32>(p, x);
            let z = add(g.read::<f32>(p + 8), c.k(g, img, 0xc6));
            g.write::<f32>(p + 8, z);
        }
        0x0237_0000 => {
            let z = sub(g.read::<f32>(p + 8), c.k(g, img, 0xdc));
            g.write::<f32>(p + 8, z);
        }
        0x0238_0000 => {
            let x = sub(g.read::<f32>(p), c.k(g, img, 0xf9));
            g.write::<f32>(p, x);
        }
        _ => {}
    }
    crate::m0070::part_call(g, img, call(0x10d), 255, [a, w(0x108), p]);
    if g.read::<u8>(w(0x113)) & 1 == 0 {
        let m = gw(g, 0x12f);
        // call width: 2
        g.call(c.slot(g, img, 0x11b, 666), 0, &[m, 0]);
    }
    // the motion's cursor `[E + 4]`: the node, the mark byte
    let mark = g.read::<u8>(w(0x139));
    g.write::<u32>(grp, w(0x145));
    g.write::<u8>(grp + 5, mark);
    let sf = Frame {
        at: call(0x14e),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    let eax = crate::s4_summon::motion_state(g, img, &sf).eax;
    crate::sb_misc::node_append(g, img, call(0x15d), w(0x158), w(0x153), eax);
    let eax = crate::m0070::model_position(g, img, call(0x167), w(0x162));
    abs_stores(g, img, f, 0x175, 0x17f);
    crate::sa_misc::node_append(g, img, call(0x180), w(0x171), w(0x16c), eax);
    g.write::<u32>(grp, w(0x18c));
    g.write::<u32>(grp + 4, w(0x194));
    crate::m0042::time_left(g, img, call(0x199), grp, below);
    if c.s520(g, img, 0x19e) == 0x0237_0000 {
        let x = gw(g, 0x1b3).wrapping_sub(0x4000);
        g.write::<u32>(w(0x1b3), x);
    }
    // four pushes: the owned node's words (they stay to 0x2db)
    let sp = base.wrapping_sub(0x10);
    for (i, o) in (0u32..).zip([0x1cbu32, 0x1c6, 0x1c1, 0x1bc]) {
        g.write::<u32>(sp + 4 * i, w(o));
    }
    let of = Frame {
        at: call(0x1da),
        esp: sp.wrapping_sub(4),
        ecx: w(0x1d5),
        edx: w(0x1d0),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // the bones of the owned node (`ecx` as the append leaves it)
    let bf = Frame {
        at: call(0x1df),
        esp: sp.wrapping_sub(4),
        ecx: w(0x1d5),
        ..Frame::default()
    };
    crate::s4_small::bone_rate(g, img, &bf);
    let eax = crate::sb_misc::node_append(g, img, call(0x1ee), w(0x1e9), w(0x1e4), 0);
    abs_stores(g, img, f, 0x1f7, 0x21f);
    crate::sb_misc::node_append(g, img, call(0x220), w(0x1f3), w(0x1e4), eax);
    // the model state's record `[E + 4]` (`[esp + 0x20]` four words down)
    g.write::<u32>(grp, w(0x22c));
    g.write::<u32>(grp + 4, w(0x234));
    let eax = crate::m0070::model_state(g, img, call(0x239), grp);
    crate::sa_misc::node_append(g, img, call(0x248), w(0x243), w(0x23e), eax);
    // four more pushes (the third `ecx`, the node just appended)
    let sp2 = base.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([w(0x258), w(0x253), w(0x243), w(0x24d)]) {
        g.write::<u32>(sp2 + 4 * i, v);
    }
    abs_stores(g, img, f, 0x261, 0x27f);
    let of = Frame {
        at: call(0x280),
        esp: sp2.wrapping_sub(4),
        ecx: w(0x25d),
        edx: w(0x23e),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // the bones' cursor `[E + 4]`: the owned node, the mark byte
    let mark = g.read::<u8>(w(0x285));
    g.write::<u32>(grp, w(0x25d));
    g.write::<u8>(grp + 5, mark);
    let bf = Frame {
        at: call(0x296),
        esp: sp2.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    let m = gw(g, 0x2a6);
    // call width: 2
    g.call(c.slot(g, img, 0x29b, 211), 0, &[m, w(0x2a0)]);
    let fl = g.read::<u16>(w(0x2ba)) | 0x10;
    g.write::<u16>(w(0x2ba), fl);
    let m = gw(g, 0x2c1);
    abs_stores(g, img, f, 0x2c5, 0x2cf);
    // call width: 1
    g.call(c.slot(g, img, 0x2b3, 3), 0, &[m]);
    // slot 228 (the byte, 0xd8), the release of the handle
    let b = u32::from(g.read::<u8>(w(0x2d7)));
    // call width: 2
    g.call(c.slot(g, img, 0x2e5, 228), 0, &[b, 0xd8]);
    let h = gw(g, 0x2f3);
    crate::m0042::release(g, img, call(0x2f8), h);
    // the small node's word, pushed under slot 228's two
    g.write::<u32>(base.wrapping_sub(0xc), w(0x2fd));
    crate::m0070::node_init_small(g, img, call(0x30c), w(0x307), w(0x302), w(0x2fd));
    abs_stores(g, img, f, 0x317, 0x321);
    // the sound node `[N, parent, 700.0]` (the parent `edx` as the node
    // set-up leaves it)
    for (i, v) in (0u32..).zip([w(0x325), w(0x302), 0x442f_0000]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x335, 0x33f);
    crate::o6_fam_node::node_reset_sound(g, img, call(0x340), grp);
    abs_stores(g, img, f, 0x34e, 0x369);
    // the camera's first vectors: `E + 0x20` through slot 229, both copies
    let v = base + 0x2c;
    for (i, x) in (0u32..).zip([0, 0xc180_0000, 0xc080_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(c.slot(g, img, 0x38b, 229), 0, &[v, w(0x34a), v]);
    for (i, (a, b)) in (0u32..).zip([
        (0x3a6u32, 0x3aeu32),
        (0x3bc, 0x3c4),
        (0x3d2, 0x3da),
        (0x3e8, 0x3f0),
    ]) {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(w(a), x);
        g.write::<u32>(w(b), x);
    }
    abs_stores(g, img, f, 0x3f4, 0x430);
    for (i, x) in (0u32..).zip([0x4120_0000, 0, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    if c.s520(g, img, 0x39e) == 0x0237_0000 {
        return;
    }
    // slot 296 for each byte slot 288 lists (the count at `E + 4`)
    let n_at = grp;
    // call width: 2
    let list = g.call(c.slot(g, img, 0x460, 288), 0, &[0x10, n_at]).eax;
    let mut n = g.read::<i32>(n_at);
    let mut i = 0i32;
    let mut at = list;
    while i < n {
        let b = g.read::<u8>(at);
        if b != g.read::<u8>(w(0x484)) && u32::from(b) != gw(g, 0x48f) && b != 0xff {
            // call width: 2
            g.call(c.slot(g, img, 0x49e, 296), 0, &[u32::from(b), 0]);
            n = g.read::<i32>(n_at);
        }
        i += 1;
        at = at.wrapping_add(1);
    }
}

/// [`camera_task_0070`]'s state 0x97 (and 0x96's end): the camera's second
/// vector (slot 229) into its globals, the static link, flags and words,
/// the node ease of the DLL's object (its cursor `[E + 4]` with the mark
/// byte); the state on, the wait reset. Returns the node ease's `eax`.
fn second_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    c.vec229(g, img, (0x4cd, 0x4c7), [0, 0xbf80_0000, 0xc248_0000, ONE]);
    c.copy4(g, img, [0x503, 0x511, 0x51f, 0x530]);
    abs_stores(g, img, f, 0x534, 0x545);
    let lf = Frame {
        at: c.target(img, 0x546),
        esp: c.base.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s4_small::static_link_last_word(g, img, &lf);
    let mark = g.read::<u8>(c.w(img, 0x54b));
    abs_stores(g, img, f, 0x553, 0x56e);
    let grp = c.base + 0x10;
    g.write::<u32>(grp, c.w(img, 0x572));
    g.write::<u8>(grp + 5, mark);
    let ef = Frame {
        at: c.target(img, 0x57b),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    let eax = crate::s9_misc::node_ease(g, img, &ef).eax;
    c.up(g);
    c.reset(g);
    eax
}

/// [`camera_task_0070`]'s state 0x98: the fade byte `(70 - wait) / K *
/// K'` (truncated; its low byte kept), on when it is 0. Returns the
/// truncated value.
fn fade_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let n = 0x46i32.wrapping_sub(c.count(g));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let x = mul(div(n as f32, c.k(g, img, 0x5a1)), c.k(g, img, 0x5a9));
    let v = cvtt(x).cast_unsigned();
    g.write::<u8>(c.w(img, 0x5b2), v as u8);
    if v as u8 == 0 {
        c.up(g);
    }
    v
}

/// [`camera_task_0070`]'s state 0xa1 past its wait: the root node and its
/// start (`xmm1` the rate), the camera's vector (slot 229) into its
/// globals, two flags; state 0xc8. Returns slot 229's `eax`.
fn root_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let node = c.w(img, 0x5e3);
    crate::m0070::node_init_root(g, img, c.target(img, 0x5e8), node);
    let rate = u128::from(g.read::<u32>(c.w(img, 0x5f0)));
    abs_stores(g, img, f, 0x5f4, 0x644);
    let nf = Frame {
        at: c.target(img, 0x645),
        esp: c.base.wrapping_sub(4),
        ecx: node,
        xmm: [0, rate, 0, 0],
        ..Frame::default()
    };
    crate::sa_misc::node_start(g, img, &nf);
    let v = [0xc234_0000, 0xc1f0_0000, 0xc22c_0000, ONE];
    let eax = c.vec229(g, img, (0x655, 0x64f), v);
    c.copy4(g, img, [0x68b, 0x699, 0x6a7, 0x6b5]);
    abs_stores(g, img, f, 0x6b9, 0x6ca);
    g.write::<u32>(c.r + 0x18, 0xc8);
    eax
}

/// [`camera_task_0070`]'s state 0xc8: two appends, an owned node, the
/// small node under it, its word kept, the axes `(0, 0, 0xe000)` /
/// `(0, 0, 0.98)` and the move's ends `(160, 0, -) -> (0.98, 0, -)`; on,
/// `R + 0x1c` 3.0. Returns the move's ends' `eax`.
fn nodes_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let base = c.base;
    let list = w(0x6da);
    crate::sa_misc::node_append(g, img, call(0x6e4), w(0x6df), list, 0);
    abs_stores(g, img, f, 0x6ed, 0x70b);
    crate::sb_misc::node_append(g, img, call(0x70c), w(0x6e9), list, 0);
    // four pushes (the third `ecx`, the node just appended)
    let sp = base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x71c), w(0x6e9), w(0x716), w(0x711)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x725, 0x74d);
    let owned = w(0x721);
    let of = Frame {
        at: call(0x74e),
        esp: sp.wrapping_sub(4),
        ecx: owned,
        edx: list,
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // push ecx (the owned node, as the append leaves it): the small node's
    // word
    g.write::<u32>(sp.wrapping_sub(4), owned);
    abs_stores(g, img, f, 0x758, 0x75f);
    let small = w(0x754);
    crate::m0070::node_init_small(g, img, call(0x760), small, list, owned);
    g.write::<u32>(w(0x76e), small);
    abs_stores(g, img, f, 0x77d, 0x7a5);
    c.axes(g, img, (0x7ea, 0x7d9), (0xe000, 0x3f7a_e148, 0));
    let eax = c.ends(g, img, 0x819, (0x4320_0000, 0, 0x3f7a_e148, 0));
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0x4040_0000);
    eax
}

/// [`camera_task_0070`]'s state 0xaa: the camera's place (slot 229) and
/// the light set, the move globals, the camera's way `(-2000, 0, 0)`; on,
/// `R + 0x1c` -100.0. Returns the light set's `eax`.
fn light_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    c.vec229(g, img, (0x889, 0x883), [0, 0xc160_0000, 0, ONE]);
    let eax = c.light(g, img, 0x8c5, 0x8b9);
    abs_stores(g, img, f, 0x8c9, 0x94f);
    let p = c.p();
    for (i, v) in (0u32..).zip([0xc4fa_0000, 0, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0x967, 0x9a3);
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0xc2c8_0000);
    eax
}

/// [`camera_task_0070`]'s states 0xab/0xac: the step `+0x60` on, the
/// camera's way `(R + 0x1c, 0, 0)` into its globals, `R + 0x1c` up by
/// `K - K'` and the light's level down by `K''`.
fn climb_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let x = g.read::<u32>(w(0x9b2)).wrapping_add(0x60);
    g.write::<u32>(w(0x9b2), x);
    let v = c.rf(g, 0x1c);
    let p = c.p();
    g.write::<f32>(p, v);
    g.write::<f32>(w(0x9c6), v);
    g.write::<f32>(w(0x9ce), v);
    g.write::<u32>(p + 4, 0);
    g.write::<u32>(p + 8, 0);
    abs_stores(g, img, f, 0x9e2, 0xa0a);
    let y = sub(add(c.rf(g, 0x1c), c.k(g, img, 0xa13)), c.k(g, img, 0xa1b));
    c.wf(g, 0x1c, y);
    let z = sub(c.k(g, img, 0xa28), c.k(g, img, 0xa30));
    g.write::<f32>(w(0xa38), z);
}

/// [`camera_task_0070`]'s state 0x191 past its wait: the move globals
/// (the way `to - from`, the flags' bit 1 cleared and bit 0 set, the
/// rates). Returns the flags.
fn move_globals_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let a = sub(k(g, 0xa5c), k(g, 0xa64));
    let fl = (g.read::<u32>(w(0xa69)) & !2) | 1;
    let b = sub(0.0, k(g, 0xa74));
    g.write::<f32>(w(0xa7f), a);
    let x = sub(k(g, 0xa87), k(g, 0xa8f));
    abs_stores(g, img, f, 0xa96, 0xab4);
    g.write::<f32>(w(0xab8), x);
    let y = sub(k(g, 0xac0), k(g, 0xac8));
    g.write::<f32>(w(0xad0), b);
    abs_stores(g, img, f, 0xad4, 0xae8);
    g.write::<u32>(w(0xae9), fl);
    g.write::<f32>(w(0xaf1), y);
    let z = sub(k(g, 0xaf9), k(g, 0xb01));
    g.write::<f32>(w(0xb09), z);
    fl
}

/// A light's growth: `R + 0x1c` up by `K` (operand `ka`); while `K'`
/// (`kb`) is above the level `L` (`kl`), `L` (`ks`) becomes `R + 0x1c +
/// L`; then the step `R + 0x3c` up by 8.
fn step_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx, (ka, kb, kl, ks): (u32, u32, u32, u32)) {
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

/// [`camera_task_0070`]'s state 0x192 (and 0x191's end): the light's
/// growth, then the move's `x` down by the step while above -0x800 and
/// its `y` up by it while below 0x5800. Returns the last `y` read or
/// stored.
fn grow_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let w = |o: u32| c.w(img, o);
    step_0070(g, img, c, (0xb19, 0xb21, 0xb2e, 0xb3f));
    let x = g.read::<i32>(w(0xb48));
    if x > -0x800 {
        let s = g.read::<i32>(c.r + 0x3c);
        g.write::<i32>(w(0xb57), x.wrapping_sub(s));
    }
    let y = g.read::<i32>(w(0xb5c));
    if y >= 0x5800 {
        return y.cast_unsigned();
    }
    let y = y.wrapping_add(g.read::<i32>(c.r + 0x3c));
    g.write::<i32>(w(0xb6f), y);
    y.cast_unsigned()
}

/// [`camera_task_0070`]'s state 0x19a: the static node (one stack word,
/// over the state `push ecx` leaves), the move globals (the flags' bit 1
/// cleared and bit 0 set, `K - 0.0`); on, the wait reset. Returns the
/// flags.
fn static_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    let f = c.f;
    let base = c.base;
    g.write::<u32>(base.wrapping_sub(4), state);
    g.write::<u32>(base.wrapping_sub(8), c.w(img, 0xbaf));
    let sf = Frame {
        at: c.target(img, 0xbb9),
        esp: base.wrapping_sub(0xc),
        ecx: c.w(img, 0xbb4),
        ..Frame::default()
    };
    crate::s4_misc::static_node(g, img, &sf);
    let fl = (g.read::<u32>(c.w(img, 0xbbe)) & !2) | 1;
    let x = sub(c.k(g, img, 0xbc6), 0.0);
    abs_stores(g, img, f, 0xbd7, 0xc6d);
    g.write::<f32>(c.w(img, 0xc71), x);
    abs_stores(g, img, f, 0xc75, 0xc93);
    g.write::<u32>(c.w(img, 0xc94), fl);
    c.up(g);
    c.reset(g);
    fl
}

/// [`camera_task_0070`]'s state 0x19b past its wait: the move's ends
/// `(-200, -20, -) -> (0.98, 0.98, -)` (over the state `push ecx`
/// leaves), then the axes `(0, 0, 0x2000)` / `(0, 0, 0.025)` in mode 1.
/// Returns the axes' `eax`.
fn ends_axes_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
    g.write::<u32>(c.base.wrapping_sub(4), state);
    // junk read: [esp+0x24] (`from`'s z, E + 0x18) is never written before
    // the move's ends read it; 0 (brief 2026-10-24)
    g.write::<u32>(c.p() + 8, 0);
    c.ends(
        g,
        img,
        0xd61,
        (0xc348_0000, 0xc1a0_0000, 0x3f7a_e148, 0x3f7a_e148),
    );
    c.axes(g, img, (0xdbd, 0xdac), (0x2000, 0x3ccc_cccd, 1))
}

/// [`camera_task_0070`]'s state 0x2bc: the flags (bit 0 set, bits 1 and 3
/// cleared), the camera's place and aim (slots 229 and 248 twice, into
/// its globals and the two vectors), its way `(60, -10, 0)`; on. Returns
/// the two vectors' `eax`.
fn turn_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let p = c.p();
    let fl = (g.read::<u32>(w(0xddb)) | 1) & !0xa;
    g.write::<u32>(w(0xde6), fl);
    abs_stores(g, img, f, 0xdfa, 0xe0b);
    c.vec229(
        g,
        img,
        (0xdf6, 0xdf0),
        [0x41f0_0000, 0xc110_0000, 0xc1c8_0000, 0],
    );
    c.aim248(g, img, (0xe42, 0xe3c));
    c.copy4(g, img, [0xe58, 0xe6e, 0xe7c, 0xe95]);
    abs_stores(g, img, f, 0xe99, 0xebe);
    c.vec229(g, img, (0xe8d, 0xe87), [0, 0x4020_0000, 0, 0]);
    let eax = c.aim248(g, img, (0xef5, 0xeef));
    let tf = Frame {
        at: c.target(img, 0xf09),
        esp: c.base.wrapping_sub(4),
        edx: p,
        eax,
        ..Frame::default()
    };
    let eax = crate::s4_small::two_vectors(g, img, &tf).eax;
    for (i, v) in (0u32..).zip([0x4270_0000, 0xc120_0000, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0xf25, 0xf61);
    c.up(g);
    eax
}

/// [`camera_task_0070`]'s state 0x2be: the camera's place and aim (slots
/// 229 and 248) into its globals, the light words (`L - K`), the angle
/// word 0; on. Returns 0.
fn away_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    c.vec229(g, img, (0xf90, 0xf8a), [0x4120_0000, 0, 0xc28c_0000, 0]);
    c.aim248(g, img, (0xfcb, 0xfc5));
    c.copy4(g, img, [0xfe1, 0xfef, 0xffd, 0x100b]);
    let x = sub(c.k(g, img, 0x1013), c.k(g, img, 0x101b));
    abs_stores(g, img, f, 0x1024, 0x1053);
    g.write::<f32>(w(0x1057), x);
    g.write::<u16>(w(0x105d), 0);
    c.up(g);
    0
}

/// [`camera_task_0070`]'s state 0x384: the move set-up (`xmm2 = xmm3 =
/// K`, `eax` 0), the camera's place (slot 229) and the light set, the
/// move globals, the move's ends `(0, 0, -) -> (0, 0, -)`, the light
/// words; on, the wait reset. Returns 0.
fn slide_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    abs_stores(g, img, f, 0x109f, 0x10a9);
    c.setup(g, img, (0x10aa, 0x109b), (0x1093, 0x1093), 0);
    let v = [0xc000_0000, 0xc1a0_0000, 0xc28c_0000, ONE];
    c.vec229(g, img, (0x10ba, 0x10b4), v);
    c.light(g, img, 0x10f6, 0x10ea);
    abs_stores(g, img, f, 0x1104, 0x1183);
    c.ends(g, img, 0x11a4, (0, 0, 0, 0));
    let x = sub(c.k(g, img, 0x11ac), c.k(g, img, 0x11b4));
    abs_stores(g, img, f, 0x11ba, 0x11d8);
    g.write::<f32>(w(0x11dc), x);
    g.write::<u16>(w(0x11e2), 0);
    c.up(g);
    c.reset(g);
    0
}

/// [`camera_task_0070`]'s state 0x385 past its wait: the camera's place
/// (slot 229) and the light set, the light words, the move globals (the
/// angles -pi/8, the word 0xf800) and set-up; on, `R + 0x1c` 0, the wait
/// reset. Returns the set-up's `eax` (0xfffff800).
fn near_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let v = [0xc140_0000, 0xc1c8_0000, 0xc28c_0000, ONE];
    c.vec229(g, img, (0x1222, 0x121c), v);
    c.light(g, img, 0x125e, 0x1252);
    let x = sub(c.k(g, img, 0x1266), c.k(g, img, 0x126e));
    g.write::<u16>(w(0x1286), 0);
    abs_stores(g, img, f, 0x1294, 0x1316);
    g.write::<f32>(w(0x131a), x);
    abs_stores(g, img, f, 0x131e, 0x1332);
    g.write::<u16>(w(0x1334), 0xf800);
    abs_stores(g, img, f, 0x1338, 0x1374);
    let eax = c.setup(g, img, (0x1375, 0x1290), (0x127e, 0x1276), 0xffff_f800);
    c.up(g);
    g.write::<u32>(c.r + 0x1c, 0);
    c.reset(g);
    eax
}

/// [`camera_task_0070`]'s state 0x3e8: the flags (bit 0 set, bits 1 and 3
/// cleared), the camera's place (slot 229 of the DLL's vector) into both
/// copies, slot 325, the move globals, the camera's way `(160, 0, 0)`;
/// on, the wait reset, `R + 0x1c` 0.05. Returns slot 325's `eax`.
fn sights_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let p = c.p();
    let fl = (g.read::<u32>(w(0x1497)) | 1) & !0xa;
    g.write::<u32>(w(0x14a2), fl);
    abs_stores(g, img, f, 0x14ba, 0x14c4);
    // call width: 3
    g.call(c.slot(g, img, 0x14b6, 229), 0, &[p, w(0x14b0), w(0x14a7)]);
    for (i, (a, b)) in (0u32..).zip([(0x14dbu32, 0x14e3u32), (0x14f1, 0x14f9), (0x150a, 0x1512)]) {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(w(a), x);
        g.write::<u32>(w(b), x);
    }
    let x = g.read::<u32>(p + 0xc);
    abs_stores(g, img, f, 0x151c, 0x152d);
    g.write::<u32>(w(0x1531), x);
    g.write::<u32>(w(0x1539), x);
    // call width: 2
    let eax = g
        .call(c.slot(g, img, 0x14d3, 325), 0, &[w(0x1549), w(0x1544)])
        .eax;
    abs_stores(g, img, f, 0x154f, 0x15b0);
    for (i, v) in (0u32..).zip([0x4320_0000, 0, 0]) {
        g.write::<u32>(p + 4 * i, v);
    }
    abs_stores(g, img, f, 0x15c8, 0x1604);
    c.up(g);
    c.reset(g);
    g.write::<u32>(c.r + 0x1c, 0x3d4c_cccd);
    eax
}

/// [`camera_task_0070`]'s states 0x3e9..0x3ec (`xmm3` the float at
/// `+0x1640` in all): 0x3e9 on into 0x3ea; 0x3ea past its wait the move
/// set-up, on; 0x3eb `R + 0x1c` up by `xmm3`, the light's place moved, on
/// past its wait; 0x3ec slot 325, the camera's place (slot 229, `z` up by
/// `K`, `y` down by `R + 0x1c`) and the two vectors. Returns the two
/// vectors' `eax`.
fn sweep_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32) -> u32 {
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
        abs_stores(g, img, f, 0x1664, 0x1678);
        c.setup(g, img, (0x1679, 0x1660), (0x165b, 0x1640), eax);
        c.up(g);
    }
    if state <= 0x3eb {
        let x = add(c.rf(g, 0x1c), k(g, 0x1640));
        c.wf(g, 0x1c, x);
        for (a, kk, b, plus) in [
            (0x1692u32, 0x169au32, 0x16a2u32, true),
            (0x16aa, 0x16b2, 0x16ba, true),
            (0x16c2, 0x16ca, 0x16d2, false),
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
    let p = c.p();
    // call width: 2
    g.call(c.slot(g, img, 0x16ea, 325), 0, &[p, w(0x16e4)]);
    let z = add(g.read::<f32>(p + 8), k(g, 0x1700));
    g.write::<f32>(p + 8, z);
    let y = sub(g.read::<f32>(p + 4), c.rf(g, 0x1c));
    g.write::<f32>(p + 4, y);
    // call width: 3
    let eax = g
        .call(c.slot(g, img, 0x1710, 229), 0, &[p, w(0x170a), p])
        .eax;
    let tf = Frame {
        at: c.target(img, 0x173b),
        esp: c.base.wrapping_sub(4),
        edx: p,
        eax,
        ..Frame::default()
    };
    crate::s4_small::two_vectors(g, img, &tf).eax
}

/// [`camera_task_0070`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const CAMERA_TASK_0070: Sig = Sig {
    len: 6308,
    hash: 0xde53_99d6_6317_6348,
    calls: &[
        (0x6b, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x10d, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x14e, Callee::Sig(&crate::s4_summon::MOTION_STATE)),
        (0x15d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x167, Callee::Sig(&crate::m0070::MODEL_POSITION)),
        (0x180, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x199, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x1da, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1df, Callee::Sig(&crate::s4_small::BONE_RATE)),
        (0x1ee, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x220, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x239, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x248, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x280, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x296, Callee::Sig(&crate::sa_summon::BONE)),
        (0x2f8, Callee::Sig(&crate::m0042::RELEASE)),
        (0x30c, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x340, Callee::Any),
        (0x546, Callee::Sig(&crate::s4_small::STATIC_LINK_LAST_WORD)),
        (0x57b, Callee::Sig(&crate::s9_misc::NODE_EASE_0070)),
        (0x5e8, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x645, Callee::Sig(&crate::sa_misc::NODE_START)),
        (0x6e4, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x70c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x74e, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x760, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x7ea, Callee::Sig(&crate::s5_misc::SIGNED_AXES)),
        (0x819, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0x8c5, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0xbb9, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0xd61, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0xdbd, Callee::Sig(&crate::s5_misc::SIGNED_AXES)),
        (0xf09, Callee::Sig(&crate::s4_small::TWO_VECTORS)),
        (0x10aa, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x10f6, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x11a4, Callee::Sig(&crate::s6_misc::MOVE_ENDS_OF)),
        (0x125e, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x1375, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x1408, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x1466, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0x148a, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x1679, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x173b, Callee::Sig(&crate::s4_small::TWO_VECTORS)),
        (0x1747, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x174c, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x1751, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x177f, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0152`'s main task (0x10009a90, 7490 bytes with its tables; a
/// task callback `(A)`, cdecl): `magic_0533`'s
/// ([`main_task_0533`](crate::r37b::main_task_0533)) on 0152's operands,
/// kind 152 ([`main_task_0533_with`](crate::r37b::main_task_0533_with):
/// the bigger frame, the common exit through slot 531, the inline
/// character shown and the blocks of states 0x14, 0x20, 0x28, 0x33).
pub fn main_task_0152<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r37b::main_task_0533_with(g, img, fr, &MAIN_0152)
}

/// [`main_task_0152`]'s map of 0533's code offsets (r46-work's mapgen
/// pairs less five textual matches across the frame shift).
const MAIN_0152: crate::r37b::Main0533 = crate::r37b::Main0533 {
    map: &[
        (0x0, 0x0),
        (0x14a, 0x16f),
        (0x1a7, 0x1f3),
        (0x1d7, 0x221),
        (0x1dc, 0x22c),
        (0x244, 0x290),
        (0x24a, 0x29c),
        (0x252, 0x2b0),
        (0x271, 0x2d3),
        (0x41a, 0x47c),
        (0x82a, 0x8a3),
        (0x89e, 0x90d),
        (0x8d8, 0x948),
        (0xb02, 0xb70),
        (0xb90, 0xc00),
        (0xc48, 0xcb6),
        (0xc5c, 0xcc8),
        (0xce3, 0xd4d),
        (0xd05, 0xd63),
        (0xd19, 0xd74),
        (0xd20, 0xd7e),
        (0xde2, 0xe4f),
        (0xdeb, 0xe81),
        (0xdf0, 0xe8d),
        (0xe09, 0xe93),
        (0xec7, 0xf5d),
        (0xedb, 0xf65),
        (0xee0, 0xf70),
        (0x1156, 0x11e4),
        (0x117a, 0x11fe),
        (0x11a0, 0x122e),
        (0x16d0, 0x1752),
        (0x1710, 0x1790),
        (0x174c, 0x17c8),
        (0x19f0, 0x1a63),
        (0x1b11, 0x1b82),
        (0x1b48, 0x1bb7),
        (0x1b80, 0x1bf0),
        (0x1bb1, 0x1c24),
        (0x1bb7, 0x1c27),
        (0x1be4, 0x1c52),
    ],
    kind: 152,
};

/// [`main_task_0152`] (the `rel32`s: its callees in order).
pub const MAIN_TASK_0152: Sig = Sig {
    len: 7490,
    hash: 0x2e15_a3c6_9449_e7dc,
    calls: &[
        (0x11b, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x12c, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x189, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x219, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x246, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x287, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x29d, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x2d9, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x3fb, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x41c, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x442, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x48b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x513, Callee::Sig(&crate::s4_small::LINK_NODE_XY)),
        (0x527, Callee::Sig(&crate::s4_small::LINK_NODE_AD)),
        (0x534, Callee::Sig(&crate::s4_small::LINK_NODE_X)),
        (0x604, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x662, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6c5, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x71b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x77a, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x81c, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x85d, Callee::Sig(&crate::s4_small::LINK_NODE_GREY)),
        (0x949, Callee::Sig(&crate::m0042::UNLINK)),
        (0xbe2, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xbf5, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xc25, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xc80, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xce3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xd25, Callee::Sig(&crate::s4_small::CALL_37)),
        (0xd33, Callee::Sig(&crate::m0042::RELEASE)),
        (0xda5, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xdbe, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xe0c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe94, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xed6, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf0f, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xfbc, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xfe6, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x108e, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x10fd, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1116, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1155, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1212, Callee::Sig(&crate::s4_small::CALL_37)),
        (0x1220, Callee::Sig(&crate::m0042::RELEASE)),
        (0x1262, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1275, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x129f, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x12e5, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1394, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x13e9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1409, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x143e, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x14a2, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x14ce, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1627, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x16ab, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x16c3, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1793, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x17a0, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1826, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1871, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x18af, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1a3b, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1aaa, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1bdd, Callee::Sig(&crate::s4_small::CALL_37)),
        (0x1beb, Callee::Sig(&crate::m0042::RELEASE)),
        (0x1c18, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
    ],
    imports: &[],
};

/// [`summon_main_0155`]'s layout in a copy: the piecewise map of 0155's
/// code offsets onto the copy's (`(start, here)`: from `start` on an
/// offset `o` is `o - start + here`) and the copy's kind (155, or 164 for
/// `magic_0164`'s copy: [`summon_main_0164`]).
pub struct Main0155 {
    pub map: &'static [(u32, u32)],
    pub kind: u16,
}

impl Main0155 {
    /// `magic_0155`'s own.
    pub const OWN: Main0155 = Main0155 {
        map: &[],
        kind: 155,
    };

    /// `magic_0155`'s offset `o` in this copy.
    #[must_use]
    pub fn m(&self, o: u32) -> u32 {
        let mut r = o;
        for &(start, here) in self.map {
            if o >= start {
                r = o - start + here;
            }
        }
        r
    }
}

/// How a state of [`summon_main_0155_with`] ends: the counts `R + 0x28`
/// and `R + 0x2c` go up (after the state on and/or the wait reset), or
/// the state was set and the dispatch runs again.
enum End155 {
    /// Only the counts.
    Count(u32),
    /// The state on, then the counts.
    Up(u32),
    /// The state on, the wait reset, then the counts (the wait ends at 1).
    UpReset(u32),
    /// Dispatch again.
    Again,
}

/// `magic_0155`'s summon main task (0x1000a990, 5590 bytes with its
/// tables; a task callback `(A)`, cdecl; an aligned frame `E` of 0x4c
/// bytes below three pushes, no /GS; the record `R = A[4]`, its state
/// `R + 0x18`, counts `R + 0x28`, `R + 0x2c`, the wait). The same states as
/// `magic_0613`'s ([`summon_main_0613`](crate::r35::summon_main_0613)),
/// each ending on its own; by the state:
/// - 0: the summon's model node (slot 294's answer as its flags, 1 when
///   it answers 0xff), a colour node, the camera's move, the static link,
///   the scene's nodes appended under the DLL's parent `P` (the store
///   runs in the code; the old last read once, then each known), slots
///   676 twice, two colour nodes, a pair under the second parent, the
///   static list's node, an effect node; on to 0xa;
/// - 0xa, 0x33: slots 719 and 306; 0xb, 0xc, 0x16, 0x28: nodes appended,
///   an owned node, a particle node (0xc: slot 237); 0xd: two nodes under
///   `P` (slot 237 between) and a particle node, on to 0x14;
/// - 0x14/0x15, 0x17, 0x1e/0x1f, 0x29..0x2b, 0x46: words after their
///   waits; 0x18, 0x21, 0x2c, 0x40, 0x49: on to the next group;
/// - 0x20: the static tail link, slot 234, nodes, slot 325, an owned
///   node, particle nodes, the part freed, an effect node; 0x34: a fade
///   word down to `K`, then the part freed, a particle and an effect
///   node; 0x35: a node freed, the part (slot 713 unless its flag), a node
///   leaves, on to 0x3c; 0x3c/0x3d: slot 325, the part freed, a particle
///   and an effect node; 0x3e/0x3f: the static node and the move's words,
///   then the fade; 0x47: nodes leave and are freed, two registered;
///   0x48: the fade;
/// - 10000: the move's words and the summon's move, the summon's node
///   leaves, its model set-up, the task unlinked.
///
/// Returns `eax` as the original leaves it.
pub fn summon_main_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_main_0155_with(g, img, fr, &Main0155::OWN)
}

/// [`summon_main_0155`] on a copy's layout ([`Main0155`]). Kind 164
/// (`magic_0164`'s, 0x1000a770, 5494 bytes): slot 676 asks for 0x80 and
/// 0x60, 0x28 and 0x29 set other words, 0x46 runs slot 213 instead of its
/// words, 0x48 also turns the model (slot 239) and starts its move, the
/// end has no move, 0x49 waits 0x32.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_main_0155_with<G: Game>(g: &mut G, img: &Image, fr: &Frame, lay: &Main0155) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let o = |x: u32| lay.m(x);
    let w = |x: u32| img.word(f + o(x));
    let k = |g: &G, x: u32| g.read::<f32>(w(x));
    let gw = |g: &G, x: u32| g.read::<u32>(w(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    // slot `n` through the imports pointer operand `x`
    let slot = |g: &G, x: u32, n: u32| slot_target(g, w(x), n);
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    // push ebp; and esp, -8; sub esp, 0x4c; three pushes
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x58);
    let l = |x: u32| base.wrapping_add(x);
    let below = base.wrapping_sub(4);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let at = |x: u32| r.wrapping_add(x);
    let count = |g: &G| g.read::<i32>(at(0x2c));
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let reset = |g: &mut G| g.write::<u32>(at(0x2c), 0);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let leave = |g: &mut G, node: u32, eax: u32| crate::sa_misc::leave(g, node, eax);
    // a sb append: the node (operand `n`) under the list (operand `p`)
    let append = |g: &mut G, (c, n, p): (u32, u32, u32), eax: u32| {
        crate::sb_misc::node_append(g, img, call(c), w(n), w(p), eax)
    };
    // an owned node (operand `n`, the list operand `p`) with its four
    // pushed words (the last pushed first) at `base - d`
    let owned = |g: &mut G, (c, n, p): (u32, u32, u32), words: [u32; 4], d: u32| {
        let sp = base.wrapping_sub(d);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let of = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(n),
            edx: w(p),
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of)
    };
    // `push n; sub esp, 8; push a`: a particle node (operand `p`)
    let particle = |g: &mut G, (c, p): (u32, u32), a: u32, n: u32| {
        let sp = base.wrapping_sub(0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 0xc, n);
        let pf = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(p),
            ..Frame::default()
        };
        crate::s3_nodes::particle_node_with(g, img, &pf)
    };
    // `push d; push ecx; push b; push a`: an effect node (operand `e`;
    // the third word whatever `ecx` held: never read)
    let effect = |g: &mut G, (c, e): (u32, u32), (a, b, d): (u32, u32, u32)| {
        let sp = base.wrapping_sub(0x10);
        g.write::<u32>(sp, w(a));
        g.write::<u32>(sp + 4, w(b));
        g.write::<u32>(sp + 0xc, d);
        let ef = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(e),
            ..Frame::default()
        };
        crate::s3_nodes::effect_node_with(g, img, &ef)
    };
    // the node at operand `x` freed and unlinked through `base + 0x10`
    let free_unlink = |g: &mut G, (c, x): (u32, u32), eax: u32| {
        g.write::<u32>(l(0x10), w(x));
        let ff = Frame {
            at: call(c),
            esp: below,
            ecx: l(0x10),
            eax,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff).eax
    };
    let free_part = |g: &mut G, (c, p): (u32, u32), eax: u32| {
        crate::m0070::free_part(g, img, call(c), w(p), eax)
    };
    // a flag word's bit 0 set (operand `x`)
    let flag1 = |g: &mut G, x: u32| {
        let v = gw(g, x) | 1;
        g.write::<u32>(w(x), v);
    };
    // the fade (the call at `c`) with the level at operand `kx`
    let fade = |g: &mut G, c: u32, kx: u32| {
        let level = u128::from(gw(g, kx));
        let ff = Frame {
            at: call(c),
            esp: below,
            xmm: [0, level, 0, 0],
            ..Frame::default()
        };
        crate::s4_misc::fade_set(g, img, &ff).eax
    };
    let ebx = w(0x21);
    let mut state = g.read::<u32>(at(0x18));
    let end = loop {
        if state > 0x2710 {
            break End155::Count(state);
        }
        if state == 0x2710 {
            break End155::Count(end_0155(g, img, f, lay, base, task));
        }
        if state > 0x49 {
            break End155::Count(state);
        }
        let idx = img.word(w(0x37) + state) & 0xff;
        let next = match state {
            0 => {
                start_0155(g, img, f, lay, base);
                abs(g, 0x6ed, 0x701);
                reset(g);
                set(g, 0xa);
                End155::Again
            }
            0xa | 0x33 => {
                if state == 0xa && count(g) < 0xf {
                    End155::Count(idx)
                } else {
                    let (i, j, a, b) = if state == 0xa {
                        (0x9e7u32, 0xa01u32, 0x9f6u32, 0)
                    } else {
                        (0xf31, 0xf4b, 0xf40, 1)
                    };
                    // call width: 3
                    let h = g.call(slot(g, i, 719), 0, &[w(a), 5, 0]).eax;
                    // call width: 3
                    let eax = g.call(slot(g, j, 306), 0, &[7, b, h]).eax;
                    if state == 0xa {
                        End155::Up(eax)
                    } else {
                        abs(g, 0xf63, 0xf95);
                        let eax = free_part(g, (0xf96, 0xf5f), eax);
                        let _ = eax;
                        particle(g, (0xfaa, 0xfa0), w(0xfa5), 4);
                        let eax = effect(g, (0xfc4, 0xfbf), (0xfba, 0xfb5, 2));
                        flag1(g, 0xfca);
                        abs(g, 0xfcf, 0xfd9);
                        End155::UpReset(eax)
                    }
                }
            }
            0xb => {
                if count(g) < 0x19 {
                    End155::Count(idx)
                } else {
                    append(g, (0xa39, 0xa34, 0xa2f), idx);
                    abs(g, 0xa52, 0xa7a);
                    let words = [w(0xa49), w(0xa44), w(0xa3f), w(0xa34)];
                    owned(g, (0xa7b, 0xa4e, 0xa2f), words, 0x10);
                    let eax = particle(g, (0xa8e, 0xa89), w(0xa4e), 5);
                    End155::Up(eax)
                }
            }
            0xc => {
                if count(g) < 0x35 {
                    End155::Count(idx)
                } else {
                    append(g, (0xaba, 0xab5, 0xab0), idx);
                    abs(g, 0xac3, 0xaeb);
                    append(g, (0xaec, 0xabf, 0xab0), idx);
                    // slot 237 `(model, v, 0, x)` on the third node
                    g.write::<u32>(base.wrapping_sub(4), w(0xaf7));
                    let cf = Frame {
                        at: call(0xb01),
                        esp: base.wrapping_sub(8),
                        ecx: w(0xafc),
                        edx: gw(g, 0xaf2),
                        ..Frame::default()
                    };
                    crate::s4_small::call_237(g, img, &cf);
                    abs(g, 0xb26, 0xb30);
                    let words = [w(0xb1d), w(0xb18), w(0xb13), w(0xb0e)];
                    owned(g, (0xb31, 0xb22, 0xb09), words, 0x10);
                    let eax = particle(g, (0xb44, 0xb3f), w(0xb22), 8);
                    End155::Up(eax)
                }
            }
            0xd => {
                if count(g) < 0x36 {
                    End155::Count(idx)
                } else {
                    nodes_0155(g, img, f, lay, base);
                    set(g, 0x14);
                    End155::Again
                }
            }
            0x14 | 0x15 => {
                if state == 0x14 {
                    up(g);
                    reset(g);
                }
                if count(g) < 0xf {
                    End155::Count(idx)
                } else {
                    abs(g, 0xb6f, 0xb79);
                    End155::Up(idx)
                }
            }
            0x16 => {
                if count(g) < 0x14 {
                    End155::Count(idx)
                } else {
                    append(g, (0xb9e, 0xb99, 0xb94), idx);
                    abs(g, 0xbb7, 0xbdf);
                    let words = [w(0xbae), w(0xba9), w(0xba4), w(0xb99)];
                    owned(g, (0xbe0, 0xbb3, 0xb94), words, 0x10);
                    let eax = particle(g, (0xbf3, 0xbee), w(0xbb3), 7);
                    End155::UpReset(eax)
                }
            }
            0x17 => {
                if count(g) < 0x1e {
                    End155::Count(idx)
                } else {
                    End155::Up(idx)
                }
            }
            0x1e | 0x1f => {
                if state == 0x1e {
                    up(g);
                    reset(g);
                }
                if count(g) < 5 {
                    End155::Count(idx)
                } else {
                    abs(g, 0xc3f, 0xc49);
                    End155::UpReset(idx)
                }
            }
            0x20 => {
                if count(g) < 9 {
                    End155::Count(idx)
                } else {
                    End155::UpReset(tail_0155(g, img, f, lay, base))
                }
            }
            0x28 => {
                append(g, (0xe33, 0xe2e, 0xe29), idx);
                abs(g, 0xe4c, 0xe74);
                let words = [w(0xe43), w(0xe3e), w(0xe39), w(0xe2e)];
                owned(g, (0xe75, 0xe48, 0xe29), words, 0x10);
                let eax = if lay.kind == 164 {
                    // 164 jumps into 0x16's particle call (`+0xbf5`) and its
                    // end; no word
                    let sp = base.wrapping_sub(0x10);
                    g.write::<u32>(sp, w(0xe48));
                    g.write::<u32>(sp + 0xc, 3);
                    let pf = Frame {
                        at: img.rel32_target(f + 0xbf6),
                        esp: sp.wrapping_sub(4),
                        ecx: w(0xe83),
                        ..Frame::default()
                    };
                    crate::s3_nodes::particle_node_with(g, img, &pf)
                } else {
                    let eax = particle(g, (0xe88, 0xe83), w(0xe48), 3);
                    abs(g, 0xe8c, 0xe96);
                    eax
                };
                End155::UpReset(eax)
            }
            0x29..=0x2b => {
                let (wait, run) = match state {
                    0x29 => (0x1e, (0xeba, 0xece)),
                    0x2a => (0x32, (0xee8, 0xef2)),
                    _ => (0x46, (0xf0c, 0xf16)),
                };
                if count(g) < wait {
                    End155::Count(idx)
                } else {
                    abs(g, run.0, run.1);
                    if state == 0x2b {
                        End155::UpReset(idx)
                    } else {
                        End155::Up(idx)
                    }
                }
            }
            0x32 => {
                up(g);
                // (on into 0x33 in the same call)
                state = 0x33;
                continue;
            }
            0x34 => {
                let x = k(g, 0xff7);
                // comiss; jbe: unordered keeps the word
                if x > k(g, 0xffe) {
                    let y = sub(x, k(g, 0x1008));
                    g.write::<f32>(w(0x1010), y);
                }
                if count(g) < 0x43 {
                    End155::Count(idx)
                } else {
                    abs(g, 0x1023, 0x1073);
                    let eax = free_part(g, (0x1074, 0x101f), idx);
                    let _ = eax;
                    particle(g, (0x1088, 0x107e), w(0x1083), 6);
                    let eax = effect(g, (0x10a2, 0x109d), (0x1098, 0x1093, 5));
                    flag1(g, 0x10a8);
                    abs(g, 0x10ad, 0x10b7);
                    End155::Up(eax)
                }
            }
            0x35 => {
                if g.read::<u8>(w(0x8ee)) & 1 == 0 {
                    End155::Count(idx)
                } else {
                    let mut eax = free_unlink(g, (0x906, 0x901), idx);
                    if g.read::<u8>(w(0x90c)) & 4 == 0 {
                        let v = gw(g, 0x91a);
                        g.write::<u32>(gw(g, 0x914), v);
                        // call width: 1
                        eax = g.call(slot(g, 0x921, 713), 0, &[w(0x926)]).eax;
                    }
                    // the node leaves its parent (inline; the node at the
                    // immediate `+0x97c`)
                    leave(g, w(0x97c), eax);
                    set(g, 0x3c);
                    End155::Again
                }
            }
            0x3c | 0x3d => {
                if state == 0x3c {
                    up(g);
                }
                abs(g, 0x10d2, 0x10fa);
                // call width: 2
                let eax = g.call(slot(g, 0x10ce, 325), 0, &[w(0x1106), w(0x1101)]).eax;
                abs(g, 0x1114, 0x111e);
                free_part(g, (0x111f, 0x1110), eax);
                particle(g, (0x1133, 0x1129), w(0x112e), 0);
                let eax = effect(g, (0x114d, 0x1148), (0x1143, 0x113e, 3));
                flag1(g, 0x1153);
                abs(g, 0x1158, 0x1162);
                End155::UpReset(eax)
            }
            0x3e | 0x3f => {
                let mut eax = idx;
                if state == 0x3e && count(g) >= 8 {
                    // push ecx (unread); push the value: the static node
                    g.write::<u32>(base.wrapping_sub(8), w(0x1188));
                    let sf = Frame {
                        at: call(0x1192),
                        esp: base.wrapping_sub(0xc),
                        ecx: w(0x118d),
                        ..Frame::default()
                    };
                    crate::s4_misc::static_node(g, img, &sf);
                    let fl = (gw(g, 0x1197) & !2) | 1;
                    // `K - xmm2`, `xmm2` 0 from the dispatch
                    let x = sub(k(g, 0x119f), 0.0);
                    abs(g, 0x11ad, 0x1243);
                    g.write::<f32>(w(0x1247), x);
                    abs(g, 0x124b, 0x1269);
                    g.write::<u32>(w(0x126a), fl);
                    up(g);
                    reset(g);
                    eax = fl;
                }
                if count(g) < 0x1e {
                    End155::Count(eax)
                } else {
                    abs(g, 0x128d, 0x12dd);
                    End155::Up(fade(g, 0x12de, 0x1289))
                }
            }
            0x40 => {
                if count(g) < 0x3a {
                    End155::Count(idx)
                } else {
                    set(g, 0x46);
                    End155::Again
                }
            }
            0x46 => {
                if lay.kind == 164 {
                    // (164's own offsets)
                    let raw = |x: u32| img.word(f + x);
                    let m = g.read::<u32>(raw(0x129a));
                    // call width: 2
                    let eax = g.call(slot_target(g, raw(0x1292), 213), 0, &[m, 0]).eax;
                    abs_stores(g, img, f, 0x12a9, 0x12b3);
                    End155::UpReset(eax)
                } else {
                    abs(g, 0x12f2, 0x1306);
                    End155::UpReset(idx)
                }
            }
            0x47 => {
                if count(g) < 1 {
                    End155::Count(idx)
                } else {
                    let mut eax = leave(g, w(0x1328), idx);
                    for (c, x, pc, p) in [
                        (0x133eu32, 0x1339u32, 0x1348u32, 0x1343u32),
                        (0x1359, 0x1354, 0x1363, 0x135e),
                    ] {
                        eax = free_unlink(g, (c, x), eax);
                        eax = free_part(g, (pc, p), eax);
                    }
                    eax = leave(g, w(0x1368), eax);
                    eax = leave(g, w(0x1372), eax);
                    for (n, c) in [(0x137cu32, 0x1386u32), (0x138b, 0x1395)] {
                        // `ecx` the node as the leave keeps it
                        leave(g, w(n), eax);
                        eax = crate::sa_misc::register_one(g, img, call(c), w(n));
                    }
                    eax = leave(g, ebx, eax);
                    eax = leave(g, w(0x13a1), eax);
                    End155::Up(eax)
                }
            }
            0x48 => {
                if count(g) < 2 {
                    End155::Count(idx)
                } else {
                    abs(g, 0x13cc, 0x141c);
                    let mut eax = fade(g, 0x141d, 0x13c8);
                    if lay.kind == 164 {
                        eax = turn_move_0164(g, img, f, base);
                    }
                    End155::Up(eax)
                }
            }
            0x18 | 0x21 | 0x2c | 0x49 => {
                let (wait, to) = match state {
                    0x18 => (0x49, 0x1e),
                    0x21 => (0xf, 0x28),
                    0x2c => (0x78, 0x32),
                    _ => (if lay.kind == 164 { 0x32 } else { 0x5a }, 0x2710),
                };
                if count(g) < wait {
                    End155::Count(idx)
                } else {
                    set(g, to);
                    End155::Again
                }
            }
            _ => End155::Count(idx),
        };
        match next {
            End155::Again => state = g.read::<u32>(at(0x18)),
            e => break e,
        }
    };
    let eax = match end {
        End155::Count(eax) => eax,
        End155::Up(eax) => {
            up(g);
            eax
        }
        End155::UpReset(eax) => {
            up(g);
            reset(g);
            eax
        }
        End155::Again => 0,
    };
    for x in [0x28u32, 0x2c] {
        let v = g.read::<u32>(at(x)).wrapping_add(1);
        g.write::<u32>(at(x), v);
    }
    Ret::eax(eax)
}

/// [`summon_main_0155_with`]'s state 0 (`base` the frame below the three
/// pushes): the summon's model node, a colour node, the camera's move,
/// the static link, the scene's nodes under `P`, slots 676, the colour
/// nodes, a pair under the second parent, the static list's node, an
/// effect node and its flag.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0155<G: Game>(g: &mut G, img: &Image, f: u32, lay: &Main0155, base: u32) {
    use crate::r25::abs_stores;
    let o = |x: u32| lay.m(x);
    let w = |x: u32| img.word(f + o(x));
    let gw = |g: &G, x: u32| g.read::<u32>(w(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let slot = |g: &G, x: u32, n: u32| slot_target(g, w(x), n);
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    // a node after the parent's last (read before, `old`): its `+8` (the
    // store at `n8`), the parent's first (the store at `eq`) when `old`
    // is the parent (operand `cmp`), else `old + 4` (the immediate `ne`)
    let after = |g: &mut G, old: u32, (n8, cmp, eq, ne): (u32, u32, u32, u32)| {
        g.write::<u32>(w(n8), old);
        if old == w(cmp) {
            abs(g, eq, eq + 10);
        } else {
            g.write::<u32>(old.wrapping_add(4), w(ne));
        }
    };
    // a node after a known one (`prev`): the parent's first (the store at
    // `eq`) when `prev` is the parent (operand `cmp`), else the store at
    // `ne` (`prev + 4`)
    let known = |g: &mut G, prev: u32, (cmp, eq, ne): (u32, u32, u32)| {
        let at = if prev == w(cmp) { eq } else { ne };
        abs(g, at, at + 10);
    };
    // the summon's model node: slot 294's answer as its flags
    let m = gw(g, 0x4c);
    // call width: 1
    let a = g.call(slot(g, 0x46, 294), 0, &[m]).eax;
    let grp = if a == 0xff {
        let grp = base + 0x1c;
        for (i, v) in (0u32..).zip([w(0x66), w(0x6e), 1]) {
            g.write::<u32>(grp + 4 * i, v);
        }
        g.write::<u8>(grp + 0xc, 0);
        grp
    } else {
        let grp = base + 0x2c;
        g.write::<u32>(grp, w(0x9a));
        g.write::<u32>(grp + 4, w(0xa2));
        let m = gw(g, 0x8c);
        // call width: 1
        let b = g.call(slot(g, 0x86, 294), 0, &[m]).eax;
        g.write::<u32>(grp + 8, b);
        g.write::<u8>(grp + 0xc, 0);
        grp
    };
    abs(g, 0xb8, 0xc2);
    crate::sa_summon::model_node_init(g, img, call(0xc3), grp);
    // a colour node from `[N, P]`
    let cg = base + 0x14;
    g.write::<u32>(cg, w(0xcf));
    g.write::<u32>(cg + 4, w(0xd7));
    abs(g, 0xdb, 0xe5);
    let cf = Frame {
        at: call(0xe6),
        esp: base.wrapping_sub(4),
        ecx: cg,
        ..Frame::default()
    };
    let eax = crate::s3_misc::colour_node(g, img, &cf);
    // the camera's move (`xmm2` its time)
    let x = g.read::<f32>(w(0xee));
    abs(g, 0xf2, 0x138);
    crate::sb_misc::move_start(g, img, call(0x139), x, eax);
    // push ecx (unread): the static link
    let lf = Frame {
        at: call(0x13f),
        esp: base.wrapping_sub(8),
        ..Frame::default()
    };
    crate::s4_small::static_link_word(g, img, &lf);
    // five nodes under `P`: the old last read once, then each known
    let old = gw(g, 0x144);
    abs(g, 0x14b, 0x17c);
    after(g, old, (0x17d, 0x182, 0x188, 0x197));
    let x = gw(g, 0x19c);
    let n = g.read::<u16>(w(0x1a3));
    g.write::<u32>(w(0x1a8), x);
    abs(g, 0x1b3, 0x231);
    known(g, w(0x1ad), (0x232, 0x238, 0x244));
    abs(g, 0x255, 0x2cc);
    known(g, w(0x24f), (0x2cd, 0x2d3, 0x2df));
    abs(g, 0x2f0, 0x371);
    known(g, w(0x2ea), (0x372, 0x378, 0x384));
    abs(g, 0x390, 0x3e9);
    known(g, w(0x21), (0x3eb, 0x3f1, 0x3fd));
    abs(g, 0x410, 0x41a);
    g.write::<u16>(w(0x41d), n.wrapping_add(5));
    abs(g, 0x421, 0x435);
    let (a1, a2) = if lay.kind == 164 {
        (0x80, 0x60)
    } else {
        (0x60, 0x40)
    };
    // call width: 3
    g.call(slot(g, 0x408, 676), 0, &[w(0x43e), 1, a1]);
    // call width: 3
    g.call(slot(g, 0x445, 676), 0, &[w(0x457), 4, a2]);
    // two colour nodes from 7-word records
    let rec = base + 0x3c;
    for (ops, run, c) in [
        (
            [0x468u32, 0x470, 0x478, 0x480, 0x488, 0x490, 0x498],
            0x49cu32,
            0x4a7u32,
        ),
        (
            [0x4b3, 0x4bb, 0x4c3, 0x4cb, 0x4d3, 0x4db, 0x4e3],
            0x4e7,
            0x4f2,
        ),
    ] {
        for (i, x) in (0u32..).zip(ops) {
            g.write::<u32>(rec + 4 * i, w(x));
        }
        abs(g, run, run + 10);
        crate::sa_misc::colour_node_init(g, img, call(c), rec);
    }
    // a pair under the second parent (the second's old last the first)
    let old = gw(g, 0x4f7);
    abs(g, 0x4fb, 0x52c);
    after(g, old, (0x52d, 0x532, 0x538, 0x547));
    let n = g.read::<u16>(w(0x54d));
    abs(g, 0x558, 0x5bb);
    known(g, w(0x552), (0x5bd, 0x5c3, 0x5cf));
    g.write::<u16>(w(0x5dd), n.wrapping_add(2));
    // the static list's node (its old last's `+4` always it)
    let old = gw(g, 0x5e2);
    abs(g, 0x5e6, 0x636);
    g.write::<u32>(w(0x637), old);
    abs(g, 0x63b, 0x645);
    g.write::<u32>(old.wrapping_add(4), w(0x648));
    let v = gw(g, 0x64e) & !1;
    g.write::<u32>(w(0x64e), v);
    abs(g, 0x657, 0x6a7);
    // an effect node from `[a, b, c, d, e, 0]`, its vtable after
    for (i, x) in (0u32..).zip([0x6abu32, 0x6b3, 0x6bb, 0x6c3, 0x6cb]) {
        g.write::<u32>(rec + 4 * i, w(x));
    }
    g.write::<u32>(rec + 0x14, 0);
    crate::sb_nodes::effect_node(g, img, call(0x6d8), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x6e2));
    let v = gw(g, 0x6e8) | 1;
    g.write::<u32>(w(0x6e8), v);
}

/// [`summon_main_0155_with`]'s state 0xd past its wait: a node under `P`
/// (its count up), slot 237 on the summon's model, a second node, a
/// particle node from `[a, b, c, *G, e, 8]`.
fn nodes_0155<G: Game>(g: &mut G, img: &Image, f: u32, lay: &Main0155, base: u32) {
    use crate::r25::abs_stores;
    let o = |x: u32| lay.m(x);
    let w = |x: u32| img.word(f + o(x));
    let gw = |g: &G, x: u32| g.read::<u32>(w(x));
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    let count_up = |g: &mut G, x: u32| {
        let n = g.read::<u16>(w(x)).wrapping_add(1);
        g.write::<u16>(w(x), n);
    };
    for (i, (run, n8, cmp, eq, ne, cnt)) in [
        (
            (0x723u32, 0x754u32),
            0x755u32,
            0x75au32,
            0x760u32,
            0x76fu32,
            0x77bu32,
        ),
        ((0x7b6, 0x7fb), 0x7fc, 0x801, 0x807, 0x816, 0x822),
    ]
    .into_iter()
    .enumerate()
    {
        let old = gw(g, if i == 0 { 0x71f } else { 0x7af });
        abs(g, run.0, run.1);
        g.write::<u32>(w(n8), old);
        if old == w(cmp) {
            abs(g, eq, eq + 10);
        } else {
            g.write::<u32>(old.wrapping_add(4), w(ne));
        }
        if i == 1 {
            break;
        }
        count_up(g, cnt);
        // slot 237 `(*M, *X, 0, V)`, the last word set between
        let x = gw(g, 0x788);
        abs(g, 0x78c, 0x796);
        let m = gw(g, 0x798);
        abs(g, 0x79c, 0x7a6);
        // call width: 4
        g.call(slot_target(g, w(0x774), 237), 0, &[m, x, 0, w(0x780)]);
    }
    let x = gw(g, 0x81b);
    count_up(g, 0x822);
    abs(g, 0x82a, 0x863);
    let rec = base + 0x3c;
    for (i, v) in (0u32..).zip([w(0x867), w(0x86f), w(0x877), x, w(0x883), 8]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    abs(g, 0x88f, 0x899);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + o(0x89a)), rec);
}

/// [`summon_main_0155_with`]'s state 0x20 past its wait: the static tail
/// link, slot 234 on the fade's words, a node, slot 325, a node and its
/// owned node, a particle node, the part freed, a second particle node,
/// an effect node and its flag. Returns the effect node.
fn tail_0155<G: Game>(g: &mut G, img: &Image, f: u32, lay: &Main0155, base: u32) -> u32 {
    use crate::r25::abs_stores;
    let o = |x: u32| lay.m(x);
    let w = |x: u32| img.word(f + o(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    let tf = Frame {
        at: call(0xc6b),
        esp: base.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s4_small::static_link_tail(g, img, &tf);
    abs(g, 0xc79, 0xcc9);
    // call width: 3
    g.call(
        slot_target(g, w(0xc70), 234),
        0,
        &[w(0xcd5), w(0xcd0), w(0xc75)],
    );
    abs(g, 0xce8, 0xd06);
    crate::sb_misc::node_append(g, img, call(0xd07), w(0xce4), w(0xcdf), 0);
    // call width: 2
    g.call(slot_target(g, w(0xd0c), 325), 0, &[w(0xd1c), w(0xd11)]);
    crate::sb_misc::node_append(g, img, call(0xd2d), w(0xd28), w(0xd23), 0);
    abs(g, 0xd46, 0xd6e);
    // slot 325's two words stay under the owned node's four
    let sp = base.wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([w(0xd3d), w(0xd38), w(0xd33), w(0xd28)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let of = Frame {
        at: call(0xd6f),
        esp: sp.wrapping_sub(4),
        ecx: w(0xd42),
        edx: w(0xd23),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    let sp = base.wrapping_sub(0x10);
    let part = |g: &mut G, (c, p): (u32, u32), a: u32, n: u32| {
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 0xc, n);
        let pf = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(p),
            ..Frame::default()
        };
        crate::s3_nodes::particle_node_with(g, img, &pf)
    };
    let eax = part(g, (0xd82, 0xd7d), w(0xd42), 2);
    abs(g, 0xd8e, 0xdc0);
    crate::m0070::free_part(g, img, call(0xdc1), w(0xd8a), eax);
    part(g, (0xdd5, 0xdcb), w(0xdd0), 9);
    g.write::<u32>(sp, w(0xde5));
    g.write::<u32>(sp + 4, w(0xde0));
    g.write::<u32>(sp + 0xc, 1);
    let ef = Frame {
        at: call(0xdef),
        esp: sp.wrapping_sub(4),
        ecx: w(0xdea),
        ..Frame::default()
    };
    let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
    let v = g.read::<u32>(w(0xdf5)) | 1;
    g.write::<u32>(w(0xdf5), v);
    abs(g, 0xdfa, 0xe0e);
    eax
}

/// [`summon_main_0155_with`]'s end (10000): (155) the move's words and the
/// summon's move from `[N, 0, K, 1.0]`, (164) its words only; the summon's
/// node leaves, its model set-up through `base + 0x10`, the task unlinked.
/// Returns the unlink's `eax`.
fn end_0155<G: Game>(g: &mut G, img: &Image, f: u32, lay: &Main0155, base: u32, task: u32) -> u32 {
    use crate::r25::abs_stores;
    let raw = |x: u32| img.word(f + x);
    let (node, c) = if lay.kind == 164 {
        abs_stores(g, img, f, 0x144e, 0x146c);
        (raw(0x144a), 0x147a)
    } else {
        abs_stores(g, img, f, 0x1435, 0x14a3);
        let args = base + 0x2c;
        for (i, v) in (0u32..).zip([raw(0x14a7), 0, 0x3dcc_cccd, ONE]) {
            g.write::<u32>(args + 4 * i, v);
        }
        crate::sa_misc::move_start(g, args);
        (raw(0x14c9), 0x14db)
    };
    crate::sa_misc::leave(g, node, 0);
    // `ecx` the node as the leave keeps it
    let rec = base + 0x10;
    g.write::<u32>(rec, node);
    let mf = Frame {
        at: img.rel32_target(f + c),
        esp: base.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::sa_summon::model_setup(g, img, &mf);
    crate::h6_setup::unlink_task(g, task)
}

/// `magic_0164`'s state 0x48 after the fade (its own offsets): slot 239
/// `(*G, -5, 0)` (`push ecx` under it: unread), the model's words and its
/// move from `[N, 0, K, 1.0]`. Returns the move's time.
fn turn_move_0164<G: Game>(g: &mut G, img: &Image, f: u32, base: u32) -> u32 {
    let cf = Frame {
        at: img.rel32_target(f + 0x13c3),
        esp: base.wrapping_sub(8),
        edx: 0xffff_fffb,
        ..Frame::default()
    };
    crate::s4_small::call_239_edx(g, img, &cf);
    crate::r25::abs_stores(g, img, f, 0x13ce, 0x1414);
    let args = base + 0x2c;
    for (i, v) in (0u32..).zip([img.word(f + 0x1418), 0, 0x3ca3_d70a, ONE]) {
        g.write::<u32>(args + 4 * i, v);
    }
    crate::sa_misc::move_start(g, args)
}

/// `magic_0164`'s summon main task (0x1000a770, 5494 bytes with its
/// tables): [`summon_main_0155_with`] kind 164 on 0164's operands.
pub fn summon_main_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_main_0155_with(g, img, fr, &MAIN_0164)
}

/// [`summon_main_0164`]'s map of 0155's code offsets (r46-work's mapgen
/// pairs; the tables' tail left out).
const MAIN_0164: Main0155 = Main0155 {
    map: &[
        (0x0, 0x0),
        (0x40e, 0x411),
        (0xbf7, 0xbfd),
        (0xc00, 0xc03),
        (0xeb0, 0xe8f),
        (0xece, 0xea3),
        (0xf2d, 0xef0),
        (0xfc8, 0xf8e),
        (0xff3, 0xfa4),
        (0x1151, 0x1105),
        (0x117c, 0x111b),
        (0x12fc, 0x12a9),
        (0x131d, 0x12b8),
        (0x1421, 0x1439),
        (0x1435, 0x144e),
        (0x14cd, 0x146c),
    ],
    kind: 164,
};

/// [`summon_main_0155`] (the `rel32`s: its callees in order).
pub const SUMMON_MAIN_0155: Sig = Sig {
    len: 5590,
    hash: 0xe095_af73_3d3e_f25c,
    calls: &[
        (0xc3, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xe6, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x139, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x13f, Callee::Sig(&crate::s4_small::STATIC_LINK_WORD)),
        (0x4a7, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x4f2, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x6d8, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x89a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x906, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xa39, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xa7b, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xa8e, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xaba, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xaec, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xb01, Callee::Sig(&crate::s4_small::CALL_237)),
        (0xb31, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xb44, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xb9e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xbe0, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xbf3, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xc6b, Callee::Sig(&crate::s4_small::STATIC_LINK_TAIL)),
        (0xd07, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xd2d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xd6f, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xd82, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xdc1, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xdd5, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xdef, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xe33, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xe75, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xe88, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xf96, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xfaa, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xfc4, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1074, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1088, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x10a2, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x111f, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1133, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x114d, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1192, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x12de, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x132d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x133e, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1348, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1359, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1363, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x136d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1377, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1381, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1386, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x1390, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1395, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x139c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x13a6, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x141d, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x14c4, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x14ce, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x14db, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// [`summon_main_0164`] (the `rel32`s: its callees in order).
pub const SUMMON_MAIN_0164: Sig = Sig {
    len: 5494,
    hash: 0x1beb_f329_c6cd_daf3,
    calls: &[
        (0xc3, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xe6, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x139, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x13f, Callee::Sig(&crate::s4_small::STATIC_LINK_WORD)),
        (0x4aa, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x4f5, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x6db, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x89d, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x909, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xa3c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xa7e, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xa91, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xabd, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xaef, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xb04, Callee::Sig(&crate::s4_small::CALL_237)),
        (0xb34, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xb47, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xba1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xbe3, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xbf6, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xc6e, Callee::Sig(&crate::s4_small::STATIC_LINK_TAIL)),
        (0xd0a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xd30, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xd72, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xd85, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xdc4, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xdd8, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xdf2, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xe36, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xe78, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xf59, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xf6d, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xf87, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1025, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1039, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1053, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x10d0, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x10e4, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x10fe, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1131, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x127d, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x12c8, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x12d9, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x12e3, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x12f4, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x12fe, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1308, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1312, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x131c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1321, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x132b, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1330, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x1337, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1341, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x13b8, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x13c3, Callee::Sig(&crate::s4_small::CALL_239_EDX)),
        (0x1435, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x146d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x147a, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};
