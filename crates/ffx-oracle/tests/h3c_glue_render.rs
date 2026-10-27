//! The render-side glue's largest functions (M4 port, list
//! `h3-glue-render`, third agent "h3c") against the original: on the image
//! alone with synthetic objects, and on the snapshots (`FFX_SNAPSHOT`).
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The harness is `h3_glue_render.rs`'s (recording stubs in
//! `FUN_006cbf20`'s body, frames at the original's address); the agents
//! keep separate files.

#![allow(
    dead_code, // the harness is h3_glue_render.rs's; not every helper is used yet
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::glue;
use ffx_prender::glue2;

/// The body of `FUN_006cbf20` (2438 bytes; 0 refs in `tools/refs.py`, never
/// reached in the coverage runs, called by no test or hook): the recording
/// stubs are written over it. Until h3c it was `graphicFontDraw`'s, which
/// is ported now and has its own test.
const CAVE: u32 = 0x006c_bf20;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison, not what
/// the previous call returned.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;
/// After the poison: `fld qword [esp+4]; call [esp+0xc]; ret`, and at
/// `+16` `fstp st0; ret`, for [`Host::call_intrinsic`] (a CRT intrinsic
/// with its argument in `ST0`; the cave function is 2438 bytes long).
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

// ---- batch 1: graphicFontDraw (ffx_prender::h3c_font) ----

use ffx_prender::h3c_font::{self as hf, bridge as bf};

/// A float for a glyph field: a PS2 coordinate or uv now, any float (NaN,
/// infinities, huge) now and then.
fn glyph_float(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 => rng.float().to_bits(),
        1 => (rng.below(512) as f32 / 4.0).to_bits(),
        _ => ((rng.below(1 << 16) as f32 - 4096.0) / 64.0).to_bits(),
    }
}

/// `graphicFontDraw`'s record, vertex record, stubs and globals: the
/// skip-frame flag, the sphere grid (absent, below 2 or not), the menu's
/// vertex info (0 for a named texture now and then), counts small, huge
/// and negative (the arrays placed so the original's offsets land in them).
#[allow(clippy::too_many_lines)] // one glyph's world
fn prep_font(t: &mut Twins, _obj: u32, _this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let rec = t.zeros(0x100);
    let mut rb = rng.bytes(0x100);
    for off in [0, 4, 8, 0xc, 0x20, 0x24, 0x28, 0x2c] {
        rb[off..off + 4].copy_from_slice(&glyph_float(rng).to_le_bytes());
    }
    for off in (0x10..0x20).chain(0x30..0x40).step_by(4) {
        let v = if rng.chance(6) {
            rng.word()
        } else {
            rng.below(256)
        };
        rb[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    let kind: u32 = if rng.chance(3) {
        hf::KIND_WIDE
    } else {
        rng.word()
    };
    rb[0x80..0x84].copy_from_slice(&kind.to_le_bytes());
    for off in [0x90, 0x94] {
        rb[off..off + 4].copy_from_slice(&rng.next().to_le_bytes());
    }
    t.write(rec, &rb);

    // the vertex record and its arrays
    let count = match rng.below(6) {
        0 => 0,
        1 => 0x8000_0000 | rng.below(0x1000),
        2 => rng.next(),
        _ => rng.below(200),
    };
    let vr = t.zeros(0x100);
    let vb = rng.bytes(0x100);
    t.write(vr, &vb);
    t.w32(vr + 4, count);
    t.w32(vr + 8, rng.word());
    for (off, stride) in [(0xc, 12), (0x10, 12), (0x14, 16), (0x18, 8)] {
        let buf = t.zeros(0x40);
        t.w32(vr + off, buf.wrapping_sub(count.wrapping_mul(stride)));
    }
    // the index base as the original computes it (ftol of f32(count) * 1.5)
    let nf = ffx_bridge::x87::store(f64::from(count));
    let idx = ffx_bridge::x87::ftol(&t.b, f64::from(f32::from_bits(nf)) * 1.5);
    let buf = t.zeros(0x10);
    t.w32(vr + 0x1c, buf.wrapping_sub(idx.wrapping_mul(2)));

    // the globals
    let skip = if rng.chance(8) {
        pick(rng, &[1, 2, 0x100, 0x8000_0000])
    } else {
        0
    };
    t.global(glue2::GRAPHIC_SKIP_FRAME.sym, skip);
    let abmap = if rng.chance(2) {
        0
    } else {
        let a = t.zeros(0x10);
        t.w32(a + 4, pick(rng, &[0, 1, 2, 3, 0xffff_ffff, 0x8000_0000]));
        a
    };
    t.global(glue2::GRAPHIC_ABMAP.sym, abmap);
    let ui = rng.next();
    t.global(glue2::GRAPHIC_UI.sym, ui);
    let fill = t.zeros(0x10);
    t.global(glue::DYN_GEO_FILL.sym, fill);

    // the arguments: the name (0: "NoTexture"), x, y, the slot
    let name = if rng.chance(4) { 0 } else { rng.next() | 1 };
    let slot = if rng.chance(5) {
        rng.next()
    } else {
        rng.below(8)
    };
    args[0] = rec;
    args[1] = name;
    args[2] = glyph_float(rng);
    args[3] = glyph_float(rng);
    args[4] = slot;

    // the callees
    t.stub_ret(hf::UI_ELEMENT_REMAP, false, 2, 0, rng.next());
    let info = t.zeros(0x100);
    t.w32(info + 0x94, vr);
    let menu_info = if name != 0 && rng.chance(5) { 0 } else { info };
    t.stub_ret(hf::GET_MENU_VERTEX_INFO, true, 2, 2, menu_info);
    let index = if slot.cast_signed() < 4 {
        slot
    } else {
        slot.wrapping_sub(4)
    };
    let ab_info = t.zeros(0x100);
    let records = if rng.chance(8) {
        // a null record
        0u32.wrapping_sub(index.wrapping_mul(hf::ABMAP_RECORD))
    } else {
        vr.wrapping_sub(index.wrapping_mul(hf::ABMAP_RECORD))
    };
    t.w32(ab_info + 0x94, records);
    t.stub_ret(hf::ABMAP_GET_VERTEX_INFO, true, 2, 2, ab_info);
    t.stub_ret(hf::GET_CURRENT_BUFFER_ID, true, 0, 0, rng.below(3));
}

const CASES_FONT: &[Case] = &[Case {
    prep: Some(prep_font),
    ..cdecl("graphicFontDraw", 0x0064_0660, 5, &[], |m, _, a| {
        bf::graphic_font_draw(m, a[0], a[1], a[2], a[3], a[4]);
        Out::None
    })
}];

#[test]
fn batch1_graphic_font_draw_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xb3c1_0001);
        run_cases(t, CASES_FONT, 400, &mut rng);
    });
}

// ---- batch 2: PhyreScene_isVFXTextureLoaded (ffx_prender::h3c_vfxtex) ----

use ffx_prender::h3c_vfxtex::{self as hv, bridge as bv};

/// The maps the special cases test, and others.
const VFX_MAPS: &[u32] = &[
    0x1bfe, 0x1b8a, 0x2416, 0x1b68, 0x1f7, 0x18bd, 0x18cd, 0x11d3, 0x25b0, 0x633, 0x125c, 0,
];
/// The magic ids the special cases and the rules test.
const VFX_MAGIC: &[u32] = &[
    0x21b,
    0x1e7,
    0x2c6,
    0x281,
    0x157,
    0x174,
    0x231,
    0x1f9,
    0xa5,
    0x1f1,
    0x1b0,
    0x1b6,
    0x1b5,
    0x171,
    0xbc,
    0x2a1,
    0x296,
    0xfd,
    0xa8,
    0x24b,
    0xaa,
    0x25f,
    0x9a,
    0xa2,
    0x24a,
    0x130,
    0x238,
    0x94,
    0x239,
    0xd0,
    0x1ae,
    0x24c,
    0x1ff,
    0x24d,
    0x46,
    0x23a,
    0x133,
    0x233,
    0x203,
    0x29b,
    0x263,
    0x29c,
    0x18d,
    0x15f,
    0x207,
    0x280,
    0x241,
    0x246,
    0xffff_ffff,
    7,
];
/// The texture addresses they test.
const VFX_ADDRS: &[u32] = &[
    0x3600, 0x3601, 0x3f00, 0x3ac0, 0x37c0, 0x3980, 0x3900, 0x3340, 0x3360, 0x3700, 0x3400, 0x3820,
    0x3800, 0x3640, 0x3420, 0x3960, 0x39a0, 0x39c0, 0x3480, 0x38c0, 0x3100, 0x2e00, 0x3680, 0x3500,
    0x3880, 0x3740, 0x3840, 0x3b00, 0x38a0, 0x3580, 0x34c0, 0x3540, 0x3a00, 0x36e0, 0x36c0, 0x3a80,
    0x3520, 0x35a0, 0x3620, 0x3440, 0x3940, 0x1234,
];
const VFX_SIZES: &[u32] = &[0x80, 0x20, 0x40, 0x100, 0x200, 4, 0x10];
const VFX_PHASES: &[u32] = &[0, 1, 2, 3, 4, 5, 8, 0x10, 0x20, 0x40, 0x7f, 0xffff_ffff];

thread_local! {
    /// [`prep_vfx`]'s scene: whether on a snapshot, its address.
    static VFX_SCENE: Cell<Option<(bool, u32)>> = const { Cell::new(None) };
}

/// A scene with a texture list whose entries mostly match the key (so the
/// rules' counts run over several), the cache sometimes holding the key,
/// the effect state from the pools above.
#[allow(clippy::too_many_lines)] // one scene
fn prep_vfx(t: &mut Twins, _obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    // one scene per oracle pair (64 KB each would run the arena out),
    // cleared where the original reads
    let scene = match VFX_SCENE.get() {
        Some((snap, at)) if snap == t.snapshot => at,
        _ => {
            let at = t.zeros(0x1_0000);
            VFX_SCENE.set(Some((t.snapshot, at)));
            at
        }
    };
    t.write(scene, &[0; 0x80]);
    t.write(scene + 0xfaf0, &[0; 0x180]);
    *this = scene;

    // the state: from the pools, or one special case's (then one word
    // off now and then)
    let mut p2 = pick(rng, VFX_ADDRS);
    let mut p3 = if rng.chance(6) { rng.word() } else { 0x13 };
    let mut p6 = pick(rng, VFX_SIZES);
    let mut p7 = pick(rng, VFX_SIZES);
    let mut map = pick(rng, VFX_MAPS);
    let mut magic = pick(rng, VFX_MAGIC);
    let mut handle = if rng.chance(5) {
        0xffff_ffff
    } else {
        rng.below(4)
    };
    let mut pass = u32::from(rng.chance(2));
    let mut yiq = if rng.chance(2) { 8 } else { rng.word() };
    let mut abmap = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let mut phase = pick(rng, VFX_PHASES);
    let scenario = rng.below(32);
    if scenario < 16 {
        // an effect with phase rules and one of the addresses they test
        let (magics, addrs) = VFX_RULES[rng.below(VFX_RULES.len() as u32) as usize];
        magic = pick(rng, magics);
        p2 = pick(rng, addrs);
        handle = 0;
        phase = pick(rng, &[0, 1, 2, 2, 3, 4, 4, 8, 8, 0x10, 0x20, 0x40, 0x7f]);
    } else if scenario < 18 {
        // the address 0x3600 read as 0x3601
        (map, pass, magic, p2) = (0x1bfe, 1, 0x21b, 0x3600);
    } else if scenario < 25 {
        let (kind, v, addr, w, h) =
            VFX_SPECIAL_INPUTS[rng.below(VFX_SPECIAL_INPUTS.len() as u32) as usize];
        p2 = addr;
        p3 = 0x13;
        if w != 0 {
            (p6, p7) = (w, h);
        }
        match kind {
            0 => map = v,
            1 => (map, pass) = (v, 1),
            2 => (map, yiq) = (v, 8),
            3 => (magic, handle) = (v, 0),
            _ => abmap = 0x40,
        }
        match rng.below(12) {
            0 => p3 = 0x12,
            1 => p6 ^= 0x10,
            2 => p7 ^= 0x10,
            3 => handle = 0xffff_ffff,
            4 => yiq = 7,
            5 => pass = 0,
            6 => map ^= 1,
            7 => abmap = 0,
            _ => {}
        }
    }
    if rng.chance(6) {
        p2 |= rng.next() << 16;
    }
    let p4 = rng.below(3);
    let p5 = rng.below(3);
    let out = t.zeros(0x100);
    for (k, v) in [p2, p3, p4, p5, p6, p7, out].into_iter().enumerate() {
        args[k] = v;
    }
    t.w32(scene + 0xfaf4, map);
    if rng.chance(6) {
        t.write(scene + 0xfc00, &[1]);
        let v = match rng.below(3) {
            0 => p2 & 0xffff,
            1 => (p2 & 0xffff) | (rng.next() << 16),
            _ => rng.next(),
        };
        t.w32(scene + 0xfc64, v);
    }

    // the list: most entries match the key, so the rules count over
    // several
    let n = if scenario < 16 {
        2 + rng.below(5)
    } else {
        rng.below(7)
    };
    t.w32(scene + 0x34, n);
    let names = t.zeros(4 * 8);
    let recs = t.zeros(0x14 * 8);
    let paths = t.zeros(4 * 8);
    t.w32(scene + 0x3c, names);
    t.w32(scene + 0x48, recs);
    t.w32(scene + 0x54, paths);
    let empty = t.alloc(&[0]);
    for i in 0..n {
        let name = match rng.below(12) {
            0 => 0,
            1 => empty,
            _ => t.alloc(b"name\0"),
        };
        t.w32(names + 4 * i, name);
        let path = t.alloc(format!("path{i}_{:x}\0", rng.next()).as_bytes());
        t.w32(paths + 4 * i, path);
        let r = recs + 0x14 * i;
        let kind = if rng.chance(4) { rng.below(3) } else { 3 };
        t.w32(r, kind);
        let addr = if rng.chance(8) {
            pick(rng, VFX_ADDRS)
        } else {
            p2 & 0xffff
        };
        t.w32(r + 4, addr);
        let fmt = if rng.chance(8) { rng.next() } else { p3 };
        t.write(r + 8, &fmt.to_le_bytes()[..2]);
        if rng.chance(4) {
            t.write(r + 0xa, &[1, 2]);
        }
        for (off, v) in [(0xe, p6), (0x10, p7)] {
            let v = if rng.chance(4) {
                pick(rng, VFX_SIZES)
            } else {
                v
            };
            t.write(r + off, &v.to_le_bytes()[..2]);
        }
    }
    // the cache: the key now and then
    if rng.chance(4) {
        let key = [p2, p3, p4, p5, p6, p7, rng.below(n + 2)];
        for (k, v) in key.into_iter().enumerate() {
            let at = scene + 0x64 + 2 * k as u32;
            let v = if rng.chance(8) { v ^ 1 } else { v };
            t.write(at, &v.to_le_bytes()[..2]);
        }
    }

    // the effect state
    t.global(hv::VFX_PASS, pass);
    t.global(hv::VFX_YIQ_MODE, yiq);
    t.global(hv::ET_CHR_HIDDEN, u32::from(rng.chance(3)));
    t.global(hv::VFX_PHASE, phase);
    t.global(hv::GRAPHIC_ABMAP, abmap);

    // the callees
    t.stub_ret(hv::GET_MAGIC_ID, false, 0, 0, magic);
    t.stub_ret(hv::IS_NEX_DATA, false, 0, 0, pick(rng, &[0, 1, 1, 2]));
    t.stub_ret(hv::GET_HANDLE, false, 0, 0, handle);
    let deleting = if rng.chance(3) {
        rng.below(4)
    } else {
        0xffff_ffff
    };
    t.stub_ret(hv::GET_TO_BE_DELETE, false, 0, 0, deleting);
    t.stub_ret(hv::GET_WORD0, false, 0, 0, u32::from(rng.chance(2)));
    t.stub_ret(hv::UNIFY_FILENAME, false, 3, 0, rng.next());
    for (slot, n, ret) in [
        (hv::SPRINTF, 3, rng.below(12)),
        (
            hv::STRSTR,
            2,
            if rng.chance(4) { 0 } else { rng.next() | 1 },
        ),
    ] {
        let s = t.stub(None, slot, false, n, 0, Some(ret));
        let at = t.rebase(slot);
        t.patch(at, &s.to_le_bytes());
    }
}

/// The effects with phase rules and the addresses each tests (from the
/// original's code at 0x00661326-0x00661c23).
const VFX_RULES: &[(&[u32], &[u32])] = &[
    (&[0xa8, 0x24b], &[0x3600, 0x3680, 0x3700, 0x3500]),
    (&[0xaa, 0x25f], &[0x3400, 0x3ac0, 0x3880]),
    (&[0x9a], &[0x3740, 0x3480, 0x3840, 0x3880, 0x3b00, 0x3400]),
    (&[0xa2, 0x24a, 0x18d, 0x15f, 0x246], &[0x3ac0, 0x3400]),
    (&[0x130, 0x238], &[0x3400, 0x39a0, 0x38a0]),
    (&[0x94, 0x239, 0xd0], &[0x3580, 0x34c0, 0x3500, 0x3540]),
    (&[0x1ae, 0x24c], &[0x3740, 0x3400]),
    (&[0x1ff, 0x24d], &[0x3540, 0x3640, 0x3840]),
    (&[0x46, 0x23a], &[0x3400, 0x3500]),
    (&[0x133, 0x233], &[0x3a00, 0x3800, 0x3600, 0x3400]),
    (
        &[0x203, 0x29b, 0x263, 0x29c],
        &[0x3680, 0x3980, 0x36e0, 0x36c0, 0x3a80, 0x3400],
    ),
    (
        &[0x207],
        &[
            0x3400, 0x3500, 0x3520, 0x35a0, 0x3620, 0x3640, 0x36c0, 0x3700, 0x38a0, 0x38c0, 0x3800,
        ],
    ),
    (&[0x280], &[0x3440, 0x3980, 0x36e0, 0x3400]),
    (&[0x241], &[0x3700, 0x3940, 0x3400, 0x34c0]),
    (&[0x2a1], &[0x38c0, 0x3400]),
    (&[0x296], &[0x3100, 0x2e00]),
    (&[0xfd], &[0x3400]),
];

const CASES_VFX: &[Case] = &[with_cookie(framed(
    Case {
        prep: Some(prep_vfx),
        ..method(
            "PhyreScene_isVFXTextureLoaded",
            0x0066_04d0,
            This::Object,
            7,
            &[],
            |m, this, a| {
                u(bv::is_vfx_texture_loaded(
                    m,
                    frame_ebp(),
                    this,
                    a[0],
                    a[1],
                    a[2],
                    a[3],
                    a[4],
                    a[5],
                    a[6],
                ))
            },
        )
    },
    hv::NUMBER,
    0,
))];

#[test]
fn batch2_is_vfx_texture_loaded_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xb3c2_0001);
        run_cases(t, CASES_VFX, 8000, &mut rng);
    });
}

// ---- batch 3: PhyreScene_isVFXTextureLoaded_00661c80 (ffx_prender::h3c_vfxname) ----

use ffx_prender::h3c_vfxname::{self as hn, bridge as bn};

thread_local! {
    /// [`prep_vfx_name`]'s scene: whether on a snapshot, its address.
    static VFXN_SCENE: Cell<Option<(bool, u32)>> = const { Cell::new(None) };
}

impl Twins {
    /// The C string at `at` (an image address, rebased).
    fn image_str(&self, at: u32) -> Vec<u8> {
        let at = self.rebase(at);
        let mut v = Vec::new();
        loop {
            let c: u8 = Mem::read(&self.b, at + v.len() as u32);
            if c == 0 {
                return v;
            }
            v.push(c);
        }
    }

    /// A stub at `sym` that runs `code` (which must not touch the
    /// arguments' stack) and then logs and returns as [`Twins::stub`].
    fn stub_then(&mut self, sym: u32, code: &[u8], n: u32, ret: u32) {
        let log = self.stub(None, sym, false, n, 0, Some(ret));
        let mut c = code.to_vec();
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

/// `strncpy(dst, src, n)`: copies up to `n` bytes, pads with zeros;
/// returns `dst`.
const STRNCPY_CODE: [u8; 34] = [
    0x56, 0x57, 0x8b, 0x7c, 0x24, 0x0c, 0x8b, 0x74, 0x24, 0x10, 0x8b, 0x4c, 0x24,
    0x14, // loads
    0x85, 0xc9, 0x74, 0x09, 0xac, 0xaa, 0x49, 0x84, 0xc0, 0x75, 0xf5, 0xf3, 0xaa, // copy, pad
    0x8b, 0x44, 0x24, 0x0c, 0x5f, 0x5e, 0xc3, // eax = dst; pop edi; pop esi; ret
];

/// The data root `fiosUnifyFilename` writes in the tests: "dat/".
const ROOT_DIR: &[u8] = b"dat/";

#[allow(clippy::too_many_lines)] // one scene
fn prep_vfx_name(t: &mut Twins, _obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = match VFXN_SCENE.get() {
        Some((snap, at)) if snap == t.snapshot => at,
        _ => {
            let at = t.zeros(0x1_0000);
            VFXN_SCENE.set(Some((t.snapshot, at)));
            at
        }
    };
    t.write(scene, &[0; 0x80]);
    t.write(scene + 0xfaf0, &[0; 0x200]);
    *this = scene;

    // the state and the name: from the pools, a special case's or a rule
    // key's
    let mut map = pick(rng, &[0x1b68, 0x18bd, 0x1346, 0x633, 0x125c, 0x199, 0]);
    let mut magic = pick(rng, VFX_MAGIC);
    let mut handle = if rng.chance(5) {
        0xffff_ffff
    } else {
        rng.below(4)
    };
    let mut pass = u32::from(rng.chance(2));
    let mut lang = if rng.chance(2) { 0xb } else { 1 };
    let mut abmap = if rng.chance(2) { 0 } else { 0x40 };
    let mut phase = pick(rng, VFX_PHASES);
    let tails: &[&[u8]] = &[
        b"",
        b"_0_0_128_64.dds.phyre",
        b"_0_0_512_256.dds.phyre",
        b"x",
    ];
    let mut name: Vec<u8> = match rng.below(5) {
        0 => b"trxtex01_19".to_vec(),
        4 => vec![],
        1 => b"13312_19_0_0_512_256.dds.phyre".to_vec(),
        2 => b"1234".to_vec(),
        _ => b"14528_19_0_0_128_64.dds.phyre".to_vec(),
    };
    let scenario = rng.below(32);
    if scenario < 14 {
        let (magics, addrs) = VFX_RULES[rng.below(VFX_RULES.len() as u32) as usize];
        magic = pick(rng, magics);
        let p = pick(rng, addrs) as u16;
        handle = 0;
        phase = pick(rng, &[0, 1, 2, 2, 3, 4, 4, 8, 8, 0x10, 0x20, 0x40, 0x7f]);
        if let Some(&(at, _)) = hn::KEYS.iter().find(|&&(_, k)| k == p) {
            name = t.image_str(at);
            name.extend_from_slice(pick_bytes(rng, tails));
        }
    } else if scenario < 26 {
        let (kind, v, string, starts) =
            VFXN_SPECIALS[rng.below(VFXN_SPECIALS.len() as u32) as usize];
        match kind {
            0 => map = v,
            1 => (map, pass) = (v, 1),
            2 => (map, lang) = (v, 0xb),
            3 => (magic, handle) = (v, 0),
            _ => abmap = 0x40,
        }
        name = t.image_str(string);
        if starts {
            name.extend_from_slice(pick_bytes(rng, tails));
        }
        match rng.below(10) {
            0 => name.push(b'z'),
            1 => handle = 0xffff_ffff,
            2 => lang = 3,
            3 => pass = 0,
            4 => map ^= 1,
            5 => abmap = 0,
            _ => {}
        }
    }
    name.push(0);
    let name_at = t.alloc(&name);
    let out = t.zeros(0x100);
    args[0] = name_at;
    args[1] = out;
    for a in &mut args[2..] {
        *a = rng.word();
    }
    t.w32(scene + 0xfaf4, map);
    if rng.chance(5) {
        t.write(scene + 0xfc00, &[1]);
        let n = name.len() - 1;
        let rt: Vec<u8> = match rng.below(4) {
            0 => vec![],
            1 => name[..n.min(5)].to_vec(),
            2 => name[..n].to_vec(),
            _ => b"RTT0".to_vec(),
        };
        t.write(scene + 0xfc68, &rt);
    }

    // the list: entries whose first eight characters are the name's
    let n = if scenario < 14 {
        2 + rng.below(5)
    } else {
        rng.below(7)
    };
    t.w32(scene + 0x34, n);
    let names = t.zeros(4 * 8);
    let paths = t.zeros(4 * 8);
    t.w32(scene + 0x3c, names);
    t.w32(scene + 0x54, paths);
    let empty = t.alloc(&[0]);
    for i in 0..n {
        let entry = match rng.below(12) {
            0 => 0,
            1 => empty,
            2 => t.alloc(b"other_19_0_0.dds\0"),
            _ => {
                let mut e = name[..(name.len() - 1).min(8)].to_vec();
                if rng.chance(2) {
                    e.extend_from_slice(&name[e.len()..name.len() - 1]);
                } else {
                    e.extend_from_slice(b"_9_9.dds");
                }
                e.push(0);
                t.alloc(&e)
            }
        };
        t.w32(names + 4 * i, entry);
        let dir = pick_bytes(
            rng,
            &[b"magic/", b"map/", b"btl/", b"yo/", b"chr/", b"ma", b""],
        );
        let mut p = ROOT_DIR.to_vec();
        p.extend_from_slice(dir);
        p.extend_from_slice(format!("t{i}.dds\0").as_bytes());
        let path = t.alloc(&p);
        t.w32(paths + 4 * i, path);
    }

    t.global(hn::VFX_PASS, pass);
    t.global(hn::ET_CHR_HIDDEN, u32::from(rng.chance(3)));
    t.global(hn::VFX_PHASE, phase);
    t.global(hn::GRAPHIC_ABMAP, abmap);

    // the callees
    t.stub_ret(hn::GET_MAGIC_ID, false, 0, 0, magic);
    t.stub_ret(hn::GET_HANDLE, false, 0, 0, handle);
    let deleting = if rng.chance(3) {
        rng.below(4)
    } else {
        0xffff_ffff
    };
    t.stub_ret(hn::GET_TO_BE_DELETE, false, 0, 0, deleting);
    t.stub_ret(hn::GET_LANG, false, 0, 0, lang);
    // fiosUnifyFilename writes the root ("dat/") into its output
    let mut unify = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
    unify.extend([0xc7, 0x00]);
    unify.extend(u32::from_le_bytes(*b"dat/").to_le_bytes()); // mov [eax], "dat/"
    unify.extend([0xc6, 0x40, 0x04, 0x00]); // mov byte [eax+4], 0
    t.stub_then(hn::UNIFY_FILENAME, &unify, 3, rng.next());
    let strncpy = t.code(&STRNCPY_CODE);
    let site = t.rebase(hn::STRNCPY);
    t.patch(site, &strncpy.to_le_bytes());
    let strstr_ret = if rng.chance(3) { 0 } else { rng.next() | 1 };
    for (slot, n, ret) in [(hn::SPRINTF, 3, rng.below(12)), (hn::STRSTR, 2, strstr_ret)] {
        let s = t.stub(None, slot, false, n, 0, Some(ret));
        let at = t.rebase(slot);
        t.patch(at, &s.to_le_bytes());
    }
}

fn pick_bytes<'a>(rng: &mut Rng, vs: &[&'a [u8]]) -> &'a [u8] {
    vs[rng.below(vs.len() as u32) as usize]
}

const CASES_VFX_NAME: &[Case] = &[with_cookie(framed(
    Case {
        prep: Some(prep_vfx_name),
        ..method(
            "PhyreScene_isVFXTextureLoaded_00661c80",
            0x0066_1c80,
            This::Object,
            6,
            &[],
            |m, this, a| u(bn::is_vfx_texture_loaded(m, frame_ebp(), this, a[0], a[1])),
        )
    },
    hn::ROOT,
    0,
))];

#[test]
fn batch3_is_vfx_texture_loaded_by_name_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0xb3c3_0001);
        run_cases(t, CASES_VFX_NAME, 8000, &mut rng);
    });
}

/// The special cases' inputs (gate kind: 0 map, 1 pass and map, 2 map
/// with YIQ mode 8, 3 magic, 4 sphere grid; its value; the address; the
/// size, 0 when not tested), from the original's code at
/// 0x006605aa-0x006611d7, kept apart from the port's table.
const VFX_SPECIAL_INPUTS: &[(u8, u32, u32, u32, u32)] = &[
    (0, 0x1b8a, 0x3f00, 0, 0),
    (0, 0x2416, 0x3ac0, 0, 0),
    (0, 0x1b68, 0x37c0, 0x80, 0x20),
    (0, 0x1f7, 0x3ac0, 0x80, 0x80),
    (0, 0x1b68, 0x37c0, 0x80, 0x40),
    (3, 0x1e7, 0x3980, 0, 0),
    (3, 0x1e7, 0x3ac0, 0, 0),
    (3, 0x2c6, 0x3900, 0, 0),
    (0, 0x18bd, 0x3ac0, 0, 0),
    (3, 0x281, 0x3600, 0x200, 0x100),
    (0, 0x18cd, 0x3340, 0x80, 0x40),
    (0, 0x18cd, 0x3360, 0x80, 0x40),
    (2, 0x11d3, 0x3700, 0x80, 0x80),
    (3, 0x157, 0x3400, 0x100, 0x200),
    (3, 0x174, 0x3ac0, 0, 0),
    (3, 0x281, 0x3820, 0x200, 0x100),
    (0, 0x25b0, 0x3ac0, 0x80, 0x80),
    (3, 0x231, 0x3800, 0, 0),
    (3, 0x231, 0x3800, 0x80, 0x40),
    (3, 0x1f9, 0x3800, 0, 0),
    (1, 0x633, 0x3ac0, 0x80, 0x80),
    (0, 0x125c, 0x3640, 0x80, 4),
    (3, 0xa5, 0x3400, 0x80, 4),
    (3, 0xa5, 0x3420, 0x80, 4),
    (4, 0, 0x3960, 0x80, 0x80),
    (4, 0, 0x39a0, 0x80, 4),
    (4, 0, 0x39c0, 0x80, 4),
    (3, 0x1f1, 0x3ac0, 0x80, 0x100),
    (3, 0x1f9, 0x3980, 0x80, 0x40),
    (3, 0x1b0, 0x3ac0, 0, 0),
    (3, 0x1b6, 0x3ac0, 0x80, 4),
    (3, 0x1b5, 0x3ac0, 0x80, 4),
    (3, 0x171, 0x3480, 0x80, 4),
    (3, 0x171, 0x3480, 0x200, 0x100),
    (3, 0xbc, 0x3ac0, 0, 0),
];

/// The by-name special cases' inputs (gate kind: 0 map, 1 pass and map,
/// 2 language and map, 3 magic, 4 sphere grid; its value; the string;
/// whether the name only starts with it), from the original's code at
/// 0x00661cd2-0x0066267d, kept apart from the port's table.
const VFXN_SPECIALS: &[(u8, u32, u32, bool)] = &[
    (0, 0x1b68, 0x00b4_36d0, false),
    (0, 0x1b68, 0x00b4_36f0, false),
    (3, 0x1e7, 0x00b4_3710, true),
    (3, 0x1e7, 0x00b4_371c, true),
    (0, 0x18bd, 0x00b4_371c, true),
    (3, 0x281, 0x00b4_3728, false),
    (3, 0x281, 0x00b4_3748, false),
    (0, 0x1346, 0x00b4_3768, false),
    (3, 0x231, 0x00b4_3788, true),
    (3, 0x231, 0x00b4_3794, false),
    (3, 0x1f9, 0x00b4_3788, true),
    (1, 0x633, 0x00b4_3768, false),
    (0, 0x125c, 0x00b4_37b4, false),
    (2, 0x199, 0x00b4_37d4, false),
    (3, 0xa5, 0x00b4_3844, false),
    (3, 0xa5, 0x00b4_3864, false),
    (4, 0, 0x00b4_3884, false),
    (4, 0, 0x00b4_38a4, false),
    (4, 0, 0x00b4_38c4, false),
    (3, 0x1f1, 0x00b4_38e4, false),
    (3, 0x1f9, 0x00b4_3904, false),
    (3, 0x1b0, 0x00b4_371c, true),
    (3, 0x1b6, 0x00b4_3924, false),
    (3, 0x1b5, 0x00b4_3924, false),
    (3, 0xbc, 0x00b4_371c, true),
];
