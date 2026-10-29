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
/// original callees, and where its scratch is.
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
        };
        t.log = t.zeros(0x10000);
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
        let out = port(&mut G(&mut self.b, top - below));
        self.b.set_call_top(None);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
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
        let out = port(&mut G(&mut self.b, scratch));
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
