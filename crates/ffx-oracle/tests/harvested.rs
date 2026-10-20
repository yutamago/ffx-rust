//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): every state the drivers
//! reached on the base snapshots (`tools/harvest.sh`). Each test skips
//! without `FFX_GAME_DIR` or stored states.

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

use ffx_oracle::harvest_test;
use ffx_oracle::{CallState, Oracle, OracleHost};

use ffx_ps2::vu0::bridge as vu0;

fn int(s: &CallState, i: u32) -> i32 {
    s.arg(i).cast_signed()
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
