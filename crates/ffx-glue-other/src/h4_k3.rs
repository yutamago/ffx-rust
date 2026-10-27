//! Continuation round 9 of list `h4-glue-other`, batch 3: the depth-bias
//! hook for instantiated characters, the post-process manager's
//! constructor, the broken-screen effect's render.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_k1::{
    BEGIN_SCENE, END_SCENE, FIND_BY_NAME, OUTPUT_SCALE, RENDER_MESH_INSTANCE, SET_COLOR_TARGET,
    SURFACE_CTOR, SURFACE_DTOR,
};
use crate::h4_k2::MEMCPY;

/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `PhyreScene_getCurrentLevelNameHash(scene)` (`0x0065de90`, thiscall).
pub const LEVEL_HASH: u32 = 0x0065_de90;
/// `SetPairFloats_006e4020(table; key, a, b)` (thiscall, ret 0xc): the
/// depth-bias entry of `key` (appended when missing) gets `a`, `b`.
pub const SET_PAIR: u32 = 0x006e_4020;
/// `Phyre_PhyrePrintf(level, format, ...)` (`0x004352f0`, cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `Phyre_PhyreFree(p)` (`0x0042fb20`, cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `g_DepthBias_Dirty` (u8 `0x00ce5176`).
pub const DIRTY: u32 = 0x00ce_5176;
/// `g_DepthBias_Count` (`0x01942998`), followed by `g_DepthBias`
/// (`0x0194299c`, 256 entries of {key, f32, f32}).
pub const TABLE: u32 = 0x0194_2998;
/// `"Add depth bias: %#08x, %#08x(%f)"` (`0x00b4add4`).
pub const FORMAT: u32 = 0x00b4_add4;
/// The maps with fixed biases: `"m121"` (`0x00b4adf8`), `"m281"`
/// (`0x00b4ae00`).
pub const M121: u32 = 0x00b4_adf8;
/// See [`M121`].
pub const M281: u32 = 0x00b4_ae00;
/// The bias constants: f64 800 (`0x00b4ae08`, printed), f32 800, 804,
/// 808, 812 (`0x00b4ae14`..`0x00b4ae20`), f32 20 (`0x00b4ae10`), 2
/// (`0x00b92268`), 4 (`0x00b922f8`).
pub const BIAS_F64: u32 = 0x00b4_ae08;
/// See [`BIAS_F64`].
pub const BIAS_800: u32 = 0x00b4_ae14;
/// See [`BIAS_F64`].
pub const BIAS_804: u32 = 0x00b4_ae18;
/// See [`BIAS_F64`].
pub const BIAS_808: u32 = 0x00b4_ae1c;
/// See [`BIAS_F64`].
pub const BIAS_812: u32 = 0x00b4_ae20;
/// See [`BIAS_F64`].
pub const BIAS_20: u32 = 0x00b4_ae10;
/// See [`BIAS_F64`].
pub const BIAS_2: u32 = 0x00b9_2268;
/// See [`BIAS_F64`].
pub const BIAS_4: u32 = 0x00b9_22f8;
/// The level name hashes with biases.
pub const LEVEL_ALL: u32 = 0x189;
/// See [`LEVEL_ALL`].
pub const LEVEL_NAMED: u32 = 0x278;

/// Whether the string at `a` equals the one at `b` (the inlined
/// `strcmp`).
fn same_text(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        if x != h.read::<u8>(b.wrapping_add(k)) {
            return false;
        }
        if x == 0 {
            return true;
        }
        k = k.wrapping_add(1);
    }
}

/// The character key of instance `i` of `items` (its `+0x1c`).
fn key(h: &impl Host, items: u32, i: u32) -> u32 {
    let instance: u32 = h.read(items.wrapping_add(4 * i));
    h.read(instance.wrapping_add(0x1c))
}

/// `Phyre_PhyrePrintf(2, "Add depth bias: ...", key, a, b)` with `a` and
/// `b` as doubles (the words the original pushes).
fn print_bias(h: &mut impl Host, key: u32, a: f64, b: u64) {
    let a = a.to_bits();
    let format = h.rebase(FORMAT);
    h.call_sym(
        PHYRE_PRINTF,
        Conv::Cdecl,
        &[
            2,
            format,
            key,
            a as u32,
            (a >> 32) as u32,
            b as u32,
            (b >> 32) as u32,
        ],
    );
}

/// `DepthBiasHooks_onCharacterInstantiated(name, count, items)`
/// (`0x006e4100`, cdecl; the instances' array by value: its count, top
/// bit "not owned", and its items; the caller reads no result). On level
/// 0x189 every instance's key gets the entry (1.0, 800) in
/// `g_DepthBias` (found or appended; past 255 entries the original
/// writes to address 0) and a debug print. On level 0x278 the map m121
/// gives the first four instances (0, 800/804/808/812) through
/// `SetPairFloats` and one print; on other levels m121 or m281 give them
/// (1, 0/2/4/20). Either sets `g_DepthBias_Dirty`. An owned array is
/// freed.
pub fn on_character_instantiated(h: &mut impl Host, name: u32, count: u32, items: u32) {
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let table = h.rebase(TABLE);
    let bias_f64: u64 = h.read(h.rebase(BIAS_F64));
    if h.call_sym(LEVEL_HASH, Conv::Thiscall(scene), &[]) == LEVEL_ALL {
        h.write(h.rebase(DIRTY), 1u8);
        let mut i = 0u32;
        while i < count & 0x7fff_ffff {
            let k = key(h, items, i);
            let mut n: u32 = h.read(table);
            if n >= 0x100 {
                // the original's deliberate crash: a write to address 0
                h.write(0, 0u32);
                n = h.read(table);
            }
            let entries = table + 4;
            let index = (0..n)
                .find(|&j| h.read::<u32>(entries.wrapping_add(12 * j)) == k)
                .unwrap_or_else(|| {
                    h.write(table, n.wrapping_add(1));
                    n
                });
            let entry = entries.wrapping_add(index.wrapping_mul(12));
            let bias = pass(h.read(h.rebase(BIAS_800)));
            h.write(entry.wrapping_add(4), 1.0f32.to_bits());
            h.write(entry, k);
            h.write(entry.wrapping_add(8), bias);
            let k = key(h, items, i);
            print_bias(h, k, 1.0, bias_f64);
            i += 1;
        }
    } else {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let level = h.call_sym(LEVEL_HASH, Conv::Thiscall(scene), &[]);
        let m121 = same_text(h, name, h.rebase(M121));
        if level == LEVEL_NAMED {
            if m121 {
                let zero = 0f32.to_bits();
                for (i, sym) in [BIAS_800, BIAS_804, BIAS_808, BIAS_812]
                    .into_iter()
                    .enumerate()
                {
                    let b = pass(h.read(h.rebase(sym)));
                    let k = key(h, items, i as u32);
                    if i == 0 {
                        h.write(h.rebase(DIRTY), 1u8);
                    }
                    h.call_sym(SET_PAIR, Conv::Thiscall(table), &[k, zero, b]);
                }
                let k = key(h, items, 0);
                print_bias(h, k, 0.0, bias_f64);
            }
        } else if m121 || same_text(h, name, h.rebase(M281)) {
            let one = 1.0f32.to_bits();
            for (i, b) in [None, Some(BIAS_2), Some(BIAS_4), Some(BIAS_20)]
                .into_iter()
                .enumerate()
            {
                let b = b.map_or(0, |sym| pass(h.read(h.rebase(sym))));
                let k = key(h, items, i as u32);
                if i == 0 {
                    h.write(h.rebase(DIRTY), 1u8);
                }
                h.call_sym(SET_PAIR, Conv::Thiscall(table), &[k, one, b]);
            }
        }
    }
    if count >> 31 == 0 {
        h.call_sym(FREE, Conv::Cdecl, &[items]);
    }
}

// ---- PostProcessManager_ctor_00692020

/// `Phyre_PCluster_operator_new(cluster)` (`0x00576630`, thiscall: an
/// embedded cluster's constructor).
pub const CLUSTER_CTOR: u32 = 0x0057_6630;
/// The post-process passes' constructors (thiscall): `PMLAA`
/// (`0x009fc7c0`), `PDepthOfField` (`0x009fbd40`), the derived pass
/// (`0x009fc650`), `PFXAA` (`0x009fc140`).
pub const MLAA_CTOR: u32 = 0x009f_c7c0;
/// See [`MLAA_CTOR`].
pub const DOF_CTOR: u32 = 0x009f_bd40;
/// See [`MLAA_CTOR`].
pub const DERIVED_CTOR: u32 = 0x009f_c650;
/// See [`MLAA_CTOR`].
pub const FXAA_CTOR: u32 = 0x009f_c140;
/// `Phyre_PFreeList_PFreeList(list; size, count, align, name)`
/// (`0x0049c480`, thiscall, ret 0x10).
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
/// `PSamplerStateD3D11_ctor(sampler)` (`0x00583b20`, thiscall).
pub const SAMPLER_CTOR: u32 = 0x0058_3b20;
/// `memset` through its import thunk (`0x0094964c`, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PRedBlackTree_orphanAllContents(tree)` (`0x00574970`,
/// thiscall).
pub const TREE_ORPHAN: u32 = 0x0057_4970;
/// `Phyre_PFreeList_reset(list)` (`0x0049cc10`, thiscall).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
/// `"PMapPair"` (`0x00b13bc8`).
pub const PMAP_PAIR: u32 = 0x00b1_3bc8;
/// The f32 defaults the constructor copies (`fld`/`fstp`), in the
/// original's load order.
pub const PPM_FLOATS: [u32; 12] = [
    0x00b3_eb58,
    0x00b1_0f40,
    0x00b4_7c54,
    0x00b4_7c80,
    0x00b4_7c70,
    0x00b4_7c84,
    0x00b4_7c88,
    0x00b4_7c64,
    0x00b4_7c58,
    0x00b4_7c5c,
    0x00b1_f2c8,
    0x00b4_3ddc,
];
/// The last f32 default (`0x00b47c98`).
pub const PPM_FLOAT_LAST: u32 = 0x00b4_7c98;

/// `PostProcessManager_ctor_00692020(this)` (thiscall, plain ret ->
/// `this`; from `PostProcessManager_createInstance`; the SEH frame is not
/// kept): the embedded clusters (`+0x3c`, `+0x12334`), the post-process
/// passes (`+0xb0` MLAA, `+0x140` depth of field, `+0x1100`, `+0x1190`
/// FXAA), the samplers (`+0x122cc`, `+0x122f0`, `+0x123dc`), the
/// `PMapPair` free list (`+0x12284`), list heads pointing at themselves
/// (`+0x84`, `+0x1226c` six words, `+0x1237c`), the tuning floats
/// (image f32s copied, 1.0 and 0.0), counters and flags; `+4`..`+0x3c`
/// cleared by `memset`; finally the tree `+0x1226c` orphans its contents
/// and the free list resets. Every store in the original's order around
/// the calls.
#[allow(clippy::too_many_lines)] // one store per field, in order
pub fn post_process_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    let one = 1.0f32.to_bits();
    h.write(at(0), 0u32);
    h.call_sym(CLUSTER_CTOR, Conv::Thiscall(at(0x3c)), &[]);
    h.write(at(0x84), at(0x84));
    h.write(at(0x88), at(0x84));
    for off in [0x8c, 0x90, 0x94, 0xa8, 0xac] {
        h.write(at(off), 0u32);
    }
    h.call_sym(MLAA_CTOR, Conv::Thiscall(at(0xb0)), &[]);
    h.call_sym(DOF_CTOR, Conv::Thiscall(at(0x140)), &[]);
    h.call_sym(DERIVED_CTOR, Conv::Thiscall(at(0x1100)), &[]);
    h.call_sym(FXAA_CTOR, Conv::Thiscall(at(0x1190)), &[]);
    let words: [(u32, u32); 26] = [
        (0x1224, image_word(h, PPM_FLOATS[0])),
        (0x1200, 0),
        (0x1228, one),
        (0x120c, 0),
        (0x1214, 0),
        (0x122c, image_word(h, PPM_FLOATS[1])),
        (0x1218, 0),
        (0x121c, 0),
        (0x1230, image_word(h, PPM_FLOATS[2])),
        (0x1220, 0),
        (0x125c, 2),
        (0x1234, image_word(h, PPM_FLOATS[3])),
        (0x1264, 1),
        (0x1238, image_word(h, PPM_FLOATS[4])),
        (0x123c, image_word(h, PPM_FLOATS[5])),
        (0x1254, image_word(h, PPM_FLOATS[5])),
        (0x1240, one),
        (0x1258, one),
        (0x1244, image_word(h, PPM_FLOATS[1])),
        (0x1248, image_word(h, PPM_FLOATS[2])),
        (0x124c, image_word(h, PPM_FLOATS[3])),
        (0x1250, image_word(h, PPM_FLOATS[4])),
        // the tree head (+0x1226c): six words pointing at it
        (0x1226c, at(0x1226c)),
        (0x12270, at(0x1226c)),
        (0x12274, at(0x1226c)),
        (0x12280, at(0x1226c)),
    ];
    for (off, v) in words {
        h.write(at(off), v);
        if off == 0x1234 {
            h.write(at(0x1260), 0u8);
        }
    }
    h.write(at(0x1227c), at(0x1226c));
    h.write(at(0x12278), at(0x1226c));
    let name = h.rebase(PMAP_PAIR);
    h.call_sym(
        FREE_LIST_CTOR,
        Conv::Thiscall(at(0x12284)),
        &[0x14, 8, 4, name],
    );
    h.write(at(0x1229c), 0u32);
    h.write(at(0x122a0), 0u32);
    let v = image_word(h, PPM_FLOATS[6]);
    h.write(at(0x122a4), v);
    h.write(at(0x122b4), 0u8);
    h.write(at(0x122b8), 0u32);
    let v = image_word(h, PPM_FLOATS[7]);
    h.write(at(0x122a8), v);
    h.write(at(0x122bc), 0u32);
    h.write(at(0x122c0), 0u32);
    let v = image_word(h, PPM_FLOATS[8]);
    h.write(at(0x122ac), v);
    h.write(at(0x122c4), 0u32);
    h.write(at(0x122c8), 0u32);
    h.write(at(0x122b0), 0u32);
    h.call_sym(SAMPLER_CTOR, Conv::Thiscall(at(0x122cc)), &[]);
    h.call_sym(SAMPLER_CTOR, Conv::Thiscall(at(0x122f0)), &[]);
    let v = image_word(h, PPM_FLOATS[9]);
    h.write(at(0x12314), v);
    let v = image_word(h, PPM_FLOATS[10]);
    h.write(at(0x12328), v);
    h.write(at(0x12318), 0u8);
    h.write(at(0x1231c), 2u32);
    for off in [0x12320, 0x12324, 0x1232c, 0x12330] {
        h.write(at(off), 0u32);
    }
    h.call_sym(CLUSTER_CTOR, Conv::Thiscall(at(0x12334)), &[]);
    h.write(at(0x1237c), at(0x1237c));
    h.write(at(0x12380), at(0x1237c));
    let v = image_word(h, PPM_FLOATS[11]);
    h.write(at(0x123c8), v);
    let v = image_word(h, PPM_FLOAT_LAST);
    h.write(at(0x123cc), v);
    h.write(at(0x12384), 0u32);
    h.write(at(0x12388), 0u32);
    h.write(at(0x123a0), 0u8);
    h.write(at(0x123a4), 0u32);
    h.write(at(0x123a8), 0u8);
    for off in [0x123b0, 0x123b4, 0x123b8, 0x123bc, 0x123c0] {
        h.write(at(off), 0u32);
    }
    h.write(at(0x123c4), 0u16);
    h.write(at(0x123d0), 0u32);
    h.write(at(0x123d4), 0u32);
    h.write(at(0x123d9), 0u8);
    h.call_sym(SAMPLER_CTOR, Conv::Thiscall(at(0x123dc)), &[]);
    h.write(at(0x12400), 0u32);
    h.write(at(0x12404), 0u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[at(4), 0, 0x38]);
    for off in [0x1238c, 0x12390, 0x12394, 0x12398, 0x1239c, 0x1268] {
        h.write(at(off), 0u32);
    }
    h.call_sym(TREE_ORPHAN, Conv::Thiscall(at(0x1226c)), &[]);
    h.call_sym(FREE_LIST_RESET, Conv::Thiscall(at(0x12284)), &[]);
    this
}

/// The f32 at image address `sym` through `fld`/`fstp`.
fn image_word(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

// ---- BrokenScreenEffect_render

/// `BrokenScreenEffect_createRenderTarget(this)` (`0x006d5050`,
/// thiscall).
pub const BROKEN_CREATE_TARGET: u32 = 0x006d_5050;
/// `0x006d3650(piece; a, b)` (thiscall, ret 0x20: two 4-float vectors by
/// value): a piece's update.
pub const PIECE_UPDATE: u32 = 0x006d_3650;
/// `BrokenScreenEffect_setColour_006d4620(piece; colour)` (thiscall, ret
/// 4).
pub const PIECE_SET_COLOUR: u32 = 0x006d_4620;
/// `BrokenScreenEffect_setColorBuffer_006d4540(piece; texture, sampler)`
/// (thiscall, ret 8).
pub const PIECE_SET_COLOR_BUFFER: u32 = 0x006d_4540;
/// `BrokenScreenEffect_render_006d44c0(piece)` (thiscall).
pub const PIECE_RENDER: u32 = 0x006d_44c0;
/// `Phyre_PRendering_PRenderer_setDepthStencilTarget(renderer; surface)`
/// (`0x005bb5b0`, thiscall, ret 4).
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
/// The passes' states (`0x00cded20` the background, `0x00cdecd0` the
/// pieces' first pass, `0x00cded10` their second), stored at the
/// renderer's `+0x48`.
pub const BROKEN_STATE_BACK: u32 = 0x00cd_ed20;
/// See [`BROKEN_STATE_BACK`].
pub const BROKEN_STATE_FIRST: u32 = 0x00cd_ecd0;
/// See [`BROKEN_STATE_BACK`].
pub const BROKEN_STATE_SECOND: u32 = 0x00cd_ed10;
/// Bytes of [`broken_render`]'s frame, from the original's `ebp - 0x50`:
/// the colour and depth surfaces (`+0`, `+0x10`), the `OutputScale`
/// value (`+0x20`) and the pieces' colour (`+0x30`), 4 words each.
pub const BROKEN_FRAME: u32 = 0x40;

/// The piece count (`+0x34`, signed).
fn broken_pieces(h: &impl Host, this: u32) -> i32 {
    h.read(this.wrapping_add(0x34))
}

/// Half `half` (0 or 4) of piece `i`'s pass pair (`+0x38`).
fn broken_piece(h: &impl Host, this: u32, i: i32, half: u32) -> u32 {
    let list: u32 = h.read(this.wrapping_add(0x38));
    let pair: u32 = h.read(list.wrapping_add(4 * i as u32));
    h.read(pair.wrapping_add(half))
}

/// `BrokenScreenEffect_render(this)` (`0x006d5680`, thiscall, plain ret
/// -> 0; `frame` see [`BROKEN_FRAME`]; the SEH frame is not kept): the
/// rendered flag `+0x75` follows the active flag `+5`; while active (the
/// render target made first unless `+0x74`) each of the `+0x34` pieces
/// (`+0x38`: pairs of pass objects) gets both halves updated with the
/// vectors `+0xc`/`+0x1c`; with the colour target `+0x60` and depth
/// target `+0x64` the background mesh `+0x58` renders with
/// `OutputScale` (0, 0, 0, `+8`) copied into its first material's
/// buffer (when the parameter fits: offset at least 4, end within the
/// buffer), then each piece's first half with colour (1, 1, 1, `+8`) and
/// the scene texture `+0`, then each second half. A one-shot effect
/// (`+6` 1) turns itself off. The original's `memcpy` of the parameter
/// reads `size` bytes from the value: the frame holds the value and the
/// colour (0x20 bytes), the real parameter is 16.
#[allow(clippy::too_many_lines)] // the three passes in order
pub fn broken_render(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0x75), 0u8);
    if h.read::<u8>(at(5)) == 0 {
        return 0;
    }
    if h.read::<u8>(at(0x74)) == 0 {
        h.call_sym(BROKEN_CREATE_TARGET, Conv::Thiscall(this), &[]);
    }
    let colour = frame + 0x30;
    let alpha = pass(h.read(at(8)));
    h.write(at(0x75), 1u8);
    let one = 1.0f32.to_bits();
    for (k, w) in [one, one, one, alpha].into_iter().enumerate() {
        h.write(colour + 4 * k as u32, w);
    }
    let mut i = 0;
    while i < broken_pieces(h, this) {
        for half in [0, 4] {
            let words: [u32; 8] = std::array::from_fn(|k| h.read(at(0xc + 4 * k as u32)));
            let p = broken_piece(h, this, i, half);
            h.call_sym(PIECE_UPDATE, Conv::Thiscall(p), &words);
        }
        i += 1;
    }
    let colour_surface = frame;
    let depth_surface = frame + 0x10;
    let target: u32 = h.read(at(0x60));
    h.call_sym(SURFACE_CTOR, Conv::Thiscall(colour_surface), &[target, 0]);
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(
        SET_COLOR_TARGET,
        Conv::Thiscall(renderer),
        &[colour_surface, 0],
    );
    let target: u32 = h.read(at(0x64));
    h.call_sym(SURFACE_CTOR, Conv::Thiscall(depth_surface), &[target, 0]);
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(SET_DEPTH_TARGET, Conv::Thiscall(renderer), &[depth_surface]);
    // the background mesh's first material: {shader parameters, buffer}
    let mesh: u32 = h.read(at(0x58));
    let materials: u32 = h.read(mesh.wrapping_add(0x10));
    let n = h.read::<u32>(materials) & 0x7fff_ffff;
    let list = if n > 1 {
        h.read(materials.wrapping_add(4))
    } else if n != 0 {
        materials.wrapping_add(4)
    } else {
        0
    };
    let material: u32 = h.read(list);
    let shader: u32 = h.read(material);
    let name = h.rebase(OUTPUT_SCALE);
    let p = h.call_sym(FIND_BY_NAME, Conv::Thiscall(shader), &[name]);
    if p != 0 {
        let offset = u32::from(h.read::<u16>(p.wrapping_add(8)));
        let buffer: u32 = h.read(material.wrapping_add(4));
        let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
        let value = frame + 0x20;
        let alpha = pass(h.read(at(8)));
        for (k, w) in [0, 0, 0, alpha].into_iter().enumerate() {
            h.write(value + 4 * k as u32, w);
        }
        if offset + size <= h.read::<u32>(buffer) && offset >= 4 {
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[offset.wrapping_add(buffer), value, size],
            );
        }
    }
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[2]);
    let renderer: u32 = h.read(at(0x2c));
    let state = h.rebase(BROKEN_STATE_BACK);
    h.write(renderer.wrapping_add(0x48), state);
    let renderer: u32 = h.read(at(0x2c));
    let mesh: u32 = h.read(at(0x58));
    let context: u32 = h.read(renderer.wrapping_add(0x13c));
    let mesh_data: u32 = h.read(mesh.wrapping_add(0x10));
    h.call_sym(
        RENDER_MESH_INSTANCE,
        Conv::Thiscall(renderer),
        &[mesh, mesh_data, 0, context],
    );
    for (state, half) in [(BROKEN_STATE_FIRST, 0), (BROKEN_STATE_SECOND, 4)] {
        let renderer: u32 = h.read(at(0x2c));
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
        let renderer: u32 = h.read(at(0x2c));
        h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        let renderer: u32 = h.read(at(0x2c));
        let state = h.rebase(state);
        h.write(renderer.wrapping_add(0x48), state);
        let mut i = 0;
        while i < broken_pieces(h, this) {
            let p = broken_piece(h, this, i, half);
            h.call_sym(PIECE_SET_COLOUR, Conv::Thiscall(p), &[colour]);
            if half == 0 {
                let scene: u32 = h.read(at(0));
                let texture = if h.read::<u8>(scene.wrapping_add(4)) == 0 {
                    h.read(scene)
                } else {
                    0
                };
                let p = broken_piece(h, this, i, half);
                h.call_sym(PIECE_SET_COLOR_BUFFER, Conv::Thiscall(p), &[texture, 0]);
            }
            let p = broken_piece(h, this, i, half);
            h.call_sym(PIECE_RENDER, Conv::Thiscall(p), &[]);
            i += 1;
        }
    }
    let renderer: u32 = h.read(at(0x2c));
    h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1]);
    if h.read::<u8>(at(6)) == 1 {
        h.write(at(5), 0u8);
    }
    h.call_sym(SURFACE_DTOR, Conv::Thiscall(depth_surface), &[]);
    h.call_sym(SURFACE_DTOR, Conv::Thiscall(colour_surface), &[]);
    0
}
