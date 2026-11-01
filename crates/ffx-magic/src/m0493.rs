//! `magic_0493.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::l3_common::{ListAt, attach, node_init};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets up the effect object (0x10006ae0, internal: `ecx` = the object,
/// `edx` = the effect, `xmm2` a float; a frame with locals). The object
/// becomes a task (vtable `+0x18`, callback `+0x1f`) appended to the
/// task ring (tail `+0x27`, the ring's flag bit cleared); its node
/// `+0x580` joins the node list (vtable `+0xa7`); slot 268 `(e[0x54],
/// 0x11, v)` fills `v = +0x59c` (`w` 1.0); the effect node
/// ([`effect_node`](crate::sb_nodes::effect_node)) of the record
/// `[+0xaf0, LIST, NAME, node, DATA, 0]` in the frame at `ebp - 0x20`
/// gets the vtable `+0x106`, flag bit 0 of `+0x2c` and `+0x30` 0; the
/// float to `+0xae0`, the effect to `+0xaec`, `+0xadc..+0xae8` 0.
/// Returns the effect.
pub fn init<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (obj, e) = (fr.ecx, fr.edx);
    let x = fr.xmm[2] as u32;
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x18));
    g.write::<u32>(obj + 0x10, w(0x1f));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x27));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x31));
    let tail = g.read::<u32>(w(0x36));
    g.write::<u32>(tail + 4, obj);
    let flags = g.read::<u32>(w(0x45));
    g.write::<u32>(w(0x45), flags & !1);
    g.write::<u32>(w(0x4c), obj);
    let b = obj + 0x580;
    node_init(g, b, w(0x5f));
    let at = ListAt {
        last: w(0x6b),
        last2: w(0x73),
        sentinel: w(0x7b),
        first: w(0x83),
        count: w(0x8f),
        last_w: w(0xa0),
    };
    attach(g, b, w(0x66), at);
    let t = slot_target(g, w(0x94), 268);
    g.write::<u32>(b, w(0xa7));
    let v = obj + 0x59c;
    let m = g.read::<u32>(e + 0x54);
    g.call(t, 0, &[m, 0x11, v]);
    g.write::<u32>(v + 0xc, ONE);
    g.write::<u32>(obj + 0x5a8, ONE);
    let rec = ebp - 0x20;
    let node = obj + 0xaf0;
    for (i, v) in (0u32..).zip([node, w(0xdb), w(0xe2), b, w(0xec), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0xf8), rec);
    let n = g.read::<u32>(rec);
    g.write::<u32>(n, w(0x106));
    let fl = g.read::<u32>(node + 0x2c);
    g.write::<u32>(node + 0x2c, fl | 1);
    g.write::<u32>(node + 0x30, 0);
    g.write::<u32>(obj + 0xae0, x);
    g.write::<u32>(obj + 0xaec, e);
    for off in [0xadc, 0xae4, 0xae8] {
        g.write::<u32>(obj + off, 0);
    }
    e
}

/// [`init`] (331 bytes, 1 overlay).
pub const INIT: Sig = Sig {
    len: 331,
    hash: 0x5824_fc5c_202e_7a1a,
    calls: &[(0xf8, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// `fstp dword` of a slot's ST0: the `f32` it rounds to (an empty x87
/// stack stores the indefinite NaN).
fn st0_f32(o: crate::Out) -> f32 {
    o.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32)
}

/// Each target's share of the throws (`s[0x1c + 4 i] = s[0x44 + 4 i] /
/// total`) for the `*G_N` targets (re-read at operand `+n` each turn).
fn shares<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, n: u32, total: f32) {
    use crate::sb_sse::{cvt, div};
    let mut i = 0u32;
    loop {
        let count = cvt(g.read::<u32>(s + 0x44 + 4 * i));
        g.write::<u32>(s + 0x1c + 4 * i, div(count, total).to_bits());
        i += 1;
        if i.cast_signed() >= g.read::<u32>(img.word(f + n)).cast_signed() {
            return;
        }
    }
}

/// The throw task's step (0x10006c30, a task callback, cdecl `(task)`;
/// the state block `s = task[+4]`, its step `s[+0x18]`): `magic_0684`'s
/// [`step`](crate::m0684::step) without the colour node, the camera and
/// the frame count. Step 0: the target nodes
/// ([`target_nodes`](crate::s4_summon::target_nodes), a record on the
/// frame: the node, its parent, the actor `*G_A`, 0, 1); step 10: each of
/// the `n` targets' throw count from slot 291 `(*G_S, T[i], &a, &a, &a)`
/// (`a` the argument's word) into `s[0x44 + 4 i]` and their sum
/// `s[+0x40]`, the shares, the six throw slots (0xb30 bytes apart) free,
/// then step 11. Step 11: every 0x14 frames (`s[+0x3c]` counting down)
/// the throw slot `s[+0x40] % 5` if free gets a throw ([`init`]) at the
/// target a random draw (slot 307, ST0) falls on by the shares, its size
/// another draw; the last throw plays slot 228 `(*G_S, 0x6a)` and ends at
/// step 10000, else the target's count and the shares go down. Step 10000:
/// once slot 411 `(*G_S)` says the sound is done, slot 213 `(*G_S, 0)`
/// and the task's node (`task[0]`) off its list. Returns the `eax` the
/// code leaves.
#[allow(clippy::too_many_lines)] // the steps as the original orders them
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let k = |g: &G, off: u32| f32::from_bits(r(g, off));
    let call = |g: &mut G, words: &[u32], slot: u32| {
        let mut p = Pushes::default();
        p.push_all(words);
        let t = slot_target(g, w(0x90), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    let task = fr.arg(g, 0);
    let arg_word = fr.esp + 4;
    let s = g.read::<u32>(task + 4);
    let ebp = fr.esp.wrapping_sub(4);
    // push ebp, sub esp 0x18, push esi, push edi, the return address
    let callee = |at: u32, ecx: u32, edx: u32, x2: u32| Frame {
        at,
        esp: fr.esp.wrapping_sub(0x28),
        eax: 0,
        ecx,
        edx,
        xmm: [0, 0, u128::from(x2), 0],
    };
    let state = g.read::<u32>(s + 0x18);
    if state > 0xb {
        if state != 0x2710 {
            return Ret::eax(state);
        }
        let done = call(g, &[r(g, 0x289)], 411).eax;
        if done == 0 {
            return Ret::eax(0);
        }
        call(g, &[0, r(g, 0x2a5)], 213);
        // the task's list node
        let e = g.read::<u32>(task);
        let (next, prev) = (g.read::<u32>(e + 4), g.read::<u32>(e + 8));
        g.write::<u32>(next + 8, prev);
        let prev = g.read::<u32>(e + 8);
        let next = g.read::<u32>(e + 4);
        g.write::<u32>(prev + 4, next);
        g.write::<u32>(e + 8, e);
        g.write::<u32>(e + 4, e);
        return Ret::eax(next);
    }
    if state != 0 && state != 0xa && state != 0xb {
        return Ret::eax(state);
    }
    if state != 0xb {
        if state == 0 {
            let rec = ebp.wrapping_sub(0x18);
            for (o, v) in [(0u32, w(0x3a)), (4, w(0x41)), (8, r(g, 0x30)), (0xc, 0)] {
                g.write::<u32>(rec + o, v);
            }
            g.write::<u8>(rec + 0x10, 1);
            g.write::<u32>(w(0x55), w(0x59));
            let at = img.rel32_target(f + 0x5e);
            crate::s4_summon::target_nodes(g, img, &callee(at, rec, 0, 0));
            g.write::<u32>(s + 0x18, 0xa);
        }
        g.write::<u32>(s + 0x40, 0);
        let mut n = r(g, 0x71);
        if n.cast_signed() > 0 {
            let mut t = w(0x7c);
            let mut i = 0u32;
            loop {
                let words = [arg_word, arg_word, arg_word, g.read::<u32>(t), r(g, 0x96)];
                let c = call(g, &words, 291).eax;
                g.write::<u32>(s + 0x44 + 4 * i, c);
                let sum = g.read::<u32>(s + 0x40).wrapping_add(c);
                g.write::<u32>(s + 0x40, sum);
                n = r(g, 0xab);
                i += 1;
                t = t.wrapping_add(0x98);
                if i.cast_signed() >= n.cast_signed() {
                    break;
                }
            }
        }
        let total = cvt(g.read::<u32>(s + 0x40));
        if n.cast_signed() > 0 {
            shares(g, img, f, s, 0xec, total);
        }
        for k in 0..6u32 {
            g.write::<u32>(s + 0xb58 + 0xb30 * k, 0xff);
        }
        let v = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, v);
        g.write::<u32>(s + 0x3c, 0);
    }
    // step 11
    let wait = g.read::<u32>(s + 0x3c);
    g.write::<u32>(s + 0x3c, wait.wrapping_sub(1));
    if wait.cast_signed() > 0 {
        return Ret::eax(wait.wrapping_sub(1));
    }
    let left = g.read::<u32>(s + 0x40).cast_signed();
    let slot = (left % 5).cast_unsigned().wrapping_mul(0xb30);
    if g.read::<u32>(s.wrapping_add(slot).wrapping_add(0xb58)) != 0xff {
        return Ret::eax((left / 5).cast_unsigned());
    }
    let a = st0_f32(call(g, &[], 307));
    g.write::<u32>(arg_word, a.to_bits());
    let last = r(g, 0x17e).wrapping_sub(1);
    let mut x = sub(a, k(g, 0x18b));
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
    let b = st0_f32(call(g, &[], 307));
    g.write::<u32>(ebp.wrapping_sub(4), b.to_bits());
    let size = add(mul(sub(b, k(g, 0x1d0)), k(g, 0x1e1)), k(g, 0x1f2));
    let target = i.wrapping_mul(0x98).wrapping_add(w(0x1ea));
    let obj = s.wrapping_add(0x70).wrapping_add(slot);
    let at = img.rel32_target(f + 0x1f9);
    init(g, img, &callee(at, obj, target, size.to_bits()));
    let left = g.read::<u32>(s + 0x40).wrapping_sub(1);
    g.write::<u32>(s + 0x40, left);
    if left == 0 {
        let e = call(g, &[0x6a, r(g, 0x20b)], 228).eax;
        g.write::<u32>(s + 0x18, 0x2710);
        return Ret::eax(e);
    }
    let c = g.read::<u32>(s + 0x44 + 4 * i).wrapping_sub(1);
    g.write::<u32>(s + 0x44 + 4 * i, c);
    g.write::<u32>(s + 0x3c, 0x14);
    if r(g, 0x239).cast_signed() <= 0 {
        return Ret::eax(i);
    }
    shares(g, img, f, s, 0x26f, cvt(left));
    let n = r(g, 0x26f);
    Ret::eax(s.wrapping_add(0x1c).wrapping_add(4 * n))
}

/// [`step`] (724 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 724,
    hash: 0xc787_f17e_f8a2_5ed1,
    calls: &[
        (0x5e, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x1f9, Callee::Sig(&INIT)),
    ],
    imports: &[],
};

/// The summon's particle start as `magic_0493` binds it (0x10006f10,
/// cdecl `(actor)`; [`summon_ppp_start`](crate::s5_misc::summon_ppp_start)
/// with part data kind 2 and an environment of 0x100000 bytes).
pub fn ppp_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s5_misc::summon_ppp_start(g, img, fr)
}

/// [`ppp_start`] (437 bytes, 1 overlay; the `rel32`:
/// [`targets_init`](crate::s4_summon::targets_init)).
pub const PPP_START: Sig = Sig {
    len: 437,
    hash: 0x3d69_b7e9_d027_b7d8,
    calls: &[(0x8, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};
