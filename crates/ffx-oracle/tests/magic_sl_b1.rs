//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 1: thunks, load and motion steps, a node build. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use ffx_magic::{Ret, m0152};
use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// A visited-node handler behind its `ecx` thunk: a node already
/// visited (`node + 0x1a` = the cursor's pass byte) and, where `full`, one
/// not visited without children.
fn thunk(n: u16, rva: u32, full: bool) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| m.r32(a[0]) ^ 0x1000);
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    let cases: &[bool] = if full { &[true, false] } else { &[true] };
    for &same in cases {
        t.write(node, &[0x22; 0x48]);
        for i in 0..9 {
            t.set_u32(node + 0x1c + 4 * i, 0x100 * i + 7);
        }
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x10, node);
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        dirty(&mut t);
        let c = Call {
            eax: 0x1234_5678,
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(at, c, &[cur, 0x0a0a_0a0a], Eax::Checked);
    }
}

#[test]
fn thunk_sub_node_visit_0146() {
    thunk(146, 0x6380, false);
}

#[test]
fn thunk_int_motion_0186() {
    thunk(186, 0x57f0, true);
}

#[test]
fn thunk_matrix_visit_0201() {
    thunk(201, 0x43a0, false);
}

/// `m0052::load_step` (0x10001250): slot 225 or 593 busy, the first and
/// a later request, the data not loaded and loaded (the start-up runs).
#[test]
fn load_step_0052() {
    let Some(mut t) = twin(52) else { return };
    let at = t.at(0x1250);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    let (busy, busy2, loaded) = (SCRATCH + 0x100, SCRATCH + 0x104, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 593, 0, move |m, _| m.r32(busy2));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    for (slot, k) in [(262u32, 0u32), (6, 1), (548, 2), (413, 3), (215, 4)] {
        rec(&mut t, slot, 1, move |_, a| a[0] ^ (0x0100_0000 * (k + 1)));
    }
    let start = t.img.rel32_target(at + 0x6c);
    let ws = |t: &Twin, o: u32| t.img.word(start + o);
    let bytes = SCRATCH + 0x500;
    t.set_u32(ws(&t, 0x3d), bytes);
    t.set_u32(ws(&t, 0x68), bytes);
    t.write(bytes, &[2, 0x40]);
    t.set_u32(ws(&t, 0x56) + 8, 0x00ee_0001);
    t.set_u32(ws(&t, 0x7e) + 8, 0x00ee_0002);
    t.set_u32(w(&t, 0x37), 0x0000_5e5e);
    let take = t.img.rel32_target(start + 0xe7);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (b, b2, asked, l) in [
        (1u32, 0u32, 0u32, 1u32),
        (0, 1, 0, 1),
        (0, 0, 0, 0),
        (0, 0, 1, 0xffff_ffff),
        (0, 0, 0, 1),
        (0, 0, 1, 2),
    ] {
        t.set_u32(busy, b);
        t.set_u32(busy2, b2);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x29), asked);
        for o in [0x72u32, 0x7c] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        for i in 1..=4 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node(if i == 1 { 4 } else { i - 1 }));
            t.set_u32(node(i) + 4, node(if i == 4 { 1 } else { i + 1 }));
        }
        t.set_u32(head, node(1));
        t.set_u16(count, 4);
        t.set_u32(ws(&t, 0x01), 0x8000_0400);
        t.set_u32(ws(&t, 0x11), 1);
        for op in [0x29, 0xb7, 0xd1, 0xed, 0x104] {
            t.set_u32(ws(&t, op), 0);
        }
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0098::motion_step` (0x10002380): either record, the end word 0 or
/// not.
#[test]
fn motion_step_0098() {
    let Some(mut t) = twin(98) else { return };
    let at = t.at(0x2380);
    let w = |t: &Twin, o: u32| t.img.word(at + o);
    rec(&mut t, 211, 2, |_, a| a[0] ^ a[1]);
    rec(&mut t, 37, 3, |_, a| 0x0370_0000 ^ a[2]);
    rec(&mut t, 30, 1, |_, _| 0x30);
    rec(&mut t, 18, 1, |_, a| a[0] & 0xff);
    rec(&mut t, 22, 1, |_, a| a[0] << 4);
    t.set_u32(w(&t, 0xc), 0x00c0_ffee);
    let entries = SCRATCH + 0x200;
    for (sel, end) in [(0u32, 0u32), (1, 0), (1, 5), (0, 9)] {
        for (k, rec_at) in [w(&t, 0x23), w(&t, 0x2a)].into_iter().enumerate() {
            let k = u32::try_from(k).unwrap();
            let e = entries + 0x40 * k;
            t.write(e, &[0; 0x20]);
            t.set_u32(e, 0x0010_0000 + k);
            t.set_u32(e + 4, 3 + k);
            t.set_u32(rec_at, e);
            t.set_u32(rec_at + 4, 0x0040_0000 + k);
            t.set_u32(rec_at + 8, 0);
            t.set_u32(rec_at + 0xc, 0);
        }
        t.set_u32(w(&t, 0x1d), sel);
        t.set_u32(w(&t, 0x37), end);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0152::build_node` (0x10008170): the record built on the stack
/// under the entry, then the node built.
#[test]
fn build_node_0152() {
    let Some(mut t) = twin(152) else { return };
    let f = t.at(0x8170);
    let node = SCRATCH + 0x1000;
    rec(&mut t, 287, 3, |_, a| 0x0287_0000 ^ a[2]);
    rec(&mut t, 283, 2, |_, a| a[0] ^ a[1]);
    t.write(node, &[0xc3; 0x200]);
    t.set_u32(node + 0x10, node);
    // the parent (the record's second word, a DLL global) and its last
    // child
    let parent = t.img.word(f + 0x18);
    let last = SCRATCH + 0x600;
    t.write(last, &[0x5a; 0x10]);
    t.set_u32(parent + 0x10, last);
    dirty(&mut t);
    let args = [1, 2, 3, 4, 0x0a0a_0005, 6, 0x0b0b_0007, 8];
    t.check(f, Call::this(node), &args, Eax::Checked, |g, img, fr| {
        Ret::eax(m0152::build_node(g, img, fr.at, fr.ecx, fr.esp))
    });
}
