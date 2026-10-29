//! Start-up and shutdown entries of OPU-host overlays (the overlay table's
//! `+0x0` and `+0x4`), the variants after `magic_0604`'s
//! ([`crate::m0604`]): the battle mode is a byte global of the DLL (not
//! the control block's word), and the start-up returns what the OPU
//! set-up ([`opu::setup`]) returns, or tail-jumps to slot 816 after it.
//! Ported by list 5 from `magic_0208`, `0261`, `0279` and `0283`;
//! addresses in the comments are those DLLs'.

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Where a start-up variant keeps its operands (offsets in the function).
struct Boot {
    /// `mov eax, ds:G_TABLE` before the allocation (slot 803).
    table: u32,
    /// `push imm32`: the OPU object's size.
    size: u32,
    /// `mov ecx, ds:G_FLAGS` (the battle flags block pointer).
    flags: u32,
    /// `mov esi, ds:G_SLOTS` (the effect slots; `+4` gets the data).
    slots: u32,
    /// `mov ds:G_OBJ, edi`.
    obj: u32,
    /// `test byte ds:G_MODE, 0x10`.
    mode: u32,
    /// `mov eax, ds:G_DATA` (the battle data when the mode bit is set).
    data: u32,
    /// `mov [esi], imm32` / `mov eax, imm32`: the default effect data.
    default: u32,
    /// `push imm32` of an extra slot-173 call `(data, imm)` after the
    /// registration (`magic_0283`).
    extra: Option<u32>,
    /// The `rel32` of the call or jump to the set-up.
    setup: u32,
    /// After the set-up: `None` returns its `eax` (`jmp setup`), `Some`
    /// the table operand of the tail jump to slot 816 (`call setup; jmp
    /// [table + 0xcc0]`).
    then_816: Option<u32>,
}

/// The common body: allocate the object (slot 803, `(size, 0x10)`),
/// store it, pick the effect data (the battle data when the battle flag
/// byte `+0x10` and the mode bit 4 are set), register it (slot 718),
/// look its entry up (slot 719 `(data, 1, 0)`), optionally slot 173,
/// create the OPU object (slot 715 `(1, entry, data, buffer, 0x200, 0,
/// arg)`) and set it up.
fn boot<G: Game>(g: &mut G, img: &Image, fr: &Frame, b: &Boot) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let table = w(b.table);
    let slot803 = slot_target(g, table, 803);
    let buffer = g.call(slot803, fr.ecx, &[w(b.size), 0x10]).eax;
    let flags = g.read::<u32>(w(b.flags));
    let slots = g.read::<u32>(w(b.slots)).wrapping_add(4);
    g.write::<u32>(w(b.obj), buffer);
    let data = if g.read::<u8>(flags + 0x10) != 0 && g.read::<u8>(w(b.mode)) & 0x10 != 0 {
        g.read::<u32>(w(b.data))
    } else {
        w(b.default)
    };
    g.write::<u32>(slots, data);
    let data = g.read::<u32>(slots);
    let slot718 = slot_target(g, table, 718);
    g.call(slot718, fr.ecx, &[data]);
    // the words each call leaves on the stack stay for the next
    let mut stack = vec![data];
    if let Some(imm) = b.extra {
        let slot173 = slot_target(g, table, 173);
        g.call(slot173, fr.ecx, &[data, w(imm), data]);
        stack = vec![data, w(imm), data];
    }
    let slot719 = slot_target(g, table, 719);
    let mut args = vec![data, 1, 0];
    args.extend(&stack);
    // call width: 6 (the words this call leaves on the stack at most)
    let entry = g.call(slot719, fr.ecx, &args).eax;
    let slot715 = slot_target(g, table, 715);
    let mut args = vec![1, entry, data, buffer, 0x200, 0, arg, data, 1, 0];
    args.extend(&stack[..stack.len().min(2)]);
    // call width: 12 (the words this call leaves on the stack at most)
    let obj = g.call(slot715, fr.ecx, &args).eax;
    let end = opu::setup(g, img, img.rel32_target(f + b.setup), obj);
    match b.then_816 {
        None => Ret::eax(end),
        Some(t) => Ret::jump(slot_target(g, w(t), 816)),
    }
}

/// Table `+0x0` (`magic_0208` 0x10001690, cdecl `(arg)`), see [`boot`];
/// returns the set-up's `this + 0x21200`.
pub fn boot_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    boot(
        g,
        img,
        fr,
        &Boot {
            table: 0x04,
            size: 0x13,
            flags: 0x1b,
            slots: 0x21,
            obj: 0x2c,
            mode: 0x3b,
            data: 0x43,
            default: 0x4d,
            extra: None,
            setup: 0x98,
            then_816: None,
        },
    )
}

/// `boot_a` (156 bytes, 94 overlays).
pub const BOOT_A: Sig = Sig {
    len: 156,
    hash: 0x5aa1_1db9_2db8_50b3,
    calls: &[(0x98, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// Table `+0x0` (`magic_0261` 0x10001690): [`boot_a`] with the slots
/// global read first and the default stored before the test (a store
/// the next one overwrites, with no call between).
pub fn boot_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    boot(
        g,
        img,
        fr,
        &Boot {
            table: 0x04,
            size: 0x13,
            flags: 0x21,
            slots: 0x1b,
            obj: 0x29,
            mode: 0x3f,
            data: 0x4c,
            default: 0x33,
            extra: None,
            setup: 0xa4,
            then_816: None,
        },
    )
}

/// `boot_b` (168 bytes, 2 overlays).
pub const BOOT_B: Sig = Sig {
    len: 168,
    hash: 0x5900_5f62_bac8_b943,
    calls: &[(0xa4, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// Table `+0x0` (`magic_0283` 0x10001690): [`boot_a`] with a 0x1f4000
/// byte object and slot 173 `(data, block)` after the registration (the
/// block a pointer into the DLL's data).
pub fn boot_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    boot(
        g,
        img,
        fr,
        &Boot {
            table: 0x04,
            size: 0x13,
            flags: 0x1b,
            slots: 0x21,
            obj: 0x2c,
            mode: 0x3b,
            data: 0x43,
            default: 0x4d,
            extra: Some(0x67),
            setup: 0xab,
            then_816: None,
        },
    )
}

/// `boot_c` (175 bytes, 2 overlays).
pub const BOOT_C: Sig = Sig {
    len: 175,
    hash: 0x5e89_a98e_57b1_f972,
    calls: &[(0xab, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// Table `+0x0` (`magic_0279` 0x10001660): [`boot_a`], then the
/// set-up called and a tail jump to slot 816 with the caller's stack.
/// (Owned by list 3 in the work list; ported here because no port of it
/// existed on list 5's base.)
pub fn boot_d<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    boot(
        g,
        img,
        fr,
        &Boot {
            table: 0x04,
            size: 0x13,
            flags: 0x1b,
            slots: 0x21,
            obj: 0x2c,
            mode: 0x3b,
            data: 0x43,
            default: 0x4d,
            extra: None,
            setup: 0x95,
            then_816: Some(0x9a),
        },
    )
}

/// `boot_d` (169 bytes, 13 overlays).
pub const BOOT_D: Sig = Sig {
    len: 169,
    hash: 0x899b_65ad_801a_2f2d,
    calls: &[(0x95, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// Table `+0x4` (`magic_0208` 0x100010d0): clears the battle flag byte
/// `+0x11`, frees the OPU object's memory (slot 804) when there is one
/// and clears its global. `eax` is slot 804's result, or 0 without an
/// object.
pub fn shutdown<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let flags = g.read::<u32>(w(0x01));
    g.write::<u8>(flags + 0x11, 0);
    let obj = g.read::<u32>(w(0x0a));
    let mut eax = obj;
    if obj != 0 {
        let slot804 = slot_target(g, w(0x14), 804);
        eax = g.call(slot804, fr.ecx, &[obj]).eax;
    }
    g.write::<u32>(w(0x25), 0);
    Ret::eax(eax)
}

/// `shutdown` (46 bytes, 114 overlays).
pub const SHUTDOWN: Sig = Sig {
    len: 46,
    hash: 0x9853_c793_3039_dfed,
    calls: &[],
    imports: &[],
};
