//! `PhyreScene_render` (`0x00668930`; M4 port, list `h3-glue-render`,
//! agent "h3e") against the original, one test per stage: on the image
//! alone with a synthetic scene, and on the snapshots (`FFX_SNAPSHOT`,
//! the game's scene with its flags varied). Results, every byte written
//! and the frame callees see must agree. Needs `FFX_GAME_DIR`.
//!
//! The harness is `h3c_glue_render.rs`'s (frames at the original's
//! address; the poison and the intrinsic trampolines in `FUN_006cbf20`'s
//! body), but the recording stubs live in code the oracles allocate: the
//! function has about 110 callees, more than the cave holds.

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
use ffx_prender::h3e_util as u3;

/// The body of `FUN_006cbf20` (2438 bytes; 0 refs in `tools/refs.py`, never
/// reached in the coverage runs, called by no test or hook): the recording
/// stubs are written over it. Until h3c it was `graphicFontDraw`'s, which
/// is ported now and has its own test.
const CAVE: u32 = 0x006c_bf20;
const CAVE_LEN: u32 = 1760;
/// The stubs' code area (allocated in both oracles alike).
const STUB_LEN: u32 = 0x2_0000;
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
        let cave = a.alloc_code(&vec![0xcc; STUB_LEN as usize]).unwrap();
        assert_eq!(b.alloc_code(&vec![0xcc; STUB_LEN as usize]).unwrap(), cave);
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
        assert!(addr + c.len() as u32 <= self.cave + STUB_LEN, "stubs full");
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
                for k in 0..n.min(1024) {
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
                // the last calls (the log holds 1024)
                for k in n.saturating_sub(24)..n.min(1024) {
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
            addr + code.len() as u32 <= self.cave + STUB_LEN,
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

// ---- PhyreScene_render (ffx_prender::h3e_*) ----

use ffx_prender::h3e_render::bridge as hr;

/// A float, mostly a moderate one, now and then any (NaN, infinities).
fn mild(rng: &mut Rng) -> u32 {
    if rng.chance(16) {
        rng.float().to_bits()
    } else {
        ((rng.below(4000) as f32 - 2000.0) / 100.0).to_bits()
    }
}

/// An `al` answer (0 or not) with stray high bits.
fn answer(rng: &mut Rng) -> u32 {
    let al = pick(rng, &[0, 1, 0x80]);
    (rng.next() & !0xff) | al
}

/// Which stage a test steers the scene into.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    /// The sphere grid's frame.
    Abmap,
    /// The movie frame, a held FMV, the pause word.
    Video,
    /// `PhyreScene_check1195` 0 or a movie fading: no 3D scene; the late
    /// stage, the UI and the present.
    Late,
    /// The 3D scene and everything after it.
    Scene,
}

thread_local! {
    static FOCUS: Cell<Focus> = const { Cell::new(Focus::Scene) };
    /// One in this many renderer calls fails.
    static FAIL_ONE_IN: Cell<u32> = const { Cell::new(200) };
}

/// The renderer's answers for 64 calls: 0, an error one in [`FAIL_ONE_IN`].
fn renderer_results(rng: &mut Rng) -> Vec<u32> {
    let n = FAIL_ONE_IN.with(Cell::get);
    (0..64)
        .map(|_| if rng.chance(n) { rng.word() | 1 } else { 0 })
        .collect()
}

fn answers(rng: &mut Rng, n: usize) -> Vec<u32> {
    (0..n).map(|_| answer(rng)).collect()
}

/// A flag byte: 0 half the time, else 1 or a high bit.
fn flag(rng: &mut Rng) -> u8 {
    pick(rng, &[0, 0, 1, 0x80]) as u8
}

/// A target slot `{texture, gone byte}` with its texture (sizes at `+0x1c`
/// and `+0x20`), the texture missing now and then.
fn slot(t: &mut Twins, rng: &mut Rng) -> u32 {
    let at = t.zeros(0x40);
    let texture = at + 0x10;
    t.w32(at, texture);
    let gone = if rng.chance(6) { flag(rng) | 1 } else { 0 };
    t.write(at + 4, &[gone, rng.next() as u8]);
    let (w, hgt) = (rng.word(), rng.word());
    t.w32(texture + 0x1c, w);
    t.w32(texture + 0x20, hgt);
    at
}

fn stub_this(t: &mut Twins, sym: u32, n: u32, ret: u32) {
    t.stub_ret(sym, true, n, n, ret);
}

fn stub_cdecl(t: &mut Twins, sym: u32, n: u32, ret: u32) {
    t.stub_ret(sym, false, n, 0, ret);
}

/// The post-process manager: its targets (`+0x8c..+0xac`, `+0x1232c`),
/// the swap word `+0x12408` (one of them or another), the cross-fade,
/// afterimage and fade bytes, the shorts `+0x123bc`, `+0x123c0` point at.
fn post_manager(t: &mut Twins, rng: &mut Rng) -> u32 {
    let pp = t.zeros(0x1_2500);
    let mut slots = vec![];
    for off in [0x8c, 0x90, 0x94, 0x9c, 0xa4, 0xa8, 0xac, 0x1_232c] {
        let s = slot(t, rng);
        slots.push(s);
        t.w32(pp + off, s);
    }
    let swap = if rng.chance(3) {
        rng.next()
    } else {
        pick(rng, &slots)
    };
    t.w32(pp + 0x1_2408, swap);
    for off in [0x123a0, 0x123bb, 0x123c4] {
        let b = flag(rng);
        t.write(pp + off, &[b]);
    }
    let b = pick(rng, &[0, 1, 1, 2]) as u8;
    t.write(pp + 0x123ba, &[b]);
    let shorts = t.zeros(8);
    let any = rng.next() & 0xffff;
    let n = pick(rng, &[0, 1, 0x80, 0x7fff, 0x8000, 0xffff, any]);
    t.w32(shorts, n);
    let m = pick(rng, &[0, 0, 1, 0x1_0000]);
    t.w32(shorts + 4, m);
    t.w32(pp + 0x123bc, shorts);
    t.w32(pp + 0x123c0, shorts + 4);
    pp
}

/// A camera: a recording virtual `+4`, a matrix at `+0x34`, `+0xb8`,
/// `+0x150`, `+0x154`.
fn camera(t: &mut Twins, rng: &mut Rng) -> u32 {
    let cam = t.zeros(0x400);
    let vtable = cam + 0x300;
    t.w32(cam, vtable);
    let ret = rng.next();
    let s = t.stub(None, 0xcafe_0004, true, 0, 0, Some(ret));
    t.w32(vtable + 4, s);
    for k in 0..16 {
        let v = mild(rng);
        t.w32(cam + 0x34 + 4 * k, v);
    }
    for off in [0xb8, 0x150, 0x154] {
        let v = mild(rng);
        t.w32(cam + off, v);
    }
    cam
}

/// The free camera: active by its bytes `+0x338`/`+0x33c` or a positive
/// first word, or idle.
fn free_camera(t: &mut Twins, rng: &mut Rng) -> u32 {
    let fc = t.zeros(0x400);
    let b = rng.bytes(0x400);
    t.write(fc, &b);
    let first = pick(rng, &[0, 1, u32::MAX, 0x8000_0000, 5]);
    t.w32(fc, first);
    for off in [0x338, 0x33c] {
        let b = if rng.chance(3) { flag(rng) } else { 0 };
        t.write(fc + off, &[b]);
    }
    for off in [0x15c, 0x160, 0x2b4, 0x2b8] {
        let v = mild(rng);
        t.w32(fc + off, v);
    }
    fc
}

/// The scene's flag bytes and words, for `focus`.
fn scene_flags(t: &mut Twins, scene: u32, focus: Focus, rng: &mut Rng) {
    for off in [
        0, 1, 2, 0x78, 0x93c, 0x93d, 0x93e, 0xd4, 0xf9c5, 0xf9c7, 0xf9c8, 0xfb44, 0xfb45, 0x1195,
    ] {
        let b = flag(rng);
        t.write(scene + off, &[b]);
    }
    let b = pick(rng, &[0, 1, 1, 2, 0x81]) as u8;
    t.write(scene + 3, &[b]);
    let held = if focus == Focus::Video { flag(rng) } else { 0 };
    t.write(scene + 0x1_0d2e, &[held]);
    for off in [0x3854, 0x3860] {
        let v = pick(rng, &[0, 0, 1, 0x8000_0000]);
        t.w32(scene + off, v);
    }
    let v = pick(rng, &[0, 0, 1, 2, u32::MAX, 0x8000_0000]);
    t.w32(scene + 0x388c, v);
    let v = pick(rng, &[0, 0, 1]);
    t.w32(scene + 0xf9cc, v);
    let v = pick(rng, &[0, 1, 1, 2]);
    t.w32(scene + 0x1_0d8c, v);
    let v = rng.word();
    t.w32(scene + 0x1_0dd8, v);
    for k in 0..4 {
        let v = rng.word();
        t.w32(scene + 0x1_0d40 + 4 * k, v);
    }
    for off in [0xfa34, 0xfa38] {
        let v = mild(rng);
        t.w32(scene + off, v);
    }
}

/// A synthetic scene: the context's clear colour, the renderer, the
/// camera (none now and then), the other cameras, the `+0x7e0` float,
/// the post-process manager, the skip prompt.
fn synthetic_scene(t: &mut Twins, obj: u32, rng: &mut Rng) -> u32 {
    let scene = t.zeros(0x1_1000);
    let ctx = t.zeros(0x100);
    for off in [0x30, 0x34, 0x38] {
        let v = mild(rng);
        t.w32(ctx + off, v);
    }
    t.w32(scene + 0xcc, ctx);
    t.w32(scene + 0xd0, obj + 0x1000);
    let cam = if rng.chance(16) { 0 } else { camera(t, rng) };
    t.w32(scene + 0xdc, cam);
    for off in [0x394, 0x238, 0x654] {
        let c = if off == 0x238 && rng.chance(3) {
            0
        } else {
            t.zeros(0x100)
        };
        t.w32(scene + off, c);
    }
    let src = t.zeros(0x100);
    let v = mild(rng);
    t.w32(src + 0xb8, v);
    t.w32(scene + 0x7e0, src);
    let pp = post_manager(t, rng);
    t.w32(scene + 0x10cc, pp);
    t.w32(scene + 0x1_0de0, obj + 0x1100);
    scene
}

/// The globals `PhyreScene_render` reads, for `focus`.
fn render_globals(t: &mut Twins, obj: u32, focus: Focus, rng: &mut Rng) {
    use u3::*;
    let fc = if rng.chance(8) {
        0
    } else {
        free_camera(t, rng)
    };
    t.global(FREE_CAMERA, fc);
    let abmap = t.zeros(0x10);
    let state = match focus {
        Focus::Abmap => pick(rng, &[0, 1, 2, 3, u32::MAX, 0x8000_0000]),
        _ => pick(rng, &[4, 5, 0x7fff_ffff]),
    };
    t.w32(abmap + 4, state);
    let none = focus != Focus::Abmap && rng.chance(2);
    t.global(GRAPHIC_ABMAP, if none { 0 } else { abmap });
    let ui = if rng.chance(3) { 0 } else { obj + 0x1200 };
    t.global(GRAPHIC_UI, ui);
    t.global(DELEGATION_HELPER, obj + 0x1300);
    t.global(IGGY_MENU_MANAGER, obj + 0x1400);
    let screen = t.zeros(0x20);
    let v = pick(rng, &[0, 2, 3, 3]);
    t.w32(screen + 0x10, v);
    t.global(LOADING_SCREEN, screen);
    for (sym, vs) in [
        (MAP_163_OR_164, &[0, 0, 1][..]),
        (WORD_CEC19C, &[0, 0, 1]),
        (MOVIE_PICTURE_ON, &[0, 0, 1]),
        (WORD_1136FC0, &[0, 1, 2]),
        (OPUS_WAVE_OMD_END, &[0, 0, 3, 7]),
        (NOISE_FILLED_0706, &[0, 1, 1, 2]),
        (CURRENT_MAP_NO, &[0x188, 0x189, 0x18e, 0x18f, 0x163, 0x40]),
        (PAUSE_ON, &[0, 0, 1]),
        (MENU_CAPTURE_STATE, &[0, 1, 2, 2, 3]),
        (YN_TEST20_SEEN, &[0, 0, 1]),
        (SCREEN_FADE_DELEGATED, &[0, 0x5a5a_5a01]),
    ] {
        let v = pick(rng, vs);
        t.global(sym, v);
    }
    let fading = if focus == Focus::Late {
        pick(rng, &[0, 1])
    } else {
        u32::from(rng.chance(8))
    };
    t.global(MOVIE_FADING, fading);
    let waiting = if focus == Focus::Video {
        pick(rng, &[0, 1, 1])
    } else {
        0
    };
    t.global(WAIT_FRAMES_BY_TIME, waiting);
    let pause = u32::from(focus == Focus::Video && rng.chance(3));
    t.global(PAUSE_WORD_134082C, pause);
}

/// The getters' objects: the application (window size around 720, the
/// float `+0x30`), the FMV manager, the motion blur, the post-process
/// manager (the scene's), and one object each for the others.
fn render_getters(t: &mut Twins, obj: u32, scene: u32, focus: Focus, rng: &mut Rng) {
    use u3::*;
    let app = t.zeros(0x300);
    let v = mild(rng);
    t.w32(app + 0x30, v);
    let w = rng.word();
    t.w32(app + 0x26c, w);
    let any = rng.next();
    let hgt = pick(rng, &[0x2cf, 0x2d0, 0x2d1, 1080, any]);
    t.w32(app + 0x270, hgt);
    stub_cdecl(t, GET_APPLICATION, 0, app);
    let fmv = t.zeros(0x700);
    for off in [0x6d0, 0x6d2] {
        let b = if focus == Focus::Video { flag(rng) } else { 0 };
        t.write(fmv + off, &[b]);
    }
    stub_cdecl(t, FMV_INSTANCE, 0, fmv);
    let mb = t.zeros(0x100);
    let b = flag(rng);
    let stray = rng.next() as u8;
    t.write(mb + 0x58, &[b, stray]);
    let v = mild(rng);
    t.w32(mb + 0x5c, v);
    stub_cdecl(t, MOTION_BLUR_INSTANCE, 0, mb);
    let pp = t.r32(scene + 0x10cc);
    stub_cdecl(t, POST_INSTANCE, 0, pp);
    let getters = [
        BROKEN_SCREEN_INSTANCE,
        INPUT_INSTANCE,
        FREE_CAMERA_INSTANCE,
        BLUR_EX_INSTANCE,
        BLUR_INSTANCE,
        DISTORTION_INSTANCE,
        SPECULAR_INSTANCE,
        INSTANCE_6DC800,
        CHARACTER_LIGHTING_INSTANCE,
    ];
    let all = getters
        .into_iter()
        .chain(TWEAK_UTILS.iter().map(|&(g, _)| g));
    for (k, sym) in (0u32..).zip(all) {
        stub_cdecl(t, sym, 0, obj + 0x1800 + 0x20 * k);
    }
}

/// Every other callee: a recording stub returning a random word; the
/// renderer's targets and scenes failing one in [`FAIL_ONE_IN`];
/// predicates answering in `al` with stray high bits.
#[allow(clippy::too_many_lines)] // the callee list
fn render_stubs(t: &mut Twins, focus: Focus, rng: &mut Rng) {
    use u3::*;
    for sym in [
        SET_COLOR_TARGET,
        SET_DEPTH_STENCIL_TARGET,
        SET_CLEAR_COLOR,
        BEGIN_SCENE,
        END_SCENE,
    ] {
        let n = match sym {
            SET_COLOR_TARGET => 2,
            SET_CLEAR_COLOR => 4,
            _ => 1,
        };
        let r = renderer_results(rng);
        t.stub_seq(sym, true, n, n, &r);
    }
    let methods = [
        (SET_VIEWPORT, 4),
        (SET_SCISSOR, 4),
        (SET_CLEAR_STENCIL, 1),
        (CLEAR_SCENE, 1),
        (QUEUE_COMMAND_1E, 2),
        (RENDER_WORLD, 3),
        (UPDATE_VIEW_MATRICES, 0),
        (MATRIX_COPY, 1),
        (MATRIX_TIMES_ROWS, 2),
        (CAMERA_COPY_4533B0, 1),
        (CAMERA_FROM_456430, 1),
        (REBUILD_MAP_OBJECTS, 0),
        (PRE_RENDER, 0),
        (SET_SHADOW_SCALE_BIAS, 0),
        (RENDER_ABMAP, 0),
        (CLEAR_ARRAYS, 0),
        (RENDER_VIDEO_AND_UI, 0),
        (RENDER_CLOUD_UI, 0),
        (RENDER_SHADOW, 0),
        (RENDER_HDAO, 0),
        (DRAW_NON_UI_PRERENDER, 0),
        (DRAW_3D_WORLD, 1),
        (FUN_0065A590, 0),
        (FUN_0066C770, 0),
        (MERGE_VFX_ARRAYS, 0),
        (DRAW_MOST_VFX, 0),
        (MOVE_CLUSTER, 0),
        (RENDER_DTT_VFX, 0),
        (RENDER_MF152, 0),
        (RENDER_POST_AA, 1),
        (RESTORE_DEFAULT_TARGETS, 0),
        (DRAW_UI_PRERENDER, 0),
        (DRAW_UI_2D_VFX, 0),
        (RENDER_VFX_AFTER_POST, 0),
        (MINIMAP_STENCIL, 0),
        (DRAW_NORMAL_UI, 0),
        (RELEASE_CLUSTER_OBJECTS, 0),
        (RESET_CROSS_FADE, 0),
        (SET_BYTE_123B1, 1),
        (SET_BYTE_123BB, 1),
        (SET_CROSS_FADE_CAPTURE, 1),
        (COPY_BUFFER, 6),
        (DEGRADE_RESOLUTION, 1),
        (RENDER_HDAO_PASS, 4),
        (POST_RENDER, 3),
        (RENDER_SCREEN_FADE, 4),
        (RENDER_FOCUS_FRAME, 2),
        (RENDER_USM, 1),
        (RENDER_BRIGHT, 2),
        (RENDER_DEBUG_4BY3, 1),
        (SET_NON_16BY9_VIEWPORT, 2),
        (LIVE_POINTER_00AC, 0),
        (LIVE_POINTER_1232C, 0),
        (BEFORE_SCENE, 0),
        (RUN_FLAG_10, 0),
        (RUN_FLAG_20, 0),
        (RUN_FLAG_4000, 0),
        (RUN_FLAG_40, 0),
        (RUN_FLAG_100, 0),
        (RUN_FLAG_200, 0),
        (BEFORE_VFX, 0),
        (BEFORE_EFFECTS, 0),
        (RUN_FLAG_400, 0),
        (RUN_FLAG_800, 0),
        (BROKEN_SCREEN_CAPTURE, 0),
        (BROKEN_SCREEN_RENDER, 0),
        (SKIP_PROMPT_RENDER, 2),
        (BLUR_EX_RENDER, 0),
        (BLUR_RENDER, 0),
        (DISTORTION_RENDER, 0),
        (RENDER_SPECULAR_PASS, 1),
        (RESET_SPECULAR_MAPS, 0),
        (MOTION_BLUR_RENDER, 2),
        (MOTION_BLUR_6C5570, 2),
        (MOTION_BLUR_SET_WEIGHT, 1),
        (VIDEO_TEXT_RENDER, 1),
        (LOADING_SCREEN_RENDER, 1),
        (IGGY_RENDER, 0),
        (CHARACTER_LIGHTING_RENDER, 3),
    ];
    let all = methods
        .into_iter()
        .chain(TWEAK_UTILS.iter().map(|&(_, r)| (r, 2)));
    for (sym, n) in all {
        let ret = rng.next();
        stub_this(t, sym, n, ret);
    }
    for (sym, n) in [(PPP_FILL_WAIT, 0), (SLEEP_MS, 1)] {
        let ret = rng.next();
        stub_cdecl(t, sym, n, ret);
    }
    let with = pick(rng, &[0, 0, 1]);
    stub_cdecl(t, WITH_3D_SCENE, 0, with);
    // the free camera's construction
    let block = if rng.chance(4) { 0 } else { t.zeros(0x400) };
    stub_cdecl(t, OPERATOR_NEW, 1, block);
    let made = free_camera(t, rng);
    stub_this(t, FREE_CAMERA_CTOR, 0, made);
    // predicates and answers
    let check = match focus {
        Focus::Scene => vec![rng.next() | 1],
        Focus::Late => vec![pick(rng, &[0, 0x100, 1])],
        _ => answers(rng, 1),
    };
    t.stub_seq(CHECK_1195, true, 0, 0, &check);
    for (sym, n) in [
        (IS_GLOW_AFTER_VFX, 0),
        (IS_MOTION_BLUR_ON, 0),
        (COPY_PREVIOUS_FRAME_FOR_DTT, 0),
        (IS_ANY_BATTLE_MAP, 0),
        (FUN_006589E0, 1),
        (FUN_006AE0C0, 1),
        (RUN_FLAG_1000, 1),
        (BROKEN_SCREEN_ACTIVE, 0),
        (INPUT_VIBRATION, 0),
        (NEED_SPECULAR_PASS, 0),
    ] {
        let mut v = answers(rng, 8);
        if sym == IS_GLOW_AFTER_VFX && rng.chance(4) {
            v[0] = 0x200;
        }
        t.stub_seq(sym, true, n, n, &v);
    }
    let v = answers(rng, 8);
    t.stub_seq(IS_SPECIAL_MAGIC, false, 0, 0, &v);
    let v: Vec<u32> = (0..8)
        .map(|_| {
            let any = rng.next();
            pick(rng, &[0, 0, 1, 0x100, any])
        })
        .collect();
    t.stub_seq(FMV_HAS_STATE, true, 1, 1, &v);
    let v: Vec<u32> = (0..8)
        .map(|_| pick(rng, &[0, 1, 2, u32::MAX, 0x8000_0000]))
        .collect();
    t.stub_seq(GET_MODE_OR_2, true, 0, 0, &v);
    let v: Vec<u32> = (0..8)
        .map(|_| pick(rng, &[0, 1, 3, u32::MAX, 0x8000_0000]))
        .collect();
    t.stub_seq(MENU_COUNT, true, 0, 0, &v);
    let v: Vec<u32> = (0..16)
        .map(|_| {
            let any = rng.next();
            pick(rng, &[0x14, 0x14, 0x110, 0x111, 0x2c2, 0x6a, 0, any])
        })
        .collect();
    t.stub_seq(MAGIC_ID, false, 0, 0, &v);
}

/// `PhyreScene_render`'s world for the test's [`Focus`]: the scene
/// (synthetic on the image; on a snapshot the game's half the time, its
/// flags varied), the globals, and every callee replaced by a recording
/// stub. Surfaces (`PRenderSurface`) are the original's. The null page
/// is mapped (a missing texture's sizes are read from it).
fn prep_render(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let focus = FOCUS.with(Cell::get);
    t.a.map_zeroed(0, 0x1000).unwrap();
    t.b.map_zeroed(0, 0x1000).unwrap();
    let game = t.r32(t.rebase(SCENE));
    let real = t.snapshot && game != 0 && rng.chance(2);
    let scene = if real {
        game
    } else {
        synthetic_scene(t, obj, rng)
    };
    *this = scene;
    scene_flags(t, scene, focus, rng);
    let cam = t.r32(scene + 0xdc);
    if cam == 0 {
        // the camera's projection is a virtual call
        t.write(scene + 0x93d, &[0]);
    } else if real {
        // the game camera's projection update (vtable slot 1) recorded
        let ret = rng.next();
        let s = t.stub(None, 0xcafe_0004, true, 0, 0, Some(ret));
        let vtable = t.r32(cam);
        t.patch(vtable + 4, &s.to_le_bytes());
    }
    render_globals(t, obj, focus, rng);
    render_getters(t, obj, scene, focus, rng);
    render_stubs(t, focus, rng);
    // the spills (clear colours, the allocation, bytes) stay in Rust locals
    FRAME_IGNORE.with(|z| z.replace(vec![(0x24, 0x14)]));
}

#[rustfmt::skip]
const CASES_RENDER: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_render), ..method("render", 0x0066_8930, This::Object, 0, &[], |m, this, _| u(hr::render(m, frame_ebp(), this))) }), 0x194, 0),
];

fn run_render(focus: Focus, fail_one_in: u32, seed: u32) {
    FOCUS.with(|f| f.set(focus));
    FAIL_ONE_IN.with(|f| f.set(fail_one_in));
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 200 };
        run_cases(t, CASES_RENDER, n, &mut Rng(seed));
    });
}

#[test]
fn render_abmap_frame_matches_the_original() {
    run_render(Focus::Abmap, 6, 0x3e01_a1a1);
}

#[test]
fn render_video_frame_matches_the_original() {
    run_render(Focus::Video, 100, 0x3e02_b2b2);
}

#[test]
fn render_late_ui_present_match_the_original() {
    run_render(Focus::Late, 200, 0x3e03_c3c3);
}

#[test]
fn render_late_errors_match_the_original() {
    run_render(Focus::Late, 12, 0x3e06_f6f6);
}

#[test]
fn render_scene_matches_the_original() {
    run_render(Focus::Scene, 400, 0x3e04_d4d4);
}

#[test]
fn render_scene_errors_match_the_original() {
    run_render(Focus::Scene, 25, 0x3e05_e5e5);
}
