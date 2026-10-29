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
    assert_eq!(morph::MORPHS.len(), 7);
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
