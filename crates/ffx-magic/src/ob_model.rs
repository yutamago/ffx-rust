//! Shapes the summon overlays' own code repeats with their own globals
//! (each function passes the operand offsets of its copy): the angle
//! wrapped once into -pi..pi, the characters read through the battle
//! selection, and a model matrix built from a character's scales and
//! three turns.

use crate::runtime::slot_target;
use crate::{Game, Image};

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
pub const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// ST0 of a call-out stored as `f32` (`fstp dword`).
#[must_use]
pub fn st0_f32(st0: Option<f64>) -> f32 {
    st0.map_or(INDEFINITE, |v| v as f32)
}

/// `x` wrapped once (the overlays' `comiss` pair, each `jbe` taken when
/// below, equal or unordered): `x - T` when `x > HI`, else `x + T2` when
/// `LO > x`. `ops` are the operand offsets of `HI`, `T`, `LO`, `T2` in
/// the code at `f`.
pub fn wrap_once<G: Game>(g: &G, img: &Image, f: u32, x: f32, ops: [u32; 4]) -> f32 {
    let k = |op: u32| g.read::<f32>(img.word(f + op));
    if x > k(ops[0]) {
        x - k(ops[1])
    } else if k(ops[2]) > x {
        x + k(ops[3])
    } else {
        x
    }
}

/// A character a function reads when it calls (operand offsets of the
/// code at `f`): `G_CHRS[(*G_SEL)[at]]` (a byte of the battle selection)
/// or the word at a global.
#[derive(Clone, Copy, Debug)]
pub enum Who {
    Sel { sel: u32, at: u32, table: u32 },
    Word(u32),
}

/// The character [`Who`] names, read now.
pub fn who<G: Game>(g: &G, img: &Image, f: u32, who: Who) -> u32 {
    let w = |off: u32| img.word(f + off);
    match who {
        Who::Sel { sel, at, table } => {
            let p = g.read::<u32>(w(sel)).wrapping_add(at);
            let k = u32::from(g.read::<u8>(p));
            g.read::<u32>(w(table).wrapping_add(4 * k))
        }
        Who::Word(op) => g.read::<u32>(w(op)),
    }
}

/// An angle argument of [`scale_model`]'s turns: a constant or the `f32`
/// at an address (read just before its call).
#[derive(Clone, Copy, Debug)]
pub enum Angle {
    Bits(u32),
    At(u32),
}

/// The operand offsets of one [`scale_model`] copy.
#[derive(Clone, Copy, Debug)]
pub struct ModelOps {
    pub s235: u32,
    /// Slot 277's character, and the scales' (slots 15..17).
    pub c277: Who,
    pub s277: u32,
    /// The factor `K` (0.001) of `Ch_GetDataScale`.
    pub k: u32,
    pub c: Who,
    pub s15: u32,
    pub s16: u32,
    pub s17: u32,
    pub s330: u32,
    pub s328: u32,
    pub s329: u32,
}

/// A scaled unit matrix at `m` (each call sees `base`, the words the
/// caller's earlier calls left on the stack, below its own): a unit
/// matrix (slot 235), its diagonal `Ch_GetDataScale(c277) * K` (slot
/// 277) times the character's scales `Ch_GetScaleX..Z(c)` (slots 15, 16,
/// 17). Returns the words left on the stack and slot 17's `eax`.
pub fn scale_unit<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: &ModelOps,
    m: u32,
    base: &[u32],
) -> (Vec<u32>, u32) {
    let w = |off: u32| img.word(f + off);
    let mut stack = vec![m];
    stack.extend_from_slice(base);
    let s235 = slot_target(g, w(o.s235), 235);
    g.call(s235, 0, &stack);
    let c277 = who(g, img, f, o.c277);
    let s277 = slot_target(g, w(o.s277), 277);
    stack.insert(0, c277);
    // call width: 14 at most (the earlier calls' words stay on the stack)
    let s = st0_f32(g.call(s277, 0, &stack).st0) * g.read::<f32>(w(o.k));
    g.write::<f32>(m + 0x28, s);
    g.write::<f32>(m + 0x14, s);
    g.write::<f32>(m, s);
    let c = who(g, img, f, o.c);
    let mut eax = 0;
    for (k, (slot, op)) in [(15u32, o.s15), (16, o.s16), (17, o.s17)]
        .into_iter()
        .enumerate()
    {
        let target = slot_target(g, w(op), slot);
        stack.insert(0, c);
        // call width: 14 at most (the earlier calls' words stay on the stack)
        let out = g.call(target, 0, &stack);
        eax = out.eax;
        let r = st0_f32(out.st0);
        let at = m + 0x14 * k as u32;
        let x = g.read::<f32>(at);
        // (the copies' `mulss` operand orders differ; the product is
        // the same but for two NaN payloads, and LLVM treats `fmul` as
        // commutative anyway)
        let v = x * r;
        g.write::<f32>(at, v);
    }
    (stack, eax)
}

/// A model matrix at `m`: [`scale_unit`], turned by Z, X and Y (slots
/// 330, 328, 329 `(m, m, angle)`; the stack's last word becomes each
/// turn's angle argument). Returns slot 329's `eax`.
pub fn scale_model<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: &ModelOps,
    m: u32,
    base: &[u32],
    turns: [Angle; 3],
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let _ = scale_unit(g, img, f, o, m, base);
    let mut eax = 0;
    for ((slot, op), a) in [(330, o.s330), (328, o.s328), (329, o.s329)]
        .into_iter()
        .zip(turns)
    {
        let a = match a {
            Angle::Bits(b) => b,
            Angle::At(p) => g.read::<u32>(p),
        };
        let target = slot_target(g, w(op), slot);
        eax = g.call(target, 0, &[m, m, a]).eax;
    }
    eax
}

/// The operand offsets of a [`root_matrix`] copy.
#[derive(Clone, Copy, Debug)]
pub struct RootOps {
    pub s210: u32,
    /// `G_OFF`: the battle characters are not to be moved.
    pub off: u32,
    pub s217: u32,
    pub s255: u32,
    pub s211: u32,
}

/// A model's root (`chr`, the matrix at `m`; `pos` the caller's local
/// the battle character's position goes through): the root matrix read
/// (slot 210 `(chr, m)`) or, for a null matrix, the battle character
/// `index()` (below 32, when `*G_OFF` is 0) moved to its own position
/// (slots 217 `(i, pos)` and 255 `(i, pos)`); then the original matrix
/// set (slot 211 `(chr, m)`). Whether the battle character was moved,
/// and slot 211's `eax`.
pub fn root_matrix<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    o: &RootOps,
    (chr, m, pos): (u32, u32, u32),
    index: impl FnOnce(&mut G) -> u32,
) -> (bool, u32) {
    let w = |off: u32| img.word(f + off);
    let mut moved = false;
    if m == 0 {
        if g.read::<u32>(w(o.off)) == 0 {
            let i = index(g);
            if i.cast_signed() < 0x20 {
                let s217 = slot_target(g, w(o.s217), 217);
                g.call(s217, 0, &[i, pos]);
                let s255 = slot_target(g, w(o.s255), 255);
                g.call(s255, 0, &[i, pos, i, pos]);
                moved = true;
            }
        }
    } else {
        let s210 = slot_target(g, w(o.s210), 210);
        g.call(s210, 0, &[chr, m]);
    }
    let s211 = slot_target(g, w(o.s211), 211);
    (moved, g.call(s211, 0, &[chr, m]).eax)
}
