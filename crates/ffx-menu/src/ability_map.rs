//! The sphere grid's menu slot (`g_TkMenuMainOptionOrder` `+0x20`/`+0x24`):
//! `TkMenuCtrlAbilityMap2` starts the grid (`eiAbmStart`) once the menu's
//! first read is done and then runs it (`eiAbmCalc`) each frame;
//! `TkMenuDrawAbilityMap2_008e2720` darkens the screen and draws the grid's
//! frame.

/// The callees, by address.
pub mod callee {
    pub const GET_CONTROL_PAD: u32 = 0x008b_e410;
    pub const GET_CONTROL_PAD_REP: u32 = 0x008b_e470;
    pub const GET_CONTROL_PAD_TRG: u32 = 0x008b_e4b0;
    pub const GET_FIRST_READ_END_FLAG: u32 = 0x008a_8e50;
    pub const ABM_START: u32 = 0x00a5_4b40;
    pub const ABM_CALC: u32 = 0x00a5_3570;
    pub const OPEN_PKT_BUFF_TMP: u32 = 0x008b_ef30;
    pub const DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const DRAW_BOX_XYWH_C2: u32 = 0x008f_3db0;
    pub const DRAW_KICK_TMP: u32 = 0x008b_ea20;
    pub const VU1_SYNC_PATH: u32 = 0x0088_ec60;
    pub const DRAW_KICK: u32 = 0x008d_ce50;
    pub const ABM_DRAW_FRAME: u32 = 0x00a5_4560;
    pub const DRAW_ENV: u32 = 0x008d_cca0;
}

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::callee as c;

    /// `TkMenuCtrlAbilityMap2(w)` (`0x008e26d0`): the menu pads read; state
    /// `w+0x1c` 0 waits for `TOMenuGetFirstReadEndFlag` (returns 0 while
    /// it is 0), then `eiAbmStart` and state 1; state 1 tail-calls
    /// `eiAbmCalc` (its `eax`). Any other state returns `state - 1`.
    pub fn tk_menu_ctrl_ability_map2<M, C>(mem: &mut M, w: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut f = |mem: &mut M, addr: u32, args: &[u32]| {
            let a = mem.rebase(addr);
            call(mem, a, args)
        };
        f(mem, c::GET_CONTROL_PAD, &[]);
        f(mem, c::GET_CONTROL_PAD_REP, &[]);
        f(mem, c::GET_CONTROL_PAD_TRG, &[]);
        let state_at = w.wrapping_add(0x1c);
        // read once: the loop runs on the register
        let mut state = mem.read::<u32>(state_at);
        if state == 0 {
            if f(mem, c::GET_FIRST_READ_END_FLAG, &[]) == 0 {
                return 0;
            }
            f(mem, c::ABM_START, &[]);
            mem.write(state_at, 1u32);
            state = 1;
        }
        if state == 1 {
            return f(mem, c::ABM_CALC, &[]);
        }
        state.wrapping_sub(1)
    }

    /// `TkMenuDrawAbilityMap2_008e2720(w)` (`0x008e2720`, name ours): a
    /// half-dark box over the whole menu screen (`0, 0, 512, 416`,
    /// colour `0x80000000`), the kicks, `eiAbmDrawFrame` when `w+0x1c` is
    /// set, the menu draw environment; returns the last `TkMenuDrawKick`'s
    /// `eax` (a tail jump).
    pub fn tk_menu_draw_ability_map2<M, C>(mem: &mut M, w: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut f = |mem: &mut M, addr: u32, args: &[u32]| {
            let a = mem.rebase(addr);
            call(mem, a, args)
        };
        f(mem, c::OPEN_PKT_BUFF_TMP, &[]);
        f(mem, c::DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
        f(
            mem,
            c::DRAW_BOX_XYWH_C2,
            &[0, 0, 0x200, 0x1a0, 0x8000_0000, 0x8000_0000],
        );
        f(mem, c::DRAW_KICK_TMP, &[]);
        f(mem, c::VU1_SYNC_PATH, &[]);
        f(mem, c::DRAW_KICK, &[]);
        f(mem, c::DRAW_KICK, &[]);
        if mem.read::<u32>(w.wrapping_add(0x1c)) != 0 {
            f(mem, c::ABM_DRAW_FRAME, &[]);
        }
        f(mem, c::DRAW_KICK, &[]);
        f(mem, c::DRAW_ENV, &[]);
        f(mem, c::DRAW_KICK, &[])
    }
}
