//! M4 port round 1, list p6-render-c (`ffx_prender`): the ports against
//! the original machine code, on the image alone and on a snapshot
//! (`FFX_SNAPSHOT`) with synthetic data over its memory. Results (the
//! whole of `eax` where the original defines it) and every byte written
//! must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its stack
//! arguments to a log in memory, so the diff checks that both made the
//! same calls in the same order, with the same `this`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::p6_descriptor::{self as descriptor, family};

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
}

/// The bridge's view of an oracle.
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
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        let r = self.call_ret(addr, conv, args, true);
        r.st0_f32()
            .unwrap_or_else(|| panic!("callee {addr:#x}: no float in ST0"))
    }
    fn call_st0_f64(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f64 {
        let r = self.call_ret(addr, conv, args, true);
        r.st0_f64()
            .unwrap_or_else(|| panic!("callee {addr:#x}: no float in ST0"))
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // the trampoline `Twins::trampoline` wrote at `CAVE`
        let at = self.0.rebase(CAVE);
        let b = x.to_bits();
        let r = self
            .0
            .call(at, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap_or_else(|e| panic!("intrinsic {addr:#x}: {e}"));
        let Some(v) = r.st0_f32() else {
            panic!("intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        v
    }
}

/// `graphicFontDraw`'s body, which no test here calls: `Twins::trampoline`
/// writes `fld qword [esp+4]; call [esp+0xc]; ret` there, and at `+16`
/// `fstp st0; ret`, for `Host::call_intrinsic` (a CRT intrinsic with its
/// argument in `ST0`).
const CAVE: u32 = 0x0064_0660;

impl G<'_> {
    /// A call through the oracle, checked for the game's call width.
    fn call_ret(
        &mut self,
        addr: u32,
        conv: host::Conv,
        args: &[u32],
        st0: bool,
    ) -> ffx_oracle::Ret {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, st0));
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
    }
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
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
}

/// Two oracles started alike: `a` runs the originals, `b` the ports.
struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
    /// stack words).
    log: u32,
    /// The bytes each stub's `jmp` replaced, by address.
    saved: HashMap<u32, Vec<u8>>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let mut t = Self {
            a,
            b,
            log: 0,
            saved: HashMap::new(),
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

    fn code(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_code(bytes).unwrap();
        assert_eq!(self.b.alloc_code(bytes).unwrap(), x);
        x
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    /// Writes `Host::call_intrinsic`'s trampoline at [`CAVE`] in both.
    fn trampoline(&mut self) {
        let at = self.rebase(CAVE);
        self.write(
            at,
            &[
                0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90,
                0x90, 0x90, 0xdd, 0xd8, 0xc3,
            ],
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// A recording stub: logs `tag`, `ecx` when `this` (thiscall callees)
    /// and `n` stack words; pops `pops` words; returns `ret`. Returns its
    /// address.
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
        self.code(&c)
    }

    /// A recording stub of a callee with no stack words that may read
    /// `ecx` and `edx`: logs `tag`, `ecx`, `edx` and the word at `ecx`
    /// (which a destructor's caller may have just written); returns `ret`.
    fn stub_regs(&mut self, tag: u32, ret: u32) -> u32 {
        self.stub_this(tag, ret, true)
    }

    /// [`Twins::stub_regs`], logging `edx` only when `edx` (a thiscall
    /// callee's `edx` is the caller's scratch).
    fn stub_this(&mut self, tag: u32, ret: u32, edx: bool) -> u32 {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        if edx {
            c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
        }
        c.extend([0x8b, 0x11]); // mov edx, [ecx]
        c.extend([0x89, 0x50, 0x0c]); // mov [eax+0xc], edx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc3);
        self.code(&c)
    }

    /// [`Twins::stub_code`] placed at the original `sym` (a `jmp` there).
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        let addr = self.stub_code(sym, this, n, pops, ret);
        self.jump(sym, addr);
    }

    /// A `jmp` to `addr` written over the original `sym`.
    fn jump(&mut self, sym: u32, addr: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        let old = Oracle::read(&mut self.a, at, 5).unwrap();
        self.saved.entry(at).or_insert(old);
        self.write(at, &j);
    }

    /// Puts back the original bytes at `sym`.
    fn unstub(&mut self, sym: u32) {
        let at = self.rebase(sym);
        let bytes = self.saved.remove(&at).expect("stubbed");
        self.write(at, &bytes);
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
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        match out {
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
        }
        self.diff(what);
    }
}

/// A descriptor pointer for `ecx` (never read).
const DESC: u32 = 0x00c0_ffee;

// ---- class-descriptor callbacks ----

#[test]
fn descriptor_destroy_virtual_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_9bdf);
        let obj = t.zeros(16);
        let vtable = t.zeros(16);
        t.w32(obj, vtable);
        for &f in family::DESTROY_VIRTUAL {
            for i in 0..4 {
                let ret = rng.next();
                let stub = t.stub_code(f, true, 1, 1, ret);
                t.w32(vtable, stub);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj + 4],
                    |g| Out::U32(descriptor::bridge::destroy_virtual(g, obj + 4)),
                );
                assert_eq!(t.log_entries(), before + 1, "{name} {f:#x}: one call");
                let this: u32 = Mem::read(&t.b, t.log + 4 + 32 * before + 4);
                assert_eq!(this, obj, "{name} {f:#x}: this");
            }
        }
    }
}

#[test]
fn descriptor_constants_match_the_original() {
    for (name, mut t) in Twins::all() {
        let obj = t.zeros(16);
        for &f in family::TRUE1 {
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(DESC),
                &[obj],
                |_| Out::U8(1),
            );
        }
        for &f in family::ZERO1 {
            t.check(
                &format!("{name} {f:#x}"),
                f,
                Conv::Thiscall(DESC),
                &[obj],
                |_| Out::U32(0),
            );
        }
    }
}

#[test]
fn descriptor_destroy_with_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x2468_0ace);
        let obj = t.zeros(16);
        for &(f, dtor) in family::DESTROY_WITH {
            for i in 0..4 {
                t.stub(dtor, true, 0, 0, rng.next());
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::destroy_with(g, dtor, obj)),
                );
                assert_eq!(t.log_entries(), before + 1);
                let this: u32 = Mem::read(&t.b, t.log + 4 + 32 * before + 4);
                assert_eq!(this, obj, "{name} {f:#x}: this");
                t.unstub(dtor);
            }
        }
    }
}

/// Array words for the free cases: owned or not, null or not.
fn array_case(rng: &mut Rng, i: u32) -> (u32, u32) {
    let count = match i % 4 {
        0 => rng.below(100),
        1 => 0x8000_0000 | rng.below(100),
        2 => rng.next(),
        _ => 0,
    };
    let data = if i.is_multiple_of(3) || rng.chance(5) {
        0
    } else {
        rng.next() | 1
    };
    (count, data)
}

#[test]
fn descriptor_array_free_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f0f_1234);
        let obj = t.zeros(32);
        let cases: Vec<(u32, u32, bool)> = family::ARRAY_FREE
            .iter()
            .map(|&f| (f, 0, false))
            .chain(family::ARRAY_FREE_NN.iter().map(|&f| (f, 0, true)))
            .chain(family::ARRAY_FREE_NN_C.iter().map(|&f| (f, 0xc, true)))
            .collect();
        for (f, at, skip_null) in cases {
            for i in 0..24 {
                let (count, data) = array_case(&mut rng, i);
                for k in 0..8 {
                    t.w32(obj + 4 * k, rng.next());
                }
                t.w32(obj + at, count);
                t.w32(obj + at + 4, data);
                t.stub(descriptor::PHYRE_FREE, false, 1, 0, rng.next());
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i} ({count:#x}, {data:#x})"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::array_free(g, obj, at, skip_null)),
                );
                let freed = count & 0x8000_0000 == 0 && !(skip_null && data == 0);
                assert_eq!(t.log_entries(), before + u32::from(freed), "{name} {f:#x}");
                t.unstub(descriptor::PHYRE_FREE);
            }
        }
    }
}

#[test]
fn descriptor_array_free_with_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_1234);
        let obj = t.zeros(16);
        for &(f, free) in family::ARRAY_FREE_WITH {
            for i in 0..24 {
                let (count, data) = array_case(&mut rng, i);
                t.w32(obj, count);
                t.w32(obj + 4, data);
                t.w32(obj + 8, rng.next());
                t.stub(free, false, 2, 0, rng.next());
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i} ({count:#x}, {data:#x})"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::array_free_with(g, free, obj)),
                );
                let freed = count & 0x8000_0000 == 0;
                assert_eq!(t.log_entries(), before + u32::from(freed), "{name} {f:#x}");
                t.unstub(free);
            }
        }
    }
}

#[test]
fn descriptor_array_init_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5a5a_0001);
        for &f in family::ARRAY_INIT {
            for i in 0..4 {
                let obj = if i == 0 { 0 } else { t.zeros(16) };
                if obj != 0 {
                    for k in 0..4 {
                        t.w32(obj + 4 * k, rng.next());
                    }
                }
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::array_init(g, obj)),
                );
            }
        }
    }
}

/// Every `ported!` entry of `ported/p6_render_c.rs` that calls a
/// descriptor bridge names the bridge (and callee) of its address's
/// family, and every family address has one entry.
#[test]
fn descriptor_shims_match_their_families() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported/p6_render_c.rs"),
    )
    .unwrap();
    let mut want: HashMap<u32, String> = HashMap::new();
    let hex = |a: u32| format!("{:#06x}_{:04x}", a >> 16, a & 0xffff);
    for &a in family::DESTROY_VIRTUAL {
        want.insert(
            a,
            "descriptor::bridge::destroy_virtual(&mut mem(), obj)".into(),
        );
    }
    for &a in family::TRUE1 {
        want.insert(a, "-> u8 {\n        1\n".into());
    }
    for &a in family::ZERO1 {
        want.insert(a, "-> u32 {\n        0\n".into());
    }
    for &(a, d) in family::DESTROY_WITH {
        want.insert(
            a,
            format!(
                "descriptor::bridge::destroy_with(&mut mem(), {}, obj)",
                hex(d)
            ),
        );
    }
    for &a in family::ARRAY_FREE {
        want.insert(
            a,
            "descriptor::bridge::array_free(&mut mem(), arr, 0, false)".into(),
        );
    }
    for &a in family::ARRAY_FREE_NN {
        want.insert(
            a,
            "descriptor::bridge::array_free(&mut mem(), arr, 0, true)".into(),
        );
    }
    for &a in family::ARRAY_FREE_NN_C {
        want.insert(
            a,
            "descriptor::bridge::array_free(&mut mem(), obj, 0xc, true)".into(),
        );
    }
    for &(a, d) in family::ARRAY_FREE_WITH {
        want.insert(
            a,
            format!(
                "descriptor::bridge::array_free_with(&mut mem(), {}, arr)",
                hex(d)
            ),
        );
    }
    for &a in family::ARRAY_INIT {
        want.insert(a, "descriptor::bridge::array_init(&mut mem(), arr)".into());
    }
    let opt = |v: Option<u32>| v.map_or("None".to_string(), |v| format!("Some({})", hex(v)));
    for &(a, n) in family::CONSTRUCT_ZEROED {
        want.insert(
            a,
            format!("descriptor::bridge::construct_zeroed(&mut mem(), obj, {n})"),
        );
    }
    for &a in family::CONSTRUCT_UNSET_PAIR {
        want.insert(
            a,
            "descriptor::bridge::construct_unset_pair(&mut mem(), obj)".into(),
        );
    }
    for &a in family::CONSTRUCT_UNSET_PAIR_ZEROED {
        want.insert(
            a,
            "descriptor::bridge::construct_unset_pair_zeroed(&mut mem(), obj)".into(),
        );
    }
    for &(a, vt, half) in family::CONSTRUCT_VTABLE_ZEROED {
        let half = half.map_or("None".to_string(), |v| format!("Some({v:#x})"));
        want.insert(
            a,
            format!(
                "descriptor::bridge::construct_vtable_zeroed(&mut mem(), obj, {}, {half})",
                hex(vt)
            ),
        );
    }
    for &(a, [x, y, vt]) in family::CONSTRUCT_INPUT {
        want.insert(
            a,
            format!(
                "descriptor::bridge::construct_input(&mut mem(), obj, [{}, {}, {}])",
                hex(x),
                hex(y),
                hex(vt)
            ),
        );
    }
    for &(a, [x, y, vt]) in family::CONSTRUCT_INPUT_PAIR {
        want.insert(
            a,
            format!(
                "descriptor::bridge::construct_input_pair(&mut mem(), obj, [{}, {}, {}])",
                hex(x),
                hex(y),
                hex(vt)
            ),
        );
    }
    for &(a, ctor, vt, w) in family::CONSTRUCT_WITH {
        want.insert(
            a,
            format!(
                "descriptor::bridge::construct_with(&mut mem(), obj, {}, {}, {})",
                hex(ctor),
                opt(vt),
                opt(w)
            ),
        );
    }
    for &(a, vt, dtor) in family::DELETING_DTOR {
        want.insert(
            a,
            format!(
                "descriptor::bridge::deleting_dtor(&mut mem(), this, flags, {}, {})",
                opt(vt),
                opt(dtor)
            ),
        );
    }
    for &a in family::MEMBER_CALL {
        want.insert(
            a,
            "descriptor::bridge::member_call(&mut mem(), member, obj, a, b)".into(),
        );
    }
    for &a in family::MEMBER_GET {
        want.insert(
            a,
            "descriptor::bridge::member_get(&mut mem(), member, obj)".into(),
        );
    }
    let mut seen = 0;
    for (a, body) in &want {
        let key = format!(" @ {} => ", hex(*a));
        let at = src
            .find(&key)
            .unwrap_or_else(|| panic!("{a:#x}: no ported! entry"));
        let end = src[at..].find("\n    }").map_or(src.len(), |e| at + e + 6);
        assert!(
            src[at..end].contains(body.as_str()),
            "{a:#x}: shim body is not {body:?}"
        );
        assert_eq!(src.matches(&key).count(), 1, "{a:#x}: hooked twice");
        seen += 1;
    }
    assert_eq!(seen, 102 + 79 + 2);
}

/// A port of a constructor shape, on `obj`.
type Port = Box<dyn Fn(&mut G, u32) -> u32>;

/// An object of `n` random words (0 when `null`).
fn object(t: &mut Twins, rng: &mut Rng, n: u32, null: bool) -> u32 {
    if null {
        return 0;
    }
    let obj = t.zeros(4 * n as usize);
    for k in 0..n {
        t.w32(obj + 4 * k, rng.next());
    }
    obj
}

#[test]
fn descriptor_construct_inline_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0c0c_5151);
        let cases: Vec<(u32, Port)> = family::CONSTRUCT_ZEROED
            .iter()
            .map(|&(f, n)| {
                let p: Port = Box::new(move |g, o| descriptor::bridge::construct_zeroed(g, o, n));
                (f, p)
            })
            .chain(family::CONSTRUCT_UNSET_PAIR.iter().map(|&f| {
                let p: Port = Box::new(|g, o| descriptor::bridge::construct_unset_pair(g, o));
                (f, p)
            }))
            .chain(family::CONSTRUCT_UNSET_PAIR_ZEROED.iter().map(|&f| {
                let p: Port =
                    Box::new(|g, o| descriptor::bridge::construct_unset_pair_zeroed(g, o));
                (f, p)
            }))
            .chain(
                family::CONSTRUCT_VTABLE_ZEROED
                    .iter()
                    .map(|&(f, vt, half)| {
                        let p: Port = Box::new(move |g, o| {
                            descriptor::bridge::construct_vtable_zeroed(g, o, vt, half)
                        });
                        (f, p)
                    }),
            )
            .chain(family::CONSTRUCT_INPUT.iter().map(|&(f, fields)| {
                let p: Port =
                    Box::new(move |g, o| descriptor::bridge::construct_input(g, o, fields));
                (f, p)
            }))
            .collect();
        for (f, port) in &cases {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 8, i == 0);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    *f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(port(g, obj)),
                );
            }
        }
    }
}

#[test]
fn descriptor_construct_with_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x00de_c0de);
        for &(f, ctor, vt, w) in family::CONSTRUCT_WITH {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 4, i == 0);
                // logs the word at obj the constructor finds
                let r = rng.next();
                let code = t.stub_this(ctor, r, false);
                t.jump(ctor, code);
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U8(descriptor::bridge::construct_with(g, obj, ctor, vt, w)),
                );
                assert_eq!(t.log_entries(), before + u32::from(obj != 0), "{f:#x}");
                t.unstub(ctor);
            }
        }
    }
}

#[test]
fn descriptor_deleting_dtor_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_f00d);
        for &(f, vt, dtor) in family::DELETING_DTOR {
            for (i, flags) in [0, 1, 2, 3, 0x100, 0xffff_fffe, rng.next()]
                .into_iter()
                .enumerate()
            {
                let this = object(&mut t, &mut rng, 4, false);
                if let Some(d) = dtor {
                    // logs the vtable word the destructor finds
                    let r = rng.next();
                    let code = t.stub_this(d, r, false);
                    t.jump(d, code);
                }
                t.stub(descriptor::PHYRE_FREE, false, 1, 0, rng.next());
                let before = t.log_entries();
                t.check(
                    &format!("{name} {f:#x} #{i} flags {flags:#x}"),
                    f,
                    Conv::Thiscall(this),
                    &[flags],
                    |g| Out::U32(descriptor::bridge::deleting_dtor(g, this, flags, vt, dtor)),
                );
                let calls = u32::from(dtor.is_some()) + (flags & 1);
                assert_eq!(t.log_entries(), before + calls, "{f:#x}");
                t.unstub(descriptor::PHYRE_FREE);
                if let Some(d) = dtor {
                    t.unstub(d);
                }
            }
        }
    }
}

#[test]
fn descriptor_member_call_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3c3c_0020);
        for &f in family::MEMBER_CALL {
            for i in 0..4 {
                let member = object(&mut t, &mut rng, 0x10, false);
                let cb = if i == 0 {
                    0
                } else {
                    let r = rng.next();
                    t.stub_code(f, true, 2, 2, r)
                };
                t.w32(member + 0x3c, cb);
                let (obj, a, b) = (rng.next(), rng.next(), rng.next());
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(member),
                    &[obj, a, b],
                    |g| Out::U32(descriptor::bridge::member_call(g, member, obj, a, b)),
                );
            }
        }
    }
}

#[test]
fn descriptor_member_get_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3838_0018);
        for &f in family::MEMBER_GET {
            for i in 0..6 {
                let member = object(&mut t, &mut rng, 0x10, false);
                let class = object(&mut t, &mut rng, 0x30, false);
                let obj = object(&mut t, &mut rng, 0x20, false);
                let cb = if i < 3 {
                    0
                } else {
                    let r = rng.next();
                    t.stub_regs(f, r)
                };
                t.w32(member + 0x38, cb);
                t.w32(member + 0xc, class);
                // the count's offset in the instance: member +0x24 less
                // the class's +0x88
                let base = rng.next();
                t.w32(class + 0x88, base);
                t.w32(member + 0x24, base.wrapping_add(4 * rng.below(0x20)));
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(member),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::member_get(g, member, obj)),
                );
            }
        }
    }
}

// ---- small engine functions (`ffx_prender::misc`) ----

use ffx_prender::p6_misc as misc;

impl Twins {
    /// A thiscall stub with no stack words at `sym` logging `ecx` and the
    /// word at `ecx`; returns `ret`.
    fn stub_at(&mut self, sym: u32, ret: u32) {
        let code = self.stub_this(sym, ret, false);
        self.jump(sym, code);
    }
}

#[test]
fn misc_constants_and_globals_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1111_2222);
        t.check(
            &format!("{name} nvn"),
            0x0059_d2e0,
            Conv::Cdecl,
            &[],
            |_| Out::U32(0x17),
        );
        for f in [0x0060_ed70, 0x0060_ee00] {
            let (a, b) = (rng.next(), rng.next());
            t.check(&format!("{name} {f:#x}"), f, Conv::Stdcall, &[a, b], |_| {
                Out::U32(0)
            });
        }
        let app = t.rebase(misc::APPLICATION);
        let v = rng.next();
        t.w32(app, v);
        t.check(&format!("{name} app"), 0x0062_3b90, Conv::Cdecl, &[], |g| {
            Out::U32(misc::bridge::get_application(g))
        });
        for (i, format) in [0x4a, 0x13, 0x90, 0x91, 0x14, 0x92, 0x1_004a, 0, rng.next()]
            .into_iter()
            .enumerate()
        {
            t.check(
                &format!("{name} format #{i}"),
                0x005a_04d0,
                Conv::Cdecl,
                &[format],
                |_| Out::U32(misc::bridge::format_in_set(format)),
            );
        }
        for n in [0, 1, 2, 3, 0x8000_0000, rng.next()] {
            t.check(
                &format!("{name} flip {n:#x}"),
                0x0062_5090,
                Conv::Cdecl,
                &[n],
                |g| Out::U32(misc::bridge::set_flip_interval(g, n)),
            );
        }
    }
}

#[test]
fn misc_setters_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3333_4444);
        for i in 0..8 {
            let obj = object(&mut t, &mut rng, 0x110, false);
            let (a, b, c) = (rng.next(), rng.next(), rng.next());
            let w = |s: &str| format!("{name} {s} #{i}");
            t.check(&w("icon"), 0x0062_cf50, Conv::Thiscall(obj), &[a], |g| {
                Out::U32(misc::bridge::set_window_icon_id(g, obj, a))
            });
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.check(
                &w("surface"),
                0x005b_35c0,
                Conv::Thiscall(obj),
                &[a, b],
                |g| Out::U32(misc::bridge::render_surface_ctor(g, obj, a, b)),
            );
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.check(&w("zero4"), 0x005b_3620, Conv::Thiscall(obj), &[], |g| {
                Out::U32(misc::bridge::zero4(g, obj))
            });
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.check(
                &w("three"),
                0x005b_3660,
                Conv::Thiscall(obj),
                &[a, b, c],
                |g| Out::U32(misc::bridge::set_three(g, obj, a, b, c)),
            );
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.check(
                &w("ring"),
                0x0061_3e80,
                Conv::Thiscall(obj),
                &[a, b, c],
                |g| Out::U32(misc::bridge::ring_init(g, obj, a, b, c)),
            );
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.stub_at(misc::CRITICAL_SECTION_INIT, rng.next());
            t.check(
                &w("range"),
                0x0062_9840,
                Conv::Thiscall(obj),
                &[a, b],
                |g| Out::U32(misc::bridge::range_init(g, obj, a, b)),
            );
            t.unstub(misc::CRITICAL_SECTION_INIT);
            let obj = object(&mut t, &mut rng, 0x110, false);
            t.stub_at(misc::EVENT_FLAG_CTOR, rng.next());
            t.check(&w("kick"), 0x0061_3fc0, Conv::Thiscall(obj), &[], |g| {
                Out::U32(misc::bridge::kick_ctor(g, obj))
            });
            t.unstub(misc::EVENT_FLAG_CTOR);
            for (f, base, vt, w4) in [
                (0x0067_2630, 0x0049_68c0, 0x00b4_3ed8, None),
                (0x009f_c140, 0x009f_c160, 0x00b7_5c88, Some(0x0194_45a8)),
                (0x009f_c7c0, 0x009f_c7e0, 0x00b7_50b8, Some(0x0194_3e88)),
            ] {
                let obj = object(&mut t, &mut rng, 0x110, false);
                t.stub_at(base, rng.next());
                t.check(&w(&format!("{f:#x}")), f, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(misc::bridge::derived_ctor(g, obj, base, vt, w4))
                });
                t.unstub(base);
            }
        }
    }
}

#[test]
fn misc_lookups_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5555_6666);
        for i in 0..24 {
            let w = |s: &str| format!("{name} {s} #{i}");
            // the cursor's run length
            let cursor = object(&mut t, &mut rng, 8, false);
            let at = rng.below(1000);
            let end = if i % 3 == 0 { at } else { rng.below(1000) };
            t.w32(cursor + 0xc, at);
            t.w32(cursor + 0x10, end);
            t.w32(cursor + 0x14, 1 + rng.below(40));
            t.check(&w("run"), 0x009f_3810, Conv::Thiscall(cursor), &[], |g| {
                Out::U32(misc::bridge::cursor_run_length(g, cursor))
            });
            // PArray element
            let data = object(&mut t, &mut rng, 8, false);
            let arr = object(&mut t, &mut rng, 4, false);
            t.w32(
                arr + 4,
                rng.below(10) | if rng.chance(2) { 0x8000_0000 } else { 0 },
            );
            t.w32(arr + 8, data);
            let idx = if rng.chance(4) {
                rng.next()
            } else {
                rng.below(12)
            };
            t.check(&w("at"), 0x005b_2c70, Conv::Thiscall(arr), &[idx], |g| {
                Out::U32(misc::bridge::array_at(g, arr, idx))
            });
        }
    }
}

#[test]
fn misc_ring_allocators_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_8888);
        let ring = t.zeros(16);
        let out = t.zeros(4);
        for i in 0..64 {
            let start = 0x1000 * rng.below(16);
            let end = start + rng.below(0x2000);
            let cursor = start + rng.below(0x2400);
            // the boundaries: blocks that end exactly at the ring's end,
            // or one byte past it, from the cursor or from the start
            let size = match i % 8 {
                0 => 0,
                1 => rng.below(0x100),
                2 => rng.below(0x2400),
                3 => end.wrapping_sub(cursor),
                4 => end.wrapping_sub(cursor).wrapping_add(1),
                5 => end - start,
                6 => end - start + 1,
                _ => rng.next(),
            };
            t.w32(ring, start);
            t.w32(ring + 4, end);
            t.w32(ring + 8, cursor);
            t.w32(out, rng.next());
            t.check(
                &format!("{name} segmented #{i}"),
                0x0061_3e10,
                Conv::Thiscall(ring),
                &[size],
                |g| Out::U32(misc::bridge::segmented_allocate(g, ring, size)),
            );
            t.check(
                &format!("{name} step #{i}"),
                0x0061_3f00,
                Conv::Thiscall(ring),
                &[cursor, size, out],
                |g| {
                    Out::U32(misc::bridge::step_for_allocation(
                        g, ring, cursor, size, out,
                    ))
                },
            );
        }
    }
}

#[test]
fn misc_calls_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x9999_aaaa);
        for i in 0..6 {
            let w = |s: &str| format!("{name} {s} #{i}");
            for f in misc::APPLICATION_BINDS.into_iter().chain(misc::TEXT_BINDS) {
                t.stub(f, false, 0, 0, rng.next());
            }
            t.check(&w("pre"), 0x0062_ac70, Conv::Cdecl, &[], |g| {
                Out::U32(misc::bridge::pre_phyre_init(g))
            });
            t.check(&w("text"), 0x00a3_ea40, Conv::Cdecl, &[], |g| {
                Out::U32(misc::bridge::text_initialize(g))
            });
            for f in misc::APPLICATION_BINDS.into_iter().chain(misc::TEXT_BINDS) {
                t.unstub(f);
            }
            // the render thread's wrappers
            let thread = object(&mut t, &mut rng, 0x60, false);
            let cmd = object(&mut t, &mut rng, 4, false);
            t.w32(cmd + 4, thread);
            t.stub(misc::RENDER_THREAD_INSTANCE, false, 0, 0, thread);
            t.stub_at(misc::RENDER_THREAD_SYNC, rng.next());
            t.stub_at(misc::RENDER_THREAD_ENTRY, rng.next());
            t.stub_at(misc::EVENT_SIGNAL, rng.next());
            t.stub_at(misc::SEMAPHORE_WAIT, rng.next());
            t.check(&w("sync"), 0x005b_c3f0, Conv::Cdecl, &[], |g| {
                Out::U32(misc::bridge::sync_render(g))
            });
            t.check(&w("entry"), 0x0061_4710, Conv::Cdecl, &[thread], |g| {
                Out::U32(misc::bridge::static_thread_entry(g, thread))
            });
            t.check(&w("signal"), 0x0061_4700, Conv::Cdecl, &[cmd], |g| {
                Out::U32(misc::bridge::signal_sync_flag(g, cmd))
            });
            t.check(&w("flip"), 0x0061_4c00, Conv::Thiscall(thread), &[], |g| {
                Out::U32(misc::bridge::get_flip_slot(g, thread))
            });
            for f in [
                misc::RENDER_THREAD_INSTANCE,
                misc::RENDER_THREAD_SYNC,
                misc::RENDER_THREAD_ENTRY,
                misc::EVENT_SIGNAL,
                misc::SEMAPHORE_WAIT,
            ] {
                t.unstub(f);
            }
            // registration
            t.stub(misc::REGISTER_TYPE, false, 1, 0, rng.next());
            t.stub_at(misc::ADD_TO_NAMESPACE, rng.next());
            t.check(&w("register"), 0x0067_2790, Conv::Cdecl, &[], |g| {
                Out::U32(misc::bridge::register_dynamic_mesh(g))
            });
            t.unstub(misc::REGISTER_TYPE);
            t.unstub(misc::ADD_TO_NAMESPACE);
            // renderMeshInstance with the renderer's pass
            let renderer = object(&mut t, &mut rng, 0x50, false);
            let instance = object(&mut t, &mut rng, 8, false);
            t.stub(misc::RENDER_MESH_INSTANCE, true, 4, 4, rng.next());
            t.check(
                &w("mesh"),
                0x005b_9a70,
                Conv::Thiscall(renderer),
                &[instance],
                |g| Out::U32(misc::bridge::render_instance(g, renderer, instance)),
            );
            t.unstub(misc::RENDER_MESH_INSTANCE);
            // pooled handle release
            let owner = object(&mut t, &mut rng, 8, false);
            let pool = object(&mut t, &mut rng, 4, false);
            t.w32(owner + 0x14, pool);
            let handle = object(&mut t, &mut rng, 2, false);
            t.w32(handle, if i % 2 == 0 { 0 } else { owner });
            t.stub(0x004b_a540, true, 2, 2, rng.next());
            let addr = t.rebase(0x0061_c890);
            t.a.call(addr, Conv::Thiscall(handle), &[]).unwrap();
            misc::bridge::release_handle(&mut G(&mut t.b), 0x004b_a540, handle);
            t.diff(&w("release"));
            t.unstub(0x004b_a540);
            // lookup
            let out = object(&mut t, &mut rng, 1, false);
            let x = rng.next();
            t.stub(
                0x005c_84f0,
                false,
                1,
                0,
                if i % 2 == 0 { 0 } else { rng.next() },
            );
            t.check(&w("lookup"), 0x005c_86e0, Conv::Cdecl, &[out, x], |g| {
                Out::U32(misc::bridge::lookup_into(g, 0x005c_84f0, out, x))
            });
            t.unstub(0x005c_84f0);
            // the 16-byte table at +0x38
            let table = object(&mut t, &mut rng, 16, false);
            let obj = object(&mut t, &mut rng, 0x10, false);
            t.w32(obj + 0x38, table);
            let k = rng.below(4);
            if i % 3 == 0 {
                t.w32(table + 16 * k, 0);
            }
            let (x, y) = (rng.next(), rng.next());
            t.stub(0x005c_6680, true, 1, 1, rng.next());
            t.stub(0x005c_6c30, true, 2, 2, rng.next());
            t.check(
                &w("table1"),
                0x005a_ead0,
                Conv::Thiscall(obj),
                &[k, x],
                |g| Out::U32(misc::bridge::table_call(g, 0x005c_6680, obj, k, &[x])),
            );
            t.check(
                &w("table2"),
                0x005a_ece0,
                Conv::Thiscall(obj),
                &[k, x, y],
                |g| Out::U32(misc::bridge::table_call(g, 0x005c_6c30, obj, k, &[x, y])),
            );
            t.unstub(0x005c_6680);
            t.unstub(0x005c_6c30);
            // Iggy's allocation callbacks
            t.stub(misc::PHYRE_MALLOC, false, 2, 0, rng.next());
            t.stub(misc::PHYRE_FREE, false, 1, 0, rng.next());
            let (u, s, p) = (rng.next(), rng.next(), rng.next());
            t.check(&w("alloc"), 0x00a3_e5a0, Conv::Stdcall, &[u, s, p], |g| {
                Out::U32(misc::bridge::iggy_alloc(g, s))
            });
            t.check(&w("free"), 0x00a3_e5c0, Conv::Stdcall, &[u, p], |g| {
                Out::U32(misc::bridge::iggy_free(g, p))
            });
            t.unstub(misc::PHYRE_MALLOC);
            t.unstub(misc::PHYRE_FREE);
            // the header's callback (cdecl)
            let header = object(&mut t, &mut rng, 0x30, false);
            let r = rng.next();
            let cb = t.stub_code(0x005c_7f50, false, 2, 0, r);
            t.w32(header + 0x94, cb);
            t.check(
                &w("header"),
                0x005c_7f50,
                Conv::Thiscall(header),
                &[x, y],
                |g| Out::U32(misc::bridge::header_call(g, header, x, y)),
            );
            // the tree head node
            let node = object(&mut t, &mut rng, 5, false);
            t.stub(misc::OPERATOR_NEW, false, 1, 0, node);
            t.check(&w("tree"), 0x0062_5850, Conv::Cdecl, &[], |g| {
                Out::U32(misc::bridge::tree_head(g))
            });
            t.unstub(misc::OPERATOR_NEW);
        }
    }
}

#[test]
fn descriptor_construct_input_pair_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0a1b_2c3d);
        for &(f, fields) in family::CONSTRUCT_INPUT_PAIR {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 10, i == 0);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U8(descriptor::bridge::construct_input_pair(g, obj, fields)),
                );
            }
        }
    }
}

#[test]
fn misc_constructors_b4_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x4444_b4b4);
        for i in 0..6 {
            let w = |s: &str| format!("{name} {s} #{i}");
            for (f, n) in [(0x0061_3d90, 5), (0x005b_4560, 7)] {
                let obj = object(&mut t, &mut rng, 0x20, false);
                t.check(&w(&format!("{f:#x}")), f, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(misc::bridge::zero_words(g, obj, n))
                });
            }
            let obj = object(&mut t, &mut rng, 0x20, false);
            t.check(&w("zero6"), 0x005b_3580, Conv::Thiscall(obj), &[], |g| {
                Out::U32(misc::bridge::zero6_with_default(g, obj))
            });
            let obj = object(&mut t, &mut rng, 0x20, false);
            t.check(&w("modifier"), 0x005b_f340, Conv::Thiscall(obj), &[], |g| {
                Out::U32(misc::bridge::zero_modifier(g, obj))
            });
            let obj = object(&mut t, &mut rng, 0x20, false);
            let a5 = [rng.next(), rng.next(), rng.next(), rng.next(), rng.next()];
            t.check(&w("stream"), 0x005b_44c0, Conv::Thiscall(obj), &a5, |g| {
                Out::U32(misc::bridge::stream_ctor(g, obj, a5))
            });
            let obj = object(&mut t, &mut rng, 0x20, false);
            let a10: [u32; 10] = std::array::from_fn(|_| rng.next());
            t.check(&w("record"), 0x005c_8100, Conv::Thiscall(obj), &a10, |g| {
                Out::U32(misc::bridge::network_record_ctor(g, obj, a10))
            });
        }
    }
}

#[test]
fn misc_arrays_b4_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5555_b4b4);
        for i in 0..32 {
            let w = |s: &str| format!("{name} {s} #{i}");
            // buffer slices: offsets around the header, lengths around the size
            let buffer = object(&mut t, &mut rng, 4, false);
            let desc = object(&mut t, &mut rng, 4, false);
            let offset = [0, 3, 4, 5, rng.below(0x100), rng.below(0x10000)][i as usize % 6];
            let len = rng.below(0x10000);
            t.write(desc + 8, &(offset as u16).to_le_bytes());
            t.write(desc + 0xa, &(len as u16).to_le_bytes());
            let size =
                (offset + (len & 0x1fff)).wrapping_add([0, 1, u32::MAX][rng.below(3) as usize]);
            t.w32(buffer, size);
            t.check(
                &w("slice"),
                0x005b_8b00,
                Conv::Thiscall(buffer),
                &[desc],
                |g| Out::U32(misc::bridge::buffer_slice(g, buffer, desc)),
            );
            // record addresses for 0, 1 and many records
            let obj = object(&mut t, &mut rng, 0x10, false);
            let count = [0, 1, 2, 0x8000_0001, rng.next()][i as usize % 5];
            t.w32(obj + 0x20, count);
            let k = rng.below(8);
            t.check(&w("record"), 0x005b_88e0, Conv::Thiscall(obj), &[k], |g| {
                Out::U32(misc::bridge::record_address(g, obj, k))
            });
            // kinds
            // one array longer than a byte's count
            let n = if i == 7 { 300 } else { rng.below(12) };
            let data = object(&mut t, &mut rng, 4 * n.max(1), false);
            let kinds = [
                0x42u8, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44, 0x41, 0xdd, 0x11, 0x45, 0xd8, 0,
            ];
            for e in 0..n {
                let kind = kinds[rng.below(kinds.len() as u32) as usize];
                t.write(data + 16 * e + 2, &[kind]);
            }
            let array = object(&mut t, &mut rng, 2, false);
            t.w32(array, n | if rng.chance(2) { 0x8000_0000 } else { 0 });
            t.w32(array + 4, data);
            t.check(&w("kinds"), 0x005b_52b0, Conv::Cdecl, &[array], |g| {
                Out::U32(misc::bridge::count_kinds(g, array))
            });
            // gather: slots in a base block, objects with 0, 1, many records
            let base = object(&mut t, &mut rng, 0x40, false);
            let n = rng.below(6);
            let entries = object(&mut t, &mut rng, 4 * n.max(1), false);
            for e in 0..n {
                let off = 12 * rng.below(8);
                t.write(entries + 16 * e + 8, &(off as u16).to_le_bytes());
                let o = if rng.chance(3) {
                    0
                } else {
                    let o = object(&mut t, &mut rng, 8, false);
                    t.w32(o + 0x18, [0, 1, 3, 0x8000_0001][rng.below(4) as usize]);
                    o
                };
                t.w32(base + off, o);
            }
            let list = object(&mut t, &mut rng, 2, false);
            t.w32(list, n | if rng.chance(2) { 0x8000_0000 } else { 0 });
            t.w32(list + 4, entries);
            let out = object(&mut t, &mut rng, 8, false);
            t.check(
                &w("gather"),
                0x005b_5370,
                Conv::Cdecl,
                &[base, list, out],
                |g| Out::U32(misc::bridge::gather_records(g, base, list, out)),
            );
        }
    }
}

#[test]
fn misc_find_named_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x6666_b4b4);
        let head = t.rebase(misc::NAMED_LIST);
        let words = [
            "", "a", "ab", "abc", "abd", "position", "positio", "normal", "Normal",
        ];
        for i in 0..40 {
            // names sharing a long prefix with the query
            if i >= 24 {
                let (have, want) = [
                    ("position", "positio"),
                    ("positio", "position"),
                    ("abcdx", "abcdy"),
                ][i as usize % 3];
                let bytes = [have.as_bytes(), &[0]].concat();
                let at = t.alloc(&bytes);
                let node = t.zeros(8);
                t.w32(node, at);
                t.w32(head, node);
                let bytes = [want.as_bytes(), &[0]].concat();
                let q = t.alloc(&bytes);
                t.check(
                    &format!("{name} prefix #{i}"),
                    0x005c_84f0,
                    Conv::Cdecl,
                    &[q],
                    |g| Out::U32(misc::bridge::find_named(g, q)),
                );
                continue;
            }
            // a fresh list of up to 5 names
            let mut next = 0;
            for _ in 0..rng.below(6) {
                let s = words[rng.below(words.len() as u32) as usize];
                let bytes = [s.as_bytes(), &[0]].concat();
                let at = t.alloc(&bytes);
                let node = t.zeros(8);
                t.w32(node, at);
                t.w32(node + 4, next);
                next = node;
            }
            t.w32(head, next);
            let q = if i % 6 == 0 {
                0
            } else {
                let s = words[rng.below(words.len() as u32) as usize];
                let bytes = [s.as_bytes(), &[0]].concat();
                t.alloc(&bytes)
            };
            t.check(
                &format!("{name} find #{i}"),
                0x005c_84f0,
                Conv::Cdecl,
                &[q],
                |g| Out::U32(misc::bridge::find_named(g, q)),
            );
        }
    }
}

#[test]
fn misc_ring_next_segment_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7777_b4b4);
        let ring = t.zeros(20);
        for i in 0..64 {
            let start = 0x1000 * rng.below(8);
            let end = start + rng.below(0x1000);
            let cursor = start + 1 + rng.below(0x1200);
            let pending = if i % 3 == 0 {
                0
            } else {
                start + rng.below(0x1200)
            };
            // a segment that ends exactly at the end, and ones longer
            // than the ring (the restart wraps too)
            let seg = [
                0,
                1,
                end - start,
                rng.below(0x800),
                rng.below(0x2000),
                end.wrapping_sub(if pending == 0 { cursor } else { pending }),
                end - start + 1 + rng.below(0x100),
                // cursor + segment overflows while start + segment does
                // not: the restart's own wrap
                cursor.wrapping_neg() + rng.below(cursor - start + 1),
            ][i as usize % 8];
            for (k, v) in [start, end, cursor, pending, seg].into_iter().enumerate() {
                t.w32(ring + 4 * k as u32, v);
            }
            let addr = t.rebase(0x0061_3eb0);
            t.a.call(addr, Conv::Thiscall(ring), &[]).unwrap();
            misc::bridge::ring_next_segment(&mut G(&mut t.b), ring);
            t.diff(&format!("{name} next #{i}"));
        }
    }
}

#[test]
fn misc_array_set_view_matches_the_original() {
    use misc::bridge::Release;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8888_b5b5);
        let cases = [
            (0x005a_f8d0, Release::Free),
            (0x005b_33c0, Release::Free),
            (0x005a_f920, Release::FreeSet),
            (0x005a_f970, Release::FreeSet),
            (0x005c_7a10, Release::FreeSet),
            (0x005c_7a60, Release::FreeSet),
            (0x005c_79c0, Release::With(0x005c_1ef0)),
        ];
        for (f, release) in cases {
            for i in 0..24 {
                let arr = object(&mut t, &mut rng, 2, false);
                let header = rng.below(50) | if i % 2 == 0 { 0x8000_0000 } else { 0 };
                let old = if i % 3 == 0 { 0 } else { rng.next() };
                t.w32(arr, header);
                t.w32(arr + 4, old);
                let data = if i % 5 == 0 { old } else { rng.next() };
                let count = if i % 7 == 0 {
                    0x8000_0000 | rng.next()
                } else {
                    rng.below(100)
                };
                t.stub(misc::PHYRE_FREE, false, 1, 0, rng.next());
                t.stub(0x005c_1ef0, false, 2, 0, rng.next());
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(arr),
                    &[data, count],
                    |g| Out::U32(misc::bridge::array_set_view(g, release, arr, data, count)),
                );
                t.unstub(misc::PHYRE_FREE);
                t.unstub(0x005c_1ef0);
            }
        }
    }
}

#[test]
fn misc_free_records_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x9999_b5b5);
        for i in 0..24 {
            let n = if i % 4 == 0 { 0 } else { rng.below(6) };
            let records = if i % 6 == 0 {
                0
            } else {
                object(&mut t, &mut rng, 5 * n.max(1), false)
            };
            if records != 0 {
                for k in 0..n {
                    let at = records + 20 * k + 0xc;
                    t.w32(
                        at,
                        rng.below(9) | if rng.chance(2) { 0x8000_0000 } else { 0 },
                    );
                    t.w32(at + 4, if rng.chance(3) { 0 } else { rng.next() });
                }
            }
            t.stub(misc::PHYRE_FREE, false, 1, 0, rng.next());
            let addr = t.rebase(0x005c_1ef0);
            let ra = t.a.call(addr, Conv::Cdecl, &[records, n]).unwrap();
            let rb = misc::bridge::free_records(&mut G(&mut t.b), records, n);
            if records != 0 {
                assert_eq!(ra.eax, rb, "{name} #{i}: eax");
            }
            t.diff(&format!("{name} records #{i}"));
            t.unstub(misc::PHYRE_FREE);
        }
    }
}

#[test]
fn misc_add_named_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xaaaa_b5b5);
        let head = t.rebase(misc::NAMED_LIST);
        let words = ["a", "ab", "position", "normal", "uv0"];
        for i in 0..24 {
            let mut first = 0;
            for _ in 0..rng.below(4) {
                let s = words[rng.below(words.len() as u32) as usize];
                let at = t.alloc(&[s.as_bytes(), &[0]].concat());
                let node = t.zeros(8);
                t.w32(node, at);
                t.w32(node + 4, first);
                first = node;
            }
            t.w32(head, first);
            let node = object(&mut t, &mut rng, 2, false);
            let s = words[rng.below(words.len() as u32) as usize];
            let at = if i % 5 == 0 {
                0
            } else {
                t.alloc(&[s.as_bytes(), &[0]].concat())
            };
            t.w32(node, at);
            t.check(
                &format!("{name} add #{i}"),
                0x005c_8660,
                Conv::Cdecl,
                &[node],
                |g| Out::U32(misc::bridge::add_named(g, node)),
            );
        }
    }
}

/// (original, constructor, vtable, word at `+4`, zeroed words).
type CtorCase = (u32, u32, Option<u32>, Option<u32>, &'static [u32]);

/// The words `+0x90..=+0xa8` the `PGlow` constructors zero.
const GLOW_ZEROS: [u32; 7] = [0xa8, 0x90, 0x9c, 0x94, 0xa0, 0x98, 0xa4];

#[test]
fn misc_derived_ctors_zeroed_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xbbbb_b6b6);
        let cases: [CtorCase; 4] = [
            (
                0x009f_c2b0,
                0x009f_c310,
                Some(0x00b7_55f0),
                Some(0x0194_4180),
                &GLOW_ZEROS,
            ),
            (
                0x009f_c480,
                0x009f_c310,
                Some(0x00b7_54f0),
                None,
                &GLOW_ZEROS,
            ),
            (
                0x009f_c4e0,
                0x009f_c310,
                Some(0x00b7_53e8),
                None,
                &GLOW_ZEROS,
            ),
            (
                0x009f_c650,
                0x009f_c540,
                None,
                None,
                &[0x78, 0x84, 0x7c, 0x88, 0x80, 0x8c],
            ),
        ];
        for (f, base, vt, w4, zeros) in cases {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 0x30, false);
                t.stub_at(base, rng.next());
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(obj),
                    &[],
                    |g| {
                        Out::U32(misc::bridge::derived_ctor_zeroed(
                            g, obj, base, vt, w4, zeros,
                        ))
                    },
                );
                t.unstub(base);
            }
        }
    }
}

#[test]
fn descriptor_construct_with_zeroed_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xcccc_b6b6);
        let dof: &[u32] = &[0xfb0, 0xfb4, 0xfb8, 0xfbc];
        let cases: [CtorCase; 4] = [
            (0x00a0_f910, 0x009f_cbb0, Some(0x00b7_4928), None, &[0xd4c]),
            (
                0x00a0_f850,
                0x009f_cbb0,
                Some(0x00b7_4a30),
                Some(0x0194_3aa8),
                &[0xd4c],
            ),
            (0x00a0_f220, 0x009f_bd90, Some(0x00b7_4590), None, dof),
            (
                0x00a0_f140,
                0x009f_bd90,
                Some(0x00b7_4698),
                Some(0x0194_38e0),
                dof,
            ),
        ];
        for (f, ctor, vt, w4, zeros) in cases {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 0x400, i == 0);
                t.stub_at(ctor, rng.next());
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| {
                        Out::U8(descriptor::bridge::construct_with_zeroed(
                            g, obj, ctor, vt, w4, zeros,
                        ))
                    },
                );
                t.unstub(ctor);
            }
        }
    }
}

#[test]
fn descriptor_construct_ssao_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xdddd_b6b6);
        let cases = [
            (0x00a0_f9e0, 0x00b7_4d60, Some(0x0194_3c70)),
            (0x00a0_fa40, 0x00b7_4b30, None),
            (0x00a0_faa0, 0x00b7_4c48, None),
        ];
        for (f, vt, w4) in cases {
            for i in 0..3 {
                let obj = object(&mut t, &mut rng, 12, i == 0);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U32(descriptor::bridge::construct_ssao(g, obj, vt, w4)),
                );
            }
        }
    }
}

#[test]
fn misc_window_and_text_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xeeee_b7b7);
        let slot = t.rebase(misc::SET_WINDOW_TEXT_SLOT);
        let set_text = t.stub_code(0x0062_cf60, false, 2, 2, 1);
        t.w32(slot, set_text);
        let cache = t.rebase(misc::TEXT_PASS_CACHE);
        for i in 0..8 {
            let app = object(&mut t, &mut rng, 0x100, false);
            let title = rng.next();
            // the assign leaves a string word with or without its flag bit
            let s = rng.next();
            let code = t.stub_code(misc::PSTRING_ASSIGN, true, 1, 1, 0);
            t.jump(misc::PSTRING_ASSIGN, code);
            t.w32(app + 0x398, s);
            t.check(
                &format!("{name} title #{i}"),
                0x0062_cf60,
                Conv::Thiscall(app),
                &[title],
                |g| Out::U32(misc::bridge::set_window_title(g, app, title)),
            );
            t.unstub(misc::PSTRING_ASSIGN);
            let k = rng.below(4);
            t.w32(cache + 4 * k, if i % 2 == 0 { 0 } else { rng.next() });
            t.stub(misc::FIND_RENDER_PASS, false, 1, 0, rng.next());
            t.check(
                &format!("{name} pass #{i}"),
                0x00a3_e9b0,
                Conv::Cdecl,
                &[k],
                |g| Out::U32(misc::bridge::text_render_pass(g, k)),
            );
            t.unstub(misc::FIND_RENDER_PASS);
        }
    }
}

#[test]
fn misc_frame_queue_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0xffff_b7b7);
        for i in 0..20 {
            let thread = object(&mut t, &mut rng, 0x60, false);
            t.w32(thread + 0x154, [0, 1, 14, 15][i % 4]);
            t.stub_at(misc::SEMAPHORE_WAIT, 0);
            t.stub_at(misc::EVENT_SIGNAL, 0);
            let (a, b, c) = (rng.next(), rng.next(), rng.next());
            t.check(
                &format!("{name} queue #{i}"),
                0x0061_5210,
                Conv::Thiscall(thread),
                &[a, b, c],
                |g| Out::U32(misc::bridge::queue_frame(g, thread, a, b, c)),
            );
            // the renderer's submit: the queue stubbed with a result
            let renderer = object(&mut t, &mut rng, 0x20, false);
            if i % 3 == 0 {
                t.w32(renderer + 0x44, 0);
            } else {
                let sem = object(&mut t, &mut rng, 2, false);
                t.w32(renderer + 0x44, sem);
            }
            t.stub(misc::RENDER_THREAD_INSTANCE, false, 0, 0, thread);
            t.stub(
                misc::QUEUE_FRAME,
                true,
                3,
                3,
                if i % 4 == 1 { rng.next() } else { 0 },
            );
            t.stub_at(misc::RING_NEXT_SEGMENT, 0);
            t.check(
                &format!("{name} submit #{i}"),
                0x005b_6c50,
                Conv::Thiscall(renderer),
                &[],
                |g| Out::U32(misc::bridge::renderer_submit_frame(g, renderer)),
            );
            for f in [
                misc::SEMAPHORE_WAIT,
                misc::EVENT_SIGNAL,
                misc::RENDER_THREAD_INSTANCE,
                misc::QUEUE_FRAME,
                misc::RING_NEXT_SEGMENT,
            ] {
                t.unstub(f);
            }
        }
    }
}

// ---- continuation round (batch 8 on): `ffx_prender::p6_more` ----

#[path = "p6_render_c/more.rs"]
mod more;

#[path = "p6_render_c/encode.rs"]
mod encode;

#[path = "p6_render_c/thread.rs"]
mod thread;

// ---- continuation round 2 (batch 11 on): `ffx_prender::p6_small` ----

#[path = "p6_render_c/small.rs"]
mod small;

#[path = "p6_render_c/setup.rs"]
mod setup;

#[path = "p6_render_c/post.rs"]
mod post;

#[path = "p6_render_c/app.rs"]
mod app;

#[path = "p6_render_c/targets.rs"]
mod targets;

#[path = "p6_render_c/late.rs"]
mod late;

// ---- continuation round 4: `ffx_prender::p6_quads` ----

#[path = "p6_render_c/quads.rs"]
mod quads;

#[path = "p6_render_c/tail.rs"]
mod tail;
