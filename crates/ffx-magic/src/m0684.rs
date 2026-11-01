//! `magic_0684.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `fstp dword` of a slot's ST0: the `f32` it rounds to (an empty x87
/// stack stores the indefinite NaN).
fn st0_f32(o: crate::Out) -> f32 {
    o.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32)
}

/// Each target's share of the throws (`s[0x1c + 4 i] = s[0x48 + 4 i] /
/// total`) for the `*G_N` targets (re-read at operand `+n` each turn).
fn shares<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, n: u32, total: f32) {
    let mut i = 0u32;
    loop {
        let count = cvt(g.read::<u32>(s + 0x48 + 4 * i));
        g.write::<u32>(s + 0x1c + 4 * i, div(count, total).to_bits());
        i += 1;
        if i.cast_signed() >= g.read::<u32>(img.word(f + n)).cast_signed() {
            return;
        }
    }
}

/// The throw task's step (0x10006e30, a task callback, cdecl `(task)`;
/// the state block `s = task[+4]`, its step `s[+0x18]`, its frame
/// `s[+0x40]` counted at every return but the setup's). Step 0: the
/// target nodes ([`target_nodes`](crate::s4_summon::target_nodes), a
/// record on the frame: the node, its parent, the actor `*G_A`, 0, 1), the
/// colour node ([`colour_node`](crate::s3_misc::colour_node)), seven
/// colour words 0.3 and the camera move
/// ([`move_start`](crate::sb_misc::move_start)); step 10: each of the `n`
/// targets' throw count from slot 291 `(*G_X, T[i], &a, &a, &a)` (`a` the
/// argument's word) into `s[0x48 + 4 i]` and their sum `s[+0x44]`, the
/// shares, the six throw slots free (0xff), then step 11. Step 11: every
/// 0x1c frames (`s[+0x3c]` counting down) the throw slot `s[+0x44] % 5`
/// if free gets a throw ([`thrown_effect`](crate::s5_misc::thrown_effect))
/// at the target a random draw (slot 307, ST0) falls on by the shares, its
/// size another draw; the last throw ends the step, else the target's
/// count and the shares go down. Step 12 from frame 0x3c: the colours 1.0
/// and the camera's way back ([`move_start_2`](crate::sb_misc::move_start_2)),
/// step 10000. Step 10000 takes the task's node (`task[0]`) off its
/// list. Returns the `eax`
/// the code leaves.
#[allow(clippy::too_many_lines)] // the steps as the original orders them
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let k = |g: &G, off: u32| f32::from_bits(r(g, off));
    let task = fr.arg(g, 0);
    let arg_word = fr.esp + 4;
    let s = g.read::<u32>(task + 4);
    let frame = |g: &mut G| {
        let v = g.read::<u32>(s + 0x40).wrapping_add(1);
        g.write::<u32>(s + 0x40, v);
    };
    let ebp = fr.esp.wrapping_sub(4);
    // push ebp, sub esp 0x1c, push esi, push edi, the return address
    let callee = |at: u32, ecx: u32, edx: u32, x2: u32| Frame {
        at,
        esp: fr.esp.wrapping_sub(0x2c),
        eax: 0,
        ecx,
        edx,
        xmm: [0, 0, u128::from(x2), 0],
    };
    let state = g.read::<u32>(s + 0x18);
    if state > 0xb {
        let eax = match state {
            0xc if g.read::<u32>(s + 0x40).cast_signed() >= 0x3c => {
                let (y, x) = (k(g, 0x31b), k(g, 0x323));
                for off in [0x329, 0x333, 0x33d, 0x347, 0x351, 0x35b, 0x365] {
                    g.write::<u32>(w(off), 0x3f80_0000);
                }
                let eax = crate::sb_misc::move_start_2(g, img, img.rel32_target(f + 0x36e), x, y);
                g.write::<u32>(s + 0x18, 0x2710);
                eax
            }
            0x2710 => {
                // the task's list node
                let e = g.read::<u32>(task);
                let (next, prev) = (g.read::<u32>(e + 4), g.read::<u32>(e + 8));
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(e + 8);
                let next = g.read::<u32>(e + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(e + 8, e);
                g.write::<u32>(e + 4, e);
                next
            }
            _ => state,
        };
        frame(g);
        return Ret::eax(eax);
    }
    if state != 0 && state != 0xa && state != 0xb {
        frame(g);
        return Ret::eax(state);
    }
    if state != 0xb {
        if state == 0 {
            let rec = ebp.wrapping_sub(0x1c);
            for (o, v) in [(0u32, w(0x41)), (4, w(0x48)), (8, r(g, 0x37)), (0xc, 0)] {
                g.write::<u32>(rec + o, v);
            }
            g.write::<u8>(rec + 0x10, 1);
            g.write::<u32>(w(0x5c), w(0x60));
            crate::s4_summon::target_nodes(g, img, &callee(img.rel32_target(f + 0x65), rec, 0, 0));
            let pair = ebp.wrapping_sub(8);
            g.write::<u32>(pair, w(0x6f));
            g.write::<u32>(pair + 4, w(0x76));
            g.write::<u32>(w(0x7c), w(0x80));
            let eax = crate::s3_misc::colour_node(
                g,
                img,
                &callee(img.rel32_target(f + 0x85), pair, 0, 0),
            );
            let x = k(g, 0x8d);
            for off in [0x93, 0x9d, 0xa7, 0xb1, 0xbb, 0xc5, 0xcf] {
                g.write::<u32>(w(off), 0x3e99_999a);
            }
            crate::sb_misc::move_start(g, img, img.rel32_target(f + 0xd8), x, eax);
            g.write::<u32>(s + 0x18, 0xa);
        }
        g.write::<u32>(s + 0x44, 0);
        let mut n = r(g, 0xeb);
        if n.cast_signed() > 0 {
            let mut t = w(0xf6);
            let mut i = 0u32;
            let t291 = slot_target(g, w(0x10a), 291);
            loop {
                let mut p = Pushes::default();
                p.push_all(&[arg_word, arg_word, arg_word, g.read::<u32>(t), r(g, 0x110)]);
                // call width: 12 (Pushes caps the words at 12)
                let c = p.call(g, t291, 0).eax;
                g.write::<u32>(s + 0x48 + 4 * i, c);
                let sum = g.read::<u32>(s + 0x44).wrapping_add(c);
                g.write::<u32>(s + 0x44, sum);
                n = r(g, 0x125);
                i += 1;
                t = t.wrapping_add(0x98);
                if i.cast_signed() >= n.cast_signed() {
                    break;
                }
            }
        }
        let total = cvt(g.read::<u32>(s + 0x44));
        if n.cast_signed() > 0 {
            shares(g, img, f, s, 0x167, total);
        }
        for k in 0..6u32 {
            g.write::<u32>(s + 0x340 + 0x310 * k, 0xff);
        }
        let v = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, v);
        g.write::<u32>(s + 0x3c, 0);
    }
    // step 11
    let wait = g.read::<u32>(s + 0x3c);
    g.write::<u32>(s + 0x3c, wait.wrapping_sub(1));
    if wait.cast_signed() > 0 {
        frame(g);
        return Ret::eax(wait.wrapping_sub(1));
    }
    let left = g.read::<u32>(s + 0x44).cast_signed();
    let slot = (left % 5).cast_unsigned().wrapping_mul(0x310);
    if g.read::<u32>(s.wrapping_add(slot).wrapping_add(0x340)) != 0xff {
        frame(g);
        return Ret::eax((left / 5).cast_unsigned());
    }
    let t307 = slot_target(g, w(0x1e9), 307);
    let a = st0_f32(g.call(t307, 0, &[]));
    g.write::<u32>(arg_word, a.to_bits());
    let last = r(g, 0x1f9).wrapping_sub(1);
    let mut x = sub(a, k(g, 0x206));
    let mut i = 0u32;
    while i.cast_signed() < last.cast_signed() {
        let share = f32::from_bits(g.read::<u32>(s + 0x1c + 4 * i));
        // comiss; ja: the draw falls on this target
        if share > x {
            break;
        }
        i += 1;
        x = sub(x, share);
    }
    g.write::<u32>(arg_word, i);
    let t307 = slot_target(g, w(0x22c), 307);
    let b = st0_f32(g.call(t307, 0, &[]));
    g.write::<u32>(ebp.wrapping_sub(4), b.to_bits());
    let size = add(mul(sub(b, k(g, 0x244)), k(g, 0x255)), k(g, 0x266));
    let target = i.wrapping_mul(0x98).wrapping_add(w(0x25e));
    let obj = s.wrapping_add(0x70).wrapping_add(slot);
    let at = img.rel32_target(f + 0x26d);
    let eax = crate::s5_misc::thrown_effect(g, img, &callee(at, obj, target, size.to_bits()));
    let left = g.read::<u32>(s + 0x44).wrapping_sub(1);
    g.write::<u32>(s + 0x44, left);
    if left == 0 {
        let v = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, v);
        g.write::<u32>(s + 0x40, 0);
        frame(g);
        return Ret::eax(eax);
    }
    let c = g.read::<u32>(s + 0x48 + 4 * i).wrapping_sub(1);
    g.write::<u32>(s + 0x48 + 4 * i, c);
    g.write::<u32>(s + 0x3c, 0x1b);
    let n = r(g, 0x29b);
    if n.cast_signed() <= 0 {
        frame(g);
        return Ret::eax(i);
    }
    let total = cvt(left);
    shares(g, img, f, s, 0x2cf, total);
    frame(g);
    let n = r(g, 0x2cf);
    Ret::eax(s.wrapping_add(0x1c).wrapping_add(4 * n))
}

/// [`step`] (898 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 898,
    hash: 0xf839_19f1_0466_0876,
    calls: &[
        (0x65, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x85, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0xd8, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x26d, Callee::Sig(&crate::s5_misc::THROWN_EFFECT)),
        (0x36e, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
    ],
    imports: &[],
};
