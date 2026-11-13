//! M5 finish, WP5 part 4 (wp5e, `ffx_prender::wp5e`): the `FFX.exe`
//! frontier of the no-Unicorn boot, continued, against the original
//! machine code: `eax` and every byte written, on the image alone and
//! over a snapshot (`FFX_SNAPSHOT`). `operator new`/`operator delete` are
//! stubs on both sides (a bump allocator and a log of the freed
//! pointers). Needs `FFX_GAME_DIR`. (The scaffolding is `wp5d.rs`'.)
#![allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    dead_code // the scaffolding shared with wp5d.rs
)]

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
/// The original at `sym` on both oracles (to build the same state on
/// each); its `eax`, which must agree.
fn both(t: &mut Twins, sym: u32, conv: Conv, args: &[u32]) -> u32 {
    let at = t.a.rebase(sym);
    let p = t.a.call(at, conv, args).unwrap().eax;
    let q = t.b.call(at, conv, args).unwrap().eax;
    assert_eq!(p, q, "{sym:#x} alike");
    p
}

/// A fresh key string (`n` random characters) in a 0x1c-byte block.
fn key(t: &mut Twins, n: u32) -> u32 {
    let k = t.alloc(&[0x5a; 0x1c]);
    put_string(t, k, n);
    k
}

/// A fresh key string holding the text of the string at `s`.
fn copy_key(t: &mut Twins, s: u32) -> u32 {
    let n: u32 = Mem::read(&t.b, s + 0x10);
    let cap: u32 = Mem::read(&t.b, s + 0x14);
    let at = if cap >= 16 { Mem::read(&t.b, s) } else { s };
    let mut text = vec![0; n as usize];
    t.b.read_bytes(at, &mut text);
    let k = t.alloc(&[0x5a; 0x1c]);
    t.write(k, &text);
    t.write(k + n, &[0]);
    t.w32(k + 0x10, n);
    t.w32(k + 0x14, 15);
    k
}

/// The symbol map (`std::map<std::string, u32>`): its `_Buynode`
/// (`0x006f34f0`, the pair moved out of an inline or heap string),
/// `_Insert_hint` (`0x006f35b0`: maps of 0..24 random keys from a small
/// alphabet, built by the original `operator[]`, a new node hung with
/// every hint, the head and the leftmost included, now and then with a
/// key already there: each branch of the hint and `_Insert_nohint`) and `operator[]` (`0x006f3ae0`, 60 random
/// keys into one map, found and new). `eax` and memory.
#[test]
fn wp5e_symbol_map() {
    use ffx_prender::wp5e as e;
    for (label, mut t) in Twins::all() {
        for round in 0..20u32 {
            let head = t.alloc(&[0x66; 0x10]);
            let tree = t.alloc(&head.to_le_bytes());
            let n = t.rng.below(24);
            let pair = key(&mut t, n);
            t.w32(pair + 0x18, 0x5100 + round);
            let w = format!("{label} symbol buy node {round} (n {n})");
            t.run(&w, e::SYMBOL_BUY_NODE, tree, &[pair], false, |g| {
                e::symbol_buy_node(g, tree, pair)
            });
        }
        let slot = t.b.stack_range().start + 0x1000;
        for round in 0..30u32 {
            let (map, head) = empty_map(&mut t);
            let count = t.rng.below(25);
            let mut nodes = vec![head];
            for _ in 0..count {
                let n = 1 + t.rng.below(3);
                let k = key(&mut t, n);
                both(&mut t, e::SYMBOL_INDEX, Conv::Thiscall(map), &[k]);
            }
            // every node of the map, in order (the hints)
            let mut at: u32 = Mem::read(&t.b, head);
            while at != head {
                nodes.push(at);
                let it = t.alloc(&at.to_le_bytes());
                both(&mut t, e::ITERATOR_INC, Conv::Thiscall(it), &[]);
                at = Mem::read(&t.b, it);
            }
            for (i, &hint) in nodes.clone().iter().enumerate() {
                if i % 2 == 1 && round % 2 == 1 {
                    continue;
                }
                // now and then a key equal to the hint's, its
                // predecessor's or (at the head) the last one's
                let same = match t.rng.below(4) {
                    0 if hint != head => Some(hint),
                    1 if i > 1 => Some(nodes[i - 1]),
                    2 if i == 0 && nodes.len() > 1 => nodes.last().copied(),
                    _ => None,
                };
                let pair = if let Some(src) = same {
                    copy_key(&mut t, src + 0x10)
                } else {
                    let n = 1 + t.rng.below(3);
                    key(&mut t, n)
                };
                let node = both(&mut t, e::SYMBOL_BUY_NODE, Conv::Thiscall(map), &[pair]);
                let out = t.alloc(&[0x33; 4]);
                let w = format!(
                    "{label} symbol insert hint {round} hint {i} of {}",
                    nodes.len()
                );
                let value = node + 0x10;
                t.run(
                    &w,
                    e::SYMBOL_INSERT_HINT,
                    map,
                    &[out, hint, value, node],
                    false,
                    |g| e::symbol_insert_hint(g, map, out, hint, value, node, slot),
                );
            }
        }
        let (map, _) = empty_map(&mut t);
        let frame = t.b.stack_range().start + 0x1100;
        for round in 0..60u32 {
            let n = t.rng.below(4) + u32::from(round % 7 == 0) * 17;
            let k = key(&mut t, n);
            let w = format!("{label} symbol map index {round} (n {n})");
            t.run(&w, e::SYMBOL_INDEX, map, &[k], false, |g| {
                e::symbol_map_index(g, map, k, frame)
            });
        }
    }
}

/// A stub like [`wide_stub`] for a cdecl callee whose first word is the
/// caller's frame buffer (different on the two sides): logs `tag` and
/// stack words 2..=n+1 (`ecx` 0), returns `ret`, pops nothing.
fn skip_stub(log: u32, tag: u32, n: u8, ret: u32) -> Vec<u8> {
    let mut c = vec![0x31, 0xc9]; // xor ecx, ecx
    c.push(0xa1);
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0x83, 0xe0, 0x1f, 0xc1, 0xe0, 0x05]); // and eax, 31; shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    for i in 0..n {
        c.extend([0x8b, 0x54, 0x24, 8 + 4 * i]); // mov edx, [esp+8+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.push(0xc3);
    c
}

/// `operator new` failing (returning 0) in both from now on.
fn new_fails(t: &mut Twins) {
    let s = t.code(&[0x31, 0xc0, 0xc3]);
    let at = t.a.rebase(ws::OPERATOR_NEW);
    let mut j = vec![0xe9];
    j.extend(s.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

/// `TipSystem` (`0x006e85f0` constructor for the languages 9, 10 and
/// others, `CreateInstance` `0x006e8690` with `operator new` failing or
/// not, `DestroyInstance` `0x006e8700` with no system, a system with and
/// without a menu, the `"Hide"` callback `0x006e8760`): every callee a
/// logging stub (`sprintf` and `IggyPlayerOpenScaled` log all but the
/// buffer). `eax` and memory.
#[test]
fn wp5e_tip_system() {
    use ffx_prender::wp5e as e;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 32 * 32]);
        let lm = t.alloc(&[0x29; 8]);
        let mut c = vec![0xb8];
        c.extend(lm.to_le_bytes());
        c.push(0xc3);
        jmp_stub(&mut t, e::LOCALIZATION_MANAGER, &c);
        let s = t.code(&skip_stub(log, 0x5e, 2, 0x29));
        set_import(&mut t, e::SPRINTF_SLOT, s);
        let menu = t.alloc(&[0x47; 0x10]);
        let s = skip_stub(log, 0x0e, 4, menu);
        jmp_stub(&mut t, e::OPEN_SCALED, &s);
        let s = wide_stub(log, 0xca, 2, 8, 0xcafe);
        jmp_stub(&mut t, e::ADD_CALLBACK, &s);
        let s = wide_stub(log, 0xe0, 1, 4, 0xe0e0);
        jmp_stub(&mut t, e::SET_ENABLE, &s);
        let s = wide_stub(log, 0xd7, 0, 0, 0xd7d7);
        jmp_stub(&mut t, e::MENU_DTOR, &s);
        let buf = t.b.stack_range().start + 0x1000;
        for (case, lang) in [9u32, 10, 0, 11, 8].into_iter().enumerate() {
            t.w32(lm + 4, lang);
            let this = t.alloc(&[0x71; 4]);
            t.w32(log, 0);
            let w = format!("{label} tip ctor case {case} (language {lang})");
            t.run(&w, e::TIP_CTOR, this, &[], false, |g| {
                e::tip_system_ctor(g, this, buf)
            });
        }
        let g_at = t.a.rebase(e::TIP_SYSTEM);
        for case in 0..2u32 {
            t.w32(g_at, 0x1234_5678);
            t.w32(log, 0);
            let w = format!("{label} tip create {case}");
            t.run_cdecl(&w, e::TIP_CREATE, &[], |g| e::tip_system_create(g));
            if case == 0 {
                let w = format!("{label} tip hide {case}");
                t.run_cdecl(&w, e::TIP_HIDE, &[], |g| e::tip_hide(g));
            }
            let w = format!("{label} tip destroy {case}");
            t.run_cdecl(&w, e::TIP_DESTROY, &[], |g| e::tip_system_destroy(g));
            let w = format!("{label} tip destroy {case} again (none)");
            t.run_cdecl(&w, e::TIP_DESTROY, &[], |g| e::tip_system_destroy(g));
            if case == 0 {
                // a system without its menu
                let sys = t.new_block(4);
                t.w32(sys, 0);
                t.w32(g_at, sys);
                let w = format!("{label} tip destroy, no menu");
                t.run_cdecl(&w, e::TIP_DESTROY, &[], |g| e::tip_system_destroy(g));
                new_fails(&mut t);
            }
        }
    }
}

/// The 51 copies of `PArray_Data_004d45a0` (`PSharray<T>`'s elements):
/// counts 0, 1, 2, large, each with and without the top bit. `eax`.
#[test]
fn wp5e_sharray_data_copies() {
    use ffx_prender::wp5e as e;
    for (label, mut t) in Twins::all() {
        let arrays: Vec<u32> = [0u32, 1, 2, 7, 0x7fff_ffff]
            .into_iter()
            .flat_map(|n| [n, n | 0x8000_0000])
            .map(|n| {
                let a = t.alloc(&[0x3d; 8]);
                t.w32(a, n);
                t.w32(a + 4, 0x0abc_0000 | n & 0xffff);
                a
            })
            .collect();
        for &at in &e::SHARRAY_DATA_COPIES {
            for &a in &arrays {
                let w = format!("{label} sharray data {at:#x} on {a:#x}");
                t.run(&w, at, a, &[], false, |g| {
                    ffx_pcore::small::bridge::array_data(g, a)
                });
            }
        }
    }
}

/// A symbol map (`std::map<std::string, u32>`) of `count` random keys
/// (some long) with random values, built by the original `operator[]`
/// on both sides.
fn symbol_map(t: &mut Twins, count: u32) -> u32 {
    use ffx_prender::wp5e as e;
    let (map, _) = empty_map(t);
    for _ in 0..count {
        let n = 1 + t.rng.below(4) + u32::from(t.rng.below(5) == 0) * 16;
        let k = key(t, n);
        let v = both(t, e::SYMBOL_INDEX, Conv::Thiscall(map), &[k]);
        let x = t.rng.next();
        t.w32(v, x);
    }
    map
}

/// The asset list's map copy: `_Buynode(&pair)` (`0x00649a60`), the
/// recursive `_Copy` (`0x00649cb0`, from the root and from a subtree),
/// the map `_Copy` (`0x00649bd0`) and the copy constructor
/// (`0x0064d050`) on maps of 0..24 keys (inline and heap strings). `eax`
/// and memory.
#[test]
fn wp5e_asset_map_copy() {
    use ffx_prender::wp5e as e;
    for (label, mut t) in Twins::all() {
        for round in 0..12u32 {
            let head = t.alloc(&[0x66; 0x10]);
            let tree = t.alloc(&head.to_le_bytes());
            let n = t.rng.below(24);
            let pair = key(&mut t, n);
            t.w32(pair + 0x18, 0x6100 + round);
            let w = format!("{label} map buy node {round} (n {n})");
            t.run(&w, e::MAP_BUY_NODE, tree, &[pair], false, |g| {
                e::map_buy_node(g, tree, pair)
            });
        }
        for round in 0..16u32 {
            let count = [0, 1, 2, 3][round as usize % 4] + (round / 4) * 6;
            let src = symbol_map(&mut t, count);
            let src_head: u32 = Mem::read(&t.b, src);
            let root: u32 = Mem::read(&t.b, src_head + 4);
            // the recursive copy under a fresh head, from the root and
            // from the root's left child
            for (k, from) in [root, Mem::read(&t.b, root)].into_iter().enumerate() {
                let (dst, dst_head) = empty_map(&mut t);
                let w = format!("{label} tree copy {round} ({count} keys) from {k}");
                t.run(&w, e::TREE_COPY, dst, &[from, dst_head, 0x77], false, |g| {
                    e::tree_copy(g, dst, from, dst_head)
                });
            }
            let (dst, _) = empty_map(&mut t);
            let w = format!("{label} map copy {round} ({count} keys)");
            t.run(&w, e::MAP_COPY, dst, &[src, 0x77], false, |g| {
                e::map_copy(g, dst, src)
            });
            let this = t.alloc(&[0x2e; 8]);
            let w = format!("{label} map copy ctor {round} ({count} keys)");
            t.run(&w, 0x0064_d050, this, &[src, 0], false, |g| {
                e::map_copy_ctor(g, this, src)
            });
        }
    }
}

/// The vector copies: `std::vector<std::string>`'s `_Buy` (`0x00654e20`,
/// 0..6 elements), the words' `_Uninit_copy` (`0x0064a850`, `0x0064a880`)
/// and the strings' (`0x0064a8b0`), with and without a NULL target (one
/// element), and
/// the copy constructors (`0x0064d570` words, `0x0064d730` strings) of
/// vectors of 0..6 elements. `eax` and memory.
#[test]
fn wp5e_vector_copies() {
    use ffx_prender::wp5e as e;
    for (label, mut t) in Twins::all() {
        for n in 0..7u32 {
            let v = t.alloc(&[0x19; 12]);
            let w = format!("{label} vector buy 24 n {n}");
            t.run(&w, e::VECTOR_BUY_24, v, &[n], false, |g| {
                e::vector_buy(g, v, n, 0x18, 0x0aaa_aaaa)
            });
        }
        for round in 0..14u32 {
            // a NULL target (then the next element at address 4...) only
            // for one element
            let n = if round >= 12 { 1 } else { round % 6 };
            let null = round >= 12;
            let words: Vec<u8> = (0..4 * n).map(|_| t.rng.next() as u8).collect();
            let src = t.alloc(&[words.as_slice(), &[0; 4]].concat());
            for at in [e::WORDS_COPY, 0x0064_a880] {
                let dst = if null { 0 } else { t.alloc(&[0x44; 0x20]) };
                let w = format!("{label} words copy {at:#x} {round}");
                let args = [src, src + 4 * n, dst, 0x1234, 0x5678];
                t.run_cdecl(&w, at, &args, |g| e::words_copy(g, src, src + 4 * n, dst));
            }
            let strings = t.alloc(&vec![0x5a; 0x18 * n as usize + 4]);
            for i in 0..n {
                let len = t.rng.below(20);
                put_string(&mut t, strings + 0x18 * i, len);
            }
            let dst = if null { 0 } else { t.alloc(&[0x45; 0x90]) };
            let end = strings + 0x18 * n;
            let w = format!("{label} strings copy {round}");
            let args = [strings, end, dst, 0x1234, 0x5678];
            t.run_cdecl(&w, e::STRINGS_COPY, &args, |g| {
                e::strings_copy(g, strings, end, dst)
            });
            // the vectors over the same elements
            let vw = t.alloc(&[0; 12]);
            t.w32(vw, src);
            t.w32(vw + 4, src + 4 * n);
            t.w32(vw + 8, src + 4 * n);
            let this = t.alloc(&[0x2f; 12]);
            let w = format!("{label} vector<u32> copy ctor {round}");
            t.run(&w, 0x0064_d570, this, &[vw], false, |g| {
                e::vector_copy_ctor(g, this, vw, 4, e::VECTOR_BUY_4, e::WORDS_COPY)
            });
            let vs = t.alloc(&[0; 12]);
            t.w32(vs, strings);
            t.w32(vs + 4, end);
            t.w32(vs + 8, end);
            let this = t.alloc(&[0x2f; 12]);
            let w = format!("{label} vector<string> copy ctor {round}");
            t.run(&w, 0x0064_d730, this, &[vs], false, |g| {
                e::vector_copy_ctor(g, this, vs, 0x18, e::VECTOR_BUY_24, e::STRINGS_COPY)
            });
        }
    }
}

/// A fresh 0x30-byte node: junk links, red, not nil, a key string of
/// `n` random characters at `key`.
fn string_node(t: &mut Twins, key: u32, n: u32) -> u32 {
    let node = t.new_block(0x30);
    t.write(node, &[0x3a; 0x30]);
    t.write(node + 0xc, &[0, 0]);
    put_string(t, node + key, n);
    node
}

/// A red-black tree of `n` string-keyed nodes (the key at `key`, inline
/// or on the heap), grown at random leaf places by the original
/// `_Insert_at` `0x006e8a50` (which balances but compares nothing). The
/// map and its nodes in order.
fn grown_string_tree(t: &mut Twins, n: u32, key: u32) -> (u32, u32, Vec<u32>) {
    let (map, head) = empty_map(t);
    let out = t.alloc(&[0; 8]);
    let mut nodes: Vec<u32> = Vec::new();
    for _ in 0..n {
        let len = t.rng.below(20);
        let node = string_node(t, key, len);
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
        for off in [0, 4, 8] {
            t.w32(node + off, head);
        }
        both(
            t,
            0x006e_8a50,
            Conv::Thiscall(map),
            &[out, left, place, node + key, node],
        );
        nodes.push(node);
    }
    // in order
    let mut order = Vec::new();
    let mut at: u32 = Mem::read(&t.b, head);
    while at != head {
        order.push(at);
        let it = t.alloc(&at.to_le_bytes());
        both(t, 0x0063_52b0, Conv::Thiscall(it), &[]);
        at = Mem::read(&t.b, it);
    }
    (map, head, order)
}

/// The string maps' erase copies ([`ffx_prender::wp5e::STRING_MAP_ERASES`]):
/// `erase(iterator)` (each node of grown trees in turn, until empty: every
/// fix-up case, the extremes, heap keys freed) against `m5b_stl`'s
/// `tree_erase_with`, `erase(first, last)` (the whole tree, a middle run,
/// one node, an empty range) and the subtree frees `0x00635ce0`,
/// `0x006a04e0`, `0x006d79c0`. `eax`, `out` and memory.
#[test]
fn wp5e_string_map_erase() {
    use ffx_prender::wp5e as e;
    use ffx_system::m5b_stl as m;
    for (label, mut t) in Twins::all() {
        let frame = t.b.stack_range().start + 0x1000;
        let slot = frame + 0x10;
        for row in &e::STRING_MAP_ERASES {
            let calls = m::EraseCalls {
                rotate_left: row.rotate_left,
                rotate_right: row.rotate_right,
                min: row.min,
                max: row.max,
                string_key: Some(row.key),
            };
            for round in 0..6u32 {
                let n = 1 + t.rng.below(20);
                let (map, _, mut nodes) = grown_string_tree(&mut t, n, row.key);
                let out = t.alloc(&[0x33; 8]);
                let mut k = 0;
                while !nodes.is_empty() {
                    let i = t.rng.below(nodes.len() as u32) as usize;
                    let wh = nodes.remove(i);
                    t.b.write(slot, &wh.to_le_bytes()).unwrap();
                    let w = format!("{label} erase {:#x} {round} ({n}) #{k}", row.erase);
                    t.run(&w, row.erase, map, &[out, wh], false, |g| {
                        m::bridge::tree_erase_with(g, map, out, slot, &calls)
                    });
                    k += 1;
                }
            }
            for round in 0..12u32 {
                let n = 1 + t.rng.below(25);
                let (map, head, nodes) = grown_string_tree(&mut t, n, row.key);
                let out = t.alloc(&[0x33; 8]);
                let (first, last) = match round % 4 {
                    0 => (nodes[0], head),
                    1 => {
                        let i = t.rng.below(n);
                        let j = i + t.rng.below(n - i + 1);
                        (
                            nodes[i as usize],
                            nodes.get(j as usize).copied().unwrap_or(head),
                        )
                    }
                    2 => (
                        nodes[(n / 2) as usize],
                        nodes.get((n / 2 + 1) as usize).copied().unwrap_or(head),
                    ),
                    _ => (nodes[(n - 1) as usize], nodes[(n - 1) as usize]),
                };
                let w = format!("{label} erase range {:#x} {round} ({n})", row.erase_range);
                t.run(&w, row.erase_range, map, &[out, first, last], false, |g| {
                    e::string_map_erase_range(g, map, out, first, last, row, frame)
                });
            }
            if row.subtree != 0x0063_5c80 {
                for round in 0..6u32 {
                    let n = t.rng.below(16);
                    let (map, head, _) = grown_string_tree(&mut t, n, row.key);
                    let root: u32 = Mem::read(&t.b, head + 4);
                    let w = format!("{label} subtree free {:#x} {round} ({n})", row.subtree);
                    t.run_any_this1(&w, row.subtree, map, root, |g| {
                        e::erase_string_subtree(g, root, row.key);
                    });
                }
            }
        }
    }
}

/// The copies of `std_Tree_Lrotate_0069a3f0`, `std_Tree_Rrotate_0069a4f0`,
/// `std_Tree_max_0069a450` and `std_Tree_min_0069a470` bound in wp5e:
/// rotations at random nodes with a child on that side, extremes from
/// random nodes. Memory (and `eax` for the extremes).
#[test]
fn wp5e_tree_helper_copies() {
    use ffx_system::stl::bridge as stl;
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for &(kind, at) in TREE_HELPER_COPIES {
            for round in 0..4u32 {
                let n = 2 + t.rng.below(14);
                let (map, head, nodes) = grown_string_tree(&mut t, n, 0x10);
                // a node with a child on the rotation's side (any for
                // the extremes); none in this tree: the next round
                let fits: Vec<u32> = nodes
                    .iter()
                    .copied()
                    .filter(|&p| {
                        let side: u32 = Mem::read(&t.b, p + if kind == 'l' { 8 } else { 0 });
                        side != head || kind == 'x' || kind == 'n'
                    })
                    .collect();
                if fits.is_empty() {
                    continue;
                }
                let pick = fits[t.rng.below(fits.len() as u32) as usize];
                let w = format!("{label} tree helper {kind} {at:#x} {round}");
                match kind {
                    'l' => t.run_any_this1(&w, at, map, pick, |g| {
                        stl::tree_rotate_left(g, map, pick);
                    }),
                    'r' => t.run_any_this1(&w, at, map, pick, |g| {
                        stl::tree_rotate_right(g, map, pick);
                    }),
                    'x' => t.run_cdecl(&w, at, &[pick], |g| tb::tree_max(g, pick)),
                    _ => t.run_cdecl(&w, at, &[pick], |g| tb::tree_min(g, pick)),
                }
            }
        }
    }
}

/// The bound copies: (`l` rotate left, `r` right, `x` max, `n` min, address).
const TREE_HELPER_COPIES: &[(char, u32)] = &[
    ('l', 0x0062_5a60),
    ('l', 0x0063_5ec0),
    ('l', 0x0063_5f20),
    ('l', 0x006a_0830),
    ('l', 0x006d_7b80),
    ('l', 0x0080_6cb0),
    ('l', 0x0080_6d10),
    ('r', 0x0062_5b60),
    ('r', 0x0063_6140),
    ('r', 0x0063_61a0),
    ('r', 0x006a_0af0),
    ('r', 0x006d_7cc0),
    ('r', 0x0080_6eb0),
    ('r', 0x0080_6f10),
    ('x', 0x0062_5ac0),
    ('x', 0x0063_5f80),
    ('x', 0x0063_5fa0),
    ('x', 0x006a_0900),
    ('x', 0x006a_0920),
    ('x', 0x006d_7be0),
    ('x', 0x0080_6d90),
    ('n', 0x0062_5ae0),
    ('n', 0x0063_5fc0),
    ('n', 0x0063_5fe0),
    ('n', 0x0063_6000),
    ('n', 0x006a_0940),
    ('n', 0x006d_7c00),
    ('n', 0x0080_6dd0),
];
