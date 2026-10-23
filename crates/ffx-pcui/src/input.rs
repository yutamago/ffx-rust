//! The PC port's per-frame input pass (`0x00630e80`, from
//! `FFXApplication_handleInputs`, `Pause_Update` and `Pause_Enter`): the
//! pad bindings, the pointer zones and the Iggy menus' input.

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    /// The word that turns the pass off while set (unnamed; read by the
    /// input code at `0x00630d50`, `0x00632d35` and others).
    pub const INPUT_OFF: Global<u32> = Global::new(0x0133_c930);
    /// `Virtuos_InputManager_getInstance()`,
    /// `Virtuos_InputManager_updateInput(this)`.
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    pub const INPUT_MANAGER_UPDATE: u32 = 0x0063_2a30;
    /// `TouchManager_GetInstance()`, `GuiTouch_update(this)`.
    pub const TOUCH_MANAGER: u32 = 0x0068_98c0;
    pub const GUI_TOUCH_UPDATE: u32 = 0x0068_a3a0;
    /// `GetGlobal01984c68_00a25a30()` and its object's update
    /// `0x00a25ab0(this)`.
    pub const IGGY_INPUT: u32 = 0x00a2_5a30;
    pub const IGGY_INPUT_UPDATE: u32 = 0x00a2_5ab0;
    /// `g_IggyMenuManager` and its update `0x00a26ce0(this)`.
    pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
    pub const IGGY_MENU_MANAGER_UPDATE: u32 = 0x00a2_6ce0;

    /// `Input_UpdateAll_00630e80()` (name ours): unless [`INPUT_OFF`] is
    /// set, the input manager's update, the pointer zones' update and the
    /// Iggy input's, each on the instance its getter returns, then a tail
    /// call to the menu manager's update (`g_IggyMenuManager` in `ecx`),
    /// whose `eax` it returns. With the word set the original returns its
    /// caller's `eax` (callers read none): 0 here.
    pub fn update_all(mem: &mut impl Host) -> u32 {
        if INPUT_OFF.get(mem) != 0 {
            return 0;
        }
        for (get, update) in [
            (INPUT_MANAGER, INPUT_MANAGER_UPDATE),
            (TOUCH_MANAGER, GUI_TOUCH_UPDATE),
            (IGGY_INPUT, IGGY_INPUT_UPDATE),
        ] {
            let this = mem.call_sym(get, Conv::Cdecl, &[]);
            mem.call_sym(update, Conv::Thiscall(this), &[]);
        }
        let menus = IGGY_MENU_MANAGER.get(mem);
        mem.call_sym(IGGY_MENU_MANAGER_UPDATE, Conv::Thiscall(menus), &[])
    }
}
