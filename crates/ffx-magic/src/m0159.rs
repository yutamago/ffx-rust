//! `magic_0159.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code in [`crate::l3_common`]).

use crate::runtime::slot_target;
use crate::{Game, Image, Sig};
use ffx_bridge::x87::cvttsd2si;

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// The character's facing as a whole-number angle (`edx` = the output
/// record: `+0x0` and `+0x8` cleared, `+0x4` the angle; `ecx` the
/// caller's, the local's start value): for a monster (flag byte at
/// operand `+0x16`, bit 0) `K - Ch_GetRot(G_CHR)` (slot 333, ST0 stored
/// as `f32`; `G_CHR` at `+0x24`), else `K - MsGetChrDir(G_ID, &local)`
/// (slot 601 writes the local, `[ebp - 4]`; `G_ID` at `+0x50`); then
/// `* M / D` and `cvttss2si` (`K` at `+0x37`/`+0x60`, `M` `+0x70`, `D`
/// `+0x78`). Returns the angle (`eax`). `esp` is the original's at its
/// return address (the local is `esp - 8`).
///
/// 0x100065f0 (136 bytes, 1 overlay).
pub fn facing_angle<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, ecx: u32, out: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let local = esp.wrapping_sub(8);
    g.write::<u32>(local, ecx);
    g.write::<u32>(out + 8, 0);
    g.write::<u32>(out, 0);
    let x = if g.read::<u8>(w(0x16)) & 1 != 0 {
        let s333 = slot_target(g, w(0x1e), 333);
        let chr = g.read::<u32>(w(0x24));
        let rot = g
            .call(s333, ecx, &[chr])
            .st0
            .map_or(INDEFINITE, |v| v as f32);
        g.read::<f32>(w(0x37)) - rot
    } else {
        let s601 = slot_target(g, w(0x4a), 601);
        let id = g.read::<u32>(w(0x50));
        g.call(s601, ecx, &[id, local]);
        g.read::<f32>(w(0x60)) - g.read::<f32>(local)
    };
    let x = x * g.read::<f32>(w(0x70)) / g.read::<f32>(w(0x78));
    let angle = cvttsd2si(f64::from(x));
    g.write::<u32>(out + 4, angle);
    angle
}

/// [`facing_angle`].
pub const FACING_ANGLE: Sig = Sig {
    len: 136,
    hash: 0x2014_bf71_6d73_0e61,
    calls: &[],
    imports: &[],
};
