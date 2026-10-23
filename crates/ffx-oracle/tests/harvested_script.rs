//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the script (ffx-script) tests. Every
//! state the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs`.

use ffx_oracle::{Oracle, OracleHost, harvest_test};
use ffx_script::vm::bridge::helpers as hp;

// script round 23 (this machine): worker positions, the guide map angle,
// the effect poll
harvest_test!(
    atel_get_actor_start_pos,
    0x0086_a720,
    args = 4,
    |m: &mut Oracle, s| { hp::atel_get_actor_start_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    atel_get_actor_end_pos,
    0x0086_a390,
    args = 4,
    |m: &mut Oracle, s| { hp::atel_get_actor_end_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    script_get_worker_pos,
    0x0086_a2f0,
    args = 4,
    |m: &mut Oracle, s| { hp::script_get_worker_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    atel_get_guide_map_ang_ex,
    0x0086_b9a0,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_get_guide_map_ang_ex(m, s.arg(0)) }
);
harvest_test!(
    atel_effect_idle_poll,
    0x0086_33a0,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_effect_idle_poll(&mut OracleHost(m)) }
);

// script round 24 (this machine): the movie check, the guide map records
// (the start/end y go to the original's own argument slot), the vibration
// volume
harvest_test!(
    movie_is_busy,
    0x0076_ed10,
    args = 0,
    |m: &mut Oracle, _s| { hp::movie_is_busy(&mut OracleHost(m)) }
);
harvest_test!(
    atel_get_guide_map_actor_info,
    0x0086_b740,
    args = 2,
    |m: &mut Oracle, s| {
        hp::atel_get_guide_map_actor_info(m, s.arg(0), s.arg(1), s.regs.esp + 8)
    }
);
harvest_test!(
    atel_get_gide_map_actor_info,
    0x0086_b700,
    args = 2,
    |m: &mut Oracle, s| {
        hp::atel_get_guide_map_actor_info(m, s.arg(0), s.arg(1), s.regs.esp + 8)
    }
);
harvest_test!(
    vibrate_update_volume,
    0x0087_5b40,
    args = 0,
    |m: &mut Oracle, _s| { hp::vibrate_update_volume(&mut OracleHost(m)) }
);

/// A float call through the emulator: `ST0` of the original at `f`.
fn fcall(m: &mut Oracle, f: u32, args: &[u32]) -> f64 {
    let at = m.rebase(f);
    m.call_f64(at, args).unwrap()
}

/// [`fcall`] for a port that takes an [`OracleHost`].
fn fcall_host(h: &mut OracleHost, f: u32, args: &[u32]) -> f64 {
    fcall(h.0, f, args)
}

// script round 25 (this machine): ports the drivers reach (frames for the
// originals' locals well below the call's stack pointer)
harvest_test!(
    atel_exec_soft_ware_reset,
    0x0087_38d0,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_exec_soft_ware_reset(&mut OracleHost(m)) }
);
harvest_test!(
    atel_exec_ctrl_event_init,
    0x0086_77a0,
    args = 2,
    |m: &mut Oracle, s| { hp::atel_exec_ctrl_event_init(&mut OracleHost(m), s.arg(0), s.arg(1)) }
);
harvest_test!(
    atel_apply_chr_dir,
    0x0087_0360,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_apply_chr_dir(&mut OracleHost(m), s.arg(0)) }
);
harvest_test!(
    atel_sync_chr_rot,
    0x0086_9f90,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_sync_chr_rot(m, s.arg(0), fcall) }
);
harvest_test!(
    atel_step_sprites,
    0x0087_43e0,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_step_sprites(&mut OracleHost(m)) }
);
// (no line for AtelDrawSemiTrans 0x00872ca0: its callee Yn_GuideMapProcEx
// copies a stack word it never wrote into the emulated VU0 registers
// (0x00c8f8xx-0x00c8f97x), and that leftover differs between the original's
// frame and the port's call; round25c's test logs the callee instead)
harvest_test!(
    atel_set_snd_camera_pos,
    0x0087_7470,
    args = 0,
    |m: &mut Oracle, s| { hp::atel_set_snd_camera_pos(&mut OracleHost(m), s.regs.esp - 0x800) }
);
harvest_test!(
    atel_update_water_status,
    0x0087_1e50,
    args = 0,
    |m: &mut Oracle, s| { hp::atel_update_water_status(&mut OracleHost(m), s.regs.esp - 0x800) }
);
harvest_test!(
    atel_exec_internal_871d10,
    0x0087_1d70,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_exec_internal_871d10(&mut OracleHost(m)) }
);
harvest_test!(
    atel_sync_turn_end,
    0x0087_0dc0,
    args = 2,
    |m: &mut Oracle, s| { hp::atel_sync_turn_end(&mut OracleHost(m), s.arg(0), s.arg(1)) }
);
harvest_test!(
    atel_apply_polar_move,
    0x0086_29c0,
    args = 1,
    |m: &mut Oracle, s| {
        hp::atel_apply_polar_move(&mut OracleHost(m), s.arg(0), s.regs.esp - 0x800);
    }
);
harvest_test!(
    atel_set_event_water_flag,
    0x0086_5740,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_set_event_water_flag(&mut OracleHost(m), s.arg(0)) }
);
harvest_test!(
    atel_update_window_status,
    0x0087_1f30,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_update_window_status(&mut OracleHost(m)) }
);
harvest_test!(
    atel_update_battle_status,
    0x0087_1bc0,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_update_battle_status(&mut OracleHost(m), fcall_host) }
);
harvest_test!(
    atel_check_line_cross,
    0x0086_80a0,
    args = 4,
    |m: &mut Oracle, s| {
        let frame = s.regs.esp - 0x800;
        hp::atel_check_line_cross(
            &mut OracleHost(m),
            s.arg(0),
            s.arg(2),
            s.arg(3),
            frame,
            fcall_host,
        );
    }
);

// script round 26 (this machine): the ctrl work pass, the pause check, the
// move step (the CRT's sin and cos on the host, as `common::crt_atan`), the
// ctrl works' callbacks, the line crossings
harvest_test!(
    atel_run_ctrl_workers,
    0x0086_7950,
    args = 3,
    |m: &mut Oracle, s| {
        hp::atel_run_ctrl_workers(&mut OracleHost(m), s.arg(0), s.arg(1), s.arg(2))
    }
);
harvest_test!(
    atel_check_pause,
    0x0087_3620,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_check_pause(&mut OracleHost(m)) }
);
harvest_test!(
    atel_move_step,
    0x0087_0540,
    args = 2,
    |m: &mut Oracle, s| {
        hp::atel_move_step(&mut TrigHost(OracleHost(m)), s.arg(0), s.arg(1));
    }
);
harvest_test!(
    atel_exec_ctrl_works,
    0x0086_88f0,
    args = 2,
    |m: &mut Oracle, s| { hp::atel_exec_ctrl_works(&mut OracleHost(m), s.arg(0), s.arg(1)) }
);
harvest_test!(
    atel_check_line_crosses,
    0x0086_8020,
    args = 2,
    |m: &mut Oracle, s| { hp::atel_check_line_crosses(&mut OracleHost(m), s.arg(0), s.arg(1)) }
);

/// [`OracleHost`] with the script VM's float calls: `_CIsin`/`_CIcos` on
/// the host (the `f32`s the ports store agree on the harvested states),
/// float originals on the oracle.
struct TrigHost<'a>(OracleHost<'a>);

impl ffx_bridge::Mem for TrigHost<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.0.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl ffx_bridge::Host for TrigHost<'_> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.0.call(addr, args)
    }
}

impl ffx_script::vm::bridge::run::NativeHost for TrigHost<'_> {
    fn call_float(&mut self, addr: u32, args: &[u32]) -> f64 {
        self.0.0.call_f64(addr, args).unwrap()
    }
    fn scratch(&mut self) -> u32 {
        self.0.0.stack_range().start + 0x100
    }
    fn call_intrinsic1(&mut self, addr: u32, a: f64) -> f64 {
        use ffx_bridge::Mem;
        if addr == self.rebase(0x0094_98c8) {
            a.sin()
        } else {
            a.cos()
        }
    }
}

// script round 27: AtelExec (the ctrl works' pass) and the per-actor
// touch/talk check the field driver reaches
harvest_test!(atel_exec, 0x0086_66d0, args = 0, |m: &mut Oracle, _s| {
    hp::atel_exec(&mut OracleHost(m))
});
harvest_test!(
    atel_check_touch_talk,
    0x0086_6980,
    args = 3,
    |m: &mut Oracle, s| {
        // the port hands `fcall` the rebased address
        hp::atel_check_touch_talk(&mut OracleHost(m), s.arg(0), s.arg(2), |h, at, args| {
            h.0.call_f64(at, args).unwrap()
        })
    }
);

// script round 28: the event sprites' draw (the render record and its
// matrix in stack scratch, which the diff leaves out)
harvest_test!(
    atel_exec_draw_shape,
    0x0087_4420,
    args = 0,
    |m: &mut Oracle, _s| {
        let frame = m.stack_range().start + 0x100;
        // void: the original's EAX is the stack cookie check's leftover
        hp::atel_exec_draw_shape(&mut OracleHost(m), frame);
    }
);

// script round 28: the water status and the event sprites (one frame in
// stack scratch; void: the tail-called draw leaves the cookie check's EAX)
harvest_test!(
    atel_draw_no_trans,
    0x0087_2c90,
    args = 0,
    |m: &mut Oracle, _s| {
        let frame = m.stack_range().start + 0x100;
        hp::atel_draw_no_trans(&mut OracleHost(m), frame);
    }
);

// script round 28: the per-actor move update (turn outputs and spline point
// in stack scratch; the port hands `fcall` the rebased address)
harvest_test!(
    atel_move_update,
    0x0086_8990,
    args = 2,
    |m: &mut Oracle, s| {
        let frame = m.stack_range().start + 0x100;
        hp::atel_move_update(
            &mut OracleHost(m),
            s.arg(0),
            s.arg(1),
            frame,
            |h, at, args| h.0.call_f64(at, args).unwrap(),
        );
    }
);

// script round 28: the per-actor turn update (aim outputs in stack scratch)
harvest_test!(
    atel_turn_update,
    0x0086_9510,
    args = 2,
    |m: &mut Oracle, s| {
        let frame = m.stack_range().start + 0x100;
        hp::atel_turn_update(
            &mut OracleHost(m),
            s.arg(0),
            s.arg(1),
            frame,
            |h, at, args| h.0.call_f64(at, args).unwrap(),
        );
    }
);

// small round 1: the battle ctrl work's +0x4c callback
harvest_test!(
    atel_exec_ctrl_event,
    0x0086_7740,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_exec_ctrl_event(&mut OracleHost(m), s.arg(0)) }
);
