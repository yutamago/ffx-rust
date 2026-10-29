//! `magic_0078.dll`'s functions (shared with `magic_0079`).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10006470 (a callback, cdecl `(task)`): the effect's clock (`+0x2c4`
/// of the object at `task +0x4`) advances by `K0`; when its hold count
/// u16 `+0x2c8` is 0 and the clock reached `K1`, slot 239
/// (`G_HANDLE`, -5, 0) and the task (`*task`) is unlinked from its ring
/// (`+0x4` prev, `+0x8` next) and left pointing at itself. Returns the
/// object, or after the unlink the task's old previous.
pub fn clock<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task + 4);
    let held = g.read::<u16>(obj + 0x2c8) != 0;
    let t = f32::from_bits(g.read::<u32>(obj + 0x2c4)) + f32::from_bits(g.read::<u32>(w(0x1e)));
    g.write::<u32>(obj + 0x2c4, t.to_bits());
    // jne; comiss; jb: unordered skips too
    let due = matches!(
        t.partial_cmp(&f32::from_bits(g.read::<u32>(w(0x2f)))),
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
    );
    if held || !due {
        return Ret::eax(obj);
    }
    let slot239 = slot_target(g, w(0x36), 239);
    let handle = g.read::<u32>(w(0x46));
    g.call(slot239, 0, &[handle, (-5i32).cast_unsigned(), 0]);
    let node = g.read::<u32>(task);
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev + 8, next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    Ret::eax(prev)
}

/// `clock` (108 bytes; 2 overlays).
pub const CLOCK: Sig = Sig {
    len: 108,
    hash: 0x9c13_5c9a_7e59_6503,
    calls: &[],
    imports: &[],
};
