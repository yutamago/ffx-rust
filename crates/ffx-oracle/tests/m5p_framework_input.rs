//! `PhyreEngine`'s application framework, timers, `PhyreOS`, file streams
//! and input devices, list `phyre-framework-input`
//! (`ffx_pother_b::m5p_framework_input*`, M5 stage 1), against the
//! original. Almost none of this code ran in a coverage run, so every case
//! is synthetic memory built for its branch, on the image alone and over a
//! snapshot (`FFX_SNAPSHOT`, run once with each of field.snap and
//! battle.snap). Results and every byte written must agree; template
//! copies each run against their own original. Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list, and the Win32 imports (through their import
//! slots), are replaced in both oracles alike by recording stubs: each call
//! appends its tag, `ecx` and its stack words to a log in memory, so the
//! diff checks that both made the same calls in the same order.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_b::m5p_framework_input_small::{self as sm, bridge as smb};

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

/// The bridge's view of an oracle: its memory, its emulator for the
/// original callees, where its scratch is, and the intrinsic trampoline
/// ([`TRAMPOLINE_CODE`]).
struct G<'a>(&'a mut Oracle, u32, u32);

/// `fld qword [esp+4]; call [esp+0xc]; ret`, and at `+16` `fstp st0;
/// ret`, for `Host::call_intrinsic` (a CRT intrinsic with its argument in
/// `ST0`).
const TRAMPOLINE_CODE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

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
        self.1
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let at = self.2;
        let b = x.to_bits();
        let r = self
            .0
            .call(at, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let Some(v) = r.st0_f32() else {
            panic!("intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
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
    /// `al` (the rest of `eax` is the caller's).
    Al(u8),
    /// A `float` left in `ST0`.
    F32(f32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up to 14
    /// arguments).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
    /// How many bytes of the frame [`Twins::check_frame`] compares after
    /// the call (where the original's frame word and the port's scratch
    /// both hold what the callee was handed).
    frame_len: u32,
    /// The intrinsic trampoline, at the same address in both.
    tramp: u32,
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
            frame_len: 0,
            tramp: 0,
        };
        t.log = t.zeros(0x10000);
        t.tramp = t.a.alloc_code(&TRAMPOLINE_CODE).unwrap();
        assert_eq!(t.b.alloc_code(&TRAMPOLINE_CODE).unwrap(), t.tramp);
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
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
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
    /// [`Twins::check`] for a port whose callees see a word of the
    /// original's frame by address: the port's scratch is put where the
    /// original's word is (`below` bytes under where the call pushes from)
    /// and its own calls run under it.
    fn check_frame(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        below: u32,
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let top = self.b.stack_range().end - 0x100;
        self.b.set_call_top(Some(top - 0x400));
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, top - below, tramp));
        self.b.set_call_top(None);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::None => {}
            _ => unreachable!(),
        }
        if self.frame_len != 0 {
            let at = top - below;
            let fa = Oracle::read(&mut self.a, at, self.frame_len as usize).unwrap();
            let fb = Oracle::read(&mut self.b, at, self.frame_len as usize).unwrap();
            assert_eq!(fa, fb, "{what}: the frame the callee saw");
            self.frame_len = 0;
        }
        self.diff(what);
    }

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
        let scratch = self.b.stack_range().start + 0x100;
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, scratch, tramp));
        if !matches!(out, Out::F32(_)) {
            assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        }
        match out {
            Out::None => {}
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::F32(v) => {
                let st0 = ra.st0_f32().unwrap_or_else(|| panic!("{what}: no ST0"));
                assert_eq!(st0.to_bits(), v.to_bits(), "{what}: ST0");
            }
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

// ---- batch 10: the pad's axis values ----

#[test]
fn pad_axes_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (seed, words) in [
            (0x71u8, [0u32, 255, 128, 1023]),
            (0x72, [0x8000_0000, 0xffff_ffff, 7, 0x7fff_ffff]),
        ] {
            let pad = t.junk(0x400, seed);
            for (i, off) in (0x370u32..0x398).step_by(4).enumerate() {
                t.w32(pad + off, words[i % 4]);
            }
            for id in 0u32..=0x14 {
                let what = format!("{label}: pad axis {seed:#x} {id:#x}");
                t.check(&what, 0x0062_8ad0, Conv::Thiscall(pad), &[id], |g| {
                    Out::F32(scb::pad_axis(g, pad, id))
                });
            }
            t.check(
                &format!("{label}: pad axis -1"),
                0x0062_8ad0,
                Conv::Thiscall(pad),
                &[0xffff_ffff],
                |g| Out::F32(scb::pad_axis(g, pad, 0xffff_ffff)),
            );
        }
    }
}

impl Twins {
    /// A recording stub (as [`Twins::stub_code`]) put into the import slot
    /// `slot` (its `ffx.sym` address), tagged with the slot.
    fn stub_import(&mut self, slot: u32, n: u32, ret: u32) {
        let code = self.stub_code(slot, false, n, n, ret);
        let at = self.rebase(slot);
        let old = Oracle::read(&mut self.a, at, 4).unwrap();
        self.saved.entry(at).or_insert(old);
        self.w32(at, code);
    }
}

// ---- batch 1: template copies and one-liners ----

#[test]
fn tail_dtors_match_the_original() {
    assert_eq!(sm::TAIL_DTORS.len(), 12);
    for (label, mut t) in Twins::all() {
        t.stub(sm::DESTRUCT, true, 0, 0, 0xd357);
        t.stub(sm::STREAM_DTOR, true, 0, 0, 0x5d7);
        for &(f, d) in sm::TAIL_DTORS {
            let obj = t.junk(0x40, f as u8);
            let what = format!("{label}: tail dtor {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::tail_dtor(g, d, obj))
            });
        }
        assert_eq!(t.log_entries(), 12);
        t.reset();
    }
}

#[test]
fn deleting_dtors_match_the_original() {
    assert_eq!(sm::DELETES.len(), 7);
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(sm::OPERATOR_DELETE, false, 1, 0, 0xde1);
        t.stub(sm::STREAM_DTOR, true, 0, 0, 0x5d7);
        t.stub(sm::APPLICATION_DTOR, true, 0, 0, 0xa99);
        t.stub(sm::SCRIPT_BUFFER_DTOR, true, 0, 0, 0x5cb);
        for &(f, d) in sm::DELETES {
            for flags in [0u32, 1, 2, 3, 0xffff_fffe, 0xffff_ffff] {
                let obj = t.junk(0x40, f as u8 ^ flags as u8);
                let what = format!("{label}: deleting dtor {f:#x} flags {flags:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(smb::deleting_dtor(g, d, obj, flags))
                });
            }
        }
        assert!(t.log_entries() > 0);
        t.reset();
    }
}

#[test]
fn constants_match_the_original() {
    assert_eq!(sm::CONSTANTS.len(), 8);
    for (label, mut t) in Twins::all() {
        for &(f, v, words) in sm::CONSTANTS {
            let obj = t.junk(0x10, 1);
            let args: Vec<u32> = (0..words).map(|i| 0x1111_1111 * (i + 1)).collect();
            let what = format!("{label}: constant {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &args, |_| Out::U32(v));
            assert_eq!(sm::constant_of(f), v);
        }
    }
}

#[test]
fn fixed_answers_match_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x10, 2);
        for (f, al) in [
            (0x0062_89c0, true),
            (0x0062_8a10, true),
            (0x0062_8a20, false),
        ] {
            let what = format!("{label}: {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[7], |_| {
                if al { Out::Al(0) } else { Out::F32(0.0) }
            });
        }
        for f in [0x0062_8a30, 0x0062_8cb0] {
            let what = format!("{label}: {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[7], |_| Out::F32(0.0));
        }
    }
}

#[test]
fn vtable_stores_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (f, vt) in [
            (0x0060_ec70, sm::TIMELINE_BASE_VTABLE),
            (0x0062_0ed0, sm::INPUT_DEVICE_VTABLE),
        ] {
            let obj = t.junk(0x20, 9);
            let what = format!("{label}: vtable store {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                smb::set_vtable(g, obj, vt);
                Out::None
            });
        }
    }
}

#[test]
fn releases_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for f in sm::RELEASES {
            for count in [
                0u32,
                1,
                5,
                0x7fff_ffff,
                0x8000_0000,
                0x8000_0003,
                0xffff_ffff,
            ] {
                let obj = t.junk(0x10, count as u8);
                t.w32(obj, count);
                let what = format!("{label}: release {f:#x} count {count:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(smb::release(g, obj))
                });
            }
        }
        assert_eq!(t.log_entries(), 8);
        t.reset();
    }
}

#[test]
fn win32_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub_import(sm::SLEEP_SLOT, 1, 0x51ee);
        t.stub_import(sm::TIME_END_PERIOD_SLOT, 1, 0x7e9);
        t.stub_import(sm::CHANGE_DISPLAY_SETTINGS_SLOT, 2, 0xcd5);
        t.stub_import(sm::RELEASE_DC_SLOT, 2, 0x8dc);
        for ms in [0u32, 1, 16, 0xffff_ffff] {
            let what = format!("{label}: sleep {ms}");
            t.check(&what, 0x0043_0030, Conv::Cdecl, &[ms], |g| {
                Out::U32(smb::sleep(g, ms))
            });
        }
        t.check(
            &format!("{label}: timer terminate"),
            0x0043_0040,
            Conv::Cdecl,
            &[],
            |g| Out::U32(smb::timer_terminate(g)),
        );
        let hwnd = t.rebase(sm::G_HWND);
        t.w32(hwnd, 0x0001_0a2c);
        for full in [0u8, 1, 0x80] {
            let settings = t.junk(0x40, full);
            t.w8(settings + 0x2c, full);
            let win = t.junk(0x10, 3);
            t.w32(win, settings);
            let what = format!("{label}: restore display full {full}");
            t.check(&what, 0x0062_70a0, Conv::Thiscall(win), &[], |g| {
                Out::U32(smb::restore_display(g, win))
            });
        }
        assert_eq!(t.log_entries(), 4 + 1 + 3 + 2);
        t.reset();
    }
}

#[test]
fn float_copies_match_the_original() {
    let floats = [
        0u32,
        0x8000_0000,
        0x3f80_0000,
        0xc2c8_0000,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc0_1234,
        0x0000_0001,
        0x807f_ffff,
        0x7f7f_ffff,
    ];
    for (label, mut t) in Twins::all() {
        for (i, w) in floats.windows(3).enumerate() {
            let src = t.alloc(&[w[0], w[1], w[2]].map(u32::to_le_bytes).concat());
            let obj = t.junk(0x40, i as u8);
            let what = format!("{label}: set vec 0x20 {i}");
            t.check(&what, 0x0046_32f0, Conv::Thiscall(obj), &[src], |g| {
                Out::U32(smb::set_vec3_20(g, obj, src))
            });
        }
        // the spline point: the evaluation's result read back through eax
        for (i, w) in floats.windows(3).enumerate() {
            let point = t.alloc(&[w[0], w[1], w[2], 0x1234].map(u32::to_le_bytes).concat());
            t.stub(sm::SPLINE_EVALUATE, true, 2, 2, point);
            let obj = t.junk(0x40, 7);
            let out = t.junk(0x40, 8);
            let tv = w[1];
            t.frame_len = 0x10;
            let what = format!("{label}: spline point {i}");
            t.check_frame(
                &what,
                0x0054_b520,
                Conv::Thiscall(obj),
                &[tv, out],
                0x20,
                |g| {
                    let frame = g.1;
                    Out::U32(smb::spline_point_to_30(g, obj, tv, out, frame))
                },
            );
            t.reset();
        }
    }
}

#[test]
fn forwarders_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::STATIC_UNLINK, false, 1, 0, 0x0511);
        t.stub(sm::FILE_SEEK, true, 1, 1, 0x5ee);
        t.stub(sm::PRINTF, false, 3, 0, 0x9f);
        t.stub(sm::STREAM_CTOR, true, 1, 1, 0xc7);
        t.stub(sm::MOTION_BLUR_CHECK, true, 2, 2, 0xb1);
        let obj = t.junk(0x40, 4);
        t.check(
            &format!("{label}: static unlink"),
            0x004d_bab0,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(smb::static_unlink(g, obj)),
        );
        t.check(
            &format!("{label}: reader seek"),
            0x0057_6a80,
            Conv::Thiscall(obj),
            &[0x77],
            |g| Out::U32(smb::reader_file_seek(g, obj, 0x77)),
        );
        t.check(
            &format!("{label}: echo"),
            0x0062_3310,
            Conv::Cdecl,
            &[0x1234, 0x5678],
            |g| Out::U32(smb::echo(g, 0x5678)),
        );
        t.check(
            &format!("{label}: reader ctor"),
            0x009f_0630,
            Conv::Thiscall(obj),
            &[0x4321],
            |g| Out::U32(smb::stream_reader_ctor(g, obj, 0x4321)),
        );
        for on in [0u8, 1, 0xff] {
            let mb = t.junk(0x20, on);
            t.w8(mb + 0x10, on);
            let what = format!("{label}: motion blur check {on}");
            t.check(&what, 0x00a1_b980, Conv::Thiscall(mb), &[3, 4], |g| {
                Out::U32(smb::motion_blur_check(g, mb, 3, 4))
            });
        }
        assert_eq!(t.log_entries(), 6);
        t.reset();
    }
}

#[test]
fn chains_and_streams_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for n in [0u32, 1, 3] {
            let head = t.zeros(4);
            let mut next = 0;
            for i in 0..n {
                let b = t.junk(0x10, i as u8);
                t.w32(b, next);
                next = b;
            }
            t.w32(head, next);
            let what = format!("{label}: free chain {n}");
            t.check(
                &what,
                0x0057_dcb0,
                Conv::Cdecl,
                &[head],
                |g| match smb::free_chain(g, head) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
        }
        assert_eq!(t.log_entries(), 4);
        t.reset();
        let reader = t.object(&[0, 0, 0, 0, 0, 0, 1], &[0, 0, 0, 0, 0, 0, 0x123]);
        t.w32(reader + 0xc, 0xffff_fff0);
        for n in [0u32, 5, 0x8000_0000] {
            let what = format!("{label}: read counted {n:#x}");
            t.check(&what, 0x0057_e160, Conv::Thiscall(reader), &[n], |g| {
                Out::U32(smb::read_counted(g, reader, n))
            });
        }
        assert_eq!(t.log_entries(), 2);
        t.reset();
    }
}

#[test]
fn getters_match_the_original() {
    for (label, mut t) in Twins::all() {
        // spline segment counts
        for count in [0u32, 1, 2, 3, 0x8000_0001, 0x8000_0005, 0xffff_ffff] {
            for closed in [0u8, 1, 0x80] {
                let obj = t.junk(0x20, count as u8);
                t.w32(obj + 4, count);
                t.w8(obj + 0x18, closed);
                let what = format!("{label}: segments {count:#x} {closed}");
                t.check(&what, 0x0054_b300, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(smb::spline_segments(g, obj))
                });
            }
        }
        // newline search: found, not found, empty and reversed ranges
        let text = t.alloc(b"abc\ndef\n\n");
        for (from, to) in [(0u32, 9u32), (4, 9), (8, 9), (0, 3), (5, 5), (6, 2), (9, 9)] {
            let obj = t.junk(0x20, 5);
            t.w32(obj + 0xc, text + from);
            t.w32(obj + 0x10, text + to);
            let what = format!("{label}: newline {from}..{to}");
            t.check(&what, 0x0062_77b0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::find_newline(g, obj))
            });
        }
        // the pad's buttons, ids around the range and with upper bits set
        let pad = t.junk(0x400, 0x3c);
        for id in [
            0u32,
            1,
            0xa,
            0xb,
            0xc,
            0x10,
            0x1a,
            0x1b,
            0x20,
            0xff,
            0x100_000b,
            0xffff_ff0b,
            0xffff_ffff,
        ] {
            let what = format!("{label}: button {id:#x}");
            t.check(&what, 0x0062_89f0, Conv::Thiscall(pad), &[id], |g| {
                Out::U32(smb::pad_button(g, pad, id))
            });
        }
        // the viewport table and the display info
        let table = t.junk(0x40, 0x44);
        let app = t.junk(0x300, 0x55);
        t.w32(app + 0x2c0, table);
        for i in [0u32, 1, 7, 15] {
            let what = format!("{label}: viewport {i}");
            t.check(&what, 0x0062_9750, Conv::Thiscall(app), &[i], |g| {
                Out::U32(smb::viewport_at(g, app, i))
            });
        }
        t.check(
            &format!("{label}: display info"),
            0x0062_97a0,
            Conv::Thiscall(app),
            &[],
            |g| Out::U32(smb::window_display_info(g, app)),
        );
        // post-effect entries: in range, out of range, null, borrowed
        let e1 = t.junk(0x10, 0x61);
        let data = t.alloc(&[e1, 0, e1 + 4].map(u32::to_le_bytes).concat());
        for count in [0u32, 1, 3, 0x8000_0001, 0x8000_0003] {
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, data);
            for i in [0u32, 1, 2, 3, 0xffff_ffff] {
                let what = format!("{label}: entries {count:#x} {i}");
                t.check(&what, 0x00a1_6500, Conv::Thiscall(arr), &[i], |g| {
                    Out::U32(smb::element_plus4(g, arr, i))
                });
                t.check(&what, 0x00a1_6530, Conv::Thiscall(arr), &[i], |g| {
                    Out::U32(smb::element_word4(g, arr, i))
                });
            }
        }
    }
}

#[test]
fn setters_match_the_original() {
    for (label, mut t) in Twins::all() {
        for flag in [0u32, 1, 0x1234_56ab, 0xffff_ff00] {
            let what = format!("{label}: poll flag {flag:#x}");
            t.check(&what, 0x0062_50d0, Conv::Cdecl, &[flag], |g| {
                smb::set_poll_flag(g, flag as u8);
                Out::None
            });
        }
        for (lo, hi) in [(0u32, 0u32), (0x1_00ff, 0xabcd_0001), (0x80, 0x7f)] {
            let pad = t.junk(0x80, 0x22);
            let what = format!("{label}: vibration {lo:#x} {hi:#x}");
            t.check(&what, 0x0062_cb70, Conv::Thiscall(pad), &[lo, hi], |g| {
                smb::set_vibration(g, pad, lo as u8, hi as u8);
                Out::None
            });
        }
        // the joypad axis source: with and without a map (the getter runs)
        let map = t.junk(0x20, 0x31);
        for m in [0u32, map] {
            let src = t.junk(0x40, 0x32);
            t.w32(src + 0x1c, m);
            let what = format!("{label}: axis refresh {m:#x}");
            t.check(
                &what,
                0x009f_0330,
                Conv::Thiscall(src),
                &[],
                |g| match smb::axis_refresh(g, src) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
        }
    }
}

// ---- batch 2: input sources, streams, lazy swaps ----

use ffx_pother_b::m5p_framework_input_sources::{self as so, bridge as sob};
use ffx_pother_b::m5p_framework_input_streams::{self as st, bridge as stb};

/// Source scales: ordinary values, zeros, infinities, quiet NaNs, denormals.
const SCALES: [u32; 6] = [
    0x3f80_0000,
    0x8000_0000,
    0x7f80_0000,
    0xffc0_1234,
    0x0000_0001,
    0xc2c8_0000,
];

/// A source object (0x40 bytes) with its words set from `seed`.
fn source(t: &mut Twins, seed: u8, scale: u32) -> u32 {
    let s = t.junk(0x40, seed);
    t.w32(s + 0x14, scale);
    s
}

#[test]
fn source_clones_match_the_original() {
    assert_eq!(so::PLAIN_CLONES.len(), 16);
    assert_eq!(so::PAIR_CLONES.len(), 4);
    for (label, mut t) in Twins::all() {
        for &(_, k) in so::PAIR_CLONES {
            t.stub(k.check, true, 0, 0, 0xc4ec);
        }
        for &(f, ctor) in so::CTOR_CLONES {
            t.stub(ctor, true, 4, 4, 0xc7 ^ f);
        }
        for (i, &scale) in SCALES.iter().enumerate() {
            for fail in [false, true] {
                let buf = t.junk(0x40, 0x99);
                t.stub(so::PHYRE_MALLOC, false, 1, 0, if fail { 0 } else { buf });
                let src = source(&mut t, i as u8, scale);
                for &(f, k) in so::PLAIN_CLONES {
                    let what = format!("{label}: plain clone {f:#x} {i} {fail}");
                    t.check(&what, f, Conv::Thiscall(src), &[], |g| {
                        Out::U32(sob::plain_clone(g, k, src))
                    });
                }
                for &(f, k) in so::PAIR_CLONES {
                    let what = format!("{label}: pair clone {f:#x} {i} {fail}");
                    t.check(&what, f, Conv::Thiscall(src), &[], |g| {
                        Out::U32(sob::pair_clone(g, k, src))
                    });
                }
                for &(f, ctor) in so::CTOR_CLONES {
                    let what = format!("{label}: ctor clone {f:#x} {i} {fail}");
                    t.check(&what, f, Conv::Thiscall(src), &[], |g| {
                        Out::U32(sob::ctor_clone(g, ctor, src))
                    });
                }
            }
        }
        assert!(t.log_entries() > 100);
        t.reset();
    }
}

#[test]
fn source_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the default mouse device: an object whose kind is 0xc
        let def = t.object(&[0, 0], &[0, 0xc]);
        let vt = t.r32(def);
        let at = t.rebase(so::DEFAULT_MOUSE_DEVICE);
        t.w32(at, vt);
        for (i, &scale) in SCALES.iter().enumerate() {
            let map = t.junk(0x20, 0x31 + i as u8);
            for m in [0u32, map] {
                let obj = t.junk(0x40, 5);
                let args = [m, 0x1234_5678 ^ i as u32, 0xabcd_ef00 | i as u32, scale];
                let what = format!("{label}: joypad axis ctor {i} {m:#x}");
                t.check(&what, 0x009e_1420, Conv::Thiscall(obj), &args, |g| {
                    Out::U32(sob::joypad_axis_ctor(g, obj, args))
                });
            }
        }
        for kind in [0u32, 0xa, 0xb, 0xc, 0xd, 0xe, 0xffff_ffff] {
            let dev = t.object(&[0, 0], &[0, kind]);
            for m in [0u32, dev] {
                let obj = t.junk(0x40, 6);
                let args = [m, 7, 0x1ff, SCALES[kind as usize % SCALES.len()]];
                let what = format!("{label}: mouse button ctor {kind:#x} {m:#x}");
                t.check(&what, 0x009e_1910, Conv::Thiscall(obj), &args, |g| {
                    Out::U32(sob::mouse_button_ctor(g, obj, args))
                });
                let src = t.junk(0x40, 7);
                t.w32(src + 0x1c, m);
                let what = format!("{label}: mouse button refresh {kind:#x} {m:#x}");
                t.check(&what, 0x009f_0430, Conv::Thiscall(src), &[], |g| {
                    Out::U32(sob::mouse_button_refresh(g, src))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn set_storage_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(0x0042_fb20, false, 1, 0, 0xf4ee);
        for f in [0x009e_f5b0, 0x009e_f600] {
            for count in [0u32, 1, 7, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
                for (owned, same) in [(true, false), (false, false), (true, true)] {
                    let old = t.junk(0x10, 1);
                    let arr = t.zeros(8);
                    t.w32(arr, if owned { 3 } else { 0x8000_0003 });
                    t.w32(arr + 4, old);
                    let data = if same { old } else { t.junk(0x10, 2) };
                    let what = format!("{label}: set storage {f:#x} {count:#x} {owned} {same}");
                    t.check(&what, f, Conv::Thiscall(arr), &[data, count], |g| {
                        Out::U32(ffx_prender::array::bridge::set_storage(
                            g, arr, data, count, None,
                        ))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn lazy_swaps_match_the_original() {
    assert_eq!(so::SWAP_LAZY.len(), 2);
    for (label, mut t) in Twins::all() {
        t.stub(0x00a3_2ef0, true, 0, 0, 0xc7);
        t.stub(0x0094_9207, false, 1, 0, 0xa7);
        t.stub(0x0043_d6a0, true, 2, 2, 0x5a9);
        for &(f, l) in so::SWAP_LAZY {
            for guard in [0u32, 1, 0xffff_fffe, 0x8000_0003] {
                let g = t.rebase(l.guard);
                t.w32(g, guard);
                let fl = t.rebase(l.flags);
                t.w32(fl, 0xffff_ffff ^ guard);
                let p = t.junk(0x20, 3);
                let what = format!("{label}: lazy swap {f:#x} guard {guard:#x}");
                t.check(&what, f, Conv::Cdecl, &[p], |g| {
                    Out::U32(ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy(
                        g,
                        &so::lazy_of(f),
                        p,
                    ))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn stream_files_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub_import(st::SET_FILE_POINTER_SLOT, 4, 0x10);
        // the archive seek, the loose seek, a file without either
        for (entry, handle, r) in [
            (0u32, 0x1c4u32, 0x10u32),
            (0, 0x1c4, 0xffff_ffff),
            (0, 0xffff_ffff, 0),
        ] {
            t.stub_import(st::SET_FILE_POINTER_SLOT, 4, r);
            let file = t.zeros(8);
            t.w32(file, handle);
            t.w32(file + 4, entry);
            for off in [0u32, 0x40, 0xffff_fff0] {
                let what = format!("{label}: seek {handle:#x} {r:#x} {off:#x}");
                t.check(&what, 0x0060_8120, Conv::Thiscall(file), &[off], |g| {
                    Out::U32(stb::file_seek(g, file, off))
                });
            }
        }
        for r in [0x22u32, 0xffff_ffff] {
            t.stub(st::VFILE_SEEK, true, 2, 2, r);
            let entry = t.junk(0x20, 4);
            let file = t.zeros(8);
            t.w32(file, 0x55);
            t.w32(file + 4, entry);
            let what = format!("{label}: archive seek {r:#x}");
            t.check(&what, 0x0060_8120, Conv::Thiscall(file), &[0x99], |g| {
                Out::U32(stb::file_seek(g, file, 0x99))
            });
        }
        // writes: an archive entry (al only), a loose file
        let file = t.zeros(8);
        t.w32(file, 0x1c8);
        t.w32(file + 4, 0x1234);
        t.check(
            &format!("{label}: write archive"),
            0x0060_8160,
            Conv::Thiscall(file),
            &[1, 2],
            |g| {
                assert!(stb::file_write(g, file, 1, 2, 0).is_none());
                Out::Al(0)
            },
        );
        t.w32(file + 4, 0);
        for r in [1u32, 0, 2] {
            t.stub_import(st::WRITE_FILE_SLOT, 5, r);
            // the stub writes nothing there: the log's pointer is the check
            let what = format!("{label}: write loose {r}");
            t.check_frame(
                &what,
                0x0060_8160,
                Conv::Thiscall(file),
                &[0x3000, 0x40],
                0x14,
                |g| {
                    let written = g.1;
                    Out::U32(stb::file_write(g, file, 0x3000, 0x40, written).unwrap())
                },
            );
        }
        t.reset();
    }
}

#[test]
fn stream_readers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // reset: closed, open but the seek fails, both succeed
        for (open, ok) in [
            (0x1200u32, 0u32),
            (0x1201, 0xab00),
            (0x0101, 0x7701),
            (1, 1),
        ] {
            for word8 in [0u32, 1, 0x1235] {
                let r = t.object(&[0, 0, 0, 1], &[0, open, 0, ok]);
                t.w32(r + 8, word8);
                t.w32(r + 0xc, 0x77);
                let what = format!("{label}: reset {open:#x} {ok:#x} {word8:#x}");
                t.check(&what, 0x0057_6a10, Conv::Thiscall(r), &[], |g| {
                    Out::U32(stb::reader_reset(g, r))
                });
            }
        }
        // open
        t.stub(st::STRING_ASSIGN, true, 1, 1, 0x5a);
        t.stub(st::FILE_SIZE, true, 0, 0, 0x1234);
        for r in [0u32, 5] {
            t.stub(st::OPEN_FILE, true, 5, 5, r);
            let obj = t.junk(0x40, 8);
            let what = format!("{label}: open {r}");
            t.check(&what, 0x0057_6990, Conv::Thiscall(obj), &[0x4000], |g| {
                Out::U32(stb::reader_open(g, obj, 0x4000))
            });
        }
        // the deleting destructors
        t.stub(st::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(st::READER_DTOR, true, 0, 0, 0xd7);
        for flags in [0u32, 1, 2, 0xffff_ffff] {
            let obj = t.junk(0x40, 9);
            let what = format!("{label}: reader file delete {flags:#x}");
            t.check(&what, 0x0057_6820, Conv::Thiscall(obj), &[flags], |g| {
                Out::U32(stb::reader_file_delete(g, obj, flags))
            });
            for name in [0u32, 0x4000, 0x4001] {
                let obj = t.junk(0x20, 10);
                t.w32(obj + 8, name);
                let what = format!("{label}: stream delete {flags:#x} {name:#x}");
                t.check(&what, 0x0060_f100, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(stb::stream_delete(g, obj, flags))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn stream_writers_match_the_original() {
    for (label, mut t) in Twins::all() {
        for ok in [0x101u32, 0x100, 1, 0] {
            let w = t.object(&[0, 0, 0, 0, 0, 2], &[0, 0, 0, 0, 0, ok]);
            t.w32(w + 0xc, 0xffff_fff8);
            let what = format!("{label}: writer write {ok:#x}");
            t.check(
                &what,
                0x009f_06a0,
                Conv::Thiscall(w),
                &[0x5000, 0x10],
                |g| Out::U32(stb::writer_write(g, w, 0x5000, 0x10)),
            );
        }
        t.stub(st::FILE_CTOR, true, 5, 5, 0xfc);
        t.stub(st::READER_CTOR, true, 1, 1, 0xec);
        let obj = t.junk(0x40, 11);
        t.check(
            &format!("{label}: writer file ctor"),
            0x009f_06d0,
            Conv::Thiscall(obj),
            &[0x4100],
            |g| Out::U32(stb::writer_file_ctor(g, obj, 0x4100)),
        );
        // the file's destructor is a bare `ret` and runs as it is
        t.stub(st::WRITER_DTOR, true, 0, 0, 0x7d);
        let obj = t.junk(0x40, 12);
        t.check(
            &format!("{label}: writer file dtor"),
            0x009f_0740,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(stb::writer_file_dtor(g, obj)),
        );
        assert_eq!(t.log_entries(), 4 + 2 + 1);
        t.reset();
    }
}

#[test]
fn registry_deletes_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(st::OPERATOR_DELETE, false, 1, 0, 0xde1);
        let head = t.rebase(st::REGISTRY_HEAD);
        let old = t.r32(head);
        // where `this` sits: absent from an empty list, first, middle, last, absent
        for (case, at) in [
            (0u32, None),
            (3, Some(0)),
            (3, Some(1)),
            (3, Some(2)),
            (3, None),
        ] {
            for flags in [0u32, 1] {
                let entries: Vec<u32> = (0..case).map(|i| t.junk(0x10, i as u8)).collect();
                let this = match at {
                    Some(i) => entries[i],
                    None => t.junk(0x10, 0x40),
                };
                for (i, &e) in entries.iter().enumerate() {
                    t.w32(e + 4, entries.get(i + 1).copied().unwrap_or(0));
                }
                t.w32(head, entries.first().copied().unwrap_or(0));
                let what = format!("{label}: registry delete {case} {at:?} {flags}");
                t.check(&what, 0x009f_4590, Conv::Thiscall(this), &[flags], |g| {
                    Out::U32(stb::registry_delete(g, this, flags))
                });
            }
        }
        t.w32(head, old);
        t.reset();
    }
}

// ---- batch 3: application helpers, script-buffer scanners, map iterators ----

use ffx_pother_b::m5p_framework_input_app::{self as ap, bridge as apb};

impl Twins {
    /// `code` in fresh code memory of both oracles, put into the import
    /// slot `slot`.
    fn import_code(&mut self, slot: u32, code: &[u8]) {
        let c = self.a.alloc_code(code).unwrap();
        assert_eq!(self.b.alloc_code(code).unwrap(), c);
        let at = self.rebase(slot);
        let old = Oracle::read(&mut self.a, at, 4).unwrap();
        self.saved.entry(at).or_insert(old);
        self.w32(at, c);
    }

    /// A recording stub at `sym` (thiscall when `this`) logging `n` words,
    /// popping `pops`, that leaves the `float` `bits` in `ST0`.
    fn stub_float_pops(&mut self, sym: u32, this: bool, n: u32, pops: u32, bits: u32) {
        let at = self.rebase(sym);
        let cell = self.alloc(&bits.to_le_bytes());
        let code = self.stub_code(sym, this, n, pops, 0);
        // the stub ends `mov eax, 0; ret [n]`: load the float before it
        let mut c = vec![0xd9, 0x05];
        c.extend(cell.to_le_bytes()); // fld dword [cell]
        c.push(0xe9);
        let tramp = self.a.alloc_code(&[0; 16]).unwrap();
        assert_eq!(self.b.alloc_code(&[0; 16]).unwrap(), tramp);
        c.extend(code.wrapping_sub(tramp + 11).to_le_bytes()); // jmp code
        self.write(tramp, &c);
        let mut j = vec![0xe9];
        j.extend(tramp.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// `isspace` for the scanners' tests: space, `'\n'` and negative values
/// (so a zero-extended byte would differ).
const ISSPACE: [u8; 27] = [
    0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
    0x83, 0xf8, 0x20, // cmp eax, ' '
    0x74, 0x0c, // je yes
    0x83, 0xf8, 0x0a, // cmp eax, '\n'
    0x74, 0x07, // je yes
    0x85, 0xc0, // test eax, eax
    0x78, 0x03, // js yes
    0x31, 0xc0, // xor eax, eax
    0xc3, // ret
    0xb8, 0x08, 0x00, 0x00, 0x00, // yes: mov eax, 8
    0xc3,
];

/// `sscanf_s(text, fmt, &v)` for the tests: `v = bits` and 1.
fn sscanf_code(bits: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x0c, 0xc7, 0x00]; // mov eax,[esp+0xc]; mov [eax], bits
    c.extend(bits.to_le_bytes());
    c.extend([0xb8, 1, 0, 0, 0, 0xc3]); // mov eax, 1; ret
    c
}

#[test]
fn phyre_exit_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ap::TIMER_TERMINATE, false, 0, 0, 0x7e);
        t.stub(ap::NAMESPACE_CLEAR, false, 0, 0, 0x5c);
        t.stub(ap::POOL_RELEASE, false, 0, 0, 0x15);
        let pool = t.object(&[0, 0, 0, 0], &[]);
        t.stub(ap::THREAD_POOL, false, 0, 0, pool);
        let head = t.rebase(ap::REGISTRY_HEAD);
        let running = t.rebase(ap::G_PHYRE_RUNNING);
        for (up, n) in [(0u8, 2u32), (1, 0), (1, 2), (0x80, 1)] {
            let mut next = 0;
            for _ in 0..n {
                let e = t.object(&[0, 0, 0], &[]);
                t.w32(e + 4, next);
                next = e;
            }
            t.w32(head, next);
            t.w8(running, up);
            let what = format!("{label}: phyre exit {up} {n}");
            t.check(
                &what,
                0x0043_0d60,
                Conv::Cdecl,
                &[],
                |g| match apb::phyre_exit(g) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                },
            );
        }
        t.reset();
    }
}

#[test]
fn app_frames_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ap::EXECUTE_NEXT, true, 0, 0, 0xe8);
        t.stub(ap::SET_VIEWPORT, true, 4, 4, 0x5e7);
        t.stub(ap::BLUR_INIT_SHADERS, true, 2, 2, 0);
        for paused in [0u8, 1] {
            for frames in [0u32, 5] {
                for r in [0u32, 7] {
                    let app = t.object(
                        &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                        &[0, 0, 0, 0, 0, 0, 0, 0, 0, r],
                    );
                    t.w8(app + 0x39, paused);
                    t.w32(app + 0x34, frames);
                    t.w32(app + 0x2c, 0x4444);
                    let what = format!("{label}: scripted frame {paused} {frames} {r}");
                    t.check(&what, 0x0062_c6f0, Conv::Thiscall(app), &[], |g| {
                        Out::U32(apb::scripted_frame(g, app))
                    });
                }
            }
        }
        let g_app = t.rebase(ap::G_APPLICATION);
        let old = t.r32(g_app);
        for app in [0u32, t.junk(0x300, 0x12)] {
            t.w32(g_app, app);
            let vp = t.junk(0x30, 0x13);
            let what = format!("{label}: viewport render {app:#x}");
            t.check(&what, 0x0062_be00, Conv::Thiscall(vp), &[], |g| {
                Out::U32(apb::viewport_render(g, vp))
            });
        }
        let pair = t.zeros(8);
        for app in [0u32, t.junk(0x10, 0x14)] {
            t.w32(g_app, app);
            let what = format!("{label}: app pair {app:#x}");
            t.check(&what, 0x0062_3ba0, Conv::Cdecl, &[pair], |g| {
                Out::U32(apb::app_pair(g, pair))
            });
        }
        t.w32(g_app, old);
        // the motion blur's initialization: no target, an empty one, one
        // whose shaders fail and one whose shaders load
        for (case, r) in [(0u32, 0u32), (1, 0), (2, 3), (2, 0)] {
            t.stub(ap::BLUR_INIT_SHADERS, true, 2, 2, r);
            let mb = t.junk(0x20, 0x15);
            let target = t.zeros(8);
            match case {
                0 => t.w32(mb + 8, 0),
                1 => t.w32(mb + 8, target),
                _ => {
                    t.w32(target, 0x77);
                    t.w32(mb + 8, target);
                }
            }
            let what = format!("{label}: motion blur init {case} {r}");
            t.check(&what, 0x00a1_b7e0, Conv::Thiscall(mb), &[], |g| {
                Out::U32(apb::motion_blur_init(g, mb))
            });
        }
        t.reset();
    }
}

#[test]
fn window_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub_import(ap::SET_PARENT_SLOT, 2, 0x9a);
        t.stub_import(ap::REGISTER_WINDOW_MESSAGE_SLOT, 1, 0xc123);
        t.stub_import(ap::SEND_MESSAGE_SLOT, 4, 0x5e);
        t.stub_import(ap::POST_MESSAGE_SLOT, 4, 1);
        t.stub_import(ap::CALL_NEXT_HOOK_SLOT, 4, 0x4e);
        for host in [0u32, 0x0002_0040] {
            let what = format!("{label}: attach {host:#x}");
            t.check(&what, 0x0062_3d50, Conv::Cdecl, &[host, 0x0003_0050], |g| {
                match apb::attach_to_host(g, host, 0x0003_0050) {
                    Some(v) => Out::U32(v),
                    None => Out::None,
                }
            });
        }
        let drag = t.rebase(ap::G_NC_DRAG);
        let hwnd = t.rebase(ap::G_HWND);
        let hook = t.rebase(ap::G_MOUSE_HOOK);
        t.w32(hwnd, 0x0001_0a2c);
        t.w32(hook, 0x0005_0001);
        for d in [0u8, 1] {
            t.w8(drag, d);
            for (code, wparam) in [
                (0u32, 0x202u32),
                (0, 0x201),
                (1, 0x202),
                (0xffff_ffff, 0x202),
            ] {
                let what = format!("{label}: mouse hook {d} {code:#x} {wparam:#x}");
                t.check(
                    &what,
                    0x0062_50e0,
                    Conv::Stdcall,
                    &[code, wparam, 0x7000],
                    |g| Out::U32(apb::mouse_hook(g, code, wparam, 0x7000)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn script_scanners_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.import_code(ap::ISSPACE_SLOT, &ISSPACE);
        let texts: [&[u8]; 7] = [
            b"  # note\n  cmd x\n",
            b"abc def",
            b"   ",
            b"#only",
            b"",
            b"\n#a\n#b\n x",
            b"\x80\xff y",
        ];
        for (i, text) in texts.iter().enumerate() {
            let at = t.alloc(&[*text, b"\0\0\0\0"].concat());
            let n = text.len() as u32;
            for from in [0u32, 1, n] {
                let buf = t.junk(0x20, 0x16);
                t.w32(buf + 0xc, at + from.min(n));
                t.w32(buf + 0x10, at + n);
                let what = format!("{label}: find space {i} {from}");
                t.check(&what, 0x0062_7810, Conv::Thiscall(buf), &[], |g| {
                    Out::U32(apb::find_space(g, buf))
                });
                let what = format!("{label}: skip blank {i} {from}");
                t.check(&what, 0x0062_cf90, Conv::Thiscall(buf), &[], |g| {
                    Out::U32(apb::skip_blank(g, buf))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn input_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the axis value: no map, no mapping, a mapping
        for (map, mapping, bits) in [
            (0u32, 0u32, 0u32),
            (1, 0, 0),
            (1, 0x5150, 0x3f40_0000),
            (1, 0x5150, 0xffc0_0001),
        ] {
            t.stub(ap::MAP_LOOKUP, true, 1, 1, mapping);
            t.stub_float_pops(ap::MAPPING_VALUE, true, 1, 1, bits);
            let input = t.junk(0x100, 0x17);
            let m = if map == 0 { 0 } else { t.junk(0x10, 0x18) };
            t.w32(input, m);
            let app = t.junk(0x300, 0x19);
            t.w32(app + 0x2b8, input);
            let what = format!("{label}: axis value {map} {mapping:#x} {bits:#x}");
            t.check(&what, 0x0062_82c0, Conv::Thiscall(app), &[0x33], |g| {
                Out::F32(apb::axis_value(g, app, 0x33))
            });
            t.reset();
        }
        // the mouse position
        let input = t.junk(0x100, 0x1a);
        let app = t.junk(0x300, 0x1b);
        t.w32(app + 0x2b8, input);
        t.check(
            &format!("{label}: mouse position"),
            0x0062_bea0,
            Conv::Thiscall(app),
            &[0x140, 0xf0],
            |g| {
                apb::set_mouse_position(g, app, 0x140, 0xf0);
                Out::None
            },
        );
        // taking keys: devices around the limit, missing blocks, keys
        // around the range, set and clear bytes
        let block = t.junk(0x200, 0x1c);
        t.w8(block + 0x18 + 5, 0);
        let input = t.zeros(0x100);
        for d in 0..7u32 {
            t.w32(input + 0x80 + 4 * d, if d == 3 { 0 } else { block });
        }
        t.w32(input + 0x80 + 4 * 7, block);
        let app = t.junk(0x300, 0x1d);
        t.w32(app + 0x2b8, input);
        for device in [0u32, 3, 6, 7, 0xffff_ffff] {
            for key in [0x1bu32, 0x1c, 0x1c + 5, 0x1c + 6, 0x11b, 0x11c, 0xffff_ffff] {
                let what = format!("{label}: take key {device:#x} {key:#x}");
                t.check(
                    &what,
                    0x0062_6a00,
                    Conv::Thiscall(app),
                    &[key, device],
                    |g| Out::U32(apb::take_key(g, app, key, device)),
                );
            }
        }
        // the keyboard's bind
        let kb = t.junk(0x400, 0x1e);
        let list = kb + 0xc;
        t.w32(list, list);
        t.w32(list + 4, list);
        for (id, bound) in [
            (0x1bu32, false),
            (0xffff_fff0, false),
            (0x1c, true),
            (0x1c, false),
            (0x80, false),
        ] {
            let mapping = t.junk(0x30, 0x1f);
            t.w32(mapping + 8, u32::from(bound) * 0x1234);
            let what = format!("{label}: keyboard bind {id:#x} {bound}");
            t.check(
                &what,
                0x0062_5ca0,
                Conv::Thiscall(kb),
                &[mapping, id],
                |g| Out::U32(apb::keyboard_bind(g, kb, mapping, id)),
            );
        }
        // the devices' destructors
        t.stub(ap::CS_RELEASE, true, 0, 0, 0xc5);
        t.stub(ap::EH_VECTOR_DTOR, false, 4, 4, 0xe4);
        let dev = t.junk(0x200, 0x20);
        t.check(
            &format!("{label}: touch dtor"),
            0x0062_1130,
            Conv::Thiscall(dev),
            &[],
            |g| Out::U32(apb::touch_dtor(g, dev)),
        );
        t.check(
            &format!("{label}: pad dtor"),
            0x0062_0ff0,
            Conv::Thiscall(dev),
            &[],
            |g| Out::U32(apb::pad_dtor(g, dev)),
        );
        t.reset();
    }
}

#[test]
fn frame_time_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let start = t.rebase(ap::G_APP_START_TIME);
        // a clock where the order of the sums rounds differently
        for (now, begin, t28, t20) in [
            (0x4a00_0010u32, 1234.5f64, 0x4120_0000u32, 0x3c88_8889u32),
            (0x3f80_0000, 0.0, 0x5a00_0000, 0x3f00_0000),
        ] {
            t.stub_float(ap::GET_TIME, 0, now);
            t.write(start, &begin.to_le_bytes());
            for bits in [
                0x3fc0_0000u32,
                0,
                0x8000_0000,
                0xc000_0000,
                0x7fc0_0000,
                0xffc0_0000,
            ] {
                t.import_code(ap::SSCANF_SLOT, &sscanf_code(bits));
                let app = t.junk(0x40, 0x21);
                t.w32(app + 0x20, t20);
                t.w32(app + 0x28, t28);
                t.frame_len = 4;
                let what = format!("{label}: frame time {now:#x} {bits:#x}");
                t.check_frame(&what, 0x0062_5720, Conv::Cdecl, &[app, 0x6000], 0x14, |g| {
                    let local = g.1;
                    Out::U32(apb::set_frame_time(g, app, 0x6000, local))
                });
            }
            t.reset();
        }
    }
}

/// A `std::map` of five nodes (keys 1..=5 at `+0x10`) under a nil head:
/// `(head, [node 1..=5])`. Root 3; 1 and 5 its children; 2 right of 1, 4
/// left of 5.
fn small_map(t: &mut Twins) -> (u32, [u32; 5]) {
    let head = t.zeros(0x14);
    let n: Vec<u32> = (1..=5u32)
        .map(|k| {
            let x = t.zeros(0x14);
            t.w32(x + 0x10, k);
            x
        })
        .collect();
    let n = [n[0], n[1], n[2], n[3], n[4]];
    t.w8(head + 0xd, 1);
    // node: left, parent, right
    let set = |t: &mut Twins, x: u32, l: u32, p: u32, r: u32| {
        t.w32(x, l);
        t.w32(x + 4, p);
        t.w32(x + 8, r);
    };
    set(t, head, n[0], n[2], n[4]);
    set(t, n[2], n[0], head, n[4]);
    set(t, n[0], head, n[2], n[1]);
    set(t, n[1], head, n[0], head);
    set(t, n[4], n[3], n[2], head);
    set(t, n[3], head, n[4], head);
    (head, n)
}

#[test]
fn map_iterators_match_the_original() {
    for (label, mut t) in Twins::all() {
        let (head, n) = small_map(&mut t);
        let it = t.zeros(4);
        for &at in [head].iter().chain(n.iter()) {
            t.w32(it, at);
            let what = format!("{label}: map next from {at:#x}");
            t.check(&what, 0x0062_14b0, Conv::Thiscall(it), &[], |g| {
                Out::U32(apb::tree_next(g, it))
            });
            t.w32(it, at);
            let what = format!("{label}: map prev from {at:#x}");
            t.check(&what, 0x0062_1580, Conv::Thiscall(it), &[], |g| {
                Out::U32(apb::tree_prev(g, it))
            });
        }
        // clear: the right subtrees go to the (stubbed) erase
        t.stub(ap::TREE_ERASE, true, 1, 1, 0xe5);
        t.stub(ap::OPERATOR_DELETE, false, 1, 0, 0xde1);
        for empty in [false, true] {
            let (head, _) = small_map(&mut t);
            if empty {
                t.w32(head + 4, head);
            }
            let map = t.zeros(8);
            t.w32(map, head);
            t.w32(map + 4, 5);
            let what = format!("{label}: map clear {empty}");
            t.check(&what, 0x0062_6b90, Conv::Thiscall(map), &[], |g| {
                Out::U32(apb::tree_clear(g, map))
            });
        }
        t.reset();
    }
}

// ---- batch 4: device and script teardown, notifiers, input-log helpers ----

use ffx_pother_b::m5p_framework_input_devices::{self as dv, bridge as dvb};

impl Twins {
    /// A COM object: a vtable of `slots` stdcall stubs (tag `0x7100 + i`,
    /// the interface and `words[i] - 1` more words logged and popped, `ecx`
    /// not logged); 0x40 bytes after the vtable pointer.
    fn com_object(&mut self, words: &[u32]) -> u32 {
        let mut vt = Vec::new();
        for (i, &n) in words.iter().enumerate() {
            let s = self.stub_code(0x7100 + i as u32, false, n, n, 0x2000 + i as u32);
            vt.extend(s.to_le_bytes());
        }
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }
}

#[test]
fn device_deletes_match_the_original() {
    assert_eq!(dv::DEVICE_DELETES.len(), 6);
    for (label, mut t) in Twins::all() {
        t.stub(dv::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(dv::CS_RELEASE, true, 0, 0, 0xc5);
        t.stub(dv::EH_VECTOR_DTOR, false, 4, 4, 0xe4);
        for &(f, d) in dv::DEVICE_DELETES {
            for (with_dev, flags) in [
                (false, 0u32),
                (true, 1),
                (true, 0),
                (true, 2),
                (false, 0xffff_ffff),
            ] {
                let obj = t.junk(0x200, f as u8);
                let field = if with_dev {
                    let com = t.com_object(&[1, 1, 1, 1, 1, 1, 1, 1, 1]);
                    if d == dv::Device::Mouse {
                        let wrapper = t.junk(0x10, 0x31);
                        t.w32(wrapper + 4, com);
                        wrapper
                    } else {
                        com
                    }
                } else {
                    0
                };
                t.w32(obj + 0x14, field);
                let what = format!("{label}: device delete {f:#x} {with_dev} {flags:#x}");
                t.check(&what, f, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(dvb::device_delete(g, d, obj, flags))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn script_teardown_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(dv::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(dv::OPERATOR_DELETE, false, 1, 0, 0xde1);
        // the buffer destructor: no count, owned data, borrowed, null data
        for (count, data) in [
            (0u32, 0u32),
            (0, 0x4400),
            (3, 0x4400),
            (0x8000_0003, 0x4400),
            (3, 0),
            (0x8000_0000, 0),
        ] {
            let obj = t.junk(0x30, 0x32);
            t.w32(obj + 4, count);
            t.w32(obj + 8, data);
            let what = format!("{label}: script buffer dtor {count:#x} {data:#x}");
            t.check(&what, 0x0062_0d80, Conv::Thiscall(obj), &[], |g| {
                Out::U32(dvb::script_buffer_dtor(g, obj))
            });
        }
        t.stub(dv::SCRIPT_BUFFER_DTOR, true, 0, 0, 0xbd);
        for text in [0u32, 0x4500] {
            let obj = t.junk(0x30, 0x33);
            t.w32(obj + 0x14, text);
            let what = format!("{label}: script dtor {text:#x}");
            t.check(&what, 0x0062_0d00, Conv::Thiscall(obj), &[], |g| {
                Out::U32(dvb::script_dtor(g, obj))
            });
            for f in dv::SCRIPT_DELETES {
                for flags in [0u32, 1] {
                    let obj = t.junk(0x30, 0x34);
                    t.w32(obj + 0x14, text);
                    let what = format!("{label}: script delete {f:#x} {text:#x} {flags}");
                    t.check(&what, f, Conv::Thiscall(obj), &[flags], |g| {
                        Out::U32(dvb::script_delete(g, obj, flags))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn app_arrays_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(dv::PHYRE_FREE, false, 1, 0, 0xf4ee);
        // 0xe4-byte elements with recording virtual destructors
        let make = |t: &mut Twins, n: u32| {
            let data = t.zeros((0xe4 * n.max(1)) as usize);
            for i in 0..n {
                let e = t.object(&[1], &[0x55]);
                let vt = t.r32(e);
                t.w32(data + 0xe4 * i, vt);
            }
            data
        };
        for (count, n, null) in [
            (0u32, 0u32, false),
            (2, 2, false),
            (0x8000_0002, 2, false),
            (3, 0, true),
        ] {
            let data = if null { 0 } else { make(&mut t, n) };
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, data);
            let what = format!("{label}: release e4 {count:#x} {null}");
            t.check(&what, 0x0062_0990, Conv::Thiscall(arr), &[], |g| {
                Out::U32(dvb::release_e4(g, arr))
            });
        }
        t.stub(dv::ARRAY_E4_RELEASE, true, 0, 0, 0xe4e4);
        for (count, data) in [(1u32, 0x4600u32), (1, 0), (0x8000_0001, 0x4600)] {
            let obj = t.junk(0x20, 0x35);
            t.w32(obj, count);
            t.w32(obj + 4, data);
            let what = format!("{label}: release pair {count:#x} {data:#x}");
            t.check(&what, 0x0062_11b0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(dvb::release_pair(g, obj))
            });
        }
        for (c1, c2) in [(1u32, 1u32), (0x8000_0001, 0), (0, 0x8000_0000)] {
            let app = t.junk(0x100, 0x36);
            t.w32(app + 0xa4, c1);
            t.w32(app + 0x9c, c2);
            let what = format!("{label}: release a4/9c {c1:#x} {c2:#x}");
            t.check(&what, 0x0062_1230, Conv::Thiscall(app), &[], |g| {
                Out::U32(dvb::release_a4_9c(g, app))
            });
        }
        // resize: same count, to 0, a new buffer, a failed allocation
        for (count, n, got, old) in [
            (3u32, 3u32, 0x7000u32, 0x4700u32),
            (3, 0, 0, 0x4700),
            (3, 5, 0x7000, 0x4700),
            (0x8000_0003, 5, 0x7000, 0x4700),
            (3, 5, 0x7000, 0x7000),
            (3, 5, 0x7000, 0),
            (3, 5, 0, 0x4700),
            (0, 0x1c71_c71d, 0x7000, 0x4700),
        ] {
            t.stub(dv::PHYRE_MALLOC_ALIGNED, false, 2, 0, got);
            let arr = t.zeros(8);
            t.w32(arr, count);
            t.w32(arr + 4, old);
            let what = format!("{label}: resize {count:#x} {n:#x} {got:#x} {old:#x}");
            t.check(&what, 0x0062_c1f0, Conv::Thiscall(arr), &[n], |g| {
                Out::U32(dvb::resize_90(g, arr, n))
            });
        }
        t.reset();
    }
}

#[test]
fn notifiers_match_the_original() {
    for (label, mut t) in Twins::all() {
        let holder = t.rebase(dv::NOTIFIERS);
        let old = t.r32(holder);
        // an empty map, then the five-node map with recording values
        let empty = t.zeros(0x14);
        t.w8(empty + 0xd, 1);
        t.w32(empty, empty);
        let (head, nodes) = small_map(&mut t);
        for &n in &nodes {
            let v = t.object(&[0, 0], &[]);
            t.w32(n + 0x10, v);
        }
        for h in [empty, head] {
            t.w32(holder, h);
            t.check(
                &format!("{label}: suspend {h:#x}"),
                0x0062_7850,
                Conv::Cdecl,
                &[],
                |g| {
                    dvb::notify_all(g, 0);
                    Out::None
                },
            );
            t.check(
                &format!("{label}: resume {h:#x}"),
                0x0062_78b0,
                Conv::Cdecl,
                &[],
                |g| {
                    dvb::notify_all(g, 1);
                    Out::None
                },
            );
        }
        assert_eq!(t.log_entries(), 10);
        t.w32(holder, old);
        t.reset();
    }
}

#[test]
fn command_drain_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(dv::CS_RELEASE, true, 0, 0, 0xc5);
        t.stub(dv::SLEEP, false, 1, 0, 0x51);
        // done at once; not done but already equal; one sleep then done
        for (first, equal) in [(1u32, false), (0, true), (0, false)] {
            // the drain step answers `first`, then 1
            let n = t.zeros(4);
            let mut c = vec![0xa1];
            c.extend(n.to_le_bytes()); // mov eax, [n]
            c.extend([0xff, 0x05]);
            c.extend(n.to_le_bytes()); // inc dword [n]
            c.extend([0x85, 0xc0, 0x75, 0x06]); // test eax, eax; jnz +6
            c.push(0xb8);
            c.extend(first.to_le_bytes()); // mov eax, first
            c.push(0xc3);
            c.extend([0xb8, 1, 0, 0, 0, 0xc3]); // mov eax, 1; ret
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(dv::DRAIN_STEP);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            let obj = t.junk(0x40, 0x37);
            t.w32(obj + 0x24, 0x99);
            t.w32(obj + 0x28, if equal { 0x99 } else { 0x98 });
            let what = format!("{label}: drain {first} {equal}");
            t.check(&what, 0x0062_d100, Conv::Thiscall(obj), &[], |g| {
                Out::U32(dvb::drain_commands(g, obj))
            });
        }
        t.reset();
    }
}

#[test]
fn tokens_match_the_original() {
    for (label, mut t) in Twins::all() {
        let texts: [&[u8]; 6] = [
            b"a,bc,d",
            b",x",
            b"abc",
            b"a,",
            b"a\0b,c",
            b"\x80,\x80y\x80",
        ];
        for (i, text) in texts.iter().enumerate() {
            for delim in [b',', 0x80u8, 0] {
                let n = text.len() as u32;
                for (from, to) in [
                    (0u32, n),
                    (0, n.saturating_sub(1)),
                    (1, n),
                    (1, 1),
                    (n, n),
                    (n, 0),
                ] {
                    let at = t.alloc(&[*text, b"\x01\x01\x01\x01"].concat());
                    let what = format!("{label}: token {i} {delim:#x} {from}..{to}");
                    let args = [at + from, at + to, 0x1234_5600 | u32::from(delim)];
                    t.check(&what, 0x0062_8e40, Conv::Cdecl, &args, |g| {
                        Out::U32(dvb::next_token(g, at + from, at + to, delim))
                    });
                }
            }
        }
    }
}

// ---- batch 5: script constructor, render sync, pad index, strongest value,
// range erase, command table, motion blur apply ----

use ffx_pother_b::m5p_framework_input_misc::{self as mi, bridge as mib};

impl Twins {
    /// [`Twins::stub_import`] for a cdecl import (the caller pops).
    fn stub_import_cdecl(&mut self, slot: u32, n: u32, ret: u32) {
        let code = self.stub_code(slot, false, n, 0, ret);
        let at = self.rebase(slot);
        let old = Oracle::read(&mut self.a, at, 4).unwrap();
        self.saved.entry(at).or_insert(old);
        self.w32(at, code);
    }
}

#[test]
fn script_ctor_and_sync_match_the_original() {
    for (label, mut t) in Twins::all() {
        // the array copy fills +4/+8 from the source PArray, adding what
        // was there (so the zeroing before it shows)
        let copy = [
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
            0x8b, 0x10, // mov edx, [eax]
            0x03, 0x11, // add edx, [ecx]
            0x89, 0x11, // mov [ecx], edx
            0x8b, 0x50, 0x04, // mov edx, [eax+4]
            0x03, 0x51, 0x04, // add edx, [ecx+4]
            0x89, 0x51, 0x04, // mov [ecx+4], edx
            0xc2, 0x04, 0x00, // ret 4
        ];
        let code = t.a.alloc_code(&copy).unwrap();
        assert_eq!(t.b.alloc_code(&copy).unwrap(), code);
        let at = t.rebase(mi::ARRAY_ASSIGN);
        let old = Oracle::read(&mut t.a, at, 5).unwrap();
        t.saved.entry(at).or_insert(old);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        for count in [0u32, 7, 0x8000_0007] {
            let src = t.zeros(8);
            t.w32(src, count);
            t.w32(src + 4, 0x5000);
            let obj = t.junk(0x30, 0x41);
            let what = format!("{label}: script ctor {count:#x}");
            t.check(&what, 0x0061_fac0, Conv::Thiscall(obj), &[0x77, src], |g| {
                Out::U32(mib::script_ctor(g, obj, 0x77, src))
            });
        }
        t.reset();
        t.stub(mi::SCOPE_LOCK_ENTER, true, 0, 0, 0x11);
        t.stub(mi::SCOPE_LOCK_LEAVE, true, 0, 0, 0x12);
        t.stub(mi::RENDER_INTERFACE_CTOR, true, 0, 0, 0x13);
        t.stub(mi::ATEXIT, false, 1, 0, 0x14);
        t.stub(mi::RENDER_INTERFACE_SYNC, true, 0, 0, 0x15);
        let guard = t.rebase(mi::RENDER_INTERFACE_GUARD);
        for g in [0u32, 1, 0xffff_fffe] {
            t.w32(guard, g);
            // the stubs write nothing there: the logged `ecx` is the check
            let what = format!("{label}: render sync {g:#x}");
            t.check_frame(&what, 0x0062_d3d0, Conv::Cdecl, &[], 0x18, |g| {
                let lock = g.1;
                Out::U32(mib::render_sync(g, lock))
            });
        }
        t.reset();
    }
}

#[test]
fn pad_index_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let head = t.rebase(mi::DEVICE_LIST);
        let saved = Oracle::read(&mut t.a, head, 8).unwrap();
        // devices of types 2, 3, 2, 2 linked through +4 (the link is the
        // device + 4)
        let devs: Vec<u32> = [2u32, 3, 2, 4, 2]
            .iter()
            .map(|&ty| t.object(&[0, 0], &[0, ty]))
            .collect();
        for (i, &d) in devs.iter().enumerate() {
            let next = devs.get(i + 1).map_or(head, |n| n + 4);
            t.w32(d + 4, next);
        }
        for (case, first) in [(0, devs[0] + 4), (1, head), (2, 0), (3, 4)] {
            t.w32(head, first);
            for this in [devs[0], devs[1], devs[2], devs[3], devs[4], 0, 0x1234] {
                let what = format!("{label}: pad index {case} {this:#x}");
                t.check(&what, 0x0062_8170, Conv::Thiscall(this), &[], |g| {
                    Out::U32(mib::pad_index(g, this))
                });
            }
        }
        t.write(head, &saved);
        t.reset();
    }
}

#[test]
fn strongest_value_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the source value: the float at the source's +0
        let value = [
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
            0xd9, 0x00, // fld dword [eax]
            0xc2, 0x04, 0x00, // ret 4
        ];
        let code = t.a.alloc_code(&value).unwrap();
        assert_eq!(t.b.alloc_code(&value).unwrap(), code);
        let at = t.rebase(mi::SOURCE_VALUE);
        let old = Oracle::read(&mut t.a, at, 5).unwrap();
        t.saved.entry(at).or_insert(old);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let cases: [&[u32]; 6] = [
            &[],
            &[0x3f80_0000],
            &[0x3f80_0000, 0xc000_0000, 0x3fc0_0000],
            &[0x8000_0000, 0x0000_0000],
            &[0x7fc0_0000, 0x3f00_0000, 0xbf00_0000],
            &[0xbf00_0000, 0x3f00_0000, 0x7f80_0000],
        ];
        for (i, vals) in cases.iter().enumerate() {
            let srcs: Vec<u32> = vals.iter().map(|v| t.alloc(&v.to_le_bytes())).collect();
            let table = t.alloc(
                &srcs
                    .iter()
                    .flat_map(|s| s.to_le_bytes())
                    .collect::<Vec<u8>>()
                    .iter()
                    .copied()
                    .chain([0; 4])
                    .collect::<Vec<u8>>(),
            );
            let mapping = t.junk(0x20, 0x42);
            t.w32(
                mapping + 8,
                vals.len() as u32 | if i == 5 { 0x8000_0000 } else { 0 },
            );
            t.w32(mapping + 0xc, table);
            for m in [0, mapping] {
                let what = format!("{label}: strongest {i} {m:#x}");
                t.check(&what, 0x0062_9620, Conv::Thiscall(0x99), &[m], |g| {
                    Out::F32(mib::strongest_value(g, 0x99, m))
                });
            }
        }
        // more than 0xff sources: the last one is the strongest
        let small = t.alloc(&0x3f00_0000u32.to_le_bytes());
        let large = t.alloc(&0x4100_0000u32.to_le_bytes());
        let mut words: Vec<u8> = Vec::new();
        for i in 0..0x101u32 {
            words.extend(if i == 0x100 { large } else { small }.to_le_bytes());
        }
        let table = t.alloc(&words);
        let mapping = t.junk(0x20, 0x48);
        t.w32(mapping + 8, 0x101);
        t.w32(mapping + 0xc, table);
        t.check(
            &format!("{label}: strongest of 0x101"),
            0x0062_9620,
            Conv::Thiscall(0x99),
            &[mapping],
            |g| Out::F32(mib::strongest_value(g, 0x99, mapping)),
        );
        t.reset();
    }
}

#[test]
fn range_erase_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(mi::MAP_CLEAR, true, 0, 0, 0xc1);
        t.stub(mi::MAP_ERASE, true, 2, 2, 0xe1);
        let (head, n) = small_map(&mut t);
        let map = t.zeros(8);
        t.w32(map, head);
        for (first, last) in [
            (n[0], head),
            (n[1], head),
            (n[0], n[3]),
            (n[2], n[2]),
            (n[4], head),
            (n[0], n[0]),
        ] {
            let out = t.zeros(4);
            // the erase stub writes nothing there: the logged pointer is the check
            let what = format!("{label}: erase {first:#x}..{last:#x}");
            t.check_frame(
                &what,
                0x0062_7130,
                Conv::Thiscall(map),
                &[out, first, last],
                0x18,
                |g| {
                    let local = g.1;
                    Out::U32(mib::erase_range(g, map, out, first, last, local))
                },
            );
        }
        t.reset();
    }
}

#[test]
fn command_table_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(mi::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(mi::MEMCPY, false, 3, 0, 0x3c);
        t.stub_import_cdecl(mi::STRNCPY_SLOT, 3, 0x5c);
        let table = t.junk(0x48 * 0x20, 0x43);
        // room left; full with a table; full, empty and small; a failed
        // allocation; a capacity that overflows
        for (count, cap, old, got) in [
            (3u32, 8u32, table, 0x9000u32),
            (8, 8, table, 0x9000),
            (0, 0, 0, 0x9000),
            (4, 4, table, 0),
            (0x2000_0000, 0x2000_0000, table, 0x9000),
            (0x8000_0000, 0x8000_0000, 0, 0x9000),
        ] {
            let buf = if got == 0 {
                0
            } else {
                t.junk(0x48 * 0x20, 0x44)
            };
            t.stub(mi::PHYRE_MALLOC, false, 1, 0, buf);
            let proxy = t.junk(0x30, 0x45);
            t.w32(proxy + 0x14, old);
            t.w32(proxy + 0x18, count);
            t.w32(proxy + 0x1c, cap);
            let what = format!("{label}: add command {count:#x} {cap:#x} {got:#x}");
            t.check(
                &what,
                0x0062_bab0,
                Conv::Thiscall(proxy),
                &[0x4321, 0x6000],
                |g| {
                    mib::add_command(g, proxy, 0x4321, 0x6000);
                    Out::None
                },
            );
        }
        t.reset();
    }
}

#[test]
fn motion_blur_apply_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(mi::BLUR_APPLY, true, 5, 5, 0xab);
        let a = t.junk(0x80, 0x46);
        for (ready, setup, has_target, moved, valid) in [
            (0u8, 0u32, true, true, true),
            (0, 7, true, false, true),
            (1, 0, false, false, false),
            (0, 0, false, false, false),
            (1, 0, true, false, true),
            (1, 0, true, true, false),
            (1, 0, true, false, false),
        ] {
            let mb = t.object(&[0, 2, 0], &[0, setup, 0]);
            t.w8(mb + 0x10, ready);
            let cur = t.zeros(8);
            if valid {
                t.w32(cur, 0x55);
            }
            t.w32(mb + 8, cur);
            t.w32(mb + 0xc, if moved { 0x1234 } else { cur });
            let target = if has_target { t.junk(0x40, 0x47) } else { 0 };
            let args = [a, 0x22, target, 0x44, 0x55];
            let what = format!("{label}: blur apply {ready} {setup} {has_target} {moved} {valid}");
            t.check(&what, 0x00a2_1c40, Conv::Thiscall(mb), &args, |g| {
                Out::U32(mib::motion_blur_apply(g, mb, [a, 0x22, target, 0x44]))
            });
        }
        t.reset();
    }
}

// ---- batch 6: the application script: commands, parser, dispatcher,
// loader; the pad's bind ----

use ffx_pother_b::m5p_framework_input_script::{self as scr, bridge as scb};

/// `strcmp` for the dispatcher's tests (0 equal, 1 different).
const STRCMP: [u8; 31] = [
    0x8b, 0x4c, 0x24, 0x04, // mov ecx, [esp+4]
    0x8b, 0x54, 0x24, 0x08, // mov edx, [esp+8]
    0x8a, 0x01, // l: mov al, [ecx]
    0x3a, 0x02, // cmp al, [edx]
    0x75, 0x0b, // jne diff
    0x84, 0xc0, // test al, al
    0x74, 0x04, // je eq
    0x41, // inc ecx
    0x42, // inc edx
    0xeb, 0xf2, // jmp l
    0x31, 0xc0, 0xc3, // eq: xor eax, eax; ret
    0xb8, 0x01, 0x00, 0x00, 0x00, 0xc3, // diff: mov eax, 1; ret
];

/// A script buffer over `text` (NUL-terminated past its end).
fn script_buffer(t: &mut Twins, text: &[u8]) -> u32 {
    let at = t.alloc(&[text, b"\0\0\0\0"].concat());
    let buf = t.junk(0x30, 0x51);
    t.w32(buf + 0xc, at);
    t.w32(buf + 0x10, at + text.len() as u32);
    buf
}

#[test]
fn command_registration_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(scr::ADD_COMMAND, true, 2, 2, 0xad);
        let proxy = t.junk(0x30, 0x52);
        t.check(
            &format!("{label}: register commands"),
            0x0062_bb60,
            Conv::Thiscall(proxy),
            &[],
            |g| {
                scb::register_commands(g, proxy);
                Out::None
            },
        );
        assert_eq!(t.log_entries(), 16);
        t.reset();
    }
}

#[test]
fn command_parser_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.import_code(0x00b0_c400, &ISSPACE);
        t.stub(scr::PRINTF, false, 3, 0, 0x7f00);
        let texts: [&[u8]; 10] = [
            b"cmd(a b) next(1)",
            b"  cmd(x)",
            b"cmd x",
            b"cmd)(x",
            b")(",
            b"",
            b"#c\n",
            b"a(b)c d(e)",
            b"(x)",
            b"a(b",
        ];
        for (i, text) in texts.iter().enumerate() {
            let buf = script_buffer(&mut t, text);
            let outs = t.zeros(8);
            let what = format!("{label}: find next command {i}");
            t.check(
                &what,
                0x0062_76e0,
                Conv::Thiscall(buf),
                &[outs, outs + 4],
                |g| Out::U32(scb::find_next_command(g, buf, outs, outs + 4)),
            );
        }
        // an end before the parentheses: they are not searched past it
        for cut in [2u32, 4] {
            let buf = script_buffer(&mut t, b"ab(c)d");
            let at = t.r32(buf + 0xc);
            t.w32(buf + 0x10, at + cut);
            let outs = t.zeros(8);
            let what = format!("{label}: find next command cut {cut}");
            t.check(
                &what,
                0x0062_76e0,
                Conv::Thiscall(buf),
                &[outs, outs + 4],
                |g| Out::U32(scb::find_next_command(g, buf, outs, outs + 4)),
            );
        }
        t.reset();
    }
}

#[test]
fn command_dispatch_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.import_code(0x00b0_c400, &ISSPACE);
        t.import_code(scr::STRICMP_SLOT, &STRCMP);
        t.import_code(scr::SSCANF_SLOT, &sscanf_code(2));
        t.stub(scr::PRINTF, false, 4, 0, 0x7f00);
        // the repeat command's own name, read from the image
        let repeat_at = t.rebase(scr::REPEAT_NAME);
        let raw = Oracle::read(&mut t.a, repeat_at, 0x20).unwrap();
        let repeat: Vec<u8> = raw.iter().copied().take_while(|&b| b != 0).collect();
        // three commands: the repeat command, "go", "stop"
        let names: [&[u8]; 3] = [&repeat, b"go", b"stop"];
        let mut table = Vec::new();
        for (i, n) in names.iter().enumerate() {
            let handler = t.stub_code(0x7300 + i as u32, false, 2, 0, 0x30 + i as u32);
            let mut rec = vec![0u8; 0x48];
            rec[..4].copy_from_slice(&handler.to_le_bytes());
            rec[4..4 + n.len()].copy_from_slice(n);
            table.extend(rec);
        }
        let table = t.alloc(&table);
        let text = [&repeat[..], b"(2) go(1) what(3) stop(4)"].concat();
        let script = script_buffer(&mut t, &text);
        t.w32(script + 0x14, table);
        t.w32(script + 0x18, 3);
        t.w32(script + 0x20, 0x0077_0000);
        t.w32(script + 0x24, 0);
        // the repeat, two repeated runs, go, the unknown, stop, the end
        for step in 0..8 {
            t.frame_len = 8;
            let what = format!("{label}: execute step {step}");
            t.check_frame(&what, 0x0062_7480, Conv::Thiscall(script), &[], 0x10, |g| {
                let frame = g.1;
                Out::U32(scb::execute_next(g, script, frame))
            });
        }
        // an empty table: a pending repeat finds nothing, a command is unknown
        let bare = script_buffer(&mut t, b"go(1) junk");
        t.w32(bare + 0x18, 0);
        for pending in [1u32, 0, 0] {
            t.w32(bare + 0x24, pending);
            t.frame_len = 8;
            let what = format!("{label}: execute bare {pending}");
            t.check_frame(&what, 0x0062_7480, Conv::Thiscall(bare), &[], 0x10, |g| {
                let frame = g.1;
                Out::U32(scb::execute_next(g, bare, frame))
            });
        }
        t.reset();
    }
}

#[test]
fn script_loader_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(scr::READER_CTOR, true, 1, 1, 0x1c);
        t.stub(scr::FILE_CLOSE, true, 0, 0, 0xc1);
        t.stub(scr::READER_DTOR, true, 0, 0, 0xd1);
        t.stub(scr::PRINTF, false, 3, 0, 0x7f00);
        for (open, read) in [(0x100u32, 0u32), (1, 0), (1, 3)] {
            t.stub(scr::READER_IS_OPEN, true, 0, 0, open);
            // the read fills the array: 0x10 bytes, borrowed, at 0x6100
            let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
            c.extend([0xc7, 0x00]);
            c.extend(0x8000_0010u32.to_le_bytes()); // mov [eax], count
            c.extend([0xc7, 0x40, 0x04]);
            c.extend(0x6100u32.to_le_bytes()); // mov [eax+4], data
            c.push(0xb8);
            c.extend(read.to_le_bytes()); // mov eax, read
            c.extend([0xc2, 0x08, 0x00]); // ret 8
            let code = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), code);
            let at = t.rebase(scr::READER_READ_ALL);
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            let buf = t.junk(0x30, 0x53);
            t.w32(buf + 4, 0x8000_0010);
            t.w32(buf + 8, 0x6100);
            let what = format!("{label}: script loader {open:#x} {read}");
            t.check_frame(
                &what,
                0x0061_fc20,
                Conv::Thiscall(buf),
                &[0x6200],
                0x38,
                |g| {
                    let reader = g.1;
                    Out::U32(scb::buffer_ctor(g, buf, 0x6200, reader))
                },
            );
        }
        t.reset();
    }
}

#[test]
fn pad_bind_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let pad = t.junk(0x400, 0x54);
        let list = pad + 0xc;
        t.w32(list, list);
        t.w32(list + 4, list);
        for id in [
            0u32,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            0xa,
            0xb,
            0x10,
            0x1a,
            0x1b,
            0x20,
            0xffff_ffff,
        ] {
            for bound in [false, true] {
                let mapping = t.junk(0x30, 0x55);
                t.w32(mapping + 8, u32::from(bound));
                let what = format!("{label}: pad bind {id:#x} {bound}");
                t.check(
                    &what,
                    0x0062_5dd0,
                    Conv::Thiscall(pad),
                    &[mapping, id],
                    |g| Out::U32(scb::pad_bind(g, pad, mapping, id)),
                );
            }
        }
    }
}

// ---- batch 7: application destructor, pad constructor, blur targets ----

use ffx_pother_b::m5p_framework_input_life::{self as lf, bridge as lfb};

#[test]
fn application_dtor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lf::WORLD_RELEASE, true, 0, 0, 0x31);
        t.stub(lf::EH_VECTOR_DTOR, false, 4, 4, 0x32);
        t.stub(lf::RENDERER_DTOR, true, 0, 0, 0x33);
        t.stub(lf::SURFACE_DTOR, true, 0, 0, 0x36);
        t.stub(lf::RELEASE_A4_9C, true, 0, 0, 0x34);
        t.stub(lf::RELEASE_E4, true, 0, 0, 0x35);
        for (name, vcount, count, data) in [
            (0x4000u32, 2u32, 1u32, 0x4100u32),
            (0x4001, 0x8000_0002, 0x8000_0001, 0x4100),
            (0, 0, 1, 0),
        ] {
            let app = t.junk(0x400, 0x61);
            t.w32(app + 0x398, name);
            t.w32(app + 0x2bc, vcount);
            t.w32(app + 0x40, count);
            t.w32(app + 0x44, data);
            let what =
                format!("{label}: application dtor {name:#x} {vcount:#x} {count:#x} {data:#x}");
            t.check(&what, 0x0062_0bc0, Conv::Thiscall(app), &[], |g| {
                Out::U32(lfb::application_dtor(g, app))
            });
        }
        t.reset();
    }
}

#[test]
fn pad_ctor_matches_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::EH_VECTOR_CTOR, false, 5, 5, 0x41);
        t.stub(lf::MEMSET, false, 3, 0, 0x42);
        let head = t.rebase(lf::DEVICE_LIST);
        let saved = Oracle::read(&mut t.a, head, 8).unwrap();
        let tail = t.zeros(8);
        t.w32(head + 4, tail);
        let pad = t.junk(0x400, 0x62);
        t.check(
            &format!("{label}: pad ctor"),
            0x0062_01a0,
            Conv::Thiscall(pad),
            &[0x5150],
            |g| Out::U32(lfb::pad_ctor(g, pad, 0x5150)),
        );
        t.write(head, &saved);
        t.reset();
    }
}

#[test]
fn blur_targets_match_the_original() {
    for (label, mut t) in Twins::all() {
        // per holder: 0 missing, 1 flagged, 2 empty, 3 sized, 4 to resize;
        // then bound, the unbind's and the resize's answers
        let cases = [
            (0u8, 3u8, false, 0u32, 0u32),
            (1, 3, false, 0, 0),
            (2, 3, false, 0, 0),
            (3, 0, false, 0, 0),
            (3, 1, false, 0, 0),
            (3, 2, false, 0, 0),
            (3, 3, false, 0, 0),
            (4, 3, false, 0, 0),
            (4, 3, true, 0, 0),
            (4, 3, true, 7, 0),
            (4, 3, false, 0, 9),
            (4, 3, true, 0, 9),
            (3, 4, true, 0, 0),
            (3, 4, false, 0, 9),
            (4, 4, false, 0, 0),
        ];
        for (i, &(k0, k1, bound, unbind, resize)) in cases.iter().enumerate() {
            t.stub(
                lf::TEXTURE_IS_BOUND,
                true,
                0,
                0,
                if bound { 0x101 } else { 0x100 },
            );
            t.stub(lf::TEXTURE_UNBIND, true, 0, 0, unbind);
            t.stub(lf::TARGET_RESIZE, true, 4, 4, resize);
            let blur = t.junk(0xe00, 0x63);
            for (off, k) in [(0xcd8u32, k0), (0xd4c, k1)] {
                let holder = t.zeros(8);
                let target = t.junk(0x30, 0x64);
                if k == 3 {
                    t.w32(target + 0x1c, 0x280);
                    t.w32(target + 0x20, 0x1e0);
                } else {
                    t.w32(target + 0x1c, 0x280);
                    t.w32(target + 0x20, 0x1e1);
                }
                match k {
                    0 => t.w32(blur + off, 0),
                    1 => {
                        t.w8(holder + 4, 1);
                        t.w32(holder, target);
                        t.w32(blur + off, holder);
                    }
                    2 => t.w32(blur + off, holder),
                    _ => {
                        t.w32(holder, target);
                        t.w32(blur + off, holder);
                    }
                }
            }
            let what = format!("{label}: blur targets {i}");
            t.check(
                &what,
                0x00a2_35f0,
                Conv::Thiscall(blur),
                &[0x280, 0x1e0],
                |g| Out::U32(lfb::check_targets(g, blur, 0x280, 0x1e0)),
            );
            t.reset();
        }
    }
}

// ---- batch 8: the viewport fit, the notifier removal ----

use ffx_pother_b::m5p_framework_input_view::{self as vw, bridge as vwb};

#[test]
fn viewport_fit_matches_the_original() {
    let aspects = [
        0x3f80_0000u32, // 1.0
        0x3fe3_8e39,    // 16:9
        0x3faa_aaab,    // 4:3
        0x4018_0000,    // 2.375
        0x3f00_0000,    // 0.5
        0x0000_0000,    // 0: the quotient is infinite
        0x7fc0_0000,    // NaN
        0xbf80_0000,    // -1
    ];
    let rects = [
        [0u32, 0, 1920, 1080],
        [10, 20, 1280, 1024],
        [0, 0, 1, 1],
        [0, 0, 0, 0],
        [5, 7, 0x8000_0010, 0x8000_0000],
        [0xffff_fff0, 3, 640, 0xffff_ffff],
        [100, 100, 1000, 1001],
    ];
    for (label, mut t) in Twins::all() {
        for &aspect in &aspects {
            for &r in &rects {
                let app = t.object(&[0, 0, 0, 0, 0], &[0, 0, 0, 0, 0x7a]);
                t.w32(app + 0x10, aspect);
                let what = format!("{label}: viewport fit {aspect:#x} {r:x?}");
                t.check(&what, 0x0062_cba0, Conv::Thiscall(app), &r, |g| {
                    Out::U32(vwb::set_viewport_keeping_aspect(g, app, r))
                });
            }
        }
    }
}

#[test]
fn notifier_removal_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // equal_range writes (first, last) = (0x5100, 0x5200) to its out
        // (first = the key itself, so the key's address is checked)
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x4c, 0x24, 0x08, 0x8b, 0x09, 0x89, 0x08]); // [eax] = *[esp+8]
        c.extend([0xc7, 0x40, 0x04]);
        c.extend(0x5200u32.to_le_bytes()); // mov [eax+4], last
        c.extend([0xc2, 0x08, 0x00]); // ret 8
        let code = t.a.alloc_code(&c).unwrap();
        assert_eq!(t.b.alloc_code(&c).unwrap(), code);
        let at = t.rebase(vw::MAP_EQUAL_RANGE);
        let old = Oracle::read(&mut t.a, at, 5).unwrap();
        t.saved.entry(at).or_insert(old);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        t.stub(vw::MAP_ERASE_RANGE, true, 3, 3, 0xe5);
        for key in [0u32, 0x4444] {
            t.frame_len = 8;
            let what = format!("{label}: remove notifier {key:#x}");
            // the original's argument word sits 4 bytes under where the
            // call pushes from; the port's frame block 0x14 under it
            t.check_frame(&what, 0x0062_bce0, Conv::Cdecl, &[key], 0x14, |g| {
                let frame = g.1;
                let key_at = frame + 0x10;
                Mem::write(g, key_at, key);
                Out::U32(vwb::remove_notifier(g, key_at, frame))
            });
        }
        t.reset();
    }
}

// ---- batch 9: the input log's Frames command ----

use ffx_pother_b::m5p_framework_input_frames::{self as fr, bridge as frb};

#[test]
fn frames_match_the_original() {
    for (label, mut t) in Twins::all() {
        let count_at = t.rebase(fr::G_LOG_COUNT);
        let next_at = t.rebase(fr::G_LOG_NEXT);
        let recs_at = t.rebase(fr::G_LOG_RECORDS);
        let saved = [
            Oracle::read(&mut t.a, count_at, 4).unwrap(),
            Oracle::read(&mut t.a, next_at, 4).unwrap(),
            Oracle::read(&mut t.a, recs_at, 4).unwrap(),
        ];
        // records at frames 3, 5, 5, 7, 7, 9
        let frames = [3u32, 5, 5, 7, 7, 9];
        let mut recs = vec![0u8; 0x90 * frames.len()];
        for (i, f) in frames.iter().enumerate() {
            recs[0x90 * i + 4..0x90 * i + 8].copy_from_slice(&f.to_le_bytes());
        }
        let recs = t.alloc(&recs);
        // (count, next, app frame, applies, the app's +0x1c, paused, step)
        let cases = [
            (6u32, 0u32, 7u32, 0x101u32, 0u32, 0u8, 0u32),
            (6, 0, 7, 0x100, 0, 0, 0x55),
            (6, 2, 5, 1, 0x33, 0, 0),
            (6, 0, 1, 1, 0, 0, 0),
            (6, 0, 9, 1, 0, 0, 0),
            (6, 0, 20, 1, 0, 0, 0),
            (6, 6, 20, 1, 0, 0, 0),
            (6, 9, 20, 1, 0, 1, 0),
            (0x8000_0006, 0, 20, 1, 0, 0, 0),
            (0x8000_0006, 3, 7, 0, 0, 1, 0),
            (0, 0, 7, 1, 0, 0, 0),
            (0x8000_0000, 0, 7, 1, 0, 1, 0),
            (0, 0, 7, 1, 0, 1, 0),
            (6, 0, 7, 1, 0, 1, 0),
        ];
        for (i, &(count, next, frame, applies, react, paused, step)) in cases.iter().enumerate() {
            t.stub(fr::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(fr::APPLY_FRAME_INPUT, false, 3, 0, applies);
            t.stub(fr::SCRIPT_STEP, true, 0, 0, step);
            t.w32(count_at, count);
            t.w32(next_at, next);
            t.w32(recs_at, recs);
            let app = t.object(&[0, 0, 0, 0, 0, 0, 0, 0], &[0, 0, 0, 0, 0, 0, 0, react]);
            t.w32(app + 0x34, frame);
            t.w8(app + 0x39, paused);
            t.w8(app + 0x14, 0x77);
            let what = format!("{label}: frames {i}");
            t.check(&what, 0x0062_3660, Conv::Cdecl, &[app, 0x6000], |g| {
                Out::U32(frb::frames(g, app))
            });
            t.reset();
        }
        // a log without records, and an unsorted one (5 then 3)
        let unsorted = t.zeros(0x90 * 2);
        t.w32(unsorted + 4, 5);
        t.w32(unsorted + 0x90 + 4, 3);
        for (i, (records, count, next, frame)) in [(0u32, 6u32, 6u32, 20u32), (unsorted, 2, 0, 5)]
            .into_iter()
            .enumerate()
        {
            t.stub(fr::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(fr::APPLY_FRAME_INPUT, false, 3, 0, 1);
            t.stub(fr::SCRIPT_STEP, true, 0, 0, 0);
            t.w32(count_at, count);
            t.w32(next_at, next);
            t.w32(recs_at, records);
            let app = t.object(&[0, 0, 0, 0, 0, 0, 0, 0], &[]);
            t.w32(app + 0x34, frame);
            t.w8(app + 0x39, 0);
            let what = format!("{label}: frames extra {i}");
            t.check(&what, 0x0062_3660, Conv::Cdecl, &[app, 0x6000], |g| {
                Out::U32(frb::frames(g, app))
            });
            t.reset();
        }
        t.write(count_at, &saved[0]);
        t.write(next_at, &saved[1]);
        t.write(recs_at, &saved[2]);
    }
}

// ---- round left2 (ffx_pother_b::m5p_framework_input_left2) ----

/// `0x0062a0d0`: lengths of every size, a zero vector (0/0), infinities,
/// quiet NaNs; `_CIsqrt` runs as it is (through the trampoline).
#[test]
fn left2_normalize2_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::bridge as l2b;
    let vals: [f32; 12] = [
        3.0,
        -4.0,
        0.0,
        -0.0,
        1.0e-20,
        5.0e30,
        1.234_567_9,
        -7.654_321e-3,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0x0000_0003),
    ];
    for (label, mut t) in Twins::all() {
        let v = t.zeros(8);
        let out = t.junk(8, 1);
        for &x in &vals {
            for &y in &vals {
                t.w32(v, x.to_bits());
                t.w32(v + 4, y.to_bits());
                let what = format!("{label}: normalize2 {x:e} {y:e}");
                t.check(&what, 0x0062_a0d0, Conv::Cdecl, &[out, v], |g| {
                    Out::U32(l2b::normalize2(g, out, v))
                });
            }
        }
    }
}

/// The crash-dump filter: dumps off and on; the exception information
/// compared in the frame.
#[test]
fn left2_crash_filter_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub_import(l2::DUMP_SLOTS[0], 0, 0x1d1d);
        t.stub_import(l2::DUMP_SLOTS[1], 7, 0x44);
        t.stub_import(l2::DUMP_SLOTS[2], 0, 0xffff_ffff);
        t.stub_import(l2::DUMP_SLOTS[3], 1, 0x99);
        t.stub_import(l2::DUMP_SLOTS[4], 1, 1);
        t.stub(l2::MINI_DUMP_WRITE_DUMP, false, 7, 7, 1);
        let enabled = t.rebase(l2::DUMP_ENABLED);
        for on in [0u32, 1, 0x100] {
            t.w32(enabled, on);
            for ptrs in [0u32, 0x0012_3450] {
                let what = format!("{label}: crash filter on {on:#x} pointers {ptrs:#x}");
                t.frame_len = 12;
                t.check_frame(&what, 0x0062_68e0, Conv::Stdcall, &[ptrs], 0x18, |g| {
                    let info = g.1;
                    Out::U32(l2b::crash_filter(g, ptrs, info))
                });
            }
        }
        t.reset();
    }
}

/// `applyScreenMode` (and the jump to it): modes 0-3, sizes of either
/// sign, the icon loaded or not; the `DEVMODEW` and the `RECT` compared in
/// the frame.
#[test]
fn left2_apply_screen_mode_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        for (slot, n, ret) in [
            (l2::SCREEN_SLOTS[0], 2u32, 0u32),
            (l2::SCREEN_SLOTS[1], 2, 0x14cf_0000),
            (l2::SCREEN_SLOTS[2], 3, 0x10cf_0000),
            (l2::SCREEN_SLOTS[3], 2, 1),
            (l2::SCREEN_SLOTS[4], 3, 1),
            (l2::SCREEN_SLOTS[5], 7, 1),
            (l2::SCREEN_SLOTS[6], 1, 0x0040_0000),
            (l2::SCREEN_SLOTS[7], 2, 0x0bad_1c0e),
            (l2::SCREEN_SLOTS[8], 4, 0x5e),
        ] {
            t.stub_import(slot, n, ret);
        }
        // AdjustWindowRect as the real one: the frame around the client
        // rectangle (left and top negative)
        let mut adjust = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        adjust.extend([0xc7, 0x00]);
        adjust.extend((-8i32).to_le_bytes()); // mov dword [eax], -8
        adjust.extend([0xc7, 0x40, 0x04]);
        adjust.extend((-31i32).to_le_bytes()); // mov dword [eax+4], -31
        adjust.extend([0x83, 0x40, 0x08, 0x08]); // add dword [eax+8], 8
        adjust.extend([0x83, 0x40, 0x0c, 0x08]); // add dword [eax+0xc], 8
        adjust.extend([0xb8, 1, 0, 0, 0, 0xc2, 0x0c, 0x00]); // mov eax, 1; ret 0xc
        let code = t.a.alloc_code(&adjust).unwrap();
        assert_eq!(t.b.alloc_code(&adjust).unwrap(), code);
        let slot = t.rebase(l2::SCREEN_SLOTS[4]);
        t.w32(slot, code);
        let settings = t.zeros(0x10);
        let cell = t.rebase(l2::G_SCREEN_SETTINGS);
        t.w32(cell, settings);
        let hwnd = t.rebase(l2::G_HWND);
        t.w32(hwnd, 0x0001_0203);
        let app = t.zeros(0x400);
        t.write(app + 0x39c, &0x8765u16.to_le_bytes());
        let app_cell = t.rebase(l2::G_APPLICATION);
        t.w32(app_cell, app);
        let icon = t.rebase(l2::G_ICON);
        for mode in 0..4u32 {
            for (w, hgt) in [(1280u16, 720u16), (0xfff0, 0x8000)] {
                for icon0 in [0u32, 0x0777_0000] {
                    if mode != 0 && icon0 != 0 {
                        continue;
                    }
                    for f in [0x0062_bf00u32, 0x0062_bef0] {
                        t.write(settings + 4, &hgt.to_le_bytes());
                        t.write(settings + 6, &w.to_le_bytes());
                        t.w32(settings + 8, mode);
                        t.w32(icon, icon0);
                        let what = format!(
                            "{label}: screen mode {f:#x} mode {mode} {w:#x}x{hgt:#x} icon {icon0:#x}"
                        );
                        t.frame_len = if mode < 3 { 0xac } else { 0x9c };
                        t.check_frame(&what, f, Conv::Cdecl, &[], 0xb8, |g| {
                            let frame = g.1;
                            l2b::apply_screen_mode(g, frame);
                            Out::None
                        });
                    }
                }
            }
        }
        t.reset();
    }
}

/// Code answering `values` in turn (then the last one again), popping
/// `pops` words; nothing logged.
fn seq_code(t: &mut Twins, pops: u16, values: &[u32]) -> u32 {
    let mut v = values.to_vec();
    v.push(*values.last().unwrap());
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let tab = t.alloc(&bytes);
    let cell = t.zeros(4);
    let last = values.len() as u32;
    let mut c = vec![0xa1];
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.push(0x3d);
    c.extend(last.to_le_bytes()); // cmp eax, last
    c.extend([0x74, 0x06]); // je +6
    c.extend([0xff, 0x05]);
    c.extend(cell.to_le_bytes()); // inc dword [cell]
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tab.to_le_bytes()); // mov eax, [eax*4 + tab]
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend((pops * 4).to_le_bytes());
    }
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

/// A `jmp` at `sym` to `code`.
fn place_code(t: &mut Twins, sym: u32, code: u32) {
    let at = t.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    let old = Oracle::read(&mut t.a, at, 5).unwrap();
    t.saved.entry(at).or_insert(old);
    t.write(at, &j);
}

#[test]
fn left2_read_all_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        // the known size: (size, pos, terminate, head, old data, block)
        for (size, pos, nul, head, old, block) in [
            (0x30u32, 0x10u32, 1u32, 0u32, 0u32, true),
            (0x30, 0x10, 0x101, 0x21, 0x0055_0000, true),
            (0x30, 0x10, 0, 0x40, 0x0055_0000, true),
            (0x30, 0x10, 1, 0x8000_0002, 0x0055_0000, true),
            (0x30, 0x10, 1, 3, 0x0055_0000, true),
            (0x30, 0x10, 1, 3, 0x0055_0000, false),
            (0x10, 0x10, 0, 3, 0x0055_0000, true),
            (0x10, 0x10, 0, 0x8000_0003, 0x0055_0000, true),
        ] {
            let b = if block { t.junk(0x80, 4) } else { 0 };
            t.stub(l2::READ_CALLS[0], false, 2, 0, b);
            t.stub(l2::READ_CALLS[1], false, 1, 0, 0);
            let stream = t.object(&[0, 0, 0, 0, 0, 2], &[0, 0, 0, 0, 0, 0x18]);
            t.w32(stream + 0x10, size);
            t.w32(stream + 0xc, pos);
            let array = t.zeros(8);
            t.w32(array, head);
            let old_block = if old != 0 { t.junk(0x80, 5) } else { 0 };
            t.w32(array + 4, old_block);
            let what = format!(
                "{label}: readAll size {size:#x} pos {pos:#x} nul {nul:#x} head {head:#x} block {block}"
            );
            // the known size uses the list word for its own count
            t.frame_len = 0;
            t.check_frame(
                &what,
                0x0057_dd60,
                Conv::Thiscall(stream),
                &[array, nul],
                0x18,
                |g| {
                    let list = g.1;
                    Out::U32(l2b::read_all(g, stream, array, nul, list))
                },
            );
            t.reset();
        }
        // the unknown size: chunks, a failed chunk, the resize's error
        for (size, chunks, reads, resize, head, nul) in [
            (u32::MAX, 3usize, &[0x10u32, 0x20, 0][..], 0u32, 0u32, 1u32),
            (0x10, 3, &[0x10, 0][..], 0, 0x40, 0),
            (u32::MAX, 3, &[0][..], 0, 0, 1),
            (u32::MAX, 1, &[0x10, 0x20, 0][..], 0, 0, 1),
            (u32::MAX, 3, &[0x10, 0x20, 0][..], 7, 0, 1),
            (u32::MAX, 3, &[0x10, 0][..], 0, 0x8000_0000, 0),
        ] {
            let mut blocks: Vec<u32> = (0..chunks).map(|_| t.junk(0x808, 6)).collect();
            blocks.push(0);
            let malloc = seq_code(&mut t, 0, &blocks);
            place_code(&mut t, l2::READ_CALLS[2], malloc);
            t.stub(l2::READ_CALLS[1], false, 1, 0, 0);
            t.stub(l2::READ_CALLS[3], true, 1, 1, resize);
            t.stub(l2::READ_CALLS[4], false, 1, 0, 0);
            t.stub(l2::MEMCPY, false, 3, 0, 0);
            let read = seq_code(&mut t, 2, reads);
            let vt = t.alloc(&[[0u8; 20].as_slice(), &read.to_le_bytes()].concat());
            let stream = t.zeros(0x20);
            t.w32(stream, vt);
            // the position (+0xc) one less than the size: -1 remaining
            t.w32(stream + 0x10, size);
            t.w32(
                stream + 0xc,
                if size == u32::MAX {
                    0
                } else {
                    size.wrapping_add(1)
                },
            );
            let data = t.junk(0x80, 7);
            let array = t.zeros(8);
            t.w32(array, head);
            t.w32(array + 4, data);
            let what = format!(
                "{label}: readAll chunks {chunks} reads {reads:?} resize {resize} head {head:#x}"
            );
            t.frame_len = 4;
            t.check_frame(
                &what,
                0x0057_dd60,
                Conv::Thiscall(stream),
                &[array, nul],
                0x18,
                |g| {
                    let list = g.1;
                    Out::U32(l2b::read_all(g, stream, array, nul, list))
                },
            );
            t.reset();
        }
    }
}

#[test]
fn left2_read_value_file_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        // texts 4 and 5 run with a logging sscanf (the lines it is handed)
        let texts: [&[u8]; 6] = [
            b"12\n34\n\0",
            b"\n7\0",
            b"5\0",
            b"\0",
            b"\n\n5\n\0",
            b"5\n\n\n1\0",
        ];
        for open in [0u32, 1] {
            for result in [0u32, 4] {
                for (k, text) in texts.iter().enumerate() {
                    if open == 0 && k > 0 {
                        continue;
                    }
                    let data = t.alloc(text);
                    let len = (text.len() - 1) as u32;
                    let head = if k == 2 { 0x8000_0000 | len } else { len };
                    t.stub(l2::VALUE_FILE[0], true, 1, 1, 0);
                    t.stub(l2::VALUE_FILE[1], true, 0, 0, open);
                    // readAll: the array filled, `result` returned
                    let mut pre = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov dword [eax], head
                    pre.extend(head.to_le_bytes());
                    pre.extend([0xc7, 0x40, 0x04]); // mov dword [eax+4], data
                    pre.extend(data.to_le_bytes());
                    let mut c = pre;
                    let log_part = t.stub_code(l2::VALUE_FILE[2], true, 2, 2, result);
                    c.push(0xe9);
                    let at = t.a.alloc_code(&[0xcc; 32]).unwrap();
                    assert_eq!(t.b.alloc_code(&[0xcc; 32]).unwrap(), at);
                    c.extend(log_part.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp log_part
                    t.write(at, &c);
                    place_code(&mut t, l2::VALUE_FILE[2], at);
                    t.stub(l2::VALUE_FILE[3], true, 0, 0, 0);
                    t.stub(l2::VALUE_FILE[4], true, 0, 0, 0);
                    t.stub(l2::READ_CALLS[1], false, 1, 0, 0);
                    t.stub(l2::PRINTF, false, 3, 0, 0);
                    if k >= 4 {
                        t.stub_import_cdecl(l2::VALUE_FILE[6], 2, 1);
                    }
                    let path = t.alloc(b"value.txt\0");
                    let what = format!("{label}: value file open {open} result {result} text {k}");
                    // the array and the value (the array only once open)
                    t.frame_len = if open != 0 { 12 } else { 0 };
                    t.check_frame(&what, 0x0062_85b0, Conv::Cdecl, &[path], 0x24, |g| {
                        let frame = g.1 - 0x1c;
                        Out::U32(l2b::read_value_file(g, path, frame))
                    });
                    t.reset();
                }
            }
        }
    }
}

#[test]
fn left2_buttons_match_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub(l2::PRINTF, false, 3, 0, 0);
        // every name (as stored and in another case), an unknown one
        let mut names: Vec<u32> = Vec::new();
        for &(s, _) in &l2::BUTTONS {
            let at = t.rebase(s);
            let mut bytes = Oracle::read(&mut t.a, at, 32).unwrap();
            let n = bytes.iter().position(|&b| b == 0).unwrap();
            bytes.truncate(n + 1);
            names.push(t.alloc(&bytes));
            let upper: Vec<u8> = bytes.iter().map(u8::to_ascii_uppercase).collect();
            names.push(t.alloc(&upper));
        }
        names.push(t.alloc(b"Nothing\0"));
        for pad_set in [true, false] {
            let pad = t.zeros(0x40);
            let input = t.zeros(0x40);
            t.w32(input + 0x38, if pad_set { pad } else { 0 });
            let app = t.zeros(0x2c0);
            t.w32(app + 0x2b8, input);
            for &name in &names {
                let what = format!("{label}: buttonPress pad {pad_set} name {name:#x}");
                t.check(&what, 0x0062_2d40, Conv::Cdecl, &[app, name], |g| {
                    Out::U32(l2b::button_press(g, app, name))
                });
                for value in [0u32, 0x1234_5602] {
                    let what =
                        format!("{label}: buttonSet pad {pad_set} name {name:#x} {value:#x}");
                    t.check(&what, 0x0062_c730, Conv::Cdecl, &[app, name, value], |g| {
                        Out::U32(l2b::button_set(g, app, name, value))
                    });
                }
            }
            t.check(
                &format!("{label}: buttonPress no name"),
                0x0062_2d40,
                Conv::Cdecl,
                &[app, 0],
                |g| Out::U32(l2b::button_press(g, app, 0)),
            );
        }
        for f in [0x0062_2d40u32, 0x0062_c730] {
            t.check(
                &format!("{label}: {f:#x} no app"),
                f,
                Conv::Cdecl,
                &[0, names[0], 1],
                |g| {
                    Out::U32(if f == 0x0062_2d40 {
                        l2b::button_press(g, 0, names[0])
                    } else {
                        l2b::button_set(g, 0, names[0], 1)
                    })
                },
            );
        }
        t.reset();
    }
}

/// A node at `node` owned by `owner` (or not) in a ring with two others.
fn owned_node(t: &mut Twins, node: u32, owner: u32) {
    let a = t.junk(0x30, 1);
    let b = t.junk(0x30, 2);
    t.w32(node, a);
    t.w32(node + 4, b);
    t.w32(a + 4, node);
    t.w32(b, node);
    t.w32(node + 8, owner);
    t.w32(node + 0x28, 0x1111);
    t.w32(node + 0x2c, 0x2222);
}

#[test]
fn left2_release_input_map_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::bridge as l2b;
    for (label, mut t) in Twins::all() {
        for with_map in [true, false] {
            let this = t.junk(0xa0, 3);
            let map = t.zeros(0x120);
            for k in 0..5 {
                owned_node(
                    &mut t,
                    map + 0x18 + 0x30 * k,
                    if k % 2 == 0 { map } else { 0x55 },
                );
            }
            t.w32(this + 0x9c, if with_map { map } else { 0 });
            for i in 0..18u32 {
                let dev = if i % 3 == 1 { 0 } else { t.zeros(0x150) };
                t.w32(this + 0x38 + 4 * i, dev);
                if dev != 0 {
                    for k in 0..4 {
                        let owner = if (i + k) % 2 == 0 { dev } else { 0x66 };
                        owned_node(&mut t, dev + 0x6c + 0x30 * k, owner);
                    }
                }
            }
            let what = format!("{label}: input map release map {with_map}");
            t.check(&what, 0x0062_d170, Conv::Thiscall(this), &[], |g| {
                l2b::release_input_map(g, this);
                Out::None
            });
        }
        t.reset();
    }
}

/// A circular `{next, prev}` list at `head` with `n` nodes (0x20 bytes);
/// node `i`'s `+0x18` is `down(i)`.
fn touch_list(t: &mut Twins, head: u32, n: u32, down: impl Fn(u32) -> u32) -> Vec<u32> {
    let mut all = vec![head];
    for i in 0..n {
        let node = t.zeros(0x20);
        t.w32(node + 0x18, down(i));
        all.push(node);
    }
    for i in 0..all.len() {
        t.w32(all[i], all[(i + 1) % all.len()]);
        t.w32(all[i] + 4, all[(i + all.len() - 1) % all.len()]);
    }
    all
}

#[test]
fn left2_touch_update_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub(l2::TOUCH_LOCK[0], true, 0, 0, 0x11);
        t.stub(l2::TOUCH_LOCK[1], true, 0, 0, 0x12);
        for (active, pending, free) in [
            (0u32, 0u32, 0u32),
            (3, 0, 1),
            (0, 2, 0),
            (4, 3, 2),
            (1, 1, 0),
        ] {
            let mut words = vec![0u32; 7];
            words[6] = 0;
            let dev = t.object(&words, &[]);
            let this = t.junk(0x190, 9);
            // the device object: its vtable from `dev`
            let vt = t.r32(dev);
            t.w32(this, vt);
            touch_list(&mut t, this + 0x164, active, |i| i % 3);
            touch_list(&mut t, this + 0x16c, pending, |i| i + 1);
            touch_list(&mut t, this + 0x174, free, |_| 0);
            let what =
                format!("{label}: touch update active {active} pending {pending} free {free}");
            t.check(&what, 0x0062_de50, Conv::Thiscall(this), &[], |g| {
                Out::U32(l2b::touch_update(g, this))
            });
        }
        t.reset();
    }
}

#[test]
fn left2_particle_system_ctor_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        for flags in [0u32, 0x80, 0xffff_ff7f] {
            // the sampler's constructor leaves its flags word as given
            let mut pre = vec![0xc7, 0x41, 0x1c];
            pre.extend(flags.to_le_bytes()); // mov dword [ecx+0x1c], flags
            let s = t.stub_code(l2::PARTICLE_CALLS[1], true, 0, 0, 0);
            let mut c = pre;
            let at = t.a.alloc_code(&[0xcc; 16]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 16]).unwrap(), at);
            c.push(0xe9);
            c.extend(s.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
            t.write(at, &c);
            place_code(&mut t, l2::PARTICLE_CALLS[1], at);
            t.stub(l2::PARTICLE_CALLS[0], true, 0, 0, 0);
            let this = t.junk(0x3200, 4);
            let what = format!("{label}: particle system ctor flags {flags:#x}");
            t.check(&what, 0x009f_c9e0, Conv::Thiscall(this), &[], |g| {
                Out::U32(l2b::particle_system_ctor(g, this))
            });
            t.reset();
        }
    }
}

#[test]
fn left2_dinput_pad_update_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub(l2::DINPUT_PAD[1], true, 0, 0, 3);
        t.stub(l2::DINPUT_PAD[2], true, 0, 0, 0);
        t.stub(l2::PRINTF, false, 3, 0, 0);
        let povs = [
            0u32,
            4500,
            9000,
            13500,
            18000,
            22500,
            27000,
            31500,
            1,
            0xffff_ffff,
        ];
        // (connected answer, the flag, poll fails, acquire answers)
        for (k, &pov) in povs.iter().enumerate() {
            for (conn, flag, poll_fails) in [
                (0u32, 0u8, false),
                (0, 1, false),
                (5, 1, true),
                (5, 0, false),
            ] {
                // the DirectInput object: vt[5] (this, guid) answers conn
                let di_vt: Vec<u32> = (0..6)
                    .map(|i| t.stub_code(0x7200 + i, false, 2, 2, conn))
                    .collect();
                let di_vt = t.alloc(
                    &di_vt
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect::<Vec<_>>(),
                );
                let di = t.zeros(8);
                t.w32(di, di_vt);
                let cell = t.rebase(l2::DINPUT_PAD[0]);
                t.w32(cell, di);
                // the device: Acquire (slot 7) answers INPUTLOST twice;
                // GetDeviceState (slot 9) fills the state; Poll (slot 25)
                let mut words = vec![1u32; 26];
                words[9] = 3;
                let dev = t.com_object(&words);
                let vt = t.r32(dev);
                let acquire = seq_code(&mut t, 1, &[l2::DIERR_INPUTLOST, l2::DIERR_INPUTLOST, 0]);
                let poll = t.stub_code(
                    0x7300,
                    false,
                    1,
                    1,
                    if poll_fails {
                        0x8000_0001
                    } else {
                        u32::from(conn != 0)
                    },
                );
                // GetDeviceState: copy 0x110 bytes from `src` into [esp+0xc]
                let src = t.junk(0x110, k as u8);
                t.w32(src + 0x20, pov);
                let mut c = vec![0x56, 0x57]; // push esi; push edi
                c.extend([0x8b, 0x7c, 0x24, 0x14]); // mov edi, [esp+0x14]
                c.push(0xbe);
                c.extend(src.to_le_bytes()); // mov esi, src
                c.push(0xb9);
                c.extend(0x44u32.to_le_bytes()); // mov ecx, 0x44
                c.extend([0xf3, 0xa5, 0x5f, 0x5e]); // rep movsd; pop edi; pop esi
                c.extend([0x31, 0xc0, 0xc2, 0x0c, 0x00]); // xor eax, eax; ret 0xc
                let get = t.a.alloc_code(&c).unwrap();
                assert_eq!(t.b.alloc_code(&c).unwrap(), get);
                t.w32(vt + 7 * 4, acquire);
                t.w32(vt + 9 * 4, get);
                t.w32(vt + 25 * 4, poll);
                let this = t.junk(0x3b0, 5);
                t.w32(this + 0x14, dev);
                t.w8(this + 0x69, flag);
                let what = format!(
                    "{label}: dinput pad pov {pov} conn {conn} flag {flag} poll fails {poll_fails}"
                );
                t.frame_len = 0x110;
                t.check_frame(&what, 0x0062_d9d0, Conv::Thiscall(this), &[], 0x11c, |g| {
                    let state = g.1;
                    l2b::dinput_pad_update(g, this, state);
                    Out::None
                });
            }
        }
        let this = t.junk(0x3b0, 6);
        t.w32(this + 0x14, 0);
        t.check(
            &format!("{label}: dinput pad no device"),
            0x0062_d9d0,
            Conv::Thiscall(this),
            &[],
            |g| {
                l2b::dinput_pad_update(g, this, 0);
                Out::None
            },
        );
        t.reset();
    }
}

#[test]
fn left2_xinput_pad_update_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub(l2::DINPUT_PAD[1], true, 0, 0, 2);
        t.stub(l2::DINPUT_PAD[2], true, 0, 0, 0);
        t.stub(l2::PRINTF, false, 4, 0, 0);
        // XInputSetState: logs the pad and the vibration word it is handed
        let pre = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x00, 0x89, 0x44, 0x24, 0x08]; // [esp+8] = *[esp+8]
        let set = t.stub_code(l2::XINPUT[1], false, 2, 2, 0);
        let at = t.a.alloc_code(&[0xcc; 32]).unwrap();
        assert_eq!(t.b.alloc_code(&[0xcc; 32]).unwrap(), at);
        let mut c = pre;
        c.push(0xe9);
        c.extend(set.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        t.write(at, &c);
        place_code(&mut t, l2::XINPUT[1], at);
        let mut rng = Rng(0x1234_5679);
        // (answer, packet, stored packet, flag, motors)
        for (k, (answer, packet, stored, flag)) in [
            (0u32, 7u32, 6u32, 0u8),
            (0, 7, 6, 1),
            (0, 7, 7, 1),
            (0x48f, 0, 7, 1),
            (0x48f, 0, 7, 0),
            (0x48f, 0, 0, 1),
            (0, 9, 8, 1),
            (0, 10, 8, 0),
            (0, 11, 8, 2),
            (0x48f, 0, 7, 2),
        ]
        .into_iter()
        .enumerate()
        {
            // XInputGetState: writes a state into [esp+8], answers `answer`
            let mut st = vec![0u8; 16];
            st[..4].copy_from_slice(&packet.to_le_bytes());
            for b in &mut st[4..] {
                *b = rng.next() as u8;
            }
            if k == 6 {
                // the sticks' extremes, triggers 0
                st[6] = 0;
                st[7] = 0;
                st[8..10].copy_from_slice(&0x8000u16.to_le_bytes());
                st[10..12].copy_from_slice(&0x7fffu16.to_le_bytes());
                st[12..14].copy_from_slice(&0xff81u16.to_le_bytes());
                st[14..16].copy_from_slice(&0x007fu16.to_le_bytes());
            }
            if k == 7 {
                st[6] = 0;
                st[7] = 5;
            }
            let src = t.alloc(&st);
            // a failing call writes nothing
            let words = if answer == 0 { 4u8 } else { 0 };
            let mut c = vec![0x56, 0x57]; // push esi; push edi
            c.extend([0x8b, 0x7c, 0x24, 0x10]); // mov edi, [esp+0x10]
            c.push(0xbe);
            c.extend(src.to_le_bytes()); // mov esi, src
            c.extend([0xb9, words, 0, 0, 0, 0xf3, 0xa5, 0x5f, 0x5e]); // mov ecx, 4; rep movsd; pop edi; pop esi
            c.push(0xb8);
            c.extend(answer.to_le_bytes()); // mov eax, answer
            c.extend([0xc2, 0x08, 0x00]); // ret 8
            let get = t.a.alloc_code(&c).unwrap();
            assert_eq!(t.b.alloc_code(&c).unwrap(), get);
            place_code(&mut t, l2::XINPUT[0], get);
            let this = t.junk(0x3d0, k as u8);
            t.w32(this + 0x3b0, 3);
            t.w32(this + 0x3b4, stored);
            t.w8(this + 0x69, flag);
            t.w8(this + 0x58, (rng.next() & 0xff) as u8);
            t.w8(this + 0x59, if k == 1 { 0xff } else { 0 });
            let what = format!("{label}: xinput pad case {k}");
            t.frame_len = 4;
            t.check_frame(&what, 0x0062_dba0, Conv::Thiscall(this), &[], 0x2c, |g| {
                let frame = g.1;
                l2b::xinput_pad_update(g, this, frame);
                Out::None
            });
            t.reset();
            // put the SetState stub back after the reset
            place_code(&mut t, l2::XINPUT[1], at);
            t.stub(l2::DINPUT_PAD[1], true, 0, 0, 2);
            t.stub(l2::DINPUT_PAD[2], true, 0, 0, 0);
            t.stub(l2::PRINTF, false, 4, 0, 0);
        }
    }
}

#[test]
fn left2_path_point_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::bridge as l2b;
    let floats: [u32; 8] = [
        0x3f80_0000,
        0xc0a0_0000,
        0x4b80_0001,
        0x0000_0001,
        0x7f7f_ffff,
        0xff80_0000,
        0x7fc0_1234,
        0x3eaa_aaab,
    ];
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_ace1);
        for looping in [0u8, 1, 0x80] {
            for count in [1u32, 2, 5, 0x8000_0003] {
                let n = count & 0x7fff_ffff;
                let pts = t.zeros(16 * n as usize + 32) + 16;
                for i in 0..(4 * n + 4) {
                    let v = floats[(rng.next() % 8) as usize];
                    t.w32(pts - 16 + 4 * i, v);
                }
                let this = t.zeros(0x20);
                t.w32(this + 4, count);
                t.w32(this + 8, pts);
                t.w8(this + 0x18, looping);
                for index in [0u32, 1, n - 1, n, n + 3, u32::MAX, 0xffff_fffe, 0x8000_0000] {
                    let out = t.junk(16, 3);
                    let what = format!(
                        "{label}: path point loop {looping} count {count:#x} index {index:#x}"
                    );
                    t.check(
                        &what,
                        0x0054_b080,
                        Conv::Thiscall(this),
                        &[out, index],
                        |g| Out::U32(l2b::path_point(g, this, out, index)),
                    );
                }
                // the output over the points (the original reads after
                // each store)
                if n >= 2 && looping == 0 {
                    for (index, out) in [(u32::MAX, pts), (n, pts + 16 * (n - 1))] {
                        let what = format!(
                            "{label}: path point aliased count {count:#x} index {index:#x}"
                        );
                        t.check(
                            &what,
                            0x0054_b080,
                            Conv::Thiscall(this),
                            &[out, index],
                            |g| Out::U32(l2b::path_point(g, this, out, index)),
                        );
                    }
                }
            }
        }
        t.reset();
    }
}

/// A gameplay path with `n` keys (28 bytes, a unit-ish quaternion at
/// `+0xc`), its length and a 3x4 matrix.
#[allow(clippy::cast_precision_loss)] // small test values
fn gameplay_path(t: &mut Twins, n: u32, length: f32, seed: u32) -> u32 {
    let mut rng = Rng(seed);
    let keys = t.zeros(28 * n as usize + 56);
    for k in 0..n + 2 {
        let a = (rng.next() % 1000) as f32 / 500.0 - 1.0;
        let b = (rng.next() % 1000) as f32 / 500.0 - 1.0;
        let q = [a * 0.5, b * 0.5, 0.25, 0.75];
        for (i, v) in q.iter().enumerate() {
            t.w32(keys + 28 * k + 0xc + 4 * i as u32, v.to_bits());
        }
    }
    let m = t.zeros(0x30);
    for i in 0..12u32 {
        t.w32(m + 4 * i, ((i as f32) * 0.37 - 1.1).to_bits());
    }
    let this = t.zeros(0x30);
    t.w32(this + 0x14, m);
    t.w32(this + 0x20, length.to_bits());
    t.w32(this + 0x24, n);
    t.w32(this + 0x28, keys);
    this
}

#[test]
#[allow(clippy::cast_precision_loss)] // small test values
fn left2_path_rotation_and_transform_match_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::bridge as l2b;
    for (label, mut t) in Twins::all() {
        for (n, length) in [(0u32, 2.0f32), (1, 2.0), (4, 2.0), (4, 3.5), (6, 1.0)] {
            let this = gameplay_path(&mut t, n, length, 0x55 + n);
            for tv in [0.0f32, 0.4, 1.0, 1.99, 2.0, 0.123_456, 3.0, -1.0] {
                // past the keys only where the extra keys are there
                if n > 0 && tv / length * n as f32 > n as f32 + 0.9 {
                    continue;
                }
                let out = t.junk(0x30, 7);
                let what = format!("{label}: path rotation n {n} length {length} t {tv}");
                t.frame_len = if n == 0 { 0 } else { 0x30 };
                t.check_frame(
                    &what,
                    0x0054_b550,
                    Conv::Thiscall(this),
                    &[out, tv.to_bits()],
                    0x48,
                    |g| {
                        let frame = g.1;
                        Out::U32(l2b::path_rotation(g, this, out, tv.to_bits(), frame))
                    },
                );
                let what = format!("{label}: path transform n {n} length {length} t {tv}");
                t.frame_len = 0x40;
                t.check_frame(
                    &what,
                    0x0054_b690,
                    Conv::Thiscall(this),
                    &[out, tv.to_bits()],
                    0x80,
                    |g| {
                        let frame = g.1;
                        Out::U32(l2b::path_transform(g, this, out, tv.to_bits(), frame))
                    },
                );
            }
        }
        t.reset();
    }
}

/// Code (thiscall, 2 stack words) writing `words` at its first argument,
/// answering that argument.
fn writer_code(t: &mut Twins, words: &[u32]) -> u32 {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    for (i, w) in words.iter().enumerate() {
        c.extend([0xc7, 0x40, (4 * i) as u8]);
        c.extend(w.to_le_bytes()); // mov dword [eax+4i], w
    }
    c.extend([0xc2, 0x08, 0x00]); // ret 8
    let at = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), at);
    at
}

#[test]
fn left2_path_frame_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    let f = |v: f32| v.to_bits();
    for (label, mut t) in Twins::all() {
        let dirs: [[f32; 3]; 5] = [
            [0.0, 0.0, 1.0],
            [0.6, 0.0, 0.8],
            [0.3, -0.5, 0.81],
            [1.0, 0.0, 0.0],
            [f32::NAN, 0.0, 1.0],
        ];
        let ups: [[f32; 3]; 3] = [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.1, 0.9, -0.2]];
        for (i, dv) in dirs.iter().enumerate() {
            for (j, uv) in ups.iter().enumerate() {
                let d = writer_code(&mut t, &[f(dv[0]), f(dv[1]), f(dv[2]), f(7.0)]);
                let mut m = vec![f(uv[0]), f(uv[1]), f(uv[2]), f(9.0)];
                for k in 0..12u16 {
                    m.push(f(f32::from(k) * 0.5));
                }
                let tr = writer_code(&mut t, &m);
                let p = writer_code(&mut t, &[f(1.5), f(-2.5), f(3.25), f(4.0)]);
                place_code(&mut t, l2::PATH_CALLS[5], d);
                place_code(&mut t, l2::PATH_CALLS[7], tr);
                place_code(&mut t, l2::PATH_CALLS[6], p);
                let this = t.zeros(0x30);
                let out = t.junk(0x40, 3);
                let tv = f(0.75);
                let what = format!("{label}: path frame dir {i} up {j}");
                t.frame_len = 0x7c;
                t.check_frame(
                    &what,
                    0x0054_b320,
                    Conv::Thiscall(this),
                    &[tv, out],
                    0x8c,
                    |g| {
                        let frame = g.1;
                        Out::U32(l2b::path_frame(g, this, tv, out, frame))
                    },
                );
                t.reset();
            }
        }
    }
}

/// The subtree of `nodes[range]` under `links.0` (leaves pointing at the
/// head `links.1`); its root.
fn rb_build(
    t: &mut Twins,
    nodes: &[u32],
    range: (usize, usize),
    links: (u32, u32),
    level: u32,
    red: &[u32],
) -> u32 {
    let (lo, hi) = range;
    let (parent, head) = links;
    if lo >= hi {
        return head;
    }
    let mid = usize::midpoint(lo, hi);
    let node = nodes[mid];
    t.w32(node + 4, parent);
    t.w8(node + 0xc, u8::from(!red.contains(&level)));
    let l = rb_build(t, nodes, (lo, mid), (node, head), level + 1, red);
    let r = rb_build(t, nodes, (mid + 1, hi), (node, head), level + 1, red);
    t.w32(node, l);
    t.w32(node + 8, r);
    node
}

/// A perfect red-black tree of `2^depth - 1` nodes in MSVC's layout
/// (`{left, parent, right, colour, nil}`, the head's parent the root, its
/// left the minimum, its right the maximum); levels in `red` are red.
/// Returns the tree object `{head, size}` and the nodes in order.
fn rb_tree(t: &mut Twins, depth: u32, red: &[u32]) -> (u32, Vec<u32>) {
    let head = t.zeros(0x20);
    t.w8(head + 0xc, 1);
    t.w8(head + 0xd, 1);
    let n = (1u32 << depth) - 1;
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
    let root = rb_build(t, &nodes, (0, n as usize), (head, head), 0, red);
    t.w32(head + 4, root);
    t.w32(head, nodes[0]);
    t.w32(head + 8, nodes[n as usize - 1]);
    let this = t.zeros(8);
    t.w32(this, head);
    t.w32(this + 4, n);
    (this, nodes)
}

#[test]
fn left2_map_erase_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_2468);
        for (depth, red) in [
            (5u32, vec![]),
            (5, vec![1, 3]),
            (4, vec![2]),
            (5, vec![4]),
            (3, vec![1]),
            (6, vec![]),
            (6, vec![]),
            (6, vec![2, 4]),
            (7, vec![5]),
        ] {
            let (this, nodes) = rb_tree(&mut t, depth, &red);
            t.stub(l2::MAP_TREE[5], false, 1, 0, 0);
            let mut order = nodes.clone();
            for i in (1..order.len()).rev() {
                let j = (rng.next() as usize) % (i + 1);
                order.swap(i, j);
            }
            for (k, &node) in order.iter().enumerate() {
                let out = t.junk(4, 9);
                let what = format!("{label}: map erase depth {depth} red {red:?} step {k}");
                // the iterator: the original's own argument word, the
                // port's scratch word (no callee logs its address)
                t.check(
                    &what,
                    0x0062_71d0,
                    Conv::Thiscall(this),
                    &[out, node],
                    |g| {
                        let slot = g.1;
                        ffx_bridge::Mem::write(g, slot, node);
                        Out::U32(l2b::map_erase(g, this, out, slot))
                    },
                );
            }
            t.reset();
        }
    }
}

#[test]
#[allow(clippy::needless_range_loop)] // the key is also an address offset
fn left2_log_keyboard_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        // sprintf_s logs its first words; the writer logs (data, n)
        // (7 words: a mapped line has 7, the stack past them is junk)
        t.stub_import_cdecl(l2::KEY_LOG[1], 7, 0);
        let buf = t.rebase(l2::KEY_LOG[0]);
        t.write(buf, b"abcde\0");
        t.stub(l2::KEY_LOG[2], true, 2, 2, 0);
        let mapping_type = t.rebase(l2::KEY_LOG[4]);
        let mut rng = Rng(0x0bad_cafe);
        for (writer, devices, with_maps) in [
            (false, 0u32, true),
            (true, 0x7f, true),
            (true, 0x15, false),
            (true, 0x40, true),
        ] {
            let this = t.zeros(0x2c0);
            let w = if writer { t.zeros(0x10) } else { 0 };
            t.w32(this + 0x29c, w);
            t.w32(this + 0x34, 0x0055_0000);
            let input = t.zeros(0xa0);
            t.w32(this + 0x2b8, input);
            for i in 0..7u32 {
                if devices & (1 << i) == 0 {
                    continue;
                }
                // keys change where the table differs from the last state
                let mut table = vec![0u8; 0x120];
                let dev = t.zeros(0x220);
                for key in 0x1c..0x11cusize {
                    let r = rng.next();
                    let last = (r & 1) as u8;
                    table[key] = if r & 0xf0 == 0 { last ^ 1 } else { last };
                    if r & 0xf00 == 0 {
                        table[key] = 0x80;
                    }
                    if (0x20..0x23).contains(&key) {
                        table[key] = if key == 0x21 { 0x80 } else { last ^ 1 };
                    }
                    t.w8(dev + 0xfc + key as u32, last);
                }
                let tab = t.alloc(&table);
                let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x80]; // mov eax, [esp+4]; movzx eax, byte [eax+tab]
                c.extend(tab.to_le_bytes());
                c.extend([0xc2, 0x04, 0x00]);
                let code = t.a.alloc_code(&c).unwrap();
                assert_eq!(t.b.alloc_code(&c).unwrap(), code);
                let vt = t.alloc(&[[0u8; 12].as_slice(), &code.to_le_bytes()].concat());
                t.w32(dev, vt);
                t.w32(input + 0x80 + 4 * i, dev);
            }
            if with_maps {
                // two maps, the second with mappings: of the type (keys
                // 0x20/0x21 and 0x22/0x20), of another type
                let mut lists = Vec::new();
                for m in 0..2u32 {
                    let mut items = Vec::new();
                    if m == 1 {
                        for (ty, a, b) in [
                            (mapping_type, 0x20u32, 0x21u32),
                            (mapping_type, 0x22, 0x20),
                            (0x1234, 0x20, 0x20),
                        ] {
                            let it = t.zeros(0x20);
                            t.w32(it + 8, ty);
                            t.w32(it + 0x18, a);
                            t.w32(it + 0x1c, b);
                            items.push(it);
                        }
                    }
                    let arr = t.alloc(
                        &items
                            .iter()
                            .flat_map(|v| v.to_le_bytes())
                            .collect::<Vec<_>>()
                            .iter()
                            .copied()
                            .chain([0u8; 4])
                            .collect::<Vec<_>>(),
                    );
                    let list = t.zeros(0x10);
                    t.w32(list + 4, 0x0066_0001 + 2 * m);
                    t.w32(list + 8, items.len() as u32);
                    t.w32(list + 0xc, arr);
                    lists.push(list);
                }
                let data = t.alloc(
                    &lists
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect::<Vec<_>>(),
                );
                let maps = t.zeros(8);
                t.w32(maps, 2);
                t.w32(maps + 4, data);
                t.w32(input, maps);
            }
            let what = format!(
                "{label}: log keyboard writer {writer} devices {devices:#x} maps {with_maps}"
            );
            t.check(&what, 0x0062_9e40, Conv::Thiscall(this), &[], |g| {
                Out::U32(l2b::log_keyboard(g, this))
            });
        }
        t.reset();
    }
}

/// `PApplication::onExit` against the original: every step that can fail
/// (with and without its message), the object array owned or not, the
/// input log written or not (slot 2's `al` only: its upper bytes are
/// set), the preprocess buffers and the old object present or not. The
/// frame word the render scope lock gets is compared in the original's
/// frame. `PhyrePrintf` logs three words (the log line's fourth, the
/// name, is past what a three-word call leaves defined).
#[test]
fn left2_on_exit_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    // which step fails (index into the steps below; 99: none), the items
    // owned, the log written, the buffers and the old object there
    for (fail, owned, written, extras) in [
        (99u32, true, true, true),
        (99, false, false, false),
        (99, true, false, true),
        (0, true, true, true),
        (1, true, true, true),
        (2, true, true, true),
        (3, true, true, true),
        (4, true, true, true),
        (5, true, true, true),
        (6, true, true, true),
        (7, true, true, true),
        (8, true, true, true),
        (8, true, true, false),
    ] {
        for (label, mut t) in Twins::all() {
            let code = |step: u32| if fail == step { 0x0b + step } else { 0 };
            // steps: 0 render slot 2, 1 syncRender, 2 item slot 2,
            // 3 exitScene, 4 exitApplication, 5 the array clear,
            // 6 removeTimeline, 7 the command drain, 8 Input_Shutdown
            let c = l2::ON_EXIT_CALLS;
            t.stub(c[0], false, 2, 0, 0x77);
            t.stub(l2::PRINTF, false, 3, 0, 0x78);
            t.stub(c[1], true, 0, 0, code(1));
            t.stub(c[2], false, 1, 0, 0);
            t.stub(c[3], true, 0, 0, code(5));
            t.stub(c[4], false, 0, 0, 0x00e4_0000);
            t.stub(c[5], true, 0, 0, 0x79);
            t.stub(c[6], true, 1, 1, code(6));
            t.stub(c[7], false, 0, 0, 0x00e7_0000);
            t.stub(c[8], true, 0, 0, code(7));
            t.stub(c[9], true, 0, 0, 0x7a);
            t.stub(c[10], true, 1, 1, 0);
            t.stub(c[11], true, 0, 0, 0x7b);
            t.stub(c[12], false, 0, 0, 0x00ec_0000);
            t.stub(c[13], true, 0, 0, 0x7c);
            t.stub(c[14], true, 0, 0, 0x7d);
            t.stub(c[15], true, 0, 0, 0x7e);
            t.stub(l2::INPUT_SHUTDOWN, false, 0, 0, code(8));
            let this = t.zeros(0x400);
            let app = t.object(&[1, 0, 0, 0, 0, 0, 0], &[0, 0, 0, 0, 0, code(3), code(4)]);
            let vt = t.r32(app);
            t.w32(this, vt);
            let render = t.object(&[1, 0, 0], &[0, 0x7f, code(0)]);
            let vt = t.r32(render);
            t.w32(this + 0x124, vt);
            let items: Vec<u32> = (0..3u32)
                .map(|i| t.object(&[1, 0, 0], &[0, 0, if i == 1 { code(2) } else { 0 }]))
                .collect();
            let arr = t.alloc(
                &[items[0], items[1], items[2]]
                    .iter()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            t.w32(this + 0x2c0, arr);
            t.w32(this + 0x2bc, if owned { 3 } else { 0x8000_0003 });
            if owned || written {
                let log = t.object(&[1, 0, 0], &[0, 0, if written { 0x101 } else { 0x100 }]);
                t.w32(log + 0xc, 0x4321);
                t.w32(this + 0x29c, log);
            }
            if extras {
                let (p, q) = (t.zeros(4), t.zeros(4));
                t.w32(this + 0x2d8, p);
                t.w32(this + 0x2d4, q);
                let old = t.object(&[1], &[0]);
                t.w32(this + 0x2c, old);
            }
            t.w32(this + 0x2b8, 0x00e8_0000);
            t.w32(this + 0x398, 0x1234_5678);
            // the lock's word (an early return leaves the original's saved
            // ecx there, a word the port has no counterpart for)
            if fail >= 8 {
                t.frame_len = 4;
            }
            let what = format!(
                "{label}: onExit fail {fail} owned {owned} written {written} extras {extras}"
            );
            t.check_frame(&what, 0x0062_a130, Conv::Thiscall(this), &[], 0x18, |g| {
                let frame = ffx_graphic::host::Host::scratch(g);
                Out::U32(l2b::on_exit(g, this, frame))
            });
            t.reset();
        }
    }
}

/// `strncmp(a, b, n)` for the command line test (the C semantics,
/// unsigned bytes; as field.rs's).
const CMD_STRNCMP: [u8; 0x2a] = [
    0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24, 0x10, 0x8b, 0x4c, 0x24, 0x14, 0x85, 0xc9,
    0x74, 0x13, 0x0f, 0xb6, 0x06, 0x0f, 0xb6, 0x17, 0x29, 0xd0, 0x75, 0x0b, 0x85, 0xd2, 0x74, 0x05,
    0x46, 0x47, 0x49, 0xeb, 0xe9, 0x31, 0xc0, 0x5f, 0x5e, 0xc3,
];

/// `PApplication::parseCommandLine` against the original. The numbers
/// after `=` are raw words: `atoi` answers the word at its argument,
/// `atof` the `float` there. The resolution lookup is a recording stub
/// answering a cell (modes 0..4) or its default word in the frame (5).
/// `PhyrePrintf` logs as many words as each message takes.
#[test]
fn left2_parse_command_line_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    // (the resolution's mode, 5 for the default; the vsync number; the
    // msaa number equal to the old one)
    for (mode, vsync, msaa_same) in [
        (0u32, 0u32, false),
        (1, 1, true),
        (2, 2, false),
        (3, 0, true),
        (4, 1, false),
        (5, 2, false),
    ] {
        for (label, mut t) in Twins::all() {
            let c = l2::COMMAND_LINE_CALLS;
            t.import_code(c[2], &CMD_STRNCMP);
            // atoi: the word at the argument; atof: the float there
            t.import_code(c[4], &[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xc3]);
            t.import_code(c[3], &[0x8b, 0x44, 0x24, 0x04, 0xd9, 0x00, 0xc3]);
            // the lookup: logs its three words, answers the mode's cell or
            // its third word
            let cell = t.alloc(&mode.to_le_bytes());
            let lookup_log = t.stub_code(c[1], false, 3, 0, 0);
            let code = t.a.alloc_code(&[0xcc; 0x20]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 0x20]).unwrap(), code);
            let mut k = vec![
                0xff, 0x74, 0x24, 0x0c, 0xff, 0x74, 0x24, 0x0c, 0xff, 0x74, 0x24, 0x0c,
            ]; // push [esp+0xc] x3
            k.push(0xe8);
            k.extend(lookup_log.wrapping_sub(code + 17).to_le_bytes()); // call log3
            k.extend([0x83, 0xc4, 0x0c]); // add esp, 0xc
            if mode == 5 {
                k.extend([0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+0xc]
            } else {
                k.push(0xb8);
                k.extend(cell.to_le_bytes()); // mov eax, cell
            }
            k.push(0xc3);
            t.write(code, &k);
            let at = t.rebase(c[1]);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);
            // PhyrePrintf: 2 words for the unsupported resolution, 3 for
            // the V sync line, else 4
            let d = l2::COMMAND_LINE_DATA;
            let (unsupported, vsync_msg) = (t.rebase(d[1]), t.rebase(d[5]));
            let logs: Vec<u32> = (2..5)
                .map(|n| t.stub_code(l2::PRINTF, false, n, 0, 0))
                .collect();
            let code = t.a.alloc_code(&[0xcc; 0x30]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 0x30]).unwrap(), code);
            let mut k = vec![0x8b, 0x44, 0x24, 0x08, 0x3d]; // mov eax, [esp+8]; cmp eax, unsupported
            k.extend(unsupported.to_le_bytes());
            k.extend([0x0f, 0x84]);
            k.extend(logs[0].wrapping_sub(code + 15).to_le_bytes()); // je log2
            k.push(0x3d);
            k.extend(vsync_msg.to_le_bytes()); // cmp eax, vsync
            k.extend([0x0f, 0x84]);
            k.extend(logs[1].wrapping_sub(code + 26).to_le_bytes()); // je log3
            k.push(0xe9);
            k.extend(logs[2].wrapping_sub(code + 31).to_le_bytes()); // jmp log4
            t.write(code, &k);
            let at = t.rebase(l2::PRINTF);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);

            let word = |name: &str, v: u32| -> Vec<u8> {
                let mut b = name.as_bytes().to_vec();
                b.extend(v.to_le_bytes());
                b.push(0);
                b
            };
            let config = t.junk(0x40, 3);
            t.w32(config, 1280);
            t.w32(config + 0x18, 4);
            t.w8(config + 0x1c, 1);
            let switches: Vec<Vec<u8>> = vec![
                b"plain\0".to_vec(),
                b"-hh\0".to_vec(),
                b"-script=a.lua\0".to_vec(),
                b"/log=out.txt\0".to_vec(),
                b"-input=keys\0".to_vec(),
                word("-timestep=", 0.25f32.to_bits()),
                b"-resolution=720p\0".to_vec(),
                word("-msaa=", if msaa_same { 4 } else { 2 }),
                word("-vsync=", vsync),
                word("-host=", 0x1234),
                word("-fullscreen=", vsync),
                word("/borderless=", vsync),
                b"-other=1\0".to_vec(),
                b"-script\0".to_vec(),
                b"-?\0".to_vec(),
            ];
            let ptrs: Vec<u32> = switches.iter().map(|a| t.alloc(a)).collect();
            let argv = t.alloc(
                &ptrs
                    .iter()
                    .flat_map(|p| p.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            // without the last switch (help), with it, none
            for n in [14u32, 15, 0] {
                let this = t.zeros(4);
                if n > 6 {
                    t.frame_len = 4;
                }
                let what = format!(
                    "{label}: command line mode {mode} vsync {vsync} msaa same {msaa_same} n {n}"
                );
                // ret 0xc: three words; ebp-4 is the default's word
                t.check_frame(
                    &what,
                    0x0062_adc0,
                    Conv::Thiscall(this),
                    &[n, argv, config],
                    0x18,
                    |g| {
                        let frame = ffx_graphic::host::Host::scratch(g);
                        Out::U32(l2b::parse_command_line(g, this, n, argv, config, frame))
                    },
                );
            }
            t.reset();
        }
    }
}

/// `strcmp(a, b)` for `_stricmp`'s import in the frame input test (the
/// test's names differ in more than case; as field.rs's `strncmp`
/// without the count).
const FI_STRCMP: [u8; 0x24] = [
    0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24,
    0x10, // push esi; push edi; mov esi,[esp+0xc]; mov edi,[esp+0x10]
    0x0f, 0xb6, 0x06, 0x0f, 0xb6, 0x17, 0x29, 0xd0, 0x75,
    0x09, // l: movzx eax,[esi]; movzx edx,[edi]; sub eax,edx; jne done
    0x85, 0xd2, 0x74, 0x05, 0x46, 0x47, 0xeb, 0xee,
    0x90, // test edx,edx; je done; inc esi; inc edi; jmp l
    0x5f, 0x5e, 0xc3, 0x90, 0x90, 0x90, 0x90, // done: pop edi; pop esi; ret
];

/// `applyFrameInput` against the original: a keyboard record with and
/// without its keyboard (a negative sub-key), mapping records (pressed,
/// released, another value; matching maps with semantic and other
/// mappings, no match, no first keyboard, no list, an empty list), a
/// command record found and not, an unknown kind (the warning once).
#[test]
#[allow(clippy::too_many_lines)]
fn big_frame_input_matches_the_original() {
    use ffx_pother_b::m5p_framework_input_left2::{self as l2, bridge as l2b};
    let cstr = |t: &mut Twins, s: &str| {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        t.alloc(&b)
    };
    // (kind, variant)
    let mut cases = vec![(1u32, 0u32), (1, 1), (3, 0), (3, 1), (4, 0), (4, 1), (0, 0)];
    for v in 0..7 {
        cases.push((2, v));
    }
    for (kind, variant) in cases {
        for (label, mut t) in Twins::all() {
            t.import_code(l2::FRAME_INPUT_CALLS[2], &FI_STRCMP);
            // PhyrePrintf: the warnings log their 2 and 3 words, the input
            // lines their first 6 (of 6 to 8)
            let texts = l2::FRAME_INPUT_TEXT;
            let (unmapped, unknown) = (t.rebase(texts[6]), t.rebase(texts[8]));
            let logs: Vec<u32> = [6u32, 3, 2]
                .iter()
                .map(|&n| t.stub_code(l2::PRINTF, false, n, 0, 0x5150))
                .collect();
            let code = t.a.alloc_code(&[0xcc; 0x20]).unwrap();
            assert_eq!(t.b.alloc_code(&[0xcc; 0x20]).unwrap(), code);
            let mut k = vec![0x8b, 0x44, 0x24, 0x08, 0x3d]; // mov eax, [esp+8]; cmp eax, unmapped
            k.extend(unmapped.to_le_bytes());
            k.extend([0x0f, 0x84]);
            k.extend(logs[1].wrapping_sub(code + 15).to_le_bytes()); // je log3
            k.push(0x3d);
            k.extend(unknown.to_le_bytes()); // cmp eax, unknown
            k.extend([0x0f, 0x84]);
            k.extend(logs[2].wrapping_sub(code + 26).to_le_bytes()); // je log2
            k.push(0xe9);
            k.extend(logs[0].wrapping_sub(code + 31).to_le_bytes()); // jmp log6
            t.write(code, &k);
            let at = t.rebase(l2::PRINTF);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut t.a, at, 5).unwrap();
            t.saved.entry(at).or_insert(old);
            t.write(at, &j);
            t.stub(l2::FRAME_INPUT_CALLS[1], true, 0, 0, 0);
            let once = t.rebase(l2::FRAME_INPUT_CALLS[3]);
            t.w8(once, u8::from(kind == 4 && variant == 1));
            let semantic = t.rebase(l2::FRAME_INPUT_CALLS[4]);
            let app = t.zeros(0x2c0);
            let mapper = t.zeros(0xa0);
            t.w32(app + 0x2b8, mapper);
            let keyboard = t.junk(0x40, 2);
            let no_keyboard = (kind == 1 && variant == 1) || (kind == 2 && variant == 4);
            if !no_keyboard {
                t.w32(mapper + 0x80, keyboard);
                t.w32(mapper + 0x80 + 4 * 3, keyboard);
            }
            let rec = t.zeros(0x90);
            t.w32(rec, kind);
            t.w32(rec + 4, 0x77);
            match kind {
                1 => {
                    t.w8(rec + 8, 3);
                    t.w8(rec + 9, 0xfe);
                    t.w8(rec + 0xa, 0x80);
                }
                2 => {
                    t.write(rec + 8, b"Jump\0");
                    t.w32(rec + 0x88, [1, 2, 3, 1, 1, 1, 2][variant as usize]);
                    t.w8(rec + 0x8c, 0x5a);
                    // two maps: "Jump" (unless variant 3) with a semantic
                    // mapping, another one and a semantic mapping of
                    // another map
                    let names = [if variant == 3 { "Walk" } else { "Jump" }, "Run"];
                    let mut maps = Vec::new();
                    for (m, n) in names.iter().enumerate() {
                        let name = cstr(&mut t, n);
                        let mut entries = Vec::new();
                        for k in 0..3u32 {
                            let e = t.zeros(0x20);
                            t.w32(e + 8, if k == 1 { 0x1234 } else { semantic });
                            t.w32(e + 0x18, 0x10 + 4 * k + m as u32);
                            t.w32(e + 0x1c, 0x20 + 4 * k + m as u32);
                            entries.push(e);
                        }
                        let arr = t.alloc(
                            &entries
                                .iter()
                                .flat_map(|v| v.to_le_bytes())
                                .collect::<Vec<_>>(),
                        );
                        let map = t.zeros(0x10);
                        t.w32(map + 8, 0x8000_0003);
                        t.w32(map + 0xc, arr);
                        // the name getter answers the map's name
                        let at = t.a.alloc_code(&[0xcc; 8]).unwrap();
                        assert_eq!(t.b.alloc_code(&[0xcc; 8]).unwrap(), at);
                        let mut k = vec![0xb8];
                        k.extend(name.to_le_bytes());
                        k.push(0xc3);
                        t.write(at, &k);
                        t.w32(map + 4, at);
                        maps.push(map);
                    }
                    // the name getter (0x009ecc70) runs the code at [map + 4]
                    let getter = t.a.alloc_code(&[0xcc; 8]).unwrap();
                    assert_eq!(t.b.alloc_code(&[0xcc; 8]).unwrap(), getter);
                    t.write(getter, &[0xff, 0x61, 0x04]); // jmp [ecx+4]
                    let at = t.rebase(l2::FRAME_INPUT_CALLS[1]);
                    let mut j = vec![0xe9];
                    j.extend(getter.wrapping_sub(at + 5).to_le_bytes());
                    t.write(at, &j);
                    let data = t.alloc(
                        &maps
                            .iter()
                            .flat_map(|v| v.to_le_bytes())
                            .collect::<Vec<_>>(),
                    );
                    let list = t.zeros(8);
                    t.w32(
                        list,
                        if variant == 6 {
                            0x8000_0000
                        } else {
                            0x8000_0002
                        },
                    );
                    t.w32(list + 4, data);
                    if variant != 5 {
                        t.w32(mapper, list);
                    }
                }
                3 => {
                    let name = cstr(&mut t, if variant == 0 { "reset" } else { "none" });
                    t.w32(rec + 8, name);
                    let f = t.stub_code(0xf00d, false, 2, 0, 0);
                    // a third entry past the count, named as the missing command
                    let mut entries = vec![0u8; 0x48 * 3];
                    for (i, n) in ["load", "reset", "none"].iter().enumerate() {
                        entries[0x48 * i..0x48 * i + 4].copy_from_slice(&f.to_le_bytes());
                        entries[0x48 * i + 4..0x48 * i + 4 + n.len()].copy_from_slice(n.as_bytes());
                    }
                    let table_entries = t.alloc(&entries);
                    let table = t.zeros(0x20);
                    t.w32(table + 0x14, table_entries);
                    t.w32(table + 0x18, 2);
                    t.w32(app + 0x2c, table);
                }
                _ => {}
            }
            let what = format!("{label}: frame input kind {kind} variant {variant}");
            t.check(&what, 0x0062_6070, Conv::Cdecl, &[app, rec, 0x99], |g| {
                Out::Al(l2b::apply_frame_input(g, app, rec, 0x99))
            });
            t.reset();
        }
    }
}
