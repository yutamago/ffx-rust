//! The `m5_main3` tests' twin oracles and recording stubs (a copy of
//! `m5_main.rs`'s harness, shared by the round-3 test files).

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::format_push_string,
    clippy::must_use_candidate,
    clippy::missing_panics_doc
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// Deterministic xorshift stream.
pub struct Rng(pub u32);

impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    pub fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// A word: small values, 0, 1, -1 and anything.
    pub fn word(&mut self) -> u32 {
        match self.below(5) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            _ => self.next(),
        }
    }
    pub fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(of.len() as u32) as usize]
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
/// The port's oracle, and the intrinsic trampoline's address
/// ([`Twins::tramp`]).
pub struct G<'a>(pub &'a mut Oracle, pub u32);

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

pub fn conv(c: host::Conv) -> Conv {
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
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // the trampoline: `fld qword [esp+4]; call [esp+0xc]; ret`, then
        // at +16 `fstp st(0); ret`
        let b = x.to_bits();
        let r = self
            .0
            .call(self.1, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap();
        let v = r.st0_f32().expect("the intrinsic's ST0");
        self.0.call(self.1 + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
pub enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
}

/// A stub's result: one value, or the `k`-th of a list on its `k`-th
/// call (the last one after that).
#[derive(Clone)]
pub enum Ret {
    Fixed(u32),
    Seq(Vec<u32>),
}

pub struct Twins {
    pub a: Oracle,
    pub b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    pub log: u32,
    /// Top bytes of a frame [`Twins::check_framed`] leaves out (the
    /// original's security cookie at `ebp - 4`).
    pub skip: u32,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    pub patched: Vec<(u32, Vec<u8>)>,
    /// The intrinsic trampoline (code in both oracles).
    pub tramp: u32,
}

pub fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    pub fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            log: 0,
            skip: 4,
            patched: Vec::new(),
            tramp: 0,
        };
        t.log = t.zeros(0x1_0000);
        let mut c = vec![0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3];
        c.resize(16, 0x90);
        c.extend([0xdd, 0xd8, 0xc3]);
        t.tramp = t.code(&c);
        t
    }

    pub fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        ))
    }

    pub fn snapshot() -> Option<Self> {
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
    pub fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
    }

    pub fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    pub fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    pub fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    pub fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    pub fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    pub fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    pub fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Writes the word at the `ffx.sym` address `sym`.
    pub fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// Writes the byte at the `ffx.sym` address `sym`.
    pub fn g8(&mut self, sym: u32, v: u8) {
        let at = self.rebase(sym);
        self.w8(at, v);
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    pub fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    pub fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        let log = self.log;
        self.w32(log, 0);
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Code in both oracles' code areas; returns its address.
    pub fn code(&mut self, c: &[u8]) -> u32 {
        let x = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), x);
        x
    }

    /// A recording stub: logs `tag`, `ecx` when `this`, and `n` stack
    /// arguments; runs `extra` (code that may use `eax`, `ecx`, `edx`;
    /// `[esp + 4]` is the first argument); pops `pops` words; returns
    /// `ret`. Returns its address.
    pub fn stub_code(
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
    pub fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// [`Twins::stub_code`] written over the original at `sym` (tag
    /// `sym`).
    pub fn stub_ex(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: &Ret, extra: &[u8]) {
        let addr = self.stub_code(sym, this, n, pops, ret, extra);
        self.jump(sym, addr);
    }

    /// A plain recording stub over `sym`.
    pub fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        self.stub_ex(sym, this, n, pops, &Ret::Fixed(ret), &[]);
    }

    /// Plain recording cdecl stubs `(sym, words, ret)`.
    pub fn stubs(&mut self, list: &[(u32, u32, u32)]) {
        for &(sym, n, ret) in list {
            self.stub(sym, false, n, 0, ret);
        }
    }

    /// A recording stub bound to the import table slot `slot`.
    pub fn stub_import(&mut self, slot: u32, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(slot, false, n, pops, &Ret::Fixed(ret), &[]);
        let at = self.rebase(slot);
        self.patch(at, &addr.to_le_bytes());
    }

    /// A fake vtable at `obj`'s first word whose slots are recording
    /// thiscall stubs `(words, ret, extra)`, tagged `0x5100 + slot`.
    pub fn fake_vtable(&mut self, obj: u32, slots: &[(u32, u32, Vec<u8>)]) {
        let mut vt = Vec::new();
        for (i, (n, ret, extra)) in slots.iter().enumerate() {
            vt.push(self.stub_code(0x5100 + i as u32, true, *n, *n, &Ret::Fixed(*ret), extra));
        }
        let vt = self.words(&vt);
        self.patch(obj, &vt.to_le_bytes());
    }

    /// The call logs of both oracles, for a failure message.
    pub fn logs(&self) -> String {
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

    pub fn diff(&mut self, what: &str) {
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
    pub fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    pub fn check(
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
        let out = bridge(&mut G(&mut self.b, self.tramp));
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
    pub fn check_framed(
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
    pub fn calls(&self) -> u32 {
        Mem::read(&self.a, self.log)
    }
}

/// `mov byte [at], v`.
pub fn set_byte(at: u32, v: u8) -> Vec<u8> {
    let mut c = vec![0xc6, 0x05];
    c.extend(at.to_le_bytes());
    c.push(v);
    c
}

/// `mov dword [at], v`.
pub fn set_word(at: u32, v: u32) -> Vec<u8> {
    let mut c = vec![0xc7, 0x05];
    c.extend(at.to_le_bytes());
    c.extend(v.to_le_bytes());
    c
}

/// Writes `w` to the first argument's memory (`mov eax, [esp + 4]` and
/// `mov dword [eax + 4i], w[i]`).
pub fn fill_arg0(w: &[u32]) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04];
    for (i, v) in w.iter().enumerate() {
        c.extend([0xc7, 0x40, (4 * i) as u8]);
        c.extend(v.to_le_bytes());
    }
    c
}

/// A printf-like original at `sym` replaced by recording stubs that log
/// the words each format (the first argument, an `ffx.sym` address) takes;
/// `other` words for any other format. Returns `ret`.
pub fn printf_stub(t: &mut Twins, sym: u32, fmts: &[(u32, u32)], other: u32, ret: u32) {
    let mut c = vec![0x8b, 0x54, 0x24, 0x04]; // mov edx, [esp + 4]
    for &(fmt, n) in fmts {
        let to = t.stub_code(sym, false, n, 0, &Ret::Fixed(ret), &[]);
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(fmt).to_le_bytes()); // cmp edx, fmt
        c.extend([0x75, 7]); // jne next
        c.push(0xb8);
        c.extend(to.to_le_bytes()); // mov eax, to
        c.extend([0xff, 0xe0]); // jmp eax
    }
    let rest = t.stub_code(sym, false, other, 0, &Ret::Fixed(ret), &[]);
    c.push(0xb8);
    c.extend(rest.to_le_bytes()); // mov eax, rest
    c.extend([0xff, 0xe0]); // jmp eax
    let at = t.code(&c);
    t.jump(sym, at);
}
