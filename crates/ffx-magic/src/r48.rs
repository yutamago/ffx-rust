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

use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
