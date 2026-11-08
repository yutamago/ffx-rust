//! Round 18's own ports (own rows of `magic_0450` and up): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code
//! written out where no ported kin exists.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`scripts_step_motion`](crate::r13_misc::scripts_step_motion) in
/// `magic_0587` (0x100014e0; the integer fade's step and the models' end;
/// the `rel32`s: `memset`, the scripts, the motion step, the fade, the
/// rigs' step, the models' end and the give-back).
pub const SCRIPTS_STEP_MOTION_0587: Sig = Sig {
    len: 174,
    hash: 0xd7b2_6026_98dc_e4a0,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x88, Callee::Sig(&crate::s4_small::INT_FADE)),
        (0x90, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x9f, Callee::Sig(&crate::m0587::MODELS_END)),
        (0xaa, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`walk_reset`](crate::s4_small::walk_reset) in `magic_0577`
/// (0x10004720, internal, `ecx` = the cursor): the summon model set-up
/// and the unlink-and-end in place of the nodes' reset and the last
/// leave.
pub const WALK_RESET_0577: Sig = Sig {
    calls: &[
        (0x68, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x76, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x7d, Callee::Sig(&crate::m0577::UNLINK_END)),
    ],
    ..crate::s4_small::WALK_RESET
};

/// `magic_0577`'s 12-byte thunk over [`WALK_RESET_0577`] (0x100047b0:
/// `ecx` = the first stack word, then a jump with the caller's stack).
pub fn jmp_walk_reset_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    crate::s4_small::walk_reset(g, img, &callee)
}

/// [`jmp_walk_reset_0577`].
pub const JMP_WALK_RESET_0577: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&WALK_RESET_0577))],
    imports: &[],
};

/// [`host_start_parts`](crate::o6_fam_start::host_start_parts) in
/// `magic_0598` (0x10006740, 465 bytes: slots 4 and 27 after slot 720).
pub const HOST_START_PARTS_0598: Sig = Sig {
    len: 465,
    hash: 0x5db1_3cd0_317f_bc9e,
    calls: &[
        (0xd, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
    ],
    imports: &[],
};

/// [`host_start_parts`](crate::o6_fam_start::host_start_parts) in
/// `magic_0600` (0x10006140, 422 bytes: slots 4 and 27 after slot 720).
pub const HOST_START_PARTS_0600: Sig = Sig {
    len: 422,
    hash: 0xe95a_d144_2d22_0a64,
    ..HOST_START_PARTS_0598
};

/// [`summon_frame_fade`](crate::s8_misc::summon_frame_fade) in
/// `magic_0610` (0x1000a6d0, 226 bytes: no fade, the scene's end).
pub const SUMMON_FRAME_0610: Sig = Sig {
    len: 226,
    hash: 0x5219_9e7f_58ab_0187,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xb2, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xbc, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xc4, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (
            0xd3,
            Callee::Sig(&crate::o6_fam_pairs6::SCENE_END_0610_AB20),
        ),
        (0xde, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0610`'s seventh object started (0x1000bcb0, a table entry, no
/// arguments; the block `S = *G_S`, operand `+0x13`): slot 282 `()` gives
/// `v` (ST0, kept as f32 at `ebp - 4`); the object `S + 0x850`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale `K1
/// / v * K2` with `K1`, `K2` at `+0x20`, `+0x30`, index 7); `S + 0x954` =
/// the callback (`+0x4d`) and byte `S + 0x976` = 2; the object `O = S +
/// 0x1938` gets its colour `+0xf0` 0x30808080, u16 `+0x124` = 4, `+0x120`
/// and `+0x110` 0 and the three scales `+0xf8..+0x100` 0.025, and its task
/// is listed ([`object_task`](crate::r13_misc::object_task), `ecx` = `O`).
/// Returns the task's node.
pub fn seventh_start_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let t = slot_target(g, w(0x5), 282);
    let s = g.read::<u32>(w(0x13));
    let v = crate::m0018::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), v);
    let scale = mul(div(g.read::<f32>(w(0x20)), v), g.read::<f32>(w(0x30)));
    // push 7 (`6a 07`); lea edx, [edi + 0x850]
    let index = w(0x35) & 0xff;
    let at = img.rel32_target(f + 0x3d);
    crate::s4_misc::start_object_plain(g, img, at, s.wrapping_add(w(0x38)), scale, index);
    let o = s.wrapping_add(w(0x43));
    g.write::<u32>(s.wrapping_add(w(0x49)), w(0x4d));
    g.write::<u8>(s.wrapping_add(w(0x53)), w(0x57) as u8);
    g.write::<u32>(o.wrapping_add(0xf0), 0x3080_8080);
    g.write::<u16>(o.wrapping_add(0x124), 4);
    g.write::<u32>(o.wrapping_add(0x120), 0);
    g.write::<u32>(o.wrapping_add(0x110), 0);
    for k in [0xf8, 0xfc, 0x100] {
        g.write::<u32>(o.wrapping_add(k), 0x3ccc_cccd);
    }
    // push ebp, ecx, esi, edi; the task's return address
    let task = Frame {
        at: img.rel32_target(f + 0xa4),
        esp: fr.esp.wrapping_sub(0x14),
        ecx: o,
        ..*fr
    };
    crate::r13_misc::object_task(g, img, &task)
}

/// [`seventh_start_0610`] (174 bytes, 1 overlay).
pub const SEVENTH_START_0610: Sig = Sig {
    len: 174,
    hash: 0xa1c0_66ae_5c40_ba1f,
    calls: &[
        (0x3d, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xa4, Callee::Sig(&crate::r13_misc::OBJECT_TASK_0610)),
    ],
    imports: &[],
};

/// `magic_0588`'s caster placement (0x10001820, cdecl, no arguments; a
/// cookie frame, `ebp` = `esp - 4`; the effect block `E = *G_E`, operand
/// `+0x13`, the actor `A = *G_A`): slot 262 `(A, &x, &y, &z)` (locals at
/// `ebp - 0x14`..`- 0xc`) into `E + 0xc04..0xc0c`, `E + 0xc10` 1.0; the
/// yaw `K - slot 333 (A)` (ST0) brought back once by `T` when above `H`
/// or below `L` (`comiss` + `jbe`: an unordered yaw stays); `E + 0xbc4`
/// = (0, yaw, 0, 0) and `+0xbd4` = (x, y, z, 1.0). The matrix `M = E +
/// 0xac4`: slots 210 `(A, M)`, 211 `(A, M)`, 235 `(M)`; slot 277 `(A)`
/// times `K2` into the diagonal `M[0]`, `M[5]`, `M[10]`, each then scaled
/// by slots 15, 16, 17 `(A)` (ST0 each); slots 330, 328 `(M, M, 0)` and
/// 329 `(M, M, yaw)`. The point `+0xbd4` copied to `+0xaf4`; slot 238
/// `(I, 1)` (`I = *G_I`); with `*G_F` 0 and `I` below 0x20 slot 296 `(I,
/// 0)`, else slot 257 `(A, 1)`; `E + 0xdf4` = yaw, `+0xdec`, `+0xdf0`,
/// `+0xe08` 0, `*G_X` and `*G_Y` 1. Returns the last slot's `eax`. Each
/// call sees the words the earlier ones left on the stack.
///
/// `magic_0610`'s copy (0x1000a800, 785 bytes,
/// [`CASTER_PLACE_0610`]; `push ebx` at `+0x10`) keeps the point at `E +
/// 0xca0`, the yaw block at `+0xc60` and `M = E + 0xbe0` (its first ten
/// operands 2 bytes later, the eleventh 1), scales `M[0]` by slot 15's
/// value the other way round (`M[0] v`), and ends with `+0xc70` copied to
/// `+0xc10`, `E + 0x1a68` = yaw, the others shown
/// ([`show_others`](crate::sb_vm::show_others) with 0), `E + 0x1a60`,
/// `+0x1a64`, `+0x1a84`, `+0x1a88` 0 and `*G_X`, `*G_Y` 1, returning the
/// show's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls and stores, in order
pub fn caster_place_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    // `magic_0610`: `push ebx` (0x53) at `+0x10`
    let c0610 = w(0x10) & 0xff == 0x53;
    let (d, d1) = if c0610 { (2, 1) } else { (0, 0) };
    let (point, block, mat) = if c0610 {
        (0xca0u32, 0xc60u32, 0xbe0u32)
    } else {
        (0xc04, 0xbc4, 0xac4)
    };
    let e = rd(g, 0x13 + d);
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let mut st = Pushes::default();
    st.push_all(&[pz, py, px, rd(g, 0x2a + d)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x24 + d, 262), fr.ecx);
    let (x, y, z) = (g.read::<u32>(px), g.read::<u32>(py), g.read::<u32>(pz));
    g.write::<u32>(e.wrapping_add(point), x);
    g.write::<u32>(e.wrapping_add(point + 4), y);
    g.write::<u32>(e.wrapping_add(point + 8), z);
    g.write::<u32>(e.wrapping_add(point + 0xc), 0x3f80_0000);
    st.push(rd(g, 0x6e + d));
    g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    let t = st0_f32(st.call(g, slot(g, 0x3c + d, 333), fr.ecx).st0);
    st.pop(5);
    let mut yaw = sub(k(g, 0x88 + d), t);
    if yaw > k(g, 0x97 + d) {
        yaw = sub(yaw, k(g, 0xa6 + d));
    } else if k(g, 0xb0 + d) > yaw {
        yaw = add(yaw, k(g, 0xbd + d));
    }
    g.write::<f32>(ebp.wrapping_sub(0x2c), yaw);
    for (o, v) in [
        (0u32, 0),
        (4, yaw.to_bits()),
        (8, 0),
        (0xc, 0),
        (0x10, x),
        (0x14, y),
        (0x18, z),
        (0x1c, 0x3f80_0000),
    ] {
        g.write::<u32>(e.wrapping_add(block + o), v);
    }
    let a = rd(g, 0x122 + d1);
    let m = e.wrapping_add(mat);
    if m == 0 {
        // `lea ebx, [edi + 0xac4]; test ebx, ebx; je`: only for a block at
        // -0xac4 (whose stores above already wrap below 0x10000)
        if rd(g, 0x14a) == 0 {
            let i = rd(g, 0x153);
            if i.cast_signed() < 0x20 {
                let l = ebp.wrapping_sub(0x24);
                st.push_all(&[l, i]);
                // call width: 12 (Pushes caps the words at 12)
                st.call(g, slot(g, 0x161, 217), fr.ecx);
                st.push_all(&[l, i]);
                // call width: 12 (Pushes caps the words at 12)
                st.call(g, slot(g, 0x173, 255), fr.ecx);
                st.pop(4);
            }
        }
    } else {
        st.push_all(&[m, a]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0x137, 210), fr.ecx);
        st.pop(2);
    }
    st.push_all(&[m, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x189, 211), fr.ecx);
    st.push(m);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x196, 235), fr.ecx);
    st.push(rd(g, 0x1aa));
    // call width: 12 (Pushes caps the words at 12)
    let v = st0_f32(st.call(g, slot(g, 0x1a4, 277), fr.ecx).st0);
    let d = mul(v, k(g, 0x1c2));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m.wrapping_add(o), d);
    }
    let a2 = rd(g, 0x1db);
    st.push(a2);
    // call width: 12 (Pushes caps the words at 12)
    let v = st0_f32(st.call(g, slot(g, 0x1c7, 15), fr.ecx).st0);
    let d0 = if c0610 {
        mul(g.read::<f32>(m), v)
    } else {
        mul(v, g.read::<f32>(m))
    };
    st.push(a2);
    g.write::<f32>(m, d0);
    // call width: 12 (Pushes caps the words at 12)
    let v = st0_f32(st.call(g, slot(g, 0x1f2, 16), fr.ecx).st0);
    let d1 = mul(g.read::<f32>(m.wrapping_add(0x14)), v);
    st.push(a2);
    g.write::<f32>(m.wrapping_add(0x14), d1);
    // call width: 12 (Pushes caps the words at 12)
    let v = st0_f32(st.call(g, slot(g, 0x20e, 17), fr.ecx).st0);
    let d2 = mul(v, g.read::<f32>(m.wrapping_add(0x28)));
    st.pop(6);
    st.set_top(0);
    g.write::<f32>(m.wrapping_add(0x28), d2);
    for (o, n, top) in [
        (0x249u32, 330u32, 0u32),
        (0x249, 328, 0),
        (0x267, 329, yaw.to_bits()),
    ] {
        st.set_top(top);
        st.push_all(&[m, m]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, o, n), fr.ecx);
        st.pop(2);
    }
    if c0610 {
        for i in 0..4 {
            let v = g.read::<u32>(e.wrapping_add(0xc70 + 4 * i));
            g.write::<u32>(e.wrapping_add(0xc10 + 4 * i), v);
        }
        g.write::<f32>(e.wrapping_add(0x1a68), yaw);
        // the call's return address below the saved registers
        let at = img.rel32_target(f + 0x2c0);
        let eax = crate::sb_vm::show_others(g, img, at, fr.esp.wrapping_sub(0x40), 0);
        for o in [0x1a60u32, 0x1a64, 0x1a84, 0x1a88] {
            g.write::<u32>(e.wrapping_add(o), 0);
        }
        g.write::<u32>(w(0x2f5), 1);
        g.write::<u32>(w(0x2ff), 1);
        return Ret::eax(eax);
    }
    for i in 0..4 {
        let v = g.read::<u32>(e.wrapping_add(0xbd4 + 4 * i));
        g.write::<u32>(e.wrapping_add(0xaf4 + 4 * i), v);
    }
    st.pop(1);
    st.push_all(&[1, rd(g, 0x2b6)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x2ae, 238), fr.ecx);
    st.pop(2);
    let i = rd(g, 0x2d1);
    let eax = if rd(g, 0x2c8) == 0 && i.cast_signed() < 0x20 {
        st.push_all(&[0, i]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0x2de, 296), fr.ecx).eax
    } else {
        st.push_all(&[1, rd(g, 0x2f3)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0x2eb, 257), fr.ecx).eax
    };
    g.write::<f32>(e.wrapping_add(0xdf4), yaw);
    for o in [0xdecu32, 0xdf0, 0xe08] {
        g.write::<u32>(e.wrapping_add(o), 0);
    }
    g.write::<u32>(w(0x334), 1);
    g.write::<u32>(w(0x33e), 1);
    Ret::eax(eax)
}

/// [`caster_place_0588`] (848 bytes, 1 overlay; the cookie check).
pub const CASTER_PLACE_0588: Sig = Sig {
    len: 848,
    hash: 0x3824_8f9d_2427_a54b,
    calls: &[(0x348, Callee::Any)],
    imports: &[],
};

/// [`caster_place_0588`] in `magic_0610` (0x1000a800, 785 bytes; the
/// others shown and the cookie check).
pub const CASTER_PLACE_0610: Sig = Sig {
    len: 785,
    hash: 0x3e40_84a4_b344_1d82,
    calls: &[
        (0x2c0, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0x309, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0588`'s effect end (0x10001c00, an overlay table entry, cdecl,
/// no arguments; an 8-aligned cookie frame): slot 384 `(T[K[1]])` (the
/// byte `K[1]` of `*G_K`, `+0x15`, indexing the word table `+0x22`);
/// slots 25 `(0x3016, 1)` and 24 `(0x3016, 0)`; with `*G_F` 0 and the
/// actor index `I = *G_I` below 0x20 slots 217 and 255 `(I, &L)` (a local
/// at the aligned frame's `+0x4`); slot 211 `(*G_W, 0)`; with the
/// halfword `*G_H` 0 slot 213 `(*G_X, 0)`; `*G_S` = 1; the others shown
/// ([`show_others`](crate::sb_vm::show_others) with 1) and the party run
/// ([`each_actor`](crate::sa_script::each_actor) with 1); with `*G_F` 0
/// and `I` below 0x20 slot 296 `(I, 1)`, else slot 257 `(*G_A, 0)`; slot
/// 238 `(I + 1, 1)`, slot 371 `(0, 0)` and `(1, 0)`; the held node `*G_N`
/// given back ([`give_back`](crate::sa_pool::give_back)) and cleared; the
/// objects freed ([`objects_freed`](crate::s6_misc::objects_freed)) and
/// the parts registered ([`register_parts`](crate::sa_misc::register_parts));
/// slot 772 `(0x1ae, 0)` and slot 779 `(eax, edx, 0x1ae, 0)` with
/// `**G_LOCK` 1 around it. Returns `*G_LOCK`.
#[allow(clippy::too_many_lines)] // the original's calls, in order
pub fn effect_end_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // and esp, -8; sub esp, 0x18; push esi; push edi
    let frame = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let callee_esp = frame.wrapping_sub(0xc);
    let kb = u32::from(g.read::<u8>(rd(g, 0x15).wrapping_add(1)));
    let v = g.read::<u32>(w(0x22).wrapping_add(4 * kb));
    let mut st = Pushes::default();
    st.push(v);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x27, 384), fr.ecx);
    st.push_all(&[1, w(0x3e)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x34, 25), fr.ecx);
    st.push_all(&[0, w(0x4f)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x45, 24), fr.ecx);
    st.pop(5);
    let wd = rd(g, 0x57);
    if rd(g, 0x60) == 0 {
        let i = rd(g, 0x69);
        if i.cast_signed() < 0x20 {
            let l = frame.wrapping_add(4);
            st.push_all(&[l, i]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, slot(g, 0x78, 217), fr.ecx);
            st.push_all(&[l, i]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, slot(g, 0x8b, 255), fr.ecx);
            st.pop(4);
        }
    }
    st.push_all(&[0, wd]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x9c, 211), fr.ecx);
    st.pop(2);
    if g.read::<u16>(w(0xb1)) == 0 {
        st.push_all(&[0, rd(g, 0xc1)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0xb9, 213), fr.ecx);
        st.pop(2);
    }
    g.write::<u32>(w(0xd7), 1);
    crate::sb_vm::show_others(g, img, img.rel32_target(f + 0xe0), callee_esp, 1);
    crate::sa_script::each_actor(g, img, img.rel32_target(f + 0xea), 1, callee_esp);
    let i = rd(g, 0xf8);
    if rd(g, 0xf0) == 0 && i.cast_signed() < 0x20 {
        st.push_all(&[1, i]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0x105, 296), fr.ecx);
    } else {
        st.push_all(&[0, rd(g, 0x11a)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, slot(g, 0x112, 257), fr.ecx);
    }
    let i = rd(g, 0x127);
    st.pop(2);
    st.push_all(&[1, i.wrapping_add(1)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x133, 238), fr.ecx);
    st.push_all(&[0, 0]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x140, 371), fr.ecx);
    st.push_all(&[0, 1]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x151, 371), fr.ecx);
    st.pop(6);
    let node = rd(g, 0x163);
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x16f), node);
        g.write::<u32>(w(0x175), 0);
    }
    let freed = Frame {
        at: img.rel32_target(f + 0x17e),
        esp: callee_esp,
        ..*fr
    };
    let eax = crate::s6_misc::objects_freed(g, img, &freed).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x183), eax);
    st.push_all(&[0, w(0x195)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot(g, 0x188, 772), fr.ecx);
    let lock = rd(g, 0x19d);
    st.push(h.edx);
    let saved = g.read::<u32>(lock);
    st.push(h.eax);
    g.write::<u32>(lock, 1);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x1a6, 779), fr.ecx);
    let lock = rd(g, 0x1b9);
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`effect_end_0588`] (467 bytes, 1 overlay; the `rel32`s: the others
/// shown, the party run, the give-back, the objects freed, the parts
/// registered and the cookie check).
pub const EFFECT_END_0588: Sig = Sig {
    len: 467,
    hash: 0xf10f_4a6b_37df_2dc7,
    calls: &[
        (0xe0, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0xea, Callee::Sig(&crate::sa_script::EACH_ACTOR)),
        (0x16f, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x17e, Callee::Sig(&crate::s6_misc::OBJECTS_FREED)),
        (0x183, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0x1cb, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_frame_fade`](crate::s8_misc::summon_frame_fade) in
/// `magic_0588` (0x100016f0, `magic_0610`'s shape; the end is
/// [`effect_end_0588`]).
pub const SUMMON_FRAME_0588: Sig = Sig {
    len: 226,
    hash: 0xfa69_ac28_9288_e15f,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xb2, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xbc, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xc4, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0xd3, Callee::Sig(&EFFECT_END_0588)),
        (0xde, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0588`'s wave tick (0x10001ec0, an overlay table entry, cdecl,
/// no arguments; an 8-aligned cookie frame; the block `E = *G_E`,
/// operand `+0x19`): the count `E + 0xdf8` steps down while positive;
/// by the state `E + 0xdfc` with `n` the count and `x = n / K2`:
/// - 0 (rising): the phase `(K1 - x) K3 + x 0`, the level `128 n / 50`;
///   at `n` 0 the count goes back to 50 and the state to 1;
/// - 1 (falling): the phase `(K1 - x) 0 + x K3`, the level `128 (50 - n)
///   / 50`; at `n` 0 slot 311 `(*G_P)`, the state on and `*G_P` = -1;
/// - any other: the phase 0, the level 0x80.
///
/// Then the table slot 562 `()` gives is filled
/// ([`wave_table`](crate::s5_misc::wave_table) at that phase), and slot
/// 862 `(h, table, &L, 0x1ae, 0)` registers it with `h` slot 772 `(0x1ae,
/// 0)`'s 64-bit handle, `L` the local `(s, s, s, 1.0)` (`s` = level `K4`),
/// `**G_LOCK` 1 around the call. Returns `*G_LOCK`.
pub fn wave_tick_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // and esp, -8; sub esp, 0x1c: the locals from `frame`
    let frame = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let e = g.read::<u32>(w(0x19));
    let count = e.wrapping_add(0xdf8);
    let state = e.wrapping_add(0xdfc);
    let c = g.read::<i32>(count);
    if c > 0 {
        g.write::<i32>(count, c - 1);
    }
    let (mut phase, mut level) = (0.0f32, 0x80i32);
    match g.read::<u32>(state) {
        0 => {
            let n = g.read::<i32>(count);
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let x = div(n as f32, k(g, 0xe7));
            level = n.wrapping_shl(7) / 50;
            phase = add(mul(sub(k(g, 0x46), x), k(g, 0x106)), mul(x, 0.0));
            if n == 0 {
                g.write::<u32>(count, 0x32);
                g.write::<u32>(state, 1);
            }
        }
        1 => {
            let n = g.read::<i32>(count);
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let x = div(n as f32, k(g, 0x70));
            level = 50i32.wrapping_sub(n).wrapping_shl(7) / 50;
            phase = add(mul(sub(k(g, 0x46), x), 0.0), mul(x, k(g, 0x88)));
            if n == 0 {
                let p = g.read::<u32>(w(0xb3));
                let t = slot(g, 0xad, 311);
                // call width: 1
                g.call(t, fr.ecx, &[p]);
                let s = g.read::<u32>(state).wrapping_add(1);
                g.write::<u32>(state, s);
                g.write::<u32>(w(0xca), u32::MAX);
            }
        }
        _ => {}
    }
    g.write::<f32>(frame.wrapping_add(4), phase);
    let t = slot(g, 0x132, 562);
    // call width: 0
    let table = g.call(t, fr.ecx, &[]).eax;
    crate::s5_misc::wave_table(g, img, img.rel32_target(f + 0x149), table, phase);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let s = mul(level as f32, k(g, 0x165));
    let l = frame.wrapping_add(8);
    for o in [0u32, 4, 8] {
        g.write::<f32>(l.wrapping_add(o), s);
    }
    g.write::<u32>(l.wrapping_add(0xc), 0x3f80_0000);
    let mut st = Pushes::default();
    st.push_all(&[0, w(0x16a)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, slot(g, 0x14e, 772), fr.ecx);
    let lock = g.read::<u32>(w(0x18c));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    st.push_all(&[l, table, h.edx, h.eax]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x1a1, 862), fr.ecx);
    let lock = g.read::<u32>(w(0x1ae));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`wave_tick_0588`] (457 bytes, 1 overlay; the `rel32`s: the wave
/// table and the cookie check).
pub const WAVE_TICK_0588: Sig = Sig {
    len: 457,
    hash: 0x8b62_9fbd_7a68_40b5,
    calls: &[
        (0x149, Callee::Sig(&crate::s5_misc::WAVE_TABLE)),
        (0x1c1, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0588`'s caster object start (0x100020d0, an overlay table
/// entry, cdecl, no arguments; a cookie frame, `ebp` = `esp - 4`; the
/// block `E = *G_E`, operand `+0x1c`): the object `O = E + 0x980`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// `*K` `+0x14`, index 8) gets the point `E + 0xc04..0xc10` at `+0xd0`,
/// its `y` lowered by `*K2` (`+0x6e`), `+0xc0` -pi/2 and `+0xc4` the yaw
/// `E + 0xdf4`; the record `P = E + 0xc20` its words (`+0x80` 0, `+0x84`
/// 2.0, `+0x88` 0x82, u16 `+0x98` 0x44, `+0x90`/`+0x8c` 0x14, `+0x94`
/// 0); a pool node ([`take`](crate::sa_pool::take), owner `+0x9a`) gets
/// `+0x14 = P` and is appended to the task list (first `+0xea`, last
/// `+0xff`); slot 332 `(&L)` with the local colour `L = (0, 0, 0, 0x80)`
/// at `ebp - 0x14`, whose `eax` it returns.
pub fn caster_object_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let e = g.read::<u32>(w(0x1c));
    let o = e.wrapping_add(0x980);
    let at = img.rel32_target(f + 0x2e);
    let scale = g.read::<f32>(w(0x14));
    crate::s4_misc::start_object_plain(g, img, at, o, scale, w(0x2a) & 0xff);
    for i in 0..4 {
        let v = g.read::<u32>(e.wrapping_add(0xc04 + 4 * i));
        g.write::<u32>(o.wrapping_add(0xd0 + 4 * i), v);
    }
    let y = sub(g.read::<f32>(o.wrapping_add(0xd4)), g.read::<f32>(w(0x6e)));
    g.write::<u32>(o.wrapping_add(0xc0), 0xbfc9_0fdb);
    let yaw = g.read::<u32>(e.wrapping_add(0xdf4));
    g.write::<u32>(o.wrapping_add(0xc4), yaw);
    g.write::<f32>(o.wrapping_add(0xd4), y);
    let p = e.wrapping_add(0xc20);
    g.write::<u32>(p.wrapping_add(0x80), 0);
    g.write::<u32>(p.wrapping_add(0x84), 0x4000_0000);
    g.write::<u32>(p.wrapping_add(0x88), 0x82);
    g.write::<u16>(p.wrapping_add(0x98), 0x44);
    g.write::<u32>(p.wrapping_add(0x90), 0x14);
    g.write::<u32>(p.wrapping_add(0x8c), 0x14);
    g.write::<u32>(p.wrapping_add(0x94), 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xe4), w(0x9a));
    let first = g.read::<u32>(w(0xea));
    g.write::<u32>(node.wrapping_add(0x14), p);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x111), node);
    } else {
        let last = g.read::<u32>(w(0xff));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x119), node);
    let l = ebp.wrapping_sub(0x14);
    for (i, v) in (0u32..).zip([0u32, 0, 0, 0x80]) {
        g.write::<u32>(l.wrapping_add(4 * i), v);
    }
    let t = slot_target(g, w(0x122), 332);
    // call width: 1
    let eax = g.call(t, fr.ecx, &[l]).eax;
    Ret::eax(eax)
}

/// [`caster_object_0588`] (347 bytes, 1 overlay; the `rel32`s: the
/// object start, the pool's take and the cookie check).
pub const CASTER_OBJECT_0588: Sig = Sig {
    len: 347,
    hash: 0x6a0d_b446_1ee2_5300,
    calls: &[
        (0x2e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xe4, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x153, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0588`'s start-up (0x10002c70, run by its load step; cdecl, no
/// arguments, no frame): four words set (the effect block `*G_E` among
/// them); slot 719 `(PART, 8, 0)` into `G_H` (`+0x42`); the part set up
/// ([`part_setup`](crate::sb_misc::part_setup), the pushed buffer
/// `+0x47`); the caster placed ([`caster_place_0588`]) and the effect
/// turned ([`start_turned`](crate::o4_effect::start_turned)); two node
/// heads (words, a halfword 0 and a byte 1 each); slot 271 `(4)`; two
/// words, `+0x0` of 0x10 records (0x140 bytes from `+0xc9`) cleared, two
/// counts; two task nodes ([`take`](crate::sa_pool::take), owners
/// `+0xe8` and `+0x13a`, bytes 1 and 2) appended to the task list with
/// the 0x400 records set up between them
/// ([`records_init_400`](crate::sa_misc::records_init_400), the block
/// `+0x128`). Returns 0.
pub fn start_up_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let s719 = slot_target(g, w(0x01), 719);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32]);
    // call width: 3
    let part = g.call(s719, fr.ecx, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x42), part);
    // push 0 stays; push BUF; call
    g.write::<u32>(e.wrapping_sub(4), 0);
    g.write::<u32>(e.wrapping_sub(8), w(0x47));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x4c), e.wrapping_sub(0xc));
    let callee = |at: u32| Frame {
        at: img.rel32_target(f + at),
        esp: e.wrapping_sub(0xc),
        ..*fr
    };
    caster_place_0588(g, img, &callee(0x51));
    crate::o4_effect::start_turned(g, img, &callee(0x56));
    let s271 = slot_target(g, w(0x5b), 271);
    stores(g, img, f, &[0x5f, 0x69]);
    g.write::<u16>(w(0x76), 0);
    g.write::<u8>(w(0x7e), 1);
    stores(g, img, f, &[0x83, 0x8d]);
    g.write::<u16>(w(0x9a), 0);
    g.write::<u8>(w(0xa2), 1);
    // call width: 3 (the buffer's words stay under it)
    g.call(s271, fr.ecx, &[4, w(0x47), 0]);
    stores(g, img, f, &[0xb4, 0xbe]);
    let mut rec = w(0xc9);
    for _ in 0..0x10 {
        g.write::<u32>(rec, 0);
        rec = rec.wrapping_add(0x140);
    }
    g.write::<u32>(w(0xe3), 0);
    stores(g, img, f, &[0xec]);
    let node = |g: &mut G, call: u32, owner: u32, flag: u8| {
        let n = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(owner));
        g.write::<u8>(n.wrapping_add(0x18), flag);
        g.write::<u32>(n.wrapping_add(4), 0);
        n
    };
    for (call, owner, flag, first, last, set_first, set_last) in [
        (0xf7u32, 0xe8u32, 1u8, 0xfdu32, 0x111u32, 0x123u32, 0x130u32),
        (0x13f, 0x13a, 2, 0x145, 0x159, 0x174, 0x166),
    ] {
        let n = node(g, call, owner, flag);
        if g.read::<u32>(w(first)) == 0 {
            g.write::<u32>(n, 0);
            g.write::<u32>(w(set_first), n);
        } else {
            let tail = g.read::<u32>(w(last));
            g.write::<u32>(n, tail);
            g.write::<u32>(tail.wrapping_add(4), n);
        }
        g.write::<u32>(n.wrapping_add(8), n);
        g.write::<u32>(w(set_last), n);
        if flag == 1 {
            crate::sa_misc::records_init_400(g, img, img.rel32_target(f + 0x135), w(0x128));
        }
    }
    Ret::eax(0)
}

/// [`start_up_0588`] (387 bytes, 1 overlay; the `rel32`s: the part's
/// set-up, the caster's placement, the turn, the takes and the records).
pub const START_UP_0588: Sig = Sig {
    len: 387,
    hash: 0x1e25_ff0d_55af_004b,
    calls: &[
        (0x4c, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x51, Callee::Sig(&CASTER_PLACE_0588)),
        (0x56, Callee::Sig(&crate::o4_effect::START_TURNED_0588_1B70)),
        (0xf7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x135, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x13f, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0588`'s load step (0x10001340, the 118-byte shape) running
/// [`start_up_0588`].
pub fn load_0588<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0588)
}

/// [`load_0588`].
pub const LOAD_0588: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0588))],
    imports: &[],
};

/// `magic_0493`'s four-part task (0x100065a0, a pool task, cdecl `(N)`;
/// an 8-aligned frame; `S = N[+4]`): the clock `S + 0xadc` += `K0`; by
/// the state `S + 0xae8`:
/// - 0: the part `i = S + 0xae4` starts once the clock reaches `T[i]`
///   (the table `+0x230`; `comiss` + `jb`: an unordered clock waits): for
///   part 2 first slot 239 `(*G_X, (*(S + 0xaec))[+0x50], 0x20)`; the
///   node `S + 0x2c (i + 0x18)` appended to the list `L`
///   ([`node_append`](crate::sb_misc::node_append)); its point `V = S +
///   0x2c i + 0x43c` from three slot 307 draws `r` (ST0): `(r - K1) K2 -
///   K3`, `K5 - (r - K1) K4`, `(r - K1) K2 - K3`; slot 233 `(V, V, s)`
///   with the scale `s = S + 0xae0`, slot 248 `(V, V, S + 0x59c)`, `V[3]`
///   1.0; the node `S + 0x2c (i + 0x1c)` appended to `L`; `W = S + 0x2c
///   i + 0x4ec` 1.0 three times, slot 233 `(W, W, s)`, `W[3] = (r - K1)
///   K6 + K7`; the group record (a local) `(S + 0x14c i + 0x5ac, L, B,
///   S + 0x2c (i + 0x18), C, S + 0x2c (i + 0x1c))` inserted
///   ([`node_insert`](crate::nodes::node_insert)); the particle record
///   `(S + 0x100 i + 0x20, P, S + 0x14c i + 0x5ac, *G_O, Q, 0)` with the
///   node's vtable set ([`particle_node`](crate::sb_nodes::particle_node));
///   `i` on, and after the fourth the state on;
/// - 1: once bit 2 of the four flags `S + 0x108 .. 0x408` is set in all,
///   the four parts' three nodes and the two nodes `S + 0x580`, `S +
///   0xaf0` leave their parents, the task's node `N[0]` leaves its ring
///   and the state is 0xff;
/// - any other: nothing.
///
/// Returns the last `eax` the original leaves (the part index when it
/// waits, the state less one, the flags' `and`, the task node's
/// neighbour, or the particle node's answer).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn four_parts_0493<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // and esp, -8; sub esp, 0x24; push ebx, esi, edi: the locals
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x24);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    g.write::<u32>(base.wrapping_add(8), s);
    let clock = add(g.read::<f32>(at(0xadc)), k(g, 0x22));
    let state = g.read::<u32>(at(0xae8));
    g.write::<f32>(at(0xadc), clock);
    match state {
        0 => {}
        1 => {
            let flags = [0x408u32, 0x308, 0x208, 0x108]
                .into_iter()
                .fold(u32::MAX, |a, o| a & g.read::<u32>(at(o)))
                & 4;
            if flags == 0 {
                return Ret::eax(0);
            }
            for p in 0..4u32 {
                for node in [0x420 + 0x2c * p, 0x4d0 + 0x2c * p, 0x5ac + 0x14c * p] {
                    crate::sa_misc::leave(g, at(node), 0);
                }
            }
            for node in [0x580u32, 0xaf0] {
                crate::sa_misc::leave(g, at(node), 0);
            }
            let n = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
            g.write::<u32>(prev.wrapping_add(8), next);
            let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
            g.write::<u32>(next.wrapping_add(4), prev);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            g.write::<u32>(at(0xae8), 0xff);
            return Ret::eax(prev);
        }
        _ => return Ret::eax(state.wrapping_sub(1)),
    }
    let i = g.read::<u32>(at(0xae4));
    let start = g.read::<f32>(w(0x230).wrapping_add(4 * i));
    // `comiss` + `jb`: below or unordered, wait
    let ready = clock >= start;
    if !ready {
        return Ret::eax(i);
    }
    let mut eax = i;
    if i == 2 {
        let p = g.read::<u32>(g.read::<u32>(at(0xaec)).wrapping_add(0x50));
        let x = g.read::<u32>(w(0x251));
        // call width: 3
        eax = g.call(slot(g, 0x24b, 239), fr.ecx, &[x, p, 0x20]).eax;
    }
    let list = w(0x26f);
    let i = g.read::<u32>(at(0xae4));
    let n1 = at(i.wrapping_add(0x18).wrapping_mul(0x2c));
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x274), n1, list, eax);
    let draw = |g: &mut G, o: u32| {
        let t = slot(g, o, 307);
        // call width: 0
        st0_f32(g.call(t, fr.ecx, &[]).st0)
    };
    let r = draw(g, 0x279);
    let v0 = sub(mul(sub(r, k(g, 0x293)), k(g, 0x2a4)), k(g, 0x2ac));
    let row = |g: &G| at(g.read::<u32>(at(0xae4)).wrapping_mul(0x2c));
    g.write::<f32>(row(g).wrapping_add(0x43c), v0);
    let r = draw(g, 0x2ba);
    let v1 = sub(k(g, 0x2e2), mul(sub(r, k(g, 0x2d4)), k(g, 0x2ed)));
    g.write::<f32>(row(g).wrapping_add(0x440), v1);
    let r = draw(g, 0x2ff);
    let v2 = sub(mul(sub(r, k(g, 0x319)), k(g, 0x32a)), k(g, 0x333));
    g.write::<f32>(row(g).wrapping_add(0x444), v2);
    let v = row(g).wrapping_add(0x43c);
    let scale = g.read::<u32>(at(0xae0));
    // call width: 3
    g.call(slot(g, 0x360, 233), fr.ecx, &[v, v, scale]);
    let v = row(g).wrapping_add(0x43c);
    // call width: 3
    g.call(slot(g, 0x388, 248), fr.ecx, &[v, v, at(0x59c)]);
    g.write::<u32>(row(g).wrapping_add(0x448), 0x3f80_0000);
    let i = g.read::<u32>(at(0xae4));
    let n2 = at(i.wrapping_add(0x1c).wrapping_mul(0x2c));
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x3c1), n2, w(0x3bc), eax);
    for o in [0x4ecu32, 0x4f0, 0x4f4] {
        g.write::<u32>(row(g).wrapping_add(o), 0x3f80_0000);
    }
    let wv = row(g).wrapping_add(0x4ec);
    let scale = g.read::<u32>(at(0xae0));
    // call width: 3
    g.call(slot(g, 0x422, 233), fr.ecx, &[wv, wv, scale]);
    let r = draw(g, 0x42f);
    let v3 = add(mul(sub(r, k(g, 0x44c)), k(g, 0x45d)), k(g, 0x475));
    g.write::<f32>(row(g).wrapping_add(0x4f8), v3);
    // the group record at `esp + 0x18`
    let rec = base.wrapping_add(0xc);
    let i = g.read::<u32>(at(0xae4));
    let words = [
        at(i.wrapping_mul(0x14c).wrapping_add(0x5ac)),
        w(0x465),
        w(0x46d),
        at(i.wrapping_add(0x18).wrapping_mul(0x2c)),
        w(0x47d),
        at(i.wrapping_add(0x1c).wrapping_mul(0x2c)),
    ];
    for (j, v) in (0u32..).zip(words) {
        g.write::<u32>(rec.wrapping_add(4 * j), v);
    }
    // ecx = the record; the call's return address below the pushes
    let insert = Frame {
        at: img.rel32_target(f + 0x4c0),
        esp: base.wrapping_sub(0x10),
        ecx: rec,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let i = g.read::<u32>(at(0xae4));
    let node = at(i.wrapping_shl(8).wrapping_add(0x20));
    let words = [
        node,
        w(0x4f7),
        at(i.wrapping_mul(0x14c).wrapping_add(0x5ac)),
        g.read::<u32>(w(0x4e7)),
        w(0x503),
        0,
    ];
    for (j, v) in (0u32..).zip(words) {
        g.write::<u32>(rec.wrapping_add(4 * j), v);
    }
    g.write::<u32>(node, w(0x511));
    let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x516), rec);
    let i = g.read::<u32>(at(0xae4)).wrapping_add(1);
    g.write::<u32>(at(0xae4), i);
    if i.cast_signed() >= 4 {
        let st = g.read::<u32>(at(0xae8)).wrapping_add(1);
        g.write::<u32>(at(0xae8), st);
    }
    Ret::eax(eax)
}

/// [`four_parts_0493`] (1334 bytes, 1 overlay; the `rel32`s: the two
/// appends, the insert and the particle node).
pub const FOUR_PARTS_0493: Sig = Sig {
    len: 1334,
    hash: 0x89b0_9ba8_627b_fa19,
    calls: &[
        (0x274, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3c1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4c0, Callee::Sig(&crate::nodes::INSERT)),
        (0x516, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// The yaw `y` brought back once by `T` when above `H` or below `L`
/// (`comiss` + `jbe`: an unordered yaw stays); the operands of `H`, `T`,
/// `L` and the second `T`.
fn yaw_wrap<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    y: f32,
    (hi, t, lo, t2): (u32, u32, u32, u32),
) -> f32 {
    use crate::sb_sse::{add, sub};
    let k = |o: u32| g.read::<f32>(img.word(f + o));
    if y > k(hi) {
        sub(y, k(t))
    } else if k(lo) > y {
        add(y, k(t2))
    } else {
        y
    }
}

/// One of `magic_0564`'s placed objects (the operand offsets in the
/// start-up): the object `+obj` started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale
/// `*K` `+scale`, the `push imm8`s `+value` and `+index`, the call at
/// `+call`); with the actor `a` slot 262 `(a, P1, P2, P3)` (`p`: the
/// three pushed immediates, last first), slot 12 `(a)` (ST0) into `*P2`,
/// `*G_ONE` 1.0 (`one`, the store's instruction); the yaw `K1 - slot 333
/// (a)` (ST0) wrapped (`wrap`) into `yaw`; `*P2 -= *K2` (`drop`: the
/// load, the constant, the store).
struct Placed {
    obj: u32,
    scale: u32,
    value: u32,
    index: u32,
    call: u32,
    p: [u32; 3],
    one: u32,
    k1: u32,
    wrap: (u32, u32, u32, u32),
    yaw: u32,
    drop: (u32, u32, u32),
}

/// [`Placed`] run with the actor `actor` reads (for slots 262, 12 and,
/// read again, 333).
fn placed_0564<G: Game>(g: &mut G, img: &Image, f: u32, p: &Placed, actor: &dyn Fn(&G) -> u32) {
    use crate::m0018::st0_f32;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, n: u32| slot_target(g, w(0x5), n);
    let at = img.rel32_target(f + p.call);
    let scale = k(g, p.scale);
    let (value, index) = (w(p.value) & 0xff, w(p.index) & 0xff);
    crate::s4_misc::start_object_anim(g, img, at, w(p.obj), scale, index, value);
    // sub esp, 8; push value; push index: the start's words stay
    let mut st = Pushes::default();
    st.push_all(&[value, index]);
    let a = actor(g);
    st.push_all(&[w(p.p[0]), w(p.p[1]), w(p.p[2]), a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 262), 0);
    st.push(a);
    // call width: 12 (Pushes caps the words at 12)
    let v = st0_f32(st.call(g, t(g, 12), 0).st0);
    g.write::<f32>(w(p.p[1]), v);
    g.write::<u32>(w(p.one + 2), w(p.one + 6));
    st.push(actor(g));
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(st.call(g, t(g, 333), 0).st0);
    let yaw = yaw_wrap(g, img, f, sub(k(g, p.k1), r), p.wrap);
    g.write::<f32>(w(p.yaw), yaw);
    let y = sub(k(g, p.drop.0), k(g, p.drop.1));
    g.write::<f32>(w(p.drop.2), y);
}

/// `magic_0564`'s start-up (0x10001340, run by its load step; cdecl, no
/// arguments; `ebp` = `esp - 4`): two words set; slot 293 `(A)` (`A =
/// *G_A`) picks the side: the neighbours `A + 1`, `A + 2` or `A - 1`,
/// `A - 2` go to the bytes `G_B0`, `G_B1` and slot 215's models into the
/// table `TB[n]`; `*G_K` 2; slot 719 `(PART, 8, 0)` into `G_H`; the part
/// set up ([`part_init`](crate::l5_objhost::part_init)). Then three
/// records `R` (0x20 bytes from `+0xbc`) by the kinds `k` of the table
/// `+0xc4` (-1 the caster `A`, else the byte `L[k]` of `*G_L`): with slot
/// 640 `(actor)` 0, the record gets the copy's word, its kind, the actor,
/// slot 215 `(actor)` (slot 413 with `*G_F` set), -1, 0, 1 and the flag
/// `*G_F != 0`; kinds 1 and 0 place one object each, -1 two (the actors
/// `TB[L[k]]` or `*G_M`; [`placed_0564`]); with slot 640 non-zero, slot
/// 541 `(actor, 0, 0x1e)` (with `*G_F` 0 and an actor below 0x20) or
/// slot 38 `(model, 0, 0x1e)`, `R + 0xc` 1 and the copy's word 0. Last
/// slot 719 `(PART, 6, 0)` and slot 220 `(that, 0x40, 0x100)`, a node
/// from the free ring (`*G_FREE`, u16 count `*G_COUNT`; with none the
/// original writes through address 0, as this does) set up and appended
/// to the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0564<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, n: u32| slot_target(g, w(0x5), n);
    let a = rd(g, 0xc);
    stores(g, img, f, &[0x19, 0x23]);
    let mut st = Pushes::default();
    st.push(a);
    // call width: 12 (Pushes caps the words at 12)
    let side = st.call(g, t(g, 293), fr.ecx).eax;
    st.pop(1);
    let (n1, n2, b0, tb1) = if side == 0 {
        (a.wrapping_add(1), a.wrapping_add(2), 0x3e, 0x4e)
    } else {
        (a.wrapping_sub(1), a.wrapping_sub(2), 0x58, 0x68)
    };
    g.write::<u8>(w(b0), n1 as u8);
    st.push(n1);
    // call width: 12 (Pushes caps the words at 12)
    let m = st.call(g, t(g, 215), fr.ecx).eax;
    g.write::<u32>(w(tb1).wrapping_add(n1.wrapping_mul(4)), m);
    g.write::<u8>(w(0x74), n2 as u8);
    st.push(n2);
    // call width: 12 (Pushes caps the words at 12)
    let m = st.call(g, t(g, 215), fr.ecx).eax;
    st.pop(2);
    g.write::<u32>(w(0x87).wrapping_add(n2.wrapping_mul(4)), m);
    stores(g, img, f, &[0x92]);
    st.push_all(&[0, 8, w(0xa5)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    g.write::<u32>(w(0xaf), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0xb4));
    let tb = |g: &G, o: u32, b: u32| {
        let l = rd(g, o);
        let i = u32::from(g.read::<u8>(l.wrapping_add(b)));
        g.read::<u32>(w(0x1bc).wrapping_add(4 * i))
    };
    let kind_actor = |g: &G, o: u32, k: u32| {
        if k == u32::MAX {
            rd(g, o)
        } else {
            u32::from(g.read::<u8>(rd(g, o + 7).wrapping_add(k)))
        }
    };
    let mut r = w(0xbc);
    let mut i = 0u32;
    while r.cast_signed() < w(0x558).cast_signed() {
        let k = g.read::<u32>(w(0xc4).wrapping_add(i));
        let actor = kind_actor(g, 0xce, k);
        let mut st = Pushes::default();
        st.push(actor);
        // call width: 12 (Pushes caps the words at 12)
        let busy = st.call(g, t(g, 640), fr.ecx).eax;
        if busy == 0 {
            let v = g.read::<u32>(w(0xf8).wrapping_add(i));
            g.write::<u32>(w(0xfe).wrapping_add(i), v);
            g.write::<u32>(r.wrapping_sub(8), w(0x105));
            g.write::<u32>(r.wrapping_sub(4), k);
            let actor = kind_actor(g, 0x112, k);
            g.write::<u32>(r, actor);
            let n = if rd(g, 0x123) == 0 { 215 } else { 413 };
            let mut st = Pushes::default();
            st.push(actor);
            // call width: 12 (Pushes caps the words at 12)
            let m = st.call(g, t(g, n), fr.ecx).eax;
            let flag = rd(g, 0x147);
            g.write::<u32>(r.wrapping_add(4), m);
            g.write::<u32>(r.wrapping_add(8), u32::MAX);
            g.write::<u32>(r.wrapping_add(0xc), 0);
            g.write::<u32>(r.wrapping_add(0x10), 1);
            g.write::<u16>(r.wrapping_add(0x14), u16::from(flag != 0));
            match k {
                1 => {
                    let p = Placed {
                        obj: 0x199,
                        scale: 0x191,
                        value: 0x19e,
                        index: 0x1a0,
                        call: 0x1a2,
                        p: [0x1ac, 0x1b5, 0x1c6],
                        one: 0x1e9,
                        k1: 0x212,
                        wrap: (0x221, 0x22b, 0x235, 0x242),
                        yaw: 0x24a,
                        drop: (0x252, 0x25a, 0x262),
                    };
                    placed_0564(g, img, f, &p, &|g: &G| tb(g, 0x1a7, 1));
                }
                0 => {
                    let p = Placed {
                        obj: 0x277,
                        scale: 0x26f,
                        value: 0x27c,
                        index: 0x27e,
                        call: 0x280,
                        p: [0x28a, 0x292, 0x2a3],
                        one: 0x2c6,
                        k1: 0x2ee,
                        wrap: (0x2fd, 0x307, 0x311, 0x31e),
                        yaw: 0x326,
                        drop: (0x32e, 0x336, 0x33e),
                    };
                    placed_0564(g, img, f, &p, &|g: &G| tb(g, 0x285, 0));
                }
                u32::MAX => {
                    let p = Placed {
                        obj: 0x353,
                        scale: 0x34b,
                        value: 0x358,
                        index: 0x35a,
                        call: 0x35c,
                        p: [0x372, 0x377, 0x37c],
                        one: 0x39f,
                        k1: 0x3b8,
                        wrap: (0x3c7, 0x3d1, 0x3db, 0x3e8),
                        yaw: 0x3f8,
                        drop: (0x400, 0x408, 0x41c),
                    };
                    placed_0564(g, img, f, &p, &|g: &G| rd(g, 0x367));
                    let p = Placed {
                        obj: 0x410,
                        scale: 0x3f0,
                        value: 0x415,
                        index: 0x417,
                        call: 0x421,
                        p: [0x437, 0x43c, 0x441],
                        one: 0x464,
                        k1: 0x47d,
                        wrap: (0x48c, 0x496, 0x4a0, 0x4ad),
                        yaw: 0x4b5,
                        drop: (0x4bd, 0x4c5, 0x4cd),
                    };
                    placed_0564(g, img, f, &p, &|g: &G| rd(g, 0x42c));
                }
                _ => {}
            }
        } else {
            let l = rd(g, 0x4dc);
            let near = if rd(g, 0x4d5) == 0 {
                let x = if k == u32::MAX {
                    rd(g, 0x4e3)
                } else {
                    u32::from(g.read::<u8>(l.wrapping_add(k)))
                };
                (x.cast_signed() < 0x20).then_some(x)
            } else {
                None
            };
            let (n, x) = if let Some(x) = near {
                (541, x)
            } else if k == u32::MAX {
                (38, rd(g, 0x513))
            } else {
                let b = u32::from(g.read::<u8>(l.wrapping_add(k)));
                (38, g.read::<u32>(w(0x520).wrapping_add(4 * b)))
            };
            let mut st = Pushes::default();
            st.push_all(&[0x1e, 0, x]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, n), fr.ecx);
            g.write::<u32>(r.wrapping_add(0xc), 1);
            g.write::<u32>(w(0x548).wrapping_add(i), 0);
        }
        r = r.wrapping_add(0x20);
        i = i.wrapping_add(4);
    }
    let mut st = Pushes::default();
    st.push_all(&[0, 6, w(0x572)]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    st.push_all(&[0x100, 0x40, h]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 220), fr.ecx);
    let count = g.read::<u16>(w(0x590));
    let node = if count == 0 {
        0
    } else {
        let node = rd(g, 0x5a2);
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node.wrapping_add(4)));
        g.write::<u32>(prev.wrapping_add(4), next);
        let (next, prev) = (g.read::<u32>(node.wrapping_add(4)), g.read::<u32>(node));
        g.write::<u32>(next, prev);
        if count > 1 {
            let next = g.read::<u32>(node.wrapping_add(4));
            g.write::<u32>(w(0x5bf), next);
        } else {
            g.write::<u32>(w(0x5c7), 0);
        }
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node.wrapping_add(o), 0);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(node.wrapping_add(0x10), w(0x5f8));
        g.write::<u8>(node.wrapping_add(0x18), 0);
        g.write::<u16>(w(0x603), count.wrapping_sub(1));
        node
    };
    g.write::<u32>(node.wrapping_add(4), 0);
    if rd(g, 0x609) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x63b), node);
    } else {
        let last = rd(g, 0x61b);
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x629), node);
    Ret::eax(0)
}

/// [`start_up_0564`] (1614 bytes, 1 overlay; the `rel32`s: the part's
/// set-up and the four object starts).
pub const START_UP_0564: Sig = Sig {
    len: 1614,
    hash: 0xd17b_e11f_9fca_9756,
    calls: &[
        (0xb4, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x1a2, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x280, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x35c, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x421, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0564`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0564`].
pub fn load_0564<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0564)
}

/// [`load_0564`].
pub const LOAD_0564: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0564))],
    imports: &[],
};

/// An object of `magic_0562`'s start-up placed at the caster: `x` and `z`
/// into the operands `+ox`, `+oz`, 1.0 into `+ow`, the yaw into `+oyaw`
/// (`movss` copies of the bits).
fn place_0562<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (x, z, yaw): (u32, u32, u32),
    (ox, oz, ow, oyaw): (u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(ox), x);
    g.write::<u32>(w(oz), z);
    g.write::<u32>(w(ow), 0x3f80_0000);
    g.write::<u32>(w(oyaw), yaw);
}

/// `magic_0562`'s start-up (0x100013e0, run by its load step; cdecl, no
/// arguments; a cookie frame, `ebp` = `esp - 4`): four words set; slot
/// 719 `(PART, 8, 0)` into `G_H`; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); slot 262 `(A, &x, &y,
/// &z)` (`A = *G_A`; locals at `ebp - 0x14`..`- 0xc`) and the yaw `K -
/// slot 333 (A)` (ST0) brought back once; six objects started
/// ([`start_object_owners`](crate::s4_misc::start_object_owners), their
/// scales, indices, owners and words the code's) at the caster's `x`,
/// `z`, 1.0 and the yaw, each `y` the caster's less a constant (the first
/// two in double: `(f64) y - D` rounded), three objects' other words
/// moved; slots 357 `(A, -1, 0x3c00)`, 356 `(A, -1, 0x2ee0)`, 362 `(*G_X,
/// A, 1, 0)`, 855 `(A, 1)`, 6 `(A, 1)`, 548 `(A)`; the records copied
/// ([`records_from_table`](crate::s4_small::records_from_table)); two
/// words 0; a task started ([`task_start`](crate::s3_misc::task_start),
/// the pushed state); seven words; the counted task
/// ([`counted_task`](crate::s4_small::counted_task)); two scale tasks
/// ([`scale_task_arg`](crate::s4_small::scale_task_arg), `ecx` `B1`
/// with 0 and `B2` with 0x15e; a word 0.5 after each); slot 271 `(4)`; the
/// actor block ([`me_block`](crate::sa_misc::me_block)); four task nodes
/// ([`take`](crate::sa_pool::take): a byte 1, then three with their
/// `+0x14` words) chained after the list's last, the 0x100 records set up
/// ([`records_init_100`](crate::sa_misc::records_init_100)) and a fifth
/// node (byte 2) appended. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0562<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::r11_fades::stores;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let e = fr.esp;
    let ebp = e.wrapping_sub(4);
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x20)]);
    stores(g, img, f, &[0x24, 0x2e, 0x38, 0x42]);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t(g, 719), fr.ecx).eax;
    st.pop(1);
    g.write::<u32>(w(0x52), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x57));
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    st.push_all(&[pz, py, px, rd(g, 0x6e)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 262), fr.ecx);
    st.push(rd(g, 0x81));
    g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    let r = st0_f32(st.call(g, t(g, 333), fr.ecx).st0);
    let yaw = yaw_wrap(g, img, f, sub(k(g, 0x9b), r), (0xaa, 0xb9, 0xc3, 0xd0));
    g.write::<f32>(ebp.wrapping_sub(0x18), yaw);
    let (x, z) = (g.read::<u32>(px), g.read::<u32>(pz));
    let y = g.read::<f32>(py);
    let yaw = yaw.to_bits();
    // push ebx, esi, edi: the starts' words follow
    let start =
        |g: &mut G, call: u32, obj: u32, scale: u32, (index, owner, a, b): (u32, u32, u32, u32)| {
            let at = img.rel32_target(f + call);
            let s = g.read::<f32>(w(scale));
            crate::s4_misc::start_object_owners(g, img, at, w(obj), s, index, owner, (a, b));
        };
    let below = |g: &G, o: u32| sub(y, k(g, o)).to_bits();
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let below_d = |g: &G, o: u32| ((f64::from(y) - g.read::<f64>(w(o))) as f32).to_bits();
    start(g, 0xf2, 0xed, 0xdd, (3, 0xa, u32::MAX, 0));
    place_0562(g, img, f, (x, z, yaw), (0xff, 0x10c, 0x119, 0x12e));
    let v = below_d(g, 0x140);
    g.write::<u32>(w(0x157), v);
    start(g, 0x15c, 0x14b, 0x126, (3, 0x23, u32::MAX, 0));
    place_0562(g, img, f, (x, z, yaw), (0x169, 0x176, 0x183, 0x198));
    let v = below_d(g, 0x1ad);
    g.write::<u32>(w(0x1c4), v);
    start(g, 0x1c9, 0x1b8, 0x190, (1, 0x62, 0, w(0x1a5)));
    let v = add(k(g, 0x1d1), k(g, 0x1d9));
    g.write::<f32>(w(0x1f6), v);
    start(g, 0x1fb, 0x1ee, 0x1e1, (4, 0x76, u32::MAX, 0));
    place_0562(g, img, f, (x, z, yaw), (0x208, 0x21d, 0x22a, 0x237));
    let v = below(g, 0x244);
    g.write::<u32>(w(0x258), v);
    let v = sub(k(g, 0x260), k(g, 0x268));
    g.write::<f32>(w(0x277), v);
    start(g, 0x27c, 0x24c, 0x215, (5, 0xa8, u32::MAX, 0));
    place_0562(g, img, f, (x, z, yaw), (0x289, 0x29e, 0x2ab, 0x2b8));
    let v = below(g, 0x2c5);
    g.write::<u32>(w(0x2db), v);
    let v = sub(k(g, 0x2e3), k(g, 0x2eb));
    g.write::<f32>(w(0x2f8), v);
    start(g, 0x2fd, 0x2f0, 0x296, (2, 0xd0, 0, w(0x2ca)));
    place_0562(g, img, f, (x, z, yaw), (0x30a, 0x322, 0x32f, 0x33c));
    let a = rd(g, 0x31a);
    let v = below(g, 0x349);
    g.write::<u32>(w(0x359), v);
    let v = sub(k(g, 0x361), k(g, 0x369));
    g.write::<f32>(w(0x371), v);
    // the last start's words stay under the slots' (sub esp: the frame)
    let mut st = Pushes::default();
    st.push_all(&[0, u32::MAX, 0xa8, 5]);
    st.push_all(&[w(0x2ca), 0, 0xd0, 2]);
    st.push_all(&[0x3c00, u32::MAX, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 357), fr.ecx);
    st.push_all(&[0x2ee0, u32::MAX, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 356), fr.ecx);
    st.push_all(&[0, 1, a, rd(g, 0x3a4)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 362), fr.ecx);
    let mut st = Pushes::default();
    st.push_all(&[1, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 855), fr.ecx);
    let a = rd(g, 0x3c4);
    st.push_all(&[1, a]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 6), fr.ecx);
    st.push(a);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 548), fr.ecx);
    // push ebx, esi, edi below the locals; the first push of 1 stays
    let base = ebp.wrapping_sub(0x18).wrapping_sub(0xc);
    let callee = |at: u32, esp: u32, ecx: u32, xmm: [u128; 4]| Frame {
        at: img.rel32_target(f + at),
        esp,
        ecx,
        xmm,
        ..*fr
    };
    crate::s4_small::records_from_table(
        g,
        img,
        &callee(0x3df, base.wrapping_sub(0x18), fr.ecx, fr.xmm),
    );
    g.write::<u32>(w(0x3e8), 0);
    g.write::<u32>(base.wrapping_sub(4), 1);
    g.write::<u32>(base.wrapping_sub(8), w(0x3f1));
    g.write::<u32>(w(0x3f7), 0);
    crate::s3_misc::task_start(
        g,
        img,
        &callee(0x400, base.wrapping_sub(0xc), fr.ecx, fr.xmm),
    );
    stores(g, img, f, &[0x404, 0x40e]);
    g.write::<u8>(w(0x41a), w(0x41e) as u8);
    g.write::<u16>(w(0x422), w(0x426) as u16);
    g.write::<u8>(w(0x42a), w(0x42e) as u8);
    stores(g, img, f, &[0x42f]);
    g.write::<u16>(w(0x43c), w(0x440) as u16);
    crate::s4_small::counted_task(
        g,
        img,
        &callee(0x443, base.wrapping_sub(0xc), fr.ecx, fr.xmm),
    );
    let xmm = |g: &G, a: u32, b: u32| {
        [
            0,
            u128::from(g.read::<u32>(w(a))),
            u128::from(g.read::<u32>(w(b))),
            0,
        ]
    };
    g.write::<u32>(base.wrapping_sub(0xc), 0);
    let x1 = xmm(g, 0x453, 0x44b);
    crate::s4_small::scale_task_arg(
        g,
        img,
        &callee(0x45f, base.wrapping_sub(0x10), w(0x45a), x1),
    );
    g.write::<u32>(base.wrapping_sub(0x10), 0x15e);
    let x2 = xmm(g, 0x46f, 0x467);
    stores(g, img, f, &[0x47d]);
    crate::s4_small::scale_task_arg(
        g,
        img,
        &callee(0x488, base.wrapping_sub(0x14), w(0x479), x2),
    );
    stores(g, img, f, &[0x491]);
    let mut st = Pushes::default();
    st.push_all(&[1, w(0x3f1), 0, 0x15e, 4]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 271), fr.ecx);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x4ab), w(0x4a6));
    let take = |g: &mut G, call: u32, owner: u32| {
        crate::sa_pool::take(g, img, img.rel32_target(f + call), w(owner))
    };
    // the first node (a byte 1) and three more (their `+0x14` words),
    // each after the one before
    let n0 = take(g, 0x4b8, 0x4b3);
    let mut first_set = rd(g, 0x4be) != 0;
    g.write::<u8>(n0.wrapping_add(0x18), 1);
    g.write::<u32>(n0.wrapping_add(4), 0);
    if first_set {
        let last = rd(g, 0x4d5);
        g.write::<u32>(n0, last);
        g.write::<u32>(last.wrapping_add(4), n0);
    } else {
        first_set = n0 != 0;
        g.write::<u32>(n0, 0);
        g.write::<u32>(w(0x4ea), n0);
    }
    let mut prev = n0;
    for (call, owner, word, first_op) in [
        (0x4f7u32, 0x4efu32, 0x0019_0000u32, 0x520u32),
        (0x52d, 0x525, 0x001a_003c, 0x556),
        (0x563, 0x55b, 0x000c_0069, 0x587),
    ] {
        g.write::<u32>(prev.wrapping_add(8), prev);
        let n = take(g, call, owner);
        g.write::<u32>(n.wrapping_add(0x14), word);
        g.write::<u32>(n.wrapping_add(4), 0);
        if first_set {
            g.write::<u32>(n, prev);
            g.write::<u32>(prev.wrapping_add(4), n);
        } else {
            first_set = n != 0;
            g.write::<u32>(n, 0);
            g.write::<u32>(w(first_op), n);
        }
        prev = n;
    }
    g.write::<u32>(prev.wrapping_add(8), prev);
    g.write::<u32>(w(0x594), prev);
    crate::sa_misc::records_init_100(g, img, img.rel32_target(f + 0x599), w(0x58c));
    let n = take(g, 0x5a3, 0x59e);
    let first = rd(g, 0x5a9);
    g.write::<u8>(n.wrapping_add(0x18), 2);
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x5d2), n);
    } else {
        let last = rd(g, 0x5c0);
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(w(0x5dd), n);
    Ret::eax(0)
}

/// [`start_up_0562`] (1518 bytes, 1 overlay; the `rel32`s in order).
pub const START_UP_0562: Sig = Sig {
    len: 1518,
    hash: 0xba75_27bb_c284_e745,
    calls: &[
        (0x57, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xf2, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x15c, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x1c9, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x1fb, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x27c, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x2fd, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x3df, Callee::Sig(&crate::s4_small::RECORDS_FROM_TABLE)),
        (0x400, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x443, Callee::Sig(&crate::s4_small::COUNTED_TASK_24)),
        (0x45f, Callee::Sig(&crate::s4_small::SCALE_TASK_ARG)),
        (0x488, Callee::Sig(&crate::s4_small::SCALE_TASK_ARG)),
        (0x4ab, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x4b8, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4f7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x52d, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x563, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x599, Callee::Sig(&crate::sa_misc::RECORDS_INIT_100)),
        (0x5a3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x5e6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0562`'s load step (0x100012b0, the 118-byte shape) running
/// [`start_up_0562`].
pub fn load_0562<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0562)
}

/// [`load_0562`].
pub const LOAD_0562: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0562))],
    imports: &[],
};

/// `magic_0603`'s model node start (0x1000cb10, an overlay table entry,
/// cdecl, no arguments; `ebp` = `esp - 4`): the static node `S` put last
/// in the DLL's first list (its words, the old last's `+4`, the list's
/// `+0x30` bit 0 cleared), then the second static node appended to the
/// node list `L` (pass halfword 0xff, an empty child list, the old last
/// `+8`, the list's first or the old last's `+4`, the count up), its
/// words and twelve more globals set; two model nodes set up
/// ([`model_node_init`](crate::sa_summon::model_node_init), a record on
/// the frame `[node, parent, object, flags 0]`: the first with the object
/// 1, the second with the byte `*G_B`), each node's vtable stored first. Returns
/// the second set-up's `eax`.
pub fn model_start_0603<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let byte_at = |g: &mut G, at: u32| g.write::<u8>(w(at + 2), w(at + 6) as u8);
    let half_at = |g: &mut G, at: u32| g.write::<u16>(w(at + 3), w(at + 7) as u16);
    let last = g.read::<u32>(w(0x4));
    g.write::<u32>(w(0x9), last);
    stores(g, img, f, &[0xd, 0x17, 0x21, 0x2b]);
    g.write::<u32>(last.wrapping_add(4), w(0x38));
    let tail = g.read::<u32>(w(0x3d));
    let v = g.read::<u32>(w(0x43)) & !1;
    g.write::<u32>(w(0x43), v);
    stores(g, img, f, &[0x4b]);
    byte_at(g, 0x55);
    half_at(g, 0x5c);
    stores(g, img, f, &[0x65, 0x6f, 0x79, 0x83]);
    g.write::<u32>(w(0x8e), tail);
    if tail == w(0x93) {
        g.write::<u32>(w(0x9b), w(0x9f));
    } else {
        g.write::<u32>(tail.wrapping_add(4), w(0xa8));
    }
    let n = g.read::<u16>(w(0xaf)).wrapping_add(1);
    g.write::<u16>(w(0xaf), n);
    stores(g, img, f, &[0xb6, 0xc0, 0xca, 0xd4, 0xde]);
    half_at(g, 0xe8);
    byte_at(g, 0xf1);
    stores(g, img, f, &[0xf8, 0x102, 0x10c, 0x116, 0x120, 0x12a, 0x134]);
    // the record at `ebp - 0x10`: node, parent, flags, a byte 0
    let rec = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(rec, w(0x141));
    g.write::<u32>(rec + 4, w(0x148));
    g.write::<u32>(rec + 8, 1);
    g.write::<u8>(rec + 0xc, 0);
    stores(g, img, f, &[0x157]);
    crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x162), rec);
    let b = u32::from(g.read::<u8>(w(0x169)));
    g.write::<u32>(rec, w(0x173));
    g.write::<u32>(rec + 4, w(0x17a));
    g.write::<u32>(rec + 8, b);
    g.write::<u8>(rec + 0xc, 0);
    stores(g, img, f, &[0x185]);
    let eax = crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x190), rec);
    Ret::eax(eax)
}

/// [`model_start_0603`] (408 bytes, 1 overlay; the two set-ups).
pub const MODEL_START_0603: Sig = Sig {
    len: 408,
    hash: 0x1e16_054b_3e6a_a6cf,
    calls: &[
        (0x162, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x190, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
    ],
    imports: &[],
};

/// `magic_0610`'s sixth object and two light records (0x1000be90, an
/// overlay table entry, no arguments; `ebp` = `esp - 4`; the block `S =
/// *G_S`, operand `+0xe`): slot 282 `()` gives `v` (ST0, kept as f32 at
/// `ebp - 4`); the object `S + 0x720`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale `K1
/// / v * K2` with `K1`, `K2` at `+0x26`, `+0x36`, index 6), `S + 0x824` =
/// the callback (`+0x53`) and byte `S + 0x846` = 2. The light records
/// `P1 = S + 0x1450` (white, count 4) and `P2 = S + 0x14f0` (count 0x19)
/// get their colours, counts 0x14 and kinds; each gets a pool node
/// ([`take`](crate::sa_pool::take), owners `+0x62`, `+0x105`) with `+0x14
/// = P`, the first appended to the task list (`P1 + 0x94` 1 after), the
/// second after it (the list's last). Returns the second node.
pub fn lights_start_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let t = slot_target(g, w(0x7), 282);
    let s = g.read::<u32>(w(0xe));
    let v = crate::m0018::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), v);
    let scale = mul(div(g.read::<f32>(w(0x26)), v), g.read::<f32>(w(0x36)));
    // push 6 (`6a 06`); lea edx, [ebx + 0x720]
    let at = img.rel32_target(f + 0x43);
    crate::s4_misc::start_object_plain(g, img, at, s.wrapping_add(0x720), scale, w(0x3b) & 0xff);
    g.write::<u32>(s.wrapping_add(0x824), w(0x53));
    g.write::<u8>(s.wrapping_add(0x846), 2);
    let p1 = s.wrapping_add(0x1450);
    g.write::<u32>(p1.wrapping_add(0x80), 0);
    g.write::<u16>(p1.wrapping_add(0x85), 0xffff);
    g.write::<u8>(p1.wrapping_add(0x84), 0xff);
    g.write::<u8>(p1.wrapping_add(0x87), 0x80);
    g.write::<u32>(p1.wrapping_add(0x88), 4);
    g.write::<u16>(p1.wrapping_add(0x98), 0x48);
    g.write::<u32>(p1.wrapping_add(0x90), 0x14);
    g.write::<u32>(p1.wrapping_add(0x8c), 0x14);
    g.write::<u32>(p1.wrapping_add(0x94), 0);
    let n1 = crate::sa_pool::take(g, img, img.rel32_target(f + 0xb9), w(0x62));
    let mut first = g.read::<u32>(w(0xbf));
    g.write::<u32>(n1.wrapping_add(0x14), p1);
    g.write::<u32>(n1.wrapping_add(4), 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0xea), n1);
    } else {
        let last = g.read::<u32>(w(0xd5));
        g.write::<u32>(n1, last);
        g.write::<u32>(last.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    g.write::<u32>(p1.wrapping_add(0x94), 1);
    let p2 = s.wrapping_add(0x14f0);
    g.write::<u32>(p2.wrapping_add(0x81), 0x0080_ffff);
    g.write::<u8>(p2.wrapping_add(0x80), 0xff);
    g.write::<u16>(p2.wrapping_add(0x86), 0);
    g.write::<u8>(p2.wrapping_add(0x85), 0);
    g.write::<u32>(p2.wrapping_add(0x88), 0x19);
    g.write::<u16>(p2.wrapping_add(0x98), 0x48);
    g.write::<u32>(p2.wrapping_add(0x90), 0x14);
    g.write::<u32>(p2.wrapping_add(0x8c), 0x14);
    g.write::<u32>(p2.wrapping_add(0x94), 0);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x15c), w(0x105));
    g.write::<u32>(n2.wrapping_add(0x14), p2);
    g.write::<u32>(n2.wrapping_add(4), 0);
    g.write::<u32>(w(0x16b), n2);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x18b), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1.wrapping_add(4), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    Ret::eax(n2)
}

/// [`lights_start_0610`] (407 bytes, 1 overlay; the object start and the
/// two takes).
pub const LIGHTS_START_0610: Sig = Sig {
    len: 407,
    hash: 0x5c51_2d8a_8c19_19bf,
    calls: &[
        (0x43, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xb9, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x15c, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
