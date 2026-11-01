//! The register thunks of the node families, ported by the shared-3 list.
//!
//! The node tables (`.data`) point at small thunks that move their stack
//! arguments into the registers an internal handler takes and jump (or
//! call) there:
//!
//! - `push ebp; mov ebp, esp; mov ecx, [ebp + 8]; pop ebp; jmp X` (12
//!   bytes): `X` gets the cursor in `ecx` and the caller's stack;
//! - the same with `mov edx, [ebp + 0xc]` first (15 bytes): `ecx` and `edx`;
//! - `push ebp; mov ebp, esp; push ecx; mov ecx, [ebp + 8]; call X; pop
//!   ecx; pop ebp; ret` (15 bytes): a call, `X`'s `eax` returned.
//!
//! The bytes are the same in every thunk of a shape; the callee tells
//! them apart, so each callee is its own code identity (`Sig` with
//! `Callee::Sig`). One entry per shape serves them all: it finds which
//! handler the `rel32` reaches (the [`TARGETS`] table, by code identity)
//! and runs its port on the frame the original's callee sees (`at` the
//! callee, `esp` the caller's return address for a jump, the thunk's own
//! frame below it for a call). [`crate::nodes`] keeps its four older
//! thunks of the same shape.

use crate::{
    Callee, Frame, Game, Image, Ret, Sig, s3_matrix, sa_misc, sa_summon, sb_matrix, sb_misc,
};
use crate::{sb_nodes, sb_ppp};

/// The masked hash of the 12-byte `mov ecx, [ebp + 8]; jmp` thunk (the
/// `rel32` zeroed), the same for every callee.
pub(crate) const ECX_JMP_HASH: u64 = 0x896f_4fde_1224_a07f;
/// The 15-byte `mov edx, [ebp + 0xc]; mov ecx, [ebp + 8]; jmp` thunk.
pub(crate) const ECX_EDX_JMP_HASH: u64 = 0x3c4e_32f4_8f4d_8fed;
/// The 15-byte `push ecx; mov ecx, [ebp + 8]; call; pop ecx` thunk.
pub(crate) const ECX_CALL_HASH: u64 = 0xd192_63e1_ae73_f895;

/// A handler's port on the frame its thunk hands it.
type Handler<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// The handlers the thunks reach: each one's code identity and its port
/// on the callee's frame (`at` the handler, `ecx`/`edx` the registers the
/// thunk loaded, `esp` at the return address the handler returns to).
fn targets<G: Game>() -> [(&'static Sig, Handler<G>); 28] {
    [
        (
            &crate::s9_misc::NODE_SPARKS,
            crate::s9_misc::node_sparks::<G>,
        ),
        (&crate::s9_misc::NODE_EASE, crate::s9_misc::node_ease::<G>),
        (&crate::s9_misc::NODE_EASE_B, crate::s9_misc::node_ease::<G>),
        (&sa_summon::PLACE, sa_summon::place::<G>),
        (&sa_summon::MODEL_SETUP, sa_summon::model_setup::<G>),
        (&sa_summon::SUB_WALK, sa_summon::sub_walk::<G>),
        (&sa_summon::SCALED_PLACE, sa_summon::scaled_place::<G>),
        (&sa_summon::VECTOR_OP, sa_summon::vector_op::<G>),
        (&sa_summon::BOUNCE, sa_summon::bounce::<G>),
        (&sa_summon::PAIR_AIM, sa_summon::pair_aim::<G>),
        (&sa_summon::BONE, sa_summon::bone_plain::<G>),
        (&sa_summon::BONE_SCALED, sa_summon::bone_scaled::<G>),
        (&sa_misc::ADD_TO_PARENT, sa_misc::add_to_parent::<G>),
        (&sa_misc::BLUR_SETUP, sa_misc::blur_setup::<G>),
        (&sb_nodes::VISIT_PAIR_229, |g, img, fr| {
            Ret::eax(sb_nodes::visit_pair_229(
                g, img, fr.at, fr.esp, fr.ecx, fr.eax,
            ))
        }),
        (&sb_nodes::VISIT_ADD_VECTOR, |g, _, fr| {
            Ret::eax(sb_nodes::visit_add_vector(g, fr.ecx, fr.eax))
        }),
        (&sb_nodes::VISIT_OFFSET_INT, |g, _, fr| {
            Ret::eax(sb_nodes::visit_offset_int(g, fr.esp, fr.ecx, fr.eax))
        }),
        (&sb_nodes::VISIT_OFFSET_FLOAT, |g, _, fr| {
            Ret::eax(sb_nodes::visit_offset_float(g, fr.esp, fr.ecx, fr.eax))
        }),
        (&sb_nodes::AXIS_SPRITES, |g, img, fr| {
            Ret::eax(sb_nodes::axis_sprites(g, img, fr.at, fr.esp, fr.ecx))
        }),
        (&sb_nodes::BOUNCE, |g, img, fr| {
            Ret::eax(sb_nodes::bounce(g, img, fr.at, fr.esp, fr.ecx))
        }),
        (&sb_nodes::DRAW_MODEL, |g, img, fr| {
            Ret::eax(sb_nodes::draw_model(g, img, fr.at, fr.esp, fr.ecx))
        }),
        (&sb_ppp::RUN_PART, |g, img, fr| {
            Ret::eax(sb_ppp::run_part(g, img, fr.at, fr.esp, fr.ecx))
        }),
        (&sb_misc::AIM_PARENT, |g, img, fr| {
            Ret::eax(sb_misc::aim_parent(g, img, fr.at, fr.esp, fr.ecx))
        }),
        (&sb_matrix::ROTATION_52A0, |g, img, fr| {
            Ret::eax(sb_matrix::rotation_52a0(g, img, fr.at, fr.ecx, fr.edx))
        }),
        (&sb_matrix::ROTATION_19D0, |g, img, fr| {
            Ret::eax(sb_matrix::rotation_19d0(g, img, fr.at, fr.ecx, fr.edx))
        }),
        (&s3_matrix::ROTATION_47C0, |g, img, fr| {
            Ret::eax(s3_matrix::rotation_47c0(g, img, fr.at, fr.ecx, fr.edx))
        }),
        (&s3_matrix::ROTATION_4D40, |g, img, fr| {
            Ret::eax(s3_matrix::rotation_4d40(g, img, fr.at, fr.ecx, fr.edx))
        }),
        (&s3_matrix::ROTATION_4250, |g, img, fr| {
            Ret::eax(s3_matrix::rotation_4250(g, img, fr.at, fr.ecx, fr.edx))
        }),
    ]
}

/// The handler at `target`, run on `fr` (`at` set to it). A target no
/// entry of [`targets`] matches cannot be bound (the thunk's `Sig`
/// names its callee), so it returns the frame's `eax`.
fn run<G: Game>(g: &mut G, img: &Image, target: u32, fr: Frame) -> Ret {
    let fr = Frame { at: target, ..fr };
    targets::<G>()
        .iter()
        .find(|(sig, _)| img.matches(target, sig))
        .map_or(Ret::eax(fr.eax), |(_, h)| h(g, img, &fr))
}

/// The 12-byte thunk: `ecx` = the first stack word, then a jump to the
/// handler with the caller's stack (its `eax`, nothing popped).
pub fn ecx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    run(g, img, target, Frame { ecx, ..*fr })
}

/// The 15-byte two-register thunk: `edx` = the second stack word, `ecx`
/// the first, then the jump.
pub fn ecx_edx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let edx = fr.arg(g, 1);
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 11);
    run(g, img, target, Frame { ecx, edx, ..*fr })
}

/// The 15-byte calling thunk: its frame (`ebp`, then the caller's `ecx`
/// pushed, a word no handler reads) and a call with `ecx` = the first stack word; the handler's
/// `eax`, nothing popped.
pub fn ecx_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    // below `ebp` (esp - 4) and the pushed `ecx` (esp - 8): the call's
    // return address
    let esp = fr.esp.wrapping_sub(12);
    let r = run(g, img, target, Frame { esp, ecx, ..*fr });
    Ret::eax(r.eax)
}

macro_rules! thunks {
    ($($(#[$doc:meta])* $name:ident: $len:literal, $hash:ident, $off:literal => $callee:path;)*) => {
        $(
            $(#[$doc])*
            pub const $name: Sig = Sig {
                len: $len,
                hash: $hash,
                calls: &[($off, Callee::Sig(&$callee))],
                imports: &[],
            };
        )*
    };
}

thunks! {
    /// `magic_0003` 0x10002f20 (116 DLLs) over [`sa_summon::place`].
    JMP_PLACE: 12, ECX_JMP_HASH, 8 => sa_summon::PLACE;
    /// `magic_0003` 0x10002a60 (116 DLLs) over [`sa_summon::model_setup`].
    JMP_MODEL_SETUP: 12, ECX_JMP_HASH, 8 => sa_summon::MODEL_SETUP;
    /// `magic_0003` 0x10004050 (116 DLLs) over [`sa_summon::sub_walk`].
    JMP_SUB_WALK: 12, ECX_JMP_HASH, 8 => sa_summon::SUB_WALK;
    /// `magic_0003` 0x100031b0 (116 DLLs) over [`sa_summon::scaled_place`].
    JMP_SCALED_PLACE: 12, ECX_JMP_HASH, 8 => sa_summon::SCALED_PLACE;
    /// `magic_0003` 0x100030f0 (115 DLLs) over [`sa_summon::vector_op`].
    JMP_VECTOR_OP: 12, ECX_JMP_HASH, 8 => sa_summon::VECTOR_OP;
    /// `magic_0003` 0x10006020 (107 DLLs; `magic_0087`/`0088` 0x100061a0)
    /// over [`sa_summon::bounce`].
    JMP_SUMMON_BOUNCE: 12, ECX_JMP_HASH, 8 => sa_summon::BOUNCE;
    /// `magic_0003` 0x10004240 (107 DLLs; `magic_0087`/`0088` 0x10004490)
    /// over [`sa_summon::pair_aim`].
    JMP_PAIR_AIM: 12, ECX_JMP_HASH, 8 => sa_summon::PAIR_AIM;
    /// `magic_0003` 0x10002e80 (105 DLLs; `magic_0087`/`0088` 0x100030c0)
    /// over [`sa_summon::bone_scaled`].
    JMP_BONE_SCALED: 12, ECX_JMP_HASH, 8 => sa_summon::BONE_SCALED;
    /// `magic_0042` 0x10004f50 (14 DLLs) over [`sa_misc::add_to_parent`].
    JMP_ADD_TO_PARENT: 12, ECX_JMP_HASH, 8 => sa_misc::ADD_TO_PARENT;
    /// `magic_0070` 0x10001150 over [`sa_misc::blur_setup`].
    JMP_BLUR_SETUP: 12, ECX_JMP_HASH, 8 => sa_misc::BLUR_SETUP;
    /// `magic_0003` 0x10003fb0 (116 DLLs) over [`sb_nodes::visit_pair_229`].
    JMP_VISIT_PAIR_229: 12, ECX_JMP_HASH, 8 => sb_nodes::VISIT_PAIR_229;
    /// `magic_0003` 0x10006240 (116 DLLs) over [`sb_nodes::visit_add_vector`].
    JMP_VISIT_ADD_VECTOR: 12, ECX_JMP_HASH, 8 => sb_nodes::VISIT_ADD_VECTOR;
    /// `magic_0003` 0x10003b60 (116 DLLs) over [`sb_nodes::visit_offset_int`].
    JMP_VISIT_OFFSET_INT: 12, ECX_JMP_HASH, 8 => sb_nodes::VISIT_OFFSET_INT;
    /// `magic_0003` 0x10003c10 (116 DLLs) over
    /// [`sb_nodes::visit_offset_float`].
    JMP_VISIT_OFFSET_FLOAT: 12, ECX_JMP_HASH, 8 => sb_nodes::VISIT_OFFSET_FLOAT;
    /// `magic_0003` 0x10003580 (107 DLLs; `magic_0087`/`0088` 0x100037c0)
    /// over [`sb_nodes::axis_sprites`].
    JMP_AXIS_SPRITES: 12, ECX_JMP_HASH, 8 => sb_nodes::AXIS_SPRITES;
    /// `magic_0003` 0x100061a0 (107 DLLs; `magic_0087`/`0088` 0x10006320)
    /// over [`sb_nodes::bounce`].
    JMP_NODE_BOUNCE: 12, ECX_JMP_HASH, 8 => sb_nodes::BOUNCE;
    /// `magic_0003` 0x10003e70 (103 DLLs; `magic_0087`/`0088` 0x100040c0)
    /// over [`sb_nodes::draw_model`].
    JMP_DRAW_MODEL: 12, ECX_JMP_HASH, 8 => sb_nodes::DRAW_MODEL;
    /// `magic_0003` 0x10003a20 (95 DLLs; `magic_0087`/`0088` 0x10003c60)
    /// over [`sb_ppp::run_part`].
    JMP_RUN_PART: 12, ECX_JMP_HASH, 8 => sb_ppp::RUN_PART;
    /// `magic_0042` 0x100051a0 (10 DLLs) over [`sb_misc::aim_parent`].
    JMP_AIM_PARENT: 12, ECX_JMP_HASH, 8 => sb_misc::AIM_PARENT;
    /// `magic_0155` 0x100037c0 (4 DLLs) over [`crate::s9_misc::node_ease`].
    JMP_NODE_EASE: 12, ECX_JMP_HASH, 8 => crate::s9_misc::NODE_EASE;
    /// `magic_0570` 0x100046d0 (2 DLLs) over [`crate::s9_misc::node_ease`].
    JMP_NODE_EASE_B: 12, ECX_JMP_HASH, 8 => crate::s9_misc::NODE_EASE_B;
    /// `magic_0070` 0x10001fe0 (3 DLLs) over [`crate::s9_misc::node_sparks`].
    JMP_NODE_SPARKS: 12, ECX_JMP_HASH, 8 => crate::s9_misc::NODE_SPARKS;
    /// `magic_0003` 0x10005b90 (116 DLLs) over [`sb_matrix::rotation_52a0`].
    JMP_ROTATION_52A0: 15, ECX_EDX_JMP_HASH, 11 => sb_matrix::ROTATION_52A0;
    /// `magic_0003` 0x10005b50 (116 DLLs) over [`sb_matrix::rotation_19d0`].
    JMP_ROTATION_19D0: 15, ECX_EDX_JMP_HASH, 11 => sb_matrix::ROTATION_19D0;
    /// `magic_0003` 0x10005c10 (107 DLLs; `magic_0087`/`0088` 0x10005e60)
    /// over [`sa_summon::bone_plain`].
    /// `magic_0003` 0x10005b80 (116 DLLs) over
    /// [`s3_matrix::rotation_47c0`].
    JMP_ROTATION_47C0: 15, ECX_EDX_JMP_HASH, 11 => s3_matrix::ROTATION_47C0;
    /// `magic_0003` 0x10005b60 (116 DLLs) over
    /// [`s3_matrix::rotation_4d40`].
    JMP_ROTATION_4D40: 15, ECX_EDX_JMP_HASH, 11 => s3_matrix::ROTATION_4D40;
    /// `magic_0003` 0x10005b70 (116 DLLs) over
    /// [`s3_matrix::rotation_4250`].
    JMP_ROTATION_4250: 15, ECX_EDX_JMP_HASH, 11 => s3_matrix::ROTATION_4250;
    CALL_BONE: 15, ECX_CALL_HASH, 8 => sa_summon::BONE;
}
