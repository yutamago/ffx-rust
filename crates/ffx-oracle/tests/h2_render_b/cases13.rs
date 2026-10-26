//! h2-render-b rest (`ffx_prender::h2_more13`), written by hand: one
//! case per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more13::bridge as h13;

/// `0x00591460`, the targets' view creation (0: views ready).
const CREATE_VIEWS: u32 = 0x0059_1460;

/// A logging handler for `ID3D11DeviceContext::RSSetViewports(ctx, n,
/// viewports)`: logs its tag, `ecx` and the six words of the first
/// viewport, pops three words.
fn viewport_logger(e: &mut Env) -> u32 {
    let log = e.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(0x0b0b_0b0bu32.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    c.extend([0x8b, 0x54, 0x24, 0x0c]); // mov edx, [esp+0xc]
    for k in 0..6u8 {
        c.extend([0x8b, 0x4a, 4 * k]); // mov ecx, [edx+4k]
        c.extend([0x89, 0x48, 8 + 4 * k]); // mov [eax+8+4k], ecx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.extend([0x31, 0xc0, 0xc2, 0x0c, 0x00]); // xor eax, eax; ret 0xc
    let code = e.a.alloc_code(&c).unwrap();
    assert_eq!(e.b.alloc_code(&c).unwrap(), code);
    code
}

/// `n` bytes of the pool's upper part from `next` on (the objects a
/// case writes must not overlap).
fn take(next: &mut u32, n: u32) -> u32 {
    let at = *next;
    *next += n;
    at
}

/// A texture for a render target: its descriptor's flags (bit 6 2D
/// target, bit 8 cube target, bit 1 not a target) and its size, mostly
/// the case's common size.
fn target_texture(e: &mut Env, rng: &mut Rng, next: &mut u32, size: (u32, u32)) -> u32 {
    let tex = take(next, 0x30);
    let desc = take(next, 0x20);
    e.w32(tex, desc);
    let flags = [0x40, 0x100, 0x140, 0x42, 0x102, 0, rng.next()][rng.below(7) as usize];
    e.w32(desc + 0x10, flags);
    let (w, h) = if rng.chance(5) {
        (rng.below(3) * 4, rng.below(3) * 4)
    } else {
        size
    };
    e.w32(tex + 0x1c, w);
    e.w32(tex + 0x20, h);
    tex
}

/// A render target: its texture (null now and then), kind byte (2D 0/1,
/// cube 2, other), sample byte, its 2D view (`+0x10`), depth view
/// (`+0x14`) and per-face views (`+0x1c`, `+0x24`).
fn target(e: &mut Env, rng: &mut Rng, next: &mut u32, size: (u32, u32), samples: u8) -> u32 {
    let t = take(next, 0x30);
    let tex = if rng.chance(12) {
        0
    } else {
        target_texture(e, rng, next, size)
    };
    e.w32(t, tex);
    let kind = [0u8, 1, 2, 2, 3][rng.below(5) as usize];
    let s = if rng.chance(6) {
        rng.below(4) as u8
    } else {
        samples
    };
    e.write(t + 4, &[kind, s]);
    for off in [0x10, 0x14] {
        let v = e.ret_word(rng);
        e.w32(t + off, v);
    }
    for off in [0x1c, 0x24] {
        let faces = e.ptr(rng);
        e.w32(t + off, faces);
    }
    t
}

/// `bindRenderTargets`: the renderer at the pool's upper part with its
/// eight colour slots (null or targets agreeing on a size mostly), a
/// depth target (null, or one with a texture), the back buffer's words,
/// the window, the viewport fields, the context with its handlers, and
/// the view creation failing now and then.
fn prep_bind_targets(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let size = (
        [4, 8, 0x8000_0000][rng.below(3) as usize],
        [4, 8, 2][rng.below(3) as usize],
    );
    let samples = rng.below(3) as u8;
    let none = rng.chance(4);
    let mut next = this + 0x1600;
    for k in 0..8 {
        let slot = this + 0x228 + 16 * k;
        let t = if none || rng.chance(2) {
            0
        } else {
            target(e, rng, &mut next, size, samples)
        };
        e.w32(slot, t);
        e.w32(slot + 4, rng.below(6));
    }
    let depth = if rng.chance(3) {
        0
    } else {
        let d = target(e, rng, &mut next, size, samples);
        let tex = e.r32(d);
        if tex == 0 {
            let tex = target_texture(e, rng, &mut next, size);
            e.w32(d, tex);
        }
        d
    };
    e.w32(this + 0x2a8, depth);
    e.w32(this + 0x2ac, rng.below(6));
    let window = e.ptr(rng);
    e.w32(this + 0x1a0, window);
    // set, the original reads the window size through a null pointer
    e.write(this + 0x1a4, &[0]);
    for off in [0x1324, 0x1328, 0x132c, 0x1330] {
        let v = [0, 16, rng.below(0x800), rng.next()][rng.below(4) as usize];
        e.w32(this + off, v);
    }
    let r = if rng.chance(10) {
        [7, 9, 1][rng.below(3) as usize]
    } else {
        0
    };
    e.set_ret(CREATE_VIEWS, r);
    // the context: vtable at ctx + 0x40, a handler of its own per method
    // (so that the log tells the methods apart)
    let ctx = e.pool + POOL_PTRS + 0x1400;
    e.w32(this + 0x70, ctx);
    e.w32(ctx, ctx + 0x40);
    for slot in [0x20, 0x64, 0x7c, 0x10c, 0xec, 0xfc, 0x84] {
        let (f, cell) = e.stub_code(0x0c0c_0000 + slot, 4, 4, true);
        let r = e.ret_word(rng);
        e.w32(cell, r);
        e.w32(ctx + 0x40 + slot, f);
    }
    let f = viewport_logger(e);
    e.w32(ctx + 0x40 + 0xb0, f);
}

/// The getters `setTextureParameters` calls (thiscall, one word).
const GETTERS: [u32; 12] = [
    0x0059_3db0,
    0x0059_3de0,
    0x0059_3e30,
    0x0059_3d60,
    0x0059_4730,
    0x0059_3f90,
    0x0059_4830,
    0x0059_37a0,
    0x0059_3ca0,
    0x0059_3fe0,
    0x0059_4010,
    0x0059_4780,
];

/// A recording `Phyre_PhyrePrintf`: logs the level and the format, and
/// the third word unless the format is one of the two messages printed
/// without an argument (the original's third word is then its caller's).
fn printf_stub(e: &mut Env) {
    let log = e.log;
    let plain = [e.rebase(0x00b3_3808), e.rebase(0x00b3_38a0)];
    e.custom(0x0043_52f0, |_| {
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(0x0043_52f0u32.to_le_bytes()); // mov [eax], tag
        c.extend([0x8b, 0x54, 0x24, 0x04, 0x89, 0x50, 0x08]); // the level
        c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x50, 0x0c]); // the format
        c.extend([0x81, 0xfa]);
        c.extend(plain[0].to_le_bytes()); // cmp edx, plain 0
        c.extend([0x74, 0x0f]);
        c.extend([0x81, 0xfa]);
        c.extend(plain[1].to_le_bytes()); // cmp edx, plain 1
        c.extend([0x74, 0x07]);
        c.extend([0x8b, 0x54, 0x24, 0x0c, 0x89, 0x50, 0x10]); // the argument
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xc3);
        c
    });
}

/// `setTextureParameters`: the program's parameter block (or none), up
/// to eight entries over parameters of every type (objects set or
/// null), locations 0..9 or -1, the count's high bits random.
fn prep_texture_parameters(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let program = e.ptr(rng);
    a[2] = program;
    let index = rng.below(4);
    a[3] = index;
    let block = e.pool + POOL_PTRS + 0x800;
    e.w32(
        program + 0x20 + 4 * index,
        if rng.chance(10) { 0 } else { block },
    );
    if !rng.chance(10) {
        e.w32(program + 0x14, 1 + rng.below(4));
    }
    let types = [
        0x42, 0x43, 0x44, 0x45, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0xde, 0xe2, 0, 0x99,
    ];
    for j in 0..16u32 {
        let p = block + 0x20 * j;
        e.w32(p, types[rng.below(16) as usize]);
        for off in [8, 0xc, 0x10, 0x14] {
            let v = if rng.chance(2) { 0 } else { e.ptr(rng) };
            e.w32(p + off, v);
        }
    }
    let entries = e.pool + POOL_PTRS + 0xc00;
    a[4] = if rng.chance(12) { 0 } else { entries };
    for k in 0..8u32 {
        let at = entries + 16 * k;
        e.write(at, &((0x20 * rng.below(16)) as u16).to_le_bytes());
        let loc = if rng.chance(6) {
            u32::MAX
        } else {
            rng.below(10)
        };
        e.w32(at + 4, loc);
    }
    a[6] = rng.below(9) | (rng.next() & 0xffff_0000);
    // the parameter's name object: a word with a low tag bit
    let name = e.ptr(rng);
    let w = e.ptr(rng) | u32::from(rng.chance(2));
    e.w32(name + 4, w);
    let r = if rng.chance(2) { 0 } else { name };
    e.set_ret(0x0060_fdd0, r);
    printf_stub(e);
}

pub static CASES: &[Case] = &[
    Case {
        name: "PRendererD3D11_bindRenderTargets_0058c830",
        addr: 0x0058_c830,
        conv: K::T,
        kinds: "pv",
        out: O::U8,
        callees: &[
            (CREATE_VIEWS, 0, 0, true),
            (0x0094_964c, 3, 0, false),
            (0x0058_e3d0, 1, 1, true),
        ],
        prep: prep_bind_targets,
        port: |g, a| {
            let frame = g.frame();
            u32::from(h13::bind_targets(g, frame, a[0], a[1]))
        },
        frame: h13::BIND_FRAME,
        gap: h13::BIND_GAP,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PShaderProgramBindingNVN_setTextureParameters",
        addr: 0x0059_b3d0,
        conv: K::T,
        kinds: "ppvvpvv",
        out: O::U8,
        callees: &[
            (GETTERS[0], 1, 1, true),
            (GETTERS[1], 1, 1, true),
            (GETTERS[2], 1, 1, true),
            (GETTERS[3], 1, 1, true),
            (GETTERS[4], 1, 1, true),
            (GETTERS[5], 1, 1, true),
            (GETTERS[6], 1, 1, true),
            (GETTERS[7], 1, 1, true),
            (GETTERS[8], 1, 1, true),
            (GETTERS[9], 1, 1, true),
            (GETTERS[10], 1, 1, true),
            (GETTERS[11], 1, 1, true),
            (0x0060_fdd0, 2, 2, true),
        ],
        prep: prep_texture_parameters,
        port: |g, a| {
            u32::from(h13::set_texture_parameters(
                g,
                a[0],
                a[1],
                a[2],
                a[3],
                a[4],
                a[6] as u16,
            ))
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
];
