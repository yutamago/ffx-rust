//! M5 finish, WP5 part 2 (`ffx_system::wp5_string`, `wp5_tree`,
//! `ffx_prender::wp5_hot`'s vsync handler): `std::string` as compiled into
//! `FFX.exe` and the other small originals, against the original machine code: `eax` and
//! every byte written, on the image alone and over a snapshot
//! (`FFX_SNAPSHOT`). `operator new`/`operator delete` and the CRT copies
//! run as the game's code and the oracle's CRT on both sides alike, so the
//! heap moves the same way. Needs `FFX_GAME_DIR`.
//!
//! Strings are random: sizes around the inline limit (15) and the growth
//! steps, short and heap buffers with spare capacity, sources inside the
//! string itself. The `_Xout_of_range`/`_Xlength_error` paths (they throw)
//! are left out: every position and size here is valid.
#![allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_system::wp5_string::{self as ws, bridge as b};

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
    /// A length: around 15/16 and the growth steps, sometimes 0.
    fn len(&mut self) -> u32 {
        match self.below(5) {
            0 => self.below(3),
            1 => 13 + self.below(6),
            2 => 30 + self.below(20),
            _ => self.below(40),
        }
    }
    /// A count that may run past the end (`npos` included).
    fn count(&mut self, max: u32) -> u32 {
        match self.below(4) {
            0 => u32::MAX,
            1 => max + self.below(5),
            _ => self.below(max + 1),
        }
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

    /// A string of `n` random characters: inline below 16 (or on the heap
    /// with spare capacity now and then), junk in the unused bytes.
    fn string(&mut self, n: u32) -> u32 {
        let s = self.alloc(&[0x5a; 0x18]);
        let text = self.text(n);
        let heap = n >= 16 || self.rng.below(4) == 0;
        let (at, cap) = if heap {
            let cap = (n | 0xf).max(16) + self.rng.below(3) * 16;
            let p = self.new_block(cap + 1);
            self.write(p, &vec![0x77; (cap + 1) as usize]);
            self.w32(s, p);
            (p, cap)
        } else {
            (s, 15)
        };
        self.write(at, &text);
        self.write(at + n, &[0]);
        self.w32(s + 0x10, n);
        self.w32(s + 0x14, cap);
        s
    }

    fn size(&self, s: u32) -> u32 {
        Mem::read(&self.b, s + 0x10)
    }

    fn data(&self, s: u32) -> u32 {
        let cap: u32 = Mem::read(&self.b, s + 0x14);
        if cap >= 16 { Mem::read(&self.b, s) } else { s }
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

#[test]
fn wp5b_string_assign_erase_cut() {
    for (label, mut t) in Twins::all() {
        for round in 0..120u32 {
            let n = t.rng.len();
            let src = t.string(n);
            let pos = t.rng.below(n + 1);
            let cnt = t.rng.count(n);
            // into another string, or itself
            let this = if round % 4 == 0 { src } else { t.rng.len() };
            let this = if round % 4 == 0 { this } else { t.string(this) };
            let w = format!("{label} assign_substr {round} (n {n}, pos {pos}, count {cnt:#x})");
            t.run(&w, 0x0040_3a10, this, &[src, pos, cnt], false, |g| {
                b::assign_substr(g, this, src, pos, cnt)
            });
            // assign(p, n) from a buffer, or from its own characters
            let l = t.rng.len();
            let s = t.string(l);
            let size = t.size(s);
            let (p, len) = if round % 3 == 0 && size > 0 {
                let off = t.rng.below(size + 1);
                (t.data(s) + off, t.rng.below(size - off + 3))
            } else {
                let len = t.rng.len();
                let text = t.text(len + 4);
                (t.alloc(&text), len)
            };
            let w = format!("{label} assign {round} (size {size}, len {len})");
            t.run(&w, 0x0040_4270, s, &[p, len], false, |g| {
                b::assign(g, s, p, len)
            });
            // assign(const char *)
            let l = t.rng.len();
            let s = t.string(l);
            let l = t.rng.len();
            let mut text = t.text(l);
            text.push(0);
            let p = t.alloc(&text);
            let w = format!("{label} assign_cstr {round}");
            t.run(&w, 0x0040_3b30, s, &[p], false, |g| b::assign_cstr(g, s, p));
            // erase(off, count) and the cut
            let n = t.rng.len();
            let s = t.string(n);
            let off = t.rng.below(n + 1);
            let cnt = t.rng.count(n - off);
            let w = format!("{label} erase {round} (n {n}, off {off}, count {cnt:#x})");
            t.run(&w, 0x0040_43b0, s, &[off, cnt], false, |g| {
                b::erase(g, s, off, cnt)
            });
            let n = t.rng.len();
            let s = t.string(n);
            let k = t.rng.below(n + 1);
            let w = format!("{label} cut {round} (n {n} to {k})");
            t.run(&w, 0x0040_4370, s, &[k], false, |g| b::cut(g, s, k));
        }
    }
}

#[test]
fn wp5b_string_append_substr_move() {
    for (label, mut t) in Twins::all() {
        for round in 0..120u32 {
            // append(str, pos, n), itself on some
            let n = t.rng.len();
            let src = t.string(n);
            let pos = t.rng.below(n + 1);
            let cnt = t.rng.count(n);
            let this = if round % 4 == 0 {
                src
            } else {
                let m = t.rng.len();
                t.string(m)
            };
            let w = format!("{label} append_substr {round} (n {n}, pos {pos}, count {cnt:#x})");
            t.run(&w, 0x0065_75a0, this, &[src, pos, cnt], false, |g| {
                b::append_substr(g, this, src, pos, cnt)
            });
            // append(p, n): a buffer, or its own characters
            let l = t.rng.len();
            let s = t.string(l);
            let size = t.size(s);
            let (p, len) = if round % 3 == 0 && size > 0 {
                let off = t.rng.below(size + 1);
                (t.data(s) + off, t.rng.below(size - off + 3))
            } else {
                let len = t.rng.len();
                let text = t.text(len + 4);
                (t.alloc(&text), len)
            };
            let w = format!("{label} append {round} (size {size}, len {len})");
            t.run(&w, 0x0065_76d0, s, &[p, len], false, |g| {
                b::append(g, s, p, len)
            });
            let l = t.rng.len();
            let s = t.string(l);
            let l = t.rng.len();
            let mut text = t.text(l);
            text.push(0);
            let p = t.alloc(&text);
            let w = format!("{label} append_cstr {round}");
            t.run(&w, 0x0065_7690, s, &[p], false, |g| b::append_cstr(g, s, p));
            // substr into junk
            let n = t.rng.len();
            let s = t.string(n);
            let out = t.alloc(&[0x33; 0x18]);
            let pos = t.rng.below(n + 1);
            let cnt = t.rng.count(n);
            let w = format!("{label} substr {round} (n {n}, pos {pos}, count {cnt:#x})");
            t.run(&w, 0x0040_2e20, s, &[out, pos, cnt], false, |g| {
                b::substr(g, s, out, pos, cnt)
            });
            // the move into junk
            let l = t.rng.len();
            let from = t.string(l);
            let to = t.alloc(&[0x44; 0x18]);
            let w = format!("{label} move {round}");
            t.run(&w, 0x0040_39b0, to, &[from], false, |g| {
                b::move_from(g, to, from)
            });
        }
    }
}

#[test]
fn wp5b_string_find_compare_grow() {
    for (label, mut t) in Twins::all() {
        for round in 0..150u32 {
            let n = t.rng.len();
            let s = t.string(n);
            // a needle from the string itself or random, 0 to 4 long
            let k = t.rng.below(5);
            let needle = if round % 4 == 1 && k > 0 && n + 1 >= k {
                // the string's tail and its 0: found only past the window
                let mut v = vec![0u8; k as usize];
                t.b.read_bytes(t.data(s) + n + 1 - k, &mut v);
                v
            } else if round % 2 == 0 && n >= k {
                let off = t.rng.below(n - k + 1);
                let mut v = vec![0u8; k as usize];
                t.b.read_bytes(t.data(s) + off, &mut v);
                v
            } else {
                t.text(k)
            };
            let mut nz = needle.clone();
            nz.extend([0x66; 4]);
            let p = t.alloc(&nz);
            let pos = t.rng.below(n + 3);
            let w = format!("{label} find {round} (n {n}, pos {pos}, k {k})");
            t.run(&w, 0x0040_3bd0, s, &[p, pos, k], false, |g| {
                b::find(g, s, p, pos, k)
            });
            let pos = t.rng.below(n + 1);
            let cnt = t.rng.count(n - pos);
            let len = t.rng.below(k + 3);
            let w =
                format!("{label} compare {round} (n {n}, pos {pos}, count {cnt:#x}, len {len})");
            t.run(&w, 0x0065_90d0, s, &[pos, cnt, p, len], false, |g| {
                b::compare(g, s, pos, cnt, p, len)
            });
            let w = format!("{label} begin {round}");
            t.run(&w, 0x0040_3d80, s, &[], false, |g| b::begin(g, s));
            // _Grow(new size, trim)
            let n = t.rng.len();
            let s = t.string(n);
            let new = match t.rng.below(4) {
                0 => 0,
                1 => t.rng.below(16),
                _ => t.rng.len(),
            };
            let trim = [0u32, 1, 0x100, 0xff01][t.rng.below(4) as usize];
            let w = format!("{label} grow {round} (n {n}, new {new}, trim {trim:#x})");
            t.run(&w, 0x0040_4480, s, &[new, trim], false, |g| {
                b::grow(g, s, new, trim)
            });
            // _Copy(new size, old size)
            let n = t.rng.len();
            let s = t.string(n);
            let new = n + t.rng.below(40);
            let old = t.rng.below(n + 1);
            let w = format!("{label} copy {round} (n {n}, new {new}, old {old})");
            t.run(&w, 0x0040_4740, s, &[new, old], false, |g| {
                b::grow_copy(g, s, new, old)
            });
        }
    }
}

/// `std::_Tree::_Insert_nohint` (`0x006995e0`) growing trees of `u32`
/// keys from empty (keys from a small range, so equal ones take the
/// freeing path; `leftish` 0 and 1, the high bytes of its word junk), and
/// the iterator's `--` (`0x00699f50`) from every node and the head.
#[test]
fn wp5b_tree_insert_nohint_and_iterator_dec() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for round in 0..12u32 {
            // the head: left, parent, right itself, black, nil
            let head = t.alloc(&[0; 0x20]);
            let mut h = Vec::new();
            for _ in 0..3 {
                h.extend(head.to_le_bytes());
            }
            h.extend([1, 1, 0, 0]);
            t.write(head, &h);
            let tree = t.alloc(&[0; 8]);
            t.w32(tree, head);
            let out = t.alloc(&[0x33; 8]);
            // the port's frame on the stack, as the original's slot
            let frame = t.b.stack_range().start + 0x1000;
            let mut nodes = vec![head];
            let keys = if round % 2 == 0 { 8 } else { 1000 };
            for i in 0..40u32 {
                let k = t.rng.below(keys);
                let node = t.alloc(&[0x5c; 0x20]);
                let mut n = Vec::new();
                for _ in 0..3 {
                    n.extend(head.to_le_bytes());
                }
                n.extend([0, 0, 0x5c, 0x5c]);
                n.extend(k.to_le_bytes());
                t.write(node, &n);
                let value = t.alloc(&k.to_le_bytes());
                let leftish = [0u32, 1, 0x100, 0xab01][t.rng.below(4) as usize];
                let w =
                    format!("{label} insert_nohint {round}.{i} (key {k}, leftish {leftish:#x})");
                t.run(
                    &w,
                    0x0069_95e0,
                    tree,
                    &[out, leftish, value, node],
                    false,
                    |g| tb::insert_nohint(g, tree, out, leftish, value, node, frame),
                );
                if Mem::read::<u8>(&t.b, out + 4) != 0 {
                    nodes.push(node);
                }
            }
            let it = t.alloc(&[0; 4]);
            for &n in &nodes {
                t.w32(it, n);
                let w = format!("{label} iterator dec {round} from {n:#x}");
                t.run(&w, 0x0069_9f50, it, &[], false, |g| {
                    ffx_glue_other::m5v_a::bridge::tree_iterator_dec(g, it)
                });
            }
        }
    }
}

/// `VsyncHandler` (`0x00887d60`): the on flag 0 or not, the disable count
/// 0 or not, each callback NULL or a stub that counts its calls in its
/// own word and returns a tag; `eax` and memory.
#[test]
fn wp5b_vsync_handler() {
    use ffx_prender::wp5_hot as hot;
    for (label, mut t) in Twins::all() {
        let counts = t.alloc(&[0; 12]);
        let stubs: Vec<u32> = (0..3u32)
            .map(|i| {
                let mut c = vec![0xff, 0x05];
                c.extend((counts + 4 * i).to_le_bytes()); // inc dword [count i]
                c.push(0xb8);
                c.extend((0x7100_0000 + i).to_le_bytes()); // mov eax, tag
                c.push(0xc3);
                t.code(&c)
            })
            .collect();
        let (on, off, table) = (
            t.a.rebase(hot::SYNCV_ON),
            t.a.rebase(hot::SYNCV_DISABLE),
            t.a.rebase(hot::SYNCV_CALLBACKS),
        );
        for case in 0..32u32 {
            t.w32(on, [0, 1, 0x100][(case % 3) as usize]);
            t.w32(off, u32::from(case & 4 != 0));
            for i in 0..3 {
                let f = if case & (8 << i) != 0 {
                    0
                } else {
                    stubs[i as usize]
                };
                t.w32(table + 4 * i, f);
            }
            let w = format!("{label} vsync handler case {case}");
            t.run_cdecl(&w, 0x0088_7d60, &[], |g| hot::vsync_handler(g));
        }
    }
}

/// `canUseAutoBattle` (`0x00786f50`) over every story flag it tests (with
/// junk above the flag's 16 bits), each with its battles, their
/// neighbours and others; the two callees stubbed to return words from
/// memory.
#[test]
fn wp5b_can_use_auto_battle() {
    use ffx_prender::wp5_hot as hot;
    for (label, mut t) in Twins::all() {
        let cells = t.alloc(&[0; 8]);
        for (i, sym) in [hot::GET_SCENARIO_FLAG, hot::MS_BATTLE_GET_NUM]
            .into_iter()
            .enumerate()
        {
            let mut c = vec![0xa1];
            c.extend((cells + 4 * i as u32).to_le_bytes()); // mov eax, [cell]
            c.push(0xc3);
            let to = t.code(&c);
            let at = t.a.rebase(sym);
            let mut j = vec![0xe9];
            j.extend(to.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
        }
        let mut cases: Vec<(u32, u32)> = Vec::new();
        for &(f, b) in &hot::NO_AUTO_BATTLE {
            for battle in [b, b + 1, b.wrapping_sub(1), 0x0022_0000] {
                cases.push((u32::from(f), battle));
            }
        }
        for f in [0xfu32, 0xd20, 0, 1, 0x36, 0xffff, 0x1_0037] {
            cases.push((f, 0x0022_0000));
        }
        for (k, (f, battle)) in cases.into_iter().enumerate() {
            let junk = if k % 2 == 0 { 0 } else { 0x5a5a_0000 };
            t.w32(cells, f | junk);
            t.w32(cells + 4, battle);
            let w = format!("{label} canUseAutoBattle flag {f:#x} battle {battle:#x}");
            t.run_cdecl(&w, 0x0078_6f50, &[], |g| hot::can_use_auto_battle(g));
        }
    }
}

#[test]
fn wp5b_string_ctors_and_move_assign() {
    for (label, mut t) in Twins::all() {
        for round in 0..100u32 {
            let s = t.alloc(&[0x5a; 0x18]);
            let l = t.rng.len();
            let mut text = t.text(l);
            text.push(0);
            let p = t.alloc(&text);
            let w = format!("{label} ctor_cstr {round} ({l})");
            t.run(&w, 0x0040_2c90, s, &[p], false, |g| b::ctor_cstr(g, s, p));
            let l = t.rng.len();
            let from = t.string(l);
            let to = t.alloc(&[0x44; 0x18]);
            let w = format!("{label} ctor_move {round} ({l})");
            t.run(&w, 0x0040_2cf0, to, &[from], false, |g| {
                b::ctor_move(g, to, from)
            });
            let l = t.rng.len();
            let to = t.string(l);
            let l2 = t.rng.len();
            let from = if round % 5 == 0 { to } else { t.string(l2) };
            let w = format!("{label} assign_move {round} ({l} <- {l2})");
            t.run(&w, 0x0040_2d60, to, &[from], false, |g| {
                b::assign_move(g, to, from)
            });
        }
    }
}

/// `std::map<std::string, ...>::lower_bound` (`0x006e9920`) on random
/// search trees of string keys (inline and heap strings, shared
/// prefixes, the empty string), with keys present, absent, longer and
/// shorter; an empty tree.
#[test]
fn wp5b_tree_lower_bound_string() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for round in 0..20u32 {
            let head = t.alloc(&[0; 0x30]);
            let mut h = Vec::new();
            for _ in 0..3 {
                h.extend(head.to_le_bytes());
            }
            h.extend([1, 1, 0, 0]);
            t.write(head, &h);
            let tree = t.alloc(&[0; 8]);
            t.w32(tree, head);
            // a plain search tree, keys inserted in random order
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut nodes: Vec<u32> = Vec::new();
            let count = if round == 0 { 0 } else { 1 + t.rng.below(30) };
            for _ in 0..count {
                let l = t.rng.len();
                let k = t.text(l);
                let s = t.string(0);
                let p = t.alloc(&k);
                b::assign(&mut G(&mut t.a), s, p, l);
                b::assign(&mut G(&mut t.b), s, p, l);
                let node = t.alloc(&[0x66; 0x30]);
                let mut n = Vec::new();
                for _ in 0..3 {
                    n.extend(head.to_le_bytes());
                }
                n.extend([0, 0, 0, 0]);
                t.write(node, &n);
                let mut sbytes = vec![0u8; 0x18];
                t.b.read_bytes(s, &mut sbytes);
                t.write(node + 0x10, &sbytes);
                // walk down from the root
                let mut at: u32 = Mem::read(&t.b, head + 4);
                if at == head {
                    t.w32(head + 4, node);
                    t.w32(node + 4, head);
                } else {
                    loop {
                        let i = nodes.iter().position(|&x| x == at).unwrap();
                        let off = if k < keys[i] { 0 } else { 8 };
                        let next: u32 = Mem::read(&t.b, at + off);
                        if next == head {
                            t.w32(at + off, node);
                            t.w32(node + 4, at);
                            break;
                        }
                        at = next;
                    }
                }
                keys.push(k);
                nodes.push(node);
            }
            t.a.clear_dirty();
            t.b.clear_dirty();
            for q in 0..20u32 {
                let k = if q % 3 == 0 && !keys.is_empty() {
                    let mut k = keys[t.rng.below(keys.len() as u32) as usize].clone();
                    match q % 4 {
                        1 => k.push(b'a'),
                        2 => {
                            k.pop();
                        }
                        _ => {}
                    }
                    k
                } else {
                    let l = t.rng.len();
                    t.text(l)
                };
                let s = t.string(0);
                let p = t.alloc(&k);
                let l = k.len() as u32;
                b::assign(&mut G(&mut t.a), s, p, l);
                b::assign(&mut G(&mut t.b), s, p, l);
                t.a.clear_dirty();
                t.b.clear_dirty();
                let w = format!("{label} lower_bound {round}.{q} ({} keys)", keys.len());
                t.run(&w, 0x006e_9920, tree, &[s], false, |g| {
                    tb::lower_bound_string(g, tree, s)
                });
            }
        }
    }
}

/// `IsCharacterCancleLoad` (`0x00635a70`): every id it names and their
/// neighbours, ids around the `0x4000` limit and negative ones (junk in
/// the high bytes), story flag `0x2e` or not (junk above its 16 bits),
/// map `0x17e` or not; full `eax`.
#[test]
fn wp5b_is_character_cancel_load() {
    use ffx_prender::wp5_hot as hot;
    for (label, mut t) in Twins::all() {
        let cell = t.alloc(&[0; 4]);
        let mut c = vec![0xa1];
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.push(0xc3);
        let to = t.code(&c);
        let at = t.a.rebase(hot::GET_SCENARIO_FLAG);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let map = t.a.rebase(hot::CURRENT_MAP_NO);
        let ids = [
            0x4067u32,
            0x4066,
            5,
            6,
            7,
            4,
            0x109b,
            0x109c,
            0x5001,
            0x5002,
            0x5003,
            0x3fff,
            0x3000,
            0x4000,
            0x4fff,
            0x8000_0005,
            0xffff_f067,
            0x1234_5006,
            0,
            0xab00_0002,
        ];
        for flag in [0x2eu32, 0x5a5a_002e, 0x2f, 0] {
            for m in [0x17eu32, 0x17f, 0] {
                for &id in &ids {
                    t.w32(cell, flag);
                    t.w32(map, m);
                    let w = format!("{label} cancel load flag {flag:#x} map {m:#x} id {id:#x}");
                    t.run_cdecl(&w, 0x0063_5a70, &[id], |g| {
                        hot::is_character_cancel_load(g, id)
                    });
                }
            }
        }
    }
}

/// The `_Tree` node makers: the head node (`0x006a0390`), the plain nodes
/// of 0x2c and 0x20 bytes (`0x006a03c0`, `0x006a0400`), the 12-byte value
/// nodes (`0x0069c5f0`, `0x0069c630`) and the prototype table's
/// (`0x0069c5c0`, its value copied by the original `0x0069c200`), over
/// the stub allocator (junk in the fresh nodes).
#[test]
fn wp5b_tree_node_makers() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for round in 0..8u32 {
            let tree = t.alloc(&[0; 8]);
            let head = t.alloc(&[0x77; 0x20]);
            t.w32(tree, head);
            let w = format!("{label} head node {round}");
            t.run_cdecl(&w, 0x006a_0390, &[], |g| tb::buy_head_node(g));
            for (sym, size) in [(0x006a_03c0u32, 0x2cu32), (0x006a_0400, 0x20)] {
                let w = format!("{label} node {sym:#x} {round}");
                t.run(&w, sym, tree, &[], false, |g| tb::buy_node(g, tree, size));
            }
            let value = t.alloc(&[0x10 + round as u8; 16]);
            for sym in [0x0069_c5f0u32, 0x0069_c630] {
                let w = format!("{label} value node {sym:#x} {round}");
                t.run(&w, sym, tree, &[value], false, |g| {
                    tb::buy_node_value12(g, tree, value)
                });
            }
            // a prototype: a string, then words (what 0x0069c200 copies)
            let l = t.rng.len();
            let s = t.string(l);
            let mut proto = vec![0u8; 0x1c];
            t.b.read_bytes(s, &mut proto[..0x18]);
            proto[0x18..].copy_from_slice(&(0x1234_0000 + round).to_le_bytes());
            let p = t.alloc(&proto);
            let w = format!("{label} prototype node {round}");
            t.run(&w, 0x0069_c5c0, tree, &[p], false, |g| {
                tb::buy_node_prototype(g, tree, p)
            });
        }
    }
}

/// `VideoIndexMap_buyNode_006d5ca0` and its node maker `0x006d7930`:
/// paths of every length around the inline limit, empty included.
#[test]
fn wp5b_video_buy_node() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        let tree = t.alloc(&[0; 8]);
        let head = t.alloc(&[0x77; 0x20]);
        t.w32(tree, head);
        for round in 0..30u32 {
            let l = t.rng.len();
            let mut path = t.text(l);
            path.push(0);
            let p = t.alloc(&path);
            let mut v = (0x0100_0000 + round).to_le_bytes().to_vec();
            v.extend(p.to_le_bytes());
            let value = t.alloc(&v);
            let w = format!("{label} video node {round} ({l})");
            t.run(&w, 0x006d_5ca0, tree, &[value], false, |g| {
                tb::video_buy_node(g, tree, value)
            });
            let w = format!("{label} video node maker {round}");
            t.run(&w, 0x006d_7930, tree, &[], false, |g| {
                tb::buy_node(g, tree, 0x2c)
            });
        }
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

/// The page manager thread (`0x00947e30`) and the FMOD manager thread
/// (`0x006fa1b0`): their callees logging stubs; the wait (and the tick)
/// clear the run flag after a few turns.
#[test]
fn wp5b_thread_bodies() {
    use ffx_prender::wp5_hot as hot;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let left = t.alloc(&[0; 4]);
        let mgr = t.alloc(&[0x21; 0x1d00]);
        let run = t.a.rebase(hot::PAGE_THREAD_RUN);
        t.w32(t.a.rebase(hot::PAGE_MANAGER), mgr);
        // the wait: logs, counts `left` down, clears the run flag at 0
        let mut w = log_stub_arg(&mut t, log, 0, 0, true);
        w.truncate(w.len() - 6); // drop `mov eax, ret; ret`
        w.extend([0xff, 0x0d]);
        w.extend(left.to_le_bytes()); // dec dword [left]
        w.extend([0x75, 0x07]); // jnz +7
        w.extend([0xc6, 0x05]);
        w.extend(run.to_le_bytes());
        w.push(0); // mov byte [run], 0
        w.extend([0x31, 0xc0, 0xc2, 0x08, 0x00]); // xor eax, eax; ret 8
        let wait = t.code(&w);
        t.w32(t.a.rebase(hot::WAIT_IAT), wait);
        let flush = log_stub(&mut t, log, 4, 7);
        jmp_stub(&mut t, hot::PAGE_FLUSH, &flush);
        for turns in [0u32, 1, 3] {
            t.write(run, &[u8::from(turns != 0)]);
            t.w32(left, turns);
            t.w32(log, 0);
            let w = format!("{label} page thread {turns}");
            t.run_cdecl(&w, 0x0094_7e30, &[0], |g| hot::page_thread(g));
        }
        // the FMOD thread: the tick clears +0x4c after `left` turns
        let fm = t.alloc(&[0x31; 0x50]);
        t.w32(t.a.rebase(hot::FMOD_MANAGER), fm);
        let mut c = log_stub(&mut t, log, 0, 0);
        c.truncate(c.len() - 6);
        c.extend([0xff, 0x0d]);
        c.extend(left.to_le_bytes()); // dec dword [left]
        c.extend([0x75, 0x04]); // jnz +4
        c.extend([0xc6, 0x41, 0x4c, 0x00]); // mov byte [ecx+0x4c], 0
        c.extend([0xb8, 0x11, 0, 0, 0, 0xc3]); // mov eax, 0x11; ret
        jmp_stub(&mut t, hot::FMOD_TICK, &c);
        for (sym, ret) in [
            (hot::FMOD_DRAIN, 0x22),
            (hot::FMOD_SHUTDOWN, 0x33),
            (hot::FMOD_THREAD_EXIT, 0x44),
        ] {
            let s = log_stub_arg(&mut t, log, 0, ret, sym == hot::FMOD_THREAD_EXIT);
            jmp_stub(&mut t, sym, &s);
        }
        for turns in [0u32, 2] {
            t.write(fm + 0x4c, &[u8::from(turns != 0), 0]);
            t.w32(fm + 8, 0x0808_0000 + turns);
            t.w32(fm + 0xc, 0x0c0c_0000 + turns);
            t.w32(left, turns);
            t.w32(log, 0);
            let w = format!("{label} fmod thread {turns}");
            t.run_cdecl(&w, 0x006f_a1b0, &[], |g| hot::fmod_thread(g));
        }
    }
}

/// `std::vector<std::string>::push_back` (`0x00667770`): vectors of 0 to
/// 6 strings, full or with room (the original growth `0x00655de0` on both
/// sides), the source outside or one of the vector's own elements.
#[test]
fn wp5b_vector_string_push_back() {
    for (label, mut t) in Twins::all() {
        for round in 0..60u32 {
            let n = t.rng.below(7);
            let cap = n + if round % 2 == 0 {
                0
            } else {
                1 + t.rng.below(2)
            };
            let buf = t.alloc(&vec![0x3c; (0x18 * cap.max(1)) as usize]);
            for i in 0..n {
                let l = t.rng.len();
                let s = t.string(l);
                let mut b = vec![0u8; 0x18];
                t.b.read_bytes(s, &mut b);
                t.write(buf + 0x18 * i, &b);
            }
            let v = t.alloc(&[0; 12]);
            t.w32(v, buf);
            t.w32(v + 4, buf + 0x18 * n);
            t.w32(v + 8, buf + 0x18 * cap);
            let src = if n > 0 && round % 3 == 0 {
                buf + 0x18 * t.rng.below(n)
            } else {
                let l = t.rng.len();
                t.string(l)
            };
            let w = format!("{label} vector push_back {round} ({n} of {cap})");
            t.run(&w, 0x0066_7770, v, &[src], false, |g| {
                b::vector_string_push_back(g, v, src)
            });
        }
    }
}

/// `_Max`/`_Min` (`0x0069a450`, `0x0069a470`) on random search trees,
/// from every node.
#[test]
fn wp5b_tree_min_max() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for round in 0..10u32 {
            let head = t.alloc(&[0; 0x20]);
            t.write(head + 0xd, &[1]);
            let mut nodes: Vec<(u32, u32)> = Vec::new();
            for _ in 0..=t.rng.below(20) {
                let k = t.rng.below(1000);
                let node = t.alloc(&[0; 0x20]);
                let mut n = Vec::new();
                for _ in 0..3 {
                    n.extend(head.to_le_bytes());
                }
                t.write(node, &n);
                if let Some(&(root, _)) = nodes.first() {
                    let mut at = root;
                    loop {
                        let ak = nodes.iter().find(|x| x.0 == at).unwrap().1;
                        let off = if k < ak { 0 } else { 8 };
                        let next: u32 = Mem::read(&t.b, at + off);
                        if next == head {
                            t.w32(at + off, node);
                            t.w32(node + 4, at);
                            break;
                        }
                        at = next;
                    }
                }
                nodes.push((node, k));
            }
            for &(n, _) in &nodes {
                let w = format!("{label} max {round} {n:#x}");
                t.run_cdecl(&w, 0x0069_a450, &[n], |g| tb::tree_max(g, n));
                let w = format!("{label} min {round} {n:#x}");
                t.run_cdecl(&w, 0x0069_a470, &[n], |g| tb::tree_min(g, n));
            }
        }
    }
}

/// The small originals of `ffx_prender::wp5_small`: junk objects, the
/// callees logging stubs (their `ecx` and argument words compared through
/// memory), the allocator stub for the list node.
#[test]
fn wp5b_small_originals() {
    use ffx_prender::wp5_small as sm;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        for round in 0..6u32 {
            let obj = t.alloc(&[0x40 + round as u8; 0x40]);
            t.w32(
                obj,
                if round % 2 == 0 {
                    0x8000_0005
                } else {
                    0x7fff_fff0
                },
            );
            let w = format!("{label} parray count {round}");
            t.run(&w, 0x0043_20c0, obj, &[], false, |g| {
                sm::parray_count(g, obj)
            });
            let w = format!("{label} zero three {round}");
            t.run(&w, 0x0069_06b0, obj, &[], false, |g| {
                sm::zero_three_words(g, obj)
            });
            let w = format!("{label} job worker ctor {round}");
            t.run(&w, 0x0070_3960, obj, &[], false, |g| {
                sm::job_worker_ctor(g, obj)
            });
            let w = format!("{label} clear word 10 {round}");
            t.run(&w, 0x0070_38f0, obj, &[], false, |g| {
                sm::clear_word10(g, obj)
            });
            let (next, prev) = if round % 3 == 0 {
                (0, 0x1234)
            } else {
                (obj, obj + 8)
            };
            let w = format!("{label} list node {round}");
            let at = t.a.rebase(0x0065_4f00);
            let r = t.a.call(at, Conv::Stdcall, &[next, prev]).unwrap();
            let p = sm::list_buy_node(&mut G(&mut t.b), next, prev);
            assert_eq!(r.eax, p, "{w}: eax");
            t.same(&w);
        }
        // the callees: logging stubs (ret 4 / ret 8 / cdecl)
        let s = log_stub(&mut t, log, 4, 0x41);
        jmp_stub(&mut t, sm::CLUSTER_DELETE_OBJECT, &s);
        let s = log_stub(&mut t, log, 8, 0x42);
        jmp_stub(&mut t, sm::SCENE_DELETE_MESH_INSTANCE, &s);
        let s = log_stub_arg(&mut t, log, 0, 0x43, true);
        jmp_stub(&mut t, sm::CD_CHANNEL_CLEAR, &s);
        let s = log_stub_arg(&mut t, log, 0, 0x44, false);
        jmp_stub(&mut t, sm::HDD_STATE_CLEAR, &s);
        let s = log_stub_arg(&mut t, log, 0, 0x45, true);
        jmp_stub(&mut t, sm::OPERATOR_DELETE, &s);
        let scene = t.a.rebase(sm::PHYRE_SCENE);
        t.w32(scene, 0x5ce0_0000);
        for round in 0..4u32 {
            let obj = t.alloc(&[0x50 + round as u8; 0x40]);
            t.w32(log, 0);
            let w = format!("{label} delete object {round}");
            t.run(&w, 0x006a_1550, obj, &[], false, |g| {
                sm::character_delete_object(g, obj)
            });
            t.w32(log, 0);
            let w = format!("{label} delete mesh instance {round}");
            t.run(&w, 0x006a_1570, obj, &[], false, |g| {
                sm::character_delete_mesh_instance(g, obj)
            });
            let abmap = t.a.rebase(sm::GRAPHIC_ABMAP);
            t.w32(abmap, if round % 2 == 0 { 0 } else { obj });
            t.w32(log, 0);
            let w = format!("{label} abmap free {round}");
            t.run_cdecl(&w, 0x0068_4ae0, &[], |g| sm::abmap_free(g));
        }
        for &a in &sm::CD_WORDS {
            let at = t.a.rebase(a);
            t.w32(at, 0x7777_7777);
        }
        let at = t.a.rebase(sm::CD_BYTE);
        t.write(at, &[0x77]);
        t.w32(log, 0);
        let w = format!("{label} cd reset channels");
        t.run_cdecl(&w, 0x0076_dcb0, &[], |g| sm::cd_reset_channels(g));
    }
}

/// [`log_stub_arg`] for a callee that takes no `ecx` (cdecl, stdcall): the
/// logged `ecx` is 0 (`xor ecx, ecx` first), as the caller's is junk.
fn log_stub_plain(t: &mut Twins, log: u32, pops: u16, ret: u32, arg: bool) -> Vec<u8> {
    let mut c = vec![0x31, 0xc9];
    c.extend(log_stub_arg(t, log, pops, ret, arg));
    c
}

/// `FreeCamera_handleInput` (`0x00680030`) with every callee and Win32
/// import a stub (logging stubs; the window rect, the client origin and
/// the clip rect written or recorded by value), over the toggles, the
/// capture counter at 0 (capture) and above (release), the mouse-free
/// word, the key held or not, the cursor shown or not, the pause object
/// present, paused or not, windowed or not.
#[test]
#[allow(clippy::too_many_lines)]
fn wp5b_free_camera_handle_input() {
    use ffx_prender::wp5_camera::{self as fc, at};
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let cells = t.alloc(&[0; 0x40]);
        let (pause_cell, key_cell, clip_seen) = (cells, cells + 4, cells + 0x10);
        let pause_obj = t.alloc(&[0; 0x700]);
        let hwnd_holder = t.alloc(&0x0000_ab00u32.to_le_bytes());
        // objects the getters return, and logging stubs for the rest
        for (sym, pops, ret, arg, this) in [
            (at::MESSAGE, 0u16, 0x6e00_0000u32, false, false),
            (at::MESSAGE_SHOW, 4, 1, true, true),
            (at::SOUND, 0, 0x6f00_0000, false, false),
            (at::SOUND_PLAY, 4, 2, true, true),
            (at::SOUND_STOP, 4, 3, true, true),
            (at::PAUSE_TOGGLE, 0, 4, false, true),
            (at::INPUT, 0, 0x6200_0000, false, false),
            (at::INPUT_WINDOW, 0, hwnd_holder, false, true),
            (at::INPUT_SET_MOUSE, 8, 5, true, true),
            (at::INPUT_CAMERA_STEP, 0x10, 6, true, true),
            (at::CONTROLLER_UPDATE, 4, 7, true, true),
            (at::CAMERA_FOLLOW, 4, 8, true, true),
            (at::MATRIX_UPDATE, 0, 9, false, true),
        ] {
            let s = if this {
                log_stub_arg(&mut t, log, pops, ret, arg)
            } else {
                log_stub_plain(&mut t, log, pops, ret, arg)
            };
            jmp_stub(&mut t, sym, &s);
        }
        // the pause getter and the key test return words from cells
        let mut c = vec![0xa1];
        c.extend(pause_cell.to_le_bytes()); // mov eax, [pause_cell]
        c.push(0xc3);
        jmp_stub(&mut t, at::PAUSE, &c);
        let mut c = vec![0xa1];
        c.extend(key_cell.to_le_bytes()); // mov eax, [key_cell]
        c.push(0xc3);
        jmp_stub(&mut t, at::KEY_HELD, &c);
        // imports: ShowCursor and SetCursorPos log; GetClientRect and
        // ClientToScreen write; ClipCursor records the rect's words
        let show = log_stub_plain(&mut t, log, 4, 0x10, true);
        let show = t.code(&show);
        let set_pos = log_stub_plain(&mut t, log, 8, 0x11, true);
        let set_pos = t.code(&set_pos);
        // mov eax,[esp+8]; mov dword [eax],1; mov dword [eax+4],2;
        // mov dword [eax+8],641; mov dword [eax+0xc],482; ret 8
        let rect = t.code(&[
            0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 1, 0, 0, 0, 0xc7, 0x40, 0x04, 2, 0, 0, 0, 0xc7,
            0x40, 0x08, 0x81, 0x02, 0, 0, 0xc7, 0x40, 0x0c, 0xe2, 0x01, 0, 0, 0xc2, 0x08, 0x00,
        ]);
        // mov eax,[esp+8]; mov dword [eax],100; mov dword [eax+4],50; ret 8
        let origin = t.code(&[
            0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00, 100, 0, 0, 0, 0xc7, 0x40, 0x04, 50, 0, 0, 0, 0xc2,
            0x08, 0x00,
        ]);
        // mov eax,[esp+4]; inc dword [clip_seen]; test eax,eax; jz end;
        // copy 4 words to clip_seen+4..; end: ret 4
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xff, 0x05];
        c.extend(clip_seen.to_le_bytes());
        c.extend([0x85, 0xc0, 0x74, 0x30]);
        for i in 0..4u32 {
            c.extend([0x8b, 0x48, (4 * i) as u8]); // mov ecx, [eax+4i]
            c.extend([0x89, 0x0d]);
            c.extend((clip_seen + 4 + 4 * i).to_le_bytes()); // mov [clip_seen+4+4i], ecx
        }
        c.extend([0x90; 0x30 - 4 * 9]);
        c.extend([0xc2, 0x04, 0x00]);
        let clip = t.code(&c);
        for (slot, f) in [
            (at::SHOW_CURSOR_IAT, show),
            (at::SET_CURSOR_POS_IAT, set_pos),
            (at::GET_CLIENT_RECT_IAT, rect),
            (at::CLIENT_TO_SCREEN_IAT, origin),
            (at::CLIP_CURSOR_IAT, clip),
        ] {
            let s = t.a.rebase(slot);
            t.w32(s, f);
        }
        let settings = t.alloc(&[0; 0x10]);
        t.w32(t.a.rebase(at::GAME_SETTINGS), settings);
        let frame = t.b.stack_range().start + 0x1000;
        for case in 0..96u32 {
            let cam = t.alloc(&[0x5c; 0x340]);
            t.write(
                cam + 0x338,
                &[
                    u8::from(case & 1 != 0),
                    u8::from(case & 2 != 0),
                    u8::from(case & 4 != 0),
                    u8::from(case & 8 != 0),
                ],
            );
            let count = [0u32, 1, 2, 0xffff_ffff][(case % 4) as usize];
            t.w32(cam, count);
            t.w32(cam + 4, if case & 16 != 0 { count } else { 0 });
            t.w32(cam + 8, 0x3f00_0000);
            t.w32(t.a.rebase(at::MOUSE_FREE), u32::from(case % 7 == 0));
            t.w32(key_cell, u32::from(case % 3 != 0));
            t.w32(pause_cell, if case % 5 == 0 { 0 } else { pause_obj });
            t.write(pause_obj + 0x6fc, &[u8::from(case & 32 != 0)]);
            t.w32(settings + 8, u32::from(case & 64 != 0));
            t.w32(log, 0);
            let w = format!("{label} free camera case {case}");
            let (dt, x) = (0x3c88_8889u32, 0x1234 + case);
            let at_ = t.a.rebase(0x0068_0030);
            t.a.call(at_, Conv::Thiscall(cam), &[dt, x])
                .unwrap_or_else(|e| panic!("{w}: the original: {e}"));
            fc::handle_input(&mut G(&mut t.b), cam, dt, x, frame);
            t.same(&w);
        }
    }
}

/// `FreeCamera_setType`, `asyncFFXInitialize`, the character instance's
/// mesh and cloth teardowns, `spendSlv`: callees logging stubs.
#[test]
#[allow(clippy::too_many_lines)]
fn wp5b_small_originals_2() {
    use ffx_prender::wp5_camera::{self as fc, at};
    use ffx_prender::wp5_small as sm;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let show = log_stub_plain(&mut t, log, 4, 0x10, true);
        let show = t.code(&show);
        let clip = log_stub_plain(&mut t, log, 4, 0x12, true);
        let clip = t.code(&clip);
        for (slot, f) in [(at::SHOW_CURSOR_IAT, show), (at::CLIP_CURSOR_IAT, clip)] {
            let s = t.a.rebase(slot);
            t.w32(s, f);
        }
        for (k, ty) in [0u32, 1, 2, 0, 0].into_iter().enumerate() {
            let cam = t.alloc(&[0x5c; 0x340]);
            t.write(cam + 0x33a, &[u8::from(k % 2 == 1)]);
            t.w32(log, 0);
            let w = format!("{label} free camera type {ty} ({k})");
            t.run(&w, 0x0068_0410, cam, &[ty], false, |g| {
                fc::set_type(g, cam, ty)
            });
        }
        // the callees: logging stubs
        for (sym, pops, ret, arg, this) in [
            (sm::ASYNC_INIT_STEP, 0u16, 0x5150u32, true, false),
            (sm::SCENE_DELETE_MESH_INSTANCE, 8, 0x42, true, true),
            (sm::CLOTH_RELEASE, 4, 0x43, true, true),
            (sm::OPERATOR_DELETE, 0, 0x44, true, false),
        ] {
            let s = if this {
                log_stub_arg(&mut t, log, pops, ret, arg)
            } else {
                log_stub_plain(&mut t, log, pops, ret, arg)
            };
            jmp_stub(&mut t, sym, &s);
        }
        // the clamp: min(max(v, lo), hi) as the original's (logged too)
        let s = log_stub_plain(&mut t, log, 0, 0, true);
        let mut c = s[..s.len() - 6].to_vec();
        // mov eax,[esp+4]; cmp eax,[esp+0xc]; jle +4; mov eax,[esp+0xc]; ret
        c.extend([
            0x8b, 0x44, 0x24, 0x04, 0x3b, 0x44, 0x24, 0x0c, 0x7e, 0x04, 0x8b, 0x44, 0x24, 0x0c,
            0xc3,
        ]);
        jmp_stub(&mut t, sm::CLAMP, &c);
        let scene = t.a.rebase(sm::PHYRE_SCENE);
        t.w32(scene, 0x5ce0_0000);
        for round in 0..6u32 {
            let (out, a, b) = (
                t.alloc(&[0x11; 4]),
                t.alloc(&[0x22; 4]),
                t.alloc(&[0x33; 4]),
            );
            t.w32(log, 0);
            let w = format!("{label} async init {round}");
            t.run_cdecl(&w, 0x0063_6410, &[out, a, b], |g| {
                sm::async_ffx_initialize(g, out, a, b)
            });
            let n = round;
            let items = t.alloc(
                &(0..8u32)
                    .flat_map(|i| (0x7000 + i).to_le_bytes())
                    .collect::<Vec<u8>>(),
            );
            let cloths: Vec<u8> = (0..8u32)
                .flat_map(|i| (if i % 2 == 0 { 0x9000 + i } else { 0 }).to_le_bytes())
                .collect();
            let cloths = t.alloc(&cloths);
            let obj = t.alloc(&[0; 0x10]);
            let flag = if round % 2 == 0 { 0 } else { 0x8000_0000 };
            t.w32(obj, n | flag);
            t.w32(obj + 4, items);
            t.w32(obj + 8, n | flag);
            t.w32(obj + 0xc, cloths);
            t.w32(log, 0);
            let w = format!("{label} destroy meshes {n}");
            t.run_any_this(&w, 0x006a_15e0, obj, |g| {
                sm::character_destroy_meshes(g, obj);
            });
            t.w32(log, 0);
            let w = format!("{label} destroy cloths {n}");
            t.run_any_this(&w, 0x006a_1590, obj, |g| {
                sm::character_destroy_cloths(g, obj);
            });
        }
        let level = t.a.rebase(sm::SLV_LEVEL);
        let gauge = t.a.rebase(sm::SLV_GAUGE);
        for (who, lv, g0, n) in [
            (0u32, 5u8, 10u8, 3u32),
            (1, 2, 0x60, 3),
            (0x102, 7, 0x64, 7),
            (3, 0, 0, 0),
            (4, 0x80, 1, 2),
            (5, 0xff, 0x65, 0xff),
            (6, 1, 1, 0xffff_ffff),
        ] {
            let e = (who & 0xff) * 0x94;
            t.write(level + e, &[lv, g0]);
            let _ = gauge;
            t.w32(log, 0);
            let w = format!("{label} spendSlv {who} {lv} {g0} {n}");
            t.run_cdecl(&w, 0x0078_6ef0, &[who, n], |g| sm::spend_slv(g, who, n));
        }
    }
}

/// `NamedPair_copy` (names NULL, flagged, empty, long; the Phyre
/// allocator a bump stub that fails once) and the map-load report (on,
/// off; the report a stub that records the buffer's first word and the
/// tag's).
#[test]
fn wp5b_small_originals_3() {
    use ffx_prender::wp5_small as sm;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let heap = t.alloc(&[0x66; 0x2000]);
        let next = t.alloc(&heap.to_le_bytes());
        // mov eax,[next]; mov ecx,[esp+4]; add ecx,eax; mov [next],ecx; ret
        let mut c = vec![0xa1];
        c.extend(next.to_le_bytes());
        c.extend([0x8b, 0x4c, 0x24, 0x04, 0x01, 0xc1, 0x89, 0x0d]);
        c.extend(next.to_le_bytes());
        c.push(0xc3);
        jmp_stub(&mut t, sm::PHYRE_ALLOC, &c);
        for (k, name) in [
            None,
            Some(&b"abc"[..]),
            Some(b""),
            Some(b"a longer name, over sixteen"),
            Some(b"x"),
        ]
        .into_iter()
        .enumerate()
        {
            let p = name.map_or(0, |n| {
                let mut v = n.to_vec();
                v.extend([0, 0x77, 0x77]);
                t.alloc(&v)
            });
            let flagged = if k % 2 == 1 { p | 1 } else { p };
            let src = t.alloc(&[0; 8]);
            t.w32(src, 0x1234_0000 + k as u32);
            t.w32(src + 4, flagged);
            let this = t.alloc(&[0x5a; 8]);
            if k == 4 {
                t.w32(next, 0); // the allocator fails: NULL
            }
            let w = format!("{label} named pair copy {k}");
            t.run(&w, 0x0069_9b70, this, &[src], false, |g| {
                sm::named_pair_copy(g, this, src)
            });
        }
        // the report: logs its first word and the tag's first word
        // mov eax,[esp+8]; mov eax,[eax]; mov [log+4],eax; mov eax,[esp+0xc];
        // mov eax,[eax]; mov [log+8],eax; mov eax,[esp+4]; mov [log+0xc],eax;
        // inc dword [log]; ret 0x10
        let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x8b, 0x00, 0xa3];
        c.extend((log + 4).to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x00, 0xa3]);
        c.extend((log + 8).to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x04, 0xa3]);
        c.extend((log + 0xc).to_le_bytes());
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes());
        c.extend([0xc2, 0x10, 0x00]);
        jmp_stub(&mut t, sm::REPORT, &c);
        let frame = t.b.stack_range().start + 0x1000;
        for (a, b) in [(1u8, 1u8), (0, 1), (1, 0), (2, 0xff)] {
            let this = t.alloc(&[a, b, 0, 0]);
            let w = format!("{label} report map load {a} {b}");
            t.run_any_this1(&w, 0x006d_cd50, this, 0x4242, |g| {
                sm::report_map_load(g, this, 0x4242, frame);
            });
        }
    }
}

/// The character-switch tweak tool: its constructor on junk (the member
/// constructor a logging stub) and `CreateInstance` (already there, made,
/// the allocation failing).
#[test]
fn wp5b_char_switch_tweak_util() {
    use ffx_prender::wp5_small as sm;
    for (label, mut t) in Twins::all() {
        let log = t.alloc(&[0; 4 + 8 * 64]);
        let s = log_stub_arg(&mut t, log, 0, 0x55, false);
        jmp_stub(&mut t, sm::CHAR_SWITCH_MEMBER_CTOR, &s);
        for round in 0..3u32 {
            let obj = t.alloc(&[0x70 + round as u8; 0xf8]);
            t.w32(log, 0);
            let w = format!("{label} char switch ctor {round}");
            t.run(&w, 0x006b_55d0, obj, &[], false, |g| {
                sm::char_switch_ctor(g, obj)
            });
        }
        let g = t.a.rebase(sm::CHAR_SWITCH);
        // made (the stub allocator), then already there
        t.w32(g, 0);
        t.run_cdecl(&format!("{label} create"), 0x006b_5eb0, &[], |g| {
            sm::char_switch_create(g)
        });
        // the allocation failing: operator new returning NULL
        let s = log_stub_plain(&mut t, log, 0, 0, true);
        jmp_stub(&mut t, sm::OPERATOR_NEW, &s);
        t.w32(g, 0);
        t.w32(log, 0);
        t.run_cdecl(
            &format!("{label} create, no memory"),
            0x006b_5eb0,
            &[],
            |g| sm::char_switch_create(g),
        );
    }
}

/// `VideoIndexMap_insertNode_006d5ff0`: the video map grown from empty
/// with nodes made by `VideoIndexMap_buyNode` (ids from a small range with
/// negative ones, so equal ids free their node and path), `leftish` 0
/// and 1; the map's `_Insert_at` and iterator run as the original on
/// both sides.
#[test]
fn wp5b_video_insert_node() {
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        for round in 0..6u32 {
            let head = t.alloc(&[0; 0x30]);
            let mut h = Vec::new();
            for _ in 0..3 {
                h.extend(head.to_le_bytes());
            }
            h.extend([1, 1, 0, 0]);
            t.write(head, &h);
            let tree = t.alloc(&[0; 8]);
            t.w32(tree, head);
            let out = t.alloc(&[0x33; 8]);
            let frame = t.b.stack_range().start + 0x1000;
            for i in 0..30u32 {
                let id = (t.rng.below(if round % 2 == 0 { 8 } else { 1000 })).wrapping_sub(3);
                let l = t.rng.len();
                let mut path = t.text(l);
                path.push(0);
                let p = t.alloc(&path);
                let mut v = id.to_le_bytes().to_vec();
                v.extend(p.to_le_bytes());
                let value = t.alloc(&v);
                let node = tb::video_buy_node(&mut G(&mut t.a), tree, value);
                assert_eq!(tb::video_buy_node(&mut G(&mut t.b), tree, value), node);
                let leftish = [0u32, 1, 0x100][t.rng.below(3) as usize];
                let w = format!("{label} video insert {round}.{i} (id {id:#x})");
                t.run(
                    &w,
                    0x006d_5ff0,
                    tree,
                    &[out, leftish, node + 0x10, node],
                    false,
                    |g| tb::video_insert_node(g, tree, out, leftish, node + 0x10, node, frame),
                );
            }
        }
    }
}

/// A red-black tree of `n` random `u32` keys grown by the original
/// `_Insert_nohint` (`0x006995e0`) on both sides; (tree, head, nodes in
/// key order).
fn grown_tree(t: &mut Twins, n: u32) -> (u32, u32, Vec<u32>) {
    let head = t.alloc(&[0; 0x20]);
    let mut h = Vec::new();
    for _ in 0..3 {
        h.extend(head.to_le_bytes());
    }
    h.extend([1, 1, 0, 0]);
    t.write(head, &h);
    let tree = t.alloc(&[0; 8]);
    t.w32(tree, head);
    let out = t.alloc(&[0; 8]);
    let at = t.a.rebase(0x0069_95e0);
    for _ in 0..n {
        let k = t.rng.below(100_000);
        let node = t.alloc(&[0x5c; 0x20]);
        let mut v = Vec::new();
        for _ in 0..3 {
            v.extend(head.to_le_bytes());
        }
        v.extend([0, 0, 0x5c, 0x5c]);
        v.extend(k.to_le_bytes());
        t.write(node, &v);
        let value = t.alloc(&k.to_le_bytes());
        for o in [&mut t.a, &mut t.b] {
            o.call(at, Conv::Thiscall(tree), &[out, 0, value, node])
                .unwrap();
        }
    }
    t.a.clear_dirty();
    t.b.clear_dirty();
    // in order: from the leftmost by the successor links
    let mut nodes = Vec::new();
    let mut nd: u32 = Mem::read(&t.b, head);
    while nd != head {
        nodes.push(nd);
        let r: u32 = Mem::read(&t.b, nd + 8);
        if Mem::read::<u8>(&t.b, r + 0xd) == 0 {
            nd = r;
            while Mem::read::<u8>(&t.b, Mem::read::<u32>(&t.b, nd) + 0xd) == 0 {
                nd = Mem::read(&t.b, nd);
            }
        } else {
            let mut cur = nd;
            let mut p: u32 = Mem::read(&t.b, nd + 4);
            while Mem::read::<u8>(&t.b, p + 0xd) == 0 && cur == Mem::read::<u32>(&t.b, p + 8) {
                cur = p;
                p = Mem::read(&t.b, p + 4);
            }
            nd = p;
        }
    }
    (tree, head, nodes)
}

/// The VFX table's range erase (`0x006a19d0`: the whole tree, a middle
/// run, one node, an empty range) and its one-node erase `0x006a1a70`
/// against `m5b_stl`'s, on grown red-black trees.
#[test]
fn wp5b_tree_erase_range() {
    use ffx_system::m5b_stl::bridge as st;
    use ffx_system::wp5_tree::bridge as tb;
    for (label, mut t) in Twins::all() {
        let frame = t.b.stack_range().start + 0x1000;
        for round in 0..12u32 {
            let n = 1 + t.rng.below(25);
            let (tree, head, nodes) = grown_tree(&mut t, n);
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
            let w = format!("{label} erase range {round} ({n})");
            t.run(&w, 0x006a_19d0, tree, &[out, first, last], false, |g| {
                tb::vfx_erase_range(g, tree, out, first, last, frame)
            });
            // the one-node erase, a node at random
            let n = 1 + t.rng.below(20);
            let (tree, _, nodes) = grown_tree(&mut t, n);
            let wh = nodes[t.rng.below(n) as usize];
            let slot = frame + 0x10;
            t.b.write(slot, &wh.to_le_bytes()).unwrap();
            let w = format!("{label} erase one {round} ({n})");
            t.run(&w, 0x006a_1a70, tree, &[out, wh], false, |g| {
                st::tree_erase(g, tree, out, slot)
            });
        }
    }
}
