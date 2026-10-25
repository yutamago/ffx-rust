//! `ffx_pother_b` (M4 port list `p4-other-b`) against the original: on
//! the image alone with synthetic objects, and on a snapshot
//! (`FFX_SNAPSHOT`) with the same objects over the snapshot's memory.
//! Results and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs (the allocator's free, the physics destructors, fake
//! objects' virtual methods): each call appends its tag, `ecx` and its
//! stack arguments to a log in memory, so the diff checks that both made
//! the same calls, with the same `this`, in the same order.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::unreadable_literal // the generated address tables
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_b::{object, physics};

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
#[allow(dead_code)] // not every kind is used yet
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

    /// Fills `len` bytes at `at` with random words.
    fn fill(&mut self, rng: &mut Rng, at: u32, len: u32) {
        let bytes: Vec<u8> = (0..len / 4)
            .flat_map(|_| rng.next().to_le_bytes())
            .collect();
        self.write(at, &bytes);
    }

    /// Writes a recording stub over the original at `sym` (a `jmp` there):
    /// logs `sym`, `ecx` when `this`, and `n` stack arguments; pops `pops`
    /// words; returns `ret`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) -> u32 {
        let at = self.rebase(sym);
        let addr = self.stub_code(sym, this, n, pops, ret);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        addr
    }

    /// The stub of [`Twins::stub`] in the cave only (for vtables), `tag`
    /// its log tag. Returns its address.
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
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// A vtable of `slots` recording thiscall methods taking `args` words
    /// each (tag `0x7e00_0000 + slot`).
    fn vtable(&mut self, slots: u32, args: u32) -> u32 {
        let mut words = Vec::new();
        for s in 0..slots {
            let f = self.stub_code(0x7e00_0000 + s, true, args, args, 0x5a5a_0000 + s);
            words.extend(f.to_le_bytes());
        }
        self.alloc(&words)
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

// ---- the physics class descriptors' instance callbacks ----

/// How a `destructInstance` destroys its instance.
#[derive(Debug, Clone, Copy)]
enum Destruct {
    /// `vtable[slot](obj - 4, 0)`.
    Virtual(u32),
    /// Nothing.
    Nothing,
    /// A non-virtual destructor at this address, `this = obj`.
    With(u32),
    /// The type's own inline releases (the bridge function).
    Own(fn(&mut G, u32) -> u8),
}

const DESTRUCTS: &[(u32, Destruct)] = &[
    (
        0x005f_8600,
        Destruct::Own(|g, o| physics::bridge::destruct_shape_array(g, o)),
    ),
    (0x005f_8640, Destruct::Virtual(3)),
    (0x005f_8670, Destruct::Virtual(3)),
    (0x005f_86a0, Destruct::Virtual(3)),
    (0x005f_86d0, Destruct::Nothing),
    (0x005f_86e0, Destruct::Virtual(3)),
    (0x005f_8710, Destruct::Virtual(3)),
    (0x005f_8740, Destruct::Virtual(3)),
    (0x005f_8770, Destruct::Virtual(0)),
    (
        0x005f_87a0,
        Destruct::Own(|g, o| physics::bridge::destruct_controller(g, o)),
    ),
    (
        0x005f_8830,
        Destruct::Own(|g, o| physics::bridge::destruct_controller_base(g, o)),
    ),
    (
        0x005f_8860,
        Destruct::Own(|g, o| physics::bridge::destruct_controller_bullet(g, o)),
    ),
    (0x005f_88d0, Destruct::Virtual(3)),
    (0x005f_8900, Destruct::Virtual(3)),
    (0x005f_8930, Destruct::Virtual(3)),
    (0x005f_89b0, Destruct::Nothing),
    (
        0x005f_89c0,
        Destruct::Own(|g, o| physics::bridge::destruct_interface_bullet(g, o)),
    ),
    (0x005f_8a10, Destruct::Nothing),
    (0x005f_8a20, Destruct::Virtual(3)),
    (0x005f_8a50, Destruct::Virtual(3)),
    (0x005f_8a80, Destruct::Virtual(3)),
    (0x005f_8ab0, Destruct::With(0x005e_23f0)),
    (0x005f_8ad0, Destruct::Virtual(3)),
    (0x005f_8b00, Destruct::Virtual(3)),
    (0x005f_8b30, Destruct::Virtual(3)),
    (0x005f_8b60, Destruct::With(0x005e_2570)),
    (0x005f_8b80, Destruct::With(0x005e_2660)),
    (
        0x005f_8ba0,
        Destruct::Own(|g, o| physics::bridge::destruct_rigid_body_bullet(g, o)),
    ),
    (0x005f_8c40, Destruct::Virtual(3)),
    (0x005f_8ca0, Destruct::Virtual(3)),
    (0x005f_8cd0, Destruct::Virtual(3)),
    (0x005f_8d00, Destruct::Virtual(3)),
    (0x005f_8d30, Destruct::Virtual(3)),
    (0x005f_8d60, Destruct::Virtual(3)),
    (0x005f_8d90, Destruct::Virtual(3)),
    (0x005f_8dc0, Destruct::With(0x005e_28d0)),
    (0x005f_8de0, Destruct::Nothing),
    (0x005f_8df0, Destruct::With(0x005e_2950)),
    (0x005f_8e10, Destruct::Nothing),
];

/// The callees the physics callbacks reach, as recording stubs.
fn physics_stubs(t: &mut Twins) {
    t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0, 0x0f0f_0f0f);
    for d in [
        0x005e_23f0,
        0x005e_2570,
        0x005e_2660,
        0x005e_28d0,
        0x005e_2950,
    ] {
        t.stub(d, true, 0, 0, 0x0d0d_0000 + (d & 0xffff));
    }
    t.stub(physics::CONTROLLER_RELEASE, true, 1, 1, 0x0c0c_0c0c);
    t.stub(physics::FIELD_8, true, 0, 0, 0x0808_0808);
}

/// An instance: a block of random words around `obj`, with fake objects
/// (or 0) in the pointer fields the callbacks read, and owned/unowned
/// buffer words.
fn instance(t: &mut Twins, rng: &mut Rng, vt: u32) -> u32 {
    let block = t.zeros(0x300);
    t.fill(rng, block, 0x300);
    // obj sits 0x10 into the block, sometimes unaligned (the interface's
    // obj - 1)
    let obj = block + 0x10 + u32::from(rng.chance(4));
    // obj - 4: the full object's vtable
    t.w32(obj - 4, vt);
    for at in [0xcc, 0xdc, 0xe0, 0x10c, 0x110, 0x114, 0x118] {
        let p = if rng.chance(3) {
            0
        } else {
            let o = t.zeros(0x20);
            t.w32(o, vt);
            o
        };
        t.w32(obj + at, p);
    }
    // the +0x100 buffer and the arrays' headers: flag bits both ways,
    // counts 0, 1, 2 and large
    let buf = rng.next() & !1 | u32::from(rng.chance(2));
    t.w32(obj + 0x100, buf);
    let header = [0, 1, 2, 7, 0x8000_0002, 0x8000_0000, 0x7fff_ffff][rng.below(7) as usize];
    t.w32(obj, header);
    t.w32(obj + 0x1b, header);
    let data = if rng.chance(3) { 0 } else { rng.next() };
    t.w32(obj + 0x1f, data);
    obj
}

#[test]
fn physics_destruct_instance_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x1357_9bdf);
        physics_stubs(&mut t);
        let vt = t.vtable(16, 1);
        for &(sym, kind) in DESTRUCTS {
            let start = t.log_entries();
            for i in 0..24 {
                let obj = instance(&mut t, &mut rng, vt);
                let desc = rng.next();
                let before = t.log_entries();
                t.check(
                    &format!("{name} {sym:#x} case {i}"),
                    sym,
                    Conv::Thiscall(desc),
                    &[obj],
                    |g| {
                        Out::U8(match kind {
                            Destruct::Virtual(slot) => {
                                physics::bridge::destruct_virtual(g, obj, slot)
                            }
                            Destruct::Nothing => physics::bridge::destruct_nothing(),
                            Destruct::With(d) => physics::bridge::destruct_with(g, obj, d),
                            Destruct::Own(f) => f(g, obj),
                        })
                    },
                );
                if matches!(kind, Destruct::Virtual(_) | Destruct::With(_)) {
                    assert!(t.log_entries() > before, "{name} {sym:#x}: no call logged");
                }
            }
            if !matches!(kind, Destruct::Nothing) {
                assert!(t.log_entries() > start, "{name} {sym:#x}: no call logged");
            }
        }
    }
}

#[test]
fn physics_controller_release_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0246_8ace);
        physics_stubs(&mut t);
        let vt = t.vtable(16, 1);
        for i in 0..200 {
            let obj = instance(&mut t, &mut rng, vt);
            t.check(
                &format!("{name} release case {i}"),
                0x005f_8e60,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    physics::bridge::controller_bullet_release(g, obj);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn physics_construct_instance_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0bad_f00d);
        for i in 0..20 {
            let obj = if i == 0 {
                0
            } else {
                let o = t.zeros(0x20);
                t.fill(&mut rng, o, 0x20);
                o
            };
            t.check(
                &format!("{name} construct case {i}"),
                0x005f_83b0,
                Conv::Thiscall(rng.next()),
                &[obj],
                |g| Out::U8(physics::bridge::construct_callback_data(g, obj)),
            );
        }
    }
}

// ---- object boilerplate: destructors, constructors, registrations ----

/// What an object-boilerplate original is.
#[derive(Debug, Clone, Copy)]
enum Obj {
    /// A dynamic-mesh instance constructor storing this vtable.
    Ctor(u32),
    /// A scalar deleting destructor: its vtable and base destructor.
    Dtor(Option<u32>, Option<u32>),
    /// A class registration of this descriptor.
    Register(u32),
    /// `destructInstance` through `vtable[0]`.
    Destruct0,
    /// A constant (thiscall, one stack word).
    Const(u32),
}

const OBJECTS: &[(u32, Obj)] = &[
    (0x006735f0, Obj::Ctor(0x00b44128)),
    (0x00674bf0, Obj::Ctor(0x00b44460)),
    (0x00675510, Obj::Ctor(0x00b445d0)),
    (0x00676b70, Obj::Ctor(0x00b44910)),
    (0x006f9490, Obj::Ctor(0x00b4c250)),
    (0x006736e0, Obj::Dtor(Some(0x00b44128), Some(0x004fa470))),
    (0x00674ce0, Obj::Dtor(Some(0x00b44460), Some(0x004fa470))),
    (0x00675600, Obj::Dtor(Some(0x00b445d0), Some(0x004fa470))),
    (0x00676c60, Obj::Dtor(Some(0x00b44910), Some(0x004fa470))),
    (0x006740b0, Obj::Dtor(Some(0x00b44288), Some(0x004969f0))),
    (0x00676030, Obj::Dtor(Some(0x00b44740), Some(0x004969f0))),
    (0x00677690, Obj::Dtor(Some(0x00b44a90), Some(0x004969f0))),
    (0x00678530, Obj::Dtor(Some(0x00b44c78), Some(0x004969f0))),
    (0x006e0db0, Obj::Dtor(Some(0x00b4aa10), Some(0x004969f0))),
    (0x006e0e10, Obj::Dtor(Some(0x00b4a8f0), Some(0x004969f0))),
    (0x006e0de0, Obj::Dtor(None, Some(0x006e0ab0))),
    (0x009e3d80, Obj::Dtor(None, Some(0x009e2330))),
    (0x009e3e10, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3e40, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3e70, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3ea0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3ed0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3f00, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3f30, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3f60, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3f90, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3fc0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e3ff0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4020, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4050, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4080, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e40b0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e40e0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4110, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4140, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4170, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e41a0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e41d0, Obj::Dtor(Some(0x00b70938), None)),
    (0x009e4200, Obj::Dtor(Some(0x00b70938), None)),
    (0x00673710, Obj::Register(0x00ccbb10)),
    (0x006740e0, Obj::Register(0x00ccbbe0)),
    (0x00674d10, Obj::Register(0x00ccbce0)),
    (0x00675630, Obj::Register(0x00ccbdb8)),
    (0x00676060, Obj::Register(0x00ccbe88)),
    (0x00676c90, Obj::Register(0x00ccbf88)),
    (0x006776c0, Obj::Register(0x00ccc058)),
    (0x00678560, Obj::Register(0x00ccc158)),
    (0x009e4ba0, Obj::Register(0x019414e8)),
    (0x009e4bc0, Obj::Register(0x01941580)),
    (0x009e4be0, Obj::Register(0x01941618)),
    (0x009e4c00, Obj::Register(0x019416b0)),
    (0x009e4c20, Obj::Register(0x01941748)),
    (0x009e4c40, Obj::Register(0x019417e0)),
    (0x009e4c60, Obj::Register(0x01941450)),
    (0x009e4c80, Obj::Register(0x01941288)),
    (0x009e4ca0, Obj::Register(0x01941320)),
    (0x009e4cc0, Obj::Register(0x019413b8)),
    (0x009e4f40, Obj::Register(0x01941028)),
    (0x009e4f60, Obj::Register(0x019410c0)),
    (0x009e4f80, Obj::Register(0x01940f90)),
    (0x009e4fa0, Obj::Register(0x01940ef8)),
    (0x009e4fc0, Obj::Register(0x01941158)),
    (0x009e4fe0, Obj::Register(0x019411f0)),
    (0x00673a70, Obj::Destruct0),
    (0x00674540, Obj::Destruct0),
    (0x00675070, Obj::Destruct0),
    (0x006759d0, Obj::Destruct0),
    (0x00676480, Obj::Destruct0),
    (0x00677030, Obj::Destruct0),
    (0x00677ae0, Obj::Destruct0),
    (0x00673ae0, Obj::Const(0x8)),
    (0x006750e0, Obj::Const(0x18)),
];

#[test]
fn object_boilerplate_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0b1e);
        t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0, 0x0f0f_0f0f);
        for d in [0x004f_a470, 0x0049_69f0, 0x006e_0ab0, 0x009e_2330] {
            t.stub(d, true, 0, 0, 0x0d0d_0000 + (d & 0xffff));
        }
        t.stub(object::DYNAMIC_MESH_INSTANCE_CTOR, true, 1, 1, 0x0c0c_0c0c);
        t.stub(object::REGISTER_TYPE, false, 1, 0, 0x0101_0101);
        t.stub(object::ADD_TO_NAMESPACE, true, 0, 0, 0x4242_4242);
        let vt = t.vtable(4, 1);
        for &(sym, kind) in OBJECTS {
            for i in 0..8 {
                let this = t.zeros(0x40) + 8;
                t.fill(&mut rng, this - 8, 0x40);
                t.w32(this - 4, vt);
                let arg = match i {
                    0 => 0,
                    1 => 1,
                    2 => 0xffff_fffe,
                    _ => rng.next(),
                };
                let what = format!("{name} {sym:#x} case {i}");
                let before = t.log_entries();
                match kind {
                    Obj::Ctor(v) => t.check(&what, sym, Conv::Thiscall(this), &[arg], |g| {
                        Out::U32(object::bridge::construct_mesh_instance(g, this, arg, v))
                    }),
                    Obj::Dtor(v, base) => t.check(&what, sym, Conv::Thiscall(this), &[arg], |g| {
                        Out::U32(object::bridge::scalar_deleting_dtor(g, this, arg, v, base))
                    }),
                    Obj::Register(desc) => t.check(&what, sym, Conv::Cdecl, &[], |g| {
                        Out::U32(object::bridge::register(g, desc))
                    }),
                    Obj::Destruct0 => t.check(&what, sym, Conv::Thiscall(arg), &[this], |g| {
                        Out::U8(physics::bridge::destruct_virtual(g, this, 0))
                    }),
                    Obj::Const(v) => {
                        t.check(&what, sym, Conv::Thiscall(this), &[arg], |_| Out::U32(v));
                    }
                }
                let calls = t.log_entries() - before;
                let want = match kind {
                    Obj::Ctor(_) | Obj::Destruct0 => 1,
                    Obj::Register(_) => 2,
                    Obj::Dtor(_, base) => u32::from(base.is_some()) + (arg & 1),
                    Obj::Const(_) => 0,
                };
                assert_eq!(calls, want, "{what}: calls");
            }
        }
    }
}
