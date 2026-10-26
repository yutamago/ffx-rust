//! M4 coverage 2's new functions (list `c2-new`: `ffx_prender::c2_shadow`,
//! `ffx_prender::c2_post`, `ffx_glue_other::c2_iggy`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that only compute (the matrix and bounds helpers, the
//! rectangle union) run as they are in both oracles; the others are
//! replaced in both alike by recording stubs: each call appends its tag,
//! `ecx` and its arguments to a log in memory, so the diff checks that
//! both made the same calls in the same order. Ports that pass their own
//! frame to such a stub get a frame at the original's `ebp`, and the
//! frames are compared after the call.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_glue_other::c2_iggy::{self, bridge as gi};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::c2_post::{self as post, bridge as gp};
use ffx_prender::c2_shadow::{self as shadow, bridge as gs};
use ffx_prender::c2_smaa::{self as smaa, bridge as gm};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1760;
/// After the stubs: `mov eax, POISON; ret`, which [`G`] calls before each
/// original so that a callee leaving `eax` alone shows the poison.
const POISON_AT: u32 = CAVE + CAVE_LEN;
const POISON: u32 = 0xdead_beef;

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
    /// A word: small values, 0, 1, -1 and anything.
    fn word(&mut self) -> u32 {
        match self.below(5) {
            0 => 0,
            1 => 1,
            2 => self.below(8),
            3 => 0xffff_ffff,
            _ => self.next(),
        }
    }
    /// An integer in `lo..hi`.
    fn range(&mut self, lo: i32, hi: i32) -> u32 {
        (lo + self.below((hi - lo) as u32) as i32) as u32
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
            if !x87_snan(bits) {
                return f32::from_bits(bits);
            }
        }
    }
    /// A float in `lo..hi`, now and then a special one.
    fn float_in(&mut self, lo: f32, hi: f32) -> f32 {
        if self.chance(40) {
            return self.float();
        }
        lo + (hi - lo) * (self.below(1 << 20) as f32 / (1 << 20) as f32)
    }
}

fn x87_snan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 && bits & 0x0040_0000 == 0
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle, u32);

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
        self.0.call(self.1, Conv::Cdecl, &[]).unwrap();
        self.0.call(addr, conv(c), args).unwrap().eax
    }
    fn scratch(&mut self) -> u32 {
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
    /// `al`.
    U8(u8),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `this`, up
    /// to 14 arguments).
    log: u32,
    cave: u32,
    cave_next: u32,
    snapshot: bool,
    /// Bytes the stubs replaced, put back by [`Twins::reset`].
    patched: Vec<(u32, Vec<u8>)>,
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle, snapshot: bool) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            snapshot,
            patched: Vec::new(),
        };
        t.log = t.zeros(0x1_0000);
        let at = t.rebase(POISON_AT);
        let mut code = vec![0xb8];
        code.extend(POISON.to_le_bytes());
        code.push(0xc3);
        t.write(at, &code);
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

    /// Both kinds: the image, then the snapshot when one is set.
    fn each(mut f: impl FnMut(&mut Self)) {
        if let Some(mut t) = Self::image() {
            f(&mut t);
        }
        if let Some(mut t) = Self::snapshot() {
            f(&mut t);
        }
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

    fn words(&mut self, w: &[u32]) -> u32 {
        let bytes: Vec<u8> = w.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.alloc(&bytes)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn wf(&mut self, at: u32, v: f32) {
        self.w32(at, v.to_bits());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Writes `bytes` at `at`, to be put back by [`Twins::reset`].
    fn patch(&mut self, at: u32, bytes: &[u8]) {
        let mut old = vec![0; bytes.len()];
        Mem::read_bytes(&self.b, at, &mut old);
        self.patched.push((at, old));
        self.write(at, bytes);
    }

    fn reset(&mut self) {
        for (at, old) in std::mem::take(&mut self.patched).into_iter().rev() {
            self.write(at, &old);
        }
        self.cave_next = self.cave;
        let log = self.log;
        self.w32(log, 0);
    }

    /// Writes a recording stub over the original at `sym`: logs `tag`
    /// (the address), `ecx` when `this`, and `n` stack arguments; pops
    /// `pops` words; returns `ret`.
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {
        assert!(n <= 14);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
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
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// A recording stub for `ShaderParams_FindByName(this; name)` that
    /// finds the names whose address's low byte has its bit set in the
    /// 256-bit map at `select`: returns the name's address, else 0.
    fn stub_select(&mut self, sym: u32, select: u32) {
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], tag
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x50, 0x08]); // mov [eax+8], edx
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
        c.extend([0x0f, 0xb6, 0xd0]); // movzx edx, al
        c.extend([0x0f, 0xa3, 0x15]);
        c.extend(select.to_le_bytes()); // bt [select], edx
        c.extend([0x1b, 0xd2]); // sbb edx, edx
        c.extend([0x23, 0xc2]); // and eax, edx
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.patch(at, &j);
    }

    /// A thiscall stub popping its `n` words.
    fn method(&mut self, sym: u32, n: u32, ret: u32) {
        self.stub(sym, true, n, n, ret);
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }

    /// The original's `ebp` for a call with `args` stack words: its
    /// arguments, the return address and the saved `ebp` below where the
    /// call pushes.
    fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// Zeroes `len` bytes below `ebp` in both oracles (frame words the
    /// original reads before writing; the port takes 0 there).
    fn clear_frame(&mut self, ebp: u32, len: u32) {
        self.write(ebp - len, &vec![0; len as usize]);
    }

    /// A frame for a port whose callees run as they are: well below where
    /// `b`'s calls push.
    fn port_frame(&self) -> u32 {
        self.b.stack_range().start + 0x1000
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
        let poison = self.rebase(POISON_AT);
        let out = bridge(&mut G(&mut self.b, poison));
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

    /// [`Twins::check`] with the port's frame at the original's `ebp`
    /// (its callees log pointers into it): `low` bytes below `ebp` are
    /// compared after the call, but for the top `skip` (the original's
    /// exception registration).
    #[allow(clippy::too_many_arguments)]
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        skip: u32,
        bridge: impl FnOnce(&mut G, u32) -> Out,
    ) {
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        self.clear_frame(ebp, low);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |m| bridge(m, ebp));
        self.b.set_call_top(None);
        let n = (low - skip) as usize;
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        assert_eq!(x, y, "{what}: frame at {lo:#x}");
    }
}

// ---- the caster rectangles' arrays ----

#[test]
fn copy_records_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1357_9bdf);
        for i in 0..64 {
            t.reset();
            let src: Vec<u32> = (0..32).map(|_| rng.next()).collect();
            let src = t.words(&src);
            let dst = t.zeros(0x100);
            let n = rng.below(8);
            // a null destination skips one record (the next one would be
            // written at 0x10)
            let (dst, n) = if rng.chance(5) {
                (0, n.min(1))
            } else {
                (dst, n)
            };
            let what = format!("copy_records #{i}");
            t.check(&what, 0x006c_7190, Conv::Cdecl, &[dst, src, n], |m| {
                gs::copy_records(m, dst, src, n);
                Out::None
            });
        }
    });
}

/// A `PArray` of 16-byte records with `n` elements (flag bit 31 now and
/// then) and a record to append.
fn record_array(t: &mut Twins, rng: &mut Rng) -> (u32, u32) {
    let n = rng.below(6);
    let data: Vec<u32> = (0..4 * n).map(|_| rng.next()).collect();
    let data = if n == 0 && rng.chance(2) {
        0
    } else {
        t.words(&data)
    };
    let flag = if rng.chance(3) { 0x8000_0000 } else { 0 };
    let this = t.words(&[n | flag, data]);
    let record: Vec<u32> = (0..4).map(|_| rng.next()).collect();
    let record = t.words(&record);
    (this, record)
}

#[test]
fn append_record_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2468_ace0);
        for i in 0..64 {
            t.reset();
            t.stub(shadow::FREE, false, 1, 0, rng.word());
            let (this, record) = record_array(t, &mut rng);
            let storage = match rng.below(6) {
                0 => 0,
                1 => Mem::read(&t.b, this + 4),
                _ => t.zeros(0x80),
            };
            let what = format!("append_record #{i}");
            t.check(
                &what,
                0x006d_2760,
                Conv::Thiscall(this),
                &[storage, record],
                |m| Out::U32(gs::append_record(m, this, storage, record)),
            );
        }
    });
}

#[test]
fn push_record_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_cafe);
        for i in 0..64 {
            t.reset();
            t.stub(shadow::FREE, false, 1, 0, rng.word());
            let block = if rng.chance(6) { 0 } else { t.zeros(0x80) };
            t.stub(shadow::MALLOC, false, 2, 0, block);
            let (this, record) = record_array(t, &mut rng);
            let what = format!("push_record #{i}");
            t.check(&what, 0x006d_17d0, Conv::Thiscall(this), &[record], |m| {
                Out::U32(gs::push_record(m, this, record))
            });
        }
    });
}

#[test]
fn rect_union_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f0f_1234);
        for i in 0..400 {
            t.reset();
            let v = |rng: &mut Rng| {
                if rng.chance(20) {
                    rng.next()
                } else {
                    rng.range(-40, 80)
                }
            };
            let a: [u32; 4] = std::array::from_fn(|_| v(&mut rng));
            let b: [u32; 4] = std::array::from_fn(|_| v(&mut rng));
            let block = t.zeros(0x40);
            // the outputs apart, or now and then sharing a word
            let mut out: [u32; 4] = std::array::from_fn(|k| block + 4 * k as u32);
            if rng.chance(5) {
                let (x, y) = (rng.below(4) as usize, rng.below(4) as usize);
                out[x] = out[y];
            }
            let mut args = a.to_vec();
            args.extend(b);
            args.extend(out);
            let what = format!("rect_union #{i}");
            let this = rng.next();
            t.check(&what, 0x006d_2b50, Conv::Thiscall(this), &args, |m| {
                Out::U32(gs::rect_union(m, a, b, out))
            });
        }
    });
}

/// A synthetic shadow manager (0x4068 bytes, about half of it 0).
fn manager(t: &mut Twins, rng: &mut Rng) -> u32 {
    let bytes: Vec<u8> = (0..0x4068)
        .map(|_| if rng.chance(2) { 0 } else { rng.next() as u8 })
        .collect();
    t.alloc(&bytes)
}

/// Cascade `i`'s rectangles: `n` of them, coordinates near the map.
fn rects(t: &mut Twins, rng: &mut Rng, this: u32, i: u32, n: u32) {
    t.w32(shadow::count_at(this, i), n);
    for k in 0..n.min(64) {
        let e = shadow::rect(this, i, k);
        for w in 0..4 {
            let v = if rng.chance(30) {
                rng.next()
            } else {
                rng.range(-80, 1200)
            };
            t.w32(e + 4 * w, v);
        }
    }
}

#[test]
fn align_rects_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7777_1111);
        for i in 0..32 {
            t.reset();
            let this = manager(t, &mut rng);
            let c = rng.below(2);
            let n = match rng.below(6) {
                0 => 0,
                1 => 0xffff_fff0,
                _ => rng.below(64),
            };
            rects(t, &mut rng, this, c, n);
            let what = format!("align_rects #{i}");
            t.check(&what, 0x006d_1800, Conv::Thiscall(this), &[c], |m| {
                gs::align_rects(m, this, c);
                Out::None
            });
        }
    });
}

#[test]
fn sync_cascades_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5151_7272);
        for i in 0..64 {
            t.reset();
            let this = manager(t, &mut rng);
            for pair in 0..2u32 {
                let a = this + 0x290 + 8 * pair;
                let b = a + 0x10;
                let m = rng.below(5);
                let ids: Vec<u32> = (0..m).map(|_| rng.below(6)).collect();
                // the second list: the same ids shuffled, or others
                let n = if rng.chance(4) { rng.below(5) } else { m };
                let other: Vec<u32> = (0..n)
                    .map(|k| {
                        if !ids.is_empty() && !rng.chance(6) {
                            ids[(k as usize * 7 + 3) % ids.len()]
                        } else {
                            rng.below(6)
                        }
                    })
                    .collect();
                let ida = t.words(&ids);
                let idb = t.words(&other);
                let fa = if rng.chance(4) { 0x8000_0000 } else { 0 };
                let fb = if rng.chance(4) { 0x8000_0000 } else { 0 };
                t.w32(a, m | fa);
                t.w32(a + 4, ida);
                t.w32(b, n | fb);
                t.w32(b + 4, idb);
            }
            for w in 0..128u32 {
                let (u, v) = (rng.float(), rng.float());
                t.wf(this + 0x90 + 4 * w, u);
                t.wf(this + 0x2b0 + 4 * w, v);
            }
            let what = format!("sync_cascades #{i}");
            t.check(&what, 0x006c_9780, Conv::Thiscall(this), &[], |m| {
                gs::sync_cascades(m, this);
                Out::None
            });
        }
    });
}

/// A 4x4 near a scaled identity (`scale` on the diagonal), with small
/// random terms; a random one now and then.
fn matrix(t: &mut Twins, rng: &mut Rng, scale: f32) -> Vec<f32> {
    let _ = t;
    (0..16)
        .map(|k| {
            if rng.chance(60) {
                rng.float()
            } else if k % 5 == 0 {
                if k == 15 { 1.0 } else { scale }
            } else if k >= 12 {
                rng.float_in(-0.2, 0.2)
            } else {
                rng.float_in(-0.05, 0.05)
            }
        })
        .collect()
}

fn floats(t: &mut Twins, v: &[f32]) -> u32 {
    let w: Vec<u32> = v.iter().map(|f| f.to_bits()).collect();
    t.words(&w)
}

#[test]
fn view_record_init_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x3c3c_5a5a);
        for i in 0..32 {
            t.reset();
            let fill: Vec<u8> = (0..0x200).map(|_| rng.next() as u8).collect();
            let this = t.alloc(&fill);
            let m: Vec<f32> = (0..12).map(|_| rng.float_in(-4.0, 4.0)).collect();
            let m = floats(t, &m);
            let mat = matrix(t, &mut rng, 1.0);
            let mat = floats(t, &mat);
            let (a, b) = (rng.float(), rng.float());
            let what = format!("view_record_init #{i}");
            let ebp = t.port_frame() + 0x100;
            t.check(
                &what,
                0x006c_7cc0,
                Conv::Thiscall(this),
                &[m, mat, a.to_bits(), b.to_bits()],
                |g| Out::U32(gs::view_record_init(g, ebp, this, m, mat, a, b)),
            );
        }
    });
}

/// One caster node: its object (`+0x1c`) with a box (`+0`, `+0x10`) in
/// `lo..hi` and its 3x4 matrix (`+0xc`).
fn node_in(t: &mut Twins, rng: &mut Rng, lo: f32, hi: f32) -> u32 {
    // below 0: boxes on a 1/16 grid with an identity matrix half the time,
    // so that texel values meet the map's bounds exactly
    let grid = lo < 0.0 && rng.chance(2);
    let m: Vec<f32> = (0..12)
        .map(|k| {
            if grid {
                if k % 4 == 0 { 1.0 } else { 0.0 }
            } else if k % 5 == 0 {
                1.0
            } else if k % 4 == 3 {
                rng.float_in(-0.3, 0.3)
            } else {
                rng.float_in(-0.05, 0.05)
            }
        })
        .collect();
    let m = floats(t, &m);
    let snap = |v: f32| if grid { (v * 16.0).round() / 16.0 } else { v };
    let v = |rng: &mut Rng, lo: f32, hi: f32| snap(rng.float_in(lo, hi));
    let x0 = v(rng, lo, hi);
    let y0 = v(rng, lo, hi);
    let z0 = v(rng, lo, hi);
    let ends = [v(rng, 0.0, 0.3), v(rng, 0.0, 0.3), v(rng, 0.0, 0.3)];
    let object = floats(
        t,
        &[
            x0,
            y0,
            z0,
            0.0,
            x0 + ends[0],
            y0 + ends[1],
            z0 + ends[2],
            0.0,
        ],
    );
    t.w32(object + 0xc, m);
    let node = t.zeros(0x40);
    t.w32(node + 0x1c, object);
    node
}

/// A manager whose cells hold caster lists of 1-3 nodes (one cell in
/// `sparse` on average, else about two in three), and cascade matrices
/// near the identity; boxes in `lo..hi`.
fn casters(t: &mut Twins, rng: &mut Rng, sparse: u32, lo: f32, hi: f32) -> u32 {
    let this = manager(t, rng);
    for cell in 0..64u32 {
        let empty = if sparse == 0 {
            rng.chance(3)
        } else {
            !rng.chance(sparse)
        };
        let list = if empty {
            0
        } else {
            let n = 1 + rng.below(3);
            let nodes: Vec<u32> = (0..n).map(|_| node_in(t, rng, lo, hi)).collect();
            let nodes = t.words(&nodes);
            let flag = if rng.chance(4) { 0x8000_0000 } else { 0 };
            let n = if rng.chance(10) { 0 } else { n };
            t.words(&[n | flag, nodes])
        };
        t.w32(this + 0xde8 + 4 * cell, list);
        let on = |rng: &mut Rng| if rng.chance(4) { rng.below(3) } else { 1 };
        let (u, v) = (on(rng), on(rng));
        t.w32(this + 0xfe8 + 4 * cell, u);
        t.w32(this + 0x10e8 + 4 * cell, v);
        for c in 0..2u32 {
            let b = u8::from(!rng.chance(3));
            t.w8(this + 0x2b28 + 0x40 * c + cell, b);
        }
    }
    for c in 0..2u32 {
        let m = matrix(t, rng, 1.0);
        for (k, v) in m.iter().enumerate() {
            t.wf(this + 0x210 + 0x40 * c + 4 * k as u32, *v);
        }
    }
    this
}

#[test]
fn build_rects_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1234_8765);
        let n = if t.snapshot { 12 } else { 40 };
        for i in 0..n {
            t.reset();
            let this = casters(t, &mut rng, 0, 0.0, 0.9);
            let c = rng.below(2);
            let scale = |rng: &mut Rng| match rng.below(8) {
                0 => rng.float(),
                1 => 512.0,
                _ => 1024.0,
            };
            let (sx, sy) = (scale(&mut rng), scale(&mut rng));
            let what = format!("build_rects #{i}");
            let ebp = t.port_frame() + 0x200;
            t.check(
                &what,
                0x006c_88a0,
                Conv::Thiscall(this),
                &[c, sx.to_bits(), sy.to_bits()],
                |g| {
                    gs::build_rects(g, ebp, this, c, sx.to_bits(), sy.to_bits());
                    Out::None
                },
            );
        }
    });
}

#[test]
fn caster_crosses_edge_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x4242_1717);
        let n = if t.snapshot { 200 } else { 2000 };
        let mut seen = [0u32; 2];
        for i in 0..n {
            t.reset();
            // a few casters, their boxes across the map's edges now and then;
            // small maps so that the texel comparisons meet their bounds
            let this = casters(t, &mut rng, 20, -0.3, 1.3);
            let mut view: Vec<f32> = (0..16)
                .map(|k| if k % 5 == 0 { 1.0 } else { 0.0 })
                .collect();
            if rng.chance(4) {
                view = matrix(t, &mut rng, 1.0);
            }
            let mut args: Vec<u32> = view.iter().map(|f| f.to_bits()).collect();
            args.push(if rng.chance(3) { 0 } else { rng.below(256) });
            let size = |rng: &mut Rng| match rng.below(10) {
                0 => rng.float(),
                1 => 1024.0,
                2..=5 => 16.0,
                _ => 24.0,
            };
            let (sx, sy) = (size(&mut rng), size(&mut rng));
            let ratio = if rng.chance(10) {
                rng.float()
            } else {
                rng.below(30) as f32 / 16.0
            };
            args.extend([ratio, sx, sy].map(f32::to_bits));
            let copy = t.words(&args);
            let what = format!("caster_crosses_edge #{i}");
            let ebp = t.port_frame() + 0x200;
            let mut got = 0;
            t.check(&what, 0x006c_8f50, Conv::Thiscall(this), &args, |g| {
                got = gs::caster_crosses_edge(g, ebp, this, copy);
                Out::U8(got)
            });
            seen[usize::from(got)] += 1;
        }
        assert!(seen[0] > n / 8 && seen[1] > n / 8, "outcomes {seen:?}");
    });
}

/// The gradient meshes' set-up: a manager with `n` rectangles in cascade
/// `i`, the mesh and extra arrays; stubs for the spare mesh and the
/// set-up. Returns (this, i, meshes, extras, scale, depth).
fn gradient_case(t: &mut Twins, rng: &mut Rng, setup: u32) -> (u32, u32, u32, u32, f32, f32) {
    let this = manager(t, rng);
    let i = rng.below(2);
    let n = rng.below(9);
    let depth = match rng.below(12) {
        0 => rng.float(),
        1 => -512.0,
        2..=5 => 256.0,
        6..=8 => 1000.0,
        _ => 512.5,
    };
    let scale = match rng.below(12) {
        0 => rng.float(),
        1 => -0.5,
        2 => 0.0,
        3..=5 => 0.5,
        6..=8 => 0.25,
        _ => 1.0,
    };
    // y0 and y1 at the edge (scale * depth), at the depth, 0, either
    // side of them, or anywhere
    let edge = scale * depth;
    let pick = |rng: &mut Rng| -> u32 {
        let at = |x: f32| {
            if x.is_finite() && x.abs() < 1e9 {
                x as i32
            } else {
                0
            }
        };
        match rng.below(10) {
            0 => at(edge) as u32,
            1 => at(edge).wrapping_add(rng.range(-1, 2) as i32) as u32,
            2 => at(depth) as u32,
            3 => at(depth).wrapping_add(rng.range(-1, 2) as i32) as u32,
            4 => rng.range(-2, 2),
            5 => rng.range(-200, 0),
            _ => rng.range(-100, 1200),
        }
    };
    t.w32(shadow::count_at(this, i), n);
    for k in 0..n {
        let e = shadow::rect(this, i, k);
        for w in 0..4 {
            let v = pick(rng);
            t.w32(e + 4 * w, v);
        }
    }
    let default = t.zeros(0x20);
    t.w32(this + 0x8b8, default);
    if rng.chance(2) {
        t.w32(this + 0xde4, 0);
    }
    let spare = t.zeros(0x20);
    let first = rng.next();
    t.w32(spare, first);
    t.method(shadow::GET_SPARE_GRAD_MESH, 1, spare);
    t.method(setup, 4, rng.word());
    let meshes: Vec<u32> = (0..n)
        .map(|_| match rng.below(4) {
            0 => 0,
            1 => default,
            _ => {
                let m = t.zeros(0x20);
                let w = rng.next();
                t.w32(m, w);
                m
            }
        })
        .collect();
    let meshes = t.words(&meshes);
    let extras: Vec<u32> = (0..n).map(|_| rng.next()).collect();
    let extras = t.words(&extras);
    (this, i, meshes, extras, scale, depth)
}

#[test]
fn grad_meshes_a_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x6060_9090);
        for k in 0..400 {
            t.reset();
            let (this, i, meshes, extras, scale, depth) =
                gradient_case(t, &mut rng, shadow::GRAD_SETUP_A);
            let unused = rng.next();
            let args = [i, meshes, extras, scale.to_bits(), unused, depth.to_bits()];
            let what = format!("grad_meshes_a #{k}");
            t.check(&what, 0x006d_12f0, Conv::Thiscall(this), &args, |g| {
                gs::grad_meshes_a(
                    g,
                    this,
                    i,
                    meshes,
                    extras,
                    scale.to_bits(),
                    unused,
                    depth.to_bits(),
                );
                Out::None
            });
        }
    });
}

#[test]
fn grad_meshes_b_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x7070_a0a0);
        for k in 0..400 {
            t.reset();
            let (this, i, meshes, extras, scale, depth) =
                gradient_case(t, &mut rng, shadow::GRAD_SETUP_B);
            let unused = rng.next();
            let args = [i, meshes, extras, scale.to_bits(), unused, depth.to_bits()];
            // the never-written frame word the port takes as 0
            let ebp = t.frame_ebp(args.len());
            t.clear_frame(ebp, 0x20);
            let what = format!("grad_meshes_b #{k}");
            t.check(&what, 0x006d_1530, Conv::Thiscall(this), &args, |g| {
                gs::grad_meshes_b(
                    g,
                    this,
                    i,
                    meshes,
                    extras,
                    scale.to_bits(),
                    unused,
                    depth.to_bits(),
                );
                Out::None
            });
        }
    });
}

// ---- post anti-aliasing, the D3D11 resolve, the menu callback ----

#[test]
fn set_value_checked_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0098_8900);
        for i in 0..128 {
            t.reset();
            let size = rng.range(0, 0x40);
            let buffer = t.zeros(0x80);
            t.w32(buffer, size);
            let at = rng.below(0x30) as u16;
            let len = (rng.below(0x24) as u16) | if rng.chance(4) { 0xe000 } else { 0 };
            let desc = t.zeros(0x10);
            t.write(desc + 8, &at.to_le_bytes());
            t.write(desc + 0xa, &len.to_le_bytes());
            let src: Vec<u32> = (0..12).map(|_| rng.next()).collect();
            let src = if rng.chance(6) { 0 } else { t.words(&src) };
            let what = format!("set_value_checked #{i}");
            t.check(
                &what,
                0x0069_8900,
                Conv::Thiscall(buffer),
                &[desc, src],
                |g| Out::U32(gp::set_value_checked(g, buffer, desc, src)),
            );
        }
    });
}

#[test]
fn element_word_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0059_3e80);
        for i in 0..64 {
            t.reset();
            let this = t.zeros(0x40);
            let n = rng.below(5) | if rng.chance(4) { 0x8000_0000 } else { 0 };
            let data: Vec<u32> = (0..7 * 5).map(|_| rng.next()).collect();
            let data = t.words(&data);
            t.w32(this + 0x24, n);
            let inline = rng.next();
            t.w32(
                this + 0x28,
                if n & 0x7fff_ffff == 1 { inline } else { data },
            );
            let k = if rng.chance(5) {
                rng.next()
            } else {
                rng.below(6)
            };
            let what = format!("element_word #{i}");
            t.check(&what, 0x0059_3e80, Conv::Thiscall(this), &[k], |g| {
                Out::U32(gp::element_word(g, this, k))
            });
        }
    });
}

/// A post-process manager's FXAA fields: the shader flag `+0x123b9`, the
/// colour surface `+0x8c`.
fn post_manager(t: &mut Twins, rng: &mut Rng) -> u32 {
    let this = t.zeros(0x12400);
    t.w8(this + 0x123b9, u8::from(!rng.chance(4)));
    let surface = t.words(&[rng.next(), rng.below(2)]);
    t.w32(this + 0x8c, surface);
    this
}

#[test]
fn render_fxaa_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0069_5740);
        for i in 0..48 {
            t.reset();
            let this = post_manager(t, &mut rng);
            t.method(post::FXAA_FORWARD, 4, rng.word());
            let map = [0x17, 0x15c, 0x47, rng.next()][rng.below(4) as usize];
            let at = t.rebase(post::CURRENT_MAP);
            t.patch(at, &map.to_le_bytes());
            let (renderer, target) = (rng.next(), rng.word());
            let what = format!("render_fxaa #{i}");
            t.check(
                &what,
                0x0069_5740,
                Conv::Thiscall(this),
                &[renderer, target],
                |g| Out::U32(gp::render_fxaa(g, this, renderer, target)),
            );
        }
    });
}

#[test]
fn fxaa_forward_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x00a2_0350);
        for i in 0..16 {
            t.reset();
            t.method(post::FXAA_PASS, 4, rng.word());
            let this = rng.next();
            let args: [u32; 4] = std::array::from_fn(|_| rng.word());
            let what = format!("fxaa_forward #{i}");
            t.check(&what, 0x00a2_0350, Conv::Thiscall(this), &args, |g| {
                Out::U32(gp::fxaa_forward(
                    g, this, args[0], args[1], args[2], args[3],
                ))
            });
        }
    });
}

/// A shader-parameter descriptor: type byte `+2`, offset `+8`, size
/// `+0xa` (16 bytes).
fn descriptor(t: &mut Twins, rng: &mut Rng) -> u32 {
    let d = t.zeros(0x10);
    let kind = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x40, rng.next() as u8][rng.below(8) as usize];
    t.w8(d + 2, kind);
    let at = [0, 2, 4, 8, 0x10, 0x30][rng.below(6) as usize] as u16;
    let size = [0x10u16, 0x10, 0x10, 0x8, 0x2010][rng.below(5) as usize];
    t.write(d + 8, &at.to_le_bytes());
    t.write(d + 0xa, &size.to_le_bytes());
    d
}

#[test]
fn fxaa_pass_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x00a2_0360);
        for i in 0..96 {
            t.reset();
            // stub results: 0, now and then an error
            let ret = |rng: &mut Rng| if rng.chance(12) { rng.word() } else { 0 };
            let trophy = if rng.chance(4) { rng.next() | 1 } else { 0 };
            t.method(post::TROPHY_INSTALL, 1, trophy);
            for (sym, n) in [
                (post::SURFACE_CTOR, 2),
                (post::SURFACE_DTOR, 0),
                (post::SET_COLOR_TARGET, 2),
                (post::SET_DEPTH_STENCIL, 1),
                (post::BEGIN_SCENE, 1),
                (post::RENDER_MESH_INSTANCE, 4),
                (post::END_SCENE, 1),
                (post::SET_TEXTURE_CHECKED, 2),
            ] {
                let r = ret(&mut rng);
                t.method(sym, n, r);
            }
            let this = t.zeros(0x80);
            let block = t.zeros(0x60);
            t.w32(block, rng.range(0x10, 0x50));
            let holder = t.words(&[0, block]);
            t.w32(this + 0x64, holder);
            let tex = if rng.chance(4) {
                0
            } else {
                descriptor(t, &mut rng)
            };
            let size = if rng.chance(4) {
                0
            } else {
                descriptor(t, &mut rng)
            };
            t.w32(this + 0x68, tex);
            t.w32(this + 0x6c, size);
            let mesh = t.words(&[rng.next(), 0, 0, 0, rng.next()]);
            t.w32(this + 0x60, mesh);
            let work = t.words(&[rng.next(), rng.below(2)]);
            t.w32(this + 0x5c, work);
            let renderer = t.zeros(0x140);
            t.w32(renderer + 0x13c, rng.next());
            let source = t.zeros(0x40);
            let dim = |rng: &mut Rng| match rng.below(4) {
                0 => rng.next(),
                1 => 0,
                _ => rng.range(1, 4000),
            };
            let (w, h) = (dim(&mut rng), dim(&mut rng));
            t.w32(source + 0x1c, w);
            t.w32(source + 0x20, h);
            let source = if rng.chance(8) && size == 0 {
                0
            } else {
                source
            };
            let target = if rng.chance(3) { 0 } else { rng.next() };
            let luma = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
            let args = [renderer, target, source, luma];
            let what = format!("fxaa_pass #{i}");
            t.check_framed(
                &what,
                0x00a2_0360,
                Conv::Thiscall(this),
                &args,
                0x60,
                0xc,
                |g, ebp| Out::U32(gp::fxaa_pass(g, ebp, this, renderer, target, source, luma)),
            );
        }
    });
}

#[test]
fn on_exit_audio_menu_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x006f_f5f0);
        for i in 0..32 {
            t.reset();
            t.method(c2_iggy::ADD_CALL, 2, rng.word());
            let v: f64 = [
                1.0,
                1.5,
                0.999,
                2.0,
                0.0,
                -1.0,
                f64::NAN,
                1e12,
                f64::from(rng.next()),
            ][rng.below(9) as usize];
            let bits = v.to_bits();
            let args = t.words(&[0, 0, bits as u32, (bits >> 32) as u32]);
            let handlers = rng.next();
            let what = format!("on_exit_audio_menu #{i}");
            t.check(&what, 0x006f_f5f0, Conv::Cdecl, &[handlers, args], |g| {
                Out::U32(gi::on_exit_audio_menu(g, handlers, args))
            });
        }
    });
}

/// A surface handle `{texture, flag byte}` whose texture (when the flag
/// is clear) has a size at `+0x1c`/`+0x20`.
fn surface(t: &mut Twins, rng: &mut Rng, flag: bool) -> u32 {
    let tex = t.zeros(0x40);
    let dim = |rng: &mut Rng| match rng.below(5) {
        0 => rng.next(),
        1 => 0,
        _ => rng.range(1, 4000),
    };
    let (w, h) = (dim(rng), dim(rng));
    t.w32(tex + 0x1c, w);
    t.w32(tex + 0x20, h);
    let flag = u32::from(flag && rng.chance(4));
    t.words(&[tex, flag])
}

#[test]
fn render_smaa_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0069_6af0);
        let n = if t.snapshot { 64 } else { 400 };
        let mut ends = [0u32; 3];
        for i in 0..n {
            t.reset();
            // most steps succeed; now and then one fails
            let ret = |rng: &mut Rng| if rng.chance(40) { rng.word() } else { 0 };
            let select = {
                let bits: Vec<u32> = (0..8)
                    .map(|_| if rng.chance(4) { rng.next() } else { u32::MAX })
                    .collect();
                t.words(&bits)
            };
            t.stub_select(smaa::FIND_BY_NAME, select);
            let allowed = u32::from(!rng.chance(10)) | (rng.next() & !0xff);
            t.stub(smaa::IS_EFFECT_ALLOWED, false, 1, 1, allowed);
            let params = rng.next();
            let buffer = rng.next();
            let pair = t.words(&[params, buffer]);
            let data = t.words(&[pair]);
            t.method(smaa::ARRAY_DATA, 0, data);
            for (sym, k) in [
                (smaa::SET_VALUE_DESC, 2),
                (smaa::SET_TEXTURE, 2),
                (smaa::SET_VALUE, 2),
                (smaa::SET_VALUE2, 2),
                (smaa::SET_INT, 2),
                (post::SURFACE_CTOR, 2),
                (post::SURFACE_DTOR, 0),
                (post::SET_COLOR_TARGET, 2),
                (post::SET_DEPTH_STENCIL, 1),
                (smaa::SET_VIEWPORT, 4),
                (smaa::SET_CLEAR_COLOR, 4),
                (smaa::SET_CLEAR_STENCIL, 1),
                (post::BEGIN_SCENE, 1),
                (smaa::RENDER_FOR_PASS, 1),
                (post::END_SCENE, 1),
                (smaa::COPY_BUFFER, 6),
                (smaa::COPY_DEPTH_BUFFER, 9),
            ] {
                let r = ret(&mut rng);
                t.method(sym, k, r);
            }
            let this = t.zeros(0x12400);
            let flag = |rng: &mut Rng| u8::from(!rng.chance(12));
            let (f1, f2) = (flag(&mut rng), flag(&mut rng));
            t.w8(this + 0x123d4, f1);
            t.w8(this + 0x123d5, f2);
            let quality = if rng.chance(12) {
                rng.word()
            } else {
                rng.below(6)
            };
            t.w32(this + 0x1231c, quality);
            let area = if rng.chance(12) { 0 } else { rng.next() };
            let search = if rng.chance(12) { 0 } else { rng.next() };
            t.w32(this + 0x122c4, area);
            t.w32(this + 0x122c8, search);
            let mesh = t.words(&[0, 0, 0, 0, rng.next()]);
            t.w32(this + 0x122b8, mesh);
            t.w32(this + 0x122c0, rng.next());
            // a set flag byte (no texture) only where no size is read
            for field in [0x8c, 0x90, 0x9c, 0xa4] {
                let s = surface(t, &mut rng, field == 0x90);
                t.w32(this + field, s);
            }
            t.w8(this + 0x12318, u8::from(rng.chance(2)));
            let renderer = t.zeros(0x80);
            let at = t.rebase(smaa::CURRENT_MAP);
            let map = rng.below(0x400);
            t.patch(at, &map.to_le_bytes());
            let what = format!("render_smaa #{i}");
            let mut got = 0;
            t.check_framed(
                &what,
                0x0069_6af0,
                Conv::Thiscall(this),
                &[renderer],
                0x104,
                0xc,
                |g, ebp| {
                    got = gm::render_smaa(g, ebp, this, renderer);
                    Out::U32(got)
                },
            );
            let log: u32 = Mem::read(&t.b, t.log);
            let end = if got != 0 { 2 } else { usize::from(log > 40) };
            ends[end] += 1;
        }
        // early outs, whole runs and errors all occur
        assert!(ends.iter().all(|&e| e > n / 20), "outcomes {ends:?}");
    });
}

/// `caster_crosses_edge` at its bounds: one caster with an exact box
/// (identity matrices, a 16-texel map) whose x0 or x1 lands on the map's
/// width.
#[test]
fn caster_crosses_edge_at_the_bounds() {
    Twins::each(|t| {
        let mut rng = Rng(0x0016_0016);
        let cases: [(f32, f32, f32, f32); 6] = [
            (19.0 / 16.0, 1.25, 0.5, 0.6),
            (13.0 / 16.0 - 0.5, 13.0 / 16.0, 0.5, 0.6),
            (20.0 / 16.0, 1.3, 0.5, 0.6),
            (0.2, 12.0 / 16.0, 0.5, 0.6),
            (0.5, 0.6, 19.0 / 16.0, 1.25),
            (0.5, 0.6, 0.1, 0.2),
        ];
        for (i, &(x0, x1, y0, y1)) in cases.iter().enumerate() {
            t.reset();
            let this = manager(t, &mut rng);
            for cell in 0..64u32 {
                t.w32(this + 0xde8 + 4 * cell, 0);
            }
            // the 3x4 is stored by columns: its identity has 1 at 0, 4, 8
            let id3x4: Vec<f32> = (0..12)
                .map(|k| if k % 4 == 0 { 1.0 } else { 0.0 })
                .collect();
            let m = floats(t, &id3x4);
            // the box is a corner and its extent (PBounds_transformBox)
            let object = floats(t, &[x0, y0, 0.25, 0.0, x1 - x0, y1 - y0, 0.25, 0.0]);
            t.w32(object + 0xc, m);
            let node = t.zeros(0x40);
            t.w32(node + 0x1c, object);
            let nodes = t.words(&[node]);
            let list = t.words(&[1, nodes]);
            t.w32(this + 0xde8, list);
            t.w32(this + 0xfe8, 1);
            t.w32(this + 0x10e8, 1);
            t.w8(this + 0x2b28, 1);
            let mut args: Vec<u32> = (0..16)
                .map(|k| if k % 5 == 0 { 1f32 } else { 0.0 }.to_bits())
                .collect();
            args.push(1);
            args.extend([10.0f32, 16.0, 16.0].map(f32::to_bits));
            let copy = t.words(&args);
            let what = format!("caster_crosses_edge bounds #{i}");
            let ebp = t.port_frame() + 0x200;
            t.check(&what, 0x006c_8f50, Conv::Thiscall(this), &args, |g| {
                Out::U8(gs::caster_crosses_edge(g, ebp, this, copy))
            });
        }
    });
}
