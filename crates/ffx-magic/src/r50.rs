//! Round 50's ports (the last rows of `magic_0387`): the DLL's own code
//! written out, in tested pieces.

use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The operands one point of [`strip_mesh_0387`] reads (offsets in the
/// function: the slot table words of its slot 325 calls, the constants).
struct PointOps {
    /// slot 325: the next point `T` from the data
    next: u32,
    /// the alpha scale (the points that write a quad record)
    alpha: Option<u32>,
    side_z: u32,
    side_x: u32,
    drop: u32,
    /// slot 325: the middle vertex
    v0: u32,
    width_a: u32,
    /// slot 325: the vertex on the `+` side
    v1: u32,
    width_b: u32,
    /// slot 325: the vertex on the `-` side
    v2: u32,
}

/// The strip's first point (+0x192; no quad yet).
const FIRST: PointOps = PointOps {
    next: 0x19c,
    alpha: None,
    side_z: 0x222,
    side_x: 0x22a,
    drop: 0x268,
    v0: 0x26e,
    width_a: 0x2a6,
    v1: 0x2dd,
    width_b: 0x2f4,
    v2: 0x36e,
};

/// Every later point (+0x3fc; its `-` vertex in the first point's code).
const NEXT: PointOps = PointOps {
    next: 0x406,
    alpha: Some(0x488),
    side_z: 0x490,
    side_x: 0x4a1,
    drop: 0x577,
    v0: 0x591,
    width_a: 0x5b4,
    v1: 0x5eb,
    width_b: 0x602,
    v2: 0x36e,
};

/// The last point (+0x64d; its record ends the list).
const LAST: PointOps = PointOps {
    next: 0x657,
    alpha: Some(0x6e1),
    side_z: 0x6e9,
    side_x: 0x6fa,
    drop: 0x7a5,
    v0: 0x7b2,
    width_a: 0x7f0,
    v1: 0x81a,
    width_b: 0x837,
    v2: 0x891,
};

/// A quad record `X` (0x30 bytes, two halves of `(colour, C)` pairs and
/// four `u16` vertex numbers) joining the previous point's vertices `k -
/// 3..k` with this point's `k..k + 3`: `X + 0`, `+8`, `+0x18`, `+0x20` =
/// `col`, the words after them `c`; the numbers `(k - 3, k - 2, k, k +
/// 1)` at `+0x10` and `(k - 3, k - 1, k, k + 2)` at `+0x28`.
#[allow(clippy::cast_possible_truncation)] // `u16` stores of the numbers
fn quad<G: Game>(g: &mut G, x: u32, k: u32, col: u32, c: u32) {
    for o in [0x20, 0x18, 8, 0] {
        g.write::<u32>(x + o, col);
    }
    for o in [0x24, 0x1c, 0xc, 4] {
        g.write::<u32>(x + o, c);
    }
    let n = |d: i32| k.wrapping_add_signed(d) as u16;
    for (o, d) in [(0x10, -3), (0x12, -2), (0x14, 0), (0x16, 1)] {
        g.write::<u16>(x + o, n(d));
    }
    for (o, d) in [(0x28, -3), (0x2a, -1), (0x2c, 0), (0x2e, 2)] {
        g.write::<u16>(x + o, n(d));
    }
}

/// One point of [`strip_mesh_0387`] (`b` the frame base, `P` at `b +
/// 0xc8` the current point, `vc` its three vertices, `dc` the next
/// point's data): `T` (`b + 0xf8`) = the next point (slot 325); the
/// direction `(dx, dz) = (T - P) / |T - P|` (x and z, the length through
/// `_libm_sse2_sqrt_precise` of the double); with `quad = (X, k, I, c)`
/// the record at `X` ([`quad`], its colour `trunc(E[0x3e4] K P.w) << 24 |
/// E[0x3f0]`) and `u16 I[2] += 2`; then `U` (`b + 0xd8`) = `P` lowered
/// by `K3` in y goes to `vc`, and `W` (`b + 0xe8`) = `U` moved by `P.w
/// (dz Ka Kc, dx Kb Kc')` in x and z to `vc + 0x10` and the other way to
/// `vc + 0x20` (slot 325 each).
#[allow(clippy::too_many_arguments)]
fn point<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    b: u32,
    ops: &PointOps,
    vc: u32,
    dc: u32,
    quad_at: Option<(u32, u32, u32, u32)>,
) {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (p, t, u, v) = (b + 0xc8, b + 0xf8, b + 0xd8, b + 0xe8);
    // call width: 2
    g.call(slot_target(g, w(ops.next), 325), 0, &[t, dc]);
    let dx = sub(rf(g, t), rf(g, p));
    let dz = sub(rf(g, t + 8), rf(g, p + 8));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
    let len = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
    let (nx, nz) = (div(dx, len), div(dz, len));
    if let (Some(k), Some((x, n, i, c))) = (ops.alpha, quad_at) {
        let e = g.read::<u32>(b + 0x1c);
        let alpha = cvtt(mul(mul(rf(g, e + 0x3e4), rf(g, w(k))), rf(g, p + 0xc)));
        let col = (alpha.cast_unsigned() << 24) | g.read::<u32>(e + 0x3f0);
        quad(g, x, n, col, c);
        let q = g.read::<u16>(i + 2).wrapping_add(2);
        g.write::<u16>(i + 2, q);
    }
    let side_a = mul(nz, rf(g, w(ops.side_z)));
    let side_b = mul(nx, rf(g, w(ops.side_x)));
    for o in [0, 8, 0xc] {
        let x = g.read::<u32>(p + o);
        g.write::<u32>(u + o, x);
    }
    let y = sub(rf(g, p + 4), rf(g, w(ops.drop)));
    g.write::<f32>(u + 4, y);
    // call width: 2
    g.call(slot_target(g, w(ops.v0), 325), 0, &[vc, u]);
    let a2 = mul(side_a, rf(g, w(ops.width_a)));
    let y = g.read::<u32>(u + 4);
    g.write::<u32>(v + 4, y);
    let x = add(mul(rf(g, p + 0xc), a2), rf(g, u));
    g.write::<f32>(v, x);
    let b2 = mul(side_b, rf(g, w(ops.width_b)));
    let z = add(mul(rf(g, p + 0xc), b2), rf(g, u + 8));
    g.write::<f32>(v + 8, z);
    // call width: 2
    g.call(slot_target(g, w(ops.v1), 325), 0, &[vc + 0x10, v]);
    let half = rf(g, p + 0xc);
    let x = sub(rf(g, u), mul(half, a2));
    g.write::<f32>(v, x);
    let z = sub(rf(g, u + 8), mul(half, b2));
    g.write::<f32>(v + 8, z);
    // call width: 2
    g.call(slot_target(g, w(ops.v2), 325), 0, &[vc + 0x20, v]);
}

/// `magic_0387`'s strip mesh (0x10003d00, 2470 bytes; internal, `ecx` =
/// the effect `E`, `edx` = the strip `S`; a 16-aligned frame, `b` its
/// base below the saved `esi`/`edi`). The cursor `L` (`b + 0x40`) gets
/// `S`'s window ([`strip_window`](crate::s4_small::strip_window): `u16
/// L[4]` the head's points `n`, `L[6]` the rest's `r`, `L[8]` and
/// `L[0xc]` their data, 16-byte points `(x, y, z, half width)`); the
/// matrices `M1 = b + 0x108` and `M2 = b + 0x158` come from slot 231 on
/// two pairs of DLL globals. The DLL's mesh buffer `H` gets its header
/// (`u16 H[0x12]` = 3 `S[0x26]`, the vertices `V` at `H + 0x2f` rounded
/// down to 16, the records' block `I` after `u16 H[0x12]` (signed) 16-byte
/// vertices, rounded up to 4; `H[4]`, `H[8]` their offsets from `H`), and
/// `I` its own (`0x400`, 0, 0 at `+0`, `+8`, `+0xc`).
///
/// `P` = the head's first point; then each later point of the head
/// (`n - 1`), each of the rest (`r`) and the one after them (the window's
/// end) give `P`'s three vertices ([`point`]: the first one none, the
/// others a quad record each at `I + 0x10 + 0x30 i`); after each but the
/// last, `b + 0x148` = `P` and `P` = the point (slot 325). The last
/// record's byte `+0x31` = 0xff. Slot 308 `(H, R)` draws the mesh with
/// the request `R` at `b + 0x50` (`0x404`, `*G_A`, byte `0x44`, ...,
/// `M1` four times, `M2`, `*G_B`, `V`; words `+0x14` and bytes `+9..=+0xb`
/// left as the frame holds them, as the original leaves them); its `eax`
/// goes to `*G_B` and is returned.
///
/// With `n` 0 or 1 the original ends in `jmp $` (+0x190) after the
/// headers: it spins for ever, and so does the port (callers check only
/// that the window holds 2 points; a window that wraps after its first
/// point reaches it).
pub fn strip_mesh_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (e, s) = (fr.ecx, fr.edx);
    let a = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x198);
    let b = a.wrapping_sub(8);
    // the cookie word (`ds:__security_cookie ^ esp`)
    let cookie = g.read::<u32>(w(0xd)) ^ a;
    g.write::<u32>(b + 0x19c, cookie);
    g.write::<u32>(b + 0x1c, e);
    let l = b + 0x40;
    g.write::<u32>(l, s);
    crate::s4_small::strip_window(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x2b),
            esp: b.wrapping_sub(4),
            ecx: l,
            ..Frame::default()
        },
    );
    let (m1, m2) = (b + 0x108, b + 0x158);
    // call width: 3
    g.call(slot_target(g, w(0x42), 231), 0, &[m1, w(0x3c), w(0x30)]);
    // call width: 3
    g.call(slot_target(g, w(0x64), 231), 0, &[m2, w(0x5e), w(0x59)]);
    let h = w(0x7e);
    let (verts, i) = mesh_header(g, img, f, h, s);
    // `W`'s w
    g.write::<u32>(b + 0xf4, 0x3f80_0000);
    let c = g.read::<u32>(e + 0x3f4);
    let rest = i32::from(g.read::<u16>(l + 6));
    let head = g.read::<u32>(l + 8);
    let p = b + 0xc8;
    for o in [0, 4, 8, 0xc] {
        let x = g.read::<u32>(head + o);
        g.write::<u32>(p + o, x);
    }
    let mut left = i32::from(g.read::<u16>(l + 4)) - 1;
    if left <= 0 {
        // `jmp $`: the original never comes back
        loop {
            core::hint::spin_loop();
        }
    }
    let (mut vc, mut x, mut k, mut dc) = (verts, i + 0x10, 0u32, head + 0x10);
    // after each point but the last: `b + 0x148` = `P`, `P` = `T`
    let step = |g: &mut G, vc: &mut u32, k: &mut u32, dc: &mut u32| {
        // call width: 2
        g.call(slot_target(g, w(0x397), 325), 0, &[b + 0x148, p]);
        // call width: 2
        g.call(slot_target(g, w(0x3b9), 325), 0, &[p, b + 0xf8]);
        *vc = vc.wrapping_add(0x30);
        *k = k.wrapping_add(3);
        *dc = dc.wrapping_add(0x10);
    };
    point(g, img, f, b, &FIRST, vc, dc, None);
    step(g, &mut vc, &mut k, &mut dc);
    left -= 1;
    let mut pass = 0;
    loop {
        if left > 0 {
            point(g, img, f, b, &NEXT, vc, dc, Some((x, k, i, c)));
            x = x.wrapping_add(0x30);
            step(g, &mut vc, &mut k, &mut dc);
            left -= 1;
            continue;
        }
        pass += 1;
        if pass >= 2 {
            break;
        }
        left = rest;
        dc = g.read::<u32>(l + 0xc);
    }
    point(g, img, f, b, &LAST, vc, dc, Some((x, k, i, c)));
    g.write::<u8>(x + 0x31, 0xff);
    let eax = draw(g, img, f, b, h, verts);
    g.write::<u32>(w(0x995), eax);
    Ret::eax(eax)
}

/// [`strip_mesh_0387`]'s mesh buffer header (`h` the buffer, `s` the
/// strip; the 64-bit `adc` halves of the original are dead): returns the
/// vertices `V` and the records' block `I`.
fn mesh_header<G: Game>(g: &mut G, img: &Image, f: u32, h: u32, s: u32) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let win = g.read::<u16>(s + 0x26);
    g.write::<u16>(h + 0x12, win.wrapping_mul(3));
    let verts = h.wrapping_add(0x2f) & !0xf;
    let n3 = i32::from(g.read::<i16>(h + 0x12)).cast_unsigned();
    let i = verts.wrapping_add(n3 << 4).wrapping_add(3) & !3;
    g.write::<u32>(h, 0);
    g.write::<u32>(h + 4, i.wrapping_sub(w(0xc1)));
    g.write::<u32>(h + 8, verts.wrapping_sub(w(0xcb)));
    g.write::<u32>(h + 0xc, 0);
    g.write::<u32>(h + 0x14, 0x44_0000);
    g.write::<u32>(h + 0x1c, 0);
    g.write::<u16>(h + 0x10, 1);
    g.write::<u32>(i, 0x400);
    g.write::<u32>(i + 8, 0);
    g.write::<u32>(i + 0xc, 0);
    (verts, i)
}

/// [`strip_mesh_0387`]'s draw: the request `R` at `b + 0x50` and slot 308
/// `(h, R)` (its `eax`).
fn draw<G: Game>(g: &mut G, img: &Image, f: u32, b: u32, h: u32, verts: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let (m1, m2) = (b + 0x108, b + 0x158);
    let r = b + 0x50;
    let ga = g.read::<u32>(g.read::<u32>(w(0x8cc)));
    let gb = g.read::<u32>(w(0x932));
    for (o, v) in [
        (0, 0x404),
        (4, ga),
        (0xc, 0),
        (0x10, 0),
        (0x18, 0x8080_8080),
        (0x1c, 0),
        (0x20, m1),
        (0x24, m1),
        (0x28, m1),
        (0x2c, m1),
        (0x30, m2),
        (0x34, gb),
        (0x38, verts),
        (0x3c, 0),
        (0x40, 0),
        (0x44, 0),
    ] {
        g.write::<u32>(r + o, v);
    }
    g.write::<u8>(r + 8, 0x44);
    // call width: 2
    g.call(slot_target(g, w(0x943), 308), 0, &[h, r]).eax
}

/// [`strip_mesh_0387`] (the `rel32`s: the strip window, three
/// `_libm_sse2_sqrt_precise`, the cookie check).
pub const STRIP_MESH_0387: Sig = Sig {
    len: 2470,
    hash: 0xe462_843a_901c_395c,
    calls: &[
        (0x2b, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x1f2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x45c, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x6ad, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x99e, Callee::Any),
    ],
    imports: &[],
};

/// [`laser_visit_0387`]'s new particles: `O[0x3f8]` = `j` up to the
/// count `c = u8 O[0x3e8]` (below 0x30): particle `j + i` (of
/// `m = c - j`) gets a random vector `O + 0x1c + 16 (j + i)` (slot 307
/// four times, each `r K - K'`, the last `r K + K'`) and its record
/// `O + 0x31c + 4 (j + i)` = `(0, i 0x200 / m)` (`u16` each); then
/// `O[0x3f8]` = `c`.
fn new_particles<G: Game>(g: &mut G, img: &Image, f: u32, o: u32) {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let c = g.read::<u8>(o + 0x3e8);
    let j = g.read::<u32>(o + 0x3f8).cast_signed();
    if j >= i32::from(c) || c >= 0x30 {
        return;
    }
    let m = i32::from(c) - j;
    let q = 0x200 / m;
    // (slot word, scale, offset; the last adds)
    let parts = [
        (0x111, 0x12b, 0x138),
        (0x130, 0x157, 0x164),
        (0x15c, 0x182, 0x18f),
        (0x187, 0x1ae, 0x1be),
    ];
    for i in 0..m {
        let n = j.wrapping_add(i).cast_unsigned();
        let p = o.wrapping_add(0x1c).wrapping_add(n << 4);
        for (d, (s, k, k2)) in (0u32..).zip(parts) {
            let r = crate::m0018::st0_f32(g.call(slot_target(g, w(s), 307), 0, &[]).st0);
            let x = mul(r, rf(g, w(k)));
            let x = if d == 3 {
                add(x, rf(g, w(k2)))
            } else {
                sub(x, rf(g, w(k2)))
            };
            g.write::<f32>(p + 4 * d, x);
        }
        let rec = o.wrapping_add(0x31c).wrapping_add(n << 2);
        g.write::<u16>(rec, 0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov word`
        g.write::<u16>(rec + 2, q.wrapping_mul(i) as u16);
    }
    let c = g.read::<u8>(o + 0x3e8);
    g.write::<u32>(o + 0x3f8, u32::from(c));
}

/// One live particle of [`laser_visit_0387`] (`b` the frame base, `o`
/// the node, `pt` its strip point's `+0xc`, `x` its vector's `+8`, `rec`
/// its record, `last` whether the point is the head's last): the
/// strip's direction there `D` (`b + 0x48`; slot 234 of this point and
/// the next, or of the one before and this one at the end; slot 232,
/// slot 233 by 4.0) moves the particle's x, y, z; the point's half width
/// drops by `K` (0 at or below 0); the point (slot 325, `b + 0xa8`), the
/// particle (slot 248 into `b + 0x98`, `w` 1.0) and slot 229 give the
/// matrix `b + 0x58` (scale = the particle's `w`, its last row lowered by
/// `K2` in y), slot 231 `(b + 0xb8, G_M, b + 0x58)`; then the record's
/// phase `+2` grows by `flag << 9`, and past the frame's length (`i16
/// Q[0x12 + 8 frame]`, `Q = O[0x3ec]`) goes to the next frame.
#[allow(clippy::too_many_arguments)]
fn particle<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    b: u32,
    pt: u32,
    x: u32,
    rec: u32,
    q: u32,
    flag: u32,
    last: bool,
) {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let d = b + 0x48;
    let (p0, p1) = if last {
        (pt.wrapping_sub(0x1c), pt.wrapping_sub(0xc))
    } else {
        (pt.wrapping_sub(0xc), pt.wrapping_add(4))
    };
    // call width: 3
    g.call(slot_target(g, w(0x2ed), 234), 0, &[d, p0, p1]);
    // call width: 2
    g.call(slot_target(g, w(0x303), 232), 0, &[d, d]);
    // call width: 3
    g.call(slot_target(g, w(0x320), 233), 0, &[d, d, 0x4080_0000]);
    let v = x.wrapping_sub(8);
    for i in 0..3 {
        let s = add(rf(g, d + 4 * i), rf(g, v + 4 * i));
        g.write::<f32>(v + 4 * i, s);
    }
    let half = sub(rf(g, pt), rf(g, w(0x372)));
    g.write::<f32>(pt, half);
    let frame = g.read::<u16>(rec);
    // comiss 0, half; jb: above 0 or unordered stays
    if !(half.is_nan() || 0.0 < half) {
        g.write::<u32>(pt, 0);
    }
    // call width: 2
    g.call(
        slot_target(g, w(0x399), 325),
        0,
        &[b + 0xa8, pt.wrapping_sub(0xc)],
    );
    let scale = g.read::<u32>(x + 4);
    for o in [0x80, 0x6c, 0x58] {
        g.write::<u32>(b + o, scale);
    }
    // call width: 3
    g.call(slot_target(g, w(0x3c3), 248), 0, &[b + 0x98, b + 0xa8, v]);
    g.write::<u32>(b + 0xa4, 0x3f80_0000);
    // call width: 3
    g.call(
        slot_target(g, w(0x400), 229),
        0,
        &[b + 0x88, w(0x3fa), b + 0x98],
    );
    let y = sub(rf(g, b + 0x90), rf(g, w(0x424)));
    g.write::<f32>(b + 0x90, y);
    // call width: 3
    g.call(
        slot_target(g, w(0x43e), 231),
        0,
        &[b + 0xb8, w(0x438), b + 0x58],
    );
    #[allow(clippy::cast_possible_truncation)] // `add word`
    let phase = g.read::<u16>(rec + 2).wrapping_add((flag << 9) as u16);
    g.write::<u16>(rec + 2, phase);
    let len = g.read::<u16>(q + 0x12 + 8 * u32::from(frame));
    if i32::from(phase) >= i32::from(len.cast_signed()) {
        g.write::<u16>(rec + 2, phase.wrapping_sub(len));
        let n = g.read::<u16>(rec).wrapping_add(1);
        g.write::<u16>(rec, n);
    }
}

/// `magic_0387`'s laser visit (0x100037a0, 1226 bytes; internal, `ecx` =
/// the visitor `V`; an 8-aligned frame, `b` its base below the saved
/// `esi`/`edi`). After the [`walk`](crate::r48::walk) (none: its `eax`),
/// the node `O`'s strip `S = O[0x3dc]` is moved with the visitor `L =
/// (S, mode)` at `b + 0x28`
/// ([`laser_move_0387`](crate::r48::laser_move_0387)), meshed when its
/// window holds 2 points or more ([`strip_mesh_0387`]; with `ecx` = `O`)
/// and windowed again into `b + 0x38`
/// ([`strip_window`](crate::s4_small::strip_window)). New particles
/// start ([`new_particles`]); with `flag` = (`*G_F` == 0) the first
/// `min(c, n)` particles (`c = u8 O[0x3e8]`, `n` the head's points) whose
/// frame `u16` is below `i16 Q[6]` move along the strip ([`particle`]).
/// `*G_B` (the draw's handle, which the mesh's draw overwrote) gets its
/// old value back and is returned.
pub fn laser_visit_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let v = fr.ecx;
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xf8);
    let b = a.wrapping_sub(8);
    // the cookie word (`ds:__security_cookie ^ esp`)
    let cookie = g.read::<u32>(w(0xd)) ^ a;
    g.write::<u32>(b + 0xfc, cookie);
    let o = g.read::<u32>(v);
    let (eax, o) = crate::r48::walk(g, img, v, b.wrapping_sub(8), o, cookie);
    let Some(o) = o else {
        return Ret::eax(eax);
    };
    let l = b + 0x28;
    let s = g.read::<u32>(o + 0x3dc);
    g.write::<u32>(l, s);
    let mode = g.read::<u8>(v + 5);
    g.write::<u8>(l + 5, mode);
    let esp = b.wrapping_sub(4);
    crate::r48::laser_move_0387(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x8c),
            esp,
            ecx: l,
            ..Frame::default()
        },
    );
    let s = g.read::<u32>(o + 0x3dc);
    if g.read::<u16>(s + 0x26) >= 2 {
        strip_mesh_0387(
            g,
            img,
            &Frame {
                at: img.rel32_target(f + 0xa0),
                esp,
                ecx: o,
                edx: s,
                ..Frame::default()
            },
        );
    }
    let l2 = b + 0x38;
    let s = g.read::<u32>(o + 0x3dc);
    g.write::<u32>(l2, s);
    crate::s4_small::strip_window(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0xb3),
            esp,
            ecx: l2,
            ..Frame::default()
        },
    );
    new_particles(g, img, f, o);
    let saved = g.read::<u32>(w(0x201));
    let flag = u32::from(g.read::<u32>(w(0x216)) == 0);
    for z in [0x5c, 0x60, 0x64, 0x68, 0x70, 0x74, 0x78, 0x7c, 0x84] {
        g.write::<u32>(b + z, 0);
    }
    let c = u16::from(g.read::<u8>(o + 0x3e8));
    let n = g.read::<u16>(l2 + 4);
    let count = u32::from(c.min(n));
    let mut pt = g.read::<u32>(l2 + 8).wrapping_add(0xc);
    let (mut x, mut rec) = (o + 0x24, o + 0x31c);
    for i in 0..count {
        let q = g.read::<u32>(o + 0x3ec);
        let frames = i32::from(g.read::<i16>(q + 6));
        if i32::from(g.read::<u16>(rec)) < frames {
            let last = i + 1 == u32::from(g.read::<u16>(l2 + 4));
            particle(g, img, f, b, pt, x, rec, q, flag, last);
        }
        x = x.wrapping_add(0x10);
        pt = pt.wrapping_add(0x10);
        rec = rec.wrapping_add(4);
    }
    g.write::<u32>(w(0x4b2), saved);
    Ret::eax(saved)
}

/// [`laser_visit_0387`] (the `rel32`s: the laser move, the strip mesh,
/// the strip window, the cookie check).
pub const LASER_VISIT_0387: Sig = Sig {
    len: 1226,
    hash: 0x805e_d95c_e85b_e6cd,
    calls: &[
        (0x8c, Callee::Sig(&crate::r48::LASER_MOVE_0387)),
        (0xa0, Callee::Sig(&STRIP_MESH_0387)),
        (0xb3, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x4c2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0387` 0x100046b0: `EgoLzr`'s visit, the thunk over
/// [`laser_visit_0387`].
pub fn jmp_laser_visit_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r48::thunk_to(g, img, fr, laser_visit_0387)
}

/// [`jmp_laser_visit_0387`].
pub const JMP_LASER_VISIT_0387: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&LASER_VISIT_0387))],
    imports: &[],
};

/// Slot 771 `(h)`'s 64-bit handle `(lo, hi)`, then slot `slot` `(lo, hi,
/// rest...)` with the lock `**G_L` held at 1 and set back after (`ops`:
/// the slot table words of the two calls and the lock's two reads, in
/// the function's order). Returns the slot's result and the lock pointer
/// as read last (the `eax` some callers leave).
fn under_lock<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ops: [u32; 4],
    slot: u32,
    h: u32,
    rest: &[u32],
) -> (crate::Out, u32) {
    let w = |o: u32| img.word(f + o);
    // call width: 1
    let id = g.call(slot_target(g, w(ops[0]), 771), 0, &[h]);
    let lock = g.read::<u32>(w(ops[1]));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let mut args = [id.eax, id.edx, 0, 0, 0];
    args[2..2 + rest.len()].copy_from_slice(rest);
    // call width: 2 + rest (at most 5)
    let out = g.call(slot_target(g, w(ops[2]), slot), 0, &args[..2 + rest.len()]);
    let lock = g.read::<u32>(w(ops[3]));
    g.write::<u32>(lock, saved);
    (out, lock)
}

/// [`burst_task_0387`]'s draw of the burst at `A2` (`b + 0xa8`): the
/// draw matrix `b + 0x138` (its first two rows `M` (`b + 0xf8`) times
/// the size `s = b[0x2c]`, its last row `A2` through the DLL's matrix
/// `G`), the colour bytes `R[4..8]` = `trunc(A1)` (`b + 0x98`), `R[0x14]
/// = *G_B`; slot 908 `(lo, hi, Z, R)` when slot 773 answers.
fn burst_draw<G: Game>(g: &mut G, img: &Image, f: u32, b: u32, h: u32) {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (m, d, r) = (b + 0xf8, b + 0x138, b + 0x60);
    let s = rf(g, b + 0x2c);
    for o in [0, 4, 8, 0x10, 0x14, 0x18] {
        let x = mul(rf(g, m + o), s);
        g.write::<f32>(d + o, x);
    }
    for i in 0..4 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // `mov byte`
        let c = cvtt(rf(g, b + 0x98 + 4 * i)) as u8;
        g.write::<u8>(r + 4 + i, c);
    }
    let (x, y, z, ww) = (
        rf(g, b + 0xa8),
        rf(g, b + 0xac),
        rf(g, b + 0xb0),
        rf(g, b + 0xb4),
    );
    let k = |g: &G, o: u32| rf(g, w(o));
    // each row: y G[1, i] + x G[0, i] + z G[2, i] + w G[3, i]
    let row = |g: &G, o: [u32; 4]| {
        let t = add(mul(y, k(g, o[0])), mul(x, k(g, o[1])));
        let t = add(t, mul(z, k(g, o[2])));
        add(t, mul(ww, k(g, o[3])))
    };
    let tx = row(g, [0x6ac, 0x762, 0x77c, 0x793]);
    let ty = row(g, [0x6ec, 0x7ab, 0x7c3, 0x7db]);
    let tz = row(g, [0x76d, 0x7f3, 0x80b, 0x826]);
    // the last column: the matrix words first
    let tw = add(mul(k(g, 0x813), y), mul(k(g, 0x832), x));
    let tw = add(tw, mul(k(g, 0x84b), z));
    let tw = add(tw, mul(k(g, 0x85d), ww));
    let gb = g.read::<u32>(w(0x7b4));
    g.write::<u32>(r + 0x14, gb);
    for (o, v) in [(0x30, tx), (0x34, ty), (0x38, tz), (0x3c, tw)] {
        g.write::<f32>(d + o, v);
    }
    let (ok, _) = under_lock(g, img, f, [0x7cc, 0x87c, 0x888, 0x89c], 773, h, &[]);
    if ok.eax != 0 {
        let z = g.read::<u32>(b + 0x3c);
        under_lock(g, img, f, [0x8aa, 0x8bc, 0x8d7, 0x8e4], 908, h, &[z, r]);
    }
}

/// [`burst_task_0387`]'s path step: the points behind `A2` until the
/// path's length `pos` (`b + 0x24`) passes the distance `d`: the ring's
/// next point `cur` (none past `lim`: `None`), the segment from the last
/// (`b + 0xb8`, `b + 0xd8`, the step `b + 0xc8`), its length (x, y, z;
/// `_libm_sse2_sqrt_precise`) added to `pos`; then `A2` = the segment's
/// start plus the step times `acc / len` (`acc` at `b + 0x20`), `pos -=
/// d`, `acc += d`. Returns the ring's point.
fn burst_follow<G: Game>(g: &mut G, b: u32, n: u32, mut cur: u32, lim: u32, d: f32) -> Option<u32> {
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (base, step, next) = (b + 0xb8, b + 0xc8, b + 0xd8);
    let mut pos = rf(g, b + 0x24);
    // comiss pos, d; jae: an ordered `pos >= d` keeps the segment
    if pos.is_nan() || d.is_nan() || pos < d {
        loop {
            cur = if cur == 3 { 0 } else { cur + 1 };
            if cur == lim {
                return None;
            }
            for o in [0, 4, 8, 0xc] {
                let x = g.read::<u32>(next + o);
                g.write::<u32>(base + o, x);
            }
            let p = n.wrapping_add(0x2c).wrapping_add(cur << 4);
            for o in [0, 4, 8, 0xc] {
                let x = g.read::<u32>(p + o);
                g.write::<u32>(next + o, x);
            }
            let dv = [0u32, 4, 8, 0xc].map(|o| sub(rf(g, next + o), rf(g, base + o)));
            for (o, x) in [0u32, 4, 8, 0xc].into_iter().zip(dv) {
                g.write::<f32>(step + o, x);
            }
            let acc = sub(rf(g, b + 0x20), rf(g, b + 0x14));
            g.write::<f32>(b + 0x20, acc);
            let sq = add(add(mul(dv[1], dv[1]), mul(dv[0], dv[0])), mul(dv[2], dv[2]));
            // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
            #[allow(clippy::cast_possible_truncation)] // `cvtsd2ss`
            let len = f64::from(sq).sqrt() as f32;
            g.write::<f32>(b + 0x14, len);
            pos = add(rf(g, b + 0x24), len);
            g.write::<f32>(b + 0x24, pos);
            // comiss pos, d; jb: below or unordered goes on
            if !(pos.is_nan() || d.is_nan() || pos < d) {
                break;
            }
        }
    }
    let t = div(rf(g, b + 0x20), rf(g, b + 0x14));
    for o in [0, 4, 8, 0xc] {
        let x = add(rf(g, base + o), mul(rf(g, step + o), t));
        g.write::<f32>(b + 0xa8 + o, x);
    }
    g.write::<f32>(b + 0x24, sub(pos, d));
    let acc = add(rf(g, b + 0x20), d);
    g.write::<f32>(b + 0x20, acc);
    Some(cur)
}

/// [`burst_task_0387`]'s start (state 1 once `N[0x20]` reached 2): the
/// colour `A1 = (128, 110, 60, 64)`, its step down `(A1 - (128, 0, 0,
/// 0)) / 15` (slots 234, 667), the draw request `R` (`b + 0x60`), `Z =
/// Q + i16 Q[0x10]` (`Q = N[0x24]`), `M` = `G_M` (slot 230) scaled by `K`
/// on its diagonal, the draw matrix an identity (slot 235); with no
/// object yet (slot 773 0) slot 907 `(lo, hi, &Q, 4, slot 776 (u8 Z[Z[4] +
/// 4]) | slot 777 (0))`; slot 909. Returns `lim`: the newest point when
/// `N[0x20] > 4`, else 0.
fn burst_start<G: Game>(g: &mut G, img: &Image, f: u32, b: u32, n: u32, h: u32, idx: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (a1, a2) = (b + 0x98, b + 0xa8);
    for (o, x) in [
        (0, 0x4300_0000),
        (4, 0x42dc_0000),
        (8, 0x4270_0000),
        (0xc, 0x4280_0000),
        (0x10, 0x4300_0000),
        (0x14, 0),
        (0x18, 0),
        (0x1c, 0),
    ] {
        g.write::<u32>(a1 + o, x);
    }
    // call width: 3
    g.call(slot_target(g, w(0x18e), 234), 0, &[b + 0xb8, a1, a2]);
    // call width: 3
    g.call(
        slot_target(g, w(0x215), 667),
        0,
        &[b + 0xe8, b + 0xb8, 0x4170_0000],
    );
    let r = b + 0x60;
    g.write::<u32>(r + 0x10, b + 0x138);
    g.write::<u16>(r + 2, 0x80);
    for o in [0xc, 0x1c, 8] {
        g.write::<u32>(r + o, 0);
    }
    g.write::<u8>(r + 0x18, 0);
    let q = g.read::<u32>(n + 0x24);
    let z = q.wrapping_add_signed(i32::from(g.read::<i16>(q + 0x10)));
    g.write::<u32>(b + 0x3c, z);
    let m = b + 0xf8;
    // call width: 2
    g.call(slot_target(g, w(0x271), 230), 0, &[m, w(0x26b)]);
    let k = rf(g, w(0x28a));
    for o in [0, 0x14, 0x28] {
        let x = mul(rf(g, m + o), k);
        g.write::<f32>(m + o, x);
    }
    // call width: 1
    g.call(slot_target(g, w(0x29b), 235), 0, &[b + 0x138]);
    g.write::<u32>(b + 0x38, q);
    let (none, _) = under_lock(g, img, f, [0x2dd, 0x2f6, 0x302, 0x316], 773, h, &[]);
    if none.eax == 0 {
        let at = z.wrapping_add(g.read::<u32>(z + 4)).wrapping_add(4);
        let x = u32::from(g.read::<u8>(at));
        // call width: 1
        let e1 = g.call(slot_target(g, w(0x331), 776), 0, &[x]).eax;
        let y = g.read::<u32>(b + 0x7c);
        // call width: 1
        let e2 = g.call(slot_target(g, w(0x33f), 777), 0, &[y]).eax;
        let rest = [b + 0x38, 4, e1 | e2];
        under_lock(g, img, f, [0x356, 0x36d, 0x387, 0x394], 907, h, &rest);
    }
    under_lock(g, img, f, [0x3a2, 0x3b4, 0x3c0, 0x3d3], 909, h, &[]);
    if g.read::<u32>(n + 0x20).cast_signed() > 4 {
        idx
    } else {
        0
    }
}

/// `magic_0387`'s burst task (0x10009c20, 3251 bytes; the callback of
/// [`burst_setup_0387`](crate::r48::burst_setup_0387), cdecl `(T)`, `N
/// = T[4]`, `H = T[0]` the handle's source; a 16-aligned frame, `b` its
/// base below the saved `esi`/`edi`). First `N`'s source `N[0x28]` is
/// visited with `(N[0x28], *G_MODE)` (its method, see
/// [`method`](crate::r48::method)); then by the state `N[0x18]`:
/// - 0x1000: with bit 2 of `N[0x328]` the four nodes `N + 0x6c..0xf0`
///   leave their parent ([`leave`](crate::sa_misc::leave)), `T` is
///   unlinked, slot 779 `(slot 771 (H))` under the lock, state 0x2000
///   (the lock pointer returned; without the bit 0);
/// - 0 (count `N[0x20]` = 0, state 1) and 1: the source's position
///   `N[0x28] + 0xcc` goes to the ring `N + 0x2c + 16 (3 - c % 4)`
///   (slot 325), the count up; from 2 on the burst runs: its start
///   ([`burst_start`]), `A2` = the newest point, the length to the next
///   (slots 325, 234, 706); up to 16 times slot 910 `(lo, hi, &Q, 0)`
///   (while slot 773 answers; until `K - 0.1 n` is no longer above 0);
///   with an object, slot 886 `(o, 0, o[0x94][4], o[0x94][8])` (0 ends:
///   0); slot 909; then up to 16 draws ([`burst_draw`]), `A1` stepping
///   down, the distance `d` and the size `s` shrinking by `K`, `A2`
///   following the path ([`burst_follow`]) until `d` reaches 0 or the
///   path its end; at the end slot 775 `(slot 771 (H))` when slot 773
///   answers (the lock pointer returned; else 0);
/// - anything else: nothing (`state - 0x1000`).
pub fn burst_task_0387<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let a = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x178);
    let b = a.wrapping_sub(8);
    // the cookie word (`ds:__security_cookie ^ esp`)
    let cookie = g.read::<u32>(w(0xd)) ^ a;
    g.write::<u32>(b + 0x17c, cookie);
    let mode = g.read::<u8>(w(0x1b));
    let tk = fr.arg(g, 0);
    let n = g.read::<u32>(tk + 4);
    let src = g.read::<u32>(n + 0x28);
    let l = b + 0x58;
    g.write::<u32>(l, src);
    g.write::<u8>(l + 5, mode);
    let visit = g.read::<u32>(g.read::<u32>(src) + 0x1c);
    crate::r48::method(g, img, visit, b.wrapping_sub(8), l, l);
    let state = g.read::<u32>(n + 0x18);
    let h = g.read::<u32>(tk);
    match state {
        0 | 1 => {}
        0x1000 => {
            if g.read::<u8>(n + 0x328) & 4 == 0 {
                return Ret::eax(0);
            }
            for o in [0x6c, 0x98, 0xc4, 0xf0] {
                crate::sa_misc::leave(g, n + o, 0);
            }
            crate::r48::unlink_task(g, tk);
            let (_, lock) = under_lock(g, img, f, [0xbd, 0xd1, 0xdd, 0xf0], 779, h, &[]);
            g.write::<u32>(n + 0x18, 0x2000);
            return Ret::eax(lock);
        }
        s => return Ret::eax(s.wrapping_sub(0x1000)),
    }
    if state == 0 {
        g.write::<u32>(n + 0x20, 0);
        g.write::<u32>(n + 0x18, 1);
    }
    let count = g.read::<u32>(n + 0x20).cast_signed();
    let idx = (3 - count % 4).cast_unsigned();
    let src = g.read::<u32>(n + 0x28);
    let ring = |i: u32| n.wrapping_add(0x2c).wrapping_add(i << 4);
    // call width: 2
    let eax = g
        .call(slot_target(g, w(0x151), 325), 0, &[ring(idx), src + 0xcc])
        .eax;
    let count = g.read::<u32>(n + 0x20).wrapping_add(1);
    g.write::<u32>(n + 0x20, count);
    if count.cast_signed() < 2 {
        return Ret::eax(eax);
    }
    let lim = burst_start(g, img, f, b, n, h, idx);
    let k = rf(g, w(0x3db));
    g.write::<f32>(b + 0x2c, k);
    g.write::<f32>(b + 0x30, k);
    // call width: 2
    g.call(slot_target(g, w(0x40b), 325), 0, &[b + 0xa8, ring(idx)]);
    // call width: 2
    g.call(slot_target(g, w(0x42e), 325), 0, &[b + 0xb8, b + 0xa8]);
    let cur = if idx == 3 { 0 } else { idx + 1 };
    // call width: 2
    g.call(slot_target(g, w(0x463), 325), 0, &[b + 0xd8, ring(cur)]);
    // call width: 3
    g.call(
        slot_target(g, w(0x488), 234),
        0,
        &[b + 0xc8, b + 0xd8, b + 0xb8],
    );
    let acc = rf(g, w(0x498));
    g.write::<f32>(b + 0x20, acc);
    // call width: 1
    let st0 = g.call(slot_target(g, w(0x4a8), 706), 0, &[b + 0xc8]).st0;
    let len = crate::m0018::st0_f32(st0);
    g.write::<f32>(b + 0x14, len);
    g.write::<f32>(b + 0x24, len);
    let mut x = rf(g, w(0x4d8));
    for left in (0..16).rev() {
        let (busy, _) = under_lock(g, img, f, [0x4f1, 0x500, 0x50c, 0x520], 773, h, &[]);
        if busy.eax != 0 {
            let rest = [b + 0x38, 0];
            under_lock(g, img, f, [0x52e, 0x53d, 0x556, 0x563], 910, h, &rest);
        }
        if left == 0 {
            break;
        }
        x = sub(x, rf(g, w(0x57c)));
        // comiss 0, x; jb: above 0 or unordered goes on
        if !(x.is_nan() || 0.0 < x) {
            break;
        }
    }
    let (obj, _) = under_lock(g, img, f, [0x593, 0x5a2, 0x5ae, 0x5c2], 773, h, &[]);
    if obj.eax != 0 {
        let (o, _) = under_lock(g, img, f, [0x5d4, 0x5e6, 0x5f2, 0x606], 773, h, &[]);
        let e = g.read::<u32>(o.eax + 0x94);
        let (e4, e8) = (g.read::<u32>(e + 4), g.read::<u32>(e + 8));
        // call width: 4
        let ok = g.call(slot_target(g, w(0x61c), 886), 0, &[o.eax, 0, e4, e8]);
        if ok.eax == 0 {
            return Ret::eax(0);
        }
    }
    under_lock(g, img, f, [0x634, 0x646, 0x652, 0x665], 909, h, &[]);
    burst_run(g, img, f, b, n, h, cur, lim)
}

/// [`burst_task_0387`]'s draws (16 at most) and its end.
#[allow(clippy::too_many_arguments)]
fn burst_run<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    b: u32,
    n: u32,
    h: u32,
    mut cur: u32,
    lim: u32,
) -> Ret {
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    for left in (0..16).rev() {
        burst_draw(g, img, f, b, h);
        if left == 0 {
            break;
        }
        for o in [0, 4, 8, 0xc] {
            let x = sub(rf(g, b + 0x98 + o), rf(g, b + 0xe8 + o));
            g.write::<f32>(b + 0x98 + o, x);
        }
        let d = sub(rf(g, b + 0x30), rf(g, w(0x913)));
        g.write::<f32>(b + 0x30, d);
        let s = sub(rf(g, b + 0x2c), rf(g, w(0x981)));
        g.write::<f32>(b + 0x2c, s);
        // comiss 0, d; jae: an ordered `0 >= d` ends
        if 0.0 >= d {
            break;
        }
        match burst_follow(g, b, n, cur, lim, d) {
            Some(c) => cur = c,
            None => break,
        }
    }
    let (any, _) = under_lock(g, img, f, [0xc27, 0xc3a, 0xc46, 0xc5a], 773, h, &[]);
    if any.eax == 0 {
        return Ret::eax(0);
    }
    let (_, lock) = under_lock(g, img, f, [0xc68, 0xc77, 0xc83, 0xc96], 775, h, &[]);
    Ret::eax(lock)
}

/// [`burst_task_0387`] (the `rel32`s: four node leaves, two cookie
/// checks, `_libm_sse2_sqrt_precise`).
pub const BURST_TASK_0387: Sig = Sig {
    len: 3251,
    hash: 0xb067_680c_8950_5df8,
    calls: &[
        (0x7e, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x89, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x94, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x9f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x110, Callee::Any),
        (0xae6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xcab, Callee::Any),
    ],
    imports: &[],
};
