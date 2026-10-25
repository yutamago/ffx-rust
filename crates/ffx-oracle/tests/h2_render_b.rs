//! M4 port round 1, list h2-render-b (`ffx_prender::descriptor`,
//! `ffx_prender::misc`): every port against its original, on the image
//! alone and on a snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! Each case (`h2_render_b/cases.rs`, one per port, the same call as its
//! shim) runs on random state: a pool of random words (no signalling
//! NaNs, which the emulator keeps and the hardware quiets), arguments by
//! kind (pointers into the pool, null, values, floats, counts), and the
//! direct callees replaced in both oracles by recording stubs: each call
//! logs a tag, `ecx` and its stack words, pops what the original callee
//! pops and returns a per-case random word (`cell`). A case's `prep` fixes
//! what random words cannot be (handlers called through pointers, lists,
//! computed addresses). Results (`eax`, `al`) and every byte written,
//! the logs included, must agree, and the original must leave `ST0`
//! empty.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::should_implement_trait,
    clippy::must_use_candidate,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unreadable_literal,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

#[path = "h2_render_b/cases.rs"]
mod cases;

use ffx_prender::descriptor::{Init, bridge as d};
use ffx_prender::misc::bridge as x;

/// Calling conventions of the cases.
#[derive(Debug, Clone, Copy)]
pub enum K {
    C,
    S,
    T,
}

/// What a case compares of the result.
#[derive(Debug, Clone, Copy)]
pub enum O {
    None,
    U32,
    U8,
}

/// One port.
pub struct Case {
    pub name: &'static str,
    pub addr: u32,
    pub conv: K,
    /// One per argument (`this` first for thiscall): `p` pointer, `n`
    /// nullable pointer, `v` value, `f` float, `c` small count, `b` flags,
    /// `i` small index.
    pub kinds: &'static str,
    pub out: O,
    /// (address, stack words, words popped, logs `ecx`).
    pub callees: &'static [(u32, u32, u32, bool)],
    pub prep: fn(&mut Env, &mut Rng, &mut [u32]),
    pub port: fn(&mut G, &[u32]) -> u32,
}

/// Deterministic xorshift stream.
pub struct Rng(u32);

impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    pub fn chance(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
}

/// A word that is not a signalling NaN as a float.
fn quiet(w: u32) -> u32 {
    if w & 0x7f80_0000 == 0x7f80_0000 && w & 0x007f_ffff != 0 {
        w | 0x0040_0000
    } else {
        w
    }
}

/// The bridge's view of an oracle.
pub struct G<'a>(&'a mut Oracle);

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

/// The pool's size and the part pointers point into.
const POOL: u32 = 0x4000;
const POOL_PTRS: u32 = 0x2000;
/// The call log: a count, then 8-word entries (tag, `ecx`, six words).
const LOG: u32 = 0x2000;

/// Both oracles and the test's memory in them.
pub struct Env {
    a: Oracle,
    b: Oracle,
    pub pool: u32,
    log: u32,
    /// Stub cells by callee (or pointer-handler) tag.
    cells: HashMap<u32, u32>,
    /// Handler stubs for pointers, by words popped: (code, cell).
    handlers: [(u32, u32); 4],
    stubbed: Vec<(u32, Vec<u8>)>,
}

impl Env {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut e = Self {
            a,
            b,
            pool: 0,
            log: 0,
            cells: HashMap::new(),
            handlers: [(0, 0); 4],
            stubbed: Vec::new(),
        };
        e.pool = e.zeros(POOL as usize);
        e.log = e.zeros(LOG as usize * 4);
        for pops in 0..4 {
            e.handlers[pops as usize] = e.stub_code(0xf00d_0000 + pops, pops, pops, true);
        }
        e
    }

    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
            eprintln!("FFX_GAME_DIR is not set; skipping");
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

    pub fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn zeros(&mut self, n: usize) -> u32 {
        let bytes = vec![0; n];
        let x = self.a.alloc_bytes(&bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(&bytes).unwrap(), x);
        x
    }

    pub fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    pub fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    pub fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.a, at)
    }

    /// A pointer into the pool's first part, word-aligned.
    pub fn ptr(&self, rng: &mut Rng) -> u32 {
        self.pool + 4 * rng.below(POOL_PTRS / 4)
    }

    /// A recording stub: logs `tag`, `ecx` (when `this`), `n` stack words; pops `pops`;
    /// returns its cell. Returns (code, cell).
    fn stub_code(&mut self, tag: u32, n: u32, pops: u32, this: bool) -> (u32, u32) {
        let cell = self.zeros(4);
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
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        self.cells.insert(tag, cell);
        (code, cell)
    }

    /// Replaces the original at `sym` by a recording stub (a `jmp`).
    fn stub(&mut self, sym: u32, n: u32, pops: u32, this: bool) {
        let at = self.rebase(sym);
        let (code, _) = self.stub_code(sym, n, pops, this);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.stubbed.push((at, old));
        self.write(at, &j);
    }

    fn unstub_all(&mut self) {
        while let Some((at, old)) = self.stubbed.pop() {
            self.write(at, &old);
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// The stub cell of callee `sym` (its return value).
    pub fn set_ret(&mut self, sym: u32, v: u32) {
        let c = self.cells[&sym];
        self.w32(c, v);
    }

    /// A handler stub for a pointer field that pops `pops` words, its
    /// return value `ret`.
    pub fn handler(&mut self, pops: u32, ret: u32) -> u32 {
        let (code, cell) = self.handlers[pops as usize];
        self.w32(cell, ret);
        code
    }

    /// A random word for a stub's or handler's result.
    pub fn ret_word(&self, rng: &mut Rng) -> u32 {
        match rng.below(4) {
            0 => 0,
            1 => 4,
            2 => self.ptr(rng),
            _ => rng.next(),
        }
    }

    fn fill(&mut self, rng: &mut Rng) {
        let mut bytes = Vec::with_capacity(POOL as usize);
        for _ in 0..POOL / 4 {
            let w = match rng.below(5) {
                0 => 0,
                1 => self.ptr(rng),
                2 => rng.below(0x100),
                3 => rng.next() | 0x8000_0000,
                _ => rng.next(),
            };
            bytes.extend(quiet(w).to_le_bytes());
        }
        let pool = self.pool;
        self.write(pool, &bytes);
        let log = self.log;
        self.w32(log, 0);
        let cells: Vec<u32> = self.cells.values().copied().collect();
        for c in cells {
            let v = self.ret_word(rng);
            self.w32(c, v);
        }
    }

    fn arg(&self, rng: &mut Rng, kind: char) -> u32 {
        match kind {
            'p' => self.ptr(rng),
            'n' => {
                if rng.chance(4) {
                    0
                } else {
                    self.ptr(rng)
                }
            }
            'f' => quiet(match rng.below(4) {
                0 => rng.next(),
                1 => ((rng.below(4000) as f32 - 2000.0) / 7.0).to_bits(),
                2 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 1]
                    [rng.below(6) as usize],
                _ => (rng.below(100) as f32).to_bits(),
            }),
            'c' => rng.below(9),
            'b' => rng.below(4),
            'i' => rng.below(8),
            _ => match rng.below(4) {
                0 => 0,
                1 => rng.below(0x100),
                2 => rng.next() | 0x8000_0000,
                _ => rng.next(),
            },
        }
    }
}

/// No fix-ups.
pub fn no_prep(_: &mut Env, _: &mut Rng, _: &mut [u32]) {}

/// `p - 4` holds an object whose first virtual method is a handler that
/// pops one word.
pub fn prep_virtual_dtor(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let p = e.ptr(rng) + 4;
    a[1] = p;
    let vt = e.ptr(rng);
    let r = e.ret_word(rng);
    let h = e.handler(1, r);
    e.w32(p - 4, vt);
    e.w32(vt, h);
}

/// The array member's handlers: 0, or stubs popping what the callers
/// expect (`+0x3c` two words, `+0x38` and `+0x34` none).
pub fn prep_member(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let m = a[0];
    for (off, pops) in [(0x34, 0), (0x38, 0), (0x3c, 2)] {
        let v = if rng.chance(2) {
            0
        } else {
            let r = e.ret_word(rng);
            e.handler(pops, r)
        };
        e.w32(m + off, v);
    }
}

/// [`prep_member`], and a field offset that lands in the pool:
/// `[member + 0x24] - [[member + 0xc] + 0x88] + i`.
pub fn prep_member_element(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    prep_member(e, rng, a);
    let m = a[0];
    let class = e.ptr(rng);
    let base = rng.next();
    e.w32(m + 0xc, class);
    e.w32(class + 0x88, base);
    let target = e.ptr(rng);
    let i = 4 * rng.below(0x40);
    e.w32(m + 0x24, base.wrapping_add(target).wrapping_sub(i));
    a[1] = i;
}

/// A list of render pass types in the pool (or none) at their global.
pub fn prep_pass_types(e: &mut Env, rng: &mut Rng, _: &mut [u32]) {
    let g = e.rebase(0x00ca_54f8);
    if rng.chance(3) {
        e.w32(g, 0);
        return;
    }
    let n = 1 + rng.below(6);
    let nodes: Vec<u32> = (0..n).map(|i| e.pool + POOL_PTRS + 0x40 * i).collect();
    for (k, &node) in nodes.iter().enumerate() {
        let next = nodes.get(k + 1).copied().unwrap_or(0);
        e.w32(node + 4, next);
    }
    e.w32(g, nodes[0]);
}

/// A float that is often a quiet NaN (the x87 rule for two NaNs: the
/// larger significand wins, where SSE takes the first operand).
fn nanish(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 => [0x7fc0_0000, 0xffc0_1234, 0x7fd0_0000, 0xfff0_0001][rng.below(4) as usize],
        1 => 0x7fc0_0000 | rng.below(0x40_0000) | (rng.below(2) << 31),
        _ => ((rng.below(4000) as f32 - 2000.0) / 7.0).to_bits(),
    }
}

/// The vector and the factor of `0x004f30d0`, NaNs among them.
pub fn prep_scale(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    for i in 0..4 {
        let f = nanish(rng);
        e.w32(a[0] + 4 * i, f);
    }
    a[2] = nanish(rng);
}

/// The element getter's result points into the pool; the count word is
/// random around the index.
pub fn prep_element_field(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let p = e.ptr(rng);
    e.set_ret(0x0058_5c90, p);
    let n = rng.below(10) | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + 0x24, n);
}

fn run(case: &Case, e: &mut Env, rng: &mut Rng, what: &str) {
    e.fill(rng);
    let mut args: Vec<u32> = case.kinds.chars().map(|k| e.arg(rng, k)).collect();
    (case.prep)(e, rng, &mut args);
    let addr = e.rebase(case.addr);
    let conv = match case.conv {
        K::C => Conv::Cdecl,
        K::S => Conv::Stdcall,
        K::T => Conv::Thiscall(args[0]),
    };
    let stack = match case.conv {
        K::T => &args[1..],
        _ => &args[..],
    };
    let ra =
        e.a.call(addr, conv, stack)
            .unwrap_or_else(|err| panic!("{what}: the original: {err}"));
    assert!(
        ra.st0_raw.is_none(),
        "{what}: the original leaves ST0 set ({:?})",
        ra.st0
    );
    let rb = (case.port)(&mut G(&mut e.b), &args);
    match case.out {
        O::None => {}
        O::U32 => assert_eq!(ra.eax, rb, "{what}: eax ({args:x?})"),
        O::U8 => assert_eq!(ra.eax & 0xff, rb, "{what}: al ({args:x?})"),
    }
    let diff = e.a.diff(&mut e.b).unwrap();
    assert!(
        diff.is_empty(),
        "{what}: memory differs at {} bytes, first {:x?} ({args:x?})",
        diff.len(),
        &diff[..diff.len().min(8)]
    );
    e.a.clear_dirty();
    e.b.clear_dirty();
}

/// Runs every case whose name contains `filter`'s value (env
/// `H2_FILTER`, default all).
fn run_cases(rounds: u32) {
    let filter = std::env::var("H2_FILTER").unwrap_or_default();
    let mut ran = 0;
    for (snap, mut e) in Env::all() {
        for case in cases::CASES.iter().filter(|c| c.name.contains(&filter)) {
            for &(sym, n, pops, this) in case.callees {
                e.stub(sym, n, pops, this);
            }
            let mut rng = Rng(0x1234_5678 ^ case.addr);
            for i in 0..rounds {
                run(
                    case,
                    &mut e,
                    &mut rng,
                    &format!("{snap} {} {:#x} #{i}", case.name, case.addr),
                );
            }
            e.unstub_all();
            ran += 1;
        }
    }
    eprintln!("{ran} case runs");
}

#[test]
fn h2_render_b_ports_match_the_original() {
    run_cases(300);
}

#[test]
fn h2_render_b_cases_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for c in cases::CASES {
        assert!(seen.insert(c.addr), "{} twice", c.name);
    }
    let _ = (Init::Word(0, 0), x::map_mode(0));
}
