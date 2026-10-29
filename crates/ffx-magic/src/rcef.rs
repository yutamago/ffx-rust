//! The map effects' task functions (docs/formats/magic.md, "Porting the
//! overlays"): the `rcef*` calls of `g_MagicImports` (slots 433 to 473:
//! `rcefEffectReady`, `rcefMainInit`, `rcefMainCreateTask`, `rcefTaskSetSeq`,
//! `rcefTaskCallSeq`, `rcefTaskWait`, ...) that start and wait on an effect's
//! tasks. Each function is carried by several DLLs, so each lives here, named
//! after the DLL and address it was ported from; the carrier count is the
//! number of overlays whose code binds to it (`magic-sig --bind`). A copy's
//! table global, callbacks and game globals are read from its own operands:
//! the code is the same, the addresses are not.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// `magic_0334` 0x10001000 (50 bytes; 18 DLLs carry it): `rcefTaskSetSeq(arg,
/// cb)` (slot 450) and `rcefTaskCallSeq(arg, cb2)` (slot 449), the callbacks
/// the copy's operands name; returns slot 449's eax.
pub fn set_seq_call_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 9)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0x1e)]);
    Ret::eax(r.eax)
}

/// [`set_seq_call_seq`]'s code identity.
pub const SET_SEQ_CALL_SEQ: Sig = Sig {
    len: 50,
    hash: 0xaeab_c4a2_d86d_c2c0,
    calls: &[],
    imports: &[],
};

/// `magic_0334` 0x10001280 (94 bytes; 33 DLLs carry it): `rcefMainInit` (slot
/// 436) with nine words, first to last: the game word at `ds:G2` (read
/// first), the argument, the copy's operands `0x10004840`, `0x10105b80` and
/// `0x10005b80` (its `0x10185b80` less `0x180000`), `0x80000`, 1, its
/// `0x1000483c` and `0x10085b40`; then `rcefMainCreateTask(V, cb)` (slot
/// 434) with `V` the game word at `ds:G2`, read again. Returns 0.
pub fn main_init_create_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 0x1b);
    let arg = fr.arg(g, 0);
    let var = img.word(f + 0x39);
    let v1 = g.read::<u32>(var);
    let slot436 = slot_target(g, table, 436);
    // mov eax, 0x10185b80; sub eax, 0x180000
    let slot436_arg5 = img.word(f + 0x10).wrapping_sub(0x0018_0000);
    g.call(
        slot436,
        fr.ecx,
        &[
            v1,
            arg,
            img.word(f + 0x30),
            img.word(f + 0x2b),
            0x0008_0000,
            slot436_arg5,
            1,
            img.word(f + 9),
            img.word(f + 4),
        ],
    );
    let v2 = g.read::<u32>(var);
    let slot434 = slot_target(g, table, 434);
    g.call(slot434, fr.ecx, &[v2, img.word(f + 0x45)]);
    Ret::eax(0)
}

/// [`main_init_create_task`]'s code identity.
pub const MAIN_INIT_CREATE_TASK: Sig = Sig {
    len: 94,
    hash: 0x7678_bf50_59c4_f7d9,
    calls: &[],
    imports: &[],
};

/// `magic_0335` 0x10001090 (94 bytes; 2 DLLs carry it, with `magic_0362`): slot
/// 473 with `(0x30, 0, 0x30, 0x40, 0xf)`, `rcefMainCreateTask(V, cb)` (slot
/// 434) with `V` the game word at `ds:G2`, `rcefTaskSetSeq(arg, cb2)` (slot
/// 450) and `rcefTaskWait(arg, 0x1e)` (slot 451); returns slot 451's eax.
pub fn create_task_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let slot473 = slot_target(g, table, 473);
    g.call(slot473, fr.ecx, &[0x30, 0, 0x30, 0x40, 0xf]);
    let v = g.read::<u32>(img.word(f + 0x26));
    let slot434 = slot_target(g, table, 434);
    g.call(slot434, fr.ecx, &[v, img.word(f + 0x20)]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x38)]);
    let slot451 = slot_target(g, table, 451);
    let r = g.call(slot451, fr.ecx, &[arg, 0x1e]);
    Ret::eax(r.eax)
}

/// [`create_task_pair`]'s code identity.
pub const CREATE_TASK_PAIR: Sig = Sig {
    len: 94,
    hash: 0x0b64_67f3_acd9_94b8,
    calls: &[],
    imports: &[],
};

/// `magic_0336` 0x10001170 (213 bytes; 2 DLLs carry it, with `magic_0363`):
/// `rcefObjAttachChrOpt(arg, [p+0x1a0], 6)` (slot 442) with `p` the game
/// pointer at `ds:G2`; `rcefObjSetMode(arg, 0x40)` (443); the words `0`,
/// `0xc0000000`, `0`, `0` at `+0xd0`..`+0xdc` and `0.06f` (`0x3d75c28f`)
/// three times and 0 at `+0x130`..`+0x13c`; `rcefPlaySE(0)` (444);
/// `rcefMainSetPar(p, arg, 0, 0)` (440) with `p` read again; `rcefTaskSetSeq(
/// arg, cb)` (450) and `rcefTaskCallSeq(arg, cb2)` (449); returns slot 449's
/// eax.
pub fn obj_attach_set_par_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let var = img.word(f + 4);
    let table = img.word(f + 0x15);
    let arg = fr.arg(g, 0);
    let p = g.read::<u32>(var);
    let v = g.read::<u32>(p.wrapping_add(0x1a0));
    let slot442 = slot_target(g, table, 442);
    g.call(slot442, fr.ecx, &[arg, v, 6]);
    let slot443 = slot_target(g, table, 443);
    g.call(slot443, fr.ecx, &[arg, 0x40]);
    g.write::<u32>(arg + 0xd0, 0);
    g.write::<u32>(arg + 0xd4, 0xc000_0000);
    g.write::<u32>(arg + 0xd8, 0);
    g.write::<u32>(arg + 0xdc, 0);
    g.write::<u32>(arg + 0x130, 0x3d75_c28f);
    g.write::<u32>(arg + 0x134, 0x3d75_c28f);
    g.write::<u32>(arg + 0x138, 0x3d75_c28f);
    g.write::<u32>(arg + 0x13c, 0);
    let slot444 = slot_target(g, table, 444);
    g.call(slot444, fr.ecx, &[0]);
    let slot440 = slot_target(g, table, 440);
    let p2 = g.read::<u32>(var);
    g.call(slot440, fr.ecx, &[p2, arg, 0, 0]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0xaf)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0xc2)]);
    Ret::eax(r.eax)
}

/// [`obj_attach_set_par_c`]'s code identity.
pub const OBJ_ATTACH_SET_PAR_C: Sig = Sig {
    len: 213,
    hash: 0x15a7_74bb_22d7_f88a,
    calls: &[],
    imports: &[],
};

/// `magic_0336` 0x10001090 (71 bytes; 3 DLLs carry it, with `magic_0193` and
/// `magic_0363`): `rcefMainCreateTask(V, cb)` (slot 434) with `V` the game word
/// at `ds:G2`, `rcefTaskSetSeq(arg, cb2)` (450) and `rcefTaskWait(arg, 0x1e)`
/// (451); returns slot 451's eax. Seven more copies wait other times (the
/// `push imm8` at `+0x35`; round 4, [`crate::o4_rcef`]'s signatures).
pub fn create_task_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let v = g.read::<u32>(img.word(f + 0xf));
    let slot434 = slot_target(g, table, 434);
    g.call(slot434, fr.ecx, &[v, img.word(f + 9)]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x21)]);
    let slot451 = slot_target(g, table, 451);
    // push imm8
    let time = crate::o4_parts::disp8(img, f + 0x36);
    let r = g.call(slot451, fr.ecx, &[arg, time]);
    Ret::eax(r.eax)
}

/// [`create_task_wait`]'s code identity.
pub const CREATE_TASK_WAIT: Sig = Sig {
    len: 71,
    hash: 0xb58b_4635_e533_bb2f,
    calls: &[],
    imports: &[],
};

/// `magic_0336` 0x100010e0 (76 bytes; 10 DLLs carry it): `rcefSetDamage([p+
/// 0x1a0], 0)` (slot 446) with `p` the game pointer at `ds:G2`;
/// `rcefTaskSetSeq(arg, cb)` (450) and `rcefTaskCallSeq(arg, cb2)` (449);
/// returns slot 449's eax.
pub fn damage_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let var = img.word(f + 4);
    let table = img.word(f + 0x11);
    let arg = fr.arg(g, 0);
    let p = g.read::<u32>(var);
    let v = g.read::<u32>(p.wrapping_add(0x1a0));
    let slot446 = slot_target(g, table, 446);
    g.call(slot446, fr.ecx, &[v, 0]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x23)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0x38)]);
    Ret::eax(r.eax)
}

/// [`damage_seq`]'s code identity.
pub const DAMAGE_SEQ: Sig = Sig {
    len: 76,
    hash: 0xf67f_fc7f_4183_9cde,
    calls: &[],
    imports: &[],
};

/// `magic_0337` 0x100011c0 (213 bytes; 2 DLLs carry it, with `magic_0364`):
/// `rcefPlaySE(0)` (slot 444) first; `rcefObjAttachChrOpt(arg, [p+0x1a0], 6)`
/// (442) with `p` the game pointer at `ds:G2`; `rcefObjSetMode(arg, 0x40)`
/// (443); the words `0`, `0xc0000000`, `0`, `0` at `+0xd0`..`+0xdc`, `0.08f`
/// (`0x3da3d70a`) three times and 0 at `+0x130`..`+0x13c`;
/// `rcefMainSetPar(p, arg, 0, 0)` (440) with `p` read again;
/// `rcefTaskSetSeq(arg, cb)` (450) and `rcefTaskCallSeq(arg, cb2)` (449);
/// returns slot 449's eax.
pub fn obj_attach_set_par_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let var = img.word(f + 0x14);
    let slot444 = slot_target(g, table, 444);
    g.call(slot444, fr.ecx, &[0]);
    let arg = fr.arg(g, 0);
    let p = g.read::<u32>(var);
    let v = g.read::<u32>(p.wrapping_add(0x1a0));
    let slot442 = slot_target(g, table, 442);
    g.call(slot442, fr.ecx, &[arg, v, 6]);
    let slot443 = slot_target(g, table, 443);
    g.call(slot443, fr.ecx, &[arg, 0x40]);
    let p2 = g.read::<u32>(var);
    g.write::<u32>(arg + 0xd0, 0);
    g.write::<u32>(arg + 0xd4, 0xc000_0000);
    g.write::<u32>(arg + 0xd8, 0);
    g.write::<u32>(arg + 0xdc, 0);
    g.write::<u32>(arg + 0x130, 0x3da3_d70a);
    g.write::<u32>(arg + 0x134, 0x3da3_d70a);
    g.write::<u32>(arg + 0x138, 0x3da3_d70a);
    g.write::<u32>(arg + 0x13c, 0);
    let slot440 = slot_target(g, table, 440);
    g.call(slot440, fr.ecx, &[p2, arg, 0, 0]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0xaf)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0xc2)]);
    Ret::eax(r.eax)
}

/// [`obj_attach_set_par_b`]'s code identity.
pub const OBJ_ATTACH_SET_PAR_B: Sig = Sig {
    len: 213,
    hash: 0x170d_4dbd_89ac_8ade,
    calls: &[],
    imports: &[],
};

/// `magic_0339` 0x10001140 (63 bytes; 9 DLLs carry it): slot 472 with no
/// words, then `rcefTaskSetSeq(arg, cb)` (slot 450) and `rcefTaskCallSeq(arg,
/// cb2)` (449); returns slot 449's eax.
pub fn task_seq_no_args<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let slot472 = slot_target(g, table, 472);
    g.call(slot472, fr.ecx, &[]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x16)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0x2b)]);
    Ret::eax(r.eax)
}

/// [`task_seq_no_args`]'s code identity.
pub const TASK_SEQ_NO_ARGS: Sig = Sig {
    len: 63,
    hash: 0xe380_238d_b4cf_c294,
    calls: &[],
    imports: &[],
};

/// `magic_0339` 0x100011c0 (173 bytes; 4 DLLs carry it, with `magic_0335`,
/// `magic_0362` and `magic_0366`): `rcefObjAttachChrOpt(arg, [p+0x1a0], 3)` (slot
/// 442) with `p` the game pointer at `ds:G2`; `rcefObjSetMode(arg, 0x40)`
/// (443); `0.1f` (`0x3dcccccd`) three times and 0 at `+0x130`..`+0x13c`;
/// `rcefPlaySE(0)` (444); `rcefMainSetPar(p, arg, 0, 0)` (440) with `p` read
/// again; `rcefTaskSetSeq(arg, cb)` (450) and `rcefTaskCallSeq(arg, cb2)`
/// (449); returns slot 449's eax.
pub fn obj_attach_set_par<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let var = img.word(f + 4);
    let table = img.word(f + 0x15);
    let arg = fr.arg(g, 0);
    let p = g.read::<u32>(var);
    let v = g.read::<u32>(p.wrapping_add(0x1a0));
    let slot442 = slot_target(g, table, 442);
    g.call(slot442, fr.ecx, &[arg, v, 3]);
    let slot443 = slot_target(g, table, 443);
    g.call(slot443, fr.ecx, &[arg, 0x40]);
    g.write::<u32>(arg + 0x130, 0x3dcc_cccd);
    g.write::<u32>(arg + 0x134, 0x3dcc_cccd);
    g.write::<u32>(arg + 0x138, 0x3dcc_cccd);
    g.write::<u32>(arg + 0x13c, 0);
    let slot444 = slot_target(g, table, 444);
    g.call(slot444, fr.ecx, &[0]);
    let slot440 = slot_target(g, table, 440);
    let p2 = g.read::<u32>(var);
    g.call(slot440, fr.ecx, &[p2, arg, 0, 0]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x87)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0x9a)]);
    Ret::eax(r.eax)
}

/// [`obj_attach_set_par`]'s code identity.
pub const OBJ_ATTACH_SET_PAR: Sig = Sig {
    len: 173,
    hash: 0x96d7_fa66_d12d_e0ee,
    calls: &[],
    imports: &[],
};

/// `magic_0335` and `magic_0336` 0x10001040 (65 bytes; 10 DLLs carry it): slot
/// 433 with 1 (`rcefEffectReady`), `rcefTaskSetSeq(arg, cb)` (slot 450) and
/// `rcefTaskCallSeq(arg, cb2)` (449); returns slot 449's eax. The second
/// callback differs between the two DLLs.
pub fn effect_ready_seq<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let slot433 = slot_target(g, table, 433);
    g.call(slot433, fr.ecx, &[1]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x18)]);
    let slot449 = slot_target(g, table, 449);
    let r = g.call(slot449, fr.ecx, &[arg, img.word(f + 0x2d)]);
    Ret::eax(r.eax)
}

/// [`effect_ready_seq`]'s code identity.
pub const EFFECT_READY_SEQ: Sig = Sig {
    len: 65,
    hash: 0xcbb8_0648_accd_cec2,
    calls: &[],
    imports: &[],
};

/// `magic_0337` and `magic_0339` 0x10001090 (94 bytes; 6 DLLs carry it): slot 473
/// with `(0, 0, 0, 0x40, 0xf)`, `rcefMainCreateTask(V, cb)` (slot 434) with
/// `V` the game word at `ds:G2`, `rcefTaskSetSeq(arg, cb2)` (slot 450) and
/// `rcefTaskWait(arg, 0x1e)` (slot 451); returns slot 451's eax.
pub fn main_create_task_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let table = img.word(f + 4);
    let arg = fr.arg(g, 0);
    let slot473 = slot_target(g, table, 473);
    g.call(slot473, fr.ecx, &[0, 0, 0, 0x40, 0xf]);
    let v = g.read::<u32>(img.word(f + 0x26));
    let slot434 = slot_target(g, table, 434);
    g.call(slot434, fr.ecx, &[v, img.word(f + 0x20)]);
    let slot450 = slot_target(g, table, 450);
    g.call(slot450, fr.ecx, &[arg, img.word(f + 0x38)]);
    let slot451 = slot_target(g, table, 451);
    let r = g.call(slot451, fr.ecx, &[arg, 0x1e]);
    Ret::eax(r.eax)
}

/// [`main_create_task_wait`]'s code identity.
pub const MAIN_CREATE_TASK_WAIT: Sig = Sig {
    len: 94,
    hash: 0xe8d2_e932_49bf_c718,
    calls: &[],
    imports: &[],
};
