//! `magic_0159.dll` against its Rust port: `DllMain` against every bound
//! port and the functions list 3 ported so far (the character fade, the
//! list unlink, the facing angle, the character show). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_l3;

use ffx_magic::{Ret, l3_common, m0159};
use magic::{Call, Eax, SCRATCH};

const N: u16 = 159;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// Every slot the fades call records its words (`(slot, arity)`) in its
/// own row.
fn record(t: &mut magic::Twin, slots: &[(u32, usize)]) {
    for (k, &(slot, n)) in (0u32..).zip(slots) {
        let row = SCRATCH + 0x2_0000 + 0x40 * k;
        t.stub(slot, move |m, a| {
            for (i, w) in (0u32..).zip(&a[..n]) {
                m.w32(row + 4 * i, *w);
            }
            0x0100 + slot
        });
    }
}

#[test]
fn chr_fade() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    record(&mut t, &[(257, 2), (38, 3), (296, 2), (541, 3)]);
    let f = t.at(0x23f0);
    let k = t.img.word(f + 0x83);
    let kv = f32::from_bits(t.u32(k));
    let this = SCRATCH;
    let rec = SCRATCH + 0x100;
    for (flags, alpha) in [
        (0u16, kv),
        (1, kv),
        (0xfffe, kv + 0.5),
        (0x0f01, 0.25),
        (0, -kv),
        (1, f32::NAN),
        (0x80, f32::INFINITY),
        // below the constant: lahf's carry
        (1, kv - 1.0),
        (0, f32::NEG_INFINITY),
    ] {
        t.set_u32(this, rec);
        t.set_u32(this + 4, alpha.to_bits());
        t.set_u32(this + 8, 0x1234_5678);
        t.set_u16(rec + 0x90, flags);
        t.set_u32(rec + 0x50, 0x0000_0050);
        t.set_u32(rec + 0x54, 0x0000_0054);
        t.set_u32(rec + 0x58, 0xdead);
        t.check(
            f,
            Call {
                eax: 7,
                ecx: this,
                edx: 9,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(l3_common::chr_fade(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn list_unlink_current() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x6590);
    let cur = t.img.word(f + 0x02);
    let first = t.img.word(f + 0x15);
    let last = t.img.word(f + 0x1a);
    let node = SCRATCH + 0x200;
    let (a, b) = (SCRATCH + 0x300, SCRATCH + 0x400);
    // the sentinel's +0x8 is G_LAST itself: unlinking past it moves the
    // last pointer the original reads again
    let sentinel = t.img.word(f + 0x47);
    for (c, fst, lst) in [
        (0, a, b),
        (node, a, b),
        (node, node, b),
        (node, a, node),
        (node, node, node),
        (node, sentinel, b),
        (node, sentinel, node),
        // a node 0x10 below G_LAST: its +0x10 is G_LAST
        (last - 0x10, last - 0x10, b),
    ] {
        t.set_u32(cur, c);
        t.set_u32(first, fst);
        t.set_u32(last, lst);
        t.set_u16(node + 0x18, 3);
        t.write(node, &[0x77; 0x18]);
        t.write(a, &[0x55; 0x20]);
        t.write(b, &[0x66; 0x20]);
        t.check(
            f,
            Call {
                eax: 0xeeee,
                ecx: 1,
                edx: 2,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(l3_common::list_unlink_current(g, img, fr.at, fr.eax)),
        );
    }
}

#[test]
fn facing_angle() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x65f0);
    let flag = t.img.word(f + 0x16);
    let chr = t.img.word(f + 0x24);
    let id = t.img.word(f + 0x50);
    // Ch_GetRot answers the character word as an f32, plus a bit more
    // than f32 holds (the store rounds)
    t.stub_float(333, |_, a| f64::from(f32::from_bits(a[0])) + 1e-9);
    // MsGetChrDir writes the id word (an f32) to the local, unless 0
    t.stub(601, |m, a| {
        if a[0] != 0 {
            m.w32(a[1], a[0]);
        }
        0x601
    });
    let out = SCRATCH + 0x500;
    for (fl, v) in [
        (1u8, 0.0f32),
        (1, 1.0),
        (3, -2.5),
        (1, 3.0e9),
        (1, f32::NAN),
        (0, 0.75),
        (0, -1.0),
        (2, 1.0e10),
        (0, 0.0),
    ] {
        t.set_u8(flag, fl);
        t.set_u32(chr, v.to_bits());
        t.set_u32(id, v.to_bits());
        t.write(out, &[0xaa; 12]);
        t.check(
            f,
            Call {
                eax: 0,
                ecx: 0x3f00_0000,
                edx: out,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0159::facing_angle(g, img, fr.at, fr.esp, fr.ecx, fr.edx)),
        );
    }
}

#[test]
fn chr_show() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    record(&mut t, &[(257, 2), (296, 2)]);
    let f = t.at(0x6680);
    let flag = t.img.word(f + 0x02);
    t.set_u32(t.img.word(f + 0x18), 0x0c0c);
    t.set_u32(t.img.word(f + 0x2c), 0x0d0d);
    for fl in [0u8, 1, 2, 0xff] {
        t.set_u8(flag, fl);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(l3_common::chr_show(g, img, fr.at))
        });
    }
}
