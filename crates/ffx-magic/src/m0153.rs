//! `magic_0153.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code in [`crate::l3_common`]): constructors of its
//! effect nodes, which link themselves into the DLL's node lists
//! ([`l3_common::attach`]), and the start-up's wait for the effect data.

use crate::l3_common::{self, ListAt};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Libm, Ret, Sig};

/// Sets up a particle node (`ecx` = a set-up record: `+0x0` the node,
/// `+0x4` its list or 0, `+0x10`/`+0x14` two words for the node): the
/// record's `+0x8` = `node + 0x60` and `+0xc` = 0x340080; the node joins
/// the list ([`l3_common::node_init`]); `+0x20` = `node + 0x60`, `+0x1c`
/// = 0x340080, `+0x30` (word) and `+0x32` cleared, `+0x48`/`+0x4c` the
/// record's two words, `+0x58` cleared. Returns `eax`: the record's
/// `+0x14`.
///
/// 0x10001900 (115 bytes, 1 overlay).
pub fn particle_node<G: Game>(g: &mut G, _img: &Image, _f: u32, this: u32) -> u32 {
    let node = g.read::<u32>(this);
    let list = g.read::<u32>(this + 4);
    let data = node.wrapping_add(0x60);
    g.write::<u32>(this + 8, data);
    g.write::<u32>(this + 0xc, 0x0034_0080);
    l3_common::node_init(g, node, list);
    if list != 0 {
        l3_common::attach(g, node, list, ListAt::of(list));
    }
    g.write::<u32>(node + 0x20, data);
    g.write::<u32>(node + 0x1c, 0x0034_0080);
    g.write::<u32>(node + 0x32, 0);
    g.write::<u16>(node + 0x30, 0);
    g.write::<u32>(node + 0x48, g.read::<u32>(this + 0x10));
    let eax = g.read::<u32>(this + 0x14);
    g.write::<u32>(node + 0x4c, eax);
    g.write::<u32>(node + 0x58, 0);
    eax
}

/// [`particle_node`].
pub const PARTICLE_NODE: Sig = Sig {
    len: 115,
    hash: 0xfeb3_2688_cff8_90b8,
    calls: &[],
    imports: &[],
};

/// Constructs a node (`ecx`) of the static list at operand `+0x1b`
/// (sentinel; its last/first/count words at `+0x27`, `+0x3c`, `+0x48`,
/// written through `+0x4e`): vtable `+0x08` first, then the list
/// ([`l3_common::attach`]), `+0x28` = 1.0, `+0x2c` the word at
/// operand `+0x02` (read first), `+0x30` = 2. Returns `eax`: the list's
/// last node as read the second time.
///
/// 0x10007a30 (100 bytes, 1 overlay).
pub fn timed_node<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let edx = g.read::<u32>(w(0x02));
    g.write::<u32>(this, w(0x08));
    l3_common::node_init(g, this, w(0x1b));
    let eax = l3_common::attach(
        g,
        this,
        w(0x22),
        ListAt {
            last: w(0x27),
            last2: w(0x2f),
            sentinel: w(0x34),
            first: w(0x3c),
            count: w(0x48),
            last_w: w(0x4e),
        },
    );
    g.write::<u32>(this + 0x28, 0x3f80_0000);
    g.write::<u32>(this + 0x2c, edx);
    g.write::<u32>(this + 0x30, 2);
    eax
}

/// [`timed_node`].
pub const TIMED_NODE: Sig = Sig {
    len: 100,
    hash: 0x6670_9e24_e05b_e0b9,
    calls: &[],
    imports: &[],
};

/// Destroys a node (`ecx`): leaves its list ([`l3_common::detach`]), then
/// `graphicSetGlobalAmbientColor` (slot 791) with the node's three `f32`
/// at `+0x5c`, `+0x60`, `+0x64` (the colour [`ambient_node`] saved; passed
/// as their bits). Returns the slot's `eax`.
///
/// 0x10007aa0 (119 bytes, 1 overlay).
pub fn node_drop<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    l3_common::detach(g, this);
    let s791 = slot_target(g, w(0x49), 791);
    let args = [
        g.read::<u32>(this + 0x5c),
        g.read::<u32>(this + 0x60),
        g.read::<u32>(this + 0x64),
    ];
    g.call(s791, this, &args).eax
}

/// [`node_drop`].
pub const NODE_DROP: Sig = Sig {
    len: 119,
    hash: 0x9070_bfd7_7ceb_f247,
    calls: &[],
    imports: &[],
};

/// Constructs a node (`ecx`, one stack word `arg`, cdecl): vtable at
/// operand `+0x05`, the static list at `+0x18` ([`l3_common::attach`];
/// last `+0x24`/`+0x2c`, sentinel `+0x31`, first `+0x39`, count `+0x45`,
/// last written through `+0x4e`), `+0x70` the pointer at operand `+0x55`,
/// `+0x6c` = `arg`, byte `+0x74` cleared, `+0x68` = 1. Returns `arg`.
///
/// 0x10007b80 (105 bytes, 1 overlay).
pub fn owner_node<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, arg: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(this, w(0x05));
    l3_common::node_init(g, this, w(0x18));
    l3_common::attach(
        g,
        this,
        w(0x1f),
        ListAt {
            last: w(0x24),
            last2: w(0x2c),
            sentinel: w(0x31),
            first: w(0x39),
            count: w(0x45),
            last_w: w(0x4e),
        },
    );
    g.write::<u32>(this + 0x70, w(0x55));
    g.write::<u32>(this + 0x6c, arg);
    g.write::<u8>(this + 0x74, 0);
    g.write::<u32>(this + 0x68, 1);
    arg
}

/// [`owner_node`].
pub const OWNER_NODE: Sig = Sig {
    len: 105,
    hash: 0x8614_300a_3b5e_d9f9,
    calls: &[],
    imports: &[],
};

/// The overlay table's wait for the effect's data (cdecl, no
/// arguments; state word `G_STATE` at operand `+0x01`): state 0 waits
/// for `op_pray_effect_check(2)` (slot 225), state 1 for
/// `op_oef2_tex_clut_cue_check(2)` (slot 313), each moving to the next
/// state at once when it answers 0 (the loop goes on with the new
/// state); state 2 waits for `op_oef2_sound_cue_check(2)` (slot 549),
/// then calls `MsEffectReady(G_ID, 0)` (slot 226, `G_ID` at `+0x5e`) and
/// sets state 4. From state 2's success on (and in every other state)
/// slot 673 runs (no arguments). A check that answers non-zero ends the
/// call. Returns 0.
///
/// 0x10008920 (135 bytes, 1 overlay).
pub fn e8920<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut state = g.read::<u32>(w(0x01));
    loop {
        match state {
            0 | 1 => {
                let target = if state == 0 {
                    slot_target(g, w(0x1b), 225)
                } else {
                    slot_target(g, w(0x0e), 313)
                };
                if g.call(target, fr.ecx, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                state = g.read::<u32>(w(0x31)).wrapping_add(1);
                g.write::<u32>(w(0x37), state);
            }
            2 => {
                let s549 = slot_target(g, w(0x41), 549);
                if g.call(s549, fr.ecx, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                let id = g.read::<u32>(w(0x5e));
                let s226 = slot_target(g, w(0x58), 226);
                g.call(s226, fr.ecx, &[id, 0]);
                g.write::<u32>(w(0x6f), 4);
                break;
            }
            _ => break,
        }
    }
    let s673 = slot_target(g, w(0x78), 673);
    g.call(s673, fr.ecx, &[]);
    Ret::eax(0)
}

/// 0x10008920 (135 bytes, 1 overlay).
pub const E8920: Sig = Sig {
    len: 135,
    hash: 0x24c9_5c2f_e2b8_36ca,
    calls: &[],
    imports: &[],
};

/// Releases a node's effects (`ecx` = a record whose `+0x0` is the
/// node): leaves its list ([`l3_common::detach`]), then for each of the
/// node's two effect arrays (`+0x24`/`+0x28` the array, `+0x2c`/`+0x2e`
/// its count, a word re-read every step) `graphicVFXDestroy` (slot 779)
/// on every element's handle (`array + 0x10 + 0x34 * i`, passed as a
/// sign-extended 64-bit value) with the game flag `*G_FLAG` (operand
/// `+0x6b`) set to 1 around each call. Returns `node + 0x30` (`eax`, the
/// count pointer after the loop).
///
/// 0x10002080 (201 bytes, 1 overlay).
pub fn release_effects<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(this);
    l3_common::detach(g, node);
    for k in 0..2 {
        let array = g.read::<u32>(node + 0x24 + 4 * k);
        let count = node + 0x2c + 2 * k;
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(count)) {
            let handle = array.wrapping_add(0x10 + 0x34 * i);
            let flag = g.read::<u32>(w(0x6b));
            let saved = g.read::<u32>(flag);
            g.write::<u32>(flag, 1);
            let s779 = slot_target(g, w(0x7c), 779);
            let high = if handle & 0x8000_0000 != 0 {
                u32::MAX
            } else {
                0
            };
            // ecx still holds the flag pointer
            g.call(s779, flag, &[handle, high]);
            let flag = g.read::<u32>(w(0x90));
            g.write::<u32>(flag, saved);
            i += 1;
        }
    }
    node + 0x30
}

/// [`release_effects`].
pub const RELEASE_EFFECTS: Sig = Sig {
    len: 201,
    hash: 0xf562_501b_c297_4845,
    calls: &[],
    imports: &[],
};

/// Constructs an ambient-colour node from a set-up record (`ecx`: `+0x0`
/// the node, `+0x4` its list or 0): the vtable at operand `+0x08`,
/// [`l3_common::node_init`] and [`l3_common::attach`] when there is a
/// list; eight `f32` 128.0 at `+0x1c..+0x38`; the game's ambient colour
/// into `+0x5c`, `+0x60`, `+0x64` (`graphicGetGlobalAmbientColor`, slot
/// 792); `+0x54` = 1. Returns the slot's `eax`.
///
/// 0x10002150 (155 bytes, 1 overlay).
pub fn ambient_node<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(this);
    let list = g.read::<u32>(this + 4);
    g.write::<u32>(node, w(0x08));
    l3_common::node_init(g, node, list);
    if list != 0 {
        l3_common::attach(g, node, list, ListAt::of(list));
    }
    for i in 0..8 {
        g.write::<u32>(node + 0x1c + 4 * i, 0x4300_0000);
    }
    let s792 = slot_target(g, w(0x4b), 792);
    let eax = g
        .call(s792, list, &[node + 0x5c, node + 0x60, node + 0x64])
        .eax;
    g.write::<u32>(node + 0x54, 1);
    eax
}

/// [`ambient_node`].
pub const AMBIENT_NODE: Sig = Sig {
    len: 155,
    hash: 0x417e_1aec_2863_12c3,
    calls: &[],
    imports: &[],
};

/// The angle of the vector `(a, b)` from two `f32` fields of `ecx`
/// (`a = +0x0`, `b = +0x8` at 0x10001350; `a = +0xc`, `b = +0x4` at
/// 0x10001450, whose register roles are swapped and whose operands sit
/// one byte further: `second`), quadrant by quadrant:
/// `a * b > 0`: `atan(b / a)`, plus -pi when `a` is not positive;
/// `a * b < 0`: `+-pi/2 - atan(a / b)` (the sign of `b`); otherwise
/// (a product of 0 or NaN) by the signs: `a > 0` 0, `a` negative or NaN
/// pi, `a == 0` with `b > 0` pi/2, `b` negative or NaN -pi/2, both 0
/// 2pi. `atan` is MSVCR110's `_libm_sse2_atan_precise` on the `f64`
/// quotient, rounded to `f32`. Returns `xmm0` (`eax` is junk: `lahf`'s
/// flags or the import's).
///
/// 0x10001350 (242 bytes) and 0x10001450 (243 bytes), 1 overlay.
pub fn vector_angle<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, second: bool) -> f32 {
    let shift = u32::from(second);
    let k = |g: &G, off: u32| g.read::<f32>(img.word(f + off + shift));
    let (a_at, b_at) = if second { (0xc, 4) } else { (0, 8) };
    let a = g.read::<f32>(this + a_at);
    let b = g.read::<f32>(this + b_at);
    let p = a * b;
    if p > 0.0 {
        let base = if a > 0.0 { 0.0 } else { k(g, 0x2a) };
        let t = g.libm(Libm::Atan, f64::from(b / a)) as f32;
        return t + base;
    }
    if 0.0 > p {
        let base = if b > 0.0 { k(g, 0x60) } else { k(g, 0x6a) };
        let t = g.libm(Libm::Atan, f64::from(a / b)) as f32;
        return base - t;
    }
    // ucomiss + lahf + jnp: taken only when ordered and equal
    if a != 0.0 {
        return if a > 0.0 { 0.0 } else { k(g, 0xaf) };
    }
    if b != 0.0 {
        return if b > 0.0 { k(g, 0xcc) } else { k(g, 0xdb) };
    }
    k(g, 0xea)
}

/// [`vector_angle`] on `+0x0`/`+0x8`.
pub const VECTOR_ANGLE_XZ: Sig = Sig {
    len: 242,
    hash: 0xd6b1_878a_775d_7dd3,
    calls: &[
        (0x3b, Callee::Import("_libm_sse2_atan_precise")),
        (0x7b, Callee::Import("_libm_sse2_atan_precise")),
    ],
    imports: &[],
};

/// [`vector_angle`] on `+0xc`/`+0x4`.
pub const VECTOR_ANGLE_WY: Sig = Sig {
    len: 243,
    hash: 0xf92b_3f20_b678_b5e5,
    calls: &[
        (0x3c, Callee::Import("_libm_sse2_atan_precise")),
        (0x7c, Callee::Import("_libm_sse2_atan_precise")),
    ],
    imports: &[],
};
