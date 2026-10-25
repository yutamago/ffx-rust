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
use ffx_prender::{
    basic, class_slots as descriptor, h1_array_type, h1_bind, h1_camera, h1_small, h1_small2,
    registry, texture,
};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;
/// At the cave's start: `fld qword [esp+4]; call [esp+0xc]; ret`, and at
/// `+16` `fstp st0; ret`, for `Host::call_intrinsic` (a CRT intrinsic with
/// its argument in `ST0`).
const TRAMPOLINE_CODE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];
/// The cave's first bytes past the trampoline.
const TRAMPOLINE_LEN: u32 = 0x20;

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
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let at = self.0.rebase(CAVE);
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
    /// `ecx` and the word at this address (distinct results per `this`).
    ThisAnd(u32),
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
    /// The end of the stub area.
    cave_end: u32,
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
            cave_next: cave + TRAMPOLINE_LEN,
            cave_end: cave + CAVE_LEN,
            stubs: std::collections::HashMap::new(),
        };
        t.log = t.zeros(0x8000);
        t.write(cave, &TRAMPOLINE_CODE);
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
        self.stub_skip(sym, this, 0, n, pops, ret);
    }

    /// [`Twins::stub`] logging stack words `skip..skip + n` (for callees
    /// of more than six words).
    fn stub_skip(&mut self, sym: u32, this: bool, skip: u32, n: u32, pops: u32, ret: Ret) {
        if self.stubs.contains_key(&sym) {
            return;
        }
        let addr = self.stub_code_from(sym, this, skip, n, pops, ret);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        self.stubs.insert(sym, addr);
    }

    /// The stub of [`Twins::stub`] in the cave only (for pointers), `tag`
    /// its log tag. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: Ret) -> u32 {
        self.stub_code_from(tag, this, 0, n, pops, ret)
    }

    /// [`Twins::stub_code`] logging stack words `skip..skip + n`.
    fn stub_code_from(
        &mut self,
        tag: u32,
        this: bool,
        skip: u32,
        n: u32,
        pops: u32,
        ret: Ret,
    ) -> u32 {
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
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * (i + skip)) as u8]); // mov edx, [esp+4+4(i+skip)]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        match ret {
            Ret::Imm(_) => c.push(0xb8),                    // mov eax, imm
            Ret::Cell(_) | Ret::ThisAnd(_) => c.push(0xa1), // mov eax, [cell]
        }
        c.extend(
            match ret {
                Ret::Imm(v) | Ret::Cell(v) | Ret::ThisAnd(v) => v,
            }
            .to_le_bytes(),
        );
        if matches!(ret, Ret::ThisAnd(_)) {
            c.extend([0x21, 0xc8]); // and eax, ecx
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave_end, "cave full");
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
        // seven words: the last six logged (the first is the descriptor)
        t.stub_skip(
            registry::DATA_MEMBER_ARRAY_CTOR,
            true,
            1,
            6,
            7,
            Ret::Imm(0x7),
        );
        for &(_, k) in registry::ARRAY_TYPE {
            t.stub(k.desc_ctor, true, 0, 0, Ret::Imm(0xd));
            if k.other_fn != 0 {
                t.stub(
                    k.other_fn,
                    false,
                    0,
                    0,
                    Ret::Imm(0x00c9_7000 | (k.other_fn & 0xfff)),
                );
            }
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

#[test]
fn typed_member_binds_match_the_original() {
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
        t.stub(registry::DATA_MEMBER_CTOR, true, 6, 6, Ret::Imm(0x6));
        for &(_, (_, k)) in registry::BIND_MEMBER_TYPED {
            t.stub(k[3], false, 0, 0, Ret::Imm(0x00c9_0000 | (k[3] & 0xffff)));
        }
        for guard_word in [0u32, 1, 0xffff_fffe, 3] {
            for &(f, (d, k)) in registry::BIND_MEMBER_TYPED {
                assert_eq!(registry::bind_typed_of(f), (d, k));
                let guard = t.rebase(k[0]);
                t.w32(guard, guard_word);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} guard {guard_word:#x}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(registry::bridge::bind_member_typed(g, d, k)),
                );
                assert_eq!(
                    t.log_entries() - before,
                    if guard_word & 1 == 0 { 5 } else { 2 }
                );
            }
        }
    }
}

#[test]
fn two_member_binds_match_the_original() {
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
        t.stub(registry::DATA_MEMBER_CTOR, true, 6, 6, Ret::Imm(0x6));
        for guard_word in [0u32, 1, 2, 3, 0xffff_fff0] {
            for &(f, k) in registry::BIND_TWO_MEMBERS {
                assert_eq!(registry::bind_two_of(f), k);
                let guard = t.rebase(k.1);
                t.w32(guard, guard_word);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} guard {guard_word:#x}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(registry::bridge::bind_two_members(g, k)),
                );
                let made = u32::from(guard_word & 1 == 0) + u32::from(guard_word & 2 == 0);
                assert_eq!(t.log_entries() - before, 2 + 2 * made);
            }
        }
    }
}

/// A world (`+0` its cluster list head, `+8` the entry count, `+0xc` the
/// 20-byte class entries, each an empty list head) with `n` entries.
fn world(t: &mut Twins, rng: &mut Rng, n: u32, owned: bool) -> u32 {
    let w = pattern(t, rng, 0x10);
    t.w32(w, w);
    t.w32(w + 4, w);
    t.w32(w + 8, n | if owned { 0 } else { 0x8000_0000 });
    let entries = pattern(t, rng, 20 * n as usize + 4);
    for i in 0..n {
        let e = entries + 20 * i;
        t.w32(e, e);
        t.w32(e + 4, e);
    }
    t.w32(w + 0xc, entries);
    w
}

/// An instance list whose classes have the ids `ids` (`+0x2c` the first
/// descriptor, linked at `+0x40`; `+0x20` its node; `+0x3c` its size).
fn instance_list(t: &mut Twins, rng: &mut Rng, ids: &[u32]) -> u32 {
    let l = pattern(t, rng, 0x40);
    let mut next = 0;
    for &id in ids.iter().rev() {
        let d = pattern(t, rng, 0x84);
        t.w32(d + 0x80, id);
        t.w32(d + 0x40, next);
        next = d;
    }
    t.w32(l + 0x2c, next);
    t.w32(l + 0x20, l + 0x20);
    t.w32(l + 0x24, l + 0x20);
    t.w32(l + 0x3c, 0x100 + rng.next() % 0x100);
    l
}

/// A cluster holding `lists` (`+0x1c` their head, chained at `+0`;
/// `+0x40` its world; `+0x48` its node).
fn cluster(t: &mut Twins, rng: &mut Rng, lists: &[u32], world: u32) -> u32 {
    let c = pattern(t, rng, 0x50);
    let head = c + 0x1c;
    let mut next = head;
    for &l in lists.iter().rev() {
        t.w32(l, next);
        next = l;
    }
    t.w32(head, next);
    t.w32(c + 0x40, world);
    t.w32(c + 0x48, c + 0x48);
    t.w32(c + 0x4c, c + 0x48);
    c
}

#[test]
fn world_lists_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb8b8_0001);
        for case in 0..30u32 {
            let n = 1 + case % 5;
            let w = world(&mut t, &mut rng, n, case % 7 != 0);
            let ids: Vec<u32> = (0..case % 4).map(|_| rng.next() % (n + 2)).collect();
            let l = instance_list(&mut t, &mut rng, &ids);
            t.check(
                &format!("{name} add list {case}"),
                0x0044_3320,
                Conv::Thiscall(w),
                &[l],
                |g| {
                    basic::bridge::add_instance_list(g, w, l);
                    Out::None
                },
            );
            t.check(
                &format!("{name} remove list {case}"),
                0x0044_3c20,
                Conv::Thiscall(w),
                &[l],
                |g| Out::U32(basic::bridge::remove_instance_list(g, w, l)),
            );
            // clusters: into a world, moved to another, removed
            let lists: Vec<u32> = (0..case % 3)
                .map(|_| {
                    let ids: Vec<u32> = (0..=(rng.next() % 3))
                        .map(|_| rng.next() % (n + 1))
                        .collect();
                    instance_list(&mut t, &mut rng, &ids)
                })
                .collect();
            let c = cluster(&mut t, &mut rng, &lists, 0);
            t.check(
                &format!("{name} add cluster {case}"),
                0x0044_3250,
                Conv::Thiscall(w),
                &[c],
                |g| {
                    basic::bridge::add_cluster(g, w, c);
                    Out::None
                },
            );
            let w2 = world(&mut t, &mut rng, n, true);
            t.check(
                &format!("{name} move cluster {case}"),
                0x0044_3250,
                Conv::Thiscall(w2),
                &[c],
                |g| {
                    basic::bridge::add_cluster(g, w2, c);
                    Out::None
                },
            );
            t.check(
                &format!("{name} remove cluster elsewhere {case}"),
                0x0044_3b70,
                Conv::Thiscall(w),
                &[c],
                |g| {
                    basic::bridge::remove_cluster(g, w, c);
                    Out::None
                },
            );
            t.check(
                &format!("{name} remove cluster {case}"),
                0x0044_3b70,
                Conv::Thiscall(w2),
                &[c],
                |g| {
                    basic::bridge::remove_cluster(g, w2, c);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn batch8_small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb8b8_0002);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0xf4ee));
        t.stub(basic::SHADER_PASS_DTOR, true, 0, 0, Ret::Imm(0x0d70));
        for (count, data) in [
            (0u32, 0x4000_0000u32),
            (2, 0x4000_0000),
            (2, 0),
            (0x8000_0002, 0x4000_0000),
        ] {
            let p = pattern(&mut t, &mut rng, 0x10);
            t.w32(p, count);
            t.w32(p + 4, data);
            t.check(
                &format!("{name} passes {count:#x} {data:#x}"),
                0x004b_be40,
                Conv::Thiscall(p),
                &[],
                |g| {
                    basic::bridge::release_shader_pass_array(g, p);
                    Out::None
                },
            );
        }
        let p = pattern(&mut t, &mut rng, 0x40);
        t.check(
            &format!("{name} zero56"),
            0x0043_3700,
            Conv::Thiscall(p),
            &[],
            |g| Out::U32(basic::bridge::clear_56(g, p)),
        );
        for kind in [0u8, 1, 2, 3, 0xff] {
            for has in [false, true] {
                let target = pattern(&mut t, &mut rng, 0x10);
                let object = pattern(&mut t, &mut rng, 0x24);
                t.w32(target, if has { object } else { 0 });
                t.write(target + 4, &[kind]);
                let (w, h) = (pattern(&mut t, &mut rng, 4), pattern(&mut t, &mut rng, 4));
                t.check(
                    &format!("{name} viewport {kind} {has}"),
                    0x004c_82b0,
                    Conv::Thiscall(target),
                    &[w, h],
                    |g| Out::U32(basic::bridge::viewport_size(g, target, w, h)),
                );
            }
        }
        // the segment draw: both wrappers stubbed
        t.stub(basic::DRAW_INSTANCED, true, 4, 4, Ret::Imm(0x4444));
        t.stub(basic::DRAW_INDEXED, true, 3, 3, Ret::Imm(0x3333));
        for case in 0..12u32 {
            let ctx = pattern(&mut t, &mut rng, 0x10);
            let segment = pattern(&mut t, &mut rng, 0x20);
            let n = case % 4;
            let entries = pattern(&mut t, &mut rng, 0x40 * 4);
            for i in 0..4 {
                if !rng.next().is_multiple_of(3) || i + 1 < n {
                    t.w32(entries + 0x40 * i + 4, 0);
                }
            }
            let blocks = pattern(&mut t, &mut rng, 8);
            t.w32(blocks, n | if case % 5 == 0 { 0x8000_0000 } else { 0 });
            t.w32(blocks + 4, entries);
            let indirect = if case % 3 == 0 {
                pattern(&mut t, &mut rng, 0x10)
            } else {
                0
            };
            let extra = rng.next();
            let unused = rng.next();
            t.check(
                &format!("{name} draw segment {case}"),
                0x0049_8500,
                Conv::Stdcall,
                &[ctx, unused, segment, blocks, indirect, extra],
                |g| {
                    Out::U32(basic::bridge::draw_segment(
                        g, ctx, segment, blocks, indirect, extra,
                    ))
                },
            );
        }
    }
}

#[test]
fn batch8_extras_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb8b8_0003);
        let this = object(&mut t, &mut rng);
        for p in [0, pattern(&mut t, &mut rng, 0x30)] {
            t.check(
                &format!("{name} 3d construct {p:#x}"),
                0x004c_53b0,
                Conv::Thiscall(this),
                &[p],
                |g| Out::U8(u8::from(descriptor::bridge::construct_texture_3d(g, p))),
            );
        }
        let stored = t.stub_code(0x0057_0ed0, false, 2, 0, Ret::Imm(0x5707));
        let d = pattern(&mut t, &mut rng, 0x98);
        t.w32(d + 0x94, stored);
        let (a, b) = (rng.next(), rng.next());
        t.check(
            &format!("{name} call stored"),
            0x004d_7560,
            Conv::Thiscall(d),
            &[a, b],
            |g| Out::U32(basic::bridge::call_stored(g, d, a, b)),
        );
        t.stub(basic::CONTEXT_CTOR, true, 0, 0, Ret::Imm(0xc7));
        t.stub(basic::ATEXIT, false, 1, 0, Ret::Imm(0));
        for guard_word in [0u32, 1, 2, 0xffff_ffff] {
            let guard = t.rebase(basic::CONTEXT_GUARD);
            t.w32(guard, guard_word);
            t.check(
                &format!("{name} context {guard_word:#x}"),
                0x0046_49d0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(basic::bridge::renderer_context(g)),
            );
        }
    }
}

// ---- round 2 ----

/// Every guard word of a `Bind`'s steps, outer and inner.
fn bind_guards(steps: &[h1_bind::Step], out: &mut Vec<u32>) {
    for s in steps {
        if let h1_bind::Step::Guard(g, _, inner) = *s {
            if !out.contains(&g) {
                out.push(g);
            }
            bind_guards(inner, out);
        }
    }
}

/// Every word a `Bind`'s `Or`/`And` steps change.
fn bind_flag_words(steps: &[h1_bind::Step], out: &mut Vec<u32>) {
    for s in steps {
        match *s {
            h1_bind::Step::Guard(_, _, inner) => bind_flag_words(inner, out),
            h1_bind::Step::Or(at, _) | h1_bind::Step::And(at, _) => out.push(at),
            _ => {}
        }
    }
}

/// Every call target and class member of a `Bind`'s steps.
fn bind_callees(steps: &[h1_bind::Step], calls: &mut Vec<(u32, u32)>, classes: &mut Vec<u32>) {
    use h1_bind::{Step, Ty};
    let ty = |t: Ty, calls: &mut Vec<(u32, u32)>| match t {
        Ty::Addr(_) => {}
        Ty::Call(f) => calls.push((f, 0)),
        Ty::CallWith(f, _) => calls.push((f, 1)),
    };
    for s in steps {
        match *s {
            Step::Guard(_, _, inner) => bind_callees(inner, calls, classes),
            Step::Member { ty: t, .. } | Step::Array { ty: t, .. } => ty(t, calls),
            Step::Class { member, .. } => classes.push(member),
            Step::Call(f, arg) => calls.push((f, u32::from(arg.is_some()))),
            Step::Descriptor { ctor, .. } => calls.push((ctor, 0x100)),
            Step::AtExit(_) | Step::Store(..) | Step::Or(..) | Step::And(..) => {}
        }
    }
}

#[test]
fn multi_member_binds_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xb1d0_0002);
        // more stubs than the cave holds (and the heap is not executable):
        // they go over `FUN_00a83bf0` (16,988 bytes, no callers)
        t.cave = t.rebase(0x00a8_3bf0);
        t.cave_next = t.cave;
        t.cave_end = t.cave + 0x4000;
        t.stub(registry::ATEXIT, false, 1, 0, Ret::Imm(0));
        t.stub(descriptor::REGISTER_TYPE, false, 1, 0, Ret::Imm(0x0101));
        t.stub(
            descriptor::ADD_TO_NAMESPACE,
            true,
            0,
            0,
            Ret::Imm(0x5a5a_1234),
        );
        for ctor in [registry::DATA_MEMBER_CTOR, 0x0057_5f30, 0x0057_5370] {
            t.stub(ctor, true, 6, 6, Ret::Imm(0x6));
        }
        t.stub_skip(h1_bind::ARRAY_MEMBER_CTOR, true, 1, 6, 7, Ret::Imm(0x7));
        t.stub(h1_bind::CLASS_MEMBER_CTOR, true, 3, 3, Ret::Imm(0x3));
        let found = t.zeros(4);
        t.stub(h1_bind::FIND_MEMBER, true, 1, 1, Ret::Cell(found));
        let mut classes = vec![];
        for &(_, k) in h1_bind::BINDS {
            let mut calls = vec![];
            bind_callees(k.steps, &mut calls, &mut classes);
            for (f, kind) in calls {
                if kind == 0x100 {
                    t.stub(f, true, 0, 0, Ret::Imm(0xd));
                } else {
                    // cdecl getters: their ecx is the caller's leftover
                    t.stub(f, false, kind, 0, Ret::Imm(0x00c9_7000 | (f & 0xfff)));
                }
            }
        }
        for case in 0..12u32 {
            for &(f, k) in h1_bind::BINDS {
                assert_eq!(h1_bind::bind_of(f), k);
                let mut guards = vec![];
                bind_guards(k.steps, &mut guards);
                for g in guards {
                    let word = match case {
                        0 => 0,
                        1 => u32::MAX,
                        _ => rng.next() & rng.next(),
                    };
                    let at = t.rebase(g);
                    t.w32(at, word);
                }
                let mut flag_words = vec![];
                bind_flag_words(k.steps, &mut flag_words);
                for w in flag_words {
                    let at = t.rebase(w);
                    let v = rng.next();
                    t.w32(at, v);
                }
                // class members: an owner with a list (empty, or two nodes),
                // a name or none, and the owner knowing it or not
                for &m in &classes {
                    let m = t.rebase(m);
                    let owner = t.zeros(0x60);
                    let head = owner + 0x4c;
                    let first = t.zeros(8);
                    let second = t.zeros(8);
                    if rng.next() & 1 == 0 {
                        t.w32(head, head);
                        t.w32(head + 4, head);
                    } else {
                        t.w32(head, first);
                        t.w32(first, second);
                        t.w32(second, head);
                        t.w32(head + 4, second);
                        t.w32(second + 4, first);
                        t.w32(first + 4, head);
                    }
                    t.w32(m + 8, owner);
                    let named = if rng.next().is_multiple_of(3) {
                        0
                    } else {
                        owner
                    };
                    t.w32(m + 0xc, named);
                }
                t.w32(found, rng.next() & 1);
                t.check(
                    &format!("{name} {f:#x} case {case}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(h1_bind::bridge::bind(g, k)),
                );
            }
        }
    }
}

/// A float for the normalisation: ordinary, tiny, huge, zero, infinite or
/// a quiet NaN (signalling NaNs stay out: the emulator keeps them).
fn any_float(rng: &mut Rng) -> f32 {
    match rng.next() % 12 {
        0 => 0.0,
        1 => -0.0,
        2 => f32::INFINITY,
        3 => f32::from_bits(0x7fc0_0000 | (rng.next() & 0x003f_ffff)),
        4 => f32::from_bits(rng.next() & 0x807f_ffff), // denormal
        5 => f32::from_bits((rng.next() & 0x8fff_ffff) | 0x7000_0000), // huge
        _ => (rng.next() as i32 as f32) / 65536.0 / 1024.0,
    }
}

#[test]
fn r2b2_normalize_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4525_a001);
        for case in 0..400u32 {
            let v = t.zeros(16);
            for i in 0..3 {
                let f = if case < 300 && i == 0 && case % 7 == 0 {
                    0.0
                } else if case < 300 {
                    (rng.next() as i32 as f32) / 1_000_000.0
                } else {
                    any_float(&mut rng)
                };
                t.w32(v + 4 * i, f.to_bits());
            }
            // in place, overlapping (each store lands on the next word the
            // original reads), or into another buffer
            let out = match case % 4 {
                0 => v,
                1 => v + 4,
                _ => pattern(&mut t, &mut rng, 16),
            };
            let slot = t.a.stack_range().end - 0x100 - 4;
            let mut port_slot = 0;
            t.check(
                &format!("{name} normalize {case}"),
                0x0045_25a0,
                Conv::Cdecl,
                &[out, v],
                |g| {
                    let (r, s) = h1_small::bridge::normalize(g, out, v);
                    port_slot = s;
                    Out::U32(r)
                },
            );
            let original_slot: u32 = Mem::read(&t.a, slot);
            assert_eq!(
                original_slot, port_slot,
                "{name} normalize {case}: the argument slot"
            );
        }
    }
}

#[test]
fn r2b2_next_list_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x443e_f001);
        for case in 0..300u32 {
            // 8 class entries (list heads, depth at +0x10) and a stop entry
            let entries = t.zeros(0x14 * 9);
            let nodes = t.zeros(4 * 8);
            for i in 0..9u32 {
                let e = entries + 0x14 * i;
                let next = match rng.next() % 4 {
                    0 | 1 => e,
                    2 => entries + 0x14 * (i + 1),
                    _ => nodes + 4 * (rng.next() % 8),
                };
                t.w32(e, next);
                let depth = if i == 8 {
                    -100i32
                } else {
                    (rng.next() % 7) as i32 - 3
                };
                t.w32(e + 0x10, depth as u32);
            }
            for j in 0..8u32 {
                let to = if rng.next() & 1 == 0 {
                    entries + 0x14 * (rng.next() % 9)
                } else {
                    nodes + 4 * (rng.next() % 8)
                };
                t.w32(nodes + 4 * j, to);
            }
            let cursor = t.zeros(12);
            let entry = entries + 0x14 * (rng.next() % 8);
            t.w32(cursor, entry);
            t.w32(cursor + 4, ((rng.next() % 7) as i32 - 3) as u32);
            let node = match rng.next() % 4 {
                0 => 0,
                1 => entry,
                _ => nodes + 4 * (rng.next() % 8),
            };
            t.w32(cursor + 8, node);
            t.check(
                &format!("{name} nextList {case}"),
                0x0044_3ef0,
                Conv::Fastcall,
                &[cursor],
                |g| Out::U32(h1_small::bridge::next_list(g, cursor)),
            );
        }
    }
}

#[test]
fn r2b2_small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0434_e401);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0));
        t.stub(h1_small::ELEMENT40_DTOR, true, 0, 0, Ret::Imm(0x40));
        let found = t.zeros(4);
        t.stub(h1_small::FIND_SWITCH_TYPE, false, 1, 0, Ret::Cell(found));
        t.stub(h1_small::MIP_COUNT, false, 3, 0, Ret::Imm(0x9));
        for case in 0..64u32 {
            // PContextSwitchLights
            let this = pattern(&mut t, &mut rng, 0x18);
            t.w32(found, rng.next() & 1);
            for g in [h1_small::SWITCH_TYPES, h1_small::SWITCH_INDEX] {
                let at = t.rebase(g);
                let v = rng.next();
                t.w32(at, v);
            }
            t.check(
                &format!("{name} switch lights {case}"),
                0x004d_2fa0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(h1_small::bridge::context_switch_lights(g, this)),
            );
            // the 0x40-byte array's buffer
            let arr = t.zeros(8);
            let buffer = t.zeros(0x40 * 4);
            // case 0: an owned buffer of 0x101 elements (the count's mask)
            let n = if case == 0 { 0x101 } else { rng.next() % 5 };
            let word = n | if case != 0 && rng.next().is_multiple_of(3) {
                0x8000_0000
            } else {
                0
            };
            t.w32(arr, word);
            let old = if case != 0 && rng.next().is_multiple_of(4) {
                0
            } else {
                buffer
            };
            t.w32(arr + 4, old);
            let data = if case != 0 && rng.next().is_multiple_of(4) {
                old
            } else {
                rng.next()
            };
            let count = rng.next();
            t.check(
                &format!("{name} set buffer {case}"),
                0x0043_4e40,
                Conv::Thiscall(arr),
                &[data, count],
                |g| Out::U32(h1_small::bridge::set_buffer40(g, arr, data, count)),
            );
            // the material switch's destroyInstance
            let p = t.zeros(8);
            let (w0, w1) = (rng.next(), rng.next());
            t.w32(p, w0);
            t.w32(p + 4, w1);
            let d = rng.next();
            t.check(
                &format!("{name} free two {case}"),
                0x004d_86e0,
                Conv::Thiscall(d),
                &[p],
                |g| Out::U8(h1_small::bridge::free_two(g, p)),
            );
            // the parameter buffer's entries
            let stride = 0x10 + 4 * (rng.next() % 4);
            let n = rng.next() % 5;
            let ents = pattern(&mut t, &mut rng, (stride * 5) as usize);
            for i in 0..n {
                let ty = pattern(&mut t, &mut rng, 0x24);
                t.w32(ents + stride * i + 4, ty);
            }
            let layout = pattern(&mut t, &mut rng, 0x20);
            t.w32(layout + 0x1c, stride);
            let p = pattern(&mut t, &mut rng, 0x40);
            t.w32(p + 0x2c, layout);
            t.w32(p + 0x30, ents);
            t.w32(p + 0x3c, n);
            t.check(
                &format!("{name} mark unowned {case}"),
                0x004d_66c0,
                Conv::Cdecl,
                &[p],
                |g| Out::U32(h1_small::bridge::mark_unowned(g, p)),
            );
            // setDimensions
            let tex = pattern(&mut t, &mut rng, 0x24);
            let pick = |rng: &mut Rng| match rng.next() % 4 {
                0 => 1 << (rng.next() % 13),
                1 => rng.next() % 3000,
                2 => rng.next(),
                _ => 0,
            };
            let (w, h, f) = (pick(&mut rng), pick(&mut rng), rng.next() % 40);
            let mips = match rng.next() % 5 {
                0 => 0,
                1 => rng.next() % 14,
                2 => 31 + rng.next() % 3,
                // exactly the larger side's power of two
                3 => w.max(h).checked_ilog2().unwrap_or(0),
                _ => rng.next(),
            };
            // all four unchanged, or all but one
            if !rng.next().is_multiple_of(3) {
                let differ = rng.next() % 5;
                for (i, (at, v)) in [(0x1c, w), (0x20, h), (0, f), (0xc, mips)]
                    .into_iter()
                    .enumerate()
                {
                    let v = if differ == i as u32 { v ^ 1 } else { v };
                    t.w32(tex + at, v);
                }
            }
            if rng.next() & 1 == 0 {
                t.w32(tex + 0x10, 9);
            }
            t.check(
                &format!("{name} setDimensions {case}"),
                0x004c_40c0,
                Conv::Thiscall(tex),
                &[w, h, f, mips],
                |g| Out::U32(h1_small::bridge::set_dimensions(g, tex, w, h, f, mips)),
            );
        }
    }
}

#[test]
fn r2b3_small_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0048_c651);
        t.stub(basic::FREE, false, 1, 0, Ret::Imm(0));
        t.stub(registry::ATEXIT, false, 1, 0, Ret::Imm(0));
        let lookups = t.zeros(4);
        // each lookup gives its descriptor masked by the cell: both,
        // neither, or only the first (0x...ff0 & 0xfb0 != 0, 0x...040 & 0xfb0 == 0)
        t.stub(h1_small2::DESCRIPTOR_GET, true, 0, 0, Ret::ThisAnd(lookups));
        t.stub(h1_small2::MATERIAL_CTOR, true, 2, 2, Ret::Imm(0x1234_5678));
        for (dtor, _) in h1_small2::SEGMENT_DTORS {
            t.stub(dtor, true, 0, 0, Ret::Imm(0x55));
        }
        t.stub(h1_small2::SEMANTIC_TYPE[2], true, 5, 5, Ret::Imm(0x5));
        for (i, &f) in h1_small2::UNBIND.iter().enumerate() {
            t.stub(f, true, 1, 1, Ret::Imm(0x00ab_0000 + i as u32));
        }
        for case in 0..64u32 {
            // PMaterial's constructInstance: either lookup 0 or both found
            t.w32(lookups, [0, u32::MAX, 0xfb0][(rng.next() % 3) as usize]);
            let p = if rng.next().is_multiple_of(4) {
                0
            } else {
                rng.next()
            };
            let d = rng.next();
            t.check(
                &format!("{name} material {case}"),
                0x004d_8630,
                Conv::Thiscall(d),
                &[p],
                |g| Out::U8(h1_small2::bridge::construct_material(g, p)),
            );
            // PMeshSegment's destroyInstance
            let seg = pattern(&mut t, &mut rng, 0x40);
            if rng.next().is_multiple_of(4) {
                t.w32(seg + 0xc, 0);
            }
            let d = rng.next();
            t.check(
                &format!("{name} mesh segment {case}"),
                0x0048_8ff0,
                Conv::Thiscall(d),
                &[seg],
                |g| Out::U8(h1_small2::bridge::destroy_mesh_segment(g, seg)),
            );
            // PSharray buffer
            let arr = t.zeros(8);
            let n = [0, 1, 2, 5, 0x7fff_ffff][(rng.next() % 5) as usize];
            let word = n | if rng.next() & 1 == 0 { 0x8000_0000 } else { 0 };
            t.w32(arr, word);
            let data = pattern(&mut t, &mut rng, 8);
            let old = match rng.next() % 3 {
                0 => data,
                1 => arr + 4,
                _ => rng.next(),
            };
            t.w32(arr + 4, old);
            let data = if rng.next().is_multiple_of(5) {
                arr + 4
            } else {
                data
            };
            let count = match rng.next() % 5 {
                0 => 1,
                1 => 0x8000_0000 | rng.next(),
                2 => 0x8000_0000,
                _ => rng.next() % 9,
            };
            t.check(
                &format!("{name} sharray {case}"),
                0x0048_c650,
                Conv::Thiscall(arr),
                &[data, count],
                |g| Out::U32(h1_small2::bridge::sharray_set(g, arr, data, count)),
            );
            // the named-semantic type singleton
            let guard = t.rebase(h1_small2::SEMANTIC_TYPE[0]);
            let g = [0, 1, 2, 0xffff_fffe, 0xffff_ffff][(rng.next() % 5) as usize];
            t.w32(guard, g);
            t.check(
                &format!("{name} semantic type {case}"),
                0x0047_fae0,
                Conv::Cdecl,
                &[],
                |g| {
                    Out::U32(h1_small2::bridge::semantic_type(
                        g,
                        h1_small2::SEMANTIC_TYPE,
                    ))
                },
            );
            // a texture binding
            let binding = t.zeros(8);
            let texture = pattern(&mut t, &mut rng, 0x20);
            let kind = [0u8, 1, 2, 3, 0xff][(rng.next() % 5) as usize];
            t.write(binding + 4, &[kind, rng.next() as u8, 0, 0]);
            t.w32(
                binding,
                if rng.next().is_multiple_of(5) {
                    0
                } else {
                    texture
                },
            );
            if !rng.next().is_multiple_of(4) {
                t.w32(texture + 0x18, binding);
            }
            t.check(
                &format!("{name} binding {case}"),
                0x004c_29b0,
                Conv::Thiscall(binding),
                &[],
                |g| Out::U32(h1_small2::bridge::release_binding(g, binding)),
            );
        }
    }
}

// ---- round 3: camera projections, the class test ----

impl Twins {
    /// `code` in the cave; its address.
    fn code(&mut self, code: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + code.len() as u32 <= self.cave_end, "cave full");
        self.write(addr, code);
        self.cave_next = addr + code.len() as u32;
        addr
    }

    /// Where the original's first stack argument lands (`Oracle::call`
    /// pushes from the stack's end - 0x100).
    fn arg_slots(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32
    }

    /// [`Twins::check`] of an original that hands callees the addresses of
    /// its locals: `low` bytes below its `ebp`. The port gets a frame at
    /// the same address (filled alike first), the port oracle's own calls
    /// run below it, and the frames must agree afterwards.
    #[allow(clippy::too_many_arguments)]
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        rng: &mut Rng,
        port: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let ebp = self.arg_slots(args.len()) - 8;
        let lo = ebp - low;
        let fill: Vec<u8> = (0..low).map(|_| rng.next() as u8).collect();
        self.write(lo, &fill);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |g| port(g, lo));
        self.b.set_call_top(None);
        let mut x = vec![0; low as usize];
        let mut y = vec![0; low as usize];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        assert_eq!(x, y, "{what}: the frame");
    }
}

/// A camera-like float: mostly ordinary, sometimes [`any_float`].
fn camera_float(rng: &mut Rng, lo: f32, hi: f32) -> f32 {
    if rng.next().is_multiple_of(5) {
        any_float(rng)
    } else {
        lo + (hi - lo) * ((rng.next() >> 8) as f32 / 16_777_216.0)
    }
}

#[test]
fn r3_projection_builders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0456_b801);
        for case in 0..400u32 {
            let out = pattern(&mut t, &mut rng, 0x40);
            let fov = camera_float(&mut rng, 0.05, 3.1);
            let aspect = camera_float(&mut rng, 0.3, 3.0);
            let near = camera_float(&mut rng, 0.01, 20.0);
            let far = if case % 9 == 0 {
                near
            } else {
                camera_float(&mut rng, 10.0, 50_000.0)
            };
            let args = [
                out,
                fov.to_bits(),
                aspect.to_bits(),
                near.to_bits(),
                far.to_bits(),
            ];
            let mut slots = [0; 4];
            t.check(
                &format!("{name} perspective {case}"),
                0x0045_6b80,
                Conv::Cdecl,
                &args,
                |g| {
                    let (r, s) =
                        h1_camera::bridge::perspective(g, out, args[1], args[2], args[3], args[4]);
                    slots = s;
                    Out::U32(r)
                },
            );
            let at = t.arg_slots(args.len());
            for (k, &s) in slots.iter().enumerate() {
                let v: u32 = Mem::read(&t.a, at + 4 + 4 * k as u32);
                assert_eq!(v, s, "{name} perspective {case}: slot {}", k + 1);
            }

            let out = pattern(&mut t, &mut rng, 0x40);
            let mut f6 = [0u32; 6];
            for k in 0..6 {
                f6[k] = camera_float(&mut rng, -500.0, 500.0).to_bits();
                if k % 2 == 1 && case % 11 == 0 {
                    f6[k] = f6[k - 1];
                }
            }
            let mut args = vec![out];
            args.extend(f6);
            let mut slots = [0; 6];
            t.check(
                &format!("{name} orthographic {case}"),
                0x0045_5dc0,
                Conv::Cdecl,
                &args,
                |g| {
                    let (r, s) = h1_camera::bridge::orthographic(g, out, f6);
                    slots = s;
                    Out::U32(r)
                },
            );
            let at = t.arg_slots(args.len());
            for (k, &s) in slots.iter().enumerate() {
                let v: u32 = Mem::read(&t.a, at + 4 + 4 * k as u32);
                assert_eq!(v, s, "{name} orthographic {case}: slot {}", k + 1);
            }
        }
    }
}

#[test]
fn r3_camera_updates_match_the_original() {
    use h1_camera::camera;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0456_d101);
        for case in 0..300u32 {
            let cam = pattern(&mut t, &mut rng, 0x160);
            t.w32(
                cam + camera::NEAR,
                camera_float(&mut rng, 0.01, 20.0).to_bits(),
            );
            t.w32(
                cam + camera::FAR,
                camera_float(&mut rng, 10.0, 50_000.0).to_bits(),
            );
            t.w32(
                cam + camera::ASPECT,
                camera_float(&mut rng, 0.3, 3.0).to_bits(),
            );
            t.w32(
                cam + camera::FOV,
                camera_float(&mut rng, 0.05, 3.1).to_bits(),
            );
            t.check_framed(
                &format!("{name} update perspective {case}"),
                0x0045_6d10,
                Conv::Thiscall(cam),
                &[],
                h1_camera::PERSPECTIVE_FRAME,
                &mut rng,
                |g, frame| Out::U32(h1_camera::bridge::update_perspective(g, frame, cam)),
            );
            t.check_framed(
                &format!("{name} update orthographic {case}"),
                0x0045_5fe0,
                Conv::Thiscall(cam),
                &[],
                h1_camera::ORTHO_FRAME,
                &mut rng,
                |g, frame| Out::U32(h1_camera::bridge::update_orthographic(g, frame, cam)),
            );
            let cam = pattern(&mut t, &mut rng, 0x160);
            t.check_framed(
                &format!("{name} construct perspective {case}"),
                0x0045_6370,
                Conv::Thiscall(cam),
                &[],
                h1_camera::PERSPECTIVE_FRAME,
                &mut rng,
                |g, frame| Out::U32(h1_camera::bridge::construct_perspective(g, frame, cam)),
            );
        }
    }
}

#[test]
fn r3_find_of_class_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x04d5_f301);
        // cells: the object and class the method hands back, what it saw
        // (x, this), and whether it skips each of its two stores
        let cells = t.zeros(24);
        let mut c = vec![0x89, 0x0d];
        c.extend((cells + 12).to_le_bytes()); // mov [cells+12], ecx
        c.extend([0x8b, 0x44, 0x24, 0x04, 0xa3]);
        c.extend((cells + 8).to_le_bytes()); // mov eax, [esp+4]; mov [cells+8], eax
        for (off, cell, skip) in [(8u8, cells, cells + 16), (12, cells + 4, cells + 20)] {
            c.extend([0x83, 0x3d]);
            c.extend(skip.to_le_bytes());
            c.extend([0x00, 0x75, 0x0c]); // cmp dword [skip], 0; jne past the store
            c.extend([0x8b, 0x44, 0x24, off, 0x8b, 0x0d]);
            c.extend(cell.to_le_bytes()); // mov eax, [esp+off]; mov ecx, [cell]
            c.extend([0x89, 0x08]); // mov [eax], ecx
        }
        c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        let method = t.code(&c);
        let vtable = pattern(&mut t, &mut rng, 0x20);
        t.w32(vtable + 0x10, method);
        let this = pattern(&mut t, &mut rng, 0x10);
        t.w32(this, vtable);
        let wanted = t.rebase(h1_camera::WANTED_CLASS);
        // classes linked at +0x40 towards higher indices, the wanted class
        // or 0
        let classes: Vec<u32> = (0..6).map(|_| pattern(&mut t, &mut rng, 0x44)).collect();
        for case in 0..300u32 {
            for (i, &k) in classes.iter().enumerate() {
                let link = match rng.next() % 4 {
                    0 => 0,
                    1 => wanted,
                    _ if i + 1 < classes.len() => {
                        classes[i + 1 + (rng.next() as usize % (classes.len() - i - 1))]
                    }
                    _ => 0,
                };
                t.w32(k + 0x40, link);
            }
            let object = if case % 7 == 0 { 0 } else { rng.next() | 1 };
            let class = match rng.next() % 5 {
                0 => 0,
                1 => wanted,
                _ => classes[rng.next() as usize % classes.len()],
            };
            t.w32(cells, object);
            t.w32(cells + 4, class);
            t.w32(cells + 16, u32::from(rng.next().is_multiple_of(6)));
            t.w32(cells + 20, u32::from(rng.next().is_multiple_of(6)));
            let x = rng.next();
            t.check_framed(
                &format!("{name} find of class {case}"),
                0x004d_5f30,
                Conv::Thiscall(this),
                &[x],
                h1_camera::IS_A_FRAME,
                &mut rng,
                |g, frame| Out::U32(h1_camera::bridge::find_of_class(g, frame, this, x)),
            );
        }
    }
}

#[test]
fn r3_array_types_match_the_original() {
    use h1_array_type::{ARRAY_KINDS, Element, kind_of};
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xa77a_0003);
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
        // seven words: the last six logged (the first is the descriptor)
        t.stub_skip(
            registry::DATA_MEMBER_ARRAY_CTOR,
            true,
            1,
            6,
            7,
            Ret::Imm(0x7),
        );
        for &(_, k) in ARRAY_KINDS {
            t.stub(k.desc_ctor, true, 0, 0, Ret::Imm(0xd));
            if let Element::Call(f) = k.element {
                t.stub(f, false, 0, 0, Ret::Imm(0x00c9_7000 | (f & 0xfff)));
            }
        }
        for case in 0..64u32 {
            for &(f, k) in ARRAY_KINDS {
                assert_eq!(kind_of(f), k);
                let guard = t.rebase(k.guard);
                let obj = t.rebase(k.obj);
                let annotation = t.rebase(k.annotation);
                let name_global = t.rebase(k.name_global);
                let array = t.rebase(k.array);
                t.w32(obj + 0x90, rng.next());
                t.w32(array + 0x14, rng.next());
                t.w32(array + 0x38, rng.next() | 1);
                t.w32(array + 0x2c, rng.next());
                t.w32(
                    guard,
                    [0, 1, 3, 7, 0xf, 5, 9, 0xffff_fff0, 0xb, 0xd][(case % 10) as usize],
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
                    |g| Out::U32(h1_array_type::bridge::array_type(g, &k, arg)),
                );
            }
        }
    }
}
