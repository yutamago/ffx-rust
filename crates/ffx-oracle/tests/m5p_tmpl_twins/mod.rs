//! The twin oracles of the `m5p_tmpl_*` rest tests (lists tmpl-concrete,
//! tmpl-members-dynamic, tmpl-members-phyrearray-a/-b): two oracles
//! started alike, recording stubs at the callees' addresses, and a runner
//! that calls the original copy on one and the port on the other and
//! compares `eax` and every byte written. Same helpers as
//! `m5p_tmpl_abstract_ab.rs`, plus `run_al` for the `al`-only returns.
#![allow(dead_code, clippy::cast_possible_truncation)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
pub struct G<'a>(&'a mut Oracle);

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

pub fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// Two oracles started alike, and the log of the recording stubs.
pub struct Twins {
    pub a: Oracle,
    pub b: Oracle,
    /// Started from the snapshot (`FFX_SNAPSHOT`), not the image alone.
    pub snapshot: bool,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// stack words).
    pub log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    pub saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    pub fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            snapshot,
            log: 0,
            saved: HashMap::new(),
        };
        t.log = t.zeros(0x400);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    pub fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(
                Oracle::load(&exe).unwrap(),
                Oracle::load(&exe).unwrap(),
                false,
            ),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    true,
                ),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
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

    pub fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    pub fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// The log entries of the last run (the port's side).
    pub fn logged(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// The logged word `i` (0 tag, 1 `ecx`, 2.. stack words) of entry `n`.
    pub fn entry(&self, n: u32, i: u32) -> u32 {
        Mem::read(&self.b, self.log + 4 + 32 * n + 4 * i)
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this` (the thiscall callees; a cdecl callee's `ecx` is whatever
    /// its caller left) and `n` stack words; pops `pops` words; returns
    /// `ret`.
    pub fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
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

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there),
    /// tagged with `sym`.
    pub fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// Puts back every stubbed original.
    pub fn unstub(&mut self) {
        let saved: Vec<_> = self.saved.drain().collect();
        for (at, old) in saved {
            self.write(at, &old);
        }
    }

    pub fn diff(&mut self, what: &str) {
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

    /// Runs the original copy at `sym` as a thiscall on `this` with the
    /// stack words `args` in `a`, and `port` on `b`, from the same memory
    /// (the log emptied first); `eax` and every byte written must agree.
    /// Returns `eax`.
    pub fn run(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u32,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Thiscall(this), args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax, eb, "{what}: eax");
        self.diff(what);
        eb
    }
}

impl Twins {
    /// As [`Twins::run`] for copies that return only `al` (the rest of
    /// `eax` is what the original's last instruction left): compares `al`.
    pub fn run_al(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u8,
    ) -> u8 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Thiscall(this), args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax & 0xff, u32::from(eb), "{what}: al");
        self.diff(what);
        eb
    }

    /// Runs the original at `sym` with `conv` and `args` in `a`, and
    /// `port` on `b`, from the same memory (the log emptied first); every
    /// byte written must agree, and `eax` too when the port gives one.
    /// Returns the original's `eax`.
    pub fn run_conv(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Option<u32>,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        if let Some(eb) = eb {
            assert_eq!(ra.eax, eb, "{what}: eax");
        }
        self.diff(what);
        ra.eax
    }

    /// A NUL-terminated string in both oracles.
    pub fn cstr(&mut self, s: &str) -> u32 {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        self.alloc(&v)
    }
}

/// Odd words for the arguments the copies never read.
pub const JUNK: [u32; 3] = [0x1357_9bdf, 0xfedc_ba98, 0x0000_0001];
