//! Round 21's ports (magic-r21): the shared rows and the own rows of
//! overlays below `magic_0450`, mostly copies of earlier ports that now
//! read their differing operands from the code.

use crate::{Callee, Sig};

/// `magic_0304`'s caster and summon placement (0x100015d0, 1,588
/// bytes): [`place_pair`](crate::m0596::place_pair) with the state's
/// fields 0x1048 higher.
pub const PLACE_PAIR_0304: Sig = Sig {
    len: 1588,
    hash: 0x9df1_b64f_333c_0110,
    // __security_check_cookie
    calls: &[(0x62c, Callee::Any)],
    imports: &[],
};

/// `magic_0430`'s wave tick (0x10002580, 457 bytes): the code of
/// [`wave_tick_0588`](crate::r18::wave_tick_0588) with the count and
/// state at `E + 0xeb8`/`0xebc`.
pub const WAVE_TICK_0430: Sig = Sig {
    len: 457,
    hash: 0xbfcf_266e_f1f6_af01,
    calls: &[
        (0x149, Callee::Sig(&crate::s5_misc::WAVE_TABLE)),
        (0x1c1, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s twenty model passes (0x100039e0, 400 bytes): the code
/// of [`passes_0586`](crate::r18::passes_0586) with the angle, point,
/// translation and records 0x60 further into the scene.
pub const PASSES_0162: Sig = Sig {
    len: 400,
    hash: 0x0e44_80d0_e415_a72b,
    calls: &[
        (
            0x16a,
            Callee::Sig(&crate::o6_fam_pairs4::THREE_PASS_MODEL_0162_1A80),
        ),
        (0x188, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0434`'s summon frame (0x10005730, 226 bytes):
/// [`summon_frame_fade`](crate::s8_misc::summon_frame_fade) in
/// `magic_0610`'s shape, its own displacements and scene end.
pub const SUMMON_FRAME_0434: Sig = Sig {
    len: 226,
    hash: 0x4066_581a_4e6b_7643,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xb2, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xbc, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xc4, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (
            0xd3,
            Callee::Sig(&crate::o6_fam_pairs6::SCENE_END_0434_5B90),
        ),
        (0xde, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0434`'s summon matrix set-up (0x10006610, 627 bytes):
/// [`summon_matrix_0610`](crate::r20::summon_matrix_0610) with the
/// colour show called before slot 257.
pub const SUMMON_MATRIX_0434: Sig = Sig {
    len: 627,
    hash: 0x83ef_6267_6166_da64,
    calls: &[
        (0x23d, Callee::Sig(&crate::m0434::COLOUR_SHOW)),
        // __security_check_cookie
        (0x26b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0434`'s sixth object and light records (0x10007fc0, 409
/// bytes): [`lights_start_0610`](crate::r18::lights_start_0610) with
/// the animated object start.
pub const LIGHTS_START_0434: Sig = Sig {
    len: 409,
    hash: 0xc32c_4cc5_bf72_ee55,
    calls: &[
        (0x45, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xbb, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x15e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0347`'s summon object between two actors (0x10003c60, 1,336
/// bytes): [`summon_object_between`](crate::s9_misc::summon_object_between)
/// with the animated object start and its callback word.
pub const SUMMON_OBJECT_BETWEEN_0347: Sig = Sig {
    len: 1336,
    hash: 0xe618_41c7_8e9f_21e3,
    calls: &[
        (0xd3, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x2ce, Callee::Any),
        (0x4ff, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0x530, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0347`'s burst start (0x100041a0, 845 bytes):
/// [`burst_0491`](crate::r20::burst_0491) with slot 618's words in
/// another order and the indices swapped.
pub const BURST_0347: Sig = Sig {
    len: 845,
    hash: 0x3bbb_c2d7_f22f_5f75,
    calls: &[
        (0xcc, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x179, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x345, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0347`'s spark task (0x100044f0, 934 bytes): the code of
/// [`spark_task`](crate::r14::spark_task) with its own object between
/// and `magic_0491`'s burst.
pub const SPARK_TASK_0347: Sig = Sig {
    len: 934,
    hash: 0xe954_e609_449f_510a,
    calls: &[
        (0x34, Callee::Sig(&SUMMON_OBJECT_BETWEEN_0347)),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x217, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x222, Callee::Any),
        (0x236, Callee::Sig(&BURST_0347)),
        (0x242, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x390, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x39e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s caster object (0x10002900, 349 bytes):
/// [`caster_object_0588`](crate::r18::caster_object_0588) with the
/// animated object start and its own block offsets.
pub const CASTER_OBJECT_0430: Sig = Sig {
    len: 349,
    hash: 0xa469_85b7_a7eb_a071,
    calls: &[
        (0x30, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xe6, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x155, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0388`'s colour object set-up (0x10006ba0, 460 bytes):
/// [`colour_setup_0645`](crate::r20::colour_setup_0645) with the alpha
/// from `xmm2`.
pub const COLOUR_SETUP_0388: Sig = Sig {
    len: 460,
    hash: 0x161e_a27c_b2d8_06e7,
    calls: &[
        (0x17c, Callee::Sig(&crate::nodes::INSERT)),
        (0x1ba, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0393`'s caster object start (0x10001b50, 689 bytes): the code
/// of [`caster_start_0450`](crate::r16::caster_start_0450) with the
/// object at `B+0xbe0` and index 10 (the `lea` and `push` operands).
pub const CASTER_START_0393: Sig = Sig {
    len: 689,
    hash: 0x8794_6a44_5809_aef2,
    calls: &[
        (0x31, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x299, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x2a9, Callee::Any),
    ],
    imports: &[],
};
