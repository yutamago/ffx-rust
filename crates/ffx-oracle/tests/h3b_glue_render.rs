//! The render-side glue at 0x00690000 and up (M4 port, list
//! `h3-glue-render`, second agent "h3b") against the original: on the image
//! alone with synthetic objects, and on the snapshots (`FFX_SNAPSHOT`).
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The harness is `h3_glue_render.rs`'s (recording stubs in
//! `graphicFontDraw`'s body, frames at the original's address); the two
//! agents keep separate files.

#![allow(
    dead_code, // the harness is h3_glue_render.rs's; not every helper is used yet
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::glue;
use ffx_prender::glue2;

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison, not what
/// the previous call returned.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;
/// After the poison: `fld qword [esp+4]; call [esp+0xc]; ret`, and at
/// `+16` `fstp st0; ret`, for [`Host::call_intrinsic`] (a CRT intrinsic
/// with its argument in `ST0`; `graphicFontDraw` is 1806 bytes long).
const TRAMPOLINE_AT: u32 = POISON_AT + 8;
///
/// At `+19`, for [`Host::call_intrinsic2`]: `fld qword [esp+4]; fld qword
/// [esp+0xc]; call [esp+0x14]; ret` (`_CIpow`, operands in `ST1` and
/// `ST0`).
const TRAMPOLINE_CODE: [u8; 32] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3, 0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3,
];

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
    /// Bytes, about half of them 0 (flags read as bytes see both).
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| if self.chance(2) { 0 } else { self.next() as u8 })
            .collect()
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
    /// A float that is not a signalling NaN (the emulator keeps those).
    fn float(&mut self) -> f32 {
        loop {
            let bits = match self.below(4) {
                0 => self.next(),
                1 => (self.below(4000) as f32 - 2000.0).to_bits(),
                2 => (self.next() as f32 / 1000.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    1,
                    0x7f7f_ffff,
                ][self.below(7) as usize],
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return f32::from_bits(bits);
            }
        }
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
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
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn call_st0_f64(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f64 {
        ffx_oracle::check_width(
            "Host::call_st0_f64",
            addr,
            args.len(),
            host::max_words(c, true),
        );
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        let r = self.0.call(addr, conv(c), args).unwrap();
        r.st0.unwrap_or_else(|| panic!("{addr:#x}: no ST0"))
    }
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call_wide", addr, args.len(), host::WIDE_WORDS);
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, Conv::Cdecl, args).unwrap().eax
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // the trampoline sits a fixed distance past the poison
        let at = self.1 + (TRAMPOLINE_AT - POISON_AT);
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
    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        let at = self.1 + (TRAMPOLINE_AT - POISON_AT);
        let (base, power) = (x.to_bits(), y.to_bits());
        let words = [
            base as u32,
            (base >> 32) as u32,
            power as u32,
            (power >> 32) as u32,
            addr,
        ];
        let r = self
            .0
            .call(at + 19, Conv::Cdecl, &words)
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let Some(v) = r.st0_f32() else {
            panic!("intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
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
    /// `al` (the rest of `eax` is the caller's or garbage).
    U8(u8),
    /// `edx:eax`.
    U64(u64),
    /// `ST0`, compared bit for bit as `fst qword` stores it.
    F64(f64),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// [`Twins::stub`] logs the word at `ecx` instead of `ecx`.
    deref_this: bool,
    /// ... at `ecx + deref_off`.
    deref_off: u8,
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
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            deref_this: false,
            deref_off: 0,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
        let at = t.rebase(TRAMPOLINE_AT);
        t.write(at, &TRAMPOLINE_CODE);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
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
            true,
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

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    /// Sets the global at `sym`.
    fn global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
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
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// Writes a recording stub: logs `tag`, `ecx` when `this`, and `n`
    /// stack arguments; pops `pops` words; returns `ret` (`None`: leaves
    /// `eax` as it was). Placed at `at` when given (a `jmp` there), else
    /// only in the cave. Returns its address.
    fn stub(
        &mut self,
        at: Option<u32>,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: Option<u32>,
    ) -> u32 {
        assert!(n <= 14);
        let log = self.log;
        // the arguments sit one word higher after `push eax`
        let keep = u8::from(ret.is_none()) * 4;
        let mut c = vec![];
        if keep != 0 {
            c.push(0x50); // push eax
        }
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this && self.deref_this && self.deref_off != 0 {
            c.extend([0x8b, 0x51, self.deref_off, 0x89, 0x50, 0x04]); // mov edx, [ecx+off]; mov [eax+4], edx
        } else if this && self.deref_this {
            c.extend([0x8b, 0x11, 0x89, 0x50, 0x04]); // mov edx, [ecx]; mov [eax+4], edx
        } else if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8 + keep]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            Some(ret) => {
                c.push(0xb8);
                c.extend(ret.to_le_bytes()); // mov eax, ret
            }
            None => c.push(0x58), // pop eax
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        if let Some(at) = at {
            let mut j = vec![0xe9];
            j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
            self.patch(at, &j);
        }
        addr
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        if !d.is_empty() && std::env::var_os("H3_LOG").is_some() {
            // the call logs side by side (tag, this, arguments)
            for (side, o) in [("a", &self.a), ("b", &self.b)] {
                let n: u32 = Mem::read(o, self.log);
                for k in 0..n.min(64) {
                    let e: Vec<u32> = (0..16)
                        .map(|w| Mem::read::<u32>(o, self.log + 4 + 64 * k + 4 * w))
                        .collect();
                    eprintln!("{side} {k}: {e:x?}");
                }
            }
        }
        assert_eq!(d, [], "{what}: memory");
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
        let ra = self.a.call(addr, conv, args).unwrap_or_else(|e| {
            if std::env::var_os("H3_LOG").is_some() {
                let n: u32 = Mem::read(&self.a, self.log);
                for k in 0..n.min(64) {
                    let e: Vec<u32> = (0..6)
                        .map(|w| Mem::read::<u32>(&self.a, self.log + 4 + 64 * k + 4 * w))
                        .collect();
                    eprintln!("a {k}: {e:x?}");
                }
            }
            panic!("{what}: the original: {e}")
        });
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
        if let Out::F64(v) = out {
            assert!(
                ra.st0_is(v),
                "{what}: ST0 {:?}, the port {v:?} ({:#x})",
                ra.st0,
                v.to_bits()
            );
        } else {
            assert!(
                ra.st0_raw.is_none(),
                "{what}: the original leaves ST0 set ({:?}); the port returns {out:?}",
                ra.st0
            );
        }
        match out {
            Out::None | Out::F64(_) => {}
            Out::U64(v) => assert_eq!(
                (u64::from(ra.edx) << 32) | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }
}

// ---- table-driven one-liners ----

/// How a callee is replaced.
#[derive(Clone, Copy)]
enum K {
    /// A method: logs `ecx` and `n` stack words, pops them.
    This(u32),
    /// A method that leaves `eax` alone (a byte store): logs as
    /// [`K::This`].
    Keep(u32),
    /// As [`K::Keep`], logging the word at `ecx` (what the callee finds in
    /// the object's first field) instead of `ecx`.
    KeepDeref(u32),
    /// A method without arguments returning an address inside the
    /// synthetic object (`+0x200`), for originals that write through it.
    Entry,
    /// cdecl: logs `n` stack words, pops none.
    Cdecl(u32),
    /// A singleton getter (cdecl, no arguments): on the image a stub
    /// returning the case's instance (the object, or 0); on a snapshot
    /// left original.
    Getter,
    /// A `stdcall` import with `n` words, through its slot (the stub's
    /// address is written into the slot).
    Import(u32),
    /// A cdecl import with `n` words, through its slot.
    ImportC(u32),
}

/// `this` of a case.
#[derive(Clone, Copy)]
enum This {
    /// A cdecl original: none.
    None,
    /// A thiscall original on the object in this global (on a snapshot,
    /// the game's).
    Global(u32),
    /// A thiscall original on the synthetic object.
    Object,
    /// A stdcall original: no `this`.
    Stdcall,
}

/// A port: the oracle, `this` (0 for cdecl originals) and the stack
/// arguments.
type Port = fn(&mut G, u32, &[u32]) -> Out;

#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // a case's switches
struct Case {
    name: &'static str,
    sym: u32,
    /// How the original takes `this`.
    this: This,
    args: usize,
    /// Float arguments (as bits), which [`Rng::float`] fills.
    float_args: bool,
    stubs: &'static [(u32, K)],
    port: Port,
    /// Extra set-up after the stubs: may change the object, `this` and the
    /// arguments.
    prep: Option<Prep>,
    /// Whether the getters may return 0 (the original only hands the
    /// instance to a method or tests it).
    null_instance: bool,
    /// A port with a frame at the original's address: the bytes below
    /// `ebp` it uses and the bytes of stack arguments (see
    /// [`FRAME_EBP`]).
    frame: Option<(u32, u32)>,
    /// The frame's word at `ebp - 4` is the original's security cookie
    /// (the port leaves it out): not compared.
    cookie: bool,
    /// The frame's top 16 bytes are the original's security cookie and
    /// C++ exception registration (`fs:[0]`), which the port leaves out:
    /// not compared.
    seh: bool,
}

/// A case's extra set-up: the twins, the synthetic object, `this`, the
/// arguments.
type Prep = fn(&mut Twins, u32, &mut u32, &mut [u32], &mut Rng);

const fn cdecl(
    name: &'static str,
    sym: u32,
    args: usize,
    stubs: &'static [(u32, K)],
    port: Port,
) -> Case {
    Case {
        name,
        sym,
        this: This::None,
        args,
        float_args: false,
        stubs,
        port,
        prep: None,
        null_instance: false,
        frame: None,
        cookie: false,
        seh: false,
    }
}

const fn method(
    name: &'static str,
    sym: u32,
    this: This,
    args: usize,
    stubs: &'static [(u32, K)],
    port: Port,
) -> Case {
    Case {
        name,
        sym,
        this,
        args,
        float_args: false,
        stubs,
        port,
        prep: None,
        null_instance: false,
        frame: None,
        cookie: false,
        seh: false,
    }
}

fn u(v: u32) -> Out {
    Out::U32(v)
}

/// The globals the one-liners load their object from (one synthetic
/// object each on the image).
const GLOBALS: [u32; 6] = [
    glue::SCENE.sym,
    glue::DYN_GEO_FILL.sym,
    glue::CHR_SHADER_RENDER.sym,
    glue::MENU_TEXTURE_MANAGER.sym,
    glue2::RENDER_DELEGATION_HELPER.sym,
    glue2::IGGY_MENU_MANAGER.sym,
];

/// Object globals the originals test for 0: an object, or 0 now and then.
const NULLABLE: [u32; 2] = [glue2::GRAPHIC_ABMAP.sym, glue2::GRAPHIC_UI.sym];

/// Plain word globals: random words.
const WORDS: [u32; 3] = [
    glue2::GRAPHIC_SKIP_FRAME.sym,
    glue2::SHADOW_SIZE.sym,
    glue2::MAGIC_FILE_ASYNC_LOAD.sym,
];

const SCENE: u32 = glue::SCENE.sym;

/// Runs every case `n` times on `t` (fresh stubs and objects each time).
#[allow(clippy::too_many_lines)] // the stub kinds
fn run_cases(t: &mut Twins, cases: &[Case], n: u32, rng: &mut Rng) {
    const OBJ_LEN: usize = 0x2000;
    let obj = t.zeros(OBJ_LEN);
    // one object per global, so a port reading the wrong one shows
    let owned: Vec<u32> = GLOBALS.iter().map(|_| t.zeros(0x100)).collect();
    let nullable: Vec<u32> = NULLABLE.iter().map(|_| t.zeros(0x100)).collect();
    let snapshot = t.snapshot;
    for case in cases {
        for i in 0..n {
            t.reset();
            // a fresh synthetic object: random bytes, about half of them 0,
            // with a valid pointer at +0xa8 (a record) and a first word that
            // is 0 every other time
            let bytes = rng.bytes(OBJ_LEN);
            t.write(obj, &bytes);
            t.w32(obj + 0xa8, obj + 0x400);
            if i % 2 == 0 {
                t.w32(obj, 0);
            } else {
                t.w32(obj, obj + 0x800);
            }
            if !snapshot {
                for (&gl, &o) in GLOBALS.iter().zip(&owned) {
                    let b = rng.bytes(0x100);
                    t.write(o, &b);
                    t.global(gl, o);
                }
                for (&gl, &o) in NULLABLE.iter().zip(&nullable) {
                    let b = rng.bytes(0x100);
                    t.write(o, &b);
                    let v = if rng.chance(4) { 0 } else { o };
                    t.global(gl, v);
                }
                for &gl in &WORDS {
                    let v = rng.word();
                    t.global(gl, v);
                }
            }
            // a getter gives the object, or 0 now and then where the case
            // only hands it to a method (fields of a null one would fault)
            let instance = if i % 4 == 3 && case.null_instance {
                0
            } else {
                obj
            };
            for &(sym, kind) in case.stubs {
                let at = t.rebase(sym);
                let ret = if rng.chance(2) {
                    rng.word()
                } else {
                    rng.word() ^ (sym & 0xff00)
                };
                match kind {
                    K::This(n) => {
                        t.stub(Some(at), sym, true, n, n, Some(ret));
                    }
                    K::KeepDeref(n) => {
                        t.deref_this = true;
                        t.stub(Some(at), sym, true, n, n, None);
                        t.deref_this = false;
                    }
                    K::Keep(n) => {
                        t.stub(Some(at), sym, true, n, n, None);
                    }
                    K::Cdecl(n) => {
                        t.stub(Some(at), sym, false, n, 0, Some(ret));
                    }
                    K::Entry => {
                        t.stub(Some(at), sym, true, 0, 0, Some(obj + 0x200));
                    }
                    K::Getter => {
                        if !snapshot {
                            t.stub(Some(at), sym, false, 0, 0, Some(instance));
                        }
                    }
                    K::Import(n) => {
                        let s = t.stub(None, sym, false, n, n, Some(ret));
                        t.patch(at, &s.to_le_bytes());
                    }
                    K::ImportC(n) => {
                        let s = t.stub(None, sym, false, n, 0, Some(ret));
                        t.patch(at, &s.to_le_bytes());
                    }
                }
            }
            let args: Vec<u32> = (0..case.args)
                .map(|_| {
                    if case.float_args {
                        rng.float().to_bits()
                    } else {
                        rng.word()
                    }
                })
                .collect();
            let mut args = args;
            let mut this = match case.this {
                This::None | This::Stdcall => 0,
                This::Global(gl) => t.r32(t.rebase(gl)),
                This::Object => obj,
            };
            if let Some(prep) = case.prep {
                prep(t, obj, &mut this, &mut args, rng);
            }
            let conv = match case.this {
                This::None => Conv::Cdecl,
                This::Stdcall => Conv::Stdcall,
                _ => Conv::Thiscall(this),
            };
            let what = format!("{} #{i}", case.name);
            let port = case.port;
            match case.frame {
                None => t.check(&what, case.sym, conv, &args, |m| port(m, this, &args)),
                Some((low, high)) => {
                    // the original's ebp: its stack arguments, the return
                    // address and the saved ebp below where the call pushes
                    let ebp = t.a.stack_range().end - 0x100 - 4 * args.len() as u32 - 8;
                    let lo = ebp - low;
                    let fill = rng.bytes((low + 8 + high) as usize);
                    t.write(lo, &fill);
                    for (off, bytes) in FRAME_WRITES.with(RefCell::take) {
                        t.write(ebp - off, &bytes);
                    }
                    FRAME_EBP.with(|f| f.set(ebp));
                    t.b.set_call_top(Some(lo - 0x100));
                    t.check(&what, case.sym, conv, &args, |m| port(m, this, &args));
                    t.b.set_call_top(None);
                    let ignore = FRAME_IGNORE.with(RefCell::take);
                    // what the callees saw through the frame's pointers
                    for (from, n) in [(lo, low), (ebp + 8, high)] {
                        let mut x = vec![0; n as usize];
                        let mut y = vec![0; n as usize];
                        Mem::read_bytes(&t.a, from, &mut x);
                        Mem::read_bytes(&t.b, from, &mut y);
                        let top = if case.seh {
                            16
                        } else {
                            4 * u32::from(case.cookie)
                        };
                        if from == lo && n >= top {
                            let cookie = (n - top) as usize;
                            x[cookie..].copy_from_slice(&y[cookie..]);
                        }
                        if from == lo {
                            for &(off, len) in &ignore {
                                let start = (low - off) as usize;
                                let end = start + len as usize;
                                y[start..end].copy_from_slice(&x[start..end]);
                            }
                        }
                        if let Some(first) = x.iter().zip(&y).position(|(p, q)| p != q) {
                            let at = from + first as u32;
                            let shown = (first + 16).min(x.len());
                            panic!(
                                "{what}: frame at {from:#x} differs at {at:#x} (ebp - {:#x}): {:?} vs {:?}",
                                ebp - at,
                                &x[first..shown],
                                &y[first..shown]
                            );
                        }
                    }
                }
            }
        }
    }
}

thread_local! {
    /// The original's `ebp` for a case with a frame ([`Case::frame`]),
    /// which the port's frame takes.
    static FRAME_EBP: Cell<u32> = const { Cell::new(0) };
}

fn frame_ebp() -> u32 {
    FRAME_EBP.with(Cell::get)
}

const fn framed(case: Case, low: u32, high: u32) -> Case {
    Case {
        frame: Some((low, high)),
        ..case
    }
}

const fn with_cookie(case: Case) -> Case {
    Case {
        cookie: true,
        ..case
    }
}

/// Writes the case's arguments into the frame's argument slots (the
/// original's call pushed them there); their address.
fn slots_at(m: &mut G, a: &[u32]) -> u32 {
    let at = frame_ebp() + 8;
    for (k, &w) in a.iter().enumerate() {
        m.write(at + 4 * k as u32, w);
    }
    at
}

thread_local! {
    /// Bytes stored into a framed case's frame (at `ebp` offsets) after
    /// the random fill: 0 for words the original reads but never writes
    /// (junk reads), which the port takes as 0 (brief 2026-10-24), or
    /// what a stubbed callee would have written.
    /// Bytes of a framed case's frame (`ebp` offset, length) not
    /// compared: what the original's prologue leaves there (the
    /// `alloca_probe` call's return address).
    static FRAME_IGNORE: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
    static FRAME_WRITES: RefCell<Vec<(u32, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

// ---- batch 1 (ffx_prender::h3b_glue1) ----

/// `ShadowManager_selectShadowSolution`: every hash of the switch, its
/// neighbours, the edges (0, negatives) and random ones; battle, the map
/// and the scenario around their tests.
#[test]
fn batch1_select_shadow_solution_matches_the_original() {
    use ffx_prender::h3b_glue1::{self as h1, bridge as b1};
    Twins::each(|t| {
        let mut rng = Rng(0xb3b1_0001);
        let this = t.zeros(0x4100);
        let mut hashes: Vec<u32> = vec![
            0,
            0xffff_ffff,
            0x8000_0000,
            0x7fff_ffff,
            0x12be,
            0x1733,
            0x049d,
            0x0b96,
            0x1432,
            0x178d,
            0x1d21,
        ];
        for &h in h1::TWO.iter().chain(&h1::FOUR).chain(&h1::ONE) {
            hashes.extend([h as u32 - 1, h as u32, h as u32 + 1]);
        }
        for _ in 0..200 {
            hashes.push(rng.below(0x2800));
        }
        // the two with their own rules, around their scenario and map
        for _ in 0..12 {
            hashes.extend([0x12be, 0x1733]);
        }
        let reps = if t.snapshot { 1 } else { 3 };
        for (i, &hash) in hashes.iter().enumerate() {
            for r in 0..reps {
                t.reset();
                let at = t.rebase(h1::CLEAR_TARGETS);
                let ret = rng.word();
                t.stub(Some(at), h1::CLEAR_TARGETS, true, 1, 1, Some(ret));
                let fill = rng.bytes(0x70);
                t.write(this + 0x4000, &fill);
                for g in [
                    h1::DEFAULT_403C,
                    h1::CASCADE_PARAMS,
                    h1::CASCADE_PARAMS + 4,
                    h1::CASCADE_EXTRA_A,
                ] {
                    let v = rng.float().to_bits();
                    t.global(g, v);
                }
                let battle = if rng.chance(6) {
                    (rng.next() & 0xff) | 1
                } else {
                    rng.next() & 0xffff_ff00
                };
                let map = if rng.chance(2) { 0x18a } else { rng.word() };
                let scenario =
                    [0xb53, 0xb54, 0xb55, 0, 0xffff_ffff, rng.next()][rng.below(6) as usize];
                let args = [rng.next(), battle, map, hash, scenario, rng.next()];
                let what = format!("select_shadow_solution {hash:#x} #{i}.{r}");
                t.check(&what, 0x006c_cd10, Conv::Thiscall(this), &args, |m| {
                    u(b1::select_shadow_solution(
                        m,
                        this,
                        args[1] as u8,
                        args[2],
                        args[3],
                        args[4],
                        args[5],
                    ))
                });
            }
        }
    });
}

// ---- helpers from h3_glue_render.rs ----

impl Twins {
    /// Places `code` in the cave; its address.
    fn code(&mut self, code: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(
            addr + code.len() as u32 <= self.cave + CAVE_LEN,
            "cave full"
        );
        self.write(addr, code);
        self.cave_next = addr + code.len() as u32;
        addr
    }

    /// A recording stub at `sym` returning `ret`.
    fn stub_ret(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        self.stub(Some(at), sym, this, n, pops, Some(ret));
    }

    /// A recording stub at `sym` (as [`Twins::stub`], `eax` kept) whose
    /// calls return `values` in turn.
    fn stub_seq(&mut self, sym: u32, this: bool, n: u32, pops: u32, values: &[u32]) {
        let words: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        let log = self.stub(None, sym, this, n, pops, None);
        let mut c = vec![0x8b, 0x15];
        c.extend(ptr.to_le_bytes()); // mov edx, [ptr]
        c.extend([0x8b, 0x02]); // mov eax, [edx]
        c.extend([0x83, 0xc2, 0x04]); // add edx, 4
        c.extend([0x89, 0x15]);
        c.extend(ptr.to_le_bytes()); // mov [ptr], edx
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

const fn with_seh(case: Case) -> Case {
    Case { seh: true, ..case }
}

/// Eight results for a stub called up to eight times: 0, with one error
/// now and then.
fn errors(rng: &mut Rng) -> Vec<u32> {
    let mut v = vec![0; 8];
    let k = rng.below(14) as usize;
    if k < v.len() {
        v[k] = rng.next() | 1;
    }
    v
}

fn pick(rng: &mut Rng, vs: &[u32]) -> u32 {
    vs[rng.below(vs.len() as u32) as usize]
}

// ---- batch 1: PostProcessManager_renderUSM (ffx_prender::h3b_glue2) ----

use ffx_prender::h3b_glue2::{self as h2, bridge as b2};

/// A render target record at `at` with a texture (`+4` clear: the
/// original reads its size), or none now and then where `none`.
fn target_usm(t: &mut Twins, at: u32, texture: u32, none: bool, rng: &mut Rng) {
    t.w32(at, texture);
    t.write(at + 4, &[u8::from(none && rng.chance(4))]);
    let w = pick(rng, &[0, 1, 1280, 0x8000_0000, 0xffff_ffff]) ^ (rng.next() & 0xff);
    t.w32(texture + 0x1c, w);
    let hgt = if rng.chance(2) { rng.next() } else { 720 };
    t.w32(texture + 0x20, hgt);
}

/// The manager's switches and map, its six targets, the mesh's material
/// array (one inline element or a pointer), the parameters (found or
/// not; the sampler record's offset and size against the buffer's size),
/// the scale and the keep float; each renderer call failing at one
/// point now and then.
fn prep_usm(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let pm = t.zeros(0x1_2400);
    *this = pm;
    for at in [0x123d2, 0x123d3] {
        let v = if rng.chance(8) {
            0
        } else {
            rng.next() as u8 | 1
        };
        t.write(pm + at, &[v]);
    }
    let map = pick(rng, &[0x17, 0x15c, 0x16, 0x41, 0x41, 0x41, 0x41, 0x41]);
    t.global(h2::CURRENT_MAP_NO, map);
    for (k, at) in [0x8c, 0x9c, 0x1178, 0xac, 0x117c, 0x1188]
        .into_iter()
        .enumerate()
    {
        let record = obj + 0x100 + 0x10 * k as u32;
        t.w32(pm + at, record);
        target_usm(t, record, obj + 0x1000 + 0x40 * k as u32, k < 3, rng);
    }
    let mesh = obj + 0x300;
    t.w32(pm + 0x1229c, mesh);
    let array = obj + 0x3a0;
    t.w32(mesh + 0x10, array);
    let element = obj + 0x380;
    if rng.chance(2) {
        t.w32(array, 1 | (rng.next() & 0x8000_0000));
        t.w32(array + 4, element);
    } else {
        t.w32(array, pick(rng, &[2, 3, 0x8000_0005]));
        t.w32(array + 4, obj + 0x3c0);
        t.w32(obj + 0x3c0, element);
    }
    let shader = rng.next();
    t.w32(element, shader);
    let buffer = obj + 0x500;
    t.w32(element + 4, buffer);
    t.w32(buffer, pick(rng, &[0, 4, 0x14, 0x20, 0x80, 0x100]));
    let param = obj + 0x400;
    let offset = pick(rng, &[0, 2, 3, 4, 5, 8, 0x10, 0x40, 0x70, 0xf0, 0xfff0]);
    t.write(param + 8, &(offset as u16).to_le_bytes());
    // sizes over 16 only where they don't fit (the copy would reach the
    // frame above the record); the high bits are masked
    let size = pick(rng, &[0, 1, 4, 12, 15, 16, 0x200, 0x1f00]) | (rng.below(8) << 13);
    t.write(param + 0xa, &(size as u16).to_le_bytes());
    let found: Vec<u32> = (0..7)
        .map(|_| if rng.chance(4) { 0 } else { param })
        .collect();
    t.stub_seq(h2::FIND_PARAMETER, true, 1, 1, &found);
    for at in [0x122a4, 0x122a8, 0x122ac] {
        let v = rng.float().to_bits();
        t.w32(pm + at, v);
    }
    let keep = if rng.chance(2) {
        pick(rng, &[0, 0x8000_0000])
    } else {
        rng.float().to_bits()
    };
    t.w32(pm + 0x122b0, keep);
    for (sym, n) in [
        (h2::SET_TEXTURE, 2),
        (h2::SET_VALUE, 2),
        (h2::SET_VALUE2, 2),
        (h2::SET_COLOR_TARGET, 2),
        (h2::SET_DEPTH_STENCIL_TARGET, 1),
        (h2::SET_CLEAR_COLOR, 4),
        (h2::BEGIN_SCENE, 1),
        (h2::END_SCENE, 1),
        (h2::RENDER_MESH_INSTANCE, 4),
        (h2::RENDER_MESH_FOR_PASS, 1),
    ] {
        let v = errors(rng);
        t.stub_seq(sym, true, n, n, &v);
    }
    let renderer = obj + 0x600;
    let extra = rng.next();
    t.w32(renderer + 0x13c, extra);
    args[0] = renderer;
}

const USM_STUBS: &[(u32, K)] = &[
    (h2::SURFACE_CTOR, K::This(2)),
    (h2::SURFACE_DTOR, K::This(0)),
    (h2::SET_VIEWPORT, K::This(4)),
    (h2::COPY_BUFFER, K::This(6)),
    (h2::GET_LINEAR_SAMPLER, K::This(1)),
];

#[rustfmt::skip]
const CASES_USM: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_usm), ..method("render_usm", 0x0069_7670, This::Object, 1, USM_STUBS, |m, this, a| { let slots = slots_at(m, a); u(b2::render_usm(m, frame_ebp(), slots, this)) }) }), 0xb8, 4),
];

#[test]
fn batch1_render_usm_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 800 };
        run_cases(t, CASES_USM, n, &mut Rng(0xb3b1_0002));
    });
}

// ---- batch 2: PostProcessManager_onAllShaderBinFilesLoaded (ffx_prender::h3b_glue3) ----

use ffx_prender::h3b_glue3::{self as h3, bridge as b3};

impl Twins {
    /// A recording stub at `sym` for a method with two stack words: logs
    /// `tag`, `ecx` and each word, or the word it points at where
    /// `deref` (what the callee reads through it at the call); pops 2
    /// words, returns `ret`.
    fn stub_deref(&mut self, sym: u32, deref: [bool; 2], ret: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        for (k, d) in (0..2u8).zip(deref) {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp+4+4k]
            if d {
                c.extend([0x8b, 0x12]); // mov edx, [edx]
            }
            c.extend([0x89, 0x50, 8 + 4 * k]); // mov [eax+8+4k], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.extend([0xc2, 0x08, 0x00]); // ret 8
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// `n` results: 0, with one error now and then.
fn errors_n(rng: &mut Rng, n: usize) -> Vec<u32> {
    let mut v = vec![0; n];
    let k = rng.below(2 * n as u32) as usize;
    if k < n {
        v[k] = rng.next() | 1;
    }
    v
}

/// A preset file's contents at `at`: a count, then records of a
/// length-prefixed name and 13 words.
fn presets(t: &mut Twins, at: u32, rng: &mut Rng) {
    let count = pick(rng, &[0, 0xffff_ffff, 1, 2, 3]);
    let mut b = count.to_le_bytes().to_vec();
    for _ in 0..count.min(3) {
        let len = pick(rng, &[0, 3, 8, 15]) as u8;
        b.push(len);
        b.extend((0..len).map(|_| b'a' + rng.below(26) as u8));
        for _ in 0..13 {
            b.extend(rng.next().to_le_bytes());
        }
    }
    t.write(at, &b);
}

/// The manager (settings or none, its size and flags), the variants (one
/// missing now and then), the MSAA mode, created textures and arrays
/// (each array's head: inline, empty, owned or borrowed storage), the
/// targets (one missing now and then), the calls failing now and then,
/// the preset files (open or not, by `al` only).
#[allow(clippy::too_many_lines)] // the callees
fn prep_loaded(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let pm = t.zeros(0x1_2410);
    *this = pm;
    let settings = if rng.chance(6) { 0 } else { obj + 0x40 };
    t.w32(pm, settings);
    for k in 0..0x28 {
        let v = rng.float().to_bits();
        t.w32(obj + 0x40 + 4 * k, v);
    }
    // the preset table and the flag bytes hold old values
    let old = rng.bytes(0x44 * 10);
    t.write(pm + 0x126c, &old);
    let flags = rng.bytes(8);
    t.write(pm + 0x123d0, &flags);
    for at in [0x123f8, 0x12400, 0x12404, 0x122e8, 0x1230c] {
        let v = rng.next();
        t.w32(pm + at, v);
    }
    let index = rng.below(6);
    t.w32(pm + 0x1268, index);
    let variants: Vec<u32> = (0..11)
        .map(|_| if rng.chance(14) { 0 } else { rng.next() | 1 })
        .collect();
    t.stub_seq(h3::POST_LOOKUP, true, 1, 1, &variants);
    let app = obj + 0x800;
    t.stub_ret(h3::GET_APPLICATION, false, 0, 0, app);
    let mode = pick(rng, &[0, 0, 2, 4, 8, 0x10, 1, 3, 0x20]);
    t.w32(app + 0x284, mode);
    // textures and arrays from one pool (an array's head and storage)
    let pool = obj + 0x1000;
    let mut created = vec![];
    for k in 0..40 {
        let at = pool + 0x40 * k;
        let bytes = rng.bytes(0x40);
        t.write(at, &bytes);
        let head = pick(rng, &[1, 0, 2, 3, 0x8000_0002, 0x8000_0001, 0x8000_0000]);
        t.w32(at, head);
        let data = pick(rng, &[at + 4, obj + 0x3000, 0]);
        t.w32(at + 4, data);
        created.push(at);
    }
    if rng.chance(10) {
        created[0] = 0;
    }
    t.stub_seq(h3::CLUSTER_CREATE, true, 3, 3, &created);
    let targets: Vec<u32> = (0..12)
        .map(|k| {
            if rng.chance(60) {
                0
            } else {
                obj + 0x2000 + 0x20 * k
            }
        })
        .collect();
    t.stub_seq(h3::MAKE_TARGET, true, 1, 1, &targets);
    let dims = errors_n(rng, 12);
    t.stub_seq(h3::SET_DIMENSIONS, true, 4, 4, &dims);
    let blur2 = errors_n(rng, 2);
    t.stub_seq(h3::BLUR2_INITIALIZE, true, 3, 3, &blur2);
    for mesh in [h3::MESH_TRIANGLE, h3::MESH_QUAD, h3::MESH_HDAO] {
        let meshes: Vec<u32> = (0..8)
            .map(|k| obj + 0x2400 + 0x20 * k + (mesh & 0xf00))
            .collect();
        t.stub_seq(mesh, false, 1, 0, &meshes);
    }
    // the preset files: open by al only
    let open: Vec<u32> = (0..2).map(|_| pick(rng, &[0, 1, 0x100, 0xff01])).collect();
    t.stub_seq(h3::READER_IS_OPEN, true, 0, 0, &open);
    let size = rng.next();
    t.stub_ret(h3::STREAM_SIZE, true, 0, 0, size);
    let buf = obj + 0x4000;
    presets(t, buf, rng);
    t.stub_ret(h3::NEW, false, 1, 0, buf);
    t.stub_deref(h3::MAP_INSERT, [true, true], 0);
    FRAME_IGNORE.with(|f| f.borrow_mut().push((0x28, 0x18)));
}

const LOADED_STUBS: &[(u32, K)] = &[
    (h3::REGISTER, K::This(0)),
    (h3::UPDATE_MAX_LEVEL, K::This(0)),
    (h3::CREATE_MATERIAL, K::This(3)),
    (h3::FREE, K::Cdecl(1)),
    (h3::SET_WORD, K::This(2)),
    (h3::GLOW_INITIALIZE, K::This(3)),
    (h3::GLOW_SHADERS, K::This(2)),
    (h3::BLUR_INITIALIZE, K::This(3)),
    (h3::BLUR_SHADERS, K::This(2)),
    (h3::BLUR2_SHADERS, K::This(2)),
    (h3::SCREEN_FADE_MESH, K::This(0)),
    (h3::SETTING_TABLE, K::This(1)),
    (h3::SAMPLER_FILTER, K::This(1)),
    (h3::SAMPLER_ADDRESS_U, K::This(1)),
    (h3::SAMPLER_ADDRESS_V, K::This(1)),
    (h3::SAMPLER_ADDRESS_W, K::This(1)),
    (h3::READER_CTOR, K::This(1)),
    (h3::READER_CLOSE, K::This(0)),
    (h3::READER_DTOR, K::This(0)),
    (h3::STREAM_READ, K::This(2)),
    (h3::DELETE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES_LOADED: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_loaded), ..method("on_all_shader_bin_files_loaded", 0x0069_3d70, This::Object, 0, LOADED_STUBS, |m, this, _| u(b3::on_all_shader_bin_files_loaded(m, frame_ebp(), this))) }), 0x44, 0),
];

#[test]
fn batch2_on_all_shader_bin_files_loaded_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES_LOADED, n, &mut Rng(0xb3b2_0003));
    });
}

// ---- batch 3: PostProcessManager_renderHDAOPass (ffx_prender::h3b_glue4) ----

use ffx_prender::h3b_glue4::{self as h4, bridge as b4};

/// The manager (on, the setting, MSAA, its targets, the HDAO mesh's
/// material, the parameter floats, the blur count, the apply switch), the
/// scene camera's projection, the parameters (found or not; their offset
/// and size against the buffer), each call failing now and then.
#[allow(clippy::too_many_lines)] // the callees
fn prep_hdao(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let pm = t.zeros(0x1_2410);
    *this = pm;
    let on = if rng.chance(8) {
        0
    } else {
        rng.next() as u8 | 1
    };
    t.write(pm + 0x123d0, &[on]);
    let was = u8::from(rng.chance(2));
    t.write(pm + 0x123d1, &[was]);
    let setting = pick(rng, &[0, 1, 2, 3, 4, 2, 3]);
    t.w32(pm + 0x1264, setting);
    let msaa = u8::from(rng.chance(2));
    t.write(pm + 0x122b4, &[msaa]);
    for (k, at) in [0x8c, 0x90, 0x98, 0x9c, 0xa4].into_iter().enumerate() {
        let record = obj + 0x100 + 0x10 * k as u32;
        t.w32(pm + at, record);
        target_usm(t, record, obj + 0x1000 + 0x40 * k as u32, at < 0x9c, rng);
    }
    // +0x8c's texture is read for its sizes
    t.write(obj + 0x104, &[0]);
    let mesh = obj + 0x300;
    t.w32(pm + 0x121c, mesh);
    let array = obj + 0x3a0;
    t.w32(mesh + 0x10, array);
    let element = obj + 0x380;
    if rng.chance(2) {
        t.w32(array, 1);
        t.w32(array + 4, element);
    } else {
        t.w32(array, pick(rng, &[2, 0x8000_0004]));
        t.w32(array + 4, obj + 0x3c0);
        t.w32(obj + 0x3c0, element);
    }
    let shader = rng.next();
    t.w32(element, shader);
    let buffer = obj + 0x500;
    t.w32(element + 4, buffer);
    t.w32(buffer, pick(rng, &[0, 4, 0x14, 0x20, 0x80, 0x100]));
    for k in 0..0x10 {
        let v = rng.float().to_bits();
        t.w32(pm + 0x1224 + 4 * k, v);
    }
    let count = pick(rng, &[0, 1, 1, 2, 3, 0xffff_ffff]);
    t.w32(pm + 0x125c, count);
    let apply = u8::from(rng.chance(2));
    t.write(pm + 0x1260, &[apply]);
    // the scene camera's projection
    let scene = obj + 0x700;
    t.global(h4::SCENE, scene);
    t.w32(scene + 0xdc, obj + 0x800);
    for k in 0..16 {
        let v = if rng.chance(2) {
            rng.float().to_bits()
        } else {
            (rng.below(2000) as f32 / 700.0 - 1.3).to_bits()
        };
        t.w32(obj + 0x800 + 0x34 + 4 * k, v);
    }
    let param = obj + 0x400;
    let offset = pick(rng, &[0, 2, 3, 4, 5, 8, 0x10, 0x40, 0x70, 0xf0, 0xfff0]);
    t.write(param + 8, &(offset as u16).to_le_bytes());
    // the ring count is one word: sizes over 4 only where they don't fit
    let size = if rng.chance(2) {
        pick(rng, &[0, 1, 4])
    } else {
        pick(rng, &[0x200, 0x1f00])
    } | (rng.below(8) << 13);
    t.write(param + 0xa, &(size as u16).to_le_bytes());
    // a second parameter at another offset, so copies to one don't hide
    // copies to the other
    let param2 = obj + 0x420;
    let offset2 = pick(rng, &[4, 0x18, 0x30]);
    t.write(param2 + 8, &(offset2 as u16).to_le_bytes());
    t.write(param2 + 0xa, &4u16.to_le_bytes());
    let found: Vec<u32> = (0..64)
        .map(|_| pick(rng, &[0, param, param, param2, param2]))
        .collect();
    t.stub_seq(h4::FIND_PARAMETER, true, 1, 1, &found);
    let value2 = if rng.chance(12) { rng.next() | 1 } else { 0 };
    t.stub_deref(h4::SET_VALUE2, [false, true], value2);
    for (sym, n) in [
        (h4::SET_TEXTURE, 2),
        (h4::SET_VALUE, 2),
        (h4::COPY_TO_BUFFER, 2),
        (h4::SET_COLOR_TARGET, 2),
        (h4::SET_DEPTH_STENCIL_TARGET, 1),
        (h4::SET_CLEAR_COLOR, 4),
        (h4::BEGIN_SCENE, 1),
        (h4::END_SCENE, 1),
        (h4::RENDER_MESH_FOR_PASS, 1),
    ] {
        let v = errors_n(rng, 24);
        t.stub_seq(sym, true, n, n, &v);
    }
    let renderer = obj + 0x600;
    args[0] = renderer;
    args[1] = rng.next();
    args[2] = obj + 0x180;
    target_usm(t, obj + 0x180, obj + 0x1200, false, rng);
    args[3] = obj + 0x190;
    target_usm(t, obj + 0x190, obj + 0x1240, true, rng);
    FRAME_IGNORE.with(|f| f.borrow_mut().push((0x2c, 0x1c)));
}

const HDAO_STUBS: &[(u32, K)] = &[
    (h4::RESOLVE_DEPTH, K::This(3)),
    (h4::SURFACE_CTOR, K::This(2)),
    (h4::SURFACE_DTOR, K::This(0)),
    (h4::SET_VIEWPORT, K::This(4)),
    (h4::COPY_BUFFER, K::This(6)),
];

#[rustfmt::skip]
const CASES_HDAO: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_hdao), ..method("render_hdao_pass", 0x0069_5ba0, This::Object, 4, HDAO_STUBS, |m, this, a| { let slots = slots_at(m, a); u(b4::render_hdao_pass(m, frame_ebp(), slots, this)) }) }), 0x16c, 0x10),
];

#[test]
fn batch3_render_hdao_pass_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 800 };
        run_cases(t, CASES_HDAO, n, &mut Rng(0xb3b3_0004));
    });
}

// ---- batch 4: ShadowManager_createResources_006d1b40 (ffx_prender::h3b_glue5) ----

use ffx_prender::h3b_glue5::{self as h5, bridge as b5};

impl Twins {
    /// A recording stub at `sym` (logging `ecx`) that first stores the
    /// words `writes` at `[ecx + off]` (an object the callee fills).
    fn stub_this_out(
        &mut self,
        sym: u32,
        n: u32,
        pops: u32,
        ret: Option<u32>,
        writes: &[(u8, u32)],
    ) {
        let log = self.stub(None, sym, true, n, pops, ret);
        let mut c = vec![];
        for &(off, v) in writes {
            c.extend([0xc7, 0x41, off]); // mov dword [ecx+off], v
            c.extend(v.to_le_bytes());
        }
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// The manager (old contents: sampler flags, arrays to release), the
/// cluster's object list (empty or not), the meshes (the triangle missing
/// now and then), the cursor's find, the material array's head.
fn prep_shadow_resources(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let sm = t.zeros(0x4100);
    *this = sm;
    let old = rng.bytes(0x4100);
    t.write(sm, &old);
    let cluster = obj + 0x200;
    t.w32(sm, cluster);
    let list = cluster + 0x1c;
    let first = if rng.chance(3) { list } else { obj + 0x240 };
    t.w32(list, first);
    let mesh = if rng.chance(8) { 0 } else { obj + 0x300 };
    t.stub_ret(h5::MESH_TRIANGLE, false, 1, 0, mesh);
    let w = rng.next();
    t.w32(obj + 0x300, w);
    t.stub_ret(h5::MESH_SPLIT, false, 6, 0, obj + 0x340);
    let w = rng.next();
    t.w32(obj + 0x340, w);
    let found = if rng.chance(3) { 0 } else { rng.next() | 1 };
    let base = rng.next();
    let off = rng.below(0x100);
    t.stub_this_out(
        h5::CURSOR_SEEK,
        0,
        0,
        None,
        &[(0xc, found), (0, base), (0x10, off)],
    );
    let head = pick(rng, &[1, 0, 2, 3, 0x8000_0002, 0x8000_0001, 0x8000_0000]);
    t.w32(obj + 0x400, head);
    let data = pick(rng, &[obj + 0x404, obj + 0x480, 0]);
    t.w32(obj + 0x404, data);
    let c84 = if rng.chance(4) { 0 } else { obj + 0x500 };
    let (c88, c8c) = (rng.next(), rng.next());
    t.stub_seq(
        h5::CLUSTER_CREATE,
        true,
        3,
        3,
        &[obj + 0x400, c84, c88, c8c],
    );
}

const SHADOW_RESOURCES_STUBS: &[(u32, K)] = &[
    (h5::SHADOW_MESH, K::This(1)),
    (h5::CREATE_MATERIAL, K::This(3)),
    (h5::FREE, K::Cdecl(1)),
    (h5::REGISTER, K::This(0)),
];

#[rustfmt::skip]
const CASES_SHADOW_RESOURCES: &[Case] = &[
    framed(Case { prep: Some(prep_shadow_resources), ..method("shadow_create_resources", 0x006d_1b40, This::Object, 0, SHADOW_RESOURCES_STUBS, |m, this, _| u(b5::create_resources(m, frame_ebp(), this))) }, 0x28, 0),
];

#[test]
fn batch4_shadow_create_resources_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES_SHADOW_RESOURCES, n, &mut Rng(0xb3b4_0005));
    });
}

// ---- round 2 batch 1: TextureAnimationManager_addAnimatedMaterials (ffx_prender::h3b_glue6) ----

use ffx_prender::h3b_glue6::{self as h6, bridge as b6};

/// A cursor script word the stub leaves alone.
const KEEP: u32 = 0x5e5e_5e5e;

impl Twins {
    /// A NUL-terminated string; its address.
    fn cstr(&mut self, s: &str) -> u32 {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        self.alloc(&b)
    }

    /// `jmp target` appended to `c`, which will sit at `base`.
    fn jmp_to(c: &mut Vec<u8>, base: u32, target: u32) {
        c.push(0xe9);
        let end = base + c.len() as u32 + 4;
        c.extend(target.wrapping_sub(end).to_le_bytes());
    }

    /// Sends the calls of `sym` to `code` (in the cave).
    fn divert(&mut self, sym: u32, code: u32) {
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// Bump allocators (cdecl, `(sym, words)`, the size first) over one
    /// arena, logged as [`Twins::stub`]; the call that brings the shared
    /// countdown to 0 returns 0 (a countdown of 0 never does). Only
    /// `addAnimatedMaterials`' own calls count down (its callees crash on
    /// a failed allocation, original and port alike).
    fn stub_bump(&mut self, syms: &[(u32, u32)], arena: u32, fail_after: u32) {
        let cell = self.alloc(&arena.to_le_bytes());
        let count = self.alloc(&fail_after.to_le_bytes());
        for &(sym, n) in syms {
            let log = self.stub(None, sym, false, n, 0, None);
            let base = self.cave_next;
            let mut ok = vec![0xa1];
            ok.extend(cell.to_le_bytes()); // mov eax, [cell]
            ok.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
            ok.extend([0x83, 0xc1, 0x13]); // add ecx, 0x13
            ok.extend([0x83, 0xe1, 0xf0]); // and ecx, -16
            ok.extend([0x01, 0x0d]);
            ok.extend(cell.to_le_bytes()); // add [cell], ecx
            // only calls from the original itself or from the port (whose
            // return address is outside the image's code) may fail
            let mut c = vec![0x8b, 0x14, 0x24]; // mov edx, [esp]
            for (bound, jump) in [
                (self.rebase(0x006b_05d0), 8u8), // jb outside
                (self.rebase(0x006b_1a0d), 16),  // jb eligible
                (self.rebase(0x0040_1000), 8),   // outside: jb eligible
                (self.rebase(0x00b0_ba7b), 8),   // jb ok
            ] {
                c.extend([0x81, 0xfa]);
                c.extend(bound.to_le_bytes()); // cmp edx, bound
                c.extend([0x72, jump]);
            }
            c.extend([0xff, 0x0d]);
            c.extend(count.to_le_bytes()); // eligible: dec dword [count]
            c.extend([0x74, (ok.len() + 5) as u8]); // jz fail
            c.extend(ok);
            Self::jmp_to(&mut c, base, log);
            c.extend([0x31, 0xc0]); // fail: xor eax, eax
            Self::jmp_to(&mut c, base, log);
            let code = self.code(&c);
            self.divert(sym, code);
        }
    }

    /// Cursor stubs at `syms` sharing one script: each call copies the next
    /// five words of `steps` (base, block, stride, count, offset) into the
    /// cursor (`ecx`), but for [`KEEP`] words, then logs as
    /// [`Twins::stub`].
    fn stub_cursor_run(&mut self, syms: &[u32], steps: &[[u32; 5]]) {
        let words: Vec<u8> = steps
            .iter()
            .flatten()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        for &sym in syms {
            let log = self.stub(None, sym, true, 0, 0, Some(sym));
            let base = self.cave_next;
            let mut c = vec![0xa1];
            c.extend(ptr.to_le_bytes()); // mov eax, [ptr]
            for i in 0..5u8 {
                c.extend([0x8b, 0x50, 4 * i]); // mov edx, [eax+4i]
                c.extend([0x81, 0xfa]);
                c.extend(KEEP.to_le_bytes()); // cmp edx, KEEP
                c.extend([0x74, 0x03]); // je +3
                c.extend([0x89, 0x51, 4 * i]); // mov [ecx+4i], edx
            }
            c.extend([0x83, 0xc0, 0x14]); // add eax, 20
            c.push(0xa3);
            c.extend(ptr.to_le_bytes()); // mov [ptr], eax
            Self::jmp_to(&mut c, base, log);
            let code = self.code(&c);
            self.divert(sym, code);
        }
    }
}

/// Cursor runs over the slots `area + 0x100 k` (the element at `+4`): one
/// to three runs of 0..4, each with a block word that may be one of its
/// slots (`[block]` the next block), then the end (nothing written).
/// With `null`, now and then a lone element at address 0 (skipped by the
/// materials' walk).
fn aam_cursor_steps(t: &mut Twins, area: u32, null: bool, rng: &mut Rng) -> Vec<[u32; 5]> {
    let mut steps = vec![];
    for _ in 0..=rng.below(3) {
        let base = area + 0x100 * rng.below(4);
        let block = if rng.chance(2) {
            base + 0x100 * (1 + rng.below(3))
        } else {
            rng.next() | 3
        };
        if block % 4 == 0 {
            let next = if rng.chance(2) { block } else { rng.next() | 3 };
            t.w32(block, next);
        }
        if null && rng.chance(4) {
            steps.push([base, block, 0x100, 1, 0u32.wrapping_sub(base)]);
        } else {
            steps.push([base, block, 0x100, rng.below(5), 4]);
        }
    }
    steps.push([KEEP; 5]);
    steps
}

/// A tagged pointer now and then (bit 0: not owned).
fn tagged(rng: &mut Rng, p: u32) -> u32 {
    if p != 0 && rng.chance(3) { p + 1 } else { p }
}

/// A word array `{count, data}` at `at`: empty, or 1..3 words (top bit:
/// not owned).
fn aam_words(t: &mut Twins, at: u32, rng: &mut Rng) {
    let n = rng.below(4);
    let data = if n == 0 && rng.chance(2) {
        0
    } else {
        let w: Vec<u8> = (0..4 * n.max(1)).map(|_| rng.next() as u8).collect();
        t.alloc(&w)
    };
    t.w32(at, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
    t.w32(at + 4, data);
}

/// The scene's materials at the cursor slots of `area`: parameter arrays
/// of 0..3 materials (names plain, empty, `Mat_CTA` ones with the ids),
/// their shaders' key/value pairs and parameter buffers.
fn aam_materials(t: &mut Twins, area: u32, ids: &[u32], rng: &mut Rng) {
    let empty = t.rebase(h6::EMPTY);
    let uv = t.rebase(h6::CHR_UV);
    for slot in 0..16 {
        let object = area + 0x100 * slot + 4;
        let n = pick(rng, &[0, 1, 2, 3, 0x8000_0002]);
        let params = t.zeros(0x20);
        t.w32(object + 0x10, params);
        t.w32(params, n);
        let mut list = vec![];
        for _ in 0..(n & 0x7fff_ffff).max(1) {
            let mat = t.zeros(0x40);
            let shader = t.zeros(0x10);
            let buffer = t.zeros(0x100);
            t.w32(mat, shader);
            t.w32(mat + 4, buffer);
            t.w32(buffer, rng.below(0x50));
            let id = ids[rng.below(6) as usize];
            let name = match rng.below(7) {
                0 => empty,
                1 => t.cstr(""),
                2 => t.cstr("plain"),
                3 => t.cstr(&format!("Mat_CTA_c{id}")),
                4 => t.cstr(&format!("Mat_CTA_c{id}G")),
                5 => t.cstr(&format!("Mat_CTA_c{id}x")),
                _ => t.cstr(&format!("xMat_CTAc{id}")),
            };
            let name = tagged(rng, name);
            t.w32(mat + 0x10, name);
            let pairs = rng.below(4);
            let table = t.zeros(8 * pairs as usize + 8);
            t.w32(
                shader + 4,
                pairs | if rng.chance(5) { 0x8000_0000 } else { 0 },
            );
            t.w32(shader + 8, table);
            for j in 0..pairs {
                let key = match rng.below(4) {
                    0 => uv,
                    1 => t.cstr("CHR_TEXTURE_ANIMATION_UV"),
                    2 => t.cstr("OTHER"),
                    _ => 0,
                };
                let value = match rng.below(4) {
                    0 => empty,
                    1 => t.cstr(""),
                    2 => t.cstr("1"),
                    _ => 0,
                };
                let (key, value) = (tagged(rng, key), tagged(rng, value));
                t.w32(table + 8 * j, key);
                t.w32(table + 8 * j + 4, value);
            }
            list.push(mat);
        }
        if n & 0x7fff_ffff == 1 {
            t.w32(params + 4, list[0]);
        } else {
            let words: Vec<u8> = list.iter().flat_map(|w| w.to_le_bytes()).collect();
            let data = t.alloc(&words);
            t.w32(params + 4, data);
        }
    }
}

/// The manager: junk, word lists `+0x5c`/`+0x74`, old Info objects
/// `+0x7c` (not owned), materials `+0x64`/`+0x68` empty or not.
fn aam_manager(t: &mut Twins, rng: &mut Rng) -> u32 {
    let manager = t.zeros(0x100);
    let junk = rng.bytes(0x100);
    t.write(manager, &junk);
    for at in [0x5c, 0x74] {
        aam_words(t, manager + at, rng);
    }
    let info_vtable = t.rebase(h6::INFO_VTABLE);
    if rng.chance(2) {
        t.w32(manager + 0x7c, 0);
        t.w32(manager + 0x80, 0);
    } else {
        let k = 1 + rng.below(2);
        let infos = t.zeros(0x20 * k as usize);
        for i in 0..k {
            t.w32(infos + 0x20 * i, info_vtable);
        }
        t.w32(manager + 0x7c, k | 0x8000_0000);
        t.w32(manager + 0x80, infos);
    }
    let k = rng.below(4);
    let mats = t.zeros(0x20 * k as usize + 0x20);
    for i in 0..k {
        let e = mats + 0x20 * i;
        match rng.below(4) {
            0 => {}
            1 => {
                let w = t.alloc(&rng.next().to_le_bytes());
                t.w32(e, 1);
                t.w32(e + 4, w);
            }
            2 => {
                let w = t.alloc(&rng.next().to_le_bytes());
                t.w32(e + 0x10, 1);
                t.w32(e + 0x14, w);
            }
            _ => {
                let s = t.cstr("old");
                let s = tagged(rng, s);
                t.w32(e + 0x1c, s);
            }
        }
    }
    t.w32(
        manager + 0x64,
        k | if rng.chance(4) { 0x8000_0000 } else { 0 },
    );
    t.w32(
        manager + 0x68,
        if k == 0 && rng.chance(2) { 0 } else { mats },
    );
    manager
}

/// The clusters (index -1 readable; each list empty or not, the last's
/// object chain holding the Char class or not), the texture seeks and the
/// last cluster's animation objects (word lists only with `words`: the
/// Info copy constructor reads a list whose copy failed); returns
/// (clusters, first, last).
fn aam_clusters(t: &mut Twins, ids: &[u32], words: bool, rng: &mut Rng) -> (u32, u32, u32) {
    let clusters = t.zeros(0x10);
    let table = t.zeros(0x20);
    t.w32(clusters + 8, table + 4);
    let last = rng.below(5);
    let first = pick(rng, &[0, 1, 2, 3, 0xffff_ffff, last, last]);
    for i in 0..6 {
        let cluster = t.zeros(0x40);
        let anim = t.zeros(0x80);
        t.w32(table + 4 * i, cluster);
        let class = match rng.below(4) {
            0 => 0,
            1 => t.rebase(h6::CHAR_CLASS),
            _ => {
                let c = t.zeros(0x80);
                let next = if rng.chance(2) {
                    t.rebase(h6::CHAR_CLASS)
                } else {
                    0
                };
                t.w32(c + 0x40, next);
                c
            }
        };
        t.w32(anim + 0x2c, class);
        let objects = cluster + 0x1c;
        let at_last = i == last + 1;
        t.w32(
            objects,
            if !at_last && rng.chance(3) {
                objects
            } else {
                anim
            },
        );
    }
    let seeks: Vec<[u32; 5]> = (0..6)
        .map(|_| [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()])
        .collect();
    t.stub_cursor_run(&[h6::ITER_SEEK], &seeks);
    let area = t.zeros(0x1000);
    for slot in 0..16 {
        let rec = area + 0x100 * slot + 4;
        let id = ids[rng.below(6) as usize];
        let text = if rng.chance(4) {
            0
        } else {
            t.cstr(&format!("t{id}"))
        };
        let path = t.cstr(&format!("c{id}"));
        let n = if words { rng.below(3) } else { 0 };
        let bytes: Vec<u8> = (0..4 * n).map(|_| rng.next() as u8).collect();
        let words = if n == 0 { 0 } else { t.alloc(&bytes) };
        let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
        let (text, path) = (tagged(rng, text), tagged(rng, path));
        t.w32(rec, text);
        t.w32(rec + 0xc, n | top);
        t.w32(rec + 0x10, words);
        t.w32(rec + 0x14, path);
        for off in [8, 0x18, 0x1c, 0x20, 0x24] {
            let f = rng.float().to_bits();
            t.w32(rec + off, f);
        }
    }
    let steps = aam_cursor_steps(t, area, false, rng);
    t.stub_cursor_run(&[h6::ANIM_BEGIN, h6::ANIM_NEXT], &steps);
    (clusters, first, last)
}

/// The manager, the scene's materials and the clusters (scripted
/// cursors), the shader parameters, the shadow table text and the name;
/// allocations fail now and then.
fn prep_aam(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let heap = t.zeros(0x8000);
    let fail = if rng.chance(3) { 1 + rng.below(80) } else { 0 };
    t.stub_bump(&[(h6::MALLOC, 2), (0x0042_fb40, 1)], heap, fail);
    // the original writes through what `new` returns unchecked
    let pool = t.zeros(0x2000);
    t.stub_bump(&[(h6::NEW, 1), (h6::NEW_ARRAY, 1)], pool, 0);
    *this = aam_manager(t, rng);
    let ids: Vec<u32> = (0..6).map(|_| 1 + rng.below(6)).collect();
    let scene = t.zeros(0x40);
    let node = t.zeros(0x40);
    t.w32(
        scene + 0x1c,
        if rng.chance(3) { scene + 0x1c } else { node },
    );
    let area = t.zeros(0x1000);
    aam_materials(t, area, &ids, rng);
    let steps = aam_cursor_steps(t, area, true, rng);
    t.stub_cursor_run(&[h6::CURSOR_BEGIN, h6::CURSOR_NEXT], &steps);
    let (clusters, first, last) = aam_clusters(t, &ids, fail == 0, rng);

    // the shader parameters: none, or offset/size records
    let records = t.zeros(0x400);
    for k in 0..0x40 {
        let off = pick(rng, &[0, 2, 4, 8, 0x10, 0x30]) as u16;
        let size = (rng.below(9) as u16) | (rng.next() as u16 & 0xe000);
        t.write(records + 0x10 * k + 8, &off.to_le_bytes());
        t.write(records + 0x10 * k + 0xa, &size.to_le_bytes());
    }
    let values: Vec<u32> = (0..1200)
        .map(|_| {
            if rng.chance(2) {
                0
            } else {
                records + 0x10 * rng.below(0x40)
            }
        })
        .collect();
    t.stub_seq(h6::FIND_PARAM, true, 1, 1, &values);
    let shadows = t.zeros(0x10);
    t.stub_ret(h6::SHADOW_INSTANCE, false, 0, 0, shadows);
    let map = t.zeros(8);
    let (tex, flag) = (rng.next(), rng.below(2) as u8);
    t.w32(map, tex);
    t.write(map + 4, &[flag]);
    t.stub_ret(h6::DUMMY_SHADOW_MAP, true, 0, 0, map);

    // the shadow table's text: rows `key:v!,v?,\r\n`
    let mut text = String::new();
    for _ in 0..16 {
        let key = if rng.chance(4) {
            9
        } else {
            ids[rng.below(6) as usize]
        };
        let _ = write!(text, "{key}:");
        for _ in 0..rng.below(4) {
            let v = pick(rng, &[0, 1, 2, 3, 0xffff_ffff, 7]) as i32;
            let mark = if rng.chance(2) { '!' } else { '?' };
            let _ = write!(text, "{v}{mark},");
        }
        text += "\r\n";
    }
    let table_text = if rng.chance(5) { 0 } else { t.cstr(&text) };
    let id = ids[rng.below(6) as usize];
    let name = if rng.chance(4) {
        0
    } else {
        t.cstr(&format!("c{id}"))
    };
    let unused = rng.next();
    args.copy_from_slice(&[scene, clusters, first, last, name, unused, table_text]);
}

const AAM_STUBS: &[(u32, K)] = &[
    (h6::FREE, K::Cdecl(1)),
    (h6::DELETE_ARRAY, K::Cdecl(1)),
    (h6::WRITE_PARAM, K::This(2)),
];

#[rustfmt::skip]
const CASES_AAM: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_aam), ..method("add_animated_materials", 0x006b_05d0, This::Object, 7, AAM_STUBS, |m, this, a| {
        slots_at(m, a);
        b6::add_animated_materials(m, frame_ebp(), this, a[0], a[1], a[2], a[3], a[4], a[6]);
        Out::None
    }) }), h6::FRAME, 0x1c),
];

#[test]
fn r2_batch1_add_animated_materials_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES_AAM, n, &mut Rng(0xb3b2_0001));
    });
}

// ---- round 2 batch 2: ShadowManager_renderCharacterTiles_006caee0 (ffx_prender::h3b_glue7) ----

use ffx_prender::h3b_glue7::{self as h7, bridge as b7};

/// A caster list `{count, data}`: 1..3 instances (each with a materials
/// word and, for the first, an object with a scale at `+0x14`), now and
/// then none or a count with the top bit.
fn tiles_list(t: &mut Twins, rng: &mut Rng) -> u32 {
    let n = pick(rng, &[0, 1, 2, 3, 0x8000_0002, 0x8000_0000]);
    let list = t.zeros(8);
    let data = t.zeros(0x10);
    t.w32(list, n);
    t.w32(list + 4, data);
    for m in 0..(n & 3).max(1) {
        let instance = t.zeros(0x20);
        let object = t.zeros(0x20);
        let (w, f) = (rng.next(), rng.float().to_bits());
        t.w32(instance + 0x10, w);
        t.w32(instance + 0x1c, object);
        t.w32(object + 0x14, f);
        t.w32(data + 4 * m, instance);
    }
    list
}

/// The shadow manager (cells in use or not, their matrices, scales and
/// depths, the camera with a recording virtual slot 1, the blur mesh's
/// material and parameter buffer), the renderer, the static rectangle
/// list and its guard, the atlas size; renderer calls fail now and then.
#[allow(clippy::too_many_lines)] // the whole set-up
fn prep_tiles(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let sm = t.zeros(0x4100);
    *this = sm;
    let junk = rng.bytes(0x4100);
    t.write(sm, &junk);
    t.w32(sm + 0x4060, u32::from(!rng.chance(8)));
    // cells in use: none, one, or about one in 1..6
    let density = rng.below(8);
    let lone = rng.below(64);
    for k in 0..64 {
        let listed = match density {
            0 => false,
            1 => k == lone,
            d => rng.below(d - 1) == 0,
        };
        let list = if listed { tiles_list(t, rng) } else { 0 };
        t.w32(sm + 0xde8 + 4 * k, list);
        let alt = if rng.chance(3) { tiles_list(t, rng) } else { 0 };
        t.w32(sm + 0xee8 + 4 * k, alt);
        for at in [0xfe8, 0x10e8] {
            let v = if rng.chance(6) { rng.word() } else { 1 };
            t.w32(sm + at + 4 * k, v);
        }
        let b = u8::from(rng.chance(4));
        t.write(sm + 0x2b28 + k, &[b]);
        for at in [0x28e8, 0x6b8] {
            let f = rng.float().to_bits();
            t.w32(sm + at + 4 * k, f);
        }
        let depth = if rng.chance(4) {
            0
        } else {
            rng.float().to_bits()
        };
        t.w32(sm + 0x7b8 + 4 * k, depth);
        let cells = {
            let any = rng.next();
            pick(rng, &[1, 1, 2, 3, 0, 0xffff_ffff, any])
        };
        t.w32(sm + 0x29e8 + 4 * k, cells);
    }
    for w in 0..0x400 {
        let f = rng.float().to_bits();
        t.w32(sm + 0x17e8 + 4 * w, f);
    }
    // the camera: virtual slot 1 records
    let camera = t.zeros(0x200);
    let vtable = t.zeros(8);
    let changed = t.stub(None, 0xca3e_0001, true, 0, 0, Some(rng.next()));
    t.w32(vtable + 4, changed);
    t.w32(camera, vtable);
    t.w32(sm + 0x84, camera);
    let out = t.zeros(0x40);
    t.w32(sm + 0x88, out);
    // the blur mesh: its first material's shader and parameter buffer
    let mesh = t.zeros(0x20);
    let materials = t.zeros(0x10);
    let material = t.zeros(0x10);
    let buffer = t.zeros(0x100);
    t.w32(sm + 0x8b8, mesh);
    t.w32(mesh + 0x10, materials);
    let n = pick(rng, &[1, 2, 0x8000_0001]);
    t.w32(materials, n);
    t.w32(
        materials + 4,
        if n & 3 == 1 { material } else { materials + 8 },
    );
    t.w32(materials + 8, material);
    t.w32(material, rng.next());
    t.w32(material + 4, buffer);
    t.w32(buffer, {
        let any = rng.below(0x100);
        pick(rng, &[0x40, 0x80, 0x10, any])
    });
    for at in [0x58, 0x70] {
        let surface = t.zeros(8);
        let (w, flag) = (rng.next(), u8::from(rng.chance(3)));
        t.w32(surface, w);
        t.write(surface + 4, &[flag]);
        t.w32(sm + at, surface);
    }
    // the renderer
    let renderer = t.zeros(0x200);
    let pass = rng.next();
    t.w32(renderer + 0x13c, pass);
    // the static rectangles and the atlas size
    let rects = t.rebase(h7::RECTS);
    let n = pick(rng, &[0, 1, 2, 3, 0x8000_0000, 0x8000_0002]);
    let data = t.zeros(0x40);
    let words: Vec<u8> = (0..0x40).map(|_| rng.below(0x400) as u8).collect();
    t.write(data, &words);
    t.w32(rects, n);
    t.w32(rects + 4, data);
    let guard = t.rebase(h7::RECTS_GUARD);
    let g = rng.next();
    t.w32(guard, g);
    let size = {
        let any = rng.next();
        pick(rng, &[1024, 1024, 512, 1000, 0xffff_fc00, 7, 0, any])
    };
    t.global(h7::SHADOW_SIZE, size);
    // the parameters: none, or offset/size records
    let records = t.zeros(0x100);
    for k in 0..0x10 {
        let off = pick(rng, &[0, 2, 4, 8, 0x10, 0x30, 0xf0]) as u16;
        let size = pick(rng, &[0, 4, 8, 0x10, 0x10, 0x2010]) as u16;
        t.write(records + 0x10 * k + 8, &off.to_le_bytes());
        t.write(records + 0x10 * k + 0xa, &size.to_le_bytes());
    }
    let values: Vec<u32> = (0..4)
        .map(|_| {
            if rng.chance(4) {
                0
            } else {
                records + 0x10 * rng.below(0x10)
            }
        })
        .collect();
    t.stub_seq(h7::FIND_PARAM, true, 1, 1, &values);
    for (sym, n) in [
        (h7::SET_COLOR_TARGET, 2),
        (h7::SET_DEPTH_TARGET, 1),
        (h7::BEGIN_SCENE, 1),
        (h7::END_SCENE, 1),
        (h7::SET_VIEWPORT, 4),
        (h7::SET_SCISSOR, 4),
        (h7::WRITE_PARAM, 2),
        (h7::SET_VALUE, 2),
    ] {
        // the parameter writes and scene ends (two or three calls) fail
        // more often
        let v = errors_n(
            rng,
            if sym == h7::WRITE_PARAM || sym == h7::END_SCENE {
                6
            } else {
                300
            },
        );
        t.stub_seq(sym, true, n, n, &v);
    }
    let post = t.zeros(0x100);
    let target = t.zeros(8);
    let texture = t.zeros(0x40);
    // the target must be there: the original reads through it unchecked
    let (w, hgt, flag) = (rng.next(), rng.next(), 0u8);
    t.w32(texture + 0x1c, w);
    t.w32(texture + 0x20, hgt);
    t.w32(target, texture);
    t.write(target + 4, &[flag]);
    t.w32(post + 0x8c, target);
    t.stub_ret(h7::POST_PROCESS, false, 0, 0, post);
    let solution = {
        let any = rng.next();
        pick(rng, &[0, 1, 2, 3, 4, any])
    };
    let keep = {
        let any = rng.next();
        pick(rng, &[0, 1, 0x100, any])
    };
    args.copy_from_slice(&[renderer, solution, keep]);
}

const TILES_STUBS: &[(u32, K)] = &[
    (h7::SET_CLEAR_COLOR, K::This(4)),
    (h7::SET_CLEAR_DEPTH, K::This(1)),
    (h7::CLEAR_SCENE, K::This(1)),
    (h7::SET_CAMERA, K::This(1)),
    (h7::RENDER_INSTANCE, K::This(4)),
    (h7::RENDER_FOR_PASS, K::This(1)),
    (h7::UPDATE_VIEW, K::This(0)),
    (h7::PUSH_RECT, K::This(1)),
    (h7::FREE, K::Cdecl(1)),
    (h7::ATEXIT, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES_TILES: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_tiles), ..method("render_character_tiles", 0x006c_aee0, This::Object, 3, TILES_STUBS, |m, this, a| {
        slots_at(m, a);
        u(b7::render_character_tiles(m, frame_ebp(), this, a[0], a[1], a[2] as u8 != 0))
    }) }), h7::FRAME, 0xc),
];

#[test]
fn r2_batch2_render_character_tiles_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES_TILES, n, &mut Rng(0xb3b2_0002));
    });
}
