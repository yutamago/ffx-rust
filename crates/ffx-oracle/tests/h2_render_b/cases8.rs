//! Continuation round batch 2 cases (`ffx_prender::h2_more8`), written by
//! hand: one per port, the same call as its shim.

use super::*;

use ffx_prender::h2_more8::bridge as h8;

/// A pool word `off` bytes into the pool.
fn at(e: &Env, off: u32) -> u32 {
    e.pool + off
}

/// A result for an engine call or `HRESULT`: mostly 0, else a failure
/// code or a random word.
fn mostly_zero(rng: &mut Rng) -> u32 {
    match rng.below(12) {
        0 => 0x8007_0057,
        1 => rng.next(),
        _ => 0,
    }
}

/// A feature level the probe may report.
fn level(rng: &mut Rng) -> u32 {
    [
        0x9100, 0x9300, 0xa000, 0xa000, 0xa100, 0xb000, 0xb000, 0xb100,
    ][rng.below(8) as usize]
}

/// `createDeviceAndSwapChain`: `this` in the pool's second half; the two
/// imports' results and feature levels; the settings block; the device,
/// its queried interface, the swap chain (back buffer or none), the
/// context and the factory as COM objects (each missing now and then);
/// the engine calls' results.
fn prep_create_device(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    a[3] = [0, 1, 2, 4, 8, 16, 17, 0x8000_0000, rng.next()][rng.below(9) as usize];
    a[4] = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
    for (sym, negative) in [(0x0096_838e, false), (0x0096_8394, true)] {
        let r = if rng.chance(10) { 0x8000_4005 } else { 0 };
        e.set_ret(sym, r);
        // the level the second call reports goes to `+0xb4` (compared
        // signed)
        let l = level(rng)
            | if negative && rng.chance(8) {
                0x8000_0000
            } else {
                0
            };
        e.set_out(sym, l);
    }
    let settings = at(e, 0x500);
    let g = e.rebase(0x022f_b504);
    e.w32(g, settings);
    for off in [0x38, 0x48] {
        let v = if rng.chance(3) { 0 } else { rng.next() | 1 };
        e.w32(settings + off, v);
    }
    // the device: QueryInterface (slot 0) stores the queried object
    let device = at(e, 0x100);
    let q = at(e, 0x180);
    let r = mostly_zero(rng) | if rng.chance(6) { 0x8000_0000 } else { 0 };
    let qi = e.writer(1, 3, 2, r, q);
    e.w32(device, device + 0x20);
    e.w32(device + 0x20, qi);
    let made = e.handler(4, rng.next());
    e.w32(device + 0x20 + 12, made);
    let r = rng.next();
    let o = rng.next();
    let qi2 = e.writer(2, 3, 2, r, o);
    e.w32(q, q + 0x20);
    e.w32(q + 0x20, qi2);
    let rel = e.handler(1, rng.next());
    e.w32(q + 0x28, rel);
    // the swap chain: GetBuffer (slot 9) stores the back buffer, GetParent
    // (slot 6) the factory's word is set here
    let chain = at(e, 0x200);
    let texture = if rng.chance(10) { 0 } else { e.ptr(rng) };
    let get = e.writer(3, 4, 3, rng.next(), texture);
    e.w32(chain, chain + 0x20);
    e.w32(chain + 0x20 + 36, get);
    let factory = at(e, 0x400);
    e.w32(this + 0x133c, factory);
    e.w32(factory, factory + 0x20);
    let assoc = e.handler(3, rng.next());
    e.w32(factory + 0x20 + 32, assoc);
    // (the same three-word handler: its result set last, for GetParent)
    let r = rng.next()
        & if rng.chance(3) {
            0x8000_0000
        } else {
            0x7fff_ffff
        };
    let parent = e.handler(3, r);
    e.w32(chain + 0x20 + 24, parent);
    // the context: OMSetRenderTargets (slot 33)
    let context = at(e, 0x300);
    e.w32(context, context + 0x20);
    let bind = e.handler(4, rng.next());
    e.w32(context + 0x20 + 33 * 4, bind);
    for (off, obj) in [(0x6c, device), (0x70, context), (0x74, chain)] {
        let v = if rng.chance(16) { 0 } else { obj };
        e.w32(this + off, v);
    }
    let level = level(rng);
    e.w32(this + 0xb4, level);
    // the two view creations (thiscall, no words) return the pool's words
    // 0x600 and 0x604 in turn and log their `ecx` at 0x608 and 0x60c
    let p = e.pool;
    e.custom(0x0059_1460, |_| {
        let mut c = vec![0xa1];
        c.extend((p + 0x610).to_le_bytes()); // mov eax, [n]
        c.extend([0x89, 0x0c, 0x85]);
        c.extend((p + 0x608).to_le_bytes()); // mov [ecxs + eax*4], ecx
        c.extend([0x8b, 0x14, 0x85]);
        c.extend((p + 0x600).to_le_bytes()); // mov edx, [results + eax*4]
        c.extend([0xff, 0x05]);
        c.extend((p + 0x610).to_le_bytes()); // inc dword [n]
        c.extend([0x89, 0xd0, 0xc3]); // mov eax, edx; ret
        c
    });
    e.w32(p + 0x610, 0);
    // at most one engine call fails (1, an HRESULT or a random word), so
    // that each check is reached
    let fail = rng.below(14);
    let bad = [1, 0x8007_0057, rng.next() | 2][rng.below(3) as usize];
    let r = |i: u32| if i == fail { bad } else { 0 };
    for k in 0..2 {
        e.w32(p + 0x600 + 4 * k, r(k));
    }
    for (i, sym) in [
        0x0058_fb90,
        0x0059_4f70,
        0x0059_5200,
        0x0059_54f0,
        0x0059_5620,
        0x0058_cf80,
    ]
    .into_iter()
    .enumerate()
    {
        e.set_ret(sym, r(2 + i as u32));
    }
}

/// `setTextureQuality`: `q` equal to `+0x1340` or not; the file list (a
/// tree header at `+0x1344`): empty, or a first node found from `+0x1350`
/// down zero to two nodes' word 0, then up to three more files linked by
/// word 4, which the iterator stand-in follows (its result is off by the
/// difference when its `ecx` is not the list).
fn prep_texture_quality(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    if rng.chance(5) {
        let q = a[1];
        e.w32(this + 0x1340, q);
    }
    let list = this + 0x1344;
    let p = e.pool;
    e.custom(0x0057_51d0, |_| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x40, 0x04]); // mov eax, [eax+4]
        c.extend([0x01, 0xc8]); // add eax, ecx
        c.extend([0xff, 0x05]);
        c.extend((p + 0x624).to_le_bytes()); // inc dword [calls]
        c.extend([0x2b, 0x05]);
        c.extend((p + 0x620).to_le_bytes()); // sub eax, [list]
        c.extend([0xc2, 0x04, 0x00]);
        c
    });
    e.w32(p + 0x620, list);
    let node = |k: u32| p + 0x800 + 0x40 * k;
    if rng.chance(5) {
        e.w32(list + 0xc, list);
        return;
    }
    let down = rng.below(3);
    let files = 1 + rng.below(4);
    // down nodes 0..down, then the files down..down + files
    for k in 0..down {
        e.w32(node(k), node(k + 1));
    }
    e.w32(list + 0xc, node(0));
    e.w32(node(down), list);
    for k in 0..files {
        let f = node(down + k);
        let next = if k + 1 == files {
            list
        } else {
            node(down + k + 1)
        };
        e.w32(f + 4, next);
    }
    let r = e.ret_word(rng);
    e.set_ret(0x0042_fb80, r);
}

/// `resizeBuffers`: the swap chain (missing now and then) whose
/// `ResizeBuffers` fails now and then and whose `GetBuffer` stores a back
/// buffer (or none), the context, the sample count.
fn prep_resize(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let context = at(e, 0x300);
    e.w32(context, context + 0x20);
    let bind = e.handler(4, rng.next());
    e.w32(context + 0x20 + 33 * 4, bind);
    e.w32(this + 0x70, context);
    let chain = at(e, 0x200);
    let texture = if rng.chance(8) { 0 } else { e.ptr(rng) };
    let get = e.writer(4, 4, 3, rng.next(), texture);
    e.w32(chain, chain + 0x20);
    e.w32(chain + 0x20 + 36, get);
    let r = if rng.chance(6) { 0x887a_0001 } else { 0 };
    let resize = e.handler(6, r);
    e.w32(chain + 0x20 + 13 * 4, resize);
    let c = if rng.chance(8) { 0 } else { chain };
    e.w32(this + 0x74, c);
    let samples = [0, 1, 2, 4, 8, 0x101, rng.next()][rng.below(7) as usize];
    e.w32(this + 0x94, samples);
}

pub static CASES: &[Case] = &[
    Case {
        name: "Phyre_PRenderInterface_setTextureQuality",
        addr: 0x0058_e000,
        conv: K::T,
        kinds: "pv",
        out: O::U32,
        callees: &[
            (0x0043_1840, 0, 0, true),
            (0x0042_fb80, 2, 0, false),
            (0x0046_3d10, 0, 0, true),
            (0x0057_66d0, 1, 1, true),
            (0x0060_80e0, 2, 2, true),
            (0x0046_6da0, 2, 2, true),
            (0x0042_fb20, 1, 0, false),
            (0x0057_6930, 0, 0, true),
            (0x0057_67c0, 0, 0, true),
            (0x0043_19d0, 0, 0, true),
        ],
        prep: prep_texture_quality,
        port: |g, a| {
            let frame = g.frame();
            h8::set_texture_quality(g, a[0], a[1], frame)
        },
        frame: 0x28,
        gap: 0xc,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_resizeBuffers_00599210",
        addr: 0x0059_9210,
        conv: K::T,
        kinds: "pvv",
        out: O::None,
        callees: &[
            (0x0059_c650, 0, 0, true),
            (0x0058_fb90, 4, 4, true),
            (0x0059_1460, 0, 0, true),
            (0x004c_40c0, 4, 4, true),
            (0x004c_4250, 0, 0, true),
            (0x0058_c790, 4, 4, true),
            (0x005b_3660, 3, 3, true),
        ],
        prep: prep_resize,
        port: |g, a| {
            let frame = g.frame();
            h8::resize_buffers(g, a[0], a[1], a[2], frame)
        },
        frame: 0xc,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_createDeviceAndSwapChain_00595800",
        addr: 0x0059_5800,
        conv: K::T,
        kinds: "pvvvvvv",
        out: O::U32,
        callees: &[
            (0x0096_838e, 10, 10, true),
            (0x0096_8394, 12, 12, true),
            (0x0040_2a00, 2, 2, true),
            (0x0094_964c, 3, 0, false),
            (0x0058_fb90, 4, 4, true),
            (0x004c_40c0, 4, 4, true),
            (0x004c_4250, 0, 0, true),
            (0x0059_bbf0, 4, 4, true),
            (0x005b_3660, 3, 3, true),
            (0x0059_4f70, 1, 1, true),
            (0x0059_5200, 1, 1, true),
            (0x0059_54f0, 3, 3, true),
            (0x0059_5620, 6, 6, true),
            (0x0058_cf80, 0, 0, true),
        ],
        prep: prep_create_device,
        port: |g, a| {
            let frame = g.frame();
            h8::create_device_and_swap_chain(
                g, a[0], a[1], a[2], a[3], a[4] as u8, a[5] as u8, a[6], frame,
            )
        },
        frame: 0x3c,
        gap: 4,
        writes: &[(0x0096_838e, 8), (0x0096_8394, 10)],
    },
];
