//! The PC menu's audio page callback (M4 coverage 2, list `c2-new`):
//! `ExitAudioMenu`, which only the `iggy-menus` replay reaches (the Audio
//! page left with Backspace or Esc).

use ffx_bridge::x87;

/// `IggyMenuHandlers_addCall(this; f, arg)` and the thunk it queues
/// (`0x006ff760`: `jmp iSoundUpdate`).
pub const ADD_CALL: u32 = 0x00a2_8df0;
pub const SOUND_UPDATE_THUNK: u32 = 0x006f_f760;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `FFEscMenu_onExitAudioMenu(handlers, args)` (cdecl, registered
    /// under `"ExitAudioMenu"` by `FFEscMenu_registerAudioPreviewCallbacks`):
    /// unless the Flash argument (the `double` at `args + 8`, toward zero
    /// by `ftol2_sse`) is 1, queues `iSoundUpdate` (through its thunk, with
    /// 0) on the handlers. Returns 1 or `addCall`'s `eax`.
    pub fn on_exit_audio_menu(h: &mut impl Host, handlers: u32, args: u32) -> u32 {
        let lo = u64::from(h.read::<u32>(args + 8));
        let hi = u64::from(h.read::<u32>(args + 12));
        let v = x87::ftol(h, f64::from_bits(hi << 32 | lo));
        if v == 1 {
            return v;
        }
        let f = h.rebase(SOUND_UPDATE_THUNK);
        h.call_sym(ADD_CALL, Conv::Thiscall(handlers), &[f, 0])
    }
}
