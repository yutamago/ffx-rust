// Batch 9 of tests/m5p_render_a.rs (included there).

use ffx_prender::m5p_render_a10::{self as m10, bridge as b10};

#[test]
fn physics_stream_convert_matches_the_original() {
    // Every format 0..0x33 and a few past it; strides that give 0 to 3
    // elements; memset and memcpy run as they are.
    Twins::each(|t| {
        let mut rng = Rng(0x5eed_a901);
        for format in (0..0x34u32).chain([0x40, 0x7f, 0xffff_ffff]) {
            for _ in 0..3 {
                let stride = [0, 1, 2, 3, 4, 6, 8, 12, 16][rng.below(9) as usize];
                t.stub_sym(m10::FORMAT_STRIDE, 1, false, 1, 0, Ret::Val(stride));
                let count = rng.below(5);
                let mut src: Vec<u8> = (0..0x200).map(|_| rng.next() as u8).collect();
                // half floats: no NaN payloads (they widen to signalling
                // NaNs the emulator keeps)
                for w in src.chunks_mut(2) {
                    let v = u16::from_le_bytes([w[0], w[1]]);
                    if v & 0x7c00 == 0x7c00 {
                        let v = v & !0x3ff;
                        w.copy_from_slice(&v.to_le_bytes());
                    }
                }
                let src = t.alloc(&src);
                let fill: Vec<u32> = (0..0x20).map(|_| rng.next()).collect();
                let out = if rng.chance(8) { 0 } else { t.words(&fill) };
                t.check(
                    &format!("PPhysicsMesh_convertStream_0049a520 format {format:#x}"),
                    0x0049_a520,
                    Conv::Cdecl,
                    &[out, src, format, count],
                    |g| Out::U32(b10::convert(g, out, src, format, count)),
                );
                t.reset();
            }
        }
    });
}
