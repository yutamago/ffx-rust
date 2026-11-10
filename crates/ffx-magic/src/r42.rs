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

/// `magic_0203`'s summon task (0x10008190, 2950 bytes; a task callback
/// `(A)`, cdecl; an 8-aligned frame, `S = ((E - 4) & !7) - 0x28` below
/// the three pushes; the record `R = A[4]`, its state `R + 0x18` and its
/// count `R + 0x2c`, up at every exit). By the state:
/// - 0: the summon's nodes ([`summon_task_0203_start`]); on;
/// - 1 after 20 calls: an effect node, then 0xa;
/// - 0xa: the summon records' words 20000, on, `R + 0x28` = 12, `R + 0x34`
///   = 0;
/// - 0xb: every `R + 0x34` calls the first free record (a word 20000)
///   gets the set-up ([`summon_setup_0203`]); `R + 0x28` counts down to
///   state 0x14; the wait is `5 + slot 324 % 5`;
/// - 0x14: two inserted nodes and a particle node; on;
/// - 0x15/0x16 after 40 calls: the parts shown, the node set-up, the
///   models' slots, the scene nodes ([`summon_task_0203_scene`]); on;
/// - 0x17 after 7 calls the light on, 0x18 after 36 calls the light's
///   slot 234; 0x19 after 40 calls the colour block's last words, 10000;
/// - 10000 with bit 2 of `R + 0x158`: the flag word, on; 10001 after 3
///   calls: `A`'s node leaves its ring.
///
/// Returns `eax` as the original leaves it (the caller's where the state
/// is out of the table's range).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0203<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let up = |g: &mut G| {
        let c = g.read::<u32>(at(0x2c)).wrapping_add(1);
        g.write::<u32>(at(0x2c), c);
    };
    let count = |g: &G| g.read::<i32>(at(0x2c));
    // inc [R + 0x18]; mov [R + 0x2c], 0; inc [R + 0x2c]
    let next = |g: &mut G| {
        let st = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), st);
        g.write::<u32>(at(0x2c), 0);
        g.write::<u32>(at(0x2c), 1);
    };
    let st = g.read::<u32>(at(0x18));
    if st > 0x2710 {
        if st == 0x2711 && count(g) >= 3 {
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
        return Ret::eax(fr.eax);
    }
    if st == 0x2710 {
        if g.read::<u8>(at(0x158)) & 4 != 0 {
            let x = g.read::<u32>(w(0xaf3)) | 1;
            g.write::<u32>(w(0xaf3), x);
            next(g);
        } else {
            up(g);
        }
        return Ret::eax(fr.eax);
    }
    if st > 0x19 {
        up(g);
        return Ret::eax(fr.eax);
    }
    let idx = img.word(w(0x33).wrapping_add(st)) & 0xff;
    let eax = match st {
        0 => {
            let eax = summon_task_0203_start(g, img, f, s, r);
            next(g);
            eax
        }
        1 if count(g) >= 0x14 => {
            // push 0; push ecx (never read); push A; push B
            let ef = frame_4(g, img, f + 0x343, s, w(0x33e), w(0x339), 0);
            g.write::<u32>(s - 0xc, w(0x334));
            let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
            let x = g.read::<u32>(w(0x349)) | 1;
            g.write::<u32>(w(0x349), x);
            abs(g, 0x34e, 0x358);
            up(g);
            g.write::<u32>(at(0x18), 0xa);
            eax
        }
        0xa => {
            let mut p = w(0x36d);
            loop {
                g.write::<u32>(p, 0x4e20);
                p = p.wrapping_add(0x490);
                if p.cast_signed() >= w(0x38c).cast_signed() {
                    break;
                }
            }
            let x = g.read::<u32>(at(0x18)).wrapping_add(1);
            g.write::<u32>(at(0x18), x);
            up(g);
            g.write::<u32>(at(0x28), 0xc);
            g.write::<u32>(at(0x34), 0);
            p
        }
        0xb => {
            let wait = g.read::<i32>(at(0x34));
            g.write::<u32>(at(0x34), wait.wrapping_sub(1).cast_unsigned());
            if wait > 0 {
                up(g);
                // lea eax, [ecx - 1]
                return Ret::eax(wait.wrapping_sub(1).cast_unsigned());
            }
            // the first record whose word is 20000, four at a time
            let (mut i, mut p) = (0u32, w(0x3c1));
            let found = loop {
                let mut hit = None;
                for j in 0..4u32 {
                    let q = p.wrapping_sub(0x490).wrapping_add(0x490 * j);
                    if g.read::<u32>(q) == 0x4e20 {
                        hit = Some(i + j);
                        break;
                    }
                }
                if let Some(h) = hit {
                    break Some(h);
                }
                p = p.wrapping_add(0x1240);
                i += 4;
                if p.cast_signed() >= w(0x405).cast_signed() {
                    break None;
                }
            };
            let i = found.unwrap_or(i);
            if i == 0x10 {
                up(g);
                return Ret::eax(p);
            }
            let rec = i.wrapping_mul(0x490).wrapping_add(w(0x439));
            let sp = s - 0xc;
            put_words(
                g,
                sp,
                &[g.read::<u32>(at(0x190)), at(0x48), g.read::<u32>(at(0x54))],
            );
            let sf = Frame {
                at: img.rel32_target(f + 0x43e),
                esp: sp - 4,
                ecx: rec,
                edx: at(0x38),
                ..Frame::default()
            };
            let eax = summon_setup_0203(g, img, &sf);
            let left = g.read::<u32>(at(0x28)).wrapping_sub(1);
            g.write::<u32>(at(0x28), left);
            if left != 0 {
                // call width: 0
                let v = g.call(t(g, 0x45c, 324), 0, &[]).eax.cast_signed();
                up(g);
                g.write::<u32>(at(0x34), (v.wrapping_rem(5) + 5).cast_unsigned());
                return Ret::eax(v.wrapping_div(5).cast_unsigned());
            }
            g.write::<u32>(at(0x18), 0x14);
            up(g);
            return Ret::eax(eax);
        }
        0x14 => {
            crate::sa_misc::node_append(g, img, img.rel32_target(f + 0x48a), w(0x485), w(0x480), 0);
            abs(g, 0x494, 0x49e);
            let x = g.read::<u32>(at(0x190)).wrapping_sub(0x4000);
            g.write::<u32>(w(0x4b9), x);
            abs(g, 0x4bd, 0x4c7);
            // the two inserts' words: (A, B, node, C) and (A', B', C', node')
            put_words(g, s - 0x10, &[w(0x4af), w(0x4aa), w(0x485), w(0x48f)]);
            let inf = Frame {
                at: img.rel32_target(f + 0x4c8),
                esp: s - 0x14,
                ecx: w(0x4b4),
                edx: w(0x480),
                ..Frame::default()
            };
            crate::sa_misc::insert_with(g, img, &inf);
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x4d7), w(0x4d2), w(0x4cd), 0);
            put_words(g, s - 0x20, &[w(0x4e7), w(0x4e2), w(0x4dd), w(0x4d2)]);
            abs(g, 0x4f0, 0x518);
            let inf = Frame {
                at: img.rel32_target(f + 0x519),
                esp: s - 0x24,
                ecx: w(0x4ec),
                edx: w(0x4cd),
                ..Frame::default()
            };
            crate::sa_misc::insert_with(g, img, &inf);
            let pf = frame_4(g, img, f + 0x530, s, w(0x521), w(0x52b), 0);
            let eax = crate::s3_nodes::particle_node_with(g, img, &pf);
            next(g);
            eax
        }
        0x15 | 0x16 => {
            let mut eax = idx;
            if st == 0x15 {
                g.write::<u32>(at(0x18), 0x16);
                eax = 0x16;
            }
            if count(g) < 0x28 {
                up(g);
                return Ret::eax(eax);
            }
            let eax = summon_task_0203_scene(g, img, f, s, r);
            next(g);
            eax
        }
        0x17 | 0x18 => {
            let mut eax = idx;
            if st == 0x17 && count(g) >= 7 {
                let c = g.read::<f32>(w(0x913));
                abs(g, 0x917, 0x93f);
                let lf = Frame {
                    at: img.rel32_target(f + 0x940),
                    esp: s - 4,
                    xmm: [0, u128::from(c.to_bits()), 0, 0],
                    ..Frame::default()
                };
                eax = crate::s4_small::light_on(g, img, &lf).eax;
                let x = g.read::<u32>(at(0x18)).wrapping_add(1);
                g.write::<u32>(at(0x18), x);
            }
            if count(g) < 0x24 {
                up(g);
                return Ret::eax(eax);
            }
            abs(g, 0x95b, 0x983);
            // call width: 3
            let eax = g
                .call(t(g, 0x952, 234), 0, &[w(0x98f), w(0x98a), w(0x957)])
                .eax;
            let x = g.read::<u16>(w(0x998)) | 1;
            g.write::<u16>(w(0x998), x);
            abs(g, 0x99d, 0x9bb);
            let x = g.read::<u32>(at(0x18)).wrapping_add(1);
            g.write::<u32>(at(0x18), x);
            up(g);
            eax
        }
        0x19 if count(g) >= 0x28 => {
            let c = k(g, 0x9d9);
            let x0 = sub(c, k(g, 0x9e4));
            let fl = (g.read::<u32>(w(0x9e9)) & !1) | 4;
            abs(g, 0x9f3, 0x9fd);
            g.write::<f32>(w(0xa01), x0);
            let x0 = sub(k(g, 0xa09), k(g, 0xa11));
            abs(g, 0xa15, 0xa3d);
            g.write::<f32>(w(0xa41), x0);
            let x0 = sub(c, k(g, 0xa4c));
            let x1 = sub(c, k(g, 0xa54));
            g.write::<u32>(w(0xa59), fl);
            abs(g, 0xa5d, 0xa67);
            g.write::<f32>(w(0xa6b), x0);
            let x0 = sub(k(g, 0xa73), k(g, 0xa7b));
            g.write::<f32>(w(0xa83), x1);
            g.write::<f32>(w(0xa8b), x0);
            for (a, b, to) in [
                (0xa93u32, 0xa9bu32, 0xaa3u32),
                (0xaab, 0xab3, 0xabb),
                (0xac3, 0xacb, 0xad3),
            ] {
                let x = sub(k(g, a), k(g, b));
                g.write::<f32>(w(to), x);
            }
            up(g);
            g.write::<u32>(at(0x18), 0x2710);
            fl
        }
        _ => {
            up(g);
            return Ret::eax(idx);
        }
    };
    Ret::eax(eax)
}

/// [`summon_task_0203`]'s state 0: the summon nodes
/// ([`summon_nodes`](crate::s4_small::summon_nodes)) of `[N, P, *G_O,
/// 0x10]` and the model node of `[N2, P, *G_O, 0]` at `S + 0xc`, the
/// head node ([`object_head_node`](crate::o4_flags::object_head_node)),
/// the summon's part records `R + 0x48..` (the table's records whose byte
/// is in the target bytes, not the caster), the centre
/// ([`centre`](crate::m0577::centre)) into `R + 0x38`, appended nodes,
/// slot 325, the facing ([`facing_int`](crate::sa_misc::facing_int)),
/// the static link, an effect node, the colour node
/// ([`colour_node`](crate::s3_misc::colour_node)) and its block's words.
/// Returns the colour node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_task_0203_start<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let at = |o: u32| r.wrapping_add(o);
    let call = |o: u32| img.rel32_target(f + o);
    let rec = s + 0xc;
    let o = g.read::<u32>(w(0x3f));
    put_words(g, rec, &[w(0x4b), w(0x53), o, 0x10]);
    abs(g, 0x63, 0x6d);
    let nf = Frame {
        at: call(0x6e),
        esp: s - 4,
        ecx: rec,
        ..Frame::default()
    };
    crate::s4_small::summon_nodes(g, img, &nf);
    let o = g.read::<u32>(w(0x73));
    put_words(g, rec, &[w(0x7f), w(0x87), o]);
    g.write::<u8>(rec + 0xc, 0);
    abs(g, 0x94, 0x9e);
    crate::sa_summon::model_node_init(g, img, call(0x9f), rec);
    crate::o4_flags::object_head_node(g, img, call(0xaf), at(0x1d4), w(0xaa));
    // the part records of the targets
    g.write::<u32>(at(0x54), 0);
    g.write::<u32>(rec, 0);
    let mut i = 0i32;
    let mut part = w(0xd7);
    while i < g.read::<i32>(w(0xbc)) {
        let b = g.read::<u32>(part);
        if b != g.read::<u32>(w(0xe4)) {
            let n = g.read::<i32>(w(0xcd));
            let mut j = 0i32;
            while j < n {
                let tb = u32::from(g.read::<u8>(w(0xf3).wrapping_add(j.cast_unsigned())));
                if b == tb {
                    let c = g.read::<u32>(at(0x54));
                    g.write::<u32>(at(0x48).wrapping_add(4 * c), part.wrapping_sub(0x50));
                    g.write::<u32>(at(0x54), c.wrapping_add(1));
                    break;
                }
                j += 1;
            }
        }
        i += 1;
        g.write::<u32>(rec, i.cast_unsigned());
        part = part.wrapping_add(0x98);
    }
    let cf = Frame {
        at: call(0x138),
        esp: s - 4,
        ecx: at(0x38),
        edx: w(0x131),
        ..Frame::default()
    };
    crate::m0577::centre(g, img, &cf);
    g.write::<u32>(at(0x3c), g.read::<u32>(w(0x140)));
    g.write::<u32>(at(0x44), ONE);
    crate::sb_misc::node_append(g, img, call(0x15b), w(0x14a), w(0x145), 0);
    // call width: 2
    g.call(t(g, 0x160, 325), 0, &[w(0x16c), at(0x38)]);
    let x = sub(k(g, 0x176), k(g, 0x17e));
    g.write::<f32>(w(0x194), x);
    crate::sa_misc::node_append(g, img, call(0x199), at(0x170), w(0x18c), 0);
    crate::sa_misc::facing_int(g, img, call(0x1a9), w(0x1a4), at(0x18c), s - 4);
    let lf = Frame {
        at: call(0x1ae),
        esp: s - 4,
        ..Frame::default()
    };
    crate::s4_small::static_link_last(g, img, &lf);
    // push 2; push ecx (never read); push A; push B
    put_words(g, s - 0x10, &[w(0x1bb), w(0x1b6)]);
    g.write::<u32>(s - 4, 2);
    let ef = Frame {
        at: call(0x1c5),
        esp: s - 0x14,
        ecx: w(0x1c0),
        ..Frame::default()
    };
    crate::s3_nodes::effect_node_with(g, img, &ef);
    let x = g.read::<u32>(w(0x1cb)) | 1;
    g.write::<u32>(w(0x1cb), x);
    abs(g, 0x1d7, 0x1e1);
    g.write::<u32>(rec, w(0x1e5));
    g.write::<u32>(rec + 4, w(0x1ed));
    abs(g, 0x1f1, 0x1fb);
    let cf = Frame {
        at: call(0x1fc),
        esp: s - 4,
        ecx: rec,
        ..Frame::default()
    };
    let eax = crate::s3_misc::colour_node(g, img, &cf);
    // the colour block's words
    let x0 = sub(k(g, 0x204), k(g, 0x20c));
    let (c2, c1) = (k(g, 0x214), k(g, 0x21c));
    let fl = g.read::<u32>(w(0x222)) & !1;
    g.write::<u32>(w(0x222), fl);
    abs(g, 0x227, 0x231);
    g.write::<f32>(w(0x235), x0);
    let x0 = sub(c2, k(g, 0x240));
    abs(g, 0x244, 0x26c);
    g.write::<f32>(w(0x270), x0);
    let x0 = sub(c1, k(g, 0x27b));
    let x1 = sub(c1, k(g, 0x283));
    abs(g, 0x287, 0x2a5);
    g.write::<f32>(w(0x2a9), x0);
    let x0 = sub(c2, k(g, 0x2b4));
    abs(g, 0x2b8, 0x2cc);
    g.write::<f32>(w(0x2d0), x1);
    g.write::<f32>(w(0x2d8), x0);
    let x0 = sub(c2, k(g, 0x2e3));
    g.write::<f32>(w(0x2eb), x0);
    let x0 = sub(c2, k(g, 0x2f6));
    let x2 = sub(c2, k(g, 0x2fe));
    g.write::<f32>(w(0x306), x0);
    g.write::<f32>(w(0x30e), x2);
    eax
}

/// [`summon_task_0203`]'s state 0x16 past its wait: the parts shown
/// ([`show`](crate::m0070::show)), the node set-up
/// ([`node_setup`](crate::m0203::node_setup)), each model's slots 239
/// and 698 and its flag step
/// ([`flag_step_arg`](crate::o4_flags::flag_step_arg)), a node appended
/// and inserted, the part record ([`part_record`](crate::m0203::part_record)),
/// the bone node ([`bone_node_setup`](crate::o6_fam_bone::bone_node_setup)),
/// two effect nodes, slot 222, the static link, slot 234 on the light's
/// words. Returns slot 234's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_task_0203_scene<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let at = |o: u32| r.wrapping_add(o);
    let call = |o: u32| img.rel32_target(f + o);
    let rec = s + 0xc;
    let mut i = 0i32;
    let mut part = w(0x566);
    while i < g.read::<i32>(w(0x55f)) {
        g.write::<u32>(rec, part);
        g.write::<u8>(rec + 4, 0xff);
        crate::m0070::show(g, img, call(0x57e), rec);
        i += 1;
        part = part.wrapping_add(0x98);
    }
    g.write::<u32>(rec, w(0x599));
    g.write::<u8>(rec + 4, 0xff);
    crate::m0070::show(g, img, call(0x5a3), rec);
    // push ecx (never read); push A; push B
    put_words(g, s - 0xc, &[w(0x5ae), w(0x5a9)]);
    let nf = Frame {
        at: call(0x5b8),
        esp: s - 0x10,
        ecx: w(0x5b3),
        ..Frame::default()
    };
    crate::m0203::node_setup(g, img, &nf);
    let mut j = 0i32;
    while j < g.read::<i32>(at(0x54)) {
        let pp = at(0x48).wrapping_add(4 * j.cast_unsigned());
        let m = g.read::<u32>(pp);
        let (o, x) = (g.read::<u32>(w(0x5de)), g.read::<u32>(m.wrapping_add(0x50)));
        // call width: 3
        g.call(t(g, 0x5d8, 239), 0, &[o, x, 2]);
        let m = g.read::<u32>(pp);
        let x = g.read::<u32>(m.wrapping_add(0x50));
        // call width: 1
        let on = g.call(t(g, 0x5f3, 698), 0, &[x]).eax != 0;
        let m = g.read::<u32>(pp);
        let (kk, word, mark) = if on { (0x18, 0x19, 4) } else { (0x48, 0x49, 5) };
        g.write::<u32>(s - 4, word);
        let ff = Frame {
            at: call(if on { 0x610 } else { 0x627 }),
            esp: s - 8,
            ecx: m,
            edx: kk,
            ..Frame::default()
        };
        crate::o4_flags::flag_step_arg(g, img, &ff);
        let m = g.read::<u32>(pp);
        g.write::<u32>(m.wrapping_add(0x6c), mark);
        j += 1;
    }
    // a node appended under the parent and inserted
    let last = g.read::<u32>(w(0x641));
    let fl = g.read::<u8>(w(0x647)) | 3;
    g.write::<u8>(w(0x647), fl);
    abs(g, 0x64c, 0x6af);
    g.write::<u32>(w(0x6b0), last);
    if last == w(0x6b5) {
        abs(g, 0x6bb, 0x6c5);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(0x6ca));
    }
    let n = g.read::<u16>(w(0x6d1)).wrapping_add(1);
    g.write::<u16>(w(0x6d1), n);
    abs(g, 0x6df, 0x71b);
    put_words(
        g,
        rec,
        &[w(0x71f), w(0x727), w(0x72f), w(0x737), at(0x170), w(0x743)],
    );
    let inf = Frame {
        at: call(0x748),
        esp: s - 4,
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &inf);
    // the part record
    let e = g.read::<u32>(w(0x74d));
    g.write::<u32>(rec, at(0x60));
    g.write::<u32>(at(0x60), w(0x75a));
    put_words(g, rec + 4, &[w(0x766), w(0x76e), e, w(0x77a), 1, w(0x78a)]);
    let pf = Frame {
        at: call(0x78f),
        esp: s - 4,
        ecx: rec,
        ..Frame::default()
    };
    crate::m0203::part_record(g, img, &pf);
    g.write::<u32>(s - 4, at(0x170));
    let bf = Frame {
        at: call(0x7a0),
        esp: s - 8,
        ecx: at(0x1f4),
        edx: w(0x79b),
        ..Frame::default()
    };
    crate::o6_fam_bone::bone_node_setup(g, img, &bf);
    // two effect nodes
    for (first, words, call_at, vt) in [
        (
            w(0x7af),
            [w(0x7b7), w(0x7bf), w(0x7c7), w(0x7cf), 1],
            0x7dc,
            0x7ec,
        ),
        (
            at(0x19c),
            [w(0x80d), w(0x815), w(0x81d), w(0x825), 3],
            0x832,
            0x83e,
        ),
    ] {
        g.write::<u32>(rec, first);
        put_words(g, rec + 4, &words);
        crate::sb_nodes::effect_node(g, img, call(call_at), rec);
        let node = g.read::<u32>(rec);
        g.write::<u32>(node, w(vt));
        if call_at == 0x7dc {
            let x = g.read::<u32>(w(0x7f2)) | 1;
            g.write::<u32>(w(0x7f2), x);
            abs(g, 0x7fb, 0x805);
        }
    }
    let n = at(0x19c);
    let x = g.read::<u32>(n + 0x2c) | 1;
    g.write::<u32>(n + 0x2c, x);
    g.write::<u32>(n + 0x30, 0);
    // call width: 4
    g.call(t(g, 0x843, 222), 0, &[w(0x85d), 0x11, 0x12, 2]);
    // slot 222's four words still pushed
    let lf = Frame {
        at: call(0x864),
        esp: s - 0x14,
        ..Frame::default()
    };
    crate::s4_small::static_link_tail(g, img, &lf);
    abs(g, 0x872, 0x8c2);
    // call width: 3
    let eax = g
        .call(t(g, 0x869, 234), 0, &[w(0x8ce), w(0x8c9), w(0x86e)])
        .eax;
    abs(g, 0x8d7, 0x8f5);
    eax
}

/// [`summon_task_0203`] (the `rel32`s: its callees in order).
pub const SUMMON_TASK_0203: Sig = Sig {
    len: 2950,
    hash: 0x1a87_ce2b_4f6f_a6f2,
    calls: &[
        (0x6e, Callee::Sig(&crate::s4_small::SUMMON_NODES)),
        (0x9f, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (
            0xaf,
            Callee::Sig(&crate::o4_flags::OBJECT_HEAD_NODE_0203_7940),
        ),
        (0x138, Callee::Sig(&crate::m0577::CENTRE)),
        (0x15b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x199, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x1a9, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x1ae, Callee::Sig(&crate::s4_small::STATIC_LINK_LAST)),
        (0x1c5, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1fc, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x343, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x43e, Callee::Sig(&SUMMON_SETUP_0203)),
        (0x48a, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x4c8, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x4d7, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x519, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x530, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x57e, Callee::Sig(&crate::m0070::SHOW)),
        (0x5a3, Callee::Sig(&crate::m0070::SHOW)),
        (0x5b8, Callee::Sig(&crate::m0203::NODE_SETUP)),
        (
            0x610,
            Callee::Sig(&crate::o4_flags::FLAG_STEP_ARG_0203_73C0),
        ),
        (
            0x627,
            Callee::Sig(&crate::o4_flags::FLAG_STEP_ARG_0203_73C0),
        ),
        (0x748, Callee::Sig(&crate::nodes::INSERT)),
        (0x78f, Callee::Sig(&crate::m0203::PART_RECORD)),
        (
            0x7a0,
            Callee::Sig(&crate::o6_fam_bone::BONE_NODE_SETUP_0203_7710),
        ),
        (0x7dc, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x832, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x864, Callee::Sig(&crate::s4_small::STATIC_LINK_TAIL)),
        (0x940, Callee::Sig(&crate::s4_small::LIGHT_ON)),
    ],
    imports: &[],
};

/// `magic_0201`'s summon end task (0x100070d0, 1542 bytes; a task
/// callback `(A)`, cdecl; an 8-aligned frame, `S = ((E - 4) & !7) -
/// 0x28` below the three pushes; the record `R = A[4]`, its state `R +
/// 0x18`, its count `R + 0x44`, up at every exit). By the state:
/// - 0: the swing `R + 0x34` (less `R + 0x30`) drives the camera word
///   (`(K - s) K2` truncated) and the height `*G_H` (less `s R[0x28]`);
///   once `s` is below 0 the state goes on with `R + 0x28` 0;
/// - 1 after 10 calls: three nodes appended under the parent from the
///   store runs and a particle node
///   ([`particle_node`](crate::sb_nodes::particle_node)), then on into 2;
/// - 2: the height falls by `R + 0x28` (which falls by the constant at
///   `+0x265`), the angle word up by 0x2000; once the height reaches `R +
///   0x2c` another particle node, slot 530, the colour block, two nodes
///   with slot 719's handles, and 10000;
/// - 10000 after 40 calls: the words cleared, the height `R + 0x38`,
///   slots 530, 228 and 239 on the model; on;
/// - 10001 after 24 calls: the flag word, `A`'s node leaves its ring.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn end_task_0201<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::cvtt;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let up = |g: &mut G| {
        let c = g.read::<u32>(at(0x44)).wrapping_add(1);
        g.write::<u32>(at(0x44), c);
    };
    let count = |g: &G| g.read::<i32>(at(0x44));
    let step = |g: &mut G| {
        let x = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), x);
    };
    let model = |g: &G| g.read::<u32>(g.read::<u32>(at(0x1c)).wrapping_add(0x50));
    let st = g.read::<u32>(at(0x18));
    if st > 2 {
        if st == 0x2710 {
            if count(g) < 0x28 {
                up(g);
                return Ret::eax(0);
            }
            abs(g, 0x57c, 0x59a);
            g.write::<u32>(w(0x5a3), g.read::<u32>(at(0x38)));
            let m = model(g);
            // call width: 2
            g.call(t(g, 0x5b3, 530), 0, &[m, 0x100]);
            let m = model(g);
            // call width: 2
            g.call(t(g, 0x5c8, 228), 0, &[m, 0x38]);
            let m = model(g);
            let o = g.read::<u32>(w(0x5e3));
            // call width: 3
            let eax = g.call(t(g, 0x5dd, 239), 0, &[o, m, 2]).eax;
            step(g);
            g.write::<u32>(at(0x44), 0);
            g.write::<u32>(at(0x44), 1);
            return Ret::eax(eax);
        }
        if st == 0x2711 && count(g) >= 0x18 {
            let x = g.read::<u32>(w(0x549)) | 1;
            g.write::<u32>(w(0x549), x);
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
        return Ret::eax(st.wrapping_sub(0x2711));
    }
    if st == 0 {
        let sw = sub(rf(g, 0x34), rf(g, 0x30));
        let x = sub(k(g, 0x38f), sw);
        g.write::<f32>(at(0x34), sw);
        let ang = g.read::<u32>(w(0x39e)).wrapping_add(0x2000);
        g.write::<u32>(w(0x39e), ang);
        let cam = cvtt(mul(x, k(g, 0x3aa))).cast_unsigned();
        let h = k(g, 0x3b6);
        g.write::<u32>(w(0x3bb), cam);
        let h = sub(h, mul(rf(g, 0x34), rf(g, 0x28)));
        g.write::<f32>(w(0x3d1), h);
        // comiss 0, s; jbe: at or above 0 (or unordered) waits
        if rf(g, 0x34) < 0.0 {
            abs(g, 0x3e2, 0x3ec);
            step(g);
            g.write::<u32>(at(0x44), 0);
            g.write::<u32>(at(0x44), 1);
            g.write::<u32>(at(0x28), 0);
        } else {
            up(g);
        }
        return Ret::eax(cam);
    }
    let mut eax = if st == 2 { 2 } else { 0 };
    if st == 1 && count(g) >= 0xa {
        eax = end_task_0201_nodes(g, img, f, s);
        step(g);
    }
    // state 2: the height falls
    let h = sub(k(g, 0x24b), rf(g, 0x28));
    g.write::<f32>(w(0x258), h);
    let fall = sub(rf(g, 0x28), k(g, 0x265));
    g.write::<f32>(at(0x28), fall);
    let h = k(g, 0x272);
    let ang = g.read::<u32>(w(0x278)).wrapping_add(0x2000);
    g.write::<u32>(w(0x278), ang);
    // comiss h, R[0x2c]; jb: below or unordered waits
    let floor = rf(g, 0x2c);
    if h < floor || h.is_nan() || floor.is_nan() {
        up(g);
        return Ret::eax(eax);
    }
    end_task_0201_land(g, img, f, s, r);
    g.write::<u32>(at(0x44), 0);
    g.write::<u32>(at(0x18), 0x2710);
    // on into 10000 at once: fewer than 40 calls
    up(g);
    Ret::eax(0)
}

/// [`end_task_0201`]'s state 1: three nodes appended under the parent
/// (each after the one before; their words from the store runs) and the
/// particle node of the record at `S + 0x10`. Returns its `eax`.
fn end_task_0201_nodes<G: Game>(g: &mut G, img: &Image, f: u32, s: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let last = g.read::<u32>(w(0x41));
    append_0178(g, img, f, last, (0x45, 0x76), [0x77, 0x7c, 0x82, 0x91]);
    let mut count = g.read::<u16>(w(0x97)).wrapping_add(1);
    abs(g, 0xa2, 0x119);
    if w(0x9c) == w(0x11b) {
        abs(g, 0x121, 0x12b);
    } else {
        abs(g, 0x12d, 0x137);
    }
    count = count.wrapping_add(1);
    abs(g, 0x13e, 0x1a1);
    if w(0x138) == w(0x1a3) {
        abs(g, 0x1a9, 0x1b3);
    } else {
        abs(g, 0x1b5, 0x1bf);
    }
    count = count.wrapping_add(1);
    g.write::<u16>(w(0x1c3), count);
    let e = g.read::<u32>(w(0x1c8));
    abs(g, 0x1d0, 0x209);
    put_words(g, s + 0x10, &[w(0x20d), w(0x215), w(0x21d), e, w(0x229), 3]);
    abs(g, 0x235, 0x23f);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x240), s + 0x10)
}

/// [`end_task_0201`]'s state 2 at the floor: a particle node, slot 530
/// `(model, 0)`, the colour block's flag and words, a node appended with
/// slot 719's handle and the node `R + 0x4c` appended with another.
fn end_task_0201_land<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let at = |o: u32| r.wrapping_add(o);
    let e = g.read::<u32>(w(0x28b));
    put_words(g, s + 0x10, &[w(0x297), w(0x29f), w(0x2a7), e, w(0x2b3), 0]);
    abs(g, 0x2bf, 0x2c9);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x2ca), s + 0x10);
    let m = g.read::<u32>(g.read::<u32>(at(0x1c)).wrapping_add(0x50));
    // call width: 2
    g.call(t(g, 0x2d7, 530), 0, &[m, 0]);
    let c = sub(g.read::<f32>(w(0x2e7)), 0.0);
    let last = g.read::<u32>(w(0x2ec));
    let fl = g.read::<u8>(w(0x2f2)) | 3;
    g.write::<u8>(w(0x2f2), fl);
    abs(g, 0x301, 0x31f);
    g.write::<f32>(w(0x323), c);
    abs(g, 0x327, 0x331);
    append_0178(
        g,
        img,
        f,
        last,
        (0x331, 0x362),
        [0x363, 0x368, 0x372, 0x40a],
    );
    let n = g.read::<u16>(w(0x416)).wrapping_add(1);
    g.write::<u16>(w(0x416), n);
    abs(g, 0x41c, 0x426);
    // call width: 3
    let h = g.call(t(g, 0x40f, 719), 0, &[w(0x42f), 6, 1]).eax;
    g.write::<u32>(w(0x436), h);
    abs(g, 0x43d, 0x483);
    // the node `R + 0x4c` under the parent
    let node = at(0x4c);
    crate::sa_misc::child_of_0512(
        g,
        img,
        f,
        node,
        [0x492, 0x499, 0x49e, 0x4a6, 0x4ae, 0x4b6, 0x4c7, 0x4cf],
    );
    // call width: 3
    let h = g.call(t(g, 0x4c0, 719), 0, &[w(0x4dc), 6, 2]).eax;
    g.write::<u32>(node + 0x24, h);
    g.write::<u32>(node + 0x1c, w(0x4e8));
    g.write::<u32>(node + 0x20, w(0x4ef));
    g.write::<u32>(node + 0x34, 0x40);
    g.write::<u32>(node + 0x28, 0x100);
    g.write::<u32>(node + 0x2c, 0);
    g.write::<u32>(node, w(0x50a));
    g.write::<u32>(node + 0x2c, 1);
    g.write::<u32>(node + 0x30, 0);
}

/// [`end_task_0201`] (the `rel32`s: the particle nodes).
pub const END_TASK_0201: Sig = Sig {
    len: 1542,
    hash: 0x074e_e346_b6b5_3042,
    calls: &[
        (0x240, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2ca, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0201`'s summon set-up (0x100076e0, 1702 bytes; internal: `edx`
/// = the model `M`, cdecl `(a, b)`; an `ebp` frame, the caller's word
/// `[ebp + 8]` (`a`'s slot) the floats' scratch): the end task
/// ([`end_task_0201`]) on the static list, its words `a`, `b` and `M`;
/// the model's slot (36 or 228 by its flag bit 0) and flags (0x120, with
/// the bit also 0x200); each of
/// `a`'s `[a + 0x4dc]` parts shown ([`show`](crate::m0070::show)); then
/// the scene's nodes appended from the store runs, each with its words:
/// slot 262 for the model's place, the facing from slot 333 (or 601) as
/// an int, the colour from slots 15/16/17 (or 290), the scale from slot
/// 277, slot 211, the model's flag 0x10 and node, the heights from slots
/// 664 and 216. Returns slot 216's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0201<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::sb_sse::cvtt;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let up16 = |g: &mut G, op: u32| {
        let n = g.read::<u16>(w(op)).wrapping_add(1);
        g.write::<u16>(w(op), n);
    };
    let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
    let m = fr.edx;
    // [ebp + 8]: `a`'s word, the scratch of the floats
    let word0 = fr.esp + 4;
    let local = fr.esp.wrapping_sub(0x10);
    let m50 = |g: &G| g.read::<u32>(m + 0x50);
    let m54 = |g: &G| g.read::<u32>(m + 0x54);
    let bit = |g: &G| g.read::<u8>(m + 0x90) & 1 != 0;
    let flags = |g: &mut G, x: u16| {
        let v = g.read::<u16>(m + 0x90) | x;
        g.write::<u16>(m + 0x90, v);
    };
    // the end task on the static list
    let last = gw(g, 0x7);
    g.write::<u32>(w(0x10), last);
    abs(g, 0x15, 0x3d);
    g.write::<u32>(last.wrapping_add(4), w(0x40));
    let fl = gw(g, 0x49) & !1;
    g.write::<u32>(w(0x49), fl);
    g.write::<u32>(w(0x51), b);
    abs(g, 0x5b, 0x65);
    g.write::<u32>(w(0x67), m);
    g.write::<u32>(w(0x6d), a);
    // the model's slot and flags
    if bit(g) {
        // call width: 2
        g.call(t(g, 0x56, 36), 0, &[m54(g), 0x10]);
        // or 0x200, then on into the other way's `or 0x120`
        flags(g, 0x200);
    } else {
        // call width: 2
        g.call(t(g, 0x56, 228), 0, &[m50(g), 0x10]);
    }
    flags(g, 0x120);
    g.write::<u32>(m + 0x60, 0x10);
    let mut i = 0i32;
    while i < g.read::<i32>(a.wrapping_add(0x4dc)) {
        let part = a.wrapping_add(0x1c).wrapping_add(0x98 * i.cast_unsigned());
        g.write::<u32>(local, part);
        g.write::<u8>(local + 4, 0xff);
        crate::m0070::show(g, img, img.rel32_target(f + 0xdb), local);
        i += 1;
    }
    // the node under the second list, `a` its word
    let last = gw(g, 0xf5);
    append_0178(g, img, f, last, (0xf9, 0x134), [0x135, 0x13a, 0x140, 0x14f]);
    let last = gw(g, 0x154);
    up16(g, 0x15b);
    abs(g, 0x15f, 0x169);
    g.write::<u32>(w(0x16b), a);
    // the model's place (slot 262)
    append_0178(
        g,
        img,
        f,
        last,
        (0x16f, 0x1aa),
        [0x1ab, 0x1b0, 0x1b6, 0x1c5],
    );
    up16(g, 0x1cc);
    abs(g, 0x1df, 0x1f3);
    // call width: 4
    g.call(t(g, 0x1d1, 262), 0, &[m54(g), w(0x1fa), w(0x1db), w(0x1d6)]);
    // the facing as an int
    let last = gw(g, 0x204);
    append_0178(
        g,
        img,
        f,
        last,
        (0x20b, 0x250),
        [0x251, 0x256, 0x25c, 0x26b],
    );
    up16(g, 0x272);
    abs(g, 0x276, 0x294);
    let x = if bit(g) {
        // call width: 1
        let y = st0_f32(g.call(t(g, 0x29e, 333), 0, &[m54(g)]).st0);
        g.write::<f32>(word0, y);
        sub(k(g, 0x2b4), y)
    } else {
        // call width: 2
        g.call(t(g, 0x2c7, 601), 0, &[m50(g), word0]);
        sub(k(g, 0x2da), g.read::<f32>(word0))
    };
    let x = div(mul(x, k(g, 0x2ea)), k(g, 0x305));
    abs(g, 0x2ee, 0x301);
    abs(g, 0x309, 0x313);
    abs(g, 0x317, 0x321);
    g.write::<u32>(w(0x322), cvtt(x).cast_unsigned());
    // the colour
    let last = gw(g, 0x327);
    abs(g, 0x32b, 0x335);
    g.write::<u32>(w(0x336), last);
    if last == w(0x33b) {
        abs(g, 0x341, 0x34b);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(0x350));
    }
    up16(g, 0x357);
    abs(g, 0x360, 0x374);
    if bit(g) {
        for (op, n, to) in [
            (0x35cu32, 15u32, 0x387u32),
            (0x38c, 16, 0x39d),
            (0x3a2, 17, 0x3b3),
        ] {
            // call width: 1
            let y = st0_f32(g.call(t(g, op, n), 0, &[m54(g)]).st0);
            g.write::<f32>(w(to), y);
        }
    } else {
        // call width: 2
        g.call(t(g, 0x35c, 290), 0, &[m50(g), w(0x3c3)]);
    }
    // the scale (slot 277)
    // call width: 1
    let y = st0_f32(g.call(t(g, 0x3d0, 277), 0, &[m54(g)]).st0);
    g.write::<f32>(word0, y);
    g.write::<f32>(w(0x3fb), mul(y, k(g, 0x3eb)));
    // three nodes after one another
    let last = gw(g, 0x3f0);
    append_0178(
        g,
        img,
        f,
        last,
        (0x3ff, 0x43a),
        [0x43b, 0x440, 0x446, 0x455],
    );
    let mut count = g.read::<u16>(w(0x45b)).wrapping_add(1);
    abs(g, 0x466, 0x4da);
    if w(0x460) == w(0x4dc) {
        abs(g, 0x4e2, 0x4ec);
    } else {
        abs(g, 0x4ee, 0x4f8);
    }
    count = count.wrapping_add(1);
    abs(g, 0x4ff, 0x562);
    if w(0x4f9) == w(0x564) {
        abs(g, 0x56a, 0x574);
    } else {
        abs(g, 0x576, 0x580);
    }
    count = count.wrapping_add(1);
    g.write::<u16>(w(0x584), count);
    abs(g, 0x58d, 0x5c6);
    // call width: 2
    g.call(t(g, 0x589, 211), 0, &[m54(g), w(0x5cd)]);
    flags(g, 0x10);
    g.write::<u32>(m + 0x4c, w(0x5e6));
    // the heights
    let x = gw(g, 0x5ee);
    g.write::<u32>(w(0x5f6), x);
    g.write::<u32>(w(0x5fe), x);
    // call width: 1
    let y = st0_f32(g.call(t(g, 0x5df, 664), 0, &[m50(g)]).st0);
    g.write::<f32>(word0, y);
    let z = sub(k(g, 0x614), y);
    g.write::<f32>(w(0x629), z);
    // call width: 1
    let out = g.call(t(g, 0x61e, 216), 0, &[m50(g)]);
    let y = st0_f32(out.st0);
    g.write::<f32>(word0, y);
    let z = sub(k(g, 0x64c), mul(y, k(g, 0x644)));
    abs(g, 0x659, 0x66d);
    g.write::<f32>(w(0x671), z);
    let h = sub(k(g, 0x679), k(g, 0x681));
    abs(g, 0x685, 0x699);
    g.write::<f32>(w(0x69e), h);
    out.eax
}

/// [`summon_setup_0201`] (the `rel32`: the parts' show).
pub const SUMMON_SETUP_0201: Sig = Sig {
    len: 1702,
    hash: 0x2b17_c89c_8932_8f4d,
    calls: &[(0xdb, Callee::Sig(&crate::m0070::SHOW))],
    imports: &[],
};

/// `magic_0201`'s summon task (0x10007d90, 2276 bytes; a task callback
/// `(A)`, cdecl; an 8-aligned /GS frame, `S = ((E - 4) & !7) - 0x38`
/// below the two pushes; the record `R = A[4]`, its state `R + 0x18`, its
/// count `R + 0x30`, up at every exit). By the state:
/// - 0: the summon's nodes, the caster's part record `R + 0x1c` (found
///   by its byte), its model node, the scene's nodes and inserts, the bone
///   node and the static link ([`summon_task_0201_start`]); then 0xa;
/// - 0xa: an appended node, slot 531 (without the model flag), the face
///   point; 0xb: the motion from the caster; 0x32 once the model id is
///   known: its value 0xbb, on into 0x33 (after 2 calls: the scene's
///   nodes, particle and effect nodes, the value 0x38; then 0x14);
/// - 0x14 on; 0x15 after 17 calls: slot 532 decides between the summon
///   (the nodes, the set-up [`summon_setup_0201`], slot 295) and the end
///   (slot 239, the current node unlinked, slot 250);
/// - 0x16 and 0x20: slot 20 or 411 (by the global flag) until it answers,
///   then 0x1e or 10000 in the same call; 0x1e: slot 531 and the motion
///   towards; 0x1f once the model id is known: the value 0xbd;
/// - 10000: the node leaves, the model set-up, `A`'s node leaves its
///   ring.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0201<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let call = |o: u32| img.rel32_target(f + o);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let up = |g: &mut G| {
        let c = g.read::<u32>(at(0x30)).wrapping_add(1);
        g.write::<u32>(at(0x30), c);
    };
    let step = |g: &mut G| {
        let x = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), x);
    };
    let model = |g: &G| g.read::<u32>(at(0x1c));
    loop {
        let st = g.read::<u32>(at(0x18));
        if st > 0x2710 || (st != 0x2710 && st > 0x33) {
            up(g);
            return Ret::eax(st);
        }
        if st == 0x2710 {
            let node = w(0x83a);
            crate::sa_misc::leave(g, node, st);
            g.write::<u32>(s + 0xc, node);
            let mf = Frame {
                at: call(0x84c),
                esp: s - 4,
                ecx: s + 0xc,
                ..Frame::default()
            };
            crate::sa_summon::model_setup(g, img, &mf);
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
        let idx = img.word(w(0x42).wrapping_add(st)) & 0xff;
        let eax = match st {
            0x16 | 0x20 => {
                // slot 20 on one word with the flag, else slot 411 on another
                let (on, off) = if st == 0x16 {
                    (0x5d, 0x68)
                } else {
                    (0x98, 0xa3)
                };
                let tb = if st == 0x16 { 0x55 } else { 0x90 };
                let eax = if g.read::<u8>(w(if st == 0x16 { 0x4f } else { 0x8a })) & 1 != 0 {
                    let x = gw(g, on);
                    // call width: 1
                    g.call(t(g, tb, 20), 0, &[x]).eax
                } else {
                    let x = gw(g, off);
                    // call width: 1
                    g.call(t(g, tb, 411), 0, &[x]).eax
                };
                if eax == 0 {
                    up(g);
                    return Ret::eax(0);
                }
                g.write::<u32>(at(0x18), if st == 0x16 { 0x1e } else { 0x2710 });
                continue;
            }
            0 => {
                let eax = summon_task_0201_start(g, img, f, s, r);
                up(g);
                g.write::<u32>(at(0x18), 0xa);
                eax
            }
            0xa => {
                crate::sb_misc::node_append(g, img, call(0x39e), w(0x399), w(0x394), idx);
                let m = model(g);
                if g.read::<u8>(m.wrapping_add(0x90)) & 1 == 0 {
                    let x = g.read::<u32>(m.wrapping_add(0x50));
                    // call width: 2
                    g.call(t(g, 0x3b7, 531), 0, &[x, w(0x3af)]);
                }
                let x = sub(k(g, 0x3ca), k(g, 0x3d2));
                g.write::<f32>(w(0x3e4), x);
                let pf = Frame {
                    at: call(0x3e9),
                    esp: s - 4,
                    ecx: w(0x3dc),
                    edx: w(0x3d7),
                    ..Frame::default()
                };
                let eax = crate::s4_small::face_point(g, img, &pf).eax;
                step(g);
                up(g);
                eax
            }
            0x14 => {
                step(g);
                up(g);
                idx
            }
            0xb => {
                put_words(g, s + 0x1c, &[w(0x40c), 0xba, w(0x41c), 0x4230_0000, 0xb]);
                let mf = Frame {
                    at: call(0x431),
                    esp: s - 4,
                    ecx: s + 0x1c,
                    ..Frame::default()
                };
                let eax = crate::r21::motion_from_0201(g, img, &mf).eax;
                up(g);
                g.write::<u32>(at(0x18), 0x32);
                eax
            }
            0x32 | 0x33 | 0x1f => {
                let mut eax = idx;
                if st == 0x32 || st == 0x1f {
                    let (rec_op, id_at, obj_op, set_at, value) = if st == 0x32 {
                        (0x458, 0x45d, 0x472, 0x477, 0xbb)
                    } else {
                        (0x803, 0x808, 0x819, 0x81e, 0xbd)
                    };
                    g.write::<u32>(s + 0xc, w(rec_op));
                    let id = crate::sa_misc::model_id(g, img, call(id_at), s + 0xc);
                    if g.read::<u32>(s + 0x10) == 0 {
                        up(g);
                        return Ret::eax(id);
                    }
                    eax = crate::m0042::set_value(g, img, call(set_at), w(obj_op), value);
                    step(g);
                    if st == 0x1f {
                        up(g);
                        return Ret::eax(eax);
                    }
                    g.write::<u32>(at(0x30), 0);
                }
                if g.read::<i32>(at(0x30)) < 2 {
                    up(g);
                    return Ret::eax(eax);
                }
                let eax = summon_task_0201_scene(g, img, f, s, r);
                up(g);
                g.write::<u32>(at(0x18), 0x14);
                eax
            }
            0x15 => {
                if g.read::<i32>(at(0x30)) < 0x11 {
                    up(g);
                    return Ret::eax(idx);
                }
                let x = g.read::<u32>(model(g).wrapping_add(0x50));
                // call width: 1
                let ended = g.call(t(g, 0x5d4, 532), 0, &[x]).eax != 0;
                let eax = if ended {
                    let o = gw(g, 0x750);
                    // call width: 3
                    let eax = g.call(t(g, 0x740, 239), 0, &[o, 0xffff_fffb, 0]).eax;
                    crate::l3_common::list_unlink_current(g, img, call(0x75a), eax);
                    // call width: 2
                    g.call(t(g, 0x75f, 250), 0, &[0, 0]).eax
                } else {
                    summon_task_0201_summon(g, img, f, s, r)
                };
                step(g);
                up(g);
                eax
            }
            0x1e => {
                if g.read::<u8>(w(0x799)) & 1 == 0 {
                    let x = gw(g, 0x7ac);
                    // call width: 2
                    g.call(t(g, 0x7a6, 531), 0, &[x, s + 0x1c]);
                }
                put_words(g, s + 0xc, &[w(0x7c7), 0xbc, s + 0x1c, 0]);
                let mf = Frame {
                    at: call(0x7e0),
                    esp: s - 4,
                    ecx: s + 0xc,
                    ..Frame::default()
                };
                let eax = crate::s5_misc::motion_towards(g, img, &mf).eax;
                step(g);
                up(g);
                eax
            }
            _ => {
                up(g);
                idx
            }
        };
        return Ret::eax(eax);
    }
}

/// [`summon_task_0201`]'s state 0. Returns the static link's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_task_0201_start<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    let rec = s + 0xc;
    let o = gw(g, 0xe4);
    put_words(g, rec, &[w(0xf0), w(0xf8), o, 0x10]);
    abs(g, 0x108, 0x112);
    let nf = Frame {
        at: call(0x113),
        esp: s - 4,
        ecx: rec,
        ..Frame::default()
    };
    crate::s4_small::summon_nodes(g, img, &nf);
    // the caster's part record by its byte
    let n = g.read::<i32>(w(0x119));
    let b = u32::from(g.read::<u8>(w(0x126)));
    let mut i = 0i32;
    while i < n {
        let p = w(0x12b).wrapping_add(0x98 * i.cast_unsigned());
        if g.read::<u32>(p) == b {
            g.write::<u32>(at(0x1c), (0x98 * i.cast_unsigned()).wrapping_add(w(0x148)));
            break;
        }
        i += 1;
    }
    let o = gw(g, 0x150);
    put_words(g, rec, &[w(0x15c), w(0x164), o]);
    g.write::<u8>(rec + 0xc, 0);
    abs(g, 0x171, 0x17b);
    crate::sa_summon::model_node_init(g, img, call(0x17c), rec);
    // a node with the model's place and height
    let last = gw(g, 0x181);
    append_0178(
        g,
        img,
        f,
        last,
        (0x185, 0x1b6),
        [0x1b7, 0x1bc, 0x1c2, 0x1d1],
    );
    let c = g.read::<u16>(w(0x1d8)).wrapping_add(1);
    g.write::<u16>(w(0x1d8), c);
    abs(g, 0x1e6, 0x1fa);
    let m = g.read::<u32>(g.read::<u32>(at(0x1c)).wrapping_add(0x54));
    // call width: 4
    g.call(t(g, 0x1dd, 262), 0, &[m, w(0x209), w(0x204), w(0x1e2)]);
    // call width: 1
    let y = st0_f32(g.call(t(g, 0x213, 12), 0, &[m]).st0);
    g.write::<f32>(w(0x221), y);
    // the node `R + 0x34`
    abs(g, 0x228, 0x232);
    let node = at(0x34);
    g.write::<u32>(node, w(0x234));
    crate::sa_misc::child_of_0512(
        g,
        img,
        f,
        node,
        [0x247, 0x24e, 0x253, 0x25b, 0x263, 0x26b, 0x277, 0x285],
    );
    crate::sa_misc::facing_int(g, img, call(0x28a), w(0x27f), at(0x50), s - 4);
    let last = gw(g, 0x28f);
    append_0178(
        g,
        img,
        f,
        last,
        (0x293, 0x2c4),
        [0x2c5, 0x2ca, 0x2d0, 0x2df],
    );
    let c = g.read::<u16>(w(0x2e6)).wrapping_add(1);
    g.write::<u16>(w(0x2e6), c);
    abs(g, 0x2ee, 0x32a);
    let grp = s + 0x1c;
    put_words(
        g,
        grp,
        &[w(0x32e), w(0x336), w(0x33e), w(0x346), node, w(0x352)],
    );
    let inf = Frame {
        at: call(0x357),
        esp: s - 4,
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &inf);
    g.write::<u32>(s - 4, node);
    abs(g, 0x364, 0x36b);
    let bf = Frame {
        at: call(0x36c),
        esp: s - 8,
        ecx: at(0x60),
        edx: w(0x360),
        ..Frame::default()
    };
    crate::o6_fam_bone::bone_node_setup(g, img, &bf);
    let lf = Frame {
        at: call(0x374),
        esp: s - 4,
        ..Frame::default()
    };
    crate::s4_small::static_link_last(g, img, &lf).eax
}

/// [`summon_task_0201`]'s state 0x33 past its wait: appended nodes, the
/// facing, slot 366 (without the model flag), the inserted node, a
/// particle node and an effect node, the model's value 0x38. Returns the
/// value's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_task_0201_scene<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    crate::sa_misc::node_append(g, img, call(0x49a), w(0x495), w(0x490), 0);
    crate::sa_misc::facing_int(g, img, call(0x4a9), w(0x4a4), w(0x49f), s - 4);
    crate::sb_misc::node_append(g, img, call(0x4b8), w(0x4b3), w(0x4ae), 0);
    let m = g.read::<u32>(at(0x1c));
    if g.read::<u8>(m.wrapping_add(0x90)) & 1 == 0 {
        let x = g.read::<u32>(m.wrapping_add(0x50));
        // call width: 2
        g.call(t(g, 0x4d1, 366), 0, &[x, w(0x4c9)]);
    }
    abs(g, 0x4ea, 0x4f4);
    crate::sb_misc::node_append(g, img, call(0x4f5), w(0x4e6), w(0x4e1), 0);
    abs(g, 0x501, 0x529);
    // [esp + 0x30] = ecx, [esp + 0x20] = edx: the node and its parent kept
    let grp = s + 0x1c;
    put_words(
        g,
        grp,
        &[w(0x52d), w(0x4e1), w(0x539), w(0x541), w(0x549), w(0x4e6)],
    );
    let inf = Frame {
        at: call(0x54e),
        esp: s - 4,
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &inf);
    abs(g, 0x561, 0x568);
    let pf = frame_4(g, img, f + 0x569, s, w(0x558), w(0x55d), 1);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    // push 0; push ecx (never read); push A; push B
    let ef = frame_4(g, img, f + 0x583, s, w(0x57e), w(0x579), 0);
    g.write::<u32>(s - 0xc, w(0x574));
    crate::s3_nodes::effect_node_with(g, img, &ef);
    let x = g.read::<u32>(w(0x589)) | 1;
    g.write::<u32>(w(0x589), x);
    abs(g, 0x58e, 0x598);
    crate::m0042::set_value(g, img, call(0x5a4), g.read::<u32>(at(0x1c)), 0x38)
}

/// [`summon_task_0201`]'s state 0x15 when slot 532 says 0: appended and
/// inserted nodes, the model's place, the facing, a particle node, the
/// colour block, the summon set-up ([`summon_setup_0201`]) and slot 295.
/// Returns slot 295's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_task_0201_summon<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    crate::sb_misc::node_append(g, img, call(0x5f6), w(0x5f1), w(0x5ec), 0);
    crate::sa_misc::model_position(g, img, call(0x603), g.read::<u32>(at(0x1c)), w(0x5fe));
    let x = sub(k(g, 0x60b), k(g, 0x613));
    abs(g, 0x621, 0x62b);
    g.write::<f32>(w(0x62f), x);
    crate::sa_misc::node_append(g, img, call(0x634), w(0x61d), w(0x618), 0);
    crate::sa_misc::facing_int(g, img, call(0x643), w(0x63e), w(0x639), s - 4);
    crate::sb_misc::node_append(g, img, call(0x652), w(0x64d), w(0x648), 0);
    put_words(g, s - 0x10, &[w(0x662), w(0x65d), w(0x658), w(0x64d)]);
    abs(g, 0x66b, 0x693);
    let inf = Frame {
        at: call(0x694),
        esp: s - 0x14,
        ecx: w(0x667),
        edx: w(0x648),
        ..Frame::default()
    };
    crate::sa_misc::insert_with(g, img, &inf);
    abs(g, 0x6a5, 0x6ac);
    let pf = frame_4(g, img, f + 0x6b2, s, w(0x69c), w(0x6ad), 2);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    // the colour block
    let c = sub(k(g, 0x6ba), 0.0);
    let fl = g.read::<u8>(w(0x6c0)) | 1;
    g.write::<u8>(w(0x6c0), fl);
    abs(g, 0x6cc, 0x6f4);
    g.write::<f32>(w(0x6f8), c);
    // the set-up (the particle node's four words still pushed)
    put_words(g, s - 0x18, &[w(0x705), w(0x700)]);
    let sf = Frame {
        at: call(0x70a),
        esp: s - 0x1c,
        edx: g.read::<u32>(at(0x1c)),
        ..Frame::default()
    };
    summon_setup_0201(g, img, &sf);
    let b = u32::from(g.read::<u8>(w(0x711)));
    // call width: 2
    g.call(t(g, 0x719, 295), 0, &[b, 0]).eax
}

/// [`summon_task_0201`] (the `rel32`s: its callees in order).
pub const SUMMON_TASK_0201: Sig = Sig {
    len: 2276,
    hash: 0x8072_e3e0_2f49_1e8e,
    calls: &[
        (0xdb, Callee::Any),
        (0x113, Callee::Sig(&crate::s4_small::SUMMON_NODES)),
        (0x17c, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x28a, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x357, Callee::Sig(&crate::nodes::INSERT)),
        (
            0x36c,
            Callee::Sig(&crate::o6_fam_bone::BONE_NODE_SETUP_0201_6F20),
        ),
        (0x374, Callee::Sig(&crate::s4_small::STATIC_LINK_LAST)),
        (0x38b, Callee::Any),
        (0x39e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3e9, Callee::Sig(&crate::s4_small::FACE_POINT)),
        (0x3fc, Callee::Any),
        (0x431, Callee::Sig(&crate::r21::MOTION_FROM_0201)),
        (0x448, Callee::Any),
        (0x45d, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x477, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x49a, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x4a9, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x4b8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4f5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x54e, Callee::Sig(&crate::nodes::INSERT)),
        (0x569, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x583, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x5a4, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x5bb, Callee::Any),
        (0x5f6, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x603, Callee::Sig(&crate::sa_misc::MODEL_POSITION)),
        (0x634, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x643, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x652, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x694, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x6b2, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x70a, Callee::Sig(&SUMMON_SETUP_0201)),
        (0x737, Callee::Any),
        (0x75a, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x78f, Callee::Any),
        (0x7e0, Callee::Sig(&crate::s5_misc::MOTION_TOWARDS)),
        (0x7f3, Callee::Any),
        (0x808, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x81e, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x831, Callee::Any),
        (0x83f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x84c, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x876, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0150`'s summon task (0x1000ccf0, 884 bytes; a task callback
/// `(A)`, cdecl; an 8-aligned frame with one word, `S = ((E - 4) & !7) -
/// 0x10` below the three pushes; the record `R = A[4]`, its timer `R +
/// 0x18` up by the constant at `+0x19` at every call, its state the byte
/// `R + 0x1c`). The states run on into each other while their timers
/// allow:
/// - 0: slot 719 `(N, 5, k)` (`k` 0, 1 or 2 by two globals) handed to slot
///   306 ([`slot_7_0_arg`](crate::o4_tiny::slot_7_0_arg)), the timer 0;
/// - 1 waits for the constant at `+0x83`; 2 past the one at `+0x9d` sets up
///   the first nodes (appended, owned, a particle and an effect node);
/// - 3: slot 248, the swing `R + 0x5f4` up; below 0 it waits; 4 waits for
///   the constant at `+0x192`, then the second nodes (two appended, owned,
///   particle, the child node, effect);
/// - 5 waits for the constant at `+0x283`, then the scene
///   ([`scene_ctor_p030`](crate::r11_misc::scene_ctor_p030)) and an effect
///   node; 6: slot 411; once it answers slot 213, two nodes leave and get
///   their model set-up, `A`'s node leaves its ring.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let a = fr.arg(g, 0);
    let r = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x10);
    let timer = add(rf(g, 0x18), k(g, 0x19));
    let st = u32::from(g.read::<u8>(at(0x1c)));
    g.write::<f32>(at(0x18), timer);
    if st > 6 {
        return Ret::eax(st);
    }
    let mut eax = st;
    let mut st = st;
    if st == 0 {
        let kind = if g.read::<u32>(w(0x37)) != 1 {
            0
        } else if g.read::<u32>(w(0x44)) == 0x1071 {
            1
        } else {
            2
        };
        // call width: 3
        let h = g.call(t(g, 0x55, 719), 0, &[w(0x62), 5, kind]).eax;
        g.write::<u32>(s - 4, h);
        let sf = Frame {
            at: call(0x6d),
            esp: s - 8,
            ..Frame::default()
        };
        eax = crate::o4_tiny::slot_7_0_arg(g, img, &sf).eax;
        g.write::<u32>(at(0x18), 0);
        g.write::<u8>(at(0x1c), 1);
        st = 1;
    }
    if st == 1 {
        // comiss K, t; ja: below K waits
        if k(g, 0x83) > rf(g, 0x18) {
            return Ret::eax(eax);
        }
        g.write::<u8>(at(0x1c), 2);
        st = 2;
    }
    if st == 2 {
        if rf(g, 0x18) > k(g, 0x9d) {
            summon_task_0150_first(g, img, f, s, r);
        }
        st = 3;
    }
    if st == 3 {
        // call width: 3
        eax = g
            .call(t(g, 0x146, 248), 0, &[w(0x156), w(0x14b), at(0x5f0)])
            .eax;
        let x = add(rf(g, 0x5f4), k(g, 0x168));
        g.write::<f32>(at(0x5f4), x);
        // comiss 0, x; ja: below 0 waits
        if 0.0 > x {
            return Ret::eax(eax);
        }
        g.write::<u32>(at(0x60), 0x3dcc_cccd);
        g.write::<u8>(at(0x1c), 4);
        st = 4;
    }
    if st == 4 {
        if k(g, 0x192) > rf(g, 0x18) {
            return Ret::eax(eax);
        }
        return Ret::eax(summon_task_0150_second(g, img, f, s, r));
    }
    if st == 5 {
        if k(g, 0x283) > timer {
            return Ret::eax(eax);
        }
        let p = at(0x118);
        let sf = Frame {
            at: call(0x29f),
            esp: s - 4,
            ecx: at(0x600),
            edx: p,
            ..Frame::default()
        };
        crate::r11_misc::scene_ctor_p030(g, img, &sf);
        // push 2; push ecx (never read); push A; push B
        put_words(g, s - 0x10, &[p, w(0x2a7)]);
        g.write::<u32>(s - 4, 2);
        let ef = Frame {
            at: call(0x2b2),
            esp: s - 0x14,
            ecx: w(0x2ad),
            ..Frame::default()
        };
        let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
        let x = g.read::<u32>(w(0x2b8)) | 1;
        g.write::<u32>(w(0x2b8), x);
        abs(g, 0x2c0, 0x2ca);
        let b = g.read::<u8>(at(0x1c)).wrapping_add(1);
        g.write::<u8>(at(0x1c), b);
        return Ret::eax(eax);
    }
    // state 6
    let x = g.read::<u32>(w(0x2db));
    // call width: 1
    if g.call(t(g, 0x2d5, 411), 0, &[x]).eax == 0 {
        return Ret::eax(0);
    }
    let x = g.read::<u32>(w(0x2f7));
    // call width: 2
    let mut eax = g.call(t(g, 0x2ef, 213), 0, &[x, 0]).eax;
    for (node_op, setup_at) in [(0x307u32, 0x319u32), (0x31e, 0x330)] {
        let node = w(node_op);
        // the node leaves (`eax` its own when it has no parent)
        crate::sa_misc::leave(g, node, eax);
        g.write::<u32>(s + 0xc, node);
        let mf = Frame {
            at: call(setup_at),
            esp: s - 4,
            ecx: s + 0xc,
            ..Frame::default()
        };
        eax = crate::sa_summon::model_setup(g, img, &mf).eax;
    }
    let n = g.read::<u32>(a);
    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
    g.write::<u32>(nx.wrapping_add(8), pv);
    let (nx, pv) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
    g.write::<u32>(pv.wrapping_add(4), nx);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    let _ = eax;
    Ret::eax(nx)
}

/// [`summon_task_0150`]'s state 2 past its time: the node `R + 0x370`
/// appended, `R + 0x39c` owned ([`node_append_owned`](crate::sa_misc::node_append_owned)),
/// a particle node and an effect node; the state 3.
fn summon_task_0150_first<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) {
    let w = |o: u32| img.word(f + o);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    crate::sb_misc::node_append(g, img, call(0xb3), at(0x370), w(0xae), 0);
    let p = at(0x39c);
    put_words(g, s - 0x10, &[w(0xc9), w(0xc4), w(0xb9), at(0x370)]);
    for (o, v) in [
        (0x38cu32, ONE),
        (0x390, ONE),
        (0x394, ONE),
        (0x398, 0x3f19_999a),
    ] {
        g.write::<u32>(at(o), v);
    }
    let of = Frame {
        at: call(0xf8),
        esp: s - 0x14,
        ecx: p,
        edx: w(0xae),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    let pf = frame_4(g, img, f + 0x10c, s, at(0x4f0), p, 3);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    g.write::<u8>(at(0x1c), 3);
    // push 0; push ecx (never read); push A; push B
    put_words(g, s - 0x10, &[p, w(0x11b)]);
    g.write::<u32>(s - 4, 0);
    let ef = Frame {
        at: call(0x126),
        esp: s - 0x14,
        ecx: w(0x121),
        ..Frame::default()
    };
    crate::s3_nodes::effect_node_with(g, img, &ef);
    let x = g.read::<u32>(w(0x12f)) | 1;
    g.write::<u32>(w(0x12f), x);
    abs(g, 0x134, 0x13e);
}

/// [`summon_task_0150`]'s state 4 past its time: two nodes appended,
/// `R + 0x118` owned, a particle node, the child node
/// ([`child_node`](crate::s4_small::child_node)) `R + 0x70` and an effect
/// node; the state 5. Returns the effect node's `eax`.
fn summon_task_0150_second<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, r: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let abs = |g: &mut G, a: u32, b: u32| crate::r25::abs_stores(g, img, f, a, b);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    let (e, c0, p) = (at(0xec), at(0xc0), at(0x118));
    crate::sa_misc::node_append(g, img, call(0x1ae), e, w(0x1a7), 0);
    crate::sb_misc::node_append(g, img, call(0x1b9), c0, w(0x1a7), 0);
    put_words(g, s - 0x10, &[w(0x1cb), w(0x1c6), e, c0]);
    for (o, v) in [
        (0xdcu32, ONE),
        (0xe0, ONE),
        (0xe4, ONE),
        (0xe8, 0x3e05_1eb8),
    ] {
        g.write::<u32>(at(o), v);
    }
    let of = Frame {
        at: call(0x1fa),
        esp: s - 0x14,
        ecx: p,
        edx: w(0x1a7),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    g.write::<u8>(at(0x260), 1);
    let pf = frame_4(g, img, f + 0x215, s, at(0x270), p, 0);
    crate::s3_nodes::particle_node_with(g, img, &pf);
    // push y; push ecx (x, never read)
    g.write::<u32>(s - 4, w(0x21f));
    let cf = Frame {
        at: call(0x228),
        esp: s - 0xc,
        ecx: at(0x70),
        edx: e,
        ..Frame::default()
    };
    crate::s4_small::child_node(g, img, &cf);
    for o in [0xb0u32, 0xb4, 0xb8] {
        g.write::<u32>(at(o), ONE);
    }
    // push 1; push ecx (never read); push A; push B
    put_words(g, s - 0x10, &[p, w(0x23d)]);
    g.write::<u32>(s - 4, 1);
    let ef = Frame {
        at: call(0x25c),
        esp: s - 0x14,
        ecx: w(0x243),
        ..Frame::default()
    };
    let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
    let x = g.read::<u32>(w(0x265)) | 1;
    g.write::<u32>(w(0x265), x);
    abs(g, 0x26a, 0x274);
    g.write::<u8>(at(0x1c), 5);
    eax
}

/// [`summon_task_0150`] (the `rel32`s: its callees in order).
pub const SUMMON_TASK_0150: Sig = Sig {
    len: 884,
    hash: 0xab36_7b20_0dad_d943,
    calls: &[
        (0x6d, Callee::Sig(&crate::o4_tiny::SLOT_7_0_ARG_0150_EAF0)),
        (0xb3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf8, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x10c, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x126, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1ae, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x1b9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1fa, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x215, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x228, Callee::Sig(&crate::s4_small::CHILD_NODE)),
        (0x25c, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x29f, Callee::Sig(&crate::r11_misc::SCENE_CTOR_0150)),
        (0x2b2, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x30c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x319, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x323, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x330, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0042`'s light visit (0x10001350, 858 bytes; internal, `ecx` =
/// the cursor `C`, reached by its 12-byte thunk 0x100016b0; a /GS frame,
/// `ebp` = `E - 4`): the node walk ([`walk`](crate::nodes::walk)); when
/// it ran and bit 0 of `N + 0x68` is set with `N + 0x28` above `K`
/// (the operand at `+0x7b`): slot 231 `(ebp - 0x44, M, N[0x70] + 0x9c)`,
/// slot 245 `(ebp - 0x4c)`, the sprite packet
/// ([`sprite_packet`](crate::s5_misc::sprite_packet)) of the record at
/// `ebp - 0x80` (the matrix, `N[0x6c] + 0x1c`, `N + 0x1c`, the size
/// `N + 0x5c`, the packet cursor `*G_C + 8`, 0x44, 0x100), slot 563 on
/// the packet's words (`*G_C` the new cursor), slots 821 `(0)`, 822 with
/// `N + 0x1c..0x28` times `K2`, 823 `(s, s)` with `s = N[0x4c] + K3`, 824
/// `(A + 0x1c, A + 0x20, A + 0x24, 1.0)` (`A` = `N[0x6c]`). Then the moves
/// of `N + 0x1c..` and `N + 0x5c` as
/// [`two_part_visit_0609`](crate::r29::two_part_visit_0609)'s. Returns
/// `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn light_visit_0042<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let ebp = esp.wrapping_sub(4);
    let eax = g.read::<u32>(w(0x0a)) ^ ebp;
    let (mut eax, tail) = crate::nodes::walk(g, this, this, eax);
    let Some(n) = tail else {
        return eax;
    };
    let rf = |g: &G, o: u32| g.read::<f32>(n.wrapping_add(o));
    let mut one = k(g, 0x62);
    if g.read::<u8>(n + 0x68) & 1 != 0 && rf(g, 0x28) > k(g, 0x7b) {
        let m = ebp.wrapping_sub(0x44);
        let b = g.read::<u32>(n + 0x70).wrapping_add(0x9c);
        let mut st = crate::pushes::Pushes::default();
        st.push_all(&[b, w(0x92), m]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x98, 231), 0);
        st.push(ebp.wrapping_sub(0x4c));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xa9, 245), 0);
        let cur = g.read::<u32>(w(0xb7));
        let rec = ebp.wrapping_sub(0x80);
        let x = g.read::<u32>(n + 0x5c);
        g.write::<u32>(rec, m);
        g.write::<u32>(rec + 4, g.read::<u32>(n + 0x6c).wrapping_add(0x1c));
        g.write::<u32>(ebp.wrapping_sub(0x60), 0x44);
        g.write::<u32>(ebp.wrapping_sub(0x54), cur.wrapping_add(8));
        g.write::<u16>(ebp.wrapping_sub(0x50), 0x100);
        g.write::<u32>(rec + 8, n + 0x1c);
        g.write::<u32>(ebp.wrapping_sub(0x58), x);
        crate::s5_misc::sprite_packet(g, img, img.rel32_target(f + 0xf1), rec);
        let next = g.read::<u32>(ebp.wrapping_sub(0x54));
        st.push_all(&[0, 0, next.wrapping_sub(cur) >> 4, cur]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x105, 563), 0);
        g.write::<u32>(w(0x119), next);
        let s = add(rf(g, 0x4c), k(g, 0x12f));
        g.write::<f32>(ebp.wrapping_sub(0x48), s);
        st.push(0);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x113, 821), 0);
        // add esp, 0x14: the words of slots 245 and 231 written over
        st.pop(5);
        let k2 = k(g, 0x140);
        let scaled = |g: &G, o: u32| mul(rf(g, o), k2).to_bits();
        st.pop(4);
        st.push_all(&[
            scaled(g, 0x28),
            scaled(g, 0x24),
            scaled(g, 0x20),
            scaled(g, 0x1c),
        ]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x14a, 822), 0);
        // add esp, 8: both words left written over with s
        st.pop(4);
        st.push_all(&[s.to_bits(), s.to_bits()]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x194, 823), 0);
        let a = g.read::<u32>(n + 0x6c);
        st.pop(2);
        st.push_all(&[
            ONE,
            g.read::<u32>(a.wrapping_add(0x24)),
            g.read::<u32>(a.wrapping_add(0x20)),
            g.read::<u32>(a.wrapping_add(0x1c)),
        ]);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, t(g, 0x1d7, 824), 0).eax;
        one = k(g, 0x1ec);
    }
    if g.read::<u8>(n + 0x68) & 2 != 0 {
        let rate = |g: &G| rf(g, 0x4c);
        for (x, to) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            let v = add(mul(sub(rf(g, to), rf(g, x)), rate(g)), rf(g, x));
            g.write::<f32>(n + x, v);
        }
        let v = add(mul(sub(rf(g, 0x60), rf(g, 0x5c)), rf(g, 0x54)), rf(g, 0x5c));
        g.write::<f32>(n + 0x5c, v);
        return eax;
    }
    // comiss, jae: taken when not below (ordered); jb: below or unordered
    let at_end = |x: f32| !(x < one || x.is_nan());
    let copy = |g: &mut G, from: u32, to: u32| {
        let v = g.read::<u32>(n + from);
        g.write::<u32>(n + to, v);
        v
    };
    let mut done = at_end(rf(g, 0x50));
    if !done {
        for (x, v) in [(0x1cu32, 0x3cu32), (0x20, 0x40), (0x24, 0x44), (0x28, 0x48)] {
            let y = add(mul(rf(g, v), rf(g, 0x4c)), rf(g, x));
            g.write::<f32>(n + x, y);
        }
        let step = add(rf(g, 0x50), rf(g, 0x4c));
        g.write::<f32>(n + 0x50, step);
        done = at_end(step);
    }
    if done {
        for (to, from) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            eax = copy(g, from, to);
        }
    }
    let fade = rf(g, 0x58);
    if !at_end(fade) {
        let y = mul(rf(g, 0x64), rf(g, 0x54));
        let fade = add(fade, rf(g, 0x54));
        let y = add(y, rf(g, 0x5c));
        g.write::<f32>(n + 0x58, fade);
        g.write::<f32>(n + 0x5c, y);
        if !at_end(rf(g, 0x50)) {
            return eax;
        }
    }
    copy(g, 0x60, 0x5c)
}

/// [`light_visit_0042`] (the `rel32`s: the sprite packet,
/// `__security_check_cookie` twice).
pub const LIGHT_VISIT_0042: Sig = Sig {
    len: 858,
    hash: 0xd8c0_d1a9_cc1e_1961,
    calls: &[
        (0xf1, Callee::Sig(&crate::s5_misc::SPRITE_PACKET)),
        (0x283, Callee::Any),
        (0x352, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0042`'s 12-byte thunk over [`light_visit_0042`] (0x100016b0:
/// `ecx` = the first stack word, then a jump with the caller's stack).
pub fn jmp_light_visit_0042<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let f = img.rel32_target(fr.at + 8);
    Ret::eax(light_visit_0042(g, img, f, fr.esp, this))
}

/// [`jmp_light_visit_0042`].
pub const JMP_LIGHT_VISIT_0042: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&LIGHT_VISIT_0042))],
    imports: &[],
};
