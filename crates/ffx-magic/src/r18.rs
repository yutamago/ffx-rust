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
    let e = rd(g, 0x13);
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let mut st = Pushes::default();
    st.push_all(&[pz, py, px, rd(g, 0x2a)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, slot(g, 0x24, 262), fr.ecx);
    let (x, y, z) = (g.read::<u32>(px), g.read::<u32>(py), g.read::<u32>(pz));
    g.write::<u32>(e.wrapping_add(0xc04), x);
    g.write::<u32>(e.wrapping_add(0xc08), y);
    g.write::<u32>(e.wrapping_add(0xc0c), z);
    g.write::<u32>(e.wrapping_add(0xc10), 0x3f80_0000);
    st.push(rd(g, 0x6e));
    g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
    // call width: 12 (Pushes caps the words at 12)
    let t = st0_f32(st.call(g, slot(g, 0x3c, 333), fr.ecx).st0);
    st.pop(5);
    let mut yaw = sub(k(g, 0x88), t);
    if yaw > k(g, 0x97) {
        yaw = sub(yaw, k(g, 0xa6));
    } else if k(g, 0xb0) > yaw {
        yaw = add(yaw, k(g, 0xbd));
    }
    g.write::<f32>(ebp.wrapping_sub(0x2c), yaw);
    for (o, v) in [
        (0xbc4u32, 0),
        (0xbc8, yaw.to_bits()),
        (0xbcc, 0),
        (0xbd0, 0),
        (0xbd4, x),
        (0xbd8, y),
        (0xbdc, z),
        (0xbe0, 0x3f80_0000),
    ] {
        g.write::<u32>(e.wrapping_add(o), v);
    }
    let a = rd(g, 0x122);
    let m = e.wrapping_add(0xac4);
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
    let d0 = mul(v, g.read::<f32>(m));
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
