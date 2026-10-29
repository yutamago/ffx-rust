//! M5 stage 1, list `static-init-d` (`ffx_pload::m5_static_init_d`): the
//! destructors the static initialisers register with `atexit`, against the
//! original. Every row of each family runs twice from the same memory: as
//! the original (`FFX.exe` at its address) and as the port (the bridge over
//! the row). Results and every byte written must agree (the stack aside:
//! `Oracle::diff` skips it). On the image alone and on the snapshot
//! (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! `atexit` is a recording stub in both oracles, as in the static-init-b
//! tests, so a destructor that registered anything would show in the diff.
//! The callees run as the original's code on both sides, except where a
//! test says it puts a recording stub in their place (`PhyreFree`,
//! `operator delete`, the `g_MemSystem` destructor, the imports). Branches
//! the image and the snapshots don't reach get synthetic lists and objects,
//! written identically to both oracles; each test names them.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5_static_init_d as si;

/// CRT `atexit`: replaced by a recording stub in both oracles.
const ATEXIT: u32 = 0x0094_9207;

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
        };
        t.log = t.zeros(0x4000);
        // atexit: returns 0 and logs its destructor (one argument)
        t.stub(ATEXIT, false, 1, 0, 0);
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

    /// Writes `v` at the image global `sym` in both oracles.
    fn set_global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// A word of `a` (both oracles agree after every run).
    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
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
    /// destructor `atexit` calls) in `a` and `port` on `b`, from the same
    /// memory, with an empty call log. `eax` is compared when `check_eax`;
    /// every byte written must agree, the log included. Returns the
    /// number of logged calls.
    fn run(
        &mut self,
        what: &str,
        sym: u32,
        check_eax: bool,
        port: impl FnOnce(&mut G) -> u32,
    ) -> u32 {
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
        self.log_entries()
    }
}

/// The list head an enum-value unlink walks: its first instruction after
/// the frame is `mov ecx, [head]` (`8b 0d`, at `+3`).
fn unlink_head(t: &mut Twins, unlink: u32) -> u32 {
    let at = t.rebase(unlink) + 3;
    assert_eq!(Oracle::read(&mut t.a, at, 2).unwrap(), [0x8b, 0x0d]);
    t.r32(at + 2)
}

#[test]
fn static_init_d_enum_value_dtors() {
    for (what, mut t) in Twins::all() {
        // two other values (their link at +8)
        let other = t.zeros(0x10);
        let other2 = t.zeros(0x10);
        for r in si::ENUM_VALUE_DTORS {
            let label = format!("{what} enum value {:#x}", r.addr);
            let head = unlink_head(&mut t, r.unlink);
            let obj = t.rebase(r.obj);
            // the list as it is (empty on the image; on the snapshot the
            // value is in it)
            t.run(&label, r.addr, true, |g| si::enum_value_dtor(g, r));
            // the value alone in its list: unlinked from the head (its
            // first word junk: the image's .data already holds the
            // vtable, so the store would not show otherwise)
            t.w32(obj, 0x5a5a_5a5a);
            t.w32(obj + 8, 0);
            t.w32(head, obj);
            t.run(&format!("{label} (alone)"), r.addr, true, |g| {
                si::enum_value_dtor(g, r)
            });
            // behind another value, with one after it
            t.w32(other + 8, obj);
            t.w32(obj + 8, other2);
            t.w32(other2 + 8, 0);
            t.w32(head, other);
            t.run(&format!("{label} (second)"), r.addr, true, |g| {
                si::enum_value_dtor(g, r)
            });
            // not in the list
            t.w32(other + 8, 0);
            t.w32(head, other);
            t.run(&format!("{label} (absent)"), r.addr, true, |g| {
                si::enum_value_dtor(g, r)
            });
            t.w32(head, 0);
        }
    }
}

/// Tail-jump targets that return with their caller's `eax` on the paths
/// these runs take (`0x009d_a05c` when its handle flag is clear,
/// `0x0094_a641` when its list is empty): the port's `eax` is the
/// emulator's leftover there, which nothing reads.
const TAIL_KEEPS_EAX: &[u32] = &[0x009d_a05c, 0x0094_a641];

/// Tail-jump targets that fault on the image, where their objects were
/// never constructed (a null pointer in the zeroed global): those rows run
/// on the snapshots only, where the objects are live.
const TAIL_IMAGE_SKIP: &[u32] = &[0x0043_fa30];

/// Tail-jump targets that call into a system DLL on the snapshots
/// (`PSamplerStateD3D11`'s destructor releases its live D3D11 sampler
/// through `d3d11.dll`, which the snapshots don't hold): those rows run on
/// the image only, where the sampler pointer is null; `0x0094_a641`
/// deletes its list's nodes through the CRT's heap, outside the snapshots,
/// and runs on the image with its list empty).
const TAIL_SNAPSHOT_SKIP: &[u32] = &[0x0058_5230, 0x0094_a641];

#[test]
fn static_init_d_tail_thiscalls() {
    for (what, mut t) in Twins::all() {
        for r in si::TAIL_THISCALLS {
            if !t.snapshot && TAIL_IMAGE_SKIP.contains(&r.target) {
                continue;
            }
            if t.snapshot && TAIL_SNAPSHOT_SKIP.contains(&r.target) {
                continue;
            }
            let check = !TAIL_KEEPS_EAX.contains(&r.target);
            t.run(&format!("{what} tail {:#x}", r.addr), r.addr, check, |g| {
                si::tail_thiscall(g, r)
            });
        }
    }
}

#[test]
fn static_init_d_chain_unlinks() {
    for (what, mut t) in Twins::all() {
        // two other chain blocks (their link at +4)
        let b0 = t.zeros(0x10);
        let b1 = t.zeros(0x10);
        for r in si::CHAIN_UNLINKS {
            let label = format!("{what} chain unlink {:#x}", r.addr);
            let obj = t.rebase(r.obj);
            // the chain as it is
            t.run(&label, r.addr, true, |g| si::chain_unlink(g, r));
            // an empty chain: nothing but the vtable
            t.set_global(si::CHAIN_HEAD, 0);
            t.w32(obj + 4, b1);
            t.run(&format!("{label} (empty)"), r.addr, true, |g| {
                si::chain_unlink(g, r)
            });
            // the object first: the head takes its link
            t.set_global(si::CHAIN_HEAD, obj);
            t.w32(obj + 4, b1);
            t.w32(b1 + 4, 0);
            t.run(&format!("{label} (first)"), r.addr, true, |g| {
                si::chain_unlink(g, r)
            });
            // behind a block: the block's link takes it
            t.set_global(si::CHAIN_HEAD, b0);
            t.w32(b0 + 4, obj);
            t.w32(obj + 4, b1);
            t.run(&format!("{label} (second)"), r.addr, true, |g| {
                si::chain_unlink(g, r)
            });
            // behind a block whose link is the object, which links nothing
            t.set_global(si::CHAIN_HEAD, b0);
            t.w32(b0 + 4, obj);
            t.w32(obj + 4, 0);
            t.run(&format!("{label} (second, last)"), r.addr, true, |g| {
                si::chain_unlink(g, r)
            });
            // not in the chain (two blocks)
            t.set_global(si::CHAIN_HEAD, b0);
            t.w32(b0 + 4, b1);
            t.w32(b1 + 4, 0);
            t.w32(obj + 4, 0x1234_5678);
            t.run(&format!("{label} (absent)"), r.addr, true, |g| {
                si::chain_unlink(g, r)
            });
            t.set_global(si::CHAIN_HEAD, 0);
        }
    }
}

#[test]
fn static_init_d_pool_trees() {
    for (what, mut t) in Twins::all() {
        for r in si::POOL_TREE_DTORS {
            // eax is not compared: on an empty tree `clear` leaves eax
            // alone, so the original returns what its callees passed
            // through from its own SEH setup (`lea eax, [ebp - 0xc]`, a
            // frame address); the CRT's exit loop reads none of it
            t.run(
                &format!("{what} pool tree {:#x}", r.addr),
                r.addr,
                false,
                |g| si::pool_tree_dtor(g, r),
            );
        }
    }
}

/// The pooled trees' destructors with their four callees as recording
/// thiscall stubs: on the empty trees above, `orphanAllContents` and the
/// free list's destructor change nothing, so only the call log (each
/// callee's tag and `this`, in order) shows that the port makes them.
/// Their `eax` is the last stub's on both sides.
#[test]
fn static_init_d_pool_tree_calls() {
    for (what, mut t) in Twins::all() {
        for (k, sym) in [
            si::RBT_ORPHAN_ALL,
            si::FREE_LIST_RESET,
            si::FREE_LIST_DTOR,
            si::RBT_CLEAR,
        ]
        .into_iter()
        .enumerate()
        {
            t.stub(sym, true, 0, 0, 0x7700 + k as u32);
        }
        for r in si::POOL_TREE_DTORS {
            let label = format!("{what} pool tree calls {:#x}", r.addr);
            let n = t.run(&label, r.addr, true, |g| si::pool_tree_dtor(g, r));
            assert_eq!(n, 5, "{label}: calls");
        }
    }
}

#[test]
fn static_init_d_maps() {
    for (what, mut t) in Twins::all() {
        // operator delete: a stub in both (the image's heap holds no node)
        t.stub(si::OPERATOR_DELETE, false, 1, 0, 0x0de1_e7e0);
        for r in si::MAP_DTORS {
            let label = format!("{what} map {:#x}", r.addr);
            let map = t.rebase(r.map);
            if t.snapshot && t.r32(map) != 0 {
                // the map as the snapshot holds it
                let out = t.b.stack_range().start + 0x100;
                let n = t.run(&label, r.addr, true, |g| si::map_dtor(g, r, out));
                assert!(n >= 1, "{label}: the head was not deleted");
            }
            // an empty map: a head node linked to itself, its colour and
            // nil bytes set
            let mut node = vec![0u8; 0x40];
            node[0xc..].fill(1);
            let head = t.alloc(&node);
            for off in [0, 4, 8] {
                t.w32(head + off, head);
            }
            t.w32(map, head);
            let out = t.b.stack_range().start + 0x100;
            let n = t.run(&format!("{label} (empty)"), r.addr, true, |g| {
                si::map_dtor(g, r, out)
            });
            assert!(n >= 1, "{label} (empty): the head was not deleted");
        }
    }
}

#[test]
fn static_init_d_owned_buffers() {
    for (what, mut t) in Twins::all() {
        // PhyreFree: a stub in both, so the data needn't be a heap block
        t.stub(si::PHYRE_FREE, false, 1, 0, 0x0f2e_e000);
        let data = t.zeros(0x10);
        for r in si::OWNED_BUFFERS {
            let label = format!("{what} buffer {:#x}", r.addr);
            let obj = t.rebase(r.obj);
            t.run(&label, r.addr, true, |g| si::owned_buffer(g, r));
            for (flags, d, frees) in [
                (0u32, data, 1),
                (0x7fff_ffff, data, 1),
                (0x8000_0000, data, 0),
                (0xffff_ffff, data, 0),
                (0, 0, 0),
            ] {
                t.w32(obj, flags);
                t.w32(obj + 4, d);
                let n = t.run(
                    &format!("{label} (flags {flags:#x}, data {d:#x})"),
                    r.addr,
                    true,
                    |g| si::owned_buffer(g, r),
                );
                assert_eq!(n, frees, "{label}: PhyreFree calls");
            }
        }
    }
}

#[test]
fn static_init_d_import_calls() {
    for (what, mut t) in Twins::all() {
        // a stdcall stub that logs its word and pops it, returning a marker
        let s1 = t.stub_code(0x0c7e_0021, false, 1, 1, 0x1111_0001);
        for r in si::IMPORT_CALLS {
            let label = format!("{what} import {:#x}", r.addr);
            t.set_global(r.iat, s1);
            if let si::ImportArg::Word(g) = r.arg {
                t.set_global(g, 0x0000_0a5c);
            }
            let n = t.run(&label, r.addr, true, |g| si::import_call(g, r));
            assert_eq!(n, 1, "{label}: the import's calls");
        }
    }
}

#[test]
fn static_init_d_cond_tail_loads() {
    for (what, mut t) in Twins::all() {
        let obj = t.zeros(0x40);
        for r in si::COND_TAIL_LOADS {
            let label = format!("{what} cond tail {:#x}", r.addr);
            // the target: a recording thiscall stub (the real one tears
            // down the memory system)
            t.stub(r.target, true, 0, 0, 0x0000_0777);
            // a pointer: the jump is taken, its eax is ours
            t.set_global(r.ptr, obj);
            let n = t.run(&format!("{label} (taken)"), r.addr, true, |g| {
                si::cond_tail_load(g, r)
            });
            assert_eq!(n, 1, "{label}: the target's calls");
            // none: the original returns with the caller's eax, which the
            // port does not reproduce (nothing reads it)
            t.set_global(r.ptr, 0);
            let n = t.run(&format!("{label} (not taken)"), r.addr, false, |g| {
                si::cond_tail_load(g, r)
            });
            assert_eq!(n, 0, "{label}: the target's calls");
        }
    }
}

#[test]
fn static_init_d_store_vts() {
    for (what, mut t) in Twins::all() {
        for r in si::STORE_VTS {
            // junk in the object first (the image's .data may hold the
            // vtable already)
            t.set_global(r.obj, 0x5a5a_5a5a);
            // a void shim: eax is the caller's, not compared
            t.run(
                &format!("{what} store vt {:#x}", r.addr),
                r.addr,
                false,
                |g| {
                    si::store_vt(g, r);
                    0
                },
            );
        }
    }
}
