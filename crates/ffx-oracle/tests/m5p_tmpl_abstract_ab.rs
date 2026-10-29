//! M5 stage 1, lists `tmpl-abstract-a` and `-b`
//! (`ffx_pload::m5p_tmpl_abstract_ab`): the `PClassDescriptorAbstract<T>`
//! slots 7, 12, 13 and 14, against the original. Every row of every table
//! runs twice from the same memory: as the original (`FFX.exe` at the
//! copy's address, thiscall on a descriptor) and as the generic port with
//! the row's constant. `eax` and every byte written must agree (the oracle
//! also checks `esp`, so each copy pops what the port's shim declares).
//! On the image alone and on the snapshot (`FFX_SNAPSHOT`). Needs
//! `FFX_GAME_DIR`.
//!
//! None of this code ran in a coverage run, so the inputs are synthetic:
//! the callees are recording stubs at their addresses in both oracles
//! (they log their tag, `ecx` and their stack words to memory, so the
//! diff checks every call and its arguments), and each branch gets its own
//! data: the destructor's `flags` bit 0 set and clear, the slot 4 object 0
//! and not 0, the slot 14 loop with no, one and several objects. The
//! `bind` copies also run for real (no stubs) on the snapshot, where the
//! descriptors exist.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pload::m5p_tmpl_abstract_ab as t;

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
        t.log = t.zeros(0x400);
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

    /// The log entries of the last run (the port's side).
    fn logged(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// The logged word `i` (0 tag, 1 `ecx`, 2.. stack words) of entry `n`.
    fn entry(&self, n: u32, i: u32) -> u32 {
        Mem::read(&self.b, self.log + 4 + 32 * n + 4 * i)
    }

    /// A recording stub in fresh code memory: logs `tag`, `ecx` when
    /// `this` (the thiscall callees; a cdecl callee's `ecx` is whatever
    /// its caller left) and `n` stack words; pops `pops` words; returns
    /// `ret`.
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

    /// Runs the original copy at `sym` as a thiscall on `this` with the
    /// stack words `args` in `a`, and `port` on `b`, from the same memory
    /// (the log emptied first); `eax` and every byte written must agree.
    /// Returns `eax`.
    fn run(
        &mut self,
        what: &str,
        sym: u32,
        this: u32,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u32,
    ) -> u32 {
        let log = self.log;
        self.w32(log, 0);
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, Conv::Thiscall(this), args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let eb = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax, eb, "{what}: eax");
        self.diff(what);
        eb
    }
}

/// Odd words for the arguments the copies never read.
const JUNK: [u32; 3] = [0x1357_9bdf, 0xfedc_ba98, 0x0000_0001];

#[test]
fn m5p_tmpl_ab_dtors() {
    for (what, mut tw) in Twins::all() {
        // PClassDescriptor::Destruct: thiscall, nothing on the stack
        tw.stub(t::DESTRUCT, true, 0, 0, 0x0bad_0001);
        // PhyreFree(block): cdecl
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0002);
        let obj = tw.zeros(0xa0);
        for &(f, vt) in t::DTORS {
            for flags in [0, 1, 2, 3, 0x100, 0xffff_ff00, 0xffff_ffff] {
                tw.write(obj, &[0x55; 0xa0]);
                let label = format!("{what} dtor {f:#x} flags {flags:#x}");
                let r = tw.run(&label, f, obj, &[flags], |g| {
                    t::bridge::scalar_deleting_dtor(g, obj, flags, vt)
                });
                assert_eq!(r, obj, "{label}: returns this");
                // Destruct on the descriptor, then PhyreFree(descriptor) for bit 0
                let n = tw.logged();
                assert_eq!(n, 1 + (flags & 1), "{label}: calls");
                assert_eq!(tw.entry(0, 0), t::DESTRUCT, "{label}");
                assert_eq!(tw.entry(0, 1), obj, "{label}");
                if flags & 1 != 0 {
                    assert_eq!(tw.entry(1, 0), t::FREE, "{label}");
                    assert_eq!(tw.entry(1, 2), obj, "{label}");
                }
                let stored: u32 = Mem::read(&tw.b, obj);
                assert_eq!(stored, tw.rebase(vt), "{label}: vtable");
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_ab_instance_members() {
    for (what, mut tw) in Twins::all() {
        for &f in t::MEMBER_NONE {
            for args in [[0, 0, 0], JUNK, [u32::MAX; 3]] {
                let label = format!("{what} getInstanceMember {f:#x} {args:x?}");
                let r = tw.run(&label, f, 0x0012_3450, &args, |_| {
                    t::bridge::no_instance_member()
                });
                assert_eq!(r, u32::MAX, "{label}");
                assert_eq!(tw.logged(), 0, "{label}: calls");
            }
        }
        for &(f, callee) in t::MEMBER_FORWARD {
            tw.stub(callee, false, 3, 0, callee ^ 0x5a5a_0000);
            for args in [[0, 0, 0], JUNK, [u32::MAX; 3]] {
                let label = format!("{what} getInstanceMember {f:#x} {args:x?}");
                tw.run(&label, f, 0x0012_3450, &args, |g| {
                    t::bridge::forward3(g, callee, args[0], args[1], args[2])
                });
                assert_eq!(tw.logged(), 1, "{label}: calls");
                assert_eq!(tw.entry(0, 0), callee, "{label}");
                for (i, &w) in args.iter().enumerate() {
                    assert_eq!(tw.entry(0, 2 + i as u32), w, "{label}: word {i}");
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_ab_fix_objects() {
    for (what, mut tw) in Twins::all() {
        for &f in t::FIX_NONE {
            for list in [0, JUNK[0], u32::MAX] {
                let label = format!("{what} fixObjectsAfterLoading {f:#x} {list:#x}");
                let r = tw.run(&label, f, 0x0012_3450, &[list], |_| {
                    t::bridge::no_objects_to_fix()
                });
                assert_eq!(r, 0, "{label}");
                assert_eq!(tw.logged(), 0, "{label}: calls");
            }
        }
        for &(f, callee) in t::FIX_FORWARD {
            tw.stub(callee, false, 1, 0, callee ^ 0x5a5a_0000);
            for list in [0, JUNK[0], u32::MAX] {
                let label = format!("{what} fixObjectsAfterLoading {f:#x} {list:#x}");
                tw.run(&label, f, 0x0012_3450, &[list], |g| {
                    t::bridge::forward1(g, callee, list)
                });
                assert_eq!(tw.logged(), 1, "{label}: calls");
                assert_eq!(tw.entry(0, 0), callee, "{label}");
                assert_eq!(tw.entry(0, 2), list, "{label}");
            }
        }
        tw.unstub();
    }
}

/// `0x0044e070`: the instance-list loop, with no, one and several objects
/// and strides 0, 0x10 and 0x24.
#[test]
fn m5p_tmpl_ab_fix_set_descriptor() {
    const F: u32 = 0x0044_e070;
    for (what, mut tw) in Twins::all() {
        let list = tw.zeros(0x40);
        let descriptor = tw.zeros(0x40);
        let objects = tw.zeros(0x400);
        for (count, stride) in [(0, 0x10), (1, 0x10), (3, 0x24), (5, 0), (7, 0x10)] {
            tw.write(objects, &[0x77; 0x400]);
            tw.w32(list + 0x2c, descriptor);
            tw.w32(list + 0x30, objects);
            tw.w32(list + 0x3c, count);
            tw.w32(descriptor + 0x1c, stride);
            let label = format!("{what} fix_set_descriptor count {count} stride {stride:#x}");
            let r = tw.run(&label, F, 0x0012_3450, &[list], |g| {
                t::bridge::fix_set_descriptor(g, list)
            });
            assert_eq!(r, 0, "{label}");
            if count > 0 {
                let first: u32 = Mem::read(&tw.b, objects + 8);
                assert_eq!(first, descriptor, "{label}: written");
            }
        }
    }
}

#[test]
fn m5p_tmpl_ab_binds() {
    for (what, mut tw) in Twins::all() {
        // the classes' Bind() (cdecl, no arguments)
        for &(_, target) in t::BIND_JUMP {
            if !tw.saved.contains_key(&tw.rebase(target)) {
                tw.stub(target, false, 0, 0, target ^ 0x5a5a_0000);
            }
        }
        tw.stub(t::REGISTER_TYPE, false, 1, 0, 0x0bad_0003);
        tw.stub(t::ADD_TO_NAMESPACE, true, 0, 0, 0x0bad_0004);
        for this in [0, 0x0012_3450] {
            for &(f, target) in t::BIND_JUMP {
                let label = format!("{what} bind {f:#x} this {this:#x}");
                let r = tw.run(&label, f, this, &[], |g| t::bridge::bind_jump(g, target));
                assert_eq!(r, target ^ 0x5a5a_0000, "{label}");
                assert_eq!(tw.logged(), 1, "{label}: calls");
                assert_eq!(tw.entry(0, 0), target, "{label}");
            }
            for &(f, d) in t::BIND_INLINE {
                let label = format!("{what} bind {f:#x} this {this:#x}");
                let r = tw.run(&label, f, this, &[], |g| t::bridge::bind_inline(g, d));
                assert_eq!(r, 0x0bad_0004, "{label}");
                let d = tw.rebase(d);
                assert_eq!(tw.logged(), 2, "{label}: calls");
                assert_eq!(
                    (tw.entry(0, 0), tw.entry(0, 2)),
                    (t::REGISTER_TYPE, d),
                    "{label}"
                );
                assert_eq!(
                    (tw.entry(1, 0), tw.entry(1, 1)),
                    (t::ADD_TO_NAMESPACE, d),
                    "{label}"
                );
            }
        }
        tw.unstub();
        // the real Bind() code where the descriptors exist
        if tw.snapshot {
            for &(f, target) in t::BIND_JUMP {
                tw.run(
                    &format!("{what} bind {f:#x} (real)"),
                    f,
                    0x0012_3450,
                    &[],
                    |g| t::bridge::bind_jump(g, target),
                );
            }
            for &(f, d) in t::BIND_INLINE {
                let this = tw.rebase(d);
                tw.run(&format!("{what} bind {f:#x} (real)"), f, this, &[], |g| {
                    t::bridge::bind_inline(g, d)
                });
            }
        }
    }
}

/// `PClassDescriptorAbstract<PClassDescriptor>`'s script slots 3 and 4.
#[test]
fn m5p_tmpl_ab_script_slots() {
    const GET: u32 = 0x0043_cdb0;
    const PUT: u32 = 0x0043_d330;
    for (what, mut tw) in Twins::all() {
        tw.stub(t::PRINTF, false, 2, 0, 0x0bad_0005);
        tw.stub(t::SCRIPT_PUT, false, 3, 0, 0x0bad_0006);
        for args in [
            [0, 0],
            [JUNK[0], JUNK[1]],
            [1, 0xffff_ffe8],
            [7, 0xffff_ffe7],
        ] {
            let label = format!("{what} script get {args:x?}");
            let r = tw.run(&label, GET, 0x0012_3450, &args, |g| {
                t::bridge::script_get_unsupported(g)
            });
            assert_eq!(r, 0x0bad_0005, "{label}");
            assert_eq!(tw.logged(), 1, "{label}: calls");
            assert_eq!(
                (tw.entry(0, 2), tw.entry(0, 3)),
                (2, tw.rebase(t::SCRIPT_GET_MESSAGE)),
                "{label}"
            );
            let label = format!("{what} script put {args:x?}");
            let r = tw.run(&label, PUT, 0x0012_3450, &args, |g| {
                t::bridge::script_put_descriptor(g, args[0], args[1])
            });
            assert_eq!(r, 0x0bad_0006, "{label}");
            assert_eq!(tw.logged(), 1, "{label}: calls");
            let base = if args[1] == 0 {
                0
            } else {
                args[1].wrapping_add(0x18)
            };
            assert_eq!(
                (tw.entry(0, 2), tw.entry(0, 3), tw.entry(0, 4)),
                (args[0], base, tw.rebase(t::CLASS_DESCRIPTOR_DESCRIPTOR)),
                "{label}"
            );
        }
        tw.unstub();
    }
}

/// Every address of the two lists is in exactly one table (or one of the
/// three single ports), so no copy goes untested.
#[test]
fn m5p_tmpl_ab_tables_cover_the_shims() {
    let mut all: Vec<u32> = Vec::new();
    all.extend(t::DTORS.iter().map(|r| r.0));
    all.extend(t::MEMBER_NONE);
    all.extend(t::MEMBER_FORWARD.iter().map(|r| r.0));
    all.extend(t::FIX_NONE);
    all.extend(t::FIX_FORWARD.iter().map(|r| r.0));
    all.extend(t::BIND_JUMP.iter().map(|r| r.0));
    all.extend(t::BIND_INLINE.iter().map(|r| r.0));
    all.extend([0x0043_cdb0, 0x0043_d330, 0x0044_e070]);
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n, "an address in two tables");
    assert_eq!(n, 698);
}
