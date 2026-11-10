//! Round 43's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0445`'s summon start (0x10007890, 513 bytes; a table entry, cdecl
/// `(actor)`, an `ebp` frame with one local `L` at `ebp - 4`, which starts
/// as the entry's `ecx`): [`summon_start_b`](crate::s4_summon::summon_start_b)
/// with its operands one byte later (the `push ecx` of the local) up to
/// slot 221, five bytes later from the ring stores on (a `lea ecx, [ebp -
/// 4]; push ecx` that stays on the stack), and three more calls at the
/// end: slot 291 (`MsGetDamageInfo`) `(*G_D, byte *G_B, &L, &L, &L)`,
/// slot 523 `(0, 0x55 e + 0xcf, ...)` with `e` its `eax`, then slot 575
/// with no words of its own. Returns 0.
pub fn summon_start_0445<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let local = ebp.wrapping_sub(4);
    g.write::<u32>(local, fr.ecx);
    // push ebp; push ecx; push esi; call
    crate::s4_summon::targets_init(
        g,
        img,
        img.rel32_target(f + 0x9),
        fr.esp.wrapping_sub(16),
        actor,
    );
    for (dst, src) in [(0x14, 0x18), (0x1e, 0x22), (0x28, 0x2c), (0x32, 0x36)] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x3c), 1);
    g.write::<u32>(w(0x46), w(0x4a));
    g.write::<u16>(w(0x51), 0x1ff);
    g.write::<u32>(w(0x59), w(0x5d));
    g.write::<u32>(w(0x63), w(0x67));
    g.write::<u32>(w(0x6d), 0);
    g.write::<u32>(w(0x77), w(0x7b));
    g.write::<u16>(w(0x82), 0xff);
    g.write::<u32>(w(0x8a), w(0x8e));
    g.write::<u32>(w(0x94), w(0x98));
    g.write::<u32>(w(0x9e), 0);
    let mut s = Pushes::default();
    s.push(w(0xad));
    g.write::<u32>(w(0xb3), w(0xb7));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x0e), 718), 0);
    // the kind: `push 0` at +0xc2
    s.push_all(&[w(0xc3) & 0xff, 8, w(0xcd)]);
    // call width: 12 (Pushes caps the words at 12)
    let handle = s.call(g, slot_target(g, w(0xbe), 719), 0).eax;
    let lock = g.read::<u32>(w(0xd5));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xde), handle);
    s.push_all(&[0, w(0xe3), handle]);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xe9), 711), 0);
    let lock = g.read::<u32>(w(0xfc));
    g.write::<u32>(lock, saved);
    let handle = g.read::<u32>(w(0x113));
    s.push_all(&[w(0x101), w(0x10d), handle, w(0x11e)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x108), 710), 0);
    s.push_all(&[2, w(0x132), 0, w(0x139)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x125), 222), 0);
    s.push_all(&[2, w(0x14d), 0, w(0x154)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x140), 221), 0);
    let tail = g.read::<u32>(w(0x15b));
    s.pop(19);
    s.push(local);
    g.write::<u32>(w(0x167), tail);
    for (dst, src) in [
        (0x16d, 0x171),
        (0x177, 0x17b),
        (0x181, 0x185),
        (0x18b, 0x18f),
    ] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(tail.wrapping_add(4), w(0x196));
    let byte = u32::from(g.read::<u8>(w(0x19d)));
    let flags = g.read::<u32>(w(0x1a3));
    g.write::<u32>(w(0x1a3), flags & !1);
    let d = g.read::<u32>(w(0x1ad));
    s.push_all(&[local, local, byte, d]);
    g.write::<u32>(w(0x1b8), w(0x1bc));
    g.write::<u32>(w(0x1c2), 0);
    // call width: 12 (Pushes caps the words at 12)
    let e = s.call(g, slot_target(g, w(0x1b2), 291), 0).eax;
    s.push_all(&[e.wrapping_mul(0x55).wrapping_add(0xcf), 0]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1dc), 523), 0);
    s.pop(7);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1eb), 575), 0);
    Ret::eax(0)
}

/// [`summon_start_0445`] (the `rel32`:
/// [`targets_init`](crate::s4_summon::targets_init)).
pub const SUMMON_START_0445: Sig = Sig {
    len: 513,
    hash: 0xd867_8598_a4e4_b0ea,
    calls: &[(0x9, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};

/// `magic_0445`'s summon object set-up (0x10006bf0, 734 bytes; internal:
/// `ecx` = the object `O`, `edx` = a record `R`, `xmm3` a scale `s`, cdecl
/// words `(P, keep)`; an `ebp` frame, the set-up record `L` at `ebp -
/// 0x20`): [`summon_object_setup`](crate::s7_misc::summon_object_setup)
/// with `B`'s point copied by slot 325 (`FFXVu0CopyVector`) `(O+0x13c,
/// P)` and no `w`, the effect node's last word 2, no slot 239, the three
/// random offsets only when `keep` is 0 (each `frnd` result stored over
/// `keep`'s stack word, as the original's `fstp`), and at the end `+0x2c4
/// = s`, `+0x2d4 = R`, `+0x2c8`, `+0x2cc`, `+0x2d0` 0. Returns `R`.
pub fn object_setup_0445<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let call = |o: u32| img.rel32_target(f + o);
    let (o, r) = (fr.ecx, fr.edx);
    let s = fr.xmm_f32(3);
    let ebp = fr.esp.wrapping_sub(4);
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x18));
    g.write::<u32>(o + 0x10, w(0x1f));
    g.write::<u32>(o + 0x14, o);
    let last = g.read::<u32>(w(0x27));
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x31));
    let last = g.read::<u32>(w(0x36));
    g.write::<u32>(last.wrapping_add(4), o);
    let v = g.read::<u32>(w(0x45)) & !1;
    g.write::<u32>(w(0x45), v);
    g.write::<u32>(w(0x4c), o);
    let (b, c) = (o + 0x120, o + 0x14c);
    crate::s7_misc::parent_append(
        g,
        img,
        f,
        b,
        [0x5f, 0x66, 0x6b, 0x73, 0x7b, 0x83, 0x92, 0xa4, 0xaa],
    );
    let p = fr.arg(g, 0);
    // call width: 2
    g.call(slot_target(g, w(0x9e), 325), o, &[o + 0x13c, p]);
    crate::s7_misc::parent_append(
        g,
        img,
        f,
        c,
        [0xce, 0xd5, 0xda, 0xe2, 0xe7, 0xef, 0xfb, 0x10e, 0x114],
    );
    let alpha = mul(s, k(g, 0x108));
    for at in [0x168u32, 0x16c, 0x170] {
        g.write::<u32>(o + at, 0x3f80_0000);
    }
    g.write::<f32>(o + 0x174, alpha);
    let rec = ebp.wrapping_sub(0x20);
    let set = |g: &mut G, words: [u32; 6]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(rec + 4 * i, v);
        }
    };
    let n = o + 0x178;
    set(g, [n, w(0x150), w(0x157), b, w(0x161), c]);
    // the frame (0x24 bytes), ebx/esi/edi, the return address
    let insert = Frame {
        at: call(0x166),
        esp: ebp.wrapping_sub(0x34),
        ecx: rec,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let d = g.read::<u32>(w(0x16b));
    let q = o + 0x20;
    g.write::<u32>(rec, q);
    g.write::<u32>(q, w(0x177));
    for (i, v) in (1u32..).zip([w(0x181), n, d, w(0x18e), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x19a), rec);
    let e = o + 0x2d8;
    set(g, [e, w(0x1ad), w(0x1b4), b, w(0x1be), 2]);
    crate::sb_nodes::effect_node(g, img, call(0x1ca), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x1d3));
    let v = g.read::<u32>(e + 0x2c) | 1;
    g.write::<u32>(e + 0x2c, v);
    g.write::<u32>(e + 0x30, 0);
    if fr.arg(g, 1) == 0 {
        let keep = fr.esp.wrapping_add(8);
        // (the slot table, K, K', K'', the field)
        for (t, k1, k2, k3, at) in [
            (0x1edu32, 0x205u32, 0x212u32, 0x21au32, 0x13cu32),
            (0x20a, 0x242, 0x24f, 0x257, 0x140),
            (0x247, 0x27f, 0x287, 0x28f, 0x144),
        ] {
            // call width: 0
            let rnd = g.call(slot_target(g, w(t), 307), rec, &[]);
            let v = crate::m0018::st0_f32(rnd.st0);
            g.write::<f32>(keep, v);
            let v = sub(mul(sub(v, k(g, k1)), k(g, k2)), k(g, k3));
            let v = add(v, g.read::<f32>(o + at));
            g.write::<f32>(o + at, v);
        }
    }
    g.write::<f32>(o + 0x2c4, s);
    g.write::<u32>(o + 0x2d4, r);
    for at in [0x2c8u32, 0x2cc, 0x2d0] {
        g.write::<u32>(o + at, 0);
    }
    r
}

/// [`object_setup_0445`] (the `rel32`s:
/// [`node_insert`](crate::nodes::node_insert),
/// [`particle_node`](crate::sb_nodes::particle_node) and
/// [`effect_node`](crate::sb_nodes::effect_node)).
pub const OBJECT_SETUP_0445: Sig = Sig {
    len: 734,
    hash: 0x0b5a_c990_d5c6_304a,
    calls: &[
        (0x166, Callee::Sig(&crate::nodes::INSERT)),
        (0x19a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1ca, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// One slot call of the 0445 task: the slot table `*G_T` (operand `o`).
fn slot_at<G: Game>(g: &mut G, img: &Image, f: u32, o: u32, slot: u32, args: &[u32]) -> u32 {
    let t = slot_target(g, img.word(f + o), slot);
    g.call(t, 0, args).eax
}

/// `magic_0445`'s summon task (0x10006ed0, 2490 bytes; a task callback,
/// cdecl `(S)`, the cursor; an 8-aligned frame, `e` its base after the
/// pushes: the count local `e + 0x14`, the index `e + 0x10`, the record
/// `e + 0x20`). With the block `T = S[1]`, its state `T + 0x18` and timer
/// `T + 0x1c`: a state above 0x2710 or with no case (the byte table at
/// `+0x3a` gives case 12) only counts the timer up; 0x2710 unlinks the
/// node `S[0]` from its ring. The cases (byte, state):
/// - 0 (0x00): the model nodes, the root's node appended, the centre of
///   the targets, an effect node (last word 0), a colour node, eight 1.0
///   globals, a root node; then case 7.
/// - 7 (0x46): a particle node (last word 2); state up, timer 1.
/// - 3 (0x3c): state up, timer 1.
/// - 8 (0x47, from timer 10): the colour globals zeroed, the target list
///   (slot 288 `(0x10, &n)`); each target not among the `*G_N` objects
///   (stride 0x98) gets slot 541 `(t, 0.0, 15)` and is kept in `T +
///   0x1290` (u16 count `T + 0x12d0`); state up, timer 1.
/// - 10 (0x50): a node appended, its colour, a node inserted, a particle
///   node and an effect node (last words 1); state up, timer 1.
/// - 1 (0xa): slot 291 `(*G_D, *G_A, &L, &L, &L)` gives the throws `T +
///   0x24`, the five objects' `+0x300` 0xff, then case 2.
/// - 2 (0xb): every 0x56 frames an object `T + 0x30 + 0x310 (T[0x24] %
///   5)` still free (0xff) is set up ([`object_setup_0445`], `keep` for
///   all but the first, the scale from slot 307) after slot 234 sets a
///   colour; the last throw ends in slot 228 `(*G_D, 0x72)` and state
///   0x3c.
/// - 4 (0x3d): slot 411 `(*G_D)`, when non-zero slot 213 `(*G_D, 0)`;
///   state up.
/// - 5 (0x3e, from timer 0x69): each kept target gets slots 296 `(t, 1)`
///   and 541 `(t, 1.0, 15)`; state up, timer 1.
/// - 9 (0x48): at timer 1 the locked slot 878 `(1)`; from timer 0x28 each
///   kept target gets slot 296 `(t, 0)`, then state 0x50, dispatched again.
/// - 11 (0x51, from timer 0x7a): state 0xa, dispatched again.
/// - 6 (0x3f, from timer 15): the colour globals faded to `K - x`, the
///   root node leaves, the locked slot 878 `(0)`; state 0x2710,
///   dispatched again.
///
/// The `ecx` the two locked calls pass is what the code leaves there: the
/// cursor with its low byte 1, and the root node (which the leave keeps
/// in `ecx`) with its low byte 0. Returns the last `eax` of the path.
#[allow(clippy::too_many_lines)] // one case per state, as the original
pub fn task_0445<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let rec = e + 0x20;
    let timer = t.wrapping_add(0x1c);
    let state = t.wrapping_add(0x18);
    let up = |g: &mut G, a: u32| {
        let v = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, v);
    };
    let below = |g: &G, n: i32| g.read::<i32>(timer) < n;
    let set = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(rec + 4 * i, v);
        }
    };
    let count = t.wrapping_add(0x12d0);
    let kept = t.wrapping_add(0x1290);
    loop {
        let s = g.read::<u32>(state);
        if s == 0x2710 {
            let node = g.read::<u32>(cursor);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next.wrapping_add(8), prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev.wrapping_add(4), next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            up(g, timer);
            return Ret::eax(next);
        }
        if s > 0x51 {
            up(g, timer);
            return Ret::eax(s);
        }
        let case = img.word(w(0x3a) + s) & 0xff;
        // the default: the timer up, `eax` as the path left it
        let idle = |g: &mut G, eax: u32| {
            up(g, timer);
            Ret::eax(eax)
        };
        match case {
            9 => {
                let mut eax = case;
                if g.read::<u32>(timer) == 1 {
                    let fr8 = Frame {
                        at: call(0x4e),
                        esp: e.wrapping_sub(4),
                        ecx: cursor & !0xff | 1,
                        ..Frame::default()
                    };
                    eax = crate::s4_small::locked_878(g, img, &fr8).eax;
                }
                if below(g, 0x28) {
                    return idle(g, eax);
                }
                let mut i = 0u32;
                while i < u32::from(g.read::<u16>(count)) {
                    let id = u32::from(g.read::<u16>(kept + 2 * i));
                    // call width: 2
                    slot_at(g, img, f, 0x77, 296, &[id, 0]);
                    i += 1;
                }
                g.write::<u32>(state, 0x50);
            }
            11 => {
                if below(g, 0x7a) {
                    return idle(g, case);
                }
                g.write::<u32>(state, 0xa);
            }
            6 => {
                if below(g, 0xf) {
                    return idle(g, case);
                }
                fade_0445(g, img, f, e);
                g.write::<u32>(state, 0x2710);
            }
            0 | 7 => {
                if case == 0 {
                    setup_0445(g, img, f, e);
                    g.write::<u32>(state, 0x46);
                }
                let d = g.read::<u32>(w(0x35b));
                set(g, &[w(0x367), w(0x36f), w(0x377), d, w(0x383), 2]);
                g.write::<u32>(w(0x391), w(0x395));
                let eax = crate::sb_nodes::particle_node(g, img, call(0x39a), rec);
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(eax);
            }
            3 => {
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(case);
            }
            8 => {
                if below(g, 0xa) {
                    return idle(g, case);
                }
                let eax = targets_0445(g, img, f, e, t);
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(eax);
            }
            10 => {
                let (node, list) = (w(0x578), w(0x573));
                crate::sb_misc::node_append(g, img, call(0x57d), node, list, 0);
                g.write::<u32>(rec + 0x14, node);
                for (o, v) in [
                    (0x58b, 0x3f80_0000u32),
                    (0x595, 0x3f80_0000),
                    (0x59f, 0x3f80_0000),
                    (0x5a9, 0x3f0f_5c29),
                ] {
                    g.write::<u32>(w(o), v);
                }
                set(g, &[w(0x5b5), list, w(0x5c1), w(0x5c9), w(0x5d1)]);
                let ins = Frame {
                    at: call(0x5d6),
                    esp: e.wrapping_sub(4),
                    ecx: rec,
                    ..Frame::default()
                };
                crate::nodes::node_insert(g, img, &ins);
                let d = g.read::<u32>(w(0x5db));
                set(g, &[w(0x5e7), w(0x5ef), w(0x5f7), d, w(0x603), 1]);
                g.write::<u32>(w(0x611), w(0x615));
                crate::sb_nodes::particle_node(g, img, call(0x61a), rec);
                set(g, &[w(0x626), w(0x62e), w(0x636), w(0x63e), w(0x646), 1]);
                crate::sb_nodes::effect_node(g, img, call(0x653), rec);
                let node = g.read::<u32>(rec);
                g.write::<u32>(node, w(0x65d));
                let v = g.read::<u32>(w(0x663)) | 1;
                g.write::<u32>(w(0x663), v);
                g.write::<u32>(w(0x66a), 0);
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(node);
            }
            1 | 2 => {
                if case == 1 {
                    let l = e + 0x1c;
                    let (a, b) = (g.read::<u32>(w(0x69a)), g.read::<u32>(w(0x68f)));
                    // call width: 5
                    let n = slot_at(g, img, f, 0x694, 291, &[a, b, l, l, l]);
                    g.write::<u32>(t + 0x24, n);
                    for i in 0..6 {
                        g.write::<u32>(t + 0x300 + 0x310 * i, 0xff);
                    }
                    up(g, state);
                    g.write::<u32>(t + 0x28, 0);
                    g.write::<u32>(t + 0x20, 0);
                }
                return throw_0445(g, img, fr, e, t);
            }
            4 => {
                let a = g.read::<u32>(w(0x857));
                // call width: 1
                if slot_at(g, img, f, 0x851, 411, &[a]) == 0 {
                    return idle(g, 0);
                }
                let a = g.read::<u32>(w(0x877));
                // call width: 2
                let eax = slot_at(g, img, f, 0x86f, 213, &[a, 0]);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            5 => {
                if below(g, 0x69) {
                    return idle(g, case);
                }
                // `eax`: the count as last read
                let mut i = 0u32;
                let mut n = u32::from(g.read::<u16>(count));
                while i < n {
                    let at = kept + 2 * i;
                    let id = u32::from(g.read::<u16>(at));
                    // call width: 2
                    slot_at(g, img, f, 0x8bb, 296, &[id, 1]);
                    let id = u32::from(g.read::<u16>(at));
                    // call width: 3
                    slot_at(g, img, f, 0x8d9, 541, &[id, 0x3f80_0000, 0xf]);
                    i += 1;
                    n = u32::from(g.read::<u16>(count));
                }
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(n);
            }
            _ => return idle(g, case),
        }
    }
}

/// [`task_0445`]'s case 6: the colour globals faded (`K - x`, `K` at
/// operand `+0x24`), the flags `(F & !1) | 4`, the root node `N`
/// (operand `+0xd8`) leaving and the locked slot 878 with `ecx` = `N`
/// with its low byte cleared.
fn fade_0445<G: Game>(g: &mut G, img: &Image, f: u32, e: u32) {
    let w = |o: u32| img.word(f + o);
    let kk = f32::from_bits(g.read::<u32>(w(0x24)));
    let less = |g: &G, o: u32| sub(kk, f32::from_bits(g.read::<u32>(w(o))));
    let one = 0x3f80_0000u32;
    let flags = g.read::<u32>(w(0xcd)) & !1 | 4;
    let x = less(g, 0xc8);
    g.write::<f32>(w(0xe0), x);
    let x = less(g, 0xeb);
    for o in [0xf1, 0xfb, 0x105, 0x10f] {
        g.write::<u32>(w(o), one);
    }
    g.write::<f32>(w(0x11b), x);
    let x = less(g, 0x126);
    for o in [0x12c, 0x136, 0x140] {
        g.write::<u32>(w(o), one);
    }
    g.write::<u32>(w(0x14a), 0x3d4c_cccd);
    g.write::<f32>(w(0x156), x);
    let x = less(g, 0x161);
    g.write::<u32>(w(0x167), 0);
    g.write::<u32>(w(0x170), flags);
    g.write::<u32>(w(0x176), one);
    g.write::<f32>(w(0x182), x);
    let x = less(g, 0x18d);
    g.write::<f32>(w(0x195), x);
    let x = less(g, 0x1a0);
    g.write::<f32>(w(0x1a8), x);
    let x = sub(
        f32::from_bits(g.read::<u32>(w(0x1b0))),
        f32::from_bits(g.read::<u32>(w(0x1b8))),
    );
    g.write::<f32>(w(0x1c0), x);
    let x = less(g, 0x1cb);
    g.write::<f32>(w(0x1d3), x);
    let root = w(0xd8);
    crate::sa_misc::leave(g, root, flags);
    let fr8 = Frame {
        at: img.rel32_target(f + 0x1df),
        esp: e.wrapping_sub(4),
        ecx: root & !0xff,
        ..Frame::default()
    };
    crate::s4_small::locked_878(g, img, &fr8);
}

/// [`task_0445`]'s case 0 up to its particle node: the model nodes (a
/// record `(G_M, P, *G_A, 0, 1)`), the root's node appended, the centre,
/// the effect node `(.., 0)` with its vtable and flags (`| 7`), the colour
/// node (`e + 0x14`: `(G_C, P2)`), eight 1.0 globals and the root node.
fn setup_0445<G: Game>(g: &mut G, img: &Image, f: u32, e: u32) {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let rec = e + 0x20;
    let a = g.read::<u32>(w(0x20b));
    for (i, v) in (0u32..).zip([w(0x217), w(0x21f), a, 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<u8>(rec + 0x10, 1);
    g.write::<u32>(w(0x236), w(0x23a));
    let callee = |at: u32, ecx: u32, edx: u32| Frame {
        at,
        esp: e.wrapping_sub(4),
        ecx,
        edx,
        ..Frame::default()
    };
    crate::s3_summon::model_nodes(g, img, &callee(call(0x23f), rec, 0));
    crate::sb_misc::node_append(g, img, call(0x24e), w(0x249), w(0x244), 0);
    crate::m0577::centre(g, img, &callee(call(0x25d), w(0x258), w(0x253)));
    let x = sub(
        f32::from_bits(g.read::<u32>(w(0x265))),
        f32::from_bits(g.read::<u32>(w(0x26d))),
    );
    for (i, v) in (0u32..).zip([w(0x279), w(0x281), w(0x289)]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<f32>(w(0x291), x);
    for (i, v) in (3u32..).zip([w(0x299), w(0x2a1), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, call(0x2ae), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x2bc));
    let v = g.read::<u32>(w(0x2c1)) | 7;
    g.write::<u32>(w(0x2cd), 0);
    g.write::<u32>(w(0x2d6), v);
    let cr = e + 0x14;
    g.write::<u32>(cr, w(0x2de));
    g.write::<u32>(cr + 4, w(0x2e6));
    g.write::<u32>(w(0x2ec), w(0x2f0));
    crate::s3_misc::colour_node(g, img, &callee(call(0x2f5), cr, 0));
    for o in [0x300, 0x30a, 0x314, 0x31e, 0x328, 0x332, 0x33c, 0x346] {
        g.write::<u32>(w(o), 0x3f80_0000);
    }
    crate::m0070::node_init_root(g, img, call(0x34f), w(0x2fa));
}

/// [`task_0445`]'s case 8 after its timer: the colour globals zeroed
/// (`0.0 - x`), the kept count 0, slot 288's target list, and each target not
/// among the objects kept (slot 541 `(t, 0.0, 15)`). Returns the index
/// (the list when it is empty).
fn targets_0445<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, t: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let neg = |g: &G, o: u32| sub(0.0, f32::from_bits(g.read::<u32>(w(o))));
    let x = neg(g, 0x3c6);
    let flags = g.read::<u32>(w(0x3cc)) & !1;
    g.write::<u32>(w(0x3cc), flags);
    for o in [0x3d5, 0x3df] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<f32>(w(0x3eb), x);
    let x = neg(g, 0x3f6);
    for o in [0x3fc, 0x406, 0x410, 0x41a] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<f32>(w(0x426), x);
    let x = neg(g, 0x431);
    g.write::<u32>(w(0x437), 0);
    g.write::<u32>(w(0x441), 0x3d4c_cccd);
    g.write::<u32>(w(0x44b), 0);
    g.write::<f32>(w(0x457), x);
    let x = neg(g, 0x462);
    g.write::<f32>(w(0x46a), x);
    let x = neg(g, 0x475);
    g.write::<f32>(w(0x47d), x);
    let x = neg(g, 0x488);
    let y = neg(g, 0x490);
    g.write::<f32>(w(0x498), x);
    let x = sub(
        f32::from_bits(g.read::<u32>(w(0x4a0))),
        f32::from_bits(g.read::<u32>(w(0x4a8))),
    );
    g.write::<f32>(w(0x4b0), y);
    g.write::<f32>(w(0x4b8), x);
    let count = t + 0x12d0;
    g.write::<u16>(count, 0);
    let n_at = e + 0x14;
    // call width: 2
    let mut list = slot_at(g, img, f, 0x4c9, 288, &[0x10, n_at]);
    let first = list;
    let index = e + 0x10;
    g.write::<u32>(index, 0);
    if g.read::<i32>(n_at) <= 0 {
        return first;
    }
    let mut i = 0u32;
    loop {
        let id = u32::from(g.read::<u8>(list));
        let n = g.read::<i32>(w(0x4f1));
        let held = (0..n.max(0).cast_unsigned())
            .any(|j| g.read::<u32>(w(0x4ff).wrapping_add(0x98 * j)) == id);
        if !held {
            // call width: 3
            slot_at(g, img, f, 0x520, 541, &[id, 0, 0xf]);
            let c = g.read::<u16>(count);
            let id = g.read::<u8>(list);
            g.write::<u16>(t + 0x1290 + 2 * u32::from(c), id.into());
            g.write::<u16>(count, c.wrapping_add(1));
        }
        i += 1;
        list = list.wrapping_add(1);
        g.write::<u32>(index, i);
        if i.cast_signed() >= g.read::<i32>(n_at) {
            return i;
        }
    }
}

/// [`task_0445`]'s case 2 (with case 1 before it): the wait `T + 0x20`
/// down; at 0 the object `T[0x24] % 5` (when its `+0x300` is 0xff) set up
/// after slot 234 `(G_C, G_B, G_A)` and the colour words; then a throw
/// less, the wait 0x55 or (the last) slot 228 `(*G_D, 0x72)` and state
/// 0x3c.
fn throw_0445<G: Game>(g: &mut G, img: &Image, fr: &Frame, e: u32, t: u32) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let timer = t + 0x1c;
    let up = |g: &mut G, a: u32| {
        let v = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, v);
    };
    let wait = g.read::<i32>(t + 0x20);
    g.write::<i32>(t + 0x20, wait.wrapping_sub(1));
    if wait > 0 {
        up(g, timer);
        return Ret::eax(wait.wrapping_sub(1).cast_unsigned());
    }
    let n = g.read::<i32>(t + 0x24);
    let (q, j) = (n.wrapping_div(5), n.wrapping_rem(5));
    let o = t
        .wrapping_add(0x30)
        .wrapping_add(j.wrapping_mul(0x310).cast_unsigned());
    if g.read::<u32>(o + 0x2d0) != 0xff {
        up(g, timer);
        return Ret::eax(q.cast_unsigned());
    }
    for (a, v) in [
        (0x73a, 0x437f_0000u32),
        (0x744, 0x437f_0000),
        (0x74e, 0x437f_0000),
        (0x758, 0x4300_0000),
        (0x762, 0),
        (0x76c, 0),
        (0x776, 0),
        (0x780, 0x4300_0000),
    ] {
        g.write::<u32>(w(a), v);
    }
    // call width: 3
    slot_at(g, img, f, 0x72f, 234, &[w(0x794), w(0x78f), w(0x734)]);
    g.write::<u32>(w(0x7a1), 1);
    g.write::<u32>(w(0x7ab), 0x3d4c_cccd);
    g.write::<u32>(w(0x7b5), 0);
    let t307 = slot_target(g, w(0x79b), 307);
    // call width: 0
    let r = g.call(t307, 0, &[]);
    let r = crate::m0018::st0_f32(r.st0);
    g.write::<f32>(e + 0x14, r);
    let s = add(mul(sub(r, k(g, 0x7d6)), k(g, 0x7e9)), k(g, 0x7f8));
    let keep = u32::from(g.read::<u32>(t + 0x28) == 0);
    let esp = e.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, w(0x7fe));
    g.write::<u32>(esp + 8, keep);
    let setup = Frame {
        at: img.rel32_target(f + 0x803),
        esp,
        ecx: o,
        edx: w(0x7f0),
        xmm: [0, 0, 0, u128::from(s.to_bits())],
        ..Frame::default()
    };
    let eax = object_setup_0445(g, img, &setup);
    let left = g.read::<u32>(t + 0x24).wrapping_sub(1);
    g.write::<u32>(t + 0x24, left);
    up(g, t + 0x28);
    if g.read::<u32>(t + 0x24) != 0 {
        g.write::<u32>(t + 0x20, 0x55);
        up(g, timer);
        return Ret::eax(eax);
    }
    let a = g.read::<u32>(w(0x81f));
    // call width: 2
    let eax = slot_at(g, img, f, 0x817, 228, &[a, 0x72]);
    up(g, timer);
    g.write::<u32>(t + 0x18, 0x3c);
    Ret::eax(eax)
}

/// [`task_0445`] (the `rel32`s: the locked slot 878, the node leave, the
/// model nodes, the node append, the centre, the effect, colour, particle
/// and inserted nodes, the root node and [`object_setup_0445`]).
pub const TASK_0445: Sig = Sig {
    len: 2490,
    hash: 0x73da_f641_7af8_0738,
    calls: &[
        (0x4e, Callee::Sig(&crate::s4_small::LOCKED_878)),
        (0x1d8, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1df, Callee::Sig(&crate::s4_small::LOCKED_878)),
        (0x23f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x24e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x25d, Callee::Sig(&crate::m0577::CENTRE)),
        (0x2ae, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2f5, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x34f, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x39a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x57d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5d6, Callee::Sig(&crate::nodes::INSERT)),
        (0x61a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x653, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x803, Callee::Sig(&OBJECT_SETUP_0445)),
    ],
    imports: &[],
};

/// `magic_0273`'s strike start (0x10007920, 304 bytes; a task callback,
/// cdecl `(S)`, the cursor; an `ebp` frame, the effect node's record at
/// `ebp - 0x1c`). With `T = S[1]`: unless `T + 0x18` is 0 and the model
/// `M = T[0x34]` has bit 0x400 of u16 `M + 0x90`, only the timer `T +
/// 0x2c` counts up (`eax` the entry's, or `M`). Else the node `B = T +
/// 0x70` joins the parent `P`
/// ([`parent_append`](crate::s7_misc::parent_append)); slot 262 `(M[0x54],
/// c, c + 4, c + 8)` with `c = T + 0x8c`, then slot 12 `(M[0x54])`'s ST0
/// over `c + 4`; the effect node `E = T + 0x38`
/// ([`effect_node`](crate::sb_nodes::effect_node): `(E, P, G_V, B, CB,
/// 1)`, then its vtable, `+0x2c |= 1`, `+0x30 = 0`); `T + 0x60 /= T +
/// 0x30` (signed; a zero divisor gives 0, where the original faults);
/// the node `S[0]` unlinked from its ring and the timer up. Returns the
/// node's old next link.
pub fn strike_start_0273<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let timer = t.wrapping_add(0x2c);
    let up = |g: &mut G| {
        let v = g.read::<u32>(timer).wrapping_add(1);
        g.write::<u32>(timer, v);
    };
    if g.read::<u32>(t.wrapping_add(0x18)) != 0 {
        up(g);
        return Ret::eax(fr.eax);
    }
    let m = g.read::<u32>(t.wrapping_add(0x34));
    if g.read::<u16>(m.wrapping_add(0x90)) & 0x400 == 0 {
        up(g);
        return Ret::eax(m);
    }
    let b = t + 0x70;
    crate::s7_misc::parent_append(
        g,
        img,
        f,
        b,
        [0x42, 0x49, 0x4e, 0x56, 0x5c, 0x64, 0x70, 0x76, 0x7c],
    );
    let m = g.read::<u32>(t + 0x34);
    let c = t + 0x8c;
    let h = g.read::<u32>(m.wrapping_add(0x54));
    // call width: 4
    g.call(slot_target(g, w(0x8e), 262), c, &[h, c, c + 4, c + 8]);
    let h = g.read::<u32>(m.wrapping_add(0x54));
    // call width: 5 (slot 262's words under it)
    let out = g.call(slot_target(g, w(0xa3), 12), c, &[h, h, c, c + 4, c + 8]);
    g.write::<f32>(c + 4, crate::m0018::st0_f32(out.st0));
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x1c);
    let e = t + 0x38;
    for (i, v) in (0u32..).zip([e, w(0xc3), w(0xca), b, w(0xd4), 1]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0xe0), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0xe9));
    let q = g
        .read::<i32>(t + 0x60)
        .checked_div(g.read::<i32>(t + 0x30))
        .unwrap_or(0);
    g.write::<i32>(t + 0x60, q);
    let v = g.read::<u32>(e + 0x2c) | 1;
    g.write::<u32>(e + 0x2c, v);
    g.write::<u32>(e + 0x30, 0);
    let node = g.read::<u32>(cursor);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next.wrapping_add(8), prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev.wrapping_add(4), next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    up(g);
    Ret::eax(next)
}

/// [`strike_start_0273`] (the `rel32`:
/// [`effect_node`](crate::sb_nodes::effect_node)).
pub const STRIKE_START_0273: Sig = Sig {
    len: 304,
    hash: 0x0fb0_9720_202f_7e86,
    calls: &[(0xe0, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// The ring node `S[0]` of a task cursor unlinked from its ring (its links
/// to itself). Returns its old next link (the `eax` the inlined code
/// leaves).
fn ring_unlink<G: Game>(g: &mut G, cursor: u32) -> u32 {
    let node = g.read::<u32>(cursor);
    let next = g.read::<u32>(node.wrapping_add(4));
    let prev = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(next.wrapping_add(8), prev);
    let prev = g.read::<u32>(node.wrapping_add(8));
    let next = g.read::<u32>(node.wrapping_add(4));
    g.write::<u32>(prev.wrapping_add(4), next);
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(node.wrapping_add(4), node);
    next
}

/// `magic_0273`'s summon task (0x10007a60, 1799 bytes; a task callback,
/// cdecl `(S)`; an aligned /GS frame, `e` its base after the pushes). With
/// `T = S[1]`, its state `T + 0x18` and timer `T + 0x2c` (the byte table
/// at `+0x42` gives the case; none, 12, or a state above 0x3e only counts
/// the timer up, `eax` the case or the state):
/// - 0 (0x00): the model node `(G_M, P, 0x14, 0)` set up
///   ([`model_node_init`](crate::sa_summon::model_node_init)); timer 1,
///   state 0x3c.
/// - 9 (0x3c, from timer 30), state up; 10 (0x3d): the model's value
///   0x68 ([`set_value`](crate::m0042::set_value)) and its facing to a
///   point ([`face_point`](crate::s4_small::face_point)).
/// - 1 (0xa): a motion `(G_M, 0xb0, G_P, 50.0)`
///   ([`motion_towards`](crate::s5_misc::motion_towards)); 3 (0x14): the
///   value 0xb1, timer 1.
/// - 4 (0x15, from timer 5): the throw's nodes (see [`throw_0273`]).
/// - 6 (0x1e): slot 531 `(*G_C, &v)` (only without bit 0 of `*G_F`), a
///   motion `(G_M, 0xb2, &v, 0)`; 7 (0x1f): the model id; when it is not
///   0 the value 0xb3.
/// - 11, 2, 5, 8 (0x3e, 0xb, 0x16, 0x20): the model's slot 20 `(*G_X)`
///   (bit 0 of `*G_F`) or the object's 411 `(*G_Y)`; non-zero gives the
///   state 0xa, 0x14, 0x1e or 0x2710, dispatched again.
/// - 0x2710: bit 0 of `*G_D` set, the model node leaves, its model set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the ring node
///   unlinked.
///
/// Case 6 with bit 0 of `*G_F` set hands the motion a vector slot 531
/// never wrote (a frame local): the port zeroes it (junk read, brief
/// 2026-10-24). Returns the last `eax` of the path.
#[allow(clippy::too_many_lines)] // one case per state, as the original
pub fn summon_task_0273<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x58);
    let state = t.wrapping_add(0x18);
    let timer = t.wrapping_add(0x2c);
    let up = |g: &mut G, a: u32| {
        let v = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, v);
    };
    let below = |g: &G, n: i32| g.read::<i32>(timer) < n;
    let frame = |at: u32, ecx: u32, edx: u32| Frame {
        at,
        esp: e.wrapping_sub(4),
        ecx,
        edx,
        ..Frame::default()
    };
    let words = |g: &mut G, at: u32, v: &[u32]| {
        for (i, &x) in (0u32..).zip(v) {
            g.write::<u32>(at + 4 * i, x);
        }
    };
    loop {
        let s = g.read::<u32>(state);
        if s == 0x2710 {
            let v = g.read::<u32>(w(0x646)) | 1;
            g.write::<u32>(w(0x646), v);
            let node = w(0x64c);
            crate::sa_misc::leave(g, node, s);
            g.write::<u32>(e + 0x44, node);
            crate::sa_summon::model_setup(g, img, &frame(call(0x65e), e + 0x44, 0));
            let eax = ring_unlink(g, cursor);
            up(g, timer);
            return Ret::eax(eax);
        }
        if s > 0x3e {
            up(g, timer);
            return Ret::eax(s);
        }
        let case = img.word(w(0x42) + s) & 0xff;
        let idle = |g: &mut G, eax: u32| {
            up(g, timer);
            Ret::eax(eax)
        };
        match case {
            // (the flag test, slot 20's word, slot 411's word, the next
            // state)
            11 | 2 | 5 | 8 => {
                let (bit, x, y, next) = match case {
                    11 => (0x4f, 0x5d, 0x68, 0xa),
                    2 => (0x8d, 0x9b, 0xa6, 0x14),
                    5 => (0xc8, 0xd6, 0xe1, 0x1e),
                    _ => (0x103, 0x111, 0x11c, 0x2710),
                };
                let table = w(bit + 6);
                let r = if g.read::<u8>(w(bit)) & 1 != 0 {
                    let a = g.read::<u32>(w(x));
                    // call width: 1
                    g.call(slot_target(g, table, 20), 0, &[a]).eax
                } else {
                    let a = g.read::<u32>(w(y));
                    // call width: 1
                    g.call(slot_target(g, table, 411), 0, &[a]).eax
                };
                if r == 0 {
                    return idle(g, 0);
                }
                g.write::<u32>(state, next);
            }
            0 => {
                let rec = e + 0x10;
                words(g, rec, &[w(0x165), w(0x16d), 0x14]);
                g.write::<u8>(rec + 0xc, 0);
                g.write::<u32>(w(0x180), w(0x184));
                let eax = crate::sa_summon::model_node_init(g, img, call(0x189), rec);
                g.write::<u32>(timer, 1);
                g.write::<u32>(state, 0x3c);
                return Ret::eax(eax);
            }
            9 => {
                if below(g, 0x1e) {
                    return idle(g, case);
                }
                up(g, state);
                up(g, timer);
                return Ret::eax(case);
            }
            10 => {
                crate::m0042::set_value(g, img, call(0x1dd), w(0x1d8), 0x68);
                let fp = frame(call(0x1ec), w(0x1e7), w(0x1e2));
                let eax = crate::s4_small::face_point(g, img, &fp).eax;
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            1 => {
                let rec = e + 0x10;
                words(g, rec, &[w(0x210), 0xb0, w(0x220), 0x4248_0000]);
                let eax = crate::s5_misc::motion_towards(g, img, &frame(call(0x22d), rec, 0)).eax;
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            3 => {
                let eax = crate::m0042::set_value(g, img, call(0x254), w(0x24f), 0xb1);
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(eax);
            }
            4 => {
                if below(g, 5) {
                    return idle(g, case);
                }
                let eax = throw_0273(g, img, f, e, t);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            6 => {
                let v = e + 0x44;
                if g.read::<u8>(w(0x5a2)) & 1 == 0 {
                    let a = g.read::<u32>(w(0x5b5));
                    // call width: 2
                    g.call(slot_target(g, w(0x5af), 531), 0, &[a, v]);
                } else {
                    // never written: 0 (junk read)
                    words(g, v, &[0; 4]);
                }
                let rec = e + 0x10;
                words(g, rec, &[w(0x5d0), 0xb2, v, 0]);
                let eax = crate::s5_misc::motion_towards(g, img, &frame(call(0x5e9), rec, 0)).eax;
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            7 => {
                let this = e + 0x44;
                g.write::<u32>(this, w(0x60d));
                let id = crate::sa_misc::model_id(g, img, call(0x612), this);
                if g.read::<u32>(this + 4) == 0 {
                    return idle(g, id);
                }
                let eax = crate::m0042::set_value(g, img, call(0x628), w(0x623), 0xb3);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            _ => return idle(g, case),
        }
    }
}

/// [`summon_task_0273`]'s case 4: a node appended, the model's position
/// (its `+0x4` less `K`, `w` 1.0), another node, the facing as an int
/// (its half-turn `+0x8000` added), a third node inserted with `(G_A,
/// G_B, G_C)` (colour 1, 1, 1, 2.0), a particle node (index 3); then one
/// group per target `i < T[0x30]` (`X = G_N + 0x2c i - 0x10`, the record
/// `p = G_R + 0x2c i`): `X` joins the parent, its heading to the target's
/// object ([`heading_to`](crate::s4_small::heading_to)), `p - 0x20`
/// appended, slot 366 `(obj[0x50], p - 4)` without bit 0 of the object's
/// `+0x90`, `p[0] += K`, `p + 0x64` appended (colour 1, 1, 1, 1.5), a
/// node inserted `(Q, P, G_W, p - 0x20, X, p + 0x64)` (`Q = G_Q + 0x14c
/// i`, its `+0x148` byte 1) and a particle node `(G_PB + 0x100 i, G_PL,
/// Q, *G_O, G_PP, 2)`; slot 239 `(*G_S, -5, 2)`; each target's bit 8
/// cleared and slot 388 `(obj[0x50], 15, 1)`. Returns the last call's
/// `eax`.
#[allow(clippy::too_many_lines)] // the throw set-up and its per-target loop, as the original
fn throw_0273<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, t: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let frame = |at: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at,
        esp,
        ecx,
        edx,
        ..Frame::default()
    };
    crate::sb_misc::node_append(g, img, call(0x28c), w(0x287), w(0x282), 4);
    crate::sa_misc::model_position(g, img, call(0x29b), w(0x296), w(0x291));
    let x = sub(
        f32::from_bits(g.read::<u32>(w(0x2a3))),
        f32::from_bits(g.read::<u32>(w(0x2ab))),
    );
    g.write::<u32>(w(0x2bb), 0x3f80_0000);
    g.write::<f32>(w(0x2c7), x);
    crate::sa_misc::node_append(g, img, call(0x2cc), w(0x2b5), w(0x2b0), 0);
    crate::sa_misc::facing_int(g, img, call(0x2db), w(0x2d6), w(0x2d1), e.wrapping_sub(4));
    let v = g.read::<u32>(w(0x2e1)).wrapping_add(0x8000);
    g.write::<u32>(w(0x2e1), v);
    let (node, list) = (w(0x2ef), w(0x2ea));
    crate::sb_misc::node_append(g, img, call(0x2f4), node, list, 0);
    // push ecx (the node append keeps ecx and edx); three operands
    let pushed = [w(0x304), w(0x2ff), w(0x2fa), node];
    let esp = e.wrapping_sub(0x14);
    for (i, &v) in (1u32..).zip(&pushed) {
        g.write::<u32>(esp + 4 * i, v);
    }
    for (o, v) in [
        (0x30f, 0x3f80_0000u32),
        (0x319, 0x3f80_0000),
        (0x323, 0x3f80_0000),
        (0x32d, 0x4000_0000),
    ] {
        g.write::<u32>(w(o), v);
    }
    crate::sa_misc::insert_with(g, img, &frame(call(0x336), w(0x309), list, esp));
    // push 3; sub esp, 8 (two of the words above stay); push G
    g.write::<u32>(esp + 0x10, 3);
    g.write::<u8>(w(0x349), 1);
    g.write::<u32>(esp + 4, w(0x34f));
    crate::s3_nodes::particle_node_with(g, img, &frame(call(0x354), w(0x33e), list, esp));
    let mut pa = w(0x361);
    let mut pb = w(0x369);
    g.write::<u32>(e + 0x24, pa);
    g.write::<u32>(e + 0x28, pb);
    g.write::<u32>(e + 0x20, 0);
    let n = |g: &G| g.read::<i32>(t + 0x30);
    let mut i = 0u32;
    if n(g) > 0 {
        let mut edi = w(0x37b);
        let mut p = w(0x383);
        g.write::<u32>(e + 0xc, p);
        loop {
            let x = edi.wrapping_sub(0x10);
            crate::s7_misc::parent_append(
                g,
                img,
                f,
                x,
                [
                    0x3a7, 0x3ae, 0x3b3, 0x3bb, 0x3c0, 0x3c8, 0x3d4, 0x3da, 0x39b,
                ],
            );
            let objs = g.read::<u32>(t + 0x34);
            g.write::<u32>(e + 0x18, w(0x3e5));
            let obj = g.read::<u32>(objs.wrapping_add(4 * i));
            g.write::<u32>(e + 0x10, obj);
            g.write::<u32>(e + 0x14, edi + 0xc);
            let fr4 = frame(call(0x3fc), e + 0x10, 0, e.wrapping_sub(4));
            crate::s4_small::heading_to(g, img, &fr4);
            let pn = p.wrapping_sub(0x20);
            g.write::<u32>(e + 0x44, pn);
            crate::sb_misc::node_append(g, img, call(0x413), pn, w(0x408), 0);
            let objs = g.read::<u32>(t + 0x34);
            let obj = g.read::<u32>(objs.wrapping_add(4 * i));
            if g.read::<u8>(obj.wrapping_add(0x90)) & 1 == 0 {
                let a = g.read::<u32>(obj.wrapping_add(0x50));
                // call width: 2
                g.call(slot_target(g, w(0x433), 366), 0, &[a, p.wrapping_sub(4)]);
            }
            let list = w(0x44a);
            let v = add(
                f32::from_bits(g.read::<u32>(p)),
                f32::from_bits(g.read::<u32>(w(0x456))),
            );
            let pc = p.wrapping_add(0x64);
            g.write::<f32>(p, v);
            crate::sb_misc::node_append(g, img, call(0x462), pc, list, 0);
            g.write::<u32>(e + 0x40, pc);
            for (o, v) in [
                (0x80u32, 0x3f80_0000u32),
                (0x84, 0x3f80_0000),
                (0x88, 0x3f80_0000),
                (0x8c, 0x3fc0_0000),
            ] {
                g.write::<u32>(p.wrapping_add(o), v);
            }
            let rec = e + 0x2c;
            for (k, v) in (0u32..).zip([pa, list, w(0x4ae), pn, x]) {
                g.write::<u32>(rec + 4 * k, v);
            }
            crate::nodes::node_insert(g, img, &frame(call(0x4bb), rec, list, e.wrapping_sub(4)));
            let pa_ = g.read::<u32>(e + 0x24);
            let pb_ = g.read::<u32>(e + 0x28);
            let o = g.read::<u32>(w(0x4c8));
            g.write::<u8>(pa_.wrapping_add(0x148), 1);
            g.write::<u32>(rec, pb_);
            g.write::<u32>(pb_, w(0x4d9));
            for (k, v) in (1u32..).zip([w(0x4e5), pa_, o, w(0x4f5), 2]) {
                g.write::<u32>(rec + 4 * k, v);
            }
            crate::sb_nodes::particle_node(g, img, call(0x502), rec);
            p = g.read::<u32>(e + 0xc).wrapping_add(0x2c);
            g.write::<u32>(e + 0xc, p);
            pb = g.read::<u32>(e + 0x28).wrapping_add(0x100);
            g.write::<u32>(e + 0x28, pb);
            i += 1;
            pa = pa_.wrapping_add(0x14c);
            edi = edi.wrapping_add(0x2c);
            g.write::<u32>(e + 0x24, pa);
            g.write::<u32>(e + 0x20, i);
            if i.cast_signed() >= n(g) {
                break;
            }
        }
    }
    let a = g.read::<u32>(w(0x543));
    // call width: 3
    let mut eax = g
        .call(slot_target(g, w(0x533), 239), 0, &[a, 0xffff_fffb, 2])
        .eax;
    let mut j = 0u32;
    while j.cast_signed() < n(g) {
        let objs = g.read::<u32>(t + 0x34);
        let obj = g.read::<u32>(objs.wrapping_add(4 * j));
        let v = g.read::<u16>(obj.wrapping_add(0x90)) & 0xfeff;
        g.write::<u16>(obj.wrapping_add(0x90), v);
        let objs = g.read::<u32>(t + 0x34);
        let obj = g.read::<u32>(objs.wrapping_add(4 * j));
        let a = g.read::<u32>(obj.wrapping_add(0x50));
        // call width: 3
        eax = g.call(slot_target(g, w(0x573), 388), 0, &[a, 0xf, 1]).eax;
        j += 1;
    }
    eax
}

/// [`summon_task_0273`] (the `rel32`s: its callees, and
/// `__security_check_cookie` at each return).
pub const SUMMON_TASK_0273: Sig = Sig {
    len: 1799,
    hash: 0xf27b_ebcd_77e1_18cd,
    calls: &[
        (0x155, Callee::Any),
        (0x189, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x1a8, Callee::Any),
        (0x1ca, Callee::Any),
        (0x1dd, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x1ec, Callee::Sig(&crate::s4_small::FACE_POINT)),
        (0x200, Callee::Any),
        (0x22d, Callee::Sig(&crate::s5_misc::MOTION_TOWARDS)),
        (0x241, Callee::Any),
        (0x254, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x26f, Callee::Any),
        (0x28c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x29b, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (0x2cc, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x2db, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x2f4, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x336, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x354, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x3fc, Callee::Sig(&crate::s4_small::HEADING_TO)),
        (0x413, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x462, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4bb, Callee::Sig(&crate::nodes::INSERT)),
        (0x502, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x598, Callee::Any),
        (0x5e9, Callee::Sig(&crate::s5_misc::MOTION_TOWARDS)),
        (0x5fd, Callee::Any),
        (0x612, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x628, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x63c, Callee::Any),
        (0x651, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x65e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x689, Callee::Any),
    ],
    imports: &[],
};

/// A node put last under a parent as `magic_0273`'s task code inlines it
/// for a static node: the old last `*G_L` read first (operand `l`), then
/// the stores `(dst, src)` of the operands, the old last stored into
/// `last_to`; an empty parent (the old last is the sentinel `p`) gets the
/// node as its first (`(first, value)`), else the old last's `+4` the
/// node (`next`); the u16 count `count` up and the node the last
/// (`(last_dst, last_src)`).
#[allow(clippy::too_many_arguments)]
fn static_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    l: u32,
    stores: &[(u32, u32)],
    last_to: u32,
    p: u32,
    first: (u32, u32),
    next: u32,
    count: u32,
    last: (u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let old = g.read::<u32>(w(l));
    for &(dst, src) in stores {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(last_to), old);
    if old == w(p) {
        g.write::<u32>(w(first.0), w(first.1));
    } else {
        g.write::<u32>(old.wrapping_add(4), w(next));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(last.0), w(last.1));
}

/// `magic_0273`'s summon start task (0x100081e0, 2109 bytes; a task
/// callback, cdecl `(S)`; an aligned /GS frame, `e` its base after the
/// pushes). With `T = S[1]`, its state `T + 0x18` and timer `T + 0x2c`
/// (the byte table at `+0x42` gives the case; case 10 or a state above
/// 0x20 only counts the timer up, `eax` the case or the state):
/// - 0 (0x00): see [`start_0273`]; timer up, state 0xa.
/// - 1 (0xa): the value 0xb8, the facing to a point, the tail node `(T[0x30])`
///   ([`tail_node`](crate::m0273::tail_node), `edx` = `T + 0x208`).
/// - 2 (0xb): the model id; when it is not 0 see [`throw_0273b`].
/// - 4 (0x14): the value 0xba, the static record (its float `K`); timer 1.
/// - 5 (0x15, from timer 5): see [`strikes_0273`].
/// - 7 (0x1e): slot 531 `(*G_C, &v)` (only without bit 0 of `*G_F`), a
///   motion `(G_M, 0xbb, &v, 0)`; 8 (0x1f): the model id; when it is not
///   0 the value 0xbc.
/// - 3, 9 (0xc, 0x20): the model's slot 20 `(*G_X)` (bit 0 of `*G_F`) or
///   the object's 411 `(*G_Y)`; non-zero gives the state 0x14 or 0x2710,
///   dispatched again; 6 (0x16): the state 0x1e, dispatched again.
/// - 0x2710: the model node leaves, its model set up, the ring node
///   unlinked.
///
/// Case 7 with bit 0 of `*G_F` set hands the motion a vector slot 531
/// never wrote (a frame local): the port zeroes it (junk read, brief
/// 2026-10-24). Words the original pushes from a register a callee left
/// (`ecx`) go to callees that never read them: the port writes 0 there.
/// Returns the last `eax` of the path.
#[allow(clippy::too_many_lines)] // one case per state, as the original
pub fn start_task_0273<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    let state = t.wrapping_add(0x18);
    let timer = t.wrapping_add(0x2c);
    let up = |g: &mut G, a: u32| {
        let v = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, v);
    };
    let frame = |at: u32, ecx: u32, edx: u32| Frame {
        at,
        esp: e.wrapping_sub(4),
        ecx,
        edx,
        ..Frame::default()
    };
    let words = |g: &mut G, at: u32, v: &[u32]| {
        for (i, &x) in (0u32..).zip(v) {
            g.write::<u32>(at + 4 * i, x);
        }
    };
    loop {
        let s = g.read::<u32>(state);
        if s == 0x2710 {
            let node = w(0x7ab);
            crate::sa_misc::leave(g, node, s);
            g.write::<u32>(e + 0x10, node);
            crate::sa_summon::model_setup(g, img, &frame(call(0x7bd), e + 0x10, 0));
            let eax = ring_unlink(g, cursor);
            up(g, timer);
            return Ret::eax(eax);
        }
        if s > 0x20 {
            up(g, timer);
            return Ret::eax(s);
        }
        let case = img.word(w(0x42) + s) & 0xff;
        let idle = |g: &mut G, eax: u32| {
            up(g, timer);
            Ret::eax(eax)
        };
        let model_id = |g: &mut G, o: u32, at: u32| {
            g.write::<u32>(e + 0x14, w(o));
            let id = crate::sa_misc::model_id(g, img, call(at), e + 0x14);
            (id, g.read::<u32>(e + 0x18))
        };
        match case {
            // (the flag test, slot 20's word, slot 411's word, the next
            // state)
            3 | 9 => {
                let (bit, x, y, next) = if case == 3 {
                    (0x4f, 0x5d, 0x68, 0x14)
                } else {
                    (0x93, 0xa1, 0xac, 0x2710)
                };
                let table = w(bit + 6);
                let r = if g.read::<u8>(w(bit)) & 1 != 0 {
                    let a = g.read::<u32>(w(x));
                    // call width: 1
                    g.call(slot_target(g, table, 20), 0, &[a]).eax
                } else {
                    let a = g.read::<u32>(w(y));
                    // call width: 1
                    g.call(slot_target(g, table, 411), 0, &[a]).eax
                };
                if r == 0 {
                    return idle(g, 0);
                }
                g.write::<u32>(state, next);
            }
            6 => g.write::<u32>(state, 0x1e),
            0 => {
                let eax = start_0273(g, img, f, e, t);
                up(g, timer);
                g.write::<u32>(state, 0xa);
                return Ret::eax(eax);
            }
            1 => {
                crate::m0042::set_value(g, img, call(0x354), w(0x34f), 0xb8);
                let fp = frame(call(0x363), w(0x35e), w(0x359));
                crate::s4_small::face_point(g, img, &fp);
                let esp = e.wrapping_sub(8);
                g.write::<u32>(esp + 4, g.read::<u32>(t + 0x30));
                let tail = Frame {
                    esp,
                    ..frame(call(0x371), 0, t + 0x208)
                };
                let eax = crate::m0273::tail_node(g, img, &tail).eax;
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            2 => {
                let (id, out) = model_id(g, 0x398, 0x39d);
                if out == 0 {
                    return idle(g, id);
                }
                let eax = throw_0273b(g, img, f, e);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            4 => {
                crate::m0042::set_value(g, img, call(0x511), w(0x50c), 0xba);
                let k = g.read::<u32>(w(0x519));
                let rf = Frame {
                    esp: e.wrapping_sub(0x14),
                    xmm: [0, 0, u128::from(k), 0],
                    ..frame(call(0x521), 0, 0)
                };
                let eax = crate::o4_tiny4::record_float(g, img, &rf).eax;
                up(g, state);
                g.write::<u32>(timer, 1);
                return Ret::eax(eax);
            }
            5 => {
                if g.read::<i32>(timer) < 5 {
                    return idle(g, case);
                }
                let eax = strikes_0273(g, img, f, e, t);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            7 => {
                let v = e + 0x2c;
                if g.read::<u8>(w(0x708)) & 1 == 0 {
                    let a = g.read::<u32>(w(0x71b));
                    // call width: 2
                    g.call(slot_target(g, w(0x715), 531), 0, &[a, v]);
                } else {
                    // never written: 0 (junk read)
                    words(g, v, &[0; 4]);
                }
                let rec = e + 0x14;
                words(g, rec, &[w(0x736), 0xbb, v, 0]);
                let eax = crate::s5_misc::motion_towards(g, img, &frame(call(0x74f), rec, 0)).eax;
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            8 => {
                let (id, out) = model_id(g, 0x773, 0x778);
                if out == 0 {
                    return idle(g, id);
                }
                let eax = crate::m0042::set_value(g, img, call(0x78e), w(0x789), 0xbc);
                up(g, state);
                up(g, timer);
                return Ret::eax(eax);
            }
            _ => return idle(g, case),
        }
    }
}

/// [`start_task_0273`]'s case 0: the summon nodes `(G_S, P, *G_A, 0x10)`
/// ([`summon_nodes`](crate::s4_small::summon_nodes)); slot 218
/// `(*G_A, &n)` the target list; each model node `i < *G_N` (0x98 apart
/// from `G_N0`) shown ([`show`](crate::m0070::show), byte 0xff) and kept
/// in `T + 0x208` (count `T + 0x30`) when its object `+0x50` is the
/// list's byte `i` (and `n > 0`); the model node `(G_M, P, 0x15, 0)`, the static link,
/// the node `(x)` linked last with 0xff, the static node; the node `X`
/// put last under `P`, the centre of the targets into `G_V`, its height
/// copied, the node `Y` put last under `P`; the facing as an int, the
/// bone node `(G_B)` (`ecx` = `T + 0x248`, `edx` = `X`). Returns the bone
/// node's `eax`.
#[allow(clippy::too_many_lines)] // the set-up's calls in the original's order
fn start_0273<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, t: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let frame = |at: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at,
        esp,
        ecx,
        edx,
        ..Frame::default()
    };
    let top = e.wrapping_sub(4);
    let rec = e + 0x14;
    let a = g.read::<u32>(w(0xee));
    for (i, v) in (0u32..).zip([w(0xfa), w(0x102), a, 0x10]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<u32>(w(0x114), w(0x118));
    crate::s4_small::summon_nodes(g, img, &frame(call(0x11d), rec, 0, top));
    let a = g.read::<u32>(w(0x12d));
    // call width: 2
    let targets = g.call(slot_target(g, w(0x127), 218), 0, &[a, e + 0x28]).eax;
    g.write::<u32>(t + 0x30, 0);
    g.write::<u32>(e + 0x24, targets);
    g.write::<u32>(e + 0x10, 0);
    if g.read::<i32>(w(0x147)) > 0 {
        let mut node = w(0x15b);
        let mut i = 0u32;
        loop {
            g.write::<u32>(rec, node);
            g.write::<u8>(rec + 4, 0xff);
            crate::m0070::show(g, img, call(0x16e), rec);
            let n = g.read::<i32>(e + 0x28);
            let at = targets.wrapping_add(i);
            if n > 0 && g.read::<u32>(node + 0x50) == u32::from(g.read::<u8>(at)) {
                let k = g.read::<u32>(t + 0x30);
                g.write::<u32>(t + 0x208 + 4 * k, node);
                up_u32(g, t + 0x30);
            }
            i += 1;
            node = node.wrapping_add(0x98);
            g.write::<u32>(e + 0x10, i);
            g.write::<u32>(e + 0x24, at.wrapping_add(1));
            if i.cast_signed() >= g.read::<i32>(w(0x1b4)) {
                break;
            }
        }
    }
    for (i, v) in (0u32..).zip([w(0x1c2), w(0x1ca), 0x15]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<u8>(rec + 0xc, 0);
    g.write::<u32>(w(0x1dd), w(0x1e1));
    crate::sa_summon::model_node_init(g, img, call(0x1e6), rec);
    crate::s4_small::static_link_last(g, img, &frame(call(0x1eb), 0, 0, top));
    // push ecx (a word the callee never reads: 0); push x
    let esp = e.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, w(0x1f1));
    g.write::<u32>(esp + 8, 0);
    crate::s4_small::link_node_x_ff(g, img, &frame(call(0x1fb), w(0x1f6), 0, esp));
    crate::m0273::static_node(g, img, &frame(call(0x203), 0, 0, top));
    g.write::<u16>(w(0x20f), 0xff);
    static_append(
        g,
        img,
        f,
        0x208,
        &[
            (0x217, 0x21b),
            (0x221, 0x225),
            (0x22b, 0x22f),
            (0x235, 0x239),
        ],
        0x23e,
        0x243,
        (0x24b, 0x24f),
        0x258,
        0x25f,
        (0x26f, 0x273),
    );
    g.write::<u32>(w(0x279), w(0x27d));
    crate::m0577::centre(g, img, &frame(call(0x282), w(0x269), w(0x264), top));
    let tail = g.read::<u32>(w(0x287));
    let h = g.read::<u32>(w(0x28f));
    g.write::<u32>(w(0x297), h);
    g.write::<u32>(w(0x29d), w(0x2a1));
    g.write::<u16>(w(0x2a8), 0xff);
    for (dst, src) in [
        (0x2b0, 0x2b4),
        (0x2ba, 0x2be),
        (0x2c4, 0x2c8),
        (0x2ce, 0x2d2),
    ] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x2d7), tail);
    if tail == w(0x2dc) {
        g.write::<u32>(w(0x2e4), w(0x2e8));
    } else {
        g.write::<u32>(tail.wrapping_add(4), w(0x2f1));
    }
    let n = g.read::<u16>(w(0x2f8)).wrapping_add(1);
    g.write::<u16>(w(0x2f8), n);
    g.write::<u32>(w(0x308), w(0x30c));
    crate::sa_misc::facing_int(g, img, call(0x311), w(0x302), w(0x2fd), top);
    let esp = e.wrapping_sub(8);
    g.write::<u32>(esp + 4, w(0x316));
    let bone = frame(call(0x326), t + 0x248, w(0x321), esp);
    crate::o6_fam_bone::bone_node_setup(g, img, &bone).eax
}

fn up_u32<G: Game>(g: &mut G, a: u32) {
    let v = g.read::<u32>(a).wrapping_add(1);
    g.write::<u32>(a, v);
}

/// [`start_task_0273`]'s case 2 after the model id: a motion `(G_M, 0xb9,
/// G_P, 20.0)`, a node appended, slot 262 `(*G_H, x, y, z)` and slot 12
/// `(*G_H)`'s ST0 over `y` (less `K`; `w` 1.0), a node appended (colour 1,
/// 1, 1, 1), inserted `(G_A, P, G_W, G_N, G_Y, node)`, a particle node
/// (index 0) and an effect node `(.., X, .., 0)`; then `*G_E |= 1`, `*G_E2
/// = 0`. Returns the effect node record's `eax`.
fn throw_0273b<G: Game>(g: &mut G, img: &Image, f: u32, e: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let frame = |at: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at,
        esp,
        ecx,
        edx,
        ..Frame::default()
    };
    let top = e.wrapping_sub(4);
    let rec = e + 0x14;
    for (i, v) in (0u32..).zip([w(0x3b4), 0xb9, w(0x3c4), 0x41a0_0000]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::s5_misc::motion_towards(g, img, &frame(call(0x3d1), rec, 0, top));
    crate::sb_misc::node_append(g, img, call(0x3e0), w(0x3db), w(0x3d6), 0);
    let h = g.read::<u32>(w(0x400));
    let (x, y, z) = (w(0x3fa), w(0x3f5), w(0x3ea));
    // call width: 4
    g.call(slot_target(g, w(0x3e5), 262), 0, &[h, x, y, z]);
    let h2 = g.read::<u32>(w(0x40d));
    // call width: 5 (slot 262's words under it)
    let out = g.call(slot_target(g, w(0x407), 12), 0, &[h2, h, x, y, z]);
    g.write::<f32>(w(0x418), crate::m0018::st0_f32(out.st0));
    let v = sub(
        f32::from_bits(g.read::<u32>(w(0x420))),
        f32::from_bits(g.read::<u32>(w(0x428))),
    );
    let (node, list) = (w(0x435), w(0x430));
    g.write::<u32>(w(0x43b), 0x3f80_0000);
    g.write::<f32>(w(0x447), v);
    crate::sb_misc::node_append(g, img, call(0x44c), node, list, 0);
    g.write::<u32>(e + 0x40, node);
    for o in [0x45a, 0x464, 0x46e, 0x478] {
        g.write::<u32>(w(o), 0x3f80_0000);
    }
    let ins = e + 0x2c;
    for (i, v) in (0u32..).zip([w(0x484), list, w(0x490), w(0x498), w(0x4a0)]) {
        g.write::<u32>(ins + 4 * i, v);
    }
    crate::nodes::node_insert(g, img, &frame(call(0x4a5), ins, list, top));
    // push 0; sub esp, 8 (words the callee never reads: 0); push G
    let esp = e.wrapping_sub(0x14);
    for (i, v) in (1u32..).zip([w(0x4b4), 0, 0, 0]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    g.write::<u8>(w(0x4ba), 1);
    crate::s3_nodes::particle_node_with(g, img, &frame(call(0x4c0), w(0x4af), 0, esp));
    // push 0; push ecx; push X; push ecx (the ecx words are never read: 0)
    for (i, v) in (1u32..).zip([0, w(0x4cb), 0, 0]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let eax = crate::s4_fam446::effect_node_a(g, img, &frame(call(0x4d6), w(0x4d1), 0, esp)).eax;
    let v = g.read::<u32>(w(0x4dc)) | 1;
    g.write::<u32>(w(0x4dc), v);
    g.write::<u32>(w(0x4e3), 0);
    eax
}

/// [`start_task_0273`]'s case 5 after its timer: a node appended (colour
/// 1, 1, 1, 0.08), inserted, a particle node (index 1); then for each
/// kept model node `M_i` (`T + 0x208`): its flag step
/// ([`this_flag_step`](crate::o4_list::this_flag_step)), its sprite set
/// up `(M_i, 20.0, 0.065, 0.25, 1, 0.4)`
/// ([`sprite_setup`](crate::m0273::sprite_setup)), and a strike task `X =
/// T + 0x34 + 0x9c i` put last in the DLL's task ring (callback `CB`, the
/// strike's start, vtable `V`; bit 0 of the ring's flags cleared) with
/// `X + 0x34 = M_i`, `X + 0x30 = T[0x30]`, `X + 0x18 = 0`; then the
/// static record `(*G_R |= 3, 0, K - 0, 0.3, 9.7, 20.0)`. Returns the
/// count (the particle node's `eax` with none).
fn strikes_0273<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, t: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let frame = |at: u32, ecx: u32, esp: u32| Frame {
        at,
        esp,
        ecx,
        ..Frame::default()
    };
    let top = e.wrapping_sub(4);
    let (node, list) = (w(0x557), w(0x552));
    crate::sb_misc::node_append(g, img, call(0x55c), node, list, 0);
    g.write::<u32>(e + 0x40, node);
    for (o, v) in [
        (0x56a, 0x3f80_0000u32),
        (0x574, 0x3f80_0000),
        (0x57e, 0x3f80_0000),
        (0x588, 0x3da3_d70a),
    ] {
        g.write::<u32>(w(o), v);
    }
    let ins = e + 0x2c;
    for (i, v) in (0u32..).zip([w(0x594), list, w(0x5a0), w(0x5a8), w(0x5b0)]) {
        g.write::<u32>(ins + 4 * i, v);
    }
    let fi = Frame {
        edx: list,
        ..frame(call(0x5b5), ins, top)
    };
    crate::nodes::node_insert(g, img, &fi);
    // push 1; sub esp, 8 (words the callee never reads: 0); push G
    let esp = e.wrapping_sub(0x14);
    for (i, v) in (1u32..).zip([w(0x5c4), 0, 0, 1]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    g.write::<u8>(w(0x5ca), 1);
    let mut eax = crate::s3_nodes::particle_node_with(g, img, &frame(call(0x5d0), w(0x5bf), esp));
    g.write::<u32>(e + 0x10, 0);
    if g.read::<i32>(t + 0x30) > 0 {
        let mut slot = t + 0x208;
        g.write::<u32>(e + 0x24, slot);
        let mut x = t + 0x34;
        let mut i = 0u32;
        loop {
            let m = g.read::<u32>(slot);
            // push ecx: a word the flag step never reads (0)
            g.write::<u32>(e.wrapping_sub(4), 0);
            let fs = frame(call(0x5fa), m, e.wrapping_sub(8));
            crate::o4_list::this_flag_step(g, img, &fs);
            let m = g.read::<u32>(slot);
            let rec = e + 0x2c;
            for (k, v) in (0u32..).zip([m, 0x41a0_0000, 0x3d85_1eb8, 0x3e80_0000]) {
                g.write::<u32>(rec + 4 * k, v);
            }
            g.write::<u8>(rec + 0x10, 1);
            g.write::<u32>(rec + 0x14, 0x3ecc_cccd);
            crate::m0273::sprite_setup(g, img, &frame(call(0x631), rec, top));
            let m = g.read::<u32>(slot);
            let n = g.read::<u32>(t + 0x30);
            g.write::<u32>(x + 8, x);
            g.write::<u32>(x + 4, x);
            g.write::<u32>(x + 0x10, w(0x645));
            g.write::<u32>(x + 0x14, x);
            g.write::<u32>(x, w(0x64e));
            let tail = g.read::<u32>(w(0x653));
            g.write::<u32>(x + 8, tail);
            g.write::<u32>(x + 4, w(0x65c));
            let tail = g.read::<u32>(w(0x661));
            g.write::<u32>(tail.wrapping_add(4), x);
            let v = g.read::<u32>(w(0x670)) & !1;
            g.write::<u32>(w(0x670), v);
            g.write::<u32>(w(0x67b), x);
            g.write::<u32>(x + 0x34, m);
            i += 1;
            g.write::<u32>(x + 0x30, n);
            g.write::<u32>(x + 0x18, 0);
            slot += 4;
            g.write::<u32>(e + 0x10, i);
            g.write::<u32>(e + 0x24, slot);
            x += 0x9c;
            if i.cast_signed() >= g.read::<i32>(t + 0x30) {
                break;
            }
        }
        eax = i;
    }
    let k = f32::from_bits(g.read::<u32>(w(0x6ac)));
    let b = g.read::<u8>(w(0x6b2)) | 3;
    g.write::<u8>(w(0x6b2), b);
    let v = sub(k, 0.0);
    g.write::<u32>(w(0x6c0), 0);
    g.write::<u32>(w(0x6ca), 0x3e99_999a);
    g.write::<u32>(w(0x6d4), 0x411b_3333);
    g.write::<u32>(w(0x6de), 0x41a0_0000);
    g.write::<f32>(w(0x6ea), v);
    eax
}

/// [`start_task_0273`] (the `rel32`s: its callees, and
/// `__security_check_cookie` at each return).
pub const START_TASK_0273: Sig = Sig {
    len: 2109,
    hash: 0x3da1_f7ed_7851_5e7a,
    calls: &[
        (0xe5, Callee::Any),
        (0x11d, Callee::Sig(&crate::s4_small::SUMMON_NODES)),
        (0x16e, Callee::Sig(&crate::m0070::SHOW)),
        (0x1e6, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x1eb, Callee::Sig(&crate::s4_small::STATIC_LINK_LAST)),
        (0x1fb, Callee::Sig(&crate::s4_small::LINK_NODE_X_FF)),
        (0x203, Callee::Sig(&crate::m0273::STATIC_NODE)),
        (0x282, Callee::Sig(&crate::m0577::CENTRE)),
        (0x311, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (
            0x326,
            Callee::Sig(&crate::o6_fam_bone::BONE_NODE_SETUP_0273_7770),
        ),
        (0x341, Callee::Any),
        (0x354, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x363, Callee::Sig(&crate::s4_small::FACE_POINT)),
        (0x371, Callee::Sig(&crate::m0273::TAIL_NODE)),
        (0x388, Callee::Any),
        (0x39d, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x3d1, Callee::Sig(&crate::s5_misc::MOTION_TOWARDS)),
        (0x3e0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x44c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4a5, Callee::Sig(&crate::nodes::INSERT)),
        (0x4c0, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x4d6, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x4fe, Callee::Any),
        (0x511, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x521, Callee::Sig(&crate::o4_tiny4::RECORD_FLOAT_0273_7690)),
        (0x53f, Callee::Any),
        (0x55c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5b5, Callee::Sig(&crate::nodes::INSERT)),
        (0x5d0, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (
            0x5fa,
            Callee::Sig(&crate::o4_list::THIS_FLAG_STEP_0273_7390),
        ),
        (0x631, Callee::Sig(&crate::m0273::SPRITE_SETUP)),
        (0x6fe, Callee::Any),
        (0x74f, Callee::Sig(&crate::s5_misc::MOTION_TOWARDS)),
        (0x763, Callee::Any),
        (0x778, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x78e, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x7a2, Callee::Any),
        (0x7b0, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x7bd, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x7e8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0388`'s drift task (0x100068b0, 648 bytes; a task callback,
/// cdecl `(S)`, no frame of its own). With `T = S[1]`, its state `T +
/// 0x18`, timer `T + 0x1c` (counted down by every return but the drift's
/// last steps, which set it first), the model record `M = T[0x2c]` and
/// its objects `A = M[0x13c]`, `B = M[0x144]`:
/// - 0: state 0x14, dispatched again; 0x14: `T + 0x24` = 7.0, timer 60,
///   then 0x15: `T + 0x24 *= K` added to `A + 0x20`; at timer 0 state
///   0xa, dispatched again;
/// - 0xa: the drift words `T + 0x20`, `+0x24`, `+0x28` 0, then 0xb: `+0x24
///   += K1`, `+0x20 += 2`, `+0x28 += K2`, `B + 0x20` (an int) grows by
///   `+0x20`, `A`'s point moved ([`drift_point`]); from `+0x20` 0x80 on
///   state 0xc, timer 0x4f;
/// - 0xc: `B + 0x20` grows, the point moved; at timer 0 state 0xd, timer
///   0xef; 0xd: `+0x20 -= 4`, `B + 0x20` grows by it, at 0 or below
///   state 0xe; 0xe (and 0xd): the point moved;
/// - 0x2710: the node `S[0]` unlinked from its ring, state 20000;
/// - any other state: only the timer.
///
/// Returns `eax` as the path leaves it (the state, `state - 0xb` from
/// 0xf to 0x270f, `A`, or the node's old next link).
pub fn drift_task_0388<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(img.word(f + o)));
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let state = t.wrapping_add(0x18);
    let timer = t.wrapping_add(0x1c);
    let down = |g: &mut G, eax: u32| {
        let v = g.read::<u32>(timer).wrapping_sub(1);
        g.write::<u32>(timer, v);
        Ret::eax(eax)
    };
    let m = |g: &G| g.read::<u32>(t.wrapping_add(0x2c));
    let grow = |g: &mut G| {
        let b = g.read::<u32>(m(g).wrapping_add(0x144));
        let v = g.read::<u32>(b.wrapping_add(0x20));
        let d = g.read::<u32>(t.wrapping_add(0x20));
        g.write::<u32>(b.wrapping_add(0x20), v.wrapping_add(d));
    };
    loop {
        let s = g.read::<u32>(state);
        match s {
            0 => g.write::<u32>(state, 0x14),
            0xa => {
                up_u32(g, state);
                for o in [0x20u32, 0x24, 0x28] {
                    g.write::<u32>(t.wrapping_add(o), 0);
                }
                return Ret::eax(drift_0388(g, img, f, t));
            }
            0x14 | 0x15 => {
                if s == 0x14 {
                    g.write::<u32>(t.wrapping_add(0x24), 0x40e0_0000);
                    g.write::<u32>(timer, 0x3c);
                    g.write::<u32>(state, 0x15);
                }
                let v = mul(g.read::<f32>(t.wrapping_add(0x24)), k(g, 0x72));
                let a = g.read::<u32>(m(g).wrapping_add(0x13c));
                g.write::<f32>(t.wrapping_add(0x24), v);
                let x = add(v, g.read::<f32>(a.wrapping_add(0x20)));
                g.write::<f32>(a.wrapping_add(0x20), x);
                if g.read::<i32>(timer) > 0 {
                    return down(g, a);
                }
                g.write::<u32>(state, 0xa);
            }
            0xb => return Ret::eax(drift_0388(g, img, f, t)),
            0xc => {
                grow(g);
                let a = drift_point(g, t);
                if g.read::<i32>(timer) > 0 {
                    return down(g, a);
                }
                up_u32(g, state);
                g.write::<u32>(timer, 0xef);
                return Ret::eax(a);
            }
            0xd | 0xe => {
                if s == 0xd {
                    let v = g.read::<u32>(t.wrapping_add(0x20)).wrapping_sub(4);
                    g.write::<u32>(t.wrapping_add(0x20), v);
                    grow(g);
                    if g.read::<i32>(t.wrapping_add(0x20)) <= 0 {
                        up_u32(g, state);
                    }
                }
                let a = drift_point(g, t);
                return down(g, a);
            }
            0x2710 => {
                let eax = ring_unlink(g, cursor);
                g.write::<u32>(state, 0x4e20);
                return down(g, eax);
            }
            // the table's other cases and the states past it up to 0x2710
            // leave `state - 0xb`; the others the state
            0xf..=0x13 | 0x16..=0x270f => return down(g, s - 0xb),
            _ => return down(g, s),
        }
    }
}

/// `A = T[0x2c][0x13c]`: `A + 0x1c -= T + 0x24`, `A + 0x20 -= T + 0x28`,
/// `A + 0x24 += T + 0x24` (`A` read again for each). Returns `A`.
fn drift_point<G: Game>(g: &mut G, t: u32) -> u32 {
    let a = |g: &G| {
        let m = g.read::<u32>(t.wrapping_add(0x2c));
        g.read::<u32>(m.wrapping_add(0x13c))
    };
    let p = a(g);
    let v = sub(
        g.read::<f32>(p.wrapping_add(0x1c)),
        g.read::<f32>(t.wrapping_add(0x24)),
    );
    g.write::<f32>(p.wrapping_add(0x1c), v);
    let p = a(g);
    let v = sub(
        g.read::<f32>(p.wrapping_add(0x20)),
        g.read::<f32>(t.wrapping_add(0x28)),
    );
    g.write::<f32>(p.wrapping_add(0x20), v);
    let p = a(g);
    let v = add(
        g.read::<f32>(p.wrapping_add(0x24)),
        g.read::<f32>(t.wrapping_add(0x24)),
    );
    g.write::<f32>(p.wrapping_add(0x24), v);
    p
}

/// [`drift_task_0388`]'s state 0xb: the drift steps (`K1`, `K2` at
/// operands `+0xc5`, `+0xe1`), the point moved; from `T + 0x20` 0x80 on
/// state 0xc and timer 0x4f, else the timer down. Returns `A`.
fn drift_0388<G: Game>(g: &mut G, img: &Image, f: u32, t: u32) -> u32 {
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(img.word(f + o)));
    let v = add(g.read::<f32>(t.wrapping_add(0x24)), k(g, 0xc5));
    let n = g.read::<u32>(t.wrapping_add(0x20)).wrapping_add(2);
    g.write::<u32>(t.wrapping_add(0x20), n);
    let m = g.read::<u32>(t.wrapping_add(0x2c));
    let c = g.read::<u32>(t.wrapping_add(0x20));
    g.write::<f32>(t.wrapping_add(0x24), v);
    let v = add(g.read::<f32>(t.wrapping_add(0x28)), k(g, 0xe1));
    g.write::<f32>(t.wrapping_add(0x28), v);
    let b = g.read::<u32>(m.wrapping_add(0x144));
    let x = g.read::<u32>(b.wrapping_add(0x20)).wrapping_add(c);
    g.write::<u32>(b.wrapping_add(0x20), x);
    let a = drift_point(g, t);
    if g.read::<i32>(t.wrapping_add(0x20)) < 0x80 {
        let v = g.read::<u32>(t.wrapping_add(0x1c)).wrapping_sub(1);
        g.write::<u32>(t.wrapping_add(0x1c), v);
        return a;
    }
    up_u32(g, t.wrapping_add(0x18));
    g.write::<u32>(t.wrapping_add(0x1c), 0x4f);
    a
}

/// [`drift_task_0388`] (no callees; its jump back to the dispatch is a
/// `rel32` inside the function).
pub const DRIFT_TASK_0388: Sig = Sig {
    len: 648,
    hash: 0x824a_3cda_219d_c38a,
    calls: &[],
    imports: &[],
};

/// A component of [`colour_task_0388`]'s vectors: the random's table, `K`,
/// `K'`, and whether `u = v - (r - K) K'` (else `+`).
type Comp0388 = (u32, u32, u32, bool);

/// One of [`colour_task_0388`]'s four colour objects: its offset in `T`,
/// the call, two components, the third's `(table, K, K')`, the alpha's
/// table.
type Set0388 = (u32, u32, [Comp0388; 2], [u32; 3], u32);

/// [`colour_task_0388`]'s frame: the function `f`, the block `T`, `ebp`
/// and the words still pushed for the next calls (slot 237's while they
/// stay on the stack).
struct Colour0388 {
    f: u32,
    t: u32,
    ebp: u32,
    words: Vec<u32>,
}

impl Colour0388 {
    /// The operand at `o`.
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    /// The float global at operand `o`.
    fn k<G: Game>(&self, g: &G, img: &Image, o: u32) -> f32 {
        f32::from_bits(g.read::<u32>(self.w(img, o)))
    }

    /// The local `ebp - o` as a float.
    fn local<G: Game>(&self, g: &G, o: u32) -> f32 {
        f32::from_bits(g.read::<u32>(self.ebp.wrapping_sub(o)))
    }

    /// The callee `esp` (the return address under the pushed words).
    fn esp(&self) -> u32 {
        let n = u32::try_from(self.words.len()).unwrap_or(0);
        self.ebp.wrapping_sub(0x30 + 4 * n + 4)
    }

    /// Slot 237 `(M[0x54], 0x70, 0, &v)` (`M = T[0x24]`, `v` at `ebp -
    /// 0x14`; the table at operand `o`): its words stay pushed.
    fn place<G: Game>(&mut self, g: &mut G, img: &Image, o: u32) {
        let m = g.read::<u32>(self.t.wrapping_add(0x24));
        let h = g.read::<u32>(m.wrapping_add(0x54));
        self.words = vec![h, 0x70, 0, self.ebp.wrapping_sub(0x14)];
        // call width: 4
        g.call(slot_target(g, self.w(img, o), 237), 0, &self.words);
    }

    /// Slot 307 (`frnd`, the table at operand `o`): its ST0 as the f32
    /// the original's `fstp` stores.
    fn rnd<G: Game>(&self, g: &mut G, img: &Image, o: u32) -> f32 {
        // call width: 4 (slot 237's words while they stay on the stack)
        let out = g.call(slot_target(g, self.w(img, o), 307), 0, &self.words);
        crate::m0018::st0_f32(out.st0)
    }

    /// The colour object `obj` set up from the vector at `v` with the
    /// alpha `x` in `xmm2` ([`colour_setup_0645`](crate::r20::colour_setup_0645)).
    fn colour<G: Game>(&self, g: &mut G, img: &Image, call: u32, obj: u32, v: u32, x: f32) {
        let fr = Frame {
            at: img.rel32_target(self.f + call),
            esp: self.esp(),
            ecx: obj,
            edx: v,
            xmm: [0, 0, u128::from(x.to_bits()), 0],
            ..Frame::default()
        };
        crate::r20::colour_setup_0645(g, img, &fr);
    }

    /// The local vector `v` moved by three randoms: `v_i += r K - K'` (the
    /// operands `(table, K, K')` of each), then the alpha from a fourth
    /// random (`table`).
    fn jitter<G: Game>(&self, g: &mut G, img: &Image, ops: [(u32, u32, u32); 3]) {
        for ((t, k1, k2), at) in ops.into_iter().zip([0x14u32, 0x10, 0xc]) {
            let r = self.rnd(g, img, t);
            let x = add(
                sub(mul(r, self.k(g, img, k1)), self.k(g, img, k2)),
                self.local(g, at),
            );
            g.write::<f32>(self.ebp.wrapping_sub(at), x);
        }
    }

    /// The colour words of the record `T + 0x28`: `(c, c, c, 128.0)` at
    /// `+0x44..+0x50` (`+0x4c`, `+0x48`, `+0x44` first), `+0x54..+0x5c` 0
    /// (from the top), `+0x60` 128.0.
    fn colour_words<G: Game>(&self, g: &mut G, c: u32) {
        let t = self.t;
        for o in [0x4cu32, 0x48, 0x44] {
            g.write::<u32>(t.wrapping_add(o), c);
        }
        g.write::<u32>(t.wrapping_add(0x50), 0x4300_0000);
        for o in [0x5cu32, 0x58, 0x54] {
            g.write::<u32>(t.wrapping_add(o), 0);
        }
        g.write::<u32>(t.wrapping_add(0x60), 0x4300_0000);
    }

    /// The record `T + 0x28`'s vectors and float `K` (operand `o`)
    /// ([`vectors_then_float`](crate::o4_tiny4::vectors_then_float)).
    fn vectors<G: Game>(&self, g: &mut G, img: &Image, call: u32, o: u32) -> u32 {
        let k = g.read::<u32>(self.w(img, o));
        let fr = Frame {
            at: img.rel32_target(self.f + call),
            esp: self.esp(),
            ecx: self.t.wrapping_add(0x28),
            xmm: [0, u128::from(k), 0, 0],
            ..Frame::default()
        };
        crate::o4_tiny4::vectors_then_float(g, img, &fr).eax
    }
}

/// `magic_0388`'s colour task (0x10006dd0, 2097 bytes; a task callback,
/// cdecl `(S)`; a /GS `ebp` frame: the vector `v` at `ebp - 0x14` that
/// slot 237 fills, the vector `u` at `ebp - 0x24`). With `T = S[1]`, its
/// state `T + 0x18`, timer `T + 0x1c` (counted down by every return),
/// the object index `T + 0x20` and the colour objects `T + 0xc0 +
/// 0x2c0 i`:
/// - 0: slot 237, four colour objects (`T + 0xc0`, `+0x380`, `+0x640`,
///   `+0x900`) set up from `u = v ± (r - K) K'` (`u + 0xc` 1.0) with a
///   random alpha; the root node `T + 0x28`, its colour `(128, 128, 128,
///   128)` and float, the effect node `T + 0x84` (`+0x2c |= 7`, `+0x30`
///   0); state 1, timer 60, index 4.
/// - 1 (at timer 0): slot 237, `v` moved by three randoms, object `T[0x20]`
///   set up (alpha `r + K`), the colour `(96, 96, 96, 128)`; index and
///   state up, timer 15.
/// - 2 (at timer 0): as 1 with slot 237's words still pushed, then slot
///   234 `(T + 0x64, T + 0x54, T + 0x44)`, bit 0 of u16 `T + 0x80`, `T +
///   0x74..0x7c = (0, 0.1, 1)`; state 0xa and the timer `rand % 8 + 8`
///   (slot 324).
/// - 0xa (at timer 0): the object `T[0x20] % 12` when its `+0x18` is
///   20000: slot 237, `v` moved, the object set up (alpha `r - K`); index
///   up, the timer `rand % 8 + 1`.
/// - 0x2710: the ring node unlinked, state 20000.
///
/// The frame's random words (`ebp - 0x28`) and a 1.0 the original stores
/// at `ebp - 0x24` in cases 1 and 2 are never read: not written here.
/// Returns `eax` as the path leaves it.
#[allow(clippy::too_many_lines)] // one case per state, as the original
pub fn colour_task_0388<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let ebp = fr.esp.wrapping_sub(4);
    let mut c = Colour0388 {
        f,
        t,
        ebp,
        words: Vec::new(),
    };
    let state = t.wrapping_add(0x18);
    let timer = t.wrapping_add(0x1c);
    let down = |g: &mut G, eax: u32| {
        let v = g.read::<u32>(timer).wrapping_sub(1);
        g.write::<u32>(timer, v);
        Ret::eax(eax)
    };
    let s = g.read::<u32>(state);
    let index = |g: &G| g.read::<u32>(t.wrapping_add(0x20));
    let v = ebp.wrapping_sub(0x14);
    match s {
        0 => {
            c.place(g, img, 0x1a2);
            let u = ebp.wrapping_sub(0x24);
            g.write::<u32>(u + 0xc, 0x3f80_0000);
            // (the objects' offsets, the calls, and per component the
            // random's table, K, K' and whether `u = v - (r - K) K'`)
            let sets: [Set0388; 4] = [
                (
                    0xc0,
                    0x265,
                    [(0x1af, 0x1ce, 0x1db, false), (0x1d3, 0x1fd, 0x20a, false)],
                    [0x202, 0x231, 0x239],
                    0x23e,
                ),
                (
                    0x380,
                    0x31d,
                    [(0x26a, 0x282, 0x28f, false), (0x287, 0x2b1, 0x2c3, true)],
                    [0x2bb, 0x2e9, 0x2f1],
                    0x2f6,
                ),
                (
                    0x640,
                    0x3d9,
                    [(0x322, 0x33a, 0x34c, true), (0x344, 0x36d, 0x37f, true)],
                    [0x377, 0x3a5, 0x3ad],
                    0x3b2,
                ),
                (
                    0x900,
                    0x491,
                    [(0x3de, 0x3f6, 0x408, true), (0x400, 0x429, 0x436, false)],
                    [0x42e, 0x45d, 0x465],
                    0x46a,
                ),
            ];
            for (obj, call, comps, third, alpha) in sets {
                for ((tbl, k1, k2, minus), at) in comps.into_iter().zip([0x14u32, 0x10]) {
                    let r = c.rnd(g, img, tbl);
                    let d = mul(sub(r, c.k(g, img, k1)), c.k(g, img, k2));
                    let x = if minus {
                        sub(c.local(g, at), d)
                    } else {
                        add(d, c.local(g, at))
                    };
                    g.write::<f32>(u + (0x14 - at), x);
                }
                let [tbl, k1, k2] = third;
                let r = c.rnd(g, img, tbl);
                let x = add(
                    sub(c.local(g, 0xc), c.k(g, img, k1)),
                    mul(r, c.k(g, img, k2)),
                );
                g.write::<f32>(u + 8, x);
                let a = c.rnd(g, img, alpha);
                c.colour(g, img, call, t.wrapping_add(obj), u, a);
            }
            let root = t.wrapping_add(0x28);
            crate::m0070::node_init_root(g, img, img.rel32_target(f + 0x499), root);
            c.colour_words(g, 0x4300_0000);
            c.vectors(g, img, 0x4de, 0x4af);
            let ctor = Frame {
                at: img.rel32_target(f + 0x4e9),
                esp: c.esp(),
                ecx: t.wrapping_add(0x84),
                ..Frame::default()
            };
            let eax = crate::o4_ctor::effect_ctor_this(g, img, &ctor).eax;
            let b = g.read::<u32>(t.wrapping_add(0xb0)) | 6;
            g.write::<u32>(t.wrapping_add(0xb0), b);
            let b = g.read::<u32>(t.wrapping_add(0xb0)) | 1;
            g.write::<u32>(t.wrapping_add(0xb0), b);
            g.write::<u32>(t.wrapping_add(0xb4), 0);
            up_u32(g, state);
            g.write::<u32>(timer, 0x3c);
            let r = down(g, eax);
            g.write::<u32>(t.wrapping_add(0x20), 4);
            r
        }
        1 => {
            if g.read::<i32>(timer) > 0 {
                return down(g, 0);
            }
            c.place(g, img, 0x52);
            c.words.clear();
            c.jitter(
                g,
                img,
                [(0x5f, 0x7a, 0x87), (0x7f, 0xa9, 0xb6), (0xae, 0xd8, 0xec)],
            );
            let r = c.rnd(g, img, 0xdd);
            let a = add(r, c.k(g, img, 0x117));
            let obj = t
                .wrapping_add(0xc0)
                .wrapping_add(index(g).wrapping_mul(0x2c0));
            c.colour(g, img, 0x127, obj, v, a);
            c.colour_words(g, 0x42c0_0000);
            let eax = c.vectors(g, img, 0x16f, 0x12f);
            up_u32(g, t.wrapping_add(0x20));
            up_u32(g, state);
            g.write::<u32>(timer, 0xf);
            down(g, eax)
        }
        2 => {
            if g.read::<i32>(timer) > 0 {
                return down(g, 2);
            }
            c.place(g, img, 0x545);
            c.jitter(
                g,
                img,
                [
                    (0x552, 0x56a, 0x577),
                    (0x56f, 0x599, 0x5a6),
                    (0x59e, 0x5c8, 0x5dc),
                ],
            );
            let r = c.rnd(g, img, 0x5cd);
            let a = add(r, c.k(g, img, 0x607));
            let obj = t
                .wrapping_add(0xc0)
                .wrapping_add(index(g).wrapping_mul(0x2c0));
            c.colour(g, img, 0x617, obj, v, a);
            let mut words = vec![
                t.wrapping_add(0x64),
                t.wrapping_add(0x54),
                t.wrapping_add(0x44),
            ];
            c.colour_words(g, 0x42c0_0000);
            words.extend_from_slice(&c.words);
            // call width: 7 (slot 237's words stay under these)
            g.call(slot_target(g, c.w(img, 0x628), 234), 0, &words);
            let b = g.read::<u16>(t.wrapping_add(0x80)) | 1;
            g.write::<u16>(t.wrapping_add(0x80), b);
            g.write::<u32>(t.wrapping_add(0x7c), 1);
            g.write::<u32>(t.wrapping_add(0x78), 0x3dcc_cccd);
            g.write::<u32>(t.wrapping_add(0x74), 0);
            g.write::<u32>(state, 0xa);
            // call width: 0
            let r = g.call(slot_target(g, c.w(img, 0x675), 324), 0, &[]).eax;
            let n = r
                .cast_signed()
                .wrapping_rem(8)
                .wrapping_add(8)
                .cast_unsigned();
            g.write::<u32>(timer, n);
            down(g, n)
        }
        0xa => {
            if g.read::<i32>(timer) > 0 {
                return down(g, 0xa);
            }
            let i = index(g).cast_signed();
            let (q, j) = (i.wrapping_div(12), i.wrapping_rem(12));
            let obj = t
                .wrapping_add(0xc0)
                .wrapping_add(j.wrapping_mul(0x2c0).cast_unsigned());
            if g.read::<u32>(obj.wrapping_add(0x18)) != 0x4e20 {
                return down(g, q.cast_unsigned());
            }
            c.place(g, img, 0x735);
            c.words.clear();
            c.jitter(
                g,
                img,
                [
                    (0x742, 0x75d, 0x76a),
                    (0x762, 0x78c, 0x799),
                    (0x791, 0x7bb, 0x7c8),
                ],
            );
            let r = c.rnd(g, img, 0x7c0);
            let a = sub(r, c.k(g, img, 0x7ea));
            c.colour(g, img, 0x7fa, obj, v, a);
            up_u32(g, t.wrapping_add(0x20));
            // call width: 0
            let r = g.call(slot_target(g, c.w(img, 0x7ff), 324), 0, &[]).eax;
            let n = r
                .cast_signed()
                .wrapping_rem(8)
                .wrapping_add(1)
                .cast_unsigned();
            g.write::<u32>(timer, n);
            down(g, n)
        }
        0x2710 => {
            let eax = ring_unlink(g, cursor);
            let r = down(g, eax);
            g.write::<u32>(state, 0x4e20);
            r
        }
        _ => down(g, s),
    }
}

/// [`colour_task_0388`] (the `rel32`s: the colour set-ups, the record's
/// vectors, the root node, the effect node, and `__security_check_cookie`
/// at each return; the jump into the shared timer store is inside).
pub const COLOUR_TASK_0388: Sig = Sig {
    len: 2097,
    hash: 0xd58a_6bc0_fc27_9234,
    calls: &[
        (0x127, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (
            0x16f,
            Callee::Sig(&crate::o4_tiny4::VECTORS_THEN_FLOAT_0388_6810),
        ),
        (0x18b, Callee::Any),
        (0x265, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x31d, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x3d9, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x491, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x499, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (
            0x4de,
            Callee::Sig(&crate::o4_tiny4::VECTORS_THEN_FLOAT_0388_6810),
        ),
        (
            0x4e9,
            Callee::Sig(&crate::o4_ctor::EFFECT_CTOR_THIS_0388_6D70),
        ),
        (0x524, Callee::Any),
        (0x617, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x6f0, Callee::Any),
        (0x7fa, Callee::Sig(&crate::r21::COLOUR_SETUP_0388)),
        (0x829, Callee::Any),
    ],
    imports: &[],
};

/// A node put last among a parent's children as the DLLs inline it
/// (operands of `f`: `(+0x14's P, +4's P, last, last, P, first, count,
/// last)`): u16 `+0x1a` 0xff, its own empty child list, `+0x14` and `+4`
/// the parent, `+8` the old last, the parent's first (empty) or the old
/// last's `+4` the node, the u16 count up, the node the last. No vtable.
fn child_last<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 8]) {
    let w = |o: u32| img.word(f + o);
    let [p14, p4, l1, l2, p, first, count, l3] = ops;
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(p14));
    g.write::<u32>(node + 4, w(p4));
    let last = g.read::<u32>(w(l1));
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(w(l2));
    if last == w(p) {
        g.write::<u32>(w(first), node);
    } else {
        g.write::<u32>(last.wrapping_add(4), node);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(l3), node);
}

/// A task node `n` put at the tail of the DLL's task ring (operands of
/// `f`: its vtable `vt`, its callback `cb`, the ring's last `l1`, the ring
/// `r`, the last again `l2`, the ring's flags `fl`, the last `l3`): its
/// own links first, `+0x10` the callback, `+0x14` itself; bit 0 of the
/// flags cleared.
fn task_tail<G: Game>(g: &mut G, img: &Image, f: u32, n: u32, ops: [u32; 7]) {
    let w = |o: u32| img.word(f + o);
    let [vt, cb, l1, r, l2, fl, l3] = ops;
    g.write::<u32>(n, w(vt));
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    g.write::<u32>(n + 0x10, w(cb));
    g.write::<u32>(n + 0x14, n);
    let last = g.read::<u32>(w(l1));
    g.write::<u32>(n + 8, last);
    g.write::<u32>(n + 4, w(r));
    let last = g.read::<u32>(w(l2));
    g.write::<u32>(last.wrapping_add(4), n);
    let v = g.read::<u32>(w(fl)) & !1;
    g.write::<u32>(w(fl), v);
    g.write::<u32>(w(l3), n);
}

/// `magic_0388`'s summon task (0x10007770, 2217 bytes; a task callback,
/// cdecl `(S)`; an aligned frame, `e` its base after the pushes). With
/// `T = S[1]`, its state `T + 0x18` and timer `T + 0x1c` (counted down by
/// every return) and the model node `M = T + 0x2234`:
/// - 0: see [`start_0388`]; then timer 30, state 0x14, dispatched again;
/// - 0x14 (at timer 0): timer 0x87, state 0xa, dispatched again;
/// - 0xa (at timer 0): a node appended (colour 1, 1, 1, 2.0), inserted
///   `(T + 0x27e4, P, G_W, T + 0x2c30, G_N, node)` and its particle
///   (index 2); state 0xb, timer 0x40;
/// - 0xb (at timer 0 exactly): `T + 0x22f8` appended, slot 325 `(T +
///   0x2314, T + 0x2c4c)`, `T + 0x2324` appended (colour 1, 1, 1, 0.49),
///   inserted and its particle (index 3); the root node `T + 0x2c70`,
///   its colour `(96, 96, 96, 128)`, started with `K`; slot 8 `(T[0x2288],
///   0)`; state 0xc, timer 9;
/// - 0xc (at timer 0): `T + 0x23a8` put last under `P` (colour 1, 1, 1,
///   0.28), inserted, a particle node `(T + 0x2b30, .., 1)`; state 0x1e,
///   dispatched again;
/// - 0x1e: state 0x1f, timer 100, then 0x1f (at timer 0): the model fades
///   `(M, 0.0, 60)`; state 0x20, timer 0x59;
/// - 0x20 (at timer 0): slot 325 `(T + 0x2314, T + 0x2c4c)`, `T + 0x38`
///   and `T + 0x221c` 10000, `T + 0x2920` and `T + 0x27d4` = `T + 0x22f8`;
///   state 0x2710, dispatched again;
/// - 0x2710: once `T + 0x21f8`, `T + 0x38`, `T + 0x221c` are 20000, `M`
///   leaves, its model set up, the ring node unlinked.
///
/// Words the original pushes from stack space a call left (the particle
/// node with's two middle words) are never read: 0 here. Returns `eax`
/// as the path leaves it.
#[allow(clippy::too_many_lines)] // one case per state, as the original
pub fn summon_task_0388<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    let state = t.wrapping_add(0x18);
    let timer = t.wrapping_add(0x1c);
    let down = |g: &mut G, eax: u32| {
        let v = g.read::<u32>(timer).wrapping_sub(1);
        g.write::<u32>(timer, v);
        Ret::eax(eax)
    };
    let busy = |g: &G| g.read::<i32>(timer) > 0;
    let frame = |at: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at,
        esp,
        ecx,
        edx,
        ..Frame::default()
    };
    let top = e.wrapping_sub(4);
    let colour = |g: &mut G, at: u32, a: u32| {
        for (i, v) in (0u32..).zip([0x3f80_0000, 0x3f80_0000, 0x3f80_0000, a]) {
            g.write::<u32>(t.wrapping_add(at + 4 * i), v);
        }
    };
    let insert = |g: &mut G, at: u32, words: [u32; 6]| {
        let rec = e + 0x30;
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(rec + 4 * i, v);
        }
        crate::nodes::node_insert(g, img, &frame(call(at), rec, words[1], top));
    };
    // push index; sub esp, 8 (words never read: 0); push node
    let particle_with = |g: &mut G, at: u32, obj: u32, node: u32, index: u32| {
        let esp = e.wrapping_sub(0x14);
        for (i, v) in (1u32..).zip([node, 0, 0, index]) {
            g.write::<u32>(esp + 4 * i, v);
        }
        crate::s3_nodes::particle_node_with(g, img, &frame(call(at), obj, 0, esp))
    };
    loop {
        let s = g.read::<u32>(state);
        match s {
            0 => {
                start_0388(g, img, f, e, t);
                g.write::<u32>(timer, 0x1e);
                g.write::<u32>(state, 0x14);
            }
            0x14 => {
                if busy(g) {
                    return down(g, s);
                }
                g.write::<u32>(timer, 0x87);
                g.write::<u32>(state, 0xa);
            }
            0xa => {
                if busy(g) {
                    return down(g, 1);
                }
                let (node, list) = (t + 0x23d4, w(0x5f3));
                crate::sb_misc::node_append(g, img, call(0x5f8), node, list, 0);
                colour(g, 0x23f0, 0x4000_0000);
                let a = t + 0x27e4;
                insert(g, 0x655, [a, list, w(0x644), t + 0x2c30, w(0x650), node]);
                let eax = particle_with(g, 0x666, t + 0x2a30, a, 2);
                up_u32(g, state);
                g.write::<u32>(timer, 0x41);
                return down(g, eax);
            }
            0xb => {
                if g.read::<u32>(timer) != 0 {
                    return down(g, 2);
                }
                let p = t + 0x22f8;
                crate::sb_misc::node_append(g, img, call(0x699), p, w(0x692), 0);
                // call width: 2
                g.call(slot_target(g, w(0x6ac), 325), 0, &[t + 0x2314, t + 0x2c4c]);
                let (node, list) = (t + 0x2324, w(0x6c2));
                crate::sb_misc::node_append(g, img, call(0x6c7), node, list, 0);
                colour(g, 0x2340, 0x3efa_e147);
                let a = t + 0x2400;
                insert(g, 0x71e, [a, list, w(0x70d), p, w(0x719), node]);
                particle_with(g, 0x72f, t + 0x2930, a, 3);
                let root = t + 0x2c70;
                // the particle node with's four words stay on the stack
                let esp4 = e.wrapping_sub(0x14);
                crate::m0070::node_init_root(g, img, call(0x73a), root);
                for (o, v) in [
                    (0x2c94u32, 0x42c0_0000u32),
                    (0x2c90, 0x42c0_0000),
                    (0x2c8c, 0x42c0_0000),
                    (0x2c98, 0x4300_0000),
                    (0x2ca4, 0),
                    (0x2ca0, 0),
                    (0x2c9c, 0),
                    (0x2ca8, 0x4300_0000),
                ] {
                    g.write::<u32>(t.wrapping_add(o), v);
                }
                let k = g.read::<u32>(w(0x742));
                let start = Frame {
                    xmm: [0, u128::from(k), 0, 0],
                    ..frame(call(0x797), root, 0, esp4)
                };
                crate::sa_misc::node_start(g, img, &start);
                let h = g.read::<u32>(t + 0x2288);
                // call width: 2 (under these: the particle node with's
                // words, two of them stack space never written here)
                let eax = g.call(slot_target(g, w(0x79c), 8), 0, &[h, 0]).eax;
                up_u32(g, state);
                g.write::<u32>(timer, 0xa);
                return down(g, eax);
            }
            0xc => {
                if busy(g) {
                    return down(g, 3);
                }
                let c = t + 0x23a8;
                child_last(
                    g,
                    img,
                    f,
                    c,
                    [0x477, 0x47e, 0x483, 0x48b, 0x490, 0x498, 0x4a4, 0x4aa],
                );
                g.write::<u32>(c, w(0x4b0));
                colour(g, 0x23c4, 0x3e8f_5c29);
                let a = t + 0x2698;
                insert(g, 0x511, [a, w(0x4f8), w(0x500), t + 0x2c30, w(0x50c), c]);
                let o = g.read::<u32>(w(0x516));
                let b = t + 0x2b30;
                let rec = e + 0x30;
                g.write::<u32>(rec, b);
                g.write::<u32>(b, w(0x526));
                for (i, v) in (1u32..).zip([w(0x532), a, o, w(0x542), 1]) {
                    g.write::<u32>(rec + 4 * i, v);
                }
                crate::sb_nodes::particle_node(g, img, call(0x54f), rec);
                g.write::<u32>(state, 0x1e);
            }
            0x1e | 0x1f => {
                if s == 0x1e {
                    up_u32(g, state);
                    g.write::<u32>(timer, 0x64);
                }
                if busy(g) {
                    return down(g, 0);
                }
                let rec = e + 0x14;
                for (i, v) in (0u32..).zip([t + 0x2234, 0, 0x3c]) {
                    g.write::<u32>(rec + 4 * i, v);
                }
                let eax = crate::l3_common::chr_fade(g, img, call(0x809), rec);
                up_u32(g, state);
                g.write::<u32>(timer, 0x5a);
                return down(g, eax);
            }
            0x20 => {
                if busy(g) {
                    return down(g, s);
                }
                g.write::<u32>(t + 0x38, 0x2710);
                g.write::<u32>(t + 0x221c, 0x2710);
                // call width: 2
                g.call(slot_target(g, w(0x5a4), 325), 0, &[t + 0x2314, t + 0x2c4c]);
                let p = t + 0x22f8;
                g.write::<u32>(t + 0x2920, p);
                g.write::<u32>(t + 0x27d4, p);
                g.write::<u32>(state, 0x2710);
            }
            0x2710 => {
                if [0x21f8u32, 0x38, 0x221c]
                    .into_iter()
                    .any(|o| g.read::<u32>(t + o) != 0x4e20)
                {
                    return down(g, s);
                }
                let m = t + 0x2234;
                crate::sa_misc::leave(g, m, s);
                g.write::<u32>(e + 0x10, m);
                crate::sa_summon::model_setup(g, img, &frame(call(0x85d), e + 0x10, 0, top));
                let eax = ring_unlink(g, cursor);
                return down(g, eax);
            }
            // the byte table's other cases (1..9): its byte 4
            1..=9 => return down(g, 4),
            0x15..=0x1d => return down(g, s.wrapping_sub(0x1f)),
            _ => return down(g, s),
        }
    }
}

/// [`summon_task_0388`]'s state 0: the model node `M = T + 0x2234` `(M,
/// P, 0x14, 1)` ([`model_node_init`](crate::sa_summon::model_node_init),
/// `T + 0x22a0` 1); `A = T + 0x2350` last under `P`; slot 262 `(M[0x54],
/// B, B + 4, B + 8)` (`B = T + 0x236c`, `B + 0xc` 1.0); `C = T + 0x22cc`
/// last under `P` (`+0x24`, `+0x1c` 0); `M`'s facing as an int into `T +
/// 0x22ec` (slot 333's ST0 with bit 0 of `M + 0x90`, else slot 601 into a
/// frame local: `(K - a) K' / K''`, truncated); `D = T + 0x237c` last
/// under `P`; the model state `(M, T + 0x2398)`; `E = T + 0x254c` last
/// under `P` (`+0x13c` A, `+0x140` D, `+0x144` C, `+0x128` the bone
/// root, `+0x148` 0); `F = T + 0x2c30` last under `P` (`+0x38` 1.0,
/// `+0x3c` E); slot 211 `(T[0x2288], E + 0x9c)` after `T + 0x2c5c..` =
/// (2000, 10000, 15000); `M + 0x90 |= 0x10`, `M + 0x4c` = E; slot 8
/// `(T[0x2288], slot 7's answer & !0x10)`; the motion task `(T + 0x21e0,
/// M)`; the colour task `G = T + 0x20` and the drift task `H = T + 0x2204`
/// at the ring's tail (the colour objects' `+0x18` 20000, `G + 0x18..0x20`
/// 0, `G + 0x24` = M; `H + 0x2c` = E, `H + 0x18` 0).
#[allow(clippy::too_many_lines)] // the set-up in the original's order
fn start_0388<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, t: u32) {
    use crate::sb_sse::div;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let call = |o: u32| img.rel32_target(f + o);
    let m = t + 0x2234;
    let rec = e + 0x20;
    for (i, v) in (0u32..).zip([m, w(0x4d), 0x14]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<u8>(rec + 0xc, 1);
    g.write::<u32>(m, w(0x60));
    crate::sa_summon::model_node_init(g, img, call(0x65), rec);
    g.write::<u32>(t + 0x22a0, 1);
    let a = t + 0x2350;
    child_last(
        g,
        img,
        f,
        a,
        [0x88, 0x8f, 0x94, 0x9c, 0xa1, 0xa9, 0xb5, 0xbb],
    );
    g.write::<u32>(a, w(0xc1));
    let b = t + 0x236c;
    let h = g.read::<u32>(m + 0x54);
    // call width: 4
    g.call(slot_target(g, w(0xd4), 262), 0, &[h, b, b + 4, b + 8]);
    g.write::<u32>(t + 0x2378, 0x3f80_0000);
    let c = t + 0x22cc;
    g.write::<u32>(c, w(0xf9));
    child_last(
        g,
        img,
        f,
        c,
        [0x10c, 0x113, 0x118, 0x120, 0x125, 0x12d, 0x139, 0x13f],
    );
    g.write::<u32>(t + 0x22f0, 0);
    g.write::<u32>(t + 0x22e8, 0);
    let d = if g.read::<u8>(m + 0x90) & 1 != 0 {
        let h = g.read::<u32>(m + 0x54);
        // call width: 1
        let out = g.call(slot_target(g, w(0x161), 333), 0, &[h]);
        let x = crate::m0018::st0_f32(out.st0);
        g.write::<f32>(e + 0xc, x);
        sub(k(g, 0x178), x)
    } else {
        let h = g.read::<u32>(m + 0x50);
        // call width: 2
        g.call(slot_target(g, w(0x18d), 601), 0, &[h, e + 0x10]);
        sub(k(g, 0x1a0), g.read::<f32>(e + 0x10))
    };
    let d = div(mul(d, k(g, 0x1b1)), k(g, 0x1bf));
    g.write::<i32>(t + 0x22ec, crate::sb_sse::cvtt(d));
    let dn = t + 0x237c;
    child_last(
        g,
        img,
        f,
        dn,
        [0x1dc, 0x1e3, 0x1e8, 0x1f0, 0x1f5, 0x1fd, 0x209, 0x215],
    );
    g.write::<u32>(dn, w(0x21f));
    g.write::<u32>(e + 0x14, m);
    g.write::<u32>(e + 0x18, t + 0x2398);
    crate::m0070::model_state(g, img, call(0x22c), e + 0x14);
    let en = t + 0x254c;
    g.write::<u32>(en, w(0x238));
    child_last(
        g,
        img,
        f,
        en,
        [0x24b, 0x252, 0x257, 0x25f, 0x264, 0x26c, 0x278, 0x27e],
    );
    g.write::<u32>(en + 0x140, dn);
    g.write::<u32>(en + 0x128, w(0x28e));
    g.write::<u8>(en + 0x148, 0);
    g.write::<u32>(en + 0x13c, a);
    g.write::<u32>(en + 0x144, c);
    let fn_ = t + 0x2c30;
    child_last(
        g,
        img,
        f,
        fn_,
        [0x2c6, 0x2cd, 0x2d2, 0x2da, 0x2df, 0x2e7, 0x2f3, 0x2f9],
    );
    g.write::<u32>(fn_ + 0x38, 0x3f80_0000);
    g.write::<u32>(fn_ + 0x3c, en);
    g.write::<u32>(fn_, w(0x309));
    for (o, v) in [
        (0x2c5cu32, 0x44fa_0000u32),
        (0x2c60, 0x461c_4000),
        (0x2c64, 0x466a_6000),
    ] {
        g.write::<u32>(t + o, v);
    }
    let h = g.read::<u32>(t + 0x2288);
    // call width: 2
    g.call(slot_target(g, w(0x315), 211), 0, &[h, en + 0x9c]);
    let v = g.read::<u16>(m + 0x90) | 0x10;
    g.write::<u16>(m + 0x90, v);
    g.write::<u32>(m + 0x4c, en);
    let h = g.read::<u32>(t + 0x2288);
    // call width: 3 (slot 211's words stay under it)
    let r = g
        .call(slot_target(g, w(0x354), 7), 0, &[h, h, en + 0x9c])
        .eax;
    let h = g.read::<u32>(t + 0x2288);
    // call width: 2
    g.call(slot_target(g, w(0x36e), 8), 0, &[h, r & !0x10]);
    let mt = Frame {
        at: call(0x389),
        esp: e.wrapping_sub(4),
        ecx: t + 0x21e0,
        edx: m,
        ..Frame::default()
    };
    crate::m0388::motion_task(g, img, &mt);
    let gn = t + 0x20;
    task_tail(
        g,
        img,
        f,
        gn,
        [0x397, 0x3a4, 0x3ac, 0x3b6, 0x3bb, 0x3c4, 0x3cb],
    );
    for i in 0..12u32 {
        g.write::<u32>(gn + 0xd8 + 0x2c0 * i, 0x4e20);
    }
    g.write::<u32>(gn + 0x20, 0);
    g.write::<u32>(gn + 0x1c, 0);
    g.write::<u32>(gn + 0x18, 0);
    g.write::<u32>(gn + 0x24, m);
    let hn = t + 0x2204;
    task_tail(
        g,
        img,
        f,
        hn,
        [0x403, 0x410, 0x418, 0x422, 0x427, 0x430, 0x437],
    );
    g.write::<u32>(hn + 0x2c, en);
    g.write::<u32>(hn + 0x18, 0);
}

/// [`summon_task_0388`] (the `rel32`s: its callees; its jumps back to the
/// dispatch are inside).
pub const SUMMON_TASK_0388: Sig = Sig {
    len: 2217,
    hash: 0x0ae4_364f_2386_bc74,
    calls: &[
        (0x65, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x22c, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x389, Callee::Sig(&crate::m0388::MOTION_TASK)),
        (0x511, Callee::Sig(&crate::nodes::INSERT)),
        (0x54f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5f8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x655, Callee::Sig(&crate::nodes::INSERT)),
        (0x666, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x699, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6c7, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x71e, Callee::Sig(&crate::nodes::INSERT)),
        (0x72f, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x73a, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x797, Callee::Sig(&crate::sa_misc::NODE_START)),
        (0x809, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x850, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x85d, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0250`'s angle wrap: `a` above `hi` loses `step`, else below
/// `lo` (`lo > a`, ordered) gains `step2` (the operands `(hi, step, lo,
/// step2)` of `f`; the two steps are one constant read twice in some
/// copies).
fn wrap_0250<G: Game>(g: &G, img: &Image, f: u32, a: f32, ops: [u32; 4]) -> f32 {
    let k = |o: u32| f32::from_bits(g.read::<u32>(img.word(f + o)));
    let [hi, step, lo, step2] = ops;
    // comiss a, hi; jbe: only above (ordered)
    if a > k(hi) {
        sub(a, k(step))
    } else if k(lo) > a {
        add(a, k(step2))
    } else {
        a
    }
}

/// `magic_0250`'s start-up (0x10004260, 1705 bytes; run by its load step,
/// no arguments; a /GS `ebp` frame, the vector `v` at `ebp - 0x14`).
///
/// - Three words set, slot 719 `(PART, 8, 0)` into `*G_P`, the part set up
///   ([`part_init`](crate::l5_objhost::part_init)); the actors' mean
///   position into `v`
///   ([`mean_position`](crate::sb_misc::mean_position)); the caster's
///   object started at `G_O` `(1, 0x1d)`
///   ([`start_object_at`](crate::s4_misc::start_object_at), scale `K`);
///   `v` copied (`y - K'`); the caster's heading `K1 - a` (slot 333,
///   wrapped: [`wrap_0250`]) and its record started
///   ([`record_start_0250`](crate::r41::record_start_0250), `(v, 0.03,
///   0.99, 30)`, actor -1); four words set (the task callbacks).
/// - Slot 600 `(i, slot 599 (i) - 1)` for the 18 slots `i`.
/// - Per actor `i < *G_N`: an object started at `O_i` `(0, 0x25, -, CB)`;
///   slot 262 into `P_i - 4 .. P_i + 4` and slot 268 `(c, 6, &u)` (`P_i =
///   u.y`, `P_i + 8` 1.0); its heading (slot 333, wrapped) into `P_i -
///   0x10`; slot 531 `(t, &v)` and its record `R_i` started `(v, 1.2,
///   1.0, 16)`, `R_i` = `(V, i, t, model, -1, 0, 1, flag)`, `+0xa0` 0x25,
///   `+0x88` the callback.
/// - The task block's words, then three pool nodes taken
///   ([`pool_pop`](crate::r13_misc::pool_pop)) and appended to the task
///   list ([`append_kind`](crate::r13_start::append_kind), kinds 0, 1, 2;
///   the first with the owner `G_W`, the caster's record between: `(G_V,
///   t, model, -1, 0, 1, flag)`), the two pools reset in between
///   ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)).
///
/// The start object's third word is a register the original leaves there
/// (never read: 0). An empty pool makes the original fault (it writes at
/// `[0 + 0x14]`); the port skips that node. Returns 0.
#[allow(clippy::too_many_lines)] // the start-up in the original's order
pub fn start_up_0250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    for (dst, src) in [(0x26, 0x2a), (0x30, 0x34), (0x3a, 0x3e)] {
        g.write::<u32>(w(dst), w(src));
    }
    // call width: 3
    let h = g
        .call(slot_target(g, w(0x11), 719), 0, &[w(0x20), 8, 0])
        .eax;
    g.write::<u32>(w(0x48), h);
    crate::l5_objhost::part_init(g, img, call(0x4d));
    crate::sb_misc::mean_position(g, img, call(0x58), ebp.wrapping_sub(0x2c), v);
    crate::s4_misc::start_object_at(g, img, call(0x71), w(0x6c), k(g, 0x60), 1, 0x1d, 0);
    let c = g.read::<u32>(w(0x81));
    for (o, i) in [(0x89u32, 0u32), (0x96, 2), (0xa3, 3)] {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(w(o), x);
    }
    let y = sub(g.read::<f32>(v + 4), k(g, 0xb0));
    g.write::<f32>(w(0xb8), y);
    // call width: 1 (the start's words under it)
    let out = g.call(slot_target(g, w(0x7b), 333), 0, &[c]);
    let x = crate::m0018::st0_f32(out.st0);
    let a = wrap_0250(g, img, f, sub(k(g, 0xcb), x), [0xe2, 0xd8, 0xf2, 0xd8]);
    let k3 = g.read::<u32>(w(0x103));
    g.write::<f32>(w(0x12d), a);
    let record = |g: &mut G, ecx: u32, edx: u32, xmm3: u32, esp: u32, args: [u32; 4]| {
        for (i, x) in (1u32..).zip(args) {
            g.write::<u32>(esp + 4 * i, x);
        }
        let fr = Frame {
            at: call(0x132),
            esp,
            ecx,
            edx,
            xmm: [0, 0, 0, u128::from(xmm3)],
            ..Frame::default()
        };
        crate::r41::record_start_0250(g, img, &fr);
    };
    // push esi; push edi; then the four words
    record(
        g,
        w(0x125),
        u32::MAX,
        k3,
        ebp.wrapping_sub(0x44),
        [v, 0x3cf5_c28f, 0x3f7d_70a4, 0x1e],
    );
    g.write::<u32>(w(0x13b), 1);
    g.write::<u32>(w(0x145), 0xe);
    g.write::<u32>(w(0x14f), w(0x153));
    g.write::<u32>(w(0x159), w(0x15d));
    for i in 0..0x12u32 {
        // call width: 1
        let r = g.call(slot_target(g, w(0x164), 599), 0, &[i]).eax;
        // call width: 3 (slot 599's word under these)
        g.call(slot_target(g, w(0x174), 600), 0, &[i, r.wrapping_sub(1), i]);
    }
    let mut i = 0u32;
    let mut rec = w(0x19a);
    let mut obj = w(0x19f);
    let mut more = g.read::<i32>(w(0x18f)) > 0;
    while more {
        crate::s4_misc::start_object_at(
            g,
            img,
            call(0x1c9),
            obj.wrapping_sub(0xd4),
            k(g, 0x1b4),
            0,
            0x25,
            w(0x1b9),
        );
        let actor = |g: &G, list: u32, table: u32| {
            let l = g.read::<u32>(w(list));
            let b = u32::from(g.read::<u8>(l.wrapping_add(i)));
            g.read::<u32>(w(table).wrapping_add(4 * b))
        };
        let c = actor(g, 0x1de, 0x1e9);
        let pv = [c, obj.wrapping_sub(4), obj, obj.wrapping_add(4)];
        // call width: 4
        g.call(slot_target(g, w(0x1f7), 262), 0, &pv);
        let u = ebp.wrapping_sub(0x24);
        let mut words = vec![c, 6, u];
        words.extend_from_slice(&pv);
        // call width: 7 (slot 262's words under these)
        g.call(slot_target(g, w(0x20c), 268), 0, &words);
        let y = g.read::<u32>(u + 4);
        g.write::<u32>(obj, y);
        g.write::<u32>(obj.wrapping_add(8), 0x3f80_0000);
        let c = actor(g, 0x23d, 0x248);
        // call width: 1
        let out = g.call(slot_target(g, w(0x24e), 333), 0, &[c]);
        let x = crate::m0018::st0_f32(out.st0);
        let a = wrap_0250(g, img, f, sub(k(g, 0x261), x), [0x270, 0x27a, 0x284, 0x291]);
        g.write::<f32>(obj.wrapping_sub(0x10), a);
        obj = obj.wrapping_add(0x130);
        let byte = |g: &G, list: u32| {
            let l = g.read::<u32>(w(list));
            u32::from(g.read::<u8>(l.wrapping_add(i)))
        };
        let t = byte(g, 0x2b0);
        // call width: 2
        g.call(slot_target(g, w(0x2be), 531), 0, &[t, v]);
        let k4 = g.read::<u32>(w(0x2ce));
        record(
            g,
            rec.wrapping_sub(8),
            i,
            k4,
            ebp.wrapping_sub(0x48),
            [v, 0x3f99_999a, 0x3f80_0000, 0x10],
        );
        g.write::<u32>(rec.wrapping_sub(8), w(0x2fd));
        g.write::<u32>(rec.wrapping_sub(4), i);
        let t = byte(g, 0x311);
        g.write::<u32>(rec, t);
        let m = model_0250(g, img, f, t, (0x31b, 0x324));
        g.write::<u32>(rec + 4, m);
        g.write::<u32>(rec + 8, u32::MAX);
        g.write::<u32>(rec + 0xc, 0);
        g.write::<u32>(rec + 0x10, 1);
        let flag = u16::from(g.read::<u32>(w(0x33f)) != 0);
        g.write::<u16>(rec + 0x14, flag);
        g.write::<u32>(rec + 0xa0, 0x25);
        g.write::<u32>(rec + 0x88, w(0x37e));
        i += 1;
        rec = rec.wrapping_add(0xb4);
        more = i.cast_signed() < g.read::<i32>(w(0x38b));
    }
    start_tail_0250(g, img, f);
    Ret::eax(0)
}

/// The model of target `t`: slot 215 `(t)` when `*G_F` (operand `ops.0`)
/// is 0, else slot 413 `(t)` (the slot table at `ops.1`).
fn model_0250<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, ops: (u32, u32)) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = if g.read::<u32>(w(ops.0)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    g.call(slot_target(g, w(ops.1), slot), 0, &[t]).eax
}

/// [`start_up_0250`]'s end: the task block's words, the three pool nodes
/// and the caster's record between them, the two pools reset.
fn start_tail_0250<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |o: u32| img.word(f + o);
    let count0 = g.read::<u16>(w(0x398));
    for o in [0x3a0, 0x3aa, 0x3b4, 0x3be] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x3c8), w(0x3cc));
    g.write::<u32>(w(0x3d2), w(0x3d6));
    g.write::<u32>(w(0x3dc), 0);
    g.write::<u32>(w(0x3e6), 0);
    g.write::<u16>(w(0x3f0), 0);
    g.write::<u32>(w(0x3f6), 1);
    g.write::<u16>(w(0x400), 0);
    // (count, free head, free, free 0, callback, count out)
    let take = |g: &mut G, count: u16, ops: (u32, u32, u32, u32, u32, u32)| {
        if count == 0 {
            return None;
        }
        let node = g.read::<u32>(w(ops.1));
        crate::r13_misc::pool_pop(g, img, f, node, (ops.0, ops.2, ops.3, ops.4, ops.5));
        Some(node)
    };
    if let Some(node) = take(g, count0, (0x398, 0x415, 0x432, 0x43a, 0x466, 0x471)) {
        g.write::<u32>(node + 0x14, w(0x47f));
        crate::r13_start::append_kind(g, img, f, node, 0, [0x477, 0x48d, 0x4c0, 0x4a0, 0x4c0]);
    }
    let t = g.read::<u32>(w(0x4af));
    g.write::<u32>(w(0x4b4), t);
    g.write::<u32>(w(0x4c6), u32::MAX);
    g.write::<u32>(w(0x4d0), w(0x4d4));
    g.write::<u32>(w(0x4da), u32::MAX);
    let m = model_0250(g, img, f, t, (0x4a9, 0x4ba));
    g.write::<u32>(w(0x504), m);
    g.write::<u32>(w(0x50a), u32::MAX);
    g.write::<u32>(w(0x514), 0);
    g.write::<u32>(w(0x51e), 1);
    if g.read::<u32>(w(0x4f9)) == 0 {
        g.write::<u16>(w(0x528), 0);
    } else {
        g.write::<u16>(w(0x531), 1);
    }
    let count = g.read::<u16>(w(0x538));
    if let Some(node) = take(g, count, (0x538, 0x547, 0x564, 0x56c, 0x598, 0x5a3)) {
        crate::r13_start::append_kind(g, img, f, node, 1, [0x5a9, 0x5bc, 0x5dd, 0x5cf, 0x5dd]);
    }
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x5e2), w(0x5d4));
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x5ec), w(0x5e7));
    let count = g.read::<u16>(w(0x5f3));
    if let Some(node) = take(g, count, (0x5f3, 0x602, 0x61f, 0x627, 0x653, 0x65e)) {
        crate::r13_start::append_kind(g, img, f, node, 2, [0x664, 0x679, 0x69a, 0x68c, 0x69a]);
    }
}

/// [`start_up_0250`] (the `rel32`s: its callees and the cookie check).
pub const START_UP_0250: Sig = Sig {
    len: 1705,
    hash: 0x8c2c_ff17_94de_f41a,
    calls: &[
        (0x4d, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x58, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x71, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x132, Callee::Sig(&crate::r41::RECORD_START_0250)),
        (0x1c9, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x2f3, Callee::Sig(&crate::r41::RECORD_START_0250)),
        (
            0x5e2,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0180_2140),
        ),
        (
            0x5ec,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0250_1E00),
        ),
        (0x6a1, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0250`'s load step (0x10004130, the 118-byte shape) running
/// [`start_up_0250`].
pub fn load_0250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0250)
}

/// [`load_0250`].
pub const LOAD_0250: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0250))],
    imports: &[],
};

/// `magic_0250`'s caster record (0x100052c0, 124 bytes; a table entry,
/// no arguments; a /GS frame, the vector `v` at `ebp - 0x14`): slot 531
/// `(*G_C, &v)`, the record `G_R` started
/// ([`record_start_0250`](crate::r41::record_start_0250), actor -1, scale
/// `K`, `(v, 0.5, 0.75, 12)`), two words set (19, 1). Returns the
/// record start's `eax`.
pub fn caster_record_0250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let c = g.read::<u32>(w(0x1b));
    // call width: 2
    g.call(slot_target(g, w(0x15), 531), 0, &[c, v]);
    let k = g.read::<u32>(w(0x2b));
    // push 0xc; sub esp, 8; push v: the frame (0x14), the four words, the
    // return address
    let esp = ebp.wrapping_sub(0x14 + 0x14);
    for (i, x) in (1u32..).zip([v, 0x3f00_0000, 0x3f40_0000, 0xc]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let rf = Frame {
        at: img.rel32_target(f + 0x53),
        esp,
        ecx: w(0x4e),
        edx: u32::MAX,
        xmm: [0, 0, 0, u128::from(k)],
        ..Frame::default()
    };
    let r = crate::r41::record_start_0250(g, img, &rf);
    g.write::<u32>(w(0x61), 0x13);
    g.write::<u32>(w(0x6b), 1);
    r
}

/// [`caster_record_0250`] (the `rel32`s: the record start, the cookie
/// check).
pub const CASTER_RECORD_0250: Sig = Sig {
    len: 124,
    hash: 0x3772_7bd4_6f52_4489,
    calls: &[
        (0x53, Callee::Sig(&crate::r41::RECORD_START_0250)),
        (0x74, Callee::Any),
    ],
    imports: &[],
};

/// [`spawn_0250`]'s placement of one spark (the operands of each phase:
/// the two randoms' slot tables, the second's offset `K` (none in the
/// first phase), the first random's range, the matrix slots' tables, slot
/// 719's table, part operand and kind).
struct Place0250 {
    rnd: (u32, u32),
    adj: Option<u32>,
    c1: u32,
    tables: [u32; 5],
    part: (u32, u32, u32),
}

/// One spark placed: from the frame, the actor's place `P` (`ebp -
/// 0x44`), the turn `(0, yaw, 0)` (`ebp - 0x34`), the matrix `M` (`ebp -
/// 0x94`), the offset `r = (rnd(c1), 0, rnd(5) - K, 1)` (`ebp - 0x24`)
/// rotated into `ebp - 0x54`; `v = P + M r` (`ebp - 0x14`, `w` 1.0).
/// Returns slot 719's handle.
fn place_0250<G: Game>(g: &mut G, img: &Image, f: u32, ebp: u32, p: &Place0250) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let local = |o: u32| ebp.wrapping_sub(o);
    let rnd = |g: &mut G, table: u32, x: u32| {
        // call width: 1
        let out = g.call(slot_target(g, w(table), 314), 0, &[x]);
        crate::m0018::st0_f32(out.st0)
    };
    let r1 = rnd(g, p.rnd.0, p.c1);
    g.write::<f32>(local(0x24), r1);
    g.write::<u32>(local(0x20), 0);
    let r2 = rnd(g, p.rnd.1, 0x40a0_0000);
    let r2 = p.adj.map_or(r2, |o| sub(r2, k(g, o)));
    g.write::<f32>(local(0x1c), r2);
    for i in 0..4u32 {
        let x = g.read::<u32>(local(0x44) + 4 * i);
        g.write::<u32>(local(0x14) + 4 * i, x);
    }
    let (m, out, r, v) = (local(0x94), local(0x54), local(0x24), local(0x14));
    let [t235, t328, t329, t229, t248] = p.tables;
    // call width: 2 (the second random's word under it)
    g.call(slot_target(g, w(t235), 235), 0, &[m, 0x40a0_0000]);
    let xr = g.read::<u32>(local(0x34));
    // call width: 3
    g.call(slot_target(g, w(t328), 328), 0, &[m, m, xr]);
    let yr = g.read::<u32>(local(0x30));
    // call width: 3
    g.call(slot_target(g, w(t329), 329), 0, &[m, m, yr]);
    g.write::<u32>(local(0x18), 0x3f80_0000);
    // call width: 6 (slot 329's words stay under these)
    g.call(slot_target(g, w(t229), 229), 0, &[out, m, r, m, m, yr]);
    // call width: 9 (the earlier words stay)
    g.call(
        slot_target(g, w(t248), 248),
        0,
        &[v, v, out, out, m, r, m, m, yr],
    );
    g.write::<u32>(local(0x8), 0x3f80_0000);
    let (table, part, kind) = p.part;
    let words = [w(part), 9, kind, v, v, out, out, m, r, m, m, yr];
    // call width: 12 (the earlier words stay)
    g.call(slot_target(g, w(table), 719), 0, &words).eax
}

/// The next free record of a round-robin pool (`G_N` records, the next
/// index `G_I`, the base `*G_B`, `stride` apart, free when `flag` is 0;
/// the operands `(count, index, base)`): `Ok((record, next index))`, or
/// `Err(eax)` when none is free (the index as the search left it).
fn free_record<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ebp: u32,
    ops: (u32, u32, u32),
    (stride, flag): (u32, u32),
) -> Result<(u32, u32), u32> {
    let w = |o: u32| img.word(f + o);
    let n = g.read::<i32>(w(ops.0));
    let mut idx = g.read::<u32>(w(ops.1));
    if n <= 0 {
        return Err(idx);
    }
    let base = g.read::<u32>(w(ops.2));
    let mut tried = 0;
    loop {
        let rec = base.wrapping_add(idx.wrapping_mul(stride));
        idx = idx.wrapping_add(1);
        g.write::<u32>(ebp.wrapping_sub(0x98), 0);
        if idx.cast_signed() >= n {
            idx = 0;
        }
        if g.read::<u32>(rec.wrapping_add(flag)) == 0 {
            return Ok((rec, idx));
        }
        tried += 1;
        if tried >= n {
            return Err(idx);
        }
    }
}

/// `magic_0250`'s sparks (0x10004af0, 1986 bytes; a callback, cdecl `(R)`,
/// the actor's record; a /GS `ebp` frame). Unless `R + 0xa4` is above 18
/// (then the original returns its cookie word, `cookie ^ ebp`): slot 217
/// `(R[0xa0], &P)`, slot 601 `(R[0xa0], &a)`, the yaw `K - a` wrapped
/// ([`wrap_0250`]). Then three sparks from the DLL's ring
/// ([`ring_take`](crate::sa_misc::ring_take); none left ends it with 0),
/// four from the round-robin pool `A` (0x68 apart, free at `+0x60`) and
/// three from `B` (0x50, `+0x48`) ([`free_record`]; none free ends it
/// with the index): each placed ([`place_0250`]) and its record filled
/// (the handle, `(1, -1, 0x200)`, the place `(v.x, P.y - K, v.z, 1)`, the
/// pool's speeds and spins from slot 314 and slot 404). Returns the last
/// slot 404's `eax`.
#[allow(clippy::too_many_lines)] // three phases, as the original
pub fn spawn_0250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let ebp = fr.esp.wrapping_sub(4);
    let local = |o: u32| ebp.wrapping_sub(o);
    let s = fr.arg(g, 0);
    if g.read::<i32>(s.wrapping_add(0xa4)) > 0x12 {
        return Ret::eax(g.read::<u32>(w(0xa)) ^ ebp);
    }
    let h = g.read::<u32>(s.wrapping_add(0xa0));
    // call width: 2
    g.call(slot_target(g, w(0x29), 217), 0, &[h, local(0x44)]);
    let h2 = g.read::<u32>(s.wrapping_add(0xa0));
    g.write::<u32>(local(0x34), 0);
    g.write::<u32>(local(0x2c), 0);
    // call width: 4 (slot 217's words stay under these)
    g.call(
        slot_target(g, w(0x40), 601),
        0,
        &[h2, local(0x30), h, local(0x44)],
    );
    let a = sub(k(g, 0x64), g.read::<f32>(local(0x30)));
    let a = wrap_0250(g, img, f, a, [0x73, 0x7d, 0x87, 0x94]);
    g.write::<f32>(local(0x30), a);
    let rnd = |g: &mut G, table: u32, x: u32| {
        // call width: 1
        g.call(slot_target(g, w(table), 314), 0, &[x])
    };
    let spin = |g: &mut G, table: u32| {
        // call width: 1
        g.call(slot_target(g, w(table), 404), 0, &[0x3ecc_cccd])
    };
    let head = |g: &mut G, o: u32, h: u32, ky: u32| {
        g.write::<u32>(o, h);
        g.write::<u32>(o + 4, 1);
        g.write::<u16>(o + 8, 0xffff);
        g.write::<u16>(o + 0xa, 0x200);
        let x = g.read::<u32>(local(0x14));
        g.write::<u32>(o + 0xc, x);
        let y = sub(g.read::<f32>(local(0x40)), k(g, ky));
        g.write::<f32>(o + 0x10, y);
        let z = g.read::<u32>(local(0xc));
        g.write::<u32>(o + 0x14, z);
        g.write::<u32>(o + 0x18, 0x3f80_0000);
    };
    let st0 = |o: crate::Out| crate::m0018::st0_f32(o.st0);
    for _ in 0..3 {
        let o = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0xa5));
        if o == 0 {
            return Ret::eax(0);
        }
        let p = Place0250 {
            rnd: (0xb6, 0xcd),
            adj: None,
            c1: 0x41c8_0000,
            tables: [0x101, 0x128, 0x14a, 0x166, 0x183],
            part: (0x190, 0x1a6, 0),
        };
        let hd = place_0250(g, img, f, ebp, &p);
        head(g, o, hd, 0x1d5);
        g.write::<u32>(o + 0x1c, 0);
        g.write::<u32>(o + 0x20, 0xbe4c_cccd);
        g.write::<u32>(o + 0x24, 0);
        let x = st0(rnd(g, 0x208, 0x4049_0fdb));
        g.write::<f32>(o + 0x28, x);
        let t = st0(spin(g, 0x21f));
        g.write::<u32>(o + 0x34, 0xc);
        g.write::<u32>(o + 0x38, 8);
        let x = mul(add(t, k(g, 0x244)), k(g, 0x25e));
        g.write::<u32>(o + 0x3c, 0xffff_fffc);
        g.write::<u8>(o + 0x43, 0x40);
        g.write::<f32>(o + 0x2c, x);
        g.write::<u8>(o + 0x47, 0);
    }
    for _ in 0..4 {
        let (o, idx) = match free_record(g, img, f, ebp, (0x283, 0x288, 0x298), (0x68, 0x60)) {
            Ok(x) => x,
            Err(eax) => return Ret::eax(eax),
        };
        g.write::<u32>(w(0x2d8), idx);
        g.write::<u32>(o + 0x58, 0x8080_8080);
        g.write::<u32>(o + 0x5c, 0x8080_8080);
        g.write::<u32>(o + 0x38, 0x3f80_0000);
        g.write::<u32>(o + 0x54, 0);
        g.write::<u32>(o + 0x60, 1);
        let p = Place0250 {
            rnd: (0x2dd, 0x318),
            adj: Some(0x344),
            c1: 0x41a0_0000,
            tables: [0x350, 0x38b, 0x3ad, 0x3c9, 0x3e6],
            part: (0x3f3, 0x409, 1),
        };
        let hd = place_0250(g, img, f, ebp, &p);
        head(g, o, hd, 0x43b);
        g.write::<u32>(o + 0x1c, 0);
        let x = st0(rnd(g, 0x440, 0x4000_0000));
        g.write::<f32>(o + 0x20, x);
        g.write::<u32>(o + 0x24, 0xc0a0_0000);
        let x = st0(rnd(g, 0x477, 0x4000_0000));
        g.write::<f32>(o + 0x28, x);
        g.write::<u32>(o + 0x2c, 0);
        g.write::<u32>(o + 0x30, 0x3f80_0000);
        // the immediate: 1.0 in `magic_0250`, 2.0 in `magic_0251`
        g.write::<u32>(o + 0x34, w(0x4aa));
        let y = g.read::<u32>(local(0x40));
        g.write::<u32>(o + 0x3c, y);
        let x = st0(rnd(g, 0x495, 0x4049_0fdb));
        g.write::<f32>(o + 0x40, x);
        g.write::<u32>(o + 0x44, 0);
        let t = st0(spin(g, 0x4cb));
        g.write::<u32>(o + 0x4c, 6);
        g.write::<u32>(o + 0x50, 6);
        let x = mul(add(t, k(g, 0x4f7)), k(g, 0x511));
        g.write::<u32>(o + 0x54, 0);
        g.write::<u8>(o + 0x5f, 0);
        g.write::<f32>(o + 0x48, x);
    }
    let mut eax = 0;
    for _ in 0..3 {
        let (o, idx) = match free_record(g, img, f, ebp, (0x532, 0x537, 0x547), (0x50, 0x48)) {
            Ok(x) => x,
            Err(eax) => return Ret::eax(eax),
        };
        g.write::<u32>(w(0x58b), idx);
        g.write::<u32>(o + 0x40, 0x8080_8080);
        g.write::<u32>(o + 0x44, 0x8080_8080);
        g.write::<u32>(o + 0x3c, 0);
        g.write::<u16>(o + 0x4c, 0);
        g.write::<u32>(o + 0x48, 1);
        let p = Place0250 {
            rnd: (0x590, 0x5c8),
            adj: Some(0x5f4),
            c1: 0x41a0_0000,
            tables: [0x600, 0x63b, 0x65d, 0x679, 0x696],
            part: (0x6a3, 0x6b9, 2),
        };
        let hd = place_0250(g, img, f, ebp, &p);
        head(g, o, hd, 0x6f0);
        let x = st0(rnd(g, 0x6de, 0x4000_0000));
        g.write::<f32>(o + 0x1c, x);
        g.write::<u32>(o + 0x20, 0xc000_0000);
        let x = st0(rnd(g, 0x720, 0x4000_0000));
        g.write::<f32>(o + 0x24, x);
        let x = st0(rnd(g, 0x73e, 0x4049_0fdb));
        g.write::<f32>(o + 0x28, x);
        let out = spin(g, 0x755);
        eax = out.eax;
        let t = st0(out);
        g.write::<u32>(o + 0x34, 6);
        g.write::<u32>(o + 0x38, 6);
        let x = mul(add(t, k(g, 0x77a)), k(g, 0x794));
        g.write::<u32>(o + 0x3c, 0);
        g.write::<u8>(o + 0x47, 0);
        g.write::<f32>(o + 0x2c, x);
    }
    Ret::eax(eax)
}

/// [`spawn_0250`] (the `rel32`s: the ring take, the cookie checks).
pub const SPAWN_0250: Sig = Sig {
    len: 1986,
    hash: 0xe51d_307a_e098_dff6,
    calls: &[
        (0xa5, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x2cf, Callee::Any),
        (0x580, Callee::Any),
        (0x7ba, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0420`'s node stores: each `(dst, src)` pair of operands of `f`
/// (`*dst = src`).
fn stores_0420<G: Game>(g: &mut G, img: &Image, f: u32, pairs: &[(u32, u32)]) {
    for &(dst, src) in pairs {
        g.write::<u32>(img.word(f + dst), img.word(f + src));
    }
}

/// A node put last under its parent as `magic_0420` inlines it with the
/// old last `last` known: the old last stored into the node (operand
/// `prev`), the parent's first (`first`, when `last` is the sentinel at
/// operand `p`) or the old last's next (`next`: a word at `[last + 4]`,
/// or the operand pair given) set.
fn last_link_0420<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    last: u32,
    p: u32,
    first: (u32, u32),
    next: Result<u32, (u32, u32)>,
) {
    let w = |o: u32| img.word(f + o);
    if last == w(p) {
        g.write::<u32>(w(first.0), w(first.1));
    } else {
        match next {
            Ok(src) => g.write::<u32>(last.wrapping_add(4), w(src)),
            Err((dst, src)) => g.write::<u32>(w(dst), w(src)),
        }
    }
}

/// `magic_0420`'s set-up (0x100069d0, 1642 bytes; internal, no arguments,
/// an aligned frame with a record at its base `e`): the DLL's nodes put
/// last under the parent `P` one after another (their pass halfwords,
/// links and vtables; the parent's count kept in a register and stored
/// once), slot 237 `(*G_C, 11, 0, G_V)`, a node inserted and a particle
/// node, slot 306 `(7, 0, slot 719 (PART, 5, 0))`, the place copied, a
/// colour node, a node inserted; slot 325 `(G_A, G_B)`, the model of the
/// caster byte (slot 215), slot 12's height of it, the throw aimed
/// ([`throw_aim`](crate::s5_misc::throw_aim), `(G_N, G_V, G_A, h, 0.63,
/// 0, 3.0, G_O)`), slot 719 `(PART, 4, 0)` kept in a task node put at the
/// task ring's tail (its callback the DLL's task), two effect nodes (last
/// words 0 and 2). Returns the second effect node.
#[allow(clippy::too_many_lines)] // the set-up in the original's order
pub fn setup_0420<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let top = e.wrapping_sub(4);
    let rec = |g: &mut G, words: [u32; 6]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(e + 4 * i, v);
        }
    };
    let up16 = |g: &mut G, o: u32| {
        let n = g.read::<u16>(w(o)).wrapping_add(1);
        g.write::<u16>(w(o), n);
    };
    let one = 0x3f80_0000u32;
    // the first node
    let last = g.read::<u32>(w(0x7));
    g.write::<u16>(w(0x11), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[(0x19, 0x1d), (0x23, 0x27), (0x2d, 0x31), (0x37, 0x3b)],
    );
    g.write::<u32>(w(0x40), last);
    last_link_0420(g, img, f, last, 0x45, (0x4d, 0x51), Ok(0x5a));
    up16(g, 0x66);
    let c = g.read::<u32>(w(0x75));
    stores_0420(g, img, f, &[(0x7b, 0x7f), (0x85, 0x89)]);
    // call width: 4
    g.call(slot_target(g, w(0x5f), 237), 0, &[c, 0xb, 0, w(0x6b)]);
    // the second node
    let last = g.read::<u32>(w(0x96));
    g.write::<u32>(w(0x9f), one);
    g.write::<u16>(w(0xaa), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[(0xb2, 0xb6), (0xbc, 0xc0), (0xc6, 0xca), (0xd0, 0xd4)],
    );
    g.write::<u32>(w(0xd9), last);
    last_link_0420(g, img, f, last, 0xde, (0xe6, 0xea), Ok(0xf3));
    up16(g, 0xfa);
    stores_0420(g, img, f, &[(0x103, 0x107), (0x10d, 0x111)]);
    for o in [0x117, 0x121, 0x12b] {
        g.write::<u32>(w(o), one);
    }
    g.write::<u32>(w(0x135), 0x3f19_999a);
    rec(
        g,
        [w(0x140), w(0x148), w(0x150), w(0x158), w(0x160), w(0x168)],
    );
    let fi = Frame {
        at: call(0x16d),
        esp: top,
        ecx: e,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fi);
    let o = g.read::<u32>(w(0x172));
    rec(g, [w(0x17c), w(0x184), w(0x18c), o, w(0x198), 1]);
    g.write::<u32>(w(0x1a6), w(0x1aa));
    crate::sb_nodes::particle_node(g, img, call(0x1af), e);
    // call width: 3
    let h = g
        .call(slot_target(g, w(0x1b4), 719), 0, &[w(0x1c3), 5, 0])
        .eax;
    // call width: 3
    g.call(slot_target(g, w(0x1ce), 306), 0, &[7, 0, h]);
    // the third node, the place copied, two more nodes (the old last is
    // the node before: a constant)
    let last = g.read::<u32>(w(0x1df));
    g.write::<u16>(w(0x1e9), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x1f1, 0x1f5),
            (0x1fb, 0x1ff),
            (0x205, 0x209),
            (0x20f, 0x213),
        ],
    );
    g.write::<u32>(w(0x218), last);
    last_link_0420(g, img, f, last, 0x21d, (0x225, 0x229), Ok(0x232));
    for (dst, src) in [(0x248u32, 0x23au32), (0x269, 0x250)] {
        for i in 0..2 {
            let x = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(w(dst) + 4 * i, x);
        }
    }
    let count = g.read::<u16>(w(0x240));
    stores_0420(g, img, f, &[(0x25d, 0x261)]);
    g.write::<u16>(w(0x270), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x278, 0x27c),
            (0x282, 0x286),
            (0x28c, 0x290),
            (0x296, 0x29a),
            (0x2a0, 0x2a4),
        ],
    );
    last_link_0420(
        g,
        img,
        f,
        w(0x255),
        0x2aa,
        (0x2b2, 0x2b6),
        Err((0x2be, 0x2c2)),
    );
    stores_0420(g, img, f, &[(0x2cf, 0x2d3)]);
    for o in [0x2d9, 0x2e3, 0x2ed] {
        g.write::<u32>(w(o), one);
    }
    g.write::<u32>(w(0x2f7), 0x3ac4_9ba6);
    stores_0420(g, img, f, &[(0x301, 0x305)]);
    g.write::<u16>(w(0x30c), 0xff);
    stores_0420(g, img, f, &[(0x314, 0x318), (0x31e, 0x322), (0x328, 0x32c)]);
    last_link_0420(
        g,
        img,
        f,
        w(0x2c7),
        0x332,
        (0x33a, 0x33e),
        Err((0x346, 0x34a)),
    );
    up16(g, 0x351);
    stores_0420(g, img, f, &[(0x35c, 0x360)]);
    g.write::<u16>(w(0x366), count.wrapping_add(3));
    g.write::<u32>(w(0x36c), 0);
    g.write::<u32>(w(0x376), 0);
    g.write::<u16>(w(0x381), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x389, 0x38d),
            (0x393, 0x397),
            (0x39d, 0x3a1),
            (0x3a7, 0x3ab),
            (0x3b1, 0x3b5),
            (0x3bb, 0x3bf),
            (0x3c5, 0x3c9),
            (0x3cf, 0x3d3),
        ],
    );
    g.write::<u32>(w(0x3d9), 0);
    g.write::<u32>(w(0x3e3), 0x2000);
    g.write::<u32>(w(0x3ed), 0x1000);
    rec(
        g,
        [w(0x3f8), w(0x400), w(0x408), w(0x410), w(0x418), w(0x420)],
    );
    let fi = Frame {
        at: call(0x425),
        esp: top,
        ecx: e,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fi);
    let (a, b) = (w(0x43a), w(0x42f));
    // call width: 2
    g.call(slot_target(g, w(0x42a), 325), 0, &[a, b]);
    let byte = u32::from(g.read::<u8>(w(0x443)));
    // call width: 3 (slot 325's words under it)
    let m = g.call(slot_target(g, w(0x449), 215), 0, &[byte, a, b]).eax;
    // the throw's record at `e`: (G_N, G_V, G_A) first, slot 12's height
    g.write::<u32>(e, w(0x45b));
    g.write::<u32>(e + 4, w(0x469));
    g.write::<u32>(e + 8, w(0x474));
    // call width: 4 (the record's words under it)
    let out = g.call(
        slot_target(g, w(0x461), 12),
        0,
        &[m, w(0x45b), w(0x469), w(0x474)],
    );
    g.write::<f32>(e + 0xc, crate::m0018::st0_f32(out.st0));
    for (i, v) in (4u32..).zip([0x3f21_47ae, 0, 0x4040_0000, w(0x4a0)]) {
        g.write::<u32>(e + 4 * i, v);
    }
    let ta = Frame {
        at: call(0x4a5),
        esp: top,
        ecx: e,
        ..Frame::default()
    };
    crate::s5_misc::throw_aim(g, img, &ta);
    stores_0420(g, img, f, &[(0x4b2, 0x4b6)]);
    g.write::<u32>(w(0x4bc), 0x3f00_0000);
    g.write::<u32>(w(0x4c6), 0x3f00_0000);
    // call width: 3
    let h2 = g
        .call(slot_target(g, w(0x4aa), 719), 0, &[w(0x4d7), 4, 0])
        .eax;
    // the task node: last under the second parent, then at the ring's tail
    let last = g.read::<u32>(w(0x4df));
    g.write::<u16>(w(0x4e9), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x4f1, 0x4f5),
            (0x4fb, 0x4ff),
            (0x505, 0x509),
            (0x50f, 0x513),
        ],
    );
    g.write::<u32>(w(0x519), last);
    last_link_0420(g, img, f, last, 0x51f, (0x527, 0x52b), Ok(0x534));
    up16(g, 0x53b);
    g.write::<u32>(w(0x540), h2);
    let tail = g.read::<u32>(w(0x545));
    g.write::<u32>(w(0x54a), tail);
    stores_0420(g, img, f, &[(0x550, 0x554), (0x55a, 0x55e)]);
    g.write::<u32>(w(0x564), 0x8080_8080);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x56e, 0x572),
            (0x578, 0x57c),
            (0x582, 0x586),
            (0x58c, 0x590),
            (0x596, 0x59a),
        ],
    );
    g.write::<u32>(tail.wrapping_add(4), w(0x5a1));
    let fl = g.read::<u32>(w(0x5a7)) & !1;
    g.write::<u32>(w(0x5a7), fl);
    stores_0420(g, img, f, &[(0x5b3, 0x5b7)]);
    g.write::<u16>(w(0x5bd), 0);
    let mut node = 0;
    for (words, call_at, vt, flags, zero) in [
        (
            [0x5c4u32, 0x5cc, 0x5d4, 0x5dc, 0x5e4],
            0x5edu32,
            0x5f9u32,
            0x5ffu32,
            0x606u32,
        ),
        (
            [0x611, 0x619, 0x621, 0x629, 0x631],
            0x63e,
            0x647,
            0x64d,
            0x654,
        ),
    ] {
        let last_word = if call_at == 0x5ed { 0 } else { 2 };
        rec(
            g,
            [
                w(words[0]),
                w(words[1]),
                w(words[2]),
                w(words[3]),
                w(words[4]),
                last_word,
            ],
        );
        crate::sb_nodes::effect_node(g, img, call(call_at), e);
        node = g.read::<u32>(e);
        g.write::<u32>(node, w(vt));
        let v = g.read::<u32>(w(flags)) | 1;
        g.write::<u32>(w(flags), v);
        g.write::<u32>(w(zero), 0);
    }
    g.write::<u32>(w(0x65e), 0);
    Ret::eax(node)
}

/// [`setup_0420`] (the `rel32`s: the node inserts, the particle node,
/// the throw aim, the effect nodes).
pub const SETUP_0420: Sig = Sig {
    len: 1642,
    hash: 0x6137_4667_90c3_81d9,
    calls: &[
        (0x16d, Callee::Sig(&crate::nodes::INSERT)),
        (0x1af, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x425, Callee::Sig(&crate::nodes::INSERT)),
        (0x4a5, Callee::Sig(&crate::s5_misc::THROW_AIM)),
        (0x5ed, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x63e, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// `magic_0420`'s task (0x10007040, 344 bytes; a task callback, cdecl
/// `(S)`; no frame): by the u16 state `*G_ST` (a jump table, 5 cases):
/// - 0 and 1: slot 228 `(*G_C, 0xb8)`, the timer `*G_T` 0, state 2;
/// - 2: once slot 240 `(*G_C)` answers, slot 228 `(*G_C, 0xb9)`, the
///   set-up ([`setup_0420`]), state 3;
/// - 3: from the timer `K` on (`comiss`: unordered waits), slot 228
///   `(*G_C, 0xba)` and the state up;
/// - 4: once slot 240 answers, slot 213 `(*G_C, 0)` and the ring node
///   `S[0]` unlinked;
/// - above 4: nothing.
///
/// Every path ends with the timer `+= K'`. Returns `eax` as the path
/// leaves it (the waits and the states past the table: the state loaded
/// into `ax`, over the entry's high half).
pub fn task_0420<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let state = g.read::<u16>(w(0x5));
    let ax = fr.eax & 0xffff_0000 | u32::from(state);
    let c = |g: &G, o: u32| g.read::<u32>(w(o));
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<u16>(w(0x20), 1);
            }
            let a = c(g, 0x30);
            // call width: 2
            g.call(slot_target(g, w(0x25), 228), 0, &[a, 0xb8]);
            g.write::<u32>(w(0x43), 0);
            g.write::<u16>(w(0x4d), 2);
            2
        }
        2 => {
            let a = c(g, 0x5d);
            // call width: 1
            if g.call(slot_target(g, w(0x57), 240), 0, &[a]).eax == 0 {
                0
            } else {
                let a = c(g, 0x80);
                // call width: 2
                g.call(slot_target(g, w(0x75), 228), 0, &[a, 0xb9]);
                let s = Frame {
                    at: img.rel32_target(f + 0x90),
                    esp: fr.esp.wrapping_sub(8),
                    ..Frame::default()
                };
                setup_0420(g, img, &s);
                g.write::<u16>(w(0x9b), 3);
                3
            }
        }
        3 => {
            // comiss t, K; jb: below (or unordered) waits
            let t = g.read::<f32>(w(0xa8));
            let kk = k(g, 0xaf);
            if t.is_nan() || kk.is_nan() || t < kk {
                ax
            } else {
                let a = c(g, 0xc1);
                // call width: 2
                let r = g.call(slot_target(g, w(0xb6), 228), 0, &[a, 0xba]).eax;
                let n = g.read::<u16>(w(0xd3)).wrapping_add(1);
                g.write::<u16>(w(0xd3), n);
                r
            }
        }
        4 => {
            let a = c(g, 0xe0);
            // call width: 1
            if g.call(slot_target(g, w(0xda), 240), 0, &[a]).eax == 0 {
                0
            } else {
                let a = c(g, 0xfc);
                // call width: 2
                g.call(slot_target(g, w(0xf4), 213), 0, &[a, 0]);
                ring_unlink(g, fr.arg(g, 0))
            }
        }
        _ => ax,
    };
    let t = add(g.read::<f32>(w(0x12c)), k(g, 0x134));
    g.write::<f32>(w(0x13c), t);
    Ret::eax(eax)
}

/// [`task_0420`] (the `rel32`s: the set-up; its jumps to the shared ends
/// are inside).
pub const TASK_0420: Sig = Sig {
    len: 344,
    hash: 0x52a7_d37f_ce38_e809,
    calls: &[(0x90, Callee::Sig(&SETUP_0420))],
    imports: &[],
};

/// `magic_0420`'s object task (0x100066e0, 742 bytes; a task callback,
/// cdecl `(S)`; an aligned frame, `e` its base after the pushes, a record
/// at `e + 0x10`). With `T = S[1]`: the clock `T + 0x7e4 += K`; from `K'`
/// on (`comiss`: below or unordered does not) the i16 state `T + 0x538`
/// is 2. By the state:
/// - 0: nothing while u16 `T + 0x524` is 0, else as 1;
/// - 1: `T + 0x640` put last under `P`; the place copied into `T +
///   0x65c`; `T + 0x66c` put last under `P` with three globals (`+0x8000`
///   on the second); a node inserted `(T + 0x698, P, G_W, T + 0x640, T +
///   0x66c, G_N)`, a particle node `(T + 0x540, .., 0)`, slot 306 `(7, 1,
///   slot 719 (PART, 5, 0))`, `T + 0x4fc` and `+0x504` scaled by `K2`, an
///   effect node (last word 1), slot 239 `(*G_C, -5, 0)`; state 2;
/// - 2: while u16 `T + 0x520` is 0 nothing; else the byte `T + 0x2e3`
///   `+= 0xf0`, and when it wraps to 0 the node `T + 0x2bc` leaves
///   ([`unlink`](crate::m0042::unlink)) and the ring node `S[0]` is
///   unlinked.
///
/// Returns `eax` as the path leaves it (the state less 2 for the others).
#[allow(clippy::too_many_lines)] // the set-up of case 1, as the original
pub fn object_task_0420<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let call = |o: u32| img.rel32_target(f + o);
    let cursor = fr.arg(g, 0);
    let t = g.read::<u32>(cursor.wrapping_add(4));
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let top = e.wrapping_sub(4);
    let state = t.wrapping_add(0x538);
    let x = add(g.read::<f32>(t.wrapping_add(0x7e4)), k(g, 0x23));
    g.write::<f32>(t.wrapping_add(0x7e4), x);
    let kk = k(g, 0x2a);
    if !(x.is_nan() || kk.is_nan() || x < kk) {
        g.write::<u16>(state, 2);
    }
    let s = i32::from(g.read::<i16>(state));
    match s {
        0 | 1 => {
            if s == 0 && g.read::<u16>(t.wrapping_add(0x524)) == 0 {
                return Ret::eax(0);
            }
        }
        2 => {
            if g.read::<u16>(t.wrapping_add(0x520)) == 0 {
                return Ret::eax(0);
            }
            let b = g.read::<u8>(t.wrapping_add(0x2e3)).wrapping_add(0xf0);
            g.write::<u8>(t.wrapping_add(0x2e3), b);
            if b != 0 {
                return Ret::eax(0);
            }
            g.write::<u32>(e + 0xc, t.wrapping_add(0x2bc));
            crate::m0042::unlink(g, img, call(0x7e), e + 0xc);
            return Ret::eax(ring_unlink(g, cursor));
        }
        _ => return Ret::eax(s.wrapping_sub(2).cast_unsigned()),
    }
    let a = t.wrapping_add(0x640);
    child_last(
        g,
        img,
        f,
        a,
        [0xc6, 0xcd, 0xd2, 0xda, 0xdf, 0xe7, 0xf3, 0xf9],
    );
    g.write::<u32>(a, w(0xff));
    for (dst, src) in [(0x65cu32, 0x107u32), (0x664, 0x117)] {
        for i in 0..2 {
            let v = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(t.wrapping_add(dst + 4 * i), v);
        }
    }
    let b = t.wrapping_add(0x66c);
    g.write::<u32>(b, w(0x12b));
    child_last(
        g,
        img,
        f,
        b,
        [0x13e, 0x145, 0x14a, 0x152, 0x157, 0x15f, 0x170, 0x176],
    );
    let v = g.read::<u32>(w(0x169));
    g.write::<u32>(t.wrapping_add(0x688), v);
    let v = g.read::<u32>(w(0x181)).wrapping_add(0x8000);
    g.write::<u32>(t.wrapping_add(0x68c), v);
    let v = g.read::<u32>(w(0x191));
    g.write::<u32>(t.wrapping_add(0x690), v);
    let rec = e + 0x10;
    let n = t.wrapping_add(0x698);
    for (i, v) in (0u32..).zip([n, w(0x1b1), w(0x1b9), a, b, w(0x1c5)]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let fi = Frame {
        at: call(0x1ca),
        esp: top,
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fi);
    let o = g.read::<u32>(w(0x1cf));
    let pn = t.wrapping_add(0x540);
    g.write::<u32>(rec, pn);
    g.write::<u32>(pn, w(0x1df));
    for (i, v) in (1u32..).zip([w(0x1eb), n, o, w(0x1fb), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x208), rec);
    // call width: 3
    let h = g
        .call(slot_target(g, w(0x20d), 719), 0, &[w(0x21c), 5, 0])
        .eax;
    // call width: 3
    g.call(slot_target(g, w(0x227), 306), 0, &[7, 1, h]);
    let v = mul(g.read::<f32>(t.wrapping_add(0x4fc)), k(g, 0x243));
    g.write::<f32>(t.wrapping_add(0x4fc), v);
    let v = mul(g.read::<f32>(t.wrapping_add(0x504)), k(g, 0x262));
    for (i, x) in (0u32..).zip([w(0x26a), w(0x272)]) {
        g.write::<u32>(rec + 4 * i, x);
    }
    g.write::<f32>(t.wrapping_add(0x504), v);
    for (i, x) in (2u32..).zip([w(0x282), w(0x28a), w(0x292), 1]) {
        g.write::<u32>(rec + 4 * i, x);
    }
    crate::sb_nodes::effect_node(g, img, call(0x29f), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x2ab));
    let fl = g.read::<u32>(w(0x2b6)) | 1;
    g.write::<u32>(w(0x2b6), fl);
    let c = g.read::<u32>(w(0x2bf));
    g.write::<u32>(w(0x2c5), 0);
    // call width: 3
    let eax = g
        .call(slot_target(g, w(0x2b0), 239), 0, &[c, 0xffff_fffb, 0])
        .eax;
    g.write::<u16>(state, 2);
    Ret::eax(eax)
}

/// [`object_task_0420`] (the `rel32`s: the unlink, the node insert, the
/// particle and effect nodes).
pub const OBJECT_TASK_0420: Sig = Sig {
    len: 742,
    hash: 0x804b_7403_cc30_18e1,
    calls: &[
        (0x7e, Callee::Sig(&crate::m0042::UNLINK)),
        (0x1ca, Callee::Sig(&crate::nodes::INSERT)),
        (0x208, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x29f, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// The yaw of the vector `(dx, -, dz)` as `magic_0420`'s start inlines it
/// (as [`vector_angle`](crate::m0153::vector_angle), its constants the
/// operands of `f`): with the product `p = dx dz` above 0,
/// `atan(dz / dx)` plus a base when `dx` is not positive; below 0, a
/// quarter turn by the sign of `dz` less `atan(dx / dz)`; else by the
/// signs (`dx` ordered and equal to 0 goes on to `dz`).
fn yaw_0420<G: Game>(g: &mut G, img: &Image, f: u32, dx: f32, dz: f32) -> f32 {
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(img.word(f + o)));
    let p = mul(dx, dz);
    if p > 0.0 {
        let base = if dx > 0.0 { 0.0 } else { k(g, 0x5a3) };
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let t = g.libm(crate::Libm::Atan, f64::from(div(dz, dx))) as f32;
        return add(t, base);
    }
    if 0.0 > p {
        let base = if dz > 0.0 { k(g, 0x5d9) } else { k(g, 0x5e3) };
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let t = g.libm(crate::Libm::Atan, f64::from(div(dx, dz))) as f32;
        return sub(base, t);
    }
    // ucomiss + lahf + jnp: taken only when ordered and equal
    if dx != 0.0 {
        return if dx > 0.0 { 0.0 } else { k(g, 0x620) };
    }
    if dz != 0.0 {
        return if dz > 0.0 { k(g, 0x638) } else { k(g, 0x642) };
    }
    k(g, 0x64c)
}

/// `magic_0420`'s start (0x100071a0, 2124 bytes; a table entry, cdecl
/// `(actor)`; an aligned /GS frame, `E` its base after `push esi`, two
/// vectors at `E + 0xc` and `E + 0x1c`): the targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the DLL's task
/// ring, parent `P` and second parent reset; two model nodes (the caster
/// and the selected target, [`model_node_init`](crate::sa_summon::model_node_init));
/// the DLL's nodes put last under `P` (the count in a register, stored
/// after each group), slot 268 `(*G_C, 0x11, G_V)`, two node inserts
/// (the caster's handle's low byte and the target byte stored in the
/// colour nodes); the caster's and the target's places (slot 217), their
/// difference (slot 234) and its yaw ([`yaw_0420`]) given to slot 666;
/// a node put last; slots 718, 719, 711 (the lock held), 710, 222, 221,
/// 719, 551; the DLL's task (`magic_0420`'s 0x10007040) put at the ring's
/// tail; the state 0. Returns 0.
#[allow(clippy::too_many_lines)] // the start in the original's order
pub fn start_0420<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let top = e.wrapping_sub(4);
    let one = 0x3f80_0000u32;
    let actor = fr.arg(g, 0);
    crate::s4_summon::targets_init_b(g, img, call(0x19), top, actor);
    let c = g.read::<u32>(w(0x1e));
    stores_0420(
        g,
        img,
        f,
        &[
            (0x28, 0x2c),
            (0x32, 0x36),
            (0x3c, 0x40),
            (0x46, 0x4a),
            (0x50, 0x54),
        ],
    );
    g.write::<u32>(w(0x5a), 1);
    stores_0420(g, img, f, &[(0x64, 0x68)]);
    g.write::<u16>(w(0x6f), 0x1ff);
    stores_0420(g, img, f, &[(0x77, 0x7b), (0x81, 0x85)]);
    g.write::<u32>(w(0x8b), 0);
    stores_0420(g, img, f, &[(0x95, 0x99)]);
    g.write::<u16>(w(0xa0), 0xff);
    stores_0420(g, img, f, &[(0xa8, 0xac), (0xb2, 0xb6)]);
    g.write::<u32>(w(0xbc), 0);
    let rec = e + 0xc;
    let model = |g: &mut G, node: u32, parent: u32, who: u32, vt: (u32, u32), at: u32| {
        g.write::<u32>(rec, w(node));
        g.write::<u32>(rec + 4, w(parent));
        g.write::<u32>(rec + 8, who);
        g.write::<u8>(rec + 0xc, 0);
        g.write::<u32>(w(vt.0), w(vt.1));
        crate::sa_summon::model_node_init(g, img, call(at), rec);
    };
    model(g, 0xc8, 0xd0, c, (0xdf, 0xe3), 0xe8);
    let b = u32::from(g.read::<u8>(w(0xef)));
    model(g, 0xfb, 0x103, b, (0x112, 0x116), 0x11b);
    // three nodes put last (the old last after the first a constant)
    let last = g.read::<u32>(w(0x120));
    g.write::<u16>(w(0x127), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x12f, 0x133),
            (0x139, 0x13d),
            (0x143, 0x147),
            (0x14d, 0x151),
        ],
    );
    g.write::<u32>(w(0x156), last);
    last_link_0420(g, img, f, last, 0x15b, (0x163, 0x167), Ok(0x170));
    let count = g.read::<u16>(w(0x176));
    g.write::<u32>(w(0x183), one);
    g.write::<u8>(w(0x18d), 0);
    stores_0420(g, img, f, &[(0x194, 0x198)]);
    g.write::<u16>(w(0x19f), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x1a7, 0x1ab),
            (0x1b1, 0x1b5),
            (0x1bb, 0x1bf),
            (0x1c5, 0x1c9),
            (0x1cf, 0x1d3),
        ],
    );
    last_link_0420(
        g,
        img,
        f,
        w(0x17b),
        0x1d9,
        (0x1e1, 0x1e5),
        Err((0x1ed, 0x1f1)),
    );
    for o in [0x1fe, 0x208] {
        g.write::<u32>(w(o), 0);
    }
    stores_0420(g, img, f, &[(0x212, 0x216)]);
    for o in [0x21c, 0x226] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x231), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x239, 0x23d),
            (0x243, 0x247),
            (0x24d, 0x251),
            (0x257, 0x25b),
            (0x261, 0x265),
        ],
    );
    last_link_0420(
        g,
        img,
        f,
        w(0x1f6),
        0x26b,
        (0x273, 0x277),
        Err((0x27f, 0x283)),
    );
    g.write::<u16>(w(0x28b), count.wrapping_add(3));
    let c = g.read::<u32>(w(0x290)).to_le_bytes()[0];
    stores_0420(g, img, f, &[(0x29a, 0x29e)]);
    for o in [0x2a4, 0x2ae, 0x2b8, 0x2c2] {
        g.write::<u32>(w(o), one);
    }
    stores_0420(g, img, f, &[(0x2cc, 0x2d0)]);
    for o in [0x2d5, 0x2da, 0x2df] {
        g.write::<u8>(w(o), c);
    }
    let insert = |g: &mut G, ops: [u32; 6], at: u32| {
        let r = e + 0x1c;
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(r + 4 * i, w(o));
        }
        let fi = Frame {
            at: call(at),
            esp: top,
            ecx: r,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &fi);
    };
    insert(g, [0x2e7, 0x2ef, 0x2f7, 0x2ff, 0x307, 0x30f], 0x314);
    // a node put last, slot 268, three more
    let last = g.read::<u32>(w(0x319));
    g.write::<u16>(w(0x320), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x328, 0x32c),
            (0x332, 0x336),
            (0x33c, 0x340),
            (0x346, 0x34a),
        ],
    );
    g.write::<u32>(w(0x34f), last);
    last_link_0420(g, img, f, last, 0x354, (0x35c, 0x360), Ok(0x369));
    let n = g.read::<u16>(w(0x375)).wrapping_add(1);
    g.write::<u16>(w(0x375), n);
    let h = g.read::<u32>(w(0x382));
    stores_0420(g, img, f, &[(0x388, 0x38c), (0x392, 0x396)]);
    // call width: 3
    g.call(slot_target(g, w(0x36e), 268), 0, &[h, 0x11, w(0x37a)]);
    let last = g.read::<u32>(w(0x3a3));
    g.write::<u32>(w(0x3ac), one);
    g.write::<u16>(w(0x3b7), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x3bf, 0x3c3),
            (0x3c9, 0x3cd),
            (0x3d3, 0x3d7),
            (0x3dd, 0x3e1),
        ],
    );
    g.write::<u32>(w(0x3e6), last);
    last_link_0420(g, img, f, last, 0x3eb, (0x3f3, 0x3f7), Ok(0x400));
    let count = g.read::<u16>(w(0x406));
    for o in [0x413, 0x41d] {
        g.write::<u32>(w(o), 0);
    }
    stores_0420(g, img, f, &[(0x427, 0x42b)]);
    for o in [0x431, 0x43b] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x446), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x44e, 0x452),
            (0x458, 0x45c),
            (0x462, 0x466),
            (0x46c, 0x470),
            (0x476, 0x47a),
        ],
    );
    last_link_0420(
        g,
        img,
        f,
        w(0x40b),
        0x480,
        (0x488, 0x48c),
        Err((0x494, 0x498)),
    );
    g.write::<u16>(w(0x4a0), count.wrapping_add(2));
    let byte = g.read::<u8>(w(0x4a5));
    stores_0420(g, img, f, &[(0x4af, 0x4b3)]);
    for o in [0x4b9, 0x4c3, 0x4cd, 0x4d7] {
        g.write::<u32>(w(o), one);
    }
    stores_0420(g, img, f, &[(0x4e1, 0x4e5)]);
    g.write::<u8>(w(0x4ea), byte);
    g.write::<u8>(w(0x4ef), byte);
    insert(g, [0x4f7, 0x4ff, 0x507, 0x50f, 0x517, 0x51f], 0x524);
    // the caster's and the target's places, their difference, its yaw
    let (pa, pb) = (e + 0xc, e + 0x1c);
    let cc = g.read::<u32>(w(0x534));
    // call width: 2
    g.call(slot_target(g, w(0x52e), 217), 0, &[cc, pa]);
    let t = u32::from(g.read::<u8>(w(0x548)));
    // call width: 4 (the first call's words stay under these)
    g.call(slot_target(g, w(0x54e), 217), 0, &[t, pb, cc, pa]);
    // call width: 7 (the earlier words stay)
    g.call(
        slot_target(g, w(0x56a), 234),
        0,
        &[pa, pb, pa, t, pb, cc, pa],
    );
    let (dx, dz) = (g.read::<f32>(pa), g.read::<f32>(pa + 8));
    let yaw = yaw_0420(g, img, f, dx, dz);
    let cc = g.read::<u32>(w(0x663));
    // call width: 2
    g.call(slot_target(g, w(0x651), 666), 0, &[cc, yaw.to_bits()]);
    let last = g.read::<u32>(w(0x66a));
    g.write::<u16>(w(0x674), 0xff);
    stores_0420(
        g,
        img,
        f,
        &[
            (0x67c, 0x680),
            (0x686, 0x68a),
            (0x690, 0x694),
            (0x69a, 0x69e),
        ],
    );
    g.write::<u32>(w(0x6a3), last);
    last_link_0420(g, img, f, last, 0x6a8, (0x6b0, 0x6b4), Ok(0x6bd));
    let n = g.read::<u16>(w(0x6c9)).wrapping_add(1);
    g.write::<u16>(w(0x6c9), n);
    stores_0420(g, img, f, &[(0x6cf, 0x6d3), (0x6d9, 0x6dd)]);
    for o in [0x6e3, 0x6ed, 0x6f7] {
        g.write::<u32>(w(o), one);
    }
    g.write::<u32>(w(0x701), 0x4000_0000);
    // call width: 1
    g.call(slot_target(g, w(0x6c2), 718), 0, &[w(0x710)]);
    // call width: 3
    let h = g
        .call(slot_target(g, w(0x717), 719), 0, &[w(0x729), 8, 0])
        .eax;
    let lock = g.read::<u32>(w(0x731));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x73d), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(slot_target(g, w(0x748), 711), 0, &[h, w(0x742), 0]);
    let lock = g.read::<u32>(w(0x75b));
    g.write::<u32>(lock, saved);
    let h2 = g.read::<u32>(w(0x77b));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[0x10_0000, w(0x775), h2, w(0x780)]);
    // call width: 4
    s.call(g, slot_target(g, w(0x765), 710), 0);
    s.push_all(&[2, 0x3e8, 0, w(0x79b)]);
    // call width: 8 (slot 710's words stay under these)
    s.call(g, slot_target(g, w(0x787), 222), 0);
    s.push_all(&[2, 0x3e8, 0, w(0x7b6)]);
    // call width: 12 (the earlier words stay)
    s.call(g, slot_target(g, w(0x7a2), 221), 0);
    s.push_all(&[2, 6, w(0x7cc)]);
    // call width: 12 (Pushes caps the words at 12)
    let h3 = s.call(g, slot_target(g, w(0x7bd), 719), 0).eax;
    // call width: 3
    g.call(slot_target(g, w(0x7db), 551), 0, &[h3, 0, 2]);
    let tail = g.read::<u32>(w(0x7e8));
    g.write::<u32>(w(0x7f4), tail);
    stores_0420(g, img, f, &[(0x7fa, 0x7fe), (0x804, 0x808)]);
    g.write::<u32>(w(0x80e), 0);
    stores_0420(g, img, f, &[(0x818, 0x81c)]);
    g.write::<u32>(tail.wrapping_add(4), w(0x823));
    let fl = g.read::<u32>(w(0x829)) & !1;
    g.write::<u32>(w(0x829), fl);
    stores_0420(g, img, f, &[(0x833, 0x837)]);
    g.write::<u16>(w(0x83d), 0);
    Ret::eax(0)
}

/// [`start_0420`] (the `rel32`s: the targets, the model nodes, the node
/// inserts, the `atan` import and the cookie check).
pub const START_0420: Sig = Sig {
    len: 2124,
    hash: 0xea02_4cf4_defc_5646,
    calls: &[
        (0x19, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0xe8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x11b, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x314, Callee::Sig(&crate::nodes::INSERT)),
        (0x524, Callee::Sig(&crate::nodes::INSERT)),
        (0x5b5, Callee::Import("_libm_sse2_atan_precise")),
        (0x5f5, Callee::Import("_libm_sse2_atan_precise")),
        (0x844, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0394`'s landing check (0x10004b80, 470 bytes; a callback, cdecl
/// `(O)`; an aligned /GS frame, `E` its base after the pushes): with the
/// state block `S = *G_S`, the caster's place `P` (slot 262 into
/// `E + 0x10..`, `w` 1.0) moved by `(0, 0, -3)` turned by the yaw
/// `S + 0xc84` (slots 235, 328 `(M, M, 0)`, 329 `(M, M, yaw)`, 229, 248;
/// the matrix `M` at `E + 0x40`), then placed
/// ([`place_me`](crate::sa_misc::place_me)); when its distance in `x`/`z`
/// from `S + 0xb40` (`_libm_sse2_sqrt_precise` on the `f64`) is below
/// `K`, `S + 0xb50..` = `P`, the state `S + 0xc00` starts its task `(V)`
/// ([`state_task`](crate::s4_misc::state_task), its second word a
/// register never read: 0) with `S + 0xc88` = 0x15 and `S + 0xc48` = -1,
/// and `O + 0x56` = 1. Returns the state task's `eax`; otherwise the
/// original's is the `sqrt` import's leftover (junk; 0 here).
pub fn landing_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x88);
    let s = g.read::<u32>(w(0x1a));
    let o = fr.arg(g, 0);
    let (p, r, out, m) = (e + 0x10, e + 0x20, e + 0x30, e + 0x40);
    let c = g.read::<u32>(w(0x38));
    // call width: 4
    g.call(slot_target(g, w(0x32), 262), 0, &[c, p, p + 4, p + 8]);
    let yaw = g.read::<u32>(s.wrapping_add(0xc84));
    for (i, v) in (0u32..).zip([0x3f80_0000u32, 0, 0, 0xc040_0000, 0x3f80_0000]) {
        g.write::<u32>(p + 0xc + 4 * i, v);
    }
    g.write::<u32>(e + 0xc, yaw);
    // call width: 5 (slot 262's words under it)
    g.call(slot_target(g, w(0x52), 235), 0, &[m, c, p, p + 4, p + 8]);
    // call width: 3
    g.call(slot_target(g, w(0x9d), 328), 0, &[m, m, 0]);
    let yaw = g.read::<u32>(e + 0xc);
    // call width: 3
    g.call(slot_target(g, w(0xbe), 329), 0, &[m, m, yaw]);
    g.write::<u32>(r + 0xc, 0x3f80_0000);
    // call width: 6 (slot 329's words stay under these)
    g.call(slot_target(g, w(0xda), 229), 0, &[out, m, r, m, m, yaw]);
    // call width: 9 (the earlier words stay)
    g.call(
        slot_target(g, w(0xfa), 248),
        0,
        &[p, p, out, out, m, r, m, m, yaw],
    );
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    crate::sa_misc::place_me(g, img, img.rel32_target(f + 0x116), p);
    let dz = sub(g.read::<f32>(p + 8), g.read::<f32>(s.wrapping_add(0xb48)));
    let dx = sub(g.read::<f32>(p), g.read::<f32>(s.wrapping_add(0xb40)));
    let sum = add(mul(dz, dz), mul(dx, dx));
    // _libm_sse2_sqrt_precise of the double, rounded back to f32
    #[allow(clippy::cast_possible_truncation)]
    let d = f64::from(sum).sqrt() as f32;
    let mut eax = 0;
    // comiss K, d; jbe: only below K (ordered)
    if f32::from_bits(g.read::<u32>(w(0x155))) > d {
        for i in 0..4u32 {
            let v = g.read::<u32>(p + 4 * i);
            g.write::<u32>(s.wrapping_add(0xb50 + 4 * i), v);
        }
        g.write::<u32>(s.wrapping_add(0xc88), 0x15);
        // push ecx (a register never read: 0); push V
        let esp = e.wrapping_sub(0xc);
        g.write::<u32>(esp + 4, w(0x198));
        g.write::<u32>(esp + 8, 0);
        let st = Frame {
            at: img.rel32_target(f + 0x1ad),
            esp,
            ecx: s.wrapping_add(0xc00),
            ..Frame::default()
        };
        eax = crate::s4_misc::state_task(g, img, &st).eax;
        g.write::<u32>(s.wrapping_add(0xc48), u32::MAX);
        g.write::<u8>(o.wrapping_add(0x56), 1);
    }
    Ret::eax(eax)
}

/// [`landing_0394`] (the `rel32`s: the place, `sqrt`, the state task,
/// the cookie check).
pub const LANDING_0394: Sig = Sig {
    len: 470,
    hash: 0xa43c_d0e5_21c9_afcd,
    calls: &[
        (0x116, Callee::Sig(&crate::sa_misc::PLACE_ME)),
        (0x146, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1ad, Callee::Sig(&crate::s4_misc::STATE_TASK)),
        (0x1ce, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0394`'s turning step (0x100053e0, 599 bytes; a task callback,
/// cdecl `(N)`, a pool node; no frame). With `B = *G_S`: the two angles
/// `B + 0xaf0` and `+0xaf4` step by `K1`, `K2` and wrap (above `hi` less
/// `step`, below `lo` plus it); the speed `B + 0xb20..` adds into `B +
/// 0xb10..`, that into the place `B + 0xb00..` (the fourth words stored
/// back as they are); the matrix `M = B + 0xab0` scaled twice
/// ([`scaled`](crate::m0394::scaled)) and turned (slots 330, 328, 329 by
/// `B + 0xaf8`, `+0xaf0`, `+0xaf4`); `B + 0xae0..` = the place. The
/// counter `B + 0xc90` down: at 10 the selected target's transparency
/// (slot 541 `(t, 0.0, 10)` for a party member, `t < 0x20` and `*G_M`
/// 0; else slot 38 `(chr, 0.0, 10)` of the character table); at 0 or
/// below slot 238 `(t, 0)` and the node goes back to the pool (a tail
/// jump into [`give_back`](crate::sa_pool::give_back)). Returns that or
/// the last `eax`.
pub fn turn_step_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let b = g.read::<u32>(w(0x1e));
    let at = |o: u32| b.wrapping_add(o);
    let (hi, step, lo) = (k(g, 0x7), k(g, 0xf), k(g, 0x17));
    for (o, kd) in [(0xaf0u32, 0x2eu32), (0xaf4, 0x5a)] {
        let x = add(g.read::<f32>(at(o)), k(g, kd));
        // comiss x, hi; jbe: above only (ordered)
        let x = if x > hi {
            sub(x, step)
        } else if lo > x {
            add(x, step)
        } else {
            x
        };
        g.write::<f32>(at(o), x);
    }
    for (dst, src) in [(0xb10u32, 0xb20u32), (0xb14, 0xb24), (0xb18, 0xb28)] {
        let v = add(g.read::<f32>(at(src)), g.read::<f32>(at(dst)));
        g.write::<f32>(at(dst), v);
    }
    let v = g.read::<u32>(at(0xb1c));
    g.write::<u32>(at(0xb1c), v);
    let v = add(g.read::<f32>(at(0xb00)), g.read::<f32>(at(0xb10)));
    g.write::<f32>(at(0xb00), v);
    for (dst, src) in [(0xb04u32, 0xb14u32), (0xb08, 0xb18)] {
        let v = add(g.read::<f32>(at(src)), g.read::<f32>(at(dst)));
        g.write::<f32>(at(dst), v);
    }
    let v = g.read::<u32>(at(0xb0c));
    g.write::<u32>(at(0xb0c), v);
    let m = at(0xab0);
    crate::m0394::scaled(g, img, img.rel32_target(f + 0x12c), m);
    crate::m0394::scaled(g, img, img.rel32_target(f + 0x133), m);
    for (table, slot, o) in [
        (0x140u32, 330u32, 0xaf8u32),
        (0x15d, 328, 0xaf0),
        (0x17c, 329, 0xaf4),
    ] {
        let a = g.read::<u32>(at(o));
        // call width: 3
        g.call(slot_target(g, w(table), slot), 0, &[m, m, a]);
    }
    let mut eax = 0;
    for i in 0..4u32 {
        eax = g.read::<u32>(at(0xb00 + 4 * i));
        g.write::<u32>(at(0xae0 + 4 * i), eax);
    }
    let n = g.read::<u32>(at(0xc90)).wrapping_sub(1);
    g.write::<u32>(at(0xc90), n);
    if n == 0xa {
        let sel = g.read::<u32>(w(0x1dd));
        let t = u32::from(g.read::<u8>(sel));
        eax = if g.read::<u32>(w(0x1d7)) == 0 && t < 0x20 {
            // call width: 3
            g.call(slot_target(g, w(0x1ec), 541), 0, &[t, 0, 0xa]).eax
        } else {
            let chr = g.read::<u32>(w(0x213).wrapping_add(4 * t));
            // call width: 3
            g.call(slot_target(g, w(0x218), 38), 0, &[chr, 0, 0xa]).eax
        };
    }
    if g.read::<i32>(at(0xc90)) > 0 {
        return Ret::eax(eax);
    }
    let sel = g.read::<u32>(w(0x232));
    let t = u32::from(g.read::<u8>(sel));
    // call width: 2
    g.call(slot_target(g, w(0x23d), 238), 0, &[t, 0]);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x251),
        node,
    ))
}

/// [`turn_step_0394`] (the `rel32`s: the scaled matrix twice, the
/// give-back's tail jump).
pub const TURN_STEP_0394: Sig = Sig {
    len: 599,
    hash: 0x3b89_d75c_6f9c_1947,
    calls: &[
        (0x12c, Callee::Sig(&crate::m0394::SCALED)),
        (0x133, Callee::Sig(&crate::m0394::SCALED)),
        (0x251, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0394`'s sparks (0x10004d60, 865 bytes; internal, no arguments; a
/// /GS `ebp` frame, the place `P` at `ebp - 0x14`): the caster's place
/// (slot 262) with its height from slot 12 less `K`; two sparks from the
/// DLL's pool ([`pool_take`](crate::sb_misc::pool_take); none ends it with
/// 0) with the part handle `*(*G_H + 0x10)`, then one with a fresh slot
/// 719 `(PART, 9, 0)` handle: each `(1, -1, 0x200)`, its place `P` plus
/// two randoms (slot 314) in `x` and `z`, its speeds and spins from slots
/// 314 and 404 (one negated by the sign mask `*G_M`), its rates. Returns
/// the last slot 404's `eax` (0 when the pool runs out).
#[allow(clippy::too_many_lines)] // two spark kinds, as the original
pub fn sparks_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let c = g.read::<u32>(w(0x13));
    // call width: 4
    g.call(slot_target(g, w(0x25), 262), 0, &[c, p, p + 4, p + 8]);
    // call width: 5 (slot 262's words under it)
    let out = g.call(slot_target(g, w(0x33), 12), 0, &[c, c, p, p + 4, p + 8]);
    let y = sub(crate::m0018::st0_f32(out.st0), k(g, 0x49));
    let h = g.read::<u32>(g.read::<u32>(w(0x4e)).wrapping_add(0x10));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.write::<f32>(p + 4, y);
    let rnd = |g: &mut G, table: u32, x: u32| {
        // call width: 1
        let out = g.call(slot_target(g, w(table), 314), 0, &[x]);
        crate::m0018::st0_f32(out.st0)
    };
    let spin = |g: &mut G, table: u32, x: u32| {
        // call width: 1
        g.call(slot_target(g, w(table), 404), 0, &[x])
    };
    let neg = |g: &G, x: f32, o: u32| f32::from_bits(x.to_bits() ^ g.read::<u32>(w(o)));
    let place = |g: &mut G, node: u32, t1: u32, t2: u32| {
        let r = rnd(g, t1, 0x40a0_0000);
        let x = add(g.read::<f32>(p), r);
        g.write::<f32>(node, x);
        let y = g.read::<u32>(p + 4);
        g.write::<u32>(node + 4, y);
        let r = rnd(g, t2, 0x40a0_0000);
        let z = add(g.read::<f32>(p + 8), r);
        g.write::<u32>(node + 0xc, 0x3f80_0000);
        g.write::<f32>(node + 8, z);
    };
    let take = img.rel32_target(f + 0x71);
    for _ in 0..2 {
        let o = crate::sb_misc::pool_take(g, img, take);
        if o == 0 {
            return Ret::eax(0);
        }
        g.write::<u32>(o + 0x30, h);
        g.write::<u16>(o + 0x38, 0xffff);
        g.write::<u16>(o + 0x3a, 0x200);
        g.write::<u32>(o + 0x34, 1);
        place(g, o, 0x96, 0xbf);
        let x = rnd(g, 0xee, 0x3e80_0000);
        g.write::<f32>(o + 0x10, x);
        let t = crate::m0018::st0_f32(spin(g, 0x111, 0x3e80_0000).st0);
        g.write::<f32>(o + 0x14, neg(g, t, 0x12f));
        let x = rnd(g, 0x134, 0x3e80_0000);
        g.write::<f32>(o + 0x18, x);
        let x = rnd(g, 0x150, 0x4049_0fdb);
        g.write::<f32>(o + 0x1c, x);
        for (at, v) in [
            (0x20u32, 0u32),
            (0x24, 0x3c23_d70a),
            (0x28, 0),
            (0x2c, 0x3f80_0000),
        ] {
            g.write::<u32>(o + at, v);
        }
        let t = rnd(g, 0x167, 0x3e4c_cccd);
        let x = mul(add(t, k(g, 0x1a2)), k(g, 0x1bc));
        g.write::<u32>(o + 0x4c, 0x20);
        g.write::<u32>(o + 0x50, 0x10);
        g.write::<u32>(o + 0x54, 0xffff_fff0);
        g.write::<u8>(o + 0x6b, 0);
        g.write::<f32>(o + 0x3c, x);
    }
    // call width: 3
    let h2 = g
        .call(slot_target(g, w(0x1da), 719), 0, &[w(0x1e9), 9, 0])
        .eax;
    let o = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x1f5));
    if o == 0 {
        return Ret::eax(0);
    }
    g.write::<u16>(o + 0x38, 0xffff);
    g.write::<u16>(o + 0x3a, 0x200);
    g.write::<u32>(o + 0x30, h2);
    g.write::<u32>(o + 0x34, 1);
    place(g, o, 0x214, 0x240);
    let x = rnd(g, 0x26f, 0x3f80_0000);
    g.write::<f32>(o + 0x10, x);
    let t = crate::m0018::st0_f32(spin(g, 0x292, 0x3f00_0000).st0);
    let x = neg(g, add(t, k(g, 0x2b1)), 0x2c4);
    g.write::<f32>(o + 0x14, x);
    let x = rnd(g, 0x2b6, 0x3f80_0000);
    g.write::<f32>(o + 0x18, x);
    let x = rnd(g, 0x2d9, 0x4049_0fdb);
    g.write::<f32>(o + 0x1c, x);
    for (at, v) in [
        (0x20u32, 0u32),
        (0x24, 0x3dcc_cccd),
        (0x28, 0),
        (0x2c, 0x3f80_0000),
    ] {
        g.write::<u32>(o + at, v);
    }
    let out = spin(g, 0x2f0, 0x3e4c_cccd);
    let t = crate::m0018::st0_f32(out.st0);
    let x = mul(add(t, k(g, 0x32b)), k(g, 0x348));
    g.write::<u32>(o + 0x4c, 0x14);
    g.write::<u32>(o + 0x50, 0x14);
    g.write::<u8>(o + 0x6b, 0);
    g.write::<f32>(o + 0x3c, x);
    Ret::eax(out.eax)
}

/// [`sparks_0394`] (the `rel32`s: the pool takes, the cookie check).
pub const SPARKS_0394: Sig = Sig {
    len: 865,
    hash: 0x4faa_223b_6734_2ff7,
    calls: &[
        (0x71, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x1f5, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x359, Callee::Any),
    ],
    imports: &[],
};

/// One coordinate of [`flight_0394`]'s curve: with `ut = u t`, `uu = u
/// u`, `tt = t t` and `d` the coordinate's distance, `((((ut uu) k1 d +
/// (uu uu) d) + (tt uu) k3 (d kA)) + (tt ut) k4 (d kB)) + (tt tt) 0 +
/// base`, each step one `f32` rounding in this order.
#[allow(clippy::too_many_arguments)] // the curve's terms, as the original keeps them
fn curve_0394(ut: f32, uu: f32, tt: f32, d: f32, k: [f32; 3], ka: f32, kb: f32, base: f32) -> f32 {
    let [k1, k3, k4] = k;
    let a1 = mul(mul(mul(ut, uu), k1), d);
    let a2 = mul(mul(uu, uu), d);
    let s = add(a1, a2);
    let a3 = mul(mul(mul(tt, uu), k3), mul(d, ka));
    let s = add(s, a3);
    let a4 = mul(mul(mul(tt, ut), k4), mul(d, kb));
    let s = add(s, a4);
    let s = add(s, mul(mul(tt, tt), 0.0));
    add(s, base)
}

/// `magic_0394`'s flight down (0x100050d0, 777 bytes; a callback, cdecl
/// `(O)`; a /GS `ebp` frame, the place `v` at `ebp - 0x14`): with `S =
/// *G_S`, the counter `S + 0xc88` down while above 0, `t = n / K` and `u =
/// K' - t`; the place `v` on the curve from `S + 0xb50..` towards `S +
/// 0xb40..` ([`curve_0394`], each coordinate with its own constants);
/// then placed as [`place_me`](crate::sa_misc::place_me) does it inline
/// (slot 255 `(me, &v)` for a party member outside a battle, else slot
/// 33 `(obj, x, y, z)` or slot 32 `(obj, x, z)` when slot 11 says it is
/// grounded); below 15 the sparks ([`sparks_0394`]); at 0 `O + 0x56` = 1.
/// Returns the last call's `eax`.
pub fn flight_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let s = g.read::<u32>(w(0x17));
    let at = |o: u32| s.wrapping_add(o);
    let n = g.read::<i32>(at(0xc88));
    if n > 0 {
        g.write::<i32>(at(0xc88), n - 1);
    }
    let n = g.read::<i32>(at(0xc88));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let t = div(n as f32, k(g, 0x43));
    let u = sub(k(g, 0x3b), t);
    let (ut, uu, tt) = (mul(u, t), mul(u, u), mul(t, t));
    let p = |g: &G, o: u32| g.read::<f32>(at(o));
    let k7 = k(g, 0x5f);
    let x = curve_0394(
        ut,
        uu,
        tt,
        sub(p(g, 0xb40), p(g, 0xb50)),
        [k7, k7, k7],
        k(g, 0xb4),
        k(g, 0xbc),
        p(g, 0xb50),
    );
    let y = curve_0394(
        ut,
        uu,
        tt,
        sub(p(g, 0xb44), p(g, 0xb54)),
        [k(g, 0x135), k(g, 0x15c), k(g, 0x180)],
        k(g, 0x154),
        k(g, 0x164),
        p(g, 0xb54),
    );
    let k6 = k(g, 0x1c4);
    let z = curve_0394(
        ut,
        uu,
        tt,
        sub(p(g, 0xb48), p(g, 0xb58)),
        [k6, k6, k6],
        k(g, 0x20a),
        k(g, 0x212),
        p(g, 0xb58),
    );
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    for (i, c) in (0u32..).zip([x, y, z]) {
        g.write::<f32>(v + 4 * i, c);
    }
    let me = g.read::<i32>(w(0x24f));
    let mut eax = if g.read::<u32>(w(0x1a6)) == 0 && me < 0x20 {
        // call width: 2
        g.call(slot_target(g, w(0x25e), 255), 0, &[me.cast_unsigned(), v])
            .eax
    } else {
        let c = g.read::<u32>(w(0x277));
        // call width: 1
        let grounded = g.call(slot_target(g, w(0x270), 11), 0, &[c]).eax;
        let table = w(0x28c);
        if grounded == 0 {
            // call width: 4
            g.call(
                slot_target(g, table, 33),
                0,
                &[c, x.to_bits(), y.to_bits(), z.to_bits()],
            )
            .eax
        } else {
            // call width: 3
            g.call(slot_target(g, table, 32), 0, &[c, x.to_bits(), z.to_bits()])
                .eax
        }
    };
    if g.read::<i32>(at(0xc88)) < 0xf {
        let sp = Frame {
            at: img.rel32_target(f + 0x2e8),
            esp: ebp.wrapping_sub(0x2c),
            ..Frame::default()
        };
        eax = sparks_0394(g, img, &sp).eax;
    }
    if g.read::<i32>(at(0xc88)) <= 0 {
        let o = fr.arg(g, 0);
        g.write::<u8>(o.wrapping_add(0x56), 1);
    }
    Ret::eax(eax)
}

/// [`flight_0394`] (the `rel32`s: the sparks, the cookie check).
pub const FLIGHT_0394: Sig = Sig {
    len: 777,
    hash: 0xd3ea_9ee3_5a4d_6590,
    calls: &[(0x2e8, Callee::Sig(&SPARKS_0394)), (0x301, Callee::Any)],
    imports: &[],
};

/// `magic_0251`'s object step (0x100050e0, 239 bytes; a task callback,
/// cdecl `(S)`; no frame of its own). With `n = *G_N` records of 0xdc
/// bytes at `G_R`: whether one has its first word 0; the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); unless they report
/// 1 with flag 0x8000 of `*G_F` set and no record free, that is all
/// (their `eax`). Else each actor `i < n` gets its mode 0
/// ([`actor_mode`](crate::sa_script::actor_mode)) and is shown again
/// (slot 296 `(t, 0)` for a party member outside a battle, else slot 257
/// `(chr, 1)` of the character table); `S + 0x14` = 3 and `S[2] + 0x10` =
/// the next callback; the grid drawn ([`draw_grid`](crate::sb_misc::draw_grid))
/// and the part programs run (a tail jump into
/// [`part_programs`](crate::m0251::part_programs)).
pub fn object_step_0251<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let n = g.read::<i32>(w(0x7));
    let mut none_free = 1u32;
    let mut rec = w(0x17);
    for _ in 0..n.max(0) {
        if g.read::<u32>(rec) == 0 {
            none_free = 0;
            break;
        }
        rec = rec.wrapping_add(0xdc);
    }
    // push ebp; push esi; push edi; call
    let r = crate::l5_objhost::run_objects(g, img, call(0x34), fr.esp.wrapping_sub(0x10));
    if r != 1 || g.read::<u32>(w(0x43)) & 0x8000 == 0 || none_free != r {
        return Ret::eax(r);
    }
    for i in 0..n.max(0).cast_unsigned() {
        let am = Frame {
            at: call(0x65),
            esp: fr.esp.wrapping_sub(0x10),
            ecx: i,
            edx: 0,
            ..Frame::default()
        };
        crate::sa_script::actor_mode(g, img, &am);
        let list = g.read::<u32>(w(0x72));
        let party = if g.read::<u32>(w(0x6b)) == 0 {
            let t = u32::from(g.read::<u8>(list.wrapping_add(i)));
            (t < 0x20).then_some(t)
        } else {
            None
        };
        if let Some(t) = party {
            // call width: 2
            g.call(slot_target(g, w(0x8f), 296), 0, &[t, 0]);
        } else {
            let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
            let c = g.read::<u32>(w(0xae).wrapping_add(4 * b));
            // call width: 2
            g.call(slot_target(g, w(0xb6), 257), 0, &[c, 1]);
        }
    }
    let s = fr.arg(g, 0);
    g.write::<u32>(s.wrapping_add(0x14), 3);
    let s8 = g.read::<u32>(s.wrapping_add(8));
    g.write::<u32>(s8.wrapping_add(0x10), w(0xda));
    crate::sb_misc::draw_grid(g, img, call(0xdf));
    let pp = Frame {
        at: call(0xe7),
        esp: fr.esp,
        ..Frame::default()
    };
    crate::m0251::part_programs(g, img, &pp)
}

/// [`object_step_0251`] (the `rel32`s: the objects, the actor mode, the
/// grid and the part programs' tail jump).
pub const OBJECT_STEP_0251: Sig = Sig {
    len: 239,
    hash: 0xf420_ed28_9cd0_c68d,
    calls: &[
        (0x34, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x65, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0xdf, Callee::Sig(&crate::sb_misc::DRAW_GRID)),
        (0xe7, Callee::Sig(&crate::m0251::PART_PROGRAMS)),
    ],
    imports: &[],
};

/// `magic_0385`'s weapon frame (0x10004e50, 378 bytes; a callback, cdecl
/// `(S)`; a /GS `ebp` frame, the matrix `M` at `ebp - 0x44`, `W` at `ebp -
/// 0x84`): slot 236 `(*G_C, chr, &M)` with `chr` the character table's
/// entry `S[0x120]`, its three rows' first three terms scaled by `K /
/// h` (`h` slot 277's ST0 of `*G_C`), a unit matrix `W` (slot 235) turned
/// a quarter (slot 330 `(W, W, pi/2)`), slot 231 `(S + 0x80, &M, &W)`;
/// `S + 0xd0..` = `M`'s last row. Returns slot 231's `eax`.
pub fn weapon_frame_0385<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, wm) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let s = fr.arg(g, 0);
    let i = g.read::<u32>(s.wrapping_add(0x120));
    let chr = g.read::<u32>(w(0x24).wrapping_add(i.wrapping_mul(4)));
    let c = g.read::<u32>(w(0x2f));
    // call width: 3
    g.call(slot_target(g, w(0x29), 236), 0, &[c, chr, m]);
    let c2 = g.read::<u32>(w(0x42));
    // call width: 4 (slot 236's words under it)
    let out = g.call(slot_target(g, w(0x3c), 277), 0, &[c2, c, chr, m]);
    let h = crate::m0018::st0_f32(out.st0);
    let k = div(f32::from_bits(g.read::<u32>(w(0x58))), h);
    for row in 0..3u32 {
        for col in 0..3u32 {
            let at = m + 0x10 * row + 4 * col;
            let v = mul(g.read::<f32>(at), k);
            g.write::<f32>(at, v);
        }
    }
    // call width: 5 (the earlier words stay)
    g.call(slot_target(g, w(0x83), 235), 0, &[wm, c2, c, chr, m]);
    // call width: 3
    g.call(slot_target(g, w(0x109), 330), 0, &[wm, wm, 0x3fc9_0fdb]);
    // call width: 6 (slot 330's words stay under these)
    let eax = g
        .call(
            slot_target(g, w(0x128), 231),
            0,
            &[s.wrapping_add(0x80), m, wm, wm, wm, 0x3fc9_0fdb],
        )
        .eax;
    for i in 0..4u32 {
        let v = g.read::<u32>(m + 0x30 + 4 * i);
        g.write::<u32>(s.wrapping_add(0xd0 + 4 * i), v);
    }
    Ret::eax(eax)
}

/// [`weapon_frame_0385`] (the `rel32`: the cookie check).
pub const WEAPON_FRAME_0385: Sig = Sig {
    len: 378,
    hash: 0x8688_e867_4045_d902,
    calls: &[(0x172, Callee::Any)],
    imports: &[],
};

/// One coordinate of a cubic Bezier as `magic_0251`'s curve task computes
/// it: `((((u t) u) K) p1 + ((u u) u) p0) + ((t t) u) K p2) + (t t) t p3`,
/// each step one `f32` rounding in this order.
#[allow(clippy::many_single_char_names)] // the curve's own terms
fn bezier_0251(u: f32, t: f32, k: f32, p: [f32; 4]) -> f32 {
    let tt = mul(t, t);
    let a = mul(mul(mul(mul(u, t), u), k), p[1]);
    let b = mul(mul(mul(u, u), u), p[0]);
    let c = mul(mul(mul(tt, u), k), p[2]);
    let d = mul(mul(tt, t), p[3]);
    add(add(add(a, b), c), d)
}

/// `magic_0251`'s curve task (0x100051d0, 701 bytes; a task callback,
/// cdecl `(S)`; a /GS `ebp` frame, the point `v` at `ebp - 0x14`). With
/// `T = S[0x14]`: while the delay `T + 0xd0` is above 0 it counts down
/// (that is all; its new value returned). Else the steps `T + 0xcc`
/// count down; while some are left the parameter `t = T + 0xc0` grows by
/// `T + 0xc4`, else it is 1 (`K`); `v` = the cubic Bezier of the control
/// points `T + 0x80`, `+0x90`, `+0xa0`, `+0xb0` ([`bezier_0251`]) moved by
/// `T + 0x70` (slot 248); the angle `T + 0x60 += T + 0xc8` wrapped; the
/// matrix `T + 0x20` of the actor `T[0xd4]`
/// ([`actor_scale`](crate::sa_misc::actor_scale)) turned (slots 330, 328,
/// 329 by `T + 0x68`, `+0x60`, `+0x64`); `T + 0x50..` = `v` (`w` 1.0);
/// the motion script ([`motion_script`](crate::s4_misc::motion_script));
/// when it reports 1 with no steps left, `T + 0xd8` = 1 and the node `S`
/// goes back to the pool ([`give_back`](crate::sa_pool::give_back)).
/// Returns the last call's `eax`.
pub fn curve_task_0251<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(w(o)));
    let call = |o: u32| img.rel32_target(f + o);
    let s = fr.arg(g, 0);
    let t = g.read::<u32>(s.wrapping_add(0x14));
    let at = |o: u32| t.wrapping_add(o);
    let delay = g.read::<i32>(at(0xd0));
    if delay > 0 {
        g.write::<i32>(at(0xd0), delay - 1);
        return Ret::eax((delay - 1).cast_unsigned());
    }
    let n = g.read::<i32>(at(0xcc));
    if n > 0 {
        g.write::<i32>(at(0xcc), n - 1);
    }
    let one = k(g, 0x55);
    let p = if g.read::<i32>(at(0xcc)) > 0 {
        let p = add(g.read::<f32>(at(0xc4)), g.read::<f32>(at(0xc0)));
        g.write::<f32>(at(0xc0), p);
        p
    } else {
        one
    };
    let u = sub(one, p);
    let k2 = k(g, 0x7c);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    for j in 0..3u32 {
        let c = |i: u32| g.read::<f32>(at(0x80 + 0x10 * i + 4 * j));
        let x = bezier_0251(u, p, k2, [c(0), c(1), c(2), c(3)]);
        g.write::<f32>(v + 4 * j, x);
    }
    // call width: 3
    g.call(slot_target(g, w(0xd1), 248), 0, &[v, at(0x70), v]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let r = add(g.read::<f32>(at(0xc8)), g.read::<f32>(at(0x60)));
    // comiss r, hi; jbe: above only (ordered)
    let r = if r > k(g, 0x1d2) {
        sub(r, k(g, 0x1dc))
    } else if k(g, 0x1e6) > r {
        add(r, k(g, 0x1f3))
    } else {
        r
    };
    g.write::<f32>(at(0x60), r);
    let m = at(0x20);
    let actor = g.read::<u32>(at(0xd4));
    crate::sa_misc::actor_scale(g, img, call(0x209), actor, m);
    for (table, slot, o) in [
        (0x213u32, 330u32, 0x68u32),
        (0x22d, 328, 0x60),
        (0x249, 329, 0x64),
    ] {
        let a = g.read::<u32>(at(o));
        // call width: 3
        g.call(slot_target(g, w(table), slot), 0, &[m, m, a]);
    }
    for i in 0..4u32 {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(at(0x50 + 4 * i), x);
    }
    let eax = crate::s4_misc::motion_script(g, img, call(0x28d), t);
    if eax != 1 || g.read::<i32>(at(0xcc)) > 0 {
        return Ret::eax(eax);
    }
    g.write::<u32>(at(0xd8), eax);
    Ret::eax(crate::sa_pool::give_back(g, img, call(0x2a9), s))
}

/// [`curve_task_0251`] (the `rel32`s: the actor's matrix, the motion
/// script, the give-back, the cookie checks).
pub const CURVE_TASK_0251: Sig = Sig {
    len: 701,
    hash: 0x8e38_75a9_7746_3ee3,
    calls: &[
        (0x31, Callee::Any),
        (0x209, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x28d, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x2a9, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x2b5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0348`'s end task (0x10006580, 58 bytes; a task callback, cdecl
/// `(N)`, a pool node): as [`task_0350`](crate::r38::task_0350) without
/// its block and words: the script ([`run`](crate::sa_script::run)) and
/// the motion step ([`motion_step`](crate::s3_motion::motion_step)); when
/// both report 1, `*G_S + 0xd180` = 1, the lights and models
/// ([`lights_models`](crate::m0348::lights_models)) and the node goes back
/// to the pool (a tail jump into [`give_back`](crate::sa_pool::give_back)).
/// Returns the give-back's `eax`, or the motion step's.
pub fn end_task_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = g.read::<u32>(img.word(f + 0x07));
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0xc));
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x13));
    if script != 1 || motion != 1 {
        return Ret::eax(motion);
    }
    g.write::<u32>(s.wrapping_add(0xd180), 1);
    // push ebp, esi, edi; the call
    let lm = Frame {
        at: img.rel32_target(f + 0x27),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    crate::m0348::lights_models(g, img, &lm);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x32),
        node,
    ))
}

/// [`end_task_0348`] (the `rel32`s: the script, the motion step, the
/// lights and models, the give-back's tail jump).
pub const END_TASK_0348: Sig = Sig {
    len: 58,
    hash: 0x16e8_2eb7_08a4_6dc7,
    calls: &[
        (0xc, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x27, Callee::Sig(&crate::m0348::LIGHTS_MODELS)),
        (0x32, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
