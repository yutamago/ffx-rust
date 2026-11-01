//! `magic_0164.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The parts' mean point (0x10007460, internal: `ecx` = a record `[fx,
/// out, model]`; a /GS frame of 0x34 bytes). With the effect's flag
/// byte `+0x1640` bit 3 set: slot 234 `(out, fx + 0xac, model)` alone.
/// Else slot 325 `(&B, model)` gives the base `B` (`ebp - 0x24`), and
/// for each of the `n` (`+0x162c`) parts (records of 0x274 bytes from
/// `+0x110`, their model the first word) slot 268 `(model, 0x11, &P)`
/// gives a point `P` (`ebp - 0x14`, its fourth word set to 1.0 after);
/// `P - B` is stored back into `P` and summed. `out` gets `B + sum * (K
/// / n)` (`K` at `+0x11d`; `n` as a float, so no parts gives `0 * inf`)
/// and 1.0. Returns `out` (read again at the end).
pub fn mean_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let rec = fr.ecx;
    let fx = g.read::<u32>(rec);
    let model = g.read::<u32>(rec + 8);
    let tail = |g: &mut G| {
        let out = g.read::<u32>(rec + 4);
        g.write::<u32>(out + 0xc, 0x3f80_0000);
        out
    };
    if g.read::<u8>(fx + 0x1640) & 8 != 0 {
        let s = slot_target(g, w(0x2d), 0x3a8 / 4);
        let out = g.read::<u32>(rec + 4);
        g.call(s, 0, &[out, fx + 0xac, model]);
        return tail(g);
    }
    let base = ebp.wrapping_sub(0x24);
    let point = ebp.wrapping_sub(0x14);
    let s = slot_target(g, w(0x49), 0x514 / 4);
    g.call(s, 0, &[base, model]);
    let mut sum = [0f32; 3];
    // the count is read again at each test and for the scale
    let count = |g: &G| g.read::<i32>(fx + 0x162c);
    if count(g) > 0 {
        let mut part = fx + 0x110;
        let mut i = 0;
        loop {
            let s = slot_target(g, w(0x8b), 0x430 / 4);
            let m = g.read::<u32>(part);
            g.call(s, 0, &[m, 0x11, point]);
            for (k, total) in (0u32..).zip(sum.iter_mut()) {
                let d = sub(g.read::<f32>(point + 4 * k), g.read::<f32>(base + 4 * k));
                g.write::<f32>(point + 4 * k, d);
                *total = add(d, *total);
            }
            g.write::<u32>(point + 0xc, 0x3f80_0000);
            part = part.wrapping_add(0x274);
            i += 1;
            if i >= count(g) {
                break;
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let scale = div(g.read::<f32>(w(0x11d)), count(g) as f32);
    let out = g.read::<u32>(rec + 4);
    let x = add(g.read::<f32>(base), mul(sum[0], scale));
    g.write::<f32>(point + 4, mul(sum[1], scale));
    g.write::<f32>(point + 8, mul(sum[2], scale));
    g.write::<f32>(out, x);
    for k in 1..3u32 {
        let v = add(g.read::<f32>(base + 4 * k), g.read::<f32>(point + 4 * k));
        g.write::<f32>(out + 4 * k, v);
    }
    tail(g)
}

/// [`mean_point`] (385 bytes, 1 overlay).
pub const MEAN_POINT: Sig = Sig {
    len: 385,
    hash: 0x5b04_242a_4f94_6522,
    calls: &[
        // __security_check_cookie
        (0x179, Callee::Any),
    ],
    imports: &[],
};

/// The summon's frame (0x1000c190, a table entry, no arguments, an
/// aligned frame; `m0070::summon_frame_376`'s kin): 0 while the phase
/// word (`+0x0b`) is below 4; else the pointer global (`+0x18`) gets its
/// block (`+0x1e`), the models are bound
/// ([`bind_models`](crate::sa_summon::bind_models)), the pass byte `*P`
/// (`+0x28`) goes up by one, and the node `+0x3b` is visited through
/// [`bone_plain`](crate::sa_summon::bone_plain) with a cursor on the
/// frame (that pass). Unless the flag (`+0x4a`) is set, every child of
/// the root (`+0x59`, the first child `+0x51`) gets its method `+0x20`
/// with the cursor; when the first child is then `X` (`+0x88`) the done
/// bit 0 of `+0x90` is set. The static nodes `+0xa9` and `+0xc9` are
/// visited through [`visit_gated`](crate::nodes::visit_gated) with the
/// pass `*P`. When bit 0 of `*A & *B` (`+0xd3`, `+0xd9`) is set and the
/// halfword `+0xe7` is 0, slot 214 `(*C)` (`+0xf5`). Returns 0.
pub fn summon_frame_214<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<i32>(w(0x0b)) < 4 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(w(0x18));
    g.write::<u32>(p, w(0x1e));
    crate::sa_summon::bind_models(g, img, img.rel32_target(f + 0x23));
    // push ebp; and esp, -8; sub esp, 0xc; push esi: the cursor at
    // [esp + 8]
    let e = fr.esp.wrapping_sub(4) & !7;
    let cursor = e.wrapping_sub(8);
    let pass = g.read::<u8>(w(0x28)).wrapping_add(1);
    g.write::<u8>(w(0x33), pass);
    g.write::<u32>(cursor, w(0x3b));
    g.write::<u8>(cursor + 5, pass);
    let bone = Frame {
        at: img.rel32_target(f + 0x44),
        esp: e.wrapping_sub(0x14),
        ecx: cursor,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bone);
    let root = w(0x59);
    let first = g.read::<u32>(w(0x51));
    let done = if g.read::<u32>(w(0x4a)) != 0 {
        first == w(0x88)
    } else if first == root {
        true
    } else {
        let mut child = first;
        loop {
            g.write::<u32>(cursor, child);
            let method = g.read::<u32>(g.read::<u32>(child) + 0x20);
            child = g.read::<u32>(child + 8);
            // push ecx: the cursor's address, then the call
            g.write::<u32>(e.wrapping_sub(0x14), cursor);
            g.call(method, cursor, &[cursor]);
            if child == w(0x7a) {
                break;
            }
        }
        g.read::<u32>(w(0x82)) == w(0x88)
    };
    if done {
        let v = g.read::<u32>(w(0x90));
        g.write::<u32>(w(0x90), v | 1);
    }
    for (pass_at, node_at, call_at) in [(0x98u32, 0xa9u32, 0xaeu32), (0xb5, 0xc9, 0xce)] {
        let pass = g.read::<u8>(w(pass_at));
        g.write::<u8>(cursor + 5, pass);
        // push eax: the cursor's address
        g.write::<u32>(e.wrapping_sub(0x14), cursor);
        g.write::<u32>(cursor, w(node_at));
        let fr = Frame {
            at: img.rel32_target(f + call_at),
            esp: e.wrapping_sub(0x18),
            ..*fr
        };
        crate::nodes::visit_gated(g, img, &fr);
    }
    let both = g.read::<u32>(w(0xd3)) & g.read::<u32>(w(0xd9));
    if both & 1 != 0 && g.read::<u16>(w(0xe7)) == 0 {
        let s = slot_target(g, w(0xef), 214);
        let x = g.read::<u32>(w(0xf5));
        g.call(s, 0, &[x]);
    }
    Ret::eax(0)
}

/// [`summon_frame_214`] (267 bytes, 1 overlay).
pub const SUMMON_FRAME_214: Sig = Sig {
    len: 267,
    hash: 0xbd22_e50a_4f7f_d644,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0x44, Callee::Sig(&crate::sa_summon::BONE)),
        (0xae, Callee::Sig(&crate::nodes::GATED)),
        (0xce, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};
