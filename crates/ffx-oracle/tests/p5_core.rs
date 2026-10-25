//! `PhyreEngine`'s core (`ffx_pcore`, M4 port round 1, list `p5-core`)
//! against the original: the matrix helpers, the thread primitives, the
//! free lists, the strings and the class descriptors' little members. On
//! the image alone with synthetic data, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same data over the snapshot's memory.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Win32 imports (events, semaphores, waits) and library callees outside
//! the list are replaced in both oracles alike by recording stubs: each
//! call appends its tag, `ecx` and its arguments to a log in memory, so
//! the diff checks that both made the same calls in the same order. The
//! allocators, `memcpy`, `memchr` and the `Interlocked*` functions run as
//! they are.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pcore::{freelist, matrix, small, string, sync};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
    /// Float bits that are not a signalling NaN (the emulator keeps
    /// those).
    fn float(&mut self) -> u32 {
        loop {
            let bits = match self.below(6) {
                0 => self.next(),
                1 | 2 => (self.below(4000) as f32 - 2000.0).to_bits(),
                3 => ((self.next() as f32) / 1e7 - 200.0).to_bits(),
                4 => (self.below(2000) as f32 / 977.0 - 1.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    0xffc0_1234,
                    1,
                    0x7f7f_ffff,
                    0x0080_0001,
                ][self.below(9) as usize],
            };
            if !ffx_bridge::x87::is_snan(bits) {
                return bits;
            }
        }
    }
    /// Floats of very different sizes and signs (sums that cancel).
    fn floats_wide(&mut self, n: usize) -> Vec<u8> {
        const W: [f32; 8] = [1e10, -1e10, 1.0, 3.0, 1e-5, 7e9, -3.3e9, 0.1];
        (0..n)
            .flat_map(|_| W[self.below(8) as usize].to_bits().to_le_bytes())
            .collect()
    }
    fn floats(&mut self, n: usize) -> Vec<u8> {
        (0..n).flat_map(|_| self.float().to_le_bytes()).collect()
    }
}

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
        tmp(self.0)
    }
}

/// Memory only the port uses: the bottom of its oracle's stack, which no
/// call reaches and the diff leaves out.
fn tmp(o: &Oracle) -> u32 {
    o.stack_range().start + 0x100
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

/// What a recording stub returns.
#[derive(Debug, Clone, Copy)]
enum StubRet {
    /// A constant.
    Imm(u32),
    /// A bump allocator's cursor (a cell: cursor, step).
    Bump(u32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
        };
        t.log = t.zeros(0x8000);
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
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
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

    /// Writes a recording stub over the original at `sym` (a `jmp`
    /// there): logs `sym`, `ecx` and `n` stack words; pops `pops` words;
    /// returns `ret`.
    fn stub(&mut self, sym: u32, n: u32, pops: u32, ret: u32) -> u32 {
        self.stub_ex(sym, true, n, pops, ret)
    }

    /// [`Twins::stub`] for a callee that does not take `ecx` (`this`
    /// false: not logged).
    fn stub_ex(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        let at = self.rebase(sym);
        let addr = self.stub_code_ex(sym, this, n, pops, StubRet::Imm(ret));
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        addr
    }

    /// Points the import slot `slot` at a recording stub (tag `slot`; no
    /// `ecx`, which a stdcall caller leaves as it is).
    fn stub_import(&mut self, slot: u32, n: u32, ret: u32) -> u32 {
        let addr = self.stub_code_ex(slot, false, n, n, StubRet::Imm(ret));
        let s = self.rebase(slot);
        self.w32(s, addr);
        addr
    }

    /// A recording allocator over the original at `sym`: returns the
    /// cursor of a two-word cell (cursor, step) and moves it by the step
    /// (cursor 0, step 0: out of memory). Returns the cell.
    fn stub_alloc(&mut self, sym: u32, n: u32) -> u32 {
        let region = self.zeros(0x4_0000);
        let cell = self.alloc(&[region.to_le_bytes(), 0x400u32.to_le_bytes()].concat());
        let at = self.rebase(sym);
        let addr = self.stub_code_ex(sym, false, n, 0, StubRet::Bump(cell));
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        cell
    }

    /// The heap callees as recording stubs: `PhyreMalloc` and the user
    /// heap bump allocators, `PhyreFree` a recorder. Returns
    /// `PhyreMalloc`'s cell.
    fn stub_heap(&mut self) -> u32 {
        let cell = self.stub_alloc(ffx_pcore::PHYRE_MALLOC_ALIGNED, 2);
        self.stub_alloc(string::USER_MALLOC, 1);
        self.stub_alloc(string::USER_REALLOC, 2);
        let free = self.stub_code_ex(ffx_pcore::PHYRE_FREE, false, 1, 0, StubRet::Imm(0xf4ee));
        let at = self.rebase(ffx_pcore::PHYRE_FREE);
        let mut j = vec![0xe9];
        j.extend(free.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        cell
    }

    /// Makes `PhyreMalloc` fail (or work again).
    fn malloc_fails(&mut self, cell: u32, fail: bool, saved: &mut [u32; 2]) {
        if fail {
            *saved = [self.r32(cell), self.r32(cell + 4)];
            self.w32(cell, 0);
            self.w32(cell + 4, 0);
        } else {
            self.w32(cell, saved[0]);
            self.w32(cell + 4, saved[1]);
        }
    }

    /// Places code in the cave; returns its address.
    fn code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A recording stub in the cave, logging `ecx`.
    fn stub_code(&mut self, tag: u32, n: u32, pops: u32, ret: u32) -> u32 {
        self.stub_code_ex(tag, true, n, pops, StubRet::Imm(ret))
    }

    /// A recording stub in the cave.
    fn stub_code_ex(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: StubRet) -> u32 {
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
        match ret {
            StubRet::Imm(v) => {
                c.push(0xb8);
                c.extend(v.to_le_bytes()); // mov eax, ret
            }
            StubRet::Bump(cell) => {
                c.push(0xa1);
                c.extend(cell.to_le_bytes()); // mov eax, [cell]
                c.extend([0x8b, 0x15]);
                c.extend((cell + 4).to_le_bytes()); // mov edx, [cell + 4]
                c.extend([0x01, 0x15]);
                c.extend(cell.to_le_bytes()); // add [cell], edx
            }
        }
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    fn log_entries(&self) -> u32 {
        self.r32(self.log)
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

    /// Runs the original at `sym` in `a` and `port` on `b`; compares the
    /// result as `port` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }
}

// ---- the thread primitives ----

#[test]
fn sync_primitives_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_9bdf);
        t.stub_import(sync::CREATE_EVENT, 4, 0x1111_0001);
        t.stub_import(sync::CREATE_SEMAPHORE, 4, 0x2222_0002);
        t.stub_import(sync::SET_EVENT, 1, 1);
        t.stub_import(sync::RESET_EVENT, 1, 1);
        t.stub_import(sync::WAIT, 2, 0);
        // ReleaseSemaphore: logs, writes 7 to *prev when it succeeds
        // (eax 1 for an even handle), returns that
        let log = t.log;
        let mut c = vec![];
        c.extend([0xa1]);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05, 0x05]);
        c.extend((log + 4).to_le_bytes()); // shl eax, 5; add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sync::RELEASE_SEMAPHORE.to_le_bytes());
        c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // handle
        c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x0c]); // count
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes());
        c.extend([0x8b, 0x44, 0x24, 0x04, 0xf7, 0xd0, 0x83, 0xe0, 0x01]); // eax = !h & 1
        c.extend([0x74, 0x0a]); // jz +10
        c.extend([0x8b, 0x54, 0x24, 0x0c]); // mov edx, [esp+0xc]
        c.extend([0xc7, 0x02, 7, 0, 0, 0]); // mov dword [edx], 7
        c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        let rel = t.code(&c);
        let slot = t.rebase(sync::RELEASE_SEMAPHORE);
        t.w32(slot, rel);
        for i in 0..300 {
            let what = format!("{name} sync {i}");
            let obj = t.alloc(&(0..0x20).map(|_| rng.next() as u8).collect::<Vec<_>>());
            let arg = rng.next() >> rng.below(32);
            let n_before = t.log_entries();
            match rng.below(14) {
                0 | 1 => {
                    let sym = [0x0042_fc80, 0x0042_fc90][rng.below(2) as usize];
                    t.check(&what, sym, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(sync::bridge::handle_init(g, obj))
                    });
                }
                2 => {
                    let flags = rng.below(8);
                    t.check(&what, 0x0043_0050, Conv::Thiscall(obj), &[flags], |g| {
                        Out::U32(sync::bridge::event_initialize(g, obj, flags))
                    });
                }
                3 => {
                    let max = [arg, 0x7fff_ffff, 0x8000_0000, u32::MAX, 0x7fff_fffe]
                        [rng.below(5) as usize];
                    t.check(&what, 0x0043_0080, Conv::Thiscall(obj), &[arg, max], |g| {
                        Out::U32(sync::bridge::semaphore_initialize(g, obj, arg, max))
                    });
                }
                4 => t.check(&what, 0x0043_01e0, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::event_reset(g, obj))
                }),
                5 => t.check(&what, 0x0043_0200, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::event_signal(g, obj))
                }),
                6 => {
                    let prev = if rng.chance(3) { 0 } else { obj + 0x10 };
                    let scratch = tmp(&t.b);
                    t.check(&what, 0x0043_0210, Conv::Thiscall(obj), &[arg, prev], |g| {
                        Out::U32(sync::bridge::semaphore_signal(g, scratch, obj, arg, prev))
                    });
                }
                7 => t.check(&what, 0x0043_0280, Conv::Thiscall(obj), &[arg], |g| {
                    Out::U32(sync::bridge::wait_for(g, obj, arg))
                }),
                8 => {
                    let sym = [0x0043_02a0, 0x0043_02b0][rng.below(2) as usize];
                    t.check(&what, sym, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(sync::bridge::wait(g, obj))
                    });
                }
                9 => t.check(&what, 0x0043_1690, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::critical_section_simple(g, obj))
                }),
                10 => t.check(&what, 0x0043_16d0, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::event_flag_with_byte(g, obj))
                }),
                11 => {
                    let sym = [0x0043_16f0, 0x0042_fca0][rng.below(2) as usize];
                    t.check(&what, sym, Conv::Thiscall(obj), &[arg], |g| {
                        Out::U32(sync::bridge::thread_initialization(g, obj, arg))
                    });
                }
                12 => {
                    if rng.chance(2) {
                        t.write(obj, &[0]);
                    }
                    t.check(&what, 0x0043_17f0, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(sync::bridge::critical_section_initialize(g, obj))
                    });
                }
                _ => {
                    let b = rng.next();
                    t.check(&what, 0x0044_cbe0, Conv::Thiscall(obj), &[arg, b], |g| {
                        Out::U32(sync::bridge::thread_pool(g, obj, arg, b))
                    });
                }
            }
            let _ = n_before;
        }
        assert!(t.log_entries() > 100, "{name}: calls logged");
    }
}

#[test]
fn critical_section_locks_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f0f_1234);
        t.stub_import(sync::SET_EVENT, 1, 1);
        t.stub_import(sync::RESET_EVENT, 1, 1);
        // WaitForSingleObject: logs, then frees the lock word at `lock`
        // (a fixed place each test writes) and returns 0
        let lock_at = t.zeros(4);
        let log = t.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes());
        c.extend([0xc1, 0xe0, 0x05, 0x05]);
        c.extend((log + 4).to_le_bytes());
        c.extend([0xc7, 0x00]);
        c.extend(sync::WAIT.to_le_bytes());
        c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]);
        c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x0c]);
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes());
        c.extend([0x8b, 0x15]);
        c.extend(lock_at.to_le_bytes()); // mov edx, [lock_at]
        c.extend([0xc7, 0x02, 0, 0, 0, 0]); // mov dword [edx], 0
        c.extend([0x31, 0xc0, 0xc2, 0x08, 0x00]); // xor eax, eax; ret 8
        let w = t.code(&c);
        let slot = t.rebase(sync::WAIT);
        t.w32(slot, w);
        let release = t.stub(sync::EVENT_DESTROY, 0, 0, 0x55);
        let _ = release;
        let mut waited = 0;
        for i in 0..300 {
            let what = format!("{name} lock {i}");
            let mut bytes: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
            // lock word: free, taken (the wait frees it) or junk
            let lock = [0u32, 1, 0, rng.next()][rng.below(4) as usize];
            bytes[0x10..0x14].copy_from_slice(&lock.to_le_bytes());
            let waiters = [0u32, 1, 0xffff_ffff, rng.next()][rng.below(4) as usize];
            bytes[0xc..0x10].copy_from_slice(&waiters.to_le_bytes());
            let obj = t.alloc(&bytes);
            t.w32(lock_at, obj + 0x10);
            if lock != 0 {
                waited += 1;
            }
            match rng.below(4) {
                0 => t.check(&what, 0x0043_1920, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::critical_section_lock(g, obj))
                }),
                1 => {
                    let other =
                        t.alloc(&[rng.next().to_le_bytes(), rng.next().to_le_bytes()].concat());
                    t.w32(lock_at, other);
                    t.check(&what, 0x0043_18c0, Conv::Thiscall(obj), &[other], |g| {
                        Out::U32(sync::bridge::lock_wait(g, obj, other))
                    });
                }
                2 => t.check(&what, 0x0043_1a80, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(sync::bridge::critical_section_unlock(g, obj))
                }),
                _ => t.check(&what, 0x0043_1950, Conv::Thiscall(obj), &[], |g| {
                    sync::bridge::critical_section_release(g, obj);
                    Out::None
                }),
            }
        }
        assert!(waited > 20, "{name}: contended locks");
        // the thread pool's globals
        for i in 0..40 {
            let what = format!("{name} pool {i}");
            let n = rng.next();
            match i % 3 {
                0 => t.check(&what, 0x0044_ce10, Conv::Cdecl, &[n], |g| {
                    Out::U32(sync::bridge::set_thread_count(g, n))
                }),
                1 => t.check(&what, 0x0044_ce20, Conv::Cdecl, &[n], |g| {
                    Out::U32(sync::bridge::set_stack_size(g, n))
                }),
                _ => t.check(&what, 0x0044_cde0, Conv::Cdecl, &[], |g| {
                    Out::U32(sync::bridge::thread_count(g))
                }),
            }
        }
    }
}

// ---- free lists ----

#[test]
fn free_lists_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_1357);
        let heap = t.stub_heap();
        let mut saved = [0u32; 2];
        let mut made = 0;
        for i in 0..300 {
            let what = format!("{name} freelist {i}");
            if saved[0] != 0 {
                t.malloc_fails(heap, false, &mut saved);
                saved = [0; 2];
            }
            if rng.chance(6) {
                t.malloc_fails(heap, true, &mut saved);
            }
            let size = [4, 5, 8, 12, 16, 0x24, 1][rng.below(7) as usize];
            let align = [4, 8, 16, 1, 0x20][rng.below(5) as usize];
            let mut fl = [0u32; 6];
            fl[0] = if rng.chance(2) { 0 } else { rng.next() };
            fl[1] = size;
            fl[3] = align;
            let this = t.alloc(&fl.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>());
            match rng.below(7) {
                0 => {
                    let args = [
                        rng.next() >> rng.below(32),
                        rng.next(),
                        rng.below(40),
                        rng.next(),
                    ];
                    t.check(&what, 0x0049_c480, Conv::Thiscall(this), &args, |g| {
                        Out::U32(freelist::bridge::new(
                            g, this, args[0], args[1], args[2], args[3],
                        ))
                    });
                }
                1 | 2 => {
                    let n = [0, 1, 2, 5, 17][rng.below(5) as usize];
                    t.check(&what, 0x0049_c8d0, Conv::Thiscall(this), &[n], |g| {
                        Out::U8(freelist::bridge::allocate_block(g, this, n))
                    });
                    if n != 0 {
                        made += 1;
                        // and the blocks freed again
                        let sym = [0x0049_c5b0, 0x0049_cc10][rng.below(2) as usize];
                        if rng.chance(2) {
                            t.check(&what, 0x0049_c8d0, Conv::Thiscall(this), &[3], |g| {
                                Out::U8(freelist::bridge::allocate_block(g, this, 3))
                            });
                        }
                        t.check(
                            &format!("{what} free"),
                            sym,
                            Conv::Thiscall(this),
                            &[],
                            |g| {
                                freelist::bridge::free_blocks(g, this);
                                Out::None
                            },
                        );
                    }
                }
                3 => {
                    let n = [0, 1, 2, 7][rng.below(4) as usize];
                    let first = if rng.chance(4) { 0 } else { t.zeros(0x200) };
                    t.check(&what, 0x0049_c7c0, Conv::Thiscall(this), &[first, n], |g| {
                        Out::U32(freelist::bridge::link_items(g, this, first, n))
                    });
                }
                4 => {
                    let item = t.zeros(8);
                    t.check(&what, 0x0049_c980, Conv::Thiscall(this), &[item], |g| {
                        Out::U32(freelist::bridge::deallocate(g, this, item))
                    });
                }
                5 => {
                    let item = t.zeros(8);
                    t.check(&what, 0x0049_c7a0, Conv::Stdcall, &[item, this], |g| {
                        Out::U32(freelist::bridge::add_item(g, item, this))
                    });
                }
                _ => t.check(&what, 0x0049_cc10, Conv::Thiscall(this), &[], |g| {
                    freelist::bridge::free_blocks(g, this);
                    Out::None
                }),
            }
        }
        assert!(made > 30, "{name}: blocks made");
    }
}

// ---- the little members ----

#[test]
fn class_members_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_1111);
        t.stub_heap();
        for sym in [
            small::DESTROY_44AF00,
            small::DESTROY_499DC0,
            small::DESTROY_4EA2D0,
            small::DESTROY_4E9F70,
        ] {
            t.stub(sym, 0, 0, 0x1234_5600 | (sym & 0xff));
        }
        t.stub_ex(small::DESTROY_ARRAY_4EC310, false, 2, 0, 0x99);
        t.stub(small::CALL_A3BAF0, 2, 2, 0x4242_4242);
        t.stub(small::RING_ALLOCATE_BLOCK, 1, 1, 0);
        // a thiscall callback for the +0x3c/+0x38 tables (logs, ret 8 / ret)
        let cb3c = t.stub_code(0x3c, 2, 2, 0x3c3c);
        let cb38 = t.stub_code(0x38, 0, 0, 0x3838);
        // a COM object whose Release logs
        // stdcall (COM): `ecx` is not an argument
        let rel = t.stub_code_ex(0xc0, false, 1, 1, StubRet::Imm(3));
        let vt = t.alloc(
            &[[0u32, 0, rel]]
                .iter()
                .flatten()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        for i in 0..600 {
            let what = format!("{name} member {i}");
            let p = if rng.chance(8) {
                0
            } else {
                t.alloc(&(0..0x100).map(|_| rng.next() as u8).collect::<Vec<_>>())
            };
            let q = if p == 0 { t.alloc(&[0xab; 0x100]) } else { p };
            let case = rng.below(16);
            let what = format!("{what} (case {case})");
            match case {
                0 => {
                    let (sym, words) = [
                        (0x0044_c2b0, 2),
                        (0x0044_c2d0, 2),
                        (0x0049_bb00, 2),
                        (0x004c_c1d0, 2),
                        (0x004e_ea90, 2),
                        (0x004e_eab0, 2),
                        (0x004e_ead0, 2),
                        (0x004e_eaf0, 2),
                        (0x004e_eb10, 2),
                        (0x00a3_4270, 2),
                        (0x00a3_4290, 3),
                        (0x0044_59e0, 1),
                    ][rng.below(12) as usize];
                    t.check(&what, sym, Conv::Stdcall, &[p], |g| {
                        Out::U32(small::bridge::clear_words(g, p, words))
                    });
                }
                1 => t.check(&what, 0x0049_bb20, Conv::Stdcall, &[p], |g| {
                    Out::U32(small::bridge::clear_49bb20(g, p))
                }),
                2 => t.check(&what, 0x004d_3eb0, Conv::Stdcall, &[p], |g| {
                    Out::U32(small::bridge::clear_4d3eb0(g, p))
                }),
                3 => t.check(&what, 0x004c_c240, Conv::Stdcall, &[p], |g| {
                    Out::U32(small::bridge::init_4cc240(g, p))
                }),
                4 | 5 => {
                    // owned or borrowed buffers, null or a real block
                    let count = rng.next() & 0xc000_00ff;
                    let data = match rng.below(3) {
                        0 => 0,
                        _ => t.zeros(0x40),
                    };
                    if data != 0 {
                        t.a.clear_dirty();
                        t.b.clear_dirty();
                    }
                    t.w32(q, count);
                    t.w32(q + 4, data);
                    let (sym, checked) = [
                        (0x0049_bb60, true),
                        (0x004c_c2a0, true),
                        (0x004e_eb50, false),
                        (0x004e_eb90, false),
                        (0x004e_ebd0, false),
                        (0x004e_ec10, false),
                        (0x00a3_42c0, false),
                        (0x00a3_4300, false),
                    ][rng.below(8) as usize];
                    t.check(&what, sym, Conv::Stdcall, &[q], |g| {
                        Out::U8(small::bridge::free_array(g, q, checked))
                    });
                }
                6 => t.check(&what, 0x004e_ec50, Conv::Stdcall, &[q], |g| {
                    Out::U8(small::bridge::destroy_array(g, q))
                }),
                7 => {
                    let (sym, target, checked) = [
                        (0x0044_c2f0, small::DESTROY_44AF00, false),
                        (0x0044_c310, small::DESTROY_44AF00, false),
                        (0x0049_bba0, small::DESTROY_499DC0, false),
                        (0x004e_ec90, small::DESTROY_4EA2D0, false),
                        (0x004e_eb30, small::DESTROY_4E9F70, true),
                    ][rng.below(5) as usize];
                    t.check(&what, sym, Conv::Stdcall, &[p], |g| {
                        Out::U8(small::bridge::forward(g, target, p, checked))
                    });
                }
                8 => {
                    let text = match rng.below(3) {
                        0 => 0,
                        1 => q | 1,
                        _ => t.zeros(0x10),
                    };
                    if text != 0 && text & 1 == 0 {
                        t.a.clear_dirty();
                        t.b.clear_dirty();
                    }
                    t.w32(q, text);
                    t.check(&what, 0x0044_5a00, Conv::Stdcall, &[q], |g| {
                        Out::U8(small::bridge::free_string(g, q))
                    });
                }
                9 => {
                    t.w32(q + 0x3c, if rng.chance(3) { 0 } else { cb3c });
                    let (o, a, b) = (rng.next(), rng.next(), rng.next());
                    let sym = [0x0044_5d60, 0x004f_9150, 0x004f_9180][rng.below(3) as usize];
                    t.check(&what, sym, Conv::Thiscall(q), &[o, a, b], |g| {
                        Out::U32(small::bridge::callback_3c(g, q, o, a, b))
                    });
                }
                10 => {
                    let owner = t.alloc(&(0..0xc0).map(|_| rng.next() as u8).collect::<Vec<_>>());
                    t.w32(q + 0x38, if rng.chance(2) { 0 } else { cb38 });
                    t.w32(q + 0xc, owner);
                    let table = t.alloc(&(0..0x40).map(|_| rng.next() as u8).collect::<Vec<_>>());
                    let base: u32 = t.r32(owner + 0x88);
                    t.w32(q + 0x24, table.wrapping_add(base));
                    let idx = rng.below(0x3c);
                    t.check(
                        &what,
                        [0x004e_ee60, 0x004e_eec0][rng.below(2) as usize],
                        Conv::Thiscall(q),
                        &[idx],
                        |g| Out::U32(small::bridge::callback_38(g, q, idx)),
                    );
                }
                11 => match rng.below(3) {
                    2 => {
                        let n =
                            [0, 1, 2, 0x8000_0000, 0x8000_0001, rng.next()][rng.below(6) as usize];
                        t.w32(q, n);
                        t.check(&what, 0x004d_45a0, Conv::Thiscall(q), &[], |g| {
                            Out::U32(small::bridge::array_data(g, q))
                        });
                    }
                    0 => t.check(&what, 0x00a3_4a30, Conv::Thiscall(q), &[], |g| {
                        Out::U32(small::bridge::clear_pair(g, q))
                    }),
                    _ => t.check(&what, 0x00a3_4a60, Conv::Thiscall(q), &[], |g| {
                        Out::U32(small::bridge::init_a34a60(g, q))
                    }),
                },
                12 => {
                    let obj = if rng.chance(3) {
                        0
                    } else {
                        t.alloc(&vt.to_le_bytes())
                    };
                    t.w32(q, obj);
                    t.check(&what, 0x00a3_4910, Conv::Cdecl, &[q], |g| {
                        small::bridge::release(g, q);
                        Out::None
                    });
                }
                13 => {
                    let inner = t.alloc(&(0..0x10).map(|_| rng.next() as u8).collect::<Vec<_>>());
                    t.w32(q + 4, inner);
                    t.check(&what, 0x00a3_4ae0, Conv::Cdecl, &[q], |g| {
                        Out::U32(small::bridge::call_a34ae0(g, q))
                    });
                }
                14 => {
                    let words: Vec<u32> = (0..12)
                        .map(|_| match rng.below(4) {
                            0 => 0x1000,
                            1 => 0x2000,
                            2 => rng.below(0x3000),
                            _ => rng.next(),
                        })
                        .collect();
                    let mut words = words;
                    if rng.chance(3) {
                        words[0] = words[4]; // start == read
                    }
                    for (k, w) in words.iter().enumerate() {
                        t.w32(q + 0x18 + 4 * k as u32, *w);
                    }
                    let (end, used, read) = (words[1], words[3], words[4]);
                    let n = match rng.below(7) {
                        0 => read.wrapping_sub(used),
                        1 => end.wrapping_sub(used),
                        2 => read.wrapping_sub(used).wrapping_sub(1),
                        3 => end.wrapping_sub(used).wrapping_sub(1),
                        _ => [0, 1, 0x1000, rng.next(), rng.below(0x4000)][rng.below(5) as usize],
                    };
                    t.check(&what, 0x005c_d340, Conv::Thiscall(q), &[n], |g| {
                        Out::U32(small::bridge::ring_fits(g, q, n))
                    });
                }
                _ => {
                    let n = rng.next();
                    t.check(&what, 0x005c_d0f0, Conv::Thiscall(q), &[n], |g| {
                        Out::U32(small::bridge::ring_allocate(g, q, n))
                    });
                }
            }
        }
        assert!(t.log_entries() > 100, "{name}: calls logged");
    }
}

#[test]
fn ring_allocate_marks_the_block() {
    for (name, mut t) in Twins::all() {
        let block = t.zeros(0x10);
        t.stub(small::RING_ALLOCATE_BLOCK, 1, 1, block);
        let ring = t.zeros(0x40);
        t.check(
            &format!("{name} ring"),
            0x005c_d0f0,
            Conv::Thiscall(ring),
            &[0x80],
            |g| Out::U32(small::bridge::ring_allocate(g, ring, 0x80)),
        );
        assert_eq!(t.r32(block + 4), 1);
    }
}

#[test]
fn a34b10_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5151_7272);
        let state = t.zeros(0xa00);
        let g = t.rebase(small::A34_STATE);
        t.w32(g, state);
        let rets = t.zeros(12);
        let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
        c.extend([0x8b, 0x04, 0x85]);
        c.extend(rets.to_le_bytes()); // mov eax, [rets + eax*4]
        c.push(0xc3);
        let code = t.code(&c);
        let at = t.rebase(small::A35A70);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        for i in 0..60 {
            // the allocator: returns the word of `rets` its kind (1, 2)
            // picks, 0 on some calls
            let pick = |r: &mut Rng| [0, 0x1000_0000, r.next()][r.below(3) as usize];
            let (r1, r2) = (pick(&mut rng), pick(&mut rng));
            t.w32(rets + 4, r1);
            t.w32(rets + 8, r2);
            let n = [0, 4, 0x13, 0x1234, 0x2_0000, 0x10_0000, rng.next()][rng.below(7) as usize];
            t.check(
                &format!("{name} a34b10 {i}"),
                0x00a3_4b10,
                Conv::Cdecl,
                &[n],
                |g| Out::U32(small::bridge::init_a34b10(g, n)),
            );
        }
    }
}

#[test]
fn constant_members_match_the_original() {
    for (name, mut t) in Twins::all() {
        for sym in [0x0044_5a20, 0x0045_2090, 0x004e_ed00] {
            t.check(
                &format!("{name} {sym:#x}"),
                sym,
                Conv::Stdcall,
                &[7],
                |_| Out::U32(0),
            );
        }
        for sym in [0x0045_2080, 0x004c_c360, 0x004d_3ee0] {
            t.check(
                &format!("{name} {sym:#x}"),
                sym,
                Conv::Stdcall,
                &[7],
                |_| Out::U8(1),
            );
        }
    }
}

// ---- strings ----

/// A C string of `n` bytes (no zero inside) and its terminator.
fn text(rng: &mut Rng, n: usize) -> Vec<u8> {
    let mut v: Vec<u8> = (0..n).map(|_| 1 + rng.below(255) as u8).collect();
    v.push(0);
    v
}

#[test]
fn strings_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3141_5926);
        let heap = t.stub_heap();
        let mut saved = [0u32; 2];
        let grow = t.stub(string::BUILDER_GROW, 1, 1, 0);
        let _ = grow;
        for i in 0..400 {
            let what = format!("{name} string {i}");
            if saved[0] != 0 {
                t.malloc_fails(heap, false, &mut saved);
                saved = [0; 2];
            }
            if rng.chance(6) {
                t.malloc_fails(heap, true, &mut saved);
            }
            let len = [0, 1, 5, 31, 200][rng.below(5) as usize];
            let s = if rng.chance(6) {
                0
            } else {
                t.alloc(&text(&mut rng, len))
            };
            // the string's current text: none, borrowed or owned
            let this = t.zeros(8);
            match rng.below(3) {
                0 => {}
                1 => t.w32(this, s | 1),
                _ => {
                    let p = t.zeros(8);
                    t.w32(this, p);
                }
            }
            t.diff(&format!("{what} setup"));
            match rng.below(7) {
                0 => t.check(&what, 0x0042_f090, Conv::Thiscall(this), &[s], |g| {
                    Out::U32(string::bridge::new(g, this, s))
                }),
                1 => t.check(&what, 0x0044_5cf0, Conv::Thiscall(this), &[s], |g| {
                    Out::U32(string::bridge::assign(g, this, s))
                }),
                2 => {
                    let off = [0, 1, 0x40, 0x41, rng.next()][rng.below(5) as usize];
                    t.check(
                        &what,
                        0x0044_5d90,
                        Conv::Thiscall(this),
                        &[off, rng.next()],
                        |g| Out::U32(string::bridge::fixup(g, this, off)),
                    );
                }
                3 | 4 => {
                    // a builder: capacity, buffer, length
                    let cap = [0, 8, 0x100, 0x8000_0100][rng.below(4) as usize];
                    let blen = [0, 1, 3, 9][rng.below(4) as usize];
                    let buf = t.alloc(&(0..0x300).map(|_| rng.next() as u8).collect::<Vec<_>>());
                    let b = t.alloc(
                        &[cap, buf, blen]
                            .iter()
                            .flat_map(|w| w.to_le_bytes())
                            .collect::<Vec<_>>(),
                    );
                    let mut src = text(&mut rng, len);
                    if rng.chance(3) && len > 2 {
                        src[len / 2] = 0;
                    }
                    let sp = t.alloc(&src);
                    let n = if rng.chance(4) {
                        0
                    } else {
                        (len as u32 + 1).min(rng.below(len as u32 + 2))
                    };
                    if rng.chance(3) {
                        let need = blen + u32::from(blen == 0) + n;
                        t.w32(b, (cap & 0x8000_0000) | need);
                    }
                    if rng.chance(2) {
                        let ra =
                            t.a.call(t.a.rebase(0x0044_4ce0), Conv::Thiscall(b), &[sp, n])
                                .unwrap();
                        let rb = string::bridge::append(&mut G(&mut t.b), b, sp, n);
                        if let Some(v) = rb {
                            assert_eq!(ra.eax, v, "{what}: append eax");
                        }
                        t.diff(&what);
                    } else {
                        let p = if rng.chance(5) { 0 } else { sp };
                        let ra =
                            t.a.call(t.a.rebase(0x0044_4bf0), Conv::Thiscall(b), &[p])
                                .unwrap();
                        let rb = string::bridge::append_str(&mut G(&mut t.b), b, p);
                        if let Some(v) = rb {
                            assert_eq!(ra.eax, v, "{what}: += eax");
                        }
                        t.diff(&what);
                    }
                }
                5 => {
                    let n = [0, 1, 7, 64][rng.below(4) as usize];
                    let count = rng.below(n + 1);
                    let src = t.alloc(&text(&mut rng, 64));
                    t.check(&what, 0x0044_4c50, Conv::Cdecl, &[n, src, count], |g| {
                        Out::U32(string::bridge::alloc_copy(g, n, src, count))
                    });
                }
                _ => {
                    if rng.chance(2) {
                        let (n, size) = (rng.below(20), rng.below(40));
                        t.check(&what, 0x0042_fae0, Conv::Cdecl, &[n, size], |g| {
                            Out::U32(string::bridge::calloc_aligned(g, n, size))
                        });
                    } else {
                        let old = if rng.chance(3) {
                            0
                        } else {
                            let p = t.zeros(0x20);
                            t.a.clear_dirty();
                            t.b.clear_dirty();
                            p
                        };
                        let size = rng.below(0x200);
                        t.check(&what, 0x0042_fbd0, Conv::Cdecl, &[old, size], |g| {
                            Out::U32(string::bridge::realloc_aligned(g, old, size))
                        });
                    }
                }
            }
        }
    }
}

// ---- matrices ----

#[test]
fn matrix_helpers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0dd0_beef);
        for i in 0..1500 {
            let what = format!("{name} matrix {i}");
            let wide = rng.chance(3);
            let m = t.alloc(&if wide {
                rng.floats_wide(16)
            } else {
                rng.floats(16)
            });
            let v = [0; 4].map(|_| {
                t.alloc(&if wide {
                    rng.floats_wide(4)
                } else {
                    rng.floats(4)
                })
            });
            // outputs apart, or over an input
            let out = match rng.below(4) {
                0 => m,
                1 => v[0],
                _ => t.zeros(64),
            };
            let scratch = tmp(&t.b);
            match rng.below(15) {
                0 => t.check(&what, 0x0045_3340, Conv::Thiscall(out), &[m], |g| {
                    Out::U32(matrix::bridge::copy(g, out, m, false))
                }),
                1 => t.check(&what, 0x004c_9430, Conv::Thiscall(out), &[m], |g| {
                    Out::U32(matrix::bridge::copy(g, out, m, true))
                }),
                2 => t.check(&what, 0x0045_0d40, Conv::Thiscall(out), &v, |g| {
                    Out::U32(matrix::bridge::from_rows(g, out, v))
                }),
                3 => t.check(&what, 0x0045_0de0, Conv::Thiscall(out), &v, |g| {
                    Out::U32(matrix::bridge::from_axes(g, out, v))
                }),
                4 => t.check(&what, 0x0045_1080, Conv::Thiscall(out), &[m], |g| {
                    Out::U32(matrix::bridge::from_3x4(g, out, m))
                }),
                5 => t.check(&what, 0x0045_1190, Conv::Thiscall(out), &[m], |g| {
                    Out::U32(matrix::bridge::to_world_3x4(g, out, m))
                }),
                6 => t.check(&what, 0x0045_1bb0, Conv::Cdecl, &[out], |g| {
                    Out::U32(matrix::bridge::identity(g, out))
                }),
                7 => {
                    let p = if rng.chance(4) { 0 } else { out };
                    t.check(&what, 0x0045_1fe0, Conv::Stdcall, &[p], |g| {
                        Out::U8(matrix::bridge::world_identity(g, scratch, p))
                    });
                }
                8 => {
                    let obj = t.zeros(0x150);
                    t.check(&what, 0x0045_30a0, Conv::Thiscall(obj), &[], |g| {
                        Out::U32(matrix::bridge::init_4530a0(g, scratch, obj))
                    });
                }
                9 => t.check(&what, 0x004c_9360, Conv::Thiscall(out), &[m], |g| {
                    Out::U32(matrix::bridge::affine(g, out, m))
                }),
                10 | 11 => {
                    let o = if rng.chance(3) { v[1] } else { out };
                    t.check(&what, 0x004c_9c20, Conv::Thiscall(m), &[o, v[1]], |g| {
                        Out::U32(matrix::bridge::row_times_vector(g, m, o, v[1]))
                    });
                }
                12 | 13 => {
                    let o = if rng.chance(3) { v[2] } else { out };
                    t.check(&what, 0x004c_9cb0, Conv::Thiscall(m), &[o, v[2]], |g| {
                        Out::U32(matrix::bridge::transform_point(g, m, o, v[2]))
                    });
                }
                _ => {
                    let b = t.alloc(&rng.floats(16));
                    let o = [b, m, out][rng.below(3) as usize];
                    t.check(&what, 0x004c_9bc0, Conv::Thiscall(m), &[o, b], |g| {
                        Out::U32(matrix::bridge::times_rows(g, scratch, m, o, b))
                    });
                }
            }
        }
    }
}
