//! The render layer (`ffx_graphic`) against the original: on the image
//! alone with synthetic objects (a render-data table, fake geometry with
//! recording virtual methods, a scene, managers, the model cache), and on
//! a snapshot of the running game (`FFX_SNAPSHOT`): the real scene, the
//! real effect table and model cache. Results and every byte written must
//! agree. Needs `FFX_GAME_DIR`; the snapshot tests also `FFX_SNAPSHOT`.
//!
//! Callees that stay original (`PhyreEngine`, other modules) are replaced
//! in both oracles alike by recording stubs: each call appends its tag,
//! `this` and arguments to a log in memory, so the diff checks that both
//! made the same calls in the same order.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_graphic::{cache, managers, scene, ui, vfx};
use ffx_oracle::{Conv, Oracle, Pages};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;
/// After the stubs: `fld qword [esp+4]; call [esp+0xc]; ret` (an intrinsic
/// with its argument in ST0), then at `+16` `fstp st(0); ret`.
const TRAMPOLINE: u32 = CAVE + CAVE_LEN;
const TRAMPOLINE_CODE: [u8; 19] = [
    0xdd, 0x44, 0x24, 0x04, 0xff, 0x54, 0x24, 0x0c, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
    0xdd, 0xd8, 0xc3,
];

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
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// A float that is not a signalling NaN (the emulator keeps those).
    fn float(&mut self) -> f32 {
        loop {
            let bits = match self.below(4) {
                0 => self.next(),
                1 => (self.below(4000) as f32 - 2000.0).to_bits(),
                2 => (self.next() as f32 / 1000.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    1,
                    0x7f7f_ffff,
                ][self.below(7) as usize],
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return f32::from_bits(bits);
            }
        }
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
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0.call(addr, conv, args).unwrap().eax
    }
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        let r = self.0.call(addr, conv, args).unwrap();
        let (Some(v), Some(raw)) = (r.st0, r.st0_raw) else {
            panic!("call_st0 {addr:#x}: no float in ST0");
        };
        // pop the result, as the caller's fstp does
        let pop = self.0.rebase(TRAMPOLINE) + 16;
        self.0.call(pop, Conv::Cdecl, &[]).unwrap();
        // as `fstp m32`: a NaN keeps its sign and top fraction bits
        let bits = if v.is_nan() {
            let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
            let sign = u32::from(raw[9] >> 7) << 31;
            sign | 0x7fc0_0000 | ((mant >> 40) as u32 & 0x007f_ffff)
        } else {
            ffx_bridge::x87::store(v)
        };
        f32::from_bits(bits)
    }
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let at = self.0.rebase(TRAMPOLINE);
        let b = x.to_bits();
        let r = self
            .0
            .call(at, Conv::Cdecl, &[b as u32, (b >> 32) as u32, addr])
            .unwrap();
        let (Some(v), Some(raw)) = (r.st0, r.st0_raw) else {
            panic!("call_intrinsic {addr:#x}: no float in ST0");
        };
        self.0.call(at + 16, Conv::Cdecl, &[]).unwrap();
        let bits = if v.is_nan() {
            let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
            let sign = u32::from(raw[9] >> 7) << 31;
            sign | 0x7fc0_0000 | ((mant >> 40) as u32 & 0x007f_ffff)
        } else {
            ffx_bridge::x87::store(v)
        };
        f32::from_bits(bits)
    }
    fn scratch(&mut self) -> u32 {
        // the bottom of the emulated stack: no call reaches that deep, and
        // the diff leaves the stacks out, as the originals' frames are
        self.0.stack_range().start + 0x100
    }
}

/// What a function returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al` (the rest of `eax` is the caller's).
    U8(u8),
    /// `ax`.
    U16(u16),
    /// `eax` and `edx`.
    U64(u64),
    /// `ST0`.
    F32(f32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 8 words (tag, `this`, six
    /// arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
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
        let at = t.rebase(TRAMPOLINE);
        t.write(at, &TRAMPOLINE_CODE);
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
        ))
    }

    fn snapshot() -> Option<Self> {
        let dir = game_dir()?;
        let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
            eprintln!("FFX_SNAPSHOT is not set; skipping");
            return None;
        };
        let exe = dir.join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        Some(Self::new(
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ))
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

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    fn r32(&self, at: u32) -> u32 {
        Mem::read(&self.b, at)
    }

    fn r8(&self, at: u32) -> u8 {
        Mem::read(&self.b, at)
    }

    /// Sets the global at `sym`.
    fn global(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    fn reset(&mut self) {
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// Writes a recording stub: logs `tag`, `ecx` when `this`, and `n`
    /// stack arguments; pops `pops` words; returns `ret`. Placed at `at`
    /// when given (a `jmp` there), else only in the cave. Returns its
    /// address.
    fn stub(&mut self, at: Option<u32>, tag: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
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
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        if let Some(at) = at {
            let mut j = vec![0xe9];
            j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
            self.write(at, &j);
        }
        addr
    }

    /// A recording stub over the original at `sym`.
    fn stub_sym(&mut self, sym: u32, conv: Conv, n: u32, ret: u32) {
        let at = self.rebase(sym);
        let (this, pops) = match conv {
            Conv::Cdecl => (false, 0),
            Conv::Stdcall => (false, n),
            Conv::Thiscall(_) => (true, n),
            Conv::Fastcall => unreachable!("no fastcall stubs"),
        };
        self.stub(Some(at), sym, this, n, pops, ret);
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }

    /// Runs the original at `sym` in `a` and `bridge` on `b`; compares the
    /// result as `bridge` says and every byte written.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
    ) {
        self.check_with(what, sym, conv, args, bridge, |_| {});
    }

    /// As [`Twins::check`], with `fixup` run on both before the diff (for
    /// bytes the original leaves undefined).
    fn check_with(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        bridge: impl FnOnce(&mut G) -> Out,
        fixup: impl FnOnce(&mut Self),
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = bridge(&mut G(&mut self.b));
        // an original that leaves a float in ST0 needs a port returning one
        assert!(
            matches!(out, Out::F32(_)) || ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?}); the port returns {out:?}",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
            Out::U16(v) => assert_eq!(ra.eax & 0xffff, u32::from(v), "{what}: ax"),
            Out::U64(v) => assert_eq!(
                u64::from(ra.edx) << 32 | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
            Out::F32(v) => {
                let st0 = ra.st0.expect("a float in ST0");
                let ours = f64::from(v);
                assert!(
                    ffx_oracle::same_float(st0, ours),
                    "{what}: st0 {st0} vs {ours}"
                );
            }
        }
        fixup(self);
        self.diff(what);
    }

    /// A recording stub for a `thiscall (matrix*, a, b)` method: logs the
    /// tag, `this`, `a` and `b` in one entry and the 16 words the matrix
    /// pointer points at in the next two (the pointer itself differs: the
    /// original passes its stack, the bridge its scratch).
    fn stub_matrix(&mut self, at: u32, tag: u32, ret: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x08]); // [eax+8] = a
        c.extend([0x8b, 0x54, 0x24, 0x0c, 0x89, 0x50, 0x0c]); // [eax+0xc] = b
        c.extend([0x56, 0x57]); // push esi; push edi
        c.extend([0x8b, 0x74, 0x24, 0x0c]); // mov esi, [esp+0xc] (the matrix)
        c.extend([0x8d, 0x78, 0x20]); // lea edi, [eax+0x20]
        c.extend([0xb9, 0x10, 0, 0, 0]); // mov ecx, 16
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.extend([0x83, 0x05]);
        c.extend(log.to_le_bytes());
        c.push(3); // add dword [log], 3
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }
}

// ---- the render-data table ----

/// Links sorted `(key, node, geometry)`s as a balanced tree under
/// `parent`; returns its root (`head` for none).
fn link(t: &mut Twins, nodes: &[(u64, u32, u32)], head: u32, parent: u32) -> u32 {
    if nodes.is_empty() {
        return head;
    }
    let mid = nodes.len() / 2;
    let n = nodes[mid].1;
    let l = link(t, &nodes[..mid], head, n);
    let r = link(t, &nodes[mid + 1..], head, n);
    t.w32(n, l);
    t.w32(n + 4, parent);
    t.w32(n + 8, r);
    n
}

/// A UI remap: its original and ours.
type Remap = (u32, fn(f32) -> f32);

/// A minimap viewport getter: ours.
type Vp = fn(&G, u32) -> u32;

/// A table whose map holds `keys` (each with a fresh geometry), built as
/// a balanced search tree under the head the way `std::map` links it
/// (absent children point at the head). Returns `(table, [(key,
/// geometry)])`.
fn build_table(t: &mut Twins, keys: &[u64], rng: &mut Rng) -> (u32, Vec<(u64, u32)>) {
    let mut keys = keys.to_vec();
    keys.sort_unstable();
    keys.dedup();
    let head = t.zeros(0x20);
    t.w8(head + 0xd, 1);
    let mut nodes = Vec::new();
    for &k in &keys {
        let n = t.zeros(0x20);
        t.write(n + 0x10, &k.to_le_bytes());
        let g = t.zeros(0x60);
        t.w32(n + 0x18, g);
        nodes.push((k, n, g));
    }
    let root = link(t, &nodes, head, head);
    t.w32(head + 4, root);
    let table = t.zeros(0x40);
    t.w32(table, head);
    // a stale cache
    if rng.chance(2) && !nodes.is_empty() {
        let (k, _, g) = nodes[rng.below(nodes.len() as u32) as usize];
        t.write(table + 0x20, &k.to_le_bytes());
        t.w32(table + 0x28, g);
    }
    let out = nodes.iter().map(|&(k, _, g)| (k, g)).collect();
    (table, out)
}

fn split(k: u64) -> (u32, u32) {
    (k as u32, (k >> 32) as u32)
}

const MAP_FIND: u32 = 0x006a_1f60;
const GET_DYNAMIC_GEOMETRY: u32 = 0x0069_f620;
const GET_VFX_DATA: u32 = 0x0069_f6e0;
const GET_RENDER_DATA_BY_INSTANCE: u32 = 0x0064_53d0;
const DYNAMIC_GEOMETRY_UPDATE_TEXTURE: u32 = 0x0064_0060;
const VFX_UPDATE: u32 = 0x0064_58a0;
const VFX_UPDATE_WITH_TYPE: u32 = 0x0064_5ba0;
const VFX_PATH3_UPDATE: u32 = 0x0064_5600;
const VFX_IS_TEXTURE_LOADED: u32 = 0x0064_5400;
const INIT_RENDER_DATA: u32 = 0x0071_1180;
const INIT_PATH3_RENDER_DATA: u32 = 0x0071_0fd0;
const INIT_RENDER_DATA_SHP: u32 = 0x0071_1460;
const INIT_PATH3_RENDER_DATA_SHP: u32 = 0x0071_1030;
const LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;

#[test]
fn render_data_table_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1357_9bdf);
    for round in 0..60 {
        t.reset();
        let n = rng.below(40) as usize;
        let keys: Vec<u64> = (0..n)
            .map(|_| {
                if rng.chance(2) {
                    u64::from(rng.below(50)) * 10_000 + u64::from(rng.below(3))
                } else {
                    u64::from(rng.next()) << 32 | u64::from(rng.next())
                }
            })
            .collect();
        let (table, entries) = build_table(&mut t, &keys, &mut rng);
        let out = t.zeros(4);
        let kp = t.zeros(8);
        for i in 0..30 {
            let k = if !entries.is_empty() && rng.chance(2) {
                entries[rng.below(entries.len() as u32) as usize].0
            } else if rng.chance(3) {
                0
            } else {
                u64::from(rng.below(50)) * 10_000 + u64::from(rng.below(3))
            };
            let (lo, hi) = split(k);
            let what = format!("round {round} lookup {i} key {k:#x}");
            t.write(kp, &k.to_le_bytes());
            t.check(&what, MAP_FIND, Conv::Thiscall(table), &[out, kp], |g| {
                Out::U32(vfx::bridge::map_find_out(g, table, out, kp))
            });
            t.check(
                &what,
                GET_DYNAMIC_GEOMETRY,
                Conv::Thiscall(table),
                &[lo, hi],
                |g| Out::U32(vfx::bridge::get_dynamic_geometry(g, table, lo, hi)),
            );
            // geometry +0x24: the render data
            for &(_, geo) in &entries {
                let v = rng.next();
                t.w32(geo + 0x24, v);
            }
            t.check(&what, GET_VFX_DATA, Conv::Thiscall(table), &[lo, hi], |g| {
                Out::U32(vfx::bridge::get_vfx_data(g, table, lo, hi))
            });
            t.global(vfx::bridge::TABLE.sym, table);
            t.check(
                &what,
                GET_RENDER_DATA_BY_INSTANCE,
                Conv::Cdecl,
                &[lo, hi],
                |g| Out::U32(vfx::bridge::get_render_data_by_instance(g, lo, hi)),
            );
        }
    }
}

/// A fake effect: a table with `parts` geometries (keys `key + i * 10000`),
/// each with a vtable whose `+8` and `+0xc` are recording stubs, and the
/// first one's render data.
struct Effect {
    key: u64,
    data: u32,
}

fn build_effect(t: &mut Twins, rng: &mut Rng) -> Effect {
    let key = if rng.chance(8) {
        // a negative part offset wraps the low word
        0xffff_ff00 + u64::from(rng.below(0x100))
    } else {
        u64::from(rng.next()) | u64::from(rng.below(3)) << 32
    };
    let parts = 1 + rng.below(6);
    let mut keys: Vec<u64> = (0..parts)
        .filter(|_| !rng.chance(5))
        .map(|i| vfx::part_key(key, i as i32))
        .collect();
    if rng.chance(4) {
        keys.push(key);
    }
    let (table, entries) = build_table(t, &keys, rng);
    t.global(vfx::bridge::TABLE.sym, table);
    let update = t.stub(None, 0x0008, true, 1, 1, rng.next());
    let texture = t.stub(None, 0x000c, true, 0, 0, rng.next());
    let vtable = t.alloc(&[0; 0x20]);
    t.w32(vtable + 8, update);
    t.w32(vtable + 12, texture);
    let mut data = 0;
    for (k, g) in entries {
        t.w32(g, vtable);
        t.w8(g + 0x51, u8::from(rng.chance(2)));
        if k == key && !rng.chance(10) {
            data = t.zeros(0x100);
            t.w8(data + 0xc0, u8::from(!rng.chance(3)));
            t.w8(data + 0xc1, u8::from(rng.chance(2)));
            let count = if rng.chance(10) {
                rng.below(0x10000)
            } else {
                rng.below(7)
            };
            t.write(data + 0xc4, &(count as u16).to_le_bytes());
            let mask = if rng.chance(3) { u32::MAX } else { rng.next() };
            t.w32(data + 0xc8, mask);
            t.w32(g + 0x24, data);
        }
    }
    Effect { key, data }
}

/// A fake scene with a level hash, as `g_PhyreScene`.
fn build_scene(t: &mut Twins, rng: &mut Rng) -> u32 {
    let bytes = rng.bytes(0x12400);
    let scene = t.alloc(&bytes);
    let hash = if rng.chance(2) {
        vfx::FILTERED_LEVELS[rng.below(21) as usize]
    } else {
        rng.below(0x3000)
    };
    t.w32(scene + 0xfaf4, hash);
    t.global(scene::bridge::SCENE.sym, scene);
    scene
}

#[test]
fn vfx_updates_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2468_ace0);
    for round in 0..300 {
        t.reset();
        build_scene(&mut t, &mut rng);
        let e = build_effect(&mut t, &mut rng);
        t.global(vfx::bridge::SKIP_FRAME.sym, u32::from(rng.chance(8)));
        t.global(
            vfx::bridge::LEVEL_FILTER.sym,
            [0, 1, 1, 2][rng.below(4) as usize],
        );
        let (lo, hi) = if rng.chance(30) { (0, 0) } else { split(e.key) };
        // the original faults on a missing first geometry under the filter
        let level = t.r32(t.r32(t.rebase(scene::bridge::SCENE.sym)) + 0xfaf4);
        let filter =
            t.r8(t.rebase(vfx::bridge::LEVEL_FILTER.sym)) == 1 && vfx::level_filtered(level);
        let table = t.r32(t.rebase(vfx::bridge::TABLE.sym));
        let first = vfx::bridge::map_find(&t.b, table, e.key);
        if filter && first == t.r32(table) {
            continue;
        }
        let what = format!("round {round} key {:#x} data {:#x}", e.key, e.data);
        match round % 3 {
            0 => t.check(&what, VFX_UPDATE, Conv::Cdecl, &[lo, hi], |g| {
                vfx::bridge::update(g, lo, hi);
                Out::None
            }),
            1 => t.check(&what, VFX_PATH3_UPDATE, Conv::Cdecl, &[lo, hi], |g| {
                vfx::bridge::path3_update(g, lo, hi);
                Out::None
            }),
            _ => {
                let flags = rng.next();
                t.check(
                    &what,
                    VFX_UPDATE_WITH_TYPE,
                    Conv::Cdecl,
                    &[lo, hi, flags],
                    |g| {
                        vfx::bridge::update_with_type(g, lo, hi, flags);
                        Out::None
                    },
                );
            }
        }
        t.check(
            &what,
            DYNAMIC_GEOMETRY_UPDATE_TEXTURE,
            Conv::Cdecl,
            &[lo, hi],
            |g| Out::U32(vfx::bridge::dynamic_geometry_update_texture(g, lo, hi)),
        );
    }
}

#[test]
fn vfx_texture_loaded_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x0bad_cafe);
    for round in 0..40 {
        t.reset();
        build_scene(&mut t, &mut rng);
        let r = rng.below(4);
        t.stub_sym(
            vfx::bridge::SCENE_IS_VFX_TEXTURE_LOADED,
            Conv::Thiscall(0),
            6,
            r,
        );
        let manager = rng.next();
        t.stub_sym(
            vfx::bridge::CLUSTER_MANAGER_GET_INSTANCE,
            Conv::Cdecl,
            0,
            manager,
        );
        let cluster = if rng.chance(2) { 0 } else { rng.next() };
        t.stub_sym(
            vfx::bridge::CLUSTER_MANAGER_GET_CLUSTER,
            Conv::Thiscall(0),
            1,
            cluster,
        );
        let args: [u32; 6] = std::array::from_fn(|_| rng.next());
        t.check(
            &format!("round {round}"),
            VFX_IS_TEXTURE_LOADED,
            Conv::Cdecl,
            &args,
            |g| Out::U32(vfx::bridge::is_texture_loaded(g, args)),
        );
    }
}

#[test]
fn vfx_init_render_data_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x7777_1234);
    for round in 0..200 {
        t.reset();
        build_scene(&mut t, &mut rng);
        t.global(vfx::bridge::SKIP_FRAME.sym, 0);
        t.global(vfx::bridge::LEVEL_FILTER.sym, 0);
        let e = build_effect(&mut t, &mut rng);
        if e.data != 0 {
            let n = rng.below(5);
            t.w32(e.data + 0x90, if rng.chance(10) { u32::MAX } else { n });
            let segs: Vec<u8> = rng.bytes(0x6c * 5);
            let s = t.alloc(&segs);
            t.w32(e.data + 0x94, s);
        }
        let shp = round % 2 == 1;
        let created = u32::from(!rng.chance(4));
        let set_up = u32::from(!rng.chance(4));
        let create = if shp {
            vfx::bridge::CREATE_BUFFER_SHP
        } else {
            vfx::bridge::CREATE_BUFFER
        };
        t.stub_sym(create, Conv::Cdecl, 5, created);
        t.stub_sym(vfx::bridge::SETUP, Conv::Cdecl, 2, set_up);
        t.stub_sym(vfx::bridge::UPDATE_RENDER_DATA, Conv::Cdecl, 4, rng.next());
        t.stub_sym(
            vfx::bridge::UPDATE_RENDER_DATA_SHP,
            Conv::Cdecl,
            5,
            rng.next(),
        );
        t.stub_sym(vfx::bridge::DESTROY, Conv::Cdecl, 2, rng.next());
        let (lo, hi) = split(e.key);
        let path3 = rng.chance(2);
        let what = format!("round {round} shp {shp} path3 {path3}");
        if shp {
            let args = [lo, hi, rng.next(), rng.next(), rng.next(), rng.next()];
            let sym = if path3 {
                INIT_PATH3_RENDER_DATA_SHP
            } else {
                INIT_RENDER_DATA_SHP
            };
            t.check(&what, sym, Conv::Cdecl, &args, |g| {
                vfx::bridge::init_render_data_shp(g, args, path3);
                Out::None
            });
        } else {
            let args = [lo, hi, rng.next(), rng.next(), rng.next()];
            let sym = if path3 {
                INIT_PATH3_RENDER_DATA
            } else {
                INIT_RENDER_DATA
            };
            t.check(&what, sym, Conv::Cdecl, &args, |g| {
                vfx::bridge::init_render_data(g, args, path3);
                Out::None
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per wrapper
fn vfx_wrappers_match_the_original() {
    use vfx::bridge as v;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x0dd_ba11);
    for round in 0..300 {
        t.reset();
        build_scene(&mut t, &mut rng);
        t.global(v::SKIP_FRAME.sym, u32::from(rng.chance(4)));
        t.global(v::LEVEL_FILTER.sym, 0);
        let e = build_effect(&mut t, &mut rng);
        // the geometry's mesh instance, for the name
        let table = t.r32(t.rebase(v::TABLE.sym));
        let first = v::map_find(&t.b, table, e.key);
        if first != t.r32(table) {
            let g = t.r32(first + 0x18);
            let mesh = t.zeros(0x40);
            t.w32(g + 0xc, mesh);
            // the vtable's +0x28 for graphicVFXUpdateParameterOnly
            let vt = t.r32(g);
            let m28 = t.stub(None, 0x28, true, 1, 1, rng.next());
            t.w32(vt + 0x28, m28);
        }
        for (sym, n) in [
            (v::SHOW, 0),
            (v::HIDE, 0),
            (v::SET_YIQ_ENABLE, 1),
            (v::SET_POINT_LIGHT, 3),
            (v::SET_CLUT, 3),
            (v::SET_Z_CORRECTION, 1),
            (v::SET_UV_OFFSET, 1),
            (v::SET_RGBA_POWER, 1),
            (v::PSTRING_ASSIGN, 1),
            (v::SCENE_SYNC_PATH, 0),
            (v::TABLE_CLEAR, 0),
            (v::TABLE_CLEAR_BY_TYPE, 1),
            (v::TABLE_REMOVE_DELETED, 0),
            (v::TABLE_UPDATE_DELETE_POOL, 0),
            (v::SCENE_LOAD_PERSISTENT_TEXTURE, 0),
            (v::SCENE_LOAD_MAP_TEXTURE, 0),
            (v::SCENE_UNLOAD_MAP_TEXTURE, 0),
            (v::SCENE_RELEASE_VFX, 0),
            (v::SHADER_LOAD_SPECIAL, 0),
            (v::SHADER_UNLOAD_SPECIAL, 0),
        ] {
            t.stub_sym(sym, Conv::Thiscall(0), n, rng.next());
        }
        t.stub_sym(v::SHADER_MANAGER, Conv::Cdecl, 0, rng.next());
        let (lo, hi) = if rng.chance(20) { (0, 0) } else { split(e.key) };
        let (a, b) = (rng.next(), rng.next());
        let fl = rng.float();
        let what = format!("round {round} key {:#x}", e.key);
        let missing = first == t.r32(table);
        match round % 17 {
            0 => t.check(&what, 0x0064_4bc0, Conv::Cdecl, &[lo, hi], |g| {
                v::update_2d(g, lo, hi);
                Out::None
            }),
            1 => t.check(&what, 0x0064_5330, Conv::Cdecl, &[lo, hi], |g| {
                v::update_dtt(g, lo, hi);
                Out::None
            }),
            2 => t.check(&what, 0x0064_5380, Conv::Cdecl, &[lo, hi], |g| {
                Out::U32(v::get_dynamic_geometry_by_instance(g, lo, hi))
            }),
            3 => {
                let vis = rng.below(3);
                t.check(&what, 0x0064_5710, Conv::Cdecl, &[lo, hi, vis], |g| {
                    Out::U32(v::set_visible(g, lo, hi, vis))
                });
            }
            4 => t.check(&what, 0x0064_5350, Conv::Cdecl, &[lo, hi], |g| {
                Out::U32(v::enable_yiq(g, lo, hi))
            }),
            5 => t.check(
                &what,
                0x0064_56d0,
                Conv::Cdecl,
                &[lo, hi, a, b, fl.to_bits()],
                |g| Out::U32(v::set_parameters(g, v::SET_POINT_LIGHT, lo, hi, a, b, fl)),
            ),
            6 => t.check(
                &what,
                0x0064_5630,
                Conv::Cdecl,
                &[lo, hi, a, b, fl.to_bits()],
                |g| Out::U32(v::set_parameters(g, v::SET_CLUT, lo, hi, a, b, fl)),
            ),
            7 => t.check(
                &what,
                0x0064_5750,
                Conv::Cdecl,
                &[lo, hi, fl.to_bits()],
                |g| Out::U32(v::set_z_correction(g, lo, hi, fl)),
            ),
            8 if !missing || lo | hi == 0 => {
                t.check(&what, 0x0064_56a0, Conv::Cdecl, &[lo, hi, a], |g| {
                    Out::U32(v::set_mesh_instance_name(g, lo, hi, a))
                });
            }
            9 => {
                let geo = if missing || rng.chance(5) {
                    0
                } else {
                    t.r32(first + 0x18)
                };
                let uv = if rng.chance(2) { 0 } else { a };
                let rgba = if rng.chance(2) { 0 } else { b };
                t.check(
                    &what,
                    0x0064_5a50,
                    Conv::Cdecl,
                    &[geo, fl.to_bits(), uv, rgba],
                    |g| {
                        v::update_parameter_only(g, geo, fl, uv, rgba);
                        Out::None
                    },
                );
            }
            10 => t.check(&what, 0x0064_5840, Conv::Cdecl, &[], |g| {
                v::sync_path(g);
                Out::None
            }),
            11 => t.check(&what, 0x0064_5310, Conv::Cdecl, &[], |g| {
                Out::U32(v::table_call(g, v::TABLE_CLEAR))
            }),
            12 => t.check(&what, 0x0064_5320, Conv::Cdecl, &[], |g| {
                Out::U32(v::destroy_all_except_field_and_eternal(g))
            }),
            13 => t.check(&what, 0x0064_5620, Conv::Cdecl, &[], |g| {
                Out::U32(v::table_call(g, v::TABLE_REMOVE_DELETED))
            }),
            14 => t.check(&what, 0x0064_5a40, Conv::Cdecl, &[], |g| {
                Out::U32(v::table_call(g, v::TABLE_UPDATE_DELETE_POOL))
            }),
            15 => t.check(&what, 0x0064_54c0, Conv::Cdecl, &[], |g| {
                Out::U32(v::load_persistent_texture(g))
            }),
            _ => {
                t.check(&what, 0x0064_54a0, Conv::Cdecl, &[], |g| {
                    Out::U32(v::load_level_data(g))
                });
                t.check(&what, 0x0064_5860, Conv::Cdecl, &[], |g| {
                    Out::U32(v::unload_level_data(g))
                });
            }
        }
    }
}

/// Sane floats for matrices (the originals invert and multiply them).
fn matrix_bytes(rng: &mut Rng) -> Vec<u8> {
    (0..16)
        .flat_map(|i| {
            let v = if i % 5 == 0 { 1.0 } else { 0.0 } + (rng.below(2000) as f32 - 1000.0) / 997.0;
            v.to_le_bytes()
        })
        .collect()
}

const UPDATE_RENDER_DATA: u32 = 0x0071_8610;

#[test]
#[allow(clippy::too_many_lines)] // one set-up of every input
fn vfx_update_render_data_matches_the_original() {
    use ffx_graphic::render_data::bridge as rdb;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x0c0f_fee5);
    for round in 0..600 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        // the instance: up to four parts, each with its render data
        let key = u64::from(rng.next()) | u64::from(rng.below(2)) << 32;
        let kind: u8 = [0, 1, 2, 2][rng.below(4) as usize];
        let count: u16 = match kind {
            1 => 4,
            2 => rng.below(5) as u16,
            _ => 1,
        };
        let parts = if kind == 1 { 4 } else { count.max(1) };
        let mut keys: Vec<u64> = (0..i32::from(parts))
            .map(|p| vfx::part_key(key, p))
            .collect();
        if rng.chance(15) {
            keys.pop();
        }
        let (table, entries) = build_table(&mut t, &keys, &mut rng);
        t.global(vfx::bridge::TABLE.sym, table);
        let yiq = t.stub(None, 0x0a20, true, 1, 1, rng.next());
        let vtable = t.zeros(0x40);
        let _ = yiq;
        for (k, g) in &entries {
            t.w32(*g, vtable);
            if rng.chance(30) {
                continue; // no render data
            }
            let rd = t.alloc(&rng.bytes(0x100));
            t.w8(rd + 0xc0, kind);
            t.write(rd + 0xc4, &count.to_le_bytes());
            // 29 segments with a buffer each
            let mut segs = rng.bytes(0x6c * 30);
            for s in 0..30 {
                let b = 0x6c * s;
                let n = rng.below(64);
                segs[b + 4..b + 8].copy_from_slice(&rng.below(64).to_le_bytes());
                segs[b + 8..b + 12].copy_from_slice(&n.to_le_bytes());
                let dst = t.alloc(&rng.bytes(0x100)).to_le_bytes();
                segs[b + 0x1c..b + 0x20].copy_from_slice(&dst);
                // the texture name the original compares with "RWTT"
                let name: &[u8] = [&b"RWTT\0"[..], b"RWT\0", b"tex0\0"][rng.below(3) as usize];
                let name = t.alloc(name).to_le_bytes();
                segs[b + 0x20..b + 0x24].copy_from_slice(&name);
            }
            let segs = t.alloc(&segs);
            t.w32(rd + 0x94, segs);
            t.w32(*g + 0x24, rd);
            let _ = k;
        }
        // the shape: headers of up to 6 segments
        let shape = t.zeros(0x8000);
        t.w32(shape + 4, 0x40);
        t.w32(shape + 8, rng.below(0x100));
        t.w32(shape + 0xc, rng.below(0x100));
        let mut at = shape + 0x40;
        let mut total = 0i32;
        let textures = [0x3780, 0x3800, 0x3880, 0x3400, 0, 0x1234];
        for s in 0..=rng.below(6) {
            let ty = if rng.chance(12) {
                [8u8, 0xff][rng.below(2) as usize]
            } else {
                rng.below(8) as u8
            };
            let n = rng.below(20) as i16;
            t.w8(at + 1, ty);
            t.write(at + 2, &n.to_le_bytes());
            let tex = if s == 0 {
                textures[rng.below(6) as usize]
            } else {
                rng.next()
            };
            t.w32(at + 8, tex | (rng.next() & 0xc000));
            total += i32::from(n);
            let stride =
                ffx_graphic::render_data::segment_type(i32::from(ty as i8)).map_or(0, |s| s.stride);
            at += 0x10 + u32::try_from(n).unwrap() * stride;
        }
        if rng.chance(10) {
            total += rng.below(40) as i32 - 20;
        }
        t.write(shape + 0x10, &(total as i16).to_le_bytes());
        let part = t.zeros(0x40);
        t.w32(part + 0x1c, shape);
        // the particle object
        let mut o = rng.bytes(0x60);
        o[0] = if rng.chance(2) { 0x40 } else { o[0] & !0x40 };
        o[8] = if rng.chance(2) { 0x48 } else { o[8] };
        let obj = t.alloc(&o);
        for field in [0x20, 0x24, 0x28, 0x2c] {
            let m = matrix_bytes(&mut rng);
            let p = t.alloc(&m);
            t.w32(obj + field, p);
        }
        for field in [0x38, 0x3c] {
            let v = if rng.chance(2) { 0 } else { rng.next() };
            t.w32(obj + field, v);
        }
        let p44 = if rng.chance(2) {
            0
        } else {
            t.alloc(&rng.bytes(0x40))
        };
        t.w32(obj + 0x44, p44);
        // the globals
        let magic = [0x176, 0x66, 0x67, 0x203, 0x29b, 0x207, 0x207, 5][rng.below(8) as usize];
        t.global(rdb::MAGIC_ID.sym, magic);
        t.global(
            rdb::PPP_CURRENT_PROGRAM.sym,
            if rng.chance(2) { 0xb } else { 3 },
        );
        t.global(rdb::VFX_PHASE.sym, if rng.chance(2) { 0x1f } else { 2 });
        t.global(
            rdb::YIQ_MODE.sym,
            if rng.chance(2) {
                u32::MAX
            } else {
                rng.below(9)
            },
        );
        t.global(
            rdb::CURRENT_MAP.sym,
            if rng.chance(2) { 0x188 } else { 0x41 },
        );
        t.global(rdb::VFX_PASS.sym, rng.next());
        let fm = rng.bytes(0x40);
        let at = t.rebase(rdb::FILL_MATRIX.sym);
        t.write(at, &fm);
        let cs: Vec<u8> = (0..4).flat_map(|_| rng.float().to_le_bytes()).collect();
        let at = t.rebase(rdb::COLOR_SCALE.sym);
        t.write(at, &cs);
        t.global(vfx::bridge::SKIP_FRAME.sym, u32::from(rng.chance(30)));
        let fill = t.alloc(&[u8::from(!rng.chance(3)), 0, 0, 0]);
        t.global(managers::bridge::DYN_GEO_FILL_BUFFER.sym, fill);
        // the callees
        let buffer = t.zeros(0x10);
        t.w32(buffer + 8, if rng.chance(2) { shape } else { rng.next() });
        let buffer = if rng.chance(4) { 0 } else { buffer };
        t.stub_sym(rdb::MODEL_BUFFER_GET, Conv::Cdecl, 1, buffer);
        t.stub_sym(rdb::MODEL_BUFFER_RELEASE, Conv::Cdecl, 1, rng.next());
        t.stub_sym(rdb::MODEL_BUFFER_ALLOCATE, Conv::Cdecl, 1, rng.next());
        let req = t.alloc(&rng.bytes(0x220));
        t.stub_sym(rdb::FILL_REQUEST, Conv::Cdecl, 1, req);
        let ok = u32::from(!rng.chance(20));
        t.stub_sym(rdb::BUFFER_FOR_SEGMENT, Conv::Cdecl, 4, ok);
        t.stub_sym(
            vfx::bridge::SET_YIQ_ENABLE,
            Conv::Thiscall(0),
            1,
            rng.next(),
        );
        let pl = t.rebase(vfx::bridge::SET_POINT_LIGHT);
        t.stub_matrix(pl, 0x2fd0, rng.next());
        let (lo, hi) = split(key);
        let what = format!("round {round} kind {kind} count {count} magic {magic:#x}");
        // a kind-2 effect of fewer than four parts leaves the request's
        // other part words to the original's uninitialised frame
        let undefined = kind == 2 && count < 4;
        t.check_with(
            &what,
            UPDATE_RENDER_DATA,
            Conv::Cdecl,
            &[lo, hi, part, obj],
            |g| Out::U32(rdb::update(g, lo, hi, part, obj)),
            |t| {
                if undefined {
                    for base in [0xc0, 0xd0, 0xe0, 0xf0] {
                        for p in u32::from(count)..4 {
                            t.w32(req + base + 4 * p, 0);
                        }
                    }
                }
            },
        );
    }
}

const UPDATE_RENDER_DATA_SHP: u32 = 0x0071_b8a0;

#[test]
#[allow(clippy::too_many_lines)] // one set-up of every input
fn vfx_update_render_data_shp_matches_the_original() {
    use ffx_graphic::render_data::bridge as rdb;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x05ba_9e11);
    let kinds = [0x48u8, 0x42, 0x46, 0x41, 0x88, 0x00, 0x13];
    for round in 0..600 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let key = u64::from(rng.next()) | u64::from(rng.below(2)) << 32;
        let kind: u8 = if rng.chance(3) {
            0
        } else {
            1 + rng.below(2) as u8
        };
        // the shape's primitives and the parts they name
        let n = if rng.chance(8) {
            -(rng.below(3) as i16)
        } else {
            rng.below(12) as i16
        };
        let prims: Vec<u8> = (0..n.max(0))
            .map(|_| kinds[rng.below(7) as usize])
            .collect();
        let used: Vec<u32> = prims.iter().map(|&k| rdb::shape_part(k)).collect();
        let keys: Vec<u64> = (0..6)
            .filter(|p| *p == 0 || used.contains(p))
            .map(|p| vfx::part_key(key, p as i32))
            .collect();
        let (table, entries) = build_table(&mut t, &keys, &mut rng);
        t.global(vfx::bridge::TABLE.sym, table);
        let mut rd0 = 0;
        for (k, g) in &entries {
            let rd = t.alloc(&rng.bytes(0x100));
            t.w8(rd + 0xc0, kind);
            let nsegs = rng.below(4);
            t.w32(rd + 0x90, if rng.chance(15) { u32::MAX } else { nsegs });
            let segs = t.alloc(&rng.bytes(0x6c * 4));
            t.w32(rd + 0x94, segs);
            t.w32(*g + 0x24, rd);
            if *k == key {
                rd0 = rd;
            }
        }
        // kind 0 keeps the old mask and names no other part
        let count: u16 = if kind == 0 {
            rng.below(2) as u16
        } else {
            rng.below(7) as u16
        };
        t.write(rd0 + 0xc4, &count.to_le_bytes());
        // the shape: a segment table, the chosen segment, its primitives
        let shape = t.zeros(0x400);
        let which = rng.below(3);
        let seg_off = 0x80 + 0x40 * which;
        t.write(shape + 0x10 + which * 8, &(seg_off as i16).to_le_bytes());
        if which != 0 {
            t.write(shape + 0x10, &0x40i16.to_le_bytes());
        }
        let first = shape + if which == 0 { seg_off } else { 0x40 };
        let tex = [0x3960, 0x3400, 0x39a0, 0x3800, 0x1111][rng.below(5) as usize];
        t.w32(first + 0x10, tex | (rng.next() & 0xc000));
        let seg = shape + seg_off;
        t.w8(seg, if rng.chance(2) { 8 } else { 0 });
        t.write(seg + 2, &n.to_le_bytes());
        t.w32(seg + 4, 0x200 - seg_off);
        for (i, &k) in prims.iter().enumerate() {
            t.w8(shape + 0x204 + 8 * i as u32, k);
        }
        let part = t.zeros(4);
        t.w32(part, shape);
        // the object
        let obj = t.alloc(&rng.bytes(0x40));
        let p10 = t.alloc(&rng.bytes(0x40));
        let depth = [100.0f32, 299.0, 300.0, 301.0, -5.0][rng.below(5) as usize];
        t.write(p10 + 0x38, &depth.to_le_bytes());
        t.w32(obj + 0x10, p10);
        let p1c = if rng.chance(2) {
            0
        } else {
            t.alloc(&rng.bytes(0x40))
        };
        t.w32(obj + 0x1c, p1c);
        // the globals
        let magic = [
            0x12e, 0x274, 0x99, 0x9a, 0x259, 0x98, 0x215, 0x123, 0x154, 7,
        ][rng.below(10) as usize];
        t.global(rdb::MAGIC_ID.sym, magic);
        let scene = build_scene(&mut t, &mut rng);
        if rng.chance(2) {
            t.w32(scene + 0xfaf4, 0x76f);
        }
        t.global(rdb::YIQ_MODE.sym, rng.below(7));
        for g in [rdb::FILL_MATRIX, rdb::FILL_MATRIX_2, rdb::FILL_MATRIX_3] {
            let v = rng.bytes(0x40);
            let at = t.rebase(g.sym);
            t.write(at, &v);
        }
        t.global(vfx::bridge::SKIP_FRAME.sym, u32::from(rng.chance(30)));
        let fill = t.alloc(&[u8::from(!rng.chance(3)), 0, 0, 0]);
        t.global(managers::bridge::DYN_GEO_FILL_BUFFER.sym, fill);
        let req = t.alloc(&rng.bytes(0x200));
        t.stub_sym(rdb::FILL_REQUEST, Conv::Cdecl, 1, req);
        t.stub_sym(
            rdb::BUFFER_FOR_SEGMENT,
            Conv::Cdecl,
            4,
            u32::from(!rng.chance(20)),
        );
        let (lo, hi) = split(key);
        let segment = which | rng.next() & 0xffff_0000;
        let what = format!("round {round} kind {kind} count {count} n {n} magic {magic:#x}");
        t.check(
            &what,
            UPDATE_RENDER_DATA_SHP,
            Conv::Cdecl,
            &[lo, hi, part, obj, segment],
            |g| Out::U32(rdb::update_shp(g, lo, hi, part, obj, segment)),
        );
    }
}

/// A dynamic-geometry manager as `g_DynGeoFillBuffer`'s: a set of four
/// buffers, the current one `room` bytes from full. Returns the manager.
fn build_dyn_geo(t: &mut Twins, rng: &mut Rng, room: u32) -> u32 {
    let mut table = Vec::new();
    for _ in 0..4 {
        let size = 0x400 + rng.below(0x400);
        let start = t.zeros(size as usize + 0x40);
        let fill = start + size.saturating_sub(room) - rng.below(16);
        let b = t.zeros(0x10);
        t.w32(b, size);
        t.w32(b + 4, start);
        t.w32(b + 8, fill);
        table.extend(b.to_le_bytes());
    }
    let table = t.alloc(&table);
    let set = t.zeros(0x20);
    t.w32(set + 4, rng.below(4));
    t.w32(set + 8, 4);
    t.w32(set + 0x10, table);
    let manager = t.zeros(8);
    t.w32(manager, if rng.chance(15) { 0 } else { set });
    // the CRT's stderr message on a full buffer: recording stubs in the
    // import slots
    let iob = t.stub(None, 0x10b, false, 0, 0, 0x5000);
    let fprintf = t.stub(None, 0xf9, false, 2, 0, 1);
    t.global(managers::bridge::IOB_FUNC_SLOT, iob);
    t.global(managers::bridge::FPRINTF_SLOT, fprintf);
    manager
}

#[test]
fn dyn_geo_buffers_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0xd1_9e0b);
    for round in 0..400 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let room = rng.below(0x300);
        let manager = build_dyn_geo(&mut t, &mut rng, room);
        t.global(managers::bridge::DYN_GEO_FILL_BUFFER.sym, manager);
        let (v, i) = (rng.below(24), rng.below(40));
        let what = format!("round {round} room {room:#x} vertices {v} indices {i}");
        if round % 2 == 0 {
            let size = rng.below(0x300);
            t.check(&what, 0x006d_e940, Conv::Thiscall(manager), &[size], |g| {
                Out::U32(managers::bridge::dyn_geo_get_buffer(g, manager, size))
            });
        } else if t.r32(manager) != 0 {
            let data = t.zeros(0x100);
            let segs = t.alloc(&rng.bytes(0x6c * 8 + 0x100));
            t.w32(data + 0x94, segs);
            let seg = rng.below(6);
            t.check(&what, 0x0063_fdb0, Conv::Cdecl, &[data, seg, v, i], |g| {
                Out::U32(managers::bridge::get_buffer_for_segment(g, data, seg, v, i))
            });
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round2_accessors_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2002_0002);
    for round in 0..300 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 6 {
            0 => {
                let mut s = rng.bytes(0x10000);
                for i in 0..16 {
                    let at = 0xfc8c + 4 * i;
                    s[at..at + 4].copy_from_slice(&rng.float().to_le_bytes());
                }
                s[0xfc88] = u8::from(rng.chance(2));
                let scene = t.alloc(&s);
                let out = t.zeros(0x20);
                let p: [u32; 6] = std::array::from_fn(|i| out + 4 * i as u32);
                t.check(&what, 0x0065_e9e0, Conv::Thiscall(scene), &p, |g| {
                    Out::U32(scene::bridge::yiq_param(g, scene, p))
                });
            }
            1 => {
                let scene = t.zeros(0x10000);
                let n = rng.below(8);
                let list: Vec<u8> = (0..8).flat_map(|_| rng.below(6).to_le_bytes()).collect();
                let list = t.alloc(&list);
                t.w32(scene + 0xfb48, n);
                t.w32(scene + 0xfb50, list);
                let chr = rng.below(6);
                t.check(&what, 0x0066_8640, Conv::Thiscall(scene), &[chr], |g| {
                    Out::U32(scene::bridge::remove_chr_from_pipeline(g, scene, chr))
                });
            }
            2 => {
                let inst = if rng.chance(8) {
                    0
                } else {
                    let n = rng.below(4);
                    let mut refs = Vec::new();
                    for _ in 0..n {
                        let segs = rng.below(5);
                        let mut sb = rng.bytes(0x6c * 5);
                        for k in 0..5 {
                            sb[0x6c * k + 0x18..0x6c * k + 0x1c]
                                .copy_from_slice(&rng.below(4).to_le_bytes());
                        }
                        let sp = t.alloc(&sb);
                        let mesh = t.zeros(8);
                        t.w32(mesh, segs | (rng.next() & 0x8000_0000));
                        t.w32(mesh + 4, sp);
                        let r = t.alloc(&mesh.to_le_bytes());
                        refs.extend(r.to_le_bytes());
                    }
                    refs.extend([0; 4]);
                    let refs = t.alloc(&refs);
                    let inst = t.zeros(8);
                    t.w32(inst, n | (rng.next() & 0x8000_0000));
                    t.w32(inst + 4, refs);
                    inst
                };
                let (id, hide) = (rng.below(4), rng.below(3));
                t.check(&what, 0x0063_bdf0, Conv::Cdecl, &[inst, id, hide], |g| {
                    Out::U32(scene::bridge::set_mesh_segment_hide(g, inst, id, hide))
                });
            }
            3 => {
                let menu = if rng.chance(4) {
                    0
                } else {
                    t.alloc(&rng.bytes(0x100))
                };
                if menu != 0 && rng.chance(2) {
                    t.w8(menu + 0x70, 0);
                }
                t.global(scene::bridge::ESC_MENU.sym, menu);
                t.check(&what, 0x0063_da80, Conv::Cdecl, &[], |g| {
                    Out::U32(scene::bridge::is_esc_menu_active(g))
                });
            }
            4 => {
                let mut s = rng.bytes(0x2c00);
                for i in 0..64 {
                    let at = 0xde8 + 4 * i;
                    s[at..at + 4].copy_from_slice(&rng.below(80).to_le_bytes());
                    let c = 0x29e8 + 4 * i;
                    s[c..c + 4].copy_from_slice(&(rng.below(4) as i32 - 1).to_le_bytes());
                }
                let sm = t.alloc(&s);
                let obj = rng.below(80);
                t.check(&what, 0x006c_e9d0, Conv::Thiscall(sm), &[obj], |g| {
                    Out::U8(managers::bridge::remove_from_shadow_map(g, sm, obj))
                });
            }
            _ => {
                // a random search tree under a head
                let head = t.zeros(0x10);
                let keys: Vec<u32> = (0..rng.below(12)).map(|_| rng.below(40)).collect();
                let mut nodes: Vec<(u32, u32)> = Vec::new();
                for &k in &keys {
                    if nodes.iter().any(|&(nk, _)| nk == k) {
                        continue;
                    }
                    let n = t.zeros(0x10);
                    t.w32(n, head);
                    t.w32(n + 4, head);
                    t.w32(n + 0xc, k);
                    // insert under the root
                    let mut at = head + 0xc;
                    loop {
                        let cur = t.r32(at);
                        if cur == head || cur == 0 {
                            t.w32(at, n);
                            break;
                        }
                        let ck = t.r32(cur + 0xc);
                        at = cur + if k < ck { 0 } else { 4 };
                    }
                    nodes.push((k, n));
                }
                if nodes.is_empty() {
                    t.w32(head + 0xc, head);
                }
                let kp = t.alloc(&rng.below(40).to_le_bytes());
                t.check(&what, 0x006d_edd0, Conv::Thiscall(head), &[kp], |g| {
                    Out::U32(managers::bridge::tree_find(g, head, kp))
                });
            }
        }
    }
}

#[test]
fn tex_anim_matches_the_original() {
    use ffx_graphic::texanim::bridge as ta;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x7e7a_0001);
    let word = |rng: &mut Rng| match rng.below(3) {
        0 => rng.below(0x100),
        1 => [0x7000_0000, 0x6fff_ffff, 0x8000_0000, 0xffff_fff1, 0][rng.below(5) as usize],
        _ => rng.next(),
    };
    for round in 0..3000 {
        t.reset();
        let what = format!("round {round}");
        let a: [u32; 8] = std::array::from_fn(|_| word(&mut rng));
        let q = t.zeros(0x10);
        match round % 10 {
            0 => t.check(&what, 0x0077_e730, Conv::Cdecl, &a[..2], |g| {
                Out::U32(ta::load_image(g, a[0], a[1], 0x100))
            }),
            1 => t.check(&what, 0x0077_e7c0, Conv::Cdecl, &a[..2], |g| {
                Out::U32(ta::load_image(g, a[0], a[1], 0x101))
            }),
            2 => t.check(&what, 0x0077_e770, Conv::Cdecl, &a[..3], |g| {
                Out::U32(ta::load_image_regs(g, a[0], a[1], a[2]))
            }),
            3 => t.check(&what, 0x0077_e800, Conv::Cdecl, &a[..1], |g| {
                Out::U32(ta::sync_path(g, a[0]))
            }),
            4 => t.check(&what, 0x0077_eaa0, Conv::Cdecl, &a[..1], |g| {
                Out::U32(ta::sync_path(g, a[0]))
            }),
            5 => t.check(
                &what,
                0x0077_ebc0,
                Conv::Cdecl,
                &[q, a[0], a[1], a[2]],
                |g| Out::U32(ta::qword(g, q, a[0], a[1], a[2])),
            ),
            6 => {
                let p: [u32; 6] = std::array::from_fn(|i| a[i]);
                t.check(
                    &what,
                    0x0077_eb70,
                    Conv::Cdecl,
                    &[q, p[0], p[1], p[2], p[3], p[4], p[5]],
                    |g| Out::U32(ta::dma_tag(g, q, p)),
                );
            }
            7 => {
                let mut args = vec![q];
                args.extend(a);
                t.check(&what, 0x0077_ebe0, Conv::Cdecl, &args, |g| {
                    Out::U32(ta::gif_tag(g, q, a))
                });
            }
            8 => {
                let i = rng.below(64);
                t.check(&what, 0x0077_e9f0, Conv::Cdecl, &[i], |g| {
                    Out::U32(ta::work(g, i))
                });
            }
            _ => {
                // the dispatch, with both sets and the dialogue stubbed
                let i = rng.below(64);
                let at = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i + 0xd;
                t.w8(at, [0u8, 1, 2, 0xff][rng.below(4) as usize]);
                t.stub_sym(ta::NO_SET_NEW, Conv::Cdecl, 6, rng.next());
                t.stub_sym(ta::NO_SET_OLD, Conv::Cdecl, 3, rng.next());
                t.check(
                    &what,
                    0x0077_e940,
                    Conv::Cdecl,
                    &[i, a[0], a[1], a[2], a[3]],
                    |g| Out::U32(ta::no_set(g, i, a[0], a[1], a[2], a[3])),
                );
                t.stub_sym(ta::DLG_CHECK, Conv::Cdecl, 0, rng.below(3));
                t.stub_sym(ta::DLG_RUN, Conv::Cdecl, 1, rng.next());
                t.check(&what, 0x0077_e920, Conv::Cdecl, &[a[4]], |g| {
                    Out::U32(ta::dlg(g, a[4]))
                });
            }
        }
    }
}

#[test]
fn round2_more_matches_the_original() {
    use ffx_graphic::texanim::bridge as ta;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2002_0003);
    for round in 0..600 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 4 {
            0 => {
                // the minimap viewport over a target height
                let scene = t.alloc(&rng.bytes(0x1100));
                t.w8(scene, u8::from(rng.chance(2)));
                t.w32(scene + 0x74, rng.below(6));
                let target = t.alloc(&rng.bytes(0x20));
                let h = [
                    0,
                    1,
                    720,
                    1080,
                    2160,
                    0x7fff_ffff,
                    0x8000_0000,
                    0xffff_ffff,
                    rng.next(),
                ][rng.below(9) as usize];
                t.w32(target + 0x1c, h);
                let cell = t.zeros(8);
                t.w32(cell, target);
                let owner = t.zeros(0x90);
                t.w32(owner + 0x8c, cell);
                t.w32(scene + 0x10cc, owner);
                let fs: [(u32, Vp); 3] = [
                    (0x0065_e320, |g, s| {
                        scene::bridge::minimap_viewport_size(g, s)
                    }),
                    (0x0065_e3f0, |g, s| scene::bridge::minimap_viewport_x(g, s)),
                    (0x0065_e560, |g, s| scene::bridge::minimap_viewport_y(g, s)),
                ];
                for (sym, f) in fs {
                    t.check(
                        &format!("{what} h {h:#x}"),
                        sym,
                        Conv::Thiscall(scene),
                        &[],
                        |g| Out::U32(f(g, scene)),
                    );
                }
            }
            1 => {
                let i = rng.below(64);
                let e = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i;
                let work = t.alloc(&rng.bytes(0xd60));
                for n in 0..64 {
                    t.w8(work + 0xd08 + n, rng.below(3) as u8);
                }
                t.w32(e, u32::from(!rng.chance(5)));
                t.w32(e + 4, work);
                let which = rng.below(4);
                let v = rng.next();
                t.check(&what, 0x0077_fda0, Conv::Cdecl, &[i, which, v], |g| {
                    ta::reset_pos(g, i, which, v);
                    Out::None
                });
            }
            _ => {
                // insert into a tree built by the original's own inserts
                let head = t.zeros(0x10);
                t.w32(head + 0xc, head);
                for k in 0..rng.below(10) {
                    let n = t.zeros(0x10);
                    t.w32(n + 0xc, rng.below(30));
                    t.w32(n + 8, rng.below(2));
                    t.check(
                        &format!("{what} insert {k}"),
                        0x006d_ee50,
                        Conv::Thiscall(head),
                        &[n],
                        |g| Out::U32(managers::bridge::tree_insert(g, head, n)),
                    );
                }
            }
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round3_matches_the_original() {
    use ffx_graphic::texanim::bridge as ta;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x3003_0003);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 4 {
            0 => {
                // a UI element: corner points, the 500 tag
                let ui_obj = t.alloc(&rng.bytes(0x5c00));
                t.global(ui::bridge::UI.sym, ui_obj);
                t.w32(ui_obj + 0x5ba0, u32::from(!rng.chance(8)));
                t.w32(ui_obj + 0x5ba4, u32::from(rng.chance(2)));
                let f = |rng: &mut Rng| -> f32 {
                    if rng.chance(20) {
                        rng.float()
                    } else {
                        rng.below(1000) as f32 - 200.0
                    }
                };
                for off in [0x5ba8, 0x5bac] {
                    let s = if rng.chance(10) {
                        rng.float()
                    } else {
                        (rng.below(400) as f32 - 100.0) / 97.0
                    };
                    t.write(ui_obj + off, &s.to_le_bytes());
                }
                for off in [0x5bb0, 0x5bb4] {
                    let v = f(&mut rng);
                    t.write(ui_obj + off, &v.to_le_bytes());
                }
                let lo = rng.below(400) as f32;
                let hi = lo + rng.below(500) as f32;
                for (off, v) in [(0x5bb8, lo), (0x5bbc, hi), (0x5bc0, lo), (0x5bc4, hi)] {
                    t.write(ui_obj + off, &v.to_le_bytes());
                }
                let mut e = rng.bytes(0x90);
                for i in 0..8 {
                    let at = 0x20 * (i / 2) + 4 * (i % 2);
                    e[at..at + 4].copy_from_slice(&f(&mut rng).to_le_bytes());
                }
                if rng.chance(2) {
                    e[0x80..0x84].copy_from_slice(&500u32.to_le_bytes());
                }
                let el = t.alloc(&e);
                let kind = rng.below(6);
                t.check(&what, 0x0064_4320, Conv::Cdecl, &[el, kind], |g| {
                    ui::bridge::element_remap(g, el, kind);
                    Out::None
                });
            }
            1 => {
                let i = rng.below(64);
                let e = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i;
                let work = t.alloc(&rng.bytes(0x800));
                t.w8(work + 0x5a9, rng.below(3) as u8);
                t.w32(e, u32::from(!rng.chance(5)));
                t.w32(e + 4, work);
                let (which, v) = (rng.below(4), rng.next());
                t.check(&what, 0x0078_08f0, Conv::Cdecl, &[i, which, v], |g| {
                    ta::pos_set_old(g, i, which, v);
                    Out::None
                });
            }
            _ => {
                // texture sets with names, some shared, some equal text
                let names: Vec<u32> = ["tex_a", "tex_b", "tex_a", "c"]
                    .iter()
                    .map(|s| t.alloc(&[s.as_bytes(), &[0]].concat()))
                    .collect();
                let n = rng.below(5);
                let mut entries = Vec::new();
                for _ in 0..n {
                    let mut ent = rng.bytes(0x20);
                    let name = if rng.chance(5) {
                        0
                    } else {
                        names[rng.below(4) as usize] | rng.below(2)
                    };
                    let count = rng.below(4);
                    let mats: Vec<u8> = (0..4).flat_map(|_| rng.next().to_le_bytes()).collect();
                    let mats = t.alloc(&mats);
                    ent[0..4].copy_from_slice(&(count | (rng.next() & 0x8000_0000)).to_le_bytes());
                    ent[4..8].copy_from_slice(&mats.to_le_bytes());
                    if round % 4 == 2 {
                        ent[4..8].copy_from_slice(&name.to_le_bytes());
                    }
                    ent[0x18..0x1c].copy_from_slice(&name.to_le_bytes());
                    entries.extend(ent);
                }
                entries.extend([0; 0x20]);
                let base = t.alloc(&entries);
                let this = t.zeros(0x90);
                let probe = if rng.chance(6) {
                    0
                } else {
                    names[rng.below(4) as usize]
                };
                if round % 4 == 2 {
                    t.w32(this + 0x7c, n | (rng.next() & 0x8000_0000));
                    t.w32(this + 0x80, base);
                    t.check(&what, 0x006b_1d10, Conv::Thiscall(this), &[probe], |g| {
                        Out::U32(managers::bridge::find_by_name(g, this, probe))
                    });
                } else {
                    t.w32(this + 0x64, n);
                    t.w32(this + 0x68, base);
                    let slots: Vec<u8> = (0..4)
                        .flat_map(|_| {
                            (if rng.chance(5) {
                                u32::MAX
                            } else {
                                rng.below(5)
                            })
                            .to_le_bytes()
                        })
                        .collect();
                    let slots = t.alloc(&slots);
                    let list = if rng.chance(8) { 0 } else { t.zeros(0x10) };
                    if list != 0 {
                        t.w32(list + 8, rng.below(5));
                        t.w32(list + 0xc, slots);
                    }
                    let index = rng.below(5);
                    let probe = names[rng.below(4) as usize];
                    t.check(
                        &what,
                        0x006b_1e40,
                        Conv::Thiscall(this),
                        &[list, index, probe],
                        |g| Out::U32(managers::bridge::find_material(g, this, list, index, probe)),
                    );
                }
            }
        }
    }
}

#[test]
fn snapshot_ui_remap_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x3003_0004);
    if t.r32(t.rebase(ui::bridge::UI.sym)) == 0 {
        return;
    }
    for round in 0..500 {
        // elements across the PS2 screen, on the game's own remap
        let mut e = vec![0u8; 0x90];
        for i in 0..8 {
            let at = 0x20 * (i / 2) + 4 * (i % 2);
            let v = rng.below(640) as f32 - 64.0;
            e[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        if rng.chance(2) {
            e[0x80..0x84].copy_from_slice(&500u32.to_le_bytes());
        }
        let el = t.alloc(&e);
        let kind = rng.below(5);
        t.check(
            &format!("snapshot round {round}"),
            0x0064_4320,
            Conv::Cdecl,
            &[el, kind],
            |g| {
                ui::bridge::element_remap(g, el, kind);
                Out::None
            },
        );
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function
fn round4_matches_the_original() {
    use managers::bridge as mg;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x4004_0004);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 10 {
            0 => {
                let keys = build_cache(&mut t, &mut rng);
                let (kind, id) = keys[rng.below(100) as usize];
                let k = kind as u32;
                match rng.below(5) {
                    0 => t.check(&what, 0x0084_0910, Conv::Cdecl, &[], |g| {
                        cache::bridge::init(g);
                        Out::None
                    }),
                    1 => t.check(&what, 0x0084_0120, Conv::Cdecl, &[], |g| {
                        cache::bridge::clear_check(g);
                        Out::None
                    }),
                    2 => t.check(&what, 0x0084_0960, Conv::Cdecl, &[k, id], |g| {
                        cache::bridge::lock(g, kind, id);
                        Out::None
                    }),
                    3 => t.check(&what, 0x0084_0890, Conv::Cdecl, &[k, id], |g| {
                        Out::U32(cache::bridge::exist_field(g, kind, id, 0x10))
                    }),
                    _ => t.check(&what, 0x0084_08d0, Conv::Cdecl, &[k, id], |g| {
                        Out::U32(cache::bridge::exist_field(g, kind, id, 0x14))
                    }),
                }
            }
            1 => {
                let scene = t.zeros(0x10000);
                let (x, y) = (rng.float(), rng.float());
                t.write(scene + 0xfa34, &x.to_le_bytes());
                t.write(scene + 0xfa38, &y.to_le_bytes());
                t.global(scene::bridge::SCENE.sym, scene);
                let out = t.zeros(8);
                t.check(&what, 0x0064_2d10, Conv::Cdecl, &[out, out + 4], |g| {
                    Out::U32(scene::bridge::screen_get_offset(g, out, out + 4))
                });
            }
            2 => {
                let scene = t.zeros(0x1100);
                let info = if rng.chance(3) { 0x6b } else { rng.next() };
                let p = rng.below(3);
                t.check(&what, 0x0066_e800, Conv::Thiscall(scene), &[info, p], |g| {
                    Out::U32(scene::bridge::set_texture_video_info(g, scene, info, p))
                });
                let v = rng.next();
                let (sym, off) = if rng.chance(2) {
                    (0x0066_cfe0, 1)
                } else {
                    (0x0066_d1b0, 3)
                };
                t.check(&what, sym, Conv::Thiscall(scene), &[v], |g| {
                    Out::U8(scene::bridge::set_byte_field(g, scene, off, v))
                });
            }
            3 => {
                let pp = t.alloc(&rng.bytes(0x12400));
                t.global(mg::POST_PROCESS.sym, pp);
                let which = rng.below(4);
                let d = rng.float();
                t.check(
                    &what,
                    0x0069_8550,
                    Conv::Thiscall(pp),
                    &[which, d.to_bits()],
                    |g| Out::U32(mg::set_focus_depth(g, pp, which, d)),
                );
                let v = rng.next();
                t.check(&what, 0x0069_89c0, Conv::Thiscall(pp), &[v], |g| {
                    Out::U8(mg::set_all_switches(g, pp, v))
                });
                t.w8(pp + 0x123a0, u8::from(rng.chance(2)));
                t.w8(pp + 0x123a8, rng.below(3) as u8);
                t.check(&what, 0x0069_8f70, Conv::Thiscall(pp), &[], |g| {
                    mg::reset_cross_fade(g, pp);
                    Out::None
                });
            }
            4 => {
                let mut s = rng.bytes(0x5c0);
                for i in 0..4 {
                    if rng.chance(2) {
                        s[4 * i..4 * i + 4].fill(0);
                    }
                    s[0x5b0 + i] = u8::from(rng.chance(2));
                }
                let sp = t.alloc(&s);
                t.check(&what, 0x006c_59a0, Conv::Thiscall(sp), &[], |g| {
                    Out::U32(mg::need_draw_specular_pass(g, sp))
                });
            }
            5 => {
                let mut s = rng.bytes(0xf00);
                for i in 0..64 {
                    s[0xde8 + 4 * i..0xdec + 4 * i].copy_from_slice(&rng.below(100).to_le_bytes());
                }
                let sm = t.alloc(&s);
                let out = t.zeros(8);
                let obj = rng.below(100);
                t.check(
                    &what,
                    0x006c_8040,
                    Conv::Thiscall(sm),
                    &[obj, out, out + 4],
                    |g| Out::U8(mg::already_in_shadow_map(g, sm, obj, out, out + 4)),
                );
            }
            6 => {
                // a list of nodes and a cursor
                let list = t.zeros(0x50);
                let nodes: Vec<u32> = (0..rng.below(5)).map(|_| t.zeros(0x50)).collect();
                for w in nodes.windows(2) {
                    t.w32(w[0] + 0x40, w[1]);
                }
                t.w32(list + 0x2c, nodes.first().copied().unwrap_or(0));
                t.w32(list + 0x40, rng.below(2));
                let cur = t.zeros(0x20);
                t.w32(cur + 0x18, if rng.chance(6) { 0 } else { list });
                let at = if !nodes.is_empty() && rng.chance(2) {
                    nodes[rng.below(nodes.len() as u32) as usize]
                } else {
                    rng.next()
                };
                t.w32(cur + 0x1c, at);
                let sym = [
                    0x0066_0240,
                    0x0066_0270,
                    0x0066_02a0,
                    0x0066_02d0,
                    0x0066_0300,
                    0x006b_28f0,
                ][rng.below(6) as usize];
                t.check(&what, sym, Conv::Thiscall(cur), &[], |g| {
                    Out::U8(mg::cursor_valid(g, cur))
                });
            }
            7 => {
                let b = t.alloc(&rng.bytes(0x90));
                let f: [f32; 4] = std::array::from_fn(|_| rng.float());
                let args: Vec<u32> = f.iter().map(|x| x.to_bits()).collect();
                t.check(&what, 0x006c_46e0, Conv::Thiscall(b), &args, |g| {
                    mg::set_floats4(g, b, f);
                    Out::None
                });
            }
            8 => {
                let d = t.zeros(0x80);
                let verts = t.zeros(0x1000);
                let uvs = t.zeros(0x800);
                t.w32(d + 0x6c, verts);
                t.w32(d + 0x70, uvs);
                let (i, j) = (rng.below(8), rng.below(9));
                let f: [f32; 6] = std::array::from_fn(|_| rng.float());
                let mut args = vec![i, j];
                args.extend(f.iter().map(|x| x.to_bits()));
                t.check(&what, 0x006c_3240, Conv::Thiscall(d), &args, |g| {
                    Out::U32(mg::distortion_update_vertex(g, d, i, j, f))
                });
            }
            _ => {
                let bounds = t.alloc(
                    &(0..8)
                        .flat_map(|_| rng.float().to_le_bytes())
                        .collect::<Vec<u8>>(),
                );
                let mesh = t.zeros(0x20);
                t.w32(mesh + 0x1c, bounds - 0x10 + 0x10);
                let meshes = t.alloc(&mesh.to_le_bytes());
                let inst = if rng.chance(6) { 0 } else { t.zeros(8) };
                if inst != 0 {
                    t.w32(inst, rng.below(3) | (rng.next() & 0x8000_0000));
                    t.w32(inst + 4, meshes);
                }
                let out = t.zeros(12);
                t.check(
                    &what,
                    0x0063_b1b0,
                    Conv::Cdecl,
                    &[inst, out, out + 4, out + 8],
                    |g| {
                        scene::bridge::character_max_bounding_box(g, inst, out, out + 4, out + 8);
                        Out::None
                    },
                );
            }
        }
    }
}

// ---- the scene ----

/// A `graphic*`/`PhyreScene` accessor under test: the original and the
/// bridge, with `this` (the scene) and two random arguments.
type Accessor = (&'static str, u32, Kind, fn(&mut G, u32, u32, u32) -> Out);

/// How an accessor is called.
#[derive(Clone, Copy)]
enum Kind {
    /// cdecl, no arguments.
    C0,
    /// cdecl, one argument.
    C1,
    /// cdecl, two arguments.
    C2,
    /// cdecl, two float arguments.
    C2F,
    /// thiscall, no arguments.
    T0,
    /// thiscall, one argument.
    T1,
    /// thiscall, one float argument.
    T1F,
    /// thiscall, two float arguments.
    T2F,
    /// stdcall, one argument.
    S1,
    /// stdcall, two arguments.
    S2,
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

const SCENE_ACCESSORS: &[Accessor] = {
    use scene::bridge as sc;
    &[
        (
            "graphicLevelGetNameHash",
            LEVEL_GET_NAME_HASH,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::level_get_name_hash(g)),
        ),
        (
            "PhyreScene_getCurrentLevelNameHash",
            0x0065_de90,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::current_level_name_hash(g, s)),
        ),
        (
            "PhyreScene_getPreviousLevelNameHash",
            0x0065_e310,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::previous_level_name_hash(g, s)),
        ),
        (
            "PhyreScene_setCurrentLevelNameHash",
            0x0066_cf90,
            Kind::T1,
            |g, s, a, _| Out::U32(sc::set_current_level_name_hash(g, s, a)),
        ),
        (
            "PhyreScene_getCurrentLevelName",
            0x0065_de70,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::current_level_name(g, s)),
        ),
        (
            "PhyreScene_getCurrentMagicPath",
            0x0065_dea0,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::current_magic_path(g, s)),
        ),
        (
            "PhyreScene_getToBeDeleteMagicPath",
            0x0065_e5e0,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::to_be_delete_magic_path(g, s)),
        ),
        (
            "PhyreScene_GetSplitScreenStat",
            0x0065_17d0,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::field(g, s, 0x10d8c)),
        ),
        (
            "PhyreScene_SetSplitScreenStat",
            0x0065_4550,
            Kind::T1,
            |g, s, a, _| Out::U32(sc::set_if_changed(g, s, 0x10d8c, a)),
        ),
        (
            "PhyreScene_SetSplitScreenType",
            0x0065_4570,
            Kind::T1,
            |g, s, a, _| Out::U32(sc::set_if_changed(g, s, 0x10d90, a)),
        ),
        (
            "PhyreScene_getLinearSampler",
            0x0065_e050,
            Kind::T1,
            |_, s, a, _| Out::U32(sc::linear_sampler(s, a)),
        ),
        (
            "PhyreScene_getVideoTexture",
            0x0065_e640,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::field(g, s, 0xde0)),
        ),
        (
            "PhyreScene_GetField0ddc",
            0x0065_e630,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::field(g, s, 0xddc)),
        ),
        (
            "PhyreScene_GetField0dd8",
            0x0065_e650,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::field(g, s, 0xdd8)),
        ),
        (
            "PhyreScene_GetLivePointer00ac",
            0x0065_ded0,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::live_pointer(g, s, 0xac)),
        ),
        (
            "PhyreScene_GetLivePointer1232c",
            0x0065_e2f0,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::live_pointer(g, s, 0x1232c)),
        ),
        (
            "PhyreScene_ForceSetShadowSolution",
            0x0065_13f0,
            Kind::T1,
            |g, s, a, _| Out::U32(sc::set_field(g, s, 0xf9cc, a)),
        ),
        (
            "PhyreScene_textureVideoPlay",
            0x0067_0750,
            Kind::T1,
            |g, s, a, _| Out::U8(sc::set_byte_field(g, s, 0x10c9, a)),
        ),
        (
            "PhyreScene_isAnyBattleMap",
            0x0066_0330,
            Kind::T0,
            |g, s, _, _| Out::U8(sc::scene_is_any_battle_map(g, s)),
        ),
        (
            "PhyreScene_isFading",
            0x0065_23e0,
            Kind::T0,
            |g, s, _, _| Out::U8(sc::scene_is_fading(g, s)),
        ),
        (
            "PhyreScene_isDrawingNonUI2DScene",
            0x0066_0350,
            Kind::T0,
            |g, s, _, _| Out::U32(sc::is_drawing_non_ui_2d_scene(g, s)),
        ),
        (
            "PhyreScene_is3DScene",
            0x0066_01d0,
            Kind::C0,
            |g, _, _, _| Out::U32(u32::from(sc::IS_3D.get(g))),
        ),
        (
            "PhyreScene_isOpeningScreenPlaying",
            0x0066_0400,
            Kind::C0,
            |g, _, _, _| Out::U8(sc::OPENING_PLAYING.get(g)),
        ),
        (
            "graphicGetMiniMapType",
            0x0064_1320,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_u32(g, 0x74)),
        ),
        (
            "graphicSetMiniMapType",
            0x0064_3720,
            Kind::C1,
            |g, _, a, _| Out::U32(sc::set_scene_u32(g, 0x74, a)),
        ),
        (
            "graphicIsInLucaTheater",
            0x0064_1c10,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 0x10d2e)),
        ),
        (
            "graphicIsFieldMapLoading",
            0x0064_1bf0,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 0x1195)),
        ),
        ("graphicIsBattleMap", 0x0064_1c20, Kind::C0, |g, _, _, _| {
            Out::U32(sc::scene_byte(g, 0x1197))
        }),
        (
            "graphicGetIsBattleMap",
            0x0064_1bb0,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 0x1196)),
        ),
        (
            "graphicNeedResetMapParam",
            0x0064_2690,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 0x10d2d)),
        ),
        (
            "graphicDoneResetMapParam",
            0x0063_df60,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::set_scene_byte(g, 0x10d2d, 0)),
        ),
        (
            "graphicSetSceneFlagFb45",
            0x0064_2fa0,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::set_scene_byte(g, 0xfb45, 1)),
        ),
        (
            "graphicIsMiniMapInBlitzBall",
            0x0064_1c70,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 0)),
        ),
        (
            "graphicGetMagic0146Key",
            0x0064_1310,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::scene_byte(g, 3)),
        ),
        (
            "graphicSetIsBattleMap",
            0x0064_3490,
            Kind::C1,
            |g, _, a, _| Out::U8(sc::set_is_battle_map(g, a)),
        ),
        (
            "graphicIsAnyBattleMap",
            0x0064_1bd0,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::is_any_battle_map(g)),
        ),
        (
            "graphicMinimapFogSetEnable",
            0x0064_25d0,
            Kind::C1,
            |g, _, a, _| Out::U32(sc::set_scene_flag(g, 0x79, a)),
        ),
        (
            "graphicMinimapSetEnable",
            0x0064_25f0,
            Kind::C1,
            |g, _, a, _| Out::U32(sc::set_scene_flag(g, 0x78, a)),
        ),
        (
            "graphicSetMapAlphaFast",
            0x0064_3610,
            Kind::C1,
            |g, _, a, _| Out::U32(sc::set_scene_flag(g, 0xd4, a)),
        ),
        (
            "graphicScreenSetOffset",
            0x0064_2d60,
            Kind::C2F,
            |g, _, a, b| Out::U32(sc::screen_set_offset(g, f(a), f(b))),
        ),
        (
            "graphicScreenSetDttOffset",
            0x0064_2d40,
            Kind::C2F,
            |g, _, a, b| {
                sc::screen_set_dtt_offset(g, f(a), f(b));
                Out::None
            },
        ),
        (
            "graphicGetCut2DSceneBias",
            0x0064_1130,
            Kind::C0,
            |g, _, _, _| Out::F32(sc::scene_float(g, 0xf9bc)),
        ),
        (
            "graphicCameraGetFOVy",
            0x0063_9570,
            Kind::C0,
            |g, _, _, _| Out::F32(sc::camera_float(g, 0x154)),
        ),
        ("getNearPlane", 0x0063_8e70, Kind::C0, |g, _, _, _| {
            Out::F32(sc::camera_float(g, 0xb8))
        }),
        (
            "graphicIsMagicFileAsyncLoadAvailable",
            0x0064_1c60,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::MAGIC_ASYNC_LOAD.get(g)),
        ),
        (
            "graphicSetGameOverNotify",
            0x0064_3260,
            Kind::C1,
            |g, _, a, _| {
                sc::GAME_OVER_NOTIFY.set(g, a);
                Out::U32(a)
            },
        ),
        (
            "graphicGetTextureVideoInfo",
            0x0064_1400,
            Kind::C0,
            |g, _, _, _| Out::U16(sc::TEXTURE_VIDEO_INFO.get(g)),
        ),
        (
            "graphicIsCharacterLoadingOrInstanciating",
            0x0064_1bc0,
            Kind::C0,
            |g, _, _, _| Out::U32(sc::CHARACTER_LOADING.get(g) & 0x7fff_ffff),
        ),
        (
            "graphicGetNowUIType",
            0x0064_1350,
            Kind::C0,
            |g, _, _, _| {
                let u = ui::bridge::UI.get(g);
                Out::U32(sc::field(g, u, 0x2be0))
            },
        ),
        (
            "graphicSetNowUIType",
            0x0064_37c0,
            Kind::C1,
            |g, _, a, _| Out::U32(ui::bridge::set_now_ui_type(g, a)),
        ),
        (
            "cgraphicSetNowUIType",
            0x0063_8d90,
            Kind::C1,
            |g, _, a, _| Out::U32(ui::bridge::set_now_ui_type(g, a)),
        ),
        ("graphicGetAbmap", 0x0064_1060, Kind::C0, |g, _, _, _| {
            Out::U32(ui::bridge::get_abmap(g))
        }),
        (
            "graphicFontGetEdgeRatio",
            0x0064_0d70,
            Kind::C0,
            |g, _, _, _| Out::F32(ui::bridge::font_get_edge_ratio(g)),
        ),
        (
            "PostProcessManager_GetInstance",
            0x0069_2be0,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::POST_PROCESS.get(g)),
        ),
        (
            "PostProcessTweakUtil_GetInstance",
            0x0069_2bf0,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::POST_PROCESS_TWEAK.get(g)),
        ),
        (
            "TextureAnimationManager_GetInstance",
            0x006b_02e0,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::TEXTURE_ANIMATION.get(g)),
        ),
        (
            "ShadowManager_getInstance",
            0x006d_2390,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::SHADOW.get(g)),
        ),
        (
            "ShadowManager_GetShadowSize",
            0x006c_cd00,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::SHADOW_SIZE.get(g)),
        ),
        (
            "SpecularMapParamsManager_GetInstance",
            0x006c_57e0,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::SPECULAR_MAP_PARAMS.get(g)),
        ),
        (
            "DebugMenuManager_GetInstance",
            0x006b_49b0,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::DEBUG_MENU.get(g)),
        ),
        (
            "graphicCheckPauseGame",
            0x006b_cc90,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::PAUSE_GAME.get(g)),
        ),
        (
            "graphicDynGeoFillBufferIsValid",
            0x0063_fd70,
            Kind::C0,
            |g, _, _, _| Out::U32(managers::bridge::dyn_geo_fill_buffer_is_valid(g)),
        ),
    ]
};

/// Accessors of a manager object (`this`, random contents).
const OBJECT_ACCESSORS: &[Accessor] = {
    use managers::bridge as mg;
    &[
        (
            "DynGeoMemManager_getCurrentBufferId",
            0x006d_e9f0,
            Kind::T0,
            |g, s, _, _| Out::U32(mg::dyn_geo_current_buffer_id(g, s)),
        ),
        (
            "DynGeoMemManager_clearDynGeoMemory",
            0x006d_e700,
            Kind::T0,
            |g, s, _, _| {
                mg::dyn_geo_clear(g, s);
                Out::None
            },
        ),
        (
            "DynGeoMemManager_selectBuffer",
            0x006d_ec90,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::dyn_geo_select(g, s, a)),
        ),
        (
            "TextureAnimationManager_getMaterialByLayNum",
            0x006b_2070,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::material_by_lay_num(g, s, a)),
        ),
        (
            "TextureAnimation_NextFrame",
            0x006b_22c0,
            Kind::T0,
            |g, s, _, _| {
                let q = mg::step_frame(g, s, 1);
                Out::U64(u64::from(g.read::<u32>(s + 0x18)) << 32 | u64::from(q))
            },
        ),
        (
            "TextureAnimation_PrevFrame",
            0x006b_2380,
            Kind::T0,
            |g, s, _, _| {
                let q = mg::step_frame(g, s, u32::MAX);
                Out::U64(u64::from(g.read::<u32>(s + 0x18)) << 32 | u64::from(q))
            },
        ),
        (
            "TextureAnimationInfoChar_active",
            0x006b_1c30,
            Kind::T0,
            |g, s, _, _| {
                mg::set_inactive(g, s, false);
                Out::None
            },
        ),
        (
            "TextureAnimationInfoChar_deactive",
            0x006b_04a0,
            Kind::T0,
            |g, s, _, _| {
                mg::set_inactive(g, s, true);
                Out::None
            },
        ),
        (
            "TextureAnimation_GetFloatPair24",
            0x006b_1cc0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::get_float_pair(g, s, 0x24, a)),
        ),
        (
            "TextureAnimation_GetFloatPair2c",
            0x006b_1ce0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::get_float_pair(g, s, 0x2c, a)),
        ),
        (
            "TextureAnimation_SetByte1c",
            0x006b_3650,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x1c, a)),
        ),
        (
            "PostProcessManager_SetByte123b0",
            0x0069_83f0,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x123b0, a)),
        ),
        (
            "PostProcessManager_SetByte123b1",
            0x0069_83e0,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x123b1, a)),
        ),
        (
            "PostProcessManager_SetByte123b3",
            0x0069_85b0,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x123b3, a)),
        ),
        (
            "PostProcessManager_setCrossFadeCaptureEnable",
            0x0069_8490,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x123ba, a)),
        ),
        (
            "PostProcessManager_SetByte123bb",
            0x0069_8480,
            Kind::T1,
            |g, s, a, _| Out::U8(mg::set_byte_field(g, s, 0x123bb, a)),
        ),
        (
            "PostProcessManager_setCrossFadeWeight",
            0x0069_84b0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::set_field(g, s, 0x123bc, a)),
        ),
        (
            "PostProcessManager_setCrossFadeTime",
            0x0069_84a0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::set_field(g, s, 0x123c0, a)),
        ),
        (
            "PostProcessManager_isEffectAllowed",
            0x0069_8730,
            Kind::S1,
            |_, _, a, _| Out::U8(mg::effect_allowed(a)),
        ),
        (
            "Distortion_updateDistortionBlur",
            0x006c_3230,
            Kind::T1F,
            |g, s, a, _| Out::U32(mg::distortion_update_blur(g, s, f(a))),
        ),
        (
            "BlurEffect_activate",
            0x006c_35a0,
            Kind::T0,
            |g, s, _, _| {
                mg::blur_activate(g, s);
                Out::None
            },
        ),
        (
            "BlurEffect_setTarget",
            0x006c_4a90,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::set_with_flag(g, s, a)),
        ),
        (
            "BlurEffect_SetField84",
            0x006c_44d0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::set_field(g, s, 0x84, a)),
        ),
        (
            "BlurEffect_SetFloats6c",
            0x006c_4710,
            Kind::T2F,
            |g, s, a, b| {
                mg::set_float(g, s, 0x6c, f(a));
                mg::set_float(g, s, 0x70, f(b));
                Out::None
            },
        ),
        (
            "ShadowManager_SetFloats401c",
            0x006c_ea80,
            Kind::T2F,
            |g, s, a, b| {
                mg::set_float(g, s, 0x401c, f(a));
                mg::set_float(g, s, 0x4018, f(b));
                Out::None
            },
        ),
        (
            "ShadowManager_clearCascades",
            0x006d_1780,
            Kind::T0,
            |g, s, _, _| Out::U32(mg::clear_words(g, s)),
        ),
        (
            "ShadowManager_getCascade",
            0x006d_22e0,
            Kind::T1,
            |g, s, a, _| Out::U32(mg::word_at(g, s, 0x60, a)),
        ),
        (
            "ShadowManager_getTextureSampler",
            0x006d_2520,
            Kind::T0,
            |_, s, _, _| Out::U32(mg::embedded(s, 0x3fc4)),
        ),
        (
            "PhyreScene_setMeshVisible",
            0x0066_d910,
            Kind::S2,
            |g, s, _, b| {
                scene::bridge::set_mesh_visible(g, s, b);
                Out::None
            },
        ),
        (
            "graphicCharacterInstanceSetDrawSituation",
            0x0063_b7d0,
            Kind::C2,
            |g, s, _, b| {
                scene::bridge::character_set_draw_situation(g, s, b);
                Out::None
            },
        ),
        (
            "graphicFontGetScreenWH",
            0x0064_0db0,
            Kind::C2,
            |g, s, _, b| Out::U32(ui::bridge::font_get_screen_wh(g, s, b)),
        ),
        (
            "graphicScreenGetDttOffset",
            0x0064_2cf0,
            Kind::C2,
            |g, s, _, b| Out::U32(scene::bridge::screen_get_dtt_offset(g, s, b)),
        ),
    ]
};

/// Calls one accessor in both oracles.
fn run_accessor(t: &mut Twins, acc: &Accessor, this: u32, a: u32, b: u32, what: &str) {
    let (name, sym, kind, bridge) = *acc;
    let what = format!("{name}: {what}");
    let (conv, args): (Conv, Vec<u32>) = match kind {
        Kind::C0 => (Conv::Cdecl, vec![]),
        Kind::C1 => (Conv::Cdecl, vec![a]),
        Kind::C2 => (Conv::Cdecl, vec![this, b]),
        Kind::C2F => (Conv::Cdecl, vec![a, b]),
        Kind::T0 => (Conv::Thiscall(this), vec![]),
        Kind::T1 | Kind::T1F => (Conv::Thiscall(this), vec![a]),
        Kind::T2F => (Conv::Thiscall(this), vec![a, b]),
        Kind::S1 => (Conv::Stdcall, vec![a]),
        Kind::S2 => (Conv::Stdcall, vec![this, b]),
    };
    t.check(&what, sym, conv, &args, |g| bridge(g, this, a, b));
}

/// A random argument: small values, flags, pointers' worth of bits.
fn arg(rng: &mut Rng, float: bool) -> u32 {
    if float {
        return rng.float().to_bits();
    }
    match rng.below(4) {
        0 => rng.below(4),
        1 => rng.below(0x200),
        2 => [0x17, 0x15c, 2, 0x100, 0xffff_ff00][rng.below(5) as usize],
        _ => rng.next(),
    }
}

#[test]
fn scene_accessors_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5eed_0001);
    for round in 0..40 {
        t.reset();
        let scene = build_scene(&mut t, &mut rng);
        // pointers the getters follow: the camera, the live-pointer cells
        let camera = t.alloc(&rng.bytes(0x200));
        t.w32(scene + 0xdc, camera);
        for off in [0xac, 0x1232c] {
            let cell = t.alloc(&rng.bytes(8));
            t.w32(scene + off, cell);
        }
        // the globals they read
        let ui_obj = t.alloc(&rng.bytes(0x3000));
        t.global(ui::bridge::UI.sym, ui_obj);
        let abmap = if rng.chance(4) {
            0
        } else {
            t.alloc(&rng.bytes(8))
        };
        t.global(ui::bridge::ABMAP.sym, abmap);
        let fill = t.alloc(&[u8::from(rng.chance(2)), 0, 0, 0]);
        t.global(managers::bridge::DYN_GEO_FILL_BUFFER.sym, fill);
        for (sym, len) in [
            (scene::bridge::IS_3D.sym, 1),
            (scene::bridge::OPENING_PLAYING.sym, 1),
            (scene::bridge::MAGIC_ASYNC_LOAD.sym, 4),
            (scene::bridge::TEXTURE_VIDEO_INFO.sym, 2),
            (scene::bridge::CHARACTER_LOADING.sym, 4),
            (scene::bridge::DTT_OFFSET.sym, 8),
            (scene::bridge::MOVIE_FADING.sym, 4),
            (managers::bridge::POST_PROCESS.sym, 4),
            (managers::bridge::POST_PROCESS_TWEAK.sym, 4),
            (managers::bridge::TEXTURE_ANIMATION.sym, 4),
            (managers::bridge::SHADOW.sym, 4),
            (managers::bridge::SHADOW_SIZE.sym, 4),
            (managers::bridge::SPECULAR_MAP_PARAMS.sym, 4),
            (managers::bridge::DEBUG_MENU.sym, 4),
            (managers::bridge::PAUSE_GAME.sym, 4),
        ] {
            let at = t.rebase(sym);
            let mut v = rng.bytes(len);
            if rng.chance(3) {
                v.fill(0);
            }
            // keep the screen offsets clear of signalling NaNs
            if sym == scene::bridge::DTT_OFFSET.sym {
                v = [rng.float().to_le_bytes(), rng.float().to_le_bytes()].concat();
            }
            t.write(at, &v);
        }
        // the camera and bias floats: no signalling NaNs either
        for at in [scene + 0xf9bc, camera + 0x154, camera + 0xb8] {
            let v = rng.float();
            t.write(at, &v.to_le_bytes());
        }
        for acc in SCENE_ACCESSORS {
            for _ in 0..3 {
                let float = matches!(acc.2, Kind::C2F);
                let (a, b) = (arg(&mut rng, float), arg(&mut rng, float));
                run_accessor(&mut t, acc, scene, a, b, &format!("round {round}"));
            }
        }
    }
}

#[test]
fn object_accessors_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5eed_0002);
    for round in 0..60 {
        t.reset();
        for acc in OBJECT_ACCESSORS {
            for _ in 0..4 {
                let mut bytes = rng.bytes(0x12400);
                // floats the getters copy: no signalling NaNs
                for off in [0x24, 0x28, 0x2c, 0x30] {
                    bytes[off..off + 4].copy_from_slice(&rng.float().to_le_bytes());
                }
                let obj = t.alloc(&bytes);
                // the pointers the managers follow
                let inner = t.alloc(&rng.bytes(0x40));
                let table = t.alloc(&rng.bytes(0x40));
                let buffers: Vec<u8> = (0..4).flat_map(|_| inner.to_le_bytes()).collect();
                let bufs = t.alloc(&buffers);
                match acc.0 {
                    "DynGeoMemManager_getCurrentBufferId"
                    | "DynGeoMemManager_clearDynGeoMemory" => {
                        let set = if rng.chance(4) { 0 } else { inner };
                        t.w32(obj, set);
                        t.w32(inner + 4, rng.below(4));
                        t.w32(inner + 0x10, bufs);
                    }
                    "TextureAnimationManager_getMaterialByLayNum" => {
                        t.w32(obj + 8, rng.below(8));
                        t.w32(obj + 0xc, rng.below(8));
                        t.w32(obj + 0x74, rng.below(24) | (rng.next() & 0x8000_0000));
                        t.w32(obj + 0x78, table);
                    }
                    "TextureAnimation_NextFrame" | "TextureAnimation_PrevFrame" => {
                        t.w32(obj + 8, (1 + rng.below(30)) | (rng.next() & 0x8000_0000));
                        t.w32(obj + 0x18, rng.below(40));
                    }
                    "PhyreScene_setMeshVisible" => {
                        let node = if rng.chance(4) { 0 } else { inner };
                        t.w32(obj + 0x1c, node);
                    }
                    _ => {}
                }
                let float = matches!(acc.2, Kind::T1F | Kind::T2F);
                let (mut a, mut b) = (arg(&mut rng, float), arg(&mut rng, float));
                if matches!(
                    acc.0,
                    "graphicFontGetScreenWH" | "graphicScreenGetDttOffset"
                ) {
                    b = obj + 8;
                }
                match acc.0 {
                    "DynGeoMemManager_selectBuffer" | "ShadowManager_getCascade" => {
                        a = rng.below(16);
                    }
                    "TextureAnimationManager_getMaterialByLayNum" => a = rng.below(16),
                    "TextureAnimation_GetFloatPair24" | "TextureAnimation_GetFloatPair2c" => {
                        a = t.zeros(8);
                    }
                    _ => {}
                }
                let this = match acc.0 {
                    "PhyreScene_setMeshVisible" | "graphicCharacterInstanceSetDrawSituation"
                        if rng.chance(5) =>
                    {
                        0
                    }
                    _ => obj,
                };
                run_accessor(&mut t, acc, this, a, b, &format!("round {round}"));
            }
            t.a.reset_heap();
            t.b.reset_heap();
            t.log = t.zeros(0x8000);
        }
    }
}

#[test]
fn singleton_deleters_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5eed_0003);
    for (sym, g) in [
        (0x0069_2bc0, managers::bridge::POST_PROCESS_TWEAK),
        (0x006b_4990, managers::bridge::DEBUG_MENU),
        (0x006b_cc70, managers::bridge::PAUSE_GAME),
    ] {
        for round in 0..6 {
            t.reset();
            let obj = if round % 3 == 0 {
                0
            } else {
                let dtor = t.stub(None, 0xd7, true, 1, 1, rng.next());
                let vtable = t.alloc(&dtor.to_le_bytes());
                t.alloc(&vtable.to_le_bytes())
            };
            t.global(g.sym, obj);
            t.check(
                &format!("{sym:#x} round {round}"),
                sym,
                Conv::Cdecl,
                &[],
                |h| {
                    managers::bridge::destroy_singleton(h, g);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn ui_remaps_match_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5eed_0004);
    let remaps: [Remap; 4] = [
        (0x0064_47e0, ui::remap_x2),
        (0x0064_4820, ui::remap_y2),
        (0x0064_47c0, ui::remap_x),
        (0x0064_4800, ui::remap_y),
    ];
    for i in 0..20_000 {
        let x = rng.float();
        let (sym, remap) = remaps[i % 4];
        t.check(
            &format!("{sym:#x}({x})"),
            sym,
            Conv::Cdecl,
            &[x.to_bits()],
            |_| Out::F32(remap(x)),
        );
    }
    t.check(
        "graphicGetScreenRatio",
        0x0064_13f0,
        Conv::Cdecl,
        &[],
        |_| Out::F32(ui::SCREEN_RATIO),
    );
    // the edge ratio over every interesting width
    for w in [
        0,
        1,
        1919,
        1920,
        1921,
        3840,
        -1,
        i32::MIN,
        i32::MAX,
        1366,
        2560,
    ] {
        let scene = t.zeros(0x10e00);
        t.global(scene::bridge::SCENE.sym, scene);
        t.w32(scene + 0x10dd0, w as u32);
        t.check(
            &format!("graphicFontGetEdgeRatio({w})"),
            0x0064_0d70,
            Conv::Cdecl,
            &[],
            |g| Out::F32(ui::bridge::font_get_edge_ratio(g)),
        );
    }
}

// ---- the model cache ----

const CACHE_CHECK_YET: u32 = 0x0083_ffe0;
const CACHE_EXIST: u32 = 0x0084_06f0;
const CACHE_EXIST_FIELD: [(u32, u32); 4] = [
    (0x0084_0790, 0xc),
    (0x0084_07d0, 0x18),
    (0x0084_0810, 0x1c),
    (0x0084_0850, 0x20),
];

/// Fills `g_CacheSlots` with 100 random entries over a few ids and kinds.
fn build_cache(t: &mut Twins, rng: &mut Rng) -> Vec<(i32, u32)> {
    let table = t.rebase(cache::bridge::TABLE.sym);
    let mut keys = Vec::new();
    for i in 0..cache::SLOTS {
        let mut e = rng.bytes(0x28);
        let id = rng.below(12) | if rng.chance(4) { 0x0001_0000 } else { 0 };
        e[..4].copy_from_slice(&id.to_le_bytes());
        let kind = [1u8, 2, 3, 4, 0xff][rng.below(5) as usize];
        e[4] = kind;
        for b in [5, 6, 7] {
            e[b] = u8::from(rng.chance(3));
        }
        for off in [0xc, 0x10, 0x14, 0x18, 0x1c, 0x20] {
            if rng.chance(3) {
                e[off..off + 4].fill(0);
            }
        }
        let at = t.alloc(&e);
        t.w32(table + 4 * i, at);
        keys.push((i32::from(kind as i8), id));
    }
    keys
}

/// Stubs the cache's callees in both oracles.
fn stub_cache_callees(t: &mut Twins, rng: &mut Rng, overflow: u32) {
    t.stub_sym(cache::bridge::SG_MEM_SET_TYPE, Conv::Cdecl, 2, rng.next());
    t.stub_sym(cache::bridge::SG_MEM_DUMP, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cache::bridge::CH_CREATE_NAME, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cache::bridge::YI_ASSERT, Conv::Cdecl, 6, rng.next());
    t.stub_sym(cache::bridge::OVERFLOW, Conv::Cdecl, 0, overflow);
}

#[test]
fn cache_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5eed_0005);
    for round in 0..80 {
        t.reset();
        let keys = build_cache(&mut t, &mut rng);
        // with no free entry, Cache_Entry calls Cache_OverFlow until it
        // fails (the stub returns 1)
        stub_cache_callees(&mut t, &mut rng, 1);
        for i in 0..12 {
            let (kind, id) = if rng.chance(4) {
                (rng.below(6) as i32 - 1, rng.below(14))
            } else {
                keys[rng.below(keys.len() as u32) as usize]
            };
            let what = format!("round {round} op {i} kind {kind} id {id:#x}");
            let k = kind as u32;
            let x = rng.next();
            match rng.below(13) {
                0 => t.check(&what, CACHE_CHECK_YET, Conv::Cdecl, &[k, id], |g| {
                    Out::U32(cache::bridge::check_yet(g, kind, id))
                }),
                1 => t.check(&what, CACHE_EXIST, Conv::Cdecl, &[k, id], |g| {
                    Out::U32(cache::bridge::exist(g, kind, id))
                }),
                2 => {
                    let (sym, off) = CACHE_EXIST_FIELD[rng.below(4) as usize];
                    t.check(&what, sym, Conv::Cdecl, &[k, id], |g| {
                        Out::U32(cache::bridge::exist_field(g, kind, id, off))
                    });
                }
                3 => t.check(&what, 0x0084_05b0, Conv::Cdecl, &[k, id, x], |g| {
                    cache::bridge::entry_cloth_vertex_buff(g, kind, id, x);
                    Out::None
                }),
                4 => {
                    let out = t.zeros(16);
                    let p = [out, out + 4, out + 8, out + 12];
                    t.check(
                        &what,
                        0x0084_0680,
                        Conv::Cdecl,
                        &[k, id, p[0], p[1], p[2], p[3]],
                        |g| {
                            cache::bridge::entry_render_data(g, kind, id, p);
                            Out::None
                        },
                    );
                }
                5 => t.check(&what, 0x0084_0ac0, Conv::Cdecl, &[k, id, x], |g| {
                    cache::bridge::set_lock(g, kind, id, x);
                    Out::None
                }),
                6 => t.check(&what, 0x0084_0cf0, Conv::Cdecl, &[k, id], |g| {
                    Out::U32(cache::bridge::touch(g, kind, id))
                }),
                7 => t.check(&what, 0x0084_0dd0, Conv::Cdecl, &[k, id], |g| {
                    cache::bridge::use_entry(g, kind, id);
                    Out::None
                }),
                8 => t.check(&what, 0x0084_0d70, Conv::Cdecl, &[k, id], |g| {
                    cache::bridge::unuse(g, kind, id);
                    Out::None
                }),
                9 | 10 => t.check(&what, 0x0084_05f0, Conv::Cdecl, &[k, id, x], |g| {
                    cache::bridge::entry_not_yet(g, kind, id, x);
                    Out::None
                }),
                _ => t.check(&what, 0x0084_0560, Conv::Cdecl, &[k, id, x], |g| {
                    cache::bridge::entry_add(g, k, id, x);
                    Out::None
                }),
            }
        }
    }
}

// ---- on a snapshot ----

/// Every key of the real effect table (an in-order walk of its tree).
fn table_keys(m: &impl Mem, table: u32) -> Vec<u64> {
    fn walk(m: &impl Mem, n: u32, out: &mut Vec<u64>) {
        if m.read::<u8>(n + 0xd) != 0 || out.len() > 10_000 {
            return;
        }
        walk(m, m.read(n), out);
        out.push(m.read(n + 0x10));
        walk(m, m.read(n + 8), out);
    }
    let head: u32 = m.read(table);
    let mut out = Vec::new();
    walk(m, m.read(head + 4), &mut out);
    out
}

#[test]
fn snapshot_render_data_and_scene_match_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5eed_0006);
    let scene = t.r32(t.rebase(scene::bridge::SCENE.sym));
    assert_ne!(scene, 0, "the snapshot has a scene");
    // the read-only scene getters on the real scene
    for acc in SCENE_ACCESSORS {
        let (name, _, kind, _) = *acc;
        let reads = matches!(kind, Kind::C0 | Kind::T0)
            && !matches!(
                name,
                "graphicDoneResetMapParam" | "graphicSetSceneFlagFb45" | "graphicGetAbmap"
            );
        // the live-pointer getters fault on an empty cell, as the original
        let empty = match name {
            "PhyreScene_GetLivePointer00ac" => t.r32(scene + 0xac) == 0,
            "PhyreScene_GetLivePointer1232c" => t.r32(scene + 0x1232c) == 0,
            "graphicCameraGetFOVy" | "getNearPlane" => t.r32(scene + 0xdc) == 0,
            _ => false,
        };
        if reads && !empty {
            run_accessor(&mut t, acc, scene, 0, 0, "snapshot");
        }
    }
    // the minimap viewport on the real render target
    let owner = t.r32(scene + 0x10cc);
    if owner != 0 && t.r32(owner + 0x8c) != 0 {
        let fs: [(u32, Vp); 3] = [
            (0x0065_e320, |g, s| {
                scene::bridge::minimap_viewport_size(g, s)
            }),
            (0x0065_e3f0, |g, s| scene::bridge::minimap_viewport_x(g, s)),
            (0x0065_e560, |g, s| scene::bridge::minimap_viewport_y(g, s)),
        ];
        for (sym, f) in fs {
            t.check("snapshot minimap", sym, Conv::Thiscall(scene), &[], |g| {
                Out::U32(f(g, scene))
            });
        }
    }
    let yiq = t.zeros(0x20);
    let p: [u32; 6] = std::array::from_fn(|i| yiq + 4 * i as u32);
    t.check(
        "snapshot yiq",
        0x0065_e9e0,
        Conv::Thiscall(scene),
        &p,
        |g| Out::U32(scene::bridge::yiq_param(g, scene, p)),
    );
    let table = t.r32(t.rebase(vfx::bridge::TABLE.sym));
    if table == 0 {
        eprintln!("no effect table in this snapshot");
        return;
    }
    let keys = table_keys(&t.b, table);
    eprintln!("{} effect instances", keys.len());
    for (i, &k) in keys.iter().enumerate() {
        let (lo, hi) = split(k);
        let what = format!("instance {i} key {k:#x}");
        t.check(
            &what,
            GET_DYNAMIC_GEOMETRY,
            Conv::Thiscall(table),
            &[lo, hi],
            |g| Out::U32(vfx::bridge::get_dynamic_geometry(g, table, lo, hi)),
        );
        t.check(
            &what,
            GET_RENDER_DATA_BY_INSTANCE,
            Conv::Cdecl,
            &[lo, hi],
            |g| Out::U32(vfx::bridge::get_render_data_by_instance(g, lo, hi)),
        );
        let miss = k ^ u64::from(1 + rng.below(9999));
        let (mlo, mhi) = split(miss);
        t.check(
            &what,
            GET_VFX_DATA,
            Conv::Thiscall(table),
            &[mlo, mhi],
            |g| Out::U32(vfx::bridge::get_vfx_data(g, table, mlo, mhi)),
        );
        // the update hands real geometry to its (original) virtual method
        t.check(&what, VFX_UPDATE, Conv::Cdecl, &[lo, hi], |g| {
            vfx::bridge::update(g, lo, hi);
            Out::None
        });
    }
}

#[test]
fn snapshot_cache_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let table = t.rebase(cache::bridge::TABLE.sym);
    let mut keys = Vec::new();
    for i in 0..cache::SLOTS {
        let e = t.r32(table + 4 * i);
        keys.push((i32::from(t.r8(e + 4) as i8), t.r32(e)));
    }
    keys.push((1, 0x7fff));
    for (i, &(kind, id)) in keys.iter().enumerate() {
        let k = kind as u32;
        let what = format!("entry {i} kind {kind} id {id:#x}");
        t.check(&what, CACHE_CHECK_YET, Conv::Cdecl, &[k, id], |g| {
            Out::U32(cache::bridge::check_yet(g, kind, id))
        });
        for (sym, off) in CACHE_EXIST_FIELD {
            t.check(&what, sym, Conv::Cdecl, &[k, id], |g| {
                Out::U32(cache::bridge::exist_field(g, kind, id, off))
            });
        }
        // the real Sg_MemSetType runs in both
        t.check(&what, 0x0084_0dd0, Conv::Cdecl, &[k, id], |g| {
            cache::bridge::use_entry(g, kind, id);
            Out::None
        });
    }
}

// ---- round 5: the scene state `ffx-render` reads ----

/// Random floats as bytes (no signalling NaNs).
fn float_bytes(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).flat_map(|_| rng.float().to_le_bytes()).collect()
}

/// A camera object whose virtual `1` (the update) is a recording stub.
fn build_camera(t: &mut Twins, rng: &mut Rng, tag: u32) -> u32 {
    let update = t.stub(None, tag, true, 0, 0, rng.next());
    let vtable = t.zeros(0x10);
    t.w32(vtable + 4, update);
    let camera = t.alloc(&rng.bytes(0x200));
    t.w32(camera, vtable);
    camera
}

/// A character instance list: a count (bit 31 a flag), the instances
/// (each a skeleton with `n` bones and its default matrices, and a skin
/// matrix buffer), and per instance a bone-index table.
fn build_skinned(t: &mut Twins, rng: &mut Rng, bones: u32) -> u32 {
    let n = rng.below(4);
    let mut insts = Vec::new();
    let mut tables = Vec::new();
    for _ in 0..n {
        let count = rng.below(6);
        let skel = t.zeros(0x20);
        t.w32(skel + 0x18, count | (rng.next() & 0x8000_0000));
        let defaults = float_bytes(rng, 16 * count as usize);
        let defaults = t.alloc(&defaults);
        t.w32(skel + 0x1c, defaults);
        let out = float_bytes(rng, 16 * count as usize);
        let out = t.alloc(&out);
        let inst = t.zeros(0x20);
        t.w32(inst, skel);
        t.w32(inst + 0xc, out);
        insts.push(inst);
        let index: Vec<u8> = (0..count)
            .flat_map(|_| {
                let b = if rng.chance(4) {
                    0x40 + rng.below(4)
                } else {
                    rng.below(bones)
                };
                b.min(bones - 1).to_le_bytes()
            })
            .collect();
        tables.push(t.alloc(&index));
    }
    let items: Vec<u8> = insts.iter().flat_map(|x| x.to_le_bytes()).collect();
    let items = t.alloc(&items);
    let tbl: Vec<u8> = tables.iter().flat_map(|x| x.to_le_bytes()).collect();
    let tbl = t.alloc(&tbl);
    let holder = t.zeros(0x20);
    t.w32(holder + 0x14, tbl);
    let list = t.zeros(0x20);
    t.w32(list, n | (rng.next() & 0x8000_0000));
    t.w32(list + 4, items);
    t.w32(list + 0x10, holder);
    list
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function; colour components
fn round5_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x5005_0005);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        let app = t.alloc(&float_bytes(&mut rng, 0x100));
        t.global(st::APPLICATION.sym, app);
        let pp = t.alloc(&rng.bytes(0x12400));
        // the fade colour: floats (no signalling NaNs, which the
        // emulator's fld keeps)
        let fade = float_bytes(&mut rng, 4);
        t.write(pp + st::FADE_COLOR, &fade);
        t.w8(pp + st::FADE_LOCKED, u8::from(rng.chance(2)));
        t.global(st::POST_PROCESS.sym, pp);
        let ptr = |t: &mut Twins, rng: &mut Rng| if rng.chance(4) { 0 } else { t.zeros(4) };
        match round % 16 {
            0 => {
                let out = [
                    ptr(&mut t, &mut rng),
                    ptr(&mut t, &mut rng),
                    ptr(&mut t, &mut rng),
                ];
                let (sym, base) = if rng.chance(2) {
                    (0x0064_1160, st::FOG_COLOR)
                } else {
                    (0x0064_1230, st::AMBIENT)
                };
                t.check(&what, sym, Conv::Cdecl, &out, |g| {
                    st::get_app_color(g, base, out);
                    Out::None
                });
                let (sym, off) = [
                    (0x0064_11d0, st::FOG_NEAR),
                    (0x0064_11b0, st::FOG_FAR),
                    (0x0064_11c0, st::FOG_LIMIT),
                ][rng.below(3) as usize];
                t.check(&what, sym, Conv::Cdecl, &[], |g| {
                    Out::F32(st::app_float(g, off))
                });
            }
            1 => {
                let rgb: [f32; 3] = std::array::from_fn(|_| rng.float());
                let args = rgb.map(f32::to_bits);
                let (sym, base) = if rng.chance(2) {
                    (0x0064_3270, st::AMBIENT)
                } else {
                    (0x0064_2e90, st::BACKGROUND)
                };
                t.check(&what, sym, Conv::Cdecl, &args, |g| {
                    Out::U32(st::set_app_color(g, base, rgb))
                });
                let on = if rng.chance(2) { 0 } else { rng.next() };
                t.check(&what, 0x0064_01e0, Conv::Cdecl, &[on], |g| {
                    Out::U32(st::enable_keyframe_anim(g, on))
                });
            }
            2 => {
                let out: [u32; 4] = std::array::from_fn(|_| ptr(&mut t, &mut rng));
                t.check(&what, 0x0064_1390, Conv::Cdecl, &out, |g| {
                    Out::U32(st::get_screen_fade_color(g, out))
                });
                t.check(&what, 0x0064_13e0, Conv::Cdecl, &[], |g| {
                    Out::U32(st::screen_fade_last_reason(g))
                });
                t.check(&what, 0x0064_1cb0, Conv::Cdecl, &[], |g| {
                    Out::U32(st::is_screen_fade_color_locked(g))
                });
                let (sym, off, v) = [
                    (0x0064_1e20, st::FADE_LOCKED, 1),
                    (0x0064_1660, st::RAN_DRAW_FILTER, 1),
                    (0x0064_1670, st::RAN_DRAW_FOCUS, 1),
                    (0x0064_2c70, st::RAN_DRAW_FOCUS, 0),
                ][rng.below(4) as usize];
                t.check(&what, sym, Conv::Cdecl, &[], |g| {
                    Out::U32(st::set_post_process_byte(g, off, v))
                });
            }
            3 => {
                let rgba: [f32; 4] = std::array::from_fn(|_| rng.float());
                let args = rgba.map(f32::to_bits);
                t.check(&what, 0x0064_3900, Conv::Cdecl, &args, |g| {
                    Out::U32(st::set_screen_fade_color(g, rgba))
                });
                let v = rng.next();
                t.check(&what, 0x0064_38e0, Conv::Cdecl, &[v], |g| {
                    Out::U32(st::set_screen_fade_alpha_mode(g, v))
                });
                t.check(&what, 0x0064_3940, Conv::Cdecl, &[v], |g| {
                    Out::U32(st::set_screen_fade_last_reason(g, v))
                });
                t.check(&what, 0x0064_48d0, Conv::Cdecl, &[], |g| {
                    Out::U32(st::unlock_screen_fade_color(g))
                });
            }
            4 => {
                t.stub_sym(
                    st::POST_PROCESS_SET_FOCUS_DEPTH,
                    Conv::Thiscall(0),
                    2,
                    rng.next(),
                );
                let which = rng.below(4);
                let z = match rng.below(4) {
                    0 => 0,
                    1 => 16_770_000 - rng.below(0x10000),
                    2 => rng.below(0x0100_0000),
                    _ => rng.next(),
                };
                t.check(&what, 0x0064_30f0, Conv::Cdecl, &[which, z], |g| {
                    Out::U32(st::set_focus_z(g, which, z))
                });
            }
            5 => {
                let scene = build_scene(&mut t, &mut rng);
                let _ = scene;
                let (sym, method, n) = [
                    (0x0064_27e0, st::SCENE_PREVIOUS_LEVEL_NAME_HASH, 0),
                    (0x0064_1b90, st::SCENE_IS_3D, 0),
                    (0x0064_1c00, st::SCENE_IS_DRAWING_NON_UI_2D, 0),
                    (0x0064_2030, st::SCENE_CURRENT_MAGIC_PATH, 0),
                    (0x0064_2040, st::SCENE_TO_BE_DELETE_MAGIC_PATH, 0),
                    (0x0063_fc10, st::SCENE_SET_BYTE_01, 1),
                    (0x0063_b5b0, st::SCENE_REMOVE_CHR_FROM_PIPELINE, 1),
                ][rng.below(7) as usize];
                t.stub_sym(method, Conv::Thiscall(0), n, rng.next());
                let args: Vec<u32> = (0..n).map(|_| rng.next()).collect();
                t.check(&what, sym, Conv::Cdecl, &args, |g| {
                    Out::U32(st::scene_call(g, method, &args))
                });
            }
            6 => {
                let mgr = t.zeros(0x20);
                t.global(st::ASYNC_LOAD.sym, mgr);
                t.stub_sym(st::ASYNC_LOAD_ALL_DONE, Conv::Thiscall(0), 0, rng.next());
                t.check(&what, 0x0064_1ba0, Conv::Cdecl, &[], |g| {
                    Out::U32(st::is_all_scene_assets_loaded(g))
                });
                let fog = t.zeros(0x20);
                t.global(st::FOG_EFFECT.sym, fog);
                t.stub_sym(st::FOG_SET_CHR_FADE_TYPE, Conv::Thiscall(0), 1, rng.next());
                let on = if rng.chance(2) { 0 } else { rng.next() };
                t.check(&what, 0x0064_2f10, Conv::Cdecl, &[on], |g| {
                    Out::U32(st::set_chr_fog_fade(g, on))
                });
                let ta = t.zeros(0x20);
                t.global(st::TEXTURE_ANIMATION.sym, ta);
                let v = rng.next();
                t.check(&what, 0x0064_2d90, Conv::Cdecl, &[v], |g| {
                    Out::U32(st::set_2d_start_layer(g, v))
                });
            }
            7 => {
                let menu = if rng.chance(4) { 0 } else { t.zeros(0x20) };
                t.global(st::GRAPHIC_UI.sym, menu);
                t.stub_sym(st::MENU_SET_FONT_SHADOW, Conv::Thiscall(0), 1, rng.next());
                let v = rng.next();
                t.check(&what, 0x0064_0dd0, Conv::Cdecl, &[v], |g| {
                    st::font_set_ui_shadow_type(g, v);
                    Out::None
                });
                let tips = t.zeros(0x20);
                t.global(st::TIP_SYSTEM.sym, tips);
                let (sym, method) = if rng.chance(2) {
                    (0x0064_2910, st::TIP_PUSH)
                } else {
                    (0x0064_2930, st::TIP_PUSH_BY_LOC_ID)
                };
                t.stub_sym(method, Conv::Thiscall(0), 1, rng.next());
                t.check(&what, sym, Conv::Cdecl, &[v], |g| {
                    Out::U32(st::push_tip(g, method, v))
                });
                let out = t.zeros(8);
                let (x, y) = if rng.chance(2) {
                    (rng.below(1024), rng.below(1024))
                } else {
                    (rng.next(), rng.next())
                };
                t.check(
                    &what,
                    0x0063_de10,
                    Conv::Cdecl,
                    &[x, y, out, out + 4],
                    |g| Out::U32(st::convert_screen_coords(g, x, y, out, out + 4)),
                );
            }
            8 => {
                t.stub_sym(st::PHYRE_PRINTF, Conv::Cdecl, 2, 0);
                let key = u64::from(rng.next()) | u64::from(rng.below(3)) << 32;
                let (table, entries) = build_table(&mut t, &[key], &mut rng);
                t.global(vfx::bridge::TABLE.sym, table);
                let virt = t.stub(
                    None,
                    0x0004,
                    true,
                    2,
                    2,
                    if rng.chance(2) { 0 } else { rng.next() },
                );
                let vtable = t.zeros(0x10);
                t.w32(vtable + 4, virt);
                for &(_, g) in &entries {
                    t.w32(g, vtable);
                    if !rng.chance(4) {
                        let data = t.zeros(0x100);
                        t.w32(g + 0x24, data);
                    }
                }
                let (lo, hi) = match rng.below(4) {
                    0 => (0, 0),
                    1 => (rng.next(), rng.below(3)),
                    _ => split(key),
                };
                t.check(&what, 0x0064_1490, Conv::Cdecl, &[lo, hi], |g| {
                    Out::U32(st::guidemap_setup(g, lo, hi))
                });
            }
            9 => {
                let l = t.zeros(0x20);
                t.global(st::CHARACTER_LIGHTING.sym, l);
                let light = rng.below(4);
                let v: [f32; 3] = std::array::from_fn(|_| rng.float());
                let [x, y, z] = v.map(f32::to_bits);
                match rng.below(6) {
                    0 => {
                        t.stub_sym(st::LIGHTING_SET_LIGHT_DIR, Conv::Thiscall(0), 4, rng.next());
                        t.check(&what, 0x0063_cb50, Conv::Cdecl, &[light, x, y, z], |g| {
                            Out::U32(st::lighting_set_light_dir(g, light, v))
                        });
                    }
                    1 => {
                        let (a, b) = (rng.next(), rng.next());
                        t.stub_sym(
                            st::LIGHTING_SET_MAP_ID_FLAG_CHANGED,
                            Conv::Thiscall(0),
                            2,
                            rng.next(),
                        );
                        t.check(&what, 0x0063_cd20, Conv::Cdecl, &[a, b], |g| {
                            Out::U32(st::lighting_set_map_id_flag_changed(g, a, b))
                        });
                    }
                    2 | 3 => {
                        let (sym, method) = if rng.chance(2) {
                            (0x0063_cd40, st::LIGHTING_SET_POINT_LIGHT_COLOR)
                        } else {
                            (0x0063_cd70, st::LIGHTING_SET_POINT_LIGHT_POSITION)
                        };
                        t.stub_sym(method, Conv::Thiscall(0), 4, rng.next());
                        t.check(&what, sym, Conv::Cdecl, &[light, x, y, z], |g| {
                            Out::U32(st::lighting_set_point_light3(g, method, light, v))
                        });
                    }
                    4 => {
                        t.stub_sym(
                            st::LIGHTING_SET_POINT_LIGHT_RANGE,
                            Conv::Thiscall(0),
                            3,
                            rng.next(),
                        );
                        t.check(&what, 0x0063_cda0, Conv::Cdecl, &[light, x, y], |g| {
                            Out::U32(st::lighting_set_point_light_range(g, light, [v[0], v[1]]))
                        });
                    }
                    _ => {
                        t.stub_sym(
                            st::LIGHTING_SET_SHADE_COLOR,
                            Conv::Thiscall(0),
                            4,
                            rng.next(),
                        );
                        let list = if rng.chance(6) {
                            0
                        } else {
                            let n = rng.below(4);
                            let items: Vec<u8> =
                                (0..n).flat_map(|_| rng.next().to_le_bytes()).collect();
                            let items = t.alloc(&items);
                            let list = t.zeros(0x20);
                            t.w32(list, n | (rng.next() & 0x8000_0000));
                            t.w32(list + 4, items);
                            list
                        };
                        t.check(&what, 0x0063_d050, Conv::Cdecl, &[list, x, y, z], |g| {
                            st::lighting_set_shade_color(g, list, v);
                            Out::None
                        });
                    }
                }
            }
            10 => {
                build_scene(&mut t, &mut rng);
                let list = if rng.chance(6) {
                    0
                } else {
                    let n = rng.below(4);
                    let mut items = Vec::new();
                    for _ in 0..n {
                        let mesh = if rng.chance(5) { 0 } else { t.zeros(0x20) };
                        if mesh != 0 && !rng.chance(4) {
                            let draw = t.alloc(&rng.bytes(0x20));
                            t.w32(mesh + 0x1c, draw);
                        }
                        items.extend(mesh.to_le_bytes());
                    }
                    let items = t.alloc(&items);
                    let list = t.alloc(&rng.bytes(0x20));
                    t.w32(list, n | (rng.next() & 0x8000_0000));
                    t.w32(list + 4, items);
                    list
                };
                let visible = if rng.chance(2) { 1 } else { rng.below(3) };
                t.check(&what, 0x0063_c390, Conv::Cdecl, &[list, visible], |g| {
                    st::character_set_visible(g, list, visible);
                    Out::None
                });
            }
            11 => {
                let nbones = 0x48;
                let bones = float_bytes(&mut rng, 16 * nbones as usize);
                let bones = t.alloc(&bones);
                let list = if rng.chance(8) {
                    0
                } else {
                    build_skinned(&mut t, &mut rng, nbones)
                };
                if rng.chance(2) {
                    let chr = if rng.chance(2) {
                        0x206a
                    } else {
                        rng.below(0x3000)
                    };
                    t.check(&what, 0x0063_be60, Conv::Cdecl, &[chr, list, bones], |g| {
                        st::character_set_skeleton_matrices(g, chr, list, bones);
                        Out::None
                    });
                } else {
                    let bone = rng.below(nbones);
                    t.check(&what, 0x0063_bfc0, Conv::Cdecl, &[list, bone], |g| {
                        st::character_restore_skeleton_matrix(g, list, bone);
                        Out::None
                    });
                }
            }
            12 => {
                let scene = build_scene(&mut t, &mut rng);
                t.global(st::SKIP_FRAME.sym, u32::from(rng.chance(8)));
                let mut view = float_bytes(&mut rng, 16);
                if rng.chance(3) {
                    // small values, so the inverse is a number
                    view = (0..16)
                        .flat_map(|_| (rng.below(2000) as f32 / 100.0 - 10.0).to_le_bytes())
                        .collect();
                }
                if rng.chance(3) {
                    let r = rng.below(4) as usize;
                    view[16 * r..16 * r + 16].fill(0);
                    if rng.chance(3) {
                        view[16 * r + 4 * rng.below(4) as usize + 3] = 0x80; // -0
                    }
                }
                let view = t.alloc(&view);
                t.check(&what, 0x0063_9860, Conv::Cdecl, &[view], |g| {
                    st::camera_set_world_matrix(g, view);
                    Out::None
                });
                let m = t.alloc(&float_bytes(&mut rng, 12));
                let alt = rng.next();
                t.check(&what, 0x0066_ce70, Conv::Thiscall(scene), &[m, alt], |g| {
                    Out::U32(st::scene_set_camera_world_matrix(g, scene, m, alt))
                });
            }
            13 => {
                let scene = build_scene(&mut t, &mut rng);
                for (at, tag) in [(0xdc, 0x10), (0x654, 0x11), (0x238, 0x12)] {
                    let camera = build_camera(&mut t, &mut rng, tag);
                    t.w32(scene + at, camera);
                }
                t.w8(scene + 0x93c, u8::from(rng.chance(2)));
                t.w32(
                    scene + 0xfaf4,
                    [0xec1, 0x107, 0xf48, 0x123][rng.below(4) as usize],
                );
                t.w32(
                    scene + 0x10d8c,
                    if rng.chance(2) { 1 } else { rng.below(3) },
                );
                t.w32(scene + 0x10d90, rng.below(3));
                t.w32(
                    app + 0x26c,
                    if rng.chance(4) {
                        rng.next()
                    } else {
                        640 + rng.below(3000)
                    },
                );
                t.w32(
                    app + 0x270,
                    if rng.chance(4) {
                        rng.next()
                    } else {
                        480 + rng.below(2000)
                    },
                );
                t.global(scene::bridge::IS_3D.sym, u32::from(rng.chance(2)));
                let v = rng.float().to_bits();
                if rng.chance(2) {
                    t.check(&what, 0x0066_cda0, Conv::Thiscall(scene), &[v], |g| {
                        Out::U32(st::scene_set_camera_fovy(g, scene, v))
                    });
                } else {
                    t.check(&what, 0x0066_cbe0, Conv::Thiscall(scene), &[v], |g| {
                        Out::U32(st::scene_set_camera_aspect_ratio(g, scene, v))
                    });
                }
            }
            14 => {
                let scene = build_scene(&mut t, &mut rng);
                t.stub_sym(st::CAMERA_UPDATE_VIEW, Conv::Thiscall(0), 0, rng.next());
                if rng.chance(2) {
                    let camera = if rng.chance(4) {
                        0
                    } else {
                        t.alloc(&rng.bytes(0x100))
                    };
                    t.w32(scene + 0x654, camera);
                    let m = t.alloc(&float_bytes(&mut rng, 12));
                    t.check(&what, 0x0066_d940, Conv::Thiscall(scene), &[m], |g| {
                        st::scene_set_minimap_camera_world_matrix(g, scene, m);
                        Out::None
                    });
                } else {
                    let p: [u32; 5] = std::array::from_fn(|i| {
                        if i == 4 && rng.chance(2) {
                            (1 + rng.below(8)) as f32
                        } else if rng.chance(2) {
                            rng.below(2000) as f32 - 1000.0
                        } else {
                            rng.float()
                        }
                        .to_bits()
                    });
                    t.check(&what, 0x0066_ca60, Conv::Thiscall(scene), &p, |g| {
                        Out::U32(st::scene_set_abmap_camera(g, scene, p))
                    });
                }
                let camera = build_camera(&mut t, &mut rng, 0x13);
                t.w32(scene + 0xdc, camera);
                let far = rng.float().to_bits();
                t.check(&what, 0x0064_5e90, Conv::Cdecl, &[far], |g| {
                    Out::U32(st::set_far_plane(g, far))
                });
                let mesh = t.alloc(&rng.bytes(0x20));
                let world = t.alloc(&float_bytes(&mut rng, 12));
                t.w32(mesh + 4, world);
                let draw = if rng.chance(3) {
                    0
                } else {
                    t.alloc(&rng.bytes(0x20))
                };
                t.w32(mesh + 0x1c, draw);
                let m = t.alloc(&float_bytes(&mut rng, 16));
                t.check(&what, 0x0066_d810, Conv::Stdcall, &[mesh, m], |g| {
                    Out::U32(st::scene_set_mesh_instance_world_matrix(g, mesh, m))
                });
                t.stub_sym(
                    st::SCENE_SET_TEXTURE_VIDEO_INFO,
                    Conv::Thiscall(0),
                    2,
                    rng.next(),
                );
                let (a, b) = (rng.next(), rng.next());
                t.check(&what, 0x0064_3aa0, Conv::Cdecl, &[a, b], |g| {
                    Out::U32(st::set_texture_video_info(g, a, b))
                });
                t.stub_sym(
                    st::POST_PROCESS_SET_CROSS_FADE_WEIGHT,
                    Conv::Thiscall(0),
                    1,
                    rng.next(),
                );
                t.stub_sym(
                    st::POST_PROCESS_SET_CROSS_FADE_TIME,
                    Conv::Thiscall(0),
                    1,
                    rng.next(),
                );
                t.check(&what, 0x0064_4910, Conv::Cdecl, &[a, b], |g| {
                    Out::U32(st::update_cross_fade(g, a, b))
                });
                let name = rng.next();
                t.stub_sym(st::PSTRING_ASSIGN, Conv::Thiscall(0), 1, rng.next());
                t.check(&what, 0x0066_cf80, Conv::Thiscall(scene), &[name], |g| {
                    Out::U32(st::scene_set_current_level_name(g, scene, name))
                });
            }
            _ => {
                let scene = build_scene(&mut t, &mut rng);
                t.w8(scene + 0x93c, u8::from(!rng.chance(6)));
                t.w32(scene + 0x1128, u32::from(!rng.chance(6)));
                let n = rng.below(6);
                t.w32(scene + 0x1138, n);
                t.w32(scene + 0x11d8, rng.below(3));
                let mut layers = Vec::new();
                for _ in 0..n {
                    let mesh = t.zeros(0x20);
                    if !rng.chance(4) {
                        let draw = t.alloc(&rng.bytes(0x20));
                        t.w32(mesh + 0x1c, draw);
                    }
                    layers.push(rng.below(2) as u8);
                    layers.push(rng.below(3) as u8);
                    layers.extend((rng.below(4) as u16).to_le_bytes());
                    layers.extend(mesh.to_le_bytes());
                }
                let layers = t.alloc(&layers);
                t.w32(scene + 0x1140, layers);
                let id = if rng.chance(8) {
                    rng.next()
                } else {
                    rng.below(4)
                };
                let show = rng.chance(2);
                let sym = if show { 0x0066_f2f0 } else { 0x0065_f050 };
                t.check(&what, sym, Conv::Thiscall(scene), &[id], |g| {
                    st::scene_set_2d_layer_shown(g, scene, id, show);
                    Out::None
                });
            }
        }
    }
}

#[test]
fn snapshot_scene_state_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    if t.r32(t.rebase(st::APPLICATION.sym)) == 0 || t.r32(t.rebase(st::SCENE.sym)) == 0 {
        return;
    }
    let mut rng = Rng(0x5005_0006);
    let out = t.zeros(16);
    let o = [out, out + 4, out + 8];
    t.check("fog colour", 0x0064_1160, Conv::Cdecl, &o, |g| {
        st::get_app_color(g, st::FOG_COLOR, o);
        Out::None
    });
    t.check("ambient", 0x0064_1230, Conv::Cdecl, &o, |g| {
        st::get_app_color(g, st::AMBIENT, o);
        Out::None
    });
    for (sym, off) in [
        (0x0064_11d0, st::FOG_NEAR),
        (0x0064_11b0, st::FOG_FAR),
        (0x0064_11c0, st::FOG_LIMIT),
    ] {
        t.check("fog", sym, Conv::Cdecl, &[], |g| {
            Out::F32(st::app_float(g, off))
        });
    }
    if t.r32(t.rebase(st::POST_PROCESS.sym)) != 0 {
        let o = [out, out + 4, out + 8, out + 12];
        t.check("fade colour", 0x0064_1390, Conv::Cdecl, &o, |g| {
            Out::U32(st::get_screen_fade_color(g, o))
        });
    }
    // the game's own view matrix (the one Cam_Apply passes)
    let view = t.rebase(0x0231_1440);
    t.check("camera", 0x0063_9860, Conv::Cdecl, &[view], |g| {
        st::camera_set_world_matrix(g, view);
        Out::None
    });
    let scene = t.r32(t.rebase(st::SCENE.sym));
    for round in 0..200 {
        let what = format!("snapshot round {round}");
        let id = rng.below(16);
        let show = rng.chance(2);
        let sym = if show { 0x0066_f2f0 } else { 0x0065_f050 };
        t.check(&what, sym, Conv::Thiscall(scene), &[id], |g| {
            st::scene_set_2d_layer_shown(g, scene, id, show);
            Out::None
        });
    }
}

// ---- round 6 ----

/// Puts a stub over the original at `sym` that leaves the float at a
/// fresh cell in `ST0` and then records the call like [`Twins::stub`]
/// (`thiscall`, `n` arguments). Returns the cell.
fn stub_st0(t: &mut Twins, sym: u32, n: u32, value: f32) -> u32 {
    let cell = t.alloc(&value.to_le_bytes());
    let rec = t.stub(None, sym, true, n, n, 0);
    let at = t.rebase(sym);
    let mut c = vec![0xd9, 0x05];
    c.extend(cell.to_le_bytes()); // fld dword [cell]
    c.push(0xe9);
    c.extend(rec.wrapping_sub(at + 11).to_le_bytes()); // jmp rec
    t.write(at, &c);
    cell
}

/// A search tree (head `+0xc` root; nodes `+0`/`+4` children, `+0xc`
/// key, `+0x10` value) holding `keys` with values from `value`, linked by
/// insertion. Returns the head.
fn build_key_tree(t: &mut Twins, keys: &[u32], mut value: impl FnMut(&mut Twins) -> u32) -> u32 {
    let head = t.zeros(0x20);
    t.w32(head + 0xc, head);
    for &k in keys {
        let n = t.zeros(0x20);
        t.w32(n, head);
        t.w32(n + 4, head);
        t.w32(n + 0xc, k);
        let v = value(t);
        t.w32(n + 0x10, v);
        let mut at = head + 0xc;
        loop {
            let cur = t.r32(at);
            if cur == head {
                t.w32(at, n);
                break;
            }
            let ck = t.r32(cur + 0xc);
            if k == ck {
                break;
            }
            at = cur + if k < ck { 0 } else { 4 };
        }
    }
    head
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round6_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x6006_0006);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 3 {
            0 => {
                let scene = build_scene(&mut t, &mut rng);
                let l = t.zeros(0x20);
                t.global(st::CHARACTER_LIGHTING.sym, l);
                let tweak = t.zeros(0x20);
                t.global(st::LIGHTING_TWEAK.sym, tweak);
                let _ = scene;
                let coeff = rng.float();
                let c = match rng.below(4) {
                    0 => coeff + (rng.below(30) as f32 - 15.0) / 100.0,
                    1 => coeff,
                    _ => rng.float(),
                };
                stub_st0(&mut t, st::LIGHTING_CALC_SHADE_COEFF, 3, c);
                let hash = [0x226d, 0x1739, 0x1c45, 0xf62, 0xc04, 0x123, rng.next()]
                    [rng.below(7) as usize];
                t.stub_sym(
                    st::SCENE_CURRENT_LEVEL_NAME_HASH,
                    Conv::Thiscall(0),
                    0,
                    hash,
                );
                t.stub_sym(
                    st::LIGHTING_SET_SHADE_COEFF,
                    Conv::Thiscall(0),
                    2,
                    rng.next(),
                );
                t.stub_sym(
                    st::LIGHTING_TWEAK_SET_SHADE_COEFF,
                    Conv::Thiscall(0),
                    3,
                    rng.next(),
                );
                let list = if rng.chance(6) {
                    0
                } else {
                    let n = rng.below(4);
                    let items: Vec<u8> = (0..n).flat_map(|_| rng.next().to_le_bytes()).collect();
                    let items = t.alloc(&items);
                    let list = t.zeros(0x20);
                    t.w32(list, n | (rng.next() & 0x8000_0000));
                    t.w32(list + 4, items);
                    list
                };
                t.check(
                    &what,
                    0x0063_cdd0,
                    Conv::Cdecl,
                    &[list, coeff.to_bits()],
                    |g| {
                        st::lighting_set_shade_coeff(g, list, coeff);
                        Out::None
                    },
                );
            }
            _ => {
                let scene = build_scene(&mut t, &mut rng);
                t.w32(scene + 0x10d4, u32::from(!rng.chance(6)));
                let keys: Vec<u32> = (0..rng.below(8)).map(|_| rng.below(20)).collect();
                let head = build_key_tree(&mut t, &keys, |t| {
                    let n = rng.below(4);
                    let mut meshes = Vec::new();
                    for _ in 0..n {
                        let mesh = t.alloc(&rng.bytes(0x20));
                        let world = t.alloc(&float_bytes(&mut rng, 12));
                        t.w32(mesh + 4, world);
                        let draw = t.alloc(&rng.bytes(0x20));
                        t.w32(mesh + 0x1c, draw);
                        meshes.extend(mesh.to_le_bytes());
                    }
                    let meshes = t.alloc(&meshes);
                    let list = t.zeros(0x10);
                    t.w32(list, n);
                    t.w32(list + 8, meshes);
                    list
                });
                // the scene's tree is embedded: copy the head in
                let h: Vec<u8> = (0..0x10).map(|i| t.r8(head + i)).collect();
                let tree = scene + 0x1198;
                t.write(tree, &h);
                // nodes point back at the old head; relink them
                let mut stack = vec![t.r32(tree + 0xc)];
                if stack[0] == head {
                    t.w32(tree + 0xc, tree);
                    stack.clear();
                }
                while let Some(n) = stack.pop() {
                    for off in [0, 4] {
                        let c = t.r32(n + off);
                        if c == head {
                            t.w32(n + off, tree);
                        } else {
                            stack.push(c);
                        }
                    }
                }
                let id = if rng.chance(5) {
                    rng.next()
                } else {
                    rng.below(20)
                };
                if round % 3 == 1 {
                    let pos = t.alloc(&float_bytes(&mut rng, 3));
                    t.check(&what, 0x0066_e3e0, Conv::Thiscall(scene), &[id, pos], |g| {
                        Out::U32(st::scene_set_object_translation(g, scene, id, pos))
                    });
                } else {
                    let key = t.alloc(&id.to_le_bytes());
                    t.check(&what, 0x0064_b9f0, Conv::Thiscall(tree), &[key], |g| {
                        Out::U32(managers::bridge::tree_find(g, tree, key))
                    });
                }
            }
        }
    }
}

/// A `std::map` of `n` nodes (MSVC layout: `+0` left, `+4` parent, `+8`
/// right, `+0xd` the head flag), balanced; returns the head and the nodes
/// in order.
fn build_std_map(t: &mut Twins, n: usize) -> (u32, Vec<u32>) {
    fn link(t: &mut Twins, nodes: &[u32], head: u32, parent: u32) -> u32 {
        if nodes.is_empty() {
            return head;
        }
        let mid = nodes.len() / 2;
        let node = nodes[mid];
        t.w32(node + 4, parent);
        let l = link(t, &nodes[..mid], head, node);
        let r = link(t, &nodes[mid + 1..], head, node);
        t.w32(node, l);
        t.w32(node + 8, r);
        node
    }
    let head = t.zeros(0x20);
    t.w8(head + 0xd, 1);
    let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
    let root = link(t, &nodes, head, head);
    t.w32(head + 4, root);
    t.w32(head, nodes.first().copied().unwrap_or(head));
    t.w32(head + 8, nodes.last().copied().unwrap_or(head));
    (head, nodes)
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function
fn round6_helpers_match_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x6006_0007);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 7 {
            0 => {
                let mut v: Vec<f32> = (0..4).map(|_| rng.float()).collect();
                if rng.chance(2) {
                    for i in 0..3 {
                        v[i] = v[3] * [1.0, -1.0, 0.5, 2.0, -2.0][rng.below(5) as usize];
                    }
                }
                let p = t.alloc(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>());
                t.check(&what, 0x0065_2470, Conv::Cdecl, &[p], |g| {
                    Out::U32(ut::clip_outcode(g, p))
                });
            }
            1 => {
                let id = if rng.chance(2) {
                    ut::LISTED_MAPS[rng.below(16) as usize]
                } else {
                    rng.below(0x200)
                };
                t.global(ut::MAP_ID.sym, id);
                t.check(&what, 0x0064_2610, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::is_map_in_list(g))
                });
            }
            2 => {
                let n = rng.below(6);
                let src = t.alloc(&rng.bytes(12 * n as usize + 4));
                // a null dest only skips the first element (the next is at 0xc)
                let dest = if n <= 1 && rng.chance(2) {
                    0
                } else {
                    t.zeros(12 * n as usize + 4)
                };
                t.check(
                    &what,
                    0x006b_4f80,
                    Conv::Cdecl,
                    &[src, src + 12 * n, dest],
                    |g| Out::U32(ut::vec3_uninitialized_copy(g, src, src + 12 * n, dest)),
                );
                let obj = t.alloc(&rng.bytes(0x210 + 64 * 4));
                let out = t.zeros(64);
                let i = rng.below(4);
                t.check(&what, 0x006d_23a0, Conv::Thiscall(obj), &[out, i], |g| {
                    Out::U32(ut::shadow_get_cascade_matrix(g, obj, out, i))
                });
            }
            3 => {
                // a float `less` (cdecl): al = a < b, in the cave (code runs there)
                let less = t.cave_next;
                let code = [
                    0xd9, 0x44, 0x24, 0x04, 0xd8, 0x5c, 0x24, 0x08, 0xdf, 0xe0, 0xf6, 0xc4, 0x01,
                    0x0f, 0x95, 0xc0, 0xc3,
                ];
                t.write(less, &code);
                t.cave_next += code.len() as u32;
                let n = 1 + rng.below(12);
                let base = t.alloc(&float_bytes(&mut rng, n as usize));
                let len = 1 + rng.below(n);
                let hole = rng.below(len);
                let value = t.alloc(&float_bytes(&mut rng, 1));
                t.check(
                    &what,
                    0x006c_7250,
                    Conv::Cdecl,
                    &[base, hole, len, value, less],
                    |g| Out::U32(ut::float_adjust_heap(g, base, hole, len, value, less)),
                );
            }
            4 => {
                let (head, nodes) = build_std_map(&mut t, rng.below(9) as usize);
                let node = if nodes.is_empty() || rng.chance(5) {
                    head
                } else {
                    nodes[rng.below(nodes.len() as u32) as usize]
                };
                let it = t.alloc(&node.to_le_bytes());
                t.check(&what, 0x0064_f360, Conv::Fastcall, &[it], |g| {
                    Out::U32(ut::map_iterator_decrement(g, it))
                });
            }
            5 => {
                let keys: Vec<u32> = (0..rng.below(8)).map(|_| rng.below(20)).collect();
                let head = build_key_tree(&mut t, &keys, |_| 0);
                let key = t.alloc(&rng.below(22).to_le_bytes());
                t.check(&what, 0x006b_e700, Conv::Thiscall(head), &[key], |g| {
                    Out::U32(managers::bridge::tree_find(g, head, key))
                });
            }
            _ => {
                let words = ["", "a", "ab", "abc", "b", "ba", "bsil", "bsil0000", "z"];
                let text = |t: &mut Twins, rng: &mut Rng| -> u32 {
                    if rng.chance(8) {
                        return rng.below(2);
                    }
                    let w = words[rng.below(words.len() as u32) as usize];
                    let mut b = w.as_bytes().to_vec();
                    b.push(0);
                    t.alloc(&b) | rng.below(2)
                };
                // a tree of distinct texts, inserted by the same order
                let head = t.zeros(0x20);
                t.w32(head + 0xc, head);
                let mut placed: Vec<String> = Vec::new();
                for _ in 0..rng.below(8) {
                    let w = words[rng.below(words.len() as u32) as usize].to_string();
                    if placed.contains(&w) {
                        continue;
                    }
                    let mut b = w.as_bytes().to_vec();
                    b.push(0);
                    let p = t.alloc(&b) | rng.below(2);
                    let n = t.zeros(0x20);
                    t.w32(n, head);
                    t.w32(n + 4, head);
                    t.w32(n + 0xc, p);
                    let mut at = head + 0xc;
                    loop {
                        let cur = t.r32(at);
                        if cur == head {
                            t.w32(at, n);
                            break;
                        }
                        let cw = &placed[(t.r32(cur + 0x10)) as usize];
                        at = cur + if w.as_str() < cw.as_str() { 0 } else { 4 };
                    }
                    t.w32(n + 0x10, placed.len() as u32);
                    placed.push(w);
                }
                let k = text(&mut t, &mut rng);
                let key = t.alloc(&k.to_le_bytes());
                t.check(&what, 0x006b_e750, Conv::Thiscall(head), &[key], |g| {
                    Out::U32(ut::pstring_tree_find(g, head, key))
                });
            }
        }
    }
}

/// A float `less` (cdecl, `al = a < b`) written into the cave.
fn cave_float_less(t: &mut Twins) -> u32 {
    let less = t.cave_next;
    let code = [
        0xd9, 0x44, 0x24, 0x04, 0xd8, 0x5c, 0x24, 0x08, 0xdf, 0xe0, 0xf6, 0xc4, 0x01, 0x0f, 0x95,
        0xc0, 0xc3,
    ];
    t.write(less, &code);
    t.cave_next += code.len() as u32;
    less
}

/// Allocates a NUL-terminated string.
fn cstr(t: &mut Twins, s: &str) -> u32 {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    t.alloc(&b)
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function
fn round7_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x7007_0007);
    let names = ["", "a", "ab", "water", "water2", "fire"];
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 12 {
            0 => {
                let scene = build_scene(&mut t, &mut rng);
                let n = rng.below(5);
                t.w32(scene + 0x11c8, n);
                let mut meshes = Vec::new();
                for _ in 0..n {
                    let mesh = if rng.chance(5) { 0 } else { t.zeros(0x20) };
                    if mesh != 0 && !rng.chance(4) {
                        let draw = t.alloc(&rng.bytes(0x20));
                        t.w32(mesh + 0x1c, draw);
                    }
                    meshes.extend(mesh.to_le_bytes());
                }
                let meshes = t.alloc(&meshes);
                t.w32(scene + 0x11d0, meshes);
                let on = rng.next();
                t.check(&what, 0x0066_e7b0, Conv::Thiscall(scene), &[on], |g| {
                    st::scene_set_meshes_visible(g, scene, on);
                    Out::None
                });
            }
            1 => {
                let scene = build_scene(&mut t, &mut rng);
                let c: [f32; 6] = std::array::from_fn(|_| rng.float());
                let second = rng.next();
                let mut args: Vec<u32> = c.iter().map(|x| x.to_bits()).collect();
                args.push(second);
                t.check(&what, 0x0066_ea70, Conv::Thiscall(scene), &args, |g| {
                    st::scene_set_colour_pair(g, scene, c, second);
                    Out::None
                });
            }
            2 => {
                let scene = build_scene(&mut t, &mut rng);
                let n = rng.below(6);
                let states: Vec<u8> = (0..n)
                    .flat_map(|_| (if rng.chance(3) { rng.below(3) } else { 1 }).to_le_bytes())
                    .collect();
                let states = t.alloc(&states);
                t.w32(scene + 0x60, states);
                let segs: Vec<u8> = (0..n)
                    .flat_map(|_| (if rng.chance(3) { 0 } else { rng.next() | 1 }).to_le_bytes())
                    .collect();
                let segs = t.alloc(&segs);
                let obj = t.alloc(&rng.bytes(0x60));
                t.w8(obj + 0x24, u8::from(!rng.chance(5)));
                t.w32(obj + 0x54, segs);
                t.w32(obj + 0x58, n);
                t.check(&what, 0x0065_2250, Conv::Fastcall, &[obj], |g| {
                    Out::U32(st::segments_ready(g, obj))
                });
            }
            3 => {
                let sm = t.alloc(&rng.bytes(0x4070));
                let q = if rng.chance(4) {
                    rng.next()
                } else {
                    rng.below(5)
                };
                t.check(&what, 0x006c_eaa0, Conv::Thiscall(sm), &[q], |g| {
                    Out::U32(ut::shadow_set_quality(g, sm, q))
                });
                let v: [f32; 4] = std::array::from_fn(|_| rng.float());
                let args: Vec<u32> = v.iter().map(|x| x.to_bits()).collect();
                let b = t.alloc(&rng.bytes(0x70));
                t.check(&what, 0x006c_4620, Conv::Thiscall(b), &args, |g| {
                    ut::set_positive_floats(g, b, v);
                    Out::None
                });
                t.check(&what, 0x006c_47a0, Conv::Thiscall(b), &[], |g| {
                    Out::U32(ut::render_params_reset(g, b))
                });
            }
            4 => {
                let less = cave_float_less(&mut t);
                let n = 1 + rng.below(12);
                let base = t.alloc(&float_bytes(&mut rng, n as usize));
                let hole = rng.below(n);
                let top = if rng.chance(4) {
                    hole + rng.below(2)
                } else {
                    rng.below(hole + 1)
                };
                let value = t.alloc(&float_bytes(&mut rng, 1));
                t.check(
                    &what,
                    0x006c_7730,
                    Conv::Cdecl,
                    &[base, hole, top, value, less],
                    |g| Out::U32(ut::float_push_heap(g, base, hole, top, value, less)),
                );
            }
            5 => {
                let less = cave_float_less(&mut t);
                let v = t.alloc(&float_bytes(&mut rng, 3));
                let (a, b, c) = (v, v + 4, v + 8);
                t.check(&what, 0x006c_74d0, Conv::Cdecl, &[a, b, c, less], |g| {
                    // the callback's eax above al is its fnstsw: x87 state, not a result
                    Out::U8(ut::float_median3(g, a, b, c, less) as u8)
                });
            }
            6 => {
                use ffx_graphic::texanim::bridge as ta;
                let i = rng.below(4);
                let e = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i;
                let work = t.alloc(&rng.bytes(0x34 * 64 + 0x440));
                t.w32(e, u32::from(!rng.chance(4)));
                t.w32(e + 4, work);
                let (which, pos) = (rng.below(16), rng.next());
                let (sym, base) = if rng.chance(2) {
                    (0x0077_fc90, 0)
                } else {
                    (0x0078_0750, 0x400)
                };
                t.check(&what, sym, Conv::Cdecl, &[i, which, pos], |g| {
                    Out::U32(ut::tex_anim_start(g, base, i, which, pos))
                });
            }
            7 => {
                let obj = t.alloc(&rng.bytes(0xb0));
                t.check(&what, 0x006f_7700, Conv::Fastcall, &[obj], |g| {
                    Out::U32(ut::init_flash_table(g, obj))
                });
                let owner = rng.next();
                t.check(&what, 0x006d_3560, Conv::Thiscall(obj), &[owner], |g| {
                    Out::U32(ut::shadow_entry_init(g, obj, owner))
                });
            }
            8 => {
                let mgr = t.alloc(&rng.bytes(0x90));
                let n = rng.below(6);
                let mut mats = Vec::new();
                for _ in 0..n {
                    let m = if rng.chance(5) {
                        0
                    } else {
                        let m = t.zeros(0x20);
                        let s = cstr(&mut t, names[rng.below(6) as usize]);
                        t.w32(m + 0x10, s | rng.below(2));
                        m
                    };
                    mats.extend(m.to_le_bytes());
                }
                let mats = t.alloc(&mats);
                t.w32(mgr + 0x74, n | (rng.next() & 0x8000_0000));
                t.w32(mgr + 0x78, mats);
                let name = cstr(&mut t, names[rng.below(6) as usize]);
                t.check(&what, 0x006b_20a0, Conv::Thiscall(mgr), &[name], |g| {
                    Out::U32(ut::tex_anim_material_index(g, mgr, name))
                });
                let n = rng.below(6);
                let anims = t.zeros(0x20 * n as usize + 4);
                for k in 0..n {
                    let s = if rng.chance(5) {
                        rng.below(2)
                    } else {
                        cstr(&mut t, names[rng.below(6) as usize]) | rng.below(2)
                    };
                    t.w32(anims + 0x20 * k + 0x18, s);
                }
                t.w32(mgr + 0x64, n | (rng.next() & 0x8000_0000));
                t.w32(mgr + 0x68, anims);
                let name = if rng.chance(6) {
                    0
                } else {
                    cstr(&mut t, names[rng.below(6) as usize])
                };
                t.check(&what, 0x006b_1ef0, Conv::Thiscall(mgr), &[name], |g| {
                    Out::U32(ut::tex_anim_find_animation(g, mgr, name))
                });
            }
            9 => {
                let obj = t.alloc(&rng.bytes(0x90));
                let n = rng.below(5);
                let list = t.zeros(0x20 * n as usize + 4);
                for k in 0..n {
                    let s = t.stub(None, 0x40 + k, true, 1, 1, rng.next());
                    let vt = t.zeros(0x10);
                    t.w32(vt + 4, s);
                    t.w32(list + 0x20 * k, vt);
                }
                t.w32(obj + 0x7c, n | (rng.next() & 0x8000_0000));
                t.w32(obj + 0x80, list);
                let v = rng.float();
                t.check(
                    &what,
                    0x006b_47e0,
                    Conv::Thiscall(obj),
                    &[v.to_bits()],
                    |g| {
                        ut::set_all_params(g, obj, v);
                        Out::None
                    },
                );
            }
            10 => {
                t.check(&what, 0x006c_26d0, Conv::Cdecl, &[], |_| {
                    Out::U32(ut::RENDER_CONST_180)
                });
                t.check(&what, 0x006c_2700, Conv::Cdecl, &[], |_| {
                    Out::U32(ut::RENDER_CONST_51)
                });
                let f = rng.next() as u8;
                let at = t.rebase(ut::RENDER_FLAG.sym);
                t.w8(at, f);
                t.check(&what, 0x006b_c960, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::render_flag(g))
                });
                let obj = t.alloc(&rng.bytes(0x40));
                let sym = if rng.chance(2) {
                    0x006f_7bd0
                } else {
                    0x006f_7c00
                };
                t.check(&what, sym, Conv::Fastcall, &[obj], |g| {
                    Out::U32(ut::buffer_size_bytes(g, obj))
                });
            }
            _ => {
                t.global(
                    ut::PRX_STATE.sym,
                    if rng.chance(2) { 3 } else { rng.below(5) },
                );
                t.global(
                    ut::PRX_MODULE.sym,
                    if rng.chance(2) { 0 } else { rng.next() },
                );
                t.check(&what, 0x006f_9b70, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::prx_complete(g))
                });
                t.check(&what, 0x006f_9b80, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::prx_idle(g))
                });
            }
        }
    }
}

/// A scene list cursor for the cursor steps: `+0x14` the end node, `+0x18`
/// the current node, `+0x1c` the node the validity check looks for (the
/// list's nodes at `+0x2c`, linked at `+0x40`). Nodes are 0x60 bytes with
/// the next node at `+0`.
fn build_cursor(t: &mut Twins, rng: &mut Rng) -> u32 {
    let n = 1 + rng.below(5) as usize;
    let nodes: Vec<u32> = (0..n).map(|_| t.alloc(&rng.bytes(0x60))).collect();
    for k in 0..n {
        let next = if k + 1 < n { nodes[k + 1] } else { 0 };
        t.w32(nodes[k], next);
        t.w32(nodes[k] + 0x40, if k + 1 < n { nodes[k + 1] } else { 0 });
        let data = t.alloc(&rng.bytes(0x90));
        t.w32(nodes[k] + 0x2c, data);
    }
    let cur = t.alloc(&rng.bytes(0x40));
    let end = if rng.chance(3) {
        nodes[rng.below(n as u32) as usize]
    } else {
        rng.next()
    };
    t.w32(cur + 0x14, end);
    // validity: node +0x18 is the "list" (0 = valid), +0x1c the sought node
    let start = if rng.chance(5) {
        0
    } else {
        nodes[rng.below(n as u32) as usize]
    };
    t.w32(cur + 0x18, start);
    // a node that ends the search: the last one, as a list with +0x40 set
    let at = nodes[rng.below(n as u32) as usize];
    t.w32(cur + 0x1c, at);
    for &k in &nodes {
        t.w32(k + 0x2c, nodes[0]);
        if rng.chance(2) {
            t.w32(k + 0x40, 0);
        }
    }
    t.w32(nodes[n - 1] + 0x40, 0);
    cur
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function
fn round8_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x8008_0008);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 6 {
            0 => {
                let scene = build_scene(&mut t, &mut rng);
                let view: Vec<u8> = (0..16)
                    .flat_map(|_| (rng.below(2000) as f32 / 100.0 - 10.0).to_le_bytes())
                    .collect();
                let view = t.alloc(&view);
                if rng.chance(2) {
                    let on = u32::from(!rng.chance(4));
                    t.check(&what, 0x0063_9b30, Conv::Cdecl, &[view, on], |g| {
                        st::camera_set_world_matrix_sub(g, view, on);
                        Out::None
                    });
                } else {
                    let camera = if rng.chance(4) {
                        0
                    } else {
                        t.alloc(&rng.bytes(0x100))
                    };
                    t.w32(scene + 0x654, camera);
                    t.stub_sym(st::CAMERA_UPDATE_VIEW, Conv::Thiscall(0), 0, rng.next());
                    t.check(&what, 0x0064_2410, Conv::Cdecl, &[view], |g| {
                        st::minimap_camera_set_world_matrix(g, view);
                        Out::None
                    });
                }
            }
            1 => {
                build_scene(&mut t, &mut rng);
                let (sym, method, n) = [
                    (0x0063_95e0, st::SCENE_SET_CAMERA_FOVY, 1),
                    (0x0063_95c0, st::SCENE_SET_CAMERA_ASPECT_RATIO, 1),
                ][rng.below(2) as usize];
                t.stub_sym(method, Conv::Thiscall(0), n, rng.next());
                let v = t.alloc(&float_bytes(&mut rng, 1));
                t.check(&what, sym, Conv::Cdecl, &[v], |g| {
                    Out::U32(st::camera_set_from_ptr(g, method, v))
                });
                let (sym, method, n) = [
                    (0x0064_26c0, st::SCENE_SET_OBJECT_TRANSLATION, 2),
                    (0x0063_8f30, st::SCENE_HIDE_2D_LAYER, 1),
                    (0x0063_8f60, st::SCENE_SHOW_2D_LAYER, 1),
                ][rng.below(3) as usize];
                let ret = rng.next();
                t.stub_sym(method, Conv::Thiscall(0), n, ret);
                let args: Vec<u32> = (0..n).map(|_| rng.next()).collect();
                t.check(&what, sym, Conv::Cdecl, &args, |g| {
                    Out::U32(st::scene_call(g, method, &args))
                });
            }
            2 | 3 => {
                build_scene(&mut t, &mut rng);
                t.stub_sym(
                    st::SCENE_SET_MESH_INSTANCE_WORLD_MATRIX,
                    Conv::Stdcall,
                    2,
                    rng.next(),
                );
                let n = rng.below(4);
                let mut items = Vec::new();
                for _ in 0..n {
                    let segs = 1 + rng.below(4);
                    let table = t.alloc(&rng.bytes(0x6c * segs as usize + 0x20));
                    for k in 0..segs {
                        t.w32(table + 0x6c * k + 0x18, rng.below(4));
                    }
                    let holder = t.alloc(&rng.bytes(0x6c * 4 + 0x20));
                    t.w32(holder, segs | (rng.next() & 0x8000_0000));
                    t.w32(holder + 4, table);
                    let mesh = t.zeros(0x20);
                    t.w32(mesh, holder);
                    items.extend(mesh.to_le_bytes());
                }
                let items = t.alloc(&items);
                let list = if rng.chance(8) { 0 } else { t.zeros(0x20) };
                if list != 0 {
                    t.w32(list, n | (rng.next() & 0x8000_0000));
                    t.w32(list + 4, items);
                }
                let m = rng.next();
                if round % 6 == 2 {
                    t.check(&what, 0x0063_c3e0, Conv::Cdecl, &[list, m], |g| {
                        st::character_set_world_matrix(g, list, m);
                        Out::None
                    });
                } else {
                    let id = rng.below(6);
                    t.check(&what, 0x0063_c430, Conv::Cdecl, &[list, m, id], |g| {
                        st::character_set_world_matrix_by_mdl_id(g, list, m, id);
                        Out::None
                    });
                }
            }
            _ => {
                t.stub_sym(st::CURSOR_SET_NODE, Conv::Thiscall(0), 1, rng.next());
                let cur = build_cursor(&mut t, &mut rng);
                let (sym, seek) = [
                    (0x0065_eaf0, true),
                    (0x0065_ebd0, true),
                    (0x0065_ec40, true),
                    (0x0065_ecb0, true),
                    (0x0065_ed20, false),
                    (0x0065_ed80, false),
                    (0x0065_ede0, false),
                    (0x0065_ee40, false),
                ][rng.below(8) as usize];
                t.check(&what, sym, Conv::Fastcall, &[cur], |g| {
                    Out::U32(st::cursor_step(g, cur, seek))
                });
            }
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round9_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x9009_0009);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 5 {
            0 => {
                let out = t.alloc(&rng.bytes(64));
                let a = if rng.chance(3) {
                    rng.float()
                } else {
                    (rng.below(10_000) as f32 - 5000.0) / 1000.0
                };
                let (sym, y) = if rng.chance(2) {
                    (0x0066_c8a0, false)
                } else {
                    (0x0066_c920, true)
                };
                t.check(&what, sym, Conv::Cdecl, &[out, a.to_bits()], |g| {
                    Out::U32(ut::rotation(g, out, a.to_bits(), y))
                });
            }
            1 => {
                let cur = t.alloc(&rng.bytes(0x20));
                let obj = t.alloc(&rng.bytes(0x90));
                let node = t.alloc(&rng.bytes(0x40));
                t.w32(node + 0x2c, obj);
                let stride = rng.below(3) * 4;
                t.w32(obj + 0x1c, stride);
                let pos = t.zeros(0x40);
                t.w32(node + 0x34, pos);
                t.w32(node + 0x3c, rng.below(5));
                // a chain of links that may equal the stepping position
                let mut link = pos;
                if rng.chance(3) {
                    link = rng.next();
                } else if stride != 0 {
                    for k in 0..6 {
                        let next = if rng.chance(4) {
                            rng.next()
                        } else {
                            pos + stride * (k + 1)
                        };
                        t.w32(pos + stride * k, next);
                    }
                } else {
                    t.w32(pos, if rng.chance(2) { pos } else { rng.next() });
                }
                t.w32(node + 8, link);
                t.check(&what, 0x0044_1dc0, Conv::Thiscall(cur), &[node], |g| {
                    Out::U32(ut::cursor_set_node(g, cur, node))
                });
            }
            2 => {
                let list = t.alloc(&rng.bytes(0x50));
                let nodes: Vec<u32> = (0..=rng.below(4)).map(|_| t.zeros(0x50)).collect();
                for w in nodes.windows(2) {
                    t.w32(w[0] + 0x40, w[1]);
                }
                t.w32(list + 0x2c, if rng.chance(6) { 0 } else { nodes[0] });
                t.w32(list + 0x40, rng.below(2));
                let cur = t.zeros(0x20);
                t.w32(cur + 0x18, if rng.chance(6) { 0 } else { list });
                let at = if rng.chance(2) {
                    nodes[rng.below(nodes.len() as u32) as usize]
                } else {
                    rng.next()
                };
                t.w32(cur + 0x1c, at);
                let sym = [0x0044_1e10, 0x0044_1e40, 0x006a_29c0][rng.below(3) as usize];
                t.check(&what, sym, Conv::Thiscall(cur), &[], |g| {
                    Out::U8(managers::bridge::cursor_valid(g, cur))
                });
            }
            3 => {
                t.stub_sym(ut::RESET_FIELDS, Conv::Thiscall(0), 0, rng.next());
                t.stub_sym(ut::MEMBER_INIT_585230, Conv::Thiscall(0), 0, rng.next());
                let obj = t.alloc(&rng.bytes(0x60));
                t.check(&what, 0x006c_40c0, Conv::Thiscall(obj), &[], |g| {
                    Out::U32(ut::reset_and_construct(g, obj))
                });
            }
            _ => {
                let pp = t.alloc(&rng.bytes(0x1210));
                let list = if rng.chance(5) {
                    0
                } else {
                    t.alloc(&rng.bytes(0x40))
                };
                if list != 0 && rng.chance(2) {
                    t.w32(list + 0x1c, list + 0x1c);
                }
                // a stand-in for the cursor advance (fastcall): logs the
                // cursor's eight words, then sets its count and position
                let at = t.rebase(ut::CURSOR_NEXT_464F30);
                let log = t.log;
                let (cnt, p0, p4) = (rng.below(2), rng.next(), rng.next());
                let mut c: Vec<u8> = vec![0xa1];
                c.extend(log.to_le_bytes()); // mov eax, [log]
                c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
                c.push(0x05);
                c.extend((log + 4).to_le_bytes()); // add eax, log + 4
                for i in 0..8u8 {
                    c.extend([0x8b, 0x51, 4 * i]); // mov edx, [ecx+4i]
                    c.extend([0x89, 0x50, 4 * i]); // mov [eax+4i], edx
                }
                c.extend([0xff, 0x05]);
                c.extend(log.to_le_bytes()); // inc dword [log]
                c.extend([0xc7, 0x41, 0x0c]);
                c.extend(cnt.to_le_bytes()); // mov [ecx+0xc], cnt
                c.extend([0xc7, 0x41, 0x10]);
                c.extend(p4.to_le_bytes()); // mov [ecx+0x10], p4
                c.extend([0xc7, 0x01]);
                c.extend(p0.to_le_bytes()); // mov [ecx], p0
                c.push(0xc3);
                let stub = t.cave_next;
                t.write(stub, &c);
                t.cave_next += c.len() as u32;
                let mut j = vec![0xe9];
                j.extend(stub.wrapping_sub(at + 5).to_le_bytes());
                t.write(at, &j);
                t.check(&what, 0x0069_8400, Conv::Thiscall(pp), &[list], |g| {
                    Out::U32(ut::post_process_set_first_entry(g, pp, list))
                });
            }
        }
    }
}

#[test]
fn round9_more_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x9009_000a);
    let names = ["", "a", "water", "fire"];
    for round in 0..2000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        {
            let mgr = t.alloc(&rng.bytes(0x90));
            t.w32(mgr + 8, rng.below(2));
            t.w32(mgr + 0xc, rng.below(2));
            // materials naming animations
            // past 0xff materials sometimes, so eax's upper bytes show
            let n = if rng.chance(4) {
                0x100 + rng.below(5)
            } else {
                rng.below(5)
            };
            let mut mats = Vec::new();
            for _ in 0..n {
                let m = if rng.chance(5) {
                    0
                } else {
                    let m = t.zeros(0x20);
                    let s = cstr(&mut t, names[rng.below(2) as usize]);
                    t.w32(m + 0x10, s | rng.below(2));
                    m
                };
                mats.extend(m.to_le_bytes());
            }
            let mats = t.alloc(&mats);
            t.w32(mgr + 0x74, n | (rng.next() & 0x8000_0000));
            t.w32(mgr + 0x78, mats);
            // animations: 0x20 bytes, the name a PString at +4
            let k = rng.below(4);
            let anims = t.alloc(&rng.bytes(0x20 * k as usize + 4));
            for j in 0..k {
                let s = cstr(&mut t, names[rng.below(2) as usize]);
                t.w32(anims + 0x20 * j + 4, s);
                t.w32(
                    anims + 0x20 * j + 8,
                    (0x3fe + rng.below(4)) | (rng.next() & 0x8000_0000),
                );
            }
            t.w32(mgr + 0x7c, k);
            t.w32(mgr + 0x80, anims);
            let i = if n > 0x100 && rng.chance(2) {
                0x100 + rng.below(6)
            } else {
                rng.below(6)
            };
            let (frame, abs) = (0x3fe + rng.below(4), rng.below(2));
            t.check(
                &what,
                0x006b_42c0,
                Conv::Thiscall(mgr),
                &[i, frame, abs],
                |g| Out::U32(ut::tex_anim_set_frame(g, mgr, i, frame, abs)),
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one case per function
fn round10_matches_the_original() {
    use ffx_graphic::state::bridge as st;
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1010_0010);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 5 {
            0 => {
                t.stub_sym(st::CURSOR_SET_NODE, Conv::Thiscall(0), 1, rng.next());
                let cur = build_cursor(&mut t, &mut rng);
                let sym = if rng.chance(2) {
                    0x0044_1c60
                } else {
                    0x0044_1cc0
                };
                t.check(&what, sym, Conv::Fastcall, &[cur], |g| {
                    Out::U32(st::cursor_step(g, cur, false))
                });
            }
            1 => {
                let less = cave_float_less(&mut t);
                let n = rng.below(14);
                let base = t.alloc(&float_bytes(&mut rng, n as usize + 1));
                t.check(
                    &what,
                    0x006c_7410,
                    Conv::Cdecl,
                    &[base, base + 4 * n, less],
                    |g| Out::U32(ut::float_make_heap(g, base, base + 4 * n, less)),
                );
            }
            2 => {
                let less = cave_float_less(&mut t);
                let n = rng.below(10);
                // no NaNs: the unguarded inner loop relies on an order
                let v: Vec<u8> = (0..=n)
                    .flat_map(|_| (rng.below(200) as f32 - 100.0).to_le_bytes())
                    .collect();
                let base = t.alloc(&v);
                t.check(
                    &what,
                    0x006c_7320,
                    Conv::Cdecl,
                    &[base, base + 4 * n, less],
                    |g| {
                        ut::float_insertion_sort(g, base, base + 4 * n, less);
                        Out::None
                    },
                );
            }
            3 => {
                t.stub_sym(ut::ARRAY_GROW_3890, Conv::Thiscall(0), 1, rng.next());
                t.stub_sym(ut::ARRAY_GROW_3898, Conv::Thiscall(0), 1, rng.next());
                let scene = t.alloc(&rng.bytes(0x38b0));
                let cap = 1 + rng.below(6);
                let a1 = t.alloc(&rng.bytes(4 * cap as usize + 8));
                let a2 = t.alloc(&rng.bytes(4 * cap as usize + 8));
                t.w32(scene + 0x3890, cap | (rng.next() & 0x8000_0000));
                t.w32(scene + 0x3894, a1);
                t.w32(scene + 0x3898, rng.next());
                t.w32(scene + 0x389c, a2);
                t.w32(scene + 0x38a0, rng.below(cap));
                let (a, b) = (rng.next(), rng.next());
                t.check(&what, 0x0065_6b90, Conv::Thiscall(scene), &[a, b], |g| {
                    Out::U32(ut::scene_push_pair(g, scene, a, b))
                });
            }
            _ => {
                // printf through its import slot: a recording stub there
                let pf = t.stub(None, 0x5052, false, 1, 0, 0);
                let slot = t.rebase(ut::PRINTF_SLOT);
                t.w32(slot, pf);
                let set = rng.below(4);
                t.global(ut::GRAD_MESH_SET.sym, set);
                let pool = t.alloc(&rng.bytes(0xe10));
                for i in 0..16 {
                    t.w8(pool + set * 16 + 0xdc4 + i, u8::from(!rng.chance(8)));
                }
                let out = t.zeros(4);
                t.check(&what, 0x006c_c980, Conv::Thiscall(pool), &[out], |g| {
                    Out::U32(ut::get_spare_grad_mesh(g, pool, out))
                });
            }
        }
    }
}

#[test]
fn round10_settings_match_the_original() {
    use ffx_graphic::state::bridge as st;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1010_0011);
    for round in 0..1000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        let sm = t.alloc(&rng.bytes(0x4070));
        t.global(managers::bridge::SHADOW.sym, sm);
        t.stub_sym(st::SHADOW_RESIZE, Conv::Thiscall(0), 0, rng.next());
        t.stub_sym(st::POST_PROCESS_REBIND, Conv::Thiscall(0), 1, rng.next());
        let cfg = t.alloc(&rng.bytes(0x50));
        t.w32(cfg + 0x30, rng.below(5));
        t.w32(
            cfg + 0x40,
            if rng.chance(5) {
                rng.next()
            } else {
                rng.below(7)
            },
        );
        t.w32(cfg + 0x44, if rng.chance(2) { 0 } else { rng.next() });
        t.global(st::SETTINGS.sym, cfg);
        let pp = t.alloc(&rng.bytes(0x12400));
        let scene = t.alloc(&rng.bytes(0x10de0));
        t.w32(scene + 0x10cc, pp);
        t.check(&what, 0x0066_f010, Conv::Thiscall(scene), &[], |g| {
            Out::U32(st::scene_apply_settings(g, scene))
        });
    }
}

#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one case per function
fn round11_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1111_0011);
    let names = ["a", "water"];
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 8 {
            0 => {
                t.stub_sym(ut::MENU_CURRENT_PLAYER_POS, Conv::Cdecl, 0, rng.next());
                t.stub_sym(ut::MENU_PLAYER_FROM_INDEX, Conv::Cdecl, 1, rng.next());
                t.check(&what, 0x008e_1c50, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::get_current_frame(g))
                });
            }
            1 => {
                t.stub_sym(ut::TEX_ANIM_SET_FRAME, Conv::Thiscall(0), 3, rng.next());
                let mgr = t.zeros(0x20);
                t.global(managers::bridge::TEXTURE_ANIMATION.sym, mgr);
                let (i, f) = (rng.next(), rng.next());
                t.check(&what, 0x0064_3a20, Conv::Cdecl, &[i, f], |g| {
                    Out::U32(ut::graphic_tex_anim_set_frame(g, i, f))
                });
            }
            2 => {
                use ffx_graphic::texanim::bridge as ta;
                t.stub_sym(ut::TEX_ANIM_RESET_POS, Conv::Cdecl, 3, rng.next());
                t.stub_sym(ut::TEX_ANIM_POS_SET_OLD, Conv::Cdecl, 3, rng.next());
                let i = rng.below(4);
                let e = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i;
                t.w8(e + 0xd, [0, 1, 2, 0xff][rng.below(4) as usize]);
                let (a, b) = (rng.next(), rng.next());
                t.check(&what, 0x0077_e9a0, Conv::Cdecl, &[i, a, b], |g| {
                    Out::U32(ut::tex_anim_pos_set(g, i, a, b))
                });
            }
            3 => {
                let pf = t.stub(None, 0x5350, false, 3, 0, rng.next());
                let slot = t.rebase(ut::SPRINTF_SLOT);
                t.w32(slot, pf);
                let scene = t.alloc(&rng.bytes(0xfd00));
                t.w8(scene + 0xfc00, u8::from(!rng.chance(4)));
                let n = rng.next();
                t.check(&what, 0x0066_e5e0, Conv::Thiscall(scene), &[n], |g| {
                    ut::scene_set_number_text(g, scene, n);
                    Out::None
                });
            }
            4 => {
                let mgr = t.alloc(&rng.bytes(0x90));
                t.w32(mgr + 8, rng.below(2));
                t.w32(mgr + 0xc, rng.below(2));
                let n = rng.below(5);
                let mut mats = Vec::new();
                for _ in 0..n {
                    let m = if rng.chance(5) {
                        0
                    } else {
                        let m = t.zeros(0x20);
                        let s = cstr(&mut t, names[rng.below(2) as usize]);
                        t.w32(m + 0x10, s | rng.below(2));
                        m
                    };
                    mats.extend(m.to_le_bytes());
                }
                let mats = t.alloc(&mats);
                t.w32(mgr + 0x74, n | (rng.next() & 0x8000_0000));
                t.w32(mgr + 0x78, mats);
                let k = rng.below(4);
                let anims = t.alloc(&rng.bytes(0x20 * k as usize + 4));
                for j in 0..k {
                    let s = cstr(&mut t, names[rng.below(2) as usize]);
                    t.w32(anims + 0x20 * j + 4, s);
                }
                t.w32(mgr + 0x7c, k);
                t.w32(mgr + 0x80, anims);
                let (i, p) = (rng.below(6), rng.next());
                t.check(&what, 0x006b_3600, Conv::Thiscall(mgr), &[i, p], |g| {
                    ut::tex_anim_set_paused(g, mgr, i, p);
                    Out::None
                });
            }
            5 => {
                t.stub_sym(ut::BROKEN_SCREEN_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
                let n = if rng.chance(2) {
                    rng.below(100)
                } else {
                    rng.next()
                };
                t.check(&what, 0x0063_93c0, Conv::Cdecl, &[n], |g| {
                    Out::U32(ut::broken_screen_buffer_size(g, n))
                });
            }
            6 => {
                t.stub_sym(ut::SCENE_CHECK_6721B0, Conv::Thiscall(0), 0, rng.below(2));
                let scene = t.alloc(&rng.bytes(0x10d30));
                let near = |rng: &mut Rng| {
                    if rng.chance(3) {
                        0xc00 - rng.below(2)
                    } else {
                        rng.below(0xc00)
                    }
                };
                let (n1, n2) = (near(&mut rng), near(&mut rng));
                t.w32(scene + 0xf8cc, n1);
                t.w32(scene + 0xf8d0, n2);
                let e = t.alloc(&rng.bytes(0x10));
                let kind = if rng.chance(2) { 0 } else { rng.next() };
                t.check(&what, 0x0065_6c10, Conv::Thiscall(scene), &[e, kind], |g| {
                    ut::scene_add_draw_entry(g, scene, e, kind);
                    Out::None
                });
            }
            _ => {
                build_scene(&mut t, &mut rng);
                t.stub_sym(
                    ut::SCENE_UPDATE_SHADOW_CLUSTERS,
                    Conv::Thiscall(0),
                    0,
                    rng.next(),
                );
                t.stub_sym(
                    ut::SCENE_FORCE_SET_SHADOW_SOLUTION,
                    Conv::Thiscall(0),
                    1,
                    rng.next(),
                );
                let at = t.rebase(0x00c3_4400);
                let f = rng.float();
                t.write(at, &f.to_le_bytes());
                let sm = t.alloc(&rng.bytes(0x4070));
                let keep = if rng.chance(2) { 0 } else { rng.next() };
                t.check(&what, 0x006c_eb10, Conv::Thiscall(sm), &[keep], |g| {
                    Out::U32(ut::shadow_reset_solution(g, sm, keep))
                });
            }
        }
    }
}

// ---- round 12 ----

/// Writes `mov eax, [esp+4]; and eax, mask; ret` over the original at
/// `sym`: a button check pressing the buttons of `mask`.
fn stub_buttons(t: &mut Twins, sym: u32, mask: u32) {
    let at = t.rebase(sym);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x25];
    c.extend(mask.to_le_bytes());
    c.push(0xc3);
    t.write(at, &c);
}

/// One round-12 case on `t` (`real`: a snapshot, whose globals the case
/// reads rather than sets where it can).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one arm per function
fn round12_case(t: &mut Twins, rng: &mut Rng, case: u32, real: bool, what: &str) {
    use ffx_graphic::texanim::bridge as ta;
    use ffx_graphic::util::bridge as ut;
    match case {
        0 => {
            if !real {
                let s = [0, 1, 2, 3, rng.next()][rng.below(5) as usize];
                t.global(ut::PRX_STATE.sym, s);
            }
            t.check(what, 0x006f_9ba0, Conv::Cdecl, &[], |g| {
                Out::U32(ut::prx_in_progress(g))
            });
        }
        1 => {
            let this = t.alloc(&rng.bytes(0x20));
            t.w8(this + 0x1c, [0, 1, 0xff][rng.below(3) as usize]);
            let count = if rng.chance(8) {
                rng.next() | 1
            } else {
                1 + rng.below(8)
            } | (rng.next() & 0x8000_0000);
            t.w32(this + 8, count);
            t.w32(
                this + 0x18,
                if rng.chance(4) {
                    rng.next()
                } else {
                    rng.below(9)
                },
            );
            let period = if rng.chance(2) {
                (rng.below(100) as f32) / 10.0
            } else {
                rng.float()
            };
            let timer = if rng.chance(2) {
                (rng.below(100) as f32) / 10.0
            } else {
                rng.float()
            };
            let dt = if rng.chance(2) {
                (rng.below(30) as f32) / 10.0
            } else {
                rng.float()
            };
            // the timer reaching the period exactly
            let (timer, period) = if rng.chance(4) {
                let a = rng.below(8) as f32;
                let dt_int = dt.trunc();
                (a, a + dt_int)
            } else {
                (timer, period)
            };
            let dt = if timer.fract() == 0.0 && period.fract() == 0.0 && rng.chance(2) {
                period - timer
            } else {
                dt
            };
            t.write(this + 0x10, &period.to_le_bytes());
            t.write(this + 0x14, &timer.to_le_bytes());
            t.check(
                what,
                0x006b_4450,
                Conv::Thiscall(this),
                &[dt.to_bits()],
                |g| {
                    ut::tex_anim_info_update(g, this, dt);
                    Out::None
                },
            );
        }
        2 => {
            let n = rng.below(6);
            let mut idx = Vec::new();
            for _ in 0..n {
                let v = match rng.below(4) {
                    0 => u32::MAX,
                    1 => rng.next(),
                    _ => rng.below(10),
                };
                idx.extend(v.to_le_bytes());
            }
            idx.extend([0; 4]);
            let words = t.alloc(&idx);
            let list = t.alloc(&rng.bytes(0x10));
            t.w32(list + 8, n | (rng.next() & 0x8000_0000));
            t.w32(list + 0xc, words);
            let list = if rng.chance(8) { 0 } else { list };
            let mgr = t.alloc(&rng.bytes(0x70));
            let m = rng.below(12);
            let items = t.alloc(&rng.bytes(4 * m as usize + 4));
            t.w32(mgr + 0x10, rng.below(4));
            t.w32(mgr + 0x5c, m | (rng.next() & 0x8000_0000));
            t.w32(mgr + 0x60, items);
            let i = if rng.chance(8) {
                rng.next()
            } else {
                rng.below(7)
            };
            let abs = if rng.chance(2) { 0 } else { rng.next() };
            t.check(
                what,
                0x006b_2140,
                Conv::Thiscall(mgr),
                &[list, i, abs],
                |g| Out::U32(ut::tex_anim_by_index_list(g, mgr, list, i, abs)),
            );
        }
        3 => {
            let i = rng.below(64);
            let e = t.rebase(ta::CHR_TEXANIM.sym) + 0x30 * i;
            // a snapshot's entries keep their loaded word; their buffers may
            // be stale, so each gets a fresh one
            if !real {
                t.w32(e, u32::from(!rng.chance(4)));
            }
            let work = t.alloc(&rng.bytes(0x600));
            t.w32(e + 4, work);
            let a = if rng.chance(4) { rng.below(8) } else { 4 };
            let v = rng.next();
            t.check(what, 0x0078_07c0, Conv::Cdecl, &[i, a, v], |g| {
                let r = ut::tex_anim_no_set_old(g, i, a, v);
                // for a != 4 the original's eax is its caller's
                if a == 4 { Out::U32(r) } else { Out::None }
            });
        }
        4 => {
            t.stub_sym(ut::DBG_PRINTF, Conv::Cdecl, 2, rng.next());
            let i = rng.below(64);
            let d = rng.next();
            t.check(what, 0x0077_ea10, Conv::Cdecl, &[i, d], |g| {
                Out::U32(ut::texanim_malloc_memory_set(g, i, d))
            });
        }
        5 => {
            // a mute call that writes g_SpdCtrl_c0 (`mov dword [c0], v;
            // mov eax, r; ret`): the clear after it must show
            let at = t.rebase(ut::SG_SET_MUTE_SOUND);
            let c0 = t.rebase(ut::SPD_CTRL_C0.sym);
            let mut c = vec![0xc7, 0x05];
            c.extend(c0.to_le_bytes());
            c.extend((rng.next() | 1).to_le_bytes());
            c.push(0xb8);
            c.extend(rng.next().to_le_bytes());
            c.push(0xc3);
            t.write(at, &c);
            if !real {
                for g in [
                    ut::SPD_CTRL_B4.sym,
                    ut::SPD_CTRL_B8.sym,
                    ut::SPD_CTRL_ENABLED.sym,
                    ut::SPD_CTRL_C0.sym,
                ] {
                    t.global(g, rng.next());
                }
            }
            t.check(what, 0x006f_72f0, Conv::Cdecl, &[], |g| {
                Out::U32(ut::spd_ctrl_init(g))
            });
        }
        6 => {
            let mask =
                [0, ut::AUTOBTL_BUTTON | ut::INVINCIBLE_BUTTON, rng.next()][rng.below(3) as usize];
            stub_buttons(t, ut::INPUT_IS_PRESS_BUTTON, mask);
            if !real {
                t.global(ut::AUTOBTL_KEY.sym, rng.next());
                t.global(ut::INVINCIBLE_KEY.sym, rng.next());
            }
            if rng.chance(2) {
                t.check(what, 0x006f_7580, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::check_key(g, ut::AUTOBTL_KEY, ut::AUTOBTL_BUTTON))
                });
            } else {
                t.check(what, 0x006f_76c0, Conv::Cdecl, &[], |g| {
                    Out::U32(ut::check_key(g, ut::INVINCIBLE_KEY, ut::INVINCIBLE_BUTTON))
                });
            }
        }
        7 => {
            let mask = [0, 0x8000, 0x2000, 0xa000, rng.next()][rng.below(5) as usize];
            stub_buttons(t, ut::INPUT_IS_PRESS_BUTTON, mask);
            let out = t.alloc(&rng.bytes(8));
            t.check(what, 0x0064_5eb0, Conv::Cdecl, &[out, out + 4], |g| {
                ut::set_lulu_od_keyboard_input(g, out, out + 4);
                Out::None
            });
        }
        8 => {
            if !real || t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 {
                let scene = t.zeros(0x10);
                t.global(scene::bridge::SCENE.sym, scene);
            }
            let stat = if rng.chance(2) { 0 } else { rng.next() };
            t.stub_sym(ut::SCENE_GET_SPLIT_SCREEN_STAT, Conv::Thiscall(0), 0, stat);
            t.stub_sym(
                ut::SCENE_SET_SPLIT_SCREEN_STAT,
                Conv::Thiscall(0),
                1,
                rng.next(),
            );
            t.stub_sym(
                ut::SCENE_SET_SPLIT_SCREEN_TYPE,
                Conv::Thiscall(0),
                1,
                rng.next(),
            );
            let kind = [0, 1, 2, rng.next()][rng.below(4) as usize];
            t.check(what, 0x0063_8d40, Conv::Cdecl, &[kind], |g| {
                Out::U32(ut::split_screen_start(g, kind))
            });
        }
        9 => {
            if !real || t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 {
                let scene = t.zeros(0x10);
                t.global(scene::bridge::SCENE.sym, scene);
            }
            t.stub_sym(ut::SCENE_SET_BYTE_03, Conv::Thiscall(0), 1, rng.next());
            let v = rng.next();
            t.check(what, 0x0064_1e40, Conv::Cdecl, &[v], |g| {
                Out::U32(ut::magic_0146_set_key(g, v))
            });
        }
        10 => {
            // the loading screen's own methods run, original on both sides
            let ls = t.rebase(ut::LOADING_SCREEN.sym);
            if !real || t.r32(ls) == 0 {
                let o = t.alloc(&float_bytes(rng, 0x20));
                t.w32(ls, o);
            }
            let o = t.r32(ls);
            if !real || rng.chance(2) {
                t.w32(o + 0x10, [0, 3, 1, rng.next()][rng.below(4) as usize]);
            }
            t.check(what, 0x0064_2c80, Conv::Cdecl, &[], |g| {
                ut::reset_loading_screen(g);
                Out::None
            });
        }
        _ => {
            use ffx_graphic::managers::bridge::POST_PROCESS;
            t.stub_sym(
                ut::POST_PROCESS_RENDER_SCREEN_FADE,
                Conv::Thiscall(0),
                4,
                rng.next(),
            );
            let pp = t.rebase(POST_PROCESS.sym);
            if !real || t.r32(pp) == 0 {
                let o = t.alloc(&rng.bytes(0x12400));
                t.w32(pp, o);
            }
            let o = t.r32(pp);
            if !real || rng.chance(2) {
                t.w8(o + 0x123a0, u8::from(rng.chance(3)));
            }
            if !real {
                t.global(ut::SCREEN_FADE_BYTE.sym, rng.next());
            }
            let d = t.alloc(&float_bytes(rng, 0x20));
            for k in 0..3 {
                t.w32(d + 4 * k, rng.next());
            }
            t.w32(d + 0x1c, rng.next());
            let d = if rng.chance(8) { 0 } else { d };
            t.check(what, 0x006a_e5b0, Conv::Cdecl, &[d], |g| {
                ut::screen_fade_process(g, d);
                Out::None
            });
        }
    }
}

#[test]
fn round12_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1212_0012);
    for round in 0..3600 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round12_case(&mut t, &mut rng, round % 12, false, &what);
    }
}

#[test]
fn snapshot_round12_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5012_0012);
    for round in 0..600 {
        t.reset();
        let what = format!("snapshot round {round}");
        round12_case(&mut t, &mut rng, round % 12, true, &what);
    }
}

// ---- round 13 ----

/// A shadow manager for `ShadowManager_AverageCasterValue`: 64 slots of
/// meshes (each `+4 ->[0] +4 -> +0x24` a float), flags, kinds, callback
/// words; the callback is the game's `Compute_006cc8b0` (`a0 == 1`).
#[allow(clippy::many_single_char_names)] // the chain's links
fn build_shadow_slots(t: &mut Twins, rng: &mut Rng) -> u32 {
    let sm = t.alloc(&rng.bytes(0x4060));
    let pick = |rng: &mut Rng| match rng.below(5) {
        0 => rng.next(),
        1 => 0,
        _ => 1,
    };
    let dense = rng.below(4);
    for i in 0..64 {
        let mesh = if rng.below(4) <= dense {
            let c = t.alloc(&float_bytes(rng, 10));
            if rng.chance(4) {
                // quiet NaNs with payloads: the x87 keeps the larger one
                let nan = 0x7fc0_0000 | (rng.next() & 0x803f_ffff);
                t.w32(c + 0x24, nan);
            }
            let b = t.zeros(8);
            t.w32(b + 4, c);
            let a = t.alloc(&b.to_le_bytes());
            let m = t.zeros(8);
            let n = if rng.chance(6) { 0 } else { 1 + rng.below(3) };
            t.w32(m, n | (rng.next() & 0x8000_0000));
            t.w32(m + 4, a);
            m
        } else {
            0
        };
        t.w32(sm + 0xde8 + 4 * i, mesh);
        let fa = pick(rng);
        let fb = pick(rng);
        t.w32(sm + 0xfe8 + 4 * i, fa);
        t.w32(sm + 0x10e8 + 4 * i, fb);
        t.w8(
            sm + 0x2ba8 + 0xc * i,
            [b'c', b'n', b'n', b'x', rng.next() as u8][rng.below(5) as usize],
        );
        t.w32(sm + 0x2ea8 + 4 * i, pick(rng));
    }
    let row = if rng.chance(6) {
        rng.next() | 0x8000_0000
    } else {
        rng.below(8)
    };
    let col = if rng.chance(6) {
        rng.next() | 0x8000_0000
    } else {
        rng.below(8)
    };
    t.w32(sm + 0x4010, row);
    t.w32(sm + 0x4014, col);
    t.w8(sm + 0x4030, u8::from(rng.chance(2)));
    let cb = t.rebase(0x006c_c8b0);
    t.w32(sm + 0x4048, cb);
    sm
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one arm per function
fn round13_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) {
    use ffx_graphic::util::bridge as ut;
    match case {
        0 => {
            let sm = build_shadow_slots(t, rng);
            t.check(what, 0x006c_cad0, Conv::Fastcall, &[sm, 0], |g| {
                Out::F32(ut::shadow_average_caster_value(g, sm))
            });
        }
        1 => {
            if t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 {
                let scene = t.zeros(0x10);
                t.global(scene::bridge::SCENE.sym, scene);
            }
            t.stub_sym(ut::SCENE_PUSH_MESH_MATRIX, Conv::Thiscall(0), 2, rng.next());
            let n = rng.below(4);
            let mut arr = Vec::new();
            let mut ids = Vec::new();
            for _ in 0..n {
                let k = rng.below(4);
                let segs = t.alloc(&rng.bytes(0x6c * k as usize + 4));
                for j in 0..k {
                    let id = rng.below(5);
                    ids.push(id);
                    t.w32(segs + 0x6c * j + 0x18, id);
                }
                let mesh = t.zeros(8);
                t.w32(mesh, k | (rng.next() & 0x8000_0000));
                t.w32(mesh + 4, segs);
                let inst = t.alloc(&mesh.to_le_bytes());
                arr.extend(inst.to_le_bytes());
            }
            arr.extend([0; 4]);
            let arr = t.alloc(&arr);
            let list = t.zeros(8);
            t.w32(list, n | (rng.next() & 0x8000_0000));
            t.w32(list + 4, arr);
            let list = if rng.chance(10) { 0 } else { list };
            let id = match rng.below(3) {
                0 => u32::MAX,
                1 if !ids.is_empty() => ids[rng.below(ids.len() as u32) as usize],
                _ => rng.below(6),
            };
            let matrix = rng.next();
            t.check(what, 0x0063_9170, Conv::Cdecl, &[list, matrix, id], |g| {
                ut::add_mesh_matrix(g, list, matrix, id);
                Out::None
            });
        }
        2 => {
            use ffx_graphic::managers::bridge::{FPRINTF_SLOT, IOB_FUNC_SLOT};
            let mgr = rng.next();
            t.stub_sym(ut::TEX_ANIM_GET_INSTANCE, Conv::Cdecl, 0, mgr);
            let i = [u32::MAX, 0x8000_0000, 0, rng.below(9), rng.next()][rng.below(5) as usize];
            t.stub_sym(ut::TEX_ANIM_FIND_MATERIAL_INDEX, Conv::Thiscall(0), 1, i);
            let set = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
            t.stub_sym(ut::TEX_ANIM_SET_FRAME, Conv::Thiscall(0), 3, set);
            t.stub_sym(
                ut::SCENE_GET_CURRENT_LEVEL_NAME,
                Conv::Thiscall(0),
                0,
                rng.next(),
            );
            let iob = t.stub(None, 0x10b, false, 0, 0, rng.next());
            let fprintf = t.stub(None, 0xf9, false, 4, 0, rng.next());
            t.global(IOB_FUNC_SLOT, iob);
            t.global(FPRINTF_SLOT, fprintf);
            if t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 {
                let scene = t.zeros(0x10);
                t.global(scene::bridge::SCENE.sym, scene);
            }
            let (frame, name) = (rng.next(), rng.next());
            t.check(what, 0x0064_3a40, Conv::Cdecl, &[frame, name], |g| {
                Out::U32(ut::tex_anim_set_frame_by_name(g, frame, name))
            });
        }
        _ => {
            let scene = t.alloc(&rng.bytes(0xfd00));
            t.global(scene::bridge::SCENE.sym, scene);
            let c: [f32; 6] = std::array::from_fn(|_| rng.float());
            let second = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
            let mut args: Vec<u32> = c.iter().map(|f| f.to_bits()).collect();
            args.push(second);
            t.check(what, 0x0064_3b30, Conv::Cdecl, &args, |g| {
                Out::U32(ut::graphic_set_colour_pair(g, c, second))
            });
        }
    }
}

#[test]
fn round13_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1313_0013);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round13_case(&mut t, &mut rng, round % 4, &what);
    }
}

#[test]
fn snapshot_round13_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5013_0013);
    // the game's own shadow manager first
    let sm = t.r32(t.rebase(managers::bridge::SHADOW.sym));
    if sm != 0 {
        t.check(
            "snapshot shadow",
            0x006c_cad0,
            Conv::Fastcall,
            &[sm, 0],
            |g| {
                Out::F32(ffx_graphic::util::bridge::shadow_average_caster_value(
                    g, sm,
                ))
            },
        );
    }
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        round13_case(&mut t, &mut rng, round % 4, &what);
    }
}

// ---- round 14 ----

/// A mesh instance list for the shader-parameter setters: instances with
/// meshes of segments (material index, hidden byte, id), materials (one
/// inline or several by pointer), each with a shader word and a parameter
/// buffer of `buf_len` bytes. Returns the list and the segment ids used.
fn build_material_list(t: &mut Twins, rng: &mut Rng, buf_len: u32) -> (u32, Vec<u32>) {
    let n = rng.below(4);
    let mut arr = Vec::new();
    let mut ids = Vec::new();
    for _ in 0..n {
        let nm = 1 + rng.below(3);
        let mut mats = Vec::new();
        for _ in 0..nm {
            let buf = t.alloc(&rng.bytes(buf_len as usize + 4));
            let size = if rng.chance(6) { rng.next() } else { buf_len };
            t.w32(buf, size);
            let mat = t.zeros(8);
            t.w32(mat, rng.next());
            t.w32(mat + 4, buf);
            mats.push(mat);
        }
        let container = t.zeros(8);
        t.w32(container, nm | (rng.next() & 0x8000_0000));
        if nm == 1 {
            t.w32(container + 4, mats[0]);
        } else {
            let p: Vec<u8> = mats.iter().flat_map(|m| m.to_le_bytes()).collect();
            let p = t.alloc(&p);
            t.w32(container + 4, p);
        }
        let k = rng.below(4);
        let segs = t.alloc(&rng.bytes(0x6c * k as usize + 4));
        for j in 0..k {
            t.w32(segs + 0x6c * j, rng.below(nm));
            let id = rng.below(4);
            ids.push(id);
            t.w32(segs + 0x6c * j + 0x18, id);
        }
        let mesh = t.zeros(8);
        t.w32(mesh, k | (rng.next() & 0x8000_0000));
        t.w32(mesh + 4, segs);
        let inst = t.alloc(&rng.bytes(0x14));
        t.w32(inst, mesh);
        t.w32(inst + 0x10, container);
        arr.extend(inst.to_le_bytes());
    }
    arr.extend([0; 4]);
    let arr = t.alloc(&arr);
    let list = t.zeros(8);
    t.w32(list, n | (rng.next() & 0x8000_0000));
    t.w32(list + 4, arr);
    (list, ids)
}

/// A parameter descriptor of `size` bytes at an offset into a buffer of
/// `buf_len` (sometimes past it, or below 4, when `fits` is false).
fn build_param(t: &mut Twins, rng: &mut Rng, buf_len: u32, size: u32, fits: bool) -> u32 {
    let p = t.alloc(&rng.bytes(0x10));
    let off = if fits || rng.chance(2) {
        4 + 4 * rng.below((buf_len - size) / 4)
    } else if rng.chance(2) {
        rng.below(4)
    } else {
        buf_len + rng.below(8)
    };
    t.write(p + 8, &(off as u16).to_le_bytes());
    let hi = (rng.next() as u16) & 0xe000;
    t.write(p + 0xa, &(size as u16 | hi).to_le_bytes());
    p
}

#[allow(clippy::too_many_lines)] // one arm per function
fn round14_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) {
    use ffx_graphic::material::bridge as mt;
    let colour = |rng: &mut Rng| -> [f32; 4] {
        let mut c: [f32; 4] = std::array::from_fn(|_| rng.float());
        match rng.below(5) {
            0 => c[3] = 0.0,
            1 => c[3] = -0.0,
            2 => c[3] = 1.0,
            3 => c[3] = (rng.below(20) as f32) / 10.0 - 0.5,
            _ => {}
        }
        c
    };
    match case {
        0 | 1 => {
            let (list, ids) = build_material_list(t, rng, 0x30);
            let size = [16, 12, 4, 0][rng.below(4) as usize];
            let param = build_param(t, rng, 0x30, size, false);
            t.stub_sym(
                mt::FIND_PARAMETER,
                Conv::Thiscall(0),
                1,
                if rng.chance(6) { 0 } else { param },
            );
            t.stub_sym(mt::FORCE_TRANSPARENT, Conv::Cdecl, 2, rng.next());
            let list = if rng.chance(10) { 0 } else { list };
            let c = colour(rng);
            let cb = c.map(f32::to_bits);
            if case == 0 {
                let x = rng.next();
                t.check(
                    what,
                    0x0063_bb30,
                    Conv::Cdecl,
                    &[list, x, cb[0], cb[1], cb[2], cb[3]],
                    |g| {
                        mt::set_mesh_colour(g, list, c);
                        Out::None
                    },
                );
            } else {
                let id = if !ids.is_empty() && rng.chance(2) {
                    ids[rng.below(ids.len() as u32) as usize]
                } else {
                    rng.below(5)
                };
                let hide = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
                t.check(
                    what,
                    0x0063_bc80,
                    Conv::Cdecl,
                    &[list, id, cb[0], cb[1], cb[2], cb[3], hide],
                    |g| {
                        mt::set_mesh_segment_colour(g, list, id, c, hide);
                        Out::None
                    },
                );
            }
        }
        2 => {
            let (list, _) = build_material_list(t, rng, 0x30);
            // every buffer large enough: the original reads the threshold
            // at address 0 for a parameter outside it
            let size = 4;
            let param = build_param(t, rng, 0x30, size, true);
            let off = u32::from(u16::from_le_bytes([t.r8(param + 8), t.r8(param + 9)]));
            let v = match rng.below(4) {
                0 => 0.001f32,
                1 => 0.0009,
                2 => (rng.below(10) as f32) / 1000.0,
                _ => rng.float(),
            };
            // the threshold at the parameter's offset of every buffer
            let n = t.r32(list) & 0x7fff_ffff;
            let arr = t.r32(list + 4);
            for i in 0..n {
                let inst = t.r32(arr + 4 * i);
                let mats = t.r32(inst + 0x10);
                let nm = t.r32(mats) & 0x7fff_ffff;
                let base = if nm == 1 { mats + 4 } else { t.r32(mats + 4) };
                for j in 0..nm {
                    let mat = t.r32(base + 4 * j);
                    let buf = t.r32(mat + 4);
                    t.w32(buf, 0x30);
                    t.write(buf + off, &v.to_le_bytes());
                }
            }
            t.stub_sym(
                mt::FIND_PARAMETER,
                Conv::Thiscall(0),
                1,
                if rng.chance(6) { 0 } else { param },
            );
            let list = if rng.chance(10) { 0 } else { list };
            t.check(what, 0x0063_b9e0, Conv::Cdecl, &[list], |g| {
                mt::set_mesh_alpha_threshold(g, list);
                Out::None
            });
        }
        _ => {
            // the original ShaderParamBuffer_SetValue runs on both sides
            let scene = t.alloc(&rng.bytes(0x1130));
            t.w8(scene + 0x93c, u8::from(!rng.chance(5)));
            t.w32(scene + 0x1128, if rng.chance(5) { 0 } else { rng.next() });
            let buf = t.alloc(&rng.bytes(0x34));
            t.w32(buf, if rng.chance(6) { rng.below(0x40) } else { 0x30 });
            let mat = t.zeros(8);
            t.w32(mat, rng.next());
            t.w32(mat + 4, buf);
            t.stub_sym(mt::TEX_ANIM_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
            t.stub_sym(
                mt::TEX_ANIM_MATERIAL_BY_LAYER,
                Conv::Thiscall(0),
                1,
                if rng.chance(6) { 0 } else { mat },
            );
            let size = [16, 12, 4][rng.below(3) as usize];
            let param = build_param(t, rng, 0x30, size, false);
            t.stub_sym(
                mt::FIND_PARAMETER,
                Conv::Thiscall(0),
                1,
                if rng.chance(6) { 0 } else { param },
            );
            let layer = rng.next();
            let alpha = match rng.below(3) {
                0 => rng.below(0x100),
                1 => rng.next(),
                _ => 0x80,
            };
            t.check(
                what,
                0x0065_82e0,
                Conv::Thiscall(scene),
                &[layer, alpha],
                |g| {
                    mt::scene_set_layer_material_alpha(g, scene, layer, alpha);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn round14_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1414_0014);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round14_case(&mut t, &mut rng, round % 4, &what);
    }
}

#[test]
fn snapshot_round14_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5014_0014);
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        round14_case(&mut t, &mut rng, round % 4, &what);
    }
}

// ---- round 15 ----

#[allow(clippy::too_many_lines)] // one arm per function
fn round15_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) {
    use ffx_graphic::material::bridge as mt;
    use ffx_graphic::util::bridge as ut;
    match case {
        0 => {
            let (list, ids) = build_material_list(t, rng, 0x30);
            let size = [16, 12, 4][rng.below(3) as usize];
            let param = build_param(t, rng, 0x30, size, false);
            t.stub_sym(
                mt::FIND_PARAMETER,
                Conv::Thiscall(0),
                1,
                if rng.chance(6) { 0 } else { param },
            );
            t.stub_sym(mt::FORCE_TRANSPARENT, Conv::Cdecl, 2, rng.next());
            let id = if !ids.is_empty() && rng.chance(2) {
                ids[rng.below(ids.len() as u32) as usize]
            } else {
                rng.below(5)
            };
            let a = match rng.below(4) {
                0 => 0.0f32,
                1 => 1.0,
                2 => 0.5,
                _ => rng.float(),
            };
            let x = rng.next();
            t.check(
                what,
                0x0063_bc50,
                Conv::Cdecl,
                &[list, x, id, a.to_bits()],
                |g| {
                    mt::set_mesh_segment_alpha(g, list, id, a);
                    Out::None
                },
            );
        }
        1 => {
            let scene = t.alloc(&rng.bytes(0x1130));
            t.w8(scene + 0x93c, u8::from(!rng.chance(5)));
            t.w32(scene + 0x1128, if rng.chance(5) { 0 } else { rng.next() });
            t.global(scene::bridge::SCENE.sym, scene);
            let buf = t.alloc(&rng.bytes(0x34));
            t.w32(buf, 0x30);
            let mat = t.zeros(8);
            t.w32(mat + 4, buf);
            t.stub_sym(mt::TEX_ANIM_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
            t.stub_sym(mt::TEX_ANIM_MATERIAL_BY_LAYER, Conv::Thiscall(0), 1, mat);
            let param = build_param(t, rng, 0x30, 16, true);
            t.stub_sym(mt::FIND_PARAMETER, Conv::Thiscall(0), 1, param);
            let (layer, alpha) = (rng.next(), rng.below(0x200));
            t.check(what, 0x0063_9d20, Conv::Cdecl, &[layer, alpha], |g| {
                mt::change_layer_alpha(g, layer, alpha);
                Out::None
            });
        }
        2 => {
            let n = rng.below(5);
            let mut arr = Vec::new();
            let small = |rng: &mut Rng| -> f32 {
                match rng.below(6) {
                    0 | 1 => rng.float(),
                    // signed zeros: equal, but not the same bits
                    2 => [0.0, -0.0][rng.below(2) as usize],
                    _ => rng.below(9) as f32 - 4.0,
                }
            };
            for _ in 0..n {
                let b: Vec<u8> = (0..8).flat_map(|_| small(rng).to_le_bytes()).collect();
                let b = t.alloc(&b);
                let inst = t.alloc(&rng.bytes(0x20));
                t.w32(inst + 0x1c, b);
                arr.extend(inst.to_le_bytes());
            }
            arr.extend([0; 4]);
            let arr = t.alloc(&arr);
            let list = t.zeros(8);
            t.w32(list, n | (rng.next() & 0x8000_0000));
            t.w32(list + 4, arr);
            t.stub_sym(
                ut::CH_GET_CHR_RENDER_INSTANCE,
                Conv::Cdecl,
                1,
                if rng.chance(8) { 0 } else { list },
            );
            let out = t.alloc(&rng.bytes(0x20));
            let chr = rng.next();
            t.check(
                what,
                0x0063_a2b0,
                Conv::Cdecl,
                &[chr, out, out + 0x10],
                |g| {
                    ut::character_bounding_box(g, chr, out, out + 0x10);
                    Out::None
                },
            );
        }
        _ => {
            // the inverse and the 3x4 conversion run, original on both sides
            if t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 || rng.chance(2) {
                let scene = t.alloc(&rng.bytes(0x10dd0));
                t.global(scene::bridge::SCENE.sym, scene);
            }
            let vals: Vec<u8> = (0..16)
                .flat_map(|_| {
                    let f = match rng.below(4) {
                        0 => rng.float(),
                        1 => 0.0,
                        _ => (rng.below(200) as f32 - 100.0) / 7.0,
                    };
                    f.to_le_bytes()
                })
                .collect();
            let m = t.alloc(&vals);
            t.check(what, 0x0063_8aa0, Conv::Cdecl, &[m], |g| {
                ut::split_screen_set_camera(g, m);
                Out::None
            });
        }
    }
}

#[test]
fn round15_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1515_0015);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round15_case(&mut t, &mut rng, round % 4, &what);
    }
}

#[test]
fn snapshot_round15_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5015_0015);
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        round15_case(&mut t, &mut rng, round % 4, &what);
    }
}

// ---- round 16 ----

/// A recording stub over `Phyre_CharacterLighting_setLightMatrix`
/// (`thiscall(this, inst, a, b)`, `ret 0xc`): logs the tag, `this`, `inst`
/// and the 16 words at each of `a` and `b` (5 entries; the pointers differ:
/// the original passes its stack, the bridge its scratch).
fn stub_light_matrix(t: &mut Twins, at: u32, tag: u32) {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(tag.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // [eax+8] = inst
    c.extend([0x56, 0x57]); // push esi; push edi
    c.extend([0x8d, 0x78, 0x20]); // lea edi, [eax+0x20]
    c.extend([0x8b, 0x74, 0x24, 0x10]); // mov esi, [esp+0x10] (a)
    c.extend([0xb9, 0x10, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 16; rep movsd
    c.extend([0x8b, 0x74, 0x24, 0x14]); // mov esi, [esp+0x14] (b)
    c.extend([0xb9, 0x10, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 16; rep movsd
    c.extend([0x5f, 0x5e]); // pop edi; pop esi
    c.extend([0x83, 0x05]);
    c.extend(log.to_le_bytes());
    c.push(5); // add dword [log], 5
    c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
    let addr = t.cave_next;
    assert!(addr + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(addr, &c);
    t.cave_next = addr + c.len() as u32;
    let mut j = vec![0xe9];
    j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

fn round16_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) {
    use ffx_graphic::util::bridge as ut;
    if case == 0 {
        let abmap = t.alloc(&rng.bytes(0xd0));
        t.w32(
            abmap + 4,
            [0, 1, 2, 3, 0xffff_ffff, rng.next()][rng.below(6) as usize],
        );
        for k in 0..12u32 {
            let n = if rng.chance(8) {
                rng.next() | 0x8000_0000
            } else {
                rng.below(4)
            };
            let segs = t.alloc(&rng.bytes(0x6c * 4));
            let rd = t.alloc(&rng.bytes(0x98));
            t.w32(rd + 0x90, n);
            t.w32(rd + 0x94, segs);
            t.w32(abmap + 0x98 + 4 * k, rd);
        }
        let abmap = if rng.chance(8) { 0 } else { abmap };
        t.global(ut::ABMAP.sym, abmap);
        t.check(what, 0x0063_90c0, Conv::Cdecl, &[], |g| {
            ut::abmap_reset_segments(g);
            Out::None
        });
    } else {
        let at = t.rebase(ut::LIGHTING_SET_LIGHT_MATRIX);
        stub_light_matrix(t, at, 0x4c4d);
        t.stub_sym(ut::LIGHTING_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
        let n = rng.below(4);
        let arr: Vec<u8> = (0..=n).flat_map(|_| rng.next().to_le_bytes()).collect();
        let arr = t.alloc(&arr);
        let list = t.zeros(8);
        t.w32(list, n | (rng.next() & 0x8000_0000));
        t.w32(list + 4, arr);
        let list = if rng.chance(10) { 0 } else { list };
        let dir = |rng: &mut Rng| -> f32 {
            match rng.below(5) {
                0 => rng.float(),
                1 => 0.0,
                2 => (rng.below(3) as f32) * 0.003,
                _ => (rng.below(200) as f32 - 100.0) / 13.0,
            }
        };
        let dirs: Vec<u8> = (0..16).flat_map(|_| dir(rng).to_le_bytes()).collect();
        let dirs = t.alloc(&dirs);
        let colours = t.alloc(&float_bytes(rng, 16));
        t.check(
            what,
            0x0063_cb80,
            Conv::Cdecl,
            &[list, dirs, colours],
            |g| {
                ut::set_light_matrix(g, list, dirs, colours);
                Out::None
            },
        );
    }
}

#[test]
fn round16_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1616_0016);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round16_case(&mut t, &mut rng, round % 2, &what);
    }
}

#[test]
fn snapshot_round16_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    // the game's own sphere-grid object, when there is one
    let abmap = t.r32(t.rebase(ut::ABMAP.sym));
    if abmap != 0 {
        t.check("snapshot abmap", 0x0063_90c0, Conv::Cdecl, &[], |g| {
            ut::abmap_reset_segments(g);
            Out::None
        });
    }
    let mut rng = Rng(0x5016_0016);
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        round16_case(&mut t, &mut rng, round % 2, &what);
    }
}

// ---- round 17 ----

/// Writes a `ShaderParams_FindByName` (`thiscall(shader, name)`, `ret 4`)
/// over the original that answers by the name's address: `params` pairs
/// a name (image address) with the parameter it finds; others find 0.
fn stub_find_param(t: &mut Twins, params: &[(u32, u32)]) {
    use ffx_graphic::material::bridge as mt;
    let at = t.rebase(mt::FIND_PARAMETER);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    for &(name, param) in params {
        c.push(0x3d);
        c.extend(t.rebase(name).to_le_bytes()); // cmp eax, name
        c.extend([0x75, 0x08]); // jne next
        c.push(0xb8);
        c.extend(param.to_le_bytes()); // mov eax, param
        c.extend([0xc2, 0x04, 0x00]); // ret 4
    }
    c.extend([0x31, 0xc0, 0xc2, 0x04, 0x00]); // xor eax, eax; ret 4
    t.write(at, &c);
}

fn round17_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) {
    use ffx_graphic::material::bridge as mt;
    use ffx_graphic::util::bridge as ut;
    if case == 0 {
        let (list, _) = build_material_list(t, rng, 0x30);
        t.stub_sym(
            mt::CH_GET_CHR_RENDER_INSTANCE,
            Conv::Cdecl,
            1,
            if rng.chance(8) { 0 } else { list },
        );
        let mut params = Vec::new();
        for name in [
            mt::NAME_FADE_BASIC_ALPHA,
            mt::NAME_FADE_Y_CENTER,
            mt::NAME_FADE_Z_CENTER,
        ] {
            if !rng.chance(5) {
                let fits = !rng.chance(5);
                let p = build_param(t, rng, 0x30, 4, fits);
                params.push((name, p));
            }
        }
        stub_find_param(t, &params);
        let axis = [0, 1, 2, rng.next()][rng.below(4) as usize];
        let center = rng.float();
        let chr = rng.next();
        t.check(
            what,
            0x0063_b5d0,
            Conv::Cdecl,
            &[chr, axis, center.to_bits()],
            |g| {
                mt::set_btu_fade_center(g, chr, axis, center);
                Out::None
            },
        );
    } else {
        if t.r32(t.rebase(scene::bridge::SCENE.sym)) == 0 {
            let scene = t.zeros(0x10);
            t.global(scene::bridge::SCENE.sym, scene);
        }
        let hash = if rng.chance(2) {
            ut::FLAT_SHADE_LEVELS[rng.below(3) as usize]
        } else {
            rng.below(0x3000)
        };
        t.stub_sym(ut::SCENE_GET_LEVEL_HASH, Conv::Thiscall(0), 0, hash);
        t.stub_sym(ut::LIGHTING_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
        t.stub_sym(
            ut::LIGHTING_SET_SHADE_COEFF,
            Conv::Thiscall(0),
            2,
            rng.next(),
        );
        t.stub_sym(
            ut::LIGHTING_RESET_SHADE_COEFF,
            Conv::Thiscall(0),
            1,
            rng.next(),
        );
        t.stub_sym(ut::LIGHTING_TWEAK_GET_INSTANCE, Conv::Cdecl, 0, rng.next());
        t.stub_sym(
            ut::LIGHTING_TWEAK_SET_SHADE_COEFF,
            Conv::Thiscall(0),
            3,
            rng.next(),
        );
        let n = rng.below(4);
        let arr: Vec<u8> = (0..=n).flat_map(|_| rng.next().to_le_bytes()).collect();
        let arr = t.alloc(&arr);
        let list = t.zeros(8);
        t.w32(list, n | (rng.next() & 0x8000_0000));
        t.w32(list + 4, arr);
        let list = if rng.chance(10) { 0 } else { list };
        t.check(what, 0x0063_cec0, Conv::Cdecl, &[list], |g| {
            ut::set_shade_coeff_debug(g, list);
            Out::None
        });
    }
}

#[test]
fn round17_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1717_0017);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round17_case(&mut t, &mut rng, round % 2, &what);
    }
}

#[test]
fn snapshot_round17_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5017_0017);
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        round17_case(&mut t, &mut rng, round % 2, &what);
    }
}

// ---- round 18 ----

/// A recording stub over the `thiscall` at `sym` taking ten words
/// (`ret 0x28`): logs the tag, `ecx` and the ten words over two entries.
fn stub_thiscall10(t: &mut Twins, sym: u32, ret: u32) {
    let at = t.rebase(sym);
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(sym.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    for i in 0..10u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    c.extend([0x83, 0x05]);
    c.extend(log.to_le_bytes());
    c.push(2); // add dword [log], 2
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.extend([0xc2, 0x28, 0x00]); // ret 0x28
    let addr = t.cave_next;
    assert!(addr + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(addr, &c);
    t.cave_next = addr + c.len() as u32;
    let mut j = vec![0xe9];
    j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

fn round18_case(t: &mut Twins, rng: &mut Rng, what: &str) {
    use ffx_graphic::state::bridge::SKIP_FRAME;
    use ffx_graphic::util::bridge as ut;
    t.global(SKIP_FRAME.sym, u32::from(rng.chance(8)));
    t.stub_sym(ut::SPECULAR_ADD_EVENT, Conv::Thiscall(0), 1, rng.next());
    stub_thiscall10(t, ut::SPECULAR_UPDATE_PARAMS, rng.next());
    let mgr = t.alloc(&rng.bytes(0x5b0));
    let ids = [0, 7, 9, 0x1234, rng.next()];
    for i in 0..4 {
        let v = if rng.chance(3) {
            0
        } else {
            ids[rng.below(5) as usize]
        };
        t.w32(mgr + 4 * i, v);
    }
    let mut p = [0u32; 10];
    p[0] = ids[rng.below(5) as usize];
    p[1] = rng.next();
    p[5] = rng.next();
    for k in [2, 3, 4, 6, 7, 8, 9] {
        p[k] = rng.float().to_bits();
    }
    t.check(what, 0x006c_7090, Conv::Thiscall(mgr), &p, |g| {
        ut::specular_set_params(g, mgr, p);
        Out::None
    });
}

#[test]
fn round18_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1818_0018);
    for round in 0..2000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round18_case(&mut t, &mut rng, &what);
    }
}

#[test]
fn snapshot_round18_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5018_0018);
    for round in 0..300 {
        t.reset();
        let what = format!("snapshot round {round}");
        round18_case(&mut t, &mut rng, &what);
    }
}

// ---- round 19 ----

/// One round-19 case; the box tests' result.
#[allow(clippy::many_single_char_names)] // box, matrix, value
fn round19_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) -> Option<u32> {
    use ffx_graphic::state::bridge::SKIP_FRAME;
    use ffx_graphic::util::bridge as ut;
    if case < 2 {
        // the transform and the original ClipOutcode run on side a
        let f = |rng: &mut Rng| -> f32 {
            match rng.below(8) {
                0 => rng.float(),
                1 => 0.0,
                2 => 1.0,
                _ => (rng.below(61) as f32 - 30.0) / 10.0,
            }
        };
        let bx: Vec<u8> = (0..8).flat_map(|_| f(rng).to_le_bytes()).collect();
        let bx = t.alloc(&bx);
        let m: Vec<u8> = (0..16)
            .flat_map(|k| {
                let v = if k == 15 && rng.chance(2) {
                    1.0
                } else {
                    f(rng)
                };
                v.to_le_bytes()
            })
            .collect();
        let m = t.alloc(&m);
        let (sym, test) = if case == 0 {
            (0x0066_3a30, ut::BoxTest::Visible)
        } else {
            (0x006d_27e0, ut::BoxTest::Inside)
        };
        let mut r = None;
        t.check(what, sym, Conv::Thiscall(bx), &[m], |g| {
            let v = ut::box_clip_test(g, bx, m, test);
            r = Some(v);
            Out::U32(v)
        });
        r
    } else {
        t.global(SKIP_FRAME.sym, u32::from(rng.chance(8)));
        let mgr = t.alloc(&rng.bytes(0x5b0));
        t.stub_sym(ut::SPECULAR_GET_INSTANCE, Conv::Cdecl, 0, mgr);
        t.stub_sym(ut::SPECULAR_ADD_EVENT, Conv::Thiscall(0), 1, rng.next());
        stub_thiscall10(t, ut::SPECULAR_UPDATE_PARAMS, rng.next());
        let mut p = [0u32; 10];
        p[0] = [0, 3, rng.next()][rng.below(3) as usize];
        p[1] = rng.next();
        p[5] = rng.next();
        for k in [2, 3, 4, 6, 7, 8, 9] {
            p[k] = rng.float().to_bits();
        }
        t.check(what, 0x0064_39b0, Conv::Cdecl, &p, |g| {
            ut::graphic_set_specular_params_ex(g, p);
            Out::None
        });
        None
    }
}

#[test]
fn round19_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x1919_0019);
    // each test must see both answers
    let mut seen = [[0u32; 2]; 2];
    for round in 0..6000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        if let Some(v) = round19_case(&mut t, &mut rng, round % 3, &what) {
            seen[(round % 3) as usize][v as usize] += 1;
        }
    }
    assert!(seen.iter().flatten().all(|&n| n > 50), "{seen:?}");
}

#[test]
fn snapshot_round19_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5019_0019);
    for round in 0..600 {
        t.reset();
        let what = format!("snapshot round {round}");
        round19_case(&mut t, &mut rng, round % 3, &what);
    }
}

/// Boxes on the clip boundary, where storing each sum as a float decides
/// the answer: x = ((cx + ex) + ey) + ez against w = 1 (rows 0-2 of the
/// matrix add their extent to x only).
#[test]
fn round19_box_rounding_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::image() else { return };
    let e24 = 2f32.powi(-24);
    let e23 = 2f32.powi(-23);
    let e26 = 2f32.powi(-26);
    let cases = [
        (1.0, [e24, e24, e24]),
        (1.0, [e24, e24, 0.0]),
        (1.0, [2f32.powi(-25), 0.0, 0.0]),
        (1.0 + e23, [-e23 + e26, 0.0, 0.0]),
        (1.0 + e23, [-e23 + e26, e26, e26]),
        (-1.0, [-e24, -e24, -e24]),
        (1.0, [-e24, e24, e24]),
    ];
    let mut m = [0f32; 16];
    for k in [0, 4, 8, 5, 10, 15] {
        m[k] = 1.0;
    }
    let m: Vec<u8> = m.iter().flat_map(|f| f.to_le_bytes()).collect();
    for (i, (cx, ext)) in cases.into_iter().enumerate() {
        for (sym, test) in [
            (0x0066_3a30, ut::BoxTest::Visible),
            (0x006d_27e0, ut::BoxTest::Inside),
        ] {
            t.reset();
            let mat = t.alloc(&m);
            let bx: Vec<u8> = [cx, 0.0, 0.0, 0.0, ext[0], ext[1], ext[2], 0.0]
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            let bx = t.alloc(&bx);
            let what = format!("case {i} {test:?}");
            t.check(&what, sym, Conv::Thiscall(bx), &[mat], |g| {
                Out::U32(ut::box_clip_test(g, bx, mat, test))
            });
        }
    }
}

// ---- round 20 ----

fn round20_case(t: &mut Twins, rng: &mut Rng, what: &str) {
    use ffx_graphic::util::bridge as ut;
    // PMatrix4_RowTimes runs, original on both sides
    let scene = t.alloc(&rng.bytes(0xfa00));
    let f = |rng: &mut Rng| -> f32 {
        if rng.chance(3) {
            rng.float()
        } else {
            (rng.below(41) as f32 - 20.0) / 16.0
        }
    };
    for off in [0xf9bc, 0xf9c0] {
        let v = f(rng);
        t.write(scene + off, &v.to_le_bytes());
    }
    let cam: Vec<u8> = (0..0x20).flat_map(|_| f(rng).to_le_bytes()).collect();
    let cam = t.alloc(&cam);
    t.w32(scene + 0xdc, cam);
    let out = t.alloc(&rng.bytes(0x40));
    t.check(what, 0x0065_1650, Conv::Thiscall(scene), &[out], |g| {
        Out::U32(ut::scene_screen_adjusted_view(g, scene, out))
    });
}

#[test]
fn round20_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2020_0020);
    for round in 0..2000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round20_case(&mut t, &mut rng, &what);
    }
}

#[test]
fn snapshot_round20_matches_the_original() {
    use ffx_graphic::util::bridge as ut;
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    // the game's own scene and camera
    let scene = t.r32(t.rebase(scene::bridge::SCENE.sym));
    if scene != 0 && t.r32(scene + 0xdc) != 0 {
        let out = t.zeros(0x40);
        t.check(
            "snapshot scene",
            0x0065_1650,
            Conv::Thiscall(scene),
            &[out],
            |g| Out::U32(ut::scene_screen_adjusted_view(g, scene, out)),
        );
    }
    let mut rng = Rng(0x5020_0020);
    for round in 0..300 {
        t.reset();
        let what = format!("snapshot round {round}");
        round20_case(&mut t, &mut rng, &what);
    }
}

// ---- round 21 ----

/// One round-21 case; for the box test, its answer (bit 1: with a
/// transform).
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // box, matrix, value; three cases
fn round21_case(t: &mut Twins, rng: &mut Rng, case: u32, what: &str) -> Option<u32> {
    use ffx_graphic::util::bridge as ut;
    match case {
        0 => {
            let lighting = rng.next();
            let tweak = rng.next();
            t.stub_sym(ut::LIGHTING_GET_INSTANCE, Conv::Cdecl, 0, lighting);
            t.stub_sym(ut::LIGHTING_TWEAK_GET_INSTANCE, Conv::Cdecl, 0, tweak);
            t.stub_sym(
                ut::LIGHTING_SET_LIGHT_COLOR,
                Conv::Thiscall(0),
                4,
                rng.next(),
            );
            t.stub_sym(
                ut::LIGHTING_TWEAK_SYNC_INTENSITY,
                Conv::Thiscall(0),
                0,
                rng.next(),
            );
            t.stub_sym(
                ut::LIGHTING_BALANCE_LUM_WITH_GLOW,
                Conv::Thiscall(0),
                0,
                rng.next(),
            );
            let kind = [0, 1, 2, 3, 3, rng.next()][rng.below(6) as usize];
            let c = [rng.float(), rng.float(), rng.float()].map(f32::to_bits);
            t.check(
                what,
                0x0063_cb00,
                Conv::Cdecl,
                &[kind, c[0], c[1], c[2]],
                |g| Out::U32(ut::set_light_colour(g, kind, c)),
            );
            None
        }
        1 => {
            let list = if rng.chance(6) {
                0
            } else {
                let n = rng.below(6);
                let arr: Vec<u8> = (0..n).flat_map(|_| rng.next().to_le_bytes()).collect();
                let arr = t.alloc(&arr);
                let list = t.zeros(8);
                t.w32(list, n | (rng.below(2) << 31));
                t.w32(list + 4, arr);
                list
            };
            t.stub_sym(ut::CH_GET_CHR_RENDER_INSTANCE, Conv::Cdecl, 1, list);
            t.stub_sym(ut::SHADER_SET_PARAMETER_VECTOR, Conv::Cdecl, 6, rng.next());
            let p: [u32; 6] = std::array::from_fn(|_| rng.next());
            t.check(what, 0x0063_d940, Conv::Cdecl, &p, |g| {
                Out::U32(ut::chr_shader_set_parameter_vector(
                    g,
                    p[0],
                    [p[1], p[2], p[3], p[4], p[5]],
                ))
            });
            None
        }
        _ => {
            // the matrix helpers, the transform and ClipOutcode run original
            let f = |rng: &mut Rng| -> f32 {
                match rng.below(10) {
                    0 => rng.float(),
                    1 => 0.0,
                    2 => 1.0,
                    _ => (rng.below(61) as f32 - 30.0) / 10.0,
                }
            };
            let bx: Vec<u8> = (0..8).flat_map(|_| f(rng).to_le_bytes()).collect();
            let bx = t.alloc(&bx);
            let xf = if rng.chance(3) {
                0
            } else {
                let xf: Vec<u8> = (0..12).flat_map(|_| f(rng).to_le_bytes()).collect();
                t.alloc(&xf)
            };
            let this = t.alloc(&rng.bytes(0x20));
            for k in 0..3 {
                let v = t.r32(bx + 4 * k);
                t.w32(this + 4 * k, v);
            }
            t.w32(this + 0xc, xf);
            for k in 0..3 {
                let v = t.r32(bx + 0x10 + 4 * k);
                t.w32(this + 0x10 + 4 * k, v);
            }
            let m: Vec<u8> = (0..16)
                .flat_map(|k| {
                    let v = if k == 15 && rng.chance(2) {
                        1.0
                    } else {
                        f(rng)
                    };
                    v.to_le_bytes()
                })
                .collect();
            let m = t.alloc(&m);
            let mut r = None;
            t.check(what, 0x006d_2a70, Conv::Thiscall(this), &[m], |g| {
                let v = ut::box_inside_transformed(g, this, m);
                r = Some(v);
                Out::U32(v)
            });
            r.map(|v| v | u32::from(xf != 0) << 1)
        }
    }
}

#[test]
fn round21_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2121_0021);
    // the box test must see both answers with and without a transform
    let mut seen = [0u32; 4];
    for round in 0..4500 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        if let Some(v) = round21_case(&mut t, &mut rng, round % 3, &what) {
            seen[v as usize] += 1;
        }
    }
    assert!(seen.iter().all(|&n| n > 30), "box answers {seen:?}");
}

#[test]
fn snapshot_round21_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5121_0021);
    for round in 0..450 {
        t.reset();
        let what = format!("snapshot round {round}");
        round21_case(&mut t, &mut rng, round % 3, &what);
    }
}

// ---- round 22 ----

/// Writes a stub over `graphicFontGetScreenWH(w*, h*)` that stores `w`
/// and `h`.
fn stub_screen_wh(t: &mut Twins, width: u32, height: u32) {
    let at = t.rebase(0x0064_0db0);
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov [eax], w
    c.extend(width.to_le_bytes());
    c.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]); // mov eax, [esp+8]; mov [eax], h
    c.extend(height.to_le_bytes());
    c.push(0xc3);
    let addr = t.cave_next;
    assert!(addr + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(addr, &c);
    t.cave_next = addr + c.len() as u32;
    let mut j = vec![0xe9];
    j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

/// One round-22 case on `scene` (a fresh one when 0); the matrix
/// helpers, `PMatrix4_RowTimes` and the screen-adjusted view run
/// original on both sides.
fn round22_case(t: &mut Twins, rng: &mut Rng, scene: u32, what: &str) {
    project_case(t, rng, scene, None, what);
}

/// [`round22_case`], or with `wrapper` `(sym, mode)` the cdecl
/// `graphicProject3DPointOntoScreen*` wrapper at `sym` on `scene` made
/// `g_PhyreScene`.
fn project_case(t: &mut Twins, rng: &mut Rng, scene: u32, wrapper: Option<(u32, u32)>, what: &str) {
    use ffx_bridge::x87::SSE2_AVAILABLE;
    use ffx_graphic::util::bridge as ut;
    let float = |rng: &mut Rng| -> f32 {
        match rng.below(12) {
            0 => rng.float(),
            1 => 0.0,
            2 => -0.0,
            3 => 1.0,
            _ => (rng.below(81) as f32 - 40.0) / 8.0,
        }
    };
    let floats = |t: &mut Twins, rng: &mut Rng, n: usize| -> u32 {
        let v: Vec<u8> = (0..n).flat_map(|_| float(rng).to_le_bytes()).collect();
        t.alloc(&v)
    };
    let scene = if scene == 0 {
        let scene = t.alloc(&rng.bytes(0xfa00));
        for off in (0xc50..0xc90).step_by(4).chain([0xf9bc, 0xf9c0]) {
            let v = float(rng);
            t.write(scene + off, &v.to_le_bytes());
        }
        let cam = floats(t, rng, 0x30);
        let xf = if rng.chance(3) { 0 } else { floats(t, rng, 12) };
        t.w32(cam + 0xb4, xf);
        t.w32(scene + 0xdc, cam);
        scene
    } else {
        scene
    };
    let at = t.rebase(scene::bridge::IS_3D.sym);
    t.w8(at, u8::from(rng.chance(2)));
    t.global(SSE2_AVAILABLE.sym, u32::from(rng.chance(2)));
    let (width, height) = match rng.below(4) {
        0 => (1920, 1080),
        1 => (rng.next(), rng.next()),
        _ => (rng.below(4000), rng.below(3000)),
    };
    stub_screen_wh(t, width, height);
    let p = floats(t, rng, 3);
    let out = t.alloc(&rng.bytes(8));
    if let Some((sym, mode)) = wrapper {
        t.global(scene::bridge::SCENE.sym, scene);
        t.check(what, sym, Conv::Cdecl, &[out, p], |g| {
            Out::F32(ut::project_point(g, out, p, mode))
        });
        return;
    }
    let mode = [0, 1, 2, 2, 3, rng.next()][rng.below(6) as usize];
    t.check(
        what,
        0x0065_26a0,
        Conv::Thiscall(scene),
        &[out, p, mode],
        |g| Out::F32(ut::scene_project_point(g, scene, out, p, mode)),
    );
}

#[test]
fn round22_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2222_0022);
    for round in 0..4000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round22_case(&mut t, &mut rng, 0, &what);
    }
}

#[test]
fn snapshot_round22_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let real = t.r32(t.rebase(scene::bridge::SCENE.sym));
    let real = if real != 0 && t.r32(real + 0xdc) != 0 {
        real
    } else {
        0
    };
    let mut rng = Rng(0x5222_0022);
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        // the game's own scene and camera every other round
        let scene = if round % 2 == 0 { real } else { 0 };
        round22_case(&mut t, &mut rng, scene, &what);
    }
}

// ---- round 22, group 2 ----

/// `graphicGetAbmapUV_006410a0`: `SheetCellUV` runs original on both
/// sides.
fn round22b_abmap(t: &mut Twins, rng: &mut Rng, what: &str) {
    use ffx_graphic::util::bridge as ut;
    if rng.chance(2) {
        t.global(ui::bridge::ABMAP.sym, rng.next());
    }
    let kinds = [
        0x3700,
        0x3720,
        0x3740,
        0x3760,
        0x37a0,
        0x3780,
        0,
        rng.next(),
    ];
    let kind = kinds[rng.below(8) as usize];
    let out = t.alloc(&rng.bytes(8));
    let p = [
        kind,
        rng.float().to_bits(),
        rng.float().to_bits(),
        out,
        out + 4,
    ];
    t.check(what, 0x0064_10a0, Conv::Cdecl, &p, |g| {
        Out::U32(ut::abmap_sheet_cell_uv(g, p))
    });
}

/// `IconPanel_showIcon` on `panel` (a fresh one when 0).
fn round22b_icon(t: &mut Twins, rng: &mut Rng, panel: u32, what: &str) {
    use ffx_graphic::util::bridge as ut;
    let panel = if panel == 0 {
        // padding before it for the negative indices
        let mut p = rng.bytes(0x60);
        for i in 0..7 {
            if rng.chance(2) {
                p[0x20 + 4 * i..0x24 + 4 * i].fill(0);
            }
        }
        t.alloc(&p) + 0x20
    } else {
        panel
    };
    t.stub_sym(ut::IGGY_PLAYER_OPEN_SCALED, Conv::Cdecl, 5, rng.next());
    t.stub_sym(ut::IGGY_MENU_SET_ENABLE, Conv::Thiscall(0), 1, rng.next());
    let i = [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        6,
        7,
        8,
        u32::MAX,
        u32::MAX - 1,
        u32::MAX - 2,
    ][rng.below(13) as usize];
    t.check(what, 0x006f_7970, Conv::Thiscall(panel), &[i], |g| {
        Out::U32(ut::icon_panel_show_icon(g, panel, i))
    });
}

/// `SpdCtrl_ScaleElapsedTime`: the predicates stubbed (or, with `real`,
/// left original on the snapshot's state), each blocking condition rare.
fn round22b_speed(t: &mut Twins, rng: &mut Rng, real: bool, what: &str) {
    use ffx_graphic::util::bridge as ut;
    let al = |rng: &mut Rng, p: u32| (rng.next() & !0xff) | u32::from(rng.chance(p));
    if !real {
        let status = [1, 1, 0, 2, rng.next()][rng.below(5) as usize];
        t.stub_sym(ut::MS_BATTLE_GET_STATUS, Conv::Cdecl, 0, status);
        let uc = if rng.chance(3) { 0 } else { rng.next() | 1 };
        t.stub_sym(ut::GET_UC_STATUS, Conv::Cdecl, 0, uc);
        let map = al(rng, 6);
        t.stub_sym(ut::SPD_CTRL_IS_MAP_EXCLUDED, Conv::Cdecl, 1, map);
        let magic = al(rng, 6);
        t.stub_sym(ut::SPD_CTRL_IS_MAGIC_EXCLUDED, Conv::Cdecl, 0, magic);
        let abmap = if rng.chance(6) { rng.next() | 1 } else { 0 };
        t.stub_sym(ut::GRAPHIC_GET_ABMAP, Conv::Cdecl, 0, abmap);
        let rare = |rng: &mut Rng| if rng.chance(8) { rng.next() | 1 } else { 0 };
        let v = rare(rng);
        t.global(ut::WAIT_FRAMES_BY_TIME.sym, v);
        let v = rng.next();
        t.global(ut::CURRENT_MAP_NO.sym, v);
        let v = rare(rng);
        t.global(ut::SAVE_MACHINE_BUSY.sym, v);
        let v = rare(rng);
        t.global(ut::MOVIE_FADING.sym, v);
        let at = t.rebase(ut::BTL_END_CTRL_DONE.sym);
        t.w8(at, [1, 1, 1, 0, 0xff][rng.below(5) as usize]);
    }
    t.stub_sym(ut::IGGY_CALL_AS3_EXTERNAL, Conv::Thiscall(0), 5, rng.next());
    t.stub_sym(ut::SG_SET_MUTE_SOUND, Conv::Cdecl, 1, rng.next());
    let at = t.rebase(ut::SPD_CTRL_ENABLED.sym);
    t.w8(at, [1, 1, 1, 1, 0, 2][rng.below(6) as usize]);
    let level = [0, 1, 1, 2, 2, 3][rng.below(6) as usize];
    t.global(ut::SPD_CTRL_B4.sym, level);
    let was = [0, 1, rng.next()][rng.below(3) as usize];
    t.global(ut::SPD_CTRL_B8.sym, was);
    let menu = if rng.chance(3) { 0 } else { rng.next() | 1 };
    t.global(ut::SPD_CTRL_C0.sym, menu);
    let dt = match rng.below(4) {
        0 => rng.float(),
        1 => 1.0 / 60.0,
        _ => rng.below(1000) as f32 / 997.0,
    }
    .to_bits();
    t.check(what, 0x006f_7340, Conv::Cdecl, &[dt], |g| {
        Out::F32(ut::spd_ctrl_scale_elapsed_time(g, dt))
    });
}

/// A stub over `dbgPrintf` for `Cache_OverFlow`: logs the format, and the
/// two arguments after it only for the one format that has them (the
/// other calls leave the caller's stack there).
fn stub_overflow_printf(t: &mut Twins, ret: u32) {
    use cache::bridge as cb;
    let one = t.stub(None, cb::DBG_PRINTF, false, 1, 0, ret);
    let three = t.stub(None, cb::DBG_PRINTF, false, 3, 0, ret);
    let fmt = t.rebase(cb::OVERFLOW_DISPOSED);
    let at = t.cave_next;
    let mut c = vec![0x81, 0x7c, 0x24, 0x04]; // cmp dword [esp+4], fmt
    c.extend(fmt.to_le_bytes());
    c.extend([0x0f, 0x84]); // je three
    c.extend(three.wrapping_sub(at + 14).to_le_bytes());
    c.push(0xe9); // jmp one
    c.extend(one.wrapping_sub(at + 19).to_le_bytes());
    assert!(at + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(at, &c);
    t.cave_next = at + c.len() as u32;
    let f = t.rebase(cb::DBG_PRINTF);
    let mut j = vec![0xe9];
    j.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.write(f, &j);
}

/// `Cache_OverFlow` on the cache as it is (`fresh`: a random one, with
/// small data words and ages, sometimes nothing to free).
fn round22b_overflow(t: &mut Twins, rng: &mut Rng, fresh: bool, what: &str) -> u32 {
    use cache::bridge as cb;
    if fresh {
        build_cache(t, rng);
        let table = t.rebase(cb::TABLE.sym);
        let none = rng.chance(6);
        for i in 0..cache::SLOTS {
            let e = t.r32(table + 4 * i);
            if none {
                // unread, in use, pinned or free: nothing to free
                match rng.below(4) {
                    0 => t.w8(e + 6, 1),
                    1 => t.w8(e + 5, 1),
                    2 => t.w8(e + 8, 7 | (rng.next() as u8 & 0xf8)),
                    _ => t.w32(e + 0xc, 0),
                }
                continue;
            }
            if rng.chance(2) {
                t.w8(e + 5, 0);
                t.w8(e + 6, 0);
            }
            if rng.chance(3) {
                // ids with the top bit: the logged high half is signed
                t.w32(e, rng.next());
            }
            if rng.chance(2) {
                let data = [
                    rng.below(0x2000_0000),
                    rng.below(64),
                    0x0fff_ffff,
                    0x0fff_fffe,
                ][rng.below(4) as usize];
                t.w32(e + 0xc, data);
            }
        }
    }
    stub_overflow_printf(t, rng.next());
    t.stub_sym(cb::CH_CREATE_NAME, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cb::FREE_ENTRY, Conv::Cdecl, 1, rng.next());
    let mut r = 0;
    t.check(what, 0x0084_09a0, Conv::Cdecl, &[], |g| {
        r = cb::overflow(g);
        Out::U32(r)
    });
    r
}

#[test]
fn round22b_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2222_b022);
    let mut freed = [0u32; 2];
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        match round % 4 {
            0 => round22b_abmap(&mut t, &mut rng, &what),
            1 => round22b_icon(&mut t, &mut rng, 0, &what),
            2 => round22b_speed(&mut t, &mut rng, false, &what),
            _ => freed[round22b_overflow(&mut t, &mut rng, true, &what) as usize] += 1,
        }
    }
    assert!(freed.iter().all(|&n| n > 50), "overflow answers {freed:?}");
}

#[test]
fn snapshot_round22b_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5222_b022);
    // first on the game's own state, before any stub or random cache
    // replaces it: the real cache, the real speed-up predicates
    let freed = round22b_overflow(&mut t, &mut rng, false, "snapshot cache");
    eprintln!("the snapshot's cache: Cache_OverFlow returns {freed}");
    for round in 0..40 {
        t.reset();
        let what = format!("snapshot real speed {round}");
        round22b_speed(&mut t, &mut rng, true, &what);
    }
    let panel = t.r32(t.rebase(0x00ce_82e4));
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        match round % 4 {
            0 => round22b_abmap(&mut t, &mut rng, &what),
            // the game's own icon panel, when there is one
            1 => {
                let p = if round % 8 == 1 { panel } else { 0 };
                round22b_icon(&mut t, &mut rng, p, &what);
            }
            2 => round22b_speed(&mut t, &mut rng, false, &what),
            _ => {
                round22b_overflow(&mut t, &mut rng, true, &what);
            }
        }
    }
}

// ---- round 23 ----

/// The projection wrappers: `(sym, mode)`.
const PROJECT_WRAPPERS: [(u32, u32); 2] = [(0x0064_28f0, 1), (0x0064_28d0, 2)];

/// Stubs the cache releasers' callees. `persistent` is what
/// `Cache_IsPersistentChr` answers. `Cache_Dispose`'s delayed-release
/// call also replaces the first model of `entry` (when set) with `model`,
/// so the bridge must read it again after the call.
fn stub_release_callees(t: &mut Twins, rng: &mut Rng, persistent: u32, entry: u32, model: u32) {
    use cache::bridge as cb;
    t.stub_sym(cb::IS_PERSISTENT_CHR, Conv::Cdecl, 1, persistent);
    t.stub_sym(cb::UNLOAD_TASK, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cb::USER_FREE, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cb::FREE_CLOTH_VERTEX_BUFF, Conv::Cdecl, 1, rng.next());
    t.stub_sym(cb::EMPTY, Conv::Cdecl, 0, rng.next());
    t.stub_sym(cb::FREE_ENTRY, Conv::Cdecl, 1, rng.next());
    let log = t.stub(None, cb::ENABLE_DELAY_RELEASE, false, 0, 0, rng.next());
    let at = t.cave_next;
    let mut c = Vec::new();
    if entry != 0 {
        c.extend([0xc7, 0x05]); // mov dword [entry + 0x10], model
        c.extend((entry + 0x10).to_le_bytes());
        c.extend(model.to_le_bytes());
    }
    c.push(0xe9); // jmp log
    c.extend(log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
    assert!(at + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(at, &c);
    t.cave_next = at + c.len() as u32;
    let f = t.rebase(cb::ENABLE_DELAY_RELEASE);
    let mut j = vec![0xe9];
    j.extend(at.wrapping_sub(f + 5).to_le_bytes());
    t.write(f, &j);
}

/// One cache releaser on the cache as it is (`fresh`: a random one with
/// small ages). Returns what `Cache_Dispose` answered (3 for the others).
fn round23_cache(t: &mut Twins, rng: &mut Rng, fresh: bool, what: &str) -> u32 {
    use cache::bridge as cb;
    let table = t.rebase(cb::TABLE.sym);
    let keys = if fresh {
        let keys = build_cache(t, rng);
        for i in 0..cache::SLOTS {
            let e = t.r32(table + 4 * i);
            if rng.chance(2) {
                let b = t.r8(e + 8);
                t.w8(e + 8, (b & !7) | [0, 0, 5, 7, 1, 3][rng.below(6) as usize]);
            }
            if rng.chance(2) {
                t.w8(e + 5, 0);
            }
            if rng.chance(2) {
                t.w8(e + 6, 0);
            }
        }
        keys
    } else {
        (0..cache::SLOTS)
            .map(|i| {
                let e = t.r32(table + 4 * i);
                (i32::from(t.r8(e + 4) as i8), t.r32(e))
            })
            .collect()
    };
    let persistent = if rng.chance(4) { rng.next() | 1 } else { 0 };
    let op = rng.below(4);
    let (kind, id) = if rng.chance(5) {
        (rng.below(6) as i32 - 1, rng.below(14))
    } else {
        keys[rng.below(keys.len() as u32) as usize]
    };
    let k = kind as u32;
    let target = if op == 3 {
        cb::find_live(&t.b, kind, id).unwrap_or(0)
    } else {
        0
    };
    let model = if rng.chance(3) { 0 } else { rng.next() };
    stub_release_callees(t, rng, persistent, target, model);
    match op {
        0 => t.check(what, 0x0084_0030, Conv::Cdecl, &[], |g| {
            cb::clear(g, false);
            // eax: Cache_FreeEntry's or the caller's, which nobody reads
            Out::None
        }),
        1 => t.check(what, 0x0084_0150, Conv::Cdecl, &[], |g| {
            cb::clear(g, true);
            // eax: as Cache_Clear's, al partly the last age
            Out::None
        }),
        2 => t.check(what, 0x0084_0440, Conv::Cdecl, &[], |g| {
            cb::dispose_check(g);
            Out::None
        }),
        _ => {
            let check = [0, 1, rng.next()][rng.below(3) as usize];
            let mut r = 0;
            t.check(what, 0x0084_0250, Conv::Cdecl, &[k, id, check], |g| {
                r = cb::dispose(g, kind, id, check);
                Out::U32(r)
            });
            return r;
        }
    }
    3
}

#[test]
fn round23_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2323_0023);
    let mut answers = [0u32; 4];
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        if round % 4 == 0 {
            let w = PROJECT_WRAPPERS[(round / 4) % 2];
            project_case(&mut t, &mut rng, 0, Some(w), &what);
        } else {
            let r = round23_cache(&mut t, &mut rng, true, &what);
            answers[match r {
                0 => 0,
                1 => 1,
                u32::MAX => 2,
                _ => 3,
            }] += 1;
        }
    }
    assert!(answers.iter().all(|&n| n > 50), "answers {answers:?}");
}

#[test]
fn snapshot_round23_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5323_0023);
    // the game's own cache first, each releaser on a fresh copy
    for op in 0..8 {
        let mut s = Twins::snapshot().unwrap();
        let mut r = Rng(0x5323_1000 + op);
        let what = format!("snapshot cache {op}");
        round23_cache(&mut s, &mut r, false, &what);
    }
    let real = t.r32(t.rebase(scene::bridge::SCENE.sym));
    let real = if real != 0 && t.r32(real + 0xdc) != 0 {
        real
    } else {
        0
    };
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        if round % 2 == 0 {
            // the game's own scene every other wrapper round
            let scene = if round % 4 == 0 { real } else { 0 };
            let w = PROJECT_WRAPPERS[(round / 2) % 2];
            project_case(&mut t, &mut rng, scene, Some(w), &what);
        } else {
            round23_cache(&mut t, &mut rng, true, &what);
        }
    }
}

// ---- round 23, group 2: the 2D scene's layers ----

/// A name for an instance: the pattern, inside other text or not, a
/// neighbour number, or a long one that runs into the original's pattern
/// buffer (at most `max` bytes before the NUL).
fn layer_name(rng: &mut Rng, pattern: &str, max: usize) -> Vec<u8> {
    let letters = |rng: &mut Rng, n: usize| -> String {
        (0..n)
            .map(|_| {
                ["a", "e", "l", "y", "r", "s", "c", "n", "1", "2", "_"][rng.below(11) as usize]
            })
            .collect()
    };
    let mut s = match rng.below(8) {
        0 => pattern.to_string(),
        1 => format!("{}{pattern}{}", letters(rng, 3), letters(rng, 2)),
        2 => format!("{pattern}{}", rng.below(10)),
        3 => format!("x{}", &pattern[..pattern.len() - 1]),
        4 => {
            let n = rng.below(12) as usize;
            letters(rng, n)
        }
        5 => {
            // long: what passes the name buffer becomes the pattern
            let n = max - rng.below(12) as usize;
            let mut s = letters(rng, n);
            if rng.chance(2) {
                s.replace_range(..pattern.len(), pattern);
            }
            s
        }
        6 => "a".repeat(max - rng.below(4) as usize),
        _ => String::new(),
    }
    .into_bytes();
    s.truncate(max);
    s.push(0);
    s
}

/// A list of the 2D level: nodes of `PhyreSceneCursor_*` (`+0` next,
/// `+8` link, `+0x2c` class, `+0x34` position, `+0x3c` count, `+0x40`
/// set), linked from the end node (`list +0x1c`); each node's instances
/// (`+4` floats, `+0x1c` target, `+0x2c` the tagged name), some slots
/// skipped by the link chain. Returns the list.
fn build_layer_list(t: &mut Twins, rng: &mut Rng, patterns: &[String], max: usize) -> u32 {
    let class = t.rebase(ffx_graphic::util::bridge::CURSOR_CLASS);
    let list = t.alloc(&rng.bytes(0x40));
    let end = list + 0x1c;
    let n = rng.below(4) as usize;
    let nodes: Vec<u32> = (0..n).map(|_| t.alloc(&rng.bytes(0x48))).collect();
    for (k, &node) in nodes.iter().enumerate() {
        t.w32(node, nodes.get(k + 1).copied().unwrap_or(end));
        let stride = [0x40, 0x48, 0x60][rng.below(3) as usize];
        let offset = [0, 8, 0x10][rng.below(3) as usize];
        let fake = t.alloc(&rng.bytes(0x90));
        t.w32(fake + 0x1c, stride);
        t.w32(fake + 0x8c, offset);
        // a node of another class is passed over by the cursor
        t.w32(fake + 0x40, if rng.chance(6) { 0 } else { class });
        t.w32(node + 0x2c, fake);
        t.w32(node + 0x40, u32::from(!rng.chance(8)));
        let slots = 1 + rng.below(5);
        let region = t.alloc(&rng.bytes((slots * stride + 0x60) as usize));
        let far = region + slots * stride + 0x40;
        t.w32(far, far + 4);
        for j in 0..slots {
            let obj = region + j * stride + offset;
            let floats: Vec<u8> = (0..12).flat_map(|_| rng.float().to_le_bytes()).collect();
            let m = t.alloc(&floats);
            t.w32(obj + 4, m);
            let target = if rng.chance(4) {
                0
            } else {
                t.alloc(&rng.bytes(0x20))
            };
            t.w32(obj + 0x1c, target);
            let pattern = &patterns[rng.below(patterns.len() as u32) as usize];
            let name = layer_name(rng, pattern, max);
            let at = t.alloc(&name);
            t.w32(obj + 0x2c, at | u32::from(rng.chance(3)));
        }
        // the link: never met, or a slot (then its word, the next link)
        let link = if rng.chance(2) {
            far
        } else {
            region + rng.below(slots + 1) * stride
        };
        let next = if rng.chance(2) {
            far
        } else {
            region + (1 + rng.below(slots)) * stride
        };
        if link != far && next > link {
            t.w32(link, next);
        } else if link != far {
            t.w32(link, far);
        }
        t.w32(node + 8, link);
        t.w32(node + 0x34, region);
        t.w32(node + 0x3c, slots);
    }
    t.w32(end, nodes.first().copied().unwrap_or(end));
    list
}

/// A random scene with a 2D level of up to four lists.
fn build_layer_scene(t: &mut Twins, rng: &mut Rng, patterns: &[String], max: usize) -> u32 {
    let scene = t.alloc(&rng.bytes(0xfa00));
    t.w8(scene + 0x93c, u8::from(!rng.chance(8)));
    t.w32(
        scene + 0x1128,
        if rng.chance(8) { 0 } else { rng.next() | 1 },
    );
    let n = rng.below(5);
    let lists: Vec<u8> = (0..n)
        .flat_map(|_| build_layer_list(t, rng, patterns, max).to_le_bytes())
        .collect();
    let at = t.alloc(&lists);
    t.w32(scene + 0x112c, n);
    t.w32(scene + 0x1134, at);
    scene
}

/// One of the six layer functions on `scene` (a fresh one when 0).
fn round23b_case(t: &mut Twins, rng: &mut Rng, scene: u32, what: &str) {
    use ffx_graphic::util::bridge as ut;
    let op = rng.below(6);
    let scenes = op.is_multiple_of(3);
    // a layer past 99 overruns the original's 8-byte pattern buffer
    let n = if scenes {
        [0, 1, 1, 2, 3, rng.below(20), rng.next()][rng.below(7) as usize]
    } else {
        (rng.below(109) as i32 - 9) as u32
    };
    let (prefix, max) = if scenes {
        ("scene", 151)
    } else {
        ("layer", 59)
    };
    let patterns: Vec<String> = [n, n.wrapping_add(1), n / 10, rng.below(4)]
        .iter()
        .map(|&k| format!("{prefix}{}", k as i32))
        .collect();
    let scene = if scene == 0 {
        build_layer_scene(t, rng, &patterns, max)
    } else {
        scene
    };
    let int = |rng: &mut Rng| {
        [
            0,
            1,
            rng.below(64),
            rng.below(64).wrapping_neg(),
            rng.next(),
        ][rng.below(5) as usize]
    };
    let (x, y) = (int(rng), int(rng));
    let (sx, sy) = (rng.float(), rng.float());
    t.global(scene::bridge::SCENE.sym, scene);
    match op {
        0 => t.check(what, 0x0065_87d0, Conv::Thiscall(scene), &[n], |g| {
            ut::change_2d_scene_num(g, scene, n);
            Out::None
        }),
        1 => t.check(what, 0x0065_8350, Conv::Thiscall(scene), &[n, x, y], |g| {
            ut::change_layer_offset(g, scene, n, x, y);
            Out::None
        }),
        2 => t.check(
            what,
            0x0065_8570,
            Conv::Thiscall(scene),
            &[n, x, y, sx.to_bits(), sy.to_bits()],
            |g| {
                ut::change_layer_scale(g, scene, n, x, y, sx, sy);
                Out::None
            },
        ),
        3 => t.check(what, 0x0063_9d90, Conv::Cdecl, &[n], |g| {
            ut::graphic_change_scene_num(g, n);
            Out::None
        }),
        4 => t.check(what, 0x0063_9d40, Conv::Cdecl, &[n, x, y], |g| {
            ut::graphic_change_layer_offset(g, n, x, y);
            Out::None
        }),
        _ => t.check(
            what,
            0x0063_9d60,
            Conv::Cdecl,
            &[n, x, y, sx.to_bits(), sy.to_bits()],
            |g| {
                ut::graphic_change_layer_scale(g, n, x, y, sx, sy);
                Out::None
            },
        ),
    }
}

#[test]
fn round23b_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2323_b023);
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        round23b_case(&mut t, &mut rng, 0, &what);
    }
}

#[test]
fn snapshot_round23b_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5323_b023);
    let real = t.r32(t.rebase(scene::bridge::SCENE.sym));
    let loaded = real != 0 && t.r8(real + 0x93c) != 0;
    eprintln!("the snapshot's scene {real:#x}: 2D level loaded {loaded}");
    for round in 0..400 {
        t.reset();
        let what = format!("snapshot round {round}");
        // the game's own scene every other round
        let scene = if round % 2 == 0 && real != 0 { real } else { 0 };
        round23b_case(&mut t, &mut rng, scene, &what);
    }
}

// ---- round 23, group 3 ----

/// A recording stub for `thiscall (a, record*)`, `ret 8`: logs the tag,
/// `ecx`, `a` and the record's four words (its address differs: the
/// original's stack, the bridge's scratch).
fn stub_record4(t: &mut Twins, sym: u32, ret: u32) {
    let at = t.rebase(sym);
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(sym.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // [eax+8] = a
    c.extend([0x56, 0x8b, 0x74, 0x24, 0x0c]); // push esi; mov esi, [esp+0xc]
    for k in 0..4u8 {
        c.extend([0x8b, 0x56, 4 * k, 0x89, 0x50, 0xc + 4 * k]); // [eax+0xc+4k] = record[k]
    }
    c.push(0x5e); // pop esi
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xb8);
    c.extend(ret.to_le_bytes()); // mov eax, ret
    c.extend([0xc2, 0x08, 0x00]); // ret 8
    let addr = t.cave_next;
    assert!(addr + c.len() as u32 <= t.cave + CAVE_LEN, "cave full");
    t.write(addr, &c);
    t.cave_next = addr + c.len() as u32;
    let mut j = vec![0xe9];
    j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
    t.write(at, &j);
}

/// `TextureAnimationManager_updateSamplers_006b44a0` on a random manager:
/// animations named from a few names, frames in and out of range, slots
/// of -1, textures past the count, parameter slots typed 2 or not.
/// Returns how many records were set.
fn round23c_case(t: &mut Twins, rng: &mut Rng, what: &str) -> u32 {
    use managers::bridge as mb;
    let names: Vec<u32> = ["texA", "texB", "anim1", "anim10"]
        .iter()
        .map(|s| cstr(t, s))
        .collect();
    let this = t.alloc(&rng.bytes(0x90));
    t.w32(this + 4, u32::from(!rng.chance(10)));
    // the textures
    let ntex = rng.below(6);
    let textures: Vec<u8> = (0..ntex + 2)
        .flat_map(|_| if rng.chance(4) { 0 } else { rng.next() | 1 }.to_le_bytes())
        .collect();
    let tex = t.alloc(&textures);
    t.w32(
        this + 0x5c,
        ntex | if rng.chance(2) { 0x8000_0000 } else { 0 },
    );
    t.w32(this + 0x60, tex);
    t.w32(this + 0xc, rng.below(5));
    t.w32(this + 0x10, rng.below(4));
    // the animations
    let nanim = rng.below(4);
    let anims = t.alloc(&rng.bytes(0x20 * nanim as usize + 4));
    for k in 0..nanim {
        let a = anims + 0x20 * k;
        let name = names[rng.below(4) as usize];
        t.w32(a + 4, name | u32::from(rng.chance(3)));
        let nslots = rng.below(4);
        let slots: Vec<u8> = (0..=nslots)
            .flat_map(|_| {
                let v = if rng.chance(4) {
                    u32::MAX
                } else {
                    rng.below(ntex + 3)
                };
                v.to_le_bytes()
            })
            .collect();
        let s = t.alloc(&slots);
        t.w32(a + 8, nslots | if rng.chance(3) { 0x8000_0000 } else { 0 });
        t.w32(a + 0xc, s);
        let frame = [u32::MAX, 0, 1, rng.below(nslots + 2)][rng.below(4) as usize];
        t.w32(a + 0x18, frame);
    }
    t.w32(this + 0x7c, nanim);
    t.w32(this + 0x80, anims);
    let offset = [0u32, 2, 4, 8, 0x10][rng.below(5) as usize];
    // the sampler parameter, its offset and size
    let param = if rng.chance(6) {
        0
    } else {
        let p = t.alloc(&rng.bytes(0x10));
        let size = [4u16, 8, 0x10, 0x30, 0xe010][rng.below(5) as usize];
        t.write(p + 8, &(offset as u16).to_le_bytes());
        t.write(p + 0xa, &size.to_le_bytes());
        p
    };
    // the materials
    let nmat = rng.below(6);
    let mut mats = Vec::new();
    for _ in 0..nmat {
        if rng.chance(6) {
            mats.extend(0u32.to_le_bytes());
            continue;
        }
        let mat = t.alloc(&rng.bytes(0x20));
        t.w32(mat, rng.next());
        let buf = t.alloc(&rng.bytes(0x40));
        let value = t.alloc(&rng.bytes(8));
        t.w8(value + 2, [2, 2, 1, 3][rng.below(4) as usize]);
        t.w32(buf + offset + 8, if rng.chance(5) { 0 } else { value });
        t.w32(buf, [0x10, 0x20, 0x40, rng.next()][rng.below(4) as usize]);
        t.w32(mat + 4, buf);
        let name = names[rng.below(4) as usize];
        t.w32(mat + 0x10, name | u32::from(rng.chance(3)));
        mats.extend(mat.to_le_bytes());
    }
    let list = t.alloc(&mats);
    t.w32(
        this + 0x74,
        nmat | if rng.chance(3) { 0x8000_0000 } else { 0 },
    );
    t.w32(this + 0x78, list);
    t.stub_sym(mb::SHADER_PARAMS_FIND, Conv::Thiscall(0), 1, param);
    stub_record4(t, mb::SET_PARAM_VALUE, rng.next());
    t.check(what, 0x006b_44a0, Conv::Fastcall, &[this], |g| {
        mb::update_samplers(g, this);
        Out::None
    });
    t.r32(t.log)
}

#[test]
fn round23c_matches_the_original() {
    let Some(mut t) = Twins::image() else { return };
    let mut rng = Rng(0x2323_c023);
    let mut set = 0;
    for round in 0..3000 {
        t.reset();
        t.a.reset_heap();
        t.b.reset_heap();
        t.log = t.zeros(0x8000);
        let what = format!("round {round}");
        set += round23c_case(&mut t, &mut rng, &what);
    }
    eprintln!("stub calls: {set}");
    assert!(set > 150, "stub calls: {set}");
}

#[test]
fn snapshot_round23c_matches_the_original() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x5323_c023);
    for round in 0..300 {
        t.reset();
        let what = format!("snapshot round {round}");
        round23c_case(&mut t, &mut rng, &what);
    }
}
