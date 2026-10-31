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
        t.set_u8(r + 0x28, 0xf6);
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
