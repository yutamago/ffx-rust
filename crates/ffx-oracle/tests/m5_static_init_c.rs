//! M5 stage 1, list `static-init-c` (`ffx_pload::m5_static_init_c`): the
//! compiler-generated constructors of globals and the destructors they
//! register with `atexit`, against the original. Every row of each family
//! runs twice from the same memory: as the original (`FFX.exe` at its
//! address) and as the port (the bridge over the row). `eax` and every byte
//! written must agree. On the image alone and on the snapshot
//! (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! `atexit` is a recording stub in both oracles: each call logs its
//! destructor to a log in memory (and returns a marker, so the shims must
//! pass its result on), so the diff checks the registrations too. The
//! constructors, the buffer setters and the enum lookups run as the
//! original's code on both sides; imports are recording stubs.
//!
//! Branch reach: the type-enum rows run on the snapshot's list (or the
//! image's), on an empty list (not found: the node is linked) and on a list
//! whose node holds the name (found: nothing linked); the chain append runs
//! on the six chain forms of static-init-b's test. Every other initialiser
//! is straight-line code.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5_static_init_c as si;

/// CRT `atexit`: replaced by a recording stub in both oracles.
const ATEXIT: u32 = 0x0094_9207;
/// What the recording `atexit` returns (not 0, so a shim that drops it
/// fails).
const ATEXIT_RET: u32 = 0x7e57;
/// `InitializeCriticalSection`, the import `ImportAtExit` calls (a
/// recording stub, stdcall, one word).
const INIT_CS: &str = "InitializeCriticalSection";

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

/// Two oracles started alike, and the log of the recording stubs.
struct Twins {
    a: Oracle,
    b: Oracle,
    /// Started from the snapshot (`FFX_SNAPSHOT`), not the image alone.
    snapshot: bool,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
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
        t.log = t.zeros(0x4000);
        // atexit: logs its destructor (one argument), returns a marker
        t.stub(ATEXIT, false, 1, 0, ATEXIT_RET);
        // InitializeCriticalSection (stdcall, one word): logs it, returns 0
        let cs = t.stub_code(0x0c7e_0001, false, 1, 1, 0);
        t.a.bind_import(INIT_CS, Some(cs)).unwrap();
        t.b.bind_import(INIT_CS, Some(cs)).unwrap();
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

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// A recording stub in fresh code memory: logs `tag` (the original's
    /// address), `ecx` when `this`, and `n` stack words; pops `pops`
    /// words; returns `ret`.
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

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there).
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let at = self.rebase(sym);
        let code = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
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

    /// Runs the original at `sym` (a `cdecl` with no arguments: every
    /// static initialiser and destructor) in `a` and `port` on `b`, from the
    /// same memory. `eax` is compared when `check_eax`; every byte written
    /// must agree, the call log included. With `must_log`, at least one
    /// call must be logged.
    fn run_logged(
        &mut self,
        what: &str,
        sym: u32,
        check_eax: bool,
        must_log: bool,
        port: impl FnOnce(&mut G) -> u32,
    ) {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Cdecl, &[])
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        if check_eax {
            assert_eq!(ra.eax, eb, "{what}: eax");
        }
        self.diff(what);
        if must_log {
            assert!(self.log_entries() >= 1, "{what}: nothing logged");
        }
    }

    /// [`Twins::run_logged`] with `eax` checked and a call logged.
    fn run(&mut self, what: &str, sym: u32, port: impl FnOnce(&mut G) -> u32) {
        self.run_logged(what, sym, true, true, port);
    }

    /// Writes the word at the image global `g` in both oracles.
    fn set_global(&mut self, g: u32, value: u32) {
        let at = self.rebase(g);
        self.w32(at, value);
    }

    /// A chain block: its count (at `+0xc`) and its next link (at `+4`).
    fn set_block(&mut self, block: u32, count: u32, next: u32) {
        self.w32(block + 0xc, count);
        self.w32(block + 4, next);
    }
}

#[test]
fn static_init_c_type_enums() {
    for (what, mut t) in Twins::all() {
        // a node that holds the name, for the lookup's "found" branch
        let node = t.zeros(0x10);
        for r in si::TYPE_ENUMS {
            let label = format!("{what} type enum {:#x}", r.addr);
            // the snapshot's own list (or the image's)
            t.run(&label, r.addr, |g| si::type_enum(g, r));
            // an empty list: the name is not found, the node is linked in
            t.set_global(r.head, 0);
            t.run(&format!("{label} (empty list)"), r.addr, |g| {
                si::type_enum(g, r)
            });
            // a list whose only node holds the name: found, nothing linked
            let name = t.rebase(r.name);
            t.w32(node + 4, name);
            t.w32(node + 8, 0);
            t.set_global(r.head, node);
            t.run(&format!("{label} (found)"), r.addr, |g| si::type_enum(g, r));
        }
    }
}

#[test]
fn static_init_c_named_types_and_ctors() {
    for (what, mut t) in Twins::all() {
        for r in si::NAMED_TYPES {
            t.run(&format!("{what} named type {:#x}", r.addr), r.addr, |g| {
                si::named_type(g, r)
            });
        }
        for r in si::CTOR0_AT_EXIT {
            t.run(&format!("{what} ctor {:#x}", r.addr), r.addr, |g| {
                si::ctor0_at_exit(g, r)
            });
        }
    }
}

#[test]
fn static_init_c_class_decls() {
    for (what, mut t) in Twins::all() {
        for r in si::CLASS_DECLS {
            t.run(&format!("{what} class {:#x}", r.addr), r.addr, |g| {
                si::class_decl(g, r)
            });
        }
        for r in si::CLASS_DECLS_FN {
            // the flags word with every bit the tail changes set, then clear
            for flags in [0xffff_ffff, 0] {
                t.set_global(r.flags, flags);
                t.run(
                    &format!("{what} class fn {:#x} (flags {flags:#x})", r.addr),
                    r.addr,
                    |g| si::class_decl_fn(g, r),
                );
            }
        }
    }
}

#[test]
fn static_init_c_chain_appends() {
    for (what, mut t) in Twins::all() {
        let b0 = t.zeros(0x10);
        let b1 = t.zeros(0x10);
        for r in si::LIST_APPENDS {
            let label = format!("{what} chain {:#x}", r.addr);
            // the snapshot's own chain
            t.run(&label, r.addr, |g| si::list_append(g, r));
            // an empty chain
            t.set_global(CHAIN_HEAD, 0);
            t.run(&format!("{label} (empty)"), r.addr, |g| {
                si::list_append(g, r)
            });
            // one full block: the walk stops at the head
            t.set_block(b0, 0xff, 0);
            t.set_global(CHAIN_HEAD, b0);
            t.run(&format!("{label} (full)"), r.addr, |g| {
                si::list_append(g, r)
            });
            // one empty block: the walk ends at its link
            t.set_block(b0, 0, 0);
            t.set_global(CHAIN_HEAD, b0);
            t.run(&format!("{label} (empty block)"), r.addr, |g| {
                si::list_append(g, r)
            });
            // an empty block, then a full one: the walk stops at the second
            t.set_block(b0, 0, b1);
            t.set_block(b1, 0xff, 0);
            t.set_global(CHAIN_HEAD, b0);
            t.run(&format!("{label} (empty, full)"), r.addr, |g| {
                si::list_append(g, r)
            });
            // two empty blocks: the walk ends at the last link
            t.set_block(b0, 0, b1);
            t.set_block(b1, 0, 0);
            t.set_global(CHAIN_HEAD, b0);
            t.run(&format!("{label} (empty, empty)"), r.addr, |g| {
                si::list_append(g, r)
            });
        }
    }
}

/// The chain the appends walk (`ffx_pload::m5_static_init_b::CHAIN_HEAD`).
const CHAIN_HEAD: u32 = 0x00c9_01a8;

#[test]
fn static_init_c_zero_words_and_imports() {
    for (what, mut t) in Twins::all() {
        for r in si::ZERO_WORDS {
            for (i, &w) in r.words.iter().enumerate() {
                t.set_global(w, 0x5a5a_0000 + i as u32);
            }
            // a void shim: the original leaves eax alone, nothing is logged
            t.run_logged(
                &format!("{what} zero words {:#x}", r.addr),
                r.addr,
                false,
                false,
                |g| {
                    si::zero_words(g, r);
                    0
                },
            );
            for &w in r.words {
                let at = t.rebase(w);
                assert_eq!(Mem::read::<u32>(&t.b, at), 0, "{what}: {w:#x} cleared");
            }
        }
        for r in si::IMPORT_AT_EXIT {
            t.run(&format!("{what} import {:#x}", r.addr), r.addr, |g| {
                si::import_at_exit(g, r)
            });
            // the import and atexit both logged
            assert_eq!(t.log_entries(), 2, "{what} import {:#x}: two calls", r.addr);
        }
        let _ = t.snapshot;
    }
}

// ---- the destructors (batch 2) ----

/// `CloseHandle`, the import of the `ImportWord` destructors (a recording
/// stub, stdcall, one word).
const CLOSE_HANDLE: &str = "CloseHandle";

impl Twins {
    /// Replaces each distinct callee in `syms` with a recording stub that
    /// logs `ecx` (thiscalls) and `n` stack words and returns a marker of its own (the
    /// callees are cdecl, or thiscall without stack words: plain `ret`).
    fn stub_callees(&mut self, this: bool, n: u32, syms: impl IntoIterator<Item = u32>) {
        let mut seen: Vec<u32> = Vec::new();
        for sym in syms {
            if !seen.contains(&sym) {
                seen.push(sym);
                self.stub(sym, this, n, 0, 0x5100_0000 | sym);
            }
        }
    }
}

#[test]
fn static_init_c_dtor_thiscalls() {
    for (what, mut t) in Twins::all() {
        t.stub_callees(true, 0, si::TAIL_THISCALLS.iter().map(|r| r.target));
        t.stub_callees(true, 0, si::FRAME_THISCALLS.iter().map(|r| r.target));
        t.stub_callees(true, 0, si::VT_TAILS.iter().map(|r| r.target));
        for r in si::TAIL_THISCALLS {
            t.run(&format!("{what} tail {:#x}", r.addr), r.addr, |g| {
                si::tail_thiscall(g, r)
            });
        }
        for r in si::FRAME_THISCALLS {
            t.run(&format!("{what} frame {:#x}", r.addr), r.addr, |g| {
                si::tail_thiscall(g, r)
            });
        }
        for r in si::VT_TAILS {
            t.set_global(r.obj, 0x1111_2222);
            t.run(&format!("{what} vt tail {:#x}", r.addr), r.addr, |g| {
                si::vt_tail(g, r)
            });
        }
    }
}

#[test]
fn static_init_c_dtor_calls() {
    for (what, mut t) in Twins::all() {
        t.stub_callees(false, 1, si::CALL_ARGS.iter().map(|r| r.func));
        t.stub_callees(false, 1, si::VT_CALLS.iter().map(|r| r.func));
        t.stub_callees(false, 1, si::FREE_UNLESS_BIT0.iter().map(|r| r.func));
        for r in si::CALL_ARGS {
            t.run(&format!("{what} call {:#x}", r.addr), r.addr, |g| {
                si::call_arg(g, r)
            });
        }
        for r in si::VT_CALLS {
            t.set_global(r.obj, 0x1111_2222);
            t.run(&format!("{what} vt call {:#x}", r.addr), r.addr, |g| {
                si::vt_call(g, r)
            });
        }
        for r in si::FREE_UNLESS_BIT0 {
            // the global as it is, then bit 0 clear (freed) and set (kept)
            let at = t.rebase(r.g);
            let own: u32 = Mem::read(&t.b, at);
            for (case, word) in [("own", own), ("clear", 0x0bad_f00c), ("set", 0x0bad_f00d)] {
                t.w32(at, word);
                let freed = word & 1 == 0;
                t.run_logged(
                    &format!("{what} free {:#x} ({case} {word:#x})", r.addr),
                    r.addr,
                    true,
                    freed,
                    |g| si::free_unless_bit0(g, r),
                );
                if !freed {
                    assert_eq!(t.log_entries(), 0, "{what} free {:#x}: kept", r.addr);
                }
            }
        }
    }
}

#[test]
fn static_init_c_dtor_stores_and_imports() {
    for (what, mut t) in Twins::all() {
        for r in si::STORE_WORDS {
            t.set_global(r.g, 0x1111_2222);
            // a void shim: the original leaves eax alone, nothing is logged
            t.run_logged(
                &format!("{what} store {:#x}", r.addr),
                r.addr,
                false,
                false,
                |g| {
                    si::store_word(g, r);
                    0
                },
            );
            let at = t.rebase(r.g);
            assert_eq!(
                Mem::read::<u32>(&t.b, at),
                t.rebase(r.value),
                "{what} store {:#x}",
                r.addr
            );
        }
        // CloseHandle: logs its handle, pops it, returns a marker
        let close = t.stub_code(0x0c7e_0002, false, 1, 1, 0x0c10_5e00);
        t.a.bind_import(CLOSE_HANDLE, Some(close)).unwrap();
        t.b.bind_import(CLOSE_HANDLE, Some(close)).unwrap();
        for r in si::IMPORT_WORDS {
            t.set_global(r.g, 0x0000_0abc ^ r.addr);
            t.run(&format!("{what} import word {:#x}", r.addr), r.addr, |g| {
                si::import_word(g, r)
            });
        }
    }
}

// ---- the destructors (batch 3) ----

/// `SendMessageW`, the import `FFXApplication_atexit_dtor` calls (a
/// recording stub, stdcall, four words).
const SEND_MESSAGE_W: &str = "SendMessageW";

#[test]
fn static_init_c_dtor_chain_unlinks() {
    for (what, mut t) in Twins::all() {
        let b0 = t.zeros(0x10);
        let b1 = t.zeros(0x10);
        for r in si::CHAIN_UNLINKS {
            let label = format!("{what} unlink {:#x}", r.addr);
            let obj = t.rebase(r.obj);
            // the snapshot's own chain (or the image's)
            t.run_logged(&label, r.addr, true, false, |g| si::chain_unlink(g, r));
            // an empty chain
            t.set_global(CHAIN_HEAD, 0);
            t.run_logged(&format!("{label} (empty)"), r.addr, true, false, |g| {
                si::chain_unlink(g, r)
            });
            // the object at the head: the head takes its next link
            t.set_global(CHAIN_HEAD, obj);
            t.w32(obj + 4, b1);
            t.w32(b1 + 4, 0);
            t.run_logged(&format!("{label} (head)"), r.addr, true, false, |g| {
                si::chain_unlink(g, r)
            });
            // the object second: the first block's link takes its next
            t.set_global(CHAIN_HEAD, b0);
            t.w32(b0 + 4, obj);
            t.w32(obj + 4, 0x0bad_0004);
            t.run_logged(&format!("{label} (second)"), r.addr, true, false, |g| {
                si::chain_unlink(g, r)
            });
            // the object not in the chain: the walk ends at the last link
            t.set_global(CHAIN_HEAD, b0);
            t.w32(b0 + 4, b1);
            t.w32(b1 + 4, 0);
            t.run_logged(&format!("{label} (absent)"), r.addr, true, false, |g| {
                si::chain_unlink(g, r)
            });
        }
    }
}

#[test]
fn static_init_c_dtor_detaches_and_counts() {
    for (what, mut t) in Twins::all() {
        t.stub_callees(false, 1, si::COND_DETACHES.iter().map(|r| r.func));
        t.stub_callees(false, 1, si::SUB_COUNTS.iter().map(|r| r.func));
        for r in si::COND_DETACHES {
            // the key clear: only the vtable; the original returns with its
            // caller's eax (not compared)
            t.set_global(r.key, 0);
            t.run_logged(
                &format!("{what} detach {:#x} (clear)", r.addr),
                r.addr,
                false,
                false,
                |g| si::cond_detach(g, r),
            );
            for index in [0, 3, 0x1f] {
                t.set_global(r.key, 1);
                t.set_global(r.index, index);
                t.run(
                    &format!("{what} detach {:#x} (index {index})", r.addr),
                    r.addr,
                    |g| si::cond_detach(g, r),
                );
            }
        }
        for r in si::SUB_COUNTS {
            for (count, total) in [(5, 7), (9, 2), (0, 0)] {
                t.set_global(r.count, count);
                t.set_global(r.total, total);
                t.run(
                    &format!("{what} count {:#x} ({count}, {total})", r.addr),
                    r.addr,
                    |g| si::sub_count(g, r),
                );
            }
        }
    }
}

#[test]
fn static_init_c_dtor_frees() {
    for (what, mut t) in Twins::all() {
        t.stub_callees(false, 1, si::FREE_IF_OWNED.iter().map(|r| r.func));
        t.stub_callees(false, 1, si::FREE_IF_NONZERO.iter().map(|r| r.func));
        for r in si::FREE_IF_OWNED {
            for (flags, block) in [
                (0x8000_0010, 0x0bad_b10c),
                (0x10, 0),
                (0x10, 0x0bad_b10c),
                (0x7fff_ffff, 0x0bad_b10c),
            ] {
                t.set_global(r.flags, flags);
                t.set_global(r.block, block);
                t.run_logged(
                    &format!("{what} owned {:#x} ({flags:#x}, {block:#x})", r.addr),
                    r.addr,
                    true,
                    false,
                    |g| si::free_if_owned(g, r),
                );
            }
        }
        for r in si::FREE_IF_NONZERO {
            for w in [0, 0x0bad_b10c] {
                t.set_global(r.g, w);
                t.run_logged(
                    &format!("{what} nonzero {:#x} ({w:#x})", r.addr),
                    r.addr,
                    true,
                    w != 0,
                    |g| si::free_if_nonzero(g, r),
                );
            }
        }
    }
}

#[test]
fn static_init_c_dtor_sequences() {
    for (what, mut t) in Twins::all() {
        t.stub_callees(
            true,
            0,
            si::THISCALL_SEQS
                .iter()
                .flat_map(|r| r.calls.iter().map(|&(f, _)| f)),
        );
        for r in si::THISCALL_SEQS {
            t.run(&format!("{what} seq {:#x}", r.addr), r.addr, |g| {
                si::thiscall_seq(g, r)
            });
            assert_eq!(t.log_entries() as usize, r.calls.len(), "{what} seq calls");
        }
    }
}

#[test]
fn static_init_c_dtor_singles() {
    for (what, mut t) in Twins::all() {
        let send = t.stub_code(0x0c7e_0003, false, 4, 4, 0x5e4d);
        t.a.bind_import(SEND_MESSAGE_W, Some(send)).unwrap();
        t.b.bind_import(SEND_MESSAGE_W, Some(send)).unwrap();
        // the thiscalls without stack words and the cdecl callees
        t.stub_callees(true, 0, [0x0062_0bc0, 0x0045_c1d0, 0x004f_d160]);
        t.stub_callees(false, 0, [0x0068_9830, 0x0068_bba0, 0x00a2_59f0]);
        t.stub_callees(false, 2, [0x004a_b4d0]);
        t.stub_callees(false, 1, [si::PHYRE_FREE, si::USER_FREE]);
        t.stub(si::EH_VECTOR_DTOR, false, 4, 4, 0xe4ec);
        // FFXApplication_atexit_dtor: the window flag 1 (WM_CLOSE) and not
        for flag in [1, 0, 2] {
            t.set_global(0x00c8_fae4, flag);
            t.set_global(0x00cc_9ce8, 0x0000_1234);
            t.run(
                &format!("{what} app dtor (flag {flag})"),
                0x00af_9ba0,
                |g| si::ffx_application_dtor(g),
            );
        }
        // 0x00aff8a0: owned and not
        for w in [0x8000_0003, 0x0000_0003] {
            t.set_global(0x00ca_8510, w);
            t.set_global(0x00ca_8514, 0x0bad_b10c);
            t.run(&format!("{what} 0xaff8a0 ({w:#x})"), 0x00af_f8a0, |g| {
                si::dtor_00aff8a0(g)
            });
        }
        t.run(&format!("{what} 0xaff930"), 0x00af_f930, |g| {
            si::dtor_00aff930(g)
        });
        for (flags, block) in [(0x8000_0000, 0x0bad_b10c), (0, 0), (0, 0x0bad_b10c)] {
            t.set_global(0x00cc_142c, flags);
            t.set_global(0x00cc_1430, block);
            t.run(
                &format!("{what} 0xb050d0 ({flags:#x}, {block:#x})"),
                0x00b0_50d0,
                |g| si::dtor_00b050d0(g),
            );
        }
        for w in [0, 0x0bad_b10c] {
            t.set_global(0x00cc_b214, w);
            t.run(&format!("{what} 0xb066e0 ({w:#x})"), 0x00b0_66e0, |g| {
                si::dtor_00b066e0(g)
            });
        }
    }
}
