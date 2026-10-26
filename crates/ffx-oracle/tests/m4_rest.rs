//! M4 leftovers (`m4-rest`) against the original: on the image alone with
//! synthetic objects, and on a snapshot (`FFX_SNAPSHOT`) with the same
//! objects over the snapshot's memory. Results and every byte written
//! must agree. Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its stack
//! arguments to a log in memory and returns the next word of its own
//! result list, so the diff checks that both made the same calls, with
//! the same `this`, in the same order, and saw the same results.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::unreadable_literal
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_b::{m4r_dynblocks, m4r_mesh};
use ffx_prender::m4r_fonttext as ft;
use ffx_prender::m4r_gauss as gs;
use ffx_prender::m4r_glow as gl;
use ffx_prender::m4r_meshlist as ml;
use ffx_prender::m4r_modnet as mn;
use ffx_prender::m4r_nodecapture as nc;
use ffx_prender::m4r_rendercmds as rc;

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;
/// `fstp st(0); ret` at the cave's last bytes: pops a callee's `ST0` in
/// the port's oracle, as the original's `fstp` after the call does.
const POP: u32 = CAVE + CAVE_LEN - 4;

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
}

/// The bridge's view of an oracle.
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

fn oracle_conv(conv: host::Conv) -> Conv {
    match conv {
        host::Conv::Cdecl => Conv::Cdecl,
        host::Conv::Stdcall => Conv::Stdcall,
        host::Conv::Thiscall(t) => Conv::Thiscall(t),
        host::Conv::Fastcall => Conv::Fastcall,
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, false));
        self.0
            .call(addr, oracle_conv(conv), args)
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
        let r = self
            .0
            .call(addr, oracle_conv(conv), args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        let v = r.st0_f32().expect("the callee's ST0");
        // the caller's fstp pops it
        let pop = self.0.rebase(POP);
        self.0.call(pop, Conv::Cdecl, &[]).unwrap();
        v
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
        t.log = t.zeros(0x4000);
        let pop = t.rebase(POP);
        t.write(pop, &[0xdd, 0xd8, 0xc3]);
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

    fn w32s(&mut self, at: u32, words: &[u32]) {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        self.write(at, &bytes);
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
    /// words; returns the next word of the list at the returned cell
    /// (`[cell]` points at it; set it with [`Twins::results`]).
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32) -> u32 {
        let at = self.rebase(sym);
        let (addr, cell) = self.stub_code(sym, this, n, pops);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        cell
    }

    /// The stub of [`Twins::stub`] in the cave only (for import slots),
    /// `tag` its log tag: its address and its result cell.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32) -> (u32, u32) {
        self.stub_custom(tag, this, n, pops, 4, &[])
    }

    /// [`Twins::stub_code`] whose result list has `stride`-byte entries
    /// (the result the first word) and which runs `body` with `eax` at the
    /// entry (`body` may use `ecx` and `edx` and must keep `eax`).
    fn stub_custom(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        stride: u8,
        body: &[u8],
    ) -> (u32, u32) {
        assert!(n <= 6);
        let sym = tag;
        let cell = self.zeros(4);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
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
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.extend(body);
        c.extend([0x8b, 0x00]); // mov eax, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(stride); // add dword [cell], stride
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        let addr = self.cave_next;
        assert!(
            addr + c.len() as u32 <= self.cave + CAVE_LEN - 4,
            "cave full"
        );
        self.write(addr, &c);
        self.cave_next = addr + c.len() as u32;
        // a default list of zeros
        let list = self.zeros(64 * 4);
        self.w32(cell, list);
        (addr, cell)
    }

    /// The stub at `cell` returns `words` in order, then zeros.
    fn results(&mut self, cell: u32, words: &[u32]) {
        let mut all = words.to_vec();
        all.resize(words.len().max(64) + 16, 0);
        let list = self.alloc(&all.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>());
        self.w32(cell, list);
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

    /// Where the original's `ebp` is when it is called with `args` words
    /// (the call pushes from the stack's end less 0x100).
    fn frame_ebp(&self, args: usize) -> u32 {
        self.a.stack_range().end - 0x100 - 4 * args as u32 - 8
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp` (its
    /// callees log pointers into it, and read what it keeps there): the
    /// `low` bytes below `ebp` start zeroed in both and are compared after
    /// the call, but for the top `skip` bytes (a stack cookie).
    #[allow(clippy::too_many_arguments)]
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        skip: u32,
        port: impl FnOnce(&mut G, u32) -> u32,
    ) {
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        self.write(lo, &vec![0; low as usize]);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |g| port(g, ebp));
        self.b.set_call_top(None);
        let n = (low - skip) as usize;
        let mut x = vec![0; n];
        let mut y = vec![0; n];
        Mem::read_bytes(&self.a, lo, &mut x);
        Mem::read_bytes(&self.b, lo, &mut y);
        let first = x.iter().zip(&y).position(|(a, b)| a != b);
        assert!(
            first.is_none(),
            "{what}: frame differs at ebp - {:#x}",
            first.map_or(0, |i| low as usize - i)
        );
    }

    /// [`Twins::check`] with the port's frame at the original's `ebp`, as
    /// [`Twins::check_framed`], but comparing only the frame's `ranges`
    /// afterwards (`(ebp offset, bytes)`: what the callees see; the
    /// original's other locals are its own).
    #[allow(clippy::too_many_arguments)]
    fn check_framed_ranges(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        low: u32,
        ranges: &[(u32, u32)],
        port: impl FnOnce(&mut G, u32) -> u32,
    ) {
        let ebp = self.frame_ebp(args.len());
        let lo = ebp - low;
        // stale bytes, so that a record the callees fill must be filled
        self.write(lo, &vec![0x5a; low as usize]);
        self.b.set_call_top(Some(lo - 0x100));
        self.check(what, sym, conv, args, |g| port(g, ebp));
        self.b.set_call_top(None);
        for &(off, len) in ranges {
            let mut x = vec![0; len as usize];
            let mut y = vec![0; len as usize];
            Mem::read_bytes(&self.a, ebp - off, &mut x);
            Mem::read_bytes(&self.b, ebp - off, &mut y);
            assert_eq!(x, y, "{what}: frame differs at ebp - {off:#x}");
        }
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    /// Runs the original at `sym` in `a` and `port` on `b`; compares `eax`
    /// and every byte written. The original must leave no `ST0`.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> u32,
    ) {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let r = port(&mut G(&mut self.b));
        assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
        assert_eq!(ra.eax, r, "{what}: eax");
        self.diff(what);
    }
}

// ---- PArray16_resize_00498f30 ----

#[test]
fn resize16_matches_the_original() {
    const SYM: u32 = 0x0049_8f30;
    for (name, mut t) in Twins::all() {
        let malloc = t.stub(m4r_mesh::MALLOC, false, 2, 0);
        t.stub(ffx_pother_b::PHYRE_FREE, false, 1, 0);
        let mut rng = Rng(0x1234_5678);
        let mut calls = 0;
        // (count, head, old data, malloc result: 0 fails, 1 = old, 2 = new)
        let cases: &[(u32, u32, u32, u32)] = &[
            (3, 3, 1, 2),           // same count: nothing
            (0, 0, 0, 2),           // same count 0
            (4, 2, 1, 2),           // grow, owned old freed
            (4, 0x8000_0002, 1, 2), // grow, old not owned
            (4, 2, 0, 2),           // grow, no old storage
            (5, 2, 1, 1),           // the allocator hands back the old storage
            (6, 2, 1, 0),           // allocation fails: 0xd
            (0, 7, 1, 2),           // shrink to nothing: old freed, data 0
            (0, 0x8000_0007, 1, 2), // shrink to nothing, not owned
            (2, 0x8000_0000, 1, 2), // a not-owned empty array
            (1, 0xffff_ffff, 1, 2),
        ];
        for (i, &(count, head, old, got)) in cases.iter().enumerate() {
            let array = t.zeros(16);
            let old_data = t.zeros(0x100);
            t.fill(&mut rng, old_data, 0x100);
            let new_data = t.zeros(0x100);
            t.fill(&mut rng, new_data, 0x100);
            t.w32s(
                array,
                &[
                    head,
                    if old == 1 { old_data } else { 0 },
                    rng.next(),
                    rng.next(),
                ],
            );
            let ret = match got {
                0 => 0,
                1 => old_data,
                _ => new_data,
            };
            t.results(malloc, &[ret, ret]);
            t.check(
                &format!("{name} case {i}"),
                SYM,
                Conv::Thiscall(array),
                &[count],
                |g| m4r_mesh::bridge::resize16(g, array, count),
            );
            calls = t.log_entries();
        }
        assert!(calls >= 9, "{name}: the stubs ran ({calls} calls)");
    }
}

// ---- RadialLine_createMeshData_006f8c50 ----

#[test]
fn radial_create_mesh_matches_the_original() {
    const SYM: u32 = 0x006f_8c50;
    for (name, mut t) in Twins::all() {
        let malloc = t.stub(m4r_mesh::MALLOC, false, 2, 0);
        t.stub(m4r_mesh::SEGMENTS_CONSTRUCT, false, 2, 0);
        t.stub(m4r_mesh::SEGMENTS_DESTROY, false, 2, 0);
        t.stub(m4r_mesh::BLOCK_CTOR, true, 0, 0);
        t.stub(m4r_mesh::ARRAY40_SET, true, 2, 2);
        let new = t.stub(m4r_mesh::NEW, false, 1, 0);
        for f in m4r_mesh::FILL {
            t.stub(f, true, 0, 0);
        }
        t.stub(m4r_mesh::FILL_INDICES, true, 0, 0);
        let bound = t.stub(m4r_mesh::IS_BOUND, true, 0, 0);
        let release = t.stub(m4r_mesh::RELEASE, true, 0, 0);
        let mut rng = Rng(0x0bad_cafe);
        for case in 0..24_u32 {
            let lines = t.zeros(0x80);
            t.fill(&mut rng, lines, 0x80);
            let holder = t.zeros(8);
            let entry = t.zeros(0x6c);
            t.fill(&mut rng, entry, 0x6c);
            let seg = t.zeros(0x6c);
            t.fill(&mut rng, seg, 0x6c);
            let blocks = t.zeros(0xc0);
            // the holder: one entry already, or something else
            let head = match case % 4 {
                0 => 1,
                1 => 2,
                2 => 0x8000_0003,
                _ => rng.next(),
            };
            t.w32s(holder, &[head, entry]);
            t.w32(lines + 0x20, holder);
            let a = rng.below(9);
            let b = rng.below(9);
            t.w32(lines + 0x30, a);
            t.w32(lines + 0x34, b);
            let n = (a + b) * 4;
            // the stream array: three already, or not
            t.w32(seg + 0x28, if case % 3 == 0 { 3 } else { rng.below(5) });
            t.w32(entry + 0x28, if case % 5 == 0 { 3 } else { rng.below(5) });
            // the segment and blocks allocations; the segment one fails
            // in case 7, the blocks one in case 11
            // (case 5 mod 8: the allocator hands back the old, owned array)
            let seg_ret = match case % 8 {
                7 => 0,
                5 => entry,
                _ => seg,
            };
            let blocks_ret = if case % 12 == 11 { 0 } else { blocks };
            let first_holder = head != 1;
            if first_holder {
                t.results(malloc, &[seg_ret, blocks_ret]);
            } else {
                t.results(malloc, &[blocks_ret]);
            }
            // the six arrays; the quads one fails in case 5
            let mut news = Vec::new();
            for k in 0..6 {
                let size = 0x40 + n * 16;
                let buf = t.zeros(size as usize);
                news.push(if k == 5 && case % 6 == 5 { 0 } else { buf });
            }
            t.results(new, &news);
            // the index block's byte size: equal to 4 * n sometimes
            let block_entry = if first_holder && seg_ret != 0 {
                seg_ret
            } else {
                entry
            };
            if case % 7 == 3 {
                t.w32(block_entry + 0x30 + 0x34, n * 4);
            }
            t.results(
                bound,
                &[if case % 2 == 0 {
                    0x1234_5600
                } else {
                    0x1234_5601
                }],
            );
            t.results(release, &[0x00ab_cdef]);
            t.check(
                &format!("{name} case {case} (n {n})"),
                SYM,
                Conv::Thiscall(lines),
                &[],
                |g| m4r_mesh::bridge::create_mesh_data(g, lines),
            );
        }
        assert!(t.log_entries() > 100, "{name}: the stubs ran");
    }
}

#[test]
fn array_size_saturates() {
    assert_eq!(m4r_mesh::array_size(3, 4), 12);
    assert_eq!(m4r_mesh::array_size(0x4000_0000, 4), u32::MAX);
    assert_eq!(m4r_mesh::array_size(0x3fff_ffff, 4), 0xffff_fffc);
}

// ---- ClassDynamicMesh_fillDynamicBlocks_00672b80 ----

#[test]
fn fill_dynamic_blocks_matches_the_original() {
    const SYM: u32 = 0x0067_2b80;
    for (name, mut t) in Twins::all() {
        // the CRT's stderr and fprintf through their import slots
        let (iob, iob_cell) = t.stub_code(m4r_dynblocks::IOB_FUNC, false, 0, 0);
        let (fprintf, _) = t.stub_code(m4r_dynblocks::FPRINTF, false, 2, 0);
        let slot = t.rebase(m4r_dynblocks::IOB_FUNC);
        t.w32(slot, iob);
        let slot = t.rebase(m4r_dynblocks::FPRINTF);
        t.w32(slot, fprintf);
        let on = t.rebase(m4r_dynblocks::BUFFERS_ON);
        let mut rng = Rng(0x5eed_0672);
        let mut ran = 0;
        for case in 0..160_u32 {
            t.results(iob_cell, &[0x0077_0000 + case]);
            let n = 1 + rng.below(4);
            let src = t.zeros(0x98);
            let records = t.zeros(0x6c * n as usize);
            t.w32(src + 0x90, n);
            t.w32(src + 0x94, records);
            let inst = t.zeros(0x18);
            t.fill(&mut rng, inst, 0x18);
            // ids: 0 and 1 mostly, -1 sometimes (2 and more read the
            // original's frame, see m4r_dynblocks::Source::of_buffer)
            let id = match rng.below(8) {
                0 => u32::MAX,
                1..=3 => 0,
                _ => 1,
            };
            t.w32(inst + 0x14, id);
            t.w32(inst + 0x10, if case % 23 == 5 { 0 } else { src });
            if case % 19 == 7 {
                t.w32(src + 0x90, 0x8000_0001); // a negative count
            }
            t.w32(on, u32::from(case % 17 != 3));
            let index = match case % 13 {
                4 => n,
                9 => u32::MAX,
                _ => rng.below(n),
            };
            // each record: counts, sources (some null), overrides with
            // data or not
            for r in 0..n {
                let rec = records + 0x6c * r;
                t.fill(&mut rng, rec, 0x6c);
                for base in [0, 0x2c, 0x48] {
                    let at = if base == 0 { rec + 4 } else { rec + base };
                    let verts = rng.below(6);
                    let indices = rng.below(12);
                    t.w32(at, verts);
                    t.w32(at + 4, indices);
                    for k in 0..5 {
                        // (the indices, k 4, are always there: the original
                        // reads them unchecked)
                        let p = if k < 4 && rng.below(6) == 0 {
                            0
                        } else {
                            let p = t.zeros(0x80);
                            t.fill(&mut rng, p, 0x80);
                            p
                        };
                        t.w32(at + 8 + 4 * k, p);
                    }
                    if base != 0 && rng.below(3) == 0 {
                        t.w32(at + 8, 0); // no data in this buffer
                    }
                }
            }
            // the four blocks (some null, some without storage)
            let streams = t.zeros(16);
            for k in 0..4 {
                if rng.below(5) == 0 {
                    continue;
                }
                let block = t.zeros(0x40);
                t.fill(&mut rng, block, 0x40);
                let store = if rng.below(6) == 0 { 0 } else { t.zeros(0x60) };
                t.w32(block + 0x3c, store);
                t.w32(streams + 4 * k, block);
            }
            let index_block = if case % 11 == 6 {
                0
            } else {
                let b = t.zeros(0x40);
                t.fill(&mut rng, b, 0x40);
                // the length: enough mostly, too short sometimes
                let len = if rng.below(5) == 0 {
                    rng.below(4)
                } else {
                    12 + rng.below(4)
                };
                t.w32(b + 8, len);
                let format = if rng.below(2) == 0 {
                    8
                } else {
                    rng.below(16) as u8
                };
                t.write(b + 0xc, &[format]);
                let to = t.zeros(0x40);
                t.w32(b + 0x38, to);
                b
            };
            // the early path writes the index block's +8: skip a null
            // one there (both fault)
            let early = case % 23 == 5
                || case % 19 == 7
                || case % 17 == 3
                || case % 13 == 4
                || case % 13 == 9
                || id == u32::MAX;
            if early && index_block == 0 {
                continue;
            }
            let mesh = rng.next();
            t.check(
                &format!("{name} case {case}"),
                SYM,
                Conv::Thiscall(mesh),
                &[index, inst, streams, index_block],
                |g| {
                    m4r_dynblocks::bridge::fill_dynamic_blocks(g, index, inst, streams, index_block)
                },
            );
            ran += 1;
        }
        assert!(ran > 140, "{name}: {ran} cases ran");
        assert!(t.log_entries() > 10, "{name}: the warnings ran");
    }
}

// ---- PRendererBase::renderMeshInstanceList (0x005b3df0) ----

/// The job builder's `add`: logs, then adds the entry's words 1..3 to the
/// builder's instance count, record count and constant bytes (`ebp -
/// 0x14`, `- 0xc`, `- 8`: `this + 0x2314`, `+ 0x231c`, `+ 0x2320`) and
/// returns word 0.
const BUILDER_ADD_BODY: [u8; 27] = [
    0x8b, 0x50, 0x04, // mov edx, [eax+4]
    0x01, 0x91, 0x14, 0x23, 0x00, 0x00, // add [ecx+0x2314], edx
    0x8b, 0x50, 0x08, // mov edx, [eax+8]
    0x01, 0x91, 0x1c, 0x23, 0x00, 0x00, // add [ecx+0x231c], edx
    0x8b, 0x50, 0x0c, // mov edx, [eax+0xc]
    0x01, 0x91, 0x20, 0x23, 0x00, 0x00, // add [ecx+0x2320], edx
];

/// A ring's `stepForAllocation`: stores the entry's word 1 at its third
/// argument and returns word 0.
const STEP_BODY: [u8; 9] = [
    0x8b, 0x50, 0x04, // mov edx, [eax+4]
    0x8b, 0x4c, 0x24, 0x0c, // mov ecx, [esp+0xc]
    0x89, 0x11, // mov [ecx], edx
];

#[test]
fn render_mesh_instance_list_matches_the_original() {
    const SYM: u32 = 0x005b_3df0;
    for (name, mut t) in Twins::all() {
        let threads = t.stub(ml::THREAD_COUNT, false, 0, 0);
        let thread_instance = t.stub(ml::THREAD_INSTANCE, false, 0, 0);
        let pool_cell = t.stub(ml::THREAD_POOL, false, 0, 0);
        t.stub(ml::SET_CAMERA, true, 1, 1);
        t.stub(ml::BUILDER_CTOR, true, 3, 3);
        let (add, add_cell) = t.stub_custom(ml::BUILDER_ADD, true, 2, 2, 16, &BUILDER_ADD_BODY);
        let (step, step_cell) = t.stub_custom(ml::STEP_FOR_ALLOCATION, true, 3, 3, 8, &STEP_BODY);
        for (sym, addr) in [(ml::BUILDER_ADD, add), (ml::STEP_FOR_ALLOCATION, step)] {
            let at = t.rebase(sym);
            let mut j = vec![0xe9];
            j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
        }
        t.stub(ml::BUILDER_FILL, true, 1, 1);
        t.stub(ml::SEMAPHORE_WAIT, true, 0, 0);
        t.stub(ml::NEXT_SEGMENT, true, 0, 0);
        let allocate = t.stub(ml::SEGMENTED_ALLOCATE, true, 1, 1);
        let new_job = t.stub(ml::NEW_JOB, false, 0, 0);
        t.stub(ml::THREAD_REGISTER, true, 1, 1);
        t.stub(ml::FREE, false, 1, 0);
        let render_one = t.stub(ml::RENDER_MESH_INSTANCE, true, 4, 4);
        // the renderer's slot 2 (no arguments); the pool's slots 5 and 7
        // (one) and 6 (none)
        let mut words = Vec::new();
        for slot in 0..8 {
            let n = u32::from(slot == 5 || slot == 7);
            let (f, _) = t.stub_code(0x7e00_0000 + slot, true, n, n);
            words.extend(f.to_le_bytes());
        }
        let vtable = t.alloc(&words);
        let mut rng = Rng(0x05b3_df00);
        let mut calls = 0;
        for case in 0..120_u32 {
            // a fresh call log per case (it holds 511 entries)
            let log = t.log;
            t.w32(log, 0);
            let r = t.zeros(0x140);
            t.fill(&mut rng, r, 0x140);
            t.w32(r, vtable);
            if rng.below(4) == 0 {
                t.w32(r + 0x5c, 0); // no camera
            }
            if rng.below(3) == 0 {
                t.w32(r + 0x44, 0); // no semaphore
            }
            // the rings' write cursors and readers: small, to reach every
            // branch of the fit test
            for ring in [r + 0x1c, r + 0x30] {
                t.w32(ring + 8, rng.below(6));
                t.w32(ring + 0xc, rng.below(6));
            }
            let n_threads = 1 + rng.below(3);
            let count = match case % 5 {
                _ if case % 22 == 3 => 0, // no threads, no contexts
                0 => n_threads + 1,
                1 => n_threads | 0x8000_0000,
                _ => n_threads,
            };
            t.results(threads, &[if case % 11 == 3 { 0 } else { n_threads }]);
            let data = t.zeros(0xe4 * 4);
            t.fill(&mut rng, data, 0xe4 * 4);
            let contexts = t.zeros(8);
            t.w32s(contexts, &[count, data]);
            let thread = rng.next();
            t.results(thread_instance, &[thread]);
            let pool = t.zeros(8);
            t.w32(pool, vtable);
            t.results(pool_cell, &[pool]);
            // the instances
            let k = if case % 13 == 0 { 0 } else { rng.below(26) };
            let list = t.zeros(4 * k as usize + 4);
            for i in 0..k {
                let inst = t.zeros(0x20);
                t.fill(&mut rng, inst, 0x20);
                t.w32(list + 4 * i, inst);
            }
            // the builder's answers: took (adding records and bytes) or
            // refused
            let mut adds = Vec::new();
            for _ in 0..40 {
                if rng.below(5) == 0 {
                    adds.extend([1, 0, 0, 0]);
                } else {
                    let bytes = if rng.below(9) == 0 {
                        0x8000_0000
                    } else {
                        rng.below(4) * 0x10
                    };
                    adds.extend([0, 1, rng.below(3), bytes]);
                }
            }
            t.results(add_cell, &adds);
            let steps: Vec<u32> = (0..64).map(|_| rng.below(7)).collect();
            t.results(step_cell, &steps);
            let blocks: Vec<u32> = (0..40).map(|_| rng.next()).collect();
            t.results(allocate, &blocks);
            let mut jobs = Vec::new();
            for _ in 0..16 {
                let job = t.zeros(0x40);
                t.fill(&mut rng, job, 0x40);
                if rng.below(2) == 0 {
                    t.w32(job + 0x1c, rng.below(4)); // owned
                }
                if rng.below(2) == 0 {
                    t.w32(job + 0x24, rng.below(4));
                }
                if rng.below(4) == 0 {
                    t.w32(job + 0x20, 0);
                }
                jobs.push(job);
            }
            t.results(new_job, &jobs);
            let errors: Vec<u32> = (0..30)
                .map(|_| if rng.below(3) == 0 { rng.next() } else { 0 })
                .collect();
            t.results(render_one, &errors);
            let begin = list;
            let end = list + 4 * k;
            t.check_framed(
                &format!("{name} case {case} ({k} instances)"),
                SYM,
                Conv::Thiscall(r),
                &[begin, end, contexts],
                ml::FRAME,
                // the cookie, and _chkstk's saved ecx below it
                8,
                |g, ebp| ml::bridge::render_list(g, r, begin, end, contexts, ebp),
            );
            calls += t.log_entries();
        }
        assert!(calls > 2000, "{name}: the stubs ran ({calls} calls)");
    }
}

// ---- PRendererBase::generateNodeParameterCaptureBuffer (0x005b7fe0) ----

/// A float in [-4, 4) (no NaNs: the emulator keeps signalling ones).
fn small_float(rng: &mut Rng) -> u32 {
    ((rng.below(0x8000) as f32 - 16384.0) / 4096.0).to_bits()
}

#[test]
fn generate_node_parameters_matches_the_original() {
    const SYM: u32 = 0x005b_7fe0;
    for (name, mut t) in Twins::all() {
        // the ring, the texture lookup and the warning stay stubs; the
        // matrix helpers, the slice and the buffer writes run as they are
        let allocate = t.stub(nc::ALLOCATE, true, 1, 1);
        let texture = t.stub(nc::TEXTURE, true, 1, 1);
        t.stub(nc::PRINTF, false, 3, 0);
        let warned = t.rebase(nc::WARNED);
        let mut rng = Rng(0x05b7_fe00);
        let floats = |t: &mut Twins, rng: &mut Rng, n: u32| -> u32 {
            let words: Vec<u32> = (0..n).map(|_| small_float(rng)).collect();
            let at = t.zeros(4 * n as usize);
            t.w32s(at, &words);
            at
        };
        let mut wrote = 0;
        for case in 0..90_u32 {
            t.w32(t.log, 0);
            t.write(warned, &[u8::from(case % 3 == 0)]);
            let r = floats(&mut t, &mut rng, 0x50);
            let camera = if case % 29 == 7 {
                0
            } else {
                floats(&mut t, &mut rng, 0x30)
            };
            t.w32(r + 0x5c, camera);
            // the instance, its mesh, joints and textures
            let joints = floats(&mut t, &mut rng, 16 * 8);
            let mesh = t.zeros(0x10);
            t.w32s(mesh, &[0, 0, rng.below(300) | (rng.below(2) << 31), joints]);
            let textures = t.zeros(16);
            for k in 0..4 {
                let tex = if rng.below(4) == 0 {
                    0
                } else {
                    let x = t.zeros(8);
                    t.w32s(x, &[rng.next(), rng.next()]);
                    x
                };
                t.w32(textures + 4 * k, tex);
            }
            let instance = t.zeros(0x80);
            t.w32(instance, mesh);
            t.w32(instance + 0x7c, textures);
            let tex = t.zeros(8);
            t.w32s(tex, &[rng.next(), rng.next()]);
            t.results(texture, &[if rng.below(3) == 0 { 0 } else { tex }; 32]);
            // the skin: bind matrices and (bind, joint) pairs
            let binds = floats(&mut t, &mut rng, 16 * 8);
            let bones = rng.below(10);
            let pairs = t.zeros(4 * 10);
            for i in 0..10 {
                let pair = rng.below(8) | (rng.below(8) << 16);
                t.w32(pairs + 4 * i, pair);
            }
            let skin = t.zeros(0x10);
            t.w32s(skin, &[0, 0, bones | (rng.below(2) << 31), pairs]);
            // the node: its matrix block inline at +0x28
            let node = t.zeros(0x70);
            let block: Vec<u32> = (0..16).map(|_| small_float(&mut rng)).collect();
            t.w32s(node + 0x28, &block);
            let world = if case % 7 == 2 {
                0
            } else {
                floats(&mut t, &mut rng, 16)
            };
            t.w32s(
                node + 0x10,
                &[instance, binds, 0, skin, rng.below(4), world],
            );
            // the layout: entries at 4 + 0x80 k, semantics around 0x80..0x91
            let n = if case % 17 == 5 { 0 } else { 1 + rng.below(12) };
            let entries = t.zeros(16 * n as usize + 16);
            for k in 0..n {
                let mut semantic = 0x7f + rng.below(0x15);
                // without a world matrix only the semantics that test it
                // or don't use it (the others read through 0)
                while world == 0 && matches!(semantic, 0x83 | 0x84 | 0x87 | 0x8a..=0x8d) {
                    semantic = 0x7f + rng.below(0x15);
                }
                if world == 0 && k == 0 {
                    semantic = 0x82; // skipped without one
                }
                let format = match rng.below(5) {
                    0 => 0x2f,
                    1 | 2 => 0x30,
                    _ => 0x31,
                };
                let size = if semantic == 0x8f && format == 0x30 {
                    48 * (1 + rng.below(9))
                } else if semantic == 0x8f && k < 5 && rng.below(3) == 0 {
                    0x1040 // room for 65 (bit 6 of the 7-bit count)
                } else if semantic == 0x8f {
                    64 * (1 + rng.below(9))
                } else {
                    [4, 16, 64][rng.below(3) as usize]
                };
                let e = entries + 16 * k;
                t.w32(e, rng.next());
                t.write(e + 2, &[semantic as u8, format as u8]);
                t.w32(e + 4, rng.next());
                t.w32(e + 8, (4 + 0x200 * k) | (size << 16));
                t.w32(e + 0xc, rng.next());
            }
            let layout = t.zeros(0x40);
            t.fill(&mut rng, layout, 0x40);
            t.w32(layout + 0x28, n | (rng.below(2) << 31));
            t.w32(layout + 0x2c, entries);
            t.write(layout + 0x32, &0x1c00_u16.to_le_bytes());
            t.w32(node + 0x68, layout);
            let buffer = t.zeros(0x1c00);
            t.results(allocate, &[if case % 23 == 9 { 0 } else { buffer }]);
            t.check_framed(
                &format!("{name} case {case} ({n} entries)"),
                SYM,
                Conv::Thiscall(r),
                &[node],
                nc::FRAME,
                0,
                |g, ebp| nc::bridge::generate(g, r, node, ebp),
            );
            let mut first = [0_u8; 8];
            t.a.read_bytes(buffer + 4, &mut first);
            wrote += u32::from(first != [0; 8]);
        }
        assert!(wrote > 30, "{name}: values written in {wrote} cases");
    }
}

// ---- the modifier network's packet run (0x005c1f90) ----

/// A modifier's process function, after the log: when its argument
/// block (`[esp+4]`: `{inputs, outputs}` counts) has both, writes
/// `inputs[0]`, the context's two words and the item count to
/// `outputs[0]`.
const MODIFIER_BODY: [u8; 61] = [
    0x50, // push eax
    0x8b, 0x44, 0x24, 0x08, // mov eax, [esp+8] (the argument block)
    0x83, 0x38, 0x00, // cmp dword [eax], 0
    0x74, 0x32, // je skip
    0x83, 0x78, 0x04, 0x00, // cmp dword [eax+4], 0
    0x74, 0x2c, // je skip
    0x8b, 0x44, 0x24, 0x14, // mov eax, [esp+0x14] (outputs)
    0x8b, 0x10, // mov edx, [eax]
    0x85, 0xd2, // test edx, edx
    0x74, 0x22, // je skip
    0x8b, 0x44, 0x24, 0x10, // mov eax, [esp+0x10] (inputs)
    0x8b, 0x00, // mov eax, [eax]
    0x89, 0x02, // mov [edx], eax
    0x8b, 0x44, 0x24, 0x18, // mov eax, [esp+0x18] (the context)
    0x8b, 0x00, // mov eax, [eax]
    0x89, 0x42, 0x04, // mov [edx+4], eax
    0x8b, 0x44, 0x24, 0x18, // mov eax, [esp+0x18]
    0x8b, 0x40, 0x04, // mov eax, [eax+4]
    0x89, 0x42, 0x0c, // mov [edx+0xc], eax
    0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp+0xc] (the item count)
    0x89, 0x42, 0x08, // mov [edx+8], eax
    0x58, // skip: pop eax
];

#[test]
fn modifier_network_run_matches_the_original() {
    const SYM: u32 = 0x005c_1f90;
    for (name, mut t) in Twins::all() {
        let malloc = t.stub(mn::MALLOC, false, 2, 0);
        t.stub(mn::FREE, false, 1, 0);
        let (process, process_cell) = t.stub_custom(0x7d00_0000, false, 5, 0, 4, &MODIFIER_BODY);
        let scratch = t.zeros(mn::SCRATCH as usize);
        let mut rng = Rng(0x05c1_f900);
        let mut calls = 0;
        for case in 0..90_u32 {
            let log = t.log;
            t.w32(log, 0);
            t.results(process_cell, &[]);
            // case 1: no streams, more items than one chunk of the
            // empty network's 0x40000
            let items = match case % 6 {
                _ if case == 1 => 0x3_0000,
                0 => 0,
                1 => 1 + rng.below(8),
                _ => 40 + rng.below(120),
            };
            // no scratch buffer only without items (the modifiers would
            // write near address 0)
            t.results(malloc, &[if case % 12 == 6 { 0 } else { scratch }]);
            let k = 1 + rng.below(3);
            let n_src = if case == 1 { 0 } else { rng.below(4) };
            let n_out = if case % 9 == 2 || case == 1 {
                0
            } else {
                1 + rng.below(4)
            };
            let n_mod = if case % 11 == 3 { 0 } else { 1 + rng.below(4) };
            // the outputs' destinations; a wide output without one makes
            // several chunks
            let mut dests = Vec::new();
            let mut size_list = Vec::new();
            for _ in 0..n_src {
                size_list.push(4 * (1 + rng.below(0x40)));
            }
            for _ in 0..n_out {
                let with_dest = rng.below(3) != 0;
                let size = if !with_dest && rng.below(2) == 0 {
                    0x1000
                } else {
                    4 * (1 + rng.below(0x40))
                };
                let dest = if with_dest {
                    t.zeros((size * items.max(1)) as usize)
                } else {
                    0
                };
                dests.push(dest);
                size_list.push(size);
            }
            // the network: k records, n_mod entries, then the sizes
            let net = t.zeros((32 * (k + 1 + n_mod) + 64) as usize);
            t.w32s(net, &[0, k, n_mod, n_src | (n_out << 16)]);
            let mut counts = Vec::new();
            for r in 0..k {
                let rec = net + 0x20 + 32 * r;
                let obj = t.zeros(0x28);
                t.w32(obj + 0x24, process);
                let n_in = if n_src + n_out == 0 { 0 } else { rng.below(3) };
                let n_o = rng.below(3);
                t.w32s(rec, &[obj, n_in, n_o]);
                counts.push((n_in, n_o));
            }
            let mut records = Vec::new();
            for m in 0..n_mod {
                let e = net + 32 * (k + 1 + m);
                let record = rng.below(k);
                records.push(record);
                t.w32(e, rng.below(0x1000));
                let mut bytes = vec![record as u8];
                let (n_in, n_o) = counts[record as usize];
                for _ in 0..n_in {
                    bytes.push(rng.below(n_src + n_out) as u8);
                }
                for _ in 0..n_o {
                    // outputs are numbered after the sources; some point
                    // past them, 0xff wraps
                    let o = n_src + rng.below(n_out + 2);
                    bytes.push(if rng.below(9) == 0 { 0xff } else { o as u8 });
                }
                t.write(e + 8, &bytes);
            }
            let sizes = net + 32 * (k + 1 + n_mod);
            for (i, &s) in size_list.iter().enumerate() {
                t.write(sizes + 2 * i as u32, &(s as u16).to_le_bytes());
            }
            // the packet: the network at +0x10, offsets from there
            let packet = t.zeros(0x300);
            let desc = packet + 0x10;
            let sources = t.zeros(4 * n_src as usize + 4);
            for i in 0..n_src {
                let len = size_list[i as usize] * items.max(1);
                let data = t.zeros(len as usize);
                t.fill(&mut rng, data, len);
                t.w32(sources + 4 * i, data);
            }
            t.w32s(packet + 0x10, &[net, sources, rng.below(0x100)]);
            t.write(desc + 0x12, &0x100_u16.to_le_bytes());
            t.w32(packet + 0x24, items);
            t.w32s(desc + 0x100, &dests);
            for (m, &record) in records.iter().enumerate() {
                let m = m as u32;
                t.write(desc + 0x24 + 2 * m, &((0x200 + 8 * m) as u16).to_le_bytes());
                let (n_in, n_o) = counts[record as usize];
                t.w32s(desc + 0x200 + 8 * m, &[n_in, n_o]);
            }
            let out = if rng.below(3) == 0 { 0 } else { t.zeros(4) };
            t.w32s(packet + 0x28, &[out, rng.next()]);
            // the frame: the context at ebp - 0x58, the _alloca'd arrays
            // where the original puts them (16-byte aligned below the
            // prologue's esp); the port's calls run below them
            let ebp = t.frame_ebp(1);
            t.write(ebp - 0x80, &[0; 0x80]);
            let mut esp = ebp - 0x74;
            let mut alloca = move |bytes: u32| {
                esp = esp.wrapping_sub(bytes) & !15;
                esp
            };
            t.b.set_call_top(Some(ebp - 0x2000));
            t.check(
                &format!("{name} case {case} ({items} items)"),
                SYM,
                Conv::Cdecl,
                &[packet],
                |g| mn::bridge::run(g, packet, ebp - 0x58, &mut alloca),
            );
            t.b.set_call_top(None);
            calls += t.log_entries();
        }
        assert!(calls > 100, "{name}: the stubs ran ({calls} calls)");
    }
}

// ---- PRenderThread::processRenderCommands (0x00615550) ----

/// The job issuer, after the log: sets byte `+1` of its list (`ecx`) to
/// the entry's word 1 ("jobs issued").
const ISSUE_BODY: [u8; 6] = [
    0x8b, 0x50, 0x04, // mov edx, [eax+4]
    0x88, 0x51, 0x01, // mov [ecx+1], dl
];

#[test]
fn process_render_commands_matches_the_original() {
    const SYM: u32 = 0x0061_5550;
    for (name, mut t) in Twins::all() {
        for (sym, this, n, pops) in [
            (rc::LOCK_ENTER, true, 0, 0),
            (rc::LOCK_LEAVE, true, 0, 0),
            (rc::CONTEXT_CTOR, true, 0, 0),
            (rc::ATEXIT, false, 1, 0),
            (rc::PASS_CTOR, true, 1, 1),
            (rc::PASS_DTOR, true, 0, 0),
            (rc::VECTOR_CTOR, false, 5, 5),
            (rc::VECTOR_DTOR, false, 4, 4),
            (rc::SET_PASS, true, 1, 1),
            (rc::RUN_MESH, false, 2, 0),
            (rc::RUN_MESH2, false, 2, 0),
            (rc::SET_TARGETS, true, 3, 3),
            (rc::RELEASE, true, 0, 0),
            (rc::OTHER_COMMAND, true, 3, 3),
        ] {
            t.stub(sym, this, n, pops);
        }
        let flush = t.stub(rc::FLUSH, true, 1, 1);
        let blocks_n = t.stub(rc::BLOCK_COUNT, true, 1, 1);
        let (issue, issue_cell) = t.stub_custom(rc::ISSUE_JOBS, true, 4, 4, 8, &ISSUE_BODY);
        let at = t.rebase(rc::ISSUE_JOBS);
        let mut j = vec![0xe9];
        j.extend(issue.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let (add, _) = t.stub_code(0x7c00_0000, false, 2, 2);
        let slot = t.rebase(rc::INTERLOCKED_ADD);
        t.w32(slot, add);
        let (callback, _) = t.stub_code(0x7c00_0001, false, 1, 0);
        let guard = t.rebase(rc::CONTEXT_GUARD);
        let mut rng = Rng(0x0061_5550);
        let mut calls = 0;
        for case in 0..60_u32 {
            let log = t.log;
            t.w32(log, 0);
            t.w32(guard, u32::from(case % 4 != 0));
            let thread = t.zeros(0x300);
            t.fill(&mut rng, thread, 0x300);
            t.w32(thread + 0x158, rng.below(2));
            t.w32(thread + 0x208, rng.below(2));
            // the commands
            let n = if case % 10 == 0 { 0 } else { 1 + rng.below(12) };
            let first = t.zeros((rc::COMMAND * n) as usize + 8);
            t.fill(&mut rng, first, rc::COMMAND * n);
            let mut counts = Vec::new();
            for c in 0..n {
                let cmd = first + rc::COMMAND * c;
                let kind = [0, 1, 1, 2, 2, 3, 0x2c, 5, 0x2b, 0x2d][rng.below(10) as usize];
                t.w32(cmd, kind);
                match kind {
                    1 | 2 => {
                        let mesh = if rng.below(5) == 0 {
                            0
                        } else {
                            let segments = t.zeros(0x40);
                            for s in 0..4 {
                                t.w32(segments + 16 * s + 0xc, rng.below(2));
                            }
                            let mesh = t.zeros(0x20);
                            t.w32(mesh + 0x14, segments);
                            mesh
                        };
                        t.w32(cmd + 0xc, mesh);
                        t.write(cmd + 0x24, &(rng.below(4) as u16).to_le_bytes());
                        let cnt = rng.below(4);
                        if mesh != 0 {
                            counts.push(cnt);
                        }
                        let blocks = if rng.below(3) == 0 {
                            0
                        } else {
                            let b = t.zeros(0x100);
                            for i in 0..4 {
                                t.w32(b + 0x40 * i + 0x38, rng.below(2));
                                t.write(b + 0x40 * i + 0x14, &[rng.below(2) as u8]);
                            }
                            b
                        };
                        t.w32(cmd + 0x10, blocks);
                        let index = t.zeros(0x40);
                        t.w32(index + 0x34, rng.below(2));
                        t.write(index + 0xe, &[rng.below(2) as u8]);
                        t.w32(cmd + 0x3c, index);
                    }
                    0x2c => t.w32(cmd + 8, callback),
                    _ => {}
                }
                // the reference counts to release (types 1 and 2 at
                // +0x38, 3 at +0x1c)
                let list = if rng.below(3) == 0 {
                    0
                } else {
                    let k = rng.below(4);
                    let l = t.zeros(4 + 4 * k as usize);
                    t.w32(l, k);
                    for i in 0..k {
                        let counter = t.zeros(4);
                        t.w32(l + 4 + 4 * i, counter);
                    }
                    l
                };
                t.w32(cmd + if kind == 3 { 0x1c } else { 0x38 }, list);
            }
            let end = first + rc::COMMAND * n;
            t.results(blocks_n, &counts);
            // the issuer: flags and returns; the flushes: increasing
            // stops up to the end
            let issues: Vec<u32> = (0..32)
                .flat_map(|_| [rng.next(), u32::from(rng.below(3) != 0)])
                .collect();
            t.results(issue_cell, &issues);
            let mut stops = Vec::new();
            let mut at = 0;
            for _ in 0..24 {
                at = (at + rng.below(4)).min(n);
                stops.push(first + rc::COMMAND * at);
            }
            // past the list, the flushes return the end
            let mut tail = stops.clone();
            tail.resize(64, end);
            t.results(flush, &tail);
            t.check_framed(
                &format!("{name} case {case} ({n} commands)"),
                SYM,
                Conv::Thiscall(thread),
                &[first, end],
                rc::FRAME,
                // the SEH record and the cookie
                0x10,
                |g, ebp| rc::bridge::process(g, thread, first, end, ebp),
            );
            calls += t.log_entries();
        }
        assert!(calls > 300, "{name}: the stubs ran ({calls} calls)");
    }
}

// ---- 0x00a0ff50: the legacy glow's Gaussian tables ----

/// A random word that is not a signalling NaN (the emulator keeps those
/// signalling where the CPU quiets them).
fn no_snan(rng: &mut Rng) -> u32 {
    let w = rng.next();
    if ffx_bridge::x87::is_snan(w) {
        w | 0x0040_0000
    } else {
        w
    }
}

#[test]
fn gaussian_tables_match_the_original() {
    const SYM: u32 = 0x00a0_ff50;
    const GLOW: usize = 0xd00;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x6a05_5150);
        let spreads: Vec<u32> = [
            1.0_f32,
            0.65,
            0.0,
            -0.0,
            2.5,
            -1.25,
            0.1,
            3.0e38,
            -7.75,
            1.0e-3,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ]
        .iter()
        .map(|f| f.to_bits())
        .chain([
            0x7fc0_1234,
            0xffc0_0000,
            0x0000_0001,
            0x8000_0003,
            0x0012_3456,
        ])
        .collect();
        let sizes: &[(u32, u32)] = &[
            (1280, 720),
            (1920, 1080),
            (0, 0),
            (1, 1),
            (2, 3),
            (641, 359),
            (0xffff_ffff, 0x8000_0000),
            (7, 0x1234_5679),
            (0x0100_0001, 0x7fff_ffff),
        ];
        let mut case = 0;
        for &n in &[7_u32, 5, 3] {
            for (i, &spread) in spreads.iter().enumerate() {
                let (w, h) = sizes[(i + n as usize) % sizes.len()];
                let glow = t.zeros(GLOW);
                let junk: Vec<u32> = (0..GLOW / 4).map(|_| no_snan(&mut rng)).collect();
                t.w32s(glow, &junk);
                t.check(
                    &format!("{name} case {case}: n {n}, {w}x{h}, spread {spread:#x}"),
                    SYM,
                    Conv::Thiscall(glow),
                    &[w, h, n, spread],
                    |g| gs::bridge::gaussian_tables(g, glow, w, h, n, spread),
                );
                case += 1;
            }
        }
        // random sizes and spreads
        for _ in 0..12 {
            let n = [3, 5, 7][rng.below(3) as usize];
            let (w, h) = (rng.next(), rng.below(4096));
            let spread = small_float(&mut rng);
            let glow = t.zeros(GLOW);
            let what = format!("{name} case {case}: n {n}, {w}x{h}, spread {spread:#x}");
            t.check(&what, SYM, Conv::Thiscall(glow), &[w, h, n, spread], |g| {
                gs::bridge::gaussian_tables(g, glow, w, h, n, spread)
            });
            case += 1;
        }
        // tap counts it refuses: 5, nothing written
        for n in [
            0_u32,
            1,
            2,
            4,
            6,
            8,
            9,
            11,
            0xffff_fffd,
            0xffff_ffff,
            0x8000_0007,
        ] {
            let glow = t.zeros(GLOW);
            let one = 1.0_f32.to_bits();
            t.check(
                &format!("{name} taps {n:#x}"),
                SYM,
                Conv::Thiscall(glow),
                &[640, 360, n, one],
                |g| gs::bridge::gaussian_tables(g, glow, 640, 360, n, one),
            );
        }
    }
}

// ---- PBitmapFontText::createMeshGeometry (0x00a29770) ----

/// The map stub's body: `[out] =` the result entry's second word.
const MAP_BODY: [u8; 9] = [
    0x8b, 0x50, 0x04, // mov edx, [eax+4]
    0x8b, 0x4c, 0x24, 0x04, // mov ecx, [esp+4] (out)
    0x89, 0x11, // mov [ecx], edx
];

/// `c` as UTF-8.
fn utf8(c: u32) -> Vec<u8> {
    char::from_u32(c).unwrap().to_string().into_bytes()
}

#[test]
fn create_mesh_geometry_matches_the_original() {
    const SYM: u32 = 0x00a2_9770;
    const CODES: [u32; 7] = [0x41, 0x42, 0x43, 0x44, 0x45, 0xe9, 0x3042];
    for (name, mut t) in Twins::all() {
        let at = t.rebase(ft::MAP);
        let (addr, map) = t.stub_custom(ft::MAP, true, 1, 1, 8, &MAP_BODY);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        let unmap = t.stub(ft::UNMAP, true, 2, 2);
        t.stub(ft::UNMAP_STEP, true, 1, 1);
        t.stub(ft::PRINTF, false, 3, 0);
        let mut rng = Rng(0x0a29_7700);
        let mut calls = 0;
        let mut quads = 0;
        for case in 0..60_u32 {
            // the font: glyphs for CODES (some missing), kerning pairs
            let font = t.zeros(0x40);
            let holder = t.zeros(0x40);
            t.w32(holder + 0x30, font);
            let n = 4 + rng.below(4);
            let glyphs = t.zeros(0x30 * 8);
            for k in 0..n {
                let g = glyphs + 0x30 * k;
                let mut w = vec![
                    CODES[(k + case) as usize % CODES.len()],
                    1 + rng.below(6),
                    rng.below(16),
                    (rng.below(512) as f32).to_bits(),
                    (rng.below(512) as f32).to_bits(),
                ];
                // sizes, offsets, advances: on the 1/4096 grid, -0, a
                // quiet NaN now and then, or with every mantissa bit (sums
                // that round)
                w.extend((0..6).map(|_| {
                    match rng.below(6) {
                        0 => (f32::from_bits(0x3f80_0000 | (rng.next() & 0x007f_ffff))
                            * [-3.0, 0.001, 37.0][rng.below(3) as usize])
                            .to_bits(),
                        1 if rng.below(5) == 0 => 0x7fc0_0000 | rng.below(0x40_0000),
                        1 => 0x8000_0000,
                        _ => small_float(&mut rng),
                    }
                }));
                w.push(rng.below(2));
                t.w32s(g, &w);
            }
            let kern = t.zeros(0x300);
            let pairs: Vec<u32> = (0..64)
                .flat_map(|_| {
                    let few = if rng.below(2) == 0 { 3 } else { 7 };
                    [
                        CODES[rng.below(few) as usize],
                        (rng.below(200) as i32 - 100) as u32,
                        rng.next(),
                    ]
                })
                .collect();
            t.w32s(kern, &pairs);
            let texture = t.zeros(0x40);
            let sizes = [256, 1024, 0, 1, 0x8000_0010, 0xffff_ffff, 300];
            t.w32(texture + 0x1c, sizes[case as usize % 7]);
            t.w32(texture + 0x20, sizes[(case as usize + 2) % 7]);
            let top = if case % 9 == 4 { 0x8000_0000 } else { 0 };
            t.w32s(
                font,
                &[
                    0,
                    0,
                    (1.0 + rng.below(64) as f32 / 8.0).to_bits(),
                    0,
                    n | top,
                ],
            );
            t.w32(font + 0x14, glyphs);
            t.w32(font + 0x1c, kern);
            t.w32(font + 0x20, texture);

            // the string: glyphs, newlines, characters the font lacks
            let len = rng.below(24);
            let mut s = Vec::new();
            let mut chars = 0;
            for _ in 0..len {
                let c = match rng.below(8) {
                    0 => 0x0a,
                    1 => 0x7a,
                    _ => CODES[rng.below(7) as usize],
                };
                s.extend(utf8(c));
                chars += 1;
            }
            s.push(0);
            let string = t.alloc(&s);
            let count = match case % 5 {
                0 => chars,
                1 => chars / 2,
                2 => chars + 3,
                3 => 1,
                _ => {
                    if case % 2 == 0 {
                        0
                    } else {
                        chars
                    }
                }
            };

            // the text and its mesh
            let text = t.zeros(0x200);
            let junk: Vec<u32> = (0..0x80).map(|_| no_snan(&mut rng)).collect();
            t.w32s(text, &junk);
            let mesh = t.zeros(0xc0);
            t.fill(&mut rng, mesh, 0xc0);
            for s in 0..3 {
                t.w32(mesh + 0x40 * s + 0x10, rng.below(4));
                t.w32(mesh + 0x40 * s + 0x18, rng.below(4) | (rng.below(2) << 31));
            }
            let colours = case % 3 != 0;
            t.w32(text + 0x64, mesh);
            t.w32(text + 0x158, holder);
            t.w32(text + 0x160, string);
            t.w32(text + 0x170, small_float(&mut rng));
            t.write(text + 0x174, &[u8::from(colours)]);
            let pos = t.zeros(0x20 * 40);
            let uv = t.zeros(0x20 * 40);
            let col = t.zeros(0x40 * 40);
            let maps: &[u32] = match case % 11 {
                3 => &[0x8000_0013, pos],
                5 => &[0, pos, 9, uv],
                7 => &[0, pos, 0, uv, 0xb, col],
                _ => &[0, pos, 0, uv, 0, col],
            };
            t.results(map, maps);
            let unmaps: &[u32] = match case % 13 {
                4 => &[0x15],
                6 => &[0, 0x16],
                8 => &[0, 0, 0x17],
                _ => &[],
            };
            t.results(unmap, unmaps);
            let before = t.log_entries();
            t.check_framed_ranges(
                &format!("{name} case {case}: {count} of {chars} characters"),
                SYM,
                Conv::Thiscall(text),
                &[count],
                ft::FRAME,
                &[
                    (ft::MAP_POS, 4),
                    (ft::MAP_UV, 4),
                    (ft::MAP_COL, 4),
                    (ft::CURSOR, 4),
                    (ft::GLYPH, 0x30),
                    (ft::KEY, 0x30),
                ],
                |g, ebp| ft::bridge::create_mesh_geometry(g, text, count, ebp),
            );
            calls += t.log_entries() - before;
            quads += Mem::read::<u32>(&t.b, text + 0x70) / 6;
        }
        assert!(calls > 150, "{name}: the stubs ran ({calls} calls)");
        assert!(quads > 100, "{name}: {quads} quads");
    }
}

// ---- PLegacyGlowGPUBase_render (0x00a1de20) ----

/// A stub body that appends `n` words to the stream at `[cur]`: from the
/// pointer in stack argument `arg` when it is set, else from `addr`.
fn snap_body(cur: u32, arg: Option<u32>, addr: u32, n: u32) -> Vec<u8> {
    let mut c = vec![0x56, 0x57]; // push esi; push edi
    if let Some(k) = arg {
        c.extend([0x8b, 0x74, 0x24, (12 + 4 * k) as u8]); // mov esi, [esp+12+4k]
    } else {
        c.push(0xbe); // mov esi, addr
        c.extend(addr.to_le_bytes());
    }
    c.extend([0x8b, 0x3d]); // mov edi, [cur]
    c.extend(cur.to_le_bytes());
    c.push(0xb9); // mov ecx, n
    c.extend(n.to_le_bytes());
    c.extend([0xf3, 0xa5]); // rep movsd
    c.extend([0x89, 0x3d]); // mov [cur], edi
    c.extend(cur.to_le_bytes());
    c.extend([0x5f, 0x5e]); // pop edi; pop esi
    c
}

impl Twins {
    /// [`Twins::stub_custom`] installed over the original at `sym`.
    fn stub_with(&mut self, sym: u32, this: bool, n: u32, pops: u32, body: &[u8]) -> u32 {
        let at = self.rebase(sym);
        let (addr, cell) = self.stub_custom(sym, this, n, pops, 4, body);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
        cell
    }
}

#[test]
fn legacy_glow_render_matches_the_original() {
    const SYM: u32 = 0x00a1_de20;
    for (name, mut t) in Twins::all() {
        // the parameter buffer and the renderer stay put: the draw stub
        // snapshots the buffer's 16 words and the pass at every draw, the
        // texture stub its record, the color-target stub its surface
        let buffer = t.zeros(0x100);
        let renderer = t.zeros(0x200);
        let cur = t.zeros(4);
        let stream = t.zeros(0x4000);
        let mut draw_snap = snap_body(cur, None, buffer + 4, 16);
        draw_snap.extend(snap_body(cur, None, renderer + 0x48, 1));
        let stubs = [
            t.stub_with(gl::SET_TEXTURE, true, 2, 2, &snap_body(cur, Some(1), 0, 4)),
            t.stub(gl::SET_SAMPLER, true, 2, 2),
            t.stub_with(gl::SET_COLOR, true, 2, 2, &snap_body(cur, Some(0), 0, 4)),
            t.stub(gl::SET_DEPTH, true, 1, 1),
            t.stub(gl::BEGIN_SCENE, true, 1, 1),
            t.stub_with(gl::RENDER_MESH, true, 4, 4, &draw_snap),
            t.stub(gl::END_SCENE, true, 1, 1),
        ];
        t.stub(gl::QUEUE18, true, 3, 3);
        let mut rng = Rng(0x00a1_de20);
        let mut draws = 0;
        let mut errors = 0;
        for case in 0..80_u32 {
            t.w32(cur, stream);
            let log = t.log;
            t.w32(log, 0); // the log holds 512 calls
            let glow = t.zeros(0xa0);
            let junk: Vec<u32> = (0..0x28).map(|_| no_snan(&mut rng)).collect();
            t.w32s(glow, &junk);
            let levels = [3, 2, 1, 0, 4, 0xffff_ffff, 3, 3][case as usize % 8];
            t.w32(glow + 0x50, levels);
            let spread = if case % 4 == 1 {
                (f32::from_bits(0x3f80_0000 | (rng.next() & 0x007f_ffff)) * 0.37).to_bits()
            } else {
                small_float(&mut rng)
            };
            t.w32(glow + 0x58, spread);
            t.w32(glow + 0x5c, small_float(&mut rng));
            let mesh = t.zeros(0x20);
            t.fill(&mut rng, mesh, 0x20);
            t.w32(glow + 0x48, mesh);
            let holder = t.zeros(0x10);
            t.w32(holder + 4, buffer);
            t.w32(glow + 0x4c, holder);
            // the buffer: its size, then stale words
            t.fill(&mut rng, buffer, 0x100);
            // the size: roomy, too small, or exactly the gain's or the scale's end
            t.w32(
                buffer,
                [0x80, 0x80, 0x80, 0x30, 0x44, 0x24][(case % 6) as usize],
            );
            t.fill(&mut rng, renderer, 0x200);
            // parameters: +0x60 texture, +0x64 sampler, +0x68 gain, +0x6c
            // scale, +0x70 size (offset, size with stale flag bits)
            let lens = [16_u16, 16, 16, 12, 8, 4, 16, 0x110];
            for (k, (field, offset)) in [
                (0x60, 0x44),
                (0x64, 4),
                (0x68, 0x34),
                (0x6c, 0x14),
                (0x70, 0x24),
            ]
            .into_iter()
            .enumerate()
            {
                let present = (case >> k) & 3 != 1;
                let desc = t.zeros(0x10);
                t.fill(&mut rng, desc, 0x10);
                let offset: u16 = if case % 23 == 7 + k as u32 { 2 } else { offset };
                let len = lens[rng.below(if case % 7 == 3 { 8 } else { 7 }) as usize]
                    | ((rng.below(8) as u16) << 13);
                t.write(desc + 8, &offset.to_le_bytes());
                t.write(desc + 0xa, &len.to_le_bytes());
                t.w32(glow + field, if present { desc } else { 0 });
            }
            // render targets {texture, unbound byte}: +0x78[3], +0x84[3]
            for k in 0..6 {
                let tex = t.zeros(0x30);
                t.fill(&mut rng, tex, 0x30);
                let target = t.zeros(8);
                t.w32(target, tex);
                let unbound = k >= 3 && rng.below(4) == 0;
                t.w32(target + 4, u32::from(unbound) | (rng.next() & 0xffff_ff00));
                t.w32(glow + 0x78 + 4 * k, target);
            }
            let target = if case % 3 == 0 { 0 } else { rng.next() };
            let source = rng.next();
            // an error from one callee now and then
            for &cell in &stubs {
                t.results(cell, &[]);
            }
            if case % 5 == 4 {
                let which = rng.below(stubs.len() as u32) as usize;
                // the sampler parameter's call, at its first level
                let which = if case == 24 || case == 64 { 1 } else { which };
                let mut list = vec![0; 1 + rng.below(if which == 1 { 3 } else { 8 }) as usize];
                *list.last_mut().unwrap() = 0x10 + which as u32;
                t.results(stubs[which], &list);
            }
            let mut ranges: Vec<(u32, u32)> = gl::RECORDS
                .iter()
                .chain(&gl::SAMPLERS)
                .chain(&gl::SURFACES)
                .chain(&[
                    gl::SCALE_DOWN,
                    gl::ONES,
                    gl::SCALE_UP,
                    gl::SIZE_DOWN,
                    gl::SIZE_UP,
                ])
                .map(|&o| (o, 16))
                .collect();
            ranges.push((gl::MASK, 4));
            t.check_framed_ranges(
                &format!("{name} case {case}: {levels} levels"),
                SYM,
                Conv::Thiscall(glow),
                &[renderer, target, source, rng.next(), rng.next()],
                gl::FRAME,
                &ranges,
                |g, ebp| gl::bridge::render(g, glow, renderer, target, source, ebp),
            );
            let end: u32 = Mem::read(&t.b, cur);
            draws += (end - stream) / 4;
            errors += u32::from(case % 5 == 4);
        }
        assert!(draws > 3000, "{name}: {draws} words snapshotted");
        assert!(errors > 10);
    }
}
