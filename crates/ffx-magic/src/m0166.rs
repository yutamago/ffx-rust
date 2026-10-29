//! `magic_0166.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::{Game, Image, Sig};

/// Resets an effect record (`ecx`): state `+0x114` = 5; `+0x100..+0x120`
/// and the vectors at `+0xc0`, `+0xd0`, `+0xe0` cleared; the scales
/// `+0xdc`, `+0xec`, `+0xf0..+0xfc` = 1.0; byte `+0x126` cleared and the
/// word `+0x124` = 0x8000. Returns `eax` = 0x8000.
///
/// 0x10001ec0 (270 bytes, 1 overlay).
pub fn reset_record<G: Game>(g: &mut G, _img: &Image, _f: u32, this: u32) -> u32 {
    const ONE: u32 = 0x3f80_0000;
    g.write::<u32>(this + 0x114, 5);
    for off in [0x11c, 0x100, 0x104, 0x108, 0x10c, 0x110, 0x118, 0x120] {
        g.write::<u32>(this + off, 0);
    }
    g.write::<u8>(this + 0x126, 0);
    g.write::<u16>(this + 0x124, 0x8000);
    for off in [0xcc, 0xc8, 0xc4, 0xc0, 0xd8, 0xd4, 0xd0] {
        g.write::<u32>(this + off, 0);
    }
    g.write::<u32>(this + 0xdc, ONE);
    for off in [0xe8, 0xe4, 0xe0] {
        g.write::<u32>(this + off, 0);
    }
    for off in [0xec, 0xfc, 0xf8, 0xf4, 0xf0] {
        g.write::<u32>(this + off, ONE);
    }
    0x8000
}

/// [`reset_record`].
pub const RESET_RECORD: Sig = Sig {
    len: 270,
    hash: 0xc85f_8c8d_81a9_e47b,
    calls: &[],
    imports: &[],
};
