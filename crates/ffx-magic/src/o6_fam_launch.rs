//! Round 6 shape families: a particle's random launch (family
//! `F0384_100015e0` of `~/ffx-work/m5/magic/r4/families.tsv`; 3 copies,
//! 883 bytes: `magic_0384` 0x100015e0, `magic_0421` 0x100015e0,
//! `magic_0535` 0x100015c0). The copies differ only in their globals,
//! one start word and one pushed immediate, all read from the code
//! (`w(off)` = the word at `+off`). SSE arithmetic in the original's
//! order through [`crate::sb_sse`]; the sine and cosine are the game's
//! MSVCR110 results.

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Game, Image, Libm, Sig};

/// `-x` as `xorps` with the sign mask at `mask` (its low word).
fn negate<G: Game>(g: &G, x: f32, mask: u32) -> f32 {
    f32::from_bits(x.to_bits() ^ g.read::<u32>(mask))
}

/// Launches a particle (internal: `ecx` = the particle `p`, `edx` a
/// count `n`, `xmm1` an angle `a`; a /GS frame of 0x4c bytes, `ebp` =
/// `esp - 4` at the entry). With `R()` = `rand()` and the constants
/// `K..` read where the code names them:
/// - with `n != 0` the angle moves by `R() / K1 * K2 - K3`;
/// - `B = -(sin a * S)`, `C = -(cos a * S)` (`S` the DLL's scale
///   global), both scaled by `q = K4 / N` (`N` the integer count
///   global); `A = G2 * q`; `D` is the stack word at `ebp - 0x1c`, which
///   the original reads before writing it (junk read: 0), times `q`;
/// - slot 474 `(w, K, &v0, &v1)` (`w` the word `(*G_D)[0x198]`, `K` the
///   pushed immediate, `v0 = (V, 0, 0, 0)` at `ebp - 0x28`, `v1` at
///   `ebp - 0x18` the slot's result);
/// - `E = R() / K1 * K5`; `F = (R() / K1 * K6 + K7) * K8` and `H = ((R()
///   / K1 * K6 + K7) * K9 - F) / N`; `m = n * K8`; `v1 += (B, A, C, D) *
///   m`;
/// - `p + 0xbc |= 0x4000`; `I = R() / K1 * K10 / N`; the particle's words
///   from those values (`+0xd0..` `v1`, `+0xe0` `B`, `+0xe4` `A - I`,
///   `+0xe8` `C`, `+0x108` `E`, `+0x130..` `F`, `+0x140..` `H`, constants
///   for `+0x170..+0x17c`, the rest 0);
/// - `p + 16 * (p[0xac] + 3)` = `n`; slot 458 `(*G_D, p, 0)` (the
///   earlier call's words below them).
///
/// Returns slot 458's `eax` (the cookie check leaves it).
pub fn random_launch<G: Game>(g: &mut G, img: &Image, f: u32, fr: &crate::Frame) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let (p, n) = (fr.ecx, fr.edx);
    let mut a = fr.xmm_f32(1);
    let ebp = fr.esp.wrapping_sub(4);
    let (v0, v1) = (ebp.wrapping_sub(0x28), ebp.wrapping_sub(0x18));
    let mask = w(0x6e);
    let rand_f = |g: &mut G| cvt(g.rand());
    if n != 0 {
        let r = rand_f(g);
        let step = sub(mul(div(r, k(g, 0x38)), k(g, 0x40)), k(g, 0x48));
        a = add(step, a);
    }
    let scale = w(0x67);
    let s = g.libm(Libm::Sin, f64::from(a)) as f32;
    let b = negate(g, mul(s, g.read::<f32>(scale)), mask);
    let c = g.libm(Libm::Cos, f64::from(a)) as f32;
    let count = w(0x97);
    let q = div(k(g, 0x88), cvt(g.read::<u32>(count)));
    let c = negate(g, mul(c, g.read::<f32>(w(0xa2))), w(0xb2));
    let b = mul(b, q);
    let big_a = mul(k(g, 0xc7), q);
    let gd = w(0xd4);
    // junk read: [ebp-0x1c] never written; 0 (brief 2026-10-24)
    let d_junk = 0.0f32;
    for (i, off) in (0u32..).zip([0xe5u32, 0xec, 0xf3, 0xfa]) {
        g.write::<u32>(v0.wrapping_add(4 * i), w(off));
    }
    let word = g.read::<u32>(g.read::<u32>(gd).wrapping_add(w(0x102)));
    let imm = disp8(img, f + 0xff);
    let c = mul(c, q);
    let d = mul(d_junk, q);
    let mut stack = vec![word, imm, v0, v1];
    let s474 = slot_target(g, w(0x107), w(0x111) / 4);
    // call width: 4
    g.call(s474, p, &stack);
    let r = rand_f(g);
    let e = mul(div(r, k(g, 0x132)), k(g, 0x13a));
    let r = rand_f(g);
    let x = add(mul(div(r, k(g, 0x155)), k(g, 0x162)), k(g, 0x16f));
    let big_f = mul(x, k(g, 0x17a));
    let h = div(
        sub(mul(x, k(g, 0x182)), big_f),
        cvt(g.read::<u32>(w(0x193))),
    );
    let m = mul(cvt(n), k(g, 0x1af));
    for (i, t) in (0u32..).zip([mul(b, m), mul(big_a, m), mul(c, m), mul(d, m)]) {
        let at = v1.wrapping_add(4 * i);
        let v = add(g.read::<f32>(at), t);
        g.write::<f32>(at, v);
    }
    let r = rand_f(g);
    let x = div(r, k(g, 0x215));
    let flags = p.wrapping_add(w(0x21b));
    let fl = g.read::<u32>(flags);
    g.write::<u32>(flags, fl | w(0x21f));
    let x = mul(x, k(g, 0x227));
    let i_term = div(x, cvt(g.read::<u32>(w(0x20d))));
    for (i, o) in (0u32..).zip([0xd0u32, 0xd4, 0xd8]) {
        let v = g.read::<u32>(v1.wrapping_add(4 * i));
        g.write::<u32>(p.wrapping_add(o), v);
    }
    g.write::<f32>(p.wrapping_add(0xe0), b);
    g.write::<f32>(p.wrapping_add(0xe8), c);
    g.write::<f32>(p.wrapping_add(0x108), e);
    let a_minus = sub(big_a, i_term);
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(p.wrapping_add(o), big_f);
    }
    g.write::<u32>(p.wrapping_add(0xdc), 0);
    g.write::<f32>(p.wrapping_add(0xe4), a_minus);
    for o in [0xecu32, 0x100, 0x104, 0x10c, 0x13c] {
        g.write::<u32>(p.wrapping_add(o), 0);
    }
    for o in [0x140u32, 0x144, 0x148] {
        g.write::<f32>(p.wrapping_add(o), h);
    }
    g.write::<u32>(p.wrapping_add(0x14c), 0);
    for (o, at) in [
        (0x170u32, 0x31a),
        (0x174, 0x324),
        (0x178, 0x32e),
        (0x17c, 0x338),
    ] {
        g.write::<u32>(p.wrapping_add(o), w(at));
    }
    let slot = g.read::<u32>(p.wrapping_add(0xac));
    let at = p.wrapping_add(slot.wrapping_add(3).wrapping_mul(16));
    g.write::<u32>(at, n);
    let first = g.read::<u32>(w(0x354));
    stack.splice(0..0, [first, p, 0]);
    let s458 = slot_target(g, w(0x34d), w(0x35a) / 4);
    // call width: 7 (the earlier call's words stay on the stack)
    g.call(s458, p, &stack).eax
}

include!("o6_fam_launch_table.rs.in");
