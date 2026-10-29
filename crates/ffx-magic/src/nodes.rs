//! The node lists of the effect overlays (`magic_0087`'s code, the same bytes
//! in 100 to 116 DLLs; docs/formats/magic.md, "Porting the overlays").
//!
//! A group object keeps the node it visits at `+0`, a flag word at `+4`
//! (the walks set its low byte) and a mode byte at `+5`. The owner node, the
//! group's `+0` at rest, is the parent of a circular list: its first child
//! is at `+0x10`, each child's `+8` is the next (the owner ends the list)
//! and its `+4` the previous one; a child's `+0x14` is its parent. A child's
//! first word is its vtable, `+0x1a` a mode byte, `+0x1c`..`+0x28` four
//! words its parent sums, `+0x5c`..`+0x64` three floats. A parent's `+0x18`
//! (u16) counts its children.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The walk of `0x10001010`, `0x10001090`, `0x100040d0`, `0x100034d0`,
/// `0x10003400`, `0x10003460` and `0x10005e70` (the same bytes): the owner's
/// `+0x1a` against the group's mode `+5`. Equal clears the flag and stops
/// (`None`). Otherwise each child, from the owner's `+0x10` along `+8` back
/// to the owner, is visited through its vtable's method at `+0x1c` with the
/// group as its one argument and `ecx` as the caller left it; eax is the
/// last method's, its low byte then the mode. The group is restored to the
/// owner, the owner's `+0x1a` takes the mode and the flag is set (`Some`).
pub(crate) fn walk<G: Game>(g: &mut G, this: u32, ecx: u32, eax: u32) -> (u32, Option<u32>) {
    let owner = g.read::<u32>(this);
    let mut eax = (eax & !0xff) | u32::from(g.read::<u8>(owner + 0x1a));
    if g.read::<u8>(this + 5) == g.read::<u8>(owner + 0x1a) {
        g.write::<u8>(this + 4, 0);
        return (eax, None);
    }
    let mut cur = g.read::<u32>(owner + 0x10);
    if cur != owner {
        loop {
            g.write::<u32>(this, cur);
            let next = g.read::<u32>(cur + 8);
            let method = g.read::<u32>(g.read::<u32>(cur) + 0x1c);
            eax = g.call(method, ecx, &[this]).eax;
            cur = next;
            if cur == owner {
                break;
            }
        }
    }
    let mode = g.read::<u8>(this + 5);
    g.write::<u32>(this, owner);
    g.write::<u8>(owner + 0x1a, mode);
    g.write::<u8>(this + 4, 1);
    ((eax & !0xff) | u32::from(mode), Some(owner))
}

/// The parent part of [`unlink`] and [`free_then_unlink`] (`0x10001110`'s
/// `+0x14` branch): the parent's count `+0x18` drops by one, the links
/// around the node are joined and the node's own links and `+0x14` are
/// cleared. Returns the node's previous link as it was last read.
pub(crate) fn leave_parent<G: Game>(g: &mut G, node: u32, parent: u32) -> u32 {
    let count = g.read::<u16>(parent + 0x18).wrapping_add(0xffff);
    g.write::<u16>(parent + 0x18, count);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    if prev == parent {
        g.write::<u32>(parent + 0x10, next);
    } else {
        g.write::<u32>(prev + 8, next);
    }
    // the original reads both links again after that store
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    if next == parent {
        g.write::<u32>(parent + 0xc, prev);
    } else {
        g.write::<u32>(next + 4, prev);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node + 0x14, 0);
    prev
}

/// `0x10001010` (105 bytes, thiscall with the group in `ecx`; 116 DLLs):
/// the walk, then the owner's four words `+0x1c`..`+0x28` are added into
/// its parent's (`+0x14`) as integers. eax: the last of those words.
pub fn visit_nodes<G: Game>(g: &mut G, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let parent = g.read::<u32>(owner + 0x14);
    let mut last = eax;
    for off in [0x1c, 0x20, 0x24, 0x28] {
        last = g.read::<u32>(owner + off);
        let sum = g.read::<u32>(parent + off).wrapping_add(last);
        g.write::<u32>(parent + off, sum);
    }
    last
}

/// `push ebp; mov ebp, esp; mov ecx, [ebp+8]; pop ebp; jmp 0x10001010`
/// (12 bytes, 116 DLLs): the thunk the vtables store for [`visit_nodes`].
/// It takes the group from its first stack word into `ecx` and runs the
/// walk with eax as it came.
pub fn thunk_visit<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    Ret::eax(visit_nodes(g, this, fr.eax))
}

/// `0x10001010`.
pub const VISIT: Sig = Sig {
    len: 105,
    hash: 0x697e_c087_45da_f493,
    calls: &[],
    imports: &[],
};

/// `push ebp; mov ebp, esp; mov ecx, [ebp+8]; pop ebp; jmp 0x10001010`.
pub const THUNK_VISIT: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT))],
    imports: &[],
};

/// `0x10001090` (106 bytes, thiscall with the group in `ecx`; 116 DLLs):
/// the walk, then slot 248 (`FFXVu0AddVector`, in the table's global at
/// `f + 0x53`) adds the owner's `+0x1c` vector into the parent's `+0x1c`:
/// `(parent + 0x1c, parent + 0x1c, owner + 0x1c)`. eax: the slot's.
pub fn visit_nodes_vu0<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let parent = g.read::<u32>(owner + 0x14);
    let slot = slot_target(g, img.word(f + 0x53), 248);
    let dst = parent + 0x1c;
    g.call(slot, owner, &[dst, dst, owner + 0x1c]).eax
}

/// `0x10001090`.
pub const VISIT_VU0: Sig = Sig {
    len: 106,
    hash: 0x6347_59e3_69ae_ab84,
    calls: &[],
    imports: &[],
};

/// The thunk over `0x10001090` (12 bytes, 116 DLLs; the same bytes as
/// [`thunk_visit`], the jump's target tells them apart).
pub fn thunk_visit_vu0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    Ret::eax(visit_nodes_vu0(g, img, target, this, fr.eax))
}

/// `push ebp; mov ebp, esp; mov ecx, [ebp+8]; pop ebp; jmp 0x10001090`.
pub const THUNK_VISIT_VU0: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT_VU0))],
    imports: &[],
};

/// `0x100040d0` (104 bytes, thiscall with the group in `ecx`; 116 DLLs):
/// the walk, then slot 268 (in the table's global at `f + 0x53`) is called
/// with the owner's words `+0x2c` and `+0x30` and the address of its
/// `+0x1c`. eax: the slot's.
pub fn visit_slot268<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, eax: u32) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let slot = slot_target(g, img.word(f + 0x53), 268);
    let args = [
        g.read::<u32>(owner + 0x2c),
        g.read::<u32>(owner + 0x30),
        owner + 0x1c,
    ];
    g.call(slot, owner, &args).eax
}

/// `0x100040d0`.
pub const VISIT_268: Sig = Sig {
    len: 104,
    hash: 0x164b_969c_4eb8_0380,
    calls: &[],
    imports: &[],
};

/// The thunk over `0x100040d0` (12 bytes, 116 DLLs).
pub fn thunk_visit_268<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    Ret::eax(visit_slot268(g, img, target, this, fr.eax))
}

/// The thunk over `0x100040d0`.
pub const THUNK_VISIT_268: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT_268))],
    imports: &[],
};

/// `0x10005e70` (157 bytes, thiscall with the group in `ecx`; 116 DLLs):
/// the walk, then, when the flag is set, the vtable methods at `+0x1c` of
/// the owner's nodes `a` (`+0x2c`) and `b` (`+0x30`) are called, each with a
/// pointer to a local word holding its node (the frame word at `esp - 12`,
/// `esp` the entry's stack pointer). Slot 248 (`FFXVu0AddVector`, in the
/// table's global at `f + 0x87`) then adds the owner's `+0x1c` and the two
/// nodes' `+0x1c` vectors: `(owner + 0x1c, a + 0x1c, b + 0x1c)`, with `a` and
/// `b` read again after the calls. The frame byte at `ebp - 3` is written by
/// the original and never read. eax: the slot's.
pub fn visit_children<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    this: u32,
    eax: u32,
) -> u32 {
    let (eax, tail) = walk(g, this, this, eax);
    let Some(owner) = tail else {
        return eax;
    };
    let local = esp - 12;
    let a = g.read::<u32>(owner + 0x2c);
    g.write::<u32>(local, a);
    let method = g.read::<u32>(g.read::<u32>(a) + 0x1c);
    g.call(method, local, &[local]);
    let b = g.read::<u32>(owner + 0x30);
    g.write::<u32>(local, b);
    let method = g.read::<u32>(g.read::<u32>(b) + 0x1c);
    g.call(method, local, &[local]);
    let b = g.read::<u32>(owner + 0x30);
    let a = g.read::<u32>(owner + 0x2c);
    let slot = slot_target(g, img.word(f + 0x87), 248);
    g.call(slot, local, &[owner + 0x1c, a + 0x1c, b + 0x1c]).eax
}

/// `0x10005e70`.
pub const VISIT_CHILDREN: Sig = Sig {
    len: 157,
    hash: 0x11d3_e514_1f3c_d839,
    calls: &[],
    imports: &[],
};

/// The thunk over `0x10005e70` (12 bytes, 116 DLLs). The target's frame is
/// the thunk's own: `esp` is the same at both entries.
pub fn thunk_visit_children<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    Ret::eax(visit_children(g, img, target, fr.esp, this, fr.eax))
}

/// The thunk over `0x10005e70`.
pub const THUNK_VISIT_CHILDREN: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT_CHILDREN))],
    imports: &[],
};

/// `0x100034d0` (73 bytes, cdecl `(group)`; 116 DLLs): the walk alone; eax
/// is the walk's.
pub fn visit_plain<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    Ret::eax(walk(g, group, fr.ecx, fr.eax).0)
}

/// `0x100034d0`.
pub const PLAIN: Sig = Sig {
    len: 73,
    hash: 0xcc3c_cbc8_6d8d_f647,
    calls: &[],
    imports: &[],
};

/// `0x10003400` (88 bytes, cdecl `(group)`; 116 DLLs): the walk, unless the
/// owner's byte `+0x1b` is set while the DLL's global `ds:0x100b0140` (the
/// operand at `f + 0x12`) is not 0. Then eax is the entry's.
pub fn visit_gated<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let owner = g.read::<u32>(group);
    let gate = img.word(fr.at + 0x12);
    if g.read::<u8>(owner + 0x1b) != 0 && g.read::<u32>(gate) != 0 {
        return Ret::eax(fr.eax);
    }
    Ret::eax(walk(g, group, fr.ecx, fr.eax).0)
}

/// `0x10003400`.
pub const GATED: Sig = Sig {
    len: 88,
    hash: 0xea64_0267_945c_7072,
    calls: &[],
    imports: &[],
};

/// `0x10003460` (97 bytes, cdecl `(group)`; 102 DLLs): the walk and, when its
/// flag is set, the owner's float `+0x1c` is added (`addss`, one rounding)
/// into the parent's float `+0x20`; eax is then the owner.
pub fn visit_add_float<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let (eax, tail) = walk(g, group, fr.ecx, fr.eax);
    let Some(owner) = tail else {
        return Ret::eax(eax);
    };
    let parent = g.read::<u32>(owner + 0x14);
    let a = f32::from_bits(g.read::<u32>(owner + 0x1c));
    let b = f32::from_bits(g.read::<u32>(parent + 0x20));
    g.write::<u32>(parent + 0x20, (a + b).to_bits());
    Ret::eax(owner)
}

/// `0x10003460`.
pub const ADD: Sig = Sig {
    len: 97,
    hash: 0xca34_72ae_aa75_c592,
    calls: &[],
    imports: &[],
};

/// `0x10003520` (34 bytes, cdecl `(group)`; 116 DLLs): takes the node the
/// group holds at `+0` out of its list (`+4` the previous link, `+8` the
/// next) and links it to itself. eax: the previous link.
pub fn unlink_list<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let node = g.read::<u32>(group);
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

/// `0x10003520`.
pub const UNLINK_LIST: Sig = Sig {
    len: 34,
    hash: 0x2af0_a74b_bcc3_1fc0,
    calls: &[],
    imports: &[],
};

/// `0x10005f70` (52 bytes, cdecl `(group)`; 116 DLLs): when bit 0 of the
/// byte `+0xc` of the node the group holds at `+0` is set, the node goes
/// into the argument slot, its vtable's method at `+0x1c` is called with a
/// pointer to that slot and the group's flag `+4` becomes 1. Otherwise the
/// flag becomes 0 and eax is the node.
pub fn flag_method_call<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let node = g.read::<u32>(group);
    if (g.read::<u8>(node + 0xc) & 1) == 0 {
        g.write::<u32>(group + 4, 0);
        return Ret::eax(node);
    }
    let slot = fr.esp + 4;
    g.write::<u32>(slot, node);
    let method = g.read::<u32>(g.read::<u32>(node) + 0x1c);
    let eax = g.call(method, slot, &[slot]).eax;
    g.write::<u32>(group + 4, 1);
    Ret::eax(eax)
}

/// `0x10005f70`.
pub const FLAG_CALL: Sig = Sig {
    len: 52,
    hash: 0x874f_c4c4_5b2d_cff0,
    calls: &[],
    imports: &[],
};

/// `0x10005f20` (80 bytes, cdecl `(group)`; 116 DLLs): as
/// [`flag_method_call`] when bit 0 of the node's `+0xc` is set. Otherwise
/// the flag becomes 0 and the node's function pointer `+0x10` is called with
/// a pointer to two words in the frame, the node and its `+0x14`. eax is
/// the call's.
pub fn flag_pointer_call<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let node = g.read::<u32>(group);
    if (g.read::<u8>(node + 0xc) & 1) != 0 {
        let slot = fr.esp + 4;
        g.write::<u32>(slot, node);
        let method = g.read::<u32>(g.read::<u32>(node) + 0x1c);
        let eax = g.call(method, slot, &[slot]).eax;
        g.write::<u32>(group + 4, 1);
        return Ret::eax(eax);
    }
    g.write::<u32>(group + 4, 0);
    // the two words sit at ebp-8 and ebp-4, and ebp is the entry's esp - 4
    let words = fr.esp - 12;
    let parent = g.read::<u32>(node + 0x14);
    g.write::<u32>(words, node);
    g.write::<u32>(words + 4, parent);
    let func = g.read::<u32>(node + 0x10);
    Ret::eax(g.call(func, words, &[words]).eax)
}

/// `0x10005f20`.
pub const FLAG_PTR: Sig = Sig {
    len: 80,
    hash: 0x87e4_8e5a_f5a9_86b8,
    calls: &[],
    imports: &[],
};

/// `0x10003c20` (51 bytes, cdecl `(group)`; 116 DLLs). Unless the node the
/// group holds has bit 2 of its byte `+0xe8`, the node's word `+0x50` is
/// stored through the pointer the DLL's global `ds:0x100b0428` holds (the
/// operand at `f + 0x12`), the argument becomes the node's `+0x60` and the
/// call tail-jumps to slot 713 (`pppAllFreePObject`, through the table's
/// global at `f + 0x24`) with the caller's stack. Otherwise eax is the
/// entry's.
pub fn all_free_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let node = g.read::<u32>(group);
    if (g.read::<u8>(node + 0xe8) & 4) != 0 {
        return Ret::eax(fr.eax);
    }
    let through = g.read::<u32>(img.word(fr.at + 0x12));
    let value = g.read::<u32>(node + 0x50);
    g.write::<u32>(through, value);
    g.write::<u32>(fr.esp + 4, node + 0x60);
    Ret::jump(slot_target(g, img.word(fr.at + 0x24), 713))
}

/// `0x10003c20`.
pub const FREE_TAIL: Sig = Sig {
    len: 51,
    hash: 0xc52f_8de0_a5b2_482d,
    calls: &[],
    imports: &[],
};

/// `0x10003c70` (120 bytes, thiscall with the group in `ecx`; 116 DLLs): the
/// node's word `+0x50` is stored through the pointer the DLL's global
/// `ds:0x100b0428` holds (the operand at `f + 0x01`), slot 713
/// (`pppAllFreePObject`, in the table's global at `f + 0x15`) is called with
/// `node + 0x60`, and then the node leaves its parent's list ([`leave_parent`])
/// when it has one. eax: the slot's, or the node's previous link.
pub fn free_then_unlink<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.ecx;
    let through = g.read::<u32>(img.word(fr.at + 0x01));
    let node = g.read::<u32>(group);
    let value = g.read::<u32>(node + 0x50);
    g.write::<u32>(through, value);
    let slot = slot_target(g, img.word(fr.at + 0x15), 713);
    let eax = g.call(slot, group, &[node + 0x60]).eax;
    let node = g.read::<u32>(group);
    let parent = g.read::<u32>(node + 0x14);
    if parent == 0 {
        return Ret::eax(eax);
    }
    Ret::eax(leave_parent(g, node, parent))
}

/// `0x10003c70`.
pub const FREE_THEN_UNLINK: Sig = Sig {
    len: 120,
    hash: 0xfa41_41c7_799e_35d5,
    calls: &[],
    imports: &[],
};

/// `0x10005df0` (107 bytes, thiscall with the group in `ecx`; 116 DLLs): the
/// node the group holds at `+0` takes the DLL's vtable (the immediate at
/// `f + 8`) and is put first among the children of the parent the group
/// holds at `+4`, when there is one (the parent's count `+0x18` goes up by
/// one). The group's words `+0xc`, `+0x10`, `+0x14` and `+8` are copied to
/// the node's `+0x13c`, `+0x144`, `+0x140` and `+0x128`, and its byte `+0x148`
/// is cleared. eax: the group's `+8`.
pub fn node_insert<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.ecx;
    let n = g.read::<u32>(group);
    let p = g.read::<u32>(group + 4);
    g.write::<u32>(n, img.word(fr.at + 8));
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, p);
    if p != 0 {
        g.write::<u32>(n + 4, p);
        let first = g.read::<u32>(p + 0x10);
        g.write::<u32>(n + 8, first);
        let first = g.read::<u32>(p + 0x10);
        if first == p {
            g.write::<u32>(p + 0xc, n);
        } else {
            g.write::<u32>(first + 4, n);
        }
        let count = g.read::<u16>(p + 0x18).wrapping_add(1);
        g.write::<u16>(p + 0x18, count);
        g.write::<u32>(p + 0x10, n);
    }
    let w = g.read::<u32>(group + 0xc);
    g.write::<u32>(n + 0x13c, w);
    let w = g.read::<u32>(group + 0x10);
    g.write::<u32>(n + 0x144, w);
    let w = g.read::<u32>(group + 0x14);
    g.write::<u32>(n + 0x140, w);
    let eax = g.read::<u32>(group + 8);
    g.write::<u32>(n + 0x128, eax);
    g.write::<u8>(n + 0x148, 0);
    Ret::eax(eax)
}

/// `0x10005df0`.
pub const INSERT: Sig = Sig {
    len: 107,
    hash: 0x3b12_3feb_b4ba_4cfa,
    calls: &[],
    imports: &[],
};

/// `0x10001110` (128 bytes, cdecl `(group)`; 116 DLLs): takes the node the
/// group holds at `+0` out of its parent's child list ([`leave_parent`]),
/// then calls slot 791 (`graphicSetGlobalAmbientColor`, in the table's global
/// at `f + 0x51`) with the node's floats `+0x5c`, `+0x60` and `+0x64`.
/// Returns its eax.
pub fn unlink<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let group = fr.arg(g, 0);
    let node = g.read::<u32>(group);
    let parent = g.read::<u32>(node + 0x14);
    if parent != 0 {
        leave_parent(g, node, parent);
    }
    let args = [
        g.read::<u32>(node + 0x5c),
        g.read::<u32>(node + 0x60),
        g.read::<u32>(node + 0x64),
    ];
    let slot = slot_target(g, img.word(fr.at + 0x51), 791);
    Ret::eax(g.call(slot, node, &args).eax)
}

/// `0x10001110`.
pub const UNLINK: Sig = Sig {
    len: 128,
    hash: 0x7bda_e141_0bde_0af1,
    calls: &[],
    imports: &[],
};
