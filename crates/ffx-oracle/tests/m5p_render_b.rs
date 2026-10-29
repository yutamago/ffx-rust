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
            let s = self.stub_code(0xc000 + i as u32, false, n, n, r);
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
