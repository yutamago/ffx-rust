//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the effects (ffx-vfx) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use common::this_call;
use ffx_oracle::{Oracle, harvest_test};

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

// effects (round 37)
harvest_test!(opu_bind_etc, 0x0081_4520, args = 2, |m: &mut Oracle, s| {
    ffx_vfx::opus::bridge::opu_bind_etc(m, s.arg(0), s.arg(1), &mut |m, f, a| {
        m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
    })
});
// effects (round 38)
harvest_test!(
    ppp_fp_loop_matrix,
    0x0072_8ac0,
    args = 3,
    |m: &mut Oracle, s| {
        // the local matrix in stack scratch
        let local = m.stack_range().start + 0x100;
        ffx_vfx::fp::bridge::ppp_fp_loop_matrix(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            local,
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
            &mut |m, f, a| m.call_f64(f, a).unwrap(),
        )
    }
);
// effects (round 39)
harvest_test!(
    update_phyre_view_projection_matrix,
    0x0071_1910,
    args = 0,
    |m: &mut Oracle, _s| {
        // the two local matrices in stack scratch
        let local = m.stack_range().start + 0x100;
        ffx_vfx::fp::bridge::update_phyre_view_projection_matrix(m, local, &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        });
    }
);
// effects (round 40)
harvest_test!(
    ppp_run_part_fp_objects,
    0x0071_7010,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_vfx::part::bridge::ppp_run_part_fp_objects(m, s.arg(0), &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        });
    }
);
harvest_test!(
    ppp_run_part_fp,
    0x0071_22f0,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_vfx::part::bridge::ppp_run_part_fp(m, s.arg(0), s.arg(1) & 0xff, &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
harvest_test!(
    ppp_draw_shape_x0,
    0x0074_0f60,
    args = 3,
    |m: &mut Oracle, s| {
        // the render request in stack scratch (its +0 and +0xc words are
        // the original's stale stack)
        let frame = (m.stack_range().start + 0x200) & !0xf;
        ffx_vfx::part::bridge::ppp_draw_shape_x0(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            frame,
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        );
    }
);
harvest_test!(
    op_random_in_slot,
    0x007e_6700,
    // no argument words: the caller's stack from the argument up is
    // compared, so the result the original leaves in its slot is too
    args = 0,
    |m: &mut Oracle, s| {
        ffx_vfx::rand::bridge::op_random_in_slot(m, s.arg_f32(0), s.regs.esp + 4)
    }
);

// effects: opu_world (round 46)
harvest_test!(opu_world, 0x0080_1d00, args = 2, |m: &mut Oracle, s| {
    ffx_vfx::opus::bridge::opu_world(m, s.arg(0), s.arg(1), &mut |m, f, a| {
        m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
    })
});

// effects: pppDrawShapeCameraDraw (round 46; no caller reads its eax)
harvest_test!(
    ppp_draw_shape_camera_draw,
    0x0074_7540,
    args = 3,
    |m: &mut Oracle, s| {
        ffx_vfx::draw::bridge::ppp_draw_shape_draw(
            m,
            &ffx_vfx::draw::bridge::CAMERA_DRAW,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        );
    }
);
// effects (round 47)
harvest_test!(
    ppp_ke_z_crct_draw,
    0x0073_5160,
    args = 3,
    |m: &mut Oracle, s| {
        // the local matrix (frame - 0x44) in stack scratch
        let frame = m.stack_range().start + 0x144;
        ffx_vfx::mat::bridge::ppp_ke_z_crct_draw(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            frame,
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        )
    }
);
// effects (round 49)
harvest_test!(ppp_fp_loop, 0x0072_9ab0, args = 0, |m: &mut Oracle, _| {
    // the colour scale (the original's ebp - 0x14) in stack scratch
    let colour = m.stack_range().start + 0x100;
    ffx_vfx::fp::bridge::ppp_fp_loop(m, colour, &mut |m, f, a| {
        m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
    });
});
// vfx-ppp (round 1): the request and matrices in stack scratch
harvest_test!(
    ppp_draw_mdl_camera_loop_draw,
    0x0073_fa50,
    args = 3,
    |m: &mut Oracle, s| {
        let frame = (m.stack_range().start + 0x400) & !0xf;
        ffx_vfx::mdl_cam::bridge::ppp_draw_mdl_camera_loop_draw(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            frame,
            &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        );
    }
);
// effects (round 52)
harvest_test!(opu_jump_etc, 0x0081_68c0, args = 2, |m: &mut Oracle, s| {
    ffx_vfx::opus::bridge::opu_jump_etc(
        m,
        s.arg(0),
        s.arg(1),
        &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        &mut |m, f, a| m.call_f64(f, a).unwrap(),
    )
});
harvest_test!(opu_v_en_bone, 0x0080_4c90, args = 2, |m: &mut Oracle, s| {
    ffx_vfx::opus::bridge::opu_v_en_bone(
        m,
        s.arg(0),
        s.arg(1),
        &mut |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        &mut |m, f, a| m.call_f64(f, a).unwrap(),
    )
});
