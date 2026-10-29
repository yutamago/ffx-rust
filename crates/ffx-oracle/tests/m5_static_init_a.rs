//! M5 stage 1, list `static-init-a` (`ffx_static_a`) against the original:
//! every row of each shape, on the image alone and on the snapshot
//! (`FFX_SNAPSHOT`). The original and the port run on twin oracles. The
//! callees that are not the initialiser itself (`atexit`, the scene pass
//! finder, the constructors) are recording stubs in both, so the call log in
//! memory compares the calls and their arguments; the globals and every other
//! byte written outside the stacks are compared by `diff`. Needs
//! `FFX_GAME_DIR`.
//!
//! Branch reach: the only branch is the find of a scene render pass type,
//! and both of its results run (the stub returns 0 and a pass type). The
//! other shapes are straight-line code. The counter and copy rows run with
//! several values in their source word (0, 1, all ones, random).

#![allow(clippy::cast_possible_truncation)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_static_a::{self as sa, rows};

/// `atexit`, the original's callee: a recording stub in both oracles.
const ATEXIT: u32 = sa::ATEXIT;
/// What the recording `atexit` returns (not 0, so the test sees that the
/// shim passes the result on).
const ATEXIT_RET: u32 = 0x7e57;
/// The call log: a count, then entries of ten words (tag, `ecx` or 0, eight
/// stack words).
const LOG_WORDS: u32 = 0x1000;
/// Bytes per log entry.
const ENTRY: u32 = 40;

const ALL: u32 = u32::MAX;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    /// A word: 0, 1, all ones or anything.
    fn word(&mut self) -> u32 {
        match self.next() % 4 {
            0 => 0,
            1 => 1,
            2 => 0xffff_ffff,
            _ => self.next(),
        }
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for calls.
struct G<'a>(&'a mut Oracle);

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

impl Host for G<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, false));
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

struct Twins {
    label: &'static str,
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries (see [`ENTRY`]).
    log: u32,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(label: &'static str, mut a: Oracle, mut b: Oracle) -> Self {
        a.set_max_instructions(5_000_000);
        b.set_max_instructions(5_000_000);
        let mut t = Self {
            label,
            a,
            b,
            log: 0,
        };
        t.log = t.zeros(LOG_WORDS as usize * 4);
        t.stub_fn(ATEXIT, false, 1, false, ATEXIT_RET);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<Self> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![Self::new(
            "image",
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push(Self::new(
                "snapshot",
                Oracle::from_snapshot(&exe, &pages).unwrap(),
                Oracle::from_snapshot(&exe, &pages).unwrap(),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
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

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// The original's copy of `at` (both oracles agree before a check).
    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    /// Replaces the original at `sym` by a recording stub in both oracles.
    /// The stub logs an entry into the call log: `sym`, `ecx` when `this`
    /// (else 0), and its first `n` stack words (at most eight), then returns
    /// `ret`. A callee that pops its words (`pops`) pops `4n`.
    fn stub_fn(&mut self, sym: u32, this: bool, n: u32, pops: bool, ret: u32) {
        assert!(n <= 8);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0x6b, 0xc0, ENTRY as u8]); // imul eax, eax, ENTRY
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov dword [eax], sym (the tag)
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax + 4], ecx
        } else {
            c.extend([0xc7, 0x40, 0x04, 0, 0, 0, 0]); // mov dword [eax + 4], 0
        }
        for i in 0..n {
            let from = (4 + 4 * i) as u8; // the argument, above the return address
            let to = (8 + 4 * i) as u8;
            c.extend([0x8b, 0x54, 0x24, from]); // mov edx, [esp + from]
            c.extend([0x89, 0x50, to]); // mov [eax + to], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        if pops && n > 0 {
            c.push(0xc2);
            c.extend(((4 * n) as u16).to_le_bytes()); // ret 4n
        } else {
            c.push(0xc3); // ret
        }
        let code_a = self.a.alloc_code(&c).unwrap();
        let code_b = self.b.alloc_code(&c).unwrap();
        assert_eq!(code_a, code_b);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code_a.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// Clears the call log in both oracles.
    fn reset_log(&mut self) {
        let log = self.log;
        self.w32(log, 0);
    }

    /// Runs the original at `sym` on `a` (cdecl, no arguments) and `port` on
    /// `b`; compares `eax & mask` and every byte written outside the stacks.
    fn check(&mut self, what: &str, sym: u32, mask: u32, port: impl FnOnce(&mut G) -> u32) {
        self.reset_log();
        let f = self.rebase(sym);
        let ra = self
            .a
            .call(f, Conv::Cdecl, &[])
            .unwrap_or_else(|e| panic!("{} {what}: original: {e}", self.label));
        let rb = port(&mut G(&mut self.b));
        assert_eq!(
            ra.eax & mask,
            rb & mask,
            "{} {what}: eax {:#x} vs {:#x}",
            self.label,
            ra.eax,
            rb
        );
        self.same(what);
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{} {what}: memory differs at {} bytes, first {:x?}",
            self.label,
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }
}

#[test]
fn atexit_only_initialisers_match_the_original() {
    for mut t in Twins::all() {
        for r in rows::DTOR_ONLY {
            let what = format!("dtor {:#x}", r.at);
            t.check(&what, r.at, ALL, |g| sa::dtor_only(g, r.at));
            // the destructor it registered, once
            let log = t.log;
            assert_eq!(t.r32(log), 1, "{} {what}: one atexit call", t.label);
        }
    }
}

#[test]
fn counter_initialisers_match_the_original() {
    let mut rng = Rng(0x2468_ace1);
    for mut t in Twins::all() {
        for r in rows::NEXT_ID {
            for _ in 0..4 {
                let v = rng.word();
                t.w32(t.rebase(r.counter), v);
                t.w32(t.rebase(r.dst), rng.word());
                let what = format!("next id {:#x} counter {v:#x}", r.at);
                t.check(&what, r.at, ALL, |g| sa::next_id(g, r.at));
            }
        }
    }
}

#[test]
fn zero_word_initialisers_match_the_original() {
    let mut rng = Rng(0x1357_9bdf);
    for mut t in Twins::all() {
        for r in rows::ZERO_WORD {
            t.w32(t.rebase(r.g), rng.word() | 1);
            t.check(&format!("zero {:#x}", r.at), r.at, 0, |g| {
                sa::zero_word(g, r.at);
                0
            });
            assert_eq!(t.r32(t.rebase(r.g)), 0, "{} zero {:#x}", t.label, r.at);
        }
    }
}

#[test]
fn copy_word_initialisers_match_the_original() {
    let mut rng = Rng(0x0bad_cafe);
    for mut t in Twins::all() {
        for r in rows::COPY_WORD {
            for _ in 0..3 {
                let v = rng.word();
                t.w32(t.rebase(r.src), v);
                t.w32(t.rebase(r.dst), rng.word());
                t.check(&format!("copy {:#x} value {v:#x}", r.at), r.at, ALL, |g| {
                    sa::copy_word(g, r.at)
                });
            }
        }
    }
}

/// Both branches of the find: `FindPass` returns 0 (the pass type is linked
/// in) and a non-zero pass type (nothing is linked). Two calls each run: the
/// find, then `atexit`.
#[test]
fn find_or_link_initialisers_match_the_original() {
    let mut rng = Rng(0x7777_1111);
    for mut t in Twins::all() {
        for found in [0, 0x5a5a_0000] {
            t.stub_fn(sa::FIND_PASS, false, 1, false, found);
            for r in rows::FIND_LINK {
                for _ in 0..2 {
                    t.w32(t.rebase(r.head), rng.word());
                    t.w32(t.rebase(r.dst), rng.word());
                    let what = format!("find-link {:#x} found {found:#x}", r.at);
                    t.check(&what, r.at, ALL, |g| sa::find_link(g, r.at));
                    let log = t.log;
                    assert_eq!(t.r32(log), 2, "{} {what}: the find, then atexit", t.label);
                }
            }
        }
    }
}

/// The constructor (a thiscall, with its one argument when the row has one),
/// then `atexit`: two calls, each logged.
#[test]
fn call_then_destructor_initialisers_match_the_original() {
    for mut t in Twins::all() {
        for r in rows::CALL_DTOR {
            let n = u32::from(r.arg.is_some());
            t.stub_fn(r.callee, true, n, true, 0x1234);
            let what = format!("call {:#x} -> {:#x}", r.at, r.callee);
            t.check(&what, r.at, ALL, |g| sa::call_dtor(g, r.at));
            let log = t.log;
            assert_eq!(
                t.r32(log),
                2,
                "{} {what}: the constructor, then atexit",
                t.label
            );
        }
    }
}

/// The eight-word constructor: the thiscall's eight words (the image addresses
/// rebased) are logged in order, then `atexit`.
#[test]
fn ctor8_initialisers_match_the_original() {
    for mut t in Twins::all() {
        for r in rows::CTOR8 {
            t.stub_fn(sa::CTOR8, true, 8, true, 0x4242);
            let what = format!("ctor8 {:#x}", r.at);
            t.check(&what, r.at, ALL, |g| sa::ctor8(g, r.at));
            let log = t.log;
            assert_eq!(
                t.r32(log),
                2,
                "{} {what}: the constructor, then atexit",
                t.label
            );
        }
    }
}
