//! Shared functions without a family module yet, ported by the shared-4
//! list. Each doc names the DLL and address it was ported from, its size
//! and how many DLLs carry it.

use crate::l5_objhost::{POOL_ALLOC, pool_alloc};
use crate::opu_parts::{START_PART, start_part};
use crate::runtime::slot_target;
use crate::sa_misc::{RESET_TRANSFORM, reset_transform};
use crate::{Callee, Game, Image, Sig};

/// A pool node ([`pool_alloc`] at `pa`) holding `obj`, appended to the
/// DLL's object list: the operands are the offsets of the list's tail
/// (read when empty, `tail0`), head and tail stores and count when it
/// was empty (`head`, `tail1`, `count1`), and of the tail read, store and
/// count when it was not (`tail2`, `tail3`, `count2`). Nothing happens
/// when the pool is empty.
#[allow(clippy::too_many_arguments)] // the operands of the original's two branches
fn append_object<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    pa: u32,
    obj: u32,
    [tail0, head, tail1, count1, tail2, tail3, count2]: [u32; 7],
) {
    let w = |off: u32| img.word(f + off);
    let node = pool_alloc(g, img, pa);
    if node == 0 {
        return;
    }
    g.write::<u32>(node + 4, 0);
    let tail = g.read::<u32>(w(tail0));
    g.write::<u32>(node, tail);
    let count = if tail == 0 {
        g.write::<u32>(w(head), node);
        g.write::<u32>(w(tail1), node);
        count1
    } else {
        let tail = g.read::<u32>(w(tail2));
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(w(tail3), node);
        count2
    };
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(w(count));
    g.write::<u32>(w(count), n.wrapping_add(1));
}

/// Starts an effect object (internal: `edx` = the object, `xmm3` its
/// scale, the stack `(index, owner, -, owner_b)`): the model words (scale
/// `+0x60..+0x6c`), `*G_P = ITEM` (operands `+0x04`, `+0x59`), the part
/// `list[index]` of slot 941 `(*G_D + 0x10)` started on it
/// ([`start_part`], `0x1000`, 1), its transform reset with the two owners
/// ([`reset_transform`]: `edx` = `owner`, `+0x100` = `owner_b`), then a
/// pool node holding it appended to the object list. Returns the object.
///
/// `magic_0018` 0x10003a10 (241 bytes; 76 DLLs).
#[allow(clippy::too_many_arguments)] // the original's registers and stack words
pub fn start_object_at<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    obj: u32,
    scale: f32,
    index: u32,
    owner: u32,
    owner_b: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let p = g.read::<u32>(w(0x04));
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + o, scale.to_bits());
    }
    g.write::<u32>(p, w(0x59));
    let d = g.read::<u32>(g.read::<u32>(w(0x5e)).wrapping_add(0x10));
    let s941 = slot_target(g, w(0x66), 941);
    let list = g.call(s941, 0, &[d]).eax;
    let program = g.read::<u32>(list.wrapping_add(index.wrapping_mul(4)));
    start_part(g, img, img.rel32_target(f + 0x84), obj, 0x1000, program, 1);
    // push owner_b; push ecx (not read); call with `edx` = owner
    reset_transform(g, obj, owner, owner_b);
    append_object(
        g,
        img,
        f,
        img.rel32_target(f + 0x9d),
        obj,
        [0xa8, 0xbb, 0xc1, 0xca, 0xd6, 0xdf, 0xe8],
    );
    obj
}

/// [`start_object_at`].
pub const START_OBJECT_AT: Sig = Sig {
    len: 241,
    hash: 0x0131_a0f5_0281_290a,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x95, Callee::Sig(&RESET_TRANSFORM)),
        (0x9d, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};
