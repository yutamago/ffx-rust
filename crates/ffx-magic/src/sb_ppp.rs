//! The node family's particle nodes (`magic_0003`'s code, about 95 to
//! 116 DLLs), ported by the shared-B list: a node holding a particle part
//! (`pppStartPart` buffer at `+0x60`) placed and run each frame. Each doc
//! names the address, size and how many DLLs carry the function.

use crate::nodes::{FREE_THEN_UNLINK, free_then_unlink, walk};
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Sig};
use ffx_bridge::x87::cvttsd2si;

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// The walk (from the frame cookie's `eax`, as in the other framed
/// walks), then, unless the node's flags (`+0xe8`) have bit 1: while its
/// delay (`+0xec`) is positive it counts down (unless the DLL's pause
/// word, through operand `+0x75`, is set); otherwise, when not paused and
/// flag bit 0 is set, the colour (`+0xd0`, four halfwords) is
/// interpolated between `+0x3c..` and `+0x2c..` by `+0xe4 / +0xe6`
/// (scaled by the constant at `+0xfe`, `cvttss2si`) and `+0xe4` counts
/// down (bit 0 cleared at 0). Then the part is placed and run: a unit
/// colour into the DLL's colour block (`+0x17e`), the node's position
/// (`+0x1c`) copied to the frame (`esp - 0x18`), the model matrix (the
/// `+0x4c` node's `+0x9c`) into the DLL's matrix (`+0x1c2`) with its rows
/// scaled by the position, the position scaled by the `+0x4c` node's
/// `+0x138` and multiplied by its `+0x12c` into the node's `+0xc0`
/// (`w` = 1.0), the DLL's matrices combined (slots 231, 230), two words
/// handed over (`+0x2e0`/`+0x2e5`, `+0x2ec`), and `pppRunPart(+0x60,
/// 0x30)` (slot 712) with the game lock (`+0x2f6`) set; when it says done
/// the node is freed and unlinked (the `rel32` at `+0x328`, the cursor a
/// frame word at `esp - 0x1c`) and flag bit 2 set. The handed-over word
/// comes back (`+0x334` -> `+0x33b`), and unless paused a user function
/// (`+0xe0`) runs through slot 160 `(+0x60, fn, +0xf4)`. Returns the last
/// `eax` the original leaves.
///
/// `0x10003660` (892 bytes, thiscall `ecx` = the cursor, 95 DLLs).
#[allow(clippy::too_many_lines)]
pub fn run_part<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let eax = g.read::<u32>(w(0x07)) ^ ebp;
    let (eax, tail) = walk(g, this, this, eax);
    let Some(o) = tail else {
        return eax;
    };
    let flags = g.read::<u32>(o + 0xe8);
    if flags & 2 != 0 {
        return eax;
    }
    let delay = g.read::<u32>(o + 0xec);
    let pause = g.read::<u32>(w(0x75));
    if delay.cast_signed() > 0 {
        if g.read::<u32>(pause) != 0 {
            return pause;
        }
        let d = delay.wrapping_sub(1);
        g.write::<u32>(o + 0xec, d);
        return d;
    }
    if g.read::<u32>(pause) == 0 && flags & 1 != 0 {
        let t = g.read::<u16>(o + 0xe4).cast_signed();
        let n = g.read::<u16>(o + 0xe6).cast_signed();
        let k = f32_at(g, w(0xfe));
        let r = div(f32::from(t), f32::from(n));
        let mut c = [0u32; 4];
        for (i, cv) in (0u32..).zip(&mut c) {
            let a = f32_at(g, o + 0x2c + 4 * i);
            let b = f32_at(g, o + 0x3c + 4 * i);
            let x = mul(add(mul(sub(a, b), r), f32_at(g, o + 0x3c + 4 * i)), k);
            *cv = cvttsd2si(f64::from(x));
        }
        for (i, cv) in (0u32..).zip(c) {
            g.write::<u16>(o + 0xd0 + 2 * i, cv as u16);
        }
        let e4 = u32::from(g.read::<u16>(o + 0xe4));
        if e4 == 0 {
            let fl = g.read::<u32>(o + 0xe8);
            g.write::<u32>(o + 0xe8, fl & !1);
        }
        g.write::<u16>(o + 0xe4, e4.wrapping_sub(1) as u16);
    }
    let unit = g.read::<u32>(w(0x17e));
    for i in 0..4 {
        g.write::<u32>(unit + 4 * i, 0x3f80_0000);
    }
    let vec = ebp.wrapping_sub(0x14);
    let s325 = slot_target(g, w(0x1a6), 325);
    g.call(s325, this, &[vec, o + 0x1c]);
    let src = g.read::<u32>(o + 0x4c) + 0x9c;
    let m = g.read::<u32>(w(0x1c2));
    let s230 = slot_target(g, w(0x1bc), 230);
    g.call(s230, this, &[m, src]);
    for (k, (mo, to)) in [(0x1cf, 0x1e3), (0x1f0, 0x207), (0x214, 0x22b)]
        .into_iter()
        .enumerate()
    {
        let row = g.read::<u32>(w(mo)) + 0x10 * k as u32;
        let s = g.read::<u32>(vec + 4 * k as u32);
        let s233 = slot_target(g, w(to), 233);
        g.call(s233, this, &[row, row, s]);
    }
    let n = g.read::<u32>(o + 0x4c);
    let scale = f32_at(g, n + 0x138);
    for i in 0..3 {
        let x = mul(f32_at(g, vec + 4 * i), scale);
        set_f32(g, vec + 4 * i, x);
    }
    let n = g.read::<u32>(o + 0x4c);
    let s668 = slot_target(g, w(0x27b), 668);
    g.call(s668, this, &[vec, vec, n + 0x12c]);
    g.write::<u32>(vec + 0xc, 0x3f80_0000);
    let s325 = slot_target(g, w(0x293), 325);
    g.call(s325, this, &[o + 0xc0, vec]);
    let a = g.read::<u32>(w(0x2bf));
    let b = g.read::<u32>(w(0x2b3));
    let m = g.read::<u32>(w(0x2a8));
    let s231 = slot_target(g, w(0x2ad), 231);
    g.call(s231, this, &[a, b, m]);
    let c = g.read::<u32>(w(0x2d2));
    let a = g.read::<u32>(w(0x2c7));
    let s230 = slot_target(g, w(0x2cc), 230);
    g.call(s230, this, &[c, a]);
    let to = g.read::<u32>(w(0x2e0));
    g.write::<u32>(to, g.read::<u32>(w(0x2e5)));
    let p = g.read::<u32>(w(0x2ec));
    g.write::<u32>(p, g.read::<u32>(o + 0x50));
    let lock = g.read::<u32>(w(0x2f6));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s712 = slot_target(g, w(0x305), 712);
    let done = g.call(s712, this, &[o + 0x60, 0x30]).eax;
    let lock = g.read::<u32>(w(0x314));
    g.write::<u32>(lock, saved);
    if done != 0 {
        let cursor = ebp.wrapping_sub(0x18);
        g.write::<u32>(cursor, o);
        free_then_unlink(
            g,
            img,
            &Frame {
                at: img.rel32_target(f + 0x328),
                esp: esp.wrapping_sub(0x2c),
                ecx: cursor,
                ..Frame::default()
            },
        );
        let fl = g.read::<u32>(o + 0xe8);
        g.write::<u32>(o + 0xe8, fl | 4);
    }
    let v = g.read::<u32>(g.read::<u32>(w(0x334)));
    g.write::<u32>(w(0x33b), v);
    let pause = g.read::<u32>(w(0x340));
    if g.read::<u32>(pause) != 0 {
        return pause;
    }
    let func = g.read::<u32>(o + 0xe0);
    if func == 0 {
        return 0;
    }
    let s160 = slot_target(g, w(0x35b), 160);
    let arg = g.read::<u32>(o + 0xf4);
    g.call(s160, this, &[o + 0x60, func, arg]).eax
}

/// [`run_part`].
pub const RUN_PART: Sig = Sig {
    len: 892,
    hash: 0xdd16_d06d_6a38_2e6c,
    calls: &[
        // __security_check_cookie
        (0x98, Callee::Any),
        (0x328, Callee::Sig(&FREE_THEN_UNLINK)),
        (0x374, Callee::Any),
    ],
    imports: &[],
};
