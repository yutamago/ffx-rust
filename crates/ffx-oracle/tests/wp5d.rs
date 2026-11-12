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

/// A stub that appends `tag` and its first `n` stack words to `log` (a
/// count, then 8-word entries; `ecx` not logged), writes `[cells + 4]`
/// to the word its stack word `out` points at (when `out` is set and the
/// pointer not 0), returns `[cells]` and pops `pops` bytes.
fn import_stub(
    t: &mut Twins,
    log: u32,
    tag: u32,
    n: u8,
    out: Option<u8>,
    cells: u32,
    pops: u16,
) -> u32 {
    let mut c = Vec::new();
    if let Some(i) = out {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * i]); // mov eax, [esp+4+4i]
        c.extend([0x85, 0xc0, 0x74, 0x08]); // test eax, eax; jz +8
        c.extend([0x8b, 0x0d]);
        c.extend((cells + 4).to_le_bytes()); // mov ecx, [cells+4]
        c.extend([0x89, 0x08]); // mov [eax], ecx
    }
    c.push(0xa1);
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 4 + 4 * i]); // mov [eax+4+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xa1);
    c.extend(cells.to_le_bytes()); // mov eax, [cells]
    if pops == 0 {
        c.push(0xc3);
    } else {
        c.push(0xc2);
        c.extend(pops.to_le_bytes());
    }
    t.code(&c)
}

/// The import slot `slot` pointed at `code` in both.
fn set_import(t: &mut Twins, slot: u32, code: u32) {
    let s = t.a.rebase(slot);
    t.w32(s, code);
}

/// `PThreadWin32`'s start (`0x004300b0`): `CreateThread` a logging stub
/// that writes a thread id (or 0) and returns a handle (or 0),
/// `SetThreadPriority` a logging stub, `RaiseException` (the debugger's
/// thread name) a stub that only returns; the priority classes 0..3, the
/// name set or not. `eax` and memory (the logs).
#[test]
fn wp5d_thread_start() {
    use ffx_prender::wp5d as d;
    const RAISE_EXCEPTION_IAT: u32 = 0x00b0_c0e0;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let cells = t.alloc(&[0; 8]);
        let none = t.alloc(&[0; 4]);
        let create = import_stub(&mut t, log, 0xc7, 5, Some(5), cells, 0x18);
        let prio = import_stub(&mut t, log, 0x9e, 2, None, none, 8);
        let raise = t.code(&[0xc2, 0x10, 0x00]);
        set_import(&mut t, d::CREATE_THREAD_IAT, create);
        set_import(&mut t, d::SET_THREAD_PRIORITY_IAT, prio);
        set_import(&mut t, RAISE_EXCEPTION_IAT, raise);
        // the port's id word: on the stack, which the diff leaves out
        // (the original's is in its frame); the id's pointer isn't logged
        let frame = t.b.stack_range().start + 0x1000;
        for case in 0..32u32 {
            let handle = if case & 1 == 0 { 0x1234 + case } else { 0 };
            let id = if case & 2 == 0 { 0x40 + case } else { 0 };
            let name = if case & 4 == 0 { 0x00b0_e060 } else { 0 };
            let class = (case >> 3) & 3;
            t.w32(cells, handle);
            t.w32(cells + 4, id);
            let desc = t.alloc(&[0; 0x14]);
            for (off, v) in [
                (0, 0x0040_1000),
                (4, 0x8000),
                (8, 0x77 + case),
                (0xc, class),
                (0x10, name),
            ] {
                t.w32(desc + off, v);
            }
            let this = t.alloc(&[0x5a; 4]);
            t.w32(log, 0);
            let w = format!("{label} thread start case {case}");
            t.run(&w, 0x0043_00b0, this, &[desc], false, |g| {
                d::thread_start(g, this, desc, frame)
            });
        }
    }
}

/// A stub that appends `tag`, `ecx` and `n` stack words to `log` (a
/// count, then 8-word entries), returns `ret` and pops `pops` bytes.
fn wide_stub(log: u32, tag: u32, n: u8, pops: u16, ret: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
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

/// [`wide_stub`] for a callee that takes no `ecx`: the logged `ecx` is 0
/// (the caller's is junk).
fn wide_stub_plain(log: u32, tag: u32, n: u8, pops: u16, ret: u32) -> Vec<u8> {
    let mut c = vec![0x31, 0xc9]; // xor ecx, ecx
    c.extend(wide_stub(log, tag, n, pops, ret));
    c
}

/// The memory report (`0x006dc800`), its panel (`0x006dc450`) and the
/// backtrace output (`0x00630870`): the allocators and every callee
/// logging stubs (an allocation, or NULL), the global set or not. `eax`
/// (but the original's leftover when the report exists) and memory.
#[test]
fn wp5d_memory_report() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        for round in 0..2u8 {
            let obj = t.alloc(&[0x41 + round; 0x190]);
            t.w32(log, 0);
            let w = format!("{label} memory panel {round}");
            t.run(&w, 0x006d_c450, obj, &[], false, |g| {
                d::memory_panel_ctor(g, obj)
            });
        }
        for case in 0..2u32 {
            let block = if case == 0 { t.alloc(&[0x2d; 0xb0]) } else { 0 };
            let s = wide_stub_plain(log, 4, 1, 0, block);
            jmp_stub(&mut t, d::USER_MALLOC, &s);
            let s = wide_stub(log, 5, 4, 0x10, 0x99);
            jmp_stub(&mut t, d::STACK_WALKER_CTOR, &s);
            for (k, slot) in [
                d::GET_CURRENT_PROCESS_IAT,
                d::GET_CURRENT_PROCESS_ID_IAT,
                d::CREATE_MUTEX_W_IAT,
            ]
            .into_iter()
            .enumerate()
            {
                let n = if k == 2 { 3 } else { 0 };
                let c = wide_stub_plain(log, 6 + k as u32, n, 4 * u16::from(n), 0x7000 + k as u32);
                let c = t.code(&c);
                set_import(&mut t, slot, c);
            }
            t.w32(t.a.rebase(d::BACKTRACE), 0x4444);
            t.w32(log, 0);
            t.run_cdecl(
                &format!("{label} backtrace {case}"),
                0x0063_0870,
                &[],
                |h| d::backtrace_create(h),
            );
        }
        for case in 0..4u32 {
            let block = if case & 1 == 0 {
                t.alloc(&[0x33; 0x290])
            } else {
                0
            };
            let s = wide_stub_plain(log, 1, 2, 0, block);
            jmp_stub(&mut t, d::PHYRE_MALLOC, &s);
            let s = wide_stub(log, 2, 0, 0, 0x5150);
            jmp_stub(&mut t, 0x006d_c450, &s);
            let s = wide_stub_plain(log, 3, 0, 0, 0x6160 + case);
            jmp_stub(&mut t, 0x0063_0870, &s);
            let g = t.a.rebase(d::MEMORY_REPORT);
            t.w32(log, 0);
            t.w32(t.a.rebase(d::MEMORY_REPORT_MADE), 7);
            if case & 2 == 0 {
                t.w32(g, 0);
                t.run_cdecl(
                    &format!("{label} memory report {case}"),
                    0x006d_c800,
                    &[],
                    |h| d::memory_report_create(h),
                );
            } else {
                t.w32(g, 0x1234);
                let entry = t.a.rebase(0x006d_c800);
                t.a.call(entry, Conv::Cdecl, &[]).unwrap();
                d::memory_report_create(&mut G(&mut t.b));
                t.same(&format!("{label} memory report exists {case}"));
            }
        }
    }
}

/// `Sys::Thread`'s creation (`0x00679cb0`): `CreateThread` a logging
/// stub that writes an id through its last word, `RaiseException` a stub
/// that returns; the flag byte and handle combinations, `run` 0, 1, 2
/// and junk above the byte. Memory only (the original's `eax` is junk:
/// a frame address, or what the exception's dispatch left).
#[test]
fn wp5d_sys_thread_create() {
    use ffx_prender::wp5d as d;
    const RAISE_EXCEPTION_IAT: u32 = 0x00b0_c0e0;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let cells = t.alloc(&[0; 8]);
        let create = import_stub(&mut t, log, 0xc7, 6, Some(5), cells, 0x18);
        let raise = t.code(&[0xc2, 0x10, 0x00]);
        set_import(&mut t, d::CREATE_THREAD_IAT, create);
        set_import(&mut t, RAISE_EXCEPTION_IAT, raise);
        for case in 0..24u32 {
            let run = [0u32, 1, 2, 0x101, 0xff00][case as usize % 5];
            t.w32(cells, 0x900 + case);
            t.w32(cells + 4, 0x30 + case);
            let this = t.alloc(&[0x5a; 0x10]);
            t.write(this + 0xc, &[u8::from(case & 8 != 0)]);
            t.w32(this + 4, if case & 16 != 0 { 0 } else { 0x77 });
            t.w32(log, 0);
            let w = format!("{label} sys thread create case {case}");
            let at = t.a.rebase(0x0067_9cb0);
            t.a.call(
                at,
                Conv::Thiscall(this),
                &[0x4000, run, 0x1111, 0x00b4_0000],
            )
            .unwrap_or_else(|e| panic!("{w}: the original: {e}"));
            d::sys_thread_create(&mut G(&mut t.b), this, 0x4000, run);
            t.same(&w);
        }
    }
}

/// `ClusterManager`'s constructor (`0x00699ab0`): the head node maker a
/// logging stub, `CreateMutexW` a logging stub, `operator new` the stub
/// allocator or NULL. `eax` and memory.
#[test]
fn wp5d_cluster_manager_ctor() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let s = wide_stub(log, 1, 0, 0, 0x4e4e);
        jmp_stub(&mut t, d::MAP_NEW_HEAD, &s);
        let c = wide_stub_plain(log, 2, 3, 0xc, 0x6d6d);
        let c = t.code(&c);
        set_import(&mut t, d::CREATE_MUTEX_W_IAT, c);
        for round in 0..3u8 {
            if round == 2 {
                let s = log_stub_plain(&mut t, log, 0, 0, true);
                jmp_stub(&mut t, ws::OPERATOR_NEW, &s);
            }
            let obj = t.alloc(&vec![0x21 + round; 0x6420]);
            t.w32(log, 0);
            let w = format!("{label} cluster manager ctor {round}");
            t.run(&w, 0x0069_9ab0, obj, &[], false, |g| {
                d::cluster_manager_ctor(g, obj)
            });
        }
    }
}

/// The pair constructors `0x006c7ca0` and `0x006c7cb0` on junk: `eax`
/// and memory.
#[test]
fn wp5d_zero_pair() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for at in [0x006c_7ca0, 0x006c_7cb0] {
            let obj = t.alloc(&[0x9d; 0x10]);
            t.run(&format!("{label} pair {at:#x}"), at, obj, &[], false, |g| {
                d::zero_pair(g, obj)
            });
        }
    }
}

/// `std::vector<T *>::_Insert_n` (`0x00655460`): random vectors (empty,
/// NULL, spare capacity or full), every insertion point, counts 0..12
/// (into the spare room, past the end, through a reallocation), the
/// value inside the vector itself now and then. `eax`, `[out]` and
/// memory (`operator new`/`delete` the stubs).
#[test]
fn wp5d_vector_insert_n() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for round in 0..400u32 {
            let size = t.rng.below(9);
            let spare = t.rng.below(6);
            let v = t.alloc(&[0; 12]);
            let (first, cap) = if size + spare == 0 && t.rng.below(2) == 0 {
                (0, 0)
            } else {
                let cap = size + spare;
                (t.new_block((cap * 4).max(4)), cap)
            };
            if first != 0 {
                let words: Vec<u8> = (0..cap * 4).map(|i| (i * 13 + round) as u8).collect();
                t.write(first, &words);
            }
            t.w32(v, first);
            t.w32(v + 4, first + size * 4);
            t.w32(v + 8, first + cap * 4);
            let at = first + t.rng.below(size + 1) * 4;
            let count = t.rng.below(13);
            let value = if size > 0 && t.rng.below(4) == 0 {
                first + t.rng.below(size) * 4
            } else {
                t.alloc(&(0xab00_0000 | round).to_le_bytes())
            };
            let out = t.alloc(&[0xee; 4]);
            let w = format!("{label} insert_n {round} (size {size}, spare {spare}, count {count})");
            t.run(&w, 0x0065_5460, v, &[out, at, count, value], false, |g| {
                d::vector_insert_n(g, v, out, at, count, value)
            });
        }
    }
}

/// A `std::string` at `s` holding `n` random characters: inline below 16
/// (or on the heap now and then), junk in the unused bytes.
fn put_string(t: &mut Twins, s: u32, n: u32) {
    t.write(s, &[0x5a; 0x18]);
    let text = t.text(n);
    let heap = n >= 16 || t.rng.below(4) == 0;
    let (at, cap) = if heap {
        let cap = (n | 0xf).max(16) + t.rng.below(3) * 16;
        let p = t.new_block(cap + 1);
        t.write(p, &vec![0x77; (cap + 1) as usize]);
        t.w32(s, p);
        (p, cap)
    } else {
        (s, 15)
    };
    t.write(at, &text);
    t.write(at + n, &[0]);
    t.w32(s + 0x10, n);
    t.w32(s + 0x14, cap);
}

/// `std::_Destroy_range` over strings (`0x00635c40`, empty and longer
/// ranges, inline and heap strings) and the string tree's `_Erase`
/// (`0x00635c80`, random trees of 0..12 nodes under a nil head): memory
/// (the freed pointers' order included).
#[test]
fn wp5d_string_range_and_tree_erase() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for round in 0..40u32 {
            let n = t.rng.below(6);
            let first = t.alloc(&vec![0; (0x18 * n.max(1)) as usize]);
            for i in 0..n {
                let len = t.rng.below(30);
                put_string(&mut t, first + 0x18 * i, len);
            }
            let last = first + 0x18 * n;
            let w = format!("{label} destroy strings {round} ({n})");
            let at = t.a.rebase(0x0063_5c40);
            t.a.call(at, Conv::Stdcall, &[first, last])
                .unwrap_or_else(|e| panic!("{w}: {e}"));
            d::destroy_strings(&mut G(&mut t.b), first, last);
            t.same(&w);
        }
        for round in 0..40u32 {
            let nil = t.alloc(&[0; 0x28]);
            t.write(nil + 0xd, &[1]);
            let n = t.rng.below(13);
            let nodes: Vec<u32> = (0..n).map(|_| t.new_block(0x28)).collect();
            for &node in &nodes {
                t.write(node, &[0x3c; 0x28]);
                t.w32(node, nil);
                t.w32(node + 8, nil);
                t.write(node + 0xd, &[0]);
                let len = t.rng.below(30);
                put_string(&mut t, node + 0x10, len);
            }
            // a random binary tree: each node hangs under an earlier one
            let mut free: Vec<(u32, u32)> = Vec::new();
            let root = nodes.first().copied().unwrap_or(nil);
            if let Some(&r) = nodes.first() {
                free.push((r, 0));
                free.push((r, 8));
            }
            for &node in nodes.iter().skip(1) {
                let k = t.rng.below(free.len() as u32) as usize;
                let (parent, side) = free.swap_remove(k);
                t.w32(parent + side, node);
                free.push((node, 0));
                free.push((node, 8));
            }
            let head = t.alloc(&[0; 8]);
            let w = format!("{label} tree erase {round} ({n})");
            t.run_any_this1(&w, 0x0063_5c80, head, root, |g| {
                d::tree_erase_strings(g, head, root);
            });
        }
    }
}

/// A render target added to a cluster (`0x0064adf0`): the allocator a
/// logging stub returning a list (with an instance, with none) or NULL,
/// the constructor a logging stub. `eax` and memory.
#[test]
fn wp5d_cluster_add_render_target() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let s = wide_stub(log, 2, 1, 4, 0x7777);
        jmp_stub(&mut t, d::RENDER_TARGET_CTOR, &s);
        for case in 0..3u32 {
            let inst = t.alloc(&[0x19; 0x40]);
            let list = match case {
                0 => {
                    let l = t.alloc(&[0x2a; 0x40]);
                    t.w32(l + 0x34, inst);
                    l
                }
                1 => {
                    let l = t.alloc(&[0x2a; 0x40]);
                    t.w32(l + 0x34, 0);
                    l
                }
                _ => 0,
            };
            let s = wide_stub(log, 1, 3, 0xc, list);
            jmp_stub(&mut t, d::ALLOCATE_INSTANCE_LIST, &s);
            let cluster = t.alloc(&[0x63; 0x10]);
            t.w32(log, 0);
            let w = format!("{label} cluster add render target {case}");
            t.run(&w, 0x0064_adf0, cluster, &[0x5e00 + case], false, |g| {
                d::cluster_add_render_target(g, cluster, 0x5e00 + case)
            });
        }
    }
}

/// `PArray<u8>::resize` (`0x00444e90` and its thunk `0x00444e80`):
/// `AllocCopy` and `PhyreFree` logging stubs (a block, or NULL), counts
/// around `n` with bit 31 set or clear, the old buffer NULL or not, the
/// same as the new one or not. `eax` and memory.
#[test]
fn wp5d_byte_array_resize() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let s = wide_stub_plain(log, 2, 1, 0, 0);
        jmp_stub(&mut t, d::PHYRE_FREE, &s);
        let mut k = 0;
        for at in [0x0044_4e90u32, 0x0044_4e80] {
            for case in 0..48u32 {
                let n = [0u32, 3, 16, 40][(case % 4) as usize];
                let count = [0u32, 3, 16, 40][((case / 4) % 4) as usize];
                let ext = case & 16 != 0;
                let old = t.alloc(&[0x44; 4]);
                let fresh = t.alloc(&[0x55; 4]);
                let ret = match case / 32 {
                    0 => fresh,
                    _ if case & 1 == 0 => old,
                    _ => 0,
                };
                let s = wide_stub_plain(log, 1, 3, 0, ret);
                jmp_stub(&mut t, d::ALLOC_COPY, &s);
                let this = t.alloc(&[0; 8]);
                t.w32(this, count | if ext { 0x8000_0000 } else { 0 });
                t.w32(this + 4, if case % 7 == 3 { 0 } else { old });
                t.w32(log, 0);
                k += 1;
                let w = format!("{label} byte resize {at:#x} case {case} ({k})");
                t.run(&w, at, this, &[n], false, |g| {
                    d::byte_array_resize(g, this, n)
                });
            }
        }
    }
}

/// dlmalloc's `init_mparams` (`0x006ea050`) and `init_user_mstate`
/// (`0x006ea180`). The imports are stubs: the thread id and the tick
/// count from cells, `GetSystemInfo` writing a page size and a
/// granularity, `InterlockedCompareExchange` a real `lock cmpxchg`,
/// `SleepEx` logging and freeing the lock (so a lock another thread
/// holds is taken after 64 spins). The lock free, held by this thread,
/// held by another; the magic set or not. Then states over random bases
/// and sizes. `eax` and memory.
#[test]
fn wp5d_dlmalloc_init() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let cells = t.alloc(&[0; 8]);
        let lock = t.a.rebase(d::MALLOC_LOCK);
        // GetCurrentThreadId: mov eax, [cells]; ret
        let mut c = vec![0xa1];
        c.extend(cells.to_le_bytes());
        c.push(0xc3);
        let tid = t.code(&c);
        // GetTickCount: mov eax, [cells+4]; ret
        let mut c = vec![0xa1];
        c.extend((cells + 4).to_le_bytes());
        c.push(0xc3);
        let ticks = t.code(&c);
        // GetSystemInfo: mov eax, [esp+4]; [eax+4] = 0x1000; [eax+0x1c] = 0x10000; ret 4
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x40, 0x04];
        c.extend(0x1000u32.to_le_bytes());
        c.extend([0xc7, 0x40, 0x1c]);
        c.extend(0x1_0000u32.to_le_bytes());
        c.extend([0xc2, 0x04, 0x00]);
        let info = t.code(&c);
        // InterlockedCompareExchange: lock cmpxchg [ecx], edx; ret 0xc
        let c = [
            0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x54, 0x24, 0x08, 0x8b, 0x44, 0x24, 0x0c, 0xf0, 0x0f,
            0xb1, 0x11, 0xc2, 0x0c, 0x00,
        ];
        let cmpxchg = t.code(&c);
        // SleepEx: inc dword [log]; mov dword [lock], 0; ret 8
        let mut c = vec![0xff, 0x05];
        c.extend(log.to_le_bytes());
        c.extend([0xc7, 0x05]);
        c.extend(lock.to_le_bytes());
        c.extend(0u32.to_le_bytes());
        c.extend([0xc2, 0x08, 0x00]);
        let sleep = t.code(&c);
        for (slot, code) in [
            (d::GET_CURRENT_THREAD_ID_IAT, tid),
            (d::GET_TICK_COUNT_IAT, ticks),
            (d::GET_SYSTEM_INFO_IAT, info),
            (d::INTERLOCKED_CMPXCHG_IAT, cmpxchg),
            (d::SLEEP_EX_IAT, sleep),
        ] {
            set_import(&mut t, slot, code);
        }
        let frame = t.b.stack_range().start + 0x1000;
        for case in 0..12u32 {
            t.w32(cells, 0x1d0 + case);
            t.w32(cells + 4, 0x0123_4567u32.wrapping_mul(case + 1));
            let (l, owner, count) = match case % 3 {
                0 => (0, 0, 0),
                1 => (1, 0x1d0 + case, 2),
                _ => (1, 0x999, 1),
            };
            t.w32(lock, l);
            t.w32(t.a.rebase(d::MALLOC_LOCK_OWNER), owner);
            t.w32(t.a.rebase(d::MALLOC_LOCK_COUNT), count);
            let magic = if case & 4 == 0 { 0 } else { 0x5858_5858 };
            t.w32(t.a.rebase(d::MPARAMS_MAGIC), magic);
            for g in [
                d::MPARAMS_PAGE_SIZE,
                d::MPARAMS_GRANULARITY,
                d::MPARAMS_MMAP_THRESHOLD,
                d::MPARAMS_TRIM_THRESHOLD,
                d::MPARAMS_DEFAULT_MFLAGS,
            ] {
                t.w32(t.a.rebase(g), 0x3300_0000 | case);
            }
            t.w32(log, 0);
            t.run_cdecl(
                &format!("{label} init_mparams {case}"),
                0x006e_a050,
                &[],
                |h| d::init_mparams(h, frame),
            );
        }
        for round in 0..24u32 {
            let size = 0x800 + 0x40 * t.rng.below(64) + t.rng.below(8);
            let block = t.alloc(&vec![0xc5; (size + 0x40) as usize]);
            let base = block + t.rng.below(8);
            t.w32(t.a.rebase(d::MPARAMS_MAGIC), 0x0bad_f00d ^ round);
            t.w32(t.a.rebase(d::MPARAMS_DEFAULT_MFLAGS), round & 7);
            t.w32(t.a.rebase(d::MPARAMS_TRIM_THRESHOLD), 0x20_0000 + round);
            let w = format!("{label} init_user_mstate {round} (base {base:#x}, size {size:#x})");
            t.run_cdecl(&w, 0x006e_a180, &[base, size], |h| {
                d::init_user_mstate(h, base, size)
            });
        }
    }
}

/// The 0x2c node makers `0x00654f40`/`0x00654f80` (bound to
/// `ffx_system::wp5_tree`'s `buy_node`) and the string map's node maker
/// `0x006e8970` (C strings of 0..40 characters, `std::string::assign`
/// running as the original on both sides). `eax` and memory.
#[test]
fn wp5d_string_map_nodes() {
    use ffx_prender::wp5d as d;
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for at in [0x0065_4f40u32, 0x0065_4f80] {
            for round in 0..3u32 {
                let head = t.alloc(&[0x66; 0x10]);
                let tree = t.alloc(&head.to_le_bytes());
                let w = format!("{label} buy node {at:#x} {round}");
                t.run(&w, at, tree, &[], false, |g| tb::buy_node(g, tree, 0x2c));
            }
        }
        for round in 0..40u32 {
            let head = t.alloc(&[0x66; 0x10]);
            let tree = t.alloc(&head.to_le_bytes());
            let n = t.rng.below(41);
            let mut text = t.text(n);
            text.push(0);
            let p = t.alloc(&text);
            let pair = t.alloc(&[0; 8]);
            t.w32(pair, p);
            t.w32(pair + 4, 0x7100_0000 | round);
            let w = format!("{label} string map node {round} (n {n})");
            t.run(&w, 0x006e_8970, tree, &[pair], false, |g| {
                d::string_map_buy_node(g, tree, pair)
            });
        }
    }
}

/// The loader's start-up task (`0x00636430`) with every callee a
/// logging stub (the singletons' getters returning distinct objects, the
/// icon's constructor returning its block): the scene with and without a
/// remap button icon, `operator new` failing. `eax` and memory (the
/// call order and every word through the log).
#[test]
fn wp5d_loader_startup_task() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 64]);
        let this_calls = [
            0x0066_3e20u32,
            0x0066_3d40,
            0x0066_53c0,
            0x0066_50f0,
            0x0066_5200,
            0x0066_52e0,
            0x0066_4360,
            0x0066_5010,
            0x0066_54a0,
            0x0066_5580,
            0x0066_f010,
            0x006c_e100,
            0x006c_0fc0,
            0x0065_7a50,
            0x0067_c440,
        ];
        for (k, &f) in this_calls.iter().enumerate() {
            let s = wide_stub(log, 0x100 + k as u32, 0, 0, 0x5000 + k as u32);
            jmp_stub(&mut t, f, &s);
        }
        let plain = [
            (0x006d_2390u32, 0x6100u32),
            (0x006e_55d0, 0x6200),
            (0x0069_f3a0, 0x6300),
            (0x006c_14c0, 0x6400),
            (0x0067_b740, 0x6500),
            (0x006d_f3c0, 0x6600),
            (0x006d_73a0, 0x6700),
            (0x0067_b6c0, 0x6800),
            (0x0063_8e90, 0x6900),
        ];
        for &(f, r) in &plain {
            let s = wide_stub_plain(log, f, 0, 0, r);
            jmp_stub(&mut t, f, &s);
        }
        let s = wide_stub(log, 0x1ff, 1, 4, 0x76);
        jmp_stub(&mut t, 0x006d_82f0, &s);
        let s = wide_stub(log, 0x200, 3, 0xc, 0x77);
        jmp_stub(&mut t, 0x006e_78e0, &s);
        let s = wide_stub(log, 0x201, 6, 0x20, 0x78);
        jmp_stub(&mut t, d::REMAP_SET_LAYOUT, &s);
        let s = ctor_stub(log);
        jmp_stub(&mut t, 0x006e_77d0, &s);
        for case in 0..3u32 {
            if case == 2 {
                let s = log_stub_plain(&mut t, log, 0, 0, true);
                jmp_stub(&mut t, ws::OPERATOR_NEW, &s);
            }
            let scene = t.alloc(&vec![0x13; 0x10de4]);
            t.w32(scene + 0x10de0, if case == 1 { 0x4321 } else { 0 });
            t.w32(t.a.rebase(d::PHYRE_SCENE), scene);
            t.w32(t.a.rebase(d::MENU_TEXTURES), 0x9a9a + case);
            t.w32(log, 0);
            let w = format!("{label} loader start-up task {case}");
            if case == 2 {
                // the icon's constructor isn't called; setButtons and the
                // layout get ecx 0 (stubs here), the scale goes to address
                // 0 in the original: only up to there
                continue;
            }
            t.run_cdecl(&w, 0x0063_6430, &[], |h| d::loader_startup_task(h));
        }
    }
}

/// An empty `std::map` (MSVC 2012): a 0x2c-byte head node (its links
/// itself, black and nil) and the map's two words (head, size).
fn empty_map(t: &mut Twins) -> (u32, u32) {
    let head = t.new_block(0x2c);
    t.write(head, &[0x4d; 0x2c]);
    for off in [0, 4, 8] {
        t.w32(head + off, head);
    }
    t.write(head + 0xc, &[1, 1]);
    let map = t.alloc(&[0; 8]);
    t.w32(map, head);
    (map, head)
}

/// A fresh 0x2c-byte node: junk links, red, not nil, a key string of
/// `n` random characters at `key`.
fn string_node(t: &mut Twins, key: u32, n: u32) -> u32 {
    let node = t.new_block(0x30);
    t.write(node, &[0x3a; 0x30]);
    t.write(node + 0xc, &[0, 0]);
    put_string(t, node + key, n);
    node
}

/// The string maps' `_Insert_at` copies (`0x006e8a50`, `0x006d5dc0`,
/// `0x0069c6b0`, `0x0064a130`) growing trees of 40 nodes at random leaf
/// places (left or right of a node with a free child, or the root):
/// every rotation and recolouring, the head's extremes; and the three
/// `_Insert_nohint` copies (`0x006e8c80`, `0x0064a360`, `0x0069cd40`)
/// growing string maps from empty with random keys from a small
/// alphabet (equal keys freed), `leftish` 0 and 1, each through its own
/// `_Insert_at` and `--`, which run as the original on both sides here.
/// `eax`, `out` and memory after every call.
#[test]
fn wp5d_string_map_insert() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for (at, key) in [
            (0x006e_8a50u32, 0x10u32),
            (0x006d_5dc0, 0x14),
            (0x0069_c6b0, 0x10),
            (0x0064_a130, 0x10),
        ] {
            let (map, head) = empty_map(&mut t);
            let out = t.alloc(&[0; 8]);
            let mut nodes: Vec<u32> = Vec::new();
            for i in 0..40u32 {
                let node = string_node(&mut t, key, 3);
                let (place, left) = if nodes.is_empty() {
                    (head, 1)
                } else {
                    loop {
                        let p = nodes[t.rng.below(nodes.len() as u32) as usize];
                        let side = t.rng.below(2);
                        let child: u32 = Mem::read(&t.b, p + if side == 1 { 0 } else { 8 });
                        if child == head {
                            break (p, side);
                        }
                    }
                };
                // the node's links as _Insert_at expects them (the
                // callers' _Buynode): head, head, head
                for off in [0, 4, 8] {
                    t.w32(node + off, head);
                }
                let w = format!("{label} insert_at {at:#x} {i}");
                let size: u32 = Mem::read(&t.b, map + 4);
                t.run(&w, at, map, &[out, left, place, 0x7a7a, node], false, |g| {
                    let _ = size;
                    ffx_system::m5b_stl::bridge::tree_insert_at(g, map, out, left, place, node)
                });
                nodes.push(node);
            }
        }
        for &(at, ins, dec) in &d::STRING_INSERT_NOHINT {
            for leftish in [0u32, 1] {
                let (map, head) = empty_map(&mut t);
                let out = t.alloc(&[0; 8]);
                let frame = t.b.stack_range().start + 0x1000;
                for i in 0..40u32 {
                    let n = t.rng.below(4);
                    let node = string_node(&mut t, 0x10, n);
                    for off in [0, 4, 8] {
                        t.w32(node + off, head);
                    }
                    let value = node + 0x10;
                    let w = format!("{label} insert_nohint {at:#x} leftish {leftish} {i}");
                    t.run(&w, at, map, &[out, leftish, value, node], false, |g| {
                        d::string_insert_nohint(g, map, out, leftish, value, node, frame, ins, dec)
                    });
                }
            }
        }
    }
}

/// Iggy's render command trampoline (`0x00a34b00`): the routine a
/// logging stub that logs its first word (and a second, which must be
/// the caller's next word: nothing here). `eax` and memory.
#[test]
fn wp5d_render_command_trampoline() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        for round in 0..4u32 {
            let s = wide_stub_plain(log, round, 1, 0, 0x4440 + round);
            let routine = t.code(&s);
            let p = t.alloc(&[0; 8]);
            t.w32(p, 0x00c9_0000 + round);
            t.w32(p + 4, routine);
            t.w32(log, 0);
            let w = format!("{label} trampoline {round}");
            t.run_cdecl(&w, 0x00a3_4b00, &[p], |g| {
                d::render_command_trampoline(g, p)
            });
        }
    }
}

/// A tool object of `size` bytes: random bytes, label pointers (a short
/// string each) at `labels..labels + 4 * count`.
fn tool(t: &mut Twins, size: u32, labels: u32, count: u32) -> u32 {
    let bytes: Vec<u8> = (0..size).map(|_| t.rng.below(3) as u8).collect();
    let obj = t.alloc(&bytes);
    for k in 0..count {
        let mut s = format!("L{k}:").into_bytes();
        s.push(0);
        let p = t.alloc(&s);
        t.w32(obj + labels + 4 * k, p);
    }
    obj
}

/// The three tweak tools' `getLineText` (`0x006b69a0`, `0x006b6bb0`,
/// `0x006b72e0`) and the save string getter (`0x007870d0`), every line
/// (and a few past the last), the cursor on it or not, before the
/// initialisation, random flags and globals; `sprintf_s` the real one on
/// both sides, the two `GameParamConfig` getters stubs. `eax` and memory
/// (the text).
#[test]
fn wp5d_tweak_line_texts() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let viewer = t.alloc(&[0x41; 0x60]);
        t.write(viewer + 0x1c, b"bsil0000\0");
        t.write(viewer + 0x30, b"grid00\0");
        t.w32(t.a.rebase(d::VIEWER_MANAGER), viewer);
        let cells = t.alloc(&[0; 8]);
        for (f, c) in [
            (d::GET_U32_018763E0, cells),
            (d::GET_FULL_ARENA_LOCALIZATION, cells + 4),
        ] {
            let mut s = vec![0xa1];
            s.extend(c.to_le_bytes());
            s.push(0xc3);
            jmp_stub(&mut t, f, &s);
        }
        let buf = t.alloc(&[0x2e; 0x100]);
        for round in 0..6u32 {
            t.write(viewer + 0x50, &[(round & 1) as u8]);
            t.w32(t.a.rebase(d::CAMERA_MODE), round % 3);
            t.w32(t.a.rebase(d::CRC_CHECK_OFF), round & 2);
            t.w32(t.a.rebase(d::PARAM_FLAG_CC), round & 1);
            t.write(t.a.rebase(d::PARAM_FLAG_B8), &[(round & 2) as u8]);
            t.w32(cells, [u32::MAX, 0, 1][(round % 3) as usize]);
            t.w32(cells + 4, round & 1);
            // BasicDebugTweakUtil
            let obj = tool(&mut t, 0x100, 0xc4, 13);
            t.write(obj + 0x28, &[u8::from(round != 0)]);
            t.w32(obj + 0xfc, round + 2);
            for n in 0..15u32 {
                let w = format!("{label} basic debug line {n} round {round}");
                t.run(&w, 0x006b_69a0, obj, &[n, buf], false, |g| {
                    d::basic_debug_line(g, obj, n, buf)
                });
            }
            // BtlChrParamConfigTweakUitl
            let obj = tool(&mut t, 0x100, 0xb0, 17);
            t.write(obj + 0x1c, &[u8::from(round != 0)]);
            t.w32(obj + 0xf8, round * 3);
            t.w32(obj, round % 5);
            for n in 0..19u32 {
                let w = format!("{label} battle param line {n} round {round}");
                t.run(&w, 0x006b_6bb0, obj, &[n, buf], false, |g| {
                    d::battle_param_line(g, obj, n, buf)
                });
            }
            // GameParamConfigTweakUitl
            let obj = tool(&mut t, 0x100, 0xa4, 15);
            t.write(obj + 0x10, &[u8::from(round != 0)]);
            t.w32(obj + 0xe4, round * 2);
            t.w32(obj, 0x1234 + round);
            t.w32(obj + 0xc, 3600 * round + 61 * round + 7);
            for n in 0..17u32 {
                let w = format!("{label} game param line {n} round {round}");
                t.run(&w, 0x006b_72e0, obj, &[n, buf], false, |g| {
                    d::game_param_line(g, obj, n, buf)
                });
            }
        }
        for i in [0u32, 3, 7, 8, 0xffff_fffe, 0x8000_0000] {
            let w = format!("{label} save string {i:#x}");
            t.run_cdecl(&w, 0x0078_70d0, &[i], |g| d::save_string(g, i));
        }
    }
}

/// GDraw's frame reset (`0x00a37fe0`): a junk context with a device
/// context whose vtable slot 110 is a logging stub, two draw lists (also
/// the same list twice). `eax` and memory.
#[test]
fn wp5d_gdraw_reset_frame() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let s = wide_stub_plain(log, 0x6e, 1, 4, 0x5150);
        let clear = t.code(&s);
        let vt = t.alloc(&[0; 0x200]);
        t.w32(vt + 0x1b8, clear);
        for round in 0..3u32 {
            let bytes: Vec<u8> = (0..0xad0u32).map(|i| (i * 7 + round) as u8).collect();
            let ctx = t.alloc(&bytes);
            let dc = t.alloc(&[0; 8]);
            t.w32(dc, vt);
            t.w32(ctx + 4, dc);
            let l1 = t.alloc(
                &(0..0x1b0u32)
                    .map(|i| (i * 3 + round) as u8)
                    .collect::<Vec<_>>(),
            );
            let l2 = t.alloc(
                &(0..0x1b0u32)
                    .map(|i| (i * 5 + round) as u8)
                    .collect::<Vec<_>>(),
            );
            t.w32(ctx + 0xab0, l1);
            t.w32(ctx + 0xab4, if round == 2 { l1 } else { l2 });
            t.w32(t.a.rebase(d::GDRAW_CONTEXT), ctx);
            t.w32(log, 0);
            let w = format!("{label} gdraw reset frame {round}");
            t.run_cdecl(&w, 0x00a3_7fe0, &[], |g| d::gdraw_reset_frame(g));
        }
    }
}

/// The name list's node maker (`0x00680490`): the node maker and
/// `std::string`'s assign run as the original on both sides; values of
/// 0..40 characters (inline and heap), `next`/`prev` NULL or not. `eax`
/// and memory.
#[test]
fn wp5d_name_list_buy_node() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        for round in 0..30u32 {
            let value = t.alloc(&[0; 0x1c]);
            let n = t.rng.below(41);
            put_string(&mut t, value, n);
            t.w32(value + 0x18, 0x5100 + round);
            let (next, prev) = if round % 3 == 0 {
                (0, 0)
            } else {
                (t.alloc(&[0; 0x24]), t.alloc(&[0; 0x24]))
            };
            let list = t.alloc(&[0; 8]);
            let w = format!("{label} name list node {round} (n {n})");
            t.run(&w, 0x0068_0490, list, &[next, prev, value], false, |g| {
                d::name_list_buy_node(g, next, prev, value)
            });
        }
    }
}

/// A cursor stub (thiscall on the cursor, no stack words): logs `tag`
/// and the cursor's list, first object and descriptor (`+0x14..+0x1c`),
/// then writes `[cells]` to `+0xc` (found), `[cells+4]` to `+0x10`
/// (offset) and `[cells+8]` to `+0` (base).
fn cursor_stub(log: u32, tag: u32, cells: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    for (i, off) in [0x14u8, 0x18, 0x1c].into_iter().enumerate() {
        c.extend([0x8b, 0x51, off]); // mov edx, [ecx+off]
        c.extend([0x89, 0x50, 4 + 4 * i as u8]); // mov [eax+4+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    for (cell, off) in [(cells, 0xcu8), (cells + 4, 0x10), (cells + 8, 0)] {
        c.extend([0x8b, 0x15]);
        c.extend(cell.to_le_bytes()); // mov edx, [cell]
        c.extend([0x89, 0x51, off]); // mov [ecx+off], edx
    }
    c.push(0xc3);
    c
}

/// A stub for `setTextureChecked` (thiscall `(p, &desc)`, `ret 8`): logs
/// `tag`, `ecx`, `p` and the descriptor's four words.
fn desc_stub(log: u32, tag: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // mov edx, [esp+4]; mov [eax+8], edx
    c.extend([0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]
    for i in 0..4u8 {
        c.extend([0x8b, 0x51, 4 * i]); // mov edx, [ecx+4i]
        c.extend([0x89, 0x50, 0xc + 4 * i]); // mov [eax+0xc+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.extend([0xc2, 0x08, 0x00]);
    c
}

/// The remap button icon's button material (`0x006e8390`) with every
/// callee a stub (logging stubs; the cursors write their found flag,
/// offset and base; the parameter lookup returns a parameter with an
/// offset and size that fit, or don't); the cluster loaded or not, the
/// texture found or not, the material there already or made. `eax` and
/// memory (`memcpy` the real one into the material's buffer).
#[test]
fn wp5d_button_material() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let cells = t.alloc(&[0; 0x20]);
        let effect = t.alloc(&[0x61; 0x10]);
        let fx_table = t.alloc(&[0; 0x40]);
        t.w32(fx_table + 0x18, effect);
        let scene = t.alloc(&[0x13; 0xfa90]);
        let effects = t.alloc(&[0x26; 0x40]);
        t.w32(scene + 0xfa84, effects);
        t.w32(t.a.rebase(d::PHYRE_SCENE), scene);
        let buffer = t.alloc(&[0x5a; 0x60]);
        t.w32(buffer, 0x40);
        let mat = t.alloc(&[0x22; 0x10]);
        t.w32(mat, 0x7e7e);
        t.w32(mat + 4, buffer);
        let param = t.alloc(&[0; 0x10]);
        for &(f, tag, n, pops, ret, this) in &[
            (d::CLUSTER_MANAGER_GET, 1u32, 0u8, 0u16, 0xc0c0u32, false),
            (d::GET_APPLICATION, 3, 0, 0, 0xa9a9, false),
            (d::FIXUP_CLUSTERS, 4, 2, 0, 0, false),
            (d::CREATE_MATERIAL, 5, 3, 0xc, mat, true),
            (d::LINEAR_SAMPLER, 6, 1, 4, 0x5a5a, true),
            (d::PARAM_BY_NAME, 7, 1, 4, param, true),
        ] {
            let s = if this {
                wide_stub(log, tag, n, pops, ret)
            } else {
                wide_stub_plain(log, tag, n, pops, ret)
            };
            jmp_stub(&mut t, f, &s);
        }
        let s = cursor_stub(log, 8, cells);
        jmp_stub(&mut t, d::CLUSTER_ITER_SEEK, &s);
        let s = cursor_stub(log, 9, cells + 0xc);
        jmp_stub(&mut t, d::SCENE_CURSOR_SEEK4, &s);
        let s = desc_stub(log, 10);
        jmp_stub(&mut t, d::SET_TEXTURE_CHECKED, &s);
        for case in 0..16u32 {
            let cluster = if case & 1 == 0 {
                let c = t.alloc(&[0; 0x40]);
                let list = c + 0x1c;
                // an empty list (the head points at itself) or one object
                t.w32(list, if case & 8 == 0 { 0x3333 } else { list });
                c
            } else {
                0
            };
            let s = wide_stub(log, 2, 1, 4, cluster);
            jmp_stub(&mut t, d::CLUSTER_LOAD, &s);
            t.w32(
                effects + 0x1c,
                if case & 8 == 0 {
                    0x4444
                } else {
                    effects + 0x1c
                },
            );
            t.w32(cells, u32::from(case & 2 == 0));
            t.w32(cells + 4, 0x10);
            t.w32(cells + 8, 0x0100_0000);
            t.w32(cells + 0xc, 1);
            t.w32(cells + 0x10, 8);
            t.w32(cells + 0x14, fx_table + 0x10);
            // the parameter: offset 8 or 2 (too small), size 16 or 0x3f
            let off = if case & 4 == 0 { 8 } else { 2 };
            let size = if case < 12 { 16 } else { 0x3f } | 0xe000;
            t.write(param + 8, &(off as u16).to_le_bytes());
            t.write(param + 0xa, &(size as u16).to_le_bytes());
            let slots = t.alloc(&[0; 16]);
            if case % 3 == 0 {
                t.w32(slots + 8, mat);
                t.w32(slots + 12, effect);
            }
            let name = t.alloc(b"button\0");
            t.w32(log, 0);
            let (cl, tx, ma, inst) = (slots, slots + 4, slots + 8, slots + 12);
            let frame = t.b.stack_range().start + 0x1000;
            let w = format!("{label} button material case {case}");
            let at = t.a.rebase(0x006e_8390);
            let r =
                t.a.call(at, Conv::Thiscall(0x1234), &[name, cl, tx, ma, inst])
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let p = d::button_material(&mut G(&mut t.b), frame, name, cl, tx, ma, inst);
            assert_eq!(r.eax, p, "{w}: eax");
            t.same(&w);
        }
    }
}

/// A stub that logs its eight stack words (32-byte entries after a
/// count), returns `ret` and pops `pops` bytes.
fn words8_stub(log: u32, pops: u16, ret: u32) -> Vec<u8> {
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    for i in 0..8u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 4 * i]); // mov [eax+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.push(0xc2);
    c.extend(pops.to_le_bytes());
    c
}

/// `RemapButtonIcon`'s layout (`0x006e8140`): `buildQuad` a stub logging
/// all eight words; floats of both signs. `eax` and memory.
#[test]
fn wp5d_remap_set_layout() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        for round in 0..8u32 {
            let s = words8_stub(log, 0x20, 0x9900 + round);
            jmp_stub(&mut t, d::BUILD_QUAD, &s);
            let this = t.alloc(&[0x47; 0x70]);
            let f: Vec<u32> = (0..8)
                .map(|k| (k as f32 * 0.25 - round as f32 * 3.5).to_bits())
                .collect();
            let a = [f[0], f[1], f[2], f[3]];
            let b = [f[4], f[5], f[6], f[7]];
            t.w32(log, 0);
            let w = format!("{label} remap layout {round}");
            t.run(&w, 0x006e_8140, this, &f, false, |g| {
                d::remap_set_layout(g, this, a, b)
            });
        }
    }
}

/// The memory report's map-unload line (`0x006dcdc0`): the report off
/// (either byte 0; memory only, the original's `eax` is its cookie word
/// then), on with an empty and a set map name. `eax` (when on) and
/// memory (the buffer is the frame's: the stack, which the diff leaves
/// out; the real `sprintf_s` on both sides).
#[test]
fn wp5d_report_map_unload() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let frame = t.b.stack_range().start + 0x1000;
        for case in 0..5u32 {
            let this = t.alloc(&[0; 0x110]);
            let (a, b) = [(0u8, 1u8), (1, 0), (1, 1), (1, 1), (1, 1)][case as usize];
            t.write(this, &[a, b]);
            if case >= 3 {
                t.write(this + 0xc, b"bsil0300\0");
            }
            if case == 4 {
                t.write(this + 0xc + 0xff, &[7]);
            }
            let w = format!("{label} map unload {case}");
            let at = t.a.rebase(0x006d_cdc0);
            let r =
                t.a.call(at, Conv::Thiscall(this), &[])
                    .unwrap_or_else(|e| panic!("{w}: {e}"));
            let p = d::report_map_unload(&mut G(&mut t.b), this, frame);
            if a != 0 && b != 0 {
                assert_eq!(r.eax, p, "{w}: eax");
            }
            t.same(&w);
        }
    }
}

/// The Phyre tree's release (`0x00636970`): `remove` a stub that pops the
/// first node (writes its `+0` link to the tree's `+0xc`) and logs it,
/// `PhyreFree` and the free list's reset logging stubs; trees of 0..4
/// nodes with owned and borrowed (bit 0) buffers, the empty tree as
/// itself or 0. `eax` and memory.
#[test]
fn wp5d_rb_tree_release() {
    use ffx_prender::wp5d as d;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 64]);
        // remove: mov eax,[esp+4]; mov edx,[eax]; mov [ecx+0xc],edx; then log
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x10, 0x89, 0x51, 0x0c];
        c.extend(wide_stub(log, 1, 1, 4, 0x1111));
        jmp_stub(&mut t, d::RB_TREE_REMOVE, &c);
        let s = wide_stub_plain(log, 2, 1, 0, 0x2222);
        jmp_stub(&mut t, d::PHYRE_FREE, &s);
        let s = wide_stub(log, 3, 0, 0, 0x3333);
        jmp_stub(&mut t, d::FREE_LIST_RESET, &s);
        for case in 0..10u32 {
            let tree = t.alloc(&[0x29; 0x30]);
            let n = case % 5;
            let nodes: Vec<u32> = (0..n).map(|_| t.alloc(&[0x30; 0x18])).collect();
            for (k, &node) in nodes.iter().enumerate() {
                let next = nodes
                    .get(k + 1)
                    .copied()
                    .unwrap_or(if case < 5 { tree } else { 0 });
                t.w32(node, next);
                t.w32(node + 0xc, 0x0100_0000 + 0x10 * k as u32 + (case & 1));
                t.w32(
                    node + 0x10,
                    0x0200_0000 + 0x10 * k as u32 + u32::from(k % 2 == 1),
                );
            }
            let empty = if case < 5 { tree } else { 0 };
            t.w32(tree + 0xc, nodes.first().copied().unwrap_or(empty));
            t.w32(log, 0);
            let w = format!("{label} rb tree release {case}");
            t.run(&w, 0x0063_6970, tree, &[], false, |g| {
                d::rb_tree_release(g, tree)
            });
        }
    }
}
