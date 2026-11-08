//! `magic_0535.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{add, div, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The camera's framing step (0x10001000, `(seq)`, a sequence step; a
/// /GS frame): slot 519 `(0x48, 0, 6)`; the actor count `n = S[0x19c]`
/// (`S = *G_S`) kept; the centre `c` (slot 515's ST0) and the source
/// point (slot 474 `(S[0x198], 0x48, &V, &B)`, `V` = 0); for each actor
/// `i`: its height `slot 518 (i) + K1` raises the kept maximum (then also
/// `max / K2` truncated and slot 516 `(i)` kept), slot 514 `(slot 517
/// (i), c)` its distance `d` raises the frame's maximum `r`, slot 513
/// `(S[0x1a0 + 4i], 0, P + 16i)`; `r` at least `K3`; the DLL's range
/// words `(c, c + r, c - r)`, the 0x40-byte block zeroed (`memset`), its
/// words after `(0, 0, 1)`; slots 450 `(seq, NEXT)` and 449 `(seq,
/// END)`. Locals: `B` `ebp - 0x24`, `V` `- 0x14`. Returns slot 449's
/// `eax`.
///
/// `magic_0384`'s copy (0x10001000, 587 bytes) has its own kind (0x4a,
/// the imm8s at `+0x29` and `+0x88`) and `V`'s first word (-4.0, the
/// imm32 at `+0x6e`), and clears one more range word before the 1.0
/// (`mov dword, 0` at `+0x1f7`, ten bytes: the operands after it ten
/// bytes later).
pub fn frame_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let seq = fr.arg(g, 0);
    let kind = w(0x29) & 0xff;
    // the memset `call` (0xe8) at `+0x201`, or one more store first
    let e = if w(0x201) & 0xff == 0xe8 { 0 } else { 10 };
    let t = slot_target(g, w(0x1a), 519);
    let mut stack = vec![kind, 0, 6, 1];
    g.call(t, 0, &stack);
    let s = g.read::<u32>(w(0x2d));
    let n = g.read::<u32>(s + 0x19c);
    g.write::<u32>(w(0x3b), n);
    let t = slot_target(g, w(0x40), 515);
    g.write::<u32>(w(0x51), 0);
    let c = st0_f32(g.call(t, 0, &stack).st0);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, v) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let s = g.read::<u32>(w(0x67));
    g.write::<u32>(v, w(0x6e));
    for i in 1..4 {
        g.write::<u32>(v + 4 * i, 0);
    }
    let h = g.read::<u32>(s + 0x198);
    let t = slot_target(g, w(0x90), 474);
    stack.splice(0..0, [h, w(0x88) & 0xff, v, b]);
    // call width: 8
    g.call(t, 0, &stack);
    let mut r = 0.0f32;
    let k3 = if g.read::<i32>(w(0xa3)) <= 0 {
        k(g, 0x1ab)
    } else {
        let mut i = 0u32;
        loop {
            let t = slot_target(g, w(0xc1), 518);
            let a = st0_f32(g.call(t, 0, &[i]).st0);
            let x = add(a, k(g, 0xda));
            if x > g.read::<f32>(w(0xe4)) {
                g.write::<f32>(w(0xee), x);
                g.write::<u32>(w(0x100), trunc(div(x, k(g, 0xf6))));
                let t = slot_target(g, w(0x105), 516);
                let y = st0_f32(g.call(t, 0, &[i]).st0);
                g.write::<f32>(w(0x113), y);
            }
            let t = slot_target(g, w(0x11b), 517);
            let e = st0_f32(g.call(t, 0, &[i]).st0);
            let t = slot_target(g, w(0x131), 514);
            let d = st0_f32(g.call(t, 0, &[e.to_bits(), c.to_bits()]).st0);
            if d > r {
                r = d;
            }
            let s = g.read::<u32>(w(0x164));
            let a = g.read::<u32>(s + 0x1a0 + 4 * i);
            let t = slot_target(g, w(0x16f), 513);
            g.call(t, 0, &[a, 0, w(0xb5) + 16 * i]);
            i += 1;
            if i.cast_signed() >= g.read::<i32>(w(0x187)) {
                break;
            }
        }
        k(g, 0x19a)
    };
    // comiss + jbe: the bound only when it is the greater
    let r = if k3 > r { k3 } else { r };
    g.write::<f32>(w(0x1be), c);
    g.write::<f32>(w(0x1d7), add(c, r));
    g.write::<f32>(w(0x1df), sub(c, r));
    g.write::<u32>(w(0x1e5), 0);
    g.write::<u32>(w(0x1ef), 0);
    if e != 0 {
        g.write::<u32>(w(0x1f9), 0);
    }
    g.write::<u32>(w(0x1f9 + e), 0x3f80_0000);
    g.write_bytes(w(0x1cf), &[0u8; 0x40]);
    let t = slot_target(g, w(0x207 + e), 450);
    let mut stack = vec![seq, w(0x215 + e), w(0x1cf), 0, 0x40];
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x21d + e), 449);
    stack.splice(0..0, [seq, w(0x222 + e)]);
    // call width: 7
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`frame_step`] (577 bytes, 1 overlay).
pub const FRAME_STEP: Sig = Sig {
    len: 577,
    hash: 0x7193_7e0c_2198_52e2,
    calls: &[
        (0x202, Callee::Import("memset")),
        // __security_check_cookie
        (0x239, Callee::Any),
    ],
    imports: &[],
};

/// [`frame_step`] in `magic_0384` (0x10001000, 587 bytes: kind 0x4a,
/// one more range word cleared).
pub const FRAME_STEP_0384: Sig = Sig {
    len: 587,
    hash: 0x14cd_241e_e4c9_e8fe,
    calls: &[
        (0x20c, Callee::Import("memset")),
        // __security_check_cookie
        (0x243, Callee::Any),
    ],
    imports: &[],
};
