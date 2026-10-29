// Batch 7 of tests/m5p_render_a.rs (included there; uses b6.rs's
// frame helpers).

use ffx_prender::m5p_render_a8::{self as m8, bridge as b8};

/// A stub over `sym` (thiscall, one stack word) that stores `value` at
/// the address its argument holds and returns `ret`.
fn store_stub(t: &mut Twins, sym: u32, value: u32, ret: u32) {
    let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
    code.extend([0xc7, 0x00]);
    code.extend(value.to_le_bytes()); // mov [eax], value
    code.push(0xb8);
    code.extend(ret.to_le_bytes()); // mov eax, ret
    code.extend([0xc2, 0x04, 0x00]); // ret 4
    let at = t.a.alloc_code(&code).unwrap();
    assert_eq!(t.b.alloc_code(&code).unwrap(), at);
    t.jump(sym, at);
}

/// The same bytes in both oracles at `at..at + n`.
fn same_bytes(t: &Twins, at: u32, n: usize, what: &str) {
    let mut x = vec![0; n];
    let mut y = vec![0; n];
    Mem::read_bytes(&t.a, at, &mut x);
    Mem::read_bytes(&t.b, at, &mut y);
    assert_eq!(x, y, "{what}");
}

#[test]
fn sprite_rotation_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a701);
        for case in 0..60u32 {
            let this = t.zeros(0x100);
            let records = if case % 11 == 10 { 0 } else { t.zeros(48 * 8) };
            let order: Vec<u32> = (0..8).map(|_| rng.below(9)).collect();
            let order = t.words(&order);
            t.w32(this + 0xb8, records);
            t.w32(this + 0xc8, order);
            t.w32(this + 0x20, 1 + rng.below(6));
            let frame = rng.below(8);
            let angle = match case % 4 {
                0 => float_bits(&mut rng),
                _ => ((rng.below(2000) as f32 - 1000.0) / 100.0).to_bits(),
            };
            t.check(
                "PSpriteCollection_setFrameRotation_0056e730",
                0x0056_e730,
                Conv::Thiscall(this),
                &[frame, angle],
                |g| Out::U32(b8::sprite_rotation(g, this, frame, angle)),
            );
            t.reset();
        }
    });
}

#[test]
fn texture3d_views_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a702);
        for case in 0..30u32 {
            let this = t.zeros(0x80);
            t.w8(this + 0x58, rng.next() as u8 | if case % 4 == 0 { 0 } else { 8 });
            let base = t.zeros(0x30);
            let fmt = rng.word();
            t.w32(base + 0x20, fmt);
            t.w32(this, base);
            let mips = rng.below(12);
            t.w32(this + 0x4c, mips);
            let n = [1, 2, 3, 0x8000_0002][rng.below(4) as usize];
            t.w32(this + 0x28, n);
            let elems: Vec<u32> = (0..15).map(|_| rng.next()).collect();
            let elems = t.words(&elems);
            if n & 0x7fff_ffff > 1 {
                t.w32(this + 0x2c, elems);
            }
            // the device: slot 7 answers from a table of HRESULTs
            let hrs: Vec<u32> = (0..8)
                .map(|_| if rng.chance(4) { 0x8007_000e } else { rng.below(2) })
                .collect();
            let seq = t.seq(&hrs);
            let srv = t.stub_code(1, false, 4, 4, seq, false);
            let mut slots = vec![0u32; 8];
            slots[7] = srv;
            let dev = t.object(&slots, 0x10);
            t.global(m8::HOLDER_DEVICE, dev);
            let g = rng.next() & !1;
            t.global(m8::HOLDER_GUARD, g | (case & 1));
            t.stub_sym(m8::HOLDER_CTOR, 2, true, 0, 0, Ret::Val(2));
            t.stub_sym(0x0094_9207, 3, false, 1, 0, Ret::Val(0));
            t.stub_sym(m8::SCOPE_ENTER, 4, true, 0, 0, Ret::Val(4));
            t.stub_sym(m8::SCOPE_LEAVE, 5, true, 0, 0, Ret::Val(5));
            let counts: Vec<u32> = (0..8).map(|_| 1 + rng.below(3)).collect();
            let seq = t.seq(&counts);
            t.stub_sym(m8::VIEW_COUNT, 6, true, 0, 0, seq);
            let frame = frame_for(t, 0, 0x30);
            t.check(
                "PTexture3DD3D11_createViews_0058f880",
                0x0058_f880,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b8::create_views(g, this, frame)),
            );
            if case % 4 != 0 {
                same_bytes(t, frame, 0x18, "PTexture3DD3D11_createViews_0058f880: the description");
            }
            t.b_top = None;
            t.reset();
        }
    });
}

#[test]
fn sprite_flush_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a703);
        for case in 0..40u32 {
            let this = t.zeros(0x100);
            let count = 1 + rng.below(4);
            let records = if case % 9 == 8 { 0 } else { t.zeros(48 * 5) };
            // signed zeros everywhere half the time: the bounds' ties
            let zeros = case % 2 == 1;
            for i in 0..(48 * 5 / 4) {
                let v = if zeros {
                    rng.below(2) << 31
                } else if rng.chance(3) {
                    float_bits(&mut rng)
                } else {
                    ((rng.below(400) as f32 - 200.0) / 4.0).to_bits()
                };
                if records != 0 {
                    t.w32(records + 4 * i, v);
                }
            }
            t.w32(this + 0xb8, records);
            t.w32(this + 0x20, count);
            t.w8(this + 4, rng.below(2) as u8);
            let vb = t.zeros(0x40);
            t.w32(this + 0x50, vb);
            t.w32(vb + 0x10, rng.below(3));
            t.w32(vb + 0x18, rng.below(4) | (rng.next() & 0x8000_0000));
            let owner_obj = t.zeros(0x20);
            let owner = if case % 3 == 0 { 0 } else { t.zeros(0x10) };
            t.w32(owner_obj + 0x1c, owner);
            t.w32(this, owner_obj);
            let data = t.zeros(0x200);
            let map_fails = case % 13 == 12;
            let unmap_fails = case % 7 == 6;
            store_stub(t, 0x0043_4c50, data, if map_fails { 7 } else { 0 });
            t.stub_sym(ffx_prender::m5p_render_a3::MEMCPY, 1, false, 3, 0, Ret::Val(1));
            t.import_sym(m8::QSORT, 2, 4, 0, Ret::Val(2));
            t.stub_sym(0x0056_4cb0, 3, true, 2, 2, Ret::Val(if unmap_fails { 5 } else { 0 }));
            t.stub_sym(0x0050_0bd0, 4, true, 2, 2, Ret::Val(4));
            let frame = frame_for(t, 0, 0x48);
            t.check(
                "PSpriteCollection_flush_0056ee50",
                0x0056_ee50,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(b8::flush(g, this, frame)),
            );
            if owner != 0 && count != 0 && records != 0 && !map_fails && !unmap_fails {
                same_bytes(t, frame + 0x48 - 0x2c, 12, "PSpriteCollection_flush_0056ee50: min");
                same_bytes(t, frame, 12, "PSpriteCollection_flush_0056ee50: max");
            }
            t.b_top = None;
            t.reset();
        }
    });
}
