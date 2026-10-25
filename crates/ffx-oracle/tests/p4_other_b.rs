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
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::unreadable_literal // the generated address tables
)]

use std::path::PathBuf;

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_b::{methods, object, physics, small};

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
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
    /// `al`.
    U8(u8),
    /// A float in `ST0`.
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
        if let Out::F32(v) = out {
            // the shim returns it in ST0 (`fld`), which quiets a
            // signalling NaN as the original's does
            let got = ra
                .st0_f32()
                .unwrap_or_else(|| panic!("{what}: the original leaves no ST0"));
            let want = x87::store(x87::load(v.to_bits()));
            assert_eq!(got.to_bits(), want, "{what}: ST0");
            self.diff(what);
            return;
        }
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None | Out::F32(_) => {}
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

// ---- small methods: input devices, lighting, dynamic meshes ----

/// A float of a modest range, a quiet NaN or an infinity (no signalling
/// NaNs: the emulator keeps those).
fn float(rng: &mut Rng) -> f32 {
    match rng.below(8) {
        0 => f32::from_bits(0x7fc0_1234),
        1 => f32::INFINITY,
        2 => -0.0,
        _ => (rng.below(20_000) as f32 - 10_000.0) / 37.0,
    }
}

#[test]
fn small_constants_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0c0f_fee0);
        for i in 0..16 {
            let w: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            let this = rng.next();
            let what = |s: &str| format!("{name} {s} case {i}");
            t.check(
                &what("profile init"),
                0x0060_ed40,
                Conv::Thiscall(this),
                &w[..1],
                |_| Out::U32(0),
            );
            t.check(
                &what("async construct"),
                0x0061_7de0,
                Conv::Thiscall(this),
                &w[..1],
                |_| Out::U8(1),
            );
            t.check(
                &what("async destruct"),
                0x0061_7df0,
                Conv::Thiscall(this),
                &w[..1],
                |_| Out::U8(1),
            );
            t.check(
                &what("callable construct"),
                0x0061_a0a0,
                Conv::Thiscall(this),
                &w[..1],
                |_| Out::U8(0),
            );
            t.check(
                &what("keyboard slot 1"),
                0x0062_81f0,
                Conv::Thiscall(this),
                &[],
                |_| Out::U32(1),
            );
            t.check(
                &what("default mesh slot 5"),
                0x006e_21b0,
                Conv::Thiscall(this),
                &w,
                |_| Out::U32(w[1]),
            );
            let f = float(&mut rng);
            t.check(
                &what("shade coeff"),
                0x006a_77a0,
                Conv::Thiscall(this),
                &[w[0], f.to_bits(), w[2]],
                |_| Out::F32(f),
            );
        }
    }
}

#[test]
fn small_getters_and_setters_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7357_1234);
        let buf = t.zeros(0x400);
        for i in 0..64 {
            t.fill(&mut rng, buf, 0x400);
            let what = |s: &str| format!("{name} {s} case {i}");
            // input devices
            let key = match rng.below(5) {
                0 => rng.below(0x1c),
                1 => 0x1c + rng.below(4),
                2 => 0xffff_ff00 | rng.below(0x100),
                _ => rng.below(0x100),
            };
            t.check(
                &what("keyboard"),
                0x0062_89a0,
                Conv::Thiscall(buf),
                &[key],
                |g| Out::U32(small::bridge::keyboard_button(g, buf, key)),
            );
            let button = match rng.below(4) {
                0 => 0xb + rng.below(3),
                1 => rng.below(0x20),
                2 => 0xffff_ff0b,
                _ => rng.next(),
            };
            t.check(
                &what("mouse"),
                0x0062_89d0,
                Conv::Thiscall(buf),
                &[button],
                |g| Out::U32(small::bridge::mouse_button(g, buf, button)),
            );
            // singletons
            for (sym, global) in [
                (0x006a_6e30, small::G_CHARACTER_LIGHTING),
                (0x006a_6e40, small::G_CHARACTER_LIGHTING_TWEAK),
                (0x006a_bf40, small::G_MORPH_MANAGER),
                (0x006e_32c0, small::G_FOG_EFFECT_MANAGER),
            ] {
                if rng.chance(2) {
                    let at = t.rebase(global);
                    let v = rng.next();
                    t.w32(at, v);
                }
                t.check(&what("singleton"), sym, Conv::Cdecl, &[], |g| {
                    Out::U32(small::bridge::singleton(g, global))
                });
            }
            t.check(
                &what("init ok"),
                0x006a_9ad0,
                Conv::Thiscall(buf),
                &[],
                |g| Out::U8(small::bridge::is_init_successful(g, buf)),
            );
            // the coefficient store, flag byte both ways
            if rng.chance(2) {
                t.write(buf, &[0]);
            }
            let slot = rng.below(32);
            let value = float(&mut rng).to_bits();
            t.check(
                &what("coefficient"),
                0x006a_a920,
                Conv::Thiscall(buf),
                &[slot, value],
                |g| {
                    small::bridge::set_coefficient(g, buf, slot, value);
                    Out::None
                },
            );
            let (a, b) = (rng.next(), rng.next());
            t.check(
                &what("map id"),
                0x006a_ad00,
                Conv::Thiscall(buf),
                &[a, b],
                |g| Out::U32(small::bridge::set_map_id_flag_changed(g, buf, a, b)),
            );
            let slot = rng.below(16);
            t.check(
                &what("vertex buffer"),
                0x006e_2b10,
                Conv::Thiscall(buf),
                &[a, slot],
                |g| Out::U32(small::bridge::set_slot(g, buf, a, slot)),
            );
            t.check(&what("clear"), 0x006e_0800, Conv::Thiscall(buf), &[], |g| {
                Out::U32(small::bridge::clear(g, buf))
            });
        }
    }
}

#[test]
fn small_iterator_valid_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0dd5_eed5);
        for i in 0..200 {
            let it = t.zeros(0x40);
            let node = t.zeros(0x80);
            // a list of 0..5 entries linked by +0x40
            let n = rng.below(5);
            let entries: Vec<u32> = (0..n).map(|_| t.zeros(0x50)).collect();
            for (k, &e) in entries.iter().enumerate() {
                let next = entries.get(k + 1).copied().unwrap_or(0);
                t.w32(e + 0x40, next);
            }
            t.w32(node + 0x2c, entries.first().copied().unwrap_or(0));
            let key = if !entries.is_empty() && rng.chance(2) {
                entries[rng.below(n) as usize]
            } else {
                rng.next() | 1
            };
            t.w32(it + 0x1c, key);
            t.w32(node + 0x40, [0, 1, 0x8000_0000][rng.below(3) as usize]);
            t.w32(it + 0x18, if rng.chance(6) { 0 } else { node });
            t.check(
                &format!("{name} iterator case {i}"),
                0x006a_9a80,
                Conv::Thiscall(it),
                &[],
                |g| Out::U8(small::bridge::iterator_valid(g, it)),
            );
        }
    }
}

#[test]
fn small_forwarders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x00f0_ba12);
        t.stub(small::UPLOAD, false, 4, 0, 0x0b0b_0b0b);
        t.stub(small::DYN_GEO_BUFFER_ID, false, 0, 0, 0x0000_1234);
        for (k, f) in small::SHADER_LOADS.into_iter().enumerate() {
            t.stub(f, false, 0, 0, 0x5ade_0000 + k as u32);
        }
        let vt = t.vtable(4, 1);
        for i in 0..24 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let obj = t.zeros(0x40);
            t.fill(&mut rng, obj, 0x40);
            let mesh = t.zeros(0x40);
            t.fill(&mut rng, mesh, 0x40);
            t.w32(mesh, vt);
            t.w32(obj + 0xc, mesh);
            let out = t.zeros(0x20);
            let (x, y) = (rng.next(), rng.next());
            t.check(
                &what("slot 1 + 8"),
                0x006e_23d0,
                Conv::Thiscall(obj),
                &[x],
                |g| Out::U32(small::bridge::mesh_slot1_plus8(g, obj, x)),
            );
            t.check(
                &what("upload"),
                0x006f_89b0,
                Conv::Thiscall(obj),
                &[x, y],
                |g| Out::U32(small::bridge::upload(g, obj, x, y)),
            );
            t.check(
                &what("preprocess"),
                0x0067_3b20,
                Conv::Thiscall(obj),
                &[out, y],
                |g| Out::U32(small::bridge::populate_preprocess_data(g, obj, out)),
            );
            t.check(&what("shaders"), 0x006d_f3c0, Conv::Cdecl, &[], |g| {
                Out::U32(small::bridge::load_shaders(g))
            });
        }
    }
}

// ---- methods: lighting, tweak utility, fog, cloth meshes, trees ----

/// A float word for the x87 copies: modest values, quiet NaNs, infinities
/// and random words that are not signalling NaNs.
fn float_word(rng: &mut Rng) -> u32 {
    loop {
        let w = if rng.chance(2) {
            float(rng).to_bits()
        } else {
            rng.next()
        };
        if !x87::is_snan(w) {
            return w;
        }
    }
}

#[test]
fn methods_wait_idle_matches_the_original() {
    for (name, mut t) in Twins::all() {
        // the sleep stub adds its argument to [total], counts down [count]
        // and clears [*flag] at 0
        let count = t.zeros(12);
        let flag_at = count + 4;
        let total = count + 8;
        let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        code.extend([0x01, 0x05]);
        code.extend(total.to_le_bytes()); // add [total], eax
        code.extend([0xff, 0x0d]);
        code.extend(count.to_le_bytes()); // dec dword [count]
        code.extend([0x75, 0x0b]); // jnz ret
        code.push(0xa1);
        code.extend(flag_at.to_le_bytes()); // mov eax, [flag_at]
        code.extend([0xc7, 0x00, 0, 0, 0, 0]); // mov dword [eax], 0
        code.push(0xc3);
        let sleep = t.a.alloc_code(&code).unwrap();
        assert_eq!(t.b.alloc_code(&code).unwrap(), sleep);
        let at = t.rebase(methods::SLEEP);
        let mut j = vec![0xe9];
        j.extend(sleep.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let mut rng = Rng(0x0517_ee90);
        for i in 0..24 {
            let mesh = t.zeros(0x80);
            t.fill(&mut rng, mesh, 0x80);
            let off = rng.below(0x18) * 4;
            t.w32(mesh + 8, off);
            let flag = mesh + 8 + off;
            let n = rng.below(4);
            if n == 0 {
                t.w32(flag, 0);
            } else if flag != mesh + 8 {
                t.w32(flag, rng.next() | 1);
            }
            t.w32(count, n.max(1));
            t.w32(flag_at, flag);
            t.check(
                &format!("{name} wait case {i}"),
                0x006e_2df0,
                Conv::Stdcall,
                &[mesh],
                |g| {
                    methods::bridge::wait_idle(g, mesh);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x3e70_0d50);
        t.stub(methods::TEXT_RESIZE, true, 2, 2, 0x7e77_0000);
        t.stub(methods::APPLY_ROTATION, true, 1, 1, 0x0a0a_0000);
        t.stub(methods::CLOTH_PREPARE, true, 3, 3, 0x0c1a_0000);
        t.stub(methods::OPERATOR_NEW, false, 1, 0, 0x0123_4560);
        t.stub(methods::MEMSET, false, 3, 0, 0x0beb_0000);
        let vt = t.vtable(8, 4);
        // a vtable whose slot 5 returns 0
        let zero = t.stub_code(0x7e00_0105, true, 4, 4, 0);
        let vt0 = t.zeros(0x20);
        t.w32(vt0 + 20, zero);
        let buf = t.zeros(0x2000);
        for i in 0..48 {
            t.fill(&mut rng, buf, 0x2000);
            let what = |s: &str| format!("{name} {s} case {i}");
            let (a, b, c, d) = (rng.next(), rng.next(), rng.next(), rng.next());
            // tweak resize, text up or not
            t.write(buf + 0xc8, &[u8::from(rng.chance(2))]);
            t.check(
                &what("resize"),
                0x006a_a3d0,
                Conv::Thiscall(buf),
                &[a, b],
                |g| Out::U32(methods::bridge::tweak_resize(g, buf, a, b)),
            );
            // cloth slot 5: the inner mesh's answer 0 or not
            let inner = t.zeros(0x10);
            t.w32(inner, vt);
            t.w32(buf + 0x1c, inner);
            if rng.chance(2) {
                t.w32(inner, vt0);
            }
            t.check(
                &what("cloth slot 5"),
                0x006e_2150,
                Conv::Thiscall(buf),
                &[a, b, c, d],
                |g| Out::U32(methods::bridge::cloth_slot5(g, buf, a, b, c, d)),
            );
            // rotation: index 0..4 (3 is skipped), flags both ways
            let idx = rng.below(5);
            t.write(buf + 0x1cc, &[u8::from(!rng.chance(4))]);
            if rng.chance(2) {
                t.w32(buf + 0xf0, 0);
            }
            let (x, y) = (float_word(&mut rng), float_word(&mut rng));
            t.check(
                &what("rotation"),
                0x006a_aea0,
                Conv::Thiscall(buf),
                &[idx, x, y],
                |g| {
                    methods::bridge::set_rotation(g, buf, idx, x, y);
                    Out::None
                },
            );
            // fog copy from a source of float words, sometimes overlapping
            // the destination (the copy order shows)
            let src = if rng.chance(3) {
                buf + 4 * rng.below(8)
            } else {
                t.zeros(0x40)
            };
            for k in 0..16 {
                let w = float_word(&mut rng);
                t.w32(src + 4 * k, w);
            }
            t.w32(buf + 0xa0, src);
            t.check(
                &what("fog copy"),
                0x006e_3b80,
                Conv::Thiscall(buf),
                &[],
                |g| Out::U32(methods::bridge::fog_copy(g, buf)),
            );
            // cloth slot buffer: slot empty or not
            let slot = rng.below(16);
            if rng.chance(2) {
                t.w32(buf + 0x210 + 4 * slot, 0);
            }
            t.check(
                &what("slot buffer"),
                0x006e_2ac0,
                Conv::Thiscall(buf),
                &[a, b, slot],
                |g| Out::U32(methods::bridge::cloth_slot_buffer(g, buf, a, b, slot)),
            );
        }
    }
}

/// A search tree of `n` nodes with keys from `keys`, built as a binary
/// search tree (greater keys to `[node]`, smaller to `[node + 4]`), leaves
/// pointing at the sentinel `tree`.
fn search_tree(t: &mut Twins, keys: &[u32], signed: bool) -> u32 {
    let tree = t.zeros(0x10);
    t.w32(tree + 0xc, tree);
    let mut nodes: Vec<(u32, u32)> = Vec::new();
    for &k in keys {
        let node = t.zeros(0x10);
        t.w32(node, tree);
        t.w32(node + 4, tree);
        t.w32(node + 0xc, k);
        if nodes.is_empty() {
            t.w32(tree + 0xc, node);
        } else {
            let mut at = nodes[0].0;
            loop {
                let ak = nodes.iter().find(|n| n.0 == at).unwrap().1;
                let greater = if signed {
                    (ak as i32) > (k as i32)
                } else {
                    ak > k
                };
                if ak == k {
                    break;
                }
                let link = if greater { at } else { at + 4 };
                let next: u32 = Mem::read(&t.b, link);
                if next == tree {
                    t.w32(link, node);
                    break;
                }
                at = next;
            }
        }
        nodes.push((node, k));
    }
    tree
}

#[test]
fn methods_tree_find_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x7ee5_eed5);
        for i in 0..200 {
            let signed = i % 2 == 0;
            let sym = if signed { 0x0064_b9a0 } else { 0x006a_5ec0 };
            let n = rng.below(12) as usize;
            let keys: Vec<u32> = (0..n)
                .map(|_| match rng.below(3) {
                    0 => rng.below(16),
                    1 => 0xffff_fff0 + rng.below(16),
                    _ => rng.next(),
                })
                .collect();
            let tree = search_tree(&mut t, &keys, signed);
            let key = if !keys.is_empty() && rng.chance(2) {
                keys[rng.below(n as u32) as usize]
            } else {
                rng.next()
            };
            let key_at = t.zeros(4);
            t.w32(key_at, key);
            t.check(
                &format!("{name} find case {i}"),
                sym,
                Conv::Thiscall(tree),
                &[key_at],
                |g| Out::U32(methods::bridge::tree_find(g, tree, key_at, signed)),
            );
        }
    }
}

#[test]
fn methods_mesh_constructors_match_the_original() {
    // (address, the mesh class descriptor or 0 for a Concrete mesh, vtable)
    const CTORS: &[(u32, u32, u32)] = &[
        (0x0067_39f0, 0x00cc_ba18, 0x00b4_4128),
        (0x0067_4ff0, 0x00cc_bbe0, 0x00b4_4460),
        (0x0067_5950, 0x00cc_be88, 0x00b4_45d0),
        (0x0067_6fb0, 0x00cc_c058, 0x00b4_4910),
        (0x0067_4460, 0, 0x00b4_4288),
        (0x0067_63a0, 0, 0x00b4_4740),
        (0x0067_7a00, 0, 0x00b4_4a90),
    ];
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0c70_75ed);
        t.stub(object::DYNAMIC_MESH_CTOR, true, 0, 0, 0x0d0d_0d0d);
        t.stub(object::DYNAMIC_MESH_INSTANCE_CTOR, true, 1, 1, 0x0c0c_0c0c);
        // the default-object lookup returns [answer]
        let answer = t.zeros(4);
        let mut code = vec![0xa1];
        code.extend(answer.to_le_bytes()); // mov eax, [answer]
        code.push(0xc3);
        let lookup = t.a.alloc_code(&code).unwrap();
        assert_eq!(t.b.alloc_code(&code).unwrap(), lookup);
        let at = t.rebase(object::DESCRIPTOR_DEFAULT_OBJECT);
        let mut j = vec![0xe9];
        j.extend(lookup.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        for &(sym, desc, vt) in CTORS {
            for i in 0..12 {
                let obj = if i == 0 {
                    0
                } else {
                    let o = t.zeros(0x20);
                    t.fill(&mut rng, o, 0x20);
                    o
                };
                t.w32(
                    answer,
                    [0, 4, 0x1000_0004, rng.next()][rng.below(4) as usize],
                );
                let what = format!("{name} {sym:#x} case {i}");
                t.check(&what, sym, Conv::Thiscall(rng.next()), &[obj], |g| {
                    Out::U8(if desc == 0 {
                        object::bridge::construct_mesh(g, obj, vt)
                    } else {
                        object::bridge::construct_mesh_instance_default(g, obj, desc, vt)
                    })
                });
            }
        }
    }
}

#[test]
fn methods_devices_and_counts_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0de7_1ce5);
        // one vtable per device kind 0..4, slot 1 returning the kind
        let kinds: Vec<u32> = (0..4)
            .map(|k| {
                let f = t.stub_code(0x7d00_0000 + k, true, 0, 0, k);
                let vt = t.zeros(8);
                t.w32(vt + 4, f);
                vt
            })
            .collect();
        let head = t.rebase(methods::G_INPUT_DEVICE_LIST);
        for i in 0..64 {
            let n = rng.below(7);
            let devices: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
            // the list ends at the head, at 0 or at a link of 4 (device 0)
            let end = [head, head, 0, 4][rng.below(4) as usize];
            for (k, &d) in devices.iter().enumerate() {
                let kind = kinds[rng.below(4) as usize];
                t.w32(d, kind);
                let next = devices.get(k + 1).map_or(end, |&n| n + 4);
                t.w32(d + 4, next);
            }
            t.w32(head, devices.first().map_or(head, |&d| d + 4));
            let (kind, index) = (rng.below(5), rng.below(4));
            t.check(
                &format!("{name} device case {i}"),
                0x0062_3c90,
                Conv::Cdecl,
                &[kind, index],
                |g| Out::U32(methods::bridge::get_device(g, kind, index)),
            );
            // the mesh counts through the original getters
            let mesh = t.zeros(0x60);
            t.fill(&mut rng, mesh, 0x60);
            t.w32(mesh + 8, [6, 8, 1, rng.next()][rng.below(4) as usize]);
            let owner = t.zeros(0x20);
            t.w32(owner + 0x1c, mesh);
            let inst = t.zeros(0x10);
            t.w32(inst + 4, owner);
            let out = t.zeros(0x30);
            t.check(
                &format!("{name} counts case {i}"),
                0x0067_70f0,
                Conv::Thiscall(inst),
                &[out, 0],
                |g| Out::U32(methods::bridge::broken_screen_preprocess(g, inst, out)),
            );
        }
    }
}

#[test]
fn methods_lighting_and_fog_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0f09_11b7);
        t.stub(methods::APPLY_INTENSITY, true, 2, 2, 0x0a11_0000);
        t.stub(methods::OPERATOR_NEW_SCALAR, false, 1, 0, 0);
        // the lock check returns [locked] (al)
        let locked = t.zeros(4);
        let mut code = vec![0xa1];
        code.extend(locked.to_le_bytes()); // mov eax, [locked]
        code.push(0xc3);
        let lock = t.a.alloc_code(&code).unwrap();
        assert_eq!(t.b.alloc_code(&code).unwrap(), lock);
        let at = t.rebase(methods::FOG_LOCKED);
        let mut j = vec![0xe9];
        j.extend(lock.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let buf = t.zeros(0x400);
        let table = t.zeros(1200);
        let fresh = t.zeros(0x100);
        for i in 0..48 {
            t.fill(&mut rng, buf, 0x400);
            t.fill(&mut rng, table, 1200);
            let what = |s: &str| format!("{name} {s} case {i}");
            // the tweak table: the id present (once or twice) or not
            t.write(buf, &[u8::from(!rng.chance(4))]);
            t.w32(buf + 0xf8, table);
            let id = rng.next();
            for _ in 0..rng.below(3) {
                // the last entry (99) often: the search's bound
                let k = if rng.chance(3) { 99 } else { rng.below(100) };
                t.w32(table + 12 * k, id);
            }
            let mode = [2, 0, 1, rng.next()][rng.below(4) as usize];
            let coeff = float_word(&mut rng);
            t.check(
                &what("tweak coeff"),
                0x006a_afc0,
                Conv::Thiscall(buf),
                &[id, coeff, mode],
                |g| {
                    methods::bridge::tweak_set_shade_coeff(g, buf, id, coeff, mode);
                    Out::None
                },
            );
            // the intensities
            let idx = rng.below(5);
            t.write(buf + 0x1cc, &[u8::from(!rng.chance(4))]);
            if rng.chance(2) {
                t.w32(buf + 0xf0, 0);
            }
            let value = float_word(&mut rng);
            t.check(
                &what("intensity"),
                0x006a_a8c0,
                Conv::Thiscall(buf),
                &[idx, value],
                |g| {
                    methods::bridge::set_intensity(g, buf, idx, value);
                    Out::None
                },
            );
            // the fog limit, locked or not
            let src = t.zeros(0x40);
            for k in 0..16 {
                let w = float_word(&mut rng);
                t.w32(src + 4 * k, w);
            }
            t.w32(buf + 0xa0, src);
            t.w32(locked, [0, 1, 0x100, 0xff][rng.below(4) as usize]);
            let limit = float_word(&mut rng);
            t.check(
                &what("fog limit"),
                0x006e_3aa0,
                Conv::Thiscall(buf),
                &[limit],
                |g| {
                    methods::bridge::fog_set_limit(g, buf, limit);
                    Out::None
                },
            );
            // the fog manager's creation: present, new fails, new succeeds
            let g_at = t.rebase(methods::G_FOG_EFFECT_MANAGER);
            let case = rng.below(3);
            t.w32(g_at, if case == 0 { rng.next() | 1 } else { 0 });
            let at = t.rebase(methods::OPERATOR_NEW_SCALAR);
            let ret = if case == 2 { fresh } else { 0 };
            let mut code = vec![0xb8];
            code.extend(ret.to_le_bytes()); // mov eax, ret
            code.push(0xc3);
            let newf = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), newf);
            let mut j = vec![0xe9];
            j.extend(newf.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            t.check(&what("fog create"), 0x006e_3240, Conv::Cdecl, &[], |g| {
                methods::bridge::fog_create(g);
                Out::None
            });
        }
    }
}
