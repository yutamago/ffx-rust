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
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::unreadable_literal
)]

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_pother_b::{m4r_dynblocks, m4r_mesh};

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
        c.extend([0x8b, 0x00]); // mov eax, [eax]
        c.extend([0x83, 0x05]);
        c.extend(cell.to_le_bytes());
        c.push(0x04); // add dword [cell], 4
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
        all.resize(64, 0);
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
