// Batch 2 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a2::{self as m2, bridge as b2};

#[test]
fn concrete_registers_match_the_original() {
    // The guard set (registered only) and clear (built first).
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b001);
        for case in 0..4u32 {
            for &(addr, r) in m2::CONCRETE_REGISTERS {
                let guard = r.d + 0x94;
                let g = (rng.next() & !1) | (case & 1);
                t.global(guard, g);
                let f = rng.next();
                t.global(r.d + 0x90, f);
                t.stub_sym(r.ctor, 1, true, 0, 0, Ret::Val(5));
                t.stub_sym(m2::ATEXIT, 2, false, 1, 0, Ret::Val(0));
                t.stub_sym(m2::CLASS_REGISTER, 3, true, 2, 2, Ret::Val(0x1234_5601 + case));
                let p = rng.word();
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[p], |g| {
                    Out::U32(b2::concrete_register(g, p, &r))
                });
                t.reset();
            }
        }
    });
}

/// An array header and an `n` for a resize: the header word itself,
/// 0, or a small count.
fn resize_case(t: &mut Twins, rng: &mut Rng) -> (u32, u32) {
    let this = t.zeros(0x10);
    header(t, rng, this);
    let hw = t.r32(this);
    let n = match rng.below(4) {
        0 => hw,
        1 => 0,
        _ => rng.below(9),
    };
    (this, n)
}

#[test]
fn storage_helpers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b002);
        for case in 0..30u32 {
            for &(addr, test, release, dtor) in m2::RELEASE_DTORS {
                let this = t.zeros(0x40);
                t.stub_sym(test, 1, true, 0, 0, Ret::Val(rng.next() & 0x101));
                t.stub_sym(release, 2, true, 0, 0, Ret::Val(7));
                t.stub_sym(dtor, 3, true, 0, 0, Ret::Val(case));
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[], |g| {
                    Out::U32(b2::release_dtor(g, this, test, release, dtor))
                });
                t.reset();
            }
            for &(addr, size, align, copy, zero) in m2::ALLOC_COPIES {
                let n = rng.below(6);
                let count = rng.below(n + 1);
                let src: Vec<u32> = (0..16).map(|_| rng.next()).collect();
                let src = t.words(&src);
                let block = if rng.chance(5) { 0 } else { t.zeros(0x80) };
                if block != 0 {
                    let fill: Vec<u8> = (0..0x80).map(|_| rng.next() as u8).collect();
                    t.write(block, &fill);
                }
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                if copy != 0 {
                    t.stub_sym(copy, 2, false, 3, 0, Ret::Val(9));
                }
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[n, src, count], |g| {
                    Out::U32(b2::alloc_copy(g, n, src, count, size, align, copy, zero))
                });
                t.reset();
            }
            {
                let n = rng.below(6);
                let src: Vec<u32> = (0..8).map(|_| rng.next()).collect();
                let src = t.words(&src);
                let block = if rng.chance(5) { 0 } else { t.zeros(0x40) };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.check(
                    "PArray_allocCopyAll_004878d0",
                    0x0048_78d0,
                    Conv::Cdecl,
                    &[n, src],
                    |g| Out::U32(b2::alloc_copy_all(g, n, src)),
                );
                t.reset();
            }
            for &(addr, helper, skip_null) in m2::RESIZE_VIA {
                let (this, n) = resize_case(t, &mut rng);
                let block = match rng.below(5) {
                    0 => 0,
                    1 => t.r32(this + 4),
                    _ => rng.next() | 1,
                };
                t.stub_sym(helper, 1, false, 3, 0, Ret::Val(block));
                t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(3));
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[n], |g| {
                    Out::U32(b2::resize_via(g, this, n, helper, skip_null))
                });
                t.reset();
            }
            for &(addr, size) in m2::RESIZE_DISCARD {
                let (this, n) = resize_case(t, &mut rng);
                let block = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(3));
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[n], |g| {
                    Out::U32(b2::resize_discard(g, this, n, size))
                });
                t.reset();
            }
            {
                let (this, n) = resize_case(t, &mut rng);
                let block = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(3));
                t.stub_sym(0x004b_2d70, 3, false, 3, 0, Ret::Val(4));
                t.stub_sym(0x004b_8cf0, 4, false, 2, 0, Ret::Val(5));
                t.check(
                    "PArray_resizeCopyInit16_004ba720",
                    0x004b_a720,
                    Conv::Thiscall(this),
                    &[n],
                    |g| Out::U32(b2::resize_copy_init16(g, this, n)),
                );
                t.reset();
            }
            {
                let (this, n) = resize_case(t, &mut rng);
                let block = if rng.chance(4) { 0 } else { rng.next() | 1 };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.stub_sym(0x004d_8a30, 3, false, 3, 0, Ret::Val(4));
                t.stub_sym(0x004d_f1d0, 4, false, 2, 0, Ret::Val(5));
                t.stub_sym(0x004e_13a0, 5, true, 2, 2, Ret::Val(6));
                t.check(
                    "PArray_PShaderInfo_resize_004e1150",
                    0x004e_1150,
                    Conv::Thiscall(this),
                    &[n],
                    |g| Out::U32(b2::resize_shader_infos(g, this, n)),
                );
                t.reset();
            }
        }
    });
}

#[test]
fn element_records_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b003);
        for _ in 0..20 {
            let count = rng.below(5);
            let src: Vec<u32> = (0..80).map(|_| rng.next()).collect();
            let src = t.words(&src);
            let fill: Vec<u32> = (0..80).map(|_| rng.next()).collect();
            let at0 = rng.chance(3);
            let count = if at0 { 1 } else { count };
            let dst = if at0 { 0 } else { t.words(&fill) };
            t.check(
                "PhyreCopyElementsU16_004b2d00",
                0x004b_2d00,
                Conv::Cdecl,
                &[dst, src, count],
                |g| {
                    b2::copy_u16(g, dst, src, count);
                    Out::None
                },
            );
            t.check(
                "PhyreCopyLocation16_004b2d70",
                0x004b_2d70,
                Conv::Cdecl,
                &[dst, src, count],
                |g| {
                    b2::copy_location16(g, dst, src, count);
                    Out::None
                },
            );
            t.check(
                "PhyreInitLocation16_004b8cf0",
                0x004b_8cf0,
                Conv::Cdecl,
                &[dst, count],
                |g| {
                    b2::init_location16(g, dst, count);
                    Out::None
                },
            );
            t.check(
                "PhyreInitSubTexture20_0056bab0",
                0x0056_bab0,
                Conv::Cdecl,
                &[dst, count],
                |g| {
                    b2::init_subtexture20(g, dst, count);
                    Out::None
                },
            );
            for (addr, w) in [(0x0048_c930, 16), (0x0048_c9d0, 8)] {
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[dst, src, count], |g| {
                    a::copy_elements(g, dst, src, count, w);
                    Out::None
                });
            }
            let this = t.words(&fill);
            t.check(
                "PShaderRef_init_004bbbd0",
                0x004b_bbd0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b2::init_shader_ref(g, this)),
            );
            // array headers, then the block
            let n = rng.below(4);
            let headers = if rng.chance(5) { 0 } else { t.zeros(0x20) };
            if headers != 0 {
                for i in 0..n {
                    header(t, &mut rng, headers + 8 * i);
                }
            }
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(2));
            t.check(
                "PhyreFreeHeaders_004d3720",
                0x004d_3720,
                Conv::Cdecl,
                &[headers, n],
                |g| {
                    b2::free_headers(g, headers, n);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn locked_setters_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b004);
        for case in 0..40u32 {
            let this = t.zeros(0x40);
            for i in 0..16 {
                let v = rng.next();
                t.w32(this + 4 * i, v);
            }
            let flags = rng.next();
            t.w32(
                this + 0x1c,
                if case % 3 == 0 { flags | 0x80 } else { flags & !0x80 },
            );
            for &(addr, off, float) in m2::LOCKED_SETTERS {
                let v = if float { float_bits(&mut rng) } else { rng.word() };
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[v], |g| {
                    Out::U32(b2::locked_set(g, this, off, float, v))
                });
            }
            let on = rng.word() & 0x101;
            t.check(
                "PLocked_setFlag40_004c06e0",
                0x004c_06e0,
                Conv::Thiscall(this),
                &[on],
                |g| Out::U32(b2::locked_set_flag40(g, this, on)),
            );
            let v = rng.below(4) | if rng.chance(4) { 0x100 } else { 0 };
            t.check(
                "PLocked_setByte1_004c0750",
                0x004c_0750,
                Conv::Thiscall(this),
                &[v],
                |g| Out::U32(b2::locked_set_byte1(g, this, v)),
            );
            for &(addr, kind) in m2::OWNER_SETTERS {
                let cur = if rng.chance(4) {
                    0
                } else {
                    let o = t.zeros(8);
                    let of = if rng.chance(2) { this } else { rng.next() };
                    t.w32(o, of);
                    t.w8(o + 4, rng.below(3) as u8);
                    o
                };
                t.w32(this + 0x18, cur);
                let owner = rng.word();
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[owner], |g| {
                    Out::U32(b2::set_owner(g, this, owner, kind))
                });
            }
            t.reset();
        }
    });
}

#[test]
fn sprite_frames_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b005);
        for case in 0..40u32 {
            let fill: Vec<u32> = (0..0x40).map(|_| rng.next()).collect();
            let this = t.words(&fill);
            let frames = 1 + rng.below(6);
            let records = if case % 7 == 0 { 0 } else { t.zeros(48 * 8) };
            let order: Vec<u32> = (0..8).map(|_| rng.below(9)).collect();
            let order = t.words(&order);
            t.w32(this + 0xb8, records);
            t.w32(this + 0xc8, order);
            t.w32(this + 0x20, frames);
            let frame = rng.below(8);
            for &(addr, off, n) in m2::SPRITE_SETTERS {
                let v: Vec<u32> = (0..n).map(|_| float_bits(&mut rng)).collect();
                let mut args = vec![frame];
                args.extend(&v);
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &args, |g| {
                    Out::U32(b2::sprite_set(g, this, frame, off, &v))
                });
            }
            let src: Vec<u32> = (0..12).map(|_| rng.next()).collect();
            let src = t.words(&src);
            t.check(
                "PSprite_setFrameRecord_0056e7b0",
                0x0056_e7b0,
                Conv::Thiscall(this),
                &[frame, src],
                |g| Out::U32(b2::sprite_set_record(g, this, frame, src)),
            );
            // the frame tables
            let n = rng.below(6);
            let next = t.zeros(0x20);
            let prev = t.zeros(0x20);
            t.w32(this + 0x1c, n);
            t.w32(this + 0xc8, next);
            t.w32(this + 0xd0, prev);
            t.check(
                "PSprite_resetFrames_0056df40",
                0x0056_df40,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b2::reset_frames(g, this)),
            );
            // the depth comparator
            let (x, y) = if case % 5 == 0 {
                let v = float_bits(&mut rng);
                (v, v)
            } else {
                (float_bits(&mut rng), float_bits(&mut rng))
            };
            let pa = t.zeros(0x30);
            let pb = t.zeros(0x30);
            t.w32(pa + 0x28, x);
            t.w32(pb + 0x28, y);
            t.check(
                "PSprite_compareDepth_0056c110",
                0x0056_c110,
                Conv::Cdecl,
                &[pa, pb],
                |g| Out::U32(b2::compare_depth(g, pa, pb)),
            );
            // set to an external buffer, the old one released
            let arr = t.zeros(0x10);
            header(t, &mut rng, arr);
            let old = t.r32(arr + 4);
            let buffer = if rng.chance(4) { old } else { rng.word() };
            let count = rng.word();
            t.stub_sym(0x0056_a700, 1, false, 2, 0, Ret::Val(4));
            t.check(
                "Phyre_PArray_setToExternalBuffer_0056ebf0",
                0x0056_ebf0,
                Conv::Thiscall(arr),
                &[buffer, count],
                |g| Out::U32(b2::set_external_release(g, arr, buffer, count)),
            );
            t.reset();
        }
    });
}

#[test]
fn render_methods_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b006);
        for case in 0..30u32 {
            let this = t.zeros(0x100);
            for i in 0..0x40 {
                let v = rng.next();
                t.w32(this + 4 * i, v);
            }
            // the tree lookup through the real FUN_004da4e0, the key by address
            {
                let tree = t.zeros(0x10);
                let node = t.zeros(0x20);
                t.w32(tree + 0xc, if case % 4 == 0 { tree } else { node });
                t.w32(node, tree);
                t.w32(node + 4, tree);
                let key = t.zeros(0x10);
                t.w32(node + 0xc, if case % 3 == 0 { key + 1 } else { key });
                let base = rng.next();
                t.w32(key + 8, base);
                let block = rng.next();
                t.w32(node + 0x14, block);
                let p = base.wrapping_add(rng.next() % 0x400).wrapping_sub(0x200);
                let slot = t.b.stack_range().start + 0x80;
                t.check(
                    "PRedBlackTree_blockEntry_004e0700",
                    0x004e_0700,
                    Conv::Thiscall(tree),
                    &[key, p],
                    |g| Out::U32(b2::node_block_entry(g, tree, key, p, slot)),
                );
            }
            t.stub_sym(0x004f_a420, 1, true, 1, 1, Ret::Val(1));
            let arg = rng.word();
            t.check(
                "Phyre_PRendering_PModifierNetworkDynamicMeshInstance_ctor_004fb3c0",
                0x004f_b3c0,
                Conv::Thiscall(this),
                &[arg],
                |g| Out::U32(b2::modifier_instance_ctor(g, this, arg)),
            );
            let n = rng.below(4);
            t.w8(this + 0x34, rng.next() as u8);
            let cnt = if rng.chance(3) { n.max(1) } else { rng.word() };
            t.w32(this + 0x18, cnt);
            t.stub_sym(0x0056_46d0, 2, true, 0, 0, Ret::Val(2));
            t.stub_sym(0x0056_4630, 3, true, 0, 0, Ret::Val(3));
            t.stub_sym(0x0056_3df0, 4, true, 1, 1, Ret::Val(rng.below(2) * 0xd));
            t.check(
                "PBufferD3D11_setCount_00563fb0",
                0x0056_3fb0,
                Conv::Thiscall(this),
                &[n],
                |g| Out::U32(b2::set_buffer_count(g, this, n)),
            );
            t.stub_sym(m2::EHVEC_DTOR, 5, false, 4, 4, Ret::Val(case));
            t.check(
                "PEhVector_destroy17C_00584e80",
                0x0058_4e80,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b2::destroy_elements(g, this)),
            );
            let mode = rng.below(10);
            t.check(
                "Phyre_PRendering_MapMode8_0058aaa0",
                0x0058_aaa0,
                Conv::Cdecl,
                &[mode],
                |_| Out::U32(b2::map_mode(mode)),
            );
            // the texture bind: the sharray's first word 0 or not (count 0
            // reads address 0: left out)
            {
                let count = [1, 2, 3, 0x8000_0001, 0x8000_0002][case as usize % 5];
                let first = if rng.chance(2) { 0 } else { rng.next() | 1 };
                let data = t.words(&[first, 5]);
                t.w32(this + 0x28, count);
                t.w32(
                    this + 0x2c,
                    if count & 0x7fff_ffff > 1 { data } else { first },
                );
                t.stub_sym(0x0059_03a0, 6, true, 4, 4, Ret::Val(6));
                t.check(
                    "PTexture3DD3D11_bind_0058db50",
                    0x0058_db50,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(b2::texture3d_bind(g, this)),
                );
            }
            // the instanced draw on a fake context
            {
                let topo = rng.below(3);
                let m1 = t.stub_code(7, false, 2, 2, Ret::Val(0), false);
                let m2s = t.stub_code(8, false, 5, 5, Ret::Val(0x4444_0000), false);
                let mut slots = vec![0u32; 25];
                slots[24] = m1;
                slots[21] = m2s;
                let ctx = t.object(&slots, 0x10);
                let r = t.zeros(0x80);
                t.w32(r + 0x70, ctx);
                let (start, count, inst) = (rng.word(), rng.below(3), rng.below(3));
                t.stub_sym(0x0058_ab60, 9, false, 1, 0, Ret::Val(topo));
                t.check(
                    "PRendererD3D11_drawInstanced_00592c10",
                    0x0059_2c10,
                    Conv::Thiscall(r),
                    &[mode, start, count, inst],
                    |g| Out::U8(b2::draw_instanced(g, r, mode, start, count, inst)),
                );
            }
            // the segment draw
            {
                let segs_n = rng.below(4);
                let segs = t.zeros(0x100);
                for i in 0..segs_n {
                    let v = if rng.chance(2) { 0 } else { rng.next() };
                    t.w32(segs + 4 + 64 * i, v);
                }
                let segments = t.words(&[segs_n | (rng.next() & 0x8000_0000), segs]);
                let desc = if case % 2 == 0 {
                    0
                } else {
                    t.words(&[1, 2, rng.next()])
                };
                let mesh = t.zeros(0x20);
                let tp = rng.next();
                t.w32(mesh + 0x10, tp);
                let (x, y, z) = (rng.word(), rng.word(), rng.word());
                t.stub_sym(0x0059_2de0, 10, true, 5, 5, Ret::Val(10));
                t.stub_sym(0x0059_2c10, 11, true, 4, 4, Ret::Val(11));
                let rr = rng.word();
                t.check(
                    "PMeshDraw_drawSegmentInstanced_004985b0",
                    0x0049_85b0,
                    Conv::Thiscall(this),
                    &[rr, x, mesh, segments, desc, y, z],
                    |g| Out::U32(b2::draw_segment(g, rr, mesh, segments, desc, y, z)),
                );
            }
            // the buffer's slot unmap
            {
                let cnt = rng.below(4);
                t.w32(this + 0x18, cnt | (rng.next() & 0x8000_0000));
                let cur = rng.below(4);
                t.w32(this + 0x10, cur);
                t.stub_sym(0x0056_4cb0, 12, true, 2, 2, Ret::Val(rng.below(2)));
                let a0 = rng.word();
                t.check(
                    "PBuffer_unmapNextSlot_004350b0",
                    0x0043_50b0,
                    Conv::Thiscall(this),
                    &[a0],
                    |g| Out::U32(b2::unmap_next_slot(g, this, a0)),
                );
            }
            // the texture iterator
            {
                let hdr = t.zeros(0x40);
                let first = if rng.chance(2) { hdr + 0x1c } else { rng.next() };
                t.w32(hdr + 0x1c, first);
                t.stub_sym(0x0046_5010, 13, true, 0, 0, Ret::Val(13));
                t.check(
                    "PTextureIterator_ctor_00463cb0",
                    0x0046_3cb0,
                    Conv::Thiscall(this),
                    &[hdr],
                    |g| Out::U32(b2::texture_iter_ctor(g, this, hdr)),
                );
            }
            // the array word getter
            {
                let words: Vec<u32> = (0..4).map(|_| rng.next()).collect();
                let data = t.words(&words);
                let arr = t.zeros(0x10);
                let count = [0, 1, 2, 4, 0x8000_0003][case as usize % 5];
                t.w32(arr, count);
                t.w32(
                    arr + 4,
                    if count & 0x7fff_ffff > 1 { data } else { words[0] },
                );
                let i = rng.below(5);
                t.check(
                    "PArray_word_0048c1a0",
                    0x0048_c1a0,
                    Conv::Thiscall(arr),
                    &[i],
                    |g| Out::U32(b2::array_word(g, arr, i)),
                );
            }
            t.reset();
        }
    });
}

#[test]
fn statics_match_the_original() {
    // Guards set and clear; lists with flagged nodes anywhere.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_b007);
        for case in 0..24u32 {
            for (addr, head, link, flag, ga, gb) in [
                (0x0043_2680, 0x00ca_4e18, 8, 0xc, 0x00ca_4edc, 0x00ca_4ef4),
                (0x0043_26f0, 0x00ca_54f8, 4, 8, 0x00ca_54fc, 0x00ca_5514),
            ] {
                let n = rng.below(5);
                let nodes = list(t, &mut rng, head, link, n);
                for &nd in &nodes {
                    let f = u8::from(rng.chance(2));
                    t.w8(nd + flag, f);
                }
                let seq = t.seq(&[case + 1, case + 100]);
                t.stub_sym(m2::FREE_LIST_RESET, 1, true, 0, 0, seq);
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[], |g| {
                    Out::U32(b2::prune_list(g, head, link, flag, ga, gb))
                });
                t.reset();
            }
            {
                let n = rng.below(3);
                let nodes = list(t, &mut rng, ma::PUTILITY_LIST, 4, n);
                let this = if n > 0 && rng.chance(2) {
                    nodes[rng.below(n) as usize]
                } else {
                    let x = t.zeros(0x20);
                    let nx = rng.word();
                    t.w32(x + 4, nx);
                    x
                };
                t.stub_sym(0x0043_3af0, 1, true, 0, 0, Ret::Val(1));
                t.check(
                    "Phyre_PSprite_PUtilitySprite_dtor_00433dd0",
                    0x0043_3dd0,
                    Conv::Thiscall(this),
                    &[],
                    |g| {
                        b2::utility_sprite_dtor(g, this);
                        Out::None
                    },
                );
                t.reset();
            }
            let bits = rng.next() & !1;
            t.global(0x00c9_02e4, bits | (case & 1));
            t.global(0x00c9_02dc, bits | ((case >> 1) & 1));
            let cell = rng.next();
            t.global(0x00c9_02e0, cell);
            t.stub_sym(m2::ATEXIT, 2, false, 1, 0, Ret::Val(0));
            t.check(
                "Phyre_PTTYCallback_current_00435270",
                0x0043_5270,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b2::tty_current(g)),
            );
            // the deleting destructor: the current callback is this or not
            let this = t.zeros(0x10);
            let cellv = if case % 3 == 0 { this } else { rng.next() };
            let cellp = t.zeros(4);
            t.w32(cellp, cellv);
            t.stub_sym(0x0043_5270, 3, false, 0, 0, Ret::Val(cellp));
            t.stub_sym(ma::OPERATOR_DELETE, 4, false, 1, 0, Ret::Val(4));
            let flags = rng.word();
            t.check(
                "Phyre_PTTYCallback_scalarDeletingDtor_004351f0",
                0x0043_51f0,
                Conv::Thiscall(this),
                &[flags],
                |g| Out::U32(b2::tty_delete(g, this, flags)),
            );
            t.global(0x00ca_4f40, bits | (case & 1));
            t.stub_sym(0x005a_0200, 5, true, 5, 5, Ret::Val(5));
            t.check(
                "PNamedSemanticDescriptorForDynamicType_PContextSwitch_get_004d0e20",
                0x004d_0e20,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b2::context_switch_semantic(g)),
            );
            t.global(0x00c9_4efc, bits | ((case >> 1) & 1));
            t.stub_sym(0x0043_15e0, 6, true, 0, 0, Ret::Val(6));
            t.stub_sym(0x0043_1950, 7, true, 0, 0, Ret::Val(case + 70));
            t.check(
                "PhyreStaticEventOwner_004ff0a0",
                0x004f_f0a0,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b2::static_event_owner(g)),
            );
            t.reset();
        }
    });
}
