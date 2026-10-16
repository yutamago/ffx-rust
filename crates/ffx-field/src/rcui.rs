//! `rcui`: the field UI helpers (mouse drag state, number input, vector and
//! matrix copies).

use ffx_bridge::Global;

/// The UI work area (`rcuiMouseMove*`).
pub const WORK: Global<u32> = Global::new(0x0193_c0f0);

/// Offsets in the work area.
pub mod work {
    /// `+0xbe8`: the ID being dragged.
    pub const MOVE_ID: u32 = 0xbe8;
    /// `+0xbec`: the ID `rcuiMouseMoveGetID` returns.
    pub const GET_ID: u32 = 0xbec;
    /// `+0xbf0`: non-zero while a drag runs (cleared by `rcuiMouseMoveEnd`).
    pub const MOVING: u32 = 0xbf0;
}

/// `+0x28` of a number input: set once a number was entered
/// (`rcuiNumberIsOK` reads and clears it).
pub const NUMBER_OK: u32 = 0x28;

/// The UI state where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{NUMBER_OK, WORK, work};
    use crate::x87;

    /// `rcuiMouseMoveEnd()` (`0x0093c050`); `rcuiMouseMoveReset`
    /// (`0x0093c110`) is the same code.
    pub fn rcui_mouse_move_end(mem: &mut impl Mem) {
        let w = WORK.get(mem);
        mem.write(w + work::MOVING, 0u32);
    }

    /// `rcuiMouseMoveGetID()` (`0x0093c100`).
    pub fn rcui_mouse_move_get_id(mem: &impl Mem) -> u32 {
        mem.read(WORK.get(mem) + work::GET_ID)
    }

    /// `rcuiMouseMoveIsID(id)` (`0x0093c120`): whether `id` is being
    /// dragged.
    pub fn rcui_mouse_move_is_id(mem: &impl Mem, id: u32) -> bool {
        let w = WORK.get(mem);
        mem.read::<u32>(w + work::MOVE_ID) == id && mem.read::<u32>(w + work::MOVING) != 0
    }

    /// `rcuiNumberIsOK(input)` (`0x0093c3d0`): the flag, cleared.
    pub fn rcui_number_is_ok(mem: &mut impl Mem, input: u32) -> u32 {
        let v = mem.read::<u32>(input + NUMBER_OK);
        mem.write(input + NUMBER_OK, 0u32);
        v
    }

    fn copy_floats(mem: &mut impl Mem, dst: u32, src: u32, n: u32) {
        // one at a time, in order, as the original (they may overlap)
        for i in 0..n {
            let v = x87::copy(mem.read(src + 4 * i));
            mem.write(dst + 4 * i, v);
        }
    }

    /// `rcuiCopyVector(dst, src)` (`0x0093ce50`): four floats.
    pub fn rcui_copy_vector(mem: &mut impl Mem, dst: u32, src: u32) {
        copy_floats(mem, dst, src, 4);
    }

    /// `rcuiCopyMatrix(dst, src)` (`0x0093ce10`): sixteen floats.
    pub fn rcui_copy_matrix(mem: &mut impl Mem, dst: u32, src: u32) {
        copy_floats(mem, dst, src, 16);
    }
}
