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
