//! Menus, windows and text drawing (the PS2 `Tk` and `Gu_` modules).
//!
//! The `menu` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function over
//! game memory (`docs/porting.md`), one module per family:
//!
//! - [`ability_map`]: the sphere grid's menu slot (ctrl and draw);
//! - [`arms`]: the field models' weapon and armour refresh
//!   (`TkMn2ArmsRefreshArms`);
//! - [`gu`]: windows, controls, selectors, the text cursor, GS packets;
//! - [`item`]: the item menu's control (`TkMenuCtrlItem`);
//! - [`m5m_a`]..: M5 stage 1 (the `Gu_` debug GUI's frame and the rest the
//!   static call graph reaches);
//! - [`main_loop`]: the main menu's frame (`0x008a9ca0`);
//! - [`math`]: float helpers (angle wrapping, radius tests);
//! - [`msg`]: message tables and message-window choices;
//! - [`save_load`]: the save/load menu's control (`TkMenuCtrlSaveLoad`);
//! - [`scene`]: menu scene data lookups;
//! - [`scissor`]: the menu's software scissor;
//! - [`start`]: the Tk boot (`TkInit`) and the menu's entry (`TkMenuStart`);
//! - [`tk`]: menu state, pads, vibration, the save CRC, command lists.

pub mod ability_map;
pub mod arms;
pub mod gu;
pub mod item;
pub mod l8_mbs;
pub mod m5m_a;
pub mod m5m_b;
pub mod m5m_c;
pub mod m5m_d;
pub mod m5m_e;
pub mod m5m_f;
pub mod m5m_g;
pub mod m5m_h;
pub mod m5m_i;
pub mod m5m_j;
pub mod m5m_k;
pub mod m5m_l;
pub mod m5m_m;
pub mod m5m_n;
pub mod main_loop;
pub mod math;
pub mod msg;
pub mod save_load;
pub mod scene;
pub mod scissor;
pub mod start;
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
