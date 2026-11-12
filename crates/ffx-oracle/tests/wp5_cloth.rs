//! M5 finish, WP5: `batchSetClothVertexBuffers` (`ffx_prender::wp5_hot::
//! cloth`) against the original machine code, its callees replaced in
//! both oracles alike by recording stand-ins (each call's tag, `ecx` and
//! words go to a log in memory, so the diff checks the calls too), on the
//! image alone and over a snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_possible_truncation)] // small test indices to u8

use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::wp5_hot::cloth as c;

/// The bridge's view of an oracle: its memory, and its emulator for the
/// callees.
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

fn twins() -> Vec<(&'static str, Oracle, Oracle)> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return vec![];
    };
    let exe = dir.join("FFX.exe");
    let mut out = vec![(
        "image",
        Oracle::load(&exe).unwrap(),
        Oracle::load(&exe).unwrap(),
    )];
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT").filter(|s| !s.is_empty()) {
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        out.push((
            "snapshot",
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ));
    }
    for (_, a, b) in &mut out {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(20_000_000);
    }
    out
}

/// `len` bytes of junk at the same address in both.
fn junk(a: &mut Oracle, b: &mut Oracle, len: usize, seed: u8) -> u32 {
    let bytes: Vec<u8> = (0..len)
        .map(|i| {
            seed.wrapping_mul(31)
                .wrapping_add((i as u8).wrapping_mul(7))
                | 1
        })
        .collect();
    let p = a.alloc_bytes(&bytes).unwrap();
    assert_eq!(
        b.alloc_bytes(&bytes).unwrap(),
        p,
        "both oracles allocate alike"
    );
    p
}

/// `len` zeroed bytes at the same address in both.
fn zeros(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let p = a.alloc_bytes(&vec![0; len]).unwrap();
    assert_eq!(b.alloc_bytes(&vec![0; len]).unwrap(), p);
    p
}

/// Writes `w` at `at` in both.
fn put(a: &mut Oracle, b: &mut Oracle, at: u32, w: u32) {
    Mem::write(a, at, w);
    Mem::write(b, at, w);
}

fn same(label: &str, a: &mut Oracle, b: &mut Oracle) {
    let d = a.diff(b).unwrap();
    assert!(
        d.is_empty(),
        "{label}: {} bytes differ, first {:x?}",
        d.len(),
        &d[..d.len().min(8)]
    );
    a.clear_dirty();
    b.clear_dirty();
}

/// The call log the stand-ins write: a count, then 8 words an entry (tag,
/// `ecx`, up to six stack words).
struct Log(u32);

impl Log {
    fn new(a: &mut Oracle, b: &mut Oracle) -> Self {
        Self(zeros(a, b, 0x4004))
    }

    /// A stand-in at the original `sym` in both: logs `n` words, writes
    /// `out` (two words) through its first argument when given, pops
    /// `pops` words, returns `ret`.
    #[allow(clippy::too_many_arguments)] // a stand-in's whole shape
    fn stub(
        &self,
        a: &mut Oracle,
        b: &mut Oracle,
        sym: u32,
        n: u8,
        pops: u8,
        ret: u32,
        out: Option<(u32, u32)>,
    ) {
        let log = self.0;
        let mut code = vec![0xa1];
        code.extend(log.to_le_bytes()); // mov eax, [log]
        code.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        code.push(0x05);
        code.extend((log + 4).to_le_bytes()); // add eax, log + 4
        code.extend([0xc7, 0x00]);
        code.extend(sym.to_le_bytes()); // mov dword [eax], sym
        code.extend([0x89, 0x48, 0x04]); // mov [eax + 4], ecx
        for i in 0..n {
            code.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp + 4 + 4i]
            code.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax + 8 + 4i], edx
        }
        code.extend([0xff, 0x05]);
        code.extend(log.to_le_bytes()); // inc dword [log]
        if let Some((w0, w1)) = out {
            code.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp + 4]
            code.extend([0xc7, 0x02]);
            code.extend(w0.to_le_bytes()); // mov dword [edx], w0
            code.extend([0xc7, 0x42, 0x04]);
            code.extend(w1.to_le_bytes()); // mov dword [edx + 4], w1
        }
        code.push(0xb8);
        code.extend(ret.to_le_bytes()); // mov eax, ret
        if pops == 0 {
            code.push(0xc3);
        } else {
            code.push(0xc2);
            code.extend(u16::from(pops * 4).to_le_bytes());
        }
        let at_code = a.alloc_code(&code).unwrap();
        assert_eq!(b.alloc_code(&code).unwrap(), at_code);
        let at = a.rebase(sym);
        let mut jmp = vec![0xe9];
        jmp.extend(at_code.wrapping_sub(at + 5).to_le_bytes());
        a.write(at, &jmp).unwrap();
        b.write(at, &jmp).unwrap();
    }

    fn entries(&self, o: &Oracle) -> u32 {
        Mem::read(o, self.0)
    }
}

/// One cloth scene: characters in table slots 0..=3 (slot 1 empty, slot 2
/// not ready), each ready one with two instances whose four segments take
/// every branch (a `-1` half on variants 2 and 3, no cloth object, no
/// parallel word, the segment array's view or none); streams with and
/// without the cloth semantic; the instance list's count (1) sending
/// instance 0 to the buffer path and instance 1 to the mapped copy; views
/// found or not and unmaps done or not by the variant's parity.
fn cloth_scene(a: &mut Oracle, b: &mut Oracle, log: &Log, variant: u32) -> (u32, u32) {
    let semantic = a.rebase(c::CLOTH_SEMANTIC);
    let array = zeros(a, b, 0x40);
    let src = junk(a, b, 0x40, 9);
    let dst = zeros(a, b, 0x40);
    log.stub(a, b, c::SEGMENT_ARRAY, 0, 0, array, None);
    log.stub(
        a,
        b,
        c::FIND_VIEW,
        1,
        1,
        if variant % 2 == 0 { 0x5150 } else { 0 },
        None,
    );
    log.stub(a, b, c::BIND_VIEW, 2, 2, 0, None);
    // the map's and unmap's `&out` is the original's stack local and the
    // port's frame: not logged (the words that are logged and the copy
    // into `dst` show they ran alike)
    log.stub(a, b, c::MAP_STREAM, 0, 1, 0, Some((dst, 0x18)));
    log.stub(a, b, c::UNMAP_STREAM, 1, 2, variant % 2, None);
    log.stub(a, b, c::SET_POSITIONS, 2, 2, 0, None);
    log.stub(a, b, c::SET_RANGE, 3, 3, 0, None);
    // the segment array: index 0 a view object, index 1 none
    let target = junk(a, b, 0x10, 3);
    let view_of = zeros(a, b, 8);
    put(a, b, view_of, 0x7777);
    put(a, b, view_of + 4, target);
    put(a, b, array, view_of);
    put(a, b, array + 4, 0);
    let table = a.rebase(c::TABLE);
    for slot in 0..4u32 {
        let entry = junk(a, b, 0x40, 20 + slot as u8);
        put(a, b, table + 4 * slot, if slot == 1 { 0 } else { entry });
        let list = zeros(a, b, 0x10);
        let parallel = zeros(a, b, 0x10);
        put(a, b, entry, list);
        put(a, b, entry + 4, parallel);
        // variant 5: slot 0 without its parallel list, slot 2 ready but
        // busy (+8 set), slot 3 with no instances
        put(a, b, entry + 8, u32::from(variant == 5 && slot == 2));
        put(
            a,
            b,
            entry + 0xc,
            if slot == 2 && variant != 5 { 1 } else { 2 },
        );
        if variant == 5 && slot == 0 {
            put(a, b, entry + 4, 0);
        }
        put(
            a,
            b,
            entry + 0x14,
            if variant == 5 && slot == 3 { 0 } else { 2 },
        );
        let instances = zeros(a, b, 8);
        let owners = zeros(a, b, 8);
        put(a, b, list + 4, instances);
        put(a, b, list + 8, 0x8000_0001);
        put(a, b, list + 0xc, owners);
        for k in 0..2u32 {
            let owner = junk(a, b, 0x20, 40 + k as u8);
            put(a, b, owners + 4 * k, owner);
            let instance = zeros(a, b, 0x20);
            put(a, b, instances + 4 * k, instance);
            let mesh = zeros(a, b, 8);
            put(a, b, instance, mesh);
            put(a, b, instance + 0x10, 0x4242 + k);
            let segments = junk(a, b, 0x6c * 4, 60 + k as u8);
            // variant 4: instance 1 of slot 0 has no segments
            let segs = if variant == 4 && slot == 0 && k == 1 {
                0x8000_0000
            } else {
                0x8000_0004
            };
            put(a, b, mesh, segs);
            put(a, b, mesh + 4, segments);
            let cloths = zeros(a, b, 0x18);
            let words = zeros(a, b, 0x18);
            // variant 4: slot 0's instance 0 without a cloth table
            let no_table = variant == 4 && slot == 0 && k == 0;
            put(
                a,
                b,
                entry + 0x30 + 4 * k,
                if no_table { 0 } else { cloths },
            );
            put(a, b, parallel + 4 + 4 * k, words);
            for s in 0..4u32 {
                let seg = segments + 0x6c * s;
                put(a, b, seg, s % 2);
                let halves = if s == 3 && variant > 1 {
                    0x0000_ffff
                } else {
                    0x0001_0002
                };
                put(a, b, seg + 0x20, halves);
                put(a, b, seg + 0x24, 0x0003_0004);
                let streams = junk(a, b, 0x80, 80 + s as u8);
                // variant 4: segment 0 without streams
                let n_streams = if variant == 4 && s == 0 {
                    0x8000_0000
                } else {
                    0x8000_0002
                };
                put(a, b, seg + 0x28, n_streams);
                put(a, b, seg + 0x2c, streams);
                for n in 0..2u32 {
                    let stream = streams + 0x40 * n;
                    let items = zeros(a, b, 0x24);
                    put(a, b, stream + 8, 3);
                    put(a, b, stream + 0xc, items);
                    put(a, b, stream + 0x10, 4 + n);
                    put(a, b, stream + 0x18, 0x8000_0005);
                    for t in 0..3u32 {
                        let w = if n == 0 && t != 1 {
                            semantic
                        } else {
                            semantic + 4
                        };
                        put(a, b, items + 0xc * t + 4, w);
                    }
                }
                let cloth = junk(a, b, 0x50, 100 + s as u8);
                put(a, b, cloth + 0x44, 3 + s);
                put(a, b, cloth + 0x4c, src);
                put(a, b, cloths + 4 + 4 * s, if s == 1 { 0 } else { cloth });
                put(a, b, words + 4 + 4 * s, u32::from(s != 2));
            }
        }
    }
    let first = zeros(a, b, 8);
    put(a, b, first, 0);
    put(a, b, first + 4, 3);
    (first, first + 4)
}

#[test]
fn batch_set_cloth_vertex_buffers_matches_the_original() {
    for (label, mut a, mut b) in twins() {
        for variant in 0..6 {
            let log = Log::new(&mut a, &mut b);
            let (first, last) = cloth_scene(&mut a, &mut b, &log, variant);
            let out = zeros(&mut a, &mut b, 8);
            let at = a.rebase(0x0063_65f0);
            let r = a.call(at, Conv::Cdecl, &[0, first, last]).unwrap();
            let port = c::batch_set_cloth_vertex_buffers(&mut G(&mut b), first, last, out);
            assert_eq!(r.eax, port, "{label} {variant}: eax");
            if variant < 5 {
                assert!(
                    log.entries(&b) > 4,
                    "{label} {variant}: {} calls",
                    log.entries(&b)
                );
            }
            assert_eq!(log.entries(&a), log.entries(&b), "{label} {variant}: calls");
            // the port's `out` is the original's stack local: written in b only
            let mut ob = [0u8; 8];
            b.read_bytes(out, &mut ob);
            a.write(out, &ob).unwrap();
            same(&format!("{label}: cloth {variant}"), &mut a, &mut b);
        }
        // first > last: nothing done, eax = last
        let first = zeros(&mut a, &mut b, 8);
        put(&mut a, &mut b, first, 5);
        put(&mut a, &mut b, first + 4, 4);
        let at = a.rebase(0x0063_65f0);
        let r = a.call(at, Conv::Cdecl, &[0, first, first + 4]).unwrap();
        let out = zeros(&mut a, &mut b, 8);
        let port = c::batch_set_cloth_vertex_buffers(&mut G(&mut b), first, first + 4, out);
        assert_eq!(r.eax, port);
        same(&format!("{label}: cloth none"), &mut a, &mut b);
    }
}
