//! `magic_0164.dll` against its Rust ports (`ffx_magic::m0164`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0164};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::{f, rec, row, twin};

const N: u16 = 164;

/// 0x1000c190: the summon's frame before its phase, with and without
/// children, the flag set and clear, the done bit and the closing slot
/// each way.
#[test]
fn summon_frame_214() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xc190);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let bind = t.img.rel32_target(at + 0x23);
    t.set_u32(t.img.word(bind + 0x7e), SCRATCH + 0x500);
    for (k, s) in (0u32..).zip([723u32, 724, 230, 253, 214]) {
        let row = SCRATCH + 0x3000 + 0x40 * k;
        t.stub(s, move |m, a| {
            let c = m.r32(row + 0x3c).wrapping_add(1);
            m.w32(row + 0x3c, c);
            // slot 253 takes no word and answers whether the binding
            // failed (it decides the flag the frame reads at +0x4a)
            if s == 253 {
                return m.r32(SCRATCH + 0x37f0);
            }
            m.w32(row + 4 * (c % 8), a[0]);
            0x0300_0000 + s
        });
    }
    // the children's method +0x20: slot 1000, logging its cursor's node
    // and pass, and emptying the root's list on the last child
    let first = w(&t, 0x51);
    let root = w(&t, 0x59);
    t.stub(1000, move |m, a| {
        let k = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, k);
        m.w32(SCRATCH + 0x3780 + 4 * (k % 16), m.r32(a[0]));
        m.w32(SCRATCH + 0x3740 + 4 * (k % 16), m.r32(a[0] + 4));
        if m.r32(m.r32(a[0]) + 8) == root {
            m.w32(first, root);
        }
        0x1000
    });
    let vt = SCRATCH + 0x800;
    t.set_u32(magic::TABLE + 4 * 1000, magic::TRAPS + 16 * 1000);
    t.set_u32(vt + 0x20, magic::TRAPS + 16 * 1000);
    t.set_u32(w(&t, 0x18), SCRATCH + 0x400);
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    let kid = |i: u32| SCRATCH + 0x1000 + 0x100 * i;
    for (phase, flag, kids, a, half, pass) in [
        (3u32, 0u32, 0u32, 1u32, 0u16, 0x41u8),
        (4, 0, 2, 1, 0, 0x41),
        (5, 0, 0, 1, 0, 0x41),
        (5, 1, 2, 1, 0, 0x41),
        (5, 1, 0, 1, 1, 0x41),
        (6, 0, 1, 0, 0, 0xff),
        (7, 1, 1, 3, 0, 0x41),
        (0xffff_ffff, 0, 1, 1, 0, 0x41),
    ] {
        t.set_u32(w(&t, 0x0b), phase);
        t.set_u32(w(&t, 0x4a), flag);
        t.set_u32(SCRATCH + 0x37f0, u32::from(flag == 0));
        // the done bit starts clear except in the phase-6 case (the done
        // word is also the second word the closing test ANDs, +0xd9)
        t.set_u32(w(&t, 0x90), 0x10 | u32::from(phase == 6));
        t.set_u32(w(&t, 0xd3), 0x20 | a);
        t.set_u16(w(&t, 0xe7), half);
        t.set_u32(w(&t, 0xf5), 0x00f5_0000);
        t.set_u8(w(&t, 0x28), pass);
        for i in 0..kids {
            t.write(kid(i), &[0x22; 0x40]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { root } else { kid(i + 1) });
        }
        t.set_u32(first, if kids == 0 { root } else { kid(0) });
        // the bone node holds still (its pass is the new one); the first
        // gated one holds still too (the same pass byte), the second moves
        // (no children)
        let bone = w(&t, 0x3b);
        t.set_u32(bone + 0x10, bone);
        t.set_u8(bone + 0x1a, pass.wrapping_add(1));
        for (node, p) in [(w(&t, 0xa9), pass.wrapping_add(1)), (w(&t, 0xc9), 0)] {
            t.set_u32(node + 0x10, node);
            t.set_u8(node + 0x1a, p);
            t.set_u8(node + 0x1b, 0);
        }
        let c = Call {
            eax: 0x00ea_0000 + phase,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}

/// 0x10007460 (internal, `ecx` = `[fx, out, model]`): the flag path,
/// and the mean over no, one and three parts (and a negative count).
#[test]
fn mean_point() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7460);
    rec(&mut t, 234, 3, |_, _| 0x234);
    // slot 325: the base vector (four words) at its first argument
    rec(&mut t, 325, 2, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[0] + 4 * i,
                f(1.5 - 0.75 * f32::from(u8::try_from(i).unwrap())),
            );
        }
        0x325
    });
    // slot 268: a point (four words) from the part's model word
    rec(&mut t, 268, 3, |m: &mut dyn StubMem, a: &[u32]| {
        let k = m.r32(row(268));
        for i in 0..4 {
            let v = f32::from(u16::try_from((a[0] + 3 * i + k) & 0xff).unwrap()) * 0.375 - 7.0;
            m.w32(a[2] + 4 * i, f(v));
        }
        0x268
    });
    let fx = SCRATCH + 0x1_0000;
    let r = SCRATCH + 0x100;
    let out = SCRATCH + 0x200;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for (flags, n) in [(8u8, 2i32), (0, 0), (0, 1), (0xf7, 3), (0, -1)] {
        t.write(fx + 0x100, &[0x33; 0x10]);
        t.set_u8(fx + 0x1640, flags);
        t.set_u32(fx + 0x162c, n.cast_unsigned());
        for i in 0..4u32 {
            t.set_u32(fx + 0x110 + 0x274 * i, 0x40 + 9 * i);
        }
        t.set_u32(r, fx);
        t.set_u32(r + 4, out);
        t.set_u32(r + 8, 0x00de_0000);
        t.write(out, &[0x44; 0x10]);
        t.check(at, Call::this(r), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0164::mean_point(g, img, fr))
        });
    }
}
