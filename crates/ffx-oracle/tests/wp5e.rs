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
