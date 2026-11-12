//! `PRendering`, the post-processing classes and the renderer's
//! display-mode and present code, list `phyre-render-b`
//! (`ffx_prender::m5p_render_b*`, M5 stage 1), against the original.
//! Almost none of this code ran in a coverage run, so every case is
//! synthetic memory built for its branch, on the image alone and over a
//! snapshot (`FFX_SNAPSHOT`, run once with each of field.snap and
//! battle.snap). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! Callees outside the list are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its stack words
//! to a log in memory, so the diff checks that both made the same calls in
//! the same order with the same arguments.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    dead_code // helpers the later batches use
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::m5p_render_b::{self as rb, bridge as b};

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
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
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0_f32()
            .unwrap_or_else(|| panic!("callee {addr:#x} left no ST0"))
    }
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0_f64()
            .unwrap_or_else(|| panic!("callee {addr:#x} left no ST0"))
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // through the trampoline (big-phyre3): `fld qword [esp+4]; call
        // [esp+0xc]; ret`, then `fstp st(0); ret` to pop the result
        let at = TRAMP.with(std::cell::Cell::get);
        assert_ne!(at, 0, "intrinsic {addr:#x}: no trampoline");
        let b = x.to_bits();
        let r = self
            .0
            .call(at, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let v = r
            .st0_f32()
            .unwrap_or_else(|| panic!("intrinsic {addr:#x}: no float in ST0"));
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
}

/// The CRT intrinsics' trampoline (as `m5p_scene.rs`'s): at `+0` `fld
/// qword [esp+4]; call [esp+0xc]; ret`, at `+16` `fstp st(0); ret`.
const INTRINSIC_TRAMPOLINE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

thread_local! {
    /// The trampoline of the oracle a [`G`] is running on (0: none).
    static TRAMP: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read (the original's `eax` is the caller's).
    None,
    /// `eax`.
    U32(u32),
    /// `edx:eax`.
    U64(u64),
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(20_000_000);
        b.set_max_instructions(20_000_000);
        let mut t = Self {
            a,
            b,
            log: 0,
            saved: HashMap::new(),
        };
        t.log = t.zeros(0x4000);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                ),
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

    /// `n` bytes of a known non-zero pattern.
    fn junk(&mut self, n: usize, seed: u8) -> u32 {
        let v: Vec<u8> = (0..n)
            .map(|i| (i as u8).wrapping_mul(37) ^ seed ^ 0x5a)
            .collect();
        self.alloc(&v)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// Clears the log and puts back every stubbed original.
    fn reset(&mut self) {
        let log = self.log;
        self.w32(log, 0);
        let saved: Vec<(u32, Vec<u8>)> = self.saved.drain().collect();
        for (at, bytes) in saved {
            self.write(at, &bytes);
        }
        self.diff("reset");
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this`, and `n` stack words; pops `pops` words; returns `ret`.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        assert!(n <= 6);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
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
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// A recording stub (as [`Twins::stub_code`], cdecl) that leaves the
    /// `float` with `bits` in `ST0`, placed at the original `sym`.
    fn stub_float(&mut self, sym: u32, n: u32, bits: u32) {
        let at = self.rebase(sym);
        let cell = self.alloc(&bits.to_le_bytes());
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0xd9, 0x05]);
        c.extend(cell.to_le_bytes()); // fld dword [cell]
        c.push(0xc3);
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there),
    /// logged with `sym` as its tag.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// An object whose vtable holds `slots` recording stubs (`this`
    /// logged, slot `i` tagged `0x7000 + i`, taking and popping `words[i]`
    /// words, returning `rets[i]`, or `0x1000 + i` past its end). Returns
    /// the object (0x40 bytes, after its vtable pointer, zero).
    fn object(&mut self, words: &[u32], rets: &[u32]) -> u32 {
        let mut vt = Vec::new();
        for (i, &n) in words.iter().enumerate() {
            let r = rets.get(i).copied().unwrap_or(0x1000 + i as u32);
            let s = self.stub_code(0x7000 + i as u32, true, n, n, r);
            vt.extend(s.to_le_bytes());
        }
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory differs at {} bytes, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `port` on `b`; compares the
    /// result as `port` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b));
        // DBG_LOG=1: both call logs, for a case that differs
        if std::env::var_os("DBG_LOG").is_some() {
            for (n, o) in [("a", &self.a), ("b", &self.b)] {
                let c: u32 = Mem::read(o, self.log);
                eprintln!("{n}: eax {:#x} count {c}", ra.eax);
                for i in 0..c.min(24) {
                    let e: [u32; 8] = Mem::read(o, self.log + 4 + 32 * i);
                    eprintln!("  {e:x?}");
                }
            }
        }
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::U64(v) => assert_eq!(
                u64::from(ra.edx) << 32 | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
        }
        self.diff(what);
    }
}

// ---- batch 1: the generated template shapes ----

/// Every destructor body against its own original: the parent stubbed
/// (it logs `this`), the object junk.
#[test]
fn vt_tails_match_the_original() {
    assert_eq!(rb::VT_TAILS.len(), 59);
    for (label, mut t) in Twins::all() {
        for &(f, vt, parent) in &rb::VT_TAILS {
            assert_eq!(rb::vt_tail_of(f), (vt, parent));
            t.stub(parent, true, 0, 0, 0x7a11_0000 ^ f);
            let obj = t.junk(0x20, f as u8);
            let what = format!("{label}: vt tail {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                Out::U32(b::vt_tail(g, obj, vt, parent))
            });
            t.reset();
        }
    }
}

/// Every array release on each of its paths: empty, not owned (bit 31),
/// owned with a null and with a real buffer.
#[test]
fn releases_match_the_original() {
    assert_eq!(rb::RELEASES.len(), 13);
    for (label, mut t) in Twins::all() {
        for &(f, kind) in &rb::RELEASES {
            assert_eq!(rb::release_of(f), kind);
            for (count, buffer) in [
                (0, 0),
                (0, 0x1234_5678),
                (3, 0),
                (3, 0x00c0_ffee),
                (0x8000_0003, 0),
                (0x8000_0003, 0x00c0_ffee),
                (0x8000_0000, 0x00c0_ffee),
                (0x7fff_ffff, 0x0bad_f00d),
                (0xffff_ffff, 0x0bad_f00d),
            ] {
                t.stub(rb::PHYRE_FREE, false, 1, 0, 0xf4ee_0000 | count & 0xff);
                t.stub(rb::FREE_RECORDS, false, 2, 0, 0x5ec0_0000 | count & 0xff);
                let arr = t.junk(0x10, count as u8);
                t.w32(arr, count);
                t.w32(arr + 4, buffer);
                let what = format!("{label}: release {f:#x} {kind:?} {count:#x}/{buffer:#x}");
                t.check(&what, f, Conv::Thiscall(arr), &[], |g| {
                    Out::U32(b::release(g, arr, kind))
                });
                t.reset();
            }
        }
    }
}

/// The unsupported state setters: `0x17`, popping their words.
#[test]
fn not_supported_match_the_original() {
    assert_eq!(rb::NOT_SUPPORTED.len(), 10);
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x20, 3);
        for &(f, n) in &rb::NOT_SUPPORTED {
            assert_eq!(rb::not_supported_words(f), n);
            let args: Vec<u32> = (0..n).map(|i| 0x1111_1111 * (i + 1)).collect();
            let what = format!("{label}: not supported {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &args, |_| {
                Out::U32(rb::E_NOT_SUPPORTED)
            });
        }
    }
}

/// The element copies and the zeroing: none, one, several, a null first
/// destination, overlapping ranges both ways.
#[test]
fn copies_match_the_original() {
    assert_eq!(rb::COPIES.len(), 5);
    for (label, mut t) in Twins::all() {
        for &(f, size) in &rb::COPIES {
            assert_eq!(rb::copy_size_of(f), size);
            for (case, count) in [("none", 0u32), ("one", 1), ("three", 3), ("five", 5)] {
                let src = t.junk(0x200, 0x31);
                let dst = t.junk(0x200, 0x77);
                let what = format!("{label}: copy {f:#x} {case}");
                t.check(&what, f, Conv::Cdecl, &[dst, src, count], |g| {
                    b::copy(g, dst, src, count, size);
                    Out::None
                });
            }
            // a null destination: nothing copied, the source stays
            let src = t.junk(0x40, 0x13);
            let what = format!("{label}: copy {f:#x} null");
            t.check(&what, f, Conv::Cdecl, &[0, src, 1], |g| {
                b::copy(g, 0, src, 1, size);
                Out::None
            });
            // overlapping: the destination one word above and below
            for shift in [4u32, 0u32.wrapping_sub(4), size / 2] {
                let buf = t.junk(0x200, 0x55);
                let src = buf + 0x80;
                let dst = src.wrapping_add(shift);
                let what = format!("{label}: copy {f:#x} overlap {shift:#x}");
                t.check(&what, f, Conv::Cdecl, &[dst, src, 3], |g| {
                    b::copy(g, dst, src, 3, size);
                    Out::None
                });
            }
        }
        for (case, dst_off, count) in [("none", 0x10, 0u32), ("one", 0x10, 1), ("four", 0x10, 4)] {
            let buf = t.junk(0x100, 0x42);
            let dst = buf + dst_off;
            let what = format!("{label}: zero16 {case}");
            t.check(&what, rb::ZERO16, Conv::Cdecl, &[dst, count], |g| {
                b::zero16(g, dst, count);
                Out::None
            });
        }
        let what = format!("{label}: zero16 null");
        t.check(&what, rb::ZERO16, Conv::Cdecl, &[0, 1], |g| {
            b::zero16(g, 0, 1);
            Out::None
        });
    }
}

// ---- batch 2: helpers ----

impl Twins {
    /// An object whose vtable holds `words.len()` recording stdcall stubs
    /// (COM: `this` is the first stack word, `ecx` not logged), slot `i`
    /// tagged `0xc000 + i`, logging and popping `words[i]` words,
    /// returning `rets[i]` (or `0x2000 + i`). Returns the object.
    fn com_object(&mut self, words: &[u32], rets: &[u32]) -> u32 {
        let mut vt = Vec::new();
        for (i, &n) in words.iter().enumerate() {
            let r = rets.get(i).copied().unwrap_or(0x2000 + i as u32);
            let s = self.stub_code(0xc000 + i as u32, false, n.min(6), n, r);
            vt.extend(s.to_le_bytes());
        }
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }

    /// The import in IAT slot `slot` replaced by a recording stub (tag
    /// `slot`), put back by [`Twins::reset`].
    fn stub_import(&mut self, slot: u32, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(slot);
        let code = self.stub_code(slot, false, n, pops, ret);
        let old = Oracle::read(&mut self.a, at, 4).unwrap();
        self.saved.entry(at).or_insert(old);
        self.w32(at, code);
    }

    /// `al` of the original against the port's `u8`.
    fn check_al(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u8,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let v = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al");
        self.diff(what);
    }
}

use ffx_prender::m5p_render_b_post::{self as post, bridge as pb};
use ffx_prender::m5p_render_b_small::{self as small, bridge as sb};

/// Floats for the x87 forwarders: no signalling NaNs (the emulator keeps
/// them signalling, the CPU quiets them).
const FWD_FLOATS: [u32; 8] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000,
    0xc2c8_0000,
    0x7f80_0000,
    0x7fc0_0000,
    0xffc1_2345,
    0x0000_0001,
];

// ---- batch 2: the post-processing effects ----

#[test]
fn post_small_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the base destructor body, the setup mark, the material in use
        let obj = t.junk(0x40, 1);
        t.check(
            &format!("{label}: base dtor"),
            0x009f_e780,
            Conv::Thiscall(obj),
            &[],
            |g| {
                pb::base_dtor(g, obj);
                Out::None
            },
        );
        t.check(
            &format!("{label}: mark set up"),
            0x00a1_9c00,
            Conv::Thiscall(obj),
            &[7, 8],
            |g| Out::U32(pb::mark_set_up(g, obj)),
        );
        t.check(
            &format!("{label}: use material"),
            0x00a1_b810,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(pb::use_material(g, obj)),
        );
        t.check(
            &format!("{label}: base render"),
            0x00a1_b9a0,
            Conv::Thiscall(obj),
            &[1, 2],
            |_| Out::U32(0),
        );
        t.check_al(
            &format!("{label}: DL runs in pass"),
            0x00a1_b180,
            Conv::Thiscall(obj),
            &[1, 2],
            |_| 0,
        );
        // can run: no handle, an empty handle, flag clear, all set
        for (handle_word, flag) in [
            (None, 1u8),
            (Some(0), 1),
            (Some(0x55), 0),
            (Some(0x55), 1),
            (Some(0x55), 0x80),
        ] {
            let obj = t.junk(0x40, 2);
            let handle = match handle_word {
                None => 0,
                Some(w) => {
                    let hd = t.junk(0x10, 3);
                    t.w32(hd, w);
                    hd
                }
            };
            t.w32(obj + 8, handle);
            t.w8(obj + 0x11, flag);
            let what = format!("{label}: can run {handle_word:?} {flag}");
            t.check(&what, 0x00a1_b200, Conv::Thiscall(obj), &[], |g| {
                Out::U32(pb::can_run(g, obj))
            });
        }
        // runs in pass: slot +0x10's al 0 or not, the pass types, flags
        let base = t.rebase(post::PASS_BASE);
        let ssr = t.rebase(post::PASS_SSR);
        for (r10, pass, flag) in [
            (0x1234_5600, base, 0xff),
            (0x1234_5601, base, 0x0f0f_0f81),
            (0x1234_5601, ssr, 0xff),
            (0xffff_ff80, base, 0x7e),
            (0x0000_0001, 0, 1),
        ] {
            let obj = t.object(&[2, 2, 0, 2, 0, 2, 5], &[0, 0, 0, 0, r10]);
            let what = format!("{label}: runs in pass {r10:#x} {pass:#x} {flag:#x}");
            t.check(
                &what,
                0x00a1_b190,
                Conv::Thiscall(obj),
                &[pass, flag],
                |g| Out::U32(pb::runs_in_pass(g, obj, pass, flag)),
            );
            let what = format!("{label}: SSR runs in pass {pass:#x} {flag:#x}");
            t.check_al(
                &what,
                0x00a1_b1c0,
                Conv::Thiscall(obj),
                &[pass, flag],
                |g| pb::ssr_runs_in_pass(g, pass, flag),
            );
            t.reset();
        }
        // set up once: set up already, a record or none, the slot's result
        for (set_up, with_record, r4) in [
            (1u8, true, 5),
            (0, true, 0),
            (0, true, 0x77),
            (0, false, 9),
            (0, false, 0),
        ] {
            let obj = t.object(&[1, 2, 0, 2, 0, 2, 5], &[0, r4]);
            t.w8(obj + 0x10, set_up);
            let record = if with_record {
                let r = t.junk(0x40, 9);
                t.w32(r + 0x1c, 640);
                t.w32(r + 0x20, 360);
                r
            } else {
                0
            };
            let what = format!("{label}: set up once {set_up} {with_record} {r4:#x}");
            t.check(
                &what,
                0x00a2_1cc0,
                Conv::Thiscall(obj),
                &[1, 2, record, 4, 5],
                |g| Out::U32(pb::set_up_once(g, obj, record)),
            );
            t.reset();
        }
        // init shaders: handle paths, the callee's result
        for &(f, init) in &post::INIT_SHADERS {
            assert_eq!(post::init_shaders_of(f), init);
            for (handle_word, r) in [
                (None, 1),
                (Some(0), 1),
                (Some(0x66), 0),
                (Some(0x66), 0x8000_0001),
            ] {
                t.stub(post::MATERIAL_OF, true, 0, 0, 0x4d47_0000);
                t.stub(init, true, 2, 2, r);
                let obj = t.junk(0x40, 4);
                let handle = match handle_word {
                    None => 0,
                    Some(w) => {
                        let hd = t.junk(0x10, 5);
                        t.w32(hd, w);
                        hd
                    }
                };
                t.w32(obj + 8, handle);
                let what = format!("{label}: init shaders {f:#x} {handle_word:?} {r:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(pb::init_shaders(g, obj, init))
                });
                t.reset();
            }
        }
        // render once set up
        for &(f, render) in &post::RENDER_IF_SET_UP {
            assert_eq!(post::render_of(f), render);
            for (set_up, r) in [(0u8, 3), (1, 0), (2, 0x99)] {
                t.stub(render, true, 2, 2, r);
                let obj = t.junk(0x40, 6);
                t.w8(obj + 0x10, set_up);
                let what = format!("{label}: render {f:#x} {set_up} {r:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[0x11, 0x22], |g| {
                    Out::U32(pb::render_if_set_up(g, obj, render, 0x11, 0x22))
                });
                t.reset();
            }
        }
        // the utility's teardown
        t.stub(post::RENDER_TARGET_RELEASE, true, 0, 0, 0x5150);
        t.check(
            &format!("{label}: utility terminate"),
            0x00a2_4e70,
            Conv::Thiscall(0),
            &[],
            |g| Out::U32(pb::utility_terminate(g)),
        );
        t.reset();
        // the samplers: each flag combination, the flag bit's neighbours
        for (f1, f2) in [
            (0u32, 0u32),
            (0x80, 0),
            (0, 0x80),
            (0xffff_ff7f, 0x180),
            (0x80, 0xffff_ffff),
            (0x80, 0x100),
            (0, 0xffff_ff7f),
        ] {
            t.stub(post::SAMPLER_RELEASE, true, 0, 0, 0x5a5a_0000 | f2 & 0xff);
            let obj = t.junk(0x60, 7);
            t.w32(obj + 0x1c, f1);
            t.w32(obj + 0x40, f2);
            let what = format!("{label}: samplers {f1:#x} {f2:#x}");
            t.check(&what, 0x00a2_4d60, Conv::Thiscall(obj), &[], |g| {
                Out::U32(pb::release_samplers(g, obj))
            });
            t.reset();
        }
        // the input accessors
        for w in [0u32, 1, 2, 0xffff_ffff, 0x8000_0001] {
            let obj = t.junk(0x10, 8);
            t.w32(obj + 4, w);
            t.check(
                &format!("{label}: word even {w:#x}"),
                0x009e_cc70,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(pb::input_word_even(g, obj)),
            );
        }
        let data = t.junk(0x40, 9);
        for (count, i) in [
            (3u32, 0u32),
            (3, 2),
            (3, 3),
            (0x8000_0003, 2),
            (0x8000_0003, 0x8000_0000),
            (0, 0),
        ] {
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, data);
            t.check(
                &format!("{label}: array at {count:#x} {i:#x}"),
                0x009e_cc80,
                Conv::Thiscall(arr),
                &[i],
                |g| Out::U32(pb::array_at(g, arr, i)),
            );
        }
        let table = t.zeros(0x10);
        let e = t.junk(0x10, 10);
        t.w32(table, e);
        t.w32(table + 8, e + 4);
        let obj = t.zeros(0x10);
        t.w32(obj + 0xc, table);
        for i in [0u32, 1, 2] {
            t.check(
                &format!("{label}: entry +4 {i}"),
                0x009e_ca80,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(pb::entry_plus4(g, obj, i)),
            );
            if i != 1 {
                t.check(
                    &format!("{label}: entry word 4 {i}"),
                    0x009e_caa0,
                    Conv::Thiscall(obj),
                    &[i],
                    |g| Out::U32(pb::entry_word4(g, obj, i)),
                );
            }
        }
    }
}

// ---- batch 2: the renderer's small methods ----

#[test]
fn render_small_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.check(
            &format!("{label}: empty"),
            0x0061_42f0,
            Conv::Cdecl,
            &[],
            |_| Out::None,
        );
        for w in [0u32, 1, 0x8000_0000] {
            let obj = t.junk(0x10, 1);
            t.w32(obj, w);
            t.check(
                &format!("{label}: can kick {w:#x}"),
                0x0061_47e0,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(sb::can_kick(g, obj)),
            );
        }
        let obj = t.junk(0x10, 2);
        t.check(
            &format!("{label}: clear word 0"),
            0x0061_3fb0,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(sb::clear_word0(g, obj)),
        );
        for b in [0u32, 5, 0x8000_0005, 0xffff_ffff] {
            let obj = t.junk(0x10, 3);
            t.check(
                &format!("{label}: init external {b:#x}"),
                0x0061_3f30,
                Conv::Thiscall(obj),
                &[0xabcd, b],
                |g| Out::U32(sb::init_external(g, obj, 0xabcd, b)),
            );
        }
        for with in [false, true] {
            let obj = t.junk(0x10, 4);
            let p = if with { t.junk(0x10, 5) } else { 0 };
            t.w32(obj, p);
            t.check(
                &format!("{label}: owner word 4 {with}"),
                0x005b_4930,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(sb::clear_owner_word4(g, obj)),
            );
            t.check(
                &format!("{label}: word 4 {with}"),
                0x006e_2e20,
                Conv::Thiscall(0x1234),
                &[p],
                |g| Out::U32(sb::clear_word4(g, p)),
            );
        }
        t.stub(small::EH_VECTOR_DTOR, false, 4, 4, 0xe4e4);
        let obj = t.junk(0x20, 6);
        t.check(
            &format!("{label}: ring elements"),
            0x0061_42d0,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(sb::ring_elements_dtor(g, obj)),
        );
        t.reset();
        // the render-interface setters
        t.stub(small::SET_STATE_570, true, 2, 2, 0x570);
        let obj = t.junk(0x700, 7);
        t.check(
            &format!("{label}: state 570"),
            0x0059_9810,
            Conv::Thiscall(obj),
            &[3, 4],
            |g| Out::U32(sb::set_state_570(g, obj, 3, 4)),
        );
        t.reset();
        for v in [0u32, 1, 0x100, 0xffff_ff00, 2] {
            t.check(
                &format!("{label}: state 5a0 {v:#x}"),
                0x0059_a1c0,
                Conv::Thiscall(obj),
                &[v],
                |g| Out::U32(sb::set_5a0(g, obj, v)),
            );
            t.check(
                &format!("{label}: state 574 {v:#x}"),
                0x0059_a8d0,
                Conv::Thiscall(obj),
                &[v],
                |g| Out::U32(sb::set_574(g, obj, v)),
            );
        }
        for m in 0..16u32 {
            let rgba = [
                m & 1,
                ((m >> 1) & 1) * 0x300,
                ((m >> 2) & 1) * 0x80,
                ((m >> 3) & 1) * 0xff,
            ];
            let rgba = [rgba[0] | 0x100, rgba[1], rgba[2] | 0x200, rgba[3]];
            t.check(
                &format!("{label}: write mask {m}"),
                0x0059_9ed0,
                Conv::Thiscall(obj),
                &rgba,
                |g| Out::U32(sb::set_write_mask(g, obj, rgba)),
            );
        }
        // the x87 forwarders
        for (k, &x) in FWD_FLOATS.iter().enumerate() {
            let y = FWD_FLOATS[(k + 3) % FWD_FLOATS.len()];
            for (f, callee) in [(0x0061_5cd0, small::FWD_3_A), (0x0061_5da0, small::FWD_3_B)] {
                t.stub(callee, true, 3, 3, 0x17);
                t.check(
                    &format!("{label}: forward3 {f:#x} {x:#x}"),
                    f,
                    Conv::Thiscall(obj),
                    &[9, x, y],
                    |g| Out::U32(sb::forward_3(g, callee, obj, 9, x, y)),
                );
                t.reset();
            }
            let v = [x, y, FWD_FLOATS[(k + 5) % 8], FWD_FLOATS[(k + 6) % 8]];
            t.stub(small::FWD_4, true, 4, 4, 0x17);
            t.check(
                &format!("{label}: forward4 {x:#x}"),
                0x0061_5b50,
                Conv::Thiscall(obj),
                &v,
                |g| Out::U32(sb::forward_4(g, obj, v)),
            );
            t.reset();
        }
        // the pass state
        for (owner, state) in [(false, false), (true, false), (true, true)] {
            t.stub(small::PASS_STATE_APPLY, true, 1, 1, 0xa991);
            let obj = t.junk(0x20, 8);
            let o = if owner { t.junk(0x20, 9) } else { 0 };
            if owner {
                let s = if state { t.junk(0x10, 10) } else { 0 };
                t.w32(o + 0x14, s);
            }
            t.w32(obj, o);
            t.check(
                &format!("{label}: pass state {owner} {state}"),
                0x0061_c870,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(sb::apply_pass_state(g, obj)),
            );
            t.reset();
        }
        // present: no swap chain, sync off and on
        for (chain, sync) in [(false, 0u8), (true, 0), (true, 1), (true, 0x80)] {
            let obj = t.junk(0x1400, 11);
            let c = if chain {
                t.com_object(
                    &[0, 0, 0, 0, 0, 0, 0, 0, 3],
                    &[0, 0, 0, 0, 0, 0, 0, 0, 0x8876_0000],
                )
            } else {
                0
            };
            t.w32(obj + 0x74, c);
            t.w8(obj + 0x131c, sync);
            t.check_al(
                &format!("{label}: present {chain} {sync}"),
                0x0059_3760,
                Conv::Thiscall(obj),
                &[],
                |g| sb::present_flip(g, obj),
            );
        }
        // the context call +0xa4
        let mut words = vec![0u32; 42];
        words[41] = 4;
        let ctx = t.com_object(&words, &[]);
        let obj = t.junk(0x80, 12);
        t.w32(obj + 0x70, ctx);
        t.check(
            &format!("{label}: context a4"),
            0x0059_2b00,
            Conv::Thiscall(obj),
            &[5, 6, 7],
            |g| Out::U32(sb::com_a4(g, obj, 5, 6, 7)),
        );
        // the COM pairs: each subset
        for m in 0..16u32 {
            let obj = t.junk(0x10, 13);
            for k in 0..4u32 {
                let o = if (m >> k) & 1 != 0 {
                    t.com_object(&[0, 0, 1], &[0, 0, 7 + k])
                } else {
                    0
                };
                t.w32(obj + 4 * k, o);
            }
            t.check(
                &format!("{label}: com pairs {m}"),
                0x0059_c330,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    sb::release_pairs(g, obj);
                    Out::None
                },
            );
        }
        // the critical sections (eax only where the original sets it)
        for with in [false, true] {
            t.stub(small::CLOSE_HANDLE_AT, true, 0, 0, 0);
            t.stub(small::RENDER_THREAD_INSTANCE, false, 0, 0, 0x7e7e_0000);
            t.stub(small::RENDER_THREAD_SYNC, true, 0, 0, 0x5c5c);
            let obj = t.junk(0x60, 14);
            let cs = if with { t.junk(0x10, 15) } else { 0 };
            t.w32(obj + 0x44, cs);
            t.check(
                &format!("{label}: close section {with}"),
                0x005b_c440,
                Conv::Thiscall(obj),
                &[],
                |g| match sb::close_section(g, obj) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
            t.w32(obj + 0x44, cs);
            t.check(
                &format!("{label}: sync and close {with}"),
                0x005b_c400,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(sb::sync_and_close(g, obj)),
            );
            t.reset();
        }
        // the scene-wide capture buffer: made, missing inputs, present
        for (have, params, camera) in [
            (0u32, true, true),
            (0, false, true),
            (0, true, false),
            (0x5858, true, true),
        ] {
            t.stub(small::SCENE_WIDE_GET_BUFFER, true, 3, 3, 0xb0b0);
            let obj = t.junk(0x80, 16);
            t.w32(obj + 0x58, have);
            let p = if params { t.junk(0x10, 17) } else { 0 };
            let c = if camera { t.junk(0x10, 18) } else { 0 };
            t.w32(obj + 0x54, p);
            t.w32(obj + 0x5c, c);
            t.check(
                &format!("{label}: scene wide {have:#x} {params} {camera}"),
                0x005b_86d0,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(sb::scene_wide_capture(g, obj)),
            );
            t.reset();
        }
        // the PSharray word
        let elem = t.junk(0x20, 19);
        for (count, i) in [
            (2u32, 0u32),
            (2, 1),
            (2, 2),
            (0x8000_0002, 1),
            (0x8000_0002, 5),
            (0, 0),
        ] {
            t.stub(small::SHARRAY_ELEMENT, true, 1, 1, elem);
            let obj = t.junk(0x40, 20);
            t.w32(obj + 0x24, count);
            t.check(
                &format!("{label}: sharray {count:#x} {i}"),
                0x0059_3fe0,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(sb::sharray_word_c(g, obj, i)),
            );
            t.reset();
        }
        // the jobs: none, a null array, three
        for (null, n) in [(true, 2u32), (false, 0), (false, 3)] {
            t.stub(small::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let jobs = if null {
                0
            } else {
                let jobs = t.zeros((small::JOB_SIZE * 3) as usize);
                for k in 0..3 {
                    let o = t.object(&[1], &[k]);
                    let vt = t.r32(o);
                    t.w32(jobs + k * small::JOB_SIZE, vt);
                }
                jobs
            };
            t.check(
                &format!("{label}: free jobs {null} {n}"),
                0x005b_5330,
                Conv::Cdecl,
                &[jobs, n],
                |g| {
                    sb::free_jobs(g, jobs, n);
                    Out::None
                },
            );
            t.reset();
        }
        // the deleting destructors, both flag values
        for flags in [0u32, 1, 2, 0xffff_ffff] {
            t.stub(small::OPERATOR_DELETE, false, 1, 0, 0xde1e);
            t.stub(small::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(small::RENDERER_JOB_DTOR, true, 0, 0, 0x10b);
            t.stub(small::RING_JOB_DTOR, true, 0, 0, 0x20b);
            t.stub(small::AVOID_OVERWRITE_PARENT, true, 0, 0, 0x30b);
            let obj = t.junk(0x20, 21);
            t.check(
                &format!("{label}: job delete {flags:#x}"),
                0x005b_5080,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(sb::job_delete(g, obj, flags, small::RENDERER_JOB_DTOR)),
            );
            t.check(
                &format!("{label}: ring job delete {flags:#x}"),
                0x005b_50b0,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(sb::job_delete(g, obj, flags, small::RING_JOB_DTOR)),
            );
            t.check(
                &format!("{label}: acceptance delete {flags:#x}"),
                0x0061_6170,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(sb::acceptance_delete(g, obj, flags)),
            );
            for buffer in [0u32, 0xb0ff] {
                t.w32(obj + 4, buffer);
                t.check(
                    &format!("{label}: allocator delete {flags:#x} {buffer:#x}"),
                    0x0061_61a0,
                    Conv::Thiscall(obj),
                    &[flags],
                    |g| Out::U32(sb::allocator_delete(g, obj, flags)),
                );
            }
            t.check(
                &format!("{label}: avoid delete {flags:#x}"),
                0x0061_65c0,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(sb::avoid_overwrite_delete(g, obj, flags)),
            );
            t.reset();
        }
        let obj = t.junk(0x20, 22);
        t.check(
            &format!("{label}: avoid ctor"),
            0x0061_64a0,
            Conv::Thiscall(obj),
            &[0x1a, 0x2b],
            |g| Out::U32(sb::avoid_overwrite_ctor(g, obj, 0x1a, 0x2b)),
        );
        // the managers' teardown, with and without an instance
        for with in [false, true] {
            t.stub(small::OPERATOR_DELETE, false, 1, 0, 0xde1e);
            t.stub(small::CHARACTER_LIGHTING_DTOR, true, 0, 0, 0xc1);
            t.stub(small::TEXT_UTIL_ENTRY_DTOR, true, 0, 0, 0xe1);
            for g in [
                small::CHARACTER_LIGHTING,
                small::CHARACTER_LIGHTING_TWEAK,
                small::FOG_EFFECT_MANAGER,
            ] {
                let p = if with { t.junk(0x20, 23) } else { 0 };
                let at = t.rebase(g);
                t.w32(at, p);
            }
            t.check(
                &format!("{label}: lighting finalize {with}"),
                0x006a_6dd0,
                Conv::Cdecl,
                &[],
                |g| {
                    sb::character_lighting_finalize(g);
                    Out::None
                },
            );
            t.check(
                &format!("{label}: tweak finalize {with}"),
                0x006a_6e00,
                Conv::Cdecl,
                &[],
                |g| {
                    sb::tweak_util_finalize(g);
                    Out::None
                },
            );
            t.check(
                &format!("{label}: fog delete {with}"),
                0x006e_32a0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(sb::fog_manager_delete(g)),
            );
            t.reset();
        }
        t.stub(small::TEXT_UTIL_TERMINATE, true, 0, 0, 0x7e);
        t.stub(small::OPERATOR_DELETE_ARRAY, false, 1, 0, 0xda);
        let obj = t.junk(0x100, 24);
        t.check(
            &format!("{label}: text util release"),
            0x006a_b5c0,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(sb::text_util_release(g, obj)),
        );
        t.reset();
        for k in [0u32, 1, 2, 3, 4, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
            t.check(
                &format!("{label}: fog fade {k:#x}"),
                0x006e_36c0,
                Conv::Thiscall(obj),
                &[k],
                |g| Out::U32(sb::set_fog_fade_type(g, obj, k)),
            );
        }
        // Iggy
        t.stub_import(small::IGGY_SHUTDOWN, 0, 0, 0x16);
        t.check(
            &format!("{label}: iggy terminate"),
            0x00a3_e720,
            Conv::Thiscall(0),
            &[],
            |g| Out::U32(sb::iggy_terminate(g)),
        );
        t.reset();
        t.stub(small::TTY_DISPATCH, false, 2, 0, 0x77);
        t.check(
            &format!("{label}: iggy warning"),
            0x00a3_e5e0,
            Conv::Stdcall,
            &[1, 2, 0x3333, 4],
            |g| Out::U32(sb::iggy_warning(g, 0x3333)),
        );
        t.reset();
        // the bitmap font
        let glyph = t.junk(0x40, 25);
        t.check(
            &format!("{label}: glyph ctor"),
            0x00a2_bab0,
            Conv::Thiscall(glyph),
            &[],
            |g| Out::U32(sb::glyph_ctor(g, glyph)),
        );
        let data = t.junk(0x30 * 5, 26);
        for k in 0..5u32 {
            t.w32(data + 0x30 * k, 0x100 + k % 3);
        }
        let key = t.zeros(4);
        for (count, want, first) in [
            (5u32, 0x101u32, 0u32),
            (5, 0x101, 2),
            (5, 0x100, 4),
            (5, 0x999, 0),
            (5, 0x100, 5),
            (5, 0x100, 9),
            (0x8000_0005, 0x102, 0),
            (0x8000_0005, 0x999, 0),
            (0, 0x100, 0),
        ] {
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, data);
            t.w32(key, want);
            t.check(
                &format!("{label}: glyph find {count:#x} {want:#x} {first}"),
                0x00a2_9320,
                Conv::Thiscall(arr),
                &[key, first],
                |g| Out::U32(sb::glyph_find(g, arr, key, first)),
            );
        }
        t.stub(small::RENDER_MESH_INSTANCE, true, 4, 4, 0x3e3e);
        let text = t.junk(0x200, 27);
        let font = t.junk(0x40, 28);
        let inst = t.junk(0x200, 29);
        t.w32(text + 0x158, font);
        t.check(
            &format!("{label}: text render"),
            0x00a2_a400,
            Conv::Thiscall(text),
            &[inst],
            |g| Out::U32(sb::text_render(g, text, inst)),
        );
        t.reset();
        for s in [&b"\0"[..], b"a\0", b"PhyreEngine\0"] {
            let text = t.alloc(s);
            let pp = t.zeros(4);
            t.w32(pp, text);
            let out = t.junk(4, 30);
            let len = t.junk(4, 31);
            t.check(
                &format!("{label}: string {}", s.len()),
                0x005c_8580,
                Conv::Cdecl,
                &[pp, out, len],
                |g| Out::U32(sb::string_with_length(g, pp, out, len)),
            );
        }
    }
}

// ---- batch 3: arrays, command hand-over, utilities, fog ----

use ffx_prender::m5p_render_b_arrays::{self as arr, bridge as ab};

impl Twins {
    /// Writes word `v` at `at` in both oracles, the old bytes put back by
    /// [`Twins::reset`].
    fn put_saved(&mut self, at: u32, v: u32) {
        let old = Oracle::read(&mut self.a, at, 4).unwrap();
        self.saved.entry(at).or_insert(old);
        self.w32(at, v);
    }
}

#[test]
fn sharray_fields_match_the_original() {
    assert_eq!(arr::SH_FIELDS.len(), 12);
    for (label, mut t) in Twins::all() {
        for &(f, e) in &arr::SH_FIELDS {
            assert_eq!(arr::sh_field_of(f), e);
            for count in [0u32, 1, 2, 3, 0x8000_0001, 0x8000_0003] {
                for i in [0u32, 1, 2, 3, 0x8000_0000] {
                    for flag in [0u8, 1, 0xfe, 0xff] {
                        if e.flag.is_none() && flag != 1 {
                            continue;
                        }
                        let obj = t.junk(0x80, i as u8 ^ count as u8);
                        let data = t.junk(0x80, 0x33);
                        t.w32(obj + e.count, count);
                        t.w32(obj + e.count + 4, data);
                        if let Some(fl) = e.flag {
                            t.w8(obj + fl, flag);
                        }
                        let what = format!("{label}: field {f:#x} {count:#x} {i:#x} {flag:#x}");
                        t.check(&what, f, Conv::Thiscall(obj), &[i], |g| {
                            Out::U32(ab::sh_field(g, obj, i, e))
                        });
                    }
                }
            }
        }
    }
}

#[test]
fn alloc_copies_match_the_original() {
    assert_eq!(arr::ALLOC_COPIES.len(), 4);
    for (label, mut t) in Twins::all() {
        for &(f, fill) in &arr::ALLOC_COPIES {
            assert_eq!(arr::fill_of(f), fill);
            for (n, m, fails) in [
                (0u32, 0u32, false),
                (4, 2, true),
                (4, 0, false),
                (4, 2, false),
                (4, 4, false),
                (1, 1, false),
            ] {
                let buf = t.junk(0x40, 0x21);
                t.stub(arr::PHYRE_MALLOC, false, 2, 0, if fails { 0 } else { buf });
                if let arr::Fill::Pairs { copy, .. } = fill {
                    t.stub(copy, false, 3, 0, 0xc0c0);
                }
                let src = t.junk(0x40, 0x45);
                let what = format!("{label}: alloc copy {f:#x} {n} {m} {fails}");
                t.check(&what, f, Conv::Cdecl, &[n, src, m], |g| {
                    Out::U32(ab::alloc_copy(g, n, src, m, fill))
                });
                t.reset();
            }
        }
        // the resize: same word, smaller, larger, not owned, failing, empty
        for (word, n, ret, same) in [
            (3u32, 3u32, 0x100u32, false),
            (0x8000_0003, 3, 0x100, false),
            (3, 2, 0x100, false),
            (3, 5, 0x100, false),
            (0x8000_0003, 5, 0x100, false),
            (3, 5, 0, false),
            (3, 0, 0, false),
            (3, 4, 0, true),
        ] {
            let obj = t.zeros(8);
            let old = t.junk(0x20, 1);
            let new = t.junk(0x20, 2);
            t.stub(
                arr::ALLOC_COPY_U8P,
                false,
                3,
                0,
                if same {
                    old
                } else if ret == 0 {
                    0
                } else {
                    new
                },
            );
            t.stub(arr::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.w32(obj, word);
            t.w32(obj + 4, old);
            let what = format!("{label}: resize {word:#x} {n} {ret:#x} {same}");
            t.check(&what, 0x005a_f000, Conv::Thiscall(obj), &[n], |g| {
                Out::U32(ab::resize_u8p(g, obj, n))
            });
            t.reset();
        }
        // the 0x1c-byte storage
        for (n, m, fails) in [
            (0u32, 0u32, false),
            (1, 1, false),
            (3, 2, false),
            (3, 0, false),
            (3, 2, true),
        ] {
            let buf = t.junk(0x80, 3);
            t.stub(arr::PHYRE_MALLOC, false, 2, 0, if fails { 0 } else { buf });
            t.stub(arr::ZERO_ARRAY_28, false, 2, 0, 0x2828);
            let obj = t.junk(0x40, 4);
            let src = t.junk(0x80, 5);
            let what = format!("{label}: alloc copy 28 {n} {m} {fails}");
            t.check(&what, 0x0058_c590, Conv::Thiscall(obj), &[n, src, m], |g| {
                Out::U32(ab::alloc_copy_28(g, obj, n, src, m))
            });
            t.reset();
        }
        let obj = t.junk(0x60, 6);
        let src = t.junk(0x40, 7);
        t.check(
            &format!("{label}: copy 16"),
            0x0060_41e0,
            Conv::Thiscall(obj),
            &[src],
            |g| Out::U32(ab::copy_16_words(g, obj, src)),
        );
        let obj = t.junk(0x3080, 8);
        t.check(
            &format!("{label}: zero 3040"),
            0x009f_c970,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(ab::zero_3040(g, obj)),
        );
        let obj = t.junk(0x2340, 9);
        t.check(
            &format!("{label}: init records"),
            0x005b_4440,
            Conv::Thiscall(obj),
            &[1, 2, 3],
            |g| Out::U32(ab::init_records(g, obj, 1, 2, 3)),
        );
    }
}

#[test]
fn hand_over_and_buffers_match_the_original() {
    for (label, mut t) in Twins::all() {
        for with in [false, true] {
            t.stub(arr::EVENT_SIGNAL, true, 0, 0, 0x5151);
            let slot = t.zeros(4);
            if with {
                let cmd = t.junk(0x20, 1);
                t.w32(slot, cmd);
            }
            let what = format!("{label}: hand over {with}");
            t.check(
                &what,
                0x0061_52a0,
                Conv::Thiscall(0x7777),
                &[slot, 0xb, 0xc, 0xd],
                |g| Out::U32(ab::hand_over(g, slot, 0xb, 0xc, 0xd)),
            );
            t.reset();
        }
        for (with, r) in [(false, 0u32), (true, 5), (true, 0)] {
            t.stub(arr::RENDER_THREAD_INSTANCE, false, 0, 0, 0x7e7e_0000);
            t.stub(arr::HAND_OVER, true, 4, 4, r);
            t.stub(arr::NEXT_SEGMENT, true, 0, 0, 0x5e);
            let obj = t.junk(0x100, 2);
            let slot = if with { t.junk(4, 3) } else { 0 };
            t.w32(obj + 0xe0, slot);
            let what = format!("{label}: flush {with} {r}");
            t.check(&what, 0x005b_6ca0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(ab::flush_commands(g, obj))
            });
            t.reset();
        }
        // the PArray external buffer
        for (word, same, n) in [
            (3u32, false, 2u32),
            (0x8000_0003, false, 2),
            (3, true, 0),
            (3, false, 0x8000_0000),
            (3, false, 0x7fff_ffff),
        ] {
            t.stub(arr::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.zeros(8);
            let old = t.junk(0x10, 4);
            let buf = if same { old } else { t.junk(0x10, 5) };
            t.w32(obj, word);
            t.w32(obj + 4, old);
            let what = format!("{label}: set external {word:#x} {same} {n:#x}");
            t.check(&what, 0x00a2_3be0, Conv::Thiscall(obj), &[buf, n], |g| {
                Out::U32(ab::set_external(g, obj, buf, n))
            });
            t.reset();
        }
        // the PSharray external buffer
        for (count, n, same) in [
            (0u32, 1u32, false),
            (1, 1, false),
            (3, 1, false),
            (0x8000_0003, 1, false),
            (3, 2, false),
            (3, 2, true),
            (0x8000_0003, 2, false),
            (1, 2, false),
            (0, 0, false),
            (3, 0x8000_0001, false),
        ] {
            t.stub(arr::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.zeros(8);
            let old = t.junk(0x10, 6);
            let buf = if same { old } else { t.junk(0x10, 7) };
            t.w32(obj, count);
            t.w32(obj + 4, old);
            let what = format!("{label}: sharray external {count:#x} {n:#x} {same}");
            t.check(&what, 0x0060_3dc0, Conv::Thiscall(obj), &[buf, n], |g| {
                Out::U32(ab::sharray_set_external(g, obj, buf, n))
            });
            t.reset();
        }
        // the shapes member's setter: its own word with one element (with
        // and without a descriptor), anything else
        for (own, n, desc) in [
            (true, 1u32, 0x4444u32),
            (true, 1, 0),
            (true, 2, 0x4444),
            (false, 1, 0x4444),
        ] {
            t.stub(arr::APPLY_DEFAULT_BUFFER, true, 2, 2, 0xad);
            t.stub(arr::SHAPES_SET_EXTERNAL, true, 2, 2, 0x5e);
            let member = t.rebase(arr::SHAPES_MEMBER);
            let o = t.object(&[0, 0], &[0, desc]);
            let vt = t.r32(o);
            t.put_saved(member, vt);
            let obj = t.junk(0x10, 8);
            let buf = if own { obj + 4 } else { t.junk(0x10, 9) };
            let what = format!("{label}: shapes buffer {own} {n} {desc:#x}");
            t.check(&what, 0x0060_3e50, Conv::Thiscall(obj), &[buf, n], |g| {
                Out::U32(ab::shapes_set_buffer(g, obj, buf, n))
            });
            t.reset();
        }
    }
}

#[test]
fn utilities_and_fog_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the registry list: empty, first, middle, absent; both flags
        for (case, flags) in [
            ("empty", 1u32),
            ("first", 0),
            ("middle", 1),
            ("absent", 0),
            ("last", 1),
            ("absent", 2),
        ] {
            t.stub(arr::OPERATOR_DELETE, false, 1, 0, 0xde1e);
            let head = t.rebase(arr::UTILITY_LIST);
            let this = t.junk(0x10, 1);
            let a = t.junk(0x10, 2);
            let b = t.junk(0x10, 3);
            let first = match case {
                "empty" => 0,
                "first" => {
                    t.w32(this + 4, a);
                    t.w32(a + 4, 0);
                    this
                }
                "middle" => {
                    t.w32(a + 4, this);
                    t.w32(this + 4, b);
                    t.w32(b + 4, 0);
                    a
                }
                "last" => {
                    t.w32(a + 4, this);
                    t.w32(this + 4, 0);
                    a
                }
                _ => {
                    t.w32(a + 4, b);
                    t.w32(b + 4, 0);
                    a
                }
            };
            t.put_saved(head, first);
            for f in [0x00a0_3140u32, 0x00a3_e800] {
                let what = format!("{label}: utility delete {f:#x} {case} {flags}");
                // both copies on the same list state
                let snapshot_head = t.r32(head);
                let next = t.r32(this + 4);
                t.check(&what, f, Conv::Thiscall(this), &[flags], |g| {
                    Out::U32(ab::utility_delete(g, this, flags))
                });
                t.w32(head, snapshot_head);
                t.w32(this + 4, next);
                if case == "middle" || case == "last" {
                    t.w32(a + 4, this);
                }
                t.diff(&what);
            }
            t.reset();
        }
        // the post-processing resources
        for (f1, f2, with) in [
            (0u32, 0u32, false),
            (0x80, 0, true),
            (0, 0x80, true),
            (0x180, 0x7f, false),
            (0xffff_ffff, 0xffff_ffff, true),
        ] {
            t.stub(arr::SAMPLER_RELEASE, true, 0, 0, 0x5a);
            t.stub(arr::SPRITE_ARRAYS_RELEASE, true, 0, 0, 0x5b);
            let obj = t.junk(0x1000, 4);
            t.w32(obj + 0xf5c, f1);
            t.w32(obj + 0xf80, f2);
            let p = if with {
                let p = t.zeros(4);
                let arrays = t.junk(0x10, 5);
                t.w32(p, arrays);
                p
            } else {
                0
            };
            t.w32(obj + 0xf38, p);
            let what = format!("{label}: post resources {f1:#x} {f2:#x} {with}");
            t.check(&what, 0x00a2_4c90, Conv::Thiscall(obj), &[], |g| {
                ab::release_post_resources(g, obj);
                Out::None
            });
            t.reset();
        }
        // the fog setters: moving (al set), not moving with a non-zero eax
        for moving in [1u32, 0x100, 0] {
            for &(f, field) in &arr::FOG_SETTERS {
                assert_eq!(arr::fog_field_of(f), field);
                t.stub(arr::FOG_IS_MOVING, true, 0, 0, moving);
                let fog = t.junk(0xb8, 6);
                let src = t.junk(0x40, 7);
                t.w32(fog + 0xa0, src);
                let what = format!("{label}: fog {f:#x} {moving:#x}");
                t.check(&what, f, Conv::Thiscall(fog), &[0x3f40_0000], |g| {
                    Out::U32(ab::fog_set(g, fog, field, &[0x3f40_0000]))
                });
                t.reset();
            }
            t.stub(arr::FOG_IS_MOVING, true, 0, 0, moving);
            let fog = t.junk(0xb8, 8);
            let src = t.junk(0x40, 9);
            t.w32(fog + 0xa0, src);
            let rgb = [0x3f80_0000, 0x7fc0_0000, 0x8000_0000];
            let what = format!("{label}: fog colour {moving:#x}");
            t.check(&what, 0x006e_36f0, Conv::Thiscall(fog), &rgb, |g| {
                Out::U32(ab::fog_set(g, fog, 0x10, &rgb))
            });
            t.reset();
        }
    }
}

// ---- batch 4: post-processing setups, entries, resizes; renderer dtors ----

use ffx_prender::m5p_render_b_dtors::{self as dt, bridge as tb};
use ffx_prender::m5p_render_b_post2::{self as post2, Render, bridge as qb};

/// A target case: no handle, or a handle with or without a texture of a size.
type Tex = Option<Option<(u32, u32)>>;

/// A render-target handle `{texture, kind}` over a texture of `w` x `h`
/// (or none, or a kind that holds none).
fn target(t: &mut Twins, tex: Option<(u32, u32)>, kind: u8) -> u32 {
    let handle = t.zeros(8);
    if let Some((w, hgt)) = tex {
        let tx = t.junk(0x40, 0x61);
        t.w32(tx + 0x1c, w);
        t.w32(tx + 0x20, hgt);
        t.w32(handle, tx);
    }
    t.w8(handle + 4, kind);
    handle
}

#[test]
fn post_setups_and_entries_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the setups: the interface made or not, default sizes, the result
        for &(f, init) in &post2::SETUPS {
            assert_eq!(post2::setup_of(f), init);
            for (made, w, hgt, r) in [
                (true, 640u32, 360u32, 0u32),
                (true, 0, 0, 0),
                (false, 0, 200, 7),
                (false, 320, 0, 0),
            ] {
                t.stub(post2::INTERFACE_CTOR, true, 0, 0, 0);
                t.stub(post2::ATEXIT, false, 1, 0, 0);
                t.stub(init, true, 3, 3, r);
                let guard = t.rebase(post2::INTERFACE_GUARD);
                let g = t.r32(guard);
                t.put_saved(guard, if made { g | 1 } else { g & !1 });
                let dw = t.rebase(post2::DEFAULT_WIDTH);
                t.put_saved(dw, 1280);
                let dh = t.rebase(post2::DEFAULT_HEIGHT);
                t.put_saved(dh, 720);
                let obj = t.junk(0x40, 1);
                let what = format!("{label}: setup {f:#x} {made} {w} {hgt} {r}");
                t.check(&what, f, Conv::Thiscall(obj), &[w, hgt], |g| {
                    Out::U32(qb::setup(g, obj, init, w, hgt))
                });
                t.reset();
            }
        }
        // the entries: set up or not (slot 1's result), no record, the
        // material changed or not, missing, each renderer's result
        for &(f, how) in &post2::ENTRIES {
            assert_eq!(post2::entry_of(f), how);
            for (set_up, r1, with_c, changed, material, e, r) in [
                (0u8, 9u32, true, false, true, 1u32, 0u32),
                (0, 0, true, true, true, 1, 0x33),
                (0, 0, false, false, true, 1, 0),
                (1, 0, true, false, false, 1, 0),
                (1, 0, true, true, true, 0, 0),
                (1, 0, true, false, true, 1, 0),
                (1, 0, true, false, true, 1, 0x44),
            ] {
                let fs: Vec<u32> = match how {
                    Render::Lighting(x)
                    | Render::DepthOfField(x)
                    | Render::Fxaa(x)
                    | Render::Reflection(x) => vec![x],
                    Render::Glow { pre, f } => vec![pre, f],
                };
                for (k, &x) in fs.iter().enumerate() {
                    let n = match (how, k) {
                        (Render::Lighting(_), _) => 3,
                        (Render::DepthOfField(_) | Render::Reflection(_), _)
                        | (Render::Glow { .. }, 1) => 5,
                        (Render::Fxaa(_), _) => 4,
                        (Render::Glow { .. }, _) => 2,
                    };
                    let ret = if k + 1 == fs.len() { r } else { r & 0x40 };
                    t.stub(x, true, n, n, ret);
                }
                let o = t.object(&[1, 2, 0, 2, 0, 2, 5], &[0, r1]);
                let vt = t.r32(o);
                let obj = t.junk(0x100, 0x13);
                t.w32(obj, vt);
                t.w8(obj + 0x10, set_up);
                let handle = t.zeros(8);
                if material {
                    let m = t.junk(0x10, 2);
                    t.w32(handle, m);
                }
                t.w32(obj + 8, handle);
                t.w32(obj + 0xc, if changed { 0x1234 } else { handle });
                let bloom = t.junk(0x10, 3);
                t.w8(bloom + 4, set_up);
                t.w32(obj + 0xa8, bloom);
                let a = t.junk(0x80, 4);
                let c = if with_c {
                    let c = t.junk(0x40, 5);
                    t.w32(c + 0x1c, 800);
                    t.w32(c + 0x20, 600);
                    c
                } else {
                    0
                };
                let args = [a, 0xb, c, 0xd, e];
                let what = format!(
                    "{label}: entry {f:#x} {set_up} {r1} {with_c} {changed} {material} {e} {r:#x}"
                );
                t.check(&what, f, Conv::Thiscall(obj), &args, |g| {
                    Out::U32(qb::entry(g, obj, how, args))
                });
                t.reset();
            }
        }
    }
}

#[test]
fn post_resizes_and_dtors_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the size check through each path: no handle, a kind without a
        // texture, the same size, bound (unbind fails or not), the resize
        // failing, done
        let cases: [(Tex, u8, u32, u32, u32); 8] = [
            (None, 0, 0, 0, 0),
            (Some(None), 0, 0, 0, 0),
            (Some(Some((64, 32))), 1, 0, 0, 0),
            (Some(Some((64, 32))), 0, 0, 0, 0),
            (Some(Some((10, 32))), 0, 1, 0x77, 0),
            (Some(Some((10, 32))), 0, 0x100, 0, 0x88),
            (Some(Some((64, 10))), 0, 1, 0, 0),
            (Some(Some((10, 10))), 0, 0, 0, 0),
        ];
        for (tex, kind, bound, unbind, set) in cases {
            for (f, at, set_up) in [
                (0x00a2_3200u32, 0x5cu32, None),
                (0x00a1_b8f0, 0x5c, Some(1u8)),
                (0x00a1_b8f0, 0x5c, Some(0)),
                (0x00a1_b870, 0xfb0, Some(1)),
                (0x00a1_b870, 0xfb0, Some(0)),
            ] {
                t.stub(post2::IS_BOUND, true, 0, 0, bound);
                t.stub(post2::UNBIND, true, 0, 0, unbind);
                t.stub(post2::SET_DIMENSIONS, true, 4, 4, set);
                t.stub(post2::HALF_TARGETS, true, 2, 2, 0x5a);
                let obj = t.junk(0x1000, 6);
                let handle = match tex {
                    None => 0,
                    Some(x) => target(&mut t, x, kind),
                };
                t.w32(obj + at, handle);
                if let Some(s) = set_up {
                    t.w8(obj + 0x10, s);
                }
                let what = format!(
                    "{label}: resize {f:#x} {tex:?} {kind} {bound} {unbind:#x} {set:#x} {set_up:?}"
                );
                t.check(&what, f, Conv::Thiscall(obj), &[64, 32], |g| {
                    Out::U32(match f {
                        0x00a2_3200 => qb::resize_5c(g, obj, 64, 32),
                        0x00a1_b8f0 => qb::resize_5c_if_set_up(g, obj, 64, 32),
                        _ => qb::resize_depth_of_field(g, obj, 64, 32),
                    })
                });
                t.reset();
            }
        }
        // the half-size targets: each failing in turn, all done
        for bad in [0usize, 1, 2, 3] {
            t.stub(post2::IS_BOUND, true, 0, 0, 0);
            t.stub(post2::SET_DIMENSIONS, true, 4, 4, 0);
            let obj = t.junk(0x1000, 7);
            for k in 0..3u32 {
                let h = if k as usize == bad {
                    0
                } else {
                    target(&mut t, Some((50, 20)), 0)
                };
                t.w32(obj + 0xf2c + 4 * k, h);
            }
            let what = format!("{label}: halves {bad}");
            t.check(&what, 0x00a2_3080, Conv::Thiscall(obj), &[101, 41], |g| {
                Out::U32(qb::resize_halves(g, obj, 101, 41))
            });
            t.reset();
        }
        // the lighting release and the destructor bodies
        for (flags, meshes) in [(0u32, false), (0x80, true), (0xffff_ffff, true)] {
            t.stub(post2::SAMPLER_RELEASE, true, 0, 0, 0x5a);
            t.stub(post2::SPRITE_ARRAYS_RELEASE, true, 0, 0, 0x5b);
            t.stub(post2::SAMPLER_DTOR, true, 0, 0, 0x5c);
            t.stub(post2::RENDER_TARGET_RELEASE, false, 1, 0, 0x5d);
            t.stub(post2::DEPTH_OF_FIELD_BASE_DTOR, true, 0, 0, 0x5e);
            t.stub(post2::PARTICLE_MEMBERS_RELEASE, true, 0, 0, 0x5f);
            t.stub(post2::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.junk(0x4000, 8);
            for at in [0x3f04, 0x3f4c, 0x3f28] {
                t.w32(obj + at, flags);
            }
            for at in [0x3edc, 0x3ee0, 0x3ee4] {
                let p = if meshes {
                    let p = t.zeros(4);
                    let arrays = t.junk(0x10, 9);
                    t.w32(p, arrays);
                    p
                } else {
                    0
                };
                t.w32(obj + at, p);
            }
            t.check(
                &format!("{label}: lighting release {flags:#x}"),
                0x00a2_4a60,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    qb::lighting_release(g, obj);
                    Out::None
                },
            );
            t.stub(post2::LIGHTING_RELEASE, true, 0, 0, 0x60);
            t.check(
                &format!("{label}: lighting dtor"),
                0x009f_db80,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(qb::lighting_dtor(g, obj)),
            );
            t.check(
                &format!("{label}: dof dtor"),
                0x009f_de10,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(qb::depth_of_field_dtor(g, obj)),
            );
            t.check(
                &format!("{label}: particles delete {flags:#x}"),
                0x00a0_2d90,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(qb::particles_delete(g, obj, flags)),
            );
            t.reset();
        }
    }
}

#[test]
fn renderer_dtors_and_states_match_the_original() {
    for (label, mut t) in Twins::all() {
        for s in [
            dt::TREE_CLEAR,
            dt::TREE_ORPHAN_ALL,
            dt::FREE_LIST_RESET,
            dt::FREE_LIST_DTOR,
            dt::POOLED_HANDLE_RELEASE,
        ] {
            t.stub(s, true, 0, 0, s & 0xfff);
        }
        t.stub(dt::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(dt::EH_VECTOR_DTOR, false, 4, 4, 0xe4);
        t.stub(dt::CLEAR_WORD0, true, 0, 0, 0xc0);
        let obj = t.junk(0x200, 1);
        for f in [0x006a_6720u32, 0x006a_6780] {
            t.check(
                &format!("{label}: tree clear {f:#x}"),
                f,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(tb::tree_clear_twice(g, obj)),
            );
        }
        t.check(
            &format!("{label}: map dtor"),
            0x006a_67e0,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(tb::tree_map_dtor(g, obj)),
        );
        // remove all: empty (the tree itself), null, a chain of nodes
        // ending at the tree or at 0; the remove stub logs and unlinks
        // (`[tree + 0xc]` = the node's word 0)
        {
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes()); // mov eax, [log]
            c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
            c.push(0x05);
            c.extend((log + 4).to_le_bytes()); // add eax, log + 4
            c.extend([0xc7, 0x00]);
            c.extend(dt::TREE_REMOVE.to_le_bytes()); // mov [eax], tag
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [log]
            c.extend([0x8b, 0x02]); // mov eax, [edx]
            c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
            c.extend([0xc2, 0x04, 0x00]); // ret 4
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(dt::TREE_REMOVE);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
        }
        let n1 = t.zeros(0x10);
        let n2 = t.zeros(0x10);
        for (first, end) in [(obj, 0u32), (0, 0), (n1, obj), (n1, 0)] {
            t.w32(n1, n2);
            t.w32(n2, end);
            t.w32(obj + 0xc, first);
            t.check(
                &format!("{label}: remove all {first:#x} {end:#x}"),
                0x006a_68f0,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(tb::tree_map_remove_all(g, obj)),
            );
        }
        t.check(
            &format!("{label}: handle dtor"),
            0x0061_4300,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(tb::handle_dtor(g, obj)),
        );
        t.check(
            &format!("{label}: renderer base dtor"),
            0x005b_49e0,
            Conv::Thiscall(obj),
            &[],
            |g| {
                tb::renderer_base_dtor(g, obj, 0);
                Out::None
            },
        );
        t.check(
            &format!("{label}: renderer dtor"),
            0x005b_4950,
            Conv::Thiscall(obj),
            &[],
            |g| {
                tb::renderer_dtor(g, obj, 0);
                Out::None
            },
        );
        for flags in [0u32, 1, 2] {
            for f in [0x005b_4fa0u32, 0x005b_5010] {
                t.check(
                    &format!("{label}: renderer base delete {f:#x} {flags}"),
                    f,
                    Conv::Thiscall(obj),
                    &[flags],
                    |g| Out::U32(tb::renderer_base_dtor(g, obj, flags)),
                );
            }
            t.check(
                &format!("{label}: renderer delete {flags}"),
                0x005b_4ef0,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(tb::renderer_dtor(g, obj, flags)),
            );
        }
        t.check(
            &format!("{label}: job ctor"),
            0x005b_4750,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(tb::job_ctor(g, obj)),
        );
        t.reset();
        // a job from the ring buffer, at once or after a wait
        for waits in [0u32, 1] {
            let ring = t.zeros(0x10);
            let block = t.junk(0x60, 2);
            t.stub(dt::RING_SINGLETON, false, 0, 0, ring);
            // the allocation fails `waits` times: a counter stub
            let counter = t.zeros(4);
            t.w32(counter, waits);
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes()); // mov eax, [log]
            c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
            c.push(0x05);
            c.extend((log + 4).to_le_bytes()); // add eax, log + 4
            c.extend([0xc7, 0x00]);
            c.extend(dt::RING_ALLOCATE.to_le_bytes()); // mov [eax], tag
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [log]
            c.extend([0x83, 0x2d]);
            c.extend(counter.to_le_bytes());
            c.push(0x01); // sub dword [counter], 1
            c.extend([0xb8, 0, 0, 0, 0]); // mov eax, 0 (flags kept)
            c.extend([0x79, 0x05]); // jns +5: fail while the counter was > 0
            c.push(0xb8);
            c.extend(block.to_le_bytes()); // mov eax, block
            c.extend([0xc2, 0x04, 0x00]); // ret 4
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(dt::RING_ALLOCATE);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            let pool = t.object(&[0, 0, 0, 0, 0, 0, 0], &[]);
            t.stub(dt::POOL_SINGLETON, false, 0, 0, pool);
            t.stub(dt::SLEEP, false, 1, 0, 0);
            t.stub(dt::JOB_CTOR, true, 0, 0, block + 0x10);
            let what = format!("{label}: new job {waits}");
            t.check(&what, 0x005b_3680, Conv::Cdecl, &[], |g| {
                Out::U32(tb::new_job(g))
            });
            t.reset();
        }
        // the records' arrays
        for (c1, b1, c2, b2) in [
            (3u32, 0x40u32, 0x8000_0001u32, 0x50u32),
            (0, 0, 2, 0x60),
            (1, 0x70, 0, 0),
        ] {
            t.stub(dt::RECORDS_RELEASE, false, 1, 0, 0x9930);
            t.stub(dt::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.junk(0x20, 3);
            t.w32(obj + 0x10, c1);
            t.w32(obj + 0x14, b1);
            t.w32(obj + 8, c2);
            t.w32(obj + 0xc, b2);
            t.check(
                &format!("{label}: records dtor {c1:#x} {c2:#x}"),
                0x005c_82a0,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    tb::records_dtor(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
        // the state setters
        let obj = t.junk(0x700, 4);
        for (a, b) in [(0u32, 0u32), (1, 2), (3, 4), (5, 0xffff_ffff), (4, 5)] {
            t.check(
                &format!("{label}: modes {a} {b}"),
                0x0059_9820,
                Conv::Thiscall(obj),
                &[a, b],
                |g| Out::U32(tb::set_modes(g, obj, a, b)),
            );
        }
        for m in [0u32, 1, 2, 3, 0xffff_ffff] {
            t.w32(obj + 0x578, 0xdead);
            t.w32(obj + 0x57c, 0xbeef);
            t.check(
                &format!("{label}: state 578 {m}"),
                0x0059_a070,
                Conv::Thiscall(obj),
                &[m],
                |g| Out::U32(tb::set_578(g, obj, m)),
            );
        }
        for f in [dt::MAP_BLEND, dt::MAP_MODE8, dt::MAP_OP] {
            t.stub(f, false, 1, 0, f & 0xff);
        }
        t.check(
            &format!("{label}: blend"),
            0x0059_9950,
            Conv::Thiscall(obj),
            &[0x1ff, 2, 3, 4, 5],
            |g| Out::U32(tb::set_blend(g, obj, 0x1ff, [2, 3, 4, 5])),
        );
        let w: [u32; 15] = core::array::from_fn(|i| 0x0101_0100 * (i as u32 + 1) + i as u32);
        t.check(
            &format!("{label}: depth stencil"),
            0x0059_bac0,
            Conv::Thiscall(obj),
            &w,
            |g| Out::U32(tb::set_depth_stencil(g, obj, w)),
        );
        t.reset();
        // the COM release loops: every other object set
        for mask in [0u32, 0x5555_5555, 0xffff_ffff] {
            t.stub(dt::RELEASE_VIEWS, true, 0, 0, 0x76);
            let obj = t.junk(0x80, 5);
            for k in 0..0x16u32 {
                let o = if (mask >> k) & 1 != 0 {
                    t.com_object(&[0, 0, 1], &[0, 0, k])
                } else {
                    0
                };
                t.w32(obj + 4 * k, o);
            }
            t.check(
                &format!("{label}: shader release {mask:#x}"),
                0x0059_c190,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(tb::shader_release(g, obj)),
            );
            let obj = t.junk(0x40, 6);
            for k in 0..0xcu32 {
                let o = if (mask >> k) & 1 != 0 {
                    t.com_object(&[0, 0, 1], &[0, 0, k])
                } else {
                    0
                };
                t.w32(obj + 4 * k, o);
            }
            t.check(
                &format!("{label}: release eight {mask:#x}"),
                0x0059_c370,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    tb::release_eight(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
        // the tweak utility
        for (on, util) in [(0u32, 1u8), (1, 0), (1, 1), (0x100, 1), (0xff, 2)] {
            t.stub(dt::TEXT_UTIL_SET_ENABLED, true, 1, 1, 0x5e);
            t.stub(dt::TEXT_UTIL_RESIZE, true, 2, 2, 0x5f);
            let obj = t.junk(0x200, 7);
            t.w8(obj, util);
            t.check(
                &format!("{label}: tweak enabled {on:#x} {util}"),
                0x006a_a5f0,
                Conv::Thiscall(obj),
                &[on],
                |g| Out::U32(tb::tweak_set_enabled(g, obj, on)),
            );
            t.reset();
        }
        for (with, count, buffer) in [
            (false, 0u32, 0u32),
            (true, 0, 0x40),
            (true, 2, 0),
            (true, 2, 0x40),
            (true, 0x8000_0002, 0x40),
        ] {
            t.stub(dt::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(dt::RENDER_TARGET_RELEASE, true, 0, 0, 0x41);
            t.stub(dt::TREE_ORPHAN_ALL, true, 0, 0, 0x42);
            t.stub(dt::FREE_LIST_RESET, true, 0, 0, 0x43);
            let obj = t.junk(0x200, 8);
            let arr = if with {
                let a = t.zeros(8);
                t.w32(a, count);
                t.w32(a + 4, buffer);
                a
            } else {
                0
            };
            t.w32(obj + 0xf0, arr);
            t.check(
                &format!("{label}: tweak release {with} {count:#x} {buffer:#x}"),
                0x006a_b540,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(tb::tweak_release(g, obj)),
            );
            t.reset();
        }
        // the char-info descriptor: made now or already
        for made in [false, true] {
            for (f, flag) in [(0x00a2_b3b0u32, 1u32), (0x00a2_b510, 0)] {
                t.stub(dt::CHAR_INFO_CTOR, true, 0, 0, 0);
                t.stub(dt::ATEXIT, false, 1, 0, 0);
                t.stub(dt::SWAP_ENDIANNESS, true, 2, 2, 0x5a00 | flag);
                let guard = t.rebase(dt::CHAR_INFO_GUARD);
                let g = t.r32(guard);
                t.put_saved(guard, if made { g | 1 } else { g & !1 });
                let fl = t.rebase(dt::CHAR_INFO_FLAGS);
                t.put_saved(fl, 0xffff_ffff);
                let desc = t.rebase(dt::CHAR_INFO_DESC);
                t.put_saved(desc, 0);
                let p = t.junk(0x10, 9);
                t.check(
                    &format!("{label}: char info {f:#x} {made}"),
                    f,
                    Conv::Cdecl,
                    &[p],
                    |g| Out::U32(tb::char_info_swap(g, p, flag)),
                );
                t.reset();
            }
        }
        // the 16-byte resize
        for (word, n, fails) in [
            (3u32, 3u32, false),
            (3, 2, false),
            (3, 5, false),
            (0x8000_0003, 5, false),
            (3, 5, true),
            (3, 0, false),
            (0, 2, false),
        ] {
            let buf = t.junk(0x80, 10);
            t.stub(dt::PHYRE_MALLOC, false, 2, 0, if fails { 0 } else { buf });
            t.stub(dt::COPY16, false, 3, 0, 0x16);
            t.stub(dt::ZERO16, false, 2, 0, 0x17);
            t.stub(dt::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.zeros(8);
            let old = if word == 0 { 0 } else { t.junk(0x40, 11) };
            t.w32(obj, word);
            t.w32(obj + 4, old);
            t.check(
                &format!("{label}: resize16 {word:#x} {n} {fails}"),
                0x005a_f070,
                Conv::Thiscall(obj),
                &[n],
                |g| Out::U32(tb::resize16(g, obj, n)),
            );
            t.reset();
        }
    }
}

// ---- batch 5: medium renderer, shader-manager, text and post methods ----

use ffx_prender::m5p_render_b_more::{self as more, bridge as mb};

impl Twins {
    /// The original's `ST0` (as `fst dword` stores it) against the port's
    /// `f32`, bit for bit, and every byte written.
    fn check_f32(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> f32,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let v = port(&mut G(&mut self.b));
        let st0 = ra
            .st0_f32()
            .unwrap_or_else(|| panic!("{what}: the original leaves no ST0"));
        assert_eq!(st0.to_bits(), v.to_bits(), "{what}: ST0 {st0} vs {v}");
        self.diff(what);
    }
}

#[test]
fn more_render_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the lighting geometry
        t.stub(more::CREATE_TRIANGLE, false, 1, 0, 0x7121);
        t.stub(more::CREATE_SPHERE, false, 2, 0, 0x7122);
        t.stub(more::CREATE_CONE, false, 2, 0, 0x7123);
        let obj = t.junk(0x4000, 1);
        t.check(
            &format!("{label}: lighting geometry"),
            0x00a1_9cb0,
            Conv::Thiscall(obj),
            &[0xc1],
            |g| Out::U32(mb::lighting_geometry(g, obj, 0xc1)),
        );
        t.reset();
        // the job slots: the cursor at 0, in the middle, at 15
        for i in [0u32, 7, 15] {
            t.stub(more::SEMAPHORE_WAIT, true, 0, 0, 0x5e);
            let obj = t.junk(0x180, 2);
            t.w32(obj + 0x154, i);
            let out = t.zeros(4);
            t.check(
                &format!("{label}: next slot {i}"),
                0x0061_4720,
                Conv::Thiscall(obj),
                &[out],
                |g| Out::U32(mb::next_slot(g, obj, out)),
            );
            t.reset();
        }
        // the motion-blur target: already set, not a 2D texture, made, failing
        for (have, class, mesh) in [
            (true, true, 1u32),
            (false, false, 1),
            (false, true, 0x5151),
            (false, true, 0),
        ] {
            t.stub(more::CREATE_TRIANGLE, false, 1, 0, mesh);
            let obj = t.junk(0x1000, 3);
            t.w32(obj + 0xd4c, if have { 0x55 } else { 0 });
            let tex = t.junk(0x20, 4);
            let c = t.rebase(more::TEXTURE_2D_CLASS);
            t.w32(tex, if class { c } else { c + 4 });
            let target = t.zeros(8);
            t.w32(target, tex);
            t.check(
                &format!("{label}: blur target {have} {class} {mesh:#x}"),
                0x00a1_7da0,
                Conv::Thiscall(obj),
                &[0xc1, target],
                |g| Out::U32(mb::blur_set_target(g, obj, 0xc1, target)),
            );
            t.reset();
        }
        // the LOD lookup: no levels, empty lists, inline and buffered hits
        for case in 0..6u32 {
            let obj = t.junk(0x60, 5);
            let levels = t.zeros(0x24 * 0x102);
            let e = t.junk(0x10, 6);
            let buf = t.zeros(0x10);
            t.w32(buf + 8, e);
            match case {
                0 => t.w32(obj + 0x3c, 0x8000_0000),
                1 => {
                    t.w32(obj + 0x3c, 2);
                }
                2 => {
                    t.w32(obj + 0x3c, 3);
                    t.w32(levels + 0x24 + 4, 1);
                    t.w32(levels + 0x24 + 8, e);
                }
                3 => {
                    t.w32(obj + 0x3c, 3);
                    t.w32(levels + 4, 0x8000_0003);
                    t.w32(levels + 8, buf);
                }
                4 => {
                    t.w32(obj + 0x3c, 1);
                    t.w32(levels + 4, 1);
                }
                _ => {
                    t.w32(obj + 0x3c, 0x101);
                    t.w32(levels + 0x24 * 0x100 + 4, 1);
                    t.w32(levels + 0x24 * 0x100 + 8, e);
                }
            }
            t.w32(obj + 0x40, levels);
            t.check(
                &format!("{label}: lod {case}"),
                0x005a_7410,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(mb::lod_first(g, obj)),
            );
        }
        // the kerning: none, found at each position, the amount's sign
        let face = t.zeros(0x40);
        let table = t.zeros(0x100);
        t.w32(face + 0x1c, table);
        let font = t.zeros(0x40);
        t.w32(font + 0x30, face);
        let text = t.zeros(0x200);
        t.w32(text + 0x158, font);
        for k in 0..8u32 {
            t.w32(table + 8 + 12 * k, 0x41 + k);
            t.w32(
                table + 8 + 12 * k + 4,
                [
                    3u32,
                    0xffff_fffe,
                    0,
                    0x7fff_ffff,
                    1,
                    0x0100_0001,
                    0x8000_0000,
                    5,
                ][k as usize],
            );
        }
        for (n, first, key) in [
            (0u32, 2u32, 0x41u32),
            (4, 2, 0x41),
            (4, 2, 0x42),
            (8, 2, 0x46),
            (8, 2, 0x47),
            (4, 2, 0x44),
            (8, 2, 0x48),
            (8, 2, 0x99),
            (0xffff_ffff, 2, 0x41),
            (3, 2, 0x43),
        ] {
            let pair = t.zeros(0x10);
            t.w32(pair + 4, n);
            t.w32(pair + 8, first);
            let glyph = t.zeros(4);
            t.w32(glyph, key);
            t.check_f32(
                &format!("{label}: kerning {n} {key:#x}"),
                0x00a2_9700,
                Conv::Thiscall(text),
                &[pair, glyph],
                |g| mb::kerning(g, text, pair, glyph),
            );
        }
        // the 3-stream blend
        let weights = t.alloc(
            &[0.25f32.to_bits(), 0x7fc0_1234, (-3.5f32).to_bits()]
                .map(u32::to_le_bytes)
                .concat(),
        );
        let w2 = t.alloc(
            &[0.1f32.to_bits(), 0.7f32.to_bits(), 1e30f32.to_bits()]
                .map(u32::to_le_bytes)
                .concat(),
        );
        let vals: Vec<u8> = [1.0f32, -2.5, 1e-40, 3.4e38, 0.333, -0.0, 7.0, 1e30]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let s0 = t.alloc(&vals);
        let s1 = t.alloc(&vals.iter().rev().copied().collect::<Vec<u8>>());
        let s2 = t.alloc(&vals);
        let sources = t.alloc(&[s0, s1, s2].map(u32::to_le_bytes).concat());
        for (w, n) in [(weights, 8u32), (w2, 8), (w2, 1)] {
            let dst = t.junk(0x40, 7);
            let out = t.zeros(4);
            t.w32(out, dst);
            t.check(
                &format!("{label}: blend3 {w:#x} {n}"),
                0x005c_b8d0,
                Conv::Cdecl,
                &[w, n, sources, out],
                |g| {
                    mb::blend3(g, w, n, sources, out);
                    Out::None
                },
            );
        }
        // the buffer count: bound (unbind failing or not), resize failing,
        // a first buffer or none (one buffer inline, several)
        for (bound, unbind, resize, count, first) in [
            (1u32, 7u32, 0u32, 1u32, 0u32),
            (1, 0, 9, 1, 0),
            (0, 0, 0, 1, 0),
            (1, 0, 0, 1, 0x55),
            (0, 0, 0, 3, 0),
            (0, 0, 0, 3, 0x66),
        ] {
            t.stub(more::UNBIND, true, 0, 0, unbind);
            t.stub(more::BUFFERS_RESIZE, true, 1, 1, resize);
            t.stub(more::CREATE_DEVICE_TEXTURE, true, 4, 4, 0xde);
            let obj = t.junk(0x80, 8);
            t.w32(obj + 0x70, bound);
            t.w32(obj + 0x24, count);
            let buf = t.zeros(0x10);
            if count == 1 {
                t.w32(obj + 0x28, first);
            } else {
                t.w32(buf, first);
                t.w32(obj + 0x28, buf);
            }
            t.w32(obj + 0x14, 0xffff_fbff ^ (first << 10));
            t.check(
                &format!("{label}: buffer count {bound} {unbind} {resize} {count:#x} {first:#x}"),
                0x0059_9b70,
                Conv::Thiscall(obj),
                &[2],
                |g| Out::U32(mb::set_buffer_count(g, obj, 2)),
            );
            t.reset();
        }
    }
}

#[test]
fn more_lighting_and_shaders_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the lights: ambient (3), set up or not, the kind matching or not
        let dir = t.rebase(more::LIGHT_KIND_DIRECTIONAL);
        let point = t.rebase(more::LIGHT_KIND_POINT);
        for (i, set_up, kind) in [
            (3u32, 1u8, dir),
            (0, 1, dir),
            (1, 0, dir),
            (0, 1, point),
            (1, 1, point),
            (2, 1, 0),
        ] {
            let obj = t.junk(0x200, 1);
            t.w8(obj + 0x1cc, set_up);
            let list = t.zeros(0x10);
            let e = t.junk(0x40, 2);
            t.w32(e + 0x18, kind);
            let node = t.junk(0x40, 3);
            t.w32(node + 0x24, 0x7fc0_0001);
            t.w32(e + 0x10, node);
            t.w32(list + 4 * i, e);
            let lights = t.zeros(8);
            t.w32(lights + 4, list);
            t.w32(obj + 0xf0, lights);
            let out = t.junk(0x10, 4);
            t.check(
                &format!("{label}: light colour {i} {set_up} {kind:#x}"),
                0x006a_8440,
                Conv::Thiscall(obj),
                &[out, i],
                |g| Out::U32(mb::light_colour(g, obj, out, i)),
            );
            if i != 3 {
                t.check(
                    &format!("{label}: light position {i} {set_up} {kind:#x}"),
                    0x006a_8b40,
                    Conv::Thiscall(obj),
                    &[out, i],
                    |g| Out::U32(mb::light_position(g, obj, out, i)),
                );
            }
        }
        // the shade sum: under the minimum, between, over 1, NaN
        for k in [
            [0.0f32, 0.0, 0.0, 0.0],
            [0.1, 0.2, 0.05, 0.0],
            [0.5, 0.5, 0.5, 0.5],
            [-1.0, 0.0, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0, 0.0],
            [0.3, -0.25, 1e-8, 0.125],
        ] {
            let obj = t.junk(0x200, 5);
            for (j, v) in k.iter().enumerate() {
                t.w32(obj + 0x1d0 + 4 * j as u32, v.to_bits());
            }
            t.check_f32(
                &format!("{label}: shade sum {k:?}"),
                0x006a_8bc0,
                Conv::Thiscall(obj),
                &[],
                |g| mb::shade_sum(g, obj),
            );
        }
        // the shader map's insert: from the list, after growing, still
        // empty, a clash
        for (have, grows, clash) in [
            (true, false, 0u32),
            (false, true, 0),
            (false, false, 0),
            (true, false, 1),
        ] {
            let obj = t.junk(0x40, 6);
            let node = t.junk(0x20, 7);
            let spare = t.junk(0x20, 8);
            t.w32(node, spare);
            t.w32(obj + 0x18, if have { node } else { 0 });
            // the grow stub puts `node` in the list when it grows
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes());
            c.extend([0xc1, 0xe0, 0x05]);
            c.push(0x05);
            c.extend((log + 4).to_le_bytes());
            c.extend([0xc7, 0x00]);
            c.extend(more::FREE_LIST_GROW.to_le_bytes());
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes());
            if grows {
                c.extend([0xc7, 0x01]);
                c.extend(node.to_le_bytes()); // mov dword [ecx], node
            }
            c.extend([0xc2, 0x04, 0x00]);
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(more::FREE_LIST_GROW);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            t.stub(more::TREE_INSERT, true, 1, 1, clash);
            t.stub(more::FREE_LIST_FREE, true, 1, 1, 0xf1);
            t.stub(more::TREE_FINALIZE, true, 1, 1, 0xf2);
            let key = t.junk(4, 9);
            let value = t.junk(4, 10);
            t.check(
                &format!("{label}: shader insert {have} {grows} {clash}"),
                0x006d_f340,
                Conv::Thiscall(obj),
                &[key, value],
                |g| Out::U32(mb::shader_insert(g, obj, key, value)),
            );
            t.reset();
        }
        // the render target: out given or not, create failing, dimensions
        // failing, the target failing, made
        for (with_out, tex, dims, target) in [
            (true, 0u32, 0u32, 0u32),
            (true, 1, 3, 0),
            (false, 1, 0, 0),
            (true, 1, 0, 1),
            (false, 1, 0, 1),
        ] {
            let tx = t.junk(0x20, 11);
            t.w32(tx + 0x14, 0xffff_ffff);
            let tg = t.junk(0x20, 12);
            t.stub(
                more::CLUSTER_CREATE,
                true,
                3,
                3,
                if tex == 0 { 0 } else { tx },
            );
            t.stub(more::SET_DIMENSIONS, true, 4, 4, dims);
            t.stub(more::UPDATE_MAX_LEVEL, true, 0, 0, 0x55);
            t.stub(
                more::MAKE_TARGET,
                true,
                1,
                1,
                if target == 0 { 0 } else { tg },
            );
            let out = if with_out { t.junk(4, 13) } else { 0 };
            let args = [0xc1, 64, 32, 0x1c, 0x1ff, 0x2fe, out];
            t.check(
                &format!("{label}: make target {with_out} {tex} {dims} {target}"),
                0x00a0_6a30,
                Conv::Cdecl,
                &args,
                |g| {
                    let slot = g.scratch();
                    Out::U32(mb::make_target(g, args, slot))
                },
            );
            t.reset();
        }
        // the clear colour
        for argb in [0u32, 0xffff_ffff, 0x80ff_4001, 0x7f00_ff80] {
            let obj = t.junk(0x400, 14);
            t.check(
                &format!("{label}: colour {argb:#x}"),
                0x0059_9cc0,
                Conv::Thiscall(obj),
                &[argb],
                |g| {
                    mb::set_colour(g, obj, argb);
                    Out::None
                },
            );
        }
        // the shader parameter: no materials, no parameter, no data, out
        // of range, an offset under 4, copied
        for (count, found, data, off, size) in [
            (0u32, true, true, 8u16, 4u16),
            (2, false, true, 8, 4),
            (1, true, false, 8, 4),
            (1, true, true, 0x1c, 8),
            (1, true, true, 2, 4),
            (2, true, true, 8, 0xe004),
            (1, true, true, 4, 0x1c),
        ] {
            let inst = t.junk(0x20, 15);
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(inst + 0x10, arr);
            let list = t.zeros(8);
            let buffer = t.junk(0x40, 16);
            t.w32(buffer, 0x20);
            for k in 0..2 {
                let e = t.zeros(8);
                let mat = t.junk(0x10, 17);
                t.w32(e, mat);
                t.w32(e + 4, buffer);
                t.w32(list + 4 * k, e);
            }
            let p = t.zeros(0x10);
            t.write(p + 8, &off.to_le_bytes());
            t.write(p + 0xa, &size.to_le_bytes());
            t.stub(more::ARRAY_DATA, true, 0, 0, list);
            t.stub(more::FIND_PARAM, true, 1, 1, if found { p } else { 0 });
            t.stub(more::MEMCPY, false, 3, 0, 0x3c);
            let d = if data { t.junk(0x20, 18) } else { 0 };
            t.check(
                &format!("{label}: shader parameter {count} {found} {data} {off} {size:#x}"),
                0x006d_f8c0,
                Conv::Cdecl,
                &[inst, 0x4e4e, d],
                |g| {
                    mb::set_shader_parameter(g, inst, 0x4e4e, d);
                    Out::None
                },
            );
            t.reset();
        }
        // the shader map's reset: an empty map, one node, three
        for nodes in [0usize, 1, 3] {
            let tree = t.rebase(more::SHADER_TREE);
            let first = t.rebase(more::SHADER_TREE_FIRST);
            let ns: Vec<u32> = (0..nodes).map(|k| t.junk(0x20, 19 + k as u8)).collect();
            // the +0xc chain: first -> ... -> last -> tree; successor stub:
            // each node's word +0x10 is its successor
            for (k, &n) in ns.iter().enumerate() {
                t.w32(n, ns.get(k + 1).copied().unwrap_or(tree));
                t.w32(n + 0x10, ns.get(k + 1).copied().unwrap_or(tree));
            }
            t.put_saved(first, ns.first().copied().unwrap_or(tree));
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes());
            c.extend([0xc1, 0xe0, 0x05]);
            c.push(0x05);
            c.extend((log + 4).to_le_bytes());
            c.extend([0xc7, 0x00]);
            c.extend(more::TREE_SUCCESSOR.to_le_bytes());
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes());
            c.extend([0x8b, 0x42, 0x10]); // mov eax, [edx+0x10]
            c.extend([0xc2, 0x04, 0x00]);
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(more::TREE_SUCCESSOR);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            t.stub(more::RESTORE_SHADER, false, 1, 0, 0x7e);
            t.stub(more::TREE_ORPHAN_ALL, true, 0, 0, 0x7f);
            t.stub(more::FREE_LIST_RESET, true, 0, 0, 0x80);
            t.check(
                &format!("{label}: reset shaders {nodes}"),
                0x006d_f620,
                Conv::Cdecl,
                &[],
                |g| Out::U32(mb::reset_shaders(g)),
            );
            t.reset();
        }
    }
}

#[test]
fn more_text_and_bindings_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the application's find by name
        let names = ["alpha\0", "beta\0", "gamma\0"];
        let data = t.zeros(0x10);
        let mut objs = Vec::new();
        for (k, n) in names.iter().enumerate() {
            let s = t.alloc(n.as_bytes());
            let obj = t.zeros(8);
            t.w32(obj + 4, s + u32::from(k == 1));
            objs.push(obj);
            t.w32(data + 4 * k as u32, obj);
        }
        let arr = t.zeros(8);
        t.w32(arr, 3);
        t.w32(arr + 4, data);
        for q in ["beta\0", "gamma\0", "alph\0", "\0", "alphabet\0"] {
            let s = t.alloc(q.as_bytes());
            t.check(
                &format!("{label}: find {q:?}"),
                0x009e_ccb0,
                Conv::Thiscall(arr),
                &[s],
                |g| Out::U32(mb::find_by_name(g, arr, s)),
            );
        }
        // the text: null, equal, different, too long (the capacity grown
        // or failing), no buffer
        for (s, buffer, cap, strncmp, grow) in [
            (None, true, 0x10u32, 0u32, 0u32),
            (Some("hello\0"), true, 0x10, 0, 0),
            (Some("hello\0"), true, 0x10, 1, 0),
            (Some("a long text here\0"), true, 0x10, 1, 0),
            (Some("a long text here\0"), true, 0x10, 1, 0x1d),
            (Some("hi\0"), false, 0x10, 1, 0),
            (Some("hello\0"), true, 6, 1, 0),
        ] {
            let text = t.junk(0x200, 1);
            let buf = if buffer { t.junk(0x20, 2) } else { 0 };
            t.w32(text + 0x160, buf);
            t.w32(text + 0x15c, cap);
            let sp = match s {
                Some(x) => t.alloc(x.as_bytes()),
                None => 0,
            };
            t.stub_import(more::STRNCMP, 3, 0, strncmp);
            t.stub(more::TEXT_SET_CAPACITY, true, 1, 1, grow);
            t.stub(more::MEMCPY, false, 3, 0, 0x3c);
            t.stub(more::TEXT_CREATE_MESH, true, 1, 1, 0x3d);
            t.check(
                &format!("{label}: set text {s:?} {buffer} {cap} {strncmp} {grow}"),
                0x00a2_aa30,
                Conv::Thiscall(text),
                &[sp],
                |g| Out::U32(mb::set_text(g, text, sp)),
            );
            t.reset();
        }
        // the compute binding: each refusal, bound
        for (uav_flags, buf_flags, size, stride) in [
            (0u8, 8u8, 8u32, 4u32),
            (2, 0, 8, 4),
            (4, 8, 3, 4),
            (6, 8, 8, 8),
            (2, 0xff, 4, 4),
            (4, 8, 8, 4),
        ] {
            t.stub(more::UAV_OF, true, 1, 1, 0x0ab);
            t.stub(more::SRV_OF, true, 1, 1, 0x05b);
            let mut words = vec![0u32; 0x32];
            words[0x31] = 4;
            words[0x2f] = 3;
            let ctx = t.com_object(&words, &[]);
            let obj = t.junk(0x400, 3);
            t.w32(obj + 0x70, ctx);
            let buf = t.junk(0x10, 4);
            t.w8(buf + 8, buf_flags);
            t.w32(buf, size);
            t.w32(buf + 4, stride);
            let uav = t.junk(0x10, 5);
            t.w8(uav + 8, uav_flags);
            t.check(
                &format!("{label}: compute {uav_flags} {buf_flags} {size} {stride}"),
                0x0058_ebb0,
                Conv::Thiscall(obj),
                &[buf, uav],
                |g| Out::U32(mb::bind_compute_buffer(g, obj, buf, uav)),
            );
            t.reset();
        }
    }
}

// ---- batch 6: teardown, hand-off, resizes, copies ----

use ffx_prender::m5p_render_b_more2::{self as more2, bridge as nb};

#[test]
fn more2_teardown_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the render thread: events open or closed
        for mask in [0u32, 0b0101, 0b1111] {
            t.stub(more2::EVENT_CLOSE, true, 0, 0, 0xec);
            t.stub(more2::EH_VECTOR_DTOR, false, 4, 4, 0xe4);
            t.stub(more2::THREAD_BASE_DTOR, true, 0, 0, 0x7b);
            let obj = t.junk(0x200, 1);
            for k in 0..4 {
                t.w8(obj + 0x170 + 8 * k, u8::from((mask >> k) & 1 != 0) * 3);
            }
            t.check(
                &format!("{label}: render thread dtor {mask:#b}"),
                0x0061_4220,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    nb::render_thread_dtor(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
        // the command hand-off: a negative count (no array), owned / not
        // owned / null / same old buffers
        for (n, old_kind) in [(0xffff_ffffu32, 0u32), (3, 0), (3, 1), (3, 2), (0, 3)] {
            let ring = t.zeros(0x10);
            let block = t.junk(0x400, 2);
            t.stub(more2::RING_SINGLETON, false, 0, 0, ring);
            t.stub(more2::RING_ALLOCATE, true, 1, 1, block);
            t.stub(more2::MEMCPY, false, 3, 0, 0x3c);
            t.stub(more2::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.junk(0x2400, 3);
            t.w32(obj + 0x2318, 2);
            t.w32(obj + 0x2310, obj + 0x200c + 0x30);
            t.w32(obj + 0x2314, n);
            let job = t.junk(0x40, 4);
            let records = 2u32 << 5;
            let (count, old) = match old_kind {
                0 => (5, t.junk(0x10, 5)),
                1 => (0x8000_0005, t.junk(0x10, 6)),
                2 => (5, 0),
                _ => (5, block + 0x10 + records),
            };
            t.w32(job + 0x2c, count);
            t.w32(job + 0x30, old);
            t.check(
                &format!("{label}: hand off {n:#x} {old_kind}"),
                0x005b_99a0,
                Conv::Thiscall(obj),
                &[job],
                |g| {
                    nb::hand_off(g, obj, job);
                    Out::None
                },
            );
            t.reset();
        }
        // the character lighting's destructor
        for s in [
            more2::MAP_REMOVE_ALL,
            more2::TREE_ORPHAN_ALL,
            more2::TREE_CLEAR,
            more2::FREE_LIST_RESET,
            more2::FREE_LIST_DTOR,
            more2::CLUSTER_DTOR,
        ] {
            t.stub(s, true, 0, 0, s & 0xff);
        }
        let obj = t.junk(0x200, 7);
        t.check(
            &format!("{label}: lighting dtor"),
            0x006a_69b0,
            Conv::Thiscall(obj),
            &[],
            |g| {
                nb::character_lighting_dtor(g, obj);
                Out::None
            },
        );
        t.reset();
        // the program's COM objects: none, some, all
        for mask in [0u32, 0x5555_5555, 0xffff_ffff] {
            let obj = t.junk(0xc40, 8);
            let mut k = 0u32;
            let mut slots: Vec<u32> = Vec::new();
            for s in 0..8 {
                for off in [0, 4, 8, 0xc] {
                    slots.push(0x194 + 0x17c * s + off);
                }
            }
            slots.extend((0..11).map(|i| 4 * i));
            for at in slots {
                let o = if (mask >> (k % 32)) & 1 != 0 {
                    t.com_object(&[0, 0, 1], &[0, 0, k])
                } else {
                    0
                };
                t.w32(obj + at, o);
                k += 1;
            }
            t.check(
                &format!("{label}: program release {mask:#x}"),
                0x0059_c230,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(nb::program_release(g, obj)),
            );
        }
        // the wide name: null, empty, a name with a high byte, failing
        for (s, fails) in [
            (None, false),
            (Some(&b"\0"[..]), false),
            (Some(&b"Ab\xe9z\0"[..]), false),
            (Some(&b"name\0"[..]), true),
        ] {
            let buf = t.junk(0x20, 9);
            t.stub(
                more2::PHYRE_MALLOC,
                false,
                2,
                0,
                if fails { 0 } else { buf },
            );
            t.stub(more2::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let sp = match s {
                Some(x) => t.alloc(x),
                None => 0,
            };
            for f in [0x0059_7f20u32, 0x0059_7ff0] {
                t.check(
                    &format!("{label}: widen {f:#x} {s:?} {fails}"),
                    f,
                    Conv::Thiscall(0x1234),
                    &[sp],
                    |g| Out::U32(nb::widen(g, sp)),
                );
            }
            t.reset();
        }
        // the renderer job's destructor
        for (a, b) in [(3u32, 0x8000_0002u32), (0, 1)] {
            t.stub(more2::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(more2::RING_JOB_DTOR, true, 0, 0, 0x2b);
            let obj = t.junk(0x40, 10);
            t.w32(obj + 0x2c, a);
            t.w32(obj + 0x24, b);
            t.w32(obj + 0x1c, a);
            t.w32(obj + 0x20, 0);
            t.check(
                &format!("{label}: job dtor {a:#x} {b:#x}"),
                0x005b_4aa0,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    nb::renderer_job_dtor(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
        // the tweak utility's arrays and the text destructor
        for s in [
            more2::SPRITE_ARRAYS_RELEASE,
            more2::MESH_INSTANCE_DTOR,
            more2::WAIT_AND_CLEAR,
            more2::ARRAY40_RELEASE,
            more2::MESH_DTOR,
        ] {
            t.stub(s, true, 0, 0, s & 0xff);
        }
        t.stub(more2::PHYRE_FREE, false, 1, 0, 0xf4ee);
        let obj = t.junk(0x200, 11);
        t.w32(obj + 0x18, 0x8000_0001);
        t.w32(obj + 8, 0);
        t.check(
            &format!("{label}: release four"),
            0x006a_bc20,
            Conv::Thiscall(obj),
            &[],
            |g| {
                nb::release_four(g, obj);
                Out::None
            },
        );
        for with in [false, true] {
            let obj = t.junk(0x200, 12);
            let o = if with { t.object(&[0, 0, 0], &[]) } else { 0 };
            t.w32(obj + 0x164, o);
            t.check(
                &format!("{label}: text dtor {with}"),
                0x00a2_95a0,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    nb::text_dtor(g, obj);
                    Out::None
                },
            );
        }
        t.reset();
        // vorbis's lookup: null, tables or not
        for (null, rows, three) in [
            (true, false, false),
            (false, false, false),
            (false, true, true),
        ] {
            let free = t.stub_code(0xf4ee, false, 1, 0, 0);
            let at = t.rebase(more2::VORBIS_FREE);
            t.put_saved(at, free);
            t.stub(more2::MEMSET, false, 3, 0, 0x5e);
            let obj = if null { 0 } else { t.junk(0x40, 13) };
            if !null {
                let r = if rows {
                    let table = t.zeros(4 * 17);
                    for row in 0..17 {
                        let rr = t.junk(0x20, row as u8);
                        t.w32(table + 4 * row, rr);
                    }
                    table
                } else {
                    0
                };
                t.w32(obj + 8, r);
                let th = if three { t.junk(0xc, 14) } else { 0 };
                t.w32(obj + 0xc, th);
                t.w32(obj + 0x14, 0);
            }
            t.check(
                &format!("{label}: vorbis lookup {null} {rows} {three}"),
                0x00a9_7330,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    nb::vorbis_lookup_free(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
    }
}

#[test]
fn more2_packets_resizes_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the modifier packet: no record, slots with negative and missing
        // inputs
        for (code, n) in [
            (false, 0u32),
            (true, 0),
            (true, 4),
            (true, 0x8000_0003),
            (true, 0x100),
        ] {
            t.stub(more2::RUN_PACKET, false, 1, 0, 0x9a);
            let obj = t.junk(0x40, 1);
            let recs = t.zeros(0x40);
            let c = if code {
                let c = t.zeros(0x20);
                t.w32(c + 0x18, n);
                c
            } else {
                0
            };
            t.w32(recs + 0x10, c);
            t.w32(recs + 0x14, 2);
            t.w32(recs + 0x18, 1);
            t.w32(obj + 0x38, recs);
            let table = t.zeros(4 * 0x108);
            t.write(
                table,
                &[0u32, 1, 0xffff_ffff, 2, 0].map(u32::to_le_bytes).concat(),
            );
            t.w32(obj + 0x30, table);
            let outs = t.junk(0x40, 2);
            t.w32(obj + 0x14, outs);
            let streams = t.zeros(0x20);
            t.w32(obj + 0x20, streams);
            let in0 = t.junk(0x40, 3);
            let inputs = t.alloc(&[in0, 0, in0].map(u32::to_le_bytes).concat());
            let packet = t.junk(0x500, 4);
            let desc = t.junk(0x20, 5);
            t.w32(packet + 0x10, desc);
            t.write(packet + 0x10 + 0x12, &0x20u16.to_le_bytes());
            let info = t.junk(8, 6);
            let args = [1, packet, inputs, 0x99, info];
            t.check(
                &format!("{label}: modifier packet {code} {n:#x}"),
                0x005a_e1d0,
                Conv::Thiscall(obj),
                &args,
                |g| Out::U32(nb::modifier_packet(g, obj, 1, packet, inputs, info)),
            );
            t.reset();
        }
        // the glow and SSR resizes: missing handles, kinds, errors, done
        for (case, bound, unbind, set) in [
            (0u32, 0u32, 0u32, 0u32),
            (1, 0, 0, 0),
            (2, 1, 6, 0),
            (3, 0, 0, 0x22),
            (4, 0, 0, 0),
            (5, 1, 0, 0),
            (6, 0, 0, 0),
        ] {
            t.stub(more2::IS_BOUND, true, 0, 0, bound);
            t.stub(more2::UNBIND, true, 0, 0, unbind);
            t.stub(more2::SET_DIMENSIONS, true, 4, 4, set);
            let obj = t.junk(0x100, 7);
            for (k, at) in [0x78u32, 0x84, 0x7c, 0x88, 0x80, 0x8c, 0x68, 0x70, 0x6c]
                .into_iter()
                .enumerate()
            {
                let tex = t.junk(0x40, 8);
                t.w32(tex + 0x1c, if k % 2 == 0 { 40 } else { 7 });
                t.w32(tex + 0x20, if case == 6 { 23 } else { 22 });
                let handle = t.zeros(8);
                t.w32(handle, if case == 1 && k == 3 { 0 } else { tex });
                t.w8(handle + 4, u8::from(case == 4 && k == 7));
                let hv = if case == 0 && (k == 2 || k == 6) {
                    0
                } else {
                    handle
                };
                t.w32(obj + at, hv);
            }
            t.check(
                &format!("{label}: glow resize {case}"),
                0x00a2_33f0,
                Conv::Thiscall(obj),
                &[320, 180],
                |g| Out::U32(nb::glow_resize(g, obj, 320, 180)),
            );
            t.check(
                &format!("{label}: ssr resize {case}"),
                0x00a2_3780,
                Conv::Thiscall(obj),
                &[80, 44],
                |g| Out::U32(nb::ssr_resize(g, obj, 80, 44)),
            );
            t.reset();
        }
        // the network records: none, a null destination, arrays empty /
        // failing / copied
        for (n, null_dst, fails) in [
            (0u32, false, false),
            (1, true, false),
            (3, false, false),
            (2, false, true),
        ] {
            let buf = t.junk(0x80, 9);
            t.stub(
                more2::PHYRE_MALLOC,
                false,
                2,
                0,
                if fails { 0 } else { buf },
            );
            let src = t.junk(0x14 * 3, 10);
            for k in 0..3u32 {
                t.w32(src + 0x14 * k + 0xc, [0u32, 3, 0x8000_0002][k as usize]);
                let from = t.junk(0x20, 11);
                t.w32(src + 0x14 * k + 0x10, from);
            }
            let dst = if null_dst { 0 } else { t.junk(0x14 * 3, 12) };
            t.check(
                &format!("{label}: copy networks {n} {null_dst} {fails}"),
                0x005b_c500,
                Conv::Cdecl,
                &[dst, src, n],
                |g| {
                    nb::copy_networks(g, dst, src, n);
                    Out::None
                },
            );
            t.reset();
        }
        // the 64-bit clear colour
        for (lo, hi) in [
            (0u32, 0u32),
            (0xffff_ffff, 0xffff_ffff),
            (0x80ff_4001, 0x1234_5678),
        ] {
            let obj = t.junk(0x400, 13);
            t.check(
                &format!("{label}: colour64 {lo:#x} {hi:#x}"),
                0x0059_9d80,
                Conv::Thiscall(obj),
                &[lo, hi],
                |g| {
                    nb::set_colour64(g, obj, lo, hi);
                    Out::None
                },
            );
        }
        // the render interface's teardown: with and without a context
        for (ctx, bufs) in [(false, false), (true, true)] {
            for s in [
                more2::SCENE_RELEASE,
                more2::SHADERS_RELEASE,
                more2::PROGRAM_RELEASE,
                more2::TARGET_RELEASE_VIEWS,
                more2::SHADER_RELEASE_VIEWS,
                more2::SAMPLER_RELEASE,
                more2::COM_PAIRS,
                more2::COM_EIGHT,
            ] {
                t.stub(s, true, 0, 0, s & 0xff);
            }
            t.stub(more2::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let obj = t.junk(0x1400, 14);
            let mut words = vec![0u32; 0x70];
            words[0x84 / 4] = 4;
            words[0x1b8 / 4] = 1;
            words[0x1bc / 4] = 1;
            words[2] = 1;
            let c = if ctx { t.com_object(&words, &[]) } else { 0 };
            t.w32(obj + 0x70, c);
            for at in [0x340u32, 0x344] {
                t.w32(obj + at, if bufs { 0x1000 + at } else { 0 });
            }
            for at in [0x378u32, 0x74, 0x6c] {
                let o = if ctx {
                    t.com_object(&[0, 0, 1], &[0, 0, at])
                } else {
                    0
                };
                t.w32(obj + at, o);
            }
            t.check(
                &format!("{label}: interface release {ctx} {bufs}"),
                0x0059_c400,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    nb::interface_release(g, obj);
                    Out::None
                },
            );
            t.reset();
        }
    }
}

// ---- batch 7: locals handed to callees ----

use ffx_prender::m5p_render_b_frames::{self as frames, bridge as fb};

impl Twins {
    /// Where the original's `ebp` lands for a call with `words` stack
    /// arguments.
    fn ebp_for(&self, words: u32) -> u32 {
        assert_eq!(self.a.stack_range(), self.b.stack_range());
        self.a.stack_range().end - 0x100 - 4 * words - 8
    }

    /// [`Twins::check`] with the port's calls below the original's frame
    /// (`ebp - below`), then the frame bytes `ebp - from .. ebp - to`
    /// compared.
    #[allow(clippy::too_many_arguments)]
    fn check_frame(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        ebp: u32,
        below: u32,
        local: (u32, u32),
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let (from, to) = local;
        let junk: Vec<u8> = (0..from - to).map(|k| 0xa5 ^ k as u8).collect();
        self.write(ebp - from, &junk);
        self.b.set_call_top(Some(ebp - below - 0x100));
        self.check(what, sym, conv, args, port);
        self.b.set_call_top(None);
        let mut x = vec![0; (from - to) as usize];
        let mut y = vec![0; (from - to) as usize];
        self.a.read_bytes(ebp - from, &mut x);
        self.b.read_bytes(ebp - from, &mut y);
        assert_eq!(x, y, "{what}: frame");
    }
}

/// A material list for the parameter copies: `count` materials over one
/// buffer of 0x20 bytes; the parameter's offset and size.
fn materials(t: &mut Twins, count: u32, found: bool, off: u16, size: u16) -> u32 {
    let arr = t.zeros(8);
    t.w32(arr, count);
    let list = t.zeros(8);
    let buffer = t.junk(0x40, 1);
    t.w32(buffer, 0x20);
    for k in 0..2 {
        let e = t.zeros(8);
        let mat = t.junk(0x10, 2);
        t.w32(e, mat);
        t.w32(e + 4, buffer);
        t.w32(list + 4 * k, e);
    }
    let p = t.zeros(0x10);
    t.write(p + 8, &off.to_le_bytes());
    t.write(p + 0xa, &size.to_le_bytes());
    t.stub(frames::ARRAY_DATA, true, 0, 0, list);
    t.stub(frames::FIND_PARAM, true, 1, 1, if found { p } else { 0 });
    t.stub(frames::MEMCPY, false, 3, 0, 0x3c);
    arr
}

#[test]
fn frames_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the shader parameter vector
        for (count, found, off, size) in [
            (0u32, true, 8u16, 4u16),
            (2, false, 8, 4),
            (2, true, 8, 0x10),
            (1, true, 0x1c, 8),
            (2, true, 8, 0xe004),
            (1, true, 2, 4),
        ] {
            let arr = materials(&mut t, count, found, off, size);
            let inst = t.junk(0x20, 3);
            t.w32(inst + 0x10, arr);
            let v: Vec<u32> = [1.5f32, -0.0, f32::from_bits(0x7fc0_0001), 3.0]
                .iter()
                .map(|f| t.alloc(&f.to_bits().to_le_bytes()))
                .collect();
            let v = [v[0], v[1], v[2], v[3]];
            let args = [inst, 0x4e4e, v[0], v[1], v[2], v[3]];
            let ebp = t.ebp_for(6);
            t.check_frame(
                &format!("{label}: vector {count} {found} {off} {size}"),
                0x006d_fa30,
                Conv::Cdecl,
                &args,
                ebp,
                0x14,
                (0x14, 0),
                |g| {
                    fb::set_vector(g, ebp, inst, 0x4e4e, v);
                    Out::None
                },
            );
            t.reset();
        }
        // the fog effect over the fog's tree: empty, one node, three
        for nodes in [0usize, 1, 3] {
            let arr = materials(&mut t, 1, true, 8, 0xc);
            let fog = t.junk(0xb8, 4);
            let tree = t.zeros(0x20);
            t.w32(fog + 0xa8, tree);
            let ns: Vec<u32> = (0..nodes).map(|k| t.junk(0x20, 5 + k as u8)).collect();
            for (k, &n) in ns.iter().enumerate() {
                t.w32(n, ns.get(k + 1).copied().unwrap_or(tree));
                t.w32(n + 0x10, ns.get(k + 1).copied().unwrap_or(tree));
                t.w32(n + 0xc, arr);
            }
            t.w32(tree + 0xc, ns.first().copied().unwrap_or(tree));
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes());
            c.extend([0xc1, 0xe0, 0x05]);
            c.push(0x05);
            c.extend((log + 4).to_le_bytes());
            c.extend([0xc7, 0x00]);
            c.extend(frames::TREE_NEXT.to_le_bytes());
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes());
            c.extend([0x8b, 0x42, 0x10]); // mov eax, [edx+0x10]
            c.extend([0xc2, 0x04, 0x00]);
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(frames::TREE_NEXT);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            let ebp = t.ebp_for(1);
            t.check_frame(
                &format!("{label}: fog effect {nodes}"),
                0x006e_35c0,
                Conv::Thiscall(fog),
                &[0x3f00_0000],
                ebp,
                0x1c,
                (0x1c, 4),
                |g| {
                    fb::set_chr_fog_effect(g, ebp, fog, 0x3f00_0000);
                    Out::None
                },
            );
            t.reset();
        }
        // the UAV clear: no context, i past the views, no view, cleared
        for (ctx, count, i, view) in [
            (false, 2u32, 0u32, 1u32),
            (true, 2, 2, 1),
            (true, 0, 0, 0),
            (true, 0x8000_0003, 1, 7),
        ] {
            t.stub(frames::UAV_OF, true, 1, 1, view);
            let mut words = vec![0u32; 0x34];
            words[0x33] = 3;
            let c = if ctx { t.com_object(&words, &[]) } else { 0 };
            let obj = t.junk(0x80, 6);
            t.w32(obj + 0x70, c);
            let buffer = t.junk(0x20, 7);
            t.w32(buffer + 0x14, count);
            let ebp = t.ebp_for(3);
            t.check_frame(
                &format!("{label}: clear uav {ctx} {count:#x} {i} {view}"),
                0x0058_e510,
                Conv::Thiscall(obj),
                &[buffer, 0x3f80_0000, i],
                ebp,
                0x18,
                (0x18, 4),
                |g| Out::U32(fb::clear_uav(g, ebp, obj, buffer, 0x3f80_0000, i)),
            );
            t.reset();
        }
        // the motion-blur setup: already set, the target failing, a
        // result word, made
        for (have, target, result) in [
            (true, 1u32, 0u32),
            (false, 0, 0),
            (false, 1, 9),
            (false, 0, 9),
            (false, 1, 0),
        ] {
            let tex = t.junk(0x20, 8);
            let tg = t.zeros(8);
            t.w32(tg, tex);
            // the make-target stub writes `result` through its last word
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes());
            c.extend([0xc1, 0xe0, 0x05]);
            c.push(0x05);
            c.extend((log + 4).to_le_bytes());
            c.extend([0xc7, 0x00]);
            c.extend(frames::MAKE_TARGET.to_le_bytes());
            for k in 0..6u8 {
                c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp+4+4k]
                c.extend([0x89, 0x50, 8 + 4 * k]); // mov [eax+8+4k], edx
            }
            c.extend([0x8b, 0x54, 0x24, 0x1c]); // mov edx, [esp+0x1c] (out)
            c.extend([0x85, 0xd2]); // test edx, edx
            c.extend([0x74, 0x06]); // je +6
            c.extend([0xc7, 0x02]);
            c.extend(result.to_le_bytes()); // mov dword [edx], result
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes());
            c.push(0xb8);
            c.extend((if target == 0 { 0 } else { tg }).to_le_bytes());
            c.push(0xc3);
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(frames::MAKE_TARGET);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            t.stub(frames::UPDATE_MAX_LEVEL, true, 0, 0, 0x55);
            t.stub(frames::BLUR_SET_TARGET, true, 2, 2, 0x5e7);
            let obj = t.junk(0x1000, 9);
            t.w32(obj + 0xd4c, u32::from(have) * 0x44);
            let ebp = t.ebp_for(3);
            t.check_frame(
                &format!("{label}: blur setup {have} {target} {result}"),
                0x00a1_7e00,
                Conv::Thiscall(obj),
                &[0xc1, 64, 32],
                ebp,
                4,
                (4, 0),
                |g| Out::U32(fb::blur_setup(g, ebp, obj, 0xc1, 64, 32)),
            );
            t.reset();
        }
        // the light direction: ambient, not set up, a directional light,
        // another kind
        let dir = t.rebase(frames::LIGHT_KIND_DIRECTIONAL);
        for (i, set_up, kind) in [(3u32, 1u8, dir), (0, 0, dir), (1, 2, dir), (1, 1, 0)] {
            t.stub(frames::MATRIX_FROM_3X4, true, 1, 1, 0x30);
            t.stub(frames::MATRIX_ROTATE3, true, 2, 2, 0x31);
            let obj = t.junk(0x200, 10);
            t.w8(obj + 0x1cc, set_up);
            let list = t.zeros(8);
            let e = t.junk(0x20, 11);
            t.w32(e + 0x18, kind);
            t.w32(list + 4 * i.min(1), e);
            let lights = t.zeros(8);
            t.w32(lights + 4, list);
            t.w32(obj + 0xf0, lights);
            let out = t.junk(0x10, 12);
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: light direction {i} {set_up} {kind:#x}"),
                0x006a_84d0,
                Conv::Thiscall(obj),
                &[out, i],
                ebp,
                0x50,
                (0x10, 4),
                |g| Out::U32(fb::light_direction(g, ebp, obj, out, i)),
            );
            t.reset();
        }
        // the compute tables: no UAV, a UAV, a slot, feature levels
        for (uav, slot, level) in [
            (0u32, 0xffff_ffffu32, 0xb000u32),
            (0x77, 0xffff_ffff, 0xa100),
            (0x77, 2, 0xb100),
            (0, 3, 0xb000),
        ] {
            let mut words = vec![0u32; 0x45];
            words[0xc0 / 4] = 7;
            words[0x110 / 4] = 5;
            words[0x5c / 4] = 4;
            words[0x58 / 4] = 4;
            words[0x7c / 4] = 4;
            words[0x80 / 4] = 4;
            let ctx = t.com_object(&words, &[]);
            let r = t.junk(0x100, 13);
            t.w32(r + 0x70, ctx);
            t.w32(r + 0xb4, level);
            let rp = t.zeros(4);
            t.w32(rp, r);
            let obj = t.junk(0x500, 14);
            t.w32(obj, uav);
            t.w32(obj + 4, 0x8000_0003);
            t.w32(obj + 0x18, slot);
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: bind tables {uav:#x} {slot:#x} {level:#x}"),
                0x0059_3380,
                Conv::Thiscall(obj),
                &[rp, 0x99],
                ebp,
                0x20,
                (0x20, 0),
                |g| Out::U8(fb::bind_tables(g, ebp, obj, rp)),
            );
            t.reset();
        }
    }
}

// ---- batch 8: the morph blends ----

use ffx_prender::m5p_render_b_morph::{self as morph, bridge as gb};

#[test]
fn morphs_match_the_original() {
    assert_eq!(morph::MORPHS.len(), 9);
    for (label, mut t) in Twins::all() {
        for &(f, m) in &morph::MORPHS {
            assert_eq!(morph::morph_of(f), m);
            // which weights are 0: none, one left, two, all; a weight of
            // 1.0 alone, -0.0, a NaN
            let z = 0u32;
            let cases: [[u32; 4]; 8] = [
                [z, z, z, z],
                [0x3f00_0000, z, z, z],
                [z, 0x3f80_0000, z, z],
                [0x8000_0000, z, 0xbf40_0000, z],
                [0x3e80_0000, 0x3f00_0000, z, z],
                [0x3e80_0000, 0x3f00_0000, 0x3f40_0000, z],
                [0x3e80_0000, 0x3f00_0000, 0x3f40_0000, 0x3fc0_0000],
                [0x7fc0_0000, z, z, z],
            ];
            for (c, w) in cases.iter().enumerate() {
                for n in [0u32, 1, 5] {
                    t.stub(morph::MEMCPY, false, 3, 0, 0x3c);
                    t.stub(morph::MEMSET, false, 3, 0, 0x3d);
                    t.stub(morph::BLEND2, false, 4, 0, 0xb2);
                    t.stub(morph::BLEND3, false, 4, 0, 0xb3);
                    t.stub(morph::BLEND4, false, 4, 0, 0xb4);
                    let weights = t.alloc(&w.map(u32::to_le_bytes).concat());
                    let srcs: Vec<u32> = [1.0f32, 2.0, 3.0, 4.0]
                        .into_iter()
                        .map(|k| {
                            let v: Vec<u8> = (0..0x60u32)
                                .flat_map(|i| {
                                    (f32::from_bits(0x3f80_0000 + i * 0x0001_1111) * k)
                                        .to_le_bytes()
                                })
                                .collect();
                            t.alloc(&v)
                        })
                        .collect();
                    let streams = t.alloc(
                        &[srcs[0], srcs[1], srcs[2], srcs[3]]
                            .map(u32::to_le_bytes)
                            .concat(),
                    );
                    let dst = t.junk(0x200, 3);
                    let out = t.zeros(4);
                    t.w32(out, dst);
                    let ebp = t.ebp_for(4);
                    let (wl, sl) = morph::locals(m.k);
                    t.check_frame(
                        &format!("{label}: morph {f:#x} case {c} n {n}"),
                        f,
                        Conv::Cdecl,
                        &[weights, n, streams, out],
                        ebp,
                        sl.max(wl),
                        (sl.max(wl), 4),
                        |g| {
                            let r = gb::morph(g, ebp, m, weights, n, streams, out);
                            // no vertices: the original leaves the caller's eax or its cookie
                            if n == 0 { Out::None } else { Out::U32(r) }
                        },
                    );
                    t.reset();
                }
            }
        }
    }
}

// ---- batch 9: compute and geometry shader programs ----

use ffx_prender::m5p_render_b_shaders::{self as shaders, bridge as hb};

#[test]
fn shader_programs_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0xd3d1_0b09);
        for case in 0..48u32 {
            let (sym, k) = if case % 2 == 1 {
                (0x0058_d350, shaders::GEOMETRY)
            } else {
                (0x0058_d030, shaders::COMPUTE)
            };
            let mut words = vec![0u32; 24];
            words[k.slot as usize] = 5;
            let dev = t.com_object(&words, &[]);
            let holder = t.rebase(shaders::HOLDER_DEVICE);
            t.put_saved(holder, if case % 7 == 3 { 0 } else { dev });
            t.stub(shaders::SCOPE_ENTER, true, 0, 0, 0x4d50);
            t.stub(shaders::SCOPE_LEAVE, true, 0, 0, 0x4e90);
            t.stub(shaders::HOLDER_CTOR, true, 0, 0, 0);
            t.stub(shaders::ATEXIT, false, 1, 0, 0);
            t.stub(shaders::PRINTF, false, 2, 0, 0);
            let guard = t.rebase(shaders::HOLDER_GUARD);
            let g = (rng.next() & !1) | u32::from(case % 3 == 0);
            t.put_saved(guard, g);
            let ctx = t.junk(0x100, case as u8);
            t.w32(ctx + 0x6c, dev);
            let level =
                [0x9300u32, 0xa0ff, 0xa100, 0xafff, 0xb000, 0xb100][(case / 2 % 6) as usize];
            t.w32(ctx + 0xb4, level);
            t.stub(shaders::RENDERER_CONTEXT, false, 0, 0, ctx);
            let sampler = t.rebase(shaders::HOLDER_SAMPLER);
            t.put_saved(sampler, rng.next());
            t.stub(shaders::CONSTANT_BUFFER_CREATE, true, 0, 0, 0xcb);
            let this = t.junk(0x500, case as u8 ^ 0x40);
            t.w32(this + 0xc, 0x100 | ((case % 3) << 30));
            t.w32(this + 0x4e0, if case % 4 == 2 { 0x0bad_0000 } else { 0 });
            let profile = [
                k.profile_50,
                k.profile_41,
                0x1234_5678,
                k.profile_50 ^ k.profile_41,
            ][(case / 12 % 4) as usize];
            t.w32(this + 0x4dc, profile);
            let ebp = t.ebp_for(0);
            let lock = ebp - 0x10;
            t.check_frame(
                &format!("{label}: shader program case {case}"),
                sym,
                Conv::Thiscall(this),
                &[],
                ebp,
                0x10,
                (0x10, 0xc),
                |g| Out::U32(hb::create(g, this, lock, k)),
            );
            t.reset();
        }
    }
}

#[test]
fn restore_shaders_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (f, r) in [
            (0x006d_f6d0u32, shaders::SHADER_MANAGER),
            (0x006e_5ee0, shaders::PS2_LIGHTING),
        ] {
            // no entry; a material saved or not; objects 0, 1, 3 (owned or
            // not); the instance found deeper in either map or absent
            for case in 0..6u32 {
                let inst = t.junk(0x40, case as u8);
                let mat_node = t.junk(0x20, 1);
                let obj_node = t.junk(0x20, 2);
                t.w32(mat_node + 0x10, if case % 2 == 0 { 0x3333 } else { 0 });
                let count = [0u32, 1, 3, 0x8000_0003, 3, 2][case as usize];
                let objs = t.zeros(8);
                let buf = t.junk(0x20, 3);
                t.w32(objs, count);
                t.w32(
                    objs + 4,
                    if count & 0x7fff_ffff == 1 {
                        0x4141
                    } else {
                        buf
                    },
                );
                t.w32(obj_node + 0x10, objs);
                // the find stub returns its tag's node: the first map's
                // the material node, the second's the objects node
                let log = t.log;
                let mut c = vec![0xa1];
                c.extend(log.to_le_bytes());
                c.extend([0xc1, 0xe0, 0x05]);
                c.push(0x05);
                c.extend((log + 4).to_le_bytes());
                c.extend([0xc7, 0x00]);
                c.extend(shaders::TREE_FIND.to_le_bytes());
                c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
                c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
                c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
                c.extend([0xff, 0x05]);
                c.extend(log.to_le_bytes());
                let tree = t.rebase(r.tree);
                c.push(0xb8);
                c.extend(obj_node.to_le_bytes()); // mov eax, obj_node
                c.extend([0x81, 0xf9]);
                c.extend(tree.to_le_bytes()); // cmp ecx, tree
                c.extend([0x75, 0x05]); // jne +5
                c.push(0xb8);
                c.extend((if case == 5 { 0 } else { mat_node }).to_le_bytes()); // mov eax, mat_node
                c.extend([0xc2, 0x04, 0x00]);
                let code = t.a.alloc_code(&c).unwrap();
                assert_eq!(t.b.alloc_code(&c).unwrap(), code);
                let at = t.rebase(shaders::TREE_FIND);
                let old = Oracle::read(&mut t.a, at, 5).unwrap();
                t.saved.entry(at).or_insert(old);
                let mut j = vec![0xe9];
                j.extend(code.wrapping_sub(at + 5).to_le_bytes());
                t.write(at, &j);
                t.stub(shaders::DELETE_OBJECT, true, 1, 1, 0xde);
                t.stub(shaders::TREE_REMOVE, true, 1, 1, 0x7e);
                t.stub(shaders::FREE_LIST_FREE, true, 1, 1, 0xf1);
                t.stub(shaders::PHYRE_FREE, false, 1, 0, 0xf4ee);
                // the maps: a root whose key is less (go right), then the
                // instance (or a leaf back at the tree)
                for tr in [r.tree, r.tree2] {
                    let tree = t.rebase(tr);
                    let root = t.junk(0x20, 4);
                    let found = t.junk(0x20, 5);
                    t.w32(root + 0xc, 0);
                    t.w32(root + 4, if case == 3 { tree } else { found });
                    t.w32(found + 0xc, inst);
                    t.put_saved(tree + 0xc, root);
                }
                let ebp = t.ebp_for(1);
                let key = ebp + 8;
                t.check(
                    &format!("{label}: restore {f:#x} case {case}"),
                    f,
                    Conv::Cdecl,
                    &[inst],
                    |g| {
                        shaders::restore::restore(g, r, inst, key);
                        Out::None
                    },
                );
                t.reset();
            }
        }
    }
}

// ---- batch 10: particle arrays, lighting buffers, FXAA setup ----

use ffx_prender::m5p_render_b_post3::{self as post3, bridge as kb};

#[test]
fn post3_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the particle arrays: owned, not owned, null
        t.stub(post3::PHYRE_FREE, false, 1, 0, 0xf4ee);
        let obj = t.junk(0x3080, 1);
        for (k, at) in [0x3060u32, 0x3058, 0x3050, 0x3048, 0x3040]
            .into_iter()
            .enumerate()
        {
            t.w32(obj + at, [3u32, 0x8000_0002, 1, 0, 5][k]);
            t.w32(obj + at + 4, if k == 3 { 0 } else { 0x1000 + at });
        }
        t.check(
            &format!("{label}: particles release"),
            0x009f_e3a0,
            Conv::Thiscall(obj),
            &[],
            |g| {
                kb::particles_release(g, obj);
                Out::None
            },
        );
        t.reset();
        // the lighting buffers: none, bound or not, a dtor
        for (mask, bound) in [(0u32, 0u32), (0x3f, 1), (0x15, 0), (0x2a, 0x100)] {
            for k in [post3::CONSTANT, post3::STRUCTURED] {
                t.stub(k.is_bound, true, 0, 0, bound);
                t.stub(k.unbind, true, 0, 0, 0x0b);
                t.stub(k.dtor, true, 0, 0, 0x0d);
            }
            t.stub(post3::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(post3::LIGHTING_RELEASE, true, 0, 0, 0x4a60);
            let obj = t.junk(0x4040, 2);
            for (k, at) in [0x3fc8u32, 0x3fcc, 0x3fd0, 0x3fd4, 0x4034, 0x4038]
                .into_iter()
                .enumerate()
            {
                let b = if (mask >> k) & 1 != 0 {
                    t.junk(0x20, k as u8)
                } else {
                    0
                };
                t.w32(obj + at, b);
            }
            t.check(
                &format!("{label}: lighting buffers {mask:#x} {bound}"),
                0x00a2_4b00,
                Conv::Thiscall(obj),
                &[],
                |g| Out::U32(kb::lighting_buffers_release(g, obj)),
            );
            // the resize: no size, a failing first, both made, malloc failing
            for (w, hgt, r1, fail) in [
                (0u32, 4u32, 0u32, false),
                (8, 0, 0, false),
                (8, 4, 7, false),
                (8, 4, 0, false),
                (8, 4, 0, true),
            ] {
                let b = if mask & 4 != 0 { t.junk(0x20, 9) } else { 0 };
                t.w32(obj + 0x3fd0, b);
                t.w32(obj + 0x3fd4, 0);
                let p = t.junk(0x48, 10);
                t.stub(post3::PHYRE_MALLOC, false, 1, 0, if fail { 0 } else { p });
                t.stub(post3::STRUCTURED_CTOR, true, 0, 0, 0xc7);
                t.stub(post3::STRUCTURED_CREATE, true, 4, 4, r1);
                t.check(
                    &format!("{label}: lighting resize {mask:#x} {w} {hgt} {r1} {fail}"),
                    0x00a1_0590,
                    Conv::Thiscall(obj),
                    &[w, hgt],
                    |g| Out::U32(kb::lighting_buffers_resize(g, obj, w, hgt)),
                );
            }
            t.reset();
        }
        // the FXAA setup: each step failing in turn, made; defaults
        for (case, w, hgt) in [
            (0u32, 0u32, 0u32),
            (1, 640, 0),
            (2, 0, 360),
            (3, 640, 360),
            (4, 640, 360),
            (5, 640, 360),
        ] {
            t.stub(post3::INTERFACE_CTOR, true, 0, 0, 0);
            t.stub(post3::ATEXIT, false, 1, 0, 0);
            let guard = t.rebase(post3::INTERFACE_GUARD);
            let g = t.r32(guard);
            t.put_saved(guard, if case % 2 == 0 { g & !1 } else { g | 1 });
            let dw = t.rebase(post3::DEFAULT_WIDTH);
            t.put_saved(dw, 1280);
            let dh = t.rebase(post3::DEFAULT_HEIGHT);
            t.put_saved(dh, 720);
            let tex = t.junk(0x40, 11);
            t.w32(tex + 0x14, 0xffff_ffff);
            let list = t.junk(0x40, 12);
            let target = t.junk(0x20, 13);
            t.w32(list + 0x34, if case == 4 { 0 } else { target });
            t.stub(
                post3::CLUSTER_CREATE,
                true,
                3,
                3,
                if case == 1 { 0 } else { tex },
            );
            t.stub(post3::SET_DIMENSIONS, true, 4, 4, u32::from(case == 2) * 9);
            t.stub(post3::UPDATE_MAX_LEVEL, true, 0, 0, 0x55);
            t.stub(
                post3::ALLOCATE_INSTANCES,
                true,
                3,
                3,
                if case == 3 { 0 } else { list },
            );
            t.stub(post3::TARGET_CTOR, true, 1, 1, 0x56);
            t.stub(post3::COLOURED_TRIANGLE, false, 1, 0, 0x7717);
            let obj = t.junk(0x80, 14);
            t.check(
                &format!("{label}: fxaa setup {case} {w} {hgt}"),
                0x00a1_9970,
                Conv::Thiscall(obj),
                &[w, hgt],
                |g| Out::U32(kb::fxaa_setup(g, obj, w, hgt)),
            );
            t.reset();
        }
    }
}

// ---- batch 11: SSR targets, glow resize, buffer views ----

use ffx_prender::m5p_render_b_post4::{self as post4, bridge as jb};

#[test]
fn post4_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the SSR targets: no triangle; textures, sizes, targets failing
        for case in 0..5u32 {
            let tex = t.junk(0x40, 1);
            t.w32(tex + 0x14, 0xffff_ffff);
            let tg = t.junk(0x20, 2);
            t.stub(
                post4::CREATE_TRIANGLE,
                false,
                1,
                0,
                u32::from(case != 0) * 0x7171,
            );
            t.stub(
                post4::CLUSTER_CREATE,
                true,
                3,
                3,
                if case == 1 { 0 } else { tex },
            );
            t.stub(post4::SET_DIMENSIONS, true, 4, 4, u32::from(case == 2) * 3);
            t.stub(post4::UPDATE_MAX_LEVEL, true, 0, 0, 0x55);
            t.stub(
                post4::MAKE_TARGET,
                true,
                1,
                1,
                if case == 3 { 0 } else { tg },
            );
            let obj = t.junk(0x80, 3);
            t.check(
                &format!("{label}: ssr targets {case}"),
                0x00a1_7ec0,
                Conv::Thiscall(obj),
                &[0xc1, 641, 361],
                |g| Out::U32(jb::ssr_targets(g, obj, 0xc1, 641, 361)),
            );
            t.reset();
        }
        // the glow resize: a missing level handle, a kind, errors, the
        // bloom target null / missing / resized, all done
        for (case, bound, unbind, set) in [
            (0u32, 0u32, 0u32, 0u32),
            (1, 0, 0, 0),
            (2, 1, 6, 0),
            (3, 0, 0, 0x22),
            (4, 0, 0, 0),
            (5, 1, 0, 0),
            (6, 0, 0, 0),
        ] {
            t.stub(post4::IS_BOUND, true, 0, 0, bound);
            t.stub(post4::UNBIND, true, 0, 0, unbind);
            t.stub(post4::SET_DIMENSIONS, true, 4, 4, set);
            let obj = t.junk(0x100, 4);
            let ats = [0x90u32, 0x9c, 0x94, 0xa0, 0x98, 0xa4, 0xa8];
            for (k, at) in ats.into_iter().enumerate() {
                let tex = t.junk(0x40, 5);
                t.w32(tex + 0x1c, if k % 2 == 0 { 40 } else { 7 });
                t.w32(tex + 0x20, if case == 6 { 23 } else { 22 });
                let handle = t.zeros(8);
                t.w32(handle, if case == 1 && k == 3 { 0 } else { tex });
                t.w8(handle + 4, u8::from(case == 4 && k == 6));
                let hv = if case == 0 && k == 2 { 0 } else { handle };
                t.w32(obj + at, hv);
            }
            if case == 5 {
                t.w32(obj + 0xa8, 0);
            }
            t.check(
                &format!("{label}: glow resize {case}"),
                0x00a2_3280,
                Conv::Thiscall(obj),
                &[320, 180],
                |g| Out::U32(jb::glow_resize(g, obj, 320, 180)),
            );
            t.reset();
        }
        // the buffer views: no views flag, one inline, three, a failure
        for (flags, count, hr) in [
            (0u8, 3u32, 0u32),
            (8, 1, 0),
            (8, 3, 0),
            (8, 3, 0x8007_000e),
            (8, 3, 1),
            (0xff, 2, 0),
        ] {
            let mut words = vec![0u32; 8];
            words[7] = 4;
            let dev = t.com_object(&words, &[0, 0, 0, 0, 0, 0, 0, hr]);
            let holder = t.rebase(post4::HOLDER_DEVICE);
            t.put_saved(holder, dev);
            t.stub(post4::SCOPE_ENTER, true, 0, 0, 0x4d50);
            t.stub(post4::SCOPE_LEAVE, true, 0, 0, 0x4e90);
            t.stub(post4::HOLDER_CTOR, true, 0, 0, 0);
            t.stub(post4::ATEXIT, false, 1, 0, 0);
            let guard = t.rebase(post4::HOLDER_GUARD);
            let g = t.r32(guard);
            t.put_saved(guard, if count == 3 { g & !1 } else { g | 1 });
            let obj = t.junk(0x40, 6);
            t.w8(obj + 0x34, flags);
            t.w32(obj + 0x14, count);
            let elems = t.junk(0x14 * 3, 7);
            if count > 1 {
                t.w32(obj + 0x18, elems);
            }
            let ebp = t.ebp_for(0);
            let lock = ebp - 0x18;
            t.check_frame(
                &format!("{label}: buffer views {flags:#x} {count} {hr:#x}"),
                0x0058_f5d0,
                Conv::Thiscall(obj),
                &[],
                ebp,
                0x18,
                (0x18, 0xc),
                |g| Out::U32(jb::create_views(g, obj, lock)),
            );
            t.reset();
        }
    }
}

// ---- batch 12: record insert, buffer upload, draw set-up ----

use ffx_prender::m5p_render_b_d3d::{self as d3d, bridge as eb};

#[test]
fn d3d_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the insert: present, an empty slot, growing (owned, not owned,
        // failing), an empty list
        for (case, count, key) in [
            (0u32, 3u32, 0x22u32),
            (1, 3, 0x99),
            (2, 3, 0x99),
            (3, 0x8000_0003, 0x99),
            (4, 3, 0x99),
            (5, 0, 0x99),
        ] {
            let buf = t.junk(0x80, 1);
            t.stub(
                d3d::PHYRE_MALLOC,
                false,
                2,
                0,
                if case == 4 { 0 } else { buf },
            );
            t.stub(d3d::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let data = t.junk(0x40, 2);
            for k in 0..3u32 {
                t.w32(
                    data + 16 * k,
                    if case == 1 && k == 1 {
                        0
                    } else {
                        0x11 * (k + 1)
                    },
                );
            }
            let list = t.zeros(8);
            t.w32(list, count);
            t.w32(list + 4, data);
            let args = [list, key, 0xa, 0xb, 0xc];
            t.check(
                &format!("{label}: insert {case}"),
                0x006a_b950,
                Conv::Stdcall,
                &args,
                |g| Out::U32(eb::insert(g, list, [key, 0xa, 0xb, 0xc])),
            );
            t.reset();
        }
        // the upload: nothing pending; map failing, no pointer; dynamic or
        // not; one element inline or several
        for (case, pending, hr, ptr, dynamic, count) in [
            (0u32, 0u32, 0u32, 1u32, 2u32, 2u32),
            (1, 1, 0x8000_4005, 1, 2, 2),
            (2, 1, 0, 0, 2, 2),
            (3, 1, 0, 1, 2, 2),
            (4, 1, 0, 1, 1, 1),
        ] {
            let mut words = vec![0u32; 0x10];
            words[0x3c / 4] = 3;
            let ctx = t.com_object(&words, &[]);
            // Map (slot 14): logs its six words, writes the pointer
            // through the last, returns `hr`
            let target = t.junk(0x40, 9);
            let log = t.log;
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes());
            c.extend([0xc1, 0xe0, 0x05]);
            c.push(0x05);
            c.extend((log + 4).to_le_bytes());
            c.extend([0xc7, 0x00]);
            c.extend(0xc00eu32.to_le_bytes());
            for k in 0..6u8 {
                c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]);
                c.extend([0x89, 0x50, 8 + 4 * k]);
            }
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes());
            c.extend([0x8b, 0x54, 0x24, 0x18]); // mov edx, [esp+0x18]
            c.extend([0xc7, 0x02]);
            c.extend((if ptr == 0 { 0 } else { target }).to_le_bytes()); // mov dword [edx], ptr
            c.push(0xb8);
            c.extend(hr.to_le_bytes());
            c.extend([0xc2, 0x18, 0x00]);
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let vt = t.r32(ctx);
            t.w32(vt + 0x38, code);
            let holder = t.rebase(d3d::HOLDER_CONTEXT);
            t.put_saved(holder, ctx);
            t.stub(d3d::SCOPE_ENTER, true, 0, 0, 0x4d50);
            t.stub(d3d::SCOPE_LEAVE, true, 0, 0, 0x4e90);
            t.stub(d3d::HOLDER_CTOR, true, 0, 0, 0);
            t.stub(d3d::ATEXIT, false, 1, 0, 0);
            t.stub(d3d::MEMCPY, false, 3, 0, 0x3c);
            let guard = t.rebase(d3d::HOLDER_GUARD);
            let g = t.r32(guard);
            t.put_saved(guard, if case == 3 { g & !1 } else { g | 1 });
            let obj = t.junk(0x40, 3);
            let elems = t.junk(0x40, 4);
            let e = if count == 1 { obj + 0x10 } else { elems + 16 };
            t.w32(obj + 0xc, count);
            if count > 1 {
                t.w32(obj + 0x10, elems);
            }
            t.w32(e + 8, pending);
            t.w32(obj + 0x24, dynamic);
            t.stub(d3d::ELEMENT, true, 1, 1, e);
            let i = u32::from(count > 1);
            let ebp = t.ebp_for(1);
            t.check_frame(
                &format!("{label}: upload {case}"),
                0x0059_37a0,
                Conv::Thiscall(obj),
                &[i],
                ebp,
                0x20,
                (0x20, 0xc),
                |g| Out::U32(eb::upload(g, ebp, obj, i)),
            );
            t.reset();
        }
        // the draw set-up: no topology; each pixel shader; made
        for (topology, alt) in [(0u32, 0u32), (4, 0), (4, 1)] {
            t.stub(d3d::MAP_TOPOLOGY, false, 1, 0, topology);
            t.stub(d3d::PASS_STATE_BIND, true, 1, 1, 0);
            let mut words = vec![0u32; 0x46];
            for (off, n) in [
                (0x114, 4),
                (0x5c, 4),
                (0x44, 2),
                (0x2c, 4),
                (0x24, 4),
                (0x1c, 4),
                (0x40, 4),
                (0x20, 4),
                (0x28, 4),
                (0x48, 6),
                (0x60, 2),
                (0x34, 3),
            ] {
                words[off / 4] = n;
            }
            let ctx = t.com_object(&words, &[]);
            let r = t.junk(0x80, 5);
            t.w32(r + 0x70, ctx);
            let obj = t.junk(0x80, 6);
            t.w32(obj + 0x58, alt);
            let ebp = t.ebp_for(4);
            t.check_frame(
                &format!("{label}: draw setup {topology} {alt}"),
                0x0059_2e90,
                Conv::Thiscall(obj),
                &[r, 7, 2, 6],
                ebp,
                0x30,
                (0x30, 4),
                |g| Out::U32(eb::draw_setup(g, ebp, obj, r, 7, 2, 6)),
            );
            t.reset();
        }
    }
}

#[test]
fn renderer_pool_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the same count (owned, external); growing (owned / not owned old
        // buffer, failing, the old buffer handed back, no old buffer); the
        // same block count (with data, without, the word's bit 31 set); no
        // renderers; a pool that has an array
        // (case, word 0, buffer: 0 junk / 1 the malloc one / 2 none, +8, n, fails, +0xc)
        for (case, word, buffer, have, n, fails, arr) in [
            (0u32, 0u32, 0u32, 3u32, 3u32, false, 0u32),
            (1, 0x8000_0000, 0, 0, 2, false, 0),
            (2, 0, 0, 0x8000_0000, 2, false, 0),
            (3, 0x8000_0000, 0, 0, 2, true, 0),
            (4, 15, 1, 0, 1, false, 0),
            (5, 0x8000_0003, 0, 2, 0, false, 0),
            (6, 0, 0, 0, 2, false, 0x5555),
            (7, 0x8000_0002, 0, 0x8000_0002, 2, false, 0),
            (8, 0x8000_000f, 0, 0, 1, false, 0),
            (9, 15, 2, 0, 1, false, 0),
            (10, 0, 1, 0, 1, false, 0),
            (11, 0, 2, 0, 1, false, 0),
            (12, 0, 0, 5, 2, false, 0),
        ] {
            let buf = t.junk(0x400, 1);
            t.stub(d3d::PHYRE_MALLOC, false, 2, 0, if fails { 0 } else { buf });
            t.stub(d3d::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(d3d::BLOCKS_INIT, false, 2, 0, 0x60);
            t.stub(d3d::RENDERER_CTOR, true, 1, 1, 0x61);
            t.stub(d3d::FREE_RENDERERS, false, 2, 0, 0x62);
            let obj = t.zeros(0x10);
            t.w32(obj, word);
            let old = t.junk(0x10, 2);
            t.w32(obj + 4, [old, buf, 0][buffer as usize]);
            t.w32(obj + 8, have);
            t.w32(obj + 0xc, arr);
            t.check(
                &format!("{label}: pool {case}"),
                0x005b_bc30,
                Conv::Thiscall(obj),
                &[n, 0xdd],
                |g| Out::U32(d3d::pool::resize(g, obj, n, 0xdd)),
            );
            t.reset();
        }
    }
}

#[test]
fn command_push_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // rings as [base, limit, cursor, end]: both with room; the data
        // cursor past its end and the command ring overrunning its end; a
        // data wrap; a command wrap; a command cursor past its end; no
        // command; a first command already set; a command ring with no
        // room even after the wrap
        let room = [0x1000u32, 0x2000, 0x1100, 0x1800];
        for (case, data, cmds, ok, first) in [
            (0u32, room, room, true, 0u32),
            (
                1,
                [0x1000, 0x2000, 0x1900, 0x1800],
                [0x1000, 0x2000, 0x17f0, 0x1800],
                true,
                0,
            ),
            (2, [0x1000, 0x1100, 0x1200, 0x1800], room, true, 0),
            (3, room, [0x1000, 0x2000, 0x1fd0, 0x1ff0], true, 0),
            (4, room, [0x1000, 0x2000, 0x1900, 0x1800], true, 0),
            (5, room, room, false, 0),
            (6, room, room, true, 0x7777),
            (7, room, [0x1000, 0x1040, 0x1100, 0x1800], true, 0),
        ] {
            let vt_owner = t.object(&[0, 0, 0], &[]);
            let vt = t.r32(vt_owner);
            let r = t.junk(0x60, 3);
            t.w32(r, vt);
            for k in 0..4 {
                t.w32(r + 0x1c + 4 * k, data[k as usize]);
                t.w32(r + 0x30 + 4 * k, cmds[k as usize]);
            }
            t.w32(r + 0x4c, first);
            let cmd = t.junk(0x48, 4);
            t.stub(d3d::RING_ALLOCATE, true, 1, 1, if ok { cmd } else { 0 });
            let ebp = t.ebp_for(1);
            t.check_frame(
                &format!("{label}: push {case}"),
                0x005b_b2e0,
                Conv::Thiscall(r),
                &[0x4242],
                ebp,
                0x20,
                (8, 0),
                |g| Out::U32(d3d::queue::push(g, ebp, r, 0x4242)),
            );
            t.reset();
        }
    }
}

#[test]
fn render_thread_stop_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // stopped; idle; running at slot 3; running at the last slot; another state
        for (case, state, slot) in [
            (0u32, 2u32, 3u32),
            (1, 0, 3),
            (2, 1, 3),
            (3, 1, 15),
            (4, 3, 3),
        ] {
            for (sym, n) in [
                (d3d::THREAD_SYNC, 0),
                (d3d::SEMAPHORE_WAIT, 0),
                (d3d::EVENT_SIGNAL, 0),
                (d3d::THREAD_JOIN, 1),
                (d3d::THREAD_NOP, 0),
                (d3d::EVENT_CLOSE, 0),
                (d3d::SEMAPHORE_CLOSE, 0),
            ] {
                t.stub(sym, true, n, n, 0x50 + n);
            }
            let th = t.junk(0x170, 5);
            t.w32(th + 0xc, state);
            t.w32(th + 0x154, slot);
            let ebp = t.ebp_for(0);
            t.check_frame(
                &format!("{label}: stop {case}"),
                0x0061_5f40,
                Conv::Thiscall(th),
                &[],
                ebp,
                0x20,
                (4, 0),
                |g| {
                    d3d::thread::stop(g, ebp, th);
                    Out::None
                },
            );
            t.reset();
        }
    }
}

#[test]
fn renderer_job_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // no lists; two lists (the count's bit 31 set); can kick or not;
        // with and without the +0xc holder
        for (case, count, kick, holder) in [
            (0u32, 0u32, 0x100u32, true),
            (1, 2, 0x101, true),
            (2, 0x8000_0002, 0x100, false),
            (3, 1, 0x1, false),
        ] {
            t.stub(d3d::INIT_RINGS, true, 5, 5, 0x41);
            t.stub(d3d::RENDER_LIST, true, 3, 3, 0x42);
            t.stub(d3d::CAN_KICK, true, 0, 0, kick);
            let vt_owner = t.object(&[0, 0, 0], &[]);
            let vt = t.r32(vt_owner);
            let arr = t.junk(0xe4 * 3, 7);
            for k in 0..3 {
                t.w32(arr + 0xe4 * k, vt);
            }
            let job = t.junk(0x40, 8);
            let a = t.junk(8, 9);
            let b = t.junk(8, 10);
            t.w32(job + 0xc, if holder { a } else { 0 });
            t.w32(job + 0x10, arr);
            t.w32(job + 0x14, b);
            t.w32(job + 0x2c, count);
            t.w32(job + 0x30, 0x5000);
            t.w32(job + 0x1c, 0x8000_1230);
            t.w32(job + 0x24, 0x8000_0480);
            let ebp = t.ebp_for(3);
            t.check_frame(
                &format!("{label}: job {case}"),
                0x005b_6820,
                Conv::Thiscall(job),
                &[0xa, 0xb, 1],
                ebp,
                0x20,
                (0x14, 0),
                |g| Out::U32(d3d::job::run(g, ebp, ebp + 0x10, job, 1)),
            );
            let mut x = [0; 4];
            let mut y = [0; 4];
            t.a.read_bytes(ebp + 0x10, &mut x);
            t.b.read_bytes(ebp + 0x10, &mut y);
            assert_eq!(x, y, "{label}: job {case}: the index slot");
            t.reset();
        }
    }
}

#[test]
fn shader_texture_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // no cluster; no texture; an empty cluster list; two shaders with
        // the parameter (count bit 31 set); shaders without it; no shaders
        for (case, cluster, found, empty, count, param) in [
            (0u32, false, true, false, 2u32, 0x77u32),
            (1, true, false, false, 2, 0x77),
            (2, true, false, true, 2, 0x77),
            (5, true, true, false, 0x8000_0002, 0x77),
            (3, true, true, false, 2, 0),
            (4, true, true, false, 0, 0x77),
        ] {
            let c = t.junk(0x40, 11);
            if empty {
                t.w32(c + 0x1c, c + 0x1c);
            }
            t.stub(d3d::CLUSTERS, false, 0, 0, 0x6600);
            t.stub(
                d3d::CLUSTER_BY_NAME,
                true,
                1,
                1,
                if cluster { c } else { 0 },
            );
            // the iterator: result word 3 (found), word 4 and word 0 (the
            // texture's parts), then ret
            let mut code = vec![0xc7, 0x41, 0x0c];
            code.extend(u32::from(found).to_le_bytes());
            code.extend([0xc7, 0x41, 0x10]);
            code.extend(0x3000u32.to_le_bytes());
            code.extend([0xc7, 0x01]);
            code.extend(0x0123u32.to_le_bytes());
            code.push(0xc3);
            let at_code = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), at_code);
            let at = t.rebase(d3d::ITER_SEEK);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(at_code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            let data = t.zeros(8);
            for k in 0..2 {
                let e = t.zeros(8);
                t.w32(e, 0x100 + k);
                t.w32(e + 4, 0x200 + k);
                t.w32(data + 4 * k, e);
            }
            t.stub(d3d::ARRAY_DATA, true, 0, 0, data);
            t.stub(d3d::FIND_PARAM, true, 1, 1, param);
            t.stub(d3d::SET_TEXTURE, true, 2, 2, 0x45);
            let arr = t.zeros(8);
            t.w32(arr, count);
            let mgr = t.junk(0x20, 12);
            t.w32(mgr + 0x10, arr);
            let ebp = t.ebp_for(3);
            t.check_frame(
                &format!("{label}: texture {case}"),
                0x006d_f950,
                Conv::Cdecl,
                &[mgr, 0x5a5a, 0x6b6b],
                ebp,
                0x40,
                (0x28, 0),
                |g| {
                    d3d::texture::set_texture(g, ebp, mgr, 0x5a5a, 0x6b6b);
                    Out::None
                },
            );
            t.reset();
        }
    }
}

// ---- round left2 (ffx_prender::m5p_render_b_left2) ----

/// `PDeferredLightingNVN::initializeShaders`: no buffers (either), the
/// common shaders failing, and the table: the lookup stub answers each
/// name with a value of its own, so every field is checked against its
/// own name.
#[test]
fn left2_nvn_shaders_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        // the lookup: logs this and the name, returns name ^ 0x5a5a_0000
        let at = t.rebase(l2::FIND_PARAM);
        let log_stub = t.stub_code(l2::FIND_PARAM, true, 1, 0, 0);
        let code_at = t.a.alloc_code(&[0xcc; 32]).unwrap();
        assert_eq!(t.b.alloc_code(&[0xcc; 32]).unwrap(), code_at);
        // call log_stub (it logs [esp+4] as the word after the return
        // address it sees, so push the name again first)
        let mut code = vec![0xff, 0x74, 0x24, 0x04]; // push dword [esp+4]
        code.push(0xe8);
        code.extend(log_stub.wrapping_sub(code_at + 9).to_le_bytes()); // call log_stub
        code.extend([0x83, 0xc4, 0x04]); // add esp, 4
        code.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
        code.push(0x35);
        code.extend(0x5a5a_0000u32.to_le_bytes()); // xor eax, imm
        code.extend([0xc2, 0x04, 0x00]); // ret 4
        t.write(code_at, &code);
        let mut j = vec![0xe9];
        j.extend(code_at.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut t.a, at, 5).unwrap();
        t.saved.entry(at).or_insert(old);
        t.write(at, &j);
        for (b0, b1, common) in [(0u32, 1u32, 0u32), (1, 0, 0), (1, 1, 0x0b), (1, 1, 0)] {
            t.stub(l2::INIT_COMMON, true, 2, 2, common);
            let this = t.junk(0x4100, 3);
            t.w32(this + 0x3ee0, b0);
            t.w32(this + 0x3ee4, b1);
            let what = format!("{label}: NVN shaders {b0} {b1} common {common:#x}");
            t.check(
                &what,
                0x00a1_a4f0,
                Conv::Thiscall(this),
                &[0x0012_0000, 0x0034_0000],
                |g| Out::U32(l2b::init_nvn_shaders(g, this, 0x0012_0000, 0x0034_0000)),
            );
        }
        t.reset();
    }
}

/// The two-sampler `initializeShaders` copies against their own
/// originals: no guard, the previous buffer copied or not, the instance
/// array's shapes, a sampler registration failing, the samplers fitting
/// or not; the records compared in the original's frame when both are
/// written.
#[test]
#[allow(clippy::type_complexity)] // one row per case, columns named above
fn left2_sampler_shaders_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        for c in l2::SAMPLER_SHADERS {
            // (guard, prev sizes equal, array head, registrations, p1, p2,
            //  p1 offset/size, p2 offset/size)
            let cases: [(
                bool,
                Option<bool>,
                u32,
                [u32; 2],
                bool,
                bool,
                [u16; 2],
                [u16; 2],
            ); 12] = [
                (false, None, 1, [0, 0], true, true, [8, 16], [0x20, 16]),
                (true, None, 1, [0, 0], true, true, [8, 16], [0x20, 16]),
                (true, Some(true), 0, [0, 0], true, true, [8, 16], [0x20, 16]),
                (
                    true,
                    Some(false),
                    2,
                    [0, 0],
                    true,
                    true,
                    [8, 0xe010],
                    [0x20, 32],
                ),
                (
                    true,
                    None,
                    0x8000_0003,
                    [0, 0],
                    true,
                    true,
                    [8, 16],
                    [0x20, 16],
                ),
                (
                    true,
                    None,
                    0x8000_0001,
                    [5, 0],
                    true,
                    true,
                    [8, 16],
                    [0x20, 16],
                ),
                (true, None, 1, [0, 6], true, true, [8, 16], [0x20, 16]),
                (true, None, 1, [0, 0], false, true, [8, 16], [0x20, 16]),
                (true, None, 1, [0, 0], true, false, [2, 16], [0x20, 16]),
                (true, None, 1, [0, 0], true, true, [8, 16], [0x78, 16]),
                (true, None, 1, [0, 0], false, false, [8, 16], [0x20, 16]),
                (true, None, 1, [0, 0], true, true, [8, 16], [0x70, 16]),
            ];
            for (k, &(guard, prev, head, regs, has1, has2, o1, o2)) in cases.iter().enumerate() {
                let this = t.junk(c.buffer as usize + 0x40, 3);
                let guard_obj = t.zeros(0x20);
                t.w32(this + c.base, if guard { guard_obj } else { 0 });
                if c.guard_call != 0 {
                    // the guard's getter (cdecl, the cluster)
                    t.w32(this + c.base, 0x0bad_0bad);
                    t.stub(c.guard_call, false, 1, 0, if guard { guard_obj } else { 0 });
                }
                // the constant buffer: {.., +4 data}; data [0] its size
                let data = t.junk(0x80, 4);
                t.w32(data, 0x80);
                t.w32(data + 8, 0x8000_0010);
                let buf = t.zeros(0x10);
                t.w32(buf + 4, data);
                t.stub(l2::SAMPLER_CALLS[0], true, 3, 3, buf);
                if let Some(same) = prev {
                    let src = t.junk(0x20, 5);
                    t.w32(src, 0x10);
                    t.w32(src + 8, if same { 0x10 } else { 0x11 });
                    let p = t.zeros(8);
                    t.w32(p + 4, src);
                    t.w32(this + 8, p);
                } else {
                    t.w32(this + 8, 0);
                }
                let arr = t.junk(0x10, 6);
                t.w32(arr, head);
                t.stub(l2::SAMPLER_CALLS[2], true, 3, 3, arr);
                t.stub(l2::SAMPLER_CALLS[3], false, 1, 0, 0);
                let reg = seq_code(&mut t, regs);
                place(&mut t, l2::SAMPLER_CALLS[4], reg);
                // the parameter lookup: the two sampler names answer their
                // records, the others name ^ 0x5a5a0000
                let n1 = t.rebase(l2::SAMPLER_DATA[1]);
                let n2 = t.rebase(l2::SAMPLER_DATA[2]);
                let p1 = t.zeros(0x10);
                t.write(p1 + 8, &o1[0].to_le_bytes());
                t.write(p1 + 0xa, &o1[1].to_le_bytes());
                let p2 = t.zeros(0x10);
                t.write(p2 + 8, &o2[0].to_le_bytes());
                t.write(p2 + 0xa, &o2[1].to_le_bytes());
                let r1 = if has1 { p1 } else { 0 };
                let r2 = if has2 { p2 } else { 0 };
                // the vector parameter (the third copy): fits, none, does
                // not fit
                let p3 = t.zeros(0x10);
                let (o3, sz3) = if k % 3 == 2 {
                    (0x7cu16, 16u16)
                } else {
                    (0x40, 16)
                };
                t.write(p3 + 8, &o3.to_le_bytes());
                t.write(p3 + 0xa, &sz3.to_le_bytes());
                let r3 = if k % 3 == 1 { 0 } else { p3 };
                let n3 = t.rebase(0x00b4_25e4);
                let mut code = vec![0x8b, 0x44, 0x24, 0x04, 0x3d];
                code.extend(n1.to_le_bytes()); // cmp eax, n1
                code.extend([0x75, 0x08, 0xb8]); // jne +8; mov eax, r1
                code.extend(r1.to_le_bytes());
                code.extend([0xc2, 0x04, 0x00, 0x3d]); // ret 4; cmp eax, n2
                code.extend(n2.to_le_bytes());
                code.extend([0x75, 0x08, 0xb8]); // jne +8; mov eax, r2
                code.extend(r2.to_le_bytes());
                code.extend([0xc2, 0x04, 0x00, 0x3d]); // ret 4; cmp eax, n3
                code.extend(n3.to_le_bytes());
                code.extend([0x75, 0x08, 0xb8]); // jne +8; mov eax, r3
                code.extend(r3.to_le_bytes());
                code.extend([0xc2, 0x04, 0x00, 0x35]); // ret 4; xor eax, imm
                code.extend(0x5a5a_0000u32.to_le_bytes());
                code.extend([0xc2, 0x04, 0x00]); // ret 4
                let find = t.a.alloc_code(&code).unwrap();
                assert_eq!(t.b.alloc_code(&code).unwrap(), find);
                place(&mut t, l2::FIND_PARAM, find);
                let (cluster, effect) = (0x0012_0000, 0x0034_0000);
                let what = format!("{label}: sampler shaders {:#x} case {k}", c.at);
                // the call pushes 2 words, the return address and ebp:
                // the frame is 0x20 bytes under that
                let addr = t.rebase(c.at);
                let ra =
                    t.a.call(addr, Conv::Thiscall(this), &[cluster, effect])
                        .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
                let top = t.b.stack_range().end - 0x100;
                t.b.set_call_top(Some(top - 0x400));
                // the records 0x20 (0x30 with the vector) under ebp
                let frame = top - if c.vec_param != 0 { 0x40 } else { 0x30 };
                let out =
                    l2b::init_sampler_shaders(&mut G(&mut t.b), &c, this, cluster, effect, frame);
                t.b.set_call_top(None);
                assert_eq!(ra.eax, out, "{what}: eax");
                if guard && regs == [0, 0] && has1 && has2 && o1[0] >= 4 && o2[0] < 0x70 {
                    let n = if c.vec_param != 0 && r3 != 0 {
                        0x30
                    } else {
                        0x20
                    };
                    let fa = Oracle::read(&mut t.a, frame, n).unwrap();
                    let fb = Oracle::read(&mut t.b, frame, n).unwrap();
                    assert_eq!(fa, fb, "{what}: the records");
                }
                t.diff(&what);
                t.reset();
            }
        }
    }
}

/// Code answering `values` in turn (then the last one again), no
/// arguments popped; nothing logged.
fn seq_code(t: &mut Twins, values: [u32; 2]) -> u32 {
    let tab = t.alloc(
        &[values[0], values[1], values[1]]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let cell = t.zeros(4);
    let mut c = vec![0xa1];
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.extend([0x83, 0xf8, 0x02, 0x74, 0x06]); // cmp eax, 2; je +6
    c.extend([0xff, 0x05]);
    c.extend(cell.to_le_bytes()); // inc dword [cell]
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tab.to_le_bytes()); // mov eax, [eax*4 + tab]
    c.push(0xc3);
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

/// A `jmp` at `sym` to `code`.
fn place(t: &mut Twins, sym: u32, code: u32) {
    let at = t.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    let old = Oracle::read(&mut t.a, at, 5).unwrap();
    t.saved.entry(at).or_insert(old);
    t.write(at, &j);
}

/// The light setup's cluster load `0x006e5be0`: no cluster; an empty
/// object list; a list whose first node is of the cursor's class (found,
/// the global set) or of another class (not found). The original's
/// never-written name word `[ebp-4]` is 0 in its frame (the junk-read
/// policy), the cursor compared in the original's frame.
#[test]
fn left2_light_cluster_matches_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let class = t.rebase(l2::LIGHT_CLUSTER[6]);
        let other = t.zeros(0x90);
        for case in 0..5u32 {
            let cluster = if case == 0 { 0 } else { t.junk(0x80, 5) };
            // the fixup moves the cluster: the port must read it again
            let moved = if case == 0 { 0 } else { t.junk(0x80, 6) };
            if cluster != 0 {
                let list = moved + 0x1c;
                if case == 1 {
                    // empty: the list head itself would pass as a node
                    t.w32(list, list);
                    t.w32(list + 0x2c, class);
                    t.w32(list + 0x40, 2);
                } else {
                    let node = t.junk(0x50, 9);
                    t.w32(list, node);
                    t.w32(node, list);
                    t.w32(node + 0x2c, if case == 3 { other } else { class });
                    t.w32(node + 0x40, if case == 4 { 0 } else { 2 });
                    // +8 differs from +0x34: SetNode does not step
                    t.w32(node + 0x08, 0x1234);
                    t.w32(node + 0x34, 0x5678);
                    t.w32(node + 0x3c, 3);
                }
            }
            t.stub(l2::LIGHT_CLUSTER[0], false, 0, 0, 0x00e0_0000);
            t.stub(l2::LIGHT_CLUSTER[1], true, 1, 1, cluster);
            t.stub(l2::LIGHT_CLUSTER[2], false, 0, 0, 0x00e1_0000);
            t.stub(l2::LIGHT_CLUSTER[3], false, 2, 0, 0);
            // the fixup stub stores `moved` through its first word first
            let fix = t.rebase(l2::LIGHT_CLUSTER[3]);
            let logger = t.stub_code(l2::LIGHT_CLUSTER[3], false, 2, 0, 0);
            let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov dword [eax], moved
            c.extend(moved.to_le_bytes());
            let code = t.a.alloc_code(&[0xcc; 16]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 16]).unwrap(), code);
            c.push(0xe9);
            c.extend(logger.wrapping_sub(code + 15).to_le_bytes()); // jmp logger
            t.write(code, &c);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(fix + 5).to_le_bytes());
            t.write(fix, &j);
            let ebp = t.ebp_for(0);
            t.w32(ebp - 4, 0);
            let what = format!("{label}: light cluster case {case}");
            t.check_frame(
                &what,
                0x006e_5be0,
                Conv::Cdecl,
                &[],
                ebp,
                0x40,
                (0x24, 4),
                |g| {
                    l2b::load_light_cluster(g, ebp - 0x24);
                    Out::None
                },
            );
            t.reset();
        }
    }
}

/// `PDeferredLightingBase::initializeCommonShaders` against the
/// original: a target missing; the old material there or not; the
/// previous block copied (sizes match) or not, or no previous; the
/// instance array's heads (already one, empty, one inline, owned and
/// borrowed data); each sampler registration failing; each sampler
/// parameter missing; a binding failing.
#[test]
#[allow(clippy::type_complexity)] // one row per case, columns named above
fn big_common_shaders_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (the missing target, old material, previous: 0 none 1 same size
    // 2 other, the array head, registration errors, the sampler name the
    // lookup answers 0 for (0: none, 1: first, 2: second), bind error)
    let cases: [(u32, bool, u32, u32, [u32; 3], u32, u32); 12] = [
        (0, true, 1, 1, [0; 3], 0, 0),
        (1, true, 1, 1, [0; 3], 0, 0),
        (2, true, 1, 1, [0; 3], 0, 0),
        (9, true, 1, 1, [0; 3], 0, 0),
        (9, false, 2, 0, [0; 3], 1, 0),
        (9, true, 0, 0x8000_0001, [0; 3], 2, 0),
        (9, false, 1, 2, [0; 3], 0, 3),
        (9, true, 2, 0x8000_0002, [0; 3], 0, 0),
        (9, true, 1, 3, [7, 0, 0], 0, 0),
        (9, true, 1, 1, [0, 8, 0], 0, 0),
        (9, true, 1, 1, [0, 0, 9], 0, 0),
        (9, true, 1, 1, [0; 3], 2, 3),
    ];
    for (missing, old, prev, head, regs, absent, bind) in cases {
        for (label, mut t) in Twins::all() {
            let c = l2::SAMPLER_CALLS;
            let k = l2::COMMON_CALLS;
            // the lookup: logs this and the name, answers name ^ key
            let key = match absent {
                1 => t.rebase(l2::SAMPLER_DATA[1]),
                2 => t.rebase(k[3]),
                _ => 0x5a5a_0000,
            };
            let at = t.rebase(l2::FIND_PARAM);
            let log_stub = t.stub_code(l2::FIND_PARAM, true, 1, 0, 0);
            let code_at = t.a.alloc_code(&[0xcc; 32]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 32]).unwrap(), code_at);
            let mut code = vec![0xff, 0x74, 0x24, 0x04]; // push dword [esp+4]
            code.push(0xe8);
            code.extend(log_stub.wrapping_sub(code_at + 9).to_le_bytes()); // call log_stub
            code.extend([0x83, 0xc4, 0x04]); // add esp, 4
            code.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
            code.push(0x35);
            code.extend(key.to_le_bytes()); // xor eax, key
            code.extend([0xc2, 0x04, 0x00]); // ret 4
            t.write(code_at, &code);
            let mut j = vec![0xe9];
            j.extend(code_at.wrapping_sub(at + 5).to_le_bytes());
            let saved = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(saved);
            t.write(at, &j);
            // the registration: logs this, answers [this + 4]
            let at = t.rebase(c[4]);
            let log_reg = t.stub_code(c[4], true, 0, 0, 0);
            let code_at = t.a.alloc_code(&[0xcc; 16]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 16]).unwrap(), code_at);
            let mut code = vec![0xe8];
            code.extend(log_reg.wrapping_sub(code_at + 5).to_le_bytes()); // call log_reg
            code.extend([0x8b, 0x41, 0x04, 0xc3]); // mov eax, [ecx+4]; ret
            t.write(code_at, &code);
            let mut j = vec![0xe9];
            j.extend(code_at.wrapping_sub(at + 5).to_le_bytes());
            let saved = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(saved);
            t.write(at, &j);

            let this = t.zeros(0x3fa0);
            for (i, off) in [0x3edc, 0x3ee0, 0x3ee4].into_iter().enumerate() {
                if i as u32 != missing {
                    let target = t.zeros(0x20);
                    t.w32(this + off, target);
                }
            }
            for (i, off) in [0x3ee8u32, 0x3f0c, 0x3f30].into_iter().enumerate() {
                t.w32(this + off + 4, regs[i]);
            }
            if old {
                t.w32(this + 0x3ed8, 0x0051_0000);
            }
            let dst = t.junk(0x20, 1);
            t.w32(dst + 8, 0x10);
            let material = t.zeros(8);
            t.w32(material + 4, dst);
            if prev != 0 {
                let src = t.junk(0x20, 2);
                t.w32(src, 0x18);
                t.w32(src + 8, if prev == 1 { 0x8000_0010 } else { 0x11 });
                let p = t.zeros(8);
                t.w32(p + 4, src);
                t.w32(this + 8, p);
            }
            let arr = t.zeros(0x10);
            t.w32(arr, head);
            let data = t.zeros(8);
            t.w32(arr + 4, data);
            t.stub(k[0], true, 1, 1, 0);
            t.stub(c[0], true, 3, 3, material);
            t.stub(c[1], false, 3, 0, 0);
            t.stub(c[2], true, 3, 3, arr);
            t.stub(c[3], false, 1, 0, 0);
            t.stub(k[1], true, 2, 2, 0);
            t.stub(k[2], true, 2, 2, bind);
            let what = format!(
                "{label}: common shaders missing {missing} old {old} prev {prev} head {head:#x} regs {regs:?} absent {absent} bind {bind}"
            );
            t.check(
                &what,
                0x00a1_82f0,
                Conv::Thiscall(this),
                &[0x0012_0000, 0x0034_0000],
                |g| Out::U32(l2b::init_common_shaders(g, this, 0x0012_0000, 0x0034_0000)),
            );
            t.reset();
        }
    }
}

/// `0x005b4c00` (a 4x4 matrix applied to a column) against the original:
/// ordinary values and the special floats (quiet NaNs with payloads,
/// infinities, denormals) anywhere in the matrix and the vector.
#[test]
fn big_transform_column_matches_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    const SPECIAL: [u32; 8] = [
        0x0000_0000,
        0x8000_0000,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc1_2345,
        0x0000_0001,
        0x7f7f_ffff,
    ];
    for (label, mut t) in Twins::all() {
        let mut seed = 0x1357_9bdfu32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for case in 0..49 {
            let mut words = [0u32; 20];
            for w in &mut words {
                let r = next();
                *w = if case >= 8 && r.is_multiple_of(9) {
                    SPECIAL[(r >> 4) as usize % SPECIAL.len()]
                } else {
                    (0x3a00_0000 + (r >> 8) % 0x0a00_0000) | (r & 0x8000_0000)
                };
            }
            if case == 48 {
                // every product -0: the sums keep the sign
                for w in &mut words[..16] {
                    *w |= 0x8000_0000;
                }
                words[16..20].fill(0);
            }
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let m = t.alloc(&bytes[..64]);
            let v = t.alloc(&bytes[64..]);
            let out = t.junk(0x10, 7);
            let what = format!("{label}: transform column case {case}");
            t.check(&what, 0x005b_4c00, Conv::Thiscall(m), &[out, v], |g| {
                Out::U32(l2b::transform_column(g, m, out, v))
            });
        }
        t.reset();
    }
}

/// `0x005930e0` (the shader resource tables bound) against the original:
/// a constant buffer or none, a slot index or -1, feature levels below
/// and at 0xb000. The context is a COM object of recording stubs; the
/// zeroed table is compared in the original's frame.
#[test]
fn big_resource_tables_match_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    for (buffer, index, level) in [
        (true, 2u32, 0xb000u32),
        (false, u32::MAX, 0xa100),
        (true, u32::MAX, 0xafff),
        (false, 0, 0xc000),
    ] {
        for (label, mut t) in Twins::all() {
            let mut words = vec![1u32; 72];
            words[0xc0 / 4] = 7;
            for slot in [
                0x20, 0x64, 0x7c, 0x10c, 0xec, 0xfc, 0x84, 0x114, 0x11c, 0x118,
            ] {
                words[slot / 4] = 4;
            }
            words[0x110 / 4] = 5;
            let ctx = t.com_object(&words, &[]);
            let device = t.zeros(0x310);
            t.w32(device + 0x70, ctx);
            t.w32(device + 0xb4, level);
            t.w32(device + 0x300, 0x55);
            let renderer = t.zeros(8);
            t.w32(renderer, device);
            let this = t.junk(0x500, 4);
            t.w32(this, if buffer { 0x0066_0000 } else { 0 });
            t.w32(this + 4, 0x8000_0040);
            t.w32(this + 0x18, index);
            let ebp = t.ebp_for(2);
            let what = format!(
                "{label}: resource tables buffer {buffer} index {index:#x} level {level:#x}"
            );
            t.check_frame(
                &what,
                0x0059_30e0,
                Conv::Thiscall(this),
                &[renderer, 0x77],
                ebp,
                0x220,
                (0x204, 4),
                |g| Out::U8(l2b::bind_resource_tables(g, this, renderer, ebp - 0x204)),
            );
            t.reset();
        }
    }
}

/// `0x0059a390` (a depth-remapped transposed matrix and a color uploaded)
/// against the original: the row products and the transpose run as the
/// game's code; the 0x50-byte block and the rows are compared in the
/// original's frame (below its saved `ecx`).
#[test]
fn big_depth_transform_matches_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    for (label, mut t) in Twins::all() {
        let mut seed = 0x2468_ace1u32;
        for case in 0..6u32 {
            let mut words = [0u32; 20];
            for w in &mut words {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                *w = (0x3c00_0000 + (seed >> 8) % 0x0600_0000) | (seed & 0x8000_0000);
            }
            if case == 5 {
                words[3] = 0x7fc0_1234;
                words[17] = 0xff80_0000;
            }
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let m = t.alloc(&bytes[..64]);
            let color = t.alloc(&bytes[64..]);
            let mut slots = vec![1u32; 49];
            slots[0xc0 / 4] = 7;
            let ctx = t.com_object(&slots, &[]);
            let renderer = t.zeros(0x80);
            t.w32(renderer + 0x70, ctx);
            let this = t.zeros(0x50);
            t.w32(this + 0x48, 0x0077_0000 + case);
            let ebp = t.ebp_for(3);
            let what = format!("{label}: depth transform case {case}");
            t.check_frame(
                &what,
                0x0059_a390,
                Conv::Thiscall(this),
                &[renderer, m, color],
                ebp,
                0x130,
                (0x114, 0x44),
                |g| {
                    Out::U32(l2b::update_depth_transform(
                        g,
                        this,
                        renderer,
                        m,
                        color,
                        ebp - 0x114,
                    ))
                },
            );
            t.reset();
        }
    }
}

/// `0x00a13400` (the deferred lighting's texture parameters) against the
/// original: the copies and `PParameterBuffer_writeParameter` run as the
/// game's code into a real buffer; each parameter missing, of a wrong
/// type, not fitting or under offset 4; the shadow texture without its
/// flag or absent; the tile setup failing.
#[test]
fn big_texture_params_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (which parameter is changed: 0..3 = +0x4024, +0x403c, +0x4014,
    // +0x4010, 9 none; how: 0 missing, 1 wrong type, 2 too far, 3 offset
    // under 4; the shadow: 0 none, 1 flagged, 2 unflagged; tile error)
    let cases: [(u32, u32, u32, u32); 16] = [
        (9, 0, 1, 0),
        (9, 0, 0, 0),
        (9, 0, 2, 0),
        (9, 0, 1, 7),
        (0, 0, 1, 0),
        (0, 1, 1, 0),
        (0, 2, 1, 0),
        (1, 1, 1, 0),
        (1, 3, 1, 0),
        (2, 0, 1, 0),
        (2, 1, 1, 0),
        (2, 2, 1, 0),
        (3, 0, 1, 0),
        (3, 1, 1, 0),
        (3, 2, 1, 0),
        (2, 4, 1, 0),
    ];
    for (which, how, shadow_kind, tiles) in cases {
        for (label, mut t) in Twins::all() {
            let this = t.zeros(0x4040);
            let buffer = t.junk(0x100, 3);
            t.w32(buffer, 0xc0);
            let material = t.zeros(8);
            t.w32(material, 0x0044_0000);
            t.w32(material + 4, buffer);
            t.w32(this + 0x3ed8, material);
            for (i, (field, ty, off)) in [
                (0x4024u32, b'H', 0x10u16),
                (0x403c, b'H', 0x24),
                (0x4014, b'I', 0x38),
                (0x4010, b'H', 0x4c),
            ]
            .into_iter()
            .enumerate()
            {
                let i = i as u32;
                if i == which && how == 0 {
                    continue;
                }
                let p = t.zeros(0x10);
                let ty = match (i == which, how) {
                    (true, 1) => b'J',
                    (true, 4) => 0x15,
                    _ => ty,
                };
                let off = match (i == which, how) {
                    (true, 2) => 0xb8,
                    (true, 3) => 2,
                    _ => off,
                };
                t.w8(p + 2, ty);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &(0xe010u16).to_le_bytes());
                t.w32(this + field, p);
            }
            let shadow = match shadow_kind {
                0 => 0,
                k => {
                    let s = t.zeros(0x10);
                    t.w8(s + 8, if k == 1 { 0x0c } else { 0x04 });
                    s
                }
            };
            t.w32(this + 0x3fd0, shadow);
            t.w32(this + 0x4034, 0x0101_0101);
            t.w32(this + 0x4038, 0x0202_0202);
            let object = t.zeros(0x24);
            t.w32(object + 0x1c, 0x41);
            t.w32(object + 0x20, 0x38);
            let target = t.zeros(8);
            t.w32(target, object);
            t.w32(this + 0x3fc4, target);
            let renderer = t.zeros(0x50);
            t.stub(l2::TEXTURE_PARAMS[1], true, 5, 5, tiles);
            let what = format!(
                "{label}: texture params which {which} how {how} shadow {shadow_kind} tiles {tiles}"
            );
            t.check(
                &what,
                0x00a1_3400,
                Conv::Thiscall(this),
                &[renderer, 0x99],
                |g| {
                    let frame = ffx_graphic::host::Host::scratch(g);
                    Out::U32(l2b::set_texture_params(g, this, renderer, frame))
                },
            );
            t.reset();
        }
    }
}

/// `0x00a965d0` (the profiles raised into a height line; SSE) against
/// the original: runs of equal keys (their largest value, a quiet NaN
/// among them), floors beaten or not, rows clamped at both ends, levels
/// clamped at 0 and 7, fractional profile bounds, positions under 1 and
/// past the limit.
#[test]
#[allow(clippy::cast_precision_loss)] // small test values
fn big_raise_profiles_match_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    for (label, mut t) in Twins::all() {
        let mut seed = 0x0f1e_2d3cu32;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for case in 0..4u32 {
            let keys: [i32; 12] = [
                0x40, 0x40, 0x55, 0x80, 0x80, 0x80, -0x10, 0x900, 0x31, 0x33, 0x33, 0x7f,
            ];
            let n = keys.len() as u32;
            let fbytes = |v: &[f32]| {
                v.iter()
                    .flat_map(|x| x.to_bits().to_le_bytes())
                    .collect::<Vec<u8>>()
            };
            let mut values: Vec<f32> = (0..n).map(|_| (next() % 1000) as f32 * 0.1 - 5.0).collect();
            if case == 1 {
                values[4] = f32::from_bits(0x7fc0_0042);
            }
            let floors: Vec<f32> = (0..n)
                .map(|k| {
                    if (k + case) % 3 == 0 {
                        500.0
                    } else {
                        (next() % 600) as f32 * 0.1 - 10.0
                    }
                })
                .collect();
            let kbytes: Vec<u8> = keys.iter().flat_map(|k| k.to_le_bytes()).collect();
            let keys_at = t.alloc(&kbytes);
            let values_at = t.alloc(&fbytes(&values));
            let floors_at = t.alloc(&fbytes(&floors));
            let out = t.alloc(&fbytes(&vec![1.0f32; 0x110]));
            // three row tables of eight profiles {first, end, heights}
            let mut rows = Vec::new();
            for r in 0..3u32 {
                let mut levels = Vec::new();
                for l in 0..8u32 {
                    let first = [0.0f32, 1.5, 3.0, 2.25][((r + l + case) % 4) as usize];
                    let end = first + 4.0 + (l % 3) as f32;
                    let mut p = vec![first, end];
                    p.extend((0..12).map(|_| (next() % 400) as f32 * 0.25 - 20.0));
                    levels.push(t.alloc(&fbytes(&p)));
                }
                rows.push(
                    t.alloc(
                        &levels
                            .iter()
                            .flat_map(|v| v.to_le_bytes())
                            .collect::<Vec<_>>(),
                    ),
                );
            }
            let table: Vec<u32> = (0..17).map(|k| rows[k % 3]).collect();
            let tables = t.alloc(
                &table
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            let owner = t.zeros(0x200);
            t.w32(owner + 0x1f0, (50.0f32 + case as f32 * 7.0).to_bits());
            let this = t.zeros(0x30);
            t.w32(this, n);
            t.w32(this + 4, owner);
            t.w32(this + 0x14, keys_at);
            t.w32(this + 0x1c, 0x30);
            t.w32(this + 0x20, 4);
            t.w32(this + 0x24, 3 + case);
            t.w32(this + 0x28, [0x100, 0x40, 0x100, 0x90][case as usize]);
            let bias = (10.0f32 + case as f32 * 15.0).to_bits();
            let what = format!("{label}: raise profiles case {case}");
            t.check(
                &what,
                0x00a9_65d0,
                Conv::FastcallCallerPops,
                &[this, tables, values_at, floors_at, out, bias],
                |g| {
                    l2b::raise_profiles(g, this, tables, values_at, floors_at, out, bias);
                    Out::None
                },
            );
        }
        t.reset();
    }
}

/// `0x00a965d0`'s edges, built by hand: a value exactly 6 under its
/// floor (not raised), a row under 0 and one at 16 (clamped), level 1
/// (`trunc(v * 0.1)` with the offset cancelling the 30), a store of -0
/// over +0 (not greater), a position landing exactly on the limit.
#[test]
fn big_raise_profiles_edges_match_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    for (label, mut t) in Twins::all() {
        let fbytes = |v: &[f32]| {
            v.iter()
                .flat_map(|x| x.to_bits().to_le_bytes())
                .collect::<Vec<u8>>()
        };
        let keys: [i32; 6] = [0x50, 0x50, -0x30, 0x105, 0x60, 0x60];
        let values = [10.0f32, 10.0, 25.0, -0.0, 3.0, 2.0];
        let floors = [-100.0f32, -100.0, -100.0, -100.0, -100.0, 9.0];
        let kbytes: Vec<u8> = keys.iter().flat_map(|k| k.to_le_bytes()).collect();
        let keys_at = t.alloc(&kbytes);
        let values_at = t.alloc(&fbytes(&values));
        let floors_at = t.alloc(&fbytes(&floors));
        let out = t.alloc(&fbytes(&[0.0f32; 0x110]));
        let mut table = Vec::new();
        for k in 0..17u16 {
            let mut levels = Vec::new();
            for l in 0..8u16 {
                let first = if k <= 1 { 30.0f32 } else { 1.0 };
                let mut p = vec![first, first + 4.0];
                p.extend((0..40u16).map(|i| {
                    if k == 16 {
                        -0.0
                    } else {
                        0.25 * f32::from(k * 8 + l) + 0.01 * f32::from(i)
                    }
                }));
                levels.push(t.alloc(&fbytes(&p)));
            }
            table.push(
                t.alloc(
                    &levels
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect::<Vec<_>>(),
                ),
            );
        }
        let tables = t.alloc(
            &table
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let owner = t.zeros(0x200);
        t.w32(owner + 0x1f0, 40.0f32.to_bits());
        let this = t.zeros(0x30);
        t.w32(this, keys.len() as u32);
        t.w32(this + 4, owner);
        t.w32(this + 0x14, keys_at);
        t.w32(this + 0x1c, 0);
        t.w32(this + 0x20, 4);
        t.w32(this + 0x24, 4);
        t.w32(this + 0x28, 26);
        let bias = 10.0f32.to_bits();
        t.check(
            &format!("{label}: raise profiles edges"),
            0x00a9_65d0,
            Conv::FastcallCallerPops,
            &[this, tables, values_at, floors_at, out, bias],
            |g| {
                l2b::raise_profiles(g, this, tables, values_at, floors_at, out, bias);
                Out::None
            },
        );
        t.reset();
    }
}

/// `0x00a191f0` (the deferred lighting's targets taken) against the
/// original: already taken; each handle missing, of another size or
/// format; formats 0 and 2 and an unsupported one; each later step
/// failing, the target maker failing with and without an error word;
/// the feature levels on both sides of 0xb000. The error word the maker
/// writes is the original's argument slot, which the port gets as its
/// `out` (the same address here).
#[test]
#[allow(clippy::type_complexity)] // one row per case
fn big_take_targets_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (taken, the handle changed (9 none), how: 0 missing, 1 width,
    // 2 height, 3 format; the sixth's format; the step errors
    // [geometry, resize, maker target, maker error, 0x00a19cf0]; level)
    let cases: [(bool, u32, u32, u8, [u32; 5], u32); 17] = [
        (true, 9, 0, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 9, 0, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 9, 0, 2, [0, 0, 1, 0, 0], 0xafff),
        (false, 9, 0, 1, [0, 0, 1, 0, 0], 0xb000),
        (false, 5, 0, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 0, 0, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 1, 1, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 3, 2, 2, [0, 0, 1, 0, 0], 0xb000),
        (false, 2, 3, 2, [0, 0, 1, 0, 0], 0xb000),
        (false, 3, 3, 0, [0, 0, 1, 0, 0], 0xb000),
        (false, 4, 3, 2, [0, 0, 1, 0, 0], 0xb000),
        (false, 9, 0, 0, [7, 0, 1, 0, 0], 0xb000),
        (false, 9, 0, 0, [0, 8, 1, 0, 0], 0xb000),
        (false, 9, 0, 0, [0, 0, 0, 0xd, 0], 0xb000),
        (false, 9, 0, 0, [0, 0, 0, 0, 0], 0xb000),
        (false, 9, 0, 0, [0, 0, 1, 0xd, 0], 0xb000),
        (false, 9, 0, 0, [0, 0, 1, 0, 9], 0xc000),
    ];
    for (taken, which, how, format, errs, level) in cases {
        for (label, mut t) in Twins::all() {
            let c = l2::TAKE_TARGETS;
            t.stub(c[0], true, 1, 1, errs[0]);
            t.stub(c[1], true, 2, 2, errs[1]);
            t.stub(c[4], true, 0, 0, errs[4]);
            let cache = t.zeros(0xc0);
            t.w32(cache + 0xb4, level);
            t.stub(c[5], false, 0, 0, cache);
            // the maker: stores the error through its seventh word, logs
            // six, answers the target
            let target = if errs[2] != 0 { 0x0088_0000 } else { 0 };
            let logger = t.stub_code(c[3], false, 6, 0, target);
            let code = t.a.alloc_code(&[0xcc; 0x20]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 0x20]).unwrap(), code);
            let mut k = vec![0x8b, 0x44, 0x24, 0x1c, 0xc7, 0x00]; // mov eax, [esp+0x1c]; mov dword [eax], err
            k.extend(errs[3].to_le_bytes());
            k.push(0xe9);
            k.extend(logger.wrapping_sub(code + 15).to_le_bytes()); // jmp logger
            t.write(code, &k);
            let at = t.rebase(c[3]);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);

            let this = t.zeros(0x4040);
            t.w32(this + 0x3ed4, u32::from(taken));
            t.w8(this + 0x402c, 0xff);
            t.w8(this + 0x3f90, 0x01);
            let mut handles = Vec::new();
            for i in 0..6u32 {
                if i == which && how == 0 {
                    handles.push(0);
                    continue;
                }
                let obj = t.zeros(0x24);
                let w = if i == which && how == 1 { 0x101 } else { 0x100 };
                let ht = if i == which && how == 2 { 0x81 } else { 0x80 };
                t.w32(obj + 0x1c, w);
                t.w32(obj + 0x20, ht);
                let hnd = t.zeros(8);
                t.w32(hnd, obj);
                let f = match i {
                    2 | 3 | 5 => format,
                    _ => 0,
                };
                let f = if i == which && how == 3 { f ^ 2 } else { f };
                t.w8(hnd + 5, f);
                handles.push(hnd);
            }
            let targets = t.alloc(
                &handles
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            let ebp = t.ebp_for(2);
            let what = format!(
                "{label}: take targets taken {taken} which {which} how {how} format {format} errs {errs:?} level {level:#x}"
            );
            t.check_frame(
                &what,
                0x00a1_91f0,
                Conv::Thiscall(this),
                &[0x0055_0000, targets],
                ebp,
                0x20,
                (0, 0),
                |g| Out::U32(l2b::take_targets(g, this, 0x0055_0000, targets, ebp + 0xc)),
            );
            t.reset();
        }
    }
}

/// `0x00a19cf0` (the deferred lighting's buffers) against the original:
/// slot counts of 8 and others, the resize and each create failing, the
/// render state cache built or not, levels on both sides of 0xb000, the
/// structured buffers' setup failing.
#[test]
fn big_create_buffers_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (slot count, resize error, create error, guard set, level, setup error)
    for (count, resize, create, guard, level, setup) in [
        (8u32, 0u32, 0u32, true, 0xb000u32, 0u32),
        (0, 0, 0, false, 0xb000, 0),
        (3, 5, 0, true, 0xafff, 0),
        (0x8000_0008, 0, 0, true, 0xc000, 4),
        (8, 0, 9, true, 0xb000, 0),
        (3, 0, 0, false, 0x9000, 0),
    ] {
        for (label, mut t) in Twins::all() {
            let c = l2::DEFERRED_BUFFERS;
            let block = t.zeros(0x60);
            t.w32(block + 4, 0x77);
            t.w32(block + 0xc, count);
            t.stub(c[0], false, 1, 0, block);
            t.stub(c[1], true, 0, 0, block);
            t.stub(c[2], true, 1, 1, resize);
            t.stub(c[3], true, 2, 2, create);
            t.stub(c[4], true, 0, 0, block);
            t.stub(c[5], true, 4, 4, setup);
            t.stub(c[8], true, 0, 0, 0x00c9_4f00);
            t.stub(c[10], false, 1, 0, 0);
            let g = t.rebase(c[6]);
            t.w32(g, if guard { 0x11 } else { 0x10 });
            let cache = t.rebase(c[7]);
            t.w32(cache + 0xb4, level);
            let this = t.zeros(0x4040);
            let what = format!(
                "{label}: create buffers count {count:#x} resize {resize} create {create} guard {guard} level {level:#x} setup {setup}"
            );
            t.check(&what, 0x00a1_9cf0, Conv::Thiscall(this), &[], |g| {
                Out::U32(l2b::create_buffers(g, this))
            });
            t.reset();
        }
    }
}

impl Twins {
    /// Code in both oracles at the same address.
    fn code_at(&mut self, c: &[u8]) -> u32 {
        let at = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), at);
        at
    }

    /// A `jmp` at the original `sym` to `code`, put back by `reset`.
    fn place_jmp(&mut self, sym: u32, code: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// `0x005b6620` (the deferred pass's draw commands queued) against the
/// original: busy, no effect, the technique from the table or the finder
/// (index -1, out of range, a null entry; none found), no pass; the block
/// tree absent, without an entry, with one; room or not; capture flags
/// 0..3; passes that draw and do not. Callees are recording stubs; the
/// sizes routine writes through the three words it gets (the original's
/// frame and first argument slot, the port's frame at the same address).
#[test]
#[allow(clippy::type_complexity)] // one row per case
fn big_queue_pass_matches_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (busy, effect, technique: 0 table, 1 index -1, 2 out of range,
    // 3 null entry, 4 none found; pass there, tree: 0 none, 1 no entry,
    // 2 entry; room, flags)
    let cases: [(bool, bool, u32, bool, u32, bool, u32); 12] = [
        (true, true, 0, true, 0, true, 0),
        (false, false, 0, true, 0, true, 0),
        (false, true, 0, true, 0, true, 0),
        (false, true, 1, true, 1, true, 1),
        (false, true, 2, true, 2, false, 2),
        (false, true, 3, true, 2, true, 3),
        (false, true, 4, true, 0, true, 0),
        (false, true, 0, false, 0, true, 0),
        (false, true, 0, true, 2, false, 3),
        (false, true, 1, true, 0, true, 2),
        (false, true, 3, true, 1, false, 1),
        (false, true, 0, true, 2, true, 1),
    ];
    for (busy, has_effect, tech, has_pass, tree, room, flags) in cases {
        for (label, mut t) in Twins::all() {
            let c = l2::QUEUE_PASS;
            let renderer_vt = t.object(&[0, 0, 0], &[0, 0, 0x99]);
            let vt = t.r32(renderer_vt);
            let this = t.zeros(0x60);
            t.w32(this, vt);
            t.w8(this + 4, u8::from(busy) | 0x10);
            let effect = t.zeros(0x10);
            t.w32(
                effect + 0xc,
                match tech {
                    1 => u32::MAX,
                    2 => 9,
                    _ => 1,
                },
            );
            if has_effect {
                t.w32(this + 0x48, effect);
            }
            // the pass: flags, n (bit 31 set), the passes (0x254 apart)
            // 257 passes: more than a byte's worth
            let passes = t.zeros(0x254 * 0x101 + 0x10);
            t.w32(passes + 0xc, 1);
            t.w32(passes + 0x254 * 2 + 0xc, 5);
            let pass = t.zeros(0x10);
            t.w32(pass, flags | 0x40);
            t.w32(pass + 4, 0x8000_0101);
            t.w32(pass + 8, passes);
            let technique = t.zeros(0x10);
            if has_pass {
                t.w32(technique + 8, pass);
            }
            let material = t.zeros(0x20);
            // entry 9 (past the count) a technique without a pass
            let other = t.zeros(0x10);
            let mut entries = [0u32; 10];
            entries[1] = if tech >= 3 { 0 } else { technique };
            entries[9] = other;
            let table = t.alloc(
                &entries
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            t.w32(material + 0x14, 0x8000_0003);
            t.w32(material + 0x18, table);
            t.stub(c[0], true, 1, 1, if tech == 4 { 0 } else { technique });
            let tree_at = t.rebase(c[1]);
            t.w32(tree_at, if tree == 0 { 0 } else { 0x00d1_0000 });
            t.stub(c[2], true, 2, 2, if tree == 2 { 0x00d2_0000 } else { 0 });
            t.stub_import(c[3], 2, 2, 0);
            // the sizes: writes total, table, blocks through its words
            let log9 = t.stub_code(c[4], false, 6, 0, 0);
            let mut k = Vec::new();
            for (i, v) in [(4u8, 0x1234u32), (8, 0x5555), (0xc, 0x80)] {
                k.extend([0x8b, 0x44, 0x24, i, 0xc7, 0x00]); // mov eax, [esp+i]; mov [eax], v
                k.extend(v.to_le_bytes());
            }
            let code = t.code_at(&[0xcc; 0x40]);
            k.push(0xe9);
            let next = code + k.len() as u32 + 4;
            k.extend(log9.wrapping_sub(next).to_le_bytes()); // jmp log9
            t.write(code, &k);
            t.place_jmp(c[4], code);
            t.stub(c[5], true, 2, 2, u32::from(room) | 0x100);
            t.stub(c[6], true, 0, 0, 0x00d3_0000);
            t.stub(c[7], true, 1, 1, 0x00d4_0000);
            // the rings: logs this and the size, the data ring answers the
            // table, the command ring a command block
            let buf = t.zeros(0x40);
            let cmd = t.zeros(0x50);
            let log1 = t.stub_code(c[8], true, 1, 0, 0);
            let code = t.code_at(&[0xcc; 0x40]);
            let mut k = vec![0xff, 0x74, 0x24, 0x04]; // push [esp+4]
            k.push(0xe8);
            k.extend(log1.wrapping_sub(code + 9).to_le_bytes()); // call log1
            k.extend([0x83, 0xc4, 0x04]); // add esp, 4
            k.extend([0x81, 0xf9]);
            k.extend((this + 0x1c).to_le_bytes()); // cmp ecx, this + 0x1c
            k.extend([0x75, 0x08]); // jne +8
            k.push(0xb8);
            k.extend(buf.to_le_bytes()); // mov eax, buf
            k.extend([0xc2, 0x04, 0x00]); // ret 4
            k.push(0xb8);
            k.extend(cmd.to_le_bytes()); // mov eax, cmd
            k.extend([0xc2, 0x04, 0x00]); // ret 4
            t.write(code, &k);
            t.place_jmp(c[8], code);
            // the texture pointers: logs, writes two counters, answers the
            // cursor past them
            let log3 = t.stub_code(c[9], false, 3, 0, 0);
            let code = t.code_at(&[0xcc; 0x40]);
            let mut k = vec![
                0xff, 0x74, 0x24, 0x0c, 0xff, 0x74, 0x24, 0x0c, 0xff, 0x74, 0x24, 0x0c,
            ]; // push [esp+0xc] x3
            k.push(0xe8);
            k.extend(log3.wrapping_sub(code + 17).to_le_bytes()); // call log3
            k.extend([0x83, 0xc4, 0x0c]); // add esp, 0xc
            k.extend([0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc]
            k.extend([0xc7, 0x00]);
            k.extend(0x00d5_0000u32.to_le_bytes()); // mov [eax], c1
            k.extend([0xc7, 0x40, 0x04]);
            k.extend(0x00d6_0000u32.to_le_bytes()); // mov [eax+4], c2
            k.extend([0x83, 0xc0, 0x08, 0xc3]); // add eax, 8; ret
            t.write(code, &k);
            t.place_jmp(c[9], code);

            let ebp = t.ebp_for(5);
            let instance = 0x00d7_0000;
            let what = format!(
                "{label}: queue pass busy {busy} effect {has_effect} tech {tech} pass {has_pass} tree {tree} room {room} flags {flags}"
            );
            t.check_frame(
                &what,
                0x005b_6620,
                Conv::Thiscall(this),
                &[material, instance, 0xa1, 0xb2, 0xc3],
                ebp,
                0x40,
                // the table word; the next is the original's loop counter
                (0xc, 8),
                |g| {
                    Out::U32(l2b::queue_pass(
                        g,
                        this,
                        material,
                        instance,
                        [0xa1, 0xb2, 0xc3],
                        ebp - 0xc,
                    ))
                },
            );
            t.reset();
        }
    }
}

/// `0x0058f2b0` (a structured buffer's D3D11 buffers) against the
/// original: one slot and two, with and without initial data, dynamic
/// and not, the cache built or not, a failed create, the views' answer.
/// The device and the buffers are COM objects of recording stubs; the
/// whole frame (description, initial data, counter) is compared at the
/// original's address.
#[test]
fn big_structured_buffers_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (slots, data, flags, guard set, create result, views result)
    for (slots, data, flags, guard, hr, views) in [
        (1u32, true, 0u32, true, 0u32, 0u32),
        (2, false, 1, false, 0, 9),
        (2, true, 3, true, 0, 0),
        (1, false, 0, true, 0x8007_000e, 0),
        (2, true, 0, true, 0x8007_0057, 0),
    ] {
        for (label, mut t) in Twins::all() {
            let c = l2::STRUCTURED;
            t.stub(c[0], true, 0, 0, 0x0042_0000);
            let g = t.rebase(c[1]);
            t.w32(g, if guard { 0x11 } else { 0x10 });
            t.stub(c[3], true, 0, 0, 0);
            t.stub(c[5], false, 1, 0, 0);
            t.stub(c[8], true, 0, 0, 0x0043_0000);
            t.stub(c[9], true, 0, 0, views);
            let device = t.com_object(&[1, 1, 1, 4], &[0, 0, 0, hr]);
            let dev_at = t.rebase(c[6]);
            t.w32(dev_at, device);
            let this = t.zeros(0x60);
            let objects: Vec<u32> = (0..slots)
                .map(|_| t.com_object(&[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2], &[]))
                .collect();
            if slots == 1 {
                t.w32(this + 0x14, 1);
                t.w32(this + 0x18, objects[0]);
            } else {
                let arr = t.zeros(0x14 * slots as usize);
                for (i, &o) in objects.iter().enumerate() {
                    t.w32(arr + 0x14 * i as u32, o);
                }
                t.w32(this + 0x14, 0x8000_0000 | slots);
                t.w32(this + 0x18, arr);
            }
            let ebp = t.ebp_for(4);
            let args = [0x40, 0x10, flags, if data { 0x0044_0000 } else { 0 }];
            let what = format!(
                "{label}: structured slots {slots} data {data} flags {flags} guard {guard} hr {hr:#x} views {views}"
            );
            t.check_frame(
                &what,
                0x0058_f2b0,
                Conv::Thiscall(this),
                &args,
                ebp,
                0x60,
                (0x44, 0x10),
                |g| Out::U32(l2b::create_structured(g, this, args, ebp - 0x44)),
            );
            t.reset();
        }
    }
}

/// `0x0058ec50` (the render target read back as RGB rows) against the
/// original: no destination, no target, the inline target and the array,
/// each texture creation failing (an error or no texture), the map
/// failing, a 3 x 2 read back. The device's `CreateTexture2D` stores a
/// texture chosen by the description's usage; `GetDesc` and `Map` fill
/// what the real ones would.
#[test]
fn big_read_back_matches_the_original() {
    use ffx_prender::m5p_render_b_left2::bridge as l2b;
    // (dst, targets: 0 none, 1 inline, 2 array; first texture: 0 ok,
    // 1 error, 2 none; second likewise; map error)
    for (with_dst, targets, first, second, map_hr) in [
        (false, 1u32, 0u32, 0u32, 0u32),
        (true, 0, 0, 0, 0),
        (true, 1, 0, 0, 0),
        (true, 2, 0, 0, 0),
        (true, 1, 1, 0, 0),
        (true, 1, 2, 0, 0),
        (true, 1, 0, 1, 0),
        (true, 2, 0, 2, 0),
        (true, 1, 0, 0, 0x8000_4005),
    ] {
        for (label, mut t) in Twins::all() {
            // a COM object whose slot `slot` runs `code` (the rest record)
            let with_slot = |t: &mut Twins, words: &[u32], slot: usize, code: &[u8]| -> u32 {
                let obj = t.com_object(words, &[]);
                let at = t.a.alloc_code(code).unwrap();
                assert_eq!(t.b.alloc_code(code).unwrap(), at);
                let vt = t.r32(obj);
                t.w32(vt + 4 * slot as u32, at);
                obj
            };
            // GetDesc(this, desc): width 3, height 2, junk elsewhere kept
            let mut get_desc = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
            get_desc.extend([0xc7, 0x00, 3, 0, 0, 0]); // mov [eax], 3
            get_desc.extend([0xc7, 0x40, 0x04, 2, 0, 0, 0]); // mov [eax+4], 2
            get_desc.extend([0xc2, 0x08, 0x00]); // ret 8
            let target = with_slot(&mut t, &[1; 11], 10, &get_desc);
            let tex1 = t.com_object(&[1; 3], &[]);
            let tex2 = t.com_object(&[1; 3], &[]);
            let obj = |kind: u32, tex: u32| if kind == 2 { 0 } else { tex };
            let hr = |kind: u32| if kind == 1 { 0x8007_000e_u32 } else { 0 };
            // CreateTexture2D(this, desc, init, out): staging (usage 3) gets
            // the second texture
            let mut create = vec![0x8b, 0x44, 0x24, 0x08, 0x83, 0x78, 0x1c, 0x03]; // mov eax,[esp+8]; cmp dword [eax+0x1c], 3
            create.extend([0x8b, 0x44, 0x24, 0x10]); // mov eax, [esp+0x10]
            create.extend([0x74, 0x0e]); // je second
            create.extend([0xc7, 0x00]);
            create.extend(obj(first, tex1).to_le_bytes()); // mov [eax], tex1
            create.push(0xb8);
            create.extend(hr(first).to_le_bytes()); // mov eax, hr1
            create.extend([0xc2, 0x10, 0x00]); // ret 0x10
            create.extend([0xc7, 0x00]);
            create.extend(obj(second, tex2).to_le_bytes()); // second: mov [eax], tex2
            create.push(0xb8);
            create.extend(hr(second).to_le_bytes()); // mov eax, hr2
            create.extend([0xc2, 0x10, 0x00]); // ret 0x10
            let device = with_slot(&mut t, &[1; 6], 5, &create);
            // Map(this, res, sub, type, flags, out): the pixels
            let pixels = t.alloc(&(0..0x40u8).collect::<Vec<_>>());
            let mut map = vec![0x8b, 0x44, 0x24, 0x18]; // mov eax, [esp+0x18]
            map.extend([0xc7, 0x00]);
            map.extend(pixels.to_le_bytes()); // mov [eax], pixels
            map.extend([0xc7, 0x40, 0x04, 0x10, 0, 0, 0]); // mov [eax+4], 16
            map.extend([0xc7, 0x40, 0x08, 0, 0, 0, 0]); // mov [eax+8], 0
            map.push(0xb8);
            map.extend(map_hr.to_le_bytes()); // mov eax, hr
            map.extend([0xc2, 0x18, 0x00]); // ret 0x18
            let mut words = vec![1u32; 0x3a];
            words[0xe4 / 4] = 6;
            words[0xbc / 4] = 3;
            words[0x3c / 4] = 3;
            words[0x38 / 4] = 6;
            let ctx = with_slot(&mut t, &words, 0x38 / 4, &map);
            let this = t.zeros(0x100);
            t.w32(this + 0x6c, device);
            t.w32(this + 0x70, ctx);
            match targets {
                1 => {
                    t.w32(this + 0xdc, 0x8000_0001);
                    t.w32(this + 0xe0, target);
                }
                2 => {
                    let arr = t.alloc(&[target.to_le_bytes(), 0u32.to_le_bytes()].concat());
                    t.w32(this + 0xdc, 2);
                    t.w32(this + 0xe0, arr);
                }
                _ => {}
            }
            let dst = if with_dst { t.junk(0x20, 6) } else { 0 };
            let ebp = t.ebp_for(1);
            let what = format!(
                "{label}: read back dst {with_dst} targets {targets} first {first} second {second} map {map_hr:#x}"
            );
            t.check_frame(
                &what,
                0x0058_ec50,
                Conv::Thiscall(this),
                &[dst],
                ebp,
                0x60,
                (0x4c, 4),
                |g| Out::U32(l2b::read_back(g, this, dst, ebp - 0x4c)),
            );
            t.reset();
        }
    }
}

/// `0x00a22be0` (the deferred lighting's targets resized) against the
/// original: states 1, 2, 3 and another; textures of the size and of
/// another; a handle missing in each way at a later position; bound or
/// not; the unbind, the resize, a render target resize and the buffers
/// failing.
#[test]
#[allow(clippy::type_complexity)] // one row per case
fn big_resize_targets_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (state, broken handle index (9 none), how: 0 null, 1 flag, 2 no
    // texture; bound, errors [unbind, setDimensions, rt resize, buffers])
    let cases: [(u32, u32, u32, bool, [u32; 4]); 12] = [
        (1, 9, 0, true, [0, 0, 0, 0]),
        (2, 9, 0, false, [0, 0, 0, 7]),
        (3, 9, 0, true, [0, 0, 0, 0]),
        (0, 9, 0, true, [0, 0, 0, 5]),
        (1, 1, 0, true, [0, 0, 0, 0]),
        (2, 3, 1, false, [0, 0, 0, 0]),
        (3, 2, 2, true, [0, 0, 0, 0]),
        (3, 9, 0, true, [4, 0, 0, 0]),
        (2, 9, 0, false, [0, 6, 0, 0]),
        (3, 9, 0, false, [0, 0, 8, 0]),
        (1, 9, 0, true, [0, 3, 0, 0]),
        (3, 0, 0, false, [0, 0, 0, 0]),
    ];
    for (state, broken, how, bound, errs) in cases {
        for (label, mut t) in Twins::all() {
            let c = l2::RESIZE_TARGETS;
            t.stub(c[0], true, 0, 0, u32::from(bound) | 0x300);
            t.stub(c[1], true, 0, 0, errs[0]);
            t.stub(c[2], true, 4, 4, errs[1]);
            t.stub(c[3], false, 4, 0, errs[2]);
            t.stub(l2::TAKE_TARGETS[1], true, 2, 2, errs[3]);
            let this = t.zeros(0x4000);
            t.w32(this + 0x3ed4, state);
            let fields: &[u32] = match state {
                1 => &[0x3f94, 0x3f98, 0x3f9c],
                2 => &[0x3fa0, 0x3fa4, 0x3fa8, 0x3fac],
                _ => &[0x3fb0, 0x3fb4, 0x3fb8, 0x3fbc],
            };
            for (i, &f) in fields.iter().enumerate() {
                let i = i as u32;
                if i == broken && how == 0 {
                    continue;
                }
                let tex = t.zeros(0x24);
                t.w32(tex, 0x0099_0000 + i);
                // every other texture already of the size
                t.w32(tex + 0x1c, if i.is_multiple_of(2) { 0x200 } else { 0x100 });
                // the fourth of the width but not the height
                t.w32(tex + 0x20, if i == 3 { 0x81 } else { 0x80 });
                let handle = t.zeros(8);
                t.w32(handle, if i == broken && how == 2 { 0 } else { tex });
                t.w8(handle + 4, u8::from(i == broken && how == 1));
                t.w32(this + f, handle);
            }
            t.w32(this + 0x3fc0, 0x0077_0000);
            t.w32(this + 0x3fc4, 0x0078_0000);
            let what = format!(
                "{label}: resize targets state {state} broken {broken} how {how} bound {bound} errs {errs:?}"
            );
            t.check(
                &what,
                0x00a2_2be0,
                Conv::Thiscall(this),
                &[0x100, 0x80],
                |g| Out::U32(l2b::resize_targets(g, this, 0x100, 0x80)),
            );
            t.reset();
        }
    }
}

/// `0x00a179a0` (the post-process utility's downsample targets) against
/// the original: every step answering, then each step failing (no
/// texture, a size error, no list, no instance). The callees are
/// recording stubs; the texture and the instance are blocks the test
/// owns.
#[test]
fn big_downsample_targets_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // which step fails: 9 none, 0 create, 1 setDimensions, 2 the list,
    // 3 the instance
    for fail in [9u32, 0, 1, 2, 3] {
        for (label, mut t) in Twins::all() {
            let c = l2::DOWNSAMPLE;
            let tex = t.junk(0x20, 3);
            let inst = t.junk(0x20, 4);
            let list = t.zeros(0x40);
            t.w32(list + 0x34, if fail == 3 { 0 } else { inst });
            t.stub(c[1], true, 3, 3, if fail == 0 { 0 } else { tex });
            t.stub(c[3], true, 4, 4, if fail == 1 { 3 } else { 0 });
            t.stub(c[4], true, 0, 0, 0);
            t.stub(c[6], true, 3, 3, if fail == 2 { 0 } else { list });
            t.stub(c[7], true, 1, 1, inst);
            let this = t.junk(0xb0, 5);
            let what = format!("{label}: downsample targets fail {fail}");
            t.check(
                &what,
                0x00a1_79a0,
                Conv::Thiscall(this),
                &[0x0055_0000, 0x500, 0x2d3],
                |g| {
                    Out::U32(l2b::create_downsample_targets(
                        g,
                        this,
                        0x0055_0000,
                        0x500,
                        0x2d3,
                    ))
                },
            );
            t.reset();
        }
    }
}

/// `0x00a19620` (the deferred lighting's own targets) against the
/// original: taken already; the size given and defaulted; the depth
/// target's steps failing (no texture, a size error, no list, no
/// instance); the k-th render target answering none or an error; all
/// made and the take answering 0 or an error. The maker is a counting
/// stub (it logs its first six words, stores the error through the
/// seventh); the six targets are compared in the original's frame.
#[test]
#[allow(clippy::type_complexity)] // one row per case
fn big_own_targets_match_the_original() {
    use ffx_prender::m5p_render_b_left2::{self as l2, bridge as l2b};
    // (taken, w, h, depth step failing (9 none: 0 create, 1 size, 2 list,
    // 3 instance), maker call answering none (0 none), maker call
    // with an error (0 none), the take's answer)
    let cases: [(bool, u32, u32, u32, u32, u32, u32); 11] = [
        (true, 0x100, 0x80, 9, 0, 0, 0),
        (false, 0x100, 0x80, 9, 0, 0, 0),
        (false, 0, 0, 9, 0, 0, 7),
        (false, 0x100, 0x80, 0, 0, 0, 0),
        (false, 0x100, 0x80, 1, 0, 0, 0),
        (false, 0x100, 0x80, 2, 0, 0, 0),
        (false, 0x100, 0x80, 3, 0, 0, 0),
        (false, 0x100, 0x80, 9, 1, 0, 0),
        (false, 0x100, 0x80, 9, 0, 3, 0),
        (false, 0x100, 0x80, 9, 5, 0, 0),
        (false, 0x100, 0x80, 9, 0, 5, 0),
    ];
    for (taken, w, ht, depth_fail, none_at, err_at, take) in cases {
        for (label, mut t) in Twins::all() {
            let c = l2::DOWNSAMPLE;
            let tex = t.junk(0x20, 3);
            let inst = t.junk(0x20, 4);
            let list = t.zeros(0x40);
            t.w32(list + 0x34, if depth_fail == 3 { 0 } else { inst });
            t.stub(c[1], true, 3, 3, if depth_fail == 0 { 0 } else { tex });
            t.stub(c[3], true, 4, 4, if depth_fail == 1 { 6 } else { 0 });
            t.stub(c[4], true, 0, 0, 0);
            t.stub(c[6], true, 3, 3, if depth_fail == 2 { 0 } else { list });
            t.stub(c[7], true, 1, 1, inst);
            t.stub(l2::TAKE_TARGETS_FN, true, 2, 2, take);
            let guard = t.rebase(l2::DEFERRED_BUFFERS[6]);
            t.w32(guard, 0x11);
            let screen = t.rebase(l2::OWN_TARGETS[0]);
            t.w32(screen, 0x500);
            t.w32(screen + 4, 0x2d0);
            // the maker
            let cnt = t.zeros(4);
            let logger = t.stub_code(l2::TAKE_TARGETS[3], false, 6, 0, 0);
            let code = t.a.alloc_code(&[0xcc; 0x60]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 0x60]).unwrap(), code);
            let mut k = Vec::new();
            for _ in 0..6 {
                k.extend([0xff, 0x74, 0x24, 0x18]); // push [esp+0x18]
            }
            k.push(0xe8);
            k.extend(logger.wrapping_sub(code + 29).to_le_bytes()); // call logger
            k.extend([0x83, 0xc4, 0x18]); // add esp, 0x18
            k.extend([0xff, 0x05]);
            k.extend(cnt.to_le_bytes()); // inc dword [cnt]
            k.push(0xa1);
            k.extend(cnt.to_le_bytes()); // mov eax, [cnt]
            k.extend([0x8b, 0x4c, 0x24, 0x1c]); // mov ecx, [esp+0x1c]
            k.extend([0xc7, 0x01, 0, 0, 0, 0]); // mov dword [ecx], 0
            k.push(0x3d);
            k.extend(err_at.to_le_bytes()); // cmp eax, err_at
            k.extend([0x75, 0x06]); // jne +6
            k.extend([0xc7, 0x01, 5, 0, 0, 0]); // mov dword [ecx], 5
            k.push(0x3d);
            k.extend(none_at.to_le_bytes()); // cmp eax, none_at
            k.extend([0x74, 0x06]); // je zero
            k.push(0x05);
            k.extend(0x0088_0000u32.to_le_bytes()); // add eax, 0x880000
            k.push(0xc3); // ret
            k.extend([0x31, 0xc0, 0xc3]); // zero: xor eax, eax; ret
            t.write(code, &k);
            let at = t.rebase(l2::TAKE_TARGETS[3]);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);

            let this = t.zeros(0x4000);
            t.w32(this + 0x3ed4, u32::from(taken));
            let ebp = t.ebp_for(2);
            let what = format!(
                "{label}: own targets taken {taken} {w}x{ht} depth {depth_fail} none {none_at} err {err_at} take {take}"
            );
            t.check_frame(
                &what,
                0x00a1_9620,
                Conv::Thiscall(this),
                &[w, ht],
                ebp,
                0x60,
                (0x34, 0x1c),
                |g| Out::U32(l2b::create_targets(g, this, w, ht, ebp - 0x34)),
            );
            t.reset();
        }
    }
}

// ---- big-phyre2 ----

use ffx_prender::m5p_render_b_bp2::{self as bp2, bridge as bp2b};

/// `0x00a139c0` on every path: no texture parameter, each parameter
/// check (type, `TrophyManager_Install`'s error, offset and size), then
/// each scene call failing in turn, and the whole pass. Only synthetic
/// data reaches it (the glow effect is never set up on the snapshots).
#[test]
fn bp2_glow_render_mesh() {
    for (label, mut t) in Twins::all() {
        // (has a parameter, type byte, texture, install error, offset,
        // size, failing scene call)
        let cases: &[(bool, u8, u32, u32, u16, u16, usize)] = &[
            (false, 0, 0x7e07, 0, 0, 0, 9),
            (false, 0, 0x7e07, 0, 0, 0, 0),
            (false, 0, 0x7e07, 0, 0, 0, 1),
            (false, 0, 0x7e07, 0, 0, 0, 2),
            (false, 0, 0x7e07, 0, 0, 0, 3),
            (false, 0, 0x7e07, 0, 0, 0, 4),
            (true, 0x41, 0x7e07, 0, 8, 0x10, 9),
            (true, 0xdd, 0x7e07, 0, 8, 0x10, 9),
            (true, 0xd8, 0x7e07, 0, 8, 0x10, 9),
            (true, 0x42, 0x7e07, 7, 8, 0x10, 9),
            (true, 0x42, 0, 7, 8, 0x10, 9),
            (true, 0xd9, 0x7e07, 0, 0x18, 0x10, 9),
            (true, 0xda, 0x7e07, 0, 0x10, 0x10, 9),
            (true, 0xdb, 0x7e07, 0, 2, 0x8, 9),
            (true, 0xdb, 0x7e07, 0, 3, 0x8, 9),
            (true, 0xdc, 0x7e07, 0, 4, 0xe008, 9),
            (true, 0x12, 0x7e07, 0, 4, 0xc, 2),
            (true, 0x42, 0x7e07, 0, 0x14, 0xc, 9),
        ];
        for (k, &(has, kind, texture, install, off, size, fail)) in cases.iter().enumerate() {
            let effect = t.junk(0xb0, k as u8);
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            let instance = t.junk(0x20, 0x80 ^ k as u8);
            t.w32(effect + 0x5c, instance);
            if has {
                let param = t.zeros(0x10);
                t.w8(param + 2, kind);
                t.write(param + 8, &off.to_le_bytes());
                t.write(param + 0xa, &size.to_le_bytes());
                let buffer = t.zeros(0x40);
                t.w32(buffer, 0x20);
                let params = t.zeros(8);
                t.w32(params + 4, buffer);
                t.w32(effect + 0x60, params);
                t.w32(effect + 0x74, param);
            } else {
                t.w32(effect + 0x74, 0);
            }
            let err = |i: usize| if i == fail { 0x20 + i as u32 } else { 0 };
            t.stub(bp2::TROPHY_INSTALL, true, 1, 1, install);
            t.stub(bp2::MEMCPY, false, 3, 0, 0x3c);
            t.stub(bp2::SURFACE_CTOR, true, 2, 2, 0x5c);
            t.stub(bp2::SURFACE_DTOR, true, 0, 0, 0x5d);
            t.stub(bp2::SET_COLOR_TARGET, true, 2, 2, err(0));
            t.stub(bp2::SET_DEPTH_TARGET, true, 1, 1, err(1));
            t.stub(bp2::BEGIN_SCENE, true, 1, 1, err(2));
            t.stub(bp2::QUEUE_COMMAND_18, true, 3, 3, 0xd);
            t.stub(bp2::RENDER_MESH_INSTANCE, true, 4, 4, err(3));
            t.stub(bp2::END_SCENE, true, 1, 1, err(4));
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: glow mesh {k}"),
                0x00a1_39c0,
                Conv::Thiscall(effect),
                &[renderer, texture],
                ebp,
                0x2c,
                (0x2c, 0xc),
                |g| Out::U32(bp2b::glow_render_mesh(g, ebp, effect, renderer, texture)),
            );
            assert!(t.log_entries() > 0 || has, "{label}: {k}: no calls");
            t.reset();
        }
    }
}

impl Twins {
    /// A recording stub at the original `sym` that logs all `n` stack
    /// words (six per log entry, `ecx` in each entry when `this`), pops
    /// `pops` words, and when `out` is `Some(v)` stores `v` through its
    /// first word; it returns `ret`.
    fn stub_wide(&mut self, sym: u32, this: bool, n: u32, pops: u32, out: Option<u32>, ret: u32) {
        let log = self.log;
        let mut c = Vec::new();
        let mut i = 0;
        loop {
            c.push(0xa1);
            c.extend(log.to_le_bytes()); // mov eax, [log]
            c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
            c.push(0x05);
            c.extend((log + 4).to_le_bytes()); // add eax, log + 4
            c.extend([0xc7, 0x00]);
            c.extend((sym ^ i).to_le_bytes()); // mov [eax], tag (^ first word)
            if this {
                c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            }
            for k in 0..(n - i).min(6) {
                c.extend([0x8b, 0x54, 0x24, (4 + 4 * (i + k)) as u8]); // mov edx, [esp+4+4w]
                c.extend([0x89, 0x50, (8 + 4 * k) as u8]); // mov [eax+8+4k], edx
            }
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [log]
            i += 6;
            if i >= n {
                break;
            }
        }
        if let Some(v) = out {
            c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
            c.extend([0xc7, 0x00]);
            c.extend(v.to_le_bytes()); // mov dword [eax], v
        }
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// A renderer for `0x005b55a0`: the record ring (junk records whose
/// target word `+4` points at a small object), the cursor at record
/// `slot`, the open range, the budgets and their limits.
fn queue_renderer(t: &mut Twins, rng: &mut Rng, slot: u32, limits: (u32, u32)) -> u32 {
    let r = t.junk(0x2330, rng.next() as u8);
    let base = t.junk(0x50, 7);
    t.w32(r, base);
    t.w32(r + 4, limits.1);
    t.w32(r + 8, limits.0);
    for k in 0..0x100u32 {
        let target = t.junk(0x10, k as u8);
        t.w32(r + 0xc + 0x20 * k + 4, target);
    }
    let range = t.zeros(0x30);
    t.w32(r + 0x2310, range);
    t.w32(r + 0x230c, r + 0xc + 0x20 * slot);
    t.w32(r + 0x2314, 3);
    t.w32(r + 0x2318, 0x40);
    t.w32(r + 0x231c, 0x10);
    t.w32(r + 0x2320, 0x20);
    r
}

/// A job for `0x005b55a0`: a mesh of `count` segments (`count` with bit
/// 31 set when `owned`), starting at `start`; `entries` material entries
/// in its list (the list's 0/1/many shapes), every third entry null;
/// segment `skip` skipped (`+0x14`); the passes' keys matching their
/// entry's kind on even segments.
fn queue_job(t: &mut Twins, count: u32, start: u32, entries: u32, skip: u32) -> u32 {
    let job = t.junk(0x14, 0x33);
    let mesh = t.junk(0x30, 0x44);
    t.w32(job + 8, mesh);
    t.w32(job + 0xc, start);
    t.w8(job + 0x10, 0x5a);
    if count == u32::MAX {
        t.w32(mesh, 0);
        return job;
    }
    let segments = t.zeros(8);
    t.w32(segments, count | 0x8000_0000);
    t.w32(mesh, segments);
    let n = count.max(1) as usize;
    let items = t.zeros(0x6c * n);
    t.w32(segments + 4, items);
    let mut kinds = Vec::new();
    let mut ptrs = Vec::new();
    for k in 0..entries.max(1) {
        let kind = t.junk(0x20, 0x60 ^ k as u8);
        t.write(kind + 0x1c, &(0x10 + 4 * k as u16).to_le_bytes());
        let e = t.junk(8, 0x70 ^ k as u8);
        t.w32(e, kind);
        kinds.push(kind);
        ptrs.push(if k % 3 == 2 { 0 } else { e });
    }
    let list = t.zeros(0x10);
    match entries {
        0 => t.w32(list, 0),
        1 => {
            t.w32(list, 1);
            t.w32(list + 4, ptrs[0]);
        }
        _ => {
            let arr = t.zeros(4 * ptrs.len());
            for (k, &p) in ptrs.iter().enumerate() {
                t.w32(arr + 4 * k as u32, p);
            }
            t.w32(list, entries);
            t.w32(list + 4, arr);
        }
    }
    t.w32(mesh + 0x10, list);
    t.w32(mesh + 0x18, 0x0d0d);
    let passes = t.junk(0x24 * n, 0x55);
    t.w32(mesh + 0x28, passes);
    let m = entries.max(1);
    for s in 0..n as u32 {
        let index = s % m;
        t.w32(items + 0x6c * s, index);
        t.w8(items + 0x6c * s + 0x14, u8::from(s == skip));
        let key = if s % 2 == 0 {
            kinds[index as usize]
        } else {
            0x1234
        };
        t.w32(passes + 0x24 * s + 0x18, key);
    }
    job
}

/// A case of [`bp2_queue_segments`]: count (MAX: no segment array),
/// start, entries, skipped segment, cursor slot, budget limits, size.
type QueueCase = (u32, u32, u32, u32, u32, (u32, u32), u32);

/// `0x005b55a0` on synthetic renderers and jobs: no segment array; start
/// past the count; the ring's first record and a cursor mid-ring; the
/// ring running out; every entry list shape and null entries; a skipped
/// segment; both budgets running over (sizes from the stub); all
/// queued. Only synthetic data reaches it. An empty entry list (data 0)
/// is not run: the original reads entry 0 through it and faults.
#[test]
fn bp2_queue_segments() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x005b_55a0);
        // (count or MAX for none, start, entries, skip, cursor slot,
        // limits (a, b), size the stub reports)
        let cases: &[QueueCase] = &[
            (u32::MAX, 0, 3, 99, 0, (0x1000, 0x1000), 4),
            (4, 4, 3, 99, 0, (0x1000, 0x1000), 4),
            (4, 6, 3, 99, 0, (0x1000, 0x1000), 4),
            (5, 0, 3, 99, 0, (0x1000, 0x1000), 4),
            (5, 1, 3, 2, 7, (0x1000, 0x1000), 4),
            (6, 0, 1, 99, 0, (0x1000, 0x1000), 4),
            (6, 0, 4, 99, 0xfd, (0x1000, 0x1000), 4),
            (6, 0, 4, 99, 0x100, (0x1000, 0x1000), 4),
            (6, 0, 2, 99, 3, (0x30, 0x1000), 4),
            (6, 0, 2, 99, 3, (0x1000, 0x2c), 4),
            (6, 0, 2, 99, 3, (0x10, 0x1000), 4),
            (6, 0, 2, 99, 3, (0x1000, 0x1f), 0),
            (8, 2, 5, 4, 1, (0x1000, 0x1000), 0x100),
            (4, 0, 2, 1, 3, (0x1000, 0x1000), 4),
            (4, 0, 2, 99, 3, (0x20, 0x1000), 4),
            (4, 1, 2, 99, 0, (0x1000, 0x1000), 4),
        ];
        for (k, &(count, start, entries, skip, slot, limits, size)) in cases.iter().enumerate() {
            let r = queue_renderer(&mut t, &mut rng, slot, limits);
            let job = queue_job(&mut t, count, start, entries, skip);
            let ctx = t.junk(0x30, 0x21);
            t.stub_wide(bp2::SELECT_TECHNIQUE_PASS, true, 8, 8, None, 0x5ba6);
            t.stub_wide(bp2::MESH_SIZES, false, 9, 0, Some(size), 0x5b51);
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: queue segments {k}"),
                0x005b_55a0,
                Conv::Thiscall(r),
                &[job, ctx],
                ebp,
                0x48,
                (0x44, 0x40),
                |g| Out::U32(bp2b::queue_segments(g, ebp, r, job, ctx)),
            );
            t.reset();
        }
    }
}

impl Twins {
    /// Recording stub code (logs `tag`, `n` stack words, pops `pops`)
    /// that stores `value` through its stack word `word` and returns `ret`.
    fn code_storing(&mut self, tag: u32, n: u32, pops: u32, word: u8, value: u32, ret: u32) -> u32 {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        for i in 0..n.min(7) {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (4 + 4 * i) as u8]); // mov [eax+4+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * word]); // mov eax, [esp+4+4w]
        c.extend([0xc7, 0x00]);
        c.extend(value.to_le_bytes()); // mov dword [eax], value
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }
}

/// A device context for `0x00591dd0`: `Map` (slot 14, stdcall, six words)
/// stores `mapped` into the mapped resource and answers `hr`, `Unmap`
/// (slot 15, three words) logs.
fn points_context(t: &mut Twins, mapped: u32, hr: u32) -> u32 {
    let map = t.code_storing(0xc0de_0038, 6, 6, 5, mapped, hr);
    let unmap = t.stub_code(0xc0de_003c, false, 3, 3, 0);
    let mut vt = vec![0u8; 0x40];
    vt[0x38..0x3c].copy_from_slice(&map.to_le_bytes());
    vt[0x3c..0x40].copy_from_slice(&unmap.to_le_bytes());
    let vt = t.alloc(&vt);
    let ctx = t.zeros(0x10);
    t.w32(ctx, vt);
    ctx
}

/// `0x00591dd0` on synthetic renderers: with and without the matrix;
/// `updateDepthTransform` and the pass state failing; no points; points
/// in one chunk and in two (0x1001), straight and through indices; the
/// helper's buffers present or not; the buffer index wrapping (15 to 0);
/// `drawSetup` failing on the first chunk. A failed `Map` is not run: the
/// original then writes through a null pointer. Only synthetic data
/// reaches it.
#[test]
fn bp2_draw_points() {
    use ffx_prender::m5p_render_b_bp2::{self as bp, points};
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x0059_1dd0);
        // (n, indices, matrix, failing step: 0 update, 1 create, 2 draw)
        let cases: &[(u32, bool, bool, u32)] = &[
            (0, false, false, 9),
            (3, false, true, 0),
            (3, false, false, 1),
            (5, false, false, 9),
            (5, true, true, 9),
            (4, false, true, 2),
            (0x1000, false, true, 9),
            (0x1001, false, false, 9),
            (0x1001, true, true, 9),
        ];
        for (k, &(n, with_indices, with_matrix, fail)) in cases.iter().enumerate() {
            let r = t.junk(0x9a0, k as u8);
            let q = r + 0x394;
            for (i, off) in [0x1c8u32, 0x1cc, 0x1d4, 0x1d0].into_iter().enumerate() {
                let obj = if (k + i) % 3 == 0 {
                    0
                } else {
                    t.com_object(&[1, 1, 1], &[0, 1, 2])
                };
                t.w32(q + off, obj);
            }
            t.w32(q + 0x40, if k % 2 == 0 { 15 } else { 3 });
            let mapped = t.zeros((0x28 * n.max(1)) as usize + 0x40);
            let ctx = points_context(&mut t, mapped, 0);
            t.w32(r + 0x70, ctx);
            let pts: Vec<u8> = (0..4 * n.max(1))
                .flat_map(|_| rng.next().to_le_bytes())
                .map(|b| b & 0x3f)
                .collect();
            let points_at = t.alloc(&pts);
            let colour = t.junk(0x10, 0x31);
            let indices = if with_indices {
                let idx: Vec<u8> = (0..n)
                    .flat_map(|_| (rng.next() % n).to_le_bytes())
                    .collect();
                t.alloc(&idx)
            } else {
                0
            };
            let m = if with_matrix { t.junk(0x40, 0x22) } else { 0 };
            let err = |i: u32| if i == fail { 0x80 + i } else { 0 };
            t.stub(bp::UPDATE_DEPTH_TRANSFORM, true, 3, 3, err(0));
            t.stub(bp::CREATE_PASS_STATE, true, 0, 0, err(1));
            t.stub(bp::DRAW_SETUP, true, 4, 4, err(2));
            let ebp = t.ebp_for(6);
            let args = [0x44, points_at, colour, indices, n, m];
            t.check_frame(
                &format!("{label}: draw points {k}"),
                0x0059_1dd0,
                Conv::Thiscall(r),
                &args,
                ebp,
                0xa0,
                (0xa0, 0x60),
                |g| {
                    Out::U32(points::draw_points(
                        g, ebp, r, 0x44, points_at, colour, indices, n, m,
                    ))
                },
            );
            // the colour (Map's stub writes only the first word after it)
            let x = Oracle::read(&mut t.a, ebp - 0x20, 0x10).unwrap();
            let y = Oracle::read(&mut t.b, ebp - 0x20, 0x10).unwrap();
            assert_eq!(x, y, "{label}: draw points {k}: colour");
            t.reset();
        }
    }
}

/// `0x00a25410` with the original matrix callees on both sides (the
/// camera setup `0x00a13b90` stubbed): random cameras and effect
/// matrices, a zero projection term (1/0: infinities), a NaN in the
/// camera's 3x4. The frame (every product, the inverse, the scale) is
/// compared after the call. Only synthetic data reaches it.
#[test]
fn bp2_blur_matrices() {
    use ffx_prender::m5p_render_b_bp2::{self as bp, blur_matrices};
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x00a2_5410);
        let mut unit = move || f32::from(u16::try_from(rng.next() % 2000).unwrap()) / 1000.0 - 1.0;
        for k in 0..12u32 {
            let mut cam: Vec<f32> = (0..0x14).map(|_| unit()).collect();
            for c in cam.iter_mut().take(12).step_by(5) {
                *c += 2.0;
            }
            match k {
                1 => cam[12] = 0.0,
                2 => cam[17] = 0.0,
                3 => cam[5] = f32::NAN,
                _ => {}
            }
            let bytes: Vec<u8> = cam.iter().flat_map(|f| f.to_le_bytes()).collect();
            let camera = t.alloc(&bytes);
            let fx: Vec<u8> = (0..0x38)
                .flat_map(|i| if i < 5 { 0 } else { (unit() * 2.0).to_bits() }.to_le_bytes())
                .collect();
            let fx = t.alloc(&fx);
            t.stub(bp::BLUR_MATRICES_CALLS[0], true, 1, 1, 0xa13b);
            let ebp = t.ebp_for(1);
            t.check_frame(
                &format!("{label}: blur matrices {k}"),
                0x00a2_5410,
                Conv::Thiscall(fx),
                &[camera],
                ebp,
                0x1c4,
                (0x1c4, 0x44),
                |g| Out::U32(blur_matrices::blur_matrices(g, ebp, fx, camera)),
            );
            t.reset();
        }
    }
}

/// `0x00a13b90` with every matrix callee original on both sides: random
/// cameras, effect matrices and the 16 slots' current and previous
/// matrices (near-identity, so the inverses exist), a zero projection term
/// (1/0), a NaN in a slot. Every slot's output, the moved matrices and the
/// frame's products are compared. Only synthetic data reaches it.
#[test]
fn bp2_camera_history() {
    use ffx_prender::m5p_render_b_bp2::history::camera_history;
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x00a1_3b90);
        let mut unit = move || f32::from(u16::try_from(rng.next() % 2000).unwrap()) / 1000.0 - 1.0;
        for k in 0..4u32 {
            let mut cam: Vec<f32> = (0..0x14).map(|_| unit()).collect();
            for c in cam.iter_mut().take(12).step_by(5) {
                *c += 2.0;
            }
            if k == 1 {
                cam[17] = 0.0;
            }
            let bytes: Vec<u8> = cam.iter().flat_map(|f| f.to_le_bytes()).collect();
            let camera = t.alloc(&bytes);
            // A, B, then 16 current and 16 previous 4x4s, then the outputs
            let mut fx: Vec<f32> = vec![0.0; (0x8d4 + 0x400) / 4];
            for (i, v) in fx.iter_mut().enumerate().skip(5) {
                let j = (i - 5) % 16;
                *v = if j % 5 == 0 {
                    1.0 + unit() * 0.1
                } else {
                    unit() * 0.3
                };
            }
            if k == 2 {
                fx[(0xd4 + 0x40 * 3 + 8) / 4] = f32::NAN;
            }
            let bytes: Vec<u8> = fx.iter().flat_map(|f| f.to_le_bytes()).collect();
            let fx = t.alloc(&bytes);
            let ebp = t.ebp_for(1);
            t.check_frame(
                &format!("{label}: camera history {k}"),
                0x00a1_3b90,
                Conv::Thiscall(fx),
                &[camera],
                ebp,
                0x30c,
                (0x10c, 0xec),
                |g| Out::U32(camera_history(g, ebp, fx, camera)),
            );
            // the other products (the words between hold the original's
            // pointers and loop count)
            for (from, len) in [
                (0xe4u32, 0x60usize),
                (0x80, 0x40),
                (0x2cc, 0xc0),
                (0x20c, 0x40),
            ] {
                let x = Oracle::read(&mut t.a, ebp - from, len).unwrap();
                let y = Oracle::read(&mut t.b, ebp - from, len).unwrap();
                assert_eq!(x, y, "{label}: camera history {k}: frame at -{from:#x}");
            }
            t.reset();
        }
    }
}

impl Twins {
    /// A recording stub at `sym` (rewritten on every call) that logs its
    /// tag, `ecx` when `this` and `n` stack words, pops `pops` words and
    /// answers from `answers` in turn (0 past the end).
    fn seq_answers(&mut self, sym: u32, this: bool, n: u32, pops: u32, answers: &[u32]) {
        let mut table: Vec<u8> = answers.iter().flat_map(|v| v.to_le_bytes()).collect();
        table.extend([0u8; 64]);
        let table = self.alloc(&table);
        let cell = self.alloc(&table.to_le_bytes());
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x8b, 0x10]); // mov edx, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(0x04); // add dword [cell], 4
        c.extend([0x89, 0xd0]); // mov eax, edx
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// `0x00a1ca10` with every callee stubbed: no mesh instance; each setup
/// parameter absent, of a wrong type, failing or not fitting; each call
/// of each kind failing in turn (the surfaces made so far destroyed, the
/// last first); targets with and without a texture (`+4` set); no final
/// target; the whole pass. Only synthetic data reaches it.
#[test]
#[allow(clippy::type_complexity)] // the case table
fn bp2_dof_render_pass() {
    use ffx_prender::m5p_render_b_bp2::{self as bp};
    // (sym, this, logged words, pops, calls in a full run)
    let kinds: [(u32, bool, u32, u32, usize); 8] = [
        (bp::SET_COLOR_TARGET, true, 2, 2, 6),
        (bp::SET_DEPTH_TARGET, true, 1, 1, 2),
        (bp::BEGIN_SCENE, true, 1, 1, 5),
        (bp::END_SCENE, true, 1, 1, 5),
        (bp::RENDER_MESH_INSTANCE, true, 4, 4, 1),
        (bp::RENDER_FOR_PASS, true, 1, 1, 4),
        (bp::SET_TEXTURE_CHECKED, true, 2, 2, 5),
        (bp::SET_VALUE, true, 2, 2, 5),
    ];
    for (label, mut t) in Twins::all() {
        // (setup variant, failing kind and call, all slots set, final target)
        let mut cases: Vec<(u32, Option<(usize, usize)>, bool, bool)> = vec![
            (0, None, true, true),
            (0, None, false, true),
            (0, None, true, false),
            (1, None, true, true),
            (2, None, true, true),
            (3, None, true, true),
            (4, None, true, true),
            (5, None, true, true),
        ];
        for (ki, &(.., calls)) in kinds.iter().enumerate() {
            for j in 0..calls {
                cases.push((0, Some((ki, j)), true, true));
            }
        }
        for (k, &(setup, fail, all, with_out)) in cases.iter().enumerate() {
            let fx = t.junk(0x1000, k as u8);
            let instance = if setup == 5 { 0 } else { t.junk(0x20, 0x11) };
            t.w32(fx + 0xf38, instance);
            let buffer = t.zeros(0x40);
            t.w32(buffer, 0x30);
            let holder = t.zeros(8);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0xf3c, holder);
            let param = |t: &mut Twins, kind: u8, off: u16, size: u16| {
                let p = t.zeros(0x10);
                t.w8(p + 2, kind);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &size.to_le_bytes());
                p
            };
            let p1 = param(&mut t, if setup == 1 { 0x41 } else { 0x42 }, 8, 0x10);
            let p2 = param(&mut t, 0xd9, if setup == 2 { 2 } else { 0x18 }, 0x10);
            let p3 = param(&mut t, 0x42, 4, if setup == 3 { 0x30 } else { 4 });
            t.w32(fx + 0xf88, p1);
            t.w32(fx + 0xf94, p2);
            for (i, slot) in [0xfb4u32, 0xfb8, 0xfbc].into_iter().enumerate() {
                let p = if i == 0 {
                    p3
                } else {
                    param(&mut t, 0x42, 0x24 + 4 * i as u16, 4)
                };
                t.w32(fx + slot, p);
            }
            for slot in [0xf8cu32, 0xf90, 0xf98, 0xf9c, 0xfa0, 0xfa4, 0xfa8, 0xfac] {
                let v = if all || slot % 8 == 0 {
                    t.junk(0x10, slot as u8)
                } else {
                    0
                };
                t.w32(fx + slot, v);
            }
            for (i, slot) in [0xfb0u32, 0xf2c, 0xf30, 0xf34].into_iter().enumerate() {
                let target = t.junk(0x10, 0x20 + i as u8);
                t.w8(target + 4, u8::from(i % 2 == 1));
                t.w32(fx + slot, target);
            }
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            t.stub(
                bp::TROPHY_INSTALL,
                true,
                1,
                1,
                if setup == 4 { 0x77 } else { 0 },
            );
            t.stub(bp::MEMCPY, false, 3, 0, 0x3c);
            t.stub(bp::SURFACE_CTOR, true, 2, 2, 0x5c);
            t.stub(bp::SURFACE_DTOR, true, 0, 0, 0x5d);
            for (ki, &(sym, this, n, pops, calls)) in kinds.iter().enumerate() {
                let mut answers = vec![0u32; calls];
                if let Some((_, fj)) = fail.filter(|&(fk, _)| fk == ki) {
                    answers[fj] = 0x30 + ki as u32;
                }
                // the last colour-target call answers its own value
                if ki == 0 {
                    answers[5] |= 0x0b00;
                }
                if sym == bp::SET_TEXTURE_CHECKED {
                    t.binding_stub(sym, &answers);
                } else {
                    t.seq_answers(sym, this, n, pops, &answers);
                }
            }
            let out = if with_out { 0x5a5a } else { 0 };
            let ebp = t.ebp_for(5);
            let args = [renderer, 0xaa, 0x7e01, 0x7e02, out];
            t.check_frame(
                &format!("{label}: dof pass {k}"),
                0x00a1_ca10,
                Conv::Thiscall(fx),
                &args,
                ebp,
                0x8c,
                (0x3c, 0xc),
                |g| {
                    Out::U32(bp::dof::dof_render_pass(
                        g, ebp, fx, renderer, 0xaa, 0x7e01, 0x7e02, out,
                    ))
                },
            );
            t.reset();
        }
    }
}

/// `0x00a220b0` with every callee stubbed: no mesh instance; each setup
/// parameter absent, of a wrong type, failing, not fitting; each call of
/// each kind failing in turn (the surfaces made destroyed, the last
/// first); targets with and without a texture; no final target; the
/// whole pass. Only synthetic data reaches it.
#[test]
#[allow(clippy::type_complexity)] // the case table
fn bp2_ssr_render_pass() {
    use ffx_prender::m5p_render_b_bp2::{self as bp};
    // (sym, this, logged words, pops, calls in a full run)
    let kinds: [(u32, bool, u32, u32, usize); 8] = [
        (bp::SET_COLOR_TARGET, true, 2, 2, 8),
        (bp::SET_DEPTH_TARGET, true, 1, 1, 4),
        (bp::BEGIN_SCENE, true, 1, 1, 4),
        (bp::END_SCENE, true, 1, 1, 4),
        (bp::RENDER_MESH_INSTANCE, true, 4, 4, 1),
        (bp::RENDER_FOR_PASS, true, 1, 1, 3),
        (bp::SET_TEXTURE_CHECKED, true, 2, 2, 5),
        (bp::WRITE_PARAMETER, true, 2, 2, 1),
    ];
    for (label, mut t) in Twins::all() {
        // (setup variant, failing kind and call, all slots set, final target)
        let mut cases: Vec<(u32, Option<(usize, usize)>, bool, bool)> = vec![
            (0, None, true, true),
            (0, None, false, true),
            (0, None, true, false),
            (1, None, true, true),
            (2, None, true, true),
            (3, None, true, true),
            (4, None, true, true),
            (5, None, true, true),
            (6, None, true, true),
            (7, None, true, true),
        ];
        for (ki, &(.., calls)) in kinds.iter().enumerate() {
            for j in 0..calls {
                cases.push((0, Some((ki, j)), true, true));
            }
        }
        for (k, &(setup, fail, all, with_out)) in cases.iter().enumerate() {
            let fx = t.junk(0x100, k as u8);
            let instance = if setup == 5 { 0 } else { t.junk(0x20, 0x11) };
            t.w32(fx + 0x1c, instance);
            let buffer = t.zeros(0x40);
            t.w32(buffer, 0x30);
            let holder = t.zeros(8);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0x74, holder);
            let param = |t: &mut Twins, kind: u8, off: u16, size: u16| {
                let p = t.zeros(0x10);
                t.w8(p + 2, kind);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &size.to_le_bytes());
                p
            };
            let p1 = param(&mut t, if setup == 1 { 0x41 } else { 0x42 }, 8, 0x10);
            let p2 = param(&mut t, 0xd9, if setup == 2 { 2 } else { 0x18 }, 0x10);
            let p3 = param(&mut t, if setup == 6 { 0xdd } else { 0x12 }, 0, 0);
            let p4 = param(&mut t, 0x42, 4, if setup == 3 { 0x30 } else { 4 });
            t.w32(fx + 0x78, p1);
            t.w32(fx + 0x7c, p2);
            t.w32(fx + 0x84, p3);
            t.w32(fx + 0x8c, p4);
            let p5 = param(&mut t, 0x42, 0x28, if setup == 7 { 0xe010 } else { 4 });
            t.w32(fx + 0x90, p5);
            for slot in [0x80u32, 0x88] {
                let v = if all { t.junk(0x10, slot as u8) } else { 0 };
                t.w32(fx + slot, v);
            }
            for (i, slot) in [0x68u32, 0x6c, 0x70].into_iter().enumerate() {
                let target = t.junk(0x10, 0x20 + i as u8);
                t.w8(target + 4, u8::from(i == 1));
                t.w32(fx + slot, target);
            }
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            t.stub(
                bp::TROPHY_INSTALL,
                true,
                1,
                1,
                if setup == 4 { 0x77 } else { 0 },
            );
            t.stub(bp::MEMCPY, false, 3, 0, 0x3c);
            t.stub(bp::SURFACE_CTOR, true, 2, 2, 0x5c);
            t.stub(bp::SURFACE_DTOR, true, 0, 0, 0x5d);
            for (ki, &(sym, this, n, pops, calls)) in kinds.iter().enumerate() {
                let mut answers = vec![0u32; calls];
                if let Some((_, fj)) = fail.filter(|&(fk, _)| fk == ki) {
                    answers[fj] = 0x30 + ki as u32;
                }
                if ki == 0 {
                    answers[7] |= 0x0b00;
                }
                if sym == bp::SET_TEXTURE_CHECKED {
                    t.binding_stub(sym, &answers);
                } else {
                    t.seq_answers(sym, this, n, pops, &answers);
                }
            }
            let out = if with_out { 0x5a5a } else { 0 };
            let ebp = t.ebp_for(5);
            let args = [renderer, out, 0x7e01, 0x7e02, 0x7e03];
            t.check_frame(
                &format!("{label}: ssr pass {k}"),
                0x00a2_20b0,
                Conv::Thiscall(fx),
                &args,
                ebp,
                0x7c,
                (0x3c, 0xc),
                |g| {
                    Out::U32(bp::ssr::ssr_render_pass(
                        g, ebp, fx, renderer, out, 0x7e01, 0x7e02, 0x7e03,
                    ))
                },
            );
            t.reset();
        }
    }
}

impl Twins {
    /// A recording stub at `sym` (thiscall, two stack words, popping
    /// them) that logs its tag, `ecx`, its first word and the four words
    /// its second word points at (a texture binding), answering from
    /// `answers` in turn (0 past the end).
    fn binding_stub(&mut self, sym: u32, answers: &[u32]) {
        let mut table: Vec<u8> = answers.iter().flat_map(|v| v.to_le_bytes()).collect();
        table.extend([0u8; 64]);
        let table = self.alloc(&table);
        let cell = self.alloc(&table.to_le_bytes());
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        for k in 0..4u8 {
            c.extend([0xff, 0x72, 4 * k]); // push dword [edx+4k]
            c.extend([0x8f, 0x40, 0x0c + 4 * k]); // pop dword [eax+0xc+4k]
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x8b, 0x10]); // mov edx, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(0x04); // add dword [cell], 4
        c.extend([0x89, 0xd0]); // mov eax, edx
        c.extend([0xc2, 0x08, 0x00]); // ret 8
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

// ---- big-phyre3 ----

impl Twins {
    /// A recording `memcpy` stub (cdecl, three words, nothing copied):
    /// logs its tag, the destination, the source, the count and the
    /// first four words at the source (a local that later code may
    /// overwrite; 0 for the words past the count).
    fn copy_stub(&mut self, sym: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        for k in 0..3u8 {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp+4+4k]
            c.extend([0x89, 0x50, 4 + 4 * k]); // mov [eax+4+4k], edx
        }
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        c.extend([0x8b, 0x4c, 0x24, 0x0c]); // mov ecx, [esp+0xc]
        for k in 0..4u8 {
            c.extend([0xc7, 0x40, 0x10 + 4 * k, 0, 0, 0, 0]); // mov dword [eax+0x10+4k], 0
            c.extend([0x83, 0xf9, 4 * k]); // cmp ecx, 4k
            c.extend([0x76, 0x06]); // jbe past the copy
            c.extend([0xff, 0x72, 4 * k]); // push dword [edx+4k]
            c.extend([0x8f, 0x40, 0x10 + 4 * k]); // pop dword [eax+0x10+4k]
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
        c.push(0xc3);
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// `0x00a1d2b0` (the glow's level passes) with every callee stubbed:
/// level counts 0 to 3 and past 3 (unsigned); each setup parameter
/// absent, of a wrong type, failing or not fitting; each loop parameter
/// absent, of a wrong type or not fitting; targets whose texture flag
/// (`+4`) is set; with and without a bloom texture, an output target
/// and the flag (only its low byte read); sizes past 2^31 and past
/// 2^24 (rounded); each call of each kind failing in turn. Only
/// synthetic data reaches it. Not reached: `effect + 0x14`, `+ 0x38`
/// or `+ 0x6c`/`+ 0x70` being 0 (the effect at the top of the address
/// space).
#[test]
#[allow(clippy::type_complexity)] // the case table
fn bp3_glow_render_passes() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    // (sym, this, logged words, pops, calls in a full run)
    let kinds: [(u32, bool, u32, u32, usize); 9] = [
        (bp::SET_COLOR_TARGET, true, 2, 2, 12),
        (bp::SET_DEPTH_TARGET, true, 1, 1, 12),
        (bp::BEGIN_SCENE, true, 1, 1, 12),
        (bp::END_SCENE, true, 1, 1, 12),
        (bp::RENDER_MESH_INSTANCE, true, 4, 4, 9),
        (bp::RENDER_FOR_PASS, true, 1, 1, 3),
        (bp::SET_TEXTURE_CHECKED, true, 2, 2, 6),
        (bp::SET_VALUE, true, 2, 2, 3),
        (bp::TROPHY_INSTALL, true, 1, 1, 7),
    ];
    // (variant, levels, failing kind and call, bloom, out, flag)
    let mut cases: Vec<(u32, u32, Option<(usize, usize)>, u32, u32, u32)> = vec![
        (0, 3, None, 0x7e02, 0x5a5a, 1),
        (0, 0, None, 0x7e02, 0x5a5a, 1),
        (0, 1, None, 0x7e02, 0x5a5a, 1),
        (0, 2, None, 0, 0x5a5a, 0),
        (0, 5, None, 0x7e02, 0, 0x100),
        (0, 0x8000_0001, None, 0, 0, 0x101),
    ];
    for v in 1..=14 {
        cases.push((v, 3, None, 0x7e02, 0x5a5a, 1));
    }
    for (ki, &(.., calls)) in kinds.iter().enumerate() {
        for j in 0..calls {
            cases.push((0, 3, Some((ki, j)), 0x7e02, 0x5a5a, 0));
        }
    }
    for (label, mut t) in Twins::all() {
        for (k, &(var, levels, fail, bloom, out, flag)) in cases.iter().enumerate() {
            let fx = t.junk(0xb0, k as u8);
            let instance = t.junk(0x20, 0x11);
            t.w32(fx + 0x5c, instance);
            let buffer = t.zeros(0x40);
            t.w32(buffer, 0x30);
            let holder = t.zeros(8);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0x60, holder);
            t.w32(fx + 0x64, levels);
            t.w32(fx + 0x68, 0.6875f32.to_bits());
            let param = |t: &mut Twins, kind: u8, off: u16, size: u16| {
                let p = t.zeros(0x10);
                t.w8(p + 2, kind);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &size.to_le_bytes());
                p
            };
            // the setup: +0x78 (texture), +0x88 and +0x8c (bytes)
            let p78 = match var {
                1 => 0,
                2 => param(&mut t, 0xdd, 8, 0x10),
                3 => param(&mut t, 0xdc, 2, 0x10),
                4 => param(&mut t, 0x12, 0x28, 0x10),
                _ => param(&mut t, if var % 2 == 0 { 0xd9 } else { 0xda }, 8, 0x10),
            };
            let p88 = match var {
                5 => 0,
                6 => param(&mut t, 0x42, 0x2c, 0x8),
                _ => param(&mut t, 0x42, 0x18, 4),
            };
            let p8c = match var {
                7 => param(&mut t, 0x42, 3, 4),
                _ => param(&mut t, 0x42, 0x1c, 4),
            };
            // the loops: +0x74 (texture), +0x7c (scale), +0x80 (size)
            let p74 = match var {
                8 => 0,
                9 => param(&mut t, 0x41, 0x20, 0x10),
                10 => param(&mut t, 0xdb, 0x20, 0xe008),
                _ => param(&mut t, 0x42, 0x20, 0x10),
            };
            let p7c = match var {
                11 => 0,
                12 => param(&mut t, 0x42, 2, 0x10),
                _ => param(&mut t, 0x42, 0x10, 0x10),
            };
            let p80 = match var {
                13 => 0,
                14 => param(&mut t, 0x42, 0x28, 0x10),
                _ => param(&mut t, 0x42, 0x14, 8),
            };
            for (slot, p) in [
                (0x78u32, p78),
                (0x88, p88),
                (0x8c, p8c),
                (0x74, p74),
                (0x7c, p7c),
                (0x80, p80),
            ] {
                t.w32(fx + slot, p);
            }
            let sizes = [
                (0x500u32, 0x2d0u32),
                (0x8000_0101, 0x0100_0003),
                (0xffff_ffff, 7),
            ];
            for i in 0..3u32 {
                let texture = t.junk(0x30, 0x60 + i as u8);
                t.w32(texture + 0x1c, sizes[i as usize].0);
                t.w32(texture + 0x20, sizes[i as usize].1);
                let first = t.junk(0x10, 0x20 + i as u8);
                t.w32(first, texture);
                t.w8(first + 4, 0);
                t.w32(fx + 0x90 + 4 * i, first);
                let second = t.junk(0x10, 0x30 + i as u8);
                // a second target with its flag set gives no texture,
                // except in the failure runs (every call made)
                t.w8(second + 4, if i == 1 && fail.is_none() { 2 } else { 0 });
                t.w32(fx + 0x9c + 4 * i, second);
            }
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            t.copy_stub(bp::MEMCPY);
            t.stub(bp::SURFACE_CTOR, true, 2, 2, 0x5c);
            t.stub(bp::SURFACE_DTOR, true, 0, 0, 0x5d);
            t.stub(bp::QUEUE_COMMAND_18, true, 3, 3, 0xd);
            for (ki, &(sym, this, n, pops, calls)) in kinds.iter().enumerate() {
                let mut answers = vec![0u32; calls];
                if let Some((_, fj)) = fail.filter(|&(fk, _)| fk == ki) {
                    answers[fj] = 0x30 + ki as u32;
                }
                if sym == bp::TROPHY_INSTALL && var == 4 {
                    answers[0] = 0x77;
                }
                if sym == bp::SET_TEXTURE_CHECKED || sym == bp::SET_VALUE {
                    t.binding_stub(sym, &answers);
                } else {
                    t.seq_answers(sym, this, n, pops, &answers);
                }
            }
            let ebp = t.ebp_for(5);
            let args = [renderer, out, 0x7e01, bloom, flag];
            t.check_frame(
                &format!("{label}: glow passes {k}"),
                0x00a1_d2b0,
                Conv::Thiscall(fx),
                &args,
                ebp,
                0xf0,
                (0xf0, 0x20),
                |g| {
                    Out::U32(bp::glow::glow_render_passes(
                        g, ebp, fx, renderer, out, 0x7e01, bloom, flag,
                    ))
                },
            );
            assert!(t.log_entries() > 0 || var == 2, "{label}: {k}: no calls");
            t.reset();
        }
    }
}

impl Twins {
    /// `bytes` written over the original at `sym` in both (put back by
    /// [`Twins::reset`]).
    fn patch(&mut self, sym: u32, bytes: &[u8]) {
        let at = self.rebase(sym);
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, bytes);
    }

    /// The intrinsic trampoline in both oracles' code, for [`G`].
    fn trampoline(&mut self) {
        let at = self.a.alloc_code(&INTRINSIC_TRAMPOLINE).unwrap();
        assert_eq!(self.b.alloc_code(&INTRINSIC_TRAMPOLINE).unwrap(), at);
        TRAMP.with(|c| c.set(at));
    }

    /// A `PConstantBufferRing_next` stub (thiscall, one word, popped):
    /// logs its tag, `ecx` and the word, stores the ring's `+0x20` (the
    /// mapped buffer) through the word and returns the ring's `+0x24`.
    fn ring_stub(&mut self, sym: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x8b, 0x41, 0x20]); // mov eax, [ecx+0x20]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.extend([0x8b, 0x41, 0x24]); // mov eax, [ecx+0x24]
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// A recording stub logging all `n` stack words (six per entry, `ecx`
    /// in each), popping `pops`, storing `put` through stack word `arg`
    /// when given, answering from `answers` in turn (0 past the end).
    fn seq_wide(&mut self, sym: u32, n: u32, pops: u32, put: Option<(u32, u32)>, answers: &[u32]) {
        let mut table: Vec<u8> = answers.iter().flat_map(|v| v.to_le_bytes()).collect();
        table.extend([0u8; 64]);
        let table = self.alloc(&table);
        let cell = self.alloc(&table.to_le_bytes());
        let log = self.log;
        let mut c = Vec::new();
        let mut i = 0;
        while i < n {
            c.push(0xa1);
            c.extend(log.to_le_bytes()); // mov eax, [log]
            c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
            c.push(0x05);
            c.extend((log + 4).to_le_bytes()); // add eax, log + 4
            c.extend([0xc7, 0x00]);
            c.extend((sym ^ i).to_le_bytes()); // mov [eax], tag
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
            for k in 0..(n - i).min(6) {
                c.extend([0x8b, 0x54, 0x24, (4 + 4 * (i + k)) as u8]); // mov edx, [esp+4+4w]
                c.extend([0x89, 0x50, (8 + 4 * k) as u8]); // mov [eax+8+4k], edx
            }
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [log]
            i += 6;
        }
        if let Some((arg, v)) = put {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * arg) as u8]); // mov edx, [esp+4+4arg]
            c.extend([0xc7, 0x02]);
            c.extend(v.to_le_bytes()); // mov dword [edx], v
        }
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend([0x8b, 0x10]); // mov edx, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(0x04); // add dword [cell], 4
        c.extend([0x89, 0xd0]); // mov eax, edx
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    fn wf(&mut self, at: u32, v: f32) {
        self.w32(at, v.to_bits());
    }
}

/// `0x00a127c0` (the deferred lighting's light buffers): the math
/// callees run as the originals (`_CIsqrt`, `_CItan`, `_CIcos` patched to
/// `fsqrt`, `fptan`, `fcos` in both), the rings, slots, copies, parameter
/// writes and the pass recorded. Cases: the main light on through each
/// colour channel or off (a channel exactly at the threshold), records of
/// each kind with and without their channel, no records; every ring and
/// slot call failing; slot wrap-around and a zero count; each parameter
/// absent, of a wrong type, not fitting; textures without flag 8 and
/// none; the pass and both parameter writes failing. Only synthetic data
/// reaches it. Not reached: `effect + 0x3e90` being 0 (an effect at the
/// top of the address space), a channel index past 3 (the original
/// overwrites its own frame record and ends).
#[test]
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::similar_names
)] // the case data
fn bp3_upload_lights() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    let [_, next, _, _, _, _, _, mark, write_parameter, queue_pass] = bp::LIGHT_CALLS;
    // (variant, failing (sym, call))
    let mut cases: Vec<(u32, Option<(u32, usize)>)> = (0..=36).map(|v| (v, None)).collect();
    for (sym, calls) in [(next, 2), (mark, 2), (write_parameter, 2), (queue_pass, 1)] {
        for j in 0..calls {
            cases.push((0, Some((sym, j))));
        }
    }
    for (label, mut t) in Twins::all() {
        t.trampoline();
        for (k, &(var, fail)) in cases.iter().enumerate() {
            let fx = t.zeros(0x4040);
            // the main light: direction, colour, channel
            for (i, v) in [0.25f32, -0.5, 0.75].into_iter().enumerate() {
                t.wf(fx + 0x3e24 + 4 * i as u32, v);
            }
            let low = f32::from_bits(0x38d1_b717); // 1e-4: not above the threshold
            let colour = match var {
                1 => [low, low, low, 0.5],
                2 => [low, 0.3, low, 0.5],
                3 => [0.0, -1.0, 0.2, 0.5],
                _ => [0.6, 0.0, 0.0, 0.25],
            };
            for (i, v) in colour.into_iter().enumerate() {
                t.wf(fx + 0x3e44 + 4 * i as u32, v);
            }
            t.w32(fx + 0x3e78, u32::from(var != 4));
            t.w32(fx + 0x3e7c, if var == 5 { 3 } else { 1 });
            // the records: (type, channel on, channel)
            let kinds: &[(u32, bool, u32)] = match var {
                6 => &[],
                7 => &[(1, true, 0), (1, false, 2)],
                31..=36 => &[(1, true, 0); 16],
                8 => &[(0, true, 2), (2, false, 0), (3, true, 3)],
                _ => &[
                    (1, true, 2),
                    (2, true, 1),
                    (1, true, 3),
                    (0, false, 0),
                    (5, true, 0),
                ],
            };
            t.w32(fx + 0x3ed0, kinds.len() as u32);
            for (i, &(kind, on, ch)) in kinds.iter().enumerate() {
                let rec = fx + 0x3c + 0x7c * i as u32;
                for w in 0..20u32 {
                    let v = if var >= 31 {
                        // pseudo-random magnitudes for the roundings
                        let mut r = Rng(0x9e37_79b9 ^ (var * 0x100 + i as u32 * 20 + w));
                        r.next();
                        f32::from_bits(0x3e00_0000 + (r.next() & 0x01ff_ffff))
                            * if r.next() & 1 == 0 { 1.0 } else { -1.0 }
                    } else {
                        0.15 * (w as f32) - 1.1 + 0.37 * i as f32
                    };
                    t.wf(rec - 0x28 + 4 * w, v);
                }
                t.w32(rec + 0x28, kind);
                t.wf(rec + 0x2c, 12.5 + i as f32);
                t.wf(rec + 0x30, 3.25);
                t.wf(rec + 0x34, 0.35 + 0.05 * i as f32);
                t.wf(rec + 0x38, 0.6 + 0.07 * i as f32);
                t.w32(rec + 0x3c, u32::from(on));
                t.w32(rec + 0x40, ch);
                t.wf(rec + 0x4c, 0.125 * i as f32);
            }
            // the rings: +4 slot, +0xc count, +0x20 the mapped buffer,
            // +0x24 the stub's error
            let rings: Vec<u32> = (0..2)
                .map(|r| {
                    let ring = t.zeros(0x30);
                    let buf = t.junk(0x400, 0x70 + r as u8);
                    t.w32(ring + 0x20, buf);
                    ring
                })
                .collect();
            let (cur, count) = match var {
                9 => (2, 3),
                10 => (0, 0),
                11 => (5, 0x8000_0007),
                12 => (0, 0x8000_0000),
                31 => (9, 4),
                _ => (1, 4),
            };
            for &ring in &rings {
                t.w32(ring + 4, cur);
                t.w32(ring + 0xc, count);
                t.w8(ring + 8, 1);
            }
            t.w32(fx + 0x3fcc, rings[0]);
            t.w32(fx + 0x3fc8, rings[1]);
            if let Some((sym, j)) = fail.filter(|&(s, _)| s == next) {
                let _ = sym;
                t.w32(rings[j] + 0x24, 0x31 + j as u32);
            }
            // the parameters
            let buffer = t.zeros(0x80);
            t.w32(buffer, 0x60);
            let holder = t.zeros(8);
            t.w32(holder, 0x6a7e);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0x3ed8, holder);
            let param = |t: &mut Twins, kind: u8, off: u16, size: u16| {
                let p = t.zeros(0x10);
                t.w8(p + 2, kind);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &size.to_le_bytes());
                p
            };
            let ring_kind = |n: u32| match n {
                0 => 0x4a,
                1 => 0x13,
                2 => 0x90,
                _ => 0x91,
            };
            let p4000 = match var {
                13 => 0,
                14 => param(&mut t, 0x92, 8, 0x10),
                15 => param(&mut t, 0x4a, 0x58, 0x10),
                _ => param(&mut t, ring_kind(var % 4), 8, 0x10),
            };
            let p4004 = match var {
                16 => 0,
                17 => param(&mut t, 0x12, 8, 0x10),
                18 => param(&mut t, 0x91, 2, 0x10),
                _ => param(&mut t, ring_kind((var + 1) % 4), 0x18, 0x10),
            };
            let p4008 = match var {
                19 => 0,
                20 => param(&mut t, 0, 3, 4),
                32 => param(&mut t, 0, 0x28, 0xe004),
                _ => param(&mut t, 0, 0x28, 4),
            };
            let p400c = match var {
                21 => param(&mut t, 0, 0x5e, 4),
                33 => param(&mut t, 0, 0x5c, 4),
                _ => param(&mut t, 0, 0x2c, 4),
            };
            let p3ffc = match var {
                22 => 0,
                23 => param(&mut t, 0, 0x30, 0xe040),
                _ => param(&mut t, 0, 0x30, 0x10),
            };
            let p4014 = match var {
                24 => param(&mut t, 0x48, 0x40, 0x14),
                25 => param(&mut t, 0x15, 0x50, 0x14),
                _ => param(&mut t, if var % 2 == 0 { 0x49 } else { 0x14 }, 0x40, 0x14),
            };
            let p4028 = match var {
                26 => 0,
                27 => param(&mut t, 0x4a, 0x40, 0x14),
                _ => param(&mut t, 0x15, 0x40, 0x14),
            };
            let p4010 = match var {
                28 => 0,
                29 => param(&mut t, 0x49, 0x40, 0x14),
                34 => param(&mut t, 0x13, 0x40, 0x14),
                _ => param(&mut t, 0x48, 0x40, 0x14),
            };
            for (slot, p) in [
                (0x4000u32, p4000),
                (0x4004, p4004),
                (0x4008, p4008),
                (0x400c, p400c),
                (0x3ffc, p3ffc),
                (0x4014, p4014),
                (0x4028, p4028),
                (0x4010, p4010),
            ] {
                t.w32(fx + slot, p);
            }
            for (i, slot) in [0x3fd0u32, 0x3fd4].into_iter().enumerate() {
                let tex = match (var, i) {
                    (30, 0) => 0,
                    _ => t.junk(0x20, 0x50 + i as u8),
                };
                if tex != 0 {
                    // the shadow texture without flag 8 in variant 25
                    t.w8(tex + 8, if var == 25 && i == 0 { 0xf7 } else { 0x0c });
                }
                t.w32(fx + slot, tex);
            }
            let tex = t.junk(0x30, 0x44);
            t.w32(tex + 0x1c, if var == 3 { 0x500 } else { 0x501 });
            t.w32(tex + 0x20, 0x2d7);
            let target = t.junk(0x10, 0x45);
            t.w32(target, tex);
            t.w8(target + 4, 0);
            t.w32(fx + 0x3fc4, target);
            for i in 0..4u32 {
                t.wf(fx + 0x3e90 + 4 * i, 0.5 * i as f32);
            }
            let view = t.zeros(0x30);
            for w in 0..12u32 {
                let v = match w % 4 {
                    0 => 0.8,
                    1 => -0.6,
                    2 => 0.1 * w as f32,
                    _ => 2.5,
                };
                t.wf(view + 4 * w, v);
            }
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            // the callees
            t.patch(0x0094_97b8, &[0xd9, 0xfa, 0xc3]); // _CIsqrt: fsqrt
            t.patch(0x0094_97be, &[0xd9, 0xf2, 0xdd, 0xd8, 0xc3]); // _CItan: fptan
            t.patch(0x0094_98c2, &[0xd9, 0xff, 0xc3]); // _CIcos: fcos
            t.ring_stub(next);
            t.copy_stub(bp::MEMCPY);
            let answers = |sym: u32, n: usize| {
                let mut a = vec![0u32; n];
                if let Some((_, j)) = fail.filter(|&(s, _)| s == sym) {
                    a[j] = 0x40 + j as u32;
                }
                a
            };
            let a = answers(mark, 2);
            t.seq_answers(mark, true, 2, 2, &a);
            let a = answers(write_parameter, 2);
            t.binding_stub(write_parameter, &a);
            let a = answers(queue_pass, 1);
            t.seq_answers(queue_pass, true, 5, 5, &a);
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: lights {k}"),
                0x00a1_27c0,
                Conv::Thiscall(fx),
                &[renderer, view],
                ebp,
                0x1f4,
                (0x1e4, 0x34),
                |g| Out::U32(bp::lights::upload_lights(g, ebp, fx, renderer, view)),
            );
            t.reset();
        }
        TRAMP.with(|c| c.set(0));
    }
}

/// `0x00a1f220` (the deferred lighting's render): the effect not set up
/// (each condition), `out` 0 (the effect's target), each parameter absent,
/// targets without a texture (`+4` set), both paths (`+0x402c`: the light
/// constants and texture parameters, results dropped; else the light
/// buffer with 0 and 3 light records), each call of each kind failing in
/// turn (the surfaces made destroyed, the last first). The matrix and
/// `maxFloat` callees run as the originals. Only synthetic data reaches
/// it.
#[test]
#[allow(
    clippy::type_complexity,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)] // the case table
fn bp3_lighting_render() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    let [
        _,
        _,
        _,
        set_value2,
        upload,
        texture_params,
        clear_colour,
        draw_light,
    ] = bp::LIGHTING_CALLS;
    // (sym, this, logged words, pops, calls in a full run)
    let kinds: [(u32, bool, u32, u32, usize); 11] = [
        (bp::SET_TEXTURE_CHECKED, true, 2, 2, 6),
        (bp::SET_VALUE, true, 2, 2, 3),
        (set_value2, true, 2, 2, 2),
        (bp::SET_COLOR_TARGET, true, 2, 2, 4),
        (bp::SET_DEPTH_TARGET, true, 1, 1, 3),
        (bp::BEGIN_SCENE, true, 1, 1, 2),
        (bp::END_SCENE, true, 1, 1, 2),
        (bp::RENDER_FOR_PASS, true, 1, 1, 1),
        (clear_colour, true, 4, 4, 1),
        (draw_light, true, 2, 2, 4),
        (upload, true, 2, 2, 1),
    ];
    // (variant, the +0x402c path, failing kind and call)
    let mut cases: Vec<(u32, bool, Option<(usize, usize)>)> = Vec::new();
    for v in 0..=15 {
        cases.push((v, false, None));
        cases.push((v, true, None));
    }
    for (ki, &(.., calls)) in kinds.iter().enumerate() {
        for j in 0..calls {
            cases.push((0, false, Some((ki, j))));
            cases.push((0, true, Some((ki, j))));
        }
    }
    for (label, mut t) in Twins::all() {
        for (k, &(var, direct, fail)) in cases.iter().enumerate() {
            let fx = t.zeros(0x4040);
            t.w32(fx + 0x3ed4, if var == 1 { 2 } else { 3 });
            for (i, slot) in [0x3fb0u32, 0x3fb4, 0x3fb8, 0x3fbc, 0x3fc0, 0x3fc4, 0x4030]
                .into_iter()
                .enumerate()
            {
                let target = t.junk(0x10, 0x20 + i as u8);
                // targets 0x3fb8 and 0x4030 without a texture in variant 8
                let flagged = var == 8 && (slot == 0x3fb8 || slot == 0x4030);
                t.w8(target + 4, u8::from(flagged));
                let zero = matches!((var, slot), (2, 0x3fbc) | (3, 0x3fb0) | (4, 0x3fc0))
                    || matches!((var, slot), (5, 0x3fb8) | (6, 0x3fc4));
                t.w32(fx + slot, if zero { 0 } else { target });
            }
            let holder = t.zeros(8);
            let buffer = t.zeros(0x40);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0x3ed8, holder);
            t.w32(fx + 0x3edc, 0x1e57);
            for (i, slot) in [
                0x3fd8u32, 0x3f58, 0x3f54, 0x3fdc, 0x3fe0, 0x3f88, 0x3f8c, 0x3f84, 0x4018, 0x401c,
                0x4020,
            ]
            .into_iter()
            .enumerate()
            {
                // variant 9 leaves the even ones out, 10 the odd ones
                let absent = (var == 9 && i % 2 == 0) || (var == 10 && i % 2 == 1);
                t.w32(fx + slot, if absent { 0 } else { 0x5a00 + i as u32 });
            }
            let (near, far): (f32, f32) = match var {
                11 => (5.0, 5.00001),
                12 => (2.0, -3.0),
                7 => (1.0, f32::NAN),
                _ => (0.25, 400.0),
            };
            t.w32(fx + 0x3eb0, far.to_bits());
            t.w32(fx + 0x3eb4, near.to_bits());
            t.w32(fx + 0x3eb8, 0.75f32.to_bits());
            for i in 0..4u32 {
                t.w32(fx + 0x3e90 + 4 * i, (0.2f32 * i as f32 + 0.1).to_bits());
            }
            t.w8(fx + 0x402c, u8::from(direct));
            t.w32(fx + 0x3ed0, if var == 13 { 0 } else { 3 });
            let view = t.zeros(0x50);
            for w in 0..12u32 {
                t.w32(view + 4 * w, (0.3f32 * w as f32 - 1.0).to_bits());
            }
            t.w32(view + 0x30, if var == 14 { 0 } else { 1.7f32.to_bits() });
            t.w32(view + 0x44, (-2.5f32).to_bits());
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            t.stub(bp::SURFACE_CTOR, true, 2, 2, 0x5c);
            t.stub(bp::SURFACE_DTOR, true, 0, 0, 0x5d);
            t.stub(texture_params, true, 2, 2, 0x66);
            for (ki, &(sym, this, n, pops, calls)) in kinds.iter().enumerate() {
                let mut answers = vec![0u32; calls];
                if let Some((_, fj)) = fail.filter(|&(fk, _)| fk == ki) {
                    answers[fj] = 0x30 + ki as u32;
                }
                if sym == upload {
                    // dropped by the original
                    answers[0] |= 0x700;
                }
                if sym == bp::SET_TEXTURE_CHECKED || sym == bp::SET_VALUE || sym == set_value2 {
                    t.binding_stub(sym, &answers);
                } else {
                    t.seq_answers(sym, this, n, pops, &answers);
                }
            }
            let out = if var == 15 { 0 } else { 0x0a7e };
            let ebp = t.ebp_for(3);
            t.check_frame(
                &format!("{label}: lighting {k}"),
                0x00a1_f220,
                Conv::Thiscall(fx),
                &[renderer, view, out],
                ebp,
                0x100,
                (0xf0, 0x28),
                |g| Out::U32(bp::lighting::render(g, ebp, fx, renderer, view, out)),
            );
            t.reset();
        }
    }
}

/// `0x00a206d0` (the deferred lighting's light drawer): the matrix
/// callees run as the originals (`_CItan`/`_CIcos` patched to
/// `fptan`/`fcos` in both), the copies, `clearScene` and
/// `renderMeshInstance` recorded. Cases: each parameter absent or not
/// fitting, light types 0, 1, 2 and another, the channel on and off,
/// `renderMeshInstance` failing for each type. The never-written word at
/// `ebp - 8` (the direction's fourth word) is 0 in both (brief,
/// 2026-10-24). Only synthetic data reaches it. Not reached: a light at
/// `-0x30` (its words at address 0).
#[test]
#[allow(clippy::cast_precision_loss)] // the case data
fn bp3_draw_light() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    let clear_scene = bp::LIGHT_DRAW_CALLS[3];
    // (variant, light type, the mesh failing)
    let mut cases: Vec<(u32, u32, bool)> = Vec::new();
    for v in 0..=17 {
        cases.push((v, 2, false));
    }
    for ty in [0, 1, 2, 3, 7] {
        cases.push((0, ty, false));
        cases.push((0, ty, true));
    }
    for (label, mut t) in Twins::all() {
        t.trampoline();
        for (k, &(var, ty, fail)) in cases.iter().enumerate() {
            let fx = t.zeros(0x4040);
            let holder = t.zeros(8);
            let buffer = t.zeros(0x100);
            t.w32(buffer, 0xc0);
            t.w32(holder + 4, buffer);
            t.w32(fx + 0x3ed8, holder);
            for (i, slot) in [0x3edcu32, 0x3ee0, 0x3ee4].into_iter().enumerate() {
                let inst = t.junk(0x20, 0x30 + i as u8);
                t.w32(fx + slot, inst);
            }
            let param = |t: &mut Twins, off: u16, size: u16| {
                let p = t.zeros(0x10);
                t.write(p + 8, &off.to_le_bytes());
                t.write(p + 0xa, &size.to_le_bytes());
                p
            };
            // (slot, offset, size); variant 1 + 2i leaves parameter i out,
            // 2 + 2i makes it not fit
            let layout: [(u32, u16, u16); 7] = [
                (0x3fe4, 0x08, 0x10),
                (0x3fe8, 0x18, 0x10),
                (0x3f5c, 0x28, 0x40),
                (0x3fec, 0x68, 0x10),
                (0x3ff0, 0x78, 0x10),
                (0x3ff4, 0x88, 0x10),
                (0x3ff8, 0x98, 0x10),
            ];
            for (i, &(slot, off, size)) in layout.iter().enumerate() {
                let i = i as u32;
                let p = if var == 17 && i == 0 {
                    // an offset of 3 that would fit
                    param(&mut t, 3, size)
                } else if var == 1 + 2 * i {
                    0
                } else if var == 2 + 2 * i {
                    param(
                        &mut t,
                        if i.is_multiple_of(2) { 2 } else { off },
                        size + 0xc0,
                    )
                } else {
                    param(&mut t, off, size)
                };
                t.w32(fx + slot, p);
            }
            let light = t.zeros(0x80);
            for w in 0..12u32 {
                t.w32(light + 4 * w, (0.2 * w as f32 - 0.9).to_bits());
            }
            for w in 12..16u32 {
                t.w32(light + 4 * w, (1.5 + w as f32).to_bits());
            }
            t.w32(light + 0x50, ty);
            t.w32(light + 0x54, 6.5f32.to_bits());
            t.w32(light + 0x58, 2.25f32.to_bits());
            t.w32(light + 0x5c, 0.4f32.to_bits());
            t.w32(light + 0x60, 0.7f32.to_bits());
            t.w32(light + 0x64, u32::from(var != 15));
            t.w32(light + 0x68, if var == 16 { 3 } else { 1 });
            let renderer = t.junk(0x140, 0x40 ^ k as u8);
            t.patch(0x0094_97be, &[0xd9, 0xf2, 0xdd, 0xd8, 0xc3]); // _CItan: fptan
            t.patch(0x0094_98c2, &[0xd9, 0xff, 0xc3]); // _CIcos: fcos
            t.copy_stub(bp::MEMCPY);
            t.stub(clear_scene, true, 1, 1, 0x99);
            t.stub(
                bp::RENDER_MESH_INSTANCE,
                true,
                4,
                4,
                if fail { 0x31 } else { 0 },
            );
            let ebp = t.ebp_for(2);
            // the word the original reads before writing it
            t.w32(ebp - 8, 0);
            t.check_frame(
                &format!("{label}: draw light {k}"),
                0x00a2_06d0,
                Conv::Thiscall(fx),
                &[renderer, light],
                ebp,
                0x140,
                (0x130, 0x30),
                |g| Out::U32(bp::light_draw::draw_light(g, ebp, fx, renderer, light)),
            );
            t.reset();
        }
        TRAMP.with(|c| c.set(0));
    }
}

/// `0x00a04b20` (the deferred lighting's cone mesh): every callee
/// recorded (`_CIcos`/`_CIsin` patched to `fcos`/`fsin`); the mesh array
/// with its segment or not (the allocation failing, the old data owned
/// or a view, the block the old data), the stream count right or not
/// (that allocation failing), size words agreeing or not with the buffers
/// bound or not, versions wrapping, both unmaps failing, no instance
/// list or no instance; rim sizes 0, 1, 3, 7, 12 and 32. Only synthetic
/// data reaches it.
#[test]
fn bp3_create_cone() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    mesh_builder(
        "cone",
        0x00a0_4b20,
        &[3, 0, 1, 32, 7, 12],
        0x38,
        |n| (n.wrapping_mul(6), n + 2),
        |g: &mut G, e, c, n| bp::cone::create_cone(g, e, c, n),
    );
}

/// `0x00a06ab0` (the deferred lighting's sphere mesh) as
/// [`bp3_create_cone`]: sizes 0, 1, 2, 3, 5, 7, 8, 11 and 13 (one vertex and no
/// quad at 0). Only synthetic data reaches it.
#[test]
fn bp3_create_sphere() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    mesh_builder(
        "sphere",
        0x00a0_6ab0,
        &[3, 0, 1, 2, 5, 8, 7, 11, 13],
        0x5c,
        |n| (6 * n * n, (n + 1) * (n + 1)),
        |g: &mut G, e, c, n| bp::cone::create_sphere(g, e, c, n),
    );
}

/// The mesh builders' cases (the cone's and the sphere's): `sizes` in
/// turn, the frame down to `frame`, `counts(n)` the index and vertex
/// counts.
#[allow(clippy::too_many_lines)] // the case data
fn mesh_builder(
    what: &str,
    sym: u32,
    sizes: &[u32],
    frame: u32,
    counts: impl Fn(u32) -> (u32, u32),
    port: impl Fn(&mut G, u32, u32, u32) -> u32,
) {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    use ffx_prender::p6_quads as qd;
    let [
        format_size,
        index_bound,
        index_release,
        index_map,
        index_unmap,
        _,
        _,
    ] = bp::CONE_CALLS;
    for (label, mut t) in Twins::all() {
        t.trampoline();
        for k in 0..24u32 {
            let n = sizes[k as usize % sizes.len()];
            let (icount, vcount) = counts(n);
            let cluster = t.junk(0x20, 0x11);
            let mesh = t.junk(0x10, 0x12);
            let seg = t.junk(0x6c, 0x13);
            let streams = t.junk(0x80, 0x14);
            t.w32(seg + 0x2c, streams);
            t.w32(seg + 0x28, if k % 3 == 0 { 2 } else { k % 5 });
            let record = t.zeros(0x10);
            t.w32(streams + 0xc, record);
            t.w32(streams, 0xc);
            t.w32(streams + 4, vcount);
            t.w32(streams + 0x38, if k % 2 == 0 { 0xc * vcount } else { 7 });
            t.w32(streams + 0x10, k % 4);
            let wrap = match k % 4 {
                1 => 2 | (k & 4) << 29,
                3 => 2,
                _ => 9 | (k & 1) << 31,
            };
            t.w32(streams + 0x18, wrap);
            t.w32(seg + 0x30 + 0x34, if k % 3 == 1 { 2 * icount } else { 5 });
            t.w32(seg + 0x30 + 0x10, k % 3);
            t.w32(
                seg + 0x30 + 0x14,
                [0x8000_0010, 0x8000_0002, 3][k as usize % 3],
            );
            let fresh = t.junk(0x6c, 0x15);
            t.w32(fresh + 0x2c, streams);
            t.w32(fresh + 0x28, 2);
            let mut mallocs = vec![];
            match k % 4 {
                0 => {
                    t.w32(mesh, 1);
                    t.w32(mesh + 4, seg);
                }
                1 => {
                    t.w32(mesh, 3 | (k & 8) << 28);
                    t.w32(mesh + 4, 0x7e00);
                    mallocs.push(seg);
                }
                2 => {
                    t.w32(mesh, 0x8000_0001);
                    t.w32(mesh + 4, seg);
                    mallocs.push(0);
                }
                _ => {
                    t.w32(mesh, 2);
                    t.w32(mesh + 4, seg);
                    mallocs.push(seg);
                }
            }
            let block = t.zeros(0x80);
            mallocs.push(if k % 7 == 3 { 0 } else { block });
            let vertices = t.junk(0x1000, 0x16);
            let indices = t.junk(0x1000, 0x17);
            let list = t.zeros(0x40);
            t.w32(list + 0x34, if k % 5 == 4 { 0 } else { 0x1a57 });
            t.seq_answers(qd::CLUSTER_CREATE, true, 3, 3, &[mesh]);
            t.seq_answers(qd::MALLOC_ALIGNED, false, 2, 0, &mallocs);
            t.stub(qd::SEGMENTS_CONSTRUCT, false, 2, 0, 0x5e90_0001);
            t.stub(qd::SEGMENTS_DESTROY, false, 2, 0, 0x5e90_0002);
            t.stub(qd::STREAM_CTOR, true, 0, 0, 0x5e90_0003);
            t.stub(qd::STREAMS_SET, true, 2, 2, 0x5e90_0004);
            t.stub(qd::STREAM_FORMAT, true, 3, 3, 0x5e90_0005);
            t.stub(
                qd::IS_BOUND,
                true,
                0,
                0,
                if k % 6 < 3 { 0x100 } else { 0x101 },
            );
            t.stub(
                index_bound,
                true,
                0,
                0,
                if k % 4 < 2 { 0x201 } else { 0x200 },
            );
            t.stub(qd::RELEASE, true, 0, 0, 0x5e90_0006);
            t.stub(index_release, true, 0, 0, 0x5e90_0008);
            t.stub(format_size, false, 1, 0, 2);
            t.stub_wide(index_map, true, 1, 1, Some(indices), 0x5e90_0009);
            t.stub_wide(qd::MAP, true, 1, 1, Some(vertices), 0x5e90_000a);
            t.stub(qd::UNMAP, true, 2, 2, u32::from(k % 5 == 2) * 7);
            t.stub(
                index_unmap,
                true,
                2,
                2,
                [0, 9, 0, 0, 5, 0, 0][k as usize % 7],
            );
            t.seq_answers(
                qd::ALLOCATE_INSTANCE_LIST,
                true,
                3,
                3,
                &[if k % 6 == 5 { 0 } else { list }],
            );
            t.stub(qd::INSTANCE_CTOR, true, 1, 1, 0x5e90_0007);
            t.patch(0x0094_98c2, &[0xd9, 0xff, 0xc3]); // _CIcos: fcos
            t.patch(0x0094_98c8, &[0xd9, 0xfe, 0xc3]); // _CIsin: fsin
            let ebp = t.ebp_for(2);
            t.check_frame(
                &format!("{label}: {what} {k}"),
                sym,
                Conv::Cdecl,
                &[cluster, n],
                ebp,
                frame + 0x18,
                (frame, frame - 0x10),
                |g| Out::U32(port(g, ebp, cluster, n)),
            );
            t.reset();
        }
        TRAMP.with(|c| c.set(0));
    }
}

/// `0x006dfad0` (the character shader manager's variant materials): every
/// callee recorded; kinds 0..3 and another (the variant kept), no
/// variant; the key found in the tree (and a node at -0x10, which counts
/// as missing); arrays of 0, 1 and 3 materials (inline and out-of-line
/// data), a new array shorter than the old, no new array; parameters
/// found or not, fitting or not on either side, a source at address 0;
/// names absent, copied, the allocation failing, the old name owned or
/// not. The key's argument slot goes to the trees and the second slot
/// holds the last source buffer, compared after each call. Only
/// synthetic data reaches it.
#[test]
#[allow(clippy::too_many_lines)] // the case data
fn bp3_make_variants() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    let [
        find,
        create,
        resize,
        create_material,
        find_param,
        malloc,
        free,
        set_variant,
        unlink,
        insert,
    ] = bp::VARIANT_CALLS;
    for (label, mut t) in Twins::all() {
        for k in 0..20u32 {
            let [g_variant, _, _, _] = bp::VARIANT_GLOBALS.map(|a| t.rebase(a));
            for i in 0..4 {
                let v = if k == 3 && i > 0 { 0 } else { 0x7a00 + i };
                t.w32(g_variant + 4 * i, v);
            }
            let kind = [0u32, 1, 2, 1, 5, 0][k as usize % 6];
            let n = [3u32, 1, 0, 3, 3][k as usize % 5];
            // the old array: inline for one material
            let old = t.zeros(0x20);
            let mut sources = vec![];
            for i in 0..n {
                let src = t.zeros(0x20);
                let inner = t.junk(0x10, 0x40 + i as u8);
                let p = t.zeros(4);
                t.w32(p, inner);
                let head = t.zeros(0x20);
                t.w32(head + 0x10, p);
                t.w32(src, head);
                // the source buffer: size word, records at +0xc
                let sp = t.zeros(0x80);
                t.w32(sp, 0x40);
                let recs = t.zeros(0x40);
                for j in 0..3u32 {
                    let r = recs + 0x10 * j;
                    t.w32(r + 4, 0x6e00 + j + u32::from(j == 1));
                    let (off, size): (u16, u16) = match (k + j) % 5 {
                        0 => (8, 8),
                        1 => (2, 8),
                        2 => (0x3c, 8),
                        _ => (0x10, 0xe004),
                    };
                    t.write(r + 8, &off.to_le_bytes());
                    t.write(r + 0xa, &size.to_le_bytes());
                }
                t.w32(sp + 8, 3 | (k & 1) << 31);
                t.w32(sp + 0xc, recs);
                t.w32(src + 4, sp);
                let name = match (k + i) % 4 {
                    0 => 0,
                    1 => {
                        let s = t.alloc(b"Variant\0");
                        s | 1
                    }
                    _ => t.alloc(b"shade_a\0"),
                };
                t.w32(src + 0x10, name);
                sources.push(src);
            }
            t.w32(old, n | (k & 2) << 30);
            if n == 1 {
                t.w32(old + 4, sources[0]);
            } else {
                let d = t.zeros(0x10);
                for (i, &s) in sources.iter().enumerate() {
                    t.w32(d + 4 * i as u32, s);
                }
                t.w32(old + 4, d);
            }
            let key = t.zeros(0x20);
            t.w32(key, 0x5100);
            t.w32(key + 0x10, old);
            // the new array
            let new = if k == 7 && n == 0 { 0 } else { t.zeros(0x20) };
            if new != 0 {
                let count = if k % 7 == 5 { n.saturating_sub(1) } else { n };
                t.w32(new, count);
                let d = t.zeros(0x10);
                t.w32(new + 4, d);
            }
            let mut mats = vec![];
            for i in 0..n {
                let mat = t.zeros(0x20);
                let pb = t.zeros(0x80);
                t.w32(pb, 0x30);
                t.w32(pb + 4, 0x5ade);
                t.w32(mat + 4, pb);
                t.w32(mat + 0x10, 0x4000 | ((k + i) & 1));
                mats.push(mat);
            }
            let mut params = vec![];
            for j in 0..3 * n {
                let p = if (k + j) % 4 == 3 {
                    0
                } else {
                    let p = t.zeros(0x10);
                    let off: u16 = [0x10, 0x28, 0x2c][((k + j) % 3) as usize];
                    t.write(p + 8, &off.to_le_bytes());
                    t.write(p + 0xa, &8u16.to_le_bytes());
                    p
                };
                params.push(p);
            }
            let blocks: Vec<u32> = (0..n)
                .map(|i| if (k + i) % 5 == 3 { 0 } else { t.zeros(0x10) })
                .collect();
            let node = match k % 4 {
                1 => 0xffff_fff0,
                2 if k > 8 => 0x7700,
                _ => 0,
            };
            t.seq_answers(find, true, 1, 1, &[node]);
            t.seq_answers(create, true, 3, 3, &[new]);
            t.stub(resize, true, 1, 1, 0);
            t.seq_answers(create_material, true, 3, 3, &mats);
            t.seq_answers(find_param, true, 1, 1, &params);
            t.seq_answers(malloc, false, 2, 0, &blocks);
            t.stub(free, false, 1, 0, 0);
            t.copy_stub(bp::MEMCPY);
            t.stub(set_variant, true, 2, 2, 0x51);
            t.stub(unlink, true, 0, 0, 0x52);
            t.seq_answers(insert, true, 2, 2, &[0x61, 0x62]);
            let ebp = t.ebp_for(2);
            let slots = ebp + 8;
            t.w32(slots, key);
            t.w32(slots + 4, kind);
            t.check_frame(
                &format!("{label}: variants {k}"),
                0x006d_fad0,
                Conv::Cdecl,
                &[key, kind],
                ebp,
                0x50,
                (0x38, 0x28),
                |g| Out::U32(bp::variants::make_variants(g, ebp, slots, key, kind)),
            );
            let (a, b): (u32, u32) = (Mem::read(&t.a, slots + 4), Mem::read(&t.b, slots + 4));
            assert_eq!(a, b, "{label}: variants {k}: argument slot");
            t.reset();
        }
    }
}

/// `0x005b9ee0` (the renderer's skinned batch): the matrix callees run as
/// the originals over joint matrices both oracles hold alike, the rest
/// recorded. The joint block is the original's `_alloca_probe_16` block
/// (`(ebp - 0x174 - bytes) & !0xf`, handed to the port) or, past 0x80
/// matrices, a preprocess ring block. Cases: the renderer not ready;
/// items without a material, hidden segments, segments of each pass;
/// the "Opaque" pass skipping "Transparent" items (and not when the
/// descriptor is transparent at `+0xc` too); with and without dynamic
/// data (a table word set or not); the skin upload needed or not; fill
/// errors (the last kept). Only synthetic data reaches it.
#[test]
#[allow(clippy::too_many_lines)] // the case data
fn bp3_render_batch() {
    use ffx_prender::m5p_render_b_bp3::{self as bp};
    let [
        singleton,
        allocate,
        compute,
        upload,
        _,
        _,
        block_count,
        count_kinds,
        fill,
        ring_alloc,
        queue,
    ] = bp::BATCH_CALLS;
    let [opaque, transparent] = bp::BATCH_PASS_NAMES;
    for (label, mut t) in Twins::all() {
        for k in 0..16u32 {
            let joints_n = match k % 5 {
                4 => 0x90,
                3 => 0x7f,
                _ => 3 + k % 3,
            };
            let r = t.junk(0x100, 0x21);
            t.w32(r + 4, if k == 1 { 0x10 } else { 0x11 });
            t.w32(r + 0x48, t.rebase(if k % 2 == 0 { opaque } else { 0x7e00 }));
            let view = t.junk(0x100, 0x22);
            t.w32(r + 0x18, view);
            let mesh = t.zeros(0x20);
            t.w32(mesh + 0x18, joints_n | (k & 1) << 31);
            let nseg = 4u32;
            let segs = t.zeros((0x6c * nseg) as usize);
            for s in 0..nseg {
                let seg = segs + 0x6c * s;
                t.w32(seg + 4, (s + k) % (joints_n.min(4)));
                t.w8(seg + 0x14, if (s + k) % 7 == 6 { 2 } else { 0 });
                t.w32(seg + 0x1c, (s * 3 + k) % 5);
            }
            t.w32(mesh + 4, segs);
            let inst = t.zeros(0x90);
            t.w32(inst, mesh);
            t.w32(inst + 0x14, 0x5c1e);
            let dynamic = if k % 3 == 0 {
                0
            } else {
                let d = t.zeros(0x10);
                let m = t.zeros(0x20);
                let table = t.zeros(0x100);
                for s in 0..nseg {
                    t.w32(table + 0x10 * s + 0xc, (s + k) & 1);
                }
                t.w32(m + 0x14, table);
                t.w32(d + 4, m);
                d
            };
            t.w32(inst + 0x18, dynamic);
            t.w32(inst + 0x80, if k % 4 == 2 { 0x0f0f } else { 0x1234 });
            let batch = t.zeros(0x10);
            t.w32(batch, inst);
            t.write(batch + 4, &[u8::from(k % 6 == 5), 0, nseg as u8, 0]);
            t.w8(batch + 8, u8::from(k % 2 == 0));
            let items = t.zeros((0x20 * nseg) as usize);
            for s in 0..nseg {
                let item = items + 0x20 * s;
                t.w32(item, 0x1700 + s);
                let mat = if (s + k) % 5 == 3 {
                    0
                } else {
                    let m = t.zeros(0x20);
                    t.w32(m, 0x4ead + s);
                    t.w32(m + 0x14, ((s + k) % 3) | ((k & 2) << 30) | ((k & 4) << 18));
                    m
                };
                t.w32(item + 4, mat);
                let desc = t.zeros(0x10);
                t.w32(desc + 4, 0x9a55 + s);
                let names = [(transparent, 0), (transparent, transparent), (0x7e01, 0)]
                    [((s + k) % 3) as usize];
                t.w32(
                    desc + 8,
                    if names.0 > 0x7f00 {
                        t.rebase(names.0)
                    } else {
                        names.0
                    },
                );
                t.w32(desc + 0xc, if names.1 == 0 { 0 } else { t.rebase(names.1) });
                t.w32(item + 0x1c, desc);
            }
            let cursor = t.zeros(4);
            t.w32(cursor, items);
            let ebp = t.ebp_for(3);
            let bytes = (joints_n + 1) << 6;
            let ring_block = t.junk(0x10 + bytes as usize, 0x23);
            let alloca = if joints_n > 0x80 {
                0
            } else {
                (ebp - 0x174 - bytes) & !0xf
            };
            let base = if alloca == 0 {
                ring_block + 0x10
            } else {
                alloca
            };
            for w in 0..(bytes / 4) {
                let v = 0.25f32 * f32::from(u8::try_from(w % 7).unwrap())
                    + if w % 5 == 0 { 1.0 } else { 0.0 };
                t.w32(base + 4 * w, v.to_bits());
            }
            t.seq_answers(singleton, false, 0, 0, &[0x6100]);
            t.seq_answers(allocate, true, 1, 1, &[ring_block]);
            t.stub(compute, true, 1, 1, 0x71);
            t.stub(upload, true, 1, 1, 0x72);
            t.stub(block_count, true, 1, 1, 2 + k % 2);
            t.stub(
                count_kinds,
                false,
                1,
                0,
                if k % 4 == 3 { 0xffff_ffff } else { 3 },
            );
            let errs: Vec<u32> = (0..16)
                .map(|j| if (j + k) % 6 == 1 { 0x30 + j } else { 0 })
                .collect();
            t.seq_wide(fill, 8, 8, Some((4, 0x5d + k)), &errs);
            t.stub(ring_alloc, true, 1, 1, 0xb10c);
            t.stub_wide(queue, true, 8, 8, None, 0x73);
            t.check_frame(
                &format!("{label}: batch {k}"),
                0x005b_9ee0,
                Conv::Thiscall(r),
                &[batch, cursor, 0x1234],
                ebp,
                0x180 + bytes,
                (0x164, 0x54),
                |g| {
                    Out::U32(bp::batch::render_batch(
                        g,
                        ebp,
                        r,
                        (batch, cursor, 0x1234),
                        alloca,
                    ))
                },
            );
            t.reset();
        }
    }
}

// ---- tail10: the lighting tweak tool's gizmo (ffx_prender::l10_tweak) ----

use ffx_prender::l10_tweak::{self as l10t, bridge as l10tb, frame as l10f};

impl Twins {
    /// A recording stub at `sym` (as [`Twins::stub`]) that first runs
    /// `pre` (code keeping `esp`, `ecx` and the arguments).
    #[allow(clippy::too_many_arguments)]
    fn l10_stub_pre(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32, pre: &[u8]) {
        let log = self.stub_code(sym, this, n, pops, ret);
        let mut c = pre.to_vec();
        c.push(0x68);
        c.extend(log.to_le_bytes()); // push log
        c.push(0xc3); // ret: on to the logging stub
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        self.patch(sym, &j);
    }
}

/// `below` / `chance` for the tail10 test.
trait L10Rng {
    fn below(&mut self, n: u32) -> u32;
    fn chance(&mut self, n: u32) -> bool;
}

impl L10Rng for Rng {
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.next().is_multiple_of(n)
    }
}

/// Code copying `words` words from the pointer at `[esp + k]` (at the
/// stub's entry) to `[cursor]`, advancing it; keeps every register.
fn l10_copy(k: u8, words: u32, cursor: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57, 0x51]; // push esi; push edi; push ecx
    c.extend([0x8b, 0x74, 0x24, k + 12]); // mov esi, [esp+k+12]
    c.extend([0x8b, 0x3d]);
    c.extend(cursor.to_le_bytes()); // mov edi, [cursor]
    c.push(0xb9);
    c.extend(words.to_le_bytes()); // mov ecx, words
    c.extend([0xf3, 0xa5]); // rep movsd
    c.extend([0x89, 0x3d]);
    c.extend(cursor.to_le_bytes()); // mov [cursor], edi
    c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
    c
}

/// Code writing the words `v` through the pointer at `[esp + k]`.
fn l10_write(k: u8, v: &[u32]) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, k]; // mov eax, [esp+k]
    for (i, w) in v.iter().enumerate() {
        c.extend([0xc7, 0x40, (4 * i) as u8]);
        c.extend(w.to_le_bytes()); // mov dword [eax+4i], w
    }
    c
}

/// `CharacterLightingTweakUtil_drawGizmo_006a7850` (tail10): the tool off
/// and on; the anchor near (kept) and far (moved); the light's colour lit
/// or dark (the 27 arrows or none); a point light of the set-up or not
/// (the set-up off, another light type: no line, range 0), dark or lit;
/// the guard set or not. The lighting queries, the matrix helpers, the
/// lock and the renderer are recording stubs: the queries write their
/// vectors (and return their output), the transform and the draws copy
/// the vectors they are handed into memory the diff sees; `_CIsqrt` is the
/// CRT's. The frame lies at the original's `ebp - 0x1b4`; the words the
/// callees see are compared after the call. Synthetic only.
#[test]
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn l10_tweak_gizmo() {
    let f32w =
        |rng: &mut Rng, span: u32| ((rng.below(2 * span) as f32 - span as f32) / 7.0).to_bits();
    for (label, mut t) in Twins::all() {
        t.trampoline();
        let mut rng = Rng(0x006a_7850);
        let mut seen = [0u32; 4];
        for case in 0..60_u32 {
            t.reset();
            let ebp = t.a.stack_range().end - 0x100 - 4 - 8;
            let f = ebp - 0x1b4;
            let copies = t.zeros(0x4000);
            let cursor = t.alloc(&copies.to_le_bytes());
            // the lighting set-up
            let n_lights = 4;
            let i = rng.below(n_lights);
            let lights: Vec<u32> = (0..n_lights).map(|_| t.zeros(0x40)).collect();
            let vt = t.rebase(l10t::POINT_LIGHT_VTABLE);
            for &l in &lights {
                for o in (0..0x10).step_by(4) {
                    let v = if case % 5 == 1 { 0 } else { f32w(&mut rng, 20) };
                    t.w32(l + o, v);
                }
                t.w32(l + 0x2c, f32w(&mut rng, 400));
                t.w32(l + 0x18, if rng.chance(3) { rng.next() } else { vt });
            }
            let list = t.alloc(
                &lights
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let set = t.zeros(0x10);
            t.w32(set + 4, list);
            let lighting = t.zeros(0x200);
            t.w32(lighting + 0xf0, set);
            t.w8(lighting + 0x1cc, u8::from(!rng.chance(4)));
            let gl = t.rebase(l10t::LIGHTING);
            t.patch(l10t::LIGHTING, &lighting.to_le_bytes());
            let _ = gl;
            let guard = t.rebase(l10t::GUARD);
            let g = t.r32(guard);
            t.w32(guard, if rng.chance(2) { g | 1 } else { g & !1 });
            // the tool and the character
            let util = t.zeros(0x100);
            t.w8(util + 0xc8, u8::from(case % 9 != 4));
            t.w32(util + 0xc0, i);
            for o in (0xe8..0xf8).step_by(4) {
                let span = if rng.chance(2) { 200 } else { 4000 };
                let v = f32w(&mut rng, span);
                t.w32(util + o, v);
            }
            let src = t.zeros(0x40);
            for o in (0x20..0x2c).step_by(4) {
                let v = f32w(&mut rng, 300);
                t.w32(src + o, v);
            }
            let character = t.zeros(0xc0);
            t.w32(character + 0xb0, src);
            // the queries write their vectors
            let dir: Vec<u32> = (0..3).map(|_| f32w(&mut rng, 7)).collect();
            let colour: Vec<u32> = (0..4)
                .map(|_| if case % 4 == 2 { 0 } else { f32w(&mut rng, 7) })
                .collect();
            let pos: Vec<u32> = (0..4).map(|_| f32w(&mut rng, 300)).collect();
            let axis: Vec<u32> = (0..4).map(|_| f32w(&mut rng, 7)).collect();
            let matrix: Vec<u32> = (0..16).map(|k| k * 0x101).collect();
            let mut pre = l10_write(4, &dir);
            t.l10_stub_pre(l10t::LIGHT_DIRECTION, true, 2, 2, f + l10f::DIRECTION, &pre);
            pre = l10_write(4, &colour);
            t.l10_stub_pre(l10t::LIGHT_COLOUR, true, 2, 2, f + l10f::COLOUR, &pre);
            pre = l10_write(4, &pos);
            t.l10_stub_pre(l10t::LIGHT_POSITION, true, 2, 2, f + l10f::POSITION, &pre);
            // FROM_3X4 writes the matrix through ecx
            pre = vec![0x89, 0xc8]; // mov eax, ecx
            for (k, w) in matrix.iter().enumerate() {
                pre.extend([0xc7, 0x40, (4 * k) as u8]);
                pre.extend(w.to_le_bytes());
            }
            t.l10_stub_pre(l10t::FROM_3X4, true, 1, 1, 0, &pre);
            // the transform: its input copied, a fixed output written (the
            // second word of the output pointer's target from the input)
            pre = l10_copy(8, 4, cursor);
            pre.extend(l10_write(4, &axis));
            t.l10_stub_pre(l10t::TRANSFORM, true, 2, 2, f + l10f::AXIS, &pre);
            // the draws: three points and the colour copied
            // the n points (n at [esp+0x14]): copy 4n words
            pre = l10_copy(8, 0, cursor);
            // `mov ecx, 0` after the three pushes, `mov esi` and `mov edi`
            let at = 13;
            assert_eq!(pre[at], 0xb9);
            pre.splice(
                at..at + 5,
                [0x8b, 0x4c, 0x24, 0x14 + 12, 0xc1, 0xe1, 0x02], // mov ecx, [esp+0x20]; shl ecx, 2
            );
            pre.extend(l10_copy(12, 4, cursor));
            t.l10_stub_pre(l10t::DRAW_POINTS, true, 6, 6, 0, &pre);
            t.stub(l10t::LOCK_ENTER, true, 0, 0, 0);
            t.stub(l10t::LOCK_LEAVE, true, 0, 0, 0);
            t.stub(l10t::INTERFACE_CTOR, true, 0, 0, 0);
            t.stub(l10t::ATEXIT, false, 1, 0, 0);
            let fill: Vec<u8> = (0..l10f::BYTES).map(|k| (k * 3) as u8).collect();
            t.write(f, &fill);
            t.b.set_call_top(Some(f - 0x100));
            let what = format!("{label}: gizmo {case}");
            t.check(
                &what,
                0x006a_7850,
                Conv::Thiscall(util),
                &[character],
                |g| {
                    l10tb::draw_gizmo(g, util, character, f);
                    Out::None
                },
            );
            t.b.set_call_top(None);
            for (lo, hi) in [
                (0, 0xe0),
                (0xf4, 0x108),
                (0x10c, 0x12c),
                (0x174, l10f::BYTES),
            ] {
                for k in (lo..hi).step_by(4) {
                    assert_eq!(
                        Mem::read::<u32>(&t.a, f + k),
                        Mem::read::<u32>(&t.b, f + k),
                        "{what}: frame at ebp - {:#x}",
                        0x1b4 - k
                    );
                }
            }
            // copied bytes: 16 per transform, 48 per line, 64 per arrow
            let bytes = t.r32(cursor) - copies;
            seen[match bytes {
                0 => 0,
                16 => 1,
                208 => 2,
                _ => 3,
            }] += 1;
        }
        assert!(seen.iter().all(|&n| n > 0), "{label}: {seen:?}");
    }
}
