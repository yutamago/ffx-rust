//! Round 3 cases (`ffx_prender::h2_more`), written by hand: one per port,
//! the same call as its shim, with the fix-ups random words cannot give
//! (lists that end, trees, counters that run down).

use super::*;

/// Image-address word helper for code bytes.
fn le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// `Sleep(1)` stand-in for the spin waits: counts down the counter the
/// getter `get`'s stub points at (its cell), counts the calls and logs
/// the argument.
fn sleep_stub(e: &mut Env, get: u32) {
    let cell = e.cell(get);
    e.custom(0x0043_0030, |n| {
        let mut c = vec![0xa1];
        c.extend(le(cell)); // mov eax, [cell]
        c.extend([0xff, 0x08]); // dec dword [eax]
        c.extend([0xff, 0x05]);
        c.extend(le(n)); // inc dword [n]
        c.extend([0x8b, 0x54, 0x24, 0x04]); // mov edx, [esp+4]
        c.extend([0x89, 0x15]);
        c.extend(le(n + 4)); // mov [n+4], edx
        c.push(0xc3);
        c
    });
}

/// A spin wait's array count at `this + off` (0..3, owned or not) and
/// the counter its getter returns (-2..3).
fn prep_spin(e: &mut Env, rng: &mut Rng, a: &mut [u32], off: u32, get: u32) {
    sleep_stub(e, get);
    let n = rng.below(4) | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + off, n);
    let counter = e.pool + POOL_PTRS + 0x40;
    e.set_ret(get, counter);
    e.w32(counter, rng.below(6).wrapping_sub(2));
}

/// A function-local static's guard word at `guard`: bit 0 set or not.
fn prep_guard(e: &mut Env, rng: &mut Rng, guard: u32) {
    let g = e.rebase(guard);
    let v = rng.next();
    e.w32(g, v);
}

/// `0x00505010`: a list of 0..4 nodes at `this + 8` (apart from `this`).
fn prep_detach(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let n = rng.below(5);
    let nodes: Vec<u32> = (0..n)
        .map(|i| e.pool + POOL_PTRS + 0xa00 + 0x10 * i)
        .collect();
    for (k, &node) in nodes.iter().enumerate() {
        let next = nodes.get(k + 1).copied().unwrap_or(0);
        e.w32(node, next);
    }
    e.w32(a[0] + 8, nodes.first().copied().unwrap_or(0));
}

/// `0x0056e670`: the type byte (a texture type or not) and the
/// texture's words.
fn prep_tex_type(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let types = [0x42u8, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0];
    let t = types[rng.below(8) as usize];
    e.write(a[1] + 2, &[t]);
    if a[2] != 0 {
        for off in [8, 0xc] {
            let v = if rng.chance(3) { 0 } else { e.ptr(rng) };
            e.w32(a[2] + off, v);
        }
    }
}

/// `0x00595ee0`: the free list at `this + 0x18` (a node or empty), the
/// block allocator storing a node (or null) through `this + 0x20`.
fn prep_tree_add(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let node = e.pool + POOL_PTRS + 0xb00;
    let head = if rng.chance(2) { node } else { 0 };
    e.w32(a[0] + 0x18, head);
    e.w32(a[0] + 0x20, a[0] + 0x18);
    let fresh = if rng.chance(3) { 0 } else { node };
    e.set_out(0x0049_c8d0, fresh);
}

/// `PWorldCursor_skipEmpty`: the next-list stand-in follows `+0x20`;
/// one to three lists with 0..3 slots, the first few of them free.
fn prep_skip_empty(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    e.custom(0x0044_3ef0, |n| {
        let mut c = vec![0x8b, 0x41, 0x08]; // mov eax, [ecx+8]
        c.extend([0x85, 0xc0, 0x74, 0x06]); // test eax, eax; je +6
        c.extend([0x8b, 0x40, 0x20]); // mov eax, [eax+0x20]
        c.extend([0x89, 0x41, 0x08]); // mov [ecx+8], eax
        c.extend([0xff, 0x05]);
        c.extend(le(n)); // inc dword [n]
        c.push(0xc3);
        c
    });
    let c = a[0];
    let base = e.pool + POOL_PTRS + 0xc00;
    let lists = 1 + rng.below(3);
    let mut prev = c + 8 - 0x20;
    for l in 0..=lists {
        let list = base + 0x100 * l + 0x20;
        e.w32(prev + 0x20, list);
        prev = list;
        let class = list + 0x30;
        let stride = 4 * (1 + rng.below(3));
        e.w32(list + 0xc, class);
        e.w32(class + 0x1c, stride);
        let w = rng.next();
        e.w32(class + 0x8c, w);
        let slots = base + 0x500 + 0x40 * l;
        e.w32(list + 0x14, slots);
        let n = rng.below(4);
        e.w32(list + 0x1c, n);
        let free = rng.below(n + 1);
        for i in 0..free {
            let next = if i + 1 < free {
                slots + stride * (i + 1)
            } else {
                e.ptr(rng)
            };
            e.w32(slots + stride * i, next);
        }
        let head = if free > 0 { slots } else { e.ptr(rng) | 2 };
        e.w32(list - 0x18, head);
    }
    e.w32(prev + 0x20, 0);
    let left = if rng.chance(4) { 1 + rng.below(3) } else { 0 };
    e.w32(c + 0x18, left);
}

/// `0x0058aa10`: the anisotropy float; the flag byte's word sometimes
/// set above its low byte only.
fn prep_aniso(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    if rng.chance(3) {
        a[2] = rng.next() & !0xff;
    }
    let g = e.rebase(0x00c2_5370);
    let v: u32 = [
        0x3f80_0000,
        0x4000_0000,
        0x3f00_0000,
        0x7fc0_0000,
        0x4180_0000,
        0x3f80_0001,
    ][rng.below(6) as usize];
    e.w32(g, v);
}

/// A callee's result that the original dereferences: null or a pointer.
fn prep_ret_ptr(e: &mut Env, rng: &mut Rng, sym: u32) {
    let p = if rng.chance(3) { 0 } else { e.ptr(rng) };
    e.set_ret(sym, p);
}

/// `0x0059c9c0`: the program's COM objects at `+0x4ec`, `+0`, `+0x4e8`
/// (null or objects whose `Release` pops one word) and its byte array.
fn prep_release_program(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x380);
    for (k, off) in [0x4ec, 0, 0x4e8].into_iter().enumerate() {
        let obj = if rng.chance(3) {
            0
        } else {
            e.pool + POOL_PTRS + 0x100 * k as u32
        };
        e.w32(a[0] + off, obj);
        if obj != 0 {
            let vt = obj + 0x80;
            let r = e.ret_word(rng);
            let f = e.handler(1, r);
            e.w32(obj, vt);
            e.w32(vt + 8, f);
        }
    }
    let n =
        if rng.chance(3) { 0 } else { rng.below(4) } | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0] + 4, n);
    let data = if rng.chance(4) { 0 } else { e.ptr(rng) };
    e.w32(a[0] + 8, data);
}

/// `PArray<u32>::resize`: the count word (sometimes the new count), the
/// storage and the allocator's result (null or storage apart).
fn prep_resize(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let old = rng.below(9) | if rng.chance(3) { 0x8000_0000 } else { 0 };
    e.w32(a[0], old);
    if rng.chance(4) {
        a[1] = old;
    }
    let data = e.pool + POOL_PTRS + 0xd00;
    if rng.chance(4) {
        e.w32(a[0] + 4, data);
    }
    let r = if rng.chance(4) { 0 } else { data };
    e.set_ret(0x0042_fb80, r);
}

/// `PMeshInstance::computeJointMatrices`: the joint above its matrix.
fn prep_joint(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[1] = e.ptr(rng) + 0x40;
    let f = rng.below(4) as u8;
    e.write(a[0] + 0x6c, &[f]);
    if rng.chance(4) {
        e.w32(a[0] + 4, 0);
    }
}

/// `0x005817d0`: a tree ([`prep_tree`]) and a node to insert, its key
/// in the tree or not.
fn prep_tree_insert(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    prep_tree(e, rng, a);
    let node = a[0] + 0x20 * 8;
    let k = rng.below(17);
    e.w32(node + 0xc, k);
    let w = rng.next();
    e.w32(node + 8, w);
    a[1] = node;
}

pub const CASES: &[Case] = &[
    Case {
        name: "Phyre_SpinWaitCounters18_00564630",
        addr: 0x0056_4630,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0043_4870, 1, 1, true)],
        prep: |e, r, a| prep_spin(e, r, a, 0x18, 0x0043_4870),
        port: |g, a| hm::spin_wait(g, a[0], 0x18, 0x0043_4870),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_SpinWaitCounters14_00564680",
        addr: 0x0056_4680,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0043_4910, 1, 1, true)],
        prep: |e, r, a| prep_spin(e, r, a, 0x14, 0x0043_4910),
        port: |g, a| hm::spin_wait(g, a[0], 0x14, 0x0043_4910),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PClassDescriptorConcrete_Phyre_PAnimation_PBlendableAnimationSource__destructInstance",
        addr: 0x0054_2420,
        conv: K::T,
        kinds: "vp",
        out: O::U8,
        callees: &[
            (0x0052_52c0, 0, 0, true),
            (0x0052_3240, 0, 0, true),
            (0x0052_2600, 0, 0, true),
        ],
        prep: no_prep,
        port: |g, a| {
            u32::from(hm::destruct_members(
                g,
                a[1],
                &[(0x38, 0x0052_52c0), (0x24, 0x0052_3240), (0xc, 0x0052_2600)],
            ))
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PInstanceList_PInstanceList",
        addr: 0x0057_6270,
        conv: K::T,
        kinds: "ppv",
        out: O::U32,
        callees: &[(0x0049_c480, 4, 4, true)],
        prep: no_prep,
        port: |g, a| hm::instance_list_ctor(g, a[0], a[1], a[2]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_DetachAll_00505010",
        addr: 0x0050_5010,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0050_7360, 1, 1, true), (0x0042_fb20, 1, 0, false)],
        prep: prep_detach,
        port: |g, a| {
            hm::detach_all(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_ConstructWithType_004f76a0",
        addr: 0x004f_76a0,
        conv: K::T,
        kinds: "vn",
        out: O::U8,
        callees: &[(0x0043_cbb0, 0, 0, true), (0x004f_27e0, 1, 1, true)],
        prep: no_prep,
        port: |g, a| u32::from(hm::construct_with_type(g, a[1], 0x00c9_9c80, 0x004f_27e0)),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PParameterBuffer_setValueDesc_0056e600",
        addr: 0x0056_e600,
        conv: K::T,
        kinds: "ppv",
        out: O::U32,
        callees: &[(0x0094_9256, 3, 0, false)],
        prep: prep_set_value,
        port: |g, a| {
            let f = g.frame();
            hm::set_value_local(g, a[0], a[1], a[2], f)
        },
        frame: 0x10,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PParameterBuffer_setTextureChecked_0056e670",
        addr: 0x0056_e670,
        conv: K::T,
        kinds: "ppn",
        out: O::U32,
        callees: &[(0x0059_e520, 1, 1, true), (0x004d_57d0, 2, 2, true)],
        prep: prep_tex_type,
        port: |g, a| hm::set_texture_checked(g, a[0], a[1], a[2]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_StaticCriticalSection_initialize_004ff020",
        addr: 0x004f_f020,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_15e0, 0, 0, true),
            (0x0094_9207, 1, 0, false),
            (0x0043_17f0, 0, 0, true),
        ],
        prep: |e, r, _| prep_guard(e, r, 0x00c9_4efc),
        port: |g, _| {
            hm::static_once_then(
                g,
                0x00c9_4efc,
                0x00c9_4ee4,
                0x0043_15e0,
                0x00af_b700,
                0x0043_17f0,
            )
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRedBlackTree_addKey_00595ee0",
        addr: 0x0059_5ee0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            (0x0049_c8d0, 1, 1, true),
            (0x0058_17d0, 1, 1, true),
            (0x0049_c980, 1, 1, true),
            (0x0057_4980, 1, 1, true),
        ],
        prep: prep_tree_add,
        port: |g, a| hm::tree_add(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[(0x0049_c8d0, 0)],
    },
    Case {
        name: "Phyre_ZeroInit16_004e9f70",
        addr: 0x004e_9f70,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[],
        prep: no_prep,
        port: |g, a| x::zero_init(g, a[0], 16, None),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_ConstructArray3c_004fd960",
        addr: 0x004f_d960,
        conv: K::C,
        kinds: "nc",
        out: O::None,
        callees: &[(0x0058_3240, 0, 0, true)],
        prep: no_prep,
        port: |g, a| {
            hm::for_each(g, a[0], a[1], 0x3c, 0x0058_3240);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PWorldCursor_skipEmpty_005009f0",
        addr: 0x0050_09f0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[],
        prep: prep_skip_empty,
        port: |g, a| {
            hm::skip_empty(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_mapSamplerFilter_0058aa10",
        addr: 0x0058_aa10,
        conv: K::C,
        kinds: "inv",
        out: O::U32,
        callees: &[],
        prep: prep_aniso,
        port: |g, a| hm::filter(g, a[0], a[1], a[2]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PCluster_allocateMeshInstance_00566680",
        addr: 0x0056_6680,
        conv: K::T,
        kinds: "pv",
        out: O::U32,
        callees: &[(0x0044_0b70, 3, 3, true), (0x004f_27e0, 1, 1, true)],
        prep: |e, r, _| prep_ret_ptr(e, r, 0x0044_0b70),
        port: |g, a| hm::instance_list_call(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_00583320",
        addr: 0x0058_3320,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0058_3e20, 0, 0, true)],
        prep: no_prep,
        port: |g, a| hm::ctor_583320(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_SetColor2a8_0059a200",
        addr: 0x0059_a200,
        conv: K::T,
        kinds: "pn",
        out: O::U32,
        callees: &[(0x005b_3620, 0, 0, true), (0x005b_3640, 0, 0, true)],
        prep: |e, r, a| {
            let p = e.ptr(r);
            e.set_ret(0x005b_3620, p);
            // `this + 0x2b4` inside the pool
            a[0] = e.pool + 4 * r.below(0x400);
        },
        port: |g, a| {
            let f = g.frame();
            hm::set_color(g, a[0], a[1], f)
        },
        frame: 0x10,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderProgramD3D11_release_0059c9c0",
        addr: 0x0059_c9c0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0042_fb20, 1, 0, false)],
        prep: prep_release_program,
        port: |g, a| {
            hm::release_program(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PArrayU32_resize_004f8880",
        addr: 0x004f_8880,
        conv: K::T,
        kinds: "pc",
        out: O::U32,
        callees: &[(0x0042_fb80, 2, 0, false), (0x0042_fb20, 1, 0, false)],
        prep: prep_resize,
        port: |g, a| hm::resize_words(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PWorldCursor_MeshInstanceBounds_004ff6e0",
        addr: 0x004f_f6e0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            (0x0044_37e0, 1, 1, true),
            (0x0044_3ef0, 0, 0, true),
            (0x0044_1dc0, 1, 1, true),
            (0x0050_09f0, 0, 0, true),
        ],
        prep: |e, r, _| prep_ret_ptr(e, r, 0x0044_37e0),
        port: |g, a| hm::bounds_cursor(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PTexture2DD3D11_PTexture2DD3D11",
        addr: 0x0058_4240,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x004c_2f90, 0, 0, true),
            (0x0058_b070, 2, 0, false),
            (0x0094_964c, 3, 0, false),
        ],
        prep: no_prep,
        port: |g, a| hm::texture2d_ctor(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PMeshInstance_computeJointMatrices",
        addr: 0x004f_6ef0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[
            (0x0045_1080, 1, 1, true),
            (0x0045_3340, 1, 1, true),
            (0x004f_6fc0, 1, 1, true),
        ],
        prep: prep_joint,
        port: |g, a| {
            let f = g.frame();
            hm::compute_joint_matrices(g, a[0], a[1], f)
        },
        frame: 0x80,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRedBlackTree_insertNode_005817d0",
        addr: 0x0058_17d0,
        conv: K::T,
        kinds: "pp",
        out: O::U32,
        callees: &[],
        prep: prep_tree_insert,
        port: |g, a| hm::tree_insert(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_0055dd30",
        addr: 0x0055_dd30,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[],
        prep: no_prep,
        port: |g, a| hm::ctor_55dd30(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_GetNamedSemanticDescriptor_004d9950",
        addr: 0x004d_9950,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[(0x005a_0200, 5, 5, true), (0x0094_9207, 1, 0, false)],
        prep: |e, r, _| prep_guard(e, r, 0x00ca_5728),
        port: |g, _| named_semantic(g),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_GetNamedSemanticDescriptor_004d9b80",
        addr: 0x004d_9b80,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[(0x005a_0200, 5, 5, true), (0x0094_9207, 1, 0, false)],
        prep: |e, r, _| prep_guard(e, r, 0x00ca_5728),
        port: |g, _| named_semantic(g),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_WaitAndClear_0055e060",
        addr: 0x0055_e060,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[
            (0x0056_4680, 0, 0, true),
            (0x0056_48c0, 0, 0, true),
            (0x0042_fb20, 1, 0, false),
        ],
        prep: |e, r, a| prep_sharray_at(e, r, a, 0x14),
        port: |g, a| {
            h2::wait_and_clear(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PBufferD3D11_dtor_00584ea0",
        addr: 0x0058_4ea0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0050_32c0, 0, 0, true)],
        prep: |e, r, a| prep_com_slots(e, r, a, 0, &[0x10, 0xc, 8]),
        port: |g, a| h2::release_buffer(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PInstanceList_freeBuffer",
        addr: 0x0057_64a0,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0042_fb20, 1, 0, false)],
        prep: prep_free_buffer,
        port: |g, a| {
            let f = g.frame();
            h2::free_buffer(g, a[0], f);
            0
        },
        frame: 0x1c,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PDynamicMeshInstanceSub_scalarDtor_004fb770",
        addr: 0x004f_b770,
        conv: K::T,
        kinds: "pb",
        out: O::U32,
        callees: &[(0x0042_fb20, 1, 0, false), (0x004f_a470, 0, 0, true)],
        prep: no_prep,
        port: |g, a| h2::dynamic_dtor(g, a[0], a[1]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PDynamicMeshInstanceSub_constructInstance_004fc630",
        addr: 0x004f_c630,
        conv: K::T,
        kinds: "vn",
        out: O::U8,
        callees: &[(0x0043_cbb0, 0, 0, true), (0x004f_a420, 1, 1, true)],
        prep: |e, r, _| {
            if r.chance(4) {
                e.set_ret(0x0043_cbb0, 4);
            }
        },
        port: |g, a| u32::from(h2::construct_dynamic(g, a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_FreePairs_004e4df0",
        addr: 0x004e_4df0,
        conv: K::C,
        kinds: "nc",
        out: O::None,
        callees: &[(0x0042_fb20, 1, 0, false)],
        prep: no_prep,
        port: |g, a| {
            h2::free_pairs(g, a[0], a[1]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Vec3Max_004f84f0",
        addr: 0x004f_84f0,
        conv: K::C,
        kinds: "ppp",
        out: O::U32,
        callees: &[],
        prep: prep_vec3,
        port: |g, a| h2::vec3_select(g, a[0], a[1], a[2], true),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Vec3Min_004f8580",
        addr: 0x004f_8580,
        conv: K::C,
        kinds: "ppp",
        out: O::U32,
        callees: &[],
        prep: prep_vec3,
        port: |g, a| h2::vec3_select(g, a[0], a[1], a[2], false),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_00583240",
        addr: 0x0058_3240,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0049_f990, 0, 0, true)],
        prep: no_prep,
        port: |g, a| {
            h2::ctor_block(
                g,
                a[0],
                0x0049_f990,
                0xc,
                0x10,
                4,
                &[0x38, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34],
                None,
            )
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PBufferD3D11_destructInstance_00592780",
        addr: 0x0059_2780,
        conv: K::T,
        kinds: "vp",
        out: O::U8,
        callees: &[(0x0050_32c0, 0, 0, true)],
        prep: |e, r, a| prep_com_slots(e, r, a, 1, &[0x10, 0xc, 8]),
        port: |g, a| u32::from(h2::destruct_buffer(g, a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PClassDescriptor_PTexture2DD3D11_constructInstance_0058e820",
        addr: 0x0058_e820,
        conv: K::T,
        kinds: "vn",
        out: O::U8,
        callees: &[
            (0x004c_2f90, 0, 0, true),
            (0x0058_b070, 2, 0, false),
            (0x0094_964c, 3, 0, false),
        ],
        prep: no_prep,
        port: |g, a| u32::from(h2::construct_texture2d(g, a[1])),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_00584400",
        addr: 0x0058_4400,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x004c_6070, 0, 0, true), (0x0094_964c, 3, 0, false)],
        prep: no_prep,
        port: |g, a| {
            h2::ctor_block(
                g,
                a[0],
                0x004c_6070,
                0x20,
                0x24,
                5,
                &[0x64],
                Some((0x38, 0x2c)),
            )
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_005841a0",
        addr: 0x0058_41a0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x0050_1c40, 0, 0, true)],
        prep: no_prep,
        port: |g, a| {
            h2::ctor_block(
                g,
                a[0],
                0x0050_1c40,
                0x14,
                0x18,
                5,
                &[0x44, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40],
                None,
            )
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_Ctor_005842c0",
        addr: 0x0058_42c0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[(0x004c_47a0, 0, 0, true)],
        prep: no_prep,
        port: |g, a| {
            h2::ctor_block(
                g,
                a[0],
                0x004c_47a0,
                0x28,
                0x2c,
                5,
                &[0x64, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c, 0x60],
                None,
            )
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PShaderProgramD3D11_dtor_005856b0",
        addr: 0x0058_56b0,
        conv: K::T,
        kinds: "p",
        out: O::U32,
        callees: &[
            (0x0059_c9c0, 0, 0, true),
            (0x0058_8190, 2, 0, false),
            (0x0058_5610, 0, 0, true),
        ],
        prep: |e, r, a| {
            a[0] = e.pool + 4 * r.below(0x380);
            let n = r.below(4) | if r.chance(2) { 0x8000_0000 } else { 0 };
            e.w32(a[0] + 0x4e0, n);
        },
        port: |g, a| h2::program_dtor(g, a[0]),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_FreeTwoArrays_00585610",
        addr: 0x0058_5610,
        conv: K::T,
        kinds: "p",
        out: O::None,
        callees: &[(0x0042_fb20, 1, 0, false)],
        prep: |e, r, a| {
            prep_sharray_at(e, r, a, 0xc);
            prep_sharray_at(e, r, a, 4);
        },
        port: |g, a| {
            h2::free_two_arrays(g, a[0]);
            0
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PDynamicMeshInstance_Bind",
        addr: 0x004f_a540,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_95b0, 1, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0094_9207, 1, 0, false),
            (0x0043_c140, 0, 0, true),
        ],
        prep: |e, r, _| prep_guard(e, r, 0x00ca_7fe8),
        port: |g, _| h2::dynamic_mesh_bind(g),
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_setViewport_0058c790",
        addr: 0x0058_c790,
        conv: K::T,
        kinds: "pvvvv",
        out: O::U32,
        callees: &[],
        prep: prep_viewport,
        port: |g, a| {
            let f = g.frame();
            h2::set_viewport_rect(g, a[0], [a[1], a[2], a[3], a[4]], f)
        },
        frame: 0x1c,
        gap: 4,
        writes: &[],
    },
    Case {
        name: "PRendererD3D11_setTargetClearColor_00599f30",
        addr: 0x0059_9f30,
        conv: K::T,
        kinds: "pnc",
        out: O::U32,
        callees: &[(0x005b_3620, 0, 0, true), (0x005b_3640, 0, 0, true)],
        prep: |e, r, _| {
            let p = e.ptr(r);
            e.set_ret(0x005b_3620, p);
        },
        port: |g, a| {
            let f = g.frame();
            h2::set_target_color(g, a[0], a[1], a[2], f)
        },
        frame: 0x10,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_NewConstantBuffer_004f49d0",
        addr: 0x004f_49d0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0042_fb40, 1, 0, false),
            (0x0058_3240, 0, 0, true),
            (0x0059_9a80, 1, 1, true),
            (0x0059_5f70, 0, 0, true),
            (0x0058_cf20, 0, 0, true),
        ],
        prep: |e, r, _| {
            let p = if r.chance(4) { 0 } else { e.ptr(r) };
            e.set_ret(0x0042_fb40, p);
            if p != 0 && r.chance(3) {
                let n = 8 | if r.chance(2) { 0x8000_0000 } else { 0 };
                e.w32(p + 0xc, n);
            }
            let b = if r.chance(2) { 0x100 } else { 1 };
            e.set_ret(0x0059_5f70, b);
        },
        port: |g, _| h2::new_constant_buffer(g),
        frame: 0,
        gap: 0,
        writes: &[],
    },
];

/// Argument `i` low in the pool (apart from the objects) and COM
/// objects (or nulls) whose `Release` pops one word at its `offs`.
fn prep_com_slots(e: &mut Env, rng: &mut Rng, a: &mut [u32], i: usize, offs: &[u32]) {
    a[i] = e.pool + 4 * rng.below(0x400);
    let p = a[i];
    for (k, &off) in (0u32..).zip(offs) {
        let obj = if rng.chance(3) {
            0
        } else {
            e.pool + POOL_PTRS + 0x100 * k
        };
        e.w32(p + off, obj);
        if obj != 0 {
            let vt = obj + 0x80;
            let r = e.ret_word(rng);
            let f = e.handler(1, r);
            e.w32(obj, vt);
            e.w32(vt + 8, f);
        }
    }
}

/// `PInstanceList::freeBuffer`: the scene cursor `SetNode` fills (from a
/// block in the pool: slot, free slot, stride, count, offset), its free
/// chain among the slots, the type whose method 11 pops one word.
fn prep_free_buffer(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let src = e.pool + POOL_PTRS + 0xe00;
    e.custom(0x0044_1dc0, |n| {
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xa3]; // mov eax, [esp+4]; mov [n], eax
        c.extend(le(n));
        for i in 0..5u8 {
            c.push(0xa1);
            c.extend(le(src + 4 * u32::from(i))); // mov eax, [src+4i]
            c.extend([0x89, 0x41, 4 * i]); // mov [ecx+4i], eax
        }
        c.extend([0xc2, 0x04, 0x00]);
        c
    });
    let slots = src + 0x40;
    let stride = 4 * (1 + rng.below(3));
    let count = rng.below(6);
    let mut free: Vec<u32> = (1..=count)
        .filter(|_| rng.chance(3))
        .map(|i| slots + stride * i)
        .collect();
    free.sort_unstable();
    for (k, &f) in free.iter().enumerate() {
        let next = free.get(k + 1).copied().unwrap_or_else(|| e.ptr(rng));
        e.w32(f, next);
    }
    let head = free.first().copied().unwrap_or_else(|| e.ptr(rng));
    let off = rng.below(0x40);
    for (i, w) in (0u32..).zip([slots, head, stride, count, off]) {
        e.w32(src + 4 * i, w);
    }
    let ty = e.pool + POOL_PTRS + 0x300;
    let vt = ty + 0x40;
    e.w32(a[0] + 0x2c, ty);
    e.w32(ty, vt);
    let r = e.ret_word(rng);
    let h = e.handler(1, r);
    e.w32(vt + 0x2c, h);
}

/// Two 3-vectors of floats (NaNs, equal pairs, signed zeros).
fn prep_vec3(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    let pick = |rng: &mut Rng| -> u32 {
        match rng.below(6) {
            0 if rng.chance(2) => [0, 0x8000_0000][rng.below(2) as usize],
            0 => [
                0x7fc0_0000,
                0xffc0_1234,
                0,
                0x8000_0000,
                0x7f80_0000,
                0xff80_0000,
            ][rng.below(6) as usize],
            1 => 0x3f80_0000,
            _ => ((rng.below(400) as f32 - 200.0) / 3.0).to_bits(),
        }
    };
    for i in 0..3 {
        let x = pick(rng);
        let mut y = if rng.chance(4) { x } else { pick(rng) };
        // equal but different: which one wins shows
        if rng.chance(5) {
            y = x ^ 0x8000_0000;
            if x & 0x7fff_ffff != 0 {
                y = x;
            }
        }
        e.w32(a[1] + 4 * i, x);
        e.w32(a[2] + 4 * i, y);
    }
}

/// `setViewport`: the renderer low in the pool (its height word at
/// `+0x1338`), the context at `+0x70` whose method 44 pops three words.
fn prep_viewport(e: &mut Env, rng: &mut Rng, a: &mut [u32]) {
    a[0] = e.pool + 4 * rng.below(0x100);
    let hgt = rng.below(0x800);
    e.w32(a[0] + 0x1338, hgt);
    let ctx = e.pool + POOL_PTRS + 0x200;
    let vt = ctx + 0x40;
    e.w32(a[0] + 0x70, ctx);
    e.w32(ctx, vt);
    let r = e.ret_word(rng);
    let h = e.handler(3, r);
    e.w32(vt + 0xb0, h);
    for w in &mut a[1..5] {
        if rng.chance(2) {
            *w = rng.below(0x1000);
        }
    }
}

/// The shims' call of `0x004d9950`/`0x004d9b80`.
fn named_semantic(g: &mut G) -> u32 {
    hm::static_once_args(
        g,
        0x00ca_5728,
        0x00ca_56f4,
        0x005a_0200,
        &[
            (0x00b2_0350, true),
            (0x10, false),
            (4, false),
            (0x004d_ec30, true),
            (0x004d_f9c0, true),
        ],
        0x00b2_062c,
        0x00af_ed30,
    )
}
