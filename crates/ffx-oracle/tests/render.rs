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
const CAVE_LEN: u32 = 1800;

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
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = bridge(&mut G(&mut self.b));
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
                    st0.to_bits() == ours.to_bits() || (st0.is_nan() && ours.is_nan()),
                    "{what}: st0 {st0} vs {ours}"
                );
            }
        }
        self.diff(what);
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
            "PhyreScene_getRenderTarget",
            0x0065_e050,
            Kind::T1,
            |_, s, a, _| Out::U32(sc::render_target(s, a)),
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
            "PostProcessManager_SetByte123ba",
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
            "PostProcessManager_SetField123c0",
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
