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

use std::collections::BTreeMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

#[path = "h2_render_b/cases.rs"]
mod cases;
#[path = "h2_render_b/cases3.rs"]
mod cases3;
#[path = "h2_render_b/cases4.rs"]
mod cases4;
#[path = "h2_render_b/cases5.rs"]
mod cases5;
#[path = "h2_render_b/cases6.rs"]
mod cases6;
#[path = "h2_render_b/cases7.rs"]
mod cases7;
#[path = "h2_render_b/cases8.rs"]
mod cases8;

use ffx_prender::array::bridge as a;
use ffx_prender::declare as dc;
use ffx_prender::descriptor::{Init, bridge as d};
use ffx_prender::dtor::bridge as dt;
use ffx_prender::h2_bind::bridge as hb;
use ffx_prender::h2_more::bridge as hm;
use ffx_prender::h2_more2::bridge as h2;
use ffx_prender::h2_more3::bridge as h3;
use ffx_prender::h2_more4::bridge as h4;
use ffx_prender::misc::bridge as x;
use ffx_prender::small::bridge as sm;

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
    /// Bytes of the original's frame below `ebp` that callees see (0:
    /// none): the port gets its frame at the same address
    /// ([`G::frame`]), its calls below it, and both frames must agree
    /// after the call (they start with the same junk pattern, so that a
    /// word the original never writes reads the same in both, and a
    /// missing store shows).
    pub frame: u32,
    /// Words of the frame above it that are not compared (the security
    /// cookie): the frame starts `frame + gap` bytes below `ebp`.
    pub gap: u32,
    /// Callees that store a random word (their cell's second word,
    /// [`Env::set_out`]) through stack argument `k`: `(callee, k)`.
    pub writes: &'static [(u32, u32)],
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
pub struct G<'a>(&'a mut Oracle, u32);

impl G<'_> {
    /// The port's frame: the original's `ebp - frame` in the other oracle.
    pub fn frame(&self) -> u32 {
        self.1
    }
}

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
    cells: BTreeMap<u32, u32>,
    /// Handler stubs for pointers, by words popped: (code, cell).
    handlers: [(u32, u32); 8],
    /// A cdecl handler (no `ecx` logged, two words, pops none): (code, cell).
    handler_c: (u32, u32),
    /// Writer handlers by tag: (code, cell).
    writers: BTreeMap<u32, (u32, u32)>,
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
            cells: BTreeMap::new(),
            handlers: [(0, 0); 8],
            handler_c: (0, 0),
            writers: BTreeMap::new(),
            stubbed: Vec::new(),
        };
        e.pool = e.zeros(POOL as usize);
        e.log = e.zeros(LOG as usize * 4);
        for pops in 0..8 {
            e.handlers[pops as usize] = e.stub_code(0xf00d_0000 + pops, pops, pops, true);
        }
        e.handler_c = e.stub_code(0xf00d_00c0, 2, 0, false);
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
        self.stub_code_w(tag, n, pops, this, None)
    }

    /// [`Env::stub_code`] that also stores the word after its cell
    /// (`cell + 4`) through its stack argument `write` (an out-pointer),
    /// as the callees that fill a caller's local do.
    fn stub_code_w(
        &mut self,
        tag: u32,
        n: u32,
        pops: u32,
        this: bool,
        write: Option<u32>,
    ) -> (u32, u32) {
        let cell = self.zeros(8);
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
        if let Some(k) = write {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * k) as u8]); // mov edx, [esp+4+4k]
            c.extend([0x8b, 0x0d]);
            c.extend((cell + 4).to_le_bytes()); // mov ecx, [cell+4]
            c.extend([0x89, 0x0a]); // mov [edx], ecx
        }
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
    fn stub(&mut self, sym: u32, n: u32, pops: u32, this: bool, write: Option<u32>) {
        let at = self.rebase(sym);
        let (code, _) = self.stub_code_w(sym, n, pops, this, write);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.stubbed.push((at, old));
        self.write(at, &j);
    }

    /// Replaces the original at `sym` by `make(count)`'s code, where
    /// `count` is two fresh zero words the code may count and log in
    /// (callees that must change state between calls: a counter a spin
    /// wait polls, a cursor's next list). Once per case: a later call
    /// for the same callee keeps the first code.
    pub fn custom(&mut self, sym: u32, make: impl FnOnce(u32) -> Vec<u8>) {
        let at = self.rebase(sym);
        if self.stubbed.iter().any(|s| s.0 == at) {
            return;
        }
        let count = self.zeros(8);
        let c = make(count);
        let code = self.a.alloc_code(&c).unwrap();
        assert_eq!(self.b.alloc_code(&c).unwrap(), code);
        let mut j = vec![0xe9];
        j.extend(code.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.stubbed.push((at, old));
        self.write(at, &j);
    }

    /// The stub cell (return word) of callee `sym`.
    pub fn cell(&self, sym: u32) -> u32 {
        self.cells[&sym]
    }

    fn unstub_all(&mut self) {
        while let Some((at, old)) = self.stubbed.pop() {
            self.write(at, &old);
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// What callee `sym`'s stub stores through its out-pointer.
    pub fn set_out(&mut self, sym: u32, v: u32) {
        let c = self.cells[&sym];
        self.w32(c + 4, v);
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

    /// Handler `id`: pops `pops` words, returns `ret` and stores `out`
    /// through its stack argument `k` (made once per id).
    pub fn writer(&mut self, id: u32, pops: u32, k: u32, ret: u32, out: u32) -> u32 {
        let tag = 0xf00d_1000 + id;
        let (code, cell) = if let Some(&w) = self.writers.get(&tag) {
            w
        } else {
            let w = self.stub_code_w(tag, pops, pops, true, Some(k));
            self.writers.insert(tag, w);
            w
        };
        self.w32(cell, ret);
        self.w32(cell + 4, out);
        code
    }

    /// The cdecl handler, its return value `ret`.
    pub fn handler_c(&mut self, ret: u32) -> u32 {
        let (code, cell) = self.handler_c;
        self.w32(cell, ret);
        code
    }

    /// A NUL-terminated string at `at`.
    pub fn string(&mut self, at: u32, s: &[u8]) {
        let mut b = s.to_vec();
        b.push(0);
        self.write(at, &b);
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
        // the out words from their own stream, so that adding them left
        // the earlier cases' random data as it was
        let mut out = Rng(rng.0 ^ 0x5a5a_0001);
        for c in cells {
            let v = self.ret_word(rng);
            self.w32(c, v);
            let o = self.ret_word(&mut out);
            self.w32(c + 4, o);
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
            'w' => rng.below(0x40),
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
    // past the pointers' part, so that no field of the member overlaps it
    let class = e.pool + POOL_PTRS + 0x1800;
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

/// A count word of a few elements, owned or borrowed, and storage.
pub fn prep_small_count(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let n = if rng.chance(8) {
        0x100 + rng.below(3)
    } else {
        rng.below(5)
    };
    let n = n | if rng.chance(3) { 0x8000_0000 } else { 0 };
    let data = if rng.chance(5) { 0 } else { e.ptr(rng) };
    e.w32(a[0], n);
    e.w32(a[0] + 4, data);
}

/// `setStorage`: the old storage sometimes the new one, counts of both
/// signs.
pub fn prep_storage(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    prep_small_count(e, rng, a);
    if rng.chance(4) {
        a[1] = e.r32(a[0] + 4);
    }
    a[2] = rng.below(6) | if rng.chance(3) { 0x8000_0000 } else { 0 };
}

/// The bounded table at `this + 0x18` and its flag.
pub fn prep_bound_element(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let n = rng.below(10) | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + 0x18, n);
    let data = e.ptr(rng);
    e.w32(a[0] + 0x1c, data);
    let flag = rng.below(4);
    e.write(a[0] + 0x34, &[flag as u8]);
}

/// Names for the pass types: short strings, some shared.
const NAMES: [&[u8]; 5] = [b"", b"a", b"ab", b"opaque", b"opaquE"];

/// A list of named pass types; the name looked up is one of them (the
/// same pointer or a copy), another string, or null.
pub fn prep_pass_names(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let strings = e.pool + POOL_PTRS + 0x800;
    for (i, s) in NAMES.iter().enumerate() {
        e.string(strings + 0x20 * i as u32, s);
    }
    prep_pass_types(e, rng, a);
    let g = e.rebase(0x00ca_54f8);
    let mut t = e.r32(g);
    while t != 0 {
        let k = rng.below(NAMES.len() as u32);
        e.w32(t, strings + 0x20 * k);
        t = e.r32(t + 4);
    }
    let copy = strings + 0x200;
    let k = rng.below(NAMES.len() as u32) as usize;
    e.string(copy, NAMES[k]);
    a[0] = match rng.below(4) {
        0 => 0,
        1 => copy,
        _ => strings + 0x20 * rng.below(NAMES.len() as u32),
    };
}

/// [`prep_pass_names`] with the compare import replaced by a handler
/// that says equal (0) or not.
pub fn prep_pass_names_import(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    prep_pass_names(e, rng, a);
    let r = u32::from(!rng.chance(3));
    let f = e.handler_c(r);
    let slot = e.rebase(0x00b0_c398);
    e.w32(slot, f);
}

/// The name word at `this` (tagged or not) and the name compared.
pub fn prep_name_is(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let strings = e.pool + POOL_PTRS + 0x800;
    for (i, s) in NAMES.iter().enumerate() {
        e.string(strings + 0x20 * i as u32, s);
    }
    let mine = if rng.chance(6) {
        0
    } else {
        strings + 0x20 * rng.below(5)
    };
    e.w32(a[0], mine | rng.below(2));
    let copy = strings + 0x200;
    let k = rng.below(5) as usize;
    e.string(copy, NAMES[k]);
    a[1] = match rng.below(4) {
        0 => 0,
        1 => mine,
        2 => copy,
        _ => strings + 0x20 * rng.below(5),
    };
}

/// The parameter's offset and size against the buffer's size word.
pub fn prep_set_value(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let off = rng.below(0x40) as u16;
    let size = (rng.below(0x40) as u16) | ((rng.below(8) as u16) << 13);
    e.write(a[1] + 8, &off.to_le_bytes());
    e.write(a[1] + 0xa, &size.to_le_bytes());
    let fit = u32::from(off) + u32::from(size & 0x1fff);
    let words = if rng.chance(3) { fit } else { rng.below(0x90) };
    e.w32(a[0], words);
}

/// The two registries' lookups (`0x0043cbb0`, run as they are): each
/// descriptor's cached type (`+0x70`), or none (bit 31 of `+0x90` and no
/// default buffer at `+0x64`: the lookup returns the null `+0x70`).
pub fn prep_two_types(e: &mut Env, rng: &mut Rng, _: &mut [u32]) {
    for r in [0x00c9_2090, 0x00ca_7228] {
        let d = e.rebase(r);
        if rng.chance(3) {
            e.w32(d + 0x70, 0);
            e.w32(d + 0x90, 0x8000_0000);
            e.w32(d + 0x64, 0);
        } else {
            let p = e.ptr(rng);
            e.w32(d + 0x70, p);
        }
    }
}

/// Context switches: a few objects (or nulls) whose virtual method 2
/// pops seven words and returns 0 or not.
pub fn prep_context_switches(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let n = rng.below(5);
    e.w32(a[0] + 0x24, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
    let list = e.ptr(rng);
    e.w32(a[0] + 0x28, list);
    let r = if rng.chance(2) { 0 } else { e.ret_word(rng) };
    let f = e.handler(7, r);
    for i in 0..n {
        let obj = if rng.chance(4) { 0 } else { e.ptr(rng) };
        e.w32(list + 4 * i, obj);
        if obj != 0 {
            let vt = e.ptr(rng);
            e.w32(obj, vt);
            e.w32(vt + 8, f);
        }
    }
    e.w32(list + 4 * n, 0);
}

/// A binary tree of a few nodes (keys 0..16) under the header `tree`,
/// and a key that is in it or not.
pub fn prep_tree(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let tree = e.pool + POOL_PTRS + 0x1000;
    a[0] = tree;
    let n = rng.below(6);
    let nodes: Vec<u32> = (0..n).map(|i| tree + 0x20 * (i + 1)).collect();
    e.w32(tree + 0xc, tree);
    let mut keys = Vec::new();
    for &node in &nodes {
        let k = loop {
            let k = rng.below(16);
            if !keys.contains(&k) {
                break k;
            }
        };
        keys.push(k);
        e.w32(node, tree);
        e.w32(node + 4, tree);
        e.w32(node + 0xc, k);
        // insert under the root
        let root = e.r32(tree + 0xc);
        if root == tree {
            e.w32(tree + 0xc, node);
            continue;
        }
        let mut cur = root;
        loop {
            let v = e.r32(cur + 0xc);
            let side = if v > k { 0 } else { 4 };
            let next = e.r32(cur + side);
            if next == tree {
                e.w32(cur + side, node);
                break;
            }
            cur = next;
        }
    }
    let key = e.ptr(rng);
    e.w32(key, rng.below(17));
    a[1] = key;
}

/// The element getter's result points into the pool; the count word is
/// random around the index.
pub fn prep_element_field(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let p = e.ptr(rng);
    e.set_ret(0x0058_5c90, p);
    let n = rng.below(10) | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + 0x24, n);
}

/// A `PArray<T>` descriptor's statics: the guard bits at `desc + 0x94`
/// (each step done or not), the descriptor's name (`+0x18`, set or not),
/// the class's name global (`name`), the annotation list head (`+0xc8`:
/// empty, pointing at itself, or not), the flags word (`+0x90`) and the
/// member words the declaration writes.
pub fn prep_declare(e: &mut Env, rng: &mut Rng, a: &mut [u32], desc: u32, name: u32) {
    let d = e.rebase(desc);
    let g = rng.below(16) | if rng.chance(3) { rng.next() & !0xf } else { 0 };
    e.w32(d + 0x94, g);
    let n = if rng.chance(3) { e.ptr(rng) } else { 0 };
    e.w32(d + 0x18, n);
    let ng = e.rebase(name);
    let v = if rng.chance(2) { e.ptr(rng) } else { 0 };
    e.w32(ng, v);
    let head = if rng.chance(2) { d + 0xc8 } else { e.ptr(rng) };
    e.w32(d + 0xc8, head);
    let f = rng.next();
    e.w32(d + 0x90, f);
    // the member's words the declaration sets (not left from the image)
    for off in [0x10c, 0x114, 0x118, 0x11c] {
        let w = rng.next();
        e.w32(d + off, w);
    }
    if rng.chance(8) {
        a[0] = 0;
    }
}

/// The shader's two COM objects (null, or objects whose `Release`
/// pops one word) and its byte array (count 0, owned or borrowed, data
/// null or not).
pub fn prep_release_shader(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    // low enough that `this + 0x4e0` stays in the pointers' part
    a[0] = e.pool + 4 * rng.below(0x400);
    for (k, off) in [0x4e0, 0].into_iter().enumerate() {
        // objects and vtables past the pointers' part: no overlap with `this`
        let obj = if rng.chance(3) {
            0
        } else {
            e.pool + POOL_PTRS + 0x100 * k as u32
        };
        e.w32(a[0] + off, obj);
        if obj != 0 {
            let vt = obj + 0x80;
            let r = e.ret_word(rng);
            let f = e.handler(1, r);
            e.w32(obj, vt);
            e.w32(vt + 8, f);
        }
    }
    let n =
        if rng.chance(3) { 0 } else { rng.below(4) } | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + 4, n);
    let data = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(a[0] + 8, data);
}

/// A `PSharray` at `this + off`: counts 0..3, owned or borrowed, data
/// null or not.
pub fn prep_sharray_at(e: &mut Env, rng: &mut Rng, a: &mut [u32], off: u32) {
    let n = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + off, n);
    let data = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(a[0] + off + 4, data);
}

/// A `PSharray` at `this + off` holding one to three elements (owned or
/// borrowed; separate storage in the pool), its first word 0 or not.
pub fn prep_sharray_first(e: &mut Env, rng: &mut Rng, a: &mut [u32], off: u32) {
    let n = 1 + rng.below(3);
    e.w32(a[0] + off, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    if n > 1 {
        // storage apart from `this`
        let data = e.pool + POOL_PTRS + 0x700 + 4 * rng.below(16);
        e.w32(a[0] + off + 4, data);
    }
    let first = if n == 1 {
        a[0] + off + 4
    } else {
        e.r32(a[0] + off + 4)
    };
    let v = if rng.chance(2) { 0 } else { e.ret_word(rng) };
    e.w32(first, v);
}

/// The D3D11 context at `this + 0x70`: an object whose methods 13
/// (three words), 24 (two) and 45 (three) are handlers.
pub fn prep_context(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let ctx = e.pool + POOL_PTRS + 0x200;
    let vt = e.pool + POOL_PTRS + 0x300;
    e.w32(a[0] + 0x70, ctx);
    e.w32(ctx, vt);
    for (slot, pops) in [(13, 3), (24, 2), (45, 3)] {
        let r = e.ret_word(rng);
        let f = e.handler(pops, r);
        e.w32(vt + 4 * slot, f);
    }
}

/// The resource tables' owner words (0x280 bytes above each of the 0x90
/// words at `this + 0x1c`): about a third are the slot looked for.
pub fn prep_owner_table(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    for i in 0..0x90 {
        if rng.chance(3) {
            e.w32(a[0] + 0x29c + 4 * i, a[2]);
        }
    }
}

/// The once flag of `0x0058ab60`'s message.
pub fn prep_once(e: &mut Env, rng: &mut Rng, _: &mut [u32]) {
    let f = e.rebase(0x00cb_4bcc);
    let v = rng.below(2) as u8;
    e.write(f, &[v]);
}

/// The object at `this + 0x133c` (or none), whose method 7 gives a
/// second object (or none), whose method 7 gives a third (or none), whose
/// method 8 takes five words.
pub fn prep_query_chain(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x100);
    let objs: Vec<u32> = (0..3)
        .map(|i| e.pool + POOL_PTRS + 0x400 + 0x80 * i)
        .collect();
    let first = if rng.chance(4) { 0 } else { objs[0] };
    e.w32(a[0] + 0x133c, first);
    for i in 0..3 {
        let vt = objs[i] + 0x40;
        e.w32(objs[i], vt);
        let r = e.ret_word(rng);
        if i < 2 {
            let next = if rng.chance(4) { 0 } else { objs[i + 1] };
            let f = e.writer(i as u32, 3, 2, r, next);
            e.w32(vt + 4 * 7, f);
        } else {
            let f = e.handler(5, r);
            e.w32(vt + 4 * 8, f);
        }
    }
}

/// The modifier network's table at `this + 0x10` (in the pool) and the
/// context's small index (`ctx + 0x20`, the context argument `k`).
pub fn prep_network(e: &mut Env, rng: &mut Rng, a: &mut [u32], k: usize) {
    let t = e.ptr(rng);
    e.w32(a[0] + 0x10, t);
    // the context apart from `this`
    a[k] = e.pool + POOL_PTRS + 0x600 + 4 * rng.below(8);
    let i = rng.below(16);
    e.w32(a[k] + 0x20, i);
}

/// The four views at `this + 0x168..0x174`: null or objects (apart from
/// `this`) whose `Release` pops one word.
pub fn prep_release_views(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    for (k, off) in [0x168, 0x16c, 0x170, 0x174].into_iter().enumerate() {
        let obj = if rng.chance(3) {
            0
        } else {
            e.pool + POOL_PTRS + 0x900 + 0x100 * k as u32
        };
        e.w32(a[0] + off, obj);
        if obj != 0 {
            let vt = obj + 0x80;
            let r = e.ret_word(rng);
            let f = e.handler(1, r);
            e.w32(obj, vt);
            e.w32(vt + 8, f);
        }
    }
}

/// `0x004fd8b0`'s guard word: bit 0 set or not, other bits random.
pub fn prep_static_once(e: &mut Env, rng: &mut Rng, _: &mut [u32]) {
    let g = e.rebase(0x00c9_628c);
    let v = rng.next();
    e.w32(g, v);
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
    // the original's frame: below the arguments, the return address and
    // the saved `ebp`, from where `Oracle::call` pushes
    let ebp = e.a.stack_range().end - 0x100 - 4 * stack.len() as u32 - 8;
    let lo = ebp - case.gap - case.frame;
    if case.frame != 0 {
        let junk: Vec<u8> = (0..case.frame).map(|i| (i * 0x3d + 0x11) as u8).collect();
        e.write(lo, &junk);
        e.b.set_call_top(Some(lo - 0x100));
    }
    let ra =
        e.a.call(addr, conv, stack)
            .unwrap_or_else(|err| panic!("{what}: the original: {err}"));
    assert!(
        ra.st0_raw.is_none(),
        "{what}: the original leaves ST0 set ({:?})",
        ra.st0
    );
    let rb = (case.port)(&mut G(&mut e.b, lo), &args);
    if case.frame != 0 {
        e.b.set_call_top(None);
        let fa = Oracle::read(&mut e.a, lo, case.frame as usize).unwrap();
        let fb = Oracle::read(&mut e.b, lo, case.frame as usize).unwrap();
        assert_eq!(fa, fb, "{what}: the frame differs ({args:x?})");
    }
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

/// Runs every case whose name contains one of `filter`'s `|`-separated
/// values (env `H2_FILTER`, default all).
fn run_cases(rounds: u32) {
    let filter = std::env::var("H2_FILTER").unwrap_or_default();
    let mut ran = 0;
    for (snap, mut e) in Env::all() {
        for case in cases::CASES
            .iter()
            .chain(cases3::CASES)
            .chain(cases4::CASES)
            .chain(cases5::CASES)
            .chain(cases6::CASES)
            .chain(cases7::CASES)
            .chain(cases8::CASES)
            .filter(|c| filter.split('|').any(|f| c.name.contains(f)))
        {
            for &(sym, n, pops, this) in case.callees {
                let write = case.writes.iter().find(|w| w.0 == sym).map(|&(_, k)| k);
                e.stub(sym, n, pops, this, write);
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
    for c in cases::CASES
        .iter()
        .chain(cases3::CASES)
        .chain(cases4::CASES)
        .chain(cases5::CASES)
        .chain(cases6::CASES)
        .chain(cases7::CASES)
        .chain(cases8::CASES)
    {
        assert!(seen.insert(c.addr), "{} twice", c.name);
    }
    let _ = (Init::Word(0, 0), x::map_mode(0));
}
