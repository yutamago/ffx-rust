//! Continuation round batch 3 cases (`ffx_prender::h2_more9`), written by
//! hand: one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more9::bridge as h9;

const FREE: u32 = 0x0042_fb20;
const HOLDER_STUBS: [(u32, u32, u32, bool); 3] = [
    (0x004f_ee70, 0, 0, true),
    (0x0094_9207, 1, 0, false),
    (0x0046_3d50, 0, 0, true),
];

/// An owned or borrowed storage at `count` (its pointer after it), null
/// now and then.
fn storage_words(e: &mut Env, rng: &mut Rng, count: u32) {
    let n = rng.below(4) | [0, 0, 0x8000_0000, 0x4000_0000][rng.below(4) as usize];
    e.w32(count, n);
    let p = if rng.chance(5) { 0 } else { e.ptr(rng) };
    e.w32(count + 4, p);
}

/// The holder's guard word (built or not), its device and context.
fn holder_state(e: &mut Env, rng: &mut Rng, device: u32, context: u32) {
    let g = e.rebase(0x00c9_628c);
    let v = rng.next() & !1 | u32::from(rng.chance(2));
    e.w32(g, v);
    let d = e.rebase(0x00c9_4f6c);
    e.w32(d, device);
    let c = e.rebase(0x00c9_4f70);
    e.w32(c, context);
}

fn prep_pass_dtor(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    for count in [0x24c, 0x234, 0x21c, 0x204, 0x1ec, 0x1d4, 0x1bc, 0x1a4, 0x10] {
        storage_words(e, rng, this + count);
    }
}

fn prep_collection_dtor(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    for at in [0, 0x10] {
        let p = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(this + at, p);
    }
    for count in [0xd4, 0xcc, 0xc4, 0x2c] {
        storage_words(e, rng, this + count);
    }
    let n = [0, 1, 2, 3, 0x8000_0002, 0x8000_0001][rng.below(6) as usize];
    e.w32(this + 0x14, n);
}

/// A `GetDesc` stand-in (stdcall `(this, desc)`): logs `ecx` and its
/// words at `log`, stores the word at `flags` as the description's CPU
/// flags (`desc + 0xc`). Made once per tag.
fn desc_getter(e: &mut Env, tag: u32, flags: u32, log: u32) -> u32 {
    if let Some(&(at, _)) = e.writers.get(&tag) {
        return at;
    }
    let mut c = vec![0x89, 0x0d];
    c.extend(log.to_le_bytes()); // mov [log], ecx
    c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
    c.push(0xa3);
    c.extend((log + 4).to_le_bytes()); // mov [log+4], eax
    c.extend([0x8b, 0x44, 0x24, 0x08]); // mov eax, [esp+8]
    c.push(0xa3);
    c.extend((log + 8).to_le_bytes()); // mov [log+8], eax
    c.extend([0x8b, 0x15]);
    c.extend(flags.to_le_bytes()); // mov edx, [flags]
    c.extend([0x89, 0x50, 0x0c]); // mov [eax+0xc], edx
    c.extend([0xc2, 0x08, 0x00]);
    let at = e.a.alloc_code(&c).unwrap();
    assert_eq!(e.b.alloc_code(&c).unwrap(), at);
    e.writers.insert(tag, (at, 0));
    at
}

/// The dynamic buffer maps: one to three 20-byte entries (owned storage
/// or inline), entry `i` with a buffer whose `GetDesc` reports CPU write
/// (0x10000), CPU read (0x20000) or neither, a staging copy or none; the
/// device's `CreateBuffer`, the context's `Map` (storing the data pointer
/// or null) and `CopyResource`, the map helper's result and pointer.
fn prep_maps(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let n = 1 + rng.below(3);
    e.w32(this + 0x18, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    let data = this + 0x100;
    let base = if n == 1 { this + 0x1c } else { data };
    if n > 1 {
        e.w32(this + 0x1c, data);
    }
    let i = rng.below(n);
    a[1] = i;
    a[2] = this + 0x200;
    let entry = base + 20 * i;
    let buffer = e.pool + 0x400;
    // (never null: the original reads its description first)
    e.w32(entry, buffer);
    let staging = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(entry + 0xc, staging);
    let flags_at = e.pool + 0x6f0;
    let flags = [0x1_0000, 0x2_0000, 0, 0x3_0000, rng.next() & !0x3_0000][rng.below(5) as usize];
    e.w32(flags_at, flags);
    let get = desc_getter(e, 0xf00d_9000, flags_at, e.pool + 0x6f4);
    e.w32(buffer, buffer + 0x20);
    e.w32(buffer + 0x20 + 40, get);
    let device = e.pool + 0x500;
    let context = e.pool + 0x580;
    holder_state(e, rng, device, context);
    e.w32(device, device + 0x20);
    let r = if rng.chance(6) { 0x8007_000e } else { 0 };
    let create = e.handler(4, r);
    e.w32(device + 0x20 + 12, create);
    e.w32(context, context + 0x20);
    let r = if rng.chance(8) {
        rng.next() | 0x8000_0000
    } else {
        rng.below(2)
    };
    let p = if rng.chance(8) { 0 } else { e.ptr(rng) };
    let map = e.writer(10, 6, 5, r, p);
    e.w32(context + 0x20 + 14 * 4, map);
    let copy = e.handler(3, rng.next());
    e.w32(context + 0x20 + 47 * 4, copy);
    if e.cells.contains_key(&0x0056_0f90) {
        let r = if rng.chance(6) { rng.next() | 1 } else { 0 };
        e.set_ret(0x0056_0f90, r);
        let p = e.ptr(rng);
        e.set_out(0x0056_0f90, p);
    }
}

/// The sampler state: made already now and then; the device (none now
/// and then), the global maximum anisotropy and the state's own (below,
/// at or over 1, NaN, huge), the bias, the filter and mode bytes, the
/// border bytes, `CreateSamplerState` (storing the object or null).
fn prep_sampler(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let p = e.pool;
    for (k, sym, words) in [(0u32, 0x0058_aa10u32, 3u8), (1, 0x0058_ab10, 1)] {
        let log = p + 0x1000 + 0x100 * k;
        e.w32(log, 0);
        e.custom(sym, |_| {
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes()); // mov eax, [n]
            c.extend([0xc1, 0xe0, 0x04]); // shl eax, 4
            c.push(0x05);
            c.extend((log + 0x10).to_le_bytes()); // add eax, log + 0x10
            for i in 0..words {
                c.extend([0x8b, 0x4c, 0x24, 4 + 4 * i]); // mov ecx, [esp+4+4i]
                c.extend([0x89, 0x48, 4 * i]); // mov [eax+4i], ecx
            }
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [n]
            c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
            c.extend([0x35, 0x55, 0x00, 0x00, 0x00]); // xor eax, 0x55
            c.push(0xc3);
            c
        });
    }
    let made = if rng.chance(8) { e.ptr(rng) } else { 0 };
    e.w32(this + 0x20, made);
    let device = e.pool + 0x500;
    let d = if rng.chance(8) { 0 } else { device };
    holder_state(e, rng, d, 0);
    let floats = [
        0.5f32.to_bits(),
        1.0f32.to_bits(),
        2.0f32.to_bits(),
        16.0f32.to_bits(),
        3.7f32.to_bits(),
        0x7fc0_0000,
        1e20f32.to_bits(),
        4.3e9f32.to_bits(),
        3e9f32.to_bits(),
        (-3.0f32).to_bits(),
    ];
    let g = e.rebase(0x00c2_5370);
    let max = floats[rng.below(10) as usize];
    e.w32(g, max);
    let own = floats[rng.below(10) as usize];
    e.w32(this + 0xc, own);
    let bias = quiet(rng.next());
    e.w32(this + 8, bias);
    let bytes = rng.next().to_le_bytes();
    e.write(this, &bytes);
    let more = rng.next().to_le_bytes();
    e.write(this + 0x10, &more);
    let flags = rng.next();
    e.w32(this + 0x1c, flags);
    e.w32(device, device + 0x20);
    let r = [0x8007_000e, 1, 0, 0, 0, 0][rng.below(6) as usize];
    let state = e.pool + 0x580;
    let s = if rng.chance(8) { 0 } else { state };
    let create = e.writer(11, 3, 2, r, s);
    e.w32(device + 0x20 + 23 * 4, create);
    e.w32(state, state + 0x20);
    let named = e.handler(4, rng.next());
    e.w32(state + 0x20 + 20, named);
}

/// The UAV binding: the bound target count, the views (count up to 12),
/// the slots already holding what they will get (or not), the kept count
/// equal or not, the initial counts all -1 or one not.
fn prep_uavs(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let views = e.pool + 0x600;
    a[1] = views;
    let count = rng.below(13);
    a[2] = count;
    let initial = e.pool + 0x680;
    a[3] = initial;
    let targets = [0, 0, 1, 2, 4, 8, 9][rng.below(7) as usize];
    e.w32(this + 0x2fc, targets);
    let n = count.min(8);
    let same = rng.chance(2);
    for k in 0..12 {
        let v = if rng.chance(4) { 0 } else { e.ptr(rng) };
        e.w32(views + 4 * k, v);
    }
    for k in 0..8 {
        let at = targets + k;
        let w = if at < n { e.r32(views + 4 * at) } else { 0 };
        let old = if same && !rng.chance(12) {
            w
        } else {
            e.ptr(rng)
        };
        e.w32(this + 0x2dc + 4 * k, old);
        let i = if rng.chance(16) { rng.next() } else { u32::MAX };
        e.w32(initial + 4 * k, i);
    }
    let kept = if rng.chance(3) { rng.below(9) } else { n };
    e.w32(this + 0x300, kept);
    let context = e.pool + 0x700;
    e.w32(this + 0x70, context);
    e.w32(context, context + 0x20);
    // (eight words: a writer storing the context's own vtable word back)
    let set = e.writer(16, 8, 0, rng.next(), context + 0x20);
    e.w32(context + 0x20 + 34 * 4, set);
}

/// The constant buffers: the slot array (one inline slot, or two or
/// three apart), the ring's slot count (0 to 3), the device (none now and
/// then) whose `CreateBuffer` fails now and then and stores a buffer
/// object (`GetDesc`, `SetPrivateData`), the data or none.
fn prep_constants(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    a[1] = rng.below(0x400);
    a[2] = if rng.chance(3) { 0 } else { e.ptr(rng) };
    let n = 1 + rng.below(3);
    e.w32(this + 0xc, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    if n > 1 {
        let data = this + 0x100;
        e.w32(this + 0x10, data);
    }
    let count = rng.below(4).min(n);
    e.set_ret(0x004f_7d50, count);
    let device = e.pool + 0x500;
    let d = if rng.chance(8) { 0 } else { device };
    holder_state(e, rng, d, 0);
    e.w32(device, device + 0x20);
    let buffer = e.pool + 0x580;
    let r = [0x8007_000e, 1, 0, 0, 0, 0, 0, 0][rng.below(8) as usize];
    let create = e.writer(12, 4, 3, r, buffer);
    e.w32(device + 0x20 + 12, create);
    e.w32(buffer, buffer + 0x20);
    let desc = e.handler(2, rng.next());
    e.w32(buffer + 0x20 + 40, desc);
    let named = e.handler(4, rng.next());
    e.w32(buffer + 0x20 + 20, named);
}

/// The pass state: each of the four state objects present or not; the
/// device's three creators (failing now and then, storing an object or
/// null), the objects' `SetPrivateData`, the key word.
fn prep_pass_state(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let key = e.pool + 0x6f0;
    a[1] = key;
    let k = rng.next();
    e.w32(key, k);
    let block = this + 0x18;
    for off in [0x168, 0x16c, 0x170, 0x174] {
        let v = if rng.chance(2) { 0 } else { e.ptr(rng) };
        e.w32(block + off, v);
    }
    let device = e.pool + 0x500;
    holder_state(e, rng, device, 0);
    e.w32(device, device + 0x20);
    let state = e.pool + 0x580;
    e.w32(state, state + 0x20);
    let named = e.handler(4, rng.next());
    e.w32(state + 0x20 + 20, named);
    for (id, slot) in [(13u32, 20u32), (14, 21), (15, 22)] {
        let r = if rng.chance(10) {
            0x8007_000e
        } else {
            rng.below(2)
        };
        let s = if rng.chance(10) { 0 } else { state };
        let f = e.writer(id, 3, 2, r, s);
        e.w32(device + 0x20 + 4 * slot, f);
    }
}

/// The constant-buffer copy: the target buffer (`+4` size, `+8` data,
/// `+0x14` used size), the source buffer (used or not, its slot's data or
/// none), one to four entries (plain, 4x4 or 4x3 matrices: up to three,
/// one ending early on a -1 word; destinations inside or outside),
/// the 16-bit count with junk above.
fn prep_copy_constants(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let target = this + 0x400;
    e.w32(this + 8, target);
    e.w32(
        this + 4,
        if rng.chance(4) { 0x100 } else { 0x200 } | if rng.chance(3) { 0x8000_0000 } else { 0 },
    );
    e.w32(this + 0x14, if rng.chance(8) { 0x80 } else { 0x200 });
    let buf = e.pool + 0x100;
    a[1] = buf;
    let slot = rng.below(4);
    a[2] = slot;
    e.w32(buf + 0x14, u32::from(!rng.chance(10)));
    let data = e.pool + 0x800;
    e.w32(buf + 0x20 + 4 * slot, if rng.chance(10) { 0 } else { data });
    let entries = e.pool + 0x700;
    a[3] = if rng.chance(12) { 0 } else { entries };
    let n = rng.below(5);
    a[5] = n | (rng.next() << 16);
    for k in 0..4 {
        let entry = entries + 0x10 * k;
        let kind = [0x30u8, 0x31, 0x31, 0x30, 0x32][rng.below(5) as usize];
        let step = if kind == 0x30 { 0x30 } else { 0x40 };
        let count = rng.below(4);
        let size = step * count + if rng.chance(4) { rng.below(step) } else { 0 };
        let from = 0x40 * rng.below(0x10);
        e.write(entry, &(from as u16).to_le_bytes());
        let at = [0, 0x40, 0x100, 0x1f0, 0x200, 0x300][rng.below(6) as usize];
        e.w32(entry + 4, at);
        e.w32(entry + 8, size);
        e.write(entry + 0xc, &[kind]);
        if rng.chance(4) {
            let m = rng.below(count.max(1));
            e.w32(data + from + step * m, u32::MAX);
        }
    }
}

/// The joint matrices: the mesh (joint count 0 to 4 with junk in bit
/// 31, the parent table or none), the local matrices or none, the
/// parent indices (0 to 7: earlier, later or the joint itself).
fn prep_joints(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let mesh = this + 0x40;
    e.w32(this, mesh);
    let n = rng.below(5) | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(mesh + 0x18, n);
    let parents = this + 0x80;
    e.w32(mesh + 0x2c, if rng.chance(10) { 0 } else { parents });
    for k in 0..4 {
        let p = rng.below(8);
        e.w32(parents + 4 * k, p);
    }
    // the transform and rotations: return their out pointer, log `ecx`
    // and both words (32 calls at most)
    let p = e.pool;
    for (k, sym) in [(0u32, 0x004c_9eb0u32), (1, 0x004c_a140)] {
        let log = p + 0x1000 + 0x400 * k;
        e.w32(log, 0);
        e.custom(sym, |_| {
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes()); // mov eax, [n]
            c.extend([0xc1, 0xe0, 0x04]); // shl eax, 4
            c.push(0x05);
            c.extend((log + 0x10).to_le_bytes()); // add eax, log + 0x10
            c.extend([0x89, 0x08]); // mov [eax], ecx
            c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x04]); // [eax+4] = out
            c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x08]); // [eax+8] = in
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [n]
            c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
            c.extend([0xc2, 0x08, 0x00]);
            c
        });
    }
    let locals = e.pool + 0x400;
    e.w32(this + 0xc, if rng.chance(10) { 0 } else { locals });
    a[1] = this + 0x200;
}

pub static CASES: &[Case] = &[
    Case {
        name: "PMeshInstance_computeJointMatrix",
        addr: 0x004f_6fc0,
        conv: K::T,
        kinds: "pp",
        out: O::None,
        callees: &[(0x0045_0de0, 4, 4, true)],
        prep: prep_joints,
        port: |g, a| {
            let frame = g.frame();
            h9::compute_joint_matrix(g, a[0], a[1], frame);
            0
        },
        frame: 0x10c,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRenderThread_copyConstantBuffer",
        addr: 0x0059_a650,
        conv: K::T,
        kinds: "ppvpvv",
        out: O::U8,
        callees: &[(0x0094_9256, 3, 0, false)],
        prep: prep_copy_constants,
        port: |g, a| {
            let frame = g.frame();
            u32::from(h9::copy_constant_buffer(
                g, a[0], a[1], a[2], a[3], a[5], frame,
            ))
        },
        frame: 0x44,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PSceneWideParametersKey_assign_004fd2a0",
        addr: 0x004f_d2a0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[(0x0045_3340, 1, 1, true)],
        prep: |e, r, a| {
            // apart, or overlapping by a few words either way
            a[0] = e.pool + POOL_PTRS;
            a[1] = match r.below(4) {
                0 => a[0] + 8,
                1 => a[0] - 8,
                _ => e.pool + 4 * r.below(0x300),
            };
        },
        port: |g, a| h9::scene_key_assign(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderPassStateD3D11_apply_0059ad20",
        addr: 0x0059_ad20,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            HOLDER_STUBS[0],
            HOLDER_STUBS[1],
            HOLDER_STUBS[2],
            (0x0046_3e90, 0, 0, true),
            (0x0058_c6c0, 1, 1, true),
        ],
        prep: prep_pass_state,
        port: |g, a| {
            let frame = g.frame();
            h9::apply_pass_state(g, a[0], a[1], frame)
        },
        frame: 0x30,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PConstantBufferD3D11_create_0058ee20",
        addr: 0x0058_ee20,
        conv: K::T,
        kinds: "pvn",
        out: O::U32,
        callees: &[
            HOLDER_STUBS[0],
            HOLDER_STUBS[1],
            HOLDER_STUBS[2],
            (0x0046_3e90, 0, 0, true),
            (0x004f_7d50, 0, 0, true),
            (0x0042_fb80, 2, 0, false),
        ],
        prep: prep_constants,
        port: |g, a| {
            let frame = g.frame();
            h9::create_constant_buffers(g, a[0], a[1], a[2], frame)
        },
        frame: 0x38,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_setUnorderedAccessViews_0058dbe0",
        addr: 0x0058_dbe0,
        conv: K::T,
        kinds: "pppp",
        out: O::U32,
        callees: &[],
        prep: prep_uavs,
        port: |g, a| h9::set_unordered_access_views(g, a[0], a[1], a[2], a[3]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderPassD3D11_ctor_00583c00",
        addr: 0x0058_3c00,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x004b_5050, 0, 0, true), (0x0058_3e20, 0, 0, true)],
        prep: |e, _, a| a[0] = e.pool + POOL_PTRS,
        port: |g, a| h9::shader_pass_ctor(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderPassD3D11_dtor_005853c0",
        addr: 0x0058_53c0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0059_c950, 0, 0, true), (FREE, 1, 0, false)],
        prep: prep_pass_dtor,
        port: |g, a| {
            h9::shader_pass_dtor(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PSpriteCollection_dtor_005676c0",
        addr: 0x0056_76c0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[
            (0x0044_1240, 1, 1, true),
            (FREE, 1, 0, false),
            (0x0058_5230, 0, 0, true),
            (0x0055_e060, 0, 0, true),
            (0x0043_3900, 0, 0, true),
        ],
        prep: prep_collection_dtor,
        port: |g, a| {
            h9::sprite_collection_dtor(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PSamplerStateD3D11_create_00595bf0",
        addr: 0x0059_5bf0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            HOLDER_STUBS[0],
            HOLDER_STUBS[1],
            HOLDER_STUBS[2],
            (0x0046_3e90, 0, 0, true),
            (0x0058_3100, 1, 1, true),
            // (the filter and mode maps: custom stand-ins that keep
            // `edx`, whose low byte the original reads after them, as
            // the real leaf functions do)
        ],
        prep: prep_sampler,
        port: |g, a| {
            let frame = g.frame();
            h9::create_sampler_state(g, a[0], frame)
        },
        frame: 0x44,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PDynamicBufferD3D11_mapForWrite_00563290",
        addr: 0x0056_3290,
        conv: K::T,
        kinds: "pvp",
        out: O::U32,
        callees: &[
            HOLDER_STUBS[0],
            HOLDER_STUBS[1],
            HOLDER_STUBS[2],
            (0x0046_3e90, 0, 0, true),
            (0x0056_0f90, 3, 0, false),
        ],
        prep: prep_maps,
        port: |g, a| {
            let frame = g.frame();
            h9::map_for_write(g, a[0], a[1], a[2], frame)
        },
        frame: 0x2c,
        gap: 0x10,
        writes: &[(0x0056_0f90, 0)],
    },
    Case {
        name: "PDynamicBufferD3D11_mapForRead_00562e00",
        addr: 0x0056_2e00,
        conv: K::T,
        kinds: "pvp",
        out: O::U32,
        callees: &[
            HOLDER_STUBS[0],
            HOLDER_STUBS[1],
            HOLDER_STUBS[2],
            (0x0046_3e90, 0, 0, true),
        ],
        prep: prep_maps,
        port: |g, a| {
            let frame = g.frame();
            h9::map_for_read(g, a[0], a[1], a[2], frame)
        },
        frame: 0x38,
        gap: 0x10,
        writes: &[],
    },
];
