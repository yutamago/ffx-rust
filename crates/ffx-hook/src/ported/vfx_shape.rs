//! `ffx-vfx`'s `op_shape_draw` (`shape_draw`): the shape draw the magic
//! overlays reach through `g_MagicImports` slot 242 (`0x00c650b0`). cdecl.
//!
//! Threads: only the effect runners hand out the overlay tables
//! (`MsEffectProcess`, `MsEventEffectStart` and their kin), and none of the
//! 32 KB-stack threads reaches one (`tools/job-threads.py`'s entries, round
//! 3 of the shape agent); the main thread and the 128 KB job workers have
//! room for the original's 8 KB frame here too.

use super::{mem, ported};
use ffx_vfx::shape_draw::bridge::{self as shape, FRAME_SIZE};

/// The original's frame in words (`0x1f3c` bytes below `ebp`).
const FRAME_WORDS: usize = FRAME_SIZE as usize / 4;

ported! {
    /// `op_shape_draw(shape, cmd)`: the head, one of the six vertex loops
    /// and the tail, the frame words on this stack. The result is the
    /// tail's `eax` (`magic_0481.dll` keeps it as its next packet cursor):
    /// `cmd +0x14`, or the cursor, which with `cmd +2` bit 2 points into
    /// the original's frame: it is computed from where that frame would be
    /// (`ebp` = the argument slots - 8), so the value is the original's on
    /// every path.
    op_shape_draw @ 0x007e_d0c0 => extern "C" fn op_shape_draw(shape_rec: u32, cmd: u32; slots: u32) -> u32 {
        let mut words = super::StackFrame::<FRAME_WORDS>::new();
        let frame = shape::Frame {
            ebp: words.at().wrapping_add(FRAME_SIZE),
            original: slots.wrapping_sub(8),
        };
        shape::op_shape_draw_at(&mut mem(), shape_rec, cmd, frame)
    }
}
