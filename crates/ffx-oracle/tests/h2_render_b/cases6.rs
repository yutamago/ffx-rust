//! Round 4 batch 3 cases (`ffx_prender::h2_more7`), written by hand: one
//! per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more7::bridge as h7;

const FREE: u32 = 0x0042_fb20;
const PRINTF: u32 = 0x0043_52f0;

/// The scoped lock, the holder's constructor and `atexit`.
const LOCKS: [(u32, u32, u32, bool); 4] = [
    (0x0046_3d50, 0, 0, true),
    (0x0046_3e90, 0, 0, true),
    (0x004f_ee70, 0, 0, true),
    (0x0094_9207, 1, 0, false),
];

/// A region of the pool's second half (0x200 bytes each, k < 12).
fn area(e: &Env, k: u32) -> u32 {
    e.pool + POOL_PTRS + 0x200 * k
}

/// A count word at `array` (owned or borrowed); more than one element
/// keeps its storage at `data`. Returns the storage.
fn sharray(e: &mut Env, rng: &mut Rng, array: u32, n: u32, data: u32) -> u32 {
    e.w32(array, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    match n {
        0 => 0,
        1 => array + 4,
        _ => {
            e.w32(array + 4, data);
            data
        }
    }
}

/// An object (or null, one in `null` times) at `obj` whose `Release`
/// pops one word; returns what goes in its slot.
fn releasable(e: &mut Env, rng: &mut Rng, obj: u32, null: u32) -> u32 {
    if rng.chance(null) {
        return 0;
    }
    let r = e.ret_word(rng);
    let f = e.handler(1, r);
    e.w32(obj, obj + 0x20);
    e.w32(obj + 0x28, f);
    obj
}

/// Handler code made once per `tag`, in both oracles at the same
/// address.
fn once(e: &mut Env, tag: u32, make: impl FnOnce(&mut Env) -> Vec<u8>) -> u32 {
    if let Some(&(at, _)) = e.writers.get(&tag) {
        return at;
    }
    let c = make(e);
    let at = e.a.alloc_code(&c).unwrap();
    assert_eq!(e.b.alloc_code(&c).unwrap(), at);
    e.writers.insert(tag, (at, 0));
    at
}

/// `~PMeshInstance`: the two buffer arrays (none to three, objects or
/// null), the other arrays and the name.
fn prep_mesh_dtor(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    for (k, off) in [0x70u32, 0x78].into_iter().enumerate() {
        let n = rng.below(4);
        e.w32(a[0] + off, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
        let data = area(e, k as u32);
        e.w32(a[0] + off + 4, data);
        for j in 0..n {
            let p = if rng.chance(3) { 0 } else { e.ptr(rng) };
            e.w32(data + 4 * j, p);
        }
    }
}

/// The stream releases: the flag, the entries (none to three, with
/// releasable objects in words 0, 1 and 3), the spin wait.
fn prep_release_streams(e: &mut Env, rng: &mut Rng, a: &mut [u32], array: u32, flag: u32) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let f = rng.next() | u32::from(!rng.chance(5));
    e.w32(a[0] + flag, f);
    let n = rng.below(4);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + array, n, data);
    for k in 0..n {
        for (j, word) in [0u32, 4, 0xc].into_iter().enumerate() {
            let obj = area(e, 1) + 0x40 * (3 * k + j as u32);
            let v = releasable(e, rng, obj, 3);
            e.w32(s + 20 * k + word, v);
        }
    }
}

/// `0x00564cb0`: the entries (one to three, some empty, a staging copy
/// or not), `GetDesc` (the CPU flags from the scratch), the render
/// context's `Unmap` and `CopyResource`, the element getter's result.
fn prep_unmap_copy(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let n = 1 + rng.below(3);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + 0x18, n, data);
    let flags = area(e, 2);
    let buf = area(e, 3);
    let desc = once(e, 0xf00d_3001, |_| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
        c.extend([0x8b, 0x15]);
        c.extend(flags.to_le_bytes()); // mov edx, [flags]
        c.extend([0x89, 0x50, 0x0c]); // mov [eax+0xc], edx
        c.extend([0xc2, 0x08, 0x00]);
        c
    });
    e.w32(flags, if rng.chance(2) { 0x2_0000 } else { 0x1_0000 });
    e.w32(buf, buf + 0x40);
    e.w32(buf + 0x40 + 4 * 10, desc);
    for k in 0..n {
        let b = if rng.chance(4) { 0 } else { buf };
        e.w32(s + 20 * k, b);
        let st = if rng.chance(2) { 0 } else { e.ptr(rng) };
        e.w32(s + 20 * k + 0xc, st);
    }
    a[1] = rng.below(n);
    let g = e.rebase(0x00c9_628c);
    let v = rng.next();
    e.w32(g, v);
    let ctx = area(e, 4);
    let at = e.rebase(0x00c9_4f70);
    e.w32(at, ctx);
    e.w32(ctx, ctx + 0x40);
    let r = e.ret_word(rng);
    let f = e.handler(3, r);
    e.w32(ctx + 0x40 + 4 * 15, f);
    let r = e.ret_word(rng);
    let f = e.handler(3, r);
    e.w32(ctx + 0x40 + 4 * 47, f);
    let p = if rng.chance(4) {
        e.ptr(rng)
    } else {
        s + 20 * a[1]
    };
    e.set_ret(0x0055_e0f0, p);
}

/// `0x0059b140`: the input layout (null sometimes), the program's
/// streams and layout (null sometimes), up to five streams with data
/// blocks bound or not, the device's feature level and context.
fn prep_vertex_streams(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x200);
    let il = if rng.chance(6) { 0 } else { e.ptr(rng) };
    e.w32(a[0] + 0x4e8, il);
    let dev = area(e, 0);
    a[1] = dev + 0x1f0;
    e.w32(a[1], dev);
    e.w32(dev + 0xb4, if rng.chance(2) { 0xa000 } else { 0xb000 });
    let ctx = area(e, 1);
    e.w32(dev + 0x70, ctx);
    e.w32(ctx, ctx + 0x40);
    for (slot, pops) in [(18, 6), (17, 2)] {
        let r = e.ret_word(rng);
        let f = e.handler(pops, r);
        e.w32(ctx + 0x40 + 4 * slot, f);
    }
    a[2] = area(e, 2);
    let base = area(e, 3);
    let layout = area(e, 4);
    e.w32(a[2] + 0x2c, if rng.chance(8) { 0 } else { base });
    e.w32(a[2] + 0x14, if rng.chance(8) { 0 } else { layout });
    let n = rng.below(6);
    e.w32(
        layout + 0x10,
        n | if rng.chance(3) { 0x8000_0000 } else { 0 },
    );
    let offs = layout + 0x40;
    e.w32(layout + 0x14, offs);
    for i in 0..n {
        let o = 0x10 * rng.below(8);
        e.write(offs + 2 * i, &(o as u16).to_le_bytes());
        let stream = base + o;
        let block = e.ptr(rng);
        e.w32(stream, block);
        let p = e.ptr(rng);
        e.w32(stream + 4, p);
    }
    if rng.chance(4) {
        e.set_ret(0x0056_2da0, 0x100);
    } else {
        e.set_ret(0x0056_2da0, 1);
    }
}

/// A render context at `ctx` (vtable at `ctx + 0x40`) with handlers for
/// `slots` (word index, words popped).
fn context(e: &mut Env, rng: &mut Rng, ctx: u32, slots: &[(u32, u32)]) {
    e.w32(ctx, ctx + 0x40);
    for &(slot, pops) in slots {
        let r = e.ret_word(rng);
        let f = e.handler(pops, r);
        e.w32(ctx + 0x40 + 4 * slot, f);
    }
}

/// `0x0058e200`: the device and its context (`Map` storing a pointer
/// into the pool), the flags, the colour, the target index.
fn prep_clear_draw(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x100);
    a[1] = area(e, 0);
    let ctx = area(e, 1);
    e.w32(a[1] + 0x70, ctx);
    let slots = [
        (15, 3),
        (69, 4),
        (23, 4),
        (17, 2),
        (11, 4),
        (9, 4),
        (7, 4),
        (16, 4),
        (18, 6),
        (24, 2),
        (13, 3),
    ];
    context(e, rng, ctx, &slots);
    let out = area(e, 2) + 4 * rng.below(8);
    let map = e.writer(0x50, 6, 5, 0, out);
    e.w32(ctx + 0x40 + 4 * 14, map);
    a[2] = rng.below(16);
    a[6] = rng.below(4);
}

/// `0x0058e3d0`: the viewport (full or not), the render targets (none
/// to three, some null), the depth-stencil view, the context's clears.
fn prep_clear(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x100);
    let full = rng.chance(2);
    for (x, y) in [(0x132c, 0x1334), (0x1330, 0x1338)] {
        let v = rng.below(4);
        e.w32(a[0] + x, v);
        let w = if full || rng.chance(2) { v } else { v + 1 };
        e.w32(a[0] + y, w);
    }
    for off in [0x1324, 0x1328] {
        let v = u32::from(!(full || rng.chance(2)));
        e.w32(a[0] + off, v);
    }
    let n = rng.below(4);
    e.w32(a[0] + 0x2fc, n);
    for i in 0..n {
        let rt = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(a[0] + 0x2b8 + 4 * i, rt);
    }
    let dsv = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(a[0] + 0x2d8, dsv);
    let ctx = area(e, 1);
    e.w32(a[0] + 0x70, ctx);
    context(e, rng, ctx, &[(50, 3), (53, 5)]);
    a[1] = rng.below(16);
}

pub const CASES: &[Case] = &[
    Case {
        name: "Phyre_PRendering_PMeshInstance_dtor_PMeshInstance",
        addr: 0x004f_2c50,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[
            (0x0049_fa10, 0, 0, true),
            (FREE, 1, 0, false),
            (0x004f_4fc0, 2, 0, false),
        ],
        prep: prep_mesh_dtor,
        port: |g, a| {
            h7::mesh_instance_dtor(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PVertexBufferD3D11_release_005646d0",
        addr: 0x0056_46d0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            (0x0056_4630, 0, 0, true),
            (PRINTF, 2, 0, false),
        ],
        prep: |e, r, a| prep_release_streams(e, r, a, 0x18, 0x34),
        port: |g, a| {
            let f = g.frame();
            h7::release_streams(g, a[0], f, (0x0056_4630, 0x18, 0x34, 0x00b2_ece8))
        },
        frame: 4,
        gap: 0xc,
        writes: &[],
    },
    Case {
        name: "PIndexBufferD3D11_release_005648c0",
        addr: 0x0056_48c0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            (0x0056_4680, 0, 0, true),
            (PRINTF, 2, 0, false),
        ],
        prep: |e, r, a| prep_release_streams(e, r, a, 0x14, 0x30),
        port: |g, a| {
            let f = g.frame();
            h7::release_streams(g, a[0], f, (0x0056_4680, 0x14, 0x30, 0x00b2_ebd0))
        },
        frame: 4,
        gap: 0xc,
        writes: &[],
    },
    Case {
        name: "PIndexBufferD3D11_unmapCopy_00564cb0",
        addr: 0x0056_4cb0,
        conv: K::T,
        kinds: "pvv",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            LOCKS[2],
            LOCKS[3],
            (0x0055_e0f0, 1, 1, true),
        ],
        prep: prep_unmap_copy,
        port: |g, a| {
            let f = g.frame();
            h7::unmap_copy(g, a[0], a[1], f)
        },
        frame: 0x24,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_bindVertexStreams_0059b140",
        addr: 0x0059_b140,
        conv: K::T,
        kinds: "ppp",
        out: O::U8,
        callees: &[(0x0056_2da0, 0, 0, true), (0x0059_3af0, 1, 1, true)],
        prep: prep_vertex_streams,
        port: |g, a| {
            let f = g.frame();
            u32::from(h7::bind_vertex_streams(g, a[0], a[1], a[2], f))
        },
        frame: 0x190,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_clearByDraw_0058e200",
        addr: 0x0058_e200,
        conv: K::T,
        kinds: "ppvpfbv",
        out: O::U32,
        callees: &[
            (0x0059_c950, 0, 0, true),
            (0x0058_d4e0, 0, 0, true),
            (0x0058_c6c0, 1, 1, true),
        ],
        prep: prep_clear_draw,
        port: |g, a| {
            let f = g.frame();
            h7::clear_by_draw(g, a[0], a[1], a[2], a[3], a[4], a[5], a[6], f)
        },
        frame: 0x30,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_clear_0058e3d0",
        addr: 0x0058_e3d0,
        conv: K::T,
        kinds: "pv",
        out: O::U8,
        callees: &[(0x0058_e200, 6, 6, true)],
        prep: prep_clear,
        port: |g, a| {
            let f = g.frame();
            u32::from(h7::clear(g, a[0], a[1], f))
        },
        frame: 0x10,
        gap: 0,
        writes: &[],
    },
];
