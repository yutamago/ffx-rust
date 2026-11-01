//! `magic_0589.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the flash (0x100028b0, an overlay table entry, no arguments;
/// the record `R = *G_R`, operand `+0x13`): a fade task on `R + 0x1bb0`
/// ([`fade_task`](crate::s3_misc::fade_task): `edx` 1, `xmm1`/`xmm2` the
/// constants at `+0x0c`/`+0x04`, the stack word 0); the part `P = R +
/// 0x1a70` gets its colour (the original's overlapping byte, half and
/// word stores in order), kind `+0x98` 0x48, sizes `+0x8c`/`+0x90` 75,
/// `+0x88`/`+0x94` 0; a pool node ([`take`], the owner the immediate at
/// `+0x33`) holding it, put at the head of the list ([`push_head`]: head
/// `+0xa4`/`+0xae`, the empty test `+0x90`, set `+0xbd`/`+0xc2`).
/// Returns the node.
pub fn flash<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (x2, x1) = (g.read::<u32>(w(0x4)), g.read::<u32>(w(0xc)));
    let r = g.read::<u32>(w(0x13));
    // push esi; push 0; the call
    g.write::<u32>(fr.esp.wrapping_sub(8), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x25),
        esp: fr.esp.wrapping_sub(0xc),
        eax: fr.eax,
        ecx: r.wrapping_add(0x1bb0),
        edx: 1,
        xmm: [0, u128::from(x1), u128::from(x2), 0],
    };
    crate::s3_misc::fade_task(g, img, &callee);
    let p = r.wrapping_add(0x1a70);
    g.write::<u32>(p + 0x81, 0x0080_ffff);
    g.write::<u8>(p + 0x80, 0xff);
    g.write::<u16>(p + 0x86, 0);
    g.write::<u8>(p + 0x85, 0);
    g.write::<u32>(p + 0x88, 0);
    g.write::<u16>(p + 0x98, 0x48);
    g.write::<u32>(p + 0x90, 0x4b);
    g.write::<u32>(p + 0x8c, 0x4b);
    g.write::<u32>(p + 0x94, 0);
    let node = take(g, img, img.rel32_target(f + 0x8a), w(0x33));
    let empty = g.read::<u32>(w(0x90)) == 0;
    g.write::<u32>(node + 0x14, p);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0xa4), w(0xae)), (w(0xbd), w(0xc2)));
    Ret::eax(node)
}

/// [`flash`] (202 bytes, 1 overlay).
pub const FLASH: Sig = Sig {
    len: 202,
    hash: 0xa8cf_9104_f224_a718,
    calls: &[
        (0x25, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x8a, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// 0x10002980 (an overlay table entry):
/// [`two_parts`](crate::m0511::two_parts) on the parts `+0x1a70` and
/// `+0x1b10`, the second 30 frames.
pub fn two_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0511::two_parts(g, img, fr.at, (0x1a70, 0x1b10, 0x1e))
}

/// [`two_parts`] (322 bytes, 1 overlay).
pub const TWO_PARTS: Sig = Sig {
    len: 322,
    hash: 0x4b31_e8f2_69d6_9608,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Blends the summon's three colours (0x100047d0, no arguments): for
/// each of the three countdowns `S[0x2208 + 4j]` (`S = *G_S`; down by one
/// while above 0) the weight `t = n / D_j` blends the start colour
/// `S[0x1a40 + 0x10j]` (times `1 - t`) with the end colour `S[0x1a10 +
/// 0x10j]` (times `t`) into `S[0x1950 + 0x20j]`, its second component
/// copied unblended and its alpha 1.0 (each product and sum one `f32`
/// step, in the original's order). Returns the last countdown (`eax`).
pub fn blend<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x5));
    let fl = |g: &G, o: u32| g.read::<f32>(s + o);
    let one = g.read::<f32>(w(0x2c));
    let mut out = [[0u32; 4]; 3];
    let mut eax = 0;
    for (j, d) in (0u32..).zip([0x34u32, 0xac, 0x121]) {
        let c = s + 0x2208 + 4 * j;
        let n = g.read::<u32>(c);
        eax = n;
        if n.cast_signed() > 0 {
            eax = n - 1;
            g.write::<u32>(c, eax);
        }
        let t = div(cvt(g.read::<u32>(c)), g.read::<f32>(w(d)));
        let u = sub(one, t);
        let b = 0x10 * j;
        let x = add(mul(fl(g, 0x1a40 + b), u), mul(fl(g, 0x1a10 + b), t));
        let y = fl(g, 0x1a44 + b).to_bits();
        let z = add(mul(fl(g, 0x1a48 + b), u), mul(fl(g, 0x1a18 + b), t));
        out[j as usize] = [x.to_bits(), y, z.to_bits(), 0x3f80_0000];
    }
    for (j, v) in (0u32..).zip(out) {
        for (i, x) in (0u32..).zip(v) {
            g.write::<u32>(s + 0x1950 + 0x20 * j + 4 * i, x);
        }
    }
    Ret::eax(eax)
}

/// [`blend`] (481 bytes, 1 overlay).
pub const BLEND: Sig = Sig {
    len: 481,
    hash: 0x789a_5a2f_4b7e_37ed,
    calls: &[],
    imports: &[],
};
