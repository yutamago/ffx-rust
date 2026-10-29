//! Node handlers of the summon family (`magic_0003`'s code, the same
//! bytes in 98 to 116 overlays; addresses are `magic_0003`'s). The nodes
//! hang in the rings [`crate::nodes`] describes: a handler gets the
//! cursor (`this`), walks the current node's children
//! ([`crate::nodes::walk`]) and, when the cursor moved, does its own work
//! on the node (`[this]`): model calls through `g_MagicImports` with the
//! node's fields. Most are internal (`ecx` = the cursor), reached through
//! the 12-byte thunks the node tables point at (`mov ecx, [ebp + 8]; jmp`);
//! they take the frame of that jump ([`Frame::ecx`], `esp` at the
//! caller's return address) and give what the thunk returns.

use crate::nodes::walk;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

fn f32_at(g: &impl crate::Mem, addr: u32) -> f32 {
    f32::from_bits(g.read::<u32>(addr))
}

/// The `g_MagicImports` entry `slot`, through the table global named by
/// the operand at `f + off` (each call's own `mov eax, ds:G`).
fn slot<G: Game>(g: &G, img: &Image, f: u32, off: u32, slot: u32) -> u32 {
    slot_target(g, img.word(f + off), slot)
}

/// A float a slot returned in ST0, stored with `fstp dword`: rounded to
/// f32; the x87 "indefinite" NaN when the callee left the stack empty.
pub(crate) fn st0_f32(out: crate::Out) -> u32 {
    out.st0.map_or(0xffc0_0000, |v| (v as f32).to_bits())
}

/// 0x10002e90 (internal, `ecx` = the cursor): the walk; when the cursor
/// moved, the node's model gets its position `+0x1c` by the model number
/// byte `+0x2c` (slot 366 when bit 0 of byte `+0x2d` is set, else slot
/// 217) and its scale `+0x28` is reset to 1.0. Returns the node, or the
/// walk's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let s = if g.read::<u8>(n + 0x2d) & 1 != 0 {
        366
    } else {
        217
    };
    let target = slot(g, img, fr.at, 0x59, s);
    let model = u32::from(g.read::<u8>(n + 0x2c));
    g.call(target, n, &[model, n + 0x1c]);
    let n = g.read::<u32>(this);
    g.write::<u32>(n + 0x28, ONE);
    Ret::eax(n)
}

/// `place` (141 bytes, 116 overlays).
pub const PLACE: Sig = Sig {
    len: 141,
    hash: 0x5693_082d_ff00_1754,
    calls: &[],
    imports: &[],
};

/// 0x100024b0 (internal, `ecx` = the cursor; no walk): sets up the model
/// of the node `[this]` from its flags `+0x90`: unless bit 0 is set, the
/// object `+0x50` gets its motion `+0x64` (slot 538, when not -1) and
/// `+0x6c` (slot 213); bit 4 makes slot 211 (`+0x54`, 0); then the model
/// `+0x54` gets the colour `+0x3c..+0x44` (slot 35, mode 7) and the alpha
/// `+0x8c` (slot 34). Flag 0x800 adds slot 547 and slot 276 on the
/// model, and, without bit 0, slot 276 on each of the object's two parts
/// slot 219 gives (0 and 1) that exist. Returns the last call's `eax`
/// (0x800 without flag 0x800, 0 for a missing part).
pub fn model_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let n = g.read::<u32>(fr.ecx);
    if g.read::<u8>(n + 0x90) & 1 == 0 {
        let motion = g.read::<u32>(n + 0x64);
        if motion != u32::MAX {
            let t = slot(g, img, f, 0x16, 538);
            let obj = g.read::<u32>(n + 0x50);
            g.call(t, 0, &[obj, motion]);
        }
        let t = slot(g, img, f, 0x2c, 213);
        let (obj, m) = (g.read::<u32>(n + 0x50), g.read::<u32>(n + 0x6c));
        g.call(t, 0, &[obj, m]);
    }
    if g.read::<u8>(n + 0x90) & 0x10 != 0 {
        let t = slot(g, img, f, 0x48, 211);
        let model = g.read::<u32>(n + 0x54);
        g.call(t, 0, &[model, 0]);
    }
    let t = slot(g, img, f, 0x62, 35);
    let model = g.read::<u32>(n + 0x54);
    let c = [
        g.read::<u32>(n + 0x3c),
        g.read::<u32>(n + 0x40),
        g.read::<u32>(n + 0x44),
    ];
    g.call(t, 0, &[model, 7, c[0], c[1], c[2]]);
    let t = slot(g, img, f, 0x9a, 34);
    let model = g.read::<u32>(n + 0x54);
    let alpha = g.read::<u32>(n + 0x8c);
    g.call(t, 0, &[model, alpha]);
    if g.read::<u16>(n + 0x90) & 0x800 == 0 {
        return Ret::eax(0x800);
    }
    let t = slot(g, img, f, 0xc7, 547);
    let model = g.read::<u32>(n + 0x54);
    g.call(t, 0, &[model]);
    let t = slot(g, img, f, 0xd7, 276);
    let model2 = g.read::<u32>(n + 0x54);
    // slot 547's word stays below the two pushes
    let mut eax = g.call(t, 0, &[model2, 0, model]).eax;
    if g.read::<u8>(n + 0x90) & 1 != 0 {
        return Ret::eax(eax);
    }
    for (o219, o276, part) in [(0xf5, 0x111, 0), (0x121, 0x13d, 1)] {
        let t = slot(g, img, f, o219, 219);
        let obj = g.read::<u32>(n + 0x50);
        eax = g.call(t, 0, &[obj, part]).eax;
        if eax != 0 {
            let t = slot(g, img, f, o276, 276);
            eax = g.call(t, 0, &[eax, 0]).eax;
        }
    }
    Ret::eax(eax)
}

/// `model_setup` (334 bytes, 116 overlays).
pub const MODEL_SETUP: Sig = Sig {
    len: 334,
    hash: 0x2658_1d84_e618_d348,
    calls: &[],
    imports: &[],
};

/// 0x10003fc0 (internal, `ecx` = the cursor): the walk; when the cursor
/// moved, the node's own ring (the node object `+0x3c`) is walked with a
/// cursor of its own, a local of this frame (`[ebp - 8]` the object,
/// byte `[ebp - 3]` the pass; the byte at `[ebp - 4]` is the callee's),
/// through the object's handler (`[[obj] + 0x1c]`, cdecl `(cursor)`),
/// then slot 229 gets the node's `+0x1c`, the object's `+0x9c`, the
/// node's `+0x2c` (and the cursor's address left on the stack). Returns
/// slot 229's `eax`, or the walk's.
pub fn sub_walk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let pass = g.read::<u8>(this + 5);
    let obj = g.read::<u32>(n + 0x3c);
    // push ebp; mov ebp, esp: the cursor is at [ebp - 8]
    let cursor = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(cursor, obj);
    g.write::<u8>(cursor + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
    g.call(handler, cursor, &[cursor]);
    let obj = g.read::<u32>(n + 0x3c);
    let t = slot(g, img, fr.at, 0x75, 229);
    let r = g.call(t, 0, &[n + 0x1c, obj.wrapping_add(0x9c), n + 0x2c, cursor]);
    Ret::eax(r.eax)
}

/// `sub_walk` (139 bytes, 116 overlays).
pub const SUB_WALK: Sig = Sig {
    len: 139,
    hash: 0xabd7_b1d6_578c_3d23,
    calls: &[],
    imports: &[],
};

/// 0x10003100 (internal, `ecx` = the cursor): the walk; when the cursor
/// moved, slot 290 gets the model number byte `+0x3c` and the position
/// `+0x1c`, then the node's `+0x1c..+0x24` are scaled by `+0x2c..+0x34`
/// and `+0x28` takes `+0x38`. Returns `+0x38`, or the walk's `eax`.
pub fn scaled_place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let t = slot(g, img, fr.at, 0x55, 290);
    let model = u32::from(g.read::<u8>(n + 0x3c));
    g.call(t, n, &[model, n + 0x1c]);
    for i in 0..3 {
        let n = g.read::<u32>(this);
        let v = mul(f32_at(g, n + 0x2c + 4 * i), f32_at(g, n + 0x1c + 4 * i));
        g.write::<u32>(n + 0x1c + 4 * i, v.to_bits());
    }
    let n = g.read::<u32>(this);
    let v = g.read::<u32>(n + 0x38);
    g.write::<u32>(n + 0x28, v);
    Ret::eax(v)
}

/// `scaled_place` (162 bytes, 116 overlays).
pub const SCALED_PLACE: Sig = Sig {
    len: 162,
    hash: 0x7e1e_6945_7d07_994c,
    calls: &[],
    imports: &[],
};

/// 0x10002370 (a node handler, cdecl `(cursor)`): the walk (`ecx` as
/// the caller left it); when the cursor moved, slot 237 turns the node's
/// `+0x3c`/`+0x40` into a vector at `+0x1c` and slot 248 combines it with
/// `+0x2c` in place (slot 237's words stay below). Returns slot 248's
/// `eax`, or the walk's.
pub fn aim<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let (eax, moved) = walk(g, this, fr.ecx, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let v = n + 0x1c;
    let t = slot(g, img, fr.at, 0x4b, 237);
    let (a, b) = (g.read::<u32>(n + 0x3c), g.read::<u32>(n + 0x40));
    g.call(t, 0, &[a, b, 0, v]);
    let t = slot(g, img, fr.at, 0x68, 248);
    let r = g.call(t, 0, &[v, v, n + 0x2c, a, b, 0, v]);
    Ret::eax(r.eax)
}

/// `aim` (126 bytes, 116 overlays).
pub const AIM: Sig = Sig {
    len: 126,
    hash: 0x3ec2_a23b_556b_6071,
    calls: &[],
    imports: &[],
};

/// 0x100023f0 (internal, `ecx` = `[node, parent, object, flags byte]`):
/// sets up a model node: pass byte `+0x1a` 0xff (as a u16), an empty
/// child list, the parent `+0x14` (appended to the parent's child list
/// as in [`crate::m0003::node_init`]); the flags (u16 `+0x90`), the object
/// `+0x50`, `+0x68`/`+0x6c` 0, motion `+0x64` -1; the model `+0x54` from
/// the object (slot 413 with flag bit 0, else slot 215), its colour into
/// `+0x3c..+0x44` (slot 14) and its alpha (slot 333's ST0) into `+0x8c`.
/// Returns slot 333's `eax`.
pub fn model_node_init<G: Game>(g: &mut G, img: &Image, f: u32, args: u32) -> u32 {
    let parent = g.read::<u32>(args + 4);
    let node = g.read::<u32>(args);
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
        let count = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, count.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    let obj = g.read::<u32>(args + 8);
    let flags = g.read::<u8>(args + 0xc);
    g.write::<u16>(node + 0x90, u16::from(flags));
    g.write::<u32>(node + 0x50, obj);
    g.write::<u32>(node + 0x68, 0);
    g.write::<u32>(node + 0x6c, 0);
    g.write::<u32>(node + 0x64, u32::MAX);
    let s = if flags & 1 != 0 { 413 } else { 215 };
    let t = slot(g, img, f, 0x49, s);
    let model = g.call(t, 0, &[obj]).eax;
    g.write::<u32>(node + 0x54, model);
    let t = slot(g, img, f, 0x8e, 14);
    let m = g.read::<u32>(node + 0x54);
    let colour = [node + 0x3c, node + 0x40, node + 0x44];
    g.call(t, 0, &[m, colour[0], colour[1], colour[2]]);
    let t = slot(g, img, f, 0x9b, 333);
    let m2 = g.read::<u32>(node + 0x54);
    let out = g.call(t, 0, &[m2, m, colour[0], colour[1], colour[2]]);
    g.write::<u32>(node + 0x8c, st0_f32(out));
    out.eax
}

/// `model_node_init` (181 bytes, 113 overlays).
pub const MODEL_NODE_INIT: Sig = Sig {
    len: 181,
    hash: 0x0396_a1aa_133d_c866,
    calls: &[],
    imports: &[],
};

/// 0x10006490 (internal, no arguments): binds the summon's models: slot
/// 723 on model A and slot 724 on model B (the DLL's `.data` blocks named
/// by the operands), slot 230 links C to B, `*G_D` to A and `*G_E` to B
/// (each call's words stay on the stack for the next), then slot 253
/// says whether the binding failed: the flag `**G_F` and `G_FLAG` become
/// 0 when it answers non-zero, else 1. Returns `*G_F`.
pub fn bind_models<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let (a, b) = (w(0x06), w(0x18));
    let t = slot(g, img, f, 0x01, 723);
    g.call(t, 0, &[a]);
    let t = slot(g, img, f, 0x13, 724);
    g.call(t, 0, &[b, a]);
    let t = slot(g, img, f, 0x25, 230);
    let mut stack = vec![w(0x35), w(0x2a), b, a];
    // call width: 4 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for (op, d, x) in [(0x3c, 0x47, 0x41), (0x54, 0x5f, 0x59)] {
        let t = slot(g, img, f, op, 230);
        let v = g.read::<u32>(w(d));
        stack.splice(0..0, [v, w(x)]);
        // call width: 8 (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let t = slot(g, img, f, 0x6c, 253);
    let failed = g.call(t, 0, &[]).eax != 0;
    let flag = g.read::<u32>(w(0x7e));
    if failed {
        g.write::<u32>(flag, 0);
        g.write::<u32>(w(0x9d), 0);
    } else {
        g.write::<u32>(flag, 1);
        g.write::<u32>(w(0x8c), 1);
    }
    flag
}

/// `bind_models` (166 bytes, 116 overlays).
pub const BIND_MODELS: Sig = Sig {
    len: 166,
    hash: 0x675d_df5a_379e_b20e,
    calls: &[],
    imports: &[],
};

/// 0x10002f30 (internal, `ecx` = the cursor): the walk; when the cursor
/// moved, the node's operation byte `+0x40` picks a model call with the
/// node's vector `+0x1c` and its owner node `+0x38` (model `+0x54`,
/// object `+0x50`, flags `+0x90`): 0 slot 262 (the vector's three
/// words), 1 slot 29, 2 slot 268 (with `+0x3c`), 3/4/5 slots 366/665/531
/// on the object unless flag bit 0, 6 slot 262 then slot 12's ST0 into
/// the vector's `+0x4`; then the vector moves by its velocity `+0x2c`.
/// Returns the last call's `eax` (the owner for a skipped call, the
/// operation for an unknown one), or the walk's.
pub fn vector_op<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let v = n + 0x1c;
    let op = u32::from(g.read::<u8>(n + 0x40));
    let owner = g.read::<u32>(n + 0x38);
    let mut eax = op;
    match op {
        0 | 6 => {
            let t = slot(g, img, f, if op == 0 { 0x71 } else { 0x145 }, 262);
            let model = g.read::<u32>(owner + 0x54);
            eax = g.call(t, 0, &[model, v, v + 4, v + 8]).eax;
            if op == 6 {
                let t = slot(g, img, f, 0x15a, 12);
                let m2 = g.read::<u32>(owner + 0x54);
                let out = g.call(t, 0, &[m2, model, v, v + 4, v + 8]);
                g.write::<u32>(v + 4, st0_f32(out));
                eax = out.eax;
            }
        }
        1 => {
            let t = slot(g, img, f, 0x94, 29);
            let model = g.read::<u32>(owner + 0x54);
            eax = g.call(t, 0, &[model, v]).eax;
        }
        2 => {
            let t = slot(g, img, f, 0xb0, 268);
            let model = g.read::<u32>(owner + 0x54);
            let x = g.read::<u32>(n + 0x3c);
            eax = g.call(t, 0, &[model, x, v]).eax;
        }
        3..=5 => {
            eax = owner;
            if g.read::<u8>(owner + 0x90) & 1 == 0 {
                let (off, s) = [(0xdf, 366), (0x104, 665), (0x129, 531)][op as usize - 3];
                let t = slot(g, img, f, off, s);
                let obj = g.read::<u32>(owner + 0x50);
                eax = g.call(t, 0, &[obj, v]).eax;
            }
        }
        _ => {}
    }
    for i in 0..3 {
        let x = add(f32_at(g, n + 0x2c + 4 * i), f32_at(g, n + 0x1c + 4 * i));
        g.write::<u32>(n + 0x1c + 4 * i, x.to_bits());
    }
    Ret::eax(eax)
}

/// `vector_op` (412 bytes and its 7-entry jump table, 116 overlays).
pub const VECTOR_OP: Sig = Sig {
    len: 440,
    hash: 0xaae2_eb66_3e66_51b2,
    calls: &[],
    imports: &[],
};

/// Walks a node's own ring with a cursor in the caller's frame (`cursor`
/// = the frame's local: the node object, byte `+0x5` the pass; `+0x4`
/// is the callee's): the object's handler `[[obj] + 0x1c]`, cdecl
/// `(cursor)`, with `stack` the words the original's frame already
/// holds above it (earlier pushes not yet popped). Returns the stack
/// with the cursor pushed.
fn sub_ring<G: Game>(g: &mut G, cursor: u32, obj: u32, pass: u8, mut stack: Vec<u32>) -> Vec<u32> {
    g.write::<u32>(cursor, obj);
    g.write::<u8>(cursor + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
    stack.insert(0, cursor);
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(handler, cursor, &stack);
    stack
}

/// 0x10005e80 (internal, `ecx` = the cursor; a /GS frame): the walk;
/// when the cursor moved, slot 248 combines the parent's (`+0x14`)
/// vector `+0x1c` with the node's into a local vector (`[ebp - 0x14]`);
/// when its height above the node's floor `+0x38` is over `K`, the node
/// bounces: the height over the node's `+0x20` (rate) gives `t`, the
/// rate becomes `-(+0x2c * rate)` and the local vector is rebuilt from
/// the node's `+0x1c`/`+0x24` scaled by `t` and `+0x30` (the bounce
/// count `+0x40` up). Slot 325 copies the local vector to the parent's,
/// the rate takes the gravity `+0x34`, and the frame count `+0x42` goes
/// up; reaching `+0x3c` sets `+0x44` to 1. Returns 1 then, else the
/// count plus 1, or the walk's `eax`.
pub fn bounce<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let vt = ebp.wrapping_sub(0x14);
    let n = g.read::<u32>(this);
    let p = g.read::<u32>(n + 0x14);
    let pv = p + 0x1c;
    let t = slot(g, img, f, 0x71, 248);
    g.call(t, 0, &[vt, pv, n + 0x1c]);
    let mut x2 = sub(f32_at(g, vt + 4), f32_at(g, n + 0x38));
    if x2 > f32_at(g, w(0x90)) {
        x2 = div(x2, f32_at(g, n + 0x20));
        let sign = g.read::<u32>(w(0xaf));
        let x0 = mul(f32_at(g, n + 0x2c), f32_at(g, n + 0x20));
        let x0 = f32::from_bits(x0.to_bits() ^ sign);
        g.write::<u32>(n + 0x20, x0.to_bits());
        let mut x6 = f32_at(g, n + 0x1c);
        let mut x5 = f32_at(g, n + 0x24);
        let mut x4 = mul(x6, x2);
        let x0 = mul(x0, x2);
        x6 = sub(x6, x4);
        x5 = mul(x5, x2);
        g.write::<u32>(vt, x6.to_bits());
        let x1 = sub(f32_at(g, n + 0x20), x0);
        g.write::<u32>(vt + 4, x1.to_bits());
        let x2b = sub(f32_at(g, n + 0x24), x5);
        g.write::<u32>(vt + 8, x2b.to_bits());
        let x3 = f32_at(g, n + 0x30);
        x4 = mul(x4, x3);
        x5 = mul(x5, x3);
        x4 = add(add(x4, f32_at(g, pv)), x6);
        g.write::<u32>(vt, x4.to_bits());
        let y = add(f32_at(g, n + 0x38), x1);
        g.write::<u32>(vt + 4, y.to_bits());
        x5 = add(add(x5, f32_at(g, p + 0x24)), x2b);
        g.write::<u32>(vt + 8, x5.to_bits());
        let a = mul(f32_at(g, n + 0x1c), x3);
        g.write::<u32>(n + 0x1c, a.to_bits());
        let c = mul(f32_at(g, n + 0x24), x3);
        g.write::<u32>(n + 0x24, c.to_bits());
        let k = g.read::<u16>(n + 0x40);
        g.write::<u16>(n + 0x40, k.wrapping_add(1));
    }
    let t = slot(g, img, f, 0x151, 325);
    g.call(t, 0, &[pv, vt]);
    let r = add(f32_at(g, n + 0x34), f32_at(g, n + 0x20));
    g.write::<u32>(n + 0x20, r.to_bits());
    let k = g.read::<u16>(n + 0x42).wrapping_add(1);
    g.write::<u16>(n + 0x42, k);
    let c = u32::from(k) + 1;
    if crate::sa_misc::cvt(c.cast_signed()) >= f32_at(g, n + 0x3c) {
        g.write::<u16>(n + 0x44, 1);
        return Ret::eax(1);
    }
    Ret::eax(c)
}

/// `bounce` (415 bytes, 107 overlays; the `rel32` is
/// `__security_check_cookie`).
pub const BOUNCE: Sig = Sig {
    len: 415,
    hash: 0xb1c3_9100_d47d_38f4,
    calls: &[(0x197, Callee::Any)],
    imports: &[],
};

/// 0x100040e0 (internal, `ecx` = the cursor; a /GS frame): the walk;
/// when the cursor moved, the rings of the node's objects `+0x20` and
/// `+0x1c` are walked with a cursor in the frame ([`sub_ring`]), slot
/// 229 fills a local vector (`[ebp - 0x14]`) from the objects' `+0x9c`
/// and `+0x1c`; then the node's flags `+0x2c`: with bit 0, the countdown
/// `+0x30` runs, and at 0 slot 220 (bit 1: target `+0x24`, `+0x34`,
/// `+0x28`) or slot 393 (`+0x24`, the vector, `+0x28`) fires and bit 0
/// is cleared; without bits 0 and 1, slot 408 (`+0x24`, the vector,
/// `+0x28`). Returns the last call's `eax` (the countdown when it runs),
/// or the walk's.
pub fn pair_aim<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let cursor = ebp.wrapping_sub(0x1c);
    let vt = ebp.wrapping_sub(0x14);
    let n = g.read::<u32>(this);
    let pass = g.read::<u8>(this + 5);
    let a = g.read::<u32>(n + 0x20);
    let stack = sub_ring(g, cursor, a, pass, Vec::new());
    let pass = g.read::<u8>(this + 5);
    let b = g.read::<u32>(n + 0x1c);
    let mut stack = sub_ring(g, cursor, b, pass, stack);
    let t = slot(g, img, f, 0xa2, 229);
    let a = g.read::<u32>(n + 0x20).wrapping_add(0x1c);
    let b = g.read::<u32>(n + 0x1c).wrapping_add(0x9c);
    stack.splice(0..0, [vt, b, a]);
    // call width: 5 (the earlier calls' words stay on the stack)
    let mut eax = g.call(t, 0, &stack).eax;
    let flags = g.read::<u32>(n + 0x2c);
    if flags & 1 != 0 {
        let c = g.read::<u32>(n + 0x30);
        g.write::<u32>(n + 0x30, c.wrapping_sub(1));
        if c != 0 {
            return Ret::eax(c.wrapping_sub(1));
        }
        let last = g.read::<u32>(n + 0x28);
        eax = if flags & 2 != 0 {
            let t = slot(g, img, f, 0xd2, 220);
            let (x, y) = (g.read::<u32>(n + 0x24), g.read::<u32>(n + 0x34));
            g.call(t, 0, &[x, y, last]).eax
        } else {
            let t = slot(g, img, f, 0xfd, 393);
            let x = g.read::<u32>(n + 0x24);
            g.call(t, 0, &[x, vt, last]).eax
        };
        let fl = g.read::<u32>(n + 0x2c);
        g.write::<u32>(n + 0x2c, fl & !1);
    } else if flags & 2 == 0 {
        let last = g.read::<u32>(n + 0x28);
        let t = slot(g, img, f, 0x130, 408);
        let x = g.read::<u32>(n + 0x24);
        eax = g.call(t, 0, &[x, vt, last]).eax;
    }
    Ret::eax(eax)
}

/// `pair_aim` (338 bytes, 107 overlays; the `rel32`s are
/// `__security_check_cookie`).
pub const PAIR_AIM: Sig = Sig {
    len: 338,
    hash: 0x525d_e023_88fd_51d4,
    calls: &[
        (0xf0, Callee::Any),
        (0x11b, Callee::Any),
        (0x14a, Callee::Any),
    ],
    imports: &[],
};

/// The operand offsets of [`bone`]'s two copies.
struct BoneOps {
    table: u32,
    /// The constant scaling the follow rate (`None`: not scaled).
    k1: Option<u32>,
    s233: [u32; 4],
    s325: u32,
    /// Root branch: slot 325, the constant scaling `+0x138`, two slot 230s.
    root: (u32, Option<u32>, u32, u32),
    /// Child branch: slot 668, the `+0x138` constant, two slot 231s, 229.
    child: (u32, Option<u32>, u32, u32, u32),
    /// The first final sum reads the node's word first (else the parent's).
    node_first: bool,
}

/// The bone update of [`BONE`] and [`BONE_SCALED`] (internal, `ecx` =
/// the cursor; a /GS frame): `p` = the node's parent bone `+0x128` (read
/// before the walk). The walk; when the cursor moved, the rings of the
/// node's three objects (`+0x13c`, `+0x144`, `+0x140`) and of the parent
/// are walked with a cursor in the frame; the DLL's function table
/// (by the node's byte `+0x148`) turns the node's `+0x5c` into the
/// target `+0x144`'s vector; slot 233 interpolates the local vector
/// (`[ebp - 0x14]`) towards the `+0x140` object by its rate `+0x28`
/// (scaled in the second copy) and the node's three vectors by it; slot
/// 325 moves the `+0x13c` object. A root bone (no parent) takes the
/// target's position into `+0x11c..+0x128`, and slot 325/230 its matrix
/// pieces; a child adds the parent's (integer words, as the original
/// does), slot 668/231/229 compose the matrices with the parent's and
/// `+0xcc..+0xd4` gets the parent's added. Returns the last call's
/// `eax` (a root) or the parent, or the walk's `eax`.
#[allow(clippy::too_many_lines)] // the original's two branches, step by step
fn bone<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &BoneOps) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<u32>(this);
    let p = g.read::<u32>(n + 0x128);
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let cursor = ebp.wrapping_sub(0x1c);
    let vt = ebp.wrapping_sub(0x14);
    let mut stack = Vec::new();
    for off in [0x13c, 0x144, 0x140] {
        let pass = g.read::<u8>(this + 5);
        let obj = g.read::<u32>(n + off);
        stack = sub_ring(g, cursor, obj, pass, stack);
    }
    let parent = g.read::<u32>(n + 0x128);
    if parent != 0 {
        let pass = g.read::<u8>(this + 5);
        sub_ring(g, cursor, parent, pass, Vec::new());
    }
    let target = g.read::<u32>(n + 0x144).wrapping_add(0x1c);
    let which = g.read::<u8>(n + 0x148);
    let func = g.read::<u32>(w(o.table).wrapping_add(4 * u32::from(which)));
    g.call(func, u32::from(which), &[n + 0x5c, target]);
    let src = g.read::<u32>(n + 0x140);
    let mut rate = f32_at(g, src + 0x28);
    if let Some(k) = o.k1 {
        rate = mul(rate, f32_at(g, w(k)));
    }
    let t = slot(g, img, f, o.s233[0], 233);
    g.call(t, 0, &[vt, src + 0x1c, rate.to_bits()]);
    for (i, (a, b)) in [(0x1c, 0x5c), (0x2c, 0x6c), (0x3c, 0x7c)]
        .into_iter()
        .enumerate()
    {
        let x = g.read::<u32>(vt + 4 * i as u32);
        let t = slot(g, img, f, o.s233[i + 1], 233);
        g.call(t, 0, &[n + a, n + b, x]);
    }
    let x = g.read::<u32>(vt + 8);
    let t = slot(g, img, f, o.s325, 325);
    let obj = g.read::<u32>(n + 0x13c).wrapping_add(0x1c);
    // slot 233's last words stay below
    g.call(t, 0, &[n + 0x4c, obj, n + 0x3c, n + 0x7c, x]);
    let scale_138 = |g: &mut G, k: Option<u32>| {
        if let Some(k) = k {
            let v = mul(f32_at(g, n + 0x138), f32_at(g, w(k)));
            g.write::<u32>(n + 0x138, v.to_bits());
        }
    };
    if p == 0 {
        let tgt = g.read::<u32>(n + 0x144);
        for i in 0..4 {
            let v = g.read::<u32>(tgt + 0x1c + 4 * i);
            g.write::<u32>(n + 0x11c + 4 * i, v);
        }
        let (s325, k2, a, b) = o.root;
        let t = slot(g, img, f, s325, 325);
        let src = g.read::<u32>(n + 0x140).wrapping_add(0x1c);
        g.call(t, 0, &[n + 0x12c, src]);
        scale_138(g, k2);
        let t = slot(g, img, f, a, 230);
        g.call(t, 0, &[n + 0xdc, n + 0x5c, n + 0x12c, src]);
        let t = slot(g, img, f, b, 230);
        let r = g.call(
            t,
            0,
            &[n + 0x9c, n + 0x1c, n + 0xdc, n + 0x5c, n + 0x12c, src],
        );
        return Ret::eax(r.eax);
    }
    for i in 0..3 {
        let tgt = g.read::<u32>(n + 0x144);
        let v = g.read::<u32>(tgt + 0x1c + 4 * i);
        let v = v.wrapping_add(g.read::<u32>(p + 0x11c + 4 * i));
        g.write::<u32>(n + 0x11c + 4 * i, v);
    }
    let (s668, k2, a, b, c) = o.child;
    let t = slot(g, img, f, s668, 668);
    let src = g.read::<u32>(n + 0x140).wrapping_add(0x1c);
    let mut stack = vec![n + 0x12c, p + 0x12c, src];
    // call width: 3 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    scale_138(g, k2);
    let t = slot(g, img, f, a, 231);
    stack.splice(0..0, [n + 0xdc, p + 0xdc, n + 0x5c]);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot(g, img, f, b, 231);
    stack.splice(0..0, [n + 0x9c, p + 0x9c, n + 0x1c]);
    // call width: 9 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot(g, img, f, c, 229);
    stack.splice(0..0, [n + 0xcc, p + 0xdc, n + 0x4c]);
    // call width: 12 (the three earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for i in 0..3 {
        let (a, b) = (f32_at(g, n + 0xcc + 4 * i), f32_at(g, p + 0xcc + 4 * i));
        // addss: the first operand's NaN wins, so the order is kept
        let (x, y) = if o.node_first && i == 0 {
            (a, b)
        } else {
            (b, a)
        };
        let v = add(x, y);
        g.write::<u32>(n + 0xcc + 4 * i, v.to_bits());
    }
    Ret::eax(p)
}

/// 0x10005820: [`bone`] with the rate as it is.
pub fn bone_plain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    bone(
        g,
        img,
        fr,
        &BoneOps {
            table: 0xed,
            k1: None,
            s233: [0x113, 0x132, 0x154, 0x176],
            s325: 0x18d,
            root: (0x1ce, None, 0x1eb, 0x203),
            child: (0x275, None, 0x291, 0x2b2, 0x2c3),
            node_first: false,
        },
    )
}

/// `bone_plain` (802 bytes, 107 overlays; the `rel32`s are
/// `__security_check_cookie`).
pub const BONE: Sig = Sig {
    len: 802,
    hash: 0x5df4_e7b5_f669_f1ee,
    calls: &[(0x21b, Callee::Any), (0x31a, Callee::Any)],
    imports: &[],
};

/// 0x10002b20: [`bone`] with the rate scaled by `K1` and `+0x138` by
/// `K2` before the matrices.
pub fn bone_scaled<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    bone(
        g,
        img,
        fr,
        &BoneOps {
            table: 0xed,
            k1: Some(0x109),
            s233: [0x11b, 0x13a, 0x15c, 0x17e],
            s325: 0x195,
            root: (0x1da, Some(0x1fa), 0x207, 0x227),
            child: (0x299, Some(0x2b1), 0x2c5, 0x2ee, 0x2ff),
            node_first: true,
        },
    )
}

/// `bone_scaled` (862 bytes, 105 overlays; the `rel32`s are
/// `__security_check_cookie`).
pub const BONE_SCALED: Sig = Sig {
    len: 862,
    hash: 0xd8c7_0b51_2f05_2921,
    calls: &[(0x23f, Callee::Any), (0x356, Callee::Any)],
    imports: &[],
};

/// 0x10006250 (internal, no arguments): resets three static nodes of the
/// DLL (A and C with the node table, B with its own; empty child lists,
/// pass bytes 0xff, A's scale 1.0, C's four words 1.0) and inserts them
/// under the root (`node_insert` with a group in this frame:
/// `[root, 0, 0, A, B, C]`). Returns `node_insert`'s `eax`.
pub fn statics_init<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u16>(w(0x0c), 0xff);
    for (dst, v) in [
        (0x14, w(0x18)),
        (0x1e, w(0x22)),
        (0x28, 0),
        (0x32, w(0x36)),
        (0x3c, 0),
        (0x46, 0),
        (0x50, 0),
        (0x5a, ONE),
        (0x64, w(0x68)),
    ] {
        g.write::<u32>(w(dst), v);
    }
    g.write::<u16>(w(0x6f), 0xff);
    for (dst, v) in [
        (0x77, w(0x7b)),
        (0x81, w(0x85)),
        (0x8b, 0),
        (0x95, 0),
        (0x9f, 0),
        (0xa9, 0),
    ] {
        g.write::<u32>(w(dst), v);
    }
    g.write::<u16>(w(0xb4), 0xff);
    for (dst, v) in [
        (0xbc, w(0xc0)),
        (0xc6, w(0xca)),
        (0xd0, 0),
        (0xda, w(0xde)),
        (0xe4, ONE),
        (0xee, ONE),
        (0xf8, ONE),
        (0x102, ONE),
    ] {
        g.write::<u32>(w(dst), v);
    }
    // push ebp; sub esp, 0x18: the group is at [ebp - 0x18]
    let group = esp.wrapping_sub(0x1c);
    for (i, v) in (0u32..).zip([w(0x10d), 0, 0, w(0x122), w(0x129), w(0x130)]) {
        g.write::<u32>(group + 4 * i, v);
    }
    let insert = img.rel32_target(f + 0x135);
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: insert,
            esp: esp.wrapping_sub(0x20),
            ecx: group,
            ..Frame::default()
        },
    )
    .eax
}

/// `statics_init` (317 bytes, 98 overlays).
pub const STATICS_INIT: Sig = Sig {
    len: 317,
    hash: 0x2e28_4a39_499a_33c8,
    calls: &[(0x135, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};
