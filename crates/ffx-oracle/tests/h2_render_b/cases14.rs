//! Last round batch 2 case (`ffx_prender::h2_more14`), written by hand:
//! `PSceneWideParameters::getBuffer`, the same call as its shim.

use super::*;

use ffx_prender::h2_more14::bridge as h14;

/// Where `prep_get_buffer` builds its objects (the pool's second half).
const G_THIS: u32 = POOL_PTRS;
const G_ENTRIES: u32 = POOL_PTRS + 0x400;
const G_CONSTANTS: u32 = POOL_PTRS + 0x600;
const G_BUFFERS: u32 = POOL_PTRS + 0x800;
const G_MAPPED: u32 = POOL_PTRS + 0x1400;
/// The key the key initializer's stand-in copies into its object.
const G_KEY: u32 = POOL_PTRS + 0x1600;
/// The stand-ins' cells: the key initializer's three arguments, the
/// buffer whose use count the `Sleep` stand-in counts down.
const G_KEY_ARGS: u32 = POOL_PTRS + 0x1fe0;
const G_WAITED: u32 = POOL_PTRS + 0x1ff0;

/// A float word for the key: small values, zeros, infinities and quiet
/// NaNs (no signalling ones).
fn key_float(rng: &mut Rng) -> u32 {
    match rng.below(8) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000][rng.below(4) as usize],
        1 => 0x7fc0_0000 | rng.below(0x40_0000) | (rng.below(2) << 31),
        _ => ((rng.below(4000) as f32 - 2000.0) / 7.0).to_bits(),
    }
}

/// The natural size of a value of type `kind` (what the parameter table
/// declares): matrices 64, vectors 16, pairs 8, scalars 4, the records
/// 16 or 20, `camera` blocks up to 64.
fn natural_size(rng: &mut Rng, kind: u32) -> u32 {
    match kind {
        0 | 1 | 0x12 | 0x13 => 16,
        2..=6 => 64,
        7 | 0xc..=0xf => 8,
        8..=0xb => 4,
        0x14 | 0x15 => 20,
        _ => 4 * rng.below(17),
    }
}

/// `getBuffer`'s state: the key initializer's stand-in (copies the
/// pool's key into its object and keeps its arguments), the `Sleep`
/// stand-in (counts the next buffer's use count down), the definitions
/// (every type, some past 0x1c; offsets under 4 or past the room now
/// and then), the buffers (stride, use counts), the current index and
/// the stored keys (the current one equal to the new key now and then),
/// the constant buffers' cursors and slot counts, the callees' results,
/// the one-time guard, the screen size (top bit set now and then) and
/// `camera`'s texture and sampler pointers.
fn prep_get_buffer(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let key = e.pool + G_KEY;
    let args = e.pool + G_KEY_ARGS;
    e.custom(0x004f_e940, |_| {
        let mut c = vec![];
        for k in 0..3u8 {
            c.extend([0x8b, 0x44, 0x24, 4 + 4 * k]); // mov eax, [esp+4+4k]
            c.push(0xa3);
            c.extend((args + 4 * u32::from(k)).to_le_bytes()); // mov [args+4k], eax
        }
        c.extend([0x56, 0x57]); // push esi; push edi
        c.push(0xbe);
        c.extend(key.to_le_bytes()); // mov esi, key
        c.extend([0x89, 0xcf]); // mov edi, ecx
        c.push(0xb9);
        c.extend(0x44u32.to_le_bytes()); // mov ecx, 0x44
        c.extend([0xf3, 0xa5]); // rep movsd
        c.extend([0x5f, 0x5e]); // pop edi; pop esi
        c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
        c
    });
    let waited = e.pool + G_WAITED;
    e.custom(0x0043_0030, |_| {
        let mut c = vec![0xa1];
        c.extend(waited.to_le_bytes()); // mov eax, [waited]
        c.extend([0xff, 0x48, 0x04]); // dec dword [eax+4]
        c.push(0xc3);
        c
    });
    for k in 0..0x44 {
        let v = key_float(rng);
        e.w32(key + 4 * k, v);
    }
    let this = e.pool + G_THIS;
    a[0] = this;
    let n = rng.below(7) | if rng.chance(4) { 0x8000_0000 } else { 0 };
    e.w32(this, n);
    let entries = e.pool + G_ENTRIES;
    e.w32(this + 4, entries);
    let stride = 0x150 + 4 * rng.below(0x40);
    e.write(this + 8, &(stride as u16).to_le_bytes());
    let buffers = e.pool + G_BUFFERS;
    e.w32(this + 0xc, buffers);
    let count = 1 + rng.below(3);
    e.w32(this + 0x10, count);
    let constants = e.pool + G_CONSTANTS;
    e.w32(this + 0x18, constants);
    let current = rng.below(count + 1);
    e.w32(this + 0x34, current);
    // the stored keys: the current one the new key now and then
    for k in 0..0x88 {
        let v = rng.next();
        e.w32(this + 0x38 + 4 * k, v);
    }
    if rng.chance(4) {
        let slot = this + 0x38 + (current & 1) * 0x110;
        for k in 0..0x44 {
            let v = e.r32(key + 4 * k);
            e.w32(slot + 4 * k, v);
        }
        if rng.chance(3) {
            // one word off
            let k = rng.below(0x44);
            let v = rng.next();
            e.w32(slot + 4 * k, v);
        }
    }
    let next = if current + 1 == count { 0 } else { current + 1 };
    let buffer = buffers + stride * next;
    e.w32(waited, buffer);
    // the index past the count (a current index at the count) included
    for k in 0..count + 2 {
        let b = buffers + stride * k;
        let uses = if rng.chance(3) { 1 + rng.below(3) } else { 0 };
        e.w32(b + 4, uses);
    }
    for k in 0..count + 2 {
        let c = constants + 0x3c * k;
        let cursor = rng.below(4);
        e.w32(c + 4, cursor);
        let slots =
            [0, 1, 2, 4][rng.below(4) as usize] | if rng.chance(4) { 0x8000_0000 } else { 0 };
        e.w32(c + 0xc, slots);
    }
    for k in 0..n & 0x7fff_ffff {
        let d = entries + 0x10 * k;
        let kind = rng.below(0x1f);
        e.write(d + 2, &[kind as u8]);
        let size = natural_size(rng, kind);
        let off = match rng.below(8) {
            0 => rng.below(4),
            1 => stride - size + 4 * rng.below(3),
            _ => 4 + 4 * rng.below((stride - size - 4) / 4),
        };
        e.write(d + 8, &(off as u16).to_le_bytes());
        let top = rng.below(8) << 13;
        e.write(d + 0xa, &((size | top) as u16).to_le_bytes());
    }
    let mapped = e.pool + G_MAPPED;
    e.set_out(0x004f_8460, mapped);
    for (f, err) in [(0x004f_8460, 5), (0x0059_d320, 5)] {
        let r = if rng.chance(err) {
            1 + rng.below(0xe)
        } else {
            0
        };
        e.set_ret(f, r);
    }
    let bound = if rng.chance(2) { 0x100 } else { rng.below(2) };
    e.set_ret(0x0059_5f70, bound);
    let guard = e.rebase(0x00c9_628c);
    let g = rng.next() & !1 | u32::from(rng.chance(2));
    e.w32(guard, g);
    for at in [0x00c9_5100, 0x00c9_5104] {
        let s = match rng.below(3) {
            0 => 0x8000_0000 | rng.next(),
            1 => rng.next(),
            _ => 320 + rng.below(3000),
        };
        let at = e.rebase(at);
        e.w32(at, s);
    }
    // `camera`'s blocks (up to +0x110) and pointers below the objects
    let camera = e.pool + 4 * rng.below(0x7b0);
    a[2] = camera;
    let texture = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(camera + 0xbc, texture);
    for off in [0xc8, 0xcc] {
        let s = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(camera + off, s);
    }
}

/// `getBuffer`'s port: its frame at the original's `ebp`.
fn get_buffer_port(g: &mut G, a: &[u32]) -> u32 {
    let ebp = g.frame() + h14::FRAME_BELOW;
    h14::get_buffer(g, a[0], a[1], a[2], a[3], ebp)
}

pub static CASES: &[Case] = &[Case {
    name: "Phyre_PRendering_PSceneWideParameters_getBuffer",
    addr: 0x004f_dec0,
    conv: K::T,
    kinds: "pvpf",
    out: O::U32,
    callees: &[
        (0x0043_1920, 0, 0, true),
        (0x0043_1a80, 0, 0, true),
        (0x004f_d2a0, 1, 1, true),
        (0x0043_52f0, 3, 0, false),
        (0x0045_1080, 1, 1, true),
        (0x004c_c0c0, 2, 0, false),
        (0x004c_9430, 1, 1, true),
        (0x004f_ee70, 0, 0, true),
        (0x0094_9207, 1, 0, false),
        (0x0059_5f70, 0, 0, true),
        (0x0058_cf20, 0, 0, true),
        (0x0058_44e0, 0, 0, true),
        (0x004f_8460, 1, 1, true),
        (0x004f_93f0, 2, 0, false),
        (0x0045_3340, 1, 1, true),
        (0x0059_d320, 2, 2, true),
        (0x0058_59f0, 0, 0, true),
    ],
    prep: prep_get_buffer,
    port: get_buffer_port,
    // what the callees see: the matrices and the key (`ebp - 0x490` up
    // to `ebp - 0x80`); the values copied out above are checked in the
    // buffer they are copied to
    frame: 0x410,
    gap: 0x80,
    writes: &[(0x004f_8460, 0)],
}];
