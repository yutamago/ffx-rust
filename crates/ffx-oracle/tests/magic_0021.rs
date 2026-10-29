//! `magic_0021.dll` against its Rust port: list 1's OPU host variants
//! (`ffx_magic::l1_opu`: the start-up that tail-jumps into the set-up,
//! and `init_data` returning the data of a part already recorded), which
//! 4 and 20 overlays carry. Needs `FFX_GAME_DIR`.

mod magic;

use magic::{Call, Eax, SCRATCH, Twin};

const N: u16 = 21;

fn twin() -> Option<Twin> {
    let mut t = Twin::load(N)?;
    t.init();
    Some(t)
}

/// Every part-callback stub: records the words it is given in its own
/// scratch row and answers something the port must pass on.
fn part_stubs(t: &mut Twin) {
    for (k, (slot, n_args)) in (0u32..).zip([(711u32, 3usize), (719, 3), (941, 1)]) {
        let row = SCRATCH + 0x2_0000 + 0x100 * k;
        t.stub(slot, move |m, a| {
            let n = m.r32(row);
            for (i, w) in (0u32..).zip(&a[..n_args]) {
                m.w32(row + 4 + 0x18 * (n % 8) + 4 * i, *w);
            }
            m.w32(row, n + 1);
            match slot {
                941 => 0x0941_0000 + a[0],
                // the effect's entries: the part's data for i == 1
                719 => {
                    if a[2] == 1 {
                        SCRATCH + 0x9000
                    } else {
                        0x0abc_0000 + a[2]
                    }
                }
                _ => 0x7100 + slot,
            }
        });
    }
}

const PART: u32 = SCRATCH + 0xa000;

#[test]
fn init_data_returns_the_data_of_a_recorded_part() {
    let Some(mut t) = twin() else { return };
    part_stubs(&mut t);
    let f = t.at(0x1740);
    let slots = t.u32(t.img.word(f + 0x5d));
    let prev = t.img.word(f + 0xaf);
    let lock = t.u32(t.img.word(f + 0xce));
    t.set_u32(lock, 0x0bad);
    let effect = SCRATCH + 0xc000;
    for i in 0..4 {
        t.set_u32(SCRATCH + 0x9000 + 0x14 + 4 * i, 0x0d00_0000 + i);
    }
    // (entries, prev[1]): found and new, found and recorded, not found,
    // no entries
    for (count, prev1) in [(3u16, 0u32), (3, 5), (1, 0), (0, 0), (2, 0x0100_0000)] {
        t.set_u32(PART + 0xd0, SCRATCH + 0x9000);
        t.set_u32(slots + 4, effect);
        t.set_u16(effect + 0x50, count);
        t.set_u32(prev + 4, prev1);
        t.check_entry(f, Call::default(), &[PART], Eax::Checked);
    }
    // no effect
    t.set_u32(slots + 4, 0);
    t.check_entry(f, Call::default(), &[PART], Eax::Checked);
}

#[test]
fn start_up_tail_jumps_into_the_set_up() {
    let Some(mut t) = twin() else { return };
    t.stub(803, |_, _| SCRATCH + 0x1_0000);
    t.stub(718, |m, a| {
        m.w32(SCRATCH + 0x3_1000, a[0]);
        0x7180
    });
    t.stub(719, |m, a| {
        for (i, w) in (0u32..).zip(&a[..3]) {
            m.w32(SCRATCH + 0x3_1010 + 4 * i, *w);
        }
        a[0] + 0x10
    });
    // slot 715 sees every word the original leaves on the stack (11)
    t.stub(715, |m, a| {
        for (i, w) in (0u32..).zip(&a[..11]) {
            m.w32(SCRATCH + 0x3_0000 + 4 * i, *w);
        }
        SCRATCH
    });
    t.stub(720, |m, a| {
        m.w32(SCRATCH + 0x3_1020, a[0]);
        m.w32(SCRATCH + 0x3_1024, a[1]);
        0
    });
    t.stub(717, |m, a| {
        m.w32(SCRATCH + 0x3_1030, a[0]);
        0
    });
    let f = t.at(0x1690);
    let ctrl = t.img.word(f + 0x13);
    let flags = t.img.word(f + 0x22);
    let slots = t.img.word(f + 0x28);
    // the set-up's tables hold old values it must clear
    let setup = t.at(0x1000);
    for (op, len) in [(0x71, 0x100), (0x76, 0x2000), (0xa8, 0x100), (0xad, 0x30)] {
        let at = t.img.word(setup + op);
        t.write(at, &vec![0xa5; len]);
    }
    for (flag, word, data) in [
        (0u8, 0u16, 0u32),
        (1, 0x10, 0x00da_7a00),
        (1, 0x0f, 0),
        (0, 0x10, 0x00d0_0000),
        (1, 0x1234, 0x00d0_0000),
    ] {
        let c = t.u32(ctrl);
        t.set_u16(c + 0x40, word);
        t.set_u32(c + 0xa8, data);
        let fl = t.u32(flags);
        t.set_u8(fl + 0x10, flag);
        let s = t.u32(slots);
        t.set_u32(s + 4, 0);
        t.check_entry(
            f,
            Call {
                eax: 0,
                ecx: 0x77,
                edx: 0,
                ..Call::default()
            },
            &[0x00ab_cdef],
            Eax::Checked,
        );
    }
}

#[test]
fn the_variants_bind_in_every_carrier() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir).join("magicFiles/FFX");
    let bind = |n: u16, rva: u32| {
        let name = format!("magic_{n:04}.dll");
        let file = std::fs::read(dir.join(&name)).unwrap();
        let img = ffx_magic::Image::new(&name, &file, 0x1000_0000).unwrap();
        ffx_magic::bind(&img, 0x1000_0000 + rva).map(|i| ffx_magic::PORTS[i].0)
    };
    for n in [21, 22, 23, 326] {
        assert_eq!(
            bind(n, 0x1690),
            Some("l1 opu start-up (set-up tail)"),
            "{n}"
        );
        assert_eq!(
            bind(n, 0x1740),
            Some("l1 opu part init data (found: data)"),
            "{n}"
        );
    }
    for n in [
        24, 25, 26, 27, 28, 29, 30, 31, 34, 35, 36, 37, 38, 39, 69, 73,
    ] {
        assert_eq!(
            bind(n, 0x1730),
            Some("l1 opu part init data (found: data)"),
            "{n}"
        );
    }
}

#[test]
fn dll_main_writes_only_crt_state() {
    let Some(mut t) = Twin::load_unstarted(N) else {
        return;
    };
    t.dll_main_check(
        &[
            0x1100, 0x1110, 0x1190, 0x1220, 0x1280, 0x13b0, 0x1470, 0x14e0, 0x1500, 0x1560, 0x15d0,
            0x1630, 0x1690, 0x1740,
        ],
        &[(0x1000, 206)],
    );
}
