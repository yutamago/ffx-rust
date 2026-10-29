//! `magic_0074.dll` against its Rust port: `ffx_magic::m0074::pick` and
//! the state reset with `edx` (`ffx_magic::m0032`). Needs
//! `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, m0032, m0074};
use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 74;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

#[test]
fn pick_reads_the_default_or_the_table() {
    let Some(mut t) = twin() else { return };
    let f = t.at(0x1660);
    t.set_u32(t.img.word(f + 0x06), 0x0def_a017);
    t.set_u32(t.img.word(f + 0x0c), SCRATCH);
    for i in 0..0x10u8 {
        t.set_u8(SCRATCH + u32::from(i), 0xf0 | i);
    }
    for index in [u32::MAX, 0, 1, 7, 15] {
        t.check(
            f,
            Call {
                eax: 0x5555_5555,
                ecx: index,
                edx: 0,
                ..Call::default()
            },
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0074::pick(g, img, fr.at, fr.ecx)),
        );
    }
}

#[test]
fn reset_state_with_edx() {
    let Some(mut t) = twin() else { return };
    let this = SCRATCH + 0x1000;
    t.write(this + 0xb0, &[0x5a; 0x90]);
    t.check(
        t.at(0x1b60),
        Call {
            eax: 0,
            ecx: this,
            edx: 0x00ed_0123,
            ..Call::default()
        },
        &[0x1111, 0x1000_2270],
        // eax is the 0x8000 the u16 store leaves (no caller reads it)
        Eax::Junk,
        |g, _, fr| {
            let arg = fr.arg(g, 1);
            m0032::reset_state(g, fr.ecx, arg, fr.edx, 0x12);
            Ret::default()
        },
    );
}

#[test]
fn the_helpers_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: [(&str, &[(u16, u32)]); 2] = [
        (
            "m0074 pick",
            &[
                (74, 0x1660),
                (200, 0x19b0),
                (247, 0x1880),
                (385, 0x11d0),
                (563, 0x68e0),
                (612, 0x1f60),
                (659, 0x1880),
            ],
        ),
        (
            "m0032 state reset (edx, 0x12)",
            &[
                (74, 0x1b60),
                (110, 0x1fb0),
                (112, 0x1fb0),
                (494, 0x1a20),
                (681, 0x1c20),
            ],
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
