//! M5 stage 1, list `vfx`, round 5 batch 17: Iggy `GDraw` D3D11's
//! filter quad (`gdraw_filterQuad_00a38aa0`, a callback stored into a
//! record at `0x00a39495`): one input texture, a pixel shader chosen by the constant
//! count, the constants mapped in, and a quad drawn over the rectangle.

use ffx_bridge::Global;

/// `GDraw`'s state (`+4` the immediate context, `+0x204` the pixel shaders
/// (8 bytes apart), `+0x7b4` the sampler, `+0x994` the constant buffer,
/// `+0xac0`/`+0xac4` the target size).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);

/// The device context's methods (`ID3D11DeviceContext`, stdcall with the
/// context first): `PSSetShaderResources`, `PSSetShader`,
/// `PSSetSamplers`, `Unmap`, `PSSetConstantBuffers`.
pub mod vt {
    pub const PS_SET_SHADER_RESOURCES: u32 = 0x20;
    pub const PS_SET_SHADER: u32 = 0x24;
    pub const PS_SET_SAMPLERS: u32 = 0x28;
    pub const UNMAP: u32 = 0x3c;
    pub const PS_SET_CONSTANT_BUFFERS: u32 = 0x40;
}

// ---- callees ----

/// `gdraw_MapBuffer_00a3cec0(buffer)` -> the mapped bytes, `memcpy(dst,
/// src, n)`, `GDraw`'s quad draw `0x00a36090(colour, stats, x0, y0, x1,
/// y1)` (cdecl).
pub const MAP_BUFFER: u32 = 0x00a3_cec0;
pub const MEMCPY: u32 = 0x0094_9256;
pub const DRAW_QUAD: u32 = 0x00a3_6090;

pub mod bridge {
    use super::{DRAW_QUAD, GDRAW, MAP_BUFFER, MEMCPY, vt};
    use crate::h4_e1::int_f32;
    use ffx_graphic::host::{Conv, Host};

    fn method(h: &mut impl Host, ctx: u32, off: u32, args: &[u32]) {
        let vtable: u32 = h.read(ctx);
        let f: u32 = h.read(vtable + off);
        let mut all = vec![ctx];
        all.extend_from_slice(args);
        // call width: 4
        h.call(f, Conv::Stdcall, &all);
    }

    fn context(h: &mut impl Host) -> u32 {
        let state = GDRAW.get(h);
        h.read(state + 4)
    }

    /// `gdraw_filterQuad_00a38aa0(filter, n, constants, rect, colour, _,
    /// head, stats)` -> `eax`: the filter's input (`+0x1c`; its `+4` view
    /// and the sampler, or a null view through slot 0) bound to the pixel
    /// shader `n`; the constant buffer gets the 16 bytes at `head` and `n`
    /// 16-byte constants, then goes to slot 1; the quad over `rect` (four
    /// `i32` as floats) in `colour`. The input then takes the target size.
    /// Writes its argument slots (`filter` 0 with no input, `n` the
    /// buffer, `head` the first float). `eax` is the target height, or the
    /// quad draw's with no input.
    #[allow(clippy::too_many_arguments)]
    pub fn filter_quad(
        h: &mut impl Host,
        slots: u32,
        filter: u32,
        n: u32,
        constants: u32,
        rect: u32,
        colour: u32,
        head: u32,
        stats: u32,
    ) -> u32 {
        let ctx = context(h);
        let input: u32 = h.read(filter + 0x1c);
        if input == 0 {
            h.write(slots, 0_u32);
            method(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[0, 1, slots]);
        } else {
            method(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[0, 1, input + 4]);
            let sampler = GDRAW.get(h) + 0x7b4;
            method(h, ctx, vt::PS_SET_SAMPLERS, &[0, 1, sampler]);
        }
        let state = GDRAW.get(h);
        let ctx: u32 = h.read(state + 4);
        let shader: u32 = h.read((state + 0x204).wrapping_add(n.wrapping_mul(8)));
        method(h, ctx, vt::PS_SET_SHADER, &[shader, 0, 0]);
        let buffer: u32 = h.read(GDRAW.get(h) + 0x994);
        let dst = h.call_sym(MAP_BUFFER, Conv::Cdecl, &[buffer]);
        for k in 0..4 {
            let w: u32 = h.read(head + 4 * k);
            h.write(dst + 4 * k, w);
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[dst + 0x10, constants, n << 4]);
        let state = GDRAW.get(h);
        let buffer: u32 = h.read(state + 0x994);
        let ctx: u32 = h.read(state + 4);
        h.write(slots + 4, buffer);
        method(h, ctx, vt::UNMAP, &[buffer, 0]);
        let ctx = context(h);
        method(h, ctx, vt::PS_SET_CONSTANT_BUFFERS, &[1, 1, slots + 4]);
        let r = |k: u32| int_f32(h.read::<i32>(rect + 4 * k));
        let (y1, x1, y0, x0) = (r(3), r(2), r(1), r(0));
        h.write(slots + 0x18, x0);
        let eax = h.call_sym(DRAW_QUAD, Conv::Cdecl, &[colour, stats, x0, y0, x1, y1]);
        let input: u32 = h.read(filter + 0x1c);
        if input == 0 {
            return eax;
        }
        let state = GDRAW.get(h);
        let w: u32 = h.read(state + 0xac0);
        h.write(input + 0x28, w);
        let hh: u32 = h.read(state + 0xac4);
        h.write(input + 0x2c, hh);
        hh
    }
}
