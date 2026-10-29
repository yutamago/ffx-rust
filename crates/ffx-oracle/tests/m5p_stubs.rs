//! M5 stage 1, list `stubs` (`ffx_pcore::m5p_stubs`): every row of the
//! table against its own original. First the row against the copy's
//! bytes in `FFX.exe` (shape, offset, convention and `ret n`), then the
//! copy runs on one oracle and the generic port on the other, from the
//! same memory: `eax` (`al` for the byte results; the rest of `eax` is the
//! caller's), `ST0` for the float fields, every byte written, and the
//! oracle's check that the copy pops the words its shim declares. On the
//! image alone and on the snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! None of these ran in a coverage run, so the inputs are synthetic: for
//! the methods, `this` 0, a pointer to filled memory, `u32::MAX` and
//! `0xffff_fff0` (`lea` wraps); for the field reads, several values at the
//! field (words, bytes 0/0x80/0xff, floats: signed zeros, a denormal,
//! infinities, a quiet NaN with a payload; signalling NaNs are left out,
//! the emulator keeps them); the stack words odd junk.
#![allow(clippy::cast_possible_truncation)]

mod m5p_tmpl_twins;

use ffx_oracle::{Conv, Oracle, f80_to_f32};
use ffx_pcore::m5p_stubs::bridge::{self, Out};
use ffx_pcore::m5p_stubs::{Kind, STUBS, Stub};
use m5p_tmpl_twins::{JUNK, Twins};

/// The bytes a row stands for.
fn expected_bytes(s: &Stub) -> Vec<u8> {
    let off = s.offset as u8;
    match s.kind {
        Kind::Zero => vec![0x33, 0xc0, 0xc3],
        Kind::False => vec![0x32, 0xc0, 0xc3],
        Kind::True => vec![0xb0, 0x01, 0xc3],
        Kind::Nothing if s.words == 0 => vec![0xc3],
        Kind::Nothing => vec![0xc2, (4 * s.words) as u8, 0x00],
        Kind::This => vec![0x8b, 0xc1, 0xc3],
        Kind::Word => vec![0x8b, 0x41, off, 0xc3],
        Kind::Addr => vec![0x8d, 0x41, off, 0xc3],
        Kind::Byte => vec![0x8a, 0x41, off, 0xc3],
        Kind::Float => vec![0xd9, 0x41, off, 0xc3],
    }
}

#[test]
fn m5p_stubs_rows_are_their_bytes() {
    for (what, mut tw) in Twins::all() {
        let mut seen = std::collections::HashSet::new();
        for s in STUBS {
            assert!(seen.insert(s.addr), "{:#x} twice", s.addr);
            let want = expected_bytes(s);
            let at = tw.rebase(s.addr);
            let have = Oracle::read(&mut tw.a, at, want.len()).unwrap();
            assert_eq!(have, want, "{what} {:#x}: the row is not its bytes", s.addr);
            // under five bytes: what makes it host-only
            assert!(want.len() < 5, "{:#x}", s.addr);
            let reads_ecx = matches!(
                s.kind,
                Kind::This | Kind::Word | Kind::Addr | Kind::Byte | Kind::Float
            );
            assert!(
                s.this || (!reads_ecx && s.words == 0),
                "{:#x}: a method",
                s.addr
            );
            if s.kind != Kind::Word
                && s.kind != Kind::Addr
                && s.kind != Kind::Byte
                && s.kind != Kind::Float
            {
                assert_eq!(s.offset, 0, "{:#x}", s.addr);
            }
        }
        assert_eq!(STUBS.len(), 179);
    }
}

/// Runs copy `s` on `this` in `tw.a` and its port in `tw.b`; compares the
/// result and every byte written.
fn check(tw: &mut Twins, what: &str, s: &Stub, this: u32) {
    let label = format!("{what} {:#x} this {this:#x}", s.addr);
    let args = &JUNK[..s.words as usize];
    let conv = if s.this {
        Conv::Thiscall(this)
    } else {
        Conv::Cdecl
    };
    let addr = tw.rebase(s.addr);
    let ra =
        tw.a.call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{label}: the original: {e}"));
    let out = bridge::run(&tw.b, s, this);
    match out {
        Out::St0(v) => {
            let raw = ra.st0_raw.unwrap_or_else(|| panic!("{label}: no ST0"));
            assert_eq!(f80_to_f32(raw).to_bits(), v.to_bits(), "{label}: ST0");
        }
        _ => assert!(ra.st0_raw.is_none(), "{label}: the original leaves ST0 set"),
    }
    match out {
        Out::Eax(v) => assert_eq!(ra.eax, v, "{label}: eax"),
        Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{label}: al"),
        Out::Void | Out::St0(_) => {}
    }
    tw.diff(&label);
}

#[test]
fn m5p_stubs_every_copy_matches_the_original() {
    let floats: [u32; 9] = [
        0x3fc0_0000, // 1.5
        0x8000_0000, // -0
        0x0000_0000,
        0x0000_0001, // the smallest denormal
        0x7f80_0000, // inf
        0xff80_0000, // -inf
        0x7fc1_2345, // a quiet NaN with a payload
        0x7f7f_ffff, // f32::MAX
        0xc2f6_e979, // -123.456
    ];
    for (what, mut tw) in Twins::all() {
        let obj = tw.alloc(
            &(0u8..0x80)
                .map(|i| i.wrapping_mul(37) ^ 0xa5)
                .collect::<Vec<_>>(),
        );
        let mut runs = 0;
        for s in STUBS {
            match s.kind {
                Kind::Zero | Kind::False | Kind::True | Kind::Nothing | Kind::This | Kind::Addr => {
                    for this in [0, obj, u32::MAX, 0xffff_fff0, 0x0012_3450] {
                        check(&mut tw, what, s, this);
                        runs += 1;
                    }
                }
                Kind::Word => {
                    for v in [0, 1, 0x8000_0000, u32::MAX, 0x1234_5678] {
                        tw.w32(obj + s.offset, v);
                        check(&mut tw, what, s, obj);
                        runs += 1;
                    }
                }
                Kind::Byte => {
                    for v in [0u8, 1, 0x7f, 0x80, 0xff] {
                        tw.write(obj + s.offset, &[v]);
                        check(&mut tw, what, s, obj);
                        runs += 1;
                    }
                }
                Kind::Float => {
                    for v in floats {
                        tw.w32(obj + s.offset, v);
                        check(&mut tw, what, s, obj);
                        runs += 1;
                    }
                }
            }
        }
        eprintln!("{what}: {runs} runs over {} copies", STUBS.len());
    }
}
