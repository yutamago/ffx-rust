//! Last round batch 1 cases (`ffx_prender::h2_more12`), written by hand:
//! one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more12::bridge as h12;

/// The `PSharray<PMeshInstanceSegmentStreamBinding const*>` descriptor's
/// statics: the guard bits (each step done or not), its name (set or
/// not), the class's name global, the annotation list head (empty,
/// pointing at itself, or not), the flags word and the member words the
/// declaration writes.
fn prep_sharray_declare(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let d = e.rebase(0x00ca_79a8);
    let g = rng.below(16) | if rng.chance(3) { rng.next() & !0xf } else { 0 };
    e.w32(d + 0x94, g);
    let n = if rng.chance(3) { e.ptr(rng) } else { 0 };
    e.w32(d + 0x18, n);
    let ng = e.rebase(0x00ca_7e84);
    let v = if rng.chance(2) { e.ptr(rng) } else { 0 };
    e.w32(ng, v);
    let head = if rng.chance(2) { d + 0x108 } else { e.ptr(rng) };
    e.w32(d + 0x108, head);
    for off in [0x90, 0x98, 0xac, 0xc4, 0xcc, 0xd0, 0xd4, 0x104, 0x11c] {
        let w = rng.next();
        e.w32(d + off, w);
    }
    if rng.chance(8) {
        a[0] = 0;
    }
}

/// Where `prep_streams` builds its objects (the pool's second half).
const R_THIS: u32 = POOL_PTRS;
const R_STREAMS: u32 = POOL_PTRS + 0x400;
const R_EXTRA: u32 = POOL_PTRS + 0x600;
const R_RECORDS: u32 = POOL_PTRS + 0x800;

/// A `PSharray` of slot records at `array` (one inline, or several
/// elsewhere: the `k`-th records block; flag bit 31 now and then) and
/// the entry's slot index at `slot` (0 for the inline one).
fn records(e: &mut Env, rng: &mut Rng, array: u32, slot: u32, k: u32) {
    let n = [1, 1, 2, 5][rng.below(4) as usize] | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(array, n);
    let i = if n & 0x7fff_ffff > 1 {
        let at = e.pool + R_RECORDS + 0x80 * k;
        e.w32(array + 4, at);
        rng.below(5)
    } else {
        0
    };
    e.w32(slot, i);
}

/// `0x0058c0d0`'s renderer, stream entries and extra entry: the buffer
/// pairs' used bytes, capacities (often just enough or one short),
/// buffers (0 now and then), the frame index, mapped bases (sometimes
/// the negated offsets, so a mapped address is 0), the entries' sizes
/// (0 now and then), slots and record arrays.
fn prep_streams(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + R_THIS;
    a[0] = this;
    a[1] = [0, 1, 1, 2, 0x8000_0001][rng.below(5) as usize];
    a[2] = e.pool + R_STREAMS;
    a[3] = rng.below(8);
    let n = a[3];
    a[4] = if rng.chance(4) { 0 } else { e.pool + R_EXTRA };
    let frame = rng.below(2);
    e.w32(this + 0x31c, frame);
    let mut need = [0u32; 2];
    for k in 0..n.min(8) {
        let s = a[2] + 0x40 * k;
        let size = if rng.chance(4) { 0 } else { rng.below(0x80) };
        e.w32(s + 0x38, size);
        need[0] = need[0].wrapping_add((size + 0xf) & !0xf);
        records(e, rng, s + 0x18, s + 0x10, k);
        let b = rng.next();
        e.w32(s + 0x14, b);
    }
    if a[4] != 0 {
        let x = a[4];
        let size = if rng.chance(4) { 0 } else { rng.below(0x80) };
        e.w32(x + 0x34, size);
        need[1] = (size + 0xf) & !0xf;
        records(e, rng, x + 0x14, x + 0x10, 9);
    }
    for pair in [this + 0x320, this + 0x348] {
        for (k, off) in [0x10, 0x14].into_iter().enumerate() {
            let used = rng.below(0x200);
            e.w32(pair + off, used);
            let cap = match rng.below(4) {
                0 => used.wrapping_add(need[k]),
                1 => used.wrapping_add(need[k]).wrapping_sub(1),
                _ => used.wrapping_add(need[k]).wrapping_add(rng.below(0x100)),
            };
            e.w32(pair + off + 8, cap);
        }
        for off in [0, 4, 8, 0xc] {
            let b = if rng.chance(5) { 0 } else { rng.next() };
            e.w32(pair + off, b);
        }
    }
    for (off, used) in [(0x340, 0x330), (0x344, 0x334)] {
        let base = if rng.chance(4) {
            0u32.wrapping_sub(e.r32(this + used))
        } else {
            rng.next()
        };
        e.w32(this + off, base);
    }
    for off in [0x368, 0x36c, 0x370, 0x374] {
        let v = rng.next();
        e.w32(this + off, v);
    }
}

/// Where `prep_skin` builds its objects.
const S_THIS: u32 = POOL_PTRS;
const S_SKELETON: u32 = POOL_PTRS + 0x100;
const S_ENTRIES: u32 = POOL_PTRS + 0x200;
const S_ARRAYS: u32 = POOL_PTRS + 0x400;
const S_BUFFERS: u32 = POOL_PTRS + 0x500;
const S_DEST: u32 = POOL_PTRS + 0x600;
const S_MATRIX: u32 = POOL_PTRS + 0x1f00;
/// The words `0x00494c20`'s stand-in stores into the frame's array
/// (count, data).
const S_HEAP: u32 = POOL_PTRS + 0x1ff0;

/// `0x004f98b0`'s mesh instance: joints (none, a few, or enough for the
/// heap block), segments with and without joints (some past the 0xc0
/// limit), the per-segment buffer arrays (empty: first call) and their
/// buffers' cursors and counts, the callees' results (errors now and
/// then), the matrices block (given or not). `0x00494c20` is a stand-in
/// that stores a count (owned or borrowed) and a data pointer (or 0)
/// into the frame's array.
fn prep_skin(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let heap = e.pool + S_HEAP;
    e.custom(0x0049_4c20, |_| {
        let mut c = vec![0xa1];
        c.extend(heap.to_le_bytes()); // mov eax, [heap]
        c.extend([0x89, 0x01]); // mov [ecx], eax
        c.push(0xa1);
        c.extend((heap + 4).to_le_bytes()); // mov eax, [heap + 4]
        c.extend([0x89, 0x41, 0x04]); // mov [ecx+4], eax
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        c
    });
    let owned = if rng.chance(2) { 0x8000_0000 } else { 0 };
    e.w32(heap, owned | rng.below(0x100));
    let data = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(heap + 4, data);
    let this = e.pool + S_THIS;
    a[0] = this;
    if rng.chance(2) {
        a[1] = 0;
    }
    let skeleton = e.pool + S_SKELETON;
    e.w32(this, skeleton);
    let joints = match rng.below(6) {
        0 => 0,
        1 => 0x7f + rng.below(3),
        _ => 1 + rng.below(6),
    } | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(skeleton + 0x18, joints);
    let segments = rng.below(5) | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(skeleton, segments);
    let entries = e.pool + S_ENTRIES;
    e.w32(skeleton + 4, entries);
    let binds = e.ptr(rng);
    e.w32(skeleton + 0xc, binds);
    let mut big = false;
    for i in 0..4 {
        let entry = entries + 0x6c * i;
        let n = match rng.below(5) {
            0 => 0,
            1 if !big && rng.chance(4) => {
                big = true;
                0xc1 + rng.below(2)
            }
            _ => rng.below(4),
        } | if rng.chance(5) { 0x8000_0000 } else { 0 };
        e.w32(entry + 8, n);
        // pairs in the pool's pointer part (0xc2 pairs fit)
        let pairs = e.pool + 4 * rng.below(0x700);
        e.w32(entry + 0xc, pairs);
    }
    // the buffer arrays: count words, data
    let first = rng.chance(3);
    let c = if first { 0 } else { 4 } | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(this + 0x70, c);
    let c2 = rng.next();
    e.w32(this + 0x78, c2);
    for (k, off) in [0x74, 0x7c].into_iter().enumerate() {
        let arr = e.pool + S_ARRAYS + 0x40 * k as u32;
        e.w32(this + off, arr);
        for i in 0..4 {
            let b = e.pool + S_BUFFERS + 0x10 * (4 * k as u32 + i);
            e.w32(arr + 4 * i, b);
            let cursor = rng.below(4);
            e.w32(b + 4, cursor);
            let count = [0, 1, 2, 3, 4][rng.below(5) as usize]
                | if rng.chance(4) { 0x8000_0000 } else { 0 };
            e.w32(b + 0xc, count);
        }
    }
    // new constant buffers: buffers too
    let nb = e.pool + S_BUFFERS + 0x10 * rng.below(8);
    e.set_ret(0x004f_49d0, nb);
    for (f, err) in [(0x004f_8880, 6), (0x004f_8460, 6), (0x0059_d320, 8)] {
        let r = if rng.chance(err) {
            1 + rng.below(0xe)
        } else {
            0
        };
        e.set_ret(f, r);
    }
    // the mapped slot: room for 0xc2 matrices when a segment is that big
    let dest = if big {
        e.zeros(0xc2 * 0x40)
    } else {
        e.pool + S_DEST
    };
    e.set_out(0x004f_8460, dest);
    // the transposed matrix: 16 floats (quiet NaNs among them)
    let m = e.pool + S_MATRIX;
    e.set_ret(0x004f_93f0, m);
    for w in 0..16 {
        let v = match rng.below(6) {
            0 => 0x7fc0_0000 | rng.below(0x40_0000),
            1 => 0xffc0_0001,
            _ => ((rng.below(4000) as f32 - 2000.0) / 9.0).to_bits(),
        };
        e.w32(m + 4 * w, v);
    }
}

/// `0x004f98b0`'s port: its frame at the original's `ebp`, the `_alloca`
/// block where the original's lands (`(ebp - 0xd4 - size) & !0xf`), and
/// the port's calls below that block.
fn skin_port(g: &mut G, a: &[u32]) -> u32 {
    let ebp = g.frame() + 0x80 + 0x44;
    let k = h12::skin_block_matrices(g, a[0]);
    let size = k.wrapping_mul(0x40);
    let block = if a[1] == 0 && k <= 0x80 {
        (ebp - 0xd4 - size) & !0xf
    } else {
        ebp - 0xd4
    };
    g.0.set_call_top(Some(block - 0x100));
    h12::upload_skin_matrices(g, a[0], a[1], ebp, block)
}

pub static CASES: &[Case] = &[
    Case {
        name: "Phyre_PSharray_Phyre_PRendering_PMeshInstanceSegmentStreamBinding_const__DeclareClassDescriptor",
        addr: 0x004f_4d60,
        conv: K::C,
        kinds: "n",
        out: O::U32,
        callees: &[
            (0x004f_2160, 0, 0, true),
            (0x0094_9207, 1, 0, false),
            (0x0057_5ef0, 6, 7, true),
            (0x0043_7fc0, 0, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0057_5840, 3, 3, true),
            (0x0057_5c10, 1, 1, true),
            (0x0043_95b0, 1, 0, false),
            (0x0043_c140, 0, 0, true),
        ],
        prep: prep_sharray_declare,
        port: |g, a| h12::declare(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_placeDynamicStreams_0058c0d0",
        addr: 0x0058_c0d0,
        conv: K::T,
        kinds: "pvpvn",
        out: O::U32,
        callees: &[],
        prep: prep_streams,
        port: |g, a| h12::place_streams(g, a[0], a[1], a[2], a[3], a[4]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PMeshInstance_uploadSkinMatrices_004f98b0",
        addr: 0x004f_98b0,
        conv: K::T,
        kinds: "pn",
        out: O::U32,
        callees: &[
            (0x004f_8880, 1, 1, true),
            (0x004f_49d0, 0, 0, false),
            (0x004f_6ef0, 1, 1, true),
            (0x0058_44e0, 0, 0, true),
            (0x004f_8460, 1, 1, true),
            (0x004c_9da0, 2, 2, true),
            (0x004f_93f0, 2, 0, false),
            (0x0059_d320, 2, 2, true),
            (0x0058_59f0, 0, 0, true),
            (0x0042_fb20, 1, 0, false),
        ],
        prep: prep_skin,
        port: skin_port,
        // the two matrices callees fill (`ebp - 0xc4` up to `ebp - 0x44`)
        frame: 0x80,
        gap: 0x44,
        writes: &[(0x004f_8460, 0)],
    },
];
