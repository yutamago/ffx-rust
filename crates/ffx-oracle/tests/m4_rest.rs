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
use ffx_prender::m4r_meshlist as ml;
use ffx_prender::m4r_modnet as mn;
use ffx_prender::m4r_nodecapture as nc;

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
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
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
