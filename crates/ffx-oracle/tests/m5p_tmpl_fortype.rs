//! M5 stage 1, lists phyre-tmpl-fortype-a and -b
//! (`ffx_pload::m5p_tmpl_fortype`): every copy in the crate's tables runs
//! its own original against the generic port, on the image alone and on a
//! snapshot (`FFX_SNAPSHOT`). Results (`eax`, or `al` where the original
//! leaves the caller's upper bytes) and every byte written outside the
//! stacks must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs (in the oracles' code area): each call appends its tag,
//! `ecx` and stack arguments to a log in memory, so the diff checks that
//! both made the same calls in the same order, with the same `this`. The
//! stubs return a word with all bytes set, so `al = 1` over the callee's
//! `eax` is checked too.
//!
//! Branch reach (almost none of this ever ran in the game, so every branch
//! is synthetic): `p` 0 and not 0, the flag byte clear, set and set only in
//! a higher byte (the originals test the low byte), the lazy descriptors'
//! guard set and clear, owned and external arrays, set and empty owners,
//! names and pointers. `p` 0 in `destroyInstance` copies that read through
//! it (the object at `0 - adjust` is 0) runs with the null page mapped and
//! a fake vtable there.

#![allow(
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_tmpl_fortype::{self as ft, bridge as t};

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
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

/// How a recording stub logs.
#[derive(Debug, Clone, Copy)]
enum Log {
    /// Its stack words.
    Words,
    /// The four words of the structure its first stack word points at
    /// (the last as a byte): the `swapEndiannessOfObject` visitors, whose
    /// argument is in the caller's frame.
    Visitor,
}

/// Bytes per log entry: tag, `ecx`, six words.
const ENTRY: u32 = 32;

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of [`ENTRY`] bytes.
    log: u32,
    /// Stubs written, by original address.
    stubs: HashMap<u32, u32>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            log: 0,
            stubs: HashMap::new(),
        };
        t.log = t.zeros(4 + 64 * ENTRY as usize);
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

    /// `n` random bytes.
    fn object(&mut self, rng: &mut Rng, n: usize) -> u32 {
        let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// The log entry `i`: tag, `ecx`, six words.
    fn entry(&self, i: u32) -> [u32; 8] {
        let mut e = [0; 8];
        for (k, w) in e.iter_mut().enumerate() {
            *w = Mem::read(&self.b, self.log + 4 + i * ENTRY + 4 * k as u32);
        }
        e
    }

    /// The null page mapped in both, with a fake object at 0 whose
    /// vtable's slots all point at `f` (a stub).
    fn null_object(&mut self, f: u32) {
        self.a.map_zeroed(0, 0x1000).unwrap();
        self.b.map_zeroed(0, 0x1000).unwrap();
        self.w32(0, 0x100);
        for i in 0..16 {
            self.w32(0x100 + 4 * i, f);
        }
    }

    /// Recording stub code (in the code area of both oracles): logs `tag`,
    /// `ecx` when `this` (a cdecl callee's `ecx` is the caller's leftover)
    /// and `n` stack words (or the visitor), pops `pops` words, returns
    /// `ret`. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32, log: Log) -> u32 {
        assert!(n <= 6);
        let l = self.log;
        let mut c = vec![0xa1];
        c.extend(l.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((l + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        match log {
            Log::Words => {
                for i in 0..n {
                    c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
                    c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
                }
            }
            Log::Visitor => {
                c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
                for i in 0..3u8 {
                    c.extend([0x8b, 0x51, 4 * i]); // mov edx, [ecx+4i]
                    c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
                }
                c.extend([0x0f, 0xb6, 0x51, 0x0c]); // movzx edx, byte [ecx+0xc]
                c.extend([0x89, 0x50, 0x14]); // mov [eax+0x14], edx
            }
        }
        c.extend([0xff, 0x05]);
        c.extend(l.to_le_bytes()); // inc dword [log]
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

    /// A recording stub over the original at `sym` (a `jmp` there), tagged
    /// with `sym`. Once per address.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        self.stub_log(sym, this, n, pops, ret, Log::Words);
    }

    fn stub_log(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32, log: Log) {
        if self.stubs.contains_key(&sym) {
            return;
        }
        let code = self.stub_code(sym, this, n, pops, ret, log);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        self.stubs.insert(sym, code);
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

    /// Runs the original at `sym` in `a` and `port` on `b` (the call log
    /// emptied first); compares the result as `port` says and every byte
    /// written. Returns the number of logged calls.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
        self.entries()
    }
}

/// Flag words: clear, set, set only above the low byte (read as clear),
/// all ones.
const FLAGS: [u32; 4] = [0, 1, 0x100, 0xffff_ffff];

/// A callee's `eax`: every byte set, so `al = 1` over it shows.
const RET: u32 = 0x8765_43fe;

#[test]
fn deleting_destructors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1234_5678);
        t.stub(ft::DESTRUCT, true, 0, 0, RET);
        t.stub(ft::PHYRE_FREE, false, 1, 0, RET ^ 1);
        for &(f, vtable) in ft::DELETE {
            assert_eq!(ft::word_of(ft::DELETE, f), vtable);
            for flags in [0, 1, 0xfe, 0xff] {
                let this = t.object(&mut rng, 0x40);
                let calls = t.check(
                    &format!("{name} {f:#x}({this:#x}, {flags:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| Out::U32(t::delete(g, this, flags, vtable)),
                );
                assert_eq!(calls, 1 + (flags & 1), "{f:#x}");
                assert_eq!(t.entry(0)[..2], [ft::DESTRUCT, this]);
            }
        }
    }
}

#[test]
fn swap_forwarders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2345_6789);
        t.stub(ft::SWAP_ENDIANNESS_OF_OBJECT, true, 2, 2, RET);
        let this = t.object(&mut rng, 0x20);
        for &(f, desc) in ft::SWAP {
            for flag in FLAGS {
                for p in [0, t.object(&mut rng, 0x20)] {
                    let calls = t.check(
                        &format!("{name} {f:#x}({p:#x}, {flag:#x})"),
                        f,
                        Conv::Thiscall(this),
                        &[p, flag],
                        |g| Out::U32(t::swap(g, desc, p, flag)),
                    );
                    assert_eq!(calls, 1);
                    let d = t.rebase(desc);
                    assert_eq!(
                        t.entry(0)[..4],
                        [
                            ft::SWAP_ENDIANNESS_OF_OBJECT,
                            d,
                            p,
                            u32::from(flag & 0xff != 0)
                        ]
                    );
                }
            }
        }
        for &(f, pair) in ft::SWAP_BASE {
            for flag in FLAGS {
                for p in [0, t.object(&mut rng, 0x20)] {
                    t.check(
                        &format!("{name} {f:#x}({p:#x}, {flag:#x})"),
                        f,
                        Conv::Thiscall(this),
                        &[p, flag],
                        |g| Out::U32(t::swap_base(g, pair, p, flag)),
                    );
                    let q = if p == 0 { 0 } else { p + pair[1] };
                    assert_eq!(t.entry(0)[2], q, "{f:#x}");
                }
            }
        }
        for &(f, getter) in ft::SWAP_GET {
            let desc = t.object(&mut rng, 0x40);
            t.stub(getter, false, 0, 0, desc);
            for flag in FLAGS {
                for p in [0, t.object(&mut rng, 0x20)] {
                    let calls = t.check(
                        &format!("{name} {f:#x}({p:#x}, {flag:#x})"),
                        f,
                        Conv::Thiscall(this),
                        &[p, flag],
                        |g| Out::U32(t::swap_get(g, getter, p, flag)),
                    );
                    assert_eq!(calls, 2);
                    assert_eq!(t.entry(1)[1], desc);
                }
            }
        }
        for &(f, pair) in ft::SWAP_PAIR {
            t.stub(pair[0], false, 1, 0, RET);
            t.stub(pair[1], false, 1, 0, RET ^ 0x100);
            for flag in FLAGS {
                for p in [0, t.object(&mut rng, 0x20)] {
                    t.check(
                        &format!("{name} {f:#x}({p:#x}, {flag:#x})"),
                        f,
                        Conv::Thiscall(this),
                        &[p, flag],
                        |g| Out::U32(t::swap_pair(g, pair, p, flag)),
                    );
                    let want = if flag & 0xff != 0 { pair[0] } else { pair[1] };
                    assert_eq!(t.entry(0)[0], want, "{f:#x}");
                }
            }
        }
    }
}

#[test]
fn lazy_descriptors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3456_789a);
        t.stub(ft::SWAP_ENDIANNESS_OF_OBJECT, true, 2, 2, RET);
        t.stub(ft::ATEXIT, false, 1, 0, RET ^ 2);
        for &(f, lazy) in ft::SWAP_LAZY {
            assert_eq!(ft::lazy_of(f), lazy);
            t.stub(lazy.ctor, true, 0, 0, RET ^ 3);
            for guard in [0x1234_5671, 0x1234_5670, 0, 0xffff_fffe] {
                let (g, fl) = (t.rebase(lazy.guard), t.rebase(lazy.flags));
                t.w32(g, guard);
                t.w32(fl, rng.next());
                let p = t.object(&mut rng, 0x20);
                let calls = t.check(
                    &format!("{name} {f:#x}({p:#x}) guard {guard:#x}"),
                    f,
                    Conv::Cdecl,
                    &[p],
                    |g| Out::U32(t::swap_lazy(g, &lazy, p)),
                );
                assert_eq!(calls, if guard & 1 == 0 { 3 } else { 1 }, "{f:#x}");
            }
        }
    }
}

#[test]
fn destroy_instances_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4567_89ab);
        let this = t.object(&mut rng, 0x20);
        for &f in ft::TRUE {
            ft::listed(ft::TRUE, f);
            for p in [0, t.object(&mut rng, 0x20)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |_| Out::U8(1),
                );
            }
        }
        let virtual_dtor = t.stub_code(0xd7, true, 1, 1, RET, Log::Words);
        t.null_object(virtual_dtor);
        for &(f, pair) in ft::DESTROY_VIRTUAL {
            let [adjust, slot] = pair;
            let object = t.object(&mut rng, 0x40);
            let vtable = t.zeros(0x40);
            t.w32(object, vtable);
            t.w32(vtable + slot, virtual_dtor);
            for p in [0, object + adjust] {
                let calls = t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U32(t::destroy_virtual(g, pair, p)),
                );
                assert_eq!(calls, 1);
                assert_eq!(t.entry(0)[1], if p == 0 { 0 } else { object });
            }
        }
        for &(f, pair) in ft::DESTROY_PAIR {
            t.stub(pair[0], true, 0, 0, RET);
            let p = t.object(&mut rng, 0x40);
            let calls = t.check(
                &format!("{name} {f:#x}({p:#x})"),
                f,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_pair(g, pair, p)),
            );
            assert_eq!(calls, 2);
        }
        for &(f, dtor) in ft::DESTROY_CALL {
            t.stub(dtor, true, 0, 0, RET);
            let p = t.object(&mut rng, 0x40);
            let calls = t.check(
                &format!("{name} {f:#x}({p:#x})"),
                f,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_call(g, dtor, p)),
            );
            assert_eq!(calls, 1);
        }
    }
}

#[test]
fn script_accessors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5678_9abc);
        t.stub(ft::PHYRE_PRINTF, false, 3, 0, RET);
        t.stub(ft::PUSH_OBJECT, false, 3, 0, RET ^ 4);
        t.stub(ft::PUSH_NIL[0], false, 2, 0, RET ^ 5);
        t.stub(ft::PUSH_NIL[1], false, 1, 0, RET ^ 6);
        for &(f, format) in ft::SCRIPT_VALUE {
            for type_name in [0, t.object(&mut rng, 8)] {
                let this = t.object(&mut rng, 0x20);
                t.w32(this + 0x18, type_name);
                t.check(
                    &format!("{name} {f:#x} name {type_name:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[rng.next(), rng.next()],
                    |g| Out::U32(t::script_value(g, this, format)),
                );
            }
        }
        for &(f, get) in ft::GET_POINTER {
            t.stub(get, false, 1, 0, RET ^ 7);
            let this = t.object(&mut rng, 0x20);
            let (l, out) = (rng.next(), t.object(&mut rng, 4));
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[l, out],
                |g| Out::U32(t::get_pointer(g, get, l, out)),
            );
        }
        for &(f, pair) in ft::PUT_POINTER {
            for object in [0, t.object(&mut rng, 0x20)] {
                let this = t.object(&mut rng, 0x20);
                let pp = t.alloc(&object.to_le_bytes());
                let l = rng.next();
                let calls = t.check(
                    &format!("{name} {f:#x} object {object:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[l, pp],
                    |g| Out::U32(t::put_pointer(g, pair, l, pp)),
                );
                assert_eq!(calls, if object == 0 { 2 } else { 1 });
            }
        }
    }
}

#[test]
fn single_slots_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x6789_abcd);
        let this = t.object(&mut rng, 0x20);
        // slot 13: -1
        t.check(
            &format!("{name} 0x43cc10"),
            0x0043_cc10,
            Conv::Thiscall(this),
            &[1, 2, 3],
            |_| Out::U32(t::minus_one()),
        );
        // the namespace unlink
        for _ in 0..2 {
            let p = t.object(&mut rng, 0x10);
            let next = t.object(&mut rng, 0x10);
            let prev = t.object(&mut rng, 0x10);
            t.w32(p + 4, next);
            t.w32(p + 8, prev);
            t.check(
                &format!("{name} unlink"),
                0x0043_d400,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::unlink(g, p)),
            );
        }
        // the owned name
        t.stub(ft::PHYRE_FREE, false, 1, 0, RET);
        for flags in [0u32, 1, 0xfe, 0xff] {
            let p = t.object(&mut rng, 0x20);
            t.w32(p + 0x10, flags | 0x1200);
            let calls = t.check(
                &format!("{name} owned name {flags:#x}"),
                0x0043_a550,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_owned_name(g, p)),
            );
            assert_eq!(calls, flags & 1);
        }
        // PClassDescriptor's swapEndianness: the visitor in the frame
        t.stub_log(ft::SWAP_VISITORS[0], true, 1, 1, RET, Log::Visitor);
        t.stub_log(ft::SWAP_VISITORS[1], true, 1, 1, RET ^ 8, Log::Visitor);
        for flag in FLAGS {
            for p in [0, t.object(&mut rng, 0x40)] {
                let calls = t.check(
                    &format!("{name} class descriptor swap {p:#x} {flag:#x}"),
                    0x0043_d5d0,
                    Conv::Thiscall(this),
                    &[p, flag],
                    |g| Out::U32(t::swap_class_descriptor(g, p, flag)),
                );
                assert_eq!(calls, 1);
                assert_eq!(t.entry(0)[4], if p == 0 { 0 } else { p + 0x18 });
                assert_eq!(t.entry(0)[5], u32::from(flag & 0xff != 0));
            }
        }
        // the namespace's owned descriptors
        t.stub(ft::RELEASE_OWNED, true, 0, 0, RET);
        for p in [0, t.object(&mut rng, 0x20)] {
            t.check(
                &format!("{name} namespace {p:#x}"),
                0x0043_e880,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_namespace(g, p)),
            );
            assert_eq!(t.entry(0)[1], if p == 0 { 0 } else { p - 8 });
        }
        // two destructors and the owner between
        t.stub(ft::WITH_OWNER[0], true, 0, 0, RET);
        t.stub(ft::WITH_OWNER[1], true, 1, 1, RET ^ 9);
        t.stub(ft::WITH_OWNER[2], true, 0, 0, RET ^ 0xa);
        for owner in [0, t.object(&mut rng, 0x10)] {
            let p = t.object(&mut rng, 0x60);
            t.w32(p + 0x40, owner);
            let calls = t.check(
                &format!("{name} with owner {owner:#x}"),
                0x0044_15a0,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_with_owner(g, p)),
            );
            assert_eq!(calls, if owner == 0 { 2 } else { 3 });
        }
        // an owned or external element array
        t.stub(ft::RELEASE_ELEMENTS, false, 2, 0, RET);
        for count in [0, 5, 0x7fff_ffff, 0x8000_0005, 0x8000_0000] {
            let p = t.object(&mut rng, 0x20);
            t.w32(p + 8, count);
            let calls = t.check(
                &format!("{name} elements {count:#x}"),
                0x004d_7140,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(t::destroy_array_elements(g, p)),
            );
            assert_eq!(calls, u32::from(count >> 31 == 0));
        }
    }
}

#[test]
fn list_b_single_slots_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x789a_bcde);
        let this = t.object(&mut rng, 0x20);
        // the shape array at p - 1, with p 0 reading the null page
        t.stub(ft::PHYRE_FREE, false, 1, 0, RET);
        let free_null = t.stub_code(0xf0, true, 1, 1, RET, Log::Words);
        t.null_object(free_null);
        for count in [0, 3, 0x8000_0003] {
            for data in [0, rng.next() | 1] {
                for p in [0, t.object(&mut rng, 0x40) + 1] {
                    let a = if p == 0 { 0 } else { p - 1 };
                    t.w32(a + 0x1c, count);
                    t.w32(a + 0x20, data);
                    let calls = t.check(
                        &format!("{name} shapes {p:#x} {count:#x} {data:#x}"),
                        0x005f_8960,
                        Conv::Thiscall(this),
                        &[p],
                        |g| Out::U32(t::destroy_shape_array(g, p)),
                    );
                    assert_eq!(calls, u32::from(count >> 31 == 0 && data != 0));
                }
            }
        }
        // the descriptor constructor
        let [base, namespace, ctor, defaults, validation, write_mask] = ft::RIGID_BODY_SHAPES_CALLS;
        t.stub(base, false, 0, 0, RET);
        t.stub(namespace, false, 0, 0, RET ^ 0x11);
        t.stub(ctor, true, 6, 6, RET ^ 0x12);
        for f in [defaults, validation, write_mask] {
            t.stub(f, true, 1, 1, RET ^ f);
        }
        let cell = t.rebase(ft::RIGID_BODY_SHAPES_DATA[0]);
        t.w32(cell, rng.next());
        let p = t.object(&mut rng, 0xa0);
        let calls = t.check(
            &format!("{name} shapes ctor"),
            0x005d_d0a0,
            Conv::Thiscall(p),
            &[],
            |g| Out::U32(t::construct_rigid_body_shapes(g, p)),
        );
        assert_eq!(calls, 6);
    }
}
