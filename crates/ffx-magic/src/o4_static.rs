//! Small set-ups of the own work list (round 4): words of the DLL's
//! records set from the instructions' immediates (some with a pool node
//! on the task list), and slot calls that set an object's words.

use crate::o4_small::append_node;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `n` `mov dword [addr], imm32` (10 bytes each) from `at`.
fn stores(g: &mut impl crate::Mem, img: &Image, at: u32, n: u32) {
    for k in 0..n {
        let insn = at + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
}

/// No arguments (`magic_0303` 0x10001730, 101 bytes): ten words set
/// (`mov dword [addr], imm32`, 10 bytes each). `eax` as given.
pub fn ten_stores<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    stores(g, img, fr.at, 10);
    Ret::eax(fr.eax)
}

/// No arguments (`magic_0303` 0x100048d0, 105 bytes): two words of a
/// static record set (from `+0x07`), its halfword at `+0x1d` 0; a pool
/// node taken ([`take`], `+0x22`, the owner the operand at `+0x03`), its
/// byte `+0x18` 1, `+0x14` the record (`+0x34`), `+0x4` 0, appended to
/// the list ([`append_node`]: head `+0x28`/`+0x5c`, tail `+0x43`,
/// `+0x4d`, `+0x61`). Returns the node.
pub fn record_two_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    stores(g, img, f + 0x07, 2);
    g.write::<u16>(w(0x1d), 0);
    let node = take(g, img, img.rel32_target(f + 0x22), w(0x03));
    let empty = g.read::<u32>(w(0x28)) == 0;
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), w(0x34));
    g.write::<u32>(node.wrapping_add(4), 0);
    append_node(g, node, empty, (w(0x43), w(0x4d)), (w(0x5c), w(0x61)));
    Ret::eax(node)
}

/// `(obj)` (`magic_0316` 0x100016b0, 103 bytes): while the word `A`
/// (`+0x05`) is 0 and the count `B` (`+0x0d`) is below 0x20, slot 541
/// (`+0x28`) with `(B, 0, K)`; else slot 38 (`+0x37`) with `(*C, 0, K)`
/// (`C` at `+0x45`; `K` at `+0x17`/`+0x34`); then two words of `obj` set
/// (`+0x53`/`+0x57`, `+0x5d`/`+0x61`). Returns `obj`.
pub fn slot_by_flag_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    let b = g.read::<u32>(w(0x0d));
    if g.read::<u32>(w(0x05)) == 0 && b.cast_signed() < 0x20 {
        let s = slot_target(g, w(0x22), w(0x28) / 4);
        g.call(s, fr.ecx, &[b, 0, d(0x17)]);
    } else {
        let c = g.read::<u32>(w(0x45));
        let s = slot_target(g, w(0x2f), w(0x37) / 4);
        g.call(s, fr.ecx, &[c, 0, d(0x34)]);
    }
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj.wrapping_add(w(0x53)), w(0x57));
    g.write::<u32>(obj.wrapping_add(w(0x5d)), w(0x61));
    Ret::eax(obj)
}

/// `(obj)` (`magic_0380` 0x10001a20, 105 bytes, a security-cookie
/// frame): slot 237 (`+0x2e`) with `(*H, K, &q, obj + 0xd0)` (`H` at
/// `+0x28`, `K` at `+0x25`; `q` a local at `ebp - 0x14`, its four words
/// the immediates at `+0x35`, `+0x3c`, `+0x43`, `+0x4a`), then the float
/// at `obj + 0xdc` = the immediate at `+0x59`. Returns the slot's `eax`.
pub fn shape_local<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let v = obj.wrapping_add(0xd0);
    // ebp = esp - 4; q at ebp - 0x14
    let q = fr.esp.wrapping_sub(0x18);
    let h = g.read::<u32>(w(0x28));
    for (k, imm) in (0u32..).zip([0x35, 0x3c, 0x43, 0x4a]) {
        g.write::<u32>(q + 4 * k, w(imm));
    }
    let s = slot_target(g, w(0x20), w(0x2e) / 4);
    let r = g
        .call(s, fr.ecx, &[h, crate::o4_parts::disp8(img, f + 0x25), q, v])
        .eax;
    g.write::<u32>(v.wrapping_add(0xc), w(0x59));
    Ret::eax(r)
}

/// [`ten_stores`] in `magic_0303` (0x10001730, 101 bytes).
pub const TEN_STORES_0303_1730: Sig = Sig {
    len: 101,
    hash: 0xda89_fcbb_7b26_6059,
    calls: &[],
    imports: &[],
};

/// [`record_two_node`] in `magic_0303` (0x100048d0, 105 bytes).
pub const RECORD_TWO_NODE_0303_48D0: Sig = Sig {
    len: 105,
    hash: 0xdbeb_04fe_647e_e33e,
    calls: &[(0x22, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`slot_by_flag_obj`] in `magic_0316` (0x100016b0, 103 bytes).
pub const SLOT_BY_FLAG_OBJ_0316_16B0: Sig = Sig {
    len: 103,
    hash: 0x1100_e334_71b1_512b,
    calls: &[],
    imports: &[],
};

/// [`shape_local`] in `magic_0380` (0x10001a20, 105 bytes).
pub const SHAPE_LOCAL_0380_1A20: Sig = Sig {
    len: 105,
    hash: 0x5d3f_d13e_8917_db8b,
    calls: &[(0x61, Callee::Any)],
    imports: &[],
};
