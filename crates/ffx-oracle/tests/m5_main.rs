//! M5 stage 1, list `main` (`ffx_sg::m5main_*`) against the original: on
//! the image alone with synthetic state, and on the snapshots
//! (`FFX_SNAPSHOT`) over the game's memory. Results and every byte
//! written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (most are the PS2 debug
//! side the PC build keeps), so every branch is reached with synthetic
//! data; the snapshots add the game's own globals underneath. Callees
//! that stay original are replaced in both oracles alike by recording
//! stubs (emulated code the test writes): each call appends its tag,
//! `ecx` and its arguments to a log in memory, so the diff checks that
//! both made the same calls in the same order with the same words. Some
//! stubs also return a sequence of values or write memory (a reply word,
//! a projected point) so that the port's later reads see what the
//! original's do.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::format_push_string,
    clippy::similar_names
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_sg::m5main_app::{self as appc, bridge as app};
use ffx_sg::m5main_dbg::bridge as dbg;

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
    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len() as u32) as usize]
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
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        ffx_oracle::check_width("Host::call_st0", addr, args.len(), host::max_words(c, true));
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0_f32().expect("the callee leaves ST0 set")
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
}

/// A stub's result: one value, or the `k`-th of a list on its `k`-th
/// call (the last one after that).
#[derive(Clone)]
enum Ret {
    Fixed(u32),
    Seq(Vec<u32>),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    /// Top bytes of a frame [`Twins::check_framed`] leaves out (the
    /// original's security cookie at `ebp - 4`).
    skip: u32,
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
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            log: 0,
            skip: 4,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
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

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Writes the word at the `ffx.sym` address `sym`.
    fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// Writes the byte at the `ffx.sym` address `sym`.
    fn g8(&mut self, sym: u32, v: u8) {
        let at = self.rebase(sym);
        self.w8(at, v);
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
        let log = self.log;
        self.w32(log, 0);
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Code in both oracles' code areas; returns its address.
    fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    /// A recording stub: logs `tag`, `ecx` when `this`, and `n` stack
    /// arguments; runs `extra` (code that may use `eax`, `ecx`, `edx`;
    /// `[esp + 4]` is the first argument); pops `pops` words; returns
    /// `ret`. Returns its address.
    fn stub_code(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: &Ret,
        extra: &[u8],
    ) -> u32 {
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
        c.extend(extra);
        match ret {
            Ret::Fixed(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, v
            }
            Ret::Seq(seq) => {
                let ctr = self.words(&[0]);
                let mut s = seq.clone();
                s.push(*seq.last().unwrap());
                let last = (seq.len() - 1) as u32;
                let tbl = self.words(&s);
                c.push(0xa1);
                c.extend(ctr.to_le_bytes()); // mov eax, [ctr]
                c.extend([0x8b, 0x04, 0x85]);
                c.extend(tbl.to_le_bytes()); // mov eax, [tbl + eax*4]
                c.extend([0x83, 0x3d]);
                c.extend(ctr.to_le_bytes());
                c.push(last as u8); // cmp dword [ctr], last
                c.extend([0x73, 0x06]); // jae +6
                c.extend([0xff, 0x05]);
                c.extend(ctr.to_le_bytes()); // inc dword [ctr]
                assert!(last < 0x80);
            }
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    /// A jump at `sym` to `to`.
    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// [`Twins::stub_code`] written over the original at `sym` (tag
    /// `sym`).
    fn stub_ex(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: &Ret, extra: &[u8]) {
        let addr = self.stub_code(sym, this, n, pops, ret, extra);
        self.jump(sym, addr);
    }

    /// A plain recording stub over `sym`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        self.stub_ex(sym, this, n, pops, &Ret::Fixed(ret), &[]);
    }

    /// Plain recording cdecl stubs `(sym, words, ret)`.
    fn stubs(&mut self, list: &[(u32, u32, u32)]) {
        for &(sym, n, ret) in list {
            self.stub(sym, false, n, 0, ret);
        }
    }

    /// A recording stub bound to the import table slot `slot`.
    fn stub_import(&mut self, slot: u32, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(slot, false, n, pops, &Ret::Fixed(ret), &[]);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A fake vtable at `obj`'s first word whose slots are recording
    /// thiscall stubs `(words, ret, extra)`, tagged `0x5100 + slot`.
    fn fake_vtable(&mut self, obj: u32, slots: &[(u32, u32, Vec<u8>)]) {
        let mut vt = Vec::new();
        for (i, (n, ret, extra)) in slots.iter().enumerate() {
            vt.push(self.stub_code(0x5100 + i as u32, true, *n, *n, &Ret::Fixed(*ret), extra));
        }
        let vt = self.words(&vt);
        self.patch(obj, &vt.to_le_bytes());
    }

    /// The call logs of both oracles, for a failure message.
    fn logs(&self) -> String {
        let mut s = String::new();
        for (name, o) in [("original", &self.a), ("port", &self.b)] {
            let n: u32 = Mem::read(o, self.log);
            s += &format!("{name}: {n} calls\n");
            for k in 0..n.min(64) {
                let e: [u32; 16] =
                    std::array::from_fn(|i| Mem::read(o, self.log + 4 + 64 * k + 4 * i as u32));
                s += &format!("  {:08x?}\n", &e[..]);
            }
        }
        s
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory (log at {:#x}): {:x?}\n{}",
            self.log,
            &d[..d.len().min(16)],
            self.logs()
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The original's `ebp` for a call with `args` stack words.
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
        let out = bridge(&mut G(&mut self.b));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
        }
        self.diff(what);
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp`
    /// (its callees log pointers into it): `low` bytes below `ebp` are
    /// compared after the call (but the top [`Twins::skip`]).
    #[allow(clippy::too_many_arguments)]
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let words = match conv {
            Conv::Fastcall | Conv::FastcallCallerPops => args.len().saturating_sub(2),
            _ => args.len(),
        };
        let ebp = self.frame_ebp(words);
        let lo = ebp - low;
        self.write(lo, &vec![0; low as usize]);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let n = (low - self.skip) as usize;
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
            let at = lo + i as u32;
            panic!(
                "{what}: frame byte ebp - {:#x} ({at:#x}): {} != {}",
                ebp - at,
                x[i],
                y[i]
            );
        }
    }

    /// The number of calls the log holds (in `a`).
    fn calls(&self) -> u32 {
        Mem::read(&self.a, self.log)
    }
}

/// `mov byte [at], v`.
fn set_byte(at: u32, v: u8) -> Vec<u8> {
    let mut c = vec![0xc6, 0x05];
    c.extend(at.to_le_bytes());
    c.push(v);
    c
}

/// `mov dword [at], v`.
fn set_word(at: u32, v: u32) -> Vec<u8> {
    let mut c = vec![0xc7, 0x05];
    c.extend(at.to_le_bytes());
    c.extend(v.to_le_bytes());
    c
}

/// Writes `w` to the first argument's memory (`mov eax, [esp + 4]` and
/// `mov dword [eax + 4i], w[i]`).
fn fill_arg0(w: &[u32]) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04];
    for (i, v) in w.iter().enumerate() {
        c.extend([0xc7, 0x40, (4 * i) as u8]);
        c.extend(v.to_le_bytes());
    }
    c
}

// ---- FFXApplication ----

#[test]
fn application_ctor_matches_the_original() {
    Twins::each(|t| {
        for stop_at in 0..=appc::UTILITIES.len() {
            t.reset();
            t.stub(app::APPLICATION_CTOR, true, 0, 0, 0x99);
            t.stub(app::SET_WINDOW_ICON_ID, true, 1, 1, 0x98);
            let application = t.zeros(0x400);
            t.stub(app::GET_APPLICATION, false, 0, 0, application);
            let stop = t.rebase(appc::UTILITY_STOP);
            t.w8(stop, 0);
            for (k, obj) in appc::UTILITIES.into_iter().enumerate() {
                let obj = t.rebase(obj);
                // the `stop_at`-th registration sets the stop byte
                let extra = if k + 1 == stop_at {
                    set_byte(stop, 1)
                } else {
                    Vec::new()
                };
                t.fake_vtable(
                    obj,
                    &[(0, 0), (0, 0x40 + k as u32)].map(|(n, r)| (n, r, Vec::new())),
                );
                if !extra.is_empty() {
                    t.fake_vtable(obj, &[(0, 0, Vec::new()), (0, 0x40, extra)]);
                }
            }
            if stop_at == 0 {
                t.w8(stop, 1);
            }
            let this = t.words(&vec![0x5555_5555; 0x100]);
            let what = format!("FFXApplication ctor stop {stop_at}");
            t.check(&what, 0x0042_ef40, Conv::Thiscall(this), &[], |m| {
                Out::U32(app::ctor(m, this))
            });
            let calls = t.calls();
            assert!(calls >= 5, "{what}: {calls} calls");
        }
    });
}

#[test]
fn phyre_free_thunk_matches_the_original() {
    Twins::each(|t| {
        for p in [0, 0x1234_5678, 0xffff_fff0] {
            t.reset();
            t.stub(app::PHYRE_FREE, false, 1, 0, p ^ 0x55);
            t.check("PhyreFree thunk", 0x0042_f140, Conv::Cdecl, &[p], |m| {
                Out::U32(app::phyre_free_thunk(m, p))
            });
        }
    });
}

#[test]
fn application_deleting_dtor_matches_the_original() {
    Twins::each(|t| {
        for close in [0, 1, 2] {
            for flags in [0, 1, 2, 3, 0xffff_fffe] {
                t.reset();
                t.stub_import(app::SEND_MESSAGE_IAT, 4, 4, 0x11);
                t.stub(app::APPLICATION_DTOR, true, 0, 0, 0x22);
                t.stub(app::PHYRE_FREE, false, 1, 0, 0x33);
                t.g32(appc::CLOSE_ON_DESTROY, close);
                t.g32(appc::HWND, 0x0001_0203);
                let this = t.words(&[0x1111_1111, 2, 3]);
                let what = format!("FFXApplication delete close {close} flags {flags:#x}");
                t.check(&what, 0x0042_f150, Conv::Thiscall(this), &[flags], |m| {
                    Out::U32(app::deleting_dtor(m, this, flags))
                });
            }
        }
    });
}

#[test]
fn mem_dup_and_reset_to_launcher_match_the_original() {
    Twins::each(|t| {
        for (src, size, got) in [
            (0, 8, 0x5000),
            (0x1234, 8, 0),
            (0x1234, 0x40, 0x5000),
            (1, 0, 0x60),
        ] {
            t.reset();
            t.stub(app::PHYRE_MALLOC, false, 2, 0, got);
            t.stub(app::MEMCPY, false, 3, 0, 0x77);
            let what = format!("mem_dup {src:#x} {size:#x} -> {got:#x}");
            t.check(&what, 0x0042_f220, Conv::Cdecl, &[src, size], |m| {
                Out::U32(app::mem_dup(m, src, size))
            });
        }
        for r in [0, 1, 0x8000_0000] {
            t.reset();
            t.stub_import(app::SEND_MESSAGE_IAT, 4, 4, r);
            t.g32(appc::HWND, 0x0aaa_0bbb);
            t.check("ResetToLauncher", 0x0042_f4d0, Conv::Cdecl, &[], |m| {
                Out::U32(app::reset_to_launcher(m))
            });
        }
    });
}

#[test]
fn after_graphic_sync_matches_the_original() {
    Twins::each(|t| {
        for done in [0u8, 1, 0x80] {
            for menu in [0, 1, 0x100, 0x1234_5600, 0xffff_ffff] {
                t.reset();
                t.stub(app::DYN_GEO_SWAP, false, 1, 0, 0x44);
                t.stub(app::IS_ESC_MENU_ACTIVE, false, 0, 0, menu);
                let this = t.zeros(0x400);
                t.w8(this + 0x3ad, done);
                let what = format!("afterGraphicSync done {done} menu {menu:#x}");
                let addr = t.rebase(0x0042_f4f0);
                let ra = t.a.call(addr, Conv::Thiscall(this), &[]).unwrap();
                let r = app::after_graphic_sync(&mut G(&mut t.b), this);
                if done == 0 {
                    assert_eq!(ra.eax, r, "{what}: eax");
                }
                t.diff(&what);
            }
        }
    });
}

#[test]
fn update_global_time_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0042_fa00);
        let wrap: u32 = Mem::read(&t.a, t.rebase(appc::TIME_WRAP));
        let w = f32::from_bits(wrap);
        let times = [
            0.0f32,
            1.5,
            w,
            w - 0.25,
            w + 1.0,
            -3.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            1e30,
        ];
        let steps = [0.0f32, 1.0 / 60.0, 0.5, -1.0, f32::NAN, 1e-30, w];
        for i in 0..200 {
            t.reset();
            let cam = t.zeros(0x400);
            t.stub(app::FREE_CAMERA, false, 0, 0, cam);
            t.w8(cam + 0x338, u8::from(rng.chance(4)));
            t.w8(cam + 0x33c, u8::from(rng.chance(4)));
            t.w32(cam, rng.pick(&[0, 1, 2, 3]));
            t.g32(appc::PAUSE_ON, u32::from(rng.chance(4)));
            let this = t.zeros(0x400);
            t.w8(this + 0x3ac, u8::from(!rng.chance(4)));
            t.w32(this + 0x24, rng.pick(&steps).to_bits());
            t.w32(this + 0x3b0, rng.pick(&times).to_bits());
            t.w32(this + 0x120, rng.next());
            let what = format!("updateGlobalTime #{i}");
            t.check(&what, 0x0042_fa00, Conv::Thiscall(this), &[], |m| {
                app::update_global_time(m, this);
                Out::None
            });
        }
    });
}

#[test]
fn poll_repeat_input_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x006e_f320);
        let counts = [
            0u32,
            1,
            2,
            9,
            10,
            11,
            14,
            15,
            20,
            21,
            0xffff_ffff,
            0x7fff_ffff,
        ];
        let sticks = [0.0f32, -0.3, 0.3, -0.31, 0.31, -1.0, 1.0, f32::NAN, 0.29];
        for i in 0..300 {
            t.reset();
            let stick = t.words(&[rng.pick(&sticks).to_bits(), rng.pick(&sticks).to_bits()]);
            t.stub(app::STICK_INFO, false, 0, 0, stick);
            let held: Vec<u32> = (0..6)
                .map(|_| if rng.chance(3) { rng.word() | 1 } else { 0 })
                .collect();
            let pressed: Vec<u32> = (0..2).map(|_| u32::from(rng.chance(3))).collect();
            t.stub_ex(app::IS_HOLD, false, 1, 0, &Ret::Seq(held), &[]);
            t.stub_ex(app::IS_PRESS, false, 1, 0, &Ret::Seq(pressed), &[]);
            let base = t.rebase(app::REPEAT_COUNTERS);
            for k in 0..8 {
                let c = rng.pick(&counts);
                t.w32(base + 4 * k, c);
            }
            let what = format!("poll_repeat_input #{i}");
            t.check(&what, 0x006e_f320, Conv::Cdecl, &[], |m| {
                Out::U32(app::poll_repeat_input(m))
            });
        }
    });
}

// ---- the debug side ----

#[test]
fn system_on_host_matches_the_original() {
    Twins::each(|t| {
        let send = t.rebase(dbg::DECI2_SEND_BUF) | dbg::UNCACHED;
        let recv = t.rebase(dbg::DECI2_RECV_BUF) | dbg::UNCACHED;
        t.a.map_zeroed(send, 0x2_0000).unwrap();
        t.b.map_zeroed(send, 0x2_0000).unwrap();
        let reply_at = t.rebase(dbg::DECI2_REPLY);
        // (send result, reply word, payload size, failed word)
        let cases = [
            (0xffff_fffe, 1, 0x20, 0),
            (0, 0xffff_fff0, 0x20, 0),
            (0, 1, 0x20, 0),
            (0, 7, 0x0c, 0),
            (5, 1, 0x30, 3),
            (0, 0x8000_0000, 0x10, 0),
        ];
        for (i, (sent, reply, size, failed)) in cases.into_iter().enumerate() {
            for cmd in ["", "ls host0:", "a much longer host command line"] {
                t.reset();
                t.stub_ex(
                    dbg::DECI2_SEND,
                    false,
                    2,
                    0,
                    &Ret::Fixed(sent),
                    &set_word(reply_at, reply),
                );
                // one word: the failed-command message has no second
                t.stub(dbg::DBG_PRINTF, false, 1, 0, 0);
                t.stub(dbg::MEMCPY, false, 3, 0, 0);
                t.g32(dbg::DECI2_SOCKET, 0x0bad_5ec7);
                t.write(recv, &(size as u16).to_le_bytes());
                t.w32(recv + 8, failed);
                let mut c = cmd.as_bytes().to_vec();
                c.push(0);
                let command = t.alloc(&c);
                let out = t.zeros(0x100);
                let what = format!("SystemOnHost #{i} {cmd:?}");
                t.check(&what, 0x0084_ede0, Conv::Cdecl, &[command, out], |m| {
                    Out::U32(dbg::system_on_host(m, command, out).cast_unsigned())
                });
            }
        }
    });
}

/// `sprintf`'s import bound to recording stubs that log the words each
/// format takes: three for the two host commands, nine for the plain
/// name, ten for the user's.
fn sprintf_stub(t: &mut Twins) {
    let mut c = vec![0x8b, 0x54, 0x24, 0x08]; // mov edx, [esp + 8]
    for (fmt, n) in [(dbg::FMT_HOST1, 3), (dbg::FMT_HOST2, 3), (dbg::FMT_FILE, 9)] {
        let to = t.stub_code(dbg::SPRINTF_IAT, false, n, 0, &Ret::Fixed(0x21), &[]);
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(fmt).to_le_bytes()); // cmp edx, fmt
        c.extend([0x75, 7]); // jne next
        c.push(0xb8);
        c.extend(to.to_le_bytes()); // mov eax, to
        c.extend([0xff, 0xe0]); // jmp eax
    }
    let long = t.stub_code(dbg::SPRINTF_IAT, false, 10, 0, &Ret::Fixed(0x21), &[]);
    c.push(0xb8);
    c.extend(long.to_le_bytes()); // mov eax, long
    c.extend([0xff, 0xe0]); // jmp eax
    let at = t.code(&c);
    let slot = t.rebase(dbg::SPRINTF_IAT);
    t.patch(slot, &at.to_le_bytes());
}

#[test]
fn capture_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_a270);
        for i in 0..120 {
            t.reset();
            let user = if rng.chance(2) { 0 } else { 0x00c0_ffee };
            t.stub(dbg::GET_USER_NAME, false, 0, 0, user);
            sprintf_stub(t);
            t.stub(dbg::DBG_PRINTF, false, 2, 0, 0);
            t.stub(dbg::SYSTEM_ON_HOST, false, 2, 0, 0);
            t.stub(dbg::CAPTURE_TO_FILE, false, 1, 0, 0);
            t.g32(dbg::CAPTURE_LEFT, rng.pick(&[0, 1, 2, 5]));
            t.g32(dbg::CAPTURE_SKIP, rng.pick(&[0, 0, 1, 3]));
            t.g32(dbg::CAPTURE_EVERY, u32::from(rng.chance(3)));
            t.g32(dbg::CAPTURE_SCENE, rng.word());
            t.g32(dbg::CAPTURE_FILE, rng.word());
            t.g32(dbg::CAPTURE_MKDIR, u32::from(rng.chance(2)));
            let name = t.rebase(dbg::CAPTURE_NAME);
            for k in 0..5 {
                t.w8(name + k, rng.next() as u8);
            }
            let what = format!("Sg_Capture #{i}");
            t.check_framed(&what, 0x0083_a270, Conv::Cdecl, &[], 0x204, |m, ebp| {
                Out::U32(dbg::capture(m, ebp - 0x204))
            });
        }
    });
}

#[test]
fn save_resource_check_matches_the_original() {
    Twins::each(|t| {
        for req in [0, 1, 0x100] {
            for n in [0, 1, 2, 30, 0xffff_ffff] {
                t.reset();
                t.stub(dbg::SAVE_VRAM, false, 0, 0, 0x5a5a);
                t.g32(dbg::SAVE_REQUEST, req);
                t.g32(dbg::SAVE_COUNTDOWN, n);
                let what = format!("Sg_SaveResourceCheck req {req} n {n}");
                t.check(&what, 0x0083_fd20, Conv::Cdecl, &[], |m| {
                    Out::U32(dbg::save_resource_check(m))
                });
            }
        }
    });
}

#[test]
fn gs_xyzf2l_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0084_21b0);
        let buf = t.zeros(0x1000);
        for i in 0..60 {
            t.reset();
            t.g32(dbg::GS_BASE, buf);
            t.g32(dbg::GS_INDEX, rng.below(0x40));
            let (x, y, z) = (rng.next(), rng.next(), rng.word());
            let what = format!("Sg_GsXYZF2L #{i}");
            t.check(&what, 0x0084_21b0, Conv::Cdecl, &[x, y, z], |m| {
                dbg::gs_xyzf2l(m, x, y, z.cast_signed());
                Out::None
            });
        }
    });
}

/// Recording stubs for the GS writers and the matrices `Sg_DbDraw` and
/// the ID draw call; `point` is what `FFXVu0RotTransPers` writes.
fn gs_stubs(t: &mut Twins, point: [u32; 3]) {
    t.stubs(&[
        (dbg::UNIT_MATRIX, 1, 0),
        (dbg::SCREEN_MATRIX, 0, 0x0012_3400),
        (dbg::CAMERA_MATRIX, 1, 0x0056_7800),
        (dbg::MUL_MATRIX, 3, 0),
        (dbg::PACKET_PTR, 0, 0x0abc_0000),
        (dbg::GS_SET_PACKET, 1, 0),
        (dbg::GS_START, 0, 0),
        (dbg::GS_TEST, 8, 0),
        (dbg::GS_PRIM, 1, 0),
        (dbg::GS_RGBAQ, 4, 0),
        (dbg::GS_XYZF2L, 3, 0),
        (dbg::GS_END, 0, 0),
        (dbg::DBG_PRINTF, 2, 0),
    ]);
    t.stub_ex(
        dbg::ROT_TRANS_PERS,
        false,
        4,
        0,
        &Ret::Fixed(0),
        &fill_arg0(&point),
    );
}

#[test]
fn db_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0084_12c0);
        for i in 0..40 {
            t.reset();
            gs_stubs(t, [rng.next(), rng.next(), rng.next()]);
            t.g8(dbg::DEBUG_ON, u8::from(!rng.chance(5)));
            // a list of every command kind, ending in 0
            let mut list = Vec::new();
            for _ in 0..rng.below(12) {
                match rng.below(4) {
                    0 => list.extend([1, 0, rng.next() as u8, rng.next() as u8]),
                    1 => list.extend([2, rng.next() as u8, rng.next() as u8, rng.next() as u8]),
                    2 => {
                        list.extend([3, 9, 9, 9]);
                        for f in [1.5f32, -2.0, f32::from_bits(rng.next() & 0x7f7f_ffff)] {
                            list.extend(f.to_bits().to_le_bytes());
                        }
                    }
                    _ => {
                        list.extend([4, 0]);
                        list.extend(rng.next().to_le_bytes());
                        list.extend([7, 7]);
                    }
                }
            }
            list.push(0);
            let l = t.alloc(&list);
            t.g32(dbg::DB_LIST, l);
            let last = t.words(&[0xffff_ffff]);
            t.g32(dbg::DB_LAST, last);
            let what = format!("Sg_DbDraw #{i}");
            t.check_framed(
                &what,
                0x0084_12c0,
                Conv::Cdecl,
                &[],
                dbg::DB_FRAME,
                |m, ebp| {
                    dbg::db_draw(m, ebp);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn id_project_matches_the_original() {
    Twins::each(|t| {
        let xs = [0x6600u32, 0x65ff, 0x9a00, 0x9a01, 0x8000, 0xffff_9000];
        let ys = [0x7000u32, 0x6fff, 0x9000, 0x9001, 0x8000];
        let zs = [0u32, 1, 0xffff_ffff, 0x8000_0000, 0x1234];
        let mut rng = Rng(0x0084_5030);
        for i in 0..120 {
            t.reset();
            let p = [rng.pick(&xs), rng.pick(&ys), rng.pick(&zs)];
            t.stub_ex(
                dbg::ROT_TRANS_PERS,
                false,
                4,
                0,
                &Ret::Fixed(0),
                &fill_arg0(&p),
            );
            let out = t.words(&[9, 9, 9, 9]);
            let (v, m) = (0x0011_2233, 0x0044_5566);
            let what = format!("Sg_IdProject #{i}");
            t.check_framed(
                &what,
                0x0084_5030,
                Conv::Cdecl,
                &[out, v, m],
                dbg::PROJECT_FRAME,
                |g, ebp| Out::U32(dbg::id_project(g, ebp, out, v, m)),
            );
        }
    });
}

#[test]
fn load_tim_and_debug_font_match_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stubs(&[(dbg::FLUSH_CACHE, 1, 0), (dbg::GS_LOAD_IMAGE, 2, 0x10)]);
        t.stub_ex(
            dbg::GS_SYNC_PATH,
            false,
            2,
            0,
            &Ret::Seq(vec![1, 2, 3, 4, 5, 6]),
            &[],
        );
        let tim = t.words(&[0, 0, 0x1111, 0x2222]);
        t.check("Load1TIM", 0x0083_f800, Conv::Cdecl, &[tim], |m| {
            Out::U32(dbg::load_1tim(m, tim))
        });
        t.reset();
        t.stub(dbg::LOAD_1TIM, false, 1, 0, 0x4321);
        t.check("Send_DebugFont", 0x0085_10b0, Conv::Cdecl, &[], |m| {
            Out::U32(dbg::send_debug_font(m))
        });
    });
}

#[test]
fn main_debug_matches_the_original() {
    Twins::each(|t| {
        for case in 0..32u32 {
            t.reset();
            t.stubs(&[
                (dbg::ID_DRAW_MAIN, 0, 0),
                (dbg::COUNTERS_FLUSH, 1, 0),
                (dbg::DB_DRAW, 0, 0),
                (dbg::CAPTURE_GUI_HIDE, 0, case & 2),
                (dbg::PACKET_PTR, 0, 0x0abc_0000),
                (dbg::GU_MAIN, 2, 0),
                (dbg::SAVE_RESOURCE_CHECK, 0, 0),
                (dbg::CAPTURE, 0, (case >> 3) & 1),
                (dbg::SET_TK_ANTI_XYB, 3, 0),
            ]);
            t.g8(dbg::DEBUG_ON, (case & 1) as u8);
            t.g8(dbg::GUI_ON, if case & 4 != 0 { 0x10 } else { 0 });
            t.g32(dbg::GUI_PACKET, 0x0bbb_0000 + case);
            let what = format!("Sg_MainDebug #{case}");
            t.check(&what, 0x0082_07f0, Conv::Cdecl, &[], |m| {
                dbg::main_debug(m);
                Out::None
            });
        }
    });
}

#[test]
fn unlock_achievement_and_movie_environment_match_the_original() {
    Twins::each(|t| {
        for (enabled, id) in [(0, 3), (1, 3), (0x100, 0x21)] {
            t.reset();
            t.stub(dbg::SAVE_ENABLED, false, 0, 0, enabled);
            t.stub(dbg::UNLOCK, false, 1, 0, 0x77);
            let what = format!("achievementUnlockAchievement {enabled} {id}");
            t.check(&what, 0x0082_2380, Conv::Cdecl, &[id], |m| {
                Out::U32(dbg::unlock_achievement(m, id))
            });
        }
        for flags in [0u8, 1, 2, 3] {
            t.reset();
            t.stubs(&[
                (dbg::GS_SET_PACKET, 1, 0),
                (dbg::GS_START, 0, 0),
                (dbg::GS_FRAME1, 2, 0),
                (dbg::GS_ZBUF1, 2, 0),
                (dbg::GS_XYOFFSET1, 4, 0),
                (dbg::GS_SCISSOR_ON1, 1, 0),
                (dbg::GS_FRAME2, 2, 0),
                (dbg::GS_ZBUF2, 2, 0),
                (dbg::GS_XYOFFSET2, 4, 0),
                (dbg::GS_SCISSOR_ON2, 1, 0),
                (dbg::GS_END, 0, 0),
                (dbg::GS_KICK, 0, 0),
                (dbg::GS_SYNC_PATH, 2, 0x66),
            ]);
            t.g8(dbg::MOVIE_FLAGS, flags);
            t.g32(dbg::MOVIE_W, 0x280 + u32::from(flags));
            t.g32(dbg::MOVIE_H, 0x1c0);
            let what = format!("Sg_DrawSetEnvironmentInitMovie flags {flags}");
            t.check(&what, 0x0082_caa0, Conv::Cdecl, &[], |m| {
                Out::U32(dbg::draw_set_environment_init_movie(m))
            });
        }
    });
}

// ---- the frame ----

use ffx_sg::m5main_loop::{self as lpc, bridge as lp, f as lf, g as lg};

/// `fld dword [cell]` (a stub's `ST0` result).
fn fld(cell: u32) -> Vec<u8> {
    let mut c = vec![0xd9, 0x05];
    c.extend(cell.to_le_bytes());
    c
}

/// Writes `v[i]` (`f32` bits) through argument `i` (`mov eax, [esp + 4 +
/// 4i]; mov dword [eax], v[i]`).
fn fill_args(v: &[u32]) -> Vec<u8> {
    let mut c = Vec::new();
    for (i, w) in v.iter().enumerate() {
        c.extend([0x8b, 0x44, 0x24, (4 + 4 * i) as u8]);
        c.extend([0xc7, 0x00]);
        c.extend(w.to_le_bytes());
    }
    c
}

/// The callees of `Sg_MainLoop` whose result no branch reads, with the
/// words the original passes.
const MAIN_LOOP_PLAIN: [(u32, u32); 112] = [
    (lf::USE_EXTENDED_ALPHA, 1),
    (lf::SCENE_CLEAR_LIST, 0),
    (lf::PS2_LOADING_CALLBACK, 0),
    (lf::MODEL_MEM_USAGE, 0),
    (lf::VIDEO_UPDATE, 2),
    (lf::SET_DISTORTION_SKIPPED, 1),
    (lf::TK_UPDATE_SCAN, 0),
    (lf::FRAME_SYNC, 1),
    (lf::TK_SCAN, 0),
    (lf::VFX_UPDATE_DELETED, 0),
    (lf::CHR_INSTANTIATE, 0),
    (lf::CHR_DELAY_RELEASE, 1),
    (lf::STUB_639380, 0),
    (lf::SAVE_UPDATE, 0),
    (lf::GET_U32_01325AC4, 0),
    (lf::ALBHED_DIC, 2),
    (lf::SAVE_SCREEN, 0),
    (lf::RESET_NAME, 0),
    (lf::DYN_GEO_SWAP, 1),
    (lf::UNLOCK_FADE, 0),
    (lf::SET_FADE_COLOR, 4),
    (lf::SET_FADE_ALPHA_MODE, 1),
    (lf::LOCK_FADE, 0),
    (lf::SET_FADE_LAST_REASON, 1),
    (lf::PROFILER_MARKER, 1),
    (lf::CONTROL_MAIN, 0),
    (lf::MENU_RESET, 0),
    (lf::ACHIEVEMENT, 1),
    (lf::CALC_RATE, 0),
    (lf::INIT_FONT, 0),
    (lf::USER_FREE, 1),
    (lf::VFX_DESTROY, 2),
    (lf::CLEAR_FLAGS24, 0),
    (lf::CONTROL_PLAYER, 0),
    (lf::BATTLE_PROCESS, 0),
    (lf::EVENT_JUMP, 0),
    (lf::MESSAGE_WINDOW, 0),
    (lf::COUNTERS_FLUSH, 1),
    (lf::OP_LOOP_BEFORE, 0),
    (lf::EFFECT_PROCESS, 1),
    (lf::CAMERA_PROCESS, 0),
    (lf::FIELD_PROC, 0),
    (lf::CALC_MAIN, 1),
    (lf::CAMERA_SET_PLAYER, 0),
    (lf::CALC_MIDDLE, 0),
    (lf::YI_ANIM, 0),
    (lf::MOVIE_CAMERA, 0),
    (lf::VIDEO_PLAY, 0),
    (lf::TEXTURE_VIDEO, 0),
    (lf::SET_DISP_DRAW_ENV, 1),
    (lf::ENV_INIT, 0),
    (lf::SET_CHADIST, 0),
    (lf::MFIFO_RESET, 0),
    (lf::CAMERA_PUT_RECTS, 0),
    (lf::GET_PART_MATRIX, 2),
    (lf::COPY_CAMERA, 1),
    (lf::COPY_SCREEN, 1),
    (lf::SET_PART_MATRIX, 2),
    (lf::INVERSE, 2),
    (lf::VIEW_PROJECTION, 0),
    (lf::YI_SCALE, 1),
    (lf::YI_VIEW, 1),
    (lf::YI_BATTLE_VIEW, 1),
    (lf::CH_CAMERA_MATRIX, 1),
    (lf::CH_SCREEN_MATRIX, 1),
    (lf::DRAW_ENV, 1),
    (lf::ACC_FRAME_MAP, 0),
    (lf::ACC_DRAW_BG, 0),
    (lf::YI_DISP, 0),
    (lf::YI_DISP2, 0),
    (lf::ACC_FRAME_CHR, 0),
    (lf::MFIFO_END, 0),
    (lf::MFIFO_SPAD_SYNC, 0),
    (lf::MFIFO_SYNC, 0),
    (lf::DREAM_EFFECT, 0),
    (lf::MENU_WATER, 0),
    (lf::OP_LOOP_AFTER, 0),
    (lf::BATTLE_DRAW, 0),
    (lf::TK_DRAW_NO_TRANS, 0),
    (lf::YN_DRAW_SCENE, 0),
    (lf::CH_DRAW_SHADOW, 1),
    (lf::GS1_TRANSFER, 0),
    (lf::SET_VFX_TYPE, 1),
    (lf::FIELD_PARTICLE, 0),
    (lf::PART_LOOP, 0),
    (lf::OT_DRAW, 0),
    (lf::SAVE_MANAGER_UPDATE, 0),
    (lf::DRAW_FILTER, 0),
    (lf::ACC_SET_CURRENT, 0),
    (lf::SCR_EFFECT, 0),
    (lf::TK_088D500, 0),
    (lf::DRAW_MESSAGE_WINDOW, 0),
    (lf::SET_PAD_CTRL, 1),
    (lf::TK_DRAW_DEBUG, 0),
    (lf::NUM_VDISP, 0),
    (lf::MAIN_DEBUG, 0),
    (lf::OP_LOOP_END, 0),
    (lf::CALM_COPYRIGHT, 1),
    (lf::MENU_UPDATE, 0),
    (lf::FLUSH_CACHE, 1),
    (lf::GS_SYNC_PATH, 2),
    (lf::COUNTERS_SNAPSHOT, 0),
    (lf::TK_SYNC_V, 1),
    (lf::SWAP_DBUFF, 2),
    (lf::SET_TK_ANTI_XYB, 3),
    (lf::ENV_MOVIE_MESSAGE, 0),
    (lf::MAGIC_FILE_UPDATE, 0),
    (lf::CLUSTER_RELATIONSHIP, 0),
    (lf::CLOTH_WAIT, 0),
    (lf::CLOTH_STATS, 0),
    (lf::SOFTWARE_RESET, 0),
    (lf::IS_PREREND, 0),
];

fn bit(rng: &mut Rng, n: u32) -> u32 {
    u32::from(rng.chance(n))
}

/// A stub over `sym` (`n` words) returning `values` in turn.
fn seq(t: &mut Twins, sym: u32, n: u32, values: Vec<u32>) {
    t.stub_ex(sym, false, n, 0, &Ret::Seq(values), &[]);
}

/// The byte and word globals `Sg_MainLoop` tests, drawn from `rng`.
fn main_loop_globals(t: &mut Twins, rng: &mut Rng) {
    for sym in [
        lg::FIELD_ON,
        lg::FIELD_DRAW,
        lg::CHARA_ON,
        lg::CHARA_DRAW,
        lg::FIELD_FIRST,
        lg::MOVIE_LAST,
        lg::CHARA_CALC,
        lg::PART_MATRIX,
        lg::CAMERA_IDLE,
        lg::EFFECT_ON,
        lg::PARTICLE_ON,
        lg::MESSAGE_ON,
        lg::DEBUG_ON,
    ] {
        let v = u8::from(!rng.chance(3));
        t.g8(sym, v);
    }
    let v = u8::from(rng.chance(4));
    t.g8(lg::MOVIE_ON, v);
    let v = u8::from(rng.chance(4));
    t.g8(lg::MOVIE_HOLD, v);
    let v = rng.pick(&[0, 1, 0xff]);
    t.g8(lg::SCREEN_LAST, v);
    let airship = rng.pick(&[0u8, 1, 0x7e, 0x7f, 0x80]);
    t.g8(lg::AIRSHIP, airship);
    for (sym, of) in [
        (lg::PAUSE_ON, &[0u32, 0, 0, 1][..]),
        (lg::SCENE_REQUEST, &[0, 0, 0, 3]),
        (lg::DEBUG_HOLD, &[0, 0, 0, 1]),
        (lg::VOICE_HOLD, &[0, 1, 2, 5]),
        (lg::REPEAT_A, &[0, 1]),
        (lg::REPEAT_B, &[0, 1]),
        (lg::REPEAT_C, &[0, 1]),
        (lg::DYN_GEO_STATE, &[0, 1, 2]),
        (lg::MENU_FADE, &[0, 1]),
        (lg::RESET_REQUEST, &[0, 1, 2]),
        (lg::DISP_FLAGS, &[0, 0x0100_0000, 0xffff_ffff]),
        (lg::INTERFACE, &[0, 0x1234]),
        (lg::MAP, &[0, 0x17]),
        (lg::SCENARIO_LAST, &[0, 0xa4, 0x43d, 0x938]),
        (lg::AIRSHIP_DONE, &[0, 1, 0xffff_ffff]),
        (lg::FREE_NEXT, &[0, 0x0077_0000, 0x0077_0010]),
        (lg::FREE_SEEN, &[0, 0x0077_0000]),
        (lg::VFX_DESTROY, &[0, 1]),
        (lg::VFX_HOLD, &[0, 1]),
        (lg::NAME_RESET, &[0, 1]),
        (lg::MENU_WATER, &[0, 1]),
        (lg::CALM_STATE, &[0, 1, 2]),
        (lg::CALM_FRAMES, &[0, 0x12f, 0x130]),
        (lg::DRAW_BUFFER, &[0, 1, 0xffff_ffff]),
    ] {
        let v = rng.pick(of);
        t.g32(sym, v);
    }
    let printed = rng.below(3) as u16;
    let at = t.rebase(lg::SCENARIO_PRINTED);
    t.write(at, &printed.to_le_bytes());
}

/// Recording stubs over every callee of `Sg_MainLoop`, with results and
/// globals drawn from `rng` so that each case takes other branches.
fn main_loop_case(t: &mut Twins, rng: &mut Rng) {
    for (sym, n) in MAIN_LOOP_PLAIN {
        t.stub(sym, false, n, 0, 0x4000 + (sym & 0xfff));
    }
    // the special-list clear may end the scene request, the characters'
    // draw the field switch (both are read again after them)
    let at = t.rebase(lg::SCENE_REQUEST);
    let extra = if rng.chance(2) {
        set_word(at, 0)
    } else {
        Vec::new()
    };
    t.stub_ex(lf::CLEAR_SPECIAL, false, 0, 0, &Ret::Fixed(0), &extra);
    let at = t.rebase(lg::FIELD_ON);
    let extra = if rng.chance(2) {
        set_byte(at, 0)
    } else {
        Vec::new()
    };
    t.stub_ex(lf::CH_DRAW, false, 0, 0, &Ret::Fixed(0), &extra);
    let scaled = rng.pick(&[1.0f32 / 60.0, 0.5, 0.0, 2.0]).to_bits();
    let cell = t.words(&[scaled]);
    t.stub_ex(lf::SPD_SCALE, false, 1, 0, &Ret::Fixed(0), &fld(cell));
    let cam = if rng.chance(8) { 2 } else { 1 };
    seq(t, lf::FREE_CAMERA_TYPE, 0, vec![cam]);
    let v = vec![bit(rng, 2), bit(rng, 2)];
    seq(t, lf::GET_DISP_MODE, 0, v);
    let v = vec![bit(rng, 3), bit(rng, 3)];
    seq(t, lf::SAVE_TAKE_LOADED, 0, v);
    let v = vec![bit(rng, 2)];
    seq(t, lf::RETURN_CONST1, 0, v);
    let v = vec![rng.pick(&[0, 0x8120, 0xffff, 0x1_8120, 0x8100])];
    seq(t, lf::CONTROLER_HIST, 3, v);
    let v = vec![bit(rng, 5), bit(rng, 5), bit(rng, 5)];
    seq(t, lf::CHECK_MENU, 0, v);
    let v = vec![u32::from(!rng.chance(8)), u32::from(!rng.chance(3))];
    seq(t, lf::SAVE_SCREEN_CLOSED, 0, v);
    let v = vec![bit(rng, 8)];
    seq(t, lf::SCENE_LOADING, 0, v);
    let v = vec![bit(rng, 2), bit(rng, 2)];
    seq(t, lf::ASSETS_LOADED, 0, v);
    let v: Vec<u32> = (0..3).map(|_| rng.pick(&[0, 1, 2, 0xffff_ffff])).collect();
    seq(t, lf::FADE_LAST_REASON, 0, v);
    let v = vec![bit(rng, 2), bit(rng, 2)];
    seq(t, lf::FADE_LOCKED, 0, v);
    let v = vec![bit(rng, 3)];
    seq(t, lf::VALIDATE_INTERFACE, 1, v);
    let v = vec![rng.pick(&[0xa4, 0x43d, 0x938, 7, 0])];
    seq(t, lf::SCENARIO_FLAG, 0, v);
    let v = vec![rng.pick(&[0, 1, 2, 3])];
    seq(t, lf::EXEC_FRAMES, 0, v);
    let v: Vec<u32> = (0..8)
        .map(|_| rng.pick(&[0, 0, 0xffff_ffff, 1]))
        .chain([1, 1, 1, 1, 1])
        .collect();
    seq(t, lf::SCREEN_PROCESS, 1, v);
    let v: Vec<u32> = (0..8).map(|_| rng.pick(&[0, 1, 2])).collect();
    seq(t, lf::SCREEN_DATA, 5, v);
    let v = vec![bit(rng, 2), bit(rng, 2), bit(rng, 2)];
    seq(t, lf::SPLIT_SCREEN, 0, v);
    let v = vec![bit(rng, 2)];
    seq(t, lf::IS_ETERNAL_CALM, 1, v);
    let v = vec![bit(rng, 2)];
    seq(t, lf::MOVIE_LOAD_IMAGE, 0, v);
    let v = vec![rng.below(3), rng.below(3)];
    seq(t, lf::GET_SCENARIO, 0, v);
    // the fade colour, alpha last
    let alpha = rng.pick(&[1.0f32, 0.5, f32::NAN]).to_bits();
    let rgb = [
        0.25f32.to_bits(),
        0.5f32.to_bits(),
        0.75f32.to_bits(),
        alpha,
    ];
    t.stub_ex(
        lf::GET_FADE_COLOR,
        false,
        4,
        0,
        &Ret::Fixed(0),
        &fill_args(&rgb),
    );
    // the movie's environment may end the movie
    let movie_at = t.rebase(lg::MOVIE_ON);
    let end_movie = if rng.chance(2) {
        set_byte(movie_at, 0)
    } else {
        Vec::new()
    };
    t.stub_ex(lf::ENV_INIT_MOVIE, false, 0, 0, &Ret::Fixed(0), &end_movie);
    // the scenario print's imports
    t.stub_import(lg::IOB_FUNC_IAT, 0, 0, 0x0055_0000);
    t.stub_import(lg::FPRINTF_IAT, 3, 0, 0);
    main_loop_globals(t, rng);
}

#[test]
fn main_loop_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0082_0ae0);
        for i in 0..400 {
            t.reset();
            main_loop_case(t, &mut rng);
            let dt = rng.pick(&[1.0f32 / 60.0, 0.25, 0.0]).to_bits();
            let what = format!("Sg_MainLoop #{i}");
            let slot = t.frame_ebp(1) + 8;
            t.check_framed(
                &what,
                0x0082_0ae0,
                Conv::Cdecl,
                &[dt],
                lpc::MAIN_LOOP_FRAME,
                |m, ebp| {
                    lp::main_loop(m, ebp, dt, ebp + 8);
                    Out::None
                },
            );
            // the scaled step in the argument slot
            let a: u32 = Mem::read(&t.a, slot);
            let b: u32 = Mem::read(&t.b, slot);
            assert_eq!(a, b, "{what}: the argument slot");
        }
    });
}

#[test]
fn update_ffx_animate_and_thunk_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0082_2840);
        for i in 0..40 {
            t.reset();
            t.stubs(&[
                (lp::SET_INPUT_INFO, 0, 1),
                (lp::DYN_GEO_CLEAR, 0, 2),
                (lp::FILL_CLEAR_REQUEST, 0, 3),
                (lp::REMOVE_DELEGATIONS, 0, 4),
                (lp::FILL_KICK, 1, 0x55),
            ]);
            // the voice hold runs out after a few frames
            let hold_at = t.rebase(lg::VOICE_HOLD);
            let frames = rng.below(3);
            t.g32(lg::VOICE_HOLD, frames);
            let ctr = t.words(&[frames + 1]);
            // dec dword [ctr]; mov eax, [ctr]; mov [hold], eax
            let mut c = vec![0xff, 0x0d];
            c.extend(ctr.to_le_bytes());
            c.push(0xa1);
            c.extend(ctr.to_le_bytes());
            c.push(0xa3);
            c.extend(hold_at.to_le_bytes());
            t.stub_ex(lp::MAIN_LOOP, false, 1, 0, &Ret::Fixed(0), &c);
            t.g32(lp::ASYNC_CONTROLLER_ON, u32::from(rng.chance(2)));
            let dt = rng.pick(&[1.0f32 / 60.0, 0.5]).to_bits();
            let what = format!("updateFFX #{i}");
            // the port's slot at the bottom of its oracle's stack, which no
            // call there reaches (the original's is its own argument word)
            let slot = t.frame_ebp(1) + 8;
            let port_slot = t.b.stack_range().start + 0x200;
            Mem::write(&mut t.b, port_slot, dt);
            t.check(&what, 0x0082_2840, Conv::Cdecl, &[dt], |m| {
                Out::U32(lp::update_ffx(m, dt, port_slot))
            });
            let a: u32 = Mem::read(&t.a, slot);
            let b: u32 = Mem::read(&t.b, port_slot);
            assert_eq!(a, b, "{what}: the argument slot");
        }
        for i in 0..120 {
            t.reset();
            t.stubs(&[
                (lp::TROPHY_ERROR, 0, rng.pick(&[0, 0, 0, 0, 1, 2])),
                (lp::IS_ETERNAL_CALM, 1, u32::from(rng.chance(2))),
                (lp::RESIZE_POST_TARGET, 0, 0),
                (lp::UPDATE_FFX, 1, 0),
                (lp::SCENE_ANIMATE, 0, 0),
            ]);
            let esc = vec![u32::from(rng.chance(2)), u32::from(rng.chance(2))];
            t.stub_ex(lp::IS_ESC_MENU_ACTIVE, false, 0, 0, &Ret::Seq(esc), &[]);
            t.stub(lp::LOAD_SPECIAL_TEXTURE, true, 3, 3, 0);
            t.stub(lp::UPDATE_GLOBAL_TIME, true, 0, 0, 0);
            t.stub(lp::APPLICATION_ANIMATE, true, 0, 0, 0x0aa0 + i);
            t.stub_import(lp::RUN_CALLBACKS_IAT, 0, 0, 0);
            t.g8(lp::STEAM_READY, u8::from(rng.chance(2)));
            t.g32(lp::CALM_TEXTURE_DONE, u32::from(rng.chance(2)));
            t.g32(lp::MENU_TEXTURES, 0x0066_0000);
            t.g32(lg::MAP, rng.below(3));
            let this = t.zeros(0x400);
            t.w8(this + 0x3ad, u8::from(rng.chance(2)));
            // the YIQ update may close the menu (the flag is read again)
            let extra = if rng.chance(2) {
                set_byte(this + 0x3ad, 0)
            } else {
                Vec::new()
            };
            t.stub_ex(lp::UPDATE_MAP_YIQ, false, 0, 0, &Ret::Fixed(0), &extra);
            t.w32(this + 0x24, rng.pick(&[1.0f32 / 60.0, 0.5]).to_bits());
            let what = format!("FFXApplication::animate #{i}");
            t.check(&what, 0x0042_f520, Conv::Thiscall(this), &[], |m| {
                Out::U32(lp::animate(m, this))
            });
        }
        t.reset();
        t.stub(lp::CLEAR_VSYNC_HANDLER, false, 0, 0, 0xffff_ffff);
        t.check("0x00822750", 0x0082_2750, Conv::Cdecl, &[], |m| {
            Out::U32(lp::clear_vsync_thunk(m))
        });
    });
}
