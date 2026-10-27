//! `ffx_prender::glue` (M4 port round 1, list `h3-glue-render`) against
//! the original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) with the game's own scene and managers.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` (for methods) and its
//! arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order with the same `this`. On the image the
//! singleton getters are stubs too (returning a synthetic object, or 0);
//! on a snapshot they run as they are, over the game's instances.

#![allow(
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
use ffx_prender::glue::{self, bridge as g};
use ffx_prender::glue2::{self, bridge as g2};
use ffx_prender::glue3::{self, bridge as g3};
use ffx_prender::glue4::{self, bridge as g4};
use ffx_prender::glue5::{self, bridge as g5};
use ffx_prender::glue6::{self, bridge as g6};
use ffx_prender::glue7::{self, bridge as g7};
use ffx_prender::glue8::{self, bridge as g8};
use ffx_prender::glue9::{self, bridge as g9};
use ffx_prender::glue10::{self, bridge as g10};
use ffx_prender::glue11::{self, bridge as g11};
use ffx_prender::h3_glue12::{self as h12, bridge as gh12};
use ffx_prender::h3_glue13::{self as h13, bridge as gh13};
use ffx_prender::h3_glue14::{self as h14, bridge as gh14};
use ffx_prender::h3_glue15::{self as h15, bridge as gh15};
use ffx_prender::h3_glue16::{self as h16, bridge as gh16};
use ffx_prender::h3_glue17::{self as h17, bridge as gh17};
use ffx_prender::h3_glue18::{self as h18, bridge as gh18};
use ffx_prender::h3_glue19::{self as h19, bridge as gh19};
use ffx_prender::h3_glue20::{self as h20, bridge as gh20};
use ffx_prender::h3_glue21::{self as h21, bridge as gh21};
use ffx_prender::h3_glue22::{self as h22, bridge as gh22};
use ffx_prender::h3_glue23::{self as h23, bridge as gh23};
use ffx_prender::h3_glue24::{self as h24, bridge as gh24};
use ffx_prender::h3_glue25::{self as h25, bridge as gh25};
use ffx_prender::h3_glue26::{self as h26, bridge as gh26};
use ffx_prender::h3_glue27::{self as h27, bridge as gh27};
use ffx_prender::h3_glue28::{self as h28, bridge as gh28};
use ffx_prender::h3_glue29::{self as h29, bridge as gh29};
use ffx_prender::h3_glue30::{self as h30, bridge as gh30};
use ffx_prender::h3_glue31::{self as h31, bridge as gh31};
use ffx_prender::h3_glue32::{self as h32, bridge as gh32};
use ffx_prender::h3_glue33::{self as h33, bridge as gh33};
use ffx_prender::h3_glue34::{self as h34, bridge as gh34};
use ffx_prender::h3_glue35::{self as h35, bridge as gh35};
use ffx_prender::h3_glue36::{self as h36, bridge as gh36};
use ffx_prender::h3_glue37::{self as h37, bridge as gh37};
use ffx_prender::h3_glue38::{self as h38, bridge as gh38};
use ffx_prender::h3_glue39::{self as h39, bridge as gh39};
use ffx_prender::h3_glue40::{self as h40, bridge as gh40};
use ffx_prender::h3_glue41::{self as h41, bridge as gh41};
use ffx_prender::h3_glue42::{self as h42, bridge as gh42};
use ffx_prender::h3_glue43::{self as h43, bridge as gh43};
use ffx_prender::h3_glue44::{self as h44, bridge as gh44};
use ffx_prender::h3_glue45::{self as h45, bridge as gh45};
use ffx_prender::h3_glue46::{self as h46, bridge as gh46};
use ffx_prender::h3_glue47::{self as h47, bridge as gh47};
use ffx_prender::h3_glue48::{self as h48, bridge as gh48};
use ffx_prender::h3_glue49::{self as h49, bridge as gh49};
use ffx_prender::h3_glue50::{self as h50, bridge as gh50};
use ffx_prender::h3_glue51::{self as h51, bridge as gh51};
use ffx_prender::h3_glue52::{self as h52, bridge as gh52};
use ffx_prender::h3_glue53::{self as h53, bridge as gh53};
use ffx_prender::h3_glue54::{self as h54, bridge as gh54};
use ffx_prender::h3_glue55::{self as h55, bridge as gh55};
use ffx_prender::h3_glue56::{self as h56, bridge as gh56};
use ffx_prender::h3_glue57::{self as h57, bridge as gh57};
use ffx_prender::h3_glue58::bridge as gh58;
use ffx_prender::h3_glue59::{self as h59, bridge as gh59};
use ffx_prender::h3_glue60::{self as h60, bridge as gh60};
use ffx_prender::h3_glue61::{self as h61, bridge as gh61};
use ffx_prender::h3_glue62::{self as h62, bridge as gh62};
use ffx_prender::h3_glue63::{self as h63, bridge as gh63};
use ffx_prender::h3_glue64::{self as h64, bridge as gh64};
use ffx_prender::h3_glue65::{self as h65, bridge as gh65};
use ffx_prender::h3_glue66::{self as h66, bridge as gh66};
use ffx_prender::h3_glue67::{self as h67, bridge as gh67};

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
const DYN_GEO: u32 = glue::DYN_GEO_FILL.sym;

#[rustfmt::skip]
const CASES: &[Case] = &[
    // g_PhyreScene one-liners
    cdecl("activate_loading_screen", 0x0063_9130, 0, &[(glue::SCENE_START_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::activate_loading_screen(m))),
    cdecl("deactivate_loading_screen", 0x0063_de40, 0, &[(glue::SCENE_STOP_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::deactivate_loading_screen(m))),
    cdecl("scene_clear_list", 0x0063_dac0, 0, &[(glue::SCENE_CLEAR_LIST, K::This(0))], |m, _, _| u(g::scene_clear_list(m))),
    cdecl("draw_main_menu_water_effect", 0x0063_e920, 0, &[(glue::SCENE_DRAW_MAIN_MENU_WAVE, K::This(0))], |m, _, _| u(g::draw_main_menu_water_effect(m))),
    cdecl("field_map_shutdown", 0x0064_03b0, 0, &[(glue::SCENE_FIELD_MAP_SHUTDOWN, K::This(0))], |m, _, _| u(g::field_map_shutdown(m))),
    cdecl("frame_start", 0x0064_0df0, 0, &[(glue::SCENE_FRAME_START, K::This(0))], |m, _, _| { g::frame_start(m); Out::None }),
    cdecl("level_get_name", 0x0064_1d90, 0, &[(glue::SCENE_LEVEL_NAME, K::This(0))], |m, _, _| u(g::level_get_name(m))),
    cdecl("render_loading_screen", 0x0064_2c00, 0, &[(glue::SCENE_RENDER_LOADING_SCREEN, K::This(0))], |m, _, _| u(g::render_loading_screen(m))),
    cdecl("show_auto_save_prompt", 0x0064_3de0, 0, &[(glue::SCENE_SHOW_AUTO_SAVE_PROMPT, K::This(0))], |m, _, _| { g::show_auto_save_prompt(m); Out::None }),
    cdecl("texture_video_update", 0x0064_42c0, 0, &[(glue::SCENE_UPDATE_TEXTURE_VIDEO, K::This(0))], |m, _, _| u(g::texture_video_update(m))),
    cdecl("field_map_initialize", 0x0064_0200, 0, &[(glue::SCENE_FIELD_MAP_INITIALIZE, K::This(1))], |m, _, _| u(g::field_map_initialize(m))),
    cdecl("is_split_screen_enabled", 0x0064_1cc0, 0, &[(glue::SCENE_SPLIT_SCREEN_STAT, K::This(0))], |m, _, _| u(g::is_split_screen_enabled(m))),
    // g_PhyreScene with one argument
    cdecl("atel_load_texture", 0x0063_9310, 1, &[(glue::SCENE_LOAD_ATEL_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::atel_load_texture(m, a[0]))),
    cdecl("atel_unload_texture", 0x0063_9330, 1, &[(glue::SCENE_UNLOAD_ATEL_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::atel_unload_texture(m, a[0]))),
    cdecl("character_instance_add_to_pipeline", 0x0063_a010, 1, &[(glue::SCENE_ADD_TO_PIPELINE, K::This(1))], |m, _, a| { g::character_instance_add_to_pipeline(m, a[0]); Out::None }),
    cdecl("get_cha_pos", 0x0064_1080, 1, &[(glue::SCENE_GET_CHA_POS, K::This(1))], |m, _, a| u(g::get_cha_pos(m, a[0]))),
    cdecl("magic_set_path", 0x0064_2050, 1, &[(glue::SCENE_SET_MAGIC_PATH, K::This(1))], |m, _, a| u(g::magic_set_path(m, a[0]))),
    cdecl("magic_set_to_be_delete_path", 0x0064_2070, 1, &[(glue::SCENE_SET_TO_BE_DELETE_PATH, K::This(1))], |m, _, a| u(g::magic_set_to_be_delete_path(m, a[0]))),
    cdecl("magic_vfx_load_texture", 0x0064_20b0, 1, &[(glue::SCENE_LOAD_MAGIC_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::magic_vfx_load_texture(m, a[0]))),
    cdecl("magic_vfx_unload_texture", 0x0064_20f0, 1, &[(glue::SCENE_UNLOAD_MAGIC_VFX_TEXTURE, K::This(1))], |m, _, a| u(g::magic_vfx_unload_texture(m, a[0]))),
    cdecl("mesh_instance_group_id_set_visible", 0x0064_23d0, 1, &[(glue::SCENE_SET_GROUP_VISIBLE, K::This(1))], |m, _, a| u(g::mesh_instance_group_id_set_visible(m, a[0]))),
    cdecl("menu_unload_special_texture", 0x0064_2390, 1, &[(glue::MENU_UNLOAD_SPECIAL, K::This(1))], |m, _, a| u(g::menu_unload_special_texture(m, a[0]))),
    // dynamic geometry, the character shader renderer
    cdecl("dyn_geo_clear_buffer", 0x0063_fd80, 0, &[(glue::DYN_GEO_CLEAR, K::This(0))], |m, _, _| { g::dyn_geo_clear_buffer(m); Out::None }),
    cdecl("dyn_geo_current_buffer_id", 0x0063_ff50, 0, &[(glue::DYN_GEO_CURRENT_ID, K::This(0))], |m, _, _| u(g::dyn_geo_current_buffer_id(m))),
    cdecl("dyn_geo_lock", 0x0063_ff60, 0, &[(glue::DYN_GEO_LOCK, K::This(0))], |m, _, _| u(g::dyn_geo_lock(m))),
    cdecl("dyn_geo_unlock", 0x0064_0050, 0, &[(glue::DYN_GEO_UNLOCK, K::This(0))], |m, _, _| u(g::dyn_geo_unlock(m))),
    method("lock_for_record_num", 0x006d_eaf0, This::Global(DYN_GEO), 0, &[(glue::WAIT_FOR_SINGLE_OBJECT, K::Import(2))], |m, this, _| u(g::lock_for_record_num(m, this))),
    method("unlock_for_record_num", 0x006d_ed50, This::Global(DYN_GEO), 0, &[(glue::RELEASE_MUTEX, K::Import(1))], |m, this, _| u(g::unlock_for_record_num(m, this))),
    method("swap_dyn_geo_memory", 0x006d_ed40, This::Object, 0, &[(glue::DYN_GEO_SWAP, K::This(0))], |m, this, _| { g::swap_dyn_geo_memory(m, this); Out::None }),
    cdecl("shader_get_vertex_info", 0x0064_3db0, 0, &[(glue::CHR_GET_VERTEX_INFO, K::This(0))], |m, _, _| u(g::shader_get_vertex_info(m))),
    cdecl("shadow_start", 0x0064_3dc0, 0, &[(glue::CHR_SHADOW_START, K::This(0))], |m, _, _| u(g::shadow_start(m))),
    cdecl("shadow_end", 0x0064_3dd0, 0, &[(glue::CHR_SHADOW_END, K::This(0))], |m, _, _| { g::shadow_end(m); Out::None }),
    // manager singletons
    cdecl("broken_screen_set_flag5", 0x0063_9360, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_5, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag5(m))),
    cdecl("broken_screen_set_flag4", 0x0063_9370, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_4, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag4(m))),
    cdecl("broken_screen_set_flag6", 0x0063_9390, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_SET_6, K::Keep(0))], |m, _, _| u(g::broken_screen_set_flag6(m))),
    cdecl("broken_screen_reset", 0x0063_9490, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_RESET, K::This(0))], |m, _, _| { g::broken_screen_reset(m); Out::None }),
    cdecl("broken_screen_start_keep_frame", 0x0063_94a0, 0, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue::BROKEN_KEEP_FRAME, K::Keep(0))], |m, _, _| u(g::broken_screen_start_keep_frame(m))),
    cdecl("get_current_frame", 0x0064_10d0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_CURRENT_FRAME, K::This(0))], |m, _, _| u(g::get_current_frame(m))),
    cdecl("video_pause", 0x0064_5de0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_PAUSE, K::This(0))], |m, _, _| { g::video_pause(m); Out::None }),
    Case { null_instance: true, ..cdecl("destroy_fmv", 0x0063_df30, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_DESTROY, K::This(0))], |m, _, _| u(g::destroy_fmv(m))) },
    cdecl("is_video_busy", 0x0064_1ce0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::FMV_STATE_CHECK, K::This(0))], |m, _, _| u(g::is_video_busy(m))),
    cdecl("is_video_playing", 0x0064_1cf0, 0, &[(glue::FMV_INSTANCE, K::Getter)], |m, _, _| u(g::is_video_playing(m))),
    cdecl("scene_is_playing_video", 0x0064_5db0, 0, &[(glue::FMV_INSTANCE, K::Getter)], |m, _, _| u(g::scene_is_playing_video(m))),
    cdecl("magic_vfx_unload_special_shader_table", 0x0064_20e0, 0, &[(glue::VFX_SHADER_INSTANCE, K::Getter), (glue::VFX_SHADER_UNLOAD_MAGIC, K::This(0))], |m, _, _| u(g::magic_vfx_unload_special_shader_table(m))),
    cdecl("update_shadow_visibility", 0x0064_4ab0, 0, &[(glue::SHADOW_INSTANCE, K::Getter), (glue::SHADOW_CLEAR_CASCADES, K::This(0))], |m, _, _| u(g::update_shadow_visibility(m))),
    cdecl("get_file_read_sema_id", 0x0064_1150, 0, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, _| u(g::get_file_read_sema_id(m))),
    cdecl("get_old_file_read_complete_sema_id", 0x0064_1360, 0, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, _| u(g::get_old_file_read_complete_sema_id(m))),
    // the application
    cdecl("start_loading_screen", 0x0066_fb30, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::start_loading_screen(m))),
    cdecl("stop_loading_screen", 0x0066_fb50, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::stop_loading_screen(m))),
    cdecl("is_loading_screen_active", 0x0064_1c40, 0, &[(glue::GET_APPLICATION, K::Getter)], |m, _, _| u(g::is_loading_screen_active(m))),
    cdecl("set_flip_vsync", 0x0064_30e0, 1, &[(glue::GET_APPLICATION, K::Getter), (glue::SET_FLIP_VSYNC_INTERVAL, K::Cdecl(1))], |m, _, a| u(g::set_flip_vsync(m, a[0]))),
    // scene and geometry methods
    method("scene_set_magic_path", 0x0066_cfb0, This::Global(SCENE), 1, &[(glue::PSTRING_ASSIGN, K::This(1))], |m, this, a| u(g::scene_set_magic_path(m, this, a[0]))),
    method("scene_set_to_be_delete_path", 0x0066_e830, This::Global(SCENE), 1, &[(glue::PSTRING_ASSIGN, K::This(1))], |m, this, a| u(g::scene_set_to_be_delete_path(m, this, a[0]))),
    method("scene_load_atel_vfx_texture", 0x0066_3f00, This::Global(SCENE), 1, &[(glue::SCENE_LOAD_VFX_TEXTURE_ASYNC, K::This(2))], |m, this, a| u(g::scene_load_atel_vfx_texture(m, this, a[0]))),
    method("scene_unload_atel_vfx_texture", 0x0067_0b00, This::Global(SCENE), 1, &[(glue::SCENE_UNLOAD_VFX_TEXTURE, K::This(2))], |m, this, a| u(g::scene_unload_atel_vfx_texture(m, this, a[0]))),
    method("scene_release_vfx_meshes", 0x0065_9090, This::Global(SCENE), 0, &[(glue::RELEASE_MESHES, K::This(0))], |m, this, _| u(g::scene_release_vfx_meshes(m, this))),
    Case { float_args: true, ..method("set_z_correction", 0x006a_3b20, This::Object, 1, &[], |m, this, a| { g::set_z_correction(m, this, f32::from_bits(a[0])); Out::None }) },
    method("get_ready_a8", 0x006a_2010, This::Object, 0, &[], |m, this, _| u(g::get_ready_a8(m, this))),
];

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

#[test]
fn batch1_one_liners_match_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES, n, &mut Rng(0x1234_5678));
    });
}

// ---- batch 2 (ffx_prender::glue2) ----

/// `args[0]` = the object.
fn arg_object(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    args[0] = obj;
}

/// `args[0]` = the object, or 0 now and then.
fn arg_object_or_null(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = if rng.chance(4) { 0 } else { obj };
}

/// The task callback: `task = (f, arg)` with `f` a recording stub.
fn prep_task_callback(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let ret = rng.word();
    let f = t.stub(None, 0xf00d, false, 1, 0, Some(ret));
    t.w32(obj, f);
    args[0] = obj;
}

/// `onCutbufferShaderLoaded(shader, task)`: the task's scene placed so
/// that its `+0xfaa4` lands in the object.
fn prep_cutbuffer(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    let scene = (obj + 0x200).wrapping_sub(glue2::SCENE_CUTBUFFER_SHADER);
    t.w32(obj, scene);
    args[1] = obj;
}

/// `deleteIggyMenu(this)`: `this` placed so that its `+0x10dc8` lands in
/// the object.
fn prep_iggy(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = (obj + 0x300).wrapping_sub(glue2::SCENE_IGGY_MENU);
}

/// The scene's "magic async off" byte: 1 half of the time (image only).
fn prep_magic_async(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    // the PRX check: idle (0) half of the time
    let busy = if rng.chance(2) { 0 } else { rng.word() };
    let at = t.rebase(glue2::LOAD_PRX_IN_PROGRESS);
    t.stub(
        Some(at),
        glue2::LOAD_PRX_IN_PROGRESS,
        false,
        0,
        0,
        Some(busy),
    );
    if !t.snapshot {
        let scene = t.r32(t.rebase(SCENE));
        let v = if rng.chance(3) { 1 } else { rng.below(3) as u8 };
        t.write(scene + glue2::SCENE_MAGIC_ASYNC_OFF, &[v]);
    }
}

/// The sphere grid's state around 2 (image only, when it exists).
fn prep_abmap_state(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let abmap = t.r32(t.rebase(glue2::GRAPHIC_ABMAP.sym));
    if !t.snapshot && abmap != 0 {
        let v = [0, 1, 2, 3, 0xffff_ffff, 0x8000_0000][rng.below(6) as usize];
        t.w32(abmap + 4, v);
    }
}

#[rustfmt::skip]
const CASES2: &[Case] = &[
    // post effects
    cdecl("activate_blur_effect_ex", 0x0063_9110, 1, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_TARGET, K::This(1))], |m, _, a| u(g2::activate_blur_effect_ex(m, a[0]))),
    cdecl("blur_set_field84", 0x0064_3810, 1, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_FIELD84, K::This(1))], |m, _, a| u(g2::blur_set_field84(m, a[0]))),
    Case { float_args: true, ..cdecl("blur_set_floats", 0x0064_3890, 2, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue2::BLUR_SET_FLOATS6C, K::Keep(2))], |m, _, a| u(g2::blur_set_floats(m, a[0], a[1]))) },
    cdecl("magic_vfx_load_special_shader_table", 0x0064_2090, 1, &[(glue::VFX_SHADER_INSTANCE, K::Getter), (glue2::VFX_SHADER_LOAD_MAGIC, K::Keep(1))], |m, _, a| u(g2::magic_vfx_load_special_shader_table(m, a[0]))),
    cdecl("set_distortion_skipped", 0x0064_2f80, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter)], |m, _, a| u(g2::set_distortion_skipped(m, a[0]))),
    cdecl("set_distortion_enable", 0x0064_2f50, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter)], |m, _, a| u(g2::set_distortion_enable(m, a[0]))),
    Case { float_args: true, ..cdecl("set_distortion_blur", 0x0064_2f30, 1, &[(glue2::DISTORTION_INSTANCE, K::Getter), (glue2::DISTORTION_UPDATE_BLUR, K::This(1))], |m, _, a| u(g2::set_distortion_blur(m, a[0]))) },
    Case { float_args: true, ..cdecl("set_fog_limit", 0x0064_31c0, 1, &[(glue2::FOG_INSTANCE, K::Getter), (glue2::FOG_SET_LIMIT, K::This(1))], |m, _, a| u(g2::set_fog_limit(m, a[0]))) },
    Case { float_args: true, ..cdecl("set_motion_blur_weight", 0x0064_3750, 1, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter), (glue2::MOTION_BLUR_SET_WEIGHT, K::This(1))], |m, _, a| { g2::set_motion_blur_weight(m, a[0]); Out::None }) },
    cdecl("set_motion_blur_enable", 0x0064_3730, 1, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter)], |m, _, a| u(g2::set_motion_blur_enable(m, a[0]))),
    cdecl("cross_fade_capture_start", 0x0063_9150, 0, &[(glue2::POST_PROCESS_INSTANCE, K::Getter), (glue2::POST_SET_CROSS_FADE_CAPTURE, K::Keep(1)), (glue2::POST_SET_BYTE_123BB, K::Keep(1))], |m, _, _| u(g2::cross_fade_capture_start(m))),
    Case { prep: Some(arg_object_or_null), ..cdecl("water_distortion_capture_frame", 0x006f_5420, 1, &[(glue2::POST_PROCESS_INSTANCE, K::Getter), (glue2::POST_COPY_BUFFER, K::This(6))], |m, _, a| u(g2::water_distortion_capture_frame(m, a[0]))) },
    // the broken screen
    Case { float_args: true, ..cdecl("broken_screen_update_opacity", 0x0063_94e0, 1, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_SET_OPACITY, K::Keep(1))], |m, _, a| u(g2::broken_screen_update_opacity(m, a[0]))) },
    cdecl("broken_screen_init_block_data", 0x0063_9470, 3, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_INIT_BLOCK_DATA, K::This(3))], |m, _, a| u(g2::broken_screen_init_block_data(m, a[0], a[1], a[2]))),
    cdecl("broken_screen_update_light_parameters", 0x0063_94b0, 7, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_UPDATE_LIGHT, K::This(7))], |m, _, a| u(g2::broken_screen_update_light_parameters(m, [a[0], a[1], a[2], a[3], a[4], a[5], a[6]]))),
    cdecl("broken_screen_update_polygon_matrix", 0x0063_9540, 3, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue2::BROKEN_POLYGON, K::This(2)), (glue2::BROKEN_POLYGON_MATRIX, K::This(1))], |m, _, a| u(g2::broken_screen_update_polygon_matrix(m, a[0], a[1], a[2]))),
    cdecl("broken_screen_block_bytes", 0x0063_93a0, 1, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter)], |m, _, a| u(g2::broken_screen_block_bytes(m, a[0]))),
    // scene
    Case { float_args: true, ..cdecl("set_chadist", 0x0064_2ef0, 1, &[(glue2::SCENE_SET_CHADIST, K::This(1))], |m, _, a| { g2::set_chadist(m, a[0]); Out::None }) },
    cdecl("character_insert_draw_situation", 0x0063_9ff0, 1, &[(glue2::SCENE_ADD_CHR_DRAW_SITUATION, K::This(1))], |m, _, a| { g2::character_insert_draw_situation(m, a[0]); Out::None }),
    cdecl("field_map_unload", 0x0064_03c0, 0, &[(glue2::CLEAR_DEPTH_BIAS, K::Cdecl(0)), (glue2::SCENE_TEXTURE_VIDEO_UNLOAD, K::This(0)), (glue2::SCENE_FIELD_MAP_UNLOAD, K::This(0))], |m, _, _| u(g2::field_map_unload(m))),
    cdecl("render", 0x0064_2bb0, 0, &[(glue2::SCENE_RENDER, K::This(0)), (glue2::WAIT_PPP_FILL, K::Cdecl(0)), (glue2::SCENE_DO_DELETE_MESH_INSTANCES, K::This(0))], |m, _, _| u(g2::render(m))),
    cdecl("texture_animation_set_material_visibility", 0x0064_4120, 2, &[(glue2::TEX_ANIM_INSTANCE, K::Getter), (glue2::TEX_ANIM_MATERIAL_BY_LAYER, K::This(1)), (glue2::SCENE_SET_MATERIAL_VISIBILITY, K::Keep(2))], |m, _, a| u(g2::texture_animation_set_material_visibility(m, a[0], a[1]))),
    Case { prep: Some(prep_magic_async), ..cdecl("set_magic_file_async_load_enable", 0x0064_35e0, 1, &[(glue2::LOAD_PRX_IN_PROGRESS, K::Cdecl(0))], |m, _, a| u(g2::set_magic_file_async_load_enable(m, a[0]))) },
    method("scene_force_unload_menu_texture", 0x0065_d780, This::Global(SCENE), 1, &[(glue2::CLUSTER_INSTANCE, K::Getter), (glue2::CLUSTER_FORCE_RELEASE, K::This(1))], |m, _, a| u(g2::scene_force_unload_menu_texture(m, a[0]))),
    method("scene_unload_menu_texture", 0x0067_1120, This::Global(SCENE), 1, &[(glue2::CLUSTER_INSTANCE, K::Getter), (glue2::CLUSTER_RELEASE, K::This(1))], |m, _, a| u(g2::scene_unload_menu_texture(m, a[0]))),
    method("scene_load_menu_texture", 0x0066_51d0, This::Global(SCENE), 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_LOAD_CLUSTER, K::Cdecl(5))], |m, _, a| u(g2::scene_load_menu_texture(m, a[0]))),
    Case { prep: Some(prep_cutbuffer), ..cdecl("scene_on_cutbuffer_shader_loaded", 0x0066_6610, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, a| u(g2::scene_on_cutbuffer_shader_loaded(m, a[0], a[1]))) },
    Case { prep: Some(prep_iggy), ..method("scene_delete_iggy_menu", 0x0065_b750, This::Object, 0, &[(glue2::IGGY_MENU_DTOR, K::This(0)), (glue2::OPERATOR_DELETE, K::Cdecl(1))], |m, this, _| { g2::scene_delete_iggy_menu(m, this); Out::None }) },
    // async tasks
    Case { prep: Some(prep_task_callback), ..cdecl("async_task_callback", 0x0063_7860, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter)], |m, _, a| u(g2::async_task_callback(m, a[0]))) },
    cdecl("regist_async_task", 0x0064_2950, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_REGIST_TASK, K::Cdecl(7))], |m, _, a| u(g2::regist_async_task(m, a[0], a[1]))),
    cdecl("try_async_load_if_available", 0x0064_42f0, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::ASYNC_REGIST_TASK, K::Cdecl(7))], |m, _, a| u(g2::regist_async_task(m, a[0], a[1]))),
    Case { prep: Some(arg_object), ..cdecl("async_load_character_task", 0x0065_7850, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_CHARACTER, K::This(2)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_character_task(m, a[0]))) },
    Case { prep: Some(arg_object), ..cdecl("async_load_magic_file_vfx_texture_task", 0x0065_7880, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_MAGIC_FILE_VFX_TEXTURE, K::This(3)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_magic_file_vfx_texture_task(m, a[0]))) },
    Case { prep: Some(arg_object), ..cdecl("async_load_vfx_texture_task", 0x0065_78b0, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue2::SCENE_LOAD_VFX_TEXTURE, K::This(3)), (glue2::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g2::async_load_vfx_texture_task(m, a[0]))) },
    // menus, the sphere grid, the UI
    cdecl("menu_load_special_texture", 0x0064_2300, 3, &[(glue2::MENU_LOAD_SPECIAL, K::This(3))], |m, _, a| u(g2::menu_load_special_texture(m, a[0], a[1], a[2]))),
    cdecl("abmap_get_vertex_info", 0x0063_8fd0, 2, &[(glue2::ABMAP_GET_VERTEX_INFO, K::This(2))], |m, _, a| u(g2::abmap_get_vertex_info(m, a[0], a[1]))),
    cdecl("abmap_reset_layers", 0x0063_90d0, 1, &[(glue2::ABMAP_RESET_LAYERS, K::This(1))], |m, _, a| { g2::abmap_reset_layers(m, a[0]); Out::None }),
    cdecl("abmap_create", 0x0063_8f90, 1, &[(glue::GET_APPLICATION, K::Getter), (glue2::RENDER_THREAD_INSTANCE, K::Getter), (glue2::RENDER_THREAD_SYNC, K::This(0)), (glue2::ABMAP_CREATE, K::Cdecl(1))], |m, _, a| u(g2::abmap_create(m, a[0]))),
    cdecl("abmap_destroy", 0x0063_8fb0, 0, &[(glue::GET_APPLICATION, K::Getter), (glue2::RENDER_THREAD_INSTANCE, K::Getter), (glue2::RENDER_THREAD_SYNC, K::This(0)), (glue2::ABMAP_TO_DESTROY, K::This(0))], |m, _, _| { g2::abmap_destroy(m); Out::None }),
    cdecl("map_part_lang_override", 0x0064_12b0, 2, &[(glue2::UI_LANG_OVERRIDE, K::This(2))], |m, _, a| u(g2::map_part_lang_override(m, a[0], a[1]))),
    cdecl("set_ui_draw_type", 0x0064_3af0, 1, &[(glue2::UI_SET_DRAW_TYPE, K::This(1))], |m, _, a| { g2::set_ui_draw_type(m, a[0]); Out::None }),
    Case { prep: Some(prep_abmap_state), ..cdecl("menu_reset", 0x0064_2320, 0, &[(glue2::UI_MENU_RESET, K::This(0))], |m, _, _| u(g2::menu_reset(m))) },
    Case { prep: Some(prep_abmap_state), ..cdecl("menu_update", 0x0064_23b0, 0, &[(glue2::UI_MENU_UPDATE, K::This(0))], |m, _, _| u(g2::menu_update(m))) },
    cdecl("update_map_yiq", 0x0064_4990, 0, &[(glue2::INPUT_INSTANCE, K::Getter), (glue2::INPUT_UPDATE, K::This(0)), (glue2::IGGY_GLOBAL, K::Getter), (glue2::IGGY_GLOBAL_UPDATE, K::This(0)), (glue2::IGGY_MENU_UPDATE, K::This(0)), (glue2::INPUT_CLEAR, K::Cdecl(0))], |m, _, _| u(g2::update_map_yiq(m))),
    // other managers
    cdecl("get_loc_text_with_length", 0x0064_12f0, 3, &[(glue2::LOCALIZATION_INSTANCE, K::Getter), (glue2::LOCALIZATION_GET_TEXT, K::This(3))], |m, _, a| u(g2::get_loc_text_with_length(m, a[0], a[1], a[2]))),
    cdecl("set_material_uv", 0x0064_36e0, 3, &[(glue2::MORPH_INSTANCE, K::Getter), (glue2::MORPH_SET_MATERIAL_UV, K::This(3))], |m, _, a| u(g2::set_material_uv(m, a[0], a[1], a[2]))),
    cdecl("texture_animation_set_paused", 0x0064_3a00, 2, &[(glue2::TEX_ANIM_INSTANCE, K::Getter), (glue2::TEX_ANIM_SET_PAUSED, K::This(2))], |m, _, a| u(g2::texture_animation_set_paused(m, a[0], a[1]))),
    cdecl("play_video", 0x0064_27c0, 1, &[(glue::FMV_INSTANCE, K::Getter), (glue2::FMV_PLAY_VIDEO, K::This(1))], |m, _, a| u(g2::play_video(m, a[0]))),
    cdecl("init_fmv_player", 0x0064_1690, 2, &[(glue::FMV_INSTANCE, K::Getter), (glue2::FMV_INITIALIZE, K::This(3))], |m, _, a| u(g2::init_fmv_player(m, a[0], a[1]))),
    cdecl("video_update", 0x0064_5df0, 0, &[(glue::FMV_INSTANCE, K::Getter), (glue::GET_APPLICATION, K::Getter), (glue2::FMV_HANDLE_INPUT, K::This(1)), (glue2::FMV_UPDATE_TEXTURE, K::This(0))], |m, _, _| u(g2::video_update(m))),
    cdecl("register_reset_screen_fade_delegation", 0x0064_5e60, 1, &[(glue2::REGISTER_DELEGATION, K::This(4))], |m, _, a| u(g2::register_reset_screen_fade_delegation(m, a[0]))),
    method("shadow_manager_initialize", 0x006c_e100, This::Object, 0, &[(glue2::SHADOW_LOAD_SHADERS, K::This(0)), (glue2::SHADOW_CREATE_BUFFER, K::This(2)), (glue2::SHADOW_SETUP, K::This(0))], |m, this, _| u(g2::shadow_manager_initialize(m, this))),
    cdecl("model_buffer_clear_all", 0x0072_ac30, 0, &[(glue2::MEMSET, K::Cdecl(3))], |m, _, _| u(g2::model_buffer_clear_all(m))),
    // the application
    cdecl("set_force_no_vsync", 0x0064_3200, 1, &[(glue::GET_APPLICATION, K::Getter)], |m, _, a| u(g2::set_force_no_vsync(m, a[0]))),
    method("application_mode_or_2", 0x0042_f750, This::Object, 0, &[], |m, this, _| u(g2::application_mode_or_2(m, this))),
];

#[test]
fn batch2_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES2, n, &mut Rng(0x9e37_79b9));
    });
}

// ---- batch 3 (ffx_prender::glue3) ----

/// The low scene fields (`+1`, `+0x10cc`, `+0x1195`) on the object
/// itself, with `+0x10cc` pointing at a post-process object in it.
fn prep_scene_low(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let post = (obj + 0x1800).wrapping_sub(0x0001_23bb);
    t.w32(obj + 0x10cc, post);
    let v = if rng.chance(2) { 0 } else { rng.next() as u8 };
    t.write(obj + 0x1800, &[v]);
    let b = [0, 1, 4, 5, 0xff][rng.below(5) as usize];
    t.write(obj + 1, &[b]);
}

/// `obj +4` set or 0, and a recording callback or none.
fn prep_regist_helper(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = obj + 0x1000;
    args[1] = obj;
    if rng.chance(3) {
        t.w32(obj + 4, 0);
    }
    args[3] = if rng.chance(3) {
        0
    } else {
        let r = rng.word();
        t.stub(None, 0xcb, false, 1, 0, Some(r))
    };
}

/// `key` = the object.
fn prep_key(_: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], _: &mut Rng) {
    args[0] = obj + 0x40;
}

/// The magic id: 0x2b9 now and then, and the two counters around 0.
fn prep_glow(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let base = obj.wrapping_sub(0xf000);
    *this = base;
    let id = if rng.chance(3) { 0x2b9 } else { rng.word() };
    let at = t.rebase(glue3::MAGIC_ID);
    t.stub(Some(at), glue3::MAGIC_ID, true, 0, 0, Some(id));
    for off in [0xfb60, 0xfb74] {
        let v = [0, 1, 0xffff_ffff, 0x8000_0000, 7][rng.below(5) as usize];
        t.w32(base + off, v);
    }
}

/// `this` = a fresh object large enough for `+0x12404` (the diff then
/// sees every store).
fn prep_12400(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = t.zeros(0x12408);
}

/// The pause-game global: set or 0.
fn prep_pause(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.global(glue3::PAUSE_GAME.sym, v);
}

/// `operator new` gives the object, or 0 now and then.
fn prep_new(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let p = if rng.chance(4) { 0 } else { obj + 0x100 };
    let at = t.rebase(glue3::OPERATOR_NEW);
    t.stub(Some(at), glue3::OPERATOR_NEW, false, 1, 0, Some(p));
}

#[rustfmt::skip]
const CASES3: &[Case] = &[
    Case { float_args: true, ..cdecl("set_material_colour_scale2", 0x0064_3670, 4, &[(glue3::SCENE_SET_MATERIAL_COLOUR_SCALE2, K::This(4))], |m, _, a| { g3::set_material_colour_scale2(m, [a[0], a[1], a[2], a[3]]); Out::None }) },
    Case { float_args: true, ..cdecl("blur_set_positive_params", 0x0064_3830, 4, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue3::BLUR_SET_POSITIVE_PARAMS, K::Keep(4))], |m, _, a| u(g3::blur_set_positive_params(m, [a[0], a[1], a[2], a[3]]))) },
    Case { float_args: true, ..cdecl("blur_set_floats74", 0x0064_3860, 4, &[(glue2::BLUR_EX_INSTANCE, K::Getter), (glue3::BLUR_SET_FLOATS74, K::Keep(4))], |m, _, a| u(g3::blur_set_floats74(m, [a[0], a[1], a[2], a[3]]))) },
    Case { float_args: true, ..cdecl("add_screen_fade_task", 0x0063_9240, 5, &[(glue3::SCREEN_FADE_REGISTER, K::Cdecl(6))], |m, _, a| u(g3::add_screen_fade_task(m, [a[0], a[1], a[2], a[3]], a[4], 0x1001))) },
    Case { float_args: true, ..cdecl("add_screen_fade_task_after_render", 0x0063_9280, 5, &[(glue3::SCREEN_FADE_REGISTER, K::Cdecl(6))], |m, _, a| u(g3::add_screen_fade_task(m, [a[0], a[1], a[2], a[3]], a[4], 0x801))) },
    Case { prep: Some(arg_object), ..cdecl("async_field_map_load_task", 0x0065_7820, 1, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue3::SCENE_FIELD_MAP_LOAD, K::This(2)), (glue3::OPERATOR_DELETE_TASK, K::Cdecl(1))], |m, _, a| u(g3::async_field_map_load_task(m, a[0]))) },
    Case { prep: Some(prep_12400), ..method("set_fields_12400", 0x0069_3be0, This::Object, 4, &[(glue3::FIELDS_12400_UPDATE, K::This(0))], |m, this, a| u(g3::set_fields_12400(m, this, a[0], a[1], a[2]))) },
    Case { prep: Some(prep_key), ..method("cluster_new_entry", 0x0069_9300, This::Object, 1, &[(glue3::CLUSTER_ALLOC_ENTRY, K::Entry)], |m, this, a| u(g3::cluster_new_entry(m, this, a[0]))) },
    Case { prep: Some(prep_scene_low), ..method("scene_is_motion_blur_on", 0x0066_01e0, This::Object, 0, &[(glue2::MOTION_BLUR_INSTANCE, K::Getter)], |m, this, _| Out::U8(g3::scene_is_motion_blur_on(m, this))) },
    Case { prep: Some(prep_scene_low), ..method("scene_check_1195", 0x0066_62f0, This::Object, 0, &[(glue3::CHECK_01326B28, K::Cdecl(0))], |m, this, _| Out::U8(g3::scene_check_1195(m, this))) },
    Case { prep: Some(prep_glow), ..method("scene_is_glow_after_vfx", 0x0065_efa0, This::Object, 0, &[], |m, this, _| Out::U8(g3::scene_is_glow_after_vfx(m, this))) },
    Case { prep: Some(prep_pause), ..cdecl("create_pause_game", 0x006b_cc40, 0, &[(glue3::OPERATOR_NEW, K::Cdecl(1)), (glue3::PAUSE_GAME_CTOR, K::This(0))], |m, _, _| { g3::create_pause_game(m); Out::None }) },
    Case { prep: Some(prep_new), ..cdecl("tree_buy_head_node", 0x0065_4ed0, 0, &[(glue3::XBAD_ALLOC, K::Import(0))], |m, _, _| u(g3::tree_buy_head_node(m))) },
    method("scene_resize_target", 0x0069_8060, This::Object, 2, &[(glue3::TEXT_RESIZE, K::This(2))], |m, this, a| u(g3::scene_resize_target(m, this, a[0], a[1]))),
    Case { prep: Some(prep_regist_helper), ..cdecl("character_regist_helper", 0x0063_7880, 4, &[(glue3::CHARACTER_LOOKUP, K::Cdecl(2))], |m, _, a| u(g3::character_regist_helper(m, a[0], a[1], a[2], a[3]))) },
];

#[test]
fn batch3_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 32 };
        run_cases(t, CASES3, n, &mut Rng(0x0bad_cafe));
    });
}

// ---- batch 4 (ffx_prender::glue4, round 2) ----

/// One of `vs`.
fn pick(rng: &mut Rng, vs: &[u32]) -> u32 {
    vs[rng.below(vs.len() as u32) as usize]
}

/// A `PArray` of pointers at `obj + 0x40` (count `+0`, elements `+4` at
/// `obj + 0x100`), or none now and then; `args[0]` = it.
fn prep_parray(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let n = rng.below(6);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 0x40, n | top);
    t.w32(obj + 0x44, obj + 0x100);
    for i in 0..n {
        t.w32(obj + 0x100 + 4 * i, obj + 0x400 + 0x40 * i);
    }
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// `prep_parray`, and the scene's visibility test returning `al` 0 most
/// of the time.
fn prep_visible_list(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_parray(t, obj, this, args, rng);
    let r = if rng.chance(3) {
        rng.next() | 1
    } else {
        rng.next() & !0xff
    };
    let at = t.rebase(glue4::SCENE_IS_MESH_VISIBLE);
    t.stub(Some(at), glue4::SCENE_IS_MESH_VISIBLE, true, 1, 1, Some(r));
}

/// The abmap's state: 4 (delete) half of the time (image only).
fn prep_abmap4(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let abmap = t.r32(t.rebase(glue2::GRAPHIC_ABMAP.sym));
    if !t.snapshot && abmap != 0 {
        let v = pick(rng, &[4, 4, 0, 3, 5, 0xffff_ffff]);
        t.w32(abmap + 4, v);
    }
}

/// The loader task `(manager, i)` at `obj + 0x40`, `i` the last (13)
/// half of the time.
fn prep_shader_bin(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let i = if rng.chance(2) { 13 } else { rng.below(14) };
    t.w32(obj + 0x40, obj + 0x100);
    t.w32(obj + 0x44, i);
    args[1] = obj + 0x40;
}

/// The delete pool: count `+8`, keys `+0xc` (8 bytes, about half 0),
/// mesh instances `+0x1c`.
fn prep_delete_pool(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(7);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 8, n | top);
    t.w32(obj + 0xc, obj + 0x100);
    t.w32(obj + 0x1c, obj + 0x200);
    for i in 0..n {
        let (lo, hi) = match rng.below(4) {
            0 => (0, 0),
            1 => (rng.next(), 0),
            2 => (0, rng.next()),
            _ => (rng.next(), rng.next()),
        };
        t.w32(obj + 0x100 + 8 * i, lo);
        t.w32(obj + 0x104 + 8 * i, hi);
        t.w32(obj + 0x200 + 4 * i, obj + 0x400 + 0x20 * i);
        t.w32(obj + 0x40c + 0x20 * i, 0x1111_1111);
    }
}

/// A scene large enough for `+0x10d8c`, an entry with or without a mesh.
fn prep_mesh_visible(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x10d90);
    *this = scene;
    let flag = if rng.chance(3) { rng.word() | 1 } else { 0 };
    t.w32(scene + 0x0001_0d8c, flag);
    let camera = rng.next();
    t.w32(scene + 0xdc, camera);
    let mesh = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0x5c, mesh);
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// The instance 0 now and then; the flag global random.
fn prep_shader_by_instance(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(4) {
        args[0] = 0;
        args[1] = 0;
    }
    let v = rng.word();
    t.global(glue4::USE_OPTIMIZED_VFX_SHADER_ARG.sym, v);
}

/// The scene's cluster fields: `+0xfb60` around 0, byte `+0xfb44`.
fn prep_move_cluster(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    let v = pick(rng, &[0, 0, 1, 0xffff_ffff, 0x8000_0000]);
    t.w32(obj + 0xb60, v);
    let b = if rng.chance(2) {
        0
    } else {
        rng.next() as u8 | 1
    };
    t.write(obj + 0xb44, &[b]);
}

/// The scene's list count `+0x10d84`: owned (top bit clear) or not.
fn prep_clear_list(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let top = if rng.chance(2) { 0x8000_0000 } else { 0 };
    let v = rng.next() & 0x7fff_ffff | top;
    t.w32(obj + 0xd84, v);
}

/// The data lookup returning 0 (the whole loop) half of the time; keys
/// near the 32-bit carry.
fn prep_atel_shp(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(2) {
        let at = t.rebase(glue4::VFX_GET_DATA_BY_INSTANCE);
        t.stub(
            Some(at),
            glue4::VFX_GET_DATA_BY_INSTANCE,
            true,
            2,
            2,
            Some(0),
        );
    }
    match rng.below(4) {
        0 => {
            args[0] = 0;
            args[1] = 0;
        }
        1 => args[0] = 0u32.wrapping_sub(rng.below(60_000)),
        _ => {}
    }
}

/// The magic id: one of the special ones half of the time.
fn prep_special_magic(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let id = if rng.chance(2) {
        pick(
            rng,
            &[0x6a, 0x119, 0x203, 0x241, 0x20a, 0x29b, 0x1bd, 0x10e],
        )
    } else {
        pick(rng, &[0, 0x6b, 0x2b9, 0x1_006a, 0x10e | 0xff00])
    };
    let high = if rng.chance(2) {
        rng.next() & 0xffff_0000
    } else {
        0
    };
    let at = t.rebase(glue4::MAGIC_ID);
    t.stub(Some(at), glue4::MAGIC_ID, true, 0, 0, Some(id | high));
}

/// A material at `m` whose remapping (`+8` from, `+0xc` to) is one of the
/// passes or none.
fn material_at(t: &mut Twins, m: u32, rng: &mut Rng) {
    let passes = [
        glue4::PASS_SCENE,
        glue4::PASS_OPTIMIZED_TRANS,
        glue4::PASS_FORCE_TRANSPARENT,
        glue4::PASS_KEEP,
    ];
    let from = if rng.chance(3) {
        rng.next()
    } else {
        t.rebase(glue4::PASS_SCENE)
    };
    let to = if rng.chance(5) {
        rng.next()
    } else {
        let p = pick(rng, &passes);
        t.rebase(p)
    };
    t.w32(m + 8, from);
    t.w32(m + 0xc, to);
}

/// A material at `obj + 0x40`; the flag word with its low byte 0 half of
/// the time.
fn prep_material(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    material_at(t, obj + 0x40, rng);
    args[0] = obj + 0x40;
    args[1] = if rng.chance(2) {
        rng.next() & !0xff
    } else {
        rng.next() | 1
    };
}

/// The post-process manager at `+0x10cc`, mode 0, 1 or other.
fn prep_post_aa(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.w32(obj + 0x10cc, obj + 0x1800);
    args[0] = pick(rng, &[0, 1, 0, 1, 2, 0xffff_ffff]);
}

/// A geometry at `obj + 0x40` (or none) with a material array at
/// `obj + 0xc0`, `PArray_Data` giving `obj + 0x200`, materials at
/// `obj + 0x400`.
fn prep_vfx_geometry(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    t.w32(obj + 0x4c, obj + 0x80);
    t.w32(obj + 0x90, obj + 0xc0);
    let n = rng.below(5);
    let top = if rng.chance(3) { 0x8000_0000 } else { 0 };
    t.w32(obj + 0xc0, n | top);
    for i in 0..n {
        let m = obj + 0x400 + 0x20 * i;
        t.w32(obj + 0x200 + 4 * i, m);
        material_at(t, m, rng);
    }
    let at = t.rebase(glue4::PARRAY_DATA);
    t.stub(Some(at), glue4::PARRAY_DATA, true, 0, 0, Some(obj + 0x200));
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
    let any = rng.next();
    args[1] = pick(rng, &[0, 1, 0x100, any]);
}

/// Two word arrays (`+0` count, `+8` data): `this` at `obj + 0x40`,
/// `other` at `obj + 0x80` (or `this`).
fn prep_assign(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let n = pick(rng, &[0, 1, 2, 5, 0xffff_ffff]);
    t.w32(obj + 0x80, n);
    t.w32(obj + 0x88, obj + 0x200);
    // a null destination only where its first slot is the only one
    let dst = if n == 1 && rng.chance(2) {
        0
    } else {
        obj + 0x300
    };
    t.w32(obj + 0x48, dst);
    args[0] = if rng.chance(6) { *this } else { obj + 0x80 };
}

/// The scene's prompt (`+0x10ddc`) set or 0, the Iggy manager 0 now and
/// then (image only).
fn prep_auto_save(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let old = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0xddc, old);
    if !t.snapshot && rng.chance(2) {
        t.global(glue2::IGGY_MENU_MANAGER.sym, 0);
    }
}

/// `isVFXTextureLoaded` giving 0..3, the cluster lookup 0 or not.
fn prep_bp_loaded(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let r = pick(rng, &[0, 1, 1, 2, 3, 0x101]);
    let at = t.rebase(glue4::SCENE_IS_VFX_TEXTURE_LOADED);
    t.stub(
        Some(at),
        glue4::SCENE_IS_VFX_TEXTURE_LOADED,
        true,
        7,
        7,
        Some(r),
    );
    let c = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue4::CLUSTER_BY_NAME);
    t.stub(Some(at), glue4::CLUSTER_BY_NAME, true, 1, 1, Some(c));
}

/// An array (count, capacity, data) at `a` of `size`-byte entries, its
/// data at `data` (or 0 when empty); now and then a negative count and
/// capacity, with the data placed so that the new slot falls on 0 (the
/// signed compare and the signed `n / 8`).
fn array_at(t: &mut Twins, a: u32, data: u32, size: u32, rng: &mut Rng) {
    if rng.chance(6) {
        // count -10: n = -9, slot = data + (n - 1) * size
        t.w32(a, 0xffff_fff6);
        let cap = pick(rng, &[0xffff_ff9c, 0xffff_ffff, 3]);
        t.w32(a + 4, cap);
        t.w32(a + 8, 10 * size);
        return;
    }
    let n = rng.below(8);
    let cap = match rng.below(3) {
        0 => n,
        1 => n + 1 + rng.below(3),
        _ => n.saturating_sub(1),
    };
    t.w32(a, n);
    t.w32(a + 4, cap);
    let d = if n == 0 && rng.chance(2) { 0 } else { data };
    t.w32(a + 8, d);
}

/// The scene's pipeline array at `+0xfb48` (count, capacity, data), an
/// entry or none.
fn prep_pipeline(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    array_at(t, obj + 0xb48, obj + 0x1000, 4, rng);
    args[0] = if rng.chance(5) { 0 } else { obj + 0x40 };
}

/// The draw-event array at `obj + 0x40`, the event at `obj + 0x80`.
fn prep_event_push(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    array_at(t, obj + 0x40, obj + 0x400, 16, rng);
    args[0] = obj + 0x80;
}

/// The scene's event fields (`+0xf8cc`, layers `+0xf910`/`+0xf954`,
/// counters `+0xfb60`) and an event of type -1..7.
fn prep_add_event(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
    let kind = pick(rng, &[0, 1, 2, 3, 4, 5, 6, 7, 0xffff_ffff, 0x8000_0000]);
    t.w32(obj + 0x80, kind);
    let layer = pick(rng, &[0, 0, 1, 3, 8, 0xffff_ffff]);
    t.w32(obj + 0x954, layer);
    args[0] = obj + 0x80;
}

/// A tree header at `obj + 0x100` whose root `+0xc` is the header, 0, or a
/// chain of left links back to the header.
fn prep_tree(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let tree = obj + 0x100;
    let root = match rng.below(4) {
        0 => tree,
        1 => 0,
        _ => {
            let n = 1 + rng.below(4);
            for k in 0..n {
                let node = obj + 0x200 + 0x20 * k;
                let left = if k + 1 == n { tree } else { node + 0x20 };
                t.w32(node, left);
            }
            obj + 0x200
        }
    };
    t.w32(tree + 0xc, root);
    args[0] = tree;
}

/// The fill's mode `+0` (0..3), count `+8` and floats `[+0x48]`.
fn prep_fill(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let mode = pick(rng, &[1, 2, 1, 2, 0, 3]);
    t.w32(obj, mode);
    // 0x4000_0000 * 4 wraps to 0: no stores
    let n = if rng.chance(6) {
        0x4000_0000
    } else {
        rng.below(7)
    };
    t.w32(obj + 8, n);
    t.w32(obj + 0x48, obj + 0x400);
}

/// An object at `obj` whose virtual slot 0 is a recording stub; `p` =
/// `obj + 4`.
fn prep_descriptor_destroy(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(obj, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = obj + 4;
}

/// 64 slots (`+8`, 8 bytes) with `data` in one of them, or none; the
/// compare-exchange giving 1 or not.
fn prep_free_list(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let data = rng.next() | 1;
    for i in 0..64 {
        t.w32(obj + 8 + 8 * i, data ^ (i + 1));
    }
    if !rng.chance(4) {
        let i = rng.below(64);
        t.w32(obj + 8 + 8 * i, data);
    }
    args[0] = data;
    let r = if rng.chance(2) { 1 } else { rng.word() };
    let s = t.stub(
        None,
        glue4::INTERLOCKED_COMPARE_EXCHANGE,
        false,
        3,
        3,
        Some(r),
    );
    let at = t.rebase(glue4::INTERLOCKED_COMPARE_EXCHANGE);
    t.patch(at, &s.to_le_bytes());
}

/// `this` such that the scene's YIQ fields (`+0xfc88`..) land in the
/// object.
fn this_for_yiq(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = obj.wrapping_sub(0xf000);
}

fn arr<const N: usize>(a: &[u32]) -> [u32; N] {
    a[..N].try_into().unwrap()
}

#[rustfmt::skip]
const CASES4: &[Case] = &[
    cdecl("graphic_reset_yiq", 0x0064_2cc0, 0, &[(glue4::SCENE_RESET_YIQ, K::This(0))], |m, _, _| u(g4::graphic_reset_yiq(m))),
    Case { prep: Some(this_for_yiq), ..method("scene_reset_yiq", 0x0066_bf40, This::Object, 0, &[(glue4::SCENE_UPDATE_MAP_YIQ, K::This(0))], |m, this, _| u(g4::scene_reset_yiq(m, this))) },
    Case { prep: Some(prep_parray), ..cdecl("update_skeleton_bounds", 0x0063_c500, 1, &[(glue4::MESH_UPDATE_BOUNDS_FROM_SKELETON, K::This(0))], |m, _, a| { g4::update_skeleton_bounds(m, a[0]); Out::None }) },
    Case { float_args: true, ..cdecl("set_abmap", 0x0064_2da0, 5, &[(glue4::SCENE_SET_ABMAP_CAMERA, K::This(5))], |m, _, a| u(g4::set_abmap(m, arr(a)))) },
    Case { prep: Some(prep_abmap4), ..cdecl("abmap_update", 0x0063_90f0, 0, &[(glue4::ABMAP_DELETE, K::Cdecl(0)), (glue4::ABMAP_UPDATE, K::This(0))], |m, _, _| { g4::abmap_update(m); Out::None }) },
    Case { prep: Some(prep_shader_bin), ..cdecl("post_on_shader_bin_loaded", 0x0069_4c20, 2, &[(glue::ASYNC_LOAD_INSTANCE, K::Getter), (glue4::POST_SHADER_BIN_STEP, K::This(1)), (glue4::POST_ON_ALL_SHADER_BINS, K::This(0))], |m, _, a| u(g4::post_on_shader_bin_loaded(m, a[0], a[1]))) },
    Case { null_instance: true, ..cdecl("broken_screen_update_polygon_data", 0x0063_9500, 11, &[(glue::BROKEN_SCREEN_INSTANCE, K::Getter), (glue4::BROKEN_POLYGON, K::This(2)), (glue4::BROKEN_POLYGON_UPDATE, K::This(9))], |m, _, a| u(g4::broken_screen_update_polygon_data(m, &arr(a)))) },
    Case { prep: Some(prep_delete_pool), ..method("vfx_remove_deleted_pool_mesh_instances", 0x0069_fc70, This::Object, 0, &[], |m, this, _| u(g4::vfx_remove_deleted_pool_mesh_instances(m, this))) },
    Case { prep: Some(prep_mesh_visible), ..method("scene_is_mesh_visible", 0x0066_03c0, This::Object, 1, &[(glue4::SCENE_MESH_VISIBLE_TEST, K::This(1))], |m, this, a| u(g4::scene_is_mesh_visible(m, this, a[0]))) },
    Case { prep: Some(prep_shader_by_instance), ..cdecl("use_optimized_vfx_shader_by_instance", 0x0064_4b10, 2, &[(glue4::VFX_GET_DYN_GEO_BY_INSTANCE, K::This(2)), (glue4::USE_OPTIMIZED_VFX_SHADER, K::Cdecl(2))], |m, _, a| u(g4::use_optimized_vfx_shader_by_instance(m, a[0], a[1]))) },
    Case { prep: Some(prep_move_cluster), ..method("scene_move_cluster", 0x0065_8f00, This::Object, 0, &[(glue4::WORLD_REMOVE_CLUSTER, K::This(1)), (glue4::WORLD_ADD_CLUSTER, K::This(1))], |m, this, _| { g4::scene_move_cluster(m, this); Out::None }) },
    Case { prep: Some(prep_clear_list), ..method("scene_clear_list", 0x0065_8fc0, This::Object, 0, &[(glue4::PHYRE_FREE, K::Cdecl(1))], |m, this, _| u(g4::scene_clear_list(m, this))) },
    Case { prep: Some(prep_atel_shp), ..cdecl("vfx_is_atel_shp_instanced", 0x0064_5d40, 2, &[(glue4::VFX_GET_DATA_BY_INSTANCE, K::This(2))], |m, _, a| u(g4::vfx_is_atel_shp_instanced(m, a[0], a[1]))) },
    Case { prep: Some(prep_special_magic), ..method("scene_is_special_magic", 0x0065_f1f0, This::Object, 0, &[], |m, this, _| u(g4::scene_is_special_magic(m, this))) },
    Case { prep: Some(prep_material), ..cdecl("use_vfx_optimized_trans", 0x0065_4b20, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::use_vfx_optimized_trans(m, a[0], a[1]))) },
    Case { prep: Some(prep_material), ..cdecl("force_transparent", 0x0065_1400, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::force_transparent(m, a[0], a[1]))) },
    Case { float_args: true, ..cdecl("update_distortion_vertex", 0x0064_4940, 8, &[(glue4::DISTORTION_INSTANCE, K::Getter), (glue4::DISTORTION_UPDATE_VERTEX, K::This(8))], |m, _, a| u(g4::update_distortion_vertex(m, a[0], a[1], arr(&a[2..])))) },
    Case { prep: Some(prep_post_aa), ..method("scene_render_post_aa", 0x0065_35b0, This::Object, 1, &[(glue4::POST_RENDER_FXAA, K::This(2)), (glue4::POST_RENDER_SMAA, K::This(1))], |m, this, a| u(g4::scene_render_post_aa(m, this, a[0]))) },
    Case { prep: Some(prep_vfx_geometry), ..cdecl("use_optimized_vfx_shader", 0x0064_4ac0, 2, &[(glue4::MATERIAL_SET_PASS_REMAPPING, K::This(2))], |m, _, a| u(g4::use_optimized_vfx_shader(m, a[0], a[1]))) },
    Case { prep: Some(prep_assign), ..method("parray_assign_0064ead0", 0x0064_ead0, This::Object, 1, &[(glue4::ARRAY_RESERVE_667F00, K::KeepDeref(1))], |m, this, a| u(g4::parray_assign(m, glue4::ARRAY_RESERVE_667F00, this, a[0]))) },
    Case { prep: Some(prep_assign), ..method("parray_assign_0064eb20", 0x0064_eb20, This::Object, 1, &[(glue4::ARRAY_RESERVE_668080, K::KeepDeref(1))], |m, this, a| u(g4::parray_assign(m, glue4::ARRAY_RESERVE_668080, this, a[0]))) },
    Case { prep: Some(prep_auto_save), ..method("scene_show_auto_saving_prompt", 0x0066_f480, This::Object, 0, &[(glue4::IGGY_MENU_DTOR, K::This(0)), (glue4::OPERATOR_DELETE, K::Cdecl(1)), (glue4::IGGY_PLAYER_OPEN_SCALED, K::Cdecl(5))], |m, this, _| { g4::scene_show_auto_saving_prompt(m, this); Out::None }) },
    Case { prep: Some(prep_bp_loaded), ..cdecl("vfx_is_texture_with_bp_loaded", 0x0064_5450, 7, &[(glue4::CLUSTER_INSTANCE, K::Getter)], |m, _, a| u(g4::vfx_is_texture_with_bp_loaded(m, &arr(a)))) },
    Case { prep: Some(prep_visible_list), ..cdecl("character_instance_is_visible", 0x0063_b230, 1, &[], |m, _, a| u(g4::character_instance_is_visible(m, a[0]))) },
    Case { prep: Some(prep_pipeline), ..method("scene_add_to_pipeline", 0x0065_6590, This::Object, 1, &[(glue4::ARRAY_RESERVE_667F80, K::Keep(1))], |m, this, a| { g4::scene_add_to_pipeline(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_event_push), ..method("vfx_draw_event_push", 0x0065_6350, This::Object, 1, &[(glue4::ARRAY_RESERVE_668280, K::Keep(1))], |m, this, a| u(g4::vfx_draw_event_push(m, this, a[0]))) },
    Case { prep: Some(prep_add_event), ..method("scene_add_vfx_draw_event", 0x0065_6940, This::Object, 1, &[(glue4::VFX_DRAW_EVENT_PUSH, K::This(1))], |m, this, a| u(g4::scene_add_vfx_draw_event(m, this, a[0]))) },
    Case { prep: Some(prep_tree), ..method("tree_iterator_begin", 0x0064_dae0, This::Object, 1, &[(glue4::TREE_SUCCESSOR, K::This(1))], |m, this, a| u(g4::tree_iterator_begin(m, this, a[0]))) },
    Case { prep: Some(prep_fill), ..method("broken_fill_floats", 0x006d_4150, This::Object, 0, &[], |m, this, _| u(g4::broken_fill_floats(m, this))) },
    Case { prep: Some(prep_descriptor_destroy), ..method("shadow_dynamic_mesh_destroy", 0x0067_89c0, This::Stdcall, 1, &[], |m, _, a| u(g4::shadow_dynamic_mesh_destroy(m, a[0]))) },
    Case { prep: Some(prep_free_list), ..method("set_data_to_free_list", 0x006a_e1a0, This::Object, 1, &[(glue4::IOB_FUNC, K::ImportC(0)), (glue4::FPRINTF, K::ImportC(3))], |m, this, a| u(g4::set_data_to_free_list(m, this, a[0]))) },
];

#[test]
fn batch4_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES4, n, &mut Rng(0x4b47_1e04));
    });
}

// ---- batch 5 (ffx_prender::glue5, round 2) ----

thread_local! {
    /// The original's `ebp` for a case with a frame ([`Case::frame`]),
    /// which the port's frame takes.
    static FRAME_EBP: Cell<u32> = const { Cell::new(0) };
}

fn frame_ebp() -> u32 {
    FRAME_EBP.with(Cell::get)
}

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

    /// A recording stub at `sym` (as [`Twins::stub`]) that first stores
    /// `value` at `[arg0 + off]` (an output pointer).
    fn stub_out(&mut self, sym: u32, n: u32, pops: u32, ret: u32, off: u8, value: u32) {
        let log = self.stub(None, sym, true, n, pops, Some(ret));
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0xc7, 0x40, off]); // mov dword [eax+off], value
        c.extend(value.to_le_bytes());
        let at = self.cave_next;
        c.push(0xe9); // jmp log
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// A scene cursor at `obj + 0x40` (end `+0x14`, node `+0x18`, target
/// `+0x1c`) over a chain of nodes at `obj + 0x200`, each valid (its
/// `+0x2c` the target and `+0x40` set) or not.
fn prep_cursor(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let cursor = obj + 0x40;
    *this = cursor;
    let end = obj + 0x1f00;
    let target = obj + 0x1800;
    t.w32(cursor + 0x14, end);
    t.w32(cursor + 0x1c, target);
    let n = rng.below(5);
    for k in 0..n {
        let node = obj + 0x200 + 0x80 * k;
        let next = if k + 1 == n { end } else { node + 0x80 };
        t.w32(node, next);
        let owner = match rng.below(3) {
            0 => 0,
            1 => target,
            _ => obj + 0x1900,
        };
        t.w32(node + 0x2c, owner);
        let set = u32::from(!rng.chance(3));
        t.w32(node + 0x40, set);
    }
    t.w32(obj + 0x1940, 0);
    let first = if n == 0 || rng.chance(5) {
        0
    } else {
        obj + 0x200
    };
    t.w32(cursor + 0x18, first);
}

/// The 20-byte array at `obj + 0x40`, the entry at `obj + 0x80`.
fn prep_push20(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    array_at(t, obj + 0x40, obj + 0x400, 20, rng);
    args[0] = obj + 0x80;
}

/// The scene's object pool (`+0xfbb8` base, `+0xfbc4` used bytes,
/// `+0xfbc8` lowest free), an object with a recording destructor, and
/// the pool cluster or another.
fn prep_delete_object(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = obj.wrapping_sub(0xf000);
    *this = scene;
    let object = obj + 0x300;
    let i = pick(rng, &[0, 5, 0xbb7, 0xbb8, 0xffff_ffff, 100]);
    let low = if rng.chance(3) { rng.below(16) } else { 0 };
    t.w32(
        obj + 0xbb8,
        object.wrapping_sub(16u32.wrapping_mul(i)).wrapping_sub(low),
    );
    t.w32(obj + 0xbc4, obj + 0x1000);
    let free = pick(rng, &[0, 3, 6, 0xbb8, 0xffff_ffff]);
    t.w32(obj + 0xbc8, free);
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(object, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = match rng.below(6) {
        0 => 0,
        1 => 0xffff_fffc,
        _ => object,
    };
    args[1] = if rng.chance(3) {
        rng.next()
    } else {
        scene + 0x12d4
    };
}

/// A word vector at `obj + 0x40` (begin, end, capacity end) and a value
/// inside it, at its end, or outside.
#[allow(clippy::many_single_char_names)] // the vector, count and code
fn prep_vector(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let v = obj + 0x40;
    *this = v;
    if rng.chance(6) {
        for k in 0..3 {
            t.w32(v + 4 * k, 0);
        }
        args[0] = obj + 0x80;
        return;
    }
    let begin = obj + 0x400;
    let n = rng.below(6);
    let end = begin + 4 * n;
    let cap = if rng.chance(2) { end } else { end + 8 };
    t.w32(v, begin);
    t.w32(v + 4, end);
    t.w32(v + 8, cap);
    if cap == end {
        // the growth moves the elements to obj + 0x600 (left as they are
        // there: the old and new copies differ)
        let log = t.stub(None, glue5::VECTOR_GROW, true, 1, 1, None);
        let moved = obj + 0x600;
        let mut c = vec![0xc7, 0x01]; // mov dword [ecx], moved
        c.extend(moved.to_le_bytes());
        c.extend([0xc7, 0x41, 0x04]); // mov dword [ecx+4], moved end
        c.extend((moved + 4 * n).to_le_bytes());
        c.extend([0xc7, 0x41, 0x08]); // mov dword [ecx+8], capacity end
        c.extend((moved + 4 * n + 8).to_le_bytes());
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = t.code(&c);
        let site = t.rebase(glue5::VECTOR_GROW);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        t.patch(site, &j);
    }
    args[0] = match rng.below(4) {
        0 => obj + 0x80,
        1 => end,
        _ => begin + 4 * rng.below(n.max(1)),
    };
}

/// The map number (0x17, 0x9f or others), the game-over flag, and the
/// pending delayed release 0 or not.
fn prep_delay_release(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let map = pick(rng, &[0x17, 0x9f, 5, 0, 0x17, 0x9f]);
    t.global(glue5::CURRENT_MAP_NO.sym, map);
    let over = u32::from(rng.chance(3));
    t.global(glue5::GAME_OVER_NOTIFY.sym, over);
    let pending = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue5::CHR_IS_DELAYED_RELEASE);
    t.stub(
        Some(at),
        glue5::CHR_IS_DELAYED_RELEASE,
        true,
        0,
        0,
        Some(pending),
    );
}

/// The dynamic geometry at `obj + 0x40` (or none) whose virtual slots 1
/// and 2 are recording stubs; the instance 0 now and then.
fn prep_vfx_2d(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let geo = obj + 0x40;
    let r1 = if rng.chance(2) { 0 } else { rng.next() | 1 };
    let s1 = t.stub(None, 0xa1, true, 2, 2, Some(r1));
    let r2 = rng.next();
    let s2 = t.stub(None, 0xa2, true, 1, 1, Some(r2));
    t.w32(geo, obj + 0x900);
    t.w32(obj + 0x904, s1);
    t.w32(obj + 0x908, s2);
    let g = if rng.chance(4) { 0 } else { geo };
    let at = t.rebase(glue5::VFX_GET_DYN_GEO_BY_INSTANCE);
    t.stub(
        Some(at),
        glue5::VFX_GET_DYN_GEO_BY_INSTANCE,
        true,
        2,
        2,
        Some(g),
    );
    if rng.chance(4) {
        args[0] = 0;
        args[1] = 0;
    }
}

/// The dream effect on or off, its phase around 5.
fn prep_dream(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let on = u32::from(!rng.chance(2));
    t.global(glue5::DREAM_ON.sym, on);
    let phase = pick(rng, &[0, 4, 5, 6, 0xffff_ffff, 0x8000_0000]);
    t.global(glue5::DREAM_PHASE.sym, phase);
}

/// A cluster or none; the task `(manager, id)` at `obj + 0x40`.
fn prep_shader_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(4) {
        args[0] = 0;
    }
    let manager = rng.next();
    t.w32(obj + 0x40, manager);
    let id = rng.next();
    t.w32(obj + 0x44, id);
    args[1] = obj + 0x40;
}

/// A material at `obj + 0x40` (or none), its parameter found or not.
fn prep_visibility(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let m = obj + 0x40;
    let params = rng.next();
    t.w32(m, params);
    let buffer = rng.next();
    t.w32(m + 4, buffer);
    args[0] = if rng.chance(5) { 0 } else { m };
    let p = if rng.chance(4) { 0 } else { rng.next() | 1 };
    let at = t.rebase(glue5::SHADER_PARAMS_FIND);
    t.stub(Some(at), glue5::SHADER_PARAMS_FIND, true, 1, 1, Some(p));
    let any = rng.next();
    args[1] = pick(rng, &[0, 1, 0xffff_ffff, 0x0100_0001, 0x8000_0000, any]);
}

/// `PeekMessageW` writing a message (`WM_QUIT` now and then) and giving
/// one or none.
fn prep_peek(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let got = u32::from(!rng.chance(3));
    let message = pick(rng, &[0x12, 0x12, 0x0f, 0x112]);
    let log = t.stub(None, glue5::PEEK_MESSAGE, false, 5, 5, Some(got));
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    c.extend([0xc7, 0x40, 0x04]); // mov dword [eax+4], message
    c.extend(message.to_le_bytes());
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    let code = t.code(&c);
    let slot = t.rebase(glue5::PEEK_MESSAGE);
    t.patch(slot, &code.to_le_bytes());
}

/// The constant entry (`+0xa0`/`+0xa4`) or none, the buffer
/// (`[[+0x1c] + 4]`) of a small size, offsets and lengths around it.
fn prep_constant(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for field in [0xa0, 0xa4] {
        let e = if rng.chance(5) { 0 } else { obj + 0x100 };
        t.w32(obj + field, e);
    }
    t.w32(obj + 0x1c, obj + 0x180);
    t.w32(obj + 0x184, obj + 0x200);
    let size = pick(rng, &[0, 8, 16, 20, 32, 64]);
    t.w32(obj + 0x200, size);
    let off = pick(rng, &[0, 3, 4, 8, 12, 16, 48]);
    let high = if rng.chance(2) { 0xe000 } else { 0 };
    let len = pick(rng, &[0, 4, 8, 16, 12]) | high;
    t.write(obj + 0x108, &(off as u16).to_le_bytes());
    t.write(obj + 0x10a, &(len as u16).to_le_bytes());
}

/// The map lookup storing the end (`[this]`, not found) or a node at
/// `obj + 0x300` into the iterator; the cached key matching or not.
fn prep_vfx_find(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let end = obj + 0x1f00;
    t.w32(obj, end);
    let node = if rng.chance(2) { end } else { obj + 0x300 };
    let r = rng.next();
    t.stub_out(glue5::VFX_MAP_FIND, 2, 2, r, 0, node);
    if rng.chance(2) {
        t.w32(obj + 0x20, args[0]);
        let hi = if rng.chance(3) { args[1] ^ 1 } else { args[1] };
        t.w32(obj + 0x24, hi);
    }
    let at = t.rebase(glue5::VFX_MAP_NEW_NODE);
    t.stub(
        Some(at),
        glue5::VFX_MAP_NEW_NODE,
        true,
        1,
        1,
        Some(obj + 0x500),
    );
}

/// A mesh instance at `obj + 0x40` with a world matrix (`+0xc`) or none,
/// the matrix argument at `obj + 0x400`.
fn prep_mesh_matrix(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x40;
    let world = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.w32(obj + 0x4c, world);
    args[0] = obj + 0x400;
}

const fn framed(case: Case, low: u32, high: u32) -> Case {
    Case {
        frame: Some((low, high)),
        ..case
    }
}

#[rustfmt::skip]
const CASES5: &[Case] = &[
    Case { prep: Some(prep_cursor), ..method("cursor_next_6377a0", 0x0063_77a0, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_next(m, this, glue5::CURSOR_IS_VALID_637F80))) },
    Case { prep: Some(prep_cursor), ..method("cursor_next_6b22e0", 0x006b_22e0, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_next(m, this, glue5::CURSOR_IS_VALID_6B28F0))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_637730", 0x0063_7730, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_637F80))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_65eb60", 0x0065_eb60, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_660270))) },
    Case { prep: Some(prep_cursor), ..method("cursor_begin_6b2230", 0x006b_2230, This::Object, 0, &[(glue5::CURSOR_SET_NODE, K::This(1))], |m, this, _| u(g5::cursor_begin(m, this, glue5::CURSOR_IS_VALID_6B28F0))) },
    Case { prep: Some(prep_push20), ..method("parray_push_back20", 0x0065_62f0, This::Object, 1, &[(glue5::ARRAY_RESERVE_668200, K::Keep(1))], |m, this, a| u(g5::parray_push_back20(m, this, a[0]))) },
    Case { prep: Some(prep_delete_object), ..method("scene_delete_object_in_cluster", 0x0065_a250, This::Object, 2, &[(glue5::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g5::scene_delete_object_in_cluster(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_vector), ..method("vector_push_back", 0x0066_7620, This::Object, 1, &[(glue5::VECTOR_GROW, K::Keep(1))], |m, this, a| { g5::vector_push_back(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_delay_release), ..cdecl("enable_chr_cluster_delay_release", 0x0064_0090, 0, &[(glue5::SCENE_LEVEL_NAME_HASH, K::This(0)), (glue5::CLUSTER_INSTANCE, K::Getter), (glue5::CHR_ENABLE_DELAY_RELEASE, K::This(0)), (glue5::CHR_APPLY_DELAYED_RELEASE, K::This(0)), (glue5::IOB_FUNC, K::ImportC(0)), (glue5::FPRINTF, K::ImportC(2))], |m, _, _| u(g5::enable_chr_cluster_delay_release(m))) },
    Case { prep: Some(prep_vfx_2d), ..cdecl("vfx_2d_setup", 0x0064_4b50, 2, &[(glue5::PHYRE_PRINTF, K::Cdecl(2))], |m, _, a| u(g5::vfx_2d_setup(m, a[0], a[1]))) },
    framed(Case { prep: Some(prep_dream), ..cdecl("scene_on_menu_texture_loaded", 0x0066_6740, 1, &[(glue5::GET_APPLICATION, K::Cdecl(0)), (glue5::FIXUP_CLUSTERS, K::Cdecl(2))], |m, _, a| u(g5::scene_on_menu_texture_loaded(m, frame_ebp(), a[0]))) }, 0, 4),
    framed(Case { prep: Some(prep_shader_loaded), ..cdecl("vfx_shader_on_loaded", 0x006c_13e0, 2, &[(glue5::ASYNC_LOAD_INSTANCE, K::Getter), (glue5::VFX_SHADER_REGISTER, K::This(2)), (glue5::GET_APPLICATION, K::Cdecl(0)), (glue5::FIXUP_CLUSTERS, K::Cdecl(2))], |m, _, a| u(g5::vfx_shader_on_loaded(m, frame_ebp(), a[0], a[1]))) }, 0, 8),
    framed(Case { prep: Some(prep_visibility), ..method("scene_set_material_visibility", 0x0066_d5c0, This::Stdcall, 2, &[(glue5::SHADER_PARAM_SET_VALUE, K::This(2))], |m, _, a| { g5::scene_set_material_visibility(m, frame_ebp(), a[0], a[1]); Out::None }) }, 0x10, 0),
    framed(Case { prep: Some(prep_peek), ..cdecl("process_windows_message", 0x0064_2860, 0, &[(glue5::TRANSLATE_MESSAGE, K::Import(1)), (glue5::DISPATCH_MESSAGE, K::Import(1))], |m, _, _| u(g5::process_windows_message(m, frame_ebp()))) }, 0x1c, 0),
    framed(Case { prep: Some(prep_constant), ..method("broken_set_constant_a0", 0x006d_46a0, This::Object, 4, &[(glue5::MEMCPY, K::Cdecl(3))], |m, this, a| u(g5::broken_set_constant(m, frame_ebp(), this, arr(a), 0xa0))) }, 0, 16),
    framed(Case { prep: Some(prep_constant), ..method("broken_set_constant_a4", 0x006d_4710, This::Object, 4, &[(glue5::MEMCPY, K::Cdecl(3))], |m, this, a| u(g5::broken_set_constant(m, frame_ebp(), this, arr(a), 0xa4))) }, 0, 16),
    framed(Case { prep: Some(prep_vfx_find), ..method("vfx_table_remove", 0x0069_fc10, This::Object, 2, &[(glue5::VFX_INSERT_TO_DELETE_POOL, K::This(4)), (glue5::VFX_MAP_ERASE, K::This(2))], |m, this, a| u(g5::vfx_table_remove(m, frame_ebp(), this, a[0], a[1]))) }, 4, 8),
    framed(Case { prep: Some(prep_vfx_find), ..method("vfx_table_insert", 0x0069_f890, This::Object, 3, &[(glue5::VFX_MAP_INSERT, K::This(4))], |m, this, a| u(g5::vfx_table_insert(m, frame_ebp(), this, a[0], a[1], a[2]))) }, 0x10, 12),
    framed(Case { prep: Some(prep_mesh_matrix), ..method("mesh_instance_is_visible", 0x0066_3cc0, This::Object, 1, &[(glue5::MATRIX4_FROM_3X4, K::This(1)), (glue5::MATRIX4_MULTIPLY, K::Cdecl(3)), (glue5::BOUNDS_IS_VISIBLE, K::This(1))], |m, this, a| u(g5::mesh_instance_is_visible(m, frame_ebp(), this, a[0]))) }, 0x80, 0),
];

#[test]
fn batch5_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES5, n, &mut Rng(0x5eed_b005));
    });
}

// ---- batch 6 (ffx_prender::glue6, round 2) ----

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

    /// A recording stub at `sym` returning `ret`.
    fn stub_ret(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        self.stub(Some(at), sym, this, n, pops, Some(ret));
    }
}

const SINGLETONS: [glue6::Singleton; 7] = [
    glue6::VFX_RENDER_DATA_TABLE_SINGLETON,
    glue6::BROKEN_SCREEN_SINGLETON,
    glue6::FREE_CAMERA_SINGLETON,
    glue6::CLUSTER_MANAGER_SINGLETON,
    glue6::VFX_SHADER_MANAGER_SINGLETON,
    glue6::DISTORTION_SINGLETON,
    glue6::SHADOW_MANAGER_SINGLETON,
];

/// Every singleton global 0 or set; `operator new` giving memory or 0.
fn prep_singletons(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    for s in SINGLETONS {
        let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
        t.global(s.global.sym, v);
    }
    let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.global(glue6::VFX_RENDER_MAP.sym, v);
    let p = if rng.chance(4) { 0 } else { obj + 0x100 };
    t.stub_ret(glue6::OPERATOR_NEW, false, 1, 0, p);
}

/// The task `(scene, a, b, list, reader, c, flag)` at `obj + 0x40`, the
/// list's count `[list]` equal to the reader's or not.
fn prep_asset_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let task = obj + 0x40;
    for k in [0, 1, 2, 4, 5] {
        let w = rng.next();
        t.w32(task + 4 * k, w);
    }
    t.w32(task + 0xc, obj + 0x100);
    let flag = if rng.chance(2) { 0 } else { rng.next() };
    t.w32(task + 0x18, flag);
    let count = rng.below(5);
    t.w32(obj + 0x100, count);
    let r = if rng.chance(2) { count } else { count + 1 };
    t.stub_ret(glue6::ASSET_LIST_COUNT, true, 0, 0, r);
    args[1] = task;
}

/// A scene at `obj + 0x1800` with its arrays `+0x60`, `+0x3c`, `+0x54`;
/// the task `(id, a, b, c)` at `obj + 0x40`.
fn prep_vfx_texture_loaded(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = obj + 0x1800;
    t.global(SCENE, scene);
    t.w32(scene + 0x60, obj + 0x1000);
    t.w32(scene + 0x3c, obj + 0x1100);
    t.w32(scene + 0x54, obj + 0x1200);
    t.w32(obj + 0x40, rng.below(16));
    for k in 1..4 {
        let w = rng.next();
        t.w32(obj + 0x40 + 4 * k, w);
    }
    args[1] = obj + 0x40;
}

/// The PS2 loader's list (count 0..4, data at `obj + 0x100`).
fn prep_ps2_list(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let m = rng.next();
    t.global(glue6::PS2_LOAD_MUTEX.sym, m);
    let list = t.rebase(glue6::PS2_LOAD_LIST);
    t.w32(list, rng.below(5));
    let cap = rng.next();
    t.w32(list + 4, cap);
    t.w32(list + 8, obj + 0x100);
}

/// A scene with the renderer (`+0xd0`) and the mesh list (`+0x3854`,
/// `+0x385c`); `beginScene` failing now and then.
fn prep_dtt(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x3860);
    *this = scene;
    t.w32(scene + 0xd0, obj + 0x100);
    let n = rng.below(5);
    t.w32(scene + 0x3854, n);
    t.w32(scene + 0x385c, obj + 0x300);
    for k in 0..n {
        let mesh = if rng.chance(4) {
            0
        } else {
            obj + 0x400 + 0x40 * k
        };
        t.w32(obj + 0x300 + 4 * k, mesh);
    }
    let r = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(glue6::RENDERER_BEGIN_SCENE, true, 1, 1, r);
}

/// A pool of `size`-byte slots in the scene (`base` field, used bytes,
/// lowest free) and an object with a recording destructor in it, or not.
fn pool_at(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    fields: [u32; 3],
    size: u32,
) {
    let scene = obj.wrapping_sub(0xf000);
    *this = scene;
    let object = obj + 0x300;
    let i = pick(rng, &[0, 5, 0xbb7, 0xbb8, 0xffff_ffff, 100]);
    let low = if rng.chance(3) { rng.below(size) } else { 0 };
    let base = if rng.chance(5) {
        // just above the object: the division rounds toward 0 (index 0)
        object + 1 + rng.below(size - 1)
    } else {
        object.wrapping_sub(size.wrapping_mul(i)).wrapping_sub(low)
    };
    t.w32(fields[0] - 0xf000 + obj, base);
    t.w32(fields[1] - 0xf000 + obj, obj + 0x1000);
    let free = pick(rng, &[0, 3, 6, 0xbb8, 0xffff_ffff]);
    t.w32(fields[2] - 0xf000 + obj, free);
    let r = rng.next();
    let f = t.stub(None, 0xd7, true, 1, 1, Some(r));
    t.w32(object, obj + 0x800);
    t.w32(obj + 0x800, f);
    args[0] = match rng.below(6) {
        0 => 0,
        1 => 0xffff_fffc,
        _ => object,
    };
    args[1] = if rng.chance(3) {
        rng.next()
    } else {
        scene + 0x12d4
    };
}

fn prep_pool56(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    pool_at(t, obj, this, args, rng, [0xfb90, 0xfb9c, 0xfba0], 56);
}

fn prep_pool28(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    pool_at(t, obj, this, args, rng, [0xfba4, 0xfbb0, 0xfbb4], 28);
}

/// The lookup's entries (`+4 + 4i`), each list empty or not, and the
/// cursor search finding an object or not.
fn prep_post_lookup(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let i = rng.below(4);
    args[0] = i;
    let entry = if rng.chance(5) { 0 } else { obj + 0x100 };
    t.w32(obj + 4 + 4 * i, entry);
    let list = obj + 0x11c;
    let first = if rng.chance(2) { list } else { obj + 0x400 };
    t.w32(list, first);
    let found = if rng.chance(3) { 0 } else { rng.next() | 1 };
    let object = rng.next();
    let base = rng.next();
    // a miss leaves the object fields as the port wrote them
    let writes: &[(u8, u32)] = if found == 0 && rng.chance(2) {
        &[(0xc, 0)]
    } else if found == 0 {
        &[(0xc, 0), (0x10, object | 1)]
    } else {
        &[(0xc, found), (0x10, object), (0, base)]
    };
    t.stub_this_out(glue6::CURSOR_SEEK3, 0, 0, None, writes);
}

/// A scene large enough for the mutex `+0xfb3c`, its array at `+0x1188`.
fn prep_regist_clusters(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb40);
    *this = scene;
    let m = rng.next();
    t.w32(scene + 0xfb3c, m);
    array_at(t, scene + 0x1188, obj + 0x400, 8, rng);
}

/// The Iggy player opened or not; the application at `obj + 0x400`.
fn prep_opening(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj.wrapping_sub(0x0001_0000);
    let player = if rng.chance(3) { 0 } else { obj + 0x200 };
    t.stub_ret(glue6::IGGY_PLAYER_OPEN, false, 5, 0, player);
    t.stub_ret(glue6::GET_APPLICATION, false, 0, 0, obj + 0x400);
    let b = rng.next() as u8;
    let at = t.rebase(glue6::OPENING_PLAYING.sym);
    t.write(at, &[b]);
}

/// The broken screen's mode and a colour at `obj + 0x80`.
fn prep_colour(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let mode = pick(rng, &[1, 2, 1, 2, 0, 3]);
    t.w32(obj, mode);
    for k in 0..4 {
        let f = if k == 3 && rng.chance(2) {
            // alphas around 1, where the clamp works
            [0.0f32, 0.5, 1.0, 1.5, 2.0, 0.999, 3.0][rng.below(7) as usize]
        } else {
            rng.float()
        };
        t.write(obj + 0x80 + 4 * k, &f.to_le_bytes());
    }
    args[0] = obj + 0x80;
}

/// A free list (`+0x18`) with a node or empty (refilled or not), the key
/// and value at `obj + 0x80`, the insert refusing now and then.
fn prep_tree_insert(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    insert: u32,
) {
    let tree = obj + 0x40;
    *this = tree;
    let node = obj + 0x200;
    t.w32(node, obj + 0x240);
    let head = if rng.chance(3) { 0 } else { node };
    t.w32(tree + 0x18, head);
    let refill = if rng.chance(2) { 0 } else { node };
    t.stub_this_out(glue6::FREE_LIST_ALLOCATE_BLOCK, 1, 1, None, &[(0, refill)]);
    let r = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(insert, true, 1, 1, r);
    args[0] = obj + 0x80;
    args[1] = obj + 0x84;
}

fn prep_tree_insert_64c020(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    prep_tree_insert(t, obj, this, args, rng, glue6::TREE_INSERT_64C020);
}

fn prep_tree_insert_6be960(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    prep_tree_insert(t, obj, this, args, rng, glue6::TREE_INSERT_6BE960);
}

/// The application at `obj + 0x400` (slot 10 a stub), its renderer at
/// `+0x124` (slot 2 a stub); `flip`, slot 2 and the window messages
/// failing or quitting now and then; the Iggy menus or none.
fn prep_render_opening(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let app = obj + 0x400;
    t.stub_ret(glue6::GET_APPLICATION, false, 0, 0, app);
    let s10 = t.stub(None, 0xa10, true, 0, 0, Some(1));
    t.w32(app, obj + 0x900);
    t.w32(obj + 0x900 + 40, s10);
    let renderer = app + 0x124;
    let r2 = if rng.chance(4) { rng.next() | 1 } else { 0 };
    let s2 = t.stub(None, 0xa02, true, 0, 0, Some(r2));
    t.w32(renderer, obj + 0xa00);
    t.w32(obj + 0xa08, s2);
    let flip = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(glue6::RENDERER_FLIP, true, 0, 0, flip);
    let quit = u32::from(rng.chance(3));
    t.stub_ret(glue6::PROCESS_WINDOWS_MESSAGE, false, 0, 0, quit);
    let menus = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.global(glue6::IGGY_MENU_MANAGER.sym, menus);
}

/// The two sampler descriptions: random bytes, the flag bit 7 set or not.
fn prep_samplers(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    for desc in [glue6::SAMPLER_DESC_A, glue6::SAMPLER_DESC_B] {
        let at = t.rebase(desc);
        let b = rng.bytes(0x24);
        t.write(at, &b);
        let flags = rng.next() & !0x80 | if rng.chance(2) { 0x80 } else { 0 };
        t.w32(at + 0x1c, flags);
    }
}

#[rustfmt::skip]
const CASES6: &[Case] = &[
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_render_data_table", 0x0069_f4c0, 0, &[(0x0069_e140, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::VFX_RENDER_DATA_TABLE_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_broken_screen", 0x006d_4d30, 0, &[(0x006d_4ba0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::BROKEN_SCREEN_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_free_camera", 0x0042_f260, 0, &[(0x0067_fe70, K::This(0))], |m, _, _| u(g6::free_camera_instance(m))) },
    Case { prep: Some(prep_singletons), ..cdecl("create_cluster_manager", 0x0069_a060, 0, &[(0x0069_9ab0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::CLUSTER_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_shader_manager", 0x006c_1420, 0, &[(0x006b_f4a0, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::VFX_SHADER_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_distortion", 0x006c_2190, 0, &[(0x006c_2010, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::DISTORTION_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_shadow_manager", 0x006c_9500, 0, &[(0x006c_7d80, K::This(0))], |m, _, _| { g6::create_singleton(m, glue6::SHADOW_MANAGER_SINGLETON); Out::None }) },
    Case { prep: Some(prep_singletons), ..cdecl("create_vfx_render_map", 0x0069_f440, 0, &[(glue6::MAP_BUY_HEAD_NODE, K::This(0))], |m, _, _| { g6::create_vfx_render_map(m); Out::None }) },
    Case { prep: Some(prep_asset_loaded), ..cdecl("scene_on_field_map_asset_loaded", 0x0066_6630, 2, &[(glue6::ASYNC_LOAD_INSTANCE, K::Getter), (glue6::SCENE_DO_ON_FIELD_MAP_ASSET_LOADED, K::This(7)), (glue6::SCENE_DO_ON_ALL_FIELD_MAP_ASSETS_LOADED, K::This(4))], |m, _, a| u(g6::scene_on_field_map_asset_loaded(m, a[0], a[1]))) },
    framed(Case { prep: Some(prep_vfx_texture_loaded), ..cdecl("scene_on_vfx_texture_loaded", 0x0066_6880, 2, &[(glue6::ASYNC_LOAD_INSTANCE, K::Getter), (glue6::GET_APPLICATION, K::Cdecl(0)), (glue6::FIXUP_CLUSTERS, K::Cdecl(2)), (glue6::SCENE_REGIST_VFX_TEXTURE, K::This(3))], |m, _, a| u(g6::scene_on_vfx_texture_loaded(m, frame_ebp(), a[0], a[1]))) }, 0, 4),
    Case { prep: Some(prep_ps2_list), ..cdecl("process_ps2_data_loading_callback", 0x0064_27f0, 0, &[(glue6::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue6::RELEASE_MUTEX, K::Import(1)), (glue6::SA_READ_CALLBACK, K::Cdecl(1)), (glue6::PS2_LOAD_LIST_RESET, K::This(1))], |m, _, _| u(g6::process_ps2_data_loading_callback(m))) },
    Case { prep: Some(prep_dtt), ..method("scene_render_dtt_vfx", 0x0065_31b0, This::Object, 0, &[(glue6::RENDER_MESH_INSTANCE, K::This(4)), (glue6::RENDERER_END_SCENE, K::This(1))], |m, this, _| u(g6::scene_render_dtt_vfx(m, this))) },
    method("sampler_holder_ctor", 0x006f_5ae0, This::Object, 1, &[(glue6::SAMPLER_STATE_CTOR, K::This(0))], |m, this, a| u(g6::sampler_holder_ctor(m, this, a[0]))),
    Case { prep: Some(prep_pool56), ..method("scene_delete_pool_object56", 0x0065_a430, This::Object, 2, &[(glue6::POOL56_DTOR, K::This(0)), (glue6::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g6::scene_delete_pool_object56(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_pool28), ..method("scene_delete_pool_object28", 0x0065_a1d0, This::Object, 2, &[(glue6::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| u(g6::scene_delete_pool_object28(m, this, a[0], a[1]))) },
    cdecl("level_set_name", 0x0064_1db0, 1, &[(glue6::HASH_U, K::Cdecl(1)), (glue6::IOB_FUNC, K::ImportC(0)), (glue6::FPRINTF, K::ImportC(3)), (glue6::POST_PROCESS_INSTANCE, K::Getter), (glue6::POST_SET_HDAO_SETTING, K::This(1)), (glue6::SCENE_SET_LEVEL_NAME_HASH, K::This(1)), (glue6::SCENE_SET_LEVEL_NAME, K::This(1))], |m, _, a| u(g6::level_set_name(m, a[0]))),
    framed(Case { prep: Some(prep_post_lookup), ..method("post_lookup", 0x0069_35f0, This::Object, 1, &[], |m, this, a| u(g6::post_lookup(m, frame_ebp(), this, a[0]))) }, 0x20, 0),
    Case { prep: Some(prep_regist_clusters), ..method("scene_regist_clusters_to_add", 0x0066_8550, This::Object, 2, &[(glue6::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue6::RELEASE_MUTEX, K::Import(1)), (glue6::ARRAY_RESERVE_668300, K::Keep(1))], |m, this, a| u(g6::scene_regist_clusters_to_add(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_opening), ..method("scene_begin_render_opening_screen", 0x0065_79d0, This::Object, 0, &[(glue6::IGGY_SET_CALLBACK, K::This(2)), (glue6::RENDERER_SET_CLEAR_COLOR, K::This(4))], |m, this, _| u(g6::scene_begin_render_opening_screen(m, this))) },
    Case { prep: Some(prep_colour), ..method("broken_set_colour", 0x006d_4620, This::Object, 1, &[], |m, this, a| { g6::broken_set_colour(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_tree_insert_64c020), ..method("tree_insert_new_0065fd50", 0x0065_fd50, This::Object, 2, &[(glue6::FREE_LIST_DEALLOCATE, K::This(1)), (glue6::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g6::tree_insert_new(m, glue6::TREE_INSERT_64C020, this, a[0], a[1]))) },
    Case { prep: Some(prep_tree_insert_6be960), ..method("tree_insert_new_006c0810", 0x006c_0810, This::Object, 2, &[(glue6::FREE_LIST_DEALLOCATE, K::This(1)), (glue6::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g6::tree_insert_new(m, glue6::TREE_INSERT_6BE960, this, a[0], a[1]))) },
    Case { prep: Some(prep_render_opening), ..method("scene_render_opening_screen", 0x0066_b2e0, This::Object, 0, &[(glue6::RENDERER_BEGIN_SCENE, K::This(1)), (glue6::RENDERER_END_SCENE, K::This(1)), (glue6::IGGY_MENU_RENDER, K::This(0)), (glue6::EXIT_PROCESS, K::Import(1)), (glue6::SLEEP_MS, K::Cdecl(1)), (glue6::SCENE_DO_DELETE_MESH_INSTANCES, K::This(0))], |m, this, _| u(g6::scene_render_opening_screen(m, this))) },
    Case { prep: Some(prep_samplers), ..cdecl("sampler_states_init", 0x006f_5470, 0, &[(glue6::SAMPLER_CREATE, K::This(0))], |m, _, _| u(g6::sampler_states_init(m))) },
];

#[test]
fn batch6_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 8 } else { 40 };
        run_cases(t, CASES6, n, &mut Rng(0x6006_b006));
    });
}

// ---- batch 7 (ffx_prender::glue7, round 3) ----

impl Twins {
    /// A recording stub at `sym` (cdecl, no arguments) that leaves the
    /// `double` `value` in `ST0`.
    fn stub_f64(&mut self, sym: u32, value: f64) {
        let cell = self.code(&value.to_bits().to_le_bytes());
        let log = self.stub(None, sym, false, 0, 0, Some(0));
        let mut c = vec![0xdd, 0x05]; // fld qword [cell]
        c.extend(cell.to_le_bytes());
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

/// A clock reading: seconds since start-up, 0, negative, huge, or any
/// double that is not a signalling NaN.
fn clock(rng: &mut Rng) -> f64 {
    loop {
        let v = match rng.below(6) {
            0 => f64::from(rng.below(1 << 20)) + f64::from(rng.next()) / 4_294_967_296.0,
            1 => 0.0,
            2 => -f64::from(rng.next()) / 1000.0,
            3 => [
                1e13,
                9.3e12,
                9.2233e12,
                -9.2233e12,
                1e300,
                f64::INFINITY,
                f64::NAN,
            ][rng.below(7) as usize],
            4 => f64::from(rng.below(100)) + 0.5,
            _ => f64::from_bits((u64::from(rng.next()) << 32) | u64::from(rng.next())),
        };
        let b = v.to_bits();
        let snan = b & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
            && b & 0x000f_ffff_ffff_ffff != 0
            && b & 0x0008_0000_0000_0000 == 0;
        if !snan {
            return v;
        }
    }
}

/// The timer stubbed with a clock reading, a random start time; the
/// application object `obj` (its `+0x28` paused time, a float).
fn prep_clock(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let now = clock(rng);
    t.stub_f64(glue7::TIMER_GET_TIME, now);
    let start = if rng.chance(3) {
        clock(rng)
    } else {
        now - f64::from(rng.below(5000))
    };
    let at = t.rebase(glue7::APP_START_TIME.sym);
    t.write(at, &start.to_bits().to_le_bytes());
    t.stub_ret(glue7::GET_APPLICATION, false, 0, 0, obj);
    let paused = rng.float();
    t.w32(obj + 0x28, paused.to_bits());
}

/// Floats in the two float slots, the effect 0 now and then, the frame
/// skipped now and then (on the snapshot too).
fn prep_ps2_lighting(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[4] = rng.float().to_bits();
    args[7] = rng.float().to_bits();
    args[0] = if rng.chance(5) { 0 } else { obj };
    let skip = if rng.chance(4) { rng.word() } else { 0 };
    t.global(glue7::GRAPHIC_SKIP_FRAME.sym, skip);
}

/// The manager `this` (high addresses too, for the sign words).
fn prep_shader_bins(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = if rng.chance(3) {
        0x8000_0000 | rng.next()
    } else {
        obj
    };
}

/// A name of 0 to 40 bytes at `obj + 0x600`, the new buffer at
/// `obj + 0x1000` (prefilled), `this` and the value any word.
fn prep_async(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    name_arg: usize,
) {
    let len = rng.below(41) as usize;
    let mut name: Vec<u8> = (0..len).map(|_| (rng.next() % 255 + 1) as u8).collect();
    name.push(0);
    t.write(obj + 0x600, &name);
    args[name_arg] = obj + 0x600;
    let fill = rng.bytes(64);
    t.write(obj + 0x1000, &fill);
    t.stub_ret(glue7::OPERATOR_NEW_6304C0, false, 1, 0, obj + 0x1000);
    *this = if rng.chance(3) {
        0x8000_0000 | rng.next()
    } else {
        obj
    };
    args[1 - name_arg] = rng.word();
}

fn prep_async_name0(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_async(t, obj, this, args, rng, 0);
}

fn prep_async_name1(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_async(t, obj, this, args, rng, 1);
}

#[rustfmt::skip]
const CASES7: &[Case] = &[
    Case { prep: Some(prep_clock), ..cdecl("get_time", 0x0064_1410, 0, &[], |m, _, _| Out::F64(g7::get_time(m))) },
    Case { prep: Some(prep_clock), ..cdecl("get_time_in_microseconds", 0x0064_1420, 0, &[], |m, _, _| Out::U64(g7::get_time_in_microseconds(m))) },
    Case { prep: Some(prep_ps2_lighting), ..cdecl("ps2_lighting_setup_effect", 0x0064_2750, 11, &[(glue7::REGISTER_POINT_LIGHT, K::Cdecl(10))], |m, _, a| u(g7::ps2_lighting_setup_effect(m, arr(a)))) },
    Case { prep: Some(prep_shader_bins), ..method("post_load_shader_bin_files", 0x0069_3d20, This::Object, 0, &[(glue7::ASYNC_LOAD_INSTANCE, K::Getter), (glue7::LOAD_CLUSTER, K::Cdecl(9)), (glue7::WAIT_TASKS, K::This(0))], |m, this, _| u(g7::post_load_shader_bin_files(m, this))) },
    Case { prep: Some(prep_async_name0), ..method("scene_load_character_async", 0x0066_42f0, This::Object, 2, &[(glue7::ASYNC_LOAD_INSTANCE, K::Getter), (glue7::REGIST_ASYNC_LOAD_TASK, K::Cdecl(9)), (glue7::ASYNC_LOAD_NEW_CHARACTER, K::Cdecl(0))], |m, this, a| u(g7::scene_load_character_async(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_async_name1), ..method("scene_load_magic_file_vfx_texture_async", 0x0066_4d10, This::Object, 2, &[(glue7::ASYNC_LOAD_INSTANCE, K::Getter), (glue7::REGIST_ASYNC_LOAD_TASK, K::Cdecl(9))], |m, this, a| u(g7::scene_load_magic_file_vfx_texture_async(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_async_name1), ..method("scene_load_vfx_texture_async", 0x0066_5f60, This::Object, 2, &[(glue7::ASYNC_LOAD_INSTANCE, K::Getter), (glue7::REGIST_ASYNC_LOAD_TASK, K::Cdecl(9))], |m, this, a| u(g7::scene_load_vfx_texture_async(m, this, a[0], a[1]))) },
];

#[test]
fn batch7_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 12 } else { 60 };
        run_cases(t, CASES7, n, &mut Rng(0x7007_b007));
    });
}

#[test]
fn batch7_ftol2_64_matches_the_original() {
    // _ftol2 (0x00949816) on the x87 values the clock gives it
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x0f70_1264);
    let ftol2 = t.rebase(0x0094_9816);
    // fld qword [esp+4]; call f; ret
    let cell = t.zeros(8);
    let mut c = vec![0xdd, 0x05];
    c.extend(cell.to_le_bytes());
    let at = t.cave_next;
    c.push(0xe8);
    c.extend(ftol2.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    c.push(0xc3);
    let code = t.code(&c);
    for i in 0..4000 {
        let x = match i {
            0..=20 => [
                0.0,
                -0.0,
                0.4,
                0.5,
                0.6,
                -0.4,
                -0.5,
                -0.6,
                1.5,
                -1.5,
                2.5,
                -2.5,
                4_294_967_296.0,
                -4_294_967_296.0,
                4_294_967_295.5,
                -4_294_967_296.5,
                9.2e18,
                -9_223_372_036_854_775_808.0,
                9_223_372_036_854_775_808.0,
                1e300,
                f64::NAN,
            ][i],
            _ => clock(&mut rng) * if rng.chance(2) { 1e6 } else { 1.0 },
        };
        t.write(cell, &x.to_bits().to_le_bytes());
        let r = t.a.call(code, Conv::Cdecl, &[]).unwrap();
        let got = (u64::from(r.edx) << 32) | u64::from(r.eax);
        assert_eq!(got, glue7::ftol2_64(x), "{x:e}");
    }
}

// ---- batch 8 (ffx_prender::glue8, round 3) ----

const fn with_cookie(case: Case) -> Case {
    Case {
        cookie: true,
        ..case
    }
}

/// A scene with its release slots: counts, objects and clusters.
fn prep_release_objects(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb40);
    *this = scene;
    let n = rng.below(7);
    t.w32(scene + 0xfb24, n);
    t.w32(scene + 0xfb20, obj + 0x100);
    t.w32(scene + 0xfb2c, obj + 0x200);
    t.w32(scene + 0xfb38, obj + 0x300);
    for k in 0..n {
        let count = rng.below(4) as i32 - 1;
        t.w32(obj + 0x100 + 4 * k, count as u32);
        let object = if rng.chance(3) {
            0
        } else {
            obj + 0x800 + 0x10 * k
        };
        t.w32(obj + 0x200 + 4 * k, object);
        let cluster = rng.next();
        t.w32(obj + 0x300 + 4 * k, cluster);
    }
}

fn prep_special_id(_: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let listed = glue8::LISTED_65EFD0[rng.below(8) as usize];
    match rng.below(4) {
        0 | 1 => args[0] = listed,
        2 => args[0] = listed ^ (1 << rng.below(32)),
        _ => {}
    }
}

/// A pool array: the scene's own cluster or another, a slot in and out
/// of range, owned or borrowed storage.
fn prep_pool_array(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb90);
    *this = scene;
    let array = if rng.chance(8) { 0 } else { obj + 0x600 };
    let slot = match rng.below(4) {
        0 => rng.below(3000),
        1 => 2998 + rng.below(4),
        2 => (rng.below(4) as i32 - 2) as u32,
        _ => rng.next() >> 3,
    };
    let skew = if rng.chance(4) { rng.below(8) } else { 0 };
    t.w32(
        scene + 0xfb7c,
        array.wrapping_sub(slot.wrapping_mul(8)).wrapping_sub(skew),
    );
    t.w32(scene + 0xfb88, (obj + 0x1000).wrapping_sub(slot));
    let low = rng.below(3005) as i32 - 2;
    t.w32(scene + 0xfb8c, low as u32);
    if array != 0 {
        let flag = [0, 0x8000_0000][rng.below(2) as usize];
        let n = rng.below(4);
        t.w32(array, flag | n);
        let data = rng.next();
        t.w32(array + 4, data);
    }
    args[0] = array;
    // a null array of the scene's own pool would be read: never passed
    args[1] = if array == 0 || rng.chance(4) {
        rng.next() | 1
    } else {
        scene + 0x12d4
    };
}

/// The pool table: next free slot near the end or anywhere, segments and
/// blocks placed so the chosen slot lands in the object, a run of used
/// bytes after it.
fn prep_segment_pool(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let next = match rng.below(4) {
        0 => 3990 + rng.below(12),
        1 => rng.below(64),
        2 => (rng.below(3) as i32 - 3) as u32,
        _ => rng.below(4000),
    };
    t.w32(obj + 0x48, next);
    t.w32(
        obj + 0x3c,
        (obj + 0x400).wrapping_sub(next.wrapping_mul(0x6c)),
    );
    t.w32(obj + 0x40, (obj + 0x600).wrapping_sub(next << 8));
    t.w32(obj + 0x44, (obj + 0x800).wrapping_sub(next));
    let run = rng.below(12) as usize;
    let mut used = vec![1u8; run];
    used.push(0);
    t.write(obj + 0x801, &used);
    args[0] = if rng.chance(8) { 0 } else { obj + 0x20 };
    args[1] = if rng.chance(8) { 0 } else { obj + 0x28 };
}

fn prep_clear_arrays(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb80);
    *this = scene;
    for k in 0..14 {
        let w = if rng.chance(2) {
            0x8000_0000 | rng.next()
        } else {
            rng.word()
        };
        t.w32(scene + 0xfb48 + 4 * k, w);
    }
}

fn prep_cameras(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0da0);
    *this = scene;
    for (k, off) in [0xdc, 0x654, 0x238].into_iter().enumerate() {
        let c = if rng.chance(4) {
            0
        } else {
            obj + 0x100 + 0x100 * k as u32
        };
        t.w32(scene + off, c);
    }
    let mode = if rng.chance(2) { 1 } else { rng.word() };
    t.w32(scene + 0x1_0d8c, mode);
}

/// A binary search tree under the head `obj` (leaves point at the head),
/// the node to insert at `obj + 0x1000` with a key that is there or not.
fn prep_tree_link(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let head = obj;
    *this = head;
    t.w32(head + 0xc, head);
    let n = rng.below(12);
    let mut keys = vec![];
    for k in 0..n {
        let node = obj + 0x100 + 0x20 * k;
        let key = if rng.chance(3) {
            rng.next()
        } else {
            rng.below(40)
        };
        if keys.contains(&key) {
            continue;
        }
        keys.push(key);
        t.w32(node, head);
        t.w32(node + 4, head);
        let colour = rng.next();
        t.w32(node + 8, colour);
        t.w32(node + 0xc, key);
        // linked at its search-tree position
        let mut slot = head + 0xc;
        let mut at = t.r32(slot);
        while at != head {
            let here = t.r32(at + 0xc);
            slot = if here > key { at } else { at + 4 };
            at = t.r32(slot);
        }
        t.w32(slot, node);
    }
    let node = obj + 0x1000;
    let key = if !keys.is_empty() && rng.chance(3) {
        keys[rng.below(keys.len() as u32) as usize]
    } else {
        rng.below(40)
    };
    let colour = rng.next();
    t.w32(node + 8, colour);
    t.w32(node + 0xc, key);
    args[0] = node;
}

fn prep_broken_render(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for k in 0..4 {
        let f = rng.float().to_bits();
        t.w32(obj + 0x30 + 4 * k, f);
    }
    t.w32(obj + 0xc, obj + 0x400);
    t.w32(obj + 0x24, obj + 0x600);
}

fn prep_sampler_pair(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for off in [0x30, 0x54] {
        let w = rng.next();
        t.w32(obj + off, w);
    }
}

/// A character (cloth flag at `+0x1c`), the allocations (or 0), the
/// pending array.
fn prep_regist_instantiate(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[1] = if rng.chance(6) { 0 } else { obj + 0x100 };
    let flag = if rng.chance(2) {
        0
    } else {
        rng.below(255) as u8 + 1
    };
    t.write(obj + 0x11c, &[flag]);
    let loading = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(glue8::CH_IS_LOADING_CLOTH, false, 1, 0, loading);
    let p = if rng.chance(6) { 0 } else { obj + 0x500 };
    t.stub_ret(glue8::OPERATOR_NEW, false, 1, 0, p);
    let buffer = if rng.chance(6) { 0 } else { obj + 0x700 };
    t.stub_ret(glue8::PHYRE_MALLOC, false, 2, 0, buffer);
    let array = t.rebase(glue8::CHARACTER_LOADING);
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(array, flag | rng.below(5));
    t.w32(array + 4, obj + 0x900);
}

impl Twins {
    /// A cdecl import stub for `strstr` (two words, logged) returning
    /// `first`, then `first + step` on the next call.
    fn stub_strstr(&mut self, slot: u32, first: u32, step: u32) {
        let cell = self.code(&first.to_le_bytes());
        let log = self.stub(None, slot, false, 2, 0, Some(0));
        // push a1; push a0
        let mut c = vec![0xff, 0x74, 0x24, 0x08, 0xff, 0x74, 0x24, 0x08];
        let at = self.cave_next;
        c.push(0xe8); // call log
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        c.extend([0x83, 0xc4, 0x08]); // add esp, 8
        c.push(0xa1); // mov eax, [cell]
        c.extend(cell.to_le_bytes());
        c.extend([0x81, 0x05]); // add dword [cell], step
        c.extend(cell.to_le_bytes());
        c.extend(step.to_le_bytes());
        c.push(0xc3);
        let code = self.code(&c);
        let at = self.rebase(slot);
        self.patch(at, &code.to_le_bytes());
    }
}

/// The path (or 0), the marker hits: none, the first only, or both with a
/// name of 0 to 29 bytes between them (30 or more ends the process: not
/// run).
fn prep_level_analysis(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = if rng.chance(6) { 0 } else { obj + 0x100 };
    let start = obj + 0x140 + rng.below(16);
    let (first, step) = match rng.below(4) {
        0 => (0, 0),
        1 => (start, start.wrapping_neg()),
        _ => (start, 4 + rng.below(30)),
    };
    t.stub_strstr(glue8::STRSTR, first, step);
}

fn prep_init_post(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0de0);
    *this = scene;
    for off in [0xcc, 0x10cc, 0x1_0dd0, 0x1_0dd4] {
        let w = rng.next();
        t.w32(scene + off, w);
    }
    let screen = rng.next();
    t.global(glue8::LOADING_SCREEN.sym, screen);
    let r = if rng.chance(2) { 0 } else { rng.next() };
    t.stub_ret(glue8::POST_SET_FIELDS_12400, true, 4, 4, r);
}

/// An effect's geometry (or none), its render data's part count, a
/// vtable whose `+0x10` logs.
fn prep_vfx_destroy(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(6) {
        args[0] = 0;
        args[1] = 0;
    }
    if rng.chance(3) {
        args[0] = 0u32.wrapping_sub(rng.below(20000));
    }
    t.global(glue8::VFX_RENDER_DATA_TABLE.sym, obj + 0x1800);
    let geo = if rng.chance(5) { 0 } else { obj + 0x400 };
    t.stub_ret(glue8::VFX_GET_DYN_GEO, true, 2, 2, geo);
    let ret = rng.next();
    let virt = t.stub(None, 0x0010_0010, true, 0, 0, Some(ret));
    t.w32(obj + 0x400, obj + 0x500);
    t.w32(obj + 0x510, virt);
    let data = if rng.chance(4) { 0 } else { obj + 0x600 };
    t.w32(obj + 0x424, data);
    let on = if rng.chance(3) {
        0
    } else {
        rng.below(255) as u8 + 1
    };
    t.write(obj + 0x6c0, &[on]);
    let parts = match rng.below(6) {
        0 => 0,
        1 => 0x100 + rng.below(2),
        _ => rng.below(6),
    } as u16;
    t.write(obj + 0x6c4, &parts.to_le_bytes());
}

/// Shader bin `i`'s cluster (or 0) and the cursor's verdict.
fn prep_check_bin(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let i = rng.below(14);
    args[0] = i;
    let cluster = if rng.chance(5) { 0 } else { obj + 0x100 };
    t.w32(obj + 4 + 4 * i, cluster);
    let list = obj + 0x11c;
    let first = if rng.chance(2) { list } else { obj + 0x400 };
    t.w32(list, first);
    let found = if rng.chance(3) { 0 } else { rng.next() | 1 };
    let object = if rng.chance(4) { 0 } else { rng.next() };
    let base = if object == 0 && rng.chance(2) {
        0
    } else {
        rng.next()
    };
    let writes: &[(u8, u32)] = if found == 0 {
        &[(0xc, 0)]
    } else {
        &[(0xc, found), (0x10, object), (0, base)]
    };
    t.stub_this_out(glue8::CURSOR_SEEK3, 0, 0, None, writes);
}

fn prep_persistent(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let r = if rng.chance(2) { 0 } else { rng.word() };
    t.stub_ret(glue8::SCENE_LOAD_VFX_TEXTURE, true, 3, 3, r);
}

/// The cloth table: a few characters, ready or not; batches of 1 to 3.
fn prep_cloth(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let n = rng.below(7);
    t.global(glue8::CLOTH_COUNT.sym, n);
    let table = t.rebase(glue8::CLOTH_TABLE);
    for k in 0..n {
        let c = obj + 0x100 + 0x20 * k;
        let entry = if rng.chance(6) { 0 } else { c };
        t.w32(table + 4 * k, entry);
        let set = u32::from(!rng.chance(5));
        t.w32(c, set);
        let busy = u32::from(rng.chance(5));
        t.w32(c + 8, busy);
        let state = [2, 2, 2, 1, 0, 3][rng.below(6) as usize];
        t.w32(c + 0xc, state);
    }
    let batch = 1 + rng.below(3);
    t.stub_ret(glue8::EIGHTH_AT_LEAST_ONE, false, 2, 2, batch);
    // the batch call may change the count (the loop rereads it)
    if rng.chance(2) {
        let count = t.rebase(glue8::CLOTH_COUNT.sym);
        let fewer = rng.below(n + 1);
        let log = t.stub(
            None,
            glue8::BATCH_SET_CLOTH_VERTEX_BUFFERS,
            false,
            3,
            0,
            Some(0),
        );
        let mut c = vec![0xc7, 0x05]; // mov dword [count], fewer
        c.extend(count.to_le_bytes());
        c.extend(fewer.to_le_bytes());
        let at = t.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = t.code(&c);
        let site = t.rebase(glue8::BATCH_SET_CLOTH_VERTEX_BUFFERS);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        t.patch(site, &j);
    }
}

#[rustfmt::skip]
const CASES8: &[Case] = &[
    Case { prep: Some(prep_release_objects), ..method("scene_release_cluster_objects", 0x0065_8f40, This::Object, 0, &[(glue8::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, _| { g8::scene_release_cluster_objects(m, this); Out::None }) },
    Case { prep: Some(prep_special_id), ..method("is_special_magic_id", 0x0065_efd0, This::Stdcall, 1, &[], |_, _, a| u(g8::is_listed_65efd0(a[0]))) },
    Case { prep: Some(prep_pool_array), ..method("scene_release_pool_array", 0x0065_a3a0, This::Object, 2, &[(glue8::PHYRE_FREE, K::Cdecl(1)), (glue8::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| { g8::scene_release_pool_array(m, this, a[0], a[1]); Out::None }) },
    Case { prep: Some(prep_segment_pool), ..method("vfx_segment_from_pool", 0x0069_f7f0, This::Object, 2, &[], |m, this, a| u(g8::vfx_segment_from_pool(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_clear_arrays), ..method("scene_clear_arrays", 0x0065_9000, This::Object, 0, &[(glue8::ARRAY_RESERVE_667F80, K::This(1)), (glue8::ARRAY_RESERVE_668280, K::This(1))], |m, this, _| u(g8::scene_clear_arrays(m, this))) },
    Case { prep: Some(prep_cameras), ..method("scene_update_cameras", 0x0067_1be0, This::Object, 0, &[(glue8::CAMERA_UPDATE_VIEW, K::This(0))], |m, this, _| { g8::scene_update_cameras(m, this); Out::None }) },
    Case { prep: Some(prep_tree_link), ..method("tree_insert_6be960", 0x006b_e960, This::Object, 1, &[], |m, this, a| u(g8::tree_find_or_link(m, this, a[0]))) },
    Case { prep: Some(prep_broken_render), ..method("broken_render", 0x006d_44c0, This::Object, 0, &[(glue8::BROKEN_SET_CONSTANT_A0, K::This(4)), (glue8::BROKEN_SET_CONSTANT_A4, K::This(4)), (glue8::RENDER_MESH_INSTANCE, K::This(4))], |m, this, _| u(g8::broken_render(m, this))) },
    Case { prep: Some(prep_sampler_pair), ..method("sampler_pair_init", 0x006b_3f40, This::Object, 0, &[(glue8::SAMPLER_CREATE, K::This(0))], |m, this, _| u(g8::sampler_pair_init(m, this))) },
    framed(Case { prep: Some(prep_regist_instantiate), ..cdecl("character_regist_to_instantiate", 0x0063_d370, 4, &[(glue8::CHR_REGIST_HELPER, K::Cdecl(4)), (glue8::PHYRE_FREE, K::Cdecl(1))], |m, _, a| { let slots = frame_ebp() + 8; for (k, &w) in a.iter().enumerate() { m.write(slots + 4 * k as u32, w); } g8::character_regist_to_instantiate(m, slots, a[0], a[1], a[2], a[3]).map_or(Out::None, u) }) }, 0, 16),
    with_cookie(framed(Case { prep: Some(prep_level_analysis), ..cdecl("level_analysis_name", 0x0064_1d00, 1, &[(glue8::STRNCPY, K::ImportC(3)), (glue8::LEVEL_SET_NAME, K::Cdecl(1))], |m, _, a| { g8::level_analysis_name(m, frame_ebp(), a[0]); Out::None }) }, 0x24, 0)),
    Case { prep: Some(prep_init_post), ..method("scene_init_post_process", 0x0066_56e0, This::Object, 0, &[(glue8::SCENE_652910, K::This(0)), (glue8::INSTANCE_6DC800, K::Getter), (glue8::INSTANCE_6DDBF0, K::This(0)), (glue8::GET_APPLICATION, K::Cdecl(0)), (glue8::POST_PROCESS_INSTANCE, K::Getter), (glue8::POST_SET_BYTE_123B3, K::This(1)), (glue8::POST_SET_BYTE_123B0, K::This(1)), (glue8::TWEAK_INSTANCE, K::Getter), (glue8::TWEAK_INIT, K::This(1)), (glue8::SCENE_LOAD_CUTBUFFER_SHADER, K::This(0)), (glue8::LOADING_SCREEN_INITIALIZE, K::This(0))], |m, this, _| u(g8::scene_init_post_process(m, this))) },
    Case { prep: Some(prep_vfx_destroy), ..cdecl("vfx_destroy", 0x0064_5280, 2, &[(glue8::VFX_REMOVE, K::This(2))], |m, _, a| { g8::vfx_destroy(m, a[0], a[1]); Out::None }) },
    framed(Case { prep: Some(prep_check_bin), ..method("post_check_shader_bin", 0x0069_3540, This::Object, 1, &[(glue8::GET_APPLICATION, K::Cdecl(0)), (glue8::FIXUP_CLUSTERS, K::Cdecl(2))], |m, this, a| u(g8::post_check_shader_bin(m, frame_ebp(), this, a[0]))) }, 0x20, 0),
    with_cookie(framed(Case { prep: Some(prep_persistent), ..method("scene_load_persistent_vfx_texture", 0x0066_5660, This::Object, 0, &[(glue8::FIOS_UNIFY_FILENAME, K::Cdecl(3))], |m, this, _| u(g8::scene_load_persistent_vfx_texture(m, frame_ebp(), this))) }, 0x204, 0)),
    framed(Case { prep: Some(prep_cloth), ..cdecl("character_set_cloth_vertex_buffers", 0x0063_b740, 0, &[(glue8::BATCH_SET_CLOTH_VERTEX_BUFFERS, K::Cdecl(3))], |m, _, _| { g8::character_set_cloth_vertex_buffers(m, frame_ebp()); Out::None }) }, 8, 0),
];

#[test]
fn batch8_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 12 } else { 60 };
        run_cases(t, CASES8, n, &mut Rng(0x8008_b008));
    });
}

/// The `PArray_PushInto` copies.
const PUSH_INTO_COPIES: [u32; 8] = [
    0x0063_7e30,
    0x0063_7ec0,
    0x0065_fac0,
    0x0065_fb50,
    0x006b_24e0,
    0x006b_2570,
    0x006d_26d0,
    0x006d_ea60,
];

#[test]
fn batch8_push_into_copies_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x9008_b008);
        for i in 0..24 {
            for sym in PUSH_INTO_COPIES {
                t.reset();
                let at = t.rebase(glue8::PHYRE_FREE);
                let ret = rng.next();
                t.stub(Some(at), glue8::PHYRE_FREE, false, 1, 0, Some(ret));
                let arr = t.zeros(8);
                let n = rng.below(6);
                let old = t.zeros(0x20);
                let fill = rng.bytes(0x20);
                t.write(old, &fill);
                let flag = [0, 0x8000_0000][rng.below(2) as usize];
                t.w32(arr, n | flag);
                t.w32(arr + 4, old);
                let fresh = t.zeros(0x20);
                let dst = [0, fresh, old][rng.below(3) as usize];
                let value = t.zeros(4);
                let v = rng.next();
                t.w32(value, v);
                t.check(
                    &format!("push {sym:#x} #{i}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[dst, value],
                    |g| {
                        u(ffx_system::container::bridge::parray_push_into(
                            g,
                            arr,
                            dst,
                            value,
                            ffx_glue_other::copies::phyre_free,
                        ))
                    },
                );
            }
        }
    });
}

// ---- batch 9 (ffx_prender::glue9, round 3) ----

/// A mesh instance of the scene's pool (slot in or out of range, the
/// division's remainder and sign) or of another cluster, with or without
/// its two parts.
fn prep_delete_pmesh(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfbe8);
    *this = scene;
    let mesh = obj + 0x600;
    let (slot, negative) = match rng.below(5) {
        0 => (rng.below(3000) as i32, false),
        1 => (2998 + rng.below(4) as i32, false),
        2 => (-(rng.below(3) as i32) - 1, true),
        // slot 0 from a negative difference: the division truncates toward 0
        3 => (0, true),
        _ => (rng.next() as i32 >> 8, false),
    };
    let negative = negative || slot < 0;
    let rest = (1 + rng.below(131)) as i32 * if negative { -1 } else { 1 };
    let diff = slot.wrapping_mul(132).wrapping_add(rest);
    t.w32(scene + 0xfbcc, mesh.wrapping_sub(diff as u32));
    let s = diff / 132;
    t.w32(scene + 0xfbe0, (obj + 0x1000).wrapping_sub(s as u32));
    let low = rng.below(3005) as i32 - 2;
    t.w32(scene + 0xfbe4, low as u32);
    for off in [4, 0x1c] {
        let p = if rng.chance(3) { 0 } else { rng.next() };
        t.w32(mesh + off, p);
    }
    args[0] = mesh;
    args[1] = if rng.chance(4) {
        rng.next() | 1
    } else {
        scene + 0x12d4
    };
}

fn prep_delete_meshes(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x38a4);
    *this = scene;
    let n = rng.below(7) as i32 - 1;
    t.w32(scene + 0x38a0, n as u32);
    t.w32(scene + 0x3894, obj + 0x100);
    t.w32(scene + 0x389c, obj + 0x200);
    for k in 0..8 {
        let mesh = if rng.chance(3) { 0 } else { rng.next() };
        t.w32(obj + 0x100 + 4 * k, mesh);
    }
    for off in [0x3890, 0x3898] {
        let flag = [0, 0x8000_0000][rng.below(2) as usize];
        t.w32(scene + off, flag | rng.below(9));
    }
}

fn prep_all_assets(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1198);
    *this = scene;
    let d8 = rng.next();
    t.w32(scene + 0xd8, d8);
    t.write(
        scene + 0x10d0,
        &[u8::from(rng.chance(2)), u8::from(rng.chance(2))],
    );
    for off in [0x10d8, 0x112c] {
        let w = if rng.chance(2) { 0 } else { rng.next() };
        t.w32(scene + off, w);
    }
    args[2] = obj + 0x100;
    let clusters = rng.next();
    t.w32(obj + 0x108, clusters);
    let r = if rng.chance(2) { 0 } else { rng.word() };
    t.stub_ret(glue9::FIXUP_CLUSTERS, false, 2, 0, r);
}

fn prep_geometry_pool(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let next = match rng.below(4) {
        0 => 2990 + rng.below(12),
        1 => rng.below(64),
        2 => (rng.below(3) as i32 - 3) as u32,
        _ => rng.below(3000),
    };
    t.w32(obj + 0x38, next);
    t.w32(
        obj + 0x2c,
        (obj + 0x400).wrapping_sub(next.wrapping_mul(0xd0)),
    );
    let geo = if rng.chance(4) { 0 } else { obj + 0x500 };
    t.w32(obj + 0x30, geo.wrapping_sub(next.wrapping_mul(0xf8)));
    t.w32(obj + 0x34, (obj + 0x900).wrapping_sub(next));
    let run = rng.below(12) as usize;
    let mut used = vec![1u8; run];
    used.push(0);
    t.write(obj + 0x901, &used);
    args[0] = if rng.chance(8) { 0 } else { obj + 0x20 };
    args[1] = if rng.chance(8) { 0 } else { obj + 0x28 };
}

/// Two parallel arrays (words, 20-byte partners) with a capacity; the
/// value present or not.
fn prep_parallel(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(7);
    let begin = obj + 0x100;
    let begin2 = obj + 0x300;
    t.w32(obj, begin);
    t.w32(obj + 4, begin2);
    t.w32(obj + 8, begin + 4 * n);
    let extra = if rng.chance(4) {
        0x14 * rng.below(2)
    } else {
        0
    };
    t.w32(obj + 0xc, begin2 + 0x14 * n + extra);
    let cap = (n as i32 + rng.below(3) as i32 - 1) as u32;
    t.w32(obj + 0x10, cap);
    let mut words = vec![];
    for k in 0..=n {
        let w = rng.below(10);
        words.push(w);
        t.w32(begin + 4 * k, w);
    }
    let fill = rng.bytes(0x14 * 8);
    t.write(begin2, &fill);
    args[0] = if n > 0 && rng.chance(2) {
        words[rng.below(n) as usize]
    } else {
        rng.below(12)
    };
}

fn prep_set_vertex(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.w32(obj + 0x40, obj + 0x400);
    t.w32(obj + 0x44, obj + 0x500);
    t.w32(obj + 0x4c, obj + 0x600);
    args[0] = rng.below(5);
    for k in 0..8 {
        let at = obj + 0x100 + 4 * k as u32;
        let f = rng.float().to_bits();
        t.w32(at, f);
        args[1 + k] = if rng.chance(4) { 0 } else { at };
    }
}

fn prep_insert_array(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let node = obj + 0x400;
    let next = rng.next();
    t.w32(node, next);
    if rng.chance(3) {
        t.w32(obj + 0x18, 0);
        let made = if rng.chance(3) { 0 } else { node };
        t.stub_this_out(glue9::FREE_LIST_ALLOCATE_BLOCK, 1, 1, None, &[(0, made)]);
    } else {
        t.w32(obj + 0x18, node);
    }
    args[0] = obj + 0x40;
    args[1] = obj + 0x50;
    let data = if rng.chance(3) { 0 } else { rng.next() };
    t.stub_this_out(glue9::PARRAY_ASSIGN, 1, 1, None, &[(8, data)]);
    let dup = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(glue9::TREE_INSERT_64BFA0, true, 1, 1, dup);
}

/// A scene pool's counter (near the end, past it, small), the slot's
/// address in the object or 0, a run of used bytes after it.
fn prep_pool_claim(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
    offsets: (u32, u32, u32),
    stride: u32,
) {
    let (counter, base, used) = offsets;
    let scene = t.zeros(0xfc00);
    *this = scene;
    let s = match rng.below(4) {
        0 => 2990 + rng.below(12),
        1 => rng.below(64),
        2 => (rng.below(3) as i32 - 3) as u32,
        _ => rng.below(3000),
    };
    t.w32(scene + counter, s);
    let p = if rng.chance(5) { 0 } else { obj + 0x600 };
    t.w32(scene + base, p.wrapping_sub(s.wrapping_mul(stride)));
    t.w32(scene + used, (obj + 0x1000).wrapping_sub(s));
    let run = rng.below(12) as usize;
    let mut bytes = vec![1u8; run];
    bytes.push(0);
    t.write(obj + 0x1001, &bytes);
    args[0] = if rng.chance(4) {
        rng.next() | 1
    } else {
        scene + 0x12d4
    };
}

fn prep_claim8(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_pool_claim(t, obj, this, args, rng, (0xfb8c, 0xfb7c, 0xfb88), 8);
}

fn prep_claim56(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_pool_claim(t, obj, this, args, rng, (0xfba0, 0xfb90, 0xfb9c), 56);
}

fn prep_capture(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x10d0);
    *this = scene;
    let r = rng.next();
    t.w32(scene + 0xd0, r);
    t.w32(scene + 0x10cc, obj + 0x200);
    t.w32(obj + 0x2ac, obj + 0x300);
    let flag = u8::from(rng.chance(3));
    t.write(obj + 0x304, &[flag]);
    t.stub_ret(glue9::GET_APPLICATION, false, 0, 0, obj);
}

fn prep_chr_relation(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let handle = if rng.chance(5) {
        u32::MAX
    } else {
        rng.below(4)
    };
    t.stub_ret(glue9::MAGIC_FILE_HANDLE, false, 0, 0, handle);
    let id = [0x9a, 0x1a6, 0x9b, rng.next()][rng.below(4) as usize];
    t.stub_ret(glue9::MAGIC_FILE_ID, false, 0, 0, id);
    let scene = t.zeros(0xfb48);
    t.global(SCENE, scene);
    t.write(scene + 0xfb45, &[u8::from(rng.chance(4))]);
    for g in glue9::CHR_RELATION_BUFFERS {
        let p = if rng.chance(3) { 0 } else { rng.next() };
        t.global(g.sym, p);
    }
    let w = rng.next();
    t.global(glue9::CHR_RELATION_WORD.sym, w);
    let _ = obj;
}

fn prep_map_vfx(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfaf4);
    *this = scene;
    t.write(scene + 0x25, &[u8::from(rng.chance(4))]);
    let map = [0, 1, rng.below(400), rng.next()][rng.below(4) as usize];
    t.w32(scene + 0xfaf0, map);
}

fn prep_texture_layout(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x50);
    t.global(SCENE, scene);
    t.w32(scene + 0x48, obj + 0x400);
    args[0] = rng.below(8);
    args[1] = (rng.below(6) as i32 - 1) as u32;
    args[2] = obj + 0x100;
}

impl Twins {
    /// A recording successor stub (thiscall, `ret 4`) returning the
    /// node's `+0x18` word.
    fn stub_successor(&mut self, sym: u32) {
        let log = self.stub(None, sym, true, 1, 0, Some(0));
        // push [esp+4]; call log; add esp, 4; mov eax, [esp+4]; mov eax, [eax+0x18]; ret 4
        let mut c = vec![0xff, 0x74, 0x24, 0x04];
        let at = self.cave_next;
        c.push(0xe8);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        c.extend([
            0x83, 0xc4, 0x04, 0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x18, 0xc2, 0x04, 0x00,
        ]);
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// A tree of named entries: a left chain to the leftmost node, in-order
/// links in `+0x18`, keys (strings, colour bit set now and then) and
/// values; the name one of the keys (another copy) or not.
fn prep_find_named(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0d64);
    *this = scene;
    let head = scene + 0x1_0d54;
    let n = rng.below(6);
    let names: Vec<Vec<u8>> = (0..n)
        .map(|_| {
            let len = rng.below(5) as usize;
            let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
            s.push(0);
            s
        })
        .collect();
    let node = |k: u32| obj + 0x100 + 0x20 * k;
    if n == 0 {
        t.w32(head + 0xc, head);
    } else {
        t.w32(head + 0xc, node(n - 1));
    }
    for k in 0..n {
        let left = if k == 0 { head } else { node(k - 1) };
        t.w32(node(k), left);
        let text = obj + 0x800 + 0x10 * k;
        t.write(text, &names[k as usize]);
        let bit = u32::from(rng.chance(3));
        // the key word's low bit is the colour: the string sits at an even
        // address so clearing it gives the string back
        t.w32(node(k) + 0xc, text | bit);
        let value = rng.next();
        t.w32(node(k) + 0x10, value);
        let next = if k + 1 == n { head } else { node(k + 1) };
        t.w32(node(k) + 0x18, next);
    }
    t.stub_successor(glue9::TREE_SUCCESSOR);
    let name = obj + 0xc00;
    if n > 0 && rng.chance(2) {
        t.write(name, &names[rng.below(n) as usize]);
    } else {
        let len = rng.below(4) as usize;
        let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
        s.push(0);
        t.write(name, &s);
    }
    args[0] = name;
}

fn prep_resolution(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0dd8);
    *this = scene;
    let (w, h) = match rng.below(7) {
        // a ratio just above 16:9 that rounds to it as a float
        6 => (17_777_778, 10_000_000),
        0 => (1280, 720),
        1 => (1920, 1080),
        2 => (800, 600),
        3 => (0, 0),
        4 => (rng.below(5000), rng.below(5000)),
        _ => (rng.next(), rng.next()),
    };
    t.w32(obj + 0x26c, w);
    t.w32(obj + 0x270, h);
    t.stub_ret(glue9::GET_APPLICATION, false, 0, 0, obj);
}

fn prep_render_pass(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.write(obj + 0x74, &[u8::from(rng.chance(2))]);
    t.w32(obj + 0x38, obj + 0x100);
    let i = rng.below(4);
    args[0] = i;
    t.w32(obj + 0x100 + 4 * i, obj + 0x200);
    t.w32(obj, obj + 0x300);
    t.write(obj + 0x304, &[u8::from(rng.chance(3))]);
}

fn prep_shadow_map(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = obj + 0x40;
    for k in [4, 8, 9] {
        args[k] = rng.float().to_bits();
    }
    t.stub_ret(glue9::SHADOW_UPDATE_MAP, true, 12, 12, obj + 0x500);
}

#[rustfmt::skip]
const CASES9: &[Case] = &[
    Case { prep: Some(prep_delete_pmesh), ..method("scene_delete_pmesh_instance", 0x0065_a4c0, This::Object, 2, &[(glue9::MESH_INSTANCE_DTOR, K::This(0)), (glue9::CLUSTER_DELETE_OBJECT, K::This(1))], |m, this, a| { g9::scene_delete_pmesh_instance(m, this, a[0], a[1]); Out::None }) },
    Case { prep: Some(prep_delete_meshes), ..method("scene_do_delete_mesh_instances", 0x0065_a620, This::Object, 0, &[(glue9::VFX_REMOVE_DELETED_POOL_MESH_INSTANCES, K::Cdecl(0)), (glue9::SCENE_DELETE_PMESH_INSTANCE, K::This(2)), (glue9::SCENE_REMOVE_VFX_FROM_RENDER_ARRAY, K::This(1))], |m, this, _| { g9::scene_do_delete_mesh_instances(m, this); Out::None }) },
    Case { prep: Some(prep_all_assets), ..method("scene_do_on_all_field_map_assets_loaded", 0x0065_a6c0, This::Object, 4, &[(glue9::GET_APPLICATION, K::Cdecl(0)), (glue9::ASSET_LIST_COUNT, K::This(0)), (glue9::SCENE_REGIST_CLUSTERS_TO_ADD, K::This(2)), (glue9::FINISH_READ_BATTLE_MAP, K::Cdecl(0))], |m, this, a| u(g9::scene_do_on_all_field_map_assets_loaded(m, this, a[0], a[2], a[3]))) },
    Case { prep: Some(prep_geometry_pool), ..method("vfx_geometry_from_pool", 0x0069_f750, This::Object, 2, &[(glue9::VFX_DYN_GEO_CTOR, K::This(0))], |m, this, a| u(g9::vfx_geometry_from_pool(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_parallel), ..method("parallel_erase", 0x006b_2ca0, This::Object, 1, &[], |m, this, a| u(g9::parallel_erase(m, this, a[0]))) },
    Case { prep: Some(prep_set_vertex), ..method("broken_set_vertex", 0x006d_49f0, This::Object, 9, &[], |m, this, a| { g9::broken_set_vertex(m, this, a[0], arr(&a[1..])); Out::None }) },
    Case { prep: Some(prep_insert_array), ..method("tree_insert_array", 0x0065_fca0, This::Object, 2, &[(glue9::USER_FREE, K::Cdecl(1)), (glue9::FREE_LIST_DEALLOCATE, K::This(1)), (glue9::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g9::tree_insert_array(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_claim8), ..method("scene_new_pool_array", 0x0065_9860, This::Object, 1, &[(glue9::CLUSTER_CREATE, K::This(3))], |m, this, a| u(g9::scene_new_pool_array(m, this, a[0]))) },
    Case { prep: Some(prep_claim56), ..method("scene_new_pool_object56", 0x0065_9910, This::Object, 1, &[(glue9::CLUSTER_CREATE, K::This(3)), (glue9::CTOR_ZERO56, K::This(0))], |m, this, a| u(g9::scene_new_pool_object56(m, this, a[0]))) },
    method("distortion_ctor", 0x006c_2010, This::Object, 0, &[(glue9::SAMPLER_STATE_CTOR, K::This(0))], |m, this, _| u(g9::distortion_ctor(m, this))),
    Case { prep: Some(prep_capture), ..method("scene_render_capture_texture", 0x0066_b130, This::Object, 0, &[(glue9::RENDERER_SET_COLOR_TARGET, K::This(2)), (glue9::RENDERER_SET_VIEWPORT, K::This(4)), (glue9::RENDERER_SET_CLEAR_COLOR, K::This(4)), (glue9::RENDERER_BEGIN_SCENE, K::This(1)), (glue9::RENDERER_END_SCENE, K::This(1)), (glue9::POST_RENDER_FULL_SCREEN_TEXTURE, K::This(3))], |m, this, _| u(g9::scene_render_capture_texture(m, this))) },
    Case { prep: Some(prep_chr_relation), ..cdecl("check_chr_cluster_relationship", 0x0063_d9c0, 0, &[(glue9::VFX_DESTROY, K::Cdecl(2)), (glue9::SCENE_ATTACH_CHR_CLUSTER, K::This(0)), (glue9::FREE_686FE0, K::Cdecl(1))], |m, _, _| u(g9::check_chr_cluster_relationship(m))) },
    with_cookie(framed(Case { prep: Some(prep_map_vfx), ..method("scene_load_map_vfx_texture", 0x0066_4f60, This::Object, 0, &[(glue9::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (glue9::SPRINTF, K::ImportC(4)), (glue9::SCENE_LOAD_VFX_TEXTURE_ASYNC, K::This(2))], |m, this, _| u(g9::scene_load_map_vfx_texture(m, frame_ebp(), this))) }, 0x204, 0)),
    framed(Case { prep: Some(prep_texture_layout), ..method("scene_set_vfx_texture_layout", 0x0065_6ae0, This::Stdcall, 3, &[(glue9::STRSTR, K::ImportC(2)), (glue9::SSCANF, K::ImportC(8))], |m, _, a| { let slots = frame_ebp() + 8; for (k, &w) in a.iter().enumerate() { m.write(slots + 4 * k as u32, w); } g9::scene_set_vfx_texture_layout(m, slots, frame_ebp() - 0x10, a[0], a[1], a[2]); Out::None }) }, 0x10, 12),
    Case { prep: Some(prep_find_named), ..method("scene_find_named", 0x0066_0420, This::Object, 1, &[], |m, this, a| u(g9::scene_find_named(m, this, a[0]))) },
    Case { prep: Some(prep_resolution), ..method("scene_set_resolution_scale", 0x0065_2910, This::Object, 0, &[], |m, this, _| u(g9::scene_set_resolution_scale(m, this))) },
    framed(Case { prep: Some(prep_render_pass), ..method("broken_render_pass", 0x006d_55b0, This::Object, 3, &[(glue9::BROKEN_CREATE_RENDER_TARGET, K::This(0)), (glue9::BROKEN_PASS_6D3FB0, K::This(6)), (glue9::BROKEN_SET_COLOUR, K::This(1)), (glue9::BROKEN_SET_TEXTURE_6D4540, K::This(2))], |m, this, a| u(g9::broken_render_pass(m, frame_ebp(), this, a[0], a[1], a[2]))) }, 0x10, 0),
    with_cookie(framed(Case { prep: Some(prep_shadow_map), ..cdecl("update_shadow_map", 0x0064_4a00, 12, &[(glue9::SHADOW_MANAGER_INSTANCE, K::Getter)], |m, _, a| u(g9::update_shadow_map(m, frame_ebp(), a[0], arr(&a[1..])))) }, 0x14, 0)),
];

#[test]
fn batch9_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 12 } else { 60 };
        run_cases(t, CASES9, n, &mut Rng(0x9009_b009));
    });
}

// ---- batch 10 (ffx_prender::glue10, round 3) ----

/// The shader loaders: the reader opens or not (`al` alone decides),
/// the cluster load and the fix-up's result.
fn prep_shader_bin_load(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfa98);
    *this = scene;
    let open = [0, 1, 0x100, 0xffff_ff01][rng.below(4) as usize];
    t.stub_ret(glue10::READER_IS_OPEN, true, 0, 0, open);
}

macro_rules! shader_case {
    ($name:literal, $addr:literal, $bin:expr) => {
        with_seh(framed(
            Case {
                prep: Some(prep_shader_bin_load),
                ..method(
                    $name,
                    $addr,
                    This::Object,
                    0,
                    &[
                        (glue10::FIOS_UNIFY_FILENAME, K::Cdecl(3)),
                        (glue10::READER_CTOR, K::This(1)),
                        (glue10::READER_CLOSE, K::This(0)),
                        (glue10::CLUSTER_MANAGER_INSTANCE, K::Getter),
                        (glue10::CLUSTER_MANAGER_LOAD, K::This(1)),
                        (glue10::GET_APPLICATION, K::Cdecl(0)),
                        (glue10::FIXUP_CLUSTERS, K::Cdecl(2)),
                        (glue10::READER_DTOR, K::This(0)),
                    ],
                    |m, this, _| u(g10::scene_load_shader(m, frame_ebp(), this, $bin)),
                )
            },
            0x12c,
            0,
        ))
    };
}

const fn with_seh(case: Case) -> Case {
    Case { seh: true, ..case }
}

/// The pending instantiation requests, some for `chr`.
fn prep_unregist(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let chr = rng.below(3);
    args[0] = chr;
    let n = rng.below(6);
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.global(glue10::CHARACTER_LOADING.sym, flag | n);
    t.global(glue10::CHARACTER_LOADING_DATA.sym, obj + 0x100);
    for k in 0..n {
        let request = obj + 0x200 + 0x20 * k;
        t.w32(obj + 0x100 + 4 * k, request);
        let who = rng.below(3);
        t.w32(request + 8, who);
    }
}

/// An effect's geometry (or none) and render data (parts), a vtable
/// whose `+4` logs and fails now and then.
fn prep_vfx_setup(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if rng.chance(6) {
        args[0] = 0;
        args[1] = 0;
    }
    if rng.chance(3) {
        args[0] = 0u32.wrapping_sub(rng.below(20000));
    }
    t.global(glue10::VFX_RENDER_DATA_TABLE.sym, obj + 0x1800);
    let geo = if rng.chance(6) { 0 } else { obj + 0x400 };
    t.stub_ret(glue10::VFX_GET_DYN_GEO, true, 2, 2, geo);
    let ret = if rng.chance(3) { rng.next() | 1 } else { 0 };
    let virt = t.stub(None, 0x0010_0004, true, 2, 2, Some(ret));
    t.w32(obj + 0x400, obj + 0x500);
    t.w32(obj + 0x504, virt);
    let data = if rng.chance(5) { 0 } else { obj + 0x600 };
    t.w32(obj + 0x424, data);
    t.write(obj + 0x6c0, &[u8::from(rng.chance(2))]);
    let parts = rng.below(5) as u16;
    t.write(obj + 0x6c4, &parts.to_le_bytes());
}

/// The scene's queue of (world, list) pairs, lists with missing clusters.
fn prep_detach(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let m = rng.next();
    t.w32(scene + 0xfb3c, m);
    // a negative count (which the release after the loop checks for)
    // would make the loop walk 2^31 pairs: the original can't get there
    // with a valid queue
    let n = rng.below(4);
    t.w32(scene + 0x1188, n);
    let cap = if rng.chance(2) {
        0x8000_0000 | rng.next()
    } else {
        rng.below(8)
    };
    t.w32(scene + 0x118c, cap);
    t.w32(scene + 0x1190, obj + 0x100);
    {
        for i in 0..n {
            let world = rng.next();
            t.w32(obj + 0x100 + 8 * i, world);
            let list = obj + 0x200 + 0x40 * i;
            t.w32(obj + 0x104 + 8 * i, list);
            let count = rng.below(4);
            t.w32(list, count);
            t.w32(list + 8, obj + 0x400 + 0x40 * i);
            for j in 0..count {
                let c = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(obj + 0x400 + 0x40 * i + 4 * j, c);
            }
        }
    }
}

/// The delegated entries: flagged to run or not, one-shot or not, a
/// logging callback; the free list taking them back or not.
fn prep_delegation(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(6);
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(obj, flag | n);
    t.w32(obj + 4, obj + 0x100);
    let callback = t.stub(None, 0x0020_0001, false, 1, 0, Some(rng.next()));
    for k in 0..n {
        let e = if rng.chance(5) {
            0
        } else {
            obj + 0x200 + 0x20 * k
        };
        t.w32(obj + 0x100 + 4 * k, e);
        if e != 0 {
            let arg = rng.next();
            t.w32(e, arg);
            let buffer = if rng.chance(3) { 0 } else { rng.next() };
            t.w32(e + 4, buffer);
            let flags =
                [0x2000, 0x2001, 0x2001, 1, 0][rng.below(5) as usize] | (rng.next() & 0xdffe);
            t.w32(e + 0xc, flags);
            t.w32(e + 0x10, callback);
        }
    }
    let mutex = rng.next();
    t.global(glue10::DELEGATION_MUTEX.sym, mutex);
    let r = if rng.chance(2) {
        0x8000_0000 | rng.next()
    } else {
        rng.below(4)
    };
    t.stub_ret(glue10::DELEGATION_SET_TO_FREE_LIST, true, 1, 1, r);
}

fn prep_shadow_resize(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for off in [0x74, 0x78, 0x80, 0x7c] {
        let tex = if rng.chance(8) { 0 } else { rng.next() | 1 };
        t.w32(obj + off, tex);
    }
    let r = if rng.chance(2) { 0 } else { rng.word() };
    t.stub_ret(glue10::TEXTURE_SET_DIMENSIONS, true, 4, 4, r);
}

fn prep_map_unload(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfaf4);
    *this = scene;
    t.write(scene + 0x25, &[u8::from(!rng.chance(4))]);
    let map = [0, 1, rng.below(400), rng.next()][rng.below(4) as usize];
    t.w32(scene + 0xfaf0, map);
}

/// The delete pool: a free slot (id 0) or none.
fn prep_vfx_delete_pool(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(5);
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(obj + 8, flag | n);
    t.w32(obj + 0xc, obj + 0x100);
    t.w32(obj + 0x14, obj + 0x200);
    t.w32(obj + 0x1c, obj + 0x300);
    for (off, w) in [(0x10, rng.below(5) | flag), (0x18, rng.below(5))] {
        t.w32(obj + off, w);
    }
    for k in 0..n {
        let (lo, hi) = match rng.below(4) {
            0 => (0, 0),
            1 => (0, rng.next()),
            2 => (rng.next(), 0),
            _ => (rng.next(), rng.next()),
        };
        t.w32(obj + 0x100 + 8 * k, lo);
        t.w32(obj + 0x104 + 8 * k, hi);
    }
    t.stub_ret(glue10::PHYRE_MALLOC, false, 2, 0, obj + 0x400);
}

fn prep_push_elements(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(6);
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(obj, flag | n);
    let old = obj + 0x100;
    t.w32(obj + 4, old);
    args[0] = [0, obj + 0x400, old][rng.below(3) as usize];
    args[1] = obj + 0x40;
}

/// Nodes `lo..hi` (in order) as a balanced tree under `parent`; its root
/// (`head` when empty).
fn build_map_tree(
    t: &mut Twins,
    lo: u32,
    hi: u32,
    parent: u32,
    head: u32,
    node: &dyn Fn(u32) -> u32,
) -> u32 {
    if lo >= hi {
        return head;
    }
    let mid = u32::midpoint(lo, hi);
    let n = node(mid);
    t.w32(n + 4, parent);
    t.write(n + 0xd, &[0]);
    let left = build_map_tree(t, lo, mid, n, head, node);
    let right = build_map_tree(t, mid + 1, hi, n, head, node);
    t.w32(n, left);
    t.w32(n + 8, right);
    n
}

/// A balanced MSVC map of effects under a head (`+0xd` set), each with
/// render data whose segments carry per-buffer draw words; the buffer id
/// (negative now and then).
fn prep_clear_draw(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let head = obj + 0x40;
    t.w32(obj, head);
    t.write(head + 0xd, &[1]);
    let n = rng.below(6);
    let node = |k: u32| obj + 0x100 + 0x20 * k;
    let root = build_map_tree(t, 0, n, head, head, &node);
    t.w32(head + 4, root);
    t.w32(head, if n == 0 { head } else { node(0) });
    t.w32(head + 8, if n == 0 { head } else { node(n - 1) });
    for k in 0..n {
        let value = obj + 0x800 + 0x40 * k;
        t.w32(node(k) + 0x18, value);
        let data = obj + 0xa00 + 0x100 * k;
        t.w32(value + 0x24, data);
        let segments = rng.below(5) as i32 - 1;
        t.w32(data + 0x90, segments as u32);
        t.w32(data + 0x94, obj + 0x1000 + 0x1c0 * (k % 5));
    }
    let fill = rng.bytes(0xa00);
    t.write(obj + 0x1000, &fill[..0x8c0]);
    let id = (rng.below(4) as i32 - 1) as u32;
    t.stub_ret(glue10::DYN_GEO_CURRENT_BUFFER_ID, false, 0, 0, id);
}

#[rustfmt::skip]
const CASES10: &[Case] = &[
    shader_case!("load_ab_cone_shader", 0x0066_3d40, glue10::AB_CONE),
    shader_case!("load_ab_wave_shader", 0x0066_3e20, glue10::AB_WAVE),
    shader_case!("load_chr_sha_shader", 0x0066_4360, glue10::CHR_SHA),
    shader_case!("load_menu_no_texture_shader", 0x0066_5010, glue10::MENU_NO_TEXTURE),
    shader_case!("load_menu_shader", 0x0066_50f0, glue10::MENU),
    shader_case!("load_na_menu_shader", 0x0066_5200, glue10::NA_MENU),
    shader_case!("load_nb_menu_shader", 0x0066_52e0, glue10::NB_MENU),
    shader_case!("load_nt_menu_ztest_shader", 0x0066_53c0, glue10::NT_MENU_ZTEST),
    shader_case!("load_ntnb_menu_shader", 0x0066_54a0, glue10::NTNB_MENU),
    shader_case!("load_optimized_font_shader", 0x0066_5580, glue10::OPTIMIZED_FONT),
    Case { prep: Some(prep_unregist), ..cdecl("character_unregist_loading", 0x0063_7b30, 1, &[(glue10::OPERATOR_DELETE, K::Cdecl(1)), (glue10::PHYRE_FREE, K::Cdecl(1))], |m, _, a| { g10::character_unregist_loading(m, a[0]); Out::None }) },
    Case { prep: Some(prep_vfx_setup), ..cdecl("vfx_setup", 0x0064_5780, 2, &[(glue10::PHYRE_PRINTF, K::Cdecl(2))], |m, _, a| u(g10::vfx_setup(m, a[0], a[1]))) },
    Case { prep: Some(prep_detach), ..method("scene_detach_chr_cluster", 0x0065_66c0, This::Object, 0, &[(glue10::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue10::RELEASE_MUTEX, K::Import(1)), (glue10::IOB_FUNC, K::ImportC(0)), (glue10::FPRINTF, K::ImportC(2)), (glue10::WORLD_ADD_CLUSTER, K::This(1)), (glue10::ARRAY_RESERVE_668300, K::This(1))], |m, this, _| { g10::scene_detach_chr_cluster(m, this); Out::None }) },
    Case { prep: Some(prep_delegation), ..method("delegation_before_vfx", 0x006a_db80, This::Object, 0, &[(glue10::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (glue10::RELEASE_MUTEX, K::Import(1)), (glue10::OPERATOR_DELETE, K::Cdecl(1))], |m, this, _| { g10::delegation_before_vfx(m, this); Out::None }) },
    Case { prep: Some(prep_shadow_resize), ..method("shadow_resize_textures", 0x006d_2df0, This::Object, 0, &[(glue10::SHADOW_TEXTURE_PREPARE, K::This(0))], |m, this, _| u(g10::shadow_resize_textures(m, this))) },
    method("vfx_dyn_geo_ctor", 0x0069_e3e0, This::Object, 0, &[], |m, this, _| u(g10::vfx_dyn_geo_ctor(m, this))),
    with_cookie(framed(Case { prep: Some(prep_map_unload), ..method("scene_unload_map_vfx_texture", 0x0067_0fe0, This::Object, 0, &[(glue10::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (glue10::SPRINTF, K::ImportC(4)), (glue10::SCENE_UNLOAD_VFX_TEXTURE, K::This(2))], |m, this, _| u(g10::scene_unload_map_vfx_texture(m, frame_ebp(), this))) }, 0x204, 0)),
    framed(Case { prep: Some(prep_vfx_delete_pool), ..method("vfx_insert_to_delete_pool", 0x0069_f900, This::Object, 4, &[(glue10::PUSH_INTO_6A2790, K::This(2)), (glue10::PUSH_INTO_6A25E0, K::This(2)), (glue10::PUSH_INTO_6A2670, K::This(2))], |m, this, a| { let args = frame_ebp() + 8; for (k, &w) in a.iter().enumerate() { m.write(args + 4 * k as u32, w); } u(g10::vfx_insert_to_delete_pool(m, args, this, a[0], a[1], a[2], a[3])) }) }, 0, 16),
    Case { prep: Some(prep_push_elements), ..method("parray_push_into32", 0x006b_2730, This::Object, 2, &[(glue10::ELEMENTS_32.copy, K::Cdecl(3)), (glue10::ELEMENTS_32.construct, K::This(1)), (glue10::ELEMENTS_32.destroy, K::Cdecl(2))], |m, this, a| u(g10::parray_push_into_elements(m, this, a[0], a[1], glue10::ELEMENTS_32))) },
    Case { prep: Some(prep_push_elements), ..method("parray_push_into64", 0x006b_2800, This::Object, 2, &[(glue10::ELEMENTS_64.copy, K::Cdecl(3)), (glue10::ELEMENTS_64.construct, K::This(1)), (glue10::ELEMENTS_64.destroy, K::Cdecl(2))], |m, this, a| u(g10::parray_push_into_elements(m, this, a[0], a[1], glue10::ELEMENTS_64))) },
    Case { prep: Some(prep_clear_draw), ..method("vfx_clear_draw_data", 0x0069_f2b0, This::Object, 0, &[], |m, this, _| { g10::vfx_clear_draw_data(m, this); Out::None }) },
];

#[test]
fn batch10_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 12 } else { 60 };
        run_cases(t, CASES10, n, &mut Rng(0x1010_b010));
    });
}

// ---- batch 11 (ffx_prender::glue11, round 3) ----

fn prep_insert_pair(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let node = obj + 0x400;
    let next = rng.next();
    t.w32(node, next);
    if rng.chance(3) {
        t.w32(obj + 0x18, 0);
        let made = if rng.chance(3) { 0 } else { node };
        t.stub_this_out(glue11::FREE_LIST_ALLOCATE_BLOCK, 1, 1, None, &[(0, made)]);
    } else {
        t.w32(obj + 0x18, node);
    }
    args[0] = rng.next();
    args[1] = rng.next();
    let built = if rng.chance(2) { node } else { obj + 0x500 };
    t.stub_ret(glue11::NODE_CTOR_64CB00, true, 2, 2, built);
    let dup = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(glue11::TREE_INSERT_64C0A0, true, 1, 1, dup);
}

fn prep_texture_info(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let array = obj + 0x40;
    let flag = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(array, flag | rng.below(9));
    let data = rng.next();
    t.w32(array + 4, data);
    args[0] = array;
    for a in &mut args[3..8] {
        *a = rng.float().to_bits();
    }
    let copy = obj + 0x600;
    let old = [0, copy, rng.next() | 1][rng.below(3) as usize];
    t.w32(obj + 0xc, old);
    let own = [0, 0x8000_0000][rng.below(2) as usize];
    t.w32(obj + 8, own | rng.below(9));
    t.stub_ret(glue11::ARRAY_COPY_5281E0, false, 2, 0, copy);
    let holder = if rng.chance(4) { 0 } else { obj + 0x700 };
    t.stub_ret(glue11::PHYRE_MALLOC_SIZE, false, 1, 0, holder);
}

fn prep_cutbuffer_load(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = if rng.chance(3) {
        0x8000_0000 | rng.next()
    } else {
        rng.next() & 0x7fff_ffff
    };
    let open = [0, 1, 0x100, 0xffff_ff01][rng.below(4) as usize];
    t.stub_ret(glue11::READER_IS_OPEN, true, 0, 0, open);
}

/// Groups of mesh instances in tree order (the iterator's end, current
/// and next; successors from `+0x18`), each instance with a group bit and
/// a draw link or none; the mask changed or not, the tree empty or not.
fn prep_group_visible(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb18);
    *this = scene;
    let head = scene + 0x1198;
    let root = match rng.below(6) {
        0 => head,
        1 => 0,
        _ => rng.next() | 1,
    };
    t.w32(scene + 0x11a4, root);
    let old = rng.next() & 0xffff;
    t.w32(scene + 0xfb14, old);
    args[0] = if rng.chance(6) {
        old
    } else {
        old ^ (rng.next() & 0xffff)
    };
    let end = obj + 0x40;
    let n = rng.below(4);
    let node = |k: u32| obj + 0x100 + 0x20 * k;
    for k in 0..n {
        let group = obj + 0x400 + 0x40 * k;
        t.w32(node(k) + 0x10, group);
        let next = if k + 1 == n { end } else { node(k + 1) };
        t.w32(node(k) + 0x18, next);
        let count = rng.below(4);
        t.w32(group, count);
        t.w32(group + 8, group + 0x10);
        for m in 0..count {
            let mesh = obj + 0xc00 + 0x100 * k + 0x40 * m;
            t.w32(group + 0x10 + 4 * m, mesh);
            let bit = rng.below(34);
            t.w32(mesh + 0x48, bit);
            let link = if rng.chance(4) {
                0
            } else {
                obj + 0x1800 + 0x40 * (4 * k + m)
            };
            t.w32(mesh + 0x1c, link);
            let w = rng.next();
            if link != 0 {
                t.w32(link + 0x1c, w);
            }
        }
    }
    let (cur, next) = match n {
        0 => (end, end),
        1 => (node(0), end),
        _ => (node(0), node(1)),
    };
    t.stub_this_out(
        glue11::TREE_ITERATOR_BEGIN,
        1,
        1,
        None,
        &[(0, end), (4, cur), (8, next)],
    );
    t.stub_successor(glue11::TREE_SUCCESSOR);
}

/// The swap's state: screen, pause, the field map's flags, loads, the
/// post-process flag and the free camera.
fn prep_swap_buffer(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = u32::from(rng.chance(2));
    let screen = u32::from(rng.chance(4));
    t.global(glue11::SCREEN_STATE.sym, screen);
    let pause = u32::from(rng.chance(4));
    t.global(glue11::PAUSE_ON.sym, pause);
    let scene = t.zeros(0x1198);
    t.global(SCENE, scene);
    let flags = [
        u8::from(rng.chance(3)),
        u8::from(rng.chance(3)),
        u8::from(rng.chance(3)),
    ];
    t.write(scene + 0x1195, &flags);
    let loading = if rng.chance(2) {
        0
    } else {
        rng.below(3) | (rng.next() & 0x8000_0000)
    };
    t.global(glue11::CHARACTER_LOADING.sym, loading);
    let listed = u32::from(rng.chance(2));
    t.stub_ret(glue11::IS_MAP_IN_LIST, false, 0, 0, listed);
    let done = [0, 1, 0x100][rng.below(3) as usize];
    t.stub_ret(glue11::ALL_REQUESTS_DONE, true, 0, 0, done);
    let post = t.zeros(0x1_23a4);
    t.write(post + 0x1_23a0, &[u8::from(rng.chance(2))]);
    t.stub_ret(glue11::POST_PROCESS_INSTANCE, false, 0, 0, post);
    let camera = obj + 0x100;
    t.write(camera + 0x338, &[u8::from(rng.chance(4))]);
    t.write(camera + 0x33c, &[u8::from(rng.chance(4))]);
    let mode = [0, 1, 2][rng.below(3) as usize];
    t.w32(camera, mode);
    t.stub_ret(glue11::FREE_CAMERA_INSTANCE, false, 0, 0, camera);
    t.global(glue11::VFX_RENDER_DATA_TABLE.sym, obj + 0x1800);
}

fn prep_rwtt(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfc74);
    *this = scene;
    let texture = if rng.chance(5) { 0 } else { obj + 0x100 };
    t.stub_ret(glue11::CLUSTER_CREATE, true, 3, 3, texture);
    let list = if rng.chance(5) { 0 } else { obj + 0x200 };
    t.stub_ret(glue11::CLUSTER_ALLOCATE_INSTANCE_LIST, true, 3, 3, list);
    let target = if rng.chance(4) { 0 } else { obj + 0x300 };
    t.w32(obj + 0x234, target);
}

/// A material of the scene's pool (slot in or out of range, remainder
/// sign), its kind, or another cluster's.
fn prep_delete_material(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfba4);
    *this = scene;
    let material = obj + 0x600;
    let (slot, negative) = match rng.below(5) {
        0 => (rng.below(4000) as i32, false),
        1 => (3998 + rng.below(4) as i32, false),
        2 => (-(rng.below(3) as i32) - 1, true),
        3 => (0, true),
        _ => (rng.next() as i32 >> 8, false),
    };
    let negative = negative || slot < 0;
    let rest = (1 + rng.below(23)) as i32 * if negative { -1 } else { 1 };
    let diff = slot.wrapping_mul(24).wrapping_add(rest);
    t.w32(scene + 0xfbec, material.wrapping_sub(diff as u32));
    let s = diff / 24;
    t.w32(scene + 0xfbf8, (obj + 0x1000).wrapping_sub(s as u32));
    let low = rng.below(4005) as i32 - 2;
    t.w32(scene + 0xfbfc, low as u32);
    let storage = rng.next();
    t.w32(material + 0x10, storage);
    let kind = [rng.below(64), 63, 64, rng.next()][rng.below(4) as usize];
    t.w32(material + 0x14, kind);
    let part = rng.next();
    t.w32(material + 4, part);
    args[0] = material;
    args[1] = if rng.chance(4) {
        rng.next() | 1
    } else {
        scene + 0x12d4
    };
}

/// Pairs (word, name or none) to copy; a null destination for a single
/// pair.
fn prep_copy_pairs(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let n = rng.below(5);
    args[2] = n;
    args[0] = if n <= 1 && rng.chance(3) {
        0
    } else {
        obj + 0x100
    };
    args[1] = obj + 0x400;
    for k in 0..n.max(1) {
        let word = rng.next();
        t.w32(obj + 0x400 + 8 * k, word);
        let text = obj + 0x800 + 0x20 * k;
        let len = rng.below(10) as usize;
        let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
        s.push(0);
        t.write(text, &s);
        let name = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => text | 1,
            _ => text,
        };
        t.w32(obj + 0x404 + 8 * k, name);
    }
    let p = if rng.chance(4) { 0 } else { obj + 0xc00 };
    t.stub_ret(glue11::PHYRE_MALLOC, false, 2, 0, p);
}

#[rustfmt::skip]
const CASES11: &[Case] = &[
    Case { prep: Some(prep_insert_pair), ..method("tree_insert_pair", 0x0065_fdd0, This::Object, 2, &[(glue11::NODE_RELEASE_64B8C0, K::This(1)), (glue11::TREE_POST_INSERT, K::This(1))], |m, this, a| u(g11::tree_insert_pair(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_texture_info), ..method("render_texture_info_set", 0x006b_2400, This::Object, 8, &[(glue11::PHYRE_FREE, K::Cdecl(1)), (glue11::PSTRING_ASSIGN, K::This(1))], |m, this, a| u(g11::render_texture_info_set(m, this, a[0], a[1], a[2], arr(&a[3..])))) },
    with_seh(framed(Case { prep: Some(prep_cutbuffer_load), ..method("scene_load_cutbuffer_shader", 0x0066_4440, This::Object, 0, &[(glue11::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (glue11::READER_CTOR, K::This(1)), (glue11::READER_CLOSE, K::This(0)), (glue11::ASYNC_LOAD_INSTANCE, K::Getter), (glue11::LOAD_CLUSTER, K::Cdecl(7)), (glue11::READER_DTOR, K::This(0))], |m, this, _| u(g11::scene_load_cutbuffer_shader(m, frame_ebp(), this))) }, 0x12c, 0)),
    framed(Case { prep: Some(prep_group_visible), ..method("scene_set_group_visible", 0x0066_d610, This::Object, 1, &[], |m, this, a| { g11::scene_set_group_visible(m, frame_ebp(), this, a[0]); Out::None }) }, 0x10, 0),
    Case { prep: Some(prep_swap_buffer), ..cdecl("dyn_geo_swap_buffer", 0x0063_ff70, 1, &[(glue11::ASYNC_LOAD_INSTANCE, K::Getter), (glue11::DYN_GEO_SWAP, K::This(0)), (glue11::VFX_CLEAR_DRAW_DATA, K::This(0))], |m, _, a| u(g11::dyn_geo_swap_buffer(m, a[0]))) },
    Case { prep: Some(prep_rwtt), ..method("scene_create_rwtt_target", 0x0065_9c30, This::Object, 0, &[(glue11::TEXTURE_SET_DIMENSIONS, K::This(4)), (glue11::TEXTURE_UPDATE_MAX_LEVEL, K::This(0)), (glue11::TARGET_CTOR, K::This(1))], |m, this, _| u(g11::scene_create_rwtt_target(m, this))) },
    Case { prep: Some(prep_delete_material), ..method("scene_delete_material", 0x0065_a2c0, This::Object, 4, &[(glue11::PHYRE_FREE, K::Cdecl(1)), (glue11::CLUSTER_DELETE_WITH_LIST, K::This(2))], |m, this, a| { g11::scene_delete_material(m, this, a[0], a[1], a[2], a[3]); Out::None }) },
    framed(Case { prep: Some(prep_copy_pairs), ..cdecl("copy_named_pairs", 0x0069_91f0, 3, &[(glue11::MEMCPY, K::Cdecl(3))], |m, _, a| { let slots = frame_ebp() + 8; for (k, &w) in a.iter().enumerate() { m.write(slots + 4 * k as u32, w); } g11::copy_named_pairs(m, slots, a[0], a[1], a[2]); Out::None }) }, 0, 12),
];

#[test]
fn batch11_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 12 } else { 60 };
        run_cases(t, CASES11, n, &mut Rng(0x1111_b011));
    });
}

// ---- round 4 batch 12 (ffx_prender::h3_glue12) ----

/// A float that is often in `0..=1` or just past it (the fade's range).
fn fade_float(rng: &mut Rng) -> f32 {
    match rng.below(4) {
        0 => rng.below(1000) as f32 / 999.0,
        1 => 1.0 + (rng.below(100) as f32 - 50.0) / 1000.0,
        2 => -(rng.below(100) as f32) / 100.0,
        _ => rng.float(),
    }
}

/// The fog manager: a fade type (0, 1, or another), its time and `dt`.
fn prep_fog_fade(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let kind = [0, 1, 0, 1, 2, rng.next()][rng.below(6) as usize];
    t.w32(obj + 0xb4, kind);
    // now and then the time lands on 1 exactly (not past it: no change)
    let (time, dt) = match rng.below(4) {
        0 => (0.5, 0.5),
        1 => (fade_float(rng), rng.float()),
        _ => (fade_float(rng), rng.below(100) as f32 / 1000.0),
    };
    t.w32(obj + 0xb0, f32::to_bits(time));
    args[0] = f32::to_bits(dt);
}

/// The scene's render-target holder (`+0x10cc` -> `+0x8c`, its byte `+4`
/// clear: set, the original reads the target at 0x20), the target's
/// height, the minimap type and blitzball.
fn prep_minimap_top(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let block = obj + 0x1000;
    t.w32(obj + 0x10cc, block);
    let holder = obj + 0x1200;
    t.w32(block + 0x8c, holder);
    let target = obj + 0x1300;
    t.w32(holder, target);
    t.write(holder + 4, &[0]);
    let height = match rng.below(4) {
        0 => [720, 1080, 1440, 2160][rng.below(4) as usize],
        1 => rng.below(5000),
        2 => rng.next() | 0x8000_0000,
        _ => rng.next(),
    };
    t.w32(target + 0x20, height);
    t.w32(
        obj + 0x74,
        [0, 1, 2, 3, 4, 5, rng.next()][rng.below(7) as usize],
    );
    t.write(obj, &[u8::from(rng.chance(2))]);
}

/// The default targets (`+0x10cc` -> `+0x8c`, `+0x90`) and a renderer.
fn prep_restore_targets(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let block = obj + 0x1000;
    t.w32(obj + 0x10cc, block);
    let a = rng.next();
    let b = rng.next();
    t.w32(block + 0x8c, a);
    t.w32(block + 0x90, b);
    let r = rng.next();
    t.w32(obj + 0xd0, r);
}

fn prep_auto_test(_: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], _: &mut Rng) {
    *this = obj + 0x100;
}

/// A scene big enough for its fade fields (`+0x10dcc`), the movie fade,
/// a mesh list (some null) and a renderer; `beginScene` mostly 0.
fn prep_fade_meshes(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    t.write(scene + 0x10dcc, &[u8::from(!rng.chance(3))]);
    t.global(h12::MOVIE_FADING.sym, u32::from(rng.chance(3)));
    let n = if rng.chance(5) { 0 } else { rng.below(5) };
    t.w32(scene + 0x3860, n);
    let list = obj + 0x100;
    t.w32(scene + 0x3868, list);
    for k in 0..n {
        let mesh = if rng.chance(4) {
            0
        } else {
            obj + 0x400 + 0x40 * k
        };
        t.w32(list + 4 * k, mesh);
        let inst = rng.next();
        t.w32(obj + 0x410 + 0x40 * k, inst);
    }
    let renderer = obj + 0x1000;
    t.w32(scene + 0xd0, renderer);
    let pass = rng.next();
    t.w32(renderer + 0x13c, pass);
    let name = rng.word();
    t.global(h12::FADE_SKIP_NAME.sym, name);
    let begin = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h12::BEGIN_SCENE, true, 1, 1, begin);
    let same = u32::from(rng.chance(3)) | (rng.next() & 0xffff_ff00);
    t.stub_ret(h12::NAME_IS, true, 1, 1, same);
}

#[rustfmt::skip]
const CASES12: &[Case] = &[
    Case { prep: Some(prep_fog_fade), ..method("fog_update_chr_fade", 0x006e_3f40, This::Object, 1, &[(h12::SET_CHR_FOG_EFFECT, K::This(1))], |m, this, a| { gh12::fog_update_chr_fade(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_minimap_top), ..method("scene_minimap_viewport_top", 0x0065_e4a0, This::Object, 0, &[(h12::MINIMAP_VIEWPORT_SIZE, K::This(0))], |m, this, _| u(gh12::scene_minimap_viewport_top(m, this))) },
    with_seh(framed(Case { prep: Some(prep_restore_targets), ..method("scene_restore_default_targets", 0x0066_c7f0, This::Object, 0, &[(h12::RENDER_SURFACE_CTOR, K::This(2)), (h12::RENDER_SURFACE_DTOR, K::This(0)), (h12::SET_COLOR_TARGET, K::This(2)), (h12::SET_DEPTH_STENCIL_TARGET, K::This(1))], |m, this, _| u(gh12::scene_restore_default_targets(m, frame_ebp(), this))) }, 0x2c, 0)),
    framed(Case { prep: Some(prep_auto_test), ..method("auto_test_manager_ctor", 0x006b_ca90, This::Object, 0, &[(h12::CREATE_THREAD_PS2, K::Cdecl(2)), (h12::START_THREAD_PS2, K::Cdecl(2))], |m, this, _| u(gh12::auto_test_manager_ctor(m, frame_ebp(), this))) }, 0x30, 0),
    Case { prep: Some(prep_fade_meshes), ..method("scene_render_fade_meshes", 0x0065_0ea0, This::Object, 0, &[(h12::SET_CAMERA, K::This(1)), (h12::UPDATE_VIEW_MATRICES, K::This(0)), (h12::RENDER_MESH_INSTANCE, K::This(4)), (h12::END_SCENE, K::This(1))], |m, this, _| u(gh12::scene_render_fade_meshes(m, this))) },
    Case { prep: Some(prep_add_unique), ..method("scene_add_unique", 0x0065_6a00, This::Object, 1, &[(h12::MEMCPY, K::Cdecl(3)), (h12::USER_FREE, K::Cdecl(1))], |m, this, a| u(gh12::scene_add_unique(m, this, a[0]))) },
    with_cookie(framed(Case { prep: Some(prep_unload_magic), ..method("scene_unload_magic_vfx_texture", 0x0067_0f00, This::Object, 1, &[(h12::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (h12::SPRINTF_SLOT, K::ImportC(4)), (h12::UNLOAD_VFX_TEXTURE, K::This(2))], |m, this, a| u(gh12::scene_unload_magic_vfx_texture(m, frame_ebp(), this, a[0]))) }, 0x204, 0)),
    Case { prep: Some(prep_shadow_grid), ..method("shadow_insert_block", 0x006c_e4f0, This::Object, 6, &[(h12::PRINTF_SLOT, K::ImportC(1))], |m, this, a| u(gh12::shadow_insert_block(m, this, a[0], a[1], a[2], a[3], a[4], a[5]))) },
    Case { prep: Some(prep_shadow_grid), ..method("shadow_insert_cell", 0x006c_e3e0, This::Object, 6, &[(h12::PRINTF_SLOT, K::ImportC(1))], |m, this, a| u(gh12::shadow_insert_cell(m, this, a[0], a[1], a[2], a[3], a[4], a[5]))) },
    Case { prep: Some(prep_scene_resize), ..method("scene_resize", 0x0066_c580, This::Object, 0, RESIZE_STUBS, |m, this, _| u(gh12::scene_resize(m, this))) },
    Case { prep: Some(prep_fade_world), ..method("scene_render_fade_world", 0x0065_0f80, This::Object, 0, &[(h12::CLEAR_SCENE, K::This(1)), (h12::SET_CAMERA, K::This(1)), (h12::UPDATE_VIEW_MATRICES, K::This(0)), (h12::RENDER_WORLD, K::This(3)), (h12::GET_APPLICATION, K::Getter), (h12::END_SCENE, K::This(1))], |m, this, _| u(gh12::scene_render_fade_world(m, this))) },
    framed(Case { prep: Some(prep_vector_parameter), ..method("write_vector_parameter", 0x0066_e4f0, This::Object, 2, &[(h12::WRITE_PARAMETER, K::This(2))], |m, this, a| u(gh12::write_vector_parameter(m, frame_ebp(), this, a[0], a[1]))) }, 0x10, 0),
    Case { prep: Some(prep_dtt_copy), ..method("scene_copy_previous_frame_for_dtt", 0x0065_93d0, This::Object, 0, &[(h12::MOTION_BLUR_INSTANCE, K::Getter)], |m, this, _| Out::U8(gh12::scene_copy_previous_frame_for_dtt(m, this))) },
    Case { prep: Some(prep_matrix), ..method("broken_screen_set_matrix", 0x006d_4a90, This::Object, 1, &[], |m, this, a| u(gh12::broken_screen_set_matrix(m, this, a[0]))) },
    method("vfx_shader_manager_ctor", 0x006b_f4a0, This::Object, 0, &[(h12::FREE_LIST_CTOR, K::This(4))], |m, this, _| u(gh12::vfx_shader_manager_ctor(m, this))),
    Case { prep: Some(prep_specular), ..cdecl("sg_set_specular", 0x0064_3cc0, 6, &[(h12::FIND_PARAMETER, K::This(1)), (h12::SET_PARAMETER_VALUE, K::This(2))], |m, _, a| { gh12::sg_set_specular(m, a[0], a[1], arr(&a[2..])); Out::None }) },
    Case { prep: Some(prep_instantiate_loaded), ..cdecl("character_instantiate_loaded", 0x0063_9db0, 0, &[(h12::MAKE_INSTANCE, K::Cdecl(2)), (h12::OPERATOR_DELETE, K::Cdecl(1)), (h12::PHYRE_FREE, K::Cdecl(1))], |m, _, _| { gh12::character_instantiate_loaded(m); Out::None }) },
    Case { prep: Some(prep_msaa), ..method("post_process_set_msaa", 0x0069_7e10, This::Object, 1, &[(h12::TEXTURE_IS_BOUND, K::This(0)), (h12::TEXTURE_UNBIND, K::This(0))], |m, this, a| u(gh12::post_process_set_msaa(m, this, a[0]))) },
    Case { prep: Some(prep_unregister_by_name), ..method("texture_animation_unregister_by_name", 0x006b_1a70, This::Object, 1, &[(h12::UNREGISTER_CHARACTER_INSTANCE, K::This(1)), (h12::ANIMATION_RELEASE, K::This(0))], |m, this, a| { gh12::texture_animation_unregister_by_name(m, this, a[0]); Out::None }) },
    Case { prep: Some(prep_draw_situation), ..method("scene_add_chr_draw_situation", 0x0065_64a0, This::Object, 1, &[(h12::WORLD_REMOVE_CLUSTER, K::This(1)), (h12::WORLD_ADD_CLUSTER, K::This(1)), (h12::ARRAY_RESERVE_16, K::This(1))], |m, this, a| { let r = gh12::scene_add_chr_draw_situation(m, this, a[0]); if a[0] as u8 == 0 { Out::None } else { u(r) } }) },
    Case { prep: Some(prep_loading_screen), ..method("scene_render_loading_screen", 0x0066_b1e0, This::Object, 0, &[(h12::LOADING_IMPORT_SLOT, K::Import(1)), (h12::SLEEP_MS, K::Cdecl(1)), (h12::APP_UPDATE, K::This(0)), (h12::LOADING_SET_TIME, K::This(1)), (h12::LOADING_RENDER, K::This(1)), (h12::IGGY_RENDER, K::This(0)), (h12::DELETE_MESH_INSTANCES, K::This(0))], |m, this, _| u(gh12::scene_render_loading_screen(m, this))) },
    Case { prep: Some(prep_delegation_init), ..method("delegation_helper_init", 0x006a_dc90, This::Object, 0, &[(h12::PHYRE_FREE, K::Cdecl(1))], |m, this, _| u(gh12::delegation_helper_init(m, this))) },
    Case { prep: Some(prep_model_release), ..cdecl("model_buffer_release", 0x0072_c1b0, 1, &[(h12::USER_FREE, K::Cdecl(1))], |m, _, a| { gh12::model_buffer_release(m, a[0]); Out::None }) },
];

/// The index array (16 or not, owned or not), the allocation (failing
/// now and then) and the node allocation.
fn prep_delegation_init(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let count = [0x10, 0x8000_0010, 0, 3, 0x8000_0004, 0x11][rng.below(6) as usize];
    t.w32(obj, count);
    let old = if rng.chance(4) {
        obj + 0x400
    } else {
        obj + 0x300
    };
    t.w32(obj + 4, old);
    let p = if rng.chance(5) { 0 } else { obj + 0x400 };
    t.stub_ret(h12::PHYRE_MALLOC, false, 2, 0, p);
    t.stub_ret(h12::OPERATOR_NEW, false, 1, 0, obj + 0x600);
}

/// Model buffers in use or not with references and buffers, the count,
/// and an index (in range or not; the high half ignored).
fn prep_model_release(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let n = rng.below(7);
    t.global(h12::MODEL_BUFFER_COUNT.sym, n);
    let base = t.rebase(h12::MODEL_BUFFERS);
    for k in 0..8 {
        let entry = base + 0x18c * k;
        let mut bytes = vec![0u8; 0x18c];
        for w in 5..0x18c / 4 {
            if rng.chance(8) {
                bytes[4 * w..4 * w + 4].copy_from_slice(&rng.next().to_le_bytes());
            }
        }
        let used = u32::from(!rng.chance(3));
        bytes[..4].copy_from_slice(&used.to_le_bytes());
        let refs = [0u32, 1, 1, 2, 0xffff_ffff][rng.below(5) as usize];
        bytes[4..8].copy_from_slice(&refs.to_le_bytes());
        t.write(entry, &bytes);
    }
    args[0] = rng.below(8) | (rng.next() & 0xffff_0000);
}

/// The draw-situation stacks: the pending cluster flag, the depth (0..3,
/// or negative) and its bases, the situation array (grown or not), and
/// the original's never-written `[ebp-4]` set to 0 (the port's value).
fn prep_draw_situation(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let ebp = t.a.stack_range().end - 0x100 - 4 - 8;
    t.w32(ebp - 4, 0);
    t.write(scene + 0xfb44, &[u8::from(rng.chance(2))]);
    let depth = [0, 1, 2, 3, 0xffff_ffff][rng.below(5) as usize];
    t.w32(scene + 0xf954, depth);
    for k in 0..8 {
        let w = rng.next();
        t.w32(scene + 0xf8cc + 4 * k, w);
        let base = rng.below(100);
        t.w32(scene + 0xf910 + 4 * k, base);
    }
    let n = rng.below(5);
    t.w32(scene + 0xfb54, n);
    let cap = [n, n + 1, n + 3, 0, 0x8000_0000][rng.below(5) as usize];
    t.w32(scene + 0xfb58, cap);
    t.w32(scene + 0xfb5c, obj + 0x100);
    let situations = rng.below(9);
    t.w32(scene + 0xfb60, situations);
    args[0] = [0, 1, 0x80, 0xff, 0x100, 0x101][rng.below(6) as usize] | (rng.next() & 0xffff_0000);
}

/// A synthetic application (shown or not, its time, vtable `+0x28`), its
/// renderer (vtable `+8`) and `flip`, the loading screen, the Iggy
/// manager (or none) and window messages (a quit now and then).
fn prep_loading_screen(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let app = obj + 0x1000;
    t.stub_ret(h12::GET_APPLICATION, false, 0, 0, app);
    t.write(
        app + 0x3a0,
        &[u8::from(rng.chance(3)), u8::from(rng.chance(3))],
    );
    let time = rng.float();
    t.w32(app + 0x24, time.to_bits());
    let present = if rng.chance(4) { rng.next() | 1 } else { 0 };
    let present = t.stub(None, 0xa08, true, 0, 0, Some(present));
    let method = t.stub(None, 0xa28, true, 0, 0, Some(rng.next()));
    t.w32(obj + 0x1c00 + 8, present);
    t.w32(obj + 0x1c40 + 0x28, method);
    t.w32(app + 0x124, obj + 0x1c00);
    t.w32(app, obj + 0x1c40);
    let flip = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h12::RENDERER_FLIP, true, 0, 0, flip);
    let quit = u32::from(rng.chance(3));
    t.stub_ret(h12::PROCESS_WINDOWS_MESSAGE, false, 0, 0, quit);
    t.global(h12::LOADING_SCREEN.sym, obj + 0x1800);
    let iggy = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.global(h12::IGGY_MENU_MANAGER.sym, iggy);
}

/// A post-process manager with four target holders (live or not) and a
/// sample count (valid or not).
fn prep_msaa(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let pp = t.zeros(0x1_2300);
    *this = pp;
    for (k, off) in [0x8c, 0x90, 0x94, 0x98].into_iter().enumerate() {
        let holder = obj + 0x100 + 0x10 * k as u32;
        t.w32(pp + off, holder);
        let texture = rng.next();
        t.w32(holder, texture);
        t.write(holder + 4, &[u8::from(rng.chance(4)), rng.next() as u8]);
    }
    args[0] = [0, 2, 4, 8, 16, 1, 3, 32, 0, 4][rng.below(10) as usize];
}

/// Animation entries with names (some null, some equal to the one
/// asked), registered instances on them, and the name (null now and
/// then).
fn prep_unregister_by_name(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    *this = obj;
    let names: [&[u8]; 4] = [b"ab\0", b"abc\0", b"b\0", b"\0"];
    for (k, s) in names.iter().enumerate() {
        t.write(obj + 0x600 + 0x10 * k as u32, s);
    }
    let n = rng.below(5);
    t.w32(obj + 0x64, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    t.w32(obj + 0x68, obj + 0x200);
    for k in 0..n {
        let entry = obj + 0x200 + 0x20 * k;
        let name = obj + 0x600 + 0x10 * rng.below(4);
        let word = match rng.below(4) {
            0 => 0,
            1 => 1,
            2 => name | 1,
            _ => name,
        };
        t.w32(entry + 0x18, word);
    }
    let m = rng.below(6);
    t.w32(obj + 0x6c, m);
    t.w32(obj + 0x70, obj + 0x400);
    for j in 0..m {
        let item = obj + 0x400 + 0xc * j;
        t.w32(item, rng.below(5));
        let instance = rng.next();
        t.w32(item + 4, instance);
    }
    args[0] = if rng.chance(6) {
        0
    } else {
        obj + 0x600 + 0x10 * rng.below(4)
    };
}

/// 16 floats to copy.
fn prep_matrix(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let m = obj + 0x400;
    for k in 0..16 {
        let f = rng.float();
        t.w32(m + 4 * k, f.to_bits());
    }
    args[0] = m;
}

/// A model (null now and then) with meshes, the index in range or not,
/// and a material list (0..3 entries of shader and buffer).
fn prep_specular(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let model = obj + 0x100;
    let n = rng.below(4);
    t.w32(model, n | if rng.chance(2) { 0x8000_0000 } else { 0 });
    t.w32(model + 4, obj + 0x140);
    for k in 0..n {
        let mesh = obj + 0x180 + 0x20 * k;
        t.w32(obj + 0x140 + 4 * k, mesh);
        t.w32(mesh + 0x10, obj + 0x300);
    }
    let materials = rng.below(4);
    t.w32(
        obj + 0x300,
        materials | if rng.chance(2) { 0x8000_0000 } else { 0 },
    );
    let data = obj + 0x340;
    for k in 0..materials {
        let entry = obj + 0x400 + 0x10 * k;
        t.w32(data + 4 * k, entry);
        let shader = rng.next();
        let buffer = rng.next();
        t.w32(entry, shader);
        t.w32(entry + 4, buffer);
    }
    t.stub_ret(h12::ARRAY_DATA, true, 0, 0, data);
    args[0] = if rng.chance(6) { 0 } else { model };
    args[1] = rng.below(5);
}

/// The character loading list: requests (out pointer, character with its
/// loaded byte and asset, argument, callback or none), the count's owned
/// bit, cloth loading now and then.
fn prep_instantiate_loaded(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let n = rng.below(6);
    let owned = if rng.chance(2) { 0x8000_0000 } else { 0 };
    t.global(h12::CHARACTER_LOADING.sym, owned | n);
    let data = obj + 0x100;
    t.global(h12::CHARACTER_LOADING_DATA.sym, data);
    let callback = t.stub(None, 0xcb, false, 1, 0, Some(rng.next()));
    for k in 0..n {
        let request = obj + 0x200 + 0x20 * k;
        t.w32(data + 4 * k, request);
        t.w32(request, obj + 0x600 + 4 * k);
        let chr = obj + 0x800 + 0x40 * k;
        t.w32(request + 4, chr);
        let arg = rng.next();
        t.w32(request + 8, arg);
        t.w32(request + 0xc, if rng.chance(3) { 0 } else { callback });
        t.write(chr + 0x1c, &[u8::from(!rng.chance(3))]);
        t.w32(chr + 4, if rng.chance(4) { 0 } else { rng.next() });
    }
    let cloth = u32::from(rng.chance(4));
    t.stub_ret(h12::IS_LOADING_CLOTH, false, 1, 0, cloth);
}

/// A parameter (type byte `+2`: taken or not) and a record (none now and
/// then; its kind, texture and argument), the texture install mostly 0.
fn prep_vector_parameter(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj + 0x800;
    let parameter = obj + 0x100;
    let kinds = [0x42, 0x43, 0x44, 0x12, 0xd9, 0xdc, 0xdd, 0x41, 0x45, 0];
    let kind = kinds[rng.below(kinds.len() as u32) as usize];
    t.write(parameter + 2, &[kind]);
    let record = obj + 0x200;
    let words = [
        [0x42, 0x43, 0x44, 0x41, 0x45, rng.next()][rng.below(6) as usize],
        rng.next(),
        if rng.chance(2) { 0 } else { obj + 0x300 },
        if rng.chance(3) { 0 } else { rng.next() },
    ];
    for (k, w) in (0u32..).zip(words) {
        t.w32(record + 4 * k, w);
    }
    args[0] = parameter;
    args[1] = if rng.chance(6) { 0 } else { record };
    let err = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h12::TEXTURE_INSTALL, true, 1, 1, err);
}

/// A scene state `+1` (0..5), `+2`, DTT objects `+0x3854`, the targets'
/// motion-blur byte, the blur's own `+0x58`, the opus end and a magic ID
/// (from the list now and then).
fn prep_dtt_copy(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x4000);
    *this = scene;
    let state = [0, 1, 1, 1, 2, 4, 5, 6][rng.below(8) as usize];
    t.write(scene + 1, &[state, u8::from(rng.chance(2))]);
    t.w32(scene + 0x3854, rng.below(3));
    let targets = t.zeros(0x1_2400);
    t.w32(scene + 0x10cc, targets);
    t.write(targets + 0x123bb, &[u8::from(rng.chance(3))]);
    if !t.snapshot {
        t.write(obj + 0x58, &[u8::from(rng.chance(2))]);
    }
    t.global(
        h12::OPUS_WAVE_OMD_END.sym,
        if rng.chance(4) { rng.next() } else { 0 },
    );
    let id = match rng.below(3) {
        0 => h12::DTT_MAGIC_IDS[rng.below(16) as usize],
        1 => [0x46, 0x23a][rng.below(2) as usize],
        _ => rng.below(0x300),
    };
    t.stub_ret(h12::MAGIC_ID, false, 0, 0, id);
}

/// The resize chain's callees: every getter and resize.
const RESIZE_STUBS: &[(u32, K)] = &[
    (h12::SET_RESOLUTION_SCALE, K::This(0)),
    (h12::RESIZE_PAIRS[0].0, K::Getter),
    (h12::RESIZE_PAIRS[0].1, K::This(2)),
    (h12::RESIZE_PAIRS[1].0, K::Getter),
    (h12::RESIZE_PAIRS[1].1, K::This(2)),
    (h12::RESIZE_PAIRS[2].0, K::Getter),
    (h12::RESIZE_PAIRS[2].1, K::This(2)),
    (h12::RESIZE_PAIRS[3].0, K::Getter),
    (h12::RESIZE_PAIRS[3].1, K::This(2)),
    (h12::RESIZE_PAIRS[4].0, K::Getter),
    (h12::RESIZE_PAIRS[4].1, K::This(2)),
    (h12::RESIZE_PAIRS[5].0, K::Getter),
    (h12::RESIZE_PAIRS[5].1, K::This(2)),
    (h12::GET_APPLICATION, K::Getter),
    (h12::IGGY_SET_VIEWPORT, K::This(2)),
];

/// A scene with the array `+0x3874` (count `+0x386c`, capacity `+0x3870`;
/// null now and then), `v` in it or not, and `user_malloc` giving a
/// buffer (or 0 where the original then writes nothing).
fn prep_add_unique(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x4000);
    *this = scene;
    let n = rng.below(7);
    let data = obj + 0x100;
    let null = n == 0 && rng.chance(2);
    t.w32(scene + 0x3874, if null { 0 } else { data });
    t.w32(scene + 0x386c, n);
    let cap = match rng.below(5) {
        0 => n,
        1 => n + 1 + rng.below(3),
        2 => n.saturating_sub(1),
        3 => rng.next() | 0x8000_0000,
        _ => rng.below(4),
    };
    t.w32(scene + 0x3870, cap);
    for k in 0..=n {
        let w = rng.below(6);
        t.w32(data + 4 * k, w);
    }
    args[0] = rng.below(6);
    let fail = n == 0 && rng.chance(3);
    t.stub_ret(
        h12::USER_MALLOC,
        false,
        1,
        0,
        if fail { 0 } else { obj + 0x400 },
    );
}

/// The magic VFX list `+0x30` (count `+0x28`, `id` in it or not, zeros),
/// the magic path `+0xfb00` (0, 1, a pointer).
fn prep_unload_magic(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let n = rng.below(6);
    t.w32(scene + 0x28, n);
    let list = obj + 0x100;
    t.w32(scene + 0x30, list);
    for k in 0..n {
        let w = rng.below(4);
        t.w32(list + 4 * k, w);
    }
    args[0] = rng.below(4);
    let magic = [0, 1, obj + 0x800, obj + 0x801][rng.below(4) as usize];
    t.w32(scene + 0xfb00, magic);
}

/// The 8 x 8 grid mostly taken (full now and then), two out-pointers,
/// a name (the special one now and then; up to 11 characters).
fn prep_shadow_grid(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let shadow = t.zeros(0x4100);
    *this = shadow;
    let full = rng.chance(5);
    let holes = rng.below(4);
    let grid: Vec<u8> = (0..64)
        .map(|_| {
            if full || !rng.chance(holes + 2) {
                (rng.below(255) + 1) as u8
            } else {
                0
            }
        })
        .collect();
    t.write(shadow + 0x2ae8, &grid);
    let name = obj + 0x200;
    let len = rng.below(12) as usize;
    let mut s: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
    if rng.chance(3) {
        s = b"c001".to_vec();
    } else if rng.chance(4) {
        s = b"c00".to_vec();
    }
    s.push(0);
    t.write(name, &s);
    args[2] = obj + 0x100;
    args[3] = obj + 0x104;
    args[4] = name;
}

/// A scene with its render size and post-process targets; their resize
/// mostly 0.
fn prep_scene_resize(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let w = rng.word();
    let h = rng.word();
    t.w32(scene + 0x10dd0, w);
    t.w32(scene + 0x10dd4, h);
    let targets = obj + 0x100;
    t.w32(scene + 0x10cc, targets);
    let err = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h12::POST_PROCESS_RESIZE, true, 2, 2, err);
    if !t.snapshot {
        let iggy = rng.word();
        t.global(h12::IGGY_MENU_MANAGER.sym, iggy);
    }
}

/// A fading scene (or not) with a renderer, the world flag `+0x93c` and
/// a fade camera (null now and then); `beginScene` mostly 0.
fn prep_fade_world(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    t.write(scene + 0x10dcc, &[u8::from(!rng.chance(3))]);
    t.global(h12::MOVIE_FADING.sym, u32::from(rng.chance(3)));
    t.write(scene + 0x93c, &[u8::from(!rng.chance(3))]);
    let camera = if rng.chance(4) { 0 } else { obj + 0x300 };
    t.w32(scene + 0x7e0, camera);
    t.w32(scene + 0xd0, obj + 0x1000);
    let begin = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h12::BEGIN_SCENE, true, 1, 1, begin);
}

#[test]
fn batch12_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 16 } else { 80 };
        run_cases(t, CASES12, n, &mut Rng(0x1212_b012));
    });
}

// ---- round 4 batch 13 (ffx_prender::h3_glue13) ----

/// The two cluster allocations (failing now and then).
fn prep_broken_ctor(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let p = if rng.chance(4) { 0 } else { obj + 0x400 };
    t.stub_ret(h13::PHYRE_MALLOC_SIZE, false, 1, 0, p);
}

/// A C string of `n` letters at `at`.
fn letters(t: &mut Twins, at: u32, n: u32, rng: &mut Rng) {
    let mut s: Vec<u8> = (0..n).map(|_| b'a' + rng.below(26) as u8).collect();
    s.push(0);
    t.write(at, &s);
}

/// The VFX table (its string pool at `+0x40`), the data found or not,
/// texture slots with names in the pool, outside it (shorter or longer
/// than the new name) or none, and the new name.
fn prep_vfx_texture(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let table = obj + 0x100;
    t.global(h13::VFX_RENDER_DATA_TABLE.sym, table);
    // the names at obj + 0x1000 lie in the pool, or (the pool's start
    // moved down) just inside its end or exactly past it
    let names = obj + 0x1000;
    let pool = names - [0, 0xf_9fff, 0xf_a000][rng.below(3) as usize];
    t.w32(table + 0x40, pool);
    let data = obj + 0x200;
    let n = rng.below(5);
    t.w32(data + 0x90, if rng.chance(8) { 0xffff_ffff } else { n });
    t.w32(data + 0x94, obj + 0x300);
    let len = rng.below(8);
    letters(t, obj + 0x700, len, rng);
    for k in 0..n {
        let slot = obj + 0x300 + 0x6c * k + 0x20;
        let name = match rng.below(4) {
            0 => 0,
            1 => names + 0x40 * k,
            _ => obj + 0x500 + 0x20 * k,
        };
        let l = rng.below(8);
        letters(t, names + 0x40 * k, l, rng);
        letters(t, obj + 0x500 + 0x20 * k, l, rng);
        t.w32(slot, name);
    }
    let found = if rng.chance(5) { 0 } else { data };
    t.stub_ret(h13::VFX_DATA_BY_INSTANCE, true, 2, 2, found);
    t.stub_ret(h13::ARRAY_NEW, false, 1, 0, obj + 0x1800);
    args[0] = if rng.chance(6) { 0 } else { rng.next() };
    args[1] = if rng.chance(2) { 0 } else { rng.next() };
    args[2] = obj + 0x700;
}

/// The map's fog (or none) and the scene's fog record.
fn prep_map_fog(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let map = obj + 0x400;
    t.w32(obj + 0x4f8, if rng.chance(3) { 0 } else { map });
    t.w32(obj + 0xcc, obj + 0x600);
    for k in 0..8 {
        let f = rng.float();
        t.w32(map + 0xd4 + 4 * k, f.to_bits());
    }
}

/// Map node `k` of [`prep_table_clear`].
fn clear_node(obj: u32, k: u32) -> u32 {
    obj + 0x200 + 0x20 * k
}

/// A map (head and a right chain of 0..4 effects with keys, some 0), the
/// global table (this one or another, its cached key matching or not,
/// the cached geometry with a vtable or null) and what `MapFind` finds.
fn prep_table_clear(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let head = obj + 0x100;
    let n = rng.below(5);
    t.write(head + 0xd, &[1]);
    t.w32(obj, head);
    let first = if n == 0 { head } else { clear_node(obj, 0) };
    let last = if n == 0 { head } else { clear_node(obj, n - 1) };
    t.w32(head, first);
    t.w32(head + 4, first);
    t.w32(head + 8, last);
    let mut keys = vec![];
    for k in 0..n {
        let at = clear_node(obj, k);
        t.w32(at, head);
        t.w32(at + 4, if k == 0 { head } else { clear_node(obj, k - 1) });
        t.w32(
            at + 8,
            if k + 1 < n {
                clear_node(obj, k + 1)
            } else {
                head
            },
        );
        t.write(at + 0xd, &[0]);
        let a = if rng.chance(4) { 0 } else { rng.below(4) };
        let b = if rng.chance(2) { 0 } else { rng.next() };
        t.w32(at + 0x10, a);
        t.w32(at + 0x14, b);
        keys.push((a, b));
        let value = rng.next();
        t.w32(at + 0x18, value);
    }
    let table = if rng.chance(2) { obj } else { obj + 0x1000 };
    t.global(h13::VFX_RENDER_DATA_TABLE.sym, table);
    if table != obj {
        t.w32(table, obj + 0x1100);
    }
    let (ca, cb) = match keys.first() {
        Some(&k) if rng.chance(2) => k,
        _ => (rng.below(2), 0),
    };
    t.w32(table + 0x20, ca);
    t.w32(table + 0x24, cb);
    let geometry = obj + 0x1400;
    t.w32(geometry, obj + 0x1500);
    let vcall = t.stub(None, 0x510, true, 0, 0, Some(rng.next()));
    t.w32(obj + 0x1500 + 0x10, vcall);
    t.w32(table + 0x28, if rng.chance(4) { 0 } else { geometry });
    let found = obj + 0x1600;
    t.w32(found + 0x18, if rng.chance(4) { 0 } else { geometry });
    let end: u32 = t.r32(table);
    let out = if rng.chance(3) { end } else { found };
    let ret = rng.next();
    t.stub_out(h13::MAP_FIND, 2, 2, ret, 0, out);
}

#[rustfmt::skip]
const CASES13: &[Case] = &[
    Case { prep: Some(prep_broken_ctor), ..method("broken_screen_ctor", 0x006d_4ba0, This::Object, 0, &[(h13::CLUSTER_CTOR, K::This(0))], |m, this, _| u(gh13::broken_screen_ctor(m, this))) },
    Case { prep: Some(prep_vfx_texture), ..cdecl("vfx_update_texture", 0x0064_5aa0, 3, &[(h13::ARRAY_DELETE, K::Cdecl(1))], |m, _, a| { gh13::vfx_update_texture(m, a[0], a[1], a[2]); Out::None }) },
    Case { prep: Some(prep_map_fog), ..method("scene_setup_map_fog", 0x0066_ee70, This::Object, 0, &[(h13::FOG_INSTANCE, K::Getter), (h13::FOG_COPY_PARAMETERS, K::This(0))], |m, this, _| u(gh13::scene_setup_map_fog(m, this))) },
    framed(Case { prep: Some(prep_table_clear), ..method("vfx_table_clear", 0x0069_f000, This::Object, 0, &[(h13::PRINTF_SLOT, K::ImportC(1)), (h13::INSERT_TO_DELETE_POOL, K::This(4)), (h13::MAP_CLEAR, K::This(0))], |m, this, _| { gh13::vfx_table_clear(m, frame_ebp(), this); Out::None }) }, 0x14, 0),
];

#[test]
fn batch13_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES13, n, &mut Rng(0x1313_b013));
    });
}

/// Delegations (some null) for tasks 0..2 with data equal to the asked
/// bytes or not (and buffers or none), the count (with the owned bit;
/// 0 and 1 too), and the free list refusing now and then.
fn prep_unregister(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(6);
    t.w32(obj, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    t.w32(obj + 4, obj + 0x100);
    let wanted: Vec<u8> = (0..12).map(|_| rng.below(3) as u8).collect();
    t.write(obj + 0x700, &wanted);
    for k in 0..n.max(1) {
        let d = obj + 0x200 + 0x20 * k;
        t.w32(obj + 0x100 + 4 * k, if rng.chance(6) { 0 } else { d });
        let own = obj + 0x400 + 0x10 * k;
        let mut bytes = wanted.clone();
        if rng.chance(2) {
            let at = rng.below(12) as usize;
            bytes[at] ^= 1;
        }
        t.write(own, &bytes);
        t.w32(d, own);
        t.w32(d + 4, if rng.chance(3) { 0 } else { rng.next() });
        t.w32(d + 0x10, rng.below(3));
    }
    // a null delegation ends the walk (with no count it is the only end)
    t.w32(obj + 0x100 + 4 * n.max(1), 0);
    let r = if rng.chance(3) {
        0x8000_0000 | rng.next()
    } else {
        rng.below(3)
    };
    t.stub_ret(h13::SET_DATA_TO_FREE_LIST, true, 1, 1, r);
    args[0] = rng.below(3);
    args[1] = obj + 0x700;
    args[2] = rng.below(13);
}

#[rustfmt::skip]
const CASES13B: &[Case] = &[
    Case { prep: Some(prep_unregister), ..method("delegation_unregister", 0x006a_e340, This::Object, 3, &[(h13::WAIT_FOR_SINGLE_OBJECT, K::Import(2)), (h13::RELEASE_MUTEX, K::Import(1)), (h13::DELEGATION_COMPACT, K::This(0)), (h13::OPERATOR_DELETE, K::Cdecl(1))], |m, this, a| u(gh13::delegation_unregister(m, this, a[0], a[1], a[2]))) },
];

#[test]
fn batch13b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 16 } else { 80 };
        run_cases(t, CASES13B, n, &mut Rng(0x1313_b113));
    });
}

// ---- round 5 batch 14 (ffx_prender::h3_glue14) ----

/// A cursor script word the stub leaves alone.
const KEEP: u32 = 0x5e5e_5e5e;

impl Twins {
    /// Stubs for a cursor's first step and its next: each call copies the
    /// next five words of `steps` (base, block, stride, count, offset)
    /// into the cursor (`ecx`), but for [`KEEP`] words (left as the port
    /// wrote them), then logs as [`Twins::stub`].
    fn stub_cursor_script(&mut self, first: u32, next: u32, steps: &[[u32; 5]]) {
        let mut words = vec![];
        for s in steps {
            for w in s {
                words.extend(w.to_le_bytes());
            }
        }
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        for sym in [first, next] {
            let log = self.stub(None, sym, true, 0, 0, Some(sym));
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
}

/// Cursor runs over the elements at `area + 0x100 k`: one to three runs
/// of 0..4 elements, each with a block word that may equal one of its
/// elements (`[block]` the next block), then a run with no count. The
/// offset is 4 (the object at the element), or one making the address 0
/// or the object 0.
fn cursor_steps(t: &mut Twins, area: u32, odd: bool, rng: &mut Rng) -> Vec<[u32; 5]> {
    let mut steps = vec![];
    for _ in 0..=rng.below(3) {
        let base = area + 0x100 * rng.below(4);
        let count = rng.below(5);
        let block = if rng.chance(2) {
            base + 0x100 * (1 + rng.below(3))
        } else {
            rng.next() | 3
        };
        let next_block = if rng.chance(2) { block } else { rng.next() | 3 };
        if block % 4 == 0 {
            t.w32(block, next_block);
        }
        // an odd offset only for a lone element (the next would fault)
        let (offset, count) = match if odd { rng.below(6) } else { 5 } {
            0 => (0u32.wrapping_sub(base), 1),
            1 => (4u32.wrapping_sub(base), 1),
            _ => (4, count),
        };
        steps.push([base, block, 0x100, count, offset]);
    }
    // the end: nothing written, so the walk stops on the count it stored
    // and the frame shows its stores
    steps.push([KEEP; 5]);
    steps
}

/// The field map (or none, or its flag clear) and objects named the
/// global name (the same pointer, tagged, a copy), another name or none.
fn prep_find_global(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    let map = obj + 0x40;
    t.w32(scene + 0x1128, if rng.chance(5) { 0 } else { map });
    t.write(scene + 0x93c, &[u8::from(!rng.chance(5))]);
    t.w32(
        map + 0x1c,
        if rng.chance(2) {
            map + 0x1c
        } else {
            obj + 0x80
        },
    );
    let name = t.rebase(h14::GLOBAL_NAME);
    let mut text = vec![0u8; 16];
    Mem::read_bytes(&t.a, name, &mut text);
    let len = text.iter().position(|&b| b == 0).unwrap_or(15);
    let copy = obj + 0x1c00;
    t.write(copy, &text[..=len]);
    let mut other = text[..=len].to_vec();
    other[rng.below(len as u32 + 1) as usize] ^= 1;
    t.write(copy + 0x20, &other);
    // a prefix of the name, and the name with one more letter
    let mut short = text[..len].to_vec();
    short[len - 1] = 0;
    t.write(copy + 0x40, &short);
    let mut long = text[..len].to_vec();
    long.extend([b'x', 0]);
    t.write(copy + 0x60, &long);
    let area = t.zeros(0x800);
    for k in 0..8 {
        let s = match rng.below(8) {
            0 => name,
            1 => name | 1,
            2 => copy,
            3 => copy + 0x20,
            4 => 0,
            5 => copy + 0x40,
            6 => copy + 0x60,
            _ => 1,
        };
        t.w32(area + 0x100 * k + 0xd0, s);
    }
    let steps = cursor_steps(t, area, true, rng);
    t.stub_cursor_script(h14::CURSOR_SEEK1, h14::CURSOR_NEXT1, &steps);
}

/// The flags, the first target (or none; its texture live or not, the
/// size equal to the arguments or not) and the arguments.
fn prep_resize_targets(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    let flags: Vec<u8> = (0..4).map(|_| u8::from(rng.chance(3))).collect();
    t.write(post + 0x1_23b0, &flags);
    let target = post + 0x200;
    let texture = post + 0x300;
    t.w32(post + 0x8c, if rng.chance(5) { 0 } else { target });
    t.w32(target, texture);
    // a set byte +4 means no texture, which the original reads at 0
    t.write(target + 4, &[0]);
    for off in [
        0x90, 0x94, 0x98, 0x9c, 0xa4, 0xa8, 0x1_232c, 0x1_2330, 0x1_22c0,
    ] {
        let v = rng.next();
        t.w32(post + off, v);
    }
    let (w, h) = (rng.below(3) * 640, rng.below(3) * 360);
    args[0] = w;
    args[1] = h;
    t.w32(texture + 0x1c, if rng.chance(2) { w } else { w + 1 });
    t.w32(texture + 0x20, if rng.chance(2) { h } else { h + 1 });
}

fn prep_claim28(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_pool_claim(t, obj, this, args, rng, (0xfbb4, 0xfba4, 0xfbb0), 0x1c);
    let r = if rng.chance(4) { 0 } else { rng.next() };
    t.stub_ret(h14::CLUSTER_CREATE, true, 3, 3, r);
}

fn prep_claim16(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_pool_claim(
        t,
        obj,
        this,
        &mut args[1..],
        rng,
        (0xfbc8, 0xfbb8, 0xfbc4),
        0x10,
    );
    args[0] = if rng.chance(6) { 0 } else { obj + 0x1f00 };
    let list = obj + 0x1e00;
    t.w32(list + 0x34, if rng.chance(4) { 0 } else { obj + 0x1d00 });
    let r = if rng.chance(4) { 0 } else { list };
    t.stub_ret(h14::CLUSTER_ALLOCATE_INSTANCE_LIST, true, 3, 3, r);
}

/// An owned/borrowed count word (`n` elements).
fn count_word(rng: &mut Rng, n: u32) -> u32 {
    n | if rng.chance(3) { 0x8000_0000 } else { 0 }
}

/// The three arrays: words, words and 32-byte objects with a destroying
/// virtual.
fn prep_animation_clear(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for (count, data, at) in [(0x5c, 0x60, 0x400), (0x74, 0x78, 0x500)] {
        let n = rng.below(5);
        let w = count_word(rng, n);
        t.w32(obj + count, w);
        t.w32(obj + data, obj + at);
    }
    let n = rng.below(5);
    let w = count_word(rng, n);
    t.w32(obj + 0x7c, w);
    t.w32(obj + 0x80, obj + 0x600);
    let vtable = obj + 0x900;
    let vcall = t.stub(None, 0x7c0, true, 1, 1, Some(rng.next()));
    t.w32(vtable, vcall);
    for k in 0..n {
        t.w32(obj + 0x600 + 0x20 * k, vtable);
    }
}

/// The scene's instance tree: the entry found (or none, or one whose
/// meshes sit at address 0), its 0..4 mesh records, and a binary tree of
/// 0..5 nodes (signed keys) that holds `id` or not.
fn prep_remove_mesh(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfa00);
    *this = scene;
    let tree = scene + 0xf9d4;
    let found = match rng.below(6) {
        0 => 0,
        1 => 0xffff_fff0,
        _ => obj + 0x100,
    };
    t.stub_ret(h14::TREE_FIND_SIGNED, true, 1, 1, found);
    let n = rng.below(5);
    t.w32(obj + 0x110, n);
    t.w32(obj + 0x118, obj + 0x180);
    for k in 0..n {
        let record = obj + 0x200 + 0x10 * k;
        t.w32(obj + 0x180 + 4 * k, record);
        for w in 0..3 {
            let v = rng.next();
            t.w32(record + 4 * w, v);
        }
    }
    // keys k * 10 - 20 in a chain of nodes, each either side of its parent
    let nodes = rng.below(6);
    let mut keys = vec![];
    let mut root = tree;
    for k in 0..nodes {
        let node = obj + 0x400 + 0x20 * k;
        let key = (rng.below(8) as i32 * 10 - 20) as u32;
        t.w32(node, tree);
        t.w32(node + 4, tree);
        t.w32(node + 0xc, key);
        t.w32(node + 0x18, if rng.chance(3) { 0 } else { rng.next() });
        keys.push(key);
        if k == 0 {
            root = node;
        } else {
            // hang it below the previous node on the side its key says
            let parent = obj + 0x400 + 0x20 * (k - 1);
            let side = u32::from((keys[k as usize - 1] as i32) < key as i32);
            t.w32(parent + 4 * side, node);
        }
    }
    t.w32(tree + 0xc, root);
    args[0] = match keys.last() {
        Some(&k) if rng.chance(2) => k,
        _ => (rng.below(8) as i32 * 10 - 25) as u32,
    };
}

/// The registrations (one for `chr` or none), its texture list (or
/// none) with 0..4 pairs whose objects (or none) hold 0..2 buffers.
fn prep_animation_unregister(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    *this = obj;
    let chr = rng.next() | 1;
    args[0] = chr;
    let n = rng.below(5);
    let w = count_word(rng, n);
    t.w32(obj + 0x6c, w);
    t.w32(obj + 0x70, obj + 0x100);
    let hit = if rng.chance(4) {
        n
    } else {
        rng.below(n.max(1))
    };
    for k in 0..n {
        let r = obj + 0x100 + 12 * k;
        t.w32(r, rng.next());
        t.w32(r + 4, if k == hit { chr } else { chr ^ 2 });
        t.w32(r + 8, if rng.chance(5) { 0 } else { obj + 0x400 });
    }
    let pairs = rng.below(5);
    let w = count_word(rng, pairs);
    t.w32(obj + 0x400, w);
    t.w32(
        obj + 0x404,
        if pairs == 0 && rng.chance(2) {
            0
        } else {
            obj + 0x500
        },
    );
    for k in 0..pairs {
        let object = if rng.chance(4) {
            0
        } else {
            obj + 0x600 + 0x10 * k
        };
        t.w32(obj + 0x500 + 8 * k + 4, object);
        for b in 0..2 {
            let v = if rng.chance(3) { 0 } else { rng.next() };
            t.w32(obj + 0x600 + 0x10 * k + 4 * b, v);
        }
    }
}

/// An instance (or none) with 0..3 lighting materials and its two owned
/// or borrowed arrays.
fn prep_instance_destroy(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let chr = obj + 0x200;
    args[0] = if rng.chance(5) { 0 } else { chr };
    // 0x100 now and then: the count is the low 31 bits, not a byte
    let count = if rng.chance(8) { 0x100 } else { rng.below(4) };
    let w = count_word(rng, count);
    t.w32(chr, w);
    t.w32(chr + 4, obj + 0x300);
    for k in 0..count {
        let v = rng.next();
        t.w32(obj + 0x300 + 4 * k, v);
    }
    let owned = rng.below(3);
    let w = count_word(rng, owned);
    t.w32(chr + 8, w);
    for off in [0xc, 0x10, 0x18] {
        let v = rng.next();
        t.w32(chr + off, v);
    }
}

/// The array (full or not, its data at 0 once), the asset (or none),
/// whether it is a model, and cursor runs over its objects.
fn prep_asset_item(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let array = obj + 0x100;
    let n = match rng.below(4) {
        0 => [0xffff_ffff, 0xffff_fff0, 0xffff_ffe9][rng.below(3) as usize],
        1 => 0,
        _ => rng.below(20),
    };
    t.w32(array, n);
    t.w32(array + 4, n.wrapping_add(rng.below(3)).wrapping_sub(1));
    let data = if n == 0 && rng.chance(3) {
        0
    } else {
        obj + 0x180
    };
    t.w32(array + 8, data);
    let asset = obj + 0x800;
    t.w32(
        asset + 0x1c,
        if rng.chance(2) {
            asset + 0x1c
        } else {
            obj + 0x880
        },
    );
    let area = t.zeros(0x800);
    let objects = t.zeros(0x400);
    for k in 0..8 {
        t.w32(area + 0x100 * k + 0x20, objects + 0x40 * k);
    }
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script(h14::CURSOR_BEGIN, h14::CURSOR_NEXT, &steps);
    let model = if rng.chance(2) { 0x100 } else { 0x101 };
    t.stub_ret(h14::IS_MODEL, true, 1, 1, model);
    args[0] = obj + 0x600;
    args[1] = rng.next();
    args[2] = array;
    args[3] = if rng.chance(5) { 0 } else { asset };
    args[4] = rng.below(8);
}

/// A shadow size: sides from 0 up, equal, NaN or infinite; `n` around
/// them.
fn prep_fit_size(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let side = |rng: &mut Rng| -> f32 {
        match rng.below(8) {
            0 => 0.0,
            1 => f32::NAN,
            2 => f32::INFINITY,
            3 => -(rng.below(100) as f32),
            4 => rng.float(),
            _ => (1 + rng.below(4096)) as f32 / [1.0, 3.0, 7.0][rng.below(3) as usize],
        }
    };
    let w = side(rng);
    let h = if rng.chance(5) { w } else { side(rng) };
    args[0] = w.to_bits();
    args[1] = h.to_bits();
    args[2] = obj + 0x10;
    args[3] = obj + 0x14;
    args[4] = match rng.below(4) {
        0 => rng.next(),
        1 => w as i32 as u32,
        _ => rng.below(4097),
    };
    t.w32(obj + 0x10, rng.next());
    t.w32(obj + 0x14, rng.next());
}

/// A name (or none), a cancelled load, a prototype found, the allocation
/// failing, the load failing, and ids near the shadow ranges.
fn prep_character_load(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    letters(t, obj + 0x700, 1 + rng.below(12), rng);
    args[0] = if rng.chance(6) { 0 } else { obj + 0x700 };
    args[1] = *[
        0x10fb,
        0x10fc,
        0x10fd,
        0x10fe,
        0x1100,
        800,
        801,
        899,
        900,
        0,
        0xffff_ffff,
    ]
    .get(rng.below(14) as usize)
    .unwrap_or(&rng.below(0x2000));
    let cancel = if rng.chance(5) { 0x301 } else { 0x300 };
    t.stub_ret(h14::IS_CANCEL_LOAD, false, 1, 0, cancel);
    let prototype = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h14::PROTOTYPE_BY_FILENAME, true, 1, 1, prototype);
    let p = if rng.chance(5) { 0 } else { obj + 0x300 };
    t.stub_ret(h14::OPERATOR_NEW, false, 1, 0, p);
    let character = obj + 0x400;
    t.stub_ret(h14::CHARACTER_CTOR, true, 1, 1, character);
    let vtable = obj + 0x480;
    let vcall = t.stub(None, 0x7d0, true, 1, 1, Some(rng.next()));
    t.w32(character, vtable);
    t.w32(vtable, vcall);
    let failed = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.stub_ret(h14::SCENE_LOAD_CHARACTER_ASYNC, true, 2, 2, failed);
}

/// The geometry's vtable (its set-up `0x18` failing or not), the shader
/// with up to 30 slots (or a negative count).
fn prep_set_shader(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let vtable = obj + 0x1f00;
    t.w32(obj, vtable);
    for off in [0x14, 0x1c, 0x20, 0x24] {
        let s = t.stub(
            None,
            0x7e0 + off,
            true,
            if off == 0x14 { 3 } else { 0 },
            if off == 0x14 { 3 } else { 0 },
            Some(rng.next()),
        );
        t.w32(vtable + off, s);
    }
    let r = if rng.chance(2) { 0 } else { rng.next() };
    let s = t.stub(None, 0x7f8, true, 3, 3, Some(r));
    t.w32(vtable + 0x18, s);
    let shader = obj + 0x40;
    t.w32(obj + 0x24, shader);
    let n = match rng.below(5) {
        0 => 0,
        1 => 30,
        2 => 0xffff_fffe,
        _ => rng.below(31),
    };
    t.w32(shader + 0x90, n);
    t.w32(shader + 0x94, obj + 0x800);
    args[0] = rng.next();
    args[1] = rng.next();
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

const DESTROY_STUBS: &[(u32, K)] = &[
    (h14::CHARACTER_UNREGIST_LOADING, K::Cdecl(1)),
    (h14::SCENE_REMOVE_MESH_INSTANCE, K::This(1)),
    (h14::CHARACTER_LIGHTING_INSTANCE, K::Getter),
    (h14::DESTROY_UNIQUE_MATERIAL_PARAMETERS, K::This(1)),
    (h14::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h14::UNREGISTER_CHARACTER_INSTANCE, K::This(1)),
    (h14::SHADOW_INSTANCE, K::Getter),
    (h14::SHADOW_REMOVE, K::This(1)),
    (h14::UNREGISTER_DELEGATION, K::This(3)),
    (h14::SCENE_REMOVE_CHR_FROM_PIPELINE, K::This(1)),
    (h14::DESTROY_CLOTH_INSTANCES, K::This(0)),
    (h14::CHARACTER_REMOVE_INSTANCE, K::This(1)),
    (h14::DESTROY_MESHES, K::This(0)),
    (h14::PHYRE_FREE, K::Cdecl(1)),
    (h14::OPERATOR_DELETE, K::Cdecl(1)),
];

const SET_SHADER_STUBS: &[(u32, K)] = &[
    (h14::NEW_POOL_ARRAY, K::This(1)),
    (h14::NEW_POOL_OBJECT56, K::This(1)),
    (h14::NEW_POOL_OBJECT28, K::This(1)),
    (h14::NEW_POOL_OBJECT16, K::This(2)),
    (h14::NEW_POOL_6599C0, K::This(2)),
];

#[rustfmt::skip]
const CASES14: &[Case] = &[
    framed(Case { prep: Some(prep_find_global), ..method("scene_find_global_object", 0x0065_18c0, This::Object, 0, &[], |m, this, _| u(gh14::scene_find_global_object(m, frame_ebp(), this))) }, 0x28, 0),
    Case { prep: Some(prep_resize_targets), ..method("post_resize_targets", 0x0069_7f20, This::Object, 2, &[(h14::RESIZE_A233F0, K::This(2)), (h14::RESIZE_A23200, K::This(2)), (h14::RESIZE_TARGET, K::Cdecl(4))], |m, this, a| u(gh14::post_resize_targets(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_claim28), ..method("scene_new_dynamic_mesh", 0x0065_94c0, This::Object, 1, &[(h14::DYNAMIC_MESH_CTOR, K::This(0))], |m, this, a| u(gh14::scene_new_dynamic_mesh(m, this, a[0]))) },
    Case { prep: Some(prep_claim16), ..method("scene_new_mesh_instance", 0x0065_95d0, This::Object, 2, &[(h14::DYNAMIC_MESH_INSTANCE_CTOR, K::This(1))], |m, this, a| u(gh14::scene_new_mesh_instance(m, this, a[0], a[1]))) },
    Case { prep: Some(prep_animation_clear), ..method("texture_animation_clear", 0x006b_2f50, This::Object, 0, &[(h14::PHYRE_FREE, K::Cdecl(1))], |m, this, _| { gh14::texture_animation_clear(m, this); Out::None }) },
    framed(Case { prep: Some(prep_remove_mesh), ..method("scene_remove_mesh_instance", 0x0066_8680, This::Object, 1, &[(h14::CLUSTER_DELETE_OBJECT, K::This(1)), (h14::OPERATOR_DELETE, K::Cdecl(1)), (h14::TREE_REMOVE, K::This(1)), (h14::USER_FREE, K::Cdecl(1)), (h14::FREE_LIST_DEALLOCATE, K::This(1))], |m, this, a| { let slots = slots_at(m, a); u(gh14::scene_remove_mesh_instance(m, this, slots)) }) }, 0, 4),
    Case { prep: Some(prep_animation_unregister), ..method("texture_animation_unregister", 0x006b_4330, This::Object, 1, &[(h14::PHYRE_PRINTF, K::Cdecl(3)), (h14::ARRAY_DELETE, K::Cdecl(1)), (h14::OPERATOR_DELETE, K::Cdecl(1)), (h14::PHYRE_FREE, K::Cdecl(1))], |m, this, a| u(gh14::texture_animation_unregister(m, this, a[0]))) },
    framed(Case { prep: Some(prep_instance_destroy), ..cdecl("character_instance_destroy", 0x0063_a030, 2, DESTROY_STUBS, |m, _, a| { let slots = slots_at(m, a); u(gh14::character_instance_destroy(m, slots, a[0], a[1])) }) }, 0, 8),
    framed(Case { prep: Some(prep_asset_item), ..method("scene_on_character_asset", 0x0065_ae40, This::Stdcall, 5, &[(h14::ARRAY_RESERVE, K::This(1)), (h14::PUSH_ASSET, K::This(1))], |m, _, a| { let slots = slots_at(m, a); u(gh14::scene_on_character_asset(m, frame_ebp(), slots, a[0], a[1], a[2], a[3], a[4])) }) }, 0x20, 20),
    framed(Case { prep: Some(prep_fit_size), ..method("shadow_fit_size", 0x006d_23d0, This::Stdcall, 5, &[], |m, _, a| { let slots = slots_at(m, a); gh14::shadow_fit_size(m, slots, a[0], a[1], a[2], a[3], a[4]); Out::None }) }, 0, 20),
    framed(with_cookie(Case { prep: Some(prep_character_load), ..cdecl("character_load", 0x0063_d140, 2, &[(h14::LOADED_CHARACTER_TABLE, K::Getter), (h14::SPRINTF_SLOT, K::ImportC(3)), (h14::PHYRE_PRINTF, K::Cdecl(2))], |m, _, a| u(gh14::character_load(m, frame_ebp(), a[0], a[1]))) }), 0x108, 0),
    framed(with_cookie(Case { prep: Some(prep_set_shader), ..method("geometry_set_shader", 0x006a_46e0, This::Object, 2, SET_SHADER_STUBS, |m, this, a| u(gh14::geometry_set_shader(m, frame_ebp(), this, a[0], a[1]))) }), 0x1e4, 0),
];

#[test]
fn batch14_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES14, n, &mut Rng(0x1414_b014));
    });
}

// ---- round 5 batch 15 (ffx_prender::h3_glue15) ----

impl Twins {
    /// `InterlockedCompareExchange(dest, exchange, comparand)` through
    /// the import slot `slot`: a real exchange, then logged (stdcall).
    fn stub_cas(&mut self, slot: u32) {
        let log = self.stub(None, slot, false, 3, 3, None);
        let mut c = vec![0x8b, 0x4c, 0x24, 0x04]; // mov ecx, [esp+4]
        c.extend([0x8b, 0x01]); // mov eax, [ecx]
        c.extend([0x3b, 0x44, 0x24, 0x0c]); // cmp eax, [esp+0xc]
        c.extend([0x75, 0x06]); // jne +6
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        c.extend([0x89, 0x11]); // mov [ecx], edx
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let at = self.rebase(slot);
        self.patch(at, &code.to_le_bytes());
    }

    /// A cdecl allocator stub at `sym` returning `base + size`, logged.
    fn stub_alloc(&mut self, sym: u32, base: u32) {
        let log = self.stub(None, sym, false, 1, 0, None);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.push(0x05);
        c.extend(base.to_le_bytes()); // add eax, base
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A cdecl import through the slot `slot` returning `ret`.
    fn stub_import_c(&mut self, slot: u32, n: u32, ret: u32) {
        let s = self.stub(None, slot, false, n, 0, Some(ret));
        let at = self.rebase(slot);
        self.patch(at, &s.to_le_bytes());
    }
}

/// The 64 free-list flags (one free now and then), their entries, the
/// delegation's buffer (none, too small or large enough), the data (or
/// none) and the list (a null slot or full).
fn prep_register(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.stub_cas(h15::INTERLOCKED_CAS);
    let free = if rng.chance(3) { 64 } else { rng.below(64) };
    let d = obj + 0x400;
    for i in 0..64 {
        t.w32(
            obj + 0xc + 8 * i,
            if i == free { 0 } else { 1 + rng.below(3) },
        );
        t.w32(obj + 8 + 8 * i, if rng.chance(4) { 0 } else { d });
    }
    t.stub_ret(h15::OPERATOR_NEW, false, 1, 0, obj + 0x500);
    for at in [d, obj + 0x500] {
        t.w32(at + 4, if rng.chance(3) { 0 } else { obj + 0x600 });
        t.w32(at + 8, rng.below(0x30));
    }
    let size = rng.below(0x40);
    args[0] = if rng.chance(4) { 0 } else { obj + 0x700 };
    args[1] = size;
    let n = rng.below(5);
    let w = count_word(rng, n);
    for k in 0..n {
        t.w32(
            obj + 0x800 + 4 * k,
            if rng.chance(4) { 0 } else { rng.next() | 1 },
        );
    }
    t.w32(obj, w);
    t.w32(obj + 4, obj + 0x800);
}

/// An instance (or the null one), a geometry already there or not, and
/// allocations at distinct places.
fn prep_guildmap(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if !t.snapshot {
        t.global(h15::VFX_RENDER_DATA_TABLE.sym, obj + 0x40);
    }
    args[0] = if rng.chance(3) { 0 } else { rng.below(4) };
    args[1] = if rng.chance(2) { 0 } else { rng.next() };
    let found = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h15::GET_DYNAMIC_GEOMETRY, true, 2, 2, found);
    t.stub_alloc(h15::OPERATOR_NEW, obj + 0x400);
    t.stub_ret(h15::GUILD_MAP_GEOMETRY_CTOR, true, 0, 0, obj + 0x800);
}

/// The cloth (or none) with 0..4 vertex-buffer lists (some null) of
/// 0..3 buffers (some null) whose storage lies in the cloth heap or not,
/// and the instance table holding it 0..2 times.
fn prep_cloth_destroy(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let base = obj + 0x1000;
    t.global(h15::CLOTH_BASE.sym, base);
    t.global(h15::CLOTH_MSPACE.sym, rng.next());
    let pick = |rng: &mut Rng| match rng.below(5) {
        0 => base,
        1 => base + 0xf_ffff,
        2 => base + 0x10_0000,
        3 => base - 1,
        _ => base + rng.below(0x10_0000),
    };
    let cloth_at = obj + 0x100;
    let cloth = if rng.chance(6) { 0 } else { cloth_at };
    args[0] = cloth;
    let lists = rng.below(5);
    t.w32(cloth_at + 0x14, lists);
    for i in 0..lists {
        let list = obj + 0x400 + 0x40 * i;
        t.w32(
            cloth_at + 0x30 + 4 * i,
            if rng.chance(4) { 0 } else { list },
        );
        let n = rng.below(4);
        t.w32(list, n);
        for j in 0..n {
            let buffer = obj + 0x800 + 0x100 * i + 0x50 * j;
            t.w32(list + 4 + 4 * j, if rng.chance(4) { 0 } else { buffer });
            let storage = pick(rng);
            t.w32(buffer + 0x4c, storage);
        }
    }
    let n = rng.below(5);
    t.global(h15::CLOTH_INSTANCE_COUNT.sym, n);
    let table = t.rebase(h15::CLOTH_INSTANCES);
    for k in 0..4 {
        let v = if rng.chance(3) { cloth } else { rng.next() };
        t.w32(table + 4 * k, v);
    }
    // the base itself now and then, so the cloth is in its heap
    if cloth != 0 && rng.chance(4) {
        t.global(h15::CLOTH_BASE.sym, cloth_at);
    }
}

/// The scene's pool (its counter near the end, small, negative or
/// anywhere), the variant's buffer size, the clusters' use counts and
/// the outputs (or none).
fn prep_create_material(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfc00);
    *this = scene;
    let s = match rng.below(4) {
        0 => 3990 + rng.below(12),
        1 => rng.below(64),
        2 => (rng.below(3) as i32 - 3) as u32,
        _ => rng.below(4000),
    };
    t.w32(scene + 0xfbfc, s);
    t.w32(
        scene + 0xfbe8,
        (obj + 0x200).wrapping_sub(s.wrapping_mul(0xe0)),
    );
    t.w32(
        scene + 0xfbec,
        (obj + 0x300).wrapping_sub(s.wrapping_mul(0x18)),
    );
    t.w32(scene + 0xfbf8, (obj + 0x1000).wrapping_sub(s));
    let run = rng.below(12) as usize;
    let mut bytes = vec![1u8; run];
    bytes.push(0);
    t.write(obj + 0x1001, &bytes);
    for k in 0..64 {
        let v = match rng.below(4) {
            0 => rng.below(3),
            1 => (rng.below(3) as i32 - 2) as u32,
            _ => 3 + rng.below(1000),
        };
        t.w32(scene + 0x2774 + 4 * k, v);
    }
    let variant = obj + 0x100;
    // the buffer size: around 0xe0, now and then over 0xff
    let low = [0xdfu8, 0xe0, 0xe1, 0x00, 0xff][rng.below(5) as usize];
    let high = u8::from(rng.chance(6));
    t.write(variant + 0x30, &[low, high]);
    args[0] = variant;
    args[1] = if rng.chance(4) {
        rng.next() | 1
    } else {
        scene + 0x12d4
    };
    args[2] = if rng.chance(3) { 0 } else { obj + 0x600 };
    args[3] = if rng.chance(3) { 0 } else { obj + 0x604 };
    t.stub_ret(h15::CREATE_MATERIAL, true, 3, 3, obj + 0x700);
}

/// The matrix, the eight floats, the count and the flag (equal to the
/// scene's or not).
fn prep_render_texture(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfd00);
    *this = scene;
    let m = obj + 0x100;
    for k in 0..16 {
        let f = rng.float();
        t.w32(m + 4 * k, f.to_bits());
    }
    let on = rng.below(3);
    t.write(
        scene + 0xfc00,
        &[if rng.chance(2) {
            on as u8
        } else {
            rng.below(3) as u8
        }],
    );
    args[0] = on | if rng.chance(2) { rng.next() & !0xff } else { 0 };
    args[1] = m;
    args[10] = rng.word();
}

/// A path with "hiku21" (where the stubbed `strstr` points) or not, the
/// language (each case and out of range), and the flag.
fn prep_field_map_load(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let path = obj + 0x700;
    letters(t, path, 20, rng);
    let found = if rng.chance(4) { 0 } else { path + 4 };
    t.stub_import_c(h15::STRSTR_SLOT, 2, found);
    let language = rng.below(21);
    t.stub_ret(h15::GET_LANGUAGE, false, 0, 0, language);
    let hides = rng.below(5);
    t.global(h15::CHR_F117_HIDE_COUNT.sym, hides);
    args[0] = path;
    args[1] = rng.word();
}

/// The water frame (in range, at the wrap, past it) and the counter.
fn prep_wave(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let frame = match rng.below(4) {
        0 => 0x45,
        1 => 0x46 + rng.below(4),
        _ => rng.below(0x46),
    };
    let at = t.rebase(h15::WATER_FRAME.sym);
    t.write(at, &[frame as u8]);
    let counter = rng.next() as u8;
    let at = t.rebase(h15::WATER_COUNTER.sym);
    t.write(at, &[counter]);
}

const REGISTER_STUBS: &[(u32, K)] = &[
    (h15::OPERATOR_DELETE, K::Cdecl(1)),
    (h15::ARRAY_NEW, K::Cdecl(1)),
    (h15::MEMSET, K::Cdecl(3)),
    (h15::MEMCPY, K::Cdecl(3)),
    (h15::WAIT_FOR_SINGLE_OBJECT, K::Import(2)),
    (h15::RELEASE_MUTEX, K::Import(1)),
    (h15::PHYRE_MALLOC_ALIGNED, K::Cdecl(2)),
    (h15::DELEGATION_GROW_PUSH, K::This(2)),
];

const FIELD_MAP_STUBS: &[(u32, K)] = &[
    (h15::LEVEL_ANALYSIS_NAME, K::Cdecl(1)),
    (h15::CURRENT_LEVEL_NAME, K::This(0)),
    (h15::VIEWER_MANAGER_INSTANCE, K::Getter),
    (h15::SPRINTF_SLOT, K::ImportC(3)),
    (h15::PRINTF_SLOT, K::ImportC(2)),
    (h15::TEXTURE_VIDEO_LOAD, K::This(1)),
    (h15::FIELD_MAP_LOAD_ASYNC, K::This(2)),
];

#[rustfmt::skip]
const CASES15: &[Case] = &[
    framed(Case { prep: Some(prep_register), ..method("delegation_register", 0x006a_de20, This::Object, 4, REGISTER_STUBS, |m, this, a| u(gh15::delegation_register(m, frame_ebp(), this, a[0], a[1], a[2], a[3]))) }, 4, 0),
    Case { prep: Some(prep_guildmap), ..cdecl("guildmap_create", 0x0064_1510, 3, &[(h15::VFX_TABLE_INSERT, K::This(3))], |m, _, a| u(gh15::guildmap_create(m, a[0], a[1], a[2]))) },
    framed(Case { prep: Some(prep_cloth_destroy), ..cdecl("cloth_vertex_buffers_destroy", 0x0063_a160, 1, &[(h15::MSPACE_FREE, K::Cdecl(2)), (h15::USER_FREE, K::Cdecl(1))], |m, _, a| { let slots = slots_at(m, a); let r = gh15::cloth_vertex_buffers_destroy(m, slots, a[0]); // without a cloth the original leaves the caller's eax
        if a[0] == 0 { Out::None } else { u(r) } }) }, 0, 4),
    Case { prep: Some(prep_create_material), ..method("scene_create_material", 0x0065_9710, This::Object, 4, &[(h15::CREATE_MATERIAL_PREALLOCATED, K::This(2))], |m, this, a| u(gh15::scene_create_material(m, this, a[0], a[1], a[2], a[3]))) },
    framed(Case { prep: Some(prep_render_texture), float_args: true, ..method("scene_set_render_world_to_texture", 0x0066_d000, This::Object, 11, &[(h15::SPRINTF_SLOT, K::ImportC(3))], |m, this, a| { slots_at(m, a); u(gh15::scene_set_render_world_to_texture(m, frame_ebp(), this, a[0], a[1], arr(&a[2..10]), a[10])) }) }, 0x40, 44),
    Case { prep: Some(prep_field_map_load), ..cdecl("field_map_load", 0x0064_0210, 2, FIELD_MAP_STUBS, |m, _, a| u(gh15::field_map_load(m, a[0], a[1]))) },
    framed(with_cookie(Case { prep: Some(prep_wave), ..cdecl("scene_draw_main_menu_wave", 0x0065_b5b0, 0, &[(h15::UNIFY_FILENAME, K::Cdecl(3)), (h15::SPRINTF_SLOT, K::ImportC(4)), (h15::DRAW_UI_ELEMENT, K::Cdecl(5))], |m, _, _| u(gh15::scene_draw_main_menu_wave(m, frame_ebp()))) }), 0x29c, 0),
];

#[test]
fn batch15_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES15, n, &mut Rng(0x1515_b015));
    });
}

// ---- round 5 batch 16 (ffx_prender::h3_glue16) ----

/// The six arrays (counts 0..4, negative now and then; the masked one
/// with its owned bit), `mesh` in one of them (or none, or twice).
fn prep_remove_vfx(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfc00);
    *this = scene;
    let mesh = rng.next() | 1;
    args[0] = mesh;
    let count = |rng: &mut Rng| -> u32 {
        if rng.chance(6) {
            (rng.below(3) as i32 - 3) as u32
        } else {
            rng.below(5)
        }
    };
    let hit = |rng: &mut Rng| -> u32 { if rng.chance(4) { mesh } else { rng.next() } };
    let n = count(rng);
    t.w32(scene + 0xf8cc, n);
    for k in 0..4 {
        let v = hit(rng);
        t.w32(scene + 0x38cc + 4 * k, v);
    }
    let n = count(rng);
    t.w32(scene + 0xf8d0, n);
    for k in 0..4 {
        let instance = obj + 0x100 + 0x20 * k;
        t.w32(
            scene + 0x68cc + 4 * k,
            if rng.chance(4) { 0 } else { instance },
        );
        let v = hit(rng);
        t.w32(instance + 0xc, v);
    }
    for (c, d, at) in [
        (0x3878, 0x3880, 0x300),
        (0x3854, 0x385c, 0x340),
        (0x3860, 0x3868, 0x380),
    ] {
        let n = count(rng);
        t.w32(scene + c, n);
        t.w32(scene + d, obj + at);
        for k in 0..4 {
            let v = hit(rng);
            t.w32(obj + at + 4 * k, v);
        }
    }
    let n = rng.below(5);
    let w = count_word(rng, n);
    t.w32(scene + 0x3884, w);
    t.w32(scene + 0x3888, obj + 0x3c0);
    for k in 0..4 {
        let v = hit(rng);
        t.w32(obj + 0x3c0 + 4 * k, v);
    }
}

/// The map's two clusters (or none), the fog record's array (none,
/// owned or not, its data null).
fn prep_map_shutdown(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    for off in [0x10d4, 0x1128] {
        t.w32(scene + off, if rng.chance(3) { 0 } else { rng.next() | 1 });
    }
    let fog = obj + 0x100;
    t.w32(scene + 0xcc, fog);
    let n = rng.below(3);
    let w = count_word(rng, n);
    t.w32(fog + 0x9c, w);
    t.w32(fog + 0xa0, if rng.chance(3) { 0 } else { rng.next() | 1 });
    t.w32(scene + 0x10cc, rng.next());
}

/// A texture video (or none) with its decoder busy or not, 0..4 effects
/// (or a negative count) and its three textures (some null).
fn prep_video_unload(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1100);
    *this = scene;
    t.write(scene + 0xdd0, &[u8::from(!rng.chance(4))]);
    t.w32(scene + 0xdcc, u32::from(!rng.chance(2)));
    let n = if rng.chance(6) {
        0xffff_ffff
    } else {
        rng.below(5)
    };
    t.w32(scene + 0x10c0, n);
    for (k, off) in [0xdd8, 0xddc, 0xde0].into_iter().enumerate() {
        let v = if rng.chance(4) {
            0
        } else {
            obj + 0x400 + 0x40 * k as u32
        };
        t.w32(scene + off, v);
    }
    t.global(h16::PPP_SPECIAL_PART.sym, rng.next());
    for off in [0xdc8, 0x10c8] {
        let v = rng.next();
        t.w32(scene + off, v);
    }
}

/// Eight samplers, some locked (bit 7 of `+0x1c`).
fn prep_scene_samplers(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for k in 0..8 {
        let flags = rng.next() & !0x80 | if rng.chance(3) { 0x80 } else { 0 };
        t.w32(obj + 0x964 + 0x24 * k + 0x1c, flags);
    }
}

/// The map's camera (or none) with its values (`+0x108` below 0.35 or
/// not, NaN), the level (each special one) and the scenario flag.
fn prep_post_params(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb00);
    *this = scene;
    let camera = obj + 0x200;
    t.w32(scene + 0x4f8, if rng.chance(4) { 0 } else { camera });
    t.w32(scene + 0xcc, obj + 0x400);
    for k in 0..7 {
        let f = rng.float();
        t.w32(camera + 0x104 + 4 * k, f.to_bits());
    }
    let low = [f32::from_bits(0x3eb3_3332), 0.35, 0.1, 0.5, f32::NAN, -1.0][rng.below(6) as usize];
    t.w32(camera + 0x108, low.to_bits());
    let level = [0x199, 0x12ea, 0x1733, 0x19a, rng.below(0x2000)][rng.below(5) as usize];
    t.w32(scene + 0xfaf4, level);
    let flag = if rng.chance(2) { 0 } else { rng.next() };
    let flag = flag & if rng.chance(2) { 0xffff_0000 } else { u32::MAX };
    t.stub_ret(h16::GET_SCENARIO_FLAG, false, 0, 0, flag);
}

/// The layout's arrays (sizes right or not, owned or not, null) and an
/// allocator that fails now and then (only where an array is left).
fn prep_layout(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let words = [(0x10, 0x14, 1, 0x300), (8, 0xc, 4, 0x400)];
    // a null array only where a new one replaces it (the original writes
    // through what it ends with)
    let fail = rng.chance(5);
    for (count, data, n, at) in words {
        let c = if rng.chance(3) { n } else { rng.below(6) };
        let w = count_word(rng, c);
        t.w32(obj + count, w);
        // null, the old array, or (with the allocator below) the very
        // block the new one gets
        let p = if !fail && c != n && rng.chance(4) {
            0
        } else if !fail && rng.chance(4) {
            obj + 0x800 + [0x10, 0x30][usize::from(n == 4)]
        } else {
            obj + at
        };
        t.w32(obj + data, p);
    }
    if fail {
        t.stub_ret(h16::PHYRE_MALLOC_ALIGNED, false, 2, 0, 0);
    } else {
        t.stub_alloc(h16::PHYRE_MALLOC_ALIGNED, obj + 0x800);
    }
    args[0] = rng.next();
    args[1] = rng.next();
}

const MAP_SHUTDOWN_STUBS: &[(u32, K)] = &[
    (h16::WORLD_REMOVE_CLUSTER, K::This(1)),
    (h16::PHYRE_FREE, K::Cdecl(1)),
    (h16::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h16::TEXTURE_ANIMATION_CLEAR, K::This(0)),
    (h16::MORPH_INSTANCE, K::Getter),
    (h16::MORPH_RESET, K::This(0)),
    (h16::POST_SET_FIRST_ENTRY, K::This(1)),
    (h16::GET_APPLICATION, K::Getter),
];

const VIDEO_UNLOAD_STUBS: &[(u32, K)] = &[
    (h16::SLEEP, K::Cdecl(1)),
    (h16::VFX_DESTROY, K::Cdecl(2)),
    (h16::VIDEO_TERMINATE, K::This(0)),
    (h16::FMV_INSTANCE, K::Getter),
    (h16::FMV_ON_TEXTURE_VIDEO_STOP, K::This(0)),
    (h16::TEXTURE_IS_BOUND, K::This(0)),
    (h16::TEXTURE_UNBIND, K::This(0)),
    (h16::TEXTURE_RELEASE, K::This(0)),
    (h16::PHYRE_FREE, K::Cdecl(1)),
    (h16::SET_TEXTURE_VIDEO_INFO, K::Cdecl(2)),
];

#[rustfmt::skip]
const CASES16: &[Case] = &[
    Case { prep: Some(prep_remove_vfx), ..method("scene_remove_vfx_from_render_array", 0x0066_87a0, This::Object, 1, &[], |m, this, a| u(gh16::scene_remove_vfx_from_render_array(m, this, a[0]))) },
    Case { prep: Some(prep_map_shutdown), ..method("scene_field_map_shutdown", 0x0065_d0c0, This::Object, 0, MAP_SHUTDOWN_STUBS, |m, this, _| u(gh16::scene_field_map_shutdown(m, this))) },
    Case { prep: Some(prep_video_unload), ..method("scene_texture_video_unload", 0x0067_0760, This::Object, 0, VIDEO_UNLOAD_STUBS, |m, this, _| { let video = m.read::<u8>(this + 0xdd0) != 0; let r = gh16::scene_texture_video_unload(m, this); // without a video the original leaves the caller's eax
        if video { u(r) } else { Out::None } }) },
    Case { prep: Some(prep_scene_samplers), ..method("scene_setup_samplers", 0x0065_f290, This::Object, 0, &[(h16::SAMPLER_REGISTER, K::This(0))], |m, this, _| u(gh16::scene_setup_samplers(m, this))) },
    Case { prep: Some(prep_post_params), ..method("scene_setup_post_process_params", 0x0066_f140, This::Object, 0, &[(h16::PHYRE_PRINTF, K::Cdecl(2))], |m, this, _| u(gh16::scene_setup_post_process_params(m, this))) },
    Case { prep: Some(prep_layout), ..method("layout_setup", 0x0067_8bd0, This::Object, 2, &[(h16::MEMSET, K::Cdecl(3)), (h16::PHYRE_FREE, K::Cdecl(1))], |m, this, a| u(gh16::layout_setup(m, this, a[0], a[1]))) },
];

#[test]
fn batch16_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES16, n, &mut Rng(0x1616_b016));
    });
}

// ---- round 5 batch 17 (ffx_prender::h3_glue17) ----

/// The frame records (ages around 16, some unused) with buffers (some
/// null), the video's flags (playing, paused, `+0xdd1`), the pause state
/// against `g_PauseOn`, the playback running or not, 0..4 effects.
fn prep_update_video(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0e00);
    *this = scene;
    for k in 0..4 {
        let record = scene + 0xd68 + 0x18 * k;
        let age = match rng.below(4) {
            0 => 0,
            1 => 16,
            2 => (rng.below(3) as i32 - 2) as u32,
            _ => rng.below(20),
        };
        t.w32(record, age);
        for b in 1..=5 {
            t.w32(
                record + 4 * b,
                if rng.chance(3) { 0 } else { rng.next() | 1 },
            );
        }
    }
    for off in [0xdd0, 0xdd1, 0x10c9, 0x10ca] {
        t.write(scene + off, &[u8::from(!rng.chance(3))]);
    }
    let pause = rng.below(2);
    t.global(h17::PAUSE_ON.sym, pause);
    t.w32(
        scene + 0x1_0dc4,
        if rng.chance(2) { pause } else { pause ^ 1 },
    );
    t.w32(
        scene + 0x10c0,
        if rng.chance(6) {
            0xffff_ffff
        } else {
            rng.below(5)
        },
    );
    t.w32(scene + 0xd0, rng.next());
    for off in [0xdd8, 0xddc, 0xde0] {
        let v = rng.next();
        t.w32(scene + off, v);
    }
    let playing = if rng.chance(4) { 0x100 } else { 0x101 };
    t.stub_ret(h17::VIDEO_IS_PLAYING, true, 0, 0, playing);
    t.stub_ret(h17::OPERATOR_NEW, false, 1, 0, obj + 0x100);
}

/// The singletons there or not (the first, the delegation helper, the
/// scene) and allocations failing now and then.
fn prep_initialize(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    for g in [
        h17::FIRST_SINGLETON.sym,
        h17::RENDER_DELEGATION_HELPER.sym,
        h17::SCENE.sym,
    ] {
        let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
        t.global(g, v);
    }
    let p = if rng.chance(5) { 0 } else { obj + 0x100 };
    t.stub_ret(h17::OPERATOR_NEW, false, 1, 0, p);
    for g in [
        h17::LOADING_SCREEN.sym,
        h17::GRAPHICS_UP.sym,
        h17::CLOTH_BASE.sym,
        h17::CLOTH_MSPACE.sym,
    ] {
        let v = rng.next();
        t.global(g, v);
    }
}

/// The window (16:9, narrower, wider, sizes past 2^31, zero), the target
/// aspect (16:9 or another) and `keep`.
fn prep_viewport(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let app = obj + 0x100;
    t.stub_ret(h17::GET_APPLICATION, false, 0, 0, app);
    let (w, h) = match rng.below(6) {
        0 => (1920, 1080),
        1 => (1280, 720),
        2 => (1024, 768),
        3 => (2560, 1080),
        4 => (rng.next(), rng.next()),
        _ => (rng.below(4000), rng.below(4000)),
    };
    t.w32(app + 0x26c, w);
    t.w32(app + 0x270, h);
    let aspect = match rng.below(4) {
        0 => 16.0 / 9.0,
        1 => rng.float(),
        _ => (1 + rng.below(4000)) as f32 / (1 + rng.below(3000)) as f32,
    };
    t.global(h17::TARGET_ASPECT.sym, aspect.to_bits());
    if !t.snapshot {
        t.global(h17::IGGY_MENU_MANAGER.sym, obj + 0x200);
    }
    args[0] = rng.next();
    args[1] = if rng.chance(5) { rng.next() | 1 } else { 0 };
}

const UPDATE_VIDEO_STUBS: &[(u32, K)] = &[
    (h17::OPERATOR_DELETE, K::Cdecl(1)),
    (h17::VIDEO_TOGGLE_PAUSE, K::This(0)),
    (h17::QUEUE_CALLBACK, K::This(2)),
    (h17::VFX_UPDATE, K::Cdecl(2)),
    (h17::DYNAMIC_GEOMETRY_UPDATE_TEXTURE, K::Cdecl(2)),
];

/// Every callee of `graphicInitialize`, stubbed.
fn initialize_stubs() -> Vec<(u32, K)> {
    let mut s = vec![
        (h17::TTY_WRITE, K::Cdecl(2)),
        (h17::INIT_FIRST, K::Cdecl(0)),
        (h17::FIRST_SINGLETON_CTOR, K::This(0)),
        (h17::FIRST_SINGLETON_SETUP, K::This(0)),
        (h17::RENDER_DELEGATION_HELPER_CTOR, K::This(0)),
        (h17::LOADING_SCREEN_CTOR, K::This(0)),
        (h17::INIT_AFTER_LOADING_SCREEN, K::Cdecl(0)),
        (h17::SCENE_CTOR, K::This(0)),
        (h17::SCENE_INIT_POST_PROCESS, K::This(0)),
        (h17::READ_MONSTER_NAMES, K::Cdecl(0)),
        (h17::ALIGNED_MALLOC, K::Cdecl(2)),
        (h17::CREATE_MSPACE, K::Cdecl(3)),
    ];
    s.extend(
        h17::CREATORS_A
            .iter()
            .chain(&h17::CREATORS_B)
            .map(|&f| (f, K::Cdecl(0))),
    );
    s
}

#[rustfmt::skip]
const CASES17: &[Case] = &[
    Case { prep: Some(prep_update_video), ..method("scene_update_texture_video", 0x0065_4960, This::Object, 0, UPDATE_VIDEO_STUBS, |m, this, _| { gh17::scene_update_texture_video(m, this); Out::None }) },
    framed(Case { prep: Some(prep_viewport), ..method("post_set_non_16_by_9_viewport", 0x0069_8750, This::Stdcall, 2, &[(h17::SET_VIEWPORT, K::This(4)), (h17::IGGY_SET_VIEWPORT_RECT, K::This(6))], |m, _, a| { let slots = slots_at(m, a); let r = gh17::post_set_non_16_by_9_viewport(m, slots, a[0], a[1]); // with `keep` the original leaves the caller's eax
        if a[1] == 0 { u(r) } else { Out::None } }) }, 0, 8),
];

#[test]
fn batch17_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES17, n, &mut Rng(0x1717_b017));
    });
}

#[test]
fn batch17_graphic_initialize_matches_the_original() {
    let stubs: &'static [(u32, K)] = Box::leak(initialize_stubs().into_boxed_slice());
    let case = Case {
        prep: Some(prep_initialize),
        ..cdecl("graphic_initialize", 0x0064_19d0, 0, stubs, |m, _, _| {
            u(gh17::graphic_initialize(m))
        })
    };
    Twins::each(|t| {
        let n = if t.snapshot { 16 } else { 64 };
        run_cases(t, &[case], n, &mut Rng(0x1717_b117));
    });
}

/// The map's camera (or none) with its lighting, the level (0x12ea or
/// another), the scenario flag, the map name (none, tagged) holding the
/// balance-skipping text or not, and the flag.
fn prep_char_light(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0xfb00);
    *this = scene;
    let map = obj + 0x200;
    t.w32(scene + 0x4f8, if rng.chance(5) { 0 } else { map });
    for k in 0..12 {
        let f = rng.float();
        t.w32(map + 0x120 + 4 * k, f.to_bits());
    }
    t.w32(
        scene + 0xfaf4,
        [0x12ea, 0x12eb, 0x199][rng.below(3) as usize],
    );
    let flag = match rng.below(3) {
        0 => 0,
        1 => rng.next() & 0xffff_0000,
        _ => rng.next() | 1,
    };
    t.stub_ret(h17::GET_SCENARIO_FLAG, false, 0, 0, flag);
    let name = match rng.below(3) {
        0 => 0,
        1 => obj + 0x700,
        _ => obj + 0x701,
    };
    t.w32(scene + 0xfaf0, name);
    let found = if rng.chance(2) { 0 } else { obj + 0x704 };
    t.stub_import_c(h17::STRSTR_SLOT, 2, found);
    args[0] = if rng.chance(2) {
        rng.next() & !0xff
    } else {
        rng.next() | 1
    };
}

/// The listed ids (some free, `id` among them or not), the count against
/// the capacity, the magic file name (none, tagged), the language and
/// the special-magic answer, async loading available or not.
fn prep_magic_texture(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let id = rng.next() | 1;
    args[0] = id;
    let n = rng.below(5);
    t.w32(scene + 0x28, n);
    t.w32(scene + 0x2c, n + rng.below(2));
    t.w32(
        scene + 0x30,
        if n == 0 && rng.chance(3) {
            0
        } else {
            obj + 0x100
        },
    );
    t.write(scene + 0x24, &[rng.below(3) as u8]);
    for k in 0..n {
        let v = match rng.below(4) {
            0 => 0,
            1 if rng.chance(2) => id,
            _ => id ^ 2,
        };
        t.w32(obj + 0x100 + 4 * k, v);
    }
    let file = match rng.below(4) {
        0 => 0,
        1 => 1,
        2 => obj + 0x601,
        _ => obj + 0x600,
    };
    t.w32(scene + 0xfafc, file);
    if !t.snapshot {
        let language = [0, 9, 10, 1, 3][rng.below(5) as usize];
        t.w32(obj + 4, language);
    }
    let special = if rng.chance(2) { 0x100 } else { 0x101 };
    t.stub_ret(h17::IS_SPECIAL_MAGIC, false, 1, 1, special);
    t.stub_ret(h17::MAGIC_ASYNC_AVAILABLE, false, 0, 0, rng.below(2));
}

const CHAR_LIGHT_STUBS: &[(u32, K)] = &[
    (h17::LIGHT_INSTANCE, K::Getter),
    (h17::LIGHT_SET_INTENSITY, K::This(2)),
    (h17::LIGHT_SET_ROTATION, K::This(3)),
    (h17::LIGHT_TWEAK_INSTANCE, K::Getter),
    (h17::LIGHT_SET_COEFFICIENT, K::This(2)),
    (h17::LIGHT_SET_FLAG, K::This(1)),
    (h17::LIGHT_SET_MAP_NAME, K::This(1)),
    (h17::LIGHT_BALANCE_GLOW, K::This(0)),
];

const MAGIC_TEXTURE_STUBS: &[(u32, K)] = &[
    (h17::UNIFY_FILENAME, K::Cdecl(3)),
    (h17::ARRAY_RESERVE_667E00, K::This(1)),
    (h17::LOCALIZATION_INSTANCE, K::Getter),
    (h17::LANGUAGE_NAME, K::Cdecl(0)),
    (h17::SPRINTF_SLOT, K::ImportC(4)),
    (h17::LOAD_MAGIC_FILE_ASYNC, K::This(2)),
    (h17::LOAD_VFX_TEXTURE, K::This(3)),
];

#[rustfmt::skip]
const CASES17B: &[Case] = &[
    Case { prep: Some(prep_char_light), ..method("scene_setup_character_lighting", 0x0066_eb20, This::Object, 1, CHAR_LIGHT_STUBS, |m, this, a| { gh17::scene_setup_character_lighting(m, this, a[0]); Out::None }) },
    framed(with_cookie(Case { prep: Some(prep_magic_texture), ..method("scene_load_magic_vfx_texture", 0x0066_4d80, This::Object, 1, MAGIC_TEXTURE_STUBS, |m, this, a| u(gh17::scene_load_magic_vfx_texture(m, frame_ebp(), this, a[0]))) }), 0x208, 0),
];

#[test]
fn batch17b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES17B, n, &mut Rng(0x1717_b217));
    });
}

// ---- round 5 batch 18 (ffx_prender::h3_glue18) ----

/// The post-process manager's targets and the renderer's calls failing
/// now and then.
fn prep_pre_render(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1100);
    *this = scene;
    let post = t.zeros(0x1_2400);
    t.w32(scene + 0x10cc, post);
    t.w32(scene + 0xd0, rng.next());
    for (target, _) in h18::PRE_RENDER_TARGETS {
        let v = rng.next();
        t.w32(post + target, v);
    }
    for sym in [h18::SET_COLOR_TARGET, h18::BEGIN_SCENE, h18::END_SCENE] {
        let n = if sym == h18::SET_COLOR_TARGET { 2 } else { 1 };
        let r = if rng.chance(5) { rng.next() | 1 } else { 0 };
        t.stub_ret(sym, true, n, n, r);
    }
}

/// The textures (made or not), their sizing failing now and then, the
/// targets (made or not).
fn prep_shadow_buffers(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.global(h18::SHADOW_SIZE.sym, 512 << rng.below(4));
    let texture = if rng.chance(8) { 0 } else { obj + 0x400 };
    t.stub_ret(h18::CLUSTER_CREATE, true, 3, 3, texture);
    let error = if rng.chance(6) { rng.next() | 1 } else { 0 };
    t.stub_ret(h18::SET_DIMENSIONS, true, 4, 4, error);
    let target = if rng.chance(8) { 0 } else { obj + 0x500 };
    t.stub_ret(h18::MAKE_TARGET, true, 1, 1, target);
    args[0] = rng.next();
    args[1] = rng.next();
}

#[rustfmt::skip]
const CASES18: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_pre_render), ..method("scene_pre_render", 0x0065_1be0, This::Object, 0, &[(h18::SURFACE_CTOR, K::This(2)), (h18::SURFACE_DTOR, K::This(0)), (h18::SHADOW_INSTANCE, K::Getter), (h18::SHADOW_PRE_RENDER, K::This(1))], |m, this, _| u(gh18::scene_pre_render(m, frame_ebp(), this))) }), 0x4c, 0),
    Case { prep: Some(prep_shadow_buffers), ..method("shadow_create_buffers", 0x006d_18b0, This::Object, 2, &[(h18::UPDATE_MAX_LEVEL, K::This(0))], |m, this, _| u(gh18::shadow_create_buffers(m, this))) },
];

#[test]
fn batch18_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES18, n, &mut Rng(0x1818_b018));
    });
}

// ---- round 6 batch 19 (ffx_prender::h3_glue19) ----

impl Twins {
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

/// The shape (6, 8 or another) and its index buffer.
fn prep_fill_indices(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let any = rng.next();
    let shape = pick(rng, &[6, 8, 6, 8, 7, 0, any]);
    t.w32(obj + 8, shape);
    t.w32(obj + 0x50, obj + 0x1000 + 4 * rng.below(8));
}

/// The parameter (or none; its kind, offset and size), the buffer and
/// its size, the install failing now and then.
fn prep_color_buffer(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let parameter = if rng.chance(6) { 0 } else { obj + 0x400 };
    t.w32(obj + 0xa8, parameter);
    let kind = pick(
        rng,
        &[
            0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x41, 0xdd, 0xd8, 0x13, 0,
        ],
    );
    t.write(obj + 0x402, &[kind as u8]);
    let offset = pick(rng, &[0, 2, 3, 4, 5, 8, 0x10, 0x40, 0x70, 0xf0, 0xfff0]);
    t.write(obj + 0x408, &(offset as u16).to_le_bytes());
    // sizes over 16 only where they don't fit (the copy would reach the
    // frame above the record); the high bits are masked
    let size = pick(rng, &[0, 1, 4, 12, 15, 16, 0x200, 0x1f00]) | (rng.below(8) << 13);
    t.write(obj + 0x40a, &(size as u16).to_le_bytes());
    t.w32(obj + 0x1c, obj + 0x500);
    t.w32(obj + 0x504, obj + 0x600);
    t.w32(obj + 0x600, pick(rng, &[0, 4, 0x14, 0x20, 0x80, 0x100]));
    let error = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h19::TROPHY_INSTALL, true, 1, 1, error);
    for a in args.iter_mut() {
        *a = if rng.chance(4) { 0 } else { rng.next() };
    }
}

/// The battle state, the map (one of the three or not), the scene's
/// flag and the manager.
fn prep_cascades(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let shadows = t.zeros(0x2c00);
    let b = rng.bytes(0x100);
    t.write(shadows + 0x2b00, &b);
    *this = shadows;
    t.global(h19::SHADOW_MANAGER, obj + 0x300);
    let encount = pick(rng, &[0, 1, 2, 0xffff_ffff, 0x8000_0000]);
    t.stub_ret(h19::ENCOUNT_TYPE, false, 0, 0, encount);
    let any = rng.next();
    args[0] = pick(rng, &[0, 1, 0x100, 0xffff_ff00, any]);
    let maps = h19::SHADOW_BATTLE_MAPS;
    args[3] = if rng.chance(2) {
        maps[rng.below(3) as usize] | ((rng.next() & 0xffff_0000) * u32::from(rng.chance(4)))
    } else {
        rng.next()
    };
    if !t.snapshot {
        let scene = t.zeros(0x1200);
        t.global(SCENE, scene);
        t.write(scene + 0x1196, &[u8::from(rng.chance(3))]);
    }
}

/// The allocations (the geometry block failing now and then).
fn prep_vfx_table(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let pool = t.zeros(0x1000);
    let b = rng.bytes(0x1000);
    t.write(pool, &b);
    let block = if rng.chance(3) { 0 } else { pool + 0x10 };
    t.stub_seq(
        h19::OPERATOR_NEW,
        false,
        1,
        0,
        &[block, pool + 0x20, pool + 0x30, pool + 0x40, pool + 0x50],
    );
    t.stub_ret(h19::VECTOR_CTOR, false, 5, 5, rng.next());
}

/// The meshes (some null; a negative count now and then), the draw events
/// and their mesh indices, the geometry of the held magic, the magic id,
/// an event failing.
fn prep_most_vfx(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let n = pick(rng, &[0, 1, 3, 5, 6, 0xffff_ffff]);
    t.w32(scene + 0xf8cc, n);
    let meshes: Vec<u32> = (0..10).map(|k| obj + 0x100 * k).collect();
    for (k, &mesh) in (0u32..).zip(&meshes) {
        let m = if rng.chance(4) { 0 } else { mesh };
        t.w32(scene + 0x38cc + 4 * k, m);
        let v = rng.next();
        t.w32(mesh + 0x10, v);
    }
    let events = rng.below(5);
    t.w32(scene + 0xfb54, events);
    let list = obj + 0x1800;
    t.w32(scene + 0xfb5c, list);
    let mut at = 0;
    for k in 0..events {
        at += rng.below(3);
        t.w32(list + 16 * k + 4, at.min(8));
    }
    let renderer = obj + 0x1a00;
    t.w32(scene + 0xd0, renderer);
    t.w32(renderer + 0x13c, rng.next());
    t.global(h19::VFX_RENDER_TABLE, obj + 0x1c00);
    let geometry = obj + 0x1d00;
    t.w32(geometry + 0xc, meshes[rng.below(10) as usize]);
    let g = if rng.chance(3) { 0 } else { geometry };
    t.stub_ret(h19::GEOMETRY_BY_INSTANCE, true, 2, 2, g);
    let magic = if rng.chance(3) {
        0x295
    } else {
        h19::LAST_DRAWN_MAGIC
    };
    t.stub_ret(h19::MAGIC_ID, true, 0, 0, magic);
    let error = if rng.chance(5) { rng.next() | 1 } else { 0 };
    t.stub_ret(h19::PROCESS_VFX_DRAW_EVENT, true, 1, 1, error);
}

/// [`prep_find_global`]'s map and objects on the scene's `+0x10d4`, and
/// the two cameras' vtables.
fn prep_find_camera(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_find_global(t, obj, this, args, rng);
    let scene = *this;
    let map = t.r32(scene + 0x1128);
    t.w32(scene + 0x10d4, map);
    let vtable = obj + 0x1e00;
    let update = t.stub(None, 0x6519, true, 0, 0, Some(rng.next()));
    t.w32(vtable + 4, update);
    t.w32(scene + 0x23c, vtable);
    t.w32(scene + 0x658, vtable);
}

/// The character (or none; known or not), its instances (null ones
/// among them; a byte count not a multiple of 4) and its vtable.
fn prep_unload(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let chr = obj + 0x100;
    args[0] = if rng.chance(6) { 0 } else { chr };
    let known = if rng.chance(5) {
        rng.next() & !0xff
    } else {
        1 | (rng.next() & !0xff)
    };
    t.stub_ret(h19::EXIST_PROTOTYPE, true, 1, 1, known);
    let list = obj + 0x1000;
    let n = rng.below(4);
    t.w32(chr + 8, list);
    let end = list + 4 * n + if rng.chance(4) { 3 } else { 0 };
    t.w32(chr + 0xc, end);
    for k in 0..n {
        let instance = obj + 0x400 + 0x100 * k;
        t.w32(list + 4 * k, if rng.chance(4) { 0 } else { instance });
        let count = rng.below(3);
        let w = count_word(rng, count);
        t.w32(instance, w);
        t.w32(instance + 4, obj + 0x300);
        let owned = rng.below(3);
        let w = count_word(rng, owned);
        t.w32(instance + 8, w);
        for off in [0xc, 0x10, 0x18] {
            let v = rng.next();
            t.w32(instance + off, v);
        }
    }
    let vtable = obj + 0x1e00;
    t.w32(chr, vtable);
    let destroy = t.stub(None, 0x7980, true, 1, 1, Some(rng.next()));
    t.w32(vtable, destroy);
}

const UNLOAD_STUBS: &[(u32, K)] = &[
    (h14::CHARACTER_UNREGIST_LOADING, K::Cdecl(1)),
    (h14::SCENE_REMOVE_MESH_INSTANCE, K::This(1)),
    (h14::CHARACTER_LIGHTING_INSTANCE, K::Getter),
    (h14::DESTROY_UNIQUE_MATERIAL_PARAMETERS, K::This(1)),
    (h14::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h14::UNREGISTER_CHARACTER_INSTANCE, K::This(1)),
    (h14::SHADOW_INSTANCE, K::Getter),
    (h14::SHADOW_REMOVE, K::This(1)),
    (h14::UNREGISTER_DELEGATION, K::This(3)),
    (h14::SCENE_REMOVE_CHR_FROM_PIPELINE, K::This(1)),
    (h14::DESTROY_CLOTH_INSTANCES, K::This(0)),
    (h14::CHARACTER_REMOVE_INSTANCE, K::This(1)),
    (h14::DESTROY_MESHES, K::This(0)),
    (h14::PHYRE_FREE, K::Cdecl(1)),
    (h14::OPERATOR_DELETE, K::Cdecl(1)),
    (h19::LOADED_CHARACTER_TABLE, K::Getter),
    (h19::PHYRE_PRINTF, K::Cdecl(2)),
    (h19::CLEAR_INSTANCE_LIST, K::This(0)),
    (h19::SCENE_UNLOAD_CHARACTER, K::This(1)),
    (h19::ERASE_VALUE, K::This(1)),
];

#[rustfmt::skip]
const CASES19: &[Case] = &[
    framed(Case { prep: Some(prep_unload), ..cdecl("character_unload_task", 0x0063_7980, 1, UNLOAD_STUBS, |m, _, a| gh19::character_unload_task(m, frame_ebp(), a[0]).map_or(Out::None, u)) }, 0xc, 0),
    Case { prep: Some(prep_fill_indices), ..method("broken_fill_indices", 0x006d_41b0, This::Object, 0, &[], |m, this, _| u(gh19::broken_fill_indices(m, this))) },
    framed(Case { prep: Some(prep_color_buffer), ..method("broken_set_color_buffer", 0x006d_4540, This::Object, 2, &[], |m, this, a| u(gh19::broken_set_color_buffer(m, frame_ebp(), this, a[0], a[1]))) }, 0x10, 0),
    framed(Case { prep: Some(prep_cascades), ..method("shadow_update_cascades", 0x006c_e860, This::Object, 4, &[(h19::SHADOW_BATTLE_PASS, K::This(2)), (h19::SHADOW_FIT, K::This(4)), (h19::SHADOW_CASCADE_PASS, K::This(2))], |m, this, a| { let slots = slots_at(m, a); u(gh19::shadow_update_cascades(m, slots, this, a[0], a[1], a[2], a[3])) }) }, 0, 16),
    Case { prep: Some(prep_vfx_table), ..method("vfx_table_ctor", 0x0069_e140, This::Object, 0, &[(h19::MAP_HEAD, K::Cdecl(0)), (h19::ALIGNED_MALLOC, K::Cdecl(2)), (h19::IOB_FUNC, K::ImportC(0)), (h19::FPRINTF, K::ImportC(6))], |m, this, _| u(gh19::vfx_table_ctor(m, this))) },
    Case { prep: Some(prep_most_vfx), ..method("scene_draw_most_vfx", 0x0065_0670, This::Object, 0, &[(h19::RENDER_MESH_INSTANCE, K::This(4))], |m, this, _| u(gh19::scene_draw_most_vfx(m, this))) },
    framed(Case { prep: Some(prep_find_camera), ..method("scene_find_global_camera", 0x0065_19f0, This::Object, 0, &[], |m, this, _| u(gh19::scene_find_global_camera(m, frame_ebp(), this))) }, 0x28, 0),
];

#[test]
fn batch19_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES19, n, &mut Rng(0x1919_b019));
    });
}

// ---- round 6 batch 20 (ffx_prender::h3_glue20) ----

impl Twins {
    /// [`Twins::stub_f64`] logging `n` cdecl arguments.
    fn stub_f64_n(&mut self, sym: u32, n: u32, value: f64) {
        let cell = self.code(&value.to_bits().to_le_bytes());
        let log = self.stub(None, sym, false, n, 0, Some(0));
        let mut c = vec![0xdd, 0x05]; // fld qword [cell]
        c.extend(cell.to_le_bytes());
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

/// The box (floats), the line counts (fewer, as many or more lines than
/// shown; past the end or not) and the remapped borders.
fn prep_scroll_parts(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    for a in args.iter_mut().take(4) {
        *a = if rng.chance(2) {
            (rng.below(1000) as f32 * 0.75).to_bits()
        } else {
            rng.float().to_bits()
        };
    }
    let total = pick(
        rng,
        &[0, 1, 5, 12, 100, 0x8000_0000, 0x7fff_ffff, 0x0100_0001],
    );
    let shown = pick(rng, &[0, 1, 4, 5, 12, 0xffff_ffff, 0x7fff_fff0]);
    let first = pick(
        rng,
        &[0, 1, 3, 7, 95, 0xffff_fffe, 0x7fff_ffff, 0x00ff_ffff],
    );
    args[4] = first;
    args[5] = shown;
    args[6] = total;
    for sym in [h20::UI_REMAP_X2, h20::UI_REMAP_Y2] {
        let v = match rng.below(4) {
            0 => 1.0,
            1 => 1.333_333_333_333_333_3,
            2 => f64::from(rng.float()),
            _ => f64::from(rng.next()) / 7.0,
        };
        t.stub_f64_n(sym, 1, v);
    }
}

/// The instance (or none; its meshes, flag), each mesh's draw elements
/// and materials (inline or not), the parameter lookups (found or not;
/// offsets and sizes in and out of the buffer), the alpha.
fn prep_mesh_alpha(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let chr = obj + 0x100;
    args[0] = if rng.chance(8) { 0 } else { chr };
    let n = pick(rng, &[0, 1, 2, 3]);
    let w = count_word(rng, n);
    t.w32(chr, w);
    t.w32(chr + 4, obj + 0x180);
    let flag = if rng.chance(3) {
        0
    } else {
        1 + rng.below(255) as u8
    };
    t.write(chr + 0x14, &[flag]);
    for i in 0..3 {
        let mesh = obj + 0x200 + 0x80 * i;
        t.w32(obj + 0x180 + 4 * i, mesh);
        let list = obj + 0x400 + 0x40 * i;
        t.w32(mesh, list);
        let count = pick(rng, &[0, 1, 2, 3]);
        let w = count_word(rng, count);
        t.w32(list, w);
        let elements = obj + 0x600 + 0x200 * i;
        t.w32(list + 4, elements);
        for e in 0..3 {
            t.w32(elements + 0x6c * e, rng.below(3));
        }
        let materials = mesh + 0x20;
        t.w32(mesh + 0x10, materials);
        let c = pick(rng, &[1, 2, 3, 0x8000_0001, 0x8000_0003]);
        t.w32(materials, c);
        let data = if c & 0x7fff_ffff > 1 {
            obj + 0xc80
        } else {
            materials + 4
        };
        if data != materials + 4 {
            t.w32(materials + 4, data);
        }
        for m in 0..3 {
            t.w32(data + 4 * m, obj + 0xc00 + 0x20 * ((m + i) % 3));
        }
    }
    for m in 0..3 {
        let material = obj + 0xc00 + 0x20 * m;
        let v = rng.next();
        t.w32(material, v);
        let buffer = obj + 0x1000 + 0x100 * m;
        t.w32(material + 4, buffer);
        t.w32(buffer, pick(rng, &[0, 0x10, 0x24, 0x40, 0x80]));
    }
    let mut params = vec![];
    for p in 0..8 {
        let at = obj + 0x1800 + 0x10 * p;
        let offset = pick(rng, &[0, 3, 4, 8, 0x20, 0x5c, 0xfff0]);
        let size = pick(rng, &[0, 4, 16, 0x14, 0x24, 0x400, 0x1010]) | (rng.below(8) << 13);
        t.write(at + 8, &(offset as u16).to_le_bytes());
        t.write(at + 0xa, &(size as u16).to_le_bytes());
        params.push(if rng.chance(4) { 0 } else { at });
    }
    t.stub_seq(h20::FIND_PARAMETER, true, 1, 1, &params);
    let alpha: f32 = match rng.below(8) {
        0 => 0.0,
        1 => -0.0,
        2 => 0.5,
        3 => 1.0,
        4 => 2.0,
        5 => -1.0,
        6 => f32::from_bits(0x7fc0_0000),
        _ => rng.float(),
    };
    args[1] = rng.next();
    args[2] = alpha.to_bits();
}

/// The map (or none), the tree's first chain (empty or 1..3 nodes), the
/// successors, each node's lists and material arrays, the parameter
/// lookups.
fn prep_map_yiq(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    t.w32(scene + 0x10d4, if rng.chance(6) { 0 } else { obj });
    let head = scene + 0x1198;
    let n = rng.below(4);
    let nodes: Vec<u32> = (0..n).map(|k| obj + 0x100 + 0x40 * k).collect();
    // the chain from +0xc: node k's first word the next, the last's the head
    t.w32(head + 0xc, nodes.first().copied().unwrap_or(head));
    for (k, &node) in nodes.iter().enumerate() {
        t.w32(node, nodes.get(k + 1).copied().unwrap_or(head));
    }
    let mut order = vec![];
    for _ in 0..rng.below(4) {
        order.push(obj + 0x100 + 0x40 * rng.below(3));
    }
    order.extend([head; 4]);
    t.stub_seq(h20::TREE_SUCCESSOR, true, 1, 1, &order);
    for k in 0..3 {
        let node = obj + 0x100 + 0x40 * k;
        let list = obj + 0x300 + 0x40 * k;
        t.w32(node + 0x10, list);
        t.w32(list, rng.below(3));
        t.w32(list + 8, list + 0x10);
        for i in 0..2 {
            let item = obj + 0x500 + 0x40 * (2 * k + i);
            t.w32(list + 0x10 + 4 * i, item);
            let array = obj + 0x800 + 0x10 * (2 * k + i);
            t.w32(item + 0x10, array);
            let c = pick(rng, &[0, 1, 2, 0x8000_0002]);
            t.w32(array, c);
        }
    }
    let data = obj + 0xa00;
    for m in 0..3 {
        let material = obj + 0xb00 + 0x20 * m;
        t.w32(data + 4 * m, material);
        let v = rng.next();
        t.w32(material, v);
        let buffer = obj + 0x1000 + 0x100 * m;
        t.w32(material + 4, buffer);
        t.w32(buffer, pick(rng, &[0, 0x10, 0x40, 0x80]));
    }
    t.stub_ret(h20::ARRAY_DATA, true, 0, 0, data);
    let mut params = vec![];
    for p in 0..24 {
        let at = obj + 0x1800 + 0x10 * p;
        let offset = pick(rng, &[0, 3, 4, 8, 0x20, 0x4c, 0xfff0]);
        let size = pick(rng, &[0, 4, 12, 16, 0x24, 0x2c, 0x400, 0x1010]) | (rng.below(8) << 13);
        t.write(at + 8, &(offset as u16).to_le_bytes());
        t.write(at + 0xa, &(size as u16).to_le_bytes());
        params.push(if rng.chance(4) { 0 } else { at });
    }
    t.stub_seq(h20::FIND_PARAMETER, true, 1, 1, &params);
}

#[rustfmt::skip]
const CASES20: &[Case] = &[
    framed(Case { prep: Some(prep_scroll_parts), ..cdecl("draw_cross_menu_scroll_parts", 0x008e_6cc0, 7, &[(h20::DRAW_CROSS_BOX, K::Cdecl(6))], |m, _, a| { let slots = slots_at(m, a); gh20::draw_cross_menu_scroll_parts(m, slots, a[0], a[1], a[2], a[3], a[4] as i32, a[5] as i32, a[6] as i32); Out::None }) }, 0, 0x1c),
    framed(Case { prep: Some(prep_mesh_alpha), ..cdecl("character_set_mesh_alpha", 0x0063_b810, 3, &[(h20::SET_MESH_VISIBLE, K::This(2)), (h20::FORCE_TRANSPARENT, K::Cdecl(2))], |m, _, a| { gh20::character_set_mesh_alpha(m, frame_ebp(), a[0], a[2]); Out::None }) }, 0x24, 0),
    framed(Case { prep: Some(prep_map_yiq), ..method("scene_update_map_yiq", 0x0067_1da0, This::Object, 0, &[(h20::GET_YIQ_PARAM, K::This(6))], |m, this, _| { gh20::scene_update_map_yiq(m, frame_ebp(), this); Out::None }) }, 0x3c, 0),
];

#[test]
fn batch20_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES20, n, &mut Rng(0x2020_b020));
    });
}

// ---- round 6 batch 21 (ffx_prender::h3_glue21) ----

/// A renderer error now and then.
fn render_error(rng: &mut Rng) -> u32 {
    if rng.chance(6) { rng.next() | 1 } else { 0 }
}

/// The full-screen mesh and its first material (inline or not), the
/// parameter (or none), the binder and the renderer failing now and then,
/// the target (or the back buffer) and its texture.
fn prep_full_screen(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    let mesh = obj + 0x100;
    t.w32(post + 0x1208, mesh);
    let materials = obj + 0x200;
    t.w32(mesh + 0x10, materials);
    let count = pick(rng, &[1, 2, 0x8000_0001, 0x8000_0002]);
    t.w32(materials, count);
    let material = obj + 0x300;
    if count & 0x7fff_ffff > 1 {
        t.w32(materials + 4, obj + 0x280);
        t.w32(obj + 0x280, material);
    } else {
        t.w32(materials + 4, material);
    }
    let shader = rng.next();
    t.w32(material, shader);
    let buffer = rng.next();
    t.w32(material + 4, buffer);
    let parameter = if rng.chance(4) { 0 } else { obj + 0x380 };
    t.stub_ret(h21::FIND_PARAMETER, true, 1, 1, parameter);
    let error = render_error(rng);
    t.stub_ret(h21::BIND_TEXTURE, true, 2, 2, error);
    for (sym, n) in [
        (h21::SET_COLOR_TARGET, 2),
        (h21::SET_DEPTH_STENCIL_TARGET, 1),
        (h21::BEGIN_SCENE, 1),
    ] {
        let error = render_error(rng);
        t.stub_ret(sym, true, n, n, error);
    }
    let renderer = obj + 0x800;
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    let target = obj + 0x900;
    let texture = obj + 0xa00;
    t.w32(target, texture);
    t.write(target + 4, &[0]);
    let w = rng.next();
    t.w32(texture + 0x1c, w);
    let ht = rng.next();
    t.w32(texture + 0x20, ht);
    args[0] = renderer;
    args[1] = rng.next();
    args[2] = if rng.chance(2) { 0 } else { target };
}

/// The alpha (zero, NaN or not), the fade mesh (or none) and its colour
/// parameter (or none; in or out of its buffer), the colour, the blend
/// mode, the target or the surface, the renderer failing now and then.
fn prep_screen_fade(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    let alpha = pick(
        rng,
        &[
            0,
            0x8000_0000,
            0x3f00_0000,
            0x3f80_0000,
            0x7fc0_0000,
            0xffc0_0001,
            0x7f80_0000,
        ],
    );
    t.w32(post + 0x1_2398, alpha);
    for k in 0..3 {
        let v = rng.float().to_bits();
        t.w32(post + 0x1_238c + 4 * k, v);
    }
    let v = rng.next();
    t.w32(post + 0x8c, v);
    let mesh = obj + 0x100;
    t.w32(post + 0x1_2388, if rng.chance(6) { 0 } else { mesh });
    let shader = rng.next();
    t.w32(mesh, shader);
    let buffer = obj + 0x400;
    t.w32(mesh + 4, buffer);
    t.w32(buffer, pick(rng, &[0, 0x10, 0x14, 0x20, 0x80]));
    let parameter = obj + 0x380;
    let offset = pick(rng, &[0, 3, 4, 8, 0x10, 0x70, 0xfff0]);
    let size = pick(rng, &[0, 4, 12, 16, 0x1010]) | (rng.below(8) << 13);
    t.write(parameter + 8, &(offset as u16).to_le_bytes());
    t.write(parameter + 0xa, &(size as u16).to_le_bytes());
    t.stub_ret(
        h21::FIND_PARAMETER,
        true,
        1,
        1,
        if rng.chance(6) { 0 } else { parameter },
    );
    t.w32(post + 0x1_239c, pick(rng, &[0, 2, 0x10, 1, 0x12]));
    let mesh2 = obj + 0x600;
    t.w32(post + 0x1_2384, mesh2);
    let v = rng.next();
    t.w32(mesh2 + 0x10, v);
    for (sym, n) in [(h21::SET_COLOR_TARGET, 2), (h21::BEGIN_SCENE, 1)] {
        let e = render_error(rng);
        t.stub_ret(sym, true, n, n, e);
    }
    let renderer = obj + 0x800;
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    args[0] = renderer;
    args[1] = rng.next();
    args[2] = if rng.chance(2) { 0 } else { rng.next() };
    args[3] = if rng.chance(2) {
        rng.next() & !0xff
    } else {
        rng.next()
    };
}

/// The flags (UI, movie fade, pause), the UI and its viewer, the menus
/// (none, no mesh, kinds 0, 2 and another), the fades, the target.
fn prep_normal_ui(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    t.write(scene + 0x1_0dcc, &[u8::from(!rng.chance(4))]);
    t.global(
        h21::MOVIE_FADING.sym,
        if rng.chance(2) { 0 } else { rng.next() },
    );
    t.global(h21::PAUSE_ON.sym, u32::from(!rng.chance(2)));
    let ui = obj + 0x100;
    t.global(h21::GRAPHIC_UI.sym, if rng.chance(6) { 0 } else { ui });
    t.write(obj + 0x50, &[u8::from(!rng.chance(5))]);
    let renderer = obj + 0x800;
    t.w32(scene + 0xd0, renderer);
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    let n = rng.below(5);
    // a negative count draws nothing
    let reported = if rng.chance(8) { 0x8000_0000 | n } else { n };
    let counts: Vec<u32> = (0..8).map(|_| reported).collect();
    t.stub_seq(h21::MENU_COUNT, true, 0, 0, &counts);
    let mut menus = vec![];
    for k in 0..n {
        let menu = obj + 0x200 + 0x40 * k;
        let mesh = if rng.chance(5) {
            0
        } else {
            obj + 0x400 + 0x40 * k
        };
        t.w32(menu + 0xc, mesh);
        let info = obj + 0x600 + 0x100 * k;
        t.w32(menu + 0x24, info);
        let kind = pick(rng, &[0, 0, 2, 2, 1, 0x102, 0x200]);
        t.write(info + 0xb4, &(kind as u16).to_le_bytes());
        if mesh != 0 {
            let v = rng.next();
            t.w32(mesh + 0x10, v);
        }
        menus.push(if rng.chance(6) { 0 } else { menu });
    }
    t.stub_seq(h21::MENU_AT, true, 1, 1, &menus);
    let fades: Vec<u32> = (0..8)
        .map(|_| {
            if rng.chance(2) {
                rng.next() & !0xff
            } else {
                rng.next() | 1
            }
        })
        .collect();
    t.stub_seq(h21::IS_FADING, true, 0, 0, &fades);
    let post = obj + 0xc00;
    t.w32(scene + 0x10cc, post);
    let target = obj + 0xd00;
    t.w32(post + 0x8c, target);
    let texture = obj + 0xe00;
    t.w32(target, texture);
    t.write(target + 4, &[0]);
    let w = rng.next();
    t.w32(texture + 0x1c, w);
    let ht = rng.next();
    t.w32(texture + 0x20, ht);
}

impl Twins {
    /// A recording stub at `sym` (thiscall, `n` words) that first stores
    /// three words through each of its first two arguments.
    fn stub_two_vec3(&mut self, sym: u32, n: u32, ret: u32, first: [u32; 3], second: [u32; 3]) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        let mut code = vec![];
        for (arg, vals) in [(4u8, first), (8u8, second)] {
            code.extend([0x8b, 0x44, 0x24, arg]); // mov eax, [esp+arg]
            for (k, v) in (0u8..).zip(vals) {
                code.extend([0xc7, 0x40, 4 * k]); // mov dword [eax+4k], v
                code.extend(v.to_le_bytes());
            }
        }
        let at = self.cave_next;
        code.push(0xe9); // jmp log
        code.extend(log.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
        let cave = self.code(&code);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(cave.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// The casters (none, one or several), the camera, the bounds the
/// forwarder writes, the min and max results, the outputs (apart, or the
/// same word).
fn prep_extent(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let list = obj + 0x100;
    args[0] = if rng.chance(8) { 0 } else { list };
    let n = pick(rng, &[1, 2, 3, 0x8000_0002]);
    t.w32(list, n);
    t.w32(list + 4, obj + 0x140);
    for k in 0..3 {
        let node = obj + 0x200 + 0x40 * k;
        t.w32(obj + 0x140 + 4 * k, node);
        let m = obj + 0x400 + 0x40 * k;
        t.w32(node + 0x1c, m);
        let v = rng.next();
        t.w32(m + 0xc, v);
    }
    let camera = obj + 0x600;
    let b = rng.bytes(0x40);
    t.write(camera, &b);
    let product = rng.next();
    t.stub_ret(h21::MATRIX_MUL_THIS, true, 2, 2, product);
    let vec3 = |rng: &mut Rng| {
        [
            rng.float().to_bits(),
            rng.float().to_bits(),
            rng.float().to_bits(),
        ]
    };
    let (lo, hi) = (vec3(rng), vec3(rng));
    t.stub_two_vec3(h21::BOUNDS_FORWARD, 3, rng.next(), lo, hi);
    for (sym, at) in [(h21::VEC3_MIN, obj + 0x900), (h21::VEC3_MAX, obj + 0x980)] {
        for k in 0..3 {
            let v = rng.float().to_bits();
            t.w32(at + 4 * k, v);
        }
        t.stub_ret(sym, false, 3, 0, at);
    }
    let out = obj + 0xa00;
    if rng.chance(4) {
        args[1] = out;
        args[2] = out;
        args[3] = out;
    } else {
        args[1] = out;
        args[2] = out + 4;
        args[3] = out + 8;
    }
    args[4] = camera;
}

impl Twins {
    /// [`Twins::stub_cursor_script`] whose first stub takes `first_args`
    /// words (a constructor with the world).
    fn stub_cursor_script_args(
        &mut self,
        first: u32,
        first_args: u32,
        next: u32,
        steps: &[[u32; 5]],
    ) {
        let mut words = vec![];
        for s in steps {
            for w in s {
                words.extend(w.to_le_bytes());
            }
        }
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        for (sym, n) in [(first, first_args), (next, 0)] {
            let log = self.stub(None, sym, true, n, n, Some(sym));
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
}

/// An array header (count, capacity, data): empty, or part full with room
/// or without (the reserve stub keeps the data; a new array's null data
/// takes the first append's null slot).
fn array_header(t: &mut Twins, at: u32, data: u32, rng: &mut Rng) {
    let (count, cap, d) = match rng.below(4) {
        0 => (0, 0, data),
        1 => (2, 2, data),
        2 => (1, 8, data),
        _ => (0, 0xffff_ffff, data),
    };
    t.w32(at, count);
    t.w32(at + 4, cap);
    t.w32(at + 8, d);
}

/// The flags (rebuild, field map, map), the map's objects through cursor
/// runs (keys, kinds), the tree's arrays (found, a node at -0x10, or
/// new), the second array.
fn prep_map_objects(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    t.write(
        scene + 0x1194,
        &[u8::from(!rng.chance(5)), rng.next() as u8],
    );
    t.write(scene + 0x10d1, &[u8::from(!rng.chance(5))]);
    t.w32(scene + 0x10d4, if rng.chance(6) { 0 } else { obj });
    let area = t.zeros(0x800);
    for k in 0..8 {
        let object = area + 0x100 * k + 4;
        t.w32(object + 0x58, rng.below(4));
        t.w32(object + 0x4c, pick(rng, &[1, 1, 0, 2, 0x101]));
    }
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script_args(h21::CURSOR_CTOR, 1, h21::CURSOR_NEXT, &steps);
    let data = t.zeros(0x400);
    let mut found = vec![];
    for k in 0..40 {
        let node = obj + 0x1000 + 0x20 * (k % 8);
        let array = obj + 0x1200 + 0x10 * (k % 8);
        t.w32(node + 0x10, array);
        array_header(t, array, data + 0x40 * (k % 8), rng);
        found.push(match rng.below(4) {
            0 => 0,
            1 => 0xffff_fff0,
            _ => node,
        });
    }
    t.stub_seq(h21::TREE_FIND, true, 1, 1, &found);
    let mut fresh = vec![];
    for k in 0..40 {
        // a failed allocation faults in the original's append
        fresh.push(obj + 0x1400 + 0x10 * (k % 16));
    }
    t.stub_seq(h21::OPERATOR_NEW, false, 1, 0, &fresh);
    array_header(t, scene + 0x11c8, data + 0x200, rng);
}

const FULL_SCREEN_STUBS: &[(u32, K)] = &[
    (h21::SURFACE_CTOR, K::This(2)),
    (h21::SURFACE_DTOR, K::This(0)),
    (h21::SET_VIEWPORT, K::This(4)),
    (h21::SET_NON_16_BY_9_VIEWPORT, K::This(2)),
    (h21::RENDER_MESH_INSTANCE, K::This(4)),
    (h21::END_SCENE, K::This(1)),
    (h21::GET_APPLICATION, K::Getter),
];

const SCREEN_FADE_STUBS: &[(u32, K)] = &[
    (h21::SURFACE_CTOR, K::This(2)),
    (h21::SURFACE_DTOR, K::This(0)),
    (h21::RENDER_MESH_INSTANCE, K::This(4)),
    (h21::END_SCENE, K::This(1)),
];

const NORMAL_UI_STUBS: &[(u32, K)] = &[
    (h21::CLEAR_SCENE, K::This(1)),
    (h21::SET_CAMERA, K::This(1)),
    (h21::UPDATE_VIEW_MATRICES, K::This(0)),
    (h21::VIEWER_MANAGER, K::Getter),
    (h21::MINIMAP_SIZE, K::This(0)),
    (h21::MINIMAP_Y, K::This(0)),
    (h21::MINIMAP_65E4A0, K::This(0)),
    (h21::MINIMAP_X, K::This(0)),
    (h21::SET_SCISSOR, K::This(4)),
    (h21::RENDER_MESH, K::This(1)),
    (h21::RENDER_MESH_INSTANCE, K::This(4)),
];

#[rustfmt::skip]
const CASES21: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_full_screen), ..method("post_render_full_screen_texture", 0x0069_59d0, This::Object, 3, FULL_SCREEN_STUBS, |m, this, a| u(gh21::post_render_full_screen_texture(m, frame_ebp(), this, a[0], a[1], a[2]))) }), 0x30, 0),
    framed(with_seh(Case { prep: Some(prep_screen_fade), ..method("post_render_screen_fade", 0x0069_7490, This::Object, 4, SCREEN_FADE_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh21::post_render_screen_fade(m, frame_ebp(), slots, this, a[0], a[2], a[3])) }) }), 0x30, 0x10),
    framed(Case { prep: Some(prep_casters), ..method("shadow_update_caster_visibility", 0x006c_e6f0, This::Object, 1, &[(h21::MATRIX_FROM_3X4, K::This(1))], |m, this, a| { let slots = slots_at(m, a); u(gh21::shadow_update_caster_visibility(m, frame_ebp(), slots, this, a[0])) }) }, 0xdc, 4),
    framed(Case { prep: Some(prep_extent), ..cdecl("shadow_casters_extent", 0x006d_2530, 5, &[(h21::MATRIX_FROM_3X4, K::This(1))], |m, _, a| { let slots = slots_at(m, a); gh21::shadow_casters_extent(m, frame_ebp(), slots, a[0], [a[1], a[2], a[3]], a[4]); Out::None }) }, 0x128, 0x14),
    framed(Case { prep: Some(prep_map_objects), ..method("scene_rebuild_map_objects", 0x0065_8ae0, This::Object, 0, &[(h21::DETACH_CHR_CLUSTER, K::This(0)), (h21::TREE_INSERT, K::This(2)), (h21::ARRAY_RESERVE, K::This(1)), (h21::FIELD_MAP_INITIALIZE, K::This(1))], |m, this, _| u(gh21::scene_rebuild_map_objects(m, frame_ebp(), this))) }, 0x38, 0),
    Case { prep: Some(prep_normal_ui), ..method("scene_draw_normal_ui", 0x0065_0cb0, This::Object, 0, NORMAL_UI_STUBS, |m, this, _| u(gh21::scene_draw_normal_ui(m, this))) },
];

#[test]
fn batch21_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES21, n, &mut Rng(0x2121_b021));
    });
}

/// The shadows on or off, the 64 casters' lists (none, empty or 1..3
/// nodes with or without a matrix), kinds ('c' or not), the previous
/// flags, the caster tests.
fn prep_casters(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let shadows = t.zeros(0x4100);
    *this = shadows;
    t.write(shadows + 0x4044, &[u8::from(!rng.chance(6))]);
    for i in 0..64 {
        let list = if rng.chance(4) {
            0
        } else {
            obj + 0x100 + 0x10 * (i % 8)
        };
        t.w32(shadows + 0xde8 + 4 * i, list);
        t.w32(shadows + 0xfe8 + 4 * i, pick(rng, &[1, 1, 0, 2]));
        let v = rng.next();
        t.w32(shadows + 0x10e8 + 4 * i, v);
        t.write(
            shadows + 0x2ba8 + 0xc * i,
            &[if rng.chance(4) { b'd' } else { b'c' }],
        );
    }
    for l in 0..8 {
        let list = obj + 0x100 + 0x10 * l;
        let n = pick(rng, &[0, 1, 2, 3, 0x8000_0002]);
        t.w32(list, n);
        t.w32(list + 4, obj + 0x200 + 0x10 * l);
        for j in 0..3 {
            let holder = obj + 0x300 + 0x40 * ((l + j) % 8);
            t.w32(obj + 0x200 + 0x10 * l + 4 * j, holder);
            let node = obj + 0x600 + 0x20 * ((l + 2 * j) % 8);
            t.w32(holder + 0x1c, node);
            t.w32(node + 0xc, if rng.chance(2) { 0 } else { obj + 0x800 });
        }
    }
    let view = obj + 0x1800;
    let b = rng.bytes(0x40);
    t.write(view + 0x70, &b);
    args[0] = view;
    let product = rng.next();
    t.stub_ret(h21::MATRIX_MULTIPLY, false, 3, 0, product);
    let tests: Vec<u32> = (0..200)
        .map(|_| {
            if rng.chance(3) {
                rng.next() | 1
            } else {
                rng.next() & !0xff
            }
        })
        .collect();
    t.stub_seq(h21::CASTER_VISIBLE, true, 1, 1, &tests);
}

// ---- round 6 batch 22 (ffx_prender::h3_glue22) ----

/// A map path with 0..5 slashes and segments of 0..20 letters, the preset
/// lookup (found, at -0x10, or not) and its floats.
fn prep_hdao(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    t.write(post + 0x1_23d1, &[rng.next() as u8 | 1]);
    let mut path = vec![];
    for _ in 0..rng.below(6) {
        for _ in 0..rng.below(4) {
            path.push(b'a' + rng.below(26) as u8);
        }
        path.push(b'/');
    }
    for _ in 0..rng.below(20) {
        path.push(b'a' + rng.below(26) as u8);
    }
    if rng.chance(2) {
        path.push(b'/');
    }
    path.push(0);
    let at = obj + 0x100;
    t.write(at, &path);
    args[0] = at;
    let node = obj + 0x400;
    let preset = obj + 0x500;
    t.w32(node + 0x10, preset);
    for k in 0..13 {
        let v = rng.float().to_bits();
        t.w32(preset + 4 * k, v);
    }
    let found = pick(rng, &[node, node, 0, 0xffff_fff0]);
    t.stub_ret(h22::MAP_FIND, true, 1, 1, found);
}

#[rustfmt::skip]
const CASES22: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_hdao), ..method("post_set_hdao_setting", 0x0069_85d0, This::Object, 1, &[], |m, this, a| { gh22::post_set_hdao_setting(m, frame_ebp(), |len| gh22::hdao_buffer(frame_ebp(), len), this, a[0]); Out::None }) }), 0x10, 0),
];

#[test]
fn batch22_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES22, n, &mut Rng(0x2222_b022));
    });
}

// ---- round 6 batch 23 (ffx_prender::h3_glue23) ----

/// The loads failing now and then, the cluster (or none, its list empty
/// or not) and what the seek finds (nothing, an object at 0, an object).
fn prep_shadow_bins(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let error = if rng.chance(5) { rng.next() | 1 } else { 0 };
    t.stub_ret(h23::LOAD_ASSET_FILE, false, 3, 0, error);
    let error = if rng.chance(5) { rng.next() | 1 } else { 0 };
    t.stub_ret(h23::FIXUP_CLUSTERS, false, 2, 0, error);
    let cluster = obj + 0x400;
    t.w32(obj, if rng.chance(5) { 0 } else { cluster });
    t.w32(
        cluster + 0x1c,
        if rng.chance(2) {
            cluster + 0x1c
        } else {
            obj + 0x500
        },
    );
    let base = rng.next();
    let (count, offset) = match rng.below(3) {
        0 => (0, 4),
        1 => (1, base.wrapping_neg()),
        _ => (pick(rng, &[1, 2]), 4),
    };
    let steps = [[base, rng.next(), 0x40, count, offset], [KEEP; 5]];
    t.stub_cursor_script(h23::CURSOR_SEEK3, h14::CURSOR_NEXT1, &steps);
}

/// The tables on or off, the name (or none), the copy (or none, or a
/// borrowed text), the lookups (found, at -0x10, or not), the flags word.
fn prep_find_shader(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    t.write(obj + 0x60, &[u8::from(rng.chance(2))]);
    t.write(obj + 0x98, &[u8::from(rng.chance(2))]);
    let name = obj + 0x200;
    let len = rng.below(12) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
    text.push(0);
    t.write(name, &text);
    args[0] = pick(rng, &[name, name, name, 0, name + 1]);
    let flags = obj + 0x300;
    let v = rng.next();
    t.w32(flags, v);
    args[1] = flags;
    let copy = pick(rng, &[obj + 0x600, obj + 0x601, 0]);
    t.stub_ret(h23::PHYRE_MALLOC, false, 2, 0, copy);
    let key = rng.next();
    t.stub_ret(h23::PSTRING_CTOR, true, 1, 1, key);
    let node = obj + 0x700;
    let shader = obj + 0x800;
    t.w32(node + 0x10, shader);
    let v = rng.next();
    t.w32(shader, v);
    let finds: Vec<u32> = (0..4).map(|_| pick(rng, &[0, node, 0xffff_fff0])).collect();
    t.stub_seq(h23::PSTRING_TREE_FIND, true, 1, 1, &finds);
    let number = pick(rng, &[0, node, 0xffff_fff0]);
    t.stub_ret(h23::UINT_TREE_FIND, true, 1, 1, number);
}

#[rustfmt::skip]
const CASES23: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_shadow_bins), ..method("shadow_load_shader_bins", 0x006d_2c10, This::Object, 0, &[(h23::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (h23::GET_APPLICATION, K::Getter)], |m, this, _| u(gh23::shadow_load_shader_bins(m, frame_ebp(), this))) }), 0x124, 0),
    framed(Case { prep: Some(prep_find_shader), ..method("vfx_find_shader", 0x006c_06e0, This::Object, 2, &[(h23::PHYRE_FREE, K::Cdecl(1))], |m, this, a| { let slots = slots_at(m, a); u(gh23::vfx_find_shader(m, frame_ebp(), slots, this, a[0], a[1])) }) }, 4, 8),
];

#[test]
fn batch23_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES23, n, &mut Rng(0x2323_b023));
    });
}

// ---- round 6 batch 24 (ffx_prender::h3_glue24) ----

impl Twins {
    /// A recording stub at `sym` (thiscall, `n` words) that first stores
    /// each (offset, value) at `ecx + offset`.
    fn stub_set_this(&mut self, sym: u32, n: u32, ret: u32, stores: &[(u8, u32)]) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        let mut code = vec![];
        for &(off, value) in stores {
            code.extend([0xc7, 0x41, off]); // mov dword [ecx+off], value
            code.extend(value.to_le_bytes());
        }
        let at = self.cave_next;
        code.push(0xe9); // jmp log
        code.extend(log.wrapping_sub(at + code.len() as u32 + 4).to_le_bytes());
        let cave = self.code(&code);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(cave.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// The map name (none, empty or a word), the strings left inline or on
/// the heap by the appends, the two lookups (found or not) and their
/// lists.
fn prep_color_correction(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let name = obj + 0x200;
    let len = pick(rng, &[0, 1, 4, 9]) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
    text.push(0);
    t.write(name, &text);
    t.w32(obj + 0xd8, if rng.chance(5) { 0 } else { name });
    let heap = obj + 0x300;
    let cap = pick(rng, &[0x10, 0x1f]);
    let assign = if rng.chance(3) {
        vec![(0x14, cap), (0, heap)]
    } else {
        vec![]
    };
    let r = rng.next();
    t.stub_set_this(h24::STRING_ASSIGN, 2, r, &assign);
    let cap = pick(rng, &[0x10, 0x2f]);
    let append = if rng.chance(2) {
        vec![(0x14, cap), (0, heap + 0x40)]
    } else {
        vec![]
    };
    let r = rng.next();
    t.stub_set_this(h24::STRING_APPEND, 2, r, &append);
    let ids: Vec<u32> = (0..2)
        .map(|_| pick(rng, &[0xffff_ffff, 0, 2, 0xffff_fffe]))
        .collect();
    t.stub_seq(h24::GET_SYMBOL_ID, true, 1, 1, &ids);
    for (lists, at) in [(0x10e0, 0x400), (0x1134, 0x500)] {
        t.w32(obj + lists, obj + at + 8);
        for k in 0..4 {
            let v = rng.next();
            t.w32(obj + at + 4 * k, v);
        }
    }
}

#[rustfmt::skip]
const CASES24: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_color_correction), ..method("scene_setup_color_correction", 0x0066_ed00, This::Object, 0, &[(h24::PHYRE_PRINTF, K::Cdecl(2)), (h24::SET_FIRST_ENTRY, K::This(1)), (h24::OPERATOR_DELETE, K::Cdecl(1))], |m, this, _| u(gh24::scene_setup_color_correction(m, frame_ebp(), this))) }), 0x40, 0),
];

#[test]
fn batch24_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES24, n, &mut Rng(0x2424_b024));
    });
}

/// An asset path (`+0x17`) of 1..4 segments before `"/bin"` and some after,
/// `keep2d` set or not (only its low byte counts), the output.
fn prep_map_path(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let mut path = vec![];
    // a '/' before "/bin" (without one the original reads address 0)
    for _ in 0..=rng.below(4) {
        path.extend(b"/");
        for _ in 0..=rng.below(8) {
            path.push(b'a' + rng.below(26) as u8);
        }
    }
    path.extend(b"/bin");
    if rng.chance(2) {
        path.extend(b"/x.phyre");
    }
    path.push(0);
    t.write(obj + 0x17, &path);
    args[0] = obj;
    args[1] = obj + 0x400;
    args[2] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
}

#[rustfmt::skip]
const CASES24B: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_map_path), ..method("scene_convert_map_path", 0x0065_9290, This::Stdcall, 3, &[(h24::FIOS_UNIFY_FILENAME, K::Cdecl(3)), (h24::SPRINTF, K::ImportC(3))], |m, _, a| u(gh24::scene_convert_map_path(m, frame_ebp(), a[0], a[1], a[2]))) }), 0x18c, 0),
];

#[test]
fn batch24b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES24B, n, &mut Rng(0x2424_b124));
    });
}

/// The kind, the allocations (a block failing now and then; a failed
/// list faults in the original's push), the list's state (current block
/// unset or not, its blocks).
fn prep_create_pool(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    args[0] = rng.below(4);
    let list = obj + 0x100;
    let blocks = [obj + 0x200, obj + 0x240];
    let news = [
        list,
        if rng.chance(5) { 0 } else { blocks[0] },
        if rng.chance(5) { 0 } else { blocks[1] },
    ];
    t.stub_seq(h24::OPERATOR_NEW, false, 1, 0, &news);
    // the constructor gives the list (the original's `this`)
    t.stub_ret(h24::POOL_LIST_CTOR, true, 0, 0, list);
    t.w32(list + 0xc, pick(rng, &[0, 1, 0x8000_0002]));
    t.w32(list + 4, pick(rng, &[0xffff_ffff, 0, 3]));
    t.w32(list + 0x10, obj + 0x300);
    t.w32(obj + 0x300, obj + 0x340);
    let v = rng.next();
    t.w32(obj + 0x344, v);
    let v = rng.next();
    t.w32(list + 8, v);
}

#[rustfmt::skip]
const CASES24C: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_create_pool), ..method("dyn_geo_create_pool", 0x006d_e720, This::Object, 1, &[(h24::POOL_RELEASE, K::This(1)), (h24::CREATE_MUTEX, K::Import(3)), (h24::OPERATOR_NEW_6304C0, K::Cdecl(1)), (h24::PHYRE_MALLOC, K::Cdecl(2)), (h24::ARRAY_PUSH_INTO, K::This(2))], |m, this, a| { let slots = slots_at(m, a); gh24::dyn_geo_create_pool(m, frame_ebp(), slots, this, a[0]); Out::None }) }), 0x14, 4),
];

#[test]
fn batch24c_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES24C, n, &mut Rng(0x2424_b224));
    });
}

/// The list (0..3 entries, the count's top bit now and then), each
/// entry's mesh and floats, the world matrices the conversion overwrites,
/// `restore` (only its low byte counts).
fn prep_mesh_matrices(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0e00);
    *this = scene;
    let n = rng.below(4);
    let count = count_word(rng, n);
    t.w32(scene + 0x1_0d84, count);
    let list = obj + 0x100;
    t.w32(scene + 0x1_0d88, list);
    for k in 0..n {
        let entry = list + 0x44 * k;
        let mesh = obj + 0x400 + 0x40 * k;
        t.w32(entry, mesh);
        for c in 0..16 {
            let word = rng.float().to_bits();
            t.w32(entry + 4 + 4 * c, word);
        }
        let world = obj + 0x600 + 0x40 * k;
        t.w32(mesh + 4, world);
        let v = rng.next();
        t.w32(mesh + 0x10, v);
        for c in 0..12 {
            let value = rng.float().to_bits();
            t.w32(world + 4 * c, value);
        }
    }
    let renderer = obj + 0x900;
    t.w32(scene + 0xd0, renderer);
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    let (head_word, tail_word) = (rng.next(), rng.next());
    let ret = rng.next();
    t.stub_set_this(
        h24::TO_WORLD_MATRIX,
        1,
        ret,
        &[(0, head_word), (0x2c, tail_word)],
    );
    args[0] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
}

#[rustfmt::skip]
const CASES24D: &[Case] = &[
    framed(Case { prep: Some(prep_mesh_matrices), ..method("scene_draw_mesh_matrices", 0x0066_b360, This::Object, 1, &[(h24::RENDER_MESH_INSTANCE, K::This(4))], |m, this, a| { gh24::scene_draw_mesh_matrices(m, frame_ebp(), this, a[0]); Out::None }) }, 0x84, 0),
];

#[test]
fn batch24d_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES24D, n, &mut Rng(0x2424_b324));
    });
}

/// A balanced map of effects (keys zero or not, type bytes meeting the
/// mask or not, each with a stopping vtable), the kept map's head and
/// lookups, the mask (only its low byte counts).
fn prep_clear_by_type(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let head = obj + 0x40;
    t.w32(obj, head);
    let size = rng.next();
    t.w32(obj + 4, size);
    t.write(head + 0xd, &[1]);
    let n = rng.below(6);
    let node = |k: u32| obj + 0x100 + 0x20 * k;
    let root = build_map_tree(t, 0, n, head, head, &node);
    t.w32(head + 4, root);
    t.w32(head, if n == 0 { head } else { node(0) });
    t.w32(head + 8, if n == 0 { head } else { node(n - 1) });
    let vtable = obj + 0x1e00;
    let stop = t.stub(None, 0xf110, true, 0, 0, Some(rng.next()));
    t.w32(vtable + 0x10, stop);
    for k in 0..n {
        let (key_a, key_b) = match rng.below(4) {
            0 => (0, 0),
            1 => (0, rng.next()),
            _ => (rng.next(), rng.next()),
        };
        t.w32(node(k) + 0x10, key_a);
        t.w32(node(k) + 0x14, key_b);
        let effect = obj + 0x800 + 0x80 * k;
        t.w32(node(k) + 0x18, effect);
        t.w32(effect, vtable);
        t.write(effect + 0x51, &[1 << rng.below(4)]);
    }
    let kept_head = obj + 0x1c00;
    t.w32(kept_head, obj + 0x1c40);
    t.stub_ret(h24::MAP_HEAD, true, 0, 0, kept_head);
    let found = rng.next();
    t.stub_ret(h24::KEPT_FIND, true, 1, 1, found);
    args[0] = pick(rng, &[0, 1, 3, 0xc, 0xff, 0x100]);
}

const CLEAR_BY_TYPE_STUBS: &[(u32, K)] = &[
    (h24::MAP_LOCK, K::This(0)),
    (h24::IOB_FUNC, K::ImportC(0)),
    (h24::FPRINTF, K::ImportC(3)),
    (h24::KEPT_INSERT, K::This(4)),
    (h24::INSERT_TO_DELETE_POOL, K::This(4)),
    (h24::ERASE_RANGE, K::This(2)),
    (h24::TREE_ERASE_RANGE, K::This(3)),
    (h24::OPERATOR_DELETE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES24E: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_clear_by_type), ..method("vfx_table_clear_by_type", 0x0069_f110, This::Object, 1, CLEAR_BY_TYPE_STUBS, |m, this, a| { let slots = slots_at(m, a); gh24::vfx_table_clear_by_type(m, frame_ebp(), slots, this, a[0]); Out::None }) }), 0x34, 4),
];

#[test]
fn batch24e_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES24E, n, &mut Rng(0x2424_b424));
    });
}

// ---- round 7 batch 25 (ffx_prender::h3_glue25) ----

/// A texture-animation entry: items (0..3, count flag now and then), the
/// animations (0..3, each with two records or none, records owning their
/// data or not, data set or not, a destroying virtual), the word array and
/// the two tagged pointers.
fn prep_entry_release(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let n = rng.below(4);
    let count = if rng.chance(8) {
        0x8000_0000
    } else {
        count_word(rng, n)
    };
    t.w32(obj + 0x10, count);
    t.w32(
        obj + 0x14,
        if n == 0 && rng.chance(2) {
            0
        } else {
            obj + 0x100
        },
    );
    for k in 0..3 {
        t.w32(obj + 0x100 + 4 * k, obj + 0x140 + 0x10 * k);
    }
    let n = rng.below(4);
    let count = if rng.chance(8) {
        0x8000_0000
    } else {
        count_word(rng, n)
    };
    t.w32(obj + 8, count);
    t.w32(
        obj + 0xc,
        if n == 0 && rng.chance(2) {
            0
        } else {
            obj + 0x400
        },
    );
    let vtable = obj + 0x1e00;
    let ret = rng.next();
    let destroy = t.stub(None, 0x3fe0, true, 1, 1, Some(ret));
    t.w32(vtable, destroy);
    for k in 0..3 {
        let item = obj + 0x400 + 0x40 * k;
        t.w32(item, vtable);
        for (r, slot) in [0x38, 0x3c].into_iter().enumerate() {
            let record = obj + 0x800 + 0x20 * (2 * k + r as u32);
            t.w32(item + slot, if rng.chance(4) { 0 } else { record });
            let c = count_word(rng, 2);
            t.w32(record, c);
            let data = if rng.chance(3) { 0 } else { rng.next() };
            t.w32(record + 4, data);
        }
    }
    let n = rng.below(4);
    let c = count_word(rng, n);
    t.w32(obj, c);
    t.w32(obj + 4, obj + 0x600);
    for slot in [0x18, 0x1c] {
        let v = rng.next();
        t.w32(obj + slot, v);
    }
}

/// The map tree at `scene + 0x1198` as [`prep_map_yiq`] builds it: the
/// first chain (empty or 1..3 nodes), the successors, each node's lists
/// and material arrays, and the parameter lookups.
fn map_tree(t: &mut Twins, obj: u32, scene: u32, rng: &mut Rng) {
    let head = scene + 0x1198;
    let n = rng.below(4);
    let nodes: Vec<u32> = (0..n).map(|k| obj + 0x100 + 0x40 * k).collect();
    t.w32(head + 0xc, nodes.first().copied().unwrap_or(head));
    for (k, &node) in nodes.iter().enumerate() {
        t.w32(node, nodes.get(k + 1).copied().unwrap_or(head));
    }
    let mut order = vec![];
    for _ in 0..rng.below(4) {
        order.push(obj + 0x100 + 0x40 * rng.below(3));
    }
    order.extend([head; 4]);
    t.stub_seq(h25::TREE_SUCCESSOR, true, 1, 1, &order);
    for k in 0..3 {
        let node = obj + 0x100 + 0x40 * k;
        let list = obj + 0x300 + 0x40 * k;
        t.w32(node + 0x10, list);
        let c = rng.below(3);
        t.w32(list, c);
        t.w32(list + 8, list + 0x10);
        for i in 0..2 {
            let item = obj + 0x500 + 0x40 * (2 * k + i);
            t.w32(list + 0x10 + 4 * i, item);
            let array = obj + 0x800 + 0x10 * (2 * k + i);
            t.w32(item + 0x10, array);
            let c = pick(rng, &[0, 1, 2, 0x8000_0002]);
            t.w32(array, c);
        }
    }
    let data = obj + 0xa00;
    for m in 0..3 {
        let material = obj + 0xb00 + 0x20 * m;
        t.w32(data + 4 * m, material);
        let v = rng.next();
        t.w32(material, v);
        let buffer = obj + 0x1000 + 0x100 * m;
        t.w32(material + 4, buffer);
        let size = pick(rng, &[0, 0x10, 0x40, 0x80]);
        t.w32(buffer, size);
    }
    t.stub_ret(h25::ARRAY_DATA, true, 0, 0, data);
    let mut params = vec![];
    for p in 0..24 {
        let at = obj + 0x1800 + 0x10 * p;
        let offset = pick(rng, &[0, 3, 4, 8, 0x20, 0x4c, 0xfff0]);
        let size = pick(rng, &[0, 4, 12, 16, 0x24, 0x2c, 0x400, 0x1010]) | (rng.below(8) << 13);
        t.write(at + 8, &(offset as u16).to_le_bytes());
        t.write(at + 0xa, &(size as u16).to_le_bytes());
        params.push(if rng.chance(4) { 0 } else { at });
    }
    t.stub_seq(h25::FIND_PARAMETER, true, 1, 1, &params);
}

/// The map cluster (or none), the colour (floats; the fourth unused), the
/// light colour (0 or tiny now and then, so the squared length stays at or
/// below 0.0001), the light, and the map tree.
fn prep_colour_scale(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0e00);
    *this = scene;
    t.w32(scene + 0x10d4, if rng.chance(6) { 0 } else { obj });
    map_tree(t, obj, scene, rng);
    for a in args.iter_mut().take(3) {
        *a = rng.float().to_bits();
    }
    for c in 0..3 {
        let v = match rng.below(5) {
            0 => 0.0,
            1 => 0.004,
            2 => 0.01,
            _ => rng.float(),
        };
        t.w32(scene + 0x1_0d30 + 4 * c, v.to_bits());
    }
    if rng.chance(4) {
        // 0.01 squared rounds to the f32 just below 0.0001; the two tiny
        // squares (0.3 ulp each) lift it above only when summed before
        // the store rounds
        for (c, v) in [0x3c23_d70a, 0x35c6_4bf8, 0x35c6_4bf8]
            .into_iter()
            .enumerate()
        {
            t.w32(scene + 0x1_0d30 + 4 * c as u32, v);
        }
    }
    t.w32(scene + 0xcc, obj + 0x1c00);
}

const UPDATE_POOL_STUBS: &[(u32, K)] = &[
    (h25::OPERATOR_DELETE_ARRAY, K::Cdecl(1)),
    (h25::RELEASE_BUFFER_BY_INDEX, K::Cdecl(1)),
    (h25::OPERATOR_DELETE, K::Cdecl(1)),
    (h25::GEOMETRY_CLEAR, K::This(0)),
];

/// The pool's entries (0..4, count flag now and then): keys set or not,
/// counters about to expire or not, each object's geometry and segment
/// array at a slot of their pools (first, middle, last, past the end,
/// below the base), with 0..3 segments or a negative count; the low
/// marks.
fn prep_update_pool(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let entries = rng.below(5);
    let count = count_word(rng, entries);
    t.w32(obj + 8, count);
    t.w32(obj + 0xc, obj + 0x100);
    t.w32(obj + 0x14, obj + 0x140);
    t.w32(obj + 0x1c, obj + 0x180);
    let vtable = obj + 0x1e00;
    let ret = rng.next();
    let destroy = t.stub(None, 0xfe50, true, 1, 1, Some(ret));
    t.w32(vtable, destroy);
    for k in 0..4 {
        let (key_a, key_b) = match rng.below(4) {
            0 => (0, 0),
            1 => (0, rng.next()),
            2 => (rng.next(), 0),
            _ => (rng.next(), rng.next()),
        };
        t.w32(obj + 0x100 + 8 * k, key_a);
        t.w32(obj + 0x104 + 8 * k, key_b);
        let counter = pick(rng, &[0, 1, 2, 0xffff_ffff, 0x8000_0000]);
        t.w32(obj + 0x140 + 4 * k, counter);
        let object = obj + 0x200 + 0x40 * k;
        t.w32(obj + 0x180 + 4 * k, object);
        t.w32(object, vtable);
        let geometry = obj + 0x400 + 0xd0 * k;
        t.w32(object + 0x24, geometry);
        let count = pick(rng, &[0, 1, 1, 2, 3, 0xffff_ffff]);
        t.w32(geometry + 0x90, count);
        let segments = obj + 0x800 + 0x6c * 3 * k;
        t.w32(geometry + 0x94, segments);
        let index = rng.next();
        t.w32(geometry + 0xc0, index);
        for w in 0..0x51 {
            let v = rng.next();
            t.w32(segments + 4 * w, v);
        }
    }
    // the pools' bases: geometry 0 (and segment array 0) at geometry_slot (segment_slot)
    let slot = |rng: &mut Rng| pick(rng, &[0, 7, 2996, 2999, 3000, 3996, 4000, 0xffff_fff0]);
    let geometry_slot = slot(rng);
    t.w32(
        obj + 0x2c,
        (obj + 0x400).wrapping_sub(0xd0u32.wrapping_mul(geometry_slot)),
    );
    let segment_slot = slot(rng);
    t.w32(
        obj + 0x3c,
        (obj + 0x800).wrapping_sub(0x6cu32.wrapping_mul(segment_slot)),
    );
    // flags: the slots' bytes land in the object's last pages
    t.w32(obj + 0x34, (obj + 0x1a00).wrapping_sub(geometry_slot));
    t.w32(obj + 0x44, (obj + 0x1b00).wrapping_sub(segment_slot));
    if rng.chance(4) {
        // the base a little above geometry 0: its slot truncates to 0
        t.w32(obj + 0x2c, obj + 0x410);
        t.w32(obj + 0x34, obj + 0x1a00);
    }
    for low in [0x38, 0x48] {
        let v = pick(rng, &[0, 5, 2999, 3001, 0x7fff_ffff, 0xffff_ffff]);
        t.w32(obj + low, v);
    }
}

#[rustfmt::skip]
const CASES25: &[Case] = &[
    Case { prep: Some(prep_entry_release), ..method("texture_entry_release", 0x006b_3fe0, This::Object, 0, &[(h25::PHYRE_FREE, K::Cdecl(1)), (h25::OPERATOR_DELETE, K::Cdecl(1))], |m, this, _| { gh25::texture_entry_release(m, this); Out::None }) },
    framed(Case { prep: Some(prep_colour_scale), ..method("scene_set_material_colour_scale2", 0x0066_d1d0, This::Object, 4, &[(h25::MEMCPY, K::Cdecl(3))], |m, this, a| { let slots = slots_at(m, a); gh25::scene_set_material_colour_scale2(m, slots, this, a[0], a[1], a[2]); Out::None }) }, 0, 0x10),
    Case { prep: Some(prep_update_pool), ..method("vfx_table_update_delete_pool", 0x0069_fe50, This::Object, 0, UPDATE_POOL_STUBS, |m, this, _| { gh25::vfx_table_update_delete_pool(m, this); Out::None }) },
];

#[test]
fn batch25_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES25, n, &mut Rng(0x2525_b025));
    });
}

const SHADOW_MESH_STUBS: &[(u32, K)] = &[
    (h25::SEGMENTS_CONSTRUCT, K::Cdecl(2)),
    (h25::SEGMENTS_DESTROY, K::Cdecl(2)),
    (h25::STREAM_CTOR, K::This(0)),
    (h25::STREAMS_SET, K::This(2)),
    (h25::INDEX_IS_BOUND, K::This(0)),
    (h25::INDEX_RELEASE, K::This(0)),
    (h25::LAYOUT_SETUP, K::This(2)),
    (h25::SHADOW_INSTANCE_CTOR, K::This(1)),
];

/// The mesh's segment array (one entry or not, owned or not, the same
/// data as the allocation now and then), the allocations (failing now and
/// then), the stream count, the index size (12 times the format size or
/// not), the dynamic mesh (or none), the shadow instance's list and
/// instance (or none), the mesh instance.
fn prep_shadow_mesh(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    args[0] = obj + 0x100;
    let mesh = obj + 0x200;
    let block = obj + 0x400;
    let count = pick(rng, &[1, 0, 2, 0x8000_0001, 0x8000_0003, 0x8000_0000]);
    t.w32(mesh, count);
    t.w32(mesh + 4, if rng.chance(5) { block } else { obj + 0x300 });
    let dynamic = if rng.chance(5) { 0 } else { obj + 0x504 };
    t.stub_seq(h25::CLUSTER_CREATE, true, 3, 3, &[mesh, dynamic]);
    let first = if rng.chance(4) { 0 } else { block };
    let second = if rng.chance(4) { 0 } else { obj + 0x800 };
    // the stream block's allocation comes first when the segment is kept
    let allocations = if count == 1 {
        [second, 0]
    } else {
        [first, second]
    };
    t.stub_seq(h25::MALLOC_ALIGNED, false, 2, 0, &allocations);
    for segment in [obj + 0x300, block] {
        let streams = pick(rng, &[4, 0, 3, 0x8000_0004]);
        t.w32(segment + 0x28, streams);
        let size = pick(rng, &[0x60, 0x30, 0]);
        t.w32(segment + 0x64, size);
    }
    let format = pick(rng, &[8, 4, 0]);
    t.stub_ret(h25::FORMAT_SIZE, false, 1, 0, format);
    let lists = [if rng.chance(4) { 0 } else { obj + 0x600 }, obj + 0x680];
    t.stub_seq(h25::ALLOCATE_INSTANCE_LIST, true, 3, 3, &lists);
    t.w32(obj + 0x634, if rng.chance(4) { 0 } else { obj + 0x900 });
    t.w32(obj + 0x6b4, obj + 0xa00);
    t.stub_ret(h25::INSTANCE_CTOR, true, 1, 1, obj + 0x700);
}

const BROKEN_PASS_STUBS: &[(u32, K)] = &[
    (h25::FIND_PARAMETER, K::This(1)),
    (h25::PASS_SETUP_6D4780, K::This(2)),
    (h25::SET_STREAMS, K::This(3)),
    (h25::POLYGON_INSTANCE_CTOR, K::This(1)),
];

/// The streams array (inline first word, or a pointer from two on; the
/// flag bit now and then), the pass kind, the meshes (the dynamic one
/// missing now and then), the polygon instance's list and instance (or
/// none), the mesh instance.
fn prep_broken_pass(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let streams = obj + 0x100;
    args[1] = streams;
    let count = pick(rng, &[1, 2, 5, 0x8000_0001, 0x8000_0003]);
    t.w32(streams, count);
    if count & 0x7fff_ffff > 1 {
        t.w32(streams + 4, obj + 0x140);
    }
    let kind = pick(rng, &[6, 8, 0, 7, 0x106]);
    t.w32(obj + 8, kind);
    let mesh = rng.next();
    let dynamic = if rng.chance(5) { 0 } else { obj + 0x504 };
    t.stub_seq(h25::CLUSTER_CREATE, true, 3, 3, &[mesh, dynamic]);
    let lists = [if rng.chance(4) { 0 } else { obj + 0x600 }, obj + 0x680];
    t.stub_seq(h25::ALLOCATE_INSTANCE_LIST, true, 3, 3, &lists);
    t.w32(obj + 0x634, if rng.chance(4) { 0 } else { obj + 0x900 });
    t.w32(obj + 0x6b4, obj + 0xa00);
    t.stub_ret(h25::INSTANCE_CTOR, true, 1, 1, obj + 0x700);
}

#[rustfmt::skip]
const CASES25B: &[Case] = &[
    framed(Case { prep: Some(prep_shadow_mesh), ..method("shadow_mesh_create", 0x006c_9580, This::Object, 1, SHADOW_MESH_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh25::shadow_mesh_create(m, slots, this, a[0])) }) }, 0, 4),
    framed(Case { prep: Some(prep_broken_pass), ..method("broken_pass_setup", 0x006d_3fb0, This::Object, 6, BROKEN_PASS_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh25::broken_pass_setup(m, slots, this, a[0], a[1], a[2], a[3], a[4], a[5])) }) }, 0, 0x18),
];

#[test]
fn batch25b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES25B, n, &mut Rng(0x2525_b125));
    });
}

// ---- round 7 batch 26 (ffx_prender::h3_glue26) ----

impl Twins {
    /// A recording `memcpy` (cdecl, 3 words) that copies too.
    fn stub_copy(&mut self, sym: u32) {
        let log = self.stub(None, sym, false, 3, 0, Some(0));
        // push esi; push edi; push ecx; mov edi, [esp+0x10]; mov esi, [esp+0x14];
        // mov ecx, [esp+0x18]; rep movsb; pop ecx; pop edi; pop esi; jmp log
        let mut c = vec![
            0x56, 0x57, 0x51, 0x8b, 0x7c, 0x24, 0x10, 0x8b, 0x74, 0x24, 0x14, 0x8b, 0x4c, 0x24,
            0x18, 0xf3, 0xa4, 0x59, 0x5f, 0x5e,
        ];
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

const COPY_STUBS: &[(u32, K)] = &[
    (h26::SURFACE_CTOR, K::This(2)),
    (h26::SURFACE_DTOR, K::This(0)),
    (h26::SET_VIEWPORT, K::This(4)),
    (h26::SET_NON_16_BY_9_VIEWPORT, K::This(2)),
    (h26::RENDER_MESH_INSTANCE, K::This(4)),
    (h26::END_SCENE, K::This(1)),
    (h26::GET_APPLICATION, K::Getter),
    (h26::SET_VALUE, K::This(2)),
];

/// [`prep_full_screen`]'s set-up, with the parameters found in turn
/// (`"ColorBuffer"`, then the others), the two colour-target calls failing
/// apart, the wide flag (its low byte) and the manager's wide byte.
fn prep_copy(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_full_screen(t, obj, this, args, rng);
    let found: Vec<u32> = (0..3)
        .map(|k| {
            if rng.chance(4) {
                0
            } else {
                obj + 0x380 + 0x10 * k
            }
        })
        .collect();
    t.stub_seq(h26::FIND_PARAMETER, true, 1, 1, &found);
    let errors = [render_error(rng), render_error(rng)];
    t.stub_seq(h26::SET_COLOR_TARGET, true, 2, 2, &errors);
    t.write(*this + 0x1_23d9, &[u8::from(rng.chance(2))]);
    args[3] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
    args[4] = rng.next();
    args[5] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
}

/// [`prep_copy`] with the colour-scale parameter (in or out of the
/// buffer, which is real now), the colour (or none), the pass and the
/// two clear flags.
fn prep_copy_depth(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_copy(t, obj, this, &mut args[..6], rng);
    let buffer = obj + 0xb00;
    t.w32(obj + 0x304, buffer);
    let size = pick(rng, &[0x40, 0x10, 0]);
    t.w32(buffer, size);
    let scale = obj + 0x390;
    let offset = pick(rng, &[0, 3, 4, 8, 0x30, 0xfff0]);
    let n = pick(rng, &[0, 4, 0x10, 0x14, 0x2000 | 8]);
    t.write(scale + 8, &(offset as u16).to_le_bytes());
    t.write(scale + 0xa, &(n as u16).to_le_bytes());
    let wide = args[3];
    let unused = args[4];
    let mode = args[5];
    args[3] = rng.next();
    args[4] = if rng.chance(5) { 0 } else { obj + 0xc00 };
    args[5] = wide;
    args[6] = unused;
    args[7] = mode;
    args[8] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
}

const BRIGHT_STUBS: &[(u32, K)] = &[
    (h26::SURFACE_CTOR, K::This(2)),
    (h26::SURFACE_DTOR, K::This(0)),
    (h26::SET_VIEWPORT, K::This(4)),
    (h26::COPY_BUFFER, K::This(6)),
];

/// The enable bytes, the bright mesh's material (inline or not; a real
/// buffer), the parameters (or none; in or out of the buffer, copying 4
/// bytes or fewer), the brightness, the target and the own target
/// (textures or not), each renderer call failing now and then.
fn prep_bright(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    t.write(
        post + 0x1_23d6,
        &[u8::from(!rng.chance(6)), u8::from(!rng.chance(6))],
    );
    let mesh = obj + 0x100;
    t.w32(post + 0x1_2320, mesh);
    let materials = obj + 0x200;
    t.w32(mesh + 0x10, materials);
    let count = pick(rng, &[1, 2, 0x8000_0001, 0x8000_0002]);
    t.w32(materials, count);
    let material = obj + 0x300;
    if count & 0x7fff_ffff > 1 {
        t.w32(materials + 4, obj + 0x280);
        t.w32(obj + 0x280, material);
    } else {
        t.w32(materials + 4, material);
    }
    let shader = rng.next();
    t.w32(material, shader);
    let buffer = obj + 0xb00;
    t.w32(material + 4, buffer);
    t.w32(buffer, 0x20);
    let found: Vec<u32> = (0..2)
        .map(|k| {
            if rng.chance(4) {
                0
            } else {
                obj + 0x380 + 0x10 * k
            }
        })
        .collect();
    t.stub_seq(h26::FIND_PARAMETER, true, 1, 1, &found);
    let offset = pick(rng, &[0, 3, 4, 8, 0x1c, 0x1e]);
    let n = pick(rng, &[0, 2, 4, 0x2000 | 4, 0x30]);
    t.write(obj + 0x388, &(offset as u16).to_le_bytes());
    t.write(obj + 0x38a, &(n as u16).to_le_bytes());
    let level = match rng.below(4) {
        0 => 50.0,
        1 => 75.5,
        2 => -3.25,
        _ => rng.float(),
    };
    t.w32(post + 0x1_2328, level.to_bits());
    t.stub_copy(h26::MEMCPY);
    let error = render_error(rng);
    t.stub_ret(h26::BIND_TEXTURE, true, 2, 2, error);
    for (sym, n) in [
        (h26::SET_COLOR_TARGET, 2),
        (h26::SET_DEPTH_STENCIL_TARGET, 1),
        (h26::SET_CLEAR_COLOR, 4),
        (h26::BEGIN_SCENE, 1),
        (h26::RENDER_MESH_INSTANCE, 4),
        (h26::END_SCENE, 1),
    ] {
        let error = render_error(rng);
        t.stub_ret(sym, true, n, n, error);
    }
    let renderer = obj + 0x800;
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    for (target, texture) in [(obj + 0x900, obj + 0xa00), (obj + 0x940, obj + 0xa40)] {
        t.w32(target, texture);
        // the own target (+0xa4) always has its texture: the viewport
        // reads through it
        let flag = target == obj + 0x900 && rng.chance(4);
        t.write(target + 4, &[u8::from(flag)]);
        let w = rng.next();
        t.w32(texture + 0x1c, w);
        let ht = rng.next();
        t.w32(texture + 0x20, ht);
    }
    t.w32(post + 0xa4, obj + 0x940);
    args[0] = renderer;
    args[1] = obj + 0x900;
}

#[rustfmt::skip]
const CASES26: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_copy), ..method("post_copy_buffer", 0x0069_2c40, This::Object, 6, COPY_STUBS, |m, this, a| u(gh26::post_copy_buffer(m, frame_ebp(), this, a[0], a[1], a[2], a[3], a[5]))) }), 0x30, 0),
    framed(with_seh(Case { prep: Some(prep_copy_depth), ..method("post_copy_depth_buffer", 0x0069_2e90, This::Object, 9, COPY_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh26::post_copy_depth_buffer(m, frame_ebp(), slots, this, a[0], a[1], a[2], a[3], a[4], a[5], a[7], a[8])) }) }), 0x30, 0x24),
    framed(with_seh(Case { prep: Some(prep_bright), ..method("post_render_bright", 0x0069_5440, This::Object, 2, BRIGHT_STUBS, |m, this, a| { gh26::post_render_bright(m, frame_ebp(), this, a[0], a[1]); Out::None }) }), 0x34, 0),
];

#[test]
fn batch26_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES26, n, &mut Rng(0x2626_b026));
    });
}

// ---- round 7 batch 27 (ffx_prender::h3_glue27) ----

const GLOW_STUBS: &[(u32, K)] = &[
    (h27::SURFACE_CTOR, K::This(2)),
    (h27::SURFACE_DTOR, K::This(0)),
    (h27::RENDER_MESH_INSTANCE, K::This(4)),
];

/// The glow mesh's material (inline or not; a real buffer of 0x30 bytes
/// or less), the four parameters (or none; in or out of the buffer,
/// copies that stay inside the frame's records), the thresholds' base (a
/// null source now and then), each renderer call failing now and then.
fn prep_glow_map(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let post = t.zeros(0x1_2400);
    *this = post;
    let mesh = obj + 0x100;
    t.w32(post + 0x1210, mesh);
    let materials = obj + 0x200;
    t.w32(mesh + 0x10, materials);
    let count = pick(rng, &[1, 2, 0x8000_0001, 0x8000_0002]);
    t.w32(materials, count);
    let material = obj + 0x300;
    if count & 0x7fff_ffff > 1 {
        t.w32(materials + 4, obj + 0x280);
        t.w32(obj + 0x280, material);
    } else {
        t.w32(materials + 4, material);
    }
    let shader = rng.next();
    t.w32(material, shader);
    let buffer = obj + 0xb00;
    t.w32(material + 4, buffer);
    let size = pick(rng, &[0x30, 0x20, 0x10]);
    t.w32(buffer, size);
    let found: Vec<u32> = (0..4)
        .map(|k| {
            if rng.chance(4) {
                0
            } else {
                obj + 0x380 + 0x10 * k
            }
        })
        .collect();
    t.stub_seq(h27::FIND_PARAMETER, true, 1, 1, &found);
    for k in 0..4 {
        let at = obj + 0x380 + 0x10 * k;
        let offset = pick(rng, &[0, 3, 4, 8, 0x10, 0x1c]);
        let n = pick(rng, &[0, 4, 0x10, 0x14, 0x2000 | 8]);
        t.write(at + 8, &(offset as u16).to_le_bytes());
        t.write(at + 0xa, &(n as u16).to_le_bytes());
    }
    // a null source for either threshold (the other one's would fault)
    let base = if found[2] == 0 {
        pick(rng, &[obj + 0xc00, 0xffff_ff7c])
    } else {
        pick(rng, &[obj + 0xc00, obj + 0xc00, 0xffff_ff80])
    };
    t.w32(post, base);
    t.stub_copy(h27::MEMCPY);
    let error = render_error(rng);
    t.stub_ret(h27::BIND_TEXTURE, true, 2, 2, error);
    for (sym, n) in [
        (h27::SET_COLOR_TARGET, 2),
        (h27::SET_DEPTH_STENCIL_TARGET, 1),
        (h27::BEGIN_SCENE, 1),
        (h27::END_SCENE, 1),
    ] {
        let error = render_error(rng);
        t.stub_ret(sym, true, n, n, error);
    }
    let renderer = obj + 0x800;
    let v = rng.next();
    t.w32(renderer + 0x13c, v);
    args[0] = renderer;
    args[1] = rng.next();
    args[2] = rng.next();
}

const LAYOUT_STUBS: &[(u32, K)] = &[
    (h27::SEGMENTS_CONSTRUCT, K::Cdecl(2)),
    (h27::SEGMENTS_DESTROY, K::Cdecl(2)),
    (h27::STREAM_CTOR, K::This(0)),
    (h27::STREAMS_SET, K::This(2)),
    (h27::INDEX_IS_BOUND, K::This(0)),
    (h27::INDEX_RELEASE, K::This(0)),
    (h27::FILL_COLOURS, K::This(0)),
    (h27::FILL_INDICES, K::This(0)),
];

/// The vertex count (6, 8 or another), the mesh's segment array and its
/// replacement (as [`prep_shadow_mesh`]), the stream count, the index
/// size (the format size times the kind's count or not), the buffer.
fn prep_broken_layout(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    let mesh = obj + 0x200;
    t.w32(obj + 0x20, mesh);
    let primitive = rng.next();
    t.w32(obj + 4, primitive);
    let block = obj + 0x400;
    let count = pick(rng, &[1, 0, 2, 0x8000_0001, 0x8000_0003, 0x8000_0000]);
    t.w32(mesh, count);
    t.w32(mesh + 4, if rng.chance(5) { block } else { obj + 0x300 });
    let first = if rng.chance(4) { 0 } else { block };
    let second = if rng.chance(4) { 0 } else { obj + 0x800 };
    let allocations = if count == 1 {
        [second, 0]
    } else {
        [first, second]
    };
    t.stub_seq(h27::MALLOC_ALIGNED, false, 2, 0, &allocations);
    for segment in [obj + 0x300, block] {
        let streams = pick(rng, &[3, 0, 4, 0x8000_0003]);
        t.w32(segment + 0x28, streams);
        let size = pick(rng, &[0x60, 0x90, 0]);
        t.w32(segment + 0x64, size);
    }
    let format = pick(rng, &[4, 2, 0]);
    t.stub_ret(h27::FORMAT_SIZE, false, 1, 0, format);
    args[0] = pick(rng, &[6, 8, 6, 8, 0, 1, 3]);
    args[1] = obj + 0x1000;
}

#[rustfmt::skip]
const CASES27: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_glow_map), ..method("post_prepare_glow_map", 0x0069_4c60, This::Object, 3, GLOW_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh27::post_prepare_glow_map(m, frame_ebp(), slots, this, a[0], a[1], a[2])) }) }), 0x4c, 0xc),
    framed(Case { prep: Some(prep_broken_layout), ..method("broken_pass_layout", 0x006d_4780, This::Object, 2, LAYOUT_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh27::broken_pass_layout(m, slots, this, a[0], a[1])) }) }, 0, 8),
];

#[test]
fn batch27_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 240 };
        run_cases(t, CASES27, n, &mut Rng(0x2727_b027));
    });
}

impl Twins {
    /// A recording `Project3DPointOntoScreen(this; out, p, mode)` stub
    /// (`ret 0xc`) that writes the point `{x, y}` and leaves 1 in `ST0`.
    fn stub_project(&mut self, sym: u32, x: u32, y: u32) {
        let log = self.stub(None, sym, true, 3, 3, Some(0));
        // mov eax, [esp+4]; mov [eax], x; mov [eax+4], y; fld1; jmp log
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
        c.extend(x.to_le_bytes());
        c.extend([0xc7, 0x40, 0x04]);
        c.extend(y.to_le_bytes());
        c.extend([0xd9, 0xe8]);
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

/// The level (the event ones, the offset ones, another), the scenario
/// flags in turn, the gates and the map, the 3D flags, the projected point,
/// the window height (on the image), the battle scene, the offset.
fn prep_cha_pos(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let level = pick(
        rng,
        &[
            0x220, 0x11ee, 0x11ee, 0x1e63, 0x226d, 0x159c, 0x5c8, 0x2ed, 0x1234,
        ],
    );
    t.w32(scene + 0xfaf4, level);
    let flags: Vec<u32> = (0..4)
        .map(|_| pick(rng, &[0x1c7, 0x1e8, 0x1ea, 0x194, 0x7e4, 0x101c7, 3]))
        .collect();
    t.stub_seq(h27::SCENARIO_FLAG, false, 0, 0, &flags);
    for gate in [h27::SCENARIO_GATE, h27::LOW_GATE] {
        let v = pick(rng, &[0, 1, 0x100]);
        t.global(gate, v);
    }
    let map = pick(rng, &[0x133, 0x134, 0x10133]);
    t.global(h27::CURRENT_MAP_NO, map);
    // the flag byte, and now and then set bytes after it (a byte read)
    let at = t.rebase(h27::SCENE_3D);
    let after = if rng.chance(3) { [1, 1, 1] } else { [0, 0, 0] };
    t.patch(at, &[u8::from(rng.chance(6)), after[0], after[1], after[2]]);
    t.write(scene + 0xf9c4, &[u8::from(!rng.chance(6))]);
    let (x, y) = (
        rng.next(),
        pick(rng, &[0, 100, 360, 0xffff_ff9c, 0x7fff_ffff, 0x0100_0001]),
    );
    t.stub_project(h27::PROJECT_POINT, x, y);
    let height = pick(rng, &[720, 1080, 0, 0x8000_0000, 0xffff_fff8, 0x0100_0001]);
    t.w32(obj + 0x270, height);
    let battle = pick(rng, &[0, 0x0205_0000, 0x0205_0001]);
    t.stub_ret(h27::BATTLE_SCENE, false, 0, 0, battle);
    let offset = rng.float();
    t.w32(scene + 0xfa38, offset.to_bits());
    args[0] = rng.next();
}

#[rustfmt::skip]
const CASES27B: &[Case] = &[
    framed(Case { prep: Some(prep_cha_pos), ..method("scene_get_cha_pos", 0x0065_d860, This::Object, 1, &[(h27::GET_APPLICATION, K::Getter)], |m, this, a| { let slots = slots_at(m, a); gh27::scene_get_cha_pos(m, frame_ebp(), slots, this, a[0]); Out::None }) }, 8, 4),
];

#[test]
fn batch27b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 400 };
        run_cases(t, CASES27B, n, &mut Rng(0x2727_b127));
    });
}

// ---- round 7 batch 28 (ffx_prender::h3_glue28) ----

/// The icon id (each range, its edges, 0x261, others), the position and
/// scale floats, the map (0xb8 or not), the language (0xb or not).
fn prep_scale_icon(t: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    args[0] = pick(
        rng,
        &[
            600, 605, 611, 0x261, 0x261, 0x257, 0x264, 0x267, 0x268, 0x268, 0x269, 0x269, 0x26a, 0,
        ],
    );
    args[1] = rng.float().to_bits();
    args[2] = rng.float().to_bits();
    let scale = match rng.below(4) {
        0 => 1.0,
        1 => 0.75,
        2 => 1.25,
        _ => rng.float(),
    };
    args[3] = f32::to_bits(scale);
    let map = pick(rng, &[0xb8, 0xb8, 0xb9]);
    t.global(h28::CURRENT_MAP_NO, map);
    let lang = pick(rng, &[0xb, 1, 0x10b]);
    t.stub_ret(h28::GET_LANG, false, 0, 0, lang);
}

#[rustfmt::skip]
const CASES28: &[Case] = &[
    framed(Case { prep: Some(prep_scale_icon), ..cdecl("draw_scale_button_icon", 0x008e_7a90, 6, &[(h28::DRAW_CROSS_BUTTON_ICON, K::Cdecl(7))], |m, _, a| { let slots = slots_at(m, a); u(gh28::draw_scale_button_icon(m, slots, a[0], a[1], a[2], a[3], a[4], a[5])) }) }, 0, 0x18),
];

#[test]
fn batch28_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 80 } else { 480 };
        run_cases(t, CASES28, n, &mut Rng(0x2828_b028));
    });
}

// ---- round 7 batch 29 (ffx_prender::h3_glue29) ----

impl Twins {
    /// A recording `thiscall(this; &x, &y)` stub (`ret 8`) that stores
    /// `x` and `y` through its two pointers and returns `ret`.
    fn stub_two_outs(&mut self, sym: u32, x: u32, y: u32, ret: u32) {
        let log = self.stub(None, sym, true, 2, 2, Some(ret));
        // mov eax, [esp+4]; mov [eax], x; mov eax, [esp+8]; mov [eax], y; jmp log
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
        c.extend(x.to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]);
        c.extend(y.to_le_bytes());
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

/// The instance key (none now and then), its render data (or none), the
/// count (1 with or without a pooled segment, more, 0, negative), the
/// allocations, the arrays and names, `c` or none, the dynamic buffer id.
fn prep_data_buffer(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let (key_a, key_b) = match rng.below(5) {
        0 => (0, 0),
        1 => (0, rng.next()),
        _ => (rng.next(), rng.next()),
    };
    args[0] = key_a;
    args[1] = key_b;
    t.global(h29::VFX_TABLE, obj + 0x1f00);
    let entry = if rng.chance(6) { 0 } else { obj + 0x100 };
    t.stub_ret(h29::GET_VFX_DATA, true, 2, 2, entry);
    args[2] = pick(rng, &[1, 1, 1, 2, 3, 0, 0x8000_0000]);
    let found = pick(rng, &[0, 1, 0x100]);
    t.stub_two_outs(h29::POOL_SEGMENT, obj + 0x400, obj + 0x600, found);
    let news = [obj + 0x800, obj + 0xa00, obj + 0xb00, obj + 0xc00];
    t.stub_seq(h29::OPERATOR_NEW_ARRAY, false, 1, 0, &news);
    let (a_words, b_words, c_words) = (obj + 0x1000, obj + 0x1040, obj + 0x1080);
    for k in 0..3 {
        for array in [a_words, b_words, c_words] {
            let v = rng.next();
            t.w32(array + 4 * k, v);
        }
    }
    args[3] = a_words;
    args[4] = b_words;
    let names = obj + 0x1100;
    for k in 0..3 {
        letters(t, names + 0x100 * k, pick(rng, &[0, 1, 5, 12]), rng);
    }
    args[5] = names;
    args[6] = if rng.chance(3) { 0 } else { c_words };
    t.global(h29::DYN_GEO_FILL, obj + 0x1e00);
    let id = pick(rng, &[0, 1]);
    t.stub_ret(h29::CURRENT_BUFFER_ID, true, 0, 0, id);
}

const WATER_STUBS: &[(u32, K)] = &[
    (h29::TEXTURE_CHECK, K::This(1)),
    (h29::MEMCPY, K::Cdecl(3)),
    (h29::SET_TEXTURE_CHECKED, K::This(2)),
    (h29::PHYRE_PRINTF, K::Cdecl(3)),
    (h29::REGISTER_DELEGATION, K::This(4)),
    (h29::OPERATOR_DELETE, K::Cdecl(1)),
];

/// The task (or none): its scene (an empty list now and then), three
/// words, the two targets (their textures, or flagged); the cursor's runs
/// over materials (shader, a real buffer); the parameters found in turn
/// (kinds that copy or not, in or out of the buffer).
fn prep_water(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let task = obj + 0x40;
    args[0] = if rng.chance(8) { 0 } else { task };
    let scene = obj + 0x100;
    t.w32(task, scene);
    let list = scene + 0x1c;
    t.w32(list, if rng.chance(4) { list } else { obj + 0x140 });
    for k in 0..3 {
        let v = rng.next();
        t.w32(task + 4 + 4 * k, v);
    }
    let (first, second) = (obj + 0x180, obj + 0x190);
    t.w32(task + 0xc, first);
    t.w32(task + 0x10, second);
    for target in [first, second] {
        let texture = if rng.chance(4) { 0 } else { rng.next() };
        t.w32(target, texture);
        t.write(target + 4, &[u8::from(rng.chance(4))]);
    }
    let area = obj + 0x400;
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script(h29::CURSOR_SEEK4, h29::CURSOR_NEXT4, &steps);
    let buffer = obj + 0x1c00;
    let size = pick(rng, &[0x38, 0x20, 0x10]);
    t.w32(buffer, size);
    for e in 0..12 {
        let element = area + 0x100 * e;
        let shader = rng.next();
        t.w32(element + 4, shader);
        t.w32(element + 8, buffer);
    }
    let found: Vec<u32> = (0..48)
        .map(|k| {
            if rng.chance(4) {
                0
            } else {
                obj + 0x1d00 + 0x10 * (k % 16)
            }
        })
        .collect();
    t.stub_seq(h29::FIND_PARAMETER, true, 1, 1, &found);
    for k in 0..16 {
        let at = obj + 0x1d00 + 0x10 * k;
        let kind = pick(rng, &[0x42, 0xd9, 0xdc, 0x12, 0x10, 0x43]);
        t.write(at + 2, &[kind as u8]);
        let offset = pick(rng, &[0, 3, 4, 8, 0x20]);
        let n = pick(rng, &[0, 4, 0x10, 0x30, 0x2000 | 8]);
        t.write(at + 8, &(offset as u16).to_le_bytes());
        t.write(at + 0xa, &(n as u16).to_le_bytes());
    }
    t.stub_ret(h29::OPERATOR_NEW, false, 1, 0, obj + 0x1f00);
    t.global(h29::RENDER_DELEGATION_HELPER, obj + 0x1f80);
}

#[rustfmt::skip]
const CASES29: &[Case] = &[
    framed(Case { prep: Some(prep_data_buffer), ..cdecl("vfx_create_data_buffer", 0x0064_5040, 7, &[(h29::STRNCPY, K::ImportC(3))], |m, _, a| { let slots = slots_at(m, a); gh29::vfx_create_data_buffer(m, frame_ebp(), slots, a[0], a[1], a[2], a[3], a[4], a[5], a[6]); Out::None }) }, 0xc, 0x1c),
    framed(Case { prep: Some(prep_water), ..cdecl("process_water_distortion", 0x006f_54f0, 1, WATER_STUBS, |m, _, a| { let slots = slots_at(m, a); gh29::process_water_distortion(m, frame_ebp(), slots, a[0]); Out::None }) }, 0x64, 4),
];

#[test]
fn batch29_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES29, n, &mut Rng(0x2929_b029));
    });
}

// ---- round 7 batch 30 (ffx_prender::h3_glue30) ----

const INSTANTIATE_STUBS: &[(u32, K)] = &[
    (h30::SET_MIN_AND_SIZE, K::This(2)),
    (h30::TREE_INSERT_ARRAY, K::This(2)),
    (h30::USER_FREE, K::Cdecl(1)),
];

/// The model (its object or none, its bounds or none), the cluster
/// choice, the bounds list and instance (or none), the key's array found
/// (with room or full) or missing, the reserve giving new data or none.
fn prep_instantiate(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let model = obj + 0x40;
    args[0] = model;
    let mesh = rng.next();
    t.w32(model, mesh);
    let object = obj + 0x80;
    t.w32(model + 0x18, if rng.chance(3) { 0 } else { object });
    t.w32(object + 4, obj + 0xa0);
    t.w32(obj + 0xa0, obj + 0x1e00);
    let ret = rng.next();
    let slot1 = t.stub(None, 0xf7d0, true, 2, 2, Some(ret));
    t.w32(obj + 0x1e04, slot1);
    let source = obj + 0xc0;
    t.w32(model + 0x1c, if rng.chance(3) { 0 } else { source });
    for k in 0..7 {
        let v = rng.float().to_bits();
        t.w32(source + 4 * k, v);
    }
    args[1] = rng.next();
    args[2] = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
    t.stub_ret(h30::OPERATOR_NEW, false, 1, 0, obj + 0x300);
    let second = if rng.chance(4) { 0 } else { obj + 0x480 };
    t.stub_seq(
        h30::ALLOCATE_INSTANCE_LIST,
        true,
        3,
        3,
        &[obj + 0x400, second],
    );
    t.w32(obj + 0x434, obj + 0x500);
    t.w32(obj + 0x4b4, if rng.chance(4) { 0 } else { obj + 0x580 });
    t.stub_ret(h30::INSTANCE_CTOR, true, 1, 1, obj + 0x600);
    t.stub_ret(h30::BOUNDS_LINK, true, 1, 1, obj + 0x700);
    let matrix = rng.next();
    t.stub_ret(h30::CLUSTER_CREATE, true, 3, 3, matrix);
    let node = match rng.below(4) {
        0 => 0,
        1 => 0xffff_fff0,
        _ => obj + 0x800,
    };
    t.stub_ret(h30::TREE_FIND, true, 1, 1, node);
    // counts and capacities compared signed (negative ones too)
    let count = pick(rng, &[0, 1, 3, 7, 0xffff_ffff, 0xffff_ffe9]);
    t.w32(obj + 0x810, count);
    let capacity = pick(rng, &[0, 1, 4, 8, 0xffff_ffff, 0x8000_0000]);
    t.w32(obj + 0x814, capacity);
    // no data in a one-entry array with room: its slot address is 0
    let kept = if count == 0 && capacity == 4 && rng.chance(2) {
        0
    } else {
        obj + 0x900
    };
    t.w32(obj + 0x818, kept);
    // no new data only for the new array (a found one's store would fault)
    let data = if node != obj + 0x800 && rng.chance(3) {
        0
    } else {
        obj + 0x980
    };
    let ret = rng.next();
    t.stub_set_this(h30::ARRAY_RESERVE, 1, ret, &[(8, data)]);
}

#[rustfmt::skip]
const CASES30: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_instantiate), ..method("scene_instantiate_mesh", 0x0066_f7d0, This::Object, 3, INSTANTIATE_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh30::scene_instantiate_mesh(m, frame_ebp(), slots, this, a[0], a[2])) }) }), 0x3c, 0xc),
];

#[test]
fn batch30_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES30, n, &mut Rng(0x3030_b030));
    });
}

// ---- round 8 batch 31 (ffx_prender::h3_glue31) ----

impl Twins {
    /// A recording method stub at `sym` (no arguments, `this` logged)
    /// that leaves the `double` `value` in `ST0`.
    fn stub_f64_this(&mut self, sym: u32, value: f64) {
        let cell = self.code(&value.to_bits().to_le_bytes());
        let log = self.stub(None, sym, true, 0, 0, Some(0));
        let mut c = vec![0xdd, 0x05]; // fld qword [cell]
        c.extend(cell.to_le_bytes());
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A recording cursor step at `sym` (thiscall, `n` words, returning
    /// `ret`): the count `+0xc` goes down by one and the offset `+0x10`
    /// up by 8.
    fn stub_cursor_step(&mut self, sym: u32, n: u32, ret: u32) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        // dec dword [ecx+0xc]; add dword [ecx+0x10], 8
        let mut c = vec![0xff, 0x49, 0x0c, 0x83, 0x41, 0x10, 0x08];
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

/// The shadow manager's five targets, the renderer, and each of the
/// renderer's calls failing at one point now and then.
fn prep_clear_targets(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for (target, _) in h31::SHADOW_TARGETS {
        let v = rng.next();
        t.w32(obj + target, v);
    }
    args[0] = rng.next();
    for (sym, n) in [
        (h31::SET_COLOR_TARGET, 2),
        (h31::SET_DEPTH_STENCIL_TARGET, 1),
        (h31::BEGIN_SCENE, 1),
        (h31::END_SCENE, 1),
    ] {
        let v = errors(rng);
        t.stub_seq(sym, true, n, n, &v);
    }
}

const CLEAR_TARGETS_STUBS: &[(u32, K)] = &[
    (h31::SURFACE_CTOR, K::This(2)),
    (h31::SURFACE_DTOR, K::This(0)),
    (h31::CLEAR_SCENE, K::This(1)),
    (h31::SET_CLEAR_DEPTH, K::This(1)),
    (h31::SET_CLEAR_COLOR, K::This(4)),
];

/// The scene's pool (index inside, at the end, past it or negative; the
/// next free slot near, far or none; slot and bounds bases that make
/// the entry 0) or another cluster, and the instance, bounds and lists
/// made or not.
fn prep_new_mesh_instance(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    args[0] = rng.next();
    args[1] = if rng.chance(3) {
        obj + 0x40
    } else {
        scene + 0x12d4
    };
    let any = rng.below(3000);
    let index = pick(
        rng,
        &[
            0,
            1,
            2998,
            2999,
            3000,
            3001,
            0xffff_ffff,
            0xffff_fffe,
            any,
            any,
        ],
    );
    t.w32(scene + 0xfbe4, index);
    let used = t.zeros(3000 + 0x20) + 0x10;
    t.write(used, &[1; 3000]);
    let free = index.wrapping_add(1 + rng.below(6));
    if rng.chance(4) && free < 3000 {
        t.write(used + free, &[0]);
    } else if !rng.chance(3) && free < 3000 {
        let far = free + rng.below(3000 - free);
        t.write(used + far, &[0]);
    }
    t.w32(scene + 0xfbe0, used);
    let slots = if rng.chance(5) {
        index.wrapping_mul(0x84).wrapping_neg()
    } else {
        obj + 0x100
    };
    t.w32(scene + 0xfbcc, slots);
    let bounds = if rng.chance(5) {
        index.wrapping_mul(0x24).wrapping_neg()
    } else {
        obj + 0x180
    };
    t.w32(scene + 0xfbd0, bounds);
    let matrices = rng.next();
    t.w32(scene + 0xfbd4, matrices);
    let instance = if rng.chance(5) { 0 } else { obj + 0x300 };
    t.stub_ret(h31::INSTANCE_CTOR, true, 1, 1, instance);
    let linked = if rng.chance(5) { 0 } else { obj + 0x400 };
    t.stub_ret(h31::BOUNDS_LINK, true, 1, 1, linked);
    let identity = rng.next();
    t.stub_ret(h31::IDENTITY, false, 1, 0, identity);
    let first = if rng.chance(6) { 0 } else { obj + 0x500 };
    let second = if rng.chance(6) { 0 } else { obj + 0x580 };
    t.stub_seq(h31::ALLOCATE_INSTANCE_LIST, true, 3, 3, &[first, second]);
    let at = if rng.chance(6) { 0 } else { rng.next() };
    t.w32(obj + 0x534, at);
    let at = if rng.chance(6) { 0 } else { rng.next() };
    t.w32(obj + 0x5b4, at);
    let matrix = rng.next();
    t.stub_ret(h31::CLUSTER_CREATE, true, 3, 3, matrix);
}

const NEW_MESH_INSTANCE_STUBS: &[(u32, K)] = &[(h31::TO_WORLD, K::This(1))];

/// The shadow mode (each, and others), the camera, the battle checks,
/// the map loaded or loading, and the materials' entries (set or not;
/// counts with the top bit set too) with their shadow maps.
fn prep_render_shadow(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let mode = pick(rng, &[0, 1, 2, 3, 4, 4, 5, 0xffff_ffff]);
    t.w32(scene + 0xf9cc, mode);
    let camera = if rng.chance(3) { 0 } else { rng.next() };
    t.w32(scene + 0xdc, camera);
    for at in [0xd0, 0xfaf4] {
        let v = rng.next();
        t.w32(scene + at, v);
    }
    let loading = u8::from(rng.chance(5));
    t.write(scene + 0x1195, &[loading]);
    let map = if rng.chance(5) { 0 } else { rng.next() };
    t.w32(scene + 0x10d4, map);
    let n = pick(rng, &[0, 1, 2, 4, 0x8000_0003]);
    t.w32(scene + 0x38c4, n);
    for (k, list) in [0x38a8, 0x38b0, 0x38b8, 0x38c0, 0x38c8]
        .into_iter()
        .enumerate()
    {
        let base = obj + 0x200 + 0x20 * k as u32;
        t.w32(scene + list, base);
        for i in 0..4 {
            let v = if list == 0x38c8 {
                obj + 0x300 + 0x10 * i
            } else if rng.chance(3) {
                0
            } else {
                rng.next()
            };
            t.w32(base + 4 * i, v);
        }
    }
    for i in 0..4 {
        let v = rng.next();
        t.w32(obj + 0x304 + 0x10 * i, v);
    }
    let shadow = obj + 0x1000;
    t.stub_ret(h31::SHADOW_INSTANCE, false, 0, 0, shadow);
    let battles: Vec<u32> = (0..16)
        .map(|_| if rng.chance(2) { 0 } else { rng.word() })
        .collect();
    t.stub_seq(h31::BATTLE_CHECK, false, 0, 0, &battles);
    t.w32(shadow + 0x68, obj + 0x1400);
    t.w32(shadow + 0x64, obj + 0x1500);
    for map in [obj + 0x1400, obj + 0x1500] {
        let tex = rng.next();
        t.w32(map, tex);
        let flag = u8::from(rng.chance(3));
        t.write(map + 4, &[flag]);
    }
    let value = f64::from(rng.float());
    t.stub_f64_this(h31::AVERAGE_CASTER_VALUE, value);
}

const RENDER_SHADOW_STUBS: &[(u32, K)] = &[
    (h31::UPDATE_CASTER_VISIBILITY, K::This(1)),
    (h31::UPDATE_CASCADES, K::This(4)),
    (h31::SHADOW_PASS_9E50, K::This(3)),
    (h31::SHADOW_PASS_AEE0, K::This(3)),
    (h31::CASCADE_MATRIX, K::This(2)),
    (h31::COPY_TO_BUFFER, K::This(2)),
    (h31::SET_VALUE2, K::This(2)),
    (h31::SET_TEXTURE_CHECKED, K::This(2)),
];

/// The scene's switches, the shadow manager's flag, the map objects (0
/// to 3) with their parameters found or not, and shadow sizes (0, 64,
/// powers of two, negative, any).
fn prep_scale_bias(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    let on = u8::from(!rng.chance(5));
    t.write(scene + 0x10d1, &[on]);
    let map = if rng.chance(6) { 0 } else { rng.next() };
    t.w32(scene + 0x10d4, map);
    let loading = u8::from(rng.chance(6));
    t.write(scene + 0x1195, &[loading]);
    let shadow = t.zeros(0x4100);
    let flag = u8::from(!rng.chance(5));
    t.write(shadow + 0x4064, &[flag]);
    t.stub_ret(h31::SHADOW_INSTANCE, false, 0, 0, shadow);
    let objects = obj + 0x400;
    let count = rng.below(4);
    let ret = rng.next();
    t.stub_set_this(
        h31::CURSOR_CTOR,
        1,
        ret,
        &[(0, objects), (0xc, count), (0x10, 0)],
    );
    let ret = rng.next();
    t.stub_cursor_step(h31::CURSOR_STEP, 2, ret);
    for k in 0..3 {
        let shader = rng.next();
        t.w32(objects + 8 * k, shader);
        let buffer = rng.next();
        t.w32(objects + 8 * k + 4, buffer);
    }
    let found: Vec<u32> = (0..6)
        .map(|_| if rng.chance(4) { 0 } else { rng.next() })
        .collect();
    t.stub_seq(h31::FIND_PARAMETER, true, 1, 1, &found);
    let sizes: Vec<u32> = (0..40)
        .map(|_| {
            let any = rng.next();
            pick(
                rng,
                &[0, 64, 512, 1024, 2048, 1, 0xffff_ffff, 0x8000_0000, any],
            )
        })
        .collect();
    t.stub_seq(h31::SHADOW_SIZE, false, 0, 0, &sizes);
}

const SCALE_BIAS_STUBS: &[(u32, K)] = &[(h31::SET_VALUE, K::This(2))];

#[rustfmt::skip]
const CASES31: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_clear_targets), ..method("shadow_clear_targets", 0x006c_e130, This::Object, 1, CLEAR_TARGETS_STUBS, |m, this, a| u(gh31::shadow_clear_targets(m, frame_ebp(), this, a[0]))) }), 0x5c, 0),
    framed(with_seh(Case { prep: Some(prep_new_mesh_instance), ..method("scene_new_mesh_instance", 0x0065_99c0, This::Object, 2, NEW_MESH_INSTANCE_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh31::scene_new_mesh_instance(m, frame_ebp(), slots, this, a[0], a[1])) }) }), 0x50, 8),
    framed(Case { prep: Some(prep_render_shadow), ..method("scene_update_shadows", 0x0065_3600, This::Object, 0, RENDER_SHADOW_STUBS, |m, this, _| u(gh31::scene_update_shadows(m, frame_ebp(), this))) }, 0x64, 0),
    framed(Case { prep: Some(prep_scale_bias), ..method("scene_set_shadow_scale_bias", 0x0065_42b0, This::Object, 0, SCALE_BIAS_STUBS, |m, this, _| { gh31::scene_set_shadow_scale_bias(m, frame_ebp(), this); Out::None }) }, 0x78, 0),
];

#[test]
fn batch31_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES31, n, &mut Rng(0x3131_b031));
    });
}

// ---- round 8 batch 32 (ffx_prender::h3_glue32) ----

/// The key (none, half or whole), a geometry found or not, the pool
/// giving data and geometry or not (then the allocations), the current
/// VFX type.
fn prep_vfx_create(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    if !t.snapshot {
        t.global(h32::VFX_RENDER_DATA_TABLE.sym, obj + 0x40);
        let kind = rng.next() & 0xff;
        let at = t.rebase(h32::VFX_CURRENT_TYPE.sym);
        t.write(at, &[kind as u8]);
    }
    args[0] = if rng.chance(3) { 0 } else { rng.below(4) };
    args[1] = if rng.chance(3) { 0 } else { rng.next() };
    args[2] = rng.next();
    let found = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h32::GET_DYNAMIC_GEOMETRY, true, 2, 2, found);
    let pooled = u32::from(rng.chance(2));
    t.stub_two_outs(h32::GEOMETRY_FROM_POOL, obj + 0x600, obj + 0x800, pooled);
    t.stub_alloc(h32::OPERATOR_NEW, obj + 0x400);
    t.stub_ret(h32::GEOMETRY_CTOR, true, 0, 0, obj + 0x900);
}

/// The text failing to start or not, the first row (before, inside,
/// at and past the 10 shown).
fn prep_map_list(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    *this = obj;
    // the on flag set before, so that clearing it shows
    let on = rng.next() as u8 | 1;
    t.write(obj, &[on]);
    let failed = if rng.chance(4) { rng.next() | 1 } else { 0 };
    t.stub_ret(h32::TEXT_INIT, true, 0, 0, failed);
    let first = pick(rng, &[0, 1, 5, 9, 10, 11, 0xffff_ffff, 0xffff_fffd]);
    t.w32(obj + 0x1ec, first);
}

#[rustfmt::skip]
const CASES32: &[Case] = &[
    // the word at ebp - 4 is the push ecx reservation: the caller's ecx
    framed(with_cookie(Case { prep: Some(prep_vfx_create), ..cdecl("vfx_create", 0x0064_4de0, 3, &[(h32::VFX_TABLE_INSERT, K::This(3))], |m, _, a| { let slots = slots_at(m, a); u(gh32::vfx_create(m, frame_ebp(), slots, a[0], a[1], a[2])) }) }), 4, 12),
    framed(with_cookie(Case { prep: Some(prep_map_list), ..method("map_list_init", 0x006b_9e20, This::Object, 0, &[(h32::TEXT_SET_ENABLED, K::This(1)), (h32::TEXT_APPEND_LINE, K::This(1)), (h32::SPRINTF_S, K::ImportC(6))], |m, this, _| u(gh32::map_list_init(m, frame_ebp(), this))) }), 0x1c, 0),
];

#[test]
fn batch32_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES32, n, &mut Rng(0x3232_b032));
    });
}

// ---- round 8 batch 33 (ffx_prender::h3_glue33) ----

/// A float for the frustum test: small values, now and then 0, a large
/// one or any (no signalling NaN).
fn frustum_float(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => 0,
        1 => rng.float().to_bits(),
        2 => (rng.below(2000) as f32 * 0.5 - 500.0).to_bits(),
        _ => ((rng.below(4000) as f32 - 2000.0) / 1000.0).to_bits(),
    }
}

/// The camera box and the caster matrix: random, or an identity with a
/// translation and a projection-like last column.
fn prep_caster_visible(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    *this = obj;
    for k in 0..7 {
        let v = frustum_float(rng);
        t.w32(obj + 4 * k, v);
    }
    let m = obj + 0x100;
    args[0] = m;
    let identity = rng.chance(2);
    for k in 0..16 {
        let v = if identity && k < 12 {
            if k % 5 == 0 {
                1f32.to_bits()
            } else if rng.chance(2) {
                0
            } else {
                frustum_float(rng)
            }
        } else {
            frustum_float(rng)
        };
        t.w32(m + 4 * k, v);
    }
    // y / w = -3.3f / 3 lies just above -1.1, but rounds to the f32
    // below it: the stored ratio decides
    if rng.chance(8) {
        for k in 0..7 {
            let v = if k == 5 { (-3.3f32).to_bits() } else { 0 };
            t.w32(obj + 4 * k, v);
        }
        for k in 0..16 {
            let v = match k {
                0 | 5 | 10 => 1f32.to_bits(),
                15 => 3f32.to_bits(),
                _ => 0,
            };
            t.w32(m + 4 * k, v);
        }
    }
}

/// The request (its name empty, short or long), the scene's eight
/// slots (used or not, one holding the id now and then; their number
/// words taken or not), the path (under `/fp/` or not), the string
/// staying inline or not, the asset last or not.
fn prep_asset_loaded33(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    let request = obj + 0x40;
    let name = obj + 0x200;
    let len = pick(rng, &[0, 3, 20]) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
    text.push(0);
    t.write(name, &text);
    let id = rng.below(4);
    let index = pick(rng, &[0, 1, 5]);
    let words = [
        scene,
        rng.next(),
        rng.next(),
        rng.next(),
        id,
        name,
        index,
        rng.next(),
    ];
    for (k, w) in words.into_iter().enumerate() {
        t.w32(request + 4 * k as u32, w);
    }
    args[0] = rng.next();
    args[1] = request;
    for k in 0..8 {
        let at = scene + 0xfccc + 0x20c * k;
        let used = if rng.chance(3) { 0 } else { 1 + rng.below(3) };
        t.w32(at, used);
        let held = if rng.chance(3) { id } else { rng.below(8) };
        t.w32(at + 4, held);
        for j in 0..2 {
            let v = if rng.chance(2) { 0 } else { rng.next() as u16 };
            t.write(at + 8 + 2 * j, &v.to_le_bytes());
        }
    }
    let path = obj + 0x300;
    let mut bytes = b"chr/fp/".to_vec();
    for _ in 0..rng.below(8) {
        bytes.push(b'0' + rng.below(10) as u8);
    }
    bytes.push(0);
    t.write(path, &bytes);
    t.stub_ret(h33::ASSET_PATH, true, 1, 1, path);
    let fp = u32::from(!rng.chance(3)) * (path + 3);
    t.stub_import_c(h33::STRSTR, 2, fp);
    let number = rng.next();
    t.stub_import_c(h33::ATOI, 1, number);
    // the string kept inline (the capacity as the original set it), or
    // grown to 16 and more
    let data = rng.next();
    let ret = rng.next();
    match rng.below(3) {
        0 => t.stub_set_this(h33::STRING_ASSIGN, 2, ret, &[(0, data)]),
        k => {
            let cap = [0x10, 0x1f][k as usize - 1];
            t.stub_set_this(h33::STRING_ASSIGN, 2, ret, &[(0x14, cap), (0, data)]);
        }
    }
    let count = if rng.chance(2) {
        index + 1
    } else {
        rng.below(8)
    };
    t.stub_ret(h33::ASSET_COUNT, true, 0, 0, count);
}

const ASSET_LOADED33_STUBS: &[(u32, K)] = &[
    (h33::ASYNC_LOAD_INSTANCE, K::Cdecl(0)),
    (h33::STRNCPY, K::ImportC(3)),
    (h33::UNIFY_FILENAME, K::Cdecl(3)),
    (h33::ASSET_ITEM, K::This(5)),
    (h33::ALL_ASSETS_LOADED, K::This(6)),
    (h33::OPERATOR_DELETE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES33: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_asset_loaded33), ..cdecl("character_asset_loaded", 0x0066_63d0, 2, ASSET_LOADED33_STUBS, |m, _, a| u(gh33::character_asset_loaded(m, frame_ebp(), a[0], a[1]))) }), 0x58, 0),
];

/// The float test, with many more cases (no allocation per case).
#[rustfmt::skip]
const CASES33_FLOAT: &[Case] = &[
    framed(Case { prep: Some(prep_caster_visible), ..method("caster_visible", 0x006c_9270, This::Object, 1, &[], |m, this, a| { let slots = slots_at(m, a); Out::U8(gh33::caster_visible(m, frame_ebp(), slots, this, a[0])) }) }, 0x60, 4),
];

#[test]
fn batch33_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES33, n, &mut Rng(0x3333_b033));
        let n = if t.snapshot { 300 } else { 3000 };
        run_cases(t, CASES33_FLOAT, n, &mut Rng(0x3333_f033));
    });
}

// ---- round 8 batch 34 (ffx_prender::h3_glue34) ----

/// An MSVC string `assign` stub writing all six words of the string
/// (the original leaves the unused ones to whatever the stack held):
/// inline or on the heap.
fn stub_assign34(t: &mut Twins, rng: &mut Rng) {
    let mut words: Vec<(u8, u32)> = (0..4).map(|k| (4 * k as u8, rng.next())).collect();
    words.push((0x10, rng.below(0x20)));
    // the capacity the original set (0xf) kept now and then
    if !rng.chance(3) {
        words.push((0x14, pick(rng, &[0x10, 0x1f])));
    }
    let ret = rng.next();
    t.stub_set_this(h34::STRING_ASSIGN, 2, ret, &words);
    // getPlatformSpecificPath's result: inline, at 16 or on the heap
    let cap = pick(rng, &[0xf, 0x10, 0x1f]);
    let data = rng.next();
    let log = t.stub(None, h34::PLATFORM_PATH, false, 7, 0, Some(0));
    // mov eax, [esp+4]; mov dword [eax+0x14], cap; mov dword [eax], data; jmp log
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x40, 0x14];
    c.extend(cap.to_le_bytes());
    c.extend([0xc7, 0x00]);
    c.extend(data.to_le_bytes());
    let at = t.cave_next;
    c.push(0xe9);
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    let code = t.code(&c);
    let site = t.rebase(h34::PLATFORM_PATH);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(site + 5).to_le_bytes());
    t.patch(site, &j);
}

/// The path, `b` (its low byte set or not), the map (0x220 or not), the
/// file there or not, `znkd12` in the path or not.
fn prep_load_async(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    // the fields the load clears, set before
    for at in [0xd8, 0xf95c, 0xf960, 0xf964, 0xf968] {
        let v = rng.next() | 1;
        t.w32(scene + at, v);
    }
    t.write(scene + 0xf9c4, &[1]);
    let path = obj + 0x200;
    let len = rng.below(12) as usize;
    let mut text: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
    text.push(0);
    t.write(path, &text);
    args[0] = path;
    args[1] = rng.word() ^ (rng.below(2) << 8);
    let map = if rng.chance(3) {
        0x220
    } else {
        rng.below(0x400)
    };
    t.w32(scene + 0xfaf4, map);
    if !t.snapshot {
        let v = rng.next() as u8;
        let at = t.rebase(h34::PHYRE_SCENE_3D.sym);
        t.write(at, &[v]);
    }
    t.stub_alloc(h34::OPERATOR_NEW_ARRAY, obj + 0x400);
    stub_assign34(t, rng);
    let exists = if rng.chance(4) { 0 } else { rng.next() | 1 };
    t.stub_ret(h34::FILE_EXISTS, false, 1, 0, exists);
    let found = if rng.chance(3) { rng.next() | 1 } else { 0 };
    t.stub_import_c(h34::STRSTR, 2, found);
}

const LOAD_ASYNC_STUBS: &[(u32, K)] = &[
    (h34::CONVERT_MAP_PATH, K::This(3)),
    (h34::OPERATOR_DELETE_ARRAY, K::Cdecl(1)),
    (h34::OPERATOR_DELETE, K::Cdecl(1)),
    (h34::ASYNC_LOAD_INSTANCE, K::Cdecl(0)),
    (h34::ASYNC_REGIST_TASK, K::Cdecl(9)),
];

/// The path, `b`, the file open or not, and the header's names: none
/// (no vector), 0 to 3 of them, inline or on the heap.
fn prep_read_header(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    args[0] = rng.next();
    args[1] = rng.word() ^ (rng.below(2) << 8);
    stub_assign34(t, rng);
    let open = if rng.chance(4) {
        rng.next() & !0xff
    } else {
        rng.next() | 1
    };
    t.stub_ret(h34::READER_IS_OPEN, true, 0, 0, open);
    let n = rng.below(4);
    let begin = if rng.chance(5) { 0 } else { obj + 0x600 };
    let end = if begin == 0 { 0 } else { begin + 0x18 * n };
    for k in 0..n {
        let at = obj + 0x600 + 0x18 * k;
        for j in 0..4 {
            let v = rng.next();
            t.w32(at + 4 * j, v);
        }
        let size = rng.below(0x20);
        t.w32(at + 0x10, size);
        let cap = pick(rng, &[0xf, 0x10, 0x1f]);
        t.w32(at + 0x14, cap);
    }
    let ret = rng.next();
    let cap = rng.next();
    t.stub_set_this(h34::READ_NAMES, 1, ret, &[(0, begin), (4, end), (8, cap)]);
}

const READ_HEADER_STUBS: &[(u32, K)] = &[
    (h34::CONVERT_MAP_PATH, K::This(3)),
    (h34::READER_CTOR, K::This(1)),
    (h34::READER_CLOSE, K::This(0)),
    (h34::READER_DTOR, K::This(0)),
    (h34::ADD_ASSET_HEADER, K::This(1)),
    (h34::OPERATOR_DELETE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES34: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_load_async), ..method("field_map_load_async", 0x0065_ce90, This::Object, 2, LOAD_ASYNC_STUBS, |m, this, a| u(gh34::field_map_load_async(m, frame_ebp(), this, a[0], a[1]))) }), 0x13c, 0),
    framed(with_seh(Case { prep: Some(prep_read_header), ..method("read_asset_header", 0x0066_7bb0, This::Object, 2, READ_HEADER_STUBS, |m, this, a| u(gh34::read_asset_header(m, frame_ebp(), this, a[0], a[1]))) }), 0x154, 0),
];

#[test]
fn batch34_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES34, n, &mut Rng(0x3434_b034));
    });
}

// ---- round 8 batch 35 (ffx_prender::h3_glue35) ----

/// The instances (none, some, one holding the id, free ones, borrowed
/// storage), the animated characters (none or some, names tagged or
/// absent, the name found or not, 0 to 3 materials), the allocations
/// failing now and then.
fn prep_register_instance(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    *this = obj;
    let id = if rng.chance(8) { 0 } else { 1 + rng.below(6) };
    let count = pick(rng, &[0, 1, 3, 4]);
    let borrowed = if rng.chance(4) { 0x8000_0000 } else { 0 };
    t.w32(obj + 0x6c, count | borrowed);
    let records = obj + 0x400;
    // no storage at all when empty now and then (nothing to free)
    let storage = if count == 0 && rng.chance(2) {
        0
    } else {
        records
    };
    t.w32(obj + 0x70, storage);
    for k in 0..4 {
        let held = match rng.below(6) {
            0 => id,
            1 | 2 => 0,
            _ => 7 + rng.below(4),
        };
        t.w32(records + 12 * k, rng.next());
        t.w32(records + 12 * k + 4, held);
        t.w32(records + 12 * k + 8, rng.next());
    }
    let n = pick(rng, &[0, 1, 3, 3, 0x8000_0002]);
    t.w32(obj + 0x64, n);
    let entries = obj + 0x600;
    t.w32(obj + 0x68, entries);
    for k in 0..3u32 {
        let at = entries + 32 * k;
        let name = obj + 0x800 + 0x20 * k;
        t.write(name, &[b'c', b'0', b'0', b'0' + k as u8, 0]);
        let tagged = match rng.below(5) {
            0 => 0,
            1 => name | 1,
            _ => name,
        };
        t.w32(at + 0x18, tagged);
        let materials = pick(rng, &[0, 1, 3, 0x8000_0002]);
        t.w32(at + 0x10, materials);
        let ids = obj + 0xa00 + 0x20 * k;
        t.w32(at + 0x14, ids);
        for j in 0..4 {
            let v = rng.next();
            t.w32(ids + 4 * j, v);
        }
    }
    let wanted = obj + 0x900;
    let pickname = rng.below(4) as u8;
    t.write(wanted, &[b'c', b'0', b'0', b'0' + pickname, 0]);
    args[0] = wanted;
    args[1] = id;
    t.stub_alloc(h35::PHYRE_MALLOC, obj + 0xc00);
    // the materials' storage (a null one faults in the original too)
    let data = obj + 0xe00;
    let grown = if rng.chance(6) { 0 } else { obj + 0x1000 };
    t.stub_seq(
        h35::PHYRE_MALLOC_ALIGNED,
        false,
        2,
        0,
        &[data, grown, grown],
    );
    let news: Vec<u32> = (0..4)
        .map(|k| {
            if rng.chance(5) {
                0
            } else {
                obj + 0x1200 + 0x20 * k
            }
        })
        .collect();
    t.stub_seq(h35::OPERATOR_NEW, false, 1, 0, &news);
}

const REGISTER_INSTANCE_STUBS: &[(u32, K)] = &[
    (h35::PHYRE_PRINTF, K::Cdecl(4)),
    (h35::SLOTS_CTOR, K::This(1)),
    (h35::PHYRE_FREE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES35: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_register_instance), ..method("register_character_instance", 0x006b_29c0, This::Object, 2, REGISTER_INSTANCE_STUBS, |m, this, a| { let slots = slots_at(m, a); gh35::register_character_instance(m, frame_ebp(), slots, this, a[0], a[1]); Out::None }) }), 0x28, 8),
];

#[test]
fn batch35_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES35, n, &mut Rng(0x3535_b035));
    });
}

// ---- round 8 batch 36 (ffx_prender::h3_glue36) ----

/// The two worlds (each there or not; their object lists), the cursor
/// runs of the four walks, the record's array (its count matching or
/// not, borrowed or not), the allocation failing now and then, the
/// light (or none) and the lighting started or not.
fn prep_lighting_objects(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    let area = t.zeros(0x800);
    let mut steps = vec![];
    for (field, world) in [(0x10d4, obj + 0x40), (0x1128, obj + 0x80)] {
        let present = !rng.chance(4);
        t.w32(scene + field, if present { world } else { 0 });
        let list = if rng.chance(3) {
            world + 0x1c
        } else {
            rng.next()
        };
        t.w32(world + 0x1c, list);
        if present {
            let runs = cursor_steps(t, area, false, rng);
            steps.extend(runs.iter().copied());
            steps.extend(runs);
        }
    }
    t.stub_cursor_script(h36::CURSOR_BEGIN, h36::CURSOR_NEXT, &steps);
    let record = obj + 0x100;
    t.w32(scene + 0xcc, record);
    let n = pick(rng, &[0, 1, 2, 4, 0x8000_0002]);
    t.w32(record + 0x9c, n);
    t.w32(record + 0xa0, obj + 0x1000);
    let light = if rng.chance(3) { 0 } else { obj + 0x200 };
    t.w32(scene + 0x4f8, light);
    for k in 0..3 {
        let v = rng.float().to_bits();
        t.w32(obj + 0x2c0 + 4 * k, v);
    }
    // new storage, none, or the old one again (not freed then)
    let data = pick(rng, &[0, obj + 0x1400, obj + 0x1400, obj + 0x1000]);
    t.stub_ret(h36::PHYRE_MALLOC_ALIGNED, false, 2, 0, data);
    let ready = rng.next();
    t.stub_ret(h36::LIGHTING_READY, true, 0, 0, ready);
}

const LIGHTING_OBJECTS_STUBS: &[(u32, K)] = &[
    (h36::ARRAY_RESIZE, K::This(1)),
    (h36::PHYRE_FREE, K::Cdecl(1)),
    (h36::LIGHTING_INSTANCE, K::Getter),
];

#[rustfmt::skip]
const CASES36: &[Case] = &[
    framed(Case { prep: Some(prep_lighting_objects), ..method("lighting_objects", 0x0065_1de0, This::Object, 0, LIGHTING_OBJECTS_STUBS, |m, this, _| u(gh36::lighting_objects(m, frame_ebp(), this))) }, 0x28, 0),
];

#[test]
fn batch36_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES36, n, &mut Rng(0x3636_b036));
    });
}

// ---- round 8 batch 37 (ffx_prender::h3_glue37) ----

/// A render target record at `at`: its texture (with a size) or none.
fn target37(t: &mut Twins, at: u32, texture: u32, rng: &mut Rng) {
    t.w32(at, texture);
    t.write(at + 4, &[u8::from(rng.chance(4))]);
    let w = rng.next();
    t.w32(texture + 0x1c, w);
    let hgt = rng.next();
    t.w32(texture + 0x20, hgt);
}

/// The manager (no settings, not ready), its switches, targets and
/// tables, the colour correction's mesh and shader, the settings'
/// floats, and every renderer call failing at one point now and then.
fn prep_post_render(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let pm = t.zeros(0x1_2400);
    *this = pm;
    let settings = if rng.chance(10) { 0 } else { obj + 0x40 };
    t.w32(pm, settings);
    // the settings' floats up to +0xa0 (no signalling NaNs: the emulator keeps them)
    for k in 0..0x28 {
        let v = rng.float().to_bits();
        t.w32(obj + 0x40 + 4 * k, v);
    }
    t.write(pm + 0x123b4, &[u8::from(!rng.chance(10))]);
    for at in [
        0x123b5, 0x123b0, 0x123b8, 0x123b3, 0x123b7, 0x123b2, 0x123d1, 0x123d9, 0x1260,
    ] {
        let v = if rng.chance(3) {
            0
        } else {
            rng.next() as u8 | 1
        };
        t.write(pm + at, &[v]);
    }
    for (k, at) in [0x8c, 0x90, 0x9c, 0xa4].into_iter().enumerate() {
        let record = obj + 0x100 + 0x10 * k as u32;
        t.w32(pm + at, record);
        target37(t, record, obj + 0x140 + 0x40 * k as u32, rng);
        if at == 0x9c {
            // the corrected target always has its texture (the viewport reads it)
            t.write(record + 4, &[0]);
        }
    }
    let table = if rng.chance(4) { 0 } else { rng.next() };
    t.w32(pm + 0x1200, table);
    let mesh = obj + 0x300;
    t.w32(pm + 0x1204, mesh);
    let materials = rng.next();
    t.w32(mesh + 0x10, materials);
    t.w32(obj + 0x3c0, obj + 0x380);
    let shader = rng.next();
    t.w32(obj + 0x380, shader);
    let buffer = rng.next();
    t.w32(obj + 0x384, buffer);
    t.stub_ret(h37::ARRAY_DATA, true, 0, 0, obj + 0x3c0);
    let found: Vec<u32> = (0..3)
        .map(|_| if rng.chance(4) { 0 } else { rng.next() })
        .collect();
    t.stub_seq(h37::FIND_PARAMETER, true, 1, 1, &found);
    let v = errors(rng);
    t.stub_seq(h37::SET_TEXTURE, true, 2, 2, &v);
    let e = if rng.chance(8) { rng.next() | 1 } else { 0 };
    t.stub_ret(h37::SET_VALUE, true, 2, 2, e);
    for (sym, n) in [
        (h37::SET_COLOR_TARGET, 2),
        (h37::SET_DEPTH_STENCIL_TARGET, 1),
        (h37::BEGIN_SCENE, 1),
        (h37::END_SCENE, 1),
        (h37::RENDER_MESH_INSTANCE, 1),
        (h37::SET_CLEAR_COLOR, 4),
    ] {
        let v = errors(rng);
        t.stub_seq(sym, true, n, n, &v);
    }
    args[0] = obj + 0x600;
    args[1] = rng.next();
    args[2] = rng.next();
}

const POST_RENDER_STUBS: &[(u32, K)] = &[
    (h37::SURFACE_CTOR, K::This(2)),
    (h37::SURFACE_DTOR, K::This(0)),
    (h37::SET_VIEWPORT, K::This(4)),
    (h37::PREPARE_GLOW_MAP, K::This(3)),
    (h37::TONE_PASS, K::This(5)),
    (h37::COPY_BUFFER, K::This(6)),
    (h37::GLOW_RENDER, K::This(5)),
];

#[rustfmt::skip]
const CASES37: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_post_render), ..method("post_process_render", 0x0069_4f10, This::Object, 3, POST_RENDER_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh37::post_process_render(m, frame_ebp(), slots, this, a[0], a[1], a[2])) }) }), 0x3c, 12),
];

#[test]
fn batch37_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES37, n, &mut Rng(0x3737_b037));
    });
}

// ---- round 9 batch 38 (ffx_prender::h3_glue38) ----

impl Twins {
    /// A cdecl import stub through `slot` (as [`Twins::stub`], logging
    /// `n` words) whose calls return `values` in turn.
    fn stub_import_seq(&mut self, slot: u32, n: u32, values: &[u32]) {
        let words: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        let log = self.stub(None, slot, false, n, 0, None);
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
        let at = self.rebase(slot);
        self.patch(at, &code.to_le_bytes());
    }

    /// A cdecl stub at `sym` logging 14 + `more` stack words (two log
    /// entries: `sym + 1` with the words past the fourteenth, then `sym`
    /// with the first fourteen); returns `ret`.
    fn stub_wide(&mut self, sym: u32, more: u32, ret: u32) {
        let first = self.stub(None, sym, false, 14, 0, Some(ret));
        let tail = self.stub(None, sym + 1, false, more, 0, Some(0));
        let mut c = vec![];
        for _ in 0..more {
            // push the last of them first: each push moves the rest down
            c.extend([0xff, 0x74, 0x24, (4 + 4 * (14 + more - 1)) as u8]); // push dword [esp+..]
        }
        let at = self.cave_next;
        c.push(0xe8); // call tail
        c.extend(tail.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        c.extend([0x83, 0xc4, (4 * more) as u8]); // add esp, 4 * more
        let at = self.cave_next;
        c.push(0xe9); // jmp first
        c.extend(first.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// The scene (random, with its map: a camera map, 0x220, a 2D-table map
/// or any), `b` (low byte 1 or not), the map number (0x163, 0x164 or
/// any), the names found or not, the header's error now and then, and 0
/// to 3 assets (or a negative count).
fn prep_map_load38(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let bytes = rng.bytes(0x1_1000);
    t.write(scene, &bytes);
    let table = t.rebase(h38::MAPS_2D);
    // the first and last entries now and then (the bounds)
    let any_k = rng.below(30);
    let k = pick(rng, &[0, 29, any_k]);
    let listed = t.r32(table + 4 * k);
    let any = rng.below(0x2000);
    let map = pick(
        rng,
        &[0x1f20, 0x220, 0xc04, 0xa6c, listed, listed, any, any],
    );
    t.w32(scene + 0xfaf4, map);
    for at in [0x10d8, 0x112c] {
        let v = if rng.chance(2) { 0 } else { rng.next() };
        t.w32(scene + at, v);
    }
    args[0] = rng.next();
    args[1] = rng.word() & !0xff | pick(rng, &[0, 1, 1, 2, 0x81]);
    let current = pick(rng, &[0x163, 0x164, any]);
    t.global(h38::CURRENT_MAP_NO.sym, current);
    let v = rng.next();
    t.global(h38::MAP_163_164.sym, v);
    let v = rng.next() as u8;
    let at = t.rebase(h38::PHYRE_SCENE_3D.sym);
    t.write(at, &[v]);
    let found: Vec<u32> = (0..4)
        .map(|_| if rng.chance(2) { 0 } else { rng.next() | 1 })
        .collect();
    t.stub_import_seq(h38::STRSTR, 2, &found);
    let error = if rng.chance(3) { rng.next() | 1 } else { 0 };
    t.stub_ret(h38::READ_ASSET_HEADER, true, 2, 2, error);
    let count = pick(rng, &[0, 1, 2, 3, 3, 0x8000_0001]);
    let first = rng.word();
    // the loop's later counts: the same, or negative (a signed compare)
    let mut counts = vec![first, count];
    for _ in 0..5 {
        let later = if rng.chance(4) {
            0x8000_0000 | rng.next()
        } else {
            count
        };
        counts.push(later);
    }
    t.stub_seq(h38::ASSET_COUNT, true, 0, 0, &counts);
    let ret = rng.next();
    t.stub_wide(h38::LOAD_CLUSTER, 5, ret);
}

const FIELD_MAP_LOAD_STUBS: &[(u32, K)] = &[
    (h38::LOAD_CAMERA_DATA, K::Cdecl(1)),
    (h38::UNLOAD_CAMERA_DATA, K::Cdecl(0)),
    (h38::ASYNC_LOAD_INSTANCE, K::Getter),
    (h38::CHILDREN_REQUEST_LIST, K::This(1)),
    (h38::ASSET_PATH, K::This(1)),
];

#[rustfmt::skip]
const CASES38: &[Case] = &[
    framed(Case { prep: Some(prep_map_load38), ..method("field_map_load", 0x0065_cbf0, This::Object, 2, FIELD_MAP_LOAD_STUBS, |m, this, a| { let slots = slots_at(m, a); u(gh38::field_map_load(m, slots, this, a[0], a[1])) }) }, 0, 8),
];

#[test]
fn batch38_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES38, n, &mut Rng(0x3838_b038));
    });
}

/// The free camera (there or not; `operator new` failing now and then),
/// the renderer (or none), 0 to 4 clusters per kind (some slots empty),
/// the arrays' counts and capacities (borrowed storage, with or without
/// data; the cluster arrays' counts stay small, a negative one would walk
/// 2^31 slots), and the tree: empty, or 1 to 4 nodes down the root's
/// chain and 0 to 4 successors, their values absent, with or without a
/// block.
fn prep_map_unload38(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_2000);
    *this = scene;
    let bytes = rng.bytes(0x1_2000);
    t.write(scene, &bytes);
    let camera = if rng.chance(2) { 0 } else { rng.next() | 1 };
    t.global(h38::FREE_CAMERA.sym, camera);
    let p = if rng.chance(4) { 0 } else { obj + 0x1f00 };
    t.stub_ret(h38::OPERATOR_NEW, false, 1, 0, p);
    let renderer = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.w32(scene + 0xd0, renderer);
    for (count, data) in [(0x10d8, obj + 0x100), (0x112c, obj + 0x140)] {
        let n = rng.below(5);
        t.w32(scene + count, n);
        t.w32(scene + count + 8, data);
        for k in 0..4 {
            let v = if rng.chance(3) { 0 } else { rng.next() };
            t.w32(data + 4 * k, v);
        }
    }
    for a in [0x1138, 0x11c8] {
        let neg = rng.chance(2);
        let n = if neg {
            rng.next() | 0x8000_0000
        } else {
            rng.word()
        };
        t.w32(scene + a, n);
        let cap = if rng.chance(2) {
            rng.next() | 0x8000_0000
        } else {
            rng.word() & 0x7fff_ffff
        };
        t.w32(scene + a + 4, cap);
        let data = if rng.chance(2) { 0 } else { rng.next() };
        t.w32(scene + a + 8, data);
    }
    let header = scene + 0x1198;
    let nodes: Vec<u32> = (0..8).map(|k| obj + 0x400 + 0x40 * k).collect();
    for &n in &nodes {
        let value = match rng.below(3) {
            0 => 0,
            _ => n + 0x20,
        };
        t.w32(n + 0x10, value);
        let block = if rng.chance(2) { 0 } else { rng.next() };
        t.w32(n + 0x28, block);
    }
    if rng.chance(4) {
        t.w32(scene + 0x11a4, header);
    } else {
        let chain = 1 + rng.below(4) as usize;
        t.w32(scene + 0x11a4, nodes[0]);
        for k in 0..chain {
            let next = if k + 1 == chain { header } else { nodes[k + 1] };
            t.w32(nodes[k], next);
        }
    }
    let m = rng.below(5) as usize;
    let mut seq: Vec<u32> = (0..m).map(|k| nodes[4 + k % 4]).collect();
    seq.extend([header; 4]);
    t.stub_seq(h38::TREE_SUCCESSOR, true, 1, 1, &seq);
}

const MAP_UNLOAD_STUBS: &[(u32, K)] = &[
    (h38::FREE_CAMERA_CTOR, K::This(0)),
    (h38::FREE_CAMERA_SET_TYPE, K::This(1)),
    (h38::SYNC_RENDER, K::This(0)),
    (h38::CLUSTER_MANAGER_INSTANCE, K::Getter),
    (h38::RELEASE_CLUSTER, K::This(1)),
    (h38::ASSET_LIST_CLEAR, K::This(0)),
    (h38::CLUSTER_BASE_CLEAR, K::This(0)),
    (h38::USER_MALLOC, K::Cdecl(1)),
    (h38::USER_FREE, K::Cdecl(1)),
    (h38::OPERATOR_DELETE, K::Cdecl(1)),
    (h38::TREE_ORPHAN_ALL, K::This(0)),
    (h38::FREE_LIST_RESET, K::This(0)),
    (h38::CHARACTER_LIGHTING_INSTANCE, K::Getter),
    (h38::CHARACTER_LIGHTING_RESET, K::This(0)),
    (h38::FOG_INSTANCE, K::Getter),
    (h38::FOG_RESET, K::This(0)),
    (h38::INSTANCE_6DC800, K::Getter),
    (h38::RESET_6DC800, K::This(0)),
];

#[rustfmt::skip]
const CASES38B: &[Case] = &[
    Case { prep: Some(prep_map_unload38), ..method("field_map_unload", 0x0065_d240, This::Object, 0, MAP_UNLOAD_STUBS, |m, this, _| u(gh38::field_map_unload(m, this))) },
];

#[test]
fn batch38b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES38B, n, &mut Rng(0x3838_b0b8));
    });
}

impl Twins {
    /// A red-black tree `successor(this; node)` stub (logged, `ret 4`)
    /// returning the node's `+0x18` (the test's own link).
    fn stub_successor38(&mut self, sym: u32) {
        let log = self.stub(None, sym, true, 1, 1, None);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x40, 0x18]); // mov eax, [eax+0x18]
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A red-black tree `remove(this; node)` stub (logged, `ret 4`) that
    /// makes the node's `+0x14` (the test's own link) the tree's root.
    fn stub_remove38(&mut self, sym: u32) {
        let log = self.stub(None, sym, true, 1, 1, Some(0));
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x40, 0x14]); // mov eax, [eax+0x14]
        c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
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

/// A test tree of `nodes` under `header`: the root is one of them, its
/// `+0` links lead down to the first node and the header, the `+0x18`
/// successors run through all of them to the header, and the `+0x14`
/// removal links from the root down to the header or 0. Each node's
/// `+0xc` block has its low bit set or not. Returns the root (the header
/// when empty).
fn tree38(t: &mut Twins, rng: &mut Rng, header: u32, nodes: &[u32]) -> u32 {
    if nodes.is_empty() {
        return header;
    }
    let c = 1 + rng.below(nodes.len() as u32) as usize;
    for (i, &n) in nodes.iter().enumerate() {
        let left = if i == 0 { header } else { nodes[i - 1] };
        t.w32(n, left);
        let next = nodes.get(i + 1).copied().unwrap_or(header);
        t.w32(n + 0x18, next);
        let removed = if i > 0 {
            nodes[i - 1]
        } else if rng.chance(2) {
            0
        } else {
            header
        };
        t.w32(n + 0x14, removed);
        let block = rng.next() & !1 | rng.below(2);
        t.w32(n + 0xc, block);
    }
    nodes[c - 1]
}

/// The table's entries (none to 3; their values absent, or with their
/// own map of 0 to 3 nodes), the copies' map (0 to 3 nodes, their name
/// pairs absent or with blocks whose low bits are set or not), `+0x64`.
fn prep_shader_table(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    table: h38::ShaderTable,
    rng: &mut Rng,
) {
    *this = obj;
    let mut next = obj + 0x200;
    let mut take = |n: u32| {
        let at = next;
        next += n;
        at
    };
    let header = obj + table.map;
    let m = rng.below(4) as usize;
    let entries: Vec<u32> = (0..m).map(|_| take(0x20)).collect();
    for &e in &entries {
        let value = if rng.chance(5) { 0 } else { take(0x40) };
        t.w32(e + 0x10, value);
        if value != 0 {
            let map = value + 4;
            let k = rng.below(4) as usize;
            let nodes: Vec<u32> = (0..k).map(|_| take(0x20)).collect();
            let root = tree38(t, rng, map, &nodes);
            t.w32(map + 0xc, root);
        }
    }
    let root = tree38(t, rng, header, &entries);
    t.w32(header + 0xc, root);
    // the copies live in the original's frame (no stack arguments)
    let copy = t.a.stack_range().end - 0x108 - 0x4c;
    let k = rng.below(4) as usize;
    let nodes: Vec<u32> = (0..k).map(|_| take(0x20)).collect();
    for &n in &nodes {
        let pair = if rng.chance(4) { 0 } else { take(0x10) };
        t.w32(n + 0x10, pair);
        if pair != 0 {
            for at in [pair, pair + 4] {
                let v = rng.next() & !1 | rng.below(2);
                t.w32(at, v);
            }
        }
    }
    let root = tree38(t, rng, copy, &nodes);
    let ret = rng.next();
    t.stub_set_this(h38::SHADER_MAP_COPY, 1, ret, &[(0xc, root)]);
    t.stub_successor38(h38::TREE_SUCCESSOR);
    t.stub_remove38(h38::TREE_REMOVE);
    let v = rng.word();
    t.w32(obj + 0x64, v);
}

fn prep_special_table(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    prep_shader_table(t, obj, this, h38::SPECIAL_TABLE, rng);
}

fn prep_magic_table(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    prep_shader_table(t, obj, this, h38::MAGIC_TABLE, rng);
}

const SHADER_TABLE_STUBS: &[(u32, K)] = &[
    (h38::FREE_LIST_RESET, K::This(0)),
    (h38::FREE_LIST_DTOR, K::This(0)),
    (h38::TREE_CLEAR, K::This(0)),
    (h38::PHYRE_FREE, K::Cdecl(1)),
    (h38::OPERATOR_DELETE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES38C: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_special_table), ..method("unload_special_shader_table", 0x006c_19b0, This::Object, 0, SHADER_TABLE_STUBS, |m, this, _| u(gh38::unload_special_shader_table(m, frame_ebp(), this, h38::SPECIAL_TABLE))) }), 0x4c, 0),
    framed(with_seh(Case { prep: Some(prep_magic_table), ..method("unload_special_shader_table_magic", 0x006c_1c50, This::Object, 0, SHADER_TABLE_STUBS, |m, this, _| u(gh38::unload_special_shader_table(m, frame_ebp(), this, h38::MAGIC_TABLE))) }), 0x4c, 0),
];

#[test]
fn batch38c_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES38C, n, &mut Rng(0x3838_b0c8));
    });
}

// ---- round 9 batch 39 (ffx_prender::h3_glue39) ----

/// Ends of `n` earlier segments of a list of `count`: rising mostly, a
/// step back now and then (an empty segment).
fn segment_ends39(rng: &mut Rng, n: u32, count: u32) -> Vec<u32> {
    let mut ends: Vec<u32> = (0..n).map(|_| rng.below(count + 1)).collect();
    if !rng.chance(4) {
        ends.sort_unstable();
    }
    ends
}

/// The sort mode (0, 1, 2 or another), the "sort" byte, 0 to 3 earlier
/// segments, the draw count (small, or at the list's room or just
/// below), the geometries (pointers into a pool or none, depths across
/// the buckets, negative, huge or NaN).
fn prep_merge39(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0d40);
    *this = scene;
    let bytes = rng.bytes(0x1_0d40);
    t.write(scene, &bytes);
    let mode = pick(rng, &[0, 1, 1, 2, 2, 3]);
    t.stub_ret(h39::SORT_MODE, true, 0, 0, mode);
    let ret = rng.next();
    t.stub_import_c(h39::QSORT, 4, ret);
    t.write(scene + h39::SORT, &[u8::from(!rng.chance(3))]);
    let n = rng.below(4);
    t.w32(scene + h39::SEGMENTS, n);
    let small = rng.below(24);
    let draws = pick(rng, &[small, small, small, 0xbf8, 0xc00]);
    let geometries = if draws == 0xc00 && rng.chance(2) {
        rng.below(6)
    } else {
        rng.below(24)
    };
    t.w32(scene + h39::DRAW_COUNT, draws);
    t.w32(scene + h39::GEOMETRY_COUNT, geometries);
    for (k, e) in segment_ends39(rng, n, draws).into_iter().enumerate() {
        t.w32(scene + h39::DRAW_ENDS + 4 * k as u32, e);
    }
    // just below the room: a first segment of 8 draws reaches it exactly
    if draws == 0xbf8 && n > 0 && rng.chance(2) {
        t.w32(scene + h39::DRAW_ENDS, 8);
    }
    for (k, e) in segment_ends39(rng, n, geometries).into_iter().enumerate() {
        t.w32(scene + h39::GEOMETRY_ENDS + 4 * k as u32, e);
    }
    let pool: Vec<u32> = (0..16).map(|k| obj + 0x400 + 0x100 * k).collect();
    for &g in &pool {
        let depth = match rng.below(5) {
            0 => rng.float().to_bits(),
            1 => (rng.below(0x1000 * 768) as f32).to_bits(),
            2 => (-(rng.below(1000) as f32)).to_bits(),
            // NaN, infinity, huge; the last two buckets
            3 => pick(
                rng,
                &[
                    0x7fc0_0000,
                    0x7f80_0000,
                    0x4f00_0000,
                    0xcf00_0001,
                    0x4a3f_ecd0,
                    0x4a3f_ecd0,
                    0x4a3f_f4a0,
                    0x4a40_0440,
                ],
            ),
            _ => (rng.below(40 * 768) as f32 / 7.0).to_bits(),
        };
        t.w32(g + 0xdc, depth);
        let draw = rng.next();
        t.w32(g + 0xc, draw);
    }
    for k in 0..geometries {
        let g = if rng.chance(6) {
            0
        } else {
            pool[rng.below(16) as usize]
        };
        t.w32(scene + h39::GEOMETRIES + 4 * k, g);
    }
}

#[rustfmt::skip]
const CASES39: &[Case] = &[
    Case { prep: Some(prep_merge39), ..method("merge_vfx_render_arrays", 0x0065_4650, This::Object, 0, &[], |m, this, _| { gh39::merge_vfx_render_arrays(m, this); Out::None }) },
];

#[test]
fn batch39_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES39, n, &mut Rng(0x3939_b039));
    });
}

// ---- round 10 batch 40 (ffx_prender::h3_glue40) ----

/// Page 0 mapped zeroed in both oracles, for paths where the original
/// reads through a null target or camera.
fn map_page_zero(t: &mut Twins) {
    t.a.map_zeroed(0, 0x1000).unwrap();
    t.b.map_zeroed(0, 0x1000).unwrap();
}

/// The flags (UI, movie fade), the fade list's meshes and their tagged
/// names (the same pointer, an equal or different string, none), the
/// target (or none: page 0), the menus (kinds 1, 0, 2, 0x101; no mesh,
/// none, a negative count), the stencil flag, the map objects (none,
/// boxes with or without a matrix), the camera (or none).
fn prep_minimap40(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let aux = t.zeros(0x2000);
    t.write(scene + 0x1_0dcc, &[u8::from(!rng.chance(5))]);
    let fading = if rng.chance(2) { 0 } else { rng.next() };
    t.global(h40::MOVIE_FADING.sym, fading);
    let renderer = obj + 0x800;
    t.w32(scene + 0xd0, renderer);
    // names: two copies of one string, a different one, a prefix, ""
    let strings: [&[u8]; 5] = [b"Minimap\0", b"Minimap\0", b"Minimal\0", b"Mini\0", b"\0"];
    let names: Vec<u32> = strings
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let at = aux + 0x1800 + 0x20 * k as u32;
            t.write(at, s);
            at
        })
        .collect();
    let (same, copy, other, prefix, empty) = (names[0], names[1], names[2], names[3], names[4]);
    let global = pick(rng, &[same, same | 1, same, 0, 1, prefix, empty]);
    t.global(h40::STENCIL_NAME.sym, global);
    let meshes = pick(rng, &[0, 1, 2, 3, 4, 5]);
    let list = obj + 0x100;
    t.w32(scene + 0x3860, meshes);
    t.w32(scene + 0x3868, list);
    for k in 0..meshes {
        let mesh = if rng.chance(6) {
            0
        } else {
            obj + 0x140 + 0x40 * k
        };
        t.w32(list + 4 * k, mesh);
        if mesh != 0 {
            let name = pick(
                rng,
                &[
                    same,
                    same | 1,
                    copy,
                    copy | 1,
                    other,
                    0,
                    1,
                    prefix,
                    empty,
                    global,
                ],
            );
            t.w32(mesh + 0x2c, name);
        }
    }
    let post = obj + 0xc00;
    t.w32(scene + 0x10cc, post);
    let target = obj + 0xd00;
    t.w32(post + 0x8c, target);
    t.w32(target, obj + 0xe00);
    let hidden = pick(rng, &[0, 0, 0, 0, 1, 2, 0x80]);
    t.write(target + 4, &[hidden as u8]);
    // the renderer's pass word at each draw
    t.deref_this = true;
    t.deref_off = 0x48;
    let v = rng.next();
    t.stub_ret(h40::RENDER_MESH_INSTANCE, true, 4, 4, v);
    t.deref_this = false;
    t.deref_off = 0;
    // a null menu's fields (page 0): a kind-1 mesh
    t.w32(0xc, obj + 0x1f00);
    t.w32(0x24, obj + 0x1e00);
    t.write(obj + 0x1e00 + 0xb4, &[1, 0]);
    // the menus
    let ui = obj + 0x180;
    t.global(h40::GRAPHIC_UI.sym, if rng.chance(6) { 0 } else { ui });
    let count = rng.below(5);
    let reported = if rng.chance(8) {
        0x8000_0000 | count
    } else {
        count
    };
    let counts: Vec<u32> = (0..8).map(|_| reported).collect();
    t.stub_seq(h40::MENU_COUNT, true, 0, 0, &counts);
    let mut menus = vec![];
    for k in 0..count {
        let menu = obj + 0x1000 + 0x40 * k;
        let mesh = if rng.chance(5) {
            0
        } else {
            obj + 0x1400 + 0x40 * k
        };
        t.w32(menu + 0xc, mesh);
        let info = obj + 0x1800 + 0x100 * k;
        t.w32(menu + 0x24, info);
        let kind = pick(rng, &[1, 1, 1, 0, 2, 0x101]);
        t.write(info + 0xb4, &(kind as u16).to_le_bytes());
        menus.push(if rng.chance(6) { 0 } else { menu });
    }
    t.stub_seq(h40::MENU_AT, true, 1, 1, &menus);
    objects40(t, scene, aux, rng);
}

/// The stencil flag, the map objects (none, boxes with or without a
/// matrix) and the camera (or none).
fn objects40(t: &mut Twins, scene: u32, aux: u32, rng: &mut Rng) {
    // the map objects
    t.write(scene + 0xf9c6, &[u8::from(rng.chance(2))]);
    let objects = aux;
    let count = pick(rng, &[0, 1, 2, 3, 4, 5]);
    t.w32(scene + 0x386c, count);
    t.w32(scene + 0x3874, objects);
    for k in 0..count {
        let entry = if rng.chance(6) {
            0
        } else {
            aux + 0x100 + 0x40 * k
        };
        t.w32(objects + 4 * k, entry);
        if entry == 0 {
            continue;
        }
        let bounds = aux + 0x400 + 0x40 * k;
        t.w32(entry + 0x1c, bounds);
        let v = rng.next();
        t.w32(entry + 0x10, v);
        let local = aux + 0x800 + 0x40 * k;
        t.w32(bounds + 0xc, if rng.chance(2) { 0 } else { local });
        for j in 0..12 {
            let f = rng.float().to_bits();
            t.w32(local + 4 * j, f);
        }
    }
    let camera = if rng.chance(6) { 0 } else { aux + 0x1000 };
    t.w32(scene + 0x654, camera);
    for j in 0..16 {
        let f = rng.float().to_bits();
        t.w32(aux + 0x1074 + 4 * j, f);
    }
}

const MINIMAP_STUBS: &[(u32, K)] = &[
    (h40::SET_CAMERA, K::This(1)),
    (h40::UPDATE_VIEW_MATRICES, K::This(0)),
    (h40::MINIMAP_X, K::This(0)),
    (h40::MINIMAP_65E4A0, K::This(0)),
    (h40::MINIMAP_Y, K::This(0)),
    (h40::MINIMAP_SIZE, K::This(0)),
    (h40::SET_VIEWPORT, K::This(4)),
    (h40::SET_SCISSOR, K::This(4)),
    (h40::BOX_IS_VISIBLE, K::This(1)),
];

#[rustfmt::skip]
const CASES40: &[Case] = &[
    framed(Case { prep: Some(prep_minimap40), ..method("minimap_stencil", 0x0065_0340, This::Object, 0, MINIMAP_STUBS, |m, this, _| u(gh40::minimap_stencil(m, frame_ebp(), this))) }, 0xc8, 0),
];

#[test]
fn batch40_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 600 };
        run_cases(t, CASES40, n, &mut Rng(0x4040_b040));
    });
}

#[rustfmt::skip]
const CASES40B: &[Case] = &[
    Case { prep: Some(prep_divider40), ..cdecl("menu_divider_shapes", 0x008e_71d0, 1, &[(h40::MKP_SHAPE_XYWHUV, K::Cdecl(9))], |m, _, a| u(gh40::menu_divider_shapes(m, a[0]))) },
];

/// The style: 0 to 8, or any.
fn prep_divider40(_: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let any = rng.next();
    args[0] = pick(rng, &[0, 1, 2, 3, 4, 5, 6, 7, 8, any]);
}

#[test]
fn batch40b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 100 };
        run_cases(t, CASES40B, n, &mut Rng(0x4040_b041));
    });
}

/// The icon: every id 599..=0x26a, or any; floats for the rectangle.
fn prep_cross_icon40(_: &mut Twins, _: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let any = rng.next();
    let id = 599 + rng.below(20);
    args[0] = if rng.chance(10) { any } else { id };
    for a in &mut args[1..5] {
        *a = rng.float().to_bits();
    }
}

#[rustfmt::skip]
const CASES40C: &[Case] = &[
    framed(Case { prep: Some(prep_cross_icon40), ..cdecl("draw_cross_button_icon", 0x008e_6070, 7, &[(h40::MKP_SHAPE_XYWHUV_C2, K::Cdecl(11))], |m, _, a| { let slots = slots_at(m, a); u(gh40::draw_cross_button_icon(m, slots, a[0], a[1], a[2], a[3], a[4], a[5], a[6])) }) }, 0, 0x1c),
];

#[test]
fn batch40c_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 200 } else { 1000 };
        run_cases(t, CASES40C, n, &mut Rng(0x4040_b042));
    });
}

// ---- round 10 batch 41 (ffx_prender::h3_glue41) ----

/// `ClassLoadedCharacterTable`'s singleton.
const LOADED_TABLE: u32 = 0x00cc_ca70;

/// A material at `q`: its parameter block (0 to 3 name/value pairs from
/// `names` and `values`) and its two passes.
fn material41(t: &mut Twins, q: u32, names: &[u32], values: &[u32], passes: &[u32], rng: &mut Rng) {
    let block = q + 0x40;
    let entries = q + 0x80;
    t.w32(q, block);
    let count = pick(rng, &[0, 1, 2, 3, 0x8000_0002]);
    t.w32(block + 4, count);
    t.w32(block + 8, entries);
    for e in 0..(count & 3) {
        let name = pick(rng, names);
        let value = pick(rng, values);
        t.w32(entries + 8 * e, name);
        t.w32(entries + 8 * e + 4, value);
    }
    let first = pick(rng, passes);
    let second = pick(rng, passes);
    t.w32(q + 8, first);
    t.w32(q + 0xc, second);
}

/// The characters' map at `aux` (head `+0x40`, nodes `+0x80` on): none,
/// a right chain of 1 to 3 nodes, or a root with two children; the
/// node count.
fn map41(t: &mut Twins, aux: u32, rng: &mut Rng) -> u32 {
    let table = aux;
    let head = aux + 0x40;
    t.w32(table, head);
    t.write(head + 0xd, &[1]);
    let node = |i: u32| aux + 0x80 + 0x40 * i;
    let shape = rng.below(5);
    let count = match shape {
        0 => 0,
        4 => 3,
        _ => shape,
    };
    if count == 0 {
        t.w32(head, head);
    } else if shape == 4 {
        // node 1 the root, 0 its left child, 2 its right one
        t.w32(head, node(0));
        t.w32(head + 4, node(1));
        t.w32(head + 8, node(2));
        t.w32(node(1), node(0));
        t.w32(node(1) + 4, head);
        t.w32(node(1) + 8, node(2));
        for i in [0, 2] {
            t.w32(node(i), head);
            t.w32(node(i) + 4, node(1));
            t.w32(node(i) + 8, head);
        }
    } else {
        t.w32(head, node(0));
        t.w32(head + 4, node(0));
        t.w32(head + 8, node(count - 1));
        for i in 0..count {
            t.w32(node(i), head);
            t.w32(node(i) + 4, if i == 0 { head } else { node(i - 1) });
            let right = if i + 1 == count { head } else { node(i + 1) };
            t.w32(node(i) + 8, right);
        }
    }
    count
}

/// The loaded characters: a synthetic table (none, a right chain of 1 to
/// 3 nodes, or a node with both children; on a snapshot the game's own
/// half the time), each character's lists (0 to 3, or a negative count),
/// their meshes, the instances' materials (inline or in an array) and
/// their pre-pass parameters (the image's names or copies, tagged or
/// not; values `""`, `"1"` or none), the passes (Transparent,
/// `FakeTransparent`, another), the list flag, map 0x180 now and then.
fn prep_zprepass41(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1000);
    *this = scene;
    t.w32(scene + 0xd0, obj + 0x800);
    t.deref_this = true;
    t.deref_off = 0x48;
    let v = rng.next();
    t.stub_ret(h41::RENDER_MESH_INSTANCE, true, 4, 4, v);
    t.deref_this = false;
    t.deref_off = 0;
    let any = rng.below(0x400);
    let map = pick(rng, &[0x180, 0x180, 0x41, any]);
    t.global(h41::CURRENT_MAP_NO.sym, map);
    if t.snapshot && rng.chance(2) {
        return;
    }
    let aux = t.zeros(0x4000);
    let strings = aux + 0x3800;
    let copies: [&[u8]; 5] = [
        b"ADDITIONAL_ZPREPASS\0",
        b"BUILT_IN_ZPREPASS\0",
        b"\0",
        b"1\0",
        b"ADDITIONAL\0",
    ];
    let mut at = strings;
    let mut copy = vec![];
    for s in copies {
        t.write(at, s);
        copy.push(at);
        at += 0x20;
    }
    let additional = t.rebase(h41::ADDITIONAL_ZPREPASS);
    let built_in = t.rebase(h41::BUILT_IN_ZPREPASS);
    let empty = t.rebase(h41::EMPTY);
    let names = [
        additional,
        additional,
        additional | 1,
        copy[0],
        built_in,
        built_in | 1,
        copy[1],
        copy[4],
        0,
    ];
    let values = [empty, empty | 1, copy[2], copy[3], copy[3] | 1, 0, 1];
    let transparent = t.rebase(h41::TRANSPARENT_PASS);
    let fake = t.rebase(h41::FAKE_TRANSPARENT_PASS);
    let other = rng.next();
    let passes = [transparent, fake, fake, other];
    for q in 0..8 {
        material41(t, aux + 0x2000 + 0x100 * q, &names, &values, &passes, rng);
    }
    let pick_material = |rng: &mut Rng| aux + 0x2000 + 0x100 * rng.below(8);
    let count = map41(t, aux, rng);
    let node = |i: u32| aux + 0x80 + 0x40 * i;
    let mut mesh_no = 0;
    for i in 0..count {
        let character = aux + 0x200 + 0x40 * i;
        t.w32(node(i) + 0x28, character);
        let lists = aux + 0x300 + 0x40 * i;
        let n = pick(rng, &[1, 1, 2, 3, 0]);
        t.w32(character + 8, lists);
        let end = if rng.chance(8) {
            lists - 4
        } else {
            lists + 4 * n
        };
        t.w32(character + 0xc, end);
        for j in 0..3 {
            let list = aux + 0x400 + 0x40 * (3 * i + j);
            t.w32(lists + 4 * j, list);
            let meshes = pick(rng, &[0, 1, 2, 3, 4, 0x8000_0002, 0x8000_0000]);
            t.w32(list, meshes);
            let array = aux + 0x700 + 0x10 * (3 * i + j);
            t.w32(list + 4, array);
            t.write(list + 0x14, &[u8::from(!rng.chance(4))]);
            for k in 0..(meshes & 7) {
                let mesh = aux + 0x800 + 0x20 * mesh_no;
                let instance = aux + 0x1000 + 0x20 * mesh_no;
                let data = aux + 0x1800 + 0x20 * mesh_no;
                mesh_no += 1;
                t.w32(array + 4 * k, mesh);
                t.w32(mesh + 0x10, instance);
                let materials = pick(rng, &[0, 1, 1, 2, 3, 0x8000_0001]);
                t.w32(instance, materials);
                if materials & 3 == 1 {
                    let q = pick_material(rng);
                    t.w32(instance + 4, q);
                } else {
                    t.w32(instance + 4, data);
                    for p in 0..(materials & 3) {
                        let q = pick_material(rng);
                        t.w32(data + 4 * p, q);
                    }
                }
            }
        }
    }
    t.global(LOADED_TABLE, aux);
}

#[rustfmt::skip]
const CASES41: &[Case] = &[
    Case { prep: Some(prep_zprepass41), ..method("draw_z_prepass", 0x0065_1070, This::Object, 0, &[], |m, this, _| u(gh41::draw_z_prepass(m, this))) },
];

#[test]
fn batch41_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 300 } else { 1000 };
        run_cases(t, CASES41, n, &mut Rng(0x4141_b041));
    });
}

/// The effect on or off, its flags, the weight (around 0.45 and 1, a
/// NaN, any), the cluster move, the camera (or none), the targets (their
/// textures hidden now and then: page 0), the scene failing to begin or
/// end now and then. `GetApplication` returns the renderer, so
/// `renderWorld`'s stub logs the pass word.
fn prep_mf152_41(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    // the post-process manager shares the scene's block (no field of one
    // overlaps a field of the other)
    let scene = t.zeros(0x1_2400);
    *this = scene;
    for (at, chance) in [(0xfaa1, 6), (0xfaa2, 2), (0x4f4, 2), (0xfb44, 2)] {
        let v = if rng.chance(chance) {
            0
        } else {
            pick(rng, &[1, 2, 0x80])
        };
        t.write(scene + at, &[v as u8]);
    }
    let any = rng.float().to_bits();
    let weight = pick(
        rng,
        &[
            0.5f32.to_bits(),
            0.46f32.to_bits(),
            0.45f32.to_bits(),
            0.44f32.to_bits(),
            0.995f32.to_bits(),
            1.0f32.to_bits(),
            1.2f32.to_bits(),
            0x7fc0_0000,
            any,
        ],
    );
    t.w32(scene + 0x4f0, weight);
    t.w32(scene + 0x394, if rng.chance(4) { 0 } else { obj + 0x300 });
    let renderer = obj + 0x800;
    t.w32(scene + 0xd0, renderer);
    let post = scene;
    t.w32(scene + 0x10cc, post);
    for (k, at) in [0xa8, 0x90, 0x8c, 0x1_232c].into_iter().enumerate() {
        let target = obj + 0xa00 + 0x20 * k as u32;
        t.w32(post + at, target);
        t.w32(target, obj + 0xc00 + 0x40 * k as u32);
        let hidden = pick(rng, &[0, 0, 0, 0, 0, 1, 2, 0x80]);
        t.write(target + 4, &[hidden as u8]);
    }
    for sym in [h41::BEGIN_SCENE, h41::END_SCENE] {
        let e = render_error(rng);
        t.stub_ret(sym, true, 1, 1, e);
    }
    t.stub_ret(h41::GET_APPLICATION, false, 0, 0, renderer);
    t.deref_this = true;
    t.deref_off = 0x48;
    t.stub_ret(h41::RENDER_WORLD, true, 3, 3, 0);
    t.deref_this = false;
    t.deref_off = 0;
    t.stub_ret(h41::MOTION_BLUR_INSTANCE, false, 0, 0, obj + 0x1000);
}

const MF152_STUBS: &[(u32, K)] = &[
    (h41::SURFACE_CTOR, K::This(2)),
    (h41::SURFACE_DTOR, K::This(0)),
    (h41::SET_COLOR_TARGET, K::This(2)),
    (h41::SET_DEPTH_STENCIL_TARGET, K::This(1)),
    (h41::SET_CAMERA, K::This(1)),
    (h41::SET_VIEWPORT, K::This(4)),
    (h41::SET_CLEAR_COLOR, K::This(4)),
    (h41::WORLD_REMOVE_CLUSTER, K::This(1)),
    (h41::WORLD_ADD_CLUSTER, K::This(1)),
    (h41::ATTACH_CHR_CLUSTER, K::This(0)),
    (h41::MOTION_BLUR_COPY, K::This(2)),
    (h41::MOTION_BLUR_SET_WEIGHT, K::This(1)),
    (h41::MOTION_BLUR_RENDER, K::This(2)),
];

#[rustfmt::skip]
const CASES41B: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_mf152_41), ..method("render_mf152", 0x0065_3230, This::Object, 0, MF152_STUBS, |m, this, _| u(gh41::render_mf152(m, frame_ebp(), this))) }), 0x30, 0),
];

#[test]
fn batch41b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 400 };
        run_cases(t, CASES41B, n, &mut Rng(0x4141_b042));
    });
}

// ---- round 10 batch 42 (ffx_prender::h3_glue42) ----

/// The variant (or none), the segment array (one entry, none, several,
/// borrowed; `PhyreMalloc` failing now and then), the segment's streams,
/// the vertex buffer (its size known or not, bound or not, the frame
/// index wrapping), the instance list (none, no instance, the ctor's
/// instance or 0: page 0), the material (or none) and its array, the
/// parameter (none, offsets below 4, sizes past the buffer).
fn prep_fade_mesh42(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    let post = t.zeros(0x1_2400);
    *this = post;
    let aux = t.zeros(0x1000);
    let variant = if rng.chance(8) { 0 } else { obj + 0x100 };
    t.stub_ret(h42::POST_LOOKUP, true, 1, 1, variant);
    let segments = aux;
    let materials = aux + 0xb00;
    t.stub_seq(h42::CLUSTER_CREATE, true, 3, 3, &[segments, materials]);
    let word = pick(rng, &[1, 1, 0, 2, 0x8000_0003]);
    t.w32(segments, word);
    let old = pick(rng, &[aux + 0x80, aux + 0x200]);
    t.w32(segments + 4, if word == 1 { aux + 0x300 } else { old });
    let entry = if rng.chance(6) { 0 } else { aux + 0x200 };
    let streams = if rng.chance(6) { 0 } else { aux + 0x280 };
    t.stub_seq(h42::PHYRE_MALLOC, false, 2, 0, &[entry, streams, streams]);
    for segment in [aux + 0x200, aux + 0x300, aux + 0x80] {
        let w = pick(rng, &[1, 0, 2]);
        t.w32(segment + 0x28, w);
        t.w32(segment + 0x2c, aux + 0x400);
    }
    let vb = aux + 0x400;
    let junk = rng.bytes(0x200);
    t.write(vb, &junk);
    let (a, b) = (rng.below(16), rng.below(16));
    t.w32(vb, a);
    t.w32(vb + 4, b);
    t.w32(vb + 0x38, if rng.chance(2) { a * b } else { rng.next() });
    t.w32(vb + 0xc, aux + 0x500);
    let index = rng.below(4);
    t.w32(vb + 0x10, index);
    let count = pick(rng, &[index + 1, index + 1, 4, 0x8000_0000 | (index + 1)]);
    t.w32(vb + 0x18, count);
    let mapped = rng.next();
    t.stub_out(h42::VB_MAP, 1, 1, mapped, 0, aux + 0x600);
    let unmapped = u32::from(rng.chance(3));
    t.stub_ret(h42::VB_UNMAP, true, 2, 2, unmapped);
    let list = if rng.chance(6) { 0 } else { aux + 0x700 };
    t.stub_ret(h42::ALLOCATE_INSTANCE_LIST, true, 3, 3, list);
    t.w32(
        aux + 0x700 + 0x34,
        if rng.chance(6) { 0 } else { aux + 0x780 },
    );
    let instance = if rng.chance(8) { 0 } else { aux + 0x800 };
    t.stub_ret(h42::MESH_INSTANCE_CTOR, true, 1, 1, instance);
    let material = if rng.chance(8) { 0 } else { aux + 0x900 };
    t.stub_ret(h42::CREATE_MATERIAL, true, 3, 3, material);
    let shader = rng.next();
    t.w32(aux + 0x900, shader);
    t.w32(aux + 0x904, aux + 0xa00);
    let size = pick(rng, &[0x10, 0x40, 0x100]);
    t.w32(aux + 0xa00, size);
    let word = pick(rng, &[1, 1, 0, 2, 0x8000_0002]);
    t.w32(materials, word);
    let old = pick(rng, &[aux + 0xb80, materials + 4]);
    t.w32(materials + 4, old);
    let parameter = if rng.chance(6) { 0 } else { aux + 0xc00 };
    t.stub_ret(h42::FIND_PARAMETER, true, 1, 1, parameter);
    let offset = pick(rng, &[0, 2, 4, 8, 0x30]);
    t.write(aux + 0xc08, &(offset as u16).to_le_bytes());
    let bytes = pick(rng, &[0x10, 0x2010, 0xc, 0xe000]);
    t.write(aux + 0xc0a, &(bytes as u16).to_le_bytes());
    t.stub_copy(h42::MEMCPY);
}

const FADE_MESH_STUBS: &[(u32, K)] = &[
    (h42::ENTRIES_CONSTRUCT, K::Cdecl(2)),
    (h42::ENTRIES_DESTROY, K::Cdecl(2)),
    (h42::STREAMS_CTOR, K::This(0)),
    (h42::SET_BUFFER40, K::This(2)),
    (h42::VB_SETUP, K::This(3)),
    (h42::VB_IS_BOUND, K::This(0)),
    (h42::VB_RELEASE, K::This(0)),
    (h42::PHYRE_FREE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES42: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_fade_mesh42), ..method("post_screen_fade_mesh", 0x0069_31a0, This::Object, 0, FADE_MESH_STUBS, |m, this, _| u(gh42::post_screen_fade_mesh(m, frame_ebp(), this))) }), 0x40, 0),
];

#[test]
fn batch42_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 400 };
        run_cases(t, CASES42, n, &mut Rng(0x4242_b042));
    });
}

impl Twins {
    /// A recording stub at `sym` (thiscall, `n` words, popping them,
    /// returning `ret`) that logs the word argument `arg` points to in
    /// place of that argument.
    fn stub_deref_arg(&mut self, sym: u32, n: u32, ret: u32, arg: u8) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        let slot = 4 + 4 * arg;
        // mov eax, [esp+slot]; mov eax, [eax]; mov [esp+slot], eax; jmp log
        let mut c = vec![0x8b, 0x44, 0x24, slot, 0x8b, 0x00, 0x89, 0x44, 0x24, slot];
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

/// The shadow manager's 64 slots (a list or none, in the map or not,
/// visible or not, sizes around the limit or a NaN, the filter's word),
/// the filter (on or off, its `al`), six lists of 0 to 3 casters (a
/// negative count too) with or without a 3x4, the two matrices, the
/// output arrays (empty, owned or borrowed storage), the kind (4 or
/// another) and `single` (its low byte counts).
fn prep_casters42(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let shadows = t.zeros(0x4100);
    *this = shadows;
    let aux = t.zeros(0x4000);
    let limit = t.r32(t.rebase(h42::CASTER_SIZE_LIMIT));
    let lim = f32::from_bits(limit);
    for i in 0..64 {
        let list = if rng.chance(5) {
            0
        } else {
            aux + 0x100 * rng.below(6)
        };
        t.w32(shadows + 0xde8 + 4 * i, list);
        t.w32(shadows + 0xfe8 + 4 * i, pick(rng, &[1, 1, 1, 0, 2]));
        t.w32(shadows + 0x10e8 + 4 * i, pick(rng, &[1, 1, 1, 0, 0x101]));
        let v = rng.next();
        t.w32(shadows + 0x2ea8 + 4 * i, v);
        for at in [0x6b8, 0x7b8] {
            let any = rng.float().to_bits();
            let size = pick(
                rng,
                &[
                    limit,
                    (lim * 2.0).to_bits(),
                    (lim * 0.5).to_bits(),
                    0x7fc0_0000,
                    any,
                ],
            );
            t.w32(shadows + at + 4 * i, size);
        }
    }
    let high = rng.next() as u8;
    t.write(shadows + 0x4030, &[u8::from(rng.chance(3)), high, 0, high]);
    let any = rng.next();
    let ret = pick(rng, &[0x100, 0xff00, 1, 0, any]);
    let filter = t.stub(None, 0x4048, false, 1, 0, Some(ret));
    t.w32(shadows + 0x4048, filter);
    for l in 0..6 {
        let list = aux + 0x100 * l;
        let n = pick(rng, &[0, 1, 2, 3, 3, 0x8000_0002]);
        t.w32(list, n);
        let items = aux + 0x800 + 0x20 * l;
        t.w32(list + 4, items);
        for j in 0..3 {
            let item = aux + 0x1000 + 0x40 * (3 * l + j);
            t.w32(items + 4 * j, item);
            let bounds = aux + 0x2000 + 0x40 * (3 * l + j);
            t.w32(item + 0x1c, bounds);
            let local = aux + 0x3000 + 0x40 * ((l + j) % 8);
            t.w32(bounds + 0xc, if rng.chance(2) { 0 } else { local });
        }
    }
    for k in 0..(8 * 12) {
        let f = rng.float().to_bits();
        t.w32(aux + 0x3000 + 4 * k, f);
    }
    let camera = aux + 0x3800;
    for k in 0..32 {
        let f = rng.float().to_bits();
        t.w32(camera + 4 * k, f);
    }
    let out = aux + 0x3c00;
    for array in [out, out + 8] {
        let word = pick(rng, &[0, 1, 3, 0x8000_0001]);
        t.w32(array, word);
        t.w32(array + 4, if rng.chance(4) { 0 } else { aux + 0x3d00 });
    }
    let tests: Vec<u32> = (0..400)
        .map(|_| {
            if rng.chance(4) {
                rng.next() & !0xff
            } else {
                rng.next() | 1
            }
        })
        .collect();
    t.stub_seq(h42::BOX_IS_VISIBLE, true, 1, 1, &tests);
    let tests: Vec<u32> = (0..400)
        .map(|_| {
            if rng.chance(4) {
                rng.next() & !0xff
            } else {
                rng.next() | 1
            }
        })
        .collect();
    t.stub_seq(h42::BOX_IS_INSIDE, true, 1, 1, &tests);
    let v = rng.next();
    t.stub_deref_arg(h42::PUSH_INTO, 2, v, 1);
    args[0] = pick(rng, &[4, 4, 4, 0, 1, 5]);
    args[1] = out;
    args[2] = camera;
    args[3] = pick(rng, &[0, 0, 1, 0x100]);
}

#[rustfmt::skip]
const CASES42B: &[Case] = &[
    framed(Case { prep: Some(prep_casters42), ..method("collect_casters", 0x006c_8090, This::Object, 4, &[(h42::PHYRE_FREE, K::Cdecl(1)), (h42::PHYRE_MALLOC, K::Cdecl(2))], |m, this, a| { slots_at(m, a); u(gh42::collect_casters(m, frame_ebp(), this, a[0], a[1], a[2], a[3])) }) }, 0xf4, 0x10),
];

#[test]
fn batch42b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 300 };
        run_cases(t, CASES42B, n, &mut Rng(0x4242_b043));
    });
}

// ---- round 11 batch 43 (ffx_prender::h3_glue43) ----

/// The ebp of a framed case's original with `args` stack words (as
/// `run_cases` computes it).
fn case_ebp(t: &Twins, args: u32) -> u32 {
    t.a.stack_range().end - 0x100 - 4 * args - 8
}

/// The application getter returns `renderer`, and `renderWorld`'s stub
/// logs the renderer's technique word (`+0x48`) in place of `ecx`.
fn stub_render_world43(t: &mut Twins, renderer: u32, rng: &mut Rng) {
    t.stub_ret(h43::GET_APPLICATION, false, 0, 0, renderer);
    let at = t.rebase(h43::RENDER_WORLD);
    let ret = rng.next();
    t.deref_this = true;
    t.deref_off = 0x48;
    t.stub(Some(at), h43::RENDER_WORLD, true, 3, 3, Some(ret));
    t.deref_this = false;
    t.deref_off = 0;
}

/// The scene's camera (or none: page 0) with a random matrix, the next
/// camera (or none), the renderer, the translation and squeeze inputs,
/// the two flags, the map (the four plain-transparent ones or others);
/// the row products returned are random rows, or rows overlapping the
/// matrix itself (the frame's fill may hold signalling NaNs).
fn prep_prerender43(t: &mut Twins, _: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let aux = t.zeros(0x1000);
    for k in 0..0x400 {
        let f = rng.float().to_bits();
        t.w32(aux + 4 * k, f);
    }
    let cam = if rng.chance(10) { 0 } else { aux + 0x100 };
    t.w32(scene + 0x7e0, cam);
    let next = if rng.chance(3) { 0 } else { aux + 0x200 };
    t.w32(scene + 0xdc, next);
    t.w32(scene + 0xd0, aux + 0x300);
    stub_render_world43(t, aux + 0x300, rng);
    for at in [0xfa34, 0xfa38, 0xf9bc] {
        let f = rng.float().to_bits();
        t.w32(scene + at, f);
    }
    let flag = u8::from(rng.chance(3));
    t.write(scene + 0xf9c7, &[flag]);
    let s3d = u8::from(rng.chance(3));
    let at = t.rebase(h43::SCENE_3D);
    t.patch(at, &[s3d]);
    let any = rng.next();
    let map = pick(rng, &[0xa0, 0xa1, 0x101, 0x16c, 0x9f, 0x16d, any]);
    let at = t.rebase(h43::CURRENT_MAP);
    t.patch(at, &map.to_le_bytes());
    let ebp = case_ebp(t, 0);
    let rows: Vec<u32> = (0..8)
        .map(|_| match rng.below(4) {
            0 => ebp - 0x44 + 4 * rng.below(12),
            _ => aux + 0x400 + 4 * rng.below(0x100),
        })
        .collect();
    t.stub_seq(h43::ROW_TIMES, true, 2, 2, &rows);
}

#[rustfmt::skip]
const CASES43: &[Case] = &[
    framed(Case { prep: Some(prep_prerender43), ..method("draw_non_ui_prerender_scene", 0x0065_07d0, This::Object, 0, &[(h43::SET_CAMERA, K::This(1)), (h43::UPDATE_VIEW_MATRICES, K::This(0)), (h43::MATRIX_COPY, K::This(1))], |m, this, _| u(gh43::draw_non_ui_prerender_scene(m, frame_ebp(), this))) }, 0x108, 0),
];

#[test]
fn batch43_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 400 };
        run_cases(t, CASES43, n, &mut Rng(0x4343_b043));
    });
}

/// The 3D world pass: the cameras `+0xdc` and `+0x238` (or none: page
/// 0), the renderer's viewport, the flags (`+0xf9c7`, 3D, `+0xd4`), the
/// split-screen word, the overlay mode, MSAA, `pre` (its low byte
/// counts); the view returned is random floats (`PMatrix4_Copy` runs).
fn prep_world43(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let post = t.zeros(0x1_2400);
    let aux = t.zeros(0x1000);
    for k in 0..0x400 {
        let f = rng.float().to_bits();
        t.w32(aux + 4 * k, f);
    }
    for (field, cam) in [(0xdc, aux + 0x100), (0x238, aux + 0x200)] {
        let v = if rng.chance(8) { 0 } else { cam };
        t.w32(scene + field, v);
    }
    let renderer = aux + 0x300;
    t.w32(scene + 0xd0, renderer);
    stub_render_world43(t, renderer, rng);
    for k in 0..4 {
        let v = rng.next();
        t.w32(renderer + 8 + 4 * k, v);
    }
    t.w32(scene + 0x10cc, post);
    t.write(post + 0x1_22b4, &[u8::from(rng.chance(2))]);
    t.write(scene + 0xf9c7, &[u8::from(rng.chance(3))]);
    t.write(scene + 0xd4, &[u8::from(rng.chance(2))]);
    let at = t.rebase(h43::SCENE_3D);
    let s3d = u8::from(rng.chance(3));
    t.patch(at, &[s3d]);
    let split = pick(rng, &[1, 1, 0, 2, 0x101]);
    t.w32(scene + 0x1_0d8c, split);
    let mode = pick(rng, &[0, 1, 2, 3, 0x101]);
    t.w32(scene + 0xf9cc, mode);
    let view = aux + 0x400 + 4 * rng.below(0x100);
    t.stub_ret(h43::SCREEN_ADJUSTED_VIEW, true, 1, 1, view);
    args[0] = pick(rng, &[0, 1, 0x100, 0xffff_ff00]);
}

#[rustfmt::skip]
const CASES43B: &[Case] = &[
    framed(Case { prep: Some(prep_world43), ..method("draw_3d_world", 0x0064_fbb0, This::Object, 1, &[(h43::SET_CAMERA, K::This(1)), (h43::UPDATE_VIEW_MATRICES, K::This(0)), (h43::SET_VIEWPORT, K::This(4)), (h43::DRAW_MESH_MATRICES, K::This(1)), (h43::DRAW_Z_PREPASS, K::This(0))], |m, this, a| { m.write(frame_ebp() + 8, a[0]); u(gh43::draw_3d_world(m, frame_ebp(), this, a[0])) }) }, 0x8c, 4),
];

#[test]
fn batch43b_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 150 } else { 400 };
        run_cases(t, CASES43B, n, &mut Rng(0x4343_b044));
    });
}

/// The targets' items, the character's meshes and value arrays, the
/// target slots and their parallel arrays (see [`prep_anim_char43`]).
fn anim_char_targets43(t: &mut Twins, aux: u32, chr: u32, rng: &mut Rng) {
    for i in 0..3 {
        let item = aux + 0x800 + 0x80 * i;
        t.w32(aux + 0x680 + 4 * i, item);
        t.w32(item + 0x74, pick(rng, &[0, 1, 0xffff_ffff]));
        t.w32(item + 0x78, pick(rng, &[0, 1, 0x8000_0000]));
    }
    // the character's meshes and their value arrays
    t.w32(chr + 4, aux + 0xa00);
    for m in 0..2 {
        let mesh = aux + 0xa40 + 0x40 * m;
        t.w32(aux + 0xa00 + 4 * m, mesh);
        let arr = aux + 0xb00 + 0x20 * m;
        t.w32(mesh + 0x10, arr);
        t.w32(arr, pick(rng, &[1, 2, 2, 0x8000_0002]));
        t.w32(arr + 4, aux + 0xb80);
        let v = if rng.chance(4) { 0 } else { rng.next() };
        t.w32(arr + 8, v);
    }
    // the target slots: entries (mesh/material pairs) and their arrays
    for s in 0..2 {
        let slots = aux + 0x380 + 8 * s;
        t.w32(slots, pick(rng, &[0, 1, 2, 0x8000_0002]));
        t.w32(slots + 4, aux + 0xc00);
    }
    for e in 0..2 {
        let entry = aux + 0xc40 + 0x80 * e;
        t.w32(aux + 0xc00 + 8 * e, entry);
        t.w32(entry + 0x74, rng.below(2));
        t.w32(entry + 0x78, rng.below(2));
        let pa = aux + 0xd40 + 0x20 * e;
        t.w32(aux + 0xc00 + 8 * e + 4, pa);
        let begin = aux + 0xe00 + 0x100 * e;
        t.w32(pa, begin);
        let used = rng.below(4);
        t.w32(pa + 8, begin + 4 * used);
        t.w32(pa + 0xc, aux + 0x1000 + 0x100 * e);
        t.w32(pa + 0x10, pick(rng, &[used, used + 1, 0, 0xffff_ffff]));
    }
}

/// Texture-animation characters: the tweak flag (log or not), the skip
/// list (0 to 3 entries, negative, the character in it or not), the
/// records (the character found or not, a high count bit), the
/// animations (tagged names: the same pointer, an equal or different
/// string, none), the targets (negative indices, a character mesh
/// array with 0, 1 or 2 values), `FindMaterial`'s results, the target
/// slots (matching or not), the frame lists and the parallel arrays
/// (full or not).
fn prep_anim_char43(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    let mgr = t.zeros(0x100);
    *this = mgr;
    let aux = t.zeros(0x4000);
    let junk = rng.bytes(0x4000);
    t.write(aux, &junk);
    let tweak = aux;
    t.write(tweak + 0x11, &[pick(rng, &[0, 1, 1, 2]) as u8]);
    t.stub_ret(h43::TWEAK_UTIL, false, 0, 0, tweak);
    let chr = aux + 0x100;
    let skip = t.rebase(h43::TEX_ANIM_SKIP);
    let n = pick(rng, &[0, 0, 1, 3, 0xffff_ffff]);
    let mut words = vec![n];
    for _ in 0..3 {
        let v = if rng.chance(6) { chr } else { rng.next() };
        words.push(v);
    }
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    t.patch(skip, &bytes);
    // names: "a12", "a12" (another copy), "b7"
    let names = [aux + 0x40, aux + 0x48, aux + 0x50];
    t.write(names[0], b"a12\0");
    t.write(names[1], b"a12\0");
    t.write(names[2], b"b7\0");
    let parts = [aux + 0x58, aux + 0x60, aux + 0x68];
    t.write(parts[0], b"p\0");
    t.write(parts[1], b"p\0");
    t.write(parts[2], b"q\0");
    t.write(aux + 0x70, b"\0");
    // records
    let count = pick(rng, &[0, 1, 2, 3, 0x8000_0002]);
    t.w32(mgr + 0x6c, count);
    let list = aux + 0x200;
    t.w32(mgr + 0x70, list);
    let set_table = aux + 0x300;
    t.w32(mgr + 0x68, set_table);
    for k in 0..3 {
        let rec = list + 12 * k;
        t.w32(rec, rng.below(2));
        let c = if rng.chance(2) { chr } else { rng.next() };
        t.w32(rec + 4, c);
        t.w32(rec + 8, aux + 0x380 + 8 * rng.below(2));
    }
    for s in 0..2 {
        let set = set_table + 0x20 * s;
        t.w32(set + 8, pick(rng, &[0, 1, 2, 3, 0x8000_0003]));
        t.w32(set + 0xc, aux + 0x400);
    }
    for a in 0..3 {
        let anim = aux + 0x400 + 0x40 * a;
        let nm = pick(rng, &[names[0], names[1], names[2], 0, names[0] | 1]);
        t.w32(anim + 0x20, nm);
        let pt = pick(rng, &[parts[0], parts[1], parts[2], 0, parts[0] | 1]);
        t.w32(anim + 4, pt);
        for o in [0x24, 0x28, 0x2c, 0x30] {
            let f = rng.float().to_bits();
            t.w32(anim + o, f);
        }
        let targets = aux + 0x600 + 0x20 * a;
        t.w32(anim + 0x38, targets);
        t.w32(targets, pick(rng, &[0, 1, 2, 3, 0x8000_0001]));
        t.w32(targets + 4, aux + 0x680);
        let frames = aux + 0x6c0 + 0x10 * a;
        t.w32(anim + 0x3c, frames);
        t.w32(frames, pick(rng, &[0, 1, 2, 0x8000_0001]));
        t.w32(frames + 4, aux + 0x700);
    }
    anim_char_targets43(t, aux, chr, rng);
    let found: Vec<u32> = (0..20)
        .map(|_| if rng.chance(4) { 0 } else { rng.next() })
        .collect();
    t.stub_seq(h43::FIND_MATERIAL, true, 3, 3, &found);
    args[0] = pick(rng, &[names[0], names[1], names[2], 0, aux + 0x70]);
    args[1] = pick(rng, &[parts[0], parts[1], parts[2], 0, aux + 0x70]);
    args[7] = rng.next();
    args[8] = chr;
}

#[rustfmt::skip]
const CASES43C: &[Case] = &[
    Case { prep: Some(prep_anim_char43), ..method("set_animated_materials_char", 0x006b_3060, This::Object, 9, &[(h43::PHYRE_PRINTF, K::Cdecl(7)), (h43::ERASE, K::This(1)), (h43::UPDATE_TARGET, K::This(2)), (h43::ATOI_SLOT, K::ImportC(1))], |m, this, a| { gh43::set_animated_materials_char(m, this, a[0], a[1], a[2], a[7], a[8]); Out::None }) },
];

#[test]
fn batch43c_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 300 } else { 1000 };
        run_cases(t, CASES43C, n, &mut Rng(0x4343_b045));
    });
}

impl Twins {
    /// A recording stub at `sym` (thiscall, `n` words popped, returning
    /// `ret`) that first appends the `words` words its argument `arg`
    /// points to into the trace at `*cursor` (so what a callee reads
    /// through a frame pointer at the time of the call is compared).
    fn stub_trace_arg(&mut self, sym: u32, n: u32, ret: u32, arg: u8, words: u32, cursor: u32) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        // push esi; push edi; push ecx; mov esi, [esp+16+4*arg]
        let mut c = vec![0x56, 0x57, 0x51, 0x8b, 0x74, 0x24, 16 + 4 * arg];
        c.extend([0x8b, 0x3d]);
        c.extend(cursor.to_le_bytes()); // mov edi, [cursor]
        c.push(0xb9);
        c.extend(words.to_le_bytes()); // mov ecx, words
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x89, 0x3d]);
        c.extend(cursor.to_le_bytes()); // mov [cursor], edi
        c.extend([0x59, 0x5f, 0x5e]); // pop ecx; pop edi; pop esi
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

/// The target update: the flags `+0x70` / `+0x71`, the parameters
/// (each set or 0), the target's frame numbers (0 to 7 bytes' worth,
/// unaligned, negative) and entries (random floats); the callees trace
/// the record, the weight and the pairs they are handed.
fn prep_update_target43(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    map_page_zero(t);
    *this = t.zeros(0x100);
    let aux = t.zeros(0x2000);
    for k in 0..0x800 {
        let f = rng.float().to_bits();
        t.w32(aux + 4 * k, f);
    }
    let value = aux;
    t.w32(value + 4, rng.next());
    let slot = aux + 0x10;
    let params = aux + 0x100;
    t.w32(slot, params);
    for k in 0..0x1a {
        let v = if rng.chance(4) { 0 } else { rng.next() | 1 };
        t.w32(params + 4 * k, v);
    }
    t.write(
        params + 0x70,
        &[u8::from(rng.chance(5)), u8::from(rng.chance(2))],
    );
    let target = aux + 0x200;
    t.w32(slot + 4, target);
    let begin = aux + 0x300;
    t.w32(target, begin);
    let bytes = pick(
        rng,
        &[
            0,
            4,
            8,
            0xc,
            0x10,
            0x14,
            0x18,
            0x1c,
            0x20,
            0x28,
            0x13,
            0xffff_fffc,
        ],
    );
    t.w32(target + 8, begin.wrapping_add(bytes));
    t.w32(target + 0xc, aux + 0x800 + 4 * rng.below(0x40));
    let trace = t.zeros(0x1000);
    t.w32(trace, trace + 4);
    let r = rng.next();
    t.stub_trace_arg(h43::SET_TEXTURE, 2, r, 1, 4, trace);
    let r = rng.next();
    t.stub_trace_arg(h43::SET_VALUE2, 2, r, 1, 1, trace);
    let r = rng.next();
    t.stub_trace_arg(h43::COPY_PAIR, 2, r, 1, 2, trace);
    args[0] = value;
    args[1] = slot;
}

#[rustfmt::skip]
const CASES43E: &[Case] = &[
    framed(Case { prep: Some(prep_update_target43), ..method("update_target", 0x006b_3660, This::Object, 2, &[], |m, this, a| { let slots = frame_ebp() + 8; for (k, &w) in a.iter().enumerate() { m.write(slots + 4 * k as u32, w); } gh43::update_target(m, frame_ebp(), this, a[0], a[1]); Out::None }) }, 0x28, 8),
];

#[test]
fn batch43e_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 300 } else { 1000 };
        run_cases(t, CASES43E, n, &mut Rng(0x4343_b046));
    });
}

// ---- round 12 batch 45 (ffx_prender::h3_glue44) ----

impl Twins {
    /// A recording stub of the thiscall at `sym` taking (and popping) `n`
    /// words (as [`Twins::stub`]) that first stores `value` at the
    /// absolute address `at`.
    fn stub_store(&mut self, sym: u32, n: u32, ret: u32, at: u32, value: u32) {
        let log = self.stub(None, sym, true, n, n, Some(ret));
        let mut c = vec![0xc7, 0x05]; // mov dword [at], value
        c.extend(at.to_le_bytes());
        c.extend(value.to_le_bytes());
        let here = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(here + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// A polling answer: `zeros` times 0 (with random upper bytes), then
/// non-zero `al` for good.
fn poll_seq(rng: &mut Rng, zeros: u32) -> Vec<u32> {
    (0..zeros + 40)
        .map(|i| {
            let high = rng.next() & 0xffff_ff00;
            if i < zeros { high } else { high | 1 }
        })
        .collect()
}

/// The boot set-up: the trophy manager (none, an error, installed or
/// after a few frames), the requests, the opening (0 to 3 frames), the
/// command line (no argument, another one, `_ECalm`), the quit flag, the
/// worker (done at once or later) and the status it reports (1 or not).
fn prep_init_scene44(t: &mut Twins, _: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let aux = t.zeros(0x1000);
    let app = aux;
    t.stub_ret(h44::GET_APPLICATION, false, 0, 0, app);
    for o in [0x26c, 0x270] {
        let v = rng.next();
        t.w32(app + o, v);
    }
    t.w32(app + 0x3a8, pick(rng, &[0, 1, 2, 3, 0xffff_ffff]));
    t.w32(app + 0x3a4, aux + 0x400);
    t.w32(aux + 0x404, pick(rng, &[aux + 0x500, aux + 0x510]));
    t.write(aux + 0x500, b"_ECalm\0");
    t.write(aux + 0x510, b"_ECal\0");
    let trophy = if rng.chance(3) { 0 } else { aux + 0x600 };
    t.stub_ret(h44::TROPHY_INSTANCE, false, 0, 0, trophy);
    let errors: Vec<u32> = (0..200)
        .map(|_| if rng.chance(8) { rng.next() | 1 } else { 0 })
        .collect();
    t.stub_seq(h44::TROPHY_ERROR, false, 0, 0, &errors);
    let z = rng.below(4);
    let seq = poll_seq(rng, z);
    t.stub_seq(h44::TROPHY_INSTALLED, true, 0, 0, &seq);
    let z = rng.below(5);
    let seq = poll_seq(rng, z);
    t.stub_seq(h44::ALL_REQUESTS_DONE, true, 0, 0, &seq);
    // the opening plays while non-zero
    let frames = rng.below(4);
    let playing: Vec<u32> = (0..frames + 40)
        .map(|i| {
            let high = rng.next() & 0xffff_ff00;
            if i < frames { high | 1 } else { high }
        })
        .collect();
    t.stub_seq(h44::OPENING_PLAYING, true, 0, 0, &playing);
    let z = rng.below(4);
    let seq = poll_seq(rng, z);
    t.stub_seq(h44::WORKER_DONE, true, 1, 1, &seq);
    let quit = t.rebase(h44::QUIT);
    let q = u8::from(rng.chance(4));
    t.patch(quit, &[q]);
    let status = pick(rng, &[1, 1, 0, 2]);
    let ebp = case_ebp(t, 0);
    let r = rng.next();
    t.stub_store(h44::WORKER_RUN, 5, r, ebp - 4, status);
}

#[rustfmt::skip]
const CASES44: &[Case] = &[
    framed(Case { prep: Some(prep_init_scene44), ..cdecl("graphic_init_scene", 0x0064_16c0, 0, &[(h44::TROPHY_CHECK, K::This(0)), (h44::ASYNC_MANAGER, K::Cdecl(0)), (h44::REGIST_ASYNC_TASK, K::Cdecl(3)), (h44::INIT_IGGY_HELPER, K::Cdecl(0)), (h44::IGGY_MENU_CREATE, K::Cdecl(0)), (h44::IGGY_MENU_SET_SIZE, K::This(3)), (h44::SET_LOADING_PARAMETER, K::This(3)), (h44::RENDER_LOADING_SCREEN, K::This(0)), (h44::BEGIN_OPENING, K::This(0)), (h44::RENDER_OPENING, K::This(0)), (h44::DELETE_IGGY_MENU, K::This(0)), (h44::INIT_SCENE, K::This(0)), (h44::INPUT_UPDATE, K::Cdecl(0))], |m, _, _| u(gh44::graphic_init_scene(m, frame_ebp()))) }, 0xc, 0),
];

#[test]
fn batch45_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 200 } else { 600 };
        run_cases(t, CASES44, n, &mut Rng(0x4444_b045));
    });
}

// ---- round 12 batch 46 (ffx_prender::h3_glue45) ----

impl Twins {
    /// A recording stub only in the cave (tagged `tag`, as [`Twins::stub`],
    /// `eax` kept) whose calls return `values` in turn; its address.
    fn seq_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, values: &[u32]) -> u32 {
        let words: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let table = self.alloc(&words);
        let ptr = self.alloc(&table.to_le_bytes());
        let log = self.stub(None, tag, this, n, pops, None);
        let mut c = vec![0x8b, 0x15];
        c.extend(ptr.to_le_bytes()); // mov edx, [ptr]
        c.extend([0x8b, 0x02]); // mov eax, [edx]
        c.extend([0x83, 0xc2, 0x04]); // add edx, 4
        c.extend([0x89, 0x15]);
        c.extend(ptr.to_le_bytes()); // mov [ptr], edx
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        self.code(&c)
    }
}

/// The console thread `0x006bd1e0` (an endless loop), N passes: its
/// argument (never read by the original) is the pass counter. The
/// `SleepThreadInMillisecond` stand-in counts an argument other than 5
/// in the counter's next word, counts the passes down and, once spent,
/// leaves the original's frame (`add esp, 8; pop edi; pop esi; mov
/// esp, ebp; pop ebp; ret`) when called from one of the loop's two
/// sleeps (their return addresses); the port side runs passes until the
/// count is spent. The line is empty, the prefix or another word;
/// `strstr` answers the buffer, another address or 0; the instance's
/// `+0x129` is set or not.
fn prep_input_thread45(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let count = t.zeros(8);
    t.w32(count, 1 + rng.below(4));
    args[0] = count;
    let buffer = t.rebase(h45::BUFFER);
    let lines: [&[u8]; 4] = [b"\0", b"EnableAutoTest 1\0", b"Go\0", b"x\0"];
    let line = lines[rng.below(4) as usize];
    t.write(buffer, line);
    let instance = t.rebase(h45::INSTANCE);
    t.w32(instance, obj);
    t.write(obj + 0x129, &[u8::from(rng.chance(2))]);
    let cin_slot = t.rebase(h45::CIN_SLOT);
    t.w32(cin_slot, obj + 0x400);
    let r = rng.next();
    let getline = t.stub(None, h45::GETLINE_SLOT, true, 3, 3, Some(r));
    let slot = t.rebase(h45::GETLINE_SLOT);
    t.w32(slot, getline);
    let r = rng.next();
    let printf = t.stub(None, h45::PRINTF_SLOT, false, 2, 0, Some(r));
    let slot = t.rebase(h45::PRINTF_SLOT);
    t.w32(slot, printf);
    let found: Vec<u32> = (0..8)
        .map(|_| pick(rng, &[buffer, buffer, buffer + 1, 0]))
        .collect();
    let strstr = t.seq_code(h45::STRSTR_SLOT, false, 2, 0, &found);
    let slot = t.rebase(h45::STRSTR_SLOT);
    t.w32(slot, strstr);
    let returns = [t.rebase(0x006b_d264), t.rebase(0x006b_d283)];
    let mut c = vec![0x83, 0x7c, 0x24, 0x04, 0x05]; // cmp dword [esp + 4], 5
    c.extend([0x74, 0x06, 0xff, 0x05]); // je +6; inc dword [count + 4]
    c.extend((count + 4).to_le_bytes());
    c.extend([0xff, 0x0d]); // dec dword [count]
    c.extend(count.to_le_bytes());
    c.extend([0x7f, 0x12]); // jg ret
    for (k, ra) in returns.into_iter().enumerate() {
        c.extend([0x81, 0x3c, 0x24]); // cmp dword [esp], ra
        c.extend(ra.to_le_bytes());
        c.extend([0x74, if k == 0 { 0x0c } else { 0x03 }]); // je escape
    }
    c.extend([0x31, 0xc0, 0xc3]); // ret: xor eax, eax; ret
    c.extend([0x83, 0xc4, 0x08, 0x5f, 0x5e, 0x8b, 0xe5, 0x5d, 0xc3]); // escape
    let code = t.code(&c);
    let site = t.rebase(h45::SLEEP);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(site + 5).to_le_bytes());
    t.patch(site, &j);
}

#[rustfmt::skip]
const CASES45: &[Case] = &[
    Case { prep: Some(prep_input_thread45), ..cdecl("input_thread", 0x006b_d1e0, 1, &[(h45::MEMSET, K::Cdecl(3)), (h45::ENABLE_COMMAND, K::This(1)), (h45::OTHER_COMMAND, K::This(1))], |m, _, a| {
        let printf: u32 = m.read(m.rebase(h45::PRINTF_SLOT));
        let strstr: u32 = m.read(m.rebase(h45::STRSTR_SLOT));
        loop {
            gh45::input_pass(m, printf, strstr);
            if m.read::<i32>(a[0]) <= 0 {
                break;
            }
        }
        Out::None
    }) },
];

#[test]
fn batch46_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 100 } else { 400 };
        run_cases(t, CASES45, n, &mut Rng(0x4545_b046));
    });
}

// ---- round 12 batch 47 (ffx_prender::h3_glue46) ----

/// The scene's per-frame update: the frame time, `+0x1195`, the
/// first-run tip byte, a pause request, the map (the three that reset
/// the cheats or another), the three modes (0, 1, 2, -1, -2 or another),
/// the key checks, the tips' answers, the two buttons, the debug word
/// and the free camera's state.
fn prep_animate46(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let aux = t.zeros(0x1000);
    let app = aux;
    let time = rng.float().to_bits();
    t.w32(app + 0x24, time);
    t.stub_ret(h46::GET_APPLICATION, false, 0, 0, app);
    t.write(obj + 0x1195, &[u8::from(rng.chance(2))]);
    t.write(obj + 0x10dcc, &[pick(rng, &[0, 1, 0x80]) as u8]);
    let settings = aux + 0x800;
    t.write(settings + 0x638, &[u8::from(rng.chance(2))]);
    t.global(h46::SETTINGS, settings);
    let request = if rng.chance(2) { 0 } else { rng.next() };
    t.global(h46::PAUSE_REQUEST, request);
    let any = rng.next();
    let map = pick(rng, &[0x17, 0x15c, 0x15d, 0x16, 0x41, any]);
    t.global(h46::CURRENT_MAP, map);
    for gl in [h46::AUTO_BATTLE, h46::CHEAT_POWERUP, h46::ENCOUNT_MODE] {
        let v = pick(rng, &[0, 1, 2, 0xffff_ffff, 0xffff_fffe, 0x7fff_ffff, 5]);
        t.global(gl, v);
    }
    for sym in [
        h46::CHECK_AUTO_BATTLE_KEY,
        h46::CHECK_INVINCIBLE_KEY,
        h46::CHECK_ENCOUNTER_KEY,
        h46::CAN_USE_AUTO_BATTLE,
        h46::PUSH_TIP_BY_LOC,
    ] {
        let v = pick(rng, &[0, 1, 0x100]);
        t.stub_ret(sym, false, u32::from(sym == h46::PUSH_TIP_BY_LOC), 0, v);
    }
    let presses = [pick(rng, &[0, 1]), pick(rng, &[0, 0x200])];
    t.stub_seq(h46::IS_PRESS_BUTTON, false, 1, 0, &presses);
    let camera = aux + 0x400;
    t.write(camera + 0x338, &[u8::from(rng.chance(3))]);
    t.write(camera + 0x33c, &[u8::from(rng.chance(3))]);
    t.w32(camera, pick(rng, &[2, 0, 1]));
    t.stub_ret(h46::FREE_CAMERA, false, 0, 0, camera);
    t.global(h46::DEBUG_WORD, pick(rng, &[0, 0, 1]));
    for gl in [h46::DELEGATION_HELPER, h46::LOADING_SCREEN] {
        let v = rng.next();
        t.global(gl, v);
    }
}

#[rustfmt::skip]
const CASES46: &[Case] = &[
    Case { prep: Some(prep_animate46), ..method("animate", 0x0065_7140, This::Object, 0, &[(h46::DELEGATION_UPDATE, K::This(0)), (h46::TEX_ANIM_ANIMATE, K::This(1)), (h46::MORPH_INSTANCE, K::Cdecl(0)), (h46::MORPH_UPDATE, K::This(1)), (h46::TIP_SYSTEM, K::Cdecl(0)), (h46::PUSH_TIP, K::This(1)), (h46::LIGHTING_INSTANCE, K::Cdecl(0)), (h46::LIGHTING_UPDATE_FLASH, K::This(1)), (h46::PAUSE_UPDATE, K::Cdecl(1)), (h46::ICON_PANEL, K::Cdecl(0)), (h46::ICON_UPDATE, K::This(0)), (h46::ICON_SHOW, K::This(1)), (h46::ICON_HIDE, K::This(1)), (h46::ICON_SET_VISIBLE, K::This(1)), (h46::ICON_HIDE_ALL, K::This(0)), (h46::SPD_INIT, K::Cdecl(0)), (h46::SPD_TICK, K::Cdecl(0)), (h46::POWERFUL_OFF, K::Cdecl(0)), (h46::FREE_CAMERA_PAUSE, K::This(0)), (h46::FREE_CAMERA_INPUT, K::This(2)), (h46::FMV_MANAGER, K::Cdecl(0)), (h46::FMV_PROCESS, K::This(0)), (h46::BLUR_INSTANCE, K::Cdecl(0)), (h46::BLUR_UPDATE, K::This(1)), (h46::BLUR_EX_INSTANCE, K::Cdecl(0)), (h46::BLUR_EX_UPDATE, K::This(1)), (h46::FOG_INSTANCE, K::Cdecl(0)), (h46::FOG_UPDATE, K::This(1)), (h46::CLOTH_RELEASE, K::Cdecl(0)), (h46::DELEGATION_RUN, K::This(0)), (h46::LOADING_UPDATE, K::This(1)), (h46::BOOT_TEXT_INSTANCE, K::Cdecl(0)), (h46::BOOT_TEXT_UPDATE, K::This(1)), (h46::TEXT_ANIMATE, K::This(1))], |m, this, _| u(gh46::animate(m, this))) },
];

#[test]
fn batch47_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 300 } else { 1000 };
        run_cases(t, CASES46, n, &mut Rng(0x4646_b047));
    });
}

// ---- round 13 batch 48 (ffx_prender::h3_glue47) ----

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

impl Twins {
    /// A recording `fiosRead(stream, buf, n)` (cdecl) serving the bytes
    /// at `src` (`len` of them): `min(n, left)` bytes copied, the count
    /// in `eax`.
    fn stub_read48(&mut self, sym: u32, src: u32, len: u32) {
        let cells = self.zeros(8);
        self.w32(cells, src);
        self.w32(cells + 4, len);
        let (cursor, left) = (cells, cells + 4);
        let log = self.stub(None, sym, false, 3, 0, None);
        // push esi; push edi; push ecx; mov edi, [esp+0x14]; mov ecx, [esp+0x18]
        let mut c = vec![
            0x56, 0x57, 0x51, 0x8b, 0x7c, 0x24, 0x14, 0x8b, 0x4c, 0x24, 0x18,
        ];
        c.extend([0x3b, 0x0d]); // cmp ecx, [left]
        c.extend(left.to_le_bytes());
        c.extend([0x76, 0x06, 0x8b, 0x0d]); // jbe +6; mov ecx, [left]
        c.extend(left.to_le_bytes());
        c.extend([0x8b, 0x35]); // mov esi, [cursor]
        c.extend(cursor.to_le_bytes());
        c.extend([0x89, 0xc8, 0x29, 0x05]); // mov eax, ecx; sub [left], eax
        c.extend(left.to_le_bytes());
        c.extend([0x01, 0x05]); // add [cursor], eax
        c.extend(cursor.to_le_bytes());
        c.extend([0xf3, 0xa4, 0x59, 0x5f, 0x5e]); // rep movsb; pop ecx; pop edi; pop esi
        let at = self.cave_next;
        c.push(0xe9); // jmp log
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// Every option name `loadShader` looks for, `VSP_ALPHA_MODULATE` first
/// among the blends.
const VSP48: [&str; 28] = [
    "VSP_GROUND_SPECIAL",
    "VSP_Z_BUF_CLEAR",
    "VSP_PERS_Z_BUF_CLEAR",
    "VSP_DTT_SPECIAL",
    "VSP_GRAY_SCALE",
    "VSP_NOISE_BLUR_ADDITIVE",
    "VSP_NOISE_BLUR",
    "VSP_SCREEN_FILTER",
    "VSP_ZMAT",
    "VSP_ALPHA_MODULATE",
    "VSP_ALPHA_ADDITIVE",
    "VSP_ALPHA_SUBSTRACT3",
    "VSP_ALPHA_SUBSTRACT2",
    "VSP_ALPHA_SUBSTRACT",
    "VSP_ENABLE_TEXTURE",
    "VSP_ZWRITE_ENABLE",
    "VSP_ALPHA_TEST_ENABLED",
    "VSP_ZTEST_DISABLE",
    "VSP_TK_RUNTIME_CLUT_16x16",
    "VSP_ALPHA_NO_BLENDING",
    "VSP_EXTENDED_ALPHA",
    "VSP_ENABLE_NCLIP",
    "VSP_SHIFT_ON_SCREEN",
    "VSP_LOGO_SPECIAL",
    "VSP_POINTLIGHT",
    "VSP_TEXTURE_VIDEO",
    "VSP_LOGO_TEXTURE_VIDEO",
    "VSP_YIQ",
];

/// A random shader entry: `name:options` (or without the colon now and
/// then), the options a few of [`VSP48`] (mostly the later, cumulative
/// ones), separated by `|`.
fn shader_entry48(rng: &mut Rng, colon: bool) -> Vec<u8> {
    let mut s: Vec<u8> = b"vfx_".to_vec();
    for _ in 0..rng.below(12) {
        s.push(b'a' + rng.below(26) as u8);
    }
    s.extend(b".phyre");
    if colon {
        s.push(b':');
        for _ in 0..rng.below(7) {
            let k = if rng.chance(4) {
                rng.below(8)
            } else {
                8 + rng.below(20)
            };
            s.extend(VSP48[k as usize].as_bytes());
            s.push(b'|');
        }
    }
    s
}

fn prep_load_shader48(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    // only handed on (to the stubbed tree and as the request's signed
    // word pair): any value, high bit too
    if rng.chance(2) {
        *this = rng.word() | 0x8000_0000;
    }
    let dir = obj + 0x100;
    t.write(dir, b"/FFX_Data/GameData/PS3Data/Shaders/\0");
    let colon = !rng.chance(6);
    let mut entry = shader_entry48(rng, colon);
    entry.push(0);
    let at = obj + 0x400;
    t.write(at, &entry);
    args[0] = dir;
    args[1] = at;
    t.stub_ret(h47::READER_IS_OPEN, true, 0, 0, rng.word() & 0x101);
    if !colon {
        // the name and the options the original reads unwritten
        FRAME_WRITES.with(|z| z.replace(vec![(0x490, vec![0]), (0x410, vec![0])]));
    }
}

const LOAD_SHADER48_STUBS: &[(u32, K)] = &[
    (h47::READER_CTOR, K::This(1)),
    (h47::READER_CLOSE, K::This(0)),
    (h47::READER_DTOR, K::This(0)),
    (h47::ASYNC_LOAD_INSTANCE, K::Getter),
    (h47::LOAD_CLUSTER, K::Cdecl(9)),
    (h47::DBG_PRINTF, K::Cdecl(2)),
    (h47::TREE_INSERT_NEW, K::This(2)),
];

/// The collection's text: lines of shader entries, comments (`//`),
/// other lines and empty ones, ending with or without a newline; read in
/// chunks; the collection missing now and then; the shaders loaded or
/// not; the array's count with or without the not-owned bit.
fn prep_load_vfx_shader48(t: &mut Twins, obj: u32, _: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let mut text = vec![];
    for _ in 0..rng.below(14) {
        match rng.below(5) {
            0 => text.extend(b"// a comment vfx_x.phyre:VSP_ZMAT"),
            1 => text.extend(b"notes"),
            2 => {}
            _ => {
                let e = shader_entry48(rng, true);
                text.extend(e);
            }
        }
        text.push(b'\n');
    }
    if rng.chance(2) {
        let e = shader_entry48(rng, true);
        text.extend(e);
    }
    let src = obj + 0x800;
    t.write(src, &text);
    // what the stubbed fiosUnifyFilename writes
    let dir = b"/fp/Shaders/\0".to_vec();
    FRAME_WRITES.with(|z| z.replace(vec![(0x190, dir)]));
    FRAME_IGNORE.with(|z| z.replace(vec![(0x14, 4)]));
    if rng.chance(8) {
        // a read error at once
        t.stub_ret(h47::FIOS_READ, false, 3, 0, u32::MAX);
    } else {
        t.stub_read48(h47::FIOS_READ, src, text.len() as u32);
    }
    let stream = if rng.chance(6) { 0 } else { obj + 0x40 };
    t.stub_ret(h47::FIOS_OPEN, false, 2, 0, stream);
    let shader = if rng.chance(3) { 0 } else { rng.word() | 1 };
    t.stub_ret(h47::LOAD_SHADER, true, 2, 2, shader);
    let count = (rng.below(5) + 1) | (rng.word() & 0x8000_0000);
    t.stub_set_this(
        h47::PARRAY_PUSH,
        2,
        rng.word(),
        &[(0, count), (4, obj + 0x1000)],
    );
}

const LOAD_VFX_SHADER48_STUBS: &[(u32, K)] = &[
    (h47::UNIFY_FILENAME, K::Cdecl(3)),
    (h47::FIOS_CLOSE, K::Cdecl(1)),
    (h47::DBG_PRINTF, K::Cdecl(1)),
    (h47::PHYRE_MALLOC, K::Cdecl(2)),
    (h47::GET_APPLICATION, K::Cdecl(0)),
    (h47::FIXUP_CLUSTERS, K::Cdecl(2)),
    (h47::PHYRE_FREE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES48: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_load_shader48), ..method("load_shader", 0x006c_0a70, This::Object, 2, LOAD_SHADER48_STUBS, |m, this, a| u(gh47::load_shader(m, frame_ebp(), this, a[0], a[1]))) }), 0x5c0, 0),
    framed(with_seh(Case { prep: Some(prep_load_vfx_shader48), ..method("load_vfx_shader", 0x006c_0fc0, This::Object, 0, LOAD_VFX_SHADER48_STUBS, |m, this, _| u(gh47::load_vfx_shader(m, frame_ebp(), this))) }), 0x31a8, 0),
];

#[test]
fn batch48_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 30 } else { 150 };
        run_cases(t, CASES48, n, &mut Rng(0x4848_b048));
    });
}

// ---- round 13 batch 49 (ffx_prender::h3_glue48) ----

/// A scene with its loaded-character tree (head at `obj + 0x1400`, one
/// node at `obj + 0x1500` whose key is the name or another, inline or
/// in a buffer), a name (empty, the image's empty string, inline or
/// longer), the request, and the stubs' answers: the lookup's node, the
/// move's capacity and buffer, the reader open or not, the allocations
/// (a long name's two first, then the three blocks or null), the reader
/// (any value), the asset counts. The platform path is the original's.
fn prep_load_character49(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let head = obj + 0x1400;
    t.w32(obj + 0x11dc, head);
    let name_at = obj + 0x100;
    let len = pick(rng, &[0, 1, 5, 15, 16, 30]) as usize;
    let mut name: Vec<u8> = (0..len).map(|_| b'a' + rng.below(3) as u8).collect();
    let mut z = name.clone();
    z.push(0);
    t.write(name_at, &z);
    args[0] = if rng.chance(6) {
        t.rebase(h48::EMPTY)
    } else {
        name_at
    };
    let request = obj + 0x180;
    args[1] = request;
    // the node: the same key, or another
    let node = obj + 0x1500;
    if rng.chance(2) {
        name = (0..rng.below(20))
            .map(|_| b'a' + rng.below(3) as u8)
            .collect();
    }
    let mut key = name.clone();
    key.push(0);
    let inline = key.len() <= 16 && rng.chance(2);
    if inline {
        t.write(node + 0x10, &key);
        t.w32(node + 0x24, 0xf);
    } else {
        t.write(obj + 0x1600, &key);
        t.w32(node + 0x10, obj + 0x1600);
        t.w32(node + 0x24, 0x10 + rng.below(0x20));
    }
    t.w32(node + 0x20, name.len() as u32);
    t.w32(node + 0x28, obj + 0x1700);
    let found = if rng.chance(3) { head } else { node };
    t.stub_ret(h48::LOWER_BOUND, true, 1, 1, found);
    let cap = pick(rng, &[0xf, 0x10, 0x40]);
    t.stub_set_this(
        h48::STRING_MOVE_ASSIGN,
        1,
        rng.word(),
        &[(0x14, cap), (0, obj + 0x1800)],
    );
    t.stub_ret(h48::READER_IS_OPEN, true, 0, 0, rng.word() & 0x101);
    // a long name's buffers (the key, the by-value copy) come first
    let mut news = vec![];
    if len > 15 {
        news.extend([obj + 0x1b00, obj + 0x1b80]);
    }
    for (k, at) in [0x1900u32, 0x1980, 0x19c0].into_iter().enumerate() {
        news.push(if (k > 0 && rng.chance(5)) || rng.chance(8) {
            0
        } else {
            obj + at
        });
    }
    news.extend([obj + 0x1c00, obj + 0x1c80]);
    t.stub_seq(h48::OPERATOR_NEW, false, 1, 0, &news);
    // the reader is only handed to stubs: any value, high bit too
    let reader = if rng.chance(2) {
        obj + 0x1a00
    } else {
        0x8000_0000 | (rng.word() & 0x3fff_ffff)
    };
    t.stub_ret(h48::ASSET_READER_CTOR, true, 0, 0, reader);
    t.stub_ret(h48::OPERATOR_NEW_ARRAY, false, 1, 0, obj + 0x1a80);
    let count = pick(rng, &[0, 1, 3, u32::MAX]);
    let mut counts = vec![rng.below(5), count];
    counts.extend(std::iter::repeat_n(count, 4));
    t.stub_seq(h48::ASSET_COUNT, true, 0, 0, &counts);
    t.stub_wide(h48::LOAD_CLUSTER, 7, rng.word());
}

const LOAD_CHARACTER49_STUBS: &[(u32, K)] = &[
    (h48::OPERATOR_DELETE, K::Cdecl(1)),
    (h48::TABLE_INSTANCE, K::Getter),
    (h48::INSERT_PROTOTYPE, K::This(2)),
    (h48::BUILD_SUFFIXES, K::This(0)),
    (h48::READER_CTOR, K::This(1)),
    (h48::READER_CLOSE, K::This(0)),
    (h48::READER_DTOR, K::This(0)),
    (h48::READ_ASSET_HEADER, K::This(1)),
    (h48::ASYNC_LOAD_INSTANCE, K::Getter),
    (h48::CREATE_CHILDREN, K::This(1)),
    (h48::ASSET_PATH, K::This(1)),
];

#[rustfmt::skip]
const CASES49: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_load_character49), ..method("load_character", 0x0066_3f20, This::Object, 2, LOAD_CHARACTER49_STUBS, |m, this, a| u(gh48::load_character(m, frame_ebp(), frame_ebp() - 0xc0, this, a[0], a[1]))) }), 0xa8, 0),
];

#[test]
fn batch49_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 200 };
        run_cases(t, CASES49, n, &mut Rng(0x4949_b049));
    });
}

// ---- round 14 batch 50 (ffx_prender::h3_glue49) ----

/// A scene (`+0x11dc` its loaded-character tree, `+0xd0` a renderer with
/// a recording virtual `+8`, eight unload slots at `+0xfccc`), the
/// character (`args[0]`, its key 0 now and then). The tree's seven nodes
/// (in order N0, N5, N6, N1, N3, N2, N4: a right subtree's leftmost and
/// climbs of one and two steps back up) each have a value; one of them holds the key. Their names mix
/// `a`, `/` and `.`, inline or in a buffer; the string code and
/// `memmove` are the original's, `operator new` gives buffers in turn.
/// The vector copy gives 0 to 3 clusters (or no buffer).
fn prep_unload_character50(
    t: &mut Twins,
    obj: u32,
    this: &mut u32,
    args: &mut [u32],
    rng: &mut Rng,
) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let renderer = obj + 0x300;
    let vtable = obj + 0x340;
    let flush = t.stub(None, 0x00d0_0008, true, 0, 0, Some(rng.word()));
    t.w32(vtable + 8, flush);
    t.w32(renderer, vtable);
    t.w32(scene + 0xd0, renderer);
    let character = obj + 0x180;
    args[0] = character;
    let key = if rng.chance(6) { 0 } else { rng.word() | 1 };
    t.w32(character + 4, key);
    // the tree: one value of its nodes holds the key
    let values: Vec<u32> = (0..7).map(|k| obj + 0x1600 + 0x40 * k).collect();
    name_tree50(t, obj, scene + 0x11dc, &values, rng);
    let owner = rng.below(7) as usize;
    for (k, &value) in values.iter().enumerate() {
        let mine = if k == owner {
            key
        } else {
            key ^ (1 + rng.below(9))
        };
        t.w32(value, mine);
        let data = if rng.chance(4) { 0 } else { obj + 0x1900 };
        t.w32(value + 4, data);
    }
    t.stub_seq(
        h48::OPERATOR_NEW,
        false,
        1,
        0,
        &[obj + 0x1c00, obj + 0x1c80, obj + 0x1d00],
    );
    // the copied vector
    let count = rng.below(4);
    let (begin, end) = if count == 0 && rng.chance(2) {
        (0, 0)
    } else {
        (obj + 0x1e00, obj + 0x1e00 + 4 * count)
    };
    for i in 0..count {
        let c = rng.word();
        t.w32(obj + 0x1e00 + 4 * i, c);
    }
    t.stub_this_out(
        h49::VECTOR_COPY,
        1,
        1,
        Some(rng.word()),
        &[(0, begin), (4, end), (8, end)],
    );
    // the unload slots: in use or not, this character or another
    for k in 0..8 {
        let slot = scene + 0xfccc + 0x20c * k;
        let used = if rng.chance(3) { 0 } else { rng.word() | 1 };
        let who = if rng.chance(3) { character } else { rng.word() };
        t.w32(slot, used);
        t.w32(slot + 4, who);
        t.w32(slot + 8, rng.word());
    }
}

const UNLOAD_CHARACTER50_STUBS: &[(u32, K)] = &[
    (h49::SYNC_RENDER, K::This(0)),
    (h49::OPERATOR_DELETE, K::Cdecl(1)),
    (h49::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h49::UNREGISTER_BY_NAME, K::This(1)),
    (h49::CLUSTER_MANAGER_INSTANCE, K::Getter),
    (h49::RELEASE_PCLUSTER, K::This(1)),
    (h49::TREE_ERASE, K::This(2)),
];

#[rustfmt::skip]
const CASES50: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_unload_character50), ..method("unload_character", 0x0067_0b20, This::Object, 1, UNLOAD_CHARACTER50_STUBS, |m, this, a| u(gh49::unload_character(m, frame_ebp(), this, a[0]))) }), 0x54, 0),
];

#[test]
fn batch50_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 200 };
        run_cases(t, CASES50, n, &mut Rng(0x5050_b050));
    });
}

/// A name tree (`*slot` its head at `obj + 0x1400`) of seven nodes, in
/// order N0, N5, N6, N1, N3, N2, N4 (a right subtree's leftmost and
/// climbs of one and two steps back up); node k's value (`+0x28`) is
/// `values[k]`. Their names (`+0x10`) mix `a`, `/` and `.`, or have a
/// long part between a '/' and a '.', inline or in a buffer.
fn name_tree50(t: &mut Twins, obj: u32, slot: u32, values: &[u32], rng: &mut Rng) {
    let head = obj + 0x1400;
    let n: Vec<u32> = (0..7).map(|k| obj + 0x1440 + 0x40 * k).collect();
    t.w32(slot, head);
    t.write(head + 0xd, &[1]);
    for (node, [left, parent, right]) in [
        (head, [n[0], n[1], n[4]]),
        (n[0], [head, n[1], n[5]]),
        (n[5], [head, n[0], n[6]]),
        (n[6], [head, n[5], head]),
        (n[1], [n[0], head, n[2]]),
        (n[2], [n[3], n[1], n[4]]),
        (n[3], [head, n[2], head]),
        (n[4], [head, n[2], head]),
    ] {
        t.w32(node, left);
        t.w32(node + 4, parent);
        t.w32(node + 8, right);
        if node != head {
            t.write(node + 0xd, &[0]);
        }
    }
    for (k, &node) in n.iter().enumerate() {
        t.w32(node + 0x28, values[k]);
        let mut name: Vec<u8> = if rng.chance(3) {
            // a long part between the last '/' and the last '.'
            let mut v = b"d/".to_vec();
            v.extend((0..16 + rng.below(12)).map(|_| b'a' + rng.below(3) as u8));
            v.extend(b".ext");
            v
        } else {
            let len = pick(rng, &[0, 1, 4, 9, 15, 16, 27]) as usize;
            (0..len).map(|_| b"aa/.b"[rng.below(5) as usize]).collect()
        };
        let len = name.len();
        name.push(0);
        let s = node + 0x10;
        if len < 16 && rng.chance(4) {
            t.write(s, &name);
            t.w32(s + 0x14, 0xf);
        } else {
            let buf = obj + 0x1a00 + 0x40 * k as u32;
            t.write(buf, &name);
            t.w32(s, buf);
            t.w32(s + 0x14, (len as u32).max(0x10) + rng.below(8));
        }
        t.w32(s + 0x10, len as u32);
    }
}

// ---- round 14 batch 51 (ffx_prender::h3_glue50) ----

/// A character (`args[0]`, 0 now and then; its model `+4` and shared
/// flag `+0x20`) and key; the list from `operator new` (then buffers for
/// long names); the cursor script over the model's meshes; the push
/// stub setting the list's count (high bit too) and data, or the
/// original push; the name tree
/// with 0 to all nodes this character's; `PhyreMalloc` giving memory or
/// 0. The string code and `memmove` are the original's, and every other
/// time the push too (`PhyreFree` stubbed).
fn prep_instantiate51(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let character = obj + 0x600;
    args[0] = if rng.chance(8) { 0 } else { character };
    args[1] = rng.word();
    let model = obj + 0x800;
    t.w32(character + 4, model);
    t.w32(
        model + 0x1c,
        if rng.chance(2) {
            model + 0x1c
        } else {
            obj + 0x880
        },
    );
    t.w32(character + 0x20, if rng.chance(2) { 0 } else { rng.word() });
    let list = obj + 0x900;
    let mut news = vec![list];
    news.extend((0..8).map(|k| obj + 0x1c00 + 0x40 * k));
    t.stub_seq(h50::OPERATOR_NEW, false, 1, 0, &news);
    let area = t.zeros(0x800);
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script(h50::CURSOR_BEGIN, h50::CURSOR_NEXT, &steps);
    let count = pick(rng, &[0, 1, 3, 0x8000_0002]);
    let src = obj + 0xa00;
    for k in 0..4 {
        let w = rng.word();
        t.w32(src + 4 * k, w);
    }
    if rng.chance(2) {
        // the original push (it reads the value through its pointer)
        t.stub_ret(h50::PHYRE_FREE, false, 1, 0, rng.word());
    } else {
        t.stub_this_out(
            h50::PARRAY_PUSH_INTO,
            2,
            2,
            Some(rng.word()),
            &[(0, count), (4, src)],
        );
    }
    let mallocs: Vec<u32> = (0..32)
        .map(|k| {
            if rng.chance(4) {
                0
            } else {
                obj + 0xb00 + 0x40 * k
            }
        })
        .collect();
    t.stub_seq(h50::PHYRE_MALLOC, false, 2, 0, &mallocs);
    let tree = obj + 0x13c0;
    t.stub_ret(h50::TABLE_TREE, true, 0, 0, tree);
    let values: Vec<u32> = (0..7)
        .map(|_| if rng.chance(3) { character } else { rng.word() })
        .collect();
    name_tree50(t, obj, tree, &values, rng);
}

const INSTANTIATE51_STUBS: &[(u32, K)] = &[
    (h50::OPERATOR_DELETE, K::Cdecl(1)),
    (h50::CHARACTER_ADD_INSTANCE, K::This(1)),
    (h50::INSTANTIATE_MESH, K::This(3)),
    (h50::INSTANCE_ADD_MESH, K::This(1)),
    (h50::LIGHTING_INSTANCE, K::Getter),
    (h50::LIGHTING_INSTANTIATE, K::This(1)),
    (h50::TABLE_INSTANCE, K::Getter),
    (h50::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h50::REGISTER_CHARACTER_INSTANCE, K::This(2)),
    (h50::DEPTH_BIAS_ON_INSTANTIATED, K::Cdecl(3)),
];

#[rustfmt::skip]
const CASES51: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_instantiate51), ..cdecl("instantiate_character", 0x0063_c540, 2, INSTANTIATE51_STUBS, |m, _, a| u(gh50::instantiate_character(m, frame_ebp(), a[0], a[1]))) }), 0x54, 0),
];

#[test]
fn batch51_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 40 } else { 200 };
        run_cases(t, CASES51, n, &mut Rng(0x5151_b051));
    });
}

// ---- round 14 batch 52 (ffx_prender::h3_glue51) ----

/// A scene with its post-process manager (texture slots `+0x8c`, `+0x90`,
/// `+0xa8`, `+0x1232c`; the colour one's texture always there, the
/// others' `+4` byte set now and then), renderer, camera (none now and
/// then) and VFX characters of several layers; an event of every kind
/// (and one past the switch) with its fields; the renderer calls
/// failing now and then. The surfaces' constructor and destructor are
/// the original's.
fn prep_vfx_event52(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let post = t.zeros(0x1_2400);
    t.w32(scene + 0x10cc, post);
    for (k, off) in [0x8cu32, 0x90, 0xa8, 0x1_232c].into_iter().enumerate() {
        let slot = obj + 0x100 + 0x10 * k as u32;
        let texture = obj + 0x200 + 0x40 * k as u32;
        t.w32(post + off, slot);
        t.w32(slot, texture);
        let none = k != 0 && rng.chance(4);
        t.write(slot + 4, &[u8::from(none)]);
        t.w32(texture + 0x1c, rng.word());
        t.w32(texture + 0x20, rng.word());
    }
    t.write(post + 0x1_23a0, &[u8::from(rng.chance(2))]);
    let renderer = obj + 0x400;
    t.w32(scene + 0xd0, renderer);
    let camera = obj + 0x500;
    for k in 0..16 {
        let f = rng.float().to_bits();
        t.w32(camera + 0x34 + 4 * k, f);
    }
    let kind = rng.below(8);
    t.w32(
        scene + 0xdc,
        if kind != 0 && rng.chance(4) {
            0
        } else {
            camera
        },
    );
    // the VFX characters: none, another layer, this one; shown or not
    let layer = pick(rng, &[0, 3, 0xffff_ffff, 0x7f]);
    let count = rng.below(6);
    t.w32(scene + 0xfb48, count);
    t.w32(scene + 0xfb50, obj + 0x600);
    for k in 0..count {
        let chr = obj + 0x700 + 0x20 * k;
        t.w32(obj + 0x600 + 4 * k, if rng.chance(5) { 0 } else { chr });
        let l = if rng.chance(2) {
            layer as u8
        } else {
            rng.below(4) as u8
        };
        t.write(chr + 0x14, &[u8::from(!rng.chance(3)), l]);
    }
    // the event
    let event = obj + 0x80;
    args[0] = event;
    t.w32(event, kind);
    let words = [rng.word(), rng.word()];
    t.w32(event + 8, words[0]);
    t.w32(event + 0xc, words[1]);
    match kind {
        0 => t.w32(event + 8, layer),
        1 => {
            let b = [rng.below(4) as u8, rng.below(4) as u8, 0, 0];
            t.write(event + 8, &b);
            let depth = pick(rng, &[0, 0, 1, 0x100, 0xff00]) as u16;
            t.write(event + 0xa, &depth.to_le_bytes());
            if rng.chance(2) {
                t.w32(event + 0xc, 0);
            }
        }
        5 => {
            t.w32(event + 8, obj + 0x700);
            t.write(obj + 0x714, &[u8::from(rng.chance(2))]);
            if rng.chance(2) {
                t.write(event + 0xc, &[0]);
            }
        }
        6 => {
            let array = obj + 0x900;
            t.w32(event + 8, array);
            let n = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
            t.w32(array, n);
            t.w32(array + 4, obj + 0x980);
            for k in 0..4 {
                let inst = obj + 0xa00 + 0x20 * k;
                t.w32(obj + 0x980 + 4 * k, inst);
                t.w32(inst + 0x10, rng.word());
            }
        }
        _ => {}
    }
    // renderer results: mostly 0
    let result = |rng: &mut Rng| if rng.chance(6) { rng.word() | 1 } else { 0 };
    let r = result(rng);
    t.stub_ret(h51::END_SCENE, true, 1, 1, r);
    let r = result(rng);
    t.stub_ret(h51::SET_COLOR_TARGET, true, 2, 2, r);
    let r = result(rng);
    t.stub_ret(h51::SET_DEPTH_STENCIL_TARGET, true, 1, 1, r);
    let blur = pick(rng, &[0, 1, 0x100, 0xffff_ff01]);
    // drawing a character may switch the scene's camera (the matrix goes
    // back into the one the scene has then)
    let other = obj + 0xc00;
    for k in 0..16 {
        let f = rng.float().to_bits();
        t.w32(other + 0x34 + 4 * k, f);
    }
    let after = if rng.chance(2) { camera } else { other };
    t.stub_store(h51::DRAW_CHARACTER, 2, rng.word(), scene + 0xdc, after);
    t.stub_ret(h51::IS_MOTION_BLUR_ON, true, 0, 0, blur);
}

const VFX_EVENT52_STUBS: &[(u32, K)] = &[
    (h51::SET_CAMERA, K::This(1)),
    (h51::UPDATE_VIEW_MATRICES, K::This(0)),
    (h51::COPY_DEPTH_BUFFER, K::This(9)),
    (h51::COPY_BUFFER, K::This(6)),
    (h51::SET_COLOR4, K::This(1)),
    (h51::RENDER_SCREEN_FADE, K::This(4)),
    (h51::PS2_LIGHTING_APPLY, K::Cdecl(1)),
    (h51::SPECULAR_INSTANCE, K::Getter),
    (h51::RENDER_CORRESPOND_SPECULAR, K::This(2)),
    (h51::SET_VISIBLE, K::Cdecl(2)),
    (h51::SET_VIEWPORT, K::This(4)),
    (h51::DEGRADE_RESOLUTION, K::This(1)),
    (h51::SET_CLEAR_COLOR, K::This(4)),
    (h51::BEGIN_SCENE, K::This(1)),
    (h51::RENDER_MESH_INSTANCE, K::This(4)),
];

#[rustfmt::skip]
const CASES52: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_vfx_event52), ..method("process_vfx_draw_event", 0x0066_6e40, This::Object, 1, VFX_EVENT52_STUBS, |m, this, a| u(gh51::process_vfx_draw_event(m, frame_ebp(), this, a[0]))) }), 0xa8, 0),
];

#[test]
fn batch52_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES52, n, &mut Rng(0x5252_b052));
    });
}

// ---- round 15 batch 53 (ffx_prender::h3_glue52) ----

/// A character's last asset loaded: its name (`a`, `b`, `/` and `.`,
/// 0 to 40 bytes; some with no `/` or no `.`), `out` (its model word),
/// a reader (0 now and then), the asset array (data or none), the
/// request and `extra` (0 or a pointer). The cluster fix-up fails now
/// and then; the reader's symbol ids are -1 or an index; the `.ovp` file
/// is missing, fails to open, or reads in up to three chunks of the
/// frame's bytes (random, or a long run without a 0). The string code
/// is the original's, `operator new` gives 0x1000-byte buffers in turn;
/// the tree's node and insert are recording stubs.
fn prep_all_assets53(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let len = pick(rng, &[0, 1, 5, 15, 16, 24, 40]) as usize;
    let alphabet: &[u8] = match rng.below(4) {
        0 => b"ab.",
        1 => b"ab/",
        _ => b"ab/.",
    };
    let mut name: Vec<u8> = (0..len)
        .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
        .collect();
    name.push(0);
    let name_at = obj + 0x100;
    t.write(name_at, &name);
    let out = obj + 0x180;
    t.w32(out, rng.word());
    let reader = match rng.below(4) {
        0 => 0,
        1 => obj + 0x1a00,
        _ => rng.word(),
    };
    let array = obj + 0x1c0;
    t.w32(array + 8, if rng.chance(5) { 0 } else { obj + 0x1d00 });
    let request = obj + 0x1e0;
    let extra = if rng.chance(3) { 0 } else { obj + 0x1e80 };
    args.copy_from_slice(&[out, reader, array, request, name_at, extra]);
    let failed = if rng.chance(5) { 1 + rng.below(9) } else { 0 };
    t.stub_ret(h52::FIXUP_CLUSTERS, false, 2, 0, failed);
    let mut id = || {
        if rng.chance(3) {
            u32::MAX
        } else {
            rng.below(4)
        }
    };
    let ids = [id(), id()];
    t.stub_seq(h52::GET_SYMBOL_ID, true, 1, 1, &ids);
    let heap = t.zeros(0x1000 * 24);
    let news: Vec<u32> = (0..24).map(|k| heap + 0x1000 * k).collect();
    t.stub_seq(h48::OPERATOR_NEW, false, 1, 0, &news);
    t.stub_ret(h52::BUY_NODE, true, 1, 1, obj + 0x1f00);
    let exists = u32::from(!rng.chance(4));
    t.stub_ret(h52::FIOS_IS_FILE_EXIST, false, 1, 0, exists);
    let stream = if rng.chance(4) { 0 } else { rng.word() | 1 };
    t.stub_ret(h52::FIOS_OPEN, false, 2, 0, stream);
    let mut reads: Vec<u32> = (0..rng.below(4))
        .map(|_| pick(rng, &[1, 7, 16, 0x40, 0x3ff]))
        .collect();
    reads.push(if rng.chance(3) { u32::MAX } else { 0 });
    t.stub_seq(h52::FIOS_READ, false, 3, 0, &reads);
    if rng.chance(2) {
        let text: Vec<u8> = (0..0x400).map(|_| b'a' + rng.below(26) as u8).collect();
        FRAME_WRITES.with(|z| z.replace(vec![(0x410, text)]));
    }
}

const ALL_ASSETS53_STUBS: &[(u32, K)] = &[
    (h52::GET_APPLICATION, K::Getter),
    (h52::ASSET_COUNT, K::This(0)),
    (h52::READER_DTOR, K::This(0)),
    (h52::OPERATOR_DELETE, K::Cdecl(1)),
    (h52::OPERATOR_DELETE_ARRAY, K::Cdecl(1)),
    (h52::USER_FREE, K::Cdecl(1)),
    (h52::TREE_INSERT, K::This(4)),
    (h52::REBIND_ALT, K::This(3)),
    (h52::FIOS_CLOSE, K::Cdecl(1)),
    (h52::TEXTURE_ANIMATION_INSTANCE, K::Getter),
    (h52::ADD_ANIMATED_MATERIALS, K::This(7)),
    (h52::TABLE_INSTANCE, K::Getter),
    (h52::INSERT_PROTOTYPE, K::This(2)),
    (h52::BUILD_SUFFIXES, K::This(0)),
    (h52::CHR_LOAD_COUNTDOWN, K::Cdecl(0)),
];

#[rustfmt::skip]
const CASES53: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_all_assets53), ..method("on_character_all_assets_loaded", 0x0065_a760, This::Object, 6, ALL_ASSETS53_STUBS, |m, this, a| u(gh52::on_character_all_assets_loaded(m, frame_ebp(), this, a[0], a[1], a[2], a[3], a[4], a[5]))) }), 0x4ac, 0),
];

#[test]
fn batch53_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES53, n, &mut Rng(0x5353_b053));
    });
}

// ---- round 15 batch 54 (ffx_prender::h3_glue53) ----

/// Renderer results for `n` calls: mostly 0, now and then an error.
fn results54(rng: &mut Rng, n: usize) -> Vec<u32> {
    (0..n)
        .map(|_| if rng.chance(10) { rng.word() | 1 } else { 0 })
        .collect()
}

/// A scene with the fade byte or `g_MovieFading` set (now and then
/// neither), a camera (random `+0x150`/`+0x154`, a recording virtual
/// `+4`), the post targets `+0x8c`, `+0x90`, `+0xa8` (their textures
/// there or not), the ability map's 15 instances (some missing), the
/// scene's VFX instances (count signed: -1 too) and sphere-grid
/// instances (some 0), visibility answers with stray high bits, and the
/// renderer calls failing now and then at any step. Surfaces and
/// `_CIatan` are the original's.
fn prep_abmap54(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let fade = u8::from(rng.chance(2));
    t.write(scene + 0x10dcc, &[fade]);
    let fading = if fade == 0 && rng.chance(5) {
        0
    } else {
        rng.word()
    };
    t.global(h53::MOVIE_FADING, fading);
    let camera = obj + 0x500;
    let vtable = obj + 0x4c0;
    let update = t.stub(None, 0x00dc_0004, true, 0, 0, Some(rng.word()));
    t.w32(vtable + 4, update);
    t.w32(camera, vtable);
    let (a, b) = (rng.float().to_bits(), rng.float().to_bits());
    t.w32(camera + 0x150, a);
    t.w32(camera + 0x154, b);
    t.w32(scene + 0xdc, camera);
    t.w32(scene + 0xcc, obj + 0x700);
    let post = t.zeros(0x200);
    t.w32(scene + 0x10cc, post);
    for (k, off) in [0x8cu32, 0x90, 0xa8].into_iter().enumerate() {
        let slot = obj + 0x100 + 0x10 * k as u32;
        let texture = obj + 0x200 + 0x40 * k as u32;
        t.w32(post + off, slot);
        t.w32(slot, texture);
        // the colour target always has its texture (the viewport reads it)
        let none = if k != 0 && rng.chance(4) {
            pick(rng, &[1, 2, 0x80]) as u8
        } else {
            0
        };
        t.write(slot + 4, &[none]);
        t.w32(texture + 0x1c, rng.word());
        t.w32(texture + 0x20, rng.word());
    }
    let renderer = obj + 0x400;
    t.w32(scene + 0xd0, renderer);
    t.w32(renderer + 0x13c, rng.word());
    t.global(h53::DELEGATION_HELPER, obj + 0x780);
    t.global(h53::ABMAP, obj + 0x7c0);
    let instances: Vec<u32> = (0..15u32)
        .map(|k| {
            if rng.chance(3) {
                0
            } else {
                obj + 0x800 + 0x20 * k
            }
        })
        .collect();
    for k in 0..15u32 {
        let w = rng.word();
        t.w32(obj + 0x810 + 0x20 * k, w);
    }
    t.stub_seq(h53::ABMAP_MESH_INSTANCE, true, 1, 1, &instances);
    let vfx = pick(rng, &[0, 1, 3, 5, u32::MAX]);
    t.w32(scene + 0xf8cc, vfx);
    for k in 0..5 {
        let inst = if rng.chance(4) {
            0
        } else {
            obj + 0xc00 + 0x20 * k
        };
        t.w32(scene + 0x38cc + 4 * k, inst);
        let w = rng.word();
        t.w32(obj + 0xc1c + 0x20 * k, w);
    }
    let grid = pick(rng, &[0, 0, 1, 4]);
    t.w32(scene + 0x3854, grid);
    t.w32(scene + 0x385c, obj + 0xe00);
    for k in 0..4 {
        let inst = if rng.chance(4) {
            0
        } else {
            obj + 0xe40 + 0x20 * k
        };
        t.w32(obj + 0xe00 + 4 * k, inst);
        let w = rng.word();
        t.w32(obj + 0xe5c + 0x20 * k, w);
    }
    let seen: Vec<u32> = (0..12)
        .map(|_| pick(rng, &[0, 1, 0x100, 0x101, 0xffff_ff00]))
        .collect();
    t.stub_seq(h53::IS_VISIBLE, true, 1, 1, &seen);
    let r = results54(rng, 6);
    t.stub_seq(h53::SET_COLOR_TARGET, true, 2, 2, &r);
    let r = results54(rng, 6);
    t.stub_seq(h53::SET_DEPTH_STENCIL_TARGET, true, 1, 1, &r);
    let r = results54(rng, 5);
    t.stub_seq(h53::BEGIN_SCENE, true, 1, 1, &r);
    let r = results54(rng, 5);
    t.stub_seq(h53::END_SCENE, true, 1, 1, &r);
}

const ABMAP54_STUBS: &[(u32, K)] = &[
    (h53::SET_CAMERA, K::This(1)),
    (h53::UPDATE_VIEW_MATRICES, K::This(0)),
    (h53::MATRIX_COPY, K::This(1)),
    (h53::BEFORE_SCENE, K::This(0)),
    (h53::BEFORE_VFX, K::This(0)),
    (h53::BEFORE_EFFECTS, K::This(0)),
    (h53::RUN_FLAG_800, K::This(0)),
    (h53::SET_CLEAR_STENCIL, K::This(1)),
    (h53::SET_VIEWPORT, K::This(4)),
    (h53::DEGRADE_RESOLUTION, K::This(1)),
    (h53::RENDER_MESH_INSTANCE, K::This(4)),
    (h53::PPP_FILL_WAIT, K::Cdecl(0)),
    (h53::MERGE_VFX_ARRAYS, K::This(0)),
    (h53::CLEAR_ARRAYS, K::This(0)),
    (h53::RENDER_FOR_PASS, K::This(1)),
    (h53::COPY_BUFFER, K::This(6)),
    (h53::CLEAR_SCENE, K::This(1)),
];

#[rustfmt::skip]
const CASES54: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_abmap54), ..method("render_abmap", 0x0065_2a10, This::Object, 0, ABMAP54_STUBS, |m, this, _| u(gh53::render_abmap(m, frame_ebp(), this))) }), 0x38, 0),
];

#[test]
fn batch54_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES54, n, &mut Rng(0x5454_b054));
    });
}

// ---- round 16 batch 55 (ffx_prender::h3_glue54) ----

/// The user heap's `malloc` (cdecl), which the arrays' `reserve` calls.
const USER_MALLOC55: u32 = 0x0068_7020;

/// A VFX texture list: its name (a directory or two, then a file),
/// `kind`, `async` (low byte 0 or not) and its text: 0 to 5 lines of 1
/// to 20 bytes (some ending in `\r`), the last with or without its
/// newline (no empty lines: an empty one ends the original in
/// `report_rangecheckfailure`). The text is the scene's named one, or
/// the file's (missing, not opening, or read: the first `new[]` block
/// holds it). The scene's four arrays have 0 to 4 slots (some free) and
/// room or not; the line files open or not, the fix-up fails now and
/// then. The string and array code is the original's, with `operator
/// new` and the user heap's `malloc` handing out blocks in turn; the
/// C runtime's `strstr`, `strncpy` and `sprintf` are the game's DLL.
fn prep_load_vfx55(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let names: [&[u8]; 4] = [
        b"vfx/list.txt",
        b"a/b/c.lst",
        b"x/",
        b"data/effect/tex/magic.txt",
    ];
    let mut name = names[rng.below(4) as usize].to_vec();
    name.push(0);
    let name_at = obj + 0x100;
    t.write(name_at, &name);
    let lines = rng.below(6);
    let mut text = vec![];
    let mut empty = false;
    for k in 0..lines {
        let len = 1 + rng.below(20);
        text.extend((0..len).map(|_| b"abcdefgh._"[rng.below(10) as usize]));
        if rng.chance(3) {
            text.push(b'\r');
        }
        if k + 1 < lines || rng.chance(2) {
            text.push(b'\n');
        }
        // an empty line ends the split; the original then reaches the
        // line "\n" only when an earlier file fails to open
        if k == 0 && lines > 1 && rng.chance(4) {
            text.push(b'\n');
            empty = true;
        }
    }
    let async_load = if rng.chance(2) {
        rng.word() & !0xff
    } else {
        rng.word() | 1
    };
    args.copy_from_slice(&[rng.word(), name_at, async_load]);
    let heap = t.zeros(0x200 * 80);
    let mut nul = text.clone();
    nul.push(0);
    // the file's block: the text without its 0 (the original adds it)
    let mut block = text.clone();
    block.extend([b'z'; 8]);
    t.write(heap, &block);
    let found = if rng.chance(3) {
        let at = obj + 0x1000;
        t.write(at, &nul);
        at
    } else {
        0
    };
    t.stub_ret(h54::FIND_NAMED, true, 1, 1, found);
    t.stub_ret(
        h54::FIOS_IS_FILE_EXIST,
        false,
        1,
        0,
        u32::from(!rng.chance(6)),
    );
    let stream = if rng.chance(6) { 0 } else { rng.word() | 1 };
    t.stub_ret(h54::FIOS_OPEN, false, 2, 0, stream);
    t.stub_ret(h54::FIOS_FILE_SIZE, false, 1, 0, text.len() as u32);
    let news: Vec<u32> = (0..40).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(h54::OPERATOR_NEW_ARRAY, false, 1, 0, &news);
    let news: Vec<u32> = (40..60).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(h48::OPERATOR_NEW, false, 1, 0, &news);
    let news: Vec<u32> = (60..80).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(USER_MALLOC55, false, 1, 0, &news);
    // the scene's arrays: names +0x34, layouts +0x40, paths +0x4c, flags +0x58
    let n = rng.below(5);
    let room = if rng.chance(2) { 8 } else { n };
    for (k, &(arr, data, size)) in [
        (0x34, 0x1400, 4),
        (0x40, 0x1500, 20),
        (0x4c, 0x1600, 4),
        (0x58, 0x1700, 4),
    ]
    .iter()
    .enumerate()
    {
        t.w32(obj + arr, n);
        t.w32(obj + arr + 4, room);
        t.w32(obj + arr + 8, obj + data);
        for j in 0..n * size / 4 {
            let free = k == 0 && rng.chance(4);
            t.w32(obj + data + 4 * j, if free { 0 } else { rng.word() | 1 });
        }
    }
    let mut opens: Vec<u32> = (0..8)
        .map(|_| if rng.chance(6) { 0x100 } else { rng.word() | 1 })
        .collect();
    if empty {
        opens[0] = 0x100;
    }
    t.stub_seq(h54::READER_IS_OPEN, true, 0, 0, &opens);
    let fixups: Vec<u32> = (0..20)
        .map(|_| if rng.chance(8) { 1 + rng.below(9) } else { 0 })
        .collect();
    t.stub_seq(h54::FIXUP_CLUSTERS, false, 2, 0, &fixups);
    t.stub_ret(h54::SET_LAYOUT, false, 3, 3, rng.word());
    // junk read: the layout's five never-written words, 0 in both
    FRAME_WRITES.with(|z| z.replace(vec![(0x360, vec![0; 0x14])]));
}

const LOAD_VFX55_STUBS: &[(u32, K)] = &[
    (h54::FIOS_READ, K::Cdecl(3)),
    (h54::FIOS_CLOSE, K::Cdecl(1)),
    (h54::OPERATOR_DELETE_ARRAY, K::Cdecl(1)),
    (h54::OPERATOR_DELETE, K::Cdecl(1)),
    (h54::USER_FREE, K::Cdecl(1)),
    (h54::PSTRING, K::This(1)),
    (h54::PHYRE_FREE, K::Cdecl(1)),
    (h54::INSERT_PAIR, K::This(2)),
    (h54::ASYNC_INSTANCE, K::Getter),
    (h54::CHILDREN_REQUEST_LIST, K::This(1)),
    (h54::LOAD_CLUSTER, K::Cdecl(13)),
    (h54::READER_CTOR, K::This(1)),
    (h54::READER_CLOSE, K::This(0)),
    (h54::READER_DTOR, K::This(0)),
    (h54::CLUSTER_MANAGER, K::Getter),
    (h54::LOAD_PCLUSTER, K::This(1)),
    (h54::GET_APPLICATION, K::Getter),
];

#[rustfmt::skip]
const CASES55: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_load_vfx55), ..method("load_vfx_texture", 0x0066_5770, This::Object, 3, LOAD_VFX55_STUBS, |m, this, a| u(gh54::load_vfx_texture(m, frame_ebp(), this, a[0], a[1], a[2]))) }), 0x37c, 0),
];

#[test]
fn batch55_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES55, n, &mut Rng(0x5555_b055));
    });
}

// ---- round 16 batch 56 (ffx_prender::h3_glue54) ----

/// A magic file's VFX texture list: its name (some with an `x_ab/`
/// directory, which gives the `x/` fallback; `x/` with its byte 3 below
/// the directory from the frame), `kind`, `async` and its text in the
/// frame's read buffer (`ebp - 0x810`, followed by non-0 bytes), read in
/// one or two chunks of its length or less, then 0 or -1. The list file
/// is missing or does not open now and then. The readers open or not in
/// turn (the fallback's included); the rest as for batch 55.
fn prep_load_magic56(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    prep_load_vfx55(t, obj, this, args, rng);
    let names: [&[u8]; 5] = [
        b"magic/m_01/list.txt",
        b"vfx/list.txt",
        b"x/",
        b"bt/ab_cd/e_fg/l.txt",
        b"a_bc/x",
    ];
    let mut name = names[rng.below(5) as usize].to_vec();
    name.push(0);
    t.write(args[1], &name);
    let lines = rng.below(6);
    let mut text = vec![];
    for k in 0..lines {
        let len = 1 + rng.below(20);
        text.extend((0..len).map(|_| b"abcdefgh._"[rng.below(10) as usize]));
        if rng.chance(3) {
            text.push(b'\r');
        }
        if k + 1 < lines || rng.chance(2) {
            text.push(b'\n');
        }
    }
    let len = text.len() as u32;
    text.extend([b'z'; 8]);
    let mut reads = vec![];
    if len > 0 {
        reads.push(if rng.chance(4) {
            rng.below(len) + 1
        } else {
            len
        });
        if rng.chance(3) {
            reads.push(len);
        }
    }
    reads.push(if rng.chance(3) { u32::MAX } else { 0 });
    t.stub_seq(h54::FIOS_READ, false, 3, 0, &reads);
    t.stub_ret(
        h54::FIOS_IS_FILE_EXIST,
        false,
        1,
        0,
        u32::from(!rng.chance(8)),
    );
    let stream = if rng.chance(8) { 0 } else { rng.word() | 1 };
    t.stub_ret(h54::FIOS_OPEN, false, 2, 0, stream);
    let opens: Vec<u32> = (0..32)
        .map(|_| if rng.chance(3) { 0x100 } else { rng.word() | 1 })
        .collect();
    t.stub_seq(h54::READER_IS_OPEN, true, 0, 0, &opens);
    // junk read: the layout's never-written words, 0 in both; the bytes
    // below the directory not '_' (for "x/" the original reads one of
    // them, and a '_' there makes its strncpy run off the frame)
    FRAME_WRITES.with(|z| {
        z.replace(vec![
            (0x87c, vec![0; 0x14]),
            (0x810, text),
            (0x314, b"qqqq".to_vec()),
        ])
    });
}

const LOAD_MAGIC56_STUBS: &[(u32, K)] = &[
    (h54::FIOS_CLOSE, K::Cdecl(1)),
    (h54::OPERATOR_DELETE, K::Cdecl(1)),
    (h54::USER_FREE, K::Cdecl(1)),
    (h54::ASYNC_INSTANCE, K::Getter),
    (h54::CHILDREN_REQUEST_LIST, K::This(1)),
    (h54::LOAD_CLUSTER, K::Cdecl(13)),
    (h54::READER_CTOR, K::This(1)),
    (h54::READER_CLOSE, K::This(0)),
    (h54::READER_DTOR, K::This(0)),
    (h54::CLUSTER_MANAGER, K::Getter),
    (h54::LOAD_PCLUSTER, K::This(1)),
    (h54::GET_APPLICATION, K::Getter),
];

#[rustfmt::skip]
const CASES56: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_load_magic56), ..method("load_magic_file_vfx_texture", 0x0066_4550, This::Object, 3, LOAD_MAGIC56_STUBS, |m, this, a| u(gh54::load_magic_file_vfx_texture(m, frame_ebp(), this, a[0], a[1], a[2]))) }), 0x884, 0),
];

#[test]
fn batch56_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES56, n, &mut Rng(0x5656_b056));
    });
}

// ---- round 16 batch 57 (ffx_prender::h3_glue55) ----

/// A VFX texture list unloaded: the mode (each range's edges, 0x400,
/// 0x29e, others), the summon byte and the map number, the list's text
/// (the scene's named text or the file: missing, not opening, read) of 0
/// to 5 lines from a small vocabulary (the three summon textures among
/// them; some ending in `\r`), the scene's 0 to 6 slots: their paths
/// (none, a line's `vfx/D3D11/` path, a summon texture elsewhere), flags,
/// layout kinds (1, 2 or another) and a layout count of all or fewer.
/// The renderer's method 2 is a recording stub; the string, array and
/// C runtime code is the original's, with `operator new` and the user
/// heap's `malloc` handing out blocks in turn.
#[allow(clippy::too_many_lines)] // one scene, its list and its slots
fn prep_unload_vfx57(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_0000);
    *this = scene;
    let renderer = obj + 0x300;
    let vtable = obj + 0x340;
    let flush = t.stub(None, 0x00d0_0008, true, 0, 0, Some(rng.word()));
    t.w32(vtable + 8, flush);
    t.w32(renderer, vtable);
    t.w32(scene + 0xd0, renderer);
    t.write(scene + 0xfb04, &[u8::from(rng.chance(2))]);
    let map = if rng.chance(2) {
        0x146
    } else {
        rng.below(0x200)
    };
    let at = t.rebase(h55::CURRENT_MAP_NO);
    t.w32(at, map);
    let name_at = obj + 0x100;
    t.write(name_at, b"vfx/list.txt\0");
    let mode = pick(
        rng,
        &[
            u32::MAX,
            0x26e,
            0x271,
            0x275,
            0x276,
            0x27a,
            0x27d,
            0x27e,
            0x26d,
            0x400,
            0x29e,
            0x29f,
            5,
        ],
    );
    args.copy_from_slice(&[name_at, mode]);
    let vocab: [&[u8]; 5] = [
        b"abc",
        b"00001_19_0_0_1_1.dds.phyre",
        b"00002_19_0_0_1_1.dds.phyre",
        b"00003_19_0_0_1_1.dds.phyre",
        b"xy.dds",
    ];
    let lines = rng.below(6);
    let mut text = vec![];
    for k in 0..lines {
        text.extend(vocab[rng.below(5) as usize]);
        if rng.chance(3) {
            text.push(b'\r');
        }
        text.push(b'\n');
        if k + 1 == lines && rng.chance(2) {
            text.pop();
        }
    }
    let heap = t.zeros(0x200 * 40);
    let mut block = text.clone();
    block.extend([b'z'; 8]);
    t.write(heap, &block);
    let mut nul = text.clone();
    nul.push(0);
    let found = if rng.chance(3) {
        let at = obj + 0x1000;
        t.write(at, &nul);
        at
    } else {
        0
    };
    t.stub_ret(h55::FIND_NAMED, true, 1, 1, found);
    t.stub_ret(
        h55::FIOS_IS_FILE_EXIST,
        false,
        1,
        0,
        u32::from(!rng.chance(8)),
    );
    let stream = if rng.chance(8) { 0 } else { rng.word() | 1 };
    t.stub_ret(h55::FIOS_OPEN, false, 2, 0, stream);
    t.stub_ret(h55::FIOS_FILE_SIZE, false, 1, 0, text.len() as u32);
    let news: Vec<u32> = (0..10).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(h55::OPERATOR_NEW_ARRAY, false, 1, 0, &news);
    let news: Vec<u32> = (10..25).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(h48::OPERATOR_NEW, false, 1, 0, &news);
    let news: Vec<u32> = (25..40).map(|k| heap + 0x200 * k).collect();
    t.stub_seq(USER_MALLOC55, false, 1, 0, &news);
    // the slots: names +0x3c, paths +0x54, flags +0x60, layouts +0x48
    let n = rng.below(7);
    t.w32(scene + 0x4c, n);
    t.w32(
        scene + 0x40,
        if rng.chance(3) {
            n.saturating_sub(1)
        } else {
            n
        },
    );
    let (names, paths, flags, layouts) = (obj + 0x1400, obj + 0x1440, obj + 0x1480, obj + 0x1500);
    t.w32(scene + 0x3c, names);
    t.w32(scene + 0x54, paths);
    t.w32(scene + 0x60, flags);
    t.w32(scene + 0x48, layouts);
    for k in 0..n {
        let text_at = obj + 0x1600 + 0x40 * k;
        let path: Vec<u8> = match rng.below(4) {
            0 => vec![],
            1 => b"fx/00002_19_0_0_1_1.dds.phyre".to_vec(),
            _ => [b"vfx/D3D11/".as_slice(), vocab[rng.below(5) as usize]].concat(),
        };
        if path.is_empty() {
            t.w32(paths + 4 * k, 0);
        } else {
            t.write(text_at, &[path.as_slice(), b"\0"].concat());
            t.w32(paths + 4 * k, text_at);
        }
        t.w32(names + 4 * k, rng.word());
        t.w32(flags + 4 * k, u32::from(!rng.chance(4)));
        t.w32(layouts + 0x14 * k, pick(rng, &[1, 2, 2, 3]));
    }
}

const UNLOAD_VFX57_STUBS: &[(u32, K)] = &[
    (h55::SYNC_RENDER, K::This(0)),
    (h55::CLUSTER_MANAGER, K::Getter),
    (h55::RELEASE_PCLUSTER, K::This(1)),
    (h55::OPERATOR_DELETE, K::Cdecl(1)),
    (h55::OPERATOR_DELETE_ARRAY, K::Cdecl(1)),
    (h55::USER_FREE, K::Cdecl(1)),
    (h55::FIOS_READ, K::Cdecl(3)),
    (h55::FIOS_CLOSE, K::Cdecl(1)),
    (h55::RESTORE_SUMMON_TEXTURE, K::Cdecl(0)),
];

#[rustfmt::skip]
const CASES57: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_unload_vfx57), ..method("unload_vfx_texture", 0x0067_1300, This::Object, 2, UNLOAD_VFX57_STUBS, |m, this, a| u(gh55::unload_vfx_texture(m, frame_ebp(), this, a[0], a[1]))) }), 0x344, 0),
];

#[test]
fn batch57_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES57, n, &mut Rng(0x5757_b057));
    });
}

// ---- round 16 batch 58 (ffx_prender::h3_glue56) ----

/// The scene at boot: random bytes (the sampler words' bit 7 set or
/// not, the texture-slot capacities around 200, the work arrays' counts
/// and capacities negative or not, the free lists' around 3000, the flag
/// array's around 4000 with or without data), the configuration's
/// texture slots (their "none" byte set or not). Every callee is a
/// recording stub but the plain singleton getters (`mov eax, [x]`: the
/// original's, so the stubs fit the cave); the allocators hand out blocks, the free lists'
/// `reserve` points their data at a block.
fn prep_boot58(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let bytes = rng.bytes(0x1000);
    t.write(scene + 0xf000, &bytes);
    let config = t.zeros(0x1_2400);
    t.w32(scene + 0x10cc, config);
    for (k, off) in [0x8c, 0xac, 0x1_232c].into_iter().enumerate() {
        let slot = obj + 0x100 + 0x10 * k as u32;
        t.w32(config + off, slot);
        t.w32(slot, rng.word());
        t.write(slot + 4, &[u8::from(rng.chance(3))]);
    }
    for off in [0x90, 0xcc_u32] {
        t.w32(config + off, rng.word());
    }
    for off in [0xcc, 0x10dd0, 0x10dd4] {
        t.w32(scene + off, rng.word());
    }
    for off in [0x38, 0x50, 0x5c, 0x44] {
        t.w32(scene + off, pick(rng, &[0, 199, 200, 0xffff_ff00]));
    }
    for off in [0xfb18, 0xfb30, 0xfb24, 0x28, 0xfb48, 0xfb54] {
        t.w32(scene + off, pick(rng, &[0, 5, u32::MAX]));
        t.w32(scene + off + 4, pick(rng, &[0, 5, u32::MAX]));
    }
    // the lists' and flags' blocks non-zero, so a byte left unzeroed shows
    let blocks = t.zeros(0x1000 * 8);
    let fill = vec![0x5a; 0x1000 * 8];
    t.write(blocks, &fill);
    for (k, off) in [0xfb80u32, 0xfb94, 0xfba8, 0xfbbc, 0xfbd8]
        .into_iter()
        .enumerate()
    {
        t.w32(scene + off, pick(rng, &[0, 2999, 3000, u32::MAX]));
        t.w32(scene + off + 4, pick(rng, &[0, 2999, 3000]));
        t.w32(scene + off + 8, blocks + 0x1000 * k as u32);
    }
    t.stub_set_this(
        h56::RESERVE_FREE_LIST,
        1,
        rng.word(),
        &[(8, blocks + 0x5000)],
    );
    let count = pick(rng, &[0, 10, 3999, 4000]);
    let cap = pick(rng, &[0, 3999, 4000]);
    t.w32(scene + 0xfbf0, count);
    t.w32(scene + 0xfbf4, cap);
    // no data only while the array still grows (else both fault)
    let grows = count < 4000 && cap < 4000;
    let old = if grows && rng.chance(3) {
        0
    } else {
        blocks + 0x6000
    };
    t.w32(scene + 0xfbf8, old);
    let heap = t.zeros(0x1000 * 8);
    let fill = vec![0xa5; 0x1000 * 8];
    t.write(heap, &fill);
    let news: Vec<u32> = (0..8).map(|k| heap + 0x1000 * k).collect();
    t.stub_seq(h56::USER_MALLOC, false, 1, 0, &news);
    let aligned: Vec<u32> = (0..8).map(|_| rng.word()).collect();
    t.stub_seq(h56::ALIGNED_MALLOC, false, 2, 0, &aligned);
    let clusters: Vec<u32> = (0..4)
        .map(|_| if rng.chance(4) { 0 } else { rng.word() })
        .collect();
    t.stub_seq(h56::CLUSTER_CREATE, true, 3, 3, &clusters);
    // the large callees' stubs over their own bodies (the cave is too small)
    t.stub_here(h56::WORLD_INITIALIZE, true, 0, 0, rng.word());
    t.stub_here(h56::WORLD_ADD_CLUSTER, true, 1, 1, rng.word());
    t.stub_here(h56::LIGHTING_INITIALIZE, true, 1, 1, rng.word());
    t.stub_here(h56::BOOT_TEXT_CREATE, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[0].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[1].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[2].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[3].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[4].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[5].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[6].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[7].1, true, 0, 0, rng.word());
    t.stub_here(h56::TWEAKS[8].1, true, 0, 0, rng.word());
    t.stub_here(h56::SCREEN_EFFECTS[0].1, true, 5, 5, rng.word());
    t.stub_here(h56::SCREEN_EFFECTS[1].1, true, 5, 5, rng.word());
    t.stub_here(h56::SCREEN_EFFECTS[2].1, true, 5, 5, rng.word());
    t.stub_here(h56::MOTION_BLUR_INITIALIZE, true, 3, 3, rng.word());
    t.stub_here(h56::BROKEN_SCREEN_INITIALIZE, true, 7, 7, rng.word());
    t.stub_here(h56::FOG_INITIALIZE, true, 3, 3, rng.word());
    t.stub_here(h56::MORPH_INITIALIZE, true, 1, 1, rng.word());
    t.stub_here(h56::SAMPLER_REGISTER, true, 0, 0, rng.word());
    t.stub_here(h56::SETUP_SAMPLERS, true, 0, 0, rng.word());
    t.stub_here(h56::CREATE_RWTT_TARGET, true, 0, 0, rng.word());
    t.stub_here(h56::VIDEO_INITIALIZE, true, 3, 3, rng.word());
    t.stub_here(h56::VIDEO_CLOSE, true, 0, 0, rng.word());
}

impl Twins {
    /// A recording stub (as [`Twins::stub`]) written over the original's
    /// own body at `sym` instead of the cave (the stub is position
    /// independent; `sym`'s body must be longer than it, as the large
    /// initializers here are).
    fn stub_here(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let start = self.cave_next;
        let at = self.stub(None, sym, this, n, pops, Some(ret));
        let mut code = vec![0; (self.cave_next - at) as usize];
        Mem::read_bytes(&self.a, at, &mut code);
        self.cave_next = start;
        let site = self.rebase(sym);
        self.patch(site, &code);
    }
}

const BOOT58_STUBS: &[(u32, K)] = &[
    (h56::SCREEN_EFFECTS[0].0, K::Getter),
    (h56::SCREEN_EFFECTS[1].0, K::Getter),
    (h56::SCREEN_EFFECTS[2].0, K::Getter),
    (h56::MOTION_BLUR_INSTANCE, K::Getter),
    (h56::BROKEN_SCREEN_INSTANCE, K::Getter),
    (h56::FOG_INSTANCE, K::Getter),
    (h56::MORPH_INSTANCE, K::Getter),
    (h56::SPD_CTRL_INIT, K::Cdecl(0)),
    (h56::RESERVE_WORDS, K::This(1)),
    (h56::RESERVE_FLAGS, K::This(1)),
    (h56::RESERVE_LAYOUTS, K::This(1)),
    (h56::RESERVE_8080, K::This(1)),
    (h56::RESERVE_8000, K::This(1)),
    (h56::RESERVE_7F80, K::This(1)),
    (h56::RESERVE_8280, K::This(1)),
    (h56::USER_FREE, K::Cdecl(1)),
];

#[rustfmt::skip]
const CASES58: &[Case] = &[
    Case { prep: Some(prep_boot58), ..method("boot_init", 0x0065_7a50, This::Object, 0, BOOT58_STUBS, |m, this, _| u(gh56::boot_init(m, this))) },
];

#[test]
fn batch58_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 30 } else { 100 };
        run_cases(t, CASES58, n, &mut Rng(0x5858_b058));
    });
}

// ---- round 17 batch 59 (ffx_prender::h3_glue57) ----

impl Twins {
    /// A recording stub at `sym` (as [`Twins::stub`]) that first stores
    /// each `(k, value)`'s word through its stack argument `k`.
    fn stub_writes(
        &mut self,
        sym: u32,
        this: bool,
        n: u32,
        pops: u32,
        ret: u32,
        writes: &[(u8, u32)],
    ) {
        let log = self.stub(None, sym, this, n, pops, Some(ret));
        let mut c = vec![];
        for &(k, v) in writes {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp+4+4k]
            c.extend([0xc7, 0x00]); // mov dword [eax], v
            c.extend(v.to_le_bytes());
        }
        let at = self.cave_next;
        c.push(0xe9); // jmp log
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// A float, mostly a moderate one (hundredths in -20..20), so that one
/// NaN or infinity does not swamp a whole matrix.
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

/// An extent around the 30 the original compares with.
fn extent30(rng: &mut Rng) -> u32 {
    pick(
        rng,
        &[
            10.0_f32.to_bits(),
            30.0_f32.to_bits(),
            0x41f0_0001, // just above 30
            50.0_f32.to_bits(),
            0x7fc0_0000,
            (-40.0_f32).to_bits(),
        ],
    )
}

/// The manager (cells used or shared), the instance (none, or one whose
/// first word is 0 but for its top bit), its mesh's bounds and 3x4
/// matrix, the atlas size (0, negative, not a multiple of 8, the game's),
/// the cell lookup's and insertions' answers and cells, the bounding box
/// and casters' extents, the radius (0, -0, NaN, any), the outputs (apart
/// or one word).
fn prep_update_shadow(t: &mut Twins, _: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let manager = t.zeros(0x2c00);
    let fill = rng.bytes(0x2c00);
    t.write(manager, &fill);
    for cell in 0..64 {
        if rng.chance(2) {
            t.w32(manager + 0x29e8 + 4 * cell, 1);
        }
    }
    *this = manager;
    let inst = t.zeros(0x200);
    let fill = rng.bytes(0x200);
    t.write(inst, &fill);
    let any = rng.next() | 1;
    let first = pick(rng, &[0, 0x8000_0000, 1, any]);
    t.w32(inst, first);
    t.w32(inst + 4, inst + 0x20);
    t.w32(inst + 0x20, inst + 0x40);
    t.w32(inst + 0x40 + 0x1c, inst + 0x80);
    t.w32(inst + 0x40 + 4, inst + 0x100);
    for k in 0..8 {
        let v = mild(rng);
        t.w32(inst + 0x80 + 4 * k, v);
    }
    for k in 0..12 {
        let v = mild(rng);
        t.w32(inst + 0x100 + 4 * k, v);
    }
    let any = rng.next();
    let size = pick(rng, &[0, 2048, 4096, 1024, 13, (-9_i32) as u32, any]);
    t.global(h57::SHADOW_SIZE, size);
    let (col, row) = (rng.below(8), rng.below(8));
    let found = answer(rng);
    t.stub_writes(
        h57::ALREADY_IN_MAP,
        true,
        3,
        3,
        found,
        &[(1, col), (2, row)],
    );
    let (wide, deep) = (extent30(rng), extent30(rng));
    let up = rng.float().to_bits();
    let ret = rng.next();
    t.stub_writes(
        h57::MAX_BOUNDING_BOX,
        false,
        4,
        0,
        ret,
        &[(1, wide), (2, deep), (3, up)],
    );
    for sym in [h57::INSERT, h57::INSERT_DOUBLE] {
        let (col, row) = (rng.below(8), rng.below(8));
        let done = answer(rng);
        t.stub_writes(sym, true, 6, 6, done, &[(2, col), (3, row)]);
    }
    let ex = [mild(rng), mild(rng), mild(rng)];
    let ret = rng.next();
    t.stub_writes(
        h57::CASTERS_EXTENT,
        false,
        5,
        0,
        ret,
        &[(1, ex[0]), (2, ex[1]), (3, ex[2])],
    );
    let outs = t.zeros(0x40);
    args[0] = outs;
    args[1] = if rng.chance(8) { 0 } else { inst };
    if rng.chance(4) {
        for k in [3, 5, 6, 7] {
            args[k] = outs + 0x20;
        }
    } else {
        args[3] = outs + 0x10;
        args[5] = outs + 0x14;
        args[6] = outs + 0x18;
        args[7] = outs + 0x1c;
    }
    let any = mild(rng);
    args[4] = pick(
        rng,
        &[
            (-0.2_f32).to_bits(),
            (-1.0_f32).to_bits(),
            5.0_f32.to_bits(),
            any,
        ],
    );
    let any = mild(rng);
    args[8] = pick(rng, &[0, 0x8000_0000, 0x7fc0_0000, any, 2.5_f32.to_bits()]);
    args[9] = rng.float().to_bits();
}

#[rustfmt::skip]
const CASES59: &[Case] = &[
    framed(Case { prep: Some(prep_update_shadow), ..method("update_shadow_map", 0x006d_3000, This::Object, 12, &[], |m, this, a| { let slots = slots_at(m, a); u(gh57::update_shadow_map(m, frame_ebp(), slots, this)) }) }, 0x2b0, 0x30),
];

#[test]
fn batch59_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES59, n, &mut Rng(0x5959_b059));
    });
}

// ---- round 17 batch 60 (ffx_prender::h3_glue58) ----

/// A float in -2..2 now and then exactly -1, 0 or 1 (the shard
/// positions' screen test), or any float.
fn near_one(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => pick(
            rng,
            &[(-1.0_f32).to_bits(), 0, 0x8000_0000, 1.0_f32.to_bits()],
        ),
        1 => rng.float().to_bits(),
        _ => ((rng.below(400) as f32 - 200.0) / 100.0).to_bits(),
    }
}

/// The mode (diffuse, specular, neither), the vertex count (0 to 5), the
/// rotation, the positions, normals and colours, the light, colour and
/// strength.
fn prep_shards(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let mode = pick(rng, &[1, 2, 1, 2, 0, 3]);
    t.w32(obj, mode);
    let n = pick(rng, &[0, 1, 2, 3, 5]);
    t.w32(obj + 8, n);
    for k in 0..12 {
        let v = if rng.chance(3) {
            pick(rng, &[0, 1.0_f32.to_bits()])
        } else {
            mild(rng)
        };
        t.w32(obj + 0x60 + 4 * k, v);
    }
    let (positions, normals, colours) = (obj + 0x400, obj + 0x500, obj + 0x600);
    t.w32(obj + 0x40, positions);
    t.w32(obj + 0x44, normals);
    t.w32(obj + 0x48, colours);
    for k in 0..15 {
        let p = near_one(rng);
        t.w32(positions + 4 * k, p);
        let v = mild(rng);
        t.w32(normals + 4 * k, v);
    }
    if rng.chance(3) {
        // the first column's sum at an f32 tie: (1+2^-12)^2 + 2^-53 +
        // 2^-53 rounds down when added left to right, up otherwise
        t.w32(obj + 0x60, 0x3f80_0800);
        t.w32(obj + 0x70, 0x3280_0000);
        t.w32(obj + 0x80, 0x3280_0000);
        for v in 0..5 {
            t.w32(normals + 12 * v, 0x3f80_0800);
            t.w32(normals + 12 * v + 4, 0x3200_0000);
            t.w32(normals + 12 * v + 8, 0x3200_0000);
        }
    }
    for k in [0, 1, 2, 4, 5, 6] {
        args[k] = mild(rng);
    }
    let any = mild(rng);
    args[7] = pick(rng, &[1.0_f32.to_bits(), 0.25_f32.to_bits(), any]);
}

#[rustfmt::skip]
const CASES60: &[Case] = &[
    Case { prep: Some(prep_shards), ..method("shade_shards", 0x006d_3650, This::Object, 8, &[], |m, this, a| u(gh58::shade_shards(m, this, [a[0], a[1], a[2]], [a[4], a[5], a[6]], a[7]))) },
];

#[test]
fn batch60_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 400 };
        run_cases(t, CASES60, n, &mut Rng(0x6060_b060));
    });
}

// ---- round 17 batch 61 (ffx_prender::h3_glue59) ----

const INIT61_STUBS: &[(u32, K)] = &[
    (h59::UPDATE_VIEW_MATRICES, K::This(0)),
    (h59::MATRIX4X3_CLASS, K::Cdecl(0)),
    (h59::SAMPLER_STATES_INIT, K::Cdecl(0)),
    (h59::COPY_TABLE, K::This(0)),
    (h59::SETTINGS_SAVE, K::This(0)),
    (h59::INPUT_MANAGER, K::Cdecl(0)),
    (h59::BUILD_PAD_MAP, K::This(0)),
    (h59::IGGY_MENU_CREATE, K::Cdecl(0)),
    (h59::IGGY_MENU_INIT, K::This(3)),
    (h59::FIOS_UNIFY, K::Cdecl(3)),
    (h59::LOAD_PAUSE_MENU, K::Cdecl(1)),
    (h59::LOAD_ESC_MENU, K::Cdecl(5)),
    (h59::ICON_PANEL_CREATE, K::Cdecl(0)),
    (h59::AFTER_MENUS, K::Cdecl(0)),
];

/// The scene (its settings' bit 7 set or not), its cameras (pointers and
/// embedded ones, all with a vtable whose `+4` logs), the registration
/// (failing now and then), the instance list and its 3x4 matrix, the two
/// new matrices, the settings' pad-map byte, the application and the Iggy
/// menu manager.
fn prep_init61(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x11400);
    *this = scene;
    let fill = rng.bytes(0x40);
    t.write(scene + 0x940, &fill);
    let flags = rng.next();
    t.w32(scene + 0x95c, flags);
    let renderer = rng.next();
    t.w32(scene + 0xd0, renderer);
    t.w32(scene + 0x10ddc, rng.next());
    let vcall = t.stub(None, 0x4444, true, 0, 0, Some(0x1234));
    let vtable = obj + 0x1f00;
    t.w32(vtable + 4, vcall);
    for (k, slot) in [0xdc, 0x394, 0x238, 0x7e0, 0x654].into_iter().enumerate() {
        let camera = obj + 0x200 * k as u32;
        let fill = rng.bytes(0x200);
        t.write(camera, &fill);
        t.w32(camera, vtable);
        t.w32(scene + slot, camera);
    }
    for at in [0xbdc, 0xa84, 0x4fc] {
        t.w32(scene + at, vtable);
    }
    let main = t.r32(scene + 0xdc);
    let far = mild(rng);
    t.w32(main + 0xbc, far);
    let aspect = mild(rng);
    t.w32(main + 0x150, aspect);
    let error = if rng.chance(6) { rng.next() | 1 } else { 0 };
    t.stub_ret(h59::REGISTER, true, 0, 0, error);
    let list = obj + 0xc00;
    let matrix = obj + 0xc80;
    t.w32(list + 0x34, matrix);
    t.stub_ret(h59::ALLOCATE_INSTANCE_LIST, true, 3, 3, list);
    let clusters = [obj + 0xd00, obj + 0xd40];
    t.stub_seq(h59::CLUSTER_CREATE, true, 3, 3, &clusters);
    let settings = obj + 0xe00;
    t.write(settings + 0x638, &[u8::from(rng.chance(2))]);
    t.global(h59::SETTINGS, settings);
    let app = obj + 0x1000;
    let words = rng.bytes(0x280);
    t.write(app, &words);
    t.stub_ret(h59::GET_APPLICATION, false, 0, 0, app);
    let manager = rng.next();
    t.global(h59::IGGY_MENU_MANAGER, manager);
}

#[rustfmt::skip]
const CASES61: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_init61), ..method("init_scene", 0x0065_f450, This::Object, 0, INIT61_STUBS, |m, this, _| u(gh59::init_scene(m, frame_ebp(), this))) }), 0x1c8, 0),
];

#[test]
fn batch61_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 20 } else { 60 };
        run_cases(t, CASES61, n, &mut Rng(0x6161_b061));
    });
}

// ---- round 17 batch 62 (ffx_prender::h3_glue60) ----

const UI62_STUBS: &[(u32, K)] = &[
    (h60::REMAP_INSTANCE, K::Cdecl(0)),
    (h60::CHECK_WORD_VERTEX, K::This(3)),
];

/// A vertex buffer header at `at` (count, total, five arrays in `room`).
fn buffer62(t: &mut Twins, at: u32, room: u32, rng: &mut Rng) {
    let count = pick(rng, &[0, 4, 8, 12, 1, 7]);
    t.w32(at + 4, count);
    let total = rng.next();
    t.w32(at + 8, total);
    for k in 0..5 {
        t.w32(at + 0xc + 4 * k, room + 0x200 * k);
    }
}

/// The skipped frame, the remap (drawing, or replacing the name), the
/// kind (named or not), the name (none now and then) and which of the
/// six names `strstr` finds (or none), the vertex info's buffers (its
/// own and the three shifted ones), the quad's corners and colours.
fn prep_ui62(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let skip = if rng.chance(10) {
        pick(rng, &[1, 2, 0x100])
    } else {
        0
    };
    t.global(h60::SKIP_FRAME, skip);
    let renamed = obj + 0x1e00;
    let drawn = if rng.chance(6) {
        pick(rng, &[1, 0x100, 0x8000_0000])
    } else {
        0
    };
    let writes: &[(u8, u32)] = if rng.chance(4) { &[(3, renamed)] } else { &[] };
    t.stub_writes(h60::REMAP_BUTTON_ICON, true, 5, 5, drawn, writes);
    let info = obj + 0x40;
    let own = obj + 0x800;
    t.w32(info + 0x94, own);
    for off in [0u32, 0x6c, 0xd8, 0x144] {
        let room = t.zeros(0x1000);
        buffer62(t, own + off, room, rng);
    }
    t.stub_ret(h60::VERTEX_INFO, true, 2, 2, info);
    let hit = rng.below(7) as usize;
    let mut found = [0u32; 6];
    if hit < 6 {
        found[hit] = obj + 0x1f00;
    }
    t.stub_import_seq(h60::STRSTR_SLOT, 2, &found);
    let abmap = rng.next();
    t.global(h60::ABMAP, abmap);
    let quad = obj + 0x100;
    for v in 0..4 {
        let at = quad + 0x20 * v;
        for k in 0..4 {
            let x = mild(rng);
            t.w32(at + 4 * k, x);
        }
        for k in 0..4 {
            let c = if rng.chance(4) {
                rng.next()
            } else {
                rng.below(256)
            };
            t.w32(at + 0x10 + 4 * k, c);
        }
    }
    args[0] = quad;
    args[1] = if rng.chance(5) { 0 } else { obj + 0x1c00 };
    args[2] = pick(rng, &[6, 0xb, 6, 0xb, 0, 3]);
}

#[rustfmt::skip]
const CASES62: &[Case] = &[
    framed(Case { prep: Some(prep_ui62), ..cdecl("draw_ui_abmap_element", 0x0063_e930, 3, UI62_STUBS, |m, _, a| { let slots = slots_at(m, a); gh60::draw_ui_abmap_element(m, slots, a[0], a[2]); Out::None }) }, 0, 0xc),
];

#[test]
fn batch62_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES62, n, &mut Rng(0x6262_b062));
    });
}

// ---- round 17 batch 63 (ffx_prender::h3_glue61) ----

const GEO63_STUBS: &[(u32, K)] = &[
    (h61::UPDATE_RENDER_DATA, K::This(1)),
    (h61::MEMCPY, K::Cdecl(3)),
];

/// The flags, the render elements (none, one inline, several, or no
/// list), their parameter buffers (sizes around the offsets), what
/// `FindByName` answers call by call (no parameter, or one of several
/// offsets and sizes), the clamped `+0x4c` (NaN, the limits), the
/// scene's YIQ values.
fn prep_geo63(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let list = obj + 0x1000;
    let n = pick(rng, &[0, 1, 2, 3, 0x8000_0002]);
    t.w32(list, n);
    let data = if n & 0x7fff_ffff == 1 {
        list + 4
    } else {
        list + 0x40
    };
    if data != list + 4 {
        t.w32(list + 4, data);
    }
    for k in 0..3 {
        let element = obj + 0x1100 + 0x20 * k;
        t.w32(data + 4 * k, element);
        let buffer = obj + 0x1200 + 0x100 * k;
        t.w32(element + 4, buffer);
        let size = pick(rng, &[0x10, 0x14, 0x50, 0x2000, 0]);
        t.w32(buffer, size);
    }
    t.w32(obj + 0x14, if rng.chance(8) { 0 } else { list });
    let shader = rng.next();
    t.w32(obj + 0x28, shader);
    let lv = pick(
        rng,
        &[
            0x7fc0_0000,
            255.0_f32.to_bits(),
            (-256.0_f32).to_bits(),
            (-300.0_f32).to_bits(),
            (-255.5_f32).to_bits(),
            1e6_f32.to_bits(),
            3.5_f32.to_bits(),
        ],
    );
    t.w32(obj + 0x4c, lv);
    let params = obj + 0x1600;
    let shapes: [(u16, u16); 6] = [
        (0, 16),
        (3, 4),
        (4, 16),
        (8, 0x40),
        (0x40, 0x2010),
        (0x10, 4),
    ];
    for (k, (off, size)) in shapes.into_iter().enumerate() {
        let p = params + 0x10 * k as u32;
        t.write(p + 8, &off.to_le_bytes());
        t.write(p + 0xa, &size.to_le_bytes());
    }
    let answers: Vec<u32> = (0..48)
        .map(|_| {
            if rng.chance(3) {
                0
            } else {
                params + 0x10 * rng.below(6)
            }
        })
        .collect();
    t.stub_seq(h61::FIND_PARAMETER, true, 1, 1, &answers);
    let yiq: Vec<(u8, u32)> = (0..6).map(|k| (k, mild(rng))).collect();
    t.stub_writes(h61::GET_YIQ_PARAM, true, 6, 6, rng.next(), &yiq);
    let scene = rng.next();
    t.global(h61::SCENE, scene);
    args[0] = rng.next();
    // updateRenderData's result and the loop words
    FRAME_IGNORE.with(|z| z.replace(vec![(0x38, 0x14)]));
}

#[rustfmt::skip]
const CASES63: &[Case] = &[
    framed(with_cookie(Case { prep: Some(prep_geo63), ..method("update_render_data_without_adding", 0x006a_53e0, This::Object, 1, GEO63_STUBS, |m, this, a| u(gh61::update_render_data(m, frame_ebp(), this, a[0]))) }), 0x5c, 0),
];

#[test]
fn batch63_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES63, n, &mut Rng(0x6363_b063));
    });
}

// ---- round 18 batch 64 (ffx_prender::h3_glue62) ----

/// A scene for the video pass: `g_MovieFading` and the fade byte (each
/// set or not), the movie camera, picture and done flags, a camera (or
/// none when nothing reads its matrix), the post targets `+0x8c` and
/// `+0x90` (their textures there or not), the bytes `+0x93e`, `+0x78`,
/// `+0xf9c5`, `+0xf9c7`, `g_PhyreScene3D`, the word `+0x3860`, the UI
/// (none, or menus counted signed), the flag-0x1000 delegations' answer
/// with stray high bits, and the renderer calls failing now and then at
/// any step. Surfaces are the original's.
fn prep_video64(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    let flag = |rng: &mut Rng| if rng.chance(2) { 0 } else { rng.word() | 1 };
    let fading = if rng.chance(3) { rng.word() | 1 } else { 0 };
    t.global(h62::MOVIE_FADING, fading);
    let fade = u8::from(rng.chance(2));
    t.write(scene + 0x10dcc, &[fade]);
    let picture = flag(rng);
    t.global(h62::MOVIE_PICTURE_ON, picture);
    let done = flag(rng);
    t.global(h62::MOVIE_DONE, done);
    let have = t.rebase(h62::MOVIE_HAVE_CAMERA);
    let camera_answer = pick(rng, &[0, 0, 1, 0x100]);
    t.stub(
        Some(have),
        h62::MOVIE_HAVE_CAMERA,
        false,
        0,
        0,
        Some(camera_answer),
    );
    let post = t.zeros(0x1_2400);
    let at = t.rebase(h62::POST_INSTANCE);
    t.stub(Some(at), h62::POST_INSTANCE, false, 0, 0, Some(post));
    let fill = rng.bytes(0x200);
    t.write(post + 0x1_2300, &fill);
    let camera = obj + 0x500;
    let matrix: Vec<u8> = (0..16).flat_map(|_| mild(rng).to_le_bytes()).collect();
    t.write(camera + 0x34, &matrix);
    let none = done == 0 && rng.chance(4);
    t.w32(scene + 0xdc, if none { 0 } else { camera });
    let targets = t.zeros(0x200);
    t.w32(scene + 0x10cc, targets);
    for (k, off) in [0x8cu32, 0x90].into_iter().enumerate() {
        let slot = obj + 0x100 + 0x10 * k as u32;
        let texture = obj + 0x200 + 0x40 * k as u32;
        t.w32(targets + off, slot);
        t.w32(slot, texture);
        // the colour target always has its texture (the viewport reads it)
        let gone = if k != 0 && rng.chance(4) {
            pick(rng, &[1, 2, 0x80]) as u8
        } else {
            0
        };
        t.write(slot + 4, &[gone]);
        t.w32(texture + 0x1c, rng.word());
        t.w32(texture + 0x20, rng.word());
    }
    t.w32(scene + 0xd0, obj + 0x400);
    for off in [0x93e, 0x78, 0xf9c5, 0xf9c7] {
        let b = pick(rng, &[0, 0, 1, 0x80]) as u8;
        t.write(scene + off, &[b]);
    }
    let b = pick(rng, &[0, 0, 1]);
    t.global(h62::SCENE_3D, b | (rng.word() & !0xff));
    let pending = pick(rng, &[0, 0, 0, 1, 0x8000_0000]);
    t.w32(scene + 0x3860, pending);
    let ui = if rng.chance(4) { 0 } else { obj + 0x900 };
    t.global(h62::GRAPHIC_UI, ui);
    let counts: Vec<u32> = (0..2)
        .map(|_| pick(rng, &[0, 1, 3, u32::MAX, 0x8000_0000]))
        .collect();
    t.stub_seq(h62::MENU_COUNT, true, 0, 0, &counts);
    t.global(h62::DELEGATION_HELPER, obj + 0x780);
    let drawn = [answer(rng)];
    t.stub_seq(h62::RUN_FLAG_1000, true, 1, 1, &drawn);
    // now and then the movie render starts a fade (read again after it)
    if rng.chance(3) {
        let at = t.rebase(h62::MOVIE_FADING);
        let (ret, value) = (rng.word(), rng.word() | 1);
        t.stub_store(h62::FMV_RENDER, 1, ret, at, value);
    }
    let r = results54(rng, 6);
    t.stub_seq(h62::SET_COLOR_TARGET, true, 2, 2, &r);
    let r = results54(rng, 6);
    t.stub_seq(h62::SET_DEPTH_STENCIL_TARGET, true, 1, 1, &r);
    let r = results54(rng, 5);
    t.stub_seq(h62::BEGIN_SCENE, true, 1, 1, &r);
    let r = results54(rng, 5);
    t.stub_seq(h62::END_SCENE, true, 1, 1, &r);
}

const VIDEO64_STUBS: &[(u32, K)] = &[
    (h62::FMV_INSTANCE, K::Getter),
    (h62::FMV_RENDER, K::This(1)),
    (h62::PRE_RENDER, K::This(0)),
    (h62::RENDER_SHADOW, K::This(0)),
    (h62::DRAW_MOST_VFX, K::This(0)),
    (h62::MINIMAP_STENCIL, K::This(0)),
    (h62::DRAW_UI_2D_VFX, K::This(0)),
    (h62::DRAW_NORMAL_UI, K::This(0)),
    (h62::DRAW_3D_WORLD, K::This(1)),
    (h62::SET_CLEAR_COLOR, K::This(4)),
    (h62::SET_CLEAR_STENCIL, K::This(1)),
    (h62::SET_VIEWPORT, K::This(4)),
    (h62::UPDATE_VIEW_MATRICES, K::This(0)),
    (h62::RENDER_SCREEN_FADE, K::This(4)),
    (h62::INSTANCE_6DC800, K::Getter),
    (h62::VIDEO_TEXT_RENDER, K::This(1)),
];

#[rustfmt::skip]
const CASES64: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_video64), ..method("render_video_and_its_ui", 0x0065_3960, This::Object, 0, VIDEO64_STUBS, |m, this, _| u(gh62::render_video_and_its_ui(m, frame_ebp(), this))) }), 0xa0, 0),
];

#[test]
fn batch64_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES64, n, &mut Rng(0x6464_b064));
    });
}

// ---- round 18 batch 65 (ffx_prender::h3_glue63) ----

/// A scene for the HDAO pass: its two enable bytes (now and then
/// clear), the colour target (now and then none) and the depth target
/// (their textures there or not), the matrix `+0xfc04` and the extents
/// `+0xfc34..+0xfc5c` (mild floats), the swap code `+0xfaf4`, a map
/// cluster or none with the map tree of [`map_tree`] (walked twice: the
/// successors run on to the head for each walk; `memcpy` is the
/// original's), the world kind `+0xf9cc`, the scene's camera or none.
/// The camera's construction (but its projection update, which logs the
/// vtable it finds) and its matrices are the original's.
fn prep_hdao65(t: &mut Twins, obj: u32, this: &mut u32, _: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1_1000);
    *this = scene;
    for off in [0xfc00, 0xfc01] {
        let b = if rng.chance(8) {
            0
        } else {
            pick(rng, &[1, 0x80]) as u8
        };
        t.write(scene + off, &[b]);
    }
    let targets = t.zeros(0x200);
    for (k, off) in [0xfc70u32, 0xfc74].into_iter().enumerate() {
        let slot = targets + 0x10 * k as u32;
        let texture = targets + 0x80 + 0x40 * k as u32;
        let none = k == 0 && rng.chance(8);
        t.w32(scene + off, if none { 0 } else { slot });
        t.w32(slot, texture);
        let gone = u8::from(k != 0 && rng.chance(4));
        // the byte after the flag set, so a port reading it shows
        t.write(slot + 4, &[gone, 0x80]);
        t.w32(texture + 0x1c, rng.word());
        t.w32(texture + 0x20, rng.word());
    }
    for k in 0..16 {
        let v = mild(rng);
        t.w32(scene + 0xfc04 + 4 * k, v);
    }
    for at in (0xfc34..0xfc60).step_by(4) {
        let v = mild(rng);
        t.w32(scene + at, v);
    }
    let code = pick(rng, &[0xc04, 0xb48, 0, 0xc05]);
    t.w32(scene + 0xfaf4, code);
    let renderer = t.zeros(0x100);
    t.w32(scene + 0xd0, renderer);
    t.w32(scene + 0xdc, if rng.chance(4) { 0 } else { obj + 0x1f00 });
    let kind = pick(rng, &[0, 1, 2, 3]);
    t.w32(scene + 0xf9cc, kind);
    t.w32(scene + 0x10d4, if rng.chance(4) { 0 } else { obj });
    for k in 0..4 {
        let v = mild(rng);
        t.w32(scene + 0xfc78 + 4 * k, v);
    }
    map_tree(t, obj, scene, rng);
    // two walks: each runs its successors on to the head
    let head = scene + 0x1198;
    let mut order = vec![];
    for _ in 0..2 {
        for _ in 0..rng.below(4) {
            order.push(obj + 0x100 + 0x40 * rng.below(3));
        }
        order.push(head);
    }
    order.extend([head; 4]);
    t.stub_seq(h63::TREE_SUCCESSOR, true, 1, 1, &order);
    // buffer sizes some parameters fill exactly (offset + size)
    for m in 0..3 {
        let size = pick(rng, &[0x10, 0x2c, 0x30, 0x4c, 0x50, 0x80]);
        t.w32(obj + 0x1000 + 0x100 * m, size);
    }
    // the walks' counters and the x87 temporaries
    FRAME_IGNORE.with(|z| z.replace(vec![(0x20, 0x10), (0xb0, 4)]));
}

const HDAO65_STUBS: &[(u32, K)] = &[
    // logs the vtable it finds (the orthographic camera's)
    (h63::ORTHO_UPDATE, K::KeepDeref(0)),
    (h63::UPDATE_VIEW_MATRICES, K::This(0)),
    (h63::PHYRE_FREE, K::Cdecl(1)),
    (h63::SET_CLEAR_DEPTH, K::This(1)),
    (h63::SET_CAMERA, K::This(1)),
    (h63::SET_VIEWPORT, K::This(4)),
    (h63::SET_COLOR_TARGET, K::This(2)),
    (h63::SET_DEPTH_STENCIL_TARGET, K::This(1)),
    (h63::SET_CLEAR_COLOR, K::This(4)),
    (h63::BEGIN_SCENE, K::This(1)),
    (h63::END_SCENE, K::This(1)),
    (h63::GET_APPLICATION, K::Getter),
    (h63::RENDER_WORLD, K::This(3)),
];

#[rustfmt::skip]
const CASES65: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_hdao65), ..method("render_hdao", 0x0066_b560, This::Object, 0, HDAO65_STUBS, |m, this, _| u(gh63::render_hdao(m, frame_ebp(), this))) }), 0x2e8, 0),
];

#[test]
fn batch65_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES65, n, &mut Rng(0x6565_b065));
    });
}

// ---- round 18 batch 66 (ffx_prender::h3_glue64) ----

/// The asset array (count, capacity: room or not, signed; data, or none
/// where the slot address comes out 0), the asset (or none), the model
/// answer with stray high bits, the scene's symbol (set or not), the
/// `scaled` byte (with high bits), and the model cluster's instances
/// through two cursor scripts: each instance a 3x4 matrix, an owner and a
/// vector. The matrix math is the original's; the new instance list
/// holds a buffer for the 3x4.
fn prep_asset66(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x200);
    *this = scene;
    t.w32(scene + 0xd8, if rng.chance(5) { rng.word() | 1 } else { 0 });
    args[0] = if rng.chance(6) { 0 } else { rng.word() };
    let model = t.zeros(4);
    args[2] = model;
    let assets = t.zeros(0x10);
    let count = pick(rng, &[0, 1, 3, 8]);
    let room = pick(rng, &[count + 1, count, 0, u32::MAX, 0x7fff_ffff]);
    let data = if count == 0 && rng.chance(4) {
        0
    } else {
        t.zeros(0x40)
    };
    t.w32(assets, count);
    t.w32(assets + 4, room);
    t.w32(assets + 8, data);
    args[3] = assets;
    args[4] = obj + 0x40;
    args[6] = pick(rng, &[0, 1, 0x100, 0xff01]);
    let model_answer = [answer(rng)];
    t.stub_seq(h64::IS_MODEL, true, 1, 1, &model_answer);
    let list = t.zeros(0x40);
    let placed = t.zeros(0x40);
    t.w32(list + 0x34, placed);
    t.stub_ret(h64::ALLOCATE_INSTANCE_LIST, true, 3, 3, list);
    let area = t.zeros(0x800);
    for k in 0..8 {
        let object = area + 0x100 * k + 4;
        let matrix = t.zeros(0x40);
        let words: Vec<u8> = (0..16).flat_map(|_| mild(rng).to_le_bytes()).collect();
        t.write(matrix, &words);
        t.w32(object + 4, matrix);
        let owner = t.zeros(0x20);
        t.w32(object + 0x1c, owner);
        let vector = t.zeros(0x40);
        let words: Vec<u8> = (0..16).flat_map(|_| mild(rng).to_le_bytes()).collect();
        t.write(vector, &words);
        t.w32(object + 0xb0, vector);
    }
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script_args(h64::CURSOR_CTOR, 1, h64::CURSOR_NEXT, &steps);
    let steps = cursor_steps(t, area, false, rng);
    t.stub_cursor_script_args(h64::CURSOR_CTOR_SEEK2, 1, h64::CURSOR_NEXT3, &steps);
}

const ASSET66_STUBS: &[(u32, K)] = &[
    (h64::ARRAY_RESERVE, K::This(1)),
    (h64::GET_SYMBOL_BY_ID, K::This(1)),
    (h64::MATRIX4X3_TYPE, K::Cdecl(0)),
];

#[rustfmt::skip]
const CASES66: &[Case] = &[
    framed(Case { prep: Some(prep_asset66), ..method("do_on_field_map_asset_loaded", 0x0065_af60, This::Object, 7, ASSET66_STUBS, |m, this, a| u(gh64::do_on_field_map_asset_loaded(m, frame_ebp(), this, a[0], a[2], a[3], a[4], a[5], a[6]))) }, 0x1c0, 0),
];

#[test]
fn batch66_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES66, n, &mut Rng(0x6666_b066));
    });
}

// ---- round 19 batch 67 (ffx_prender::h3_glue65) ----

impl Twins {
    /// `InterlockedExchangeAdd` through `slot`, not logged: the original
    /// does its `lock xadd` inline.
    fn stub_xadd(&mut self, slot: u32) {
        // mov ecx, [esp+4]; mov eax, [esp+8]; lock xadd [ecx], eax; ret 8
        let c = [
            0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x44, 0x24, 0x08, 0xf0, 0x0f, 0xc1, 0x01, 0xc2, 0x08,
            0x00,
        ];
        let code = self.code(&c);
        let at = self.rebase(slot);
        self.patch(at, &code.to_le_bytes());
    }
}

/// The texture video file: its effect count (signed; none, some or
/// negative), then each effect's `n` positions and `m` of the other four
/// arrays (`m` vec3s, vec4s, vec2s and halves). The last effect's counts
/// are now and then large enough for the sizes' `mul` to overflow
/// (negative `n` skips the bounds' loop).
fn video_data67(rng: &mut Rng) -> Vec<u8> {
    let count = pick(rng, &[0, 1, 1, 2, 3, 0x8000_0002]);
    let mut d = count.to_le_bytes().to_vec();
    let real = if (count as i32) > 0 { count } else { 0 };
    for k in 0..real {
        let last = k + 1 == real;
        let n = if last && rng.chance(6) {
            pick(rng, &[0x8000_0001, 0xc000_0000])
        } else {
            rng.below(6)
        };
        let m = if last && rng.chance(6) {
            pick(rng, &[0x4000_0001, 0x8000_0001, 0x2000_0001])
        } else {
            rng.below(4)
        };
        d.extend(n.to_le_bytes());
        d.extend(m.to_le_bytes());
        let positions = if (n as i32) >= 0 { 3 * n } else { 3 };
        for _ in 0..positions {
            // now and then a quiet NaN or a zero of either sign (no signalling
            // NaNs: the oracle's fld keeps them, the hardware quiets them)
            let v = if rng.chance(4) {
                pick(rng, &[0x7fc0_0000, 0xffc0_0000, 0, 0x8000_0000])
            } else {
                mild(rng)
            };
            d.extend(v.to_le_bytes());
        }
        let rest = if m < 8 {
            9 * m as usize * 4 + 2 * m as usize
        } else {
            16
        };
        d.extend(rng.bytes(rest));
    }
    d.extend(rng.bytes(16));
    d
}

/// A scene for the texture video load: the scenario flag (0x194 with
/// stray high bits or not) and event jump (0x15d or not), the video
/// (none, 0x6b, 200 or another) and its loop flag, loaded or not, its
/// name found or not; the map in the path (the `strstr`s' answers in
/// turn), the file read in chunks (then 0 or negative), the three
/// textures (their buffer counts 3, with the top bit, or not; bound or
/// not; the count change failing or not), the movie's open answering -3
/// (the error message: a resource or none, the conversion inline or
/// not, the string's last reference or not) or not, and the file's
/// effects with their render data. `memcpy` only records; the file's
/// contents are the buffer's from the start.
fn prep_video67(t: &mut Twins, obj: u32, this: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let scene = t.zeros(0x1200);
    *this = scene;
    args[0] = obj + 0x40;
    let r = rng.word();
    let flag = pick(rng, &[0x194, 0x194, 0x1_0194, 0x195, r]);
    t.stub_ret(h65::GET_SCENARIO_FLAG, false, 0, 0, flag);
    let jump = pick(rng, &[0x15d, 0x15d, 0x1_015d, 3]);
    t.stub_ret(h65::EVENT_JUMP_ID, false, 0, 0, jump);
    let video = pick(rng, &[u32::MAX, 5, 200, 0x6b, 7, 7]);
    t.w32(scene + 0x10c4, video);
    let records = rng.bytes(0x68);
    t.write(scene + 0xd60, &records);
    let bytes = rng.bytes(8);
    t.write(scene + 0x10c8, &bytes);
    t.write(scene + 0xdd0, &[u8::from(rng.chance(8))]);
    t.write(scene + 0xdd1, &[0xa5]);
    let r = rng.word();
    let found = pick(rng, &[1, 0x100, r | 1, r | 1]);
    t.stub_here(h65::GET_VIDEO_NAME, true, 2, 2, found);
    let hits: Vec<u32> = (0..3)
        .map(|_| if rng.chance(3) { rng.word() | 1 } else { 0 })
        .collect();
    t.stub_import_seq(h65::STRSTR_IAT, 2, &hits);
    t.stub_here(h65::FIOS_OPEN, false, 2, 0, rng.word());
    let mut reads: Vec<u32> = (0..rng.below(4)).map(|_| 1 + rng.below(0x3ff)).collect();
    reads.push(pick(rng, &[0, u32::MAX, 0x8000_0000]));
    t.stub_seq(h65::FIOS_READ, false, 3, 0, &reads);
    let data = video_data67(rng);
    let buffer = t.alloc(&data);
    let mut news = vec![buffer];
    news.extend((0..20).map(|_| rng.word()));
    t.stub_seq(h65::ARRAY_NEW, false, 1, 0, &news);
    let mut mallocs = vec![];
    for _ in 0..3 {
        let texture = t.zeros(0x80);
        let count = pick(rng, &[3, 3, 0x8000_0003, 0, 1, 0x8000_0000]);
        t.w32(texture + 0x24, count);
        t.w32(texture + 0x14, rng.word());
        t.w32(texture + 8, rng.word());
        mallocs.push(texture);
    }
    t.stub_seq(h65::PHYRE_MALLOC, false, 1, 0, &mallocs);
    let bound: Vec<u32> = (0..3).map(|_| answer(rng)).collect();
    t.stub_seq(h65::TEXTURE_IS_BOUND, true, 0, 0, &bound);
    let set: Vec<u32> = (0..3).map(|_| pick(rng, &[0, 0, 1, 0x100])).collect();
    t.stub_seq(h65::SET_BUFFER_COUNT, true, 1, 1, &set);
    let r = rng.word();
    let opened = pick(rng, &[0xffff_fffd, 0xffff_fffd, 0, 1, r]);
    t.stub_here(h65::VIDEO_OPEN, true, 2, 2, opened);
    // the open error's strings
    let module = if rng.chance(3) { 0 } else { rng.word() | 1 };
    t.stub_here(h65::FIND_RESOURCE, false, 2, 0, module);
    let other = obj + 0x700;
    let stores: &[(u8, u32)] = if rng.chance(2) { &[(0, other)] } else { &[] };
    t.stub_set_this(h65::A2W, 2, rng.word(), stores);
    let header = t.zeros(0x20);
    let owner = t.zeros(0x10);
    let owner_vtable = t.zeros(0x10);
    let free = t.stub(None, 0xf4ee, true, 1, 1, Some(0));
    t.w32(owner_vtable + 4, free);
    t.w32(owner, owner_vtable);
    t.w32(header, owner);
    t.w32(header + 0xc, pick(rng, &[0, 1, 2, 5]));
    let nil = t.stub(None, 0x0011_0000, true, 0, 0, Some(header));
    let strings_vtable = t.zeros(0x20);
    t.w32(strings_vtable + 0xc, nil);
    t.global(h65::STRING_MANAGER, strings_vtable);
    t.stub_xadd(h65::INTERLOCKED_ADD_IAT);
    // the effects' render data
    let renders: Vec<u32> = (0..4)
        .map(|_| {
            let render = t.zeros(0xc0);
            let sub = t.zeros(0x40);
            t.w32(render + 0x94, sub);
            render
        })
        .collect();
    t.stub_seq(h65::VFX_RENDER_DATA, false, 2, 0, &renders);
    // the larger callees' stubs over their own bodies (the cave is small)
    for (sym, this, n) in [
        (h65::FIOS_UNIFY, false, 3),
        (h65::SET_DIMENSIONS, true, 4),
        (h65::TEXTURE_CTOR, true, 0),
        (h65::VFX_CREATE, false, 3),
        (h65::VFX_CREATE_DATA_BUFFER, false, 7),
        (h65::VFX_SETUP, false, 2),
        (h65::LOAD_STRING, true, 2),
        (h65::STRING_REPLACE, true, 2),
    ] {
        let pops = if this { n } else { 0 };
        t.stub_here(sym, this, n, pops, rng.word());
    }
}

const VIDEO67_STUBS: &[(u32, K)] = &[
    (h65::FMV_INSTANCE, K::Getter),
    (h65::PHYRE_PRINTF, K::Cdecl(2)),
    (h65::FIOS_CLOSE, K::Cdecl(1)),
    (h65::MEMCPY, K::Cdecl(3)),
    (h65::ARRAY_DELETE, K::Cdecl(1)),
    (h65::UPDATE_MAX_LEVEL, K::This(0)),
    (h65::ON_TEXTURE_VIDEO_PLAY, K::This(0)),
    (h65::VFX_SET_TYPE, K::Cdecl(1)),
    (h65::VFX_SET_VISIBLE, K::Cdecl(3)),
    (h65::RESET_TO_LAUNCHER, K::Cdecl(0)),
    (h65::FREE_IAT, K::ImportC(1)),
    (h65::SHOW_CURSOR_IAT, K::Import(1)),
    (h65::MESSAGE_BOX_IAT, K::Import(4)),
];

#[rustfmt::skip]
const CASES67: &[Case] = &[
    framed(with_seh(Case { prep: Some(prep_video67), ..method("texture_video_load", 0x0066_fb70, This::Object, 1, VIDEO67_STUBS, |m, this, a| { gh65::texture_video_load(m, frame_ebp(), this, a[0]); Out::None }) }), 0xb60, 0),
];

#[test]
fn batch67_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES67, n, &mut Rng(0x6767_b067));
    });
}

// ---- round 19 batch 68 (ffx_prender::h3_glue66) ----

const UI68_STUBS: &[(u32, K)] = &[
    (h66::REMAP_INSTANCE, K::Cdecl(0)),
    (h66::UI_ELEMENT_REMAP, K::Cdecl(2)),
    (h66::DRAW_UI_ABMAP_ELEMENT, K::Cdecl(3)),
];

/// The skipped frame; the ability map (none, or up with its `+4` below 2
/// or not, signed); the remap (drawing, or replacing the name in the
/// frame slot); the vertex record's buffer (small counts: a negative
/// one sends the index offsets off the heap) and the dynamic geometry's
/// current buffer id; the quad's corners and colours
/// (in and out of 0..255); the name (none now and then), `screen`, the
/// kinds (1, 2, 3, 4, `kind % 5 == 1`, negatives) and the vertex kind.
fn prep_ui68(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let skip = if rng.chance(10) {
        pick(rng, &[1, 2, 0x100])
    } else {
        0
    };
    t.global(h66::SKIP_FRAME, skip);
    let abmap = if rng.chance(3) {
        let abmap = t.zeros(0x10);
        let state = pick(rng, &[0, 1, 2, 3, 0xffff_ffff, 0x8000_0000]);
        t.w32(abmap + 4, state);
        abmap
    } else {
        0
    };
    t.global(h66::ABMAP, abmap);
    let renamed = obj + 0x1e00;
    let drawn = if rng.chance(6) {
        pick(rng, &[1, 0x100, 0x8000_0000])
    } else {
        0
    };
    let writes: &[(u8, u32)] = if rng.chance(4) { &[(3, renamed)] } else { &[] };
    t.stub_writes(h66::REMAP_BUTTON_ICON, true, 5, 5, drawn, writes);
    let info = t.zeros(0x100);
    let fill = rng.bytes(0x100);
    t.write(info, &fill);
    let buffer = t.zeros(0x100);
    t.w32(info + 0x94, buffer);
    let count = pick(rng, &[0, 4, 8, 12, 1, 7, 0x40, 0x3f]);
    t.w32(buffer + 4, count);
    let total = rng.next();
    t.w32(buffer + 8, total);
    let room = t.zeros(0x2000);
    for k in 0..5 {
        t.w32(buffer + 0xc + 4 * k, room + 0x400 * k);
    }
    t.stub_ret(h66::MENU_VERTEX_INFO, true, 2, 2, info);
    t.global(h66::GRAPHIC_UI, obj + 0x600);
    t.global(h66::DYN_GEO_FILL, obj + 0x700);
    let id = pick(rng, &[0, 1, 2]);
    t.stub_ret(h66::CURRENT_BUFFER_ID, true, 0, 0, id);
    let quad = obj + 0x100;
    for v in 0..4 {
        let at = quad + 0x20 * v;
        for k in 0..4 {
            let x = mild(rng);
            t.w32(at + 4 * k, x);
        }
        for k in 0..4 {
            let c = if rng.chance(4) {
                rng.next()
            } else {
                rng.below(256)
            };
            t.w32(at + 0x10 + 4 * k, c);
        }
    }
    args[0] = quad;
    args[1] = if rng.chance(5) { 0 } else { obj + 0x1c00 };
    args[2] = if rng.chance(3) { rng.word() | 1 } else { 0 };
    args[3] = pick(
        rng,
        &[
            1,
            1,
            2,
            2,
            3,
            3,
            4,
            4,
            0,
            5,
            6,
            11,
            0xffff_fffc,
            0xffff_fff9,
            0xffff_fffb,
            7,
        ],
    );
}

#[rustfmt::skip]
const CASES68: &[Case] = &[
    with_cookie(framed(Case { prep: Some(prep_ui68), ..cdecl("draw_ui_element", 0x0063_eee0, 5, UI68_STUBS, |m, _, a| { gh66::draw_ui_element(m, frame_ebp(), a[0], a[1], a[2], a[3], a[4]); Out::None }) }, 0x9c, 0)),
];

#[test]
fn batch68_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES68, n, &mut Rng(0x6868_b068));
    });
}

// ---- round 19 batch 69 (ffx_prender::h3_glue67) ----

impl Twins {
    /// A recording `thiscall(this; i, &out)` stub (`ret 8`) that stores
    /// `ptr` and `mapped` through `out` and returns `ret`s in turn.
    fn stub_map69(&mut self, sym: u32, ptr: u32, mapped: u32, rets: &[u32]) {
        let words: Vec<u8> = rets.iter().flat_map(|v| v.to_le_bytes()).collect();
        let table = self.alloc(&words);
        let cell = self.alloc(&table.to_le_bytes());
        let log = self.stub(None, sym, true, 2, 2, None);
        let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
        c.extend([0xc7, 0x00]);
        c.extend(ptr.to_le_bytes()); // mov [eax], ptr
        c.extend([0xc7, 0x40, 0x04]);
        c.extend(mapped.to_le_bytes()); // mov [eax+4], mapped
        c.extend([0x8b, 0x15]);
        c.extend(cell.to_le_bytes()); // mov edx, [cell]
        c.extend([0x8b, 0x02]); // mov eax, [edx]
        c.extend([0x83, 0xc2, 0x04]); // add edx, 4
        c.extend([0x89, 0x15]);
        c.extend(cell.to_le_bytes()); // mov [cell], edx
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A recording `memset` (cdecl, 3 words) that clears too.
    fn stub_memset69(&mut self, sym: u32) {
        let log = self.stub(None, sym, false, 3, 0, Some(0));
        // push edi; mov edi, [esp+8]; mov eax, [esp+0xc]; mov ecx, [esp+0x10];
        // rep stosb; pop edi; jmp log
        let mut c = vec![
            0x57, 0x8b, 0x7c, 0x24, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x4c, 0x24, 0x10, 0xf3,
            0xaa, 0x5f,
        ];
        let at = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }

    /// A recording cdecl stub at `sym` (`n` words, not popped) that
    /// first stores `value` at the absolute address `at`.
    fn stub_store_c(&mut self, sym: u32, n: u32, at: u32, value: u32) {
        let log = self.stub(None, sym, false, n, 0, Some(0));
        let mut c = vec![0xc7, 0x05]; // mov dword [at], value
        c.extend(at.to_le_bytes());
        c.extend(value.to_le_bytes());
        let here = self.cave_next;
        c.push(0xe9);
        c.extend(log.wrapping_sub(here + c.len() as u32 + 4).to_le_bytes());
        let code = self.code(&c);
        let site = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(site + 5).to_le_bytes());
        self.patch(site, &j);
    }
}

/// One vertex block at `block` (0x40 bytes): its attributes (positions
/// of either name, colours of 16, 8, 4 or 6 bytes, others), the stride
/// (their sizes, sometimes more), its buffer index and entry array (one
/// inline entry, or an array of two), pending 0 or (once per case) 1.
fn block69(t: &mut Twins, block: u32, rng: &mut Rng, pending: &mut Option<u32>) -> bool {
    let n = rng.below(4);
    let attrs = t.zeros(0x40);
    let mut offset = 0;
    let mut positions = false;
    for k in 0..n {
        let (size, name) = match rng.below(6) {
            0 => {
                positions = true;
                (12, t.rebase(h67::NAME_POSITION_A))
            }
            1 => {
                positions = true;
                (12, t.rebase(h67::NAME_POSITION_B))
            }
            5 => (4, rng.word()),
            _ => (pick(rng, &[16, 8, 4, 6]), t.rebase(h67::NAME_COLOR)),
        };
        t.w32(attrs + 0xc * k, offset);
        t.w32(attrs + 0xc * k + 4, name);
        offset += size;
    }
    let stride = offset + pick(rng, &[0, 0, 4]);
    let stride = if stride == 0 { 4 } else { stride };
    t.w32(block, stride);
    t.w32(block + 8, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
    t.w32(block + 0xc, attrs);
    let entries = pick(rng, &[1, 2, 0x8000_0002]);
    t.w32(block + 0x18, entries);
    let index = if entries == 1 { 0 } else { rng.below(2) };
    t.w32(block + 0x10, index);
    let array = if entries == 1 {
        block + 0x1c
    } else {
        let a = t.zeros(0x40);
        t.w32(block + 0x1c, a);
        a
    };
    let at = array + 20 * index + 0x10;
    let wait = pending.is_none() && rng.chance(4);
    t.w32(at, u32::from(wait));
    if wait {
        *pending = Some(at);
    }
    positions
}

/// A character instance: 0-2 meshes (stray top bits), each 0-2 elements
/// (the pair halves set or 0xffff, 0-2 blocks); the CPU side new or
/// given (with lists and entries or none); the allocations (the heap
/// failing now and then); the buffer bound or made or not; the maps'
/// answers and the mapped data (small sizes, halves and bytes random);
/// the cloth list (free slots or full).
#[allow(clippy::too_many_lines)] // one structure per level
fn prep_cloth69(t: &mut Twins, obj: u32, _: &mut u32, args: &mut [u32], rng: &mut Rng) {
    let instance = t.zeros(0x20);
    let meshes = rng.below(3);
    t.w32(
        instance,
        meshes | if rng.chance(4) { 0x8000_0000 } else { 0 },
    );
    let list = t.zeros(0x10);
    t.w32(instance + 4, list);
    let mut pending = None;
    let mut positions = false;
    for i in 0..meshes {
        let slot = t.zeros(4);
        let object = t.zeros(0x10);
        t.w32(slot, object);
        t.w32(list + 4 * i, slot);
        let elements = rng.below(3);
        t.w32(
            object,
            elements | if rng.chance(4) { 0x8000_0000 } else { 0 },
        );
        let base = t.zeros(0x6c * 2);
        t.w32(object + 4, base);
        for j in 0..elements {
            let e = base + 0x6c * j;
            let a = if rng.chance(6) {
                0xffff
            } else {
                rng.below(0x100)
            };
            let b = if rng.chance(6) {
                0xffff
            } else {
                rng.below(0x100)
            };
            t.write(e + 0x20, &(a as u16).to_le_bytes());
            t.write(e + 0x24, &(b as u16).to_le_bytes());
            let blocks = rng.below(3);
            t.w32(
                e + 0x28,
                blocks | if rng.chance(4) { 0x8000_0000 } else { 0 },
            );
            let bs = t.zeros(0x80);
            t.w32(e + 0x2c, bs);
            for k in 0..blocks {
                positions |= block69(t, bs + 0x40 * k, rng, &mut pending);
            }
        }
    }
    if let Some(at) = pending {
        t.stub_store_c(h67::SLEEP, 1, at, 0);
    }
    let out = obj + 0x40;
    let cpu = if rng.chance(2) {
        0
    } else {
        let cpu = t.zeros(0x20);
        for i in 0..2 {
            if rng.chance(3) {
                continue;
            }
            let cs = t.zeros(0x20);
            t.w32(cpu + 4 + 4 * i, cs);
            for j in 0..2 {
                if rng.chance(2) {
                    let e = t.zeros(0x10);
                    t.w32(e, rng.below(6));
                    t.w32(cs + 4 + 4 * j, e);
                }
            }
        }
        cpu
    };
    t.w32(out, cpu);
    args[0] = if rng.chance(12) { 0 } else { instance };
    args[1] = out;
    let heap: Vec<u32> = (0..48)
        .map(|_| {
            if rng.chance(6) {
                0
            } else {
                let fill = rng.bytes(0x240);
                t.alloc(&fill)
            }
        })
        .collect();
    t.stub_seq(h67::MSPACE_MEMALIGN, false, 3, 0, &heap);
    let user: Vec<u32> = (0..48)
        .map(|_| {
            let fill = rng.bytes(0x240);
            t.alloc(&fill)
        })
        .collect();
    t.stub_memset69(h67::MEMSET);
    t.stub_seq(h67::ALIGNED_MALLOC, false, 2, 0, &user);
    let bound: Vec<u32> = (0..24).map(|_| answer(rng)).collect();
    t.stub_seq(h67::VB_IS_BOUND, true, 0, 0, &bound);
    // a skipped map leaves the pointer 0 (junk read): only where nothing
    // raises the colour count above the mapped one (a new CPU side
    // without position streams)
    let skips = if cpu == 0 && !positions {
        &[0, 1, 9][..]
    } else {
        &[0][..]
    };
    let made: Vec<u32> = (0..24).map(|_| pick(rng, skips)).collect();
    t.stub_seq(h67::VB_CREATE, true, 0, 0, &made);
    let data = t.alloc(&rng.bytes(0x100));
    // now and then halves at infinity (65536 exactly, not above) and next
    // to it
    if rng.chance(2) {
        let halves: Vec<u8> = [0x7c00_u16, 0xfc00, 0x7bff, 0x7c01, 0x0001, 0x8400]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        for k in 0..8 {
            t.write(data + 12 * k, &halves);
        }
    }
    let size = pick(rng, &[0, 16, 40, 64, 100, 0xffff_ffd0]);
    let maps: Vec<u32> = (0..24).map(|_| pick(rng, &[0, 0, 1, 9])).collect();
    t.stub_map69(h67::VB_MAP_FOR_READ, data, size, &maps);
    let unmaps: Vec<u32> = (0..24).map(|_| pick(rng, &[0, 1])).collect();
    t.stub_seq(h67::VB_UNMAP, true, 2, 2, &unmaps);
    let count = rng.below(4);
    t.global(h67::CLOTH_COUNT, count);
    let slots = t.rebase(h67::CLOTH_SLOTS);
    for k in 0..5 {
        let v = if rng.chance(3) { 0 } else { rng.word() | 1 };
        t.w32(slots + 4 * k, v);
    }
    // junk reads: the map outputs, never written when no map ran
    FRAME_WRITES.with(|z| z.replace(vec![(0xa8, vec![0; 0x10])]));
}

const CLOTH69_STUBS: &[(u32, K)] = &[];

#[rustfmt::skip]
const CASES69: &[Case] = &[
    with_cookie(framed(Case { prep: Some(prep_cloth69), ..cdecl("get_cloth_vertex_buffers", 0x0063_a810, 2, CLOTH69_STUBS, |m, _, a| u(gh67::get_cloth_vertex_buffers(m, frame_ebp(), a[0], a[1]))) }, 0xb8, 0)),
];

#[test]
fn batch69_glue_matches_the_original() {
    Twins::each(|t| {
        let n = if t.snapshot { 60 } else { 300 };
        run_cases(t, CASES69, n, &mut Rng(0x6969_b069));
    });
}
