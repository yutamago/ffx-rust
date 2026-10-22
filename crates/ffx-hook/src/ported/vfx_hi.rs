//! `ffx-vfx`'s high range (`0x00a30000..0x00a80000`): the op layer's model
//! helpers and the `opus_0437`/`0582`/`0283` effects
//! (`docs/formats/vfx.md`, "High range"). All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::{hi_0283, hi_draw, hi_light, hi_md, hi_opus};

/// An original cdecl function through its address (rebased by the bridge).
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// MSVCR110's `_CIcos` (`cos`) or `_CIsin` through the game's import slot
/// (`0x00b0c3b4`, `0x00b0c3b8`) on a 53-bit operand: loaded as `fld
/// qword`, the result stored as `fstp dword` through `edi`.
fn crt_trig(cos: bool, x: f64) -> f32 {
    let slot = crate::rebase(if cos { 0x00b0_c3b4 } else { 0x00b0_c3b8 }) as *const usize;
    let mut out = 0f32;
    // SAFETY: the slot holds the CRT's intrinsic, which takes its operand
    // in ST0 and leaves the result there; the asm loads it, calls it and
    // stores the result through edi (which the callee keeps), leaving the
    // x87 stack empty.
    unsafe {
        let f = slot.read();
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp dword ptr [edi]",
            x = in(reg) &raw const x,
            f = in(reg) f,
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
    }
    out
}

ported! {
    /// `op_md_copy(src, dst)`.
    op_md_copy @ 0x00a6_9290 => extern "C" fn op_md_copy(src: u32, dst: u32) -> u32 {
        hi_md::bridge::op_md_copy(&mut mem(), src, dst, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_list_count10_00a6a2b0(list)`.
    op_md_list_count10_00a6a2b0 @ 0x00a6_a2b0 => extern "C" fn op_md_list_count10(list: u32) -> u32 {
        hi_md::bridge::op_md_list_count10(&mem(), list)
    }
    /// `op_oef2_entry_link_00a6a320(e, i)`.
    op_oef2_entry_link_00a6a320 @ 0x00a6_a320 => extern "C" fn op_oef2_entry_link(e: u32, i: u32) -> u32 {
        hi_md::bridge::op_oef2_entry_link(&mem(), e, i)
    }
    /// `op_normal_light44_ffx(obj, model, work)`.
    op_normal_light44_ffx @ 0x00a6_cf10 => extern "C" fn op_normal_light44_ffx(_obj: u32, model: u32, work: u32) -> u32 {
        hi_md::bridge::op_normal_light44_ffx(&mut mem(), model, work, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0437_dot_init_00a6e540(b, n)`: `op_random_per`'s three stale
    /// words (the caller's `edi`, `esi`, `ebx`) as 0.
    opus_0437_dot_init_00a6e540 @ 0x00a6_e540 => extern "C" fn opus_0437_dot_init(b: u32, n: u32) -> u32 {
        hi_opus::bridge::opus_0437_dot_init(&mut mem(), b, n, [0; 3], &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0437_dot_set(b, pos, colour)`.
    opus_0437_dot_set @ 0x00a6_e5d0 => extern "C" fn opus_0437_dot_set(b: u32, pos: u32, colour: u32) -> u32 {
        hi_opus::bridge::opus_0437_dot_set(&mut mem(), b, pos, colour, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0582_break_bytes_00a72310(list)`.
    opus_0582_break_bytes_00a72310 @ 0x00a7_2310 => extern "C" fn opus_0582_break_bytes(list: u32) -> u32 {
        hi_md::bridge::opus_0582_break_bytes(&mem(), list)
    }
    // op_md_tex0_change (0x00a69c80) and op_md_tex0_change_limit (0x00a69ca0)
    // are hooked in vfx.rs (ported there first); hi_md has a second port.
    /// `op_md_vn_index(packet, out, table)`.
    op_md_vn_index @ 0x00a6_9f10 => extern "C" fn op_md_vn_index(packet: u32, out: u32, table: u32) -> u32 {
        hi_md::bridge::op_md_vn_index(&mut mem(), packet, out, table)
    }
    /// `op_ffxlight_to_omdlight_0279(colours, dirs)`.
    op_ffxlight_to_omdlight_0279 @ 0x00a6_91c0 => extern "C" fn op_ffxlight_to_omdlight_0279(colours: u32, dirs: u32) -> u32 {
        hi_md::bridge::op_ffxlight_to_omdlight_0279(&mut mem(), colours, dirs, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_ffxambient_to_omdambient()`: its locals in a buffer on this
    /// shim's stack.
    op_ffxambient_to_omdambient @ 0x00a6_8ff0 => extern "C" fn op_ffxambient_to_omdambient() -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x20]);
        let mut locals = Locals([0; 0x20]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x20;
        hi_md::bridge::op_ffxambient_to_omdambient(&mut mem(), frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_list_bounds_00a6a0c0(list, m, min, max)`: its locals in a
    /// buffer on this shim's stack.
    op_md_list_bounds_00a6a0c0 @ 0x00a6_a0c0 => extern "C" fn op_md_list_bounds(list: u32, m: u32, min: u32, max: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x60]);
        let mut locals = Locals([0; 0x60]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x60;
        hi_md::bridge::op_md_list_bounds(&mut mem(), list, m, min, max, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0582_dot_set_00a73740(b, rec)`.
    opus_0582_dot_set_00a73740 @ 0x00a7_3740 => extern "C" fn opus_0582_dot_set(b: u32, rec: u32) -> u32 {
        hi_opus::bridge::opus_0582_dot_set(&mut mem(), b, rec)
    }
    /// `opus_0437_dot_draw(b)`.
    opus_0437_dot_draw @ 0x00a6_e2e0 => extern "C" fn opus_0437_dot_draw(b: u32) -> u32 {
        hi_opus::bridge::opus_0437_dot_draw(&mut mem(), b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0437_poly_init_00a6e1b0(b, list, m, k)`.
    opus_0437_poly_init_00a6e1b0 @ 0x00a6_e1b0 => extern "C" fn opus_0437_poly_init(b: u32, list: u32, m: u32, k: u32) -> u32 {
        hi_opus::bridge::opus_0437_poly_init(&mut mem(), b, list, m, k, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0437_poly_check(list, b, dots, m, limit)`: its matrix and
    /// vector in a buffer on this shim's stack; the two frame words the
    /// original never writes as 0.
    opus_0437_poly_check @ 0x00a6_ed40 => extern "C" fn opus_0437_poly_check(list: u32, b: u32, dots: u32, m: u32, limit: f32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x60]);
        let mut locals = Locals([0; 0x60]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x60;
        hi_opus::bridge::opus_0437_poly_check(&mut mem(), list, b, dots, m, limit, [0; 2], frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0279_draw(obj, init, rec, _, tex)`: `eax` the last
    /// `g_GsRegisterSink` read.
    opus_0279_draw @ 0x00a7_0210 => extern "C" fn opus_0279_draw(obj: u32, init: u32, rec: u32, _unused: u32, tex: u32) -> u32 {
        hi_draw::bridge::opus_0279_draw(&mut mem(), obj, init, rec, tex, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_md_etc_draw_00a693b0(obj, b, m, view, e)`: the init block in a
    /// buffer on this shim's stack, its never-written words 0.
    opu_md_etc_draw_00a693b0 @ 0x00a6_93b0 => extern "C" fn opu_md_etc_draw(obj: u32, b: u32, m: u32, view: u32, e: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x80]);
        let mut locals = Locals([0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x80;
        hi_draw::bridge::opu_md_etc_draw(&mut mem(), obj, b, m, view, e, [0; 7], frame, &mut |_, c, x| crt_trig(c, x), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_x64_draw_00a69300(obj, b, e)`: the init block in a buffer on
    /// this shim's stack, its never-written words 0.
    op_md_x64_draw_00a69300 @ 0x00a6_9300 => extern "C" fn op_md_x64_draw(obj: u32, b: u32, e: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x80]);
        let mut locals = Locals([0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x80;
        hi_draw::bridge::op_md_x64_draw(&mut mem(), obj, b, e, [0; 8], frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_vertex_normal_point44_ffx(obj, model, work, chr)`: its locals
    /// in a buffer on this shim's stack.
    op_vertex_normal_point44_ffx @ 0x00a6_d380 => extern "C" fn op_vertex_normal_point44_ffx(_obj: u32, model: u32, work: u32, chr: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x100]);
        let mut locals = Locals([0; 0x100]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x100;
        hi_light::bridge::op_vertex_normal_point44_ffx(&mut mem(), model, work, chr, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_vertex_normal_point44_ffx_2(obj, model, work, chr)`: its
    /// locals in a buffer on this shim's stack.
    op_vertex_normal_point44_ffx_2 @ 0x00a6_d670 => extern "C" fn op_vertex_normal_point44_ffx_2(_obj: u32, model: u32, work: u32, chr: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x190]);
        let mut locals = Locals([0; 0x190]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x190;
        hi_light::bridge::op_vertex_normal_point44_ffx_2(&mut mem(), model, work, chr, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_0437_vertex_normal_point44_ffx(packet, model, work, base, mul1,
    /// mul0, slot)`: its locals in a buffer on this shim's stack.
    op_0437_vertex_normal_point44_ffx @ 0x00a6_d890 => extern "C" fn op_0437_vertex_normal_point44_ffx(packet: u32, model: u32, work: u32, base: u32, mul1: u32, mul0: u32, slot: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x110]);
        let mut locals = Locals([0; 0x110]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x110;
        hi_light::bridge::op_0437_vertex_normal_point44_ffx(&mut mem(), packet, model, work, [base, mul1, mul0], slot, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_chara_path3(obj, init, chr)`.
    op_md_chara_path3 @ 0x00a6_a3a0 => extern "C" fn op_md_chara_path3(obj: u32, init: u32, chr: u32) -> u32 {
        hi_draw::bridge::op_md_chara_path3(&mut mem(), obj, init, chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_x64_00a6b710(obj, init)`: its locals in a buffer on this
    /// shim's stack.
    op_md_x64_00a6b710 @ 0x00a6_b710 => extern "C" fn op_md_x64(obj: u32, init: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x60]);
        let mut locals = Locals([0; 0x60]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x60;
        hi_draw::bridge::op_md_x64(&mut mem(), obj, init, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_x64_fog_00a6bfe0(obj, init)`.
    op_md_x64_fog_00a6bfe0 @ 0x00a6_bfe0 => extern "C" fn op_md_x64_fog(obj: u32, init: u32) -> u32 {
        hi_draw::bridge::op_md_x64_fog(&mut mem(), obj, init)
    }
    /// `op_md_persp_00a6bae0(obj, init)`.
    op_md_persp_00a6bae0 @ 0x00a6_bae0 => extern "C" fn op_md_persp(obj: u32, init: u32) -> u32 {
        hi_draw::bridge::op_md_persp(&mut mem(), obj, init, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_flag80_fog_00a6c3d0(obj, init)`.
    op_md_flag80_fog_00a6c3d0 @ 0x00a6_c3d0 => extern "C" fn op_md_flag80_fog(obj: u32, init: u32) -> u32 {
        hi_draw::bridge::op_md_flag80_fog(&mut mem(), obj, init, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_flag80_00a6acb0(obj, init)`: its light local in a buffer on
    /// this shim's stack.
    op_md_flag80_00a6acb0 @ 0x00a6_acb0 => extern "C" fn op_md_flag80(obj: u32, init: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x50]);
        let mut locals = Locals([0; 0x50]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x50;
        hi_draw::bridge::op_md_flag80(&mut mem(), obj, init, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_one8(obj, init)`: its light local in a buffer on this
    /// shim's stack.
    op_md_one8 @ 0x00a6_b220 => extern "C" fn op_md_one8(obj: u32, init: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x50]);
        let mut locals = Locals([0; 0x50]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x50;
        hi_draw::bridge::op_md_one8(&mut mem(), obj, init, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_one8_fog(obj, init)`.
    op_md_one8_fog @ 0x00a6_c8b0 => extern "C" fn op_md_one8_fog(obj: u32, init: u32) -> u32 {
        hi_draw::bridge::op_md_one8_fog(&mut mem(), obj, init, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op1_mdc_draw(list, init, m, view)`: its three matrices in a buffer
    /// on this shim's stack (`[init +0x20]` points into it during the
    /// call).
    op1_mdc_draw @ 0x00a6_8c90 => extern "C" fn op1_mdc_draw(list: u32, init: u32, m: u32, view: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xd0]);
        let mut locals = Locals([0; 0xd0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xd0;
        hi_draw::bridge::op1_mdc_draw(&mut mem(), list, init, m, view, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_refrection_st_float(obj, model, _, out, scale, du, dv)`: its
    /// locals in a buffer on this shim's stack.
    op_refrection_st_float @ 0x00a6_cfb0 => extern "C" fn op_refrection_st_float(obj: u32, model: u32, _unused: u32, out: u32, scale: u32, du: u32, dv: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x100]);
        let mut locals = Locals([0; 0x100]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x100;
        hi_light::bridge::op_refrection_st_float(&mut mem(), obj, model, out, scale, du, dv, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_map_draw_init_00a753f0(b, pos)`.
    opp_map_draw_init_00a753f0 @ 0x00a7_53f0 => extern "C" fn opp_map_draw_init(b: u32, pos: u32) -> u32 {
        hi_opus::bridge::opp_map_draw_init(&mut mem(), b, pos, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0283_draw(obj, rec)`: its two render-data part locals in a
    /// buffer on this shim's stack.
    opus_0283_draw @ 0x00a7_6770 => extern "C" fn opus_0283_draw(obj: u32, rec: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0x80]);
        let mut locals = Locals([0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x80;
        hi_0283::bridge::opus_0283_draw(&mut mem(), obj, rec, frame, &mut |_, f, a| call_ptr(f, a));
    }
}
