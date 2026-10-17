//! The PC UI functions (`ffx_pcui`) against the original: two oracles
//! started alike, one runs `FFX.exe`'s code, the other the Rust bridge;
//! results (`eax`, or `ST0` for float results) and every written byte must
//! agree. Every test runs on the image alone (`FFX_GAME_DIR`), then on a
//! snapshot of the running game when `FFX_SNAPSHOT` is set; the sphere
//! grid tests put a made-up `g_Abmap` in both (the snapshots are not on the
//! sphere grid screen) and also read the snapshot's own when it has one.
//! Float arguments include signed zeros, denormals, infinities and quiet
//! NaNs, never signalling ones (the emulator keeps them through `fld m32`).

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pcui::abmap::bridge as abmap;
use ffx_pcui::iggy::bridge as iggy;

const IGGY_SET_INT: u32 = 0x00a2_8d30;
const IGGY_SET_UTF8: u32 = 0x00a2_8d70;
const IGGY_SET_BOOL: u32 = 0x00a2_8db0;
const HANDLERS_ADD_CALL: u32 = 0x00a2_8df0;
const HANDLERS_ADD: u32 = 0x00a2_8e40;
const IGGY_MENU_SET_ENABLE: u32 = 0x00a2_8ef0;
const ABMAP_ZOOM_SCALE: u32 = 0x00a5_6820;
const ABMAP_NODE_RADIUS: u32 = 0x00a5_6f30;
const ABMAP_RANGES_OVERLAP: u32 = 0x00a5_7000;

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
