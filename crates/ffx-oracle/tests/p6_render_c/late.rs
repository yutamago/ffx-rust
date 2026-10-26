//! Continuation round 3: the post-processing module's `Bind`, the
//! window's focus change and other late functions (`ffx_prender::p6_late`)
//! against their originals.

use ffx_oracle::Conv;
use ffx_prender::p6_late as lt;

use super::small::run_framed;
use super::{Out, Rng, Twins, object};

#[test]
fn post_bind_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1001);
        for &step in lt::POST_BIND {
            if let lt::BindStep::Call(f) = step {
                t.stub(f, false, 0, 0, rng.next());
            }
        }
        t.stub(lt::REGISTER_TYPE, false, 1, 0, rng.next());
        t.stub(lt::ADD_TO_NAMESPACE, true, 0, 0, rng.next());
        let vt = t.vtable(4, &[(1, 0, 0x55)]);
        let utility = t.rebase(lt::UTILITY);
        t.w32(utility, vt);
        let ready = t.rebase(lt::UTILITY_READY);
        for i in 0..4u8 {
            t.write(ready, &[[0, 1, 0, 0x80][i as usize]]);
            t.reset_log();
            t.check(
                &format!("{name} post bind #{i}"),
                0x00a1_8020,
                Conv::Cdecl,
                &[],
                |g| Out::U32(lt::bridge::post_bind(g)),
            );
            let n = t.log_entries();
            assert_eq!(n, 47 + u32::from(i % 2 == 0), "{name} #{i}: the calls");
        }
    }
}

#[test]
fn set_focus_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1002);
        t.stub(lt::PRINTF, false, 3, 0, rng.next());
        for (slot, n) in [
            (lt::CHANGE_DISPLAY_SETTINGS, 2),
            (lt::GET_WINDOW_LONG, 2),
            (lt::SET_WINDOW_LONG, 3),
            (lt::GET_CLIENT_RECT, 2),
        ] {
            t.import_stub(slot, n, rng.next());
        }
        // AdjustWindowRect grows the rectangle by a frame and a title bar
        let adjust = t.code(&[
            0x8b, 0x44, 0x24, 0x04, // mov eax, [esp+4]
            0x83, 0x28, 0x08, // sub dword [eax], 8
            0x83, 0x68, 0x04, 0x1f, // sub dword [eax+4], 31
            0x83, 0x40, 0x08, 0x08, // add dword [eax+8], 8
            0x83, 0x40, 0x0c, 0x08, // add dword [eax+0xc], 8
            0xb8, 0x01, 0x00, 0x00, 0x00, // mov eax, 1
            0xc2, 0x0c, 0x00, // ret 0xc
        ]);
        let at = t.rebase(lt::ADJUST_WINDOW_RECT);
        t.w32(at, adjust);
        // SetWindowPos takes 7 words; the stub logs the first six (the
        // seventh is the constant flags)
        let pos = t.stub_code(lt::SET_WINDOW_POS, false, 6, 7, 0x0bad_f00d);
        let at = t.rebase(lt::SET_WINDOW_POS);
        t.w32(at, pos);
        let hwnd = t.rebase(lt::HWND);
        let settings_at = t.rebase(lt::SETTINGS);
        for i in 0..24u32 {
            let settings = object(&mut t, &mut rng, 4, false);
            t.w32(settings + 8, if i % 4 == 3 { rng.below(3) } else { 1 });
            t.w32(hwnd, rng.next());
            t.w32(settings_at, settings);
            let focus = if i % 2 == 0 {
                rng.next() & !0xff
            } else {
                rng.next() | 1
            };
            run_framed(
                &mut t,
                &format!("{name} setFocus #{i}"),
                0x0062_a460,
                Conv::Thiscall(rng.next()),
                &[focus],
                0xb0,
                lt::FOCUS_FRAME as usize,
                |g, ebp| lt::bridge::set_focus(g, ebp - 0xb0, focus),
            );
        }
    }
}

#[test]
fn glow_gpu_dtor_matches_the_original() {
    use ffx_prender::p6_post as po;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_1003);
        t.stub(po::TARGET_RELEASE, false, 1, 0, 0x88);
        t.stub(po::SAMPLER_RELEASE, true, 0, 0, 0x89);
        t.stub(po::EFFECT_RELEASE, true, 0, 0, 0x8a);
        t.stub(po::MEMBER_DTOR, true, 0, 0, 0x8b);
        for i in 0..24u32 {
            // the bound tests: bound or not
            for k in 0..3 {
                let r = if (i + k) % 3 == 0 {
                    0x1200
                } else {
                    0x0101 * (k + 1)
                };
                t.stub(0x0059_5f90 + 0x10 * k, true, 0, 0, r);
            }
            for (k, f) in [0x0059_cc40, 0x0059_cf00, 0x0059_d0f0]
                .into_iter()
                .enumerate()
            {
                t.stub(f, true, 0, 0, 0x70 + k as u32);
            }
            let this = object(&mut t, &mut rng, 0x30, false);
            for (k, s) in [0x14u32, 0x38].into_iter().enumerate() {
                let fl = rng.next() & !0x80 | if (i >> k) & 1 == 1 { 0x80 } else { 0 };
                t.w32(this + s + 0x1c, fl);
            }
            let eff = object(&mut t, &mut rng, 1, i % 3 == 2);
            t.w32(this + 0x5c, eff);
            for off in lt::GLOW_GPU_INLINE {
                let (no_tex, no_target) = (rng.chance(4), rng.chance(5));
                let tex = object(&mut t, &mut rng, 2, no_tex);
                let target = object(&mut t, &mut rng, 2, no_target);
                if target != 0 {
                    t.w32(target, tex);
                    t.write(target + 4, &[rng.below(4) as u8]);
                }
                t.w32(this + off, target);
            }
            t.reset_log();
            t.check(
                &format!("{name} glow GPU dtor #{i}"),
                0x009f_e0d0,
                Conv::Thiscall(this),
                &[],
                |g| Out::U32(lt::bridge::glow_gpu_dtor(g, this)),
            );
            for k in 0..3 {
                t.unstub(0x0059_5f90 + 0x10 * k);
            }
            for f in [0x0059_cc40, 0x0059_cf00, 0x0059_d0f0] {
                t.unstub(f);
            }
        }
    }
}
