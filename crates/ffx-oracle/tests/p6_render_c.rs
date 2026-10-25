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
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::descriptor::{self, family};

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
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
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
    assert_eq!(seen, 102 + 79);
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
                let code = t.stub_regs(ctor, r);
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
                    let code = t.stub_regs(d, r);
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
