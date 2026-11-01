//! Hosted object starts (agent magic-small-lo): an object's model words
//! reset, a part program started on it, its owners reset and a pool node
//! holding it appended to the DLL's object list. Three copies (`magic_0391`
//! 0x10006620, `magic_0448` 0x10003740, `magic_0444` 0x10001e90) differ in
//! the program's index, the frame and the reset callee (and
//! `magic_0199` 0x10002c40 its owner word); [`Layout`] gives
//! each copy's operand offsets.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A copy's operand offsets and shape.
pub struct Layout {
    /// The program's index in slot 941's table.
    pub index: u32,
    /// `push ebp; mov ebp, esp` first (the stack words then `ebp + 8..`).
    pub framed: bool,
    /// `G_HOST` (its value the immediate 0x55 bytes on), the host global
    /// (`+0x10` of its word goes to slot 941), slot 941's table.
    pub host: (u32, u32, u32),
    /// The `rel32`s of the part start, the reset and the pool take.
    pub calls: (u32, u32, u32),
    /// The reset's `owner_b`: the immediate at this offset, or (0) the
    /// caller's stack word 3.
    pub owner_b: u32,
    /// The list globals: the tail as first read, the head, the count.
    pub list: (u32, u32, u32),
}

/// The copies' layouts.
pub const L0391: Layout = Layout {
    index: 1,
    framed: false,
    host: (0x1, 0x5b, 0x63),
    calls: (0x7e, 0x8e, 0x96),
    owner_b: 0x86,
    list: (0xa1, 0xb4, 0xc3),
};
/// `magic_0448`'s.
pub const L0448: Layout = Layout {
    index: 0,
    framed: false,
    host: (0x1, 0x5b, 0x63),
    calls: (0x7d, 0x8d, 0x95),
    owner_b: 0x85,
    list: (0xa0, 0xb3, 0xc2),
};
/// `magic_0199`'s (`owner_b` the stack word 3).
pub const L0199: Layout = Layout {
    index: 0,
    framed: true,
    host: (0x4, 0x5e, 0x66),
    calls: (0x80, 0x8e, 0x96),
    owner_b: 0,
    list: (0xa1, 0xb4, 0xc3),
};
/// `magic_0444`'s.
pub const L0444: Layout = Layout {
    index: 0,
    framed: true,
    host: (0x4, 0x5e, 0x66),
    calls: (0x80, 0x93, 0x9b),
    owner_b: 0x8b,
    list: (0xa6, 0xb9, 0xc8),
};

/// The reset a copy calls (internal: `ecx` the object, `edx` the owner,
/// the stack `(-, owner_b)`).
pub type Reset<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// The start (internal: `edx` = the object `O`, `xmm3` its scale; the
/// framed copy's stack word 1 the owner): `O + 0x40..0x5c` 0 but `+0x4c`
/// 1.0, `+0x60..0x6c` the scale; `*G_HOST` the immediate; the program
/// `list[index]` of slot 941 `(host[0x10])` started on it
/// ([`start_part`](crate::opu_parts::start_part), 0x1000, 1); the owners
/// reset; a pool node ([`pool_alloc`](crate::l5_objhost::pool_alloc))
/// holding `O` (`+0x8`) appended to the list (head, tail, count up).
/// Returns `O`.
pub fn hosted_start<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    l: &Layout,
    reset: Reset<G>,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.edx;
    let scale = fr.xmm_f32(3).to_bits();
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + o, scale);
    }
    let gh = g.read::<u32>(w(l.host.0));
    g.write::<u32>(gh, w(l.host.0 + 0x55));
    let d = g.read::<u32>(g.read::<u32>(w(l.host.1)) + 0x10);
    let t = slot_target(g, w(l.host.2), 941);
    let table = g.call(t, 0, &[d]).eax;
    let program = g.read::<u32>(table.wrapping_add(4 * l.index));
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + l.calls.0),
        obj,
        0x1000,
        program,
        1,
    );
    // push (ebp,) esi; push owner_b; push ecx; the call
    let base = if l.framed {
        fr.esp.wrapping_sub(8)
    } else {
        fr.esp.wrapping_sub(4)
    };
    let esp = base.wrapping_sub(12);
    g.write::<u32>(esp + 4, 0);
    let owner_b = if l.owner_b == 0 {
        fr.arg(g, 3)
    } else {
        w(l.owner_b)
    };
    g.write::<u32>(esp + 8, owner_b);
    let owner = if l.framed { fr.arg(g, 1) } else { fr.edx };
    let r = Frame {
        at: img.rel32_target(f + l.calls.1),
        esp,
        ecx: obj,
        edx: owner,
        ..*fr
    };
    reset(g, img, &r);
    let node = crate::l5_objhost::pool_alloc(g, img, img.rel32_target(f + l.calls.2));
    if node == 0 {
        return Ret::eax(obj);
    }
    let (tail_at, head_at, count_at) = (w(l.list.0), w(l.list.1), w(l.list.2));
    let tail = g.read::<u32>(tail_at);
    g.write::<u32>(node + 4, 0);
    g.write::<u32>(node, tail);
    if tail == 0 {
        g.write::<u32>(head_at, node);
    } else {
        g.write::<u32>(tail + 4, node);
    }
    g.write::<u32>(tail_at, node);
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(count_at);
    g.write::<u32>(count_at, n.wrapping_add(1));
    Ret::eax(obj)
}

/// `magic_0391`'s copy (reset with owner 3).
pub fn hosted_0391<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    hosted_start(g, img, fr, &L0391, crate::s4_small::reset_owners_3::<G>)
}

/// `magic_0448`'s copy (reset with owner 3).
pub fn hosted_0448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    hosted_start(g, img, fr, &L0448, crate::s4_small::reset_owners_3::<G>)
}

/// `magic_0444`'s copy (reset with the owner and a 30-frame count).
pub fn hosted_0444<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    hosted_start(g, img, fr, &L0444, crate::s4_small::reset_owners_30::<G>)
}

/// `magic_0199`'s copy (the state reset with the stack word as `+0x100`).
pub fn hosted_0199<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let reset: Reset<G> = |g, img, fr| {
        let arg = g.read::<u32>(fr.esp + 8);
        crate::o4_state::reset_state_arg(g, img, fr.at, fr.ecx, arg);
        Ret::eax(0x8000)
    };
    hosted_start(g, img, fr, &L0199, reset)
}

/// [`hosted_0199`] (234 bytes, 1 overlay).
pub const HOSTED_0199: Sig = Sig {
    len: 234,
    hash: 0xf7c4_1193_de3b_8a08,
    calls: &[
        (0x80, Callee::Sig(&crate::opu_parts::START_PART)),
        (
            0x8e,
            Callee::Sig(&crate::o4_state::RESET_STATE_ARG_0199_2B20),
        ),
        (0x96, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`hosted_0391`] (232 bytes, 1 overlay).
pub const HOSTED_0391: Sig = Sig {
    len: 232,
    hash: 0x6ce5_bf1c_ffff_0c34,
    calls: &[
        (0x7e, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8e, Callee::Sig(&crate::s4_small::RESET_OWNERS_3)),
        (0x96, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`hosted_0448`] (231 bytes, 1 overlay).
pub const HOSTED_0448: Sig = Sig {
    len: 231,
    hash: 0xa281_110d_314d_b0b4,
    calls: &[
        (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8d, Callee::Sig(&crate::s4_small::RESET_OWNERS_3)),
        (0x95, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};

/// [`hosted_0444`] (239 bytes, 1 overlay).
pub const HOSTED_0444: Sig = Sig {
    len: 239,
    hash: 0xfbf5_d324_0967_22ae,
    calls: &[
        (0x80, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x93, Callee::Sig(&crate::s4_small::RESET_OWNERS_30)),
        (0x9b, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
