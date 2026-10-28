//! M5 stage 1, list `static-init-b` (`ffx_pload::m5_static_init_b`): the
//! compiler-generated constructors of globals and the destructors they
//! register with `atexit`, against the original. Every row of each family
//! runs twice from the same memory: as the original (`FFX.exe` at its
//! address) and as the port (the bridge over the row). Results and every
//! byte written must agree. On the image alone and on the snapshot
//! (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! `atexit` is a recording stub in both oracles: each call logs its
//! destructor to a log in memory, so the diff checks the registrations
//! too, and each row must register exactly one. The constructors, the
//! buffer setters and the enum lookups run as the original's code on both
//! sides. Branches the snapshot's lists don't reach get synthetic list
//! heads, written identically to both oracles.
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
use ffx_pload::m5_static_init_b as si;

/// CRT `atexit`: replaced by a recording stub in both oracles.
const ATEXIT: u32 = 0x0094_9207;
/// The import a constructor here starts a thread with (`0x00630f40`'s
/// callees); its stub is installed in both oracles. The oracle keys its
/// import slots by the bare name.
const CREATE_THREAD: &str = "CreateThread";
/// `GetSystemMetrics`, the other import the constructors reach (see above).
const GET_SYSTEM_METRICS: &str = "GetSystemMetrics";
/// The page (of a system DLL's data, outside the image) that the
/// mouse-state constructor reads on the snapshot.
const SYSTEM_DATA_PAGE: u32 = 0x7568_e000;

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
        // atexit: returns 0 and logs its destructor (one argument)
        t.stub(ATEXIT, false, 1, 0, 0);
        // CreateThread (stdcall, six words): logs them, returns a fake
        // handle, and the thread never runs in either oracle
        let create_thread = t.stub_code(0x0c7e_0001, false, 6, 6, 0x0000_0abc);
        t.a.bind_import(CREATE_THREAD, Some(create_thread)).unwrap();
        t.b.bind_import(CREATE_THREAD, Some(create_thread)).unwrap();
        // GetSystemMetrics (stdcall, one word; the mouse's swap-button
        // query): logs its index, returns 0
        let metrics = t.stub_code(0x0c7e_0002, false, 1, 1, 0);
        t.a.bind_import(GET_SYSTEM_METRICS, Some(metrics)).unwrap();
        t.b.bind_import(GET_SYSTEM_METRICS, Some(metrics)).unwrap();
        // a page of a system DLL's data that the snapshot does not hold (the
        // constructors read it): zeroed in both, so the parity holds
        t.a.map_zeroed(SYSTEM_DATA_PAGE, 0x1000).unwrap();
        t.b.map_zeroed(SYSTEM_DATA_PAGE, 0x1000).unwrap();
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
    /// static initialiser) in `a` and `port` on `b`, from the same memory.
    /// `eax` is compared when `check_eax`; every byte written must agree,
    /// the destructor log included. The first call of `GetGlobalNamespace`
    /// in a run registers its own destructor (its guard bit), so the log
    /// is not one entry per row: it must be the same on both sides, and
    /// at least one entry.
    fn run(&mut self, what: &str, sym: u32, check_eax: bool, port: impl FnOnce(&mut G) -> u32) {
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
        assert!(self.log_entries() >= 1, "{what}: no destructor registered");
    }

    /// Writes the list head at `head` (an image global) in both oracles.
    fn set_head(&mut self, head: u32, value: u32) {
        let at = self.rebase(head);
        self.w32(at, value);
    }
}

#[test]
fn static_init_b_type_enums() {
    for (what, mut t) in Twins::all() {
        // a node that holds the name, for the lookup's "found" branch
        let node = t.zeros(0x10);
        for r in si::TYPE_ENUMS {
            let label = format!("{what} type enum {:#x}", r.addr);
            // the snapshot's own list
            t.run(&label, r.addr, true, |g| si::type_enum(g, r));
            // an empty list: the name is not found, the node is linked in
            t.set_head(r.head, 0);
            t.run(&format!("{label} (empty list)"), r.addr, true, |g| {
                si::type_enum(g, r)
            });
            // a list whose only node holds the name: found, nothing linked
            let name = t.rebase(r.name);
            t.w32(node + 4, name);
            t.w32(node + 8, 0);
            t.set_head(r.head, node);
            t.run(&format!("{label} (found)"), r.addr, true, |g| {
                si::type_enum(g, r)
            });
        }
    }
}

#[test]
fn static_init_b_named_types() {
    for (what, mut t) in Twins::all() {
        for r in si::NAMED_TYPES {
            t.run(
                &format!("{what} named type {:#x}", r.addr),
                r.addr,
                true,
                |g| si::named_type(g, r),
            );
        }
    }
}

#[test]
fn static_init_b_at_exit_only() {
    for (what, mut t) in Twins::all() {
        for r in si::AT_EXIT_ONLY {
            t.run(&format!("{what} atexit {:#x}", r.addr), r.addr, true, |g| {
                si::at_exit_only(g, r)
            });
        }
    }
}

/// The constructors whose original reads system DLL data the snapshot does
/// not map (an invalid access at `0x7568xxxx` on the snapshot, while the
/// image run passes). The rows run on the image alone: the same original
/// callee runs in every other row's test on the snapshot.
const SNAPSHOT_SKIP_CTORS: &[u32] = &[0x00ae_ea80];

#[test]
fn static_init_b_ctors() {
    for (what, mut t) in Twins::all() {
        for r in si::CTOR0_AT_EXIT {
            if t.snapshot && SNAPSHOT_SKIP_CTORS.contains(&r.addr) {
                continue;
            }
            t.run(&format!("{what} ctor {:#x}", r.addr), r.addr, true, |g| {
                si::ctor0_at_exit(g, r)
            });
        }
        for r in si::CTOR0_FLAGS_VT {
            if t.snapshot && SNAPSHOT_SKIP_CTORS.contains(&r.addr) {
                continue;
            }
            t.run(
                &format!("{what} ctor+vt {:#x}", r.addr),
                r.addr,
                true,
                |g| si::ctor0_flags_vt(g, r),
            );
        }
    }
}

#[test]
fn static_init_b_class_decls() {
    for (what, mut t) in Twins::all() {
        for r in si::CLASS_DECLS {
            t.run(&format!("{what} class {:#x}", r.addr), r.addr, true, |g| {
                si::class_decl(g, r)
            });
        }
    }
}
