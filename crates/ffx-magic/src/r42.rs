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
