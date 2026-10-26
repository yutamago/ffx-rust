//! Continuation round 2, batch 14: owned-array destructors, a material's
//! buffer size, the renderer's initialisation and the application's input
//! devices (`ffx_prender::p6_app`) against their originals.

use ffx_oracle::Conv;
use ffx_prender::p6_app as ap;

use super::small::run_framed;
use super::{Out, Rng, Twins, object};

#[test]
fn app_array_dtors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0e01);
        t.stub(ap::FREE, false, 1, 0, 0x0f4e_e000);
        t.stub(ap::FREE_RECORDS, false, 2, 0, 0x0f4e_e001);
        for &(f, arrays) in ap::ARRAY_DTORS {
            for i in 0..12u32 {
                let this = object(&mut t, &mut rng, 0x10, false);
                for (k, &(off, _)) in arrays.iter().enumerate() {
                    let j = i + k as u32;
                    let view = if j.is_multiple_of(3) { 0x8000_0000 } else { 0 };
                    let high = if j % 5 == 2 { 0x4000_0000 } else { 0 };
                    t.w32(this + off, rng.below(4) | view | high);
                    if j % 4 == 1 {
                        t.w32(this + off + 4, 0);
                    }
                }
                t.reset_log();
                t.check(
                    &format!("{name} array dtor {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(ap::bridge::array_dtor(g, this, arrays)),
                );
            }
        }
    }
}

#[test]
fn app_material_size_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0e02);
        let sizer = t.stub_code(0x0005_1e00, false, 3, 0, 0x37);
        let other = t.stub_code(0x0005_1e01, false, 2, 0, 0x21);
        for i in 0..16u32 {
            let n = rng.below(5);
            let params = t.zeros(0x14 * n as usize + 4);
            for k in 0..n {
                let e = object(&mut t, &mut rng, 9, false);
                match rng.below(3) {
                    0 => {
                        t.w32(e + 0x1c, sizer);
                    }
                    1 => {
                        t.w32(e + 0x1c, 0);
                        t.w32(e + 0x20, other);
                    }
                    _ => {
                        t.w32(e + 0x1c, 0);
                        t.w32(e + 0x20, 0);
                    }
                }
                t.w32(params + 0x14 * k, e);
            }
            let material = object(&mut t, &mut rng, 8, false);
            t.w32(material + 4, n | (rng.next() & 0x8000_0000));
            t.w32(material + 8, params);
            t.w32(material + 0x18, rng.below(8) | (rng.next() & 0x8000_0000));
            let ctx = object(&mut t, &mut rng, 4, false);
            run_framed(
                &mut t,
                &format!("{name} material size #{i}"),
                0x005c_6680,
                Conv::Thiscall(material),
                &[ctx],
                0x14,
                0x10,
                |g, ebp| ap::bridge::material_size(g, material, ebp - 0x14, ctx),
            );
        }
    }
}

#[test]
fn app_renderer_initialize_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0e03);
        t.stub(ap::GUARD_CTOR, true, 0, 0, 0);
        t.stub(ap::GUARD_DTOR, true, 0, 0, 0);
        t.stub(ap::INTERFACE_CTOR, true, 0, 0, 0);
        t.stub(ap::ATEXIT, false, 1, 0, 0);
        t.stub(ap::VIEWPORT_SIZE, true, 3, 3, 0);
        t.stub(ap::INIT_RINGS, true, 5, 5, 0x0001_e000);
        for i in 0..8u32 {
            t.stub(
                ap::SEMAPHORE_INIT,
                true,
                2,
                2,
                if i % 4 == 3 { 0x16 } else { 0 },
            );
            let guard = t.rebase(ap::INTERFACE_GUARD);
            t.w32(guard, rng.next() & !1 | (i & 1));
            let this = object(&mut t, &mut rng, 0x40, false);
            let args = [rng.next(), rng.next(), rng.next()];
            run_framed(
                &mut t,
                &format!("{name} renderer initialize #{i}"),
                0x005b_8da0,
                Conv::Thiscall(this),
                &args,
                0x14,
                8,
                |g, ebp| {
                    ap::bridge::renderer_initialize(g, this, ebp - 0x14, args[0], args[1], args[2])
                },
            );
            t.unstub(ap::SEMAPHORE_INIT);
        }
    }
}

#[test]
fn app_setup_inputs_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0e04);
        t.stub(ap::APP_AFTER_DEVICE, true, 0, 0, 0);
        let head = t.rebase(ap::INPUT_LIST);
        for i in 0..10u32 {
            let dev = if i % 4 == 3 {
                0
            } else {
                let vt = t.vtable(4, &[(2, 2, 0)]);
                let d = object(&mut t, &mut rng, 0x44, false);
                t.w32(d, vt);
                d
            };
            t.stub(ap::CREATE_DEVICE, false, 2, 0, dev);
            // inputs: some answer 1 (slot 1), the list ends at the head, 0 or 4
            let n = rng.below(6);
            let end = [head, 0, 4][i as usize % 3];
            let mut next = end;
            for _ in 0..n {
                let ret = [0, 1, 1, 2][rng.below(4) as usize];
                let vt = t.vtable(4, &[(1, 0, ret)]);
                let input = object(&mut t, &mut rng, 2, false);
                t.w32(input, vt);
                t.w32(input + 4, next);
                next = input + 4;
            }
            t.w32(head, next);
            let app = object(&mut t, &mut rng, 0x2c, false);
            t.reset_log();
            t.check(
                &format!("{name} setup inputs #{i}"),
                0x0062_9870,
                Conv::Thiscall(app),
                &[],
                |g| Out::U32(ap::bridge::setup_inputs(g, app)),
            );
            t.unstub(ap::CREATE_DEVICE);
        }
    }
}

#[test]
fn app_morph_weights_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0e05);
        t.map_page0();
        for i in 0..24u32 {
            // the weights array: none, short or long
            let wn = 2 + rng.below(5);
            let wdata = t.zeros(4 * wn as usize + 4);
            // one word more than the count: read only past the end
            for k in 0..=wn {
                let f = super::small::float_bits(&mut rng);
                t.w32(wdata + 4 * k, f);
            }
            let weights = if i % 5 == 4 {
                0
            } else {
                let w = t.zeros(8);
                t.w32(w, wn | (rng.next() & 0x8000_0000));
                t.w32(w + 4, wdata);
                w
            };
            t.stub(ap::WEIGHTS_OF, true, 1, 1, weights);
            // the modifier: 3 segments, records
            let nrec = 1 + rng.below(4);
            let recs = t.zeros(20 * nrec as usize + 4);
            for k in 0..nrec {
                t.w32(recs + 20 * k + 0x10, wn - 1 + rng.below(3));
            }
            let mw = object(&mut t, &mut rng, 8, false);
            t.w32(mw + 4, 3 | (rng.next() & 0x8000_0000));
            t.w32(mw + 0x10, nrec | (rng.next() & 0x8000_0000));
            t.w32(mw + 0x14, recs);
            // a fourth segment past the count of 3
            let segs = t.zeros(20 * 4);
            for k in 0..4 {
                let n = if i % 4 == 2 {
                    8 + rng.below(4)
                } else {
                    rng.below(5)
                };
                let entries = t.zeros(8 * n as usize + 8);
                for j in 0..n {
                    let first = match rng.below(4) {
                        0 | 1 => 0x8000_0000 | rng.next(),
                        2 => 0,
                        _ => rng.below(0x100),
                    };
                    t.w32(entries + 8 * j, first);
                    t.w32(entries + 8 * j + 4, rng.below(5));
                }
                let record = object(&mut t, &mut rng, 4, i % 7 == 6 && k == 1);
                if record != 0 {
                    t.w32(record + 8, n | (rng.next() & 0x8000_0000));
                }
                t.w32(segs + 20 * k, record);
                t.w32(segs + 20 * k + 0x10, entries);
            }
            t.w32(mw + 8, segs);
            let list = t.zeros(4);
            t.w32(list, if i % 9 == 8 { 0 } else { mw });
            let ctx = t.zeros(0x10);
            t.w32(ctx + 8, list);
            t.w32(ctx + 0xc, rng.below(4));
            let out = if i % 6 == 5 { 0 } else { t.zeros(0x40) };
            // page 0 as a modifier (read through a null one by a wrong check)
            t.w32(4, 3);
            t.w32(8, 0x100);
            t.w32(0x100, 0x200);
            t.w32(0x208, 2);
            t.reset_log();
            t.check(
                &format!("{name} morph weights #{i}"),
                0x005c_b750,
                Conv::Cdecl,
                &[out, ctx],
                |g| Out::U32(ap::bridge::morph_weights(g, out, ctx)),
            );
            t.unstub(ap::WEIGHTS_OF);
        }
    }
}
