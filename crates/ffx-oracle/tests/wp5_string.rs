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
