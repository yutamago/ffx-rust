//! M5 stage 1, round 10 (`tail10`): the last game-code and engine tail
//! (`ffx_pother_a::l10_print`, ...). Parity tests in the lists' oracle
//! files (`m5p_scripting.rs`, ...).
//!
//! Threads: the script-running threads (main, the async init job) for
//! `print`. Frames are per call.

use super::{StackFrame, mem, ported};
use ffx_pother_a::l10_print::bridge as print;

ported! {
    /// `(L) -> 0`: Lua's `print`, its 0x200-byte buffer in a frame.
    Phyre_PScripting_Print @ 0x005d_5c00 => extern "C" fn phyre_pscripting_print(l: u32) -> u32 {
        let mut buf = StackFrame::<0x80>::new();
        let at = buf.at();
        print::print(&mut mem(), l, at)
    }
}
