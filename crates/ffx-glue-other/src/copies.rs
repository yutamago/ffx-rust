//! Exact copies of functions other modules ported (`tools/dupes.py`):
//! the same code at another address, so the same bridge, each checked
//! against its own original in `tests/h4_glue_other.rs`.
//!
//! - `PhyreCursor_IsValid_copy_00637f80`, `PhyreCursor_IsValid_006f63b0`:
//!   `ffx_graphic::managers::bridge::cursor_valid`;
//! - `TaggedTreeInsert_0069dae0`: `ffx_system::stl::bridge::tagged_tree_insert`;
//! - `PArray_PushInto_006a2670`, `_006a2700`:
//!   `ffx_system::container::bridge::parray_push_into` with
//!   `Phyre_PhyreFree` called at its address ([`phyre_free`]);
//! - `vfxIntTree_Insert_0070bb20`, `_0070bba0`, `_00710540`:
//!   `ffx_vfx::tree::bridge::vfx_int_tree_insert`;
//! - `SfxEventTree_Find_0070ba20`, `_007104c0`:
//!   `ffx_sound::ffxfmod::bridge::sfx_find` (the key read from its
//!   pointer).

use ffx_graphic::host::{Conv, Host};

use crate::menu::PHYRE_FREE;

/// The `PArray_PushInto` copies' free: `Phyre_PhyreFree(p)` at its address
/// (ported; the hook takes the call).
pub fn phyre_free<H: Host>(h: &mut H, p: u32) {
    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
}
