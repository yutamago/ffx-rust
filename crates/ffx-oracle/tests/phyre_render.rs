//! The map world's render chain (`ffx_prender`, the M4 pilot) against the
//! original: the entries' sort and comparator, `PVisibilityCheck` and the
//! culling job, `mergeResults`, `renderWorldCulledSorted` and
//! `renderWorld`. On the image alone with synthetic data, and on a
//! snapshot (`FFX_SNAPSHOT`) with the same data over the snapshot's memory.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are run as they are (the math helpers, the
//! box test, the comparator) or replaced in both oracles alike by
//! recording stubs (the world cursors, the job system, the ring
//! allocators, the mesh-instance list): each call appends its tag, `this`
//! and arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order.
//!
//! Frames: an original that hands the address of its locals to callees
//! (`renderWorldCulledSorted`) is compared with its port running on a
//! frame at the same address in the other oracle, whose calls then start
//! below it (`Oracle::set_call_top`), so both log the same pointers.
//! Values the port only keeps for itself (the sort's entry being placed)
//! live at the bottom of its oracle's stack, which the diff leaves out as
//! it does the original's frames.

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
use ffx_prender::{sort, thread, visibility, world};

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
    /// A float that is not a signalling NaN (the emulator keeps those).
    fn float(&mut self) -> f32 {
        loop {
            let bits = match self.below(5) {
                0 => self.next(),
                1 | 2 => (self.below(4000) as f32 - 2000.0).to_bits(),
                3 => ((self.next() as f32) / 1e7 - 200.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    0xffc0_1234,
                    1,
                    0x7f7f_ffff,
                ][self.below(8) as usize],
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return f32::from_bits(bits);
            }
        }
    }
    /// A float of a modest range, no NaNs (geometry).
    fn coord(&mut self) -> f32 {
        if self.chance(40) {
            return self.float();
        }
        (self.below(20_000) as f32 - 10_000.0) / 37.0
    }
    /// A render order: the classes, the `% 1000` and `/ 1000` parts the
    /// comparator reads, negative ones too.
    fn order(&mut self) -> i32 {
        let base = match self.below(4) {
            0 => 0,
            1 => sort::CLASS,
            2 => (self.next() as i32) % 300_000_000,
            _ => -(self.below(3) as i32) * sort::CLASS,
        };
        base + (self.below(4) as i32) * 1000 + self.below(4) as i32
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

/// Memory only the port uses (its `tmp`): the bottom of its oracle's
/// stack, which no call reaches and the diff leaves out.
fn tmp(o: &Oracle) -> u32 {
    o.stack_range().start + 0x100
}

/// Where [`Oracle::call`] pushes by default.
fn call_top(o: &Oracle) -> u32 {
    o.stack_range().end - 0x100
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

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: std::collections::HashMap<u32, Vec<u8>>,
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
            saved: std::collections::HashMap::new(),
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

    fn wf(&mut self, at: u32, v: f32) {
        self.w32(at, v.to_bits());
    }

    fn reset(&mut self) {
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
        self.a.reset_heap();
        self.b.reset_heap();
        // the log stays where it is: the first allocation again
        let log2 = self.zeros(0x8000);
        assert_eq!(log2, self.log);
    }

    /// Writes a recording stub: logs `tag`, `ecx` when `this`, and `n`
    /// stack arguments; pops `pops` words; returns `ret`. Placed at the
    /// original `sym` (a `jmp` there). Returns its address.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        let at = self.rebase(sym);
        let addr = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
        addr
    }

    /// The stub of [`Twins::stub`] in the cave only (for vtables), `tag`
    /// its log tag. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        self.stub_code_from(tag, this, 0, n, pops, ret)
    }

    /// [`Twins::stub_code`] logging stack words `skip..skip + n`.
    fn stub_code_from(
        &mut self,
        tag: u32,
        this: bool,
        skip: u32,
        n: u32,
        pops: u32,
        ret: u32,
    ) -> u32 {
        assert!(n <= 6);
        let sym = tag;
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * (i + skip)) as u8]); // mov edx, [esp+4+4(i+skip)]
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
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// Puts back the original bytes at `sym` (undoes [`Twins::stub`]).
    fn unstub(&mut self, sym: u32) {
        let at = self.rebase(sym);
        let bytes = self.saved.remove(&at).expect("stubbed");
        self.write(at, &bytes);
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
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

// ---- the comparator and the sort ----

const COMPARE: u32 = 0x009f_2d70;
const SORT: u32 = 0x009f_17c0;
const INSERTION: u32 = 0x009f_0d20;
const MEDIAN3: u32 = 0x009f_10e0;
const MEDIAN: u32 = 0x009f_1230;
const PARTITION: u32 = 0x009f_1c60;
const PUSH_HEAP: u32 = 0x009f_1630;
const ADJUST_HEAP: u32 = 0x009f_0b60;
const MAKE_HEAP: u32 = 0x009f_0f50;
const SORT_HEAP: u32 = 0x009f_1970;

/// A second comparator for the shared helpers (other sorts use them with
/// their own): `a[0] < b[0]` unsigned (cdecl, in `al`).
const BY_WORD: [u8; 16] = [
    0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
    0x8b, 0x00, // mov eax, [eax]
    0x8b, 0x4c, 0x24, 0x08, // mov ecx, [esp+8]
    0x3b, 0x01, // cmp eax, [ecx]
    0x0f, 0x92, 0xc0, // setb al
    0xc3, // ret
];

/// `n` entries with few distinct keys and orders when `dups` (the fat
/// partition's equal runs), else random ones.
fn entries(rng: &mut Rng, n: usize, dups: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * 12);
    for i in 0..n {
        let (key, order) = if dups {
            (
                (rng.below(4) as f32).to_bits(),
                [0, 1000, sort::CLASS][rng.below(3) as usize],
            )
        } else {
            (rng.float().to_bits(), rng.order())
        };
        out.extend((0x1000_0000 + i as u32).to_le_bytes());
        out.extend(key.to_le_bytes());
        out.extend(order.to_le_bytes());
    }
    out
}

fn preds(t: &mut Twins) -> [u32; 2] {
    let compare = t.rebase(COMPARE);
    let code = t.a.alloc_code(&BY_WORD).unwrap();
    assert_eq!(t.b.alloc_code(&BY_WORD).unwrap(), code);
    [compare, code]
}

#[test]
fn compare_entries_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_ace1);
        let a = t.zeros(12);
        let b = t.zeros(12);
        for i in 0..4000 {
            for p in [a, b] {
                let key = if rng.chance(3) { 1.0 } else { rng.float() };
                t.wf(p + 4, key);
                t.w32(p + 8, rng.order() as u32);
            }
            if rng.chance(4) {
                let o: u32 = Mem::read(&t.b, a + 8);
                t.w32(b + 8, o);
            }
            t.check(
                &format!("{name} compare {i}"),
                COMPARE,
                Conv::Cdecl,
                &[a, b],
                |g| Out::U8(u8::from(sort::bridge::compare_entries(g, a, b))),
            );
        }
    }
}

#[test]
fn sort_helpers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_cafe);
        let preds = preds(&mut t);
        for round in 0..400 {
            let pred = preds[rng.below(2) as usize];
            let n = match rng.below(6) {
                0 => rng.below(4),
                1 | 2 => rng.below(40),
                3 => 30 + rng.below(20),
                4 if rng.chance(3) => 41 + rng.below(2), // the median's 40
                4 => 40 + rng.below(200),
                _ => 200 + rng.below(1500),
            } as usize;
            let dups = rng.chance(3);
            let bytes = entries(&mut rng, n.max(1), dups);
            let first = t.alloc(&bytes);
            let last = first + 12 * n as u32;
            let what = format!("{name} round {round} (n {n})");
            let ta = tmp(&t.b);
            match rng.below(9) {
                0 => {
                    // small budgets reach the heap sort at different depths
                    let ideal = match rng.below(3) {
                        0 => rng.below(3) as i32,
                        1 => 2 + rng.below(12) as i32,
                        _ => n as i32,
                    };
                    t.check(
                        &format!("{what}: sort ideal {ideal}"),
                        SORT,
                        Conv::Cdecl,
                        &[first, last, ideal as u32, pred],
                        |g| {
                            sort::bridge::sort(g, ta, first, last, ideal, pred);
                            Out::None
                        },
                    );
                }
                1 if n >= 1 => t.check(
                    &format!("{what}: insertion"),
                    INSERTION,
                    Conv::Cdecl,
                    &[first, last, pred, 0],
                    |g| {
                        sort::bridge::insertion_sort(g, ta, first, last, pred);
                        Out::None
                    },
                ),
                2 if n >= 3 => {
                    let i = |r: &mut Rng| first + 12 * r.below(n as u32);
                    let (x, y, z) = (i(&mut rng), i(&mut rng), i(&mut rng));
                    t.check(
                        &format!("{what}: median3"),
                        MEDIAN3,
                        Conv::Cdecl,
                        &[x, y, z, pred],
                        |g| {
                            sort::bridge::median3(g, x, y, z, pred);
                            Out::None
                        },
                    );
                }
                3 if n >= 3 => {
                    let mid = first + 12 * (n as u32 / 2);
                    let back = last - 12;
                    t.check(
                        &format!("{what}: median"),
                        MEDIAN,
                        Conv::Cdecl,
                        &[first, mid, back, pred],
                        |g| {
                            sort::bridge::median(g, first, mid, back, pred);
                            Out::None
                        },
                    );
                }
                4 if n >= 2 => {
                    let out = t.zeros(8);
                    t.check(
                        &format!("{what}: partition"),
                        PARTITION,
                        Conv::Cdecl,
                        &[out, first, last, pred],
                        |g| Out::U32(sort::bridge::partition_out(g, out, first, last, pred)),
                    );
                }
                5 => t.check(
                    &format!("{what}: make_heap"),
                    MAKE_HEAP,
                    Conv::Cdecl,
                    &[first, last, pred, 0, 0],
                    |g| {
                        sort::bridge::make_heap(g, ta, first, last, pred);
                        Out::None
                    },
                ),
                6 => {
                    // a heap first, as sort_heap expects (made by the
                    // original in both)
                    let mh = t.rebase(MAKE_HEAP);
                    t.a.call(mh, Conv::Cdecl, &[first, last, pred, 0, 0])
                        .unwrap();
                    t.b.call(mh, Conv::Cdecl, &[first, last, pred, 0, 0])
                        .unwrap();
                    t.check(
                        &format!("{what}: sort_heap"),
                        SORT_HEAP,
                        Conv::Cdecl,
                        &[first, last, pred],
                        |g| {
                            sort::bridge::sort_heap(g, ta, first, last, pred);
                            Out::None
                        },
                    );
                }
                7 if n >= 1 => {
                    let val = t.alloc(&entries(&mut rng, 1, false));
                    let hole = rng.below(n as u32) as i32;
                    let top = rng.below(hole as u32 + 1) as i32;
                    t.check(
                        &format!("{what}: push_heap {hole} {top}"),
                        PUSH_HEAP,
                        Conv::Cdecl,
                        &[first, hole as u32, top as u32, val, pred],
                        |g| {
                            sort::bridge::push_heap(g, first, hole, top, val, pred);
                            Out::None
                        },
                    );
                }
                8 if n >= 1 => {
                    let val = t.alloc(&entries(&mut rng, 1, false));
                    let bottom = 1 + rng.below(n as u32) as i32;
                    let hole = rng.below(bottom as u32) as i32;
                    t.check(
                        &format!("{what}: adjust_heap {hole} {bottom}"),
                        ADJUST_HEAP,
                        Conv::Cdecl,
                        &[first, hole as u32, bottom as u32, val, pred],
                        |g| {
                            sort::bridge::adjust_heap(g, first, hole, bottom, val, pred);
                            Out::None
                        },
                    );
                }
                _ => {}
            }
            t.a.reset_heap();
            t.b.reset_heap();
            let log = t.zeros(0x8000);
            t.log = log;
        }
    }
}

// ---- PVisibilityCheck and the culling job ----

const VIS_INIT: u32 = 0x009f_2580;
const EXECUTE: u32 = 0x009f_3160;
const MERGE: u32 = 0x009f_3660;

/// A 3x4 world matrix of modest values.
fn matrix3x4(t: &mut Twins, rng: &mut Rng) -> u32 {
    let m = t.zeros(48);
    for i in 0..12 {
        let v = if rng.chance(8) {
            rng.float()
        } else {
            (rng.below(400) as f32 - 200.0) / 100.0
        };
        t.wf(m + 4 * i, v);
    }
    m
}

#[test]
fn visibility_check_init_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1234_5678);
        for i in 0..300 {
            let camera = t.zeros(0x100);
            for k in 0..48 {
                let v = rng.float();
                t.wf(camera + 4 * k, v);
            }
            let this = t.zeros(0x40);
            let pass = match rng.below(3) {
                0 => 0,
                1 => u32::MAX,
                _ => rng.next(),
            };
            let frame = tmp(&t.b);
            t.check(
                &format!("{name} init {i}"),
                VIS_INIT,
                Conv::Thiscall(this),
                &[camera, pass],
                |g| {
                    Out::U32(visibility::bridge::init(
                        g,
                        frame,
                        this,
                        camera,
                        pass as i32,
                    ))
                },
            );
        }
    }
}

/// A job over `n` bounds (stride 0x30), with a view-projection matrix,
/// a depth-bias table, entries space; returns the job.
fn job(t: &mut Twins, rng: &mut Rng, n: u32, culler: u32) -> u32 {
    let stride = 0x30 + 4 * rng.below(3);
    let bounds = t.zeros((n * stride + 4) as usize);
    let mut all = Vec::new();
    for i in 0..n {
        let b = bounds + i * stride;
        for k in 0..3 {
            let lo = rng.coord();
            t.wf(b + 4 * k, lo);
            let size = if rng.chance(20) {
                rng.float()
            } else {
                rng.below(500) as f32 / 7.0
            };
            t.wf(b + 0x10 + 4 * k, size);
        }
        let world = if rng.chance(3) { 0 } else { matrix3x4(t, rng) };
        t.w32(b + 0xc, world);
        let inst = if rng.chance(8) { 0 } else { 0x2000_0000 + i };
        t.w32(b + 0x1c, inst);
        t.w32(b + 0x20, rng.order() as u32);
        all.push(b);
    }
    // a view-projection that keeps about half the boxes
    let vp = t.zeros(64);
    let scale = 1.0 / (50.0 + rng.below(400) as f32);
    let m: [f32; 16] = [
        scale,
        0.0,
        0.0,
        0.0, //
        0.0,
        scale,
        0.0,
        0.0, //
        0.0,
        0.0,
        scale * 0.5,
        0.0, //
        rng.coord() * scale,
        rng.coord() * scale,
        0.5,
        1.0,
    ];
    for (k, v) in m.iter().enumerate() {
        t.wf(vp + 4 * k as u32, *v);
    }
    let j = t.zeros(0x50);
    t.w32(j + 0x10, bounds);
    t.w32(j + 0x14, n);
    t.w32(j + 0x18, stride);
    for k in 0..4 {
        let v = if rng.chance(30) {
            rng.float()
        } else {
            (rng.below(200) as f32 - 100.0) / 64.0
        };
        t.wf(j + 0x1c + 4 * k, v);
    }
    let entries = t.zeros((n * 12 + 12) as usize);
    t.w32(j + 0x30, entries);
    t.w32(j + 0x2c, rng.next());
    let nb = if rng.chance(2) { rng.below(6) } else { 0 };
    let table = t.zeros((nb * 12 + 4) as usize);
    for k in 0..nb {
        let e = table + 12 * k;
        let target = if all.is_empty() || rng.chance(4) {
            rng.next()
        } else {
            all[rng.below(all.len() as u32) as usize]
        };
        t.w32(e, target);
        let s = (rng.below(300) as f32) / 100.0;
        t.wf(e + 4, s);
        let b = rng.coord();
        t.wf(e + 8, b);
    }
    t.w32(j + 0x34, table);
    t.w32(j + 0x38, nb);
    t.w32(j + 0x3c, culler);
    t.w32(j + 0x44, vp);
    j
}

#[test]
fn culling_job_matches_the_original() {
    const SECOND: u32 = visibility::SECOND_TEST;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0001);
        for round in 0..160 {
            t.reset();
            // the second test: stubbed (it needs a frustum object), keeping
            // all or none; or no second test
            let culler = match rng.below(3) {
                0 => 0,
                k => {
                    t.stub(SECOND, true, 3, 3, u32::from(k == 1));
                    0x0dd0_0000 + round
                }
            };
            let n = match rng.below(4) {
                0 => rng.below(4),
                1 | 2 => rng.below(40),
                _ => 40 + rng.below(300),
            };
            let j = job(&mut t, &mut rng, n, culler);
            // the original's frame (ebp - 0x200) for 3 arguments, a return
            // address and the saved ebp; the port's at the same place
            let top = call_top(&t.a);
            let frame = top - 20 - 0x200;
            t.b.set_call_top(Some(frame - 0x100));
            t.check(
                &format!("{name} job {round} (n {n}, culler {culler:#x})"),
                EXECUTE,
                Conv::Thiscall(j),
                &[0, 0, 0],
                |g| {
                    visibility::bridge::execute(g, frame, j);
                    Out::None
                },
            );
            t.b.set_call_top(None);
            if culler != 0 {
                t.unstub(SECOND);
            }
        }
    }
}

// ---- mergeResults ----

/// `jobs` sorted entry lists linked from `check +0x18` through `+0x40`,
/// and a done list at `+0x1c`; returns `(check, total entries)`.
fn merge_input(t: &mut Twins, rng: &mut Rng) -> (u32, u32) {
    let check = t.zeros(0x40);
    let jobs = rng.below(6);
    let mut link = check + 0x18;
    let mut total = 0;
    for _ in 0..jobs {
        let n = 1 + rng.below(30);
        let job = t.zeros(0x50);
        let e = t.zeros((12 * n) as usize);
        // keys in each job's order (the merge reads heads only)
        let mut keys: Vec<(bool, f32)> = (0..n)
            .map(|_| {
                let k = if rng.chance(10) {
                    rng.float()
                } else {
                    rng.below(50) as f32
                };
                (rng.chance(3), k)
            })
            .collect();
        keys.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
        for (i, (class, k)) in keys.iter().enumerate() {
            let p = e + 12 * i as u32;
            t.w32(p, 0x3000_0000 + total + i as u32);
            t.wf(p + 4, *k);
            let order = if *class { sort::CLASS + 3 } else { 7 };
            t.w32(p + 8, order as u32);
        }
        t.w32(job + 0x30, e);
        t.w32(job + 0x2c, e + 12 * n);
        let block = if rng.chance(2) { t.zeros(8) } else { 0 };
        if block != 0 {
            t.w32(block + 4, 1);
        }
        t.w32(job + 0xc, block);
        t.w32(link, job);
        link = job + 0x40;
        total += n;
    }
    // the done list: nodes [next, block]
    let mut next = 0;
    for _ in 0..rng.below(4) {
        let node = t.zeros(8);
        let block = t.zeros(8);
        t.w32(block + 4, 1);
        t.w32(node, next);
        t.w32(node + 4, block);
        next = node;
    }
    t.w32(check + 0x1c, next);
    (check, total)
}

#[test]
fn merge_results_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f0f_1234);
        for round in 0..400 {
            let (check, total) = merge_input(&mut t, &mut rng);
            let dst = t.zeros((4 * total + 4) as usize);
            let range = t.zeros(8);
            t.w32(range, dst);
            t.w32(range + 4, dst + 4 * total);
            let out = t.zeros(8);
            t.check(
                &format!("{name} merge {round} ({total} entries)"),
                MERGE,
                Conv::Thiscall(check),
                &[out, range],
                |g| Out::U32(world::bridge::merge(g, check, out, range)),
            );
        }
    }
}

// ---- renderWorldCulledSorted and renderWorld ----

/// Recording stubs over every callee `renderWorldCulledSorted` does not
/// port; `jobs` is what the job wait returns, `ring` the block the ring
/// allocator hands out.
fn stub_culled_sorted(t: &mut Twins, jobs: u32, ring: u32) {
    t.stub(world::BOUNDS_CURSOR, true, 1, 1, 0);
    t.stub(world::OCCLUDER_CURSOR, true, 1, 1, 0);
    t.stub(world::START_OCCLUDERS, true, 1, 1, 0);
    t.stub(world::BUILD_OCCLUDERS, true, 0, 0, 0);
    t.stub(world::START_JOBS, true, 1, 1, 0);
    t.stub(world::WAIT_JOBS, true, 0, 0, jobs);
    t.stub(world::RING_SINGLETON, false, 0, 0, 0x0aa0_0000);
    t.stub(world::RING_ALLOCATE, true, 1, 1, ring);
    t.stub(world::RENDER_LIST, true, 3, 3, 0);
    t.stub(world::FREE_JOBS, true, 0, 0, 0);
}

#[test]
fn render_world_culled_sorted_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_0001);
        for round in 0..60 {
            t.reset();
            let jobs = match rng.below(4) {
                0 => 0,
                1 => 1,
                _ => 1 + rng.below(200),
            };
            let ring = t.zeros(0x10 + 4 * 200 + 8);
            t.w32(ring + 4, 1);
            stub_culled_sorted(&mut t, jobs, ring);
            let camera = t.zeros(0x100);
            for k in 0..48 {
                let v = rng.float();
                t.wf(camera + 4 * k, v);
            }
            let renderer = 0x0bbb_0000 + round;
            let world = 0x0ccc_0000 + round;
            let ctx = 0x0ddd_0000 + round;
            let pass = if rng.chance(2) { 0 } else { u32::MAX };
            // the original's frame (ebp - 0x9c) for 4 arguments, a return
            // address and the saved ebp; the port's at the same place, its
            // calls below it
            let top = call_top(&t.a);
            let frame = top - 24 - 0x9c;
            t.b.set_call_top(Some(frame - 0x100));
            let args = [world, camera, ctx, pass];
            t.check(
                &format!("{name} culled sorted {round} ({jobs} jobs)"),
                world::CULLED_SORTED,
                Conv::Thiscall(renderer),
                &args,
                |g| {
                    Out::U32(world::bridge::culled_sorted(
                        g, frame, renderer, world, camera, ctx, pass,
                    ))
                },
            );
            t.b.set_call_top(None);
            let n = t.log_entries();
            assert_eq!(n, if jobs == 0 { 7 } else { 10 }, "{name}: calls logged");
        }
    }
}

const RENDER_WORLD: u32 = 0x0062_be70;

#[test]
fn render_world_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_0002);
        t.stub(world::CULLED_SORTED, true, 4, 4, 0);
        for round in 0..50 {
            let app = rng.next();
            let args = [rng.next(), rng.next(), rng.next()];
            t.check(
                &format!("{name} render world {round}"),
                RENDER_WORLD,
                Conv::Thiscall(app),
                &args,
                |g| {
                    Out::U32(world::bridge::render_world(
                        g, app, args[0], args[1], args[2],
                    ))
                },
            );
        }
        assert_eq!(t.log_entries(), 50);
    }
}

// ---- the render thread: the mesh-command handlers and the state bind ----

const RUN_MESH: u32 = 0x0061_44f0;
const RUN_MESH2: u32 = 0x0061_4610;
const BIND_STATE: u32 = 0x0058_c6c0;

/// A fake COM object: a vtable of 64 words, the given slots recording
/// stubs (`stdcall`, `this` first on the stack: `n` words logged and
/// popped). Returns the object.
fn com_object(t: &mut Twins, tag: u32, slots: &[(u32, u32)]) -> u32 {
    let vtable = t.zeros(4 * 64);
    for &(slot, n) in slots {
        let s = t.stub_code(tag + slot, false, n, n, 0);
        t.w32(vtable + 4 * slot, s);
    }
    let obj = t.zeros(16);
    t.w32(obj, vtable);
    obj
}

#[test]
fn pass_state_bind_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0d3d_0011);
        for round in 0..200 {
            t.reset();
            // RSSetState (43), OMSetBlendState (35), OMSetDepthStencilState (36)
            let d3d = com_object(&mut t, 0xd3d0_0000, &[(43, 2), (35, 4), (36, 3)]);
            let ctx = t.zeros(0x600);
            t.w32(ctx + 0x70, d3d);
            t.w32(ctx + 0x574, rng.below(4));
            let state = t.zeros(0x180);
            for off in [0x168, 0x16c, 0x170, 0x174] {
                let v = if rng.chance(3) { 0 } else { rng.next() };
                t.w32(state + off, v);
            }
            t.write(state + 0x178, &[rng.next() as u8]);
            // the original's frame: one argument, the return address, ebp;
            // the blend factor at ebp - 0x14
            let top = call_top(&t.a);
            let frame = top - 12 - 0x14;
            t.b.set_call_top(Some(frame - 0x100));
            t.check(
                &format!("{name} bind {round}"),
                BIND_STATE,
                Conv::Thiscall(state),
                &[ctx],
                |g| Out::U32(thread::bridge::bind_pass_state(g, frame, state, ctx)),
            );
            t.b.set_call_top(None);
            // the blend factor the call got (both frames at one address)
            if Mem::read::<u32>(&t.b, state + 0x174) != 0 {
                assert_eq!(
                    Oracle::read(&mut t.a, frame, 16).unwrap(),
                    Oracle::read(&mut t.b, frame, 16).unwrap(),
                    "{name} bind {round}: blend factor"
                );
            }
        }
    }
}

/// A command for the handlers: a dynamic mesh (or not), a segment, an
/// index stream and the words they read.
fn mesh_command(t: &mut Twins, rng: &mut Rng, second: bool) -> u32 {
    let cmd = t.zeros(0x48);
    for off in (0..0x48).step_by(4) {
        let v = rng.next();
        t.w32(cmd + off, v);
    }
    let dynamic = if rng.chance(3) {
        let slot = if second { 4 } else { 3 };
        let n = if second { 7 } else { 6 };
        let vtable = t.zeros(4 * 8);
        // the log holds six words: the second handler skips the context
        // (the same in every call) to log its seventh (+0x3c)
        let s = t.stub_code_from(0x00d1_0000 + slot, true, n - 6, 6, n, 0);
        t.w32(vtable + 4 * slot, s);
        let obj = t.zeros(8);
        t.w32(obj, vtable);
        obj
    } else {
        0
    };
    t.w32(cmd + 4, dynamic);
    let segment = t.zeros(0x10);
    t.w32(segment + 0xc, u32::from(rng.chance(3)));
    t.w32(cmd + 0x14, segment);
    let stream = t.zeros(0x14);
    let v = rng.next();
    t.w32(stream + 0x10, v);
    t.w32(cmd + 0x18, stream);
    let start = if rng.chance(2) { 0 } else { rng.next() };
    t.w32(cmd + 0x34, start);
    let count = match rng.below(3) {
        0 => rng.below(130),
        1 => 64 * rng.below(5),
        _ => rng.next(),
    };
    t.w32(cmd + 0x38, count);
    cmd
}

#[test]
fn mesh_command_handlers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0d3d_0022);
        for round in 0..300 {
            t.reset();
            t.stub(thread::DATA_BLOCK_COUNT, true, 1, 1, 0x40 + rng.below(4));
            t.stub(thread::BLOCK_LIST_DTOR, true, 0, 0, 0);
            t.stub(thread::DRAW_INDIRECT, true, 3, 3, 0);
            t.stub(thread::DRAW_INDEXED, true, 3, 3, 0);
            t.stub(thread::DRAW_INDEXED_INSTANCED, true, 4, 4, 0);
            t.stub(thread::DRAW_INDEXED_2, true, 4, 4, 0);
            t.stub(thread::DRAW_INDEXED_INSTANCED_2, true, 5, 5, 0);
            let second = rng.chance(2);
            let cmd = mesh_command(&mut t, &mut rng, second);
            let ctx = 0x00c9_4f00;
            // the original's frame: two arguments, the return address, ebp;
            // the block list at ebp - 0x20
            let top = call_top(&t.a);
            let frame = top - 16 - 0x20;
            t.b.set_call_top(Some(frame - 0x100));
            let (sym, what) = if second {
                (RUN_MESH2, "second")
            } else {
                (RUN_MESH, "first")
            };
            t.check(
                &format!("{name} {what} handler {round}"),
                sym,
                Conv::Cdecl,
                &[ctx, cmd],
                |g| {
                    if second {
                        thread::bridge::run_mesh_command2(g, frame, ctx, cmd);
                    } else {
                        thread::bridge::run_mesh_command(g, frame, ctx, cmd);
                    }
                    Out::None
                },
            );
            t.b.set_call_top(None);
            // the block list the dynamic draw got (both frames at one address)
            assert_eq!(
                Oracle::read(&mut t.a, frame, 8).unwrap(),
                Oracle::read(&mut t.b, frame, 8).unwrap(),
                "{name} {what} handler {round}: block list"
            );
            assert!(t.log_entries() >= 1, "{name}: a draw logged");
        }
    }
}

// ---- selectTechniquePass, the technique lookup, the parameter buffers ----

const SELECT_PASS: u32 = 0x005b_a690;
const FIND_TECHNIQUE: u32 = 0x004e_71d0;
const FILL_BUFFERS: u32 = 0x005b_5a50;

/// An effect with `n` techniques (40-byte entries keyed by pass
/// pointers) and an id table; returns `(effect, passes)`.
fn effect(t: &mut Twins, rng: &mut Rng, n: u32) -> (u32, Vec<u32>) {
    let passes: Vec<u32> = (0..4)
        .map(|i| {
            let p = t.zeros(0x10);
            let id = if rng.chance(4) {
                u32::MAX
            } else {
                rng.below(6)
            };
            t.w32(p + 0xc, id);
            let _ = i;
            p
        })
        .collect();
    let techs = t.zeros((40 * n + 4) as usize);
    for i in 0..n {
        let e = techs + 40 * i;
        let key = if rng.chance(4) {
            rng.next()
        } else {
            passes[rng.below(4) as usize]
        };
        t.w32(e, key);
        let base = if rng.chance(5) { 0 } else { rng.next() };
        t.w32(e + 8, base);
    }
    let fx = t.zeros(0x20);
    t.w32(fx, rng.next());
    let count = n | if rng.chance(2) { 0x8000_0000 } else { 0 };
    t.w32(fx + 0xc, count);
    t.w32(fx + 0x10, techs);
    let ids = rng.below(6);
    // entries past the count too: an id at or past it must not be read
    let table = t.zeros(4 * 8);
    for i in 0..8 {
        let v = if rng.chance(3) {
            0
        } else {
            techs + 40 * rng.below(n.max(1))
        };
        t.w32(table + 4 * i, v);
    }
    t.w32(fx + 0x14, ids | if rng.chance(3) { 0x8000_0000 } else { 0 });
    t.w32(fx + 0x18, table);
    (fx, passes)
}

#[test]
fn find_technique_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7ec4_0001);
        for round in 0..300 {
            t.reset();
            let n = rng.below(8);
            let (fx, passes) = effect(&mut t, &mut rng, n);
            let pass = if rng.chance(4) {
                rng.next()
            } else {
                passes[rng.below(4) as usize]
            };
            t.check(
                &format!("{name} find technique {round}"),
                FIND_TECHNIQUE,
                Conv::Thiscall(fx),
                &[pass],
                |g| Out::U32(ffx_prender::mesh::bridge::find_technique(g, fx, pass)),
            );
        }
    }
}

#[test]
fn select_technique_pass_matches_the_original() {
    use ffx_prender::mesh;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7ec4_0002);
        for round in 0..400 {
            t.reset();
            let variant = match rng.below(3) {
                0 => u32::MAX,
                1 => 0,
                _ => rng.below(5),
            };
            t.stub(mesh::FIND_CONTEXT_VARIANT, true, 5, 5, variant);
            let stats = if rng.chance(2) {
                t.stub(mesh::STATS_COUNTER, true, 2, 2, rng.next() & 0x0fff_fff0);
                let add = t.stub_code(0x1a7e_0000, false, 2, 2, 0);
                let slot = t.rebase(mesh::INTERLOCKED_ADD);
                t.w32(slot, add);
                0x0555_0000
            } else {
                0
            };
            let g = t.rebase(mesh::EFFECT_STATS);
            t.w32(g, stats);
            let n = rng.below(6);
            let (fx, passes) = effect(&mut t, &mut rng, n);
            let material = t.zeros(0x10);
            let pass = match rng.below(5) {
                0 => 0,
                // a pass no technique names (its id read too)
                1 => t.zeros(0x10),
                _ => passes[rng.below(4) as usize],
            };
            if rng.chance(2) {
                t.w32(material + 8, pass);
                let to = if rng.chance(4) {
                    0
                } else {
                    passes[rng.below(4) as usize]
                };
                t.w32(material + 0xc, to);
            }
            let segment = t.zeros(0x20);
            if rng.chance(3) {
                t.w32(segment + 0x18, fx);
                t.w32(segment + 0x14, rng.below(4));
            }
            let renderer = t.zeros(0x20);
            let v = rng.next();
            t.w32(renderer + 0x18, v);
            let out = t.zeros(8);
            let (a, b, node) = (rng.next(), rng.next(), rng.next());
            let args = [out, a, b, fx, material, pass, node, segment];
            t.check(
                &format!("{name} select pass {round}"),
                SELECT_PASS,
                Conv::Thiscall(renderer),
                &args,
                |g| {
                    Out::U32(mesh::bridge::select_technique_pass(
                        g, renderer, out, a, b, fx, material, pass, node, segment,
                    ))
                },
            );
        }
    }
}

#[test]
fn parameter_buffers_match_the_original() {
    use ffx_prender::mesh;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7ec4_0003);
        for round in 0..400 {
            t.reset();
            let r = |rng: &mut Rng| if rng.chance(3) { 0 } else { rng.next() };
            t.stub(mesh::SCENE_WIDE_BUFFER, true, 3, 3, r(&mut rng));
            t.stub(mesh::MATERIAL_BUFFER, true, 1, 1, r(&mut rng));
            t.stub(mesh::NODE_BUFFER, true, 1, 1, r(&mut rng));
            t.stub(mesh::CONTEXT_BUFFER, true, 3, 3, r(&mut rng));
            t.stub(mesh::DYNAMIC_STREAMS, true, 6, 6, r(&mut rng));
            t.stub(mesh::STREAMS, true, 4, 4, if rng.chance(3) { 5 } else { 0 });
            let block = t.zeros(0x20);
            t.w32(block + 4, 1);
            t.stub(world::RING_SINGLETON, false, 0, 0, 0x0aa0_0000);
            t.stub(world::RING_ALLOCATE, true, 1, 1, block);
            let dynamic = if rng.chance(2) {
                let vtable = t.zeros(16);
                let size = if rng.chance(3) { 0 } else { 0x40 };
                let s1 = t.stub_code(0x00d7_0001, true, 1, 1, size);
                let s2 = t.stub_code(0x00d7_0002, true, 2, 2, 0);
                t.w32(vtable + 4, s1);
                t.w32(vtable + 8, s2);
                let obj = t.zeros(8);
                t.w32(obj, vtable);
                obj
            } else {
                0
            };
            let renderer = t.zeros(0x60);
            for off in [0x18, 0x54, 0x58, 0x5c] {
                let v = r(&mut rng);
                t.w32(renderer + off, v);
            }
            let out = t.zeros(0x20);
            for k in 0..8 {
                let v = rng.next();
                t.w32(out + 4 * k, v);
            }
            let flags = rng.below(16);
            let (m, nd, a, b, c) = (rng.next(), rng.next(), rng.next(), rng.next(), rng.next());
            let args = [out, flags, m, nd, a, b, c, dynamic];
            t.check(
                &format!("{name} buffers {round} (flags {flags})"),
                FILL_BUFFERS,
                Conv::Thiscall(renderer),
                &args,
                |g| {
                    Out::U32(mesh::bridge::allocate_and_fill(
                        g, renderer, out, flags, m, nd, a, b, c, dynamic,
                    ))
                },
            );
        }
    }
}
