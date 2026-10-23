//! `ffx-vfx`'s ppp draws below `0x007e0000` ported by the vfx-ppp rounds
//! (`tail`, ...). All cdecl.

use super::{mem, ported};
use ffx_vfx::tail;

/// An original cdecl function through its address (rebased by the bridge).
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// An original cdecl function that returns its result in `ST0`.
fn call_ptr_f64(f: u32, args: &[u32]) -> f64 {
    super::call_cdecl_f64(f, args)
}

/// The locals of a draw whose original aligns its frame to 16 bytes, on
/// this shim's stack.
#[repr(C, align(16))]
struct Locals([u8; 0x180]);

impl Locals {
    /// The original's frame pointer: the end of the block.
    fn ebp(&mut self) -> u32 {
        u32::try_from(self.0.as_mut_ptr() as usize + 0x180).unwrap_or(0)
    }
}

/// `pppKeShpTailDraw`, `pppKeShpTailXDraw`: the request, vectors and
/// matrices in a 16-aligned block, as the original aligns its frame.
fn shp_tail(v: &tail::bridge::Tail, obj: u32, key: u32, ctrl: u32) {
    let mut locals = Locals([0; 0x180]);
    let ebp = locals.ebp();
    tail::bridge::ppp_ke_shp_tail_draw(
        &mut mem(),
        v,
        obj,
        key,
        ctrl,
        ebp,
        &mut |_, f, a| call_ptr(f, a),
        &mut |_, f, a| call_ptr_f64(f, a),
    );
    std::hint::black_box(&mut locals.0);
}

ported! {
    /// `pppKeShpTailDraw(obj, key, ctrl)`.
    pppKeShpTailDraw @ 0x0074_c570 => extern "C" fn ppp_ke_shp_tail_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL, obj, key, ctrl);
    }
    /// `pppKeShpTailXDraw(obj, key, ctrl)`.
    pppKeShpTailXDraw @ 0x0074_d570 => extern "C" fn ppp_ke_shp_tail_x_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL_X, obj, key, ctrl);
    }
}
