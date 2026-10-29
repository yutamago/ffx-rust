// Batch 4 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a4::{self as m4, bridge as b4};

/// A `PString` word: none, or a text (tagged half the time).
fn pstring(t: &mut Twins, rng: &mut Rng) -> u32 {
    if rng.chance(3) {
        return rng.below(2);
    }
    let text: Vec<u8> = (0..rng.below(9))
        .map(|_| (rng.below(90) + 32) as u8)
        .chain([0])
        .collect();
    t.alloc(&text) | rng.below(2)
}

#[test]
fn statics_b4_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_d001);
        for case in 0..6u32 {
            t.stub_sym(0x0047_fe60, 1, false, 1, 0, Ret::Val(case));
            t.check(
                "PUtilityGeometry_releaseAll_00432430",
                0x0043_2430,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b4::release_all(g)),
            );
            let k = rng.below(16) as usize;
            let rets: Vec<u32> = (0..16).map(|i| if i == k { 5 } else { 0 }).collect();
            let seq = t.seq(&rets);
            t.stub_sym(0x005c_9920, 2, false, 1, 0, seq);
            t.check(
                "PUtilityDynamicGeometry_unlinkAll_00504040",
                0x0050_4040,
                Conv::Cdecl,
                &[],
                |g| Out::U32(b4::unlink_all(g)),
            );
            assert!(t.calls() >= 27);
            t.reset();
        }
    });
}

#[test]
fn pass_records_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_d002);
        for case in 0..30u32 {
            let n = rng.below(4);
            let at0 = case % 6 == 5;
            let n = if at0 { 1 } else { n };
            let src = t.zeros(0x100);
            for i in 0..(0x100 / 4) {
                let v = rng.next();
                t.w32(src + 4 * i, v);
            }
            // array counts: 0, small, external
            for r in 0..4 {
                for off in [4u32, 0xc, 0x14] {
                    let c = [0, 1, 3, 0x8000_0002][rng.below(4) as usize];
                    t.w32(src + 32 * r + off, c);
                }
            }
            let dst = if at0 { 0 } else { t.zeros(0x100) };
            let blocks: Vec<u32> = (0..64)
                .map(|_| if rng.chance(5) { 0 } else { rng.next() | 1 })
                .collect();
            let seq = t.seq(&blocks);
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, seq);
            t.stub_sym(0x004b_aed0, 2, false, 3, 0, Ret::Val(2));
            t.stub_sym(0x004a_3530, 3, false, 3, 0, Ret::Val(3));
            t.stub_sym(0x004a_3620, 4, false, 3, 0, Ret::Val(4));
            t.check(
                "PSceneRenderPass_copyRecords_004d8a30",
                0x004d_8a30,
                Conv::Cdecl,
                &[dst, src, n],
                |g| {
                    b4::pass_records_copy(g, dst, src, n);
                    Out::None
                },
            );
            t.reset();
            // records whose string words are all PStrings
            let src = t.zeros(0x100);
            for i in 0..(0x100 / 4) {
                let v = pstring(t, &mut rng);
                t.w32(src + 4 * i, v);
            }
            let blocks: Vec<u32> = (0..64)
                .map(|_| if rng.chance(5) { 0 } else { t.zeros(0x10) })
                .collect();
            let seq = t.seq(&blocks);
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, seq);
            t.stub_sym(ffx_prender::m5p_render_a3::MEMCPY, 5, false, 3, 0, Ret::Val(5));
            t.check(
                "PShaderPassInfo_copyRecords_004d8bd0",
                0x004d_8bd0,
                Conv::Cdecl,
                &[dst, src, n],
                |g| {
                    b4::pass_infos_copy(g, dst, src, n);
                    Out::None
                },
            );
            let Ret::Seq(cell) = seq else { unreachable!() };
            let table = t.words(&blocks);
            t.w32(cell, table);
            t.check(
                "PMaterialSwitch_copyRecords_004e19c0",
                0x004e_19c0,
                Conv::Cdecl,
                &[dst, src, n],
                |g| {
                    b4::string_pairs_copy(g, dst, src, n);
                    Out::None
                },
            );
            t.reset();
            // the resizes
            for &(addr, size, copy, init, release) in m4::COPY_INIT_RESIZES {
                let this = t.zeros(0x10);
                header(t, &mut rng, this);
                let hw = t.r32(this);
                let n = match rng.below(4) {
                    0 => hw,
                    1 => 0,
                    _ => rng.below(6),
                };
                // an owned null block to replace, now and then
                if case % 4 == 0 {
                    t.w32(this, 2);
                    t.w32(this + 4, 0);
                }
                let block = match rng.below(5) {
                    0 => 0,
                    1 => t.r32(this + 4),
                    _ => rng.next() | 1,
                };
                t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 1, false, 2, 0, Ret::Val(block));
                t.stub_sym(copy, 2, false, 3, 0, Ret::Val(2));
                t.stub_sym(init, 3, false, 2, 0, Ret::Val(3));
                t.stub_sym(ma::PHYRE_FREE, 4, false, 1, 0, Ret::Val(4));
                if release != 0 {
                    t.stub_sym(release, 5, false, 2, 0, Ret::Val(5));
                }
                t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[n], |g| {
                    Out::U32(b4::copy_init_resize(g, this, n, size, copy, init, release))
                });
                t.reset();
            }
        }
    });
}

#[test]
fn physics_decoders_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_d003);
        // every exponent and sign, fractions 0 and random (no NaN payloads:
        // the half NaN widens to a signalling NaN the emulator keeps)
        for e in 0..32u32 {
            for s in 0..2u32 {
                for k in 0..3 {
                    let mant = if k == 0 { 0 } else { rng.below(0x400) };
                    if e == 31 && mant != 0 {
                        continue;
                    }
                    let half = (s << 15) | (e << 10) | mant | (rng.next() & 0xffff_0000);
                    t.check(
                        "PPhysicsMesh_halfToFloat_0049a470",
                        0x0049_a470,
                        Conv::Cdecl,
                        &[half],
                        |g| Out::F32(b4::half_to_float(g, half)),
                    );
                }
            }
        }
        for case in 0..40u32 {
            let this = t.words(&[rng.next()]);
            let out = t.zeros(0x10);
            t.check(
                "PPhysicsMesh_unpackNormal_0049bbe0",
                0x0049_bbe0,
                Conv::Thiscall(this),
                &[out, out + 4, out + 8],
                |g| Out::U32(b4::unpack_normal(g, this, out, out + 4, out + 8)),
            );
            let f = rng.below(60);
            t.check(
                "PPhysicsMesh_formatSize_0055bb40",
                0x0055_bb40,
                Conv::Cdecl,
                &[f],
                |_| Out::U32(b4::format_size(f)),
            );
            // the stream decode: format 2, no array, a failed resize, a decode
            let stream = t.zeros(0x20);
            let fmt = [2u8, 0, 1, 3][case as usize % 4];
            t.w8(stream + 0x18, fmt);
            let cnt = rng.below(9);
            t.w32(stream + 0xc, cnt);
            let sdata = rng.next();
            t.w32(stream + 0x14, sdata);
            let arr = if case % 5 == 1 { 0 } else { t.words(&[0, rng.next()]) };
            t.stub_sym(0x0049_bfc0, 1, true, 1, 1, Ret::Val(rng.below(2) * 0xd));
            t.stub_sym(0x0049_a520, 2, false, 4, 0, Ret::Val(2));
            t.check(
                "PPhysicsMesh_decodeStream_0049bec0",
                0x0049_bec0,
                Conv::Thiscall(stream),
                &[arr],
                |g| Out::U32(b4::decode_stream(g, stream, arr)),
            );
            t.reset();
        }
    });
}

#[test]
fn small_classes_b4_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_d004);
        for case in 0..30u32 {
            let fill: Vec<u32> = (0..0x20).map(|_| rng.next()).collect();
            let this = t.words(&fill);
            t.stub_sym(0x0094_966a, 1, false, 5, 5, Ret::Val(1));
            t.stub_sym(0x005b_3620, 2, true, 0, 0, Ret::Val(2));
            t.check("PZeroQuads_ctor_004fef30", 0x004f_ef30, Conv::Thiscall(this), &[], |g| {
                Out::U32(b4::quads_ctor(g, this))
            });
            header(t, &mut rng, this + 0xc);
            let s = pstring(t, &mut rng);
            t.w32(this, s);
            t.stub_sym(ma::PHYRE_FREE, 3, false, 1, 0, Ret::Val(3));
            t.check(
                "PSpriteAnimationInfoChar_dtor_00567590",
                0x0056_7590,
                Conv::Thiscall(this),
                &[],
                |g| {
                    b4::animation_info_char_dtor(g, this);
                    Out::None
                },
            );
            // the class chain: each class at some depth, none, no object
            let classes = m4::MATRIX_SOURCE_CLASSES;
            let depth = rng.below(3);
            let pick = rng.below(4) as usize;
            let mut desc = 0;
            for i in 0..=depth {
                let d = t.zeros(0x48);
                t.w32(d + 0x40, desc);
                desc = d;
                if i == 0 && pick < 3 {
                    let c = t.rebase(classes[pick]);
                    // the chain goes on through the real descriptor object
                    t.w32(d + 0x40, c);
                }
            }
            t.w32(this + 0x64, desc);
            let obj = match case % 4 {
                0 => 0,
                1 => 4,
                _ => t.zeros(0x10),
            };
            t.w32(this + 0x60, obj);
            let out = t.zeros(0x40);
            t.stub_sym(0x0045_1080, 4, true, 1, 1, Ret::Val(4));
            t.stub_sym(0x004f_71e0, 5, true, 2, 2, Ret::Val(5));
            t.stub_sym(0x0050_6660, 6, true, 1, 1, Ret::Val(6));
            t.stub_sym(0x0045_1bb0, 7, false, 1, 0, Ret::Val(7));
            t.check(
                "PGameplay_objectMatrix_0055b490",
                0x0055_b490,
                Conv::Thiscall(this),
                &[out],
                |g| Out::U32(b4::object_matrix(g, this, out)),
            );
            t.reset();
        }
    });
}
