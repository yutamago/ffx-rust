//! Menus, windows and text drawing (the PS2 `Tk` and `Gu_` modules).
//!
//! The `menu` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function over
//! game memory (`docs/porting.md`), one module per family:
//!
//! - [`arms`]: the field models' weapon and armour refresh
//!   (`TkMn2ArmsRefreshArms`);
//! - [`gu`]: windows, controls, selectors, the text cursor, GS packets;
//! - [`main_loop`]: the main menu's frame (`0x008a9ca0`);
//! - [`math`]: float helpers (angle wrapping, radius tests);
//! - [`msg`]: message tables and message-window choices;
//! - [`scene`]: menu scene data lookups;
//! - [`scissor`]: the menu's software scissor;
//! - [`tk`]: menu state, pads, vibration, the save CRC, command lists.

pub mod arms;
pub mod gu;
pub mod main_loop;
pub mod math;
pub mod msg;
pub mod scene;
pub mod scissor;
pub mod tk;

/// `get_ptr32_func(p)` (`0x00a44650`): `p` (a 32-bit pointer as itself).
#[must_use]
pub const fn get_ptr32_func(p: u32) -> u32 {
    p
}

/// `is_ptr32_null(p)` (`0x00a446a0`).
#[must_use]
pub const fn is_ptr32_null(p: u32) -> bool {
    p == 0
}
