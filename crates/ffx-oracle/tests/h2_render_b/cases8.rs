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

/// A storage array at `at` (count word, then the inline word or the
/// storage pointer) with `n` words, the first `first`.
fn storage_array(e: &mut Env, rng: &mut Rng, at: u32, n: u32, first: u32, data: u32) {
    e.w32(at, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    if n == 1 {
        e.w32(at + 4, first);
    } else {
        e.w32(at + 4, data);
        e.w32(data, first);
    }
}

/// `createViews`: the kind (0 2D, 1 volume, 2 cube, now and then 3), the
/// texture (none, the placeholder, or one whose device object exists or
/// not, its format depth or colour), the views present or not, the
/// face arrays (six already or not, owned or not), the device's two
/// view creators (results, the views they store) and the getters.
fn prep_create_views(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let v = e.pool + POOL_PTRS + 0x40;
    a[0] = v;
    let t = v + 0xc0;
    let d = v + 0x1c0;
    let ctx = v + 0x2c0;
    let device = v + 0x3c0;
    let data = v + 0x4c0;
    let kind = [0u8, 0, 1, 1, 2, 2, 2, 3][rng.below(8) as usize];
    e.write(v + 4, &[kind]);
    let tex = match rng.below(12) {
        0 => 0,
        1 => {
            let p = e.rebase(0x00ca_34cc);
            e.w32(t, p);
            t
        }
        _ => {
            e.w32(t, d);
            t
        }
    };
    e.w32(v, tex);
    let b5 = [0u8, 1, 0x80][rng.below(3) as usize];
    let b8 = rng.next() as u8;
    e.write(v + 5, &[b5]);
    e.write(v + 8, &[b8]);
    for off in [0x10, 0x14] {
        let w = if rng.chance(5) { e.ptr(rng) } else { 0 };
        e.w32(v + off, w);
    }
    for (count, ptr) in [(0x18, 0x1c), (0x20, 0x24)] {
        let n = [6, 0, 3, 0x8000_0006, 0x8000_0002][rng.below(5) as usize];
        e.w32(v + count, n);
        let p = e.ptr(rng);
        e.w32(v + ptr, p);
    }
    // the format: bit 1 of `+0x10` depth
    let flags = rng.next();
    e.w32(d + 0x10, flags);
    // the texture's device object made or not; the storage arrays the
    // kinds read (never empty when the object is still to make)
    for made in [0x70, 0x64] {
        let m = if rng.chance(2) { 0 } else { rng.next() | 1 };
        e.w32(t + made, m);
    }
    // (the kinds' arrays overlap: only this kind's)
    let (at, made) = match kind {
        0 | 3 => (0x24, 0x70),
        1 => (0x28, 0x64),
        _ => (0x20, 0x64),
    };
    let lo = u32::from(e.r32(t + made) == 0 || at != 0x24);
    let n = lo.max(rng.below(4));
    let first = if rng.chance(3) { 0 } else { e.ptr(rng) };
    storage_array(e, rng, t + at, n, first, data);
    let m = [1, 1, 2, 0][rng.below(4) as usize];
    e.w32(t + 0x4c, m);
    e.set_ret(0x0046_49d0, ctx);
    e.w32(ctx + 0x6c, device);
    e.w32(device, device + 0x20);
    // the view creators: per call a result and a stored view (a failure
    // or a null view now and then, at any call)
    for (k, slot) in [(0u32, 9u32), (1, 10)] {
        let (f, base) = view_creator(e, 0xf00d_8000 + k, 4);
        e.w32(device + 0x20 + 4 * slot, f);
        e.w32(base, 0);
        for i in 0..8 {
            let r = match rng.below(24) {
                0 => 0x8007_000e,
                1 => rng.next() | 0x8000_0000,
                k => k & 1,
            };
            let view = if rng.chance(30) { 0 } else { e.ptr(rng) };
            e.w32(base + 0x20 + 4 * i, r);
            e.w32(base + 0x80 + 4 * i, view);
        }
    }
    let r = if rng.chance(2) { 0 } else { e.ptr(rng) };
    e.set_ret(0x0059_4060, r);
    let r = if rng.chance(8) { 0 } else { e.ptr(rng) };
    e.set_ret(0x0042_fb80, r);
    // the face array's storage the allocation returns, now and then
    if r != 0 && rng.chance(4) {
        for ptr in [0x1c, 0x24] {
            e.w32(v + ptr, r);
        }
    }
}

/// A creator method of `words` words (stdcall, `this` first, its out
/// pointer last; at most five): logs `ecx` and its words at
/// `base + 0xe0 + 32 n`, stores word `n` of `base + 0x80` through the out
/// pointer, returns word `n` of `base + 0x20` (`n` the call count at
/// `base`, at most 24). Made once per `tag`, its state (`base`) in a
/// block of its own: a pool word the port writes through a random
/// pointer must not be one of its counters. Returns (code, base).
fn view_creator(e: &mut Env, tag: u32, words: u8) -> (u32, u32) {
    if let Some(&w) = e.writers.get(&tag) {
        return w;
    }
    let base = e.zeros(0x400);
    let n = base;
    let mut c = vec![0xa1];
    c.extend(n.to_le_bytes()); // mov eax, [n]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((base + 0xe0).to_le_bytes()); // add eax, log
    c.extend([0x89, 0x08]); // mov [eax], ecx
    for i in 0..words {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 4 + 4 * i]); // mov [eax+4+4i], edx
    }
    c.push(0xa1);
    c.extend(n.to_le_bytes()); // mov eax, [n]
    c.extend([0x8b, 0x0c, 0x85]);
    c.extend((base + 0x80).to_le_bytes()); // mov ecx, [views + eax*4]
    c.extend([0x89, 0x0a]); // mov [edx], ecx (edx = out)
    c.extend([0x8b, 0x0c, 0x85]);
    c.extend((base + 0x20).to_le_bytes()); // mov ecx, [results + eax*4]
    c.extend([0xff, 0x05]);
    c.extend(n.to_le_bytes()); // inc dword [n]
    c.extend([0x89, 0xc8, 0xc2, 4 * words, 0x00]); // mov eax, ecx; ret 4 * words
    let at = e.a.alloc_code(&c).unwrap();
    assert_eq!(e.b.alloc_code(&c).unwrap(), at);
    e.writers.insert(tag, (at, base));
    (at, base)
}

/// The renderer constructor: the free list's constructor stand-in logs
/// its words and leaves the list's first node (`+0xc`) a chain of zero
/// to two nodes (linked by word 0) ending at the list, which the
/// remove stand-in unlinks one by one.
fn prep_renderer_ctor(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let p = e.pool;
    e.custom(0x0049_c480, |_| {
        let mut c = vec![];
        for i in 0..4u8 {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * i]); // mov eax, [esp+4+4i]
            c.push(0xa3);
            c.extend((p + 0x640 + 4 * u32::from(i)).to_le_bytes()); // mov [log+4i], eax
        }
        c.extend([0x89, 0x0d]);
        c.extend((p + 0x650).to_le_bytes()); // mov [log+0x10], ecx
        c.push(0xa1);
        c.extend((p + 0x630).to_le_bytes()); // mov eax, [first]
        c.extend([0x89, 0x41, 0xf4]); // mov [ecx-0xc], eax
        c.extend([0xc2, 0x10, 0x00]);
        c
    });
    e.custom(0x0057_5000, |_| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x00]); // mov eax, [eax]
        c.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
        c.extend([0xff, 0x05]);
        c.extend((p + 0x654).to_le_bytes()); // inc dword [removed]
        c.extend([0xc2, 0x04, 0x00]);
        c
    });
    let list = this + 0x1344;
    let n = rng.below(3);
    let node = |k: u32| p + 0x700 + 0x20 * k;
    for k in 0..n {
        let next = if k + 1 == n { list } else { node(k + 1) };
        e.w32(node(k), next);
    }
    let first = match rng.below(4) {
        0 => 0,
        _ if n == 0 => list,
        _ => node(0),
    };
    e.w32(p + 0x630, first);
}

/// `initWindow`: the window info (null now and then: a pool record with
/// the window, a size word and the flag byte), `GetClientRect` through
/// its import word (a stand-in storing the rectangle's first word).
fn prep_init_window(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + POOL_PTRS;
    let r = rng.next();
    let left = rng.next() & 0xfff;
    let f = e.writer(7, 2, 1, r, left);
    let iat = e.rebase(0x00b0_c680);
    e.w32(iat, f);
}

/// `findContextVariant`: the variant count (none now and then), the
/// switch count, the key builder's cursor (a stand-in storing it past
/// the original's `alloca` buffer), the variant entries' comparison
/// results (sorted around a target, or random signs) read by the
/// comparison stand-in from each entry's first word.
fn prep_find_variant(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let n = if rng.chance(8) { 0 } else { 1 + rng.below(9) };
    e.w32(this + 0x2c, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    let data = this + 0x100;
    e.w32(this + 0x30, data);
    let words = rng.below(9);
    let sc = e.rebase(0x00ca_4e20);
    e.w32(sc, words);
    let ebp = e.a.stack_range().end - 0x100 - 4 * 5 - 8;
    let buf = (ebp - 0x20 - 4 * words) & !0xf;
    // (now and then before the buffer: the count is a signed shift)
    let end = if rng.chance(8) {
        buf - 4 * (1 + rng.below(3))
    } else {
        buf + 4 * rng.below(words + 1)
    };
    e.set_out(0x004e_f3f0, end);
    let p = e.pool;
    e.custom(0x004e_ea10, |_| {
        let mut c = vec![0x8b, 0x54, 0x24, 0x04]; // mov edx, [esp+4]
        c.extend([0x89, 0x15]);
        c.extend((p + 0x65c).to_le_bytes()); // mov [key], edx
        c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
        c.extend([0x89, 0x15]);
        c.extend((p + 0x660).to_le_bytes()); // mov [count], edx
        c.extend([0x8b, 0x01]); // mov eax, [ecx]
        c.extend([0xff, 0x05]);
        c.extend((p + 0x658).to_le_bytes()); // inc dword [calls]
        c.extend([0xc2, 0x08, 0x00]);
        c
    });
    let target = rng.below(n + 2);
    let random = rng.chance(4);
    for k in 0..n {
        let r: u32 = if random {
            [0, 1, u32::MAX, 0x8000_0000, 0x7fff_ffff][rng.below(5) as usize]
        } else if k < target {
            u32::MAX
        } else {
            u32::from(k != target)
        };
        e.w32(data + 8 * k, r);
    }
}

/// `initA` / `initB`: the renderer's device (`+0x6c`) with the four
/// creation methods (buffer 3, input layout 11, vertex shader 12, pixel
/// shader 15: results mostly success), the objects already in `this`'s
/// out words (null now and then), the constant buffer's `SetPrivateData`.
fn prep_init_shaders(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let this = e.pool + POOL_PTRS;
    a[0] = this;
    let renderer = e.pool + 0x100;
    a[1] = renderer;
    let device = e.pool + 0x200;
    e.w32(renderer + 0x6c, device);
    e.w32(device, device + 0x20);
    for (slot, pops) in [(11u32, 6u32), (12, 5)] {
        let r = match rng.below(16) {
            0 => 0x8007_0057,
            1 => rng.next(),
            k => k & 1,
        };
        let f = e.handler(pops, r);
        e.w32(device + 0x20 + 4 * slot, f);
    }
    let buffer = e.pool + 0x300;
    let b = if rng.chance(8) { 0 } else { buffer };
    // CreateBuffer (slot 3) and CreatePixelShader (15): one result and
    // one stored object per call, a failure now and then at any call
    for (k, slot, words) in [(0u32, 3u32, 4u8), (1, 15, 5)] {
        let (f, base) = view_creator(e, 0xf00d_8100 + k, words);
        e.w32(device + 0x20 + 4 * slot, f);
        e.w32(base, 0);
        for i in 0..24 {
            let r = match rng.below(40) {
                0 => 0x8007_000e,
                1 => rng.next() | 0x8000_0000,
                k => k & 1,
            };
            let obj = if rng.chance(40) { 0 } else { e.ptr(rng) };
            e.w32(base + 0x20 + 4 * i, r);
            e.w32(base + 0x80 + 4 * i, obj);
        }
        // the constant buffer: the 17th buffer of initA, the 1st of initB
        if slot == 3 {
            e.w32(base + 0x80 + 4 * 16, b);
            e.w32(base + 0x80, b);
        }
    }
    for k in 0..0x16 {
        let w = if rng.chance(30) { 0 } else { e.ptr(rng) };
        e.w32(this + 4 * k, w);
    }
    e.w32(buffer, buffer + 0x20);
    let named = e.handler(4, rng.next());
    e.w32(buffer + 0x20 + 20, named);
    let r = if rng.chance(10) { rng.next() | 1 } else { 0 };
    if e.cells.contains_key(&0x0058_d4e0) {
        e.set_ret(0x0058_d4e0, r);
    }
}

pub static CASES: &[Case] = &[
    Case {
        name: "PRendererD3D11_initA_00594f70",
        addr: 0x0059_4f70,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[],
        prep: prep_init_shaders,
        port: |g, a| {
            let frame = g.frame();
            h8::init_a(g, a[0], a[1], frame)
        },
        frame: 0x8c,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_initB_00595200",
        addr: 0x0059_5200,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[(0x0058_d4e0, 0, 0, true)],
        prep: prep_init_shaders,
        port: |g, a| {
            let frame = g.frame();
            h8::init_b(g, a[0], a[1], frame)
        },
        frame: 0x5c,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_initWindow_00595750",
        addr: 0x0059_5750,
        conv: K::T,
        kinds: "pn",
        out: O::U32,
        callees: &[(0x0059_5800, 6, 6, true)],
        prep: prep_init_window,
        port: |g, a| {
            let frame = g.frame();
            h8::init_window(g, a[0], a[1], frame)
        },
        frame: 0x10,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PEffect_findContextVariant",
        addr: 0x004e_f440,
        conv: K::T,
        kinds: "pvvvvv",
        out: O::U32,
        callees: &[(0x004e_f3f0, 6, 6, true)],
        prep: prep_find_variant,
        port: |g, a| {
            let frame = g.frame();
            let words = h8::variant_key_words(g);
            let ebp = frame + 0x14;
            let buf = (ebp - 0x20 - 4 * words) & !0xf;
            h8::find_context_variant(g, a[0], [a[1], a[2], a[3], a[4], a[5]], buf, frame)
        },
        frame: 0x10,
        gap: 4,
        writes: &[(0x004e_f3f0, 2)],
    },
    Case {
        name: "PRendererD3D11_ctor_00583580",
        addr: 0x0058_3580,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x004f_ee80, 0, 0, true),
            (0x004c_2f90, 0, 0, true),
            (0x0058_b070, 2, 0, false),
            (0x0094_964c, 3, 0, false),
            (0x004c_78a0, 1, 1, true),
            (0x005b_3580, 0, 0, true),
            (0x0094_966a, 5, 5, false),
            (0x005b_35c0, 2, 2, true),
            (0x0058_3320, 0, 0, true),
            (0x0058_3e20, 0, 0, true),
            (0x0058_33b0, 0, 0, true),
            (0x004b_e870, 0, 0, true),
            (0x0043_15e0, 0, 0, true),
            (0x0049_cc10, 0, 0, true),
            (0x0043_17f0, 0, 0, true),
        ],
        prep: prep_renderer_ctor,
        port: |g, a| h8::renderer_ctor(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRenderTargetD3D11_createViews_00591460",
        addr: 0x0059_1460,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x0058_fc20, 4, 4, true),
            (0x0059_03a0, 4, 4, true),
            (0x0059_0800, 4, 4, true),
            (0x0059_4060, 1, 1, true),
            (0x0059_3eb0, 1, 1, true),
            (0x0059_3f00, 1, 1, true),
            (0x0046_3d50, 0, 0, true),
            (0x0046_3e90, 0, 0, true),
            (0x0046_49d0, 0, 0, false),
            (0x0042_fb80, 2, 0, false),
            (0x0042_fb20, 1, 0, false),
        ],
        prep: prep_create_views,
        port: |g, a| {
            let frame = g.frame();
            h8::create_views(g, a[0], frame)
        },
        frame: 0x44,
        gap: 0xc,
        writes: &[],
    },
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
