//! Small functions many overlays share outside the node family, ported by
//! the shared-B list. Each doc names the DLL and address it was ported
//! from, its size and how many DLLs carry it.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Sets an actor's motion speed (`ecx` = an index into the DLL's actor
/// byte table, -1 for the effect's own actor; `edx` = the speed). While
/// the DLL's mode word (operand `+0x02`) is 0, an actor number below
/// 0x20 (the default at operand `+0x11` for -1, else the table byte;
/// the table pointer at `+0x0a`) goes to `MsEffectSetSpeed(n, speed)`
/// (slot 530). Otherwise (a negative speed becomes 0x100 in that mode)
/// `Ch_SetMotionSpeed(chr, speed)` (slot 249) with the effect's
/// character (operand `+0x47`) for -1, else the character table's entry
/// (operand `+0x67`) for the table byte. Returns the slot's `eax`.
///
/// `magic_0018` 0x10002bc0 (126 bytes, 158 DLLs).
pub fn set_speed<G: Game>(g: &mut G, img: &Image, f: u32, index: u32, speed: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let bytes = g.read::<u32>(w(0x0a));
    let mut speed = speed;
    if g.read::<u32>(w(0x02)) == 0 {
        let n = if index == u32::MAX {
            g.read::<u32>(w(0x11))
        } else {
            u32::from(g.read::<u8>(bytes.wrapping_add(index)))
        };
        if n.cast_signed() < 0x20 {
            let s530 = slot_target(g, w(0x26), 530);
            return g.call(s530, index, &[n, speed]).eax;
        }
    } else if speed.cast_signed() < 0 {
        speed = 0x100;
    }
    if index == u32::MAX {
        let chr = g.read::<u32>(w(0x47));
        let s249 = slot_target(g, w(0x4e), 249);
        g.call(s249, index, &[chr, speed]).eax
    } else {
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(index)));
        let chr = g.read::<u32>(w(0x67).wrapping_add(4 * b));
        let s249 = slot_target(g, w(0x6d), 249);
        g.call(s249, index, &[chr, speed]).eax
    }
}

/// [`set_speed`].
pub const SET_SPEED: Sig = Sig {
    len: 126,
    hash: 0x3318_36db_eead_6e93,
    calls: &[],
    imports: &[],
};

/// Steps an animation cursor (`ecx`: `+0x0` the key for slot 941, which
/// gives the frame table; `+0x4` playing; `+0x8` the frame, a halfword;
/// `+0xa` the time left in it): while playing, the time drops by 0x200;
/// when it runs out the next frame starts (its time from the table's
/// entry, 8 bytes per frame from `+0x10`, at `+0x2`), wrapping to frame 0
/// past the table's count (`+0x6`) when the last frame's byte `+0x4` has
/// bit 7, else stopping on the last frame. Returns the current frame's
/// entry.
///
/// `magic_0009` 0x10002470 (122 bytes, `ecx` = the cursor, 85 DLLs).
pub fn anim_step<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let s941 = slot_target(g, img.word(f + 0x01), 941);
    let table = g.call(s941, this, &[g.read::<u32>(this)]).eax;
    let frame = |n: u32| {
        let i = i32::from((n as u16).cast_signed())
            .cast_unsigned()
            .wrapping_add(2);
        table.wrapping_add(i.wrapping_mul(8))
    };
    let n = u32::from(g.read::<u16>(this + 8));
    let cur = frame(n);
    if g.read::<u32>(this + 4) == 0 {
        return cur;
    }
    let t = g.read::<u16>(this + 0xa).wrapping_add(0xfe00);
    g.write::<u16>(this + 0xa, t);
    if g.read::<u16>(this + 0xa).cast_signed() > 0 {
        return cur;
    }
    let next = n.wrapping_add(1);
    g.write::<u16>(this + 8, next as u16);
    if (next as u16).cast_signed() >= g.read::<u16>(table.wrapping_add(6)).cast_signed() {
        if g.read::<u8>(cur + 4) & 0x80 == 0 {
            g.write::<u16>(this + 8, n as u16);
            g.write::<u32>(this + 4, 0);
            return cur;
        }
        g.write::<u16>(this + 8, 0);
    }
    let e = frame(u32::from(g.read::<u16>(this + 8)));
    g.write::<u16>(this + 0xa, g.read::<u16>(e + 2));
    e
}

/// [`anim_step`].
pub const ANIM_STEP: Sig = Sig {
    len: 122,
    hash: 0x6047_cfde_e1ff_13f2,
    calls: &[],
    imports: &[],
};

/// Slot 239 `(G, [ctx + 0x120], 0)` (`G` the DLL's word at operand
/// `+0x15`). Returns the slot's `eax`.
///
/// `magic_0009` 0x100038c0 (38 bytes, cdecl `(ctx)`, 76 DLLs; a callback
/// the DLL hands the game).
pub fn call_239<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ctx = fr.arg(g, 0);
    let a = [
        g.read::<u32>(img.word(fr.at + 0x15)),
        g.read::<u32>(ctx + 0x120),
        0,
    ];
    let s = slot_target(g, img.word(fr.at + 0x0f), 239);
    Ret::eax(g.call(s, fr.ecx, &a).eax)
}

/// [`call_239`].
pub const CALL_239: Sig = Sig {
    len: 38,
    hash: 0x5af2_5e4d_5809_4348,
    calls: &[],
    imports: &[],
};
