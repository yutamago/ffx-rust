//! `PhyreEngine`'s `PAnimation` code, list `phyre-anim`
//! (`ffx_panim::m5p_anim*`, M5 stage 1), against the original. Almost none
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
    clippy::too_many_lines,
    clippy::many_single_char_names,
    dead_code
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_panim::array;
use ffx_panim::m5p_anim_small::{self as sm, bridge as smb};

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
}

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
        };
        t.log = t.zeros(0x10000);
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
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
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
        let out = port(&mut G(&mut self.b, top - below));
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
        let out = port(&mut G(&mut self.b, scratch));
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

// ---- batch 1: template copies and one-liners ----

/// A `PArray` count with the "not owned" bit as asked.
fn count(n: u32, owned: bool) -> u32 {
    if owned { n } else { n | sm::NOT_OWNED }
}

#[test]
fn tail_dtors_match_the_original() {
    assert_eq!(sm::TAIL_DTORS.len(), 46);
    for (label, mut t) in Twins::all() {
        t.stub(sm::DESTRUCT, true, 0, 0, 0xd357);
        for &(f, _) in sm::TAIL_DTORS {
            let obj = t.junk(0x40, f as u8);
            let what = format!("{label}: tail dtor {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::tail_dtor(g, sm::word_of(sm::TAIL_DTORS, f), obj))
            });
        }
        assert_eq!(t.log_entries(), 46);
        t.reset();
    }
}

#[test]
fn lazy_swaps_match_the_original() {
    assert_eq!(sm::SWAP_LAZY.len(), 7);
    for (label, mut t) in Twins::all() {
        for &(_, l) in sm::SWAP_LAZY {
            t.stub(l.ctor, true, 0, 0, 0xc7);
        }
        t.stub(0x0094_9207, false, 1, 0, 0xa7);
        t.stub(0x0043_d6a0, true, 2, 2, 0x5a9);
        for &(f, l) in sm::SWAP_LAZY {
            for guard in [0u32, 1, 0xffff_fffe, 0x8000_0003] {
                let g = t.rebase(l.guard);
                t.w32(g, guard);
                let fl = t.rebase(l.flags);
                t.w32(fl, 0xffff_ffff ^ guard);
                let p = t.junk(0x20, 3);
                let what = format!("{label}: lazy swap {f:#x} guard {guard:#x}");
                t.check(&what, f, Conv::Cdecl, &[p], |g| {
                    Out::U32(smb::swap_lazy(g, &sm::lazy_of(f), p))
                });
            }
        }
        t.reset();
    }
}

#[test]
fn vtable_dtors_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (f, vt) in [
            (0x005c_db70, sm::TARGET_BIND_VTABLE),
            (0x0052_1230, sm::VISIT_CONTEXT_VTABLE),
            (0x0053_4af0, sm::VISIT_CONTEXT_VTABLE),
            (0x0053_4b00, sm::VISIT_CONTEXT_VTABLE),
        ] {
            let obj = t.junk(0x20, 9);
            let what = format!("{label}: vtable dtor {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                smb::set_vtable(g, obj, vt);
                Out::None
            });
        }
    }
}

#[test]
fn deleting_dtors_match_the_original() {
    assert_eq!(sm::DELETES.len(), 13);
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(sm::OPERATOR_DELETE, false, 1, 0, 0xde1);
        t.stub(sm::FREELIST_DELETE, false, 1, 0, 0xf1);
        t.stub(sm::THREAD_POOL_JOB_DTOR, true, 0, 0, 0x70b);
        for &(f, d) in sm::DELETES {
            for flags in [0, 1, 2, 3, 0xffff_fffe, 0xffff_ffff] {
                let obj = t.junk(0x30, flags as u8);
                let what = format!("{label}: deleting dtor {f:#x}({flags:#x})");
                t.check(&what, f, Conv::Thiscall(obj), &[flags], |g| {
                    Out::U32(smb::delete(g, d, obj, flags))
                });
            }
        }
        t.reset();
    }
}

/// An array of `n` objects of `stride` bytes, each with a vtable whose
/// slot 0 is a recording stub taking (and popping) one word.
fn objects(t: &mut Twins, n: u32, stride: u32) -> u32 {
    let els = t.junk((n.max(1) * stride) as usize, 0x31);
    for i in 0..n {
        let s = t.stub_code(0x7100 + i, true, 1, 1, 0x900 + i);
        let vt = t.alloc(&s.to_le_bytes());
        t.w32(els + i * stride, vt);
    }
    els
}

#[test]
fn array_releases_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        for &(_, helper) in sm::RELEASES_WITH {
            t.stub(helper, false, 2, 0, 0x4e1);
        }
        for owned in [true, false] {
            for n in [0, 1, 3, 0x7fff_ffff] {
                for null in [false, true] {
                    let what = format!("{label}: owned {owned} n {n:#x} null {null}");
                    let els = if null { 0 } else { t.junk(0x10, 2) };
                    for &f in sm::RELEASES {
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(n, owned));
                        t.w32(arr + 4, els);
                        t.check(
                            &format!("{what}: release {f:#x}"),
                            f,
                            Conv::Thiscall(arr),
                            &[],
                            |g| Out::U32(smb::release(g, arr)),
                        );
                    }
                    for &(f, helper) in sm::RELEASES_WITH {
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(n, owned));
                        t.w32(arr + 4, els);
                        t.check(
                            &format!("{what}: release with {f:#x}"),
                            f,
                            Conv::Thiscall(arr),
                            &[],
                            |g| Out::U32(smb::release_with(g, helper, arr)),
                        );
                    }
                    if n > 3 {
                        continue;
                    }
                    for &(f, stride) in sm::RELEASES_VIRTUAL {
                        let els = if null { 0 } else { objects(&mut t, n, stride) };
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(n, owned));
                        t.w32(arr + 4, els);
                        t.check(
                            &format!("{what}: release virtual {f:#x}"),
                            f,
                            Conv::Thiscall(arr),
                            &[],
                            |g| Out::U32(smb::release_virtual(g, stride, arr)),
                        );
                    }
                }
            }
        }
        t.reset();
    }
}

#[test]
fn set_borrowed_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        let same = t.junk(0x10, 4);
        let other = t.junk(8, 1);
        for &f in sm::SET_BORROWED {
            for owned in [true, false] {
                for (prev, els) in [(same, same), (0, same), (same, 0), (other, same)] {
                    for n in [0, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(5, owned));
                        t.w32(arr + 4, prev);
                        let what = format!(
                            "{label}: setBorrowed {f:#x} owned {owned} {prev:#x} {els:#x} {n:#x}"
                        );
                        t.check(&what, f, Conv::Thiscall(arr), &[els, n], |g| {
                            Out::U32(array::bridge::set_borrowed(
                                g,
                                arr,
                                els,
                                n,
                                array::Old::Free,
                            ))
                        });
                    }
                }
            }
        }
        t.reset();
    }
}

#[test]
fn constants_match_the_original() {
    assert_eq!(sm::CONSTANTS.len(), 11);
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x20, 5);
        for &(f, v, a) in sm::CONSTANTS {
            let what = format!("{label}: constant {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::constant(g, (v, a)))
            });
        }
        for f in [0x0052_1730, 0x0052_1740, 0x005c_db60] {
            let what = format!("{label}: al 1 {f:#x}");
            t.check(&what, f, Conv::Thiscall(obj), &[3, 4], |_| Out::Al(1));
        }
        t.check(
            &format!("{label}: 0x531650"),
            0x0053_1650,
            Conv::Thiscall(obj),
            &[1, 2, 3],
            |_| Out::U32(0),
        );
        let what = format!("{label}: PTimeController time");
        for bits in [0x3f80_0000u32, 0xc2f6_e979, 0x7fc0_0001, 0x0000_0001] {
            let at = t.rebase(sm::TIME_CONSTANT);
            t.w32(at, bits);
            t.check(&what, 0x0052_2110, Conv::Thiscall(obj), &[], |g| {
                Out::F32(smb::time_constant(g))
            });
        }
    }
}

#[test]
fn getters_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_ace1);
        for round in 0..24 {
            let obj = t.junk(0x200, round as u8);
            let els = t.junk(0x100, 0x77);
            let src = t.junk(0x20, 0x13);
            // counts around the top bit, small and huge
            let c = [0, 1, 7, 0x8000_0003, 0xffff_ffff, rng.next()][round % 6];
            t.w32(obj + 0xc, c);
            t.w32(obj + 0x14, c ^ 0x8000_0000);
            t.w32(obj + 0x100, c);
            t.w32(obj + 0x118, [u32::MAX, 0, rng.next()][round % 3]);
            t.w32(obj + 0x10, els);
            t.w32(obj + 8, if round % 4 == 3 { 0 } else { src });
            t.w32(src + 0x10, c);
            t.w32(src + 8, els);
            t.w32(obj + 0x1c, if round % 2 == 0 { 0 } else { src });
            let i = [0, 1, 5, 0x8000_0001, 0x3f][round % 5];
            let what = format!("{label}: round {round}");
            let w = |s: &str| format!("{what}: {s}");
            t.check(&w("0x52cc80"), 0x0052_cc80, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::count_at(g, obj, 0xc))
            });
            t.check(&w("0x616a20"), 0x0061_6a20, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::count_at(g, obj, 0x100))
            });
            t.check(
                &w("0x52a500"),
                0x0052_a500,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(smb::word_at(g, obj, 0x14)),
            );
            t.check(&w("0x616aa0"), 0x0061_6aa0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::word_at(g, obj, 0x118))
            });
            t.check(&w("0x616d50"), 0x0061_6d50, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::has_word_118(g, obj))
            });
            t.check(
                &w("0x52cc50"),
                0x0052_cc50,
                Conv::Thiscall(obj),
                &[i & 7],
                |g| Out::U32(smb::source_at(g, obj, i & 7)),
            );
            if t.r32(obj + 8) != 0 {
                for f in [0x0052_a4d0, 0x0053_0690] {
                    t.check(&w("size"), f, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(smb::size(g, sm::size_of(f), obj))
                    });
                }
                t.check(
                    &w("0x5111c0"),
                    0x0051_11c0,
                    Conv::Thiscall(src),
                    &[i & 3],
                    |g| Out::U32(smb::element_8(g, src, i & 3)),
                );
                t.check(
                    &w("0x432f50"),
                    0x0043_2f50,
                    Conv::Thiscall(src),
                    &[i & 3],
                    |g| Out::U32(smb::element_8(g, src, i & 3)),
                );
            }
            t.check(&w("0x52e260"), 0x0052_e260, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::size(g, sm::size_of(0x0052_e260), obj))
            });
            t.check(
                &w("0x432f70"),
                0x0043_2f70,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(smb::element_34(g, obj, i)),
            );
            t.check(
                &w("0x5306c0"),
                0x0053_06c0,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(smb::first_source(g, obj, i)),
            );
            t.check(&w("0x50e730"), 0x0050_e730, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::batch_sort_key(g, obj))
            });
            let out = t.junk(8, 1);
            t.check(
                &w("0x45de80"),
                0x0045_de80,
                Conv::Thiscall(obj),
                &[out],
                |g| Out::U32(smb::copy_pair(g, obj, out)),
            );
            t.check(&w("0x526250"), 0x0052_6250, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::source_bytes(g, obj))
            });
            let v = rng.next();
            t.check(
                &w("0x616eb0"),
                0x0061_6eb0,
                Conv::Thiscall(obj),
                &[v],
                |g| Out::U32(smb::set_word_118(g, obj, v)),
            );
            t.check(
                &w("0x5265f0"),
                0x0052_65f0,
                Conv::Thiscall(obj),
                &[v],
                |g| Out::U32(smb::set_time_controller(g, obj, v)),
            );
            for (f, off) in [(0x005c_dab0, 8), (0x005c_dac0, 4)] {
                t.check(&w("count visit"), f, Conv::Thiscall(obj), &[1, 2], |g| {
                    Out::Al(smb::count_visit(g, obj, off))
                });
            }
            let b = t.junk(0x20, 0x44);
            for k in [c, c.wrapping_add(1), c.wrapping_sub(1)] {
                t.w32(b + 0x14, k);
                t.w32(obj + 0x14, c);
                t.check(&w("compare"), 0x0051_0f60, Conv::Cdecl, &[obj, b], |g| {
                    Out::U32(smb::compare_modules(g, obj, b))
                });
            }
        }
    }
}

#[test]
fn forwarders_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::STRING_ASSIGN, true, 1, 1, 0x5a);
        t.stub(sm::TREE_CLEAR, true, 0, 0, 0x7c);
        t.stub(sm::PUSH_OBJECT, false, 3, 0, 0x9b);
        let obj = t.junk(0x80, 1);
        for (f, off) in [(0x0050_74c0, 0x50), (0x0051_7220, 0x1c)] {
            t.check(
                &format!("{label}: assign {f:#x}"),
                f,
                Conv::Thiscall(obj),
                &[0x1234],
                |g| Out::U32(smb::assign_string(g, obj, off, 0x1234)),
            );
        }
        t.check(
            &format!("{label}: clear"),
            0x005c_e190,
            Conv::Thiscall(obj),
            &[],
            |g| Out::U32(smb::clear_tree(g, obj)),
        );
        for event in [0, obj] {
            t.check(
                &format!("{label}: push {event:#x}"),
                0x0052_4bb0,
                Conv::Cdecl,
                &[7, event, 9],
                |g| Out::U32(smb::push_event(g, 7, event, 9)),
            );
        }
        assert_eq!(t.log_entries(), 5);
        t.reset();
        // the async headers: none, all zero, the first non-zero
        for ret in [0, 0x33] {
            t.stub(sm::ASYNC_PROCESS, true, 0, 0, ret);
            for n in [0, 1, 3] {
                let buf = t.junk(0x10, 2);
                let mut next = 0;
                for _ in 0..n {
                    let p = t.junk(0x10, 3);
                    t.w32(p + 8, next);
                    next = p;
                }
                t.w32(buf + 4, next);
                let what = format!("{label}: process {n} {ret:#x}");
                t.check(&what, 0x005c_ea70, Conv::Thiscall(buf), &[], |g| {
                    Out::U32(smb::process_all(g, buf))
                });
            }
            t.reset();
        }
    }
}

// ---- batch 2: visits, accessors, array helpers ----

use ffx_panim::m5p_anim_misc::{self as mi, bridge as mb};

impl Twins {
    /// A recording stub in fresh code memory (as [`Twins::stub_code`])
    /// that leaves the `float` with `bits` in `ST0`.
    fn stub_float_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, bits: u32) -> u32 {
        let cell = self.alloc(&bits.to_le_bytes());
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
        c.extend([0xd9, 0x05]);
        c.extend(cell.to_le_bytes()); // fld dword [cell]
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

    /// An object (0x40 bytes) whose vtable slot `slot` is `code` (the
    /// other slots null).
    fn object_with(&mut self, slot: u32, code: u32) -> u32 {
        let mut vt = vec![0u8; 4 * (slot as usize + 1)];
        vt[4 * slot as usize..].copy_from_slice(&code.to_le_bytes());
        let vt = self.alloc(&vt);
        let obj = self.zeros(0x40);
        self.w32(obj, vt);
        obj
    }
}

/// Floats for the x87 paths: signs, zero, fractions, huge, tiny,
/// infinities and quiet NaNs (no signalling NaNs: the emulator keeps
/// them signalling).
const FLOATS: [u32; 12] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000,
    0xbfc0_0000,
    0x3eaa_aaab,
    0x7f7f_ffff,
    0x0000_0001,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc1_2345,
    0x4b80_0001,
];

#[test]
fn visits_match_the_original() {
    assert_eq!(mi::VISITS.len(), 6);
    for (label, mut t) in Twins::all() {
        for &(f, v) in mi::VISITS {
            for begin in [0x100, 0x1ff, 0x0f00] {
                for child in [None, Some(0x2100), Some(0x2201)] {
                    let visitor = t.object(&[0, 2, 2, 2, 2], &[0, begin, begin, 0x3301, 0x3355]);
                    let this = t.junk(0x40, 1);
                    if let Some(off) = v.child {
                        let c = match child {
                            None => 0,
                            Some(r) => t.object(&[0, 0, 0, 2], &[0, 0, 0, r]),
                        };
                        t.w32(this + off, c);
                    }
                    let what = format!("{label}: visit {f:#x} begin {begin:#x} child {child:?}");
                    t.check(&what, f, Conv::Thiscall(this), &[visitor, 0x77], |g| {
                        Out::U32(mb::visit(g, v, this, visitor, 0x77))
                    });
                }
            }
        }
        t.reset();
    }
}

#[test]
fn times_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (k, &scale) in FLOATS.iter().enumerate() {
            let offset = FLOATS[(k * 5 + 3) % FLOATS.len()];
            let tbits = FLOATS[(k * 7 + 1) % FLOATS.len()];
            for with_src in [false, true] {
                let this = t.junk(0x20, 2);
                t.w32(this + 0xc, scale);
                t.w32(this + 0x10, offset);
                let src = if with_src {
                    let s = t.stub_float_code(0x7104, true, 0, 0, tbits);
                    t.object_with(4, s)
                } else {
                    let at = t.rebase(sm::TIME_CONSTANT);
                    t.w32(at, tbits);
                    0
                };
                t.w32(this + 8, src);
                let what =
                    format!("{label}: scaled {scale:#x} {offset:#x} {tbits:#x} src {with_src}");
                t.check(&what, 0x0052_3890, Conv::Thiscall(this), &[], |g| {
                    Out::F32(mb::scaled_time(g, this))
                });
            }
        }
    }
}

/// A component's 0x4c-byte source records (`[this + 0x18]`, count at
/// `+0x14`): record `i`'s first word set for odd `i`, its object at
/// `+0xc` answering slot 4 with a float.
fn records(t: &mut Twins, this: u32, n: u32, count: u32) {
    let recs = t.junk((n.max(1) * 0x4c) as usize, 0x29);
    for i in 0..n {
        let rec = recs + i * 0x4c;
        t.w32(rec, i & 1);
        let s = t.stub_float_code(
            0x7200 + i,
            true,
            0,
            0,
            FLOATS[i as usize % FLOATS.len()] ^ 0x10,
        );
        // the object at +0xc: its vtable's slot 4 answers
        let mut vt = vec![0u8; 0x10];
        vt.extend(s.to_le_bytes());
        let vt = t.alloc(&vt);
        t.w32(rec + 0xc, vt);
        t.w32(rec + 4, FLOATS[(i as usize + 3) % FLOATS.len()]);
        t.w32(rec + 8, FLOATS[(i as usize + 6) % FLOATS.len()]);
    }
    t.w32(this + 0x18, recs);
    t.w32(this + 0x14, count);
}

#[test]
fn component_accessors_match_the_original() {
    for (label, mut t) in Twins::all() {
        for (n, count) in [(0, 0), (1, 1), (4, 4), (4, 0x8000_0004), (3, 2)] {
            let this = t.junk(0x60, 3);
            records(&mut t, this, n, count);
            let w = t.junk(0x40, 9);
            t.w32(this + 0x1c + 0x18, w);
            t.w32(this + 0x1c + 0x14, [3, 0, 0x8000_0002][n as usize % 3]);
            let what = format!("{label}: {n} records, count {count:#x}");
            t.check(
                &format!("{what}: first"),
                0x0054_2560,
                Conv::Thiscall(this),
                &[],
                |g| Out::F32(mb::first_value(g, this)),
            );
            for i in [0, 1, 2, 3, 4, 0x8000_0001] {
                let w = |s: &str| format!("{what}: {s} {i:#x}");
                t.check(&w("value"), 0x0054_25c0, Conv::Thiscall(this), &[i], |g| {
                    Out::F32(mb::value_at(g, this, i))
                });
                for (f, off) in [(0x0054_2e90, 8), (0x0054_2fb0, 4)] {
                    t.check(&w("float"), f, Conv::Thiscall(this), &[i], |g| {
                        Out::F32(mb::float_at(g, this, i, off))
                    });
                }
                t.check(&w("word"), 0x0054_2500, Conv::Thiscall(this), &[i], |g| {
                    Out::U32(mb::word_at(g, this, i))
                });
                for &bits in &FLOATS[..6] {
                    t.check(
                        &w("set weight"),
                        0x0054_42e0,
                        Conv::Thiscall(this),
                        &[i, bits],
                        |g| Out::U32(mb::set_source_weight(g, this, i, bits)),
                    );
                    let b = this + 0x1c;
                    t.check(
                        &w("weight"),
                        0x0052_e660,
                        Conv::Thiscall(b),
                        &[i, bits ^ 0x8000_0000],
                        |g| Out::U32(mb::set_weight(g, b, i, bits ^ 0x8000_0000)),
                    );
                }
            }
        }
        t.reset();
    }
}

#[test]
fn small_methods_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        // channel times' range
        for n in [0, 1, 2, 5] {
            let this = t.junk(0x10, 1);
            let els = t.junk(0x20, 2);
            for i in 0..5 {
                t.w32(
                    els + 4 * i,
                    FLOATS[(i as usize + n as usize) % FLOATS.len()],
                );
            }
            t.w32(this, n);
            t.w32(this + 8, els);
            let out = t.zeros(8);
            t.check(
                &format!("{label}: range {n}"),
                0x0050_acf0,
                Conv::Thiscall(this),
                &[out, out + 4],
                |g| Out::U32(mb::first_last(g, this, out, out + 4)),
            );
        }
        // the owner walk
        let owner = t.rebase(mi::GAMEPLAY_OWNER);
        for (with, head) in [(true, 0), (true, 1), (false, 1), (true, 2)] {
            let this = t.junk(0x10, 2);
            let n0 = t.junk(0x50, 4);
            let n1 = t.junk(0x50, 5);
            t.w32(n1 + 0x40, n0);
            t.w32(n0 + 0x40, if with { owner } else { 0 });
            t.w32(this + 4, if head == 0 { 0 } else { n1 });
            t.w32(this, if head == 2 { 0 } else { 0x1234 });
            t.check(
                &format!("{label}: owner {with} {head}"),
                0x0053_37d0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(mb::owner_part(g, this)),
            );
        }
        // marks
        for n in [0, 1, 4] {
            let flags = t.junk(0x20, 5);
            let idx = t.alloc(
                &[3u32, 7, 0x1f, 0]
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            t.check(
                &format!("{label}: mark {n}"),
                0x0052_41d0,
                Conv::Thiscall(0x99),
                &[flags, idx, n],
                |g| Out::U32(mb::mark(g, flags, idx, n)),
            );
        }
        // arrays of pairs and records
        for round in 0..12u32 {
            let this = t.junk(0x200, round as u8);
            let n = [0, 1, 3, 0x8000_0002][round as usize % 4];
            let recs = t.junk(0x60, 6);
            let pairs = t.junk(0x40, 7);
            for k in 0..4 {
                let obj = t.junk(0x120, 8 + k as u8);
                t.w32(obj + 0x10c, recs);
                t.w32(
                    obj + 0x118,
                    if (round >> k) & 1 == 1 { u32::MAX } else { k },
                );
                t.w32(pairs + 8 * k, obj);
                t.w32(pairs + 8 * k + 4, k & 1);
            }
            t.w32(this + 0x100, n);
            t.w32(this + 0x104, pairs);
            t.w32(this + 0x108, n);
            t.w32(this + 0x10c, recs);
            t.w32(this + 8, n);
            t.w32(this + 0xc, recs);
            let what = format!("{label}: round {round}");
            t.check(
                &format!("{what}: all bound"),
                0x0061_6d10,
                Conv::Thiscall(this),
                &[],
                |g| Out::Al(mb::all_bound(g, this)),
            );
            for i in [0, 1, 2, 0x8000_0000] {
                t.check(
                    &format!("{what}: record {i:#x}"),
                    0x0061_6a40,
                    Conv::Thiscall(this),
                    &[i],
                    |g| Out::U32(mb::record_word(g, this, i)),
                );
                t.check(
                    &format!("{what}: pair {i:#x}"),
                    0x0061_69e0,
                    Conv::Thiscall(this),
                    &[i],
                    |g| Out::U32(mb::pair_record_word(g, this, i)),
                );
                t.check(
                    &format!("{what}: element {i:#x}"),
                    0x0053_afd0,
                    Conv::Thiscall(this),
                    &[i],
                    |g| Out::U32(mb::element_28(g, this, i)),
                );
            }
            // clip bytes
            let clip = t.junk(0x20, 9);
            let hdr = t.junk(4, round as u8);
            t.w32(clip, hdr);
            t.w32(clip + 4, rng.next());
            t.w32(clip + 0xc, rng.next());
            t.w32(this + 0xc, if round % 3 == 0 { 0 } else { clip });
            t.check(
                &format!("{what}: clip bytes"),
                0x0052_6290,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(mb::clip_bytes(g, this)),
            );
            // the ring buffer
            let ringbuf = t.junk(0x30, 1);
            let (s, e, w, r) = (
                0x100,
                0x200,
                [0x150, 0x1f0, 0x120][round as usize % 3],
                [0x180, 0x100, 0x120, 0x140][round as usize % 4],
            );
            t.w32(ringbuf + 0x18, s);
            t.w32(ringbuf + 0x1c, e);
            t.w32(ringbuf + 0x24, w);
            t.w32(ringbuf + 0x28, r);
            for n in [0, 0x10, 0x30, 0x60, 0x100] {
                t.check(
                    &format!("{what}: ringbuf {n:#x}"),
                    0x005c_e9d0,
                    Conv::Thiscall(ringbuf),
                    &[n],
                    |g| Out::U32(mb::would_overrun(g, ringbuf, n)),
                );
            }
        }
        // sources sharing the set
        for (set, other, n, i) in [
            (0, 5, 2, 0),
            (5, 6, 2, 0),
            (5, 5, 2, 1),
            (5, 5, 2, 2),
            (5, 5, 0x8000_0003, 2),
        ] {
            let this = t.junk(0x20, 3);
            let els = t.junk(0x10, 4);
            let src = t.junk(0x10, 5);
            t.w32(this + 8, set);
            t.w32(this + 0xc, n);
            t.w32(this + 0x10, els);
            t.w32(src + 8, other);
            t.check(
                &format!("{label}: set source {set} {other}"),
                0x0053_1a10,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(mb::set_source(g, this, src)),
            );
            t.w32(this + 0xc, n);
            t.check(
                &format!("{label}: set source at {set} {other} {i}"),
                0x0052_cec0,
                Conv::Thiscall(this),
                &[i, src],
                |g| Out::U32(mb::set_source_at(g, this, i, src)),
            );
        }
        // the sharray
        for n in [0, 1, 2, 3, 0x8000_0001] {
            let this = t.junk(0x10, 6);
            let els = t.junk(0x10, 7);
            t.w32(this, n);
            t.w32(this + 4, els);
            for i in [0, 1, 2, 5] {
                t.check(
                    &format!("{label}: sharray {n:#x} {i}"),
                    0x0051_cc50,
                    Conv::Thiscall(this),
                    &[i],
                    |g| Out::U32(mb::sharray_get(g, this, i)),
                );
            }
        }
        // records of 0x11c bytes
        for n in [0u32, 1, 3] {
            let recs = t.junk(0x11c * 3, 8);
            for k in 0..3 {
                t.w32(recs + 0x11c * k + 0x118, 10 + k);
            }
            let this = t.alloc(&[recs.to_le_bytes(), n.to_le_bytes()].concat());
            for key in [10, 11, 12, 13] {
                t.check(
                    &format!("{label}: find {n} {key}"),
                    0x005c_d750,
                    Conv::Thiscall(this),
                    &[key],
                    |g| Out::U32(mb::find_record(g, this, key)),
                );
            }
        }
        // sprite frames
        for (frames, limit, k) in [(0, 5, 1), (0x1000, 5, 1), (0x1000, 5, 5), (0x1000, 5, 9)] {
            let this = t.junk(0xd0, 9);
            let map = t.alloc(
                &[0u32, k, 3]
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            t.w32(this + 0xb8, frames);
            t.w32(this + 0xc8, map);
            t.w32(this + 0x20, limit);
            let out = t.zeros(4);
            t.check(
                &format!("{label}: frame {frames:#x} {k}"),
                0x0056_d690,
                Conv::Thiscall(this),
                &[1, out],
                |g| Out::U32(mb::frame(g, this, 1, out)),
            );
        }
        // list unlinks
        for (owned, pos) in [(false, 0), (true, 0), (true, 1), (true, 2), (true, 9)] {
            let this = t.junk(0x50, 1);
            let nodes: Vec<u32> = (0..3).map(|_| t.junk(0x14, 2)).collect();
            for k in 0..3 {
                t.w32(nodes[k] + 0xc, if k + 1 < 3 { nodes[k + 1] } else { 0 });
                t.w32(nodes[k] + 0x10, this);
            }
            t.w32(this + 0x44, nodes[0]);
            let node = if pos < 3 { nodes[pos] } else { t.junk(0x14, 3) };
            if !owned {
                t.w32(node + 0x10, 0);
            } else if pos >= 3 {
                t.w32(node + 0x10, this);
            }
            t.check(
                &format!("{label}: unlink {owned} {pos}"),
                0x005c_ea90,
                Conv::Thiscall(this),
                &[node],
                |g| Out::U32(mb::unlink_node(g, this, node)),
            );
        }
        let head = t.rebase(mi::UTILITY_LIST);
        for (f, pos) in [
            (0x0050_7b80, 0usize),
            (0x0050_7b80, 2),
            (0x0050_7b70, 1),
            (0x0050_7b70, 9),
            (0x0050_7b80, 8),
        ] {
            let nodes: Vec<u32> = (0..3).map(|_| t.junk(8, 4)).collect();
            for k in 0..3 {
                t.w32(nodes[k] + 4, if k + 1 < 3 { nodes[k + 1] } else { 0 });
            }
            t.w32(head, if pos == 8 { 0 } else { nodes[0] });
            let x = if pos < 3 { nodes[pos] } else { t.junk(8, 5) };
            t.check(
                &format!("{label}: unlink utility {f:#x} {pos}"),
                f,
                Conv::Cdecl,
                &[x],
                |g| Out::U32(mb::unlink_utility(g, x)),
            );
        }
    }
}

#[test]
fn constructs_match_the_original() {
    assert_eq!(mi::CONSTRUCTS.len(), 6);
    for (label, mut t) in Twins::all() {
        for &(f, c) in mi::CONSTRUCTS {
            for n in [0, 1, 3] {
                let dst = t.junk((c.size * 4) as usize, 1);
                let src = t.junk((c.size * 4) as usize, 2);
                let copy = matches!(c.fill, mi::Fill::Copy);
                let args: Vec<u32> = if copy {
                    vec![dst, src, n]
                } else {
                    vec![dst, n]
                };
                let what = format!("{label}: construct {f:#x} {n}");
                t.check(&what, f, Conv::Cdecl, &args, |g| {
                    let r = mb::construct(g, c, dst, src, n);
                    if n == 0 { Out::None } else { Out::U32(r) }
                });
            }
        }
    }
}

#[test]
fn releases_and_arrays_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(mi::HIERARCHY_NODE_DTOR, true, 0, 0, 0x5d);
        t.stub(mi::FREE_LIST_DTOR, true, 0, 0, 0x6e);
        t.stub(0x0054_0b80, false, 2, 0, 0x4c);
        for flags in [0, 1, 2, 3] {
            let obj = t.junk(0x20, 1);
            t.check(
                &format!("{label}: data source delete {flags}"),
                0x0052_3dd0,
                Conv::Thiscall(obj),
                &[flags],
                |g| Out::U32(mb::delete_data_source(g, obj, flags)),
            );
        }
        for owned in [true, false] {
            for n in [0, 1, 2, 5] {
                for null in [false, true] {
                    let els = if null { 0 } else { t.junk(8, 3) };
                    let what = format!("{label}: owned {owned} n {n} null {null}");
                    for f in [0x0051_9150u32, 0x0051_9340] {
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(n, owned));
                        t.w32(arr + 4, els);
                        t.check(
                            &format!("{what}: release {f:#x}"),
                            f,
                            Conv::Thiscall(arr),
                            &[],
                            |g| {
                                Out::U32(if f == 0x0051_9150 {
                                    mb::release_free(g, arr)
                                } else {
                                    mb::release_sharray(g, arr)
                                })
                            },
                        );
                    }
                    for keep in [0, 1, 3, 9] {
                        let arr = t.alloc(&[0; 8]);
                        t.w32(arr, count(n, owned));
                        t.w32(arr + 4, els);
                        t.check(
                            &format!("{what}: truncate {keep}"),
                            0x0053_cf20,
                            Conv::Thiscall(arr),
                            &[keep],
                            |g| Out::U32(mb::truncate(g, arr, keep)),
                        );
                    }
                }
            }
        }
        let same = t.junk(0x10, 4);
        let other = t.junk(8, 1);
        for owned in [true, false] {
            for (prev, els) in [(same, same), (0, same), (same, 0), (other, same)] {
                for n in [0, 2, 0x8000_0000, 0xffff_ffff] {
                    let what =
                        format!("{label}: setBorrowed owned {owned} {prev:#x} {els:#x} {n:#x}");
                    let arr = t.alloc(&[0; 8]);
                    t.w32(arr, count(5, owned));
                    t.w32(arr + 4, prev);
                    t.check(
                        &format!("{what} 0x52d000"),
                        0x0052_d000,
                        Conv::Thiscall(arr),
                        &[els, n],
                        |g| {
                            Out::U32(array::bridge::set_borrowed(
                                g,
                                arr,
                                els,
                                n,
                                array::Old::FreeAlways,
                            ))
                        },
                    );
                    let arr = t.alloc(&[0; 8]);
                    t.w32(arr, count(5, owned));
                    t.w32(arr + 4, prev);
                    t.check(
                        &format!("{what} 0x5441e0"),
                        0x0054_41e0,
                        Conv::Thiscall(arr),
                        &[els, n],
                        |g| Out::U32(mb::set_borrowed_with(g, 0x0054_0b80, arr, els, n)),
                    );
                }
            }
        }
        let g = t.rebase(mi::FREE_LIST);
        for p in [0, 0x1234_5678] {
            t.w32(g, p);
            t.check(
                &format!("{label}: free list {p:#x}"),
                0x0053_a0e0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(mb::delete_free_list(g)),
            );
        }
        t.reset();
    }
}

#[test]
fn forwarding_methods_match_the_original() {
    for (label, mut t) in Twins::all() {
        // copy batch, the blend batches
        t.stub(mi::MEMCPY, false, 3, 0, 0xc0);
        t.stub(mi::BLEND_BATCH, false, 7, 0, 0xb1);
        let a = t.junk(8, 1);
        let b = t.junk(8, 2);
        let inner = t.junk(8, 3);
        t.w32(b, inner);
        let desc = t.junk(8, 4);
        t.check(
            &format!("{label}: copy batch"),
            0x0053_0580,
            Conv::Cdecl,
            &[a, b, 3, 4, 5, desc],
            |g| Out::U32(mb::copy_batch(g, a, b, desc)),
        );
        for (f, k) in [(0x0052_e140, 0x0052_dae0), (0x0052_f600, 0x0052_ef70)] {
            t.check(
                &format!("{label}: blend {f:#x}"),
                f,
                Conv::Cdecl,
                &[1, 2, 3, 4, 5, 6],
                |g| Out::U32(mb::blend_batch(g, k, [1, 2, 3, 4, 5, 6])),
            );
        }
        // strncpy through its import slot
        let s = t.stub_code(0x5c, false, 3, 0, 0x1234_5600);
        let slot = t.rebase(mi::STRNCPY);
        t.w32(slot, s);
        let this = t.junk(0x110, 5);
        t.check(
            &format!("{label}: set name"),
            0x0061_6dc0,
            Conv::Thiscall(this),
            &[0x4242],
            |g| Out::U32(mb::set_name(g, this, 0x4242)),
        );
        // the avoid-overwrite check
        for (found, v, limit) in [
            (false, 0u32, 0),
            (true, 5, 4),
            (true, 4, 4),
            (true, 0x1ff, 0x300),
        ] {
            t.reset();
            let cell = t.alloc(&v.to_le_bytes());
            t.stub(mi::AVOID_FIND, true, 1, 1, if found { cell } else { 0 });
            let this = t.junk(0x10, 6);
            t.w32(this + 8, limit);
            t.check(
                &format!("{label}: may write {found} {v} {limit}"),
                0x0061_6870,
                Conv::Thiscall(this),
                &[7, 8],
                |g| Out::U32(mb::may_write(g, this, 7)),
            );
        }
        // set lookups and inserts
        for r in [0, 0x55] {
            t.reset();
            t.stub(mi::SET_LOOKUP, true, 1, 1, r);
            t.stub(mi::SET_INSERT, true, 2, 2, 0x66);
            for n in [0, 3, 0x8000_0002] {
                let this = t.junk(8, 7);
                t.w32(this, n);
                t.check(
                    &format!("{label}: append {r:#x} {n:#x}"),
                    0x0051_c2e0,
                    Conv::Thiscall(this),
                    &[9],
                    |g| Out::U32(mb::append(g, this, 9)),
                );
            }
        }
        // by-name and by-source setters
        for (name, found) in [(0, 0), (0x71, 0), (0x71, 0x99)] {
            for find in [0, 0x44] {
                t.reset();
                t.stub(mi::SOURCE_BY_NAME, true, 1, 1, found);
                t.stub(mi::COMPONENT_FIND, true, 1, 1, find);
                t.stub(0x0054_3280, true, 2, 2, 0x82);
                t.stub(0x0054_3480, true, 2, 2, 0x84);
                let this = t.junk(0x20, 8);
                let set = if name == 0 { 0 } else { t.junk(8, 9) };
                t.w32(this + 0x10, set);
                let what = format!("{label}: by name {name:#x} {found:#x} {find:#x}");
                for (f, tail) in [(0x0054_3240, 0x0054_3280), (0x0054_3440, 0x0054_3480)] {
                    t.check(
                        &format!("{what}: {f:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[3, name],
                        |g| Out::U32(mb::by_name2(g, tail, this, 3, name)),
                    );
                }
                for (f, tail) in [(0x0054_31f0, 0x0054_3280), (0x0054_33f0, 0x0054_3480)] {
                    t.check(
                        &format!("{what}: {f:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[name],
                        |g| Out::U32(mb::by_name(g, tail, this, name)),
                    );
                }
                for (f, tail) in [(0x0054_3120, 0x0054_3280), (0x0054_3300, 0x0054_3480)] {
                    t.check(
                        &format!("{what}: {f:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[name],
                        |g| Out::U32(mb::by_source(g, tail, this, name)),
                    );
                }
            }
        }
        // the appends
        for r in [0, 0xd] {
            t.reset();
            t.stub(mi::PHYRE_MALLOC, false, 2, 0, 0x0a11_0c00);
            t.stub(mi::APPEND_WORD, true, 2, 2, r);
            t.stub(mi::APPEND_PAIR, true, 2, 2, r);
            for n in [0, 4, 0x8000_0001] {
                let this = t.junk(0x120, 9);
                t.w32(this + 0x100, n);
                t.w32(this + 0x110, n);
                t.frame_len = 4;
                t.check_frame(
                    &format!("{label}: append word {r} {n:#x}"),
                    0x0061_66b0,
                    Conv::Thiscall(this),
                    &[0x31],
                    4,
                    |g| Out::U32(mb::append_word(g, this, 0x31)),
                );
                t.frame_len = 8;
                t.check_frame(
                    &format!("{label}: append pair {r} {n:#x}"),
                    0x0061_66f0,
                    Conv::Thiscall(this),
                    &[0x32, 0x33],
                    24,
                    |g| Out::U32(mb::append_pair(g, this, 0x32, 0x33)),
                );
            }
        }
        t.reset();
        // constant channels: assign and remove (the base assign and the
        // resize stubbed)
        t.stub(mi::CHANNEL_BASE_ASSIGN, true, 1, 1, 0x3a);
        t.stub(mi::CONSTANT_CHANNELS_RESIZE, true, 1, 1, 0x3b);
        let a = t.junk(0x40, 1);
        let b = t.junk(0x40, 2);
        for (x, y) in [(a, b), (a, a)] {
            t.check(
                &format!("{label}: assign {x:#x} {y:#x}"),
                0x0050_f220,
                Conv::Thiscall(x),
                &[y],
                |g| Out::U32(mb::assign_constant_channel(g, x, y)),
            );
        }
        for (n, i) in [(0, 0), (3, 0), (3, 2), (3, 3), (0x8000_0002, 1)] {
            let this = t.junk(0x20, 3);
            let els = t.junk(0x34 * 3, 4);
            t.w32(this + 0xc, n);
            t.w32(this + 0x10, els);
            t.check(
                &format!("{label}: remove {n:#x} {i}"),
                0x0051_6e60,
                Conv::Thiscall(this),
                &[i],
                |g| Out::U32(mb::remove_constant_channel(g, this, i)),
            );
        }
        t.reset();
    }
}

#[test]
fn sharray_lookups_match_the_original() {
    // the PSharray find runs as it is: it only reads the key through the
    // address the original and the port pass
    for (label, mut t) in Twins::all() {
        for n in [0u32, 1, 3, 0x8000_0002] {
            let els = t.alloc(
                &[5u32, 6, 7]
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            let arr = t.alloc(
                &[
                    n.to_le_bytes(),
                    if n == 1 { 6u32 } else { els }.to_le_bytes(),
                ]
                .concat(),
            );
            for key in [5, 6, 7, 8] {
                t.check(
                    &format!("{label}: contains {n:#x} {key}"),
                    0x0051_ca20,
                    Conv::Thiscall(arr),
                    &[key],
                    |g| Out::U32(mb::contains(g, arr, key)),
                );
                for list in [0, arr] {
                    let this = t.junk(0x10, 1);
                    t.w32(this + 8, list);
                    t.check(
                        &format!("{label}: set target {n:#x} {key} {list:#x}"),
                        0x0052_6590,
                        Conv::Thiscall(this),
                        &[key],
                        |g| Out::U32(mb::set_target(g, this, key)),
                    );
                }
            }
        }
    }
}

#[test]
fn more_getter_copies_match_the_original() {
    for (label, mut t) in Twins::all() {
        for round in 0..8u32 {
            let obj = t.junk(0x40, round as u8);
            let c = [0, 1, 7, 0x8000_0003][round as usize % 4];
            t.w32(obj + 0x14, c);
            let i = [0, 1, 5, 0x8000_0001][round as usize % 4];
            let w = |s: &str| format!("{label}: round {round}: {s}");
            t.check(
                &w("0x5111d0"),
                0x0051_11d0,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(smb::element_34(g, obj, i)),
            );
            t.check(&w("0x52f750"), 0x0052_f750, Conv::Thiscall(obj), &[], |g| {
                Out::U32(smb::size(g, (sm::Count::Own(0x14), 0xf), obj))
            });
            t.check(
                &w("0x5315c0"),
                0x0053_15c0,
                Conv::Thiscall(obj),
                &[i],
                |g| Out::U32(smb::first_source(g, obj, i)),
            );
        }
    }
}

// ---- batch 3: array helpers, channel bases, frame-passing updates ----

use ffx_panim::m5p_anim_arrays::{self as ar, bridge as ab};

impl Twins {
    /// A recording stub at the original `sym` (logs `ecx` and `n` words,
    /// pops `pops`) that first writes each `(k, value)`: `value` through
    /// its stack word `k` (a pointer the caller passed), then returns `ret`.
    fn stub_writes(&mut self, sym: u32, n: u32, pops: u32, ret: u32, writes: &[(u32, u32)]) {
        let at = self.rebase(sym);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        for &(k, v) in writes {
            c.extend([0x8b, 0x44, 0x24, (4 + 4 * k) as u8]); // mov eax, [esp+4+4k]
            c.extend([0xc7, 0x00]);
            c.extend(v.to_le_bytes()); // mov dword [eax], v
        }
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// Code (thiscall, no words) that logs `tag` and `ecx` and returns the
    /// `float` at `[ecx + off]` in `ST0`.
    fn float_field_code(&mut self, tag: u32, off: u8) -> u32 {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes());
        c.extend([0xc1, 0xe0, 0x06]);
        c.push(0x05);
        c.extend((log + 4).to_le_bytes());
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes());
        c.extend([0x89, 0x48, 0x04]);
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes());
        c.extend([0xd9, 0x41, off]); // fld dword [ecx+off]
        c.push(0xc3);
        let at = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), at);
        at
    }

    /// Words as bytes.
    fn words(&mut self, w: &[u32]) -> u32 {
        let b: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).collect();
        self.alloc(&b)
    }
}

#[test]
fn word_allocations_match_the_original() {
    for (label, mut t) in Twins::all() {
        let src = t.words(&[11, 12, 13, 14, 15, 16]);
        for fail in [false, true] {
            t.reset();
            let buf = t.junk(0x40, 7);
            t.stub(mi::PHYRE_MALLOC, false, 2, 0, if fail { 0 } else { buf });
            t.stub(sm::PHYRE_FREE, false, 1, 0, 0xf4ee);
            for (total, n) in [(0, 0), (1, 0), (1, 1), (2, 1), (4, 2), (4, 4), (6, 3)] {
                let what = format!("{label}: fail {fail} total {total} n {n}");
                for (f, rest) in [
                    (0x0052_8270, ar::Rest::Keep),
                    (0x0051_3b80, ar::Rest::Zero),
                    (0x0052_bc40, ar::Rest::Zero),
                ] {
                    t.check(
                        &format!("{what}: {f:#x}"),
                        f,
                        Conv::Cdecl,
                        &[total, src, n],
                        |g| Out::U32(ab::alloc_copy(g, rest, total, src, n)),
                    );
                }
                let this = t.junk(0x20, 3);
                t.check(
                    &format!("{what}: sharray"),
                    0x0051_c270,
                    Conv::Thiscall(this),
                    &[total, src, n],
                    |g| Out::U32(ab::sharray_storage(g, this, total, src, n)),
                );
            }
            for f in [0x0051_6eb0, 0x0052_cde0] {
                for (c, n) in [
                    (3, 3),
                    (3, 5),
                    (5, 2),
                    (0x8000_0003, 6),
                    (3, 0),
                    (0x8000_0002, 0x8000_0002),
                ] {
                    let old = t.words(&[21, 22, 23, 24, 25]);
                    let this = t.words(&[c, old]);
                    t.check(
                        &format!("{label}: resize {f:#x} fail {fail} {c:#x} {n:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[n],
                        |g| Out::U32(ab::resize(g, this, n)),
                    );
                }
            }
        }
        t.reset();
    }
}

#[test]
fn channel_lookups_match_the_original() {
    for (label, mut t) in Twins::all() {
        // channels with type ids, the records keyed by them
        let ch: Vec<u32> = (0..3)
            .map(|k| {
                let c = t.junk(0x10, k as u8);
                t.w32(c + 0xc, 0x10 + k);
                c
            })
            .collect();
        let recs = t.words(&[ch[0], 0, 5, 0, ch[1], 0, 6, 0, 0, 0, 7, 0, ch[2], 0, 7, 0]);
        for n in [0, 2, 4, 0x8000_0004] {
            let set = t.junk(0x20, 1);
            t.w32(set + 0x10, n);
            t.w32(set + 0x14, recs);
            for (c, v) in [(ch[0], 5), (ch[1], 6), (0, 7), (ch[2], 7), (ch[2], 8)] {
                let key = t.words(&[c, 0x99, v]);
                t.check(
                    &format!("{label}: find {n:#x} {c:#x} {v}"),
                    0x0051_cb40,
                    Conv::Thiscall(set),
                    &[key],
                    |g| Out::U32(ab::find_channel(g, set, key)),
                );
                // the lookup without a table forwards to the set
                let lk = t.words(&[set, 0]);
                t.check(
                    &format!("{label}: lookup no table {n:#x}"),
                    0x005c_c800,
                    Conv::Thiscall(lk),
                    &[key],
                    |g| Out::U32(ab::lookup_channel(g, lk, key)),
                );
            }
        }
        // the hashed lookup
        let table = t.junk(0x40, 2);
        for r in [0, table, table + 0x30, table.wrapping_sub(0x10)] {
            t.reset();
            t.stub(ar::HASH_FIND, true, 1, 1, r);
            let lk = t.words(&[0, table, 0, 0]);
            for c in [0, ch[1]] {
                let key = t.words(&[c, 1, 0x00ab_cdef]);
                t.frame_len = 4;
                t.check_frame(
                    &format!("{label}: hashed {r:#x} {c:#x}"),
                    0x005c_c800,
                    Conv::Thiscall(lk),
                    &[key],
                    4,
                    |g| Out::U32(ab::lookup_channel(g, lk, key)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn channel_bases_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(sm::STRING_ASSIGN, true, 1, 1, 0x5a);
        let a = t.junk(0x1c, 1);
        let b = t.junk(0x1c, 2);
        t.w32(b + 0x14, 0x1235);
        for (x, y) in [(a, b), (a, a)] {
            t.check(
                &format!("{label}: assign {x:#x} {y:#x}"),
                0x0050_b730,
                Conv::Thiscall(x),
                &[y],
                |g| Out::U32(ab::assign_channel_base(g, x, y)),
            );
        }
        for eq in [0x100, 0x101] {
            t.reset();
            t.stub(ar::STRING_EQUALS, true, 1, 1, eq);
            let base = t.words(&[1, 2, 3, 4, 5, 0x2001, 7]);
            for off in [None, Some(0x10), Some(0), Some(8), Some(0xc), Some(0x18)] {
                let other = t.words(&[1, 2, 3, 4, 5, 0x3000, 7]);
                if let Some(o) = off {
                    t.w32(other + o, 0xee);
                }
                t.check(
                    &format!("{label}: equals {eq:#x} {off:?}"),
                    0x0050_b7b0,
                    Conv::Thiscall(base),
                    &[other],
                    |g| Out::Al(ab::channel_base_equals(g, base, other)),
                );
            }
            // the search over 28-byte bases
            let els = t.words(&[
                1, 2, 3, 4, 9, 0, 7, 1, 2, 3, 4, 5, 0, 7, 1, 2, 3, 4, 5, 0, 8,
            ]);
            let probe = t.words(&[1, 2, 3, 4, 5, 0x10, 7]);
            for n in [0, 1, 3, 0x8000_0002] {
                let this = t.words(&[0, 0, n, els]);
                t.check(
                    &format!("{label}: find base {eq:#x} {n:#x}"),
                    0x0051_cbb0,
                    Conv::Thiscall(this),
                    &[probe],
                    |g| Out::U32(ab::find_channel_base(g, this, probe)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn component_records_match_the_original() {
    for (label, mut t) in Twins::all() {
        for n in [0, 1, 3, 0x8000_0002] {
            let this = t.junk(0x120, 1);
            let recs = t.junk(0x30, 2);
            t.w32(this + 0x108, n);
            t.w32(this + 0x10c, recs);
            t.check(
                &format!("{label}: reset {n:#x}"),
                0x0061_6d60,
                Conv::Thiscall(this),
                &[],
                |g| {
                    ab::reset_records(g, this);
                    Out::None
                },
            );
            // the bound lookup through pairs
            let pairs = t.junk(0x20, 3);
            for k in 0..3 {
                let obj = t.junk(0x110, 4);
                let rs = t.words(&[0, 0, 40 + k, 0, 0, 50 + k]);
                t.w32(obj + 0x10c, rs);
                t.w32(pairs + 8 * k, obj);
                t.w32(pairs + 8 * k + 4, k & 1);
            }
            t.w32(this + 0x100, n);
            t.w32(this + 0x104, pairs);
            for key in [40, 51, 42, 7] {
                t.check(
                    &format!("{label}: bound {n:#x} {key}"),
                    0x0061_6cb0,
                    Conv::Thiscall(this),
                    &[key],
                    |g| Out::U32(ab::find_bound(g, this, key)),
                );
            }
            // the sharray set
            let els = t.junk(0x10, 5);
            let arr = t.words(&[n, els]);
            for (i, v) in [(0, 0), (0, 9), (1, 9), (2, 9), (5, 9)] {
                t.check(
                    &format!("{label}: sharray set {n:#x} {i} {v}"),
                    0x0051_d7b0,
                    Conv::Thiscall(arr),
                    &[i, v],
                    |g| Out::U32(ab::sharray_set(g, arr, i, v)),
                );
            }
        }
        // the triple append (the record by address)
        for r in [0, 0xd] {
            t.reset();
            t.stub(mi::PHYRE_MALLOC, false, 2, 0, 0x0a11_0c00);
            t.stub(ar::APPEND_TRIPLE, true, 2, 2, r);
            for n in [0, 4, 0x8000_0001] {
                let this = t.junk(0x120, 9);
                t.w32(this + 0x108, n);
                t.frame_len = 12;
                t.check_frame(
                    &format!("{label}: append record {r} {n:#x}"),
                    0x0061_6740,
                    Conv::Thiscall(this),
                    &[0x31],
                    24,
                    |g| Out::U32(ab::append_record(g, this, 0x31)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn batch_steps_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(mi::MEMCPY, false, 3, 0, 0xc0);
        for n in [0, 3, 0x8000_0005] {
            let this = t.junk(0x20, 1);
            t.w32(this + 0xc, n);
            let desc = t.junk(0x40, 2);
            t.check(
                &format!("{label}: slot filter {n:#x}"),
                0x0052_a6b0,
                Conv::Thiscall(this),
                &[1, desc, 2],
                |g| Out::U32(ab::slot_filter_setup(g, this, desc)),
            );
        }
        t.stub(ar::EVENT_DISPATCH, true, 4, 4, 0xe1);
        for (with_tc, with_events) in [(false, true), (true, false), (true, true)] {
            for &now in &FLOATS[..6] {
                let this = t.junk(0x20, 3);
                let tc = if with_tc {
                    let s = t.stub_float_code(0x7300, true, 0, 0, now);
                    t.object_with(4, s)
                } else {
                    0
                };
                t.w32(this + 0x10, tc);
                let ev = if with_events { t.junk(8, 4) } else { 0 };
                t.w32(this + 0xc, ev);
                let ctx = t.words(&[0, FLOATS[9], 0, 0]);
                t.check(
                    &format!("{label}: event update {with_tc} {with_events} {now:#x}"),
                    0x0052_4d80,
                    Conv::Thiscall(this),
                    &[0x1234, 5, ctx],
                    |g| Out::U32(ab::event_update(g, this, 0x1234, ctx)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn allocations_and_posts_match_the_original() {
    for (label, mut t) in Twins::all() {
        for fail in [false, true] {
            t.reset();
            let buf = t.junk(0x80, 1);
            t.stub(mi::PHYRE_MALLOC, false, 2, 0, if fail { 0 } else { buf });
            t.stub(ar::TARGETS_COPY, false, 3, 0, 0x71);
            t.stub(ar::TARGETS_CONSTRUCT, false, 2, 0, 0x72);
            for n in [0, 1, 4] {
                let this = t.junk(8, 2);
                t.check(
                    &format!("{label}: trees {fail} {n}"),
                    0x0061_7170,
                    Conv::Thiscall(this),
                    &[n],
                    |g| Out::U32(ab::tree_array(g, this, n)),
                );
                t.check(
                    &format!("{label}: targets {fail} {n}"),
                    0x0051_9d30,
                    Conv::Cdecl,
                    &[n, 0x5000, n / 2],
                    |g| Out::U32(ab::alloc_targets(g, n, 0x5000, n / 2)),
                );
            }
        }
        // the command post (the message by address)
        t.reset();
        t.stub(ar::QUEUE_POST, true, 4, 4, 0x9e);
        let q = t.rebase(ar::COMMAND_QUEUE);
        t.w32(q, 0x00ab_c000);
        let params = t.junk(0x40, 3);
        for (object, p) in [(0, params), (0x77, 0), (0x77, params)] {
            if object != 0 && p != 0 {
                t.frame_len = 0x44;
            }
            t.check_frame(
                &format!("{label}: post {object:#x} {p:#x}"),
                0x0064_2b10,
                Conv::Cdecl,
                &[object, p],
                0x58,
                |g| Out::U32(ab::post_command(g, object, p)),
            );
        }
        // remove a word
        t.stub(ar::WORDS_RESIZE, true, 1, 1, 0x3c);
        for (n, i) in [(0, 0), (3, 0), (3, 2), (3, 3), (0x8000_0002, 1)] {
            let els = t.words(&[1, 2, 3]);
            let this = t.words(&[0, n, els]);
            t.check(
                &format!("{label}: remove word {n:#x} {i}"),
                0x0051_6e00,
                Conv::Thiscall(this),
                &[i],
                |g| Out::U32(ab::remove_word(g, this, i)),
            );
        }
        // the lookup table's destructor
        t.stub(ar::TREE_RELEASE, true, 0, 0, 0x61);
        t.stub(sm::PHYRE_FREE, false, 1, 0, 0x62);
        t.stub(sm::TREE_CLEAR, true, 0, 0, 0x63);
        for f in [0x005c_c770, 0x005c_cc50] {
            let this = t.junk(0x20, 4);
            t.check(
                &format!("{label}: lookup dtor {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(ab::lookup_dtor(g, this)),
            );
        }
        // ring buffer blocks
        let block = t.junk(0x40, 5);
        for r in [0, block] {
            t.stub(ar::RING_ALLOCATE, true, 1, 1, r);
            for id in [0, 5, u32::MAX] {
                let g = t.rebase(ar::BLOCK_ID);
                t.w32(g, id);
                let this = t.junk(0x30, 6);
                t.check(
                    &format!("{label}: block {r:#x} {id:#x}"),
                    0x005c_e930,
                    Conv::Thiscall(this),
                    &[0x20],
                    |g| Out::U32(ab::allocate_block(g, this, 0x20)),
                );
            }
        }
        t.reset();
    }
}

#[test]
fn source_times_match_the_original() {
    for (label, mut t) in Twins::all() {
        let ctl = t.float_field_code(0x7400, 0xc);
        let vt = t.words(&[0, 0, 0, 0, ctl]);
        for (k, &tbits) in FLOATS.iter().enumerate() {
            let this = t.junk(0x20, 1);
            let recs = t.junk(0x4c * 2, 2);
            for r in 0..2 {
                t.w32(recs + 0x4c * r + 0x24, vt);
                t.w32(
                    recs + 0x4c * r + 0x30,
                    FLOATS[(k + 3 * r as usize + 1) % FLOATS.len()],
                );
                t.w32(recs + 0x4c * r + 0x34, FLOATS[(k + 5) % FLOATS.len()]);
            }
            t.w32(this + 0x14, [2, 0x8000_0001, 0][k % 3]);
            t.w32(this + 0x18, recs);
            for i in [0, 1, 2] {
                t.check(
                    &format!("{label}: source time {tbits:#x} {i}"),
                    0x0054_4110,
                    Conv::Thiscall(this),
                    &[i, tbits],
                    |g| Out::U32(ab::set_source_time(g, this, i, tbits)),
                );
            }
        }
    }
}

#[test]
fn past_end_checks_match_the_original() {
    for (label, mut t) in Twins::all() {
        for &end in &FLOATS {
            t.reset();
            t.stub_writes(ar::CLIP_RANGE, 2, 2, 0x55, &[(0, 0x3f00_0000), (1, end)]);
            for (k, &v) in FLOATS.iter().enumerate() {
                let this = t.junk(0x20, 1);
                let recs = t.junk(0x4c * 2, 2);
                for r in 0..2u32 {
                    let rec = recs + 0x4c * r;
                    t.w32(
                        rec,
                        if (k + r as usize).is_multiple_of(5) {
                            0
                        } else {
                            0x0c11
                        },
                    );
                    let s = t.stub_float_code(0x7500 + r, true, 0, 0, v);
                    let ovt = t.words(&[0, 0, 0, 0, s]);
                    t.w32(rec + 0xc, ovt);
                }
                t.w32(this + 0x14, [2, 1, 0, 0x8000_0002][k % 4]);
                t.w32(this + 0x18, recs);
                t.check_frame(
                    &format!("{label}: first past end {end:#x} {v:#x}"),
                    0x0054_2ff0,
                    Conv::Thiscall(this),
                    &[],
                    16,
                    |g| Out::Al(ab::past_end(g, this, 0, 4, 0)),
                );
                for i in [0, 1, 2] {
                    t.check_frame(
                        &format!("{label}: past end {end:#x} {v:#x} {i}"),
                        0x0054_30a0,
                        Conv::Thiscall(this),
                        &[i],
                        16,
                        |g| Out::Al(ab::past_end(g, this, i, 0xc, 0)),
                    );
                }
            }
        }
        t.reset();
    }
}

#[test]
fn clip_set_arrays_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(ar::REGISTER_STORAGE, true, 2, 2, 0x41);
        t.stub(ar::CLIPS_SET_ARRAY, true, 2, 2, 0x42);
        let reg = t.rebase(ar::REGISTRY);
        for ts in [0, 0x0bad_0000] {
            let s = t.stub_code(0x7600, true, 0, 0, ts);
            let vt = t.words(&[0, s]);
            t.w32(reg, vt);
            let this = t.junk(0x10, 1);
            for (els, n) in [(this + 4, 1), (this + 4, 2), (0x1234, 1), (0, 0)] {
                t.check(
                    &format!("{label}: clips set {ts:#x} {els:#x} {n}"),
                    0x0051_dbf0,
                    Conv::Thiscall(this),
                    &[els, n],
                    |g| Out::U32(ab::clips_set_array(g, this, els, n)),
                );
            }
        }
        t.reset();
    }
}
