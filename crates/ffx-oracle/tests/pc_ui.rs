//! The PC UI functions (`ffx_pcui`) against the original: two oracles
//! started alike, one runs `FFX.exe`'s code, the other the Rust bridge;
//! results (`eax`, or `ST0` for float results) and every written byte must
//! agree. Every test runs on the image alone (`FFX_GAME_DIR`), then on a
//! snapshot of the running game when `FFX_SNAPSHOT` is set; the sphere
//! grid tests put a made-up `g_Abmap` in both (the snapshots are not on the
//! sphere grid screen) and also read the snapshot's own when it has one.
//! Float arguments include signed zeros, denormals, infinities and quiet
//! NaNs, never signalling ones (the emulator keeps them through `fld m32`).

#![allow(clippy::many_single_char_names)] // pointers, rectangles

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pcui::abmap::bridge as abmap;
use ffx_pcui::cursor::bridge as cursor;
use ffx_pcui::iggy::bridge as iggy;
use ffx_pcui::item::bridge as item;
use ffx_pcui::link::bridge as link;
use ffx_pcui::panel::bridge as panel;
use ffx_pcui::sea::bridge as sea;
use ffx_pcui::spdctrl::bridge as spdctrl;
use ffx_pcui::touch::bridge as touch;

const IGGY_SET_INT: u32 = 0x00a2_8d30;
const IGGY_SET_UTF8: u32 = 0x00a2_8d70;
const IGGY_SET_BOOL: u32 = 0x00a2_8db0;
const HANDLERS_ADD_CALL: u32 = 0x00a2_8df0;
const HANDLERS_ADD: u32 = 0x00a2_8e40;
const IGGY_MENU_SET_ENABLE: u32 = 0x00a2_8ef0;
const ABMAP_ZOOM_SCALE: u32 = 0x00a5_6820;
const ABMAP_NODE_RADIUS: u32 = 0x00a5_6f30;
const ABMAP_RANGES_OVERLAP: u32 = 0x00a5_7000;
const ABMAP_RESET_VECTORS: u32 = 0x00a5_7120;
const ABMAP_READ_PAD: u32 = 0x00a5_7520;
const ABMAP_FLAG_CONNECTED_LINKS: u32 = 0x00a5_8df0;
const ABMAP_TRY_CHANGE_PLAYER: u32 = 0x00a5_8ec0;
const ABMAP_INIT_PLY_TAG: u32 = 0x00a5_7f80;
const ABMAP_POSITION_PLY_TAG: u32 = 0x00a5_8080;
const SPD_CTRL_IS_MAP_EXCLUDED: u32 = 0x006f_6fa0;
const SPD_CTRL_IS_MAGIC_EXCLUDED: u32 = 0x006f_6ff0;
const ABMAP_READ_CURSOR_STICK: u32 = 0x00a5_6bd0;
const ABMAP_FIND_NEAREST_NODE: u32 = 0x00a5_6a40;
const ABMAP_FIND_NODE_IN_DIRECTION: u32 = 0x00a5_6860;
const ABMAP_CALC_LINK_POINTS_STRAIGHT: u32 = 0x00a5_81f0;
const ABMAP_CALC_LINK_POINTS_CURVED: u32 = 0x00a5_7710;
const ABMAP_PAD_ACTIVATE_HANDLER: u32 = 0x00a5_6160;
const GUI_TOUCH_INIT_GROUP: u32 = 0x0068_a420;
const ABMAP_PAD_CTRL: u32 = 0x00a5_83e0;
const MENU_ITEM_PARSE_TARGET_KEYS: u32 = 0x008c_5320;
const MENU_ITEM_DRAW_ROW: u32 = 0x008c_9a70;
const MENU_ITEM_GET_LINE_POS: u32 = 0x008c_9b60;
const RCUI_SEA_ADD_UV_SPD: u32 = 0x0093_0c00;
const GUI_TOUCH_RESET_BUTTONS: u32 = 0x0068_ab30;
const GUI_TOUCH_HANDLE_EVENT: u32 = 0x0068_b470;
const GUI_TOUCH_HOVER: u32 = 0x0068_b6d0;

/// An oracle as the bridges' host: original callees run in the emulator.
struct Run<'a>(&'a mut Oracle);

impl Mem for Run<'_> {
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

fn conv(c: host::Conv) -> Conv {
    match c {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for Run<'_> {
    fn call(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> u32 {
        self.0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }

    fn call_st0(&mut self, addr: u32, c: host::Conv, args: &[u32]) -> f32 {
        let r = self
            .0
            .call(addr, conv(c), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        let v = r
            .st0
            .unwrap_or_else(|| panic!("callee {addr:#x}: no float in ST0"));
        // the stand-ins return f32 values (no NaNs): exact
        f32::from_bits(ffx_bridge::x87::store(v))
    }

    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        #[allow(clippy::cast_possible_truncation)]
        (0..n).map(|_| self.next() as u8).collect()
    }
}

/// What a function returns.
#[derive(Debug, Clone, Copy)]
enum Res {
    /// Nothing callers read (`eax` is left over).
    Void,
    Int(u32),
    Float(f32),
}

/// The two oracles.
struct Twin {
    name: &'static str,
    a: Oracle,
    b: Oracle,
}

/// The oracles to run on: the image alone (needs `FFX_GAME_DIR`), then the
/// snapshot when `FFX_SNAPSHOT` is set.
fn twins() -> Vec<Twin> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return Vec::new();
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    for o in [&mut a, &mut b] {
        // the game thread's control word, as the snapshot oracle sets it
        o.set_fpu_control(0x027f).unwrap();
    }
    let mut out = vec![Twin {
        name: "image",
        a,
        b,
    }];
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        out.push(Twin {
            name: "snapshot",
            a: Oracle::from_snapshot(&exe, &pages).unwrap(),
            b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        });
    } else {
        eprintln!("FFX_SNAPSHOT is not set; image only");
    }
    out
}

impl Twin {
    /// The same bytes in both.
    fn put(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    /// The same allocation in both.
    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    /// Puts `code` in the oracles' code space and a `jmp` to it at `at`
    /// (stubs of neighbouring functions would overlap in place).
    fn jump(&mut self, at: u32, code: &[u8]) {
        let to = self.a.alloc_code(code).unwrap();
        assert_eq!(self.b.alloc_code(code).unwrap(), to);
        let mut j = vec![0xe9];
        j.extend(to.wrapping_sub(at + 5).to_le_bytes());
        self.put(at, &j);
    }

    fn alloc_bytes(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    /// Runs the original at `f` in `a` and `rust` in `b`; compares the
    /// result and every written byte.
    fn check(
        &mut self,
        what: &str,
        f: u32,
        conv: Conv,
        args: &[u32],
        rust: impl FnOnce(&mut Oracle) -> Res,
    ) {
        let what = &format!("{}: {what}", self.name);
        let at = self.a.rebase(f);
        let ra = self
            .a
            .call(at, conv, args)
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        let rb = rust(&mut self.b);
        // A float the original leaves in ST0 is a result its callers use.
        assert!(
            ra.st0.is_none() || matches!(rb, Res::Float(_)),
            "{what}: the original returns {:?} in ST0",
            ra.st0
        );
        match rb {
            Res::Void => {}
            Res::Int(v) => assert_eq!(ra.eax, v, "{what}: eax"),
            Res::Float(v) => {
                let st0 = ra.st0.unwrap_or_else(|| panic!("{what}: no float result"));
                assert_eq!(
                    st0.to_bits(),
                    f64::from(v).to_bits(),
                    "{what}: {st0} vs {v}"
                );
            }
        }
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }
}

/// Integers worth trying: edges of i32 and of f32's exact range, small
/// ones.
const INTS: [u32; 14] = [
    0,
    1,
    0xffff_ffff,
    0x7fff_ffff,
    0x8000_0000,
    0x00ff_ffff,
    0x0100_0000,
    0x0100_0001,
    0x0100_0003,
    0xfeff_ffff,
    0xff00_0001,
    0x1234_5679,
    0xffff,
    100,
];

#[test]
fn iggy_data_values_match_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x1997_0705);
        let v = t.alloc(0x20);
        // null leaves everything alone
        for (f, rust) in [(IGGY_SET_INT, 0), (IGGY_SET_UTF8, 1), (IGGY_SET_BOOL, 2)] {
            t.check("null value", f, Conv::Cdecl, &[0, 7], |b| {
                match rust {
                    0 => iggy::set_int(b, 0, 7),
                    1 => iggy::set_utf8(b, 0, 7),
                    _ => iggy::set_bool(b, 0, 7),
                }
                Res::Void
            });
        }
        for n in 0..400 {
            let junk = rng.bytes(0x20);
            t.put(v, &junk);
            #[allow(clippy::cast_possible_truncation)]
            let x = INTS.get(n).copied().unwrap_or_else(|| rng.next());
            #[allow(clippy::cast_possible_wrap)]
            t.check(
                &format!("int {x:#x}"),
                IGGY_SET_INT,
                Conv::Cdecl,
                &[v, x],
                |b| {
                    iggy::set_int(b, v, x as i32);
                    Res::Void
                },
            );
            t.put(v, &junk);
            t.check(
                &format!("bool {x:#x}"),
                IGGY_SET_BOOL,
                Conv::Cdecl,
                &[v, x],
                |b| {
                    iggy::set_bool(b, v, x);
                    Res::Void
                },
            );
        }
        for len in [0usize, 1, 2, 15, 16, 17, 300] {
            let mut s: Vec<u8> = (0..len)
                .map(|_| {
                    #[allow(clippy::cast_possible_truncation)]
                    let c = rng.below(255) as u8 + 1;
                    c
                })
                .collect();
            s.push(0);
            s.extend(rng.bytes(8));
            let text = t.alloc_bytes(&s);
            t.put(v, &rng.bytes(0x20));
            t.check(
                &format!("utf8 of {len}"),
                IGGY_SET_UTF8,
                Conv::Cdecl,
                &[v, text],
                |b| {
                    iggy::set_utf8(b, v, text);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn iggy_menu_set_enable_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x0bad_cafe);
        let menu = t.alloc(0x100);
        for _ in 0..64 {
            t.put(menu, &rng.bytes(0x100));
            let on = rng.next();
            t.check(
                &format!("setEnable {on:#x}"),
                IGGY_MENU_SET_ENABLE,
                Conv::Thiscall(menu),
                &[on],
                |b| {
                    iggy::set_enable(b, menu, on);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn iggy_menu_handlers_match_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x5eed_0042);
        let menu = t.alloc(0x80);
        // every pattern of used slots, used bytes of any non-zero value
        for case in 0..256u32 {
            let mut bytes = rng.bytes(0x80);
            for i in 0..5 {
                let used = (case >> i) & 1 != 0 || (case >= 32 && rng.below(2) == 0);
                #[allow(clippy::cast_possible_truncation)]
                let mark = rng.below(255) as u8 + 1;
                bytes[4 + 0x14 * i] = if used { mark } else { 0 };
            }
            let w: [u32; 4] = std::array::from_fn(|_| rng.next());
            t.put(menu, &bytes);
            t.check(
                &format!("add {case:#x}"),
                HANDLERS_ADD,
                Conv::Thiscall(menu),
                &w,
                |b| Res::Int(iggy::add(b, menu, w)),
            );
            t.put(menu, &bytes);
            t.check(
                &format!("addCall {case:#x}"),
                HANDLERS_ADD_CALL,
                Conv::Thiscall(menu),
                &w[..2],
                |b| Res::Int(iggy::add_call(b, menu, w[0], w[1])),
            );
        }
    }
}

/// Puts a made-up `g_Abmap` of `len` random bytes in both; returns it.
fn made_up_abmap(t: &mut Twin, rng: &mut Xorshift, len: usize) -> u32 {
    let g = t.alloc(len);
    t.put(g, &rng.bytes(len));
    let at = t.a.rebase(abmap::ABMAP.sym);
    t.put(at, &g.to_le_bytes());
    g
}

#[test]
fn abmap_zoom_scale_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0001);
        let own = abmap::ABMAP.get(&t.b);
        if own != 0 {
            t.check("own zoom", ABMAP_ZOOM_SCALE, Conv::Cdecl, &[], |b| {
                Res::Float(abmap::zoom_scale(b))
            });
        }
        let g = made_up_abmap(&mut t, &mut rng, 0x11600);
        for level in 0..=255u8 {
            t.put(g + 0x115cc, &[level]);
            t.check(
                &format!("zoom {level}"),
                ABMAP_ZOOM_SCALE,
                Conv::Cdecl,
                &[],
                |b| Res::Float(abmap::zoom_scale(b)),
            );
        }
    }
}

#[test]
fn abmap_node_radius_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0002);
        let own = abmap::ABMAP.get(&t.b);
        if own != 0 {
            let nodes = u32::from(t.b.read::<u16>(own + 2));
            for node in 0..nodes {
                t.check(
                    &format!("own node {node}"),
                    ABMAP_NODE_RADIUS,
                    Conv::Cdecl,
                    &[node],
                    |b| Res::Float(abmap::node_radius(b, node)),
                );
            }
        }
        // room for every kind (u16) after the kind table
        let len = 0xf834 + 0x1_0000 * 0x30;
        let g = made_up_abmap(&mut t, &mut rng, len);
        for n in 0..600u32 {
            let node = rng.below(0x600);
            let kind = match n % 4 {
                0 => 0xffff,
                1 => rng.below(0x1_0000),
                _ => rng.below(0x20),
            };
            #[allow(clippy::cast_possible_truncation)]
            t.put(g + 0x80e + node * 0x28, &(kind as u16).to_le_bytes());
            let size: u16 = match n % 5 {
                0 => 0x8000,
                1 => 0x7fff,
                2 => 0xffff,
                3 => 1,
                #[allow(clippy::cast_possible_truncation)]
                _ => rng.next() as u16,
            };
            t.put(g + 0xf834 + kind * 0x30, &size.to_le_bytes());
            t.check(
                &format!("node {node} kind {kind:#x} size {size:#x}"),
                ABMAP_NODE_RADIUS,
                Conv::Cdecl,
                &[node],
                |b| Res::Float(abmap::node_radius(b, node)),
            );
        }
    }
}

/// Floats worth trying, as bits: zeros, ±1, close neighbours, large and
/// tiny, a denormal, infinities, quiet NaNs.
const FLOATS: [u32; 12] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000,
    0x3f80_0001,
    0xbf80_0000,
    0x4f80_0000,
    0x0000_0001,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0001,
    0x4040_0000,
];

#[test]
fn abmap_ranges_overlap_matches_the_original() {
    for mut t in twins() {
        for &a in &FLOATS {
            for &b in &FLOATS {
                for &c in &FLOATS {
                    for &d in &FLOATS {
                        let what = format!("{a:#x} {b:#x} {c:#x} {d:#x}");
                        t.check(
                            &what,
                            ABMAP_RANGES_OVERLAP,
                            Conv::Cdecl,
                            &[a, b, c, d],
                            |_| {
                                Res::Int(abmap::ranges_overlap(
                                    f32::from_bits(a),
                                    f32::from_bits(b),
                                    f32::from_bits(c),
                                    f32::from_bits(d),
                                ))
                            },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn abmap_reset_vectors_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0003);
        for n in 0..8 {
            made_up_abmap(&mut t, &mut rng, 0x11600);
            let gpr = t.a.rebase(abmap::EE_GPR.sym);
            let r0 = if n == 0 { vec![0; 16] } else { rng.bytes(16) };
            t.put(gpr, &r0);
            t.check(
                "reset vectors",
                ABMAP_RESET_VECTORS,
                Conv::Cdecl,
                &[],
                |b| Res::Int(abmap::reset_vectors(b)),
            );
        }
    }
}

#[test]
fn abmap_read_pad_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0004);
        let pad = t.alloc(0x30);
        let stub1 = t.a.rebase(abmap::TO_MENU_GET_CONTROL_PAD2);
        let stub2 = t.a.rebase(abmap::TO_MENU_GET_CONTROL_PAD_ANALOG_HIST);
        let mut state = rng.bytes(0x30);
        for n in 0..3000u32 {
            // runs of frames on one pad, now and then a scrambled one
            if n % 97 == 0 {
                state = rng.bytes(0x30);
                if n % 2 == 0 {
                    for c in &mut state[0xa..0x2a] {
                        *c %= 8;
                    }
                }
            }
            let buttons = match rng.below(4) {
                0 => rng.next(),
                1 => 0,
                _ => 1 << rng.below(16) | (rng.next() & 0xffff_0000),
            };
            let analog = match rng.below(3) {
                0 => rng.next(),
                _ => {
                    (rng.next() & !0xff)
                        | [0x3f, 0x40, 0x41, 0xbf, 0xc0, 0xc1, 0x80, 0][n as usize % 8]
                }
            };
            for o in [&mut t.a, &mut t.b] {
                o.stub_return(stub1, buttons).unwrap();
            }
            // the stick: `analog` xor the sum of the four arguments
            let mut code = vec![0x8b, 0x44, 0x24, 0x10];
            code.extend([0x03, 0x44, 0x24, 0x0c, 0x03, 0x44, 0x24, 0x08]);
            code.extend([0x03, 0x44, 0x24, 0x04, 0x35]);
            code.extend((analog ^ 2).to_le_bytes());
            code.push(0xc3);
            t.put(stub2, &code);
            t.put(pad, &state);
            let no_stick = [0, 1, 0x100, 0][n as usize % 4];
            t.check(
                &format!("pad {n} {buttons:#x} {analog:#x} {no_stick}"),
                ABMAP_READ_PAD,
                Conv::Cdecl,
                &[no_stick, pad],
                |b| Res::Int(abmap::read_pad(&mut Run(b), no_stick, pad)),
            );
            state = Oracle::read(&mut t.b, pad, 0x30).unwrap();
        }
    }
}

/// A pointer-zone object with `regions` hover regions and `groups` button
/// groups of random content, coordinates small enough to hit.
fn made_up_touch(t: &mut Twin, rng: &mut Xorshift, regions: u32, groups: u32) -> u32 {
    let this = t.alloc(0x170);
    t.put(this, &rng.bytes(0x170));
    let r = t.alloc(0x150 * regions as usize + 4);
    let mut bytes = rng.bytes(0x150 * regions as usize + 4);
    for i in 0..regions as usize {
        let e = &mut bytes[i * 0x150..(i + 1) * 0x150];
        e[1] = if rng.below(4) == 0 { 0 } else { e[1] | 1 };
        for k in 0..32 {
            for f in 0..4 {
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let v = (rng.below(30) as i16 - 5).cast_unsigned();
                let at = 0xe + 10 * k + 2 * f;
                e[at..at + 2].copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    t.put(r, &bytes);
    let g = t.alloc(0x1c8 * groups as usize + 4);
    let mut bytes = rng.bytes(0x1c8 * groups as usize + 4);
    for i in 0..groups as usize {
        let e = &mut bytes[i * 0x1c8..(i + 1) * 0x1c8];
        e[1] = if rng.below(4) == 0 { 0 } else { e[1] | 1 };
        for j in 0..8 {
            let b = &mut e[8 + 0x38 * j..8 + 0x38 * (j + 1)];
            for f in 0..4 {
                #[allow(clippy::cast_precision_loss)]
                let v = rng.below(30) as f32 - 5.0;
                b[4 * f..4 * f + 4].copy_from_slice(&v.to_le_bytes());
            }
            #[allow(clippy::cast_possible_truncation)]
            {
                b[0x14] = rng.below(3) as u8;
                b[0x16] = rng.below(6) as u8;
            }
            for f in [0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30] {
                #[allow(clippy::cast_precision_loss)]
                let v = rng.below(1000) as f32 / 7.0;
                b[f..f + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
    }
    t.put(g, &bytes);
    let top = |rng: &mut Xorshift| if rng.below(2) == 0 { 0x8000_0000 } else { 0 };
    let rc = regions | top(rng);
    let gc = groups | top(rng);
    t.put(this + 0x150, &rc.to_le_bytes());
    t.put(this + 0x154, &r.to_le_bytes());
    t.put(this + 0x158, &gc.to_le_bytes());
    t.put(this + 0x15c, &g.to_le_bytes());
    this
}

/// A pointer coordinate: on the grid the zones use, between, or odd.
fn coord(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => FLOATS[rng.below(12) as usize],
        #[allow(clippy::cast_precision_loss)]
        1..=3 => (rng.below(36) as f32 - 6.0).to_bits(),
        #[allow(clippy::cast_precision_loss)]
        _ => (rng.below(360) as f32 / 10.0 - 6.0).to_bits(),
    }
}

#[test]
fn gui_touch_reset_buttons_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x7007_0001);
        for n in 0..40 {
            let this = made_up_touch(&mut t, &mut rng, 0, n % 5);
            t.check(
                &format!("reset {n}"),
                GUI_TOUCH_RESET_BUTTONS,
                Conv::Thiscall(this),
                &[],
                |b| {
                    touch::reset_buttons(b, this);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn gui_touch_hover_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x7007_0002);
        for n in 0..300 {
            let this = made_up_touch(&mut t, &mut rng, n % 4, 0);
            let (x, y) = (coord(&mut rng), coord(&mut rng));
            t.check(
                &format!("hover {n} {x:#x} {y:#x}"),
                GUI_TOUCH_HOVER,
                Conv::Thiscall(this),
                &[x, y],
                |b| {
                    touch::hover(b, this, f32::from_bits(x), f32::from_bits(y));
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn gui_touch_handle_event_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x7007_0003);
        // graphicGetTime as `fld dword [clock]; ret` in both
        let clock = t.alloc(4);
        let get_time = t.a.rebase(touch::GRAPHIC_GET_TIME);
        let mut code = vec![0xd9, 0x05];
        code.extend_from_slice(&clock.to_le_bytes());
        code.push(0xc3);
        t.put(get_time, &code);
        let event = t.alloc(0x20);
        for n in 0..1500 {
            let this = made_up_touch(&mut t, &mut rng, 0, n % 4);
            #[allow(clippy::cast_precision_loss)]
            let now = rng.below(100_000) as f32 / 3.0;
            t.put(clock, &now.to_le_bytes());
            let mut ev = rng.bytes(0x20);
            let kind = if n % 8 == 0 { rng.next() } else { rng.below(7) };
            ev[0x18..0x1c].copy_from_slice(&kind.to_le_bytes());
            #[allow(clippy::cast_possible_truncation)]
            {
                ev[0x1c] = rng.below(3) as u8;
            }
            ev[8..0xc].copy_from_slice(&coord(&mut rng).to_le_bytes());
            ev[0xc..0x10].copy_from_slice(&coord(&mut rng).to_le_bytes());
            t.put(event, &ev);
            t.check(
                &format!("event {n} kind {kind:#x}"),
                GUI_TOUCH_HANDLE_EVENT,
                Conv::Thiscall(this),
                &[event],
                |b| {
                    touch::handle_event(&mut Run(b), this, event);
                    Res::Void
                },
            );
        }
    }
}

/// A call log both oracles share the layout of: a cursor word, then
/// entries (stub id, the first three arguments in 16 bytes; up to seven
/// in 32 bytes).
struct Log {
    cursor: u32,
    /// Stubs made while set return this `f32` in `ST0` instead.
    float: Option<u32>,
}

impl Log {
    fn new(t: &mut Twin) -> Self {
        let cursor = t.alloc(0x1004);
        t.put(cursor, &(cursor + 4).to_le_bytes());
        Self {
            cursor,
            float: None,
        }
    }

    /// Replaces the original at `sym` in both with a cdecl stub that logs
    /// `id` and its first `args` arguments (0 to 7) and returns
    /// `base + (first argument & mask) * stride`.
    fn stub(&self, t: &mut Twin, sym: u32, id: u32, args: u8, ret: (u32, u32, u32)) {
        self.stub_poke(t, sym, id, args, ret, None);
    }

    /// [`Log::stub`] that also stores `poke.1` at `poke.0` (a callee
    /// changing state its caller reads again).
    fn stub_poke(
        &self,
        t: &mut Twin,
        sym: u32,
        id: u32,
        args: u8,
        ret: (u32, u32, u32),
        poke: Option<(u32, u32)>,
    ) {
        let at = t.a.rebase(sym);
        let c = self.code(id, args, ret, poke);
        t.jump(at, &c);
    }

    /// A logging function (as [`Log::stub`]) in the oracles' code space,
    /// for function pointers; returns its address.
    fn func(&self, t: &mut Twin, id: u32, args: u8, ret: (u32, u32, u32)) -> u32 {
        let c = self.code(id, args, ret, None);
        let to = t.a.alloc_code(&c).unwrap();
        assert_eq!(t.b.alloc_code(&c).unwrap(), to);
        to
    }

    /// The stubs' code.
    fn code(&self, id: u32, args: u8, ret: (u32, u32, u32), poke: Option<(u32, u32)>) -> Vec<u8> {
        let mut c = Vec::new();
        if let Some((addr, value)) = poke {
            c.extend([0xc7, 0x05]);
            c.extend(addr.to_le_bytes());
            c.extend(value.to_le_bytes());
        }
        c.extend([0x8b, 0x0d]);
        c.extend(self.cursor.to_le_bytes());
        c.extend([0xc7, 0x01]);
        c.extend(id.to_le_bytes());
        for k in 1..=args {
            c.extend([0x8b, 0x54, 0x24, 4 * k, 0x89, 0x51, 4 * k]);
        }
        let size = if args > 3 { 0x20 } else { 0x10 };
        c.extend([0x83, 0xc1, size, 0x89, 0x0d]);
        c.extend(self.cursor.to_le_bytes());
        if let Some(f) = self.float {
            // fld dword [f]; ret
            c.extend([0xd9, 0x05]);
            c.extend(f.to_le_bytes());
            c.push(0xc3);
            return c;
        }
        let (base, mask, stride) = ret;
        c.extend([0x8b, 0x44, 0x24, 0x04, 0x25]);
        c.extend(mask.to_le_bytes());
        c.extend([0x69, 0xc0]);
        c.extend(stride.to_le_bytes());
        c.push(0x05);
        c.extend(base.to_le_bytes());
        c.push(0xc3);
        c
    }

    fn rewind(&self, t: &mut Twin) {
        t.put(self.cursor, &(self.cursor + 4).to_le_bytes());
    }
}

#[test]
fn abmap_flag_connected_links_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0005);
        let log = Log::new(&mut t);
        // kind records: 0x14 bytes each by the kind's low byte
        let records = t.alloc(0x14 * 256);
        log.stub(
            &mut t,
            abmap::MS_GET_EXCEL_DATA,
            1,
            2,
            (records, 0xff, 0x14),
        );
        let table = t.a.rebase(abmap::NODE_KINDS.sym);
        for n in 0..60u32 {
            log.rewind(&mut t);
            let mut recs = rng.bytes(0x14 * 256);
            for r in 0..256 {
                recs[r * 0x14 + 0x11] &= if rng.below(2) == 0 { 0x7f } else { 0xff };
            }
            t.put(records, &recs);
            t.put(table, &rng.next().to_le_bytes());
            let g = made_up_abmap(&mut t, &mut rng, 0x11600);
            let nodes: i16 = match n % 6 {
                0 => 0,
                1 => -3,
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                _ => rng.below(40) as i16,
            };
            let links: i16 = match n % 5 {
                0 => -1,
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                _ => rng.below(80) as i16,
            };
            t.put(g + 2, &nodes.to_le_bytes());
            t.put(g + 4, &links.to_le_bytes());
            for k in 0..40u32 {
                let kind: u16 = if rng.below(5) == 0 {
                    0xffff
                } else {
                    #[allow(clippy::cast_possible_truncation)]
                    let v = rng.next() as u16;
                    v
                };
                t.put(g + 0x80e + k * 0x28, &kind.to_le_bytes());
            }
            for l in 0..80u32 {
                #[allow(clippy::cast_possible_truncation)]
                let ends = [rng.below(45) as u16, rng.below(45) as u16];
                t.put(g + 0xa808 + l * 0x14, &ends[0].to_le_bytes());
                t.put(g + 0xa80a + l * 0x14, &ends[1].to_le_bytes());
            }
            let flag = rng.next();
            t.check(
                &format!("flag links {n}"),
                ABMAP_FLAG_CONNECTED_LINKS,
                Conv::Cdecl,
                &[flag],
                |b| Res::Int(abmap::flag_connected_links(&mut Run(b), flag)),
            );
        }
    }
}

#[test]
fn abmap_try_change_player_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0006);
        let log = Log::new(&mut t);
        let none = (0, 0, 0);
        log.stub(&mut t, abmap::TK_MENU_SET_PREV_PLAYER, 1, 0, none);
        log.stub(&mut t, abmap::TK_MENU_SET_NEXT_PLAYER, 2, 0, none);
        log.stub(&mut t, abmap::SND_SEP_PLAY_SIMPLE, 4, 1, none);
        log.stub(&mut t, abmap::ABMAP_INIT_HOMING, 5, 2, none);
        log.stub(&mut t, abmap::ABMAP_CHECK_ACHIEVEMENTS, 6, 0, none);
        log.stub(&mut t, abmap::ABMAP_MARK_EMPTY_MENUS, 7, 0, none);
        log.stub(
            &mut t,
            abmap::ABMAP_RESET_CURSOR_CAMERA,
            8,
            0,
            (0x1234_5678, 0, 0),
        );
        for n in 0..400u32 {
            log.rewind(&mut t);
            let current = match n % 3 {
                0 => rng.below(7),
                1 => rng.next(),
                _ => rng.below(0x100) | 0x0100,
            };
            log.stub(
                &mut t,
                abmap::TK_MENU_GET_CURRENT_PLAYER,
                3,
                0,
                (current, 0, 0),
            );
            let g = made_up_abmap(&mut t, &mut rng, 0x11700);
            // homing sometimes stacks a state of its own
            let poke = (rng.below(3) == 0).then(|| (g + 0x115b0, rng.next() & 0xff));
            log.stub_poke(&mut t, abmap::ABMAP_INIT_HOMING, 5, 2, none, poke);
            if rng.below(3) != 0 {
                t.put(g + 0x115b0, &[0; 4]);
            }
            #[allow(clippy::cast_possible_truncation)]
            {
                let shown = if rng.below(2) == 0 {
                    current as u8
                } else {
                    rng.below(7) as u8
                };
                t.put(g + 0x115bc, &[shown]);
                let edge = [0u16, 4, 8, 12, rng.next() as u16][n as usize % 5];
                t.put(g + 0x11672, &edge.to_le_bytes());
            }
            t.check(
                &format!("change player {n}"),
                ABMAP_TRY_CHANGE_PLAYER,
                Conv::Cdecl,
                &[],
                |b| Res::Int(abmap::try_change_player(&mut Run(b))),
            );
        }
    }
}

/// A float for tag geometry: small, signed, odd, or special.
fn tag_float(rng: &mut Xorshift) -> f32 {
    match rng.below(6) {
        0 => f32::from_bits(FLOATS[rng.below(12) as usize]),
        #[allow(clippy::cast_precision_loss)]
        1 => rng.below(70_000) as f32,
        #[allow(clippy::cast_precision_loss)]
        _ => (rng.below(4000) as f32 - 2000.0) / 7.0,
    }
}

#[test]
fn abmap_init_ply_tag_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0007);
        let mut log = Log::new(&mut t);
        let names = t.alloc(0x1000);
        log.stub(
            &mut t,
            abmap::TO_GET_SAVE_CHR_NAME,
            1,
            1,
            (names, 0xff, 0x10),
        );
        let width = t.alloc(4);
        log.float = Some(width);
        log.stub(&mut t, abmap::TO_GET_TEXT_WIDTH, 2, 2, (0, 0, 0));
        log.float = None;
        let min: f32 = t.b.read(t.b.rebase(abmap::TAG_MIN_WIDTH));
        for n in 0..600u32 {
            log.rewind(&mut t);
            let g = made_up_abmap(&mut t, &mut rng, 0x11700);
            let w = match n % 4 {
                0 => min,
                1 => f32::from_bits(min.to_bits() + 1),
                _ => tag_float(&mut rng),
            };
            t.put(width, &w.to_le_bytes());
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let chr = (rng.below(12) as i32 - 3) as u32;
            let node = rng.below(0x5f0);
            #[allow(clippy::cast_possible_truncation)]
            let kind = rng.below(0x20) as u16;
            t.put(g + 0x80e + node * 0x28, &kind.to_le_bytes());
            let args = [
                chr,
                node,
                tag_float(&mut rng).to_bits(),
                rng.next(),
                rng.next(),
                rng.next(),
            ];
            t.check(
                &format!("init tag {n} chr {chr} width {w}"),
                ABMAP_INIT_PLY_TAG,
                Conv::Cdecl,
                &args,
                |b| Res::Int(abmap::init_ply_tag(&mut Run(b), args)),
            );
        }
    }
}

#[test]
fn abmap_position_ply_tag_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0008);
        let log = Log::new(&mut t);
        // the image never ran the start-up code that fills the sines
        let sines = t.a.rebase(abmap::SIN_TABLE);
        let quarter: f32 = t.b.read(sines + 4 * 0x400);
        if quarter == 0.0 {
            let v: Vec<u8> = (0..0x1000)
                .flat_map(|_| tag_float(&mut rng).to_le_bytes())
                .collect();
            t.put(sines, &v);
        }
        let tag_box = t.a.rebase(abmap::TAG_BOX);
        for n in 0..600u32 {
            log.rewind(&mut t);
            let g = made_up_abmap(&mut t, &mut rng, 0x11700);
            t.put(tag_box, &rng.bytes(0x30));
            #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
            let chr = (rng.below(12) as i32 - 3) as u32;
            let tag = g.wrapping_add(0x11088).wrapping_add(chr.wrapping_mul(0x50));
            let mut fields = Vec::new();
            for _ in 0..2 {
                fields.extend(tag_float(&mut rng).to_le_bytes());
            }
            t.put(tag, &fields);
            t.put(tag + 0x3c, &tag_float(&mut rng).to_le_bytes());
            // the setup sometimes moves the corner the anchor already used
            let poke = (n % 3 == 0).then(|| (tag + 0x4c, rng.next()));
            log.stub_poke(&mut t, abmap::ABMAP_SETUP_PLY_TAG, 1, 1, (0, 0, 0), poke);
            t.check(
                &format!("position tag {n} chr {chr}"),
                ABMAP_POSITION_PLY_TAG,
                Conv::Cdecl,
                &[chr],
                |b| Res::Int(abmap::position_ply_tag(&mut Run(b), chr)),
            );
        }
    }
}

#[test]
fn spd_ctrl_is_map_excluded_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x5bd0_0001);
        let log = Log::new(&mut t);
        let table = t.a.rebase(spdctrl::EXCLUDED_MAPS.sym);
        let maps: Vec<u32> = (0..3)
            .map(|k| {
                u32::from_le_bytes(
                    Oracle::read(&mut t.a, table + 4 * k, 4)
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
            })
            .collect();
        for n in 0..800u32 {
            log.rewind(&mut t);
            let status = [1, 0, 2, rng.next()][n as usize % 4];
            log.stub(&mut t, spdctrl::MS_BATTLE_GET_STATUS, 1, 0, (status, 0, 0));
            let scenario = match rng.below(3) {
                0 => 0x71b + rng.below(8),
                1 => rng.next(),
                _ => (rng.next() & 0xffff_0000) | (0x71c + rng.below(6)),
            };
            log.stub(&mut t, spdctrl::GET_SCENERIO_FLAG, 2, 0, (scenario, 0, 0));
            let map = match rng.below(4) {
                0 => 0x118,
                1 => maps[rng.below(3) as usize],
                2 => rng.below(0x200),
                _ => rng.next(),
            };
            t.check(
                &format!("map {map:#x} status {status:#x} scenario {scenario:#x}"),
                SPD_CTRL_IS_MAP_EXCLUDED,
                Conv::Cdecl,
                &[map],
                |b| Res::Int(spdctrl::is_map_excluded(&mut Run(b), map)),
            );
        }
    }
}

#[test]
fn spd_ctrl_is_magic_excluded_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x5bd0_0002);
        let log = Log::new(&mut t);
        let window = t.a.rebase(spdctrl::BTL_WINDOW_OPEN.sym);
        let mut ids: Vec<u32> = (0x140..0x2d0).collect();
        ids.extend([
            0,
            0xffff_ffff,
            0x8000_0000,
            0x8000_0170,
            0xffff_ff70,
            0x1_015b,
            0x1_01b7,
            0x7fff_ffff,
        ]);
        for (n, &id) in ids.iter().chain(&ids).enumerate() {
            log.rewind(&mut t);
            let handle = if n % 7 == 0 { u32::MAX } else { rng.below(4) };
            log.stub(&mut t, spdctrl::MAGIC_FILE_GET_HANDLE, 1, 0, (handle, 0, 0));
            log.stub(&mut t, spdctrl::MAGIC_FILE_GET_MAGIC_ID, 2, 0, (id, 0, 0));
            let control = [1, 0, 0x101, rng.next()][rng.below(4) as usize];
            log.stub(
                &mut t,
                spdctrl::TO_GET_BTL_WINDOW_CONTROL_ENABLE,
                3,
                0,
                (control, 0, 0),
            );
            let open = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
            t.put(window, &open.to_le_bytes());
            t.check(
                &format!("magic {id:#x} handle {handle:#x} open {open:#x} control {control:#x}"),
                SPD_CTRL_IS_MAGIC_EXCLUDED,
                Conv::Cdecl,
                &[],
                |b| Res::Int(spdctrl::is_magic_excluded(&mut Run(b))),
            );
        }
    }
}

#[test]
fn menu_item_parse_target_keys_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0101);
        let log = Log::new(&mut t);
        let none = (0, 0, 0);
        log.stub(
            &mut t,
            item::TO_MENU_GET_NAME_MES,
            2,
            1,
            (0x0100_0000, 0xffff, 8),
        );
        log.stub(&mut t, item::MENU_PARSE_ITEM_KEYS, 3, 1, none);
        for n in 0..300u32 {
            log.rewind(&mut t);
            let lang = [0, 1, 0xb, 0x100, rng.next()][n as usize % 5];
            log.stub(&mut t, item::TO_GET_FFX_LANG_JMP, 1, 0, (lang, 0, 0));
            #[allow(clippy::cast_possible_wrap)]
            let target = match n % 5 {
                0 => -(rng.below(5) as i32) - 1,
                1 => i32::MIN,
                _ => rng.below(0x40) as i32,
            };
            t.check(
                &format!("target keys {target} lang {lang:#x}"),
                MENU_ITEM_PARSE_TARGET_KEYS,
                Conv::Cdecl,
                &[target.cast_unsigned()],
                |b| {
                    item::parse_target_keys(&mut Run(b), target);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn menu_item_get_line_pos_matches_the_original() {
    for mut t in twins() {
        let log = Log::new(&mut t);
        log.stub(&mut t, item::TK_MN2_SEQ_GET_POS, 1, 2, (0, 0, 0));
        t.check("line pos", MENU_ITEM_GET_LINE_POS, Conv::Cdecl, &[], |b| {
            item::get_line_pos(&mut Run(b));
            Res::Void
        });
    }
}

#[test]
fn menu_item_draw_row_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0102);
        let log = Log::new(&mut t);
        let none = (0, 0, 0);
        log.stub(&mut t, item::TO_DRAW_MENU_PLATE_XYWH_TYPE, 1, 5, none);
        log.stub(&mut t, item::TK_MN2_DRAW_CROSS_CMD_CURSOR, 2, 3, none);
        log.stub(
            &mut t,
            item::MS_GET_SAVE_WEAPON,
            3,
            1,
            (0x0200_0000, 0xffff, 0x10),
        );
        log.stub(&mut t, item::DRAW_CROSS_MENU_ICON_WEAPON_NAME2, 4, 4, none);
        let window = t.alloc(0x100);
        let list = t.a.rebase(item::ITEM_LIST);
        for n in 0..600u32 {
            log.rewind(&mut t);
            t.put(window, &rng.bytes(0x100));
            let mut entries = rng.bytes(0x800);
            for e in entries.chunks_mut(4) {
                if rng.below(2) == 0 {
                    e[2] = 0;
                }
            }
            t.put(list, &entries);
            #[allow(clippy::cast_possible_truncation)]
            let rows = match n % 7 {
                0 => rng.below(0x10) as u16,
                1 => 0,
                // rows past 0x7fff: the cursor row is signed
                2 => 0xffff,
                _ => rng.below(0x200) as u16,
            };
            t.put(window + 0x30, &rows.to_le_bytes());
            #[allow(clippy::cast_possible_wrap)]
            let index = match n % 6 {
                0 => i32::from(rows),
                1 => i32::from(rows) - 1,
                2 => -(rng.below(4) as i32) - 1,
                _ if rows == 0xffff => 0x8000 + rng.below(0x100) as i32,
                _ => rng.below(u32::from(rows) + 2) as i32,
            };
            if rng.below(2) == 0 {
                #[allow(clippy::cast_possible_truncation)]
                let c = index as i16;
                t.put(window + 0x2c, &c.to_le_bytes());
            }
            let shown = [0u8, 1, 0x7f, 0x80, 0xff][rng.below(5) as usize];
            t.put(window + 0x45, &[shown]);
            let (x, y) = (tag_float(&mut rng), tag_float(&mut rng));
            t.check(
                &format!("draw row {n} index {index} of {rows}"),
                MENU_ITEM_DRAW_ROW,
                Conv::Cdecl,
                &[window, x.to_bits(), y.to_bits(), index.cast_unsigned()],
                |b| {
                    item::draw_row(&mut Run(b), window, x, y, index);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn rcui_sea_add_uv_spd_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0103);
        let log = Log::new(&mut t);
        log.stub(&mut t, sea::YN_SEA_SET_UV_SPD, 1, 2, (0, 0, 0));
        let flag = t.a.rebase(sea::SPEED_SET.sym);
        let speeds = t.a.rebase(sea::SPEED);
        for n in 0..600u32 {
            log.rewind(&mut t);
            let set = [0u32, 1, rng.next(), 0x100][n as usize % 4];
            t.put(flag, &set.to_le_bytes());
            let mut v = Vec::new();
            for _ in 0..6 {
                v.extend(tag_float(&mut rng).to_le_bytes());
            }
            t.put(speeds - 8, &v);
            let axis = match n % 5 {
                0 => 0,
                1 => 1,
                2 => 0xffff_fffe,
                _ => rng.below(4),
            };
            let mut step = tag_float(&mut rng);
            // speed + step / 10 at an f32 tie that step * 0.1 misses
            if let Some(&(speed, st)) = [
                (0x31cc_cccd_u32, 0xc112_13c4_u32),
                (0x354c_cccd, 0xc46a_b47b),
            ]
            .get(n as usize % 50)
            {
                t.put(speeds + 4 * axis, &speed.to_le_bytes());
                step = f32::from_bits(st);
            }
            t.check(
                &format!("sea {n} axis {axis:#x} step {step}"),
                RCUI_SEA_ADD_UV_SPD,
                Conv::Cdecl,
                &[axis, step.to_bits()],
                |b| {
                    sea::add_uv_spd(&mut Run(b), axis, step);
                    Res::Void
                },
            );
        }
    }
}

/// A small `i16` for list fields: mostly 0..24, sometimes negative or
/// large.
fn list_word(rng: &mut Xorshift) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    match rng.below(10) {
        0 => rng.next() as i16,
        1 => -(rng.below(4) as i16),
        _ => rng.below(25) as i16,
    }
}

#[test]
fn abmap_pad_ctrl_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0104);
        let log = Log::new(&mut t);
        log.stub(&mut t, abmap::SND_SEP_PLAY_SIMPLE, 1, 1, (0, 0, 0));
        let handler = log.func(&mut t, 2, 4, (0x0bad_0000, 0, 0));
        let pads = t.alloc(0x2800);
        let at = t.a.rebase(panel::PAD.sym);
        t.put(at, &pads.to_le_bytes());
        let g = made_up_abmap(&mut t, &mut rng, 0x11700);
        for n in 0..3000u32 {
            log.rewind(&mut t);
            t.put(g + 0x1_1600, &rng.bytes(0x100));
            t.put(pads, &rng.bytes(0x2800));
            t.put(pads + 0x2704, &handler.to_le_bytes());
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let open = if n % 20 == 0 {
                -(rng.below(3) as i16) - 1
            } else {
                rng.below(12) as i16
            };
            t.put(pads + 0x2700, &open.to_le_bytes());
            if open >= 0 {
                #[allow(clippy::cast_sign_loss)]
                let p = pads + open as u32 * 0x340;
                let page = list_word(&mut rng);
                let count = if rng.below(3) == 0 {
                    list_word(&mut rng)
                } else {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    let c = page.wrapping_add(rng.below(20) as i16);
                    c
                };
                for (o, v) in [
                    (0x16, page),
                    (0x18, list_word(&mut rng)),
                    (0x1a, list_word(&mut rng)),
                    (0x1e, count),
                ] {
                    t.put(p + o, &v.to_le_bytes());
                }
                if rng.below(8) != 0 {
                    t.put(p + 0x26, &[0]);
                }
                // fewer locked entries
                for e in 0..30 {
                    if rng.below(3) != 0 {
                        t.put(p + 0x44 + 12 * e, &[0]);
                    }
                }
            }
            #[allow(clippy::cast_possible_truncation)]
            {
                let bits = [1u16, 2, 3, 0x1000, 0x4000, 0x5000, 0x1001, 0x4002];
                let b = bits[rng.below(8) as usize]
                    | if rng.below(4) == 0 {
                        rng.next() as u16
                    } else {
                        0
                    };
                t.put(g + 0x1_1674, &b.to_le_bytes());
                let edges = [0u16, 0x20, 0x40, 0x60];
                let e = edges[rng.below(4) as usize]
                    | if rng.below(4) == 0 {
                        rng.next() as u16
                    } else {
                        0
                    };
                t.put(g + 0x1_166e, &e.to_le_bytes());
            }
            t.check(
                &format!("pad ctrl {n}"),
                ABMAP_PAD_CTRL,
                Conv::Cdecl,
                &[],
                |b| {
                    panel::pad_ctrl(&mut Run(b));
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn abmap_read_cursor_stick_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0105);
        // the stick: table[axis] plus the other three arguments plus 1
        let table = t.alloc(0x10);
        let mut code = vec![0x8b, 0x44, 0x24, 0x0c, 0x8b, 0x04, 0x85];
        code.extend(table.to_le_bytes());
        code.extend([0x03, 0x44, 0x24, 0x04, 0x03, 0x44, 0x24, 0x08]);
        code.extend([0x03, 0x44, 0x24, 0x10, 0x40, 0xc3]);
        let at = t.a.rebase(abmap::TO_MENU_GET_CONTROL_PAD_ANALOG_HIST);
        t.jump(at, &code);
        let mv = t.alloc(0x10);
        let lows = [0x3fu32, 0x40, 0x41, 0x80, 0xbf, 0xc0, 0xc1, 0, 0xff, 0x7f];
        let g = made_up_abmap(&mut t, &mut rng, 0x11700);
        for n in 0..3000u32 {
            t.put(g + 0x1_1500, &rng.bytes(0x200));
            let axis = |rng: &mut Xorshift| {
                let low = if rng.below(4) == 0 {
                    rng.below(0x100)
                } else if rng.below(2) == 0 {
                    0x41 + rng.below(0x7f)
                } else {
                    lows[rng.below(10) as usize]
                };
                (rng.next() & !0xff) | low
            };
            let (mut a, mut b) = (axis(&mut rng), axis(&mut rng));
            // exactly at the fast push (x² + y² = 0x3840: 120 or 96, 72)
            match n % 25 {
                0 => (a, b) = (0xf8, 0x80),
                1 => (a, b) = (0x80 + 96, 0x80 - 72),
                _ => {}
            }
            t.put(table + 8, &a.to_le_bytes());
            t.put(table + 12, &b.to_le_bytes());
            t.put(mv, &rng.bytes(0x10));
            let speeds = [1.0f32, 2.0, 0.95, 1.05, 1.95, 2.05, 1.5, f32::NAN];
            #[allow(clippy::cast_precision_loss)]
            let speed = if rng.below(8) == 0 {
                // -0.1 + 0.1 is not 0 on the x87 (the step is a double)
                f32::from_bits(0xbdcc_cccd + rng.below(3) - 1)
            } else if rng.below(4) == 0 {
                tag_float(&mut rng)
            } else if rng.below(2) == 0 {
                // near the bounds, where the steps round
                0.9 + rng.below(1 << 20) as f32 / (1 << 20) as f32 * 1.3
            } else {
                speeds[rng.below(8) as usize]
            };
            t.put(g + 0x1_15e4, &speed.to_le_bytes());
            #[allow(clippy::cast_possible_truncation)]
            {
                let held = [
                    0u16, 0x8000, 0x2000, 0x1000, 0x4000, 0xa000, 0x5000, 0x9000, 0x6000,
                ][rng.below(9) as usize]
                    | if rng.below(3) == 0 {
                        rng.next() as u16
                    } else {
                        0
                    };
                t.put(g + 0x1_166c, &held.to_le_bytes());
                let count = [0u8, 1, 2, 0xff, rng.next() as u8][rng.below(5) as usize];
                t.put(g + 0x1_15ca, &[count]);
            }
            t.check(
                &format!("stick {n} {a:#x} {b:#x} speed {speed}"),
                ABMAP_READ_CURSOR_STICK,
                Conv::Cdecl,
                &[mv],
                |o| Res::Int(cursor::read_cursor_stick(&mut Run(o), mv)),
            );
        }
    }
}

/// A made-up sphere grid for the node searches: up to 60 nodes near the
/// origin, some without a kind; `vf00.x` sometimes not 0.
fn made_up_nodes(t: &mut Twin, rng: &mut Xorshift, g: u32, n: u32) {
    t.put(g, &rng.bytes(0x808 + 0x28 * 60));
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let count: i16 = match n % 9 {
        0 => -(rng.below(3) as i16),
        _ => rng.below(60) as i16,
    };
    t.put(g + 2, &count.to_le_bytes());
    for k in 0..60 {
        let p = g + 0x808 + 0x28 * k;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        for o in [0, 2] {
            let v = rng.below(600) as i16 - 300;
            t.put(p + o, &v.to_le_bytes());
        }
        if rng.below(5) == 0 {
            t.put(p + 6, &[0xff, 0xff]);
        }
    }
    let vf00 = if rng.below(4) == 0 {
        tag_float(rng)
    } else {
        0.0
    };
    let at = t.a.rebase(cursor::VU0_VF00.sym);
    t.put(at, &vf00.to_le_bytes());
}

/// A float near the grid's nodes, sometimes special.
fn grid_float(rng: &mut Xorshift) -> f32 {
    if rng.below(8) == 0 {
        tag_float(rng)
    } else {
        #[allow(clippy::cast_precision_loss)]
        let v = (rng.below(6000) as f32 - 3000.0) / 10.0;
        v
    }
}

#[test]
fn abmap_find_nearest_node_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0106);
        let log = Log::new(&mut t);
        let filter = log.func(&mut t, 1, 1, (0, 0x10, 1));
        let pos = t.alloc(8);
        let dist = t.alloc(4);
        let g = made_up_abmap(&mut t, &mut rng, 0x11700);
        for n in 0..1500u32 {
            log.rewind(&mut t);
            made_up_nodes(&mut t, &mut rng, g, n);
            let p = [grid_float(&mut rng), grid_float(&mut rng)];
            t.put(pos, &p[0].to_le_bytes());
            t.put(pos + 4, &p[1].to_le_bytes());
            t.put(dist, &rng.bytes(4));
            let f = if n % 2 == 0 { 0 } else { filter };
            t.check(
                &format!("nearest {n} {p:?}"),
                ABMAP_FIND_NEAREST_NODE,
                Conv::Cdecl,
                &[dist, pos, f],
                |o| Res::Int(cursor::find_nearest_node(&mut Run(o), dist, pos, f)),
            );
        }
    }
}

#[test]
fn abmap_find_node_in_direction_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0107);
        let log = Log::new(&mut t);
        let filter = log.func(&mut t, 1, 1, (0, 0x10, 1));
        let pos = t.alloc(8);
        let dir = t.alloc(12);
        let g = made_up_abmap(&mut t, &mut rng, 0x11700);
        for n in 0..1500u32 {
            log.rewind(&mut t);
            made_up_nodes(&mut t, &mut rng, g, n);
            let p = [grid_float(&mut rng), grid_float(&mut rng)];
            t.put(pos, &p[0].to_le_bytes());
            t.put(pos + 4, &p[1].to_le_bytes());
            let units = [
                (1.0f32, 0.0f32),
                (-1.0, 0.0),
                (0.0, 1.0),
                (0.0, -1.0),
                (0.6, 0.8),
            ];
            // two nodes whose scores order differently with d² rounded to
            // f32 (found by search): the second must win
            let pair = [[(9i16, 93i16), (2, -44)], [(20, 100), (8, -64)]].get(n as usize % 40);
            if let Some(pair) = pair {
                let g = abmap::ABMAP.get(&t.b);
                t.put(g + 2, &2i16.to_le_bytes());
                for (k, (x, y)) in (0u32..).zip(pair) {
                    let node = g + 0x808 + 0x28 * k;
                    t.put(node, &x.to_le_bytes());
                    t.put(node + 2, &y.to_le_bytes());
                    t.put(node + 6, &[0, 0]);
                }
                t.put(pos, &[0; 8]);
                let at = t.a.rebase(cursor::VU0_VF00.sym);
                t.put(at, &[0; 4]);
            }
            // one node at 0.5 along the direction (not ahead enough), or
            // ahead at length 0 (vf00.x = -1)
            if matches!(n % 40, 2 | 3) {
                let g = abmap::ABMAP.get(&t.b);
                t.put(g + 2, &1i16.to_le_bytes());
                let node = g + 0x808;
                t.put(node, &10i16.to_le_bytes());
                t.put(node + 2, &(-7i16).to_le_bytes());
                t.put(node + 6, &[0, 0]);
                let back = if n % 40 == 2 { 0.5f32 } else { 1.0 };
                t.put(pos, &(10.0 - back).to_le_bytes());
                t.put(pos + 4, &(-7.0f32).to_le_bytes());
                let at = t.a.rebase(cursor::VU0_VF00.sym);
                let vf00 = if n % 40 == 2 { 0.0f32 } else { -1.0 };
                t.put(at, &vf00.to_le_bytes());
            }
            let (x, y) = if pair.is_some() || matches!(n % 40, 2 | 3) {
                (1.0, 0.0)
            } else if rng.below(3) == 0 {
                (tag_float(&mut rng), tag_float(&mut rng))
            } else {
                units[rng.below(5) as usize]
            };
            let z = if rng.below(6) == 0 {
                tag_float(&mut rng)
            } else {
                0.0
            };
            for (k, v) in [x, y, z].into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                t.put(dir + 4 * k as u32, &v.to_le_bytes());
            }
            let f = if n % 2 == 0 || n % 40 == 3 { 0 } else { filter };
            t.check(
                &format!("direction {n} {p:?} ({x}, {y}, {z})"),
                ABMAP_FIND_NODE_IN_DIRECTION,
                Conv::Cdecl,
                &[pos, dir, f],
                |o| Res::Int(cursor::find_node_in_direction(&mut Run(o), pos, dir, f)),
            );
        }
    }
}

#[test]
fn abmap_calc_link_points_straight_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0108);
        let out = t.alloc(0x40);
        let ends = t.alloc(0x10);
        let kind = t.alloc(4);
        let vf00 = t.a.rebase(cursor::VU0_VF00.sym);
        for n in 0..3000u32 {
            t.put(out, &rng.bytes(0x40));
            t.put(kind, &rng.bytes(4));
            let mut p: Vec<f32> = (0..4).map(|_| grid_float(&mut rng)).collect();
            match n % 10 {
                // the same point: length 0
                0 => (p[2], p[3]) = (p[0], p[1]),
                1 => p[2] = p[0],
                _ => {}
            }
            let v: Vec<u8> = p.iter().flat_map(|f| f.to_le_bytes()).collect();
            t.put(ends, &v);
            let (a, b) = (ends, ends + 8);
            // vf00 is (0, 0, 0, 1) in the game; other values now and then
            let regs = if rng.below(4) == 0 {
                [
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                ]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            };
            let r: Vec<u8> = regs.iter().flat_map(|f| f.to_le_bytes()).collect();
            t.put(vf00, &r);
            let (ra, rb) = (tag_float(&mut rng), tag_float(&mut rng));
            let args = [out, a, b, ra.to_bits(), rb.to_bits(), kind];
            t.check(
                &format!("straight {n} {p:?} {ra} {rb} {regs:?}"),
                ABMAP_CALC_LINK_POINTS_STRAIGHT,
                Conv::Cdecl,
                &args,
                |o| Res::Int(link::calc_link_points_straight(o, out, a, b, ra, rb, kind)),
            );
        }
    }
}

/// `fpatan; ret` behind `_CIatan2`'s import slot in both oracles (the
/// oracle has no stand-in for it; the CRT computes it the same way at 53
/// bits), and a thunk running it on two doubles in memory for the bridge:
/// returns (thunk, data: y, x, then the f32 result).
fn atan2_stand_in(t: &mut Twin) -> (u32, u32) {
    let code = [0xd9, 0xf3, 0xc3];
    let f = t.a.alloc_code(&code).unwrap();
    assert_eq!(t.b.alloc_code(&code).unwrap(), f);
    let slot = t.a.rebase(link::CI_ATAN2_SLOT);
    t.put(slot, &f.to_le_bytes());
    let data = t.alloc(0x14);
    t.put(data, &[0; 0x14]);
    let mut c = vec![0xdd, 0x05]; // fld qword [y]
    c.extend(data.to_le_bytes());
    c.extend([0xdd, 0x05]); // fld qword [x]
    c.extend((data + 8).to_le_bytes());
    c.extend([0xff, 0x15]); // call [slot]
    c.extend(slot.to_le_bytes());
    c.extend([0xd9, 0x1d]); // fstp dword [out]
    c.extend((data + 0x10).to_le_bytes());
    c.push(0xc3);
    let thunk = t.a.alloc_code(&c).unwrap();
    assert_eq!(t.b.alloc_code(&c).unwrap(), thunk);
    (thunk, data)
}

/// The bridge's `atan2` in the second oracle: the stand-in through the
/// thunk, its scratch cleared after.
fn oracle_atan2(thunk: u32, data: u32) -> impl FnMut(&mut Oracle, f64, f64) -> f32 {
    move |m, y, x| {
        Mem::write(m, data, y);
        Mem::write(m, data + 8, x);
        m.call(thunk, Conv::Cdecl, &[]).unwrap();
        let r: f32 = Mem::read(m, data + 0x10);
        Mem::write(m, data, [0u32; 5]);
        r
    }
}

#[test]
fn abmap_calc_link_points_curved_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_0109);
        let (thunk, data) = atan2_stand_in(&mut t);
        let out = t.alloc(0x100);
        let ends = t.alloc(0x18);
        let kind = t.alloc(4);
        let vf00 = t.a.rebase(cursor::VU0_VF00.sym);
        // the image never ran the start-up code that fills the sines
        let sines = t.a.rebase(link::SIN_TABLE);
        let quarter: f32 = t.b.read(sines + 4 * 0x400);
        if quarter == 0.0 {
            let v: Vec<u8> = (0..0x1000)
                .flat_map(|_| tag_float(&mut rng).to_le_bytes())
                .collect();
            t.put(sines, &v);
        }
        for n in 0..2000u32 {
            t.put(out, &rng.bytes(0x100));
            t.put(kind, &rng.bytes(4));
            let mut p: Vec<f32> = (0..6).map(|_| grid_float(&mut rng)).collect();
            match n % 10 {
                // an end on the centre, both ends alike
                0 => (p[4], p[5]) = (p[0], p[1]),
                1 => (p[2], p[3]) = (p[0], p[1]),
                // the ends at angles 0 and pi (f32): a difference of
                // exactly +-pi stays
                2 => p = vec![10.0, 0.0, -10.0, 0.0, 0.0, 0.0],
                3 => p = vec![-10.0, 0.0, 10.0, 0.0, 0.0, 0.0],
                _ => {}
            }
            let v: Vec<u8> = p.iter().flat_map(|f| f.to_le_bytes()).collect();
            t.put(ends, &v);
            let pts = [ends, ends + 8, ends + 16];
            let regs = if rng.below(4) == 0 {
                [
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                    tag_float(&mut rng),
                ]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            };
            let r: Vec<u8> = regs.iter().flat_map(|f| f.to_le_bytes()).collect();
            t.put(vf00, &r);
            let (ra, rb) = if rng.below(4) == 0 {
                (tag_float(&mut rng), tag_float(&mut rng))
            } else {
                #[allow(clippy::cast_precision_loss)]
                let r = |rng: &mut Xorshift| rng.below(400) as f32 / 10.0;
                (r(&mut rng), r(&mut rng))
            };
            let args = [
                out,
                pts[0],
                pts[1],
                pts[2],
                ra.to_bits(),
                rb.to_bits(),
                kind,
            ];
            t.check(
                &format!("curved {n} {p:?} {ra} {rb} {regs:?}"),
                ABMAP_CALC_LINK_POINTS_CURVED,
                Conv::Cdecl,
                &args,
                |o| {
                    Res::Int(link::calc_link_points_curved(
                        o,
                        out,
                        pts,
                        ra,
                        rb,
                        kind,
                        oracle_atan2(thunk, data),
                    ))
                },
            );
        }
    }
}

#[test]
fn gui_touch_init_group_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_010a);
        let group = t.alloc(0x1d0);
        for n in 0..20 {
            t.put(group, &rng.bytes(0x1d0));
            t.check(
                &format!("init group {n}"),
                GUI_TOUCH_INIT_GROUP,
                Conv::Fastcall,
                &[group],
                |o| {
                    touch::init_group(o, group);
                    Res::Void
                },
            );
        }
    }
}

#[test]
fn abmap_pad_activate_handler_matches_the_original() {
    for mut t in twins() {
        let mut rng = Xorshift(0x00ab_010b);
        let log = Log::new(&mut t);
        let none = (0, 0, 0);
        log.stub(&mut t, panel::ABMAP_RESET_HELP, 1, 0, none);
        log.stub(&mut t, abmap::SND_SEP_PLAY_SIMPLE, 2, 1, none);
        log.stub(&mut t, panel::SPEND_SLV, 3, 2, none);
        log.stub(&mut t, panel::ABMAP_SET_CHR_MARKER, 4, 1, none);
        log.stub(&mut t, panel::ABMAP_POSITION_PLY_TAG, 5, 1, none);
        log.stub(&mut t, abmap::ABMAP_RESET_CURSOR_CAMERA, 6, 0, none);
        log.stub(&mut t, abmap::ABMAP_INIT_HOMING, 7, 2, none);
        let pads = t.alloc(0x2800);
        let at = t.a.rebase(panel::PAD.sym);
        t.put(at, &pads.to_le_bytes());
        let g = made_up_abmap(&mut t, &mut rng, 0x11700);
        for n in 0..600u32 {
            log.rewind(&mut t);
            t.put(pads + 0x1600, &rng.bytes(0x800));
            t.put(g + 0xa800, &rng.bytes(0x14 * 82));
            t.put(g + 0x1_1000, &rng.bytes(0x700));
            #[allow(clippy::cast_possible_truncation)]
            {
                let links = [0u16, 1, rng.below(80) as u16][n as usize % 3];
                t.put(g + 4, &links.to_le_bytes());
                t.put(g + 0x1_1638, &[rng.below(18) as u8]);
                if rng.below(2) == 0 {
                    t.put(g + 0x1_15bc, &[rng.below(8) as u8]);
                }
            }
            match rng.below(3) {
                0 => t.put(g + 0x1_15b0, &[0; 4]),
                1 => t.put(g + 0x1_15b0, &0x100u32.to_le_bytes()),
                _ => {}
            }
            let choice = [0u32, 1, 1, 2, 0xffff_ffff, 0x100, 0x101][n as usize % 7];
            let (panel_at, entry) = (rng.next(), rng.next());
            t.check(
                &format!("activate handler {n} choice {choice:#x}"),
                ABMAP_PAD_ACTIVATE_HANDLER,
                Conv::Cdecl,
                &[panel_at, entry, choice, 1],
                |o| Res::Int(panel::activate_handler(&mut Run(o), choice)),
            );
        }
    }
}
