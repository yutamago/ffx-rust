//! Round 23's ports (`ffx_magic::r23`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::type_complexity
)]

mod magic;
mod magic_hi;
mod magic_owna;
mod magic_r11;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, r23};
use magic::{Call, Eax, SCRATCH, StubMem, Twin};
use magic_owna::twin;

/// Junk in the stack below the entry (the frames' locals start as it).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// Junk words at the globals the operands `ops` of `f` name.
fn junk_ops(t: &mut Twin, f: u32, ops: &[u32]) {
    for &o in ops {
        let at = t.img.word(f + o);
        t.set_u32(at, 0x5a5a_5a5a ^ o);
    }
}

/// `n` bytes of junk at `at` (varied by `k`).
fn junk(t: &mut Twin, at: u32, n: u32, k: u32) {
    let b: Vec<u8> = (0..n)
        .map(|i| (i.wrapping_mul(0x3d) ^ (0x71 + k)).to_le_bytes()[0])
        .collect();
    t.write(at, &b);
}

/// Folds `v` into word `i` of `slot`'s dump row (the outputs a stub writes
/// to the stack, which the comparison skips).
fn fold(m: &mut dyn StubMem, slot: u32, i: u32, v: u32) {
    let at = SCRATCH + 0x2_0000 + 0x40 * (slot % 0x400) + 4 * i;
    let x = m.r32(at).rotate_left(5) ^ v;
    m.w32(at, x);
}

fn rf(m: &dyn StubMem, at: u32) -> f32 {
    f32::from_bits(m.r32(at))
}

fn wf(m: &mut dyn StubMem, at: u32, v: f32) {
    m.w32(at, v.to_bits());
}

fn kf(t: &Twin, f: u32, o: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(f + o)))
}

const CFG: u32 = SCRATCH + 0x700;
const CHRS: u32 = SCRATCH + 0x800;

/// The placement's slots (as `magic_0596.rs`'s, with the words the
/// port passes): positions, the rotation (`CFG`), the scales, the
/// matrices, slot 8.
fn place_stubs_0598(t: &mut Twin) {
    model_stubs(t, 2);
}

/// [`place_stubs_0598`] with `base` words under the model matrix's first
/// call (`scale_model`'s `base`).
fn model_stubs(t: &mut Twin, base: usize) {
    use ownb::{rec, rec_all};
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = 1.5 * f32::from(i as u8) + f32::from((a[0] & 0xff) as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(t, 333, 5, |_, _| 0);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    rec_all(
        t,
        &[(210, 2), (211, 2), (238, 5), (296, 2), (257, 2), (8, 2)],
    );
    rec(t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, 0x4100_0000 + 0x1_0000 * i + a[0]);
        }
        0
    });
    rec(t, 255, 4, |m, a| m.r32(a[1] + 0xc));
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0
    });
    rec(t, 277, 2 + base, |_, _| 0);
    t.stub_st0(277, |_, a| 40.0 + f64::from(a[0] & 0xf));
    for (s, n, v) in [(15u32, 3usize, 1.5f64), (16, 4, -0.75), (17, 5, 3.0)] {
        rec(t, s, n + base, |_, _| 0);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0x3));
    }
    for s in [330u32, 328, 329] {
        rec(t, s, 3, move |m, a| {
            for i in 0..16 {
                let v = m.r32(a[1] + 4 * i);
                m.w32(a[0] + 4 * i, v ^ (a[2] & 0x00ff_0000) ^ (s << 8));
            }
            0
        });
    }
    rec(t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i).rotate_left(3);
            m.w32(a[0] + 4 * i, v);
            // (kept where the comparison sees it: the vector is a local)
            fold(m, 229, i, v);
        }
        0
    });
    rec(t, 248, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(7);
            m.w32(a[0] + 4 * i, v);
        }
        0x248 ^ a[0]
    });
}

/// The placement's globals in `magic_0598` (operands of 0x10006cf0): the
/// selection, the characters, the state block, `*G_OFF`, the summon's
/// battle index and the caster's rotation.
fn place_globals_0598(t: &mut Twin, state: u32, off: u32, index: u32, rot: f32) {
    let f = t.at(0x6cf0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let sel = SCRATCH + 0x600;
    for o in [0x14u32, 0x76, 0x145, 0x1d3, 0x213] {
        t.set_u32(w(t, o), sel);
    }
    t.set_u8(sel, 3);
    for o in [0x33u32, 0x87, 0x155, 0x1dd, 0x21d] {
        for i in 0..8 {
            t.set_u32(w(t, o) + 4 * i, CHRS + 0x10 * i);
        }
    }
    t.set_u32(w(t, 0x20), state);
    for o in [0x179u32, 0x399, 0x534] {
        t.set_u32(w(t, o), off);
    }
    for o in [0x3a2u32, 0x523, 0x53c] {
        t.set_u32(w(t, o), index);
    }
    for o in [0x36fu32, 0x3fc, 0x433, 0x55e, 0x579] {
        t.set_u32(w(t, o), CHRS + 0x99);
    }
    t.set_u32(CFG, rot.to_bits());
    for o in [0x759u32, 0x763, 0x76d] {
        let a = w(t, o);
        t.set_u32(a, 0x5a5a_0000 + o);
    }
}

/// `magic_0598` 0x10006cf0 (`r23::place_pair_0598`): the angle wrapped
/// down, kept, wrapped up, on either limit, NaN; the summon's index in
/// the battle range or not, the battle off. (The null-matrix paths need
/// a state block that wraps into page 0, as in `magic_0596.rs`: the
/// game's callers point it at the DLL's own block.)
#[test]
fn place_pair_0598() {
    let Some(mut t) = twin(598) else { return };
    place_stubs_0598(&mut t);
    let f = t.at(0x6cf0);
    let state = SCRATCH + 0xa000;
    let rots = ownb::rotations(&t, f, 0xa2, 0xb4, 0xd0);
    let mut cases = vec![(0u32, 2u32), (0, 0x20), (1, 2), (0, 0xffff_ffff), (0, 0x1f)];
    while cases.len() < rots.len() {
        cases.push((0, 3));
    }
    for (k, ((off, index), rot)) in cases.into_iter().zip(rots).enumerate() {
        ownb::fill(&mut t, state + 0x840, 0x150, 5 + k as u32);
        ownb::fill(&mut t, state + 0xd10, 0x10, 9 + k as u32);
        place_globals_0598(&mut t, state, off, index, rot);
        ownb::dirty_stack(&mut t, rot.to_bits(), &[]);
        ownb::reset_counters(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r23::place_pair_0598(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x400, "the placements' calls");
}

/// The second start's set-up in `magic_0598` (0x100085c0's callees):
/// slots 719, 711, 710, 271, the turned effect's actor, the part's lock,
/// the records and the task pool; the task list empty or not.
fn start2_setup_0598(t: &mut Twin, busy: bool, k: u32) {
    let f = t.at(0x85c0);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let st = t.img.rel32_target(f + 0x56);
    t.set_u32(w(t, st, 0x03), CHRS + 0x200);
    let pi = t.img.rel32_target(f + 0x4c);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(t, pi, 0x04), lock);
    t.set_u32(lock, 0x77 + k);
    let take = t.img.rel32_target(f + 0xba);
    let ri = t.img.rel32_target(f + 0xf8);
    let state = w(t, f, 0x38);
    ownb::fill(t, state + 0x840, 0x150, 5 + k);
    ownb::fill(t, state + 0xd10, 0x40, 9 + k);
    let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
    for i in 0..3u32 {
        t.write(node(i), &[0x33; 0x40]);
        t.set_u32(node(i), node((i + 2) % 3));
        t.set_u32(node(i) + 4, node((i + 1) % 3));
    }
    t.set_u32(w(t, ri, 0x3e), node(0));
    t.set_u16(w(t, ri, 0x20), 3);
    t.set_u32(w(t, ri, 0xa6), 0);
    magic_hi::pool(t, take, SCRATCH + 0x4000, 4);
    let last = SCRATCH + 0x3400;
    t.write(last, &[0x34; 0x20]);
    t.set_u32(w(t, f, 0xc0), if busy { last } else { 0 });
    t.set_u32(w(t, f, 0xd4), last);
    // the script blocks' words the start clears or sets: old values
    for op in [0x61u32, 0x6b, 0x76, 0x85, 0x8f, 0x9a] {
        t.set_u32(w(t, f, op), 0x5a5a_5a5a ^ op);
    }
    // the part set-up's four words it clears
    for op in [0x58u32, 0x62, 0x6c, 0x76] {
        t.set_u32(w(t, pi, op), 0x5a5a_0000 ^ op);
    }
}

fn start2_stubs_0598(t: &mut Twin) {
    use ownb::rec;
    place_stubs_0598(t);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    // pppInitData runs with the game's lock set
    let lock = SCRATCH + 0x7f0;
    rec(t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(t, 710, 4, |_, a| a[2] ^ 0x710);
    rec(t, 271, 3, |_, a| 0x0271_0000 ^ a[0]);
}

/// `magic_0598` 0x100085c0 (`r23::start2_0598`): the task list empty and
/// not; the placement as in [`place_pair_0598`].
#[test]
fn start2_0598() {
    let Some(mut t) = twin(598) else { return };
    start2_stubs_0598(&mut t);
    let f = t.at(0x85c0);
    for (k, busy) in [false, true].into_iter().enumerate() {
        let state = t.img.word(f + 0x38);
        place_globals_0598(&mut t, state, 0, 2, 1.25 + k as f32);
        start2_setup_0598(&mut t, busy, k as u32);
        ownb::dirty_stack(&mut t, 0x85c0 + k as u32, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0598` 0x10006a60 (`r23::cue_start_0598`): the waits (slots
/// 225, 5, 28), the once flag, and the start.
#[test]
fn cue_start_0598() {
    use ownb::rec;
    let Some(mut t) = twin(598) else { return };
    start2_stubs_0598(&mut t);
    let f = t.at(0x6a60);
    let s2 = t.at(0x85c0);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 5, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 28, 2, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    // (225, 5, 28, once, 227)
    let cases: [(u32, u32, u32, u32, u32); 6] = [
        (1, 0, 0, 0, 1),
        (0, 1, 0, 0, 1),
        (0, 0, 2, 0, 1),
        (0, 0, 0, 0, 0),
        (0, 0, 0, 0, 1),
        (0, 0, 0, 1, 3),
    ];
    for (k, (w225, w5, w28, once, r227)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x14, w5);
        t.set_u32(SCRATCH + 0x18, w28);
        t.set_u32(t.img.word(f + 0x50), once);
        let state = t.img.word(s2 + 0x38);
        place_globals_0598(&mut t, state, 0, 2, -0.5 + k as f32);
        start2_setup_0598(&mut t, k % 2 == 1, k);
        ownb::dirty_stack(&mut t, 0x6a60 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x400, "the starts' calls");
}

/// Every slot answering `first word ^ (slot << 20)` and logging its first
/// word (into `SCRATCH + 0xa00..`), as `magic_o6_fam.rs`'s `stubs`.
fn all_stubs(t: &mut Twin) {
    for s in 0..1024u32 {
        t.stub(s, move |m, a| {
            let k = m.r32(SCRATCH + 0xa00);
            if let Some(w) = a.first() {
                m.w32(SCRATCH + 0xa10 + 4 * (k % 0x100), *w ^ s);
            }
            m.w32(SCRATCH + 0xa00, k + 1);
            a.first().copied().unwrap_or(0) ^ (s << 20)
        });
    }
}

/// The effect end's slots and callees in `magic_0598` (0x10007500): the
/// two-word slots logged, slot 219 answering a character or 0 (`hide`),
/// the party's list (slots 288 and 293), the parts (two records, `banks`
/// static banks) and the node `*G_N` at rest or none.
fn end_setup_0598(t: &mut Twin, k: u32, hide: bool, node: bool, banks: u32) {
    use ownb::rec;
    let f = t.at(0x7500);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    for s in [25u32, 24, 217, 255, 211, 213, 296, 257, 399, 8, 371] {
        rec(t, s, 2, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    rec(
        t,
        219,
        2,
        move |_, a| if hide { 0x0219_0000 ^ a[0] } else { 0 },
    );
    t.stub(288, |m, a| {
        m.w32(a[1], 2);
        SCRATCH + 0x4000
    });
    t.write(SCRATCH + 0x4000, &[3, 5]);
    t.stub(293, |_, a| a[0] & 1);
    // the parts: two records, the lock, `banks` banks of the static table
    let rp = t.img.rel32_target(f + 0x288);
    let recs = SCRATCH + 0x6000;
    t.set_u32(w(t, rp, 0x16), recs);
    t.set_u32(recs + 0x30, 0x00c0_0030 + k);
    t.set_u32(recs + 0xa4, 0x00c0_00a4 + k);
    t.set_u32(w(t, rp, 0x0e), 2);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(t, rp, 0x43), lock);
    t.set_u32(lock, 0x10 + k);
    t.set_u32(w(t, rp, 0x76), banks);
    let n = SCRATCH + 0x3400;
    if node {
        t.write(n, &[0x5a; 0x1c]);
        t.set_u32(n, n);
        t.set_u32(n + 4, n);
        t.set_u32(n + 8, n);
        t.set_u32(n + 0xc, 0);
    }
    t.set_u32(w(t, f, 0x270), if node { n } else { 0 });
    for o in [0x131u32, 0x161] {
        t.set_u32(w(t, f, o), 0x5a5a_0000 + o);
    }
}

/// The effect end's globals: the selection (caster index `c`), the
/// characters, `*G_OFF`, the summon and its battle index `i`, the two
/// u16 waits.
fn end_globals_0598(t: &mut Twin, off: u32, c: u8, i: u32, waits: (u16, u16)) {
    let f = t.at(0x7500);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let sel = SCRATCH + 0x600;
    for o in [0x11u32, 0x53, 0x15b, 0x23d] {
        t.set_u32(w(t, o), sel);
    }
    t.write(sel, &[c, 5]);
    for o in [0x1eu32, 0x67, 0x18b] {
        for j in 0..0x40 {
            t.set_u32(w(t, o) + 4 * j, CHRS + 0x10 * j);
        }
    }
    for o in [0x5cu32, 0xb7, 0x155, 0x1a1] {
        t.set_u32(w(t, o), off);
    }
    for o in [0xc0u32, 0x1a9] {
        t.set_u32(w(t, o), i);
    }
    for o in [0xaeu32, 0x1cb, 0x1e9, 0x1f8] {
        t.set_u32(w(t, o), CHRS + 0x99);
    }
    t.set_u32(w(t, 0x118), 0x00a0_0118);
    t.set_u32(w(t, 0x144), 0x00a0_0144);
    t.set_u16(w(t, 0x106), waits.0);
    t.set_u16(w(t, 0x12a), waits.1);
}

/// `magic_0598` 0x10007500 (`r23::effect_end_0598`): the battle on and
/// off, the caster's and the summon's indices below 32 and not, the
/// waits 0 and not, slot 219 answering a character or 0, the node at
/// rest or none, no banks and one.
#[test]
fn effect_end_0598() {
    let Some(mut t) = twin(598) else { return };
    all_stubs(&mut t);
    let f = t.at(0x7500);
    // (off, caster, summon index, waits, hide, node, banks)
    let cases: [(u32, u8, u32, (u16, u16), bool, bool, u32); 6] = [
        (0, 3, 2, (0, 0), true, true, 0),
        (0, 0x20, 0x20, (1, 0), false, false, 0),
        (1, 3, 2, (0, 1), true, false, 1),
        (0, 0x1f, 0xffff_ffff, (1, 1), false, true, 0),
        (0, 0x40, 0x1f, (0, 0), true, true, 0),
        (1, 0x20, 0x40, (0, 0), false, true, 0),
    ];
    for (k, (off, c, i, waits, hide, node, banks)) in (0u32..).zip(cases) {
        end_setup_0598(&mut t, k, hide, node, banks);
        end_globals_0598(&mut t, off, c, i, waits);
        ownb::dirty_stack(&mut t, 0x7500 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r23::effect_end_0598(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the end's calls");
}

/// `magic_0598` 0x10006920 (`r23::host_start_end_0598`): slot 549
/// answering 0 and not, the end run either way.
#[test]
fn host_start_end_0598() {
    let Some(mut t) = twin(598) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6920);
    for (k, r549) in [0u32, 1].into_iter().enumerate() {
        t.stub(549, move |_, _| r549);
        end_setup_0598(&mut t, k as u32, k == 1, k == 0, 0);
        end_globals_0598(&mut t, 0, 3, 2, (0, 0));
        t.set_u32(t.img.word(f + 0x89), 0x5a5a_5a5a);
        ownb::dirty_stack(&mut t, 0x6920 + k as u32, &[]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x40, "the end's calls");
}

/// The selection and characters the model matrices read in `magic_0598`
/// (`sel` operands, `table` operands of the function at `f`), the
/// selection bytes `c0`, `c1`.
fn chars_0598(t: &mut Twin, f: u32, sels: &[u32], tables: &[u32], c: [u8; 2]) {
    let sel = SCRATCH + 0x600;
    for &o in sels {
        t.set_u32(t.img.word(f + o), sel);
    }
    t.write(sel, &c);
    for &o in tables {
        for j in 0..0x40 {
            t.set_u32(t.img.word(f + o) + 4 * j, CHRS + 0x10 * j);
        }
    }
}

/// The fade task the turn ends with (`s3_misc::fade_task` at `fade`): a
/// free ring of three nodes, the task list empty or not.
fn fade_setup(t: &mut Twin, fade: u32, full: bool) {
    let take = t.img.rel32_target(fade + 0x75);
    magic_hi::pool(t, take, SCRATCH + 0x5000, 3);
    let (fh, ft) = (t.img.word(fade + 0x7b), t.img.word(fade + 0x93));
    magic_hi::list(t, fh, ft, SCRATCH + 0x5400, full);
}

/// `magic_0598` 0x100081b0 (`r23::turn_0598`): the yaw wrapped down,
/// kept, wrapped up and NaN; the task list empty and not.
#[test]
fn turn_0598() {
    let Some(mut t) = twin(598) else { return };
    model_stubs(&mut t, 0);
    let f = t.at(0x81b0);
    let fade = t.img.rel32_target(f + 0x3ae);
    let s = SCRATCH + 0xa000;
    t.set_u32(t.img.word(f + 0x14), s);
    let (step, hi, lo) = (kf(&t, f, 0x25), kf(&t, f, 0x2f), kf(&t, f, 0x43));
    let yaws = [
        hi - step + 0.5,
        0.25,
        lo - step - 0.5,
        f32::NAN,
        hi - step,
        lo - step,
    ];
    for (k, yaw) in (0u32..).zip(yaws) {
        ownb::fill(&mut t, s + 0x840, 0x1a0, 3 + k);
        ownb::fill(&mut t, s + 0xbb0, 0x10, 7 + k);
        t.set_u32(s + 0x914, yaw.to_bits());
        t.set_u32(s + 0xd18, (0.75 * k as f32 - 1.0).to_bits());
        chars_0598(
            &mut t,
            f,
            &[0x123, 0x15d, 0x291, 0x2cc],
            &[0x12d, 0x167, 0x29c, 0x2d7],
            [3, 5 + k as u8],
        );
        fade_setup(&mut t, fade, k % 2 == 1);
        ownb::dirty_stack(&mut t, 0x81b0 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the turn's calls");
}

/// The effect objects' set-up (`o6_fam_object::start_object_first`'s
/// `magic_0369` shape at `at`, as `magic_o6_fam.rs` sets it up): `free`
/// pool nodes, the object list's tail `tail`.
fn objects_setup_0598(t: &mut Twin, at: u32, free: u16, tail: u32) {
    t.set_u32(t.img.word(at + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(at + 0x5b), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    t.stub(941, |m, a| {
        m.w32(SCRATCH + 0x900, a[0]);
        SCRATCH + 0x400
    });
    t.stub(714, |m, a| {
        let k = m.r32(SCRATCH + 0x90c);
        m.w32(SCRATCH + 0x90c, k + 1);
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 0x14 * (k % 8) + 4 * i, *w);
        }
        0x714
    });
    let call = 0x8e;
    let pa = t.img.rel32_target(at + call + 1);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), free);
    t.set_u32(t.img.word(at + call + 0xc), tail);
    for op in [call + 5 + 0x29, call + 5 + 0x31 + 0x15] {
        let count_at = t.img.word(at + op);
        t.set_u32(count_at, 0x40 + op);
    }
}

/// `magic_0598` 0x100078c0 (`r23::objects_start_0598`): the yaws wrapped
/// down, kept, up; the object pool with nodes and without, the object
/// list empty or not; the task list empty and not; the second
/// character's battle index below 32 and not, the battle off.
#[test]
fn objects_start_0598() {
    let Some(mut t) = twin(598) else { return };
    model_stubs(&mut t, 2);
    let f = t.at(0x78c0);
    let so = t.img.rel32_target(f + 0x2d);
    let take = t.img.rel32_target(f + 0x4ad);
    let s = SCRATCH + 0xa000;
    t.set_u32(t.img.word(f + 0x1d), s);
    let (step, hi, lo) = (kf(&t, f, 0xdb), kf(&t, f, 0xe5), kf(&t, f, 0xf5));
    // (yaw, free object nodes, object list, task list, index, off)
    let cases: [(f32, u16, bool, bool, u8, u32); 5] = [
        (hi - step + 0.5, 6, false, false, 3, 0),
        (0.25, 6, true, true, 0x20, 0),
        (lo + step - 0.5, 1, true, false, 2, 1),
        (f32::NAN, 0, false, true, 0x1f, 0),
        (lo - step - 0.5, 6, true, true, 5, 0),
    ];
    for (k, (yaw, free, objs, tasks, idx, off)) in (0u32..).zip(cases) {
        ownb::fill(&mut t, s + 0x4c0, 0x400, 3 + k);
        ownb::fill(&mut t, s + 0x8c0, 0x100, 5 + k);
        ownb::fill(&mut t, s + 0x9a0, 0x380, 7 + k);
        t.set_u32(s + 0xd18, yaw.to_bits());
        objects_setup_0598(&mut t, so, free, if objs { SCRATCH + 0x700 } else { 0 });
        chars_0598(
            &mut t,
            f,
            &[0x221, 0x301, 0x33c, 0x42e],
            &[0x22c, 0x30c, 0x347, 0x43b],
            [5, idx],
        );
        t.set_u32(t.img.word(f + 0x24d), off);
        magic_hi::pool(&mut t, take, SCRATCH + 0x5000, 4);
        let last = SCRATCH + 0x5400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0x4b3), if tasks { last } else { 0 });
        t.set_u32(t.img.word(f + 0x4c6), last);
        ownb::dirty_stack(&mut t, 0x78c0 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the start's calls");
}

/// `magic_0514` 0x10003570 (`r23::thunk_visit_draw_0514`, with
/// 0x10003220 `r23::visit_draw_0514`): the walk stopping at once and
/// visiting two children and none; the count still running, at its end
/// with and without the draw (`+0x7f`), the leave flag set with a parent
/// and without, a negative count.
#[test]
fn visit_draw_0514() {
    use ownb::rec;
    let Some(mut t) = twin(514) else { return };
    all_stubs(&mut t);
    let f = t.at(0x3570);
    let v = t.img.rel32_target(f + 8);
    // the children's `+0x1c` methods: a logging stub on both sides
    let vt = SCRATCH + 0x880;
    t.set_u32(magic::TABLE + 4 * 1001, magic::TRAPS + 16 * 1001);
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * 1001);
    rec(&mut t, 1001, 1, |m, a| {
        // the cursor: its node and pass byte
        let x = m.r32(a[0]) ^ (m.r32(a[0] + 4) & 0xff00);
        fold(m, 1001, 0, x);
        0x1001_0000 ^ x
    });
    rec(&mut t, 355, 2, |_, a| 0x0355_0000 ^ a[0]);
    rec(&mut t, 354, 2, |_, a| 0x0354_0000 ^ a[0]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let x = m.r32(a[1] + 4 * i) ^ 0x0325_0000;
            m.w32(a[0] + 4 * i, x);
        }
        0x325
    });
    rec(&mut t, 231, 3, |m, a| {
        for i in 0..16 {
            let x = m.r32(a[2] + 4 * (i % 4)) ^ (i << 12) ^ a[1];
            m.w32(a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 567, 5, |m, a| {
        let r = a[1];
        for i in 0..20 {
            let x = m.r32(r + 4 * i);
            fold(m, 567, i % 16, x);
        }
        for (k, n) in [(0x20u32, 16u32), (0x24, 4), (0x28, 4)] {
            let p = m.r32(r + k);
            for i in 0..n {
                let x = m.r32(p + 4 * i);
                fold(m, 567, (k + i) % 16, x);
            }
        }
        0x0567_0000 ^ m.r32(r + 0x18)
    });
    let cursor = SCRATCH + 0x100;
    let o = SCRATCH + 0x2000;
    let (c1, c2, c3, c4) = (
        SCRATCH + 0x2400,
        SCRATCH + 0x2500,
        SCRATCH + 0x2600,
        SCRATCH + 0x2700,
    );
    let (blk, parent, sib) = (SCRATCH + 0x2800, SCRATCH + 0x2900, SCRATCH + 0x2a00);
    let gp = SCRATCH + 0x7e0;
    t.set_u32(t.img.word(v + 0x149), gp);
    // (equal pass, children, count, draw, flags, parent)
    let cases: [(bool, bool, u16, u8, u8, bool); 7] = [
        (true, true, 0, 1, 0x10, true),
        (false, true, 3, 1, 0x10, true),
        (false, false, 1, 0, 0, false),
        (false, true, 0, 1, 0x10, true),
        (false, true, 0x8000, 1, 0x10, false),
        (false, false, 0, 1, 0x0f, true),
        (false, true, 1, 0, 0x10, true),
    ];
    for (k, (equal, kids, count, draw, flags, has_parent)) in (0u32..).zip(cases) {
        junk(&mut t, o, 0x100, k);
        junk(&mut t, blk, 0x100, k + 1);
        for c in [c1, c2, c3, c4] {
            junk(&mut t, c, 0x40, k + 2);
            t.set_u32(c, vt);
        }
        t.set_u32(cursor, o);
        t.set_u8(cursor + 4, 0x77);
        t.set_u8(cursor + 5, 0x30 + k as u8);
        t.set_u8(o + 0x1a, if equal { 0x30 + k as u8 } else { 0x11 });
        if kids {
            t.set_u32(o + 0x10, c1);
            t.set_u32(c1 + 8, c2);
            t.set_u32(c2 + 8, o);
        } else {
            t.set_u32(o + 0x10, o);
        }
        t.set_u16(o + 0xaa, count);
        t.set_u8(o + 0x7f, draw);
        t.set_u8(o + 0xa4, flags);
        t.set_u32(o + 0x6c, c3);
        t.set_u32(o + 0x70, c4);
        t.set_u32(o + 0x68, blk);
        t.set_u32(o + 0x78, 0x00a0_0078 + k);
        t.set_u32(o + 0x7c, 0x00a0_007c + k);
        if has_parent {
            junk(&mut t, parent, 0x40, k + 3);
            junk(&mut t, sib, 0x40, k + 4);
            t.set_u32(o + 0x14, parent);
            t.set_u32(o + 4, parent);
            t.set_u32(o + 8, sib);
        } else {
            t.set_u32(o + 0x14, 0);
        }
        t.set_u32(gp, 0x0bad_0000 + k);
        t.set_u32(t.img.word(v + 0x2a0), 0x0cc0_0000 + k);
        t.set_u32(t.img.word(v + 0x2ce), 0x5a5a_5a5a);
        ownb::dirty_stack(&mut t, 0x3220 + k, &[]);
        let c = Call {
            eax: 0x00ea_0000 + k,
            ..Call::default()
        };
        t.check_entry(f, c, &[cursor], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) >= 0x40, "the draws");
}

/// The placement's globals in `magic_0600` (operands of 0x10006870), as
/// [`place_globals_0598`].
fn place_globals_0600(t: &mut Twin, state: u32, off: u32, index: u32, rot: f32) {
    let f = t.at(0x6870);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    chars_0598(
        t,
        f,
        &[0x11, 0x71, 0x134, 0x1be, 0x1f8],
        &[0x2e, 0x82, 0x145, 0x1c8, 0x202],
        [3, 4],
    );
    t.set_u32(w(t, 0x1b), state);
    for o in [0x167u32, 0x363, 0x4e1] {
        t.set_u32(w(t, o), off);
    }
    for o in [0x36cu32, 0x4cf, 0x4ea] {
        t.set_u32(w(t, o), index);
    }
    for o in [0x33cu32, 0x3c3, 0x3f4, 0x50c] {
        t.set_u32(w(t, o), CHRS + 0x99);
    }
    t.set_u32(CFG, rot.to_bits());
    for o in [0x54du32, 0x557] {
        let a = w(t, o);
        t.set_u32(a, 0x5a5a_0000 + o);
    }
}

/// `magic_0600` 0x10006870 (`r23::place_pair_0600`): as
/// [`place_pair_0598`].
#[test]
fn place_pair_0600() {
    let Some(mut t) = twin(600) else { return };
    place_stubs_0598(&mut t);
    let f = t.at(0x6870);
    let state = SCRATCH + 0xa000;
    let rots = ownb::rotations(&t, f, 0x9a, 0xa9, 0xc2);
    let mut cases = vec![(0u32, 2u32), (0, 0x20), (1, 2), (0, 0xffff_ffff), (0, 0x1f)];
    while cases.len() < rots.len() {
        cases.push((0, 3));
    }
    for (k, ((off, index), rot)) in cases.into_iter().zip(rots).enumerate() {
        ownb::fill(&mut t, state + 0xac0, 0x220, 5 + k as u32);
        ownb::fill(&mut t, state + 0xea0, 0x30, 9 + k as u32);
        place_globals_0600(&mut t, state, off, index, rot);
        ownb::dirty_stack(&mut t, rot.to_bits(), &[]);
        ownb::reset_counters(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r23::place_pair_0600(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x400, "the placements' calls");
}

/// The start's set-up in `magic_0600` (0x10006530's callees): the part's
/// lock, the records, the task pool; the task list empty or not.
fn start_setup_0600(t: &mut Twin, busy: bool, k: u32) {
    let f = t.at(0x6530);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let pi = t.img.rel32_target(f + 0x71);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(t, pi, 0x04), lock);
    t.set_u32(lock, 0x77 + k);
    let take = t.img.rel32_target(f + 0x115);
    let ri = t.img.rel32_target(f + 0x153);
    let state = w(t, f, 0x38);
    ownb::fill(t, state + 0xac0, 0x220, 5 + k);
    ownb::fill(t, state + 0xea0, 0x40, 9 + k);
    let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
    for i in 0..3u32 {
        t.write(node(i), &[0x33; 0x40]);
        t.set_u32(node(i), node((i + 2) % 3));
        t.set_u32(node(i) + 4, node((i + 1) % 3));
    }
    t.set_u32(w(t, ri, 0x3e), node(0));
    t.set_u16(w(t, ri, 0x20), 3);
    t.set_u32(w(t, ri, 0xa6), 0);
    magic_hi::pool(t, take, SCRATCH + 0x4000, 4);
    let last = SCRATCH + 0x3400;
    t.write(last, &[0x34; 0x20]);
    t.set_u32(w(t, f, 0x11b), if busy { last } else { 0 });
    t.set_u32(w(t, f, 0x12f), last);
    // the words the start clears or sets: old values
    for op in [
        0x7u32, 0x16, 0x20, 0x2a, 0x34, 0x4c, 0x86, 0x90, 0x9b, 0xa3, 0xaa, 0xb4, 0xbf, 0xc7, 0xdb,
        0x101, 0x10c,
    ] {
        t.set_u32(w(t, f, op), 0x5a5a_5a5a ^ op);
    }
    let recs = w(t, f, 0xe4);
    for i in 0..16 {
        t.set_u32(recs + 0x140 * i, 0x7e7e_0000 + i);
    }
    for op in [0x58u32, 0x62, 0x6c, 0x76] {
        t.set_u32(w(t, pi, op), 0x5a5a_0000 ^ op);
    }
}

fn start_stubs_0600(t: &mut Twin) {
    use ownb::rec;
    start2_stubs_0598(t);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
}

/// `magic_0600` 0x10006400 (`r23::cue_start_0600`, with 0x10006530
/// `r23::start_0600`): the waits, the once flag, the start with the task
/// list empty and not.
#[test]
fn cue_start_0600() {
    use ownb::rec;
    let Some(mut t) = twin(600) else { return };
    start_stubs_0600(&mut t);
    let f = t.at(0x6400);
    let st = t.at(0x6530);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 5, 1, |m, _| m.r32(SCRATCH + 0x14));
    rec(&mut t, 28, 2, |m, _| m.r32(SCRATCH + 0x18));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    // (225, 5, 28, once, 227)
    let cases: [(u32, u32, u32, u32, u32); 6] = [
        (1, 0, 0, 0, 1),
        (0, 1, 0, 0, 1),
        (0, 0, 2, 0, 1),
        (0, 0, 0, 0, 0),
        (0, 0, 0, 0, 1),
        (0, 0, 0, 1, 3),
    ];
    for (k, (w225, w5, w28, once, r227)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x14, w5);
        t.set_u32(SCRATCH + 0x18, w28);
        t.set_u32(t.img.word(f + 0x50), once);
        let state = t.img.word(st + 0x38);
        place_globals_0600(&mut t, state, 0, 2, -0.5 + k as f32);
        start_setup_0600(&mut t, k % 2 == 1, k);
        ownb::dirty_stack(&mut t, 0x6400 + k, &[]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x400, "the starts' calls");
}

/// The effect end's set-up in `magic_0600` (0x10006e80): the two-word
/// slots, the party's list, the objects (none listed), the parts, the
/// node `*G_N` at rest or none; the selection, `*G_OFF`, the summon's
/// index and the wait.
fn end_setup_0600(t: &mut Twin, k: u32, node: bool, (off, c, i, wait): (u32, [u8; 3], u32, u16)) {
    use ownb::rec;
    let f = t.at(0x6e80);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    for s in [25u32, 24, 217, 255, 211, 213, 296, 257, 371] {
        rec(t, s, 2, move |_, a| (s << 16) ^ a[0] ^ a[1]);
    }
    t.stub(288, |m, a| {
        m.w32(a[1], 2);
        SCRATCH + 0x4000
    });
    t.write(SCRATCH + 0x4000, &[3, 5]);
    t.stub(293, |_, a| a[0] & 1);
    let rp = t.img.rel32_target(f + 0x254);
    let recs = SCRATCH + 0x6000;
    t.set_u32(w(t, rp, 0x16), recs);
    t.set_u32(recs + 0x30, 0x00c0_0030 + k);
    t.set_u32(w(t, rp, 0x0e), 1);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(t, rp, 0x43), lock);
    t.set_u32(lock, 0x10 + k);
    t.set_u32(w(t, rp, 0x76), 0);
    let n = SCRATCH + 0x3400;
    if node {
        t.write(n, &[0x5a; 0x1c]);
        t.set_u32(n, n);
        t.set_u32(n + 4, n);
        t.set_u32(n + 8, n);
        t.set_u32(n + 0xc, 0);
    }
    t.set_u32(w(t, f, 0x234), if node { n } else { 0 });
    t.set_u32(w(t, f, 0x18d), 0x5a5a_018d);
    let sel = SCRATCH + 0x600;
    for o in [0x11u32, 0x53, 0x101, 0x187] {
        t.set_u32(w(t, f, o), sel);
    }
    t.write(sel, &c);
    for o in [0x1eu32, 0x67, 0x116, 0x1b7] {
        for j in 0..0x40 {
            t.set_u32(w(t, f, o) + 4 * j, CHRS + 0x10 * j);
        }
    }
    for o in [0x5cu32, 0xb7, 0x10a, 0x181, 0x1d7] {
        t.set_u32(w(t, f, o), off);
    }
    for o in [0xc0u32, 0x1df] {
        t.set_u32(w(t, f, o), i);
    }
    for o in [0xaeu32, 0x201] {
        t.set_u32(w(t, f, o), CHRS + 0x99);
    }
    t.set_u32(w(t, f, 0x170), 0x00a0_0170);
    t.set_u16(w(t, f, 0x15e), wait);
}

/// `magic_0600` 0x10006e80 (`r23::effect_end_0600`): the battle on and
/// off, the three indices below 32 and not, the wait 0 and not, the node
/// at rest or none.
#[test]
fn effect_end_0600() {
    let Some(mut t) = twin(600) else { return };
    all_stubs(&mut t);
    let f = t.at(0x6e80);
    let cases: [(u32, [u8; 3], u32, u16, bool); 5] = [
        (0, [3, 4, 5], 2, 0, true),
        (0, [0x20, 0x21, 5], 0x20, 1, false),
        (1, [3, 4, 5], 2, 0, false),
        (0, [0x1f, 0x40, 6], 0xffff_ffff, 1, true),
        (0, [7, 1, 0x30], 0x1f, 0, true),
    ];
    for (k, (off, c, i, wait, node)) in (0u32..).zip(cases) {
        end_setup_0600(&mut t, k, node, (off, c, i, wait));
        ownb::dirty_stack(&mut t, 0x6e80 + k, &[]);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r23::effect_end_0600(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x100, "the end's calls");
}

/// `magic_0600` 0x100066e0 (`r23::task_0600`): the first script running
/// (the points, the motion scripts done, the camera rigs' step) and done
/// (the end, the node given back), as `magic_0598.rs`'s task test.
#[test]
fn task_0600() {
    let Some(mut t) = twin(600) else { return };
    all_stubs(&mut t);
    let f = t.at(0x66e0);
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x7), s);
    let (s1, s2) = (t.img.word(f + 0x2e), t.img.word(f + 0x3d));
    let give = t.img.rel32_target(f + 0x142);
    let node = SCRATCH + 0x5800;
    t.set_u32(t.img.word(f + 0x26), 0x0026_1234);
    for (k, (first, second)) in (0u32..).zip([(4u8, 0u8), (0, 4)]) {
        let block: Vec<u8> = (0..0x30u8).map(|i| 0x51 ^ i.wrapping_mul(7)).collect();
        t.write(t.img.word(f + 0x10), &block);
        t.set_u8(s1 + 0x56, first);
        t.set_u8(s2 + 0x56, second);
        t.write(s + 0xa00, &[0x2d; 0x600]);
        for i in 0..0x20 {
            t.set_u32(s + 0xc54 + 4 * i, 0x0c54_0000 + i);
        }
        // (the end's flag `G_F` is the first motion script's done word)
        end_setup_0600(&mut t, k, k == 0, (0, [3, 4, 5], 2, 0));
        for op in [0x10bu32, 0x11b] {
            let ms = t.img.word(f + op);
            t.set_u32(ms + 0x14, 1);
        }
        magic_hi::lone_node(&mut t, give, node);
        ownb::dirty_stack(&mut t, 0x66e0 + k, &[]);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// The placement's slots and globals in `magic_0568` (operands of
/// 0x10001570): the summon, its index, `*G_OFF`, the rotation.
fn place_setup_0568(t: &mut Twin, state: u32, off: u32, index: u32, rot: f32) {
    let f = t.at(0x1570);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    t.set_u32(w(t, 0x17), state);
    for o in [0x2eu32, 0x72, 0x1c4] {
        t.set_u32(w(t, o), CHRS + 0x99);
    }
    for o in [0x189u32, 0x1a2] {
        t.set_u32(w(t, o), index);
    }
    t.set_u32(w(t, 0x19a), off);
    t.set_u32(CFG, rot.to_bits());
    t.set_u32(w(t, 0x2d2), 0x5a5a_02d2);
    // the part toggle's and the scale's globals: the summon, `*G_B`, `G_C`
    let pt = t.img.rel32_target(f + 0x13d);
    t.set_u32(t.img.word(pt + 0x13), CHRS + 0x99);
    t.set_u32(t.img.word(pt + 0x34), off);
    t.set_u32(t.img.word(pt + 0x3e), index);
}

/// `magic_0568`'s slots for the placement and the start-up.
fn stubs_0568(t: &mut Twin) {
    use ownb::rec;
    place_stubs_0598(t);
    // scale_rotate's slots pass only their own words (and the earlier
    // calls' words under them, which the port leaves out)
    rec(t, 277, 1, |_, _| 0);
    t.stub_st0(277, |_, a| 40.0 + f64::from(a[0] & 0xf));
    for (s, v) in [(15u32, 1.5f64), (16, -0.75), (17, 3.0)] {
        rec(t, s, 1, |_, _| 0);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0x3));
    }
    rec(t, 238, 3, |_, a| 0x0238_0000 ^ a[0]);
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0] ^ a[2]);
    let lock = SCRATCH + 0x7f0;
    rec(t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        fold(m, 711, 0, l);
        0x0711_0000 ^ a[0]
    });
    rec(t, 710, 4, |_, a| a[2] ^ 0x710);
    rec(t, 271, 1, |_, a| 0x0271_0000 ^ a[0]);
}

/// `magic_0568` 0x10001570 (`r23::place_0568`): the angle wrapped down,
/// kept, up, on either limit, NaN; the summon's index in the battle
/// range or not, the battle off. (The stale word `ebp - 0x18` 0.)
#[test]
fn place_0568() {
    let Some(mut t) = twin(568) else { return };
    stubs_0568(&mut t);
    let f = t.at(0x1570);
    let state = SCRATCH + 0xa000;
    let rots = ownb::rotations(&t, f, 0x96, 0xa8, 0xbc);
    let mut cases = vec![(0u32, 2u32), (0, 0x20), (1, 2), (0, 0xffff_ffff), (0, 0x1f)];
    while cases.len() < rots.len() {
        cases.push((0, 3));
    }
    for (k, ((off, index), rot)) in cases.into_iter().zip(rots).enumerate() {
        ownb::fill(&mut t, state + 0x13c0, 0x320, 5 + k as u32);
        place_setup_0568(&mut t, state, off, index, rot);
        ownb::dirty_stack(&mut t, rot.to_bits(), &[0x1c]);
        ownb::reset_counters(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(r23::place_0568(g, img, fr.at, fr.esp))
        });
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the placements' calls");
}

/// `magic_0568` 0x10001340 (`r23::load_0568`, with 0x10003030
/// `r23::start_up_0568`): the waits, the once flag, the start-up with the
/// task list empty and not.
#[test]
fn load_0568() {
    use ownb::rec;
    let Some(mut t) = twin(568) else { return };
    stubs_0568(&mut t);
    let f = t.at(0x1340);
    let su = t.at(0x3030);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
    rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
    rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x1c));
    let lock = SCRATCH + 0x7f0;
    for o in [0x41u32, 0xd0, 0x162] {
        t.set_u32(w(&t, su, o), lock);
    }
    let take = t.img.rel32_target(su + 0x243);
    let ri = t.img.rel32_target(su + 0x282);
    let st = t.img.rel32_target(su + 0x1df);
    t.set_u32(w(&t, st, 0x03), CHRS + 0x200);
    // (225, once, 227, task list)
    let cases: [(u32, u32, u32, bool); 5] = [
        (1, 0, 1, false),
        (0, 0, 0, false),
        (0, 0, 1, false),
        (0, 1, 2, true),
        (0, 0, 1, true),
    ];
    for (k, (w225, once, r227, busy)) in (0u32..).zip(cases) {
        t.set_u32(SCRATCH + 0x10, w225);
        t.set_u32(SCRATCH + 0x1c, r227);
        t.set_u32(w(&t, f, 0x18), once);
        t.set_u32(lock, 0x77 + k);
        let state = w(&t, su, 0x39);
        ownb::fill(&mut t, state + 0x13c0, 0x320, 5 + k);
        place_setup_0568(&mut t, state, 0, 2, -0.5 + k as f32);
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(w(&t, ri, 0x3e), node(0));
        t.set_u16(w(&t, ri, 0x20), 3);
        t.set_u32(w(&t, ri, 0xa6), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(w(&t, su, 0x249), if busy { last } else { 0 });
        t.set_u32(w(&t, su, 0x25e), last);
        for op in [
            0x17u32, 0x21, 0x2b, 0x35, 0x48, 0x99, 0xa3, 0xad, 0xb7, 0xd7, 0x12b, 0x135, 0x13f,
            0x149, 0x169, 0x1b3, 0x1bd, 0x1c7, 0x1d1, 0x1ea, 0x1f4, 0x1ff, 0x207, 0x20e, 0x218,
            0x223, 0x22b,
        ] {
            t.set_u32(w(&t, su, op), 0x5a5a_5a5a ^ op);
        }
        // the placement's stale word (`ebp - 0x18`, 0x5c below the
        // start-up's entry, 0x60 below the load step's)
        ownb::dirty_stack(&mut t, 0x1340 + k, &[0x60]);
        ownb::reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    assert!(t.u32(ownb::LOG) > 0x200, "the start-ups' calls");
}
