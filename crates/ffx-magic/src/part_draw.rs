//! The part drawing that many overlays' own table functions inline
//! (`magic_0477`, `magic_0486` and others): the camera and model
//! matrices (slots 723, 724), a point copied into the part, the part's
//! transform (slots 258, 231, 230) and the draw (slot 712) under the
//! game flag `*G_LOCK`. Each copy has its own operand layout, given as
//! the operands' offsets in its code; every call passes the words left
//! on the original's stack.

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Game, Image};

/// One axis of a part's point: the `f32` at the operand `src`, then each
/// `(add, k)` step (`addss`/`subss` with the constant at operand `k`),
/// stored at the operand `dst`.
pub struct Axis {
    pub src: u32,
    pub steps: &'static [(bool, u32)],
    pub dst: u32,
}

impl Axis {
    /// The axis copied as it is.
    #[must_use]
    pub const fn copy(src: u32, dst: u32) -> Axis {
        Axis {
            src,
            steps: &[],
            dst,
        }
    }

    /// Computes and stores the axis.
    pub fn run<G: Game>(&self, g: &mut G, img: &Image, f: u32) {
        let w = |off: u32| img.word(f + off);
        let mut x = g.read::<f32>(w(self.src));
        for &(plus, k) in self.steps {
            let k = g.read::<f32>(w(k));
            x = if plus { add(x, k) } else { sub(x, k) };
        }
        g.write::<f32>(w(self.dst), x);
    }
}

/// The operands of the matrices' set-up: `G_P`, `G_CAM`, the item stored
/// at `*G_P`, the tables of slots 723 and 724, `G_M`.
pub struct Camera {
    pub gp: u32,
    pub cam: u32,
    pub item: u32,
    pub t723: u32,
    pub t724: u32,
    pub m: u32,
}

/// `*G_P = ITEM`, slot 723 `(*G_CAM)`, slot 724 `(*G_M)` (`below`: the
/// words under the first push). Returns the stack after them.
pub fn camera<G: Game>(g: &mut G, img: &Image, f: u32, c: &Camera, below: &[u32]) -> Vec<u32> {
    let w = |off: u32| img.word(f + off);
    let gp = g.read::<u32>(w(c.gp));
    let cam = g.read::<u32>(w(c.cam));
    g.write::<u32>(gp, w(c.item));
    let mut stack = vec![cam];
    stack.extend(below);
    let t = slot_target(g, w(c.t723), 723);
    // call width: 6
    g.call(t, 0, &stack);
    stack.insert(0, g.read::<u32>(w(c.m)));
    let t = slot_target(g, w(c.t724), 724);
    // call width: 6
    g.call(t, 0, &stack);
    stack
}

/// The operands of the part's transform: slot 258's table, the part,
/// slot 231's three words (in push order) and table, slot 230's two
/// words and table.
pub struct Transform {
    pub t258: u32,
    pub part: u32,
    pub a231: [u32; 3],
    pub t231: u32,
    pub a230: [u32; 2],
    pub t230: u32,
}

/// The point's axes, slot 258 `(part)`, slot 231 and slot 230 with the
/// camera words; `stack` grows by the pushes.
pub fn transform<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    t: &Transform,
    axes: &[Axis],
    stack: &mut Vec<u32>,
) {
    let w = |off: u32| img.word(f + off);
    let t258 = slot_target(g, w(t.t258), 258);
    for a in axes {
        a.run(g, img, f);
    }
    stack.insert(0, w(t.part));
    // call width: 12
    g.call(t258, 0, stack);
    for op in t.a231 {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let s = slot_target(g, w(t.t231), 231);
    // call width: 12
    g.call(s, 0, stack);
    for op in t.a230 {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let s = slot_target(g, w(t.t230), 230);
    // call width: 12
    g.call(s, 0, stack);
}

/// The operands of a draw that restores the flag: `G_P`, the item, the
/// flag pointer's global, the part, slot 712's table, the flag pointer's
/// global read after the call.
pub struct Restore {
    pub gp: u32,
    pub item: u32,
    pub lock: u32,
    pub part: u32,
    pub t712: u32,
    pub lock2: u32,
}

/// `*G_P = ITEM`, slot 712 `(part, 0x30)` with `*G_LOCK` 1, the flag
/// restored after. Returns slot 712's `eax`.
pub fn draw_restore<G: Game>(g: &mut G, img: &Image, f: u32, r: &Restore, stack: &[u32]) -> u32 {
    let w = |off: u32| img.word(f + off);
    let gp = g.read::<u32>(w(r.gp));
    g.write::<u32>(gp, w(r.item));
    let lock = g.read::<u32>(w(r.lock));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(r.t712), 712);
    let mut args = vec![w(r.part), 0x30];
    args.extend(stack);
    // call width: 14
    let eax = g.call(t, 0, &args).eax;
    let lock = g.read::<u32>(w(r.lock2));
    g.write::<u32>(lock, saved);
    eax
}

/// Starts the object `obj` as a part: `*G_P = ITEM`, slot 941 `(o[4])`
/// with `o = *G_O` gives the program table, and the `pppStartPart`
/// wrapper starts it with `(0x1000, table[index / 4], 1)`. `ops`: the
/// operands of `G_P`, `ITEM`, `G_O`, 941's table and the wrapper's
/// `rel32`. `below`: the words under the first push. Returns the words
/// left on the stack (the wrapper's three, 941's, `below`).
pub fn start_part<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    ops: [u32; 5],
    index: u32,
    below: &[u32],
) -> Vec<u32> {
    let w = |off: u32| img.word(f + off);
    let [gp, item, go, t941, call] = ops;
    let p = g.read::<u32>(w(gp));
    g.write::<u32>(p, w(item));
    let o = g.read::<u32>(w(go));
    let mut stack = vec![g.read::<u32>(o.wrapping_add(0x10))];
    stack.extend(below);
    let t = slot_target(g, w(t941), 941);
    // call width: 2
    let table = g.call(t, 0, &stack).eax;
    let program = g.read::<u32>(table.wrapping_add(index));
    let wrapper = img.rel32_target(f + call);
    crate::opu_parts::start_part(g, img, wrapper, obj, 0x1000, program, 1);
    stack.splice(0..0, [program, 1, 0]);
    stack
}

/// The operands of a [`draw_restart`] copy, in its code's order.
pub struct DrawOps {
    pub t258: u32,
    /// slot 231's three words, pushed in this order
    pub a231: [u32; 3],
    pub t231: u32,
    /// slot 230's two words, pushed in this order
    pub a230: [u32; 2],
    pub t230: u32,
    pub lock: u32,
    pub gp: u32,
    pub item: u32,
    pub t712: u32,
    pub lock2: u32,
    pub gp2: u32,
    pub item2: u32,
    pub t713: u32,
    /// the restart's [`start_part`] operands
    pub start: [u32; 5],
    pub index: u32,
    pub lock3: u32,
}

/// Draws the object `obj` as a part: slot 258 `(obj)`, slots 231 and 230
/// with the camera globals (the earlier words left on the stack), slot
/// 712 `(obj, 0x30)` with the game flag `*G_LOCK` 1 (and restored); when
/// 712 says nonzero, slot 713 `(obj)` removes it (`*G_P = ITEM`) and the
/// part starts again ([`start_part`]). The flag ends 0.
pub fn draw_restart<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, o: &DrawOps) {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(o.t258), 258);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in o.a231 {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(o.t231), 231);
    // call width: 8
    g.call(t, 0, &stack);
    for op in o.a230 {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(o.t230), 230);
    // call width: 8
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(o.lock));
    let gp = g.read::<u32>(w(o.gp));
    g.write::<u32>(lock, 1);
    g.write::<u32>(gp, w(o.item));
    let t = slot_target(g, w(o.t712), 712);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(o.lock2));
    g.write::<u32>(lock, saved);
    if r == 0 {
        g.write::<u32>(lock, 0);
        return;
    }
    let gp = g.read::<u32>(w(o.gp2));
    g.write::<u32>(gp, w(o.item2));
    let t = slot_target(g, w(o.t713), 713);
    g.call(t, 0, &[obj]);
    start_part(g, img, f, obj, o.start, o.index, &[obj]);
    let lock = g.read::<u32>(w(o.lock3));
    g.write::<u32>(lock, 0);
}
