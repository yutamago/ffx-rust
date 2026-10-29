//! Small entries the summon overlays repeat with their own state block
//! layout (the field offsets are the instructions' displacements, read
//! from the code like the globals): the task steps that end the effect,
//! the weapon afterimage and its fade. One function each, bound in every
//! DLL by that DLL's signature (the code differs only in the
//! displacements).

use crate::l5_objhost as host;
use crate::ob_model::{st0_f32, wrap_once};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (`magic_0394` 0x10004890, no arguments): nothing while
/// the game is paused (slot 561's result returned); runs the objects
/// ([`host::run_objects`]) and, once none ran and the state's done word
/// (`*G_STATE + D`, `D` the displacement at `+0x24`) is 1, ends the
/// effect (slot 214 `(G_HANDLE)`, its `eax` returned).
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s561 = slot_target(g, w(0x01), 561);
    let state = g.read::<u32>(w(0x0e));
    let p = g.call(s561, 0, &[]).eax;
    if p != 0 {
        return Ret::eax(p);
    }
    // push esi; call
    let r = host::run_objects(g, img, img.rel32_target(f + 0x19), fr.esp - 8);
    // cmp [esi + D], eax
    if r != 1 || g.read::<u32>(state.wrapping_add(w(0x24))) != 1 {
        return Ret::eax(r);
    }
    let s214 = slot_target(g, w(0x2b), 214);
    let h = g.read::<u32>(w(0x31));
    Ret::eax(g.call(s214, 0, &[h]).eax)
}

/// A task step without the pause check (`magic_0596` 0x100069d0, no
/// arguments): runs the objects ([`host::run_objects`]); once none ran
/// and the state's done word (`*G_STATE + D`, the displacement at
/// `+0x13`) is 1, ends the effect (slot 214 `(G_HANDLE)`, its `eax`
/// returned). Returns `run_objects`' result otherwise.
pub fn step_plain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x03));
    // push esi; call
    let r = host::run_objects(g, img, img.rel32_target(f + 0x08), fr.esp - 8);
    // cmp [esi + D], eax
    if r != 1 || g.read::<u32>(state.wrapping_add(w(0x13))) != 1 {
        return Ret::eax(r);
    }
    let s214 = slot_target(g, w(0x1a), 214);
    let h = g.read::<u32>(w(0x20));
    Ret::eax(g.call(s214, 0, &[h]).eax)
}

/// A callback (`magic_0394` 0x10004b10, no arguments): the caster's
/// weapon afterimage started (slot 623 `(*G_CASTER, 0, 0, 0, 0, 0, 0)`),
/// its handle kept in `*G_STATE + D` (the displacement at `+0x2b`) and
/// returned.
pub fn afterimage<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s623 = slot_target(g, w(0x01), 623);
    let state = g.read::<u32>(w(0x0e));
    let c = g.read::<u32>(w(0x20));
    let h = g.call(s623, 0, &[c, 0, 0, 0, 0, 0, 0]).eax;
    g.write::<u32>(state.wrapping_add(w(0x2b)), h);
    Ret::eax(h)
}

/// A callback (`magic_0394` 0x10004b50, no arguments): the afterimage
/// `*G_STATE + D` (the displacements at `+0x14` and `+0x1f`) fades (slot
/// 624) and the handle becomes -1. Returns slot 624's `eax`.
pub fn afterimage_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x08));
    let s624 = slot_target(g, w(0x01), 624);
    let h = g.read::<u32>(state.wrapping_add(w(0x14)));
    let r = g.call(s624, 0, &[h]).eax;
    g.write::<u32>(state.wrapping_add(w(0x1f)), 0xffff_ffff);
    Ret::eax(r)
}

/// [`step`] in `magic_0394` (0x10004890, 66 bytes).
pub const STEP_0394: Sig = Sig {
    len: 66,
    hash: 0x77a6_7d5a_2ee5_9017,
    calls: &[(0x19, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step`] in `magic_0395` (0x100041b0, 66 bytes).
pub const STEP_0395: Sig = Sig {
    len: 66,
    hash: 0xebbb_dabe_41bc_edd0,
    calls: &[(0x19, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step`] in `magic_0396` (0x10004b70, 66 bytes).
pub const STEP_0396: Sig = Sig {
    len: 66,
    hash: 0x46b3_7bf7_e372_3e5b,
    calls: &[(0x19, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step`] in `magic_0397` (0x100042b0, 66 bytes).
pub const STEP_0397: Sig = Sig {
    len: 66,
    hash: 0xf20f_3e27_74a5_1deb,
    calls: &[(0x19, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0596` (0x100069d0, 49 bytes).
pub const STEP_PLAIN_0596: Sig = Sig {
    len: 49,
    hash: 0x4be4_761c_c31b_904e,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0430` (0x10001890, 49 bytes).
pub const STEP_PLAIN_0430: Sig = Sig {
    len: 49,
    hash: 0xd860_fff0_cc84_c44e,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0431` (0x10001720, 49 bytes).
pub const STEP_PLAIN_0431: Sig = Sig {
    len: 49,
    hash: 0xdd91_e15e_719b_cbd3,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0432` (0x10001350, 49 bytes).
pub const STEP_PLAIN_0432: Sig = Sig {
    len: 49,
    hash: 0xbcb8_75a5_b6f1_f44a,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0434` (0x10005820, 49 bytes).
pub const STEP_PLAIN_0434: Sig = Sig {
    len: 49,
    hash: 0xee90_e4cd_208f_e0d6,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0447` (0x10004af0, 49 bytes).
pub const STEP_PLAIN_0447: Sig = Sig {
    len: 49,
    hash: 0xdcbe_4ea8_3fb6_2c49,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0511` (0x100015d0, 49 bytes).
pub const STEP_PLAIN_0511: Sig = Sig {
    len: 49,
    hash: 0x47b0_f10b_fd45_cce6,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0586` (0x10002130, 49 bytes).
pub const STEP_PLAIN_0586: Sig = Sig {
    len: 49,
    hash: 0x7ce6_07d2_453f_a01f,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0587` (0x10001590, 49 bytes).
pub const STEP_PLAIN_0587: Sig = Sig {
    len: 49,
    hash: 0x99ea_9a68_a665_235f,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0588` (0x100017e0, 49 bytes).
pub const STEP_PLAIN_0588: Sig = Sig {
    len: 49,
    hash: 0x90aa_76d2_f953_8a8e,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0589` (0x100015a0, 49 bytes).
pub const STEP_PLAIN_0589: Sig = Sig {
    len: 49,
    hash: 0xa773_a108_3d3d_0286,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0598` (0x10006cb0, 49 bytes).
pub const STEP_PLAIN_0598: Sig = Sig {
    len: 49,
    hash: 0xa961_e737_2085_0b57,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0599` (0x10005fd0, 49 bytes).
pub const STEP_PLAIN_0599: Sig = Sig {
    len: 49,
    hash: 0x749e_11e5_e6bc_f53f,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0610` (0x1000a7c0, 49 bytes).
pub const STEP_PLAIN_0610: Sig = Sig {
    len: 49,
    hash: 0x0d62_9935_3a96_3c62,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0638` (0x100031a0, 49 bytes).
pub const STEP_PLAIN_0638: Sig = Sig {
    len: 49,
    hash: 0xc019_d409_2256_fb1f,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0673` (0x10001370, 49 bytes).
pub const STEP_PLAIN_0673: Sig = Sig {
    len: 49,
    hash: 0xd154_ecce_7d66_b102,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`step_plain`] in `magic_0689` (0x100031a0, 49 bytes).
pub const STEP_PLAIN_0689: Sig = Sig {
    len: 49,
    hash: 0x5b70_7327_7fe3_0905,
    calls: &[(0x8, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// [`afterimage`] in `magic_0394` (0x10004b10, 49 bytes).
pub const AFTERIMAGE_0394: Sig = Sig {
    len: 49,
    hash: 0x293c_b2c8_f2ca_3556,
    calls: &[],
    imports: &[],
};

/// [`afterimage`] in `magic_0395` (0x10004340, 49 bytes).
pub const AFTERIMAGE_0395: Sig = Sig {
    len: 49,
    hash: 0xa6bc_70bb_f28e_63b5,
    calls: &[],
    imports: &[],
};

/// [`afterimage`] in `magic_0396` (0x10004d20, 49 bytes).
pub const AFTERIMAGE_0396: Sig = Sig {
    len: 49,
    hash: 0x2d55_7046_7d5a_0536,
    calls: &[],
    imports: &[],
};

/// [`afterimage`] in `magic_0397` (0x100044f0, 49 bytes).
pub const AFTERIMAGE_0397: Sig = Sig {
    len: 49,
    hash: 0x5e3a_af33_c72e_6fc6,
    calls: &[],
    imports: &[],
};

/// [`afterimage`] in `magic_0448` (0x10004e80, 49 bytes).
pub const AFTERIMAGE_0448: Sig = Sig {
    len: 49,
    hash: 0xe84b_f82b_f150_d1ae,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0394` (0x10004b50, 41 bytes).
pub const AFTERIMAGE_FADE_0394: Sig = Sig {
    len: 41,
    hash: 0x31fc_b09f_9463_141f,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0395` (0x10004920, 41 bytes).
pub const AFTERIMAGE_FADE_0395: Sig = Sig {
    len: 41,
    hash: 0xc588_604f_5e78_14f9,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0396` (0x10004d60, 41 bytes).
pub const AFTERIMAGE_FADE_0396: Sig = Sig {
    len: 41,
    hash: 0x3aec_ff6b_2aa7_ae2f,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0397` (0x10004530, 41 bytes).
pub const AFTERIMAGE_FADE_0397: Sig = Sig {
    len: 41,
    hash: 0xe117_e591_bb6a_44ef,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0447` (0x100054b0, 41 bytes).
pub const AFTERIMAGE_FADE_0447: Sig = Sig {
    len: 41,
    hash: 0x02c4_7529_f915_0149,
    calls: &[],
    imports: &[],
};

/// [`afterimage_fade`] in `magic_0448` (0x100074b0, 41 bytes).
pub const AFTERIMAGE_FADE_0448: Sig = Sig {
    len: 41,
    hash: 0x21e3_c879_3d51_481f,
    calls: &[],
    imports: &[],
};

/// The summon's facing (`magic_0395` 0x10004200, internal, no
/// arguments; the state block's fields are the displacements at `+0x0e`
/// (the position), `+0x87` (the facing), `+0x8d` (the afterimage) and
/// `+0x97` (the done word)): its position (slot 262 `(*G_SUMMON,
/// &pos..)`, `w` = 1), the facing `A - Ch_GetRot` (slot 333) wrapped
/// once; the afterimage = -1, the done word = 0, two flags. Returns slot
/// 333's `eax`.
pub fn face<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x08));
    let pos = state.wrapping_add(w(0x0e));
    let chr = g.read::<u32>(w(0x22));
    let s262 = slot_target(g, w(0x1b), 262);
    let mut stack = vec![chr, pos, pos + 4, pos + 8];
    g.call(s262, 0, &stack);
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    let s333 = slot_target(g, w(0x2f), 333);
    stack.insert(0, g.read::<u32>(w(0x3c)));
    // call width: 5 at most (the earlier calls' words stay on the stack)
    let out = g.call(s333, 0, &stack);
    let x = g.read::<f32>(w(0x4f)) - st0_f32(out.st0);
    let x = wrap_once(g, img, f, x, [0x5e, 0x68, 0x72, 0x7f]);
    g.write::<f32>(state.wrapping_add(w(0x87)), x);
    g.write::<u32>(state.wrapping_add(w(0x8d)), 0xffff_ffff);
    g.write::<u32>(state.wrapping_add(w(0x97)), 0);
    g.write::<u32>(w(0xa2), 1);
    g.write::<u32>(w(0xac), 0xffff_ffff);
    out.eax
}

/// [`face`] in `magic_0395` (0x10004200, 185 bytes).
pub const FACE_0395: Sig = Sig {
    len: 185,
    hash: 0xba51_00b8_0c19_e84b,
    calls: &[],
    imports: &[],
};

/// [`face`] in `magic_0396` (0x10004bc0, 185 bytes).
pub const FACE_0396: Sig = Sig {
    len: 185,
    hash: 0x37c9_d218_27a4_025f,
    calls: &[],
    imports: &[],
};
