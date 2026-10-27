//! M5 stage 1, list `system-a` (`ffx_system::m5a_*`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Most of these functions never ran in a coverage run, so every branch
//! is reached with synthetic data (the snapshots add the game's own
//! tables where a function reads them). Callees that are still original
//! (`operator delete`, the string assign, the `MSVCP110` imports, virtual
//! methods of fake objects) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its arguments to
//! a log in memory, so the diff checks that both made the same calls in
//! the same order. Imports are stubbed through their import table slots.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::m5a_settings::{self as settings, bridge as st};
use ffx_system::m5a_std::{self as sd, bridge as sb};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// A word: small values, 0, 1, -1 and anything.
    fn word(&mut self) -> u32 {
        match self.below(5) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            _ => self.next(),
        }
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle, u32);

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            true,
        ))
    }

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
    }

    fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// A recording stub in the cave: logs `tag`, `ecx` when `this`, and
    /// `n` stack arguments; pops `pops` words; returns `ret`. Returns its
    /// address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// [`Twins::stub_code`] written over the original at `sym`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(sym, this, n, pops, ret);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// [`Twins::stub_code`] bound to the import table slot `slot`.
    fn stub_import(&mut self, slot: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(slot, this, n, pops, ret);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A fake object whose vtable's slots are recording thiscall stubs
    /// `(words, ret)`, tagged `0x5100 + slot`. Returns the object (16
    /// bytes, the vtable pointer first).
    fn object(&mut self, slots: &[(u32, u32)]) -> u32 {
        let mut vt = Vec::new();
        for (i, &(n, ret)) in slots.iter().enumerate() {
            vt.push(self.stub_code(0x5100 + i as u32, true, n, n, ret));
        }
        let vt = self.words(&vt);
        self.words(&[vt, 0, 0, 0])
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The original's `ebp` for a call with `args` stack words: its
    /// arguments, the return address and the saved `ebp` below where the
    /// call pushes.
    fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp`
    /// (its callees log pointers into it): `low` bytes below `ebp` are
    /// compared after the call.
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        self.write(lo, &vec![0; low as usize]);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let mut x = vec![0; low as usize];
        let mut y = vec![0; low as usize];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        assert_eq!(x, y, "{what}: frame at {lo:#x}");
    }
}

// ---- <system_error> ----

#[test]
fn release_facet_matches_the_original() {
    Twins::each(|t| {
        for case in 0..3 {
            t.reset();
            let dead = t.object(&[(1, 0x77)]);
            let facet = t.object(&[(0, 0), (0, 0), (0, if case == 2 { dead } else { 0 })]);
            let holder = t.words(&[if case == 0 { 0 } else { facet }]);
            let what = format!("release_facet #{case}");
            t.check(&what, 0x0040_11d0, Conv::Thiscall(holder), &[], |m| {
                sb::release_facet(m, holder);
                Out::None
            });
        }
    });
}

#[test]
fn category_deleting_dtors_match_the_original() {
    Twins::each(|t| {
        for sym in [0x0040_1260, 0x0040_1430, 0x0040_14e0, 0x0040_1600] {
            for flags in [0, 1, 2, 3, 0xffff_ff00, 0x0000_0101] {
                t.reset();
                t.stub(sd::OPERATOR_DELETE, false, 1, 0, 0x55);
                let this = t.words(&[0x1234_5678, 9]);
                let what = format!("deleting dtor {sym:#x} flags {flags:#x}");
                t.check(&what, sym, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(sb::category_deleting_dtor(m, this, flags))
                });
            }
        }
    });
}

#[test]
fn equivalent_value_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2468_ace1);
        for i in 0..40 {
            t.reset();
            let value = rng.word();
            let cat = rng.next();
            let pair = t.words(&[value, cat]);
            let cond_value = if rng.chance(2) { value } else { rng.word() };
            let cond_cat = if rng.chance(3) { rng.next() } else { cat };
            let cond = t.words(&[cond_value, cond_cat]);
            let this = t.object(&[(0, 0), (0, 0), (0, 0), (2, pair)]);
            let code = rng.word();
            let what = format!("equivalent_value #{i}");
            t.check_framed(
                &what,
                0x0040_1330,
                Conv::Thiscall(this),
                &[code, cond],
                8,
                |m, ebp| Out::U32(sb::equivalent_code(m, ebp, this, code, cond)),
            );
        }
    });
}

#[test]
fn equivalent_code_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1357_2468);
        for i in 0..40 {
            t.reset();
            let this = rng.next();
            let value = rng.word();
            let code = t.words(&[value, if rng.chance(3) { rng.next() } else { this }]);
            let cond = if rng.chance(2) { value } else { rng.word() };
            let what = format!("equivalent_code #{i}");
            t.check(
                &what,
                0x0040_1370,
                Conv::Thiscall(this),
                &[code, cond],
                |m| Out::U32(sb::equivalent_cond(m, this, code, cond)),
            );
        }
    });
}

/// The message functions: the map import answers `text` (0: unknown).
#[test]
fn category_messages_match_the_original() {
    Twins::each(|t| {
        for (sym, slot) in [
            (0x0040_13b0u32, sd::SYSERROR_MAP_IAT),
            (0x0040_1540, sd::WINERROR_MAP_IAT),
            (0x0040_1490, sd::SYSERROR_MAP_IAT),
        ] {
            for (case, code) in [0u32, 1, 2, 5, 0xffff_ffff].into_iter().enumerate() {
                for text in ["", "x", "file not found", "-"] {
                    t.reset();
                    let s = if text == "-" {
                        0
                    } else {
                        let mut b = text.as_bytes().to_vec();
                        b.push(0);
                        t.alloc(&b)
                    };
                    t.stub_import(slot, false, 1, 0, s);
                    t.stub(sd::STRING_ASSIGN, true, 2, 2, 0x99);
                    let out = t.words(&[0x1111_1111; 8]);
                    let what = format!("message {sym:#x} #{case} {text:?}");
                    t.check(&what, sym, Conv::Stdcall, &[out, code], |m| {
                        Out::U32(match sym {
                            0x0040_13b0 => sb::generic_message(m, out, code),
                            0x0040_1540 => sb::system_message(m, out, code),
                            _ => sb::iostream_message(m, out, code),
                        })
                    });
                }
            }
        }
    });
}

#[test]
fn default_condition_matches_the_original() {
    Twins::each(|t| {
        for (code, known) in [(0u32, 0u32), (2, 0x1000), (0xffff_ffff, 0), (5, 1)] {
            t.reset();
            t.stub_import(sd::SYSERROR_MAP_IAT, false, 1, 0, known);
            let out = t.words(&[7, 7]);
            let what = format!("default_condition {code:#x} {known:#x}");
            t.check(&what, 0x0040_15c0, Conv::Stdcall, &[out, code], |m| {
                Out::U32(sb::default_condition(m, out, code))
            });
        }
    });
}

// ---- the streams ----

#[test]
fn sentry_unlock_matches_the_original() {
    Twins::each(|t| {
        for (case, dist) in [0u32, 8, 0x68, 0x68].into_iter().enumerate() {
            t.reset();
            let buf = t.object(&[(0, 0), (0, 0), (0, 0x31)]);
            let vbtable = t.words(&[0, dist]);
            let stream = t.zeros(0x100);
            t.w32(stream, vbtable);
            let rdbuf = if case == 3 { 0 } else { buf };
            t.w32(stream + dist + 0x38, rdbuf);
            let holder = t.words(&[stream]);
            let what = format!("sentry_unlock #{case}");
            t.check(&what, 0x0040_1660, Conv::Thiscall(holder), &[], |m| {
                sb::sentry_unlock(m, holder);
                Out::None
            });
        }
    });
}

#[test]
fn stream_dtors_match_the_original() {
    Twins::each(|t| {
        for (sym, d) in [
            (0x0040_1840u32, sd::STRINGSTREAM_DTOR),
            (0x0040_22b0, sd::ISTRINGSTREAM_DTOR),
            (0x0040_29c0, sd::OSTRINGSTREAM_DTOR),
        ] {
            // the real distance, 0 (the vtable lands on the vbase
            // pointer: the original reads it again) and another
            for dist in [d.ios, 0, 0x10] {
                t.reset();
                t.stub(sd::STRINGBUF_DTOR, true, 0, 0, 0x41);
                t.stub_import(d.base_dtor, true, 0, 0, 0x42);
                t.stub_import(sd::IOS_DTOR_IAT, true, 0, 0, 0x43);
                let vbtable = t.words(&[0, dist]);
                let this = t.words(&[0x5555_5555; 0x40]);
                t.w32(this, vbtable);
                let what = format!("stream dtor {sym:#x} dist {dist:#x}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |m| {
                    Out::U32(sb::stream_dtor(m, this, d))
                });
            }
        }
    });
}

#[test]
fn vtordisp_thunks_match_the_original() {
    Twins::each(|t| {
        for disp in [0u32, 4, 0x60, 0xffff_fff0] {
            t.reset();
            t.stub_import(sd::ISTREAM_VTORDISP_IAT, true, 0, 0, 0x61);
            t.stub(sd::STRINGSTREAM_DELETING, true, 1, 1, 0x62);
            let obj = t.words(&[disp, 0, 0, 0]);
            let this = obj + 4;
            let what = format!("vtordisp_istream {disp:#x}");
            t.check(&what, 0x0040_5a77, Conv::Thiscall(this), &[], |m| {
                Out::U32(sb::vtordisp_istream(m, this))
            });
            for flags in [0, 1] {
                let what = format!("vtordisp_delete {disp:#x} {flags}");
                t.check(&what, 0x0040_5a83, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(sb::vtordisp_stringstream_delete(m, this, flags))
                });
            }
        }
    });
}

// ---- the settings ----

#[test]
fn settings_construct_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_f00d);
        for case in 0..4 {
            t.reset();
            let fill: Vec<u32> = (0..0x190).map(|_| rng.next()).collect();
            let this = t.words(&fill);
            let path = match case {
                0 => 0,
                1 => t.words(&[0]),
                _ => {
                    let w: Vec<u16> = "C:\\x\\GameSetting.ini\0".encode_utf16().collect();
                    let b: Vec<u8> = w.iter().flat_map(|c| c.to_le_bytes()).collect();
                    t.alloc(&b)
                }
            };
            let what = format!("Settings_Construct #{case}");
            t.check(&what, 0x0040_1880, Conv::Thiscall(this), &[path], |m| {
                Out::U32(st::construct(m, this, path))
            });
        }
    });
}

#[test]
fn settings_detect_quality_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7777_1234);
        let presets = t.rebase(settings::PRESETS);
        let mask = t.rebase(settings::PRESET_MASK);
        let masked: Vec<u32> = (0..24)
            .filter(|&k| Mem::read::<u8>(&t.a, mask + k) != 0)
            .collect();
        assert!(!masked.is_empty(), "the preset mask fixes some values");
        for case in 0..40 {
            t.reset();
            // a preset's values (0..2), then now and then one changed
            let level = rng.below(4);
            let mut v: Vec<u32> = (0..24)
                .map(|k| {
                    if level < 3 {
                        t.r32(presets + 16 * k + 4 * level)
                    } else {
                        rng.word()
                    }
                })
                .collect();
            if rng.chance(3) {
                let k = masked[rng.below(masked.len() as u32) as usize];
                v[k as usize] ^= 1 << rng.below(32);
            }
            if rng.chance(4) {
                // an unmasked value never matters
                let k = (0..24).find(|k| !masked.contains(k)).unwrap_or(0);
                v[k as usize] = rng.next();
            }
            let this = t.words(&v);
            let what = format!("Settings_DetectQuality #{case}");
            t.check(&what, 0x0040_2a20, Conv::Thiscall(this), &[], |m| {
                Out::U32(st::detect_quality(m, this))
            });
        }
    });
}

fn slots() -> Vec<u32> {
    let mut v: Vec<u32> = (0..0x60).collect();
    v.extend([
        0x7f,
        0x80,
        0x81,
        0xff,
        0x100,
        0xffff_ffff,
        0x8000_0000,
        0x1234_5629,
    ]);
    v
}

#[test]
fn key_name_tables_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f0f_1234);
        for (table, has, get, set) in [
            (st::KEYBOARD, 0x0040_5a90u32, 0x0040_5af0u32, 0x0040_5b30u32),
            (st::CONTROLLER, 0x0040_5ac0, 0x0040_5b10, 0x0040_5b60),
        ] {
            let base = t.rebase(table);
            if !t.snapshot {
                // the image's tables are empty until the menus fill them
                for i in 0..0x80 {
                    let v = if rng.chance(3) { 0 } else { rng.next() };
                    t.w32(base + 4 * i, v);
                }
            }
            for slot in slots() {
                t.reset();
                let what = format!("has {has:#x} {slot:#x}");
                t.check(&what, has, Conv::Cdecl, &[slot], |m| {
                    Out::U32(st::has_name(m, table, slot))
                });
                let what = format!("get {get:#x} {slot:#x}");
                t.check(&what, get, Conv::Cdecl, &[slot], |m| {
                    Out::U32(st::get_name(m, table, slot))
                });
                let name = rng.next();
                let what = format!("set {set:#x} {slot:#x}");
                t.check(&what, set, Conv::Cdecl, &[slot, name], |m| {
                    Out::U8(st::set_name(m, table, slot, name))
                });
            }
        }
        let what = "ReturnConst80";
        t.check(what, 0x0040_5b90, Conv::Cdecl, &[], |_| Out::U32(0x80));
    });
}

#[test]
fn binding_slot_filters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5a5a_1111);
        let mut all = slots();
        all.extend((0..64).map(|_| rng.next()));
        all.extend((0..64).map(|_| rng.next() & 0xffff_ff7f));
        for slot in all {
            for (sym, f) in [
                (0x0040_5cb0u32, st::is_ffx_slot as fn(u32) -> u32),
                (0x0040_5d00, st::is_ffx2_slot),
                (0x0040_5d80, st::is_lm_slot),
            ] {
                let what = format!("filter {sym:#x} {slot:#x}");
                t.check(&what, sym, Conv::Cdecl, &[slot], |_| Out::U32(f(slot)));
            }
        }
    });
}
