//! Round 42's ports (own rows of the DLLs below `magic_0250`): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code.

use crate::ob_model::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0186`'s matrix visit (0x10004f00, 1108 bytes; internal, `ecx` =
/// the cursor, reached through its thunk 0x10005390; a cookie frame, `ebp`
/// = `E - 4` with `E` the entry `esp`). The node walk; when it ran, the
/// owner `N`'s three objects `N[0x13c]`, `N[0x144]` and `N[0x140]` are
/// visited (each through its handler with a cursor at `ebp - 0x9c`: the
/// object, the walk's pass byte at `+5`). While the fade `N[0x1f8]` runs,
/// `N[0x1e4]` = `N[0x1dc] + i * N[0x1e0] / n` (`i` = `N[0x1fc]`, counted
/// up, `n` = `N[0x200]`; at `i >= n` the fade stops at `N[0x1dc] +
/// N[0x1e0]`) and slot 21 `(N[0x1e8], N[0x1f4], N[0x1e4])` gets it. Then
/// the DLL's function table (the operand at `+0x176`) by the byte
/// `N[0x148]` `(N + 0x5c, N[0x144] + 0x1c)`, slot 325 `(N + 0x12c,
/// N[0x140] + 0x1c)`; the point `N + 0x17c` follows `N + 0x1cc` through
/// three scaled rows (slots 325, 233 x3, 325, 229 with `N[0x13c] + 0x1c`)
/// when `N[0x1f0]` is set, or is `N[0x13c]`'s `+0x1c`..`+0x24`. Slot 231
/// builds the matrix at `ebp - 0x94`; slot 233 scales `N[0x140] + 0x1c` by
/// its `+0x28` into the vector at `ebp - 0x14`, whose three lanes scale
/// the rows slot 232 gives into `N + 0x9c`..`N + 0xbc`; slot 325 copies
/// row 3 to `N + 0xcc`; slots 210, 327 and 231 build `N + 0x18c`; the
/// three floats `N + 0x1bc`.. are scaled by the constant at `+0x3f1`, and
/// slot 9 `(N[0x1e8], N[0x1ec], N + 0x18c)` ends it. Returns slot 9's
/// `eax`, or the walk's.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn matrix_visit_0186<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let rd = |g: &G, a: u32| g.read::<u32>(a);
    let ebp = esp.wrapping_sub(4);
    let eax = g.read::<u32>(w(0x0a)) ^ ebp;
    let (eax, tail) = crate::nodes::walk(g, this, this, eax);
    let Some(n) = tail else {
        return eax;
    };
    let cur = ebp.wrapping_sub(0x9c);
    for off in [0x13cu32, 0x144, 0x140] {
        let pass = g.read::<u8>(this + 5);
        let obj = rd(g, n + off);
        g.write::<u32>(cur, obj);
        g.write::<u8>(cur + 5, pass);
        let handler = rd(g, rd(g, obj).wrapping_add(0x1c));
        g.call(handler, cur, &[cur]);
    }
    if rd(g, n + 0x1f8) != 0 {
        let i = rd(g, n + 0x1fc).cast_signed();
        let count = rd(g, n + 0x200).cast_signed();
        let span = g.read::<f32>(n + 0x1e0);
        let from = g.read::<f32>(n + 0x1dc);
        g.write::<f32>(
            n + 0x1e4,
            add(
                div(
                    mul(cvt(i.cast_unsigned()), span),
                    cvt(count.cast_unsigned()),
                ),
                from,
            ),
        );
        g.write::<u32>(n + 0x1fc, i.wrapping_add(1).cast_unsigned());
        if i >= count {
            g.write::<u32>(n + 0x1f8, 0);
            g.write::<f32>(n + 0x1e4, add(from, span));
        }
        let (a, b, x) = (rd(g, n + 0x1e8), rd(g, n + 0x1f4), rd(g, n + 0x1e4));
        // call width: 3
        g.call(t(g, 0x144, 21), 0, &[a, b, x]);
    }
    let idx = u32::from(g.read::<u8>(n + 0x148));
    let fun = rd(g, w(0x176).wrapping_add(4 * idx));
    let m = rd(g, n + 0x144).wrapping_add(0x1c);
    // call width: 2 (the table's thunks: ecx, edx = the two words)
    g.call(fun, idx, &[n + 0x5c, m]);
    let m = rd(g, n + 0x140).wrapping_add(0x1c);
    // call width: 2
    g.call(t(g, 0x192, 325), 0, &[n + 0x12c, m]);
    let p = n + 0x17c;
    // 0.1f, the scale word the three slot 233 calls share
    let tenth = 0x3dcc_cccd;
    if rd(g, n + 0x1f0) == 0 {
        for i in 0..3 {
            let v = rd(g, rd(g, n + 0x13c).wrapping_add(0x1c + 4 * i));
            g.write::<u32>(p + 4 * i, v);
        }
    } else {
        // call width: 2
        g.call(t(g, 0x1b6, 325), 0, &[p, n + 0x1cc]);
        for (op, row, src) in [
            (0x1df, 0x54u32, 0x14cu32),
            (0x201, 0x44, 0x15c),
            (0x223, 0x34, 0x16c),
        ] {
            // call width: 3
            g.call(t(g, op, 233), 0, &[ebp - row, n + src, tenth]);
        }
        // call width: 2
        g.call(t(g, 0x235, 325), 0, &[ebp - 0x24, p]);
        let o = rd(g, n + 0x13c).wrapping_add(0x1c);
        // call width: 3
        g.call(t(g, 0x250, 229), 0, &[p, ebp - 0x54, o]);
    }
    let mat = ebp - 0x94;
    // call width: 3
    g.call(t(g, 0x29f, 231), 0, &[mat, n + 0x14c, n + 0x5c]);
    let o = rd(g, n + 0x140);
    let k = rd(g, o.wrapping_add(0x28));
    // call width: 3
    g.call(t(g, 0x2c7, 233), 0, &[ebp - 0x14, o.wrapping_add(0x1c), k]);
    for (i, (o232, o233)) in [(0x2dbu32, 0x304u32), (0x318, 0x341), (0x352, 0x37b)]
        .into_iter()
        .enumerate()
    {
        let i = i as u32;
        let row = n + 0xdc + 0x10 * i;
        // call width: 2
        g.call(t(g, o232, 232), 0, &[row, mat + 0x10 * i]);
        let s = rd(g, ebp - 0x14 + 4 * i);
        // call width: 3
        g.call(t(g, o233, 233), 0, &[n + 0x9c + 0x10 * i, row, s]);
    }
    // call width: 2
    g.call(t(g, 0x393, 325), 0, &[n + 0xcc, mat + 0x30]);
    let id = rd(g, n + 0x1e8);
    // call width: 2
    let r = g.call(t(g, 0x3a0, 210), 0, &[id, 0]).eax;
    let m2 = ebp - 0x54;
    // call width: 2
    g.call(t(g, 0x3ba, 327), 0, &[m2, r]);
    let q = n + 0x18c;
    // call width: 3
    g.call(t(g, 0x3d2, 231), 0, &[q, m2, mat]);
    let c = g.read::<f32>(w(0x3f1));
    for off in [0x1bcu32, 0x1c0, 0x1c4] {
        let x = mul(g.read::<f32>(n + off), c);
        g.write::<f32>(n + off, x);
    }
    let (a, b) = (rd(g, n + 0x1e8), rd(g, n + 0x1ec));
    // call width: 3
    g.call(t(g, 0x3f6, 9), 0, &[a, b, q]).eax
}

/// [`matrix_visit_0186`] (1108 bytes, 1 overlay).
pub const MATRIX_VISIT_0186: Sig = Sig {
    len: 1108,
    hash: 0x7e61_4e3a_6dfd_d758,
    calls: &[
        // __security_check_cookie
        (0x44c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0186`'s 12-byte thunk over [`matrix_visit_0186`] (0x10005390:
/// `ecx` = the first stack word, then a jump with the caller's stack).
pub fn jmp_matrix_visit_0186<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let f = img.rel32_target(fr.at + 8);
    Ret::eax(matrix_visit_0186(g, img, f, fr.esp, this))
}

/// [`jmp_matrix_visit_0186`].
pub const JMP_MATRIX_VISIT_0186: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&MATRIX_VISIT_0186))],
    imports: &[],
};

/// `magic_0072`'s particle records task (0x10002760, 904 bytes; a stored
/// callback, no arguments; an 8-aligned /GS frame, `B = ((E - 4) & !7) -
/// 0x160` the frame base after its three pushes). Slots 723 `(B + 0x118)`
/// and 724 `(B + 0xd8)`; the globals at the operands `+0x55` and `+0x61`
/// get the frame matrices `M = B + 0x58` and `W = B + 0x98`; slot 235 sets
/// the scale matrix `S = B + 0x18`. For each of the `*G_N` records `R`
/// (0x6c bytes from `*G_R`) with `R + 0x68` set:
/// - a delay `R + 0x58` above 0 counts down and skips the record;
/// - else (`R + 0x44` cleared unless `*G_HOLD`) the velocity `R + 0x2c`
///   is added to `R + 0x1c` and that to the position `R + 0xc` (three
///   lanes each); the colour word `G_REC + 4` is `R + 0x3c` while the timer
///   `R + 0x44` runs (down by the constant at `+0x134`), a blend of the
///   bytes `R + 0x3c` and `R + 0x40` by `|R + 0x60| / R + 0x5c` while the
///   fade `R + 0x5c` is above 0 (`R + 0x60` counted up, the fade stopped
///   past it), else `R + 0x40` with `R + 0x68` cleared;
/// - above the floor `R + 0x54` the height `R + 0x10` is put on it, the
///   vertical speeds cleared, the fade doubled, and `R + 0x1c` and `R +
///   0x24` damped by `slot 404 (0.5) + c` each;
/// - `S`'s diagonal is `R + 0x4c`; slot 330 `(M, S, R + 0x28)`, slot 229
///   `(M + 0x30, B + 0xd8, R + 0xc)`, slot 230 `(W, M)`, `W`'s `+0x38`
///   raised by `R + 0x50`; the animation steps
///   ([`anim_step`](crate::sb_common::anim_step)), `R + 0x68` clears once
///   `R + 0x64` is 1 and `R + 4` is 0, and the part is linked
///   ([`link_part`](crate::sa_misc::link_part) `(G_REC, i)`), counted.
///
/// The count goes to the global at `+0x37b` and is returned.
#[allow(clippy::too_many_lines)] // one loop
pub fn records_task_0072<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x160);
    let base = g.read::<u32>(w(0x1f));
    // call width: 1
    g.call(t(g, 0x2c, 723), 0, &[b + 0x118]);
    // call width: 1
    g.call(t(g, 0x44, 724), 0, &[b + 0xd8]);
    let (m, s, wm) = (b + 0x58, b + 0x18, b + 0x98);
    g.write::<u32>(w(0x55), m);
    g.write::<u32>(w(0x61), wm);
    // call width: 1
    g.call(t(g, 0x6e, 235), 0, &[s]);
    let mut count = g.read::<i32>(w(0x7c));
    let mut linked = 0u32;
    let mut i = 0i32;
    while i < count {
        let r = base.wrapping_add(i.cast_unsigned().wrapping_mul(0x6c));
        let rd = |g: &G, o: u32| g.read::<u32>(r.wrapping_add(o));
        if rd(g, 0x68) == 0 {
            i += 1;
            continue;
        }
        let delay = rd(g, 0x58).cast_signed();
        if delay > 0 {
            g.write::<u32>(r + 0x58, delay.wrapping_sub(1).cast_unsigned());
            i += 1;
            continue;
        }
        if g.read::<u32>(w(0xc0)) == 0 {
            g.write::<u32>(r + 0x44, 0);
        }
        for o in [0u32, 4, 8] {
            let v = add(rf(g, r + 0x2c + o), rf(g, r + 0x1c + o));
            g.write::<f32>(r + 0x1c + o, v);
        }
        for o in [0u32, 4, 8] {
            let p = add(rf(g, r + 0xc + o), rf(g, r + 0x1c + o));
            g.write::<f32>(r + 0xc + o, p);
        }
        let timer = rf(g, r + 0x44);
        if timer > 0.0 {
            g.write::<f32>(r + 0x44, sub(timer, g.read::<f32>(w(0x134))));
            g.write::<u32>(w(0x141), rd(g, 0x3c));
        } else if rd(g, 0x5c).cast_signed() > 0 {
            let at = rd(g, 0x60).cast_signed().wrapping_abs();
            for (j, op) in [0x178u32, 0x196, 0x1b4, 0x1d6].into_iter().enumerate() {
                let j = j as u32;
                let n = rd(g, 0x5c).cast_signed();
                let lo = i32::from(g.read::<u8>(r + 0x3c + j));
                let hi = i32::from(g.read::<u8>(r + 0x40 + j));
                let v = lo
                    .wrapping_mul(n.wrapping_sub(at))
                    .wrapping_add(hi.wrapping_mul(at));
                g.write::<u8>(w(op), v.wrapping_div(n).to_le_bytes()[0]);
            }
            let next = rd(g, 0x60).wrapping_add(1);
            g.write::<u32>(r + 0x60, next);
            if next.cast_signed() > rd(g, 0x5c).cast_signed() {
                g.write::<u32>(r + 0x5c, 0);
            }
        } else {
            g.write::<u32>(w(0x1f2), rd(g, 0x40));
            g.write::<u32>(r + 0x68, 0);
        }
        let floor = rf(g, r + 0x54);
        if rf(g, r + 0x10) > floor {
            g.write::<f32>(r + 0x10, floor);
            g.write::<u32>(r + 0x20, 0);
            g.write::<u32>(r + 0x30, 0);
            g.write::<u32>(r + 0x5c, rd(g, 0x5c).wrapping_mul(2));
            for (o, op, ok) in [(0x1cu32, 0x22c, 0x24e), (0x24, 0x253, 0x27e)] {
                // call width: 1
                let x = st0_f32(g.call(t(g, op, 404), 0, &[0x3f00_0000]).st0);
                let v = mul(add(x, g.read::<f32>(w(ok))), rf(g, r + o));
                g.write::<f32>(r + o, v);
            }
        }
        let k = rd(g, 0x4c);
        for o in [0u32, 0x14, 0x28] {
            g.write::<u32>(s + o, k);
        }
        let y = rd(g, 0x28);
        // call width: 3
        g.call(t(g, 0x2ba, 330), 0, &[m, s, y]);
        // call width: 3
        g.call(t(g, 0x2db, 229), 0, &[m + 0x30, b + 0xd8, r + 0xc]);
        // call width: 2
        g.call(t(g, 0x2f8, 230), 0, &[wm, m]);
        let z = add(rf(g, r + 0x50), rf(g, wm + 0x38));
        g.write::<f32>(wm + 0x38, z);
        let anim = crate::sb_common::anim_step(g, img, img.rel32_target(f + 0x322), r);
        if rd(g, 0x64) == 1 && rd(g, 4) == 0 {
            g.write::<u32>(r + 0x68, 0);
        }
        // push i; push G_REC: the return address at `B - 12`
        let esp = b.wrapping_sub(12);
        g.write::<u32>(esp + 4, w(0x33b));
        g.write::<u32>(esp + 8, i.cast_unsigned());
        let lf = Frame {
            at: img.rel32_target(f + 0x345),
            esp,
            ecx: r,
            edx: anim,
            ..*fr
        };
        crate::sa_misc::link_part(g, img, &lf);
        linked = linked.wrapping_add(1);
        count = g.read::<i32>(w(0x34f));
        i += 1;
    }
    g.write::<u32>(w(0x37b), linked);
    Ret::eax(linked)
}

/// [`records_task_0072`] (904 bytes, 1 overlay).
pub const RECORDS_TASK_0072: Sig = Sig {
    len: 904,
    hash: 0x3c57_b1f7_36a2_d220,
    calls: &[
        (0x322, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x345, Callee::Sig(&crate::sa_misc::LINK_PART)),
        // __security_check_cookie
        (0x380, Callee::Any),
    ],
    imports: &[],
};

const ONE: u32 = 0x3f80_0000;

/// `magic_0152`'s screen object set-up (0x10008580, 1014 bytes; internal,
/// `ecx` = the object `O`, no frame):
/// [`screen_setup_0601`](crate::r31::screen_setup_0601) with the node
/// insert inlined. The
/// static list, the screen slots and the three children `O + 0xb0`, `O +
/// 0xdc`, `O + 0x108` as there; then `O + 0x124..` = `(0x1000, -0x1000,
/// 0)` and the node `N = O + 0x134` (its vtable, a child of the parent)
/// takes the three children (`+0x13c`, `+0x140`, `+0x144`: the first,
/// second and third), the DLL's block (`+0x128`, the operand at `+0x2ee`)
/// and the table index byte `+0x148` 0; the children `O + 0x30` and `O +
/// 0x70` hold `N` as there. Returns the parent's last child before `O +
/// 0x70` was appended.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn screen_setup_0152<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_misc::child_of_0512 as child;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.ecx;
    let o = |off: u32| obj.wrapping_add(off);
    // the object on the static list
    g.write::<u32>(o(8), obj);
    g.write::<u32>(o(4), obj);
    g.write::<u32>(obj, w(0xd));
    g.write::<u32>(o(0x10), w(0x14));
    g.write::<u32>(o(0x14), obj);
    let last = g.read::<u32>(w(0x1c));
    g.write::<u32>(o(8), last);
    g.write::<u32>(o(4), w(0x26));
    let last = g.read::<u32>(w(0x2b));
    g.write::<u32>(last.wrapping_add(4), obj);
    let fl = g.read::<u32>(w(0x3b)) & !1;
    g.write::<u32>(w(0x3b), fl);
    g.write::<u32>(w(0x42), obj);
    // the screen slots
    // call width: 3
    let eax = g.call(t(g, 0x35, 687), 0, &[1, 1, 7]).eax;
    g.write::<u32>(o(0x20), eax);
    // call width: 3
    g.call(t(g, 0x59, 692), 0, &[0, 1, 0]);
    // call width: 3
    g.call(t(g, 0x6c, 692), 0, &[1, 1, 1]);
    for (n, t691, t690) in [(0u32, 0x82u32, 0xa6u32), (1, 0xc6, 0xea)] {
        // call width: 7
        g.call(t(g, t691, 691), 0, &[n, 0, 0, 0x200, 0x1a0, n, u32::MAX]);
        // call width: 5
        g.call(t(g, t690, 690), 0, &[n, 0, 0, 0x200, 0x1a0]);
    }
    // call width: 2
    g.call(t(g, 0x10a, 693), 0, &[0x28, 0x14]);
    // the children
    let c1 = o(0xb0);
    child(
        g,
        img,
        f,
        c1,
        [0x135, 0x13c, 0x141, 0x149, 0x14e, 0x156, 0x162, 0x168],
    );
    g.write::<u32>(c1, w(0x16e));
    for (off, v) in [(0xccu32, ONE), (0xd0, 0xc170_0000), (0xd4, 0), (0xd8, ONE)] {
        g.write::<u32>(o(off), v);
    }
    let c2 = o(0xdc);
    child(
        g,
        img,
        f,
        c2,
        [0x1af, 0x1b6, 0x1bb, 0x1c3, 0x1c8, 0x1d0, 0x1dc, 0x1e2],
    );
    g.write::<u32>(c2, w(0x1e8));
    for (off, v) in [
        (0xf8u32, ONE),
        (0xfc, ONE),
        (0x100, ONE),
        (0x104, 0x447a_0000),
    ] {
        g.write::<u32>(o(off), v);
    }
    let c3 = o(0x108);
    g.write::<u32>(c3, w(0x21c));
    child(
        g,
        img,
        f,
        c3,
        [0x22f, 0x236, 0x23b, 0x243, 0x248, 0x250, 0x25c, 0x262],
    );
    for (off, v) in [(0x124u32, 0x1000u32), (0x128, 0xffff_f000), (0x12c, 0)] {
        g.write::<u32>(o(off), v);
    }
    // the node holding the three children (the inlined insert)
    let node = o(0x134);
    g.write::<u32>(node, w(0x28c));
    child(
        g,
        img,
        f,
        node,
        [0x29f, 0x2a6, 0x2ab, 0x2b3, 0x2b8, 0x2c0, 0x2cc, 0x2d2],
    );
    g.write::<u32>(node + 0x144, c3);
    g.write::<u32>(node + 0x13c, c1);
    g.write::<u32>(node + 0x140, c2);
    g.write::<u32>(node + 0x128, w(0x2ee));
    g.write::<u8>(node + 0x148, 0);
    // the two children holding the node
    let c4 = o(0x30);
    child(
        g,
        img,
        f,
        c4,
        [0x30b, 0x312, 0x317, 0x31f, 0x324, 0x32c, 0x338, 0x33e],
    );
    g.write::<u32>(c4 + 0x38, ONE);
    g.write::<u32>(c4 + 0x3c, node);
    g.write::<u32>(c4, w(0x34e));
    for (off, v) in [(0x5cu32, 0u32), (0x60, 0), (0x64, 0xc120_0000)] {
        g.write::<u32>(o(off), v);
    }
    let c5 = o(0x70);
    let eax = g.read::<u32>(w(0x38d));
    child(
        g,
        img,
        f,
        c5,
        [0x379, 0x380, 0x385, 0x38d, 0x392, 0x39a, 0x3a6, 0x3ac],
    );
    g.write::<u32>(c5 + 0x38, ONE);
    g.write::<u32>(c5 + 0x3c, node);
    g.write::<u32>(c5, w(0x3bc));
    for off in [0x9cu32, 0xa0, 0xa4] {
        g.write::<u32>(o(off), 0);
    }
    g.write::<u32>(o(0x24), c4);
    g.write::<u32>(o(0x28), c5);
    g.write::<u32>(o(0x2c), w(0x3e8));
    g.write::<u32>(o(0x1c), 0);
    Ret::eax(eax)
}

/// [`screen_setup_0152`] (1014 bytes, 1 overlay).
pub const SCREEN_SETUP_0152: Sig = Sig {
    len: 1014,
    hash: 0x98fa_a1a5_7d7d_d032,
    calls: &[],
    imports: &[],
};

/// `magic_0533`'s summon task offsets in `magic_0152`'s copy.
const MAP_0152_8980: &[(u32, u32)] = &[
    (0x0, 0x0),
    (0x2e1, 0x2e8),
    (0x2e4, 0x301),
    (0x3c9, 0x3e0),
    (0x3f1, 0x40e),
    (0x6a4, 0x6a4),
    (0xa38, 0xa39),
    (0xa57, 0xa57),
    (0xa9f, 0xa99),
    (0xaa3, 0xaa2),
    (0xac4, 0xac8),
    (0xafc, 0xb06),
    (0xaff, 0xb0b),
    (0xb09, 0xb1c),
    (0xb1d, 0xb49),
    (0xc69, 0xced),
];

/// `magic_0152`'s summon task (0x10008980, 4133 bytes; a task callback
/// `(A)`): [`summon_task_0533`](crate::r35::summon_task_0533) on its
/// layout (the model set-up its own copy,
/// [`build_node`](crate::m0152::build_node)), with slot 228 `(*G_B, 0xb9)`
/// after state 0's slot 3, slot 520
/// choosing state 0x3d's camera end (instead of state 0x64's test), and
/// in state 0x47 the model's slot (257 or 296 by a flag), slot 228 `(*G_B,
/// 0xb8)` and the screen object ([`screen_setup_0152`] on `R + 0x39c`).
pub fn summon_task_0152<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r35::summon_task_0533_with(
        g,
        img,
        fr,
        &crate::r35::Layout0533 {
            map: MAP_0152_8980,
            kind: 152,
        },
    )
}

/// [`summon_task_0152`] (the `rel32`s: the 0533 task's callees, the
/// copy's own model set-up and the screen set-up).
pub const SUMMON_TASK_0152: Sig = Sig {
    len: 4133,
    hash: 0x014a_1aae_e8ab_f382,
    calls: &[
        (0x9c, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xd5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xe9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xfe, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x11b, Callee::Sig(&crate::m0152::BUILD_NODE)),
        (0x14d, Callee::Sig(&crate::s4_small::LINK_NODE)),
        (0x163, Callee::Sig(&crate::sa_misc::STATIC_NODE)),
        (0x1a7, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x256, Callee::Sig(&crate::s7_misc::GLOBAL_DIFFERENCES)),
        (0x2c8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x2cd, Callee::Sig(&crate::s4_small::CALL_211_FLAG)),
        (0x60f, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x70f, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x724, Callee::Sig(&crate::s4_small::CALL_250)),
        (0xabf, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xce3, Callee::Sig(&SCREEN_SETUP_0152)),
        (0xe44, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0xf0f, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xf1a, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// `magic_0186`'s scene task (0x100077c0, 2669 bytes; a task callback
/// `(A)`, cdecl; an 8-aligned frame, `S = ((E - 4) & !7) - 0x28` the
/// frame below the three pushes, the record words at `S + 0xc..`). The
/// record `R = A[4]` keeps the state `R + 0x18` and the count `R + 0x2c`
/// (up at every exit; set to 1 where a state ends). By the state:
/// - 0 ([`scene_start_0186`]) runs on into 0xa;
/// - 0xa ([`scene_nodes_0186`]), 0xb after 0x18 calls
///   ([`scene_model_0186`]) and 0x14 once the byte at `+0x734` is set
///   ([`scene_parts_0186`]) build the scene's nodes;
/// - 0x15: slot 411 `(*G)`; once it answers the task goes on to 0x1e in
///   the same call;
/// - 0x1e frees the parts and runs on into 0x1f ([`scene_end_0186`]);
/// - 0x20 waits while the fade `*G_F` (the operand at `+0x5f`) is above 0
///   (it falls by the constant at `+0x89f`), then sets the end scene up
///   ([`scene_last_0186`]) and goes to 10000;
/// - 10000, after 60 calls, unlinks `A`'s node from its ring.
///
/// Returns `eax` as the original leaves it: the last call's, the table's
/// byte for the states 0..0x14 it does not handle, the state itself or
/// less 0x1f for the others.
pub fn scene_task_0186<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let up = |g: &mut G| {
        let c = g.read::<u32>(at(0x2c)).wrapping_add(1);
        g.write::<u32>(at(0x2c), c);
    };
    // mov [R + 0x18], ..; mov dword [R + 0x2c], 0; inc dword [R + 0x2c]
    let next = |g: &mut G, st: u32| {
        g.write::<u32>(at(0x18), st);
        g.write::<u32>(at(0x2c), 0);
        g.write::<u32>(at(0x2c), 1);
    };
    loop {
        let st = g.read::<u32>(at(0x18));
        match st {
            0x15 => {
                let x = g.read::<u32>(w(0x2c));
                // call width: 1
                let eax = g.call(slot_target(g, w(0x26), 411), 0, &[x]).eax;
                if eax == 0 {
                    up(g);
                    return Ret::eax(0);
                }
                g.write::<u32>(at(0x18), 0x1e);
            }
            0x20 => {
                let fade = g.read::<f32>(w(0x5f));
                // comiss 0, x; jb: above 0 or unordered waits
                if fade > 0.0 || fade.is_nan() {
                    let x = sub(fade, g.read::<f32>(w(0x89f)));
                    g.write::<f32>(w(0x8a7), x);
                    up(g);
                    return Ret::eax(st);
                }
                scene_last_0186(g, img, f, s);
                g.write::<u32>(at(0x18), 0x2710);
            }
            0x1e | 0x1f => {
                let eax = scene_end_0186(g, img, f, s, r, st == 0x1e);
                let s2 = g.read::<u32>(at(0x18)).wrapping_add(1);
                next(g, s2);
                return Ret::eax(eax);
            }
            0x16..=0x1d => {
                up(g);
                return Ret::eax(st.wrapping_sub(0x1f));
            }
            0x2710 => {
                if g.read::<i32>(at(0x2c)) >= 0x3c {
                    let n = g.read::<u32>(a);
                    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
                    g.write::<u32>(nx.wrapping_add(8), pv);
                    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
                    g.write::<u32>(pv.wrapping_add(4), nx);
                    g.write::<u32>(n + 8, n);
                    g.write::<u32>(n + 4, n);
                    up(g);
                    return Ret::eax(nx);
                }
                up(g);
                return Ret::eax(st);
            }
            0..=0x14 => {
                let idx = img.word(w(0x3dc).wrapping_add(st)) & 0xff;
                let eax = match st {
                    0 | 0xa => {
                        if st == 0 {
                            scene_start_0186(g, img, f, s);
                            g.write::<u32>(at(0x18), 0xa);
                        }
                        scene_nodes_0186(g, img, f, s)
                    }
                    0xb if g.read::<i32>(at(0x2c)) >= 0x18 => {
                        let eax = scene_model_0186(g, img, f, s);
                        // mov [R + 0x18], 0x14; inc [R + 0x2c] (no reset)
                        g.write::<u32>(at(0x18), 0x14);
                        up(g);
                        return Ret::eax(eax);
                    }
                    0x14 if g.read::<u8>(w(0x734)) != 0 => scene_parts_0186(g, img, f, s),
                    _ => {
                        up(g);
                        return Ret::eax(idx);
                    }
                };
                let s2 = g.read::<u32>(at(0x18)).wrapping_add(1);
                next(g, s2);
                return Ret::eax(eax);
            }
            _ => {
                up(g);
                return Ret::eax(st);
            }
        }
    }
}

/// Writes `words` from `at` on (a call's stack words or a frame record).
fn put_words<G: Game>(g: &mut G, at: u32, words: &[u32]) {
    for (i, &v) in (0u32..).zip(words) {
        g.write::<u32>(at + 4 * i, v);
    }
}

/// [`scene_task_0186`]'s calls of the `(ecx, a, _, _, d)` node set-ups
/// (`push d; sub esp, 8; push a` from the frame `S`): `a` at `S - 0x10`,
/// `d` at `S - 4`, the return address at `S - 0x14`.
fn frame_4<G: Game>(g: &mut G, img: &Image, at: u32, s: u32, ecx: u32, a: u32, d: u32) -> Frame {
    g.write::<u32>(s - 0x10, a);
    g.write::<u32>(s - 4, d);
    Frame {
        at: img.rel32_target(at),
        esp: s - 0x14,
        ecx,
        ..Frame::default()
    }
}

/// [`scene_task_0186`]'s state 0: the model nodes
/// ([`model_nodes`](crate::s3_summon::model_nodes)) of the group `[N, P,
/// *G, 0, 1]` at `S + 0x10` (`N`'s vtable stored first).
fn scene_start_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) {
    let w = |o: u32| img.word(f + o);
    let x = g.read::<u32>(w(0x3e8));
    let grp = s + 0x10;
    put_words(g, grp, &[w(0x3f4), w(0x3fc), x, 0]);
    g.write::<u8>(grp + 0x10, 1);
    crate::r25::abs_stores(g, img, f, 0x411, 0x41b);
    let mf = Frame {
        at: img.rel32_target(f + 0x41c),
        esp: s - 4,
        ecx: grp,
        ..Frame::default()
    };
    crate::s3_summon::model_nodes(g, img, &mf);
}

/// [`scene_task_0186`]'s state 0xa: three nodes appended under `P` (the
/// operand at `+0x428`; two [`node_append`](crate::sb_misc::node_append),
/// one [`sa_misc`'s](crate::sa_misc::node_append)) with their words, the
/// big node ([`node_init_big_table`](crate::m0070::node_init_big_table))
/// of `[N, P, A, B, C]` at `S + 0x10`, slot 215, the particle node and the
/// effect node. Returns the effect node's `eax`.
fn scene_nodes_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let p = w(0x428);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x432), w(0x42d), p, 0);
    abs(g, 0x43b, 0x463);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x464), w(0x437), p, 0);
    abs(g, 0x46d, 0x495);
    crate::sa_misc::node_append(g, img, img.rel32_target(f + 0x496), w(0x469), p, 0);
    // mov [esp + 0x1c], ecx: the appended node (the callee keeps ecx)
    g.write::<u32>(s + 0x1c, w(0x469));
    abs(g, 0x4a2, 0x4c0);
    g.write::<u32>(s + 0x10, w(0x4c4));
    g.write::<u32>(s + 0x14, p);
    g.write::<u32>(s + 0x18, w(0x4d0));
    g.write::<u32>(s + 0x20, w(0x4d8));
    crate::m0070::node_init_big_table(g, img, img.rel32_target(f + 0x4dd), s + 0x10);
    let x = g.read::<u32>(w(0x4e8));
    g.write::<u8>(w(0x4ee), 1);
    // call width: 1
    let h = g.call(t(g, 0x4e2, 215), 0, &[x]).eax;
    g.write::<u32>(w(0x509), h);
    abs(g, 0x512, 0x51c);
    let pf = frame_4(g, img, f + 0x51d, s, w(0x4ff), w(0x50e), 0);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    let ef = frame_4(g, img, f + 0x534, s, w(0x525), w(0x52f), 0);
    let eax = crate::s4_fam446::effect_node_b(g, img, &ef).eax;
    let fl = g.read::<u32>(w(0x53a)) | 1;
    g.write::<u32>(w(0x53a), fl);
    abs(g, 0x53f, 0x549);
    eax
}

/// [`scene_task_0186`]'s state 0xb: slot 228 `(*G, 0xba)`, the node copy
/// ([`node_copy`](crate::s4_small::node_copy)) of `[N, P, M]` at `S +
/// 0x10`, a particle node, the node `[N2, P, A, 10.0, 0.5]`
/// ([`node_abc`](crate::m0186::node_abc)), slot 325, two cleared nodes and
/// a model node, an appended node and an inserted one (on `S - 0x28`),
/// another particle node and an effect node. Returns the effect node's
/// `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn scene_model_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let x = g.read::<u32>(w(0x576));
    // call width: 2
    g.call(t(g, 0x56b, 228), 0, &[x, 0xba]);
    put_words(g, s + 0x10, &[w(0x58d), w(0x595), w(0x59d)]);
    abs(g, 0x5a1, 0x5ab);
    let cf = Frame {
        at: img.rel32_target(f + 0x5ac),
        esp: s - 4,
        ecx: s + 0x10,
        ..Frame::default()
    };
    crate::s4_small::node_copy(g, img, &cf);
    let pf = frame_4(g, img, f + 0x5c0, s, w(0x5b6), w(0x5bb), 3);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    // the particle node's four words stay: `[esp + 0x20]` is `S + 0x10`
    abs(g, 0x5c8, 0x5d2);
    put_words(
        g,
        s + 0x10,
        &[w(0x5d6), w(0x5de), w(0x5e6), 0x4120_0000, 0x3f00_0000],
    );
    abs(g, 0x5fa, 0x604);
    crate::m0186::node_abc(g, s + 0x10);
    // call width: 2
    let eax = g.call(t(g, 0x60a, 325), 0, &[w(0x61a), w(0x60f)]).eax;
    // slot 325's two words on the four: the callee's entry at `S - 0x1c`
    crate::m0186::cleared_node(g, img, img.rel32_target(f + 0x626), w(0x621), s - 0x1c, eax);
    let x = g.read::<u32>(w(0x631));
    abs(g, 0x635, 0x63f);
    // call width: 1
    let h = g.call(t(g, 0x62b, 215), 0, &[x]).eax;
    // add esp, 0x14: two of the particle node's words left; then (a, b,
    // ecx, h): word 2 is whatever ecx held (never read)
    put_words(g, s - 0x18, &[w(0x652), w(0x64d)]);
    g.write::<u32>(s - 0xc, h);
    let mf = Frame {
        at: img.rel32_target(f + 0x65c),
        esp: s - 0x1c,
        ecx: w(0x657),
        ..Frame::default()
    };
    let eax = crate::m0186::model_node(g, img, &mf).eax;
    abs(g, 0x66a, 0x671);
    let eax =
        crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x672), w(0x666), w(0x661), eax);
    abs(g, 0x67b, 0x6a3);
    crate::m0186::cleared_node(g, img, img.rel32_target(f + 0x6a4), w(0x677), s - 0x1c, eax);
    let v = g.read::<u32>(w(0x6a9));
    put_words(g, s - 0x28, &[w(0x6bd), w(0x6b8), w(0x6b3), w(0x6ae)]);
    abs(g, 0x6cb, 0x6d5);
    g.write::<u32>(w(0x6d6), v);
    let inf = Frame {
        at: img.rel32_target(f + 0x6db),
        esp: s - 0x2c,
        ecx: w(0x6c7),
        edx: w(0x6c2),
        ..Frame::default()
    };
    crate::sa_misc::insert_with(g, img, &inf);
    let pf = frame_4(g, img, f + 0x6f2, s, w(0x6e3), w(0x6ed), 2);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    let ef = frame_4(g, img, f + 0x709, s, w(0x6fa), w(0x704), 2);
    let eax = crate::s4_fam446::effect_node_b(g, img, &ef).eax;
    let fl = g.read::<u32>(w(0x712)) | 1;
    g.write::<u32>(w(0x712), fl);
    abs(g, 0x717, 0x721);
    eax
}

/// [`scene_task_0186`]'s state 0x14: slot 215, the nodes `[N, P, h,
/// 0x63]` ([`node_init`](crate::m0042::node_init)) and `[N2, P, A]`
/// ([`node_init`](crate::m0003::node_init)) at `S + 0x10`, an appended
/// node, the record node ([`record_node`](crate::l3_common::record_node))
/// of `[N3, that node, N2, N]`, another appended node and an inserted one,
/// a particle node and slot 239 `(*G, -5, 0)`. Returns slot 239's `eax`.
fn scene_parts_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let x = g.read::<u32>(w(0x746));
    g.write::<u32>(s + 0x10, w(0x754));
    g.write::<u32>(s + 0x14, w(0x75c));
    // call width: 1
    let h = g.call(t(g, 0x740, 215), 0, &[x]).eax;
    g.write::<u32>(s + 0x18, h);
    g.write::<u32>(s + 0x1c, 0x63);
    crate::m0042::node_init(g, img, img.rel32_target(f + 0x776), s + 0x10);
    put_words(g, s + 0x10, &[w(0x782), w(0x78a), w(0x792)]);
    crate::m0003::node_init(g, img, img.rel32_target(f + 0x797), s + 0x10);
    abs(g, 0x7a5, 0x7ac);
    crate::sa_misc::node_append(g, img, img.rel32_target(f + 0x7ad), w(0x7a1), w(0x79c), 0);
    // mov [esp + 0x14], ecx: the appended node
    g.write::<u32>(s + 0x14, w(0x7a1));
    g.write::<u32>(s + 0x10, w(0x7bd));
    g.write::<u32>(s + 0x18, w(0x7c5));
    g.write::<u32>(s + 0x1c, w(0x7cd));
    crate::l3_common::record_node(g, img, img.rel32_target(f + 0x7d2), s + 0x10);
    abs(g, 0x7e0, 0x808);
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x809), w(0x7dc), w(0x7d7), 0);
    // push ecx (the appended node: the callee keeps ecx and edx)
    put_words(g, s - 0x10, &[w(0x819), w(0x814), w(0x80f), w(0x7dc)]);
    abs(g, 0x822, 0x84a);
    let inf = Frame {
        at: img.rel32_target(f + 0x84b),
        esp: s - 0x14,
        ecx: w(0x81e),
        edx: w(0x7d7),
        ..Frame::default()
    };
    crate::sa_misc::insert_with(g, img, &inf);
    g.write::<u32>(s - 4, 1);
    abs(g, 0x85c, 0x863);
    g.write::<u32>(s - 0x10, w(0x864));
    let pf = Frame {
        at: img.rel32_target(f + 0x869),
        esp: s - 0x14,
        ecx: w(0x853),
        ..Frame::default()
    };
    crate::s3_nodes::particle_node_with(g, img, &pf);
    let x = g.read::<u32>(w(0x87e));
    // call width: 3
    g.call(t(g, 0x86e, 239), 0, &[x, 0xffff_fffb, 0]).eax
}

/// [`scene_task_0186`]'s states 0x1e and 0x1f. 0x1e (`first`): two nodes
/// leave ([`leave`](crate::sa_misc::leave)), unless bit 2 of `*G` the
/// cursor on the part at `S + 0xc` frees it
/// ([`free_then_unlink`](crate::nodes::free_then_unlink)), the part freed
/// ([`free_part`](crate::m0070::free_part)), slots 21 `(a, b, 1.0)` and
/// 213 `(c, 0)`, the state on with the count 0. 0x1f: an appended node
/// with its words, slots 215 and 1, slot 215 again and the model node
/// `(N, M, _, h)` ([`model_node`](crate::m0186::model_node)), the words
/// after it. Returns the model node's `eax`.
fn scene_end_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32, first: bool) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let mut eax = 0;
    if first {
        // sub eax, 0x1e: eax 0
        eax = crate::sa_misc::leave(g, w(0x8cc), 0);
        if g.read::<u8>(w(0x8d7)) & 4 == 0 {
            g.write::<u32>(s + 0xc, w(0x8e6));
            let ff = Frame {
                at: img.rel32_target(f + 0x8eb),
                esp: s - 4,
                ecx: s + 0xc,
                ..Frame::default()
            };
            eax = crate::nodes::free_then_unlink(g, img, &ff).eax;
        }
        eax = crate::m0070::free_part(g, img, img.rel32_target(f + 0x8f5), w(0x8f0), eax);
        eax = crate::sa_misc::leave(g, w(0x8fa), eax);
        let (a, b) = (g.read::<u32>(w(0x91b)), g.read::<u32>(w(0x915)));
        // call width: 3
        g.call(t(g, 0x904, 21), 0, &[a, b, ONE]);
        let c = g.read::<u32>(w(0x933));
        // call width: 2
        g.call(t(g, 0x922, 213), 0, &[c, 0]);
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x2c, 0);
    }
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x951), w(0x94c), w(0x947), eax);
    let x = g.read::<u32>(w(0x95c));
    abs(g, 0x960, 0x988);
    // call width: 1
    let h = g.call(t(g, 0x956, 215), 0, &[x]).eax;
    // call width: 1
    g.call(t(g, 0x995, 1), 0, &[h]);
    let x = g.read::<u32>(w(0x9a6));
    // call width: 1
    let h = g.call(t(g, 0x99f, 215), 0, &[x]).eax;
    // (a, b, ecx, h): word 2 is whatever ecx held (never read)
    put_words(g, s - 0x18, &[w(0x9bd), w(0x9b8)]);
    g.write::<u32>(s - 0xc, h);
    let mf = Frame {
        at: img.rel32_target(f + 0x9c7),
        esp: s - 0x1c,
        ecx: w(0x9c2),
        ..Frame::default()
    };
    let eax = crate::m0186::model_node(g, img, &mf).eax;
    abs(g, 0x9cb, 0x9fa);
    eax
}

/// [`scene_task_0186`]'s state 0x20 past its wait: slots 230 x3 and 325,
/// the node `*G_N` (the operand at `+0x10`) and a second node appended
/// under the parent (their words from the store runs), unless bit 2 of
/// `*G` the environment word and slot 713, the particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of the record at
/// `S + 0x10`, the node at `*G_L` leaving its parent (inlined), slot 21,
/// a third node appended with slot 719's handle.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn scene_last_0186<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    abs(g, 0x74, 0x7e);
    let mut words: Vec<u32> = Vec::new();
    for (op, n, a, b) in [
        (0x70u32, 230u32, 0x8au32, 0x85u32),
        (0x91, 230, 0xa1, 0x96),
        (0xa8, 230, 0xb8, 0xad),
        (0xbf, 325, 0xcf, 0xc4),
    ] {
        words.splice(0..0, [w(a), w(b)]);
        // call width: 8 (the earlier calls' words stay on the stack)
        g.call(t(g, op, n), 0, &words);
    }
    // the first node (ebx = the operand at `+0x10`) appended
    let last = gw(g, 0xd6);
    abs(g, 0xdd, 0x10e);
    g.write::<u32>(w(0x10f), last);
    if last == w(0x114) {
        abs(g, 0x11a, 0x124);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(0x129));
    }
    let mut count = g.read::<u16>(w(0x12f)).wrapping_add(1);
    // its words, then the second node after it
    abs(g, 0x135, 0x1ac);
    if w(0x10) == w(0x1ae) {
        abs(g, 0x1b4, 0x1be);
    } else {
        abs(g, 0x1c0, 0x1ca);
    }
    count = count.wrapping_add(1);
    let keep = g.read::<u8>(w(0x1ce)) & 4 != 0;
    abs(g, 0x1d3, 0x1dd);
    g.write::<u16>(w(0x1df), count);
    abs(g, 0x1e3, 0x212);
    if !keep {
        let p = gw(g, 0x215);
        let v = gw(g, 0x21b);
        g.write::<u32>(p, v);
        // call width: 1
        g.call(t(g, 0x222, 713), 0, &[w(0x227)]);
    }
    let x = gw(g, 0x237);
    put_words(g, s + 0x10, &[w(0x243), w(0x24b), w(0x253), x, w(0x25f), 4]);
    abs(g, 0x26b, 0x275);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x276), s + 0x10);
    // the node at `*G_L` leaves its parent
    let parent = gw(g, 0x27c);
    if parent != 0 {
        let n = g.read::<u16>(parent + 0x18).wrapping_add(0xffff);
        g.write::<u16>(parent + 0x18, n);
        let (prev, next) = (gw(g, 0x28f), gw(g, 0x294));
        if prev == parent {
            g.write::<u32>(parent + 0x10, next);
        } else {
            g.write::<u32>(prev.wrapping_add(8), next);
        }
        let (next, prev) = (gw(g, 0x2a6), gw(g, 0x2ab));
        if next == parent {
            g.write::<u32>(parent + 0xc, prev);
        } else {
            g.write::<u32>(next.wrapping_add(4), prev);
        }
        abs(g, 0x2bb, 0x2d9);
    }
    let (a, b) = (gw(g, 0x2f1), gw(g, 0x2eb));
    // call width: 3
    g.call(t(g, 0x2da, 21), 0, &[a, b, ONE]);
    // the third node appended
    let last = gw(g, 0x2f8);
    abs(g, 0x2ff, 0x330);
    g.write::<u32>(w(0x331), last);
    if last == w(0x336) {
        abs(g, 0x33c, 0x346);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(0x34b));
    }
    let n = g.read::<u16>(w(0x357)).wrapping_add(1);
    g.write::<u16>(w(0x357), n);
    abs(g, 0x35d, 0x367);
    // call width: 3
    let h = g.call(t(g, 0x350, 719), 0, &[w(0x370), 6, 1]).eax;
    g.write::<u32>(w(0x377), h);
    abs(g, 0x37b, 0x3c1);
}

/// [`scene_task_0186`] (the `rel32`s: its callees in order).
pub const SCENE_TASK_0186: Sig = Sig {
    len: 2669,
    hash: 0x8510_dd02_ea59_06c9,
    calls: &[
        (0x276, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x41c, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x432, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x464, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x496, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x4dd, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x51d, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x534, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x5ac, Callee::Sig(&crate::s4_small::NODE_COPY)),
        (0x5c0, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x605, Callee::Sig(&crate::m0186::NODE_ABC)),
        (0x626, Callee::Sig(&crate::m0186::CLEARED_NODE)),
        (0x65c, Callee::Sig(&crate::m0186::MODEL_NODE)),
        (0x672, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6a4, Callee::Sig(&crate::m0186::CLEARED_NODE)),
        (0x6db, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x6f2, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x709, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x776, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0x797, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x7ad, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x7d2, Callee::Sig(&crate::l3_common::RECORD_NODE)),
        (0x809, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x84b, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x869, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x8d1, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x8eb, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x8f5, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x8ff, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x951, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x9c7, Callee::Sig(&crate::m0186::MODEL_NODE)),
    ],
    imports: &[],
};

/// `magic_0178`'s summon task (0x10007020, 217 bytes; a task callback
/// `(A)`, cdecl). By the state halfword `*G_S` (the operand at `+0x6`):
/// - 0: slot 696 `(*G, 0xb2)`, the timer `*G_T` 0, the state 2; returns 2;
/// - 2: the timer up by the constant at `+0x80`; once it is not below the
///   one at `+0x87` the scene is set up ([`scene_0178`]) and the state is
///   3 (returns 3); else returns 0;
/// - 3: slot 240 `(*G)`; once it answers, slot 213 `(*G, 0)` and `A`'s
///   node leaves its ring (returns the node's next, read again), else 0;
/// - any other: returns the state less 3.
pub fn summon_task_0178<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let st = u32::from(g.read::<u16>(w(0x6)));
    match st {
        0 => {
            let x = g.read::<u32>(w(0xb3));
            // call width: 2
            g.call(t(g, 0xa8, 696), 0, &[x, 0xb2]);
            crate::r25::abs_stores(g, img, f, 0xc4, 0xce);
            g.write::<u16>(w(0xd0), 2);
            Ret::eax(2)
        }
        2 => {
            let x = add(g.read::<f32>(w(0x78)), g.read::<f32>(w(0x80)));
            let k = g.read::<f32>(w(0x87));
            g.write::<f32>(w(0x8f), x);
            // comiss x, K; jb: below or unordered waits
            if x >= k {
                // push ebp; call: the callee's entry `esp` 8 below
                scene_0178(g, img, img.rel32_target(f + 0x96), fr.esp.wrapping_sub(8));
                g.write::<u16>(w(0xa1), 3);
                Ret::eax(3)
            } else {
                Ret::eax(0)
            }
        }
        3 => {
            let x = g.read::<u32>(w(0x26));
            // call width: 1
            if g.call(t(g, 0x20, 240), 0, &[x]).eax == 0 {
                return Ret::eax(0);
            }
            let x = g.read::<u32>(w(0x46));
            // call width: 2
            g.call(t(g, 0x3e, 213), 0, &[x, 0]);
            let n = g.read::<u32>(fr.arg(g, 0));
            let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
            g.write::<u32>(nx.wrapping_add(8), pv);
            let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
            g.write::<u32>(pv.wrapping_add(4), nx);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            Ret::eax(nx)
        }
        _ => Ret::eax(st.wrapping_sub(3)),
    }
}

/// [`summon_task_0178`] (the `rel32`: the scene set-up).
pub const SUMMON_TASK_0178: Sig = Sig {
    len: 217,
    hash: 0x92d2_b9c6_ba8c_57ea,
    calls: &[(0x96, Callee::Sig(&SCENE_0178))],
    imports: &[],
};

/// A node appended under the parent `P` as `magic_0178` inlines it: the
/// store run `run` (the node's pass halfword, child list, parent and
/// link), `+8` the parent's last (operand `lb`, the value `last`), then
/// `P`'s first (the store run at `first`) when `last` is `P` (operand
/// `cmp`), else `last + 4` = the node (operand `ll`).
fn append_0178<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    last: u32,
    run: (u32, u32),
    [lb, cmp, first, ll]: [u32; 4],
) {
    let w = |o: u32| img.word(f + o);
    crate::r25::abs_stores(g, img, f, run.0, run.1);
    g.write::<u32>(w(lb), last);
    if last == w(cmp) {
        crate::r25::abs_stores(g, img, f, first, first + 10);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(ll));
    }
}

/// `magic_0178`'s scene set-up (0x100069a0, 1654 bytes; internal, no
/// arguments; an 8-aligned /GS frame, `S = ((E - 4) & !7) - 0x38` with
/// `E` the entry `esp`): under the DLL's parent `P` the nodes are
/// appended one after the other with their words (the store runs in the
/// code), slot 229 places the first `(V, M, S + 0x24)`, the second is
/// inserted ([`node_insert`](crate::nodes::node_insert)) with the group
/// at `S + 4`, a particle node ([`particle_node`](crate::sb_nodes::particle_node)),
/// the third copies the first's two words `+0x1c..+0x24`, the fourth,
/// fifth and sixth (the sixth under the fifth) and another insert, slot
/// 325, slot 215 `(*G_B)` into slot 12's float for the throw
/// ([`throw_aim`](crate::s5_misc::throw_aim)) of the record at `S + 4`,
/// slot 719's handle in a node under the second list, the static node
/// (the group task as its callback), and two effect nodes
/// ([`effect_node`](crate::sb_nodes::effect_node)) given their vtable.
/// Returns the last effect node's node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn scene_0178<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let s = (esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let count_up = |g: &mut G, op: u32| {
        let n = g.read::<u16>(w(op)).wrapping_add(1);
        g.write::<u16>(w(op), n);
    };
    // the first node, placed by slot 229
    let last = gw(g, 0x15);
    append_0178(g, img, f, last, (0x19, 0x4a), [0x4b, 0x50, 0x56, 0x65]);
    count_up(g, 0x6c);
    abs(g, 0x7a, 0x8e);
    put_words(g, s + 0x24, &[0, 0xc0f0_0000, 0xc120_0000, ONE]);
    // call width: 3
    g.call(t(g, 0x76, 229), 0, &[w(0x9a), w(0x95), s + 0x24]);
    // the second node, inserted
    let last = gw(g, 0xc1);
    append_0178(g, img, f, last, (0xc8, 0xf9), [0xfa, 0xff, 0x105, 0x114]);
    count_up(g, 0x11b);
    abs(g, 0x123, 0x15f);
    let grp = s + 4;
    let ins = |g: &mut G, op: u32, words: &[u32]| {
        put_words(g, grp, words);
        let fr = Frame {
            at: img.rel32_target(f + op),
            esp: s - 4,
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &fr);
    };
    ins(
        g,
        0x190,
        &[w(0x163), w(0x16b), w(0x173), w(0x17b), w(0x183), w(0x18b)],
    );
    // a particle node
    let x = gw(g, 0x195);
    put_words(g, grp, &[w(0x1a1), w(0x1a9), w(0x1b1), x, w(0x1bd), 1]);
    abs(g, 0x1c9, 0x1d3);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x1d4), grp);
    // the third node (the first's two words copied), the fourth after it
    let last = gw(g, 0x1d9);
    append_0178(
        g,
        img,
        f,
        last,
        (0x1dd, 0x20e),
        [0x20f, 0x214, 0x21a, 0x229],
    );
    for (from, to) in [(0x231u32, 0x23fu32), (0x247, 0x260)] {
        let v = g.read::<u64>(w(from));
        g.write::<u64>(w(to), v);
    }
    let mut count = g.read::<u16>(w(0x237)).wrapping_add(1);
    abs(g, 0x252, 0x25c);
    abs(g, 0x264, 0x29f);
    let third = w(0x24c);
    if third == w(0x2a1) {
        abs(g, 0x2a7, 0x2b1);
    } else {
        abs(g, 0x2b3, 0x2bd);
    }
    // the fifth after the fourth
    count = count.wrapping_add(1);
    abs(g, 0x2c4, 0x327);
    let fourth = w(0x2be);
    if fourth == w(0x329) {
        abs(g, 0x32f, 0x339);
    } else {
        abs(g, 0x33b, 0x345);
    }
    count_up(g, 0x348);
    count = count.wrapping_add(1);
    abs(g, 0x352, 0x35c);
    g.write::<u16>(w(0x35e), count);
    // the sixth under the fifth, its words; the insert
    abs(g, 0x362, 0x3ed);
    ins(
        g,
        0x41e,
        &[w(0x3f1), w(0x3f9), w(0x401), w(0x409), w(0x411), w(0x419)],
    );
    // call width: 2
    g.call(t(g, 0x423, 325), 0, &[w(0x433), w(0x428)]);
    let b = u32::from(g.read::<u8>(w(0x43c)));
    // call width: 1 (slot 325's words stay under it)
    let h = g.call(t(g, 0x442, 215), 0, &[b]).eax;
    // the throw's record at `S + 4`: slot 12's float its fourth word
    g.write::<u32>(s + 4, w(0x455));
    g.write::<u32>(s + 8, w(0x463));
    g.write::<u32>(s + 0xc, w(0x46e));
    // call width: 1
    let y = st0_f32(g.call(t(g, 0x45b, 12), 0, &[h]).st0);
    g.write::<f32>(s + 0x10, y);
    put_words(g, s + 0x14, &[0x3f21_47ae, 0, 0x4040_0000, w(0x49b)]);
    let af = Frame {
        at: img.rel32_target(f + 0x4a0),
        esp: s - 4,
        ecx: grp,
        ..Frame::default()
    };
    crate::s5_misc::throw_aim(g, img, &af);
    abs(g, 0x4ab, 0x4c9);
    // call width: 3
    let h = g.call(t(g, 0x4a5, 719), 0, &[w(0x4d2), 4, 0]).eax;
    // a node under the second list holding slot 719's handle
    let last = gw(g, 0x4da);
    append_0178(
        g,
        img,
        f,
        last,
        (0x4e1, 0x512),
        [0x514, 0x51a, 0x520, 0x52f],
    );
    count_up(g, 0x536);
    g.write::<u32>(w(0x53b), h);
    // the static node: its words, after the static list's last
    let last = gw(g, 0x540);
    g.write::<u32>(w(0x545), last);
    abs(g, 0x549, 0x599);
    g.write::<u32>(last.wrapping_add(4), w(0x59c));
    let fl = gw(g, 0x5a2) & !1;
    g.write::<u32>(w(0x5a2), fl);
    abs(g, 0x5ad, 0x5b7);
    g.write::<u16>(w(0x5b9), 0);
    // the two effect nodes
    let mut node = 0;
    for (words, call, vt, flag, run) in [
        (
            [w(0x5c1), w(0x5c9), w(0x5d1), w(0x5d9), w(0x5e1), 0],
            0x5ea,
            0x5f8,
            0x5fe,
            (0x603, 0x60d),
        ),
        (
            [w(0x611), w(0x619), w(0x621), w(0x629), w(0x631), 2],
            0x63e,
            0x64c,
            0x652,
            (0x659, 0x66d),
        ),
    ] {
        put_words(g, grp, &words);
        crate::sb_nodes::effect_node(g, img, img.rel32_target(f + call), grp);
        node = g.read::<u32>(grp);
        g.write::<u32>(node, w(vt));
        let x = gw(g, flag) | 1;
        g.write::<u32>(w(flag), x);
        abs(g, run.0, run.1);
    }
    node
}

/// [`scene_0178`] (the `rel32`s: its callees, `__security_check_cookie`).
pub const SCENE_0178: Sig = Sig {
    len: 1654,
    hash: 0x13f6_1145_5034_a812,
    calls: &[
        (0x190, Callee::Sig(&crate::nodes::INSERT)),
        (0x1d4, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x41e, Callee::Sig(&crate::nodes::INSERT)),
        (0x4a0, Callee::Sig(&crate::s5_misc::THROW_AIM)),
        (0x5ea, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x63e, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x66e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0178`'s start-up (0x10007100, 2124 bytes; a table entry, cdecl
/// `(actor)`; an 8-aligned /GS frame, `S = ((E - 4) & !7) - 0x38` below
/// `esi`): the targets ([`targets_init_b`](crate::s4_summon::targets_init_b)),
/// the static list and the parent `P` set up from the store runs, two
/// model nodes ([`model_node_init`](crate::sa_summon::model_node_init))
/// of `[N, P, *G_O, 0]` and `[N2, P, *G_B, 0]` at `S + 0xc`, three nodes
/// appended under `P` and inserted
/// ([`node_insert`](crate::nodes::node_insert)) with the group at `S +
/// 0x1c`, a fourth with slot 268 `(*G, 0x11, Q)`, three more and another
/// insert; the actors' flat angle (slot 217 twice, slot 234, the
/// quadrant atan) into slot 666; a node with slots 718 and 719, slot 711
/// with the lock word set (and put back), slots 710, 222, 221, 719 and
/// 551; the static node with the summon task as its callback; the state
/// halfword 0. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0178<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let actor = fr.arg(g, 0);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    crate::s4_summon::targets_init_b(g, img, call(0x19), s - 4, actor);
    let o = gw(g, 0x1e);
    abs(g, 0x26, 0xc4);
    let rec = s + 0xc;
    put_words(g, rec, &[w(0xc8), w(0xd0), o]);
    g.write::<u8>(rec + 0xc, 0);
    abs(g, 0xdd, 0xe7);
    crate::sa_summon::model_node_init(g, img, call(0xe8), rec);
    let b = u32::from(g.read::<u8>(w(0xef)));
    put_words(g, rec, &[w(0xfb), w(0x103), b]);
    g.write::<u8>(rec + 0xc, 0);
    abs(g, 0x110, 0x11a);
    crate::sa_summon::model_node_init(g, img, call(0x11b), rec);
    // three nodes under `P`, each after the one before
    let last = gw(g, 0x120);
    append_0178(
        g,
        img,
        f,
        last,
        (0x124, 0x155),
        [0x156, 0x15b, 0x161, 0x170],
    );
    let mut count = g.read::<u16>(w(0x176)).wrapping_add(1);
    abs(g, 0x181, 0x1d7);
    if w(0x17b) == w(0x1d9) {
        abs(g, 0x1df, 0x1e9);
    } else {
        abs(g, 0x1eb, 0x1f5);
    }
    count = count.wrapping_add(1);
    abs(g, 0x1fc, 0x269);
    if w(0x1f6) == w(0x26b) {
        abs(g, 0x271, 0x27b);
    } else {
        abs(g, 0x27d, 0x287);
    }
    count = count.wrapping_add(1);
    g.write::<u16>(w(0x28b), count);
    let al = g.read::<u8>(w(0x290));
    abs(g, 0x298, 0x2d4);
    for op in [0x2d5u32, 0x2da, 0x2df] {
        g.write::<u8>(w(op), al);
    }
    let grp = s + 0x1c;
    let ins = |g: &mut G, op: u32, words: &[u32]| {
        put_words(g, grp, words);
        let fr = Frame {
            at: img.rel32_target(f + op),
            esp: s - 4,
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &fr);
    };
    ins(
        g,
        0x314,
        &[w(0x2e7), w(0x2ef), w(0x2f7), w(0x2ff), w(0x307), w(0x30f)],
    );
    // the fourth with slot 268
    let last = gw(g, 0x319);
    append_0178(
        g,
        img,
        f,
        last,
        (0x31d, 0x34e),
        [0x34f, 0x354, 0x35a, 0x369],
    );
    let n = g.read::<u16>(w(0x375)).wrapping_add(1);
    g.write::<u16>(w(0x375), n);
    let x = gw(g, 0x382);
    abs(g, 0x386, 0x39a);
    // call width: 3
    g.call(t(g, 0x36e, 268), 0, &[x, 0x11, w(0x37a)]);
    // three more and the insert
    let last = gw(g, 0x3a3);
    append_0178(
        g,
        img,
        f,
        last,
        (0x3aa, 0x3e5),
        [0x3e6, 0x3eb, 0x3f1, 0x400],
    );
    let mut count = g.read::<u16>(w(0x406)).wrapping_add(1);
    abs(g, 0x411, 0x47e);
    if w(0x40b) == w(0x480) {
        abs(g, 0x486, 0x490);
    } else {
        abs(g, 0x492, 0x49c);
    }
    count = count.wrapping_add(1);
    g.write::<u16>(w(0x4a0), count);
    let al = g.read::<u8>(w(0x4a5));
    abs(g, 0x4ad, 0x4e9);
    for op in [0x4eau32, 0x4ef] {
        g.write::<u8>(w(op), al);
    }
    ins(
        g,
        0x524,
        &[w(0x4f7), w(0x4ff), w(0x507), w(0x50f), w(0x517), w(0x51f)],
    );
    // the flat angle from the summon's actor to the caster
    let (p, q) = (s + 0xc, s + 0x1c);
    let a = gw(g, 0x534);
    // call width: 2
    g.call(t(g, 0x52e, 217), 0, &[a, p]);
    let b = u32::from(g.read::<u8>(w(0x548)));
    // call width: 2
    g.call(t(g, 0x54e, 217), 0, &[b, q]);
    // call width: 3
    g.call(t(g, 0x56a, 234), 0, &[p, q, p]);
    let (x, z) = (g.read::<f32>(p), g.read::<f32>(p + 8));
    let k = [0x5a3, 0x5d9, 0x5e3, 0x620, 0x638, 0x642, 0x64c];
    let angle = crate::r40::flat_angle(g, img, f, x, z, k);
    let m = gw(g, 0x663);
    // call width: 2
    g.call(t(g, 0x651, 666), 0, &[m, angle.to_bits()]);
    // a node with slots 718 and 719
    let last = gw(g, 0x66a);
    append_0178(
        g,
        img,
        f,
        last,
        (0x671, 0x6a2),
        [0x6a3, 0x6a8, 0x6ae, 0x6bd],
    );
    let n = g.read::<u16>(w(0x6c9)).wrapping_add(1);
    g.write::<u16>(w(0x6c9), n);
    abs(g, 0x6cd, 0x709);
    // call width: 1
    g.call(t(g, 0x6c2, 718), 0, &[w(0x710)]);
    // call width: 3
    let h = g.call(t(g, 0x717, 719), 0, &[w(0x729), 8, 0]).eax;
    // slot 711 with the lock word 1, then put back
    let lock = gw(g, 0x731);
    let old = g.read::<u32>(lock);
    g.write::<u32>(w(0x73d), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x748, 711), 0, &[h, w(0x742), 0]);
    let lock = gw(g, 0x75b);
    g.write::<u32>(lock, old);
    let y = gw(g, 0x77b);
    // call width: 4
    g.call(t(g, 0x765, 710), 0, &[w(0x780), y, w(0x775), 0x0010_0000]);
    // call width: 4
    g.call(t(g, 0x787, 222), 0, &[w(0x79b), 0, 0x3e8, 2]);
    // call width: 4
    g.call(t(g, 0x7a2, 221), 0, &[w(0x7b6), 0, 0x3e8, 2]);
    // call width: 3
    let r = g.call(t(g, 0x7bd, 719), 0, &[w(0x7cc), 6, 2]).eax;
    // call width: 3
    g.call(t(g, 0x7db, 551), 0, &[r, 0, 2]);
    // the static node: the summon task its callback
    let last = gw(g, 0x7e8);
    g.write::<u32>(w(0x7f4), last);
    abs(g, 0x7f8, 0x820);
    g.write::<u32>(last.wrapping_add(4), w(0x823));
    let fl = gw(g, 0x829) & !1;
    g.write::<u32>(w(0x829), fl);
    abs(g, 0x831, 0x83b);
    g.write::<u16>(w(0x83d), 0);
    Ret::eax(0)
}

/// [`start_up_0178`] (the `rel32`s: its callees, `__security_check_cookie`).
pub const START_UP_0178: Sig = Sig {
    len: 2124,
    hash: 0x0d14_e589_4973_c48a,
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

/// `magic_0203`'s summon end task (0x100079b0, 1042 bytes; a task callback
/// `(A)`, cdecl; the record `R = A[4]`, its state `R + 0x48c`, its count
/// `R + 0x480` up at every exit). State 0: the timer `R + 0x38` grows by
/// the step `R + 0x474` (which grows by the constant at `+0x43`); below
/// the constant at `+0x57` it waits. Then the part `P = R + 0x360` is freed
/// (the environment word and slot 713, unconditionally), leaves its
/// parent ([`leave_parent`](crate::nodes::leave_parent)) and is freed again
/// unless bit 2 of `P + 0xe8`; a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[P, L, R + 0xc8,
/// *G_E, Q, 3]` at `S + 0x10` (`P`'s vtable stored first); the colour
/// block's flag and words from `R + 0x468`; for each of the `R + 0x488`
/// parts whose bit is set in slot 324's mask, slot 388 `(part + 0x50,
/// T[slot 324 & 1], 1)`; the state 10000, on at once. State 10000 with bit
/// 2 of `R + 0x448`: six nodes leave their parents, the state 20000 and
/// `A`'s node leaves its ring. Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn end_task_0203<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let up = |g: &mut G| {
        let c = g.read::<u32>(at(0x480)).wrapping_add(1);
        g.write::<u32>(at(0x480), c);
    };
    let st = g.read::<u32>(at(0x48c));
    let mut eax = st;
    if st == 0 {
        let x = add(rf(g, 0x474), rf(g, 0x38));
        g.write::<f32>(at(0x38), x);
        let step = add(rf(g, 0x474), k(g, 0x43));
        g.write::<f32>(at(0x474), step);
        // comiss x, K; jb: below or unordered waits
        let (x, end) = (rf(g, 0x38), k(g, 0x57));
        if x < end || x.is_nan() || end.is_nan() {
            up(g);
            return Ret::eax(0);
        }
        g.write::<u32>(at(0xc0), 0);
        g.write::<u32>(at(0x38), 0x3dcc_cccd);
        let p = at(0x360);
        let env = g.read::<u32>(w(0x68));
        g.write::<u32>(env, g.read::<u32>(at(0x3b0)));
        // call width: 1
        g.call(t(g, 0x86, 713), 0, &[p + 0x60]);
        let parent = g.read::<u32>(p + 0x14);
        if parent != 0 {
            crate::nodes::leave_parent(g, p, parent);
        }
        if g.read::<u8>(p + 0xe8) & 4 == 0 {
            let env = g.read::<u32>(w(0xe0));
            g.write::<u32>(env, g.read::<u32>(p + 0x50));
            // call width: 1
            g.call(t(g, 0xee, 713), 0, &[p + 0x60]);
        }
        let e = g.read::<u32>(w(0x108));
        put_words(g, s + 0x10, &[p, w(0x118), at(0xc8), e, w(0x124), 3]);
        g.write::<u32>(p, w(0x132));
        crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x137), s + 0x10);
        // the colour block
        let x = rf(g, 0x468);
        let fl = g.read::<u8>(w(0x14a)) | 1;
        g.write::<u8>(w(0x14a), fl);
        let c1 = sub(add(mul(x, k(g, 0x156)), k(g, 0x166)), k(g, 0x194));
        let c2 = sub(add(mul(x, k(g, 0x15e)), k(g, 0x178)), k(g, 0x19c));
        crate::r25::abs_stores(g, img, f, 0x16a, 0x174);
        crate::r25::abs_stores(g, img, f, 0x17c, 0x190);
        g.write::<f32>(w(0x1a4), c1);
        g.write::<f32>(w(0x1ac), c2);
        // call width: 0
        let mask = g.call(t(g, 0x144, 324), 0, &[]).eax;
        eax = mask;
        let mut i = 0i32;
        // the count read again at every turn
        while i < g.read::<i32>(at(0x488)) {
            // sar edx, cl: the count taken modulo 32
            if (mask.cast_signed() >> (i & 31)) & 1 != 0 {
                // call width: 0
                let c = g.call(t(g, 0x1d0, 324), 0, &[]).eax & 1;
                let tex = g.read::<u32>(w(0x1e4).wrapping_add(4 * c));
                let part =
                    g.read::<u32>(g.read::<u32>(at(0x460)).wrapping_add(4 * i.cast_unsigned()));
                let x = g.read::<u32>(part.wrapping_add(0x50));
                // call width: 3
                eax = g.call(t(g, 0x1f5, 388), 0, &[x, tex, 1]).eax;
            }
            i += 1;
        }
        g.write::<u32>(at(0x48c), 0x2710);
    } else if st != 0x2710 {
        up(g);
        return Ret::eax(st);
    }
    if g.read::<u8>(at(0x448)) & 4 == 0 {
        up(g);
        return Ret::eax(eax);
    }
    for node in [0x18u32, 0x44, 0x9c, 0x70, 0xc8, 0x214] {
        let node = at(node);
        let parent = g.read::<u32>(node + 0x14);
        if parent != 0 {
            crate::nodes::leave_parent(g, node, parent);
        }
    }
    g.write::<u32>(at(0x48c), 0x4e20);
    let n = g.read::<u32>(a);
    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
    g.write::<u32>(nx.wrapping_add(8), pv);
    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
    g.write::<u32>(pv.wrapping_add(4), nx);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    up(g);
    Ret::eax(nx)
}

/// [`end_task_0203`] (the `rel32`: the particle node).
pub const END_TASK_0203: Sig = Sig {
    len: 1042,
    hash: 0x04c9_51d8_ab4f_81ea,
    calls: &[(0x137, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE))],
    imports: &[],
};

/// `magic_0203`'s summon set-up (0x10007de0, 935 bytes; internal: `ecx` =
/// the record `O`, `edx` a vector, stack `(t, parts, count)`; an `ebp`
/// frame, the group at `ebp - 0x1c`): `O` on the static list with the end
/// task ([`end_task_0203`]) as its callback, `O + 0x460` = `parts`, `O +
/// 0x488` = `count`; the node `O + 0x44` under the parent (its vtable
/// after), slot 325 `(O + 0x60, edx)` and the place moved by two of slot
/// 307's floats (each also left in the caller's word `[ebp + 0xc]`); the
/// node `O + 0x9c` (vtable first), `O + 0xbc` = `t - 0x4000`, `O + 0xc0`
/// from slot 324's bit 10; inserts ([`node_insert`](crate::nodes::node_insert))
/// of `O + 0x214` and `O + 0xc8` with the nodes `O + 0x18` and `O + 0x70`
/// set up between, slot 307 once more for `O + 0x468`, the colour and the
/// step `O + 0x474` from it, and the particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) `O + 0x360`; the
/// state 0. Returns the particle node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0203<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::sa_misc::child_of_0512 as child;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let o = fr.ecx;
    let at = |off: u32| o.wrapping_add(off);
    let rf = |g: &G, off: u32| g.read::<f32>(at(off));
    // [ebp + 0xc]: the caller's second word, the floats' scratch
    let word1 = fr.esp + 8;
    let grp = fr.esp.wrapping_sub(0x20);
    // the record on the static list
    g.write::<u32>(at(8), o);
    g.write::<u32>(at(4), o);
    g.write::<u32>(o, w(0x13));
    g.write::<u32>(at(0x10), w(0x1a));
    g.write::<u32>(at(0x14), o);
    let last = g.read::<u32>(w(0x22));
    g.write::<u32>(at(8), last);
    g.write::<u32>(at(4), w(0x2c));
    let last = g.read::<u32>(w(0x31));
    g.write::<u32>(last.wrapping_add(4), o);
    let fl = g.read::<u32>(w(0x3d)) & !1;
    g.write::<u32>(w(0x3d), fl);
    let (t0, parts, count) = (fr.arg(g, 0), fr.arg(g, 1), fr.arg(g, 2));
    g.write::<u32>(w(0x47), o);
    g.write::<u32>(at(0x460), parts);
    g.write::<u32>(at(0x488), count);
    // the node `O + 0x44`
    let b = at(0x44);
    child(
        g,
        img,
        f,
        b,
        [0x69, 0x70, 0x75, 0x7d, 0x82, 0x8a, 0x96, 0xa1],
    );
    g.write::<u32>(b, w(0xa8));
    let v = at(0x60);
    // call width: 2
    g.call(t(g, 0x9b, 325), 0, &[v, fr.edx]);
    // call width: 0
    let x = st0_f32(g.call(t(g, 0xb9, 307), 0, &[]).st0);
    g.write::<f32>(word1, x);
    let y = add(mul(sub(x, k(g, 0xd4)), k(g, 0xe1)), rf(g, 0x60));
    g.write::<f32>(v, y);
    g.write::<f32>(at(0x64), sub(rf(g, 0x64), k(g, 0xf6)));
    // call width: 0
    let x = st0_f32(g.call(t(g, 0xd9, 307), 0, &[]).st0);
    g.write::<f32>(word1, x);
    g.write::<u32>(at(0x6c), ONE);
    let y = add(mul(sub(x, k(g, 0x113)), k(g, 0x128)), rf(g, 0x68));
    g.write::<f32>(at(0x68), y);
    // the node `O + 0x9c`
    let c = at(0x9c);
    g.write::<u32>(c, w(0x138));
    child(
        g,
        img,
        f,
        c,
        [0x14b, 0x152, 0x157, 0x15f, 0x164, 0x16c, 0x178, 0x186],
    );
    g.write::<u32>(at(0xbc), t0.wrapping_add(0xffff_c000));
    g.write::<u32>(at(0xb8), 0);
    // call width: 0
    let r = g.call(t(g, 0x191, 324), 0, &[]).eax;
    g.write::<u32>(at(0xc0), (r & 0x400).wrapping_add(0x51c));
    let ins = |g: &mut G, op: u32, words: [u32; 6]| {
        put_words(g, grp, &words);
        let fr = Frame {
            at: img.rel32_target(f + op),
            esp: fr.esp.wrapping_sub(0x34),
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &fr);
    };
    ins(g, 0x1df, [at(0x214), w(0x1c6), w(0x1cd), b, c, w(0x1da)]);
    // the node `O + 0x18`
    let d = at(0x18);
    child(
        g,
        img,
        f,
        d,
        [0x1f5, 0x1fc, 0x201, 0x209, 0x20e, 0x216, 0x222, 0x228],
    );
    g.write::<u32>(d, w(0x22e));
    g.write::<u32>(at(0x464), g.read::<u32>(at(0x64)));
    for (off, val) in [(0x34u32, 0u32), (0x38, 0xc348_0000), (0x3c, 0), (0x40, ONE)] {
        g.write::<u32>(at(off), val);
    }
    // call width: 0
    let x = st0_f32(g.call(t(g, 0x23c, 307), 0, &[]).st0);
    g.write::<f32>(word1, x);
    g.write::<f32>(at(0x468), sub(x, k(g, 0x270)));
    // the node `O + 0x70`
    let e = at(0x70);
    child(
        g,
        img,
        f,
        e,
        [0x28e, 0x295, 0x29a, 0x2a2, 0x2a7, 0x2af, 0x2bb, 0x2c1],
    );
    g.write::<u32>(e, w(0x2c7));
    for off in [0x8cu32, 0x90, 0x94] {
        g.write::<u32>(at(off), ONE);
    }
    let z = add(mul(rf(g, 0x468), k(g, 0x2f5)), k(g, 0x306));
    g.write::<f32>(at(0x98), z);
    ins(g, 0x333, [at(0xc8), w(0x319), at(0x214), d, w(0x32e), e]);
    let s = mul(sub(k(g, 0x33b), rf(g, 0x468)), k(g, 0x360));
    g.write::<f32>(at(0x474), add(s, k(g, 0x36e)));
    // the particle node `O + 0x360`
    let env = g.read::<u32>(w(0x348));
    put_words(g, grp, &[at(0x360), w(0x358), at(0xc8), env, w(0x375), 2]);
    g.write::<u32>(at(0x360), w(0x38a));
    let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x392), grp);
    g.write::<u32>(at(0x48c), 0);
    eax
}

/// [`summon_setup_0203`] (the `rel32`s: the inserts and the particle
/// node).
pub const SUMMON_SETUP_0203: Sig = Sig {
    len: 935,
    hash: 0x6cce_56be_8dcc_02e7,
    calls: &[
        (0x1df, Callee::Sig(&crate::nodes::INSERT)),
        (0x333, Callee::Sig(&crate::nodes::INSERT)),
        (0x392, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};
