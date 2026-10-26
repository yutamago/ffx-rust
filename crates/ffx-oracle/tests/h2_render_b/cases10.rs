//! Continuation round batch 4 cases (`ffx_prender::h2_more10`), written
//! by hand: one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more10::bridge as h10;

/// The skeleton bounds: the bounds object (none now and then), the
/// mesh's bone count (0 to 3, junk in bit 31) and bone boxes (or none),
/// its joint count, the allocation (failing now and then), the boxes'
/// joint indices, the vector helpers' results (pool vectors).
fn prep_skeleton_bounds(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let mesh = this + 0x40;
    e.w32(this, mesh);
    let bounds = if rng.chance(10) { 0 } else { e.ptr(rng) };
    e.w32(this + 0x1c, bounds);
    let bone_count = rng.below(4) | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(mesh + 8, bone_count);
    let boxes = this + 0x100;
    e.w32(mesh + 0x14, if rng.chance(10) { 0 } else { boxes });
    // (0x7fffffff: one more joint makes the block's count word borrowed)
    let joints = match rng.below(8) {
        0 => 0x7fff_ffff,
        1 => rng.below(4) | 0x8000_0000,
        _ => rng.below(4),
    };
    e.w32(mesh + 0x18, joints);
    for k in 0..3 {
        let j = rng.below(5);
        e.w32(boxes + 0x20 * k + 0xc, j);
    }
    let block = if rng.chance(8) { 0 } else { e.ptr(rng) };
    e.set_ret(0x0042_fb80, block);
    for sym in [0x004f_8580, 0x004f_84f0] {
        let v = e.ptr(rng);
        e.set_ret(sym, v);
    }
}

/// The box transform: `lo`, `hi`, the outputs and the matrix in the
/// pool (floats from the pool's random words, quiet); the point
/// transform and the vector helpers as stand-ins that log their words
/// (out of the pool's pointer range) and return their out pointer, so
/// that the pairing of the eight corners shows.
fn prep_box(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let base = e.pool + 4 * rng.below(0x300);
    for (k, arg) in a.iter_mut().take(5).enumerate() {
        *arg = base + 0x40 * k as u32;
    }
    // mostly floats of like size (where the order of the sums shows in
    // the rounding), else the pool's random words
    if !rng.chance(4) {
        for k in 0..0x50 {
            let v = (rng.below(20_000) as f32 - 10_000.0) / 7.0;
            e.w32(base + 4 * k, v.to_bits());
        }
    }
    let log = e.pool + POOL_PTRS + 0x1800;
    e.w32(log, 0);
    for sym in [0x004c_9eb0u32, 0x004f_8580, 0x004f_84f0] {
        let words: u8 = if sym == 0x004c_9eb0 { 2 } else { 3 };
        e.custom(sym, |_| {
            let mut c = vec![0xa1];
            c.extend(log.to_le_bytes()); // mov eax, [n]
            c.extend([0xc1, 0xe0, 0x04]); // shl eax, 4
            c.push(0x05);
            c.extend((log + 0x10).to_le_bytes()); // add eax, log + 0x10
            if words == 2 {
                c.extend([0x89, 0x08]); // mov [eax], ecx (the thiscall only)
            }
            for i in 0..words {
                c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
                c.extend([0x89, 0x50, 4 + 4 * i]); // mov [eax+4+4i], edx
            }
            c.extend([0xff, 0x05]);
            c.extend(log.to_le_bytes()); // inc dword [n]
            // the helpers return their out pointer; the point transform its
            // input (so that the first corner is of like size)
            let ret_arg = if words == 2 { 0x08 } else { 0x04 };
            c.extend([0x8b, 0x44, 0x24, ret_arg]); // mov eax, [esp+ret_arg]
            if words == 2 {
                c.extend([0xc2, 0x08, 0x00]); // thiscall: ret 8
            } else {
                c.push(0xc3);
            }
            c
        });
    }
}

/// The scene key: the camera (its world matrix or none), the vector,
/// the viewport (sizes with bit 31 now and then).
fn prep_scene_key(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let camera = e.pool + 0x200;
    a[1] = camera;
    let world = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(camera + 0xb0, world);
    a[2] = e.pool + 4 * rng.below(0x300);
    let viewport = e.pool + 0x400;
    a[3] = viewport;
    for off in [0x10, 0x14] {
        let v = [1280, 720, 0x8000_0000 | rng.next(), rng.next()][rng.below(4) as usize];
        e.w32(viewport + off, v);
    }
}

pub static CASES: &[Case] = &[
    Case {
        name: "Phyre_PRendering_PSceneWideParameters_PSceneWideParametersKey_initialize",
        addr: 0x004f_e940,
        conv: K::T,
        kinds: "pppp",
        out: O::U32,
        callees: &[
            (0x0045_3340, 1, 1, true),
            (0x0045_4260, 3, 0, false),
            (0x0045_1080, 1, 1, true),
            (0x004c_a360, 3, 0, false),
            (0x0045_1190, 1, 1, true),
        ],
        prep: prep_scene_key,
        port: |g, a| {
            let frame = g.frame();
            h10::scene_key_initialize(g, a[0], a[1], a[2], a[3], frame)
        },
        frame: 0x94,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PBounds_transformBox_004f63d0",
        addr: 0x004f_63d0,
        conv: K::C,
        kinds: "ppppp",
        out: O::None,
        callees: &[],
        prep: prep_box,
        port: |g, a| {
            let frame = g.frame();
            h10::transform_box(g, a[0], a[1], a[2], a[3], a[4], frame);
            0
        },
        frame: 0x130,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PBounds_transformBox_004ffe40",
        addr: 0x004f_fe40,
        conv: K::C,
        kinds: "ppppp",
        out: O::None,
        callees: &[],
        prep: prep_box,
        port: |g, a| {
            let frame = g.frame();
            h10::transform_box(g, a[0], a[1], a[2], a[3], a[4], frame);
            0
        },
        frame: 0x130,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PMeshInstance_updateBoundsFromSkeleton",
        addr: 0x004f_96a0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x0042_fb80, 2, 0, false),
            (0x004f_6f70, 1, 1, true),
            (0x004f_63d0, 5, 0, false),
            (0x004f_8580, 3, 0, false),
            (0x004f_84f0, 3, 0, false),
            (0x0050_0bd0, 2, 2, true),
            (0x0042_fb20, 1, 0, false),
        ],
        prep: prep_skeleton_bounds,
        port: |g, a| {
            let frame = g.frame();
            h10::update_bounds_from_skeleton(g, a[0], frame)
        },
        frame: 0xa0,
        gap: 0xc,
        writes: &[],
    },
];
