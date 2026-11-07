//! `PhyreEngine`'s `PAnimation` code, list `phyre-anim-left`
//! (`ffx_panim::m5p_anim_left*`, M5 stage 1), against the original. Almost none
//! of this code ran in a coverage run, so every case is synthetic memory
//! built for its branch, on the image alone and over a snapshot
//! (`FFX_SNAPSHOT`, run once with each of field.snap and battle.snap).
//! Results and every byte written must agree; template copies each run
//! against their own original. Needs `FFX_GAME_DIR`.
//!
//! Callees outside the list run as they are where they are plain code,
//! or are replaced in both oracles alike by recording stubs: each call
//! appends its tag, `ecx` and its stack words to a log in memory, so the
//! diff checks that both made the same calls in the same order.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_panim::m5p_anim_left_a::{self as la, bridge as lab};
use ffx_panim::m5p_anim_left_b::{self as lb, bridge as lbb};
use ffx_panim::m5p_anim_left_c::{self as lc, bridge as lcb};
use ffx_panim::m5p_anim_left_d::{self as ld, bridge as ldb};
use ffx_panim::m5p_anim_left_e::{self as le, bridge as leb};
use ffx_panim::m5p_anim_left_f::{self as lf, bridge as lfb};
use ffx_panim::m5p_anim_left_g::{self as lg, bridge as lgb};
use ffx_panim::m5p_anim_left_h::{self as lh, bridge as lhb};
use ffx_panim::m5p_anim_left_i::{self as li, bridge as lib};
use ffx_panim::m5p_anim_left_j::{self as lj, bridge as ljb};
use ffx_panim::m5p_anim_left_k::{self as lk, bridge as lkb};
use ffx_panim::m5p_anim_left_l::{self as ll, bridge as llb};
use ffx_panim::m5p_anim_left_m::{self as lm, bridge as lmb};
use ffx_panim::m5p_anim_small as sm;

/// `ret` or `ret 4 * pops`.
fn ret_bytes(pops: u32) -> Vec<u8> {
    if pops == 0 {
        vec![0xc3]
    } else {
        let mut v = vec![0xc2];
        v.extend(((pops * 4) as u16).to_le_bytes());
        v
    }
}

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
struct G<'a>(&'a mut Oracle, u32, u32);

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
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0_f64()
            .unwrap_or_else(|| panic!("callee {addr:#x} left no ST0"))
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let b = x.to_bits();
        let r = self
            .0
            .call(self.2, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let Some(v) = r.st0_f32() else {
            panic!("intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(self.2 + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        let (a, b) = (x.to_bits(), y.to_bits());
        let words = [a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32, addr];
        let r = self
            .0
            .call(self.2 + 32, Conv::Cdecl, &words)
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let Some(v) = r.st0_f32() else {
            panic!("intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(self.2 + 16, Conv::Cdecl, &[]).unwrap();
        v
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
    /// `fld qword [esp+4]; call [esp+0xc]; ret`, and at `+16` `fstp st0;
    /// ret`: [`Host::call_intrinsic`]'s way into a CRT intrinsic.
    tramp: u32,
}

/// At `+32`: `fld qword [esp+4]; fld qword [esp+0xc]; call [esp+0x14];
/// ret` for [`Host::call_intrinsic2`].
const TRAMPOLINE_CODE: [u8; 45] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3,
];

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
            tramp: 0,
        };
        t.log = t.zeros(0x10000);
        t.tramp = t.a.alloc_code(&TRAMPOLINE_CODE).unwrap();
        assert_eq!(t.b.alloc_code(&TRAMPOLINE_CODE).unwrap(), t.tramp);
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
        let mut tail = vec![0xb8];
        tail.extend(ret.to_le_bytes()); // mov eax, ret
        tail.extend(ret_bytes(pops));
        self.stub_code_tail(tag, this, n, &tail)
    }

    /// The logging part of [`Twins::stub_code`], then `tail` (which sets
    /// `eax` and returns).
    fn stub_code_tail(&mut self, tag: u32, this: bool, n: u32, tail: &[u8]) -> u32 {
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
        c.extend(tail);
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// Puts a `jmp` to `code` at the original `sym`.
    fn hook(&mut self, sym: u32, code: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// A recording `PhyreMalloc` stub (cdecl `(size, align)`) handing out
    /// 16-byte aligned blocks from a fresh `bytes`-byte area of junk.
    fn stub_bump(&mut self, sym: u32, bytes: usize) {
        let area = self.junk(bytes + 16, 0x6b);
        let cur = self.alloc(&((area + 15) & !15).to_le_bytes());
        let mut tail = vec![0xa1];
        tail.extend(cur.to_le_bytes()); // mov eax, [cur]
        tail.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        tail.extend([0x83, 0xc2, 0x0f, 0x83, 0xe2, 0xf0]); // add edx, 15; and edx, -16
        tail.extend([0x01, 0xc2]); // add edx, eax
        tail.extend([0x89, 0x15]);
        tail.extend(cur.to_le_bytes()); // mov [cur], edx
        tail.push(0xc3);
        let code = self.stub_code_tail(sym, false, 2, &tail);
        self.hook(sym, code);
    }

    /// A recording stub at `sym` that returns `hit` when its first stack
    /// word is `key`, else 0.
    fn stub_match(&mut self, sym: u32, n: u32, pops: u32, key: u32, hit: u32) {
        let mut tail = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        tail.push(0x3d);
        tail.extend(key.to_le_bytes()); // cmp eax, key
        tail.extend([0xb8, 0, 0, 0, 0]); // mov eax, 0
        tail.extend([0x75, 0x05]); // jne +5
        tail.push(0xb8);
        tail.extend(hit.to_le_bytes()); // mov eax, hit
        tail.extend(ret_bytes(pops));
        let code = self.stub_code_tail(sym, false, n, &tail);
        self.hook(sym, code);
    }

    fn cstr(&mut self, s: &str) -> u32 {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        self.alloc(&v)
    }

    fn wf(&mut self, at: u32, v: f32) {
        self.w32(at, v.to_bits());
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
        let code = self.stub_code(sym, this, n, pops, ret);
        self.hook(sym, code);
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
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, top - below, tramp));
        self.b.set_call_top(None);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::Al(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::None => {}
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
        let tramp = self.tramp;
        let out = port(&mut G(&mut self.b, scratch, tramp));
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

// ---- batch 1: utilities ----

/// A `PArray` count with the "not owned" bit as asked.
fn count(n: u32, owned: bool) -> u32 {
    if owned { n } else { n | sm::NOT_OWNED }
}

/// A name list object (`+0x20` count, `+0x24` the names) of `names`
/// (`None` for a null entry; odd indices get the "not owned" bit 0).
fn name_list(t: &mut Twins, names: &[Option<&str>], raw_count: u32) -> u32 {
    let mut words = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let w = match n {
            // odd entries sit at an address with bit 1 set
            Some(s) if i & 1 == 1 => (t.cstr(&format!("..{s}")) + 2) | 1,
            Some(s) => t.cstr(s),
            None => 0,
        };
        words.extend(w.to_le_bytes());
    }
    words.extend([0; 4]);
    let arr = t.alloc(&words);
    let obj = t.junk(0x30, 0x21);
    t.w32(obj + 0x20, raw_count);
    t.w32(obj + 0x24, arr);
    obj
}

#[test]
fn name_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let names = [Some("alpha"), None, Some("beta"), Some("gamma"), Some("")];
        for raw in [5, 5 | sm::NOT_OWNED, 0, 3] {
            let obj = name_list(&mut t, &names, raw);
            for q in [
                "alpha", "beta", "gamma", "", "delta", "alph", "alphabet", "gamm",
            ] {
                let name = t.cstr(q);
                let what = format!("{label}: find {q:?} in {raw:#x}");
                t.check(&what, 0x0049_3f50, Conv::Thiscall(obj), &[name], |g| {
                    Out::U32(lab::find_name(g, obj, name))
                });
            }
            let what = format!("{label}: find null in {raw:#x}");
            t.check(&what, 0x0049_3f50, Conv::Thiscall(obj), &[0], |g| {
                Out::U32(lab::find_name(g, obj, 0))
            });
        }
    }
}

/// The stubs the target set calls: the member check answering `has`
/// (in `al` only), `PhyrePrintf`, `memcpy`, `PhyreFree`, and
/// `PhyreMalloc` handing out blocks or failing.
fn target_stubs(t: &mut Twins, has: bool, fail: bool) {
    t.stub(
        la::HAS_MEMBER,
        false,
        2,
        0,
        if has { 0x7701 } else { 0x7700 },
    );
    t.stub(la::PRINTF, false, 4, 0, 0x99);
    t.stub(la::MEMCPY, false, 3, 0, 0xc0);
    t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
    if fail {
        t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
    } else {
        t.stub_bump(la::PHYRE_MALLOC, 0x400);
    }
}

#[test]
fn target_sets_match_the_original() {
    for (label, mut t) in Twins::all() {
        for has in [true, false] {
            for fail in [false, true] {
                for owned in [true, false] {
                    target_stubs(&mut t, has, fail);
                    let this = t.junk(0x20, 0x44);
                    let old = t.cstr("old");
                    t.w32(this + 0x14, old | u32::from(!owned));
                    let desc = t.junk(0x20, 0x45);
                    let name = t.cstr("m_member");
                    for nm in [name, 0] {
                        let what = format!(
                            "{label}: set has {has} fail {fail} owned {owned} name {nm:#x}"
                        );
                        t.check(
                            &what,
                            0x0050_c850,
                            Conv::Thiscall(this),
                            &[desc, 0x0b1e, nm, 7],
                            |g| Out::U32(lab::set_target(g, this, desc, 0x0b1e, nm, 7)),
                        );
                    }
                    t.reset();
                }
            }
        }
    }
}

#[test]
fn current_pose_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let pose = t.rebase(la::POSE_DESCRIPTOR);
        for has in [true, false] {
            target_stubs(&mut t, has, false);
            let list = name_list(
                &mut t,
                &[
                    Some("root"),
                    Some("m_currentPose"),
                    Some("spine"),
                    Some("arm"),
                ],
                4,
            );
            let owner = t.junk(0x10, 0x46);
            t.w32(owner, list);
            let names = name_list(&mut t, &[Some("x"), Some("spine"), Some("nope")], 0);
            let names: u32 = t.r32(names + 0x24);
            // the class's names sit at +0x24 with the count at +0x18
            let class = t.junk(0x30, 0x47);
            t.w32(class + 0x18, 3 | sm::NOT_OWNED);
            t.w32(class + 0x24, names);
            let obj = t.alloc(&class.to_le_bytes());
            let cases: [(&str, u32, u32, u32); 9] = [
                ("spine", 0x1234, 0, 0),
                ("arm", pose, obj, 1),
                ("leg", 0x1234, 0, 0),
                ("m_currentPose", pose, obj, 1),
                ("m_currentPose", pose, obj, 2),
                ("m_currentPose", pose, obj, 3),
                ("m_currentPose", pose, 0, 1),
                ("m_currentPose", 0x1234, obj, 1),
                ("", pose, obj, 1),
            ];
            for (name, desc, o, idx) in cases {
                for null in [false, true] {
                    let bind = t.junk(0x20, 0x48);
                    let s = if null { 0 } else { t.cstr(name) | 1 };
                    t.w32(bind + 0x14, s);
                    t.w32(bind, desc);
                    t.w32(bind + 4, o);
                    t.w32(bind + 0x18, idx);
                    let what = format!(
                        "{label}: has {has} {name:?} desc {desc:#x} obj {o:#x} idx {idx} null {null}"
                    );
                    t.check(&what, 0x0043_2e90, Conv::Cdecl, &[bind, owner], |g| {
                        Out::U32(lab::bind_current_pose(g, bind, owner))
                    });
                }
            }
            t.reset();
        }
    }
}

#[test]
fn static_resets_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        let begin = t.rebase(la::STATIC_BEGIN);
        let cnt = t.rebase(la::STATIC_COUNT);
        for same in [true, false] {
            for n in [0, 3, 3 | sm::NOT_OWNED] {
                for data in [0, 0x0bad_da7a] {
                    for k in 0..4 {
                        t.w32(begin + 4 * k, 0x1111 * (k + 1));
                    }
                    t.w32(cnt, n);
                    t.w32(cnt + 4, data);
                    t.w32(begin, if same { data } else { data ^ 0x40 });
                    let what = format!("{label}: reset same {same} n {n:#x} data {data:#x}");
                    t.check(&what, 0x0053_a070, Conv::Cdecl, &[], |g| {
                        Out::U32(lab::reset_static(g))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn shutdowns_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(la::DELETE_FREE_LIST, false, 0, 0, 0xdf);
        let begin = t.rebase(la::STATIC_BEGIN);
        for stop in [None, Some(0), Some(7), Some(18)] {
            match stop {
                None => t.stub(la::UNLINK, false, 1, 0, 0),
                Some(k) => {
                    let rec = t.rebase(la::TARGET_TYPES + 0x10 * k);
                    t.stub_match(la::UNLINK, 1, 0, rec, 0x5a + k);
                }
            }
            t.w32(begin, 0x77);
            let what = format!("{label}: shutdown stop {stop:?}");
            t.check(&what, 0x0043_3250, Conv::Cdecl, &[], |g| {
                Out::U32(lab::shutdown(g))
            });
        }
        t.reset();
    }
}

#[test]
fn small_array_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for n in [0, 1, 3, 3 | sm::NOT_OWNED] {
            for fail in [false, true] {
                if fail {
                    t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
                } else {
                    t.stub_bump(la::PHYRE_MALLOC, 0x200);
                }
                t.stub(la::SEGMENT_COPY, false, 3, 0, 0x5c);
                let els = t.junk(0x3c, 0x51);
                let src = t.junk(0x10, 0x52);
                t.w32(src, n);
                if n & !sm::NOT_OWNED > 1 {
                    t.w32(src + 4, els);
                }
                let this = t.junk(0x40, 0x53);
                let what = format!("{label}: small copy n {n:#x} fail {fail}");
                t.check(&what, 0x0048_6ff0, Conv::Thiscall(this), &[src], |g| {
                    Out::U32(lab::small_array_copy(g, this, src))
                });
                t.reset();
                t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            }
        }
        // the copy overwrites the count (the storage handed out overlaps
        // `this`): the old storage is looked at and freed when owned
        for (word, data) in [
            (5, 0x0bad_0000),
            (5 | sm::NOT_OWNED, 0x0bad_0000),
            (1, 0),
            (0, 0),
            (5, 0),
        ] {
            let this = t.junk(0x60, 0x54) + 0x20;
            t.stub(la::PHYRE_MALLOC, false, 2, 0, this - 0x14);
            let els = t.junk(0x3c, 0x55);
            t.w32(els + 0x14, word);
            t.w32(els + 0x18, data);
            let src = t.alloc(&[2, 0, 0, 0, 0, 0, 0, 0]);
            t.w32(src + 4, els);
            let what = format!("{label}: small copy over itself {word:#x} {data:#x}");
            t.check(&what, 0x0048_6ff0, Conv::Thiscall(this), &[src], |g| {
                Out::U32(lab::small_array_copy(g, this, src))
            });
            t.reset();
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        }
    }
}

#[test]
fn batch_compares_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut tys = Vec::new();
        for v in [1u32, 2, 0x1ff] {
            let ty = t.junk(0x10, v as u8);
            t.w32(ty + 0xc, v);
            tys.push(ty);
        }
        let mut chans = Vec::new();
        for (ty, k) in [
            (0, 0),
            (0, 5),
            (tys[0], 5),
            (tys[0], 7),
            (tys[1], 0),
            (tys[2], 0x0100_0003),
            (0, 0x0200_0000),
        ] {
            let c = t.junk(0x10, 4);
            t.w32(c, ty);
            t.w32(c + 8, k);
            chans.push(c);
        }
        for &a in &chans {
            for &b in &chans {
                let what = format!("{label}: compare {a:#x} {b:#x}");
                t.check(&what, 0x0050_8ba0, Conv::Cdecl, &[a, b], |g| {
                    Out::U32(lab::compare_batch(g, a, b))
                });
            }
        }
    }
}

/// Float bits of every kind: numbers, zeros, denormals, infinities,
/// quiet NaNs (signalling ones stay out: the emulator keeps them).
const FLOATS: [u32; 12] = [
    0x3f80_0000,
    0xbf00_0000,
    0x4049_0fdb,
    0x0000_0000,
    0x8000_0000,
    0x0000_0003,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_1234,
    0xffc0_0001,
    0x3e4c_cccd,
    0xc2f6_e979,
];

#[test]
fn float_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        for n in [0u32, 1, 3, 4, 5, 8, 9] {
            let mut v = Vec::new();
            for f in FLOATS {
                v.extend(f.to_le_bytes());
            }
            let src = t.alloc(&v);
            let dst = t.junk(0x40, 0x61);
            let what = format!("{label}: copy {n} floats");
            t.check(&what, 0x0050_8f00, Conv::Cdecl, &[dst, src, n], |g| {
                Out::U32(lab::copy_floats(g, dst, src, n))
            });
        }
    }
}

#[test]
fn range_assigns_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            let buf = t.junk(0x40, 0x62);
            t.stub(la::MEM_DUP, false, 2, 0, if fail { 0 } else { buf });
            let text = t.cstr("abcdefghij");
            for owned in [true, false] {
                for (b, e) in [
                    (0, text + 3),
                    (text, 0),
                    (text + 4, text + 2),
                    (text, text),
                    (text + 2, text + 7),
                ] {
                    let this = t.alloc(&[0; 4]);
                    t.w32(this, 0x0bad_0010 | u32::from(!owned));
                    let what = format!("{label}: assign {b:#x}..{e:#x} fail {fail} owned {owned}");
                    t.check(&what, 0x0050_c7e0, Conv::Thiscall(this), &[b, e], |g| {
                        Out::U32(lab::assign_range(g, this, b, e))
                    });
                }
            }
            t.reset();
        }
    }
}

#[test]
fn lerps_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut r = Rng(0x1357_9bdf);
        for f in [0x0050_eb70, 0x0050_ebf0] {
            for case in 0..40 {
                let pick = |r: &mut Rng| {
                    if case < 30 {
                        (((r.next() % 2000) as f32 - 1000.0) / 7.0).to_bits()
                    } else {
                        FLOATS[(r.next() % 12) as usize]
                    }
                };
                let a = t.zeros(16);
                let b = t.zeros(16);
                for i in 0..4 {
                    let x = pick(&mut r);
                    t.w32(a + 4 * i, x);
                    let y = pick(&mut r);
                    t.w32(b + 4 * i, y);
                }
                let tt = pick(&mut r);
                let out = t.junk(16, 0x63);
                let what = format!("{label}: lerp {f:#x} case {case}");
                t.check(&what, f, Conv::Cdecl, &[out, tt, a, b], |g| {
                    Out::U32(lab::lerp4(g, out, tt, a, b))
                });
            }
        }
    }
}

#[test]
fn slerps_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut r = Rng(0x2468_ace1);
        for case in 0..80u32 {
            let q = |r: &mut Rng| ((r.next() % 2001) as f32 - 1000.0) / 1000.0;
            let a = t.zeros(16);
            let b = t.zeros(16);
            for i in 0..4 {
                let x = q(&mut r);
                t.wf(a + 4 * i, x);
                let y = if case % 5 == 0 { x } else { q(&mut r) };
                t.wf(b + 4 * i, y);
            }
            if case == 66 || case == 67 {
                // a cosine of exactly 0 and a threshold equal to the cosine
                for i in 0..4 {
                    t.wf(a + 4 * i, if i == 0 { 1.0 } else { 0.0 });
                    t.wf(b + 4 * i, if i == 67 - case { 1.0 } else { 0.0 });
                }
            }
            if case >= 70 {
                // NaNs and infinities through both paths
                let k = (r.next() % 4) * 4;
                let v = FLOATS[6 + (case as usize % 4)];
                t.w32(if case % 2 == 0 { a + k } else { b + k }, v);
            }
            let tt = ((r.next() % 1001) as f32 / 1000.0).to_bits();
            let thr = [0.0f32, 0.5, 0.95, -2.0, 2.0][(case % 5) as usize].to_bits();
            let thr = match case {
                69 => 0x7fc0_0000,
                67 => 1.0f32.to_bits(),
                _ => thr,
            };
            let out = t.junk(16, 0x64);
            let what = format!("{label}: slerp case {case}");
            t.check(
                &what,
                0x0050_ed70,
                Conv::Cdecl,
                &[out, tt, a, b, thr],
                |g| Out::U32(lab::slerp(g, out, tt, a, b, thr)),
            );
        }
    }
}

/// `n` channels (as `0x0050e730` reads them: `+0x1c` a type with `+0xc`,
/// `+0x20` the key) in a `PArray` at `this + 4`.
fn keyed_channels(t: &mut Twins, this: u32, keys: &[(u32, u32)], raw: u32) {
    let mut ptrs = Vec::new();
    for &(ty, k) in keys {
        let c = t.junk(0x40, 0x71);
        let tp = if ty == 0 {
            0
        } else {
            let x = t.junk(0x10, 0x72);
            t.w32(x + 0xc, ty);
            x
        };
        t.w32(c + 0x1c, tp);
        t.w32(c + 0x20, k);
        ptrs.extend(c.to_le_bytes());
    }
    ptrs.extend([0; 4]);
    let arr = t.alloc(&ptrs);
    t.w32(this + 4, raw);
    t.w32(this + 8, arr);
}

#[test]
fn distinct_keys_match_the_original() {
    for (label, mut t) in Twins::all() {
        let slots = t.rebase(la::KEY_SLOTS);
        t.w32(slots, 64);
        let sets: [&[(u32, u32)]; 5] = [
            &[],
            &[(0, 3)],
            &[
                (0, 3),
                (0, 3),
                (1, 0),
                (0, 4),
                (0, 0x45),
                (2, 0x40),
                (0, 0x47),
            ],
            &[(0, 31), (0, 32), (0, 33), (0, 31), (0, 0xff)],
            &[(3, 1), (0, 0xd), (0, 0xc), (0, 0xd), (0, 3), (0, 19)],
        ];
        for keys in sets {
            for owned in [true, false] {
                let this = t.junk(0x10, 0x73);
                keyed_channels(&mut t, this, keys, count(keys.len() as u32, owned));
                let what = format!("{label}: keys {keys:?} owned {owned}");
                t.check(&what, 0x0051_6190, Conv::Thiscall(this), &[], |g| {
                    Out::U32(lab::distinct_keys(g, this))
                });
            }
        }
    }
}

/// A target key (8 words: hash, name, six words).
fn key(t: &mut Twins, words: [u32; 8], name: &str) -> u32 {
    let s = t.cstr(name);
    let mut v = Vec::new();
    for (i, x) in words.into_iter().enumerate() {
        v.extend(if i == 1 { s } else { x }.to_le_bytes());
    }
    t.alloc(&v)
}

#[test]
fn key_compares_match_the_original() {
    for (label, mut t) in Twins::all() {
        let base = [5, 0, 1, 2, 3, 4, 5, 6];
        let mut keys = vec![key(&mut t, base, "abc")];
        for i in [0, 2, 3, 4, 5, 6, 7] {
            for d in [1u32, u32::MAX] {
                let mut w = base;
                w[i] = w[i].wrapping_add(d);
                keys.push(key(&mut t, w, "abc"));
            }
        }
        for n in ["abd", "ab", "abcd", "", "abc"] {
            keys.push(key(&mut t, base, n));
        }
        for &a in &keys {
            for &b in &keys {
                let what = format!("{label}: keys {a:#x} {b:#x}");
                t.check(&what, 0x005c_ccd0, Conv::Cdecl, &[a, b], |g| {
                    Out::U32(lab::compare_keys(g, a, b))
                });
            }
        }
    }
}

/// A `0x2c`-byte tree node with a key from `words` and `name`, its
/// links junk.
fn node(t: &mut Twins, words: [u32; 8], name: &str) -> u32 {
    let n = t.junk(0x2c, 0x74);
    let s = t.cstr(name);
    for (i, w) in (0u32..).zip(words) {
        t.w32(n + 0xc + 4 * i, if i == 1 { s } else { w });
    }
    n
}

#[test]
fn tree_inserts_and_finds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let hdr = t.junk(0x18, 0x75);
        t.w32(hdr + 0xc, hdr);
        let mut r = Rng(0x0f0f_1234);
        let mut keys: Vec<([u32; 8], &str)> = Vec::new();
        for i in 0..24u32 {
            let mut w = [r.next() % 5, 0, r.next() % 3, 1, 2, 3, 4, r.next() % 2];
            let mut name = ["a", "b", "c"][(r.next() % 3) as usize];
            if i % 7 == 6 {
                // a duplicate of an earlier key
                let (ow, on) = keys[(r.next() as usize) % keys.len()];
                w = ow;
                name = on;
            }
            keys.push((w, name));
            let n = node(&mut t, w, name);
            // odd ones start with their colour bit set
            let p: u32 = t.r32(n + 8);
            t.w32(n + 8, p & !1 | (i & 1));
            let what = format!("{label}: insert {i}");
            t.check(&what, 0x005c_c9f0, Conv::Thiscall(hdr), &[n], |g| {
                Out::U32(lab::tree_insert(g, hdr, n))
            });
        }
        let mut probes = keys.clone();
        probes.push(([9, 0, 0, 0, 0, 0, 0, 0], "a"));
        probes.push(([0, 0, 0, 1, 2, 3, 4, 0], "zz"));
        for (i, (w, name)) in probes.into_iter().enumerate() {
            let k = key(&mut t, w, name);
            let what = format!("{label}: find {i}");
            t.check(&what, 0x005c_c870, Conv::Thiscall(hdr), &[k], |g| {
                Out::U32(lab::tree_find(g, hdr, k))
            });
        }
        // the empty tree
        let empty = t.junk(0x18, 0x76);
        t.w32(empty + 0xc, empty);
        let k = key(&mut t, [0; 8], "a");
        let what = format!("{label}: find in empty");
        t.check(&what, 0x005c_c870, Conv::Thiscall(empty), &[k], |g| {
            Out::U32(lab::tree_find(g, empty, k))
        });
    }
}

/// A target: five words, the name, the word at `+0x18`.
type Target<'a> = ([u32; 5], Option<&'a str>, u32);

/// A `PAnimationSet`-like object for the lookup: `+8` count, `+0xc` the
/// `0x1c`-byte targets (words, name at `+0x14`, `+0x18`).
fn target_set(t: &mut Twins, targets: &[Target]) -> u32 {
    let mut v = Vec::new();
    for (i, (w, name, last)) in targets.iter().enumerate() {
        for x in w {
            v.extend(x.to_le_bytes());
        }
        let s = match name {
            Some(n) => t.cstr(n) | (i as u32 & 1),
            None => 0,
        };
        v.extend(s.to_le_bytes());
        v.extend(last.to_le_bytes());
    }
    v.extend([0; 4]);
    let arr = t.alloc(&v);
    let set = t.junk(0x20, 0x77);
    t.w32(set + 8, targets.len() as u32);
    t.w32(set + 0xc, arr);
    set
}

#[test]
fn lookup_ctors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let sets: [&[Target]; 4] = [
            &[],
            &[([1, 2, 3, 4, 5], Some("a"), 6)],
            &[
                ([1, 2, 3, 4, 5], Some("a"), 6),
                ([1, 2, 3, 4, 5], Some("b"), 6),
                ([0, 2, 3, 4, 5], None, 6),
                ([1, 2, 3, 4, 5], Some("a"), 6),
                ([1, 2, 3, 4, 9], Some("a"), 6),
                ([1, 2, 3, 4, 5], Some("a"), 7),
                ([1, 2, 3, 4, 5], None, 6),
            ],
            &[
                ([7, 0, 0, 0, 0], Some("x"), 0),
                ([7, 0, 0, 0, 1], Some("x"), 0),
                ([6, 0, 0, 0, 1], Some("x"), 0),
            ],
        ];
        for (k, targets) in sets.iter().enumerate() {
            for hash in [None, Some(0x77)] {
                for fail in [false, true] {
                    t.stub(la::PRINTF, false, 3, 0, 0x99);
                    let set = target_set(&mut t, targets);
                    match hash {
                        // the first target's own hash differs from the rest
                        None => {
                            let first: u32 = t.r32(set + 0xc);
                            t.stub_match(la::TARGET_HASH, 1, 0, first, 0x55);
                        }
                        Some(h) => t.stub(la::TARGET_HASH, false, 1, 0, h),
                    }
                    if fail {
                        t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
                    } else {
                        t.stub_bump(la::PHYRE_MALLOC, 0x200);
                    }
                    let this = t.junk(0x20, 0x78);
                    let what = format!("{label}: lookup set {k} hash {hash:?} fail {fail}");
                    t.check(&what, 0x005c_ca80, Conv::Thiscall(this), &[set], |g| {
                        Out::U32(lab::lookup_ctor(g, this, set))
                    });
                    t.reset();
                }
            }
        }
    }
}

#[test]
fn channel_constructs_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(la::CHANNEL_BASE_COPY, true, 1, 1, 0xc0b);
        t.stub(la::CHANNEL_BASE_CTOR, true, 0, 0, 0xc7);
        for n in [0u32, 1, 3] {
            let src = t.junk(0xa0, 0x81);
            let dst = t.junk(0xa0, 0x82);
            let what = format!("{label}: copy {n} channels");
            t.check(&what, 0x0051_1b40, Conv::Cdecl, &[dst, src, n], |g| {
                lab::copy_channels(g, dst, src, n);
                Out::None
            });
            let what = format!("{label}: construct {n} channels");
            t.check(&what, 0x0051_5880, Conv::Cdecl, &[dst, n], |g| {
                lab::construct_channels(g, dst, n);
                Out::None
            });
        }
        assert!(t.log_entries() > 0);
        t.reset();
    }
}

#[test]
fn set_rebuilds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let slots = t.rebase(la::KEY_SLOTS);
        t.w32(slots, 64);
        t.stub(la::CLUSTER_DELETE, true, 1, 1, 0xde1);
        t.stub(la::SET_DATA_CTOR, true, 2, 2, 0xc7);
        for fail in [false, true] {
            t.stub(
                la::CLUSTER_CREATE,
                true,
                3,
                3,
                if fail { 0 } else { 0x0c1c_0000 },
            );
            for old in [0, 0x0b1d] {
                for cluster in [0, 0x0c1a] {
                    for (chans, slots_n) in [(0u32, 0u32), (3, 2), (5, 9 | sm::NOT_OWNED)] {
                        let this = t.junk(0x20, 0x83);
                        let keys: Vec<(u32, u32)> = (0..chans).map(|i| (i & 1, i * 3)).collect();
                        keyed_channels(&mut t, this, &keys, chans);
                        t.w32(this, old);
                        t.w32(this + 0xc, slots_n);
                        let what = format!(
                            "{label}: rebuild fail {fail} old {old:#x} cluster {cluster:#x} {chans} {slots_n:#x}"
                        );
                        t.check(
                            &what,
                            0x0051_6240,
                            Conv::Thiscall(this),
                            &[cluster, 0xa59],
                            |g| Out::U32(lab::rebuild(g, this, cluster, 0xa59)),
                        );
                    }
                }
            }
        }
        t.reset();
    }
}

#[test]
fn word_resizes_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            for raw in [0u32, 3, 3 | sm::NOT_OWNED] {
                for n in [0u32, 1, 3, 5, 3 | sm::NOT_OWNED] {
                    // a count with the flag only as the raw word's match
                    if n & sm::NOT_OWNED != 0 && n != raw {
                        continue;
                    }
                    let els = t.junk(0x20, 0x84);
                    let this = t.junk(0x10, 0x85);
                    t.w32(this + 4, raw);
                    t.w32(this + 8, if raw == 0 || n == 0 { 0 } else { els });
                    let what = format!("{label}: resize words fail {fail} {raw:#x} -> {n:#x}");
                    t.check(&what, 0x0051_6fe0, Conv::Thiscall(this), &[n], |g| {
                        Out::U32(lab::resize_words(g, this, n))
                    });
                }
            }
            t.reset();
        }
    }
}

#[test]
fn channel_resizes_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(la::CHANNEL_BASE_COPY, true, 1, 1, 0xc0b);
            t.stub(la::CHANNEL_BASE_CTOR, true, 0, 0, 0xc7);
            t.stub(la::ARRAY_ASSIGN, true, 2, 2, 0xa55);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x1000);
            }
            for raw in [0u32, 2, 2 | sm::NOT_OWNED] {
                for n in [0u32, 1, 2, 4, 2 | sm::NOT_OWNED] {
                    // a count with the flag only as the raw word's match
                    if n & sm::NOT_OWNED != 0 && n != raw {
                        continue;
                    }
                    let els = t.junk(0x80, 0x86);
                    let this = t.junk(0x20, 0x87);
                    t.w32(this, raw);
                    t.w32(this + 4, els);
                    let what = format!("{label}: resize channels fail {fail} {raw:#x} -> {n:#x}");
                    t.check(&what, 0x0051_6f20, Conv::Thiscall(this), &[n], |g| {
                        Out::U32(lab::resize_channels(g, this, n))
                    });
                    let this = t.junk(0x20, 0x88);
                    t.w32(this + 0xc, raw);
                    t.w32(this + 0x10, els);
                    let what =
                        format!("{label}: resize set channels fail {fail} {raw:#x} -> {n:#x}");
                    t.check(&what, 0x0051_70c0, Conv::Thiscall(this), &[n], |g| {
                        Out::U32(lab::resize_set_channels(g, this, n))
                    });
                }
            }
            t.reset();
        }
    }
}

#[test]
fn target_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(la::MEMCPY, false, 3, 0, 0xc0);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x200);
            }
            for n in [0u32, 1, 4] {
                let set = target_set(
                    &mut t,
                    &[
                        ([1, 2, 3, 4, 5], Some("first"), 6),
                        ([7, 8, 9, 10, 11], None, 12),
                        ([13, 14, 15, 16, 17], Some(""), 18),
                        ([19, 20, 21, 22, 23], Some("last"), 24),
                    ],
                );
                let src: u32 = t.r32(set + 0xc);
                let dst = t.junk(0x80, 0x89);
                let what = format!("{label}: copy {n} targets fail {fail}");
                t.check(&what, 0x0051_7800, Conv::Cdecl, &[dst, src, n], |g| {
                    lab::copy_targets(g, dst, src, n);
                    Out::None
                });
            }
            t.reset();
        }
    }
}

// ---- batch 2: controllers and blenders ----

impl Twins {
    /// A recording `thiscall` method without stack words that leaves the
    /// float `bits` in `ST0`.
    fn float_method(&mut self, tag: u32, bits: u32) -> u32 {
        let cell = self.alloc(&bits.to_le_bytes());
        let mut tail = vec![0xd9, 0x05];
        tail.extend(cell.to_le_bytes()); // fld dword [cell]
        tail.push(0xc3);
        self.stub_code_tail(tag, true, 0, &tail)
    }

    /// An object whose vtable slot `slot` is `code` (the others 0).
    fn object_with(&mut self, slot: u32, code: u32) -> u32 {
        let mut vt = vec![0u8; 4 * (slot as usize + 1)];
        vt[4 * slot as usize..].copy_from_slice(&code.to_le_bytes());
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }

    /// `f32` values in fresh memory.
    fn floats(&mut self, v: &[f32]) -> u32 {
        let mut b = Vec::new();
        for x in v {
            b.extend(x.to_bits().to_le_bytes());
        }
        b.extend([0; 4]);
        self.alloc(&b)
    }
}

#[test]
fn event_dispatches_match_the_original() {
    for (label, mut t) in Twins::all() {
        let cb = t.stub_code(0x7e, false, 2, 0, 0xcb);
        let times = [0.0f32, 0.5, 1.0, 1.0, 2.0, 3.0, 3.5];
        let mut ev = Vec::new();
        for (i, x) in times.iter().enumerate() {
            ev.extend(x.to_bits().to_le_bytes());
            ev.extend((0x100 + i as u32).to_le_bytes());
        }
        let evs = t.alloc(&ev);
        let points = [-1.0f32, 0.0, 0.25, 1.0, 1.5, 3.5, 9.0, f32::NAN];
        for raw in [7u32, 7 | sm::NOT_OWNED, 0, 5, 1] {
            let this = t.junk(0x10, 0x91);
            t.w32(this, raw);
            t.w32(this + 4, evs);
            for &from in &points {
                for &to in &points {
                    let (f, g) = (from.to_bits(), to.to_bits());
                    let what = format!("{label}: events {raw:#x} {from}..{to}");
                    t.check(
                        &what,
                        0x0052_07c0,
                        Conv::Thiscall(this),
                        &[f, g, cb, 0xc7],
                        |h| Out::U32(lbb::dispatch_events(h, this, f, g, cb, 0xc7)),
                    );
                }
            }
            let what = format!("{label}: events {raw:#x} no callback");
            t.check(
                &what,
                0x0052_07c0,
                Conv::Thiscall(this),
                &[0, 0x3f80_0000, 0, 1],
                |h| Out::U32(lbb::dispatch_events(h, this, 0, 0x3f80_0000, 0, 1)),
            );
        }
        assert!(t.log_entries() > 0);
    }
}

#[test]
fn interval_times_match_the_original() {
    for (label, mut t) in Twins::all() {
        let konst = t.rebase(sm::TIME_CONSTANT);
        t.wf(konst, 7.25);
        let mut r = Rng(0x7777_1234);
        for case in 0..60u32 {
            let src_time = [0.0f32, 3.7, -2.2, 100.5, 1e-3, f32::INFINITY][(case % 6) as usize];
            let src = if case % 4 == 0 {
                0
            } else {
                let code = t.float_method(0x7f, src_time.to_bits());
                t.object_with(4, code)
            };
            let len = [2.0f32, -2.0, 0.5, 0.0, -0.0, f32::NAN, 1e-6, -7.5][(case % 8) as usize];
            let start = ((r.next() % 200) as f32 - 100.0) / 9.0;
            let offset = ((r.next() % 200) as f32 - 100.0) / 7.0;
            let this = t.junk(0x20, 0x92);
            t.w32(this + 8, src);
            t.wf(this + 0xc, start);
            t.wf(this + 0x10, offset);
            t.wf(this + 0x14, len);
            let what = format!("{label}: interval case {case}");
            t.check(&what, 0x0052_2cd0, Conv::Thiscall(this), &[], |h| {
                Out::F32(lbb::interval_time(h, this))
            });
        }
        // remainders of exactly 0 and -0
        for (time, start, len) in [
            (4.0f32, 0.0f32, 2.0f32),
            (-4.0, 0.0, 2.0),
            (0.0, 4.0, -2.0),
            (1.5, -0.5, 0.5),
        ] {
            let code = t.float_method(0x7f, time.to_bits());
            let src = t.object_with(4, code);
            let this = t.junk(0x20, 0x92);
            t.w32(this + 8, src);
            t.wf(this + 0xc, start);
            t.wf(this + 0x10, 0.25);
            t.wf(this + 0x14, len);
            let what = format!("{label}: interval exact {time} {start} {len}");
            t.check(&what, 0x0052_2cd0, Conv::Thiscall(this), &[], |h| {
                Out::F32(lbb::interval_time(h, this))
            });
        }
    }
}

#[test]
fn key_searches_match_the_original() {
    for (label, mut t) in Twins::all() {
        let sets: [&[f32]; 6] = [
            &[0.0],
            &[0.0, 1.0],
            &[0.0, 1.0, 2.0, 3.0, 4.0],
            &[0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 5.0, 9.0],
            &[-3.0, -1.0, 0.5, 0.5, 8.0, 8.5, 9.0, 10.0, 11.0, 12.0, 13.0],
            &[0.0, f32::NAN, 2.0, 3.0],
        ];
        let values = [
            -5.0f32,
            -1.0,
            0.0,
            0.25,
            0.5,
            1.0,
            1.5,
            2.0,
            4.0,
            8.25,
            13.0,
            99.0,
            f32::NAN,
        ];
        for keys in sets {
            let at = t.floats(keys);
            for n in [keys.len() as u32, 0] {
                for &v in &values {
                    let b = v.to_bits();
                    let what = format!("{label}: search {keys:?}[..{n}] for {v}");
                    t.check(&what, 0x0052_53c0, Conv::Cdecl, &[at, n, b], |h| {
                        Out::U32(lbb::key_search(h, at, n, b))
                    });
                }
            }
        }
    }
}

/// A clip for the batch sizes: `+4` `k` channels, `+0xc` `c` constant
/// channels (counts with the flag on odd `k`).
fn sized_clip(t: &mut Twins, k: u32, c: u32) -> u32 {
    let clip = t.junk(0x20, 0x93);
    t.w32(clip + 4, if k & 1 == 1 { k | sm::NOT_OWNED } else { k });
    t.w32(clip + 0xc, c);
    clip
}

#[test]
fn batch_sizes_match_the_original() {
    for (label, mut t) in Twins::all() {
        let shapes: [&[(u32, u32)]; 6] = [
            &[],
            &[(3, 0)],
            &[(1, 2)],
            &[(0, 0), (5, 1), (2, 0), (9, 3), (4, 0)],
            &[(9, 3), (0, 1)],
            &[(5, 0)],
        ];
        for clips in shapes {
            let ptrs: Vec<u32> = clips
                .iter()
                .map(|&(k, c)| sized_clip(&mut t, k, c))
                .collect();
            let arr = t.zeros(0x10);
            t.w32(
                arr,
                ptrs.len() as u32 | if ptrs.len() == 2 { sm::NOT_OWNED } else { 0 },
            );
            if ptrs.len() == 1 {
                t.w32(arr + 4, ptrs[0]);
            } else {
                let mut b = Vec::new();
                for p in &ptrs {
                    b.extend(p.to_le_bytes());
                }
                b.extend([0; 4]);
                let data = t.alloc(&b);
                t.w32(arr + 4, data);
            }
            let this = t.junk(0x10, 0x94);
            t.w32(this + 8, arr);
            let what = format!("{label}: batch size {clips:?}");
            t.check(&what, 0x0052_62c0, Conv::Thiscall(this), &[], |h| {
                Out::U32(lbb::max_batch_size(h, this))
            });
        }
    }
}

#[test]
fn controller_headers_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (ctl_time, chans, consts) in [
            (None, 3u32, 2u32),
            (Some(1.5f32), 0x1_0005 | sm::NOT_OWNED, 0x2_0001),
            (Some(f32::NAN), 0, 0),
            (Some(-0.0), 7, 9),
        ] {
            let first = t.junk(0x10, 0x95);
            let clip = t.junk(0x20, 0x96);
            t.w32(clip, first);
            t.w32(clip + 4, chans);
            t.w32(clip + 0xc, consts);
            let this = t.junk(0x20, 0x97);
            t.w32(this + 0xc, clip);
            let ctl = match ctl_time {
                None => 0,
                Some(v) => {
                    let code = t.float_method(0x7d, v.to_bits());
                    t.object_with(4, code)
                }
            };
            t.w32(this + 0x10, ctl);
            let out = t.junk(0x40, 0x98);
            let what = format!("{label}: header {ctl_time:?} {chans:#x} {consts:#x}");
            t.check(
                &what,
                0x0052_6480,
                Conv::Thiscall(this),
                &[1, out, 2],
                |h| Out::U32(lbb::controller_header(h, this, out)),
            );
        }
    }
}

#[test]
fn blender_deletes_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lb::DATA_SOURCE_DTOR, true, 0, 0, 0xd5);
        for owned in [true, false] {
            for flags in [0u32, 1, 2, 3] {
                let this = t.junk(0x20, 0x99);
                t.w32(this + 0xc, count(4, owned));
                let what = format!("{label}: blender delete owned {owned} flags {flags}");
                t.check(&what, 0x0052_bb60, Conv::Thiscall(this), &[flags], |h| {
                    Out::U32(lbb::blender_delete(h, this, flags))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn blend_batches_match_the_original() {
    for (label, mut t) in Twins::all() {
        for ok in [true, false] {
            let kernel = t.stub_code(0x7c, false, 6, 0, if ok { 0x3301 } else { 0x3300 });
            for (n, nb) in [(0u32, 2u32), (1, 1), (3, 3), (5, 0), (2, 4)] {
                // each source's batch entries: `+0x10` set on some, sizes
                let mut srcs = Vec::new();
                for k in 0..n {
                    let base = t.junk(0x20 * nb as usize + 0x20, 0xa0 + k as u8);
                    for b in 0..nb {
                        let e = base + 0x20 * b;
                        t.w32(e + 0x10, u32::from((k + b) % 3 != 0) * (0x40 + k));
                        t.w32(
                            e + 0x14,
                            [5u32, 0xffff_fff0, 9, 0, 7][((k * 2 + b) % 5) as usize],
                        );
                    }
                    let holder = t.alloc(&base.to_le_bytes());
                    srcs.extend(holder.to_le_bytes());
                }
                srcs.extend([0; 4]);
                let sources = t.alloc(&srcs);
                let recs = t.junk(0x10 * nb as usize + 0x10, 0xb0);
                let batches = t.alloc(&[0; 8]);
                t.w32(batches, recs);
                t.w32(batches + 4, nb);
                let entries = t.junk(0x20 * nb as usize + 0x20, 0xb1);
                let out = t.alloc(&entries.to_le_bytes());
                // the list sits where the original's `alloca` puts it
                let below = 0x4c + 4 * n;
                let below = below + (0x10 - below % 0x10) % 0x10;
                let args = [out, sources, n, 0xa1, 0xb2, batches, kernel];
                // the list the kernel saw, after the last batch
                if nb != 0 {
                    t.frame_len = 4 * n;
                }
                let what = format!("{label}: blend batches ok {ok} n {n} batches {nb}");
                t.check_frame(&what, 0x0052_c8f0, Conv::Cdecl, &args, below, |h| {
                    let list = h.1;
                    Out::U32(lbb::blend_batches(
                        h, out, sources, n, 0xa1, 0xb2, batches, kernel, list,
                    ))
                });
            }
        }
        assert!(t.log_entries() > 0);
    }
}

#[test]
fn source_resizes_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            for raw in [0u32, 3, 3 | sm::NOT_OWNED] {
                // (a count equal to the raw word with its flag would zero
                // two billion slots in the original)
                for n in [0u32, 1, 3, 6] {
                    for f in [0x0052_cf10, 0x0052_e550, 0x0052_fbf0] {
                        let this = t.junk(0x20, 0xc1);
                        let srcs = t.junk(0x20, 0xc2);
                        let weights = t.floats(&[1.0, f32::NAN, -0.0, 4.0, 5.0, 6.0]);
                        t.w32(this + 0xc, raw);
                        // no storage under a count: the new one is null too
                        let null = raw == 0 || (n == 0 && f == 0x0052_cf10);
                        t.w32(this + 0x10, if null { 0 } else { srcs });
                        // the weights run one behind, so they resize too
                        let wraw = if raw == 0 { 2 } else { raw - 1 };
                        t.w32(this + 0x14, wraw);
                        t.w32(
                            this + 0x18,
                            if n == 0 && f == 0x0052_e550 {
                                0
                            } else {
                                weights
                            },
                        );
                        let what = format!("{label}: resize {f:#x} fail {fail} {raw:#x} -> {n:#x}");
                        t.check(&what, f, Conv::Thiscall(this), &[n], |h| {
                            Out::U32(if f == 0x0052_cf10 {
                                lbb::resize_sources(h, this, n)
                            } else {
                                lbb::resize_weights(h, this, n)
                            })
                        });
                    }
                }
            }
            t.reset();
        }
        // weights without storage: the new storage replaces no old one
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub_bump(la::PHYRE_MALLOC, 0x400);
        for &f in lb::RESIZE_WEIGHTS {
            let this = t.junk(0x20, 0xc1);
            t.w32(this + 0xc, 0);
            t.w32(this + 0x10, 0);
            t.w32(this + 0x14, 0);
            t.w32(this + 0x18, 0);
            let what = format!("{label}: resize {f:#x} from no weights");
            t.check(&what, f, Conv::Thiscall(this), &[3], |h| {
                Out::U32(lbb::resize_weights(h, this, 3))
            });
        }
        t.reset();
    }
}

#[test]
fn blender_visits_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (begin, child, end) in [
            (0x101u32, 0x201u32, 0x301u32),
            (0x100, 0x201, 0x301),
            (0x101, 0x200, 0x301),
            (0x1ff01, 0x2ff01, 0x300),
        ] {
            let visitor = t.object(&[0, 2, 0, 2], &[0, begin, 0, end]);
            let c1 = t.object(&[0, 0, 0, 2], &[0, 0, 0, child]);
            let c2 = t.object(&[0, 0, 0, 2], &[0, 0, 0, 0x401]);
            let data = t.alloc(&[0; 16]);
            t.w32(data, c1);
            t.w32(data + 8, c2);
            for raw in [0u32, 3, 3 | sm::NOT_OWNED] {
                let this = t.junk(0x20, 0xc3);
                t.w32(this + 0xc, raw);
                t.w32(this + 0x10, data);
                let what = format!("{label}: visit {begin:#x} {child:#x} {end:#x} {raw:#x}");
                t.check(
                    &what,
                    0x0052_d180,
                    Conv::Thiscall(this),
                    &[visitor, 0xc7],
                    |h| Out::U32(lbb::blender_visit(h, this, visitor, 0xc7)),
                );
            }
        }
    }
}

#[test]
fn weight_marks_match_the_original() {
    for (label, mut t) in Twins::all() {
        let w = [1.0f32, 0.0, -0.0, f32::NAN, 2.0, 0.0, 1e-30, 0.0, 5.0, 0.0];
        let weights = t.floats(&w);
        let mut b = Vec::new();
        for i in 0..10u32 {
            b.extend((i * 3 % 7).to_le_bytes());
        }
        let ids = t.alloc(&b);
        for n in [0u32, 1, 2, 4, 5, 9, 10] {
            for (f, additive) in [(0x0052_e2f0, false), (0x0052_f8a0, true)] {
                let this = t.junk(0x20, 0xc4);
                t.w32(this + 0x18, weights);
                let flags = t.zeros(0x10);
                let what = format!("{label}: marks {f:#x} n {n}");
                t.check(&what, f, Conv::Thiscall(this), &[flags, ids, n], |h| {
                    if additive {
                        lbb::mark_additive(h, this, flags, ids, n);
                    } else {
                        lbb::mark_weighted(h, this, flags, ids, n);
                    }
                    Out::None
                });
            }
        }
    }
}

#[test]
fn weight_batch_data_match_the_original() {
    for (label, mut t) in Twins::all() {
        let weights = t.floats(&[
            1.0,
            f32::NAN,
            -0.0,
            4.5,
            1e-40,
            6.0,
            7.0,
            f32::INFINITY,
            9.0,
        ]);
        for raw in [0u32, 1, 3, 4, 5, 8, 9 | sm::NOT_OWNED] {
            for &f in lb::WEIGHT_BATCH_DATA {
                let this = t.junk(0x20, 0xc5);
                t.w32(this + 0x14, raw);
                t.w32(this + 0x18, weights);
                let out = t.junk(0x60, 0xc6);
                let what = format!("{label}: batch data {f:#x} {raw:#x}");
                t.check(&what, f, Conv::Thiscall(this), &[1, out, 2], |h| {
                    Out::U32(lbb::weight_batch_data(h, this, out))
                });
            }
        }
    }
}

/// Random quaternions, then special values.
fn quats(t: &mut Twins, seed: u32, n: u32) -> Vec<u32> {
    let mut r = Rng(seed);
    let mut out = Vec::new();
    for i in 0..n {
        let q = t.zeros(16);
        for k in 0..4 {
            let v = ((r.next() % 2001) as f32 - 1000.0) / 997.0;
            t.wf(q + 4 * k, v);
        }
        if i >= n - 6 {
            let k = (r.next() % 4) * 4;
            t.w32(q + k, FLOATS[3 + (i % 7) as usize]);
        }
        out.push(q);
    }
    let z = t.zeros(16);
    out.push(z);
    // sums whose rounding order shows: products of 1 and 2^-53 pairs,
    // squares 1, 2^-26, 2^-24, a component that overflows doubled
    for v in [
        [1.0f32, 1.0, 2f32.powi(-26), 2f32.powi(-26)],
        [2f32.powi(-27), 2f32.powi(-27), 1.0, 1.0],
        [1.0, 0.0, 2f32.powi(-13), 2f32.powi(-12)],
        [3e38, 1e-10, 0.5, 0.5],
    ] {
        let q = t.zeros(16);
        for (k, x) in (0u32..).zip(v) {
            t.wf(q + 4 * k, x);
        }
        out.push(q);
    }
    out
}

#[test]
fn quaternion_products_match_the_original() {
    for (label, mut t) in Twins::all() {
        let qs = quats(&mut t, 0x5151_aaaa, 24);
        for (i, &a) in qs.iter().enumerate() {
            // the two crafted ones go together
            let b = if i == qs.len() - 4 {
                qs[i + 1]
            } else {
                qs[(i * 7 + 3) % qs.len()]
            };
            let out = t.junk(16, 0xc7);
            let what = format!("{label}: quat mul {i}");
            t.check(&what, 0x0052_ecb0, Conv::Thiscall(a), &[out, b], |h| {
                Out::U32(lbb::quat_mul(h, a, out, b))
            });
            // in place: the product written over `this`
            let c = t.zeros(16);
            for k in 0..4 {
                let v = t.r32(a + 4 * k);
                t.w32(c + 4 * k, v);
            }
            let what = format!("{label}: quat mul {i} in place");
            t.check(&what, 0x0052_ecb0, Conv::Thiscall(c), &[c, b], |h| {
                Out::U32(lbb::quat_mul(h, c, c, b))
            });
        }
    }
}

#[test]
fn quaternion_normalisations_match_the_original() {
    for (label, mut t) in Twins::all() {
        let qs = quats(&mut t, 0x1234_5151, 24);
        for (i, &q) in qs.iter().enumerate() {
            let out = t.junk(16, 0xc8);
            let what = format!("{label}: normalise {i}");
            t.check(&what, 0x0052_fa10, Conv::Cdecl, &[out, q], |h| {
                Out::U32(lbb::quat_normalize(h, out, q))
            });
        }
    }
}

#[test]
fn quaternion_matrices_match_the_original() {
    for (label, mut t) in Twins::all() {
        let qs = quats(&mut t, 0x0bad_f00d, 30);
        for (i, &q) in qs.iter().enumerate() {
            let out = t.junk(0x30, 0xc9);
            let what = format!("{label}: matrix {i}");
            t.check(&what, 0x0053_0c30, Conv::Thiscall(out), &[q], |h| {
                Out::U32(lbb::quat_matrix(h, out, q))
            });
        }
    }
}

#[test]
fn target_headers_match_the_original() {
    for (label, mut t) in Twins::all() {
        for raw in [0u32, 1, 3, 4 | sm::NOT_OWNED] {
            let owner = t.junk(0x20, 0xca);
            t.w32(owner + 0x10, raw);
            let this = t.junk(0x10, 0xcb);
            t.w32(this + 8, owner);
            let out = t.junk(0x40, 0xcc);
            let what = format!("{label}: target header {raw:#x}");
            t.check(
                &what,
                0x0053_0750,
                Conv::Thiscall(this),
                &[1, out, 2],
                |h| Out::U32(lbb::target_header(h, this, out)),
            );
        }
    }
}

#[test]
fn log_forwards_match_the_original() {
    for (label, mut t) in Twins::all() {
        let code = t.stub_code(0x7b, true, 4, 4, 0x10c);
        let logger = t.object_with(2, code);
        for with in [true, false] {
            let obj = t.junk(0x20, 0xcd);
            t.w32(obj + 0x14, if with { logger } else { 0 });
            let top = t.b.stack_range().end - 0x100;
            // four words: the cell is the original's [ebp-4], the
            // variable arguments start at the third word
            let va = top - 16 + 8;
            if with {
                t.frame_len = 4;
            }
            let what = format!("{label}: log with {with}");
            t.check_frame(
                &what,
                0x0053_0b20,
                Conv::Cdecl,
                &[obj, 0xf0f, 1, 2],
                28,
                |h| {
                    let cell = h.1;
                    Out::U32(lbb::log(h, obj, 0xf0f, va, cell))
                },
            );
        }
    }
}

// ---- batch 3: PAnimatableComponent and the network optimiser ----

impl Twins {
    /// A recording `thiscall` stub (two stack words, popped) that writes
    /// `x` and `y` through them and returns `ret`.
    fn stub_out2(&mut self, sym: u32, x: u32, y: u32, ret: u32) {
        let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov [eax], x
        tail.extend(x.to_le_bytes());
        tail.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]); // mov eax, [esp+8]; mov [eax], y
        tail.extend(y.to_le_bytes());
        tail.push(0xb8);
        tail.extend(ret.to_le_bytes());
        tail.extend(ret_bytes(2));
        let code = self.stub_code_tail(sym, true, 2, &tail);
        self.hook(sym, code);
    }
}

#[test]
fn fresh_resizes_match_the_original() {
    assert_eq!(lc::FRESH_RESIZES.len(), 2);
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            for &(_, r) in lc::FRESH_RESIZES {
                t.stub(r.init, false, 2, 0, 0x1a);
            }
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x1000);
            }
            for &(f, r) in lc::FRESH_RESIZES {
                for (raw, data) in [
                    (0u32, 0u32),
                    (3, 0x0bad_0010),
                    (3 | sm::NOT_OWNED, 0x0bad_0020),
                    (2, 0),
                ] {
                    for n in [0u32, 2, 3, 5] {
                        let this = t.junk(0x10, 0xd1);
                        t.w32(this, raw);
                        t.w32(this + 4, data);
                        let what = format!("{label}: fresh {f:#x} fail {fail} {raw:#x} -> {n}");
                        t.check(&what, f, Conv::Thiscall(this), &[n], |h| {
                            Out::U32(lcb::fresh_resize(h, r, this, n))
                        });
                    }
                }
            }
            t.reset();
        }
    }
}

#[test]
fn record_resizes_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(lc::COPY_RECORDS, false, 3, 0, 0xc3);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x2000);
            }
            for (raw, data) in [
                (0u32, 0u32),
                (2, 0x0bad_0010),
                (2 | sm::NOT_OWNED, 0x0bad_0020),
                (1, 0),
            ] {
                for n in [0u32, 1, 2, 4] {
                    let this = t.junk(0x10, 0xd2);
                    t.w32(this, raw);
                    t.w32(this + 4, data);
                    let what = format!("{label}: records fail {fail} {raw:#x} -> {n}");
                    t.check(&what, 0x0053_bfd0, Conv::Thiscall(this), &[n], |h| {
                        Out::U32(lcb::resize_records(h, this, n))
                    });
                }
            }
            t.reset();
        }
        for n in [0u32, 1, 3] {
            let dst = t.junk(0x94 * 3 + 8, 0xd3);
            let what = format!("{label}: construct {n} records");
            t.check(&what, 0x0053_9480, Conv::Cdecl, &[dst, n], |h| {
                lcb::construct_records(h, dst, n);
                Out::None
            });
        }
    }
}

#[test]
fn appends_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lc::COPY_TRIPLES, false, 3, 0, 0x3c);
        for &(_, _, rel) in lc::OBJECT_APPENDS {
            t.stub(rel, false, 2, 0, 0x7e1);
        }
        let value = t.junk(0x10, 0xd4);
        for (raw, null_old) in [
            (0u32, true),
            (0, false),
            (3, false),
            (3 | sm::NOT_OWNED, false),
            (1, false),
        ] {
            for with in [true, false] {
                let old = if null_old { 0 } else { t.junk(0x40, 0xd5) };
                let mk = |t: &mut Twins| {
                    let this = t.junk(0x10, 0xd6);
                    t.w32(this, raw);
                    t.w32(this + 4, old);
                    let new = if with { t.junk(0x40, 0xd7) } else { 0 };
                    (this, new)
                };
                for &f in lc::WORD_APPENDS {
                    let (this, new) = mk(&mut t);
                    let what = format!("{label}: append {f:#x} {raw:#x} old {old:#x} new {new:#x}");
                    t.check(&what, f, Conv::Thiscall(this), &[new, value], |h| {
                        Out::U32(lcb::append(h, 1, this, new, value))
                    });
                }
                let (this, new) = mk(&mut t);
                let what = format!("{label}: append pair {raw:#x} old {old:#x} new {new:#x}");
                t.check(
                    &what,
                    0x0061_6b90,
                    Conv::Thiscall(this),
                    &[new, value],
                    |h| Out::U32(lcb::append(h, 2, this, new, value)),
                );
                let (this, new) = mk(&mut t);
                let what = format!("{label}: append triple {raw:#x} old {old:#x} new {new:#x}");
                t.check(
                    &what,
                    0x0061_6c30,
                    Conv::Thiscall(this),
                    &[new, value],
                    |h| Out::U32(lcb::append_triple(h, this, new, value)),
                );
                for &(f, _, _) in lc::OBJECT_APPENDS {
                    let (this, new) = mk(&mut t);
                    let what =
                        format!("{label}: append object {f:#x} {raw:#x} old {old:#x} new {new:#x}");
                    t.check(&what, f, Conv::Thiscall(this), &[new, value], |h| {
                        Out::U32(lcb::append_object(
                            h,
                            lc::object_append_of(f),
                            this,
                            new,
                            value,
                        ))
                    });
                }
            }
        }
        t.reset();
    }
}

/// An optimiser node table: `n` entries (12 bytes: key, state, id) at
/// `+0x10c` (count `+0x108`), owners at `+0x114` (count `+0x110`).
fn net_node(t: &mut Twins, entries: &[(u32, u32, u32)], owners: &[u32]) -> u32 {
    let this = t.junk(0x120, 0xd8);
    let mut v = Vec::new();
    for &(k, s, id) in entries {
        for w in [k, s, id] {
            v.extend(w.to_le_bytes());
        }
    }
    v.extend([0; 4]);
    let arr = t.alloc(&v);
    t.w32(this + 0x108, entries.len() as u32);
    t.w32(this + 0x10c, arr);
    let mut o = Vec::new();
    for w in owners {
        o.extend(w.to_le_bytes());
    }
    o.extend([0; 4]);
    let list = t.alloc(&o);
    t.w32(this + 0x110, owners.len() as u32);
    t.w32(this + 0x114, list);
    this
}

/// An owner listing `links` (`(node, index)`), with `+0x118` = `state`.
fn net_owner(t: &mut Twins, links: &[(u32, u32)], state: u32) -> u32 {
    let o = t.junk(0x120, 0xd9);
    let mut v = Vec::new();
    for &(a, b) in links {
        v.extend(a.to_le_bytes());
        v.extend(b.to_le_bytes());
    }
    v.extend([0; 4]);
    let arr = t.alloc(&v);
    t.w32(o + 0x100, links.len() as u32);
    t.w32(o + 0x104, arr);
    t.w32(o + 0x118, state);
    o
}

#[test]
fn network_links_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lc::RELEASE, true, 1, 1, 0x4e1);
        // the owners point back at the node, so it is made first
        let entries = [
            (5u32, 1u32, 7u32),
            (6, 1, 0x8000_0000),
            (u32::MAX, 1, 3),
            (8, 2, 4),
            (9, 1, 5),
            (10, 1, 6),
            (11, 0, 2),
        ];
        // case 2: index 4 is listed by an unresolved owner, then a
        // resolved one
        for case in 0..4u32 {
            let node = net_node(&mut t, &entries, &[]);
            let mut owners = Vec::new();
            for (k, st) in [(0u32, 1u32), (4, u32::MAX), (5, 2), (1, 3)] {
                let links = [(0x1234, k), (node, k), (node, k ^ 1)];
                let o = net_owner(
                    &mut t,
                    &links[..=(case as usize % 3)],
                    if case == 3 { u32::MAX } else { st },
                );
                owners.push(o);
            }
            let mut v = Vec::new();
            for o in &owners {
                v.extend(o.to_le_bytes());
            }
            let list = t.alloc(&v);
            t.w32(
                node + 0x110,
                if case == 0 { 0 } else { owners.len() as u32 },
            );
            t.w32(node + 0x114, list);
            for i in 0..7 {
                let what = format!("{label}: links case {case} index {i}");
                t.check(&what, 0x0061_68a0, Conv::Thiscall(node), &[i], |h| {
                    Out::U32(lcb::links_resolved(h, node, i))
                });
            }
            let what = format!("{label}: release case {case}");
            t.check(&what, 0x0061_6930, Conv::Thiscall(node), &[0xa7], |h| {
                Out::U32(lcb::release_resolved(h, node, 0xa7))
            });
        }
        let empty = net_node(&mut t, &[], &[]);
        t.check(
            &format!("{label}: release none"),
            0x0061_6930,
            Conv::Thiscall(empty),
            &[0xa7],
            |h| Out::U32(lcb::release_resolved(h, empty, 0xa7)),
        );
        t.reset();
    }
}

#[test]
fn entry_resolves_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lc::RESOLVE, true, 2, 2, 0x5e5);
        t.stub(lc::CONTEXT_DTOR, true, 0, 0, 0xd7);
        for n in [0usize, 1, 4] {
            let entries: Vec<(u32, u32, u32)> = (0..n as u32).map(|i| (0x40 + i, 0, 9)).collect();
            let node = net_node(&mut t, &entries, &[]);
            if n != 0 {
                t.frame_len = 12;
            }
            let what = format!("{label}: resolve {n}");
            t.check_frame(
                &what,
                0x0061_67a0,
                Conv::Thiscall(node),
                &[0xa8],
                0x28,
                |h| {
                    let frame = h.1;
                    let r = lcb::resolve_entries(h, node, 0xa8, frame);
                    if n == 0 {
                        Out::Al(r as u8)
                    } else {
                        Out::U32(r)
                    }
                },
            );
        }
        t.reset();
    }
}

#[test]
fn node_links_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lc::NODE_SETUP, true, 1, 1, 0x5e7);
        t.stub(lc::NODE_LINK_IN, true, 2, 2, 0x111);
        t.stub(lc::NODE_LINK_OUT, true, 1, 1, 0x222);
        t.stub(lc::NODE_ADD_OUTPUT, true, 1, 1, 0x3300);
        let nodes = t.junk(0x11c * 4, 0xda);
        let this = t.alloc(&[0; 8]);
        t.w32(this, nodes);
        t.w32(this + 4, 3);
        let pairs = t.alloc(&[1, 0, 0, 0, 7, 0, 0, 0, 2, 0, 0, 0, 9, 0, 0, 0]);
        let words = t.alloc(&[5, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0]);
        for i in [0u32, 2, 3, 9] {
            for (n, m, w) in [
                (0u32, 0u32, 0u32),
                (2, 0, words),
                (0, 3, words),
                (2, 2, 0),
                (1, 0, 0),
            ] {
                let what = format!("{label}: link node {i} pairs {n} outputs {m} words {w:#x}");
                t.check(
                    &what,
                    0x005c_d920,
                    Conv::Thiscall(this),
                    &[i, 0xa, n, pairs, m, w],
                    |h| Out::U32(lcb::link_node(h, this, i, 0xa, n, pairs, m, w)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn node_tables_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lc::VECTOR_CTOR, false, 5, 5, 0x0c7);
        t.stub(lc::VECTOR_DTOR, false, 4, 4, 0x0d7);
        for fail in [false, true] {
            let block = t.junk(0x40, 0xdf);
            t.stub(lc::NEW_ARRAY, false, 1, 0, if fail { 0 } else { block });
            for old in [0u32, 1] {
                for n in [0u32, 3, 0x00e6_0000, 0xffff_ffff] {
                    let this = t.alloc(&[0; 8]);
                    if old == 1 {
                        let block = t.junk(0x40, 0xdb);
                        t.w32(block, 2);
                        t.w32(this, block + 4);
                    }
                    let what = format!("{label}: node table fail {fail} old {old} n {n:#x}");
                    t.check(&what, 0x005c_d9e0, Conv::Thiscall(this), &[n], |h| {
                        Out::U32(lcb::make_nodes(h, this, n))
                    });
                }
            }
        }
        t.reset();
    }
}

/// A `0x4c`-byte source record whose time controller (`+0x24`) answers
/// its scale (`+0x30`) plus `time`.
fn source_record(t: &mut Twins, at: u32, time: f32, w: f32) {
    let cell = t.alloc(&time.to_bits().to_le_bytes());
    let mut tail = vec![0xd9, 0x41, 0x0c, 0xd8, 0x05]; // fld dword [ecx+0xc]; fadd dword [cell]
    tail.extend(cell.to_le_bytes());
    tail.push(0xc3);
    let code = t.stub_code_tail(0x7a, true, 0, &tail);
    let vt = t.alloc(&[0; 0x14]);
    t.w32(vt + 0x10, code);
    t.w32(at + 0x24, vt);
    t.wf(at + 4, w);
    t.wf(at + 8, 2.5);
    t.wf(at + 0x34, 0.75);
}

#[test]
fn record_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let konst = t.rebase(sm::TIME_CONSTANT);
        t.wf(konst, 3.25);
        t.stub(lc::BLENDER_SET_SOURCE, true, 2, 2, 0x55);
        t.stub(lc::BLENDER_SET_WEIGHT, true, 2, 2, 0x56);
        t.stub(lc::COMPONENT_REFRESH, true, 0, 0, 0x57);
        t.stub(lc::RECORD_SET_TIME, true, 1, 1, 0);
        for (bind, range) in [(0u32, 0u32), (0x1d, 0), (0, 0x2e)] {
            t.stub(lc::RECORD_BIND, true, 1, 1, bind);
            t.stub_out2(lc::CLIP_RANGE, 1.5f32.to_bits(), 9.0f32.to_bits(), range);
            for time in [0.0f32, 4.5, -1.25, f32::NAN] {
                let rec = t.junk(0x4c, 0xdc);
                source_record(&mut t, rec, time, 0.5);
                let what =
                    format!("{label}: bind record bind {bind:#x} range {range:#x} time {time}");
                t.check_frame(
                    &what,
                    0x0054_3330,
                    Conv::Thiscall(rec),
                    &[0xc11b],
                    0x20,
                    |h| {
                        let frame = h.1;
                        Out::U32(lcb::bind_record(h, rec, 0xc11b, frame))
                    },
                );
                // through a component's source set
                let recs = t.junk(0x4c * 3, 0xdd);
                for k in 0..3 {
                    source_record(&mut t, recs + 0x4c * k, time, k as f32);
                }
                let this = t.junk(0x48, 0xde);
                t.w32(this + 0x14, 3 | sm::NOT_OWNED);
                t.w32(this + 0x18, recs);
                for (i, clip) in [(1u32, 0xc11bu32), (3, 0xc11b), (0, 0)] {
                    let what = format!(
                        "{label}: set source {i} clip {clip:#x} bind {bind:#x} range {range:#x} time {time}"
                    );
                    t.check_frame(
                        &what,
                        0x0054_3480,
                        Conv::Thiscall(this),
                        &[i, clip],
                        0x38,
                        |h| {
                            let frame = h.1;
                            Out::U32(lcb::set_source(h, this, i, clip, frame))
                        },
                    );
                }
                // all the sources' times (and none)
                let none = t.junk(0x48, 0xe0);
                t.w32(none + 0x14, sm::NOT_OWNED);
                t.check(
                    &format!("{label}: set no times"),
                    0x0054_4040,
                    Conv::Thiscall(none),
                    &[0],
                    |h| {
                        lcb::set_times(h, none, 0);
                        Out::None
                    },
                );
                for tt in [0.0f32, 2.0, f32::NAN] {
                    let what = format!("{label}: set times {tt} time {time}");
                    let b = tt.to_bits();
                    t.check(&what, 0x0054_4040, Conv::Thiscall(this), &[b], |h| {
                        lcb::set_times(h, this, b);
                        Out::None
                    });
                }
            }
        }
        t.reset();
    }
}

// ---- batch 4: sets, components, scene keys, shader parameters ----

impl Twins {
    /// `c` as code in both oracles.
    fn alloc_code_both(&mut self, c: &[u8]) -> u32 {
        let at = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), at);
        at
    }
}

#[test]
fn named_constructs_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ld::DEFAULT_NAME, false, 0, 0, 0xde7a);
        for n in [0u32, 1, 3] {
            let dst = t.junk(0x1c * 3 + 4, 0xe1);
            let what = format!("{label}: construct {n} named");
            t.check(&what, 0x0051_bb80, Conv::Cdecl, &[dst, n], |h| {
                ldb::construct_named(h, dst, n);
                Out::None
            });
        }
        t.reset();
    }
}

#[test]
fn small_arrays_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for fail in [false, true] {
            let fresh = t.junk(0x40, 0xe2);
            t.stub(
                ld::SMALL_ALLOC_COPY,
                false,
                3,
                3,
                if fail { 0 } else { fresh },
            );
            for raw in [0u32, 1, 3, 3 | sm::NOT_OWNED, 1 | sm::NOT_OWNED] {
                for n in [0u32, 1, 2, 3] {
                    let this = t.junk(0x10, 0xe3);
                    t.w32(this, raw);
                    let what = format!("{label}: small resize fail {fail} {raw:#x} -> {n}");
                    t.check(&what, 0x0051_d720, Conv::Thiscall(this), &[n], |h| {
                        Out::U32(ldb::small_resize(h, this, n))
                    });
                }
                // the allocation handing back the storage in place
                let this = t.junk(0x10, 0xe3);
                t.w32(this, raw);
                t.stub(ld::SMALL_ALLOC_COPY, true, 3, 3, this + 4);
                let what = format!("{label}: small resize in place {raw:#x}");
                t.check(&what, 0x0051_d720, Conv::Thiscall(this), &[5], |h| {
                    Out::U32(ldb::small_resize(h, this, 5))
                });
                t.stub(
                    ld::SMALL_ALLOC_COPY,
                    false,
                    3,
                    3,
                    if fail { 0 } else { fresh },
                );
            }
        }
        let src = t.junk(0x10, 0xe4);
        for raw in [0u32, 1, 3, 3 | sm::NOT_OWNED] {
            for n in [0u32, 1, 2, 0xffff_ffff] {
                for same in [false, true] {
                    let this = t.junk(0x10, 0xe5);
                    t.w32(this, raw);
                    let other = t.junk(0x10, 0xe4);
                    t.w32(this + 4, src);
                    // `same`: the buffer handed in is the one held
                    let ext = if same { src } else { other };
                    let what = format!("{label}: small external {raw:#x} n {n:#x} same {same}");
                    t.check(&what, 0x0051_dac0, Conv::Thiscall(this), &[ext, n], |h| {
                        Out::U32(ldb::small_set_external(h, this, ext, n))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn copy_resizes_match_the_original() {
    assert_eq!(ld::COPY_RESIZES.len(), 2);
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            for &(_, r) in ld::COPY_RESIZES {
                t.stub(r.copy, false, 3, 0, 0xc0);
                t.stub(r.construct, false, 2, 0, 0xc1);
                if r.release != 0 {
                    t.stub(r.release, false, 2, 0, 0xc2);
                }
            }
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x2000);
            }
            for &(f, r) in ld::COPY_RESIZES {
                for (raw, data) in [
                    (0u32, 0u32),
                    (2, 0x0bad_0010),
                    (2 | sm::NOT_OWNED, 0x0bad_0020),
                    (1, 0),
                ] {
                    for n in [0u32, 1, 2, 4] {
                        let this = t.junk(0x10, 0xe6);
                        t.w32(this, raw);
                        t.w32(this + 4, data);
                        let what =
                            format!("{label}: copy resize {f:#x} fail {fail} {raw:#x} -> {n}");
                        t.check(&what, f, Conv::Thiscall(this), &[n], |h| {
                            Out::U32(ldb::copy_resize(h, r, this, n))
                        });
                    }
                }
            }
            t.reset();
        }
    }
}

/// A `PAnimationSet`-like clip list (one kept in place) of clips named
/// `names` (`None`: a null clip; `Some("")`: a clip without a name).
fn clip_list(t: &mut Twins, names: &[Option<&str>]) -> u32 {
    let mut ptrs = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let c = match n {
            None => 0,
            Some(s) => {
                let c = t.junk(0x20, 0xe7);
                let w = if s.is_empty() {
                    0
                } else {
                    t.cstr(s) | (i as u32 & 1)
                };
                t.w32(c + 0x1c, w);
                c
            }
        };
        ptrs.push(c);
    }
    let this = t.zeros(0x10);
    t.w32(this, ptrs.len() as u32);
    if ptrs.len() == 1 {
        t.w32(this + 4, ptrs[0]);
    } else {
        let mut b = Vec::new();
        for p in &ptrs {
            b.extend(p.to_le_bytes());
        }
        b.extend([0; 4]);
        let data = t.alloc(&b);
        t.w32(this + 4, data);
    }
    this
}

#[test]
fn clip_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        let lists: [&[Option<&str>]; 4] = [
            &[],
            &[Some("walk")],
            &[None, Some("run_fast"), Some(""), Some("walk"), Some("run")],
            &[Some("idle_a"), Some("idle")],
        ];
        for names in lists {
            let this = clip_list(&mut t, names);
            for q in ["walk", "run", "fast", "idle", "zzz", "", "wal"] {
                let s = t.cstr(q);
                let what = format!("{label}: clip {q:?} in {names:?}");
                t.check(&what, 0x0051_ca40, Conv::Thiscall(this), &[s], |h| {
                    Out::U32(ldb::find_clip(h, this, s))
                });
            }
            let what = format!("{label}: clip null in {names:?}");
            t.check(&what, 0x0051_ca40, Conv::Thiscall(this), &[0], |h| {
                Out::U32(ldb::find_clip(h, this, 0))
            });
        }
    }
}

#[test]
fn row_scales_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut r = Rng(0x3344_5566);
        for case in 0..20u32 {
            let m = t.zeros(0x30);
            for k in 0..12 {
                let v = if case >= 16 && k == case - 16 {
                    FLOATS[(6 + case % 4) as usize]
                } else {
                    (((r.next() % 2000) as f32) - 1000.0).to_bits() ^ (r.next() & 0x3ff)
                };
                t.w32(m + 4 * k, v);
            }
            let s = t.zeros(0x10);
            for k in 0..3 {
                t.wf(s + 4 * k, ((r.next() % 300) as f32 - 150.0) / 13.0);
            }
            let out = if case == 5 { m } else { t.junk(0x30, 0xe8) };
            let what = format!("{label}: rows case {case}");
            t.check(&what, 0x0053_1310, Conv::Cdecl, &[out, m, s], |h| {
                Out::U32(ldb::scale_rows(h, out, m, s))
            });
        }
    }
}

#[test]
fn data_clears_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for case in 0..6u32 {
            let this = t.junk(0x40, 0xe9);
            for (k, arr) in [4u32, 0x20, 0x2c, 0xc].into_iter().enumerate() {
                let raw = [0u32, 3, 3 | sm::NOT_OWNED, 2, 1][(k + case as usize) % 5];
                t.w32(this + arr, raw);
                if (k as u32 + case).is_multiple_of(3) {
                    t.w32(this + arr + 4, 0);
                }
            }
            let what = format!("{label}: data clear {case}");
            t.check(&what, 0x0053_d100, Conv::Thiscall(this), &[], |h| {
                Out::U32(ldb::clear_data(h, this))
            });
        }
        t.reset();
    }
}

#[test]
fn record_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        for n in [0u32, 1, 3] {
            let src = t.junk(0x4c * 3, 0xea);
            // a quiet NaN and a denormal among the floats
            t.w32(src + 4, 0x7fc0_1234);
            t.w32(src + 0x18, 3);
            let dst = t.junk(0x4c * 3, 0xeb);
            let what = format!("{label}: copy {n} records");
            t.check(&what, 0x0053_d210, Conv::Cdecl, &[dst, src, n], |h| {
                ldb::copy_records(h, dst, src, n);
                Out::None
            });
        }
    }
}

#[test]
fn moves_to_end_match_the_original() {
    for (label, mut t) in Twins::all() {
        for n in [0u32, 1, 4] {
            for key in [0x10u32, 0x11, 0x13, 0x99] {
                let base = t.junk(0x94 * 6, 0xec);
                for i in 0..n {
                    t.w32(base + 0x94 * i, 0x10 + i);
                }
                let this = t.junk(0x10, 0xed);
                t.w32(this + 4, base);
                t.w32(this + 8, n);
                let what = format!("{label}: move {key:#x} of {n}");
                t.check(
                    &what,
                    0x005c_dad0,
                    Conv::Thiscall(this),
                    &[key, 0x5e],
                    |h| Out::U32(ldb::move_to_end(h, this, key)),
                );
            }
        }
    }
}

#[test]
fn shader_parameter_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let (da, db) = (t.rebase(ld::SHADER_DESC_A), t.rebase(ld::SHADER_DESC_B));
        let param = t.junk(0x10, 0xee);
        t.write(param + 0xa, &0xe007u16.to_le_bytes());
        let small = t.junk(0x10, 0xef);
        t.write(small + 0xa, &0x0009u16.to_le_bytes());
        // the lookup finds the parameter whose name word is odd
        for (lookup, bind) in [(param, 0xb1u32), (small, 0), (0, 0xb2)] {
            t.stub(ld::PARAM_LOOKUP, true, 1, 1, lookup);
            t.stub(ld::PARAM_BIND, true, 1, 1, bind);
            for own in [0u32, 1] {
                let mat = t.junk(0x10, 0xf0);
                let holder = t.junk(0x10, 0xf1);
                t.w32(holder + 4, mat);
                let this = t.junk(0x10, 0xf2);
                t.w32(this + 4, if own == 1 { mat } else { 0 });
                let mut tv = Vec::new();
                for (desc, obj, ty, name) in [
                    (da, mat, 5u32, "p0"),
                    (db, holder, 5, "p1"),
                    (db, 0, 5, "p2"),
                    (0x1234, mat, 5, "p3"),
                    (da, 0, 5, "p4"),
                    (da, mat, 4, "p5"),
                    (db, t.zeros(8), 5, "p6"),
                ] {
                    let s = t.cstr(name) | 1;
                    for w in [desc, obj, 0, 0, ty, s, 0] {
                        tv.extend(w.to_le_bytes());
                    }
                }
                let targets = t.alloc(&tv);
                let out = t.zeros(0x18 * 7);
                t.w32(out + 0x18 * 2, 9);
                for n in [0u32, 7] {
                    for (o, g) in [(out, targets), (0, targets), (out, 0)] {
                        let what = format!(
                            "{label}: shader bind {lookup:#x} {bind:#x} own {own} n {n} {o:#x} {g:#x}"
                        );
                        t.check(&what, 0x005c_de50, Conv::Thiscall(this), &[o, g, n], |h| {
                            Out::U32(ldb::bind_shader_params(h, this, o, g, n))
                        });
                    }
                }
            }
            t.reset();
        }
    }
}

/// A scene key (`+0` a word, `+4` the length, `+8` the bytes).
fn scene_key(t: &mut Twins, first: u32, bytes: &[u8]) -> u32 {
    let b = t.alloc(&[bytes, &[0x5a; 4]].concat());
    let k = t.zeros(12);
    t.w32(k, first);
    t.w32(k + 4, bytes.len() as u32);
    t.w32(k + 8, b);
    k
}

#[test]
fn scene_keys_and_tree_match_the_original() {
    for (label, mut t) in Twins::all() {
        let samples: [(u32, &[u8]); 10] = [
            (1, b"abcd"),
            (1, b"abce"),
            (1, b"abcdefg"),
            (1, b"abcdefh"),
            (2, b"abcd"),
            (0, b""),
            (1, b"zbcd"),
            (1, b"abcdefgh"),
            (1, b"abcdefgi"),
            (1, b"a"),
        ];
        let keys: Vec<u32> = samples
            .iter()
            .map(|&(f, b)| scene_key(&mut t, f, b))
            .collect();
        for &a in &keys {
            for &b in &keys {
                let what = format!("{label}: scene keys {a:#x} {b:#x}");
                t.check(&what, 0x0061_7260, Conv::Cdecl, &[a, b], |h| {
                    Out::U32(ldb::compare_scene_keys(h, a, b))
                });
            }
        }
        let hdr = t.junk(0x18, 0xf3);
        t.w32(hdr + 0xc, hdr);
        for (i, &(f, b)) in samples.iter().chain(samples[..3].iter()).enumerate() {
            let node = t.junk(0x18, 0xf4);
            let k = scene_key(&mut t, f, b);
            for w in 0..3 {
                let v = t.r32(k + 4 * w);
                t.w32(node + 0xc + 4 * w, v);
            }
            let what = format!("{label}: scene insert {i}");
            t.check(&what, 0x0061_70e0, Conv::Thiscall(hdr), &[node], |h| {
                Out::U32(ldb::scene_tree_insert(h, hdr, node))
            });
        }
    }
}

#[test]
fn inverse_blends_match_the_original() {
    for (label, mut t) in Twins::all() {
        // f(v) = v.x
        let f = t.alloc_code_both(&[0x8b, 0x44, 0x24, 0x04, 0xd9, 0x00, 0xc3]);
        let eps = t.rebase(ld::BLEND_EPSILON);
        let e: u32 = t.r32(eps);
        for (ax, bx) in [
            (1.0f32, 3.0f32),
            (2.0, 2.0),
            (5.0, 1.0),
            (f32::NAN, 1.0),
            (1.0, 1.0 + 1e-7),
            (-4.0, 4.0),
        ] {
            let a = t.junk(16, 0xf5);
            t.wf(a, ax);
            let b = t.junk(16, 0xf6);
            t.wf(b, bx);
            let out = t.junk(16, 0xf7);
            let what = format!(
                "{label}: inverse blend {ax} {bx} (eps {:e})",
                f32::from_bits(e)
            );
            t.check(&what, 0x00a2_f560, Conv::Cdecl, &[out, b, a, f], |h| {
                Out::U32(ldb::inverse_blend(h, out, b, a, f))
            });
        }
    }
}

/// A component for the clip-set changes: `+0x14`/`+0x18` `n` source
/// records, the blender at `+0x1c` (vtable slot 12 a recording stub).
fn component(t: &mut Twins, n: u32, set: u32) -> u32 {
    let recs = t.junk(0x4c * 4, 0xf8);
    for i in 0..n {
        let rec = recs + 0x4c * i;
        t.w32(rec, i & 1);
        t.w32(rec + 0x40, if i == 1 { set } else { 0x4040 });
    }
    let this = t.junk(0x50, 0xf9);
    t.w32(this + 0x14, n);
    t.w32(this + 0x18, recs);
    // the blender's slot 12 (its resize) answers 0
    let blender_vt = {
        let obj = t.object(&[1; 13], &[0; 13]);
        t.r32(obj)
    };
    t.w32(this + 0x1c, blender_vt);
    t.w32(this + 0x10, set);
    this
}

/// A clip set of `n` clips (one kept in place).
fn clip_set(t: &mut Twins, n: u32) -> u32 {
    let set = t.zeros(0x10);
    t.w32(set, n);
    if n == 1 {
        t.w32(set + 4, 0xc1);
    } else if n > 1 {
        let data = t.alloc(&[0xc2, 0, 0, 0, 0xc3, 0, 0, 0, 0, 0, 0, 0]);
        t.w32(set + 4, data);
    }
    set
}

#[test]
fn clip_set_changes_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ld::DATA_CLEAR, true, 0, 0, 0xd1);
        t.stub(ld::BLENDER_SET_WEIGHT, true, 2, 2, 0xd2);
        t.stub(ld::RECORD_BIND, true, 1, 1, 0xd3);
        t.stub(ld::COMPONENT_REFRESH, true, 0, 0, 0xd4);
        for (bind, time, source) in [(0u32, 0u32, 0u32), (0x1b, 0, 0), (0, 0x2b, 0), (0, 0, 0x3b)] {
            t.stub(ld::CLIP_SET_BIND, true, 1, 1, bind);
            t.stub(ld::RECORD_SET_TIME, true, 1, 1, time);
            t.stub(ld::BLENDER_SET_SOURCE, true, 2, 2, source);
            for k in [None, Some(0u32), Some(1), Some(2)] {
                let set = k.map_or(0, |k| clip_set(&mut t, k));
                for n in [0u32, 3] {
                    let this = component(&mut t, n, set);
                    let what = format!(
                        "{label}: clip set {k:?} n {n} errors {bind:#x} {time:#x} {source:#x}"
                    );
                    t.check(&what, 0x0054_3c60, Conv::Thiscall(this), &[set], |h| {
                        Out::U32(ldb::set_clip_set(h, this, set))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn source_counts_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ld::BLENDER_SET_WEIGHT, true, 2, 2, 0xd2);
        t.stub(ld::RECORD_BIND, true, 1, 1, 0xd3);
        t.stub(ld::COMPONENT_REFRESH, true, 0, 0, 0xd4);
        for (resize, time, source) in [(0u32, 0u32, 0u32), (0x1c, 0, 0), (0, 0x2c, 0), (0, 0, 0x3c)]
        {
            // the records' resize keeps the storage and sets the count
            t.stub(ld::RECORD_SET_TIME, true, 1, 1, time);
            t.stub(ld::BLENDER_SET_SOURCE, true, 2, 2, source);
            for set_n in [None, Some(1u32), Some(2)] {
                let set = set_n.map_or(0, |k| clip_set(&mut t, k));
                for (old, n) in [(1u32, 3u32), (3, 3), (3, 1), (0, 2)] {
                    let this = component(&mut t, old, set);
                    if resize != 0 {
                        t.stub(ld::RECORDS_RESIZE, true, 1, 1, resize);
                    } else {
                        // a resize that only stores the count
                        let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x89, 0x01]; // mov eax, [esp+4]; mov [ecx], eax
                        tail.extend([0x31, 0xc0]); // xor eax, eax
                        tail.extend(ret_bytes(1));
                        let code = t.stub_code_tail(ld::RECORDS_RESIZE, true, 1, &tail);
                        t.hook(ld::RECORDS_RESIZE, code);
                    }
                    let what = format!(
                        "{label}: source count {old} -> {n} set {set_n:?} errors {resize:#x} {time:#x} {source:#x}"
                    );
                    t.check(&what, 0x0054_3e50, Conv::Thiscall(this), &[n], |h| {
                        Out::U32(ldb::set_source_count(h, this, n))
                    });
                }
            }
        }
        t.reset();
    }
}

// ---- batch 5: callable binds, scene binds, the optimiser's run ----

#[test]
fn callable_binds_match_the_original() {
    assert_eq!(le::CALLABLE_BINDS.len(), 2);
    for (label, mut t) in Twins::all() {
        t.stub(le::PRINTF, false, 3, 0, 0x99);
        t.stub(le::VECTOR_TYPE, false, 0, 0, 0x40);
        // a class chain c0 -> c1 -> c2 (through +0x40)
        let c2 = t.zeros(0x50);
        let c1 = t.zeros(0x50);
        t.w32(c1 + 0x40, c2);
        let c0 = t.zeros(0x50);
        t.w32(c0 + 0x40, c1);
        // signatures: good for both kinds, bad, and a callable without one
        let good = t.alloc(&[0x41, 0, 0, 0, 0x41, 0, 0, 0, 0, 0, 0, 0]);
        let bad = t.alloc(&[0x41, 0, 0, 0, 7, 0, 0, 0, 7, 0, 0, 0]);
        let mut callables = Vec::new();
        for sig in [good, bad] {
            let code = t.stub_code(0x7d, true, 0, 0, sig);
            let x = t.object_with(4, code);
            let p = t.junk(0x20, 0xa1);
            t.w32(p + 0x14, x);
            callables.push(p);
        }
        for &(f, k) in le::CALLABLE_BINDS {
            for (found, obj8) in [(callables[0], 0u32), (callables[1], 0x0b8), (0, 0)] {
                t.stub(k.lookup, true, 1, 1, found);
                for head in [c0, 0] {
                    let this = t.junk(0x10, 0xa2);
                    t.w32(this + 4, head);
                    t.w32(this + 8, obj8);
                    let mut tv = Vec::new();
                    for (class, ty, name) in [
                        (c1, k.ty, "f0"),
                        (c2, k.ty, "f1"),
                        (0x7777, k.ty, "f2"),
                        (c0, k.ty ^ 7, "f3"),
                        (c2, k.ty, "f4"),
                    ] {
                        let s = t.cstr(name) | 1;
                        for w in [class, 0x0b1e, 0, 0, ty, s, 0] {
                            tv.extend(w.to_le_bytes());
                        }
                    }
                    let targets = t.alloc(&tv);
                    let out = t.zeros(0x18 * 5);
                    t.w32(out + 0x18 * 4, 5);
                    for n in [0u32, 5] {
                        for (o, g) in [(out, targets), (0, targets), (out, 0)] {
                            let what = format!(
                                "{label}: callables {f:#x} found {found:#x} head {head:#x} n {n} {o:#x} {g:#x}"
                            );
                            t.check(&what, f, Conv::Thiscall(this), &[o, g, n], |h| {
                                Out::U32(leb::bind_callables(h, k, this, o, g, n))
                            });
                        }
                    }
                }
            }
        }
        t.reset();
    }
}

/// The scene bind's hash of a name.
fn scene_hash(name: &str) -> u32 {
    name.bytes().fold(0x7b5u32, |h, c| {
        h.wrapping_mul(33).wrapping_add(u32::from(c & 0x1f))
    })
}

/// A scene bind object: `+4` slots, `+8` 0x18-byte entries keyed by the
/// names' scene keys, the tree at `+0xc` (built with the original insert
/// in both oracles).
fn scene_bind(t: &mut Twins, names: &[&str]) -> u32 {
    let this = t.junk(0x20, 0xa3);
    let slots = t.zeros(4 * names.len() + 4);
    let entries = t.junk(0x18 * names.len() + 4, 0xa4);
    t.w32(this + 4, slots);
    t.w32(this + 8, entries);
    let hdr = this + 0xc;
    t.w32(hdr + 0xc, hdr);
    for (i, name) in names.iter().enumerate() {
        let e = entries + 0x18 * i as u32;
        let s = t.cstr(name);
        t.w32(e + 0xc, scene_hash(name));
        t.w32(e + 0x10, name.len() as u32);
        t.w32(e + 0x14, s);
        let at = t.rebase(0x0061_70e0);
        t.a.call(at, Conv::Thiscall(hdr), &[e]).unwrap();
        t.b.call(at, Conv::Thiscall(hdr), &[e]).unwrap();
    }
    this
}

/// A scene node named `name` (`None`: no name word, `Some("")`: empty).
fn scene_node(t: &mut Twins, name: Option<&str>) -> u32 {
    let n = t.zeros(0x60);
    let w = match name {
        None => 0,
        Some(s) => t.cstr(s) | 1,
    };
    t.w32(n + 0x50, w);
    n
}

#[test]
fn scene_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        // "Arm_L" and "arm_l" share a hash: the bytes decide
        let names = [
            "root", "spine", "head", "arm_l", "arm_r", "leg", "", "Arm_L",
        ];
        let this = scene_bind(&mut t, &names);
        // lookups by key
        for q in [
            "spine", "leg", "hand", "", "arm_r", "spinf", "arm_l", "Arm_L", "ARM_L",
        ] {
            let s = t.cstr(q);
            let key = t.zeros(12);
            t.w32(key, scene_hash(q));
            t.w32(key + 4, q.len() as u32);
            t.w32(key + 8, s);
            let tree = this + 0xc;
            let what = format!("{label}: scene find {q:?}");
            t.check(&what, 0x0061_6fd0, Conv::Thiscall(tree), &[key], |h| {
                Out::U32(leb::scene_tree_find(h, tree, key))
            });
        }
        // single nodes
        for name in [
            Some("head"),
            Some("hand"),
            Some(""),
            None,
            Some("head"),
            Some("ARM_L"),
            Some("Arm_L"),
            Some("arm_l"),
        ] {
            let node = scene_node(&mut t, name);
            let what = format!("{label}: scene node {name:?}");
            t.check(&what, 0x0061_74b0, Conv::Thiscall(this), &[node], |h| {
                Out::U32(leb::bind_scene_node(h, this, node))
            });
        }
        // a tree: root { a { b, c { d } }, e }
        for shape in 0..3u32 {
            let this = scene_bind(&mut t, &names);
            let n: Vec<u32> = ["root", "spine", "arm_l", "x", "head", "leg"]
                .iter()
                .map(|s| scene_node(&mut t, Some(s)))
                .collect();
            let link = |t: &mut Twins, parent: u32, kids: &[u32]| {
                if let Some(&first) = kids.first() {
                    t.w32(parent + 8, first);
                }
                for w in kids.windows(2) {
                    t.w32(w[0], w[1]);
                }
                for &k in kids {
                    t.w32(k + 4, parent);
                }
            };
            match shape {
                0 => {
                    link(&mut t, n[0], &[n[1], n[5]]);
                    link(&mut t, n[1], &[n[2], n[3]]);
                    link(&mut t, n[3], &[n[4]]);
                }
                1 => link(&mut t, n[0], &[n[1], n[2], n[3]]),
                _ => {}
            }
            let root = n[0];
            let what = format!("{label}: scene tree shape {shape}");
            t.check(&what, 0x0061_6ee0, Conv::Cdecl, &[this, root], |h| {
                Out::U32(leb::bind_scene_tree(h, this, root))
            });
        }
    }
}

/// An optimiser of `inputs.len()` nodes; node `i` takes inputs from the
/// nodes `inputs[i]` (its links at `+0x104`, count `+0x100`).
fn optimiser(t: &mut Twins, inputs: &[&[u32]]) -> u32 {
    let n = inputs.len() as u32;
    let nodes = t.zeros(0x11c * n as usize + 4);
    for (i, ins) in inputs.iter().enumerate() {
        let node = nodes + 0x11c * i as u32;
        let mut links = Vec::new();
        for &j in *ins {
            links.extend((nodes + 0x11c * j).to_le_bytes());
            links.extend(0u32.to_le_bytes());
        }
        links.extend([0; 4]);
        let l = t.alloc(&links);
        t.w32(node + 0x100, ins.len() as u32);
        t.w32(node + 0x104, l);
        t.w32(node + 0x118, 5);
    }
    let this = t.alloc(&[0; 8]);
    t.w32(this, nodes);
    t.w32(this + 4, n);
    this
}

#[test]
fn optimiser_runs_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(le::CONTEXT_CTOR, true, 0, 0, 0xc7);
        t.stub(le::CONTEXT_DTOR, true, 0, 0, 0x1234_5678);
        t.stub(le::NODE_RELEASE, true, 1, 1, 0x7e1);
        t.stub(le::NODE_RESOLVE, true, 1, 1, 0x7e2);
        let graphs: [&[&[u32]]; 4] = [&[], &[&[]], &[&[1], &[], &[0, 1]], &[&[0], &[]]];
        for inputs in graphs {
            for first in [0u32, 1, 0x100] {
                let this = optimiser(&mut t, inputs);
                let what = format!("{label}: run {inputs:?} release first {first:#x}");
                t.check_frame(
                    &what,
                    0x005c_d7a0,
                    Conv::Thiscall(this),
                    &[first],
                    0x38,
                    |h| {
                        let frame = h.1;
                        Out::U32(leb::run(h, this, first, frame))
                    },
                );
            }
        }
        t.reset();
    }
}

#[test]
fn first_source_uses_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ld::BLENDER_SET_WEIGHT, true, 2, 2, 0xd2);
        t.stub(ld::RECORD_BIND, true, 1, 1, 0xd3);
        t.stub(ld::COMPONENT_REFRESH, true, 0, 0, 0xd4);
        t.stub(ld::RECORD_SET_TIME, true, 1, 1, 0);
        t.stub(0x0054_1780, false, 2, 0, 0xc1);
        t.stub(0x0054_0b80, false, 2, 0, 0xc2);
        for (fail, source) in [(false, 0u32), (true, 0), (false, 0x3e)] {
            t.stub(ld::BLENDER_SET_SOURCE, true, 2, 2, source);
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x400);
            }
            for set_n in [None, Some(1u32), Some(2)] {
                let set = set_n.map_or(0, |k| clip_set(&mut t, k));
                for n in [0u32, 1, 3] {
                    for arg in [0u32, 0xa9] {
                        let this = component(&mut t, n, set);
                        let what = format!(
                            "{label}: first source n {n} set {set_n:?} arg {arg:#x} fail {fail} source {source:#x}"
                        );
                        t.check(&what, 0x0054_3570, Conv::Thiscall(this), &[arg], |h| {
                            Out::U32(leb::use_first_source(h, this, arg))
                        });
                    }
                }
            }
        }
        t.reset();
    }
}

// ---- batch 6: the blenders' kernels ----

#[test]
fn blend_kernels_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut r = Rng(0x6006_1234);
        for case in 0..48u32 {
            let n = [0u32, 1, 2, 3, 5][(case % 5) as usize];
            let mut ptrs = Vec::new();
            let mut ws = Vec::new();
            for i in 0..n {
                let q = t.zeros(16);
                for k in 0..4 {
                    let v = ((r.next() % 2001) as f32 - 1000.0) / 1000.0;
                    t.wf(q + 4 * k, v);
                }
                if case >= 40 && i == 1 {
                    t.w32(q + 4 * (r.next() % 4), FLOATS[6 + (case % 4) as usize]);
                }
                // some null sources
                let p = if (case + i) % 7 == 3 { 0 } else { q };
                ptrs.extend(p.to_le_bytes());
                let w = match (case / 5) % 4 {
                    0 => ((r.next() % 1000) as f32) / 999.0,
                    1 => ((r.next() % 1000) as f32) / 999.0 - 0.5,
                    2 => [0.0f32, 1.0, 0.25, 0.25, f32::NAN][i as usize % 5],
                    _ => 0.0,
                };
                ws.extend(w.to_bits().to_le_bytes());
            }
            ptrs.extend([0; 4]);
            ws.extend([0; 4]);
            let ptrs = t.alloc(&ptrs);
            let wdata = t.alloc(&ws);
            // the relative pointer sits a little before the weights
            let wref = t.zeros(8);
            t.w32(wref, wdata.wrapping_sub(wref));
            for mode in [0u32, 1, 2, 3] {
                for (f, add) in [(0x0052_dae0, false), (0x0052_ef70, true)] {
                    for nulls in [false, true] {
                        let out = if nulls && case % 2 == 0 {
                            0
                        } else {
                            t.junk(16, 0xb7)
                        };
                        let p = if nulls && case % 2 == 1 { 0 } else { ptrs };
                        if nulls && out != 0 && p != 0 {
                            continue;
                        }
                        let what =
                            format!("{label}: kernel {f:#x} case {case} mode {mode} nulls {nulls}");
                        t.check(&what, f, Conv::Cdecl, &[out, mode, p, n, wref], |h| {
                            let v = if add {
                                lfb::additive(h, out, mode, p, n, wref)
                            } else {
                                lfb::weighted(h, out, mode, p, n, wref)
                            };
                            Out::Al(v as u8)
                        });
                    }
                }
            }
        }
    }
}

#[test]
fn target_applies_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(lf::LOG, false, 3, 0, 0x106);
        let (rot, trans, scale) = (
            t.rebase(lf::ROT_TYPE),
            t.rebase(lf::TRANS_TYPE),
            t.rebase(lf::SCALE_TYPE),
        );
        let mut r = Rng(0x0a11_7a11);
        // runs of (target, types); targets 0..5
        let runs: [&[(u32, u32)]; 6] = [
            &[(5, 3), (0, 1), (2, 2)],
            &[(0, 1), (0, 2), (0, 3)],
            &[(1, 1), (2, 3), (2, 2), (3, 9), (4, 1), (4, 9)],
            &[(5, 2), (5, 3), (0, 9), (1, 9)],
            &[(3, 1), (3, 0), (2, 1)],
            &[],
        ];
        for (case, run) in runs.iter().enumerate() {
            // values: 32 bytes each, the record order reversed
            let n = run.len() as u32;
            let vals = t.zeros(32 * run.len() + 32);
            let recs = t.zeros(16 * run.len() + 16);
            for (i, &(tgt, ty)) in run.iter().enumerate() {
                let i = i as u32;
                let idx = n - 1 - i;
                t.w32(recs + 16 * i + 0xc, idx);
                t.w32(recs + 16 * idx + 8, tgt);
                let v = vals + 32 * idx;
                for k in 0..4 {
                    let x = ((r.next() % 2001) as f32 - 1000.0) / 250.0;
                    t.wf(v + 4 * k, x);
                }
                let tyw = match ty {
                    0 => 0,
                    1 => rot,
                    2 => scale,
                    3 => trans,
                    _ => 0x0e0e,
                };
                t.w32(v + 0x10, tyw);
            }
            let owner = t.zeros(0x20);
            t.w32(owner + 0x10, n | if case == 2 { sm::NOT_OWNED } else { 0 });
            t.w32(owner + 0x14, recs);
            let this = t.zeros(0x10);
            t.w32(this + 8, owner);
            let values = t.alloc(&vals.to_le_bytes());
            // binds: kind per target, a destination for each
            let table = t.zeros(24 * 6);
            for (tgt, kind) in [(0u32, 1u32), (1, 2), (2, 1), (3, 3), (4, 0), (5, 1)] {
                let e = table + 24 * tgt;
                let dst = t.junk(0x40, 0xc0 + tgt as u8);
                t.w32(e, if case == 4 && tgt == 2 { 0 } else { kind });
                t.w32(e + 4, dst);
                t.w32(e + 8, 12);
                t.w32(e + 0xc, 0xf00 + tgt);
                t.w32(e + 0x10, 0xb00 + tgt);
                t.w32(e + 0x14, 0xc00 + tgt);
            }
            let binds = t.zeros(0x20);
            t.w32(binds + 0x10, table);
            for b in [binds, 0] {
                let what = format!("{label}: apply targets case {case} binds {b:#x}");
                t.check_frame(
                    &what,
                    0x0053_1660,
                    Conv::Thiscall(this),
                    &[b, values],
                    0x134,
                    |h| {
                        let frame = h.1;
                        Out::U32(lfb::apply_targets(h, this, b, values, frame))
                    },
                );
            }
        }
        assert!(t.log_entries() > 0);
        t.reset();
    }
}

// ---- batch 7: PAnimationSet's packed data ----

#[test]
fn set_data_builds_match_the_original() {
    for (label, mut t) in Twins::all() {
        let slots_at = t.rebase(lg::KEY_SLOTS);
        t.w32(slots_at, 16);
        t.stub(lg::LOOKUP_CTOR, true, 1, 1, 0x1c7);
        t.stub(lg::LOOKUP_DTOR, true, 0, 0, 0x1d7);
        t.stub(lg::TABLE_CTOR, true, 1, 1, 0x2c7);
        t.stub(lg::TABLE_DTOR, true, 0, 0, 0x2d7);
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        // the lookup's key: the item's +0x20 word; the table's answer:
        // the key's class plus its lookup key (so the key it saw shows)
        let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x20]; // mov eax,[esp+4]; mov eax,[eax+0x20]
        tail.extend(ret_bytes(1));
        let code = t.stub_code_tail(lg::LOOKUP_KEY, true, 1, &tail);
        t.hook(lg::LOOKUP_KEY, code);
        let mut tail = vec![
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x50, 0x08, 0x03, 0x50, 0x04, 0x8b, 0x00, 0x01, 0xd0,
        ]; // eax = [k] + [k+4] + [k+8]
        tail.extend(ret_bytes(1));
        let code = t.stub_code_tail(lg::TABLE_FIND, true, 1, &tail);
        t.hook(lg::TABLE_FIND, code);
        for fail in [false, true] {
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            for (nch, nslots) in [(0u32, 0u32), (1, 0), (5, 2), (7, 3), (0, 2)] {
                let mut list = Vec::new();
                for i in 0..nch {
                    let ch = t.junk(0x40, 0x30 + i as u8);
                    let class = t.junk(0x10, 0x40 + i as u8);
                    // a few classes, keys from them and +0x20
                    t.w32(class + 8, 0x10 + (i % 3));
                    t.w32(class + 0xc, i % 2);
                    let target = t.junk(0x10, 0x50 + i as u8);
                    t.w32(ch + 0x1c, class);
                    t.w32(ch + 0x20, (i * 5) % 7);
                    t.w32(ch + 0x24, target);
                    list.extend(ch.to_le_bytes());
                }
                list.extend([0; 4]);
                let list = t.alloc(&list);
                let slots = t.junk(0x34 * nslots as usize + 4, 0x60);
                for j in 0..nslots {
                    let class = t.junk(0x10, 0x70 + j as u8);
                    t.w32(slots + 0x34 * j + 0x1c, class);
                }
                let set = t.junk(0x20, 0x61);
                t.w32(set + 4, nch);
                t.w32(set + 8, list);
                t.w32(
                    set + 0xc,
                    nslots | if nslots == 3 { sm::NOT_OWNED } else { 0 },
                );
                t.w32(set + 0x10, slots);
                let this = t.junk(0x200, 0x62);
                let what = format!("{label}: set data fail {fail} {nch} channels {nslots} slots");
                t.check_frame(
                    &what,
                    0x0051_14b0,
                    Conv::Thiscall(this),
                    &[0x7a6, set],
                    0xa0,
                    |h| {
                        let frame = h.1;
                        Out::U32(lgb::build_set_data(h, this, 0x7a6, set, frame))
                    },
                );
            }
        }
        t.reset();
    }
}

// ---- batch 8: PAnimationSet's slot list ----

/// A clip's channels and constant channels: (class, target) each.
type ClipSpec = (Vec<(usize, u32)>, Vec<(usize, u32)>);

#[test]
fn slot_lists_match_the_original() {
    for (label, mut t) in Twins::all() {
        let classes_at = t.rebase(lh::CLASSES);
        t.w32(classes_at, 4);
        t.stub(lh::LOOKUP_CTOR, true, 1, 1, 0x1c7);
        t.stub(lh::LOOKUP_DTOR, true, 0, 0, 0x1d7);
        t.stub(0x0051_7900, false, 3, 0, 0xc0);
        t.stub(0x0051_bc30, false, 2, 0, 0xc1);
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        // the target of an item: its +0x24
        let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x24]; // mov eax,[esp+4]; mov eax,[eax+0x24]
        tail.extend(ret_bytes(1));
        let code = t.stub_code_tail(lh::LOOKUP_KEY, true, 1, &tail);
        t.hook(lh::LOOKUP_KEY, code);
        let cls: Vec<u32> = (0..5u32)
            .map(|g| {
                let c = t.junk(0x10, 0x90 + g as u8);
                t.w32(c + 0xc, g % 4);
                c
            })
            .collect();
        for (case, fail) in [(0u32, false), (1, false), (2, false), (3, true), (4, false)] {
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            let n_targets = 6u32;
            // clips: channels (class, target) and constant channels
            let clip_specs: Vec<ClipSpec> = match case {
                0 => vec![],
                1 => vec![(
                    vec![(0, 1), (1, 2), (0, 1), (2, 0)],
                    vec![(1, 3), (3, 7), (3, 5)],
                )],
                2 => vec![
                    (vec![(0, 1), (4, 1), (2, 5)], vec![]),
                    (vec![(0, 2), (0, 1)], vec![(1, 0), (2, 5), (0, 0)]),
                    (vec![], vec![]),
                ],
                _ => vec![
                    (vec![(1, 4), (3, 4), (2, 2)], vec![(1, 4)]),
                    (vec![(1, 2)], vec![]),
                ],
            };
            let mut clips = Vec::new();
            for (chs, ccs) in &clip_specs {
                let clip = t.junk(0x40, 0xa0);
                let mut list = Vec::new();
                for &(c, tg) in chs {
                    let ch = t.junk(0x40, 0xa1);
                    t.w32(ch + 0x1c, cls[c]);
                    t.w32(ch + 0x24, tg);
                    list.extend(ch.to_le_bytes());
                }
                list.extend(0u32.to_le_bytes());
                list.extend([0; 4]);
                let list = t.alloc(&list);
                t.w32(clip + 4, chs.len() as u32 + 1);
                t.w32(clip + 8, list);
                let recs = t.junk(0x34 * ccs.len() + 4, 0xa2);
                for (j, &(c, tg)) in ccs.iter().enumerate() {
                    let cc = recs + 0x34 * j as u32;
                    t.w32(cc + 0x1c, cls[c]);
                    t.w32(cc + 0x24, tg);
                }
                t.w32(clip + 0xc, ccs.len() as u32);
                t.w32(clip + 0x10, recs);
                clips.push(clip);
            }
            if case == 2 {
                clips.push(0);
            }
            let this = t.junk(0x20, 0xa3);
            t.w32(this, clips.len() as u32);
            if clips.len() == 1 {
                t.w32(this + 4, clips[0]);
            } else {
                let mut b = Vec::new();
                for c in &clips {
                    b.extend(c.to_le_bytes());
                }
                b.extend([0; 4]);
                let data = t.alloc(&b);
                t.w32(this + 4, data);
            }
            t.w32(this + 8, n_targets);
            t.w32(this + 0x10, 0);
            t.w32(this + 0x14, 0);
            let what = format!("{label}: slot list case {case}");
            t.check_frame(&what, 0x0051_de90, Conv::Thiscall(this), &[], 0x68, |h| {
                let frame = h.1;
                Out::U32(lhb::build_slots(h, this, frame))
            });
        }
        t.reset();
    }
}

// ---- batch 9: PAnimationSet's targets ----

#[test]
fn target_rebuilds_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(li::TARGETS_RELEASE, false, 2, 0, 0x7e1);
        t.stub(li::TARGET_ASSIGN, true, 1, 1, 0xa55);
        t.stub(li::TARGETS_CONSTRUCT, false, 2, 0, 0xc0);
        t.stub(li::CLIP_REBUILD, true, 2, 2, 0xc1);
        // targets are equal when their first words are
        let mut tail = vec![
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x3b, 0x01, 0x0f, 0x94, 0xc0,
        ];
        tail.extend(ret_bytes(1));
        let code = t.stub_code_tail(li::TARGET_EQUALS, true, 1, &tail);
        t.hook(li::TARGET_EQUALS, code);
        for (case, fail, slots) in [
            (0u32, 0u32, 0u32),
            (1, 0, 0),
            (2, 0, 0x5e),
            (3, 1, 0),
            (4, 2, 0),
            (5, 0, 0),
        ] {
            t.stub(li::BUILD_SLOTS, true, 0, 0, slots);
            match fail {
                1 => t.stub(la::PHYRE_MALLOC, false, 2, 0, 0),
                // the list succeeds, the target array fails
                2 => {
                    let buf = t.junk(0x100, 0x11);
                    let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x3d, 0x54, 0, 0, 0, 0xb8];
                    tail.extend(buf.to_le_bytes());
                    tail.extend([0x75, 0x02, 0x31, 0xc0, 0xc3]); // 0 for 0x54 bytes
                    let code = t.stub_code_tail(la::PHYRE_MALLOC, false, 2, &tail);
                    t.hook(la::PHYRE_MALLOC, code);
                }
                _ => t.stub_bump(la::PHYRE_MALLOC, 0x800),
            }
            // items: first words decide equality
            let mk = |t: &mut Twins, key: u32| {
                let it = t.junk(0x34, 0x12);
                t.w32(it, key);
                it
            };
            let specs: Vec<(Vec<u32>, Vec<u32>)> = match case {
                0 => vec![],
                1..=4 => vec![
                    (vec![1, 2, 1], vec![3, 2]),
                    (vec![], vec![]),
                    (vec![3], vec![]),
                ],
                _ => vec![(vec![7, 7], vec![])],
            };
            let mut clips = Vec::new();
            for (chs, ccs) in &specs {
                let clip = t.junk(0x40, 0x13);
                let mut list = Vec::new();
                for &k in chs {
                    let it = mk(&mut t, k);
                    list.extend(it.to_le_bytes());
                }
                list.extend([0; 4]);
                let list = t.alloc(&list);
                t.w32(clip + 4, chs.len() as u32);
                t.w32(clip + 8, list);
                let recs = t.junk(0x34 * ccs.len() + 4, 0x14);
                for (j, &k) in ccs.iter().enumerate() {
                    t.w32(recs + 0x34 * j as u32, k);
                }
                t.w32(clip + 0xc, ccs.len() as u32);
                t.w32(clip + 0x10, recs);
                clips.push(clip);
            }
            // (a null clip faults in the original's count loop)
            let this = t.junk(0x20, 0x15);
            t.w32(this, clips.len() as u32);
            if clips.len() == 1 {
                t.w32(this + 4, clips[0]);
            } else {
                let mut b = Vec::new();
                for c in &clips {
                    b.extend(c.to_le_bytes());
                }
                b.extend([0; 4]);
                let data = t.alloc(&b);
                t.w32(this + 4, data);
            }
            // the old targets: owned with names, or none
            let old = t.junk(0x1c * 2, 0x16);
            let name = t.cstr("tgt");
            t.w32(old + 0x14, name);
            t.w32(old + 0x1c + 0x14, name | 1);
            let (raw, data) = match case {
                2 => (3, old),
                3 => (2 | sm::NOT_OWNED, old),
                4 => (0, 0),
                5 => (1, old),
                _ => (2, old),
            };
            t.w32(this + 8, raw);
            t.w32(this + 0xc, data);
            let what = format!("{label}: target rebuild case {case}");
            t.check(&what, 0x0051_c490, Conv::Thiscall(this), &[0xc1c], |h| {
                Out::U32(lib::rebuild_targets(h, this, 0xc1c))
            });
            t.reset();
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            t.stub(li::TARGETS_RELEASE, false, 2, 0, 0x7e1);
            t.stub(li::TARGET_ASSIGN, true, 1, 1, 0xa55);
            t.stub(li::TARGETS_CONSTRUCT, false, 2, 0, 0xc0);
            t.stub(li::CLIP_REBUILD, true, 2, 2, 0xc1);
            t.hook(li::TARGET_EQUALS, code);
        }
    }
}

// ---- batch 10: the component's evaluation order ----

impl Twins {
    /// A recording `thiscall` stub at `sym` taking `n` words that answers
    /// `(word[arg] + add) & mask`.
    fn stub_arg(&mut self, sym: u32, n: u32, arg: u32, add: u8, mask: u8) {
        let mut tail = vec![0x8b, 0x44, 0x24, (4 + 4 * arg) as u8]; // mov eax, [esp+4+4arg]
        tail.extend([0x83, 0xc0, add, 0x83, 0xe0, mask]); // add eax, add; and eax, mask
        tail.extend(ret_bytes(n));
        let code = self.stub_code_tail(sym, true, n, &tail);
        self.hook(sym, code);
    }
}

#[test]
fn record_orders_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lj::OPT_CTOR, true, 0, 0, 0x1c7);
        t.stub(lj::OPT_DTOR, true, 0, 0, 0x1d7);
        t.stub(lj::OPT_MAKE_NODES, true, 1, 1, 1);
        t.stub(lj::OPT_LINK, true, 6, 6, 0x101);
        t.stub(lj::OPT_RUN, true, 1, 1, 0x201);
        t.stub_arg(lj::OPT_LEVEL, 2, 0, 1, 1);
        t.stub_arg(lj::OPT_NODE, 1, 0, 1, 1);
        t.stub_arg(lj::OPT_INPUTS, 1, 0, 0, 1);
        t.stub_arg(lj::OPT_INPUT, 2, 1, 1, 1);
        for (case, n, nb, fail) in [
            (0u32, 0u32, 0u32, false),
            (1, 3, 3, false),
            (2, 4, 4, false),
            (3, 4, 4, true),
            (4, 2, 2, false),
            (5, 1, 1, false),
        ] {
            t.stub(lj::OPT_COUNT, true, 0, 0, nb);
            // case 5: a single node at level 0
            t.stub_arg(lj::OPT_LEVEL, 2, 0, u8::from(case != 5), 1);
            t.stub_arg(lj::OPT_NODE, 1, 0, u8::from(case != 5), 1);
            if fail {
                // only the level array's 8 bytes fail
                let buf = t.junk(0x100, 0x35);
                let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x3d, 8, 0, 0, 0, 0xb8];
                tail.extend(buf.to_le_bytes());
                tail.extend([0x75, 0x02, 0x31, 0xc0, 0xc3]);
                let code = t.stub_code_tail(la::PHYRE_MALLOC, false, 2, &tail);
                t.hook(la::PHYRE_MALLOC, code);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            let recs = t.junk(0x94 * 5, 0x31);
            let mut srcs = Vec::new();
            for i in 0..n {
                // source i has inputs: the sources before it, a stranger, a null
                let inputs = t.zeros(0x20);
                for j in 0..i {
                    let prev = srcs[j as usize];
                    t.w32(inputs + 4 * j, prev);
                }
                t.w32(inputs + 4 * i, 0x7777);
                let m = i + 2;
                let count = t.stub_code(0x7900 + i, true, 0, 0, m);
                let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x04, 0x85];
                tail.extend(inputs.to_le_bytes()); // mov eax, [inputs + 4*arg]
                tail.extend(ret_bytes(1));
                let get = t.stub_code_tail(0x7a00 + i, true, 1, &tail);
                let vt = t.alloc(&[0; 12]);
                t.w32(vt + 4, count);
                t.w32(vt + 8, get);
                let src = t.zeros(0x10);
                t.w32(src, vt);
                srcs.push(src);
                t.w32(recs + 0x94 * i, src);
            }
            let this = t.junk(0x40, 0x32);
            t.w32(this + 4, n);
            t.w32(this + 8, recs);
            let (raw, data) = match case {
                2 => (4, t.junk(0x10, 0x33)),
                4 => (2 | sm::NOT_OWNED, t.junk(0x10, 0x34)),
                _ => (0, 0),
            };
            t.w32(this + 0x2c, raw);
            t.w32(this + 0x30, data);
            let what = format!("{label}: record order case {case}");
            t.check_frame(&what, 0x0053_cb90, Conv::Thiscall(this), &[], 0x38, |h| {
                let frame = h.1;
                Out::U32(ljb::order_records(h, this, frame))
            });
        }
        t.reset();
    }
}

// ---- batch 11: the component's network set-up ----

#[test]
fn network_setups_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lk::VISITOR_DTOR, true, 0, 0, 0xd7);
        t.stub(lk::RECORDS_SET_COUNT, true, 1, 1, 0x5c);
        t.stub(lk::TARGET_ARRAY, true, 1, 1, 0xda7a);
        t.stub(lk::NODE_PREPARE, true, 0, 0, 0x9e);
        t.stub(lk::MEMSET, false, 3, 0, 0x3e);
        // the root's visit (slot 3): the visitor's count word becomes 3
        let mut tail = vec![
            0x8b, 0x44, 0x24, 0x04, 0xc7, 0x40, 0x08, 3, 0, 0, 0, 0x31, 0xc0,
        ];
        tail.extend(ret_bytes(2));
        let visit = t.stub_code_tail(0x7c00, true, 2, &tail);
        let root = t.object_with(3, visit);
        for (case, errs) in [
            (0u32, [0u32; 5]),
            (1, [0x11, 0, 0, 0, 0]),
            (2, [0, 0x12, 0, 0, 0]),
            (3, [0, 0, 0x13, 0, 0]),
            (4, [0, 0, 0, 0x14, 0]),
            (5, [0, 0, 0, 0, 0x15]),
            (6, [0; 5]),
            (7, [0; 5]),
            (8, [0; 5]),
        ] {
            t.stub(lk::RECORDS_RESIZE, true, 1, 1, errs[0]);
            t.stub(lk::ORDER_RECORDS, true, 0, 0, errs[1]);
            t.stub(lk::DATA_RESIZE, true, 1, 1, errs[2]);
            t.stub(lk::TARGETS_RESIZE, true, 1, 1, errs[3]);
            if case == 6 {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x1000);
            }
            // three records, each with a node answering sizes in slots 5, 6
            let recs = t.junk(0x94 * 3, 0x41);
            for k in 0..3u32 {
                let s5 = t.stub_code(0x7d00 + k, true, 0, 0, 0x13 * (k + 1));
                let s6 = t.stub_code(0x7e00 + k, true, 0, 0, 0x7 + k);
                let vt = t.alloc(&[0; 0x20]);
                t.w32(vt + 0x14, s5);
                t.w32(vt + 0x18, s6);
                let node = t.zeros(0x10);
                t.w32(node, vt);
                let rec = recs + 0x94 * k;
                t.w32(rec, node);
                t.w32(rec + 4, k);
            }
            let set = t.junk(0x20, 0x42);
            t.w32(set + 8, 2);
            t.w32(set + 0x10, 3 | sm::NOT_OWNED);
            let this = t.junk(0x40, 0x43);
            t.w32(this + 4, 3);
            t.w32(this + 8, recs);
            t.w32(this + 0x14, if case == 7 { set } else { 0x51 });
            t.w32(this + 0x20, [0, 2, 2 | sm::NOT_OWNED][(case % 3) as usize]);
            let data = if case == 8 { 0 } else { t.junk(0x10, 0x44) };
            t.w32(this + 0x24, data);
            t.w32(this + 0x2c, 2);
            let network = t.zeros(0x10);
            t.w32(network + 8, set);
            t.w32(network + 0xc, if case == 8 { 0 } else { root });
            let b1 = t.object(&[0, 3], &[0, 0]);
            let b2 = t.object(&[0, 3], &[0, errs[4]]);
            let binders = t.alloc(&[0; 8]);
            t.w32(binders, b1);
            t.w32(binders + 4, b2);
            for (net, n) in [(network, 2u32), (0, 2)] {
                let what = format!("{label}: network case {case} net {net:#x}");
                t.check_frame(
                    &what,
                    0x0053_a2e0,
                    Conv::Thiscall(this),
                    &[net, binders, n],
                    0x3c,
                    |h| {
                        let frame = h.1;
                        Out::U32(lkb::set_network(h, this, net, binders, n, frame))
                    },
                );
            }
        }
        // a network without targets
        let network = t.zeros(0x10);
        let this = t.junk(0x40, 0x45);
        t.check_frame(
            &format!("{label}: network without targets"),
            0x0053_a2e0,
            Conv::Thiscall(this),
            &[network, 0, 0],
            0x3c,
            |h| {
                let frame = h.1;
                Out::U32(lkb::set_network(h, this, network, 0, 0, frame))
            },
        );
        t.reset();
    }
}

// ---- batch 12: the scene bind's targets ----

#[test]
fn scene_name_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(ll::BIND_TREE, false, 2, 0, 0xb7);
        for (case, fail) in [(0u32, false), (1, false), (2, true), (3, false)] {
            if fail {
                t.stub(la::PHYRE_MALLOC, false, 2, 0, 0);
            } else {
                t.stub_bump(la::PHYRE_MALLOC, 0x800);
            }
            let words: &[Option<&str>] = match case {
                0 => &[],
                1 => &[
                    Some("root"),
                    Some("#skip"),
                    None,
                    Some("arm#2"),
                    Some("leg"),
                    Some("root"),
                ],
                _ => &[Some("a"), Some("b#c")],
            };
            let mut v = Vec::new();
            for w in words {
                let p = match w {
                    Some(s) => t.cstr(s),
                    None => 0,
                };
                v.extend(p.to_le_bytes());
            }
            v.extend([0; 4]);
            let names = t.alloc(&v);
            let results = t.zeros(4 * words.len() + 4);
            let this = t.junk(0x30, 0x51);
            let hdr = this + 0xc;
            for off in [0u32, 4, 8, 0xc, 0x10, 0x14] {
                t.w32(hdr + off, hdr);
            }
            let n = if case == 3 { 0 } else { words.len() as u32 };
            let what = format!("{label}: scene names case {case}");
            t.check_frame(
                &what,
                0x0061_7350,
                Conv::Thiscall(this),
                &[names, results, n],
                0x2c,
                |h| {
                    let frame = h.1;
                    llb::bind_names(h, this, names, results, n, frame);
                    Out::None
                },
            );
        }
        t.reset();
    }
}

#[test]
fn scene_target_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ll::TREE_CLEAR, true, 0, 0, 0xc1);
        // the names binder: logs, and "finds" each name as its node
        // (results[i] = names[i])
        let tail = vec![
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08, 0x8b, 0x4c, 0x24,
            0x0c, // names, results, n
            0x85, 0xc9, 0x74, 0x09, 0x49, 0xff, 0x34, 0x88, 0x8f, 0x04, 0x8a, 0xeb,
            0xf3, // copy
            0xc2, 0x0c, 0x00,
        ];
        let code = t.stub_code_tail(ll::BIND_NAMES, true, 3, &tail);
        t.hook(ll::BIND_NAMES, code);
        let scene = t.rebase(ll::SCENE_NODE_CLASS);
        let sub = t.zeros(0x50);
        t.w32(sub + 0x40, scene);
        let other = t.zeros(0x50);
        let sub2 = t.zeros(0x50);
        // two steps to a dead end
        t.w32(sub2 + 0x40, other);
        for n in [0u32, 1, 5] {
            for (o, g) in [(1u32, 1u32), (0, 1), (1, 0)] {
                let mut tv = Vec::new();
                for (i, (class, ty)) in [(sub, 1u32), (scene, 1), (other, 1), (scene, 2), (sub2, 1)]
                    .into_iter()
                    .enumerate()
                {
                    let s = t.cstr(&format!("n{i}")) | 1;
                    for w in [class, 0, 0, 0, ty, s, 0] {
                        tv.extend(w.to_le_bytes());
                    }
                }
                let targets = if g == 0 { 0 } else { t.alloc(&tv) };
                let out = if o == 0 { 0 } else { t.zeros(0x18 * 5) };
                if out != 0 {
                    t.w32(out + 0x18, 0);
                    if n == 5 {
                        t.w32(out, 9);
                    }
                }
                let this = t.junk(0x10, 0x52);
                // the original's temporaries: ebp - 0x3c, then the two
                // 16-aligned allocas below its saved registers
                let top = t.b.stack_range().end - 0x100;
                let ebp = top - 20;
                let names = (ebp - 0x4c - 4 * n) & !0xf;
                let results = (names - 4 * n) & !0xf;
                let s = llb::Scratch {
                    bind: ebp - 0x3c,
                    names,
                    results,
                };

                let what = format!("{label}: scene targets n {n} out {o} targets {g}");
                t.check_frame(
                    &what,
                    0x005c_e1a0,
                    Conv::Thiscall(this),
                    &[out, targets, n],
                    top - results,
                    |h| Out::U32(llb::bind_targets(h, this, out, targets, n, s)),
                );
            }
        }
        t.reset();
    }
}

// ---- batch 13: data-member paths ----

#[test]
fn member_binds_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(lm::PRINTF, false, 6, 0, 0x99);
        t.stub(lm::STORED_SIZE, true, 0, 0, 0x14);
        t.stub_bump(la::PHYRE_MALLOC, 0x1000);
        // members by the segment's first letter: a, b scalar; c an array
        // of kind 4 (4 elements of 6 bytes at 0); d an array of kind 7
        let table = t.zeros(0x400);
        let mk_member = |t: &mut Twins, off: u32, array: u32| {
            let cls = t.zeros(0x90);
            t.w32(cls + 0x88, 0x10);
            let next_cls = t.zeros(0x10);
            let ty_get = t.stub_code(0x7f00, true, 0, 0, next_cls);
            let ty = t.object_with(1, ty_get);
            let arr_get = t.stub_code(0x7f10, true, 0, 0, array);
            let m = t.object_with(1, arr_get);
            t.w32(m + 0xc, cls);
            t.w32(m + 0x18, ty);
            t.w32(m + 0x24, off);
            m
        };
        let mk_array = |t: &mut Twins, kind: u32| {
            let el = t.zeros(0x20);
            t.w32(el + 0x1c, 6);
            let count = t.stub_code(0x7f20, true, 1, 1, 4);
            let elem = t.stub_code(0x7f21, true, 0, 0, el);
            let data = t.stub_code(0x7f22, true, 1, 1, 0);
            let vt = t.zeros(0x40);
            t.w32(vt + 0x10, count);
            t.w32(vt + 0x18, data);
            t.w32(vt + 0x24, elem);
            let a = t.zeros(0x40);
            t.w32(a, vt);
            t.w32(a + 0x30, kind);
            a
        };
        let ma = mk_member(&mut t, 0x18, 0);
        let mb = mk_member(&mut t, 0x30, 0);
        let arr = mk_array(&mut t, 4);
        let mc = mk_member(&mut t, 0x40, arr);
        let arr7 = mk_array(&mut t, 7);
        let md = mk_member(&mut t, 0x50, arr7);
        for (ch, m) in [(b'a', ma), (b'b', mb), (b'c', mc), (b'd', md)] {
            t.w32(table + 4 * u32::from(ch), m);
        }
        let mut tail = vec![0x8b, 0x44, 0x24, 0x04, 0x0f, 0xb6, 0x00, 0x8b, 0x04, 0x85];
        tail.extend(table.to_le_bytes()); // eax = table[first byte]
        tail.extend(ret_bytes(1));
        let code = t.stub_code_tail(lm::FIND_MEMBER, true, 1, &tail);
        t.hook(lm::FIND_MEMBER, code);
        // a class chain for the bind
        let want = t.zeros(0x50);
        let head = t.zeros(0x50);
        t.w32(head + 0x40, want);
        t.w32(head + 0x18, 0x0c1a);
        let paths = ["a", "a.b", "a.b.c", "c", "d", "a.x", "x", "", "a.", "b.a.c"];
        for obj8 in [0u32, 0x0b80] {
            for (idx, nhead) in [(1u32, head), (4, head), (0, head), (2, 0)] {
                let mut tv = Vec::new();
                for (i, p) in paths.iter().enumerate() {
                    let s = t.cstr(p) | (i as u32 & 1);
                    let class = if i == 6 { 0x1234 } else { want };
                    let ty = if i == 0 { 3 } else { 2 };
                    for w in [class, 0x0b40 + i as u32, 0, 0, ty, s, idx] {
                        tv.extend(w.to_le_bytes());
                    }
                }
                let targets = t.alloc(&tv);
                let n = paths.len() as u32;
                let out = t.zeros(0x18 * n as usize);
                t.w32(out + 0x18 * 2, 0);
                t.w32(out + 0x18 * 9, 5);
                let this = t.junk(0x10, 0x61);
                t.w32(this + 4, nhead);
                t.w32(this + 8, obj8);
                for (o, g, nn) in [
                    (out, targets, n),
                    (0, targets, n),
                    (out, 0, n),
                    (out, targets, 0),
                ] {
                    let what = format!(
                        "{label}: members obj {obj8:#x} idx {idx} head {nhead:#x} {o:#x} {g:#x} {nn}"
                    );
                    t.check_frame(
                        &what,
                        0x005c_dba0,
                        Conv::Thiscall(this),
                        &[o, g, nn],
                        0x28,
                        |h| {
                            let frame = h.1;
                            Out::U32(lmb::bind_members(h, this, o, g, nn, frame))
                        },
                    );
                }
            }
        }
        t.reset();
    }
}

// ---- tail9: the component's binder lists (ffx_panim::l9_anim) ----

use ffx_panim::l9_anim::bridge as l9a;

impl Rng {
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
}

/// A `PhyreMalloc` stub (cdecl `(size, align)`, logged) handing out
/// 16-byte aligned junk blocks, and 0 from the `fail_at`-th call on (0:
/// never).
fn bump_failing(t: &mut Twins, sym: u32, bytes: usize, fail_at: u32) {
    let area = t.junk(bytes + 16, 0x6c);
    let cur = t.alloc(&((area + 15) & !15).to_le_bytes());
    let left = t.alloc(&(if fail_at == 0 { u32::MAX } else { fail_at }).to_le_bytes());
    let mut tail = vec![0xff, 0x0d];
    tail.extend(left.to_le_bytes()); // dec dword [left]
    tail.extend([0x75, 0x03, 0x31, 0xc0, 0xc3]); // jnz +3; xor eax, eax; ret
    tail.push(0xa1);
    tail.extend(cur.to_le_bytes()); // mov eax, [cur]
    tail.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    tail.extend([0x83, 0xc2, 0x0f, 0x83, 0xe2, 0xf0]); // add edx, 15; and edx, -16
    tail.extend([0x01, 0xc2]); // add edx, eax
    tail.extend([0x89, 0x15]);
    tail.extend(cur.to_le_bytes()); // mov [cur], edx
    tail.push(0xc3);
    let code = t.stub_code_tail(sym, false, 2, &tail);
    t.hook(sym, code);
}

/// The frame slots the lists and temporaries use (below `ebp`).
const L9_SLOTS: [u32; 40] = [
    0x84, 0x80, 0x4c, 0x74, 0x70, 0x28, 0x7c, 0x78, 0x44, 0x60, 0x5c, 0x18, 0x8c, 0x88, 0x68, 0x38,
    0x34, 0x30, 0x58, 0x54, 0x40, 0x90, 0xac, 0xa8, 0xa4, 0xa0, 0xc0, 0xbc, 0xb8, 0xcc, 0xc8, 0xc4,
    0xd8, 0xd4, 0xd0, 0xb4, 0xb0, 0x9c, 0x98, 0x94,
];

#[test]
#[allow(clippy::too_many_lines)] // the set, the pairs and their classes
fn l9_bind_targets_matches_the_original() {
    // records of every kind (1 and 5 twice: once only), kind 1/2 classes
    // and others, kind 4 classes repeated (W already has them); pairs
    // deriving from the descriptors directly or through a parent, objects
    // 0, 4 and real (+0xc set or not), classes 0; the storage running out
    // at each allocation in turn; setNetwork's result
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x0053_6230);
        let (k1, k2, k5) = (
            t.rebase(l9a::CLASS_K1),
            t.rebase(l9a::CLASS_K2),
            t.rebase(l9a::CLASS_K5),
        );
        for case in 0..800_u32 {
            // classes: the three descriptors, two of their children, two
            // unrelated ones (a chain ending in 0)
            let other = t.zeros(0x44);
            let other2 = t.zeros(0x44);
            t.w32(other2 + 0x40, other);
            let mut classes = vec![k1, k2, k5, other, other2];
            for parent in [k1, k2, k5] {
                let c = t.zeros(0x44);
                t.w32(c + 0x40, parent);
                classes.push(c);
            }
            let n_pairs = rng.below(10);
            let mut pairs = Vec::new();
            for _ in 0..n_pairs {
                let object = match rng.below(5) {
                    0 => 0,
                    1 => 4,
                    _ => {
                        let o = t.junk(0x20, 0x31);
                        let v = if rng.chance(3) { 0 } else { 0x77 };
                        t.w32(o + 0xc, v);
                        o
                    }
                };
                let mut class = if rng.chance(8) {
                    0
                } else {
                    classes[rng.below(classes.len() as u32) as usize]
                };
                // the object 4 (the kind 1 skip) only with kind 1's class:
                // the other kinds read an object's +0xc
                if object == 4 {
                    class = k1;
                }
                pairs.push(object);
                pairs.push(class);
            }
            let pair_words = t.alloc(
                &pairs
                    .iter()
                    .chain([0_u32, 0].iter())
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            let list = t.alloc(&[0; 8]);
            t.w32(list, n_pairs | if rng.chance(4) { 0x8000_0000 } else { 0 });
            t.w32(list + 4, pair_words);
            let n_recs = rng.below(12);
            let recs = t.zeros(0x1c * 12 + 4);
            for i in 0..n_recs {
                let kind = rng.below(7);
                let class = match kind {
                    1 => {
                        if rng.chance(4) {
                            other
                        } else {
                            k1
                        }
                    }
                    2 => {
                        if rng.chance(2) {
                            k2
                        } else {
                            classes[rng.below(classes.len() as u32) as usize]
                        }
                    }
                    _ => classes[rng.below(classes.len() as u32) as usize],
                };
                t.w32(recs + 0x1c * i, class);
                t.w32(recs + 0x1c * i + 0x10, kind);
            }
            let set = t.zeros(0x10);
            t.w32(
                set + 8,
                n_recs | if rng.chance(5) { 0x8000_0000 } else { 0 },
            );
            t.w32(set + 0xc, recs);
            // now and then 0x00ca91f8 derives from 0x00ca7228 (its parent
            // word set for this case): a kind 1 pair then matches both, and
            // the second match must not run for an object kind 1 took
            let k1_parent: u32 = Mem::read(&t.a, k1 + 0x40);
            // (not with the object 4: kind 2 reads an object's +0xc)
            if case % 4 == 1 && !pairs.contains(&4) {
                t.w32(k1 + 0x40, k2);
            }
            let fail_at = if rng.chance(2) { 1 + rng.below(12) } else { 0 };
            bump_failing(&mut t, l9a::MALLOC, 0x4000, fail_at);
            t.stub(l9a::FREE, false, 1, 0, 0);
            let net_ret = if rng.chance(4) { 0x13 } else { 0 };
            t.stub(l9a::SET_NETWORK, true, 3, 3, net_ret);
            let (this, network) = (t.junk(0x40, 0x22), rng.next());
            // the original's ebp: four argument words, the return address
            // and ebp pushed below the call top
            let top = t.a.stack_range().end - 0x100;
            let ebp = top - 0x18;
            let total_cell = std::cell::Cell::new(None);
            let what = format!("{label}: bindTargets #{case}");
            t.check_frame(
                &what,
                0x0053_6230,
                Conv::Cdecl,
                &[this, network, set, list],
                0,
                |h| {
                    Out::U32(match l9a::collect(h, ebp, set, list) {
                        Err(code) => code,
                        Ok(total) => {
                            // the alloca below the original's esp (ebp - 0xe8)
                            let buf = (ebp - 0xe8).wrapping_sub(total.wrapping_mul(4)) & !0xf;
                            total_cell.set(Some((buf, total)));
                            l9a::finish(h, ebp, buf, this, network, total)
                        }
                    })
                },
            );
            for o in L9_SLOTS {
                let (x, y): (u32, u32) = (Mem::read(&t.a, ebp - o), Mem::read(&t.b, ebp - o));
                assert_eq!(x, y, "{what}: frame word ebp - {o:#x}");
            }
            if let Some((buf, total)) = total_cell.get() {
                for i in 0..total {
                    let (x, y): (u32, u32) =
                        (Mem::read(&t.a, buf + 4 * i), Mem::read(&t.b, buf + 4 * i));
                    assert_eq!(x, y, "{what}: binder {i}");
                }
            }
            t.w32(k1 + 0x40, k1_parent);
            t.reset();
        }
    }
}
