//! M5 stage 1, list `scene` (`ffx_prender::m5p_scene*`), against the
//! original. Every case runs twice from the same memory: as the original
//! (`FFX.exe` at its address) and as the port. `eax` and every byte
//! written must agree (the oracle also checks `esp`, so each port pops
//! what its shim declares). On the image alone and on the snapshot
//! (`FFX_SNAPSHOT`; run once per snapshot). Needs `FFX_GAME_DIR`.
//!
//! None of this code ran in a coverage run, so the inputs are synthetic:
//! callees outside the list are recording stubs at their addresses in
//! both oracles where a test says so (they log their tag, `ecx` and their
//! stack words to memory, so the diff checks every call and its
//! arguments), or the original code on both sides. Each branch gets its
//! own data; the tests say which ones only synthetic data reaches.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::m5p_scene as t;

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
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, true));
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0
            .unwrap_or_else(|| panic!("callee {addr:#x} leaves no float"))
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

/// The log's size in entries.
const LOG_ENTRIES: u32 = 1024;

/// Two oracles started alike, and the log of the recording stubs.
struct Twins {
    a: Oracle,
    b: Oracle,
    /// Started from the snapshot (`FFX_SNAPSHOT`), not the image alone.
    snapshot: bool,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// stack words).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            snapshot,
            log: 0,
            saved: HashMap::new(),
        };
        t.log = t.zeros(4 + 32 * LOG_ENTRIES as usize);
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
            Self::new(
                Oracle::load(&exe).unwrap(),
                Oracle::load(&exe).unwrap(),
                false,
            ),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    true,
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

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    /// The log entries of the last run (the port's side).
    fn logged(&self) -> u32 {
        self.r32(self.log)
    }

    /// The logged word `i` (0 tag, 1 `ecx`, 2.. stack words) of entry `n`.
    fn entry(&self, n: u32, i: u32) -> u32 {
        self.r32(self.log + 4 + 32 * n + 4 * i)
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this` and `n` stack words; pops `pops` words; returns `ret`.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
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

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there),
    /// tagged with `sym`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// Puts back every stubbed original.
    fn unstub(&mut self) {
        let saved: Vec<_> = self.saved.drain().collect();
        for (at, old) in saved {
            self.write(at, &old);
        }
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

    /// Runs the original at `sym` with `conv` and `args` in `a`, and `port`
    /// on `b`, from the same memory (the log emptied first); `eax` and
    /// every byte written must agree. Returns `eax`.
    fn run(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u32,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax, eb, "{what}: eax");
        self.diff(what);
        eb
    }

    /// [`Twins::run`] without the `eax` check: the original's result and
    /// the port's, for the caller to compare (`al`, `edx:eax`, `ST0`), or
    /// a void port whose original leaves only junk in `eax`. Memory must
    /// agree.
    fn run_any<T>(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> T,
    ) -> (ffx_oracle::Ret, T) {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let r = port(&mut G(&mut self.b));
        self.diff(what);
        (ra, r)
    }

    /// [`Twins::run_any`] for a void port: the original leaves no float.
    fn run_void(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G),
    ) {
        let (ra, ()) = self.run_any(what, sym, conv, args, port);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
    }

    /// [`Twins::run_any`] for an `al` result.
    fn run_al(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u8,
    ) -> u8 {
        let (ra, r) = self.run_any(what, sym, conv, args, port);
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax as u8, r, "{what}: al");
        r
    }

    /// A stub at `sym` that logs its tag and `ecx`, then loads float
    /// `table[first stack word]` into `ST0` and pops `pops` words.
    fn float_stub(&mut self, sym: u32, pops: u32, table: u32) {
        // logs, then `mov eax, [esp+4]; fld dword [table + eax*4]; ret`
        let at = self.rebase(sym);
        let log_code = self.stub_code(sym, true, 0, 0, 0);
        let mut c = vec![0xe8];
        let here = self.a.alloc_code(&[0; 32]).unwrap();
        assert_eq!(self.b.alloc_code(&[0; 32]).unwrap(), here);
        c.extend(log_code.wrapping_sub(here + 5).to_le_bytes()); // call log_code
        c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
        c.extend([0xd9, 0x04, 0x85]);
        c.extend(table.to_le_bytes()); // fld dword [table + eax*4]
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.write(here, &c);
        let mut j = vec![0xe9];
        j.extend(here.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// A fresh vtable whose `slot` (byte offset) is a recording stub
    /// (thiscall, tagged `0x5100_0000 | slot`, `n` words logged and
    /// popped, returning `ret`).
    fn vtable_with_slot(&mut self, slot: u32, n: u32, ret: u32) -> u32 {
        let code = self.stub_code(0x5100_0000 | slot, true, n, n, ret);
        let vt = self.zeros(0x80);
        self.w32(vt + slot, code);
        vt
    }
}

/// A small deterministic generator for the synthetic cases.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
}

// ---- batch 1: the converter table, the destructor tails ----

/// An empty tree head: the root slot (`+0xc`) points at the head.
fn empty_tree(tw: &mut Twins, head: u32) {
    tw.write(head, &[0; 0x10]);
    tw.w32(head + 0xc, head);
}

/// `0x00477710` (attach) and `0x00477150` (find) on synthetic trees: keys
/// from a small range, so first words tie and whole keys repeat (the
/// found path), node colour words with bit 0 clear and set; the insert
/// fix-up runs as the original on both sides between attaches, so the
/// trees grow balanced. Then finds of present and absent keys, and of an
/// empty tree with a key pointer that can't be read (the original doesn't
/// read it then).
#[test]
fn m5p_scene_tree_attach_find() {
    for (what, mut tw) in Twins::all() {
        let fixup = tw.rebase(t::TREE_FIXUP);
        for round in 0..3u64 {
            let mut rng = Rng(round * 77 + 5);
            let head = tw.zeros(0x10);
            empty_tree(&mut tw, head);
            let mut keys = Vec::new();
            for i in 0..80 {
                let node = tw.zeros(0x18);
                let k = [rng.next() % (4 + 3 * round as u32), rng.next() % 3];
                let colour = rng.next() & 0xff;
                tw.write(node, &[0xcc; 0x18]);
                tw.w32(node + 8, colour);
                tw.w32(node + 0xc, k[0]);
                tw.w32(node + 0x10, k[1]);
                let label = format!("{what} attach r{round} #{i} key {k:x?} colour {colour:#x}");
                let r = tw.run(&label, t::ATTACH, Conv::Thiscall(head), &[node], |g| {
                    t::bridge::tree_attach(g, head, node)
                });
                if r == 0 {
                    keys.push(k);
                    tw.a.call(fixup, Conv::Thiscall(head), &[node]).unwrap();
                    tw.b.call(fixup, Conv::Thiscall(head), &[node]).unwrap();
                    tw.diff(&format!("{label} fix-up"));
                }
            }
            assert!(keys.len() > 4, "{what}: the trees grow");
            let key = tw.zeros(8);
            for q in 0..40u32 {
                let k = [q / 3, q % 3 + u32::from(q > 30) * 5];
                tw.w32(key, k[0]);
                tw.w32(key + 4, k[1]);
                let label = format!("{what} find r{round} {k:x?}");
                let r = tw.run(&label, t::FIND, Conv::Thiscall(head), &[key], |g| {
                    t::bridge::tree_find(g, head, key)
                });
                assert_eq!(r != 0, keys.contains(&k), "{label}");
            }
        }
        let head = tw.zeros(0x10);
        empty_tree(&mut tw, head);
        for key in [0, 0xffff_fff0] {
            let label = format!("{what} find in an empty tree, key at {key:#x}");
            let r = tw.run(&label, t::FIND, Conv::Thiscall(head), &[key], |g| {
                t::bridge::tree_find(g, head, key)
            });
            assert_eq!(r, 0, "{label}");
        }
    }
}

/// Stubs for the twelve type getters (cdecl, no arguments; each returns
/// its own address with the top bits flipped) and `atexit`.
fn stub_getters(tw: &mut Twins) {
    for r in t::CONVERTERS {
        for g in r.types {
            if !tw.saved.contains_key(&tw.rebase(g)) {
                tw.stub(g, false, 0, 0, g ^ 0x5a00_0000);
            }
        }
    }
    tw.stub(t::ATEXIT, false, 1, 0, 0);
}

/// Every registration: built (guard clear) into an empty tree, again with
/// the node in the tree (the found path: no fix-up), and again with the
/// guard set but the tree emptied (attach and fix-up only), and once more
/// with the guards clear but the nodes in the tree (rebuilt, then found:
/// the found path sees the rebuilt `+8`). The fix-up is
/// the original on both sides; the getters and `atexit` are recording
/// stubs. The snapshots never ran them (guards clear, the game's tree
/// empty): the same cases on the game's other state.
#[test]
fn m5p_scene_converter_registrations() {
    for (what, mut tw) in Twins::all() {
        stub_getters(&mut tw);
        let tree = tw.rebase(t::TREE);
        if !tw.snapshot {
            empty_tree(&mut tw, tree);
        }
        let mut found = 0;
        for pass in 0..4 {
            if pass == 2 {
                empty_tree(&mut tw, tree);
            }
            for (i, r) in t::CONVERTERS.iter().enumerate() {
                let guard = tw.rebase(r.guard);
                if pass == 0 || pass == 3 {
                    // only bit 0 counts
                    tw.w32(guard, [0, 2, 0x8000_0000, 0xffff_fffe][i % 4]);
                }
                let built = tw.r32(guard) & 1 != 0;
                let label = format!("{what} register {:#x} pass {pass}", r.addr);
                let eax = tw.run(&label, r.addr, Conv::Cdecl, &[], |g| {
                    t::bridge::register(g, r)
                });
                let n = tw.logged();
                if built {
                    assert_eq!(n, 0, "{label}: calls");
                } else {
                    assert_eq!(n, 3, "{label}: calls");
                    assert_eq!(tw.entry(0, 0), r.types[0], "{label}");
                    assert_eq!(tw.entry(1, 0), r.types[1], "{label}");
                    assert_eq!(tw.entry(2, 0), t::ATEXIT, "{label}");
                    assert_eq!(tw.entry(2, 2), tw.rebase(r.dtor), "{label}");
                }
                let entry = tw.rebase(r.entry);
                // attach resets a node's children before it walks, so a
                // second pass cuts the subtrees below each node found:
                // later nodes are found or attached again
                if eax == entry {
                    found += 1;
                }
                assert_eq!(tw.r32(guard) & 1, 1, "{label}: guard");
                assert_eq!(tw.r32(entry + 0x14), tw.rebase(r.convert), "{label}");
            }
        }
        eprintln!("{what}: {found} registrations found their node");
        assert!(found > 0, "{what}: the found path runs");
        tw.unstub();
    }
}

/// `0x0047b630`: the first call (flag clear) runs every registration in
/// its order (the log holds the getters and `atexit` of all 144), then
/// lookups of every pair and of pairs that aren't there, with the flag
/// set. The snapshots never looked a converter up (flag clear, tree
/// empty, nodes zero), so they run the same synthetic cases on the
/// game's other state.
#[test]
fn m5p_scene_converter_lookup() {
    for (what, mut tw) in Twins::all() {
        let flag = tw.rebase(t::REGISTERED);
        let tree = tw.rebase(t::TREE);
        // the snapshots never ran it either: the flag is clear, the tree
        // empty
        assert_eq!(Mem::read::<u8>(&tw.b, flag), 0, "{what}: flag");
        {
            stub_getters(&mut tw);
            empty_tree(&mut tw, tree);
            let label = format!("{what} lookup, first call");
            let k = [0x4321, 0x8765];
            let r = tw.run(&label, t::CONVERTER_FIND, Conv::Cdecl, &k, |g| {
                t::bridge::converter_find(g, k[0], k[1])
            });
            assert_eq!(r, 0, "{label}");
            assert_eq!(tw.logged(), 3 * 144, "{label}: calls");
            for (i, f) in t::REGISTER_ORDER.iter().enumerate() {
                let r = t::converter(*f);
                assert_eq!(tw.entry(3 * i as u32 + 2, 2), tw.rebase(r.dtor), "{label}");
            }
            assert_eq!(Mem::read::<u8>(&tw.b, flag), 1, "{label}: flag");
            // any value but 0 counts as done
            tw.write(flag, &[0x80]);
        }
        let mut rng = Rng(3);
        let mut found = 0;
        for r in t::CONVERTERS {
            let entry = tw.rebase(r.entry);
            let k = [tw.r32(entry + 0xc), tw.r32(entry + 0x10)];
            let wrong = [k[0], k[1] ^ (1 + rng.next() % 0x40)];
            for k in [k, wrong, [k[1], k[0]]] {
                let label = format!("{what} lookup {k:x?}");
                let res = tw.run(&label, t::CONVERTER_FIND, Conv::Cdecl, &k, |g| {
                    t::bridge::converter_find(g, k[0], k[1])
                });
                found += u32::from(res != 0);
            }
        }
        assert!(found >= 144, "{what}: {found} lookups found");
        tw.unstub();
    }
}

/// The destructor tails: `Destruct` is a recording stub (thiscall, logs
/// `ecx`); each copy stores its own vtable.
#[test]
fn m5p_scene_vt_tails() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::DESTRUCT, true, 0, 0, 0x0bad_0001);
        let obj = tw.zeros(0x40);
        for &(f, vt) in t::VT_TAILS {
            tw.write(obj, &[0x55; 0x40]);
            let label = format!("{what} dtor {f:#x}");
            let r = tw.run(&label, f, Conv::Thiscall(obj), &[], |g| {
                t::bridge::vt_tail_destruct(g, obj, vt)
            });
            assert_eq!(r, 0x0bad_0001, "{label}");
            assert_eq!(tw.logged(), 1, "{label}: calls");
            assert_eq!(
                (tw.entry(0, 0), tw.entry(0, 1)),
                (t::DESTRUCT, obj),
                "{label}"
            );
            assert_eq!(tw.r32(obj), tw.rebase(vt), "{label}: vtable");
        }
        tw.unstub();
    }
}

/// The lookups the shims use find each copy's own row, and the tables
/// hold each copy once.
#[test]
fn m5p_scene_tables() {
    for r in t::CONVERTERS {
        assert_eq!(t::converter(r.addr).addr, r.addr);
    }
    for &(f, vt) in t::VT_TAILS {
        assert_eq!(t::vt_tail(f), vt);
    }
    let mut order = t::REGISTER_ORDER.to_vec();
    order.sort_unstable();
    let mut all: Vec<u32> = t::CONVERTERS.iter().map(|r| r.addr).collect();
    all.sort_unstable();
    assert_eq!(order, all, "every registration runs once");
    assert_eq!(t::CONVERTERS.len(), 144);
    assert_eq!(t::VT_TAILS.len(), 34);
}

// ---- batch 2: the small methods (ffx_prender::m5p_scene_b) ----

use ffx_prender::m5p_scene_b as sb;

/// Array headers every clear and setter sees: counts with bit 31 clear and
/// set, 0, 1 and more elements.
const COUNTS: [u32; 8] = [
    0,
    1,
    2,
    5,
    0x7fff_ffff,
    0x8000_0000,
    0x8000_0003,
    0xffff_ffff,
];

/// The array clears: every count, a null and a real buffer; `PhyreFree`
/// and the element releases are recording stubs.
#[test]
fn m5p_scene_b_array_clears() {
    type Clear = fn(&mut G, u32) -> u32;
    for (what, mut tw) in Twins::all() {
        tw.stub(sb::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(sb::PAIRS_RELEASE, false, 2, 0, 0x0bad_0003);
        tw.stub(sb::SEGMENT_CONTEXTS_RELEASE, false, 2, 0, 0x0bad_0004);
        let hdr = tw.zeros(0x10);
        let clears: [(u32, Clear); 11] = [
            (0x004c_97e0, |g, h| sb::bridge::array_clear_checked(g, h)),
            (0x0054_7440, |g, h| sb::bridge::array_clear_checked(g, h)),
            (0x0054_7470, |g, h| sb::bridge::array_clear_checked(g, h)),
            (0x004e_37c0, |g, h| sb::bridge::array_clear(g, h)),
            (0x004f_29c0, |g, h| sb::bridge::array_clear(g, h)),
            (0x004e_37f0, |g, h| {
                sb::bridge::array_clear_release(g, h, sb::PAIRS_RELEASE)
            }),
            (0x004f_2a20, |g, h| {
                sb::bridge::array_clear_release(g, h, sb::SEGMENT_CONTEXTS_RELEASE)
            }),
            (0x0057_7f20, |g, h| sb::bridge::sharray_clear(g, h)),
            (0x005a_2880, |g, h| sb::bridge::sharray_clear(g, h)),
            (0x005a_28c0, |g, h| sb::bridge::sharray_clear_checked(g, h)),
            (0x0044_3f80, |g, h| sb::bridge::array_clear_at8(g, h)),
        ];
        for &(f, port) in &clears {
            for count in COUNTS {
                for buffer in [0, 0x0012_3456] {
                    // 0x00443f80's header is at +8
                    let at = if f == 0x0044_3f80 { hdr - 8 } else { hdr };
                    tw.w32(hdr, count);
                    tw.w32(hdr + 4, buffer);
                    let label = format!("{what} clear {f:#x} ({count:#x}, {buffer:#x})");
                    tw.run(&label, f, Conv::Thiscall(at), &[], |g| port(g, at));
                }
            }
        }
        // the global array of PUtilityScripting
        let global = tw.rebase(sb::SCRIPT_ARRAY);
        for count in COUNTS {
            for buffer in [0, 0x0012_3456] {
                tw.w32(global, count);
                tw.w32(global + 4, buffer);
                let label = format!("{what} script array clear ({count:#x}, {buffer:#x})");
                tw.run(&label, 0x005d_6c80, Conv::Cdecl, &[], |g| {
                    sb::bridge::script_array_clear(g)
                });
            }
        }
        tw.unstub();
    }
}

/// `setToExternalBuffer`: negative and other counts, the old buffer the
/// same or another one, owned or external, null.
#[test]
fn m5p_scene_b_set_external() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sb::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(sb::LOD_SET_EXTERNAL, true, 2, 2, 0x0bad_0005);
        let hdr = tw.zeros(0x10);
        let new = 0x0055_0000;
        for f in [0x0045_a110, 0x0054_cac0, 0x005a_8230] {
            for header in COUNTS {
                for old in [0, new, 0x0012_3456] {
                    for count in [0, 3, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe] {
                        tw.w32(hdr, header);
                        tw.w32(hdr + 4, old);
                        let label = format!(
                            "{what} {f:#x} header ({header:#x}, {old:#x}) count {count:#x}"
                        );
                        tw.run(&label, f, Conv::Thiscall(hdr), &[new, count], |g| {
                            if f == 0x005a_8230 {
                                sb::bridge::lod_set_external(g, hdr, new, count)
                            } else {
                                sb::bridge::array_set_external(g, hdr, new, count)
                            }
                        });
                    }
                }
            }
        }
        tw.unstub();
    }
}

/// The destructors: free-unless-bit-0 words, the camera projection's
/// two destructors and its deleting one, the registered types', and the
/// three deleting destructors with a base destructor.
#[test]
fn m5p_scene_b_destructors() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sb::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(sb::OPERATOR_DELETE, false, 1, 0, 0x0bad_0006);
        for d in [sb::LIGHT_TYPE_DTOR, sb::SHADOW_CASTER_TYPE_DTOR] {
            tw.stub(d.release, false, 1, 0, 0x0bad_0007);
        }
        for d in [
            sb::LOCATOR_SHAPE_TYPE_DTOR,
            sb::FLY_CONTROLLER_DTOR,
            sb::VISIBLE_BOUNDS_JOB_DTOR,
        ] {
            tw.stub(
                d.base,
                !d.base_cdecl,
                u32::from(d.base_cdecl),
                0,
                0x0bad_0008,
            );
        }
        let obj = tw.zeros(0x100);
        for word in [0, 0x0012_3456, 0x0012_3457, 0xffff_fffe] {
            tw.w32(obj, word);
            let label = format!("{what} free unless bit 0 {word:#x}");
            tw.run_void(&label, 0x0055_1670, Conv::Thiscall(obj), &[], |g| {
                sb::bridge::free_unless_bit0(g, obj);
            });
            tw.w32(obj + 0xd0, word);
            for f in [0x0045_53e0, 0x0045_6410] {
                let label = format!("{what} camera dtor {f:#x} {word:#x}");
                tw.run_void(&label, f, Conv::Thiscall(obj), &[], |g| {
                    sb::bridge::camera_projection_dtor(g, obj);
                });
            }
            for flags in [0, 1, 2, 0xffff_ffff] {
                let label = format!("{what} camera delete {word:#x} flags {flags:#x}");
                tw.run(&label, 0x0045_55a0, Conv::Thiscall(obj), &[flags], |g| {
                    sb::bridge::camera_projection_delete(g, obj, flags)
                });
            }
        }
        for (f, d) in [
            (0x0049_d880, sb::LIGHT_TYPE_DTOR),
            (0x004c_aa80, sb::SHADOW_CASTER_TYPE_DTOR),
        ] {
            for registered in [0, 1, 0x8000_0000] {
                for index in [0, 3, 7] {
                    for flags in [0, 1, 0xffff_fffe] {
                        tw.w32(obj + 0xc, registered);
                        tw.w32(obj + 0x10, index);
                        let table = tw.rebase(d.table);
                        tw.w32(table + 4 * index, 0x5555_5555);
                        let label = format!(
                            "{what} type delete {f:#x} ({registered:#x}, {index}) flags {flags:#x}"
                        );
                        tw.run(&label, f, Conv::Thiscall(obj), &[flags], |g| {
                            sb::bridge::type_delete(g, obj, flags, &d)
                        });
                    }
                }
            }
        }
        for (f, d) in [
            (0x0054_5450, sb::LOCATOR_SHAPE_TYPE_DTOR),
            (0x005d_6d20, sb::FLY_CONTROLLER_DTOR),
            (0x009f_2860, sb::VISIBLE_BOUNDS_JOB_DTOR),
        ] {
            for flags in [0, 1, 2, 0xffff_ffff] {
                let label = format!("{what} delete {f:#x} flags {flags:#x}");
                tw.run(&label, f, Conv::Thiscall(obj), &[flags], |g| {
                    sb::bridge::deleting_dtor(g, obj, flags, &d)
                });
                let n = tw.logged();
                assert_eq!(n, 1 + (flags & 1), "{label}: calls");
                assert_eq!(tw.entry(0, 0), d.base, "{label}");
            }
        }
        // the jump to a bare `ret`
        tw.run_void(
            &format!("{what} unwind nothing"),
            0x005a_2a90,
            Conv::Cdecl,
            &[],
            |_| {},
        );
        tw.unstub();
    }
}

/// The accessors and setters, in and past their arrays.
#[test]
fn m5p_scene_b_accessors() {
    for (what, mut tw) in Twins::all() {
        let obj = tw.zeros(0x200);
        let words = tw.zeros(0x40);
        for i in 0..16 {
            tw.w32(words + 4 * i, 0x1000 + i);
        }
        tw.w32(obj + 0x10, 0x0040_0000);
        tw.w32(obj + 0xc, 0x0080_0000);
        tw.w32(obj + 0x74, words);
        for i in [0, 1, 7, 0x1000_0000, 0xffff_ffff] {
            let label = format!("{what} element8 {i:#x}");
            tw.run(&label, 0x0045_9980, Conv::Thiscall(obj), &[i], |g| {
                sb::bridge::element8(g, obj, i)
            });
            let label = format!("{what} element64 {i:#x}");
            tw.run(&label, 0x004f_8070, Conv::Thiscall(obj), &[i], |g| {
                sb::bridge::element64(g, obj, i)
            });
        }
        for count in [0, 4, 0x8000_0004, 16] {
            tw.w32(obj + 0x70, count);
            for i in [0, 3, 4, 15, 0xffff_ffff] {
                let label = format!("{what} array word {i:#x} of {count:#x}");
                tw.run(&label, 0x005b_8c30, Conv::Thiscall(obj), &[i], |g| {
                    sb::bridge::array_word(g, obj, i)
                });
            }
        }
        for v in [0, 0x1234_5678, 0xffff_ffff] {
            for (f, offset) in [(0x004f_8ad0, 0x1c), (0x0050_7400, 0xc), (0x0055_8a30, 0xb4)] {
                let label = format!("{what} set {f:#x} {v:#x}");
                tw.run(&label, f, Conv::Thiscall(obj), &[v], |g| {
                    sb::bridge::set_word(g, obj, offset, v)
                });
            }
            let label = format!("{what} set byte {v:#x}");
            let (ra, ()) = tw.run_any(&label, 0x0055_05f0, Conv::Thiscall(obj), &[v], |g| {
                sb::bridge::set_byte40(g, obj, v);
            });
            // `al` is the byte; the rest of `eax` is the caller's
            assert_eq!(ra.eax as u8, v as u8, "{label}");
        }
        // the ring count: start below the end (with a remainder), at it,
        // above it
        for (start, end, stride) in [(0, 100, 7), (8, 8, 3), (50, 10, 4), (3, 0xffff_ffff, 0x10)] {
            tw.w32(obj + 0xc, start);
            tw.w32(obj + 0x10, end);
            tw.w32(obj + 0x14, stride);
            tw.w32(obj + 0x18, 0x0dd0_0dd0);
            let label = format!("{what} ring ({start}, {end}, {stride})");
            let (ra, r) = tw.run_any(&label, 0x009f_3830, Conv::Thiscall(obj), &[], |g| {
                sb::bridge::ring_count(g, obj)
            });
            assert_eq!(
                (u64::from(ra.edx) << 32) | u64::from(ra.eax),
                r,
                "{label}: edx:eax"
            );
        }
        // the light's range against FLT_MAX (as a double)
        for range in [f32::MAX, f32::INFINITY, 1.0, -f32::MAX, f32::NAN, 0.0] {
            tw.write(obj + 0x2c, &range.to_le_bytes());
            let label = format!("{what} range {range}");
            let r = tw.run_al(&label, 0x005b_8d80, Conv::Thiscall(obj), &[], |g| {
                sb::bridge::range_is_infinite(g, obj)
            });
            assert_eq!(
                r,
                u8::from(range.to_bits() == f32::MAX.to_bits()),
                "{label}"
            );
        }
        // the render list's constructor and the fly controller's identity
        tw.write(obj, &[0x77; 0x80]);
        let label = format!("{what} render list ctor");
        tw.run(&label, 0x004d_b1e0, Conv::Thiscall(obj), &[], |g| {
            sb::bridge::render_list_ctor(g, obj)
        });
        tw.write(obj, &[0x77; 0x80]);
        let label = format!("{what} identity");
        tw.run_void(&label, 0x005d_7400, Conv::Thiscall(obj), &[], |g| {
            sb::bridge::identity_at34(g, obj);
        });
    }
}

/// The node owner checks and the holder search over synthetic lists.
#[test]
fn m5p_scene_b_lists() {
    for (what, mut tw) in Twins::all() {
        let node = tw.zeros(0x40);
        let owner = tw.zeros(0x80);
        let items: Vec<u32> = (0..4).map(|_| tw.zeros(0x80)).collect();
        // a chain items[0] -> items[1] -> items[2] through +0x40
        tw.w32(items[0] + 0x40, items[1]);
        tw.w32(items[1] + 0x40, items[2]);
        for case in 0..6 {
            let (o, first, want, next) = match case {
                0 => (0, 0, 0, 0),
                1 => (owner, 0, items[1], 1),
                2 => (owner, items[0], items[3], 1),
                3 => (owner, items[0], items[2], 0),
                4 => (owner, items[0], items[1], 5),
                _ => (owner, items[0], items[0], 0xffff_ffff),
            };
            tw.w32(node + 0x18, o);
            tw.w32(node + 0x1c, want);
            tw.w32(owner + 0x2c, first);
            tw.w32(owner + 0x40, next);
            for f in [0x0044_3ae0, 0x009f_3a10] {
                let label = format!("{what} owner lists {f:#x} case {case}");
                tw.run_al(&label, f, Conv::Thiscall(node), &[], |g| {
                    sb::bridge::owner_lists(g, node)
                });
            }
        }
        // holders: head -> e0 -> e1; e0's chain empty, e1's items[0..3]
        let head = tw.zeros(0x10);
        let e: Vec<u32> = (0..2).map(|_| tw.zeros(0x10)).collect();
        tw.w32(head, e[0]);
        tw.w32(e[0], e[1]);
        tw.w32(e[1] + 8, items[0]);
        let out = tw.zeros(8);
        for x in [items[0], items[2], items[3], 0] {
            let label = format!("{what} holder of {x:#x}");
            tw.run(&label, 0x0045_01d0, Conv::Thiscall(head), &[out, x], |g| {
                sb::bridge::find_holder(g, head, out, x)
            });
        }
        tw.w32(head, 0);
        let label = format!("{what} holder in an empty list");
        tw.run(&label, 0x0045_01d0, Conv::Thiscall(head), &[out, 5], |g| {
            sb::bridge::find_holder(g, head, out, 5)
        });
    }
}

/// Writes a binary search tree of `keys` (one-word keys at `+0xc`,
/// children `+0`/`+4`, the head the sentinel) under `head`.
fn one_key_tree(tw: &mut Twins, head: u32, keys: &[u32]) {
    tw.write(head, &[0; 0x10]);
    tw.w32(head + 0xc, head);
    for &k in keys {
        let n = tw.zeros(0x20);
        tw.w32(n, head);
        tw.w32(n + 4, head);
        tw.w32(n + 0xc, k);
        tw.w32(n + 0x10, k ^ 0x00ff_0000);
        let mut slot = head + 0xc;
        loop {
            let cur = tw.r32(slot);
            if cur == head {
                tw.w32(slot, n);
                break;
            }
            let ck = tw.r32(cur + 0xc);
            slot = if k < ck { cur } else { cur + 4 };
        }
    }
}

/// The one-word-key tree find and the entity class lookup through it.
#[test]
fn m5p_scene_b_one_key_tree() {
    for (what, mut tw) in Twins::all() {
        let head = tw.zeros(0x10);
        let keys = [50, 20, 80, 10, 30, 70, 90, 25, 0, 0xffff_ffff];
        one_key_tree(&mut tw, head, &keys);
        let key = tw.zeros(4);
        for q in [50, 25, 0, 0xffff_ffff, 26, 85, 5, 0x8000_0000] {
            tw.w32(key, q);
            let label = format!("{what} find1 {q:#x}");
            let r = tw.run(&label, 0x0044_e8c0, Conv::Thiscall(head), &[key], |g| {
                sb::bridge::tree_find1(g, head, key)
            });
            assert_eq!(r != 0, keys.contains(&q), "{label}");
        }
        let empty = tw.zeros(0x10);
        one_key_tree(&mut tw, empty, &[]);
        let label = format!("{what} find1 in an empty tree");
        tw.run(&label, 0x0044_e8c0, Conv::Thiscall(empty), &[0], |g| {
            sb::bridge::tree_find1(g, empty, 0)
        });
        // the class tree: the game's on the snapshot, a synthetic one on
        // the image
        let tree = tw.rebase(0x00c0_bb70);
        if !tw.snapshot {
            one_key_tree(&mut tw, tree, &keys);
        }
        let root = tw.r32(tree + 0xc);
        let root_key = if root == tree { 1 } else { tw.r32(root + 0xc) };
        for q in [root_key, 25, 0, 0xffff_ffff, 26, 0x00b1_0000] {
            let label = format!("{what} class lookup {q:#x}");
            tw.run(&label, 0x0044_f750, Conv::Cdecl, &[q], |g| {
                sb::bridge::entity_class_lookup(g, q)
            });
        }
    }
}

/// Copies and zeroing of items, the context-switch packers.
#[test]
fn m5p_scene_b_items_and_packers() {
    for (what, mut tw) in Twins::all() {
        let src = tw.alloc(&(0..64u8).collect::<Vec<_>>());
        let dst = tw.zeros(0x100);
        for n in [0, 1, 5] {
            tw.write(dst, &[0xee; 0x100]);
            let label = format!("{what} copy items {n}");
            tw.run_void(&label, 0x0045_6da0, Conv::Cdecl, &[dst, src, n], |g| {
                sb::bridge::copy_items8(g, dst, src, n);
            });
            let label = format!("{what} zero items {n}");
            tw.run_void(&label, 0x004d_f180, Conv::Cdecl, &[dst, n], |g| {
                sb::bridge::zero_items20(g, dst, n);
            });
        }
        // a null destination slot: skipped
        let label = format!("{what} copy to a null slot");
        tw.run_void(&label, 0x0045_6da0, Conv::Cdecl, &[0, src, 1], |g| {
            sb::bridge::copy_items8(g, 0, src, 1);
        });
        let label = format!("{what} zero a null slot");
        tw.run_void(&label, 0x004d_f180, Conv::Cdecl, &[0, 1], |g| {
            sb::bridge::zero_items20(g, 0, 1);
        });
        let this = tw.zeros(0x40);
        let cursor = tw.zeros(4);
        let out = tw.zeros(0x40);
        let inst = tw.zeros(0x40);
        for flag in [0u8, 1, 0x80] {
            tw.write(this + 0x14, &[flag]);
            tw.w32(cursor, out);
            let args = [1, 2, 3, cursor, 5, 6, 7];
            let label = format!("{what} pack flag {flag}");
            tw.run(&label, 0x004d_4070, Conv::Thiscall(this), &args, |g| {
                sb::bridge::pack_flag(g, this, cursor)
            });
        }
        for (object, w) in [(0, 0), (inst, 0), (inst, 1), (inst, 0x8000_0000)] {
            tw.w32(inst + 0x18, w);
            tw.w32(cursor, out + 8);
            let args = [1, 2, 3, cursor, 5, object, 7];
            let label = format!("{what} pack instancing {object:#x} {w:#x}");
            tw.run(&label, 0x004d_4090, Conv::Thiscall(this), &args, |g| {
                sb::bridge::pack_instancing(g, cursor, object)
            });
        }
    }
}

/// The forwarders: sub-object tail jumps, the timer's initialize, the
/// fly controller's virtual slots with floats (a quiet NaN included), the
/// orthographic camera's size and copy.
#[test]
fn m5p_scene_b_forwarders() {
    for (what, mut tw) in Twins::all() {
        let fwd: [(u32, u32, u32); 4] = [
            (0x0050_73f0, 0x10, sb::MATRIX_COPY),
            (0x0055_88e0, 0x34, sb::FLY_SET[0]),
            (0x0055_8a50, 0x34, sb::FLY_SET[1]),
            (0x005d_7790, 4, sb::TO_WORLD_MATRIX),
        ];
        for &(_, _, target) in &fwd {
            tw.stub(target, true, 1, 1, target ^ 0x0a00_0000);
        }
        tw.stub(sb::SCRIPTABLE_INITIALIZE, true, 3, 3, 0x0bad_0009);
        let this = tw.zeros(0x200);
        for &(f, offset, target) in &fwd {
            let label = format!("{what} forward {f:#x}");
            tw.run(&label, f, Conv::Thiscall(this), &[0x1234], |g| {
                sb::bridge::forward_sub(g, this, offset, target, 0x1234)
            });
            assert_eq!(tw.entry(0, 1), this + offset, "{label}: this");
        }
        let qnan = 0x7fc0_1234u32;
        for period in [1.5f32.to_bits(), qnan, 0x8000_0000] {
            tw.w32(this + 0x20, period);
            let label = format!("{what} timer initialize {period:#x}");
            tw.run(&label, 0x0055_5d90, Conv::Thiscall(this), &[7, 8, 9], |g| {
                sb::bridge::timer_initialize(g, this, [7, 8, 9])
            });
        }
        // the fly controller at +0x34
        let vt = tw.zeros(0x40);
        for (slot, n) in [(0x1c, 1), (0x20, 2), (0x24, 2)] {
            let code = tw.stub_code(0x5100_0000 | slot, true, n, n, 0x0bad_0100 + slot);
            tw.w32(vt + slot, code);
        }
        tw.w32(this + 0x34, vt);
        for (a, b) in [
            (1.25f32.to_bits(), (-3.0f32).to_bits()),
            (qnan, 0x7f80_0000),
        ] {
            let label = format!("{what} fly slot 7 {a:#x}");
            tw.run(&label, 0x0055_88f0, Conv::Thiscall(this), &[a], |g| {
                sb::bridge::sub_virtual1(g, this, f32::from_bits(a))
            });
            assert_eq!(
                (tw.entry(0, 1), tw.entry(0, 2)),
                (this + 0x34, a),
                "{label}"
            );
            for (f, slot) in [(0x0055_8910, 0x20), (0x0055_89b0, 0x24)] {
                let label = format!("{what} fly slot {f:#x} {a:#x} {b:#x}");
                tw.run(&label, f, Conv::Thiscall(this), &[a, b], |g| {
                    sb::bridge::sub_virtual2(g, this, slot, f32::from_bits(a), f32::from_bits(b))
                });
                assert_eq!((tw.entry(0, 2), tw.entry(0, 3)), (a, b), "{label}");
            }
        }
        // the orthographic camera
        let cam_vt = tw.vtable_with_slot(4, 0, 0x0bad_000a);
        let cam = tw.zeros(0x200);
        tw.w32(cam, cam_vt);
        let p = tw.zeros(4);
        let label = format!("{what} ortho size, null");
        tw.run(&label, 0x0045_5f40, Conv::Thiscall(cam), &[0], |g| {
            sb::bridge::ortho_set_size(g, cam, 0)
        });
        for size in [
            2.5f32.to_bits(),
            0,
            0x8000_0000,
            0x7f80_0000,
            qnan,
            (-3.0f32).to_bits(),
            1,
        ] {
            tw.w32(p, size);
            let label = format!("{what} ortho size {size:#x}");
            tw.run(&label, 0x0045_5f40, Conv::Thiscall(cam), &[p], |g| {
                sb::bridge::ortho_set_size(g, cam, p)
            });
        }
        tw.stub(sb::CAMERA_COPY, true, 1, 1, 0x0bad_000b);
        let src = tw.zeros(0x200);
        tw.w32(src + 0x150, qnan);
        tw.w32(src + 0x154, 4.5f32.to_bits());
        let label = format!("{what} ortho copy");
        tw.run(&label, 0x0045_6430, Conv::Thiscall(cam), &[src], |g| {
            sb::bridge::ortho_copy(g, cam, src)
        });
        assert_eq!(
            (tw.entry(0, 1), tw.entry(0, 2)),
            (cam + 4, src + 4),
            "{label}"
        );
        tw.unstub();
    }
}

/// The utilities' clean-ups and resets, the name lookups, the named
/// entries' constructor, the held object's delete, the timer's
/// uninitialize, the world's script count and the spline length.
#[test]
fn m5p_scene_b_utilities() {
    for (what, mut tw) in Twins::all() {
        for f in sb::RENDERING_TERMINATE {
            tw.stub(f, false, 0, 0, f);
        }
        tw.stub(sb::GAMEPLAY_CLEANUP, false, 0, 0, 0x0bad_000c);
        let label = format!("{what} rendering terminate");
        tw.run(&label, 0x0043_2ac0, Conv::Cdecl, &[], |g| {
            sb::bridge::rendering_terminate(g)
        });
        assert_eq!(tw.logged(), 6, "{label}");
        let label = format!("{what} gameplay terminate");
        tw.run(&label, 0x0043_3650, Conv::Cdecl, &[], |g| {
            sb::bridge::gameplay_terminate(g)
        });
        tw.unstub();
        // the reset runs the real setOverallocation
        let label = format!("{what} render reset");
        tw.run(&label, 0x004e_68d0, Conv::Cdecl, &[], |g| {
            sb::bridge::render_reset(g)
        });
        // names: found and not found
        let out = tw.zeros(4);
        for (f, find) in [
            (0x0054_5e60, sb::FIND_LOCATOR_SHAPE),
            (0x0054_d320, sb::FIND_GAMEPLAY_TYPE),
        ] {
            for ret in [0, 0x0077_0000] {
                tw.stub(find, false, 1, 0, ret);
                let label = format!("{what} {f:#x} finds {ret:#x}");
                tw.run(&label, f, Conv::Cdecl, &[out, 0x4444], |g| {
                    sb::bridge::find_into(g, find, out, 0x4444)
                });
                assert_eq!(tw.entry(0, 2), 0x4444, "{label}");
            }
        }
        tw.unstub();
        // the named list with the real `_stricmp`: three entries (a name in other case finds its entry)
        let head = tw.rebase(sb::NAMED_LIST);
        let names: Vec<u32> = ["alpha", "beta", "gamma", "delta", "BeTa"]
            .iter()
            .map(|s| tw.alloc(format!("{s}\0").as_bytes()))
            .collect();
        let entries: Vec<u32> = (0..3).map(|_| tw.zeros(0x10)).collect();
        for (i, &e) in entries.iter().enumerate() {
            tw.w32(e, names[i]);
            tw.w32(e + 4, entries.get(i + 1).copied().unwrap_or(0));
        }
        let saved = tw.r32(head);
        for list in [entries[0], 0] {
            tw.w32(head, list);
            for &name in names.iter().chain(&[0]) {
                let label = format!("{what} find named {name:#x} in {list:#x}");
                tw.run(&label, 0x0054_d190, Conv::Cdecl, &[name], |g| {
                    sb::bridge::find_named(g, name)
                });
            }
        }
        // a null name never reaches `_stricmp` (a recording stand-in here)
        let cmp = tw.stub_code(0x5700_0000, false, 2, 0, 1);
        let real = tw.a.bind_import("_stricmp", Some(cmp)).unwrap();
        tw.b.bind_import("_stricmp", Some(cmp)).unwrap();
        tw.w32(head, entries[0]);
        let label = format!("{what} find named null, stand-in _stricmp");
        tw.run(&label, 0x0054_d190, Conv::Cdecl, &[0], |g| {
            sb::bridge::find_named(g, 0)
        });
        assert_eq!(tw.logged(), 0, "{label}: calls");
        let label = format!("{what} find named, stand-in _stricmp");
        tw.run(&label, 0x0054_d190, Conv::Cdecl, &[names[3]], |g| {
            sb::bridge::find_named(g, names[3])
        });
        assert_eq!(tw.logged(), 3, "{label}: calls");
        tw.a.bind_import("_stricmp", Some(real)).unwrap();
        tw.b.bind_import("_stricmp", Some(real)).unwrap();
        // the constructor: no name, a listed name, a new one
        let e = tw.zeros(0x10);
        for (name, found) in [(0, 0), (names[1], entries[1]), (names[3], 0)] {
            tw.w32(head, entries[0]);
            tw.stub(sb::FIND_NAMED, false, 1, 0, found);
            let label = format!("{what} named entry {name:#x} found {found:#x}");
            tw.run(
                &label,
                0x0054_d040,
                Conv::Thiscall(e),
                &[0x9999, name],
                |g| sb::bridge::named_entry_ctor(g, e, name),
            );
        }
        tw.w32(head, saved);
        tw.unstub();
        // a name and its size
        let pname = tw.zeros(4);
        let len = tw.zeros(4);
        for &name in &names {
            tw.w32(pname, name);
            let label = format!("{what} name and size {name:#x}");
            tw.run(&label, 0x0054_d2b0, Conv::Cdecl, &[pname, out, len], |g| {
                sb::bridge::name_and_size(g, pname, out, len)
            });
        }
        // the held object
        tw.stub(sb::DELETE_OBJECT, true, 1, 1, 0x0bad_000d);
        let holder = tw.zeros(4);
        for (obj, cluster) in [(0, 0), (0, 0x5000), (0x6000, 0), (0x6000, 0x5000)] {
            tw.w32(holder, obj);
            let label = format!("{what} delete held {obj:#x} from {cluster:#x}");
            tw.run(
                &label,
                0x0051_6cf0,
                Conv::Thiscall(holder),
                &[cluster],
                |g| sb::bridge::delete_held(g, holder, cluster),
            );
        }
        // the timer
        tw.stub(sb::TIMER_RELEASE, false, 1, 0, 0x0bad_000e);
        let timer = tw.zeros(0x40);
        for (t, owned) in [(0, 1u8), (0x7000, 0), (0x7000, 1), (0x7000, 0x80)] {
            tw.w32(timer + 0x1c, t);
            tw.write(timer + 0x28, &[owned]);
            let label = format!("{what} timer uninitialize {t:#x} {owned}");
            tw.run(&label, 0x0055_5ef0, Conv::Thiscall(timer), &[], |g| {
                sb::bridge::timer_uninitialize(g, timer)
            });
        }
        // the world's count of a named class
        tw.stub(sb::GLOBAL_NAMESPACE, false, 0, 0, 0x00ab_c000);
        tw.stub(sb::WORLD_INSTANCE_COUNT, true, 1, 1, 0x0bad_000f);
        let world = tw.zeros(0x20);
        for d in [0, 0x00de_0000] {
            tw.stub(sb::GET_CLASS_DESCRIPTOR, true, 1, 1, d);
            let label = format!("{what} instance count, class {d:#x}");
            tw.run(
                &label,
                0x0044_36a0,
                Conv::Thiscall(world),
                &[names[0]],
                |g| sb::bridge::world_instance_count_for_script(g, world, names[0]),
            );
            assert_eq!(
                (tw.entry(1, 0), tw.entry(1, 1), tw.entry(1, 2)),
                (sb::GET_CLASS_DESCRIPTOR, 0x00ab_c000, names[0]),
                "{label}"
            );
        }
        tw.unstub();
        // the spline: 0, 1 and 5 segments, lengths that round
        let lengths = tw.alloc(
            &[0.1f32, 1.0e7, 3.3, -0.25, 1.0e-3, 7.0]
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        tw.float_stub(sb::SPLINE_SEGMENT_LENGTH, 1, lengths);
        let spline = tw.zeros(0x10);
        for n in [0, 1, 5] {
            tw.stub(sb::SPLINE_SEGMENTS, true, 0, 0, n);
            let label = format!("{what} spline of {n}");
            let (ra, r) = tw.run_any(&label, 0x0054_98c0, Conv::Thiscall(spline), &[], |g| {
                sb::bridge::spline_length(g, spline)
            });
            assert!(ra.st0_is(f64::from(r)), "{label}: ST0 {:?} vs {r}", ra.st0);
            assert_eq!(tw.logged(), 1 + n, "{label}: calls");
        }
        tw.unstub();
    }
}

// ---- batch 3 (ffx_prender::m5p_scene_c) ----

use ffx_prender::m5p_scene_c as sc;

/// The `PUtility*` deleting destructors: the singleton first, later or
/// missing in the registry (an empty one too), each flag.
#[test]
fn m5p_scene_c_utility_dtors() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sc::OPERATOR_DELETE, false, 1, 0, 0x0bad_0006);
        let head = tw.rebase(sc::UTILITY_LIST);
        let nodes: Vec<u32> = (0..3).map(|_| tw.zeros(0x10)).collect();
        for f in [0x0043_2b80, 0x0043_2ce0, 0x0043_3460, 0x0050_41e0] {
            for (case, this) in [(0, 0), (0, 1), (0, 2), (1, 0), (2, 1)] {
                // case 0: nodes 0 -> 1 -> 2; 1: empty; 2: only node 0
                let this = nodes[this];
                tw.w32(head, if case == 1 { 0 } else { nodes[0] });
                tw.w32(nodes[0] + 4, if case == 2 { 0 } else { nodes[1] });
                tw.w32(nodes[1] + 4, nodes[2]);
                tw.w32(nodes[2] + 4, 0);
                for flags in [0, 1] {
                    let label =
                        format!("{what} utility delete {f:#x} case {case} this {this:#x} {flags}");
                    tw.run(&label, f, Conv::Thiscall(this), &[flags], |g| {
                        sc::bridge::utility_delete(g, this, flags)
                    });
                }
            }
        }
        tw.unstub();
    }
}

/// The `PSharray<T*>` buffers: one element or none in the header, more
/// allocated (a failing allocation too), copies with and without zero
/// fill.
#[test]
fn m5p_scene_c_sharray_buffers() {
    for (what, mut tw) in Twins::all() {
        let this = tw.zeros(0x10);
        let src = tw.alloc(&(1..=64u8).collect::<Vec<_>>());
        let block = tw.zeros(0x80);
        for ret in [block, 0] {
            tw.stub(sc::PHYRE_MALLOC, false, 2, 0, ret);
            for n in [0, 1, 2, 7] {
                for f in [0x004f_6db0, 0x005a_6aa0] {
                    tw.write(block, &[0xee; 0x80]);
                    let label = format!("{what} sharray copy {f:#x} n {n} alloc {ret:#x}");
                    tw.run(&label, f, Conv::Thiscall(this), &[n, src], |g| {
                        sc::bridge::sharray_copy(g, this, n, src)
                    });
                }
                for m in [0, 1, n] {
                    if m > n {
                        continue;
                    }
                    for f in [0x004f_6e10, 0x0054_fa00, 0x005a_6ba0] {
                        tw.write(block, &[0xee; 0x80]);
                        tw.w32(this + 4, 0xdddd_dddd);
                        let label =
                            format!("{what} sharray fill {f:#x} n {n} m {m} alloc {ret:#x}");
                        tw.run(&label, f, Conv::Thiscall(this), &[n, src, m], |g| {
                            sc::bridge::sharray_copy_fill(g, this, n, src, m)
                        });
                    }
                }
            }
        }
        // zeroed 32-byte items
        let items = tw.zeros(0x100);
        for n in [0, 1, 3] {
            tw.write(items, &[0xee; 0x100]);
            let label = format!("{what} zero items32 {n}");
            tw.run_void(&label, 0x004d_f240, Conv::Cdecl, &[items, n], |g| {
                sc::bridge::zero_items32(g, items, n);
            });
        }
        let label = format!("{what} zero items32 null");
        tw.run_void(&label, 0x004d_f240, Conv::Cdecl, &[0, 1], |g| {
            sc::bridge::zero_items32(g, 0, 1);
        });
        tw.unstub();
    }
}

/// The array resizes: the shrink, the render pass reallocation and the
/// 8-byte reset, over the header counts and old buffers.
#[test]
fn m5p_scene_c_array_resizes() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sc::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let hdr = tw.zeros(0x10);
        let fresh = 0x0066_0000;
        for header in COUNTS {
            for old in [0, fresh, 0x0012_3456] {
                for n in [0, 1, 3, 0x7fff_ffff, header] {
                    tw.w32(hdr, header);
                    tw.w32(hdr + 4, old);
                    let label = format!("{what} shrink ({header:#x}, {old:#x}) to {n:#x}");
                    tw.run(&label, 0x00a3_07f0, Conv::Thiscall(hdr), &[n], |g| {
                        sc::bridge::array_shrink(g, hdr, n)
                    });
                    for ret in [fresh, 0] {
                        tw.stub(sc::RENDER_PASS_REALLOC, false, 3, 0, ret);
                        tw.stub(sc::PHYRE_MALLOC, false, 2, 0, ret);
                        tw.w32(hdr, header);
                        tw.w32(hdr + 4, old);
                        let label = format!(
                            "{what} render pass resize ({header:#x}, {old:#x}) to {n:#x}, {ret:#x}"
                        );
                        tw.run(&label, 0x004e_74c0, Conv::Thiscall(hdr), &[n], |g| {
                            sc::bridge::render_pass_resize(g, hdr, n)
                        });
                        tw.w32(hdr, header);
                        tw.w32(hdr + 4, old);
                        let label = format!(
                            "{what} array8 reset ({header:#x}, {old:#x}) to {n:#x}, {ret:#x}"
                        );
                        tw.run(&label, 0x00a3_0780, Conv::Thiscall(hdr), &[n], |g| {
                            sc::bridge::array8_reset(g, hdr, n)
                        });
                    }
                }
            }
        }
        // the constant buffers: the destructor is a recording stub
        tw.stub(sc::CONSTANT_BUFFER_DTOR, true, 0, 0, 0x0bad_0010);
        for count in [0, 1, 3, 0x8000_0002] {
            for buffer in [0, 0x0012_3000] {
                tw.w32(hdr, count);
                tw.w32(hdr + 4, buffer);
                let label = format!("{what} constant buffers ({count:#x}, {buffer:#x})");
                tw.run(&label, 0x004f_d160, Conv::Thiscall(hdr), &[], |g| {
                    sc::bridge::constant_buffers_clear(g, hdr)
                });
            }
        }
        tw.unstub();
    }
}

/// The list unlinks: the gameplay types and locator shapes (first, later,
/// missing, empty), the component detach.
#[test]
fn m5p_scene_c_unlinks() {
    for (what, mut tw) in Twins::all() {
        let nodes: Vec<u32> = (0..3).map(|_| tw.zeros(0x10)).collect();
        for (f, l) in [
            (0x0043_3550, sc::GAMEPLAY_TYPES),
            (0x0054_5e80, sc::LOCATOR_SHAPES),
        ] {
            let head = tw.rebase(l.head);
            let saved = tw.r32(head);
            for (empty, node) in [(false, 0), (false, 1), (false, 2), (true, 0)] {
                tw.w32(head, if empty { 0 } else { nodes[0] });
                tw.w32(nodes[0] + l.link, nodes[1]);
                tw.w32(nodes[1] + l.link, 0);
                let node = nodes[node];
                let label = format!("{what} unlink {f:#x} {node:#x} empty {empty}");
                tw.run(&label, f, Conv::Cdecl, &[node], |g| {
                    sc::bridge::list_unlink(g, &l, node)
                });
            }
            tw.w32(head, saved);
        }
        let owner = tw.zeros(0x10);
        for (owned, at) in [(true, 0), (true, 1), (true, 2), (false, 0)] {
            tw.w32(owner, nodes[0]);
            tw.w32(nodes[0], nodes[1]);
            tw.w32(nodes[1], 0);
            let c = nodes[at];
            tw.w32(c + 4, if owned { owner } else { 0x9999 });
            let label = format!("{what} detach {c:#x} owned {owned}");
            tw.run(&label, 0x0045_0450, Conv::Thiscall(owner), &[c], |g| {
                sc::bridge::component_detach(g, owner, c)
            });
        }
        tw.w32(owner, 0);
        tw.w32(nodes[0] + 4, owner);
        let label = format!("{what} detach from an empty list");
        tw.run(
            &label,
            0x0045_0450,
            Conv::Thiscall(owner),
            &[nodes[0]],
            |g| sc::bridge::component_detach(g, owner, nodes[0]),
        );
    }
}

/// The entity's class node (the tree calls stubbed), the world class
/// test and the instances search over synthetic class chains.
#[test]
fn m5p_scene_c_classes() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sc::TREE_REMOVE, true, 1, 1, 0x0bad_0011);
        tw.stub(sc::TREE_FIXUP, true, 1, 1, 0x0bad_0012);
        let entity = tw.zeros(0x40);
        for (had, class, found) in [
            (0, 0, 0),
            (1, 0, 0),
            (0, 0x0077_0000, 0),
            (1, 0x0077_0000, 0x0055_0000),
            (2, 0x0077_0000, 0),
        ] {
            tw.stub(sc::TREE_ATTACH1, true, 1, 1, found);
            tw.write(entity, &[0x44; 0x40]);
            tw.w32(entity + 4, had);
            let label = format!("{what} set class {had} {class:#x} found {found:#x}");
            tw.run(&label, 0x0045_0500, Conv::Thiscall(entity), &[class], |g| {
                sc::bridge::entity_set_class(g, entity, class)
            });
        }
        tw.unstub();
        // classes: c0 -> c1 -> world, c2 alone
        let world = tw.rebase(sc::WORLD_CLASS);
        let c: Vec<u32> = (0..3).map(|_| tw.zeros(0x50)).collect();
        tw.w32(c[0] + 0x40, c[1]);
        tw.w32(c[1] + 0x40, world);
        let list = tw.zeros(0x10);
        let e: Vec<u32> = (0..2).map(|_| tw.zeros(0x10)).collect();
        for (entries, class) in [
            (vec![], 0),
            (vec![], c[2]),
            (vec![], c[0]),
            (vec![c[2]], c[0]),
            (vec![c[2], c[1]], c[0]),
            (vec![c[0]], c[1]),
            (vec![0, c[2]], c[1]),
        ] {
            tw.w32(list, if entries.is_empty() { 0 } else { e[0] });
            tw.w32(e[0], if entries.len() > 1 { e[1] } else { 0 });
            tw.w32(e[1], 0);
            for (i, &k) in entries.iter().enumerate() {
                tw.w32(e[i] + 8, k);
            }
            let label = format!("{what} new world class {class:#x} entries {entries:x?}");
            tw.run_al(&label, 0x0044_ff60, Conv::Thiscall(list), &[class], |g| {
                sc::bridge::world_class_new(g, list, class)
            });
        }
        // instances: three elements whose classes are c2, c0 (-> c1), 0
        let comp = tw.zeros(0x20);
        let items = tw.zeros(0x20);
        for (i, k) in [c[2], c[0], 0].iter().enumerate() {
            tw.w32(items + 8 * i as u32, 0x100 + i as u32);
            tw.w32(items + 8 * i as u32 + 4, *k);
        }
        tw.w32(comp + 0x10, items);
        let out = tw.zeros(8);
        for count in [3, 0x8000_0003, 0, 1] {
            tw.w32(comp + 0xc, count);
            for class in [0, c[1], c[2], world] {
                let label = format!("{what} instances find {class:#x} in {count:#x}");
                tw.run(
                    &label,
                    0x0045_99c0,
                    Conv::Thiscall(comp),
                    &[out, class],
                    |g| sc::bridge::instances_find(g, comp, out, class),
                );
            }
        }
    }
}

/// The scene cursor: its constructor, seek and next over a node ring
/// whose nodes' owners list them or not; `PhyreSceneCursor_SetNode` is a
/// recording stub.
#[test]
fn m5p_scene_c_cursor() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sc::CURSOR_SET_NODE, true, 1, 1, 0x0bad_0013);
        // a ring of three nodes after the owner's +0x1c; each node's
        // first word links on. Ownership: the cursor's +0x1c is matched
        // against the owner's list from +0x2c (owner_lists): nodes are
        // listed when the node itself is the owner (+0x18 = node) whose
        // list (+0x2c) holds the mask.
        let owner = tw.zeros(0x80);
        let ring = owner + 0x1c;
        let nodes: Vec<u32> = (0..3).map(|_| tw.zeros(0x60)).collect();
        tw.w32(ring, nodes[0]);
        tw.w32(nodes[0], nodes[1]);
        tw.w32(nodes[1], nodes[2]);
        tw.w32(nodes[2], ring);
        let mask = tw.zeros(0x50);
        let it = tw.zeros(0x20);
        for listed in [
            [true, true, true],
            [false, true, false],
            [false, false, true],
            [false, false, false],
        ] {
            for (i, &n) in nodes.iter().enumerate() {
                // the cursor's +0x18 is the node: its +0x2c chain and +0x40
                tw.w32(n + 0x2c, if listed[i] { mask } else { 0 });
                tw.w32(n + 0x40, 1);
            }
            let label = format!("{what} cursor ctor {listed:?}");
            tw.run(
                &label,
                0x009f_2470,
                Conv::Thiscall(it),
                &[owner, mask],
                |g| sc::bridge::cursor_ctor(g, it, owner, mask),
            );
            for f in [0x0044_3930, 0x009f_38c0, 0x0044_39a0] {
                for (start, &node) in nodes.iter().enumerate() {
                    // the words a settle at the end clears
                    tw.write(it, &[0x5a; 0x14]);
                    tw.w32(it + 0x14, ring);
                    tw.w32(it + 0x18, node);
                    tw.w32(it + 0x1c, mask);
                    let label = format!("{what} cursor {f:#x} {listed:?} from {start}");
                    tw.run(&label, f, Conv::Thiscall(it), &[], |g| {
                        if f == 0x0044_39a0 {
                            sc::bridge::cursor_next(g, it)
                        } else {
                            sc::bridge::cursor_seek(g, it)
                        }
                    });
                }
            }
        }
        // an empty ring, its head made to look listed (a cursor that took
        // the head for a node would settle on it)
        tw.w32(ring, ring);
        tw.w32(ring + 0x2c, mask);
        tw.w32(ring + 0x40, 1);
        let label = format!("{what} cursor ctor, empty ring");
        tw.run(
            &label,
            0x009f_2470,
            Conv::Thiscall(it),
            &[owner, mask],
            |g| sc::bridge::cursor_ctor(g, it, owner, mask),
        );
        tw.unstub();
    }
}

/// The light/caster pairs, the held name compare, the context switch
/// type constructor, the timer assignment and the receivers setter.
#[test]
fn m5p_scene_c_misc() {
    for (what, mut tw) in Twins::all() {
        let mut lights: Vec<u32> = sc::COMPATIBLE.iter().map(|&(l, _)| tw.rebase(l)).collect();
        let mut casters: Vec<u32> = sc::COMPATIBLE.iter().map(|&(_, c)| tw.rebase(c)).collect();
        lights.push(0x1234);
        casters.push(0);
        for &l in &lights {
            for &c in &casters {
                let label = format!("{what} compatible {l:#x} {c:#x}");
                tw.run_al(&label, 0x004c_ae20, Conv::Cdecl, &[l, c], |g| {
                    sc::bridge::light_caster_compatible(g, l, c)
                });
            }
        }
        // names
        let names: Vec<u32> = ["abc", "abd", "ab", "", "abc"]
            .iter()
            .map(|s| tw.alloc(format!("{s}\0").as_bytes()))
            .collect();
        let holder = tw.zeros(4);
        let pname = tw.zeros(4);
        tw.w32(holder, pname);
        for held in [names[0], 0, names[3]] {
            tw.w32(pname, held);
            for &n in names.iter().chain(&[0]) {
                let label = format!("{what} name is {held:#x} {n:#x}");
                tw.run_al(&label, 0x0055_0430, Conv::Thiscall(holder), &[n], |g| {
                    sc::bridge::name_is(g, holder, n)
                });
            }
        }
        // the switch type constructor
        let head = tw.rebase(sc::SWITCH_TYPES[0]);
        let saved = [tw.r32(head), tw.r32(head + 8)];
        let ty = tw.zeros(0x20);
        for (name, found) in [(0, 0), (names[0], 0x4444), (names[1], 0)] {
            tw.stub(sc::SWITCH_TYPE_FIND, false, 1, 0, found);
            let label = format!("{what} switch type {name:#x} found {found:#x}");
            tw.run(&label, 0x004d_2e50, Conv::Thiscall(ty), &[7, name], |g| {
                sc::bridge::switch_type_ctor(g, ty, name)
            });
        }
        tw.w32(head, saved[0]);
        tw.w32(head + 8, saved[1]);
        tw.unstub();
        // the timer assignment: slots 8 of the source and 7 of the timer
        tw.stub(sc::TIMER_RELEASE, false, 1, 0, 0x0bad_000e);
        let src_vt = tw.vtable_with_slot(0x20, 0, 0x0bad_0014);
        let vt = tw.vtable_with_slot(0x1c, 1, 0x0bad_0015);
        let src = tw.zeros(0x10);
        tw.w32(src, src_vt);
        let timer = tw.zeros(0x40);
        tw.w32(timer, vt);
        let label = format!("{what} timer assign, no source");
        tw.run(&label, 0x0055_5e20, Conv::Thiscall(timer), &[0], |g| {
            sc::bridge::timer_assign(g, timer, 0)
        });
        for (t, owned) in [(0, 1u8), (0x7000, 0), (0x7000, 1)] {
            tw.w32(timer + 0x1c, t);
            tw.write(timer + 0x28, &[owned]);
            let label = format!("{what} timer assign {t:#x} {owned}");
            tw.run(&label, 0x0055_5e20, Conv::Thiscall(timer), &[src], |g| {
                sc::bridge::timer_assign(g, timer, src)
            });
        }
        tw.unstub();
        // the receivers: the global's slot 1 gives a descriptor or none
        tw.stub(sc::APPLY_DEFAULT_BUFFER, true, 2, 2, 0x0bad_0016);
        tw.stub(sc::RECEIVERS_RESIZE, true, 2, 2, 0x0bad_0017);
        let g = tw.rebase(sc::TRIGGER_GLOBAL);
        let saved = tw.r32(g);
        let this = tw.zeros(0x10);
        for d in [0, 0x00de_0000] {
            let gvt = tw.vtable_with_slot(4, 0, d);
            tw.w32(g, gvt);
            for (buffer, n) in [(this + 4, 1), (this + 4, 2), (0x5000, 1), (0, 0)] {
                let label = format!("{what} receivers {d:#x} ({buffer:#x}, {n})");
                tw.run(
                    &label,
                    0x0055_0750,
                    Conv::Thiscall(this),
                    &[buffer, n],
                    |g| sc::bridge::receivers_set(g, this, buffer, n),
                );
            }
        }
        tw.w32(g, saved);
        tw.unstub();
    }
}

/// The functions under five bytes (no hook; their bridges for the 64-bit
/// table).
#[test]
fn m5p_scene_c_stubs() {
    for (what, mut tw) in Twins::all() {
        for this in [0, 0x0012_3450, 0xffff_fff8] {
            for f in [0x0043_2bf0, 0x0050_42a0] {
                let label = format!("{what} stub {f:#x}");
                tw.run(&label, f, Conv::Thiscall(this), &[], |_| {
                    sc::bridge::stub_zero()
                });
            }
            let label = format!("{what} stub 0x555d00");
            tw.run_al(&label, 0x0055_5d00, Conv::Thiscall(this), &[], |_| {
                sc::bridge::stub_false()
            });
            let label = format!("{what} stub 0x4fd120");
            tw.run(&label, 0x004f_d120, Conv::Thiscall(this), &[], |_| {
                sc::bridge::stub_this(this)
            });
            let label = format!("{what} stub 0x4fc880");
            tw.run(&label, 0x004f_c880, Conv::Thiscall(this), &[], |_| {
                sc::bridge::stub_field_c(this)
            });
            let label = format!("{what} stub 0x5d7440");
            tw.run_void(&label, 0x005d_7440, Conv::Thiscall(this), &[], |_| {
                sc::bridge::stub_nothing();
            });
        }
    }
}
