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
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
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
    assert_eq!(rb::COPIES.len(), 4);
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
