//! `ffx-vfx`'s ppp draws below `0x007e0000` ported by the vfx-ppp rounds
//! (`tail`, `mdl_cam`, ...). All cdecl.

use super::{mem, ported};
use ffx_vfx::{mdl_cam, tail};

/// An original cdecl function through its address (rebased by the bridge).
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// An original cdecl function that returns its result in `ST0`.
fn call_ptr_f64(f: u32, args: &[u32]) -> f64 {
    super::call_cdecl_f64(f, args)
}

/// The locals (`N` bytes) of a draw whose original aligns its frame to
/// 16 bytes, on this shim's stack.
#[repr(C, align(16))]
struct Locals<const N: usize>([u8; N]);

impl<const N: usize> Locals<N> {
    const fn new() -> Self {
        Self([0; N])
    }

    /// The original's frame pointer: the end of the block.
    fn ebp(&mut self) -> u32 {
        u32::try_from(self.0.as_mut_ptr() as usize + N).unwrap_or(0)
    }
}

/// `pppKeShpTail*Draw`: the request, vectors and matrices in a 16-aligned
/// block, as the original aligns its frame.
fn shp_tail(v: &tail::bridge::Tail, obj: u32, key: u32, ctrl: u32) {
    let mut locals = Locals::<0x180>::new();
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

/// `pppKeShpTail3*Draw`: the request, vectors and matrices in a 16-aligned
/// block, as the original aligns its frame. The block starts zeroed: the
/// words the original reads there unwritten (stale stack, scratch VU0
/// registers only) come out 0.
fn shp_tail3(v: &tail::bridge::Tail3, obj: u32, key: u32, ctrl: u32) {
    let mut locals = Locals::<0x1b0>::new();
    let ebp = locals.ebp();
    tail::bridge::ppp_ke_shp_tail3_draw(
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
    /// `pppKeShpTail3Draw(obj, key, ctrl)`.
    pppKeShpTail3Draw @ 0x0075_0620 => extern "C" fn ppp_ke_shp_tail3_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail3(&tail::bridge::TAIL3, obj, key, ctrl);
    }
    /// `pppKeShpTail3XDraw(obj, key, ctrl)`.
    pppKeShpTail3XDraw @ 0x0075_1fc0 => extern "C" fn ppp_ke_shp_tail3_x_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail3(&tail::bridge::TAIL3_X, obj, key, ctrl);
    }
    /// `pppKeShpTailPhtDraw(obj, key, ctrl)`: the request, vectors,
    /// matrices and the distortion sprite's record in a 16-aligned block.
    pppKeShpTailPhtDraw @ 0x0075_4300 => extern "C" fn ppp_ke_shp_tail_pht_draw(obj: u32, key: u32, ctrl: u32) {
        let mut locals = Locals::<0x250>::new();
        let ebp = locals.ebp();
        tail::bridge::ppp_ke_shp_tail_pht_draw(
            &mut mem(),
            obj,
            key,
            ctrl,
            ebp,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, f, a| call_ptr_f64(f, a),
        );
        std::hint::black_box(&mut locals.0);
    }
    /// `pppKeShpTailDraw(obj, key, ctrl)`.
    pppKeShpTailDraw @ 0x0074_c570 => extern "C" fn ppp_ke_shp_tail_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL, obj, key, ctrl);
    }
    /// `pppKeShpTailXDraw(obj, key, ctrl)`.
    pppKeShpTailXDraw @ 0x0074_d570 => extern "C" fn ppp_ke_shp_tail_x_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL_X, obj, key, ctrl);
    }
    /// `pppKeShpTail2Draw(obj, key, ctrl)`.
    pppKeShpTail2Draw @ 0x0074_e570 => extern "C" fn ppp_ke_shp_tail2_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL2, obj, key, ctrl);
    }
    /// `pppKeShpTail2XDraw(obj, key, ctrl)`.
    pppKeShpTail2XDraw @ 0x0074_f5b0 => extern "C" fn ppp_ke_shp_tail2_x_draw(obj: u32, key: u32, ctrl: u32) {
        shp_tail(&tail::bridge::TAIL2_X, obj, key, ctrl);
    }
    /// `pppDrawMdlCameraDraw(obj, key, prog)`: the request and matrices in
    /// a 16-aligned block, as the original aligns its frame.
    pppDrawMdlCameraDraw @ 0x0073_d930 => extern "C" fn ppp_draw_mdl_camera_draw(obj: u32, key: u32, prog: u32) {
        let mut locals = Locals::<0x1b0>::new();
        let ebp = locals.ebp();
        mdl_cam::bridge::ppp_draw_mdl_camera_draw(
            &mut mem(),
            obj,
            key,
            prog,
            ebp,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, f, a| call_ptr_f64(f, a),
        );
        std::hint::black_box(&mut locals.0);
    }
    /// `pppDrawMdlCameraLoopDraw(obj, key, prog)`: the request and matrices
    /// in a 16-aligned block, as the original aligns its frame.
    pppDrawMdlCameraLoopDraw @ 0x0073_fa50 => extern "C" fn ppp_draw_mdl_camera_loop_draw(obj: u32, key: u32, prog: u32) {
        let mut locals = Locals::<0x1d0>::new();
        let ebp = locals.ebp();
        mdl_cam::bridge::ppp_draw_mdl_camera_loop_draw(
            &mut mem(),
            obj,
            key,
            prog,
            ebp,
            &mut |_, f, a| call_ptr(f, a),
        );
        std::hint::black_box(&mut locals.0);
    }
    /// `pppKeMdlBmpDraw(obj, key, ctrl)`: the matrices, vectors and the
    /// body's argument block in a 16-aligned block, as the original aligns
    /// its frame.
    pppKeMdlBmpDraw @ 0x0074_90d0 => extern "C" fn ppp_ke_mdl_bmp_draw(obj: u32, key: u32, ctrl: u32) {
        let mut locals = Locals::<0x110>::new();
        let ebp = locals.ebp();
        mdl_cam::bridge::ppp_ke_mdl_bmp_draw(&mut mem(), obj, key, ctrl, ebp, &mut |_, f, a| call_ptr(f, a));
        std::hint::black_box(&mut locals.0);
    }
    /// `pppDrawMdlSeaDraw(obj, key, prog)`: the request, the texture name
    /// and record in a 16-aligned block, as the original aligns its frame.
    pppDrawMdlSeaDraw @ 0x0073_abe0 => extern "C" fn ppp_draw_mdl_sea_draw(obj: u32, key: u32, prog: u32) {
        let mut locals = Locals::<0x2f0>::new();
        let ebp = locals.ebp();
        mdl_cam::bridge::ppp_draw_mdl_sea_draw(&mut mem(), obj, key, prog, ebp, &mut |_, f, a| call_ptr(f, a));
        std::hint::black_box(&mut locals.0);
    }
}
