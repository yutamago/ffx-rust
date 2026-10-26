//! M4 port, list p6-render-c, continuation round 2 (batch 14): the
//! shims. Ports in `ffx_prender::p6_app`.

use super::{StackFrame, mem, ported};
use ffx_prender::p6_app as ap;

ported! {
    /// An owned-array destructor (`this`).
    Phyre_PDynamicGeometry_arraysDtor_005bf7d0 @ 0x005b_f7d0 => extern "thiscall" fn x_005bf7d0(this: u32) -> u32 {
        ap::bridge::array_dtor(&mut mem(), this, const { ap::array_dtor(0x005b_f7d0) })
    }
    /// An owned-array destructor (`this`).
    Phyre_PText_arraysDtor_00a2bbd0 @ 0x00a2_bbd0 => extern "thiscall" fn x_00a2bbd0(this: u32) -> u32 {
        ap::bridge::array_dtor(&mut mem(), this, const { ap::array_dtor(0x00a2_bbd0) })
    }
    /// A material's buffer size (`this`; ctx).
    Phyre_PRendering_PMaterial_bufferSize_005c6680 @ 0x005c_6680 => extern "thiscall" fn x_005c6680(this: u32, ctx: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        ap::bridge::material_size(&mut mem(), this, frame.at(), ctx)
    }
    /// `PRendererBase::initialize(this; base, size, command_size)`.
    Phyre_PRendering_PRendererBase_initialize_005b8da0 @ 0x005b_8da0 => extern "thiscall" fn x_005b8da0(this: u32, base: u32, size: u32, command_size: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        ap::bridge::renderer_initialize(&mut mem(), this, frame.at(), base, size, command_size)
    }
    /// The application's input devices (`this`).
    Phyre_PFramework_PApplication_setupInputs_00629870 @ 0x0062_9870 => extern "thiscall" fn x_00629870(this: u32) -> u32 {
        ap::bridge::setup_inputs(&mut mem(), this)
    }
    /// A segment's morph weights (out, ctx) -> their bytes.
    Phyre_PDynamicGeometry_PMorphModifier_weights_005cb750 @ 0x005c_b750 => extern "C" fn x_005cb750(out: u32, ctx: u32) -> u32 {
        ap::bridge::morph_weights(&mut mem(), out, ctx)
    }
}
