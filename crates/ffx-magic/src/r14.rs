//! Round 14's own ports: functions whose closest ported kin differs in
//! its control flow or stack layout, written out for their DLL.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0384`'s launches (0x10001b60, an overlay table entry, `(arg)`;
/// an 8-aligned frame of 0x1c bytes, 454 bytes): as `magic_0421`'s
/// [`launches_p055`](crate::r11_misc::launches_p055), with the first
/// object's callback by the count (`CB1` `+0x6c`, or `CB1b` `+0x73` while
/// `*N` is below the imm8 `+0x62`, 0x19), slot 482's words popped before
/// it, and the bound above which the argument's word gets bit 2 0x32 (the
/// imm8 `+0x1b8`). The first launch's word under its call is `ecx` as
/// slot 434 left it (`push ecx`; nothing reads it): the port writes 0
/// there.
pub fn launches_0384<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let imm8 = |o: u32| i32::from((w(o) as u8).cast_signed());
    // push ebp; and esp, -8; sub esp, 0x1c
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let n = g.read::<u32>(w(0x0d));
    let q = div(cvt(n), g.read::<f32>(w(0x1d)));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[
        1,
        q.to_bits(),
        g.read::<u32>(w(0x37)),
        g.read::<u32>(w(0x45)),
    ]);
    // call width: 12 (Pushes caps the words at 12)
    let level = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x15), 0x788 / 4), 0).st0);
    g.write::<f32>(base, level);
    let cb = if g.read::<i32>(w(0x5e)) < imm8(0x62) {
        w(0x73)
    } else {
        w(0x6c)
    };
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[cb, g.read::<u32>(w(0x79))]);
    // call width: 12 (Pushes caps the words at 12)
    let first = s.call(g, slot_target(g, w(0x55), 0x6c8 / 4), 0).eax;
    let launch = |g: &mut G, obj: u32, edx: u32, call: u32, under: u32| {
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: base.wrapping_sub(0xc),
            ecx: obj,
            edx,
            xmm: [0, u128::from(level.to_bits()), 0, 0],
            ..*fr
        };
        g.write::<u32>(base.wrapping_sub(8), under);
        crate::o6_fam_launch::random_launch(g, img, fr2.at, &fr2);
    };
    launch(g, first, 0, 0x90, 0);
    // test byte [N], 0xf (the count read again)
    if g.read::<u8>(w(0x99)).trailing_zeros() >= 4 {
        let fields = [0xd0u32, 0xd4, 0xd8, 0xe0, 0xe4, 0xe8];
        let v: [u32; 6] = core::array::from_fn(|i| g.read::<u32>(first + fields[i]));
        let t = g.read::<u32>(w(0xad));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[w(0xfe), g.read::<u32>(w(0x104))]);
        // call width: 12 (Pushes caps the words at 12)
        let second = s.call(g, g.read::<u32>(t + 0x6c8), 0).eax;
        for (o, x) in fields.into_iter().zip(v) {
            g.write::<u32>(second + o, x);
        }
        g.write::<u32>(second + 0xdc, 0);
        g.write::<u32>(second + 0xec, 0);
    }
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x181), g.read::<u32>(w(0x187))]);
    // call width: 12 (Pushes caps the words at 12)
    let third = s.call(g, slot_target(g, w(0x17c), 0x6c8 / 4), 0).eax;
    launch(g, third, 1, 0x1a4, w(0x181));
    let count = g.read::<u32>(w(0x1a9)).wrapping_add(1);
    g.write::<u32>(w(0x1b2), count);
    if count.cast_signed() > imm8(0x1b8) {
        let arg = fr.arg(g, 0);
        let v = g.read::<u32>(arg);
        g.write::<u32>(arg, v | 4);
        return Ret::eax(arg);
    }
    Ret::eax(count)
}

/// [`launches_0384`] (the launch's `Sig`: `magic_0384`'s).
pub const LAUNCHES_0384: Sig = Sig {
    len: 454,
    hash: 0xa491_bc0c_96cf_584a,
    calls: &[
        (
            0x90,
            Callee::Sig(&crate::o6_fam_launch::RANDOM_LAUNCH_0384_15E0),
        ),
        (
            0x1a4,
            Callee::Sig(&crate::o6_fam_launch::RANDOM_LAUNCH_0384_15E0),
        ),
    ],
    imports: &[],
};

/// `magic_0146`'s summon particle start (0x10007a10, a table entry, cdecl
/// `(actor)`; 450 bytes): [`summon_ppp_start`](crate::s5_misc::summon_ppp_start)'s
/// head, then its record and ring links with the earlier calls' words
/// still on the stack (the operands to the ring's tail three bytes
/// earlier, those after slot 575's table load two bytes later), `G_DONE
/// = 0` and slot 575 (`()`, after the stack is popped). Returns 0.
pub fn summon_ppp_start_0146<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    crate::s5_misc::summon_ppp_head(g, img, fr);
    let tail = g.read::<u32>(w(0x15a));
    g.write::<u32>(w(0x15f), tail);
    for (dst, v) in [
        (0x165u32, 0x169u32),
        (0x16f, 0x173),
        (0x179, 0x17d),
        (0x183, 0x187),
    ] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(tail.wrapping_add(4), w(0x18e));
    let t = g.read::<u32>(w(0x193));
    let flags = g.read::<u32>(w(0x199)) & 0xffff_fffe;
    g.write::<u32>(w(0x199), flags);
    g.write::<u32>(w(0x1a0), w(0x1a4));
    g.write::<u32>(w(0x1aa), 0);
    // call width: 0 (`add esp, 0x4c` before the call)
    g.call(g.read::<u32>(t + 0x8fc), 0, &[]);
    Ret::eax(0)
}

/// [`summon_ppp_start_0146`] (the `rel32`: the targets).
pub const SUMMON_PPP_START_0146: Sig = Sig {
    len: 450,
    hash: 0xbfc4_bb18_c6b0_54cf,
    calls: &[(0x8, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};
