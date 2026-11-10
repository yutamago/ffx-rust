//! Round 37's port of `magic_0533`'s main task (0x10009520), in pieces:
//! one function per state block, the dispatch in [`main_task_0533`].

// `comiss` with `ja`/`jb`: an unordered compare takes one side
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::r25::abs_stores;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// [`main_task_0533`]'s code offset, frame and record: `base` is `E -
/// 0xc` (the aligned frame `E` below the three pushes, so `[esp + x]` is
/// `base + x` with nothing pushed), `r` the task's record `R`.
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

    fn at(&self, o: u32) -> u32 {
        self.r.wrapping_add(o)
    }

    fn rd<G: Game>(&self, g: &G, o: u32) -> u32 {
        g.read::<u32>(self.at(o))
    }

    fn wr<G: Game>(&self, g: &mut G, o: u32, v: u32) {
        g.write::<u32>(self.at(o), v);
    }

    fn rf<G: Game>(&self, g: &G, o: u32) -> f32 {
        g.read::<f32>(self.at(o))
    }

    /// The state `R + 0x2908`.
    fn state<G: Game>(&self, g: &G) -> u32 {
        self.rd(g, 0x2908)
    }

    fn set<G: Game>(&self, g: &mut G, s: u32) {
        self.wr(g, 0x2908, s);
    }

    fn up<G: Game>(&self, g: &mut G) {
        let s = self.state(g).wrapping_add(1);
        self.set(g, s);
    }

    /// The count `R + 0x2918` (up at every call).
    fn count<G: Game>(&self, g: &G) -> i32 {
        self.rd(g, 0x2918).cast_signed()
    }

    /// The second timer `R + 0x2910` (up by `K` at every call).
    fn timer<G: Game>(&self, g: &G) -> f32 {
        self.rf(g, 0x2910)
    }

    /// `K` (operand `o`) above the timer: `comiss K, T; ja` waits.
    fn before<G: Game>(&self, g: &G, img: &Image, o: u32) -> bool {
        self.k(g, img, o) > self.timer(g)
    }

    /// `K` (operand `o`) above the first timer `R + 0x290c`.
    fn before_t1<G: Game>(&self, g: &G, img: &Image, o: u32) -> bool {
        self.k(g, img, o) > self.rf(g, 0x290c)
    }

    /// Slot 549 `(2)` (operand `op`) until it answers 0. Returns 0.
    fn wait_549<G: Game>(&self, g: &mut G, img: &Image, op: u32) -> u32 {
        loop {
            // call width: 1
            if g.call(self.slot(g, img, op, 549), 0, &[2]).eax == 0 {
                return 0;
            }
        }
    }

    /// An effect node (the call at `c`, `ecx` the node) with `(a, b, -,
    /// d)` pushed `d0` below `base` (the third word whatever `ecx` held:
    /// never read). Returns the node.
    fn effect<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, node): (u32, u32),
        (a, b, d): (u32, u32, u32),
        d0: u32,
    ) -> u32 {
        let sp = self.base.wrapping_sub(d0 + 0x10);
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

    /// An effect node's flags (operand `o`, the word written at `wo`): bits
    /// 0, 1 and 2 set, with the run of constant stores `run` between.
    /// Returns the flags.
    fn flags<G: Game>(&self, g: &mut G, img: &Image, (o, wo): (u32, u32), run: (u32, u32)) -> u32 {
        let fl = self.gw(g, img, o) | 7;
        abs_stores(g, img, self.f, run.0, run.1);
        g.write::<u32>(self.w(img, wo), fl);
        fl
    }

    /// A particle node (the call at `c`, `ecx` the node) with `(a, -, -,
    /// n)` pushed below `base` (the middle words the frame's). Returns its
    /// `eax`.
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

    /// The node `n` freed and unlinked through the local `base + loc` (the
    /// call at `c`). Returns its `eax`.
    fn free<G: Game>(&self, g: &mut G, img: &Image, c: u32, loc: u32, n: u32, eax: u32) -> u32 {
        let p = self.base + loc;
        g.write::<u32>(p, n);
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

    /// Slot 37 `(handle, 1, x)` through the call at `c` (`ecx` the handle
    /// at operand `h`), `esp` `d` below `base`; then the handle (operand
    /// `h2`) released (the call at `c2`). Returns the release's `eax`.
    fn call_release<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, h, x): (u32, u32, u32),
        (c2, h2): (u32, u32),
        d: u32,
    ) -> u32 {
        let sp = self.base.wrapping_sub(d + 4);
        g.write::<u32>(sp, x);
        let cf = Frame {
            at: self.target(img, c),
            esp: sp.wrapping_sub(4),
            ecx: self.gw(g, img, h),
            ..Frame::default()
        };
        crate::s4_small::call_37(g, img, &cf);
        let h2 = self.gw(g, img, h2);
        crate::m0042::release(g, img, self.target(img, c2), h2)
    }

    /// A record `[a, b, c, d, e, n]` at `base + 0x28`.
    fn rec6<G: Game>(&self, g: &mut G, words: [u32; 6]) -> u32 {
        let rec = self.base + 0x28;
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(rec + 4 * i, v);
        }
        rec
    }
}

/// `magic_0533`'s main task (0x10009520, 7378 bytes with its tables; a
/// task callback `(A)`, cdecl; an aligned frame `E` of 0x34 bytes below
/// three pushes, no /GS). The record `R = A[4]` keeps the state `R +
/// 0x2908`, two timers `R + 0x290c`/`R + 0x2910` (up by `K` at every
/// call), the count `R + 0x2918` (up at every call) and a step `R +
/// 0x2914`; the DLL's word `G_S` mirrors the state. States up to 0x65
/// dispatch through a byte table; some states run on into the next (0 into
/// 0x64; 0x65 into 0x5a past its wait):
/// - 0, 0x64, 0x1e: slots 719/306, an effect node, on;
/// - 0x5a, 0x5b, 0x14: the character shown, the nodes, owned and particle
///   nodes, the summon's model handle (slots 26, 0, 212, 30, 10, 257, 211,
///   37, 670), the move's nodes and the 2x link nodes; then four steps of
///   the bones (nodes, owned nodes, link nodes) and their unlinking;
/// - 0x1f, 0x20, 0x28: the fade, free and particle nodes, effect nodes
///   and slot 551; the move down, slot 296 for slot 288's list; the two
///   handles and their nodes and particle nodes;
/// - 0x32..0x3a: slot 20 and 689; the handles' nodes up and down in eight
///   steps; slots 296/37/670/211/384; the nodes freed and unlinked, the
///   static node, the effect nodes, slots 551/222;
/// - 0x46, 0x47, 0x50: slot 546, slot 37 and the release, an effect node,
///   then 10000;
/// - 10000/0x2711: slot 30, the list unlinked, the model set-up, the task
///   unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's dispatch, in order
pub fn main_task_0533<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; and esp, -8; sub esp, 0x34; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    let base = e.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(base + 0xc, r);
    let c = Ctx { f, base, r };
    let x = add(c.rf(g, 0x290c), c.k(g, img, 0x22));
    let n = c.rd(g, 0x2918).wrapping_add(1);
    c.wr(g, 0x2918, n);
    g.write::<f32>(c.at(0x290c), x);
    let y = add(c.rf(g, 0x2910), c.k(g, img, 0x46));
    g.write::<f32>(c.at(0x2910), y);
    let mut eax;
    let mut state = c.state(g);
    loop {
        if state >= 0x2710 {
            if state == 0x2710 {
                c.set(g, 0x2711);
                abs_stores(g, img, f, 0x1c01, 0x1c0b);
            } else if state != 0x2711 {
                return Ret::eax(state);
            }
            return Ret::eax(end_0533(g, img, &c, task));
        }
        if state > 0x65 {
            return Ret::eax(state);
        }
        let tb = img.word(c.w(img, 0x72) + state) & 0xff;
        eax = tb;
        match state {
            0 => {
                // call width: 0
                let flag = u32::from(g.call(c.slot(g, img, 0x7e, 689), 0, &[]).eax != 0);
                // call width: 3
                let h = g
                    .call(c.slot(g, img, 0x8f, 719), 0, &[c.w(img, 0x9c), 5, 0])
                    .eax;
                // call width: 3
                g.call(c.slot(g, img, 0xb5, 306), 0, &[7, flag, h]);
                c.set(g, 0x64);
                state = 0x64;
            }
            0x65 => {
                if c.count(g) < 0x3c {
                    return Ret::eax(tb);
                }
                c.wr(g, 0x290c, 0);
                c.set(g, 0x5a);
                state = 0x5a;
            }
            0x64 => {
                c.effect(
                    g,
                    img,
                    (0x164, c.w(img, 0x15f)),
                    (c.w(img, 0x15a), c.w(img, 0x155), 1),
                    0,
                );
                let fl = (c.gw(g, img, 0x169) | 6) | 1;
                abs_stores(g, img, f, 0x176, 0x180);
                g.write::<u32>(c.w(img, 0x181), fl);
                c.wr(g, 0x2918, 0);
                c.up(g);
                return Ret::eax(fl);
            }
            0x1e => {
                c.up(g);
                return Ret::eax(tb);
            }
            0x5a => {
                eax = show_0533(g, img, &c);
                if c.before_t1(g, img, 0x2ac) {
                    return Ret::eax(eax);
                }
                return Ret::eax(model_0533(g, img, &c));
            }
            0x5b => {
                if c.before_t1(g, img, 0x2ac) {
                    return Ret::eax(tb);
                }
                return Ret::eax(model_0533(g, img, &c));
            }
            0x14 => return Ret::eax(bones_0533(g, img, &c, tb)),
            0x1f => {
                if c.count(g) < 7 {
                    return Ret::eax(tb);
                }
                return Ret::eax(fade_0533(g, img, &c));
            }
            0x20 => return Ret::eax(down_0533(g, img, &c)),
            0x28 => return Ret::eax(handles_or_wait_0533(g, img, &c, tb)),
            0x32 => return Ret::eax(check_0533(g, img, &c)),
            0x33 => return Ret::eax(steps_0533(g, img, &c, tb)),
            0x34 => return Ret::eax(step_up_0533(g, img, &c)),
            0x35 => return Ret::eax(step_down_0533(g, img, &c, tb)),
            0x36 => return Ret::eax(handles_off_0533(g, img, &c)),
            0x37 => return Ret::eax(frees_0533(g, img, &c, tb)),
            0x38 => return Ret::eax(statics_0533(g, img, &c)),
            0x39 | 0x3a => return Ret::eax(waits_0533(g, img, &c, state, tb)),
            0x46 => {
                let h = c.gw(g, img, 0x1b18);
                // call width: 1
                let eax = g.call(c.slot(g, img, 0x1b12, 546), 0, &[h]).eax;
                c.up(g);
                abs_stores(g, img, f, 0x1b2d, 0x1b37);
                c.wr(g, 0x2910, 0);
                return Ret::eax(eax);
            }
            0x47 => {
                if c.count(g) < 0xb2 {
                    return Ret::eax(tb);
                }
                abs_stores(g, img, f, 0x1b63, 0x1b6d);
                c.call_release(g, img, (0x1b6e, 0x1b5a, 0xc8), (0x1b7c, 0x1b74), 0);
                let z = c.wait_549(g, img, 0x1b81);
                c.effect(
                    g,
                    img,
                    (0x1ba8, c.w(img, 0x1ba3)),
                    (c.w(img, 0x1b9e), c.w(img, 0x1b99), z),
                    0,
                );
                let fl = c.flags(g, img, (0x1bad, 0x1bc5), (0x1bba, 0x1bc4));
                c.set(g, 0x50);
                c.wr(g, 0x2918, 0);
                return Ret::eax(fl);
            }
            0x50 => {
                if c.count(g) <= 0x7d {
                    return Ret::eax(tb);
                }
                c.set(g, 0x2710);
                state = 0x2710;
            }
            _ => return Ret::eax(eax),
        }
    }
}

/// [`main_task_0533`]'s end: slot 30 on the handle, the list unlinked,
/// the model set-up of `G_M` (a record in the frame), the task unlinked.
/// Returns the task node's next.
fn end_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, task: u32) -> u32 {
    let h = c.gw(g, img, 0x10e);
    // call width: 1
    let eax = g.call(c.slot(g, img, 0x108, 30), 0, &[h]).eax;
    crate::l3_common::list_unlink_current(g, img, c.target(img, 0x11b), eax);
    let rec = c.base + 0x24;
    g.write::<u32>(rec, c.w(img, 0x127));
    let mf = Frame {
        at: c.target(img, 0x12c),
        esp: c.base.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::sa_summon::model_setup(g, img, &mf);
    crate::h6_setup::unlink_task(g, task)
}

/// [`main_task_0533`]'s state 0x5a: the character shown, the summon's
/// node and vector (slot 325), a second node, the owned node with its
/// colour, its particle node; slot 549 until 0, an effect node and its
/// flags; on, `G_S` 0x5a. Returns the flags.
fn show_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    crate::l3_common::chr_show(g, img, c.target(img, 0x19d));
    let a = c.at(0xd1c);
    abs_stores(g, img, f, 0x1ae, 0x1cc);
    crate::sb_misc::node_append(g, img, c.target(img, 0x1cd), a, w(0x1a8), 0);
    // call width: 2
    g.call(c.slot(g, img, 0x1de, 325), 0, &[c.at(0xd38), w(0x1d8)]);
    let b = c.at(0xd48);
    let list = w(0x1f1);
    crate::sb_misc::node_append(g, img, c.target(img, 0x1f6), b, list, 0);
    for (o, v) in [
        (0xd64u32, ONE),
        (0xd68, ONE),
        (0xd6c, ONE),
        (0xd70, 0x3f66_6666),
    ] {
        c.wr(g, o, v);
    }
    // slot 325's two words stay under the owned node's four
    let ebx = c.at(0xbd0);
    let sp = c.base.wrapping_sub(0x18);
    c.owned(g, img, (0x237, ebx, list), [w(0x208), a, w(0x1fc), b], sp);
    c.particle(g, img, (0x24b, c.at(0xad0)), (ebx, 4));
    c.wait_549(g, img, 0x253);
    c.effect(g, img, (0x277, w(0x272)), (ebx, w(0x26c), 6), 0);
    let fl = c.flags(g, img, (0x27c, 0x294), (0x289, 0x293));
    c.up(g);
    abs_stores(g, img, f, 0x29e, 0x2a8);
    fl
}

/// [`main_task_0533`]'s state 0x5b past its wait: the summon's handle
/// (slots 26, 0, 212, 30, 10, 257, 211, 257, 37, 670), the move's nodes,
/// two link nodes per bone (each the next's `d`/`x`), the bones' words;
/// then the four bones of state 0x14. Returns 0x14's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn model_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    // call width: 2
    g.call(c.slot(g, img, 0x2be, 26), 0, &[0x300f, 1]);
    // call width: 1
    let h = g.call(c.slot(g, img, 0x2cf, 0), 0, &[0x300f]).eax;
    g.write::<u32>(w(0x2e0), h);
    // call width: 2
    g.call(c.slot(g, img, 0x2e8, 212), 0, &[h, 0]);
    let hw = |g: &G, o: u32| c.gw(g, img, o);
    // call width: 1 (slot 212's words stay under it)
    g.call(c.slot(g, img, 0x2f5, 30), 0, &[hw(g, 0x2fb)]);
    // call width: 2
    g.call(c.slot(g, img, 0x305, 10), 0, &[hw(g, 0x313), 0]);
    // call width: 2
    g.call(c.slot(g, img, 0x31a, 257), 0, &[hw(g, 0x322), 1]);
    // call width: 2
    g.call(c.slot(g, img, 0x32f, 211), 0, &[hw(g, 0x33a), w(0x334)]);
    // call width: 2
    g.call(c.slot(g, img, 0x347, 257), 0, &[hw(g, 0x34f), 0]);
    // call width: 3
    g.call(c.slot(g, img, 0x35c, 37), 0, &[hw(g, 0x36f), 1, 0xb2]);
    // call width: 2
    g.call(c.slot(g, img, 0x376, 670), 0, &[hw(g, 0x387), 0]);
    let r = c.r;
    crate::sb_misc::node_append(g, img, c.target(img, 0x399), c.at(0x58), w(0x394), 0);
    for (o, v) in [(0x74u32, 0), (0x78, 0x400), (0x7c, 0)] {
        c.wr(g, o, v);
    }
    crate::sa_misc::node_append(g, img, c.target(img, 0x3ba), r, w(0x39e), 0);
    for (o, v) in [(0x1cu32, 0), (0x20, 0x4040_0000), (0x24, 0), (0x28, 0)] {
        c.wr(g, o, v);
    }
    crate::sa_misc::node_append(g, img, c.target(img, 0x3e0), c.at(0x2c), r, 0);
    for (o, v) in [(0x48u32, 0), (0x4c, 0x3ee6_6667), (0x50, 0), (0x54, 0)] {
        c.wr(g, o, v);
    }
    // the bones: four words in the frame step on per bone
    let (p10, p14, p18) = (base + 0x10, base + 0x14, base + 0x18);
    g.write::<u32>(p18, w(0x406));
    g.write::<u32>(p14, w(0x40e));
    g.write::<u32>(p10, w(0x416));
    let list = w(0x422);
    let mut i = 0u32;
    let mut node = w(0x41b);
    loop {
        crate::sb_misc::node_append(g, img, c.target(img, 0x429), node, list, 0);
        g.write::<u32>(node + 0x20, 0x479e_b100);
        g.write::<u32>(node + 0x28, ONE);
        // the second node appended inline (`esi` = 0x34 i)
        let o = 0x34 * i;
        let n2 = w(0x43d).wrapping_add(o);
        g.write::<u16>(w(0x444).wrapping_add(o), 0xff);
        g.write::<u32>(w(0x44c).wrapping_add(o), n2);
        g.write::<u32>(w(0x452).wrapping_add(o), n2);
        g.write::<u32>(w(0x458).wrapping_add(o), list);
        g.write::<u32>(w(0x45e).wrapping_add(o), list);
        let tail = c.gw(g, img, 0x463);
        g.write::<u32>(w(0x469).wrapping_add(o), tail);
        let tail = c.gw(g, img, 0x46e);
        if tail == list {
            g.write::<u32>(w(0x478), n2);
        } else {
            g.write::<u32>(tail + 4, n2);
        }
        let k = g.read::<u16>(w(0x484)).wrapping_add(1);
        g.write::<u16>(w(0x484), k);
        g.write::<u32>(w(0x48a), n2);
        g.write::<u32>(n2, w(0x490));
        g.write::<u32>(w(0x4a2).wrapping_add(o), node);
        g.write::<u32>(w(0x4a8).wrapping_add(o), w(0x4ac));
        // the link nodes: (node, y), then (a, -, -, the first link node),
        // then (the second)
        let n3 = w(0x49b).wrapping_add(o);
        let sp = base.wrapping_sub(8);
        g.write::<u32>(sp, node);
        g.write::<u32>(sp + 4, w(0x495));
        let lf = Frame {
            at: c.target(img, 0x4b1),
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
            at: c.target(img, 0x4c5),
            esp: sp.wrapping_sub(4),
            ecx: n4,
            ..Frame::default()
        };
        crate::s4_small::link_node_ad(g, img, &lf);
        let p = g.read::<u32>(p18);
        let sp = base.wrapping_sub(0x14);
        g.write::<u32>(sp, n4);
        let lf = Frame {
            at: c.target(img, 0x4d2),
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
        if node.cast_signed() >= w(0x4ff).cast_signed() {
            break;
        }
    }
    abs_stores(g, img, f, 0x50d, 0x535);
    for o in [0x2914u32, 0x2918, 0x2910] {
        c.wr(g, o, 0);
    }
    c.set(g, 0x14);
    // 0x14's `eax`: the last bone's `3 i + 4`
    bones_0533(g, img, c, 3 * (i - 1) + 4)
}

/// [`main_task_0533`]'s state 0x14: while fewer than four bones and the
/// timer reached `K`, one bone: slot 324's word, its nodes (appends, slot
/// 325 vectors), the scale from slot 307 (`(v - K) K' + K''`), the owned
/// node, slot 719's handle and the grey link node; then past the count
/// 0x2d the camera's words, slots 37 and 670, the four link nodes
/// released, the bones' nodes leave their lists; state 0x1e. Returns
/// `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn bones_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, eax_in: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let mut eax = eax_in;
    let i = c.rd(g, 0x2914);
    if i.cast_signed() < 4 && c.timer(g) >= c.k(g, img, 0x575) {
        // call width: 0
        let v = g.call(c.slot(g, img, 0x580, 324), 0, &[]).eax;
        let i = c.rd(g, 0x2914);
        let node = |j: u32| c.r.wrapping_add(j.wrapping_mul(0x2c));
        crate::sb_misc::node_append(g, img, c.target(img, 0x5a2), node(i + 3), w(0x59b), 0);
        let i = c.rd(g, 0x2914);
        // call width: 2
        g.call(
            c.slot(g, img, 0x5bd, 325),
            0,
            &[node(i).wrapping_add(0xa0), w(0x5b7)],
        );
        let i = c.rd(g, 0x2914);
        let x = add(g.read::<f32>(node(i) + 0xa4), c.k(g, img, 0x5e4));
        g.write::<f32>(node(i) + 0xa4, x);
        let i = c.rd(g, 0x2914);
        crate::sb_misc::node_append(g, img, c.target(img, 0x600), node(i + 7), w(0x5d3), 0);
        let i = c.rd(g, 0x2914);
        // call width: 2 (slot 325's words stay under it)
        g.call(
            c.slot(g, img, 0x61b, 325),
            0,
            &[node(i).wrapping_add(0x150), w(0x615)],
        );
        let i = c.rd(g, 0x2914);
        g.write::<u32>(node(i) + 0x154, 0x4000_0000);
        let i = c.rd(g, 0x2914);
        g.write::<u32>(node(i) + 0x15c, 0);
        let i = c.rd(g, 0x2914);
        crate::sa_misc::node_append(g, img, c.target(img, 0x663), node(i + 0xf), w(0x631), 0);
        for (o, x) in [(0x2b0u32, 0x8000u32), (0x2b4, v & 0xffff), (0x2b8, 0)] {
            let i = c.rd(g, 0x2914);
            g.write::<u32>(node(i) + o, x);
        }
        let i = c.rd(g, 0x2914);
        crate::sb_misc::node_append(
            g,
            img,
            c.target(img, 0x6b9),
            node(i + 0x13),
            node(i + 0xf),
            0,
        );
        for (o, x) in [(0x360u32, 0u32), (0x364, (v & 0x400) + 0x200), (0x368, 0)] {
            let i = c.rd(g, 0x2914);
            g.write::<u32>(node(i) + o, x);
        }
        let i = c.rd(g, 0x2914);
        crate::sa_misc::node_append(g, img, c.target(img, 0x718), node(i + 0xb), node(i + 7), 0);
        let i = c.rd(g, 0x2914);
        // call width: 2 (slot 325's words stay under it)
        g.call(
            c.slot(g, img, 0x733, 325),
            0,
            &[node(i).wrapping_add(0x200), w(0x72d)],
        );
        // call width: 0 (the earlier calls' words stay under it)
        let out = g.call(c.slot(g, img, 0x740, 307), 0, &[]);
        let s = f32::from_bits(crate::sa_summon::st0_f32(out));
        // fstp [esp + 0x34]: `E + 0x10`
        g.write::<f32>(base + 0x1c, s);
        let x = add(
            mul(sub(s, c.k(g, img, 0x75a)), c.k(g, img, 0x76b)),
            c.k(g, img, 0x778),
        );
        let i = c.rd(g, 0x2914);
        g.write::<f32>(node(i) + 0x20c, x);
        let i = c.rd(g, 0x2914);
        let owner = c.r.wrapping_add(i.wrapping_mul(0x14c)).wrapping_add(0x3f4);
        let sp = base.wrapping_sub(0x28);
        c.owned(
            g,
            img,
            (0x7ba, owner, w(0x770)),
            [w(0x7b3), node(i + 3), node(i + 0xf), node(i + 7)],
            sp,
        );
        // call width: 3 (the earlier calls' words stay under it)
        let h = g.call(c.slot(g, img, 0x7bf, 719), 0, &[w(0x7ce), 4, 0]).eax;
        let i = c.rd(g, 0x2914);
        let sp = base.wrapping_sub(8);
        g.write::<u32>(
            sp,
            c.r.wrapping_add(i.wrapping_mul(0x14c)).wrapping_add(0x3f4),
        );
        g.write::<u32>(sp + 4, h);
        let lf = Frame {
            at: c.target(img, 0x7fb),
            esp: sp.wrapping_sub(4),
            ecx: c.r.wrapping_add(i.wrapping_mul(0x28)).wrapping_add(0x924),
            ..Frame::default()
        };
        crate::s4_small::link_node_grey(g, img, &lf);
        let i = c.rd(g, 0x2914);
        g.write::<u8>(
            c.r.wrapping_add(i.wrapping_mul(0x28)).wrapping_add(0x94b),
            0x40,
        );
        c.wr(g, 0x2914, i.wrapping_add(1));
        c.wr(g, 0x2910, 0);
        eax = i.wrapping_mul(5);
    }
    if c.count(g) < 0x2d {
        return eax;
    }
    let x = c.gw(g, img, 0x834);
    g.write::<u32>(w(0x841), x);
    let x = c.gw(g, img, 0x849);
    g.write::<u32>(w(0x851), x);
    let x = sub(c.k(g, img, 0x859), c.k(g, img, 0x861));
    let h = c.gw(g, img, 0x86e);
    abs_stores(g, img, f, 0x872, 0x87c);
    g.write::<f32>(w(0x880), x);
    abs_stores(g, img, f, 0x884, 0x88e);
    abs_stores(g, img, f, 0x894, 0x89e);
    // call width: 3
    g.call(c.slot(g, img, 0x839, 37), 0, &[h, 1, 0xb3]);
    let h = c.gw(g, img, 0x8b2);
    // call width: 2
    g.call(c.slot(g, img, 0x8a1, 670), 0, &[h, 0]);
    let p = base + 0x1c;
    for k in 0..4u32 {
        g.write::<u32>(p, c.at(0x924 + 0x28 * k));
        crate::m0042::unlink(g, img, c.target(img, 0x8d9), p);
    }
    // the bones' nodes leave their lists: five kinds, two bones
    // (`edx` walks the first kind's `+4` up to the operand at `+0xacf`)
    let mut eax = 0;
    let mut k = 0u32;
    loop {
        let first = w(0x8ee).wrapping_sub(4).wrapping_add(0x2c * k);
        for (i, n) in (0u32..).zip([
            first,
            w(0x99f).wrapping_add(0x34 * k),
            w(0xa0e).wrapping_add(0x34 * k),
            w(0x8e9).wrapping_sub(4).wrapping_add(0x3c * k),
            w(0x8e4).wrapping_sub(4).wrapping_add(0x24 * k),
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
        if (first + 0x30).cast_signed() >= w(0xacf).cast_signed() {
            break;
        }
    }
    for o in [0x2910u32, 0x2918] {
        c.wr(g, o, 0);
    }
    c.set(g, 0x1e);
    eax
}

/// [`main_task_0533`]'s state 0x1f past its wait: the fade's vector
/// (slot 325), the node freed, a particle node; slot 549 until 0, an
/// effect node, slot 719's handle into slot 551, a second effect node;
/// on, `G_S` 0x64. Returns the second node's flags.
fn fade_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    abs_stores(g, img, f, 0xb20, 0xb2a);
    c.wr(g, 0x4c, 0);
    c.wr(g, 0x20, 0);
    // call width: 2
    let eax = g
        .call(c.slot(g, img, 0xb1c, 325), 0, &[c.at(0xd38), w(0xb16)])
        .eax;
    let x = sub(c.rf(g, 0xd3c), c.k(g, img, 0xb4c));
    let a = c.at(0xad0);
    c.wr(g, 0xd70, 0x3e4c_cccd);
    g.write::<f32>(c.at(0xd3c), x);
    c.free(g, img, 0xb74, 0x1c, a, eax);
    let ebx = c.at(0xbd0);
    c.particle(g, img, (0xb87, a), (ebx, 1));
    c.wait_549(g, img, 0xb91);
    c.effect(g, img, (0xbb5, w(0xbb0)), (ebx, w(0xbaa), 7), 0);
    let fl = c.gw(g, img, 0xbba) | 7;
    g.write::<u32>(w(0xbc5), fl);
    abs_stores(g, img, f, 0xbd0, 0xbda);
    // call width: 3
    let h = g.call(c.slot(g, img, 0xbca, 719), 0, &[w(0xbe3), 6, 8]).eax;
    // call width: 3
    g.call(c.slot(g, img, 0xbf2, 551), 0, &[h, 0, 2]);
    c.effect(g, img, (0xc10, w(0xc0b)), (ebx, w(0xc05), 2), 0);
    let fl = c.flags(g, img, (0xc15, 0xc2d), (0xc22, 0xc2c));
    c.up(g);
    abs_stores(g, img, f, 0xc37, 0xc41);
    fl
}

/// [`main_task_0533`]'s state 0x20: the move's word `R + 0x78` down by
/// 0x40 while not negative; then at count 0x77 the node leaves, the light
/// word, slot 37 and the release, slot 296 for each byte slot 288 lists
/// but two; from count 0x78 the node freed (unless bit 2), an effect node,
/// state 0x28 (run on). Returns `eax` as the original leaves it.
fn down_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let m = c.rd(g, 0x78);
    if m.cast_signed() >= 0 {
        let m = m.wrapping_add(0xffff_ffc0);
        c.wr(g, 0x78, m);
        return m;
    }
    let mut eax = m;
    if c.count(g) == 0x77 {
        abs_stores(g, img, f, 0xc6c, 0xc76);
        crate::sa_misc::leave(g, c.at(0x58), eax);
        let x1 = mul(c.k(g, img, 0xc7f), c.k(g, img, 0xc87));
        let x = sub(c.k(g, img, 0xc8f), x1);
        g.write::<f32>(w(0xca6), x);
        c.wr(g, 0x4c, 0xbc44_9ba6);
        c.wr(g, 0x20, 0);
        c.call_release(g, img, (0xcb9, 0xc95, 0xb0), (0xcc7, 0xcbf), 0);
        // slot 296 for each byte slot 288 lists (the count at `E + 0xc`)
        let n_at = base + 0x18;
        // call width: 2
        let list = g.call(c.slot(g, img, 0xcd1, 288), 0, &[0x10, n_at]).eax;
        eax = list;
        let mut n = g.read::<i32>(n_at);
        let mut i = 0i32;
        let mut at = list;
        while i < n {
            let b = g.read::<u8>(at);
            eax = (eax & !0xff) | u32::from(b);
            if u32::from(b) != c.gw(g, img, 0xcf7) && b != g.read::<u8>(w(0xcff)) {
                // call width: 2
                eax = g
                    .call(c.slot(g, img, 0xd06, 296), 0, &[u32::from(b), 0])
                    .eax;
                n = g.read::<i32>(n_at);
            }
            i += 1;
            at = at.wrapping_add(1);
        }
    }
    if c.count(g) < 0x78 {
        return eax;
    }
    if g.read::<u8>(c.at(0xbb8)) & 4 == 0 {
        c.free(g, img, 0xd47, 0x1c, c.at(0xad0), eax);
    }
    c.effect(g, img, (0xd60, w(0xd5b)), (c.at(0xbd0), w(0xd55), 3), 0);
    let fl = c.flags(g, img, (0xd65, 0xd7d), (0xd72, 0xd7c));
    c.wr(g, 0x2910, 0);
    c.set(g, 0x28);
    handles_or_wait_0533(g, img, c, fl)
}

/// [`main_task_0533`]'s state 0x28: past `K` on the timer, the node
/// leaves, the fade's vector (slot 325), a particle node, the light word,
/// slot 26; the two handles (slots 0, 212, 30, 10) with their nodes,
/// owned nodes and particle nodes; state 0x32 (run on). Returns `eax` as
/// the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn handles_or_wait_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, eax: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    if c.before(g, img, 0xd99) {
        return eax;
    }
    crate::sa_misc::leave(g, c.at(0x2c), eax);
    c.wr(g, 0x20, 0xbf33_3333);
    // call width: 2
    g.call(c.slot(g, img, 0xdbf, 325), 0, &[c.at(0xd64), w(0xdb9)]);
    let x = add(c.rf(g, 0xd3c), c.k(g, img, 0xdde));
    c.wr(g, 0xd70, 0x3e19_999a);
    g.write::<f32>(c.at(0xd3c), x);
    c.particle(g, img, (0xe0a, c.at(0xad0)), (c.at(0xbd0), 6));
    let y = mul(c.k(g, img, 0xe12), c.k(g, img, 0xe1a));
    g.write::<f32>(w(0xe29), y);
    // call width: 2 (the particle node's words stay under it)
    g.call(c.slot(g, img, 0xe1f, 26), 0, &[0x3010, 1]);
    let list = w(0xe41);
    let (a, b) = (c.at(0x1218), c.at(0x1244));
    g.write::<u32>(base + 0x24, a);
    crate::sb_misc::node_append(g, img, c.target(img, 0xe4c), a, list, 0);
    for (o, v) in [
        (0x1234u32, 0),
        (0x1238, 0xbdcc_cccd),
        (0x123c, 0x434d_0000),
        (0x1240, ONE),
    ] {
        c.wr(g, o, v);
    }
    g.write::<u32>(base + 0x20, b);
    crate::sb_misc::node_append(g, img, c.target(img, 0xe85), b, list, 0);
    g.write::<u32>(base + 0x1c, c.at(0xf80));
    for (o, v) in [
        (0x1260u32, ONE),
        (0x1264, ONE),
        (0x1268, ONE),
        (0x126c, 0x3dcc_cccd),
    ] {
        c.wr(g, o, v);
    }
    let (p10, p14, p18, p1c) = (base + 0x10, base + 0x14, base + 0x18, base + 0x1c);
    g.write::<u32>(p14, 0);
    g.write::<u32>(p18, c.at(0xd80));
    g.write::<u32>(p10, 0);
    let mut esi = c.at(0xa84);
    let mut hp = w(0xedc);
    loop {
        // call width: 1
        let h = g.call(c.slot(g, img, 0xee1, 0), 0, &[0x3010]).eax;
        g.write::<u32>(hp, h);
        // call width: 2
        g.call(c.slot(g, img, 0xef7, 212), 0, &[h, 0]);
        let hv = g.read::<u32>(hp);
        // call width: 1 (slot 212's words stay under it)
        g.call(c.slot(g, img, 0xf04, 30), 0, &[hv]);
        let hv = g.read::<u32>(hp);
        // call width: 2
        g.call(c.slot(g, img, 0xf10, 10), 0, &[hv, 0]);
        let n0 = esi.wrapping_sub(0xc0);
        crate::sb_misc::node_append(g, img, c.target(img, 0xf2c), n0, w(0xf21), 0);
        // call width: 2 (slot 10's words stay under it)
        g.call(
            c.slot(g, img, 0xf3d, 325),
            0,
            &[esi.wrapping_sub(0xa4), w(0xf37)],
        );
        crate::sa_misc::node_append(g, img, c.target(img, 0xf56), esi.wrapping_sub(0x68), n0, 0);
        g.write::<u32>(esi.wrapping_sub(0x4c), 0);
        g.write::<u32>(esi.wrapping_sub(0x48), 0xbe4c_cccd);
        g.write::<u32>(esi.wrapping_sub(0x44), 0);
        // a node appended inline: `esi - 0x10`, its table first
        let n1 = esi.wrapping_sub(0x10);
        g.write::<u16>(esi + 0xa, 0xff);
        g.write::<u32>(n1, w(0xf7a));
        g.write::<u32>(esi, n1);
        g.write::<u32>(esi.wrapping_sub(4), n1);
        g.write::<u32>(esi + 4, w(0xf86));
        g.write::<u32>(esi.wrapping_sub(0xc), w(0xf8d));
        let tail = c.gw(g, img, 0xf92);
        g.write::<u32>(esi.wrapping_sub(8), tail);
        let tail = c.gw(g, img, 0xf9a);
        if tail == w(0xf9f) {
            g.write::<u32>(w(0xfa7), n1);
        } else {
            g.write::<u32>(tail + 4, n1);
        }
        let k = g.read::<u16>(w(0xfb3)).wrapping_add(1);
        g.write::<u16>(w(0xfb3), k);
        g.write::<u32>(w(0xfb9), n1);
        g.write::<u32>(esi + 0xc, 0);
        let v = c
            .gw(g, img, 0xfc5)
            .wrapping_sub(g.read::<u32>(p14))
            .wrapping_add(0xa000);
        g.write::<u32>(esi + 0x10, v);
        g.write::<u32>(esi + 0x14, 0);
        let o10 = g.read::<u32>(p10);
        let sp = base.wrapping_sub(0x10);
        c.owned(
            g,
            img,
            (0xffe, w(0xfed).wrapping_add(o10), w(0xff2)),
            [w(0xfe7), n0, n1, w(0xfce)],
            sp,
        );
        let o10 = g.read::<u32>(p10);
        g.write::<u8>(w(0x1008).wrapping_add(o10), 1);
        let hv = g.read::<u32>(hp);
        // call width: 2 (the owned node's words stay under it)
        g.call(
            c.slot(g, img, 0x1015, 211),
            0,
            &[hv, w(0x100f).wrapping_add(o10)],
        );
        let hv = g.read::<u32>(hp);
        // call width: 3
        g.call(c.slot(g, img, 0x1024, 37), 0, &[hv, 1, 0xb0]);
        let hv = g.read::<u32>(hp);
        // call width: 2
        g.call(c.slot(g, img, 0x103a, 670), 0, &[hv, 0]);
        let o10 = g.read::<u32>(p10);
        let owned = w(0x105e).wrapping_add(o10);
        let list2 = w(0x1068);
        let sp = base.wrapping_sub(0x18);
        c.owned(
            g,
            img,
            (0x106d, owned, list2),
            [w(0x1063), w(0x1058), n1, w(0x1052)],
            sp,
        );
        let words = [
            owned,
            g.read::<u32>(base + 0x24),
            w(0x107a),
            g.read::<u32>(base + 0x20),
        ];
        let ebx = g.read::<u32>(p1c);
        let sp = base.wrapping_sub(0x28);
        c.owned(g, img, (0x1086, ebx, list2), words, sp);
        let part = g.read::<u32>(p18);
        let src = c.gw(g, img, 0x108f);
        g.write::<u32>(part, w(0x1099));
        let rec = c.rec6(g, [part, w(0x10a8), ebx, src, w(0x10b8), 3]);
        crate::sb_nodes::particle_node(g, img, c.target(img, 0x10c5), rec);
        for (p, by) in [(p14, 0x4000u32), (p18, 0x100), (p10, 0x14c)] {
            let v = g.read::<u32>(p).wrapping_add(by);
            g.write::<u32>(p, v);
        }
        hp = hp.wrapping_add(4);
        esi = esi.wrapping_add(0x2c);
        g.write::<u32>(p1c, ebx.wrapping_add(0x14c));
        if hp.cast_signed() >= w(0x10f3).cast_signed() {
            break;
        }
    }
    abs_stores(g, img, f, 0x1101, 0x110b);
    c.set(g, 0x32);
    check_0533(g, img, c)
}

/// [`main_task_0533`]'s state 0x32: slot 20 on the first handle; when it
/// answers 0, slot 689: 0 resets the count and steps on, else state 0x36.
/// Returns the last slot's `eax`.
fn check_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let h = c.gw(g, img, 0x111c);
    // call width: 1
    let eax = g.call(c.slot(g, img, 0x1116, 20), 0, &[h]).eax;
    if eax != 0 {
        return eax;
    }
    // call width: 0
    let eax = g.call(c.slot(g, img, 0x1131, 689), 0, &[]).eax;
    if eax == 0 {
        c.wr(g, 0x2918, 0);
        c.up(g);
    } else {
        c.set(g, 0x36);
    }
    eax
}

/// [`main_task_0533`]'s state 0x33: at count 1 the vector (slot 325),
/// slot 37 and the release; from count 2 the light word, the node freed,
/// a particle node, an effect node; slot 549 until 0, a second effect
/// node, slot 719's handle into slot 551; on, the count reset, then the
/// steps of 0x34. Returns `eax` as the original leaves it.
fn steps_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, tb: u32) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let mut eax = tb;
    if c.count(g) == 1 {
        abs_stores(g, img, f, 0x1170, 0x117a);
        // call width: 2
        g.call(c.slot(g, img, 0x116c, 325), 0, &[c.at(0xa0c), w(0x1166)]);
        // slot 325's words stay under slot 37's
        eax = c.call_release(g, img, (0x118e, 0x1184, 0xb1), (0x119c, 0x1194), 8);
    }
    if c.count(g) < 2 {
        return eax;
    }
    let x = mul(c.k(g, img, 0x11b1), c.k(g, img, 0x11b9));
    let a = c.at(0xad0);
    g.write::<f32>(w(0x11cb), x);
    c.free(g, img, 0x11d4, 0x24, a, eax);
    let ebx = c.at(0xbd0);
    c.particle(g, img, (0x11e7, a), (ebx, 3));
    c.wr(g, 0x2910, 0);
    c.wr(g, 0x2914, 8);
    c.effect(g, img, (0x1211, w(0x1202)), (ebx, w(0x11fc), 5), 0);
    c.flags(g, img, (0x1216, 0x122e), (0x1223, 0x122d));
    c.wait_549(g, img, 0x1233);
    c.effect(g, img, (0x1257, w(0x1252)), (ebx, w(0x124c), 8), 0);
    let fl = c.gw(g, img, 0x125c) | 7;
    g.write::<u32>(w(0x1267), fl);
    abs_stores(g, img, f, 0x1272, 0x127c);
    // call width: 3
    let h = g
        .call(c.slot(g, img, 0x126c, 719), 0, &[w(0x1285), 6, 9])
        .eax;
    // call width: 3
    g.call(c.slot(g, img, 0x1294, 551), 0, &[h, 1, 2]);
    c.up(g);
    c.wr(g, 0x2918, 0);
    step_up_0533(g, img, c)
}

/// [`main_task_0533`]'s state 0x34: while bones are left and the timer
/// reached `K`, the next bone down: its node, words, vector (slot 325),
/// scale (slot 307), slot 324's word, the owned node and its particle
/// node; then from count 0x1e the light word, slot 325, slots 30, 37 and
/// 670 on the first handle; on, the step 8, then the steps of 0x35.
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn step_up_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let n = c.rd(g, 0x2914);
    g.write::<u32>(base + 0x1c, n);
    let mut eax = n;
    if n.cast_signed() > 0 && c.timer(g) >= c.k(g, img, 0x12d0) {
        let j = n - 1;
        let o = j.wrapping_mul(0x2c);
        let at = |x: u32| c.r.wrapping_add(o).wrapping_add(x);
        let node = at(0x24d0);
        c.wr(g, 0x2910, 0);
        c.wr(g, 0x2914, j);
        g.write::<u32>(base + 0x24, node);
        let list = w(0x12eb);
        crate::sb_misc::node_append(g, img, c.target(img, 0x1306), node, list, 0);
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let x = add(
            mul(j.cast_signed() as f32, c.k(g, img, 0x131b)),
            c.k(g, img, 0x1326),
        );
        let ebx = at(0x2630);
        g.write::<u32>(at(0x24ec), 0);
        g.write::<u32>(at(0x24f0), 0);
        g.write::<f32>(at(0x24f4), x);
        g.write::<u32>(at(0x24f8), ONE);
        crate::sb_misc::node_append(g, img, c.target(img, 0x135b), ebx, list, 0);
        // call width: 2
        g.call(c.slot(g, img, 0x136e, 325), 0, &[at(0x264c), w(0x1368)]);
        // slot 325's words stay under the float's call
        let ff = Frame {
            at: c.target(img, 0x137b),
            esp: base.wrapping_sub(0xc),
            ..Frame::default()
        };
        let v = match crate::s4_misc::float_307(g, img, &ff).xmm0 {
            Some(crate::Xmm::F32(x)) => f32::from_bits(x),
            _ => 0.0,
        };
        let y = add(
            mul(sub(v, c.k(g, img, 0x1383)), c.k(g, img, 0x1396)),
            c.k(g, img, 0x139e),
        );
        g.write::<f32>(at(0x2658), y);
        let edi = at(0x2790);
        crate::sa_misc::node_append(g, img, c.target(img, 0x13b0), edi, w(0x138e), 0);
        g.write::<u32>(at(0x27ac), 0);
        // call width: 0 (slot 325's words stay under it)
        let h = g.call(c.slot(g, img, 0x13c4, 324), 0, &[]).eax & 0xffff;
        g.write::<u32>(at(0x27b0), h);
        g.write::<u32>(at(0x27b4), 0);
        let owner = c.r.wrapping_add(j.wrapping_mul(0x14c)).wrapping_add(0x1a70);
        let sp = base.wrapping_sub(0x18);
        c.owned(
            g,
            img,
            (0x1414, owner, w(0x140f)),
            [w(0x140a), node, edi, ebx],
            sp,
        );
        g.write::<u8>(
            c.r.wrapping_add(j.wrapping_mul(0x14c)).wrapping_add(0x1bb8),
            1,
        );
        let part = c.r.wrapping_add(0x1270).wrapping_add(j << 8);
        eax = c.particle(g, img, (0x1440, part), (owner, 0));
    }
    if c.count(g) < 0x1e {
        return eax;
    }
    let x = mul(c.k(g, img, 0x1458), c.k(g, img, 0x1460));
    g.write::<f32>(w(0x1479), x);
    abs_stores(g, img, f, 0x1483, 0x148d);
    // call width: 2
    g.call(c.slot(g, img, 0x1471, 325), 0, &[c.at(0x9e0), w(0x146b)]);
    let h = c.gw(g, img, 0x1496);
    // call width: 1 (slot 325's words stay under it)
    g.call(c.slot(g, img, 0x1490, 30), 0, &[h]);
    let h = c.gw(g, img, 0x14b6);
    // call width: 3
    g.call(c.slot(g, img, 0x14a0, 37), 0, &[h, 1, 0xb1]);
    let h = c.gw(g, img, 0x14ce);
    // call width: 2
    let eax = g.call(c.slot(g, img, 0x14bd, 670), 0, &[h, 0]).eax;
    c.up(g);
    c.wr(g, 0x2910, 0);
    c.wr(g, 0x2914, 8);
    c.wr(g, 0x2918, 0);
    step_down_0533(g, img, c, eax)
}

/// [`main_task_0533`]'s state 0x35: while bones are left and the timer
/// reached `K`, the next bone back: its height and words, the vector
/// (slot 325), the scale (slot 307), slot 324's word, its particle part
/// freed and a new particle node; then from count 0x1e on, into 0x36.
/// Returns `eax` as the original leaves it.
fn step_down_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, eax_in: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let base = c.base;
    let mut eax = eax_in;
    let n = c.rd(g, 0x2914);
    if n.cast_signed() > 0 && c.timer(g) >= c.k(g, img, 0x1514) {
        let j = n - 1;
        let o = j.wrapping_mul(0x2c);
        let at = |x: u32| c.r.wrapping_add(o).wrapping_add(x);
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let x = add(
            mul(j.cast_signed() as f32, c.k(g, img, 0x1530)),
            c.k(g, img, 0x153b),
        );
        c.wr(g, 0x2910, 0);
        c.wr(g, 0x2914, j);
        g.write::<f32>(at(0x24f4), x);
        g.write::<u32>(at(0x24ec), 0);
        g.write::<u32>(at(0x24f0), 0);
        g.write::<u32>(at(0x24f8), ONE);
        // call width: 2
        g.call(c.slot(g, img, 0x1568, 325), 0, &[at(0x264c), w(0x1562)]);
        let ff = Frame {
            at: c.target(img, 0x1599),
            esp: base.wrapping_sub(4),
            ..Frame::default()
        };
        let v = match crate::s4_misc::float_307(g, img, &ff).xmm0 {
            Some(crate::Xmm::F32(x)) => f32::from_bits(x),
            _ => 0.0,
        };
        let y = add(
            mul(sub(v, c.k(g, img, 0x15a1)), c.k(g, img, 0x15b9)),
            c.k(g, img, 0x15c1),
        );
        g.write::<u32>(at(0x27ac), 0);
        g.write::<f32>(at(0x2658), y);
        // call width: 0
        let h = g.call(c.slot(g, img, 0x15a6, 324), 0, &[]).eax & 0xffff;
        g.write::<u32>(at(0x27b0), h);
        g.write::<u32>(at(0x27b4), 0);
        let b = c.r.wrapping_add(j.wrapping_mul(0x14c));
        let part = c.r.wrapping_add(0x1270).wrapping_add(j << 8);
        g.write::<u32>(b + 0x1b98, w(0x1614));
        // `eax` is the record here (`mov eax, [esp + 0xc]`)
        c.free(g, img, 0x161d, 0x24, part, c.r);
        eax = c.particle(g, img, (0x1635, part), (b + 0x1a70, 0));
    }
    if c.count(g) < 0x1e {
        return eax;
    }
    c.up(g);
    handles_off_0533(g, img, c)
}

/// [`main_task_0533`]'s state 0x36: slot 296 `(G_A, 1)`, slots 37 and 670
/// and 211 on the summon's handle, slot 384 on both handles; the timer
/// reset, the count reset, on. Returns slot 384's `eax`.
fn handles_off_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let a = c.gw(g, img, 0x165c);
    abs_stores(g, img, f, 0x1666, 0x1670);
    // call width: 2
    g.call(c.slot(g, img, 0x1654, 296), 0, &[a, 1]);
    let h = c.gw(g, img, 0x1686);
    // call width: 3 (slot 296's words stay under it)
    g.call(c.slot(g, img, 0x1673, 37), 0, &[h, 1, 0xc9]);
    let h = c.gw(g, img, 0x169e);
    // call width: 2
    g.call(c.slot(g, img, 0x168d, 670), 0, &[h, 0]);
    let h = c.gw(g, img, 0x16b0);
    // call width: 2 (slot 670's words stay under it)
    g.call(c.slot(g, img, 0x16a5, 211), 0, &[h, w(0x16aa)]);
    let mut hp = w(0x16c0);
    let eax = loop {
        let h = g.read::<u32>(hp);
        // call width: 1
        let eax = g.call(c.slot(g, img, 0x16d1, 384), 0, &[h]).eax;
        hp = hp.wrapping_add(4);
        if hp.cast_signed() >= w(0x16e7).cast_signed() {
            break eax;
        }
    };
    c.wr(g, 0x2910, 0);
    c.wr(g, 0x2918, 0);
    c.up(g);
    eax
}

/// [`main_task_0533`]'s state 0x37: the two handles' nodes leave and
/// their particle parts are freed; unless slot 689 answers, the eight
/// bones' owned nodes leave their lists and their parts are freed; the
/// fade, the node freed, the particle node again; on, then 0x38. Returns
/// `eax` as the original leaves it.
fn frees_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, tb: u32) -> u32 {
    let w = |o: u32| c.w(img, o);
    let (mut esi, mut ebx) = (c.at(0xd80), c.at(0xf80));
    let mut eax = tb;
    for _ in 0..2 {
        eax = crate::sa_misc::leave(g, ebx, eax);
        eax = c.free(g, img, 0x1720, 0x24, esi, eax);
        ebx = ebx.wrapping_add(0x14c);
        esi = esi.wrapping_add(0x100);
    }
    // call width: 0
    eax = g.call(c.slot(g, img, 0x1734, 689), 0, &[]).eax;
    if eax == 0 {
        let (mut part, mut node) = (c.at(0x1270), c.at(0x1a70));
        for _ in 0..8 {
            let parent = g.read::<u32>(node + 0x14);
            if parent != 0 {
                crate::nodes::leave_parent(g, node, parent);
                eax = node;
            }
            eax = c.free(g, img, 0x17aa, 0x24, part, eax);
            node = node.wrapping_add(0x14c);
            part = part.wrapping_add(0x100);
        }
    }
    let x = sub(c.rf(g, 0xd3c), c.k(g, img, 0x17d0));
    let a = c.at(0xad0);
    c.wr(g, 0xd70, 0x3eb3_3333);
    g.write::<f32>(c.at(0xd3c), x);
    c.free(g, img, 0x17f5, 0x24, a, eax);
    let src = c.gw(g, img, 0x1804);
    g.write::<u32>(a, w(0x182e));
    let rec = c.rec6(g, [a, w(0x1814), c.at(0xbd0), src, w(0x1820), 2]);
    crate::sb_nodes::particle_node(g, img, c.target(img, 0x1833), rec);
    c.up(g);
    statics_0533(g, img, c)
}

/// [`main_task_0533`]'s state 0x38: the static node appended inline (its
/// words, after the list's last), its globals, the camera's words, slots
/// 37 and 670; two effect nodes (slot 549 until 0 between), slot 719's
/// handle into slot 551, slot 222; on, then 0x39. Returns `eax` as the
/// original leaves it.
fn statics_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let f = c.f;
    let w = |o: u32| c.w(img, o);
    let last = c.gw(g, img, 0x183e);
    abs_stores(g, img, f, 0x1842, 0x187d);
    g.write::<u32>(w(0x187e), last);
    if last == w(0x1883) {
        abs_stores(g, img, f, 0x1889, 0x1893);
    } else {
        g.write::<u32>(last + 4, w(0x1898));
    }
    let k = g.read::<u16>(w(0x189f)).wrapping_add(1);
    g.write::<u16>(w(0x189f), k);
    abs_stores(g, img, f, 0x18a8, 0x18f5);
    c.wr(g, 0x20, 0);
    let x = mul(c.k(g, img, 0x1900), c.k(g, img, 0x1908));
    let h = c.gw(g, img, 0x1915);
    g.write::<f32>(w(0x191d), x);
    let y = mul(c.k(g, img, 0x1925), c.k(g, img, 0x192d));
    abs_stores(g, img, f, 0x1931, 0x193b);
    g.write::<f32>(w(0x193f), y);
    let z = mul(c.k(g, img, 0x1947), c.k(g, img, 0x194f));
    g.write::<f32>(w(0x1957), z);
    // call width: 3
    g.call(c.slot(g, img, 0x18a4, 37), 0, &[h, 1, 0xb1]);
    let h = c.gw(g, img, 0x1975);
    // call width: 2
    g.call(c.slot(g, img, 0x1964, 670), 0, &[h, 0]);
    let esi = c.at(0xbd0);
    c.wr(g, 0x2918, 0);
    let rec = c.rec6(g, [w(0x1996), w(0x199e), esi, w(0x19aa), w(0x19b2), 4]);
    crate::sb_nodes::effect_node(g, img, c.target(img, 0x19bf), rec);
    let n = g.read::<u32>(rec);
    g.write::<u32>(n, w(0x19c9));
    c.flags(g, img, (0x19ce, 0x19e3), (0x19d8, 0x19e2));
    c.wait_549(g, img, 0x19f1);
    let rec = c.rec6(g, [w(0x1a0e), w(0x1a16), esi, w(0x1a22), w(0x1a2a), 9]);
    crate::sb_nodes::effect_node(g, img, c.target(img, 0x1a37), rec);
    let n = g.read::<u32>(rec);
    g.write::<u32>(n, w(0x1a43));
    let fl = c.gw(g, img, 0x1a48) | 7;
    g.write::<u32>(w(0x1a53), fl);
    abs_stores(g, img, f, 0x1a5c, 0x1a66);
    // call width: 3
    let h = g
        .call(c.slot(g, img, 0x1a58, 719), 0, &[w(0x1a6f), 6, 0])
        .eax;
    // call width: 3
    g.call(c.slot(g, img, 0x1a7e, 551), 0, &[h, 0, 2]);
    // call width: 4
    let eax = g
        .call(c.slot(g, img, 0x1a8b, 222), 0, &[w(0x1a9f), 0xc, 0xc, 2])
        .eax;
    c.up(g);
    c.wr(g, 0x2910, 0);
    waits_0533(g, img, c, 0x39, eax)
}

/// [`main_task_0533`]'s states 0x39 and 0x3a: past `K` on the timer, on
/// (`G_S` 0x46) into 0x3a; past `K'` the camera's words and state 0x46.
/// Returns `eax` as it came.
fn waits_0533<G: Game>(g: &mut G, img: &Image, c: &Ctx, state: u32, eax: u32) -> u32 {
    if state == 0x39 {
        if c.before(g, img, 0x1abc) {
            return eax;
        }
        c.up(g);
        abs_stores(g, img, c.f, 0x1ad3, 0x1add);
    }
    if c.before(g, img, 0x1ae1) {
        return eax;
    }
    c.wr(g, 0x24, 0xbecc_cccd);
    c.wr(g, 0x20, 0xbf80_0000);
    c.set(g, 0x46);
    eax
}

/// [`main_task_0533`] (the `rel32`s: its callees in order).
pub const MAIN_TASK_0533: Sig = Sig {
    len: 7378,
    hash: 0x2d61_7d2a_bd42_77b0,
    calls: &[
        (0x11b, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x12c, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x164, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x19d, Callee::Sig(&crate::l3_common::CHR_SHOW)),
        (0x1cd, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1f6, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x237, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x24b, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x277, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x399, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3ba, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x3e0, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x429, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4b1, Callee::Sig(&crate::s4_small::LINK_NODE_XY)),
        (0x4c5, Callee::Sig(&crate::s4_small::LINK_NODE_AD)),
        (0x4d2, Callee::Sig(&crate::s4_small::LINK_NODE_X)),
        (0x5a2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x600, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x663, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x6b9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x718, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x7ba, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x7fb, Callee::Sig(&crate::s4_small::LINK_NODE_GREY)),
        (0x8d9, Callee::Sig(&crate::m0042::UNLINK)),
        (0xb74, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xb87, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xbb5, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xc10, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xc77, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xcb9, Callee::Sig(&crate::s4_small::CALL_37)),
        (0xcc7, Callee::Sig(&crate::m0042::RELEASE)),
        (0xd47, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xd60, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xdae, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe0a, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xe4c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xe85, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf2c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf56, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xffe, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x106d, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1086, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x10c5, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x118e, Callee::Sig(&crate::s4_small::CALL_37)),
        (0x119c, Callee::Sig(&crate::m0042::RELEASE)),
        (0x11d4, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x11e7, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1211, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1257, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1306, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x135b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x137b, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x13b0, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x1414, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1440, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1599, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x161d, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1635, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1713, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1720, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x17aa, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x17f5, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1833, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x19bf, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1a37, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1b6e, Callee::Sig(&crate::s4_small::CALL_37)),
        (0x1b7c, Callee::Sig(&crate::m0042::RELEASE)),
        (0x1ba8, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
    ],
    imports: &[],
};
