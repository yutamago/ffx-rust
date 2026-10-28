//! M5 `system-b`: small pieces of the PC glue that belong to no larger
//! area of this list.

/// `FFEscMenu_vftable` and the vtable of its second base at `+0x9c`.
pub const ESC_MENU_VFTABLE: u32 = 0x00b4_6c20;
pub const ESC_MENU_LISTENER_VFTABLE: u32 = 0x00b4_6c2c;
/// `0x0062bce0(listener)` (cdecl): erases the listener from the set at
/// `0x00cc9ee4`.
pub const REMOVE_LISTENER: u32 = 0x0062_bce0;
/// `IggyMenu_dtor_IggyMenu` (thiscall).
pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;

/// The originals over game memory.
pub mod bridge {
    use super::{ESC_MENU_LISTENER_VFTABLE, ESC_MENU_VFTABLE, IGGY_MENU_DTOR, REMOVE_LISTENER};
    use ffx_graphic::host::{Conv, Host};

    /// `FFEscMenu::~FFEscMenu(this)` (`0x0068e1f0`, thiscall; without its
    /// SEH frame): the two vtables, the listener at `+0x9c` removed, then
    /// `IggyMenu::~IggyMenu(this)`, whose `eax` it returns.
    pub fn esc_menu_destruct(h: &mut impl Host, this: u32) -> u32 {
        let listener = this.wrapping_add(0x9c);
        h.write(this, h.rebase(ESC_MENU_VFTABLE));
        h.write(listener, h.rebase(ESC_MENU_LISTENER_VFTABLE));
        h.call_sym(REMOVE_LISTENER, Conv::Cdecl, &[listener]);
        h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(this), &[])
    }
}
