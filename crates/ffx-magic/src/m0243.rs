//! `magic_0243.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Frame, Game, Image, Ret, Sig};

/// One wrap of an angle: above `hi` it loses `span`, else below `lo`
/// (`lo > x`) it gains it; unordered values stay (`comiss; jbe`).
fn wrap(x: f32, hi: f32, lo: f32, span: f32) -> f32 {
    if x > hi {
        sub(x, span)
    } else if lo > x {
        add(x, span)
    } else {
        x
    }
}

/// The parts' facing (0x10001d50, no arguments): for each of the `*N`
/// (`+0x09`, read again after each) parts (from `+0x15`, 0x130 bytes
/// apart) `K - f` with `f` slot 333 `(*X)`'s float (`X` `+0x27`, `K`
/// `+0x3a`), wrapped into range (`HI` `+0x47`, `LO` `+0x4f`, `SPAN`
/// `+0x57`), moved by `HI` and wrapped again, into the part's first word.
/// Returns the last slot call's `eax` (the caller's with no parts).
pub fn facing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut eax = fr.eax;
    let n = |g: &G| g.read::<i32>(w(0x09));
    if n(g) <= 0 {
        return Ret::eax(eax);
    }
    let mut part = w(0x15);
    let mut i = 0;
    loop {
        let s = slot_target(g, w(0x21), 0x534 / 4);
        let x = g.read::<u32>(w(0x27));
        let out = g.call(s, 0, &[x]);
        eax = out.eax;
        let f = out.st0.unwrap_or(0.0) as f32;
        let a = sub(g.read::<f32>(w(0x3a)), f);
        let hi = g.read::<f32>(w(0x47));
        let lo = g.read::<f32>(w(0x4f));
        let span = g.read::<f32>(w(0x57));
        let a = wrap(add(wrap(a, hi, lo, span), hi), hi, lo, span);
        g.write::<f32>(part, a);
        part = part.wrapping_add(0x130);
        i += 1;
        if i >= n(g) {
            break;
        }
    }
    Ret::eax(eax)
}

/// [`facing`] (163 bytes, 1 overlay).
pub const FACING: Sig = Sig {
    len: 163,
    hash: 0x1c2c_fb08_8dca_c170,
    calls: &[],
    imports: &[],
};

/// An object's facing (0x10001e00, cdecl `(obj)`): nothing while its
/// frame count `+0x118` is not 0 (the caller's `eax`); else slot 237
/// `(*X, 6, 0, obj + 0xd0)` (`X` `+0x2d`) and `+0xdc` = 1.0, then `K - f`
/// with `f` slot 333 `(*X)`'s float (stored over the argument word),
/// wrapped once (`HI` `+0x64`, `LO` `+0x78`, `SPAN` `+0x6e`/`+0x85`) into
/// `+0xc4`, and `+0xe8` = -30.0. Returns slot 333's `eax`. Each slot call
/// passes the words the original leaves on the stack.
pub fn object_facing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    if g.read::<u32>(obj + 0x118) != 0 {
        return Ret::eax(fr.eax);
    }
    let s = slot_target(g, w(0x15), 0x3b4 / 4);
    let v = obj + 0xd0;
    let x = g.read::<u32>(w(0x2d));
    g.call(s, 0, &[x, 6, 0, v]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let s = slot_target(g, w(0x34), 0x534 / 4);
    let x2 = g.read::<u32>(w(0x41));
    let out = g.call(s, 0, &[x2, x, 6, 0, v]);
    let f = out.st0.unwrap_or(0.0) as f32;
    // fstp [ebp + 8]: the float over the caller's argument word
    g.write::<f32>(fr.esp + 4, f);
    let a = sub(g.read::<f32>(w(0x54)), f);
    let a = if a > g.read::<f32>(w(0x64)) {
        sub(a, g.read::<f32>(w(0x6e)))
    } else if g.read::<f32>(w(0x78)) > a {
        add(a, g.read::<f32>(w(0x85)))
    } else {
        a
    };
    g.write::<f32>(obj + 0xc4, a);
    g.write::<u32>(obj + 0xe8, 0xc1f0_0000);
    Ret::eax(out.eax)
}

/// [`object_facing`] (158 bytes, 1 overlay).
pub const OBJECT_FACING: Sig = Sig {
    len: 158,
    hash: 0x5184_cdc7_9ea6_c702,
    calls: &[],
    imports: &[],
};
