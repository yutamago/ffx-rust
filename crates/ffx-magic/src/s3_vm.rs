//! More opcodes of the `magic_0162` family's effect script
//! ([`crate::sb_vm`]: cdecl `(ctx, ip)`, the next instruction returned),
//! ported by the shared-3 list: the ones that hand an instruction word to
//! the family's actor helpers ([`crate::sa_script`], [`crate::sb_vm`]).
//! Each doc names the address in `magic_0162`, the size and how many DLLs
//! carry the opcode.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig, sa_script, sb_vm};

/// `(ctx, ip)`.
fn args<G: Game>(g: &G, fr: &Frame) -> (u32, u32) {
    (fr.arg(g, 0), fr.arg(g, 1))
}

/// The shape of the three one-word opcodes: `push esi; mov esi, [ebp +
/// 0xc]; mov ecx, [esi + 4]; call X; lea eax, [esi + 8]` (21 bytes).
/// Returns `(the callee, [ip + 4], the callee's esp, ip + 8)`.
fn one_word<G: Game>(g: &G, img: &Image, fr: &Frame) -> (u32, u32, u32, u32) {
    let (_, ip) = args(g, fr);
    let target = img.rel32_target(fr.at + 0x0b);
    // push ebp, push esi, the call's return address
    (
        target,
        g.read::<u32>(ip + 4),
        fr.esp.wrapping_sub(12),
        ip + 8,
    )
}

/// Every actor of the party list through [`sa_script::each_actor`] with
/// `[ip + 4]`. `0x10006ff0` (21 bytes, 79 DLLs).
pub fn op_each_actor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, v, esp, next) = one_word(g, img, fr);
    sa_script::each_actor(g, img, f, v, esp);
    Ret::eax(next)
}

/// [`op_each_actor`].
pub const OP_EACH_ACTOR: Sig = Sig {
    len: 21,
    hash: 0x7830_99f4_1d75_a6b5,
    calls: &[(0x0b, Callee::Sig(&sa_script::EACH_ACTOR))],
    imports: &[],
};

/// The other actors shown or hidden ([`sb_vm::show_others`]) with
/// `[ip + 4]`. `0x10007780` (21 bytes, 79 DLLs).
pub fn op_show_others<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, v, esp, next) = one_word(g, img, fr);
    sb_vm::show_others(g, img, f, esp, v);
    Ret::eax(next)
}

/// [`op_show_others`].
pub const OP_SHOW_OTHERS: Sig = Sig {
    len: 21,
    hash: 0x7830_99f4_1d75_a6b5,
    calls: &[(0x0b, Callee::Sig(&sb_vm::SHOW_OTHERS))],
    imports: &[],
};

/// Every actor of the DLL's list shown or hidden
/// ([`sa_script::show_all`]) with `[ip + 4]`. `0x10006fd0` (21 bytes, 79
/// DLLs).
pub fn op_show_all<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, v, _, next) = one_word(g, img, fr);
    sa_script::show_all(g, img, f, v, fr.eax);
    Ret::eax(next)
}

/// [`op_show_all`].
pub const OP_SHOW_ALL: Sig = Sig {
    len: 21,
    hash: 0x7830_99f4_1d75_a6b5,
    calls: &[(0x0b, Callee::Sig(&sa_script::SHOW_ALL))],
    imports: &[],
};

/// The shape of the two record opcodes: `ecx` = `i16 [ip + 2]`, `edx` =
/// `[ip + 4]`, `[ip + 8]` pushed, a call (31 bytes). Writes the pushed
/// word where the callee reads it and returns `(the callee's frame, ip +
/// 0xc)`.
fn record_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> (Frame, u32) {
    let (_, ip) = args(g, fr);
    let index = i32::from(g.read::<i16>(ip + 2)).cast_unsigned();
    let word = g.read::<u32>(ip + 8);
    // push ebp, push esi, push [esi + 8], the call's return address
    let esp = fr.esp.wrapping_sub(16);
    g.write::<u32>(esp + 4, word);
    let callee = Frame {
        at: img.rel32_target(fr.at + 0x12),
        esp,
        ecx: index,
        edx: g.read::<u32>(ip + 4),
        ..*fr
    };
    (callee, ip + 0xc)
}

/// An actor record set up ([`sb_vm::actor_record`]: actor `i16 [ip + 2]`,
/// the record `[ip + 4]`, the owner `[ip + 8]`). `0x10006dd0` (31 bytes,
/// 79 DLLs).
pub fn op_actor_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (c, next) = record_call(g, img, fr);
    sb_vm::actor_record(g, img, c.at, c.esp, c.ecx, c.edx);
    Ret::eax(next)
}

/// [`op_actor_record`].
pub const OP_ACTOR_RECORD: Sig = Sig {
    len: 31,
    hash: 0x7e8e_0ce7_edbe_dd0d,
    calls: &[(0x12, Callee::Sig(&sb_vm::ACTOR_RECORD))],
    imports: &[],
};

/// An actor block filled ([`sa_script::actor_block`]: actor `i16 [ip +
/// 2]`, the block `[ip + 4]`, its first word `[ip + 8]`). `0x10006df0`
/// (31 bytes, 79 DLLs).
pub fn op_actor_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (c, next) = record_call(g, img, fr);
    sa_script::actor_block(g, img, &c);
    Ret::eax(next)
}

/// [`op_actor_block`].
pub const OP_ACTOR_BLOCK: Sig = Sig {
    len: 31,
    hash: 0x7e8e_0ce7_edbe_dd0d,
    calls: &[(0x12, Callee::Sig(&sa_script::ACTOR_BLOCK))],
    imports: &[],
};

/// Slot 520 (no arguments; the battle's current something, looked up in
/// the DLL's `-1`-terminated word list at operand `+0x17`): when the list
/// does not hold it, every party actor through [`sa_script::each_actor`]
/// with 0. Returns `ip + 4`. `0x100079c0` (62 bytes, 79 DLLs).
pub fn op_each_unless_listed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let s = slot_target(g, w(0x04), 520);
    let v = g.call(s, fr.ecx, &[]).eax;
    let mut at = w(0x17);
    let listed = loop {
        let x = g.read::<u32>(at);
        if x == u32::MAX {
            break false;
        }
        if x == v {
            break true;
        }
        at += 4;
    };
    if !listed {
        // push ebp, the call's return address
        let f = img.rel32_target(fr.at + 0x32);
        sa_script::each_actor(g, img, f, 0, fr.esp.wrapping_sub(8));
    }
    Ret::eax(fr.arg(g, 1).wrapping_add(4))
}

/// [`op_each_unless_listed`].
pub const OP_EACH_UNLESS_LISTED: Sig = Sig {
    len: 62,
    hash: 0xd658_f64e_e913_b713,
    calls: &[(0x32, Callee::Sig(&sa_script::EACH_ACTOR))],
    imports: &[],
};
