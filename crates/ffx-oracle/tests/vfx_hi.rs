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
use ffx_vfx::{hi_0283, hi_draw, hi_light, hi_md, hi_opus, x87};

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
    prefixed_recorder(p, sym, &[], words, ret)
}

/// [`deref_recorder_with`] run after `prefix` (code that keeps the
/// stack as it found it).
fn prefixed_recorder(p: &mut Pair, sym: u32, prefix: &[u8], words: &[u8], ret: Option<u32>) -> u32 {
    let log = p.alloc(4 + 0x100 * 64);
    let mut c = prefix.to_vec();
    c.extend([0x8b, 0x0d]); // mov ecx, [count]
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

#[test]
fn md_list_bounds_match_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_a0c0);
        let (list, m, min, max) = (p.alloc(0x100), p.alloc(0x40), p.alloc(0x10), p.alloc(0x10));
        let (packets, mats) = (p.alloc(0x200 * 6), p.alloc(0x40 * 6));
        for round in 0..1500 {
            p.fill(list, 0x100, &mut rng);
            let n = rng.below(6);
            p.set(list + 4, u16::try_from(n).unwrap());
            for k in 0..6 {
                let (pk, mk) = (packets + 0x200 * k, mats + 0x40 * k);
                p.set(list + 0x10 + 0x10 * k, pk);
                p.set(list + 0x14 + 0x10 * k, mk);
                let verts = rng.below(6);
                p.set(pk + 0x12, u16::try_from(verts).unwrap());
                let off = 0x20 + 0x10 * rng.below(4);
                p.set(pk + 8, off);
                for i in 0..0x18 {
                    // vertices mostly finite, now and then a NaN or infinity
                    let x = if rng.chance(5) {
                        rng.float()
                    } else {
                        finite(&mut rng)
                    };
                    p.set(pk + off + 4 * i, x);
                }
                for i in 0..16 {
                    let x = finite(&mut rng);
                    p.set(mk + 4 * i, x);
                }
            }
            if round % 5 == 0 {
                // a negative vertex count draws nothing
                p.set(packets + 0x12, -3i16);
            }
            for i in 0..16 {
                let x = finite(&mut rng);
                p.set(m + 4 * i, x);
            }
            if round % 3 == 0 {
                // a zero matrix: every point +-0, so the box's `<` meets
                // equal zeros of either sign
                for i in 0..16 {
                    p.set(m + 4 * i, rng.pick(&[0u32, 0x8000_0000]));
                }
            }
            p.fill(min, 0x10, &mut rng);
            p.fill(max, 0x10, &mut rng);
            check_deep_n(
                &mut p,
                "op_md_list_bounds",
                0x00a6_a0c0,
                &[list, m, min, max],
                |o, frame| {
                    Some(hi_md::bridge::op_md_list_bounds(
                        o,
                        list,
                        m,
                        min,
                        max,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
    }
}

#[test]
fn opus_0582_dot_set_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_3740);
        let (b, rec) = (p.alloc(0x10 + 0x20 * 16), p.alloc(0x100));
        for round in 0..3000 {
            p.fill(b, 0x10 + 0x20 * 16, &mut rng);
            p.fill(rec, 0x100, &mut rng);
            p.set(b, rng.below(16) + 1);
            for at in [0x30u32, 0x34, 0x38, 0x3c, 0x40, 0x48, 0x4c, 0x94] {
                let x: u32 = match round % 4 {
                    // the game's range: angles, steps and lives in the hundreds
                    0 => (f32::from(i16::try_from(rng.below(0x2000)).unwrap()) - 4096.0).to_bits(),
                    1 => finite(&mut rng),
                    // beyond i32, where ftol2's and SSE2's truncations differ
                    2 => rng
                        .pick(&[3.0e9f32, -3.3e9, 1.2e12, -7.0e15, 2.2e19])
                        .to_bits(),
                    _ => rng.float(),
                };
                p.set(rec + at, x);
            }
            p.check("opus_0582_dot_set", 0x00a7_3740, &[b, rec], |o| {
                Some(hi_opus::bridge::opus_0582_dot_set(o, b, rec))
            });
        }
    }
}

/// `mov ecx, [esp + 4]; sub ecx, 1000; shl ecx, 2`: the first argument's
/// cell index for instance ids 1000..
const INSTANCE_CELL: [u8; 13] = [
    0x8b, 0x4c, 0x24, 0x04, 0x81, 0xe9, 0xe8, 0x03, 0, 0, 0xc1, 0xe1, 0x02,
];

#[test]
#[allow(clippy::too_many_lines)]
fn opus_0437_dot_draw_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_e2e0);
        // cells[i]: graphicVFXGetRenderDataByInstance(1000 + i, _); made[i]
        // what createVFXRenderDataBufferLine puts there
        let (cells, made, rets) = (p.alloc(0x10), p.alloc(0x10), p.alloc(0x10));
        let (spr_ret, seg_ret, valid_ret, req_ret) = (rets, rets + 4, rets + 8, rets + 12);
        let mut c = INSTANCE_CELL.to_vec();
        c.extend([0x8b, 0x81]); // mov eax, [ecx + cells]
        c.extend(cells.to_le_bytes());
        c.push(0xc3);
        jump_to(&mut p, 0x0064_53d0, &c);
        let mut c = INSTANCE_CELL.to_vec();
        c.extend([0x8b, 0x81]); // mov eax, [ecx + made]
        c.extend(made.to_le_bytes());
        c.extend([0x89, 0x81]); // mov [ecx + cells], eax
        c.extend(cells.to_le_bytes());
        let create = prefixed_recorder(&mut p, 0x0071_4b60, &c, &[0, 0, 0, 0], None);
        let mut logs = vec![create];
        for (sym, words, ret) in [
            (0x007e_6a00u32, &[0u8][..], Some(spr_ret)),
            (0x0062_f880, &[0, 0], None),
            (0x0062_f710, &[0, 0], None),
            (0x0064_5780, &[0, 0], None),
            (0x0063_fdb0, &[0, 0, 0, 0], Some(seg_ret)),
            (0x0064_5710, &[0, 0, 0], None),
            (0x0064_5600, &[0, 0], None),
            (0x0063_fd70, &[], Some(valid_ret)),
            (0x0072_8660, &[0], Some(req_ret)),
            (0x007e_6a60, &[0], None),
        ] {
            logs.push(deref_recorder_with(&mut p, sym, words, ret));
        }
        let (spr, req) = (p.alloc(0x1100), p.alloc(0x40));
        let rds = [p.alloc(0x100), p.alloc(0x100), p.alloc(0x100)];
        let geos = [p.alloc(0x10), p.alloc(0x10), p.alloc(0x10)];
        let b = p.alloc(0x40);
        let channel = p.sym(0x0233_28e0);
        for _ in 0..1500 {
            p.fill(spr, 0x1100, &mut rng);
            p.fill(req, 0x40, &mut rng);
            p.fill(b, 0x40, &mut rng);
            p.fill(channel, 4, &mut rng);
            p.set(spr_ret, spr);
            p.set(req_ret, req);
            for i in 0..3u32 {
                let k = i as usize;
                p.fill(rds[k], 0x100, &mut rng);
                p.fill(geos[k], 0x10, &mut rng);
                p.set(rds[k] + 0x94, geos[k]);
                let pick = |rng: &mut Xorshift| if rng.chance(15) { 0 } else { rds[k] };
                let (c, m) = (pick(&mut rng), pick(&mut rng));
                p.set(cells + 4 * i, c);
                p.set(made + 4 * i, m);
            }
            p.set(seg_ret, u32::from(!rng.chance(8)));
            p.set(valid_ret, u32::from(!rng.chance(20)));
            p.check("opus_0437_dot_draw", 0x00a6_e2e0, &[b], |o| {
                Some(hi_opus::bridge::opus_0437_dot_draw(
                    o,
                    b,
                    &mut call_original,
                ))
            });
        }
        for log in logs {
            assert!(p.get::<u32>(log) > 0, "{}: a stand-in never ran", p.name);
        }
    }
}

// ---- round 4: opus_0437 polygons, vertex lights ---------------------------

/// A stand-in at `sym` like [`deref_recorder_with`], with 0x200 bytes a
/// call (up to 127 logged words), returning the dword at `ret`.
fn wide_recorder(p: &mut Pair, sym: u32, words: &[u8], ret: u32) -> u32 {
    let log = p.alloc(4 + 0x100 * 0x200);
    let mut c = vec![0x8b, 0x0d]; // mov ecx, [count]
    c.extend(log.to_le_bytes());
    c.extend([0x81, 0xe1, 0xff, 0, 0, 0]); // and ecx, 0xff
    c.extend([0xc1, 0xe1, 0x09]); // shl ecx, 9
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
    assert!(pos <= 127);
    c.extend([0xff, 0x05]); // inc dword [count]
    c.extend(log.to_le_bytes());
    c.push(0xa1); // mov eax, [ret]
    c.extend(ret.to_le_bytes());
    c.push(0xc3);
    jump_to(p, sym, &c);
    log
}

/// A stand-in at `sym` returning the dword at `cell`.
fn returns_cell(p: &mut Pair, sym: u32, cell: u32) {
    let mut c = vec![0xa1]; // mov eax, [cell]
    c.extend(cell.to_le_bytes());
    c.push(0xc3);
    jump_to(p, sym, &c);
}

/// 16 floats at `at`: finite, now and then any (no signalling NaNs).
fn matrix(p: &mut Pair, rng: &mut Xorshift, at: u32) {
    for i in 0..16 {
        let x = if rng.chance(5) {
            rng.float()
        } else {
            finite(rng)
        };
        p.set(at + 4 * i, x);
    }
}

#[test]
fn opus_0437_poly_init_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_e1b0);
        let (list, packets, mats) = (p.alloc(0x100), p.alloc(0x400 * 4), p.alloc(0x40 * 4));
        let (b, m) = (p.alloc(0x800), p.alloc(0x40));
        for round in 0..1500 {
            p.fill(list, 0x100, &mut rng);
            p.fill(packets, 0x400 * 4, &mut rng);
            let n = rng.below(5);
            p.set(list + 4, u16::try_from(n).unwrap());
            for k in 0..n {
                let (pk, mk) = (packets + 0x400 * k, mats + 0x40 * k);
                p.set(list + 0x10 + 0x10 * k, pk);
                p.set(list + 0x14 + 0x10 * k, mk);
                let mut t = pk + 0x20;
                for _ in 0..rng.below(4) {
                    let c = rng.pick(&[-1i16, 0, 1, 2, 3, 3]);
                    p.set(t + 1, 0x10u8);
                    p.set(t + 2, c);
                    t = t.wrapping_add_signed(0x20 + 0x1c * i32::from(c));
                }
                p.set(t + 1, rng.pick(&[0x11u8, 0xff, 0x00]));
                p.set(pk + 8, 0x300u32);
                p.set(pk + 0x12, i16::try_from(rng.below(4)).unwrap());
                for i in 0..12 {
                    let x = finite(&mut rng);
                    p.set(pk + 0x300 + 4 * i, x);
                }
                for i in 0..16 {
                    let x = finite(&mut rng);
                    p.set(mk + 4 * i, x);
                }
            }
            for i in 0..16 {
                let x = finite(&mut rng);
                p.set(m + 4 * i, x);
            }
            p.fill(b, 0x800, &mut rng);
            let k = match round % 6 {
                0 => rng.pick(&[0u32, 0xffff_fffe, 0x8000_0000]),
                _ => rng.below(5),
            };
            // op_md_list_bounds leaves stack addresses in the EE registers
            // (0x00c8f548): run it at the depth the original calls it, its
            // frame 36 bytes below this one's
            let bounds = p.sym(0x00a6_a0c0);
            check_deep_n(
                &mut p,
                "opus_0437_poly_init",
                0x00a6_e1b0,
                &[b, list, m, k],
                |o, frame| {
                    let mut call = |o: &mut Oracle, f: u32, a: &[u32]| {
                        if f == bounds {
                            hi_md::bridge::op_md_list_bounds(
                                o,
                                a[0],
                                a[1],
                                a[2],
                                a[3],
                                frame - 36,
                                &mut call_original,
                            )
                        } else {
                            call_original(o, f, a)
                        }
                    };
                    Some(hi_opus::bridge::opus_0437_poly_init(
                        o, b, list, m, k, &mut call,
                    ))
                },
            );
        }
    }
}

/// The set-up shared by the two `op_vertex_normal_point44_ffx` tests:
/// `(model, work, chr cells)`, the scene lights and a character `chr`
/// (`ffx_chara` layout: name `+4`, render instance `+0x830`).
struct LightScene {
    model: u32,
    work: u32,
    area: u32,
    mats: [u32; 2],
    chr: u32,
    name: u32,
    ret: u32,
    log: u32,
}

impl LightScene {
    fn new(p: &mut Pair) -> Self {
        let ret = p.alloc(0x10);
        // graphicPS2LightingSetupEffect(instance, is_w, mat 16, tex, f, dirs
        // 16, colours 16, f, shade 3, scale 4, last)
        let log = wide_recorder(p, 0x0064_2750, &[0, 0, 16, 0, 0, 16, 16, 0, 3, 4, 0], ret);
        LightScene {
            model: p.alloc(0x60),
            work: p.alloc(0x400),
            area: p.alloc(0x200),
            mats: [p.alloc(0x40), p.alloc(0x40)],
            chr: p.alloc(0x900),
            name: p.alloc(0x10),
            ret,
            log,
        }
    }

    /// A fresh scene; returns the `chr` argument (0 a tenth of the time).
    fn fill(&self, p: &mut Pair, rng: &mut Xorshift) -> u32 {
        scene_lights(p, rng);
        p.fill(self.model, 0x60, rng);
        p.fill(self.work, 0x400, rng);
        p.fill(self.area, 0x200, rng);
        for at in [0x370u32, 0x374, 0x378, 0x37c] {
            let x = rng.float();
            p.set(self.work + at, x);
        }
        p.set(self.work + 0x390, self.area);
        for (k, at) in [0x28u32, 0x2c].into_iter().enumerate() {
            matrix(p, rng, self.mats[k]);
            p.set(self.model + at, self.mats[k]);
        }
        p.fill(self.chr, 0x900, rng);
        p.fill(self.name, 0x10, rng);
        p.set(self.name, rng.pick(&[b'w', b'W', b'v', 0]));
        p.set(self.chr + 4, self.name);
        // Ch_DrawCalcShade's own floats: any but signalling NaNs
        for i in 0..0x30 {
            let x = rng.float();
            p.set(self.chr + 0x330 + 4 * i, x);
        }
        let by_light = rng.pick(&[0u8, 1]);
        p.set(p.sym(0x00c4_96ea), by_light);
        p.fill(self.ret, 4, rng);
        if rng.chance(10) { 0 } else { self.chr }
    }

    fn check_log(&self, p: &Pair) {
        assert!(p.get::<u32>(self.log) > 0, "{}: no draw call", p.name);
    }
}

#[test]
fn vertex_normal_point44_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_d380);
        let s = LightScene::new(&mut p);
        let obj = p.alloc(0x10);
        for _ in 0..1000 {
            let chr = s.fill(&mut p, &mut rng);
            let (model, work) = (s.model, s.work);
            check_deep_n(
                &mut p,
                "op_vertex_normal_point44_ffx",
                0x00a6_d380,
                &[obj, model, work, chr],
                |o, frame| {
                    let aligned = (frame - 8) & !0xf;
                    Some(hi_light::bridge::op_vertex_normal_point44_ffx(
                        o,
                        model,
                        work,
                        chr,
                        aligned,
                        &mut call_original,
                    ))
                },
            );
        }
        s.check_log(&p);
    }
}

#[test]
fn vertex_normal_point44_2_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_d670);
        let s = LightScene::new(&mut p);
        let (obj, magic) = (p.alloc(0x10), p.alloc(0x10));
        returns_cell(&mut p, 0x009d_a3b0, magic);
        let mut special = 0;
        for _ in 0..1000 {
            let chr = s.fill(&mut p, &mut rng);
            let id = if rng.chance(30) {
                0x245
            } else {
                rng.pick(&[0x244u32, 0x246, 0x1b5, 0x10245])
            };
            special += u32::from(id == 0x245 && chr != 0);
            p.set(magic, id);
            let (model, work) = (s.model, s.work);
            check_deep_n(
                &mut p,
                "op_vertex_normal_point44_ffx_2",
                0x00a6_d670,
                &[obj, model, work, chr],
                |o, frame| {
                    let aligned = (frame - 8) & !0xf;
                    Some(hi_light::bridge::op_vertex_normal_point44_ffx_2(
                        o,
                        model,
                        work,
                        chr,
                        aligned,
                        &mut call_original,
                    ))
                },
            );
        }
        assert!(special > 0, "{}: no magic 0x245", p.name);
        s.check_log(&p);
    }
}

#[test]
fn vertex_normal_point44_0437_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_d890);
        let (packet, model, work) = (p.alloc(0x200), p.alloc(0x60), p.alloc(0x400));
        let (light, normals, bufs) = (p.alloc(0x40), p.alloc(0x400), p.alloc(0x400));
        let (vecs, ret) = (p.alloc(0x30), p.alloc(0x10));
        let malloc = deref_recorder_with(&mut p, 0x0068_6ff0, &[0, 0], Some(ret));
        let cells = p.sym(0x022f_b3e0);
        for round in 0..1500 {
            scene_lights(&mut p, &mut rng);
            p.fill(packet, 0x200, &mut rng);
            p.fill(model, 0x60, &mut rng);
            p.fill(work, 0x400, &mut rng);
            p.fill(bufs, 0x400, &mut rng);
            matrix(&mut p, &mut rng, light);
            p.set(model + 0x2c, light);
            p.set(model + 0x3c, normals + 0x200);
            for i in 0..0x100 {
                let x = if rng.chance(5) {
                    rng.float()
                } else {
                    finite(&mut rng)
                };
                p.set(normals + 4 * i, x);
            }
            for i in 0..12 {
                let x = if rng.chance(5) {
                    rng.float()
                } else {
                    finite(&mut rng)
                };
                p.set(vecs + 4 * i, x);
            }
            if round % 4 == 1 {
                // zeros of either sign and a few round values (zero light
                // rows, exact sums); no colour comes out -0 even so: both
                // matrices' row 3 is +0, a +0 term in every sum
                let some = [0u32, 0x8000_0000, 0x8000_0000, 0x3f80_0000, 0xbf00_0000];
                for i in 0..0x100 {
                    let x = rng.pick(&some);
                    p.set(normals + 4 * i, x);
                }
                for i in 0..12 {
                    let x = rng.pick(&some);
                    p.set(vecs + 4 * i, x);
                }
                let lights = p.sym(0x00c4_9724);
                for i in 0..0x1c {
                    let x = rng.pick(&some);
                    p.set(lights + 4 * i, x);
                }
                for i in 0..16 {
                    let x = rng.pick(&some);
                    p.set(light + 4 * i, x);
                }
            }
            // the stream: groups of a count and that many vertex numbers
            let groups = match round % 8 {
                0 => rng.pick(&[0i16, -1, -0x7fff]),
                _ => i16::try_from(rng.below(4)).unwrap() + 1,
            };
            p.set(packet + 0x12, groups);
            p.set(packet + 0x1c, 0x20u32);
            let mut s = packet + 0x30;
            for _ in 0..groups.max(0) {
                let n = rng.pick(&[0i16, -2, 1, 2, 3]);
                p.set(s, n);
                s += 2;
                for _ in 0..n.max(0) {
                    let v: u16 = match rng.below(4) {
                        0 => 0x3ff0 + u16::try_from(rng.below(0x20)).unwrap(),
                        1 => 0xfff0 + u16::try_from(rng.below(0x10)).unwrap(),
                        _ => u16::try_from(rng.below(0x10)).unwrap(),
                    };
                    p.set(s, v);
                    s += 2;
                }
            }
            let slot = rng.below(5);
            let cell = if rng.chance(20) { 0 } else { bufs + 0x200 };
            p.set(cells + 4 * slot, cell);
            p.set(ret, bufs + 0x200);
            let (base, mul1, mul0) = (vecs, vecs + 0x10, vecs + 0x20);
            check_deep_n(
                &mut p,
                "op_0437_vertex_normal_point44_ffx",
                0x00a6_d890,
                &[packet, model, work, base, mul1, mul0, slot],
                |o, frame| {
                    Some(hi_light::bridge::op_0437_vertex_normal_point44_ffx(
                        o,
                        packet,
                        model,
                        work,
                        [base, mul1, mul0],
                        slot,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
        assert!(p.get::<u32>(malloc) > 0, "{}: no allocation", p.name);
    }
}

// ---- round 5: model draw set-ups ------------------------------------------

/// The op scratch stack with its top at a random index and that mark at
/// `area`; returns the top index.
fn spr_top(p: &mut Pair, rng: &mut Xorshift, area: u32) -> i8 {
    let top = i8::try_from(rng.below(6)).unwrap();
    p.set(p.sym(0x0233_2ea1), top);
    let mark = p.sym(0x0119_fe10) + 4 * u32::try_from(top).unwrap();
    p.set(mark, area);
    let next = rng.next();
    p.set(mark + 4, next);
    // the GS is never busy here (the original would spin for ever)
    let sink = rng.next() & !0x100;
    p.set(p.sym(0x0230_56e8), sink);
    top
}

#[test]
fn md_chara_path3_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_a3a0);
        let s = LightScene::new(&mut p);
        let (obj, spr) = (p.alloc(0x40), p.alloc(0x1a00));
        let mut paths = [0u32; 3];
        for round in 0..1500 {
            let chr = s.fill(&mut p, &mut rng);
            let ambient = p.sym(0x00c4_9774);
            for k in 0..4 {
                let x = f32::from(u16::try_from(rng.below(300)).unwrap()) / 256.0 - 0.1;
                p.set(ambient + 4 * k, x.to_bits());
            }
            p.fill(obj, 0x40, &mut rng);
            p.fill(spr, 0x400, &mut rng);
            spr_top(&mut p, &mut rng, spr);
            let init = s.model;
            let flags = rng.next() & !0x60 | rng.pick(&[0u32, 0x20, 0x40, 0x60]);
            p.set(init, flags);
            p.set(init + 4, s.area);
            p.set(obj + 0x12, i16::try_from(rng.below(3)).unwrap());
            p.set(obj + 0x14, rng.pick(&[0i16, 1, -1, 0x100]));
            for at in [0x38u32, 0x3c] {
                if rng.chance(50) {
                    p.set(init + at, 0u32);
                }
            }
            if round % 5 == 0 {
                p.set(init + 0x4c, rng.pick(&[0u8, 0x80, 0xff, 1]));
            }
            let path = if flags & 0x40 != 0 {
                0
            } else if p.get::<i16>(obj + 0x14) != 0 && flags & 0x20 == 0 {
                1
            } else {
                2
            };
            paths[path] += 1;
            check_deep_n(
                &mut p,
                "op_md_chara_path3",
                0x00a6_a3a0,
                &[obj, init, chr],
                |o, _| {
                    Some(hi_draw::bridge::op_md_chara_path3(
                        o,
                        obj,
                        init,
                        chr,
                        &mut call_original,
                    ))
                },
            );
        }
        assert!(paths.iter().all(|&n| n > 0), "{}: paths {paths:?}", p.name);
    }
}

/// A tag chain from `at`: `n` tags of the types in `types` (each 0x10 or,
/// for 0x20 and up, 0x20 bytes, random fields, `i16 +2` 0 now and then),
/// then a tag of type `end`.
fn md_tags(p: &mut Pair, rng: &mut Xorshift, at: u32, types: &[i8], n: u32, end: i8) {
    let mut t = at;
    for _ in 0..n {
        let ty = rng.pick(types);
        p.set(t + 1, ty);
        if rng.chance(25) {
            p.set(t + 2, 0i16);
        }
        if rng.chance(25) {
            p.set(t + 0x10, 0u32);
        }
        t += if ty >= 0x20 { 0x20 } else { 0x10 };
    }
    p.set(t + 1, end);
}

#[test]
fn md_x64_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_b710);
        let (obj, init, spr, area) = (
            p.alloc(0x400),
            p.alloc(0x60),
            p.alloc(0x1a00),
            p.alloc(0x400),
        );
        let mats = [p.alloc(0x40), p.alloc(0x40)];
        let opc2 = deref_recorder(&mut p, 0x007e_6b20, &[]);
        // opc and 0x00a6cd40 are bare `ret`s: recorders tell them apart
        let opc = deref_recorder(&mut p, 0x00a6_d370, &[0, 0, 0]);
        let cd40 = deref_recorder(&mut p, 0x00a6_cd40, &[0, 0, 0]);
        let ends = [8i8, 0x1f, 0x21, 0x23, -1, -0x80, 0x7f];
        for round in 0..2000 {
            scene_lights(&mut p, &mut rng);
            p.fill(obj, 0x400, &mut rng);
            p.fill(init, 0x60, &mut rng);
            p.fill(spr, 0x400, &mut rng);
            spr_top(&mut p, &mut rng, spr);
            for (k, at) in [0x28u32, 0x2c].into_iter().enumerate() {
                matrix(&mut p, &mut rng, mats[k]);
                p.set(init + at, mats[k]);
            }
            p.set(init + 4, area + 0x100);
            let flags = rng.next() & !0x460 | rng.pick(&[0u32, 0x20, 0x40, 0x60, 0x400, 0x440]);
            p.set(init, flags);
            let own = rng.next() & !0x40c | rng.pick(&[0u32, 4, 8, 0x400, 0x408, 0x10]);
            p.set(obj, own);
            p.set(obj + 4, 0x40u32);
            p.set(obj + 0x12, i16::try_from(rng.below(6)).unwrap() - 2);
            p.set(obj + 0x14, rng.pick(&[0i16, 1, 3, -1]));
            for at in [0x38u32, 0x3c] {
                if rng.chance(50) {
                    p.set(init + at, 0u32);
                }
            }
            let types = [0i8, 1, 2, 3, 4, 5, 6, 7, 0x20, 0x22];
            let (n, end) = (rng.below(10), rng.pick(&ends));
            md_tags(&mut p, &mut rng, obj + 0x40, &types, n, end);
            if round % 7 == 0 {
                p.set(obj + 0x41, rng.pick(&ends));
            }
            check_deep_n(
                &mut p,
                "op_md_x64_00a6b710",
                0x00a6_b710,
                &[obj, init],
                |o, frame| {
                    Some(hi_draw::bridge::op_md_x64(
                        o,
                        obj,
                        init,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
        for (log, what) in [(opc2, "opc2"), (opc, "opc"), (cd40, "0x00a6cd40")] {
            assert!(p.get::<u32>(log) > 0, "{}: {what} never ran", p.name);
        }
    }
}

#[test]
fn md_x64_fog_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_bfe0);
        let (obj, init, spr) = (p.alloc(0x400), p.alloc(0x60), p.alloc(0x1a00));
        let ends = [1i8, 3, 5, 7, 0x20, -1, -0x80];
        for round in 0..2000 {
            p.fill(obj, 0x400, &mut rng);
            p.fill(init, 0x60, &mut rng);
            p.fill(spr, 0x400, &mut rng);
            for sym in [0x011a_0740u32, 0x011a_07f0] {
                let at = p.sym(sym);
                p.fill(at, 0x10, &mut rng);
            }
            spr_top(&mut p, &mut rng, spr);
            p.set(obj + 4, 0x40u32);
            for at in [0x38u32, 0x3c] {
                if rng.chance(50) {
                    p.set(init + at, 0u32);
                }
            }
            if rng.chance(40) {
                p.set(init + 8, 0u16);
            } else if rng.chance(20) {
                p.set(init + 8, 0x100u16);
            }
            let (n, end) = (rng.below(8), rng.pick(&ends));
            md_tags(&mut p, &mut rng, obj + 0x40, &[0, 2, 4, 6], n, end);
            if round % 7 == 0 {
                p.set(obj + 0x41, rng.pick(&ends));
            }
            p.check("op_md_x64_fog_00a6bfe0", 0x00a6_bfe0, &[obj, init], |o| {
                Some(hi_draw::bridge::op_md_x64_fog(o, obj, init))
            });
        }
    }
}

#[test]
fn refrection_st_float_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_cfb0);
        let (obj, model, light) = (p.alloc(0x400), p.alloc(0x40), p.alloc(0x40));
        let (pos, normals, out) = (p.alloc(0x200), p.alloc(0x1000), p.alloc(0x1000));
        let mut kinds = [0u32; 2];
        for round in 0..1500 {
            p.fill(obj, 0x400, &mut rng);
            p.fill(out, 0x1000, &mut rng);
            matrix(&mut p, &mut rng, light);
            for i in 0..0x80 {
                let x = if rng.chance(3) {
                    rng.float()
                } else {
                    finite(&mut rng)
                };
                p.set(pos + 4 * i, x);
            }
            for i in 0..0x400 {
                let x = if rng.chance(3) {
                    rng.float()
                } else {
                    finite(&mut rng)
                };
                p.set(normals + 4 * i, x);
            }
            if round % 4 == 1 {
                // small positions: the wrap's remainders of either sign
                for i in 0..0x80 {
                    let x = (f32::from(i16::try_from(rng.below(4000)).unwrap()) - 2000.0) / 3.0;
                    p.set(pos + 4 * i, x.to_bits());
                }
            }
            p.set(model + 0x2c, light);
            p.set(model + 0x38, pos);
            p.set(model + 0x3c, normals + 0x800);
            let groups = match round % 8 {
                0 => rng.pick(&[0i16, -1, -0x7fff]),
                _ => i16::try_from(rng.below(6)).unwrap() + 1,
            };
            p.set(obj + 0x12, groups);
            let stream = rng.chance(50);
            kinds[usize::from(stream)] += 1;
            p.set(obj + 0x1c, if stream { 0x20u32 } else { 0 });
            let mut s = obj + 0x30;
            for _ in 0..groups.max(0) {
                let n = rng.pick(&[0i16, -2, 1, 2, 3]);
                p.set(s, n);
                s += 2;
                for _ in 0..n.max(0) {
                    let v = i16::try_from(rng.below(0x50)).unwrap() - 0x28;
                    p.set(s, v);
                    s += 2;
                }
            }
            let scale = match rng.below(3) {
                0 => rng.next(),
                1 => rng.below(0x2000),
                _ => rng.pick(&[0u32, 1, 0x1000, 0xffff_f000, 0x8000_0000, 0x7fff_ffff]),
            };
            // never 0 or -1: the original faults (0x1000 / 0, and
            // ftol's 0x80000000 % -1 out of range)
            let d = |rng: &mut Xorshift| match rng.below(3) {
                0 => rng.below(64) + 1,
                1 => rng.pick(&[1u32, 0xffff_fffe, 0x1000, 0x1001, 0xffff_fff0, 7]),
                _ => (rng.next() | 1).min(0xffff_fffd),
            };
            let (du, dv) = (d(&mut rng), d(&mut rng));
            let junk = rng.next();
            check_deep_n(
                &mut p,
                "op_refrection_st_float",
                0x00a6_cfb0,
                &[obj, model, junk, out + 0x800, scale, du, dv],
                |o, frame| {
                    Some(hi_light::bridge::op_refrection_st_float(
                        o,
                        obj,
                        model,
                        out + 0x800,
                        scale,
                        du,
                        dv,
                        frame,
                        &mut call_original,
                    ))
                },
            );
        }
        assert!(kinds.iter().all(|&n| n > 0), "{}: {kinds:?}", p.name);
    }
}

#[test]
fn opp_map_draw_init_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_53f0);
        // the buffer (0x27e20 bytes) and the 8 bytes the original writes
        // past it
        let (b, pos) = (p.alloc(0x27e20 + 0x40), p.alloc(0x10));
        for round in 0..200 {
            if round % 10 == 0 {
                p.fill(b, 0x27e60, &mut rng);
            }
            for k in 0..2 {
                let x = match round % 4 {
                    0 => rng.float(),
                    1 => (f32::from(u16::try_from(rng.below(8000)).unwrap()) - 4000.0).to_bits(),
                    // 2^31..2^39: X - 256 and Y - 208 are no f32 there
                    2 => {
                        (rng.next() & 0x8000_0000)
                            | ((158 + rng.below(8)) << 23)
                            | (rng.next() & 0x007f_ffff)
                    }
                    _ => finite(&mut rng),
                };
                p.set(pos + 4 * k, x);
            }
            p.check("opp_map_draw_init_00a753f0", 0x00a7_53f0, &[b, pos], |o| {
                Some(hi_opus::bridge::opp_map_draw_init(
                    o,
                    b,
                    pos,
                    &mut call_original,
                ))
            });
        }
    }
}

// ---- round 6: model draw paths -------------------------------------------

#[test]
fn md_persp_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_bae0);
        let (obj, init, spr) = (p.alloc(0x400), p.alloc(0x60), p.alloc(0x1a00));
        let (view, src) = (p.alloc(0x40), p.alloc(0x40));
        let ends = [8i8, 9, 0x20, -1, -0x80, 0x7f];
        for round in 0..2000 {
            p.fill(obj, 0x400, &mut rng);
            p.fill(init, 0x60, &mut rng);
            p.fill(spr, 0x400, &mut rng);
            p.fill(src, 0x40, &mut rng);
            matrix(&mut p, &mut rng, view);
            for sym in [0x011a_0740u32, 0x011a_07f0] {
                let at = p.sym(sym);
                p.fill(at, 0x10, &mut rng);
            }
            spr_top(&mut p, &mut rng, spr);
            p.set(obj + 4, 0x40u32);
            p.set(obj + 0x12, i16::try_from(rng.below(6)).unwrap() - 2);
            p.set(init + 0x20, src);
            p.set(init + 0x30, view);
            for at in [0x38u32, 0x3c] {
                if rng.chance(50) {
                    p.set(init + at, 0u32);
                }
            }
            if rng.chance(40) {
                p.set(init + 8, 0u16);
            } else if rng.chance(20) {
                p.set(init + 8, 0x100u16);
            }
            let (n, end) = (rng.below(10), rng.pick(&ends));
            md_tags(
                &mut p,
                &mut rng,
                obj + 0x40,
                &[0, 1, 2, 3, 4, 5, 6, 7],
                n,
                end,
            );
            if round % 7 == 0 {
                p.set(obj + 0x41, rng.pick(&ends));
            }
            p.check("op_md_persp_00a6bae0", 0x00a6_bae0, &[obj, init], |o| {
                Some(hi_draw::bridge::op_md_persp(
                    o,
                    obj,
                    init,
                    &mut call_original,
                ))
            });
        }
    }
}

#[test]
fn md_flag80_fog_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a6_c3d0);
        let (obj, init, spr) = (p.alloc(0x400), p.alloc(0x60), p.alloc(0x2c00));
        // op_md_packet_size_check is always 0; a stand-in that logs its
        // arguments and returns the cell `skip` covers the other way out
        let skip = p.alloc(4);
        let log = deref_recorder_with(&mut p, 0x00a6_98b0, &[0, 0, 0], Some(skip));
        let ends = [1i8, 3, 5, 7, 8, 0x20, -1, -0x80, 0x7f];
        let mut skipped = 0;
        for round in 0..2000 {
            p.fill(obj, 0x400, &mut rng);
            p.fill(init, 0x60, &mut rng);
            p.fill(spr, 0x400, &mut rng);
            for sym in [0x011a_0740u32, 0x011a_0820] {
                let at = p.sym(sym);
                p.fill(at, 0x10, &mut rng);
            }
            spr_top(&mut p, &mut rng, spr);
            let s = if rng.chance(10) { rng.next() | 1 } else { 0 };
            p.set(skip, s);
            skipped += u32::from(s != 0);
            p.set(obj + 4, 0x40u32);
            p.set(obj + 0x12, i16::try_from(rng.below(6)).unwrap() - 2);
            for at in [0x38u32, 0x3c] {
                if rng.chance(50) {
                    p.set(init + at, 0u32);
                }
            }
            if rng.chance(40) {
                p.set(init + 8, 0u16);
            } else if rng.chance(20) {
                p.set(init + 8, 0x100u16);
            }
            let (n, end) = (rng.below(8), rng.pick(&ends));
            md_tags(&mut p, &mut rng, obj + 0x40, &[0, 2, 4, 6], n, end);
            if round % 7 == 0 {
                p.set(obj + 0x41, rng.pick(&ends));
            }
            p.check(
                "op_md_flag80_fog_00a6c3d0",
                0x00a6_c3d0,
                &[obj, init],
                |o| {
                    Some(hi_draw::bridge::op_md_flag80_fog(
                        o,
                        obj,
                        init,
                        &mut call_original,
                    ))
                },
            );
        }
        assert!(skipped > 0 && p.get::<u32>(log) > 0, "{}", p.name);
    }
}

/// A recorder whose third argument (a render-data part) is logged as the
/// part's `+0x1c`, the only word the renderer reads from it.
fn part_recorder(p: &mut Pair, sym: u32, words: &[u8]) -> u32 {
    let prefix = [
        0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp + 0xc]
        0x8b, 0x40, 0x1c, // mov eax, [eax + 0x1c]
        0x89, 0x44, 0x24, 0x0c, // mov [esp + 0xc], eax
    ];
    prefixed_recorder(p, sym, &prefix, words, None)
}

/// A part radius: realistic sizes, now and then anything.
fn radius(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.float(),
        1 => finite(rng),
        _ => (f32::from(u16::try_from(rng.below(4000)).unwrap()) - 200.0).to_bits(),
    }
}

#[test]
fn opus_0283_draw_matches_the_original() {
    for mut p in Pair::all() {
        let mut rng = Xorshift(0x00a7_6770);
        let (obj, rec, b) = (p.alloc(0x400), p.alloc(0x100), p.alloc(0xd100));
        let (id, has, parent, pm) = (p.alloc(4), p.alloc(4), p.alloc(4), p.alloc(0x40));
        returns_cell(&mut p, 0x009d_a3b0, id);
        // logs (lo, hi) and returns the cell `has`
        let lookup = deref_recorder_with(&mut p, 0x0064_53d0, &[0, 0], Some(has));
        returns_cell(&mut p, 0x0080_1f00, parent);
        p.set(parent, pm);
        let update = part_recorder(&mut p, 0x0071_8610, &[0, 0, 0, 0]);
        let init = part_recorder(&mut p, 0x0071_1180, &[0, 0, 0, 0, 0]);
        let vfx_update = deref_recorder(&mut p, 0x0064_58a0, &[0, 0]);
        let skip = p.sym(0x012f_b7d0);
        let mut skipped = 0;
        for round in 0..60 {
            if round % 10 == 0 {
                p.fill(b, 0xd100, &mut rng);
            }
            p.fill(obj, 0x400, &mut rng);
            p.fill(rec, 0x100, &mut rng);
            p.set(obj + 0x328, b);
            matrix(&mut p, &mut rng, pm);
            for at in [0x90u32, 0x94, 0x98] {
                let x = finite(&mut rng);
                p.set(rec + at, x);
            }
            p.set(rec + 0x30, radius(&mut rng));
            let bulge = match rng.below(3) {
                0 => rng.float(),
                1 => finite(&mut rng),
                _ => (f32::from(u16::try_from(rng.below(512)).unwrap()) - 128.0).to_bits(),
            };
            p.set(rec + 0x34, bulge);
            p.set(rec + 0x3c, radius(&mut rng));
            let id_v = rng.next();
            p.set(id, id_v);
            let has_v = if rng.chance(50) { 0 } else { rng.next() | 1 };
            p.set(has, has_v);
            let s = u32::from(rng.chance(10));
            p.set(skip, s);
            skipped += s;
            check_deep_n(
                &mut p,
                "opus_0283_draw",
                0x00a7_6770,
                &[obj, rec],
                |o, frame| {
                    hi_0283::bridge::opus_0283_draw(o, obj, rec, frame, &mut call_original);
                    None
                },
            );
        }
        assert!(skipped > 0, "{}: never skipped", p.name);
        for (log, what) in [
            (lookup, "graphicVFXGetRenderDataByInstance"),
            (update, "update"),
            (init, "init"),
            (vfx_update, "graphicVFXUpdate"),
        ] {
            assert!(p.get::<u32>(log) > 0, "{}: {what} never ran", p.name);
        }
    }
}
