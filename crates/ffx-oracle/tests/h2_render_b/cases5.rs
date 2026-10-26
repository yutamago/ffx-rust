//! Round 4 batch 2 cases (`ffx_prender::h2_more6`, one `Bind()`), written
//! by hand: one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more6::bridge as h6;

const FREE: u32 = 0x0042_fb20;
const MALLOC: u32 = 0x0042_fb80;
const MEMSET: u32 = 0x0094_964c;
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

/// A COM result: success (0 or 1) or a failure code.
fn hresult(rng: &mut Rng) -> u32 {
    [0, 0, 1, 0x8000_4005][rng.below(4) as usize]
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

/// The render-thread holder's guard (built or not) and its device (at
/// `0x00c94f6c`) or context (`0x00c94f70`) object whose vtable is at
/// `vt`.
fn holder_obj(e: &mut Env, rng: &mut Rng, global: u32, obj: u32, vt: u32) {
    let g = e.rebase(0x00c9_628c);
    let v = rng.next();
    e.w32(g, v);
    let at = e.rebase(global);
    e.w32(at, obj);
    e.w32(obj, vt);
}

/// `0x004f8b90`: the mesh's counts (often the current ones), the old
/// storage (null or not), allocations that fail sometimes.
fn prep_size_arrays(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    a[1] = area(e, 0);
    for (mesh_off, this_off) in [(0, 0x24), (0x18, 8)] {
        let cur = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
        e.w32(a[0] + this_off, cur);
        let n = match rng.below(4) {
            0 => cur,
            1 => cur & 0x7fff_ffff,
            _ => rng.below(4),
        };
        e.w32(
            a[1] + mesh_off,
            n | if rng.chance(4) { 0x8000_0000 } else { 0 },
        );
        let old = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(a[0] + this_off + 4, old);
    }
    let src = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(a[1] + 0x1c, src);
    if rng.chance(5) {
        e.set_ret(MALLOC, 0);
    } else if rng.chance(4) {
        // the new storage at the old matrices' address
        let old = e.r32(a[0] + 0xc);
        e.set_ret(MALLOC, old);
    }
}

/// `0x005954f0`: the device's `CreateBuffer` (method 3), sizes 0 or not,
/// the CPU copies present or not.
fn prep_dynamic_buffers(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[1] = area(e, 0);
    let f = e.handler(4, hresult(rng));
    e.w32(a[1], a[1] + 0x40);
    e.w32(a[1] + 0x40 + 4 * 3, f);
    for k in [2, 3] {
        if rng.chance(3) {
            a[k] = 0;
        }
    }
    for off in [0x20, 0x24] {
        if rng.chance(2) {
            e.w32(a[0] + off, 0);
        }
    }
}

/// `0x004e4ec0`: the slot count (0..4), the slot memory (null or not),
/// one to three parameter blocks with tables, parameters and indices.
fn prep_assign_slots(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = area(e, 0);
    let ty = area(e, 0) + 0x100;
    e.w32(a[0] + 0x2c, ty);
    let stride = 0x40;
    e.w32(ty + 0x1c, stride);
    let count = rng.below(4);
    e.w32(a[0] + 0x3c, count);
    let blocks = area(e, 1);
    e.w32(a[0] + 0x30, blocks);
    let n = rng.below(4);
    let slots = e.rebase(0x00ca_5fec);
    e.w32(slots, n);
    // the flag byte's neighbours
    let flag = e.rebase(0x00ca_5fe8);
    let w = rng.next();
    e.w32(flag, w);
    let mem = if rng.chance(6) { 0 } else { area(e, 2) };
    e.w32(a[0] + 0x38, mem);
    for b in 0..count {
        let blk = blocks + stride * b;
        e.w32(
            blk + 0x14,
            rng.below(3) | if rng.chance(2) { 0x8000_0000 } else { 0 },
        );
        let t = match rng.below(5) {
            0 => 0,
            1 => mem,
            _ => e.ptr(rng),
        };
        e.w32(blk + 0x18, t);
        let m = rng.below(3);
        e.w32(blk + 0xc, m | if rng.chance(3) { 0x8000_0000 } else { 0 });
        let params = area(e, 3) + 0x80 * b;
        e.w32(blk + 0x10, params);
        for k in 0..m {
            let p = area(e, 4) + 0x20 * (3 * b + k);
            e.w32(params + 0x28 * k, p);
            let idx = if rng.chance(3) {
                u32::MAX
            } else {
                rng.below(n.max(1))
            };
            e.w32(p + 0xc, idx);
        }
    }
}

/// The stream-buffer creators: the width (0 sometimes), the entries
/// (none to three), the device's `CreateBuffer`.
fn prep_stream_buffers(e: &mut Env, rng: &mut Rng, a: &mut [u32], width: u32, array: u32) {
    a[0] = e.pool + 4 * rng.below(0x400);
    if rng.chance(5) {
        e.w32(a[0] + width, 0);
    }
    let n = rng.below(4);
    let data = area(e, 0);
    sharray(e, rng, a[0] + array, n, data);
    let dev = area(e, 1);
    holder_obj(e, rng, 0x00c9_4f6c, dev, dev + 0x40);
    let f = e.handler(4, hresult(rng));
    e.w32(dev + 0x40 + 4 * 3, f);
}

/// `0x00564ab0`: the entries at `+0x18` (one to three, some empty, a
/// staging word or not), the buffer's `GetDesc` stand-in (stores the CPU
/// flags word from the scratch), the render context's `Unmap`.
fn prep_unmap_stream(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let n = 1 + rng.below(3);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + 0x18, n, data);
    let flags = area(e, 2);
    let buf = area(e, 3);
    let desc = once(e, 0xf00d_3000, |_| {
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
    let ctx = area(e, 4);
    holder_obj(e, rng, 0x00c9_4f70, ctx, ctx + 0x40);
    let r = e.ret_word(rng);
    let f = e.handler(3, r);
    e.w32(ctx + 0x40 + 4 * 15, f);
    // the element getter's stand-in result: an entry (or any word)
    let p = if rng.chance(4) {
        e.ptr(rng)
    } else {
        s + 20 * a[1]
    };
    e.set_ret(0x0055_e0f0, p);
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

/// `0x0058f710`: the flag bit, the resource, the entry count the getter
/// gives (0..3), the entries, the device's `CreateShaderResourceView`.
fn prep_create_views(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    e.write(a[0] + 0x64, &[if rng.chance(4) { 0 } else { 8 }]);
    let res = e.ptr(rng);
    e.w32(a[0], res);
    e.w32(a[0] + 0x58, rng.below(3));
    let n = rng.below(4);
    e.set_ret(0x0059_3bb0, n);
    let k = 1 + rng.below(3);
    let data = area(e, 0);
    sharray(e, rng, a[0] + 0x24, k, data);
    let dev = area(e, 1);
    holder_obj(e, rng, 0x00c9_4f6c, dev, dev + 0x40);
    let f = e.handler(4, if rng.chance(6) { 0x8000_0001 } else { 0 });
    e.w32(dev + 0x40 + 4 * 7, f);
}

pub const CASES: &[Case] = &[
    Case {
        name: "PMeshInstance_ctor_004f27e0",
        addr: 0x004f_27e0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[(0x004f_8b90, 1, 1, true)],
        prep: no_prep,
        port: |g, a| h6::mesh_instance_ctor(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PMeshInstance_sizeArrays_004f8b90",
        addr: 0x004f_8b90,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            (MALLOC, 2, 0, false),
            (0x004f_6380, 2, 0, false),
            (0x004f_4fc0, 2, 0, false),
            (FREE, 1, 0, false),
            (0x0045_3340, 1, 1, true),
        ],
        prep: prep_size_arrays,
        port: |g, a| h6::size_arrays(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Bind_00502100",
        addr: 0x0050_2100,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_7fc0, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_c140, 0, 0, true),
            (0x0057_52a0, 6, 6, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| super::cases3::prep_bind(e, r, &ffx_prender::h2_bind::B_00502100),
        port: |g, _| hb::bind(g, &ffx_prender::h2_bind::B_00502100),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_createDynamicBuffers_005954f0",
        addr: 0x0059_54f0,
        conv: K::T,
        kinds: "ppvv",
        out: O::U32,
        callees: &[(MALLOC, 2, 0, false)],
        prep: prep_dynamic_buffers,
        port: |g, a| {
            let f = g.frame();
            h6::create_dynamic_buffers(g, a[0], a[1], a[2], a[3], f)
        },
        frame: 0x34,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PShaderD3D11_ctor_00583f20",
        addr: 0x0058_3f20,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(MEMSET, 3, 0, false)],
        prep: |e, r, a| a[0] = e.pool + 4 * r.below(0x300),
        port: |g, a| h6::shader_ctor(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PParameterBlocks_assignSlots_004e4ec0",
        addr: 0x004e_4ec0,
        conv: K::C,
        kinds: "p",
        out: O::U32,
        callees: &[(FREE, 1, 0, false)],
        prep: prep_assign_slots,
        port: |g, a| h6::assign_slots(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Destroy_004ea2d0",
        addr: 0x004e_a2d0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(FREE, 1, 0, false), (0x004e_c310, 2, 0, false)],
        prep: no_prep,
        port: |g, a| {
            h6::destroy_4ea2d0(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PVertexBufferD3D11_create_00561a20",
        addr: 0x0056_1a20,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            LOCKS[2],
            LOCKS[3],
            (PRINTF, 2, 0, false),
        ],
        prep: |e, r, a| prep_stream_buffers(e, r, a, 0x38, 0x18),
        port: |g, a| {
            let f = g.frame();
            h6::create_stream_buffers(g, a[0], f, (0x38, 0x18, 1, 0x34, 0x00b2_ec98))
        },
        frame: 0x24,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PIndexBufferD3D11_create_00561b70",
        addr: 0x0056_1b70,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            LOCKS[2],
            LOCKS[3],
            (PRINTF, 2, 0, false),
        ],
        prep: |e, r, a| prep_stream_buffers(e, r, a, 0x34, 0x14),
        port: |g, a| {
            let f = g.frame();
            h6::create_stream_buffers(g, a[0], f, (0x34, 0x14, 2, 0x30, 0x00b2_eb80))
        },
        frame: 0x24,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PVertexBufferD3D11_unmap_00564ab0",
        addr: 0x0056_4ab0,
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
        prep: prep_unmap_stream,
        port: |g, a| {
            let f = g.frame();
            h6::unmap_stream(g, a[0], a[1], f)
        },
        frame: 0x20,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PTexture2DD3D11_createViews_0058f710",
        addr: 0x0058_f710,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            LOCKS[0],
            LOCKS[1],
            LOCKS[2],
            LOCKS[3],
            (0x0059_3bb0, 0, 0, true),
        ],
        prep: prep_create_views,
        port: |g, a| {
            let f = g.frame();
            h6::create_views(g, a[0], f)
        },
        frame: 0x24,
        gap: 0xc,
        writes: &[],
    },
];
