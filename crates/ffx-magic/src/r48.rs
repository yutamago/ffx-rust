//! Round 48's ports (own rows of `magic_0387`, `0396`, `0397`, `0431`):
//! the DLLs' own code written out, in tested pieces.
//!
//! `magic_0387`'s "Ego" nodes: an object's first word is its class
//! record (a 16-byte name, `+0x10` a version, `+0x14` the parent class,
//! `+0x18` the factory, `+0x1c` the visit method). A visit takes a
//! visitor `V` (`+0` the node it is at, byte `+4` a flag, byte `+5` the
//! pass's mode) and first walks the node's children (see [`walk`]).

// `comiss` with `jb`/`jbe`: an unordered compare takes one side
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// A node's visit method (`call eax` with `eax = [[node] + 0x1c]`, cdecl
/// with the one word `arg`; `ecx` as the caller leaves it): overlay code
/// with a port runs here on the original's stack (`esp` at the return
/// address, the word above it `arg`), as the original runs it; anything
/// else goes through the host. Returns `eax`.
pub(crate) fn method<G: Game>(
    g: &mut G,
    img: &Image,
    target: u32,
    esp: u32,
    ecx: u32,
    arg: u32,
) -> u32 {
    if let Some(f) = crate::bind(img, target).and_then(crate::entry::<G>) {
        g.write::<u32>(esp + 4, arg);
        let r = f(
            g,
            img,
            &Frame {
                at: target,
                esp,
                eax: target,
                ecx,
                ..Frame::default()
            },
        );
        return match r.jump {
            Some(t) => g.call(t, ecx, &[arg]).eax,
            None => r.eax,
        };
    }
    g.call(target, ecx, &[arg]).eax
}

/// The visitor's walk every Ego visit starts with (`ecx`-style: `v` the
/// visitor, `esp` the methods' stack pointer at their return address,
/// `ecx` what the original passes them): the node `O = V[0]`'s byte
/// `+0x1a` against the mode `V[5]`. Equal clears the flag `V[4]` and
/// stops (`None`). Otherwise each child, from `O + 0x10` along `+8` back
/// to `O`, becomes `V[0]` and its visit method runs with `V`; then `V[0]
/// = O`, `O[0x1a]` = the mode and the flag is set (`Some(O)`). `eax` as
/// the original leaves it: the low byte the last byte loaded.
pub(crate) fn walk<G: Game>(
    g: &mut G,
    img: &Image,
    v: u32,
    esp: u32,
    ecx: u32,
    eax: u32,
) -> (u32, Option<u32>) {
    let o = g.read::<u32>(v);
    let mut eax = (eax & !0xff) | u32::from(g.read::<u8>(o + 0x1a));
    if g.read::<u8>(v + 5) == g.read::<u8>(o + 0x1a) {
        g.write::<u8>(v + 4, 0);
        return (eax, None);
    }
    let mut cur = g.read::<u32>(o + 0x10);
    while cur != o {
        g.write::<u32>(v, cur);
        let next = g.read::<u32>(cur + 8);
        let m = g.read::<u32>(g.read::<u32>(cur) + 0x1c);
        eax = method(g, img, m, esp, ecx, v);
        cur = next;
    }
    let mode = g.read::<u8>(v + 5);
    g.write::<u32>(v, o);
    g.write::<u8>(o + 0x1a, mode);
    g.write::<u8>(v + 4, 1);
    ((eax & !0xff) | u32::from(mode), Some(o))
}

/// A child's visit through a local visitor `L` (`L[0]` = `child`, byte
/// `L[5]` = `mode`; `ecx` = `L`): the child's method with `L`.
fn visit_child<G: Game>(g: &mut G, img: &Image, l: u32, esp: u32, child: u32, mode: u8) -> u32 {
    g.write::<u32>(l, child);
    g.write::<u8>(l + 5, mode);
    let m = g.read::<u32>(g.read::<u32>(child) + 0x1c);
    method(g, img, m, esp, l, l)
}

/// `magic_0387`'s line place (0x10003680, 211 bytes; internal, `ecx` =
/// the visitor `V`; an aligned frame, the local visitor `L` at `A -
/// 0x10`, `A` the aligned `ebp`): after the [`walk`] (none: its `eax`),
/// the node `O`'s child `C = O[0x2c]` is visited with `L`, then `L[0] =
/// C` gets its strip window
/// ([`strip_window`](crate::s4_small::strip_window)): with no head part
/// (`u16 L[4]` 0) `L = (C, O + 0x1c)` goes to the bone place
/// ([`bone_place_0387`](crate::r45::bone_place_0387); its `eax`), else
/// slot 325 `(O + 0x1c, L[0xc] + 16 (u16 L[6] - 1))` and `O[0x28]` =
/// 1.0 (the slot's `eax`).
pub fn line_place_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let v = fr.ecx;
    let a = fr.esp.wrapping_sub(4) & !7;
    let l = a.wrapping_sub(0x10);
    let (eax, o) = walk(g, img, v, a.wrapping_sub(0x28), v, fr.eax);
    let Some(o) = o else {
        return Ret::eax(eax);
    };
    let mode = g.read::<u8>(v + 5);
    let c = g.read::<u32>(o + 0x2c);
    visit_child(g, img, l, a.wrapping_sub(0x28), c, mode);
    let c = g.read::<u32>(o + 0x2c);
    g.write::<u32>(l, c);
    let at = img.rel32_target(f + 0x78);
    crate::s4_small::strip_window(
        g,
        img,
        &Frame {
            at,
            esp: a.wrapping_sub(0x24),
            ecx: l,
            ..Frame::default()
        },
    );
    if g.read::<u16>(l + 4) == 0 {
        g.write::<u32>(l, c);
        g.write::<u32>(l + 4, o + 0x1c);
        let at = img.rel32_target(f + 0x94);
        return crate::r45::bone_place_0387(
            g,
            img,
            &Frame {
                at,
                esp: a.wrapping_sub(0x24),
                ecx: l,
                ..Frame::default()
            },
        );
    }
    let rest = u32::from(g.read::<u16>(l + 6));
    let p = g
        .read::<u32>(l + 0xc)
        .wrapping_sub(0x10)
        .wrapping_add(rest << 4);
    let eax = g
        .call(slot_target(g, img.word(f + 0xb6), 325), 0, &[o + 0x1c, p])
        .eax;
    g.write::<u32>(o + 0x28, ONE);
    Ret::eax(eax)
}

/// [`line_place_0387`] (the `rel32`s: the strip window, the bone place).
pub const LINE_PLACE_0387: Sig = Sig {
    len: 211,
    hash: 0xf62b_2837_82cb_cc43,
    calls: &[
        (0x78, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x94, Callee::Sig(&crate::r45::BONE_PLACE_0387)),
    ],
    imports: &[],
};

/// `magic_0387`'s matrix move (0x10004ca0, 488 bytes; internal, `ecx` =
/// the visitor `V`; a /GS frame, the local visitor `L` at `ebp - 0x1c`,
/// the vector `P` at `ebp - 0x14`): after the [`walk`] (none: its
/// `eax`), the node `O`'s children `A = O[0x13c]` and `B = O[0x150]` are
/// visited with `L`; `P` = `B + 0x1c` (slot 232) scaled by -1 (slot 233),
/// then `P = P k + O[0x3c..] (K - k)` with `k = O[0x14c]` stored to `O +
/// 0x3c` (slot 232); with `U = (0, 1, 0, 1)`: slot 266 `(O + 0x1c, U, O +
/// 0x3c)` and `(O + 0x2c, O + 0x3c, O + 0x1c)`, slot 230 `(O + 0x9c, O +
/// 0x1c)` and slot 325 `(O + 0xcc, A' + 0x1c)` (`A'` = `O[0x13c]` again;
/// its `eax`).
pub fn matrix_move_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let v = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let (l, p) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x14));
    // eax: the cookie word (`ds:__security_cookie ^ ebp`)
    let cookie = g.read::<u32>(w(7)) ^ ebp;
    let (eax, o) = walk(g, img, v, ebp.wrapping_sub(0x30), v, cookie);
    let Some(o) = o else {
        return Ret::eax(eax);
    };
    let mode = g.read::<u8>(v + 5);
    let a = g.read::<u32>(o + 0x13c);
    visit_child(g, img, l, ebp.wrapping_sub(0x30), a, mode);
    let b = g.read::<u32>(o + 0x150);
    let mode = g.read::<u8>(v + 5);
    visit_child(g, img, l, ebp.wrapping_sub(0x34), b, mode);
    let b = g.read::<u32>(o + 0x150);
    g.call(slot_target(g, w(0xa1), 232), 0, &[p, b + 0x1c]);
    g.call(slot_target(g, w(0xbd), 233), 0, &[p, p, 0xbf80_0000]);
    let k = g.read::<f32>(o + 0x14c);
    let c = sub(g.read::<f32>(w(0xd5)), k);
    let r = |g: &G, i: u32| g.read::<f32>(p + 4 * i);
    let q = |g: &G, i: u32| g.read::<f32>(o + 0x3c + 4 * i);
    let x0 = add(mul(r(g, 0), k), mul(c, q(g, 0)));
    let v1 = mul(r(g, 1), k);
    g.write::<f32>(p, x0);
    let x1 = add(mul(q(g, 1), c), v1);
    let v2 = mul(r(g, 2), k);
    g.write::<f32>(p + 4, x1);
    let x2 = add(mul(q(g, 2), c), v2);
    let v3 = mul(r(g, 3), k);
    g.write::<f32>(p + 8, x2);
    let x3 = add(mul(q(g, 3), c), v3);
    g.write::<f32>(p + 0xc, x3);
    g.call(slot_target(g, w(0xf5), 232), 0, &[o + 0x3c, p]);
    for (i, x) in [0u32, ONE, 0, ONE].into_iter().enumerate() {
        g.write::<u32>(p + 4 * i as u32, x);
    }
    let s266 = slot_target(g, w(0x162), 266);
    g.call(s266, 0, &[o + 0x1c, p, o + 0x3c]);
    let s266 = slot_target(g, w(0x195), 266);
    g.call(s266, 0, &[o + 0x2c, o + 0x3c, o + 0x1c]);
    g.call(slot_target(g, w(0x1aa), 230), 0, &[o + 0x9c, o + 0x1c]);
    let a = g.read::<u32>(o + 0x13c);
    let eax = g
        .call(slot_target(g, w(0x1c8), 325), 0, &[o + 0xcc, a + 0x1c])
        .eax;
    Ret::eax(eax)
}

/// [`matrix_move_0387`] (the `rel32`: the cookie check).
pub const MATRIX_MOVE_0387: Sig = Sig {
    len: 488,
    hash: 0xb408_a9c8_543d_1d25,
    calls: &[(0x1e0, Callee::Any)],
    imports: &[],
};

/// The visit thunk over an internal visit with the visitor in `ecx`
/// (`push ebp; mov ebp, esp; mov ecx, [ebp + 8]; pop ebp; jmp target`):
/// the target runs on the thunk's own stack.
fn thunk_to<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    port: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let at = img.rel32_target(fr.at + 8);
    let ecx = fr.arg(g, 0);
    port(g, img, &Frame { at, ecx, ..*fr })
}

/// `magic_0387` 0x10003760: `EgoLinePos`'s visit, the thunk over
/// [`line_place_0387`].
pub fn jmp_line_place_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, line_place_0387)
}

/// [`jmp_line_place_0387`].
pub const JMP_LINE_PLACE_0387: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&LINE_PLACE_0387))],
    imports: &[],
};

/// `magic_0387` 0x10004e90: `EgoMovMat`'s visit, the thunk over
/// [`matrix_move_0387`].
pub fn jmp_matrix_move_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    thunk_to(g, img, fr, matrix_move_0387)
}

/// [`jmp_matrix_move_0387`].
pub const JMP_MATRIX_MOVE_0387: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&MATRIX_MOVE_0387))],
    imports: &[],
};

/// `call [G_R]` with `(m, src)`: the DLL's rotation pointer, its start
/// value the thunk over [`rotation_19d0`](crate::sb_matrix::rotation_19d0)
/// (run here, as the original runs it in the overlay); anything else is
/// called.
fn rotate_ptr<G: Game>(g: &mut G, img: &Image, ptr: u32, m: u32, src: u32) {
    let rot = g.read::<u32>(ptr);
    if img.matches(rot, &crate::s3_wrap::JMP_ROTATION_19D0) {
        crate::sb_matrix::rotation_19d0(g, img, img.rel32_target(rot + 11), m, src);
    } else {
        g.call(rot, 0, &[m, src]);
    }
}

/// `magic_0387`'s laser move (0x10004940, 559 bytes; internal, `ecx` =
/// the visitor `V`; a /GS frame: the request `R` at `ebp - 0x90`, the
/// local visitor `L` at `ebp - 0x74`, the steps `q` at `ebp - 0x7c`,
/// `- 0x78`, `- 0x70`, the cursor `P` at `ebp - 0x6c`, the matrix `M` at
/// `ebp - 0x68`, `Q` at `ebp - 0x38`, `D` at `ebp - 0x24`, `U` at `ebp -
/// 0x14`). After the [`walk`] (none: its `eax`), the node `O`'s bone
/// `O[0x6c]` is updated with `L`
/// ([`bone_plain`](crate::sa_summon::bone_plain)); a negative `O[0x70]`
/// becomes 0. Unless `u16 O[0x7a]` is 0, the global `G` is set or `u16
/// O[0x7c]` is 0 (the bone's `eax`): `R = (O, 4)` takes room from the
/// ring ([`ring_take`](crate::s6_misc::ring_take)), `n = u16 R[4]`, `D`
/// = slot 233 `(D, O + 0x3c, K / n)`, `q = O[0x5c..0x64] / n` (signed),
/// `P = R[0xc]`; then `n` times: `M` = the rotation of `O + 0x4c`
/// (through `*G_R`, see [`rotate_ptr`]), `Q` = slot 325 `(Q, O + 0x2c)`,
/// `M` = slot 231 `(M, O[0x6c] + 0x9c, M)`, `U = M (5, 15, 0, 1)` (slot
/// 229), `U[1] = K2`, `U[3] = O[0x70]`, slot 325 `(P, U)`, `P += 0x10`,
/// `O[0x2c..0x34] += D` and the ints `O[0x4c..0x54] += q`. Returns `O +
/// 0x4c`. A zero `n` faults in the original (`idiv`).
pub fn laser_move_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let v = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let at = |d: u32| ebp.wrapping_sub(d);
    let (r, l, m, q, d, u) = (at(0x90), at(0x74), at(0x68), at(0x38), at(0x24), at(0x14));
    // eax: the cookie word (`ds:__security_cookie ^ ebp`)
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    let (eax, o) = walk(g, img, v, at(0xa8), v, cookie);
    let Some(o) = o else {
        return Ret::eax(eax);
    };
    let bone = g.read::<u32>(o + 0x6c);
    g.write::<u32>(l, bone);
    let mode = g.read::<u8>(v + 5);
    g.write::<u8>(l + 5, mode);
    let eax = crate::sa_summon::bone_plain(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x6e),
            esp: at(0xa4),
            eax: (bone & !0xff) | u32::from(mode),
            ecx: l,
            ..Frame::default()
        },
    )
    .eax;
    // comiss 0, O[0x70]; jbe: only "above" (ordered) clears it
    if 0.0 > g.read::<f32>(o + 0x70) {
        g.write::<u32>(o + 0x70, 0);
    }
    if g.read::<u16>(o + 0x7a) == 0 || g.read::<u32>(w(0x8f)) != 0 || g.read::<u16>(o + 0x7c) == 0 {
        return Ret::eax(eax);
    }
    g.write::<u32>(r, o);
    g.write::<u16>(r + 4, 4);
    crate::s6_misc::ring_take(
        g,
        &Frame {
            at: img.rel32_target(f + 0xbe),
            esp: at(0xa4),
            ecx: r,
            ..Frame::default()
        },
    );
    let n = u32::from(g.read::<u16>(r + 4));
    let k = crate::sb_sse::div(g.read::<f32>(w(0xcd)), crate::sb_sse::cvt(n));
    g.call(slot_target(g, w(0xeb), 233), 0, &[d, o + 0x3c, k.to_bits()]);
    let ni = n.cast_signed();
    let step = |g: &G, off: u32| g.read::<u32>(o + off).cast_signed().wrapping_div(ni);
    let (q0, q1, q2) = (step(g, 0x5c), step(g, 0x60), step(g, 0x64));
    let mut p = g.read::<u32>(r + 0xc);
    for _ in 0..n {
        for (i, x) in [0x40a0_0000u32, 0x4170_0000, 0, ONE]
            .into_iter()
            .enumerate()
        {
            g.write::<u32>(u + 4 * i as u32, x);
        }
        rotate_ptr(g, img, w(0x153), m, o + 0x4c);
        g.call(slot_target(g, w(0x15d), 325), 0, &[q, o + 0x2c]);
        let b = g.read::<u32>(o + 0x6c).wrapping_add(0x9c);
        g.call(slot_target(g, w(0x17b), 231), 0, &[m, b, m]);
        g.call(slot_target(g, w(0x197), 229), 0, &[u, m, u]);
        let k2 = g.read::<u32>(w(0x1a7));
        g.write::<u32>(u + 4, k2);
        let y = g.read::<u32>(o + 0x70);
        g.write::<u32>(u + 0xc, y);
        g.call(slot_target(g, w(0x1b3), 325), 0, &[p, u]);
        let x = add(g.read::<f32>(d), g.read::<f32>(o + 0x2c));
        p = p.wrapping_add(0x10);
        g.write::<f32>(o + 0x2c, x);
        let x = add(g.read::<f32>(o + 0x30), g.read::<f32>(d + 4));
        g.write::<f32>(o + 0x30, x);
        let y = add(g.read::<f32>(o + 0x34), g.read::<f32>(d + 8));
        for (off, s) in [(0x4c, q0), (0x50, q1), (0x54, q2)] {
            let x = g.read::<u32>(o + off).wrapping_add(s.cast_unsigned());
            g.write::<u32>(o + off, x);
        }
        g.write::<f32>(o + 0x34, y);
    }
    Ret::eax(o + 0x4c)
}

/// [`laser_move_0387`] (the `rel32`s: the bone, the ring take, the
/// cookie check).
pub const LASER_MOVE_0387: Sig = Sig {
    len: 559,
    hash: 0x1d1c_db87_eec5_17d1,
    calls: &[
        (0x6e, Callee::Sig(&crate::sa_summon::BONE)),
        (0xbe, Callee::Sig(&crate::s6_misc::RING_TAKE)),
        (0x227, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387` 0x10004b70: `EgoLzrMv`'s visit (`push ecx; mov ecx,
/// [ebp + 8]; call`): [`laser_move_0387`] on the visitor (its `eax`).
pub fn call_laser_move_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let at = img.rel32_target(fr.at + 8);
    let ecx = fr.arg(g, 0);
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, fr.ecx);
    let r = laser_move_0387(
        g,
        img,
        &Frame {
            at,
            esp,
            ecx,
            ..*fr
        },
    );
    Ret::eax(r.eax)
}

/// [`call_laser_move_0387`].
pub const CALL_LASER_MOVE_0387: Sig = Sig {
    len: 15,
    hash: 0xd192_63e1_ae73_f895,
    calls: &[(8, Callee::Sig(&LASER_MOVE_0387))],
    imports: &[],
};

/// A node `n` set up under the root `root` and appended after its last
/// child, as the overlays inline it: `u16 +0x1a` = 0xff, `+0x10` and
/// `+0xc` itself, `+0x14` and `+4` the root, `+8` the root's old last
/// (`root + 0x10`, read twice), the old last's `+4` (or the root's
/// `+0xc` when it had none) = `n`, `u16 root + 0x18` up by one, `root +
/// 0x10` = `n`, then `n[0]` = `class`.
fn root_append<G: Game>(g: &mut G, n: u32, root: u32, class: u32) {
    crate::l3_common::node_init(g, n, root);
    crate::l3_common::attach(g, n, root, crate::l3_common::ListAt::of(root));
    g.write::<u32>(n, class);
}

/// `magic_0387`'s start task (0x10008b10, 866 bytes; cdecl `(T)`, `N =
/// T[4]`; an aligned frame `A`: the cursor at `A - 0x20`, the record
/// `R` at `A - 0x18`). By the state `N[0x18]` (the count `N[0x20]` drops
/// at every return):
/// - 0 becomes 0x10 and runs on;
/// - 0x10: `p` = slot 941 `(N[0x1290])`; unless `u16 p[0x3e]` has bit
///   0x800 (then it waits, `eax` 0): `p` again, the nodes `B = N + 0xd38`
///   and `C = N + 0xd64` set up under the root `G0` ([`root_append`],
///   class `K_VEC`), slot 685 `(N + 0xd54, p + 0x40 + 48 p[0x36])`,
///   `N + 0xd60` = 1, slot 325 `(N + 0xd80, N + 0xd28)`, `N[0xd8c] *= K`,
///   the owned node `N + 0xf08` under `G0`
///   ([`node_append_owned`](crate::sa_misc::node_append_owned) with
///   `(K_O, B, K_B, C)`), `N[0x1160]` = `K_CLASS`, the particle node from
///   `R = (N + 0x1160, K_L, N + 0xf08, *G_P, K_T, 3)`
///   ([`particle_node`](crate::sb_nodes::particle_node)); then with no
///   `N[0x1c]` slot 388 `(N[0x28][0x50], TBL[frnd & 1], 1)`, else for
///   each part `P` of `N[0x2c]` slot 388 `(P, TBL[frnd & 1], 1)` and
///   slot 239 `(*G_E, P, 6)` (the random bit from slot 324); then the
///   state is 0x1000;
/// - 0x1000: once `N[0x1248] & N[0x1148]` has bit 4: the nodes at
///   `N + 0xd38`, `0xd64`, `0xf08` leave, the cursor on `N + 0x1260` leaves
///   and registers ([`leave_register`](crate::s4_small::leave_register)),
///   `0xce0`, `0xd0c`, `0xd90`, `0xdbc` leave; the task `T[0]` is
///   unlinked from its ring, state 0x2000.
///
/// Returns the last value in `eax`.
#[allow(clippy::too_many_lines)] // the states
pub fn start_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let (cursor, r) = (a.wrapping_sub(0x20), a.wrapping_sub(0x18));
    let done = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(n + 0x20);
        g.write::<u32>(n + 0x20, c.wrapping_sub(1));
        Ret::eax(eax)
    };
    let state = g.read::<u32>(n + 0x18);
    match state {
        0 | 0x10 => {}
        0x1000 => {
            let gate = g.read::<u32>(n + 0x1248) & g.read::<u32>(n + 0x1148);
            if gate & 4 == 0 {
                return done(g, gate);
            }
            for off in [0xd38, 0xd64, 0xf08] {
                crate::sa_misc::leave(g, n + off, 0);
            }
            g.write::<u32>(cursor, n + 0x1260);
            crate::s4_small::leave_register(
                g,
                img,
                &Frame {
                    at: img.rel32_target(f + 0x104),
                    esp: a.wrapping_sub(0x34),
                    ecx: cursor,
                    ..Frame::default()
                },
            );
            for off in [0xce0, 0xd0c, 0xd90, 0xdbc] {
                crate::sa_misc::leave(g, n + off, 0);
            }
            let d = g.read::<u32>(tk);
            let (next, prev) = (g.read::<u32>(d + 4), g.read::<u32>(d + 8));
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(d + 8);
            let eax = g.read::<u32>(d + 4);
            g.write::<u32>(prev + 4, eax);
            g.write::<u32>(d + 8, d);
            g.write::<u32>(d + 4, d);
            let c = g.read::<u32>(n + 0x20);
            g.write::<u32>(n + 0x20, c.wrapping_sub(1));
            g.write::<u32>(n + 0x18, 0x2000);
            return Ret::eax(eax);
        }
        _ => return done(g, state),
    }
    if state == 0 {
        g.write::<u32>(n + 0x18, 0x10);
    }
    let s941 = |g: &mut G, op: u32| {
        let x = g.read::<u32>(n + 0x1290);
        g.call(slot_target(g, w(op), 941), 0, &[x]).eax
    };
    let p = s941(g, 0x28);
    let bit = u32::from(g.read::<u16>(p.wrapping_add(0x3e))) & 0x800;
    if bit == 0 {
        return done(g, 0);
    }
    let p = s941(g, 0x51);
    let root = w(0x7a);
    let class = w(0x171);
    let b = n + 0xd38;
    root_append(g, b, root, class);
    let i = u32::from(g.read::<u8>(p.wrapping_add(0x36)));
    let src = (i * 0x30 + 0x40).wrapping_add(p);
    g.call(slot_target(g, w(0x18d), 685), 0, &[n + 0xd54, src]);
    g.write::<u32>(n + 0xd60, ONE);
    let c = n + 0xd64;
    root_append(g, c, root, class);
    g.call(slot_target(g, w(0x1fb), 325), 0, &[n + 0xd80, n + 0xd28]);
    let x = mul(g.read::<f32>(n + 0xd8c), g.read::<f32>(w(0x21f)));
    g.write::<f32>(n + 0xd8c, x);
    // node_append_owned's four words over slot 325's two
    let esp = a.wrapping_sub(0x4c);
    for (i, x) in [w(0x237), b, w(0x22b), c].into_iter().enumerate() {
        g.write::<u32>(esp + 4 + 4 * i as u32, x);
    }
    crate::sa_misc::node_append_owned(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x24b),
            esp,
            ecx: n + 0xf08,
            edx: w(0x23c),
            ..Frame::default()
        },
    );
    let pp = g.read::<u32>(w(0x250));
    g.write::<u32>(n + 0x1160, w(0x260));
    for (i, x) in [n + 0x1160, w(0x26f), n + 0xf08, pp, w(0x27f), 3]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r + 4 * i as u32, x);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x28c), r);
    let pick = |g: &mut G, op: u32| {
        let bit = g.call(slot_target(g, w(op), 324), 0, &[]).eax & 1;
        g.read::<u32>(w(op + 0x14).wrapping_add(4 * bit))
    };
    let eax = if g.read::<u32>(n + 0x1c) == 0 {
        let t = pick(g, 0x323);
        let x = g.read::<u32>(g.read::<u32>(n + 0x28) + 0x50);
        g.call(slot_target(g, w(0x342), 388), 0, &[x, t, 1]).eax
    } else {
        let mut i = 0u32;
        let mut eax = g.read::<u32>(n + 0x2c);
        while i.cast_signed() < g.read::<u32>(eax + 0x4dc).cast_signed() {
            let t = pick(g, 0x2b1);
            let part = g.read::<u32>(g.read::<u32>(n + 0x2c) + 0x6c + 0x98 * i);
            g.call(slot_target(g, w(0x2d1), 388), 0, &[part, t, 1]);
            let part = g.read::<u32>(g.read::<u32>(n + 0x2c) + 0x6c + 0x98 * i);
            let e = g.read::<u32>(w(0x2f0));
            g.call(slot_target(g, w(0x2ea), 239), 0, &[e, part, 6]);
            eax = g.read::<u32>(n + 0x2c);
            i += 1;
        }
        eax
    };
    g.write::<u32>(n + 0x18, 0x1000);
    done(g, eax)
}

/// [`start_task_0387`] (the `rel32`s: three node leaves, the leave and
/// register, four node leaves, the owned append, the particle node).
pub const START_TASK_0387: Sig = Sig {
    len: 866,
    hash: 0x63e0_68fb_6514_27d9,
    calls: &[
        (0xdb, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe6, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xf1, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x104, Callee::Sig(&crate::s4_small::LEAVE_REGISTER)),
        (0x10f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x11a, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x125, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x130, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x24b, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x28c, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0387`'s shot set-up (0x10008e80, 744 bytes; internal, `ecx` =
/// the task `N`, `edx` = a vector `P`, `xmm2` = `x`, stack `(a0 .. a5)`;
/// the records `R` at `ebp - 0x1c` and `R2` at `ebp - 0x20`): `N` becomes
/// a task (class `VT`, callback [`start_task_0387`], `N + 0x14` itself)
/// linked after `*G_LAST` (`G_FLAGS` bit 0 cleared); under the root `G0`
/// ([`root_append`]): `B = N + 0xce0` (slot 325 `(N + 0xcfc, P)`, `N +
/// 0xd08` = 1), `D = N + 0xd90` (`N[0xdac..0xdb4] = (0, a1 a2, 0)`), `C =
/// N + 0xd0c` (`N[0xd28..0xd34] = (1, 1, 1, x K)`) and `E = N + 0xdbc`
/// (`E[0x13c] = B`, `E[0x140] = C`, `E[0x144] = D`, `E[0x128] = K_O`,
/// byte `E[0x148]` 0); the particle node from `R = (N + 0x1060, K_L, E,
/// *G_P, K_T, 2)` (`N[0x1060]` = `K_CLASS`); slot 676 `(N + 0x30, 1,
/// 0x20)`; the colour node from `R2 = (N + 0x1260, K_L, a0, B, N +
/// 0x30, K_C, E)` (`N[0x1260]` = `K_CLASS2`); bytes `N[0x1294] |=
/// 0x2c`, `N[0x12b4] |= 1`; `N[0x2c] = a5`, `N[0x1c] = a4`, `N[0x28] =
/// a3`, `N[0x24] = a2`, state `N[0x18]` 0. Returns `a2`.
pub fn shot_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let n = fr.ecx;
    let x = fr.xmm_f32(2);
    let ebp = fr.esp.wrapping_sub(4);
    let (r, r2) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x20));
    let arg = |g: &G, i: u32| fr.arg(g, i);
    task_link(g, img, f, n, [0x18, 0x1f, 0x27, 0x31, 0x36, 0x45, 0x4c]);
    let root = w(0x5f);
    let b = n + 0xce0;
    root_append(g, b, root, w(0xa5));
    g.call(slot_target(g, w(0x99), 325), 0, &[n + 0xcfc, fr.edx]);
    g.write::<u32>(n + 0xd08, ONE);
    let d = n + 0xd90;
    root_append(g, d, root, w(0xc6));
    let ab = arg(g, 1).wrapping_mul(arg(g, 2));
    g.write::<u32>(n + 0xdac, 0);
    g.write::<u32>(n + 0xdb0, ab);
    g.write::<u32>(n + 0xdb4, 0);
    let c = n + 0xd0c;
    root_append(g, c, root, w(0x18c));
    let s = mul(x, g.read::<f32>(w(0x180)));
    for off in [0xd28, 0xd2c, 0xd30] {
        g.write::<u32>(n + off, ONE);
    }
    g.write::<f32>(n + 0xd34, s);
    let e = n + 0xdbc;
    root_append(g, e, root, w(0x1be));
    let pp = g.read::<u32>(w(0x203));
    g.write::<u32>(e + 0x140, c);
    g.write::<u32>(e + 0x13c, b);
    g.write::<u32>(e + 0x144, d);
    g.write::<u32>(e + 0x128, w(0x22b));
    g.write::<u8>(e + 0x148, 0);
    g.write::<u32>(n + 0x1060, w(0x23b));
    for (i, v) in [n + 0x1060, w(0x245), e, pp, w(0x252), 2]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r + 4 * i as u32, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x25e), r);
    g.call(slot_target(g, w(0x263), 676), 0, &[n + 0x30, 1, 0x20]);
    let a0 = arg(g, 0);
    g.write::<u32>(n + 0x1260, w(0x28e));
    for (i, v) in [n + 0x1260, w(0x29b), a0, b, n + 0x30, w(0x2a8), e]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r2 + 4 * i as u32, v);
    }
    crate::sa_misc::colour_node_init(g, img, img.rel32_target(f + 0x2b0), r2);
    let a5 = arg(g, 5);
    let b1 = g.read::<u8>(n + 0x1294) | 0x2c;
    g.write::<u8>(n + 0x1294, b1);
    let b2 = g.read::<u8>(n + 0x12b4) | 1;
    g.write::<u8>(n + 0x12b4, b2);
    g.write::<u32>(n + 0x2c, a5);
    let a4 = arg(g, 4);
    g.write::<u32>(n + 0x1c, a4);
    let a3 = arg(g, 3);
    g.write::<u32>(n + 0x28, a3);
    let a2 = arg(g, 2);
    g.write::<u32>(n + 0x24, a2);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(a2)
}

/// [`shot_setup_0387`] (the `rel32`s: the particle node, the colour
/// node).
pub const SHOT_SETUP_0387: Sig = Sig {
    len: 744,
    hash: 0x35cf_449b_c032_06ca,
    calls: &[
        (0x25e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2b0, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
    ],
    imports: &[],
};

/// `magic_0387`'s shots task (0x10009170, 1701 bytes; cdecl `(T)`, `N =
/// T[4]`, `O = N[0x30]` the owner, `n = O[0x4dc]` its parts; an aligned
/// frame `A`, `S = A - 0x38`: the angle at `S + 0xc`, the part `q` at
/// `S + 0x10`, the draw `r` at `S + 0x14`, the vector `V` at `S +
/// 0x24`). By the state `N[0x18]`, with the wait `N[0x1c]` (every return
/// takes one off it and off the count `N[0x20]`):
/// - 0: once the wait is out, state 10 (run on);
/// - 10: for each part `i`, `X = N + 0x9774 + 0x2c i` and `Y = N +
///   0x9de0 + 0x44 i` go under the root `G0` ([`root_append`]): slot 268
///   `(O[0x70 + 0x98 i], 0x11, X + 0x1c)`, `X[0x28]` = 1, `Y[0x3c] = X`,
///   `Y[0x40] = 0`, `Y[0x2c..0x34] = (50, 0, 10)`; then state 11,
///   `N[0x28]` = 1, the count 0x38, the wait 0, and on;
/// - 11 (once the wait is out): `s = N[0x24] rem 8`; unless the shot
///   `N + 0x40 + 0x12c0 s` has state 0x2000 at its `+0x18` (then that is
///   all): `r` = slot 324; while `N[0x28]` is set and the count is at
///   most 20, `q = n / 2` and the angle `K`, else `q = r rem n` and the
///   angle `frnd + K1 - K2` (slot 307); `V = M (0, frnd K3 - K4, 0, 1)`
///   (slot 307, slot 229 with `M = N + 0x9894 + 0x14c (s & 1)`); the shot is
///   set up ([`shot_setup_0387`] with `edx` = `V`, the angle, `(Y_q, r &
///   0x3fff, 2 (s & 1) - 1, O + 0x1c + 0x98 q, flag, O)`); `N[0x28]`
///   clears under the same test, `N[0x24]` up; with count left the wait
///   is `r' rem 6 + 2` (slot 324), else state 10000 (run on);
/// - 10000: once `N[0x9dc8] & N[0x9cc8]` has bit 4 and the eight shots'
///   states all have bit 0x2000: the parts' `X` and `Y` nodes, four
///   nodes in each of two rows and two more leave; the task `T[0]` is
///   unlinked from its ring; state 20000.
///
/// Returns the last value in `eax` (the state for 1..9 and unknown ones).
#[allow(clippy::too_many_lines)] // the states
pub fn shots_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x38);
    let v = s0 + 0x24;
    let parts = |g: &G| g.read::<u32>(g.read::<u32>(n + 0x30) + 0x4dc).cast_signed();
    let end = |g: &mut G, eax: u32| {
        for off in [0x20, 0x1c] {
            let c = g.read::<u32>(n + off);
            g.write::<u32>(n + off, c.wrapping_sub(1));
        }
        Ret::eax(eax)
    };
    let mut state = g.read::<u32>(n + 0x18);
    let mut eax = state;
    loop {
        match state {
            0 => {
                if g.read::<u32>(n + 0x1c).cast_signed() > 0 {
                    return end(g, eax);
                }
                g.write::<u32>(n + 0x18, 10);
                state = 10;
                eax = 10;
            }
            10 => {
                let root = w(0x83);
                let mut i = 0u32;
                eax = g.read::<u32>(n + 0x30);
                while i.cast_signed() < parts(g) {
                    let x = n + 0x9774 + 0x2c * i;
                    let y = n + 0x9de0 + 0x44 * i;
                    root_append(g, x, root, w(0xbc));
                    let o = g.read::<u32>(n + 0x30);
                    let p = g.read::<u32>(o + 0x70 + 0x98 * i);
                    g.call(slot_target(g, w(0xc8), 268), 0, &[p, 0x11, x + 0x1c]);
                    g.write::<u32>(x + 0x28, ONE);
                    root_append(g, y, root, w(0xec));
                    g.write::<u32>(y + 0x40, 0);
                    g.write::<u32>(y + 0x3c, x);
                    g.write::<u32>(y + 0x2c, 0x4248_0000);
                    g.write::<u32>(y + 0x30, 0);
                    g.write::<u32>(y + 0x34, 0x4120_0000);
                    eax = g.read::<u32>(n + 0x30);
                    i += 1;
                }
                let st = g.read::<u32>(n + 0x18).wrapping_add(1);
                g.write::<u32>(n + 0x18, st);
                g.write::<u32>(n + 0x28, 1);
                g.write::<u32>(n + 0x20, 0x38);
                g.write::<u32>(n + 0x1c, 0);
                state = 11;
            }
            11 => {
                if g.read::<u32>(n + 0x1c).cast_signed() > 0 {
                    return end(g, eax);
                }
                let s = g.read::<u32>(n + 0x24).cast_signed() % 8;
                let shot = s.wrapping_mul(0x12c0).cast_unsigned();
                if g.read::<u32>(n.wrapping_add(0x58).wrapping_add(shot)) != 0x2000 {
                    return end(g, eax);
                }
                let odd = s.cast_unsigned() & 1;
                let r = g.call(slot_target(g, w(0x1c7), 324), 0, &[]).eax;
                g.write::<u32>(s0 + 0x14, r);
                let first = |g: &G| {
                    g.read::<u32>(n + 0x28) != 0 && g.read::<u32>(n + 0x20).cast_signed() <= 0x14
                };
                let (q, ang) = if first(g) {
                    (parts(g) / 2, k(g, 0x1ed))
                } else {
                    let q = r.cast_signed().wrapping_rem(parts(g));
                    let x = st0_f32(g.call(slot_target(g, w(0x223), 307), 0, &[]).st0);
                    (q, sub(add(x, k(g, 0x241)), k(g, 0x249)))
                };
                let q = q.cast_unsigned();
                g.write::<u32>(s0 + 0x10, q);
                g.write::<u32>(v, 0);
                g.write::<f32>(s0 + 0xc, ang);
                let y = st0_f32(g.call(slot_target(g, w(0x24e), 307), 0, &[]).st0);
                g.write::<f32>(s0 + 0x20, y);
                let y = sub(mul(y, k(g, 0x276)), k(g, 0x283));
                g.write::<f32>(v + 4, y);
                g.write::<u32>(v + 8, 0);
                g.write::<u32>(v + 0xc, ONE);
                let m = n + 0x9894 + 0x14c * odd;
                g.call(slot_target(g, w(0x29d), 229), 0, &[v, m, v]);
                let flag = u32::from(first(g));
                let o = g.read::<u32>(n + 0x30);
                let shot_args = [
                    n.wrapping_add(0x9de0).wrapping_add(0x44u32.wrapping_mul(q)),
                    r & 0x3fff,
                    (2 * odd).wrapping_sub(1),
                    0x98u32.wrapping_mul(q).wrapping_add(o + 0x1c),
                    flag,
                    o,
                ];
                let esp = s0.wrapping_sub(0x1c);
                for (i, x) in shot_args.into_iter().enumerate() {
                    g.write::<u32>(esp + 4 + 4 * i as u32, x);
                }
                let mut xmm = [0u128; 4];
                xmm[2] = u128::from(ang.to_bits());
                shot_setup_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x321),
                        esp,
                        ecx: n.wrapping_add(0x40).wrapping_add(shot),
                        edx: v,
                        xmm,
                        ..Frame::default()
                    },
                );
                if first(g) {
                    g.write::<u32>(n + 0x28, 0);
                }
                let c = g.read::<u32>(n + 0x24).wrapping_add(1);
                g.write::<u32>(n + 0x24, c);
                if g.read::<u32>(n + 0x20).cast_signed() > 0 {
                    let r2 = g
                        .call(slot_target(g, w(0x3f3), 324), 0, &[])
                        .eax
                        .cast_signed();
                    g.write::<u32>(n + 0x1c, (r2.wrapping_rem(6) + 2).cast_unsigned());
                    return end(g, r2.wrapping_div(6).cast_unsigned());
                }
                g.write::<u32>(n + 0x18, 10000);
                state = 10000;
            }
            10000 => {
                let gate = g.read::<u32>(n + 0x9dc8) & g.read::<u32>(n + 0x9cc8) & 4;
                if gate == 0 {
                    return end(g, 0);
                }
                let mut all = g.read::<u32>(n + 0x70d8);
                for off in [0x5e18, 0x4b58, 0x3898, 0x25d8, 0x1318, 0x8398, 0x58] {
                    all &= g.read::<u32>(n + off);
                }
                if all & 0x2000 == 0 {
                    return end(g, 0);
                }
                let leave = |g: &mut G, node: u32| {
                    crate::sa_misc::leave(g, node, 0);
                };
                let mut i = 0u32;
                while i.cast_signed() < parts(g) {
                    leave(g, n + 0x9774 + 0x2c * i);
                    leave(g, n + 0x9de0 + 0x44 * i);
                    i += 1;
                }
                for j in 0..2u32 {
                    leave(g, n + 0x971c + 0x2c * j);
                    leave(g, n + 0x96c4 + 0x2c * j);
                    leave(g, n + 0x9640 + 0x2c * j);
                    leave(g, n + 0x97f8 + 0x14c * j);
                }
                leave(g, n + 0x9698);
                leave(g, n + 0x9a90);
                let d = g.read::<u32>(tk);
                let (next, prev) = (g.read::<u32>(d + 4), g.read::<u32>(d + 8));
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(d + 8);
                let eax = g.read::<u32>(d + 4);
                g.write::<u32>(prev + 4, eax);
                g.write::<u32>(d + 8, d);
                g.write::<u32>(d + 4, d);
                g.write::<u32>(n + 0x18, 20000);
                return end(g, eax);
            }
            _ => return end(g, eax),
        }
    }
}

/// [`shots_task_0387`] (the `rel32`s: the shot set-up, the cookie
/// check).
pub const SHOTS_TASK_0387: Sig = Sig {
    len: 1701,
    hash: 0x6231_43a4_93e6_52d9,
    calls: &[(0x321, Callee::Sig(&SHOT_SETUP_0387)), (0x69d, Callee::Any)],
    imports: &[],
};

/// `N` becomes a task linked after the DLL's last one (the set-ups'
/// inlined link): `+8`, `+4` itself, `N[0]` the class, `+0x10` the
/// callback, `+0x14` itself; `+8` = `*G_LAST`, `+4` = the head, the old
/// last's `+4` = `N`, `G_FLAGS` bit 0 cleared, `*G_LAST` = `N`. `ops`:
/// the operand offsets of the class, the callback, `G_LAST` (read), the
/// head, `G_LAST` (read again), `G_FLAGS` and `G_LAST` (written).
fn task_link<G: Game>(g: &mut G, img: &Image, f: u32, n: u32, ops: [u32; 7]) {
    let w = |i: usize| img.word(f + ops[i]);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    g.write::<u32>(n, w(0));
    g.write::<u32>(n + 0x10, w(1));
    g.write::<u32>(n + 0x14, n);
    let last = g.read::<u32>(w(2));
    g.write::<u32>(n + 8, last);
    g.write::<u32>(n + 4, w(3));
    let last = g.read::<u32>(w(4));
    g.write::<u32>(last.wrapping_add(4), n);
    let flags = g.read::<u32>(w(5));
    g.write::<u32>(w(5), flags & !1);
    g.write::<u32>(w(6), n);
}

/// `magic_0387`'s pulse set-up (0x1000ba40, 584 bytes; internal, `ecx`
/// = the task `N`, `edx` = the effect `E`, stack `(O, x)`; a frame of
/// seven words, `l` at `ebp - 4`): `N` is linked as a task (callback
/// [`pulse_task_0387`]); under the root `G0`: the node `N + 0x868c`
/// ([`node_init`](crate::m0042::node_init) with `(E[0x54], 0x70)`), `D
/// = N + 0x8660` ([`root_append`]; then
/// [`time_left`](crate::m0042::time_left) of `E` into `N + 0x867c`) and
/// `C = N + 0x86d0` (`C[0x13c]` = `N + 0x868c`, `C[0x144] = D`,
/// `C[0x140]` = `K_P`, `C[0x128]` = `K_O`, byte `C[0x148]` 0);
/// `N[0x50..0x5c] = (0, 20, 30, 1)`; the sixteen pulses' states
/// `N[0x78 + 0x860 i]` = 0x2000; for each of `O`'s parts the count
/// `N[0x2c + 4 i]` = slot 291 `(*G_E, part, &l, &l, &l)` less 2, summed into
/// `N[0x28]`, and its weight `N[0x38 + 4 i]` = count / sum; `N[0x4c] =
/// x`, `N[0x48] = O`, `N[0x44] = E`, `N[0x24]` 0, the wait `N[0x1c]` 20,
/// the state `N[0x18]` 0. The parts' loop keeps its cursor in the
/// argument word `O` (the stack above the entry). Returns `E`.
pub fn pulse_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, e) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    task_link(g, img, f, n, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let root = w(0x5a);
    let r = ebp.wrapping_sub(0x18);
    let p = g.read::<u32>(e + 0x54);
    for (i, x) in [e, n + 0x868c, root, p, 0x70].into_iter().enumerate() {
        g.write::<u32>(r + 4 * i as u32, x);
    }
    crate::m0042::node_init(g, img, img.rel32_target(f + 0x69), r + 4);
    let d = n + 0x8660;
    root_append(g, d, root, w(0x75));
    let e2 = g.read::<u32>(r);
    g.write::<u32>(r + 0xc, e2);
    g.write::<u32>(r + 0x10, n + 0x867c);
    crate::m0042::time_left(
        g,
        img,
        img.rel32_target(f + 0xd2),
        r + 0xc,
        ebp.wrapping_sub(0x2c),
    );
    let c = n + 0x86d0;
    root_append(g, c, root, w(0xde));
    g.write::<u32>(c + 0x13c, n + 0x868c);
    g.write::<u32>(c + 0x144, d);
    g.write::<u32>(c + 0x140, w(0x13a));
    g.write::<u32>(c + 0x128, w(0x144));
    g.write::<u8>(c + 0x148, 0);
    for (off, x) in [
        (0x50, 0),
        (0x54, 0x41a0_0000),
        (0x58, 0x41f0_0000),
        (0x5c, ONE),
    ] {
        g.write::<u32>(n + off, x);
    }
    for i in 0..16 {
        g.write::<u32>(n + 0x78 + 0x860 * i, 0x2000);
    }
    let o = fr.arg(g, 0);
    let l = ebp.wrapping_sub(4);
    g.write::<u32>(n + 0x28, 0);
    let count = |g: &G| g.read::<u32>(o + 0x4dc).cast_signed();
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let part = g.read::<u32>(o + 0x6c + 0x98 * i);
        let s = g.read::<u32>(w(0x1af));
        let c = g
            .call(slot_target(g, w(0x1a9), 291), 0, &[s, part, l, l, l])
            .eax
            .wrapping_sub(2);
        g.write::<u32>(n + 0x2c + 4 * i, c);
        let sum = g.read::<u32>(n + 0x28).wrapping_add(c);
        g.write::<u32>(n + 0x28, sum);
        i += 1;
        g.write::<u32>(fr.esp + 4, n + 0x2c + 4 * i);
    }
    let sum = crate::sb_sse::cvt(g.read::<u32>(n + 0x28));
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let x = crate::sb_sse::div(crate::sb_sse::cvt(g.read::<u32>(n + 0x2c + 4 * i)), sum);
        g.write::<f32>(n + 0x38 + 4 * i, x);
        i += 1;
    }
    let x = fr.arg(g, 1);
    g.write::<u32>(n + 0x4c, x);
    g.write::<u32>(n + 0x48, o);
    g.write::<u32>(n + 0x44, e2);
    g.write::<u32>(n + 0x24, 0);
    g.write::<u32>(n + 0x1c, 0x14);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(e2)
}

/// [`pulse_setup_0387`] (the `rel32`s: the node init, the time left).
pub const PULSE_SETUP_0387: Sig = Sig {
    len: 584,
    hash: 0xcbd1_f88f_440b_38c7,
    calls: &[
        (0x69, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0xd2, Callee::Sig(&crate::m0042::TIME_LEFT)),
    ],
    imports: &[],
};

/// `magic_0387`'s glow task (0x1000bf10, 576 bytes; cdecl `(T)`, `N =
/// T[4]`; an aligned frame `A`: the cursor at `A - 0x18`, `V` at `A -
/// 0x14`). By the state `N[0x18]`, the wait `N[0x1c]` down by one at
/// every return:
/// - 0 becomes 0x10 (run on); 0x10: once slot 941 `(N[0x32f0])`'s `u16
///   +0x3e` has bit 0x800, state 0x50 (else its `eax`);
/// - 0x50: the wait 10, state 0x51; 0x51: once the wait is out, 0x80;
/// - 0x80: the wait 8; `N[0x339c..0x33a8]` = the four `u16 N[0x3440..]`
///   times `K`; slot 325 `(N + 0x33ac, V = (1, 1, 1, 0))`; `N[0x3458] |=
///   1`, `u16 N[0x3456]` and `N[0x3454]` = 8; state 0x81 (run on);
/// - 0x81: `N[0x32e8] -= K2`; once the wait is out, state 0x1000;
/// - 0x1000: once byte `N[0x3458]` has bit 4: the cursor on `N + 0x32c0`
///   leaves and registers, five nodes leave, `T[0]` is unlinked from its
///   ring, state 0x2000.
///
/// Returns the last value in `eax`.
#[allow(clippy::too_many_lines)] // the states
pub fn glow_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let (cursor, v) = (a.wrapping_sub(0x18), a.wrapping_sub(0x14));
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(n + 0x1c);
        g.write::<u32>(n + 0x1c, c.wrapping_sub(1));
        Ret::eax(eax)
    };
    let wait_out = |g: &G| g.read::<u32>(n + 0x1c).cast_signed() <= 0;
    let k = g.read::<f32>(w(0x26));
    loop {
        let state = g.read::<u32>(n + 0x18);
        match state {
            0 => g.write::<u32>(n + 0x18, 0x10),
            0x10 => {
                let x = g.read::<u32>(n + 0x32f0);
                let p = g.call(slot_target(g, w(0x6b), 941), 0, &[x]).eax;
                if g.read::<u16>(p.wrapping_add(0x3e)) & 0x800 == 0 {
                    return end(g, p);
                }
                g.write::<u32>(n + 0x18, 0x50);
            }
            0x50 | 0x51 => {
                if state == 0x50 {
                    g.write::<u32>(n + 0x1c, 10);
                    g.write::<u32>(n + 0x18, 0x51);
                }
                if !wait_out(g) {
                    return end(g, 0x51);
                }
                g.write::<u32>(n + 0x18, 0x80);
            }
            0x80 | 0x81 => {
                let mut eax = 0;
                if state == 0x80 {
                    g.write::<u32>(n + 0x1c, 8);
                    for (i, x) in [ONE, ONE, ONE, 0].into_iter().enumerate() {
                        g.write::<u32>(v + 4 * i as u32, x);
                    }
                    let si = g.read::<u16>(n + 0x1c);
                    for i in 0..4 {
                        let c = u32::from(g.read::<u16>(n + 0x3440 + 2 * i));
                        let x = mul(crate::sb_sse::cvt(c), k);
                        g.write::<f32>(n + 0x339c + 4 * i, x);
                    }
                    eax = g
                        .call(slot_target(g, w(0x1d6), 325), 0, &[n + 0x33ac, v])
                        .eax;
                    let b = g.read::<u32>(n + 0x3458) | 1;
                    g.write::<u32>(n + 0x3458, b);
                    g.write::<u16>(n + 0x3456, si);
                    g.write::<u16>(n + 0x3454, si);
                    g.write::<u32>(n + 0x18, 0x81);
                }
                let x = sub(g.read::<f32>(n + 0x32e8), g.read::<f32>(w(0x21e)));
                g.write::<f32>(n + 0x32e8, x);
                if !wait_out(g) {
                    return end(g, eax);
                }
                g.write::<u32>(n + 0x18, 0x1000);
            }
            0x1000 => {
                if g.read::<u8>(n + 0x3458) & 4 == 0 {
                    return end(g, 0);
                }
                g.write::<u32>(cursor, n + 0x32c0);
                crate::s4_small::leave_register(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0xcd),
                        esp: a.wrapping_sub(0x2c),
                        ecx: cursor,
                        ..Frame::default()
                    },
                );
                for off in [0x3274, 0x3128, 0x30d0, 0x30fc, 0x3320] {
                    crate::sa_misc::leave(g, n + off, 0);
                }
                let d = g.read::<u32>(tk);
                let (next, prev) = (g.read::<u32>(d + 4), g.read::<u32>(d + 8));
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(d + 8);
                let eax = g.read::<u32>(d + 4);
                g.write::<u32>(prev + 4, eax);
                g.write::<u32>(d + 8, d);
                g.write::<u32>(d + 4, d);
                g.write::<u32>(n + 0x18, 0x2000);
                return end(g, eax);
            }
            s if s > 0x51 => return end(g, s.wrapping_sub(0x1000)),
            s => return end(g, s),
        }
    }
}

/// [`glow_task_0387`] (the `rel32`s: the leave and register, five node
/// leaves, the cookie check).
pub const GLOW_TASK_0387: Sig = Sig {
    len: 576,
    hash: 0x3638_5d95_489c_a864,
    calls: &[
        (0xcd, Callee::Sig(&crate::s4_small::LEAVE_REGISTER)),
        (0xd8, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xee, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xf9, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x104, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x136, Callee::Any),
    ],
    imports: &[],
};

/// A node `n` set up under `parent` (no link when it is 0), as the
/// overlays inline it ([`node_init`](crate::l3_common::node_init) and
/// [`attach`](crate::l3_common::attach)), then `n[0]` = `class`.
fn child_append<G: Game>(g: &mut G, n: u32, parent: u32, class: u32) {
    crate::l3_common::node_init(g, n, parent);
    if parent != 0 {
        crate::l3_common::attach(g, n, parent, crate::l3_common::ListAt::of(parent));
    }
    g.write::<u32>(n, class);
}

/// `T[0]` unlinked from its ring (made its own): the overlays' task end.
/// Returns the `eax` the original leaves (`T[0]`'s `+4` as read last).
fn unlink_task<G: Game>(g: &mut G, tk: u32) -> u32 {
    let d = g.read::<u32>(tk);
    let (next, prev) = (g.read::<u32>(d + 4), g.read::<u32>(d + 8));
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(d + 8);
    let eax = g.read::<u32>(d + 4);
    g.write::<u32>(prev + 4, eax);
    g.write::<u32>(d + 8, d);
    g.write::<u32>(d + 4, d);
    eax
}

/// `magic_0387`'s pulse init (0x1000b1a0, 1182 bytes; internal, `ecx` =
/// the pulse `N`, `edx` = a vector `P`, stack `(-, -, a2, a3, a4, a5)`;
/// a /GS frame: `V` at `ebp - 0x14`, the group `R` at `ebp - 0x2c`): `N`
/// is linked as a task (callback the pulse step at 0x1000ab90); `B = N +
/// 0x44` under the root `G0` (slot 325 `(N + 0x60, P)`), `C = N + 0x9c`
/// under `B`, `V = (frnd K1 K2 - K3, (frnd - K4) K5 + K6, (frnd - K4)
/// K7 - K8, 0)` (slot 307 three times), slot 229 `(N + 0xb8, a4 + 0x9c,
/// V)`; `D = N + 0xc8` under `C`, `N[0xe4..0xec] = (0, 0.1, 0)`; the model
/// node `N + 0x298` ([`insert_0387`](crate::r14::insert_0387) with `(G0,
/// B, C)`, its class then `K_M`); `F = N + 0xf4` under `G0`,
/// `N[0x110..0x118] = (0x8000, 0, 0)`; `G = N + 0x120` under `F`,
/// `N[0x13c..0x144] = (-0x400, 0, 0)`; `H = N + 0x70` under `G0`,
/// `N[0x8c..0x98] = (1, 1, 1, K9)`; `I = N + 0x14c` under `G0` (`I[0x128]
/// = N + 0x298`, `I[0x13c]` = `K_T`, `I[0x144] = F`, `I[0x140] = H`, byte
/// `I[0x148]` 0); `J = N + 0x4f0` in the list `L` (`J[0x20]` = slot 719
/// `(K_E, 4, 0)`, `J[0x1c] = I`, `J[0x24]` = 0x80808080, class `K_J`);
/// `N[0x30] = a5`, `N[0x2c] = a3`, `N[0x20] = a2`, state 0. Returns `a2`.
pub fn pulse_init_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let n = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let (v, r) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x2c));
    let a4 = fr.arg(g, 4);
    task_link(g, img, f, n, [0x22, 0x29, 0x31, 0x3b, 0x40, 0x4c, 0x53]);
    let root = w(0x66);
    let b = n + 0x44;
    root_append(g, b, root, w(0xaa));
    g.call(slot_target(g, w(0x9e), 325), 0, &[n + 0x60, fr.edx]);
    let c = n + 0x9c;
    child_append(g, c, b, w(0xc1));
    let frnd = |g: &mut G, op: u32| st0_f32(g.call(slot_target(g, w(op), 307), 0, &[]).st0);
    let x = frnd(g, 0xf8);
    let v0 = sub(mul(mul(x, k(g, 0x110)), k(g, 0x11d)), k(g, 0x125));
    g.write::<f32>(v, v0);
    let x = frnd(g, 0x115);
    let v1 = add(mul(sub(x, k(g, 0x142)), k(g, 0x14f)), k(g, 0x157));
    g.write::<f32>(v + 4, v1);
    let x = frnd(g, 0x147);
    let v2 = sub(mul(sub(x, k(g, 0x174)), k(g, 0x188)), k(g, 0x197));
    g.write::<u32>(v + 0xc, 0);
    g.write::<f32>(v + 8, v2);
    g.call(slot_target(g, w(0x19d), 229), 0, &[n + 0xb8, a4 + 0x9c, v]);
    let d = n + 0xc8;
    child_append(g, d, c, w(0x1c0));
    g.write::<u32>(n + 0xe4, 0);
    g.write::<u32>(n + 0xe8, 0x3dcc_cccd);
    g.write::<u32>(n + 0xec, 0);
    for (i, x) in [n + 0x298, w(0x223), b, c].into_iter().enumerate() {
        g.write::<u32>(r + 4 * i as u32, x);
    }
    crate::r14::insert_0387(g, img, img.rel32_target(f + 0x22e), r);
    let m = g.read::<u32>(r);
    g.write::<u32>(m, w(0x23d));
    let fv = n + 0xf4;
    root_append(g, fv, root, w(0x243));
    for (off, x) in [(0x110, 0x8000), (0x114, 0), (0x118, 0)] {
        g.write::<u32>(n + off, x);
    }
    let gn = n + 0x120;
    child_append(g, gn, fv, w(0x2e5));
    for (off, x) in [(0x13c, 0xffff_fc00), (0x140, 0), (0x144, 0)] {
        g.write::<u32>(n + off, x);
    }
    let h = n + 0x70;
    root_append(g, h, root, w(0x352));
    for (off, x) in [(0x8c, ONE), (0x90, ONE), (0x94, ONE), (0x98, 0x39b2_4207)] {
        g.write::<u32>(n + off, x);
    }
    let i = n + 0x14c;
    root_append(g, i, root, w(0x386));
    g.write::<u32>(i + 0x128, n + 0x298);
    g.write::<u32>(i + 0x13c, w(0x3e9));
    g.write::<u32>(i + 0x144, fv);
    g.write::<u32>(i + 0x140, h);
    g.write::<u8>(i + 0x148, 0);
    let e = g
        .call(slot_target(g, w(0x3dd), 719), 0, &[w(0x409), 4, 0])
        .eax;
    let j = n + 0x4f0;
    let jl = w(0x424);
    crate::l3_common::node_init(g, j, jl);
    crate::l3_common::attach(g, j, jl, crate::l3_common::ListAt::of(jl));
    g.write::<u32>(j + 0x20, e);
    g.write::<u32>(j + 0x1c, i);
    g.write::<u32>(j + 0x24, 0x8080_8080);
    g.write::<u32>(j, w(0x476));
    let a5 = fr.arg(g, 5);
    g.write::<u32>(n + 0x30, a5);
    let a3 = fr.arg(g, 3);
    g.write::<u32>(n + 0x2c, a3);
    let a2 = fr.arg(g, 2);
    g.write::<u32>(n + 0x20, a2);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(a2)
}

/// [`pulse_init_0387`] (the `rel32`s: the model node insert, the cookie
/// check).
pub const PULSE_INIT_0387: Sig = Sig {
    len: 1182,
    hash: 0xcf23_2f8f_7592_f9d8,
    calls: &[
        (0x22e, Callee::Sig(&crate::r14::INSERT_0387)),
        (0x496, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387`'s pulse task (0x1000b640, 1019 bytes; cdecl `(T)`, `N =
/// T[4]`, `O = N[0x48]` the owner; an aligned frame `A`, `S = A - 0x48`:
/// the record `R` at `S + 0x1c`, `V` at `S + 0x34`). By the state
/// `N[0x18]`, with the wait `N[0x1c]` (every return takes one off it
/// and off the count `N[0x20]`):
/// - 0 (once the wait is out): the effect node `N + 0x881c`
///   ([`effect_node`](crate::sb_nodes::effect_node) with `R = (N +
///   0x881c, G0, K1, K2, K3, 1)`, its class then `K_E`), `N[0x8848] |=
///   6` and `|= 1`, `N[0x884c]` 0, state 10 (run on);
/// - 10: the count 0x70, the wait 0, state 11 (run on);
/// - 11 (once the wait is out): `s = N[0x24] rem 16`; unless the pulse
///   `N + 0x60 + 0x860 s` has state 0x2000 at its `+0x18` (then that is
///   all): `r` = slot 324; while weights are left (`N[0x28] > 0`) the
///   part `i` is drawn by the weights `N[0x38 + 4 i]` (slot 307, `frnd -
///   K`, as [`weights_step_0387`](crate::r45::weights_step_0387)) and
///   `N[0x28]` drops, else `i = r rem n`; `V` = slot 325 `(V, N + 0x50)`,
///   slot 229 `(V, N + 0x876c, V)`, a draw of slot 307 dropped; the pulse
///   starts ([`pulse_init_0387`] with `edx = V`, `(-, -, 2 (s & 1) - 1,
///   O + 0x1c + 0x98 i, N + 0x86d0, N[0x4c])`); `N[0x24]` up; with count
///   left the drawn part's count `N[0x2c + 4 i]` drops and the weights
///   are recomputed (when drawn), and the wait is `r' rem 8 + 4` (slot
///   324), else state 10000 (run on);
/// - 10000: once the sixteen pulses' states all have bit 0x2000: three
///   nodes leave, `T[0]` is unlinked, state 20000.
///
/// Returns the last value in `eax` (the state for 1..9 and unknown ones).
#[allow(clippy::too_many_lines)] // the states
pub fn pulse_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x48);
    let (r, v) = (s0 + 0x1c, s0 + 0x34);
    let parts = |g: &G| g.read::<u32>(g.read::<u32>(n + 0x48) + 0x4dc).cast_signed();
    let end = |g: &mut G, eax: u32| {
        for off in [0x20, 0x1c] {
            let c = g.read::<u32>(n + off);
            g.write::<u32>(n + off, c.wrapping_sub(1));
        }
        Ret::eax(eax)
    };
    let wait_out = |g: &G| g.read::<u32>(n + 0x1c).cast_signed() <= 0;
    let mut state = g.read::<u32>(n + 0x18);
    let mut eax = state;
    loop {
        match state {
            0 => {
                if !wait_out(g) {
                    return end(g, eax);
                }
                let e = n + 0x881c;
                for (i, x) in [e, w(0x119), w(0x121), w(0x129), w(0x131), 1]
                    .into_iter()
                    .enumerate()
                {
                    g.write::<u32>(r + 4 * i as u32, x);
                }
                crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x13e), r);
                let node = g.read::<u32>(r);
                g.write::<u32>(node, w(0x148));
                let x = g.read::<u32>(n + 0x8848) | 6;
                g.write::<u32>(n + 0x8848, x);
                let x = g.read::<u32>(e + 0x2c) | 1;
                g.write::<u32>(e + 0x2c, x);
                g.write::<u32>(e + 0x30, 0);
                g.write::<u32>(n + 0x18, 10);
                state = 10;
            }
            10 => {
                g.write::<u32>(n + 0x20, 0x70);
                g.write::<u32>(n + 0x1c, 0);
                g.write::<u32>(n + 0x18, 11);
                state = 11;
                eax = 11;
            }
            11 => {
                if !wait_out(g) {
                    return end(g, eax);
                }
                let s = g.read::<u32>(n + 0x24).cast_signed() % 16;
                let off = s.wrapping_mul(0x860).cast_unsigned();
                if g.read::<u32>(off.wrapping_add(n).wrapping_add(0x78)) != 0x2000 {
                    return end(g, off);
                }
                let odd = s.cast_unsigned() & 1;
                let rv = g.call(slot_target(g, w(0x8c), 324), 0, &[]).eax;
                let drawn = g.read::<u32>(n + 0x28).cast_signed() > 0;
                let i = if drawn {
                    let x = st0_f32(g.call(slot_target(g, w(0xb2), 307), 0, &[]).st0);
                    let last = parts(g).wrapping_sub(1);
                    let mut y = sub(x, g.read::<f32>(w(0xd5)));
                    let mut i = 0i32;
                    while i < last {
                        let wt = g.read::<f32>(n + 0x38 + 4 * i.cast_unsigned());
                        // comiss wt, y; ja: above (ordered) stops
                        if wt > y {
                            break;
                        }
                        i += 1;
                        y = sub(y, wt);
                    }
                    let c = g.read::<u32>(n + 0x28).wrapping_sub(1);
                    g.write::<u32>(n + 0x28, c);
                    i.cast_unsigned()
                } else {
                    rv.cast_signed().wrapping_rem(parts(g)).cast_unsigned()
                };
                g.call(slot_target(g, w(0x182), 325), 0, &[v, n + 0x50]);
                g.call(slot_target(g, w(0x1a0), 229), 0, &[v, n + 0x876c, v]);
                g.call(slot_target(g, w(0x1ad), 307), 0, &[]);
                let o = g.read::<u32>(n + 0x48);
                let esp = s0.wrapping_sub(0x1c);
                let a4 = n + 0x86d0;
                let a5 = g.read::<u32>(n + 0x4c);
                let a3 = (0x98u32.wrapping_mul(i)).wrapping_add(o + 0x1c);
                for (j, x) in [(2u32, (2 * odd).wrapping_sub(1)), (3, a3), (4, a4), (5, a5)] {
                    g.write::<u32>(esp + 4 + 4 * j, x);
                }
                pulse_init_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x1f2),
                        esp,
                        ecx: off.wrapping_add(0x60).wrapping_add(n),
                        edx: v,
                        ..Frame::default()
                    },
                );
                let c = g.read::<u32>(n + 0x24).wrapping_add(1);
                g.write::<u32>(n + 0x24, c);
                if g.read::<u32>(n + 0x20).cast_signed() > 0 {
                    if drawn {
                        let at = n.wrapping_add(0x2c).wrapping_add(4u32.wrapping_mul(i));
                        let c = g.read::<u32>(at).wrapping_sub(1);
                        g.write::<u32>(at, c);
                        let mut j = 0u32;
                        while j.cast_signed() < parts(g) {
                            let c = crate::sb_sse::cvt(g.read::<u32>(n + 0x2c + 4 * j));
                            let t = crate::sb_sse::cvt(g.read::<u32>(n + 0x28));
                            g.write::<f32>(n + 0x38 + 4 * j, crate::sb_sse::div(c, t));
                            j += 1;
                        }
                    }
                    let r2 = g.call(slot_target(g, w(0x2e9), 324), 0, &[]).eax;
                    let wait = (r2.cast_signed() % 8 + 4).cast_unsigned();
                    g.write::<u32>(n + 0x1c, wait);
                    return end(g, wait);
                }
                g.write::<u32>(n + 0x18, 10000);
                state = 10000;
            }
            10000 => {
                let mut all = 0x2000u32;
                let mut eax = 0;
                for k in 0..4u32 {
                    let p = |j: u32| n + 0x78 + 0x860 * (4 * k + j);
                    eax = g.read::<u32>(p(0))
                        & g.read::<u32>(p(1))
                        & g.read::<u32>(p(2))
                        & g.read::<u32>(p(3))
                        & 0x2000;
                    all &= eax;
                }
                if all != 0x2000 {
                    return end(g, eax);
                }
                for off in [0x868c, 0x8660, 0x86d0] {
                    crate::sa_misc::leave(g, n + off, 0);
                }
                let eax = unlink_task(g, tk);
                g.write::<u32>(n + 0x18, 20000);
                return end(g, eax);
            }
            _ => return end(g, eax),
        }
    }
}

/// [`pulse_task_0387`] (the `rel32`s: the effect node, the pulse init,
/// the cookie check).
pub const PULSE_TASK_0387: Sig = Sig {
    len: 1019,
    hash: 0x5bae_b1a0_9794_9ab6,
    calls: &[
        (0x13e, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1f2, Callee::Sig(&PULSE_INIT_0387)),
        (0x3f3, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387`'s burst set-up (0x1000a8e0, 678 bytes; internal, `ecx`
/// = the task `N`, `edx` = the source node `Q`; the record `R` at `ebp -
/// 0x1c`): `N` is linked as a task (callback the burst task at
/// 0x10009c20); under the root `G0` ([`root_append`]): `B = N + 0x6c`
/// (slot 325 `(N + 0x88, Q + 0xcc)`), `C = N + 0x98` (slot 325 `(N +
/// 0xb4, Q[0x140] + 0x1c)`, `N[0xc0] *= K`), `D = N + 0xc4`
/// (`N[0xe0..0xe8]` = `Q[0x144]`'s `+0x1c..0x24`, the first plus
/// 0x8000) and `E = N + 0xf0` (`E[0x13c..0x144] = (B, C, D)`, `E[0x128]`
/// = `K_O`, byte `E[0x148]` 0); the particle node from `R = (N + 0x240,
/// K_L, E, *G_P, K_T, 7)`; `N[0x28] = Q`, `N[0x24]` = slot 719 `(K_E, 9,
/// 0)` (its `eax` returned), the state `N[0x18]` 0.
pub fn burst_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, q) = (fr.ecx, fr.edx);
    let r = fr.esp.wrapping_sub(4).wrapping_sub(0x1c);
    task_link(g, img, f, n, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x3d, 0x44]);
    let root = w(0x57);
    let b = n + 0x6c;
    root_append(g, b, root, w(0xa6));
    g.call(slot_target(g, w(0x9a), 325), 0, &[n + 0x88, q + 0xcc]);
    let c = n + 0x98;
    root_append(g, c, root, w(0x106));
    let src = g.read::<u32>(q + 0x140).wrapping_add(0x1c);
    g.call(slot_target(g, w(0x11c), 325), 0, &[n + 0xb4, src]);
    let x = mul(g.read::<f32>(n + 0xc0), g.read::<f32>(w(0x134)));
    g.write::<f32>(n + 0xc0, x);
    let d = n + 0xc4;
    root_append(g, d, root, w(0x14b));
    for (i, off) in [0x1cu32, 0x20, 0x24].into_iter().enumerate() {
        let p = g.read::<u32>(q + 0x144);
        let mut x = g.read::<u32>(p.wrapping_add(off));
        if i == 0 {
            x = x.wrapping_add(0x8000);
        }
        g.write::<u32>(n + 0xe0 + 4 * i as u32, x);
    }
    let e = n + 0xf0;
    root_append(g, e, root, w(0x1d2));
    let pp = g.read::<u32>(w(0x217));
    g.write::<u32>(e + 0x144, d);
    g.write::<u32>(e + 0x13c, b);
    g.write::<u32>(e + 0x140, c);
    g.write::<u32>(e + 0x128, w(0x23f));
    g.write::<u8>(e + 0x148, 0);
    g.write::<u32>(n + 0x240, w(0x24f));
    for (i, v) in [n + 0x240, w(0x259), e, pp, w(0x266), 7]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r + 4 * i as u32, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x272), r);
    g.write::<u32>(n + 0x28, q);
    let eax = g
        .call(slot_target(g, w(0x27d), 719), 0, &[w(0x28c), 9, 0])
        .eax;
    g.write::<u32>(n + 0x24, eax);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(eax)
}

/// [`burst_setup_0387`] (the `rel32`: the particle node).
pub const BURST_SETUP_0387: Sig = Sig {
    len: 678,
    hash: 0x14dd_0e9b_7691_87e0,
    calls: &[(0x272, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE))],
    imports: &[],
};

/// `magic_0387`'s glow set-up (0x1000c150, 835 bytes; internal, `ecx` =
/// the task `N`, `edx` = `X`, stack `(a0)`; the records `R` at `ebp -
/// 0x1c` and `R2` at `ebp - 0x20`): `N` is linked as a task (callback
/// [`glow_task_0387`]); under the root `G0` ([`root_append`]): `A = N +
/// 0x3320` (`A[0x3c] = X`, `A[0x40]` 0; `N[0x334c..0x3354] = (100, 1,
/// 5)`), `B = N + 0x3274` (`B[0x38]` = 1, `B[0x3c] = a0`; `N[0x32a0..]`
/// three zeros), `C = N + 0x30fc` (`N[0x3118..]` three zeros), `D = N +
/// 0x30d0` (`N[0x30ec..0x30f8] = (1, 1, 1, 0.4)`) and `E = N + 0x3128`
/// (`E[0x13c..0x144] = (B, D, C)`, `E[0x128]` = `K_O`, byte `E[0x148]`
/// 0); the particle node from `R = (N + 0x3370, K_L, E, *G_P, K_T, 1)`;
/// slot 676 `(N + 0x20, 2, 0x80)`; the colour node from `R2 = (N +
/// 0x32c0, K_L, A, B, N + 0x20, K_C, a0)`; bytes `N[0x32f4] |= 0x18`,
/// `N[0x3314] |= 1`, the state `N[0x18]` 0. Returns the colour node's
/// `eax`.
pub fn glow_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, x) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (r, r2) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x20));
    task_link(g, img, f, n, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let root = w(0x60);
    let a = n + 0x3320;
    root_append(g, a, root, w(0x4d));
    g.write::<u32>(a + 0x3c, x);
    g.write::<u32>(a + 0x40, 0);
    for (off, v) in [(0x334c, 0x42c8_0000), (0x3350, ONE), (0x3354, 0x40a0_0000)] {
        g.write::<u32>(n + off, v);
    }
    let b = n + 0x3274;
    root_append(g, b, root, w(0x11a));
    let a0 = fr.arg(g, 0);
    g.write::<u32>(b + 0x38, ONE);
    g.write::<u32>(b + 0x3c, a0);
    for off in [0x32a0, 0x32a4, 0x32a8] {
        g.write::<u32>(n + off, 0);
    }
    let c = n + 0x30fc;
    root_append(g, c, root, w(0x144));
    for off in [0x3118, 0x311c, 0x3120] {
        g.write::<u32>(n + off, 0);
    }
    let d = n + 0x30d0;
    root_append(g, d, root, w(0x1fa));
    for (off, v) in [
        (0x30ec, ONE),
        (0x30f0, ONE),
        (0x30f4, ONE),
        (0x30f8, 0x3ecc_cccd),
    ] {
        g.write::<u32>(n + off, v);
    }
    let e = n + 0x3128;
    root_append(g, e, root, w(0x22e));
    let pp = g.read::<u32>(w(0x273));
    g.write::<u32>(e + 0x144, c);
    g.write::<u32>(e + 0x13c, b);
    g.write::<u32>(e + 0x140, d);
    g.write::<u32>(e + 0x128, w(0x29b));
    g.write::<u8>(e + 0x148, 0);
    g.write::<u32>(n + 0x3370, w(0x2ab));
    for (i, v) in [n + 0x3370, w(0x2b5), e, pp, w(0x2c2), 1]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r + 4 * i as u32, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x2ce), r);
    g.call(slot_target(g, w(0x2d3), 676), 0, &[n + 0x20, 2, 0x80]);
    let a0 = fr.arg(g, 0);
    g.write::<u32>(n + 0x32c0, w(0x31e));
    for (i, v) in [n + 0x32c0, w(0x30b), a, b, n + 0x20, w(0x318), a0]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r2 + 4 * i as u32, v);
    }
    let eax = crate::sa_misc::colour_node_init(g, img, img.rel32_target(f + 0x323), r2);
    let b1 = g.read::<u8>(n + 0x32f4) | 0x18;
    g.write::<u8>(n + 0x32f4, b1);
    let b2 = g.read::<u8>(n + 0x3314) | 1;
    g.write::<u8>(n + 0x3314, b2);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(eax)
}

/// [`glow_setup_0387`] (the `rel32`s: the particle node, the colour
/// node).
pub const GLOW_SETUP_0387: Sig = Sig {
    len: 835,
    hash: 0x7eb3_4968_07fe_0183,
    calls: &[
        (0x2ce, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x323, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
    ],
    imports: &[],
};

/// `magic_0387`'s laser task (0x1000c4a0, 572 bytes; cdecl `(T)`, `N =
/// T[4]`; an aligned frame `A`, `S = A - 0x38`: the record `R` at `S +
/// 0xc`, `V` at `S + 0x24`). By the state `N[0x18]` (states 0x41..0x44
/// through a jump table), the wait `N[0x1c]` and the count `N[0x20]`
/// down by one at every return:
/// - 0: once `N[0xbb8]` is at least 0x50, the particle node from
///   `R = (N + 0x5b0, K_L, N + 0x464, *G_P, K_T, 1)`, state 0x10 (run
///   on);
/// - 0x10: the wait 8, state 0x11; 0x11: once the wait is out, `u16
///   N[0x72a]` = 1, state 0x40 (run on);
/// - 0x40: state 0x41, the wait 0, the count 4; when `N[0xbb8]` is at
///   least 0x80 the sprite `N + 0x5b0` takes `V = (1, 1, 1, 0)` and 8
///   ([`sprite_size`](crate::m0387::sprite_size)) and the state goes
///   on to 0x42; then as 0x42;
/// - 0x42: unless `i16 N[0x694]` is positive, `u16 N[0x72a]` 0 and the
///   next state; then as 0x43;
/// - 0x43: once the count is out: while the wait is out byte `N[0xb18]
///   += 3` and the wait 2; at 0x30 or more the next state, the wait 10;
/// - 0x44: once the wait is out, state 0x1000 (run on);
/// - 0x1000: once `N[0xbb8]` has bit 0x2000 and byte `N[0x698]` bit 4:
///   five nodes leave, `T[0]` is unlinked, state 0x2000.
///
/// Returns the last value in `eax`.
#[allow(clippy::too_many_lines)] // the states
pub fn laser_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x38);
    let (r, v) = (s0 + 0xc, s0 + 0x24);
    let end = |g: &mut G, eax: u32| {
        for off in [0x1c, 0x20] {
            let c = g.read::<u32>(n + off);
            g.write::<u32>(n + off, c.wrapping_sub(1));
        }
        Ret::eax(eax)
    };
    let wait_out = |g: &G| g.read::<u32>(n + 0x1c).cast_signed() <= 0;
    loop {
        let state = g.read::<u32>(n + 0x18);
        // the jump table's entry for 0x41..0x44 (read, as the original)
        let entry = |i: u32| img.word(w(0xe7) + 4 * i).wrapping_sub(f);
        let mut at = match state {
            0 => 0x6e,
            0x10 => 0x49,
            0x11 => 0x54,
            0x40 => 0x101,
            0x1000 => 0x1a7,
            0x41..=0x44 => entry(state - 0x41),
            s if s > 0x40 && s < 0x1000 => return end(g, s - 0x41),
            s => return end(g, s),
        };
        let mut eax = if (0x41..=0x44).contains(&state) {
            state - 0x41
        } else {
            state
        };
        loop {
            match at {
                0x6e => {
                    if g.read::<u32>(n + 0xbb8) < 0x50 {
                        return end(g, eax);
                    }
                    let pp = g.read::<u32>(w(0x8c));
                    g.write::<u32>(n + 0x5b0, w(0x96));
                    for (i, x) in [n + 0x5b0, w(0xa2), n + 0x464, pp, w(0xae), 1]
                        .into_iter()
                        .enumerate()
                    {
                        g.write::<u32>(r + 4 * i as u32, x);
                    }
                    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0xb7), r);
                    g.write::<u32>(n + 0x18, 0x10);
                    break;
                }
                0x49 => {
                    g.write::<u32>(n + 0x1c, 8);
                    g.write::<u32>(n + 0x18, 0x11);
                    eax = 0x11;
                    at = 0x54;
                }
                0x54 => {
                    if !wait_out(g) {
                        return end(g, eax);
                    }
                    g.write::<u16>(n + 0x72a, 1);
                    g.write::<u32>(n + 0x18, 0x40);
                    break;
                }
                0x101 => {
                    let st = g.read::<u32>(n + 0x18).wrapping_add(1);
                    g.write::<u32>(n + 0x18, st);
                    g.write::<u32>(n + 0x1c, 0);
                    g.write::<u32>(n + 0x20, 4);
                    if g.read::<u32>(n + 0xbb8) >= 0x80 {
                        for (i, x) in [ONE, ONE, ONE, 0].into_iter().enumerate() {
                            g.write::<u32>(v + 4 * i as u32, x);
                        }
                        let esp = s0.wrapping_sub(8);
                        g.write::<u32>(esp + 4, 8);
                        crate::m0387::sprite_size(
                            g,
                            img,
                            &Frame {
                                at: img.rel32_target(f + 0x14b),
                                esp,
                                ecx: n + 0x5b0,
                                edx: v,
                                ..Frame::default()
                            },
                        );
                        let st = g.read::<u32>(n + 0x18).wrapping_add(1);
                        g.write::<u32>(n + 0x18, st);
                    }
                    at = 0x155;
                }
                0x155 => {
                    eax = g.read::<u32>(n + 0x18);
                    if eax >= 0x42 && g.read::<i16>(n + 0x694) <= 0 {
                        g.write::<u16>(n + 0x72a, 0);
                        eax += 1;
                        g.write::<u32>(n + 0x18, eax);
                    }
                    at = 0x174;
                }
                0x174 => {
                    if g.read::<u32>(n + 0x20).cast_signed() > 0 {
                        return end(g, eax);
                    }
                    if wait_out(g) {
                        let b = g.read::<u8>(n + 0xb18).wrapping_add(3);
                        g.write::<u8>(n + 0xb18, b);
                        g.write::<u32>(n + 0x1c, 2);
                    }
                    if g.read::<u8>(n + 0xb18) >= 0x30 {
                        let st = g.read::<u32>(n + 0x18).wrapping_add(1);
                        g.write::<u32>(n + 0x18, st);
                        g.write::<u32>(n + 0x1c, 10);
                    }
                    return end(g, eax);
                }
                0xeb => {
                    if !wait_out(g) {
                        return end(g, eax);
                    }
                    g.write::<u32>(n + 0x18, 0x1000);
                    break;
                }
                _ => {
                    // 0x1a7: state 0x1000
                    if g.read::<u32>(n + 0xbb8) & 0x2000 == 0 || g.read::<u8>(n + 0x698) & 4 == 0 {
                        return end(g, eax);
                    }
                    for off in [0x464, 0x438, 0xb2c, 0x730, 0x6b0] {
                        crate::sa_misc::leave(g, n + off, 0);
                    }
                    let eax = unlink_task(g, tk);
                    g.write::<u32>(n + 0x18, 0x2000);
                    return end(g, eax);
                }
            }
        }
    }
}

/// [`laser_task_0387`] (the `rel32`s: the particle node, the sprite
/// size, five node leaves, the cookie check).
pub const LASER_TASK_0387: Sig = Sig {
    len: 572,
    hash: 0x07cd_fe69_2fa9_6a39,
    calls: &[
        (0xb7, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x14b, Callee::Sig(&crate::m0387::SPRITE_SIZE)),
        (0x1c3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1ce, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1d9, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1e4, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1ef, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x224, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387`'s laser set-up (0x1000c6e0, 661 bytes; internal, `ecx`
/// = the task `N`, `edx` = `X`, stack `(a0)`; the records `R` at `ebp -
/// 0x18` and `R2` at `ebp - 0x14`): `N` is linked as a task (callback
/// [`laser_task_0387`]); the strip node `N + 0x6b0`
/// ([`strip_record`](crate::m0387::strip_record) with `(G0, N + 0x38,
/// 0x100040, X)`, class `K_S`); with the draws `r1`, `r2`, `r3` (slot
/// 324) `N[0x70c] = ((r2 & 0x1000) + 0x1800) (1 - (r1 & 2))`,
/// `N[0x710] = ((r3 & 0x1000) + 0x1800) (1 - ((r1 >> 1) & 2))`,
/// `N[0x714]` 0; the sprite node `N + 0x730`
/// ([`sprite_record`](crate::m0387::sprite_record) with `(K_L, N +
/// 0x6b0, slot 719 (K_E, 9, 1))`, class `K_P`); under the root `G0`: `S
/// = N + 0xb2c` (`S[0x2c]` = `N + 0x6b0`, `S[0x30]` 0), `D = N + 0x438`
/// (`N[0x454..0x460] = (1, 1, 1, 0.5)`) and `E = N + 0x464`
/// (`E[0x13c..0x144] = (S, D, K_T)`, `E[0x128]` = `K_O`, byte
/// `E[0x148]` 0); the glow at `N + 0xba0` ([`glow_setup_0387`] with
/// `S` and `a0`); state `N[0x18]` 0. Returns the glow set-up's `eax`.
pub fn laser_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, x) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (r, r2) = (ebp.wrapping_sub(0x18), ebp.wrapping_sub(0x14));
    task_link(g, img, f, n, [0x12, 0x19, 0x21, 0x2b, 0x30, 0x3f, 0x4d]);
    let strip = n + 0x6b0;
    for (i, v) in [strip, w(0x57), n + 0x38, 0x0010_0040, x]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r + 4 * i as u32, v);
    }
    let fr1 = Frame {
        at: img.rel32_target(f + 0x69),
        esp: ebp.wrapping_sub(0x28),
        ecx: r,
        ..Frame::default()
    };
    crate::m0387::strip_record(g, img, &fr1);
    g.write::<u32>(strip, w(0x74));
    let draw = |g: &mut G, op: u32| g.call(slot_target(g, w(op), 324), 0, &[]).eax;
    let r1 = draw(g, 0x6e);
    let rr = draw(g, 0x82);
    let side = 1u32.wrapping_sub(r1 & 2);
    g.write::<u32>(n + 0x70c, ((rr & 0x1000) + 0x1800).wrapping_mul(side));
    let rr = draw(g, 0xb2);
    let side = 1u32.wrapping_sub((r1.cast_signed() >> 1).cast_unsigned() & 2);
    g.write::<u32>(n + 0x710, ((rr & 0x1000) + 0x1800).wrapping_mul(side));
    g.write::<u32>(n + 0x714, 0);
    let sp = n + 0x730;
    g.write::<u32>(r2, sp);
    g.write::<u32>(r2 + 4, w(0x108));
    g.write::<u32>(r2 + 8, strip);
    let e = g
        .call(slot_target(g, w(0xd9), 719), 0, &[w(0xfe), 9, 1])
        .eax;
    g.write::<u32>(r2 + 0xc, e);
    let fr2 = Frame {
        at: img.rel32_target(f + 0x11b),
        esp: ebp.wrapping_sub(0x28),
        ecx: r2,
        ..Frame::default()
    };
    crate::m0387::sprite_record(g, img, &fr2);
    g.write::<u32>(sp, w(0x121));
    let root = w(0x13a);
    let s = n + 0xb2c;
    root_append(g, s, root, w(0x17d));
    g.write::<u32>(s + 0x2c, strip);
    g.write::<u32>(s + 0x30, 0);
    let d = n + 0x438;
    root_append(g, d, root, w(0x1cf));
    for (off, v) in [
        (0x454, ONE),
        (0x458, ONE),
        (0x45c, ONE),
        (0x460, 0x3f00_0000),
    ] {
        g.write::<u32>(n + off, v);
    }
    let el = n + 0x464;
    root_append(g, el, root, w(0x203));
    let a0 = fr.arg(g, 0);
    g.write::<u32>(el + 0x140, d);
    g.write::<u32>(el + 0x13c, s);
    g.write::<u32>(el + 0x144, w(0x262));
    g.write::<u32>(el + 0x128, w(0x26c));
    g.write::<u8>(el + 0x148, 0);
    let esp = ebp.wrapping_sub(0x2c);
    g.write::<u32>(esp + 4, a0);
    let ret = glow_setup_0387(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x280),
            esp,
            ecx: n + 0xba0,
            edx: s,
            ..Frame::default()
        },
    );
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(ret.eax)
}

/// [`laser_setup_0387`] (the `rel32`s: the strip record, the sprite
/// record, the glow set-up).
pub const LASER_SETUP_0387: Sig = Sig {
    len: 661,
    hash: 0x7845_a0c9_06cc_3e35,
    calls: &[
        (0x69, Callee::Sig(&crate::m0387::STRIP_RECORD)),
        (0x11b, Callee::Sig(&crate::m0387::SPRITE_RECORD)),
        (0x280, Callee::Sig(&GLOW_SETUP_0387)),
    ],
    imports: &[],
};

/// `magic_0387`'s lasers set-up (0x1000d000, 786 bytes; internal, `ecx`
/// = the task `N`, `edx` = the effect `E`, stack `(a0, a1, a2)`; the
/// records `R` at `ebp - 0x10` and `R2` at `ebp - 0x24`): `N` is linked
/// as a task (callback the lasers task at 0x1000c980); under the root
/// `G0`: `B = N + 0x2c` (slot 325 `(N + 0x48, a0)`), `C = N + 0x160`
/// ([`time_left`](crate::m0042::time_left) of `E` into `N + 0x17c`), `D
/// = N + 0x1b8` (`D[0x13c..0x144] = (B, K_P, C)`, `D[0x128]` = `K_O`,
/// byte `D[0x148]` 0) and `F = N + 0x18c` (`N[0x1a8..0x1b0] = (0,
/// 0x4000, 0)`); the bone node `N + 0x6e8`
/// ([`node_init_big`](crate::m0070::node_init_big) with `(G0, K_A, F,
/// K_P)`, class `K_B`, `N[0x838]` = 0x6f, `N[0x834]` = `E[0x54]`); the
/// two lasers at `N + 0xb80` and `N + 0x4b90` ([`laser_setup_0387`]
/// with `D` and the bone node), each followed by its own constants;
/// `N[0x28] = a2`, `N[0x24] = a1`, the state `N[0x18]` 0. Returns `a1`.
pub fn lasers_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, e) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (r, r2) = (ebp.wrapping_sub(0x10), ebp.wrapping_sub(0x24));
    task_link(g, img, f, n, [0x15, 0x1c, 0x24, 0x2e, 0x33, 0x3f, 0x46]);
    let root = w(0x59);
    let b = n + 0x2c;
    root_append(g, b, root, w(0xa1));
    let a0 = fr.arg(g, 0);
    g.call(slot_target(g, w(0x95), 325), 0, &[n + 0x48, a0]);
    let c = n + 0x160;
    root_append(g, c, root, w(0xb8));
    g.write::<u32>(r, e);
    g.write::<u32>(r + 4, n + 0x17c);
    crate::m0042::time_left(
        g,
        img,
        img.rel32_target(f + 0x112),
        r,
        ebp.wrapping_sub(0x38),
    );
    let d = n + 0x1b8;
    root_append(g, d, root, w(0x11e));
    g.write::<u32>(d + 0x13c, b);
    g.write::<u32>(d + 0x140, w(0x174));
    g.write::<u32>(d + 0x128, w(0x17e));
    g.write::<u8>(d + 0x148, 0);
    g.write::<u32>(d + 0x144, c);
    let fv = n + 0x18c;
    root_append(g, fv, root, w(0x19d));
    let bone = n + 0x6e8;
    for (off, v) in [(0x1a8, 0), (0x1ac, 0x4000), (0x1b0, 0)] {
        g.write::<u32>(n + off, v);
    }
    for (i, v) in [bone, w(0x217), w(0x21e), fv, w(0x225)]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(r2 + 4 * i as u32, v);
    }
    crate::m0070::node_init_big(g, r2);
    g.write::<u32>(bone, w(0x233));
    g.write::<u32>(n + 0x838, 0x6f);
    let x = g.read::<u32>(e + 0x54);
    g.write::<u32>(n + 0x834, x);
    let esp = ebp.wrapping_sub(0x3c);
    let laser = |g: &mut G, at: u32, ecx: u32| {
        g.write::<u32>(esp + 4, bone);
        laser_setup_0387(
            g,
            img,
            &Frame {
                at: img.rel32_target(f + at),
                esp,
                ecx,
                edx: d,
                ..Frame::default()
            },
        );
    };
    laser(g, 0x254, n + 0xb80);
    for (off, v) in [
        (0x125c, 0xc270_0000),
        (0x1260, 0),
        (0x1264, 0x41f0_0000),
        (0x126c, 0x41a0_0000),
        (0x49c0, 0x4118_0000),
        (0x49c4, ONE),
        (0x49c8, 0xc128_0000),
    ] {
        g.write::<u32>(n + off, v);
    }
    laser(g, 0x2ab, n + 0x4b90);
    let a2 = fr.arg(g, 2);
    g.write::<u32>(n + 0x28, a2);
    let a1 = fr.arg(g, 1);
    for (off, v) in [
        (0x526c, 0xc270_0000),
        (0x5270, 0),
        (0x5274, 0xc1f0_0000),
        (0x527c, 0x41a0_0000),
        (0x89d0, 0xc118_0000),
        (0x89d4, ONE),
        (0x89d8, 0xc128_0000),
    ] {
        g.write::<u32>(n + off, v);
    }
    g.write::<u32>(n + 0x24, a1);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(a1)
}

/// [`lasers_setup_0387`] (the `rel32`s: the time left, the big node init,
/// the two laser set-ups).
pub const LASERS_SETUP_0387: Sig = Sig {
    len: 786,
    hash: 0xe5f5_439d_8ea8_d0e3,
    calls: &[
        (0x112, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x22a, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0x254, Callee::Sig(&LASER_SETUP_0387)),
        (0x2ab, Callee::Sig(&LASER_SETUP_0387)),
    ],
    imports: &[],
};

/// `magic_0387`'s lasers task (0x1000c980, 1652 bytes; cdecl `(T)`, `N
/// = T[4]`, `O = N[0x28]` the owner; an aligned frame `A`, `S = A -
/// 0x40`: the record `R` at `S + 0x14`, `V` at `S + 0x2c`). By the state
/// `N[0x18]`, the wait `N[0x1c]` down by one at every return:
/// - 0: once both lasers' `+0x18` (`N[0xb98]`, `N[0x4ba8]`) are above
///   0x10, state 0x10;
/// - 0x10: the step `N[0x20]` 0, the wait `W[0]`, state 0x11 (run on);
/// - 0x11 (once the wait is out), step `i`: `X = N + 0x2c (i + 2)` under
///   the root `G0`, slot 229 `(N + 0x74 + 0x2c i, N + 0x254, (T[i], 0, 0,
///   1))`; `Y = N + 0x2c (i + 5)` under `G0` (colour 1, 1, 1 and an alpha
///   0.48 from the third step on, else `(frnd K1 + 0) K2`);
///   `E = N + 0x304 + 0x14c i` under `G0` (`E[0x13c] = X`, `E[0x140] =
///   Y`, `E[0x144]` = `K_T`, `E[0x128]` = `K_O`); the particle node from
///   `R = (N + 0x840 + 0x100 i, K_L, E, *G_P, K_P, 0)`; the glow `P =
///   N[0x24]` takes the alpha (`P[0x1c..0x2c]`, byte `P[0x30] |= 1`); the
///   first step also makes the effect node `N + 0xb40` (`R = (.., G0,
///   K_O, N + 0x2c, K_E, 2)`); the step goes up: after three, state 0x12 and the wait 5,
///   else slot 388 `(part, TBL[frnd & 1], 1)` for each of `O`'s parts and
///   the wait `W[i + 1]`;
/// - 0x12: once the wait is out, slot 239 `(*G_E, part, 0)` for each
///   part, state 0x1000 (run on);
/// - 0x1000: once `N[0x4ba8] & N[0xb98]` has bit 0x2000: five nodes leave,
///   `T[0]` is unlinked, state 0x2000.
///
/// Returns the last value in `eax` (the cookie word `*G_COOKIE ^ (A -
/// 0x34)` where nothing else was loaded).
#[allow(clippy::too_many_lines)] // the states
pub fn lasers_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x40);
    let (r, v) = (s0 + 0x14, s0 + 0x2c);
    let cookie = g.read::<u32>(w(0xa)) ^ a.wrapping_sub(0x34);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(n + 0x1c);
        g.write::<u32>(n + 0x1c, c.wrapping_sub(1));
        Ret::eax(eax)
    };
    let wait_out = |g: &G| g.read::<u32>(n + 0x1c).cast_signed() <= 0;
    let parts = |g: &G| g.read::<u32>(g.read::<u32>(n + 0x28) + 0x4dc).cast_signed();
    let mut eax = cookie;
    let mut state = g.read::<u32>(n + 0x18);
    loop {
        match state {
            0 => {
                if g.read::<u32>(n + 0xb98) > 0x10 && g.read::<u32>(n + 0x4ba8) > 0x10 {
                    g.write::<u32>(n + 0x18, 0x10);
                }
                return end(g, eax);
            }
            0x10 | 0x11 => {
                if state == 0x10 {
                    g.write::<u32>(n + 0x20, 0);
                    let w0 = g.read::<u32>(w(0x25b));
                    g.write::<u32>(n + 0x1c, w0);
                    g.write::<u32>(n + 0x18, 0x11);
                    eax = 0x11;
                }
                if !wait_out(g) {
                    return end(g, eax);
                }
                let root = w(0x28c);
                let i = g.read::<u32>(n + 0x20);
                let x = n + 0x2c * (i + 2);
                root_append(g, x, root, w(0x2fb));
                let i = g.read::<u32>(n + 0x20);
                g.write::<u32>(v + 4, 0);
                let t0 = g.read::<u32>(w(0x30f) + 4 * i);
                g.write::<u32>(v, t0);
                g.write::<u32>(v + 8, 0);
                g.write::<u32>(v + 0xc, ONE);
                let i = g.read::<u32>(n + 0x20);
                let s229 = slot_target(g, w(0x342), 229);
                g.call(s229, 0, &[n + 0x74 + 0x2c * i, n + 0x254, v]);
                let i = g.read::<u32>(n + 0x20);
                let y = n + 0x2c * (i + 5);
                root_append(g, y, root, w(0x3a4));
                for off in [0xf8, 0xfc, 0x100] {
                    let i = g.read::<u32>(n + 0x20);
                    g.write::<u32>(n + off + 0x2c * i, ONE);
                }
                let i = g.read::<u32>(n + 0x20);
                if i.cast_signed() >= 2 {
                    g.write::<u32>(n + 0x104 + 0x2c * i, 0x3ef5_c290);
                } else {
                    let x = st0_f32(g.call(slot_target(g, w(0x3f4), 307), 0, &[]).st0);
                    let i = g.read::<u32>(n + 0x20);
                    let al = mul(add(mul(x, k(g, 0x40e)), 0.0), k(g, 0x423));
                    g.write::<f32>(n + 0x104 + 0x2c * i, al);
                }
                let i = g.read::<u32>(n + 0x20);
                let e = n + 0x304 + 0x14c * i;
                let (xx, yy) = (n + 0x2c * (i + 2), n + 0x2c * (i + 5));
                root_append(g, e, root, w(0x453));
                g.write::<u32>(e + 0x140, yy);
                g.write::<u32>(e + 0x13c, xx);
                g.write::<u32>(e + 0x144, w(0x4b1));
                g.write::<u32>(e + 0x128, w(0x4bb));
                g.write::<u8>(e + 0x148, 0);
                let i = g.read::<u32>(n + 0x20);
                let pn = n + 0x840 + 0x100 * i;
                let pp = g.read::<u32>(w(0x4e9));
                g.write::<u32>(pn, w(0x513));
                for (j, x) in [pn, w(0x4f9), n + 0x304 + 0x14c * i, pp, w(0x505), 0]
                    .into_iter()
                    .enumerate()
                {
                    g.write::<u32>(r + 4 * j as u32, x);
                }
                crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x518), r);
                let i = g.read::<u32>(n + 0x20);
                let al = g.read::<f32>(n + 0x104 + 0x2c * i);
                g.write::<f32>(v, al);
                let p = g.read::<u32>(n + 0x24);
                let a2 = mul(al, k(g, 0x53b));
                let b = g.read::<u8>(p + 0x30) | 1;
                g.write::<u8>(p + 0x30, b);
                let a0 = mul(al, k(g, 0x54a));
                let a1 = mul(al, k(g, 0x555));
                let a3 = mul(al, k(g, 0x55d));
                g.write::<f32>(p + 0x1c, a2);
                g.write::<f32>(p + 0x24, a1);
                g.write::<u32>(p + 0x2c, 0x41c8_0000);
                g.write::<f32>(p + 0x20, sub(a0, a2));
                g.write::<f32>(p + 0x28, sub(a3, a1));
                eax = p;
                if g.read::<u32>(n + 0x20) == 0 {
                    let en = n + 0xb40;
                    for (j, x) in [en, w(0x59f), w(0x5a7), n + 0x2c, w(0x5b3), 2]
                        .into_iter()
                        .enumerate()
                    {
                        g.write::<u32>(r + 4 * j as u32, x);
                    }
                    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x5c0), r);
                    let node = g.read::<u32>(r);
                    g.write::<u32>(node, w(0x5ca));
                    let x = g.read::<u32>(en + 0x2c) | 1;
                    g.write::<u32>(en + 0x2c, x);
                    g.write::<u32>(en + 0x30, 0);
                    eax = node;
                }
                let i = g.read::<u32>(n + 0x20).wrapping_add(1);
                g.write::<u32>(n + 0x20, i);
                if i.cast_signed() >= 3 {
                    let st = g.read::<u32>(n + 0x18).wrapping_add(1);
                    g.write::<u32>(n + 0x18, st);
                    g.write::<u32>(n + 0x1c, 5);
                    return end(g, eax);
                }
                let mut j = 0u32;
                while j.cast_signed() < parts(g) {
                    let bit = g.call(slot_target(g, w(0x611), 324), 0, &[]).eax & 1;
                    let t = g.read::<u32>(w(0x625).wrapping_add(4 * bit));
                    let o = g.read::<u32>(n + 0x28);
                    let part = g.read::<u32>(o + 0x6c + 0x98 * j);
                    g.call(slot_target(g, w(0x631), 388), 0, &[part, t, 1]);
                    j += 1;
                }
                let i = g.read::<u32>(n + 0x20);
                let wt = g.read::<u32>(w(0x658).wrapping_add(4 * i));
                g.write::<u32>(n + 0x1c, wt);
                return end(g, wt);
            }
            0x12 => {
                if !wait_out(g) {
                    return end(g, eax);
                }
                let mut j = 0u32;
                eax = g.read::<u32>(n + 0x28);
                while j.cast_signed() < parts(g) {
                    let o = g.read::<u32>(n + 0x28);
                    let part = g.read::<u32>(o + 0x6c + 0x98 * j);
                    let e = g.read::<u32>(w(0x60));
                    g.call(slot_target(g, w(0x5a), 239), 0, &[e, part, 0]);
                    eax = g.read::<u32>(n + 0x28);
                    j += 1;
                }
                g.write::<u32>(n + 0x18, 0x1000);
                state = 0x1000;
            }
            0x1000 => {
                let gate = g.read::<u32>(n + 0x4ba8) & g.read::<u32>(n + 0xb98);
                if gate & 0x2000 == 0 {
                    return end(g, gate);
                }
                for off in [0x6e8, 0x18c, 0x1b8, 0x2c, 0x160] {
                    crate::sa_misc::leave(g, n + off, 0);
                }
                let eax = unlink_task(g, tk);
                let c = g.read::<u32>(n + 0x1c);
                g.write::<u32>(n + 0x1c, c.wrapping_sub(1));
                g.write::<u32>(n + 0x18, 0x2000);
                return Ret::eax(eax);
            }
            _ => return end(g, eax),
        }
    }
}

/// [`lasers_task_0387`] (the `rel32`s: the particle node, the effect
/// node, the cookie checks).
pub const LASERS_TASK_0387: Sig = Sig {
    len: 1652,
    hash: 0xa72a_7e7b_8b08_e44c,
    calls: &[
        (0x23c, Callee::Any),
        (0x2e1, Callee::Any),
        (0x518, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5c0, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x5f9, Callee::Any),
        (0x66c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387`'s shots set-up (0x10009820, 1024 bytes; internal, `ecx`
/// = the task `N`, `edx` = the effect `E`, stack `(a0, a1)`; the records
/// `R` at `ebp - 0x10` and `R2` at `ebp - 0x30`): `N` is linked as a task
/// (callback [`shots_task_0387`]); under the root `G0`: `A = N + 0x9698`
/// ([`time_left`](crate::m0042::time_left) of `E` into `N + 0x96b4`) and
/// `B = N + 0x9a90` (`B[0x13c] = a1`, `B[0x144] = A`, `B[0x140]` =
/// `K_P`, `B[0x128]` = `K_O`); for both rows `j`: `X = N + 0x971c + 0x2c
/// j` (`X[0x1c..0x28] = (T1[j], -33, 0, 1)`), `Y = N + 0x9640 + 0x2c j`
/// (`Y[0x1c..0x24] = (-0x700, 0, T2[j])`), `Z = N + 0x96c4 + 0x2c j`
/// (colour `(1, 1, 1, 0.22)`) and `W = N + 0x97f8 + 0x14c j` (`W[0x13c..
/// 0x144] = (X, Z, Y)`, `W[0x128] = B`) under `G0`, and the particle
/// node from `R2 = (N + 0x9be0 + 0x100 j, K_L, W, *G_P, K_T, 4)`; the
/// row counter is the argument word `a1` (the stack above the entry:
/// 4, then 8). The eight shots' states 0x2000, `N[0x30] = a0`, `N[0x2c]
/// = E`, `N[0x24]` 0, the wait `N[0x1c]` 20, the state 0. Returns `E`.
pub fn shots_setup_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (n, e) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (r, r2) = (ebp.wrapping_sub(0x10), ebp.wrapping_sub(0x30));
    task_link(g, img, f, n, [0x19, 0x20, 0x28, 0x32, 0x37, 0x46, 0x4d]);
    let root = w(0x66);
    let a = n + 0x9698;
    root_append(g, a, root, w(0x53));
    g.write::<u32>(r, e);
    g.write::<u32>(r + 4, n + 0x96b4);
    crate::m0042::time_left(
        g,
        img,
        img.rel32_target(f + 0xad),
        r,
        ebp.wrapping_sub(0x44),
    );
    let b = n + 0x9a90;
    root_append(g, b, root, w(0xbc));
    let a1 = fr.arg(g, 1);
    g.write::<u32>(b + 0x13c, a1);
    g.write::<u32>(b + 0x144, a);
    g.write::<u32>(b + 0x140, w(0x12f));
    g.write::<u32>(b + 0x128, w(0x139));
    g.write::<u8>(b + 0x148, 0);
    g.write::<u32>(fr.esp + 8, 0);
    for j in 0..2u32 {
        let x = n + 0x971c + 0x2c * j;
        root_append(g, x, root, w(0x19a));
        let t1 = g.read::<u32>(w(0x1a0) + 4 * j);
        for (off, v) in [(0x1c, t1), (0x20, 0xc204_0000), (0x24, 0), (0x28, ONE)] {
            g.write::<u32>(x + off, v);
        }
        let y = n + 0x9640 + 0x2c * j;
        root_append(g, y, root, w(0x1cd));
        g.write::<u32>(y + 0x1c, 0xffff_f900);
        g.write::<u32>(y + 0x20, 0);
        let t2 = g.read::<u32>(w(0x239) + 4 * j);
        g.write::<u32>(y + 0x24, t2);
        let z = n + 0x96c4 + 0x2c * j;
        root_append(g, z, root, w(0x298));
        for (off, v) in [(0x1c, ONE), (0x20, ONE), (0x24, ONE), (0x28, 0x3e61_47ae)] {
            g.write::<u32>(z + off, v);
        }
        let wn = n + 0x97f8 + 0x14c * j;
        root_append(g, wn, root, w(0x2bf));
        g.write::<u32>(wn + 0x13c, x);
        g.write::<u32>(wn + 0x140, z);
        g.write::<u32>(wn + 0x144, y);
        g.write::<u32>(wn + 0x128, b);
        g.write::<u8>(wn + 0x148, 0);
        let p = n + 0x9be0 + 0x100 * j;
        let pp = g.read::<u32>(w(0x339));
        g.write::<u32>(p, w(0x342));
        for (i, v) in [p, w(0x34c), wn, pp, w(0x356), 4].into_iter().enumerate() {
            g.write::<u32>(r2 + 4 * i as u32, v);
        }
        crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x362), r2);
        g.write::<u32>(fr.esp + 8, 4 * (j + 1));
    }
    for i in 0..8 {
        g.write::<u32>(n + 0x58 + 0x12c0 * i, 0x2000);
    }
    let a0 = fr.arg(g, 0);
    g.write::<u32>(n + 0x30, a0);
    g.write::<u32>(n + 0x2c, e);
    g.write::<u32>(n + 0x24, 0);
    g.write::<u32>(n + 0x1c, 0x14);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(e)
}

/// [`shots_setup_0387`] (the `rel32`s: the time left, the particle node).
pub const SHOTS_SETUP_0387: Sig = Sig {
    len: 1024,
    hash: 0x9469_17f5_2741_ef2f,
    calls: &[
        (0xad, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x362, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `out = k - *src` (`subss`, the constant first) for the globals at the
/// operands `(src, out)` of `f`, in order.
fn global_rests<G: Game>(g: &mut G, img: &Image, f: u32, k: f32, ops: &[(u32, u32)]) {
    for &(src, out) in ops {
        let x = sub(k, g.read::<f32>(img.word(f + src)));
        g.write::<f32>(img.word(f + out), x);
    }
}

/// The word `v` into each global at the operands `ops` of `f`.
fn global_words<G: Game>(g: &mut G, img: &Image, f: u32, v: u32, ops: &[u32]) {
    for &o in ops {
        g.write::<u32>(img.word(f + o), v);
    }
}

/// `magic_0387`'s main task (0x1000d320, 1616 bytes; cdecl `(T)`, `N =
/// T[4]`; an aligned frame `A`, `S = A - 0x50`: the records at `S + 0xc`,
/// `S + 0x18`, `S + 0x28`, the centre `V` at `S + 0x3c`). The effect's
/// node `G_FX` and its globals are the DLL's. By the state `N[0x18]`,
/// the clock `N[0x1c]` up and the count `N[0x20]` down at every return:
/// - 0: `G_FX` becomes a model node under the root `G0`
///   ([`model_node_init`](crate::sa_summon::model_node_init)), `N + 0x24`
///   the owner's model nodes ([`model_nodes`](crate::s3_summon::model_nodes)),
///   `L = N + 0x514` joins the list `G_L`, the colour node at `G_C`
///   ([`colour_node`](crate::s3_misc::colour_node)); then `G_FX`'s colour
///   words (`K1 - x` and 0.5 runs, [`global_rests`]); `B = N + 0x580`
///   under `G0` (`B[0x3c] = *G_B`, `B[0x40]` 0x70, its vectors 1 and 0),
///   the effect node `P = N + 0x548` under `G0` (slot 719 `(K_E, 6, 0)`)
///   and the task `Q = N + 0x5c4` (callback 0x100088d0, `Q[0x3c] =
///   G_FX`); state 10 (run on);
/// - 10: at clock 0x28, the shots ([`shots_setup_0387`] at `N + 0x610`
///   with `(N + 0x24, B)`), state 11;
/// - 11: at clock 0x34, state 30 (run on);
/// - 30: at clock 0x66, the pulses ([`pulse_setup_0387`] at `N + 0xa4c0`
///   with `(N + 0x24, L)`) and their weights
///   ([`weights_task_0387`](crate::r45::weights_task_0387) at `N +
///   0x12d20`), state 20 (run on);
/// - 20: at clock 0x12a, the lasers ([`lasers_setup_0387`] at `N +
///   0x12d70` with the targets' centre
///   ([`centre`](crate::m0577::centre)), `L` and `N + 0x24`), state 21,
///   the count 0x28;
/// - 21: once the count is out, `G_FX`'s colour words again (`K - x` and
///   1.0 runs, the flags word's bit 2 set and bit 0 cleared), state 10000
///   (run on);
/// - 10000: once `Q`'s state is 20000: `L` and `N + 0x24` leave, slot 250
///   `(0, 0)`, the list's current node is unlinked, `G_FX`'s model is
///   set up ([`model_setup`](crate::sa_summon::model_setup)) and `T[0]` is
///   unlinked from its ring.
///
/// Returns the last value in `eax`.
#[allow(clippy::too_many_lines)] // the states
pub fn main_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x50);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(n + 0x1c).wrapping_add(1);
        g.write::<u32>(n + 0x1c, c);
        let c = g.read::<u32>(n + 0x20).wrapping_sub(1);
        g.write::<u32>(n + 0x20, c);
        Ret::eax(eax)
    };
    let clock = |g: &G| g.read::<u32>(n + 0x1c).cast_signed();
    let k = g.read::<f32>(w(0x25));
    loop {
        let state = g.read::<u32>(n + 0x18);
        match state {
            0 => {
                let fx = w(0x77);
                let e = g.read::<u32>(w(0x6b));
                let r = s0 + 0x18;
                for (i, x) in [fx, w(0x7f), e].into_iter().enumerate() {
                    g.write::<u32>(r + 4 * i as u32, x);
                }
                g.write::<u8>(r + 0xc, 0);
                g.write::<u32>(w(0x8e), w(0x92));
                crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x97), r);
                let e = g.read::<u32>(w(0x9c));
                let r2 = s0 + 0x28;
                let m = n + 0x24;
                g.write::<u32>(r2, m);
                g.write::<u32>(m, w(0xa9));
                g.write::<u32>(r2 + 4, w(0xb5));
                g.write::<u32>(r2 + 8, e);
                g.write::<u32>(r2 + 0xc, 0);
                g.write::<u8>(r2 + 0x10, 0);
                let fr2 = Frame {
                    at: img.rel32_target(f + 0xcb),
                    esp: s0.wrapping_sub(4),
                    ecx: r2,
                    ..Frame::default()
                };
                crate::s3_summon::model_nodes(g, img, &fr2);
                let l = n + 0x514;
                let gl = w(0xea);
                g.write::<u32>(l, w(0xd7));
                crate::l3_common::node_init(g, l, gl);
                crate::l3_common::attach(g, l, gl, crate::l3_common::ListAt::of(gl));
                g.write::<u8>(l + 0x30, 0);
                let r3 = s0 + 0xc;
                g.write::<u32>(r3, w(0x12d));
                g.write::<u32>(r3 + 4, w(0x135));
                g.write::<u32>(w(0x13b), w(0x13f));
                let fr3 = Frame {
                    at: img.rel32_target(f + 0x144),
                    esp: s0.wrapping_sub(4),
                    ecx: r3,
                    ..Frame::default()
                };
                crate::s3_misc::colour_node(g, img, &fr3);
                let k1 = g.read::<f32>(w(0x14c));
                global_rests(g, img, f, k1, &[(0x157, 0x17c)]);
                let fl = g.read::<u32>(w(0x15d)) & !1;
                g.write::<u32>(w(0x15d), fl);
                let gb = g.read::<u32>(w(0x164));
                let half = 0x3f00_0000;
                global_words(g, img, f, half, &[0x170]);
                global_rests(g, img, f, k1, &[(0x187, 0x1b7)]);
                global_words(g, img, f, half, &[0x18d, 0x197, 0x1a1, 0x1ab]);
                global_rests(g, img, f, k1, &[(0x1c2, 0x1f2)]);
                global_words(g, img, f, half, &[0x1c8, 0x1d2]);
                global_words(g, img, f, 0x3cf5_c28f, &[0x1dc]);
                global_words(g, img, f, 0, &[0x1e6]);
                global_rests(
                    g,
                    img,
                    f,
                    k1,
                    &[(0x1fd, 0x205), (0x210, 0x218), (0x223, 0x22b)],
                );
                let x = sub(g.read::<f32>(w(0x233)), g.read::<f32>(w(0x23b)));
                g.write::<f32>(w(0x243), x);
                global_rests(g, img, f, k1, &[(0x24e, 0x256)]);
                let b = n + 0x580;
                root_append(g, b, w(0x269), w(0x2cf));
                g.write::<u32>(b + 0x28, ONE);
                g.write::<u32>(b + 0x3c, gb);
                g.write::<u32>(b + 0x40, 0x70);
                for off in [0x2c, 0x30, 0x34, 0x38] {
                    g.write::<u32>(b + off, 0);
                }
                let p = n + 0x548;
                root_append(g, p, w(0x2e8), w(0x359));
                let e = g
                    .call(slot_target(g, w(0x313), 719), 0, &[w(0x32f), 6, 0])
                    .eax;
                g.write::<u32>(p + 0x24, e);
                g.write::<u32>(p + 0x1c, w(0x33b));
                g.write::<u32>(p + 0x20, b);
                g.write::<u32>(p + 0x34, 0x40);
                g.write::<u32>(p + 0x28, 0x100);
                g.write::<u32>(p + 0x2c, 1);
                g.write::<u32>(p + 0x30, 0);
                let q = n + 0x5c4;
                task_link(
                    g,
                    img,
                    f,
                    q,
                    [0x373, 0x380, 0x388, 0x392, 0x397, 0x3a0, 0x3a7],
                );
                g.write::<u32>(q + 0x3c, w(0x3ae));
                g.write::<u32>(q + 0x30, 0);
                g.write::<u32>(q + 0x18, 0);
                g.write::<u32>(n + 0x18, 10);
            }
            10 => {
                if clock(g) < 0x28 {
                    return end(g, 0);
                }
                let esp = s0.wrapping_sub(0xc);
                g.write::<u32>(esp + 4, n + 0x24);
                g.write::<u32>(esp + 8, n + 0x580);
                let r = shots_setup_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x563),
                        esp,
                        ecx: n + 0x610,
                        edx: w(0x55e),
                        ..Frame::default()
                    },
                );
                g.write::<u32>(n + 0x18, 11);
                return end(g, r.eax);
            }
            11 => {
                if clock(g) < 0x34 {
                    return end(g, 0);
                }
                g.write::<u32>(n + 0x18, 0x1e);
            }
            0x1e => {
                if clock(g) < 0x66 {
                    return end(g, 0x1e);
                }
                let esp = s0.wrapping_sub(0xc);
                g.write::<u32>(esp + 4, n + 0x24);
                g.write::<u32>(esp + 8, n + 0x514);
                pulse_setup_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x3fb),
                        esp,
                        ecx: n + 0xa4c0,
                        edx: w(0x3f6),
                        ..Frame::default()
                    },
                );
                crate::r45::weights_task_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x40b),
                        esp: s0.wrapping_sub(4),
                        ecx: n + 0x12d20,
                        edx: n + 0x24,
                        ..Frame::default()
                    },
                );
                g.write::<u32>(n + 0x18, 0x14);
            }
            0x14 => {
                if clock(g) < 0x12a {
                    return end(g, 0x14);
                }
                let v = s0 + 0x3c;
                let fc = Frame {
                    at: img.rel32_target(f + 0x589),
                    esp: s0.wrapping_sub(4),
                    ecx: v,
                    edx: w(0x580),
                    ..Frame::default()
                };
                crate::m0577::centre(g, img, &fc);
                let esp = s0.wrapping_sub(0x10);
                for (i, x) in [v, n + 0x514, n + 0x24].into_iter().enumerate() {
                    g.write::<u32>(esp + 4 + 4 * i as u32, x);
                }
                let r = lasers_setup_0387(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x5a9),
                        esp,
                        ecx: n + 0x12d70,
                        edx: w(0x5a4),
                        ..Frame::default()
                    },
                );
                g.write::<u32>(n + 0x18, 0x15);
                g.write::<u32>(n + 0x20, 0x28);
                return end(g, r.eax);
            }
            0x15 => {
                if g.read::<u32>(n + 0x20).cast_signed() > 0 {
                    return end(g, 0x15);
                }
                let fl = (g.read::<u32>(w(0x431)) & !1) | 4;
                global_words(g, img, f, ONE, &[0x43d]);
                global_rests(g, img, f, k, &[(0x42c, 0x449)]);
                global_rests(g, img, f, k, &[(0x454, 0x484)]);
                global_words(g, img, f, ONE, &[0x45a, 0x464, 0x46e, 0x478]);
                global_rests(g, img, f, k, &[(0x48f, 0x4bf)]);
                global_words(g, img, f, ONE, &[0x495, 0x49f]);
                global_words(g, img, f, 0x3ca3_d70a, &[0x4a9]);
                global_words(g, img, f, 0, &[0x4b3]);
                let x = sub(k, g.read::<f32>(w(0x4ca)));
                g.write::<u32>(w(0x4cf), fl);
                global_words(g, img, f, ONE, &[0x4d5]);
                g.write::<f32>(w(0x4e1), x);
                global_rests(g, img, f, k, &[(0x4ec, 0x4f4), (0x4ff, 0x507)]);
                let x = sub(g.read::<f32>(w(0x50f)), g.read::<f32>(w(0x517)));
                g.write::<f32>(w(0x51f), x);
                global_rests(g, img, f, k, &[(0x52a, 0x532)]);
                g.write::<u32>(n + 0x18, 10000);
            }
            10000 => {
                if g.read::<u32>(n + 0x5dc) != 20000 {
                    return end(g, 10000);
                }
                crate::sa_misc::leave(g, n + 0x514, 0);
                let e = g.call(slot_target(g, w(0x5db), 250), 0, &[0, 0]).eax;
                let eax = crate::sa_misc::leave(g, n + 0x24, e);
                crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0x605), eax);
                let r = s0 + 0xc;
                g.write::<u32>(r, w(0x611));
                crate::sa_summon::model_setup(
                    g,
                    img,
                    &Frame {
                        at: img.rel32_target(f + 0x616),
                        esp: s0.wrapping_sub(4),
                        ecx: r,
                        ..Frame::default()
                    },
                );
                let eax = unlink_task(g, tk);
                return end(g, eax);
            }
            s if s > 0x14 => return end(g, s),
            s => return end(g, s.wrapping_sub(11)),
        }
    }
}

/// [`main_task_0387`] (the `rel32`s: the model node init, the model nodes,
/// the colour node, the pulse set-up, the weights task, the shots set-up,
/// the centre, the lasers set-up, two node leaves, the list unlink, the
/// model set-up, the cookie check).
pub const MAIN_TASK_0387: Sig = Sig {
    len: 1616,
    hash: 0x6c69_c71a_c4de_79de,
    calls: &[
        (0x97, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xcb, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x144, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x3fb, Callee::Sig(&PULSE_SETUP_0387)),
        (0x40b, Callee::Sig(&crate::r45::WEIGHTS_TASK_0387)),
        (0x563, Callee::Sig(&SHOTS_SETUP_0387)),
        (0x589, Callee::Sig(&crate::m0577::CENTRE)),
        // the lasers set-up (its own callees nest past `matches`' depth)
        (0x5a9, Callee::Any),
        (0x5d6, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x600, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x605, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x616, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x648, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0397`'s actors' start (0x10005020, 1114 bytes; a table
/// callback, no arguments; an aligned /GS frame `A`, `S = A - 0x48`: the
/// position `(x, y, z)` at `S + 0x34`). As
/// [`spin_start_0307`](crate::r32::spin_start_0307) on 0397's records
/// (`R = *G_S + 0x1e90 + 0xac k`, the centre at `*G_S + 0x1900`): for
/// each actor `k < *G_N` and its object `O`:
/// - slot 262 `(O, &x, &y, &z)` (`w` 1.0); slot 12 `(O)`'s height `h`
///   and slot 314 `(5.0)`'s `r` (ST0); `R[0x74] = y`,
///   `u = ((y - h) + (r + C)) / D`;
/// - the distance `d` from the centre (the height difference `c.y - c.y`)
///   to `R[0x9c]`, `R[0xa0]` 80; the yaw `K - atan2(dz, dx)` (wrapped
///   once) to `R[0x90]`, `R[0x94..0x98] = (0, K_S)`; `R[0x40..0x60]` zeros
///   but `R[0x44] = u C1` and `R[0x54] = u C2 / D2`; slot 333 `(O)`'s
///   angle `K2 - a` wrapped to `R[0x64]`; zeros, three scales 1.0, `R[0xa4]`
///   500 and `R[0xa8]` 0;
/// - unless slot 532 `(LIST[k])` answers: the actor's mode
///   ([`actor_mode`](crate::sa_script::actor_mode), `ecx = k`, `edx = R`),
///   slot 295 `(LIST[k], 0)` and slot 528 `(LIST[k], 0)`.
///
/// Then a task node (callback the actors' sway) is taken from the free
/// ring ([`pool_take_0587`](crate::r27::pool_take_0587)), flagged and
/// goes last on the task list, also kept at `*G_S + 0x23f0`. Returns
/// `*G_S`.
#[allow(clippy::too_many_lines)] // the record's fields, as the original
pub fn actors_start_0397<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::wrap_once;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let a = fr.esp.wrapping_sub(4) & !7;
    let s0 = a.wrapping_sub(0x48);
    let (px, py, pz) = (s0 + 0x34, s0 + 0x38, s0 + 0x3c);
    let s = g.read::<u32>(w(0x15));
    g.write::<u32>(s0 + 0x14, s);
    let n = g.read::<u32>(w(0x1c)).cast_signed();
    // `k` counts up from 0: the inlined "actor -1 is `*G_SELF`" branches
    // never run
    let obj = |g: &G, k: u32, list: u32, table: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(list)).wrapping_add(k)));
        g.read::<u32>(w(table).wrapping_add(4 * i))
    };
    let actor =
        |g: &G, k: u32, list: u32| u32::from(g.read::<u8>(g.read::<u32>(w(list)).wrapping_add(k)));
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(0x1e90 + 0xac * k);
        let o = obj(g, k, 0x43, 0x4e);
        g.call(t(g, 0x63, 262), 0, &[o, px, py, pz]);
        g.write::<u32>(s0 + 0x40, ONE);
        let o = obj(g, k, 0x87, 0x92);
        let h = st0_f32(g.call(t(g, 0x98, 12), 0, &[o]).st0);
        g.write::<f32>(s0 + 0x18, h);
        let y = rf(g, py);
        g.write::<f32>(r + 0x74, y);
        let r5 = st0_f32(g.call(t(g, 0xac, 314), 0, &[0x40a0_0000]).st0);
        g.write::<f32>(s0 + 0x10, r5);
        let lift = add(sub(rf(g, py), h), add(r5, c(g, 0xde)));
        let sy = rf(g, s.wrapping_add(0x1904));
        g.write::<f32>(py, sy);
        let dz = sub(rf(g, pz), rf(g, s.wrapping_add(0x1908)));
        let dy = sub(sy, sy);
        let u = div(lift, c(g, 0x111));
        g.write::<f32>(s0 + 0x10, u);
        let dx = sub(rf(g, px), rf(g, s.wrapping_add(0x1900)));
        let flat = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        #[allow(clippy::cast_possible_truncation)]
        let d = f64::from(flat).sqrt() as f32;
        g.write::<u32>(r + 0xa0, 0x42a0_0000);
        g.write::<f32>(r + 0x9c, d);
        let dz = sub(rf(g, pz), rf(g, s.wrapping_add(0x1908)));
        let dx = sub(rf(g, px), rf(g, s.wrapping_add(0x1900)));
        // `_CIatan2` on x87 (`ST1` = dz), rounded once
        #[allow(clippy::cast_possible_truncation)]
        let an = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = sub(c(g, 0x1a8), an);
        let yaw = wrap_once(g, img, f, yaw, [0x1b7, 0x1c1, 0x1cb, 0x1d8]);
        g.write::<f32>(r + 0x90, yaw);
        let u = rf(g, s0 + 0x10);
        let side = div(mul(u, c(g, 0x1f1)), c(g, 0x201));
        let up = mul(u, c(g, 0x1f9));
        g.write::<u32>(r + 0x94, 0);
        g.write::<u32>(r + 0x98, 0xbbcd_e32f);
        for i in 0..9u32 {
            g.write::<u32>(r + 0x40 + 4 * i, 0);
        }
        g.write::<f32>(r + 0x44, up);
        g.write::<f32>(r + 0x54, side);
        let o = obj(g, k, 0x261, 0x26c);
        let v = st0_f32(g.call(t(g, 0x272, 333), 0, &[o]).st0);
        g.write::<f32>(s0 + 0x10, v);
        let a2 = sub(c(g, 0x286), v);
        let a2 = wrap_once(g, img, f, a2, [0x296, 0x2a0, 0x2aa, 0x2b7]);
        g.write::<f32>(r + 0x64, a2);
        for (off, x) in [
            (0x68u32, 0u32),
            (0x6c, 0),
            (0x80, ONE),
            (0x84, ONE),
            (0x88, ONE),
            (0xa4, 0x1f4),
            (0xa8, 0),
        ] {
            g.write::<u32>(r + off, x);
        }
        let i = actor(g, k, 0x30d);
        if g.call(t(g, 0x317, 532), 0, &[i]).eax != 0 {
            continue;
        }
        let mode = Frame {
            at: img.rel32_target(f + 0x32f),
            esp: s0.wrapping_sub(4),
            ecx: k,
            edx: r,
            ..Frame::default()
        };
        crate::sa_script::actor_mode(g, img, &mode);
        let i = actor(g, k, 0x340);
        g.call(t(g, 0x34c, 295), 0, &[i, 0]);
        let i = actor(g, k, 0x368);
        g.call(t(g, 0x374, 528), 0, &[i, 0]);
    }
    let (node, _) =
        crate::r27::pool_take_0587(g, img, f, [0x395, 0x3a4, 0x3c1, 0x3c9, 0x3fa, 0x405], 0);
    g.write::<u8>(node + 0x18, 1);
    crate::r32::append_last(g, img, f, node, [0x40b, 0x41e, 0x431, 0x447], 0);
    let s = g.read::<u32>(s0 + 0x14);
    g.write::<u32>(s + 0x23f0, node);
    Ret::eax(s)
}

/// [`actors_start_0397`] (the `rel32`s: `_libm_sse2_sqrt_precise`,
/// `_CIatan2`, the actor's mode, `__security_check_cookie`).
pub const ACTORS_START_0397: Sig = Sig {
    len: 1114,
    hash: 0xf90f_55b9_2e87_b419,
    calls: &[
        (0x141, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x196, Callee::Import("_CIatan2")),
        (0x32f, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x452, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0396`'s load step (0x100047d0, the 118-byte shape) running
/// [`start_up_0396`](crate::r45::start_up_0396).
pub fn load_0396<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(
        g,
        img,
        fr,
        crate::r9_load::Shape::Zero,
        crate::r45::start_up_0396,
    )
}

/// [`load_0396`].
pub const LOAD_0396: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::r45::START_UP_0396))],
    imports: &[],
};

/// `magic_0397`'s actors' landing (0x100058d0, 1941 bytes; a table
/// callback, no arguments; a /GS frame: the place `(x, y, z, 1)` at
/// `ebp - 0x14`). With `S = *G_S`: the sway task `S[0x23f0]` goes back to
/// the free ring ([`give_back`](crate::sa_pool::give_back)); then for each
/// actor `k < *G_N` (its record `R = S + 0x1e90 + 0xac k`, `P = R +
/// 0x64`, its object `O`):
/// - slot 262 `(O, &x, &y, &z)`; `h` = slot 12 `(O)` (ST0, over `y`);
///   `P[0xc] = x`, `P[0x10] -= K`, `P[0x14] = z`; `P[-4]`, `P[0]` = slot
///   314 `(PI)` each; `P[4]` 0, `P[0x1c..0x24]` 1.0; the actor's scale
///   into `R` ([`actor_scale`](crate::sa_misc::actor_scale)); slots 330,
///   328, 329 `(R, R, P[4] / P[-4] / P[0])`; `R[0x30..0x38]` =
///   `P[0xc..0x14]`; `P[-0x20] = -((P[0x10] - h) / K2)` among zeros;
///   `P[0x40]` 30, `P[0x44] = c = rand rem 11 + 20`;
/// - unless slot 532 `(LIST[k])` answers: the object `S + 0x130 (k + 13)`
///   starts ([`start_object_owners`](crate::s4_misc::start_object_owners),
///   `K3`, `(2, c + 0x1c, 0, CB)`) at `(x, z, 1)` with `y = h - K4`, its
///   trail `T = S + 0x1cf0 + 0x34 k` reset; in a battle also the object
///   `S + 0x130 (k + 5)` (`K3`, `(4, 0x14, 0, 0)`, its step `CB2`); the
///   ring record `Q` (`S + 0x1b00`, one per such actor) is set up and a
///   task node from the free ring ([`pool_take_0587`](crate::r27::pool_take_0587),
///   kept at its `+0x14`) goes last on the task list; slot 295 `(LIST[k],
///   -1)`;
/// - else the object `S + 0x130 (k + 5)` starts (`K3` in a battle, else
///   0; `(4, 0, 0x14, CB)`) at the actor's place (slot 262) and its
///   matrix (slot 268 `(O, 0x10, V)`).
///
/// Then two task nodes from the free ring: the actors' settle (kept at
/// `S[0x23f0]`) and the landing's light on `W = S + 0x1cb8` (set up with
/// its globals), appended after it. Returns 0xffff (`S` when the ring
/// had no third node).
#[allow(clippy::too_many_lines)] // the original's steps
pub fn landing_0397<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_misc::start_object_owners;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (px, py, pz, pw) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
        ebp.wrapping_sub(8),
    );
    let s = g.read::<u32>(w(0x13));
    let back = g.read::<u32>(s + 0x23f0);
    crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x22), back);
    let n = g.read::<u32>(w(0x27)).cast_signed();
    let actor =
        |g: &G, k: u32, list: u32| u32::from(g.read::<u8>(g.read::<u32>(w(list)).wrapping_add(k)));
    let obj = |g: &G, k: u32, list: u32, table: u32| {
        g.read::<u32>(w(table).wrapping_add(4 * actor(g, k, list)))
    };
    let mut q = s + 0x1b00;
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(0x1e90 + 0xac * k);
        let p = r + 0x64;
        let o = obj(g, k, 0x5d, 0x68);
        g.call(t(g, 0x7d, 262), 0, &[o, px, py, pz]);
        let h = st0_f32(g.call(t(g, 0x8a, 12), 0, &[o]).st0);
        g.write::<f32>(py, h);
        let x = g.read::<f32>(px);
        g.write::<f32>(p + 0xc, x);
        let y = sub(g.read::<f32>(p + 0x10), c(g, 0xaf));
        g.write::<u32>(pw, ONE);
        g.write::<f32>(p + 0x10, y);
        let z = g.read::<f32>(pz);
        g.write::<f32>(p + 0x14, z);
        let pi = 0x4049_0fdb;
        let r1 = st0_f32(g.call(t(g, 0xd4, 314), 0, &[pi]).st0);
        g.write::<f32>(p.wrapping_sub(4), r1);
        let r2 = st0_f32(g.call(t(g, 0xee, 314), 0, &[pi]).st0);
        g.write::<f32>(p, r2);
        g.write::<u32>(p + 4, 0);
        for off in [0x1c, 0x20, 0x24] {
            g.write::<u32>(p + off, ONE);
        }
        crate::sa_misc::actor_scale(g, img, img.rel32_target(f + 0x121), k, r);
        for (op, sl, at) in [(0x138, 330, p + 4), (0x15a, 328, p - 4), (0x17b, 329, p)] {
            let a = g.read::<u32>(at);
            g.call(t(g, op, sl), 0, &[r, r, a]);
        }
        for i in 0..3u32 {
            let v = g.read::<u32>(p + 0xc + 4 * i);
            g.write::<u32>(p - 0x34 + 4 * i, v);
        }
        let d = div(sub(g.read::<f32>(p + 0x10), h), c(g, 0x1c9));
        for off in [0x38u32, 0x3c] {
            g.write::<u32>(p + off, 0);
        }
        for off in [0x24u32, 0x1c, 0x18, 0x14, 0x10, 0xc, 8] {
            g.write::<u32>(p - off, 0);
        }
        let mask = g.read::<u32>(w(0x1d7));
        g.write::<u32>(p - 0x20, d.to_bits() ^ mask);
        let cnt = g
            .rand()
            .cast_signed()
            .wrapping_rem(11)
            .wrapping_add(20)
            .cast_unsigned();
        g.write::<u32>(p + 0x40, 0x1e);
        g.write::<u32>(p + 0x44, cnt);
        let i = actor(g, k, 0x22a);
        let busy = g.call(t(g, 0x234, 532), 0, &[i]).eax;
        if busy == 0 {
            let o1 = s + 0x130 * (k + 0xd);
            start_object_owners(
                g,
                img,
                img.rel32_target(f + 0x272),
                o1,
                c(g, 0x254),
                2,
                cnt + 0x1c,
                (0, w(0x244)),
            );
            let y = sub(h, c(g, 0x2ac));
            g.write::<f32>(o1 + 0xd0, x);
            g.write::<f32>(o1 + 0xd8, z);
            g.write::<u32>(o1 + 0xdc, ONE);
            g.write::<f32>(o1 + 0xd4, y);
            let v = g.read::<u32>(s + 0x23f8);
            g.write::<u32>(o1 + 0xc4, v);
            let i = actor(g, k, 0x2d4);
            g.write::<u32>(o1 + 0x120, i);
            let tr = s + 0x1cf0 + 0x34 * k;
            for off in [0x18u32, 0x14, 0x10, 0x1c] {
                g.write::<u32>(tr + off, 0);
            }
            g.write::<u16>(tr + 0x30, 0);
            g.write::<u32>(tr + 0x20, w(0x31c));
            g.write::<u32>(tr + 0x24, w(0x323));
            g.write::<u32>(tr + 0x28, 0);
            g.write::<u32>(tr + 0x2c, 0);
            g.write::<u32>(o1 + 0x110, tr);
            if g.read::<u32>(w(0x2f2)) != 0 {
                let o2 = s + 0x130 * (k + 5);
                start_object_owners(
                    g,
                    img,
                    img.rel32_target(f + 0x35b),
                    o2,
                    c(g, 0x341),
                    4,
                    0x14,
                    (0, 0),
                );
                let v = g.read::<u32>(s + 0x23f8);
                g.write::<u32>(o2 + 0xc4, v);
                g.write::<u32>(o2 + 0x104, w(0x377));
                g.write::<u32>(o2 + 0x120, k);
            }
            for (off, v) in [
                (0u32, 0u32),
                (0x1c, 0),
                (0x20, 0x3d88_8889),
                (0x24, 0),
                (0x28, 0xf),
            ] {
                g.write::<u32>(q + off, v);
            }
            g.write::<u16>(q + 0x2c, 0);
            let (node, _) = crate::r27::pool_take_0587(
                g,
                img,
                f,
                [0x3af, 0x3bb, 0x3df, 0x3e7, 0x418, 0x423],
                0,
            );
            g.write::<u8>(node + 0x18, 1);
            g.write::<u32>(node + 0x14, q);
            crate::r32::append_last(g, img, f, node, [0x429, 0x43f, 0x452, 0x496], 0);
            for (off, v) in [
                (4u32, 0x4000_0000u32),
                (8, ONE),
                (0xc, 0),
                (0x10, 0x4000_0000),
                (0x14, ONE),
                (0x18, 0),
            ] {
                g.write::<u32>(q + off, v);
            }
            let fl = g.read::<u16>(q + 0x2c) | 3;
            g.write::<u16>(q + 0x2c, fl);
            g.write::<u32>(q + 0x24, cnt + 0x1e);
            q += 0x30;
            let i = actor(g, k, 0x4aa);
            g.call(t(g, 0x4b6, 295), 0, &[i, u32::MAX]);
        } else {
            let o2 = s + 0x130 * (k + 5);
            let k3 = if g.read::<u32>(w(0x4d8)) != 0 {
                c(g, 0x4ee)
            } else {
                0.0
            };
            start_object_owners(
                g,
                img,
                img.rel32_target(f + 0x4f8),
                o2,
                k3,
                4,
                0,
                (0x14, w(0x244)),
            );
            let v = o2 + 0xd0;
            let o = obj(g, k, 0x519, 0x524);
            g.call(t(g, 0x531, 262), 0, &[o, v, v + 4, v + 8]);
            g.call(t(g, 0x546, 268), 0, &[o, 0x10, v]);
            g.write::<u32>(v + 0xc, ONE);
            let x = g.read::<u32>(s + 0x23f8);
            g.write::<u32>(o2 + 0xc4, x);
            let i = actor(g, k, 0x583);
            g.write::<u32>(o2 + 0x120, i);
        }
    }
    // the settle task (its append keeps no G_LAST: the light's follows)
    let (second, _) =
        crate::r27::pool_take_0587(g, img, f, [0x5b1, 0x5c6, 0x5f7, 0x5f7, 0x61c, 0x627], 0);
    let listed = g.read::<u32>(w(0x62d)) != 0;
    g.write::<u8>(second + 0x18, 1);
    g.write::<u32>(second + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x640));
        g.write::<u32>(second, last);
        g.write::<u32>(last.wrapping_add(4), second);
    } else {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x653), second);
    }
    g.write::<u32>(second + 8, second);
    let wl = s + 0x1cb8;
    g.write::<u32>(s + 0x23f0, second);
    g.write::<u32>(w(0x66b), 0x3f00_0000);
    g.write::<u32>(w(0x675), ONE);
    for (off, v) in [
        (8u32, 0x3f00_0000u32),
        (4, 0x3f00_0000),
        (0, 0x3f00_0000),
        (0xc, ONE),
        (0x18, ONE),
        (0x14, ONE),
        (0x10, ONE),
        (0x1c, ONE),
        (0x28, 0x1e),
        (0x34, 0),
        (0x30, 0x14),
        (0x2c, 0x14),
    ] {
        g.write::<u32>(wl + off, v);
    }
    let (third, eax) =
        crate::r27::pool_take_0587(g, img, f, [0x6eb, 0x5f7, 0x6f9, 0x701, 0x732, 0x73d], s);
    let listed = g.read::<u32>(w(0x743)) != 0;
    g.write::<u8>(third + 0x18, 1);
    g.write::<u32>(third + 0x14, wl);
    g.write::<u32>(third + 4, 0);
    g.write::<u32>(w(0x758), third);
    if listed {
        g.write::<u32>(third, second);
        g.write::<u32>(second + 4, third);
    } else {
        g.write::<u32>(third, 0);
        g.write::<u32>(w(0x784), third);
    }
    g.write::<u32>(third + 8, third);
    Ret::eax(eax)
}

/// [`landing_0397`] (the `rel32`s: the give back, the actor scale, three
/// object starts, the cookie checks; `rand` through the IAT).
pub const LANDING_0397: Sig = Sig {
    len: 1941,
    hash: 0xedd2_d4b8_894e_0131,
    calls: &[
        (0x22, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x121, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x272, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x35b, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x4f8, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x76e, Callee::Any),
        (0x78d, Callee::Any),
    ],
    imports: &[(0x1fe, "rand")],
};
