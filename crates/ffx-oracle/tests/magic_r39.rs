//! Round 39's ports (`ffx_magic::r39`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

mod magic;
#[path = "magic/ownb.rs"]
mod ownb;
#[path = "magic/summon_stubs.rs"]
mod summon_stubs;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;
use summon_stubs::all_stubs;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// The control words of [`scene_task_0519`]'s stubs: slot 261's key (the
/// key that answers, 8 for none), the busy mode (0 never, 1 from the
/// start, 2 set by slot 222), and slot 225's and 227's answers.
const KEY: u32 = SCRATCH + 0x7f00;
const MODE: u32 = SCRATCH + 0x7f04;
const A225: u32 = SCRATCH + 0x7f08;
const A227: u32 = SCRATCH + 0x7f0c;
const BUSY: u32 = SCRATCH + 0x7f10;
const LOCK: u32 = SCRATCH + 0x7f14;
const HOLDER: u32 = SCRATCH + 0x7f18;
const TABLE: u32 = SCRATCH + 0x7e00;

/// One case of [`scene_task_0519`]: the globals by operand (on top of the
/// defaults), the key slot 261 answers, the busy mode, slot 225's and
/// 227's answers.
struct Case0519 {
    set: Vec<(u32, u32)>,
    key: u32,
    mode: u32,
    a225: u32,
    a227: u32,
}

fn c519(set: &[(u32, u32)]) -> Case0519 {
    Case0519 {
        set: set.to_vec(),
        key: 8,
        mode: 0,
        a225: 0,
        a227: 1,
    }
}

fn keyed(key: u32, set: &[(u32, u32)]) -> Case0519 {
    Case0519 { key, ..c519(set) }
}

// the globals by the operand that names them
const G90: u32 = 0x08;
const G94: u32 = 0x35;
const G98: u32 = 0x60;
const G70: u32 = 0x8e;
const C1: u32 = 0x117;
const C2: u32 = 0x146;
const C5: u32 = 0x174;
const C4: u32 = 0x1c1;
const C6: u32 = 0x1c7;
const C7: u32 = 0x225;
const C3: u32 = 0x251;
const C8: u32 = 0x257;
const REC: u32 = 0x129c;

/// `magic_0519` 0x10002d60 (`r39::scene_task_0519`): the start-up checks
/// (slots 225 and 227 both ways, the one-time slots), every key with and
/// without the busy word, `G_C5` reaching 10, every part block, P8 running
/// or not, every `G_C8` value of the halfword, every block of the chain,
/// the busy word set late, the camera record and none.
#[test]
fn scene_task_0519() {
    let Some(mut t) = twin(519) else { return };
    all_stubs(&mut t);
    let f = t.at(0x2d60);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 225, 1, |m, _| m.r32(A225));
    rec(&mut t, 227, 1, |m, _| m.r32(A227));
    rec(&mut t, 261, 1, |m, a| {
        u32::from(a[0] == m.r32(KEY)) * (0x100 | a[0])
    });
    rec(&mut t, 941, 1, |_, a| TABLE + (a[0] & 0xf0));
    // the list slot: three entries
    rec(&mut t, 335, 1, |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, 0x0335_0000 + i);
        }
        3
    });
    rec(&mut t, 336, 1, |_, a| a[0] ^ 0x0336_0000);
    t.stub_st0(333, |_, a| 1.25 + f64::from(a[1] & 3));
    // the game's pointers: busy word, lock word, holder
    for (op, at) in [(0x108u32, BUSY), (0x97, LOCK), (0x2a8, HOLDER)] {
        let g = w(&t, op);
        if t.u32(g) == 0 {
            t.set_u32(g, at);
        }
    }
    let busy = t.u32(w(&t, 0x108));
    // slot 222 sets the busy word in mode 2
    rec(&mut t, 222, 4, move |m, a| {
        if m.r32(MODE) == 2 {
            m.w32(busy, 1);
        }
        a[1] ^ 0x222
    });
    let record = w(&t, 0x90c);
    let mut cases = vec![
        // the start-up checks
        Case0519 {
            a225: 5,
            ..c519(&[(G90, 0)])
        },
        c519(&[(G90, 0), (G94, 0), (G98, 0), (G70, 0)]),
        Case0519 {
            a227: 0,
            ..c519(&[(G98, 0)])
        },
        Case0519 {
            a227: 0xffff_ffff,
            ..c519(&[(G98, 0)])
        },
        c519(&[(G70, 0), (REC, record)]),
        c519(&[]),
        c519(&[(REC, record)]),
        // busy from the start, with and without a key
        Case0519 {
            mode: 1,
            ..c519(&[(REC, record)])
        },
        Case0519 {
            mode: 1,
            ..keyed(3, &[(REC, record)])
        },
        Case0519 {
            mode: 1,
            ..c519(&[])
        },
        // busy set late (slot 222)
        Case0519 {
            mode: 2,
            ..c519(&[(C7, 0), (REC, record)])
        },
        Case0519 {
            mode: 2,
            ..c519(&[(C7, 0)])
        },
        // G_C5 to 10
        keyed(5, &[(C5, 9)]),
        keyed(5, &[(C5, 9), (C3, 0x87)]),
        // the parts
        c519(&[(C3, 0x64)]),
        c519(&[(C3, 0x87)]),
        c519(&[(C3, 0x8a)]),
        c519(&[(C3, 0x122)]),
        c519(&[(C8, 0x290)]),
        c519(&[(C3, 0x64), (C8, 0x290)]),
        c519(&[(C8, 0x2e1)]),
        c519(&[(C8, 0x2e1), (C3, 0x8a)]),
        // P8 not running (its neighbour halfword set)
        c519(&[(C8, 0x2e1), (0x705, 0x5a5a_0000)]),
        // the halfword by G_C8
        c519(&[(C8, 0x106)]),
        c519(&[(C8, 0x14d)]),
        c519(&[(C8, 0x20f)]),
        c519(&[(C8, 0x122)]),
        c519(&[(C8, 0x1a4)]),
        c519(&[(C8, 0x23f)]),
        // the chain
        c519(&[(C7, 0)]),
        c519(&[(C7, 0x1e)]),
        c519(&[(C6, 0)]),
        c519(&[(C6, 0), (REC, record)]),
        c519(&[(C4, 0), (REC, record)]),
        c519(&[(C5, 0)]),
        c519(&[(C5, 0xc8)]),
        c519(&[(C2, 0)]),
        c519(&[(C2, 0x64)]),
        c519(&[(C1, 0)]),
        c519(&[(C1, 0x12c)]),
        c519(&[(C1, 0x12d)]),
    ];
    // every key (the counts land on the blocks: key 1 and 0 bump G_C3)
    for key in 0..8u32 {
        cases.push(keyed(key, &[(REC, record)]));
    }
    cases.push(keyed(0, &[(C3, 0x63)]));
    cases.push(keyed(1, &[(C3, 0x86), (C8, 0x2e0)]));
    cases.push(keyed(2, &[(C7, 0x1d)]));
    cases.push(keyed(3, &[(C6, 0xffff_ffff)]));
    cases.push(keyed(4, &[(C4, 0xffff_ffff), (C6, 0xffff_ffff)]));
    cases.push(keyed(6, &[(C2, 0x63)]));
    cases.push(keyed(7, &[(C1, 0x12b)]));
    for (k, cs) in (0u32..).zip(cases) {
        // the defaults: start-up done, every count off its values
        for (i, op) in (0u32..).zip([G90, G94, G98, G70]) {
            t.set_u32(w(&t, op), 0x10 + i + k);
        }
        for (i, op) in (0u32..).zip([C1, C2, C5, C4, C6, C7, C3, C8, 0xa63, 0x8f6]) {
            t.set_u32(w(&t, op), 0x55 + 3 * i + (k << 12));
        }
        t.set_u32(w(&t, REC), 0);
        t.set_u32(w(&t, 0x16), 0x7c00 + k);
        // P8 running, P0's halfword, the camera words junk first
        t.set_u16(w(&t, 0x705), 0x1234 + k as u16);
        t.set_u16(w(&t, 0x1282), 0x5a5a);
        for op in [0x912u32, 0x917, 0x91c] {
            t.set_u32(w(&t, op), 0x6b6b_0000 + op);
        }
        for (op, v) in &cs.set {
            t.set_u32(w(&t, *op), *v);
        }
        t.set_u32(KEY, cs.key);
        t.set_u32(MODE, cs.mode);
        t.set_u32(A225, cs.a225);
        t.set_u32(A227, cs.a227);
        t.set_u32(busy, u32::from(cs.mode == 1));
        t.set_u32(t.u32(w(&t, 0x97)), 0x0bad_0000 + k);
        for i in 0..0x40 {
            t.set_u32(TABLE + 4 * i, 0x0941_0000 + i + (k << 8));
        }
        ownb::dirty_stack(&mut t, 0x2d60 + k, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
