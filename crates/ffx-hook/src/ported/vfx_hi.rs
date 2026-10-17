//! `ffx-vfx`'s high range (`0x00a30000..0x00a80000`): the op layer's model
//! helpers and the `opus_0437`/`0582`/`0283` effects
//! (`docs/formats/vfx.md`, "High range"). All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::{hi_md, hi_opus};

/// An original cdecl function through its address (rebased by the bridge).
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
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
    /// `op_md_tex0_change(packet, x)`: the range `0..=0x4000`.
    op_md_tex0_change @ 0x00a6_9c80 => extern "C" fn op_md_tex0_change(packet: u32, _x: u32) -> u32 {
        hi_md::bridge::op_md_tex0_change_range(&mut mem(), packet, 0, 0x4000)
    }
    /// `op_md_tex0_change_range_00a69ca0(packet, x, lo, hi)`.
    op_md_tex0_change_range_00a69ca0 @ 0x00a6_9ca0 => extern "C" fn op_md_tex0_change_range(packet: u32, _x: u32, lo: u32, hi: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 arguments
        hi_md::bridge::op_md_tex0_change_range(&mut mem(), packet, lo as u16, hi as u16)
    }
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
}
