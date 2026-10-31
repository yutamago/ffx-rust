//! `magic_0680.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's task (0x10001af0, `(task)`, a task callback): the angle
/// `*G_A` (operand `+0x07`) turns by `S` (`+0x0f`), wrapped once by `W`
/// (`+0x20`/`+0x37`) above `H` (`+0x16`) or below `L` (`+0x2a`); the word
/// `+0x3c` is copied to `+0x49` and `+0x4e`; the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when they say 1
/// and the count at `+0x5e` is not above 0, the task's state `+0x14`
/// becomes 3 and its record (`task[8]`) gets the callback at `+0x75`.
/// Returns the record, else the objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &mut G, off: u32| g.read::<f32>(w(off));
    let mut a = add(k(g, 0x7), k(g, 0xf));
    // comiss a, H; jbe: below, equal or unordered
    if a > k(g, 0x16) {
        a = sub(a, k(g, 0x20));
    } else if k(g, 0x2a) > a {
        a = add(a, k(g, 0x37));
    }
    let e = g.read::<u32>(w(0x3c));
    g.write::<f32>(w(0x44), a);
    g.write::<u32>(w(0x49), e);
    g.write::<u32>(w(0x4e), e);
    // push ebp: the call's return address
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x53), fr.esp.wrapping_sub(8));
    if r != 1 || g.read::<i32>(w(0x5e)) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x75));
    Ret::eax(rec)
}

/// [`task`] (123 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 123,
    hash: 0x0982_e5cf_7187_2f2e,
    calls: &[(0x53, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};
