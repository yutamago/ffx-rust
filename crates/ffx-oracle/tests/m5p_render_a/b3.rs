// Batch 3 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a3::{self as m3, bridge as b3};

/// `n` random words.
fn random_words(t: &mut Twins, rng: &mut Rng, n: usize) -> u32 {
    let w: Vec<u32> = (0..n).map(|_| rng.next()).collect();
    t.words(&w)
}

#[test]
fn mesh_copies_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_c001);
        for case in 0..30u32 {
            let src = random_words(t, &mut rng, 0x40);
            let this = random_words(t, &mut rng, 0x40);
            let count = [0, 1, 3, 0x8000_0002][case as usize % 4];
            t.w32(src + 8, count);
            t.w32(src + 0x28, count);
            let block = if case % 5 == 4 { 0 } else { t.zeros(0x200) };
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
            t.stub_sym(0x0048_0bb0, 2, false, 3, 0, Ret::Val(2));
            t.check(
                "Phyre_PGeometry_PMeshSegmentBase_copy_004870b0",
                0x0048_70b0,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(b3::segment_base_copy(g, this, src)),
            );
            t.stub_sym(0x0048_78d0, 3, false, 2, 0, Ret::Val(block));
            t.check(
                "Phyre_PGeometry_PMeshSegment_copy_0048f060",
                0x0048_f060,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(b3::mesh_segment_copy(g, this, src)),
            );
            t.stub_sym(0x0048_f060, 4, true, 1, 1, Ret::Val(4));
            t.stub_sym(0x0048_6120, 5, false, 3, 0, Ret::Val(5));
            t.stub_sym(0x0048_eed0, 6, true, 1, 1, Ret::Val(6));
            t.check(
                "Phyre_PGeometry_PMesh_copy_0048f0d0",
                0x0048_f0d0,
                Conv::Thiscall(this),
                &[src],
                |g| Out::U32(b3::mesh_copy(g, this, src)),
            );
            t.reset();
            // a sharray of 20-byte elements: inline or a block
            let scount = [0, 1, 2, 5, 0x8000_0003][case as usize % 5];
            t.w32(src + 0x44, scount);
            let sdata = random_words(t, &mut rng, 0x20);
            if scount & 0x7fff_ffff > 1 {
                t.w32(src + 0x48, sdata);
            }
            let dst = this + 0x44;
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
            t.stub_sym(0x0048_4a30, 7, false, 3, 0, Ret::Val(7));
            t.check(
                "PSharray20_copy_0048eed0",
                0x0048_eed0,
                Conv::Thiscall(dst),
                &[src + 0x44],
                |g| Out::U32(b3::sharray20_copy(g, dst, src + 0x44)),
            );
            t.reset();
            // the element loops
            let n = rng.below(4);
            let at0 = rng.chance(4);
            let n = if at0 { 1 } else { n };
            let dst = if at0 { 0 } else { t.zeros(0x800) };
            let srcs = random_words(t, &mut rng, 0x200);
            t.stub_sym(0x0048_70b0, 1, true, 1, 1, Ret::Val(1));
            t.stub_sym(0x0048_6ff0, 2, true, 1, 1, Ret::Val(2));
            t.check(
                "Phyre_PGeometry_PMeshSegment_copyArray_00486120",
                0x0048_6120,
                Conv::Cdecl,
                &[dst, srcs, n],
                |g| {
                    b3::segments_copy(g, dst, srcs, n);
                    Out::None
                },
            );
            for &(addr, size, ctor) in m3::COPY_CONSTRUCTS {
                t.stub_sym(ctor, 3, true, 1, 1, Ret::Val(3));
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[dst, srcs, n], |g| {
                    b3::copy_construct(g, dst, srcs, n, size, ctor);
                    Out::None
                });
            }
            t.stub_sym(0x0058_3c00, 4, true, 0, 0, Ret::Val(4));
            t.check(
                "PShaderPassD3D11_constructArray_004bd1e0",
                0x004b_d1e0,
                Conv::Cdecl,
                &[dst, n],
                |g| {
                    b3::shader_pass_construct(g, dst, n);
                    Out::None
                },
            );
            // the string records: texts tagged or not, none, no memory
            let recs = t.zeros(0x80);
            for i in 0..4 {
                let r = recs + 16 * i;
                let fill: Vec<u8> = (0..16).map(|_| rng.next() as u8).collect();
                t.write(r, &fill);
                let word = match rng.below(3) {
                    0 => 0,
                    _ => {
                        let text: Vec<u8> = (0..rng.below(9))
                            .map(|_| (rng.below(90) + 32) as u8)
                            .chain([0])
                            .collect();
                        t.alloc(&text) | rng.below(2)
                    }
                };
                t.w32(r + 4, word);
            }
            let blocks: Vec<u32> = (0..4)
                .map(|_| if rng.chance(4) { 0 } else { t.zeros(0x10) })
                .collect();
            let seq = t.seq(&blocks);
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 5, false, 2, 0, seq);
            t.stub_sym(m3::MEMCPY, 6, false, 3, 0, Ret::Val(6));
            let rdst = if at0 { 0 } else { random_words(t, &mut rng, 0x20) };
            for &addr in m3::STRING_RECORD_COPIES {
                t.check(&format!("{addr:#x}"), addr, Conv::Cdecl, &[rdst, recs, n], |g| {
                    b3::string_records_copy(g, rdst, recs, n);
                    Out::None
                });
                let Ret::Seq(cell) = seq else {
                    unreachable!()
                };
                let table = t.words(&blocks);
                t.w32(cell, table);
            }
            t.reset();
        }
    });
}

#[test]
fn sharray_storage_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_c002);
        for case in 0..40u32 {
            // the inline-or-block word storage
            let this = random_words(t, &mut rng, 0x10);
            let n = rng.below(6);
            let count = rng.below(n + 1);
            let src = random_words(t, &mut rng, 8);
            let block = if rng.chance(5) { 0 } else { random_words(t, &mut rng, 8) };
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
            t.check(
                "PSharray_wordsAlloc_0048bbc0",
                0x0048_bbc0,
                Conv::Thiscall(this),
                &[n, src, count],
                |g| Out::U32(b3::sharray_words_alloc(g, this, n, src, count)),
            );
            t.reset();
            // a sharray header: inline (count 1), a block, external, empty
            let mk = |t: &mut Twins, rng: &mut Rng| -> u32 {
                let a = random_words(t, rng, 8);
                let c = [0, 1, 2, 4, 0x8000_0003, 0x8000_0001][rng.below(6) as usize];
                t.w32(a, c);
                if c & 0x7fff_ffff > 1 {
                    let b = if rng.chance(5) { 0 } else { t.zeros(0x40) };
                    t.w32(a + 4, b);
                }
                a
            };
            for &(addr, copy, zero) in m3::SHARRAY_RESIZES {
                let this = mk(t, &mut rng);
                let hw = t.r32(this);
                let n = if rng.chance(5) { hw } else { rng.below(5) };
                let block = if rng.chance(5) { 0 } else { t.zeros(0x80) };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.stub_sym(copy, 2, false, 3, 0, Ret::Val(2));
                t.stub_sym(zero, 3, false, 2, 0, Ret::Val(3));
                t.stub_sym(ma::PHYRE_FREE, 4, false, 1, 0, Ret::Val(4));
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[n], |g| {
                    Out::U32(b3::sharray20_resize(g, this, n, copy, zero))
                });
                t.reset();
            }
            for &addr in m3::SHARRAY_EXTERNALS {
                let this = mk(t, &mut rng);
                let buffer = if rng.chance(5) {
                    this + 4
                } else {
                    random_words(t, &mut rng, 8)
                };
                let count = [0, 1, 2, 7, 0x8000_0001][rng.below(5) as usize];
                t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(1));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[buffer, count],
                    |g| Out::U32(b3::sharray20_external(g, this, buffer, count)),
                );
                t.reset();
            }
            for &(addr, class, setter) in m3::MEMBER_EXTERNALS {
                let this = t.zeros(0x20);
                let buffer = if case % 2 == 0 { this + 4 } else { rng.word() };
                let count = rng.below(3);
                // the class object's virtual 1 on a fake vtable
                let reloc = if rng.chance(3) { 0 } else { rng.next() | 1 };
                let v1 = t.stub_code(1, true, 0, 0, Ret::Val(reloc), false);
                let vt = t.words(&[0, v1]);
                t.global(class, vt);
                t.stub_sym(0x0043_c160, 2, true, 2, 2, Ret::Val(2));
                t.stub_sym(setter, 3, true, 2, 2, Ret::Val(3));
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[buffer, count],
                    |g| Out::U32(b3::member_external(g, this, buffer, count, class, setter)),
                );
                t.reset();
            }
            // truncations
            let arr = t.zeros(0x10);
            let cnt = rng.below(5) | if rng.chance(4) { 0x8000_0000 } else { 0 };
            let words: Vec<u32> = (0..5).map(|_| rng.next()).collect();
            let data = t.words(&words);
            t.w32(arr, cnt);
            t.w32(arr + 4, data);
            let n = rng.below(6);
            t.stub_sym(ma::PHYRE_FREE, 1, false, 1, 0, Ret::Val(1));
            t.check(
                "PArray_truncateTagged_004d5940",
                0x004d_5940,
                Conv::Thiscall(arr),
                &[n],
                |g| {
                    b3::truncate_tagged(g, arr, n);
                    Out::None
                },
            );
            let hdrs = t.zeros(0x30);
            for i in 0..5 {
                header(t, &mut rng, hdrs + 8 * i);
            }
            t.w32(arr, cnt);
            t.w32(arr + 4, hdrs);
            t.check(
                "PArray_truncateHeaders_004d58b0",
                0x004d_58b0,
                Conv::Thiscall(arr),
                &[n],
                |g| {
                    b3::truncate_headers(g, arr, n);
                    Out::None
                },
            );
            t.reset();
        }
    });
}

#[test]
fn texture_and_dtor_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_c003);
        for case in 0..60u32 {
            let this = random_words(t, &mut rng, 0x20);
            // zero sizes with no mips (1 << 0 exceeds them only when checked)
            let (w, h, d) = if case % 8 == 0 {
                (0, 0, rng.below(5))
            } else {
                (1 << rng.below(12), 1 << rng.below(12), rng.below(5))
            };
            let format = rng.below(4);
            let mips = if case % 4 == 0 { 0 } else { rng.below(14) | (rng.next() & 0x20) };
            if case % 3 == 0 {
                t.w32(this + 0x1c, w);
                t.w32(this + 0x20, h);
                t.w32(this + 0x24, d);
                t.w32(this, format);
                // all equal but the mip count, half the time
                t.w32(this + 0xc, if case % 2 == 0 { mips } else { mips ^ 1 });
            }
            let levels = if rng.chance(2) { t.r32(this + 0x10) } else { rng.below(13) };
            t.stub_sym(0x004c_2430, 1, false, 3, 0, Ret::Val(levels));
            t.check(
                "Phyre_PTexture3DBase_setDimensions_004c5980",
                0x004c_5980,
                Conv::Thiscall(this),
                &[w, h, d, format, mips],
                |g| Out::U32(b3::set_dimensions(g, this, w, h, d, format, mips)),
            );
            header(t, &mut rng, this + 8);
            t.stub_sym(0x0055_e060, 2, true, 0, 0, Ret::Val(2));
            t.stub_sym(0x0043_3900, 3, true, 0, 0, Ret::Val(3));
            t.stub_sym(ma::PHYRE_FREE, 4, false, 1, 0, Ret::Val(4));
            t.check(
                "PSpriteCollection_dtorBody_00433c80",
                0x0043_3c80,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b3::sprite_collection_dtor(g, this)),
            );
            t.reset();
        }
    });
}
