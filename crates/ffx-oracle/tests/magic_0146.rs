//! `magic_0146.dll` against its Rust port (`ffx_magic::m0146`): the
//! model nodes with a picked one and the node move start (both
//! internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0146};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, recs, twin};

const N: u16 = 146;

/// 0x10006000: no, one and three models, under no parent and under one,
/// the pick matching a model or none.
#[test]
fn model_nodes_pick() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6000);
    let (grp, node, parent, list, pick) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x2000,
        SCRATCH + 0x200,
        SCRATCH + 0x280,
    );
    t.write(list, &[4, 0x81, 7, 0xff]);
    let n = SCRATCH + 0x300;
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0x5a5a_5a5a);
        pick
    });
    rec(&mut t, 368, 4, move |m, a| {
        m.w32(a[1], m.r32(n));
        list
    });
    recs(&mut t, &[(413, 1), (215, 1), (14, 2)]);
    t.stub_st0(333, |_, _| 0.625);
    for (count, p, flags, picked) in [
        (0u32, 0u32, 0u8, 4u8),
        (1, parent, 1, 4),
        (3, parent, 0xfe, 7),
        (3, 0, 3, 9),
    ] {
        t.write(node, &[0x66; 0x500]);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.write(parent, &[0x77; 0x40]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(n, count);
        t.write(pick, &[picked, 0x55, 0x56, 0x57]);
        t.set_u32(grp, node);
        t.set_u32(grp + 4, p);
        t.set_u32(grp + 8, 0x00ac_0000);
        t.set_u32(grp + 0xc, 0x0c0c_0c0c);
        t.set_u32(grp + 0x10, 0x5a5a_5a00 | u32::from(flags));
        t.check(at, Call::this(grp), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0146::model_nodes_pick(g, img, fr))
        });
    }
}

/// 0x10006ba0: the move start for two nodes and times.
#[test]
fn move_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ba0);
    let node = SCRATCH + 0x400;
    for (k, time) in [(0u32, 1.5f32), (1, -0.25)] {
        for i in 0..0x2c {
            #[allow(clippy::cast_precision_loss)]
            t.set_u32(node + 4 * i, f(0.5 + (i + 7 * k) as f32 * 1.25));
        }
        // the flags with bit 0 set (the start clears it)
        t.set_u32(node + 0xac, 0x8000_0003);
        let c = Call {
            eax: 0x00ea_0000 + k,
            ecx: node,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(9.0),
            ffx_magic::Xmm::f32(8.0),
            ffx_magic::Xmm::f32(time),
        ]);
        t.check(at, c, &[], Eax::Checked, |g, _, fr| {
            m0146::move_start(g, fr.ecx, fr.xmm_f32(2));
            Ret::eax(fr.eax)
        });
    }
}

/// 0x10006190: the handler on a node already visited, on one with no
/// children, two children, and with the flag bit set or clear.
#[test]
fn sub_node_visit() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6190);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    // the methods (+0x1c of every vtable) and the copy behind the
    // pointer global: slots 1000 and 1001, logging their cursor's node
    for s in [1000u32, 1001] {
        t.set_u32(magic::TABLE + 4 * s, magic::TRAPS + 16 * s);
    }
    rec(&mut t, 1000, 1, |m, a| {
        let c = m.r32(SCRATCH + 0x37fc).wrapping_add(1);
        m.w32(SCRATCH + 0x37fc, c);
        // the cursor's node and its pass byte
        let pass = (m.r32(a[0] + 4) >> 8) & 0xff;
        m.w32(SCRATCH + 0x3780 + 4 * (c % 16), m.r32(a[0]) ^ (pass << 24));
        0x1000 + c
    });
    rec(&mut t, 1001, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0101);
        }
        0x1001
    });
    t.set_u32(w(&t, 0x10a), magic::TRAPS + 16 * 1001);
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 677, 4, |_, a| a[0] ^ 0x677);
    rec(&mut t, 679, 1, |m, a| {
        for i in 0..6 {
            let v = m.r32(a[0] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        0x679
    });
    rec(&mut t, 680, 11, |m, a| {
        for i in 0..2 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3840 + 4 * i, v);
        }
        0x680
    });
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1000);
    let (cur, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    let kid = |i: u32| SCRATCH + 0x1200 + 0x100 * i;
    let sub = |i: u32| SCRATCH + 0x1800 + 0x100 * i;
    for (kids, same, flag) in [
        (0u32, true, 0u8),
        (0, false, 1),
        (2, false, 0),
        (1, false, 3),
    ] {
        t.write(node, &[0x22; 0x60]);
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u8(node + 0x54, flag);
        for i in 0..3 {
            t.write(sub(i), &[0x33; 0x40]);
            t.set_u32(sub(i), vt);
            t.set_u32(node + 0x44 + 4 * i, sub(i));
        }
        for i in 0..kids {
            t.write(kid(i), &[0x44; 0x20]);
            t.set_u32(kid(i), vt);
            t.set_u32(kid(i) + 8, if i + 1 == kids { node } else { kid(i + 1) });
        }
        t.set_u32(node + 0x10, if kids == 0 { node } else { kid(0) });
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        // junk under the entry: the frame's locals must be written
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::this(cur), &[cur], Eax::Checked);
    }
}
