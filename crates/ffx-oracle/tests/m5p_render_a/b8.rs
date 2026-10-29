// Batch 8 of tests/m5p_render_a.rs (included there; uses b6.rs's and
// b7.rs's frame helpers).

use ffx_prender::m5p_render_a9::{self as m9, bridge as b9};

#[test]
fn texture3d_create_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a801);
        for case in 0..60u32 {
            let this = t.zeros(0x80);
            let base = t.zeros(0x30);
            t.w32(this, base);
            t.w32(base + 0xc, [4, 8, 16, 32][rng.below(4) as usize]);
            t.w32(base + 0x10, rng.below(4));
            let fmt = rng.word();
            t.w32(base + 0x18, fmt);
            t.w32(this + 0x14, rng.next() & 0x106);
            t.w32(this + 0x10, rng.below(2));
            // at most 32 mips: more overwrite the original's cookie (it aborts)
            t.w32(this + 0xc, [0, 1, 2, 5, 31][rng.below(5) as usize]);
            for off in [0x1c, 0x20, 0x24] {
                let v = [0, 1, 3, 16, 33][rng.below(5) as usize];
                t.w32(this + off, v);
            }
            // the textures: fake objects whose GetDesc logs
            let get_desc = t.stub_code(1, false, 2, 2, Ret::Val(0), false);
            let tex: Vec<u32> = (0..3).map(|_| t.object(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, get_desc], 8)).collect();
            let elems: Vec<u32> = (0..15).map(|i| if i % 5 == 0 { tex[i / 5] } else { 0 }).collect();
            let elems = t.words(&elems);
            let n = [1, 2, 3, 0x8000_0003][rng.below(4) as usize];
            t.w32(this + 0x28, n);
            t.w32(this + 0x2c, if n & 0x7fff_ffff > 1 { elems } else { tex[0] });
            // the device
            let hrs: Vec<u32> = (0..4).map(|_| u32::from(rng.chance(5))).collect();
            let seq = t.seq(&hrs);
            let create = t.stub_code(2, false, 4, 4, seq, false);
            let mut slots = vec![0u32; 8];
            slots[6] = create;
            let dev = if case % 11 == 10 { 0 } else { t.object(&slots, 0x10) };
            t.global(m8::HOLDER_DEVICE, dev);
            let g = rng.next() & !1;
            t.global(m8::HOLDER_GUARD, g | (case & 1));
            t.stub_sym(m8::HOLDER_CTOR, 3, true, 0, 0, Ret::Val(3));
            t.stub_sym(0x0094_9207, 4, false, 1, 0, Ret::Val(0));
            t.stub_sym(m8::SCOPE_ENTER, 5, true, 0, 0, Ret::Val(5));
            t.stub_sym(m8::SCOPE_LEAVE, 6, true, 0, 0, Ret::Val(6));
            // no more views than textures (past them the original reads 0)
            let have = n & 0x7fff_ffff;
            let counts: Vec<u32> = (0..4).map(|_| 1 + rng.below(have)).collect();
            let seq = t.seq(&counts);
            t.stub_sym(m8::VIEW_COUNT, 7, true, 0, 0, seq);
            t.stub_sym(m9::CREATE_VIEWS, 8, true, 0, 0, Ret::Val(rng.below(2) * 9));
            let data = if rng.chance(2) { 0 } else { rng.next() };
            let views = rng.next() & 0x101;
            let skip = u32::from(case % 5 == 0);
            let unused = rng.next();
            let frame = frame_for(t, 4, 0x1e0);
            t.check(
                "PTexture3DD3D11_create_005903a0",
                0x0059_03a0,
                Conv::Thiscall(this),
                &[data, views, unused, skip],
                |g| Out::U32(b9::create(g, this, data, views as u8, skip as u8, frame)),
            );
            if dev != 0 && t.calls() > 2 {
                let local = |off: u32| frame + 0x1e0 - off;
                same_bytes(t, local(0x1b4), 0x24, "PTexture3DD3D11_create_005903a0: the description");
                let mips = Mem::read::<u32>(&t.b, local(0x1b4) + 0xc).min(b9::SUBRESOURCES);
                same_bytes(
                    t,
                    local(0x190),
                    12 * mips as usize,
                    "PTexture3DD3D11_create_005903a0: the subresources",
                );
            }
            t.b_top = None;
            t.reset();
        }
    });
}
