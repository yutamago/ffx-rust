//! M5 stage 1, list `phyre-framework-input`, round big-phyre3
//! (`ffx_pother_b::m5p_framework_input_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`bp3_*`). The
//! frames are per call.

use ffx_pother_b::m5p_framework_input_bp3 as fi;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PFramework_PApplicationScriptProxy_readInputLog(app, path)`:
    /// the reader, the text array and the scan targets in a frame.
    Phyre_PFramework_PApplicationScriptProxy_readInputLog @ 0x0062_b5f0 => extern "C" fn fibp3_0062b5f0(app: u32, path: u32) -> u32 {
        let mut f = StackFrame::<{ fi::INPUT_LOG_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * fi::INPUT_LOG_FRAME_WORDS;
        fi::input_log::read_input_log(&mut mem(), ebp, app, path)
    }
    /// `Phyre_PFramework_PInputMapping_valueOf_00628f00(mapper; input)` -> `ST0`:
    /// the normalised touch directions in a frame.
    Phyre_PFramework_PInputMapping_valueOf_00628f00 @ 0x0062_8f00 => extern "thiscall" fn fibp3_00628f00(this: u32, input: u32) -> f32 {
        let mut f = StackFrame::<{ fi::AXIS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * fi::AXIS_FRAME_WORDS;
        fi::axis::value_of(&mut mem(), ebp, this, input)
    }
    /// `PApplication_updateMouseCamera_0062e3a0(app; camera, p3, p4, p5)`:
    /// `GetClientRect`'s rectangle in a frame.
    PApplication_updateMouseCamera_0062e3a0 @ 0x0062_e3a0 => extern "thiscall" fn fibp3_0062e3a0(app: u32, cam: u32, p3: u32, p4: u32, p5: u32) {
        let mut f = StackFrame::<{ fi::MOUSE_CAMERA_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * fi::MOUSE_CAMERA_FRAME_WORDS;
        fi::mouse_camera::update_mouse_camera(&mut mem(), ebp, app, (cam, p3, p4, p5));
    }
}
