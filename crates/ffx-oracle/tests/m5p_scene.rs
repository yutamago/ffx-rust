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
#![allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

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
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let at = TRAMP.with(std::cell::Cell::get);
        let b = x.to_bits();
        let r = self
            .0
            .call(at, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let v = r
            .st0_f32()
            .unwrap_or_else(|| panic!("intrinsic {addr:#x}: no float in ST0"));
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        let at = TRAMP.with(std::cell::Cell::get);
        let (p, q) = (x.to_bits(), y.to_bits());
        let words = [p as u32, (p >> 32) as u32, q as u32, (q >> 32) as u32, addr];
        let r = self
            .0
            .call(at + 19, Conv::Cdecl, &words)
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let v = r
            .st0_f32()
            .unwrap_or_else(|| panic!("intrinsic {addr:#x}: no float in ST0"));
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
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
    /// [`INTRINSIC_TRAMPOLINE`] in both oracles' code.
    tramp: u32,
}

/// The CRT intrinsics' trampolines (big-phyre2): at `+0` `fld qword
/// [esp+4]; call [esp+0xc]; ret` (one operand in `ST0`), at `+16` `fstp
/// st(0); ret` (the caller's store popping the result), at `+19` `fld
/// qword [esp+4]; fld qword [esp+0xc]; call [esp+0x14]; ret` (`_CIatan2`:
/// operands in `ST1` and `ST0`).
const INTRINSIC_TRAMPOLINE: [u8; 32] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3, 0xdd, 0x44, 0x24, 0x04, 0xdd, 0x44, 0x24, 0x0c, 0xff, 0x54, 0x24, 0x14, 0xc3,
];

thread_local! {
    /// The trampoline of the oracle a [`G`] is running on.
    static TRAMP: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
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
            tramp: 0,
        };
        t.log = t.zeros(4 + 32 * LOG_ENTRIES as usize);
        let tramp = t.a.alloc_code(&INTRINSIC_TRAMPOLINE).unwrap();
        assert_eq!(t.b.alloc_code(&INTRINSIC_TRAMPOLINE).unwrap(), tramp);
        t.tramp = tramp;
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
        TRAMP.with(|c| c.set(self.tramp));
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
        TRAMP.with(|c| c.set(self.tramp));
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

// ---- batch 4 (ffx_prender::m5p_scene_d) ----

use ffx_prender::m5p_scene_d as sd;

/// The one-word-key attach on synthetic trees (repeated keys find their
/// node; the fix-up runs as the original between attaches), the loaded
/// entities' fix-up (classes and none, a stride) on the image's or the
/// snapshot's class tree, and the entity lookup with the member calls
/// stubbed.
#[test]
fn m5p_scene_d_class_tree() {
    for (what, mut tw) in Twins::all() {
        let fixup = tw.rebase(sd::TREE_FIXUP);
        let head = tw.zeros(0x10);
        empty_tree(&mut tw, head);
        let mut rng = Rng(11);
        for i in 0..60 {
            let node = tw.zeros(0x14);
            let k = rng.next() % 25;
            tw.w32(node + 8, rng.next() & 0xff);
            tw.w32(node + 0xc, k);
            let label = format!("{what} attach1 #{i} key {k}");
            let r = tw.run(
                &label,
                sd::TREE_ATTACH1,
                Conv::Thiscall(head),
                &[node],
                |g| sd::bridge::tree_attach1(g, head, node),
            );
            if r == 0 {
                tw.a.call(fixup, Conv::Thiscall(head), &[node]).unwrap();
                tw.b.call(fixup, Conv::Thiscall(head), &[node]).unwrap();
                tw.diff(&format!("{label} fix-up"));
            }
        }
        // the class tree: emptied on the image (it holds no head there)
        let tree = tw.rebase(sd::CLASS_TREE);
        if !tw.snapshot {
            empty_tree(&mut tw, tree);
        }
        // a loaded list: four entities of 0x34 bytes, classes 0, c, the second entity itself (a key 0x0044f6d0 finds), c
        let desc = tw.zeros(0x20);
        tw.w32(desc + 0x1c, 0x34);
        let ents = tw.zeros(0x34 * 4);
        for (i, c) in [0u32, 0x00c1_0000, ents + 0x34, 0x00c1_0000]
            .iter()
            .enumerate()
        {
            tw.w32(ents + 0x34 * i as u32 + 4, *c);
        }
        let list = tw.zeros(0x40);
        tw.w32(list + 0x2c, desc);
        tw.w32(list + 0x30, ents);
        for n in [0, 4] {
            tw.w32(list + 0x3c, n);
            let label = format!("{what} fix instances {n}");
            tw.run(&label, 0x0044_f780, Conv::Cdecl, &[list], |g| {
                sd::bridge::fix_instances(g, list)
            });
        }
        // a node of key 0 (no loaded entity has one)
        let zero = tw.zeros(0x20);
        tw.w32(zero + 0x10, 0x0000_1234);
        let attach = tw.rebase(sd::TREE_ATTACH1);
        if tw
            .a
            .call(attach, Conv::Thiscall(tree), &[zero])
            .unwrap()
            .eax
            == 0
        {
            tw.a.call(fixup, Conv::Thiscall(tree), &[zero]).unwrap();
        }
        if tw
            .b
            .call(attach, Conv::Thiscall(tree), &[zero])
            .unwrap()
            .eax
            == 0
        {
            tw.b.call(fixup, Conv::Thiscall(tree), &[zero]).unwrap();
        }
        tw.diff(&format!("{what} key 0"));
        // lookups: an entity class, a class with the member, without it
        let entity = tw.rebase(sd::ENTITY_CLASS);
        let derived = tw.zeros(0x50);
        tw.w32(derived + 0x40, entity);
        let other = tw.zeros(0x50);
        for (member, value) in [
            (0, 0),
            (0x0099_0000, 0),
            (0x0099_0000, 0x00c2_0000),
            (0x0099_0000, 0x00c3_0000),
        ] {
            tw.stub(sd::CLASS_MEMBER, true, 1, 1, member);
            tw.stub(sd::MEMBER_VALUE, false, 2, 0, value);
            for (object, class) in [
                (0, derived),
                (ents + 0x34, derived),
                (ents + 0x68, derived),
                (ents, other),
                (ents, 0),
            ] {
                let label = format!(
                    "{what} entity lookup {object:#x} {class:#x} member {member:#x} {value:#x}"
                );
                tw.run(&label, 0x0044_f6d0, Conv::Cdecl, &[object, class], |g| {
                    sd::bridge::entity_lookup(g, object, class)
                });
            }
        }
        tw.unstub();
    }
}

/// The script count of entities of a class: no class, a class not derived
/// from the world class, one with entities of it and of others.
#[test]
fn m5p_scene_d_instance_member() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sd::GLOBAL_NAMESPACE, false, 0, 0, 0x00ab_c000);
        tw.stub(sd::PUSH_INSTANCE, false, 3, 0, 0x0bad_0018);
        let world = tw.rebase(sd::WORLD_CLASS);
        let d = tw.zeros(0x50);
        tw.w32(d + 0x40, world);
        let sub = tw.zeros(0x50);
        tw.w32(sub + 0x40, d);
        let alone = tw.zeros(0x50);
        let e: Vec<u32> = (0..3).map(|_| tw.zeros(0x10)).collect();
        tw.w32(e[0], e[1]);
        tw.w32(e[1], e[2]);
        tw.w32(e[0] + 8, sub);
        tw.w32(e[1] + 8, alone);
        tw.w32(e[2] + 8, d);
        let list = tw.zeros(4);
        for (class, first) in [(0, e[0]), (alone, e[0]), (d, e[0]), (sub, e[0]), (d, 0)] {
            tw.stub(sd::GET_CLASS_DESCRIPTOR, true, 1, 1, class);
            tw.w32(list, first);
            let label = format!("{what} instance member {class:#x} from {first:#x}");
            tw.run(
                &label,
                0x0044_e520,
                Conv::Cdecl,
                &[list, 0x5555, 0x6666],
                |g| sd::bridge::entity_instance_member(g, list, 0x5555, 0x6666),
            );
        }
        tw.unstub();
    }
}

/// The timer component's copy, the data block search, the array
/// constructors, the free list's named node and the destructors.
#[test]
fn m5p_scene_d_components() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sd::STRING_ASSIGN, true, 1, 1, 0);
        let src = tw.alloc(&(0..0x30u8).map(|b| b.wrapping_mul(7)).collect::<Vec<_>>());
        tw.write(src + 0x20, &0x7fc0_0042u32.to_le_bytes());
        let this = tw.zeros(0x30);
        for owned in [0u8, 1] {
            tw.write(src + 0x10, &[0x40 | owned]);
            let label = format!("{what} timer copy, name bit {owned}");
            tw.run(&label, 0x0055_5270, Conv::Thiscall(this), &[src], |g| {
                sd::bridge::timer_copy(g, this, src)
            });
        }
        let label = format!("{what} timer copy from itself");
        tw.run(&label, 0x0055_5270, Conv::Thiscall(this), &[this], |g| {
            sd::bridge::timer_copy(g, this, this)
        });
        assert_eq!(tw.logged(), 0, "{label}: no string assign");
        // the mesh: two segments of 2 and 3 blocks
        let mesh = tw.zeros(0x40);
        let segs = tw.zeros(0x80);
        let blocks = tw.zeros(0x80);
        tw.w32(segs + 8, 2);
        tw.w32(segs + 0xc, blocks);
        tw.w32(segs + 0x48, 0x8000_0003);
        tw.w32(segs + 0x4c, blocks + 0x18);
        for i in 0..5 {
            tw.w32(blocks + 0xc * i + 4, 0x100 + i);
        }
        tw.w32(mesh + 0x2c, segs);
        let out = tw.zeros(4);
        for count in [0, 1, 2, 0x8000_0002] {
            tw.w32(mesh + 0x28, count);
            for key in [0x100, 0x101, 0x103, 0x104, 0x105] {
                let label = format!("{what} data block {key:#x} in {count:#x}");
                tw.run(
                    &label,
                    0x0048_9150,
                    Conv::Thiscall(mesh),
                    &[out, key],
                    |g| sd::bridge::find_data_block(g, mesh, out, key),
                );
            }
        }
        // the array constructors (the element constructors are stubs)
        for (f, stride, ctor) in [
            (0x004d_f1d0, 0x20, sd::SHADER_CTOR),
            (0x004e_61d0, 8, sd::MATERIAL_SWITCH_CTOR),
            (0x004e_6250, 0x28, sd::RENDER_LIST_CTOR),
        ] {
            tw.stub(ctor, true, 0, 0, 0x0bad_0019);
            let items = tw.zeros(0x100);
            for n in [0, 1, 3] {
                let label = format!("{what} construct {f:#x} {n}");
                tw.run_void(&label, f, Conv::Cdecl, &[items, n], |g| {
                    sd::bridge::construct_items(g, items, n, stride, ctor);
                });
                assert_eq!(tw.logged(), n, "{label}: calls");
            }
            let label = format!("{what} construct {f:#x} null slot");
            tw.run_void(&label, f, Conv::Cdecl, &[0, 1], |g| {
                sd::bridge::construct_items(g, 0, 1, stride, ctor);
            });
        }
        tw.stub(sd::SHARRAY_COPY_CTOR, true, 1, 1, 0x0bad_001a);
        let dst = tw.zeros(0x40);
        for (d, n) in [(dst, 0), (dst, 3), (0, 1)] {
            let label = format!("{what} copy construct {d:#x} {n}");
            tw.run_void(&label, 0x004e_7d70, Conv::Cdecl, &[d, src, n], |g| {
                sd::bridge::copy_construct_items(g, d, src, n, sd::SHARRAY_COPY_CTOR);
            });
        }
        tw.unstub();
        // the named node
        tw.stub(sd::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(sd::FREE_LIST_DEALLOCATE, true, 1, 1, 0x0bad_001b);
        let node = tw.zeros(0x20);
        for (a, b2) in [
            (0x1000, 0x2000),
            (0x1001, 0x2000),
            (0x1000, 0x2001),
            (0x1001, 0x2001),
        ] {
            tw.w32(node + 0x10, a);
            tw.w32(node + 0xc, b2);
            let label = format!("{what} named node {a:#x} {b2:#x}");
            tw.run(&label, 0x0064_b8c0, Conv::Thiscall(0x7777), &[node], |g| {
                sd::bridge::free_named_node(g, 0x7777, node)
            });
        }
        // the scriptable component's deleting destructor
        tw.stub(sd::OBJECT_REF_RELEASE, true, 0, 0, 0x0bad_001c);
        let comp = tw.zeros(0x20);
        for (name, flags) in [(0x1000, 0), (0x1001, 1), (0x1000, 3), (0x1000, 2)] {
            tw.w32(comp + 0x10, name);
            let label = format!("{what} scriptable delete {name:#x} {flags}");
            tw.run(&label, 0x0055_2d30, Conv::Thiscall(comp), &[flags], |g| {
                sd::bridge::scriptable_delete(g, comp, flags)
            });
        }
        // the tree owner's destructor (all callees stubbed)
        for f in [
            sd::TREE_ORPHAN,
            sd::TREE_CLEAR,
            sd::FREE_LIST_RESET,
            sd::FREE_LIST_DTOR,
        ] {
            tw.stub(f, true, 0, 0, f);
        }
        let label = format!("{what} tree owner dtor");
        tw.run(&label, 0x0057_7e90, Conv::Thiscall(comp), &[], |g| {
            sd::bridge::tree_owner_dtor(g, comp)
        });
        assert_eq!(tw.logged(), 5, "{label}: calls");
        tw.unstub();
    }
}

/// The `PSharray` copy constructor (the buffer call stubbed, returning the
/// header's own word or a block) and the receivers' set, over sources and
/// old headers of 0, 1 and more elements, owned and external.
#[test]
fn m5p_scene_d_sharrays() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sd::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let this = tw.zeros(0x10);
        let src = tw.zeros(0x10);
        for count in [0, 1, 3, 0x8000_0003] {
            for own in [false, true] {
                let ret = if own { this + 4 } else { 0x0044_0000 };
                tw.stub(sd::SHARRAY_ALLOC, true, 2, 2, ret);
                tw.w32(src, count);
                tw.w32(src + 4, 0x0033_0000);
                tw.w32(this + 4, 0x0022_0000);
                let label = format!("{what} sharray copy ctor {count:#x} own {own}");
                tw.run(&label, 0x004e_9ee0, Conv::Thiscall(this), &[src], |g| {
                    sd::bridge::sharray_copy_ctor(g, this, src, sd::SHARRAY_ALLOC, false)
                });
            }
        }
        // the stub writes nothing, so the count the original reads back
        // is the 0 it stored: a stand-in that leaves a count shows the
        // free paths
        for header in [2, 0x8000_0002, 1] {
            let code = {
                // mov dword [ecx], header; mov eax, 0x00440000; ret 8
                let mut c = vec![0xc7, 0x01];
                c.extend(u32::to_le_bytes(header));
                c.push(0xb8);
                c.extend(0x0044_0000u32.to_le_bytes());
                c.extend([0xc2, 0x08, 0x00]);
                let at = tw.a.alloc_code(&c).unwrap();
                assert_eq!(tw.b.alloc_code(&c).unwrap(), at);
                at
            };
            let at = tw.rebase(sd::SHARRAY_ALLOC);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            tw.write(at, &j);
            for old in [0, 0x0022_0000, 0x0044_0000] {
                tw.w32(src, 3);
                tw.w32(this + 4, old);
                let label = format!("{what} sharray copy ctor, left {header:#x}, old {old:#x}");
                tw.run(&label, 0x004e_9ee0, Conv::Thiscall(this), &[src], |g| {
                    sd::bridge::sharray_copy_ctor(g, this, src, sd::SHARRAY_ALLOC, false)
                });
            }
        }
        tw.unstub();
        tw.stub(sd::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let buf = tw.alloc(&[1, 2, 3, 4, 5, 6, 7, 8]);
        for header in [0, 1, 3, 0x8000_0003] {
            for old in [0, 0x0022_0000, buf] {
                for n in [0, 1, 2, 0x8000_0000] {
                    tw.w32(this, header);
                    tw.w32(this + 4, old);
                    let label = format!("{what} sharray set ({header:#x}, {old:#x}) n {n:#x}");
                    tw.run(&label, 0x0055_06c0, Conv::Thiscall(this), &[buf, n], |g| {
                        sd::bridge::sharray_set(g, this, buf, n)
                    });
                }
            }
        }
        tw.unstub();
    }
}

/// The LOD blend packer: no LOD data, no blend, weights 0, 1, between, a
/// NaN, over sharrays of one and more levels.
#[test]
fn m5p_scene_d_lod_blend() {
    for (what, mut tw) in Twins::all() {
        let node = tw.zeros(0x40);
        let lod = tw.zeros(0x40);
        let ctx = tw.zeros(0x100);
        let levels = tw.zeros(0x40);
        let cursor = tw.zeros(4);
        let out = tw.zeros(0x10);
        for (count, data) in [(1, lod + 0x18), (3, levels)] {
            tw.w32(lod + 0x14, count);
            tw.w32(lod + 0x18, if count == 1 { 0 } else { levels });
            for (has, blends) in [(0, 1), (lod, 0), (lod, 1)] {
                tw.w32(node + 0x20, has);
                tw.w32(ctx + 0xbc, blends);
                for (index, w) in [
                    (0, 0.0f32),
                    (0, 1.0),
                    (0, 0.5),
                    (0, -0.0),
                    (0, f32::NAN),
                    (2, 0.25),
                ] {
                    if count == 1 && index != 0 {
                        continue;
                    }
                    tw.write(data + 12 * index, &w.to_le_bytes());
                    tw.w32(ctx + 0xc4, index);
                    tw.w32(cursor, out);
                    let args = [1, node, ctx, cursor, 5, 6, 7];
                    let label = format!("{what} lod blend ({count}, {has:#x}, {blends}) {w}");
                    tw.run(&label, 0x004d_40c0, Conv::Thiscall(0x1234), &args, |g| {
                        sd::bridge::pack_lod_blend(g, node, ctx, cursor)
                    });
                }
            }
        }
    }
}

// ---- batch 5 (ffx_prender::m5p_scene_e) ----

use ffx_prender::m5p_scene_e as se;

/// The gameplay types: the clean-up over registered and other entries
/// (first, middle, last; the free list resets stubbed), and the type
/// getter: listed, a null name, new (free lists with and without nodes,
/// the grow stubbed, the real `memcpy`).
#[test]
fn m5p_scene_e_gameplay_types() {
    for (what, mut tw) in Twins::all() {
        tw.stub(se::FREE_LIST_RESET, true, 0, 0, 0x0bad_001d);
        let head = tw.rebase(se::GAMEPLAY_TYPES);
        let saved = tw.r32(head);
        let e: Vec<u32> = (0..4).map(|_| tw.zeros(0x10)).collect();
        for marks in [[1u8, 0, 1, 0], [0, 1, 0, 1], [1, 1, 1, 1], [0, 0, 0, 0]] {
            tw.w32(head, e[0]);
            for i in 0..4 {
                tw.w32(e[i] + 4, e.get(i + 1).copied().unwrap_or(0));
                tw.write(e[i] + 8, &[marks[i]]);
            }
            let label = format!("{what} gameplay cleanup {marks:?}");
            tw.run(&label, 0x0043_34e0, Conv::Cdecl, &[], |g| {
                se::bridge::gameplay_cleanup(g)
            });
            assert_eq!(tw.logged(), 2, "{label}: resets");
        }
        tw.w32(head, 0);
        let label = format!("{what} gameplay cleanup, empty");
        tw.run(&label, 0x0043_34e0, Conv::Cdecl, &[], |g| {
            se::bridge::gameplay_cleanup(g)
        });
        tw.unstub();
        // the getter
        tw.stub(se::PRINTF, false, 3, 0, 0);
        tw.stub(se::FIND_TYPE, false, 1, 0, 0);
        tw.stub(se::TYPE_CTOR, true, 2, 2, 0x0bad_001e);
        tw.stub(se::FREE_LIST_GROW, true, 1, 1, 0);
        let pools = [
            tw.rebase(se::GAMEPLAY_TYPE_POOL),
            tw.rebase(se::GAMEPLAY_NAME_POOL),
        ];
        let saved_pools = [tw.r32(pools[0]), tw.r32(pools[1])];
        let name = tw.alloc(b"trigger\0");
        let nodes: Vec<u32> = (0..2).map(|_| tw.zeros(0x20)).collect();
        for (a, b2) in [(true, true), (false, true), (true, false), (false, false)] {
            tw.w32(pools[0], if a { nodes[0] } else { 0 });
            tw.w32(pools[1], if b2 { nodes[1] } else { 0 });
            tw.write(nodes[0], &[0xee; 0x20]);
            tw.write(nodes[1], &[0xee; 0x20]);
            tw.w32(nodes[0], 0x0101_0101);
            tw.w32(nodes[1], 0x0202_0202);
            tw.w32(pools[0] + 8, 0x40);
            tw.w32(pools[1] + 8, 0x20);
            let label = format!("{what} gameplay type, pools {a} {b2}");
            tw.run(&label, 0x0054_d1e0, Conv::Cdecl, &[name], |g| {
                se::bridge::gameplay_type(g, name)
            });
        }
        let label = format!("{what} gameplay type, null name");
        tw.run(&label, 0x0054_d1e0, Conv::Cdecl, &[0], |g| {
            se::bridge::gameplay_type(g, 0)
        });
        tw.stub(se::FIND_TYPE, false, 1, 0, 0x0077_0000);
        let label = format!("{what} gameplay type, listed");
        tw.run(&label, 0x0054_d1e0, Conv::Cdecl, &[name], |g| {
            se::bridge::gameplay_type(g, name)
        });
        tw.w32(pools[0], saved_pools[0]);
        tw.w32(pools[1], saved_pools[1]);
        tw.w32(head, saved);
        tw.unstub();
    }
}

/// The resizes: `PSharray<T*>` through their fills (stubbed: the header's
/// own word, a block, none), `PArray<PSceneRenderPass>` and
/// `PArray<PMaterialSwitch>` (allocation, copy, construct and release
/// stubbed), the LOD levels' external buffer, the segment contexts' copy
/// constructor.
#[test]
fn m5p_scene_e_resizes() {
    for (what, mut tw) in Twins::all() {
        tw.stub(se::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let this = tw.zeros(0x10);
        let block = 0x0066_0000;
        for (f, fill) in [
            (0x004f_8a40, se::STREAM_BINDINGS_FILL),
            (0x0055_0530, se::RECEIVERS_FILL),
        ] {
            for ret in [this + 4, block, 0] {
                tw.stub(fill, true, 3, 3, ret);
                for header in [0, 1, 3, 0x8000_0003] {
                    for n in [0, 1, 2, 3, header] {
                        tw.w32(this, header);
                        tw.w32(this + 4, 0x0012_3000);
                        let label = format!(
                            "{what} sharray resize {f:#x} ({header:#x}) to {n:#x}, {ret:#x}"
                        );
                        tw.run(&label, f, Conv::Thiscall(this), &[n], |g| {
                            se::bridge::sharray_resize(g, this, n, fill)
                        });
                    }
                }
            }
        }
        for (f, words) in [
            (se::RENDER_PASS_COPY, 3),
            (se::RENDER_PASS_CONSTRUCT, 2),
            (se::MATERIAL_COPY, 3),
            (se::MATERIAL_CONSTRUCT, 2),
            (se::MATERIAL_RELEASE, 2),
        ] {
            tw.stub(f, false, words, 0, f);
        }
        tw.stub(se::RENDER_PASS_SET_STORAGE, true, 2, 2, 0x0bad_001f);
        for ret in [block, 0] {
            tw.stub(se::PHYRE_MALLOC, false, 2, 0, ret);
            for header in [0, 1, 3, 0x8000_0003] {
                for old in [0, block, 0x0012_3000] {
                    for n in [0, 1, 2, 5, header] {
                        for f in [0x004e_75e0, 0x004e_7530] {
                            tw.w32(this, header);
                            tw.w32(this + 4, old);
                            let label = format!(
                                "{what} array resize {f:#x} ({header:#x}, {old:#x}) to {n:#x}, {ret:#x}"
                            );
                            tw.run(&label, f, Conv::Thiscall(this), &[n], |g| {
                                if f == 0x004e_75e0 {
                                    se::bridge::render_pass_array_resize(g, this, n)
                                } else {
                                    se::bridge::material_array_resize(g, this, n)
                                }
                            });
                        }
                    }
                }
            }
        }
        tw.stub(se::LOD_LEVEL_DTOR, true, 0, 0, 0x0bad_0020);
        for header in [0, 2, 0x8000_0002] {
            for old in [0, block, 0x0012_3000] {
                tw.w32(this, header);
                tw.w32(this + 4, old);
                let label = format!("{what} lod levels ({header:#x}, {old:#x})");
                tw.run(
                    &label,
                    0x005a_7d90,
                    Conv::Thiscall(this),
                    &[block, 7],
                    |g| se::bridge::lod_levels_set_external(g, this, block, 7),
                );
            }
        }
        // the segment contexts' copy constructor: its buffer call stubbed
        let src = tw.zeros(0x10);
        for count in [0, 1, 3] {
            for ret in [this + 4, block] {
                tw.stub(se::SEGMENT_CONTEXTS_COPY, true, 2, 2, ret);
                tw.w32(src, count);
                tw.w32(src + 4, 0x0033_0000);
                let label = format!("{what} segment contexts copy {count} {ret:#x}");
                tw.run(&label, 0x004f_2740, Conv::Thiscall(this), &[src], |g| {
                    sd::bridge::sharray_copy_ctor(g, this, src, se::SEGMENT_CONTEXTS_COPY, true)
                });
            }
        }
        tw.unstub();
    }
}

/// The tree destructors (callees stubbed; the removing one over trees of
/// three roots, an empty and a null root).
#[test]
fn m5p_scene_e_tree_dtors() {
    for (what, mut tw) in Twins::all() {
        for f in [
            se::TREE_ORPHAN,
            se::TREE_CLEAR,
            se::FREE_LIST_RESET,
            se::FREE_LIST_DTOR,
        ] {
            tw.stub(f, true, 0, 0, f);
        }
        // remove: a stub that sets the root (+0xc) to the next word of the
        // removed node, so the loop ends
        let code = {
            // mov eax, [esp+4]; mov eax, [eax]; mov [ecx+0xc], eax; ret 4
            let c = [
                0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x89, 0x41, 0x0c, 0xc2, 0x04, 0x00,
            ];
            let at = tw.a.alloc_code(&c).unwrap();
            assert_eq!(tw.b.alloc_code(&c).unwrap(), at);
            at
        };
        let at = tw.rebase(se::TREE_REMOVE);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        tw.write(at, &j);
        let tree = tw.zeros(0x100);
        let n: Vec<u32> = (0..3).map(|_| tw.zeros(0x10)).collect();
        for end in [tree, 0] {
            tw.w32(tree + 0xc, n[0]);
            tw.w32(n[0], n[1]);
            tw.w32(n[1], n[2]);
            tw.w32(n[2], end);
            let label = format!("{what} tree dtor removing, end {end:#x}");
            tw.run(&label, 0x0057_7da0, Conv::Thiscall(tree), &[], |g| {
                se::bridge::tree_dtor_removing(g, tree)
            });
        }
        tw.w32(tree + 0x64 + 0xc, tree + 0x64);
        let label = format!("{what} three trees dtor");
        tw.run(&label, 0x0057_8030, Conv::Thiscall(tree), &[], |g| {
            se::bridge::three_trees_dtor(g, tree)
        });
        assert_eq!(tw.logged(), 4 + 5 + 5, "{label}: calls");
        tw.unstub();
    }
}

/// The split distances over cascade counts 0..5 (a quiet NaN distance
/// among them).
#[test]
fn m5p_scene_e_split_distances() {
    for (what, mut tw) in Twins::all() {
        let this = tw.zeros(0x20);
        let cascades = tw.zeros(0x400);
        for i in 0..4u32 {
            let v = if i == 2 {
                0x7fc0_0077
            } else {
                (10.0f32 * f32::from(u8::try_from(i + 1).unwrap())).to_bits()
            };
            tw.w32(cascades + 0xc4 + 0xe0 * i, v);
        }
        tw.w32(this + 0xc, cascades);
        let out = tw.zeros(0x10);
        for count in [0, 1, 2, 3, 4, 5, 0x8000_0002] {
            tw.w32(this + 8, count);
            let label = format!("{what} split distances {count:#x}");
            tw.run(&label, 0x005b_8c70, Conv::Thiscall(this), &[out], |g| {
                se::bridge::split_distances(g, this, out)
            });
        }
    }
}

/// The world cursor's advance over a chain of blocks (the skip stubbed):
/// within a block, across blocks, to the end, by 0.
#[test]
fn m5p_scene_e_cursor_advance() {
    for (what, mut tw) in Twins::all() {
        tw.stub(se::CURSOR_SKIP_EMPTY, true, 0, 0, 0x0bad_0021);
        let cur = tw.zeros(0x20);
        // three blocks of 16-byte entries; each block's end word links on
        let blocks: Vec<u32> = (0..3).map(|_| tw.zeros(0x44)).collect();
        for i in 0..3 {
            let next_end = if i < 2 { blocks[i + 1] + 0x40 } else { 0 };
            tw.w32(blocks[i] + 0x40, next_end);
        }
        for (start, remaining, n) in [
            (0, 10, 1),
            (0, 10, 4),
            (0, 10, 6),
            (0x30, 8, 3),
            (0, 4, 4),
            (0x20, 12, 12),
            (0x40, 5, 2),
        ] {
            tw.w32(cur + 0xc, blocks[0] + start);
            tw.w32(cur + 0x10, blocks[0] + 0x40);
            tw.w32(cur + 0x14, 0x10);
            tw.w32(cur + 0x18, remaining);
            let label = format!("{what} cursor advance from {start:#x} ({remaining}) by {n}");
            tw.run(&label, 0x009f_3e50, Conv::Thiscall(cur), &[n], |g| {
                se::bridge::cursor_advance(g, cur, n)
            });
        }
        // by 0: nothing (the original leaves its caller's eax)
        let label = format!("{what} cursor advance by 0");
        tw.run_any(&label, 0x009f_3e50, Conv::Thiscall(cur), &[0], |g| {
            se::bridge::cursor_advance(g, cur, 0)
        });
        // past a start beyond the end: the remaining count rules
        tw.w32(cur + 0xc, blocks[2] + 0x50);
        tw.w32(cur + 0x10, 0);
        tw.w32(cur + 0x18, 2);
        let label = format!("{what} cursor advance, no end");
        tw.run(&label, 0x009f_3e50, Conv::Thiscall(cur), &[2], |g| {
            se::bridge::cursor_advance(g, cur, 2)
        });
        tw.unstub();
    }
}

// ---- batch 6 (ffx_prender::m5p_scene_f) ----

use ffx_prender::m5p_scene_f as sf;

/// Floats for the maths tests: ordinary values, signed zeros, values that
/// round, infinities and quiet NaNs.
fn float_words(rng: &mut Rng, n: usize) -> Vec<u8> {
    let special = [
        0.0f32.to_bits(),
        (-0.0f32).to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc0_1234,
        0xffc0_0001,
        f32::MAX.to_bits(),
        1.0e-40f32.to_bits(),
    ];
    let mut out = Vec::new();
    for _ in 0..n {
        let r = rng.next();
        let w = if r.is_multiple_of(7) {
            special[(r >> 8) as usize % special.len()]
        } else {
            let f = f32::from(u16::try_from(r >> 16).unwrap_or(0)) / 16.0 - 1000.0;
            (f * 1.000_123).to_bits()
        };
        out.extend(w.to_le_bytes());
    }
    out
}

/// The polygon area over random polygons (0 to 9 points) and the 3x3
/// product over random matrices (into fresh memory and onto itself).
#[test]
fn m5p_scene_f_maths() {
    for (what, mut tw) in Twins::all() {
        let mut rng = Rng(21);
        for round in 0..40 {
            let n = round % 10;
            let pts = tw.alloc(&float_words(&mut rng, 4 * n.max(1) as usize));
            let label = format!("{what} polygon area {round} ({n} points)");
            let (ra, r) = tw.run_any(&label, 0x005a_3af0, Conv::Cdecl, &[pts, n], |g| {
                sf::bridge::polygon_area(g, pts, n)
            });
            assert!(ra.st0_is(f64::from(r)), "{label}: ST0 {:?} vs {r}", ra.st0);
        }
        // sums whose grouping shows: (big - big) + 1 per element
        let a = tw.alloc(
            &[
                1.0e30f32, 1.0e30, 1.0e30, 0.0, -1.0e30, -1.0e30, -1.0e30, 0.0, 1.0, 1.0, 1.0, 0.0,
            ]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<_>>(),
        );
        let b = tw.alloc(
            &[
                1.0f32, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0,
            ]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<_>>(),
        );
        let out = tw.zeros(48);
        let label = format!("{what} rotation product, cancelling sums");
        tw.run(&label, 0x0054_7870, Conv::Thiscall(a), &[out, b], |g| {
            sf::bridge::rotation_product(g, a, out, b)
        });
        for round in 0..40 {
            let a = tw.alloc(&float_words(&mut rng, 12));
            let b = tw.alloc(&float_words(&mut rng, 12));
            let out = if round % 4 == 3 {
                a
            } else {
                tw.alloc(&[0x5a; 48])
            };
            let label = format!("{what} rotation product {round}");
            tw.run(&label, 0x0054_7870, Conv::Thiscall(a), &[out, b], |g| {
                sf::bridge::rotation_product(g, a, out, b)
            });
        }
    }
}

/// Builds a node of the scene tree with a name (`None`: no name; owned
/// names get bit 0 when `mark`) and links: `(child, sibling, parent)`
/// written later.
fn scene_node(tw: &mut Twins, name: Option<&str>, mark: bool) -> u32 {
    let node = tw.zeros(0x60);
    if let Some(s) = name {
        let p = tw.alloc(format!("{s}\0").as_bytes());
        tw.w32(node + 0x50, p + u32::from(mark));
    }
    node
}

/// The node searches over a small tree (root; children a, b; a's child
/// c; b's child d; names repeated so the last match counts), and the
/// attachable component's target.
#[test]
fn m5p_scene_f_nodes() {
    for (what, mut tw) in Twins::all() {
        let root = scene_node(&mut tw, Some("root"), false);
        let a = scene_node(&mut tw, Some("arm"), true);
        let b = scene_node(&mut tw, Some(""), false);
        let e = scene_node(&mut tw, Some("leg"), false);
        let f = scene_node(&mut tw, None, false);
        let c = scene_node(&mut tw, Some("hand"), false);
        let d = scene_node(&mut tw, Some("hand"), true);
        // child +8, sibling +0, parent +4
        tw.w32(root + 8, a);
        tw.w32(a, b);
        tw.w32(a + 4, root);
        tw.w32(b + 4, root);
        tw.w32(a + 8, c);
        tw.w32(c + 4, a);
        // c's sibling e (no child), e's sibling f (no name)
        tw.w32(c, e);
        tw.w32(e + 4, a);
        tw.w32(e, f);
        tw.w32(f + 4, a);
        tw.w32(b + 8, d);
        tw.w32(d + 4, b);
        let names: Vec<u32> = ["root", "arm", "hand", "leg", "", "han", "Leg"]
            .iter()
            .map(|s| tw.alloc(format!("{s}\0").as_bytes()))
            .collect();
        let query = tw.zeros(8);
        for start in [root, a, b, c, e] {
            for &n in names.iter().chain(&[0]) {
                tw.w32(query, n);
                tw.w32(query + 4, 0x1111);
                let label = format!("{what} find node {n:#x} under {start:#x}");
                tw.run(&label, 0x0050_4a00, Conv::Cdecl, &[query, start], |g| {
                    sf::bridge::find_node(g, query, start)
                });
                let label = format!("{what} find named {n:#x} under {start:#x}");
                tw.run(&label, 0x0050_6b80, Conv::Thiscall(start), &[n], |g| {
                    sf::bridge::find_node_named(g, start, n)
                });
            }
        }
        // the attachable component
        tw.stub(sf::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let block = tw.zeros(0x20);
        let comp = tw.zeros(0x80);
        for ret in [block, 0] {
            tw.stub(sf::PHYRE_MALLOC, false, 2, 0, ret);
            for (node, name) in [
                (root, names[2]),
                (root, names[3]),
                (root, names[4]),
                (root, 0),
                (0, 0),
            ] {
                for old in [0x0012_3000, 0x0012_3001] {
                    tw.write(block, &[0xee; 0x20]);
                    tw.w32(comp + 0x54, old);
                    let label =
                        format!("{what} attach {node:#x} {name:#x} old {old:#x} alloc {ret:#x}");
                    tw.run(
                        &label,
                        0x0055_ab30,
                        Conv::Thiscall(comp),
                        &[node, name],
                        |g| sf::bridge::attach_target(g, comp, node, name),
                    );
                }
            }
        }
        tw.unstub();
    }
}

/// The fly controller's matrix set (real callees; the port's frame on the
/// stack) and the time span (a stand-in range call writing the two
/// times, or failing).
#[test]
fn m5p_scene_f_frames() {
    for (what, mut tw) in Twins::all() {
        let mut rng = Rng(31);
        let frame = tw.b.stack_range().start + 0x1000;
        for round in 0..10 {
            let this = tw.alloc(&float_words(&mut rng, 28));
            let v = tw.alloc(&float_words(&mut rng, 3));
            let label = format!("{what} fly set matrix {round}");
            tw.run(&label, 0x005d_77a0, Conv::Thiscall(this), &[v], |g| {
                sf::bridge::fly_set_matrix(g, this, v, frame)
            });
        }
        let at = tw.rebase(sf::TIME_RANGE);
        for (start, end, ret) in [
            (1.5f32, 4.25f32, 0u32),
            (0.1, 0.3, 0),
            (f32::INFINITY, f32::INFINITY, 0),
            (2.0, 1.0, 1),
        ] {
            // mov eax,[esp+4]; mov [eax],start; mov eax,[esp+8]; mov [eax],end; mov eax,ret; ret 8
            let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
            c.extend(start.to_bits().to_le_bytes());
            c.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]);
            c.extend(end.to_bits().to_le_bytes());
            c.push(0xb8);
            c.extend(ret.to_le_bytes());
            c.extend([0xc2, 0x08, 0x00]);
            let code = tw.a.alloc_code(&c).unwrap();
            assert_eq!(tw.b.alloc_code(&c).unwrap(), code);
            let mut j = vec![0xe9];
            j.extend(code.wrapping_sub(at + 5).to_le_bytes());
            let old = Oracle::read(&mut tw.a, at, 5).unwrap();
            tw.saved.entry(at).or_insert(old);
            tw.write(at, &j);
            let label = format!("{what} time span {start} {end} {ret}");
            let (ra, r) = tw.run_any(&label, 0x0051_68b0, Conv::Thiscall(0x1234), &[], |g| {
                sf::bridge::time_span(g, 0x1234, frame)
            });
            assert!(ra.st0_is(f64::from(r)), "{label}: ST0 {:?} vs {r}", ra.st0);
        }
        tw.unstub();
    }
}

// ---- batch 7 (ffx_prender::m5p_scene_g) ----

use ffx_prender::m5p_scene_g as sg;

/// The fly controller's steps over random matrices, positions and step
/// sizes (real matrix callees; the port's frame on the stack).
#[test]
fn m5p_scene_g_fly_steps() {
    for (what, mut tw) in Twins::all() {
        let mut rng = Rng(41);
        let frame = tw.b.stack_range().start + 0x1000;
        for round in 0..30 {
            let this = tw.alloc(&float_words(&mut rng, 28));
            let s = float_words(&mut rng, 2);
            let a = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
            let b = u32::from_le_bytes([s[4], s[5], s[6], s[7]]);
            let label = format!("{what} fly forward {round}");
            tw.run(&label, 0x005d_7190, Conv::Thiscall(this), &[a], |g| {
                sg::bridge::fly_forward(g, this, f32::from_bits(a), frame)
            });
            let label = format!("{what} fly strafe {round}");
            tw.run(&label, 0x005d_72b0, Conv::Thiscall(this), &[a, b], |g| {
                sg::bridge::fly_strafe(g, this, f32::from_bits(a), f32::from_bits(b), frame)
            });
        }
    }
}

/// The context switch's default state: already two pairs (each with one
/// word or not), another count (owned and external old pairs), failing
/// allocations (a stand-in allocator that fails on a chosen call).
#[test]
fn m5p_scene_g_switch_state() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sg::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(sg::STATE_RELEASE, false, 2, 0, 0x0bad_0022);
        let state = tw.zeros(8);
        let pairs = tw.zeros(0x10);
        let words = tw.zeros(0x40);
        let old_words = tw.zeros(8);
        // the allocator: hands out 16-byte blocks from `words` + 0x10 on,
        // the call numbered `fail` (0-based, counted in `words`) gets 0
        let at = tw.rebase(sg::PHYRE_MALLOC);
        let code = {
            // mov eax,[words]; inc dword [words]; cmp eax,[words+4]; je zero;
            // shl eax,4; add eax, words+0x10; ret; zero: xor eax,eax; ret
            let mut c = vec![0xa1];
            c.extend(words.to_le_bytes());
            c.extend([0xff, 0x05]);
            c.extend(words.to_le_bytes());
            c.extend([0x3b, 0x05]);
            c.extend((words + 4).to_le_bytes());
            c.extend([0x74, 0x09]);
            c.extend([0xc1, 0xe0, 0x04]);
            c.push(0x05);
            c.extend((words + 0x10).to_le_bytes());
            c.push(0xc3);
            c.extend([0x31, 0xc0, 0xc3]);
            let code = tw.a.alloc_code(&c).unwrap();
            assert_eq!(tw.b.alloc_code(&c).unwrap(), code);
            code
        };
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        tw.write(at, &j);
        for header in [2, 0, 3, 0x8000_0003] {
            for (c0, c1) in [(1, 1), (0, 1), (1, 3), (0x8000_0002, 0)] {
                for fail in [9, 0, 1, 2] {
                    tw.write(words, &[0; 0x40]);
                    // blocks that look like a one-word pair at +8 (the
                    // original zeroes them)
                    for k in 0..3 {
                        tw.w32(words + 0x10 + 0x10 * k + 8, 1);
                        tw.w32(words + 0x10 + 0x10 * k + 0xc, old_words + 4);
                    }
                    tw.w32(words + 4, fail);
                    tw.w32(state, header);
                    tw.w32(state + 4, pairs);
                    tw.w32(pairs, c0);
                    tw.w32(pairs + 4, old_words);
                    tw.w32(pairs + 8, c1);
                    tw.w32(pairs + 0xc, if c1 == 0 { 0 } else { old_words + 4 });
                    let label =
                        format!("{what} switch state {header:#x} ({c0:#x}, {c1:#x}) fail {fail}");
                    tw.run(
                        &label,
                        0x004d_4d50,
                        Conv::Thiscall(0x1234),
                        &[7, state],
                        |g| sg::bridge::switch_default_state(g, state),
                    );
                }
            }
        }
        tw.unstub();
    }
}

/// The joint target: named joints found (owned and unowned names, a
/// null slot), not found, no joints; unnamed with and without a node;
/// the copy's allocation failing.
#[test]
fn m5p_scene_g_attach_joint() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sg::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let block = tw.zeros(0x20);
        let names: Vec<u32> = ["hip", "spine", "head", "", "Head"]
            .iter()
            .map(|s| tw.alloc(format!("{s}\0").as_bytes()))
            .collect();
        let joints = tw.zeros(0x14);
        // a second "head": the first one wins
        tw.w32(joints + 0x10, names[2]);
        tw.w32(joints, names[0] + 1);
        tw.w32(joints + 4, 0);
        tw.w32(joints + 8, names[1]);
        tw.w32(joints + 0xc, names[2] + 1);
        let obj = tw.zeros(0x30);
        tw.w32(obj + 0x24, joints);
        let skel = tw.zeros(8);
        tw.w32(skel, obj);
        let comp = tw.zeros(0x80);
        for ret in [block, 0] {
            tw.stub(sg::PHYRE_MALLOC, false, 2, 0, ret);
            for count in [5, 0x8000_0004, 0, 2] {
                tw.w32(obj + 0x18, count);
                for node in [0, 0x0077_0000] {
                    tw.w32(skel + 4, node);
                    for &n in names.iter().chain(&[0]) {
                        for old in [0x0012_3000, 0x0012_3001] {
                            tw.write(block, &[0xee; 0x20]);
                            tw.w32(comp + 0x54, old);
                            let label = format!(
                                "{what} joint {n:#x} count {count:#x} node {node:#x} old {old:#x} alloc {ret:#x}"
                            );
                            tw.run(&label, 0x0055_a980, Conv::Thiscall(comp), &[skel, n], |g| {
                                sg::bridge::attach_joint(g, comp, skel, n)
                            });
                        }
                    }
                }
            }
        }
        tw.unstub();
    }
}

// ---- batch 8 (ffx_prender::m5p_scene_h) ----

use ffx_prender::m5p_scene_h as sh;

/// The render passes' copy construction: arrays empty and not (counts with
/// bit 31), the allocation failing, a null destination slot (the copies
/// and the allocator stubbed).
#[test]
fn m5p_scene_h_render_passes() {
    for (what, mut tw) in Twins::all() {
        for &(copy, _) in &sh::PASS_ARRAYS {
            tw.stub(copy, false, 3, 0, copy);
        }
        let src = tw.zeros(0x28 * 3);
        for i in 0..3u32 {
            let s = src + 0x28 * i;
            tw.w32(s, 0x100 + i);
            for k in 0..4u32 {
                let count = [0, 2, 0x8000_0003, 1][((i + k) % 4) as usize];
                tw.w32(s + 4 + 8 * k, count);
                tw.w32(s + 8 + 8 * k, 0x0050_0000 + 0x100 * k);
            }
            tw.write(s + 0x24, &[0x80 | i as u8]);
        }
        let dst = tw.zeros(0x28 * 3);
        for ret in [0x0066_0000, 0] {
            tw.stub(sh::PHYRE_MALLOC, false, 2, 0, ret);
            for (d, n) in [(dst, 3), (dst, 0), (0, 1)] {
                tw.write(dst, &[0xee; 0x28 * 3]);
                let label = format!("{what} render passes copy {d:#x} {n} alloc {ret:#x}");
                tw.run_void(&label, 0x004e_1ae0, Conv::Cdecl, &[d, src, n], |g| {
                    sh::bridge::render_passes_copy(g, d, src, n);
                });
            }
        }
        tw.unstub();
    }
}

/// The rendering utility's teardown over its globals: tagged arrays
/// owned, external, null; constant buffers bound and not, owned,
/// external, null (the callees stubbed; `isBound` answers from the
/// buffer's `+0x38` as the original does).
#[test]
fn m5p_scene_h_render_resources() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sh::LOCK_RELEASE, true, 0, 0, 0);
        tw.stub(sh::RELEASE_TAGGED, false, 2, 0, 0x0bad_0023);
        tw.stub(sh::CB_RELEASE, true, 0, 0, 0x0bad_0024);
        tw.stub(sh::CONSTANT_BUFFER_DTOR, true, 0, 0, 0x0bad_0025);
        tw.stub(sh::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let tagged = tw.rebase(sh::TAGGED);
        let cbs = tw.rebase(sh::CONSTANT_BUFFERS);
        let scratch = tw.rebase(sh::SCRATCH[0]);
        let saved: Vec<u8> = Oracle::read(&mut tw.a, tagged, 0x20).unwrap();
        let buffers = tw.zeros(0x3c * 3);
        tw.w32(buffers + 0x38, 1);
        tw.w32(buffers + 0x3c * 2 + 0x38, 7);
        for (tc, tb) in [(0, 0), (3, 0x0012_3000), (0x8000_0002, 0x0012_3000), (2, 0)] {
            for (cc, cb) in [
                (0, 0),
                (0, buffers),
                (3, buffers),
                (0x8000_0003, buffers),
                (0x8000_0000, 0),
            ] {
                tw.w32(tagged, tc);
                tw.w32(tagged + 4, tb);
                tw.write(scratch, &[0x11, 0x22, 0x33, 0x44]);
                tw.w32(scratch + 4, 0x0014_0000);
                tw.w32(scratch + 8, 0x3333);
                tw.w32(cbs, cc);
                tw.w32(cbs + 4, cb);
                let label =
                    format!("{what} render resources ({tc:#x}, {tb:#x}) ({cc:#x}, {cb:#x})");
                tw.run(&label, 0x004f_dca0, Conv::Cdecl, &[], |g| {
                    sh::bridge::render_resources_release(g)
                });
            }
        }
        tw.write(tagged, &saved);
        tw.unstub();
    }
}

/// The animation's time range: clips' ranges from a stand-in that writes
/// the next pair of a table (or fails), own ranges ordered, reversed and
/// NaN, no clips, a missing controller.
#[test]
fn m5p_scene_h_time_range() {
    for (what, mut tw) in Twins::all() {
        let frame = tw.b.stack_range().start + 0x1000;
        // the stand-in: k = [table]; *start = table[1 + 2k]; *end =
        // table[2 + 2k]; [table] = k + 1; eax = [table + 0x40]
        let table = tw.zeros(0x44);
        let mut c = vec![0x53]; // push ebx
        c.extend([0x8b, 0x1d]);
        c.extend(table.to_le_bytes()); // mov ebx, [table]
        c.extend([0x8b, 0x04, 0xdd]);
        c.extend((table + 4).to_le_bytes()); // mov eax, [table + 4 + ebx*8]
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.extend([0x8b, 0x04, 0xdd]);
        c.extend((table + 8).to_le_bytes()); // mov eax, [table + 8 + ebx*8]
        c.extend([0x8b, 0x54, 0x24, 0x0c]); // mov edx, [esp+0xc]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.extend([0xff, 0x05]);
        c.extend(table.to_le_bytes()); // inc dword [table]
        c.push(0xa1);
        c.extend((table + 0x40).to_le_bytes()); // mov eax, [table + 0x40]
        c.push(0x5b); // pop ebx
        c.extend([0xc2, 0x08, 0x00]); // ret 8
        let code = tw.a.alloc_code(&c).unwrap();
        assert_eq!(tw.b.alloc_code(&c).unwrap(), code);
        let at = tw.rebase(sh::CLIP_RANGE);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        tw.write(at, &j);
        // two tables of clip ranges: ordinary ones, and ties of signed
        // zeros and NaNs (which the compares keep or drop as the original)
        let tables: [[(f32, f32); 3]; 2] = [
            [(1.0, 5.0), (0.5, 3.0), (2.0, 9.0)],
            [(0.0, 5.0), (-0.0, f32::NAN), (f32::NAN, 5.0)],
        ];
        let ctrls: Vec<u32> = (0..3).map(|_| tw.zeros(0x30)).collect();
        let clips = tw.zeros(0x10);
        for (i, &ctl) in ctrls.iter().enumerate() {
            let clip = tw.zeros(0x30);
            tw.w32(clip + 0x24, ctl);
            tw.w32(clips + 4 * i as u32, clip);
        }
        let anim = tw.zeros(0x20);
        tw.w32(anim + 8, clips);
        let out = tw.zeros(8);
        for (t, pairs) in tables.iter().enumerate() {
            let mut words = Vec::new();
            for (a, b) in pairs {
                words.extend(a.to_le_bytes());
                words.extend(b.to_le_bytes());
            }
            for count in [0, 1, 3, 0x8000_0002] {
                for (own_lo, own_hi) in [
                    (0.0f32, 10.0f32),
                    (4.0, 2.0),
                    (f32::NAN, 1.0),
                    (0.25, 0.25),
                    (-1.0, 4.0),
                ] {
                    for fail in [0, 3] {
                        tw.write(table + 4, &words);
                        tw.w32(table, 0);
                        tw.w32(table + 0x40, fail);
                        tw.w32(anim + 4, count);
                        tw.write(anim + 0x14, &own_lo.to_le_bytes());
                        tw.write(anim + 0x18, &own_hi.to_le_bytes());
                        tw.w32(out, 0x5555_5555);
                        tw.w32(out + 4, 0x6666_6666);
                        let label = format!(
                            "{what} time range table {t} {count:#x} own ({own_lo}, {own_hi}) fail {fail}"
                        );
                        tw.run(
                            &label,
                            0x0051_6a20,
                            Conv::Thiscall(anim),
                            &[out, out + 4],
                            |g| sh::bridge::time_range(g, anim, out, out + 4, frame),
                        );
                    }
                }
            }
        }
        // a missing controller (second clip)
        let second_clip = tw.r32(clips + 4);
        tw.w32(second_clip + 0x24, 0);
        tw.w32(anim + 4, 3);
        tw.w32(table, 0);
        tw.w32(table + 0x40, 0);
        let label = format!("{what} time range, no controller");
        tw.run(
            &label,
            0x0051_6a20,
            Conv::Thiscall(anim),
            &[out, out + 4],
            |g| sh::bridge::time_range(g, anim, out, out + 4, frame),
        );
        // and the first clip's
        let first_clip = tw.r32(clips);
        tw.w32(first_clip + 0x24, 0);
        let label = format!("{what} time range, no first controller");
        tw.run(
            &label,
            0x0051_6a20,
            Conv::Thiscall(anim),
            &[out, out + 4],
            |g| sh::bridge::time_range(g, anim, out, out + 4, frame),
        );
        tw.unstub();
    }
}

// ---- batch 9 (ffx_prender::m5p_scene_i) ----

use ffx_prender::m5p_scene_i as si;

/// The partition step over random arrays (2 to 60 elements, few distinct
/// keys so runs of equal ones form, and many), with the real medians and
/// comparators that count their calls (by value and by pointer, the
/// second ordering 8-byte records by their first word).
#[test]
fn m5p_scene_i_partitions() {
    for (what, mut tw) in Twins::all() {
        let counter = tw.zeros(4);
        // by value: inc [counter]; mov eax,[esp+4]; cmp eax,[esp+8]; setb al; ret
        let mut c = vec![0xff, 0x05];
        c.extend(counter.to_le_bytes());
        c.extend([
            0x8b, 0x44, 0x24, 0x04, 0x3b, 0x44, 0x24, 0x08, 0x0f, 0x92, 0xc0, 0xc3,
        ]);
        let by_value = tw.a.alloc_code(&c).unwrap();
        assert_eq!(tw.b.alloc_code(&c).unwrap(), by_value);
        // by pointer: inc [counter]; mov eax,[esp+4]; mov eax,[eax];
        // mov ecx,[esp+8]; cmp eax,[ecx]; setb al; ret
        let mut c = vec![0xff, 0x05];
        c.extend(counter.to_le_bytes());
        c.extend([
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x8b, 0x4c, 0x24, 0x08, 0x3b, 0x01, 0x0f, 0x92,
            0xc0, 0xc3,
        ]);
        let by_pointer = tw.a.alloc_code(&c).unwrap();
        assert_eq!(tw.b.alloc_code(&c).unwrap(), by_pointer);
        let out = tw.zeros(8);
        let mut rng = Rng(51);
        for round in 0..60u32 {
            let n = 2 + rng.next() % 59;
            let keys = if round % 3 == 0 { 3 } else { 1000 };
            for (f, order) in [
                (0x004d_2400, si::Order::Below),
                (0x004d_2560, si::Order::ByValue(by_value)),
                (0x004d_2740, si::Order::ByPointer(by_pointer)),
            ] {
                let size = order.size();
                let mut bytes = Vec::new();
                for i in 0..n {
                    bytes.extend((rng.next() % keys).to_le_bytes());
                    if size == 8 {
                        bytes.extend((0x7700_0000 + i).to_le_bytes());
                    }
                }
                let first = tw.alloc(&bytes);
                let last = first + n * size;
                tw.w32(counter, 0);
                let mut args = vec![out, first, last];
                match order {
                    si::Order::ByValue(p) | si::Order::ByPointer(p) => args.push(p),
                    si::Order::Below => {}
                }
                let label = format!("{what} partition {f:#x} round {round} ({n} of {keys})");
                tw.run(&label, f, Conv::Cdecl, &args, |g| {
                    si::bridge::partition(g, order, out, first, last)
                });
            }
        }
    }
}

// ---- batch 10 (ffx_prender::m5p_scene_j) ----

use ffx_prender::m5p_scene_j as sj;

/// The heap sort step over random heaps of 8-byte records (0 to 40; the
/// real `_Adjust_heap` and a by-pointer comparator), the port's frame on
/// the stack.
#[test]
fn m5p_scene_j_sort_heap() {
    for (what, mut tw) in Twins::all() {
        // mov eax,[esp+4]; mov eax,[eax]; mov ecx,[esp+8]; cmp eax,[ecx]; setb al; ret
        let c = [
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0x8b, 0x4c, 0x24, 0x08, 0x3b, 0x01, 0x0f, 0x92,
            0xc0, 0xc3,
        ];
        let pred = tw.a.alloc_code(&c).unwrap();
        assert_eq!(tw.b.alloc_code(&c).unwrap(), pred);
        let frame = tw.b.stack_range().start + 0x1000;
        let mut rng = Rng(61);
        for round in 0..30u32 {
            let n = rng.next() % 41;
            let mut bytes = Vec::new();
            for i in 0..n {
                bytes.extend((rng.next() % 50).to_le_bytes());
                bytes.extend((0x7700_0000 + i).to_le_bytes());
            }
            let first = tw.alloc(&bytes);
            let last = first + 8 * n;
            let label = format!("{what} sort heap round {round} ({n})");
            tw.run_void(
                &label,
                0x004d_2360,
                Conv::Cdecl,
                &[first, last, pred],
                |g| {
                    sj::bridge::sort_heap(g, first, last, pred, frame);
                },
            );
        }
    }
}

/// The occluder job: entry counts 0, 3 and negative, an old entry buffer
/// owned, external, the job's own, the camera from `+0x34` or `+0`; the
/// ring buffer and the fill stubbed.
#[test]
fn m5p_scene_j_occluder_job() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sj::RING, false, 0, 0, 0x00ab_0000);
        tw.stub(sj::OCCLUDER_FILL, false, 6, 0, 0x0bad_0026);
        tw.stub(sj::PHYRE_FREE, false, 1, 0, 0x0bad_0002);
        let block = tw.zeros(0x80);
        tw.stub(sj::RING_ALLOCATE, true, 1, 1, block);
        let this = tw.zeros(0x40);
        let tail = tw.zeros(4);
        for n in [0, 3, 0x8000_0001] {
            for (old_header, old) in [
                (0, 0),
                (2, 0x0012_3000),
                (0x8000_0002, 0x0012_3000),
                (2, block + 0x20),
            ] {
                for camera in [0, 0x00cc_0000] {
                    tw.write(block, &[0xee; 0x80]);
                    tw.w32(block + 0x18, old_header);
                    tw.w32(block + 0x1c, old);
                    tw.w32(this, 0x00dd_0000);
                    tw.w32(this + 0x34, camera);
                    tw.w32(this + 0x30, 0x4444);
                    tw.w32(this + 0x2c, tail);
                    let label = format!(
                        "{what} occluder job {n:#x} old ({old_header:#x}, {old:#x}) camera {camera:#x}"
                    );
                    tw.run(
                        &label,
                        0x009f_2f20,
                        Conv::Thiscall(this),
                        &[0x5555, n, 0x6666],
                        |g| sj::bridge::occluder_job(g, this, 0x5555, n, 0x6666),
                    );
                }
            }
        }
        tw.unstub();
    }
}

// ---- batch 11 (ffx_prender::m5p_scene_k) ----

use ffx_prender::m5p_scene_k as sk;

/// An instance list node for the world count test: class, first
/// instance, count, block word; returns the node (`+0x20` links on).
fn instance_node(tw: &mut Twins, class: u32, first: u32, count: u32, block: u32) -> u32 {
    let n = tw.zeros(0x60) + 0x20;
    tw.w32(n + 0xc, class);
    tw.w32(n + 0x14, first);
    tw.w32(n + 0x1c, count);
    tw.w32(n - 0x18, block);
    n
}

/// The world's instance count. The world's list entries (20 bytes, in
/// class depth order: `+0` the ring of its lists, `+8` its instance count,
/// `+0x10` the class depth) and their rings (each list node links on
/// through `+0`, the last one to the next entry). Classes: base (entry 0)
/// <- mid (no entry) <- leaf (entry 1); other (entry 2). Queries of each
/// class with its own entry, through the chain (mid: base's entry, then
/// the leaf lists under it), past the entries, and with no entry on the
/// chain. Lists hold instances over blocks with an empty block in front.
/// The real `PWorldCursor_nextList`.
#[test]
fn m5p_scene_k_instance_count() {
    for (what, mut tw) in Twins::all() {
        let frame = tw.b.stack_range().start + 0x1000;
        let base = tw.zeros(0x90);
        let mid = tw.zeros(0x90);
        let leaf = tw.zeros(0x90);
        let other = tw.zeros(0x90);
        tw.w32(base + 0x80, 0);
        tw.w32(mid + 0x80, 0x8000_0000);
        tw.w32(mid + 0x40, base);
        tw.w32(leaf + 0x80, 1);
        tw.w32(leaf + 0x40, mid);
        tw.w32(other + 0x80, 2);
        for c in [base, mid, leaf, other] {
            tw.w32(c + 0x1c, 0x10);
        }
        // blocks: [.. blocks+0x100), then [.. blocks+0x200), then the end
        let blocks = tw.zeros(0x400);
        tw.w32(blocks + 0x100, blocks + 0x200);
        tw.w32(blocks + 0x200, 0);
        let entries = tw.zeros(20 * 4);
        let e = |i: u32| entries + 20 * i;
        // lists: base (2 instances, the first block empty); five leaf
        // lists over the blocks (starting at a block's end, before it,
        // in the last block, longer than their blocks); other (2)
        let n_base = instance_node(&mut tw, base, blocks + 0x100, 2, blocks + 0x100);
        let leaf_lists = [
            (blocks + 0xc0, 9, blocks + 0x100),
            (blocks + 0x1d0, 3, blocks + 0x200),
            (blocks + 0x100, 4, blocks + 0x100),
            (blocks + 0x210, 5, 0),
            (blocks + 0xf0, 2, blocks + 0x100),
        ];
        let mut leaves = Vec::new();
        for (first, count, block) in leaf_lists {
            leaves.push(instance_node(&mut tw, leaf, first, count, block));
        }
        let n_other = instance_node(&mut tw, other, blocks + 0x40, 2, blocks + 0x100);
        tw.w32(e(0), n_base);
        tw.w32(n_base, e(1));
        tw.w32(e(1), leaves[0]);
        for w in leaves.windows(2) {
            tw.w32(w[0], w[1]);
        }
        tw.w32(leaves[4], e(2));
        tw.w32(e(2), n_other);
        tw.w32(n_other, e(3));
        // depths: base and other 1, leaf 3; other derives from mid too, so
        // a walk that ran past base's subtree would count it. Each entry's
        // +0xc names a class, so a walk that took an entry for a list
        // would count it.
        tw.w32(other + 0x40, mid);
        for (i, (count, depth)) in [(2u32, 1u32), (23, 3), (2, 1)].into_iter().enumerate() {
            let at = e(u32::try_from(i).unwrap());
            tw.w32(at + 8, count);
            tw.w32(at + 0xc, leaf);
            tw.w32(at + 0x10, depth);
        }
        let world = tw.zeros(0x20);
        tw.w32(world + 0xc, entries);
        for count in [3, 0x8000_0003, 1, 0] {
            tw.w32(world + 8, count);
            for class in [base, mid, leaf, other] {
                // stale words in the port's frame (the shim's starts zeroed)
                tw.write(frame, &[0x77; 12]);
                let label = format!("{what} instance count {class:#x} lists {count:#x}");
                let r = tw.run(&label, 0x0044_3550, Conv::Thiscall(world), &[class], |g| {
                    sk::bridge::world_instance_count(g, world, class, frame)
                });
                if class == mid && count == 3 {
                    assert!(r > 0, "{label}: the walk counts");
                }
            }
        }
        // no entry anywhere on the chain
        tw.w32(base + 0x80, 0x8000_0001);
        tw.w32(world + 8, 3);
        let label = format!("{what} instance count, no list on the chain");
        tw.run(&label, 0x0044_3550, Conv::Thiscall(world), &[mid], |g| {
            sk::bridge::world_instance_count(g, world, mid, frame)
        });
    }
}

// ---- batch 12 (ffx_prender::m5p_scene_l, two m5p_scene_b bridges) ----

use ffx_prender::m5p_scene_l as sl;

/// The camera controller's look-at update: off, no node, and following
/// random nodes (the real matrix and normalize code; the look-at and fly
/// controller calls stubbed), the port's frame on the stack.
#[test]
fn m5p_scene_l_camera_follow() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sl::LOOK_AT_SET, true, 3, 3, 0x0bad_0027);
        tw.stub(sl::FLY_SET[1], true, 1, 1, 0x0bad_0028);
        tw.stub(sl::FLY_SET[0], true, 1, 1, 0x0bad_0029);
        // the port's frame where the original's is (`ebp - 0x70`: the call
        // pushes one word, the return address and `ebp` from the call
        // top), the port's own calls below it, so the stubs log the same
        // pointers
        let top = tw.a.stack_range().end - 0x100;
        let frame = top - 0xc - 0x70;
        tw.b.set_call_top(Some(frame - 0x40));
        let this = tw.zeros(0x120);
        let mut rng = Rng(71);
        let f = 0.75f32.to_bits();
        for (on, has) in [(0u8, true), (1, false), (0x80, true)] {
            tw.write(this + 0xb1, &[on]);
            tw.w32(this + 0xb4, u32::from(has));
            let label = format!("{what} camera follow, on {on}, node {has}");
            if on == 0 {
                // the original leaves its caller's eax: memory only
                let (_, r) = tw.run_any(&label, 0x0055_8b10, Conv::Thiscall(this), &[f], |g| {
                    sl::bridge::camera_follow(g, this, f32::from_bits(f), frame)
                });
                assert_eq!(r, None, "{label}");
                continue;
            }
            if !has {
                tw.run(&label, 0x0055_8b10, Conv::Thiscall(this), &[f], |g| {
                    sl::bridge::camera_follow(g, this, f32::from_bits(f), frame).unwrap_or(1)
                });
                continue;
            }
            for round in 0..10 {
                let node = tw.alloc(&float_words(&mut rng, 12));
                tw.w32(this + 0xb4, node);
                let s = float_words(&mut rng, 1);
                let f = u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
                let label = format!("{what} camera follow {round}");
                tw.run(&label, 0x0055_8b10, Conv::Thiscall(this), &[f], |g| {
                    sl::bridge::camera_follow(g, this, f32::from_bits(f), frame).unwrap_or(1)
                });
                assert_eq!(tw.logged(), 3, "{label}: calls");
                assert_eq!(tw.entry(0, 4), f, "{label}: f");
                // the position and the direction the look-at got
                let fa = Oracle::read(&mut tw.a, frame + 0x40, 0x20).unwrap();
                let fb = Oracle::read(&mut tw.b, frame + 0x40, 0x20).unwrap();
                assert_eq!(fa, fb, "{label}: the frame's vectors");
            }
        }
        tw.b.set_call_top(None);
        tw.unstub();
        // the vector getter and the device getter
        let src = tw.alloc(&float_words(&mut rng, 8));
        let out = tw.zeros(0x10);
        let label = format!("{what} get vector");
        tw.run(&label, 0x004f_8100, Conv::Thiscall(src), &[out], |g| {
            sb::bridge::get_vector3(g, src, out)
        });
        tw.stub(sb::GET_DEVICE, false, 2, 0, 0x0bad_002a);
        for n in [0, 1, 7] {
            let label = format!("{what} device {n}");
            tw.run(&label, 0x0062_8700, Conv::Stdcall, &[n], |g| {
                sb::bridge::input_device4(g, n)
            });
            assert_eq!((tw.entry(0, 2), tw.entry(0, 3)), (4, n), "{label}");
        }
        tw.unstub();
    }
}

// ---- batch 13 (ffx_prender::m5p_scene_m) ----

use ffx_prender::m5p_scene_m as sm;

/// A bump allocator at `PhyreMalloc` (cdecl `(size, align)`): blocks from
/// an arena whose cursor is its first word; `fail_at` (the arena's second
/// word) numbers the call that returns 0 (counted in the third).
fn bump_malloc(tw: &mut Twins) -> u32 {
    let arena = tw.zeros(0x1000);
    tw.w32(arena, arena + 0x10);
    tw.w32(arena + 4, u32::MAX);
    // inc [arena+8]; mov eax,[arena+8]; dec eax; cmp eax,[arena+4]; je zero;
    // mov eax,[arena]; mov ecx,[esp+4]; add ecx,0xf; and ecx,-0x10;
    // add [arena],ecx; ret; zero: xor eax,eax; ret
    let mut c = vec![0xff, 0x05];
    c.extend((arena + 8).to_le_bytes());
    c.push(0xa1);
    c.extend((arena + 8).to_le_bytes());
    c.push(0x48);
    c.extend([0x3b, 0x05]);
    c.extend((arena + 4).to_le_bytes());
    c.extend([0x74, 0x16]);
    c.push(0xa1);
    c.extend(arena.to_le_bytes());
    c.extend([0x8b, 0x4c, 0x24, 0x04, 0x83, 0xc1, 0x0f, 0x83, 0xe1, 0xf0]);
    c.extend([0x01, 0x0d]);
    c.extend(arena.to_le_bytes());
    c.push(0xc3);
    c.extend([0x31, 0xc0, 0xc3]);
    let code = tw.a.alloc_code(&c).unwrap();
    assert_eq!(tw.b.alloc_code(&c).unwrap(), code);
    let at = tw.rebase(sm::PHYRE_MALLOC);
    let mut j = vec![0xe9];
    j.extend(code.wrapping_sub(at + 5).to_le_bytes());
    let old = Oracle::read(&mut tw.a, at, 5).unwrap();
    tw.saved.entry(at).or_insert(old);
    tw.write(at, &j);
    arena
}

/// The LOD level's define string: levels 0 and others, string arrays of
/// one and other counts (owned, external), old strings owned and not;
/// allocations failing at each call. The real builder code over a bump
/// allocator; `PhyreFree` and the array release stubbed.
#[test]
fn m5p_scene_m_lod_level_strings() {
    for (what, mut tw) in Twins::all() {
        tw.stub(sm::PHYRE_FREE, false, 1, 0, 0);
        tw.stub(sm::STRINGS_RELEASE, false, 2, 0, 0x0bad_002b);
        let arena = bump_malloc(&mut tw);
        let frame = tw.b.stack_range().start + 0x1000;
        let name = tw.alloc(b"LODLEVEL_\0");
        let this = tw.zeros(0x10);
        tw.w32(this + 4, name);
        let packed = tw.alloc(&[0, 0, 0, 0, 3, 0, 0, 0, 0xff, 0xff, 0xff, 0x7f]);
        let cursor = tw.zeros(4);
        let strings = tw.zeros(8);
        let slot = tw.zeros(4);
        for word in 0..3 {
            for (count, old_buf) in [
                (1, slot),
                (0, 0),
                (2, 0x0012_3000),
                (0x8000_0002, 0x0012_3000),
            ] {
                for old_string in [0x0013_0000, 0x0013_0001] {
                    for fail in [u32::MAX, 0, 1, 2] {
                        tw.w32(arena, arena + 0x10);
                        tw.w32(arena + 4, fail);
                        tw.w32(arena + 8, 0);
                        tw.w32(cursor, packed + 4 * word);
                        tw.w32(strings, count);
                        tw.w32(strings + 4, old_buf);
                        tw.w32(slot, old_string);
                        let label = format!(
                            "{what} lod strings word {word} ({count:#x}, {old_buf:#x}) old {old_string:#x} fail {fail}"
                        );
                        tw.run(
                            &label,
                            0x004d_4ba0,
                            Conv::Thiscall(this),
                            &[cursor, 9, strings],
                            |g| sm::bridge::lod_level_strings(g, this, cursor, strings, frame),
                        );
                    }
                }
            }
        }
        tw.unstub();
    }
}

// ---- batch 14 (ffx_prender::m5p_scene_n) ----

use ffx_prender::m5p_scene_n as sn;

/// The plane test over random matrices and points (specials included),
/// and points on the planes (results of exactly 0 and -0).
#[test]
fn m5p_scene_n_planes_below() {
    for (what, mut tw) in Twins::all() {
        let mut rng = Rng(81);
        for round in 0..200 {
            let planes = tw.alloc(&float_words(&mut rng, 16));
            let point = tw.alloc(&float_words(&mut rng, 3));
            let label = format!("{what} planes below {round}");
            tw.run(&label, 0x009f_2890, Conv::Cdecl, &[planes, point], |g| {
                sn::bridge::planes_below(g, planes, point)
            });
        }
        // on the planes: x = 1, the first row 1, the last row 1 (0), and
        // a negative zero from -0 * 1 - 0
        let planes = tw.alloc(
            &[
                1.0f32, -0.0, 2.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.0,
                -2.0,
            ]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<_>>(),
        );
        let point = tw.alloc(
            &[1.0f32, 5.0, 7.0]
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let label = format!("{what} planes below, on the planes");
        let r = tw.run(&label, 0x009f_2890, Conv::Cdecl, &[planes, point], |g| {
            sn::bridge::planes_below(g, planes, point)
        });
        assert_eq!(r, 0x0001_0000, "{label}");
        // the sums' order and rounding decide the sign: plane 0 is
        // (1e8 - 1e8) + 0.5 - 0.25 (above; summed the other way round it
        // would be below), plane 1 is 1 - 1.0000001 (below; one extra
        // rounding step up would make it 0)
        let planes = tw.alloc(
            &[
                1.0e8f32,
                1.0,
                0.0,
                0.0,
                -1.0e8,
                0.0,
                0.0,
                0.0,
                0.5,
                0.0,
                0.0,
                0.0,
                0.25,
                1.000_000_1,
                0.0,
                0.0,
            ]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<_>>(),
        );
        let point = tw.alloc(
            &[1.0f32, 1.0, 1.0]
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let label = format!("{what} planes below, rounding");
        let r = tw.run(&label, 0x009f_2890, Conv::Cdecl, &[planes, point], |g| {
            sn::bridge::planes_below(g, planes, point)
        });
        assert_eq!(r, 0x0000_0100, "{label}");
    }
}

/// A joint's world matrix over a skeleton of five joints (a chain 4 -> 2
/// -> 0 and a root 1, 3 -> 1), random local and world matrices, the
/// world applied or not; joints past the count.
#[test]
fn m5p_scene_n_joint_world() {
    for (what, mut tw) in Twins::all() {
        let frame = tw.b.stack_range().start + 0x1000;
        let mut rng = Rng(91);
        let parents = tw.alloc(
            &[u32::MAX, u32::MAX, 0, 1, 2]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let skel = tw.zeros(0x40);
        tw.w32(skel + 0x2c, parents);
        let this = tw.zeros(0x80);
        tw.w32(this, skel);
        let out = tw.zeros(0x40);
        for round in 0..6 {
            let mats = tw.alloc(&float_words(&mut rng, 16 * 5));
            let world = tw.alloc(&float_words(&mut rng, 12));
            tw.w32(this + 0xc, mats);
            tw.w32(this + 4, world);
            for count in [5, 0x8000_0005, 3] {
                tw.w32(skel + 0x18, count);
                for flags in [0, 1, 2] {
                    tw.w32(this + 0x6c, flags);
                    for joint in [0, 1, 3, 4, 5, 0x8000_0000] {
                        let label = format!(
                            "{what} joint world {round} joint {joint:#x} count {count:#x} flags {flags}"
                        );
                        tw.run(
                            &label,
                            0x004f_71e0,
                            Conv::Thiscall(this),
                            &[out, joint],
                            |g| sn::bridge::joint_world(g, this, out, joint, frame),
                        );
                    }
                }
            }
        }
    }
}

/// `0x00a2f9b0` (the box between two points tested against a matrix)
/// against the original, the point transform and the outcodes running as
/// the game's code: random boxes and matrices (visible and not), boxes
/// around the origin with a projection-like matrix (visible), a quiet NaN
/// in the box.
#[test]
fn m5p_scene_big_box_between() {
    for (what, mut tw) in Twins::all() {
        let mut rng = Rng(173);
        let frame = tw.b.stack_range().start + 0x1000;
        let f = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>();
        for round in 0..120 {
            let from = tw.alloc(&float_words(&mut rng, 3));
            let to = tw.alloc(&float_words(&mut rng, 3));
            let m = tw.alloc(&float_words(&mut rng, 16));
            let label = format!("{what} box between {round}");
            tw.run(&label, 0x00a2_f9b0, Conv::Cdecl, &[from, to, m], |g| {
                sn::bridge::box_visible_between(g, from, to, m, frame)
            });
        }
        let ident = tw.alloc(&f(&[
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]));
        for (k, (lo, hi)) in [
            ([-0.5f32, -0.5, -0.5], [0.5f32, 0.5, 0.5]),
            ([2.0, 2.0, 2.0], [3.0, 3.0, 3.0]),
            ([-3.0, 0.25, 0.25], [-2.0, 0.5, 0.5]),
            ([f32::from_bits(0x7fc0_0001), 0.0, 0.0], [1.0, 1.0, 1.0]),
        ]
        .into_iter()
        .enumerate()
        {
            let from = tw.alloc(&f(&lo));
            let to = tw.alloc(&f(&hi));
            let label = format!("{what} box between, fixed {k}");
            tw.run(&label, 0x00a2_f9b0, Conv::Cdecl, &[from, to, ident], |g| {
                sn::bridge::box_visible_between(g, from, to, ident, frame)
            });
        }
        // a single corner inside the x planes decides: t + c (first
        // matrix), (t + b) + c (second); the box from 0 to (0.5, 0.5, 1)
        for (k, rows) in [
            [
                1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.2, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0,
            ],
            [
                1.0, 0.0, 0.0, 0.0, -1.2, 0.0, 0.0, 0.0, -0.6, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 1.0,
            ],
        ]
        .into_iter()
        .enumerate()
        {
            let m = tw.alloc(&f(&rows));
            let from = tw.alloc(&f(&[0.0, 0.0, 0.0]));
            let to = tw.alloc(&f(&[0.5, 0.5, 1.0]));
            let label = format!("{what} box between, one corner inside {k}");
            let r = tw.run(&label, 0x00a2_f9b0, Conv::Cdecl, &[from, to, m], |g| {
                sn::bridge::box_visible_between(g, from, to, m, frame)
            });
            assert_eq!(r, 1, "{label}");
        }
    }
}

// ---- big-phyre2: the occluder classes' Bind tables (h1_bind) ----

/// Recording stubs for every callee of a `Bind`: the data member
/// constructor, the class member constructor and its finder (answering
/// `found`), `RegisterType`, the type getters, `atexit` and
/// `addToNamespace` (returning a tag, the function's `eax`).
fn bind_stubs(tw: &mut Twins, found: u32) {
    tw.stub(0x0057_52a0, true, 6, 6, 0);
    tw.stub(0x0043_a080, true, 3, 3, 0);
    tw.stub(0x0043_59d0, true, 1, 1, found);
    tw.stub(0x0043_c140, true, 0, 0, 0x0043_c140);
    tw.stub(0x0043_95b0, false, 1, 0, 0);
    tw.stub(0x0094_9207, false, 1, 0, 0);
    tw.stub(0x0043_7fc0, false, 0, 0, 0x0043_7fc0);
    tw.stub(0x0045_fd90, false, 0, 0, 0x0045_fd90);
}

#[test]
fn m5p_scene_bp2_occluder_binds() {
    use ffx_prender::h1_bind::bridge::bind;
    use ffx_prender::m5p_scene_bp2 as b;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x00a3_0c90);
        // the guards as found (all built on the snapshots: register, the
        // caller stores, addToNamespace), callees stubbed (the original
        // RegisterType walks heap state the snapshot's loader owns)
        bind_stubs(&mut tw, 0);
        let what = format!("{label}: instance bind as found");
        tw.run(&what, b::INSTANCE_BIND_ADDR, Conv::Cdecl, &[], |g| {
            bind(g, b::INSTANCE_BIND)
        });
        let what = format!("{label}: object bind as found");
        tw.run(&what, b::OBJECT_BIND_ADDR, Conv::Cdecl, &[], |g| {
            bind(g, b::OBJECT_BIND)
        });
        tw.unstub();
        // stubbed callees: every guard bit clear, all set, random bits; the
        // method member with a name or none, found or not
        let owner = tw.zeros(0x100);
        let tail = tw.zeros(0x10);
        let method = tw.rebase(0x01a8_5510);
        for round in 0..12u32 {
            let found = u32::from(round % 3 == 2);
            bind_stubs(&mut tw, found);
            let guard = match round {
                0..=2 => 0,
                3 => 0xffff_ffff,
                _ => rng.next(),
            };
            let at = tw.rebase(b::INSTANCE_GUARD);
            tw.w32(at, guard);
            let at = tw.rebase(b::OBJECT_GUARD);
            tw.w32(at, guard.rotate_left(round));
            // the class member as its constructor leaves it: owner, name
            tw.w32(method + 8, owner);
            tw.w32(method + 0xc, if round == 1 { 0 } else { 0x00b1_071c });
            tw.w32(owner + 0x4c, owner + 0x4c);
            tw.w32(owner + 0x50, tail);
            let flags = tw.rebase(0x01a8_5a20);
            let v = rng.next();
            tw.w32(flags, v);
            let what = format!("{label}: instance bind {round}");
            tw.run(&what, b::INSTANCE_BIND_ADDR, Conv::Cdecl, &[], |g| {
                bind(g, b::INSTANCE_BIND)
            });
            let what = format!("{label}: object bind {round}");
            tw.run(&what, b::OBJECT_BIND_ADDR, Conv::Cdecl, &[], |g| {
                bind(g, b::OBJECT_BIND)
            });
            tw.unstub();
        }
    }
}

/// A float in `[-10, 10)` in steps of 0.01.
fn centi(rng: &mut Rng) -> f32 {
    f32::from(u16::try_from(rng.next() % 2000).unwrap()) / 100.0 - 10.0
}

/// `0x005d6f00` with the original matrix callees on both sides: random
/// controllers and targets, a target straight above and below (`|f.y|`
/// over 0.999), just under the limit, the target at the position (a zero
/// direction: infinities and NaNs), NaN and infinite inputs (only
/// synthetic data reaches it: no camera controller flies on the
/// snapshots). The port's calls run below the original's frame, whose words are compared
/// after the call (`ebp - 4` holds only a copy of a word the original
/// never writes: not compared).
#[test]
fn m5p_scene_bp2_look_at() {
    use ffx_prender::m5p_scene_bp2::look::look_at;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x005d_6f00);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 - 8;
        for k in 0..240u32 {
            let this = tw.zeros(0x80);
            let mut m: Vec<f32> = (0..12).map(|_| centi(&mut rng)).collect();
            let mut t: Vec<f32> = (0..3).map(|_| centi(&mut rng)).collect();
            // the position is the 4x4's row 3: the 3x4's (m8, m9, m10)
            match k {
                0 => t = vec![m[8], m[9] + 5.0, m[10]],
                1 => t = vec![m[8], m[9] - 5.0, m[10]],
                2 => t = vec![m[8] + 0.04, m[9] + 1.0, m[10]],
                3 => t = vec![m[8] + 0.0447, m[9] + 1.0, m[10]],
                4 => t = vec![m[8], m[9], m[10]],
                5 => t[1] = f32::NAN,
                6 => m[9] = f32::from_bits(0xffc0_1234),
                7 => m[0] = f32::INFINITY,
                // a direction (0, 1, z) whose normalised y is exactly the
                // limit f32(0.999) (found by search): not over it
                8 | 9 => {
                    m[8] = 0.0;
                    m[9] = 0.0;
                    m[10] = 0.0;
                    let z = f32::from_bits(0x3d37_4fac);
                    t = vec![0.0, if k == 8 { -1.0 } else { 1.0 }, -z];
                }
                _ => {}
            }
            let bytes: Vec<u8> = m.iter().flat_map(|f| f.to_le_bytes()).collect();
            tw.write(this + 4, &bytes);
            let bytes: Vec<u8> = t.iter().flat_map(|f| f.to_le_bytes()).collect();
            let target = tw.alloc(&bytes);
            let junk: Vec<u8> = (0..0x90u32).map(|i| (i * 7 + k) as u8).collect();
            tw.write(ebp - 0x90, &junk);
            tw.b.set_call_top(Some(ebp - 0x90 - 0x100));
            let what = format!("{label}: look at {k}");
            tw.run(&what, 0x005d_6f00, Conv::Thiscall(this), &[target], |g| {
                look_at(g, ebp, this, target)
            });
            tw.b.set_call_top(None);
            let mut x = vec![0; 0x8c];
            let mut y = vec![0; 0x8c];
            tw.a.read_bytes(ebp - 0x90, &mut x);
            tw.b.read_bytes(ebp - 0x90, &mut y);
            assert_eq!(x, y, "{what}: frame");
        }
    }
}

/// A plane set for `planesBelow` (four planes as the columns of a 4x4):
/// `Some(true)` every point below all four, `Some(false)` no point
/// below any, `None` random planes.
fn plane_set(rng: &mut Rng, kind: Option<bool>) -> Vec<u8> {
    let w: Vec<f32> = (0..16)
        .map(|i| match kind {
            Some(below) if i >= 12 => {
                if below {
                    1.0
                } else {
                    -1.0
                }
            }
            Some(_) => 0.0,
            None => (tw_rand(rng) - 0.5) * 4.0,
        })
        .collect();
    w.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn tw_rand(rng: &mut Rng) -> f32 {
    f32::from(u16::try_from(rng.next() % 10_000).unwrap()) / 10_000.0
}

/// A case of [`m5p_scene_bp2_box_planes`]: the outer plane sets, an
/// empty range, the inner set, a NaN extent.
type BoxCase = (Vec<Option<bool>>, bool, Option<bool>, bool);

/// `0x009f2a60` with the original matrix and plane callees on both sides:
/// outer plane sets that let every corner through (one, several, an
/// empty range: the loop runs once), one that a corner is below (first
/// and later in the range), random planes and boxes, inner sets passing
/// every corner, none, or some; a NaN extent. The corners are compared in
/// the frame after the call. Only synthetic data reaches it.
#[test]
fn m5p_scene_bp2_box_planes() {
    use ffx_prender::m5p_scene_bp2::box_planes::box_planes;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x009f_2a60);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 6 - 8;
        let mut seen = std::collections::BTreeSet::new();
        // (outer sets, empty range, inner set, NaN extent)
        let cases: Vec<BoxCase> = vec![
            (vec![Some(false)], false, Some(false), false),
            (vec![Some(false)], false, Some(true), false),
            (
                vec![Some(false), Some(false), Some(false)],
                false,
                None,
                false,
            ),
            (vec![Some(false), Some(true)], false, Some(false), false),
            (vec![Some(true)], false, Some(false), false),
            (vec![Some(false)], true, Some(true), false),
            (vec![None], false, None, false),
            (vec![None, None], false, None, false),
            (vec![Some(false)], false, None, true),
            // patched below: a set some corners are below (x < 0)
            (vec![Some(true)], false, Some(false), false),
        ];
        let cut = cases.len() - 1;
        for k in 0..30usize {
            let (outer, empty, inner, nan) = if k < cases.len() {
                cases[k].clone()
            } else {
                (vec![Some(false)], false, None, false)
            };
            let mut planes = Vec::new();
            for &o in &outer {
                planes.extend(plane_set(&mut rng, o));
            }
            if k == cut {
                // plane 0 is x < 0, the others never below; the box
                // -0.5..0.5 under the identity straddles it
                let mut set = [0f32; 16];
                set[0] = 1.0;
                set[13] = -1.0;
                set[14] = -1.0;
                set[15] = -1.0;
                planes = set.iter().flat_map(|f| f.to_le_bytes()).collect();
            }
            let first = tw.alloc(&planes);
            let last = if empty {
                first
            } else {
                first + 0x40 * outer.len() as u32
            };
            let bytes = plane_set(&mut rng, inner);
            let inner_at = tw.alloc(&bytes);
            let origin: Vec<u8> = (0..3)
                .flat_map(|_| {
                    if k == cut {
                        -0.5
                    } else {
                        (tw_rand(&mut rng) - 0.5) * 2.0
                    }
                    .to_le_bytes()
                })
                .collect();
            let origin = tw.alloc(&origin);
            let mut e: Vec<f32> = (0..3)
                .map(|_| if k == cut { 1.0 } else { tw_rand(&mut rng) })
                .collect();
            if nan {
                e[1] = f32::NAN;
            }
            let e: Vec<u8> = e.iter().flat_map(|f| f.to_le_bytes()).collect();
            let extent = tw.alloc(&e);
            let m: Vec<u8> = (0..16)
                .flat_map(|i| {
                    let v = if i % 5 == 0 {
                        1.0
                    } else if k == cut {
                        0.0
                    } else {
                        (tw_rand(&mut rng) - 0.5) * 0.5
                    };
                    f32::to_le_bytes(v)
                })
                .collect();
            let m = tw.alloc(&m);
            let junk: Vec<u8> = (0..0x80u32).map(|i| (i * 13 + k as u32) as u8).collect();
            tw.write(ebp - 0x80, &junk);
            tw.b.set_call_top(Some(ebp - 0x80 - 0x100));
            let what = format!("{label}: box planes {k}");
            let args = [first, last, inner_at, origin, extent, m];
            let r = tw.run(&what, 0x009f_2a60, Conv::Cdecl, &args, |g| {
                box_planes(g, ebp, first, last, inner_at, origin, extent, m)
            });
            seen.insert(r);
            tw.b.set_call_top(None);
            let mut x = vec![0; 0x80];
            let mut y = vec![0; 0x80];
            tw.a.read_bytes(ebp - 0x80, &mut x);
            tw.b.read_bytes(ebp - 0x80, &mut y);
            assert_eq!(x, y, "{what}: frame");
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            [0, 1, u32::MAX],
            "{label}: results"
        );
    }
}

/// An occluder of [`m5p_scene_bp2_box_occluded`]: (plane sets, block);
/// `Some(false)` lets every point through, `Some(true)` has every point
/// below, `None` random.
type Occ = (Vec<Option<bool>>, Option<bool>);

/// `PVisibilityCheck_isBoxOccluded` with the original matrix and plane
/// callees on both sides: no occluders (a zero count first); an occluder
/// whose plane sets let every corner through and whose block every
/// corner is below (occluded: 0), one with a corner outside a set
/// (skipped), one whose block lets a corner through, the second or
/// fourth occluder occluding, random planes; a NaN extent. The corners
/// and the origin copy are compared in the frame after the call. Only
/// synthetic data reaches it.
#[test]
fn m5p_scene_bp2_box_occluded() {
    use ffx_prender::m5p_scene_bp2::box_occluded::is_box_occluded;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x009f_4100);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 3 - 8;
        let mut seen = std::collections::BTreeSet::new();
        let never = Some(false);
        let all = Some(true);
        let cases: Vec<(Vec<Occ>, bool)> = vec![
            (vec![], false),
            (vec![(vec![never], all)], false),
            (vec![(vec![never, never], never)], false),
            (vec![(vec![never, all], all)], false),
            (vec![(vec![all], all), (vec![never], all)], false),
            (
                vec![
                    (vec![never], never),
                    (vec![never], never),
                    (vec![never], never),
                    (vec![never; 3], all),
                ],
                false,
            ),
            (vec![(vec![None], None), (vec![None, None], None)], false),
            (vec![(vec![never], None), (vec![None], all)], false),
            (vec![(vec![never], all)], true),
            // four occluders, none occluding: the list's end
            (vec![(vec![never], never); 4], false),
            // patched below: a set some corners are below (x < 0)
            (vec![(vec![all], all)], false),
        ];
        let cut = cases.len() - 1;
        for k in 0..24usize {
            let (occs, nan) = cases
                .get(k)
                .cloned()
                .unwrap_or_else(|| (vec![(vec![None], None), (vec![never], None)], false));
            let mut counts = [0u32; 4];
            let mut sets = Vec::new();
            let mut blocks = vec![0u8; 0x40 * 4];
            for (i, (s, b)) in occs.iter().enumerate() {
                counts[i] = s.len() as u32;
                for &p in s {
                    sets.extend(plane_set(&mut rng, p));
                }
                blocks[0x40 * i..0x40 * (i + 1)].copy_from_slice(&plane_set(&mut rng, *b));
            }
            let mut o = vec![0u8; 0x310 + 0x40 * 4];
            for (i, c) in counts.iter().enumerate() {
                o[4 * i..4 * i + 4].copy_from_slice(&c.to_le_bytes());
            }
            o[0x10..0x10 + sets.len()].copy_from_slice(&sets);
            o[0x310..].copy_from_slice(&blocks);
            if k == cut {
                // plane 0 is x < 0, the others never below; the box
                // -0.5..0.5 under the identity straddles it
                let mut set = [0f32; 16];
                set[0] = 1.0;
                set[13] = -1.0;
                set[14] = -1.0;
                set[15] = -1.0;
                let bytes: Vec<u8> = set.iter().flat_map(|f| f.to_le_bytes()).collect();
                o[0x10..0x50].copy_from_slice(&bytes);
            }
            let occluders = tw.alloc(&o);
            let origin: Vec<u8> = (0..3)
                .flat_map(|_| {
                    if k == cut {
                        -0.5
                    } else {
                        (tw_rand(&mut rng) - 0.5) * 2.0
                    }
                    .to_le_bytes()
                })
                .collect();
            let origin = tw.alloc(&origin);
            let mut e: Vec<f32> = (0..3)
                .map(|_| if k == cut { 1.0 } else { tw_rand(&mut rng) })
                .collect();
            if nan {
                e[2] = f32::NAN;
            }
            let e: Vec<u8> = e.iter().flat_map(|f| f.to_le_bytes()).collect();
            let extent = tw.alloc(&e);
            let m: Vec<u8> = (0..16)
                .flat_map(|i| {
                    let v = if i % 5 == 0 {
                        1.0
                    } else if k == cut {
                        0.0
                    } else {
                        (tw_rand(&mut rng) - 0.5) * 0.5
                    };
                    f32::to_le_bytes(v)
                })
                .collect();
            let m = tw.alloc(&m);
            let junk: Vec<u8> = (0..0x118u32).map(|i| (i * 13 + k as u32) as u8).collect();
            tw.write(ebp - 0x118, &junk);
            tw.b.set_call_top(Some(ebp - 0x118 - 0x100));
            let what = format!("{label}: box occluded {k}");
            let r = tw.run_al(
                &what,
                0x009f_4100,
                Conv::Thiscall(occluders),
                &[origin, extent, m],
                |g| is_box_occluded(g, ebp, occluders, origin, extent, m),
            );
            seen.insert(r);
            tw.b.set_call_top(None);
            // the corners and the origin copy (the rest holds the
            // original's copies of its inputs)
            for (from, len) in [
                (0x118u32, 0xc),
                (0x104, 0xc),
                (0xf4, 0xc),
                (0xe4, 0xc),
                (0xd4, 0xc),
                (0x54, 0xc),
                (0x40, 0xc),
                (0x24, 0x1c),
            ] {
                let mut x = vec![0; len];
                let mut y = vec![0; len];
                tw.a.read_bytes(ebp - from, &mut x);
                tw.b.read_bytes(ebp - from, &mut y);
                assert_eq!(x, y, "{what}: frame at -{from:#x}");
            }
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            [0, 1],
            "{label}: results"
        );
    }
}

/// `0x00a2fe70` with the original matrix and min/max callees on both
/// sides: rays from inside the box and from outside towards it and away,
/// a direction with zero components (infinite reciprocals), a NaN start,
/// `near`/`far` null or not; random boxes, matrices and rays. The frame's
/// distances are compared after the call. Only synthetic data reaches it.
#[test]
fn m5p_scene_bp2_ray_box() {
    use ffx_prender::m5p_scene_bp2::ray_box::ray_box;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x00a2_fe70);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 7 - 8;
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..120u32 {
            let mut r = || (tw_rand(&mut rng) - 0.5) * 4.0;
            let lo = [r() - 1.0, r() - 1.0, r() - 1.0];
            let hi = [lo[0] + 2.0, lo[1] + 1.5, lo[2] + 2.5];
            let mut p = match k % 3 {
                0 => [lo[0] + 0.5, lo[1] + 0.5, lo[2] + 0.5],
                _ => [r() * 2.0, r() * 2.0, r() * 2.0],
            };
            let mut d = [r(), r(), r()];
            if k % 7 == 3 {
                d[1] = 0.0;
            }
            if k % 11 == 5 {
                d = [0.0, 0.0, r()];
            }
            if k == 13 || k == 14 {
                // a NaN start, inside the box on the other axes (14: and
                // in x too, so the NaN alone decides)
                p = [f32::NAN, lo[1] + 0.5, lo[2] + 0.5];
            }
            if k == 15 {
                p = [lo[0] + 0.5, f32::NAN, f32::NAN];
            }
            let m: Vec<f32> = (0..16)
                .map(|i| {
                    if i % 5 == 0 {
                        1.0
                    } else if i % 4 == 3 {
                        0.0
                    } else {
                        (tw_rand(&mut rng) - 0.5) * 0.4
                    }
                })
                .collect();
            let bytes = |v: &[f32]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
            let m = tw.alloc(&bytes(&m));
            let lo_at = tw.alloc(&bytes(&lo));
            let hi_at = tw.alloc(&bytes(&hi));
            let p_at = tw.alloc(&bytes(&[p[0], p[1], p[2], 1.0]));
            let d_at = tw.alloc(&bytes(&[d[0], d[1], d[2], 0.0]));
            let near = if k % 5 == 4 { 0 } else { tw.zeros(4) };
            let far = if k % 6 == 5 { 0 } else { tw.zeros(4) };
            let junk: Vec<u8> = (0..0x4cu32).map(|i| (i * 3 + k) as u8).collect();
            tw.write(ebp - 0x4c, &junk);
            tw.b.set_call_top(Some(ebp - 0x4c - 0x100));
            let what = format!("{label}: ray box {k}");
            let args = [m, lo_at, hi_at, p_at, d_at, near, far];
            let res = tw.run_al(&what, 0x00a2_fe70, Conv::Cdecl, &args, |g| {
                ray_box(g, ebp, m, lo_at, hi_at, p_at, d_at, near, far)
            });
            seen.insert(res);
            tw.b.set_call_top(None);
            // the distances and their min/max (the rest holds the
            // original's compare copies)
            for from in [0x4cu32, 0x3c, 0x2c, 0x1c] {
                let mut x = vec![0; 0xc];
                let mut y = vec![0; 0xc];
                tw.a.read_bytes(ebp - from, &mut x);
                tw.b.read_bytes(ebp - from, &mut y);
                assert_eq!(x, y, "{what}: frame at -{from:#x}");
            }
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            [0, 1],
            "{label}: results"
        );
    }
}

impl Twins {
    /// A stub at `sym` that logs its tag and `n` stack words, then loads
    /// the `f32` `value` into `ST0` (cdecl).
    fn float_const_stub(&mut self, sym: u32, n: u32, value: f32) {
        let at = self.rebase(sym);
        let log_code = self.stub_code(sym, false, n, 0, 0);
        let cell = self.alloc(&value.to_bits().to_le_bytes());
        let here = self.a.alloc_code(&[0; 48]).unwrap();
        assert_eq!(self.b.alloc_code(&[0; 48]).unwrap(), here);
        // the log stub reads its words from esp+4: jump to it with the
        // caller's frame intact by calling it from a copy of the words
        let mut c = vec![0xff, 0x74, 0x24, 0x14]; // push [esp+0x14] (word 4)
        c.extend([0xff, 0x74, 0x24, 0x14]); // push [esp+0x14] (word 3)
        c.extend([0xff, 0x74, 0x24, 0x14]); // word 2
        c.extend([0xff, 0x74, 0x24, 0x14]); // word 1
        c.extend([0xff, 0x74, 0x24, 0x14]); // word 0
        c.push(0xe8);
        c.extend(
            log_code
                .wrapping_sub(here + c.len() as u32 + 4)
                .to_le_bytes(),
        ); // call log
        c.extend([0x83, 0xc4, 0x14]); // add esp, 0x14
        c.extend([0xd9, 0x05]);
        c.extend(cell.to_le_bytes()); // fld dword [cell]
        c.push(0xc3);
        assert!(c.len() <= 48);
        self.write(here, &c);
        let mut j = vec![0xe9];
        j.extend(here.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }
}

/// `0x00a30260` with the matrix, box, visibility, length and ray callees
/// original on both sides (the array's reset, shrink and free, and the
/// occluder weight `0x00a2ef70` stubbed): no stride; no items with the
/// array owned or not; a null list; the array growing (and failing to);
/// items with and without their 3x4, a camera inside one box, with and
/// without the owner's camera, with and without a ray (one blocked
/// ahead, one not). Only synthetic data reaches it.
#[test]
#[allow(clippy::type_complexity)] // the case tables
fn m5p_scene_bp2_select_occluders() {
    use ffx_prender::m5p_scene_bp2::{SELECT_CALLS, select::select_occluders};
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x00a3_0260);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 6 - 8;
        let mut kept = 0u32;
        // (count, stride, items null, capacity, owned, reset error,
        // camera, ray)
        let cases: &[(u32, u32, bool, u32, bool, u32, bool, bool)] = &[
            (4, 0, false, 8, false, 0, true, false),
            (0, 8, false, 8, false, 0, true, false),
            (0, 8, false, 8, true, 0, true, false),
            (4, 8, true, 8, false, 0, true, false),
            (6, 8, false, 4, false, 0xd, true, false),
            (6, 12, false, 4, false, 0, true, false),
            (6, 8, false, 8, false, 0, false, false),
            (8, 8, false, 8, false, 0, true, true),
            (8, 16, false, 8, false, 0, true, true),
            (8, 8, false, 8, false, 0, false, true),
        ];
        for (k, &(count, stride, null, cap, owned, reset_err, camera, with_ray)) in
            cases.iter().enumerate()
        {
            for round in 0..3u32 {
                let data = tw.alloc(&(0..8 * 16).map(|i: u8| i ^ k as u8).collect::<Vec<u8>>());
                let array = tw.zeros(8);
                tw.w32(array, cap | if owned { 0x8000_0000 } else { 0 });
                tw.w32(array + 4, data);
                let n = count.max(1) as usize;
                let list = tw.zeros(stride.max(8) as usize * n);
                for i in 0..n as u32 {
                    let g = tw.zeros(0x30);
                    // boxes around the origin's neighbourhood; item 1 of
                    // round 1 holds the camera
                    let c = [
                        (tw_rand(&mut rng) - 0.5) * 6.0,
                        (tw_rand(&mut rng) - 0.5) * 6.0,
                        5.0 + tw_rand(&mut rng) * 4.0,
                    ];
                    let s = 0.5 + tw_rand(&mut rng);
                    let (lo, hi) = if round == 1 && i == 1 {
                        ([-50.0f32, -50.0, -50.0], [50.0f32, 50.0, 50.0])
                    } else {
                        (
                            [c[0] - s, c[1] - s, c[2] - s],
                            [c[0] + s, c[1] + s, c[2] + s],
                        )
                    };
                    let b: Vec<u8> = lo
                        .iter()
                        .chain(&[0.0])
                        .chain(&hi)
                        .flat_map(|f| f.to_le_bytes())
                        .collect();
                    tw.write(g + 0x10, &b);
                    let at = list + i * stride.max(8);
                    tw.w32(at, g);
                    if i % 2 == 1 {
                        let m: Vec<u8> = (0..12)
                            .flat_map(|j| {
                                if j % 5 == 0 {
                                    1.0f32
                                } else if j % 4 == 3 {
                                    0.25
                                } else {
                                    0.0
                                }
                                .to_le_bytes()
                            })
                            .collect();
                        let m = tw.alloc(&m);
                        tw.w32(at + 4, m);
                    }
                }
                let owner = tw.zeros(0xc0);
                // a perspective-like view: the identity with z towards w
                let view: [f32; 16] = [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -0.1, 0.0,
                ];
                let vb: Vec<u8> = view.iter().flat_map(|f| f.to_le_bytes()).collect();
                tw.write(owner + 0x70, &vb);
                if camera {
                    let cam = tw.zeros(0x30);
                    tw.write(
                        cam + 0x20,
                        &[0.0f32, 0.0, 0.0].map(f32::to_le_bytes).concat(),
                    );
                    tw.w32(owner + 0xb0, cam);
                }
                let ray = if with_ray {
                    let to = if round == 2 {
                        [0.0f32, 0.0, 40.0]
                    } else {
                        [0.5f32, 0.25, 6.0]
                    };
                    let r: Vec<u8> = [0.0f32, 0.0, 0.0, 0.0]
                        .iter()
                        .chain(&to)
                        .flat_map(|f| f.to_le_bytes())
                        .collect();
                    tw.alloc(&r)
                } else {
                    0
                };
                tw.stub(SELECT_CALLS[0], false, 1, 0, 0);
                tw.stub(SELECT_CALLS[1], true, 1, 1, reset_err);
                tw.stub(SELECT_CALLS[13], true, 1, 1, 0x3078);
                tw.float_const_stub(
                    SELECT_CALLS[12],
                    5,
                    2.5 + f32::from(u8::try_from(round).unwrap()),
                );
                let items = if null { 0 } else { list };
                let junk: Vec<u8> = (0..0x34cu32).map(|i| (i * 7 + k as u32) as u8).collect();
                tw.write(ebp - 0x34c, &junk);
                tw.b.set_call_top(Some(ebp - 0x34c - 0x100));
                let what = format!("{label}: select occluders {k}.{round}");
                let args = [array, items, count, stride, owner, ray];
                tw.run(&what, 0x00a3_0260, Conv::Cdecl, &args, |g| {
                    select_occluders(g, ebp, array, items, count, stride, owner, ray)
                });
                tw.b.set_call_top(None);
                kept += tw.logged();
                tw.unstub();
            }
        }
        assert!(kept > 0, "{label}: no calls");
    }
}

/// `0x00a30260` on a fixed scene, so that each test changes the answer:
/// a box ahead (no 3x4), a box whose 3x4 moves it around the camera
/// (dropped only through the right inverse), a rotated and moved box (the
/// view times its 3x4), the camera off the origin; no ray, rays that end
/// past the box ahead (blocked), inside it, miss it, and start in it.
#[test]
#[allow(clippy::type_complexity)] // the case tables
fn m5p_scene_bp2_select_occluders_fixed() {
    use ffx_prender::m5p_scene_bp2::{SELECT_CALLS, select::select_occluders};
    let f = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    for (label, mut tw) in Twins::all() {
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 6 - 8;
        let rays: [Option<([f32; 3], [f32; 3])>; 8] = [
            None,
            Some(([0.1, 0.05, 0.0], [0.0, 0.0, 10.0])),
            Some(([0.1, 0.05, 0.0], [0.1, 0.1, 5.0])),
            Some(([0.1, 0.05, 0.0], [10.0, 0.0, 0.0])),
            Some(([0.0, 0.0, 5.0], [0.0, 0.0, 10.0])),
            // starting on the face (near exactly 0), ending on the far
            // face (far exactly the length), slanted through the box
            Some(([0.0, 0.0, 4.0], [0.0, 0.0, 10.0])),
            Some(([0.0, 0.0, 0.0], [0.0, 0.0, 6.0])),
            Some(([2.0, 0.0, 0.0], [-2.0, 0.0, 10.0])),
        ];
        for (k, ray) in rays.iter().enumerate() {
            // (lo, hi, 3x4 as FromMatrix3x4 reads it: the 4x4's rows
            // (m3, m7, m11), (m0, m1, m2), (m4, m5, m6) and the
            // translation (m8, m9, m10))
            let boxes: [([f32; 3], [f32; 3], Option<[f32; 12]>); 3] = [
                ([-1.0, -1.0, 4.0], [1.0, 1.0, 6.0], None),
                (
                    [-1.0, -1.0, -9.0],
                    [1.0, 1.0, -7.0],
                    Some([0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 8.0, 0.0]),
                ),
                (
                    [-1.0, -1.0, -1.0],
                    [1.0, 1.0, 1.0],
                    Some([0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 3.0, 0.0, 6.0, -1.0]),
                ),
            ];
            let list = tw.zeros(8 * boxes.len());
            for (i, (lo, hi, m)) in boxes.iter().enumerate() {
                let g = tw.zeros(0x30);
                let mut b = f(lo);
                b.extend([0; 4]);
                b.extend(f(hi));
                tw.write(g + 0x10, &b);
                tw.w32(list + 8 * i as u32, g);
                if let Some(m) = m {
                    let m = tw.alloc(&f(m));
                    tw.w32(list + 8 * i as u32 + 4, m);
                }
            }
            let data = tw.zeros(8 * 8);
            let array = tw.zeros(8);
            tw.w32(array, 8);
            tw.w32(array + 4, data);
            let owner = tw.zeros(0xc0);
            let view: [f32; 16] = [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, -0.1, 0.0,
            ];
            tw.write(owner + 0x70, &f(&view));
            let cam = tw.zeros(0x30);
            tw.write(cam + 0x20, &f(&[0.2, -0.1, 0.3, 7.0]));
            tw.w32(owner + 0xb0, cam);
            let ray_at = match ray {
                None => 0,
                Some((from, to)) => {
                    let mut r = f(from);
                    r.extend([0; 4]);
                    r.extend(f(to));
                    tw.alloc(&r)
                }
            };
            tw.stub(SELECT_CALLS[0], false, 1, 0, 0);
            tw.stub(SELECT_CALLS[1], true, 1, 1, 0);
            tw.stub(SELECT_CALLS[13], true, 1, 1, 0x3078);
            tw.float_const_stub(SELECT_CALLS[12], 5, 1.5);
            tw.b.set_call_top(Some(ebp - 0x34c - 0x100));
            let what = format!("{label}: fixed select {k}");
            let args = [array, list, 3, 8, owner, ray_at];
            tw.run(&what, 0x00a3_0260, Conv::Cdecl, &args, |g| {
                select_occluders(g, ebp, array, list, 3, 8, owner, ray_at)
            });
            tw.b.set_call_top(None);
            tw.unstub();
        }
    }
}

/// `0x00a2ef70` with every callee original on both sides (the clip
/// planes, the area): the camera inside the box (1), without a camera
/// (the origin), boxes ahead in the middle of the screen, crossing each
/// screen edge (clipped), covering it all (1), behind the camera (a
/// corner's `1/w` below 1e-5: 1), the box moved by its matrix. The
/// frame's corners, bounds and polygons are compared after the call. Only
/// synthetic data reaches it.
#[test]
#[allow(clippy::type_complexity)] // the case table
fn m5p_scene_bp2_screen_cover() {
    use ffx_prender::m5p_scene_bp2::cover::screen_cover;
    let f = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    for (label, mut tw) in Twins::all() {
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 5 - 8;
        let mut seen = Vec::new();
        // (lo, hi, camera, moved by (0, 0, 5))
        let cases: &[([f32; 3], [f32; 3], Option<[f32; 3]>, bool)] = &[
            (
                [-1.0, -1.0, -1.0],
                [1.0, 1.0, 1.0],
                Some([0.2, 0.1, 0.0]),
                false,
            ),
            ([-1.0, -1.0, 5.0], [1.0, 1.0, 7.0], None, false),
            (
                [-1.0, -1.0, 5.0],
                [1.0, 1.0, 7.0],
                Some([0.3, -0.2, 0.0]),
                false,
            ),
            (
                [-0.5, -0.5, 8.0],
                [0.5, 0.5, 9.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [4.0, -1.0, 5.0],
                [7.0, 1.0, 7.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-7.0, -1.0, 5.0],
                [-4.0, 1.0, 7.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-1.0, 4.0, 5.0],
                [1.0, 7.0, 7.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-1.0, -7.0, 5.0],
                [1.0, -4.0, 7.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-30.0, -30.0, 2.0],
                [30.0, 30.0, 3.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-6.0, -6.0, 5.0],
                [6.0, 6.0, 6.0],
                Some([1.0, 0.5, 0.0]),
                false,
            ),
            (
                [-1.0, -1.0, -7.0],
                [1.0, 1.0, -5.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-1.0, -1.0, -1.0],
                [1.0, 1.0, 1.0],
                Some([0.5, 0.2, 0.0]),
                true,
            ),
            (
                [-2.0, -3.0, 4.0],
                [3.0, 2.0, 9.0],
                Some([0.1, 0.1, -1.0]),
                false,
            ),
            // wider than the screen in x (clipped on both sides), past
            // two edges, a camera off every axis
            (
                [-10.0, -0.5, 5.0],
                [10.0, 0.5, 6.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [0.5, 0.5, 5.0],
                [10.0, 10.0, 6.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-10.0, -10.0, 5.0],
                [-0.5, -0.5, 6.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-0.5, -10.0, 5.0],
                [0.5, 10.0, 6.0],
                Some([0.0, 0.0, 0.0]),
                false,
            ),
            (
                [-1.0, -1.0, 5.0],
                [1.0, 1.0, 7.0],
                Some([0.5, 2.0, -1.0]),
                false,
            ),
            (
                [-1.0, -1.0, 5.0],
                [1.0, 1.0, 7.0],
                Some([-0.7, -1.5, 1.0]),
                true,
            ),
        ];
        for (k, &(lo, hi, camera, moved)) in cases.iter().enumerate() {
            let g = tw.alloc(&f(&[lo[0], lo[1], lo[2], 0.0, hi[0], hi[1], hi[2], 0.0]));
            let owner = tw.zeros(0xc0);
            let view: [f32; 16] = [
                1.0, 0.05, 0.02, 0.01, 0.03, -1.0, 0.04, 0.02, 0.01, 0.02, 1.0, 1.0, 0.1, 0.2, 0.3,
                0.05,
            ];
            tw.write(owner + 0x70, &f(&view));
            if let Some(c) = camera {
                let cam = tw.zeros(0x30);
                tw.write(cam + 0x20, &f(&c));
                tw.w32(owner + 0xb0, cam);
            }
            let t = if moved { 5.0 } else { 0.0 };
            let m = tw.alloc(&f(&[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, t, 1.0,
            ]));
            let inv = tw.alloc(&f(&[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -t, 1.0,
            ]));
            let junk: Vec<u8> = (0..0x2b8u32).map(|i| (i * 5 + k as u32) as u8).collect();
            tw.write(ebp - 0x2b8, &junk);
            tw.b.set_call_top(Some(ebp - 0x2b8 - 0x100));
            let what = format!("{label}: screen cover {k}");
            let args = [g, g + 0x10, owner, m, inv];
            let (ra, r) = tw.run_any(&what, 0x00a2_ef70, Conv::Cdecl, &args, |gm| {
                screen_cover(gm, ebp, g, g + 0x10, owner, m, inv)
            });
            tw.b.set_call_top(None);
            let st0 = ra.st0_f32().unwrap_or_else(|| panic!("{what}: no ST0"));
            assert_eq!(st0.to_bits(), r.to_bits(), "{what}: ST0");
            seen.push(r);
            // the corners, bounds, clipped polygons and the result word
            for (from, len) in [
                (0x64u32, 0x60usize),
                (0x124, 0xc0),
                (0x1e4, 0xc0),
                (0x1e8, 4),
                (0x24c, 0x20),
            ] {
                let mut x = vec![0; len];
                let mut y = vec![0; len];
                tw.a.read_bytes(ebp - from, &mut x);
                tw.b.read_bytes(ebp - from, &mut y);
                assert_eq!(x, y, "{what}: frame at -{from:#x}");
            }
        }
        assert!(
            seen.iter().any(|&r| r > 0.0 && r < 1.0),
            "{label}: no partial cover {seen:?}"
        );
    }
}

/// `0x005d7450` with the original matrix callees and CRT intrinsics on
/// both sides: random controllers and turns, turns that wrap the angles
/// below 0 and past 2pi (several steps), a zero turn, a NaN turn. The
/// frame's axes are compared after the call (`ebp - 0x50` holds a copy of
/// a word the original never writes: not compared). Only synthetic data
/// reaches it.
#[test]
fn m5p_scene_bp2_turn() {
    use ffx_prender::m5p_scene_bp2::turn::turn;
    for (label, mut tw) in Twins::all() {
        let mut rng = Rng(0x005d_7450);
        let end = tw.a.stack_range().end;
        let ebp = end - 0x100 - 4 * 2 - 8;
        for k in 0..60u32 {
            let this = tw.zeros(0x80);
            let m: Vec<f32> = (0..12).map(|_| centi(&mut rng) / 4.0).collect();
            let bytes: Vec<u8> = m.iter().flat_map(|f| f.to_le_bytes()).collect();
            tw.write(this + 4, &bytes);
            let (yaw, pitch) = match k {
                0 => (0.0f32, 0.0f32),
                1 => (20.0, -20.0),
                2 => (-20.0, 20.0),
                3 => (f32::NAN, 0.5),
                4 => (0.25, f32::NAN),
                5 => (6.0, -6.0),
                _ => (centi(&mut rng), centi(&mut rng) / 4.0),
            };
            let junk: Vec<u8> = (0..0x9cu32).map(|i| (i * 11 + k) as u8).collect();
            tw.write(ebp - 0x9c, &junk);
            tw.b.set_call_top(Some(ebp - 0x9c - 0x100));
            let what = format!("{label}: turn {k}");
            let args = [yaw.to_bits(), pitch.to_bits()];
            tw.run(&what, 0x005d_7450, Conv::Thiscall(this), &args, |g| {
                turn(g, ebp, this, yaw.to_bits(), pitch.to_bits())
            });
            tw.b.set_call_top(None);
            for (from, len) in [
                (0x9cu32, 0x40usize),
                (0x5c, 0xc),
                (0x4c, 0xc),
                (0x3c, 0xc),
                (0x28, 0xc),
                (0x18, 0xc),
            ] {
                let mut x = vec![0; len];
                let mut y = vec![0; len];
                tw.a.read_bytes(ebp - from, &mut x);
                tw.b.read_bytes(ebp - from, &mut y);
                assert_eq!(x, y, "{what}: frame at -{from:#x}");
            }
        }
    }
}
