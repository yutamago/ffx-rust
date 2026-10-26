//! Continuation round batch 1 cases: the six large `Bind()`s
//! (`ffx_prender::h2_binds2` tables on `ffx_prender::h1_bind`'s steps).

use super::*;

use ffx_prender::h1_bind;

/// A `Bind()` on `h1_bind`'s steps: every guard word random (each static
/// built or not), the flag words and stored words random, and each class member's owner
/// (`[member + 8]`) an object whose member list at `+0x4c` is empty or
/// holds two nodes, its name (`[member + 0xc]`) null or not, and
/// `findMember`'s answer 0 or not.
pub fn prep_h1_bind(e: &mut Env, rng: &mut Rng, b: &h1_bind::Bind) {
    fn walk(e: &mut Env, rng: &mut Rng, steps: &[h1_bind::Step], k: &mut u32) {
        use h1_bind::Step;
        for s in steps {
            match *s {
                Step::Guard(g, _, inner) => {
                    let g = e.rebase(g);
                    let v = match rng.below(8) {
                        0 => 0,
                        1 => u32::MAX,
                        _ => rng.next() & rng.next(),
                    };
                    e.w32(g, v);
                    walk(e, rng, inner, k);
                }
                Step::Or(at, _) | Step::And(at, _) | Step::Store(at, _) => {
                    let at = e.rebase(at);
                    let v = rng.next();
                    e.w32(at, v);
                }
                Step::Class { member, .. } => {
                    let m = e.rebase(member);
                    // owners in the pool's second half, 0x80 bytes each
                    let owner = e.pool + POOL_PTRS + 0x80 * (*k % 0x30);
                    *k += 1;
                    let head = owner + 0x4c;
                    if rng.chance(2) {
                        e.w32(head, head);
                        e.w32(head + 4, head);
                    } else {
                        let first = e.pool + POOL_PTRS + 0x1800 + 0x10 * (*k % 0x40);
                        let second = first + 8;
                        e.w32(head, first);
                        e.w32(first, second);
                        e.w32(second, head);
                        e.w32(head + 4, second);
                        e.w32(second + 4, first);
                        e.w32(first + 4, head);
                    }
                    e.w32(m + 8, owner);
                    let name = if rng.chance(3) { 0 } else { e.ptr(rng) | 1 };
                    e.w32(m + 0xc, name);
                }
                _ => {}
            }
        }
    }
    let mut k = 0;
    walk(e, rng, b.steps, &mut k);
    let found = if rng.chance(2) { 0 } else { e.ptr(rng) };
    e.set_ret(h1_bind::FIND_MEMBER, found);
}

/// Random words at `at` (the image words a `Bind` stores constants into,
/// read from its machine code, so that a store the table misses shows).
pub fn prep_words(e: &mut Env, rng: &mut Rng, at: &[u32]) {
    for &w in at {
        let w = e.rebase(w);
        let v = rng.next();
        e.w32(w, v);
    }
}

/// `Phyre_PSprite_PSpriteAnimationInfoChar_Bind`'s constant stores (from the machine code).
const STORES_568AB0: &[u32] = &[
    0x00cb_2a24,
    0x00cb_2a28,
    0x00cb_2a2c,
    0x00cb_2a4c,
    0x00cb_2a50,
    0x00cb_2a54,
];

/// `Phyre_PSprite_PSpriteAnimationInfo_Bind`'s constant stores (from the machine code).
const STORES_5686F0: &[u32] = &[
    0x00ca_c3f8,
    0x00cb_2884,
    0x00cb_28cc,
    0x00cb_28d0,
    0x00cb_28d4,
    0x00cb_28f4,
    0x00cb_28f8,
    0x00cb_28fc,
    0x00cb_291c,
    0x00cb_2920,
    0x00cb_2924,
];

/// `Phyre_PSprite_PSpriteCollection_Bind`'s constant stores (from the machine code).
const STORES_5691B0: &[u32] = &[
    0x00ca_c3f8,
    0x00cb_2330,
    0x00cb_23fc,
    0x00cb_2400,
    0x00cb_2404,
    0x00cb_2424,
    0x00cb_2428,
    0x00cb_242c,
    0x00cb_244c,
    0x00cb_2450,
    0x00cb_2454,
    0x00cb_2474,
    0x00cb_2478,
    0x00cb_247c,
    0x00cb_249c,
    0x00cb_24a0,
    0x00cb_24a4,
    0x00cb_24c4,
    0x00cb_24c8,
    0x00cb_24cc,
    0x00cb_24ec,
    0x00cb_24f0,
    0x00cb_24f4,
    0x00cb_2514,
    0x00cb_2518,
    0x00cb_251c,
    0x00cb_253c,
    0x00cb_2540,
    0x00cb_2544,
    0x00cb_2564,
    0x00cb_2568,
    0x00cb_256c,
    0x00cb_258c,
    0x00cb_2590,
    0x00cb_2594,
    0x00cb_25b4,
    0x00cb_25b8,
    0x00cb_25bc,
    0x00cb_25dc,
    0x00cb_25e0,
    0x00cb_25e4,
    0x00cb_2604,
    0x00cb_2608,
    0x00cb_260c,
    0x00cb_262c,
    0x00cb_2630,
    0x00cb_2634,
    0x00cb_2654,
    0x00cb_2658,
    0x00cb_265c,
    0x00cb_267c,
    0x00cb_2680,
    0x00cb_2684,
    0x00cb_26a4,
    0x00cb_26a8,
    0x00cb_26ac,
];

/// `Phyre_PGameplay_PCameraControllerComponent_Bind`'s constant stores (from the machine code).
const STORES_5576A0: &[u32] = &[
    0x00cb_00c4,
    0x00cb_00c8,
    0x00cb_00d0,
    0x00cb_00d4,
    0x00cb_00ec,
    0x00cb_00f0,
    0x00cb_00f8,
    0x00cb_00fc,
    0x00cb_0114,
    0x00cb_0118,
    0x00cb_0120,
    0x00cb_0124,
    0x00cb_013c,
    0x00cb_0140,
    0x00cb_0148,
    0x00cb_014c,
    0x00cb_0164,
    0x00cb_0168,
    0x00cb_0170,
    0x00cb_0174,
    0x00cb_018c,
    0x00cb_0190,
    0x00cb_0198,
    0x00cb_019c,
    0x00cb_01b4,
    0x00cb_01b8,
    0x00cb_01c0,
    0x00cb_01c4,
    0x00cb_01dc,
    0x00cb_01e0,
    0x00cb_01e8,
    0x00cb_01ec,
    0x00cb_0204,
    0x00cb_0208,
    0x00cb_0210,
    0x00cb_0214,
];

/// `Phyre_PRendering_PMeshInstance_Bind`'s constant stores (from the machine code).
const STORES_4F38A0: &[u32] = &[
    0x00c9_a1b0,
    0x00ca_7430,
    0x00ca_7540,
    0x00ca_7874,
    0x00ca_7878,
    0x00ca_787c,
    0x00ca_789c,
    0x00ca_78a0,
    0x00ca_78a4,
    0x00ca_78c4,
    0x00ca_78c8,
    0x00ca_78cc,
    0x00ca_78ec,
    0x00ca_78f0,
    0x00ca_78f4,
    0x00ca_7914,
    0x00ca_7918,
    0x00ca_791c,
    0x00ca_793c,
    0x00ca_7940,
    0x00ca_7944,
    0x00ca_7964,
    0x00ca_7968,
    0x00ca_796c,
    0x00ca_7990,
    0x00ca_7994,
    0x00ca_7998,
    0x00ca_7ac8,
];

/// `Phyre_PRendering_PEffect_Bind`'s constant stores (from the machine code).
const STORES_4EB2D0: &[u32] = &[
    0x00ca_6830,
    0x00ca_6864,
    0x00ca_6898,
    0x00ca_68cc,
    0x00ca_6900,
    0x00ca_6990,
    0x00ca_6ab0,
    0x00ca_6bd0,
    0x00ca_6cf0,
    0x00ca_6e10,
];

pub static CASES: &[Case] = &[
    Case {
        name: "Phyre_PSprite_PSpriteAnimationInfoChar_Bind",
        addr: 0x0056_8ab0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_59d0, 1, 1, true),
            (0x0043_8060, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_a080, 3, 3, true),
            (0x0043_c140, 0, 0, true),
            (0x0057_52a0, 6, 6, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_568AB0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x0056_8ab0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x0056_8ab0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PSprite_PSpriteAnimationInfo_Bind",
        addr: 0x0056_86f0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_59d0, 1, 1, true),
            (0x0043_7fc0, 0, 0, false),
            (0x0043_8060, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_a080, 3, 3, true),
            (0x0043_c140, 0, 0, true),
            (0x0052_75f0, 0, 0, true),
            (0x0052_8610, 1, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0057_5ef0, 7, 7, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_5686F0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x0056_86f0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x0056_86f0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PSprite_PSpriteCollection_Bind",
        addr: 0x0056_91b0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_59d0, 1, 1, true),
            (0x0043_7fc0, 0, 0, false),
            (0x0043_8390, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_a080, 3, 3, true),
            (0x0043_c140, 0, 0, true),
            (0x0052_75f0, 0, 0, true),
            (0x0052_8610, 1, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0057_5ef0, 7, 7, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_5691B0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x0056_91b0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x0056_91b0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PGameplay_PCameraControllerComponent_Bind",
        addr: 0x0055_76a0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_59d0, 1, 1, true),
            (0x0043_8390, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_a080, 3, 3, true),
            (0x0043_c140, 0, 0, true),
            (0x0057_52a0, 6, 6, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_5576A0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x0055_76a0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x0055_76a0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PMeshInstance_Bind",
        addr: 0x004f_38a0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_59d0, 1, 1, true),
            (0x0043_7f10, 0, 0, false),
            (0x0043_7fc0, 0, 0, false),
            (0x0043_8060, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_a080, 3, 3, true),
            (0x0043_c140, 0, 0, true),
            (0x0045_f3f0, 0, 0, false),
            (0x0048_e880, 0, 0, true),
            (0x0049_0b60, 1, 0, false),
            (0x004f_20c0, 0, 0, true),
            (0x004f_4b50, 1, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0057_5ef0, 7, 7, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_4F38A0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x004f_38a0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x004f_38a0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
    Case {
        name: "Phyre_PRendering_PEffect_Bind",
        addr: 0x004e_b2d0,
        conv: K::C,
        kinds: "",
        out: O::U32,
        callees: &[
            (0x0043_7fc0, 0, 0, false),
            (0x0043_95b0, 1, 0, false),
            (0x0043_c140, 0, 0, true),
            (0x0049_d050, 0, 0, false),
            (0x004c_8a70, 0, 0, false),
            (0x004d_0e20, 0, 0, false),
            (0x004e_9940, 0, 0, true),
            (0x004e_99e0, 0, 0, true),
            (0x004e_9a80, 0, 0, true),
            (0x004e_9b20, 0, 0, true),
            (0x004e_9bc0, 0, 0, true),
            (0x004e_b850, 1, 0, false),
            (0x004e_ba60, 1, 0, false),
            (0x004e_bc70, 1, 0, false),
            (0x004e_be80, 1, 0, false),
            (0x004e_c090, 1, 0, false),
            (0x0057_52a0, 6, 6, true),
            (0x0057_5ef0, 7, 7, true),
            (0x0094_9207, 1, 0, false),
        ],
        prep: |e, r, _| {
            prep_words(e, r, STORES_4EB2D0);
            prep_h1_bind(
                e,
                r,
                &const { ffx_prender::h2_binds2::bind_of(0x004e_b2d0) },
            );
        },
        port: |g, _| {
            h1_bind::bridge::bind(g, const { ffx_prender::h2_binds2::bind_of(0x004e_b2d0) })
        },
        frame: 0,
        gap: 0,
        writes: &[],
    },
];
