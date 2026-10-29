//! `magic_0052.dll` against its Rust port (`ffx_magic::m0052`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0052};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 52;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// A stub that logs `n` words of every call in its own row and answers
/// `ret`.
fn logger(t: &mut Twin, slot: u32, row: u32, n: usize, ret: u32) {
    let at = SCRATCH + 0x1_0000 + 0x200 * row;
    t.stub(slot, move |m, a| {
        let k = m.r32(at);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(at + 4 + 0x40 * (k % 7) + 4 * i, *w);
        }
        m.w32(at, k + 1);
        ret
    });
}

/// Model handles in the table and the index byte.
fn models(t: &mut Twin, f: u32, index_op: u32, table_op: u32, index: u8) {
    let ip = t.img.word(f + index_op);
    t.set_u32(ip, SCRATCH + 0x100);
    t.set_u8(SCRATCH + 0x100, index);
    let table = t.img.word(f + table_op);
    for i in 0..0x30u32 {
        t.set_u32(table + 4 * i, 0x00ab_0000 + i);
    }
}

#[test]
fn reset_model_calls_both_slots() {
    let Some(mut t) = twin() else { return };
    logger(&mut t, 357, 0, 3, 0x357);
    logger(&mut t, 356, 1, 6, 0x356);
    let f = t.at(0x1580);
    for index in [0u8, 5, 0x2f] {
        models(&mut t, f, 0x01, 0x0e, index);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0052::reset_model(g, img, fr.at))
        });
    }
}

#[test]
fn place_picks_the_slot() {
    let Some(mut t) = twin() else { return };
    logger(&mut t, 255, 0, 2, 0x255);
    logger(&mut t, 33, 1, 4, 0x33);
    logger(&mut t, 32, 2, 3, 0x32);
    let f = t.at(0x1cd0);
    let mode = t.img.word(f + 0x02);
    let pos = SCRATCH + 0x200;
    for (i, v) in (0u32..).zip([1.5f32, -2.0, f32::NAN]) {
        t.set_u32(pos + 4 * i, v.to_bits());
    }
    // (mode, index, slot 11's answer)
    for (m, index, is_model) in [
        (0u32, 3u8, 0u32),
        (0, 0x1f, 1),
        (0, 0x20, 0),
        (0, 0x21, 1),
        (1, 3, 0),
        (2, 3, 7),
    ] {
        models(&mut t, f, 0x09, 0x35, index);
        t.set_u32(mode, m);
        logger(&mut t, 11, 3, 1, is_model);
        t.check(
            f,
            Call {
                eax: 0,
                ecx: 0,
                edx: pos,
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0052::place(g, img, fr.at, fr.edx)),
        );
    }
}

#[test]
fn scaled_camera_scales_the_rows() {
    let Some(mut t) = twin() else { return };
    // slot 544 fills row i through the pointer
    t.stub(544, |m, a| {
        for j in 0..4 {
            let v = f32::from(u8::try_from(a[0] * 4 + j).unwrap()) * 0.75 - 3.0;
            m.w32(a[1] + 4 * j, v.to_bits());
        }
        // the words the earlier calls left on the stack
        for k in 0..2 * a[0] {
            m.w32(SCRATCH + 0x300 + 0x20 * a[0] + 4 * k, a[2 + k as usize]);
        }
        0x544
    });
    // slot 553 reads the four rows it is given (the scaled locals)
    t.stub(553, |m, a| {
        for (i, w) in (0u32..).zip(&a[..13]) {
            m.w32(SCRATCH + 0x400 + 4 * i, *w);
        }
        for r in 0..4u32 {
            for j in 0..4u32 {
                let v = m.r32(a[1 + r as usize] + 4 * j);
                m.w32(SCRATCH + 0x500 + 0x10 * r + 4 * j, v);
            }
        }
        0x553
    });
    let f = t.at(0x1f00);
    t.check(
        f,
        Call {
            eax: 0,
            ecx: 0x0123_4567,
            edx: 0,
        },
        &[],
        Eax::Checked,
        |g, img, fr| Ret::eax(m0052::scaled_camera(g, img, fr.at, fr.ecx, fr.esp)),
    );
}

#[test]
fn the_helpers_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: [(&str, &[(u16, u32)]); 3] = [
        ("m0052 model reset", &[(52, 0x1580), (64, 0x2ac0)]),
        (
            "m0052 place",
            &[
                (52, 0x1cd0),
                (55, 0x20f0),
                (64, 0x10f0),
                (67, 0x10f0),
                (435, 0x1c00),
            ],
        ),
        (
            "m0052 scaled camera",
            &[(52, 0x1f00), (64, 0x1320), (266, 0x2a80), (353, 0x32f0)],
        ),
    ];
    for (port, carriers) in lists {
        for &(n, rva) in carriers {
            let name = format!("magic_{n:04}.dll");
            let file = std::fs::read(dir.join(&name)).unwrap();
            let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
            assert_eq!(
                ffx_magic::covered(&img, 0x1000_0000 + rva),
                Some(port),
                "{name}"
            );
        }
    }
}
