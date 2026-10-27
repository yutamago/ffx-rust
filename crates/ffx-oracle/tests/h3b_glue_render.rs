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
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
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
    /// A recording stub at `sym` for a method taking two pointers: logs
    /// `tag`, `ecx` and the words they point at (what the callee reads
    /// through them at the call), pops 2 words, returns `ret`.
    fn stub_deref2(&mut self, sym: u32, ret: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        for k in 0..2u8 {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp+4+4k]
            c.extend([0x8b, 0x12]); // mov edx, [edx]
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
    t.stub_deref2(h3::MAP_INSERT, 0);
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
