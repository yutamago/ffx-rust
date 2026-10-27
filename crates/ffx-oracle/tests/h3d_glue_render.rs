//! `PhyreScene::fieldMapInitialize` (M4 port, list `h3-glue-render`,
//! agent "h3d") against the original: on the image alone with a synthetic
//! scene, and on the snapshots (`FFX_SNAPSHOT`) with the game's scene.
//! The result, `ST0`, every byte written (the callees' logs included) and
//! the frame the callees see must agree. Needs `FFX_GAME_DIR`.
//!
//! The callees with side effects are recording stubs in code of the
//! test's own (`Oracle::alloc_code`), placed once per oracle pair; each
//! returns the next word of its own table, which a case rewrites. On a
//! snapshot the lookups (cursors, the tree, the shader parameters, the
//! shadow manager's getters, `sscanf`) run as the original.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::h3d_fmi::{self as fmi, bridge};

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
    fn pick(&mut self, vs: &[u32]) -> u32 {
        vs[self.below(vs.len() as u32) as usize]
    }
    /// A float that is not a signalling NaN (the emulator keeps those).
    fn float(&mut self) -> u32 {
        loop {
            let bits = match self.below(5) {
                0 => self.next(),
                1 => (self.below(2000) as f32 / 1000.0).to_bits(),
                2 => self.pick(&[
                    0,
                    0x8000_0000,
                    0x3f80_0000,
                    0x3f00_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    0xffc0_1234,
                    1,
                    0x3f7f_ffff,
                ]),
                _ => (self.below(4000) as f32 / 1000.0 - 2.0).to_bits(),
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return bits;
            }
        }
    }
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// The bridge's view of the port's oracle.
struct G<'a> {
    o: &'a mut Oracle,
    /// `fstp st(0); ret`, to empty the x87 stack after a float callee.
    pop: u32,
}

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.o.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.o, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.o.image_base()
    }
}

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(c, false));
        self.o
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("port's call {addr:#x}: {e}"))
            .eax
    }
    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        ffx_oracle::check_width("Host::call_st0", addr, args.len(), host::max_words(c, true));
        let r = self.o.call(addr, conv(c), args).unwrap();
        let v = r
            .st0_f32()
            .unwrap_or_else(|| panic!("{addr:#x}: no float in ST0"));
        self.o.call(self.pop, Conv::Cdecl, &[]).unwrap();
        v
    }
    fn scratch(&mut self) -> u32 {
        self.o.stack_range().start + 0x100
    }
}

// ---- stubs ----

/// What a stub does besides logging and returning its next word.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Extra {
    None,
    /// Fills the cursor at `ecx` (five words) from its table.
    Cursor,
    /// `sscanf`: two words of its table into the third and fourth
    /// arguments' targets.
    Scan,
    /// `fld dword` of the next float of its table.
    Float,
    /// The next word of its table into `[ecx + off]`.
    StoreThis(u32),
    /// Sixteen words of its table into the first argument's target.
    Matrix,
}

/// A stubbed callee: address (ffx.sym), `this` logged, stack words
/// logged, words popped, extra.
#[derive(Clone, Copy)]
struct Callee {
    sym: u32,
    this: bool,
    n: u32,
    pops: u32,
    extra: Extra,
    /// Left original on a snapshot.
    lookup: bool,
}

const fn this(sym: u32, n: u32) -> Callee {
    Callee {
        sym,
        this: true,
        n,
        pops: n,
        extra: Extra::None,
        lookup: false,
    }
}

const fn cdecl(sym: u32, n: u32) -> Callee {
    Callee {
        sym,
        this: false,
        n,
        pops: 0,
        extra: Extra::None,
        lookup: false,
    }
}

const fn lookup(c: Callee) -> Callee {
    Callee { lookup: true, ..c }
}

const fn with(c: Callee, extra: Extra) -> Callee {
    Callee { extra, ..c }
}

/// The import slot `sscanf` is called through: the stub's address goes
/// there.
const SSCANF: Callee = lookup(with(cdecl(fmi::SSCANF_SLOT, 4), Extra::Scan));

const CALLEES: &[Callee] = &[
    this(fmi::DELEGATION_RESET, 1),
    this(fmi::ARRAY_ASSIGN, 1),
    this(fmi::LIST_COPY, 1),
    cdecl(fmi::ON_SCENE_LOADED, 14),
    this(fmi::REBIND, 2),
    this(fmi::ARRAY_RESERVE, 1),
    lookup(with(this(fmi::CURSOR_BEGIN, 0), Extra::Cursor)),
    lookup(with(this(fmi::CURSOR_NEXT, 0), Extra::Cursor)),
    cdecl(fmi::USER_MALLOC, 1),
    cdecl(fmi::USER_FREE, 1),
    SSCANF,
    this(fmi::GET_SYMBOL_ID, 1),
    lookup(cdecl(fmi::TEX_ANIM_INSTANCE, 0)),
    this(fmi::ADD_ANIMATED_MATERIALS, 7),
    this(fmi::SCENE_SETUP[0], 0),
    with(this(fmi::SCENE_SETUP[1], 0), Extra::StoreThis(0x4f8)),
    this(fmi::SCENE_SETUP[2], 0),
    this(fmi::SCENE_SETUP[3], 0),
    this(fmi::SCENE_SETUP[4], 0),
    lookup(cdecl(fmi::MORPH_INSTANCE, 0)),
    this(fmi::REGISTER_MORPHS, 1),
    cdecl(fmi::OPERATOR_NEW, 1),
    this(fmi::REGISTER_DELEGATION, 4),
    cdecl(fmi::OPERATOR_DELETE, 1),
    this(fmi::UPDATE_MAP_YIQ, 0),
    this(fmi::SETUP_CHARACTER_LIGHTING, 1),
    this(fmi::SETUP_MAP_FOG, 0),
    lookup(this(fmi::TREE_SUCCESSOR, 1)),
    this(fmi::CHANGE_2D_SCENE_NUM, 1),
    cdecl(fmi::GET_SCENERIO_FLAG, 0),
    lookup(cdecl(fmi::SHADOW_INSTANCE, 0)),
    this(fmi::SELECT_SHADOW_SOLUTION, 6),
    this(fmi::REMOVE_CLUSTER, 1),
    this(fmi::ADD_CLUSTER, 1),
    cdecl(fmi::PHYRE_FREE, 1),
    lookup(with(this(fmi::AVERAGE_CASTER_VALUE, 0), Extra::Float)),
    lookup(with(this(fmi::CURSOR_SEEK4, 0), Extra::Cursor)),
    lookup(this(fmi::FIND_PARAM, 1)),
    lookup(this(fmi::DUMMY_SHADOW_MAP, 0)),
    lookup(this(fmi::GET_CASCADE, 1)),
    lookup(this(fmi::GET_LINEAR_SAMPLER, 1)),
    lookup(this(fmi::TROPHY_INSTALL, 1)),
    lookup(cdecl(fmi::GET_SHADOW_SIZE, 0)),
    lookup(with(this(fmi::GET_CASCADE_MATRIX, 2), Extra::Matrix)),
    cdecl(fmi::PHYRE_MALLOC, 2),
    this(fmi::PUSH_INTO_FIRST, 2),
    this(fmi::PUSH_INTO, 2),
    lookup(cdecl(fmi::INSTANCE_6DC800, 0)),
    this(fmi::ON_MAP_NAME, 1),
];

/// Words per table.
const TABLE: u32 = 0x100;

struct Stub {
    /// The cell holding the return table's cursor, and the table.
    cell: u32,
    table: u32,
    /// The extra's cell and table.
    xcell: u32,
    xtable: u32,
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words.
    log: u32,
    pop: u32,
    stubs: BTreeMap<u32, Stub>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let pop = a.alloc_code(&[0xdd, 0xd8, 0xc3]).unwrap();
        assert_eq!(b.alloc_code(&[0xdd, 0xd8, 0xc3]).unwrap(), pop);
        let mut t = Self {
            a,
            b,
            log: 0,
            pop,
            stubs: BTreeMap::new(),
        };
        t.log = t.zeros(0x1_0000);
        for c in CALLEES {
            if snapshot && c.lookup {
                continue;
            }
            t.place(c);
        }
        t
    }

    fn image() -> Option<Self> {
        let exe = game_dir()?.join("FFX.exe");
        Some(Self::new(
            Oracle::load(&exe).unwrap(),
            Oracle::load(&exe).unwrap(),
            false,
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
            true,
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

    fn code(&mut self, code: &[u8]) -> u32 {
        let x = self.a.alloc_code(code).unwrap();
        assert_eq!(self.b.alloc_code(code).unwrap(), x);
        x
    }

    /// Writes the stub of `c` and jumps to it from the original (or
    /// points the import slot at it).
    fn place(&mut self, c: &Callee) {
        let cell = self.zeros(4);
        let table = self.zeros(4 * TABLE as usize);
        let xcell = self.zeros(4);
        let xtable = self.zeros(4 * 4 * TABLE as usize);
        self.w32(cell, table);
        self.w32(xcell, xtable);
        let log = self.log;
        let mut k = vec![0xa1];
        k.extend(log.to_le_bytes()); // mov eax, [log]
        k.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        k.push(0x05);
        k.extend((log + 4).to_le_bytes()); // add eax, log + 4
        k.extend([0xc7, 0x00]);
        k.extend(c.sym.to_le_bytes()); // mov [eax], tag
        if c.this {
            k.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..c.n {
            k.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            k.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        k.extend([0xff, 0x05]);
        k.extend(log.to_le_bytes()); // inc dword [log]
        let x = xcell.to_le_bytes();
        match c.extra {
            Extra::None => {}
            Extra::Cursor | Extra::Matrix => {
                k.extend([0x56, 0x57, 0x8b, 0x35]); // push esi; push edi; mov esi, [xcell]
                k.extend(x);
                if c.extra == Extra::Cursor {
                    k.extend([0x89, 0xcf, 0xb9, 5, 0, 0, 0]); // mov edi, ecx; mov ecx, 5
                } else {
                    k.extend([0x8b, 0x7c, 0x24, 0x0c, 0xb9, 16, 0, 0, 0]); // mov edi, [esp+0xc]; mov ecx, 16
                }
                k.extend([0xf3, 0xa5, 0x89, 0x35]); // rep movsd; mov [xcell], esi
                k.extend(x);
                k.extend([0x5f, 0x5e]); // pop edi; pop esi
            }
            Extra::Scan => {
                k.extend([0x8b, 0x15]);
                k.extend(x); // mov edx, [xcell]
                k.extend([0x8b, 0x0a, 0x8b, 0x44, 0x24, 0x0c, 0x89, 0x08]); // mov ecx,[edx]; mov eax,[esp+0xc]; mov [eax],ecx
                k.extend([0x8b, 0x4a, 0x04, 0x8b, 0x44, 0x24, 0x10, 0x89, 0x08]); // mov ecx,[edx+4]; mov eax,[esp+0x10]; mov [eax],ecx
                k.extend([0x83, 0xc2, 0x08, 0x89, 0x15]);
                k.extend(x); // add edx, 8; mov [xcell], edx
            }
            Extra::Float => {
                k.extend([0x8b, 0x15]);
                k.extend(x); // mov edx, [xcell]
                k.extend([0xd9, 0x02, 0x83, 0xc2, 0x04, 0x89, 0x15]); // fld dword [edx]; add edx, 4; mov [xcell], edx
                k.extend(x);
            }
            Extra::StoreThis(off) => {
                k.extend([0x8b, 0x15]);
                k.extend(x); // mov edx, [xcell]
                k.extend([0x8b, 0x02, 0x83, 0xc2, 0x04, 0x89, 0x15]); // mov eax, [edx]; add edx, 4; mov [xcell], edx
                k.extend(x);
                k.extend([0x89, 0x81]);
                k.extend(off.to_le_bytes()); // mov [ecx+off], eax
            }
        }
        k.extend([0x8b, 0x15]);
        k.extend(cell.to_le_bytes()); // mov edx, [cell]
        k.extend([0x8b, 0x02, 0x83, 0xc2, 0x04, 0x89, 0x15]); // mov eax, [edx]; add edx, 4; mov [cell], edx
        k.extend(cell.to_le_bytes());
        if c.pops == 0 {
            k.push(0xc3);
        } else {
            k.push(0xc2);
            k.extend(((c.pops * 4) as u16).to_le_bytes());
        }
        let addr = self.code(&k);
        let site = self.rebase(c.sym);
        if c.sym == fmi::SSCANF_SLOT {
            self.w32(site, addr);
        } else {
            let mut j = vec![0xe9];
            j.extend(addr.wrapping_sub(site + 5).to_le_bytes());
            self.write(site, &j);
        }
        self.stubs.insert(
            c.sym,
            Stub {
                cell,
                table,
                xcell,
                xtable,
            },
        );
    }

    /// Sets a stub's return words (and resets its cursor).
    fn returns(&mut self, sym: u32, words: &[u32]) {
        let Some(s) = self.stubs.get(&sym) else {
            return;
        };
        let (cell, table) = (s.cell, s.table);
        assert!(words.len() <= TABLE as usize);
        let mut bytes = vec![0u8; 4 * TABLE as usize];
        for (i, w) in words.iter().enumerate() {
            bytes[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        self.write(table, &bytes);
        self.w32(cell, table);
    }

    /// Sets a stub's extra words.
    fn extras(&mut self, sym: u32, words: &[u32]) {
        let Some(s) = self.stubs.get(&sym) else {
            return;
        };
        let (cell, table) = (s.xcell, s.xtable);
        assert!(words.len() <= 4 * TABLE as usize);
        let mut bytes = vec![0u8; 16 * TABLE as usize];
        for (i, w) in words.iter().enumerate() {
            bytes[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        self.write(table, &bytes);
        self.w32(cell, table);
    }

    fn dump_log(&self) {
        for (side, o) in [("a", &self.a), ("b", &self.b)] {
            let n: u32 = Mem::read(o, self.log);
            for k in 0..n.min(200) {
                let e: Vec<u32> = (0..8)
                    .map(|w| Mem::read::<u32>(o, self.log + 4 + 64 * k + 4 * w))
                    .collect();
                eprintln!("{side} {k}: {e:x?}");
            }
        }
    }

    /// Runs the original and the port on `scene` with `rebuild`; compares
    /// `eax`, `ST0`, memory and the frame.
    fn check(&mut self, what: &str, scene: u32, rebuild: u32, rng: &mut Rng) {
        self.w32(self.log, 0);
        // the original's ebp: the argument, the return address and the
        // saved ebp below where the call pushes
        let ebp = self.a.stack_range().end - 0x100 - 4 - 8;
        let lo = ebp - fmi::FRAME_LOW;
        let fill: Vec<u8> = (0..fmi::FRAME_LOW + 0x10)
            .map(|_| if rng.chance(2) { 0 } else { rng.next() as u8 })
            .collect();
        self.write(lo, &fill);
        self.b.set_call_top(Some(lo - 0x100));
        let addr = self.rebase(0x0065_b8a0);
        let ra = self.a.call(addr, Conv::Thiscall(scene), &[rebuild]);
        let ra = ra.unwrap_or_else(|e| {
            self.dump_log();
            panic!("{what}: the original: {e}")
        });
        let on_scene_loaded = self.rebase(fmi::ON_SCENE_LOADED);
        let mut g = G {
            o: &mut self.b,
            pop: self.pop,
        };
        let eax = bridge::field_map_initialize(&mut g, ebp, scene, rebuild, |m, w| {
            m.o.call(on_scene_loaded, Conv::Cdecl, w).unwrap();
        });
        self.b.set_call_top(None);
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        assert_eq!(ra.eax, eax, "{what}: eax");
        let d = self.a.diff(&mut self.b).unwrap();
        if !d.is_empty() {
            self.dump_log();
        }
        assert_eq!(d, [], "{what}: memory");
        // the frame: the locals (from the lowest, ebp - 0x178, to the
        // exception record at ebp - 0x10) and the argument word; the x87
        // and pointer temporaries no callee sees are left out
        let ignore = [(0x20u32, 0xcu32), (0x60, 8)];
        let low = 0x178u32;
        for (from, n) in [(ebp - low, low - 0x10), (ebp + 8, 4)] {
            let mut x = vec![0; n as usize];
            let mut y = vec![0; n as usize];
            Mem::read_bytes(&self.a, from, &mut x);
            Mem::read_bytes(&self.b, from, &mut y);
            if from < ebp {
                for &(off, len) in &ignore {
                    let s = (low - off) as usize;
                    y[s..s + len as usize].copy_from_slice(&x[s..s + len as usize]);
                }
            }
            if let Some(i) = x.iter().zip(&y).position(|(p, q)| p != q) {
                let at = from + i as u32;
                let end = (i + 16).min(x.len());
                panic!(
                    "{what}: frame differs at ebp - {:#x}: {:?} vs {:?}",
                    ebp.wrapping_sub(at),
                    &x[i..end],
                    &y[i..end]
                );
            }
        }
        self.a.clear_dirty();
        self.b.clear_dirty();
    }
}

// ---- the synthetic scene ----

const SCENE_LEN: usize = 0x1_0e00;

/// A parameter descriptor: type, offset, size (sizes the frame blocks
/// hold, or one the check refuses).
fn param(t: &mut Twins, rng: &mut Rng) -> u32 {
    let p = t.zeros(0x10);
    let kind = if rng.chance(3) {
        rng.next() as u8
    } else {
        fmi::SAMPLER_TYPES[rng.below(6) as usize]
    };
    t.w8(p + 2, kind);
    let off = rng.pick(&[0, 2, 3, 4, 8, 0x20, 0x40, 0x80, 0xf0]);
    let len = rng.pick(&[0, 4, 8, 0x10, 0x20, 0x2010, 0xe010]);
    t.write(p + 8, &(off as u16).to_le_bytes());
    t.write(p + 0xa, &(len as u16).to_le_bytes());
    p
}

/// A parameter buffer: its size word, room for any parameter.
fn buffer(t: &mut Twins, rng: &mut Rng) -> u32 {
    let b = t.zeros(0x200);
    let size = rng.pick(&[0, 4, 0x10, 0x30, 0x60, 0x100]);
    t.w32(b, size);
    b
}

/// A list head at `at` (`+0x1c` of a cluster): empty (pointing at
/// itself) or holding a node.
fn list_head(t: &mut Twins, at: u32, rng: &mut Rng) {
    let node = if rng.chance(3) { at } else { t.zeros(0x80) };
    t.w32(at, node);
}

/// A cursor run: position, block end, stride, count, offset, over
/// `objects` (`stride`-byte records).
fn cursor_run(rng: &mut Rng, objects: u32, stride: u32, max: u32) -> [u32; 5] {
    let count = if rng.chance(4) { 0 } else { 1 + rng.below(max) };
    let start = rng.below(2) * stride;
    let pos = objects + start;
    // the block ends at one of the positions now and then; its first
    // word (the next block) is whatever the record holds
    let end = if rng.chance(2) {
        pos + stride * (1 + rng.below(count.max(1)))
    } else {
        rng.next() | 1
    };
    [pos, end, stride, count, 0]
}

/// One synthetic scene and the stubs' tables for it.
fn prep_image(t: &mut Twins, rng: &mut Rng) -> (u32, u32) {
    let scene = t.zeros(SCENE_LEN);
    let main = if rng.chance(5) { 0 } else { t.zeros(0x100) };
    let two_d = if rng.chance(4) { 0 } else { t.zeros(0x100) };
    for c in [main, two_d] {
        if c != 0 {
            list_head(t, c + 0x1c, rng);
        }
    }
    t.w32(scene + 0x10d4, main);
    t.w32(scene + 0x1128, two_d);
    t.w8(scene + 0x10d0, rng.pick(&[0, 1, 1, 2]) as u8);
    t.w8(scene + 0x10d1, rng.pick(&[0, 1, 1, 2]) as u8);
    t.w8(scene + 0x1195, rng.pick(&[0, 0, 1]) as u8);
    let rebuild = rng.pick(&[0, 1, 0x100, 0xffff_ff00]);
    t.w32(scene + 0xfaf0, rng.next());
    t.w32(scene + 0xfaf4, rng.pick(&[0xec1, 0xc58, 0x17, 0, 0x123]));
    t.w32(scene + 0xd0, rng.next());
    t.w8(scene + 0x93c, rng.next() as u8);
    t.w8(scene + 0x93d, rng.pick(&[0, 7]) as u8);
    t.w8(scene + 0x1196, rng.next() as u8);
    t.w32(scene + 0x11d8, rng.next());
    t.w32(scene + 0xfb14, rng.next());
    t.w8(scene + 0xf9d0, rng.below(2) as u8);
    t.w8(scene + 0xf9d1, rng.below(2) as u8);
    let map = rng.pick(&[0x17, 0x17, 0, 0x18]);
    let g = t.rebase(fmi::CURRENT_MAP_NO);
    t.w32(g, map);
    let g = t.rebase(fmi::GRAPHIC_MAP_ID);
    let v = rng.next();
    t.w32(g, v);
    let helper = t.zeros(0x40);
    let g = t.rebase(fmi::RENDER_DELEGATION_HELPER);
    t.w32(g, helper);

    // the asset arrays: data words, 0 now and then
    for (count, data) in [(0x10d8, 0x10e0), (0x112c, 0x1134)] {
        let n = rng.below(4);
        let d = t.zeros(0x40);
        for i in 0..n {
            let v = if rng.chance(3) { 0 } else { t.zeros(0x80) };
            t.w32(d + 4 * i, v);
        }
        t.w32(scene + count, n);
        t.w32(scene + data, d);
    }
    // the clusters of +0x112c get list heads
    let n: u32 = t.r32(scene + 0x112c);
    let list: u32 = t.r32(scene + 0x1134);
    for i in 0..n {
        let c: u32 = t.r32(list + 4 * i);
        if c != 0 {
            list_head(t, c + 0x1c, rng);
        }
    }
    // the layer shapes
    let (count, cap) = match rng.below(4) {
        0 => (0xffff_fff0, 0xffff_fff0),
        1 => (5, 0),
        2 => (0, 2),
        _ => (rng.next(), 0xffff_ffff),
    };
    let data = if cap as i32 > 0 || rng.chance(2) {
        t.zeros(0x400)
    } else {
        0
    };
    t.w32(scene + 0x1138, count);
    t.w32(scene + 0x113c, cap);
    t.w32(scene + 0x1140, data);
    // the cluster object +0x10cc: a texture handle at +0x8c
    let obj = t.zeros(0x100);
    let handle = t.zeros(8);
    t.w32(handle, rng.next());
    t.w8(handle + 4, rng.pick(&[0, 0, 1, 0x40]) as u8);
    t.w32(obj + 0x8c, handle);
    t.w32(obj + 0xa4, rng.next());
    t.w32(obj + 0x9c, rng.next());
    t.w32(scene + 0x10cc, obj);
    let view = t.zeros(0x40);
    t.w32(scene + 0xcc, view);
    for k in 0..12 {
        let f = rng.float();
        t.w32(view + 4 * k, f);
    }
    // the shadow-parameter arrays
    for off in [0x38a4u32, 0x38ac, 0x38b4, 0x38bc, 0x38c4] {
        let c = rng.pick(&[0, 3, 0x8000_0000, 0x8000_0002]);
        t.w32(scene + off, c);
        t.w32(scene + off + 4, rng.next());
    }

    // the layer tree: the leftmost chain from +0x1198 + 0xc, then the
    // successor's answers
    let head = scene + 0x1198;
    let nodes: Vec<u32> = (0..rng.below(4)).map(|_| t.zeros(0x20)).collect();
    if nodes.is_empty() {
        t.w32(head + 0xc, head);
    } else {
        // [head + 0xc] -> nodes[0] -> ... -> head through +0
        t.w32(head + 0xc, nodes[0]);
        let chain = 1 + rng.below(nodes.len() as u32) as usize;
        for i in 0..chain {
            let next = if i + 1 < chain { nodes[i + 1] } else { head };
            t.w32(nodes[i], next);
        }
        for &node in &nodes {
            let arr = t.zeros(0x10);
            let n = rng.below(4);
            let d = t.zeros(0x10);
            for i in 0..n {
                let o = t.zeros(0x50);
                t.w32(o + 0x48, rng.next());
                let rec = if rng.chance(3) { 0 } else { t.zeros(0x20) };
                t.w32(o + 0x1c, rec);
                t.w32(d + 4 * i, o);
            }
            t.w32(arr, n);
            t.w32(arr + 8, d);
            t.w32(node + 0x10, arr);
        }
    }
    let mut succ: Vec<u32> = nodes.iter().copied().rev().collect();
    succ.push(head);
    t.returns(fmi::TREE_SUCCESSOR, &succ);

    // layer objects and their cursor runs
    let objects = t.zeros(0x60 * 8);
    for i in 0..8 {
        let o = objects + 0x60 * i;
        let f = rng.float();
        t.w32(o + 0x5c, f);
        t.w32(o + 0x2c, rng.next());
    }
    let mut runs = vec![];
    for _ in 0..12 {
        runs.extend(cursor_run(rng, objects, 0x60, 3));
    }
    t.extras(fmi::CURSOR_BEGIN, &runs);
    let mut runs = vec![];
    for _ in 0..24 {
        runs.extend(cursor_run(rng, objects, 0x60, 3));
    }
    t.extras(fmi::CURSOR_NEXT, &runs);
    // Begin's and Next's runs share one table each; Next stops at an empty
    // run after a few
    let scans: Vec<u32> = (0..64)
        .map(|_| rng.pick(&[2, 2, 2, 1, 0, 0xffff_ffff]))
        .collect();
    t.returns(fmi::SSCANF_SLOT, &scans);
    let outs: Vec<u32> = (0..128).map(|_| rng.word_small()).collect();
    t.extras(fmi::SSCANF_SLOT, &outs);
    let bufs: Vec<u32> = (0..12).map(|_| t.zeros(0x400)).collect();
    t.returns(fmi::USER_MALLOC, &bufs);

    // set-up callees
    let ids: Vec<u32> = (0..2).map(|_| rng.pick(&[u32::MAX, 0, 7])).collect();
    t.returns(fmi::GET_SYMBOL_ID, &ids);
    let cam = if rng.chance(3) { 0 } else { t.zeros(0x100) };
    if cam != 0 {
        for k in 0..3 {
            let f = rng.float();
            t.w32(cam + 0xf4 + 4 * k, f);
        }
    }
    t.extras(fmi::SCENE_SETUP[1], &[cam]);
    let rec = t.zeros(0x20);
    t.returns(fmi::OPERATOR_NEW, &[rec]);
    let manager = t.zeros(0x4100);
    for g in [
        fmi::TEX_ANIM_INSTANCE,
        fmi::MORPH_INSTANCE,
        fmi::SHADOW_INSTANCE,
        fmi::INSTANCE_6DC800,
    ] {
        t.returns(g, &[manager; 64]);
    }
    let flag = rng.next();
    t.returns(fmi::GET_SCENERIO_FLAG, &[flag]);
    let solution = rng.pick(&[0, 1, 2, 3, 4, 5, 4, 1]);
    t.returns(fmi::SELECT_SHADOW_SOLUTION, &[solution]);

    // the shadow parameters: materials (0x10-byte records: parameters,
    // buffer) and their cursor
    let materials = t.zeros(0x10 * 16);
    for i in 0..16 {
        let m = materials + 0x10 * i;
        t.w32(m, rng.next());
        let b = buffer(t, rng);
        t.w32(m + 4, b);
    }
    let run = cursor_run(rng, materials, 0x10, 4);
    t.extras(fmi::CURSOR_SEEK4, &run);
    if main != 0 {
        material_nodes(t, main + 0x1c, materials, rng);
    }
    let params: Vec<u32> = (0..64)
        .map(|_| if rng.chance(4) { 0 } else { param(t, rng) })
        .collect();
    t.returns(fmi::FIND_PARAM, &params);
    let handles: Vec<u32> = (0..32)
        .map(|_| {
            let h = t.zeros(8);
            t.w32(h, if rng.chance(3) { 0 } else { rng.next() });
            t.w8(h + 4, rng.pick(&[0, 0, 1, 2, 0x80]) as u8);
            h
        })
        .collect();
    t.returns(fmi::DUMMY_SHADOW_MAP, &handles);
    t.returns(fmi::GET_CASCADE, &handles);
    let samplers: Vec<u32> = (0..32)
        .map(|_| if rng.chance(3) { 0 } else { rng.next() })
        .collect();
    t.returns(fmi::GET_LINEAR_SAMPLER, &samplers);
    let holds: Vec<u32> = (0..32).map(|_| rng.pick(&[0, 0, 1, 0x100])).collect();
    t.returns(fmi::TROPHY_INSTALL, &holds);
    let sizes: Vec<u32> = (0..128)
        .map(|_| rng.pick(&[1024, 2048, 4096, 512, 64, 0, 0xffff_ffc0, 0x7fff_ffff, 3]))
        .collect();
    t.returns(fmi::GET_SHADOW_SIZE, &sizes);
    let floats: Vec<u32> = (0..4).map(|_| rng.float()).collect();
    t.extras(fmi::AVERAGE_CASTER_VALUE, &floats);
    let matrix: Vec<u32> = (0..16 * 32).map(|_| rng.float()).collect();
    t.extras(fmi::GET_CASCADE_MATRIX, &matrix);
    let mems: Vec<u32> = (0..64).map(|_| rng.next()).collect();
    t.returns(fmi::PHYRE_MALLOC, &mems);
    // the rest return words the port ignores (or passes on)
    for sym in [
        fmi::DELEGATION_RESET,
        fmi::ARRAY_ASSIGN,
        fmi::LIST_COPY,
        fmi::ON_SCENE_LOADED,
        fmi::REBIND,
        fmi::REGISTER_MORPHS,
        fmi::ON_MAP_NAME,
    ] {
        let w: Vec<u32> = (0..16).map(|_| rng.next()).collect();
        t.returns(sym, &w);
    }
    (scene, rebuild)
}

impl Rng {
    /// A small int, or any now and then (the `%d` results).
    fn word_small(&mut self) -> u32 {
        if self.chance(5) {
            self.next()
        } else {
            self.below(40)
        }
    }
}

/// The material cursor's node list at `head`: nodes after the first
/// (which the cursor stub stands for), some with the class in their
/// chain, some without, some empty.
fn material_nodes(t: &mut Twins, head: u32, materials: u32, rng: &mut Rng) {
    let first: u32 = t.r32(head);
    if first == head {
        return;
    }
    let class = t.rebase(fmi::CLASS_MATERIAL);
    let mut node = first;
    for _ in 0..rng.below(4) {
        let next = t.zeros(0x50);
        t.w32(node, next);
        let entry = t.zeros(0x90);
        let stride = rng.pick(&[0x10, 0x20]);
        t.w32(entry + 0x1c, stride);
        let offset = rng.pick(&[0, 0x10]);
        t.w32(entry + 0x8c, offset);
        let other = t.zeros(0x50);
        let chain = match rng.below(4) {
            0 => 0,
            1 => other,
            _ => class,
        };
        t.w32(entry + 0x40, chain);
        t.w32(next + 0x2c, if rng.chance(6) { 0 } else { entry });
        t.w32(next + 0x40, rng.pick(&[0, 1, 2]));
        let count = rng.pick(&[0, 1, 2, 3]);
        t.w32(next + 0x3c, count);
        let pos = materials + 0x10 * rng.below(3);
        t.w32(next + 0x34, pos);
        let end = if rng.chance(3) { pos } else { rng.next() | 1 };
        t.w32(next + 8, end);
        node = next;
    }
    t.w32(node, head);
}

#[test]
fn field_map_initialize_matches_the_original_on_the_image() {
    let Some(mut t) = Twins::image() else {
        return;
    };
    let mut rng = Rng(0x1234_5678);
    let n = std::env::var("H3D_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    for i in 0..n {
        let (scene, rebuild) = prep_image(&mut t, &mut rng);
        t.check(&format!("image #{i}"), scene, rebuild, &mut rng);
    }
}

#[test]
fn field_map_initialize_matches_the_original_on_the_snapshot() {
    let Some(mut t) = Twins::snapshot() else {
        return;
    };
    let mut rng = Rng(0x9abc_def1);
    let g = t.rebase(0x00cc_b9d8);
    let scene = t.r32(g);
    assert_ne!(scene, 0, "no scene in the snapshot");
    for i in 0..6 {
        let bufs: Vec<u32> = (0..12).map(|_| t.zeros(0x400)).collect();
        t.returns(fmi::USER_MALLOC, &bufs);
        let rec = t.zeros(0x20);
        t.returns(fmi::OPERATOR_NEW, &[rec]);
        let solution = [0, 1, 2, 3, 4, 5][i % 6];
        t.returns(fmi::SELECT_SHADOW_SOLUTION, &[solution]);
        let mems: Vec<u32> = (0..256).map(|_| rng.next()).collect();
        t.returns(fmi::PHYRE_MALLOC, &mems);
        let ids: Vec<u32> = (0..2).map(|_| rng.pick(&[u32::MAX, 3])).collect();
        t.returns(fmi::GET_SYMBOL_ID, &ids);
        let cam = if i % 2 == 0 { 0 } else { t.zeros(0x100) };
        t.extras(fmi::SCENE_SETUP[1], &[cam]);
        let flag = rng.next();
        t.returns(fmi::GET_SCENERIO_FLAG, &[flag]);
        let rebuild = u32::from(i % 3 == 2);
        t.check(&format!("snapshot #{i}"), scene, rebuild, &mut rng);
    }
}
