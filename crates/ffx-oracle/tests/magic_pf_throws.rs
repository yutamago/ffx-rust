//! The target-throw task (`ffx_magic::pf_throws`, a parameter family:
//! one port for the copies in five DLLs) against each copy's original.
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

mod magic;

use magic::{Call, Eax, SCRATCH, STACK_TOP, StubMem, Twin};

fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// `n` bytes that differ from their neighbours (`seed` picks the pattern).
fn varied(n: u32, seed: u32) -> Vec<u8> {
    (0..n).map(|i| (i * 7 + seed * 13 + 1) as u8).collect()
}

/// The entry's stack pointer (at the return address).
const ENTRY_ESP: u32 = STACK_TOP - 0x100;

/// Logs each call of `slot` into a row of its own (16 rows of 0x40 bytes
/// at `log`): the slot, its first `n` words. Answers `ret(m, args, k)`.
fn log_calls(
    t: &mut Twin,
    slot: u32,
    log: u32,
    n: usize,
    ret: impl Fn(&mut dyn StubMem, &[u32], u32) -> u32 + 'static,
) {
    t.stub(slot, move |m, a| {
        let k = m.r32(log);
        m.w32(log, k + 1);
        let row = log + 4 + 0x40 * (k % 16);
        m.w32(row, slot);
        for (i, w) in (1u32..).zip(&a[..n.min(a.len())]) {
            m.w32(row + 4 * i, *w);
        }
        ret(m, a, k)
    });
}

/// The DLL globals the set-up at `setup` names (its relocated operands
/// outside the code) that hold 0 get blocks of their own whose every word
/// points at the block: an empty list ring, a node with itself as
/// neighbours (the set-ups append to the DLL's lists through them).
fn prime(t: &mut Twin, setup: u32) {
    let mut next = SCRATCH + 0x1_0000;
    let mut todo = vec![(setup, 0)];
    let mut seen = Vec::new();
    while let Some((at, depth)) = todo.pop() {
        if seen.contains(&at) {
            continue;
        }
        seen.push(at);
        let Some(len) = ffx_magic::INTERNAL
            .iter()
            .chain(ffx_magic::PORTS)
            .find(|(_, sig)| t.img.matches(at, sig))
            .map(|(_, sig)| sig.len)
        else {
            continue;
        };
        for v in t.img.operands(at, len) {
            if t.img.in_text(v)
                || !(t.img.base..t.img.base + 0x0100_0000).contains(&v)
                || t.u32(v) != 0
            {
                continue;
            }
            for i in 0..0x40 {
                t.set_u32(next + 4 * i, next);
            }
            t.set_u32(v, next);
            next += 0x100;
        }
        // the callees' globals too (`call rel32`)
        if depth < 3 {
            for p in at..at + len.saturating_sub(4) {
                if t.img.word(p) as u8 == 0xe8 {
                    let c = t.img.rel32_target(p + 1);
                    if t.img.in_text(c) {
                        todo.push((c, depth + 1));
                    }
                }
            }
        }
    }
}

/// The copies: DLL, RVA.
const COPIES: &[(u16, u32)] = &[
    (412, 0x6d90),
    (414, 0x6e40),
    (425, 0x6db0),
    (439, 0x70d0),
    (441, 0x6e30),
];

/// One case: the state, the wait `S + 0x3c`, the objects left `S +
/// 0x44`, the frame count `S + 0x40`, the target count `*G_N`, slot
/// 411's answer, the first object's flag.
struct Case {
    state: u32,
    wait: u32,
    left: u32,
    frames: u32,
    targets: u32,
    done: u32,
    flag: u32,
}

const fn case(state: u32, wait: u32, left: u32, frames: u32, targets: u32) -> Case {
    Case {
        state,
        wait,
        left,
        frames,
        targets,
        done: 1,
        flag: 0xff,
    }
}

/// Every state of every copy: 0 with three targets (states 0, 10 and 11
/// in one run); an idle state; 10 with none and with one target; 11
/// waiting, with the object busy, the last throw, throws drawn among
/// three targets (from several random draws), a negative count; 12
/// before and past its frame count; 10000 not done and done; a large
/// state. Slot 291 counts each target (and writes the word it is
/// given), slot 307 draws from a sequence (ST0), every other slot logs
/// its words; the callees run as ported.
#[test]
#[allow(clippy::too_many_lines)] // the cases
fn throws() {
    for &(n, rva) in COPIES {
        let Some(mut t) = twin(n) else {
            return;
        };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let log = SCRATCH + 0x6000;
        log_calls(&mut t, 291, log, 5, |m, a, k| {
            m.w32(a[2], 0x2910 + k);
            1 + (a[1] ^ k) % 4
        });
        log_calls(&mut t, 228, log, 2, |_, a, k| 0x2280 ^ a[0] ^ k);
        log_calls(&mut t, 213, log, 2, |_, a, k| 0x2130 ^ a[0] ^ k);
        let done = SCRATCH + 0x6800;
        log_calls(&mut t, 411, log, 1, move |m, _, _| m.r32(done));
        log_calls(&mut t, 307, log, 0, |_, _, k| 0x3070 + k);
        t.stub_st0(307, move |m, _| {
            let k = m.r32(log);
            f64::from(k % 7) * 0.375 + 0.125
        });
        // the target nodes' slots: two targets
        let list = SCRATCH + 0x6c00;
        t.write(list, &[5, 7, 9, 11]);
        log_calls(&mut t, 218, log, 2, move |m, a, _| {
            m.w32(a[1], 2);
            list
        });
        for s in [413u32, 215] {
            log_calls(&mut t, s, log, 1, move |_, a, _| (s << 16) ^ a[0]);
        }
        log_calls(&mut t, 14, log, 4, |m, a, _| {
            for i in 1..4 {
                m.w32(a[i], a[0].wrapping_mul(i as u32));
            }
            0x14
        });
        // the colour node's slots (558 fills four integers)
        log_calls(&mut t, 558, log, 1, |m, a, k| {
            for i in 0..4 {
                m.w32(a[0] + 4 * i, 0xffff_fff0u32.wrapping_add(0x11 * i + k));
            }
            0x558
        });
        for (s, n) in [(331u32, 1), (353, 3), (271, 4)] {
            log_calls(&mut t, s, log, n, move |_, a, k| {
                (s << 16) ^ a.first().copied().unwrap_or(0) ^ k
            });
        }
        // the set-ups' slots: the program table, the nodes' handles
        let table = SCRATCH + 0x6e00;
        for i in 0..0x40 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        log_calls(&mut t, 941, log, 1, move |_, _, _| table);
        for (s, n) in [
            (714u32, 5),
            (719, 3),
            (325, 2),
            (268, 3),
            (239, 3),
            (390, 1),
            (534, 1),
        ] {
            log_calls(&mut t, s, log, n, move |_, a, k| {
                (s << 16) ^ a.first().copied().unwrap_or(0) ^ k
            });
        }
        for s in [12u32, 333, 790, 289] {
            // (no argument words read: some take none)
            t.stub_st0(s, move |m, _| {
                f64::from(m.r32(log) % 9) * 0.25 + f64::from(s)
            });
        }
        let (task, s, link) = (SCRATCH + 0x3000, SCRATCH + 0x8000, SCRATCH + 0x3200);
        let cases = [
            case(0, 0, 0, 0, 3),
            case(5, 0, 0, 7, 3),
            case(10, 0, 0, 0, 0),
            case(10, 4, 5, 0, 1),
            case(11, 4, 9, 0, 3),
            Case {
                flag: 0x7f,
                ..case(11, 0, 6, 0, 3)
            },
            case(11, 0, 1, 0, 3),
            case(11, 0, 7, 0, 3),
            case(11, 0, 12, 0, 3),
            case(11, 0, 13, 0, 1),
            case(11, 0, 0xffff_fff9, 0, 2),
            case(12, 0, 0, 0x10, 3),
            case(12, 0, 0, 0x50, 3),
            // exactly the closing wait (`u32::MAX`: the copy's own `+0x35a`)
            case(12, 0, 0, u32::MAX, 3),
            Case {
                done: 0,
                ..case(0x2710, 0, 0, 0, 3)
            },
            case(0x2710, 0, 0, 0, 3),
            case(0x3000, 0, 0, 0, 3),
        ];
        let setup = t.img.rel32_target(f + 0x26d);
        prime(&mut t, setup);
        let saved = t.save();
        for (c, k) in cases.iter().zip(0u32..) {
            t.restore(&saved);
            t.set_u32(log, 0);
            t.set_u32(done, c.done);
            t.write(ENTRY_ESP - 0x3000, &varied(0x3000, 0x5b + k));
            t.write(s, &varied(0x2400, k + 1));
            t.set_u32(s + 0x18, c.state);
            t.set_u32(s + 0x3c, c.wait);
            let frames = if c.frames == u32::MAX {
                u32::from(t.img.word(f + 0x35a) as u8)
            } else {
                c.frames
            };
            t.set_u32(s + 0x40, frames);
            t.set_u32(s + 0x44, c.left);
            for i in 0..4 {
                t.set_u32(s + 0x1c + 4 * i, (0.25f32 + 0.125 * i as f32).to_bits());
                t.set_u32(s + 0x48 + 4 * i, 2 + i);
            }
            // the objects' flags (`S + 0x340`-style, one per object)
            for k2 in 0..6 {
                let d = w(&t, 0x16f + 10 * k2);
                let mine = c.left.cast_signed().rem_euclid(5) == k2.cast_signed();
                t.set_u32(s.wrapping_add(d), if mine { c.flag } else { 0xff });
            }
            t.set_u32(w(&t, 0xeb), c.targets);
            // `*G_A`, `*G_P` and the sound's word: distinct values
            t.set_u32(w(&t, 0x37), 0x0037_a000 + k);
            t.set_u32(w(&t, 0x110), 0x0110_b000 + k);
            // the node parents' child lists: empty
            for o in [0x48u32, 0x76] {
                let l = w(&t, o);
                t.write(l, &varied(0x20, k + 9));
                t.set_u32(l + 0x10, l);
            }
            t.set_u32(task, link);
            t.set_u32(task + 4, s);
            let (prev, next) = (SCRATCH + 0x3280, SCRATCH + 0x32c0);
            t.write(link, &[0x66; 0x10]);
            t.set_u32(link + 4, prev);
            t.set_u32(link + 8, next);
            let call = Call {
                eax: 0x00ea_0000 + k,
                ecx: 0x00ec_0000 + rva,
                edx: 0x00ed_0000 + rva,
                ..Call::default()
            };
            t.check_entry(f, call, &[task], Eax::Checked);
        }
    }
}
