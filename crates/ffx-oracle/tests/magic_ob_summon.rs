//! The summon overlays' repeated entries (`ffx_magic::ob_summon`: the
//! task steps, the afterimage and its fade) against every DLL's own copy
//! (each binds by its own signature; the state block's field offsets
//! are read from the code as the port reads them). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation)]

mod magic;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{fill, rec, rec_all};

const STATE: u32 = SCRATCH + 0x8000;
const CFG: u32 = SCRATCH + 0x700;

/// Each copy: (DLL, address), every one bound to the port by name.
fn copies(name: &str) -> Vec<(u16, u32)> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return Vec::new();
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let lists: &[(&str, &[(u16, u32)])] = &[
        (
            "step",
            &[(394, 0x4890), (395, 0x41b0), (396, 0x4b70), (397, 0x42b0)],
        ),
        (
            "step_plain",
            &[
                (596, 0x69d0),
                (430, 0x1890),
                (431, 0x1720),
                (432, 0x1350),
                (434, 0x5820),
                (447, 0x4af0),
                (511, 0x15d0),
                (586, 0x2130),
                (587, 0x1590),
                (588, 0x17e0),
                (589, 0x15a0),
                (598, 0x6cb0),
                (599, 0x5fd0),
                (600, 0x6830),
                (602, 0x6cc0),
                (610, 0xa7c0),
                (638, 0x31a0),
                (673, 0x1370),
                (689, 0x31a0),
            ],
        ),
        (
            "afterimage",
            &[
                (394, 0x4b10),
                (395, 0x4340),
                (396, 0x4d20),
                (397, 0x44f0),
                (448, 0x4e80),
            ],
        ),
        (
            "afterimage_fade",
            &[
                (394, 0x4b50),
                (395, 0x4920),
                (396, 0x4d60),
                (397, 0x4530),
                (447, 0x54b0),
                (448, 0x74b0),
            ],
        ),
    ];
    let list = lists.iter().find(|l| l.0 == name).expect("a family").1;
    for &(n, rva) in list {
        let path = dir.join(format!("magic_{n:04}.dll"));
        let file = std::fs::read(&path).expect("the DLL");
        let pe = ffx_magic::pe::Pe::parse(&file).expect("a PE");
        let img = ffx_magic::Image::new(&format!("magic_{n:04}.dll"), &file, pe.preferred_base)
            .expect("an image");
        let a = img.at(rva);
        let i = ffx_magic::bind(&img, a).unwrap_or_else(|| panic!("{n}: nothing binds at {a:#x}"));
        let label = match name {
            "step" => " task step",
            "step_plain" => " task step (no pause)",
            "afterimage" => " afterimage",
            _ => " afterimage fade",
        };
        assert!(
            ffx_magic::PORTS[i].0.ends_with(label),
            "{n}: {a:#x} binds to {}",
            ffx_magic::PORTS[i].0
        );
    }
    list.to_vec()
}

/// A step's object list: empty, or one paused object (it ran).
fn steps(t: &mut Twin, f: u32, run_at: u32, done_disp_at: u32, pause: bool) {
    let ro = t.img.rel32_target(f + run_at);
    let list = t.img.word(ro + 0x7c);
    let d = t.img.word(f + done_disp_at);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(node + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let cases: &[(u32, bool, u32)] = if pause {
        &[
            (1, false, 1),
            (0, false, 1),
            (0, false, 0),
            (0, false, 2),
            (0, true, 1),
        ]
    } else {
        &[(0, false, 1), (0, false, 0), (0, false, 2), (0, true, 1)]
    };
    for &(p, busy, done) in cases {
        t.set_u32(CFG, p);
        t.set_u32(list + 8, if busy { node } else { 0 });
        t.set_u32(STATE + d, done);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

fn stubs(t: &mut Twin) {
    rec(t, 561, 0, |m, _| m.r32(CFG));
    rec_all(t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
    rec(t, 623, 7, |_, a| a[0] ^ 0x623);
    rec(t, 624, 1, |_, a| a[0] ^ 0x624);
}

#[test]
fn steps_in_every_copy() {
    for (n, rva) in copies("step") {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x0e), STATE);
        t.set_u32(t.img.word(f + 0x31), 0x0bfc + u32::from(n));
        steps(&mut t, f, 0x19, 0x24, true);
    }
    for (n, rva) in copies("step_plain") {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x03), STATE);
        t.set_u32(t.img.word(f + 0x20), 0x0bfc + u32::from(n));
        steps(&mut t, f, 0x08, 0x13, false);
    }
}

#[test]
fn afterimages_in_every_copy() {
    for (n, rva) in copies("afterimage") {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x0e), STATE);
        t.set_u32(t.img.word(f + 0x20), SCRATCH + 0x800 + u32::from(n));
        let d = t.img.word(f + 0x2b);
        fill(&mut t, STATE + d - 8, 4, u32::from(n));
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    for (n, rva) in copies("afterimage_fade") {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        stubs(&mut t);
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x08), STATE);
        let d = t.img.word(f + 0x14);
        fill(&mut t, STATE + d - 8, 4, u32::from(n));
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
