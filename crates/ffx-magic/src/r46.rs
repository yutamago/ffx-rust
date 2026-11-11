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

/// [`main_task_0042`]'s code offset, frame and record: `base` is `E -
/// 0xc` (the aligned frame `E` below the three pushes), `r` the task's
/// record `R` (its state `R + 0x2a88`, timers `R + 0x2a8c`/`R + 0x2a90`,
/// the bone step `R + 0x2a94`, the count `R + 0x2a98`).
struct Ctx42 {
    f: u32,
    base: u32,
    r: u32,
}

impl Ctx42 {
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    fn k<G: Game>(&self, g: &G, img: &Image, o: u32) -> f32 {
        g.read::<f32>(self.w(img, o))
    }

    fn gw<G: Game>(&self, g: &G, img: &Image, o: u32) -> u32 {
        g.read::<u32>(self.w(img, o))
    }

    fn target(&self, img: &Image, o: u32) -> u32 {
        img.rel32_target(self.f + o)
    }

    /// Slot `n` through the imports pointer the instruction at operand
    /// `op` reads.
    fn slot<G: Game>(&self, g: &G, img: &Image, op: u32, n: u32) -> u32 {
        slot_target(g, self.w(img, op), n)
    }

    fn abs<G: Game>(&self, g: &mut G, img: &Image, from: u32, to: u32) {
        crate::r25::abs_stores(g, img, self.f, from, to);
    }

    fn at(&self, o: u32) -> u32 {
        self.r.wrapping_add(o)
    }

    fn wr<G: Game>(&self, g: &mut G, o: u32, v: u32) {
        g.write::<u32>(self.at(o), v);
    }

    fn rf<G: Game>(&self, g: &G, o: u32) -> f32 {
        g.read::<f32>(self.at(o))
    }

    fn set<G: Game>(&self, g: &mut G, s: u32) {
        self.wr(g, 0x2a88, s);
    }

    fn up<G: Game>(&self, g: &mut G) {
        let s = g.read::<u32>(self.at(0x2a88)).wrapping_add(1);
        self.set(g, s);
    }

    /// The count `R + 0x2a98` (up at every call).
    fn count<G: Game>(&self, g: &G) -> i32 {
        g.read::<i32>(self.at(0x2a98))
    }

    /// `K` (operand `o`) above the second timer: `comiss K, T; ja` waits.
    fn before<G: Game>(&self, g: &G, img: &Image, o: u32) -> bool {
        self.k(g, img, o) > self.rf(g, 0x2a90)
    }

    /// Slot `n` with one word `*G` (operand `x`), the imports operand `i`.
    fn slot_g<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (i, n, x): (u32, u32, u32),
        b: Option<u32>,
    ) -> u32 {
        let v = self.gw(g, img, x);
        let t = self.slot(g, img, i, n);
        match b {
            // call width: 2
            Some(b) => g.call(t, 0, &[v, b]).eax,
            // call width: 1
            None => g.call(t, 0, &[v]).eax,
        }
    }

    /// The value `v` set on the effect object at operand `obj` (the call
    /// at `c`).
    fn set_value<G: Game>(&self, g: &mut G, img: &Image, (c, obj): (u32, u32), v: u32) -> u32 {
        crate::m0042::set_value(g, img, self.target(img, c), self.w(img, obj), v)
    }

    /// `push d; push ecx; push b; push a`: an effect node (the call at
    /// `c`, `ecx` the node; the third word never read).
    fn effect<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node): (u32, u32),
        (a, b, d): (u32, u32, u32),
    ) -> u32 {
        let sp = self.base.wrapping_sub(0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 4, b);
        g.write::<u32>(sp + 0xc, d);
        let ef = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: node,
            ..Frame::default()
        };
        crate::s3_nodes::effect_node_with(g, img, &ef)
    }

    /// An effect node's flags (operand `o`, written at `wo`): bits 0, 1
    /// and 2 set, the run of constant stores `run` between. Returns them.
    fn flags<G: Game>(&self, g: &mut G, img: &Image, (o, wo): (u32, u32), run: (u32, u32)) -> u32 {
        let fl = self.gw(g, img, o) | 7;
        self.abs(g, img, run.0, run.1);
        g.write::<u32>(self.w(img, wo), fl);
        fl
    }

    /// `push n; sub esp, 8; push a`: a particle node (the call at `c`,
    /// `ecx` the node) `d` words below `base`. Returns its `eax`.
    fn particle<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node): (u32, u32),
        (a, n): (u32, u32),
    ) -> u32 {
        let sp = self.base.wrapping_sub(0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 0xc, n);
        let pf = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: node,
            ..Frame::default()
        };
        crate::s3_nodes::particle_node_with(g, img, &pf)
    }

    /// The node `R + 0xc70` freed and unlinked through `base + 0x1c` (the
    /// call at `c`). Returns its `eax`.
    fn free<G: Game>(&self, g: &mut G, img: &Image, c: u32, eax: u32) -> u32 {
        let p = self.base + 0x1c;
        g.write::<u32>(p, self.at(0xc70));
        let ff = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(4),
            ecx: p,
            eax,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff).eax
    }

    /// An owned node (`ecx`, the call at `c`, the list `edx`) with four
    /// stack words at `sp`. Returns its `eax`.
    fn owned<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node, list): (u32, u32, u32),
        words: [u32; 4],
        sp: u32,
    ) -> u32 {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let of = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: node,
            edx: list,
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of)
    }
}

/// `magic_0042`'s main task (0x10009490, 4333 bytes with its tables; a
/// task callback `(A)`, cdecl; an aligned frame `E` of 0x24 bytes below
/// three pushes, no /GS): `magic_0533`'s main task
/// ([`main_task_0533`](crate::r37b::main_task_0533)) in its own record
/// layout and steps. Every call steps both timers by `K` and the count;
/// states up to 0xcc dispatch through a byte table:
/// - 0 on into 0x64; 0x66 past its wait resets the first timer, on into
///   0x5a; 0x64: an effect node, a node placed by slots 413 and 262, an
///   owned and a particle node; 0x65: slots 719 and 306;
/// - 0x5a: nodes, slot 325, an owned and a particle node, on into 0x5b;
///   0x5b past `K`: the summon's model node and handle (slots 30, 10,
///   211, 671, the value, 670), the move's nodes and the four bones' link
///   nodes (as 0533's), on into 0x14;
/// - 0x14: one bone per call while the timer allows (as 0533's), then
///   slot 20 on the handle; when it answers 0, the camera's words, the
///   value, slot 670, the four link nodes released and the bones' nodes
///   leave their lists; on to 0x1e (0x1e on by one);
/// - 0x1f, 0x20, 0x28, 0x32: the fade (slot 325), the node freed, the
///   particle and effect nodes; the move down, slot 20, the node leaves,
///   the value, the release, an effect node; the timer's `K`, the node
///   leaves and is freed, a particle node, slot 549; 0xc8..0xcc: slots
///   671/670 around the value, an owned node and the camera's words,
///   slot 222; 0x46..0x48, 0x50/0x51: slots 20 and 549, the value, slot
///   670, then the task unlinked;
/// - 10000: `G_S` 10000, the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn main_task_0042<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; and esp, -8; sub esp, 0x24; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x24);
    let base = e.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let c = Ctx42 { f, base, r };
    let x = add(c.rf(g, 0x2a8c), c.k(g, img, 0x25));
    let y = add(c.rf(g, 0x2a90), c.k(g, img, 0x2d));
    let n = g.read::<u32>(c.at(0x2a98)).wrapping_add(1);
    c.wr(g, 0x2a98, n);
    g.write::<u32>(base + 0x1c, r);
    g.write::<f32>(c.at(0x2a8c), x);
    g.write::<f32>(c.at(0x2a90), y);
    let mut state = g.read::<u32>(c.at(0x2a88));
    let eax = loop {
        if state > 0x2710 {
            break state;
        }
        if state == 0x2710 {
            break end_0042(g, img, &c, task);
        }
        if state > 0xcc {
            break state;
        }
        let idx = img.word(c.w(img, 0x77) + state) & 0xff;
        match state {
            0 => {
                c.set(g, 0x64);
                state = 0x64;
            }
            0x66 => {
                if c.count(g) < 0x55 {
                    break idx;
                }
                c.wr(g, 0x2a8c, 0);
                c.set(g, 0x5a);
                state = 0x5a;
            }
            0x64 => break start_0042(g, img, &c),
            0x65 => {
                if c.count(g) < 0x19 {
                    break idx;
                }
                // call width: 3
                let h = g
                    .call(c.slot(g, img, 0x1c1, 719), 0, &[c.w(img, 0x1d0), 5, 0])
                    .eax;
                // call width: 3
                let eax = g.call(c.slot(g, img, 0x1db, 306), 0, &[7, 1, h]).eax;
                c.up(g);
                break eax;
            }
            0x5a | 0x5b => {
                let mut eax = idx;
                if state == 0x5a {
                    eax = show_0042(g, img, &c);
                }
                // comiss K, T1; ja: waits
                if c.k(g, img, 0x2a2) > c.rf(g, 0x2a8c) {
                    break eax;
                }
                let eax = model_0042(g, img, &c);
                break bones_0042(g, img, &c, eax);
            }
            0x14 => break bones_0042(g, img, &c, idx),
            0x1e => {
                c.up(g);
                break idx;
            }
            0x1f => break fade_0042(g, img, &c, idx),
            0x20 => break down_0042(g, img, &c),
            0x28 | 0x32 => {
                if state == 0x28 {
                    if c.before(g, img, 0xc9f) {
                        break idx;
                    }
                    let eax = crate::sa_misc::leave(g, c.at(0x2c), idx);
                    c.wr(g, 0x20, 0xbf33_3333);
                    let eax = c.free(g, img, 0xcca, eax);
                    let _ = eax;
                    c.particle(g, img, (0xcdd, c.at(0xc70)), (c.at(0xd70), 3));
                    c.set(g, 0x32);
                }
                // call width: 1
                let eax = g.call(c.slot(g, img, 0xcef, 549), 0, &[2]).eax;
                if eax == 0 {
                    c.set(g, 0xc8);
                }
                break eax;
            }
            0xc8 => {
                let eax = c.slot_g(g, img, (0xd1a, 671, 0xd22), Some(0));
                let _ = eax;
                c.set_value(g, img, (0xd39, 0xd34), 0x3007_105a);
                let eax = c.slot_g(g, img, (0xd3e, 670, 0xd46), Some(0));
                c.wr(g, 0x2a90, 0);
                c.wr(g, 0x2a98, 0);
                c.up(g);
                break eax;
            }
            0xc9 => {
                if c.count(g) < 2 {
                    break idx;
                }
                c.abs(g, img, 0xd6a, 0xd74);
                c.up(g);
                break idx;
            }
            0xca..=0xcc => break camera_0042(g, img, &c, state, idx),
            0x46..=0x48 => {
                if state == 0x46 {
                    c.up(g);
                    c.wr(g, 0x2a90, 0);
                }
                if state <= 0x47 {
                    c.up(g);
                    c.abs(g, img, 0xef4, 0xefe);
                }
                let eax = c.slot_g(g, img, (0xeff, 20, 0xf05), None);
                if eax != 0 {
                    break eax;
                }
                // call width: 1
                let eax = g.call(c.slot(g, img, 0xf1a, 549), 0, &[2]).eax;
                if eax != 0 {
                    break eax;
                }
                c.set_value(g, img, (0xf3e, 0xf39), 0x3007_105b);
                let eax = c.slot_g(g, img, (0xf43, 670, 0xf4b), Some(0));
                c.abs(g, img, 0xf5a, 0xf64);
                c.set(g, 0x50);
                break eax;
            }
            0x50 | 0x51 => {
                if state == 0x50 {
                    c.up(g);
                    c.wr(g, 0x2a90, 0);
                }
                if c.before(g, img, 0xf89) {
                    break idx;
                }
                break end_0042(g, img, &c, task);
            }
            _ => break idx,
        }
    };
    Ret::eax(eax)
}

/// [`main_task_0042`]'s end (10000, or 0x51 past `K`): `G_S` 10000, the
/// task unlinked. Returns the unlink's `eax`.
fn end_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42, task: u32) -> u32 {
    c.abs(g, img, 0xf98, 0xfa2);
    crate::h6_setup::unlink_task(g, task)
}

/// [`main_task_0042`]'s state 0x64: an effect node (flag bit 0), the node
/// `R + 0x9c4`, slot 413's handle placed by slot 262 into `R + 0x9e0..`,
/// the node `R + 0x9f0`, the owned node `R + 0xa1c` (its colour) and its
/// particle node; the count reset, on. Returns the particle node's `eax`.
fn start_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42) -> u32 {
    let w = |o: u32| c.w(img, o);
    c.effect(g, img, (0xd2, w(0xcd)), (w(0xc8), w(0xc3), 0));
    let fl = c.gw(g, img, 0xd8) | 1;
    g.write::<u32>(w(0xd8), fl);
    c.abs(g, img, 0xea, 0xf4);
    let a = c.at(0x9c4);
    crate::sb_misc::node_append(g, img, c.target(img, 0xf5), a, w(0xe4), 0);
    // call width: 1 (the effect node's words stay under it)
    let h = g.call(c.slot(g, img, 0xfa, 413), 0, &[0x16]).eax;
    let place = [c.at(0x9e0), c.at(0x9e4), c.at(0x9e8)];
    // the place's three words stay to the owned node
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([h, place[0], place[1], place[2]]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    // call width: 4
    g.call(
        c.slot(g, img, 0x122, 262),
        0,
        &[h, place[0], place[1], place[2]],
    );
    let list = w(0x135);
    let b = c.at(0x9f0);
    c.wr(g, 0x9ec, ONE);
    crate::sb_misc::node_append(g, img, c.target(img, 0x144), b, list, 0);
    for o in [0xa0cu32, 0xa10, 0xa14, 0xa18] {
        c.wr(g, o, ONE);
    }
    let owner = c.at(0xa1c);
    let sp = c.base.wrapping_sub(0x20);
    c.owned(g, img, (0x185, owner, list), [w(0x156), a, w(0x14a), b], sp);
    let eax = c.particle(g, img, (0x199, c.at(0xb70)), (owner, 5));
    c.wr(g, 0x2a98, 0);
    c.up(g);
    eax
}

/// [`main_task_0042`]'s state 0x5a: the node `R + 0xebc`, slot 325 into
/// `R + 0xed8`, the node `R + 0xee8`, the owned node `R + 0xd70` with its
/// colour, its particle node; on, `G_S` 0x5a. Returns the particle node's
/// `eax`.
fn show_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42) -> u32 {
    let w = |o: u32| c.w(img, o);
    let a = c.at(0xebc);
    crate::sb_misc::node_append(g, img, c.target(img, 0x209), a, w(0x202), 0);
    // call width: 2
    g.call(c.slot(g, img, 0x21a, 325), 0, &[c.at(0xed8), w(0x214)]);
    let b = c.at(0xee8);
    let list = w(0x22d);
    crate::sb_misc::node_append(g, img, c.target(img, 0x232), b, list, 0);
    for (o, v) in [
        (0xf04u32, ONE),
        (0xf08, ONE),
        (0xf0c, ONE),
        (0xf10, 0x3f66_6666),
    ] {
        c.wr(g, o, v);
    }
    // slot 325's two words stay under the owned node's four
    let owner = c.at(0xd70);
    let sp = c.base.wrapping_sub(0x18);
    c.owned(g, img, (0x273, owner, list), [w(0x244), a, w(0x238), b], sp);
    let eax = c.particle(g, img, (0x287, c.at(0xc70)), (owner, 4));
    c.up(g);
    c.abs(g, img, 0x294, 0x29e);
    eax
}

/// [`main_task_0042`]'s state 0x5b past its wait: the summon's model node
/// (`[M, P, 0x15, 1]`), slots 30, 10, 211 on its handle, the flag word,
/// slot 671, the value, slot 670; the move's nodes, the four bones' link
/// nodes (as 0533's), the bones' words; state 0x14. Returns the last
/// bone's `3 i + 4`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn model_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let grp = base + 0x20;
    for (i, v) in (0u32..).zip([w(0x2bb), w(0x2c3), 0x15]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0xc, 1);
    c.abs(g, img, 0x2d4, 0x2de);
    crate::sa_summon::model_node_init(g, img, c.target(img, 0x2df), grp);
    c.slot_g(g, img, (0x2e4, 30, 0x2ea), None);
    c.slot_g(g, img, (0x2f4, 10, 0x302), Some(0));
    let m = c.gw(g, img, 0x314);
    // call width: 2
    g.call(c.slot(g, img, 0x309, 211), 0, &[m, w(0x30e)]);
    let fl = g.read::<u16>(w(0x328)) | 0x10;
    g.write::<u16>(w(0x328), fl);
    let m = c.gw(g, img, 0x331);
    c.abs(g, img, 0x335, 0x33f);
    // call width: 2
    g.call(c.slot(g, img, 0x321, 671), 0, &[m, 0]);
    c.set_value(g, img, (0x352, 0x34d), 0x300f_1002);
    c.slot_g(g, img, (0x357, 670, 0x35f), Some(0));
    let r = c.r;
    crate::sb_misc::node_append(g, img, c.target(img, 0x377), c.at(0x58), w(0x372), 0);
    for (o, v) in [(0x74u32, 0), (0x78, 0x400), (0x7c, 0)] {
        c.wr(g, o, v);
    }
    crate::sa_misc::node_append(g, img, c.target(img, 0x398), r, w(0x37c), 0);
    for (o, v) in [(0x1cu32, 0), (0x20, 0x4040_0000), (0x24, 0), (0x28, 0)] {
        c.wr(g, o, v);
    }
    crate::sa_misc::node_append(g, img, c.target(img, 0x3be), c.at(0x2c), r, 0);
    for (o, v) in [(0x48u32, 0), (0x4c, 0x3ee6_6667), (0x50, 0), (0x54, 0)] {
        c.wr(g, o, v);
    }
    // the bones: four words in the frame step on per bone
    let (p10, p14, p18) = (base + 0x10, base + 0x14, base + 0x18);
    g.write::<u32>(p18, w(0x3e4));
    g.write::<u32>(p14, w(0x3ec));
    g.write::<u32>(p10, w(0x3f4));
    let list = w(0x401);
    let mut i = 0u32;
    let mut node = w(0x3f9);
    loop {
        crate::sb_misc::node_append(g, img, c.target(img, 0x408), node, list, 0);
        g.write::<u32>(node + 0x20, 0x479e_b100);
        g.write::<u32>(node + 0x28, ONE);
        // the second node appended inline (`esi` = 0x34 i)
        let o = 0x34 * i;
        let n2 = w(0x41c).wrapping_add(o);
        g.write::<u16>(w(0x423).wrapping_add(o), 0xff);
        g.write::<u32>(w(0x42b).wrapping_add(o), n2);
        g.write::<u32>(w(0x431).wrapping_add(o), n2);
        g.write::<u32>(w(0x437).wrapping_add(o), list);
        g.write::<u32>(w(0x43d).wrapping_add(o), list);
        let tail = c.gw(g, img, 0x442);
        g.write::<u32>(w(0x448).wrapping_add(o), tail);
        let tail = c.gw(g, img, 0x44d);
        if tail == list {
            g.write::<u32>(w(0x457), n2);
        } else {
            g.write::<u32>(tail + 4, n2);
        }
        let k = g.read::<u16>(w(0x463)).wrapping_add(1);
        g.write::<u16>(w(0x463), k);
        g.write::<u32>(w(0x469), n2);
        g.write::<u32>(n2, w(0x46f));
        g.write::<u32>(w(0x481).wrapping_add(o), node);
        g.write::<u32>(w(0x487).wrapping_add(o), w(0x48b));
        // the link nodes: (node, y), then (a, -, -, the first link node),
        // then (the second)
        let n3 = w(0x47a).wrapping_add(o);
        let sp = base.wrapping_sub(8);
        g.write::<u32>(sp, node);
        g.write::<u32>(sp + 4, w(0x474));
        let lf = Frame {
            at: c.target(img, 0x490),
            esp: sp.wrapping_sub(4),
            ecx: n3,
            ..Frame::default()
        };
        crate::s4_small::link_node_xy(g, img, &lf);
        let sp = base.wrapping_sub(0x10);
        g.write::<u32>(sp, g.read::<u32>(p10));
        g.write::<u32>(sp + 0xc, n3);
        let n4 = g.read::<u32>(p14);
        let lf = Frame {
            at: c.target(img, 0x4a4),
            esp: sp.wrapping_sub(4),
            ecx: n4,
            ..Frame::default()
        };
        crate::s4_small::link_node_ad(g, img, &lf);
        let p = g.read::<u32>(p18);
        let sp = base.wrapping_sub(0x14);
        g.write::<u32>(sp, n4);
        let lf = Frame {
            at: c.target(img, 0x4b1),
            esp: sp.wrapping_sub(4),
            ecx: p.wrapping_sub(0x20),
            ..Frame::default()
        };
        crate::s4_small::link_node_x(g, img, &lf);
        let v = g.read::<u32>(p10).wrapping_add(0x400);
        g.write::<u32>(p10, v);
        let v = g.read::<u32>(p14).wrapping_add(0x3c);
        g.write::<u32>(p14, v);
        node = node.wrapping_add(0x2c);
        #[allow(clippy::cast_possible_truncation)] // mov [edx], ax
        g.write::<u16>(p, (3 * i + 4) as u16);
        g.write::<u32>(p18, p.wrapping_add(0x24));
        i += 1;
        if node.cast_signed() >= w(0x4de).cast_signed() {
            break;
        }
    }
    c.abs(g, img, 0x4ec, 0x514);
    for o in [0x2a94u32, 0x2a98, 0x2a90] {
        c.wr(g, o, 0);
    }
    c.set(g, 0x14);
    // `lea eax, [ebx + eax * 2]`: the last bone's `3 i + 4`
    3 * (i - 1) + 4
}

/// [`main_task_0042`]'s state 0x14: while fewer than four bones and the
/// timer reached `K`, one bone (as 0533's: slot 324's word, the nodes and
/// their slot 325 vectors, the scale from slot 307, the owned node, slot
/// 719's handle and the grey link node); then slot 20 on the handle: when
/// it answers 0, the camera's words, the value, slot 670, the four link
/// nodes released, the bones' nodes leave their lists, state 0x1e.
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn bones_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42, eax_in: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let _ = eax_in;
    let step = |g: &G| g.read::<u32>(c.at(0x2a94));
    if step(g).cast_signed() < 4 && c.rf(g, 0x2a90) >= c.k(g, img, 0x554) {
        // call width: 0
        let v = g.call(c.slot(g, img, 0x55f, 324), 0, &[]).eax;
        let node = |j: u32| c.r.wrapping_add(j.wrapping_mul(0x2c));
        let i = step(g);
        crate::sb_misc::node_append(g, img, c.target(img, 0x581), node(i + 3), w(0x57a), 0);
        let i = step(g);
        // call width: 2
        g.call(
            c.slot(g, img, 0x59c, 325),
            0,
            &[node(i).wrapping_add(0xa0), w(0x596)],
        );
        let i = step(g);
        let x = add(g.read::<f32>(node(i) + 0xa4), c.k(g, img, 0x5c3));
        g.write::<f32>(node(i) + 0xa4, x);
        let i = step(g);
        crate::sb_misc::node_append(g, img, c.target(img, 0x5df), node(i + 7), w(0x5b2), 0);
        let i = step(g);
        // call width: 2 (slot 325's words stay under it)
        g.call(
            c.slot(g, img, 0x5fa, 325),
            0,
            &[node(i).wrapping_add(0x150), w(0x5f4)],
        );
        let i = step(g);
        g.write::<u32>(node(i) + 0x154, 0x4000_0000);
        let i = step(g);
        g.write::<u32>(node(i) + 0x15c, 0);
        let i = step(g);
        crate::sa_misc::node_append(g, img, c.target(img, 0x642), node(i + 0xf), w(0x610), 0);
        for (o, x) in [(0x2b0u32, 0x8000u32), (0x2b4, v & 0xffff), (0x2b8, 0)] {
            let i = step(g);
            g.write::<u32>(node(i) + o, x);
        }
        let i = step(g);
        crate::sb_misc::node_append(
            g,
            img,
            c.target(img, 0x698),
            node(i + 0x13),
            node(i + 0xf),
            0,
        );
        for (o, x) in [(0x360u32, 0u32), (0x364, (v & 0x400) + 0x200), (0x368, 0)] {
            let i = step(g);
            g.write::<u32>(node(i) + o, x);
        }
        let i = step(g);
        crate::sa_misc::node_append(g, img, c.target(img, 0x6f7), node(i + 0xb), node(i + 7), 0);
        let i = step(g);
        // call width: 2 (slot 325's words stay under it)
        g.call(
            c.slot(g, img, 0x712, 325),
            0,
            &[node(i).wrapping_add(0x200), w(0x70c)],
        );
        // call width: 0 (the earlier calls' words stay under it)
        let out = g.call(c.slot(g, img, 0x71f, 307), 0, &[]);
        let s = f32::from_bits(crate::sa_summon::st0_f32(out));
        // fstp [esp + 0x30]: `E + 0xc`
        g.write::<f32>(base + 0x18, s);
        let x = add(
            mul(sub(s, c.k(g, img, 0x739)), c.k(g, img, 0x74a)),
            c.k(g, img, 0x757),
        );
        let i = step(g);
        g.write::<f32>(node(i) + 0x20c, x);
        let i = step(g);
        let owner = c.r.wrapping_add(i.wrapping_mul(0x14c)).wrapping_add(0x3f4);
        let sp = base.wrapping_sub(0x28);
        c.owned(
            g,
            img,
            (0x799, owner, w(0x74f)),
            [w(0x792), node(i + 3), node(i + 0xf), node(i + 7)],
            sp,
        );
        // call width: 3 (the earlier calls' words stay under it)
        let h = g.call(c.slot(g, img, 0x79e, 719), 0, &[w(0x7ad), 4, 0]).eax;
        let i = step(g);
        let sp = base.wrapping_sub(8);
        g.write::<u32>(
            sp,
            c.r.wrapping_add(i.wrapping_mul(0x14c)).wrapping_add(0x3f4),
        );
        g.write::<u32>(sp + 4, h);
        let lf = Frame {
            at: c.target(img, 0x7da),
            esp: sp.wrapping_sub(4),
            ecx: c.r.wrapping_add(i.wrapping_mul(0x28)).wrapping_add(0x924),
            ..Frame::default()
        };
        crate::s4_small::link_node_grey(g, img, &lf);
        let i = step(g);
        g.write::<u8>(
            c.r.wrapping_add(i.wrapping_mul(0x28)).wrapping_add(0x94b),
            0x40,
        );
        c.wr(g, 0x2a94, i.wrapping_add(1));
        c.wr(g, 0x2a90, 0);
    }
    let eax = c.slot_g(g, img, (0x803, 20, 0x809), None);
    if eax != 0 {
        return eax;
    }
    // the camera's words
    for (a, b) in [(0x821u32, 0x829u32), (0x831, 0x839)] {
        let x = c.gw(g, img, a);
        g.write::<u32>(w(b), x);
    }
    let y = sub(c.k(g, img, 0x841), c.k(g, img, 0x849));
    c.abs(g, img, 0x857, 0x86b);
    g.write::<f32>(w(0x86f), y);
    c.abs(g, img, 0x873, 0x87d);
    c.set_value(g, img, (0x87e, 0x853), 0x300f_1003);
    c.slot_g(g, img, (0x883, 670, 0x88b), Some(0));
    let p = base + 0x18;
    for k in 0..4u32 {
        g.write::<u32>(p, c.at(0x924 + 0x28 * k));
        crate::m0042::unlink(g, img, c.target(img, 0x8b9), p);
    }
    // the bones' nodes leave their lists: five kinds, two bones (`edx`
    // walks the first kind's `+4` up to the operand at `+0xaaf`)
    let mut eax = 0;
    let mut k = 0u32;
    loop {
        let first = w(0x8ce).wrapping_sub(4).wrapping_add(0x2c * k);
        for (i, n) in (0u32..).zip([
            first,
            w(0x97f).wrapping_add(0x34 * k),
            w(0x9ee).wrapping_add(0x34 * k),
            w(0x8c9).wrapping_sub(4).wrapping_add(0x3c * k),
            w(0x8c4).wrapping_sub(4).wrapping_add(0x24 * k),
        ]) {
            let parent = g.read::<u32>(n + 0x14);
            if i == 0 {
                eax = parent;
            }
            if parent != 0 {
                crate::nodes::leave_parent(g, n, parent);
                eax = n;
            }
        }
        k += 1;
        if (first + 0x30).cast_signed() >= w(0xaaf).cast_signed() {
            break;
        }
    }
    for o in [0x2a90u32, 0x2a98] {
        c.wr(g, o, 0);
    }
    c.set(g, 0x1e);
    eax
}

/// [`main_task_0042`]'s state 0x1f once `G >= G'`: the fade's vector (slot
/// 325, its words cleared first), the light word, the node freed, a
/// particle node and an effect node (flags); on, `G_S` 0x64. Returns the
/// flags.
fn fade_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42, idx: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    // comiss G, G'; jb: waits (unordered too)
    if !(c.k(g, img, 0xae6) >= c.k(g, img, 0xaed)) {
        return idx;
    }
    c.abs(g, img, 0xb08, 0xb12);
    c.wr(g, 0x4c, 0);
    c.wr(g, 0x20, 0);
    // call width: 2
    let eax = g
        .call(c.slot(g, img, 0xb04, 325), 0, &[c.at(0xed8), w(0xafe)])
        .eax;
    let x = sub(c.rf(g, 0xedc), c.k(g, img, 0xb34));
    c.wr(g, 0xf10, 0x3e4c_cccd);
    g.write::<f32>(c.at(0xedc), x);
    c.free(g, img, 0xb5c, eax);
    let edi = c.at(0xd70);
    c.particle(g, img, (0xb6f, c.at(0xc70)), (edi, 1));
    c.effect(g, img, (0xb85, w(0xb80)), (edi, w(0xb7a), 1));
    let fl = c.flags(g, img, (0xb8a, 0xba2), (0xb97, 0xba1));
    c.up(g);
    c.abs(g, img, 0xbac, 0xbb6);
    fl
}

/// [`main_task_0042`]'s state 0x20: the move's word `R + 0x78` down by
/// 0x40 while not negative; then unless slot 20 answers, the node leaves,
/// the light word `G - G' K`, the value, the release, an effect node
/// (flags), `G_S` 0x14, state 0x28. Returns `eax` as the original leaves
/// it.
fn down_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42) -> u32 {
    let w = |o: u32| c.w(img, o);
    let m = g.read::<u32>(c.at(0x78));
    if m.cast_signed() >= 0 {
        let m = m.wrapping_add(0xffff_ffc0);
        c.wr(g, 0x78, m);
        return m;
    }
    let eax = c.slot_g(g, img, (0xbd2, 20, 0xbd8), None);
    if eax != 0 {
        return eax;
    }
    crate::sa_misc::leave(g, c.at(0x58), eax);
    let x = mul(c.k(g, img, 0xbf8), c.k(g, img, 0xc00));
    let y = sub(c.k(g, img, 0xc08), x);
    g.write::<f32>(w(0xc1e), y);
    c.wr(g, 0x4c, 0xbc13_74bc);
    c.wr(g, 0x20, 0);
    c.set_value(g, img, (0xc31, 0xc16), 0x300f_1000);
    let h = c.gw(g, img, 0xc37);
    crate::m0042::release(g, img, c.target(img, 0xc3c), h);
    c.effect(g, img, (0xc55, w(0xc50)), (c.at(0xd70), w(0xc4a), 2));
    let fl = c.flags(g, img, (0xc5a, 0xc72), (0xc67, 0xc71));
    c.abs(g, img, 0xc76, 0xc80);
    c.wr(g, 0x2a90, 0);
    c.set(g, 0x28);
    fl
}

/// [`main_task_0042`]'s states 0xca..0xcc: (0xca) an owned node (its word
/// kept), the camera's words `G K`, the value, slot 670, the light word,
/// the node freed, a particle node, slot 222, on; (0xcb) past `K` on,
/// `G_S` 0x46; (0xcc) past `K'` the move's words, state 0x46. Returns
/// `eax` as the original leaves it.
fn camera_0042<G: Game>(g: &mut G, img: &Image, c: &Ctx42, state: u32, idx: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let mut eax = idx;
    if state == 0xca {
        let sp = c.base.wrapping_sub(0x10);
        let node = w(0xd9b);
        c.owned(
            g,
            img,
            (0xda0, node, w(0xd96)),
            [w(0xd91), w(0xd8c), w(0xd87), w(0xd82)],
            sp,
        );
        // `ecx` the owned node as the append leaves it
        g.write::<u32>(w(0xda6), node);
        c.abs(g, img, 0xdaa, 0xdb4);
        c.wr(g, 0x20, 0);
        let x = mul(c.k(g, img, 0xdbf), c.k(g, img, 0xdc7));
        g.write::<f32>(w(0xdd9), x);
        let y = mul(c.k(g, img, 0xde1), c.k(g, img, 0xde9));
        c.abs(g, img, 0xded, 0xdf7);
        g.write::<f32>(w(0xdfb), y);
        let z = mul(c.k(g, img, 0xe03), c.k(g, img, 0xe0b));
        g.write::<f32>(w(0xe13), z);
        c.set_value(g, img, (0xe18, 0xdd1), 0x300f_1001);
        c.slot_g(g, img, (0xe1d, 670, 0xe25), Some(0));
        c.wr(g, 0xf10, 0x3eb3_3333);
        let eax2 = c.free(g, img, 0xe4d, 0);
        let _ = eax2;
        c.particle(g, img, (0xe60, c.at(0xc70)), (c.at(0xd70), 2));
        // call width: 4
        eax = g
            .call(c.slot(g, img, 0xe65, 222), 0, &[w(0xe76), 0xc, 0xc, 2])
            .eax;
        c.up(g);
    }
    if state <= 0xcb {
        if c.before(g, img, 0xe89) {
            return eax;
        }
        c.up(g);
        c.abs(g, img, 0xea0, 0xeaa);
    }
    if c.before(g, img, 0xeae) {
        return eax;
    }
    c.wr(g, 0x24, 0xbecc_cccd);
    c.wr(g, 0x20, 0xbf80_0000);
    c.set(g, 0x46);
    eax
}

/// [`main_task_0042`] (the `rel32`s: its callees in order).
pub const MAIN_TASK_0042: Sig = Sig {
    len: 4333,
    hash: 0x5e05_cd5f_d6f8_5512,
    calls: &[
        (0xd2, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xf5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x144, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x185, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x199, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x209, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x232, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x273, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x287, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x2df, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x352, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x377, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x398, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x3be, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x408, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x490, Callee::Sig(&crate::s4_small::LINK_NODE_XY)),
        (0x4a4, Callee::Sig(&crate::s4_small::LINK_NODE_AD)),
        (0x4b1, Callee::Sig(&crate::s4_small::LINK_NODE_X)),
        (0x581, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5df, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x642, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x698, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6f7, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x799, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x7da, Callee::Sig(&crate::s4_small::LINK_NODE_GREY)),
        (0x87e, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x8b9, Callee::Sig(&crate::m0042::UNLINK)),
        (0xb5c, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xb6f, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xb85, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xbf0, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xc31, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xc3c, Callee::Sig(&crate::m0042::RELEASE)),
        (0xc55, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xcb0, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xcca, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xcdd, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xd39, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xda0, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xe18, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xe4d, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xe60, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xf3e, Callee::Sig(&crate::m0042::SET_VALUE)),
    ],
    imports: &[],
};

/// A node after the parent's last as the overlays inline it, the last
/// read before (`old`): its `+8` (the store at operand `n8`), the
/// parent's first (the store run at `eq`) when `old` is the parent
/// (operand `cmp`), else `old + 4` the node (the immediate `ne`).
fn after_old<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    old: u32,
    (n8, cmp, eq, ne): (u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(n8), old);
    if old == w(cmp) {
        crate::r25::abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(ne));
    }
}

/// A node after a known one (`prev`) as the overlays inline it: the
/// parent's first (the store at `eq`) when `prev` is the parent (operand
/// `cmp`), else the store at `ne` (`prev + 4`).
fn after_known<G: Game>(g: &mut G, img: &Image, f: u32, prev: u32, (cmp, eq, ne): (u32, u32, u32)) {
    let at = if prev == img.word(f + cmp) { eq } else { ne };
    crate::r25::abs_stores(g, img, f, at, at + 10);
}

/// `magic_0071`'s main task (0x100076e0, 3544 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame `E` of 0x34 bytes below three
/// pushes; the record `R = A[4]`, its state `R + 0x18`, counts `R +
/// 0x30`/`R + 0x34` up at every exit). By the state:
/// - 0: two groups of nodes under the DLL's parent `P` (the store runs in
///   the code), each with the light's level (slot 333, or slot 601's
///   float when the flag is clear: `(K - v) K' / K''` truncated) and a
///   particle node; on into 0xa;
/// - 0xa: when the time since the last step reached `s K`, the step `s`
///   scaled, two nodes per call under `P` with their root nodes, node
///   inserts and particle nodes (three calls, then on);
/// - 0xb: past `K` a word run, on into 0x32;
/// - 0x32: past `K` slot 239, a node under `P`, slot 268, slots 232, 233
///   and 248 on a vector, two nodes, a particle and an effect node, slots
///   719 and 306, eight parts freed (slot 713, the node leaves, slot 713
///   again unless its flag); then 10000;
/// - 10000: the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn main_task_0071<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    // push ebp; and esp, -8; sub esp, 0x34; the cookie; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    g.write::<u32>(e + 0x30, g.read::<u32>(w(0xa)) ^ e);
    let base = e.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(base + 0x28, task);
    g.write::<u32>(base + 0xc, r);
    let at = |o: u32| r.wrapping_add(o);
    let mut state = g.read::<u32>(at(0x18));
    let eax = loop {
        match state {
            0xb => {
                // comiss K, G; jb: waits (unordered too)
                if !(k(g, 0x29) >= k(g, 0x43)) {
                    break state;
                }
                abs(g, 0x4d, 0x9d);
                g.write::<u32>(at(0x18), 0x32);
                state = 0x32;
            }
            0x32 => {
                if !(k(g, 0xb3) >= k(g, 0xba)) {
                    break state;
                }
                scene_0071(g, img, f, base);
                parts_0071(g, img, f);
                g.write::<u32>(at(0x18), 0x2710);
                state = 0x2710;
            }
            0 | 0xa => {
                let mut eax = state;
                if state == 0 {
                    eax = start_0071(g, img, f, base);
                    g.write::<u32>(at(0x2c), ONE);
                    g.write::<u32>(at(0x38), 0);
                    g.write::<u32>(at(0x18), 0xa);
                }
                break step_0071(g, img, f, base, r, eax);
            }
            0x2710 => break crate::h6_setup::unlink_task(g, task),
            _ => break state,
        }
    };
    for o in [0x30u32, 0x34] {
        let v = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), v);
    }
    Ret::eax(eax)
}

/// [`main_task_0071`]'s state 0x32 past its wait: slot 239 `(*G, -5,
/// 0)`, a node under `P` (its count up), slot 268, slots 232, 233 and 248
/// on the vector `E + 0x20`, two nodes (the second after the first), a
/// particle node `[a, b, c, *G, e, 2]`, an effect node and its flag,
/// slots 719 and 306.
fn scene_0071<G: Game>(g: &mut G, img: &Image, f: u32, base: u32) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    let m = gw(g, 0xd5);
    // call width: 3
    g.call(slot(g, 0xc5, 239), 0, &[m, 0xffff_fffb, 0]);
    let old = gw(g, 0xdc);
    abs(g, 0xe3, 0x114);
    after_old(g, img, f, old, (0x115, 0x11a, 0x120, 0x12f));
    let n = g.read::<u16>(w(0x13b)).wrapping_add(1);
    g.write::<u16>(w(0x13b), n);
    let m = gw(g, 0x148);
    abs(g, 0x14c, 0x160);
    // call width: 3
    g.call(slot(g, 0x134, 268), 0, &[m, 0x11, w(0x140)]);
    // the vector `E + 0x20` (`base + 0x2c`)
    let v = base + 0x2c;
    abs(g, 0x17a, 0x184);
    // call width: 2
    g.call(slot(g, 0x176, 232), 0, &[v, w(0x170)]);
    // call width: 3 (`mov [esp], 5.0` over the word left)
    g.call(slot(g, 0x19d, 233), 0, &[v, v, 0x40a0_0000]);
    // call width: 3
    g.call(slot(g, 0x1b2, 248), 0, &[w(0x1c2), w(0x1b7), v]);
    let old = gw(g, 0x1c9);
    abs(g, 0x1d0, 0x20b);
    after_old(g, img, f, old, (0x20c, 0x211, 0x217, 0x226));
    let n = g.read::<u16>(w(0x22c));
    abs(g, 0x232, 0x2a9);
    after_known(g, img, f, w(0x2e), (0x2ab, 0x2b1, 0x2bd));
    g.write::<u16>(w(0x2cb), n.wrapping_add(2));
    let x = gw(g, 0x2d0);
    abs(g, 0x2d8, 0x311);
    let rec = base + 0x10;
    for (i, v) in (0u32..).zip([w(0x315), w(0x31d), w(0x325), x, w(0x331), 2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    abs(g, 0x33d, 0x347);
    crate::sb_nodes::particle_node(g, img, call(0x348), rec);
    for (i, v) in (0u32..).zip([w(0x354), w(0x35c), w(0x364), w(0x36c), w(0x374), 1]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, call(0x381), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x38d));
    let fl = gw(g, 0x398) | 1;
    g.write::<u32>(w(0x398), fl);
    abs(g, 0x39d, 0x3a7);
    // call width: 3
    let h = g.call(slot(g, 0x392, 719), 0, &[w(0x3b0), 5, 0]).eax;
    // call width: 3
    g.call(slot(g, 0x3bb, 306), 0, &[7, 1, h]);
}

/// [`main_task_0071`]'s eight parts (`0x100` apart from the operand
/// `+0x3cf`, each `+4` into its node): the environment pointer takes the
/// part's `+0x4c`, slot 713 `(part + 0x5c)`, the node leaves its parent,
/// and again the pointer and slot 713 unless bit 2 of `+0xe4`.
fn parts_0071<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |o: u32| img.word(f + o);
    let mut p = w(0x3cf);
    for _ in 0..8 {
        let v = g.read::<u32>(p + 0x4c);
        g.write::<u32>(g.read::<u32>(w(0x3e1)), v);
        // call width: 1
        g.call(slot_target(g, w(0x3eb), 713), 0, &[p + 0x5c]);
        crate::sa_misc::leave(g, p.wrapping_sub(4), 0);
        if g.read::<u8>(p + 0xe4) & 4 == 0 {
            let v = g.read::<u32>(p + 0x4c);
            g.write::<u32>(g.read::<u32>(w(0x449)), v);
            // call width: 1
            g.call(slot_target(g, w(0x453), 713), 0, &[p + 0x5c]);
        }
        p = p.wrapping_add(0x100);
    }
}

/// [`main_task_0071`]'s state 0: two groups of a node pair under `P`, the
/// light's level and a particle node `[a, b, c, *G, e, 1]` each, the
/// first group's level then a pair after it. Returns the second particle
/// node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0071<G: Game>(g: &mut G, img: &Image, f: u32, base: u32) -> u32 {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    let rec = base + 0x10;
    let mut eax = 0;
    // (the two groups' operands: the pair's, the level's, the second
    // pair's, the particle node's)
    for (pair1, level, pair2, part) in [
        (
            (
                0x493u32,
                (0x497u32, 0x4c8u32),
                (0x4c9u32, 0x4ceu32, 0x4d4u32, 0x4e3u32),
                0x4e9u32,
                0x4eeu32,
                (0x4f4u32, 0x56bu32),
                (0x56du32, 0x573u32, 0x57fu32),
            ),
            (
                0x58du32,
                (0x592u32, 0x59cu32),
                0x59eu32,
                (0x5a2u32, 0x5b6u32),
                [
                    0x5b9u32, 0x5bfu32, 0x5d3u32, 0x5e8u32, 0x5eeu32, 0x5feu32, 0x60fu32, 0x62bu32,
                ],
                [(0x613u32, 0x627u32), (0x62f, 0x638), (0x63c, 0x646)],
                0x647u32,
            ),
            (
                0x64cu32,
                (0x650u32, 0x66eu32),
                (0x66fu32, 0x674u32, 0x67au32, 0x689u32),
                0x68fu32,
                0x694u32,
                (0x69au32, 0x707u32),
                (0x709u32, 0x70fu32, 0x71bu32),
                0x729u32,
            ),
            (
                0x72eu32,
                (0x736u32, 0x76fu32),
                [0x773u32, 0x77b, 0x783, 0x78f],
                (0x79bu32, 0x7a5u32),
                0x7a6u32,
            ),
        ),
        (
            (
                0x7ab,
                (0x7af, 0x7e0),
                (0x7e1, 0x7e6, 0x7ec, 0x7fb),
                0x801,
                0x806,
                (0x80c, 0x883),
                (0x885, 0x88b, 0x897),
            ),
            (
                0x8a5,
                (0x8aa, 0x8b4),
                0x8b6,
                (0x8ba, 0x8ce),
                [0x8d1, 0x8d7, 0x8eb, 0x900, 0x906, 0x916, 0x927, 0x943],
                [(0x92b, 0x93f), (0x947, 0x950), (0x954, 0x95e)],
                0x95f,
            ),
            (
                0x964,
                (0x968, 0x986),
                (0x987, 0x98c, 0x992, 0x9a1),
                0x9a7,
                0x9ac,
                (0x9b2, 0xa1f),
                (0xa21, 0xa27, 0xa33),
                0xa41,
            ),
            (
                0xa46,
                (0xa4e, 0xa87),
                [0xa8b, 0xa93, 0xa9b, 0xaa7],
                (0xab3, 0xabd),
                0xabe,
            ),
        ),
    ] {
        // a node after `P`'s last, then one after it (the count kept in
        // `ax` to the level's store)
        let (last, run, ops, cnt, prev, run2, known) = pair1;
        let old = gw(g, last);
        abs(g, run.0, run.1);
        after_old(g, img, f, old, ops);
        let n = g.read::<u16>(w(cnt));
        abs(g, run2.0, run2.1);
        after_known(g, img, f, w(prev), known);
        // the light's level: slot 333 with the flag, else slot 601's float
        let (flag, pre, count_w, post, lv, scale_runs, store) = level;
        let on = g.read::<u8>(w(flag)) & 1 != 0;
        abs(g, pre.0, pre.1);
        g.write::<u16>(w(count_w), n.wrapping_add(2));
        abs(g, post.0, post.1);
        let [s333, a333, k333, s601, a601, k601, km, kd] = lv;
        let x = if on {
            let m = gw(g, a333);
            // call width: 1
            let out = g.call(slot_target(g, w(s333), 333), 0, &[m]);
            let v = f32::from_bits(crate::sa_summon::st0_f32(out));
            // fstp [esp + 0x10]: over the record word at `base + 0xc`
            g.write::<f32>(base + 0xc, v);
            sub(k(g, k333), v)
        } else {
            let m = gw(g, a601);
            // call width: 2 (the float into `base + 0xc`)
            g.call(slot_target(g, w(s601), 601), 0, &[m, base + 0xc]);
            sub(k(g, k601), g.read::<f32>(base + 0xc))
        };
        let x = mul(x, k(g, km));
        abs(g, scale_runs[0].0, scale_runs[0].1);
        let x = div(x, k(g, kd));
        abs(g, scale_runs[1].0, scale_runs[1].1);
        let v = cvtt(x).cast_unsigned();
        abs(g, scale_runs[2].0, scale_runs[2].1);
        g.write::<u32>(w(store), v);
        // a second pair after `P`'s last
        let (last, run, ops, cnt, prev, run2, known, cnt_w) = pair2;
        let old = gw(g, last);
        abs(g, run.0, run.1);
        after_old(g, img, f, old, ops);
        let n = g.read::<u16>(w(cnt));
        abs(g, run2.0, run2.1);
        after_known(g, img, f, w(prev), known);
        g.write::<u16>(w(cnt_w), n.wrapping_add(2));
        // the particle node `[a, b, c, *G, e, 1]`
        let (gx, run5, ops5, run6, c) = part;
        let x = gw(g, gx);
        abs(g, run5.0, run5.1);
        for (i, v) in (0u32..).zip([w(ops5[0]), w(ops5[1]), w(ops5[2]), x, w(ops5[3]), 1]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        abs(g, run6.0, run6.1);
        eax = crate::sb_nodes::particle_node(g, img, call(c), rec);
    }
    eax
}

/// [`main_task_0071`]'s state 0xa: when `G - R + 0x28` reached `s K` (`s
/// = R + 0x2c`), `s` scaled by `K'`, `R + 0x28 = G`, and for the step
/// `i = R + 0x38` two nodes under `P` (`0x2c i` on from their operands)
/// with their root nodes (`0x3c i`), node inserts (`0x14c i`) and
/// particle nodes (`0x100 i`, the kind from a table); the step on, the
/// state on after three. Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn step_0071<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, r: u32, eax_in: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    let s = g.read::<f32>(at(0x2c));
    let d = sub(k(g, 0xae0), g.read::<f32>(at(0x28)));
    let lim = mul(s, k(g, 0xaf0));
    // comiss d, lim; jb: waits (unordered too)
    if !(d >= lim) {
        return eax_in;
    }
    let i = g.read::<u32>(at(0x38));
    let s = mul(s, k(g, 0xb04));
    g.write::<f32>(at(0x2c), s);
    let t = gw(g, 0xb25);
    g.write::<u32>(at(0x28), t);
    let rec = base + 0x10;
    let mut eax = eax_in;
    for (ops, x0, ops2) in [
        (
            [
                0xb2bu32, 0xb37, 0xb67, 0xb6d, 0xb4a, 0xb51, 0xb56, 0xb61, 0xb72, 0xb7a, 0xb86,
                0xb8c, 0xb92, 0xbb4,
            ],
            ONE,
            [
                0xbc7u32, 0xbd4, 0xbf0, 0xbf8, 0xc00, 0xc0e, 0xc27, 0xc33, 0xc3f, 0xc49, 0xc09,
                0xbbf, 0xc4e,
            ],
        ),
        (
            [
                0xc65, 0xc72, 0xca2, 0xca8, 0xc85, 0xc8c, 0xc91, 0xc9c, 0xcad, 0xcb5, 0xcc1, 0xcc7,
                0xccd, 0xcef,
            ],
            0xbf80_0000,
            [
                0xd02, 0xd0f, 0xd2b, 0xd33, 0xd3b, 0xd49, 0xd62, 0xd6e, 0xd7a, 0xd84, 0xd44, 0xcfa,
                0xd89,
            ],
        ),
    ] {
        let [nb, cb, eb, sb, par, prv, l1, l2, cmp, eq, cnt, lw, vt, kk] = ops;
        let node = w(nb).wrapping_add(i.wrapping_mul(0x2c));
        let cn = w(cb).wrapping_add(i.wrapping_mul(0x3c));
        let part = w(eb).wrapping_add(i << 8);
        let ins = w(sb).wrapping_add(i.wrapping_mul(0x14c));
        g.write::<u16>(node + 0x1a, 0xff);
        g.write::<u32>(node + 0x10, node);
        g.write::<u32>(node + 0xc, node);
        g.write::<u32>(node + 0x14, w(par));
        g.write::<u32>(node + 4, w(prv));
        let old = gw(g, l1);
        g.write::<u32>(node + 8, old);
        let old = gw(g, l2);
        if old == w(cmp) {
            g.write::<u32>(w(eq), node);
        } else {
            g.write::<u32>(old.wrapping_add(4), node);
        }
        let n = g.read::<u16>(w(cnt)).wrapping_add(1);
        g.write::<u16>(w(cnt), n);
        g.write::<u32>(w(lw), node);
        g.write::<u32>(node, w(vt));
        g.write::<u32>(node + 0x1c, x0);
        g.write::<u32>(node + 0x20, ONE);
        g.write::<u32>(node + 0x24, ONE);
        let y = mul(g.read::<f32>(at(0x2c)), k(g, kk));
        g.write::<f32>(node + 0x28, y);
        let [
            k34,
            k38,
            a1,
            a2,
            a4,
            gx,
            tab,
            b1,
            b4,
            pvt,
            cins,
            croot,
            cpart,
        ] = ops2;
        // push ecx (unread): the root node
        g.write::<u32>(base.wrapping_sub(4), cn);
        crate::m0071::root_node(g, img, call(croot), cn);
        let v = gw(g, k34);
        g.write::<u32>(cn + 0x34, v);
        let v = gw(g, k38);
        g.write::<u32>(cn + 0x38, v);
        // the insert's group `[ins, a, b, cn, d, node]`
        for (j, v) in (0u32..).zip([ins, w(a1), w(a2), cn, w(a4), node]) {
            g.write::<u32>(rec + 4 * j, v);
        }
        let nf = Frame {
            at: call(cins),
            esp: base.wrapping_sub(4),
            ecx: rec,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &nf);
        let x = gw(g, gx);
        g.write::<u8>(ins + 0x148, 0);
        let kind = g.read::<u32>(w(tab).wrapping_add(4 * g.read::<u32>(at(0x38))));
        for (j, v) in (0u32..).zip([part, w(b1), ins, x, w(b4), kind]) {
            g.write::<u32>(rec + 4 * j, v);
        }
        g.write::<u32>(part, w(pvt));
        eax = crate::sb_nodes::particle_node(g, img, call(cpart), rec);
    }
    let i = g.read::<u32>(at(0x38)).wrapping_add(1);
    g.write::<u32>(at(0x38), i);
    if i.cast_signed() >= 3 {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    }
    eax
}

/// [`main_task_0071`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const MAIN_TASK_0071: Sig = Sig {
    len: 3544,
    hash: 0x86fa_0afc_759e_cf61,
    calls: &[
        (0x348, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x381, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x7a6, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xabe, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xbbf, Callee::Sig(&crate::m0071::ROOT_NODE)),
        (0xc09, Callee::Sig(&crate::nodes::INSERT)),
        (0xc4e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xcfa, Callee::Sig(&crate::m0071::ROOT_NODE)),
        (0xd44, Callee::Sig(&crate::nodes::INSERT)),
        (0xd89, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xdd0, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0071`'s scene task (0x100084c0, 3369 bytes with its tables; a
/// task callback `(A)`, cdecl; an aligned /GS frame `E` of 0x48 bytes
/// below two pushes; the record `R = A[4]`, its state `R + 0x18`, counts
/// `R + 0x28`/`R + 0x2c` (the wait) up at every exit). By the state:
/// - 0: slots 719 and 306, two summon model nodes, the root node and a
///   colour node; on into 0xa;
/// - 0xa: the fade's words (`K - G` each), a pair under the DLL's parent
///   `P`, the big node, an effect node, the static node, a node and its
///   insert, a particle node; on (the wait reset);
/// - 0xb: past its wait slot 228 `(*G, 0x58)`, on to 0x14;
/// - 0x14: past its wait slot 268 into the vector `V`, the static list's
///   node, a node under `P` placed `K` of the way to the target, the
///   direction scaled by `max(K', K'' + dy) / |(dx, dz)|`, slot 234,
///   nodes, slot 325, a pair, a particle node, the fade's words; on into
///   0x15;
/// - 0x15, 0x3c, 0x3d: when slot 411 answers, slot 228 (0x59, 0x5a, 0x5b);
///   0x15 then slot 237 into `V`, the vector node, slot 719 twice (state
///   0x3c), 0x3c on, 0x3d to 10000;
/// - 10000: when slot 411 answers, the list unlinked, the summon's model
///   set-up, the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn scene_task_0071<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    // push ebp; and esp, -8; sub esp, 0x48; the cookie; two pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    g.write::<u32>(e + 0x44, g.read::<u32>(w(0xa)) ^ e);
    let base = e.wrapping_sub(8);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let at = |o: u32| r.wrapping_add(o);
    let count = |g: &G| g.read::<i32>(at(0x2c));
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    // slot 411 `(*G)` (operands `i`, `x`) and, when it answers, slot 228
    // `(*G', n)`
    let gate = |g: &mut G, (i, x): (u32, u32), (j, y, n): (u32, u32, u32)| {
        let m = gw(g, x);
        // call width: 1
        let a = g.call(slot(g, i, 411), 0, &[m]).eax;
        if a == 0 {
            return None;
        }
        let m = gw(g, y);
        // call width: 2
        Some(g.call(slot(g, j, 228), 0, &[m, n]).eax)
    };
    let mut state = g.read::<u32>(at(0x18));
    let eax = loop {
        // `eax`: the byte table's index up to 0x14, else the state less
        // the cases' subtractions (10000 in all)
        let idx = if state <= 0x14 {
            img.word(w(0x3a) + state) & 0xff
        } else {
            state.wrapping_sub(0x2710)
        };
        match state {
            0xb => {
                if count(g) < 0x28 {
                    break idx;
                }
                let m = gw(g, 0x58);
                // call width: 2
                g.call(slot(g, 0x50, 228), 0, &[m, 0x58]);
                g.write::<u32>(at(0x2c), 0);
                set(g, 0x14);
                state = 0x14;
            }
            0x14 => {
                if count(g) < 0x14 {
                    break idx;
                }
                aim_0071(g, img, f, base);
                up(g);
                state = g.read::<u32>(at(0x18));
            }
            0 | 0xa => {
                if state == 0 {
                    // call width: 3
                    let h = g.call(slot(g, 0x6e5, 719), 0, &[w(0x6f4), 5, 0]).eax;
                    // call width: 3
                    g.call(slot(g, 0x6ff, 306), 0, &[7, 0, h]);
                    let grp = base + 0x34;
                    for (i, (n, x, b)) in [(0x71fu32, 0x727u32, 0x710u32), (0x752, 0x75a, 0x746)]
                        .into_iter()
                        .enumerate()
                    {
                        let v = if i == 0 {
                            gw(g, b)
                        } else {
                            u32::from(g.read::<u8>(w(b)))
                        };
                        for (j, y) in (0u32..).zip([w(n), w(x), v]) {
                            g.write::<u32>(grp + 4 * j, y);
                        }
                        g.write::<u8>(grp + 0xc, 0);
                        let (run, c) = if i == 0 {
                            (0x734, 0x73f)
                        } else {
                            (0x767, 0x772)
                        };
                        crate::r25::abs_stores(g, img, f, run, run + 10);
                        crate::sa_summon::model_node_init(g, img, call(c), grp);
                    }
                    crate::m0070::node_init_root(g, img, call(0x77c), w(0x777));
                    let cg = base + 0xc;
                    g.write::<u32>(cg, w(0x788));
                    g.write::<u32>(cg + 4, w(0x790));
                    crate::r25::abs_stores(g, img, f, 0x794, 0x79e);
                    let cf = Frame {
                        at: call(0x79f),
                        esp: base.wrapping_sub(4),
                        ecx: cg,
                        ..Frame::default()
                    };
                    crate::s3_misc::colour_node(g, img, &cf);
                    set(g, 0xa);
                }
                let eax = build_0071(g, img, f, base);
                g.write::<u32>(at(0x2c), 0);
                up(g);
                break eax;
            }
            0x15 => {
                let Some(_) = gate(g, (0xbd1, 0xbd7), (0xbef, 0xbf7, 0x59)) else {
                    break 0;
                };
                // slot 237 `(*G, 0x122, 0, V)`
                let v = base + 0x34;
                let m = gw(g, 0xc1c);
                // call width: 4
                g.call(slot(g, 0xc09, 237), 0, &[m, 0x122, 0, v]);
                // push V; sub esp, 0xc: the vector node `(_, _, _, V)`
                g.write::<u32>(base.wrapping_sub(4), v);
                let vf = Frame {
                    at: call(0xc2e),
                    esp: base.wrapping_sub(0x14),
                    ..Frame::default()
                };
                crate::o4_ctor::vec_ctor(g, img, &vf);
                for (i, a, s) in [(0xc33u32, 0xc42u32, 0xc4cu32), (0xc51, 0xc60, 0xc67)] {
                    // call width: 3
                    let h = g.call(slot(g, i, 719), 0, &[w(a), 9, 0]).eax;
                    g.write::<u32>(w(s), h);
                }
                let eax = gw(g, 0xc67);
                set(g, 0x3c);
                break eax;
            }
            0x3c => {
                let Some(eax) = gate(g, (0xcb3, 0xcb9), (0xccd, 0xcd5, 0x5a)) else {
                    break 0;
                };
                up(g);
                break eax;
            }
            0x3d => {
                let Some(eax) = gate(g, (0xc78, 0xc7e), (0xc92, 0xc9a, 0x5b)) else {
                    break 0;
                };
                set(g, 0x2710);
                break eax;
            }
            0x2710 => {
                let m = gw(g, 0x698);
                // call width: 1
                let a = g.call(slot(g, 0x692, 411), 0, &[m]).eax;
                if a == 0 {
                    break 0;
                }
                crate::l3_common::list_unlink_current(g, img, call(0x6b0), a);
                let rec = base + 0xc;
                g.write::<u32>(rec, w(0x6bc));
                let mf = Frame {
                    at: call(0x6c1),
                    esp: base.wrapping_sub(4),
                    ecx: rec,
                    ..Frame::default()
                };
                crate::sa_summon::model_setup(g, img, &mf);
                break crate::h6_setup::unlink_task(g, task);
            }
            _ => break idx,
        }
    };
    for o in [0x28u32, 0x2c] {
        let v = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), v);
    }
    Ret::eax(eax)
}

/// `K - G` (operands `k`, `x`) into the operand `d`'s word.
fn k_minus<G: Game>(g: &mut G, img: &Image, f: u32, (k, x, d): (u32, u32, u32)) {
    let w = |o: u32| img.word(f + o);
    let v = sub(g.read::<f32>(w(k)), g.read::<f32>(w(x)));
    g.write::<f32>(w(d), v);
}

/// [`scene_task_0071`]'s state 0xa: the fade's words (`K - G`), a node
/// after `P`'s last and one after it, the big node, an effect node and
/// its flag, the static node, a node and its insert, a particle node
/// `[a, b, c, *G, e, 6]`. Returns the particle node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn build_0071<G: Game>(g: &mut G, img: &Image, f: u32, base: u32) -> u32 {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    let km = |g: &mut G, x: u32, d: u32| k_minus(g, img, f, (0x7ae, x, d));
    km(g, 0x7b9, 0x7e1);
    let old = gw(g, 0x7be);
    let v = gw(g, 0x7c4) & !1;
    g.write::<u32>(w(0x7c4), v);
    abs(g, 0x7c9, 0x7dd);
    km(g, 0x7ec, 0x81c);
    abs(g, 0x7f0, 0x818);
    km(g, 0x827, 0x856);
    abs(g, 0x82b, 0x852);
    km(g, 0x861, 0x891);
    abs(g, 0x865, 0x88d);
    km(g, 0x89c, 0x8a9);
    g.write::<u32>(w(0x8a1), old);
    km(g, 0x8b4, 0x8c4);
    km(g, 0x8bc, 0x8dc);
    let x = sub(g.read::<f32>(w(0x8cc)), g.read::<f32>(w(0x8d4)));
    g.write::<f32>(w(0x8e4), x);
    if old == w(0x8e9) {
        abs(g, 0x8ef, 0x8f9);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x8fe));
    }
    let n = g.read::<u16>(w(0x904));
    abs(g, 0x90f, 0x97c);
    after_known(g, img, f, w(0x909), (0x97e, 0x984, 0x990));
    abs(g, 0x9a0, 0x9aa);
    g.write::<u16>(w(0x9ac), n.wrapping_add(2));
    abs(g, 0x9b0, 0x9e2);
    let rec = base + 0x34;
    for (i, o) in (0u32..).zip([0x9e6u32, 0x9ee, 0x9f6, 0x9fe, 0xa06]) {
        g.write::<u32>(rec + 4 * i, w(o));
    }
    crate::m0070::node_init_big_table(g, img, call(0xa0b), rec);
    let x = gw(g, 0xa10);
    g.write::<u32>(w(0xa19), x);
    abs(g, 0xa1d, 0xa27);
    for (i, o) in (0u32..).zip([0xa2bu32, 0xa33, 0xa3b, 0xa43, 0xa4b]) {
        g.write::<u32>(rec + 4 * i, w(o));
    }
    g.write::<u32>(rec + 0x14, 0);
    crate::sb_nodes::effect_node(g, img, call(0xa58), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0xa63));
    let fl = gw(g, 0xa69) | 1;
    g.write::<u32>(w(0xa69), fl);
    abs(g, 0xa6e, 0xa78);
    // push ecx (unread): the static node
    crate::m0071::static_node(g, img, call(0xa79));
    let old = gw(g, 0xa7e);
    abs(g, 0xa85, 0xade);
    after_old(g, img, f, old, (0xadf, 0xae4, 0xaea, 0xaf9));
    let n = g.read::<u16>(w(0xb00)).wrapping_add(1);
    g.write::<u16>(w(0xb00), n);
    abs(g, 0xb08, 0xb44);
    let grp = base + 0x14;
    for (i, o) in (0u32..).zip([0xb48u32, 0xb50, 0xb58, 0xb60, 0xb68, 0xb70]) {
        g.write::<u32>(grp + 4 * i, w(o));
    }
    let nf = Frame {
        at: call(0xb75),
        esp: base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &nf);
    let x = gw(g, 0xb7a);
    abs(g, 0xb82, 0xb89);
    for (i, v) in (0u32..).zip([w(0xb8d), w(0xb95), w(0xb9d), x, w(0xba9), 6]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs(g, 0xbb5, 0xbbf);
    crate::sb_nodes::particle_node(g, img, call(0xbc0), grp)
}

/// [`scene_task_0071`]'s state 0x14 past its wait: slot 268 into `V`
/// (`base + 0x34`, `w` 1.0), the static list's node, a node under `P`
/// placed `K` of the way from `V` to the target `T` (`d = (T - V) K`), the
/// flat direction `(-dz, 0, dx)` scaled by `max(K', K'' + dy) /
/// |(dx, dz)|` (by 0 when the length is 0), slot 234, nodes, slot 325, a
/// pair, a particle node `[a, b, c, *G, e, 0]`, the fade's words.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn aim_0071<G: Game>(g: &mut G, img: &Image, f: u32, base: u32) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    let v = base + 0x34;
    let m = gw(g, 0x92);
    // call width: 3
    g.call(slot(g, 0x8a, 268), 0, &[m, 0x11, v]);
    // the static list's node (its old last's `+4` always it)
    let old = gw(g, 0x9f);
    g.write::<u32>(w(0xa4), old);
    g.write::<u32>(v + 0xc, ONE);
    abs(g, 0xb0, 0xd8);
    g.write::<u32>(old.wrapping_add(4), w(0xdb));
    let last = gw(g, 0xe0);
    let x = gw(g, 0xe6) & !1;
    g.write::<u32>(w(0xe6), x);
    abs(g, 0xee, 0x133);
    after_old(g, img, f, last, (0x134, 0x139, 0x13f, 0x14e));
    // the node `K` of the way from `V` to the target
    let vf = |g: &G, i: u32| g.read::<f32>(v + 4 * i);
    let kk = k(g, 0x164);
    let dx = mul(sub(k(g, 0x156), vf(g, 0)), kk);
    let dz = mul(sub(k(g, 0x16c), vf(g, 2)), kk);
    let n = g.read::<u16>(w(0x185)).wrapping_add(1);
    g.write::<u16>(w(0x185), n);
    abs(g, 0x189, 0x193);
    g.write::<f32>(base + 0xc, dx);
    g.write::<f32>(w(0x1aa), dx);
    g.write::<f32>(w(0x1b2), add(dx, vf(g, 0)));
    let y = vf(g, 1);
    let dy = mul(sub(k(g, 0x174), y), kk);
    g.write::<f32>(w(0x1c4), dz);
    g.write::<f32>(base + 0x2c, dy);
    g.write::<f32>(w(0x1da), dy);
    g.write::<f32>(w(0x1e2), add(y, dy));
    g.write::<f32>(w(0x1f3), add(dz, vf(g, 2)));
    // xorps with the sign mask: -dz
    let ndz = f32::from_bits(dz.to_bits() ^ g.read::<u32>(w(0x1fd)));
    g.write::<f32>(base + 0x30, ndz);
    let len = f64::from(add(mul(dx, dx), mul(dz, dz))).sqrt() as f32;
    let ndy = f32::from_bits(dy.to_bits() ^ g.read::<u32>(w(0x237)));
    let mut s = sub(k(g, 0x223), ndy);
    let floor = k(g, 0x243);
    // comiss K', s; jbe: unordered keeps `s`
    if floor > s {
        s = floor;
    }
    // ucomiss len, 0; lahf; test ah, 0x44; jnp: divides unless `len == 0`
    let q = if len == 0.0 { len } else { div(s, len) };
    let a = mul(ndz, q);
    abs(g, 0x276, 0x280);
    g.write::<f32>(w(0x284), a);
    let b = mul(q, 0.0);
    abs(g, 0x28f, 0x2a3);
    g.write::<f32>(w(0x2a7), b);
    let c = mul(g.read::<f32>(base + 0xc), q);
    abs(g, 0x2b5, 0x2c9);
    g.write::<f32>(w(0x2cd), c);
    abs(g, 0x2d1, 0x303);
    // call width: 3
    g.call(slot(g, 0x269, 234), 0, &[w(0x30f), w(0x30a), w(0x272)]);
    let last = gw(g, 0x316);
    let fl = g.read::<u16>(w(0x31d)) | 1;
    g.write::<u16>(w(0x31d), fl);
    abs(g, 0x325, 0x374);
    after_old(g, img, f, last, (0x375, 0x37a, 0x380, 0x38f));
    let n = g.read::<u16>(w(0x39b)).wrapping_add(1);
    g.write::<u16>(w(0x39b), n);
    abs(g, 0x39f, 0x3b3);
    // call width: 2
    g.call(slot(g, 0x394, 325), 0, &[w(0x3bf), w(0x3ba)]);
    let last = gw(g, 0x3c6);
    abs(g, 0x3cd, 0x408);
    after_old(g, img, f, last, (0x409, 0x40e, 0x414, 0x423));
    let n = g.read::<u16>(w(0x429));
    abs(g, 0x434, 0x4ab);
    after_known(g, img, f, w(0x42e), (0x4ad, 0x4b3, 0x4bf));
    g.write::<u16>(w(0x4cd), n.wrapping_add(2));
    let x = gw(g, 0x4d2);
    abs(g, 0x4da, 0x513);
    let rec = base + 0x14;
    for (i, v) in (0u32..).zip([w(0x517), w(0x51f), w(0x527), x, w(0x533), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    abs(g, 0x53f, 0x549);
    crate::sb_nodes::particle_node(g, img, call(0x54a), rec);
    // the fade's words
    let km = |g: &mut G, x: u32, d: u32| k_minus(g, img, f, (0x552, x, d));
    km(g, 0x55d, 0x57a);
    let fl = (gw(g, 0x562) & !1) | 4;
    abs(g, 0x56c, 0x576);
    km(g, 0x585, 0x5b5);
    abs(g, 0x589, 0x5b1);
    km(g, 0x5c0, 0x5f0);
    abs(g, 0x5c4, 0x5ec);
    g.write::<u32>(w(0x600), fl);
    km(g, 0x5fb, 0x612);
    abs(g, 0x604, 0x60e);
    km(g, 0x61d, 0x625);
    km(g, 0x630, 0x638);
    let x = sub(k(g, 0x640), k(g, 0x648));
    g.write::<f32>(w(0x650), x);
    km(g, 0x65b, 0x663);
}

/// [`scene_task_0071`] (the `rel32`s: its callees in order, the root's
/// import, `__security_check_cookie`).
pub const SCENE_TASK_0071: Sig = Sig {
    len: 3369,
    hash: 0xf8ec_e604_bde0_fbd6,
    calls: &[
        (0x21b, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x54a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x6b0, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x6c1, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x73f, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x772, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x77c, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x79f, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0xa0b, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0xa58, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0xa79, Callee::Sig(&crate::m0071::STATIC_NODE)),
        (0xb75, Callee::Sig(&crate::nodes::INSERT)),
        (0xbc0, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xc2e, Callee::Sig(&crate::o4_ctor::VEC_CTOR_0071_7530)),
        (0xcf6, Callee::Any),
    ],
    imports: &[],
};

/// [`main_task_0070`]'s code offset and frame: `base` is `E - 0xc` (the
/// aligned frame `E` below the three pushes), `r` the record.
struct Ctx70 {
    f: u32,
    base: u32,
    r: u32,
}

impl Ctx70 {
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    fn gw<G: Game>(&self, g: &G, img: &Image, o: u32) -> u32 {
        g.read::<u32>(self.w(img, o))
    }

    fn k<G: Game>(&self, g: &G, img: &Image, o: u32) -> f32 {
        g.read::<f32>(self.w(img, o))
    }

    fn target(&self, img: &Image, o: u32) -> u32 {
        img.rel32_target(self.f + o)
    }

    fn slot<G: Game>(&self, g: &G, img: &Image, op: u32, n: u32) -> u32 {
        slot_target(g, self.w(img, op), n)
    }

    fn abs<G: Game>(&self, g: &mut G, img: &Image, a: u32, b: u32) {
        crate::r25::abs_stores(g, img, self.f, a, b);
    }

    fn at(&self, o: u32) -> u32 {
        self.r.wrapping_add(o)
    }

    /// The wait `R + 0x30`.
    fn wait<G: Game>(&self, g: &G) -> i32 {
        g.read::<i32>(self.at(0x30))
    }

    fn set<G: Game>(&self, g: &mut G, s: u32) {
        g.write::<u32>(self.at(0x18), s);
    }

    fn up<G: Game>(&self, g: &mut G) {
        let s = g.read::<u32>(self.at(0x18)).wrapping_add(1);
        self.set(g, s);
    }

    /// The summon's byte (`movzx`, operand `x`).
    fn byte<G: Game>(&self, g: &G, img: &Image, x: u32) -> u32 {
        u32::from(g.read::<u8>(self.w(img, x)))
    }

    /// Slot 228 `(v, n)` (imports operand `i`). Returns its `eax`.
    fn s228<G: Game>(&self, g: &mut G, img: &Image, i: u32, v: u32, n: u32) -> u32 {
        // call width: 2
        g.call(self.slot(g, img, i, 228), 0, &[v, n]).eax
    }

    /// Slot 411 on the summon's byte (operands `i`, `x`). Returns its
    /// answer.
    fn s411<G: Game>(&self, g: &mut G, img: &Image, i: u32, x: u32) -> u32 {
        let b = self.byte(g, img, x);
        // call width: 1
        g.call(self.slot(g, img, i, 411), 0, &[b]).eax
    }

    /// Slots 719 `(K, 5, 0)` and 306 `(7, b, h)`. Returns 306's `eax`.
    fn s719_306<G: Game>(&self, g: &mut G, img: &Image, (i, a, j): (u32, u32, u32), b: u32) -> u32 {
        // call width: 3
        let h = g
            .call(self.slot(g, img, i, 719), 0, &[self.w(img, a), 5, 0])
            .eax;
        // call width: 3
        g.call(self.slot(g, img, j, 306), 0, &[7, b, h]).eax
    }

    /// A sb append: the node (operand `n`) under the list (operand `p`).
    fn sb<G: Game>(&self, g: &mut G, img: &Image, (c, n, p): (u32, u32, u32)) -> u32 {
        crate::sb_misc::node_append(
            g,
            img,
            self.target(img, c),
            self.w(img, n),
            self.w(img, p),
            0,
        )
    }

    /// A sa append: the node (operand `n`) under the list (operand `p`).
    fn sa<G: Game>(&self, g: &mut G, img: &Image, (c, n, p): (u32, u32, u32)) -> u32 {
        crate::sa_misc::node_append(
            g,
            img,
            self.target(img, c),
            self.w(img, n),
            self.w(img, p),
            0,
        )
    }

    /// An owned node (operand `n`, the list operand `p`) with its four
    /// pushed words (the last pushed first) at `base - d`.
    fn owned<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, n, p): (u32, u32, u32),
        words: [u32; 4],
        d: u32,
    ) -> u32 {
        let sp = self.base.wrapping_sub(d);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let of = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: self.w(img, n),
            edx: self.w(img, p),
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of)
    }

    /// `push n; sub esp, 8; push a`: a particle node (operand `p`) with
    /// the words `d` below `base`.
    fn particle<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, p): (u32, u32),
        (a, n): (u32, u32),
        d: u32,
    ) -> u32 {
        let sp = self.base.wrapping_sub(d + 0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 0xc, n);
        let pf = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: self.w(img, p),
            ..Frame::default()
        };
        crate::s3_nodes::particle_node_with(g, img, &pf)
    }

    /// `push d; push ecx; push b; push a`: an effect node (operand `e`;
    /// `a`, `b` operands), then its flag word's bit 0 (operand `fl`) and
    /// the run `run`. Returns the node's `eax`.
    fn effect<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, e): (u32, u32),
        (a, b, d): (u32, u32, u32),
        fl: u32,
        run: (u32, u32),
    ) -> u32 {
        let sp = self.base.wrapping_sub(0x10);
        g.write::<u32>(sp, self.w(img, a));
        g.write::<u32>(sp + 4, self.w(img, b));
        g.write::<u32>(sp + 0xc, d);
        let ef = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: self.w(img, e),
            ..Frame::default()
        };
        let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
        let v = self.gw(g, img, fl) | 1;
        g.write::<u32>(self.w(img, fl), v);
        self.abs(g, img, run.0, run.1);
        eax
    }

    /// The node at operand `x` freed and unlinked through `base + o`.
    fn free<G: Game>(&self, g: &mut G, img: &Image, (c, x): (u32, u32), o: u32, eax: u32) -> u32 {
        let p = self.base + o;
        g.write::<u32>(p, self.w(img, x));
        let ff = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(4),
            ecx: p,
            eax,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff).eax
    }

    fn free_part<G: Game>(&self, g: &mut G, img: &Image, (c, p): (u32, u32), eax: u32) -> u32 {
        crate::m0070::free_part(g, img, self.target(img, c), self.w(img, p), eax)
    }

    /// A light (the call at `c`) with `xmm1` the float at operand `kx`
    /// after the run `run`.
    fn light_on<G: Game>(&self, g: &mut G, img: &Image, c: u32, kx: u32, run: (u32, u32)) -> u32 {
        let level = u128::from(self.gw(g, img, kx));
        self.abs(g, img, run.0, run.1);
        let lf = Frame {
            at: self.target(img, c),
            esp: self.base.wrapping_sub(4),
            xmm: [0, level, 0, 0],
            ..Frame::default()
        };
        crate::s4_small::light_on(g, img, &lf).eax
    }
}

/// `magic_0070`'s summon main task (0x1000b6e0, 6137 bytes with its
/// tables; a task callback `(A)`, cdecl; an aligned /GS frame `E` of 0x6c
/// bytes below three pushes; the record `R = A[4]`, its state `R + 0x18`,
/// counts `R + 0x2c` and `R + 0x30` (the wait) up at every exit but the
/// first). Nothing while the DLL's level word is below 4. By the state
/// (each past its wait):
/// - 0 into 0xa, 0xc, 0x1e, 0x20, 0x34, 0x28, 0x5c and the others: the
///   scene's nodes (appends, owned, particle and effect nodes, the
///   summon's model and its bones, colour nodes, the light, the moves),
///   slots 719/306/228/411/219/222/237/262/325/366/230/229/546/676/6/3;
/// - 0x48/0x49, 0x50, 0x68: the camera's words stepped;
/// - 10000..10002: the end (parts freed, nodes leave, the models set up,
///   the task unlinked; then 20000).
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn main_task_0070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; and esp, -8; sub esp, 0x6c; the cookie; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x6c);
    let cookie = g.read::<u32>(img.word(f + 0xa)) ^ e;
    g.write::<u32>(e + 0x68, cookie);
    if g.read::<i32>(img.word(f + 0x16)) < 4 {
        return Ret::eax(cookie);
    }
    let base = e.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(base + 0x10, r);
    let c = Ctx70 { f, base, r };
    let mut state = g.read::<u32>(c.at(0x18));
    // how a state ends: (counts only, state on first)
    let (eax, up) = loop {
        if state > 0x2710 {
            match state {
                0x2711 => {
                    c.abs(g, img, 0x16d4, 0x16de);
                    break (0, true);
                }
                0x2712 => break (end_0070(g, img, &c, task), false),
                _ => break (state.wrapping_sub(0x2712), false),
            }
        }
        if state == 0x2710 {
            c.up(g);
            g.write::<u32>(c.at(0x30), 0);
            c.abs(g, img, 0x16d4, 0x16de);
            break (0x2710, true);
        }
        if state > 0x68 {
            break (state, false);
        }
        let idx = img.word(c.w(img, 0x4a) + state) & 0xff;
        let wait = |g: &G, n: i32| c.wait(g) < n;
        match state {
            0x68 => {
                let x = add(g.read::<f32>(c.at(0x24)), c.k(g, img, 0x5e));
                g.write::<f32>(c.w(img, 0x66), x);
                let y = sub(c.k(g, img, 0x6e), g.read::<f32>(c.at(0x20)));
                g.write::<f32>(c.w(img, 0x7b), y);
                if wait(g, 0x1e) {
                    break (idx, false);
                }
                c.set(g, 0x5a);
                state = 0x5a;
            }
            0x60 => {
                if wait(g, 0x7d) {
                    break (idx, false);
                }
                c.set(g, 0x2710);
                state = 0x2710;
            }
            0 | 0xa => {
                let mut eax = idx;
                if state == 0 {
                    eax = main_start_0070(g, img, &c);
                    c.set(g, 0xa);
                }
                main_fade_0070(g, img, &c);
                break (eax, true);
            }
            0xc => {
                if c.s411(g, img, 0x351, 0x34b) == 0 {
                    break (0, false);
                }
                break (show_0070(g, img, &c), false);
            }
            0x1e => {
                if wait(g, 0x50) {
                    break (idx, false);
                }
                break (camera_0070(g, img, &c), true);
            }
            0x64 => {
                g.write::<u32>(c.at(0x30), 0);
                break (idx, true);
            }
            0x1f => {
                if wait(g, 0x50) {
                    break (idx, false);
                }
                let b = c.byte(g, img, 0x729);
                break (c.s228(g, img, 0x72f, b, 0xda), true);
            }
            0x20 => {
                if wait(g, 0x5a) {
                    break (idx, false);
                }
                // push ecx (unread); push the value: the static node
                g.write::<u32>(base.wrapping_sub(8), c.w(img, 0x74f));
                let sf = Frame {
                    at: c.target(img, 0x759),
                    esp: base.wrapping_sub(0xc),
                    ecx: c.w(img, 0x754),
                    ..Frame::default()
                };
                let eax = crate::s4_misc::static_node(g, img, &sf).eax;
                let x = u128::from(c.gw(g, img, 0x761));
                c.abs(g, img, 0x76b, 0x7cf);
                let mf = Frame {
                    at: c.target(img, 0x7d0),
                    esp: base.wrapping_sub(4),
                    ecx: c.w(img, 0x754),
                    eax,
                    xmm: [0, 0, x, x],
                    ..Frame::default()
                };
                let eax = crate::s4_small::move_setup(g, img, &mf).eax;
                c.set(g, 0x3c);
                break (eax, false);
            }
            0x3c | 0x3d => {
                if state == 0x3c && !wait(g, 0x75) {
                    crate::m0070::node_init_root(g, img, c.target(img, 0x7ec), c.w(img, 0x7e7));
                    c.light_on(g, img, 0x849, 0x7f4, (0x7f8, 0x848));
                    c.up(g);
                    g.write::<u32>(c.at(0x30), 0);
                }
                if c.s411(g, img, 0x860, 0x85a) == 0 {
                    break (0, false);
                }
                let b = c.byte(g, img, 0x729);
                break (c.s228(g, img, 0x72f, b, 0xdb), true);
            }
            0x3e => {
                if wait(g, 8) {
                    break (idx, false);
                }
                break (lights_0070(g, img, &c), false);
            }
            0x32 => {
                if wait(g, 0x14) {
                    break (idx, false);
                }
                c.abs(g, img, 0xa1f, 0xa29);
                break (idx, true);
            }
            0x33 => {
                if wait(g, 0x1e) {
                    break (idx, false);
                }
                // call width: 4
                g.call(
                    c.slot(g, img, 0xa39, 222),
                    0,
                    &[c.w(img, 0xa4a), 0xa, 0xb, 2],
                );
                let b = c.byte(g, img, 0xa53);
                let eax = c.s228(g, img, 0xa5e, b, 0xdc);
                g.write::<u32>(c.at(0x30), 0);
                break (eax, true);
            }
            0x34 => {
                if wait(g, 0x21) {
                    break (idx, false);
                }
                break (main_nodes_0070(g, img, &c), false);
            }
            0x28 => {
                // call width: 1
                let a = g.call(c.slot(g, img, 0xd92, 313), 0, &[2]).eax;
                // comiss G, K; jb: waits (unordered too)
                if a != 0 || !(c.k(g, img, 0xdaf) >= c.k(g, img, 0xdb6)) {
                    break (a, false);
                }
                break (bones_0070(g, img, &c, a), true);
            }
            0x29 => {
                if c.s411(g, img, 0xef9, 0xef3) == 0 {
                    break (0, false);
                }
                let b = c.byte(g, img, 0xf13);
                let eax = c.s228(g, img, 0xf1e, b, 0xdd);
                g.write::<u32>(c.at(0x30), 0);
                c.set(g, 0x46);
                break (eax, false);
            }
            0x46 => {
                if wait(g, 0x28) {
                    break (idx, false);
                }
                let m = c.gw(g, img, 0xf51);
                c.abs(g, img, 0xf55, 0xf5f);
                // call width: 1
                g.call(c.slot(g, img, 0xf4b, 546), 0, &[m]);
                c.s719_306(g, img, (0xf68, 0xf7a, 0xf85), 1);
                let eax = c.effect(
                    g,
                    img,
                    (0xfab, 0xfa6),
                    (0xfa1, 0xf9c, 2),
                    0xfb4,
                    (0xfb9, 0xfc3),
                );
                break (eax, true);
            }
            0x47 => {
                if wait(g, 0x36) {
                    break (idx, false);
                }
                crate::m0070::hide_model(g, img, c.target(img, 0xfd8), c.w(img, 0xfd3));
                let m = c.gw(g, img, 0xfe5);
                // call width: 2
                g.call(c.slot(g, img, 0xfdd, 6), 0, &[m, 0]);
                c.sb(g, img, (0xff9, 0xff4, 0xfef));
                c.abs(g, img, 0x1012, 0x103a);
                let words = [
                    c.w(img, 0x1009),
                    c.w(img, 0x1004),
                    c.w(img, 0xfff),
                    c.w(img, 0xff4),
                ];
                // slot 6's two words stay under the owned node's four
                c.owned(g, img, (0x103b, 0x100e, 0xfef), words, 0x18);
                c.abs(g, img, 0x1042, 0x1049);
                let eax = c.particle(g, img, (0x1055, 0x1050), (c.w(img, 0x100e), 9), 0);
                g.write::<u32>(c.at(0x20), ONE);
                g.write::<u32>(c.at(0x24), 0x3c23_d70a);
                break (eax, true);
            }
            0x48 | 0x49 => {
                if state == 0x48 {
                    if wait(g, 0x70) {
                        // (on into 0x49's code without the step)
                    } else {
                        c.up(g);
                    }
                }
                let s = mul(g.read::<f32>(c.at(0x20)), c.k(g, img, 0x1081));
                g.write::<f32>(c.at(0x20), s);
                let x = sub(c.k(g, img, 0x108e), s);
                g.write::<f32>(c.w(img, 0x109a), x);
                if wait(g, 0x74) {
                    break (idx, false);
                }
                g.write::<u32>(c.at(0x30), 0);
                c.set(g, 0x14);
                break (idx, false);
            }
            0x14 => {
                let eax =
                    crate::m0042::set_value(g, img, c.target(img, 0x10c6), c.w(img, 0x10c1), 0xde);
                c.abs(g, img, 0x10ca, 0x10d4);
                g.write::<u32>(c.at(0x30), 0);
                break (eax, true);
            }
            0x15 => {
                if wait(g, 2) {
                    break (idx, false);
                }
                for x in [0x10ecu32, 0x10f3] {
                    let v = c.gw(g, img, x) | 2;
                    g.write::<u32>(c.w(img, x), v);
                }
                break (idx, true);
            }
            0x16 => {
                if wait(g, 0x59) {
                    break (idx, false);
                }
                c.abs(g, img, 0x1107, 0x1111);
                c.set(g, 0x64);
                break (idx, false);
            }
            0x65 => {
                if wait(g, 1) {
                    break (idx, false);
                }
                let b = c.byte(g, img, 0x112a);
                c.abs(g, img, 0x1139, 0x1157);
                c.s228(g, img, 0x1135, b, 0xdf);
                for x in [0x1166u32, 0x116d] {
                    let v = c.gw(g, img, x) & !2;
                    g.write::<u32>(c.w(img, x), v);
                }
                let m = c.gw(g, img, 0x117f);
                let eax = c.s228(g, img, 0x1160, m, 0xb1);
                break (eax, true);
            }
            0x66 => {
                if wait(g, 0xc) {
                    break (idx, false);
                }
                break (statics_0070(g, img, &c), true);
            }
            0x67 => {
                if wait(g, 0xc) {
                    break (idx, false);
                }
                g.write::<u32>(c.at(0x20), 0x4000_0000);
                g.write::<u32>(c.at(0x24), 0xbf00_0000);
                g.write::<u32>(c.at(0x30), 0);
                break (idx, true);
            }
            0x50 => {
                let eax =
                    crate::m0070::model_position(g, img, c.target(img, 0x12a4), c.w(img, 0x129f));
                let x = sub(c.k(g, img, 0x12ac), c.k(g, img, 0x12b4));
                c.abs(g, img, 0x12b8, 0x12cc);
                g.write::<f32>(c.w(img, 0x12d0), x);
                let y = add(c.k(g, img, 0x12d8), c.k(g, img, 0x12e0));
                g.write::<f32>(c.w(img, 0x12e8), y);
                g.write::<u32>(c.at(0x30), 0);
                break (eax, true);
            }
            0x51 => {
                let x = sub(c.k(g, img, 0x12fc), c.k(g, img, 0x130b));
                g.write::<f32>(c.w(img, 0x1319), x);
                let a = c.s411(g, img, 0x1311, 0x1303);
                if a != 0 {
                    c.set(g, 0x5a);
                }
                break (a, false);
            }
            0x5a => {
                c.abs(g, img, 0x1343, 0x134d);
                let eax = c.s719_306(g, img, (0x133d, 0x1356, 0x1361), 2);
                break (eax, true);
            }
            0x5b => break (frees_0070(g, img, &c), true),
            0x5c => {
                if wait(g, 0x20) {
                    break (idx, false);
                }
                break (ring_0070(g, img, &c), true);
            }
            0x5d => {
                if wait(g, 0x35) {
                    break (idx, false);
                }
                let eax = c.effect(
                    g,
                    img,
                    (0x15a5, 0x15a0),
                    (0x159b, 0x1596, 5),
                    0x15ae,
                    (0x15b3, 0x15bd),
                );
                break (eax, true);
            }
            0x5e => {
                if wait(g, 0x3c) {
                    break (idx, false);
                }
                c.abs(g, img, 0x15db, 0x1617);
                c.particle(g, img, (0x1618, 0x15d2), (c.w(img, 0x15d7), 8), 0);
                let eax = c.effect(
                    g,
                    img,
                    (0x1632, 0x162d),
                    (0x1628, 0x1623, 4),
                    0x163b,
                    (0x1640, 0x164a),
                );
                break (eax, true);
            }
            0x5f => {
                if wait(g, 0x64) {
                    break (idx, false);
                }
                c.abs(g, img, 0x165d, 0x16a3);
                let args = base + 0x10;
                for (i, v) in (0u32..).zip([c.w(img, 0x16a7), 0, 0x3d4c_cccd, ONE]) {
                    g.write::<u32>(args + 4 * i, v);
                }
                break (crate::sa_misc::move_start(g, args), true);
            }
            0xb => break (idx, true),
            _ => break (idx, false),
        }
    };
    if up {
        c.up(g);
    }
    for o in [0x2cu32, 0x30] {
        let v = g.read::<u32>(c.at(o)).wrapping_add(1);
        g.write::<u32>(c.at(o), v);
    }
    Ret::eax(eax)
}

/// [`main_task_0070`]'s state 0: slots 719 and 306, the small node, an
/// effect node (its words, flags), a colour node. Returns its `eax`.
fn main_start_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    c.s719_306(g, img, (0x180, 0x18f, 0x19a), 0);
    g.write::<u32>(c.base.wrapping_sub(4), w(0x1b3));
    crate::m0070::node_init_small(g, img, c.target(img, 0x1bd), w(0x1b8), w(0x1ae), w(0x1b3));
    c.abs(g, img, 0x1c4, 0x1ce);
    c.abs(g, img, 0x1e0, 0x1f4);
    let sp = c.base.wrapping_sub(0x10);
    g.write::<u32>(sp, w(0x1d7));
    g.write::<u32>(sp + 4, w(0x1d2));
    g.write::<u32>(sp + 0xc, 0);
    let ef = Frame {
        at: c.target(img, 0x1f5),
        esp: sp.wrapping_sub(4),
        ecx: w(0x1dc),
        ..Frame::default()
    };
    crate::s3_nodes::effect_node_with(g, img, &ef);
    let fl = c.gw(g, img, 0x1fa) | 7;
    c.abs(g, img, 0x20b, 0x215);
    g.write::<u32>(w(0x216), fl);
    let cg = c.base + 0x10;
    g.write::<u32>(cg, w(0x21e));
    g.write::<u32>(cg + 4, w(0x226));
    c.abs(g, img, 0x22a, 0x234);
    let cf = Frame {
        at: c.target(img, 0x235),
        esp: c.base.wrapping_sub(4),
        ecx: cg,
        ..Frame::default()
    };
    crate::s3_misc::colour_node(g, img, &cf)
}

/// [`main_task_0070`]'s state 0xa: the fade's words (`K - G` each, the
/// flag's bit 0 cleared).
fn main_fade_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) {
    let km = |g: &mut G, x: u32, d: u32| k_minus(g, img, c.f, (0x244, x, d));
    km(g, 0x24f, 0x27c);
    let v = c.gw(g, img, 0x255) & !1;
    g.write::<u32>(c.w(img, 0x255), v);
    c.abs(g, img, 0x25a, 0x278);
    km(g, 0x287, 0x2b7);
    c.abs(g, img, 0x28b, 0x2b3);
    km(g, 0x2c2, 0x2de);
    c.abs(g, img, 0x2c6, 0x2da);
    km(g, 0x2e9, 0x2f1);
    km(g, 0x2fc, 0x304);
    km(g, 0x30f, 0x31f);
    km(g, 0x317, 0x337);
    let x = sub(c.k(g, img, 0x327), c.k(g, img, 0x32f));
    g.write::<f32>(c.w(img, 0x33f), x);
}

/// [`main_task_0070`]'s state 0xc once slot 411 answers: slot 228, the
/// camera's nodes and words, an owned node, the summon's model node,
/// slot 211, its show, slot 6, the value, the model state, the fade's
/// node, slot 366 unless the flag, slots 230, 325 and 229 on the frame's
/// vectors, the camera's place, a node, an owned and a particle node;
/// the wait reset, state 0x1e. Returns the particle node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn show_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let b = c.byte(g, img, 0x36b);
    // (slot 228's two words stay to the model node)
    g.write::<u32>(base.wrapping_sub(4), 0xd9);
    g.write::<u32>(base.wrapping_sub(8), b);
    c.s228(g, img, 0x376, b, 0xd9);
    c.sb(g, img, (0x38d, 0x388, 0x383));
    let v = c.gw(g, img, 0x395);
    g.write::<u32>(w(0x39d), v);
    let x = add(c.k(g, img, 0x3a5), c.k(g, img, 0x3ad));
    c.abs(g, img, 0x3b6, 0x3c0);
    g.write::<f32>(w(0x3c4), x);
    let y = add(c.k(g, img, 0x3cc), c.k(g, img, 0x3d4));
    g.write::<f32>(w(0x3dc), y);
    c.sa(g, img, (0x3e1, 0x3b2, 0x383));
    c.abs(g, img, 0x3ea, 0x408);
    c.sb(g, img, (0x409, 0x3e6, 0x383));
    let words = [w(0x419), w(0x414), w(0x40f), w(0x3e6)];
    c.owned(g, img, (0x423, 0x41e, 0x383), words, 0x18);
    // the model node `(*G)` over slot 228's first word
    g.write::<u32>(base.wrapping_sub(8), c.gw(g, img, 0x431));
    let mf = Frame {
        at: c.target(img, 0x436),
        esp: base.wrapping_sub(0xc),
        ecx: w(0x42b),
        edx: w(0x383),
        ..Frame::default()
    };
    crate::s4_misc::model_node_with(g, img, &mf);
    let af = Frame {
        at: c.target(img, 0x445),
        esp: base.wrapping_sub(0xc),
        ecx: w(0x440),
        edx: w(0x43b),
        ..Frame::default()
    };
    crate::s4_small::attach_211(g, img, &af);
    let cur = base + 0x18;
    g.write::<u32>(cur, w(0x451));
    g.write::<u8>(cur + 4, 0xff);
    crate::m0070::show(g, img, c.target(img, 0x45b), cur);
    let m = c.gw(g, img, 0x468);
    // call width: 2
    g.call(c.slot(g, img, 0x460, 6), 0, &[m, 1]);
    crate::m0042::set_value(g, img, c.target(img, 0x47f), w(0x47a), 0xb0);
    let rec = base + 0x10;
    g.write::<u32>(rec, w(0x48b));
    g.write::<u32>(rec + 4, w(0x493));
    crate::m0070::model_state(g, img, c.target(img, 0x498), rec);
    c.sb(g, img, (0x4a7, 0x4a2, 0x49d));
    if g.read::<u8>(w(0x4ad)) & 1 == 0 {
        let m = c.gw(g, img, 0x4c0);
        // call width: 2
        g.call(c.slot(g, img, 0x4b5, 366), 0, &[m, w(0x4ba)]);
    }
    let x = sub(c.k(g, img, 0x4d3), c.k(g, img, 0x4db));
    g.write::<f32>(w(0x4f2), x);
    c.abs(g, img, 0x4f6, 0x500);
    // the vectors `A = E + 0x14` (`base + 0x20`, `(-0.5, 0, 0, 1)`), `B =
    // E + 0x24`, `C = E + 0x54`
    let va = base + 0x20;
    let vb = base + 0x30;
    for (i, v) in (0u32..).zip([0xbf00_0000u32, 0, 0, ONE]) {
        g.write::<u32>(va + 4 * i, v);
    }
    // call width: 2
    g.call(c.slot(g, img, 0x4ea, 230), 0, &[vb, w(0x4e4)]);
    // call width: 2 (slot 230's words stay under it)
    g.call(c.slot(g, img, 0x533, 325), 0, &[base + 0x60, w(0x52d)]);
    // call width: 3
    g.call(c.slot(g, img, 0x54f, 229), 0, &[va, vb, va]);
    let x = add(g.read::<f32>(va), c.k(g, img, 0x565));
    g.write::<f32>(w(0x57a), x);
    let y = add(g.read::<f32>(va + 8), c.k(g, img, 0x588));
    g.write::<f32>(w(0x590), y);
    c.sb(g, img, (0x595, 0x572, 0x56d));
    c.abs(g, img, 0x5ae, 0x5d6);
    let words = [w(0x5a5), w(0x5a0), w(0x59b), w(0x572)];
    c.owned(g, img, (0x5d7, 0x5aa, 0x56d), words, 0x10);
    c.abs(g, img, 0x5de, 0x5e5);
    let eax = c.particle(g, img, (0x5f1, 0x5ec), (w(0x5aa), 0), 0);
    g.write::<u32>(c.at(0x30), 0);
    c.set(g, 0x1e);
    eax
}

/// [`main_task_0070`]'s state 0x1e past its wait: a node, slot 262's
/// place into three words, a second node (`K` below), an owned node, a
/// particle and an effect node; the wait reset. Returns the effect node's
/// `eax`.
fn camera_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    c.abs(g, img, 0x61f, 0x629);
    c.sb(g, img, (0x62a, 0x61b, 0x616));
    let m = c.gw(g, img, 0x64a);
    // call width: 4
    g.call(
        c.slot(g, img, 0x62f, 262),
        0,
        &[m, w(0x644), w(0x63f), w(0x634)],
    );
    let x = sub(c.k(g, img, 0x654), c.k(g, img, 0x65c));
    c.abs(g, img, 0x66a, 0x674);
    g.write::<f32>(w(0x678), x);
    c.sb(g, img, (0x67d, 0x666, 0x661));
    c.abs(g, img, 0x696, 0x6be);
    let words = [w(0x68d), w(0x688), w(0x683), w(0x666)];
    // slot 262's four words stay under the owned node's four
    c.owned(g, img, (0x6bf, 0x692, 0x661), words, 0x20);
    c.abs(g, img, 0x6c6, 0x6cd);
    c.particle(g, img, (0x6d9, 0x6d4), (w(0x692), 1), 0);
    let eax = c.effect(
        g,
        img,
        (0x6f3, 0x6ee),
        (0x6e9, 0x6e4, 1),
        0x6fc,
        (0x701, 0x70b),
    );
    g.write::<u32>(c.at(0x30), 0);
    eax
}

/// [`main_task_0070`]'s state 0x3e past its wait: the light's level and
/// its light, the flag, two nodes, an owned and a particle node, slot
/// 219's answer as the static child, the node freed, its part freed, a
/// particle node; the wait reset, state 0x32. Returns the particle node's
/// `eax`.
fn lights_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    c.light_on(g, img, 0x8ee, 0x88f, (0x893, 0x8ed));
    let v = c.gw(g, img, 0x8f4) & !1;
    g.write::<u32>(w(0x8f4), v);
    c.sb(g, img, (0x904, 0x8ff, 0x8fa));
    c.abs(g, img, 0x90d, 0x935);
    c.sb(g, img, (0x936, 0x909, 0x8fa));
    c.abs(g, img, 0x94f, 0x977);
    let words = [w(0x946), w(0x941), w(0x93c), w(0x909)];
    c.owned(g, img, (0x978, 0x94b, 0x8fa), words, 0x10);
    c.abs(g, img, 0x97f, 0x986);
    c.particle(g, img, (0x992, 0x98d), (w(0x94b), 0xa), 0);
    let b = c.byte(g, img, 0x999);
    // call width: 2 (one of the particle node's words stays under it)
    let h = g.call(c.slot(g, img, 0x9a4, 219), 0, &[b, 0]).eax;
    crate::m0070::static_child(g, img, c.target(img, 0x9b5), h);
    let eax = c.free(g, img, (0x9c9, 0x9c4), 0xc, h);
    let eax = c.free_part(g, img, (0x9d3, 0x9ce), eax);
    let _ = eax;
    c.abs(g, img, 0x9e6, 0x9fa);
    let eax = c.particle(g, img, (0x9fb, 0x9dd), (w(0x9e2), 2), 0);
    g.write::<u32>(c.at(0x30), 0);
    c.set(g, 0x32);
    eax
}

/// [`main_task_0070`]'s state 0x34 past its wait: slot 325, nodes and the
/// camera's words, owned and particle nodes, the scaled node, slot 325,
/// the static link (slot 845), nodes, an owned and a particle node, the
/// big node, slot 219 into a word, the root child, slot 676, the colour
/// node and its flags; state 0x28. Returns the colour node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn main_nodes_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    // call width: 2
    g.call(c.slot(g, img, 0xa84, 325), 0, &[w(0xa94), w(0xa89)]);
    c.sb(g, img, (0xaa5, 0xaa0, 0xa9b));
    c.abs(g, img, 0xaae, 0xad6);
    c.sa(g, img, (0xad7, 0xaaa, 0xa9b));
    let v = c.gw(g, img, 0xadc);
    g.write::<u32>(w(0xae1), v);
    let v = c.gw(g, img, 0xae6).wrapping_add(0x8000);
    g.write::<u32>(w(0xaf6), v);
    let v = c.gw(g, img, 0xafb);
    g.write::<u32>(w(0xb0f), v);
    // slot 325's two words stay under the owned node's four
    let words = [w(0xb05), w(0xb00), w(0xaaa), w(0xaeb)];
    c.owned(g, img, (0xb14, 0xb0a, 0xa9b), words, 0x18);
    c.abs(g, img, 0xb1b, 0xb22);
    // (its four words stay under the next owned node's)
    c.particle(g, img, (0xb2e, 0xb29), (w(0xb0a), 6), 0);
    c.sb(g, img, (0xb3d, 0xb38, 0xb33));
    c.abs(g, img, 0xb56, 0xb7e);
    let words = [w(0xb4d), w(0xb48), w(0xb43), w(0xb38)];
    c.owned(g, img, (0xb7f, 0xb52, 0xb33), words, 0x20);
    c.abs(g, img, 0xb86, 0xb8d);
    c.particle(g, img, (0xb99, 0xb94), (w(0xb52), 3), 0);
    // the scaled node `[N, P, a]` at `E + 0x4`
    let rec = base + 0x10;
    for (i, o) in (0u32..).zip([0xba5u32, 0xbad, 0xbb5]) {
        g.write::<u32>(rec + 4 * i, w(o));
    }
    c.abs(g, img, 0xbb9, 0xbc3);
    crate::m0070::node_init_scaled(g, rec);
    // call width: 2
    g.call(c.slot(g, img, 0xbc9, 325), 0, &[w(0xbd9), w(0xbce)]);
    c.abs(g, img, 0xbe2, 0xbf6);
    let lf = Frame {
        at: c.target(img, 0xbf7),
        esp: base.wrapping_sub(8),
        ..Frame::default()
    };
    let r = crate::s4_small::static_link_845(g, img, &lf);
    if let Some(t) = r.jump {
        // call width: 0 (the tail jump to slot 845)
        g.call(t, 0, &[]);
    }
    c.sb(g, img, (0xc09, 0xc04, 0xbff));
    c.abs(g, img, 0xc12, 0xc3a);
    c.sa(g, img, (0xc3b, 0xc0e, 0xbff));
    c.abs(g, img, 0xc3f, 0xc53);
    c.abs(g, img, 0xc58, 0xc62);
    c.sb(g, img, (0xc63, 0xc54, 0xbff));
    c.abs(g, img, 0xc7c, 0xca4);
    let words = [w(0xc73), w(0xc6e), w(0xc69), w(0xc54)];
    c.owned(g, img, (0xca5, 0xc78, 0xbff), words, 0x10);
    c.abs(g, img, 0xcac, 0xcb3);
    c.particle(g, img, (0xcbf, 0xcba), (w(0xc78), 7), 0);
    // the big node `(x)` (two of the particle node's words under it)
    g.write::<u32>(base.wrapping_sub(0xc), w(0xccc));
    c.abs(g, img, 0xcd0, 0xce4);
    let bf = Frame {
        at: c.target(img, 0xce5),
        esp: base.wrapping_sub(0x10),
        ecx: w(0xcc7),
        ..Frame::default()
    };
    crate::s4_small::big_node(g, img, &bf);
    let b = c.byte(g, img, 0xcec);
    // call width: 2
    let h = g.call(c.slot(g, img, 0xcf7, 219), 0, &[b, 0]).eax;
    g.write::<u32>(w(0xd10), h);
    c.abs(g, img, 0xd14, 0xd1e);
    crate::m0070::root_child(g, img, c.target(img, 0xd1f), w(0xd0b), w(0xd06), 0);
    c.abs(g, img, 0xd2d, 0xd4b);
    // call width: 3
    g.call(c.slot(g, img, 0xd24, 676), 0, &[w(0xd54), 2, 0x80]);
    // push a; sub esp, 8 (two words under them): the colour node `(_, _, a)`
    let sp = base.wrapping_sub(0x14);
    g.write::<u32>(sp + 8, w(0xd5e));
    let cf = Frame {
        at: c.target(img, 0xd66),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    let eax = crate::o4_ctor::colour_ctor(g, img, &cf).eax;
    for (x, bit) in [(0xd6cu32, 0x18u8), (0xd76, 1)] {
        let v = g.read::<u8>(w(x)) | bit;
        g.write::<u8>(w(x), v);
    }
    c.abs(g, img, 0xd7b, 0xd85);
    c.set(g, 0x28);
    eax
}

/// [`main_task_0070`]'s state 0x28 past slot 313 and its float: the node
/// freed, its part freed, the big node, two small nodes, the root child
/// (its word kept), the bones' words, the bones, a particle node. Returns
/// the particle node's `eax`.
fn bones_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70, eax: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let eax = c.free(g, img, (0xdcd, 0xdc8), 0xc, eax);
    c.free_part(g, img, (0xdd7, 0xdd2), eax);
    // sub esp, 8; push x: the big node
    g.write::<u32>(base.wrapping_sub(0xc), w(0xde4));
    let bf = Frame {
        at: c.target(img, 0xde9),
        esp: base.wrapping_sub(0x10),
        ecx: w(0xddf),
        ..Frame::default()
    };
    crate::s4_small::big_node(g, img, &bf);
    let x = c.gw(g, img, 0xdee);
    g.write::<u32>(w(0xe05), x);
    c.abs(g, img, 0xe09, 0xe13);
    let parent = w(0xdf6);
    g.write::<u32>(base.wrapping_sub(4), w(0xdfb));
    crate::m0070::node_init_small(g, img, c.target(img, 0xe14), w(0xe00), parent, w(0xdfb));
    g.write::<u32>(base.wrapping_sub(4), w(0xe21));
    c.abs(g, img, 0xe25, 0xe43);
    crate::m0070::node_init_small(g, img, c.target(img, 0xe44), w(0xe1c), parent, w(0xe21));
    c.abs(g, img, 0xe4b, 0xe55);
    c.abs(g, img, 0xe60, 0xe74);
    // `(a, b)`: `a` the small node as its set-up leaves `ecx`
    crate::m0070::root_child(g, img, c.target(img, 0xe75), w(0xe5c), w(0xe1c), w(0xe56));
    let p = c.gw(g, img, 0xe7a);
    g.write::<u32>(w(0xe80), w(0xe5c));
    c.abs(g, img, 0xe84, 0xec0);
    g.write::<u32>(p.wrapping_add(0x28), 0x3e01_0625);
    let rf = Frame {
        at: c.target(img, 0xed0),
        esp: base.wrapping_sub(4),
        ecx: w(0xec4),
        ..Frame::default()
    };
    crate::s4_small::bone_rate(g, img, &rf);
    c.particle(g, img, (0xee4, 0xeda), (w(0xedf), 4), 0)
}

/// [`main_task_0070`]'s state 0x66 past its wait: the static node from
/// `(a, b)`, slot 719's two handles, the node's start, an effect node.
/// Returns the effect node's `eax`.
fn statics_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    // push b; push a; sub esp, 0xc: `(-, -, -, a, b)`
    g.write::<u32>(base.wrapping_sub(4), w(0x1198));
    g.write::<u32>(base.wrapping_sub(8), w(0x119d));
    let sf = Frame {
        at: c.target(img, 0x11a5),
        esp: base.wrapping_sub(0x18),
        ..Frame::default()
    };
    crate::s4_small::static_node_args(g, img, &sf);
    for (i, a, s) in [(0x11aau32, 0x11bcu32, 0x11c6u32), (0x11cb, 0x11da, 0x11f1)] {
        // call width: 3
        let h = g.call(c.slot(g, img, i, 719), 0, &[w(a), 9, 0]).eax;
        g.write::<u32>(w(s), h);
    }
    let rate = u128::from(c.gw(g, img, 0x11e4));
    c.abs(g, img, 0x11f5, 0x1245);
    let nf = Frame {
        at: c.target(img, 0x1246),
        esp: base.wrapping_sub(4),
        ecx: w(0x11ec),
        xmm: [0, rate, 0, 0],
        ..Frame::default()
    };
    crate::sa_misc::node_start(g, img, &nf);
    c.effect(
        g,
        img,
        (0x125d, 0x1258),
        (0x1253, 0x124e, 3),
        0x1266,
        (0x126b, 0x1275),
    )
}

/// [`main_task_0070`]'s state 0x5b: three nodes freed with their parts,
/// a node leaves and is registered, slot 228, the release, slot 3, the
/// camera's words, slot 228 again, the release; the wait reset. Returns
/// the last release's `eax`.
fn frees_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let mut eax = 0;
    for (cf, x, cp, p) in [
        (0x1386u32, 0x1381u32, 0x1390u32, 0x138bu32),
        (0x13a1, 0x139c, 0x13ab, 0x13a6),
        (0x13bc, 0x13b7, 0x13c6, 0x13c1),
    ] {
        eax = c.free(g, img, (cf, x), 0xc, eax);
        eax = c.free_part(g, img, (cp, p), eax);
    }
    let _ = eax;
    let cur = base + 0xc;
    g.write::<u32>(cur, w(0x13d2));
    let lf = Frame {
        at: c.target(img, 0x13d7),
        esp: base.wrapping_sub(4),
        ecx: cur,
        ..Frame::default()
    };
    crate::s4_small::leave_register(g, img, &lf);
    let b = c.byte(g, img, 0x13de);
    c.abs(g, img, 0x13ed, 0x140b);
    c.s228(g, img, 0x13e9, b, 0xdf);
    let h = c.gw(g, img, 0x1415);
    crate::m0042::release(g, img, c.target(img, 0x141a), h);
    let m = c.gw(g, img, 0x1425);
    // call width: 1
    g.call(c.slot(g, img, 0x141f, 3), 0, &[m]);
    c.abs(g, img, 0x1436, 0x1440);
    let m = c.gw(g, img, 0x1447);
    c.abs(g, img, 0x144b, 0x1473);
    c.s228(g, img, 0x142f, m, 0xb3);
    let h = c.gw(g, img, 0x147d);
    let eax = crate::m0042::release(g, img, c.target(img, 0x1485), h);
    g.write::<u32>(c.at(0x30), 0);
    eax
}

/// [`main_task_0070`]'s state 0x5c past its wait: a node and its words,
/// then for each of the bones (`0x2c` apart from `+0x14e4`, owners `0x14c`
/// apart from `+0x14e9`, parts `0x100` apart from `+0x14d7`) a node, slot
/// 237 on the next word of the table at `+0x14df`, an owned node and a
/// particle node; the wait reset. Returns the last particle node's `eax`.
fn ring_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    c.sb(g, img, (0x14aa, 0x14a5, 0x14a0));
    c.abs(g, img, 0x14ae, 0x14d6);
    let (mut part, mut node, mut owner) = (w(0x14d7), w(0x14e4), w(0x14e9));
    let mut cur = w(0x14df);
    let list = w(0x14f1);
    let mut eax;
    loop {
        crate::sb_misc::node_append(g, img, c.target(img, 0x14f8), node, list, 0);
        let v = g.read::<u32>(cur);
        let m = c.gw(g, img, 0x150f);
        // call width: 4
        g.call(c.slot(g, img, 0x1509, 237), 0, &[m, v, 0, node + 0x1c]);
        g.write::<u32>(node + 0x28, ONE);
        // slot 237's four words stay under the owned node's four
        let sp = base.wrapping_sub(0x20);
        for (i, x) in (0u32..).zip([w(0x1527), node, w(0x1521), w(0x151c)]) {
            g.write::<u32>(sp + 4 * i, x);
        }
        let of = Frame {
            at: c.target(img, 0x153a),
            esp: sp.wrapping_sub(4),
            ecx: owner,
            edx: w(0x152c),
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of);
        g.write::<u8>(owner + 0x148, 1);
        let sp = base.wrapping_sub(0x10);
        g.write::<u32>(sp, owner);
        g.write::<u32>(sp + 0xc, 5);
        let pf = Frame {
            at: c.target(img, 0x1551),
            esp: sp.wrapping_sub(4),
            ecx: part,
            ..Frame::default()
        };
        eax = crate::s3_nodes::particle_node_with(g, img, &pf);
        cur = cur.wrapping_add(4);
        g.write::<u32>(base + 0xc, cur);
        owner = owner.wrapping_add(0x14c);
        node = node.wrapping_add(0x2c);
        part = part.wrapping_add(0x100);
        if owner.cast_signed() >= w(0x156e).cast_signed() {
            break;
        }
    }
    g.write::<u32>(c.at(0x30), 0);
    eax
}

/// [`main_task_0070`]'s end (10002): a node freed and its part, the
/// static leave (slot 846), a node leaves, the model's parts (slots 531,
/// 255), the model leaves and is set up, the second model likewise; the
/// task unlinked, state 20000. Returns the unlink's `eax`.
fn end_0070<G: Game>(g: &mut G, img: &Image, c: &Ctx70, task: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    // (`eax` the dispatch's 0)
    let eax = c.free(g, img, (0xcc, 0xc7), 0x10, 0);
    let eax = c.free_part(g, img, (0xd6, 0xd1), eax);
    let lf = Frame {
        at: c.target(img, 0xdb),
        esp: base.wrapping_sub(4),
        eax,
        ..Frame::default()
    };
    let r = crate::s4_small::static_leave_846(g, img, &lf);
    let eax = match r.jump {
        // call width: 0 (the tail jump to slot 846)
        Some(t) => g.call(t, 0, &[]).eax,
        None => r.eax,
    };
    let mut eax = crate::sa_misc::leave(g, w(0xe0), eax);
    let v = base + 0x20;
    for (c531, o531, c255, o255, cl, ol, cs, run) in [
        (
            0xf3u32, 0xeeu32, 0x101u32, 0xfcu32, 0x10bu32, 0x106u32, 0x118u32, None,
        ),
        (
            0x126,
            0x121,
            0x134,
            0x12f,
            0x148,
            0x139,
            0x155,
            Some((0x13d, 0x147)),
        ),
    ] {
        eax = crate::m0070::part_call(g, img, c.target(img, c531), 531, [eax, w(o531), v]);
        eax = crate::m0070::part_call(g, img, c.target(img, c255), 255, [eax, w(o255), v]);
        if let Some((a, b)) = run {
            c.abs(g, img, a, b);
        }
        let node = w(ol);
        let _ = crate::sa_misc::leave(g, node, eax);
        let _ = cl;
        // `ecx` the node as the leave keeps it
        let rec = base + 0x10;
        g.write::<u32>(rec, node);
        let mf = Frame {
            at: c.target(img, cs),
            esp: base.wrapping_sub(4),
            ecx: rec,
            ..Frame::default()
        };
        eax = crate::sa_summon::model_setup(g, img, &mf).eax;
    }
    let eax = crate::h6_setup::unlink_task(g, task);
    c.set(g, 0x4e20);
    eax
}

/// [`main_task_0070`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const MAIN_TASK_0070: Sig = Sig {
    len: 6137,
    hash: 0x5c40_2ec3_2f1a_61d9,
    calls: &[
        (0xcc, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xd6, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xdb, Callee::Sig(&crate::s4_small::STATIC_LEAVE_846)),
        (0xe5, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xf3, Callee::Sig(&crate::m0070::PART_CALL_531)),
        (0x101, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x10b, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x118, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x126, Callee::Sig(&crate::m0070::PART_CALL_531)),
        (0x134, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x148, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x155, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x1bd, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x1f5, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x235, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x38d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3e1, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x409, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x423, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x436, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x445, Callee::Sig(&crate::s4_small::ATTACH_211)),
        (0x45b, Callee::Sig(&crate::m0070::SHOW)),
        (0x47f, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x498, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x4a7, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x595, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5d7, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x5f1, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x62a, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x67d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6bf, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x6d9, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x6f3, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x759, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x7d0, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x7ec, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x849, Callee::Sig(&crate::s4_small::LIGHT_ON)),
        (0x8ee, Callee::Sig(&crate::s4_small::LIGHT_ON)),
        (0x904, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x936, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x978, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x992, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x9b5, Callee::Sig(&crate::m0070::STATIC_CHILD)),
        (0x9c9, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x9d3, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x9fb, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xaa5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xad7, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xb14, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xb2e, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xb3d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xb7f, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xb99, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xbc4, Callee::Sig(&crate::m0070::NODE_INIT_SCALED)),
        (0xbf7, Callee::Sig(&crate::s4_small::STATIC_LINK_845)),
        (0xc09, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xc3b, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xc63, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xca5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xcbf, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xce5, Callee::Sig(&crate::s4_small::BIG_NODE)),
        (0xd1f, Callee::Sig(&crate::m0070::ROOT_CHILD)),
        (0xd66, Callee::Sig(&crate::o4_ctor::COLOUR_CTOR_0070_97B0)),
        (0xdcd, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xdd7, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xde9, Callee::Sig(&crate::s4_small::BIG_NODE)),
        (0xe14, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0xe44, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0xe75, Callee::Sig(&crate::m0070::ROOT_CHILD)),
        (0xed0, Callee::Sig(&crate::s4_small::BONE_RATE)),
        (0xee4, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xfab, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xfd8, Callee::Sig(&crate::m0070::HIDE_MODEL)),
        (0xff9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x103b, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1055, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x10c6, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x11a5, Callee::Sig(&crate::s4_small::STATIC_NODE_ARGS)),
        (0x1246, Callee::Sig(&crate::sa_misc::NODE_START)),
        (0x125d, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x12a4, Callee::Sig(&crate::m0070::MODEL_POSITION)),
        (0x1386, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1390, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x13a1, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x13ab, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x13bc, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x13c6, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x13d7, Callee::Sig(&crate::s4_small::LEAVE_REGISTER)),
        (0x141a, Callee::Sig(&crate::m0042::RELEASE)),
        (0x1485, Callee::Sig(&crate::m0042::RELEASE)),
        (0x14aa, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x14f8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x153a, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1551, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x15a5, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1618, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1632, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x16c4, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x16f1, Callee::Any),
    ],
    imports: &[],
};
