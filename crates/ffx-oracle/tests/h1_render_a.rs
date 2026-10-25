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
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::descriptor;

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

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
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
    /// `sym`, `ecx` and `n` stack arguments, pops `pops` words, returns
    /// `ret`. Once per address.
    fn stub(&mut self, sym: u32, n: u32, pops: u32, ret: u32) {
        if self.stubs.contains_key(&sym) {
            return;
        }
        assert!(n <= 6);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
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
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        self.stubs.insert(sym, addr);
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
            t.stub(callee, 0, 0, rng.next());
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
        t.stub(descriptor::REGISTER_TYPE, 1, 0, 0x0000_0101);
        t.stub(descriptor::ADD_TO_NAMESPACE, 0, 0, 0x5a5a_1234);
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
