//! Continuation round 4: the four full-screen mesh builders
//! (`ffx_prender::p6_quads`) against their originals.

use ffx_oracle::Conv;
use ffx_prender::p6_quads as qd;

use super::small::run_framed;
use super::{Rng, Twins, object};

impl Twins {
    /// The logging part of a recording stub (as [`Twins::stub_code`]):
    /// `tag`, `ecx` when `this`, `n` stack words; leaves `eax` at the
    /// entry.
    pub(super) fn log_code(&self, tag: u32, this: bool, n: u32) -> Vec<u8> {
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
        c
    }

    /// A recording stub at `sym` (as [`Twins::stub_code`]: `tag` the
    /// address, `ecx` when `this`, `n` stack words, pops `pops`) whose
    /// result is the next word of the table `cursor` points at (the
    /// cursor then moves on by a word). Returns the cursor's address.
    pub(super) fn seq_stub(&mut self, sym: u32, this: bool, n: u32, pops: u32) -> u32 {
        self.seq_stub_with(sym, this, n, pops, &[])
    }

    /// [`Twins::seq_stub`] running `extra` after the logging (`eax` the
    /// log entry, see [`deref_code`]).
    pub(super) fn seq_stub_with(
        &mut self,
        sym: u32,
        this: bool,
        n: u32,
        pops: u32,
        extra: &[u8],
    ) -> u32 {
        let (addr, cursor) = self.seq_code_with(sym, this, n, pops, extra);
        self.jump(sym, addr);
        cursor
    }

    /// [`Twins::seq_stub`]'s code, not placed: returns its address and
    /// the cursor.
    pub(super) fn seq_code(&mut self, tag: u32, this: bool, n: u32, pops: u32) -> (u32, u32) {
        self.seq_code_with(tag, this, n, pops, &[])
    }

    fn seq_code_with(
        &mut self,
        tag: u32,
        this: bool,
        n: u32,
        pops: u32,
        extra: &[u8],
    ) -> (u32, u32) {
        let cursor = self.zeros(4);
        let mut c = self.log_code(tag, this, n);
        c.extend(extra);
        c.extend([0x8b, 0x15]);
        c.extend(cursor.to_le_bytes()); // mov edx, [cursor]
        c.extend([0x8b, 0x02]); // mov eax, [edx]
        c.extend([0x83, 0x05]);
        c.extend(cursor.to_le_bytes());
        c.push(0x04); // add dword [cursor], 4
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        (self.code(&c), cursor)
    }

    /// Points `cursor` at a fresh table of `words`.
    pub(super) fn seq_set(&mut self, cursor: u32, words: &[u32]) {
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let table = self.alloc(&bytes);
        self.w32(cursor, table);
    }
}

/// Code for [`Twins::seq_stub_with`]: logs `words` words at the pointer
/// in stack argument `arg` into the log entry after `n` logged words
/// (clobbers `ecx` and `edx`).
pub(super) fn deref_code(arg: u32, n: u32, words: u32) -> Vec<u8> {
    assert!(n + words <= 6);
    let mut c = vec![0x8b, 0x4c, 0x24, (4 + 4 * arg) as u8]; // mov ecx, [esp+4+4arg]
    for w in 0..words {
        c.extend([0x8b, 0x51, (4 * w) as u8]); // mov edx, [ecx+4w]
        c.extend([0x89, 0x50, (8 + 4 * (n + w)) as u8]); // mov [eax+8+4(n+w)], edx
    }
    c
}

/// A stream: random words, the semantic record at `+0xc`, the vertex
/// buffer the map stub hands out at `+0x3c`; the size words `+0`, `+4`
/// and `+0x38` agree or not, the version `+0x10` wraps at `+0x18` or not.
fn stream(t: &mut Twins, rng: &mut Rng, at: u32) {
    for k in 0..16 {
        t.w32(at + 4 * k, rng.next());
    }
    let record = object(t, rng, 3, false);
    t.w32(at + 0xc, record);
    let buffer = object(t, rng, 18, false);
    t.w32(at + 0x3c, buffer);
    let (a, b) = (rng.below(0x40), rng.below(8));
    t.w32(at, a);
    t.w32(at + 4, b);
    if rng.chance(2) {
        t.w32(at + 0x38, a * b);
    }
    let v = rng.below(4);
    t.w32(at + 0x10, v);
    let wrap = if rng.chance(2) { v + 1 } else { rng.below(6) };
    t.w32(at + 0x18, wrap | (rng.next() & 0x8000_0000));
}

/// A segment: random words, `n` or another count of streams at `+0x2c`.
fn segment(t: &mut Twins, rng: &mut Rng, n: u32) -> u32 {
    let seg = object(t, rng, 0x1b, false);
    let streams = t.zeros(0xc0);
    for k in 0..3 {
        stream(t, rng, streams + 0x40 * k);
    }
    t.w32(seg + 0x2c, streams);
    let count = if rng.chance(2) { n } else { rng.below(5) };
    t.w32(seg + 0x28, count);
    seg
}

#[test]
fn fullscreen_meshes_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1101);
        let create = t.seq_stub(qd::CLUSTER_CREATE, true, 3, 3);
        let list = t.seq_stub(qd::ALLOCATE_INSTANCE_LIST, true, 3, 3);
        let malloc = t.seq_stub(qd::MALLOC_ALIGNED, false, 2, 0);
        let bound = t.seq_stub(qd::IS_BOUND, true, 0, 0);
        let unmap = t.seq_stub(qd::UNMAP, true, 2, 2);
        t.stub(qd::SEGMENTS_CONSTRUCT, false, 2, 0, 0x5e90_0001);
        t.stub(qd::SEGMENTS_DESTROY, false, 2, 0, 0x5e90_0002);
        t.stub(qd::STREAM_CTOR, true, 0, 0, 0x5e90_0003);
        t.stub(qd::STREAMS_SET, true, 2, 2, 0x5e90_0004);
        t.stub(qd::STREAM_FORMAT, true, 3, 3, 0x5e90_0005);
        t.stub(qd::RELEASE, true, 0, 0, 0x5e90_0006);
        t.stub(qd::INSTANCE_CTOR, true, 1, 1, 0x5e90_0007);
        // map(stream; &slot): the slot gets the stream's +0x3c
        let mut c = t.log_code(qd::MAP, true, 1);
        c.extend([0x8b, 0x41, 0x3c]); // mov eax, [ecx+0x3c]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let map = t.code(&c);
        t.jump(qd::MAP, map);
        for q in &qd::FULLSCREEN_MESHES {
            let n = q.streams.len() as u32;
            for i in 0..24u32 {
                let cluster = object(&mut t, &mut rng, 4, false);
                let mesh = object(&mut t, &mut rng, 2, false);
                let old = segment(&mut t, &mut rng, n);
                let fresh = segment(&mut t, &mut rng, n);
                t.w32(mesh + 4, old);
                let mut mallocs = vec![];
                match i % 4 {
                    0 => t.w32(mesh, 1),
                    1 => {
                        // a new segment; the old data owned or a view
                        t.w32(mesh, rng.below(4) | (rng.next() & 0x8000_0000));
                        mallocs.push(fresh);
                    }
                    2 => {
                        // the allocation fails
                        t.w32(mesh, 0x8000_0001);
                        mallocs.push(0);
                    }
                    _ => {
                        // the block is the old data
                        t.w32(mesh, rng.below(3));
                        t.w32(mesh + 4, fresh);
                        mallocs.push(fresh);
                    }
                }
                let null = rng.chance(3);
                let block = object(&mut t, &mut rng, 0x30, null);
                mallocs.push(block);
                t.seq_set(malloc, &mallocs);
                t.seq_set(create, &[mesh]);
                let bounds: Vec<u32> = (0..3)
                    .map(|_| (rng.next() & 0xffff_ff00) | u32::from(rng.chance(2)))
                    .collect();
                t.seq_set(bound, &bounds);
                let unmaps: Vec<u32> = (0..3)
                    .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                    .collect();
                t.seq_set(unmap, &unmaps);
                let lst = match i % 3 {
                    0 => 0,
                    _ => {
                        let l = object(&mut t, &mut rng, 0x10, false);
                        if i % 6 == 1 {
                            t.w32(l + 0x34, 0);
                        }
                        l
                    }
                };
                t.seq_set(list, &[lst]);
                t.a.clear_dirty();
                t.b.clear_dirty();
                run_framed(
                    &mut t,
                    &format!("{name} mesh {:#x} #{i}", q.addr),
                    q.addr,
                    Conv::Cdecl,
                    &[cluster],
                    0x20 + 8 * n,
                    8 * n as usize,
                    |g, ebp| qd::bridge::build(g, cluster, ebp - 0x20 - 8 * n, q),
                );
                assert!(t.log_entries() >= 2 + 3 * n, "{name} #{i}: the calls");
            }
        }
    }
}
