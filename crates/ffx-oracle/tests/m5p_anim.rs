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

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle);

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
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
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
        };
        t.log = t.zeros(0x4000);
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

    /// A recording stub (as [`Twins::stub_code`], cdecl) that leaves the
    /// `float` with `bits` in `ST0`, placed at the original `sym`.
    fn stub_float(&mut self, sym: u32, n: u32, bits: u32) {
        let at = self.rebase(sym);
        let cell = self.alloc(&bits.to_le_bytes());
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
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
        let out = port(&mut G(&mut self.b));
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
