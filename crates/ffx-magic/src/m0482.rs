//! `magic_0482.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Inserts a model node (0x10003550, internal: `ecx` = a group of words
/// `[node, parent, a, b, c]`): the node's own children list empty
/// (`+0x1a` 0xff, `+0xc`/`+0x10` itself, `+0x14` the parent); with a
/// parent, it is appended to the parent's children (`+0x4` the parent,
/// `+0x8` the old last, the parent's `+0xc` or the old last's `+0x4` the
/// node, the count `+0x18` up, `+0x10` the node). Then `+0x13c` = `a`,
/// `+0x150` = `c`, `+0x140` = `b`, the two DLL words at operands `+0x5f`
/// (`+0x144`) and `+0x69` (`+0x128`), byte `+0x148` 0, `+0x14c` and
/// `+0xb0` 1.0, `+0xac`, `+0xb4`, `+0xb8`, `+0x11c..+0x124` 0; slot 235
/// `(m)` makes the matrix `m = +0x5c`, slot 230 copies it to `+0xdc` and
/// `+0x1c`, slot 325 `(+0x12c, V)` (`V` at `+0xfe`; the earlier words
/// left on the stack). Returns slot 325's `eax`.
pub fn insert<G: Game>(g: &mut G, img: &Image, f: u32, group: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let parent = g.read::<u32>(group + 4);
    let node = g.read::<u32>(group);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    for (src, dst) in [(8u32, 0x13cu32), (0x10, 0x150), (0xc, 0x140)] {
        let v = g.read::<u32>(group + src);
        g.write::<u32>(node + dst, v);
    }
    g.write::<u32>(node + 0x144, w(0x5f));
    g.write::<u32>(node + 0x128, w(0x69));
    g.write::<u8>(node + 0x148, 0);
    g.write::<u32>(node + 0x14c, ONE);
    g.write::<u32>(node + 0xac, 0);
    g.write::<u32>(node + 0xb0, ONE);
    for off in [0xb4, 0xb8, 0x124, 0x120, 0x11c] {
        g.write::<u32>(node + off, 0);
    }
    let m = node + 0x5c;
    let t = slot_target(g, w(0x55), 235);
    g.call(t, 0, &[m]);
    let t = slot_target(g, w(0xd9), 230);
    let mut stack = vec![node + 0xdc, m, m];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xeb), 230);
    stack.splice(0..0, [node + 0x1c, m]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x104), 325);
    stack.splice(0..0, [node + 0x12c, w(0xfe)]);
    g.call(t, 0, &stack).eax
}

/// [`insert`] (278 bytes, 1 overlay).
pub const INSERT: Sig = Sig {
    len: 278,
    hash: 0x3cf1_9b7c_776d_4101,
    calls: &[],
    imports: &[],
};

/// A task (0x10006d00, `(task)`, a task callback) on the record `r =
/// task[1]`, by its state `+0x18`:
/// - 0: state 1 and a wait `+0x34` of 10; 1: while the wait runs, it
///   waits; then state 10;
/// - 10: slot 415 `(*G)` (`G` at operand `+0x57`); motion 0x7a when it
///   says 0x79, else 0x8a, through slot 228 `(*G, motion)`; state 11;
/// - 11: when slot 411 `(*G)` says nonzero, the task's node `*task` leaves
///   its ring (linked to itself) and the state is 20000.
///
/// The wait goes down at every exit. Returns the last call's `eax` (the
/// jump table's index for states 0 and 1, the state past 11).
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let mut state = g.read::<u32>(r + 0x18);
    let eax = loop {
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(r + 0x18, 1);
                    g.write::<u32>(r + 0x34, 0xa);
                }
                if g.read::<i32>(r + 0x34) > 0 {
                    // the jump table's index
                    break state;
                }
                g.write::<u32>(r + 0x18, 0xa);
                state = 0xa;
            }
            10 => {
                let t = slot_target(g, w(0x51), 415);
                let a = g.read::<u32>(w(0x57));
                let r415 = g.call(t, 0, &[a]).eax;
                let t = slot_target(g, w(0x6a), 228);
                let (op, motion) = if r415 == 0x79 {
                    (0x7a, 0x7a)
                } else {
                    (0x94, 0x8a)
                };
                let a = g.read::<u32>(w(op));
                let eax = g.call(t, 0, &[a, motion]).eax;
                let s = g.read::<u32>(r + 0x18);
                g.write::<u32>(r + 0x18, s.wrapping_add(1));
                break eax;
            }
            11 => {
                let t = slot_target(g, w(0xa8), 411);
                let a = g.read::<u32>(w(0xae));
                let r411 = g.call(t, 0, &[a]).eax;
                if r411 == 0 {
                    break 0;
                }
                let node = g.read::<u32>(task);
                let next = g.read::<u32>(node + 4);
                let prev = g.read::<u32>(node + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(node + 8);
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                g.write::<u32>(r + 0x18, 0x4e20);
                break next;
            }
            // 2..9 share the table's last entry (index 4)
            2..=9 => break 4,
            _ => break state,
        }
    };
    let n = g.read::<u32>(r + 0x34);
    g.write::<u32>(r + 0x34, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`task`] (268 bytes with its jump tables, 1 overlay).
pub const TASK: Sig = Sig {
    len: 268,
    hash: 0x88fc_fa82_7e8c_621b,
    calls: &[],
    imports: &[],
};
