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
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        ffx_oracle::check_width(
            "Host::call_st0_f64",
            addr,
            args.len(),
            host::max_words(c, true),
        );
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0_f64().expect("the callee leaves ST0 set")
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

// ---- round 2: FFXApplication's other methods and start-up ----

use ffx_sg::m5main_init::bridge as init;

/// `mov dword [ecx], v`.
fn set_at_ecx(v: u32) -> Vec<u8> {
    let mut c = vec![0xc7, 0x01];
    c.extend(v.to_le_bytes());
    c
}

#[test]
fn exit_application_matches_the_original() {
    Twins::each(|t| {
        for err in [0, 1, 0x100] {
            for up in [0u8, 1, 0x80] {
                t.reset();
                t.stubs(&[
                    (init::TTY_WRITE, 2, 0),
                    (init::FREE_BLOCK, 0, 1),
                    (init::GRAPHIC_FINALIZE, 0, 2),
                    (init::CLEAR_VSYNC, 0, 3),
                    (init::TROPHY_GET_ERROR, 0, err),
                    (init::FMOD_FINALIZE, 0, 4),
                    (init::SYNC_SHUTDOWN, 0, 5),
                    (init::FIOS_FINALIZE, 0, 6),
                ]);
                t.stub(init::BASE_EXIT_APPLICATION, true, 0, 0, 0x77);
                t.g8(init::FMOD_UP, up);
                let this = t.zeros(0x400);
                let what = format!("exitApplication err {err:#x} up {up}");
                t.check(&what, 0x0042_f600, Conv::Thiscall(this), &[], |m| {
                    Out::U32(init::exit_application(m, this))
                });
            }
        }
    });
}

#[test]
fn scene_entry_exit_and_pre_init_match_the_original() {
    Twins::each(|t| {
        t.reset();
        for (k, f) in init::EXIT_SCENE_STEPS.into_iter().enumerate() {
            t.stub(f, false, 0, 0, k as u32);
        }
        t.stub(init::BASE_EXIT_SCENE, true, 0, 0, 0x55);
        let this = t.zeros(0x400);
        t.check("exitScene", 0x0042_f660, Conv::Thiscall(this), &[], |m| {
            Out::U32(init::exit_scene(m, this))
        });
        t.reset();
        t.stub(init::GRAPHIC_INIT_SCENE, false, 0, 0, 9);
        t.stub(init::BASE_INIT_SCENE, true, 0, 0, 0x56);
        t.check("initScene", 0x0042_f8e0, Conv::Thiscall(this), &[], |m| {
            Out::U32(init::init_scene(m, this))
        });
        t.reset();
        for (k, f) in init::REGISTER_TYPES.into_iter().enumerate() {
            t.stub(f, false, 0, 0, 0x10 + k as u32);
        }
        t.stub(init::BASE_PRE_PHYRE_INIT, true, 0, 0, 0x57);
        t.check(
            "prePhyreInit",
            0x0042_f8f0,
            Conv::Thiscall(this),
            &[],
            |m| Out::U32(init::pre_phyre_init(m, this)),
        );
    });
}

#[test]
fn input_methods_match_the_original() {
    Twins::each(|t| {
        for utility in [0u8, 1, 0xff] {
            t.reset();
            t.stub(init::INPUT_UPDATE_ALL, false, 0, 0, 3);
            t.stub(init::BASE_HANDLE_INPUTS, true, 0, 0, 0x58);
            let this = t.zeros(0x400);
            t.w8(this + 0x3ad, utility);
            let what = format!("handleInputs utility {utility}");
            t.check(&what, 0x0042_f770, Conv::Thiscall(this), &[], |m| {
                Out::U32(init::handle_inputs(m, this))
            });
        }
        for swap in [false, true] {
            for sync in [false, true] {
                t.reset();
                let this = t.zeros(0x400);
                let mapper = t.zeros(0x40);
                let other = t.zeros(0x40);
                t.w32(this + 0x2b8, mapper);
                t.stub(init::SET_POLL_FLAG, false, 1, 0, 0);
                // the update may swap the mapper (the pointer is read again)
                let extra = if swap {
                    set_word(this + 0x2b8, other)
                } else {
                    Vec::new()
                };
                t.stub_ex(init::MAPPER_UPDATE, true, 0, 0, &Ret::Fixed(1), &extra);
                t.stub(init::MAPPER_LATCH, true, 0, 0, 2);
                t.stub(init::INPUT_OD_UPDATE, false, 0, 0, 0x99);
                let what = format!("pollInput sync {sync} swap {swap}");
                if sync {
                    let slots: Vec<(u32, u32, Vec<u8>)> =
                        (0..8).map(|k| (0, 0x60 + k, Vec::new())).collect();
                    t.fake_vtable(this, &slots);
                    t.check(&what, 0x0042_f7c0, Conv::Thiscall(this), &[], |m| {
                        Out::U32(init::poll_input_sync(m, this))
                    });
                } else {
                    t.check(&what, 0x0042_f790, Conv::Thiscall(this), &[], |m| {
                        Out::U32(init::poll_input_async(m, this))
                    });
                }
            }
        }
    });
}

#[test]
fn init_systems_matches_the_original() {
    Twins::each(|t| {
        for steam in [0u8, 1, 0x40] {
            t.reset();
            let mgr = t.zeros(0x40);
            t.stubs(&[
                (init::SAVE_MANAGER_CREATE, 0, 1),
                (init::SAVE_FOLDER, 0, 2),
                (init::STATS_CREATE, 0, 3),
                (init::TROPHY_CREATE, 0, 4),
                (init::SAVE_MANAGER_INSTANCE, 0, mgr),
            ]);
            t.stub(init::SAVE_INIT_SYS_MODULES, true, 0, 0, 0x5a);
            t.g8(init::STEAM_READY, steam);
            let what = format!("Game_InitSystems steam {steam}");
            t.check(&what, 0x0064_7fe0, Conv::Cdecl, &[], |m| {
                Out::U32(init::init_systems(m))
            });
        }
    });
}

#[test]
fn init_file_system_matches_the_original() {
    Twins::each(|t| {
        // registered: al 1 (upper bytes set too); 0x100 has al 0
        for ok in [1, 0x101, 0, 0x100] {
            for inst in [0, 0x0040_0000] {
                for refs in [1, 2, 0] {
                    for lang in [0, 1, 2, 3, 4, 0xffff_ffff] {
                        if ok & 0xff != 0 && (inst != 0 || refs != 1) {
                            continue;
                        }
                        t.reset();
                        let big = t.zeros(0x40);
                        let loc = t.words(&[lang, 0, 0, 0]);
                        t.stubs(&[
                            (init::SET_STREAM_PREFIX, 1, 0),
                            (init::BIG_FILE_CREATE, 0, 0),
                            (init::BIG_FILE_GET, 0, big),
                            (init::FIND_STRING_RESOURCE, 2, inst),
                            (init::RESET_TO_LAUNCHER, 0, 0),
                            (init::LOC_CREATE, 0, 0),
                            (init::LOC_INSTANCE, 0, loc),
                            (init::GET_STREAM_PREFIX, 0, 0x0123_4567),
                            (init::FIOS_INITIALIZE, 1, 0x0bad),
                        ]);
                        t.stub(init::BIG_FILE_SET_PREFIX, true, 1, 1, 0);
                        t.stub(init::BIG_FILE_REGISTER, true, 1, 1, ok);
                        t.stub(init::LOC_INITIALIZE, true, 0, 0, 0);
                        t.stub_import(init::SHOW_CURSOR_IAT, 1, 1, 1);
                        t.stub_import(init::MESSAGE_BOX_IAT, 4, 4, 1);
                        // the empty string's block and the loaded one
                        let owner = t.zeros(0x10);
                        t.fake_vtable(owner, &[(1, 0, Vec::new()), (1, 0, Vec::new())]);
                        let nil = t.words(&[owner, 0, 0, refs, 0]);
                        let loaded = t.words(&[owner, 4, 4, refs, 0x41]);
                        let mgr = t.rebase(init::STRING_MANAGER);
                        t.fake_vtable(
                            mgr,
                            &[
                                (0, 0, Vec::new()),
                                (0, 0, Vec::new()),
                                (0, 0, Vec::new()),
                                (0, nil, Vec::new()),
                            ],
                        );
                        let extra = set_at_ecx(loaded + 0x10);
                        t.stub_ex(init::LOAD_STRING, true, 2, 2, &Ret::Fixed(1), &extra);
                        let what = format!(
                            "Game_InitFileSystem ok {ok:#x} inst {inst:#x} refs {refs} lang {lang:#x}"
                        );
                        t.check_framed(
                            &what,
                            0x0067_95b0,
                            Conv::Cdecl,
                            &[],
                            init::INIT_FS_FRAME,
                            |m, ebp| Out::U32(init::init_file_system(m, ebp)),
                        );
                    }
                }
            }
        }
    });
}

#[test]
fn init_application_matches_the_original() {
    Twins::each(|t| {
        for arg in [&b"_ECalm\0"[..], b"_ECal\0", b"_ECalmX\0", b"\0", b"FFX\0"] {
            for argc in [0, 1, 2, 5, 0xffff_ffff] {
                for err in [0, 1] {
                    t.reset();
                    t.stubs(&[
                        (init::TTY_WRITE, 2, 0),
                        (init::INIT_FILE_SYSTEM, 0, 1),
                        (init::INIT_ENCODING, 0, 2),
                        (init::INIT_SYSTEMS, 0, 3),
                        (init::SAVE_DATA_INIT, 0, 4),
                        (init::GRAPHIC_INITIALIZE, 0, 5),
                        (init::TROPHY_GET_ERROR, 0, err),
                        (init::FMOD_INITIALIZE, 0, 6),
                        (init::SYNC_INITIALIZE, 0, 7),
                    ]);
                    let input = t.zeros(0x40);
                    t.stub(init::INPUT_MANAGER, false, 0, 0, input);
                    t.stub(init::BUILD_PAD_MAP, true, 0, 0, 8);
                    t.stub(init::SET_WINDOW_TITLE, true, 1, 1, 9);
                    t.g8(init::FMOD_UP, 0);
                    let s = t.alloc(arg);
                    let argv = t.words(&[0x0011_2233, s, 0]);
                    let this = t.zeros(0x400);
                    t.w32(this + 0x3b0, 0x3f80_0000);
                    let what = format!("initApplication {arg:?} argc {argc:#x} err {err}");
                    t.check(
                        &what,
                        0x0042_f7f0,
                        Conv::Thiscall(this),
                        &[argv, argc],
                        |m| Out::U32(init::init_application(m, this, argv, argc)),
                    );
                }
            }
        }
    });
}

#[test]
fn render_matches_the_original() {
    Twins::each(|t| {
        for utility in [0u8, 1] {
            for err in [0, 1, 2] {
                for abmap in [None, Some(3), Some(4), Some(0xffff_ffff), Some(9)] {
                    for esc in [0, 1] {
                        if utility != 0 && (err != 0 || abmap.is_some() || esc != 0) {
                            continue;
                        }
                        t.reset();
                        t.stubs(&[
                            (init::RENDER_CAPTURE_IGGY, 0, 1),
                            (init::TROPHY_GET_ERROR, 0, err),
                            (init::SET_CLOTH_BUFFERS, 0, 2),
                            (init::GRAPHIC_RENDER, 0, 3),
                            (init::VSYNC_WAKE, 0, 4),
                            (init::IS_ESC_MENU_ACTIVE, 0, esc),
                            (init::COPY_POST_BUFFER, 0, 5),
                        ]);
                        let obj = match abmap {
                            None => 0,
                            Some(s) => t.words(&[0, s]),
                        };
                        t.g32(init::GRAPHIC_ABMAP, obj);
                        let this = t.zeros(0x400);
                        t.w8(this + 0x3ad, utility);
                        let what =
                            format!("render utility {utility} err {err} abmap {abmap:?} esc {esc}");
                        t.check(&what, 0x0042_f930, Conv::Thiscall(this), &[], |m| {
                            Out::U32(init::render(m, this))
                        });
                    }
                }
            }
        }
    });
}

#[test]
fn settings_reset_and_animated_materials_match_the_original() {
    Twins::each(|t| {
        for scene in [false, true] {
            t.reset();
            let obj = if scene { t.zeros(0x40) } else { 0 };
            t.g32(init::PHYRE_SCENE, obj);
            t.stub(init::SCENE_APPLY_SETTINGS, true, 0, 0, 0x33);
            let what = format!("ApplySettings scene {scene}");
            t.check(&what, 0x0042_f9b0, Conv::Cdecl, &[], |m| {
                Out::U32(init::apply_settings(m))
            });
        }
        for r in [0, 1, 0xffff_ffff] {
            t.reset();
            t.stub_import(init::SEND_MESSAGE_IAT, 4, 4, r);
            t.g32(init::HWND, 0x0005_0607);
            t.g32(init::RESET_PENDING, 0x1234);
            t.check("PostReset", 0x0042_fa70, Conv::Cdecl, &[], |m| {
                Out::U32(init::post_reset(m))
            });
        }
        let mut rng = Rng(0x0082_9c00);
        let floats = [
            0.0f32.to_bits(),
            (-0.0f32).to_bits(),
            1.5f32.to_bits(),
            f32::INFINITY.to_bits(),
            0x7fc0_1234, // a quiet NaN with a payload
            0xffc0_0000,
            0x0000_0001, // a denormal
        ];
        for i in 0..24 {
            t.reset();
            let player = if i % 6 == 0 {
                0
            } else {
                let p = t.zeros(0x900);
                t.w32(p + 0x830, rng.word());
                p
            };
            t.stub(init::GET_PLAYER, false, 0, 0, player);
            t.stub(init::SET_ANIMATED_MATERIALS, false, 11, 0, 0x4400 + i);
            let a: [u32; 10] = std::array::from_fn(|k| {
                if (3..9).contains(&k) {
                    if rng.chance(2) {
                        rng.pick(&floats)
                    } else {
                        rng.next() & 0xbfff_ffff // finite
                    }
                } else {
                    rng.word()
                }
            });
            let what = format!("player animated materials #{i}");
            t.check(&what, 0x0082_9c00, Conv::Cdecl, &a, |m| {
                Out::U32(init::player_animated_materials(m, a))
            });
        }
    });
}

// ---- round 2: the task layer, the CD callbacks, the debug printfs ----

use ffx_sg::m5main_task::bridge as task;

#[test]
fn task_warnings_match_the_original() {
    Twins::each(|t| {
        type Port = fn(&mut G) -> u32;
        let cases: [(u32, Port); 4] = [
            (0x0077_a4e0, |m| task::warn(m, task::MSG_TASK_WAIT)),
            (0x0077_a4c0, |m| task::warn(m, task::MSG_TASK_INIT_ASM)),
            (0x0076_f0f0, |m| task::warn(m, task::MSG_CAMERA_SET)),
            (0x0076_f170, |m| task::warn(m, task::MSG_0076F170)),
        ];
        for (addr, port) in cases {
            t.reset();
            t.stub(task::DBG_PRINTF, false, 1, 0, 0x1234 + addr);
            let what = format!("warning {addr:#x}");
            t.check(&what, addr, Conv::Cdecl, &[], |m| Out::U32(port(m)));
        }
        t.reset();
        t.stub(task::DBG_PRINTF, false, 1, 0, 0x99);
        t.check(
            "task_create_asm",
            0x0077_a4b0,
            Conv::Cdecl,
            &[1, 2, 3],
            |m| Out::U32(task::task_create_asm(m)),
        );
        t.reset();
        t.stub(task::TASK_CREATE_ASM, false, 0, 0, 0x98);
        t.check("0x0077a320", 0x0077_a320, Conv::Cdecl, &[], |m| {
            Out::U32(task::task_create_asm_thunk(m))
        });
    });
}

/// A task record (0x60 bytes, the stack area after it) with known words.
fn task_record(t: &mut Twins, rng: &mut Rng) -> u32 {
    let w: Vec<u32> = (0..0x40).map(|_| rng.word()).collect();
    t.words(&w)
}

#[test]
fn task_init_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0077_a1d0);
        let groups = t.rebase(task::GROUPS);
        for i in 0..40 {
            t.reset();
            t.stub(task::MOVIE_ERR, false, 3, 0, 0);
            t.stub(task::TASK_INIT_ASM, false, 0, 0, 0x11);
            // the first `used` slots taken (all eight sometimes)
            let used = if i % 5 == 0 { 8 } else { rng.below(8) };
            for k in 0..9 {
                let v = if k < used { 0x0050_0000 + k } else { 0 };
                t.patch(groups + 4 * k, &v.to_le_bytes());
            }
            let rec = task_record(t, &mut rng) + rng.below(16);
            let id = rng.pick(&[0, 1, 2, 7, 0x100]);
            let flag = rng.word();
            for (cur, v) in [
                (task::CUR_ID, rng.word()),
                (task::CUR_STACK_TOP, rng.word()),
                (task::CUR_STACK_END, rng.word()),
                (task::CUR_STACK_BASE, rng.word()),
            ] {
                t.g32(cur, v);
            }
            let what = format!("sa_task_init2 #{i} used {used}");
            t.check(&what, 0x0077_a1d0, Conv::Cdecl, &[id, rec, flag], |m| {
                Out::U32(task::task_init2(m, id, rec, flag))
            });
        }
        for id in [0, 3] {
            t.reset();
            t.stubs(&[
                (task::TASK_BUFF_SIZE, 1, 0x1340),
                (task::USER_MALLOC, 1, 0x0070_0000),
                (task::TASK_INIT2, 3, 0x0070_0010),
            ]);
            t.check("sa_task_init", 0x0077_a1a0, Conv::Cdecl, &[id], |m| {
                Out::U32(task::task_init(m, id))
            });
        }
    });
}

#[test]
fn task_groups_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0077_a390);
        let groups = t.rebase(task::GROUPS);
        for g in 0..8 {
            for heap in [0, 1] {
                t.reset();
                let rec = task_record(t, &mut rng);
                t.w32(rec + 0x24, heap);
                t.patch(groups + 4 * g, &rec.to_le_bytes());
                for (cur, v) in [
                    (task::CUR_ID, rng.word()),
                    (task::CUR_STACK_TOP, rng.word()),
                    (task::CUR_STACK_END, rng.word()),
                    (task::CUR_STACK_BASE, rng.word()),
                    (task::CUR_RECORD, rng.word()),
                ] {
                    t.g32(cur, v);
                }
                // the asm step changes the current words (put back after)
                let mut extra = set_word(t.rebase(task::CUR_ID), 0x5151);
                extra.extend(set_word(t.rebase(task::CUR_STACK_TOP), 0x5252));
                t.stub_ex(
                    task::TASK_CREATE_ASM,
                    false,
                    3,
                    0,
                    &Ret::Fixed(0x77),
                    &extra,
                );
                t.stub(task::USER_FREE, false, 1, 0, 0x66);
                let what = format!("task_create_in_group {g} heap {heap}");
                t.check(&what, 0x0077_a390, Conv::Cdecl, &[g, 4, 5, 6], |m| {
                    Out::U32(task::task_create_in_group(m, g, 4, 5, 6))
                });
                let what = format!("sa_task_run_in_group {g}");
                t.check(&what, 0x0077_a420, Conv::Cdecl, &[g], |m| {
                    Out::U32(task::task_run_in_group(m, g))
                });
                let what = format!("sa_task_delete_in_group {g} heap {heap}");
                t.check(&what, 0x0077_a3f0, Conv::Cdecl, &[g], |m| {
                    Out::U32(task::task_delete_in_group(m, g))
                });
            }
        }
        let create = [
            0x0077_9f80,
            0x0077_9fa0,
            0x0077_9fc0,
            0x0077_9fe0,
            0x0077_a000,
            0x0077_a020,
            0x0077_a040,
        ];
        for (g, addr) in (0u32..).zip(create) {
            t.reset();
            t.stub(task::TASK_CREATE_IN_GROUP, false, 4, 0, 0x10 + g);
            let what = format!("create entry {g}");
            t.check(&what, addr, Conv::Cdecl, &[7, 8, 9], |m| {
                Out::U32(task::task_create_group(m, g, 7, 8, 9))
            });
        }
        for g in 0..8 {
            t.reset();
            t.stub(task::TASK_RUN_IN_GROUP, false, 1, 0, 0x20 + g);
            t.stub(task::TASK_DELETE_IN_GROUP, false, 1, 0, 0x30 + g);
            let what = format!("run entry {g}");
            t.check(&what, 0x0077_a080 + 0x10 * g, Conv::Cdecl, &[], |m| {
                Out::U32(task::task_run_group(m, g))
            });
            let what = format!("delete entry {g}");
            t.check(&what, 0x0077_a100 + 0x10 * g, Conv::Cdecl, &[], |m| {
                Out::U32(task::task_delete_group(m, g))
            });
        }
    });
}

#[test]
fn chgname_printf_and_read_helpers_match_the_original() {
    Twins::each(|t| {
        for ready in [0u8, 1, 0x80] {
            t.reset();
            t.g8(task::CHGNAME_READY, ready);
            for k in 0..32 {
                t.g32(task::CHGNAME_FROM + 4 * k, 0x1111_0000 + k);
            }
            t.check("read_chgname_init", 0x00a4_3130, Conv::Cdecl, &[], |m| {
                task::chgname_init(m);
                Out::None
            });
        }
        t.reset();
        t.stub(task::PRINTF2, false, 2, 0, 0x42);
        t.check_framed(
            "sa_printf_0076de70",
            0x0076_de70,
            Conv::Cdecl,
            &[0x00b0_0000, 1, 2, 3],
            4,
            |m, ebp| Out::U32(task::printf(m, 0x00b0_0000, ebp + 8)),
        );
        t.reset();
        t.stubs(&[(task::PRINTF_SET, 1, 1), (task::FLUSH_CACHE, 1, 0x77)]);
        t.check("printf_cut", 0x0076_de90, Conv::Cdecl, &[], |m| {
            Out::U32(task::printf_cut(m))
        });
        t.reset();
        t.stub(task::REGIST_BG_THREAD, false, 1, 0, 0x78);
        t.check("read_thread_init", 0x0076_e630, Conv::Cdecl, &[], |m| {
            Out::U32(task::read_thread_init(m))
        });
        t.reset();
        let cb = t.stub_code(0x0cb0, false, 1, 0, &Ret::Fixed(0x79), &[]);
        let req = t.words(&[0x0101_0101; 16]);
        t.w32(req + 0x20, cb);
        t.check("sa_read_callback", 0x0076_e640, Conv::Cdecl, &[req], |m| {
            Out::U32(task::read_callback(m, req))
        });
    });
}

#[test]
fn cd_callback_init_matches_the_original() {
    Twins::each(|t| {
        for env in [1, 0, 2] {
            for debug in [0, 1] {
                for flag in [0, 1, 0x100] {
                    t.reset();
                    t.stubs(&[
                        (task::TASK_START, 0, 1),
                        (task::DBG_PRINTF, 1, 0x5050),
                        (task::SET_DEF_DRAW_ENV2, 0, env),
                        (task::CD_ENV_INIT, 1, 2),
                        (task::IS_DEBUG, 0, debug),
                        (task::MS_ENV_INIT, 0, 3),
                    ]);
                    let what = format!("sa_cd_callback_init env {env} debug {debug} flag {flag}");
                    let slot = t.frame_ebp(1) + 8;
                    t.check_framed(&what, 0x0076_f100, Conv::Cdecl, &[flag], 4, |m, ebp| {
                        Out::U32(task::cd_callback_init(m, flag, ebp + 8))
                    });
                    let a: u32 = Mem::read(&t.a, slot);
                    let b: u32 = Mem::read(&t.b, slot);
                    assert_eq!(a, b, "{what}: the argument slot");
                }
            }
        }
    });
}

#[test]
fn movie_resets_and_counters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0077_0340);
        for handler in [false, true] {
            t.reset();
            t.stub(task::DBG_PRINTF, false, 1, 0, 0x60);
            t.stub(task::TK_FREE, false, 1, 0, 0x61);
            for w in task::MOVIE_END_WORDS
                .into_iter()
                .chain(task::MOVIE_RESET_WORDS)
                .chain(task::MOVIE_RESET_HALVES)
            {
                t.g32(w, rng.word() | 1);
            }
            let obj = if handler {
                let f = t.stub_code(0x0ab0, false, 0, 0, &Ret::Fixed(0x62), &[]);
                t.words(&[0, f])
            } else {
                0
            };
            t.g32(task::MOVIE_HANDLER, obj);
            t.g32(task::MOVIE_HANDLER_DONE, 5);
            let what = format!("movie_end handler {handler}");
            t.check(&what, 0x0077_0340, Conv::Cdecl, &[], |m| {
                Out::U32(task::movie_end(m))
            });
            t.check("movie_reset", 0x0076_f3b0, Conv::Cdecl, &[], |m| {
                Out::U32(task::movie_reset(m))
            });
        }
        for (n, s) in [(5, 3), (3, 5), (0, 0)] {
            t.reset();
            t.stubs(&[
                (task::SAVE_BATTLE_COUNT, 0, n),
                (task::ENCOUNT_EVENT_START, 0, s),
                (task::GET_GIL, 0, n * 100),
            ]);
            t.check("battles since event", 0x0077_22a0, Conv::Cdecl, &[], |m| {
                Out::U32(task::battles_since_event(m))
            });
            t.check("battle count thunk", 0x0077_2310, Conv::Cdecl, &[], |m| {
                Out::U32(task::battle_count_thunk(m))
            });
            t.check("gil thunk", 0x0077_2550, Conv::Cdecl, &[], |m| {
                Out::U32(task::gil_thunk(m))
            });
        }
        for v in [0, 0x1234_5678] {
            t.reset();
            t.g32(task::WORD_012FBBF4, v);
            t.check("0x00820520", 0x0082_0520, Conv::Cdecl, &[], |m| {
                Out::U32(task::get_012fbbf4(m))
            });
        }
    });
}

#[test]
fn decode_and_ring_printf_match_the_original() {
    Twins::each(|t| {
        for kind in [0u8, 1, 7, 0xff] {
            t.reset();
            // mov eax, [esp + 0xc]; add dword [eax], 3 (the decoder advances src)
            let extra = [0x8b, 0x44, 0x24, 0x0c, 0x83, 0x00, 0x03];
            let f = t.stub_code(0x0dec, false, 3, 0, &Ret::Fixed(0x63), &extra);
            let entry = t.rebase(task::DECODERS) + 4 * u32::from(kind);
            t.patch(entry, &f.to_le_bytes());
            let src = t.alloc(&[kind, 2, 3, 4]);
            let out = t.zeros(0x10);
            let slot = t.frame_ebp(2) + 0xc;
            Mem::write(&mut t.b, slot, src);
            let what = format!("sa_decode kind {kind}");
            t.check_framed(&what, 0x00a4_3de0, Conv::Cdecl, &[out, src], 4, |m, ebp| {
                Out::U32(task::decode(m, out, src, ebp + 8))
            });
        }
        let mut rng = Rng(0x0076_b860);
        for index in [0, 1, 0xfe, 0xff, 0x100, 0x7fff_ffff, 0xffff_ffff] {
            t.reset();
            t.stub_import(task::VSPRINTF_IAT, 3, 0, 0x64);
            t.g32(task::RING_INDEX, index);
            let mut args = vec![0x00b1_2340];
            args.extend((0..16).map(|_| rng.word()));
            let a: [u32; 16] = std::array::from_fn(|k| args[k + 1]);
            let what = format!("printf ring index {index:#x}");
            t.check_framed(
                &what,
                0x0076_b860,
                Conv::Cdecl,
                &args,
                task::RING_FRAME,
                |m, ebp| Out::U32(task::ring_printf(m, ebp, args[0], a)),
            );
        }
    });
}

// ---- round 2: the debug captures and the ID draw ----

use ffx_sg::m5main_dbg2::bridge as dbg2;

/// The import at `slot` bound to recording stubs that log the words each
/// format (the second argument) takes; `other` words for the rest.
fn varargs_stub(t: &mut Twins, slot: u32, fmts: &[(u32, u32)], other: u32) {
    let mut c = vec![0x8b, 0x54, 0x24, 0x08]; // mov edx, [esp + 8]
    for &(fmt, n) in fmts {
        let to = t.stub_code(slot, false, n, 0, &Ret::Fixed(0x21), &[]);
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(fmt).to_le_bytes()); // cmp edx, fmt
        c.extend([0x75, 7]); // jne next
        c.push(0xb8);
        c.extend(to.to_le_bytes()); // mov eax, to
        c.extend([0xff, 0xe0]); // jmp eax
    }
    let rest = t.stub_code(slot, false, other, 0, &Ret::Fixed(0x21), &[]);
    c.push(0xb8);
    c.extend(rest.to_le_bytes()); // mov eax, rest
    c.extend([0xff, 0xe0]); // jmp eax
    let at = t.code(&c);
    let slot = t.rebase(slot);
    t.patch(slot, &at.to_le_bytes());
}

#[test]
fn capture_to_file_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_a690);
        // the stored frame (one line more: the blend reads below the last)
        let pixels: Vec<u8> = (0..0xd_0800).map(|_| rng.next() as u8).collect();
        let a = t.alloc(&pixels);
        let b = t.zeros(0x100);
        let out = t.zeros(0x13_8000);
        let name = t.alloc(b"shot.bmp\0");
        for odd in [0u8, 1] {
            for (interlaced, blend) in [(0u8, 0u8), (0, 1), (1, 0), (1, 1)] {
                t.reset();
                varargs_stub(
                    t,
                    dbg2::SPRINTF_IAT,
                    &[(dbg2::FMT_CONCAT, 4), (dbg2::FMT_CHMOD, 3)],
                    2,
                );
                t.stub_ex(
                    dbg2::USER_MALLOC,
                    false,
                    1,
                    0,
                    &Ret::Seq(vec![a, b, out]),
                    &[],
                );
                t.stubs(&[
                    (dbg2::SCE_OPEN, 2, 7),
                    (dbg2::SCE_WRITE, 3, 0x36),
                    (dbg2::SCE_CLOSE, 1, 0),
                    (dbg2::SET_DEF_STORE_IMAGE, 8, 0),
                    (dbg2::EXEC_STORE_IMAGE, 2, 0),
                    (dbg2::GS_SYNC_PATH, 2, 0),
                    (dbg2::FLUSH_CACHE, 1, 0),
                    (dbg2::SYSTEM_ON_HOST, 2, 0),
                    (dbg2::USER_FREE, 1, 0x5a),
                ]);
                t.g8(dbg2::SG_FRAME, odd + 2);
                t.g8(dbg2::CAPTURE_INTERLACED, interlaced);
                t.g8(dbg2::CAPTURE_BLEND, blend);
                let what =
                    format!("Sg_CaptureToFile odd {odd} interlaced {interlaced} blend {blend}");
                t.check_framed(
                    &what,
                    0x0083_a690,
                    Conv::Cdecl,
                    &[name],
                    dbg2::CAPTURE_FILE_FRAME,
                    |m, ebp| Out::U32(dbg2::capture_to_file(m, ebp, name)),
                );
            }
        }
    });
}

#[test]
fn id_triangles_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0084_47a0);
        for i in 0..60 {
            t.reset();
            // vertices -2..16, the base two records in
            let vw: Vec<u8> = (0..18 * 8).map(|_| rng.next() as u8).collect();
            let verts = t.alloc(&vw) + 16;
            let n = if i % 10 == 0 { 0 } else { rng.below(6) };
            let mut list = Vec::new();
            for _ in 0..n {
                for _ in 0..3 {
                    let idx = rng.below(18) as i32 - 2;
                    list.extend((idx as i16).to_le_bytes());
                }
                list.extend((rng.next() as u16).to_le_bytes());
                list.extend(rng.next().to_le_bytes());
                list.extend(rng.word().to_le_bytes()); // the shade word
                list.extend(rng.next().to_le_bytes());
            }
            list.extend([0; 16]);
            let list = t.alloc(&list);
            let model = t.zeros(0x20);
            t.write(model + 8, &(n as i16).to_le_bytes());
            t.w32(model + 0xc, 0x0bad_0000 + i);
            t.stub(dbg2::GET_PTR32, false, 1, 0, list);
            let proj: Vec<u32> = (0..=3 * n).map(|_| u32::from(rng.chance(4))).collect();
            let pw: Vec<u32> = (0..0x100).map(|_| rng.next()).collect();
            let packet = t.words(&pw);
            // half the cases: the projection moves the packet global (the
            // original reads it again after each call)
            let other = t.zeros(0x400);
            let extra = if i % 2 == 1 {
                set_word(t.rebase(dbg2::ID_PACKET), other)
            } else {
                Vec::new()
            };
            t.stub_ex(dbg2::ID_PROJECT, false, 3, 0, &Ret::Seq(proj), &extra);
            let shades: Vec<u32> = (0..=n).map(|_| rng.word()).collect();
            t.stub_ex(dbg2::ID_GET_SHADE, false, 1, 0, &Ret::Seq(shades), &[]);
            let m = 0x00ab_cd00;
            let what = format!("id triangles #{i} n {n}");
            t.check_framed(
                &what,
                0x0084_47a0,
                Conv::Cdecl,
                &[verts, model, m, packet],
                dbg2::ID_TRIANGLES_FRAME,
                |g, ebp| Out::U32(dbg2::id_triangles(g, ebp, verts, model, m, packet)),
            );
        }
    });
}

#[test]
fn id_draw_main_matches_the_original() {
    Twins::each(|t| {
        let zooms = [1.0f32, 3.0, 0.0, -2.5, 0.1];
        for (k, zoom) in zooms.into_iter().enumerate() {
            for (has_id, on, parts) in [(false, 1, 0), (true, 0, 2), (true, 1, 0), (true, 1, 3)] {
                t.reset();
                let id = if has_id {
                    let p = t.zeros(0x20);
                    t.write(p + 6, &(parts as i16).to_le_bytes());
                    t.w32(p + 0x18, 0x1818);
                    t.w32(p + 0x1c, 0x1c1c);
                    p
                } else {
                    0
                };
                let cell = t.words(&[zoom.to_bits()]);
                t.stubs(&[
                    (dbg2::PACKET_GET_PTR, 0, 0x0077_0000),
                    (dbg2::ID_GET_PTR, 0, id),
                    (dbg2::SCREEN_MATRIX, 0, 0x0066_0000),
                    (dbg2::CAMERA_MATRIX, 0, 0x0066_0040),
                    (dbg2::MEMSET, 3, 0),
                    (dbg2::UNIT_MATRIX, 1, 0),
                    (dbg2::MUL_MATRIX, 3, 0),
                    (dbg2::FLUSH_CACHE, 1, 0),
                    (dbg2::DMA_SEND, 2, 0),
                    (dbg2::GS_SYNC_PATH, 2, 0),
                ]);
                // the model packet moves the channel word (read again)
                let extra = set_word(t.rebase(dbg2::DMA_CHANNEL), 0x2000_0000 + k as u32);
                t.stub_ex(dbg2::ID_MODEL_PACKET, false, 5, 0, &Ret::Fixed(0), &extra);
                t.stub_ex(dbg2::ID_GET_ZOOM, false, 0, 0, &Ret::Fixed(0), &fld(cell));
                let gets: Vec<u32> = (0..=2 * parts).map(|j| 0x0055_0000 + 0x100 * j).collect();
                t.stub_ex(dbg2::GET_PTR32, false, 1, 0, &Ret::Seq(gets), &[]);
                t.g32(dbg2::ID_DRAW_ON, on);
                t.g32(dbg2::DMA_CHANNEL, 0x1000_9000 + k as u32);
                let what = format!("Sg_IdDrawMain zoom {zoom} id {has_id} on {on} parts {parts}");
                t.check_framed(
                    &what,
                    0x0084_4a80,
                    Conv::Cdecl,
                    &[],
                    dbg2::ID_DRAW_FRAME,
                    |g, ebp| {
                        dbg2::id_draw_main(g, ebp);
                        Out::None
                    },
                );
            }
        }
    });
}

// ---- round 2: the Sg_Mem heaps, the allocation wrappers, small getters ----

use ffx_sg::m5main_mem::bridge as sgm;

/// `strchr(s, c)` in emulated code.
const STRCHR_CODE: [u8; 24] = [
    0x8b, 0x44, 0x24, 0x04, // mov eax, [esp + 4]
    0x8a, 0x54, 0x24, 0x08, // mov dl, [esp + 8]
    0x8a, 0x08, // mov cl, [eax]
    0x38, 0xd1, // cmp cl, dl
    0x74, 0x09, // je done
    0x84, 0xc9, // test cl, cl
    0x74, 0x03, // je null
    0x40, // inc eax
    0xeb, 0xf3, // jmp loop
    0x31, 0xc0, // null: xor eax, eax
    0xc3, // done: ret
];

/// `memcpy(d, s, n)` / a downward `memmove` in emulated code (forward
/// bytes).
const COPY_CODE: [u8; 23] = [
    0x56, 0x57, // push esi; push edi
    0x8b, 0x7c, 0x24, 0x0c, // mov edi, [esp + 0xc]
    0x8b, 0x74, 0x24, 0x10, // mov esi, [esp + 0x10]
    0x8b, 0x4c, 0x24, 0x14, // mov ecx, [esp + 0x14]
    0xf3, 0xa4, // rep movsb
    0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp + 0xc]
    0x5f, 0x5e, // pop edi; pop esi
    0xc3, // ret
];

#[test]
fn sg_alloc_wrappers_match_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stub(sgm::USER_REALLOC, false, 2, 0, 0x71);
        t.check(
            "Sg_MemRealloc",
            0x0083_c480,
            Conv::Cdecl,
            &[1, 2, 0x1234, 0x40],
            |m| Out::U32(sgm::mem_realloc(m, 0x1234, 0x40)),
        );
        for (heap, addr) in [(0, 0x0083_f010), (1, 0x0083_f030)] {
            t.reset();
            t.stub(sgm::MEM_REALLOC, false, 4, 0, 0x72 + heap);
            t.check("SgRealloc", addr, Conv::Cdecl, &[0x5678, 0x20], |m| {
                Out::U32(sgm::sg_realloc(m, heap, 0x5678, 0x20))
            });
        }
        t.reset();
        t.stub(sgm::USER_MALLOC, false, 1, 0, 0x73);
        t.check("0x0083f100", 0x0083_f100, Conv::Cdecl, &[0x30], |m| {
            Out::U32(sgm::malloc_thunk(m, 0x30))
        });
        for s in [&b"\0"[..], b"abc\0", b"a longer string\0"] {
            t.reset();
            let out = t.zeros(0x40);
            t.stub(sgm::MALLOC_THUNK, false, 1, 0, out);
            let src = t.alloc(s);
            t.check("SgStrdupD", 0x0083_f050, Conv::Cdecl, &[src], |m| {
                Out::U32(sgm::strdup(m, src))
            });
        }
        for size in [0, 5, 8, 0xffff_fffb, 0x100] {
            for give in [false, true] {
                t.reset();
                let buf = t.zeros(0x10);
                t.stub(sgm::SCE_OPEN, false, 2, 0, 9);
                t.stub_ex(sgm::SCE_LSEEK, false, 3, 0, &Ret::Seq(vec![size, 0]), &[]);
                t.stubs(&[
                    (sgm::USER_MALLOC, 1, buf),
                    (sgm::FLUSH_CACHE, 1, 0),
                    (sgm::SCE_READ, 3, size),
                    (sgm::SCE_CLOSE, 1, 0),
                ]);
                let out = if give { t.words(&[0x5555]) } else { 0 };
                let name = t.alloc(b"f\0");
                let what = format!("read whole file size {size:#x} out {give}");
                t.check(&what, 0x0084_f1c0, Conv::Cdecl, &[name, out], |m| {
                    Out::U32(sgm::read_whole_file(m, name, out))
                });
            }
        }
    });
}

#[test]
fn file_lists_match_the_original() {
    Twins::each(|t| {
        let ext_at = t.rebase(sgm::STR_EXTENSION);
        let mut ext = Vec::new();
        loop {
            let c: u8 = Mem::read(&t.a, ext_at + ext.len() as u32);
            if c == 0 {
                break;
            }
            ext.push(c);
        }
        let with = |name: &str| {
            let mut v = name.as_bytes().to_vec();
            v.extend(&ext);
            v
        };
        let mut texts: Vec<Vec<u8>> = Vec::new();
        let mut a = with("a");
        a.extend(b"\r\nb.txt\n");
        a.extend(with("c.d"));
        a.extend(b"\n\nlast");
        texts.push(a);
        texts.push(b"x\n".to_vec());
        texts.push(with("only"));
        texts.push(b"\r\r\n".to_vec());
        for text in &texts {
            for filter in [0, 1, 2] {
                t.reset();
                let mut buf = text.clone();
                buf.extend([0, 0, 0, 0]);
                let p = t.alloc(&buf);
                let out = t.zeros(0x40);
                let dups: Vec<u32> = (0..8).map(|k| 0x0d00 + k).collect();
                t.stub_ex(sgm::STRDUP, false, 1, 0, &Ret::Seq(dups), &[]);
                let f = t.code(&STRCHR_CODE);
                let slot = t.rebase(sgm::STRCHR_IAT);
                t.patch(slot, &f.to_le_bytes());
                let what = format!(
                    "split lines {:?} filter {filter}",
                    String::from_utf8_lossy(text)
                );
                t.check_framed(
                    &what,
                    0x0085_0590,
                    Conv::Cdecl,
                    &[p, out, filter],
                    sgm::SPLIT_FRAME,
                    |m, ebp| Out::U32(sgm::split_lines(m, ebp, p, out, filter)),
                );
            }
        }
        let copy = t.code(&COPY_CODE);
        for (data, size) in [(true, 6), (true, 0x1_0000), (false, 0), (true, 0)] {
            for old in [0, 3] {
                t.reset();
                let text = t.alloc(&[0x41; 0x40]);
                let arr = t.words(&[0x5a5a_5a5a; 0x10]);
                t.stub_ex(
                    sgm::USER_MALLOC,
                    false,
                    1,
                    0,
                    &Ret::Seq(vec![text, arr]),
                    &[],
                );
                t.stubs(&[
                    (sgm::DBG_PRINTF, 2, 0),
                    (sgm::SCE_OPEN, 2, 0x44),
                    (sgm::USER_FREE, 1, 0x45),
                    (sgm::SPLIT_LINES, 3, 7),
                ]);
                t.stub_import(sgm::SPRINTF_IAT, 4, 0, 1);
                t.jump(sgm::MEMCPY, copy);
                let file = t.alloc(b"one\ntwo\n\nthree\0");
                let file = if data { file } else { 0 };
                // mov eax, [esp + 8]; mov dword [eax], size
                let mut extra = vec![0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00];
                extra.extend(u32::to_le_bytes(size));
                t.stub_ex(sgm::READ_WHOLE_FILE, false, 2, 0, &Ret::Fixed(file), &extra);
                let strings = t.words(&[0x0e00, 0, 0x0e02]);
                let list = if old == 0 {
                    t.words(&[9, 0, 0])
                } else {
                    t.words(&[9, old, strings])
                };
                let path = t.alloc(b"dir\0");
                let what = format!("sg_make_filelist data {data} size {size:#x} old {old}");
                t.check_framed(
                    &what,
                    0x0085_09e0,
                    Conv::Cdecl,
                    &[list, path, 1],
                    sgm::FILELIST_FRAME,
                    |m, ebp| Out::U32(sgm::make_filelist(m, ebp, list, path, 1)),
                );
            }
        }
    });
}

#[test]
fn sqs_command_and_resource_toggles_match_the_original() {
    Twins::each(|t| {
        // below the limit, ending at it exactly, past it
        for full in [0, 0x20, 0x10] {
            for debug in [0u8, 1, 2] {
                t.reset();
                let q = t.zeros(0x100);
                let at = if full != 0 {
                    t.rebase(sgm::SQS_CMD_LIMIT) - full
                } else {
                    q
                };
                t.g32(sgm::SQS_CMD_NEXT, at);
                t.g8(sgm::SQS_DEBUG_FLAGS, debug);
                t.stub(sgm::DBG_PRINTF, false, 1, 0, 0);
                let extra = set_word(t.rebase(sgm::SQS_CMD_NEXT), q + 0x40);
                t.stub_ex(sgm::SQS_SEND, false, 0, 0, &Ret::Fixed(0x66), &extra);
                let what = format!("sqs 0x3b full {full} debug {debug}");
                t.check(&what, 0x0081_fc30, Conv::Cdecl, &[0x1234, 0x1ff], |m| {
                    Out::U32(sgm::sqs_command_3b(m, 0x1234, 0x1ff))
                });
            }
        }
        for on in [0, 1, 5] {
            t.reset();
            t.stub(sgm::SET_RESOURCE, false, 2, 0, 0x31);
            t.check("set resource 9", 0x0083_ff00, Conv::Cdecl, &[on], |m| {
                Out::U32(sgm::set_resource9(m, on))
            });
        }
        for b in [0u8, 1, 2, 0xff] {
            t.reset();
            t.g8(sgm::RESOURCE9_TOGGLE, b);
            t.stub(sgm::SET_RESOURCE9, false, 1, 0, 0x32);
            let what = format!("toggle resource 9 from {b}");
            t.check(&what, 0x0082_1e40, Conv::Cdecl, &[], |m| {
                Out::U32(sgm::toggle_resource9(m))
            });
        }
    });
}

#[test]
fn small_getters_and_setters_match_the_original() {
    Twins::each(|t| {
        for v in [0u32, 0x1234_8765, 0xffff_ffff] {
            t.reset();
            t.g32(sgm::WORD_012FBC3C, v);
            t.g32(sgm::HALF_0130074C, v);
            t.g8(sgm::CAMERA_FLAGS, v as u8);
            t.check("0x00821740", 0x0082_1740, Conv::Cdecl, &[], |m| {
                Out::U32(sgm::get_012fbc3c(m))
            });
            t.check("0x0082cc10", 0x0082_cc10, Conv::Cdecl, &[], |m| {
                Out::U32(sgm::get_half_0130074c(m))
            });
            t.check("camera follow", 0x0082_d850, Conv::Cdecl, &[], |m| {
                Out::U32(sgm::get_camera_follow(m))
            });
            t.check("0x0082cc00", 0x0082_cc00, Conv::Cdecl, &[], |m| {
                Out::U32(sgm::clear_half_0130074c(m))
            });
            t.g32(sgm::CAPTURE_LEFT, v);
            t.check("0x0083a570", 0x0083_a570, Conv::Cdecl, &[], |m| {
                sgm::capture_clear(m);
                Out::None
            });
            t.check("Sg_CaptureSetFilter", 0x0083_a650, Conv::Cdecl, &[v], |m| {
                sgm::capture_set(m, sgm::CAPTURE_BLEND, v);
                Out::None
            });
            t.check("Sg_CaptureSetMode", 0x0083_a670, Conv::Cdecl, &[v], |m| {
                sgm::capture_set(m, sgm::CAPTURE_INTERLACED, v);
                Out::None
            });
        }
        for yaw in [0.5f32, -3.0, 1e-3] {
            t.reset();
            let cell = t.words(&[yaw.to_bits()]);
            t.stub_ex(
                sgm::MS_CAMERA_GET_YAW,
                false,
                0,
                0,
                &Ret::Fixed(0),
                &fld(cell),
            );
            t.g32(sgm::YAW_ZERO, 0x1234);
            t.check("0x0082d930", 0x0082_d930, Conv::Cdecl, &[], |m| {
                sgm::store_camera_yaw(m);
                Out::None
            });
            let addr = t.rebase(0x0083_f2d0);
            let ra = t.a.call(addr, Conv::Cdecl, &[]).unwrap();
            let r = sgm::camera_get_yaw(&mut G(&mut t.b));
            assert!(ra.st0_is(r), "Sg_CameraGetYaw {yaw}: {:?} != {r}", ra.st0);
            t.diff("Sg_CameraGetYaw");
        }
        t.reset();
        t.stub(sgm::PLIGHT_DISPOSE, false, 0, 0, 0x88);
        t.check("PLightDispose thunk", 0x0083_1a30, Conv::Cdecl, &[], |m| {
            Out::U32(sgm::plight_dispose_thunk(m))
        });
        for monster in [0x114, 0x158, 0x82, 0xb0, 0x83, 0, 0x115] {
            t.reset();
            t.stub(sgm::UNLOCK_ACHIEVEMENT, false, 1, 0, 0x99);
            let what = format!("trophyCheckDefeatMonster {monster:#x}");
            t.check(&what, 0x0082_2760, Conv::Cdecl, &[monster], |m| {
                sgm::trophy_check_defeat_monster(m, monster);
                Out::None
            });
        }
    });
}

#[test]
fn mem_init_and_setup_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_c260);
        for i in 0..24 {
            t.reset();
            let id = rng.below(8);
            let area = t.words(&(0..0x80).map(|_| rng.next()).collect::<Vec<_>>());
            let start = area + rng.below(8);
            let end = area + 0x100 + rng.below(0x100);
            t.stub(sgm::DBG_PRINTF, false, 3, 0, 0);
            let what = format!("Sg_MemInit #{i} id {id}");
            t.check(&what, 0x0083_c260, Conv::Cdecl, &[id, start, end], |m| {
                Out::U32(sgm::mem_init(m, id, start, end))
            });
        }
        t.reset();
        let p = t.zeros(0x40);
        t.stubs(&[
            (sgm::SET_CHECK, 1, 0),
            (sgm::SET_FAST, 1, 0),
            (sgm::USER_ALIGNED_MALLOC, 2, p),
            (sgm::MEM_INIT, 3, 0),
            (sgm::SET_MOVE_START, 1, 0x5f),
        ]);
        t.check("Sg_MemSetup", 0x0083_c200, Conv::Cdecl, &[], |m| {
            Out::U32(sgm::mem_setup(m))
        });
    });
}

/// A heap of `states.len()` blocks laid out back to back from `base`
/// with `sizes` data bytes each, then the end block (state 0xf); the
/// first block's previous is the end block. Returns the headers.
fn build_heap(t: &mut Twins, rng: &mut Rng, states: &[u32], sizes: &[u32]) -> Vec<u32> {
    let total: u32 = sizes.iter().map(|s| s + 0x10).sum::<u32>() + 0x10;
    let fill: Vec<u32> = (0..total / 4).map(|_| rng.next()).collect();
    let base = t.words(&fill);
    let mut at = Vec::new();
    let mut p = base;
    for s in sizes {
        at.push(p);
        p += s + 0x10;
    }
    at.push(p);
    let n = at.len();
    for k in 0..n {
        let next = if k + 1 < n { at[k + 1] } else { 0 };
        let prev = if k == 0 { at[n - 1] } else { at[k - 1] };
        let state = if k + 1 < n { states[k] } else { 0xf };
        t.w32(at[k], next);
        t.w32(at[k] + 4, prev);
        t.w32(at[k] + 8, 0);
        t.w32(at[k] + 0xc, state);
    }
    at
}

#[test]
fn heap_blocks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_c5b0);
        for i in 0..60 {
            t.reset();
            let n = 3 + rng.below(5) as usize;
            let mut states: Vec<u32> = (0..n).map(|_| u32::from(!rng.chance(2))).collect();
            states[0] = 1;
            let sizes: Vec<u32> = (0..n).map(|_| 0x10 * (1 + rng.below(4))).collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            let b = at[rng.below(n as u32) as usize];
            let what = format!("merge free #{i}");
            t.check(&what, 0x0083_c5b0, Conv::Cdecl, &[b], |m| {
                Out::U32(sgm::merge_free(m, b))
            });
        }
        for i in 0..30 {
            t.reset();
            t.stub(sgm::SET_FREE, false, 1, 0, 0x11);
            t.stub(sgm::MERGE_FREE, false, 1, 0, 0x12);
            let blocks = t.words(&(0..0x30).map(|_| rng.next()).collect::<Vec<_>>());
            let heap = rng.below(4);
            let (block, prev, next) = (blocks + 0x40, blocks, blocks + 0x80);
            let what = format!("Sg_MemCreateFree #{i}");
            t.check(
                &what,
                0x0083_baf0,
                Conv::Cdecl,
                &[heap, block, prev, next],
                |m| Out::U32(sgm::create_free(m, heap, block, prev, next)),
            );
        }
        let copy = t.code(&COPY_CODE);
        for i in 0..80 {
            t.reset();
            let slot = t.rebase(sgm::MEMMOVE_IAT);
            t.patch(slot, &copy.to_le_bytes());
            let n = 3 + rng.below(6) as usize;
            let mut states: Vec<u32> = (0..n).map(|_| u32::from(!rng.chance(2))).collect();
            states[0] = 1;
            let target = 1 + rng.below(n as u32 - 1) as usize;
            states[target] = 1;
            let sizes: Vec<u32> = (0..n).map(|_| 0x10 * (1 + rng.below(4))).collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            let start = at[rng.below(target as u32) as usize];
            t.g32(sgm::MOVE_START, if i % 9 == 0 { at[n] } else { start });
            let data = if i % 13 == 0 { 0 } else { at[target] + 0x10 };
            let pp = t.words(&[data]);
            let moved = if i % 3 == 0 {
                0
            } else {
                t.words(&[rng.below(0x100)])
            };
            let what =
                format!("Sg_MemMoveWork #{i} target {target} states {states:?} sizes {sizes:x?}");
            t.check(&what, 0x0083_c350, Conv::Cdecl, &[pp, moved], |m| {
                Out::U32(sgm::move_work(m, pp, moved))
            });
        }
    });
}

// ---- round 2: the Sg_Mem debug side, the host files, small wrappers ----

use ffx_sg::m5main_mem2::bridge as sgm2;

/// Recording stubs for a varargs function, picked by the word at `[esp +
/// key]` (a format): `fmts` gives the words each logs (format included),
/// `other` the rest's. Returns the dispatcher's address.
fn keyed_code(t: &mut Twins, tag: u32, key: u8, fmts: &[(u32, u32)], other: u32) -> u32 {
    let mut c = vec![0x8b, 0x54, 0x24, key]; // mov edx, [esp + key]
    for &(fmt, n) in fmts {
        let to = t.stub_code(tag, false, n, 0, &Ret::Fixed(0x21), &[]);
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(fmt).to_le_bytes()); // cmp edx, fmt
        c.extend([0x75, 7]); // jne next
        c.push(0xb8);
        c.extend(to.to_le_bytes()); // mov eax, to
        c.extend([0xff, 0xe0]); // jmp eax
    }
    let rest = t.stub_code(tag, false, other, 0, &Ret::Fixed(0x21), &[]);
    c.push(0xb8);
    c.extend(rest.to_le_bytes());
    c.extend([0xff, 0xe0]);
    t.code(&c)
}

/// `dbgPrintf` bound to [`keyed_code`] on its format.
fn dbg_stub(t: &mut Twins, fmts: &[(u32, u32)]) {
    let at = keyed_code(t, sgm2::DBG_PRINTF, 4, fmts, 1);
    t.jump(sgm2::DBG_PRINTF, at);
}

#[test]
fn host_files_match_the_original() {
    Twins::each(|t| {
        for on in [0u8, 1] {
            for fd in [0, 7] {
                t.reset();
                varargs_stub(t, sgm2::SPRINTF_IAT, &[(sgm2::FMT_CONCAT, 4)], 2);
                t.stubs(&[
                    (sgm2::SCE_OPEN, 2, fd),
                    (sgm2::YI_ASSERT, 4, 0x41),
                    (sgm2::SCE_WRITE, 3, 0x42),
                    (sgm2::SCE_CLOSE, 1, 0x43),
                ]);
                t.g8(sgm2::HOST_WRITE_ON, on);
                let name = t.alloc(b"out.bin\0");
                let what = format!("Sg_PcWrite on {on} fd {fd}");
                t.check_framed(
                    &what,
                    0x0083_afe0,
                    Conv::Cdecl,
                    &[0x1000, 0x20, name],
                    sgm2::PC_WRITE_FRAME,
                    |m, ebp| {
                        let r = sgm2::pc_write(m, ebp, 0x1000, 0x20, name);
                        if on == 0 { Out::None } else { Out::U32(r) }
                    },
                );
            }
        }
        for mode in [0, 1, 2, 3] {
            for fd in [0, 5] {
                for seen in [0, 1] {
                    t.reset();
                    dbg_stub(t, &[(sgm2::MSG_PC_READ, 3), (sgm2::MSG_PC_READ_SIZE, 3)]);
                    t.stub_import(sgm2::SPRINTF_IAT, 3, 0, 1);
                    t.stubs(&[
                        (sgm2::SCE_OPEN, 2, fd),
                        (sgm2::YI_ASSERT, 4, 0x41),
                        (sgm2::USER_MALLOC, 1, 0x0070_0000),
                        (sgm2::SCE_READ, 3, 0x30),
                        (sgm2::SCE_CLOSE, 1, 0),
                    ]);
                    t.stub_ex(sgm2::SCE_LSEEK, false, 3, 0, &Ret::Seq(vec![0x30, 0]), &[]);
                    t.g32(sgm2::PC_READ_SEEN, seen);
                    let name = t.alloc(b"data/x\0");
                    let what = format!("Sg_PcRead mode {mode} fd {fd} seen {seen}");
                    t.check_framed(
                        &what,
                        0x0083_b1d0,
                        Conv::Cdecl,
                        &[name, mode],
                        sgm2::PC_READ_FRAME,
                        |m, ebp| Out::U32(sgm2::pc_read(m, ebp, name, mode)),
                    );
                }
            }
        }
    });
}

#[test]
fn block_checks_and_dumps_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_b960);
        let mcb: Vec<(u32, u32)> = sgm2::MSG_MCB
            .iter()
            .zip([3, 4, 3, 3, 6, 3])
            .map(|(&f, n)| (f, n))
            .collect();
        for i in 0..80 {
            t.reset();
            dbg_stub(t, &mcb);
            let area = t.zeros(0x200);
            let heap = rng.below(4);
            let rec = t.rebase(sgm2::HEAPS) + 0x18 * heap;
            let (first, end) = (area + 0x40, area + 0x180);
            t.w32(rec + 0xc, first);
            t.w32(rec + 0x10, end);
            let spots = [first - 0x10, first, area + 0x100, end, end + 0x10, 0];
            let b = if i % 4 == 0 {
                rng.pick(&spots)
            } else {
                area + 0x100
            };
            let b = if b == 0 { area + 0x80 } else { b };
            let state = rng.pick(&[0, 1, 0xf]);
            t.w32(b, rng.pick(&spots));
            t.w32(b + 4, rng.pick(&spots));
            t.w32(b + 0xc, (heap << 27) | state | (rng.next() & 0x0400_0ff0));
            let what = format!("Sg_MemCheckMCB #{i}");
            t.check(&what, 0x0083_b960, Conv::Cdecl, &[b], |m| {
                Out::U32(sgm2::check_mcb(m, b))
            });
        }
        for state in [0, 1, 5, 0xf] {
            for checked in [0, 0x0400_0000] {
                for name in [0, 0x0bb0] {
                    t.reset();
                    dbg_stub(
                        t,
                        &[
                            (sgm2::FMT_DUMP_HEAD, 3),
                            (sgm2::FMT_DUMP_SIZE, 3),
                            (sgm2::FMT_DUMP_FUNC, 3),
                            (sgm2::FMT_DUMP_NAME, 2),
                        ],
                    );
                    t.stubs(&[
                        (sgm2::FUNC_BASE, 0, 0x0040_0000),
                        (sgm2::GET_FUNC_NAME, 1, 0x0bf0),
                        (sgm2::MEM_GET_STR, 1, name),
                    ]);
                    let b = t.zeros(0x40);
                    t.w32(b, b + 0x30);
                    t.w32(b + 0xc, state | checked | (0x0312_3450 & 0x03ff_fff0));
                    let what =
                        format!("Sg_MemDumpMCB state {state} checked {checked:#x} name {name:#x}");
                    t.check(&what, 0x0083_bbc0, Conv::Cdecl, &[b, 7], |m| {
                        Out::U32(sgm2::dump_mcb(m, b, 7))
                    });
                }
            }
        }
    });
}

#[test]
#[allow(clippy::too_many_lines)] // one test per group of functions
fn heap_dumps_and_checks_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_c630);
        for i in 0..20 {
            t.reset();
            dbg_stub(t, &[]);
            t.stubs(&[(sgm2::RELINK_BACK, 1, 0), (sgm2::DUMP_MCB, 2, 0)]);
            let n = 2 + rng.below(5) as usize;
            let states: Vec<u32> = (0..n).map(|_| rng.pick(&[0, 0, 1])).collect();
            let sizes: Vec<u32> = (0..n)
                .map(|_| rng.pick(&[0x10, 0x30, 0x40, 0x80]))
                .collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            // the end block links nowhere (0 ends the walk)
            let rec = t.zeros(0x18);
            t.w8(rec, (i % 2) as u8);
            t.w32(rec + 0xc, at[0]);
            let what = format!("Sg_MemDump_0083c630 #{i}");
            let slot = t.frame_ebp(1) + 8;
            t.check_framed(&what, 0x0083_c630, Conv::Cdecl, &[rec], 4, |m, ebp| {
                Out::U32(sgm2::heap_dump(m, rec, ebp + 8))
            });
            let a: u32 = Mem::read(&t.a, slot);
            let b: u32 = Mem::read(&t.b, slot);
            assert_eq!(a, b, "{what}: the argument slot");
        }
        for i in 0..30 {
            t.reset();
            dbg_stub(
                t,
                &[
                    (sgm2::FMT_SUM_AT, 3),
                    (sgm2::FMT_SUM_AT2, 3),
                    (sgm2::FMT_LINK_AT, 3),
                    (sgm2::FMT_LINK_CODE, 2),
                ],
            );
            let n = 2 + rng.below(4) as usize;
            let states: Vec<u32> = (0..n)
                .map(|_| 1 | (u32::from(rng.chance(2)) << 26))
                .collect();
            let sizes: Vec<u32> = (0..n).map(|_| 0x20).collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            for &b in &at {
                t.write(b + 8, &(rng.below(3) as u16).to_le_bytes());
            }
            let sums: Vec<u32> = (0..=n as u32).map(|_| 0xabcd_0000 | rng.below(3)).collect();
            t.stub_ex(sgm2::MEM_GET_SUM, false, 1, 0, &Ret::Seq(sums), &[]);
            let codes: Vec<u32> = (0..=n as u32)
                .map(|_| if rng.chance(3) { 0xffff_fffd } else { 0 })
                .collect();
            t.stub_ex(sgm2::CHECK_MCB, false, 1, 0, &Ret::Seq(codes), &[]);
            t.stubs(&[
                (sgm2::HISTORY_DUMP, 0, 0),
                (sgm2::HEAP_DUMP, 1, 0),
                (sgm2::DUMP_MCB, 2, 0),
                (sgm2::YI_ASSERT, 3, 0),
            ]);
            let rec = t.zeros(0x18);
            t.w8(rec + 1, u8::from(i % 5 != 0));
            t.w32(rec + 0xc, if i % 7 == 0 { 0 } else { at[0] });
            let what = format!("Sg_MemCheckHeap #{i}");
            t.check(
                &what,
                0x0083_b7d0,
                Conv::Cdecl,
                &[rec, 0x00b0_1234, 99],
                |m| {
                    sgm2::check_heap(m, rec, 0x00b0_1234, 99);
                    Out::None
                },
            );
        }
        for id in [0, 1, 3, 9] {
            t.reset();
            t.stub(sgm2::CHECK_HEAP, false, 3, 0, 0x51);
            let slot = t.frame_ebp(3) + 8;
            let what = format!("Sg_MemCheck {id}");
            t.check_framed(
                &what,
                0x0083_b7b0,
                Conv::Cdecl,
                &[id, 0x00b0_0000, 7],
                4,
                |m, ebp| Out::U32(sgm2::mem_check(m, id, 0x00b0_0000, 7, ebp + 8)),
            );
            let a: u32 = Mem::read(&t.a, slot);
            let b: u32 = Mem::read(&t.b, slot);
            assert_eq!(a, b, "{what}: the argument slot");
        }
        for p in [0, 0x1230] {
            t.reset();
            t.stub(sgm2::USER_MEM_SIZE, false, 1, 0, 0x52);
            t.check("Sg_MemGetSize", 0x0083_c050, Conv::Cdecl, &[p], |m| {
                Out::U32(sgm2::mem_get_size(m, p))
            });
        }
        for heap in [0, 1, 2, 3] {
            t.reset();
            t.stubs(&[(sgm2::HEAP_DUMP, 1, 0), (sgm2::CACHE_DUMP, 1, 0x53)]);
            let slot = t.frame_ebp(1) + 8;
            let what = format!("Sg_MemDump {heap}");
            t.check_framed(&what, 0x0083_bb50, Conv::Cdecl, &[heap], 4, |m, ebp| {
                Out::U32(sgm2::mem_dump(m, heap, ebp + 8))
            });
            let a: u32 = Mem::read(&t.a, slot);
            let b: u32 = Mem::read(&t.b, slot);
            assert_eq!(a, b, "{what}: the argument slot");
        }
        t.reset();
        t.stub(sgm2::RELINK_BACK, false, 1, 0, 0x54);
        t.check("make_yi_font16", 0x0083_c5a0, Conv::Cdecl, &[], |m| {
            Out::U32(sgm2::relink_heap0(m))
        });
    });
}

#[test]
#[allow(clippy::too_many_lines)] // one test per group of functions
fn resource_dump_and_heap_queries_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0083_ff20);
        for i in 0..24 {
            t.reset();
            let mut fmts = vec![(sgm2::FMT_RES_MEM, 5), (sgm2::FMT_RES_INFO, 5)];
            fmts.extend(sgm2::RES_KIND_NAMES.iter().map(|&k| (k, 1)));
            dbg_stub(t, &fmts);
            t.stubs(&[(sgm2::MEM_GET_SIZE, 1, 0x61), (sgm2::CREATE_NAME, 1, 0x62)]);
            let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            let res = t.words(&w);
            t.w32(res + 0xc, if i % 6 == 0 { 0 } else { 0x0077_0000 });
            t.w8(res + 4, rng.pick(&[0u8, 1, 2, 3, 4, 5, 0xff]));
            let slot = t.frame_ebp(1) + 8;
            let what = format!("resource dump #{i}");
            t.check_framed(&what, 0x0083_ff20, Conv::Cdecl, &[res], 4, |m, ebp| {
                let r = sgm2::resource_dump(m, res, ebp + 8);
                if i % 6 == 0 { Out::None } else { Out::U32(r) }
            });
            // without memory the slot keeps the caller's word
            if i % 6 != 0 {
                let a: u32 = Mem::read(&t.a, slot);
                let b: u32 = Mem::read(&t.b, slot);
                assert_eq!(a, b, "{what}: the argument slot");
            }
        }
        for i in 0..16 {
            t.reset();
            let n = 2 + rng.below(5) as usize;
            let states: Vec<u32> = (0..n).map(|_| rng.pick(&[0, 1, 8])).collect();
            let sizes: Vec<u32> = (0..n).map(|_| 0x10 * (1 + rng.below(6))).collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            // the walk ends at a 0 link
            t.w32(at[n], 0);
            t.g32(sgm2::HEAPS + 0xc, at[0]);
            t.stub(sgm2::RELINK_HEAP0, false, 0, 0, 0);
            t.stub_ex(
                sgm2::MEM_MOVE_OVER,
                false,
                0,
                0,
                &Ret::Seq(vec![1, 1, 0]),
                &[],
            );
            t.stub_ex(
                sgm2::CACHE_OVERFLOW,
                false,
                0,
                0,
                &Ret::Seq(vec![0, 0, 1]),
                &[],
            );
            let mode = i % 4;
            let what = format!("Sg_MemGet #{i} mode {mode}");
            t.check(&what, 0x0083_bff0, Conv::Cdecl, &[mode], |m| {
                Out::U32(sgm2::mem_get(m, mode))
            });
        }
        for i in 0..30 {
            t.reset();
            let n = 3 + rng.below(8) as usize;
            let states: Vec<u32> = (0..n).map(|_| rng.below(16)).collect();
            let sizes: Vec<u32> = (0..n).map(|_| 0x10 * (1 + rng.below(4))).collect();
            let at = build_heap(t, &mut rng, &states, &sizes);
            t.w32(at[n], 0);
            for &b in &at {
                t.write(b + 0xa, &((rng.below(8) as u16) << 11).to_le_bytes());
            }
            t.g32(sgm2::HEAPS + 0xc, at[0]);
            t.g32(sgm2::HEAPS + 8, 0x1100_0000);
            t.stub(sgm2::MEM_CHECK, false, 3, 0, 0);
            let out = t.words(&[0x0101_0101; 9]);
            let what = format!("Sg_MemUsage #{i} states {states:?}");
            t.check(&what, 0x0083_c0f0, Conv::Cdecl, &[out], |m| {
                Out::U32(sgm2::mem_usage(m, out))
            });
        }
        for p in [0, 1] {
            t.reset();
            let extra = set_word(t.rebase(sgm2::RT_ADDR), 0x0012_3456);
            t.stub_ex(sgm2::SET_RT_ADDR, false, 1, 0, &Ret::Fixed(0), &extra);
            t.g32(sgm2::RT_ADDR_SOURCE, 0x0050_0000);
            let b = t.words(&[0, 0, 0, 0xffff_ffff, 0]);
            let p = if p == 0 { 0 } else { b + 0x10 };
            t.check(
                "Sg_MemSetCurrentAddr",
                0x0083_c4e0,
                Conv::Cdecl,
                &[p],
                |m| {
                    sgm2::set_current_addr(m, p);
                    Out::None
                },
            );
        }
        t.reset();
        t.stubs(&[
            (sgm2::ALIGNED_FREE, 1, 0x71),
            (sgm2::ALIGNED_MALLOC, 2, 0x72),
            (sgm2::CACHE_CLEAR, 0, 0x73),
            (sgm2::CACHE_CLEAR_FAST, 0, 0x74),
        ]);
        t.check("Sg_FreeA", 0x0083_f090, Conv::Cdecl, &[0x40], |m| {
            Out::U32(sgm2::free_a(m, 0x40))
        });
        t.check("Sg_MallocA", 0x0083_f0a0, Conv::Cdecl, &[0x40, 0x10], |m| {
            Out::U32(sgm2::malloc_a(m, 0x40, 0x10))
        });
        t.check("cache clear thunk", 0x0084_0e40, Conv::Cdecl, &[], |m| {
            Out::U32(sgm2::cache_thunk(m, sgm2::CACHE_CLEAR))
        });
        t.check(
            "cache clear fast thunk",
            0x0084_0e50,
            Conv::Cdecl,
            &[],
            |m| Out::U32(sgm2::cache_thunk(m, sgm2::CACHE_CLEAR_FAST)),
        );
    });
}

#[test]
fn db_tables_and_clock_match_the_original() {
    Twins::each(|t| {
        // cosf/sinf left original (they are ported: their own tests)
        t.reset();
        t.check("Sg_DbInitTables", 0x0084_1100, Conv::Cdecl, &[], |m| {
            sgm2::db_init_tables(m);
            Out::None
        });
        for on in [0u8, 1] {
            t.reset();
            t.stubs(&[
                (sgm2::USER_MALLOC, 1, 0x0066_0000),
                (sgm2::DB_INIT_TABLES, 0, 0),
            ]);
            t.g8(sgm2::HOST_WRITE_ON, on);
            t.check("Sg_DbInitAll", 0x0084_1440, Conv::Cdecl, &[], |m| {
                sgm2::db_init_all(m);
                Out::None
            });
        }
        for (a, b) in [(3, 0), (4, 0), (0, 1), (0xffff_ffff, 0)] {
            t.reset();
            dbg_stub(t, &[(sgm2::FMT_DB_COUNTS, 3)]);
            t.g32(sgm2::DB_COUNT_A, a);
            t.g32(sgm2::DB_COUNT_B, b);
            t.check("Sg_DbReport", 0x0084_3560, Conv::Cdecl, &[], |m| {
                sgm2::db_report(m);
                Out::None
            });
        }
        t.reset();
        t.stub(sgm2::ID_FIXUP_AND_SET, false, 1, 0, 0x81);
        t.stub(sgm2::IVEC4_TO_FVEC4, false, 2, 0, 0x82);
        t.check("id fixup thunk", 0x0084_5000, Conv::Cdecl, &[0x40], |m| {
            Out::U32(sgm2::id_fixup_and_set_thunk(m, 0x40))
        });
        t.check(
            "ivec4 thunk",
            0x0084_5de0,
            Conv::Cdecl,
            &[0x40, 0x50],
            |m| Out::U32(sgm2::ivec4_to_fvec4_thunk(m, 0x40, 0x50)),
        );
        for s in [0, 5, 0xffff_ffff] {
            t.reset();
            t.stub(sgm2::DECI2_OPEN, false, 3, 0, s);
            t.check("deci2 open", 0x0084_eee0, Conv::Cdecl, &[], |m| {
                Out::U32(sgm2::deci2_open(m))
            });
        }
        for r in [0xffff_fff0, 0, 12] {
            t.reset();
            t.stub(sgm2::SYSTEM_ON_HOST, false, 2, 0, r);
            t.stub_import(sgm2::SSCANF_IAT, 8, 0, 6);
            let what = format!("clock from host {r:#x}");
            t.check_framed(
                &what,
                0x0084_f080,
                Conv::Cdecl,
                &[],
                sgm2::CLOCK_FRAME,
                |m, ebp| Out::U32(sgm2::clock_from_host(m, ebp)),
            );
        }
        let mut rng = Rng(0x0084_ef10);
        for i in 0..60 {
            t.reset();
            let roll = |rng: &mut Rng, top: u32| {
                if rng.chance(2) {
                    top - 1
                } else {
                    rng.below(top)
                }
            };
            t.g32(sgm2::CLOCK_FRAMES, roll(&mut rng, 0x3c));
            t.g32(sgm2::CLOCK_SEC, roll(&mut rng, 0x3c));
            t.g32(sgm2::CLOCK_MIN, roll(&mut rng, 0x3c));
            t.g32(sgm2::CLOCK_HOUR, roll(&mut rng, 0x18));
            t.g32(sgm2::CLOCK_WEEKDAY, rng.below(7));
            t.g32(sgm2::CLOCK_DAY, rng.below(31));
            t.g32(sgm2::CLOCK_RATE, 7);
            let what = format!("clock tick #{i}");
            t.check(&what, 0x0084_ef10, Conv::Cdecl, &[], |m| {
                Out::U32(sgm2::clock_tick(m))
            });
        }
    });
}

// ---- round 2: the debug GUI's menu helpers, small thunks ----

use ffx_sg::m5main_gui::bridge as gui;

#[test]
fn debug_menu_helpers_match_the_original() {
    Twins::each(|t| {
        for (addr, height, step) in [
            (0x0085_0850, 0xa, 0xb),
            (0x0085_08a0, 9, 0xa),
            (0x0085_08f0, 0xa, 0xa),
        ] {
            t.reset();
            t.stubs(&[
                (gui::ADD_BUTTON, 7, 0x31),
                (gui::ADD_STATIC, 7, 0x32),
                (gui::SET_HIDE, 3, 0x33),
            ]);
            let list = t.words(&[3, 1, 2, 3, 0, 0]);
            t.g32(gui::MENU_LIST, list);
            t.g32(gui::MENU_Y, 0x20);
            let what = format!("menu helper {addr:#x}");
            t.check(
                &what,
                addr,
                Conv::Cdecl,
                &[0x0055_0000, 0x77, 0x00b0_0000],
                |m| {
                    Out::U32(if addr == 0x0085_08f0 {
                        gui::add_static(m, 0x0055_0000, 0x77, 0x00b0_0000)
                    } else {
                        gui::add_button(m, 0x0055_0000, 0x77, 0x00b0_0000, height, step)
                    })
                },
            );
        }
        for (addr, w) in gui::WINDOWS {
            for open in [0u8, 1] {
                t.reset();
                t.stub(gui::WINDOW_CREATE, false, 6, 0, 0x34);
                t.g8(w.open, open);
                let what = format!("window {addr:#x} open {open}");
                t.check(&what, addr, Conv::Cdecl, &[], |m| {
                    gui::open_once(m, &w);
                    Out::None
                });
            }
        }
        for (n, c) in [(0, 0x41), (5, 0x1ff), (0x10, 0)] {
            t.reset();
            t.g32(gui::TEXT_LEN, n);
            t.check("text put", 0x0085_0b50, Conv::Cdecl, &[c], |m| {
                gui::text_put(m, c);
                Out::None
            });
        }
    });
}

#[test]
fn gui_thunks_and_string_helpers_match_the_original() {
    Twins::each(|t| {
        t.reset();
        t.stubs(&[
            (gui::EVENTJUMP_DISPOSE, 0, 0x41),
            (gui::ABM_PARA_GET, 0, 0x42),
            (gui::TARGET_00947CC0, 0, 0x43),
            (gui::USER_FREE, 1, 0),
            (gui::ABMAP_DESTROY, 0, 0),
            (gui::SET_FLIP_VSYNC, 1, 0),
            (gui::LOADING_SCREEN, 0, 0x44),
        ]);
        t.check("eventjump thunk", 0x0087_5940, Conv::Cdecl, &[], |m| {
            Out::U32(gui::thunk(m, gui::EVENTJUMP_DISPOSE))
        });
        t.check("abm para thunk", 0x008a_97e0, Conv::Cdecl, &[], |m| {
            Out::U32(gui::thunk(m, gui::ABM_PARA_GET))
        });
        t.check("0x009436b0", 0x0094_36b0, Conv::Cdecl, &[], |m| {
            Out::U32(gui::thunk(m, gui::TARGET_00947CC0))
        });
        t.g32(gui::ABM_BUF_A, 0x1111);
        t.g32(gui::ABM_BUF_B, 0x2222);
        t.check("eiAbmExit", 0x00a5_4720, Conv::Cdecl, &[], |m| {
            Out::U32(gui::abm_exit(m))
        });
        for s in [&b"\0"[..], b"a\0", b"hello\0"] {
            t.reset();
            let src = t.alloc(s);
            let dst = t.zeros(0x10);
            t.check("strcpy", 0x0094_2030, Conv::Cdecl, &[dst, src], |m| {
                Out::U32(gui::str_copy(m, dst, src))
            });
        }
        for (list, n) in [(0, 3), (1, 0), (2, 0xffff_ffff), (1, 5)] {
            t.reset();
            let entries = t.words(&[0xffff_ffff; 0x20]);
            t.g32(gui::NAME_LIST, list);
            t.g32(gui::NAME_ENTRIES + 4 * list, entries);
            t.g32(gui::NAME_COUNTS + 4 * list, n);
            let what = format!("name list clear {list} {n:#x}");
            t.check(&what, 0x0094_2320, Conv::Cdecl, &[], |m| {
                Out::U32(gui::name_list_clear(m))
            });
        }
    });
}
