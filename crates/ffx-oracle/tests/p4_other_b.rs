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
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        ffx_oracle::check_width(
            "Host::call_st0",
            addr,
            args.len(),
            host::max_words(conv, true),
        );
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .st0_f32()
            .unwrap_or_else(|| panic!("callee {addr:#x} leaves no ST0"))
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
    /// `edx:eax`.
    U64(u64),
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
            Out::U64(v) => assert_eq!(
                (u64::from(ra.edx) << 32) | u64::from(ra.eax),
                v,
                "{what}: edx:eax"
            ),
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

// ---- continuation round (c1): input, lighting, fog ----

use ffx_pother_b::{p4_fog, p4_input, p4_light};

impl Twins {
    /// Empties the call log (each case starts from a clean log, so it
    /// never overflows).
    fn reset_log(&mut self) {
        let log = self.log;
        self.w32(log, 0);
    }

    /// Code in both oracles; returns its address.
    fn code(&mut self, code: &[u8]) -> u32 {
        let at = self.a.alloc_code(code).unwrap();
        assert_eq!(self.b.alloc_code(code).unwrap(), at);
        at
    }

    /// A `jmp` at the original `sym` to `to`.
    fn jump(&mut self, sym: u32, to: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// A recording stub over `sym` (as [`Twins::stub`]) that ends with
    /// `tail` instead of a constant return.
    fn stub_tail(&mut self, sym: u32, this: bool, n: u32, tail: &[u8]) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend(tail);
        let f = self.code(&c);
        self.jump(sym, f);
    }

    /// A recording stub over `sym` returning `[cell]` in `eax` and
    /// popping `pops` words; returns the cell.
    fn stub_cell(&mut self, sym: u32, this: bool, n: u32, pops: u32) -> u32 {
        let cell = self.zeros(4);
        let mut tail = vec![0xa1];
        tail.extend(cell.to_le_bytes()); // mov eax, [cell]
        tail.push(0xc2);
        tail.extend(((pops * 4) as u16).to_le_bytes()); // ret pops*4
        self.stub_tail(sym, this, n, &tail);
        cell
    }

    /// A recording thiscall stub over `sym` (no stack words) returning
    /// the `f32` at `[cell]` in `ST0`; returns the cell.
    fn stub_st0(&mut self, sym: u32) -> u32 {
        let cell = self.zeros(4);
        let mut tail = vec![0xd9, 0x05];
        tail.extend(cell.to_le_bytes()); // fld dword [cell]
        tail.push(0xc3);
        self.stub_tail(sym, true, 0, &tail);
        cell
    }

    /// Runs [`Twins::check`] with a clean log.
    fn case(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) {
        self.reset_log();
        self.check(what, sym, conv, args, port);
    }
}

#[test]
fn p4_input_devices_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_63e0);
        t.stub(p4_input::MEMSET, false, 3, 0, 0x0e3e_0000);
        let head = t.rebase(p4_input::DEVICE_LIST);
        for i in 0..64 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // the viewport
            let vp = t.zeros(0x40);
            t.fill(&mut rng, vp, 0x40);
            let app = t.zeros(0x280);
            t.fill(&mut rng, app, 0x280);
            t.case(
                &what("viewport"),
                0x0061_fdd0,
                Conv::Thiscall(vp),
                &[app],
                |g| Out::U32(p4_input::bridge::viewport_ctor(g, vp, app)),
            );
            // the keyboard: linked after the list's last device
            let last = t.zeros(8);
            t.fill(&mut rng, last, 8);
            t.w32(head + 4, last);
            let dev = t.zeros(0x320);
            t.fill(&mut rng, dev, 0x320);
            let state = rng.next();
            t.case(
                &what("keyboard"),
                0x0061_fff0,
                Conv::Thiscall(dev),
                &[state],
                |g| Out::U32(p4_input::bridge::keyboard_ctor(g, dev, state)),
            );
            // the mouse's axes and binds
            let mouse = t.zeros(0x120);
            t.fill(&mut rng, mouse, 0x120);
            for at in [0x10c, 0x110, 0x114] {
                let v = match rng.below(4) {
                    0 => rng.below(511).wrapping_sub(255),
                    1 => [0, 0x7fff_ffff, 0x8000_0000, 0x0100_0001][rng.below(4) as usize],
                    _ => rng.next(),
                };
                t.w32(mouse + at, v);
            }
            // ids around both ranges' ends (0, 1-9, 0xb-0xd, 0xe)
            let id = rng.below(20).wrapping_sub(2);
            t.case(
                &what("axis"),
                0x0062_8a40,
                Conv::Thiscall(mouse),
                &[id],
                |g| Out::F32(p4_input::bridge::mouse_get_axis(g, mouse, id)),
            );
            let mapping = t.zeros(0x30);
            t.fill(&mut rng, mapping, 0x30);
            if !rng.chance(4) {
                t.w32(mapping + 8, 0);
            }
            let tail = t.zeros(8);
            t.w32(mouse + 0x10, tail);
            t.case(
                &what("bind"),
                0x0062_5d10,
                Conv::Thiscall(mouse),
                &[mapping, id],
                |g| Out::U32(p4_input::bridge::mouse_bind(g, mouse, mapping, id)),
            );
        }
    }
}

/// A mapping's id and float fields for [`p4_input::mapping_value`].
fn mapping_fields(t: &mut Twins, rng: &mut Rng, m: u32) {
    t.w32(m + 0xc, rng.below(13));
    for at in [0x10, 0x14, 0x18] {
        let w = match rng.below(3) {
            0 => float_word(rng),
            1 => (rng.below(100) as f32 / 50.0 - 1.0).to_bits(),
            _ => (rng.below(100) as f32 / 400.0).to_bits(),
        };
        t.w32(m + at, w);
    }
    if rng.chance(4) {
        // a value exactly at the deadzone: scale 0, offset = deadzone
        let off: u32 = Mem::read(&t.b, m + 0x14);
        t.w32(m + 0x10, 0);
        t.w32(m + 0x18, off & 0x7fff_ffff);
    }
}

fn raw_value(rng: &mut Rng) -> u32 {
    match rng.below(4) {
        0 => rng.below(2048).wrapping_sub(1024),
        1 => [0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ff01, 0x7fff_ff00][rng.below(6) as usize],
        _ => rng.below(256),
    }
}

#[test]
fn p4_input_mappings_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_d550);
        for i in 0..200 {
            let m = t.zeros(0x30);
            t.fill(&mut rng, m, 0x30);
            mapping_fields(&mut t, &mut rng, m);
            let raw = raw_value(&mut rng);
            t.case(
                &format!("{name} mapping case {i}"),
                0x0062_63e0,
                Conv::Thiscall(m),
                &[raw],
                |g| {
                    p4_input::bridge::mapping_update(g, m, raw);
                    Out::None
                },
            );
        }
        for i in 0..64 {
            // a device's list of byte, word and unbound mappings
            let dev = t.zeros(0x20);
            let list = dev + 0xc;
            let n = rng.below(6);
            let mut prev = list;
            t.w32(list, list);
            for _ in 0..n {
                let m = t.zeros(0x30);
                t.fill(&mut rng, m, 0x30);
                mapping_fields(&mut t, &mut rng, m);
                t.write(m + 0x24, &[rng.below(3) as u8]);
                let src = t.zeros(4);
                match rng.below(3) {
                    0 => {
                        t.write(src, &[rng.below(3) as u8, 0x55, 0x66, 0x77]);
                        t.w32(m + 0x28, src);
                    }
                    1 => {
                        let v = raw_value(&mut rng);
                        t.w32(src, v);
                        t.w32(m + 0x28, 0);
                        t.w32(m + 0x2c, src);
                    }
                    _ => {
                        t.w32(m + 0x28, 0);
                        t.w32(m + 0x2c, 0);
                    }
                }
                t.w32(prev, m);
                t.w32(m + 4, prev);
                prev = m;
            }
            // the list ends at its head, or at a null link
            t.w32(prev, if rng.chance(4) && n > 0 { 0 } else { list });
            t.case(
                &format!("{name} update mappings case {i}"),
                0x0062_d550,
                Conv::Thiscall(dev),
                &[],
                |g| {
                    p4_input::bridge::update_mappings(g, dev);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn p4_input_frame_tree_distortion_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_e340);
        let update = t.stub_cell(p4_input::INPUT_UPDATE, false, 0, 0);
        t.stub(p4_input::TTY_WRITE, false, 3, 0, 0x0777_0000);
        t.stub(p4_input::MAPPER_UPDATE, true, 0, 0, 0x0e04_0000);
        let inst = t.zeros(0x10);
        t.stub(p4_input::DISTORTION_INSTANCE, false, 0, 0, inst);
        for (k, &g) in p4_input::DISTORTION_GETTERS.iter().enumerate() {
            t.stub(g, true, 0, 0, 0x0d15_0000 + k as u32);
        }
        let g_app = t.rebase(p4_input::G_APPLICATION);
        for i in 0..48 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let app = t.zeros(0x2c0);
            t.fill(&mut rng, app, 0x2c0);
            t.write(app + 0x39, &[u8::from(rng.chance(3))]);
            t.w32(g_app, app);
            t.w32(update, [0, 0, 1, rng.next()][rng.below(4) as usize]);
            t.case(&what("frame"), 0x0062_e340, Conv::Cdecl, &[], |g| {
                Out::U32(p4_input::bridge::app_frame(g))
            });
            let params = t.zeros(0x30);
            t.fill(&mut rng, params, 0x30);
            let x = rng.next();
            t.case(
                &what("distortion"),
                0x0067_5120,
                Conv::Stdcall,
                &[params, x],
                |g| Out::U32(p4_input::bridge::distortion_params(g, params)),
            );
        }
        for i in 0..240 {
            let (sym, signed) = [
                (0x0064_bfa0, true),
                (0x0064_c020, false),
                (0x006a_5fc0, false),
            ][i % 3];
            let n = rng.below(10) as usize;
            let keys: Vec<u32> = (0..n)
                .map(|_| match rng.below(3) {
                    0 => rng.below(16),
                    1 => 0xffff_fff0 + rng.below(16),
                    _ => rng.next(),
                })
                .collect();
            let tree = search_tree(&mut t, &keys, signed);
            let node = t.zeros(0x10);
            t.fill(&mut rng, node, 0x10);
            let key = if n > 0 && rng.chance(3) {
                keys[rng.below(n as u32) as usize]
            } else {
                match rng.below(3) {
                    0 => rng.below(16),
                    1 => 0xffff_fff0 + rng.below(16),
                    _ => rng.next(),
                }
            };
            t.w32(node + 0xc, key);
            t.case(
                &format!("{name} insert case {i}"),
                sym,
                Conv::Thiscall(tree),
                &[node],
                |g| Out::U32(p4_input::bridge::tree_insert(g, tree, node, signed)),
            );
        }
    }
}

/// A lighting manager over random words, with a light set of five lights
/// (point, colour or other types).
fn lighting(t: &mut Twins, rng: &mut Rng) -> u32 {
    let cl = t.zeros(0x240);
    t.fill(rng, cl, 0x240);
    t.write(cl + 0x1cc, &[u8::from(!rng.chance(4))]);
    let set = t.zeros(8);
    let lights = t.zeros(20);
    t.w32(cl + 0xf0, set);
    t.w32(set + 4, lights);
    let point = t.rebase(p4_light::POINT_LIGHT_TYPE);
    let color = t.rebase(p4_light::COLOR_LIGHT_TYPE);
    for k in 0..5 {
        let l = t.zeros(0x30);
        t.fill(rng, l, 0x30);
        let node = t.zeros(0x30);
        t.fill(rng, node, 0x30);
        t.w32(l + 0x10, node);
        let ty = [point, color, rng.next()][rng.below(3) as usize];
        t.w32(l + 0x18, ty);
        let w = float_word(rng);
        t.w32(l + 0x24, w);
        t.w32(lights + 4 * k, l);
    }
    for at in [0x1a0, 0x1a4, 0x1d0, 0x1d4, 0x1d8, 0x1dc] {
        let w = match rng.below(4) {
            0 => float_word(rng),
            1 => (rng.below(100) as f32 / 100.0 - 0.1).to_bits(),
            2 => [
                0,
                0x3c23_d70a,
                0xbc23_d70a,
                0x3c23_d70b,
                0x3e4c_cccd,
                0x3f2a_aaab,
            ][rng.below(6) as usize],
            _ => (rng.below(100) as f32 / 150.0).to_bits(),
        };
        t.w32(cl + at, w);
    }
    cl
}

#[test]
fn p4_light_methods_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_a940);
        t.stub(p4_light::APPLY_INTENSITY, true, 2, 2, 0x0a11_0000);
        t.stub(p4_light::SET_SHADE_COEFF, true, 2, 2, 0x05c0_0000);
        let alt = t.stub_st0(p4_light::SHADE_COEFF_ALT);
        let g_cl = t.rebase(p4_light::G_LIGHTING);
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let cl = lighting(&mut t, &mut rng);
            let k = rng.below(5);
            t.case(
                &what("has intensity"),
                0x006a_9ae0,
                Conv::Thiscall(cl),
                &[k],
                |g| Out::U32(p4_light::bridge::has_intensity(g, cl, k)),
            );
            let (r, gg, b) = (
                float_word(&mut rng),
                float_word(&mut rng),
                float_word(&mut rng),
            );
            let li = rng.below(4);
            t.case(
                &what("point light"),
                0x006a_ad20,
                Conv::Thiscall(cl),
                &[r, gg, b, li],
                |g| {
                    p4_light::bridge::set_point_light(g, cl, r, gg, b, li);
                    Out::None
                },
            );
            let kind = rng.below(5);
            t.case(
                &what("light color"),
                0x006a_a940,
                Conv::Thiscall(cl),
                &[kind, r, gg, b],
                |g| {
                    p4_light::bridge::set_light_color(g, cl, kind, r, gg, b);
                    Out::None
                },
            );
            t.case(
                &what("default coeff"),
                0x006a_8c40,
                Conv::Thiscall(cl),
                &[],
                |g| Out::F32(p4_light::bridge::shade_coeff_default(g, cl)),
            );
            let inst = rng.next();
            t.case(
                &what("reset coeff"),
                0x006a_afa0,
                Conv::Thiscall(cl),
                &[inst],
                |g| Out::U32(p4_light::bridge::reset_shade_coeff(g, cl, inst)),
            );
            // the tweak table, applied to g_CharacterLighting
            let tw = t.zeros(0x100);
            t.fill(&mut rng, tw, 0x100);
            t.write(tw, &[u8::from(!rng.chance(4))]);
            let table = t.zeros(0x4b0);
            t.fill(&mut rng, table, 0x4b0);
            for e in 0..100 {
                let at = table + 12 * e;
                if rng.chance(2) {
                    t.w32(at, 0);
                }
                let mode = [0, 1, 2, 2, rng.next()][rng.below(5) as usize];
                t.w32(at + 8, mode);
                let c = float_word(&mut rng);
                t.w32(at + 4, c);
            }
            t.w32(tw + 0xf8, table);
            t.w32(g_cl, cl);
            let a = float_word(&mut rng);
            t.w32(alt, a);
            t.case(
                &what("tweak apply"),
                0x006a_b720,
                Conv::Thiscall(tw),
                &[],
                |g| {
                    p4_light::bridge::tweak_apply_shade_coeffs(g, tw);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn p4_light_creation_and_arrays_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_6cd0);
        let new = t.stub_cell(p4_light::OPERATOR_NEW, false, 1, 0);
        let ctor = t.stub_cell(p4_light::LIGHTING_CTOR, true, 0, 0);
        t.stub(p4_light::TEXT_CTOR, true, 0, 0, 0x7e47_0000);
        t.stub(p4_light::MEMSET, false, 3, 0, 0x0e3e_0000);
        t.stub(p4_light::PHYRE_FREE, false, 1, 0, 0x0f0f_0000);
        for i in 0..48 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let fresh = t.zeros(0x100);
            t.fill(&mut rng, fresh, 0x100);
            let made = rng.next();
            t.w32(ctor, made);
            for (sym, global, which) in [
                (0x006a_6cd0, p4_light::G_LIGHTING, 0),
                (0x006a_6d50, p4_light::G_TWEAK, 1),
                (0x006a_be40, p4_light::G_MORPH, 2),
            ] {
                let g_at = t.rebase(global);
                let case = rng.below(3);
                t.w32(g_at, if case == 0 { rng.next() | 1 } else { 0 });
                t.w32(new, if case == 2 { fresh } else { 0 });
                t.case(&what("create"), sym, Conv::Cdecl, &[], |g| {
                    match which {
                        0 => p4_light::bridge::lighting_create(g),
                        1 => p4_light::bridge::tweak_create(g),
                        _ => p4_light::bridge::morph_create(g),
                    }
                    Out::None
                });
            }
            let tw = t.zeros(0x100);
            t.fill(&mut rng, tw, 0x100);
            t.case(
                &what("tweak ctor"),
                0x006a_6610,
                Conv::Thiscall(tw),
                &[],
                |g| Out::U32(p4_light::bridge::tweak_ctor(g, tw)),
            );
        }
        for i in 0..160 {
            let (sym, words, free_null) = if i % 2 == 0 {
                (0x006a_94b0, 1, false)
            } else {
                (0x006a_c380, 2, true)
            };
            let count = rng.below(6);
            let arr = t.zeros(8);
            // a null old buffer only without elements to copy (with them
            // both fault)
            let old = if count == 0 && rng.chance(2) {
                0
            } else {
                let o = t.zeros(0x40);
                t.fill(&mut rng, o, 0x40);
                o
            };
            let head = count | if rng.chance(3) { 0x8000_0000 } else { 0 };
            t.w32(arr, head);
            t.w32(arr + 4, old);
            let buffer = match rng.below(6) {
                0 => 0,
                1 => old,
                _ => {
                    let b = t.zeros(0x40);
                    t.fill(&mut rng, b, 0x40);
                    b
                }
            };
            let item = t.zeros(8);
            t.fill(&mut rng, item, 8);
            t.case(
                &format!("{name} push case {i}"),
                sym,
                Conv::Thiscall(arr),
                &[buffer, item],
                |g| {
                    Out::U32(p4_light::bridge::array_push(
                        g, arr, buffer, item, words, free_null,
                    ))
                },
            );
        }
    }
}

#[test]
fn p4_fog_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006e_3480);
        for i in 0..160 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let fog = t.zeros(0xc0);
            t.fill(&mut rng, fog, 0xc0);
            t.case(&what("reset"), 0x006e_34f0, Conv::Thiscall(fog), &[], |g| {
                p4_fog::bridge::reset(g, fog);
                Out::None
            });
            let (s, a, b) = (rng.next(), rng.next(), rng.next());
            t.case(
                &what("initialize"),
                0x006e_3400,
                Conv::Thiscall(fog),
                &[s, a, b],
                |g| {
                    p4_fog::bridge::initialize(g, fog, s, a, b);
                    Out::None
                },
            );
            // two parameter sets, near each other or not
            let other = t.zeros(0x40);
            let src = t.zeros(0x40);
            for k in 0..16 {
                let w = float_word(&mut rng);
                t.w32(fog + 4 * k, w);
            }
            for k in 0..16 {
                let mine: u32 = Mem::read(&t.b, fog + 4 * k);
                let w = match rng.below(5) {
                    0 => float_word(&mut rng),
                    1 => mine,
                    2 => {
                        let x = f32::from_bits(mine);
                        (x + (rng.below(64) as f32 - 32.0) / 512.0).to_bits()
                    }
                    // a difference with every bit set: the sums round
                    _ => {
                        let x = f32::from_bits(mine);
                        let r = rng.next() as f32 / 4_294_967_296.0;
                        (x * (1.0 + r / 64.0) + r).to_bits()
                    }
                };
                let w = if x87::is_snan(w) { mine } else { w };
                t.w32(other + 4 * k, w);
                t.w32(src + 4 * k, w);
            }
            if rng.chance(3) {
                // differences of one magnitude with every bit set, so each
                // f32 step of the sums rounds
                for at in [0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x2c] {
                    let r = (rng.next() >> 8) as f32 / 16_777_216.0;
                    t.w32(fog + at, (r + 0.5).to_bits());
                    t.w32(other + at, 0);
                    t.w32(src + at, 0);
                }
            }
            // the source as the manager compares it: factors swapped back
            if rng.chance(2) {
                let f28: u32 = Mem::read(&t.b, other + 0x28);
                let f2c: u32 = Mem::read(&t.b, other + 0x2c);
                t.w32(src + 0x2c, f28);
                t.w32(src + 0x28, f2c);
            }
            t.case(
                &what("distance"),
                0x006e_3320,
                Conv::Thiscall(fog),
                &[other],
                |g| Out::F32(p4_fog::bridge::distance_to(g, fog, other)),
            );
            if rng.chance(4) {
                // a distance of exactly 0.1f, or the floats next to it:
                // f32(d * d) at +0x20 a few ulps under it, and e = 2^k at
                // +0x10 whose square is exactly the rest
                let want = 0.1f32.to_bits() + rng.below(3) - 1;
                let (d, e) = [(2u32, -13), (8, -12), (32, -11), (128, -10), (512, -9)]
                    .into_iter()
                    .find_map(|(j, k)| {
                        (0x3ea1_0000..0x3ea3_0000u32)
                            .map(f32::from_bits)
                            .find(|d| {
                                ((f64::from(*d) * f64::from(*d)) as f32).to_bits() == want - j
                            })
                            .map(|d| (d, 2f32.powi(k)))
                    })
                    .unwrap();
                for at in (0..0x40).step_by(4) {
                    let v: u32 = Mem::read(&t.b, fog + at);
                    let v = if x87::is_snan(v) { 0 } else { v };
                    t.w32(fog + at, v);
                    t.w32(src + at, v);
                }
                t.w32(fog + 0x20, d.to_bits());
                t.w32(src + 0x20, 0);
                t.w32(fog + 0x10, e.to_bits());
                t.w32(src + 0x10, 0);
                let f28: u32 = Mem::read(&t.b, fog + 0x28);
                let f2c: u32 = Mem::read(&t.b, fog + 0x2c);
                t.w32(src + 0x2c, f28);
                t.w32(src + 0x28, f2c);
            }
            t.w32(fog + 0xa0, src);
            t.case(
                &what("moving"),
                0x006e_3480,
                Conv::Thiscall(fog),
                &[],
                |g| Out::U8(p4_fog::bridge::is_moving(g, fog)),
            );
        }
    }
}

// ---- continuation round (c1) batch 2: morphs, material parameters,
// cloth, iterators, button latches ----

use ffx_pother_b::{p4_cloth, p4_morph};

impl Twins {
    /// Zeros the top of the original's stack, so words its frame never
    /// writes read 0 as the port's work blocks do.
    fn zero_stack_top(&mut self) {
        let end = self.a.stack_range().end;
        self.a.write(end - 0x1000, &[0; 0x1000]).unwrap();
    }
}

/// `ShaderParams_FindByName` as a stub: logs, returns the shader's
/// `+0x40` word.
fn find_param_stub(t: &mut Twins) {
    t.stub_tail(
        p4_morph::FIND_PARAM,
        true,
        1,
        &[0x8b, 0x41, 0x40, 0xc2, 0x04, 0x00], // mov eax, [ecx+0x40]; ret 4
    );
}

/// An instance whose material list (`+0x10`) has `n` elements (shader,
/// buffer); each shader's `+0x40` is a parameter (offset, size) or 0.
/// Null elements only with `nulls` (only `hasParameter` checks for them).
/// Returns the instance and the list.
fn material(t: &mut Twins, rng: &mut Rng, n: u32, max_size: u32, nulls: bool) -> (u32, u32) {
    let inst = t.zeros(0x20);
    t.fill(rng, inst, 0x20);
    let list = t.zeros(8);
    let flag = if rng.chance(4) { 0x8000_0000 } else { 0 };
    t.w32(list, n | flag);
    let data = t.zeros(4 * n.max(1) as usize);
    t.w32(list + 4, data);
    t.w32(inst + 0x10, list);
    for i in 0..n {
        let e = if nulls && rng.chance(6) {
            0
        } else {
            let e = t.zeros(8);
            let shader = t.zeros(0x44);
            let buf = t.zeros(0x80);
            t.fill(rng, buf, 0x80);
            t.w32(buf, [0x80, 0x40, 0x10, 0][rng.below(4) as usize]);
            let p = if rng.chance(4) {
                0
            } else {
                let p = t.zeros(0x10);
                t.fill(rng, p, 0x10);
                let off = [0, 2, 4, 8, 0x20, 0x3c, 0x7c][rng.below(7) as usize];
                let size = (rng.below(max_size + 1)) | (rng.below(8) << 13);
                t.write(p + 8, &(off as u16).to_le_bytes());
                t.write(p + 0xa, &(size as u16).to_le_bytes());
                p
            };
            t.w32(shader + 0x40, p);
            t.w32(e, shader);
            t.w32(e + 4, buf);
            e
        };
        t.w32(data + 4 * i, e);
    }
    (inst, list)
}

#[test]
fn p4_material_parameters_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006e_60d0);
        find_param_stub(&mut t);
        // PArray_Data: the pointer at +4 (the test's lists are never inline)
        t.stub_tail(
            p4_morph::ARRAY_DATA,
            true,
            0,
            &[0x8b, 0x41, 0x04, 0xc3], // mov eax, [ecx+4]; ret
        );
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let n = rng.below(5);
            let (inst, _) = material(&mut t, &mut rng, n, 0x40, false);
            let pname = rng.next();
            let src = if rng.chance(5) {
                0
            } else {
                let s = t.zeros(0x80);
                t.fill(&mut rng, s, 0x80);
                s
            };
            t.case(
                &what("set param"),
                0x006e_60d0,
                Conv::Cdecl,
                &[inst, pname, src],
                |g| {
                    p4_morph::bridge::set_parameter(g, inst, pname, src);
                    Out::None
                },
            );
            // the lighting's shade coefficient (4 bytes) and colour (up to
            // the five frame words)
            let cl = lighting(&mut t, &mut rng);
            let (inst, _) = material(&mut t, &mut rng, n, 4, false);
            let coeff = rng.next();
            t.zero_stack_top();
            t.case(
                &what("shade coeff"),
                0x006a_aef0,
                Conv::Thiscall(cl),
                &[inst, coeff],
                |g| {
                    let s = g.scratch();
                    p4_morph::bridge::set_shade_coeff(g, cl, inst, s);
                    Out::None
                },
            );
            let (inst, _) = material(&mut t, &mut rng, n, 0x14, false);
            let rgb = [
                float_word(&mut rng),
                float_word(&mut rng),
                float_word(&mut rng),
            ];
            t.zero_stack_top();
            t.case(
                &what("shade color"),
                0x006a_b010,
                Conv::Thiscall(cl),
                &[inst, rgb[0], rgb[1], rgb[2]],
                |g| {
                    let s = g.scratch();
                    p4_morph::bridge::set_shade_color(g, cl, inst, rgb, s);
                    Out::None
                },
            );
            // hasParameter: inline (count 1) and pointer lists, null inst
            let (inst, list) = material(&mut t, &mut rng, n, 0x10, true);
            if n == 1 {
                // one element is inline at +4
                let data: u32 = Mem::read(&t.b, list + 4);
                let e: u32 = Mem::read(&t.b, data);
                t.w32(list + 4, e);
            }
            let inst = if rng.chance(8) { 0 } else { inst };
            t.case(
                &what("has param"),
                0x006a_c4a0,
                Conv::Stdcall,
                &[inst, pname],
                |g| Out::U8(p4_morph::bridge::has_parameter(g, inst, pname)),
            );
        }
    }
}

#[test]
fn p4_morph_weights_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_c900);
        let morphs = t.stub_cell(p4_morph::MORPH_ARRAY, true, 0, 0);
        t.stub_tail(
            p4_morph::ARRAY_GET_AT,
            true,
            1,
            &[0x8b, 0x41, 0x40, 0xc2, 0x04, 0x00], // mov eax, [ecx+0x40]; ret 4
        );
        let malloc = t.stub_cell(p4_morph::PHYRE_MALLOC, false, 2, 0);
        t.stub(p4_light::PHYRE_FREE, false, 1, 0, 0x0f0f_0000);
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // setMorphWeight
            let n = rng.below(5);
            let arr = t.zeros(8);
            t.w32(arr, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
            let elems = t.zeros(4 * n.max(1) as usize);
            t.w32(arr + 4, elems);
            for k in 0..n {
                let e = if rng.chance(5) {
                    0
                } else {
                    let e = t.zeros(0x44);
                    let w = if rng.chance(5) {
                        0
                    } else {
                        let w = t.zeros(8);
                        let vals = t.zeros(8);
                        t.fill(&mut rng, vals, 8);
                        t.w32(
                            w,
                            rng.below(4) | if rng.chance(4) { 0x8000_0000 } else { 0 },
                        );
                        t.w32(w + 4, vals);
                        w
                    };
                    t.w32(e + 0x40, w);
                    e
                };
                t.w32(elems + 4 * k, e);
            }
            t.w32(morphs, arr);
            let inst = if rng.chance(8) { 0 } else { rng.next() | 1 };
            let w = float_word(&mut rng);
            t.case(
                &what("morph weight"),
                0x006a_c900,
                Conv::Stdcall,
                &[inst, w],
                |g| {
                    p4_morph::bridge::set_morph_weight(g, inst, w);
                    Out::None
                },
            );
            // addPair: present, an empty slot, or grown
            let count = rng.below(5);
            let pairs = t.zeros(8);
            let data = t.zeros(8 * count.max(1) as usize);
            t.fill(&mut rng, data, 8 * count.max(1));
            let keys: Vec<u32> = (0..count).map(|_| rng.below(6)).collect();
            for (k, key) in keys.iter().enumerate() {
                t.w32(data + 8 * k as u32, *key);
            }
            t.w32(pairs, count | if rng.chance(4) { 0x8000_0000 } else { 0 });
            t.w32(pairs + 4, data);
            let key = rng.below(6) + 1;
            let value = rng.next();
            let fresh = t.zeros(0x40);
            t.w32(malloc, if rng.chance(5) { 0 } else { fresh });
            t.case(
                &what("add pair"),
                0x006a_b8c0,
                Conv::Stdcall,
                &[pairs, key, value],
                |g| {
                    let item = g.scratch();
                    Out::U32(p4_morph::bridge::add_pair(g, pairs, key, value, item))
                },
            );
            // setWeights over the four tables
            let mm = t.zeros(0x20);
            let id = rng.below(3);
            for (table, stride) in [(0u32, 8u32), (8, 8), (0x10, 8), (0x18, 16)] {
                let n = rng.below(4);
                let d = t.zeros((stride * n.max(1)) as usize);
                t.fill(&mut rng, d, stride * n.max(1));
                for k in 0..n {
                    let obj = if rng.chance(5) {
                        0
                    } else {
                        let o = t.zeros(0x5c);
                        t.w32(o + 0x58, rng.below(3));
                        o
                    };
                    t.w32(d + stride * k, obj);
                }
                t.w32(mm + table, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
                t.w32(mm + table + 4, d);
            }
            let a = t.zeros(4);
            let b = t.zeros(4);
            let (fa, fb) = (float_word(&mut rng), float_word(&mut rng));
            t.w32(a, fa);
            t.w32(b, fb);
            t.case(
                &what("weights"),
                0x006a_bfc0,
                Conv::Thiscall(mm),
                &[id, a, b],
                |g| {
                    p4_morph::bridge::set_weights(g, mm, id, a, b);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn p4_cloth_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006e_2f70);
        t.stub(p4_cloth::OPERATOR_DELETE, false, 1, 0, 0x0de1_0000);
        t.stub(p4_cloth::MESH_INSTANCE_DTOR, true, 0, 0, 0x0d70_0000);
        let vt = t.vtable(4, 1);
        let vt2 = t.vtable(4, 2);
        let g = t.rebase(p4_cloth::G_CLOTH_ENTRIES);
        // a deleting destructor that shrinks the table (the loop rereads
        // the count after each entry it counts down)
        let mut code = vec![0xa1];
        code.extend(g.to_le_bytes()); // mov eax, [g]
        code.extend([0x84, 0xc0, 0x74, 0x06, 0x48, 0xa3]); // test al, al; jz +6; dec eax; mov [g], eax
        code.extend(g.to_le_bytes());
        code.extend([0xc2, 0x04, 0x00]); // ret 4
        let shrink = t.code(&code);
        let vt_shrink = t.zeros(0x10);
        t.w32(vt_shrink, shrink);
        for i in 0..64 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // submit
            let mesh = t.zeros(0x1a00);
            t.fill(&mut rng, mesh, 0x1a00);
            let inner = t.zeros(8);
            t.w32(inner, vt2);
            t.w32(mesh + 0xc, inner);
            let slot = rng.below(4);
            let src = t.zeros(0x40);
            t.fill(&mut rng, src, 0x40);
            let dst = t.zeros(0x40);
            t.w32(mesh + 0x810 + 4 * slot, rng.below(3));
            t.w32(mesh + 0xc10 + 4 * slot, src);
            t.w32(mesh + 0x210 + 4 * slot, dst);
            t.w32(mesh + 0x1810 + 4 * slot, rng.below(0x41));
            let inst = t.zeros(0x40);
            t.fill(&mut rng, inst, 0x40);
            t.w32(inst + 8, 4 * rng.below(8));
            let job = t.zeros(0x24);
            t.fill(&mut rng, job, 0x24);
            t.w32(job + 0x20, slot);
            t.case(
                &what("submit"),
                0x006e_28f0,
                Conv::Thiscall(mesh),
                &[inst, job],
                |g| Out::U32(p4_cloth::bridge::submit(g, mesh, inst, job)),
            );
            // releaseExpired
            let n = rng.below(6);
            let table = t.zeros(16 * n.max(1) as usize);
            for k in 0..n {
                let e = table + 16 * k;
                for (field, child) in [(0u32, 0x1cu32), (4, 0xc)] {
                    let obj = if rng.chance(4) {
                        0
                    } else {
                        let o = t.zeros(0x20);
                        t.w32(o, if rng.chance(4) { vt_shrink } else { vt });
                        let c = if rng.chance(3) {
                            0
                        } else {
                            let c = t.zeros(4);
                            t.w32(c, vt);
                            c
                        };
                        t.w32(o + child, c);
                        o
                    };
                    t.w32(e + field, obj);
                }
                t.w32(e + 8, u32::from(rng.chance(3)));
                t.w32(
                    e + 0xc,
                    [0, 0, 1, 5, 0xffff_ffff, 0x8000_0000][rng.below(6) as usize],
                );
            }
            t.w32(g, n | if rng.chance(4) { 0x8000_0000 } else { 0 });
            t.w32(g + 4, table);
            t.case(&what("release"), 0x006e_2f70, Conv::Cdecl, &[], |g| {
                p4_cloth::bridge::release_expired(g);
                Out::None
            });
            // the instance dtor: some buffers set
            let inst = t.zeros(0x1e10);
            for k in 0..rng.below(24) {
                let at = [0x1c10, 0x10, 0x210, 0x410][(k % 4) as usize] + 4 * rng.below(0x80);
                let v = rng.next() | 1;
                t.w32(inst + at, v);
            }
            t.case(&what("dtor"), 0x006e_0ab0, Conv::Thiscall(inst), &[], |g| {
                p4_cloth::bridge::instance_dtor(g, inst);
                Out::None
            });
            // the dynamic meshes' buffer copies
            let n = rng.below(5);
            let srcm = t.zeros(0x28);
            t.fill(&mut rng, srcm, 0x28);
            t.w32(srcm + 0x20, n);
            let idx = rng.below(6);
            t.w32(srcm + 0x24, idx);
            for at in [0x10, 0x14, 0x18, 0x1c] {
                let b = t.zeros(0x50);
                t.fill(&mut rng, b, 0x50);
                t.w32(srcm + at, b);
            }
            let outs = t.zeros(12);
            for k in 0..3 {
                let o = if rng.chance(4) && n == 0 {
                    0
                } else {
                    let o = t.zeros(0x40);
                    let b = t.zeros(0x50);
                    t.w32(o + 0x3c, b);
                    o
                };
                t.w32(outs + 4 * k, o);
            }
            let meshb = t.zeros(0x3c);
            let ib = t.zeros(0x20);
            t.w32(meshb + 0x38, ib);
            let sym = if i % 2 == 0 { 0x0067_4570 } else { 0x0067_7b10 };
            let x = rng.next();
            t.case(
                &what("copy"),
                sym,
                Conv::Stdcall,
                &[x, srcm, outs, meshb],
                |g| Out::U32(p4_cloth::bridge::copy_buffers(g, srcm, outs, meshb)),
            );
        }
    }
}

#[test]
fn p4_iterators_and_latches_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_8d30);
        // valid(it): 1 without a node, else the node's byte +0x20
        t.stub_tail(
            p4_light::ITERATOR_VALID,
            true,
            0,
            &[
                0x8b, 0x41, 0x18, // mov eax, [ecx+0x18]
                0x85, 0xc0, // test eax, eax
                0x74, 0x04, // jz one
                0x8a, 0x40, 0x20, // mov al, [eax+0x20]
                0xc3, // ret
                0xb0, 0x01, // one: mov al, 1
                0xc3,
            ],
        );
        t.stub(p4_light::CURSOR_SET_NODE, true, 1, 1, 0x05e7_0000);
        for i in 0..96 {
            // a chain of nodes ending at the end marker or 0
            let it = t.zeros(0x20);
            t.fill(&mut rng, it, 0x20);
            let end = t.zeros(0x24);
            t.w32(it + 0x14, end);
            let n = rng.below(5);
            let mut next = if rng.chance(2) { end } else { 0 };
            for _ in 0..n {
                let node = t.zeros(0x24);
                t.w32(node, next);
                t.write(node + 0x20, &[u8::from(rng.chance(3))]);
                next = node;
            }
            t.w32(it + 0x18, next);
            let (sym, first) = if i % 2 == 0 {
                (0x006a_8d30, true)
            } else {
                (0x006a_8da0, false)
            };
            t.case(
                &format!("{name} iterator case {i}"),
                sym,
                Conv::Thiscall(it),
                &[],
                |g| Out::U32(p4_light::bridge::iterator_advance(g, it, first)),
            );
        }
        // devices whose vtable[3](id) gives a mixed value of id and the
        // device (0..3)
        let get = t.code(&[
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
            0x31, 0xc8, // xor eax, ecx
            0x69, 0xc0, 0xb1, 0x79, 0x37, 0x9e, // imul eax, eax, 0x9e3779b1
            0xc1, 0xe8, 0x1e, // shr eax, 30
            0xc2, 0x04, 0x00, // ret 4
        ]);
        let vt = t.zeros(0x10);
        t.w32(vt + 0xc, get);
        for i in 0..24 {
            let mapper = t.zeros(0xa0);
            t.fill(&mut rng, mapper, 0xa0);
            for k in 0..25 {
                let at = if k < 7 {
                    0x80 + 4 * k
                } else {
                    0x38 + 4 * (k - 7)
                };
                let dev = if rng.chance(4) {
                    0
                } else {
                    let d = t.zeros(0x220);
                    t.fill(&mut rng, d, 0x220);
                    for b in 0..0x100 {
                        t.write(d + 0x118 + b, &[rng.below(4) as u8]);
                    }
                    for b in 0..0x10 {
                        t.write(d + 0x28 + b, &[rng.below(4) as u8]);
                    }
                    t.w32(d, vt);
                    d
                };
                t.w32(mapper + at, dev);
            }
            t.case(
                &format!("{name} latch case {i}"),
                0x0062_8320,
                Conv::Thiscall(mapper),
                &[],
                |g| {
                    p4_input::bridge::latch_buttons(g, mapper);
                    Out::None
                },
            );
        }
    }
}

// ---- continuation round (c1) batch 3: reflection registrations ----

use ffx_pother_b::p4_bind::{self, Bind, Step};

/// Every callee a `Bind` reaches, as recording stubs; returns the
/// `findMember` result cell.
fn bind_stubs(t: &mut Twins) -> u32 {
    t.stub(p4_bind::REGISTER_TYPE, false, 1, 0, 0x0e60_0000);
    t.stub(p4_bind::ADD_TO_NAMESPACE, true, 0, 0, 0x0a2d_0000);
    t.stub(p4_bind::DATA_MEMBER_CTOR, true, 6, 6, 0x0da7_0000);
    // seven words: the stub logs the first six
    t.stub(p4_bind::ARRAY_MEMBER_CTOR, true, 6, 7, 0x0a77_0000);
    t.stub(p4_bind::METHOD_MEMBER_CTOR, true, 3, 3, 0x0ae7_0000);
    t.stub(p4_bind::ATEXIT, false, 1, 0, 0x0a7e_0000);
    let found = t.stub_cell(p4_bind::FIND_MEMBER, true, 1, 1);
    for (k, f) in [
        0x009f_4730u32,
        0x0043_7fc0,
        0x0043_8390,
        0x0043_8060,
        0x00a2_dce0,
        0x00a2_e150,
        0x00a2_e5c0,
        0x00a2_ea30,
    ]
    .into_iter()
    .enumerate()
    {
        t.stub(f, false, 0, 0, 0x07e0_0000 + k as u32);
    }
    for f in [0x009d_fbd0, 0x009d_fb30, 0x00a3_2ef0] {
        t.stub(f, true, 0, 0, 0x0c70_0000);
    }
    for f in [0x009e_5220, 0x009e_5010, 0x00a3_3680] {
        t.stub(f, false, 1, 0, 0x0dec_0000);
    }
    found
}

/// Random guard bits and member words for `b`'s globals.
fn bind_state(t: &mut Twins, rng: &mut Rng, b: &Bind) {
    let guard = t.rebase(b.guard);
    let g = rng.next() & 0xf | (rng.next() & 0xffff_0000);
    t.w32(guard, g);
    for step in b.steps {
        let (member, inner) = match *step {
            Step::Field { member, .. } | Step::Method { member, .. } => (member, None),
            Step::Array { member, inner, .. } => (member, Some(inner.obj)),
            Step::Call { arg, .. } => {
                let a = t.rebase(arg);
                let v = rng.next();
                t.w32(a, v);
                continue;
            }
        };
        let m = t.rebase(member);
        t.fill(rng, m, 0x2c);
        if let Some(obj) = inner {
            let obj = t.rebase(obj);
            let w = rng.next();
            t.w32(obj + 0x94, w);
            let w = rng.next();
            t.w32(obj + 0x90, w);
        }
        if let Step::Method { .. } = *step {
            // the class (+8) with a member list at +0x4c, a name or none
            let class = t.zeros(0x58);
            let last = t.zeros(8);
            t.w32(class + 0x50, last);
            t.w32(m + 8, class);
            let name = if rng.chance(4) { 0 } else { rng.next() | 1 };
            t.w32(m + 0xc, name);
        }
    }
}

const BINDS: &[(u32, &Bind)] = &[
    (0x009e_4430, &p4_bind::INPUT_ACTION),
    (0x009e_46d0, &p4_bind::INPUT_MAP),
    (0x009e_47c0, &p4_bind::INPUT_SOURCE),
    (0x009e_4940, &p4_bind::JOYPAD_AXIS),
    (0x009e_49e0, &p4_bind::JOYPAD_BUTTON),
    (0x009e_4ac0, &p4_bind::SOURCE_KEY),
    (0x009e_4ce0, &p4_bind::MOUSE_BUTTON),
    (0x009e_4d80, &p4_bind::MOUSE_BUTTON_KEY),
    (0x009e_4e60, &p4_bind::MOUSE_BUTTON_KEY_2),
    (0x00a3_3540, &p4_bind::GAME_SETTINGS),
    (0x0061_7960, &p4_bind::ASYNC_PROCESS_HEADER),
    (0x0061_95a0, &p4_bind::CALLABLE_METHOD_SCRIPT),
];

#[test]
fn p4_binds_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x009e_4430);
        let found = bind_stubs(&mut t);
        for i in 0..12 {
            for &(sym, b) in BINDS {
                bind_state(&mut t, &mut rng, b);
                t.w32(found, if rng.chance(3) { rng.next() } else { 0 });
                t.case(
                    &format!("{name} bind {sym:#x} case {i}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(p4_bind::bridge::bind(g, b)),
                );
            }
        }
    }
}

#[test]
fn p4_array_declarations_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x009e_5220);
        // the callees (not bind_stubs: it stubs these two declarations)
        t.stub(p4_bind::REGISTER_TYPE, false, 1, 0, 0x0e60_0000);
        t.stub(p4_bind::ADD_TO_NAMESPACE, true, 0, 0, 0x0a2d_0000);
        t.stub(p4_bind::DATA_MEMBER_CTOR, true, 6, 6, 0x0da7_0000);
        t.stub(p4_bind::ARRAY_MEMBER_CTOR, true, 6, 7, 0x0a77_0000);
        t.stub(p4_bind::ATEXIT, false, 1, 0, 0x0a7e_0000);
        for f in [0x009d_fbd0, 0x009d_fb30] {
            t.stub(f, true, 0, 0, 0x0c70_0000);
        }
        t.stub(p4_bind::TYPE_UINT, false, 0, 0, 0x07e0_1111);
        t.stub(p4_bind::ANNOTATION_CTOR, true, 3, 3, 0x0a00_0000);
        t.stub(p4_bind::ADD_ANNOTATION, true, 1, 1, 0x0add_0000);
        for i in 0..48 {
            for (sym, d) in [
                (0x009e_5010, &p4_bind::ARRAY_INPUT_ACTION),
                (0x009e_5220, &p4_bind::ARRAY_INPUT_SOURCE),
            ] {
                let desc = t.rebase(d.inner.obj);
                t.fill(&mut rng, desc, 0x130);
                t.w32(desc + 0x94, rng.next());
                if rng.chance(2) {
                    t.w32(desc + 0x18, 0);
                }
                // the annotation unlinked (its +4 at itself) or linked
                if rng.chance(2) {
                    t.w32(desc + 0xc8, desc + 0xc8);
                }
                let g = t.rebase(d.name_global);
                let v = if rng.chance(2) { 0 } else { rng.next() };
                t.w32(g, v);
                let arg = if rng.chance(6) { 0 } else { rng.next() };
                t.case(
                    &format!("{name} array decl {sym:#x} case {i}"),
                    sym,
                    Conv::Cdecl,
                    &[arg],
                    |g| Out::U32(p4_bind::bridge::array_decl(g, d, arg)),
                );
            }
        }
    }
}

#[test]
fn p4_descriptor_ctors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x009d_fbd0);
        t.stub(p4_bind::BASE_CLASS_DESCRIPTOR, false, 0, 0, 0x0bcd_0000);
        t.stub(p4_bind::GLOBAL_NAMESPACE, false, 5, 0, 0x0a5e_0000);
        t.stub(p4_bind::CLASS_DESCRIPTOR_CTOR, true, 6, 6, 0x0c1d_0000);
        t.stub(p4_bind::SET_DEFAULT_BUFFER, true, 1, 1, 0x0def_0000);
        t.stub(p4_bind::SET_VALIDATION_BUFFER, true, 1, 1, 0x0fa1_0000);
        t.stub(p4_bind::SET_WRITE_MASK_BUFFER, true, 1, 1, 0x0a5c_0000);
        for i in 0..32 {
            for (sym, c) in [
                (0x009d_fbd0, &p4_bind::CTOR_INPUT_SOURCE_ARRAY),
                (0x00a3_2ef0, &p4_bind::CTOR_INPUT_MAP_ARRAY),
            ] {
                let desc = t.zeros(0xa0);
                t.fill(&mut rng, desc, 0xa0);
                let g = t.rebase(c.name_global);
                let v = rng.next();
                t.w32(g, v);
                t.case(
                    &format!("{name} descriptor ctor {sym:#x} case {i}"),
                    sym,
                    Conv::Thiscall(desc),
                    &[],
                    |g| Out::U32(p4_bind::bridge::descriptor_ctor(g, c, desc)),
                );
            }
        }
    }
}

#[test]
fn p4_startup_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_9a40);
        for &f in &p4_bind::UTILITY_BINDS {
            t.stub(f, false, 0, 0, 0x0b1d_0000);
        }
        for (k, &f) in p4_bind::UTILITY_TYPES.iter().enumerate() {
            t.stub(f, false, 0, 0, 0x07e0_0000 + k as u32);
        }
        t.stub(p4_bind::REGISTER_TYPE, false, 1, 0, 0x0e60_0000);
        t.case(
            &format!("{name} utility inputs"),
            0x009f_4650,
            Conv::Cdecl,
            &[],
            |g| Out::U32(p4_bind::startup::utility_inputs_initialize(g)),
        );
        t.stub(p4_bind::STARTUP_FIRST, false, 1, 0, 0x05f1_0000);
        let init = t.stub_cell(p4_bind::PHYRE_INIT, false, 0, 0);
        let app_init = t.stub_cell(p4_bind::INIT_WINDOW_AND_APP, true, 2, 2);
        // the import slot points at a recording stdcall stub
        let st = t.stub_code(0x5e75_0000, false, 1, 1, 0x0001_0000);
        let slot = t.rebase(p4_bind::SET_THREAD_EXECUTION_STATE);
        t.w32(slot, st);
        for i in 0..16 {
            t.w32(init, [0, 0, 1, rng.next()][rng.below(4) as usize]);
            t.w32(app_init, [0, 0, 1, rng.next()][rng.below(4) as usize]);
            for a in [0x00cc_9cec, 0x00cc_9cf0, 0x00cc_9cd0] {
                let a = t.rebase(a);
                let v = rng.next();
                t.w32(a, v);
            }
            let (app, argv, argc) = (rng.next(), rng.next(), rng.below(5));
            t.case(
                &format!("{name} startup case {i}"),
                0x0062_9a40,
                Conv::Thiscall(app),
                &[argv, argc],
                |g| Out::U32(p4_bind::startup::startup(g, app, argv, argc)),
            );
        }
    }
}

// ---- continuation round (c1) batch 4: sticks, pads, the lighting's flash ----

#[test]
fn p4_sticks_and_pads_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_8470);
        for i in 0..240 {
            let mapper = t.zeros(0x90);
            t.fill(&mut rng, mapper, 0x90);
            let pad = rng.below(18);
            let dev = if rng.chance(8) {
                0
            } else {
                let d = t.zeros(0x50);
                t.fill(&mut rng, d, 0x50);
                for k in 0..4 {
                    let m = t.zeros(0x20);
                    let v = match rng.below(3) {
                        0 => float_word(&mut rng),
                        _ => (rng.below(200) as f32 / 100.0 - 1.0).to_bits(),
                    };
                    let th =
                        [0x3e4c_cccd, 0, 0x3f00_0000, float_word(&mut rng)][rng.below(4) as usize];
                    // sometimes exactly at the threshold (either sign)
                    let v = if rng.chance(4) {
                        th ^ (rng.below(2) << 31)
                    } else {
                        v
                    };
                    t.w32(m + 0x1c, v);
                    t.w32(m + 0x18, th);
                    t.w32(d + 0x38 + 4 * k, m);
                }
                // the words after the four mappings: 0 or a mapping
                let m = t.zeros(0x20);
                t.w32(m + 0x1c, float_word(&mut rng));
                t.w32(d + 0x48, if rng.chance(2) { 0 } else { m });
                t.w32(d + 0x4c, if rng.chance(2) { 0 } else { m });
                d
            };
            t.w32(mapper + 0x38 + 4 * pad, dev);
            let axis = rng.below(6);
            t.case(
                &format!("{name} stick case {i}"),
                0x0062_8470,
                Conv::Thiscall(mapper),
                &[axis, pad],
                |g| Out::F32(p4_input::bridge4::get_stick_axis(g, mapper, axis, pad)),
            );
        }
        // attachPads: a device list with pads (kind 2) and others
        let head = t.rebase(p4_input::DEVICE_LIST);
        let kind = t.code(&[0x8b, 0x41, 0x40, 0xc3]); // mov eax, [ecx+0x40]; ret
        let bind = t.stub_code(0x0b1d_0000, true, 2, 2, 0);
        let vt = t.zeros(0x10);
        t.w32(vt + 4, kind);
        t.w32(vt + 8, bind);
        for i in 0..48 {
            let mapper = t.zeros(0x90);
            t.fill(&mut rng, mapper, 0x90);
            for p in 0..18 {
                if rng.chance(2) {
                    t.w32(mapper + 0x38 + 4 * p, 0);
                }
            }
            let n = rng.below(24);
            let mut prev = head;
            for _ in 0..n {
                let d = t.zeros(0x130);
                t.w32(d, vt);
                t.w32(d + 0x40, if rng.chance(3) { 1 } else { 2 });
                t.w32(prev, d + 4);
                prev = d + 4;
            }
            // the list ends at the head, at 0, or at a link of 4
            t.w32(prev, [head, head, 0, 4][rng.below(4) as usize]);
            t.case(
                &format!("{name} attach pads case {i}"),
                0x0062_6aa0,
                Conv::Thiscall(mapper),
                &[],
                |g| {
                    p4_input::bridge4::attach_pads(g, mapper);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn p4_lighting_flash_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_b630);
        let found = t.zeros(4);
        // the import slot: a stub returning [found]
        let mut code = vec![0xa1];
        code.extend(found.to_le_bytes()); // mov eax, [found]
        code.push(0xc3);
        let ret_found = t.code(&code);
        let slot = t.rebase(p4_light::STRSTR);
        t.w32(slot, ret_found);
        t.stub(p4_light::PLIGHT_ALLOCATE, false, 1, 0, 0x0a11_0000);
        t.stub(p4_light::PLIGHT_DEFINE, false, 6, 0, 0x0def_0000);
        t.stub(p4_light::PLIGHT_SET_COLOR, false, 4, 0, 0x5c01_0000);
        t.stub(p4_light::SET_POINT_LIGHT, true, 4, 4, 0x0901_0000);
        // the level (cdecl, three words) returns [level] in ST0
        let level = t.zeros(4);
        let mut tail = vec![0xd9, 0x05];
        tail.extend(level.to_le_bytes()); // fld dword [level]
        tail.push(0xc3);
        t.stub_tail(p4_light::FLASH_LEVEL, false, 3, &tail);
        for i in 0..160 {
            let cl = t.zeros(0x240);
            t.fill(&mut rng, cl, 0x240);
            t.write(cl + 0x1cc, &[u8::from(!rng.chance(4))]);
            let flash = match rng.below(4) {
                0 => float_word(&mut rng),
                1 => (rng.below(2000) as f32 / 1.9).to_bits(),
                2 => [0, 0x8000_0000, 0x3c23_d70a, 0x447a_0000, 0xbf80_0000][rng.below(5) as usize],
                _ => (rng.below(100) as f32 / 7.0).to_bits(),
            };
            t.w32(cl + 0x154, flash);
            let lv = match rng.below(3) {
                0 => float_word(&mut rng),
                _ => (rng.below(1000) as f32 / 999.0).to_bits(),
            };
            t.w32(level, lv);
            // a flash that lands exactly on 1000
            if rng.chance(6) {
                t.w32(cl + 0x154, 0x447a_0000);
            }
            let dt = match rng.below(4) {
                0 => float_word(&mut rng),
                1 => 0,
                _ => (rng.below(100) as f32 / 60.0).to_bits(),
            };
            t.case(
                &format!("{name} flash case {i}"),
                0x006a_b630,
                Conv::Thiscall(cl),
                &[dt],
                |g| {
                    p4_light::bridge4::update_flash(g, cl, dt);
                    Out::None
                },
            );
            let map = if rng.chance(5) { 0 } else { rng.next() | 1 };
            t.w32(found, if rng.chance(2) { 0 } else { rng.next() | 1 });
            t.case(
                &format!("{name} map name case {i}"),
                0x006a_b0d0,
                Conv::Thiscall(cl),
                &[map],
                |g| {
                    p4_light::bridge4::set_map_name(g, cl, map);
                    Out::None
                },
            );
        }
    }
}

// ---- continuation round 2 (c2) batch 1: dynamic meshes, shader cluster
// loads, tree inserts, the point light's position, the mouse ----

use ffx_pother_b::{p4_frames, p4_mesh};

impl Twins {
    /// Where the original's `ebp` is for a call with `words` stack words
    /// (the call pushes them below `stack_range().end - 0x100`, then the
    /// return address; the original pushes `ebp`).
    fn ebp(&self, words: u32) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * words - 8
    }

    /// [`Twins::case`] for an original whose frame callees see: the port
    /// gets `ebp` (the original's), its oracle's calls start below
    /// `below` bytes of frame, and after the call the frame words at
    /// `offsets` (from `ebp`) must agree.
    #[allow(clippy::too_many_arguments)]
    fn frame_case(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        below: u32,
        offsets: &[i32],
        port: impl FnOnce(&mut G, u32) -> Out,
    ) {
        self.frame_case_fill(0x5a, what, sym, conv, args, below, offsets, port);
    }

    /// [`Twins::frame_case`] with both stacks filled with `fill` first: 0
    /// for originals that read frame words they never write (the port
    /// uses 0 there, brief 2026-10-24).
    #[allow(clippy::too_many_arguments)]
    fn frame_case_fill(
        &mut self,
        fill: u8,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        below: u32,
        offsets: &[i32],
        port: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let ebp = self.ebp(args.len() as u32);
        // the same pattern in both frames, so words neither writes agree
        // and a missing store shows
        let end = self.a.stack_range().end;
        self.a.write(end - 0x1000, &[fill; 0x1000]).unwrap();
        self.b.write(end - 0x1000, &[fill; 0x1000]).unwrap();
        self.b.set_call_top(Some(ebp - below - 0x100));
        self.case(what, sym, conv, args, |g| port(g, ebp));
        self.b.set_call_top(None);
        for &off in offsets {
            let at = ebp.wrapping_add_signed(off);
            let x: u32 = Mem::read(&self.a, at);
            let y: u32 = Mem::read(&self.b, at);
            assert_eq!(x, y, "{what}: frame word ebp{off:+#x}");
        }
    }

    /// A recording stub over `sym` (cdecl, `n` arguments) handing out
    /// 0x40-byte blocks from `[cell]` on, or 0 while the cell is 0.
    fn stub_bump(&mut self, sym: u32, n: u32) -> u32 {
        let cell = self.zeros(4);
        let mut tail = vec![0xa1];
        tail.extend(cell.to_le_bytes()); // mov eax, [cell]
        tail.extend([0x85, 0xc0, 0x74, 0x0a]); // test eax, eax; jz ret
        tail.extend([0x81, 0x05]);
        tail.extend(cell.to_le_bytes());
        tail.extend(0x40u32.to_le_bytes()); // add dword [cell], 0x40
        tail.push(0xc3);
        self.stub_tail(sym, false, n, &tail);
        cell
    }
}

#[test]
fn p4c2_dynamic_meshes_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0067_6680);
        let malloc = t.stub_bump(p4_mesh::PHYRE_MALLOC, 2);
        t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0, 0x0f0f_0f0f);
        let alloc = t.stub_cell(p4_mesh::ALLOCATE_INSTANCE_LIST, true, 3, 3);
        t.stub(p4_mesh::DISTORTION_CTOR, true, 1, 1, 0x0d15_0000);
        t.stub(p4_mesh::RADIAL_CTOR, true, 1, 1, 0x0ad1_0000);
        let buffer_id = t.stub_cell(p4_mesh::CURRENT_BUFFER_ID, false, 0, 0);
        t.stub(p4_mesh::RESIZE16, true, 1, 1, 0);
        t.stub(p4_mesh::MESH_INSTANCE_CTOR, true, 1, 1, 0x0e5e_0000);
        t.stub(p4_mesh::MEMSET, false, 3, 0, 0x0e3e_0000);
        let buffers = t.rebase(p4_mesh::DYN_GEO_BUFFERS);
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // the stream tables: counts that match, don't, or aren't owned
            for sym in [0x0067_6680, 0x0067_7ce0] {
                let mesh = t.zeros(0x20);
                t.fill(&mut rng, mesh, 0x20);
                let draw = t.zeros(0x10);
                let streams = t.zeros(0x24);
                let owned = |rng: &mut Rng| if rng.chance(3) { 0x8000_0000 } else { 0 };
                let dn = [1, 0, 2][rng.below(3) as usize] | owned(&mut rng);
                let sn = [3, 0, 1][rng.below(3) as usize] | owned(&mut rng);
                t.w32(mesh + 0x10, dn);
                t.w32(mesh + 0x14, if rng.chance(6) { 0 } else { draw });
                t.w32(mesh + 8, sn);
                t.w32(mesh + 0xc, if rng.chance(6) { 0 } else { streams });
                // without a block the old pointers stay, so they must be real
                let block = if rng.chance(4) { 0 } else { t.zeros(0x100) };
                if block == 0 || dn == 1 {
                    t.w32(mesh + 0x14, draw);
                }
                if block == 0 || sn == 3 {
                    t.w32(mesh + 0xc, streams);
                }
                // the allocator can hand back the storage a table has
                if block != 0 && rng.chance(4) {
                    t.w32(mesh + 0x14, block);
                }
                t.w32(malloc, block);
                let [a, b, c] = [rng.next(), rng.next(), rng.next()];
                t.case(
                    &what(&format!("streams {sym:#x}")),
                    sym,
                    Conv::Thiscall(mesh),
                    &[a, b, c],
                    |g| Out::U32(p4_mesh::bridge::set_streams(g, mesh, Some(a), b, c)),
                );
            }
            // one instance: no list, no storage, or both
            for (desc, ctor, sym) in [
                (
                    p4_mesh::DISTORTION_DESC,
                    p4_mesh::DISTORTION_CTOR,
                    0x0067_3e20,
                ),
                (p4_mesh::RADIAL_DESC, p4_mesh::RADIAL_CTOR, 0x0067_5d90),
            ] {
                let list = match rng.below(3) {
                    0 => 0,
                    k => {
                        let l = t.zeros(0x40);
                        t.fill(&mut rng, l, 0x40);
                        if k == 1 {
                            t.w32(l + 0x34, 0);
                        }
                        l
                    }
                };
                t.w32(alloc, list);
                let p = t.zeros(4);
                t.fill(&mut rng, p, 4);
                let cluster = rng.next();
                t.case(
                    &what(&format!("create {sym:#x}")),
                    sym,
                    Conv::Thiscall(cluster),
                    &[p],
                    |g| Out::U32(p4_mesh::bridge::create_instance(g, cluster, desc, ctor, p)),
                );
            }
            // the draw table: buffer ids, buffers on or off, empty entries
            let n: u32 = [0, 1, 2, 5, 9, 0xffff_fffe][rng.below(6) as usize];
            let real = if (n as i32) > 0 { n } else { 0 };
            let id: u32 = [0xffff_ffff, 0, 1, 2, 0x8000_0000][rng.below(5) as usize];
            t.w32(buffer_id, id);
            t.w32(buffers, u32::from(rng.chance(3)));
            let src = t.zeros(0x98);
            t.fill(&mut rng, src, 0x98);
            t.w32(src + 0x90, n);
            let len = real * 0x6c + 3 * 0x1c + 0x40;
            let recs = t.zeros(len as usize);
            t.fill(&mut rng, recs, len);
            for k in 0..real {
                let r = recs + k * 0x6c;
                for at in [4, 0x34, 0x50, 0x6c, 0x2c, 0x48, 0x64] {
                    if rng.chance(3) {
                        t.w32(r + at, 0);
                    }
                }
            }
            t.w32(src + 0x94, recs);
            let mesh = t.zeros(0x20);
            t.fill(&mut rng, mesh, 0x20);
            let table = t.zeros((real * 0x10 + 0x10) as usize);
            t.w32(mesh + 0x14, table);
            let mut slot = 0;
            t.case(
                &what("draw table"),
                0x0067_32a0,
                Conv::Thiscall(mesh),
                &[src],
                |g| {
                    slot = p4_mesh::bridge::fill_draw_table(g, mesh, src);
                    Out::None
                },
            );
            let at = t.a.stack_range().end - 0x100 - 4;
            assert_eq!(Mem::read::<u32>(&t.a, at), slot, "{}", what("slot"));
            // the cloth instance
            let inst = t.zeros(0x2010);
            t.fill(&mut rng, inst, 0x2010);
            let m = rng.next();
            t.case(
                &what("cloth instance"),
                0x006e_0860,
                Conv::Thiscall(inst),
                &[m],
                |g| Out::U32(p4_mesh::bridge::cloth_instance_ctor(g, inst, m)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: nothing logged");
    }
}

#[test]
fn p4c2_frames_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006d_f3d0);
        t.stub(p4_frames::CLUSTER_MANAGER, false, 1, 0, 0x3a3a_0000);
        let cluster = t.stub_cell(p4_frames::LOAD_PCLUSTER, true, 1, 1);
        t.stub(p4_frames::GET_APPLICATION, false, 0, 0, 0x0a99_0000);
        // the fix-up moves the cluster to [moved] when that is set
        let moved = t.zeros(4);
        let mut tail = vec![0xa1];
        tail.extend(moved.to_le_bytes()); // mov eax, [moved]
        tail.extend([0x85, 0xc0, 0x74, 0x06]); // test eax, eax; jz ret
        tail.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
        tail.extend([0x89, 0x01]); // mov [ecx], eax
        tail.push(0xc3);
        t.stub_tail(p4_frames::FIXUP_CLUSTERS, false, 2, &tail);
        // the seek fills the cursor's +0, +0xc, +0x10 and returns [r]
        let cells = t.zeros(16);
        let mut tail = Vec::new();
        for (k, at) in [(0u32, 0u8), (4, 0xc), (8, 0x10)] {
            tail.push(0xa1);
            tail.extend((cells + k).to_le_bytes()); // mov eax, [cell k]
            tail.extend([0x89, 0x41, at]); // mov [ecx+at], eax
        }
        tail.push(0xa1);
        tail.extend((cells + 12).to_le_bytes()); // mov eax, [r]
        tail.push(0xc3);
        t.stub_tail(p4_frames::CURSOR_SEEK3, true, 0, &tail);
        t.stub(p4_frames::TREE_INSERT, true, 4, 4, 0x07ee_0000);
        // the string copy sets its capacity (+0x14) and buffer from cells
        let s_cells = t.zeros(8);
        let mut tail = vec![0xa1];
        tail.extend(s_cells.to_le_bytes()); // mov eax, [cap]
        tail.extend([0x85, 0xc0, 0x74, 0x03]); // test eax, eax; jz keep
        tail.extend([0x89, 0x41, 0x14]); // mov [ecx+0x14], eax
        tail.push(0xa1);
        tail.extend((s_cells + 4).to_le_bytes()); // mov eax, [buf]
        tail.extend([0x89, 0x01]); // mov [ecx], eax
        tail.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        t.stub_tail(p4_frames::STRING_ASSIGN, true, 3, &tail);
        t.stub(p4_frames::PAIR_INSERT, true, 2, 2, 0x0a1e_0000);
        t.stub(p4_frames::OPERATOR_DELETE, false, 1, 0, 0);
        let flag = t.rebase(p4_frames::HANDLER_FLAG);
        let cursor: Vec<i32> = (0..8).map(|k| -0x20 + 4 * k).collect();
        for i in 0..64 {
            let what = |s: &str| format!("{name} {s} case {i}");
            for (sym, c) in [
                (0x006d_f3d0, p4_frames::BTM_TO_TOP),
                (0x006d_f470, p4_frames::MAGIC_249),
                (0x006d_f510, p4_frames::MAGIC_297),
            ] {
                let cl = if rng.chance(4) {
                    0
                } else {
                    let cl = t.zeros(0x40);
                    t.fill(&mut rng, cl, 0x40);
                    if rng.chance(3) {
                        t.w32(cl + 0x1c, cl + 0x1c);
                    }
                    cl
                };
                t.w32(cluster, cl);
                let other = if rng.chance(3) {
                    let o = t.zeros(0x40);
                    t.fill(&mut rng, o, 0x40);
                    o
                } else {
                    0
                };
                t.w32(moved, other);
                for k in 0..4 {
                    let v = if k == 1 && rng.chance(3) {
                        0
                    } else {
                        rng.next()
                    };
                    t.w32(cells + 4 * k, v);
                }
                t.frame_case(
                    &what(&format!("cluster {sym:#x}")),
                    sym,
                    Conv::Cdecl,
                    &[],
                    0x20,
                    &cursor,
                    |g, ebp| Out::U32(p4_frames::bridge::load_shader_cluster(g, ebp, &c)),
                );
            }
            let w = rng.next();
            t.w32(flag, w);
            let key = rng.next();
            t.frame_case(
                &what("handler"),
                0x0062_5c70,
                Conv::Cdecl,
                &[key],
                8,
                &[-8, -4, 8],
                |g, ebp| Out::U32(p4_frames::bridge::insert_handler(g, ebp, key)),
            );
            let cap = [0xf, 0x10, 0x1f, 0][rng.below(4) as usize];
            t.w32(s_cells, cap);
            t.w32(s_cells + 4, rng.next());
            let value = t.zeros(4);
            t.fill(&mut rng, value, 4);
            let [map, nm] = [rng.next(), rng.next()];
            t.frame_case(
                &what("named"),
                0x0064_bde0,
                Conv::Cdecl,
                &[map, nm, value],
                0x24,
                &[-0x24, -0x20, -0x1c, -0x18, -0x14, -0x10, -0xc, -8],
                |g, ebp| Out::U32(p4_frames::bridge::insert_named(g, ebp, map, nm, value)),
            );
        }
    }
}

#[test]
fn p4c2_point_light_and_mouse_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_ad90);
        t.stub(p4_input::EHVEC_CTOR, false, 5, 5, 0x0e4e_0000);
        let head = t.rebase(p4_input::DEVICE_LIST);
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            let cl = lighting(&mut t, &mut rng);
            let on = Mem::read::<u8>(&t.b, cl + 0x1cc) != 0;
            let xyz = [
                float_word(&mut rng),
                float_word(&mut rng),
                float_word(&mut rng),
            ];
            let li = rng.below(5);
            t.case(
                &what("point light position"),
                0x006a_ad90,
                Conv::Thiscall(cl),
                &[xyz[0], xyz[1], xyz[2], li],
                |g| {
                    let m = g.scratch();
                    let r = p4_light::bridge5::set_light_pos(g, cl, xyz, li, m);
                    if on { Out::U32(r) } else { Out::None }
                },
            );
            let last = t.zeros(8);
            t.fill(&mut rng, last, 8);
            t.w32(head + 4, last);
            let dev = t.zeros(0x120);
            t.fill(&mut rng, dev, 0x120);
            let state = rng.next();
            t.case(
                &what("mouse"),
                0x0062_00d0,
                Conv::Thiscall(dev),
                &[state],
                |g| Out::U32(p4_input::bridge5::mouse_ctor(g, dev, state)),
            );
        }
    }
}

// ---- c2 batch 2: the scripting bind (in BINDS), UTF-8 to UTF-16, the
// keyboard, cloth release queueing, the string tree, setLightDir ----

use ffx_pother_b::p4_tree;

impl Twins {
    /// Recording code (as [`Twins::stub_tail`]) that is not patched over
    /// an original, for vtables; returns its address.
    fn logged(&mut self, tag: u32, this: bool, n: u32, tail: &[u8]) -> u32 {
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
        c.extend(tail);
        self.code(&c)
    }
}

/// A UTF-8 test string: ASCII, valid 2-4 byte sequences, encoded
/// surrogates, code points past U+10FFFF, stray continuation bytes.
fn utf8_text(rng: &mut Rng) -> Vec<u8> {
    let mut s = Vec::new();
    for _ in 0..rng.below(12) {
        match rng.below(12) {
            0..=2 => s.push(0x20 + rng.below(0x5f) as u8),
            3 => s.extend("é".as_bytes()),
            4 => s.extend("語".as_bytes()),
            5 => s.extend("😀".as_bytes()),
            6 => s.extend([0xed, 0xa0 + rng.below(0x20) as u8, 0x80]), // a surrogate
            9 => s.extend([0xee, 0x80, 0x80]),                         // U+E000, just past them
            10 => s.extend([0xed, 0x9f, 0xbf]),                        // U+D7FF, just before
            11 => s.extend([0xf4, 0x8f, 0xbf, 0xbf]),                  // U+10FFFF
            7 => s.extend([0xf4, 0x90, 0x80, 0x80]),                   // past U+10FFFF
            _ => s.push(0x80 + rng.below(0x80) as u8),
        }
    }
    s.push(0);
    s
}

#[test]
fn p4c2_utf8_to_wide_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0060_7c40);
        for i in 0..400 {
            let what = format!("{name} utf8 case {i}");
            let text = utf8_text(&mut rng);
            let src = if rng.chance(20) { 0 } else { t.alloc(&text) };
            let cap = rng.below(2 * text.len() as u32 + 4);
            let dst = t.zeros(2 * cap as usize + 8);
            t.fill(&mut rng, dst, 2 * cap + 8);
            let dst = if rng.chance(25) { 0 } else { dst };
            let end = match rng.below(12) {
                0 => 0,
                1 => dst.wrapping_sub(2),
                _ => dst + 2 * cap,
            };
            t.case(&what, 0x0060_7c40, Conv::Cdecl, &[src, dst, end], |g| {
                let cursor = g.scratch();
                Out::U32(p4_frames::bridge2::utf8_to_wide(g, cursor, src, dst, end))
            });
        }
    }
}

#[test]
fn p4c2_keyboard_update_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0062_d5c0);
        t.stub(p4_input::MEMSET, false, 3, 0, 0x0e3e_0000);
        t.stub(p4_input::UPDATE_MAPPINGS, true, 0, 0, 0x0aa0_0000);
        // GetDeviceState copies `state` into the buffer and returns [hr]
        let state = t.zeros(0x100);
        let hr = t.zeros(4);
        let mut tail = vec![0x56, 0x57, 0xbe]; // push esi; push edi; mov esi,
        tail.extend(state.to_le_bytes());
        tail.extend([0x8b, 0x7c, 0x24, 0x14]); // mov edi, [esp+0x14]
        tail.extend([0xb9, 0x40, 0, 0, 0, 0xf3, 0xa5]); // mov ecx, 0x40; rep movsd
        tail.extend([0x5f, 0x5e, 0xa1]); // pop edi; pop esi; mov eax,
        tail.extend(hr.to_le_bytes());
        tail.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        let get_state = t.logged(0x7e00_0009, false, 3, &tail);
        // Acquire: 0x8007001e while [left] counts down, then [ok]
        let left = t.zeros(8);
        let mut tail = vec![0xa1];
        tail.extend(left.to_le_bytes()); // mov eax, [left]
        tail.extend([0x85, 0xc0, 0x74, 0x0e]); // test eax, eax; jz done
        tail.extend([0xff, 0x0d]);
        tail.extend(left.to_le_bytes()); // dec dword [left]
        tail.push(0xb8);
        tail.extend(p4_input::E_ACCESS_DENIED.to_le_bytes()); // mov eax, E
        tail.extend([0xc2, 0x04, 0x00]); // ret 4
        tail.push(0xa1);
        tail.extend((left + 4).to_le_bytes()); // done: mov eax, [ok]
        tail.extend([0xc2, 0x04, 0x00]); // ret 4
        let acquire = t.logged(0x7e00_0007, false, 1, &tail);
        let mut vt = vec![0u8; 0x28];
        vt[0x1c..0x20].copy_from_slice(&acquire.to_le_bytes());
        vt[0x24..0x28].copy_from_slice(&get_state.to_le_bytes());
        let vt = t.alloc(&vt);
        let app_global = t.rebase(p4_input::G_APPLICATION);
        let app = t.zeros(0x40);
        let buf: Vec<i32> = (0..0x40).map(|k| -0x104 + 4 * k).collect();
        for i in 0..96 {
            let what = format!("{name} keyboard case {i}");
            let kb = t.zeros(0x320);
            t.fill(&mut rng, kb, 0x320);
            let dev = if rng.chance(8) {
                0
            } else {
                let d = t.zeros(8);
                t.w32(d, vt);
                d
            };
            t.w32(kb + 0x14, dev);
            let bytes: Vec<u8> = (0..0x100)
                .map(|_| if rng.chance(3) { rng.next() as u8 } else { 0 })
                .collect();
            t.write(state, &bytes);
            let r = match rng.below(4) {
                0 => 0x8000_4005,
                1 => rng.next(),
                _ => 0,
            };
            t.w32(hr, r);
            t.w32(left, rng.below(3));
            t.w32(
                left + 4,
                [0, 1, p4_input::E_ACCESS_DENIED + 1][rng.below(3) as usize],
            );
            t.w32(app_global, if rng.chance(4) { 0 } else { app });
            t.w32(app + 0x2c, u32::from(rng.chance(3)));
            t.frame_case(
                &what,
                0x0062_d5c0,
                Conv::Thiscall(kb),
                &[],
                0x104,
                &buf,
                |g, ebp| {
                    p4_input::bridge6::keyboard_update(g, ebp, kb);
                    Out::None
                },
            );
        }
    }
}

#[test]
fn p4c2_cloth_terminate_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006e_2e40);
        let malloc = t.stub_bump(p4_cloth::PHYRE_MALLOC, 2);
        t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0, 0);
        let g = t.rebase(p4_cloth::G_CLOTH_ENTRIES);
        for i in 0..200 {
            let what = format!("{name} cloth terminate case {i}");
            let n = rng.below(4);
            let table = t.zeros(16 * n as usize + 16);
            t.fill(&mut rng, table, 16 * n + 16);
            for k in 0..n {
                let left = if rng.chance(3) {
                    0xffff_ffff
                } else {
                    rng.below(5)
                };
                t.w32(table + 16 * k + 0xc, left);
            }
            let owned = if rng.chance(3) { 0x8000_0000 } else { 0 };
            t.w32(g, n | owned);
            t.w32(g + 4, if n == 0 && rng.chance(2) { 0 } else { table });
            let block = if rng.chance(6) { 0 } else { t.zeros(0x100) };
            t.w32(malloc, block);
            let cloth = t.zeros(0x20);
            t.fill(&mut rng, cloth, 0x20);
            let x = rng.next();
            t.case(&what, 0x006e_2e40, Conv::Thiscall(cloth), &[x], |g| {
                p4_cloth::bridge2::terminate(g, cloth);
                Out::None
            });
        }
    }
}

#[test]
fn p4c2_string_tree_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0064_c0a0);
        let keys: Vec<u32> = ["", "a", "ab", "abc", "b", "ba", "\u{e9}", "zz"]
            .iter()
            .map(|k| {
                let mut v = k.as_bytes().to_vec();
                v.push(0);
                t.alloc(&v)
            })
            .collect();
        for round in 0..48 {
            let tree = t.zeros(0x10);
            t.fill(&mut rng, tree, 0x10);
            t.w32(tree + 0xc, tree);
            for i in 0..rng.below(10) {
                let node = t.zeros(0x10);
                t.fill(&mut rng, node, 0x10);
                let key = if rng.chance(12) {
                    0
                } else {
                    keys[rng.below(keys.len() as u32) as usize]
                };
                t.w32(node + 0xc, key | rng.below(2));
                let what = format!("{name} tree {round} insert {i}");
                t.case(&what, 0x0064_c0a0, Conv::Thiscall(tree), &[node], |g| {
                    Out::U32(p4_tree::bridge::insert(g, tree, node))
                });
            }
        }
    }
}

#[test]
fn p4c2_set_light_dir_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006a_a9f0);
        t.stub(p4_light::LOOK_AT, true, 3, 3, 0x100c_0000);
        t.stub(p4_light::ROTATION_X, false, 2, 0, 0x0e07_0000);
        t.stub(p4_light::APPLY_ROTATION, true, 1, 1, 0x0a77_0000);
        let frame: Vec<i32> = (0..0x1c).map(|k| -0x70 + 4 * k).collect();
        for i in 0..160 {
            let what = format!("{name} light dir case {i}");
            let cl = lighting(&mut t, &mut rng);
            let li = rng.below(5);
            let color = t.rebase(p4_light::COLOR_LIGHT_TYPE);
            if li < 5 && rng.chance(2) {
                let set: u32 = Mem::read(&t.b, cl + 0xf0);
                let lights: u32 = Mem::read(&t.b, set + 4);
                let l: u32 = Mem::read(&t.b, lights + 4 * li);
                t.w32(l + 0x18, color);
            }
            let y = match rng.below(5) {
                0 => (-0.9f32).to_bits(),
                1 => 0xbf66_6665, // just above -0.9
                2 => 0xbf66_6667,
                _ => float_word(&mut rng),
            };
            let xyz = [float_word(&mut rng), y, float_word(&mut rng)];
            let on = Mem::read::<u8>(&t.b, cl + 0x1cc) != 0;
            let set: u32 = Mem::read(&t.b, cl + 0xf0);
            let lights: u32 = Mem::read(&t.b, set + 4);
            let l: u32 = Mem::read(&t.b, lights + 4 * li);
            let colored = Mem::read::<u32>(&t.b, l + 0x18) == color;
            t.frame_case(
                &what,
                0x006a_a9f0,
                Conv::Thiscall(cl),
                &[li, xyz[0], xyz[1], xyz[2]],
                0xb0,
                &frame,
                |g, ebp| {
                    let r = p4_light::bridge6::set_light_dir(g, ebp, cl, li, xyz);
                    if on && li != 3 && colored {
                        Out::U32(r)
                    } else {
                        Out::None
                    }
                },
            );
        }
    }
}

#[test]
fn p4c2_stream_variants_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0067_3030);
        let malloc = t.stub_bump(p4_mesh::PHYRE_MALLOC, 2);
        t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0, 0x0f0f_0f0f);
        t.stub(p4_mesh::RESIZE16, true, 1, 1, 0);
        let owned = |rng: &mut Rng| if rng.chance(3) { 0x8000_0000 } else { 0 };
        for i in 0..128 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // no +0x1c, two arguments
            let mesh = t.zeros(0x20);
            t.fill(&mut rng, mesh, 0x20);
            let dn = [1, 0, 2][rng.below(3) as usize] | owned(&mut rng);
            let sn = [3, 0, 4][rng.below(3) as usize] | owned(&mut rng);
            let block = if rng.chance(4) { 0 } else { t.zeros(0x100) };
            let draw = t.zeros(0x10);
            let streams = t.zeros(0x30);
            t.w32(mesh + 0x10, dn);
            t.w32(mesh + 8, sn);
            t.w32(mesh + 0x14, draw);
            t.w32(mesh + 0xc, streams);
            if block != 0 && rng.chance(4) {
                t.w32(mesh + 0xc, block + if dn == 1 { 0 } else { 0x40 });
            }
            t.w32(malloc, block);
            let [b, c] = [rng.next(), rng.next()];
            t.case(
                &what("streams 0x674740"),
                0x0067_4740,
                Conv::Thiscall(mesh),
                &[b, c],
                |g| Out::U32(p4_mesh::bridge::set_streams(g, mesh, None, b, c)),
            );
            // four streams, the draw table from the records
            let n: u32 = [0, 1, 3, 7, 0xffff_ffff][rng.below(5) as usize];
            let real = if (n as i32) > 0 { n } else { 0 };
            let src = t.zeros(0x98);
            t.fill(&mut rng, src, 0x98);
            t.w32(src + 0x90, n);
            let recs = t.zeros((real * 0x6c + 0x10) as usize);
            t.fill(&mut rng, recs, real * 0x6c + 0x10);
            t.w32(src + 0x94, recs);
            let mesh = t.zeros(0x20);
            t.fill(&mut rng, mesh, 0x20);
            let sn = [4, 3, 0][rng.below(3) as usize] | owned(&mut rng);
            t.w32(mesh + 8, sn);
            let streams = t.zeros(0x30);
            t.w32(mesh + 0xc, streams);
            let table = t.zeros((real * 0x10 + 0x10) as usize);
            t.w32(mesh + 0x14, table);
            let block = if rng.chance(4) { 0 } else { t.zeros(0x100) };
            if block != 0 && rng.chance(4) {
                t.w32(mesh + 0xc, block);
            }
            t.w32(malloc, block);
            t.case(
                &what("streams4"),
                0x0067_3030,
                Conv::Thiscall(mesh),
                &[src],
                |g| Out::U32(p4_mesh::bridge::set_streams4(g, mesh, src)),
            );
        }
    }
}

use ffx_pother_b::p4_ps2light;

#[test]
fn p4c2_ps2_lighting_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x006e_5a40);
        t.stub(p4_ps2light::SHADER_PARAM_WRITE, false, 3, 0, 0x05a0_0000);
        let app = t.zeros(0x300);
        t.fill(&mut rng, app, 0x300);
        t.stub(p4_ps2light::GET_APPLICATION, false, 0, 0, app);
        t.stub(p4_ps2light::RENDER_MESH_INSTANCE, true, 4, 4, 0x0e1d_0000);
        t.stub(p4_ps2light::ADD_VFX_DRAW_EVENT, true, 1, 1, 0x0e7e_0000);
        t.stub(p4_ps2light::SET_VISIBLE, false, 2, 0, 0x0515_0000);
        let meshes = t.rebase(p4_ps2light::MESHES);
        let recs = t.rebase(p4_ps2light::RECORDS);
        let count = t.rebase(p4_ps2light::COUNT);
        let event: Vec<i32> = vec![-0x14, -0x10, -0xc, -8];
        for i in 0..96 {
            let what = |s: &str| format!("{name} {s} case {i}");
            // a mesh with 0..3 instances
            let n = rng.below(4);
            let mesh = t.zeros(8);
            let list = t.zeros(4 * n as usize + 4);
            for k in 0..n {
                let inst = t.zeros(0x20);
                t.fill(&mut rng, inst, 0x20);
                t.w32(list + 4 * k, inst);
            }
            t.w32(mesh, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
            t.w32(mesh + 4, list);
            let keys = [0, 0, rng.next(), 7];
            for k in 0..8 {
                let w = match rng.below(4) {
                    0 => mesh,
                    1 => rng.next(),
                    _ => 0,
                };
                t.w32(meshes + 4 * k, w);
                let key = keys[rng.below(4) as usize];
                t.w32(recs + p4_ps2light::RECORD * k + 0x128, key);
            }
            t.w32(count, rng.below(9));
            let m = if rng.chance(10) { 0 } else { mesh };
            let mut slot = 0;
            t.case(&what("apply"), 0x006e_5a40, Conv::Cdecl, &[m], |g| {
                let (r, s) = p4_ps2light::bridge::apply(g, m);
                slot = s;
                Out::U32(r)
            });
            let at = t.a.stack_range().end - 0x100 - 4;
            assert_eq!(Mem::read::<u32>(&t.a, at), slot, "{}", what("slot"));
            // register: the matrices and vectors as random blocks
            let block = |t: &mut Twins, rng: &mut Rng| {
                let b = t.zeros(0x40);
                t.fill(rng, b, 0x40);
                b
            };
            let s = p4_ps2light::bridge::Setup {
                m1: block(&mut t, &mut rng),
                m2: block(&mut t, &mut rng),
                f1: float_word(&mut rng),
                m3: block(&mut t, &mut rng),
                m4: block(&mut t, &mut rng),
                f2: float_word(&mut rng),
                v1: block(&mut t, &mut rng),
                v2: block(&mut t, &mut rng),
            };
            let key = keys[rng.below(4) as usize];
            if rng.chance(6) {
                // every slot taken by another mesh
                for k in 0..8 {
                    t.w32(meshes + 4 * k, mesh ^ (k + 1));
                }
            }
            let args = [mesh, s.m1, s.m2, s.f1, s.m3, s.m4, s.f2, s.v1, s.v2, key];
            t.frame_case_fill(
                0,
                &what("register"),
                0x006e_5c90,
                Conv::Cdecl,
                &args,
                0x14,
                &event,
                |g, ebp| Out::U32(p4_ps2light::bridge::register(g, ebp, mesh, &s, key)),
            );
        }
    }
}

#[test]
fn p4c2_menu_geometry_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x0068_1970);
        let scene = t.zeros(0xfb00);
        let g = t.rebase(p4_mesh::PHYRE_SCENE);
        t.w32(g, scene);
        let block = t.stub_cell(p4_mesh::OPERATOR_NEW, false, 1, 0);
        // the constructor returns its `this`
        t.stub_tail(p4_mesh::MENU_GEOMETRY_CTOR, true, 0, &[0x89, 0xc8, 0xc3]);
        t.stub(p4_mesh::GEOMETRY_HIDE, true, 0, 0, 0x41de_0000);
        let mut vt = Vec::new();
        for (slot, n) in [(0u32, 0u32), (1, 2), (2, 1)] {
            let f = t.stub_code(0x7e00_0000 + slot, true, n, n, 0x5a5a_0000 + slot);
            vt.extend(f.to_le_bytes());
        }
        let vt = t.alloc(&vt);
        t.a.map_zeroed(0xc000_0000, 0x1000).unwrap();
        t.b.map_zeroed(0xc000_0000, 0x1000).unwrap();
        for i in 0..160 {
            let what = format!("{name} menu geometry case {i}");
            for (flag, word) in [
                (0xfa9d, 0xfa84),
                (0xfaa0, 0xfa94),
                (0xfa9f, 0xfa90),
                (0xfa9a, 0xfa78),
                (0xfa98, 0xfa70),
                (0xfa99, 0xfa74),
            ] {
                t.write(scene + flag, &[u8::from(rng.chance(3))]);
                let w = rng.next();
                t.w32(scene + word, w);
            }
            // the sign extension: geometry at a "negative" address too
            let geom = if rng.chance(3) {
                0xc000_0000 + 0x40 * rng.below(0x40)
            } else {
                t.zeros(0x3c)
            };
            t.fill(&mut rng, geom, 0x3c);
            t.w32(geom, vt);
            t.w32(block, geom);
            let kind = [1, 0xffff_ffff, 0xffff_fffe, 0, 2, 3, 4, rng.next()][rng.below(8) as usize];
            let owner = rng.next();
            let mut word = 0;
            t.case(&what, 0x0068_1970, Conv::Stdcall, &[owner, kind], |g| {
                let (r, w) = p4_mesh::bridge2::create(g, owner, kind);
                word = w;
                Out::U64(r)
            });
            let at = t.a.stack_range().end - 0x100 - 4;
            assert_eq!(Mem::read::<u32>(&t.a, at), word, "{what}: slot");
        }
    }
}
