//! `ffx_vfx`'s high range (`0x00a30000..0x00a80000`: `hi_md`, `hi_opus`)
//! against the original, on two oracles started alike: one runs
//! `FFX.exe`'s code, the other the Rust bridge on the same memory; results
//! and every written byte must agree. Each test runs on the image alone
//! (synthetic state, needs `FFX_GAME_DIR`) and, with `FFX_SNAPSHOT`, on the
//! snapshot's state too.

#![allow(dead_code, clippy::many_single_char_names)]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ffx_bridge::{Global, Mem, Value};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_vfx::{hi_md, hi_opus, x87};

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

fn pages() -> Option<Arc<Pages>> {
    static PAGES: OnceLock<Option<Arc<Pages>>> = OnceLock::new();
    PAGES
        .get_or_init(|| {
            let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
                eprintln!("FFX_SNAPSHOT is not set; snapshot cases skipped");
                return None;
            };
            Some(Pages::read(PathBuf::from(snap)).unwrap())
        })
        .clone()
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
    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }
    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.below(u32::try_from(of.len()).unwrap()) as usize]
    }
    /// Float bits: specials (quiet NaNs, infinities, zeros, denormals) a
    /// fifth of the time, else ordinary values. No signalling NaNs: the
    /// oracle's x87 keeps them signalling where the hardware quiets them
    /// (`x87_nans_match_the_oracle`).
    fn float(&mut self) -> u32 {
        if self.chance(20) {
            let f = self.pick(&SPECIALS);
            return if x87::is_snan(f) { f | 0x0040_0000 } else { f };
        }
        let mantissa = self.next() & 0x007f_ffff;
        let exp = 100 + self.below(56); // about 2^-27 .. 2^28
        (self.next() & 0x8000_0000) | (exp << 23) | mantissa
    }
}

const SPECIALS: [u32; 14] = [
    0x0000_0000,
    0x8000_0000,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc0_0000,
    0x7fc1_2345,
    0xffe0_0001,
    0x7f80_0001, // signalling
    0xff9f_0000, // signalling
    0x0000_0001, // denormal
    0x807f_ffff, // denormal
    0x7f7f_ffff,
    0x3f80_0000,
];

/// The two oracles.
struct Pair {
    name: &'static str,
    a: Oracle,
    b: Oracle,
}

/// Free code space in the image (the end of `.text`), for the float thunk.
const THUNK: u32 = 0x00b0_bf00;

impl Pair {
    /// On the image alone, and on the snapshot when there is one.
    fn all() -> Vec<Pair> {
        let Some(exe) = exe() else { return Vec::new() };
        // Oracle::load sets the game thread's control word (0x027f)
        let image = Pair {
            name: "image",
            a: Oracle::load(&exe).unwrap(),
            b: Oracle::load(&exe).unwrap(),
        };
        let mut v = vec![image];
        if let Some(p) = pages() {
            v.push(Pair {
                name: "snapshot",
                a: Oracle::from_snapshot(&exe, &p).unwrap(),
                b: Oracle::from_snapshot(&exe, &p).unwrap(),
            });
        }
        v
    }

    fn snapshot(&self) -> bool {
        self.name == "snapshot"
    }

    fn sym(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn set<T: Value>(&mut self, addr: u32, v: T) {
        Mem::write(&mut self.a, addr, v);
        Mem::write(&mut self.b, addr, v);
    }

    fn put<T: Value>(&mut self, g: Global<T>, v: T) {
        let at = self.sym(g.sym);
        self.set(at, v);
    }

    fn get<T: Value>(&self, addr: u32) -> T {
        Mem::read(&self.a, addr)
    }

    fn alloc(&mut self, len: usize) -> u32 {
        let x = self.a.alloc(len).unwrap();
        assert_eq!(self.b.alloc(len).unwrap(), x);
        x
    }

    fn fill(&mut self, at: u32, len: u32, rng: &mut Xorshift) {
        for i in (0..len).step_by(4) {
            let v = rng.next();
            self.set(at + i, v);
        }
    }

    /// Runs the original (cdecl) in `a` and `rust` in `b`; checks the result
    /// (when `rust` gives one) and the memory; returns the original's.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        args: &[u32],
        rust: impl FnOnce(&mut Oracle) -> Option<u32>,
    ) -> u32 {
        let at = self.sym(sym);
        let r = self
            .a
            .call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", self.name));
        // a float the original leaves in ST0 is a result: the port must
        // return it (check_f32 and float_call), not only eax
        assert!(
            r.st0.is_none(),
            "{}: {what}: the original returns {:?} in ST0",
            self.name,
            r.st0
        );
        let mine = rust(&mut self.b);
        if let Some(m) = mine {
            assert_eq!(r.eax, m, "{}: {what}: result", self.name);
        }
        self.same(what);
        r.eax
    }

    fn same(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{}: {what}: memory differs (address, original, rust): {:x?}",
            self.name,
            &d[..d.len().min(16)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }
}

/// A callee run as the original, in the bridge's oracle.
fn call_original(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args).unwrap().eax
}

/// A stand-in at `sym` that logs its arguments (`words[k]` 0: the word
/// itself, else that many words it points to), 64 bytes per call after
/// the count; returns the count (or the dword at `ret`). Returns the log.
fn deref_recorder(p: &mut Pair, sym: u32, words: &[u8]) -> u32 {
    deref_recorder_with(p, sym, words, None)
}

/// [`deref_recorder`] that returns the dword at `ret` (when given)
/// instead of the count.
fn deref_recorder_with(p: &mut Pair, sym: u32, words: &[u8], ret: Option<u32>) -> u32 {
    let log = p.alloc(4 + 0x100 * 64);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x06]); // shl ecx, 6
    let mut pos = 0u32;
    for (k, &n) in (0u8..).zip(words) {
        c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp + 4 + 4k]
        if n == 0 {
            c.extend([0x89, 0x81]); // mov [ecx + slot], eax
            c.extend((log + 4 + 4 * pos).to_le_bytes());
            pos += 1;
        } else {
            for j in 0..n {
                c.extend([0x8b, 0x50, 4 * j]); // mov edx, [eax + 4j]
                c.extend([0x89, 0x91]); // mov [ecx + slot], edx
                c.extend((log + 4 + 4 * pos).to_le_bytes());
                pos += 1;
            }
        }
    }
    assert!(pos <= 15);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [count] or [ret]
    c.extend(ret.unwrap_or(log).to_le_bytes());
    c.push(0xc3);
    jump_to(p, sym, &c);
    log
}

/// Puts `code` in the oracles' code area and a jump to it at `sym`, so
/// stand-ins for neighbouring functions (`dbgPrintf` at `0x0062f500`,
/// `debug_exit_trace` 16 bytes on) do not overlap.
fn jump_to(p: &mut Pair, sym: u32, code: &[u8]) {
    let to = p.a.alloc_code(code).unwrap();
    assert_eq!(p.b.alloc_code(code).unwrap(), to);
    let at = p.sym(sym);
    let mut j = vec![0xe9]; // jmp to
    j.extend(to.wrapping_sub(at + 5).to_le_bytes());
    p.a.write(at, &j).unwrap();
    p.b.write(at, &j).unwrap();
}

// ---- hi_md ---------------------------------------------------------------

/// Writes a chain of tags from `packet + 0x20`: `(type byte, count)`
/// pairs, each tag `0x10 + count * step(type)` bytes on from the last;
/// returns the end of the chain (the last tag's address).
fn tags(p: &mut Pair, packet: u32, chain: &[(u8, i16)], step: impl Fn(u8) -> u32) -> u32 {
    let mut t = packet + 0x20;
    for (k, &(ty, n)) in chain.iter().enumerate() {
        p.set(t + 1, ty);
        p.set(t + 2, n);
        if k + 1 < chain.len() {
            t += 0x10 + u32::try_from(n).unwrap() * step(ty);
        }
    }
    t
}

#[test]
fn md_copy_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_9290);
        let sizes: [u32; 8] = p.get(p.sym(0x00c8_8aa8));
        let (buf, dst) = (p.alloc(0x2000), p.alloc(0x2000));
        let dbg = deref_recorder(&mut p, 0x0062_f500, &[0]);
        let exit = deref_recorder(&mut p, 0x0062_f510, &[0]);
        for round in 0..3000 {
            p.fill(buf, 0x2000, &mut rng);
            p.fill(dst, 0x800, &mut rng);
            let src = buf + 0x10 * rng.below(8);
            let mut chain = Vec::new();
            for _ in 0..rng.below(5) {
                let ty = rng.pick(&[0x00u8, 0x01, 0x03, 0x37, 0x45, 0x86, 0xc7]);
                let n = i16::try_from(rng.below(6)).unwrap();
                chain.push((ty, n));
            }
            // the end (0xf0), or a 0x10/0x20 tag that makes the size 0
            let end = if round % 7 == 0 {
                rng.pick(&[0x10u8, 0x2f])
            } else {
                rng.pick(&[0xf0u8, 0xff, 0xf5])
            };
            chain.push((end, 0));
            tags(&mut p, src, &chain, |ty| sizes[usize::from(ty & 7)]);
            for at in [0x12u32, 0x14] {
                // (a negative total wraps the size to 2 GB: the original faults)
                let v = i16::try_from(rng.below(0x40)).unwrap();
                p.set(src + at, v);
            }
            // a destination elsewhere, or over the packet (either side)
            let to = match rng.below(4) {
                0 => src + 4 * rng.below(8),
                1 => src.wrapping_sub(4 * rng.below(8)),
                _ => dst,
            };
            p.check("op_md_copy", 0x00a6_9290, &[src, to], |o| {
                Some(hi_md::bridge::op_md_copy(o, src, to, &mut call_original))
            });
        }
        let calls: u32 = p.get(dbg);
        assert!(calls > 0, "{}: no size-0 packet", p.name);
        assert_eq!(calls, p.get::<u32>(exit));
    }
}

/// A model list at `list`: `n` packets in `packets` (0x400 bytes each),
/// their tags of `types` with counts 0..`max`, ending in `end`.
fn md_list(p: &mut Pair, rng: &mut Xorshift, list: u32, packets: u32, types: &[u8], end: u8) {
    p.fill(list, 0x100, rng);
    p.fill(packets, 0x400 * 6, rng);
    let n = rng.below(6);
    p.set(list + 4, u16::try_from(n).unwrap());
    for k in 0..n {
        let at = packets + 0x400 * k;
        p.set(list + 0x10 + 0x10 * k, at);
        let mut chain = Vec::new();
        for _ in 0..rng.below(5) {
            let n = rng.pick(&[0i16, 1, 2, 3, 7]);
            chain.push((rng.pick(types), n));
        }
        chain.push((end, rng.pick(&[0i16, 5, -3])));
        tags(p, at, &chain, |_| 0x1c);
        // the tag's 0x20 bytes, as the walk steps (0x10 in `tags`)
        let mut t = at + 0x20;
        for &(ty, n) in &chain {
            p.set(t + 1, ty);
            p.set(t + 2, n);
            t += 0x20 + 0x1c * u32::try_from(n.max(0)).unwrap();
        }
    }
}

#[test]
fn md_list_sizes_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_a2b0);
        let (list, packets) = (p.alloc(0x100), p.alloc(0x400 * 6));
        for _ in 0..3000 {
            md_list(&mut p, &mut rng, list, packets, &[0x10], 0x11);
            p.check("op_md_list_count10", 0x00a6_a2b0, &[list], |o| {
                Some(hi_md::bridge::op_md_list_count10(o, list))
            });
            let end = rng.pick(&[0x10u8, 0x30]);
            md_list(
                &mut p,
                &mut rng,
                list,
                packets,
                &[0x10, 0x00, 0xfe, end],
                0xff,
            );
            p.check("opus_0582_break_bytes", 0x00a7_2310, &[list], |o| {
                Some(hi_md::bridge::opus_0582_break_bytes(o, list))
            });
        }
    }
}

#[test]
fn oef2_entry_links_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_a320);
        let (e, table) = (p.alloc(0x1000), p.alloc(0x400));
        let big = p.alloc(0x10_1000);
        assert!(table >= 0x10_0000);
        for _ in 0..3000 {
            p.fill(e, 0x1000, &mut rng);
            p.fill(table, 0x400, &mut rng);
            let i = rng.below(16);
            let rel = rng.chance(50);
            let t = if rel { 0x20 * rng.below(0x40) } else { table };
            // offsets just below 0x100000 (the entry in a large buffer)
            let (e, t) = if rel && rng.chance(20) {
                (big, 0x10_0000 - 0x20 * (rng.below(16) + 1))
            } else {
                (e, t)
            };
            p.set(e + 0x14, t);
            let slot = if rel { e + t } else { t } + 32 * i + 0x1c;
            let link = match rng.below(4) {
                0 => 0,
                1 => e,
                2 => rng.below(0x1000),
                _ => rng.next(),
            };
            p.set(slot, link);
            p.check("op_oef2_entry_link", 0x00a6_a320, &[e, i], |o| {
                Some(hi_md::bridge::op_oef2_entry_link(o, e, i))
            });
        }
    }
}

/// Finite float bits, mostly ordinary.
fn finite(rng: &mut Xorshift) -> u32 {
    loop {
        let f = rng.float();
        if f32::from_bits(f).is_finite() {
            return f;
        }
    }
}

#[test]
fn normal_light44_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_cf10);
        let (obj, model, work, area) =
            (p.alloc(0x40), p.alloc(0x40), p.alloc(0x400), p.alloc(0x200));
        let (m, c) = (p.alloc(0x40), p.alloc(0x40));
        let (lights, third) = (p.sym(0x00c4_9724), p.sym(0x0130_08e8));
        for _ in 0..1000 {
            for i in 0..0x1c {
                let x = finite(&mut rng);
                p.set(lights + 4 * i, x);
            }
            for i in 0..4 {
                let x = finite(&mut rng);
                p.set(third + 4 * i, x);
            }
            for at in [m, c] {
                for i in 0..16 {
                    let x = finite(&mut rng);
                    p.set(at + 4 * i, x);
                }
            }
            p.fill(area, 0x200, &mut rng);
            p.fill(model, 0x40, &mut rng);
            p.set(model + 0x28, m);
            p.set(model + 0x2c, c);
            p.set(work + 0x390, area);
            p.check(
                "op_normal_light44_ffx",
                0x00a6_cf10,
                &[obj, model, work],
                |o| {
                    Some(hi_md::bridge::op_normal_light44_ffx(
                        o,
                        model,
                        work,
                        &mut call_original,
                    ))
                },
            );
        }
    }
}

// ---- hi_opus -------------------------------------------------------------

/// The callee-saved registers `Oracle::call` starts the original with
/// (`edi`, `esi`, `ebx`): what a callee reading past its arguments finds
/// pushed by a function that saves them.
const STALE: [u32; 3] = [0x3333_3333, 0x2222_2222, 0x1111_1111];

#[test]
fn opus_0437_dots_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_e540);
        let (b, pos, colour) = (p.alloc(0x10 + 0x20 * 40), p.alloc(0x10), p.alloc(0x10));
        let r0 = p.sym(0x00c8_f508);
        for _ in 0..1000 {
            p.fill(b, 0x10 + 0x20 * 40, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            let n = rng.pick(&[0u32, 1, 2, 7, 40]);
            p.check("opus_0437_dot_init", 0x00a6_e540, &[b, n], |o| {
                Some(hi_opus::bridge::opus_0437_dot_init(
                    o,
                    b,
                    n,
                    STALE,
                    &mut call_original,
                ))
            });
            for _ in 0..rng.below(4) {
                if p.get::<u32>(b) == 0 {
                    p.set(b, rng.below(40) + 1);
                }
                if rng.chance(30) {
                    p.set(b + 4, rng.next());
                }
                for i in 0..4 {
                    let x = finite(&mut rng);
                    p.set(pos + 4 * i, x);
                }
                p.fill(colour, 0x10, &mut rng);
                p.check("opus_0437_dot_set", 0x00a6_e5d0, &[b, pos, colour], |o| {
                    Some(hi_opus::bridge::opus_0437_dot_set(
                        o,
                        b,
                        pos,
                        colour,
                        &mut call_original,
                    ))
                });
            }
        }
    }
}
fn check_deep_n(
    p: &mut Pair,
    what: &str,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle, u32) -> Option<u32>,
) {
    let n = u8::try_from(args.len()).unwrap();
    let at = p.sym(THUNK);
    let mut c = vec![0x81, 0xec, 0, 2, 0, 0]; // sub esp, 0x200
    for _ in 0..n {
        c.extend([0xff, 0xb4, 0x24]); // push [esp + 0x200 + 4n]
        c.extend((0x200 + 4 * u32::from(n)).to_le_bytes());
    }
    c.push(0xe8); // call sym
    let call_end = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(p.sym(sym).wrapping_sub(call_end).to_le_bytes());
    c.extend([0x81, 0xc4]); // add esp, 0x200 + 4n
    c.extend((0x200 + 4 * u32::from(n)).to_le_bytes());
    c.push(0xc3);
    p.a.write(at, &c).unwrap();
    p.b.write(at, &c).unwrap();
    let w = 4 * u32::from(n);
    let frame = p.b.stack_range().end - 0x100 - w - 4 - 0x200 - w - 4 - 4;
    let r =
        p.a.call(at, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("{}: {what}: {e}", p.name));
    assert!(
        r.st0.is_none(),
        "{}: {what}: the original returns {:?} in ST0",
        p.name,
        r.st0
    );
    if let Some(mine) = rust(&mut p.b, frame) {
        assert_eq!(r.eax, mine, "{}: {what}: result", p.name);
    }
    p.same(what);
}

/// Random scene lights (`Sg_LightGetDir`/`Sg_LightGetColor`'s cells):
/// finite floats, and now and then any (quiet NaNs, infinities).
fn scene_lights(p: &mut Pair, rng: &mut Xorshift) {
    let (lights, third) = (p.sym(0x00c4_9724), p.sym(0x0130_08e8));
    for i in 0..0x1c {
        let x = if rng.chance(10) {
            rng.float()
        } else {
            finite(rng)
        };
        p.set(lights + 4 * i, x);
    }
    for i in 0..4 {
        let x = if rng.chance(10) {
            rng.float()
        } else {
            finite(rng)
        };
        p.set(third + 4 * i, x);
    }
}

#[test]
fn light_conversions_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_91c0);
        let (colours, dirs) = (p.alloc(0x40), p.alloc(0x40));
        let r0 = p.sym(0x00c8_f508);
        let ambient = p.sym(0x00c4_9774);
        for round in 0..3000 {
            scene_lights(&mut p, &mut rng);
            p.fill(colours, 0x40, &mut rng);
            p.fill(dirs, 0x40, &mut rng);
            p.fill(r0, 0x10, &mut rng);
            p.check(
                "op_ffxlight_to_omdlight_0279",
                0x00a6_91c0,
                &[colours, dirs],
                |o| {
                    Some(hi_md::bridge::op_ffxlight_to_omdlight_0279(
                        o,
                        colours,
                        dirs,
                        &mut call_original,
                    ))
                },
            );
            // the ambient channels round 0..1, the edges and beyond
            for k in 0..4 {
                let x: f32 = match round % 3 {
                    0 => f32::from_bits(rng.float()),
                    1 => rng.pick(&[0.0, -0.0, 0.996_093_75, 0.999, 1.0, 1.001, -0.001, 0.5]),
                    _ => f32::from(u16::try_from(rng.below(300)).unwrap()) / 256.0 - 0.1,
                };
                p.set(ambient + 4 * k, x.to_bits());
            }
            check_deep_n(
                &mut p,
                "op_ffxambient_to_omdambient",
                0x00a6_8ff0,
                &[],
                |o, frame| {
                    Some(hi_md::bridge::op_ffxambient_to_omdambient(
                        o,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
    }
}

#[test]
fn tex0_change_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_9ca0);
        let sizes: [u32; 8] = p.get(p.sym(0x00c8_8aa8));
        let packet = p.alloc(0x2000);
        for round in 0..3000 {
            p.fill(packet, 0x2000, &mut rng);
            let mut chain = Vec::new();
            for _ in 0..rng.below(7) {
                let ty = rng.pick(&[0x02u8, 0x03, 0x06, 0x07, 0x12, 0x00, 0x01, 0x45, 0xfe, 0x80]);
                chain.push((ty, i16::try_from(rng.below(5)).unwrap()));
            }
            chain.push((0xff, 0));
            tags(&mut p, packet, &chain, |ty| sizes[usize::from(ty & 7)]);
            // TEX0 bases near each other, so the range test picks some
            let mut t = packet + 0x20;
            for &(ty, n) in &chain {
                if rng.chance(70) {
                    let v: u32 = p.get(t + 8);
                    let base = 0x3000 + rng.below(0x1000);
                    p.set(t + 8, v & 0xffff_c000 | base);
                }
                t += 0x10 + u32::try_from(n).unwrap() * sizes[usize::from(ty & 7)];
            }
            let mut bases = Vec::new();
            let mut t = packet + 0x20;
            for &(ty, n) in &chain {
                bases.push(p.get::<u32>(t + 8) & 0x3fff);
                t += 0x10 + u32::try_from(n).unwrap() * sizes[usize::from(ty & 7)];
            }
            // a tag stepping back (count -1 of 0x30 bytes) onto the end
            if round % 4 == 3 {
                p.set(packet + 0x21, rng.pick(&[0x07u8, 0x47]));
                p.set(packet + 0x22, -1i16);
                p.set(packet + 1, 0xffu8);
            }
            let (lo, hi) = match round % 3 {
                0 => (0, 0x4000),
                1 if rng.chance(50) => (rng.pick(&bases), rng.pick(&bases)),
                1 => (0x3000 + rng.below(0x800), 0x3800 + rng.below(0x800)),
                _ => (rng.next() & 0xffff, rng.next()),
            };
            if round % 5 == 0 {
                p.check(
                    "op_md_tex0_change",
                    0x00a6_9c80,
                    &[packet, rng.next()],
                    |o| Some(hi_md::bridge::op_md_tex0_change_range(o, packet, 0, 0x4000)),
                );
            } else {
                let x = rng.next();
                p.check(
                    "op_md_tex0_change_range",
                    0x00a6_9ca0,
                    &[packet, x, lo, hi],
                    |o| {
                        #[allow(clippy::cast_possible_truncation)]
                        Some(hi_md::bridge::op_md_tex0_change_range(
                            o, packet, lo as u16, hi as u16,
                        ))
                    },
                );
            }
        }
    }
}

#[test]
fn md_vn_index_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_9f10);
        let (packet, out, table) = (p.alloc(0x1000), p.alloc(0x400), p.alloc(0x40 * 8));
        for _ in 0..3000 {
            p.fill(packet, 0x1000, &mut rng);
            p.fill(out, 0x400, &mut rng);
            p.fill(table, 0x200, &mut rng);
            let lists = rng.pick(&[0i16, 1, 3, 8, -2]);
            p.set(packet + 0x12, lists);
            let mut t = packet + 0x20;
            for _ in 0..rng.below(4) {
                let n = rng.below(4);
                p.set(t + 1, rng.pick(&[0x00u8, 0x10, 0x12, 0xfe]));
                p.set(t + 2, u16::try_from(n).unwrap());
                for k in 0..n {
                    let e = t + 0x20 + 0x1c * k;
                    for v in 0..3 {
                        let idx = rng.below(u32::try_from(lists.max(1)).unwrap());
                        p.set(e + 2 * v, u16::try_from(idx).unwrap());
                        p.set(e + 0x14 + 2 * v, rng.pick(&[0u16, 1, 2, 3, 0x8001]));
                    }
                }
                t += 0x20 + 0x1c * n;
            }
            p.set(t + 1, 0xffu8);
            p.check("op_md_vn_index", 0x00a6_9f10, &[packet, out, table], |o| {
                Some(hi_md::bridge::op_md_vn_index(o, packet, out, table))
            });
        }
    }
}
