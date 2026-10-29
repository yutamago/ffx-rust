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

const TRAMPOLINE_CODE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
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
