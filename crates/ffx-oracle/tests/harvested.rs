//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): every state the drivers
//! reached on the base snapshots (`tools/harvest.sh`). Each test skips
//! without `FFX_GAME_DIR` or stored states.

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

use ffx_oracle::harvest_test;
use ffx_oracle::{CallState, Conv, Oracle, OracleHost};

use ffx_ps2::vu0::bridge as vu0;

fn int(s: &CallState, i: u32) -> i32 {
    s.arg(i).cast_signed()
}

/// `_CIatan` for the port's side: the host's `atan` stored as `f32` (the
/// original runs the CRT on the oracle's emulated x87; the `f32`s agree on
/// the harvested states).
fn crt_atan(_: &mut Oracle, x: f64) -> u32 {
    ffx_bridge::x87::store(x.atan())
}

/// `_CIsin` and `_CIcos` for the port's side: the host's, stored as `f32`
/// (`f32` bits in and out, as the battle vector code calls them).
fn crt_vector_trig(t: ffx_battle::pos::bridge::Trig, x: u32) -> u32 {
    use ffx_battle::pos::bridge::Trig;
    let x = f64::from(f32::from_bits(x));
    ffx_bridge::x87::store(match t {
        Trig::Sin => x.sin(),
        Trig::Cos => x.cos(),
        Trig::Atan => x.atan(),
    })
}

// ps2_emu: VU0 helpers (float math on vectors and matrices)
harvest_test!(
    ffx_vu0_apply_matrix,
    0x006e_d2e0,
    args = 3,
    |m: &mut Oracle, s| { vu0::apply_matrix(m, s.arg(0), s.arg(1), s.arg(2)) }
);
harvest_test!(
    ffx_vu0_rot_trans_pers,
    0x006e_d840,
    args = 4,
    |m: &mut Oracle, s| { vu0::rot_trans_pers(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);

// main (ffx-sg): the sequence step of a character, the Z sort
harvest_test!(sg_seq_step, 0x0083_7900, args = 1, |m: &mut Oracle, s| {
    ffx_sg::seq::bridge::seq_step(&mut OracleHost(m), s.arg(0))
});
harvest_test!(
    sg_chr_sort_by_z,
    0x0083_eb30,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_sg::state::bridge::chr_sort_by_z(m, int(s, 0));
    }
);
harvest_test!(
    sg_camera_put_rects,
    0x0083_f130,
    args = 0,
    |m: &mut Oracle, _s| { ffx_sg::camera::bridge::sg_camera_put_rects(&mut OracleHost(m)) }
);
harvest_test!(cam_apply, 0x007b_c980, args = 1, |m: &mut Oracle, s| {
    let work = m.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    ffx_sg::camera::bridge::cam_apply(m, s.arg(0), work, call, crt_atan);
});

// chara
harvest_test!(
    ch_set_sequence,
    0x0083_7ac0,
    args = 2,
    |m: &mut Oracle, s| { ffx_chara::motion::bridge::ch_set_sequence(m, s.arg(0), s.arg(1)) }
);

// battle
harvest_test!(
    ms_get_screen_data,
    0x007b_bd00,
    args = 4,
    |m: &mut Oracle, s| {
        ffx_battle::flow::bridge::ms_get_screen_data(m, [s.arg(0), s.arg(1), s.arg(2), s.arg(3)])
    }
);

// battle round 29: small leaves the drivers reach
harvest_test!(
    ms_battle_draw,
    0x0078_0d00,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_battle::minor::bridge::ms_battle_draw(m, &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
harvest_test!(
    ms_battle_encount_flag,
    0x0078_0fe0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_battle::misc::bridge::ms_battle_encount_flag(m, s.arg(0));
    }
);
harvest_test!(
    ms_get_chr_head_pos,
    0x0079_50b0,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_get_chr_head_pos(m, s.arg(0), s.arg(1));
    }
);
harvest_test!(
    ms_add_next_ctb,
    0x0079_a020,
    args = 5,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_add_next_ctb(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            s.arg(3),
            s.arg(4),
        );
    }
);
harvest_test!(
    ms_get_ctb_list,
    0x0079_a200,
    args = 1,
    |m: &mut Oracle, s| {
        // the CTBs and counters in stack scratch
        let frame = m.stack_range().start + 0x100;
        ffx_battle::turn::bridge::ms_get_ctb_list(m, s.arg(0), frame)
    }
);
harvest_test!(
    ms_wait_motion_pick,
    0x007a_8c50,
    args = 3,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_wait_motion_pick(m, s.arg(0), s.arg(1), s.arg(2))
    }
);

// map (ffx-field)
harvest_test!(
    yi_render_setup_fog_param,
    0x0090_d1b0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_field::yi::bridge::yi_render_setup_fog_param(m, s.arg(0));
    }
);

// menu (pad history)
harvest_test!(
    tk_get_controler_analog_hist,
    0x0088_8c20,
    args = 4,
    |m: &mut Oracle, s| {
        ffx_menu::tk::bridge::tk_get_controler_analog_hist(
            m,
            int(s, 0),
            int(s, 1),
            s.arg(2),
            int(s, 3),
        )
    }
);

// render (ffx-graphic): a UI element's texture remap
harvest_test!(
    graphic_ui_element_remap,
    0x0064_4320,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_graphic::ui::bridge::element_remap(m, s.arg(0), s.arg(1));
    }
);

// kernel: a record of a loaded kernel table
harvest_test!(
    ms_get_excel_data,
    0x007a_b870,
    args = 3,
    |m: &mut Oracle, s| { ffx_kernel::bridge::ms_get_excel_data(m, int(s, 0), s.arg(1), s.arg(2)) }
);

// effects: the particle frame loop's view test
harvest_test!(
    ppp_check_view_cylinder,
    0x0072_8f10,
    args = 5,
    |m: &mut Oracle, s| {
        let f = |i| f32::from_bits(s.arg(i));
        ffx_vfx::small::bridge::ppp_check_view_cylinder(m, s.arg(0), s.arg(1), f(2), f(3), f(4))
    }
);

// effects: pppDrawShapeCameraDraw's particle step
harvest_test!(
    ppp_shape_camera_step,
    0x0074_72b0,
    args = 2,
    |m: &mut Oracle, s| {
        let kind = ffx_vfx::draw::bridge::ShapeStep::Camera;
        ffx_vfx::draw::bridge::ppp_shape_step(m, kind, s.arg(0), s.arg(1));
    }
);

// effects: pppDrawShapeCameraDraw's emitter (its local matrix in the
// stack's scratch, which the diff leaves out)
harvest_test!(
    ppp_shape_camera_emit,
    0x0074_7090,
    args = 3,
    |m: &mut Oracle, s| {
        let frame = m.stack_range().start + 0x100;
        ffx_vfx::draw::bridge::ppp_shape_camera_emit(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            frame,
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        );
    }
);

// system: the per-frame magic file check (Sg_MainLoop)
harvest_test!(
    magic_file_update,
    0x009d_ac60,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_system::menu::bridge::magic_file_update(m, |m, f, a| {
            let f = m.rebase(f);
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);

// effects (round 33): the thiscall callees run as the originals
fn this_call(m: &mut Oracle, f: u32, this: u32, a: &[u32]) -> u32 {
    m.call(f, ffx_oracle::Conv::Thiscall(this), a).unwrap().eax
}

harvest_test!(
    class_dynamic_geometry_setup_dyn_mesh,
    0x006a_3b30,
    args = 0,
    |m: &mut Oracle, s| { ffx_vfx::small::bridge::setup_dyn_mesh(m, s.ecx(), &mut this_call) }
);
harvest_test!(
    vfx_geometry_register,
    0x006a_52f0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_vfx::small::bridge::vfx_geometry_register(m, s.ecx(), s.arg(0), &mut this_call)
    }
);
harvest_test!(
    dyn_geo_reset_current_buffer,
    0x0069_fcb0,
    args = 0,
    |m: &mut Oracle, s| {
        ffx_vfx::small::bridge::dyn_geo_reset_current_buffer(m, s.ecx(), &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
// the random draw's three extra words are the caller's edi, esi, ebx
harvest_test!(
    opp_anmm_frame_random,
    0x0081_0dd0,
    args = 4,
    |m: &mut Oracle, s| {
        let junk = [s.regs.edi, s.regs.esi, s.regs.ebx];
        ffx_vfx::anmm::bridge::frame_random(m, s.arg(2), s.arg(3), junk)
    }
);
// effects (round 34): op_irandom_p's three extra words are the caller's
// edi, esi and ebp
harvest_test!(opu_wait_cond, 0x0080_c520, args = 2, |m: &mut Oracle, s| {
    let junk = [s.regs.edi, s.regs.esi, s.regs.ebp];
    ffx_vfx::opus::bridge::opu_wait_cond(m, s.arg(0), s.arg(1), junk)
});

/// The store's format round-trips, and a state of the first target applies
/// to a fresh oracle (layout check).
#[test]
fn harvested_states_load() {
    let Some(cases) = ffx_oracle::harvested(0x006e_d2e0) else {
        return;
    };
    let c = &cases[0];
    let again = CallState::from_bytes(&c.state.to_bytes(), c.state.key).unwrap();
    assert_eq!(again, c.state);
    let (a, _) = c.oracles().unwrap();
    assert_eq!(a.layout().image_base, c.state.layout.image_base);
}

// battle round 30
harvest_test!(
    ms_walk_ability_process,
    0x0078_2d60,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_walk_ability_process(m, s.arg(0));
    }
);
// main round 16: the free camera, the effect read poll, the id map move,
// the motion update
harvest_test!(
    sg_camera_set_player,
    0x0083_f2e0,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_sg::state::bridge::camera_set_player(&mut OracleHost(m), &crt_vector_trig)
    }
);
harvest_test!(
    sg_effect_read_poll,
    0x0082_2250,
    args = 0,
    |m: &mut Oracle, _s| { ffx_sg::state::bridge::effect_read_poll(&mut OracleHost(m)) }
);
harvest_test!(
    sg_id_chr_move,
    0x0083_e5f0,
    args = 1,
    |m: &mut Oracle, s| {
        let point = m.stack_range().start + 0x1000;
        let fcall = &mut |h: &mut OracleHost<'_>, f: u32, a: &[u32]| {
            // the next call starts with an empty x87 stack (`Oracle::call`)
            h.0.call_f64(f, a).unwrap()
        };
        ffx_sg::state::bridge::id_chr_move(&mut OracleHost(m), s.arg(0), point, fcall);
    }
);
harvest_test!(
    sg_update_motion,
    0x0083_8c90,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_sg::seq::bridge::update_motion(&mut OracleHost(m), s.arg(0), s.arg(1));
    }
);
// main round 17: the main loop's frame sync
harvest_test!(sg_frame_sync, 0x0082_1e80, args = 1, |m: &mut Oracle, s| {
    let at = m.rebase(ffx_sg::state::bridge::TIME_US);
    let time = &mut |h: &mut OracleHost<'_>| {
        let r = h.0.call(at, Conv::Cdecl, &[]).unwrap();
        u64::from(r.edx) << 32 | u64::from(r.eax)
    };
    ffx_sg::state::bridge::frame_sync(&mut OracleHost(m), s.arg(0), time)
});
// effects (round 37)
harvest_test!(opu_bind_etc, 0x0081_4520, args = 2, |m: &mut Oracle, s| {
    ffx_vfx::opus::bridge::opu_bind_etc(m, s.arg(0), s.arg(1), &mut |m, f, a| {
        m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
    })
});
