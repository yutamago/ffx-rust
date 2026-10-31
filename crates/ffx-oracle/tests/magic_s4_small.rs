//! The shared-4 list's small slot wrappers (`ffx_magic::s4_small`) against
//! the DLLs they were ported from. Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

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
    // the flag clear and already set
    for x in [0x0f0f_0f0b, 0x0f0f_0f0f] {
        t.set_u32(flags, x);
        t.check_entry(t.at(0x12e0), Call::default(), &[flags], Eax::Checked);
    }
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

#[test]
fn slot_pairs() {
    let Some(mut t) = twin(70) else { return };
    recs(&mut t, &[(211, 2)]);
    let f = t.at(0x95f0);
    let obj = SCRATCH + 0x100;
    t.set_u32(obj + 0x54, 0x0054_0054);
    for flags in [0u16, 0x10, 0xffef] {
        t.set_u16(obj + 0x90, flags);
        let c = Call {
            ecx: obj,
            edx: 0x0e0d_0000,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::attach_211);
    }
    drop(t);
    let Some(mut t) = twin(168) else { return };
    recs(&mut t, &[(568, 1), (566, 2), (575, 1)]);
    t.check_entry(t.at(0x2370), Call::default(), &[0x5750_0575], Eax::Checked);
    drop(t);
    for (n, rva) in [(356u16, 0x1770u32), (301, 0x2220)] {
        let Some(mut t) = twin(n) else { return };
        recs(&mut t, &[(450, 2), (451, 4)]);
        t.check_entry(t.at(rva), Call::default(), &[0x0b0b_0001], Eax::Checked);
    }
    let Some(mut t) = twin(384) else { return };
    recs(&mut t, &[(450, 2), (451, 4), (432, 0)]);
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0x11; 0x200]);
    t.check_entry(t.at(0x15a0), Call::default(), &[obj], Eax::Checked);
    t.check_entry(t.at(0x1460), Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(415) else { return };
    recs(&mut t, &[(447, 2), (461, 4)]);
    let f = t.at(0x1100);
    let base = SCRATCH + 0x200;
    t.set_u32(op(&t, f, 0x4), base);
    t.set_u32(base + 0x198, 0x0198_0001);
    t.check_entry(f, Call::default(), &[0x0b0b_0002], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(393) else { return };
    recs(&mut t, &[(623, 7)]);
    let f = t.at(0x1ae0);
    let block = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0xe), block);
    t.set_u32(op(&t, f, 0x20), 0x0623_0623);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    let f = t.at(0x30a0);
    for i in 0..4 {
        t.set_u32(block + 0x1c00 + 4 * i, 0x1c00 + i);
    }
    t.set_u32(op(&t, f, 0x5), block);
    t.check_entry(f, Call::default(), &[SCRATCH + 0x100], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(162) else { return };
    recs(&mut t, &[(257, 2)]);
    let f = t.at(0x4f30);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x1), p);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 219, 2, move |m, _| m.r32(answer));
    for part in [0u32, 0x0219_0001] {
        t.set_u8(p, 0x42);
        t.set_u32(answer, part);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(484) else { return };
    recs(&mut t, &[(402, 0), (713, 1)]);
    let f = t.at(0x1320);
    t.set_u32(op(&t, f, 0x11), SCRATCH + 0x80);
    t.check_entry(f, Call::default(), &[0x0713_0001], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(530) else { return };
    let f = t.at(0x1100);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 411, 1, move |m, _| m.r32(answer));
    recs(&mut t, &[(213, 2)]);
    t.set_u32(op(&t, f, 0x7), 0x0411_0411);
    for a in [0u32, 1, u32::MAX] {
        t.set_u32(answer, a);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(249) else { return };
    recs(&mut t, &[(296, 2), (257, 2)]);
    let f = t.at(0x24b0);
    t.set_u32(op(&t, f, 0x30), 0x0257_0257);
    for (done, n) in [(0u32, 0u32), (0, 0x1f), (0, 0x20), (0, u32::MAX), (1, 3)] {
        t.set_u32(op(&t, f, 0x2), done);
        t.set_u32(op(&t, f, 0xa), n);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn bones_and_maps() {
    let obj = SCRATCH + 0x100;
    let Some(mut t) = twin(470) else { return };
    recs(&mut t, &[(237, 4)]);
    let f = t.at(0x1640);
    t.set_u32(op(&t, f, 0x22), 0x0237_0001);
    t.write(obj, &[0x22; 0x200]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(307) else { return };
    recs(&mut t, &[(237, 4)]);
    let f = t.at(0x2b30);
    t.set_u32(op(&t, f, 0x1d), 0x0237_0002);
    t.write(obj, &[0x22; 0x200]);
    t.set_u32(obj + 0x120, 0x33);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(349) else { return };
    recs(&mut t, &[(388, 3)]);
    let f = t.at(0x4e90);
    let map = SCRATCH + 0x300;
    t.write(map, &[9, 8, 7, 6, 5, 4]);
    t.set_u32(op(&t, f, 0x19), map);
    t.set_u32(op(&t, f, 0x12), 0x0e0e_0012);
    for i in [u32::MAX, 0, 3] {
        t.set_u32(obj + 0x120, i);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(297) else { return };
    recs(&mut t, &[(357, 3), (356, 6)]);
    let task = SCRATCH + 0x100;
    let (rec, actor) = (SCRATCH + 0x200, SCRATCH + 0x300);
    t.set_u32(task, rec);
    t.set_u32(rec + 0x88, actor);
    t.set_u32(actor + 0x54, 0x0054_0001);
    t.check_entry(t.at(0x3ce0), Call::default(), &[task], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(481) else { return };
    recs(&mut t, &[(712, 2)]);
    let f = t.at(0x1000);
    let lock = SCRATCH + 0x300;
    t.set_u32(op(&t, f, 0xd), lock);
    t.set_u32(op(&t, f, 0x1), SCRATCH + 0x80);
    for saved in [0u32, 9] {
        t.set_u32(lock, saved);
        let c = Call {
            edx: 0x0712_0001,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::locked_712);
    }
    drop(t);
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x8d70);
    for n in [0u32, 1, 30, 0x1_0005, u32::MAX] {
        let c = Call {
            eax: 0x00ea_0000,
            edx: n,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::frames_time);
    }
}

#[test]
fn node_records() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x98a0);
    let parent = op(&t, f, 0x15);
    let node = SCRATCH + 0x100;
    for kids in [0u32, 1] {
        t.write(node, &[0x44; 0x200]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(
            parent + 0x10,
            if kids == 0 { parent } else { SCRATCH + 0x400 },
        );
        t.set_u32(
            parent + 0xc,
            if kids == 0 { parent } else { SCRATCH + 0x400 },
        );
        t.write(SCRATCH + 0x400, &[0; 0x20]);
        t.check(
            f,
            Call::this(node),
            &[0x0c0c_0001],
            Eax::Checked,
            s4_small::big_node,
        );
    }
    drop(t);
    let Some(mut t) = twin(159) else { return };
    let f = t.at(0x66c0);
    let owner = SCRATCH + 0x600;
    t.set_u32(op(&t, f, 0xd), owner);
    t.write(owner, &[0x10; 0x100]);
    recs(&mut t, &[(325, 2)]);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x800);
    let list = op(&t, f, 0x20);
    t.write(list, &[0; 0x20]);
    t.set_u32(list + 0x10, list);
    t.set_u32(list + 0xc, list);
    let node = SCRATCH + 0x100;
    t.write(node, &[0x44; 0x1100]);
    t.check(
        f,
        Call::this(node),
        &[0x0c0c_0002],
        Eax::Checked,
        s4_small::particle_from,
    );
}

/// The object list of the `run_objects` at `ro`: empty, or one object
/// that counts as run (flag 0x8000) without running (flag 1).
fn objects(t: &mut Twin, ro: u32, one: bool) {
    let list = t.img.word(ro + 0x7c);
    let node = SCRATCH + 0x7000;
    let obj = SCRATCH + 0x7100;
    t.write(node, &[0; 12]);
    t.set_u32(node + 8, obj);
    t.write(obj, &[0; 0x130]);
    t.set_u16(obj + 0x124, 0x8001);
    let n = u32::from(one);
    for o in [0u32, 4, 8] {
        t.set_u32(list + o, if one { node } else { 0 });
    }
    t.set_u32(list + 0xc, n);
}

#[test]
fn steps_until_done() {
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x4760);
    let ro = t.img.rel32_target(f + 0x8);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3), (214, 1)]);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    for (one, ready) in [(false, 1u32), (false, 0), (true, 1)] {
        objects(&mut t, ro, one);
        t.set_u32(block + 0x1c08, ready);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(369) else { return };
    let f = t.at(0x1340);
    let run = t.img.rel32_target(f + 0x5);
    let ro = t.img.rel32_target(f + 0xc);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3)]);
    let s = t.img.word(run + 0x1c);
    let task = SCRATCH + 0x100;
    let rec_at = SCRATCH + 0x200;
    for (mode, one) in [(0u8, false), (2, false), (0, true)] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, rec_at);
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x28, 0);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(442) else { return };
    let f = t.at(0x2f10);
    let ms = t.img.rel32_target(f + 0x5);
    let ro = t.img.rel32_target(f + 0xc);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3)]);
    let task = SCRATCH + 0x100;
    let rec_at = SCRATCH + 0x200;
    for (done, one, wait) in [
        (1u32, false, 0u32),
        (0, false, 0),
        (1, true, 0),
        (1, false, 1),
        (1, false, u32::MAX),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, rec_at);
        t.set_u32(t.img.word(ms + 0x02), done);
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(op(&t, f, 0x1b), wait);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn records_and_sequences() {
    let Some(mut t) = twin(205) else { return };
    let f = t.at(0x2450);
    let r = SCRATCH + 0x100;
    t.write(r, &[0x66; 0x40]);
    t.check(
        f,
        Call::this(r),
        &[0x0024_0024],
        Eax::Checked,
        s4_small::record_reset,
    );
    drop(t);
    for (n, rva, nop, with) in [(667u16, 0x2660u32, 0x8u32, false), (515, 0x2660, 0x7, true)] {
        let Some(mut t) = twin(n) else { return };
        recs(&mut t, &[(296, 2)]);
        let f = t.at(rva);
        let recs_at = op(&t, f, 0xd);
        for ids in [
            &[][..],
            &[3u32, 0xff][..],
            &[0xff, 4, 5][..],
            &[7, 0xff, 0xff][..],
        ] {
            t.set_u32(op(&t, f, nop), ids.len() as u32);
            for (i, &id) in (0u32..).zip(ids) {
                t.set_u32(recs_at + 0xcc * i, id);
            }
            let c = Call {
                eax: 0x00ea_0000,
                ..Call::default()
            };
            let args: &[u32] = if with { &[0x0296_0002] } else { &[] };
            t.check_entry(f, c, args, Eax::Checked);
        }
    }
    let Some(mut t) = twin(312) else { return };
    recs(&mut t, &[(450, 2), (451, 4), (440, 4), (449, 4)]);
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0x10; 0x200]);
    let f = t.at(0x1d30);
    t.set_u32(op(&t, f, 0x12), 0x5555);
    t.set_u32(op(&t, f, 0x1c), 0x6666);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    let f = t.at(0x16f0);
    t.set_u32(op(&t, f, 0x19), 0x0440_0440);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    let f = t.at(0x1300);
    let base = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0x4), base);
    t.set_u32(base + 0x198, 0x0198_0198);
    for k in [0u32, 3] {
        t.set_u32(obj + 0xac, k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(194) else { return };
    recs(&mut t, &[(450, 2), (449, 4), (434, 2), (451, 6)]);
    let f = t.at(0x1000);
    t.set_u32(op(&t, f, 0x18), 0x5555);
    t.set_u32(op(&t, f, 0x22), 0x6666);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    let f = t.at(0x1170);
    t.set_u32(op(&t, f, 0xf), 0x0434_0434);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(378) else { return };
    recs(&mut t, &[(450, 4), (434, 2), (451, 6)]);
    let f = t.at(0x10d0);
    t.set_u32(op(&t, f, 0xf), 0x0434_0435);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(421) else { return };
    recs(&mut t, &[(447, 2), (450, 4), (451, 6)]);
    let f = t.at(0x13d0);
    let base = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0x4), base);
    t.set_u32(base + 0x198, 0x0198_0199);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(193) else { return };
    recs(&mut t, &[(448, 2), (450, 4), (449, 6)]);
    let f = t.at(0x1000);
    t.set_u32(op(&t, f, 0x14), 0x5555);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(162) else { return };
    recs(&mut t, &[(4, 1), (27, 3)]);
    t.check_entry(t.at(0x2d60), Call::default(), &[], Eax::Checked);
}

#[test]
fn indexed_calls() {
    let Some(mut t) = twin(391) else { return };
    recs(&mut t, &[(542, 2)]);
    let f = t.at(0x1630);
    let map = SCRATCH + 0x300;
    t.write(map, &[4, 3, 2, 1]);
    t.set_u32(op(&t, f, 0x1d), map);
    t.set_u32(op(&t, f, 0x16), 0x0e0e_0016);
    for n in [0u32, 1, 3, u32::MAX] {
        t.set_u32(op(&t, f, 0x8), n);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: 0x0542_0ecc,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::each_542);
    }
    drop(t);
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x3c90);
    let map = SCRATCH + 0x300;
    t.write(map, &[0, 1, 2, 3, 0]);
    t.set_u32(op(&t, f, 0x15), map);
    t.set_u32(op(&t, f, 0x1c), SCRATCH + 0x800);
    for k in 0..4u32 {
        let chr = SCRATCH + 0x1000 + 0x1000 * k;
        t.set_u32(op(&t, f, 0x30) + 4 * k, chr);
        t.set_u32(chr + 0x830, 0x0c00_0000 + k);
    }
    for (n, this) in [
        (1u32, 0x0c00_0001u32),
        (4, 0x0c00_0002),
        (4, 0x0c00_0000),
        (5, 0x0c00_0009),
        (5, 0x0c00_0001),
    ] {
        t.set_u32(op(&t, f, 0x2), n);
        t.check(
            f,
            Call::this(this),
            &[],
            Eax::Checked,
            s4_small::actor_index,
        );
    }
}

#[test]
fn places_and_products() {
    let obj = SCRATCH + 0x100;
    let Some(mut t) = twin(393) else { return };
    recs(&mut t, &[(268, 3), (713, 1)]);
    let f = t.at(0x2f80);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x4), p);
    for i in 0..4 {
        t.set_u32(op(&t, f, 0x1d) + 4 * i, 0x0268_0000 + i);
    }
    for k in [0u8, 2] {
        t.set_u8(p + 1, k);
        t.write(obj, &[0x77; 0x200]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    // the block object, listed or not
    let f = t.at(0x3050);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    t.set_u32(op(&t, f, 0x1e), SCRATCH + 0x90);
    let lr = t.img.rel32_target(f + 0x15);
    let list = op(&t, f, 0x10);
    let node = SCRATCH + 0x3000;
    let free = t.img.word(lr + 0x4d);
    for listed in [false, true] {
        t.write(block + 0x850, &[0x5a; 0x130]);
        t.set_u32(node, 0);
        t.set_u32(node + 4, 0);
        t.set_u32(node + 8, if listed { block + 0x850 } else { 0x1234 });
        t.set_u32(list, node);
        t.set_u32(list + 4, node);
        t.set_u32(list + 8, node);
        t.set_u32(list + 0xc, 1);
        t.set_u16(free, 7);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(55) else { return };
    recs(&mut t, &[(237, 4)]);
    let f = t.at(0x1950);
    t.set_u32(op(&t, f, 0x24), 0x0237_0237);
    for k in [0u32, 3] {
        t.set_u32(op(&t, f, 0x4), k);
        t.write(obj, &[0x77; 0x200]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(104) else { return };
    let f = t.at(0x1830);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x7), p);
    t.set_u8(p, 2);
    rec(&mut t, 364, 1, |_, _| 0x364);
    rec(&mut t, 16, 2, |_, _| 0x16);
    for (a, b) in [(1.5f64, 2.0f64), (-3.0, 0.1), (1e30, 1e30)] {
        t.stub_st0(364, move |_, _| a);
        t.stub_st0(16, move |_, _| b);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(673) else { return };
    let f = t.at(0x1900);
    for count in [0x41u32, 0x40, 0x3f, 2, 1, 0, u32::MAX] {
        t.write(obj, &[0x33; 0x200]);
        t.set_u32(obj + 0x120, count);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(249) else { return };
    let f = t.at(0x1ee0);
    let (src, end) = (op(&t, f, 0x6) - 4, op(&t, f, 0x43));
    for i in 0..(end - src) / 4 {
        t.set_u32(src + 4 * i, 0x0010_0000 + i);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn lights() {
    let Some(mut t) = twin(70) else { return };
    recs(&mut t, &[(325, 2), (234, 3)]);
    let f = t.at(0x9480);
    let c = Call {
        edx: 0x0edd_0001,
        ..Call::default()
    }
    .with_xmm(&[Xmm::F32(0), Xmm::F32(0), Xmm::F32(2.5f32.to_bits())]);
    t.check(f, c, &[], Eax::Checked, s4_small::light_set);
    let f = t.at(0x9950);
    let c = Call::default().with_xmm(&[Xmm::F32(0), Xmm::F32((-0.75f32).to_bits())]);
    t.check(f, c, &[], Eax::Checked, s4_small::light_on);
}

#[test]
fn motion_register() {
    let Some(mut t) = twin(347) else { return };
    let f = t.at(0x3c10);
    let ms = t.img.rel32_target(f + 0x5);
    let ro = t.img.rel32_target(f + 0xc);
    let rp = t.img.rel32_target(f + 0x42);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3)]);
    // no parts and no banks to register
    t.set_u32(t.img.word(rp + 0xe), 0);
    let task = SCRATCH + 0x100;
    let rec_at = SCRATCH + 0x200;
    for (done, one, wait, busy) in [
        (1u32, false, 0u32, 0u32),
        (0, false, 0, 0),
        (1, true, 0, 0),
        (1, false, 1, 0),
        (1, false, 0, 1),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, rec_at);
        t.set_u32(t.img.word(ms + 0x02), done);
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(op(&t, f, 0x1b), wait);
        t.set_u32(op(&t, f, 0x24), busy);
        objects(&mut t, ro, one);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
}

#[test]
fn fades_and_levels() {
    let Some(mut t) = twin(168) else { return };
    recs(&mut t, &[(541, 3), (38, 3)]);
    let f = t.at(0x3670);
    t.set_u32(op(&t, f, 0x46), 0x0038_0038);
    for (done, n) in [(0u32, 0u32), (0, 0x1f), (0, 0x20), (0, u32::MAX), (1, 3)] {
        t.set_u32(op(&t, f, 0x2), done);
        t.set_u32(op(&t, f, 0xa), n);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(55) else { return };
    recs(&mut t, &[(541, 3), (38, 3)]);
    let f = t.at(0x2320);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x9), p);
    for i in 0..0x21 {
        t.set_u32(op(&t, f, 0x3f) + 4 * i, 0x0038_0000 + i);
    }
    for (done, k) in [(0u32, 3u8), (0, 0x1f), (0, 0x20), (1, 3)] {
        t.set_u32(op(&t, f, 0x2), done);
        t.set_u8(p, k);
        let c = Call::default().with_xmm(&[Xmm::F32(0), Xmm::F32(0.4f32.to_bits())]);
        t.check(f, c, &[], Eax::Checked, s4_small::fade_541_or_38_by);
    }
    drop(t);
    let Some(mut t) = twin(530) else { return };
    recs(&mut t, &[(391, 4)]);
    let f = t.at(0x1010);
    for l in [0u32, 1, 5, 0x40, 0x200, 0xffff_fff0, 0x7fff_ffff] {
        t.set_u32(op(&t, f, 0x4), l);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(150) else { return };
    recs(&mut t, &[(10, 2), (804, 3)]);
    let f = t.at(0xddf0);
    t.set_u32(op(&t, f, 0x9), 0x0010_0001);
    t.set_u32(op(&t, f, 0x19), 0x0804_0001);
    t.set_u32(op(&t, f, 0x2c), 0x0804_0002);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(393) else { return };
    recs(&mut t, &[(4, 1), (27, 3)]);
    let f = t.at(0x1990);
    let block = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0xb), block);
    for h in [u32::MAX, 7] {
        t.set_u32(block + 0x1eec, h);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(191) else { return };
    recs(&mut t, &[(450, 2), (451, 4)]);
    let obj = SCRATCH + 0x100;
    for rva in [0x1b50u32, 0x1940] {
        t.write(obj, &[0x55; 0x200]);
        t.check_entry(t.at(rva), Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn scale_task_through() {
    let Some(mut t) = twin(434) else { return };
    recs(&mut t, &[(371, 2)]);
    let f = t.at(0x6890);
    let st = t.img.rel32_target(f + 0x48);
    let take = t.img.rel32_target(st + 0x7d);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xe), block);
    let head = t.img.word(st + 0x83);
    for listed in [false, true] {
        let node = |i: u32| SCRATCH + 0x4000 + 0x40 * i;
        for i in 1..=3u32 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
            t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
        }
        t.set_u32(t.img.word(take + 0x17), node(1));
        t.set_u16(t.img.word(take + 0x04), 3);
        t.write(block + 0x1768, &[0x11; 0x40]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(head, SCRATCH + 0x5000);
            t.set_u32(t.img.word(st + 0x9b), SCRATCH + 0x5000);
        } else {
            t.set_u32(head, 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn node_links() {
    for (n, rva, args, port) in [
        (152u16, 0x83e0u32, &[][..], s4_small::link_node as Port),
        (42, 0x8360, &[0x0001_001c][..], s4_small::link_node_x),
        (262, 0x6cc0, &[0x0001_0038][..], s4_small::link_node_38),
        (
            42,
            0x81a0,
            &[0x0001_002c, 0x0001_0030][..],
            s4_small::link_node_xy,
        ),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let node = SCRATCH + 0x100;
        // the list's last word: read through two operands, the same word
        let last_at = match rva {
            0x83e0 => op(&t, f, 0x29),
            0x8360 => op(&t, f, 0x2c),
            _ => op(&t, f, 0x26),
        };
        let parent = match rva {
            0x83e0 => op(&t, f, 0x2e),
            0x8360 => op(&t, f, 0x31),
            _ => op(&t, f, 0x2b),
        };
        for empty in [true, false] {
            t.write(node, &[0x66; 0x40]);
            t.write(SCRATCH + 0x300, &[0x77; 0x10]);
            t.set_u32(last_at, if empty { parent } else { SCRATCH + 0x300 });
            t.check(f, Call::this(node), args, Eax::Checked, port);
        }
    }
}

#[test]
fn clears_and_registers() {
    let Some(mut t) = twin(376) else { return };
    let f = t.at(0x3a30);
    let (head, recs_at) = (SCRATCH + 0x100, SCRATCH + 0x1000);
    t.write(head, &[0x99; 0x60]);
    t.write(recs_at, &[0x99; 0x3a00]);
    let c = Call {
        ecx: head,
        edx: recs_at,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, s4_small::records_clear_100);
    drop(t);
    let Some(mut t) = twin(511) else { return };
    let f = t.at(0x5c80);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    recs(&mut t, &[(779, 4)]);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x21), lock);
    t.set_u32(lock, 5);
    t.check(
        f,
        Call::this(0x0511_0511),
        &[],
        Eax::Checked,
        s4_small::register_320,
    );
    drop(t);
    let Some(mut t) = twin(132) else { return };
    let f = t.at(0x1930);
    let ro = t.img.rel32_target(f + 0x7);
    let da = t.img.rel32_target(f + 0x37);
    let r80 = t.img.rel32_target(f + 0x48);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3), (779, 4)]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    t.set_u32(t.img.word(da + 0x0b), 0);
    t.set_u32(t.img.word(da + 0x76), 0);
    let lock = SCRATCH + 0x80;
    t.set_u32(t.img.word(r80 + 0x21), lock);
    let block = SCRATCH + 0x2000;
    t.set_u32(op(&t, f, 0x3d), block);
    let task = SCRATCH + 0x100;
    for (one, wait, busy) in [
        (false, 0u32, 0u32),
        (true, 0, 0),
        (false, 1, 0),
        (false, 0, 1),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x200);
        t.set_u32(op(&t, f, 0x12), wait);
        t.set_u32(op(&t, f, 0x1b), busy);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn motion_entries() {
    for (n, rva) in [
        (357u16, 0x1280u32),
        (382, 0x11c0),
        (421, 0x1420),
        (415, 0x10a0),
        (312, 0x1400),
    ] {
        let Some(mut t) = twin(n) else { return };
        recs(&mut t, &[(447, 2), (450, 4), (449, 6)]);
        let f = t.at(rva);
        let base = SCRATCH + 0x400;
        t.set_u32(op(&t, f, 0x4), base);
        t.set_u32(base + 0x198, 0x0198_0447);
        let obj = SCRATCH + 0x100;
        for k in [0u32, 2] {
            t.write(obj, &[0x44; 0x100]);
            t.set_u32(obj + 0xac, k);
            t.check_entry(f, Call::default(), &[obj], Eax::Checked);
        }
    }
}

#[test]
fn more_links() {
    for (n, rva, args, port) in [
        (
            106u16,
            0x6b10u32,
            &[0x0001_0038][..],
            s4_small::link_node_38_one as Port,
        ),
        (273, 0x76d0, &[0x0001_001c][..], s4_small::link_node_x_ff),
        (609, 0x8dc0, &[][..], s4_small::link_node_2c),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let (last_at, parent) = match n {
            106 => (op(&t, f, 0x26), op(&t, f, 0x2b)),
            273 => (op(&t, f, 0x2c), op(&t, f, 0x31)),
            _ => (op(&t, f, 0x23), op(&t, f, 0x28)),
        };
        let node = SCRATCH + 0x100;
        for empty in [true, false] {
            t.write(node, &[0x66; 0x40]);
            t.write(SCRATCH + 0x300, &[0x77; 0x10]);
            t.set_u32(last_at, if empty { parent } else { SCRATCH + 0x300 });
            t.check(f, Call::this(node), args, Eax::Checked, port);
        }
    }
}

#[test]
fn starts_and_pairs() {
    let Some(mut t) = twin(376) else { return };
    recs(&mut t, &[(719, 3)]);
    let f = t.at(0x1ca0);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x19), recs_at);
    t.write(op(&t, f, 0x21), &[0x99; 0x5c]);
    t.write(recs_at, &[0x99; 0x3a00]);
    t.set_u32(op(&t, f, 0x11), 0x0719_0001);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(530) else { return };
    let f = t.at(0x10a0);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 415, 1, move |m, _| m.r32(answer));
    recs(&mut t, &[(228, 2)]);
    t.set_u32(op(&t, f, 0x45), 0x0228_0228);
    for m in [0xb1u32, 0xb2, 0xb9, 0xba, 0xb0, 0xbb, 0] {
        t.set_u32(answer, m);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let obj = SCRATCH + 0x100;
    let Some(mut t) = twin(384) else { return };
    recs(&mut t, &[(450, 2), (451, 4)]);
    let f = t.at(0x2000);
    for n in [4u32, 7, 1, 0, u32::MAX - 6] {
        t.set_u32(op(&t, f, 0xf), n);
        t.write(obj, &[0x55; 0x200]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(415) else { return };
    let f = t.at(0x1140);
    let answer_rec = SCRATCH + 0x600;
    rec(&mut t, 434, 2, move |_, _| answer_rec);
    recs(&mut t, &[(461, 6)]);
    t.set_u32(op(&t, f, 0xf), 0x0434_0001);
    t.set_u32(op(&t, f, 0x27), 0x0434_0002);
    t.set_u32(op(&t, f, 0x3b), 0x0e0e_003b);
    for k in [0u32, 1] {
        t.write(answer_rec, &[0x22; 0x100]);
        t.set_u32(answer_rec + 0xac, k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(430) else { return };
    recs(&mut t, &[(357, 3), (356, 6), (355, 8), (354, 10)]);
    let f = t.at(0x3880);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x1), p);
    t.set_u8(p + 2, 2);
    t.set_u32(op(&t, f, 0x12) + 8, 0x0354_0002);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(356) else { return };
    recs(&mut t, &[(447, 2), (434, 4), (444, 5), (461, 7)]);
    let f = t.at(0x10f0);
    let base = SCRATCH + 0x400;
    t.set_u32(op(&t, f, 0x4), base);
    t.set_u32(base + 0x198, 0x0198_0001);
    t.set_u32(op(&t, f, 0x2c), 0x0434_0003);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(312) else { return };
    recs(&mut t, &[(450, 2), (451, 4)]);
    let f = t.at(0x1740);
    for s in [1.0f32, -2.5, 0.0, f32::NAN] {
        t.set_u32(op(&t, f, 0x7), s.to_bits());
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn walks_and_registers() {
    let Some(mut t) = twin(70) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(275, 1)]);
    let f = t.at(0x5000);
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        t.check(
            f,
            Call::this(sa::CURSOR),
            &[],
            Eax::Checked,
            s4_small::walk_275,
        );
    }
    drop(t);
    let Some(mut t) = twin(515) else { return };
    let f = t.at(0x49d0);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    recs(&mut t, &[(779, 4), (941, 1)]);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x2d), lock);
    t.set_u32(lock, 3);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(388) else { return };
    let f = t.at(0x6b40);
    let (task, obj, node) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x200);
    for (busy, flag) in [(0u32, 4u8), (1, 4), (0, 0)] {
        t.write(obj, &[0; 0x300]);
        t.set_u32(obj + 0x18, busy);
        t.set_u8(obj + 0x2a8, flag);
        for (i, off) in [0x168u32, 0x194, 0x1c].into_iter().enumerate() {
            let parent = SCRATCH + 0x3000 + 0x100 * i as u32;
            t.write(parent, &[0; 0x20]);
            t.set_u16(parent + 0x18, 1);
            t.set_u32(parent + 0x10, obj + off);
            t.set_u32(obj + off + 0x14, if i == 1 { 0 } else { parent });
            t.set_u32(obj + off + 4, parent);
            t.set_u32(obj + off + 8, parent);
        }
        t.set_u32(task, node);
        t.set_u32(task + 4, obj);
        t.set_u32(node + 4, SCRATCH + 0x280);
        t.set_u32(node + 8, SCRATCH + 0x2c0);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(247) else { return };
    let f = t.at(0x17a0);
    let ro = t.img.rel32_target(f + 0x5);
    let ms = t.img.rel32_target(f + 0x3a);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3), (239, 3)]);
    t.set_u32(op(&t, f, 0x2a), 0x0239_0001);
    let task = SCRATCH + 0x100;
    for (once, one, done) in [
        (0u32, false, 1u32),
        (1, false, 1),
        (0, true, 1),
        (0, false, 0),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x200);
        t.set_u32(op(&t, f, 0x12), once);
        t.set_u32(t.img.word(ms + 0x02), done);
        t.set_u16(t.img.word(ms + 0x11), 5);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn links_grey_and_blocks() {
    for (n, rva, args, port) in [
        (
            42u16,
            0x9420u32,
            &[0x0001_001c, 0x0001_0020][..],
            s4_small::link_node_grey as Port,
        ),
        (577, 0x8f10, &[][..], s4_small::link_node_blocks),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let (last_at, parent) = if n == 42 {
            (op(&t, f, 0x26), op(&t, f, 0x2b))
        } else {
            (op(&t, f, 0x29), op(&t, f, 0x2e))
        };
        let node = SCRATCH + 0x100;
        for empty in [true, false] {
            t.write(node, &[0x66; 0x80]);
            t.write(SCRATCH + 0x300, &[0x77; 0x10]);
            t.set_u32(last_at, if empty { parent } else { SCRATCH + 0x300 });
            t.check(f, Call::this(node), args, Eax::Checked, port);
        }
    }
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x9500);
    let last = SCRATCH + 0x300;
    t.write(last, &[0x77; 0x10]);
    t.set_u32(op(&t, f, 0x1), last);
    t.set_u32(op(&t, f, 0x3b), 0xffff_ffff);
    let c = Call {
        edx: 0x0edd_0001,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, s4_small::static_node_link);
}

#[test]
fn states_models_vectors() {
    let Some(mut t) = twin(511) else { return };
    recs(&mut t, &[(4, 1), (27, 3)]);
    t.check_entry(t.at(0x21d0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(262) else { return };
    recs(&mut t, &[(217, 2), (248, 5)]);
    let f = t.at(0x6b60);
    let (obj, v) = (SCRATCH + 0x100, SCRATCH + 0x300);
    for flags in [0u16, 1, 0xfffe] {
        t.write(obj, &[0x11; 0x100]);
        t.set_u16(obj + 0x90, flags);
        t.write(v, &[0x22; 0x10]);
        let c = Call {
            eax: 0x00ea_0000,
            ecx: obj,
            edx: v,
            ..Call::default()
        }
        .with_xmm(&[Xmm::F32(0), Xmm::F32(0), Xmm::F32(3.25f32.to_bits())]);
        t.check(f, c, &[], Eax::Checked, s4_small::model_place);
    }
    drop(t);
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x93e0);
    let v = SCRATCH + 0x300;
    for i in 0..4 {
        t.set_u32(v + 4 * i, (1.5f32 + i as f32).to_bits());
    }
    let c = Call {
        eax: 0x00ea_0000,
        edx: v,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, s4_small::two_vectors);
    drop(t);
    let Some(mut t) = twin(180) else { return };
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x2ad0);
    let s = SCRATCH + 0x100;
    t.write(s, &[0x33; 0x300]);
    t.check(f, Call::this(s), &[], Eax::Checked, s4_small::state_reset);
}

/// A free ring of three pool nodes for the take at `take`.
fn pool_ring(t: &mut Twin, take: u32) {
    let node = |i: u32| SCRATCH + 0x4000 + 0x40 * i;
    for i in 1..=3u32 {
        t.write(node(i), &[0x5a; 0x1c]);
        t.set_u32(node(i), node(if i == 1 { 3 } else { i - 1 }));
        t.set_u32(node(i) + 4, node(if i == 3 { 1 } else { i + 1 }));
    }
    t.set_u32(t.img.word(take + 0x17), node(1));
    t.set_u16(t.img.word(take + 0x04), 3);
}

#[test]
fn tasks_from_the_pool() {
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x32b0);
    let take = t.img.rel32_target(f + 0x17);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    for listed in [false, true] {
        pool_ring(&mut t, take);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x1d), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x31), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x1d), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    for (n, rva) in [(249u16, 0x2fc0u32), (270, 0x41e0), (419, 0x2900)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x24);
        for listed in [false, true] {
            pool_ring(&mut t, take);
            if listed {
                t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
                t.set_u32(op(&t, f, 0x2a), SCRATCH + 0x5000);
                t.set_u32(op(&t, f, 0x41), SCRATCH + 0x5000);
            } else {
                t.set_u32(op(&t, f, 0x2a), 0);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

#[test]
fn walk_thunk() {
    let Some(mut t) = twin(70) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(275, 1)]);
    let f = t.at(0x5070);
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x31);
        let call = Call {
            eax: 0xaabb_ccdd,
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(f, call, &[sa::CURSOR, 0x0bad_f00e], Eax::Checked);
    }
}

#[test]
fn statics_children_sparkles() {
    let Some(mut t) = twin(152) else { return };
    let f = t.at(0x99b0);
    let last = SCRATCH + 0x300;
    t.write(last, &[0x77; 0x10]);
    t.set_u32(op(&t, f, 0x1), last);
    t.set_u32(op(&t, f, 0x3b), 0x0000_0003);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(150) else { return };
    let f = t.at(0xcc70);
    let (node, parent, kid) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for (p, kids) in [(0u32, false), (parent, false), (parent, true)] {
        t.write(node, &[0x66; 0x60]);
        t.write(parent, &[0; 0x20]);
        t.write(kid, &[0x77; 0x10]);
        let last = if kids { kid } else { parent };
        t.set_u32(parent + 0x10, last);
        t.set_u32(parent + 0xc, last);
        t.set_u16(parent + 0x18, u16::from(kids));
        let c = Call {
            ecx: node,
            edx: p,
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x0001_0001, 0x0001_0030],
            Eax::Checked,
            s4_small::child_node,
        );
    }
    drop(t);
    let Some(mut t) = twin(446) else { return };
    recs(&mut t, &[(219, 2), (257, 4)]);
    let f = t.at(0x3ad0);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x3), block);
    t.set_u32(op(&t, f, 0x21), 0x0219_0001);
    for on in [0u32, 1] {
        t.write(block + 0xd740, &[0x44; 0x14]);
        t.set_u32(block + 0xd748, on);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(430) else { return };
    recs(&mut t, &[(0, 1), (26, 3), (10, 5), (257, 7)]);
    let f = t.at(0x2210);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x3e), p);
    t.set_u8(p + 2, 1);
    t.set_u32(op(&t, f, 0x55) + 4, 0x0257_0001);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn moves_and_static_leave() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x9310);
    let m = SCRATCH + 0x100;
    for i in 0..0x1c {
        t.set_u32(m + 4 * i, (0.5f32 * i as f32 - 3.0).to_bits());
    }
    t.set_u32(m + 0x68, 0xffff_fff7);
    let c = Call {
        eax: 0x00ea_0000,
        ecx: m,
        ..Call::default()
    }
    .with_xmm(&[
        Xmm::F32(0),
        Xmm::F32(0),
        Xmm::F32(1.25f32.to_bits()),
        Xmm::F32(f32::NAN.to_bits()),
    ]);
    t.check(f, c, &[], Eax::Checked, s4_small::move_setup);
    let f = t.at(0x96e0);
    recs(&mut t, &[(846, 1)]);
    let parent = SCRATCH + 0x400;
    let (a, b) = (SCRATCH + 0x500, SCRATCH + 0x600);
    for (p, prev, next) in [
        (0u32, a, b),
        (parent, a, b),
        (parent, parent, b),
        (parent, a, parent),
    ] {
        t.write(parent, &[0x11; 0x20]);
        t.write(a, &[0x22; 0x10]);
        t.write(b, &[0x33; 0x10]);
        t.set_u32(op(&t, f, 0x2), p);
        t.set_u32(op(&t, f, 0x15), prev);
        t.set_u32(op(&t, f, 0x1a), next);
        t.check_entry(f, Call::default(), &[0x0846_0001], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(42) else { return };
    let f = t.at(0x82f0);
    let node = SCRATCH + 0x100;
    for empty in [true, false] {
        t.write(node, &[0x66; 0x40]);
        t.write(SCRATCH + 0x300, &[0x77; 0x10]);
        let parent = op(&t, f, 0x31);
        t.set_u32(
            op(&t, f, 0x2c),
            if empty { parent } else { SCRATCH + 0x300 },
        );
        t.check(
            f,
            Call::this(node),
            &[0xa, 0xb, 0xc, 0xd],
            Eax::Checked,
            s4_small::link_node_ad,
        );
    }
}

#[test]
fn tracks_ranges_colours() {
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x8060);
    let (c, owner, rec, track, out) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x1000,
        SCRATCH + 0x400,
    );
    let answer = SCRATCH + 0x90;
    rec_941(&mut t, answer);
    for i in 0..0x40 {
        t.set_u32(track + 0x40 + 4 * i, 0x0100_0000 + i);
    }
    t.set_u32(track + 0x18, 4.0f32.to_bits());
    t.set_u32(owner + 0x40, rec);
    t.set_u32(rec + 0x20, 0.5f32.to_bits());
    t.set_u32(owner + 0x30, 0x0941_0001);
    for (time, there) in [
        (0.0f32, true),
        (1.2, true),
        (2.0, true),
        (9.0, true),
        (f32::NAN, true),
        (1.0, false),
    ] {
        t.write(c, &[0; 0x10]);
        t.set_u32(c, owner);
        t.set_u32(c + 4, out);
        t.set_u32(c + 8, time.to_bits());
        t.set_u32(answer, if there { track } else { 0 });
        t.check(f, Call::this(c), &[], Eax::Checked, s4_small::track_key);
    }
    drop(t);
    let Some(mut t) = twin(348) else { return };
    recs(&mut t, &[(357, 3), (356, 6), (362, 10)]);
    let f = t.at(0x6790);
    t.set_u32(op(&t, f, 0x8), 0x0357_0001);
    t.set_u32(op(&t, f, 0x43), 0x0362_0001);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(390) else { return };
    recs(&mut t, &[(933, 4), (785, 1)]);
    let f = t.at(0x1530);
    t.set_u32(op(&t, f, 0x38), 0.75f32.to_bits());
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

/// Slot 941 answering the word at `answer`.
fn rec_941(t: &mut Twin, answer: u32) {
    rec(t, 941, 1, move |m, _| m.r32(answer));
}

#[test]
fn statics_strips_faders() {
    let Some(mut t) = twin(374) else { return };
    let f = t.at(0x7d30);
    let last = SCRATCH + 0x300;
    t.write(last, &[0x77; 0x10]);
    t.set_u32(op(&t, f, 0x1), last);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(513) else { return };
    let f = t.at(0x6a10);
    let (parent, kid) = (SCRATCH + 0x200, SCRATCH + 0x300);
    for (p, kids) in [(0u32, false), (parent, false), (parent, true)] {
        t.write(parent, &[0; 0x20]);
        t.write(kid, &[0x77; 0x10]);
        let l = if kids { kid } else { parent };
        t.set_u32(parent + 0x10, l);
        t.set_u32(parent + 0xc, l);
        let c = Call {
            eax: 0x00ea_0000,
            edx: p,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::static_child);
    }
    drop(t);
    let Some(mut t) = twin(150) else { return };
    let f = t.at(0x6f40);
    let (c, p) = (SCRATCH + 0x100, SCRATCH + 0x200);
    for (len, stride, window, from) in [
        (10u16, 4u16, 3u16, 2u16),
        (10, 4, 3, 7),
        (10, 4, 3, 8),
        (0x8000, 2, 0x10, 0x7ff8),
        (0, 1, 0, 0),
    ] {
        t.write(c, &[0x55; 0x10]);
        t.set_u32(c, p);
        t.write(p, &[0; 0x30]);
        t.set_u32(p + 0x1c, 0x0d00_0000);
        t.set_u16(p + 0x20, len);
        t.set_u16(p + 0x22, stride);
        t.set_u16(p + 0x26, window);
        t.set_u16(p + 0x28, from);
        t.check(f, Call::this(c), &[], Eax::Checked, s4_small::strip_window);
    }
    drop(t);
    for (n, rva) in [(168u16, 0x7680u32), (162, 0x7c90), (392, 0x6580)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x31);
        let s = SCRATCH + 0x100;
        for listed in [false, true] {
            pool_ring(&mut t, take);
            t.write(s, &[0x44; 0x40]);
            if listed {
                t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
                t.set_u32(op(&t, f, 0x37), SCRATCH + 0x5000);
                t.set_u32(op(&t, f, 0x4f), SCRATCH + 0x5000);
            } else {
                t.set_u32(op(&t, f, 0x37), 0);
            }
            t.check(f, Call::this(s), &[], Eax::Checked, s4_small::fader_task);
        }
    }
    for (n, rva) in [(206u16, 0x2cb0u32), (371, 0x2a00)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let recs_at = SCRATCH + 0x1_0000;
        t.set_u32(op(&t, f, 0x7), recs_at);
        t.write(op(&t, f, 0x11), &[0x99; 0x5c]);
        t.write(recs_at, &vec![0x99; 0x94 * 1000]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn colour_tasks_and_fades() {
    let Some(mut t) = twin(53) else { return };
    let f = t.at(0x2af0);
    let take = t.img.rel32_target(f + 0x2c);
    for listed in [false, true] {
        pool_ring(&mut t, take);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x32), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x49), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x32), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(168) else { return };
    recs(&mut t, &[(332, 1)]);
    let f = t.at(0x1e60);
    let s = SCRATCH + 0x100;
    for (k, n) in [
        (5u32, 8u32),
        (1, 8),
        (0, 8),
        (0xffff_fffe, 3),
        (3, 0x7fff_ffff),
    ] {
        t.write(s, &[0x55; 0x40]);
        for i in 0..3 {
            t.set_u32(s + 0x10 + 4 * i, 0x40 + 0x30 * i);
            t.set_u32(s + 0x20 + 4 * i, 0xffff_ff00 + 0x11 * i);
        }
        t.set_u32(s + 0x30, n);
        t.set_u32(s + 0x34, k);
        t.check(f, Call::this(s), &[], Eax::Checked, s4_small::int_fade);
    }
    drop(t);
    let Some(mut t) = twin(146) else { return };
    recs(&mut t, &[(942, 1)]);
    let f = t.at(0x6ca0);
    let node = SCRATCH + 0x100;
    for empty in [true, false] {
        t.write(node, &[0x66; 0x50]);
        t.write(SCRATCH + 0x300, &[0x77; 0x10]);
        let parent = op(&t, f, 0x34);
        t.set_u32(
            op(&t, f, 0x2f),
            if empty { parent } else { SCRATCH + 0x300 },
        );
        t.check(
            f,
            Call::this(node),
            &[0x003c_0001],
            Eax::Checked,
            s4_small::link_node_942,
        );
    }
}

#[test]
fn statics_and_lights() {
    let Some(mut t) = twin(164) else { return };
    for s in [538, 213, 211] {
        rec(&mut t, s, 2, move |_, a| s << 16 | a[1] & 0xffff);
    }
    rec(&mut t, 547, 1, |_, a| 0x5470_0000 ^ a[0]);
    recs(&mut t, &[(35, 5), (34, 2), (276, 2), (219, 2)]);
    let f = t.at(0xbfc0);
    let parent = SCRATCH + 0x400;
    let (a, b) = (SCRATCH + 0x500, SCRATCH + 0x600);
    let node = op(&t, f, 0x69);
    for (p, prev, next) in [(0u32, a, b), (parent, a, b), (parent, parent, parent)] {
        t.write(parent, &[0x11; 0x20]);
        t.write(a, &[0x22; 0x10]);
        t.write(b, &[0x33; 0x10]);
        t.write(node, &[0x3c; 0x98]);
        t.set_u16(node + 0x90, 0x11);
        t.set_u32(op(&t, f, 0x6), p);
        t.set_u32(op(&t, f, 0x19), prev);
        t.set_u32(op(&t, f, 0x1e), next);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(201) else { return };
    let f = t.at(0x6e00);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x46) } else { last });
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(188) else { return };
    recs(&mut t, &[(363, 3), (362, 4)]);
    let f = t.at(0x11a0);
    let p = SCRATCH + 0x80;
    t.set_u32(p, 0x0362_0004);
    t.set_u32(op(&t, f, 0x24), p);
    t.set_u32(op(&t, f, 0x2a), 0x0362_0001);
    t.set_u32(op(&t, f, 0x11), 0x0363_0001);
    for set in [0u32, 5] {
        t.set_u32(op(&t, f, 0x1c), set);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn defaults_rings_links() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x3ab0);
    t.write(op(&t, f, 0x8), &[0x5a; 0x110]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(150) else { return };
    let f = t.at(0xaa80);
    let (w, r, e) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for i in 0..4 {
        t.set_u32(e + 4 * i, 0x0e00_0000 + i);
    }
    for (size, wi, fill, ri) in [
        (4u16, 1u16, 2u16, 0u16),
        (4, 3, 4, 1),
        (4, 1, 4, 3),
        (1, 0, 0, 0),
        (0x8000, 0x7ffe, 0x8000, 2),
    ] {
        t.write(r, &[0; 0x30]);
        t.set_u32(w, r);
        t.set_u32(w + 4, e);
        t.set_u32(r + 0x1c, SCRATCH + 0x1000);
        t.set_u16(r + 0x20, size);
        t.set_u16(r + 0x22, 0x10);
        t.set_u16(r + 0x24, wi);
        t.set_u16(r + 0x26, fill);
        t.set_u16(r + 0x2a, ri);
        t.set_u8(r + 0x28, if size == 4 { 0xf7 } else { 0xf6 });
        t.check(f, Call::this(w), &[], Eax::Checked, s4_small::ring_write);
    }
    drop(t);
    let Some(mut t) = twin(262) else { return };
    let f = t.at(0x6f50);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x3c) } else { last });
        let c = Call::default().with_xmm(&[Xmm::F32(0), Xmm::F32(0), Xmm::F32(0.5f32.to_bits())]);
        t.check(f, c, &[], Eax::Checked, s4_small::static_link_xmm);
    }
}

#[test]
fn actors_faces_statics() {
    let Some(mut t) = twin(393) else { return };
    rec(&mut t, 217, 2, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        0x217
    });
    recs(&mut t, &[(255, 4), (211, 2)]);
    let f = t.at(0x2b70);
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x18), p);
    for i in 0..0x21 {
        t.set_u32(op(&t, f, 0x25) + 4 * i, 0x0211_0000 + i);
    }
    for (done, k) in [(0u32, 3u8), (0, 0x20), (1, 3)] {
        t.set_u32(op(&t, f, 0x12), done);
        t.set_u8(p + 1, k);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(201) else { return };
    rec(&mut t, 217, 2, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        m.w32(a[1] + 8, 0xbf80_0000);
        0x217
    });
    recs(&mut t, &[(234, 3), (666, 2)]);
    let f = t.at(0x6cb0);
    let obj = SCRATCH + 0x100;
    for flags in [0u8, 1] {
        t.write(obj, &[0; 0x100]);
        t.set_u32(obj + 0x50, 0x0050_0001);
        t.set_u8(obj + 0x90, flags);
        let c = Call {
            ecx: obj,
            edx: SCRATCH + 0x400,
            ..Call::default()
        };
        let eax = if flags == 0 { Eax::Checked } else { Eax::Junk };
        t.check(f, c, &[], eax, s4_small::face_point);
    }
    drop(t);
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x8ff0);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x3c) } else { last });
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn places_tables_links() {
    let Some(mut t) = twin(349) else { return };
    recs(&mut t, &[(237, 4)]);
    let f = t.at(0x4e10);
    let block = SCRATCH + 0x2000;
    t.set_u32(op(&t, f, 0x6), block);
    t.set_u32(block + 0x1a24, 2.5f32.to_bits());
    let map = SCRATCH + 0x300;
    t.write(map, &[2, 1, 0, 3]);
    t.set_u32(op(&t, f, 0x28), map);
    t.set_u32(op(&t, f, 0x21), 0x0e0e_0021);
    for k in 0..4 {
        t.set_u32(op(&t, f, 0x33) + 4 * k, 0x0237_0000 + k);
    }
    let obj = SCRATCH + 0x100;
    for i in [u32::MAX, 0, 3] {
        t.write(obj, &[0x22; 0x130]);
        t.set_u32(obj + 0x120, i);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x91c0);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x46) } else { last });
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(638) else { return };
    let f = t.at(0x31e0);
    let so = t.img.rel32_target(f + 0x2f);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    let pa = t.img.rel32_target(so + 0x96);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    let base = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x18), base);
    for i in [0u32, 1] {
        t.write(base + 0x130 * i, &[0x5a; 0x130]);
        t.set_u16(free, 3);
        t.check(f, Call::this(i), &[], Eax::Checked, s4_small::table_object);
    }
}

#[test]
fn actors_leaves_waits() {
    let Some(mut t) = twin(165) else { return };
    recs(&mut t, &[(530, 2), (249, 2)]);
    let f = t.at(0x3670);
    let map = SCRATCH + 0x300;
    t.write(map, &[3, 0x20, 0x21, 1]);
    t.set_u32(op(&t, f, 0x12), map);
    for k in 0..0x22 {
        t.set_u32(op(&t, f, 0x5c) + 4 * k, 0x0249_0000 + k);
    }
    for (n, done) in [(0u32, 0u32), (4, 0), (4, 1)] {
        t.set_u32(op(&t, f, 0x5), n);
        t.set_u32(op(&t, f, 0x1b), done);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(70) else { return };
    rec(&mut t, 771, 1, |_, a| 0x0771_0000 ^ a[0]);
    t.stub_edx(771, |_, a| 0x0edd_0000 ^ a[0]);
    recs(&mut t, &[(779, 2)]);
    let f = t.at(0x8850);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x54), lock);
    let (cur, node, parent) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    for p in [0u32, parent] {
        t.write(node, &[0; 0x20]);
        t.write(parent, &[0; 0x20]);
        t.set_u16(parent + 0x18, 1);
        t.set_u32(parent + 0x10, node);
        t.set_u32(parent + 0xc, node);
        t.set_u32(node + 4, parent);
        t.set_u32(node + 8, parent);
        t.set_u32(node + 0x14, p);
        t.set_u32(cur, node);
        t.set_u32(lock, 6);
        t.check(
            f,
            Call::this(cur),
            &[],
            Eax::Checked,
            s4_small::leave_register,
        );
    }
    drop(t);
    let Some(mut t) = twin(166) else { return };
    let f = t.at(0x14f0);
    let ms = t.img.rel32_target(f + 0x53);
    let ro = t.img.rel32_target(f + 0x5a);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3), (296, 2), (257, 2)]);
    let task = SCRATCH + 0x100;
    for (w, done, n, mdone) in [
        (0u32, 0u32, 3u32, 1u32),
        (2, 0, 3, 1),
        (1, 0, 3, 1),
        (1, 0, 0x20, 0),
        (1, 1, 3, 1),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x200);
        t.set_u32(op(&t, f, 0x4), w);
        t.set_u32(op(&t, f, 0x18), done);
        t.set_u32(op(&t, f, 0x20), n);
        t.set_u32(t.img.word(ms + 0x02), mdone);
        t.set_u16(t.img.word(ms + 0x11), 5);
        objects(&mut t, ro, false);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn camera_points() {
    let Some(mut t) = twin(563) else { return };
    rec(&mut t, 327, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f80_0000 + i);
        }
        0x327
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x4000_0000 + i);
        }
        0x229
    });
    for rva in [0x56f0u32, 0x5770] {
        let f = t.at(rva);
        t.set_u32(op(&t, f, 0x13), SCRATCH + 0x1000);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn loads_and_buffers() {
    let Some(mut t) = twin(341) else { return };
    let f = t.at(0x1370);
    let answers = SCRATCH + 0x80;
    rec(&mut t, 225, 1, move |m, _| m.r32(answers));
    rec(&mut t, 227, 1, move |m, _| m.r32(answers + 4));
    recs(&mut t, &[(226, 2)]);
    t.set_u32(op(&t, f, 0x10), 0x0e5e_0001);
    for (a, busy, asked, b, there) in [
        (0u32, 1u32, 0u32, 0u32, 0u32),
        (0, 0, 0, 0, 0),
        (1, 0, 1, 0, 1),
        (1, 0, 1, 0, 0xffff_ffff),
        (1, 0, 1, 2, 1),
    ] {
        t.set_u32(op(&t, f, 0x2), a);
        t.set_u32(op(&t, f, 0x2b), asked);
        t.set_u32(op(&t, f, 0x56), b);
        t.set_u32(answers, busy);
        t.set_u32(answers + 4, there);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    for (n, rva) in [(149u16, 0x1cb0u32), (126, 0x2100)] {
        let Some(mut t) = twin(n) else { return };
        recs(&mut t, &[(711, 3), (710, 7)]);
        let f = t.at(rva);
        let lock = SCRATCH + 0x80;
        t.set_u32(op(&t, f, 0x1), lock);
        t.set_u32(lock, 4);
        t.set_u32(op(&t, f, 0x7), 0x0711_0001);
        t.set_u32(op(&t, f, 0x11), 0x0710_0003);
        for o in [0x5a, 0x65, 0x6f, 0x79] {
            t.set_u32(op(&t, f, o), 0x5555);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn drops_and_walk_resets() {
    let Some(mut t) = twin(243) else { return };
    recs(&mut t, &[(711, 3), (710, 7)]);
    let f = t.at(0x3110);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x1), lock);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(481) else { return };
    recs(&mut t, &[(258, 1), (230, 2)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x1760);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x52), lock);
    t.set_u32(op(&t, f, 0x45), SCRATCH + 0x88);
    let part = SCRATCH + 0x100;
    for a in [0u32, 1] {
        t.write(part, &[0x44; 0x20]);
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check(f, Call::this(part), &[], Eax::Checked, s4_small::part_drop);
    }
    drop(t);
    let Some(mut t) = twin(201) else { return };
    sa::handler_stubs(&mut t);
    for s in [538u32, 213, 211, 35, 34, 547, 276, 219, 275] {
        rec(&mut t, s, 1, move |_, _| s);
    }
    let f = t.at(0x2fe0);
    let (owner2, parent) = (SCRATCH + 0x4000, SCRATCH + 0x6000);
    for (k, &(c, p, cp)) in sa::RINGS.iter().enumerate() {
        for bit in [0u8, 1] {
            sa::ring(&mut t, c, p, cp, 0x30 + k as u8);
            let owner = sa::node(0);
            t.set_u8(owner + 0x24, bit);
            t.write(owner2, &[0; 0x500]);
            t.set_u32(owner2 + 0x4dc, 1);
            t.write(parent, &[0x45; 0x40]);
            t.set_u16(parent + 0x18, 2);
            let n0 = owner2 + 0x1c;
            t.set_u32(n0 + 0x14, parent);
            t.set_u32(n0 + 4, parent);
            t.set_u32(n0 + 8, parent);
            t.set_u32(n0 + 0x64, u32::MAX);
            t.set_u32(owner + 0x1c, owner2);
            t.set_u32(owner2 + 0x14, parent);
            t.set_u32(owner2 + 4, parent);
            t.set_u32(owner2 + 8, parent);
            t.set_u32(owner + 0x14, 0);
            t.check(
                f,
                Call::this(sa::CURSOR),
                &[],
                Eax::Checked,
                s4_small::walk_reset,
            );
        }
    }
}

#[test]
fn thunks_lights_rings_records() {
    let Some(mut t) = twin(188) else { return };
    recs(&mut t, &[(363, 3), (362, 4)]);
    let f = t.at(0x1d90);
    let li = t.img.rel32_target(f + 0x1);
    t.set_u32(op(&t, li, 0x24), SCRATCH + 0x80);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(150) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0xa9f0);
    let ring = SCRATCH + 0x5000;
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        let owner = sa::node(0);
        t.write(ring, &[0x41; 0x40]);
        t.set_u32(ring, sa::VTABLE);
        t.set_u32(owner + 0x2c, ring);
        t.set_u32(owner + 0x1c, SCRATCH + 0x6000);
        t.set_u16(owner + 0x20, 4);
        t.set_u16(owner + 0x22, 0x10);
        t.set_u16(owner + 0x24, 1);
        t.set_u16(owner + 0x26, 1);
        t.set_u16(owner + 0x2a, 0);
        t.check(
            f,
            Call::this(sa::CURSOR),
            &[],
            Eax::Checked,
            s4_small::walk_ring,
        );
    }
    drop(t);
    let Some(mut t) = twin(206) else { return };
    let f = t.at(0x2eb0);
    let hdr = SCRATCH + 0x400;
    t.write(hdr, &[0x99; 0x20]);
    rec(&mut t, 942, 1, |_, a| 0x9420_0000 ^ a[0]);
    rec(&mut t, 941, 1, move |_, _| hdr);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xb), recs_at);
    for (n, used) in [(0u32, 0u32), (3, 3), (3, 1)] {
        t.write(recs_at, &[0x22; 0x94 * 3]);
        for i in 0..3 {
            t.set_u32(recs_at + 0x94 * i, u32::from(i < used));
        }
        t.set_u32(op(&t, f, 0x2), n);
        let c = Call {
            edx: 0x00a1_0000,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::record_take);
    }
    drop(t);
    let Some(mut t) = twin(638) else { return };
    let f = t.at(0x3260);
    let so = t.img.rel32_target(t.at(0x31e0) + 0x2f);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
    t.stub(941, |_, _| SCRATCH + 0x400);
    t.stub(714, |_, _| 0x714);
    let to = t.at(0x31e0);
    let base = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, to, 0x18), base);
    let arg = SCRATCH + 0x100;
    t.set_u32(arg + 0x2c, 1);
    t.write(base + 0x130, &[0x5a; 0x130]);
    t.check_entry(f, Call::default(), &[arg, 0x0bad_f00e], Eax::Checked);
}

#[test]
fn walk_reset_thunk() {
    let Some(mut t) = twin(201) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(275, 1)]);
    let f = t.at(0x3070);
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        t.set_u8(sa::node(0) + 0x24, 0);
        let call = Call {
            eax: 0xaabb_ccdd,
            ecx: 0x0bad_0ecc,
            ..Call::default()
        };
        t.check_entry(f, call, &[sa::CURSOR, 0x0bad_f00e], Eax::Checked);
    }
}

#[test]
fn links_845_and_handles() {
    let Some(mut t) = twin(70) else { return };
    recs(&mut t, &[(845, 1)]);
    let f = t.at(0x9650);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x46) } else { last });
        t.check_entry(f, Call::default(), &[0x0845_0001], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(183) else { return };
    recs(&mut t, &[(718, 1), (222, 4), (221, 4), (719, 3)]);
    t.check_entry(t.at(0x1000), Call::default(), &[], Eax::Checked);
}

#[test]
fn handles_tracks_clocks() {
    let Some(mut t) = twin(162) else { return };
    recs(&mut t, &[(0, 1), (26, 3), (10, 5)]);
    t.check_entry(t.at(0x2db0), Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(155) else { return };
    sa::handler_stubs(&mut t);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 941, 1, move |m, _| m.r32(answer));
    let f = t.at(0x8350);
    let (track, rec_at) = (SCRATCH + 0x6000, SCRATCH + 0x5000);
    for i in 0..0x40 {
        t.set_u32(track + 0x40 + 4 * i, 0x0100_0000 + i);
    }
    t.set_u32(track + 0x18, 4.0f32.to_bits());
    t.set_u32(answer, track);
    for &(c, p, cp) in sa::RINGS {
        for (tr, time) in [
            (0u32, 0.0f32),
            (SCRATCH + 0x5800, 0.6),
            (SCRATCH + 0x5800, 9.0),
        ] {
            sa::ring(&mut t, c, p, cp, 0x30);
            let owner = sa::node(0);
            t.write(SCRATCH + 0x5800, &[0; 0x60]);
            t.set_u32(SCRATCH + 0x5800 + 0x40, rec_at);
            t.set_u32(rec_at + 0x20, 0.5f32.to_bits());
            t.set_u32(owner + 0x2c, tr);
            t.set_u32(owner + 0x30, time.to_bits());
            t.check(
                f,
                Call::this(sa::CURSOR),
                &[],
                Eax::Checked,
                s4_small::walk_track,
            );
        }
    }
    drop(t);
    let Some(mut t) = twin(412) else { return };
    let f = t.at(0x69d0);
    let (task, obj, node) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x200);
    for (busy, flag) in [(0u32, 4u8), (1, 4), (0, 0)] {
        t.write(obj, &[0; 0x300]);
        t.set_u32(obj + 0x2c4, 1.5f32.to_bits());
        t.set_u32(obj + 0x2d0, busy);
        t.set_u8(obj + 0x108, flag);
        t.set_u32(task, node);
        t.set_u32(task + 4, obj);
        t.set_u32(node + 4, SCRATCH + 0x280);
        t.set_u32(node + 8, SCRATCH + 0x2c0);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
}

#[test]
fn lists_floats_releases() {
    let Some(mut t) = twin(98) else { return };
    let f = t.at(0x2b10);
    let lists = SCRATCH + 0x300;
    t.write(lists, &[3, 0xfe, 7, 0, 0, 0, 0, 0, 5, 0x80, 9]);
    let counts = SCRATCH + 0x80;
    rec(&mut t, 288, 2, move |m, a| {
        let n = m.r32(counts + 4 * (a[0] & 2) / 2);
        m.w32(a[1], n);
        if a[0] == 9 { lists } else { lists + 8 }
    });
    for (n1, n2, id) in [
        (3u32, 3u32, 7u32),
        (3, 3, 0xffff_fffe),
        (3, 3, 9),
        (3, 3, 0xffff_ff80),
        (3, 3, 4),
        (0, 0, 3),
        (2, 2, 7),
    ] {
        t.set_u32(counts, n1);
        t.set_u32(counts + 4, n2);
        t.check(f, Call::this(id), &[], Eax::Checked, s4_small::id_list);
    }
    let f = t.at(0x1000);
    recs(&mut t, &[(786, 3)]);
    for (s, v) in [(788u32, 1.5f64), (787, -2.0), (789, 0.25), (790, 1e3)] {
        rec(&mut t, s, 0, move |_, _| s);
        t.stub_st0(s, move |_, _| v);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(148) else { return };
    let f = t.at(0x1820);
    let base = SCRATCH + 0x200;
    t.set_u32(op(&t, f, 0x5), base);
    t.set_u32(base + 0x24, 0x0024_0001);
    recs(&mut t, &[(168, 1), (170, 2), (804, 1), (817, 0)]);
    rec(&mut t, 169, 1, |_, _| 0x169);
    t.stub_st0(169, |_, _| 0.75);
    let b = SCRATCH + 0x300;
    t.write(b, &[0x55; 0x20]);
    t.set_u32(op(&t, f, 0x75), b);
    for h in [0u32, 0x0804_0001] {
        t.set_u32(op(&t, f, 0x4a), h);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn scales_drops_resets() {
    let Some(mut t) = twin(391) else { return };
    let f = t.at(0x4000);
    let map = SCRATCH + 0x300;
    t.write(map, &[2, 0x20, 1]);
    t.set_u32(op(&t, f, 0x5), map);
    t.set_u32(op(&t, f, 0x16), 5);
    t.set_u32(op(&t, f, 0x4d), 0x0e0e_0001);
    for k in 0..0x21 {
        t.set_u32(op(&t, f, 0x5a) + 4 * k, 0x0a00_0000 + k);
    }
    for s in [251u32, 19, 17] {
        rec(&mut t, s, 1, move |_, _| s);
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xff) * 0.5 + f64::from(s));
    }
    for (done, i) in [(0u32, u32::MAX), (0, 0), (0, 1), (1, 2), (1, u32::MAX)] {
        t.set_u32(op(&t, f, 0xe), done);
        t.check(
            f,
            Call::this(i),
            &[],
            Eax::Checked,
            s4_small::actor_scale_by,
        );
    }
    drop(t);
    let Some(mut t) = twin(490) else { return };
    recs(&mut t, &[(258, 1), (231, 3), (230, 2)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x1070);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x59), lock);
    t.set_u32(op(&t, f, 0x4c), SCRATCH + 0x88);
    for a in [0u32, 1] {
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(168) else { return };
    recs(&mut t, &[(287, 3), (283, 2), (281, 1), (235, 1), (329, 3)]);
    let f = t.at(0x1c30);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    for i in 0..4 {
        t.set_u32(block + 0xa90 + 4 * i, 0x3f00_0000 + i);
    }
    t.set_u32(block + 0x3150, 0x4000_0000);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn resets_and_scales() {
    for (n, rva) in [(165u16, 0x1a40u32), (511, 0x2040), (563, 0x1f40)] {
        let Some(mut t) = twin(n) else { return };
        recs(&mut t, &[(287, 3), (283, 2), (281, 1), (235, 1), (329, 3)]);
        let f = t.at(rva);
        let block = SCRATCH + 0x1000;
        t.set_u32(op(&t, f, 0x3), block);
        let (s, v) = (op(&t, f, 0x29), op(&t, f, 0x4d));
        for i in 0..4 {
            t.set_u32(block + v + 4 * i, 0x3f00_0000 + i);
        }
        t.set_u32(block + s, 0x4000_0000);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn colours_and_child_places() {
    let Some(mut t) = twin(430) else { return };
    recs(&mut t, &[(287, 3), (283, 2), (281, 1), (235, 1), (329, 3)]);
    let f = t.at(0x1e60);
    t.set_u32(op(&t, f, 0x3), SCRATCH + 0x1000);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(512) else { return };
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x75d0);
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0x11; 0x100]);
    for (i, w) in [0u16, 1, 0x80, 0xffff].into_iter().enumerate() {
        t.set_u16(obj + 0xd0 + 2 * i as u32, w);
    }
    let c = Call {
        ecx: obj,
        edx: SCRATCH + 0x400,
        ..Call::default()
    };
    t.check(f, c, &[], Eax::Checked, s4_small::colour_from_words);
    drop(t);
    let Some(mut t) = twin(155) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x4910);
    let child = SCRATCH + 0x5000;
    for &(c, p, cp) in sa::RINGS {
        for kid in [0u32, child] {
            sa::ring(&mut t, c, p, cp, 0x30);
            t.write(child, &[0x41; 0x20]);
            t.set_u32(child, sa::VTABLE);
            t.set_u32(sa::node(0) + 0x2c, kid);
            t.check(
                f,
                Call::this(sa::CURSOR),
                &[],
                Eax::Checked,
                s4_small::walk_child_place,
            );
        }
    }
}

#[test]
fn drops_frames_colours() {
    let Some(mut t) = twin(168) else { return };
    recs(&mut t, &[(933, 4), (785, 1)]);
    let f = t.at(0x4330);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x73), block);
    for node in [0u32, SCRATCH + 0x600] {
        if node != 0 {
            t.write(node, &[0; 0x20]);
            t.write(node + 0x100, &[0; 0x20]);
            t.set_u32(node + 8, node + 0x100);
        }
        t.set_u32(block + 0x314c, node);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(429) else { return };
    recs(&mut t, &[(258, 1), (231, 3)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x1170);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x63), lock);
    t.set_u32(op(&t, f, 0x53), SCRATCH + 0x88);
    for a in [0u32, 1] {
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check_entry(f, Call::default(), &[0x0258_0001], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(702) else { return };
    recs(&mut t, &[(723, 1), (724, 2), (346, 1), (214, 2)]);
    let f = t.at(0x14d0);
    for o in [0xe, 0x1f] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x80 + o);
    }
    let part = op(&t, f, 0x4d);
    for (on, alive) in [(0u32, 0u16), (1, 0), (1, 3)] {
        t.set_u32(op(&t, f, 0x2), on);
        t.write(part, &[0; 0x20]);
        t.set_u16(part + 0x1e, alive);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn loads_particles_squares() {
    let Some(mut t) = twin(98) else { return };
    let f = t.at(0x32a0);
    let answers = SCRATCH + 0x80;
    rec(&mut t, 253, 0, move |m, _| m.r32(answers + 8));
    rec(&mut t, 225, 1, move |m, _| m.r32(answers));
    rec(&mut t, 227, 1, move |m, _| m.r32(answers + 4));
    recs(&mut t, &[(226, 2)]);
    t.set_u32(op(&t, f, 0x4c), 0x0e5e_0001);
    for (ready, a, busy, asked, b, there) in [
        (0u32, 0u32, 0u32, 0u32, 0u32, 0u32),
        (1, 0, 1, 0, 0, 0),
        (1, 0, 0, 0, 0, 0),
        (1, 1, 0, 1, 0, 2),
        (1, 1, 0, 1, 0, 1),
        (1, 1, 0, 1, 2, 1),
        (1, 1, 0, 0, 3, 1),
    ] {
        t.set_u32(answers + 8, ready);
        t.set_u32(op(&t, f, 0x14), a);
        t.set_u32(op(&t, f, 0x3c), asked);
        t.set_u32(op(&t, f, 0x67), b);
        t.set_u32(answers, busy);
        t.set_u32(answers + 4, there);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x1e70);
    let table = SCRATCH + 0x400;
    t.set_u32(table + 4, 0x0700_0001);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (268, 3)]);
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    let base = SCRATCH + 0x200;
    t.set_u32(op(&t, f, 0x10), base);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(180) else { return };
    let f = t.at(0x2530);
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    recs(&mut t, &[(779, 4)]);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x53), lock);
    let recs_at = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xc), recs_at);
    for i in 0..3 {
        t.set_u32(recs_at + 0x50 * i, 0x0100_0000 + i);
    }
    for n in [0u32, 2, 3] {
        t.set_u32(op(&t, f, 0x5), n);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn particles_and_static_x() {
    let Some(mut t) = twin(662) else { return };
    let f = t.at(0x11c0);
    let table = SCRATCH + 0x400;
    t.set_u32(table, 0x0700_0001);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(609) else { return };
    let f = t.at(0x9070);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x4), if empty { op(&t, f, 0x49) } else { last });
        t.check_entry(f, Call::default(), &[0x0006_c001], Eax::Checked);
    }
}

#[test]
fn more_walk_thunks() {
    let call = Call {
        eax: 0xaabb_ccdd,
        ecx: 0x0bad_0ecc,
        ..Call::default()
    };
    let Some(mut t) = twin(155) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(325, 2)]);
    for rva in [0x83e0u32, 0x49b0] {
        let f = t.at(rva);
        for &(c, p, cp) in sa::RINGS {
            sa::ring(&mut t, c, p, cp, 0x30);
            t.set_u32(sa::node(0) + 0x2c, 0);
            t.check_entry(f, call, &[sa::CURSOR, 0x0bad_f00e], Eax::Checked);
        }
    }
    drop(t);
    let Some(mut t) = twin(150) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0xaa70);
    let ring = SCRATCH + 0x5000;
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        let owner = sa::node(0);
        t.write(ring, &[0x41; 0x40]);
        t.set_u32(ring, sa::VTABLE);
        t.set_u32(owner + 0x2c, ring);
        t.set_u32(owner + 0x1c, SCRATCH + 0x6000);
        t.set_u16(owner + 0x20, 4);
        t.set_u16(owner + 0x22, 0x10);
        t.set_u16(owner + 0x24, 1);
        t.set_u16(owner + 0x26, 1);
        t.set_u16(owner + 0x2a, 0);
        t.check_entry(f, call, &[sa::CURSOR, 0x0bad_f00e], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(98) else { return };
    recs(&mut t, &[(271, 1), (786, 3)]);
    for (s, v) in [(788u32, 1.5f64), (787, -2.0), (789, 0.25), (790, 1e3)] {
        rec(&mut t, s, 0, move |_, _| s);
        t.stub_st0(s, move |_, _| v);
    }
    t.check_entry(t.at(0x1ee0), Call::default(), &[], Eax::Checked);
}

#[test]
fn static_tail_and_ring_entries() {
    let Some(mut t) = twin(155) else { return };
    let f = t.at(0x90e0);
    let last = SCRATCH + 0x300;
    for empty in [true, false] {
        t.write(last, &[0x77; 0x10]);
        t.set_u32(op(&t, f, 0x1), if empty { op(&t, f, 0x46) } else { last });
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(42) else { return };
    sa::handler_stubs(&mut t);
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x7c50);
    let child = SCRATCH + 0x5000;
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        let owner = sa::node(0);
        t.write(child, &[0x41; 0x40]);
        t.set_u32(child, sa::VTABLE);
        t.set_u32(owner + 0x38, child);
        t.set_u32(owner + 0x1c, SCRATCH + 0x6000);
        t.set_u16(owner + 0x20, 4);
        t.set_u16(owner + 0x22, 0x10);
        t.set_u16(owner + 0x24, 1);
        t.set_u16(owner + 0x26, 1);
        t.set_u16(owner + 0x2a, 0);
        t.check_entry(f, Call::default(), &[sa::CURSOR], Eax::Checked);
    }
}

#[test]
fn particles_leaves_nodes() {
    let Some(mut t) = twin(664) else { return };
    let f = t.at(0x1640);
    let table = SCRATCH + 0x400;
    t.set_u32(table + 0x14, 0x0700_0014);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(155) else { return };
    for s in [538, 213, 211] {
        rec(&mut t, s, 2, move |_, a| s << 16 | a[1] & 0xffff);
    }
    rec(&mut t, 547, 1, |_, a| 0x5470_0000 ^ a[0]);
    recs(&mut t, &[(35, 5), (34, 2), (276, 2), (219, 2), (560, 1)]);
    rec(&mut t, 557, 0, |_, _| 0x5570);
    let f = t.at(0xc260);
    let node = op(&t, f, 0x69);
    t.write(node, &[0x3c; 0x98]);
    t.set_u16(node + 0x90, 0x11);
    t.set_u32(op(&t, f, 0x6), 0);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(70) else { return };
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x1840);
    let (r, n, parent, kid) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
    );
    for (p, kids) in [(0u32, false), (parent, false), (parent, true)] {
        t.write(n, &[0x66; 0x80]);
        t.write(parent, &[0; 0x20]);
        t.write(kid, &[0x77; 0x10]);
        let l = if kids { kid } else { parent };
        t.set_u32(parent + 0x10, l);
        t.set_u32(parent + 0xc, l);
        for (i, w) in [n, p, 0x55, 0x66, 0x0a0a_0010, 0x0b0b_0014]
            .into_iter()
            .enumerate()
        {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(
            f,
            Call::this(r),
            &[],
            Eax::Checked,
            s4_small::node_from_args,
        );
    }
}

#[test]
fn falls_and_headings() {
    let Some(mut t) = twin(515) else { return };
    let f = t.at(0x3980);
    let p = SCRATCH + 0x100;
    // the stop bound `K` (an f64 constant): just below it and at it
    let ka = op(&t, f, 0x20);
    let k = f64::from_bits(u64::from(t.u32(ka)) | (u64::from(t.u32(ka + 4)) << 32)) as f32;
    for (time, step) in [
        (k - 1.0, 0.0),
        (k, 0.0),
        (0.0f32, 0.5f32),
        (-50.0, 0.1),
        (5.0, -20.0),
        (f32::NAN, 1.0),
    ] {
        for i in 0..0x10 {
            t.set_u32(p + 4 * i, (0.25 * i as f32 - 1.0).to_bits());
        }
        t.set_u32(p + 0x34, time.to_bits());
        t.set_u32(p + 0x38, step.to_bits());
        t.check_entry(f, Call::default(), &[p], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(202) else { return };
    rec(&mut t, 217, 2, |m, a| {
        m.w32(a[1], 0x3f00_0000);
        m.w32(a[1] + 8, 0xbf80_0000);
        0x217
    });
    recs(&mut t, &[(234, 3)]);
    let f = t.at(0x2970);
    let (r, obj, out) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x400);
    for flags in [0u8, 1] {
        t.write(obj, &[0; 0x100]);
        t.set_u32(obj + 0x50, 0x0050_0001);
        t.set_u8(obj + 0x90, flags);
        t.write(out, &[0x55; 0x10]);
        t.set_u32(r, obj);
        t.set_u32(r + 4, out);
        t.set_u32(r + 8, SCRATCH + 0x500);
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::heading_to);
    }
}

#[test]
fn opu_news() {
    let Some(mut t) = twin(437) else { return };
    let f = t.at(0x1690);
    let obj = SCRATCH + 0x1_0000;
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x8000);
    rec(&mut t, 715, 7, move |_, _| obj);
    recs(&mut t, &[(718, 1), (719, 3), (720, 2), (717, 1)]);
    let (c, tt) = (SCRATCH + 0x100, SCRATCH + 0x200);
    t.set_u32(op(&t, f, 0x1b), c);
    t.set_u32(op(&t, f, 0x21), tt);
    t.set_u32(op(&t, f, 0x43), 0x0a17_0001);
    let setup = t.img.rel32_target(f + 0x98);
    t.set_u32(t.img.word(setup + 0x24), c);
    t.set_u32(t.img.word(setup + 0x33), SCRATCH + 0x300);
    for (battle, flag) in [(0u8, 0x10u8), (1, 0x10), (1, 0)] {
        t.write(obj, &vec![0x5a; 0x2_2000]);
        t.set_u8(c + 0x10, battle);
        t.set_u8(op(&t, f, 0x3b), flag);
        t.check_entry(f, Call::default(), &[0x0715_0007], Eax::Checked);
    }
}

#[test]
fn counted_frames_unlocked_drops() {
    let Some(mut t) = twin(700) else { return };
    recs(&mut t, &[(723, 1), (724, 2), (346, 1), (214, 2)]);
    let f = t.at(0x14d0);
    for o in [0x1b, 0x2c] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x80 + o);
    }
    let part = op(&t, f, 0x5a);
    for (on, n, alive) in [(0u32, 0u32, 0u16), (1, u32::MAX, 0), (1, 0, 0), (1, 5, 3)] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(op(&t, f, 0xf), n);
        t.write(part, &[0; 0x20]);
        t.set_u16(part + 0x1e, alive);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(372) else { return };
    recs(&mut t, &[(258, 1), (231, 3), (230, 2)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x10b0);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x4e), lock);
    t.set_u32(op(&t, f, 0x53), SCRATCH + 0x88);
    for a in [0u32, 1] {
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check_entry(f, Call::default(), &[0x0258_0001], Eax::Checked);
    }
}

#[test]
fn static_args_and_level_colours() {
    let Some(mut t) = twin(70) else { return };
    recs(&mut t, &[(325, 2)]);
    let f = t.at(0x9ab0);
    let parent = op(&t, f, 0x21);
    t.write(op(&t, f, 0x1a), &[0x66; 0x80]);
    t.write(parent, &[0; 0x20]);
    t.set_u32(parent + 0x10, parent);
    t.set_u32(parent + 0xc, parent);
    t.check_entry(
        f,
        Call::default(),
        &[1, 2, 3, 0x0a0a_0004, 0x0b0b_0005],
        Eax::Checked,
    );
    drop(t);
    let Some(mut t) = twin(515) else { return };
    recs(&mut t, &[(776, 1), (915, 5)]);
    let f = t.at(0x2320);
    for (level, full) in [
        (5u32, 5u32),
        (9, 5),
        (3, 8),
        (0xffff_fffd, 8),
        (1, 0x7fff_ffff),
    ] {
        let c = Call {
            ecx: level,
            edx: full,
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x1ff, 0x80, 0x7],
            Eax::Checked,
            s4_small::level_colour,
        );
    }
}

#[test]
fn particles_steps_points() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1c20);
    let table = SCRATCH + 0x400;
    t.set_u32(table, 0x0700_0001);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + i);
        }
        0x325
    });
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(262) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0x45a0);
    for &(c, p, cp) in sa::RINGS {
        sa::ring(&mut t, c, p, cp, 0x30);
        let o = sa::node(0);
        for i in 0..3 {
            t.set_u32(o + 0x38 + 4 * i, [7u32, 0xffff_fff0, 0x100][i as usize]);
            t.set_u32(o + 0x2c + 4 * i, 0x1000 * (i + 1));
        }
        t.set_u32(o + 0x44, 1.75f32.to_bits());
        t.check(
            f,
            Call::this(sa::CURSOR),
            &[],
            Eax::Checked,
            s4_small::walk_scale_steps,
        );
    }
    drop(t);
    let Some(mut t) = twin(393) else { return };
    let f = t.at(0x43b0);
    recs(&mut t, &[(255, 2), (32, 3), (33, 4)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 11, 1, move |m, _| m.r32(answer));
    let p = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x9), p);
    for k in 0..0x21 {
        t.set_u32(op(&t, f, 0x37) + 4 * k, 0x0a00_0000 + k);
    }
    let v = SCRATCH + 0x300;
    for i in 0..3 {
        t.set_u32(v + 4 * i, (1.5f32 + i as f32).to_bits());
    }
    for (done, k, ground) in [(0u32, 3u8, 0u32), (0, 0x20, 0), (1, 3, 1), (1, 3, 0)] {
        t.set_u32(op(&t, f, 0x2), done);
        t.set_u8(p + 2, k);
        t.set_u32(answer, ground);
        let c = Call {
            edx: v,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::actor_to_point);
    }
}

#[test]
fn timeline_frames_drops_states() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x2c90);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 253, 0, move |m, _| m.r32(answer));
    recs(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    for o in [0xd, 0x29, 0x34, 0x66] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x100 + o);
    }
    for (on, busy) in [(0u32, 0u32), (1, 0), (1, 1)] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(answer, busy);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(443) else { return };
    recs(&mut t, &[(258, 1), (231, 3), (230, 2)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x1280);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x74), lock);
    t.set_u32(op(&t, f, 0x67), SCRATCH + 0x88);
    let part = SCRATCH + 0x100;
    for a in [0u32, 1] {
        t.write(part, &[0x44; 0x80]);
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check(
            f,
            Call::this(part),
            &[],
            Eax::Checked,
            s4_small::part_drop_rates,
        );
    }
    drop(t);
    let Some(mut t) = twin(392) else { return };
    let f = t.at(0x2530);
    let take = t.img.rel32_target(f + 0x65);
    let block = SCRATCH + 0x2000;
    t.set_u32(op(&t, f, 0x3), block);
    for listed in [false, true] {
        pool_ring(&mut t, take);
        t.write(block + 0xff0, &[0x11; 0xa0]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x6b), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x7f), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x6b), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn particles_e_and_camera_places() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1580);
    let table = SCRATCH + 0x400;
    t.set_u32(table + 8, 0x0700_0008);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + i);
        }
        0x325
    });
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(434) else { return };
    recs(&mut t, &[(723, 1), (724, 2), (327, 2), (229, 3)]);
    let f = t.at(0x68f0);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
}

#[test]
fn trail_steps() {
    for (n, rva, count_at, points, flag_at, sh) in [
        (180u16, 0x2a20u32, 0x248u32, 0x24u32, 0x284u32, 0u32),
        (18, 0x3630, 0x354, 0x130, 0x394, 3),
    ] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        // the trail draw's own stubs (see magic_s4_misc's trail_draw)
        let td = t.img.rel32_target(f + 0x70 + sh);
        let lock = SCRATCH + 0x9000;
        t.set_u32(t.img.word(td + 0x46e), lock);
        let table = SCRATCH + 0x8000;
        rec(&mut t, 941, 1, move |_, _| table);
        t.write(table, &[0; 0x100]);
        t.set_u16(table + 6, 1);
        t.set_u16(table + 0x10, 0x80);
        t.set_u16(table + 0x12, 0x300);
        recs(
            &mut t,
            &[(235, 1), (723, 1), (724, 1), (234, 3), (248, 3), (229, 3)],
        );
        rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[1]);
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
        rec(&mut t, 773, 4, |_, _| 0);
        for s in [774u32, 775, 778, 776, 777] {
            rec(&mut t, s, 1, move |_, _| s);
        }
        let (task, trail) = (SCRATCH + 0x100, SCRATCH + 0x1000);
        for count in [0u32, 0xffff_ffff, 3, 40] {
            t.write(trail, &[0; 0x400]);
            t.set_u16(trail + flag_at, 0x0101);
            let p = trail + points;
            for k in 0..32u32 {
                for i in 0..3 {
                    t.set_u32(
                        p + 0xc + 16 * k + 4 * i,
                        (k as f32 * 0.5 + i as f32).to_bits(),
                    );
                }
                t.set_u32(p + 0xc + 16 * k + 0xc, 0x3f80_0000);
            }
            t.write(p + 0x20c, &[10, 200, 0, 0x80, 250, 3, 128, 0x40]);
            t.set_u32(p + 0x214, 1.5f32.to_bits());
            t.set_u32(p + 0x218, 0.25f32.to_bits());
            t.set_u32(p + 0x21c, 0.75f32.to_bits());
            t.set_u32(p + 0x220, 5);
            t.set_u32(trail + 0x24c + (count_at - 0x248), 0x3f00_0000);
            // the step count is the trail's sprite count (`points + 0x224`)
            t.set_u32(trail + count_at, count);
            t.write(task, &[0; 0x20]);
            t.write(task + 0x100, &[0; 0x20]);
            t.set_u32(task + 8, task + 0x100);
            t.set_u32(task + 0x14, trail);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

#[test]
fn actors_motions_spins() {
    let Some(mut t) = twin(193) else { return };
    let f = t.at(0x1560);
    let rec_at = SCRATCH + 0x600;
    rec(&mut t, 434, 2, move |_, _| rec_at);
    recs(&mut t, &[(444, 1), (446, 2), (450, 2), (449, 4)]);
    let block = SCRATCH + 0x1000;
    for o in [0x14, 0x6a] {
        t.set_u32(op(&t, f, o), block);
    }
    for i in 0..3 {
        t.set_u32(block + 0x1a0 + 4 * i, 0x0a00_0000 + i);
    }
    let obj = SCRATCH + 0x100;
    for n in [0u32, 1, 3] {
        t.write(rec_at, &[0x22; 0x100]);
        t.set_u32(rec_at + 0xac, 1);
        t.set_u32(block + 0x19c, n);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    let f = t.at(0x1bc0);
    recs(&mut t, &[(442, 3), (443, 2), (440, 4)]);
    t.set_u32(op(&t, f, 0x16), block);
    t.write(obj, &[0; 0x140]);
    t.set_u32(obj + 0xac, 1);
    t.set_u32(obj + 0x3c, 2);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(84) else { return };
    let f = t.at(0x1710);
    let ro = t.img.rel32_target(f + 0x83);
    recs(&mut t, &[(723, 1), (724, 2), (235, 3)]);
    let task = SCRATCH + 0x100;
    for (a, b, one, wait) in [
        (0.5f32, 1.0f32, false, 0u32),
        (6.2, -6.2, false, 1),
        (100.0, -100.0, true, 0),
        (f32::NAN, 0.0, false, 0),
    ] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x200);
        t.set_u32(op(&t, f, 0x7), a.to_bits());
        t.set_u32(op(&t, f, 0x4b), b.to_bits());
        t.set_u32(op(&t, f, 0x8e), wait);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn static_moves_light_nodes() {
    let Some(mut t) = twin(609) else { return };
    let f = t.at(0x9170);
    let m = op(&t, f, 0x4) - 0x2c;
    for i in 0..0x1c {
        t.set_u32(m + 4 * i, (0.5f32 * i as f32 - 3.0).to_bits());
    }
    t.set_u32(m + 0x68, 0xffff_fff6);
    let c = Call::default().with_xmm(&[
        Xmm::F32(0),
        Xmm::F32(0),
        Xmm::F32(1.25f32.to_bits()),
        Xmm::F32(f32::NAN.to_bits()),
    ]);
    t.check(f, c, &[], Eax::Checked, s4_small::move_setup_static);
    drop(t);
    let Some(mut t) = twin(578) else { return };
    recs(&mut t, &[(235, 1)]);
    let f = t.at(0x3020);
    let (r, n, parent, kid) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
    );
    for (p, kids) in [(0u32, false), (parent, false), (parent, true)] {
        t.write(n, &[0x66; 0xb0]);
        t.write(parent, &[0; 0x20]);
        t.write(kid, &[0x77; 0x10]);
        let l = if kids { kid } else { parent };
        t.set_u32(parent + 0x10, l);
        t.set_u32(parent + 0xc, l);
        for (i, w) in [n, p, 0xa, 0xb, 0xc].into_iter().enumerate() {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::light_node);
    }
}

#[test]
fn turns() {
    let Some(mut t) = twin(55) else { return };
    recs(&mut t, &[(235, 1), (328, 3), (329, 3), (229, 3), (248, 3)]);
    let f = t.at(0x30f0);
    let (ang, out, point) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    t.set_u32(ang, 0.5f32.to_bits());
    t.set_u32(ang + 4, (-1.25f32).to_bits());
    t.write(out, &[0x11; 0x10]);
    t.write(point, &[0x22; 0x10]);
    let c = Call {
        ecx: ang,
        edx: out,
        ..Call::default()
    };
    t.check(f, c, &[point], Eax::Checked, s4_small::turn_point);
}

#[test]
fn sounds_and_means() {
    let Some(mut t) = twin(563) else { return };
    rec(&mut t, 562, 0, |_, _| 0x5620);
    let f = t.at(0x1fd0);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x8), block);
    t.write(block + 0x3b30, &[0x77; 0x50]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(307) else { return };
    let f = t.at(0x79b0);
    recs(&mut t, &[(327, 2), (229, 3)]);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (1u8..).zip(&a[1..4]) {
            m.w32(*at, (f32::from(i) * 0.5).to_bits());
        }
        0x262
    });
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn node_copies_and_frames_261() {
    let Some(mut t) = twin(186) else { return };
    recs(&mut t, &[(230, 2), (325, 2)]);
    let f = t.at(0x3ad0);
    let (r, n, parent, src) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x800,
    );
    for p in [0u32, parent] {
        t.write(n, &[0x66; 0x140]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        t.set_u32(r, n);
        t.set_u32(r + 4, p);
        t.set_u32(r + 8, src);
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::node_copy);
    }
    drop(t);
    let Some(mut t) = twin(206) else { return };
    let f = t.at(0x1bc0);
    let answers = SCRATCH + 0x90;
    rec(&mut t, 261, 1, move |m, a| m.r32(answers + 4 * (a[0] & 1)));
    recs(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    for o in [0xd, 0x3f] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x100 + o);
    }
    for (on, run, stop) in [(0u32, 0u32, 0u32), (1, 0, 0), (1, 1, 0), (1, 0, 1)] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(answers, run);
        t.set_u32(answers + 4, stop);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn colour_scale_tasks_light_copies() {
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x9390);
    let take = t.img.rel32_target(f + 0x59);
    let block = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x4), block);
    for listed in [false, true] {
        pool_ring(&mut t, take);
        t.write(block + 0x1590, &[0x11; 0x300]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x5f), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x72), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x5f), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x2550);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x33), block);
    for i in 0..4 {
        t.set_u32(block + 0xe70 + 4 * i, 0x0e70_0000 + i);
    }
    let light = SCRATCH + 0x100;
    for i in 0..4 {
        t.set_u32(light + 0x2c + 4 * i, 0x3f00_0000 + i);
    }
    t.check_entry(f, Call::default(), &[light], Eax::Checked);
}

#[test]
fn actor_entries_and_scale_tasks() {
    let Some(mut t) = twin(384) else { return };
    let f = t.at(0x12f0);
    let rec_at = SCRATCH + 0x600;
    rec(&mut t, 434, 2, move |_, _| rec_at);
    recs(&mut t, &[(444, 1), (450, 2), (449, 4)]);
    let base = SCRATCH + 0x1000;
    for o in [0x4, 0x4c, 0x6d] {
        t.set_u32(op(&t, f, o), base);
    }
    t.set_u32(base + 0x198, 0x0198_0001);
    let obj = SCRATCH + 0x100;
    for n in [0u32, 1, 3] {
        t.write(rec_at, &[0x22; 0x100]);
        t.set_u32(rec_at + 0xac, 1);
        t.write(obj, &[0x33; 0x100]);
        t.set_u32(obj + 0xac, 2);
        t.set_u32(op(&t, f, 0x14), n);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(348) else { return };
    let f = t.at(0x2c00);
    let take = t.img.rel32_target(f + 0x73);
    let s = SCRATCH + 0x100;
    for (listed, a, b) in [(false, 0.5f32, -1.0f32), (true, 0.0, f32::NAN)] {
        pool_ring(&mut t, take);
        t.write(s, &[0x44; 0x40]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x79), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x91), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x79), 0);
        }
        let c = Call {
            ecx: s,
            edx: 0x1e,
            ..Call::default()
        }
        .with_xmm(&[Xmm::F32(0), Xmm::F32(a.to_bits()), Xmm::F32(b.to_bits())]);
        t.check(f, c, &[], Eax::Checked, s4_small::scale_task_edx);
    }
}

#[test]
fn guarded_drops_and_resets() {
    let Some(mut t) = twin(188) else { return };
    recs(&mut t, &[(258, 1), (231, 3), (230, 2)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    let f = t.at(0x1f10);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x85), lock);
    t.set_u32(op(&t, f, 0x78), SCRATCH + 0x88);
    let part = SCRATCH + 0x100;
    for (on, a) in [(0u32, 0u32), (1, 0), (1, 1)] {
        t.write(part, &[0x44; 0x80]);
        t.set_u32(op(&t, f, 0x5), on);
        t.set_u32(answer, a);
        t.set_u32(lock, 3);
        t.check_entry(f, Call::default(), &[part], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(285) else { return };
    let f = t.at(0x4c00);
    let ms = t.img.rel32_target(f + 0x9);
    let ro = t.img.rel32_target(f + 0x10);
    recs(
        &mut t,
        &[(723, 1), (724, 2), (235, 3), (325, 2), (941, 1), (779, 4)],
    );
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    let rg = t.img.rel32_target(f + 0x3a);
    t.set_u32(t.img.word(rg + 0x05), 0);
    let r8 = t.img.rel32_target(f + 0x9f);
    t.set_u32(t.img.word(r8 + 0x21), SCRATCH + 0x80);
    let task = SCRATCH + 0x100;
    for (done, one) in [(0u32, false), (1, true), (1, false)] {
        t.write(task, &[0; 0x20]);
        t.set_u32(task + 8, SCRATCH + 0x200);
        t.set_u32(t.img.word(ms + 0x02), done);
        t.set_u16(t.img.word(ms + 0x11), 5);
        objects(&mut t, ro, one);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn two_children_and_light_levels() {
    let Some(mut t) = twin(146) else { return };
    sa::handler_stubs(&mut t);
    let f = t.at(0x6390);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 941, 1, move |m, _| m.r32(answer));
    let (c1, c2) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    for &(c, p, cp) in sa::RINGS {
        for a in [0u32, 1] {
            sa::ring(&mut t, c, p, cp, 0x30);
            let owner = sa::node(0);
            for (k, ch) in [c1, c2].into_iter().enumerate() {
                t.write(ch, &[0x41 + k as u8; 0x40]);
                t.set_u32(ch, sa::VTABLE);
            }
            t.set_u32(owner + 0x3c, c1);
            t.set_u32(owner + 0x40, c2);
            t.set_u32(answer, a);
            t.check_entry(f, Call::default(), &[sa::CURSOR], Eax::Checked);
        }
    }
    drop(t);
    let Some(mut t) = twin(578) else { return };
    let f = t.at(0x2f60);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 363, 3, move |m, _| m.r32(answer));
    recs(&mut t, &[(357, 3), (356, 6), (362, 4), (704, 4)]);
    let (r, obj, lm, rp) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
        SCRATCH + 0x380,
    );
    t.set_u32(obj + 0x68, lm);
    t.set_u32(lm + 0x54, 0x0054_0001);
    t.set_u32(obj + 0x70, rp);
    t.set_u32(rp, 0xffff_4321);
    for (level, max, mode) in [(5u32, 3u32, 0u32), (2, 3, 0), (3, 3, 1)] {
        t.write(r, &[0; 0x20]);
        t.set_u32(r, obj);
        t.set_u32(r + 4, 0x0004_0001);
        t.set_u32(r + 8, max);
        t.set_u32(r + 0x14, 0x14);
        t.set_u32(r + 0x18, 0x18);
        t.set_u32(r + 0x1c, mode);
        t.set_u32(answer, level);
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::light_level);
    }
}

#[test]
fn summon_nodes_and_light_links() {
    let Some(mut t) = twin(201) else { return };
    let f = t.at(0x3660);
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 5, 1]);
    let count = SCRATCH + 0x94;
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14] {
        rec(&mut t, s, 1, move |_, _| s);
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    let (r, n, parent) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x400);
    for k in [0u32, 2] {
        t.write(n, &vec![0x66; 0x500]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        t.set_u32(count, k);
        for (i, w) in [n, parent, 0xa, 0xb].into_iter().enumerate() {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::summon_nodes);
    }
    drop(t);
    let Some(mut t) = twin(609) else { return };
    let f = t.at(0x9bb0);
    let tail = SCRATCH + 0x300;
    t.write(tail, &[0x77; 0x10]);
    t.set_u32(op(&t, f, 0x1f), tail);
    t.set_u32(op(&t, f, 0x2e), tail);
    let (n, a, c) = (SCRATCH + 0x100, SCRATCH + 0x400, SCRATCH + 0x500);
    t.write(n, &[0x11; 0x40]);
    t.write(a, &[0x22; 0x60]);
    t.write(c, &[0x33; 0x40]);
    t.check(
        f,
        Call::this(n),
        &[a, 0, c],
        Eax::Checked,
        s4_small::light_link,
    );
}

#[test]
fn place_turns_and_two_timelines() {
    let Some(mut t) = twin(393) else { return };
    let f = t.at(0x2bf0);
    let atp = t.img.rel32_target(f + 0xe);
    recs(&mut t, &[(255, 2), (32, 3), (33, 4), (11, 1), (634, 2)]);
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x3), block);
    let p = SCRATCH + 0x80;
    t.set_u32(t.img.word(atp + 0x9), p);
    t.set_u32(op(&t, f, 0x70), p);
    t.set_u8(p + 2, 1);
    for k in 0..4 {
        t.set_u32(op(&t, f, 0x81) + 4 * k, 0x0a00_0000 + k);
    }
    for ang in [0.0f32, 3.0, -3.0, 100.0, f32::NAN] {
        t.set_u32(block + 0x1ee0, ang.to_bits());
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(530) else { return };
    let f = t.at(0x1aa0);
    recs(&mut t, &[(723, 1), (724, 2), (214, 1)]);
    for o in [0xd, 0x3f] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x100 + o);
    }
    for (on, which, live) in [(0u32, 0u32, 0u32), (1, 0, 0), (1, 1, 0), (1, 0, 1)] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(op(&t, f, 0x4d), which);
        for o in [0x55, 0x5c] {
            let tl = op(&t, f, o);
            t.write(tl, &[0; 0x10]);
            t.set_u32(tl + 8, live);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn particles_f() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x17a0);
    let table = SCRATCH + 0x400;
    t.set_u32(table + 0xc, 0x0700_000c);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
        }
        0x325
    });
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
}

#[test]
fn particle_sounds_and_index_points() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x2580);
    let table = SCRATCH + 0x400;
    t.set_u32(table + 8, 0x0700_0008);
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5), (306, 3)]);
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, 0x3f00_0000 + i);
        }
        0x325
    });
    t.set_u32(op(&t, f, 0x4), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x10), SCRATCH + 0x200);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(298) else { return };
    let f = t.at(0x2110);
    recs(&mut t, &[(255, 2), (32, 3), (33, 4)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 11, 1, move |m, _| m.r32(answer));
    let map = SCRATCH + 0x80;
    t.write(map, &[3, 0x20, 1]);
    t.set_u32(op(&t, f, 0xc), map);
    t.set_u32(op(&t, f, 0x13), 4);
    t.set_u32(op(&t, f, 0x41), 0x0e0e_0041);
    for k in 0..0x21 {
        t.set_u32(op(&t, f, 0x4e) + 4 * k, 0x0a00_0000 + k);
    }
    let v = SCRATCH + 0x300;
    for i in 0..3 {
        t.set_u32(v + 4 * i, (1.5f32 + i as f32).to_bits());
    }
    for (done, i, ground) in [
        (0u32, u32::MAX, 0u32),
        (0, 0, 0),
        (0, 1, 1),
        (1, u32::MAX, 0),
        (1, 2, 1),
    ] {
        t.set_u32(op(&t, f, 0x2), done);
        t.set_u32(answer, ground);
        let c = Call {
            ecx: i,
            edx: v,
            ..Call::default()
        };
        t.check(f, c, &[], Eax::Checked, s4_small::actor_index_to_point);
    }
}

#[test]
fn motion_sets_b_and_camera_boxes() {
    let Some(mut t) = twin(384) else { return };
    let f = t.at(0x22f0);
    recs(
        &mut t,
        &[(444, 1), (442, 3), (443, 2), (440, 4), (450, 2), (449, 2)],
    );
    let block = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0x17), block);
    for i in 0..3 {
        t.set_u32(block + 0x1a0 + 4 * i, 0x0a00_0000 + i);
    }
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0; 0x140]);
    t.set_u32(obj + 0xac, 1);
    t.set_u32(obj + 0x3c, 2);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x9730);
    recs(&mut t, &[(327, 2)]);
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, (1.5f32 + i as f32).to_bits());
        }
        0x229
    });
    t.set_u32(op(&t, f, 0x13), SCRATCH + 0x1000);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn fixed_scale_tasks() {
    for (n, rva) in [(307u16, 0xb740u32), (168, 0x78e0), (588, 0x5690)] {
        let Some(mut t) = twin(n) else { return };
        let f = t.at(rva);
        let take = t.img.rel32_target(f + 0x7b);
        let s = SCRATCH + 0x100;
        for (listed, a, b) in [(false, 0.5f32, -1.0f32), (true, 0.0, f32::NAN)] {
            pool_ring(&mut t, take);
            t.write(s, &[0x44; 0x40]);
            if listed {
                t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
                t.set_u32(op(&t, f, 0x81), SCRATCH + 0x5000);
                t.set_u32(op(&t, f, 0x99), SCRATCH + 0x5000);
            } else {
                t.set_u32(op(&t, f, 0x81), 0);
            }
            let c = Call {
                ecx: s,
                edx: 0x0bad_0edd,
                ..Call::default()
            }
            .with_xmm(&[Xmm::F32(0), Xmm::F32(a.to_bits()), Xmm::F32(b.to_bits())]);
            t.check(f, c, &[], Eax::Checked, s4_small::scale_task_fixed);
        }
    }
}

#[test]
fn argument_scale_tasks() {
    let Some(mut t) = twin(249) else { return };
    let f = t.at(0x31b0);
    let take = t.img.rel32_target(f + 0x7d);
    let s = SCRATCH + 0x100;
    for (listed, a, b) in [(false, 0.5f32, -1.0f32), (true, 0.0, f32::NAN)] {
        pool_ring(&mut t, take);
        t.write(s, &[0x44; 0x40]);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, f, 0x83), SCRATCH + 0x5000);
            t.set_u32(op(&t, f, 0x9b), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, f, 0x83), 0);
        }
        let c = Call {
            ecx: s,
            ..Call::default()
        }
        .with_xmm(&[Xmm::F32(0), Xmm::F32(a.to_bits()), Xmm::F32(b.to_bits())]);
        t.check(f, c, &[0x0028_0028], Eax::Checked, s4_small::scale_task_arg);
    }
}

#[test]
fn move_ends_and_pulses() {
    let Some(mut t) = twin(70) else { return };
    let f = t.at(0x3e00);
    let (r, obj, from, to) = (
        SCRATCH + 0x100,
        SCRATCH + 0x200,
        SCRATCH + 0x400,
        SCRATCH + 0x500,
    );
    for i in 0..3 {
        t.set_u32(from + 4 * i, (1.5f32 + i as f32).to_bits());
        t.set_u32(to + 4 * i, (-2.0f32 * i as f32).to_bits());
    }
    for mode in [0u32, 1, 2, 0x0105] {
        t.write(obj, &[0x11; 0x140]);
        for i in 0..3 {
            t.set_u32(obj + 0xac + 4 * i, (10.0f32 + i as f32).to_bits());
        }
        for (i, w) in [obj, from, to, mode].into_iter().enumerate() {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::move_ends);
    }
    drop(t);
    let Some(mut t) = twin(515) else { return };
    recs(&mut t, &[(776, 1), (915, 5)]);
    let f = t.at(0x23c0);
    for step in [0u32, 3, 4, 5, 18, 19, 0xffff_fffb] {
        let c = Call {
            ecx: step,
            edx: 0x1ff,
            ..Call::default()
        };
        t.check(
            f,
            c,
            &[0x80, 0x7, 0x0776_0001],
            Eax::Checked,
            s4_small::pulse_colour,
        );
    }
}

#[test]
fn clock_modes_and_whites() {
    let Some(mut t) = twin(411) else { return };
    let f = t.at(0x6a20);
    recs(&mut t, &[(239, 3)]);
    t.set_u32(op(&t, f, 0xb1), 0x0239_00b1);
    let (task, obj, node, part) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x300,
    );
    for (clock, mode, flag) in [
        (0.0f32, 0u32, 0u8),
        (1e9, 0, 0),
        (f32::NAN, 0, 0),
        (0.0, 1, 4),
        (0.0, 1, 0),
        (0.0, 2, 4),
    ] {
        t.write(obj, &vec![0; 0x600]);
        t.set_u32(obj + 0x568, clock.to_bits());
        t.set_u32(obj + 0x574, mode);
        t.set_u8(obj + 0x108, flag);
        t.set_u32(obj + 0x578, part);
        t.set_u32(part + 0x50, 0x0050_0001);
        t.set_u32(task, node);
        t.set_u32(task + 4, obj);
        t.set_u32(node + 4, SCRATCH + 0x280);
        t.set_u32(node + 8, SCRATCH + 0x2c0);
        let c = Call {
            eax: 0x00ea_0000,
            ..Call::default()
        };
        t.check_entry(f, c, &[task], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(384) else { return };
    let f = t.at(0x1f20);
    recs(&mut t, &[(440, 4), (450, 2), (451, 4)]);
    let obj = SCRATCH + 0x100;
    for n in [9u32, 3, 0xffff_fff9] {
        t.write(obj, &[0x55; 0x200]);
        t.set_u32(op(&t, f, 0x4), n);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

#[test]
fn summon_nodes_b_and_sound_frames() {
    let Some(mut t) = twin(332) else { return };
    let f = t.at(0x5db0);
    let list = SCRATCH + 0x300;
    t.write(list, &[2, 5, 1]);
    let count = SCRATCH + 0x94;
    rec(&mut t, 412, 3, move |m, a| {
        m.w32(a[1], m.r32(count));
        list
    });
    for s in [413u32, 215, 14] {
        rec(&mut t, s, 1, move |_, _| s);
    }
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, |_, _| 0.5);
    let (r, n, parent) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x400);
    for k in [0u32, 2] {
        t.write(n, &vec![0x66; 0x2640]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        t.set_u32(count, k);
        for (i, w) in [n, parent, 0xa, 0xb, 0x0303].into_iter().enumerate() {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(
            f,
            Call::this(r),
            &[],
            Eax::Checked,
            s4_small::summon_nodes_b,
        );
    }
    drop(t);
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x2280);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 253, 0, move |m, _| m.r32(answer));
    recs(&mut t, &[(723, 1), (724, 2), (214, 1), (393, 3)]);
    for o in [0xd, 0x29, 0x34, 0x66] {
        t.set_u32(op(&t, f, o), SCRATCH + 0x100 + o);
    }
    let tl = op(&t, f, 0x75);
    for (on, busy, live, state) in [
        (0u32, 0u32, 0u32, 0u32),
        (1, 0, 0, 5),
        (1, 1, 1, 5),
        (1, 0, 1, 4),
    ] {
        t.set_u32(op(&t, f, 0x2), on);
        t.set_u32(answer, busy);
        t.write(tl, &[0; 0x10]);
        t.set_u32(tl + 8, live);
        t.set_u32(op(&t, f, 0xa0), state);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn resource_parts_and_actors_shown() {
    let Some(mut t) = twin(162) else { return };
    let f = t.at(0x1c40);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1] ^ (a[0] << 4));
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 773, 4, |_, a| (a[3] ^ a[2]) & 1);
    recs(&mut t, &[(779, 2)]);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x59), lock);
    t.set_u32(lock, 7);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x48a0);
    recs(&mut t, &[(257, 2), (384, 1), (25, 2), (24, 2)]);
    let map = SCRATCH + 0x80;
    t.write(map, &[2, 0, 1]);
    t.set_u32(op(&t, f, 0x1), map);
    for k in 0..3 {
        t.set_u32(op(&t, f, 0xd) + 4 * k, 0x0a00_0000 + k);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn host_objects_and_turned_offsets() {
    let Some(mut t) = twin(304) else { return };
    let at = t.at(0x7cd0);
    t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    let pa = t.img.rel32_target(at + 0x96);
    let free = t.img.word(pa + 0x04);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    let (host, obj, d) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x300);
    for (k, (count, scale, tl)) in [
        (3u16, 0.5f32, 0u32),
        (2, -2.0, SCRATCH + 0x600),
        (0, 1.0, 0),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(obj, &[0x5a; 0x130]);
        t.write(host, &[0x33; 0x40]);
        t.set_u32(host + 0x2c, d);
        t.set_u32(d + 0x10, 0x00d0_0010);
        t.set_u32(host + 0x34, tl);
        t.set_u16(free, count);
        let c = Call {
            ecx: host,
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
        let args = [k as u32 * 2 + 1, 0x0a00 + k as u32];
        t.check(at, c, &args, Eax::Checked, s4_small::start_host_object);
    }
    drop(t);
    let Some(mut t) = twin(430) else { return };
    let f = t.at(0x38e0);
    recs(&mut t, &[(235, 1), (328, 3), (329, 3), (229, 6), (248, 9)]);
    let b = SCRATCH + 0x1000;
    t.write(b, &vec![0x44; 0x1000]);
    t.set_u32(b + 0xeb4, 1.25f32.to_bits());
    t.set_u32(op(&t, f, 0x11), b);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn part_with_sound() {
    let Some(mut t) = twin(188) else { return };
    let f = t.at(0x2000);
    t.set_u32(op(&t, f, 0x11), SCRATCH + 0x200);
    t.set_u32(op(&t, f, 0x20), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    t.set_u32(SCRATCH + 0x400, 0x1111);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..3 {
            m.w32(a[0] + 4 * i, m.r32(a[1] + 4 * i));
        }
        0x325
    });
    rec(&mut t, 393, 3, |m, a| m.r32(a[1] + 4) ^ 0x0393_0000);
    t.set_u32(op(&t, f, 0x98), 2.5f32.to_bits());
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x5a; 0x130]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
}

#[test]
fn count_down_objects_and_sound_at() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x3410);
    recs(&mut t, &[(713, 1)]);
    let b = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xd), b);
    t.set_u32(op(&t, f, 0x30), SCRATCH + 0x80);
    t.set_u32(op(&t, f, 0x6f), SCRATCH + 0x84);
    let list = op(&t, f, 0x22);
    let task = SCRATCH + 0x100;
    let n1 = SCRATCH + 0x200;
    for (count, listed) in [
        (5u32, 0u32),
        (0, 0),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (0x8000_0001, 0),
    ] {
        t.write(b, &vec![0x3c; 0x600]);
        t.write(task, &[0; 0x60]);
        t.set_u32(task + 0x2c, count);
        t.write(n1, &[0; 0x40]);
        t.write(list, &[0; 0x10]);
        let objs: Vec<u32> = [0x390, 0x4c0]
            .iter()
            .enumerate()
            .filter(|(i, _)| listed >> i & 1 != 0)
            .map(|(_, o)| b + o)
            .collect();
        for (i, o) in objs.iter().enumerate() {
            let node = n1 + 0x10 * i as u32;
            t.set_u32(node + 8, *o);
            if i > 0 {
                t.set_u32(node - 0x10 + 4, node);
                t.set_u32(node, node - 0x10);
            }
        }
        if !objs.is_empty() {
            t.set_u32(list, n1);
            t.set_u32(list + 4, n1 + 0x10 * (objs.len() as u32 - 1));
            t.set_u32(list + 0xc, objs.len() as u32);
        }
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(530) else { return };
    let f = t.at(0x13f0);
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[1] + 4 * i, 0x0210_0000 + i);
        }
        0x210
    });
    t.set_u32(op(&t, f, 0x2c), SCRATCH + 0x200);
    t.set_u32(op(&t, f, 0x37), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    t.set_u32(SCRATCH + 0x404, 0x1111);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    t.stub(714, |m, a| {
        for (i, w) in (0u32..).zip(&a[..5]) {
            m.w32(SCRATCH + 0x910 + 4 * i, *w);
        }
        0x714
    });
    rec(&mut t, 393, 3, |m, a| m.r32(a[1] + 4) ^ 0x0393_0000);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x5a; 0x130]);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
}

#[test]
fn object_models_and_key_curves() {
    let Some(mut t) = twin(191) else { return };
    let f = t.at(0x1bb0);
    recs(
        &mut t,
        &[(446, 2), (442, 3), (443, 2), (440, 4), (450, 2), (449, 2)],
    );
    let base = SCRATCH + 0x1000;
    for i in 0..8 {
        t.set_u32(base + 0x1a0 + 4 * i, 0x0a00_0000 + i);
    }
    t.set_u32(op(&t, f, 0x17), base);
    t.set_u32(op(&t, f, 0x5d), base + 4);
    let obj = SCRATCH + 0x100;
    t.write(obj, &[0x22; 0x140]);
    t.set_u32(obj + 0xac, 2);
    t.set_u32(obj + 0x2c + 0x20, 5);
    t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(55) else { return };
    let f = t.at(0x3000);
    let keys = SCRATCH + 0x100;
    for i in 0..10u32 {
        t.set_u32(keys - 0x20 + 16 * i, (1.5f32 * i as f32 - 3.0).to_bits());
    }
    for x in [
        0.0f32,
        0.25,
        0.5,
        1.25,
        1.75,
        3.5,
        4.75,
        5.0,
        5.2,
        5.9,
        -0.5,
        -1.5,
        f32::NAN,
    ] {
        let c = Call {
            ecx: keys,
            ..Call::default()
        }
        .with_xmm(&[Xmm::f32(x)]);
        t.check(f, c, &[], Eax::Checked, s4_small::key_curve);
    }
}

#[test]
fn light_objects() {
    let Some(mut t) = twin(578) else { return };
    let f = t.at(0x7fb0);
    recs(&mut t, &[(235, 1), (357, 3), (356, 6), (362, 4), (704, 4)]);
    let answer = SCRATCH + 0x90;
    rec(&mut t, 363, 3, move |m, _| m.r32(answer));
    let rp = SCRATCH + 0x380;
    t.set_u32(rp, 0xffff_4321);
    rec(&mut t, 719, 3, move |_, _| rp);
    let (obj, lm, tail, s) = (
        SCRATCH + 0x1000,
        SCRATCH + 0x300,
        SCRATCH + 0x600,
        SCRATCH + 0x700,
    );
    t.set_u32(lm + 0x54, 0x0054_0001);
    t.set_u32(op(&t, f, 0x88), s);
    t.set_u32(s, 0x0055_0000);
    let parent = op(&t, f, 0x6a);
    for level in [0x7_0000u32, 5] {
        t.write(obj, &[0x11; 0x100]);
        t.write(tail, &[0x22; 0x10]);
        t.set_u32(op(&t, f, 0x21), tail);
        t.set_u32(op(&t, f, 0x3d), 0xffff_ffff);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        t.set_u32(answer, level);
        let c = Call {
            ecx: obj,
            edx: lm,
            ..Call::default()
        };
        t.check(f, c, &[0x0b0b], Eax::Checked, s4_small::light_object);
    }
}

#[test]
fn effect_end_states() {
    let Some(mut t) = twin(152) else { return };
    let f = t.at(0xc6b0);
    recs(
        &mut t,
        &[
            (694, 4),
            (384, 1),
            (24, 2),
            (25, 2),
            (211, 2),
            (688, 1),
            (213, 2),
        ],
    );
    for (i, o) in [0x1c, 0x51, 0x64, 0x9b, 0xb9, 0xc6].into_iter().enumerate() {
        t.set_u32(op(&t, f, o), 0x0a00_0000 + i as u32);
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn light_block_defaults() {
    let Some(mut t) = twin(168) else { return };
    let f = t.at(0x4230);
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.1);
    recs(&mut t, &[(557, 1), (560, 2)]);
    let sc = t.img.rel32_target(f + 0xce);
    let take = t.img.rel32_target(sc + 0x7b);
    let b = SCRATCH + 0x1000;
    for listed in [false, true] {
        pool_ring(&mut t, take);
        t.write(b, &vec![0x44; 0xf00]);
        t.set_u32(op(&t, f, 0x33), b);
        if listed {
            t.write(SCRATCH + 0x5000, &[0x22; 0x1c]);
            t.set_u32(op(&t, sc, 0x81), SCRATCH + 0x5000);
            t.set_u32(op(&t, sc, 0x99), SCRATCH + 0x5000);
        } else {
            t.set_u32(op(&t, sc, 0x81), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn sprite_record_setups() {
    let Some(mut t) = twin(443) else { return };
    let f = t.at(0x1d60);
    rec(&mut t, 0, 1, |_, _| 0x0abc_0000);
    recs(&mut t, &[(26, 2), (387, 2), (385, 2), (386, 2)]);
    for h in [0u32, 0x0123_4567] {
        t.set_u32(op(&t, f, 0x1), h);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn release_and_remove_two() {
    let Some(mut t) = twin(563) else { return };
    let f = t.at(0x4ff0);
    recs(&mut t, &[(713, 1), (779, 4)]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[0]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    let lock = SCRATCH + 0x80;
    t.set_u32(op(&t, f, 0x49), lock);
    t.set_u32(op(&t, f, 0x31), SCRATCH + 0x8000);
    t.set_u32(op(&t, f, 0x95), SCRATCH + 0x84);
    let b = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0xa), b);
    let list = op(&t, f, 0x82);
    let (n1, held) = (SCRATCH + 0x200, SCRATCH + 0x600);
    for (hold, listed) in [(false, 0u32), (true, 3), (false, 2), (true, 1)] {
        t.write(b, &vec![0x3c; 0x3c00]);
        t.set_u32(lock, 7);
        if hold {
            pool_node(&mut t, held);
            t.set_u32(b + 0x3ba0, held);
        } else {
            t.set_u32(b + 0x3ba0, 0);
        }
        t.write(n1, &[0; 0x40]);
        t.write(list, &[0; 0x10]);
        let objs: Vec<u32> = [0x5f0, 0x980]
            .iter()
            .enumerate()
            .filter(|(i, _)| listed >> i & 1 != 0)
            .map(|(_, o)| b + o)
            .collect();
        for (i, o) in objs.iter().enumerate() {
            let node = n1 + 0x10 * i as u32;
            t.set_u32(node + 8, *o);
            if i > 0 {
                t.set_u32(node - 0x10 + 4, node);
                t.set_u32(node, node - 0x10);
            }
        }
        if !objs.is_empty() {
            t.set_u32(list, n1);
            t.set_u32(list + 4, n1 + 0x10 * (objs.len() as u32 - 1));
            t.set_u32(list + 0xc, objs.len() as u32);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn actors_restored() {
    let Some(mut t) = twin(165) else { return };
    let f = t.at(0x1ad0);
    recs(&mut t, &[(8, 2), (256, 5), (530, 2), (249, 2)]);
    let map = SCRATCH + 0x80;
    t.write(map, &[3, 0x25, 0x1f, 0x20]);
    t.set_u32(op(&t, f, 0x31), map);
    for k in [3u32, 0x1f, 0x20, 0x25] {
        t.set_u32(op(&t, f, 0x3c) + 4 * k, 0x0a00_0000 + k);
    }
    let held = SCRATCH + 0x600;
    for (n, keep, hold) in [
        (0u32, 0u32, false),
        (4, 0, true),
        (4, 1, false),
        (0xffff_fffe, 0, true),
    ] {
        t.set_u32(op(&t, f, 0x1a), n);
        t.set_u32(op(&t, f, 0x78), keep);
        if hold {
            pool_node(&mut t, held);
            t.set_u32(op(&t, f, 0xde), held);
        } else {
            t.set_u32(op(&t, f, 0xde), 0);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

#[test]
fn command_blocks_and_spin_records() {
    let Some(mut t) = twin(434) else { return };
    let f = t.at(0x3b90);
    recs(&mut t, &[(559, 3), (558, 1), (382, 2)]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
    drop(t);
    let Some(mut t) = twin(563) else { return };
    let f = t.at(0x4c20);
    let b = SCRATCH + 0x1_0000;
    t.set_u32(op(&t, f, 0x1), b);
    let base = b + 0x2ed0;
    for i in 0..(0x12 * 0x60 / 4) {
        let v = match i % 7 {
            0 => 3.0f32,
            1 => -3.5,
            2 => 0.25,
            3 => f32::NAN,
            4 => 1.5,
            5 => -0.75,
            _ => 2.75,
        };
        t.set_u32(base + 4 * i, v.to_bits());
    }
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

#[test]
fn camera_faces_and_sprite_nodes() {
    let Some(mut t) = twin(349) else { return };
    let f = t.at(0x47a0);
    let turn = SCRATCH + 0x90;
    rec(&mut t, 333, 1, |_, _| 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(turn))));
    rec(&mut t, 262, 4, |_, _| 0x262);
    rec(&mut t, 12, 1, |_, _| 0x12);
    t.stub_st0(12, |_, _| 1.0e40);
    let b = SCRATCH + 0x1000;
    t.set_u32(op(&t, f, 0xe), b);
    for x in [0.5f32, 9.0, -9.0, f32::NAN] {
        t.write(b, &vec![0x11; 0x1c10]);
        t.set_u32(turn, x.to_bits());
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
    drop(t);
    let Some(mut t) = twin(150) else { return };
    let f = t.at(0x4b80);
    rec(&mut t, 312, 4, |_, a| 0x0312_0000 ^ a[0]);
    let (r, n, parent) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x400);
    for (p, glow) in [(0u32, 0u32), (parent, 5)] {
        t.write(n, &vec![0x66; 0x950]);
        t.write(parent, &[0; 0x20]);
        t.set_u32(parent + 0x10, parent);
        t.set_u32(parent + 0xc, parent);
        for (i, w) in [n, p, 0xa, 0xb, 0xc, 0xd, 0xe, glow]
            .into_iter()
            .enumerate()
        {
            t.set_u32(r + 4 * i as u32, w);
        }
        t.check(f, Call::this(r), &[], Eax::Checked, s4_small::sprite_node);
    }
}
