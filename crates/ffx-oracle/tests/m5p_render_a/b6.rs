// Batch 6 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a6::{self as m6, bridge as b6};
use ffx_prender::m5p_render_a7::bridge as b7;

/// A matrix of small floats (exact sums and products stay out of the
/// way of nothing: the math runs in the original code on both sides).
fn matrix(t: &mut Twins, rng: &mut Rng) -> u32 {
    let m: Vec<u32> = (0..16)
        .map(|_| ((rng.below(41) as f32 - 20.0) / 8.0).to_bits())
        .collect();
    t.words(&m)
}

/// The original's frame base (`ebp - size`) for a call with `words`
/// stack words, zeroed on both sides, and the call top below it for the
/// port's callees.
fn frame_for(t: &mut Twins, words: u32, size: u32) -> u32 {
    let ebp = call_top(&t.a) - 4 * words - 8;
    let frame = ebp - size;
    t.write(frame, &vec![0; size as usize]);
    t.b_top = Some(frame - 0x40);
    frame
}

/// The frames after the call: what the callees wrote there must agree.
fn same_frame(t: &Twins, frame: u32, size: u32, what: &str) {
    let mut x = vec![0; size as usize];
    let mut y = vec![0; size as usize];
    Mem::read_bytes(&t.a, frame, &mut x);
    Mem::read_bytes(&t.b, frame, &mut y);
    assert_eq!(x, y, "{what}: the frame");
}

#[test]
fn draws_and_slots_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f001);
        for case in 0..30u32 {
            // the indexed draw on a fake context
            let m24 = t.stub_code(1, false, 2, 2, Ret::Val(0), false);
            let m19 = t.stub_code(2, false, 4, 4, Ret::Val(0), false);
            let m20 = t.stub_code(3, false, 6, 6, Ret::Val(0x5555_0000), false);
            let mut slots = vec![0u32; 25];
            slots[24] = m24;
            slots[19] = m19;
            slots[20] = m20;
            let ctx = t.object(&slots, 0x10);
            let r = t.zeros(0x80);
            t.w32(r + 0x70, ctx);
            let ind = t.zeros(0x40);
            t.w8(ind + 0xc, [8u8, 4, 2][case as usize % 3]);
            let off = rng.word();
            t.w32(ind + 0x2c, off);
            t.w8(ind + 0x30, rng.next() as u8);
            let n = [1, 2, 3, 0x8000_0002][rng.below(4) as usize];
            t.w32(ind + 0x14, n);
            let elems: Vec<u32> = (0..20).map(|_| rng.next()).collect();
            let elems = t.words(&elems);
            if n & 0x7fff_ffff > 1 {
                t.w32(ind + 0x18, elems);
            } else {
                let v = rng.next();
                t.w32(ind + 0x18, v);
            }
            let index = if n & 0x7fff_ffff > 1 { rng.below(3) } else { 0 };
            let topo = rng.below(3);
            t.stub_sym(0x0058_ab60, 4, false, 1, 0, Ret::Val(topo));
            let (count, inst) = (rng.word(), rng.word());
            t.check(
                "PRendererD3D11_drawIndexedInstanced_00592de0",
                0x0059_2de0,
                Conv::Thiscall(r),
                &[case, count, ind, index, inst],
                |g| Out::U8(b6::draw_indexed_instanced(g, r, case, count, ind, index, inst)),
            );
            t.reset();
            // the slot table
            let this = t.zeros(0x10);
            let n = if case % 5 == 0 { 20 } else { rng.below(5) };
            let table = t.zeros(0x100);
            for i in 0..n {
                t.w8(table + 8 * i, u8::from(rng.chance(3)));
                let sz = if rng.chance(3) { 0 } else { rng.below(6) * 16 };
                t.w32(table + 8 * i + 4, sz);
            }
            t.w32(this + 4, if n == 0 && case % 2 == 0 { 0 } else { table });
            t.w32(this + 8, n);
            let block = if case % 7 == 6 {
                0
            } else {
                let fill: Vec<u32> = (0..0x80).map(|_| rng.next()).collect();
                t.words(&fill)
            };
            t.stub_sym(m6::PHYRE_MALLOC, 1, false, 1, 0, Ret::Val(block));
            t.stub_sym(ma::PHYRE_FREE, 2, false, 1, 0, Ret::Val(0x7777_7700));
            t.check(
                "Phyre_PAnimation_PNetworkOptimizeBufferAllocator_grow_00616330",
                0x0061_6330,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b6::grow_slots(g, this)),
            );
            t.reset();
            // the search: an accepting object answering from a table
            let this = t.zeros(0x10);
            let n = 1 + rng.below(5);
            let table = t.zeros(0x40);
            for i in 0..n {
                t.w8(table + 8 * i, u8::from(rng.chance(3)));
                let sz = if rng.chance(4) { 0 } else { rng.below(6) * 16 };
                t.w32(table + 8 * i + 4, sz);
            }
            t.w32(this + 4, table);
            t.w32(this + 8, n);
            let answers: Vec<u32> = (0..64).map(|_| rng.next() & 0x101).collect();
            let seq = t.seq(&answers);
            let pred_fn = t.stub_code(5, true, 2, 2, seq, false);
            let accept = t.object(&[pred_fn], 8);
            // the real grow (a fresh block, or none) and the real retry
            let block = if rng.chance(3) { 0 } else { t.zeros(0x200) };
            t.stub_sym(m6::PHYRE_MALLOC, 6, false, 1, 0, Ret::Val(block));
            t.stub_sym(ma::PHYRE_FREE, 7, false, 1, 0, Ret::Val(7));
            // none big enough, with equal sizes: the largest search's ties
            let size = if case % 4 == 0 {
                for i in 0..n {
                    t.w8(table + 8 * i, 0);
                    t.w32(table + 8 * i + 4, 0x40);
                }
                0x1000
            } else {
                rng.below(6) * 16
            };
            t.check(
                "Phyre_PAnimation_PNetworkOptimizeBufferAllocator_findSlot_006161e0",
                0x0061_61e0,
                Conv::Thiscall(this),
                &[size, accept],
                |g| Out::U32(b6::find_slot(g, this, size, accept)),
            );
            t.reset();
        }
    });
}

#[test]
fn scene_matrices_match_the_original() {
    // The Vectormath callees run as they are on both sides; the frames
    // sit where the original's are, so pointers handed on agree.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f002);
        for case in 0..16u32 {
            // a node with 0..3 parents
            let mut parent = 0;
            for _ in 0..rng.below(4) {
                let p = t.zeros(0x60);
                let m = matrix(t, &mut rng);
                for i in 0..16 {
                    let v = t.r32(m + 4 * i);
                    t.w32(p + 0x10 + 4 * i, v);
                }
                t.w32(p + 4, parent);
                parent = p;
            }
            let node = t.zeros(0x60);
            let m = matrix(t, &mut rng);
            for i in 0..16 {
                let v = t.r32(m + 4 * i);
                t.w32(node + 0x10 + 4 * i, v);
            }
            t.w32(node + 4, parent);
            let out = t.zeros(0x40);
            let frame = frame_for(t, 1, 0x100);
            t.check(
                "PUtilityScene_nodeWorldMatrix_00506660",
                0x0050_6660,
                Conv::Thiscall(node),
                &[out],
                |g| Out::U32(b7::node_world_matrix(g, node, out, frame)),
            );
            same_frame(t, frame, 0x100, "PUtilityScene_nodeWorldMatrix_00506660");
            t.b_top = None;
            // the locator: a parent with its own matrix object or a composed one
            let this = t.zeros(0x60);
            let has_parent = case % 4 != 0;
            if has_parent {
                let own = if case % 2 == 0 { matrix(t, &mut rng) } else { 0 };
                t.w32(node + 0xc, own);
                t.w32(this + 4, node);
            }
            let obj = if case % 3 == 0 { 0 } else { t.zeros(0x40) };
            t.w32(this + 0xc, obj);
            let target = matrix(t, &mut rng);
            let frame = frame_for(t, 1, 0x100);
            t.check(
                "PUtilityScene_setWorldMatrix_00507410",
                0x0050_7410,
                Conv::Thiscall(this),
                &[target],
                |g| Out::U32(b7::set_world_matrix(g, this, target, frame)),
            );
            same_frame(t, frame, 0x100, "PUtilityScene_setWorldMatrix_00507410");
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn gameplay_update_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_f003);
        for case in 0..40u32 {
            let this = t.zeros(0x80);
            t.w8(this + 0x68, u8::from(case % 9 != 0));
            let target = match rng.below(3) {
                0 => 0,
                1 => t.words(&[0, 0]),
                _ => {
                    let w = t.zeros(0x40);
                    t.words(&[0, w])
                }
            };
            t.w32(this + 4, target);
            let scene = if rng.chance(2) { t.zeros(0x40) } else { 0 };
            t.w32(this + 0x58, scene);
            let src = if case % 7 == 0 { 0 } else { matrix(t, &mut rng) };
            t.w32(this + 0x60, src);
            let k = rng.word();
            t.w32(this + 0x5c, k);
            let offset = matrix(t, &mut rng);
            for i in 0..16 {
                let v = t.r32(offset + 4 * i);
                t.w32(this + 0xc + 4 * i, v);
            }
            // the class chain
            let classes = m4::MATRIX_SOURCE_CLASSES;
            let pick = rng.below(4) as usize;
            let d = t.zeros(0x48);
            if pick < 2 {
                let c = t.rebase(classes[pick]);
                t.w32(d + 0x40, c);
            }
            t.w32(this + 0x64, d);
            let node = if rng.chance(3) { 0 } else { t.zeros(0x60) };
            t.stub_sym(0x0053_37d0, 1, true, 0, 0, Ret::Val(node));
            t.stub_sym(0x0050_6660, 2, true, 1, 1, Ret::Val(3));
            t.stub_sym(0x004f_71e0, 3, true, 2, 2, Ret::Val(4));
            t.stub_sym(0x0050_7410, 4, true, 1, 1, Ret::Val(5));
            t.stub_sym(0x0045_1190, 5, true, 1, 1, Ret::Val(6));
            t.stub_sym(0x0055_b780, 6, true, 1, 1, Ret::Val(7));
            let frame = frame_for(t, 0, 0x80);
            t.check(
                "PGameplay_updateObject_0055b670",
                0x0055_b670,
                Conv::Thiscall(this),
                &[],
                |g| {
                    b7::update_object(g, this, frame);
                    Out::None
                },
            );
            same_frame(t, frame, 0x80, "PGameplay_updateObject_0055b670");
            t.b_top = None;
            t.reset();
        }
    });
}
