//! The Iggy menus' manager (`g_IggyMenuManager`) and the Esc menu
//! (`FFEscMenu`): the pieces of their C++ classes that are game logic.

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `0x00a28f40(menu, x, y, w, h)` (`thiscall`, unnamed): a menu's
    /// viewport, the four words through `ftol` into `+0x84..+0x90`.
    pub const MENU_SET_VIEWPORT: u32 = 0x00a2_8f40;

    /// `IggyMenuManager_ApplyViewport_00a265a0(this)` (name ours;
    /// `thiscall`, from the viewport setters `0x00a26a20` and
    /// `0x00a26bd0`): every menu of the list (`+0xc`, `+8 & 0x7fffffff`
    /// entries, both re-read for each) that is not null gets the viewport
    /// `(x +0x18, screen h +0x2c - y +0x1c - h +0x24, w +0x20, h +0x24)`,
    /// bottom-up, its fields re-read for each.
    #[allow(clippy::many_single_char_names)] // the rectangle
    pub fn apply_viewport(mem: &mut impl Host, this: u32) {
        let mut i = 0u32;
        while i < mem.read::<u32>(this.wrapping_add(8)) & 0x7fff_ffff {
            let list: u32 = mem.read(this.wrapping_add(0xc));
            let menu: u32 = mem.read(list.wrapping_add(4 * i));
            if menu != 0 {
                let [x, y, w, h, screen_h] = [0x18, 0x1c, 0x20, 0x24, 0x2c]
                    .map(|off| mem.read::<u32>(this.wrapping_add(off)));
                let bottom = screen_h.wrapping_sub(y).wrapping_sub(h);
                mem.call_sym(MENU_SET_VIEWPORT, Conv::Thiscall(menu), &[x, bottom, w, h]);
            }
            i += 1;
        }
    }

    /// `0x007018a0(this, a, b)` and `0x00a28c10(this, a, b)` (`thiscall`,
    /// unnamed): the Esc menu's check and the base menu's handler.
    pub const ESC_MENU_CHECK: u32 = 0x0070_18a0;
    pub const MENU_HANDLE: u32 = 0x00a2_8c10;

    /// `FFEscMenu_vfunc_0068e530(this, a, b)` (name ours; a slot of the
    /// vtable at `0x00b46c24`, `thiscall`, `ret 8`): the base menu's
    /// handler `0x00a28c10(this, a, b)` only when `0x007018a0(this, a, b)`
    /// returns `al` 1; `eax` the last call's.
    pub fn esc_menu_handle(mem: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let r = mem.call_sym(ESC_MENU_CHECK, Conv::Thiscall(this), &[a, b]);
        if r.to_le_bytes()[0] != 1 {
            return r;
        }
        mem.call_sym(MENU_HANDLE, Conv::Thiscall(this), &[a, b])
    }
}
