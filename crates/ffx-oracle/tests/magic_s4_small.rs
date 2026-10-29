//! The shared-4 list's small slot wrappers (`ffx_magic::s4_small`) against
//! the DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation)]

mod magic;
#[path = "magic/sa.rs"]
mod sa;

use ffx_magic::{Frame, Image, Ret, Xmm, s4_small};
use magic::{Call, Eax, PortGame, SCRATCH, Twin};
use sa::rec;

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

type Port = fn(&mut PortGame, &Image, &Frame) -> Ret;

/// Each slot recorded with `n` words, answering a number of its own.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (s << 20) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// The word at `f + off` of the code.
fn op(t: &Twin, f: u32, off: u32) -> u32 {
    t.img.word(f + off)
}

#[test]
fn word_clear_store_and_574() {
    let Some(mut t) = twin(393) else { return };
    let f = t.at(0x2f70);
    for x in [0u32, 5, 0x8000_0000] {
        t.set_u32(op(&t, f, 0x4), x);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    for (n, rva) in [(443u16, 0x19c0u32), (536, 0x1190)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        t.set_u32(op(&t, f, 0x2), 0x1234_5678);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let Some(mut t) = twin(106) else { return };
    recs(&mut t, &[(574, 0)]);
    t.check_entry(t.at(0x7750), Call::default(), &[], Eax::Checked);
}

#[test]
fn this_calls() {
    let Some(mut t) = twin(155) else { return };
    recs(&mut t, &[(296, 2), (237, 4)]);
    let f = t.at(0xd280);
    t.check(
        f,
        Call::this(0x2961_2961),
        &[],
        Eax::Checked,
        s4_small::call_296,
    );
    let f = t.at(0x8e50);
    let obj = SCRATCH + 0x100;
    t.set_u32(obj + 0x54, 0x0540_0540);
    let c = Call {
        ecx: obj,
        edx: 0x0ed0_0ed0,
        ..Call::default()
    };
    t.check(f, c, &[0x0a0a_0a0a], Eax::Checked, s4_small::call_237);
    drop(t);
    let Some(mut t) = twin(611) else { return };
    recs(&mut t, &[(276, 2)]);
    let obj = SCRATCH + 0x100;
    t.set_u32(obj + 4, 0x0276_0004);
    let f = t.at(0x2960);
    t.check(f, Call::this(obj), &[], Eax::Checked, s4_small::call_276);
    drop(t);
    let Some(mut t) = twin(152) else { return };
    recs(&mut t, &[(37, 3)]);
    let f = t.at(0xd5c0);
    t.check(
        f,
        Call::this(0x0037_0037),
        &[0x3737],
        Eax::Checked,
        s4_small::call_37,
    );
    drop(t);
    let Some(mut t) = twin(106) else { return };
    recs(&mut t, &[(878, 1)]);
    let f = t.at(0x86f0);
    let lock = SCRATCH + 0x300;
    t.set_u32(op(&t, f, 0x1), lock);
    for saved in [0u32, 7] {
        t.set_u32(lock, saved);
        t.check(
            f,
            Call::this(0x0878_0878),
            &[],
            Eax::Checked,
            s4_small::locked_878,
        );
    }
    drop(t);
    let Some(mut t) = twin(391) else { return };
    recs(&mut t, &[(719, 3), (393, 6)]);
    let f = t.at(0x4870);
    let c = Call {
        ecx: 0x0391_0ecc,
        edx: 0x0391_0edd,
        ..Call::default()
    };
    t.check(f, c, &[0xaaaa, 0xbbbb], Eax::Checked, s4_small::handle_393);
}

#[test]
fn done_flags() {
    let flags = SCRATCH + 0x40;
    let Some(mut t) = twin(300) else { return };
    recs(&mut t, &[(432, 0)]);
    t.set_u32(flags, 0x0f0f_0f0b);
    t.check_entry(t.at(0x12e0), Call::default(), &[flags], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(356) else { return };
    recs(&mut t, &[(434, 2)]);
    let f = t.at(0x1550);
    t.set_u32(op(&t, f, 0xf), 0x0434_0434);
    t.set_u32(flags, 0x0101_0101);
    t.check_entry(f, Call::default(), &[flags], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(193) else { return };
    recs(&mut t, &[(213, 2)]);
    let f = t.at(0x1cd0);
    let obj = SCRATCH + 0x200;
    t.set_u32(op(&t, f, 0x4), obj);
    t.set_u32(obj + 0x198, 0x0198_0198);
    t.set_u32(flags, 0xffff_fff3);
    t.check_entry(f, Call::default(), &[flags], Eax::Checked);
}

#[test]
fn plain_slot_calls() {
    let Some(mut t) = twin(481) else { return };
    recs(&mut t, &[(941, 1), (231, 3)]);
    t.check_entry(t.at(0x33a0), Call::default(), &[0x0941_0941], Eax::Checked);
    let f = t.at(0x3170);
    for (i, o) in [0x19u32, 0xd, 0x2].into_iter().enumerate() {
        t.set_u32(op(&t, f, o), 0x0231_0000 + i as u32);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(434) else { return };
    recs(&mut t, &[(579, 3)]);
    let f = t.at(0x7bb0);
    t.set_u32(op(&t, f, 0x11), 0x0579_0579);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x7b90), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(382) else { return };
    recs(&mut t, &[(450, 2)]);
    t.check_entry(t.at(0x1100), Call::default(), &[0x0450_0450], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(430) else { return };
    recs(&mut t, &[(4, 1), (27, 3)]);
    t.check_entry(t.at(0x21e0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(304) else { return };
    recs(&mut t, &[(371, 2)]);
    t.check_entry(t.at(0x1ca0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(570) else { return };
    recs(&mut t, &[(215, 1), (276, 3)]);
    let f = t.at(0xcad0);
    t.set_u32(op(&t, f, 0x7), 0x0215_0215);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(152) else { return };
    recs(&mut t, &[(211, 2)]);
    let f = t.at(0x8230);
    t.set_u32(op(&t, f, 0xc), 0x0211_0211);
    for v in [0u16, 0x10, 0xffef] {
        t.set_u16(op(&t, f, 0x1b), v);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn slot_239_calls() {
    let Some(mut t) = twin(250) else { return };
    recs(&mut t, &[(239, 3)]);
    let f = t.at(0x33e0);
    t.set_u32(op(&t, f, 0x11), 0x0239_0001);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(347) else { return };
    recs(&mut t, &[(239, 3)]);
    let f = t.at(0x3040);
    let obj = SCRATCH + 0x100;
    t.set_u32(obj + 0x120, 0x0120_0120);
    t.set_u32(op(&t, f, 0x15), 0x0239_0002);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(164) else { return };
    recs(&mut t, &[(239, 3)]);
    let f = t.at(0xcf90);
    t.set_u32(op(&t, f, 0x14), 0x0239_0003);
    let c = Call {
        ecx: 0x0ecc_0ecc,
        edx: 0x0edd_0edd,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, s4_small::call_239_edx);
    drop(t);
    let Some(mut t) = twin(188) else { return };
    recs(&mut t, &[(239, 3)]);
    let f = t.at(0x1d60);
    t.set_u32(op(&t, f, 0x1e), 0x0239_000a);
    t.set_u32(op(&t, f, 0x12), 0x0239_000b);
    for busy in [0u32, 1, 0x100] {
        t.set_u32(op(&t, f, 0x2), busy);
        let c = Call {
            eax: 0x00ea_0000 + busy,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(205) else { return };
    recs(&mut t, &[(213, 2)]);
    let f = t.at(0x2420);
    t.set_u32(op(&t, f, 0x12), 0x0213_0213);
    for busy in [0u16, 1, 0x100] {
        t.set_u16(op(&t, f, 0x3), busy);
        t.set_u32(op(&t, f, 0x23), 0x5555);
        let c = Call {
            eax: 0x00ea_0000 + u32::from(busy),
            edx: 0x0edd_0edd,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::call_213_idle);
    }
}

#[test]
fn float_slot_call() {
    let Some(mut t) = twin(42) else { return };
    let f = t.at(0xbde0);
    let c = Call::default().with_xmm(&[Xmm::F32(1.5f32.to_bits()), Xmm::F32((-2.25f32).to_bits())]);
    recs(&mut t, &[(250, 2)]);
    t.check(f, c, &[], Eax::Checked, s4_small::call_250);
    t.stub_st0(250, |_, a| f64::from(f32::from_bits(a[0])) * 3.0);
    t.check(f, c, &[], Eax::Checked, s4_small::call_250);
}

#[test]
fn table_and_records() {
    let Some(mut t) = twin(393) else { return };
    recs(&mut t, &[(632, 3), (624, 1)]);
    let f = t.at(0x3020);
    let p = SCRATCH + 0x100;
    t.set_u32(op(&t, f, 0x1), p);
    for i in 0..4 {
        t.set_u32(op(&t, f, 0x10) + 4 * i, 0x0632_0000 + i);
    }
    for k in [0u8, 1, 3] {
        t.set_u8(p + 1, k);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    let f = t.at(0x1b20);
    let block = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0x8), block);
    t.set_u32(block + 0x1ee4, 0x1ee4);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(249) else { return };
    let f = t.at(0x1ea0);
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0x77; 0x140]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(184) else { return };
    let f = t.at(0x1000);
    let (table, end) = (op(&t, f, 0x21), op(&t, f, 0x12));
    let n = (end - table) / 0x18;
    for free in [None, Some(0u32), Some(5), Some(n - 1)] {
        for i in 0..n {
            t.set_u16(
                table + 0x18 * i + 0x16,
                if Some(i) == free { 0xffff } else { 0xfffe },
            );
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// A node with one chained child, given back.
fn pool_node(t: &mut Twin, node: u32) {
    t.write(node, &[0; 0x20]);
    t.write(node + 0x100, &[0; 0x20]);
    t.set_u32(node + 8, node + 0x100);
}

#[test]
fn give_backs() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x3320);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    for node in [0u32, SCRATCH + 0x600] {
        if node != 0 {
            pool_node(&mut t, node);
        }
        t.set_u32(block + 0xeac, node);
        let c = Call {
            eax: 0x00ea_0001,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(52) else { return };
    let f = t.at(0x1550);
    let answer = SCRATCH + 0x80;
    rec(&mut t, 595, 0, move |m, _| m.r32(answer));
    for busy in [0u32, 5] {
        let node = SCRATCH + 0x600;
        pool_node(&mut t, node);
        t.set_u32(answer, busy);
        t.set_u32(op(&t, f, 0x16), 0x5555);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

#[test]
fn bone_and_models() {
    let Some(mut t) = twin(178) else { return };
    for s in [538, 213, 211] {
        rec(&mut t, s, 2, move |_, a| s << 16 | a[1] & 0xffff);
    }
    rec(&mut t, 547, 1, |_, a| 0x5470_0000 ^ a[0]);
    rec(&mut t, 35, 5, |_, _| 0x35);
    rec(&mut t, 34, 2, |_, _| 0x34);
    rec(&mut t, 276, 2, |_, a| 0x2760_0000 ^ a[0] ^ a[1]);
    rec(&mut t, 219, 2, |_, a| 0x2190_0000 ^ a[1]);
    let f = t.at(0x7950);
    for flags in [0u16, 0x11, 0x810] {
        for node in [op(&t, f, 0xa), op(&t, f, 0x19)] {
            t.write(node, &[0x3c; 0x98]);
            t.set_u16(node + 0x90, flags);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x8e80);
    let bone = t.img.rel32_target(f + 0x15);
    rec(&mut t, 32, 2, |_, a| 0x0032_0000 ^ a[0]);
    rec(&mut t, 233, 3, |m, a| {
        m.w32(a[0], a[2] ^ 0x8000_0000);
        0x233
    });
    for (s, n) in [(325u32, 2usize), (230, 2), (668, 3), (231, 3), (229, 3)] {
        rec(&mut t, s, n, move |_, _| s);
    }
    let table = t.img.word(bone + 0xed);
    for i in 0..4 {
        t.set_u32(table + 4 * i, magic::TRAPS + 16 * 32);
    }
    sa::handler_stubs(&mut t);
    for (k, &(c, p, cp)) in sa::RINGS.iter().enumerate() {
        let node = sa::node(0);
        sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
        for j in 0..0x38 {
            let v = 1.0f32 / (f32::from(j as u8) + 3.0);
            t.set_u32(node + 0x5c + 4 * j, v.to_bits());
        }
        for (o, i) in [(0x13c, 5u32), (0x144, 6), (0x140, 7), (0x128, 8)] {
            let part = sa::node(i);
            t.write(part, &[0x50 + i as u8; 0x200]);
            t.set_u32(part, sa::VTABLE_2);
            for j in 0..0x40 {
                let v = (f32::from(j as u8) + 1.0) * 0.25 * f32::from(i as u8);
                t.set_u32(part + 0x1c + 4 * j, v.to_bits());
            }
            t.set_u32(node + o, part);
        }
        t.set_u8(node + 0x148, k as u8);
        // the walk's pass byte comes from the DLL's global
        t.set_u8(op(&t, f, 0x7), cp);
        t.check(
            f,
            Call::this(node),
            &[],
            if k == 0 { Eax::Junk } else { Eax::Checked },
            s4_small::bone_rate as Port,
        );
    }
}
