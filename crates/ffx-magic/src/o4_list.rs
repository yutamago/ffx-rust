//! Single functions of the own work list (round 4): a timer that takes
//! a node off its ring and a table cleared and set from the code's
//! immediates.

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `(a)` (`magic_0513` 0x10006d50, 81 bytes): with `s = a[1]`, its timer
/// `s[0x18]`: 0 starts it at 10000 when bit 2 of `s[0x2d8]` is set;
/// 10000 (also just started) takes the node `*a` off its ring (its
/// neighbours linked, `+0x4`/`+0x8` itself) and sets the timer to 20000;
/// in every case `s[0x1c]` counts down. Returns the timer read, or the
/// node's old `+0x4` when it was taken off.
pub fn timer_unlink<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let t = g.read::<u32>(s.wrapping_add(0x18));
    let unlink = if t == 0 {
        let on = g.read::<u8>(s.wrapping_add(0x2d8)) & 4 != 0;
        if on {
            g.write::<u32>(s.wrapping_add(0x18), w(0x1d));
        }
        on
    } else {
        t == w(0x24)
    };
    let mut r = t;
    if unlink {
        let n = g.read::<u32>(a);
        let next = g.read::<u32>(n.wrapping_add(4));
        let prev = g.read::<u32>(n.wrapping_add(8));
        g.write::<u32>(next.wrapping_add(8), prev);
        let prev = g.read::<u32>(n.wrapping_add(8));
        r = g.read::<u32>(n.wrapping_add(4));
        g.write::<u32>(prev.wrapping_add(4), r);
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(n.wrapping_add(4), n);
        g.write::<u32>(s.wrapping_add(0x18), w(0x47));
    }
    let c = g.read::<u32>(s.wrapping_add(0x1c));
    g.write::<u32>(s.wrapping_add(0x1c), c.wrapping_sub(1));
    Ret::eax(r)
}

/// No arguments (`magic_0523` 0x100021b0, 81 bytes): the table `T`
/// (`+0x08`) cleared (`memset(T, 0, 0x110)`), three words set from the
/// immediates (`+0x16`/`+0x1a`, `+0x20`/`+0x24`, `+0x2a`/`+0x2e`), then
/// every 8-byte entry from `+0x33` below `+0x4a` gets the word 1.
/// Returns the end.
pub fn table_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let len = w(0x01) as usize;
    g.write_bytes(w(0x08), &vec![0u8; len]);
    for (a, v) in [(0x16, 0x1a), (0x20, 0x24), (0x2a, 0x2e)] {
        g.write::<u32>(w(a), w(v));
    }
    let mut p = w(0x33);
    loop {
        g.write::<u32>(p, w(0x42));
        p = p.wrapping_add(8);
        if p.cast_signed() >= w(0x4a).cast_signed() {
            break;
        }
    }
    Ret::eax(p)
}

/// `this` in `ecx` (`magic_0273` 0x10007390, 82 bytes): with bit 0 of
/// the flags `this[0x90]`, slot 36 (`+0x18`) with `(this[0x54], K)` and
/// the flags' low half or 0x200 (`+0x1f`); else slot 228 (`+0x34`) with
/// `(this[0x50], K)` (`K` the `push imm8` at `+0x09`); then the low half
/// or 0x120 (`+0x3e`) and `this[0x60]` = the immediate at `+0x4c`.
/// Returns 0x120.
pub fn this_flag_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.ecx;
    let k = crate::o4_parts::disp8(img, f + 0x09);
    let flags = this.wrapping_add(0x90);
    if g.read::<u8>(flags) & 1 != 0 {
        let a = g.read::<u32>(this.wrapping_add(0x54));
        let s = crate::runtime::slot_target(g, w(0x01), w(0x18) / 4);
        g.call(s, this, &[a, k]);
        let v = g.read::<u16>(flags) | w(0x1f) as u16;
        g.write::<u16>(flags, v);
    } else {
        let a = g.read::<u32>(this.wrapping_add(0x50));
        let s = crate::runtime::slot_target(g, w(0x01), w(0x34) / 4);
        g.call(s, this, &[a, k]);
    }
    let v = g.read::<u16>(flags) | w(0x3e) as u16;
    g.write::<u16>(flags, v);
    g.write::<u32>(this.wrapping_add(0x60), w(0x4c));
    Ret::eax(w(0x3e))
}

/// [`timer_unlink`] in `magic_0513` (0x10006d50, 81 bytes).
pub const TIMER_UNLINK_0513_6D50: Sig = Sig {
    len: 81,
    hash: 0x66f1_4b01_6efb_abe5,
    calls: &[],
    imports: &[],
};

/// [`table_reset`] in `magic_0523` (0x100021b0, 81 bytes).
pub const TABLE_RESET_0523_21B0: Sig = Sig {
    len: 81,
    hash: 0x338a_6ce5_cb82_e959,
    calls: &[(0xd, Callee::Import("memset"))],
    imports: &[],
};

/// [`this_flag_step`] in `magic_0273` (0x10007390, 82 bytes).
pub const THIS_FLAG_STEP_0273_7390: Sig = Sig {
    len: 82,
    hash: 0x55ee_1873_3f1e_9899,
    calls: &[],
    imports: &[],
};
