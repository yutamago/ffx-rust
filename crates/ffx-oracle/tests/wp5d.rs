//! M5 finish, WP5 part 3 (wp5d, `ffx_prender::wp5d`): the `FFX.exe`
//! frontier of the no-Unicorn boot, against the original machine code:
//! `eax` and every byte written, on the image alone and over a snapshot
//! (`FFX_SNAPSHOT`). `operator new`/`operator delete` are stubs on both
//! sides (a bump allocator and a log of the freed pointers). Needs
//! `FFX_GAME_DIR`.
#![allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::wp5_string as ws;

/// The bridge's view of an oracle.
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

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

struct Twins {
    a: Oracle,
    b: Oracle,
    rng: Rng,
}

impl Twins {
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
            eprintln!("FFX_GAME_DIR is not set; skipping");
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT").filter(|s| !s.is_empty()) {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Oracle::from_snapshot(&exe, &pages).unwrap(),
                Oracle::from_snapshot(&exe, &pages).unwrap(),
            ));
        }
        out.into_iter()
            .map(|(l, mut a, mut b)| {
                a.set_max_instructions(20_000_000);
                b.set_max_instructions(20_000_000);
                let mut t = Self { a, b, rng: Rng(81) };
                t.allocator();
                (l, t)
            })
            .collect()
    }

    /// `operator new` and `operator delete` replaced in both by stubs (the
    /// game's need its heap's thread): a bump allocator over a fresh
    /// block (16-byte steps), and a log of the freed pointers (a count,
    /// then 256 words), which the diff compares.
    fn allocator(&mut self) {
        let heap = self.alloc(&vec![0x11; 0x8_0000]);
        let next = self.alloc(&heap.to_le_bytes());
        let freed = self.alloc(&[0; 4 + 4 * 256]);
        let mut c = vec![0xa1];
        c.extend(next.to_le_bytes()); // mov eax, [next]
        c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        c.extend([0x83, 0xc1, 0x0f, 0x83, 0xe1, 0xf0, 0x01, 0xc1]); // add ecx, 15; and ecx, -16; add ecx, eax
        c.extend([0x89, 0x0d]);
        c.extend(next.to_le_bytes()); // mov [next], ecx
        c.push(0xc3);
        let new = self.code(&c);
        let mut c = vec![0xa1];
        c.extend(freed.to_le_bytes()); // mov eax, [freed]
        c.extend([0x25, 0xff, 0, 0, 0]); // and eax, 0xff
        c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        c.extend([0x89, 0x0c, 0x85]);
        c.extend((freed + 4).to_le_bytes()); // mov [freed+4+eax*4], ecx
        c.extend([0xff, 0x05]);
        c.extend(freed.to_le_bytes()); // inc dword [freed]
        c.push(0xc3);
        let delete = self.code(&c);
        for (sym, to) in [(ws::OPERATOR_NEW, new), (ws::OPERATOR_DELETE, delete)] {
            let at = self.a.rebase(sym);
            let mut j = vec![0xe9];
            j.extend(to.wrapping_sub(at + 5).to_le_bytes());
            self.write(at, &j);
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    fn code(&mut self, c: &[u8]) -> u32 {
        let p = self.a.alloc_code(c).unwrap();
        assert_eq!(self.b.alloc_code(c).unwrap(), p);
        p
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let p = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), p, "both allocate alike");
        p
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    /// `operator new(n)` on both (the stub, so later frees match).
    fn new_block(&mut self, n: u32) -> u32 {
        let at = self.a.rebase(ws::OPERATOR_NEW);
        let p = self.a.call(at, Conv::Cdecl, &[n]).unwrap().eax;
        let q = self.b.call(at, Conv::Cdecl, &[n]).unwrap().eax;
        assert_eq!(p, q, "operator new alike");
        p
    }

    /// Random text of `n` characters from a small alphabet (so `find`
    /// matches), with junk after it.
    fn text(&mut self, n: u32) -> Vec<u8> {
        (0..n)
            .map(|_| b"abcab"[self.rng.below(5) as usize])
            .collect()
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: {} bytes differ, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The original at `sym` (cdecl) against `port`: `eax` and memory.
    fn run_cdecl(&mut self, what: &str, sym: u32, args: &[u32], port: impl FnOnce(&mut G) -> u32) {
        let at = self.a.rebase(sym);
        let r = self
            .a
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let p = port(&mut G(&mut self.b));
        assert_eq!(r.eax, p, "{what}: eax");
        self.same(what);
    }

    /// [`Twins::run_any_this`] with one stack word.
    fn run_any_this1(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        arg: u32,
        port: impl FnOnce(&mut G),
    ) {
        let at = self.a.rebase(sym);
        self.a
            .call(at, Conv::Thiscall(this), &[arg])
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        port(&mut G(&mut self.b));
        self.same(what);
    }

    /// The original at `sym` (thiscall on `this`, no stack words) against
    /// a void `port`: memory only.
    fn run_any_this(&mut self, what: &str, sym: u32, this: u32, port: impl FnOnce(&mut G)) {
        let at = self.a.rebase(sym);
        self.a
            .call(at, Conv::Thiscall(this), &[])
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        port(&mut G(&mut self.b));
        self.same(what);
    }

    /// The original at `sym` (thiscall on `this`) against `port`: `eax`
    /// (unless `al` only) and memory.
    fn run(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        args: &[u32],
        al: bool,
        port: impl FnOnce(&mut G) -> u32,
    ) {
        let at = self.a.rebase(sym);
        let r = self
            .a
            .call(at, Conv::Thiscall(this), args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let p = port(&mut G(&mut self.b));
        if al {
            assert_eq!(r.eax & 0xff, p & 0xff, "{what}: al");
        } else {
            assert_eq!(r.eax, p, "{what}: eax");
        }
        self.same(what);
    }
}

/// A stub at `sym` in both (a `jmp` there): `code`.
fn jmp_stub(t: &mut Twins, sym: u32, code: &[u8]) {
    let to = t.code(code);
    let at = t.a.rebase(sym);
    let mut j = vec![0xe9];
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

/// A logging stub: `inc dword [count]`, its `ecx` and (when `pops` is not
/// 0, or for `arg`) its first stack word appended to `log` (pairs), `ret
/// pops`; `eax` = `ret`.
fn log_stub(t: &mut Twins, log: u32, pops: u16, ret: u32) -> Vec<u8> {
    log_stub_arg(t, log, pops, ret, pops != 0)
}

fn log_stub_arg(_t: &mut Twins, log: u32, pops: u16, ret: u32, arg: bool) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x3f]); // and eax, 63
    c.extend([0x89, 0x0c, 0xc5]);
    c.extend((log + 4).to_le_bytes()); // mov [log+4+eax*8], ecx
    if arg {
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
    } else {
        c.extend([0x31, 0xd2, 0x90, 0x90]); // xor edx, edx
    }
    c.extend([0x89, 0x14, 0xc5]);
    c.extend((log + 8).to_le_bytes()); // mov [log+8+eax*8], edx
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend(pops.to_le_bytes());
    }
    c
}

/// [`log_stub_arg`] for a callee that takes no `ecx` (cdecl, stdcall): the
/// logged `ecx` is 0 (`xor ecx, ecx` first), as the caller's is junk.
fn log_stub_plain(t: &mut Twins, log: u32, pops: u16, ret: u32, arg: bool) -> Vec<u8> {
    let mut c = vec![0x31, 0xc9];
    c.extend(log_stub_arg(t, log, pops, ret, arg));
    c
}

/// A stub constructor: logs `ecx` (pairs, as [`log_stub_arg`]), returns
/// `ecx`.
fn ctor_stub(log: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x3f]); // and eax, 63
    c.extend([0x89, 0x0c, 0xc5]);
    c.extend((log + 4).to_le_bytes()); // mov [log+4+eax*8], ecx
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.extend([0x89, 0xc8, 0xc3]); // mov eax, ecx; ret
    c
}

/// `PPhysicsCharacterCamera`'s constructor (`0x005dff30`) and destructor
/// (`0x005e20c0`, memory only: its `eax` is the caller's) on junk
/// objects.
#[test]
fn wp5d_character_camera() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for round in 0..3u32 {
            let obj = t.alloc(&[0x6b + round as u8; 0x210]);
            let w = format!("{label} camera ctor {round}");
            t.run(&w, 0x005d_ff30, obj, &[], false, |g| {
                d::character_camera_ctor(g, obj)
            });
            let obj = t.alloc(&[0x3c + round as u8; 0x10]);
            let w = format!("{label} camera dtor {round}");
            t.run_any_this(&w, 0x005e_20c0, obj, |g| d::character_camera_dtor(g, obj));
        }
    }
}

/// The debug tools' constructors (`0x006b5520`, `0x006b5670`,
/// `0x006b5730`, `0x006b57d0`) with the text constructor a logging stub,
/// and the viewer manager's (`0x006b5870`, the real `sprintf_s`), on junk
/// objects; then every `CreateInstance` of [`d::CREATE_ONCE`] with its
/// constructor a stub: made (the stub allocator), already there (memory
/// only: the original's `eax` is a frame address or the caller's then),
/// and the allocation failing.
#[test]
fn wp5d_debug_tools() {
    use ffx_prender::wp5d as d;
    type Ctor = fn(&mut G, u32) -> u32;
    let ctors: [(u32, u32, Ctor); 5] = [
        (0x006b_5520, 0xe0, |g, o| d::tweak_ctor_6b5520(g, o)),
        (0x006b_5670, 0xf0, |g, o| d::tweak_ctor_6b5670(g, o)),
        (0x006b_5730, 0x108, |g, o| d::tweak_ctor_6b5730(g, o)),
        (0x006b_57d0, 0x214, |g, o| d::tweak_ctor_6b57d0(g, o)),
        (0x006b_5870, 0xb8, |g, o| d::viewer_manager_ctor(g, o)),
    ];
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let s = ctor_stub(log);
        jmp_stub(&mut t, d::TEXT_UTIL_CTOR, &s);
        for (k, &(at, size, port)) in ctors.iter().enumerate() {
            for round in 0..2u8 {
                let obj = t.alloc(&vec![0x50 + (k as u8) * 3 + round; size as usize]);
                t.w32(log, 0);
                let w = format!("{label} ctor {at:#x} {round}");
                t.run(&w, at, obj, &[], false, |g| port(g, obj));
            }
        }
        for &(at, global, size, ctor) in &d::CREATE_ONCE {
            let s = ctor_stub(log);
            jmp_stub(&mut t, ctor, &s);
            let g = t.a.rebase(global);
            t.w32(g, 0);
            t.w32(log, 0);
            t.run_cdecl(&format!("{label} create {at:#x}"), at, &[], |h| {
                d::create_once(h, global, size, ctor)
            });
            // already there: memory only
            let entry = t.a.rebase(at);
            t.a.call(entry, Conv::Cdecl, &[]).unwrap();
            d::create_once(&mut G(&mut t.b), global, size, ctor);
            t.same(&format!("{label} create {at:#x} again"));
        }
        // the allocation failing: operator new returning NULL
        let s = log_stub_plain(&mut t, log, 0, 0, true);
        jmp_stub(&mut t, ws::OPERATOR_NEW, &s);
        for &(at, global, size, ctor) in &d::CREATE_ONCE {
            let g = t.a.rebase(global);
            t.w32(g, 0);
            t.w32(log, 0);
            t.run_cdecl(
                &format!("{label} create {at:#x}, no memory"),
                at,
                &[],
                |h| d::create_once(h, global, size, ctor),
            );
        }
    }
}
