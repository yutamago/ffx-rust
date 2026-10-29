// Batch 5 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a5::{self as m5, Step, bridge as b5};

#[test]
fn dtor_sequences_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e001);
        for &(addr, steps) in m5::DTOR_SEQUENCES {
            let this = t.zeros(0x1400);
            for (tag, s) in (1u32..).zip(steps) {
                match *s {
                    Step::Member(f, _) => {
                        t.stub_sym(f, tag, true, 0, 0, Ret::Val(tag));
                    }
                    Step::Vector(..) => {
                        t.stub_sym(m2::EHVEC_DTOR, 99, false, 4, 4, Ret::Val(0x99));
                    }
                }
            }
            t.check(&format!("{addr:#x}"), addr, Conv::Thiscall(this), &[], |g| {
                Out::U32(b5::dtor_sequence(g, this, steps))
            });
            assert_eq!(t.calls() as usize, steps.len());
            t.reset();
        }
        // the tree: 0..3 nodes before the sentinel or a null
        for case in 0..8u32 {
            let this = t.zeros(0x40);
            let n = rng.below(4);
            let firsts: Vec<u32> = (0..n)
                .map(|_| rng.next() | 1)
                .chain([if case % 2 == 0 { this } else { 0 }])
                .collect();
            t.w32(this + 0xc, firsts[0]);
            let seq = t.seq(&firsts[1..]);
            let Ret::Seq(cell) = seq else { unreachable!() };
            // a stub that stores the next node at this + 0xc
            let mut code = vec![0x8b, 0x15];
            code.extend(cell.to_le_bytes()); // mov edx, [cell]
            code.extend([0x8b, 0x02]); // mov eax, [edx]
            code.extend([0x83, 0x05]);
            code.extend(cell.to_le_bytes());
            code.push(0x04); // add dword [cell], 4
            code.extend([0x89, 0x41, 0x0c]); // mov [ecx+0xc], eax
            code.extend([0xc2, 0x04, 0x00]); // ret 4
            let st = t.a.alloc_code(&code).unwrap();
            assert_eq!(t.b.alloc_code(&code).unwrap(), st);
            t.jump(0x0057_5000, st);
            t.stub_sym(0x0049_cc10, 2, true, 0, 0, Ret::Val(2));
            t.stub_sym(0x0049_c5b0, 3, true, 0, 0, Ret::Val(3));
            t.stub_sym(0x0057_4760, 4, true, 0, 0, Ret::Val(case + 4));
            // both sides start the node sequence from the same table
            let table = t.words(&firsts[1..]);
            t.w32(cell, table);
            t.check("PRedBlackTree_dtor_00584aa0", 0x0058_4aa0, Conv::Thiscall(this), &[], |g| {
                Out::U32(b5::tree_dtor(g, this))
            });
            t.reset();
        }
    });
}

#[test]
fn sprite_bookkeeping_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e002);
        for case in 0..40u32 {
            // a collection of capacity 6 with some frames in use
            let cap = 6u32;
            let this = t.zeros(0x100);
            let used = rng.below(cap + 1);
            let records = if case % 9 == 8 { 0 } else { t.zeros(48 * 8) };
            let order: Vec<u32> = (0..8).map(|i| (i + 1) % 8).collect();
            let order = t.words(&order);
            let back: Vec<u32> = (0..8).collect();
            let back = t.words(&back);
            t.w32(this + 0x1c, cap);
            t.w32(this + 0x20, used);
            t.w32(this + 0xb8, records);
            t.w32(this + 0xc8, order);
            t.w32(this + 0xd0, back);
            t.w32(this + 0xc0, rng.below(6));
            if records != 0 {
                let fill: Vec<u8> = (0..48 * 8).map(|_| rng.next() as u8).collect();
                t.write(records, &fill);
            }
            let out = t.zeros(4);
            t.check("PSpriteCollection_addFrame_0056c150", 0x0056_c150, Conv::Thiscall(this), &[out], |g| {
                Out::U32(b5::add_frame(g, this, out))
            });
            // remove: a frame whose slot is in use or not
            let frame = rng.below(6);
            t.check(
                "PSpriteCollection_removeFrame_0056df90",
                0x0056_df90,
                Conv::Thiscall(this),
                &[frame],
                |g| Out::U32(b5::remove_frame(g, this, frame)),
            );
            // sizes over the texture
            let tex = t.zeros(0x30);
            let (tw, th) = (rng.word(), rng.word());
            t.w32(tex + 0x1c, tw);
            t.w32(tex + 0x20, th);
            t.w32(this + 0x90, tex);
            for &(addr, off) in m5::SPRITE_SIZES {
                let (w, h) = (rng.word(), rng.word());
                let frame = rng.below(6);
                t.check(
                    &format!("{addr:#x}"),
                    addr,
                    Conv::Thiscall(this),
                    &[frame, w, h],
                    |g| Out::U32(b5::sprite_size(g, this, frame, w, h, off)),
                );
            }
            // the id wrapper over the real FUN_0056c150
            if case % 5 == 0 {
                t.w32(this + 0xc0, 0x8000_0000 | rng.below(4));
            }
            let slot = t.b.stack_range().start + 0x80;
            t.check("PSpriteCollection_addFrameId_0056c230", 0x0056_c230, Conv::Thiscall(this), &[], |g| {
                Out::U32(b5::add_frame_id(g, this, slot))
            });
            t.reset();
        }
    });
}

#[test]
fn sprite_buffers_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_e003);
        for case in 0..40u32 {
            let this = t.zeros(0x60);
            let fill: Vec<u32> = (0..0x10).map(|_| rng.next() | 1).collect();
            let vb = t.words(&fill);
            t.w32(this + 0x50, vb);
            let n = rng.below(4);
            let cnt = if rng.chance(2) { n.max(1) } else { rng.word() };
            t.w32(vb + 0x18, cnt);
            let bound: Vec<u32> = (0..3).map(|_| rng.next() & 0x101).collect();
            let seq = t.seq(&bound);
            t.stub_sym(0x0056_2da0, 1, true, 0, 0, seq);
            t.stub_sym(0x0056_46d0, 2, true, 0, 0, Ret::Val(rng.below(2) * 3));
            t.stub_sym(0x0043_52f0, 3, false, 2, 0, Ret::Val(0));
            t.stub_sym(0x0056_3fb0, 4, true, 1, 1, Ret::Val(rng.below(2) * 0xd));
            t.stub_sym(0x0056_1a20, 5, true, 0, 0, Ret::Val(rng.below(2) * 7));
            t.check(
                "PSpriteCollection_setVertexCount_0056e410",
                0x0056_e410,
                Conv::Thiscall(this),
                &[n],
                |g| Out::U32(b5::set_vertex_count(g, this, n)),
            );
            t.reset();
            // the word push over the real Phyre_PArray_pushInto4
            let arr = t.zeros(0x20);
            let words: Vec<u32> = (0..4).map(|_| rng.next()).collect();
            let data = t.words(&words);
            let count = [0, 1, 3, 0x8000_0002][case as usize % 4];
            t.w32(arr + 0xc, count);
            t.w32(arr + 0x10, data);
            let block = if case % 6 == 5 { 0 } else { t.zeros(0x20) };
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 6, false, 2, 0, Ret::Val(block));
            t.stub_sym(ma::PHYRE_FREE, 7, false, 1, 0, Ret::Val(7));
            let v = rng.next();
            let slot = t.b.stack_range().start + 0x80;
            t.check(
                "PSpriteAnimationInfo_pushWord_0056c270",
                0x0056_c270,
                Conv::Thiscall(arr),
                &[v],
                |g| Out::U32(b5::push_word(g, arr, v, slot)),
            );
            // the subtexture records
            let n = rng.below(4);
            let src = t.zeros(0x80);
            for i in 0..4 {
                let r = src + 20 * i;
                let s = pstring(t, &mut rng);
                t.w32(r, s);
                for k in 1..5 {
                    let f = float_bits(&mut rng);
                    t.w32(r + 4 * k, f);
                }
            }
            let at0 = case % 7 == 6;
            let (dst, n) = if at0 { (0, 1) } else { (t.zeros(0x80), n) };
            let blocks: Vec<u32> = (0..4)
                .map(|_| if rng.chance(4) { 0 } else { t.zeros(0x10) })
                .collect();
            let seq = t.seq(&blocks);
            t.stub_sym(ma::PHYRE_MALLOC_ALIGNED, 8, false, 2, 0, seq);
            t.stub_sym(ffx_prender::m5p_render_a3::MEMCPY, 9, false, 3, 0, Ret::Val(9));
            t.check(
                "PSubTextureInfo_copyRecords_00565110",
                0x0056_5110,
                Conv::Cdecl,
                &[dst, src, n],
                |g| {
                    b5::subtextures_copy(g, dst, src, n);
                    Out::None
                },
            );
            t.reset();
        }
    });
}
