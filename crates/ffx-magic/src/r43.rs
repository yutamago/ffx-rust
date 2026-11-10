//! Round 43's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
