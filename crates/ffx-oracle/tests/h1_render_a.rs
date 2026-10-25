//! M4 port round 1, list h1-render-a (`ffx_prender`): each port against
//! its own original, on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! Families of copies (the class-descriptor callbacks, the `Bind`
//! functions) are checked copy by copy: every address in the crate's
//! tables runs its own original.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and stack arguments
//! to a log in memory, so the diff checks that both made the same calls in
//! the same order, with the same `this`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::{basic, descriptor, registry, texture};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;

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

/// What a recording stub returns in `eax`.
#[derive(Debug, Clone, Copy)]
enum Ret {
    /// This value.
    Imm(u32),
    /// The word at this address when it runs.
    Cell(u32),
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    /// Stubs written, by original address.
    stubs: std::collections::HashMap<u32, u32>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            stubs: std::collections::HashMap::new(),
        };
        t.log = t.zeros(0x8000);
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

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// A recording stub at the original `sym` (a `jmp` there): logs
    /// `sym`, `ecx` (when `this`: a cdecl callee's `ecx` is the caller's
    /// leftover) and `n` stack arguments, pops `pops` words, returns `ret`.
    /// Once per address.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: Ret) {
        if self.stubs.contains_key(&sym) {
            return;
        }
        let addr = self.stub_code(sym, this, n, pops, ret);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        self.stubs.insert(sym, addr);
    }

    /// The stub of [`Twins::stub`] in the cave only (for pointers), `tag`
    /// its log tag. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: Ret) -> u32 {
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
        match ret {
            Ret::Imm(_) => c.push(0xb8),  // mov eax, imm
            Ret::Cell(_) => c.push(0xa1), // mov eax, [cell]
        }
        c.extend(
            match ret {
                Ret::Imm(v) | Ret::Cell(v) => v,
            }
            .to_le_bytes(),
        );
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
        addr
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
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
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }
}

// ---- class-descriptor callbacks ----

/// A descriptor-like `this` and object pointers to call the callbacks
/// with: 0, and a buffer full of a pattern.
fn object(t: &mut Twins, rng: &mut Rng) -> u32 {
    let bytes: Vec<u8> = (0..32).map(|_| rng.next() as u8).collect();
    t.alloc(&bytes)
}

#[test]
fn trivial_callbacks_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_9bdf);
        let this = object(&mut t, &mut rng);
        for &f in descriptor::NOTHING {
            for p in [0, object(&mut t, &mut rng)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |_| Out::U8(u8::from(descriptor::bridge::nothing_to_do())),
                );
            }
        }
        for &f in descriptor::FIX_NONE {
            for p in [0, object(&mut t, &mut rng)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |_| Out::U32(descriptor::bridge::no_objects_to_fix()),
                );
            }
        }
    }
}

#[test]
fn construct_init_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_1357);
        let this = object(&mut t, &mut rng);
        for &(f, init) in descriptor::INIT {
            assert_eq!(descriptor::init_of(f), init);
            for p in [0, object(&mut t, &mut rng)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::construct_init(g, p, init))),
                );
            }
        }
    }
}

#[test]
fn destroy_and_construct_call_the_original_callee() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f1e_2d3c);
        let this = object(&mut t, &mut rng);
        for &(_, callee) in descriptor::DESTROY.iter().chain(descriptor::CONSTRUCT) {
            t.stub(callee, true, 0, 0, Ret::Imm(rng.next()));
        }
        for &(f, dtor) in descriptor::DESTROY {
            assert_eq!(descriptor::callee_of(descriptor::DESTROY, f), dtor);
            for p in [0, object(&mut t, &mut rng)] {
                let before = t.log_entries();
                t.check(
                    &format!("{name} destroy {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::destroy(g, dtor, p))),
                );
                // the destructor runs, on `p`, even when it is 0
                assert_eq!(t.log_entries(), before + 1, "{f:#x}");
            }
        }
        for &(f, ctor) in descriptor::CONSTRUCT {
            assert_eq!(descriptor::callee_of(descriptor::CONSTRUCT, f), ctor);
            for p in [0, object(&mut t, &mut rng)] {
                let before = t.log_entries();
                t.check(
                    &format!("{name} construct {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::construct(g, ctor, p))),
                );
                assert_eq!(t.log_entries(), before + u32::from(p != 0), "{f:#x}");
            }
        }
    }
}

#[test]
fn bind_matches_the_original() {
    for (name, mut t) in Twins::all() {
        t.stub(
            descriptor::REGISTER_TYPE,
            false,
            1,
            0,
            Ret::Imm(0x0000_0101),
        );
        t.stub(
            descriptor::ADD_TO_NAMESPACE,
            true,
            0,
            0,
            Ret::Imm(0x5a5a_1234),
        );
        for &(f, d) in descriptor::BIND {
            assert_eq!(descriptor::callee_of(descriptor::BIND, f), d);
            let before = t.log_entries();
            t.check(&format!("{name} bind {f:#x}"), f, Conv::Cdecl, &[], |g| {
                Out::U32(descriptor::bridge::bind(g, d))
            });
            assert_eq!(t.log_entries(), before + 2, "{f:#x}");
        }
    }
}

// ---- arrays: destroyInstance, setToExternalBuffer, the element release ----

/// Array header count words: owned (0, 1, 2, many) and external.
const COUNTS: [u32; 6] = [0, 1, 2, 5, 0x8000_0003, 0x8000_0000];

/// Stubs `PhyreFree` and every element release of the tables.
fn stub_releases(t: &mut Twins) {
    t.stub(descriptor::FREE, false, 1, 0, Ret::Imm(0x0bad_f00d));
    let elements = descriptor::ARRAY_DESTROY
        .iter()
        .map(|(_, (_, r))| *r)
        .chain(descriptor::SET_EXTERNAL.iter().map(|(_, r)| *r));
    for r in elements.collect::<Vec<_>>() {
        if let descriptor::Release::Elements(f) = r {
            t.stub(f, false, 2, 0, Ret::Imm(0x0e1e_0e1e));
        }
    }
}

#[test]
fn array_destroy_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_1234);
        let this = object(&mut t, &mut rng);
        stub_releases(&mut t);
        for &(f, (off, how)) in descriptor::ARRAY_DESTROY {
            assert_eq!(descriptor::array_of(f), (off, how));
            for count in COUNTS {
                for data in [0, 0x1234_5670 + rng.next() % 0x100] {
                    let p = object(&mut t, &mut rng);
                    t.w32(p + off, count);
                    t.w32(p + off + 4, data);
                    let before = t.log_entries();
                    t.check(
                        &format!("{name} {f:#x} count {count:#x} data {data:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[p],
                        |g| Out::U8(u8::from(descriptor::bridge::destroy_array(g, p, off, how))),
                    );
                    let calls = t.log_entries() - before;
                    let owned = descriptor::owned(count);
                    let want = match how {
                        descriptor::Release::Free => owned && data != 0,
                        descriptor::Release::FreeShared { check_null } => {
                            owned && count > 1 && (data != 0 || !check_null)
                        }
                        descriptor::Release::Elements(_) => owned,
                    };
                    assert_eq!(calls, u32::from(want), "{f:#x} {count:#x} {data:#x}");
                }
            }
        }
    }
}

#[test]
fn tagged_and_virtual_destroy_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4242_4242);
        let this = object(&mut t, &mut rng);
        stub_releases(&mut t);
        for &(f, off) in descriptor::TAGGED_FREE {
            assert_eq!(descriptor::callee_of(descriptor::TAGGED_FREE, f), off);
            for block in [0, 0x2000_0000, 0x2000_0001, 0xffff_fffe, 0xffff_ffff] {
                let bytes: Vec<u8> = (0..0x100).map(|_| rng.next() as u8).collect();
                let p = t.alloc(&bytes);
                t.w32(p + off, block);
                t.check(
                    &format!("{name} {f:#x} block {block:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::destroy_tagged(g, p, off))),
                );
            }
        }
        // the object's vtable: slot 0 a recording stub popping the flags
        let dtor = t.stub_code(0x00de_1e7e, true, 1, 1, Ret::Imm(0x1357_2468));
        let vtable = t.alloc(&dtor.to_le_bytes());
        for &f in descriptor::VIRTUAL_DELETE {
            let object_ = object(&mut t, &mut rng);
            t.w32(object_, vtable);
            let p = object_ + 4;
            let before = t.log_entries();
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U8(u8::from(descriptor::bridge::destroy_virtual(g, p))),
            );
            assert_eq!(t.log_entries(), before + 1);
        }
    }
}

#[test]
fn destroy_checked_and_construct_default_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0dd0_0dd0);
        let this = object(&mut t, &mut rng);
        let cell = t.zeros(4);
        for &(_, [check, release, dtor]) in descriptor::DESTROY_CHECKED {
            t.stub(check, true, 0, 0, Ret::Cell(cell));
            t.stub(release, true, 0, 0, Ret::Imm(rng.next()));
            t.stub(dtor, true, 0, 0, Ret::Imm(rng.next()));
        }
        for &(f, calls) in descriptor::DESTROY_CHECKED {
            assert_eq!(descriptor::three_of(descriptor::DESTROY_CHECKED, f), calls);
            // only `al` of the check counts
            for checked in [0, 1, 0x100, 0xffff_ff01, 0x80] {
                t.w32(cell, checked);
                let p = object(&mut t, &mut rng);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} check {checked:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::destroy_checked(g, calls, p))),
                );
                let want = 2 + u32::from(checked & 0xff != 0);
                assert_eq!(t.log_entries() - before, want, "{f:#x} {checked:#x}");
            }
        }
        for &(_, [_, get, ctor]) in descriptor::CONSTRUCT_DEFAULT {
            t.stub(get, true, 0, 0, Ret::Cell(cell));
            t.stub(ctor, true, 1, 1, Ret::Imm(0x7654_3210 ^ ctor));
        }
        for &(f, calls) in descriptor::CONSTRUCT_DEFAULT {
            assert_eq!(
                descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, f),
                calls
            );
            for default in [0, 0x1111_2222, 0x3333_4400] {
                t.w32(cell, default);
                for p in [0, object(&mut t, &mut rng)] {
                    t.check(
                        &format!("{name} {f:#x} default {default:#x} p {p:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[p],
                        |g| Out::U32(descriptor::bridge::construct_default(g, calls, p)),
                    );
                }
            }
        }
    }
}

#[test]
fn set_external_and_release_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0001);
        stub_releases(&mut t);
        for &(f, how) in descriptor::SET_EXTERNAL {
            assert_eq!(descriptor::release_of(descriptor::SET_EXTERNAL, f), how);
            for old in COUNTS {
                for data in [0, 0x4000_0000] {
                    for (buffer, count) in [
                        (0x4000_0000, 3),
                        (0x5000_0000, 0),
                        (0x5000_0000, 7),
                        (0, 2),
                        (0x5000_0000, 0x8000_0001),
                        (0x5000_0000, 0x7fff_ffff),
                    ] {
                        let array = object(&mut t, &mut rng);
                        t.w32(array, old);
                        t.w32(array + 4, data);
                        t.check(
                            &format!(
                                "{name} {f:#x} old {old:#x}/{data:#x} new {buffer:#x}/{count:#x}"
                            ),
                            f,
                            Conv::Thiscall(array),
                            &[buffer, count],
                            |g| {
                                Out::U32(descriptor::bridge::set_external(
                                    g, array, buffer, count, how,
                                ))
                            },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn release_tagged_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0002);
        // the element releases themselves stay original here
        t.stub(descriptor::FREE, false, 1, 0, Ret::Imm(0x0bad_f00d));
        for &f in descriptor::RELEASE_TAGGED {
            for count in [0u32, 1, 2, 5] {
                let mut bytes = Vec::new();
                for _ in 0..count.max(1) {
                    for k in 0..4 {
                        let w = match (k, rng.next() % 3) {
                            (1, 0) => 0x2000_0000 + (rng.next() & 0xfff0),
                            (1, 1) => 0x2000_0001 + (rng.next() & 0xfff0),
                            _ => rng.next(),
                        };
                        bytes.extend(w.to_le_bytes());
                    }
                }
                let data = t.alloc(&bytes);
                for d in [0, data] {
                    let before = t.log_entries();
                    t.check(
                        &format!("{name} {f:#x} {d:#x} x {count}"),
                        f,
                        Conv::Cdecl,
                        &[d, count],
                        |g| {
                            descriptor::bridge::release_tagged(g, d, count);
                            Out::None
                        },
                    );
                    if d == 0 {
                        assert_eq!(t.log_entries(), before);
                    }
                }
            }
        }
    }
}

#[test]
fn array_members_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x00ab_cdef);
        let setter = t.stub_code(0x5e77_e400, true, 2, 2, Ret::Imm(0x0000_0007));
        let counter = t.stub_code(0xc0c0_0000, true, 0, 0, Ret::Imm(0x0000_0031));
        let starter = t.stub_code(0x57a7_0000, true, 0, 0, Ret::Imm(0x1000_2000));
        let class = object(&mut t, &mut rng);
        let object_ = t.alloc(
            &(0..64u32)
                .flat_map(|i| (i * 0x0101_0101).to_le_bytes())
                .collect::<Vec<_>>(),
        );
        for with in [false, true] {
            let member = t.zeros(0x40);
            t.w32(member + 0xc, class);
            t.w32(class + 0x88, 0x100);
            t.w32(member + 0x24, 0x100 + 4 * (rng.next() % 60));
            if with {
                t.w32(member + 0x3c, setter);
                t.w32(member + 0x34, counter);
                t.w32(member + 0x38, starter);
            }
            for &f in descriptor::MEMBER_SET_EXTERNAL {
                let (buffer, count) = (rng.next(), rng.next());
                t.check(
                    &format!("{name} {f:#x} with {with}"),
                    f,
                    Conv::Thiscall(member),
                    &[object_, buffer, count],
                    |g| {
                        Out::U32(descriptor::bridge::member_set_external(
                            g, member, object_, buffer, count,
                        ))
                    },
                );
            }
            for &f in descriptor::MEMBER_COUNT {
                t.check(
                    &format!("{name} {f:#x} with {with}"),
                    f,
                    Conv::Thiscall(member),
                    &[object_],
                    |g| Out::U32(descriptor::bridge::member_count(g, member, object_)),
                );
            }
            t.check(
                &format!("{name} start with {with}"),
                MEMBER_START,
                Conv::Thiscall(member),
                &[object_],
                |g| Out::U32(descriptor::bridge::member_start(g, member, object_)),
            );
        }
    }
}

/// `PClassDataMemberArrayDynamic<PSharray<PMaterial>>::getArrayStart`.
const MEMBER_START: u32 = 0x0048_c060;

// ---- batch 3: more descriptor callbacks, texture and sampler bases, small methods ----

/// A constructor's port.
type Ctor = fn(&mut G, u32) -> u32;

/// A buffer of `n` pattern bytes.
fn pattern(t: &mut Twins, rng: &mut Rng, n: usize) -> u32 {
    let bytes: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
    t.alloc(&bytes)
}

#[test]
fn more_descriptor_callbacks_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3333_aaaa);
        let this = object(&mut t, &mut rng);
        t.stub(texture::PRINTF, false, 2, 0, Ret::Imm(0x0000_0040));
        t.stub(texture::RELEASE_DATA, true, 0, 0, Ret::Imm(0x0000_0d0d));
        for &(f, spans) in descriptor::INIT_SPANS {
            assert_eq!(descriptor::spans_of(f), spans);
            for p in [0, pattern(&mut t, &mut rng, 0x40)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::construct_spans(g, p, spans))),
                );
            }
        }
        for &(f, callee) in descriptor::FORWARD {
            t.stub(callee, false, 1, 0, Ret::Imm(0x7700_0000 ^ callee));
            let p = pattern(&mut t, &mut rng, 0x20);
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U32(descriptor::bridge::forward(g, callee, p)),
            );
        }
        for &f in descriptor::LIST_HEAD {
            for p in [0, pattern(&mut t, &mut rng, 0x20)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::construct_list_head(g, p))),
                );
            }
        }
        for &f in descriptor::TEXTURE_DESTROY {
            for (flag, data) in [(0u8, 0u32), (1, 0), (0, 0x2222_0000), (0x80, 0x2222_0000)] {
                let p = pattern(&mut t, &mut rng, 0x40);
                t.write(p + 4, &[flag]);
                t.w32(p + 0x18, data);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} {flag} {data:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| Out::U8(u8::from(descriptor::bridge::destroy_texture(g, p))),
                );
                assert_eq!(
                    t.log_entries() - before,
                    u32::from(flag != 0) + u32::from(data != 0)
                );
            }
        }
    }
}

const COMMON_CTOR: u32 = 0x004c_1d70;
const CTOR_2D: u32 = 0x004c_2f90;
const CTOR_3D: u32 = 0x004c_47a0;
const CTOR_CUBE: u32 = 0x004c_6070;
const COMMON_DTOR: u32 = 0x004c_1df0;
const DTOR_THUNKS: [u32; 3] = [0x004c_3070, 0x004c_4890, 0x004c_6150];
const RELEASE_THUNK: u32 = 0x004c_79f0;
const MIP_COUNT: u32 = 0x004c_2430;
const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
const SET_DATA: u32 = 0x004c_28f0;
const SAMPLER_CTOR: u32 = 0x004b_e870;
const SAMPLER_SETTERS: [(u32, u32); 4] = [
    (0x004c_07f0, texture::SET_MIN_FILTER),
    (0x004c_0870, texture::SET_WRAP_S),
    (0x004c_08a0, texture::SET_WRAP_T),
    (0x004c_0840, texture::SET_WRAP_R),
];

#[test]
fn texture_and_sampler_bases_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7e47_0001);
        let ctors: [(u32, Ctor); 5] = [
            (COMMON_CTOR, |g, p| texture::bridge::common_ctor(g, p)),
            (CTOR_2D, |g, p| texture::bridge::ctor_2d(g, p)),
            (CTOR_3D, |g, p| texture::bridge::ctor_3d(g, p)),
            (CTOR_CUBE, |g, p| texture::bridge::ctor_cube(g, p)),
            (SAMPLER_CTOR, |g, p| texture::bridge::sampler_ctor(g, p)),
        ];
        for (f, port) in ctors {
            let p = pattern(&mut t, &mut rng, 0x40);
            t.check(
                &format!("{name} ctor {f:#x}"),
                f,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(port(g, p)),
            );
        }
        // the destructor and its thunks: the message and the release
        t.stub(texture::PRINTF, false, 2, 0, Ret::Imm(0x0000_0040));
        t.stub(texture::RELEASE_DATA, true, 0, 0, Ret::Imm(0x0000_0d0d));
        for f in std::iter::once(COMMON_DTOR).chain(DTOR_THUNKS) {
            for (flag, data) in [(0u8, 0u32), (2, 0), (0, 0x2222_0000), (1, 0x2222_0000)] {
                let p = pattern(&mut t, &mut rng, 0x40);
                t.write(p + 4, &[flag]);
                t.w32(p + 0x18, data);
                let before = t.log_entries();
                t.check(
                    &format!("{name} dtor {f:#x} {flag} {data:#x}"),
                    f,
                    Conv::Thiscall(p),
                    &[],
                    |g| {
                        texture::bridge::common_dtor(g, p);
                        Out::None
                    },
                );
                assert_eq!(
                    t.log_entries() - before,
                    u32::from(flag != 0) + u32::from(data != 0)
                );
            }
        }
        let data = pattern(&mut t, &mut rng, 0x10);
        t.check(
            &format!("{name} release thunk"),
            RELEASE_THUNK,
            Conv::Thiscall(data),
            &[],
            |g| Out::U32(texture::bridge::release_data(g, data)),
        );
        // the mip count and the max level
        for i in 0..400 {
            let mut size = || match rng.next() % 4 {
                0 => rng.next() % 4,
                1 => 1 << (rng.next() % 32),
                2 => rng.next() % 5000,
                _ => rng.next(),
            };
            let (w, h, d) = (size(), size(), size());
            t.check(
                &format!("{name} mips {w} {h} {d}"),
                MIP_COUNT,
                Conv::Cdecl,
                &[w, h, d],
                |_| Out::U32(texture::mip_count(w, h, d)),
            );
            let p = pattern(&mut t, &mut rng, 0x40);
            let flags = rng.next();
            t.w32(p + 0x14, flags);
            t.w32(p + 0x1c, w);
            t.w32(p + 0x20, h);
            if i % 3 == 0 {
                // the stored level already right
                let level = if flags & 2 != 0 {
                    texture::mip_count(w, h, 0)
                } else {
                    Mem::read(&t.b, p + 0xc)
                };
                t.w32(p + 0x10, level);
            }
            t.check(
                &format!("{name} max level {i}"),
                UPDATE_MAX_LEVEL,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(texture::bridge::update_max_level(g, p)),
            );
        }
        // the data object
        for case in 0..6 {
            let p = pattern(&mut t, &mut rng, 0x40);
            let old = match case {
                0 => 0,
                _ => pattern(&mut t, &mut rng, 0x10),
            };
            t.w32(p + 0x18, old);
            if old != 0 {
                t.write(old + 4, &[u8::from(case % 2 == 0)]);
                t.w32(old, if case < 4 { p } else { p + 4 });
            }
            let new = rng.next();
            t.check(
                &format!("{name} set data {case}"),
                SET_DATA,
                Conv::Thiscall(p),
                &[new],
                |g| Out::U32(texture::bridge::set_data(g, p, new)),
            );
        }
        // the sampler setters
        for (f, off) in SAMPLER_SETTERS {
            for flags in [0u32, 0x80, 0x7f, 0xffff_ff7f, 0x180] {
                let p = pattern(&mut t, &mut rng, 0x20);
                t.w32(p + 0x1c, flags);
                let v = rng.next();
                t.check(
                    &format!("{name} setter {f:#x} {flags:#x}"),
                    f,
                    Conv::Thiscall(p),
                    &[v],
                    |g| Out::U32(texture::bridge::sampler_set(g, p, off, v as u8)),
                );
            }
        }
    }
}

const CLEAR_WORD: u32 = 0x0042_fcc0;
const CLEAR_9: u32 = 0x0049_f990;
const WORLD_CTOR: u32 = 0x0044_29e0;
const CTOR_4A1660: u32 = 0x004a_1660;
const CLEAR_24: u32 = 0x004b_5050;
const DYNAMIC_MESH_CTOR: u32 = 0x0049_68c0;
const CTOR_47FBE0: u32 = 0x0047_fbe0;
const SCENE_INIT: u32 = 0x0043_2be0;
const SPRITE_INIT: u32 = 0x0043_4be0;
const SAMPLER_NEW: u32 = 0x004b_e860;
const CTOR_4835D0: u32 = 0x0048_35d0;
const LESS_BY_SECOND: u32 = 0x004d_3700;
const FORWARD_VISIT: u32 = 0x0047_fac0;
const INIT_ELEMENTS: u32 = 0x0048_2d70;
const FIND_ENTRY: u32 = 0x004b_d770;
const RELEASE_PASSES: u32 = 0x004b_c850;
const CLASS_ENTRY: u32 = 0x0044_37e0;
const PLATFORM_ID: u32 = 0x0046_4710;

#[test]
fn small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xba51_c000);
        let ctors: [(u32, Ctor); 6] = [
            (CLEAR_WORD, |g, p| basic::bridge::clear_word(g, p)),
            (CLEAR_9, |g, p| basic::bridge::clear_9(g, p)),
            (WORLD_CTOR, |g, p| basic::bridge::world_ctor(g, p)),
            (CTOR_4A1660, |g, p| basic::bridge::ctor_4a1660(g, p)),
            (CLEAR_24, |g, p| basic::bridge::clear_24(g, p)),
            (DYNAMIC_MESH_CTOR, |g, p| {
                basic::bridge::dynamic_mesh_ctor(g, p)
            }),
        ];
        for (f, port) in ctors {
            let p = pattern(&mut t, &mut rng, 0x40);
            t.check(
                &format!("{name} ctor {f:#x}"),
                f,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(port(g, p)),
            );
        }
        let p = pattern(&mut t, &mut rng, 0x20);
        let (a, b) = (rng.next(), rng.next());
        t.check(
            &format!("{name} 47fbe0"),
            CTOR_47FBE0,
            Conv::Thiscall(p),
            &[a, b],
            |g| Out::U32(basic::bridge::ctor_47fbe0(g, p, a, b as u8)),
        );
        t.check(
            &format!("{name} platform id"),
            PLATFORM_ID,
            Conv::Cdecl,
            &[],
            |_| Out::U32(basic::RENDERER_ID),
        );
        // initializers and forwarders with recording callees
        t.stub(basic::NODE_REGISTER, false, 0, 0, Ret::Imm(0x11));
        for f in basic::SPRITE_REGISTER {
            t.stub(f, false, 0, 0, Ret::Imm(f));
        }
        t.stub(basic::SAMPLER_D3D11_CTOR, true, 0, 0, Ret::Imm(0x5a5a));
        t.stub(basic::STREAM_CTOR, true, 1, 1, Ret::Imm(0x0051_7ea0));
        t.stub(basic::SHADER_PASS_DTOR, true, 0, 0, Ret::Imm(0x0d70));
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        let this = object(&mut t, &mut rng);
        t.check(
            &format!("{name} scene init"),
            SCENE_INIT,
            Conv::Thiscall(this),
            &[],
            |g| Out::U32(basic::bridge::scene_initialize(g)),
        );
        t.check(
            &format!("{name} sprite init"),
            SPRITE_INIT,
            Conv::Thiscall(this),
            &[],
            |g| Out::U32(basic::bridge::sprite_initialize(g)),
        );
        t.check(
            &format!("{name} sampler new"),
            SAMPLER_NEW,
            Conv::Thiscall(this),
            &[],
            |g| Out::U32(basic::bridge::sampler_d3d11_new(g, this)),
        );
        let p = pattern(&mut t, &mut rng, 0x40);
        let (a, b, c) = (rng.next(), rng.next(), rng.next());
        t.check(
            &format!("{name} 4835d0"),
            CTOR_4835D0,
            Conv::Thiscall(p),
            &[a, b, c],
            |g| Out::U32(basic::bridge::ctor_4835d0(g, p, a, b, c)),
        );
        // the visitor's virtual slot 1 (four words, popped)
        let visit = t.stub_code(0x0000_0715, true, 4, 4, Ret::Imm(0x0000_0909));
        let vtable = t.alloc(
            &[0u32, visit]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let visitor = pattern(&mut t, &mut rng, 0x10);
        t.w32(visitor, vtable);
        let args = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
        t.check(
            &format!("{name} visit"),
            FORWARD_VISIT,
            Conv::Thiscall(visitor),
            &args,
            |g| {
                Out::U32(basic::bridge::forward_visit(
                    g,
                    visitor,
                    [args[0], args[1], args[2], args[3]],
                ))
            },
        );
        // comparators and lookups
        for i in 0..300 {
            let (x, y) = (pattern(&mut t, &mut rng, 8), pattern(&mut t, &mut rng, 8));
            if i % 4 == 0 {
                let w: u32 = Mem::read(&t.b, x + 4);
                t.w32(y + 4, w);
            }
            t.check(
                &format!("{name} less {i}"),
                LESS_BY_SECOND,
                Conv::Cdecl,
                &[x, y],
                |g| Out::U32(basic::bridge::less_by_second(g, x, y)),
            );
        }
        for count in [0u32, 1, 3, 6] {
            let data = pattern(&mut t, &mut rng, 12 * 6 + 4);
            t.check(
                &format!("{name} init elements {count}"),
                INIT_ELEMENTS,
                Conv::Cdecl,
                &[data, count],
                |g| {
                    basic::bridge::init_elements(g, data, count);
                    Out::None
                },
            );
        }
        t.check(
            &format!("{name} init elements at 0"),
            INIT_ELEMENTS,
            Conv::Cdecl,
            &[0, 1],
            |g| {
                basic::bridge::init_elements(g, 0, 1);
                Out::None
            },
        );
        for case in 0..24u32 {
            let n = case % 5;
            let data = pattern(&mut t, &mut rng, 16 * 5);
            let this = pattern(&mut t, &mut rng, 0x20);
            t.w32(this + 0x14, n | if case % 3 == 0 { 0x8000_0000 } else { 0 });
            t.w32(this + 0x18, data);
            // a 0 first word (key 0 must not find it)
            t.w32(data, 0);
            let key = match case % 4 {
                0 => 0,
                1 | 2 if n > 1 => {
                    // the key also mid-entry before it: a wrong stride finds that
                    let j = 1 + rng.next() % (n - 1);
                    let k = 0x4000_0000 | rng.next();
                    t.w32(data + 16 * j, k);
                    t.w32(data + 16 * (j - 1) + 8, k);
                    k
                }
                _ => rng.next(),
            };
            t.check(
                &format!("{name} find {case}"),
                FIND_ENTRY,
                Conv::Thiscall(this),
                &[key],
                |g| Out::U32(basic::bridge::find_entry(g, this, key)),
            );
        }
        for count in [0u32, 1, 3] {
            let data = pattern(&mut t, &mut rng, 0x254 * 3);
            for d in [0, data] {
                t.check(
                    &format!("{name} release passes {d:#x} {count}"),
                    RELEASE_PASSES,
                    Conv::Cdecl,
                    &[d, count],
                    |g| {
                        basic::bridge::release_shader_passes(g, d, count);
                        Out::None
                    },
                );
            }
        }
        for case in 0..10u32 {
            let world = pattern(&mut t, &mut rng, 0x10);
            let descriptor = pattern(&mut t, &mut rng, 0x90);
            let id = case % 5;
            t.w32(world + 8, 3 | if case % 2 == 0 { 0x8000_0000 } else { 0 });
            t.w32(descriptor + 0x80, if case == 9 { 0xffff_fff0 } else { id });
            t.check(
                &format!("{name} class entry {case}"),
                CLASS_ENTRY,
                Conv::Thiscall(world),
                &[descriptor],
                |g| Out::U32(basic::bridge::class_entry(g, world, descriptor)),
            );
        }
    }
}

// ---- batch 4: descriptor constructors, type singletons, Bind with a member ----

#[test]
fn descriptor_constructors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xde5c_0001);
        t.stub(
            registry::PBASE_DESCRIPTOR,
            false,
            0,
            0,
            Ret::Imm(0x00c9_02e8),
        );
        t.stub(
            registry::GLOBAL_NAMESPACE,
            false,
            0,
            0,
            Ret::Imm(0x1234_0000),
        );
        t.stub(
            registry::DESCRIPTOR_BASE_CTOR,
            true,
            6,
            6,
            Ret::Imm(0x0bad_0001),
        );
        t.stub(registry::SET_DEFAULT_BUFFER, true, 1, 1, Ret::Imm(1));
        t.stub(registry::SET_VALIDATION_BUFFER, true, 1, 1, Ret::Imm(2));
        t.stub(registry::SET_WRITE_MASK_BUFFER, true, 1, 1, Ret::Imm(3));
        for &(f, k) in registry::DESCRIPTOR_CTOR {
            assert_eq!(registry::five_of(registry::DESCRIPTOR_CTOR, f), k);
            let this = pattern(&mut t, &mut rng, 0xa0);
            let before = t.log_entries();
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(registry::bridge::descriptor_ctor(g, this, k)),
            );
            assert_eq!(t.log_entries() - before, 6, "{f:#x}");
        }
    }
}

#[test]
fn type_singletons_and_member_binds_match_the_original() {
    for (name, mut t) in Twins::all() {
        t.stub(registry::ATEXIT, false, 1, 0, Ret::Imm(0));
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x0101));
        t.stub(
            descriptor::ADD_TO_NAMESPACE,
            true,
            0,
            0,
            Ret::Imm(0x5a5a_1234),
        );
        for &(_, [_, _, ctor, ..]) in registry::TYPE_SINGLETON {
            t.stub(ctor, true, 5, 5, Ret::Imm(0x7777));
        }
        for &(_, (_, [_, _, ctor, ..])) in registry::BIND_MEMBER {
            t.stub(ctor, true, 6, 6, Ret::Imm(0x8888));
        }
        for guard_word in [0u32, 1, 0xffff_fffe, 3] {
            for &(f, k) in registry::TYPE_SINGLETON {
                assert_eq!(registry::singleton_of(f), k);
                let guard = t.rebase(k[0]);
                t.w32(guard, guard_word);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} guard {guard_word:#x}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(registry::bridge::type_singleton(g, k)),
                );
                assert_eq!(
                    t.log_entries() - before,
                    if guard_word & 1 == 0 { 2 } else { 0 }
                );
            }
            for &(f, (d, k)) in registry::BIND_MEMBER {
                assert_eq!(registry::bind_member_of(f), (d, k));
                let guard = t.rebase(k[0]);
                t.w32(guard, guard_word);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} guard {guard_word:#x}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(registry::bridge::bind_member(g, d, k)),
                );
                assert_eq!(
                    t.log_entries() - before,
                    if guard_word & 1 == 0 { 4 } else { 2 }
                );
            }
        }
    }
}

#[test]
fn array_types_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xa77a_0001);
        t.stub(registry::ATEXIT, false, 1, 0, Ret::Imm(0));
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x0101));
        t.stub(
            descriptor::ADD_TO_NAMESPACE,
            true,
            0,
            0,
            Ret::Imm(0x5a5a_1234),
        );
        t.stub(registry::TYPE_U32, false, 0, 0, Ret::Imm(0x00c9_04d0));
        t.stub(registry::DATA_MEMBER_CTOR, true, 6, 6, Ret::Imm(0x6));
        t.stub(registry::ANNOTATION_CTOR, true, 3, 3, Ret::Imm(0x3));
        t.stub(registry::ADD_ANNOTATION, true, 1, 1, Ret::Imm(0x1));
        // seven words: the first six logged
        t.stub(registry::DATA_MEMBER_ARRAY_CTOR, true, 6, 7, Ret::Imm(0x7));
        for &(_, k) in registry::ARRAY_TYPE {
            t.stub(k.desc_ctor, true, 0, 0, Ret::Imm(0xd));
        }
        for case in 0..48u32 {
            for &(f, k) in registry::ARRAY_TYPE {
                assert_eq!(registry::array_type_of(f), k);
                let guard = t.rebase(k.guard);
                let obj = t.rebase(k.obj);
                let annotation = t.rebase(k.annotation);
                let name_global = t.rebase(k.name_global);
                let array = t.rebase(k.array_member);
                t.w32(obj + 0x90, rng.next());
                t.w32(array + 0x38, rng.next() | 1);
                t.w32(
                    guard,
                    [0, 1, 3, 7, 0xf, 5, 9, 0xffff_fff0][(case % 8) as usize],
                );
                t.w32(obj + 0x18, if case % 3 == 0 { 0x1234 } else { 0 });
                t.w32(
                    name_global,
                    if case % 5 < 2 { 0 } else { 0x00b0_0000 + case },
                );
                let list = if case % 2 == 0 {
                    annotation + 4
                } else {
                    rng.next()
                };
                t.w32(annotation + 4, list);
                let arg = if case % 7 == 6 {
                    0
                } else {
                    0x00c0_0000 + rng.next() % 0x1000
                };
                t.check(
                    &format!("{name} {f:#x} case {case}"),
                    f,
                    Conv::Cdecl,
                    &[arg],
                    |g| Out::U32(registry::bridge::array_type(g, &k, arg)),
                );
            }
        }
    }
}

// ---- batch 5: more constructInstance, element loops, type lists, cameras ----

/// A port taking `p`.
type Port = fn(&mut G, u32) -> Out;

/// A C string in both oracles.
fn cstr(t: &mut Twins, s: &str) -> u32 {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    t.alloc(&b)
}

#[test]
fn batch5_descriptor_callbacks_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb5b5_0001);
        let this = object(&mut t, &mut rng);
        let [base, _, setup] = descriptor::CAMERA_ORTHO;
        t.stub(base, true, 0, 0, Ret::Imm(0xba5e));
        t.stub(setup, true, 0, 0, Ret::Imm(0x5e7));
        let cell = t.zeros(4);
        t.stub(
            descriptor::SHADER_SOURCE_DEFAULT[1],
            true,
            0,
            0,
            Ret::Cell(cell),
        );
        let cases: [(u32, Port); 4] = [
            (0x004c_26a0, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_texture(g, p)))
            }),
            (0x004c_0190, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_sampler(g, p)))
            }),
            (0x0045_5b30, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_camera_ortho(
                    g,
                    descriptor::CAMERA_ORTHO,
                    p,
                )))
            }),
            (0x004b_0470, |g, p| {
                Out::U32(descriptor::bridge::construct_default_fields(
                    g,
                    descriptor::SHADER_SOURCE_DEFAULT,
                    p,
                ))
            }),
        ];
        for (f, port) in cases {
            for default in [0u32, 0x1234_5600, 0x0abc_def1] {
                t.w32(cell, default);
                for p in [0, pattern(&mut t, &mut rng, 0x160)] {
                    t.check(
                        &format!("{name} {f:#x}({p:#x}) default {default:#x}"),
                        f,
                        Conv::Thiscall(this),
                        &[p],
                        |g| port(g, p),
                    );
                }
            }
        }
    }
}

#[test]
fn element_loops_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xe1e3_0001);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        for count in [0u32, 1, 2, 5] {
            let dst = pattern(&mut t, &mut rng, 20 * 5 + 4);
            let src = pattern(&mut t, &mut rng, 20 * 5 + 4);
            for d in [dst, 0] {
                if d == 0 && count > 1 {
                    continue; // the next element would be at a small address
                }
                t.check(
                    &format!("{name} list heads {d:#x} {count}"),
                    0x0044_3150,
                    Conv::Cdecl,
                    &[d, count],
                    |g| {
                        basic::bridge::init_list_heads(g, d, count);
                        Out::None
                    },
                );
                t.check(
                    &format!("{name} zero16 {d:#x} {count}"),
                    0x0049_7db0,
                    Conv::Cdecl,
                    &[d, count],
                    |g| {
                        basic::bridge::zero_elements16(g, d, count);
                        Out::None
                    },
                );
                t.check(
                    &format!("{name} copy16 {d:#x} {count}"),
                    0x0049_f450,
                    Conv::Cdecl,
                    &[d, src, count],
                    |g| {
                        basic::bridge::copy_elements16(g, d, src, count);
                        Out::None
                    },
                );
                // a fresh destination: the 16-byte copy made it equal to `src`
                let d = if d == 0 {
                    0
                } else {
                    pattern(&mut t, &mut rng, 20 * 5 + 4)
                };
                t.check(
                    &format!("{name} copy12 {d:#x} {count}"),
                    0x0048_0bb0,
                    Conv::Cdecl,
                    &[d, src, count],
                    |g| {
                        basic::bridge::copy_elements10(g, d, src, count);
                        Out::None
                    },
                );
            }
            let words: Vec<u8> = (0..count.max(1))
                .flat_map(|_| {
                    let w = 0x2000_0000 | (rng.next() & 0xfff3) | (rng.next() & 1);
                    w.to_le_bytes()
                })
                .collect();
            let data = t.alloc(&words);
            for d in [data, 0] {
                t.check(
                    &format!("{name} tagged words {d:#x} {count}"),
                    0x0043_4360,
                    Conv::Cdecl,
                    &[d, count],
                    |g| {
                        basic::bridge::release_tagged_words(g, d, count);
                        Out::None
                    },
                );
            }
        }
    }
}

/// The type lists: `(find, head, name offset, link offset)`.
const TYPE_LISTS: [(u32, u32, u32, u32); 5] = [
    (0x0047_fca0, 0x00c9_85b0, 0, 4),
    (0x004c_0f10, 0x00ca_2f04, 0, 4),
    (0x0049_e2c0, 0x00c9_aef8, 4, 8),
    (0x004c_b090, 0x00ca_43a8, 4, 8),
    (0x004d_3780, 0x00ca_4e18, 4, 8),
];

#[test]
fn type_lists_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7e7e_0001);
        let words = ["alpha", "beta", "betb", "be", "", "PTexture2D", "PTexture2"];
        for (find, head, name_off, next_off) in TYPE_LISTS {
            // a fresh list of entries named by `words`
            let mut first = 0;
            for w in words.iter().rev() {
                let e = pattern(&mut t, &mut rng, 0x10);
                let s = cstr(&mut t, w);
                t.w32(e + name_off, s);
                t.w32(e + next_off, first);
                first = e;
            }
            let at = t.rebase(head);
            for list in [first, 0] {
                t.w32(at, list);
                for q in [
                    "beta",
                    "be",
                    "",
                    "PTexture2",
                    "PTexture2D",
                    "gamma",
                    "alphaX",
                ] {
                    let s = cstr(&mut t, q);
                    t.check(
                        &format!("{name} find {find:#x} {q:?}"),
                        find,
                        Conv::Cdecl,
                        &[s],
                        |g| Out::U32(basic::bridge::find_named(g, head, name_off, next_off, s)),
                    );
                }
                t.check(
                    &format!("{name} find {find:#x} null"),
                    find,
                    Conv::Cdecl,
                    &[0],
                    |g| Out::U32(basic::bridge::find_named(g, head, name_off, next_off, 0)),
                );
            }
            t.w32(at, first);
        }
        // the create callbacks and the registrations, with the lookups stubbed
        let cell = t.zeros(4);
        for (find, ..) in TYPE_LISTS {
            t.stub(find, false, 1, 0, Ret::Cell(cell));
        }
        for (f, find) in [
            (0x0047_fdf0, 0x0047_fca0),
            (0x0049_e900, 0x0049_e2c0),
            (0x004c_13f0, 0x004c_0f10),
        ] {
            for found in [0, 0x4000_1230] {
                t.w32(cell, found);
                let out = pattern(&mut t, &mut rng, 4);
                let s = rng.next();
                t.check(
                    &format!("{name} create {f:#x} {found:#x}"),
                    f,
                    Conv::Cdecl,
                    &[out, s],
                    |g| Out::U32(basic::bridge::create_result(g, find, out, s)),
                );
            }
        }
        for (f, find, head, table) in [
            (0x0047_fdb0, 0x0047_fca0, 0x00c9_85b0, None),
            (0x004c_1000, 0x004c_0f10, 0x00ca_2f04, Some(0x00ca_2f08)),
        ] {
            for (named, found) in [(0, 0), (0x1111_0000, 0x2222_0000), (0x1111_0000, 0)] {
                t.w32(cell, found);
                let obj = pattern(&mut t, &mut rng, 0x10);
                t.w32(obj, named);
                t.w32(obj + 8, rng.next() % 8);
                t.check(
                    &format!("{name} register {f:#x} {named:#x} {found:#x}"),
                    f,
                    Conv::Cdecl,
                    &[obj],
                    |g| Out::U32(basic::bridge::register_named(g, obj, find, head, table)),
                );
            }
        }
    }
}

#[test]
fn find_by_import_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1f1f_0001);
        // the compare import: a recording stub that returns a cell
        let cell = t.zeros(4);
        let compare = t.stub_code(0x00c0_c0c0, false, 2, 0, Ret::Cell(cell));
        let slot = t.rebase(0x00b0_c398);
        t.w32(slot, compare);
        let mut first = 0;
        for _ in 0..4 {
            let e = pattern(&mut t, &mut rng, 0x10);
            t.w32(e + 8, first);
            first = e;
        }
        let head = t.rebase(0x00ca_4e18);
        for list in [first, 0] {
            t.w32(head, list);
            for (q, verdict) in [(0x5555_0000, 0), (0x5555_0004, 1), (0, 0)] {
                t.w32(cell, verdict);
                t.check(
                    &format!("{name} find by {list:#x} {q:#x} {verdict}"),
                    0x004d_37e0,
                    Conv::Cdecl,
                    &[q],
                    |g| Out::U32(basic::bridge::find_named_by(g, 0x00ca_4e18, 0x00b0_c398, q)),
                );
            }
        }
    }
}

#[test]
fn cameras_events_sharrays_and_texture_data_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xca3e_0001);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        t.stub(basic::EVENT_FLAG_CTOR, true, 0, 0, Ret::Imm(0xe7));
        for f in [0x0045_5560, 0x0045_6500] {
            for (flags, block) in [
                (0, 0x2000_0000),
                (1, 0x2000_0001),
                (3, 0),
                (0xfe, 0x2000_0000),
            ] {
                let p = pattern(&mut t, &mut rng, 0xe0);
                t.w32(p + 0xd0, block);
                t.check(
                    &format!("{name} delete {f:#x} {flags} {block:#x}"),
                    f,
                    Conv::Thiscall(p),
                    &[flags],
                    |g| Out::U32(basic::bridge::camera_delete(g, p, flags, 0x00b1_0f5c)),
                );
            }
        }
        let p = pattern(&mut t, &mut rng, 0x20);
        t.check(
            &format!("{name} event owner"),
            0x0043_15e0,
            Conv::Thiscall(p),
            &[],
            |g| Out::U32(basic::bridge::event_owner_ctor(g, p)),
        );
        let p = pattern(&mut t, &mut rng, 0x20);
        t.check(
            &format!("{name} zero32"),
            0x0046_3d10,
            Conv::Thiscall(p),
            &[],
            |g| Out::U32(basic::bridge::clear_32(g, p)),
        );
        for f in [0x0043_48c0, 0x0043_4910] {
            for n in [0u32, 1, 2, 7, 0x8000_0001, 0x8000_0000] {
                let p = pattern(&mut t, &mut rng, 0x20);
                t.w32(p + 0x14, n);
                let i = rng.next() % 9;
                t.check(
                    &format!("{name} element {f:#x} {n:#x} {i}"),
                    f,
                    Conv::Thiscall(p),
                    &[i],
                    |g| Out::U32(basic::bridge::sharray_element(g, p, i)),
                );
            }
        }
        for case in 0..6 {
            let texture = pattern(&mut t, &mut rng, 0x20);
            let this = pattern(&mut t, &mut rng, 0x10);
            let old = if case == 0 {
                0
            } else {
                pattern(&mut t, &mut rng, 0x10)
            };
            t.w32(texture + 0x18, old);
            if old != 0 {
                t.write(old + 4, &[u8::from(case % 2 == 0)]);
                t.w32(old, if case < 4 { texture } else { this });
            }
            t.check(
                &format!("{name} data ctor {case}"),
                0x004c_78a0,
                Conv::Thiscall(this),
                &[texture],
                |g| Out::U32(texture::bridge::data_ctor(g, this, texture)),
            );
        }
    }
}

// ---- batch 6 ----

#[test]
fn batch6_small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb6b6_0001);
        let this = object(&mut t, &mut rng);
        for p in [0, pattern(&mut t, &mut rng, 0x40)] {
            let spans = descriptor::spans_of(0x0048_8f40);
            t.check(
                &format!("{name} 488f40 {p:#x}"),
                0x0048_8f40,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U8(u8::from(descriptor::bridge::construct_spans(g, p, spans))),
            );
            let init = descriptor::init_of(0x004a_2ec0);
            t.check(
                &format!("{name} 4a2ec0 {p:#x}"),
                0x004a_2ec0,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U8(u8::from(descriptor::bridge::construct_init(g, p, init))),
            );
        }
        for count in [0u32, 1, 4] {
            let dst = pattern(&mut t, &mut rng, 20 * 4);
            let src = pattern(&mut t, &mut rng, 20 * 4);
            for d in [dst, 0] {
                if d == 0 && count > 1 {
                    continue;
                }
                t.check(
                    &format!("{name} copy20 {d:#x} {count}"),
                    0x0044_2290,
                    Conv::Cdecl,
                    &[d, src, count],
                    |g| {
                        basic::bridge::copy_elements20(g, d, src, count);
                        Out::None
                    },
                );
            }
        }
        for n in [0u32, 1, 2, 7, 0x8000_0001, 0x8000_0000] {
            let p = pattern(&mut t, &mut rng, 0x24);
            t.w32(p + 0x18, n);
            let i = rng.next() % 9;
            t.check(
                &format!("{name} element at 18 {n:#x} {i}"),
                0x0043_4870,
                Conv::Thiscall(p),
                &[i],
                |g| Out::U32(basic::bridge::sharray_element_at(g, p + 0x18, i)),
            );
            let a = pattern(&mut t, &mut rng, 0x10);
            t.w32(a, n);
            let data = pattern(&mut t, &mut rng, 0x20);
            if n & 0x7fff_ffff > 1 {
                t.w32(a + 4, data);
            }
            for i in [0u32, 1, 6, 7, 8] {
                let v = rng.next();
                t.check(
                    &format!("{name} sharray set {n:#x} {i}"),
                    0x0048_c540,
                    Conv::Thiscall(a),
                    &[i, v],
                    |g| Out::U32(basic::bridge::sharray_set(g, a, i, v)),
                );
            }
        }
        // the data-block count
        for case in 0..20u32 {
            let mesh = pattern(&mut t, &mut rng, 0x20);
            let runs = pattern(&mut t, &mut rng, 0x40);
            let blocks = pattern(&mut t, &mut rng, 12 * 8);
            for k in 0..8 {
                if rng.next().is_multiple_of(2) {
                    t.w32(blocks + 12 * k, 0);
                }
            }
            t.w32(mesh + 8, 8 | if case % 3 == 0 { 0x8000_0000 } else { 0 });
            t.w32(mesh + 0xc, blocks);
            t.w32(mesh + 0x14, runs);
            let kind = rng.next() % 4;
            let first = rng.next() % 8;
            t.w32(runs + 16 * kind, first);
            t.w32(
                runs + 16 * kind + 4,
                (rng.next() % (9 - first)).min(8 - first),
            );
            t.check(
                &format!("{name} data blocks {case}"),
                0x0049_8770,
                Conv::Thiscall(mesh),
                &[kind],
                |g| Out::U32(basic::bridge::data_block_count(g, mesh, kind)),
            );
        }
        // the projection's depth range
        for case in 0..400u32 {
            let camera = pattern(&mut t, &mut rng, 0x40);
            let kind: u32 = match case % 4 {
                0 | 1 => 0,
                2 => 0x8000_0000,
                _ => ((rng.next() % 2000) as f32 / 7.0).to_bits(),
            };
            t.w32(camera + 0x3c, kind);
            let mut f = || -> u32 {
                match rng.next() % 6 {
                    0 => 0,
                    1 => ((rng.next() % 100_000) as f32 / 3.0).to_bits(),
                    2 => (-((rng.next() % 1000) as f32) / 7.0).to_bits(),
                    3 => [
                        0x7f80_0000u32,
                        0xff80_0000,
                        0x7fc0_0000,
                        0xffc0_1234,
                        1,
                        0x7f7f_ffff,
                    ][(rng.next() % 6) as usize],
                    _ => ((rng.next() % 5000) as f32 + 0.25).to_bits(),
                }
            };
            let (near, far) = (f(), f());
            t.check(
                &format!("{name} depth {case} {near:#x} {far:#x}"),
                0x0045_4260,
                Conv::Cdecl,
                &[camera, near, far],
                |g| {
                    basic::bridge::depth_range_zero_one(g, camera, near, far);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn batch6_objects_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb6b6_0002);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        let cell = t.zeros(4);
        t.stub(basic::MALLOC, false, 2, 0, Ret::Cell(cell));
        for block in [0, pattern(&mut t, &mut rng, 4)] {
            t.w32(cell, block);
            let p = pattern(&mut t, &mut rng, 0x20);
            let (a, b) = (rng.next(), rng.next());
            t.check(
                &format!("{name} material {block:#x}"),
                0x004d_7b10,
                Conv::Thiscall(p),
                &[a, b],
                |g| Out::U32(basic::bridge::material_ctor(g, p, a, b)),
            );
        }
        // the defaults object: a vtable whose slot 1 returns the cell
        let get = t.stub_code(0x00de_fa17, true, 0, 0, Ret::Cell(cell));
        let vtable = t.alloc(
            &[0u32, get]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let object_ = t.rebase(basic::DEFAULTS_OBJECT);
        t.w32(object_, vtable);
        t.stub(basic::APPLY_DEFAULTS, true, 2, 2, Ret::Imm(0xa));
        t.stub(basic::APPLY_MEMBER, true, 2, 2, Ret::Imm(0xb));
        for default in [0, 0x1357_9000] {
            t.w32(cell, default);
            let p = pattern(&mut t, &mut rng, 0x10);
            for (buffer, flag) in [(p + 4, 1), (p + 4, 0), (p + 8, 1), (rng.next(), 2)] {
                t.check(
                    &format!("{name} apply {default:#x} {buffer:#x} {flag}"),
                    0x0048_c6e0,
                    Conv::Thiscall(p),
                    &[buffer, flag],
                    |g| Out::U32(basic::bridge::apply_defaults(g, p, buffer, flag)),
                );
            }
        }
        t.stub(basic::ITERATOR_FIRST, true, 0, 0, Ret::Imm(0x1f));
        for empty in [true, false] {
            let list = pattern(&mut t, &mut rng, 0x24);
            if empty {
                t.w32(list + 0x1c, list + 0x1c);
            }
            let p = pattern(&mut t, &mut rng, 0x20);
            t.check(
                &format!("{name} iterator {empty}"),
                0x0046_3bf0,
                Conv::Thiscall(p),
                &[list],
                |g| Out::U32(basic::bridge::iterator_ctor(g, p, list)),
            );
        }
        // the utilities list: empty, a lower priority before, a higher one first
        t.stub(basic::SPRITE_MEMBER_CTOR, true, 0, 0, Ret::Imm(0x5));
        let head = t.rebase(basic::UTILITIES);
        for prios in [
            vec![],
            vec![0x10u32],
            vec![0x10, 0x20],
            vec![0x100],
            vec![0x10, 0xff, 0x10],
        ] {
            let mut first = 0;
            for &prio in prios.iter().rev() {
                let e = pattern(&mut t, &mut rng, 0x10);
                t.w32(e + 4, first);
                t.w32(e + 0xc, prio);
                first = e;
            }
            t.w32(head, first);
            let p = pattern(&mut t, &mut rng, 0x20);
            t.check(
                &format!("{name} sprite utility {prios:x?}"),
                0x0043_3870,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(basic::bridge::utility_sprite_ctor(g, p)),
            );
        }
        for old in [0u32, 3, 0x8000_0002] {
            for data in [0u32, 0x4000_0000] {
                for (buffer, count) in [
                    (0x4000_0000, 2),
                    (0x5000_0000, 0),
                    (0x5000_0000, 0x8000_0000),
                ] {
                    let p = pattern(&mut t, &mut rng, 0x10);
                    t.w32(p + 8, old);
                    t.w32(p + 0xc, data);
                    t.check(
                        &format!("{name} set at 8 {old:#x} {data:#x} {buffer:#x} {count:#x}"),
                        0x0048_3810,
                        Conv::Thiscall(p),
                        &[buffer, count],
                        |g| Out::U32(basic::bridge::set_external_at8(g, p, buffer, count)),
                    );
                }
            }
        }
        // PWorld's release: lists of nodes linked through +0x48
        for (n, count, data) in [
            (0u32, 0u32, 0u32),
            (1, 2, 0x4000_0000),
            (3, 0x8000_0002, 0x4000_0000),
            (2, 1, 0),
        ] {
            let world = pattern(&mut t, &mut rng, 0x10);
            let mut link = world;
            for _ in 0..n {
                let node = pattern(&mut t, &mut rng, 0x50);
                t.w32(node + 0x48, link);
                link = node + 0x48;
            }
            t.w32(world, if n == 0 && count == 0 { world } else { link });
            t.w32(world + 8, count);
            t.w32(world + 0xc, data);
            t.check(
                &format!("{name} world release {n}"),
                0x0044_2a80,
                Conv::Thiscall(world),
                &[],
                |g| {
                    basic::bridge::world_release(g, world);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn batch6_extras_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb6b6_0003);
        let this = object(&mut t, &mut rng);
        t.stub(
            descriptor::PBASE_DESCRIPTOR,
            false,
            0,
            0,
            Ret::Imm(0x00c9_02e8),
        );
        let cases: [(u32, Port); 4] = [
            (0x004c_3b50, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_texture_2d(g, p)))
            }),
            (0x004c_6be0, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_texture_cube(g, p)))
            }),
            (0x0049_ebc0, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_light(
                    g,
                    descriptor::LIGHT,
                    p,
                )))
            }),
            (0x0045_dc70, |g, p| {
                Out::U8(u8::from(descriptor::bridge::construct_asset_import(g, p)))
            }),
        ];
        for (f, port) in cases {
            for p in [0, pattern(&mut t, &mut rng, 0x40)] {
                t.check(
                    &format!("{name} {f:#x}({p:#x})"),
                    f,
                    Conv::Thiscall(this),
                    &[p],
                    |g| port(g, p),
                );
            }
        }
        // the constant buffer's destructor
        let calls = descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004a_0560);
        let cell = t.zeros(4);
        t.stub(calls[0], true, 0, 0, Ret::Cell(cell));
        t.stub(calls[1], true, 0, 0, Ret::Imm(0x11));
        t.stub(calls[2], true, 0, 0, Ret::Imm(0x2233_4455));
        for checked in [0, 1, 0x100] {
            t.w32(cell, checked);
            let p = pattern(&mut t, &mut rng, 0x10);
            t.check(
                &format!("{name} constant buffer dtor {checked:#x}"),
                0x0049_fa10,
                Conv::Thiscall(p),
                &[],
                |g| Out::U32(basic::bridge::dtor_checked(g, calls, p)),
            );
        }
        // the 0x40-byte array release
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        t.stub(basic::ELEMENT40_DTOR, true, 0, 0, Ret::Imm(0x40));
        for (count, data) in [
            (0u32, 0x4000_0000u32),
            (3, 0x4000_0000),
            (3, 0),
            (0x8000_0003, 0x4000_0000),
            (0, 0),
        ] {
            let p = pattern(&mut t, &mut rng, 0x10);
            t.w32(p, count);
            t.w32(p + 4, data);
            t.check(
                &format!("{name} release40 {count:#x} {data:#x}"),
                0x0043_3900,
                Conv::Thiscall(p),
                &[],
                |g| {
                    basic::bridge::release_array40(g, p);
                    Out::None
                },
            );
        }
        // the parameter write (memcpy stubbed)
        t.stub(basic::MEMCPY, false, 3, 0, Ret::Imm(0x0e0e));
        for case in 0..90u32 {
            let buffer = pattern(&mut t, &mut rng, 0x20);
            t.w32(buffer, 0x40 + case % 3);
            let parameter = pattern(&mut t, &mut rng, 0x10);
            let off = [0u16, 3, 4, 8, 0x30, 0x3c][(case % 6) as usize];
            let size = [0u16, 4, 8, 0x10, 0xe004][(case % 5) as usize];
            t.write(parameter + 8, &off.to_le_bytes());
            t.write(parameter + 0xa, &size.to_le_bytes());
            if case % 4 == 0 {
                // exactly full
                t.w32(buffer, u32::from(off) + u32::from(size & 0x1fff));
            }
            let src = if case % 7 == 0 { 0 } else { rng.next() };
            t.check(
                &format!("{name} write parameter {case}"),
                0x004d_57d0,
                Conv::Thiscall(buffer),
                &[parameter, src],
                |g| Out::U32(basic::bridge::write_parameter(g, buffer, parameter, src)),
            );
        }
    }
}
