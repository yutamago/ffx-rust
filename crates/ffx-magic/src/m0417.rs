//! `magic_0417.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A free sprite record (0x10004430, no arguments, no frame): from the
/// rover `*R` (`+0x02`) on, the first of the `*N` (`+0x0a`) records
/// (0x7c bytes from `*B`, `+0x17`) whose `+0x78` is 0 (the rover wrapping
/// at `n`): its `+0x74`, `+0x68`, `+0x6c` 0, colours `+0x3c`/`+0x40`
/// 0x80808080, `+0x54` 1.0 and `+0x78` 1, the rover one past it. Returns
/// the record, or 0 when every one is taken (the rover kept).
pub fn take_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut rover = g.read::<u32>(w(0x02));
    let n = g.read::<i32>(w(0x0a));
    let base = g.read::<u32>(w(0x17));
    let mut tried = 0i32;
    while tried < n {
        let rec = rover.wrapping_mul(0x7c).wrapping_add(base);
        rover = rover.wrapping_add(1);
        if rover.cast_signed() >= n {
            rover = 0;
        }
        if g.read::<u32>(rec + 0x78) == 0 {
            for off in [0x74, 0x68, 0x6c] {
                g.write::<u32>(rec + off, 0);
            }
            g.write::<u32>(rec + 0x3c, 0x8080_8080);
            g.write::<u32>(rec + 0x40, 0x8080_8080);
            g.write::<u32>(rec + 0x54, 0x3f80_0000);
            g.write::<u32>(rec + 0x78, 1);
            g.write::<u32>(w(0x68), rover);
            return Ret::eax(rec);
        }
        tried += 1;
    }
    Ret::eax(0)
}

/// [`take_record`] (110 bytes, 1 overlay).
pub const TAKE_RECORD: Sig = Sig {
    len: 110,
    hash: 0xf79f_c97b_75cf_808f,
    calls: &[],
    imports: &[],
};
