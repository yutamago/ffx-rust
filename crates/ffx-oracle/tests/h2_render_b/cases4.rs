//! Round 4 cases (`ffx_prender::h2_more5`), written by hand: one per
//! port, the same call as its shim, with the fix-ups random words cannot
//! give (COM objects with recording methods, rings that drain, trees that
//! end, strings).

use super::*;

use ffx_prender::h2_more5::bridge as h5;

/// `PhyreFree`.
const FREE: u32 = 0x0042_fb20;
const MEMSET: u32 = 0x0094_964c;
const MEMCPY: u32 = 0x0094_9256;
const PRINTF: u32 = 0x0043_52f0;
const SLEEP: u32 = 0x0043_0030;
const LOCK: u32 = 0x0046_3d50;
const UNLOCK: u32 = 0x0046_3e90;

fn le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// Words the custom stubs read, rewritten by each prep (the pool's last
/// 0x100 bytes; nothing else uses them).
fn scratch(e: &Env) -> u32 {
    e.pool + POOL - 0x100
}

/// Handler code made once per `tag` (kept with the writer handlers),
/// in both oracles at the same address.
fn once_code(e: &mut Env, tag: u32, make: impl FnOnce(&mut Env) -> Vec<u8>) -> u32 {
    if let Some(&(at, _)) = e.writers.get(&tag) {
        return at;
    }
    let c = make(e);
    let at = e.a.alloc_code(&c).unwrap();
    assert_eq!(e.b.alloc_code(&c).unwrap(), at);
    e.writers.insert(tag, (at, 0));
    at
}

/// A region of the pool's second half for case `k`'s objects (0x200
/// bytes each, k < 12), apart from the random pointers.
fn area(e: &Env, k: u32) -> u32 {
    e.pool + POOL_PTRS + 0x200 * k
}

/// A COM object at `obj` whose vtable (at `vt`) holds `slots` (word index,
/// code).
fn com_obj(e: &mut Env, obj: u32, vt: u32, slots: &[(u32, u32)]) {
    e.w32(obj, vt);
    for &(slot, f) in slots {
        e.w32(vt + 4 * slot, f);
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
    com_obj(e, obj, obj + 0x20, &[(2, f)]);
    obj
}

/// A COM result: success (0 or 1) or a failure code.
fn hresult(rng: &mut Rng) -> u32 {
    [0, 0, 1, 0x8000_4005][rng.below(4) as usize]
}

/// A `PSharray` count word at `array`: `n` elements (owned or not);
/// returns its storage (more than one: `data`, else inline or null).
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

/// `0x004f8460`: the ring at `+0xc` (one to four 16-byte entries), the
/// slot word `+4`, the next entry's busy count (-1..2) which the `Sleep`
/// stand-in counts down.
fn prep_ring(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let target = scratch(e);
    e.custom(SLEEP, |n| {
        let mut c = vec![0xa1];
        c.extend(le(target)); // mov eax, [target]
        c.extend([0xff, 0x48, 0x0c]); // dec dword [eax+0xc]
        c.extend([0xff, 0x05]);
        c.extend(le(n)); // inc dword [n]
        c.push(0xc3);
        c
    });
    a[0] = e.pool + 4 * rng.below(0x400);
    let n = 1 + rng.below(4);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + 0xc, n, data);
    let cur = rng.below(n + 1).wrapping_sub(1);
    e.w32(a[0] + 4, cur);
    let next = cur.wrapping_add(1);
    let slot = if next == n { 0 } else { next };
    let entry = s + 16 * slot;
    e.w32(entry + 0xc, rng.below(4).wrapping_sub(1));
    e.w32(target, entry);
}

/// `0x00596020`: the ring (none to four entries, some empty), the warning
/// bit, a slot around the count.
fn prep_ring_entry(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let n = rng.below(5);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + 0xc, n, data);
    for k in 0..n {
        let w = if rng.chance(3) { 0 } else { rng.next() };
        e.w32(s + 16 * k, w);
    }
    // an empty ring's slot 0 reads through null in the original too
    a[1] = if n == 0 {
        1 + rng.below(2)
    } else {
        rng.below(n + 2)
    };
    e.w32(a[0] + 0x2c, if rng.chance(2) { 0x1_0000 } else { 0 });
}

/// `0x0058fb90`: the `PSharray` at `+0x24` (one to three elements), the
/// texture's `GetDesc` (method 10).
fn prep_set_texture(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    // a byte argument: the words above it are the caller's junk
    if rng.chance(2) {
        a[3] |= rng.next() & 0xffff_ff00;
    }
    let n = 1 + rng.below(3);
    let data = area(e, 1);
    sharray(e, rng, a[0] + 0x24, n, data);
    if a[1] != 0 {
        a[1] = area(e, 2);
        let r = e.ret_word(rng);
        let f = e.handler(2, r);
        com_obj(e, a[1], a[1] + 0x40, &[(10, f)]);
    }
}

/// `0x004ff800`: the node's bounds pointer, null or not.
fn prep_link_bounds(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let src = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(a[1] + 0x1c, src);
}

/// `0x00593930`: the context's `Map` (method 14: a result, the mapped
/// pointer stored through its sixth word) and `Unmap` (15).
fn prep_upload(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[1] = area(e, 3);
    let out = if rng.chance(4) { 0 } else { e.ptr(rng) };
    let map = e.writer(0x40, 6, 5, hresult(rng), out);
    let r = e.ret_word(rng);
    let unmap = e.handler(3, r);
    com_obj(e, a[1], a[1] + 0x40, &[(14, map), (15, unmap)]);
    a[2] = rng.below(4);
    for off in [0x10, 0x14] {
        if rng.chance(3) {
            e.w32(a[0] + off, 0);
        }
    }
}

/// `0x004dd910`: a short name (or null), the lookup's result (0 or a
/// type), the two free lists (empty, refilled or not, or a node).
fn prep_pass_type(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let value = scratch(e);
    e.custom(0x0049_c8d0, |n| {
        // the refill stand-in: [this] = [value], counts the calls
        let mut c = vec![0xa1];
        c.extend(le(value)); // mov eax, [value]
        c.extend([0x89, 0x01]); // mov [ecx], eax
        c.extend([0xff, 0x05]);
        c.extend(le(n)); // inc dword [n]
        c.extend([0xc2, 0x04, 0x00]);
        c
    });
    let name = area(e, 0);
    let s: Vec<u8> = (0..rng.below(6))
        .map(|_| 1 + rng.below(0xff) as u8)
        .collect();
    e.string(name, &s);
    a[0] = if rng.chance(6) { 0 } else { name };
    if rng.chance(2) {
        e.set_ret(0x004d_d8c0, 0);
    }
    for head in [0x00ca_54fc, 0x00ca_5514] {
        let h = e.rebase(head);
        let v = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(h, v);
    }
    let v = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(value, v);
}

/// `0x0059c770`: the section's initialised byte, the sampler state.
fn prep_release_sampler(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let cs = e.rebase(0x00cb_3c94);
    let b = rng.below(2) as u8;
    e.write(cs, &[b]);
    let obj = area(e, 0);
    let v = releasable(e, rng, obj, 3);
    e.w32(a[0] + 0x20, v);
}

/// `ShaderParams_FindByName`: two arrays of named parameters (tagged
/// names among the shared strings), the name looked up.
fn prep_find_param(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let strings = area(e, 4);
    for (i, s) in NAMES.iter().enumerate() {
        e.string(strings + 0x20 * i as u32, s);
    }
    a[0] = e.pool + 4 * rng.below(0x400);
    for (k, off) in [0x20, 0x28].into_iter().enumerate() {
        let n = rng.below(4);
        e.w32(a[0] + off, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
        let data = area(e, k as u32);
        e.w32(a[0] + off + 4, data);
        for j in 0..n {
            let s = strings + 0x20 * rng.below(NAMES.len() as u32) + rng.below(2);
            e.w32(data + 0x10 * j + 4, s);
        }
    }
    let copy = strings + 0x100;
    let k = rng.below(NAMES.len() as u32) as usize;
    e.string(copy, NAMES[k]);
    a[1] = match rng.below(5) {
        0 => 0,
        1 => copy,
        _ => strings + 0x20 * rng.below(NAMES.len() as u32),
    };
}

/// `0x00593010`: eight render targets (or none), their flags, textures
/// with views or not, the context's `ResolveSubresource` (method 57).
fn prep_resolve(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    if !rng.chance(8) {
        e.set_ret(0x0059_1460, 0);
    }
    let ctx = area(e, 0);
    let r = e.ret_word(rng);
    let f = e.handler(6, r);
    com_obj(e, ctx, ctx + 0x20, &[(57, f)]);
    e.w32(a[0] + 0x70, ctx);
    for i in 0..8 {
        let ent = area(e, 2) + 0x20 * i;
        let v = if rng.chance(4) { 0 } else { ent };
        e.w32(a[0] + 0x228 + 0x10 * i, v);
        let flags = [u8::from(rng.chance(4)), rng.below(2) as u8];
        e.write(ent + 4, &flags);
        e.write(ent + 8, &[rng.below(4) as u8]);
        let tex = area(e, 3) + 0x40 * i;
        e.w32(ent, if rng.chance(6) { 0 } else { tex });
        let n = rng.below(3);
        let s = sharray(e, rng, tex + 0x24, n, area(e, 4) + 0x10 * i);
        if s != 0 {
            let w = if rng.chance(4) { 0 } else { rng.next() };
            e.w32(s + 4, w);
        }
    }
}

/// The shader binds: the device (feature level around 0xb000), its
/// context with the methods `slots` (word index, words popped), the
/// constant buffer slot (-1 or 0..7) and buffer (null or not).
fn prep_bind_shader(e: &mut Env, rng: &mut Rng, a: &mut [u32], slots: &[(u32, u32)]) {
    a[0] = e.pool + 4 * rng.below(0x200);
    let dev = area(e, 8);
    a[1] = dev + 0x1f0;
    e.w32(a[1], dev);
    e.w32(dev + 0xb4, 0xa000 + 0x800 * rng.below(4));
    let ctx = area(e, 9);
    e.w32(dev + 0x70, ctx);
    let vt = area(e, 10);
    e.w32(ctx, vt);
    for &(slot, pops) in slots {
        let r = e.ret_word(rng);
        let f = e.handler(pops, r);
        e.w32(vt + 4 * slot, f);
    }
    let slot = if rng.chance(3) {
        u32::MAX
    } else {
        rng.below(8)
    };
    e.w32(a[0] + 0x18, slot);
    if rng.chance(3) {
        e.w32(a[0], 0);
    }
}

/// `Renderer_SetAnisotropy`: the level (often the current one), the
/// sampler tree (0..4 nodes in order, their states released or not) and
/// the successor stand-in (`[node + 8]`).
fn prep_anisotropy(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    e.custom(0x0057_51d0, |n| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x40, 0x08]); // mov eax, [eax+8]
        c.extend([0xff, 0x05]);
        c.extend(le(n)); // inc dword [n]
        c.extend([0x89, 0x0d]);
        c.extend(le(n + 4)); // mov [n+4], ecx
        c.extend([0xc2, 0x04, 0x00]);
        c
    });
    a[0] = rng.below(20).wrapping_sub(4);
    let g = e.rebase(0x00c2_5370);
    let cur = if rng.chance(3) {
        (a[0] as i32 as f32).to_bits()
    } else {
        [0, 0x7fc0_0000, 0x8000_0000, (2.0f32).to_bits()][rng.below(4) as usize]
    };
    e.w32(g, cur);
    let head = e.rebase(0x00c2_5374);
    let k = rng.below(5);
    let nodes: Vec<u32> = (0..k).map(|i| area(e, 0) + 0x20 * i).collect();
    for (j, &node) in nodes.iter().enumerate() {
        let left = if j == 0 { head } else { nodes[j - 1] };
        e.w32(node, left);
        let next = nodes.get(j + 1).copied().unwrap_or(head);
        e.w32(node + 8, next);
        let s = area(e, 1) + 0x40 * j as u32;
        e.w32(node + 0xc, s);
        let obj = area(e, 2) + 0x40 * j as u32;
        let v = releasable(e, rng, obj, 3);
        e.w32(s + 0x20, v);
    }
    e.w32(head + 0xc, nodes.last().copied().unwrap_or(head));
}

/// `0x00599460`: the array (none to four elements, owned or not, its
/// storage apart), the new count (often the count word itself).
fn prep_resize16(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let n = rng.below(5);
    let data = area(e, 0);
    sharray(e, rng, a[0], n, data);
    a[1] = if rng.chance(4) {
        e.r32(a[0])
    } else {
        rng.below(6)
    };
    if rng.chance(4) {
        e.set_ret(0x0042_fb80, 0);
    }
}

/// `0x00565000`: the entries at `+0x14` (20 bytes; none to three, some
/// empty), the holder's guard, the render context's `Unmap`.
fn prep_unmap_entry(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    // at least one: the original reads an empty array's entry through null
    let n = 1 + rng.below(3);
    let data = area(e, 0);
    let s = sharray(e, rng, a[0] + 0x14, n, data);
    for k in 0..n {
        let w = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(s + 20 * k, w);
    }
    a[1] = rng.below(n + 1);
    holder_state(e, rng, &[(15, 3)]);
}

/// The holder's guard word (built or not) and its context (at
/// `0x00c94f70`) with the methods `slots`.
fn holder_state(e: &mut Env, rng: &mut Rng, slots: &[(u32, u32)]) {
    let g = e.rebase(0x00c9_628c);
    let v = rng.next();
    e.w32(g, v);
    let ctx = area(e, 5);
    let at = e.rebase(0x00c9_4f70);
    e.w32(at, ctx);
    e.w32(ctx, ctx + 0x40);
    for &(slot, pops) in slots {
        let r = e.ret_word(rng);
        let f = e.handler(pops, r);
        e.w32(ctx + 0x40 + 4 * slot, f);
    }
}

/// `0x00560e90`: the resource (or null) and its `GetDesc` stand-in
/// (method 10: the usage word from the scratch), the mode (often 2), the
/// render context's `Map` (14: a result, the pointer through word 5).
fn prep_map_resource(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let usage = scratch(e) + 4;
    let desc = once_code(e, 0xf00d_2000, |_| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
        c.extend([0x8b, 0x15]);
        c.extend(le(usage)); // mov edx, [usage]
        c.extend([0x89, 0x50, 0x04]); // mov [eax+4], edx
        c.extend([0xc2, 0x08, 0x00]);
        c
    });
    e.w32(usage, rng.below(4));
    if a[1] != 0 {
        a[1] = area(e, 6);
        com_obj(e, a[1], a[1] + 0x40, &[(10, desc)]);
    }
    a[2] = if rng.chance(2) { 2 } else { rng.below(5) };
    let g = e.rebase(0x00c9_628c);
    let v = rng.next();
    e.w32(g, v);
    let ctx = area(e, 5);
    let at = e.rebase(0x00c9_4f70);
    e.w32(at, ctx);
    let out = if rng.chance(4) { 0 } else { e.ptr(rng) };
    let map = e.writer(0x41, 6, 5, hresult(rng), out);
    com_obj(e, ctx, ctx + 0x40, &[(14, map)]);
}

/// `0x0059ac40`: the four shaders (null or objects), the state's device
/// and context with the four set methods.
fn prep_set_program(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    for k in 0..4 {
        let v = if rng.chance(3) {
            0
        } else {
            e.pool + 4 * rng.below(0x400)
        };
        e.w32(a[0] + 4 * k, v);
    }
    a[1] = area(e, 0);
    let dev = area(e, 1);
    e.w32(a[1], dev);
    let ctx = area(e, 2);
    e.w32(dev + 0x70, ctx);
    let mut slots = Vec::new();
    for slot in [11, 23, 9, 69] {
        let r = e.ret_word(rng);
        slots.push((slot, e.handler(4, r)));
    }
    com_obj(e, ctx, area(e, 3), &slots);
}

/// The table binds: the layout pointer (null or into the pool).
fn prep_bind_tables(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let l = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(a[1] + 0x14, l);
}

/// `PEffectVariant::createMaterial`: the instance lists (null or in
/// the pool).
/// The stand-in returns its two calls' lists from the scratch (`+0x14`,
/// `+0x18`, counted at `+0x10`) and logs `ecx` and the three words of
/// each at `+0x20`.
fn prep_create_material(e: &mut Env, rng: &mut Rng, _: &mut [u32]) {
    let t = scratch(e);
    e.custom(ALLOCATE, |_| {
        let mut c = vec![0xa1];
        c.extend(le(t + 0x10)); // mov eax, [t+0x10]
        c.extend([0xff, 0x05]);
        c.extend(le(t + 0x10)); // inc dword [t+0x10]
        c.extend([0xc1, 0xe0, 0x04]); // shl eax, 4
        c.extend([0x89, 0x88]);
        c.extend(le(t + 0x20)); // mov [eax+t+0x20], ecx
        for k in 0..3u8 {
            c.extend([0x8b, 0x54, 0x24, 4 + 4 * k]); // mov edx, [esp+4+4k]
            c.extend([0x89, 0x90]);
            c.extend(le(t + 0x24 + 4 * u32::from(k))); // mov [eax+t+0x24+4k], edx
        }
        c.extend([0xc1, 0xe8, 0x02]); // shr eax, 2
        c.extend([0x8b, 0x80]);
        c.extend(le(t + 0x14)); // mov eax, [eax+t+0x14]
        c.extend([0xc2, 0x0c, 0x00]);
        c
    });
    e.w32(t + 0x10, 0);
    for k in 0..2 {
        let l = if rng.chance(4) { 0 } else { e.ptr(rng) };
        e.w32(t + 0x14 + 4 * k, l);
    }
}

const ALLOCATE: u32 = 0x0044_0b70;

/// `0x0059c540`: the entry count (0..4), the array (at least one
/// element), four entries with objects (or none) and buffers.
fn prep_release_entries(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    let count = rng.below(5);
    e.set_ret(0x004f_7d50, count);
    let n = 1 + rng.below(4);
    let s = sharray(e, rng, a[0] + 0xc, n, area(e, 0));
    for k in 0..4 {
        let obj = area(e, 1) + 0x40 * k;
        let v = releasable(e, rng, obj, 3);
        e.w32(s + 16 * k, v);
        let p = if rng.chance(3) { 0 } else { e.ptr(rng) };
        e.w32(s + 16 * k + 4, p);
    }
}

/// `0x00595620`: the device's `CreateBuffer` (method 3), sizes and bind
/// words 0 or not.
fn prep_create_buffers(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[1] = area(e, 0);
    let f = e.handler(4, hresult(rng));
    com_obj(e, a[1], a[1] + 0x40, &[(3, f)]);
    for k in [2, 3, 5] {
        if rng.chance(3) {
            a[k] = 0;
        }
    }
}

/// `0x0059c650`: the two view arrays (none to three, objects or
/// null), the two objects, the texture and its flag.
fn prep_release_views2(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x400);
    for (k, off) in [0x18u32, 0x20].into_iter().enumerate() {
        let n = rng.below(4);
        e.w32(a[0] + off, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
        let data = if n == 0 && rng.chance(2) {
            0
        } else {
            area(e, k as u32)
        };
        e.w32(a[0] + off + 4, data);
        for j in 0..n {
            let obj = area(e, 2 + k as u32) + 0x40 * j;
            let v = releasable(e, rng, obj, 3);
            e.w32(data + 4 * j, v);
        }
    }
    for (k, off) in [0x10u32, 0x14].into_iter().enumerate() {
        let obj = area(e, 4) + 0x40 * k as u32;
        let v = releasable(e, rng, obj, 3);
        e.w32(a[0] + off, v);
    }
    e.write(a[0] + 4, &[u8::from(rng.chance(3))]);
    let t = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.w32(a[0], t);
}

/// The constant-table layouts of `0x0059ab50` (first, count, base).
const TABLES_A: [(u32, u32, u32); 4] = [
    (0x194, 0x19c, 0x1a8),
    (0x1ac, 0x1b4, 0x1c0),
    (0x1c4, 0x1cc, 0x1d8),
    (0x1dc, 0x1e4, 0x1f0),
];
/// The same of `0x0059b010`.
const TABLES_B: [(u32, u32, u32); 4] = [
    (0x1f4, 0x1fc, 0x208),
    (0x20c, 0x214, 0x220),
    (0x224, 0x22c, 0x238),
    (0x23c, 0x244, 0x250),
];

/// The scoped lock, the holder's constructor and `atexit`.
const LOCKS: [(u32, u32, u32, bool); 4] = [
    (LOCK, 0, 0, true),
    (UNLOCK, 0, 0, true),
    (0x004f_ee70, 0, 0, true),
    (0x0094_9207, 1, 0, false),
];

pub const CASES: &[Case] = &[
    Case {
        name: "Phyre_DeleteArray4c_00540b80",
        addr: 0x0054_0b80,
        conv: K::C,
        kinds: "nc",
        out: O::None,
        callees: &[
            (0x0052_52c0, 0, 0, true),
            (0x0052_3240, 0, 0, true),
            (0x0052_2600, 0, 0, true),
            (FREE, 1, 0, false),
        ],
        prep: no_prep,
        port: |g, a| {
            h5::delete_array(
                g,
                a[0],
                a[1],
                0x4c,
                &[(0x38, 0x0052_52c0), (0x24, 0x0052_3240), (0xc, 0x0052_2600)],
            );
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_DeleteStrings4_004dd770",
        addr: 0x004d_d770,
        conv: K::C,
        kinds: "nc",
        out: O::None,
        callees: &[(FREE, 1, 0, false)],
        prep: no_prep,
        port: |g, a| {
            h5::delete_strings4(g, a[0], a[1]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_DestructArrayName_0056cab0",
        addr: 0x0056_cab0,
        conv: K::T,
        kinds: "vp",
        out: O::U8,
        callees: &[(FREE, 1, 0, false)],
        prep: no_prep,
        port: |g, a| u32::from(h5::destruct_array_name(g, a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_DestructInstance_00562410",
        addr: 0x0056_2410,
        conv: K::T,
        kinds: "vp",
        out: O::U8,
        callees: &[
            (0x0055_e060, 0, 0, true),
            (0x0043_3900, 0, 0, true),
            (FREE, 1, 0, false),
        ],
        prep: no_prep,
        port: |g, a| u32::from(h5::destruct_562410(g, a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PTexture2DD3D11_setTexture_0058fb90",
        addr: 0x0058_fb90,
        conv: K::T,
        kinds: "pnvbv",
        out: O::U32,
        callees: &[(0x0058_f710, 0, 0, true)],
        prep: prep_set_texture,
        port: |g, a| h5::set_texture(g, a[0], a[1], a[2], a[3]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PMeshInstanceBounds_link_004ff800",
        addr: 0x004f_f800,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[],
        prep: prep_link_bounds,
        port: |g, a| h5::link_bounds(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PConstantBufferRing_next_004f8460",
        addr: 0x004f_8460,
        conv: K::T,
        kinds: "pv",
        out: O::U32,
        callees: &[
            (0x0059_5f70, 0, 0, true),
            (0x0058_cf20, 0, 0, true),
            (0x0059_6020, 2, 2, true),
        ],
        prep: prep_ring,
        port: |g, a| h5::next_ring_slot(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PConstantBufferRing_entry_00596020",
        addr: 0x0059_6020,
        conv: K::T,
        kinds: "pip",
        out: O::U32,
        callees: &[(PRINTF, 2, 0, false)],
        prep: prep_ring_entry,
        port: |g, a| h5::ring_entry(g, a[0], a[1], a[2]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PEffectVariant_createMaterialWithPreallocatedBuffer",
        addr: 0x004e_6b40,
        conv: K::T,
        kinds: "pnn",
        out: O::U32,
        callees: &[
            (MEMSET, 3, 0, false),
            (0x004d_6180, 1, 1, true),
            (0x004d_7b10, 2, 2, true),
        ],
        prep: no_prep,
        port: |g, a| h5::create_material_prealloc(g, a[0], a[1], a[2]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PEffectVariant_createMaterial",
        addr: 0x004e_6a40,
        conv: K::T,
        kinds: "ppnn",
        out: O::U32,
        callees: &[
            (MEMSET, 3, 0, false),
            (0x004d_6180, 1, 1, true),
            (0x004d_7b10, 2, 2, true),
            (0x0044_1240, 1, 1, true),
        ],
        prep: prep_create_material,
        port: |g, a| h5::create_material(g, a[0], a[1], a[2], a[3]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PConstantBufferD3D11_upload_00593930",
        addr: 0x0059_3930,
        conv: K::T,
        kinds: "ppi",
        out: O::U32,
        callees: &[(MEMCPY, 3, 0, false)],
        prep: prep_upload,
        port: |g, a| {
            let f = g.frame();
            h5::upload_constants(g, a[0], a[1], a[2], f)
        },
        frame: 0xc,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRenderPassType_findOrAdd_004dd910",
        addr: 0x004d_d910,
        conv: K::C,
        kinds: "n",
        out: O::U32,
        callees: &[
            (0x004d_d8c0, 1, 0, false),
            (PRINTF, 3, 0, false),
            (MEMCPY, 3, 0, false),
            (0x004d_b280, 2, 2, true),
        ],
        prep: prep_pass_type,
        port: |g, a| h5::find_or_add_pass_type(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PSamplerStateD3D11_release_0059c770",
        addr: 0x0059_c770,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x0043_17f0, 0, 0, true),
            (0x0043_1840, 0, 0, true),
            (0x0058_1670, 1, 1, true),
            (0x0057_5000, 1, 1, true),
            (0x0049_c980, 1, 1, true),
            (0x0043_19d0, 0, 0, true),
        ],
        prep: prep_release_sampler,
        port: |g, a| {
            let f = g.frame();
            h5::release_sampler(g, a[0], f)
        },
        frame: 8,
        gap: 0xc,
        writes: &[],
    },
    Case {
        name: "ShaderParams_FindByName",
        addr: 0x0056_cbf0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[],
        prep: prep_find_param,
        port: |g, a| h5::find_param(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_resolveTargets_00593010",
        addr: 0x0059_3010,
        conv: K::T,
        kinds: "p",
        out: O::U8,
        callees: &[
            (0x0059_1460, 0, 0, true),
            (0x0059_4060, 1, 1, true),
            (0x0059_3e80, 1, 1, true),
        ],
        prep: prep_resolve,
        port: |g, a| u32::from(h5::resolve_targets(g, a[0])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderD3D11_bindPixel_005932b0",
        addr: 0x0059_32b0,
        conv: K::T,
        kinds: "ppv",
        out: O::U8,
        callees: &[(0x0058_dbe0, 3, 3, true)],
        prep: |e, r, a| prep_bind_shader(e, r, a, &[(48, 7), (9, 4), (16, 4), (8, 4), (10, 4)]),
        port: |g, a| u32::from(h5::bind_pixel_shader(g, a[0], a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderD3D11_bindVertex_00593510",
        addr: 0x0059_3510,
        conv: K::T,
        kinds: "ppv",
        out: O::U8,
        callees: &[],
        prep: |e, r, a| {
            prep_bind_shader(
                e,
                r,
                a,
                &[(68, 5), (48, 7), (11, 4), (7, 4), (25, 4), (26, 4)],
            );
        },
        port: |g, a| {
            let f = g.frame();
            u32::from(h5::bind_vertex_shader(g, a[0], a[1], f))
        },
        frame: 0x20,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRenderStateBlock_defaults_00582f50",
        addr: 0x0058_2f50,
        conv: K::T,
        kinds: "pv",
        out: O::U32,
        callees: &[],
        prep: no_prep,
        port: |g, a| {
            let f = g.frame();
            h5::default_blocks(g, a[0], f)
        },
        frame: 0x20,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Renderer_SetAnisotropy",
        addr: 0x0058_de30,
        conv: K::C,
        kinds: "v",
        out: O::U32,
        callees: &[(0x0059_5bf0, 0, 0, true)],
        prep: prep_anisotropy,
        port: |g, a| h5::set_anisotropy(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PSharray16_resize_00599460",
        addr: 0x0059_9460,
        conv: K::T,
        kinds: "pv",
        out: O::U32,
        callees: &[
            (0x0042_fb80, 2, 0, false),
            (0x0049_f450, 3, 0, false),
            (0x0058_af90, 2, 0, false),
            (FREE, 1, 0, false),
        ],
        prep: prep_resize16,
        port: |g, a| h5::resize16(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Destroy_004db9d0",
        addr: 0x004d_b9d0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x0043_4360, 2, 0, false),
            (FREE, 1, 0, false),
            (0x004d_d770, 2, 0, false),
            (0x004d_b710, 0, 0, true),
        ],
        prep: no_prep,
        port: |g, a| h5::destroy_4db9d0(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PConstantBufferD3D11_unmapEntry_00565000",
        addr: 0x0056_5000,
        conv: K::T,
        kinds: "piv",
        out: O::U32,
        callees: &LOCKS,
        prep: prep_unmap_entry,
        // the lock is the original's argument slot `ebp + 8`, above the
        // compared word (`frame` 4 at `ebp - 0x104`, below the callees'
        // pushes)
        port: |g, a| {
            let lock = g.frame() + 4 + 0x100 + 8;
            h5::unmap_entry(g, a[0], a[1], lock)
        },
        frame: 4,
        gap: 0x100,
        writes: &[],
    },
    Case {
        name: "PRenderThread_mapResource_00560e90",
        addr: 0x0056_0e90,
        conv: K::C,
        kinds: "pnv",
        out: O::U32,
        callees: &LOCKS,
        prep: prep_map_resource,
        port: |g, a| {
            let f = g.frame();
            h5::map_resource(g, a[0], a[1], a[2], f)
        },
        frame: 0x28,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PRenderThread_mapResource_00560f90",
        addr: 0x0056_0f90,
        conv: K::C,
        kinds: "pnv",
        out: O::U32,
        callees: &LOCKS,
        prep: prep_map_resource,
        port: |g, a| {
            let f = g.frame();
            h5::map_resource(g, a[0], a[1], a[2], f)
        },
        frame: 0x28,
        gap: 0x10,
        writes: &[],
    },
    Case {
        name: "PShaderProgramD3D11_set_0059ac40",
        addr: 0x0059_ac40,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            (0x0058_d780, 0, 0, true),
            (0x0058_d350, 0, 0, true),
            (0x0058_d1c0, 0, 0, true),
            (0x0058_d030, 0, 0, true),
        ],
        prep: prep_set_program,
        port: |g, a| h5::set_program(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderProgramD3D11_bindTables_0059ab50",
        addr: 0x0059_ab50,
        conv: K::S,
        kinds: "ppi",
        out: O::U32,
        callees: &[(0x0059_a650, 5, 5, true)],
        prep: prep_bind_tables,
        port: |g, a| h5::bind_tables(g, a[0], a[1], a[2], 0x0059_a650, false, &TABLES_A),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderProgramD3D11_bindTextures_0059b010",
        addr: 0x0059_b010,
        conv: K::S,
        kinds: "ppi",
        out: O::U32,
        callees: &[(0x0059_b3d0, 6, 6, true)],
        prep: prep_bind_tables,
        port: |g, a| h5::bind_tables(g, a[0], a[1], a[2], 0x0059_b3d0, true, &TABLES_B),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_00567110",
        addr: 0x0056_7110,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0043_37d0, 0, 0, true), (0x0058_3b20, 0, 0, true)],
        prep: no_prep,
        port: |g, a| h5::ctor_567110(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PConstantBufferD3D11_releaseEntries_0059c540",
        addr: 0x0059_c540,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x004f_7d50, 0, 0, true), (FREE, 1, 0, false)],
        prep: prep_release_entries,
        port: |g, a| h5::release_entries(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Destroy_004e39b0",
        addr: 0x004e_39b0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[
            (0x004a_b4d0, 2, 0, false),
            (FREE, 1, 0, false),
            (0x004e_3830, 0, 0, true),
            (0x004e_4df0, 2, 0, false),
        ],
        prep: no_prep,
        port: |g, a| {
            h5::destroy_4e39b0(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_createBuffers_00595620",
        addr: 0x0059_5620,
        conv: K::T,
        kinds: "ppvvvbv",
        out: O::U32,
        callees: &[],
        prep: prep_create_buffers,
        port: |g, a| {
            let f = g.frame();
            h5::create_buffers(g, a[0], a[1], a[2], a[3], a[4], a[5], a[6], f)
        },
        frame: 0x30,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PRenderTargetD3D11_releaseViews_0059c650",
        addr: 0x0059_c650,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0059_cc40, 0, 0, true), (FREE, 1, 0, false)],
        prep: prep_release_views2,
        port: |g, a| h5::release_views2(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
];
