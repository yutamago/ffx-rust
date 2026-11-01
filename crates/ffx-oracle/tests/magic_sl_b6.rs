//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 5: hosted object starts (`ffx_magic::sl_host`). Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

mod magic;
mod magic_owna;

use ffx_magic::{Ret, Xmm, sl_host};
use magic::{Call, Eax, SCRATCH, STACK_TOP, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// `n` bytes that differ from word to word (seeded by `seed`), so a port
/// that reads the wrong word of a block reads another value.
fn varied(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| u8::try_from((i * 7 + usize::from(seed)) % 251).unwrap())
        .collect()
}

/// One copy at `rva`: the pool with free nodes or none, the list empty
/// or holding a node.
fn hosted(
    n: u16,
    rva: u32,
    l: &sl_host::Layout,
    port: fn(&mut magic::PortGame, &ffx_magic::Image, &ffx_magic::Frame) -> Ret,
) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let sp = t.img.rel32_target(at + l.calls.0);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(sp + 0x29), SCRATCH + 0x200);
    t.set_u32(SCRATCH + 0x200, 0x0000_0077);
    rec(&mut t, 714, 5, |_, a| a[2] ^ 0x714);
    let table = SCRATCH + 0x400;
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0900_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    let gh = SCRATCH + 0x300;
    t.set_u32(w(&t, l.host.0), gh);
    let host = SCRATCH + 0x340;
    t.set_u32(w(&t, l.host.1), host);
    t.set_u32(host + 0x10, 0x00d0_0010);
    let pa = t.img.rel32_target(at + l.calls.2);
    let nodes = t.img.word(pa + 0x27) - 8;
    let obj = SCRATCH + 0x1000;
    let last = SCRATCH + 0x700;
    for (count, full) in [(3u16, false), (2, true), (0, false)] {
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        t.set_u16(t.img.word(pa + 0x04), count);
        t.write(last, &varied(0x10, 0x3a));
        t.set_u32(w(&t, l.list.0), if full { last } else { 0 });
        t.set_u32(w(&t, l.list.1), if full { last } else { 0 });
        t.set_u32(w(&t, l.list.2), 5);
        t.write(obj, &varied(0x130, 0x5a));
        dirty(&mut t);
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(1.75)]);
        t.check(at, c, &[1, 0x00ee_0002, 3, 0x00ee_0004], Eax::Checked, port);
    }
}

#[test]
fn hosted_0391() {
    hosted(391, 0x6620, &sl_host::L0391, sl_host::hosted_0391);
}

#[test]
fn hosted_0448() {
    hosted(448, 0x3740, &sl_host::L0448, sl_host::hosted_0448);
}

#[test]
fn hosted_0444() {
    hosted(444, 0x1e90, &sl_host::L0444, sl_host::hosted_0444);
}

#[test]
fn hosted_0199() {
    hosted(199, 0x2c40, &sl_host::L0199, sl_host::hosted_0199);
}

/// Every slot in `slots` recorded with its word count, answering a
/// distinct value mixed with its first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// `m0300::object_setup` (0x10001600).
#[test]
fn object_setup_0300() {
    let Some(mut t) = twin(300) else { return };
    let at = t.at(0x1600);
    recs(&mut t, &[(442, 3), (443, 2), (440, 4), (450, 2), (449, 2)]);
    let e = SCRATCH + 0x500;
    t.set_u32(t.img.word(at + 0x4), e);
    t.set_u32(e + 0x1a0, 0x00e0_01a0);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x140, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0301::aimed_setup` (0x10001ea0): two point indices.
#[test]
fn aimed_setup_0301() {
    let Some(mut t) = twin(301) else { return };
    let at = t.at(0x1ea0);
    recs(&mut t, &[(440, 4), (450, 2), (451, 2)]);
    t.set_u32(t.img.word(at + 0x55), 0x00e0_0055);
    let base = t.img.word(at + 0x23);
    for i in 0..12u32 {
        t.set_u32(base + 4 * i, (1.5 * i as f32 - 4.0).to_bits());
    }
    let a = SCRATCH + 0x3000;
    for (k, i) in [0u32, 1].into_iter().enumerate() {
        t.write(a, &varied(0x140, 0x44));
        for j in 0..3u32 {
            t.set_u32(a + 0xd0 + 4 * j, (0.25 * j as f32 + k as f32).to_bits());
        }
        t.set_u32(a + 0xac, i);
        t.set_u32(a + 0x2c + 16 * i, 2 - i);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[a], Eax::Checked);
    }
}

/// `m0301::thrown_setup` (0x10001fb0): even, odd and negative steps.
#[test]
fn thrown_setup_0301() {
    let Some(mut t) = twin(301) else { return };
    let at = t.at(0x1fb0);
    let obj = SCRATCH + 0x5000;
    rec(&mut t, 434, 2, move |_, _| obj);
    rec(&mut t, 477, 1, |_, a| a[0] ^ 0x477);
    t.stub_st0(477, |_, a| f64::from(a[0] & 0xff) * 0.375 - 3.0);
    recs(&mut t, &[(450, 3), (449, 5)]);
    t.set_u32(t.img.word(at + 0x35), 0x00e0_0035);
    let a = SCRATCH + 0x3000;
    for (k, e) in [4u32, 7, 0xffff_fffd].into_iter().enumerate() {
        t.write(a, &varied(0x140, 0x44));
        t.set_u32(a + 0xac, 1);
        t.set_u32(a + 0x3c, e);
        t.write(obj, &varied(0x140, 0x55));
        let tab = t.img.word(at + 0x4b);
        let half = (e.cast_signed() / 2).cast_unsigned();
        t.set_u32(
            tab.wrapping_add(half.wrapping_mul(4)),
            0x40 + u32::try_from(k).unwrap(),
        );
        for o in [0x80u32, 0x8e, 0xad] {
            let p = t.img.word(at + o).wrapping_add(e << 4);
            t.set_u32(p, (1.25 * o as f32).to_bits());
        }
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[a], Eax::Checked);
    }
}

/// `m0300::bone_setup` (0x100014f0).
#[test]
fn bone_setup_0300() {
    let Some(mut t) = twin(300) else { return };
    let at = t.at(0x14f0);
    rec(&mut t, 457, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (0.75 * i as f32 - 1.5).to_bits());
        }
        0x457
    });
    recs(&mut t, &[(440, 4), (450, 2), (449, 2)]);
    let e = SCRATCH + 0x500;
    t.set_u32(t.img.word(at + 0x19), e);
    t.set_u32(e + 0x1a0, 0x00e0_01a0);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x140, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0300::offset_setup` (0x100013c0).
#[test]
fn offset_setup_0300() {
    let Some(mut t) = twin(300) else { return };
    let at = t.at(0x13c0);
    rec(&mut t, 474, 4, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) + 0.5 * i as f32;
            m.w32(a[3] + 4 * i, v.to_bits());
        }
        0x474
    });
    recs(&mut t, &[(440, 4), (450, 2), (449, 2)]);
    let e = SCRATCH + 0x500;
    t.set_u32(t.img.word(at + 0x1d), e);
    t.set_u32(e + 0x198, 0x00e0_0198);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x140, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0415::model_setup` (0x10001b90).
#[test]
fn model_setup_0415() {
    let Some(mut t) = twin(415) else { return };
    let at = t.at(0x1b90);
    recs(
        &mut t,
        &[
            (444, 1),
            (446, 2),
            (442, 3),
            (443, 2),
            (440, 4),
            (450, 2),
            (449, 2),
        ],
    );
    let e = SCRATCH + 0x500;
    t.set_u32(t.img.word(at + 0x26), e);
    for i in 0..4u32 {
        t.set_u32(e + 0x1a0 + 4 * i, 0x00e0_0000 + i);
    }
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x140, 0x44));
    t.set_u32(a + 0xac, 1);
    t.set_u32(a + 0x3c, 2);
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}
