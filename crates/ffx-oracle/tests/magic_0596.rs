//! `magic_0596.dll` against its Rust port (`ffx_magic::m0596`): the
//! start entry, the part set-up, an object's fields, the task step that
//! ends the effect, the height callback and the caster's and summon's
//! placement. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

mod magic;
mod magic_hi;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use ffx_magic::{Ret, m0596};
use magic::{Call, Eax, SCRATCH, Twin};
use ownb::{dirty_stack, fill, floats, rec, rec_all, rec_seq, reset_counters};

const N: u16 = 596;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn part_init() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x5060);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    let lock = t.u32(t.img.word(f + 0x04));
    // pppInitData runs with the game's lock set
    rec(&mut t, 711, 3, move |m, a| {
        let l = m.r32(lock);
        m.w32(SCRATCH + 0x300, l);
        m.r32(a[0]) ^ 0x711
    });
    rec(&mut t, 710, 7, |_, a| a[2] ^ 0x710);
    for (saved, part) in [(0u32, 0x1234u32), (7, SCRATCH + 0x100)] {
        t.set_u32(lock, saved);
        t.set_u32(w(&t, 0x0a), part);
        for off in [0x58, 0x62, 0x6c, 0x76] {
            let g = w(&t, off);
            t.set_u32(g, 0x5555_0000 | off);
        }
        t.check(
            f,
            Call::default(),
            &[SCRATCH + 0x200],
            Eax::Checked,
            |g, img, fr| {
                let buf = fr.arg(g, 0);
                Ret::eax(m0596::part_init(g, img, fr.at, buf))
            },
        );
    }
}

#[test]
fn object_init() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x50f0);
    let obj = SCRATCH + 0x400;
    fill(&mut t, obj, 0x50, 9);
    t.check(f, Call::this(obj), &[], Eax::Checked, |g, _, fr| {
        Ret::eax(m0596::object_init(g, fr.ecx))
    });
}

/// The object block's slots (`l5_objhost::obj_block_init`): `n` entries.
fn block_stubs(t: &mut Twin, n: u32) {
    const BLOCK: u32 = SCRATCH + 0x1000;
    const BYTES: u32 = SCRATCH + 0x1800;
    t.set_u32(BLOCK + 4, n);
    for i in 0..8 {
        t.set_u8(BYTES + i, (3 * i + 1) as u8);
    }
    rec(t, 168, 1, |_, _| BLOCK);
    rec(t, 218, 2, |_, _| BYTES);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(t, 291, 5, |_, a| a[1] + 0x10);
    rec(t, 367, 8, |_, a| a[2] ^ 0x367);
}

#[test]
fn start() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x6560);
    rec_all(&mut t, &[(556, 1), (555, 2), (331, 3), (790, 0), (720, 2)]);
    rec_seq(&mut t, 718, &[3, 12, 1], |_, a| a[0] ^ 0x718);
    rec(&mut t, 222, 7, |_, _| 0x222);
    rec_seq(&mut t, 221, &[11, 16], |_, _| 0x221);
    rec(&mut t, 719, 4, |_, a| a[0] + 0x40);
    rec(&mut t, 551, 7, |_, _| 0x551);
    let payloads = t.img.word(f + 0x81);
    let done = t.img.word(f + 0x17d);
    for (n, scale) in [(0u32, None), (3, Some(1.25f64)), (8, Some(-0.1))] {
        block_stubs(&mut t, n);
        match scale {
            Some(s) => t.stub_st0(790, move |_, _| s),
            None => t.stub_st0(790, |_, _| f64::NAN),
        }
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        t.set_u32(done, 0x77);
        for op in [0x57, 0x61, 0x6b, 0xa3, 0xab] {
            let g = t.img.word(f + op);
            t.set_u32(g, 0x0101_0101);
        }
        reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[0x00ab_c000], Eax::Checked);
    }
}

#[test]
fn step_and_height() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x69d0);
    let state = SCRATCH + 0x8000;
    t.set_u32(t.img.word(f + 0x03), state);
    t.set_u32(t.img.word(f + 0x20), 0x1234);
    rec_all(&mut t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
    let ro = t.img.rel32_target(f + 0x08);
    let list = t.img.word(ro + 0x7c);
    let (node, obj) = (SCRATCH + 0x9000, SCRATCH + 0x9100);
    t.set_u32(node + 8, obj);
    // an object that counts but is paused (it ran: no end)
    t.set_u16(obj + 0x124, 0x8001);
    for (busy, s) in [(false, 1u32), (false, 0), (false, 2), (true, 1)] {
        t.set_u32(list + 8, if busy { node } else { 0 });
        t.set_u32(state + 0x6e0, s);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let h = t.at(0x7340);
    t.set_u32(t.img.word(h + 0x01), state);
    floats(&mut t, state + 0x3c4, &[1.0]);
    floats(&mut t, state + 0x424, &[2.0]);
    floats(&mut t, state + 0x454, &[-3.5]);
    t.check_entry(h, Call::default(), &[], Eax::Checked);
}

const CFG: u32 = SCRATCH + 0x700;
const CHRS: u32 = SCRATCH + 0x800;

/// The placement's slots: positions, the rotation (`CFG`), the scales
/// and the matrices.
fn place_stubs(t: &mut Twin) {
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            let v = 1.5 * f32::from(i as u8) + f32::from((a[0] & 0xff) as u8);
            m.w32(a[1 + i as usize], v.to_bits());
        }
        0
    });
    rec(t, 333, 5, |_, _| 0);
    t.stub_st0(333, |m, _| f64::from(f32::from_bits(m.r32(CFG))));
    rec_all(t, &[(210, 2), (211, 2), (238, 5), (296, 2), (257, 2)]);
    rec(t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, 0x4100_0000 + 0x1_0000 * i + a[0]);
        }
        0
    });
    rec(t, 255, 4, |m, a| m.r32(a[1] + 0xc));
    rec_seq(t, 235, &[3, 3, 1], |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, if i % 5 == 0 { 0x3f80_0000 } else { 0 });
        }
        0
    });
    rec(t, 277, 4, |_, _| 0);
    t.stub_st0(277, |_, a| 40.0 + f64::from(a[0] & 0xf));
    for (s, n, v) in [(15u32, 5usize, 1.5f64), (16, 6, -0.75), (17, 7, 3.0)] {
        rec(t, s, n, |_, _| 0);
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
    rec(t, 229, 6, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
            // (kept where the comparison sees it: the vector is a local)
            m.w32(SCRATCH + 0x3c0 + 4 * i, v);
        }
        0
    });
    rec(t, 248, 9, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i).rotate_left(7);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
}

/// One placement: the state block at `state`, `*G_OFF`, the summon's
/// battle index and the caster's rotation.
fn place(t: &mut Twin, state: u32, off: u32, index: u32, rot: f32) {
    let f = t.at(0x6a20);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let sel = SCRATCH + 0x600;
    t.set_u32(w(t, 0x11), sel);
    t.set_u8(sel, 3);
    for i in 0..8 {
        t.set_u32(w(t, 0x30) + 4 * i, CHRS + 0x10 * i);
    }
    t.set_u32(w(t, 0x1d), state);
    t.set_u32(w(t, 0x16a), off);
    t.set_u32(w(t, 0x37a), index);
    t.set_u32(w(t, 0x350), CHRS + 0x99);
    t.set_u32(CFG, rot.to_bits());
    for g in [0x618, 0x622] {
        let a = w(t, g);
        t.set_u32(a, 0);
    }
    // the frame holds junk, but for the junk read's word (0)
    dirty_stack(t, rot.to_bits(), &[0x1c]);
    reset_counters(t);
    t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
        Ret::eax(m0596::place_pair(g, img, fr.at, fr.esp))
    });
}

#[test]
fn place_pair() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    place_stubs(&mut t);
    let state = SCRATCH + 0xa000;
    fill(&mut t, state + 0x380, 0x1c0, 5);
    // the angle wrapped down, kept, wrapped up; the summon's index
    // inside the battle range or not, the battle off
    for (off, index, rot) in [
        (0u32, 2u32, 0.0f32),
        (0, 0x20, 4.712_389),
        (1, 2, 9.0),
        (0, 0xffff_ffff, -1.0),
        (0, 0x1f, f32::NAN),
        (0, 3, 5.0),
    ] {
        place(&mut t, state, off, index, rot);
    }
    // a rotation whose angle lands exactly on the upper wrap limit
    let f = t.at(0x6a20);
    let k = |t: &Twin, op: u32| f32::from_bits(t.u32(t.img.word(f + op)));
    let (a, hi, lo) = (k(&t, 0x9c), k(&t, 0xab), k(&t, 0xc4));
    for limit in [hi, lo] {
        let guess = (a - limit).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (a - r).to_bits() == limit.to_bits())
        {
            place(&mut t, state, 0, 2, r);
        }
    }
    // (the null-matrix paths need a state block at 0xfffffc70 or
    // 0xfffffc30, whose fields wrap into page 0, which the harness leaves
    // unmapped; the game's only caller, 0x10007360, sets the state to
    // the DLL's own block 0x1024a170, so they never run)
}

#[test]
fn place_object() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x5200);
    let w = |t: &Twin, off: u32| t.img.word(f + off);
    t.set_u32(w(&t, 0x01), SCRATCH + 0x200);
    t.set_u32(w(&t, 0x5b), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cd00);
    let programs = SCRATCH + 0x600;
    fill(&mut t, programs, 8, 0x5200);
    rec(&mut t, 941, 1, move |_, _| programs);
    let wrapper = t.img.rel32_target(f + 0x7e);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 9);
    rec(&mut t, 714, 5, |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x220, v);
        0x714
    });
    let alloc = t.img.rel32_target(f + 0x90);
    let (free, nodes) = (t.img.word(alloc + 0x04), t.img.word(alloc + 0x27));
    let obj = SCRATCH + 0x2000;
    let last = SCRATCH + 0x3000;
    // free nodes and an empty list, a list with a last node, no free node
    for (k, (n_free, tail)) in [(5u16, 0u32), (5, last), (0, last)].into_iter().enumerate() {
        fill(&mut t, obj, 0x50, 0x77 + k as u32);
        // the nodes' payloads all free (the free count lies just past them)
        t.write(nodes - 8, &vec![0; 12 * 0x3ff]);
        for i in 0..0x3ff {
            t.set_u32(nodes - 8 + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes - 4 + 12 * i, 0x0ee0_0000 + i);
        }
        t.set_u16(free, n_free);
        t.set_u32(w(&t, 0x9b), tail);
        t.set_u32(last + 4, 0x5a5a);
        t.set_u32(w(&t, 0xbd), 3);
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(0.0),
            ffx_magic::Xmm::f32(1.75),
        ]);
        t.check(f, c, &[], Eax::Checked, |g, img, fr| {
            let scale = fr.xmm_f32(3);
            Ret::eax(m0596::place_object(g, img, fr.at, fr.edx, scale))
        });
    }
}

// Mutation testing of `place_object` (M5 magic round 4, tools/mutants.py):
// 16 mutants, all killed.

/// `m0596::objects_start` (0x10007150): the task list empty and not,
/// the object pool free and full.
#[test]
fn objects_start() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    let f = t.at(0x7150);
    let po = t.img.rel32_target(f + 0x1c);
    let w = |t: &Twin, at: u32, off: u32| t.img.word(at + off);
    t.set_u32(w(&t, po, 0x01), SCRATCH + 0x200);
    t.set_u32(w(&t, po, 0x5b), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cd00);
    let programs = SCRATCH + 0x600;
    fill(&mut t, programs, 8, 0x5200);
    rec(&mut t, 941, 1, move |_, _| programs);
    let wrapper = t.img.rel32_target(po + 0x7e);
    t.set_u32(t.img.word(wrapper + 0x04), SCRATCH + 0x204);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
    let alloc = t.img.rel32_target(po + 0x90);
    let (free, nodes) = (t.img.word(alloc + 0x04), t.img.word(alloc + 0x27));
    let take = t.img.rel32_target(f + 0xdd);
    let r = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, f, 0x0c), r);
    for (k, (n_free, busy)) in [(5u16, false), (5, true), (0, true)]
        .into_iter()
        .enumerate()
    {
        fill(&mut t, r, 0x200, 0x7150 + k as u32);
        t.write(nodes - 8, &vec![0; 12 * 0x20]);
        for i in 0..0x20 {
            t.set_u32(nodes - 8 + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes - 4 + 12 * i, 0x0ee0_0000 + i);
        }
        t.set_u16(free, n_free);
        t.set_u32(w(&t, po, 0x9b), 0);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 6);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        for (op, v) in [(0xe3u32, if busy { last } else { 0 }), (0xfa, last)] {
            t.set_u32(w(&t, f, op), v);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0596::start2` (0x10007360): the task list empty and not; the
/// placement as in [`place_pair`] (its junk word 0, 12 bytes deeper).
#[test]
fn start2() {
    let Some(mut t) = magic_l3::twin(N) else {
        return;
    };
    place_stubs(&mut t);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    rec(&mut t, 711, 3, |_, a| 0x0711_0000 ^ a[0]);
    rec(&mut t, 710, 7, |_, a| a[2] ^ 0x710);
    rec(&mut t, 271, 3, |_, a| 0x0271_0000 ^ a[0]);
    let f = t.at(0x7360);
    let pp = t.at(0x6a20);
    let w = |t: &Twin, at: u32, o: u32| t.img.word(at + o);
    let sel = SCRATCH + 0x600;
    t.set_u32(w(&t, pp, 0x11), sel);
    t.set_u8(sel, 3);
    for i in 0..8 {
        t.set_u32(w(&t, pp, 0x30) + 4 * i, CHRS + 0x10 * i);
    }
    t.set_u32(w(&t, pp, 0x16a), 0);
    t.set_u32(w(&t, pp, 0x37a), 2);
    t.set_u32(w(&t, pp, 0x350), CHRS + 0x99);
    t.set_u32(CFG, 1.25f32.to_bits());
    let st = t.img.rel32_target(f + 0x56);
    t.set_u32(w(&t, st, 0x03), CHRS + 0x200);
    let pi = t.img.rel32_target(f + 0x4c);
    t.set_u32(t.u32(w(&t, pi, 0x04)), 0);
    let take = t.img.rel32_target(f + 0xba);
    let ri = t.img.rel32_target(f + 0xf8);
    for (k, busy) in [false, true].into_iter().enumerate() {
        // the state block the start points the placement at (the DLL's)
        let state = w(&t, f, 0x38);
        fill(&mut t, state + 0x380, 0x1c0, 5 + k as u32);
        for g in [0x618, 0x622] {
            let a = w(&t, pp, g);
            t.set_u32(a, 0);
        }
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
        t.set_u32(w(&t, f, 0xc0), if busy { last } else { 0 });
        t.set_u32(w(&t, f, 0xd4), last);
        // the script blocks' words the start clears: old values
        for op in [0x6bu32, 0x76, 0x8f, 0x9a] {
            t.set_u32(w(&t, f, op), 0x5a5a_5a5a);
        }
        dirty_stack(&mut t, 0x7360 + k as u32, &[0x28]);
        reset_counters(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
