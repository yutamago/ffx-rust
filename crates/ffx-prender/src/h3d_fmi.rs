//! The render-side glue (M4, list `h3-glue-render`, agent h3d):
//! `PhyreScene::fieldMapInitialize` (`0x0065b8a0`, Fahrenheit's name),
//! which sets a field map up once its clusters are loaded: the depth-bias
//! hooks, the asset rebinds, the layer shapes, texture animations, camera,
//! post-processing, colour correction, morphs, the water-distortion
//! delegation, lighting and fog, which layers are linked, the shadow
//! solution and the shadow-map shader parameters.
//!
//! `thiscall(scene; rebuild)`, `ret 4`: 0x15 without the clusters it needs
//! (or for a rebuild the scene refuses, `+0x1195`), else 0. It leaves
//! nothing in `ST0` (the packet's flag comes from a callee's heuristic;
//! the oracle tests check it). Its C++ exception frame only destroys the
//! by-value copies when a copy throws; the port leaves it out.
//!
//! The scene: `+0x10d0`/`+0x10d1` flags, `+0x10d4` the map's main cluster,
//! `+0x1128` the 2D cluster; `+0x10d8`, `+0x112c` asset arrays (count,
//! capacity, data), `+0x10e4`, `+0x1144` their asset lists (0x44 bytes:
//! three arrays and a name); `+0x1138` the layer shapes (count, capacity,
//! records of 8 bytes: flags, object); `+0x1198` a red-black tree of
//! layers; `+0x38a4..+0x38cc` five arrays of the shadow parameters (count
//! with bit 31 "not owned", data); `+0xf9cc` the shadow solution,
//! `+0xf9d0`/`+0xf9d1` which world holds the shadow cluster `+0x28c4`
//! (`+0x7c` or `+0xbc`).
//!
//! The frame: the original keeps its scene cursors, the `sscanf` results,
//! the shader parameter blocks, the two cascade matrices and the averaged
//! caster value in its frame and hands their addresses to callees, so the
//! bridge takes a frame laid out at the original's `ebp` offsets
//! ([`FRAME_LOW`] bytes below `ebp`, the argument word at `ebp + 8`).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

// ---- callees (ffx.sym addresses) ----

/// `RenderDelegationHelper_clearDelegations_006adf70(helper; map)`
/// (thiscall, `ret 4`; the map word is popped, never read) on
/// `g_RenderDelegationHelper`: drops every delegation but the screen
/// fade's.
pub const DELEGATION_RESET: u32 = 0x006a_df70;
/// `PArray_assign_0064eb20(dst; src)` (thiscall, `ret 4`): the arrays'
/// copy.
pub const ARRAY_ASSIGN: u32 = 0x0064_eb20;
/// `AssetListReader_copy_0064d9e0(dst; src)` (thiscall, `ret 4`): the
/// asset lists' copy constructor.
pub const LIST_COPY: u32 = 0x0064_d9e0;
/// `DepthBiasHooks_onSceneLoaded(name, arrays and lists by value)`
/// (cdecl): [`ON_SCENE_LOADED_WORDS`] words; it destroys the copies.
pub const ON_SCENE_LOADED: u32 = 0x006e_4470;
/// `ShaderTexture_rebind_006f6c00(scene; slot, 0)` (thiscall, `ret 8`):
/// rebinds an asset's textures.
pub const REBIND: u32 = 0x006f_6c00;
/// `Array_Reserve_00668180(array; 0)` (thiscall, `ret 4`).
pub const ARRAY_RESERVE: u32 = 0x0066_8180;
/// `PhyreSceneCursor_Begin`/`_Next` (thiscall on the cursor, no
/// arguments) and `PhyreSceneCursor_Seek4`.
pub const CURSOR_BEGIN: u32 = 0x0063_7730;
pub const CURSOR_NEXT: u32 = 0x0063_77a0;
pub const CURSOR_SEEK4: u32 = 0x0065_ecb0;
/// `user_malloc(size)`, `user_free_thunk(p)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;
/// The CRT's `memcpy` (cdecl, a jump in `.text`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `sscanf`'s import slot (cdecl, varargs).
pub const SSCANF_SLOT: u32 = 0x00b0_c448;
/// `AssetListReader_getSymbolId(list; name)` (thiscall, `ret 4`): -1 when
/// absent.
pub const GET_SYMBOL_ID: u32 = 0x006f_4350;
/// `TextureAnimationManager_GetInstance()` (cdecl) and
/// `addAnimatedMaterials(manager; cluster, array, 0, id, 0, 0, 0)`
/// (thiscall, `ret 0x1c`).
pub const TEX_ANIM_INSTANCE: u32 = 0x006b_02e0;
pub const ADD_ANIMATED_MATERIALS: u32 = 0x006b_05d0;
/// The scene set-up, in the original's order (thiscall on the scene, no
/// arguments): `FUN_006518c0` (finds the global map object),
/// `findGlobalCamera` (sets `+0x4f8`), `setupPostProcessParams`,
/// `FUN_00651de0` (updates the lighting objects),
/// `setupColorCorrection`.
pub const SCENE_SETUP: [u32; 5] = [
    0x0065_18c0,
    0x0065_19f0,
    0x0066_f140,
    0x0065_1de0,
    0x0066_ed00,
];
/// `MorphManager_GetInstance()` (cdecl) and
/// `registerClusterMorphs(manager; cluster)` (thiscall, `ret 4`).
pub const MORPH_INSTANCE: u32 = 0x006a_bf40;
pub const REGISTER_MORPHS: u32 = 0x006a_ca10;
/// `operator_new(size)`, `operator_delete(p)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `RenderDelegationHelper_registerDelegation(helper; record, 0x14,
/// 0x41, callback)` (thiscall, `ret 0x10`).
pub const REGISTER_DELEGATION: u32 = 0x006a_de20;
/// `WaterDistortionEffectDelegations_processWaterDistortion`: the
/// delegation's callback.
pub const WATER_DISTORTION: u32 = 0x006f_54f0;
/// `PhyreScene_updateMapYIQ(scene)`, `FUN_0066eb20(scene; no_2d)` (sets
/// the character lighting up, `ret 4`), `PhyreScene_setupMapFog(scene)`.
pub const UPDATE_MAP_YIQ: u32 = 0x0067_1da0;
pub const SETUP_CHARACTER_LIGHTING: u32 = 0x0066_eb20;
pub const SETUP_MAP_FOG: u32 = 0x0066_ee70;
/// `Phyre_PRedBlackTree_successor(tree; node)` (thiscall, `ret 4`).
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// `PhyreScene_change2DSceneNum(scene; n)` (thiscall, `ret 4`).
pub const CHANGE_2D_SCENE_NUM: u32 = 0x0065_87d0;
/// `getScenerioFlag()` (cdecl, a `u16`).
pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
/// `ShadowManager_getInstance()` (cdecl).
pub const SHADOW_INSTANCE: u32 = 0x006d_2390;
/// `ShadowManager_selectShadowSolution(manager; 6 words)` (thiscall,
/// `ret 0x18`).
pub const SELECT_SHADOW_SOLUTION: u32 = 0x006c_cd10;
/// `Phyre_PWorld_removeCluster`/`addCluster(world; cluster)` (thiscall,
/// `ret 4`).
pub const REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const ADD_CLUSTER: u32 = 0x0044_3250;
/// `Phyre_PhyreFree(p)`, `Phyre_PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `ShadowManager_AverageCasterValue(manager)` (thiscall, a float in
/// `ST0`): the `mainChrYPos` parameter.
pub const AVERAGE_CASTER_VALUE: u32 = 0x006c_cad0;
/// `ShaderParams_FindByName(params; name)` (thiscall, `ret 4`): the
/// parameter's descriptor (`+2` its type, `+8` its offset, `+0xa` its
/// size in the low 13 bits) or 0.
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `ShadowManager_getDummyShadowMap(manager)` (thiscall) and
/// `getCascade(manager; i)` (thiscall, `ret 4`): a texture handle (the
/// texture, a "none" byte at `+4`).
pub const DUMMY_SHADOW_MAP: u32 = 0x006d_2380;
pub const GET_CASCADE: u32 = 0x006d_22e0;
/// `PhyreScene_getLinearSampler(scene; 2)` (thiscall, `ret 4`).
pub const GET_LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `TrophyManager_Install` (`ret 4`, `xor eax, eax`: a `return 0` the
/// linker folded), called as `(sampler; texture)`: when it returns
/// nonzero the block is not written (never in this build).
pub const TROPHY_INSTALL: u32 = 0x0059_e520;
/// `ShadowManager_GetShadowSize()` (cdecl, an `int`).
pub const GET_SHADOW_SIZE: u32 = 0x006c_cd00;
/// `ShadowManager_GetCascadeMatrix(manager; out, i)` (thiscall, `ret 8`):
/// 64 bytes into `out`.
pub const GET_CASCADE_MATRIX: u32 = 0x006d_23a0;
/// `PArray_PushInto_0065fac0`/`_0065fb50(array; memory, &value)`
/// (thiscall, `ret 8`).
pub const PUSH_INTO_FIRST: u32 = 0x0065_fac0;
pub const PUSH_INTO: u32 = 0x0065_fb50;
/// `Instance_006dc800_Get()` (cdecl) and
/// `Instance_006dc800_reportMapLoad_006dcd50(instance; name)` (thiscall,
/// `ret 4`: a `"MapLoad"` event with the map's name when enabled).
pub const INSTANCE_6DC800: u32 = 0x006d_c8a0;
pub const ON_MAP_NAME: u32 = 0x006d_cd50;

// ---- globals and constants ----

/// `g_RenderDelegationHelper`, `g_CurrentMapNo`, `g_GraphicMapId`.
pub const RENDER_DELEGATION_HELPER: u32 = 0x00cc_ca98;
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
pub const GRAPHIC_MAP_ID: u32 = 0x0133_0844;
/// The cursors' class descriptors (the layer objects', the materials').
pub const CLASS_LAYER_OBJECT: u32 = 0x00ca_7228;
pub const CLASS_MATERIAL: u32 = 0x00ca_5268;
/// Strings: `"textureAnimation"`, `"scene%dlayer%dShape"`, and the shadow
/// parameters' names: `"MySMSampler"`, `"MySMScaleBias"`,
/// `"MySMScaleBias2"`, `"LightViewProjection"`, `"LVP2"`,
/// `"mainChrYPos"`, `"MySMSampler2"`.
pub const STR_TEX_ANIM: u32 = 0x00b4_2318;
pub const STR_LAYER_SHAPE: u32 = 0x00b4_2500;
pub const STR_SM_SAMPLER: u32 = 0x00b4_2514;
pub const STR_SM_SCALE_BIAS: u32 = 0x00b4_2520;
pub const STR_SM_SCALE_BIAS2: u32 = 0x00b4_2530;
pub const STR_LIGHT_VIEW_PROJECTION: u32 = 0x00b4_2540;
pub const STR_LVP2: u32 = 0x00b4_2554;
pub const STR_MAIN_CHR_Y_POS: u32 = 0x00b4_255c;
pub const STR_SM_SAMPLER2: u32 = 0x00b4_2568;
/// `+0xf9c0` by map: map 0xec1 0.23, map 0xc58 -0.01 (`.rdata`
/// `0x00b43cf4`, `0x00b43de4`), else 0.
pub const MAP_EC1_BIAS: u32 = 0x3e6b_851f;
pub const MAP_C58_BIAS: u32 = 0xbc23_d70a;

/// The words `DepthBiasHooks_onSceneLoaded` takes: the name and the four
/// copies (0xa0 bytes).
pub const ON_SCENE_LOADED_WORDS: usize = 41;

/// The frame below `ebp` the bridge uses: down to the name word of
/// `DepthBiasHooks_onSceneLoaded`'s arguments (`ebp - 0x22c`).
pub const FRAME_LOW: u32 = 0x22c;

// ---- frame (`ebp` offsets) ----

/// The copies `DepthBiasHooks_onSceneLoaded` gets, where the original
/// builds them (its outgoing arguments): `+0x112c`'s, `+0x1144`'s,
/// `+0x10d8`'s, `+0x10e4`'s; the name below them.
pub const COPY_A: i32 = -0x194;
pub const COPY_B: i32 = -0x1d8;
pub const COPY_C: i32 = -0x1e4;
pub const COPY_D: i32 = -0x228;
pub const COPY_NAME: i32 = -0x22c;
/// The scene and `scene + 0x112c` (saved locals).
pub const F_SCENE: i32 = -0x14;
pub const F_ASSETS: i32 = -0x18;
/// The object of the layer-shape scan.
pub const F_OBJECT: i32 = -0x10;
/// The cluster index; later the first cascade matrix parameter.
pub const F_INDEX: i32 = -0x24;
/// The new capacity's buffer; later the second matrix parameter.
pub const F_NEW_DATA: i32 = -0x28;
/// The object's `+0x5c` and the new capacity; later the caster value
/// parameter.
pub const F_TEMP: i32 = -0x2c;
/// The fourth shadow parameter (the second shadow map).
pub const F_PARAM_MAP_1: i32 = -0x30;
/// The layer-object cursor (8 words) and the `sscanf` results.
pub const F_CURSOR: i32 = -0x88;
pub const F_SCAN_SCENE: i32 = -0x64;
pub const F_SCAN_LAYER: i32 = -0x68;
/// The layer-shape record (flags 1, the scene number's byte, the layer's
/// word).
pub const F_RECORD: i32 = -0x58;
/// The material cursor (8 words) and the material pushed with the
/// matrices.
pub const F_CURSOR2: i32 = -0x50;
pub const F_MATERIAL: i32 = -0x54;
/// The tree walk's head, node and next node.
pub const F_TREE_HEAD: i32 = -0x98;
pub const F_TREE_NODE: i32 = -0x94;
pub const F_TREE_NEXT: i32 = -0x90;
/// The averaged caster value (`f32`).
pub const F_CASTER: i32 = -0x8c;
/// The shadow-map blocks (0x42, 0, the sampler, the texture): the first
/// parameter with cascade 0 or the dummy map, the fourth with cascade 1
/// or the dummy map.
pub const F_BLOCK_CASCADE0: i32 = -0xb8;
pub const F_BLOCK_DUMMY0: i32 = -0xd8;
pub const F_BLOCK_CASCADE1: i32 = -0xf8;
pub const F_BLOCK_DUMMY1: i32 = -0xe8;
/// The two scale blocks (four `f32`).
pub const F_SCALE_A: i32 = -0xa8;
pub const F_SCALE_B: i32 = -0xc8;
/// The two cascade matrices.
pub const F_MATRIX_0: i32 = -0x138;
pub const F_MATRIX_1: i32 = -0x178;
/// The argument word (`rebuild`), which the original reuses.
pub const F_ARG: i32 = 8;

/// The shader-parameter types whose block is written.
pub const SAMPLER_TYPES: [u8; 6] = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12];

/// Whether `fieldMapInitialize` stops at once (returns 0x15): the main
/// cluster missing where `+0x10d1` wants it, the 2D one where `+0x10d0`
/// does, both missing, or a rebuild the scene refuses.
#[must_use]
#[allow(clippy::fn_params_excessive_bools)] // the scene's flags
pub fn refuses(
    wants_main: bool,
    main: u32,
    wants_2d: bool,
    two_d: u32,
    rebuild: bool,
    no_rebuild: bool,
) -> bool {
    (wants_main && main == 0)
        || (wants_2d && two_d == 0)
        || (main == 0 && two_d == 0)
        || (rebuild && no_rebuild)
}

/// The capacity the layer-shape array grows to for `n` entries
/// (`n + 2 + n / 8`, rounded toward zero).
#[must_use]
pub const fn grown(n: i32) -> i32 {
    n.wrapping_add(2).wrapping_add(n / 8)
}

/// The shadow map's border scale for a size the game returns as an `int`:
/// `(f32(size) - 64.0) / f32(size2)` (53-bit, rounded to `f32`).
#[must_use]
pub fn border_scale(size: i32, size2: i32) -> u32 {
    let a = x87::sub(as_f32(size), 64.0);
    x87::store(x87::div(a, as_f32(size2)))
}

/// `32.0 / f32(size)`, rounded to `f32`.
#[must_use]
pub fn texel_scale(size: i32) -> u32 {
    x87::store(x87::div(32.0, as_f32(size)))
}

/// `fild` then `fstp m32` then `fld m32`.
fn as_f32(v: i32) -> f64 {
    x87::load(x87::store(f64::from(v)))
}

/// Whether a layer object's `+0x5c` lies strictly between 0 and 1 (NaN
/// does not).
#[must_use]
pub fn is_partial(bits: u32) -> bool {
    let f = x87::load(bits);
    f > 0.0 && f < 1.0
}

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn at(ebp: u32, off: i32) -> u32 {
        ebp.wrapping_add_signed(off)
    }

    fn get(h: &impl Mem, ebp: u32, off: i32) -> u32 {
        h.read(at(ebp, off))
    }

    fn put(h: &mut impl Mem, ebp: u32, off: i32, v: u32) {
        h.write(at(ebp, off), v);
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// `PhyreScene::fieldMapInitialize(scene; rebuild)` -> 0, or 0x15
    /// when it stops at once. `ebp` is the frame ([`FRAME_LOW`] bytes
    /// below it, the argument word at `ebp + 8`); `on_scene_loaded` calls
    /// `DepthBiasHooks_onSceneLoaded` with the [`ON_SCENE_LOADED_WORDS`]
    /// words at `ebp - 0x22c` (the name and the copies, which it
    /// destroys).
    pub fn field_map_initialize<H: Host>(
        h: &mut H,
        ebp: u32,
        scene: u32,
        rebuild: u32,
        on_scene_loaded: impl FnOnce(&mut H, &[u32; ON_SCENE_LOADED_WORDS]),
    ) -> u32 {
        put(h, ebp, F_ARG, rebuild);
        put(h, ebp, F_SCENE, scene);
        let refused = refuses(
            h.read::<u8>(scene + 0x10d1) == 1,
            h.read(scene + 0x10d4),
            h.read::<u8>(scene + 0x10d0) == 1,
            h.read(scene + 0x1128),
            rebuild & 0xff != 0,
            h.read::<u8>(scene + 0x1195) != 0,
        );
        if refused {
            return 0x15;
        }
        copy_and_rebind(h, ebp, scene, on_scene_loaded);
        scan_layer_shapes(h, ebp, scene);
        set_scene_up(h, scene);
        reset_lighting(h, scene);
        link_layers(h, ebp, scene);
        select_shadows(h, ebp, scene);
        clear_shadow_params(h, scene);
        if h.read::<u32>(scene + 0x10d4) != 0 {
            shadow_params(h, ebp, scene);
            let manager = cdecl(h, SHADOW_INSTANCE, &[]);
            h.write(manager + 0x4064, 0u8);
        }
        let name = tagged(h, scene + 0xfaf0);
        let instance = cdecl(h, INSTANCE_6DC800, &[]);
        method(h, ON_MAP_NAME, instance, &[name]);
        0
    }

    /// A pointer word with its low (tag) bit cleared.
    fn tagged(h: &impl Mem, at: u32) -> u32 {
        let w: u32 = h.read(at);
        w.wrapping_sub(u32::from(h.read::<u8>(at) & 1))
    }

    /// Step 1: `FUN_006adf70` on the map, the four copies for
    /// `DepthBiasHooks_onSceneLoaded`, then the asset rebinds of
    /// `+0x10d8` and `+0x112c`.
    pub fn copy_and_rebind<H: Host>(
        h: &mut H,
        ebp: u32,
        scene: u32,
        on_scene_loaded: impl FnOnce(&mut H, &[u32; ON_SCENE_LOADED_WORDS]),
    ) {
        let map: u32 = h.read(scene + 0xfaf4);
        let helper: u32 = h.read(h.rebase(RENDER_DELEGATION_HELPER));
        method(h, DELEGATION_RESET, helper, &[map]);
        let assets = scene + 0x112c;
        for (copy, src, list) in [
            (COPY_A, assets, false),
            (COPY_B, scene + 0x1144, true),
            (COPY_C, scene + 0x10d8, false),
            (COPY_D, scene + 0x10e4, true),
        ] {
            let dst = at(ebp, copy);
            match copy {
                COPY_A => put(h, ebp, F_ARG, dst),
                COPY_B => put(h, ebp, F_MATERIAL, dst),
                COPY_C => put(h, ebp, F_SCAN_LAYER, dst),
                _ => {}
            }
            if list {
                method(h, LIST_COPY, dst, &[src]);
            } else {
                h.write(dst, [0u32; 3]);
                if copy == COPY_A {
                    put(h, ebp, F_ASSETS, assets);
                }
                method(h, ARRAY_ASSIGN, dst, &[src]);
            }
        }
        let name = tagged(h, scene + 0xfaf0);
        put(h, ebp, COPY_NAME, name);
        let mut words = [0u32; ON_SCENE_LOADED_WORDS];
        for (k, w) in (0u32..).zip(words.iter_mut()) {
            *w = h.read(at(ebp, COPY_NAME) + 4 * k);
        }
        on_scene_loaded(h, &words);
        for (count, data) in [(scene + 0x10d8, scene + 0x10e0), (assets, scene + 0x1134)] {
            let mut i = 0u32;
            while i < h.read::<u32>(count) {
                let slot = h.read::<u32>(data) + 4 * i;
                if h.read::<u32>(slot) != 0 {
                    method(h, REBIND, scene, &[slot, 0]);
                }
                i += 1;
            }
        }
    }

    /// Step 2: the layer shapes. Empties `+0x1138` (reserving first when
    /// both its words are negative), then walks each cluster of
    /// `+0x112c` with a layer-object cursor: an object whose `+0x5c` lies
    /// in (0, 1) sets `+0x93d`; one named `scene%dlayer%dShape` gets a
    /// record (flags, object), the array growing to
    /// [`grown`] entries.
    pub fn scan_layer_shapes(h: &mut impl Host, ebp: u32, scene: u32) {
        let shapes = scene + 0x1138;
        put(h, ebp, F_ARG, shapes);
        if h.read::<i32>(shapes) < 0 && h.read::<i32>(shapes + 4) < 0 {
            method(h, ARRAY_RESERVE, shapes, &[0]);
        }
        h.write(shapes, 0u32);
        let clusters = scene + 0x112c;
        let mut index = 0u32;
        put(h, ebp, F_INDEX, index);
        while index < h.read::<u32>(clusters) {
            let list: u32 = h.read(scene + 0x1134);
            let cluster: u32 = h.read(list + 4 * index);
            if cluster != 0 {
                start_cursor(
                    h,
                    ebp,
                    F_CURSOR,
                    cluster + 0x1c,
                    CLASS_LAYER_OBJECT,
                    CURSOR_BEGIN,
                );
                walk_layer_objects(h, ebp, scene);
            }
            index += 1;
            put(h, ebp, F_INDEX, index);
        }
    }

    /// A scene cursor at `ebp + cursor` over the list at `head`, for the
    /// class at `class`: position, block end, stride, count and offset
    /// 0, the first node (0 for an empty list), then `start`.
    fn start_cursor(h: &mut impl Host, ebp: u32, cursor: i32, head: u32, class: u32, start: u32) {
        put(h, ebp, cursor + 0x14, head);
        let first: u32 = h.read(head);
        let node = if first == head { 0 } else { first };
        h.write(at(ebp, cursor), [0u32; 5]);
        put(h, ebp, cursor + 0x18, node);
        let class = h.rebase(class);
        put(h, ebp, cursor + 0x1c, class);
        method(h, start, at(ebp, cursor), &[]);
    }

    /// The layer-object cursor's loop (its position and count advanced
    /// inline, `PhyreSceneCursor_Next` when a run ends).
    fn walk_layer_objects(h: &mut impl Host, ebp: u32, scene: u32) {
        let c = F_CURSOR;
        let mut count = get(h, ebp, c + 0xc);
        while count != 0 {
            let mut pos = get(h, ebp, c);
            let mut end = get(h, ebp, c + 4);
            'objects: loop {
                let object = get(h, ebp, c + 0x10).wrapping_add(pos);
                put(h, ebp, F_OBJECT, object);
                layer_object(h, ebp, scene, object);
                // the original tests the count here too; it is never 0
                loop {
                    pos = pos.wrapping_add(get(h, ebp, c + 8));
                    count = count.wrapping_sub(1);
                    put(h, ebp, c, pos);
                    put(h, ebp, c + 0xc, count);
                    if end == pos {
                        end = h.read(end);
                        put(h, ebp, c + 4, end);
                        if count != 0 {
                            continue;
                        }
                    } else if count != 0 {
                        continue 'objects;
                    }
                    break 'objects;
                }
            }
            method(h, CURSOR_NEXT, at(ebp, c), &[]);
            count = get(h, ebp, c + 0xc);
        }
    }

    /// One layer object: the (0, 1) test and the shape record.
    fn layer_object(h: &mut impl Host, ebp: u32, scene: u32, object: u32) {
        let bits: u32 = h.read(object + 0x5c);
        // `fld`/`fstp` into the frame: a signalling NaN comes out quiet
        put(h, ebp, F_TEMP, x87::store(x87::load(bits)));
        if is_partial(bits) {
            h.write(scene + 0x93d, 1u8);
        }
        let name = tagged(h, object + 0x2c);
        let sscanf: u32 = h.read(h.rebase(SSCANF_SLOT));
        let format = h.rebase(STR_LAYER_SHAPE);
        let matched = h.call(
            sscanf,
            Conv::Cdecl,
            &[name, format, at(ebp, F_SCAN_SCENE), at(ebp, F_SCAN_LAYER)],
        );
        if matched != 2 {
            return;
        }
        let scene_no: u8 = h.read(at(ebp, F_SCAN_SCENE));
        let layer: u16 = h.read(at(ebp, F_SCAN_LAYER));
        h.write(at(ebp, F_RECORD), 1u8);
        h.write(at(ebp, F_RECORD) + 1, scene_no);
        h.write(at(ebp, F_RECORD) + 2, layer);
        let shapes = scene + 0x1138;
        let n = h.read::<u32>(shapes).wrapping_add(1);
        h.write(shapes, n);
        if n.cast_signed() > h.read::<i32>(shapes + 4) {
            let cap = grown(n.cast_signed());
            put(h, ebp, F_TEMP, cap.cast_unsigned());
            let bytes = cap.cast_unsigned().wrapping_mul(8);
            if h.read::<u32>(shapes + 8) == 0 {
                let data = cdecl(h, USER_MALLOC, &[bytes]);
                h.write(shapes + 8, data);
            } else {
                put(h, ebp, F_NEW_DATA, 0);
                if cap > 0 {
                    let data = cdecl(h, USER_MALLOC, &[bytes]);
                    put(h, ebp, F_NEW_DATA, data);
                    if data != 0 {
                        let old: u32 = h.read(shapes + 8);
                        let len = h.read::<u32>(shapes) << 3;
                        cdecl(h, MEMCPY, &[data, old, len]);
                    }
                }
                let old: u32 = h.read(shapes + 8);
                cdecl(h, USER_FREE, &[old]);
                let data = get(h, ebp, F_NEW_DATA);
                h.write(shapes + 8, data);
            }
            h.write(shapes + 4, cap);
        }
        let n: u32 = h.read(shapes);
        let data: u32 = h.read(shapes + 8);
        let slot = data.wrapping_add(n.wrapping_sub(1).wrapping_mul(8));
        if slot != 0 {
            let record = get(h, ebp, F_RECORD);
            h.write(slot, record);
            let object = get(h, ebp, F_OBJECT);
            h.write(slot + 4, object);
        }
    }

    /// Step 3: texture animations of both clusters, the scene set-up
    /// calls, the clusters' morphs, map 0x17's flag (`+0x10d50`) and the
    /// water-distortion delegation of the main cluster.
    pub fn set_scene_up(h: &mut impl Host, scene: u32) {
        let symbol = h.rebase(STR_TEX_ANIM);
        for (list, cluster, array) in [
            (scene + 0x10e4, scene + 0x10d4, scene + 0x10d8),
            (scene + 0x1144, scene + 0x1128, scene + 0x112c),
        ] {
            let id = method(h, GET_SYMBOL_ID, list, &[symbol]);
            if id != u32::MAX {
                let cluster: u32 = h.read(cluster);
                let manager = cdecl(h, TEX_ANIM_INSTANCE, &[]);
                method(
                    h,
                    ADD_ANIMATED_MATERIALS,
                    manager,
                    &[cluster, array, 0, id, 0, 0, 0],
                );
            }
        }
        h.write(scene + 0x4f8, 0u32);
        for f in SCENE_SETUP {
            method(h, f, scene, &[]);
        }
        for cluster in [scene + 0x10d4, scene + 0x1128] {
            let cluster: u32 = h.read(cluster);
            if cluster != 0 {
                let manager = cdecl(h, MORPH_INSTANCE, &[]);
                method(h, REGISTER_MORPHS, manager, &[cluster]);
            }
        }
        if h.read::<u32>(h.rebase(CURRENT_MAP_NO)) == 0x17 {
            h.write(scene + 0x10d50, 1u8);
        }
        if h.read::<u32>(scene + 0x10d4) != 0 {
            let rec = cdecl(h, OPERATOR_NEW, &[0x14]);
            if rec != 0 {
                h.write(rec, [0u32; 5]);
            }
            // a failed allocation writes through 0, as the original
            let main: u32 = h.read(scene + 0x10d4);
            h.write(rec, main);
            let v: u32 = h.read(scene + 0xd0);
            h.write(rec + 4, v);
            let cluster: u32 = h.read(scene + 0x10cc);
            let handle: u32 = h.read(cluster + 0x8c);
            let tex = if h.read::<u8>(handle + 4) == 0 {
                h.read(handle)
            } else {
                0
            };
            h.write(rec + 8, tex);
            let cluster: u32 = h.read(scene + 0x10cc);
            let v: u32 = h.read(cluster + 0xa4);
            h.write(rec + 0xc, v);
            let cluster: u32 = h.read(scene + 0x10cc);
            let v: u32 = h.read(cluster + 0x9c);
            h.write(rec + 0x10, v);
            let helper: u32 = h.read(h.rebase(RENDER_DELEGATION_HELPER));
            let callback = h.rebase(WATER_DISTORTION);
            method(h, REGISTER_DELEGATION, helper, &[rec, 0x14, 0x41, callback]);
            cdecl(h, OPERATOR_DELETE, &[rec]);
        }
    }

    /// Step 4: the lighting constants (`+0xfc88..+0xfccc`), the map's YIQ
    /// and character lighting, the global camera's position into the
    /// view (`+0xcc`) and `+0x10d30`, and the fog.
    pub fn reset_lighting(h: &mut impl Host, scene: u32) {
        let one = 1.0f32.to_bits();
        for (k, v) in (0u32..).zip([
            one, one, one, one, 0, 0, 0, 0, one, one, one, one, 0, 0, 0, 0,
        ]) {
            h.write(scene + 0xfc8c + 4 * k, v);
        }
        h.write(scene + 0xfc88, 0u8);
        method(h, UPDATE_MAP_YIQ, scene, &[]);
        let no_2d = u32::from(h.read::<u32>(scene + 0x1128) == 0);
        method(h, SETUP_CHARACTER_LIGHTING, scene, &[no_2d]);
        let camera: u32 = h.read(scene + 0x4f8);
        if camera != 0 {
            let view: u32 = h.read(scene + 0xcc);
            copy_floats(h, camera + 0xf4, view + 0x30);
            let view: u32 = h.read(scene + 0xcc);
            copy_floats(h, view + 0x30, scene + 0x10d30);
        }
        method(h, SETUP_MAP_FOG, scene, &[]);
    }

    /// Three floats copied through the x87 stack.
    fn copy_floats(h: &mut impl Mem, from: u32, to: u32) {
        for k in 0..3 {
            let bits: u32 = h.read(from + 4 * k);
            h.write(to + 4 * k, x87::store(x87::load(bits)));
        }
    }

    /// Step 5: the layer tree at `+0x1198`, from its leftmost node: each
    /// node's objects (`+0x10`, an array) point their `+0x1c` record back
    /// at themselves when their layer (`+0x48`) is in the mask `+0xfb14`,
    /// else clear it. Leaves `ebp + 8` at the head.
    pub fn link_layers(h: &mut impl Host, ebp: u32, scene: u32) {
        let head = scene + 0x1198;
        put(h, ebp, F_TREE_HEAD, head);
        let x: u32 = h.read(head + 0xc);
        let start = if x == head || x == 0 {
            x
        } else {
            let mut node = x;
            loop {
                let left: u32 = h.read(node);
                if left == head {
                    break node;
                }
                node = left;
            }
        };
        put(h, ebp, F_TREE_NODE, start);
        put(h, ebp, F_ARG, start);
        let mut next = start;
        if start != head {
            next = method(h, TREE_SUCCESSOR, head, &[start]);
            put(h, ebp, F_ARG, next);
        }
        put(h, ebp, F_TREE_NEXT, next);
        if start == head {
            return;
        }
        let mut node = start;
        loop {
            link_node(h, scene, node);
            node = next;
            if next != head {
                next = method(h, TREE_SUCCESSOR, head, &[next]);
            }
            put(h, ebp, F_ARG, next);
            put(h, ebp, F_TREE_NEXT, next);
            if node == head {
                break;
            }
        }
        put(h, ebp, F_TREE_NODE, node);
    }

    fn link_node(h: &mut impl Mem, scene: u32, node: u32) {
        let objects: u32 = h.read(node + 0x10);
        let mut i = 0u32;
        while i < h.read::<u32>(objects) {
            let data: u32 = h.read(objects + 8);
            let object: u32 = h.read(data + 4 * i);
            // the original reads the layer before testing the object
            let layer: u32 = h.read(object + 0x48);
            let on = h.read::<u32>(scene + 0xfb14) & (1u32 << (layer & 31)) != 0;
            if object != 0 {
                let rec: u32 = h.read(object + 0x1c);
                if rec != 0 {
                    h.write(rec + 0x1c, if on { object } else { 0 });
                }
            }
            i += 1;
        }
    }

    /// Step 6: the 2D scene number, `+0xf9c0` by map, the shadow solution
    /// (`+0xf9cc`) and the shadow cluster `+0x28c4` moved between the
    /// worlds `+0x7c` and `+0xbc` (`+0xf9d0`: in `+0x7c`; `+0xf9d1`:
    /// removed).
    pub fn select_shadows(h: &mut impl Host, ebp: u32, scene: u32) {
        if h.read::<u32>(scene + 0x1128) != 0 {
            let n: u32 = h.read(scene + 0x11d8);
            method(h, CHANGE_2D_SCENE_NUM, scene, &[n]);
        }
        let bias = match h.read::<u32>(scene + 0xfaf4) {
            0xec1 => MAP_EC1_BIAS,
            0xc58 => MAP_C58_BIAS,
            _ => 0,
        };
        let flag: u8 = h.read(scene + 0x1196);
        h.write(scene + 0xf9c0, bias);
        let a5: u32 = h.read(scene + 0xd0);
        h.write(scene + 0x10d2d, 1u8);
        // only the byte: the word's top bytes are what the tree walk left
        h.write(at(ebp, F_ARG), flag);
        let scenario = cdecl(h, GET_SCENERIO_FLAG, &[]) & 0xffff;
        let map: u32 = h.read(scene + 0xfaf4);
        let a0 = u32::from(h.read::<u8>(scene + 0x93c));
        let map_id: u32 = h.read(h.rebase(GRAPHIC_MAP_ID));
        let a1 = get(h, ebp, F_ARG);
        let manager = cdecl(h, SHADOW_INSTANCE, &[]);
        let solution = method(
            h,
            SELECT_SHADOW_SOLUTION,
            manager,
            &[a0, a1, map_id, map, scenario, a5],
        );
        h.write(scene + 0xf9cc, solution);
        let cluster = scene + 0x28c4;
        let (main, other) = (scene + 0x7c, scene + 0xbc);
        let in_main = h.read::<u8>(scene + 0xf9d0) != 0;
        let removed = h.read::<u8>(scene + 0xf9d1) != 0;
        match solution {
            4 => {
                method(
                    h,
                    REMOVE_CLUSTER,
                    if in_main { main } else { other },
                    &[cluster],
                );
                h.write(scene + 0xf9d1, 1u8);
            }
            1 | 3 => {
                if removed {
                    h.write(scene + 0xf9d1, 0u8);
                } else if in_main {
                    method(h, REMOVE_CLUSTER, main, &[cluster]);
                } else {
                    return;
                }
                method(h, ADD_CLUSTER, other, &[cluster]);
                h.write(scene + 0xf9d0, 0u8);
            }
            _ => {
                if removed {
                    h.write(scene + 0xf9d1, 0u8);
                } else if in_main {
                    return;
                } else {
                    method(h, REMOVE_CLUSTER, other, &[cluster]);
                }
                method(h, ADD_CLUSTER, main, &[cluster]);
                h.write(scene + 0xf9d0, 1u8);
            }
        }
    }

    /// Step 7: the five shadow-parameter arrays emptied (their memory
    /// freed unless bit 31 says it is not theirs).
    pub fn clear_shadow_params(h: &mut impl Host, scene: u32) {
        for off in [0x38a4, 0x38ac, 0x38b4, 0x38bc, 0x38c4] {
            let count = h.read::<u32>(scene + off) & 0x8000_0000;
            h.write(scene + off, count);
            if count == 0 {
                let data: u32 = h.read(scene + off + 4);
                cdecl(h, PHYRE_FREE, &[data]);
                h.write(scene + off + 4, 0u32);
            }
        }
    }

    /// Step 8: the averaged caster value, then each material of the main
    /// cluster (a material cursor, advanced inline): its shadow-map,
    /// scale, matrix and caster-value parameters written into its
    /// parameter buffer (`+4`; its first word is the size), and when it
    /// has both matrices, the material and its four parameters pushed
    /// into the arrays of step 7.
    pub fn shadow_params(h: &mut impl Host, ebp: u32, scene: u32) {
        let manager = cdecl(h, SHADOW_INSTANCE, &[]);
        let addr = h.rebase(AVERAGE_CASTER_VALUE);
        let caster = h.call_st0(addr, Conv::Thiscall(manager), &[]);
        put(h, ebp, F_CASTER, caster.to_bits());
        let main: u32 = h.read(scene + 0x10d4);
        start_cursor(h, ebp, F_CURSOR2, main + 0x1c, CLASS_MATERIAL, CURSOR_SEEK4);
        let c = F_CURSOR2;
        let mut count = get(h, ebp, c + 0xc);
        if count == 0 {
            return;
        }
        let mut pos = get(h, ebp, c);
        'materials: loop {
            material(h, ebp, scene, pos);
            // the original tests the count here too; it is never 0
            let mut end = get(h, ebp, c + 4);
            loop {
                pos = pos.wrapping_add(get(h, ebp, c + 8));
                count = count.wrapping_sub(1);
                put(h, ebp, c, pos);
                put(h, ebp, c + 0xc, count);
                if end == pos {
                    end = h.read(end);
                    put(h, ebp, c + 4, end);
                    if count != 0 {
                        continue;
                    }
                } else if count != 0 {
                    continue 'materials;
                }
                break;
            }
            // the next node holding materials (`PhyreSceneCursor_Next`
            // inlined)
            let Some(node) = next_material_node(h, ebp) else {
                return;
            };
            let first: u32 = h.read(node + 0x2c);
            count = h.read(node + 0x3c);
            let stride: u32 = h.read(first + 0x1c);
            pos = h.read(node + 0x34);
            put(h, ebp, c + 8, stride);
            let offset: u32 = h.read(first + 0x8c);
            put(h, ebp, c + 0x10, offset);
            let mut end: u32 = h.read(node + 8);
            put(h, ebp, c, pos);
            put(h, ebp, c + 0xc, count);
            put(h, ebp, c + 4, end);
            loop {
                if count == 0 {
                    return;
                }
                if end != pos {
                    continue 'materials;
                }
                pos = pos.wrapping_add(get(h, ebp, c + 8));
                end = h.read(end);
                count -= 1;
                put(h, ebp, c, pos);
                put(h, ebp, c + 0xc, count);
                put(h, ebp, c + 4, end);
            }
        }
    }

    /// The material cursor's next node with materials of its class
    /// (`+0x2c` a chain of class entries linked by `+0x40`, `+0x40` the
    /// count), or `None` at the list's end.
    fn next_material_node(h: &mut impl Mem, ebp: u32) -> Option<u32> {
        let c = F_CURSOR2;
        loop {
            let node = get(h, ebp, c + 0x18);
            if node == 0 {
                return None;
            }
            let next: u32 = h.read(node);
            let next = if next == get(h, ebp, c + 0x14) {
                0
            } else {
                next
            };
            put(h, ebp, c + 0x18, next);
            if next == 0 {
                return None;
            }
            let mut entry: u32 = h.read(next + 0x2c);
            let mut found = false;
            while entry != 0 {
                if entry == get(h, ebp, c + 0x1c) {
                    found = true;
                    break;
                }
                entry = h.read(entry + 0x40);
            }
            if found && h.read::<u32>(next + 0x40) != 0 {
                return Some(next);
            }
        }
    }

    /// The texture of a shadow-map handle (none when its `+4` byte is
    /// set).
    fn handle_texture(h: &impl Mem, handle: u32) -> u32 {
        if h.read::<u8>(handle + 4) == 0 {
            h.read(handle)
        } else {
            0
        }
    }

    /// A shadow-map block at `ebp + block` for the parameter `param`:
    /// 0x42, 0, the linear sampler, the texture of cascade `cascade` (or
    /// of the dummy map), copied into the material's buffer when the
    /// parameter is a sampler and the sampler does not hold the texture
    /// already.
    fn map_block(
        h: &mut impl Host,
        ebp: u32,
        scene: u32,
        param: u32,
        block: i32,
        cascade: Option<u32>,
    ) {
        put(h, ebp, block, 0x42);
        let manager = cdecl(h, SHADOW_INSTANCE, &[]);
        let handle = match cascade {
            Some(i) => method(h, GET_CASCADE, manager, &[i]),
            None => method(h, DUMMY_SHADOW_MAP, manager, &[]),
        };
        let tex = handle_texture(h, handle);
        put(h, ebp, block + 0xc, tex);
        let sampler = method(h, GET_LINEAR_SAMPLER, scene, &[2]);
        let material = get(h, ebp, F_ARG);
        let buf: u32 = h.read(material + 4);
        put(h, ebp, block + 8, sampler);
        put(h, ebp, block + 4, 0);
        let kind: u8 = h.read(param + 2);
        if !SAMPLER_TYPES.contains(&kind) {
            return;
        }
        if sampler != 0 && tex != 0 && method(h, TROPHY_INSTALL, sampler, &[tex]) != 0 {
            return;
        }
        write_param(h, param, buf, at(ebp, block));
    }

    /// `memcpy`s `from` into the buffer at the parameter's offset when it
    /// fits (and the offset is past the size word).
    fn write_param(h: &mut impl Host, param: u32, buf: u32, from: u32) {
        let off = u32::from(h.read::<u16>(param + 8));
        let len = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if off + len <= h.read::<u32>(buf) && off >= 4 {
            cdecl(h, MEMCPY, &[buf.wrapping_add(off), from, len]);
        }
    }

    /// One material at cursor position `pos`.
    fn material(h: &mut impl Host, ebp: u32, scene: u32, pos: u32) {
        let solution: u32 = h.read(scene + 0xf9cc);
        let material = get(h, ebp, F_CURSOR2 + 0x10).wrapping_add(pos);
        put(h, ebp, F_ARG, material);
        let params: u32 = h.read(material);
        let name = h.rebase(STR_SM_SAMPLER);
        let p = method(h, FIND_PARAM, params, &[name]);
        if p != 0 {
            if matches!(solution, 2..=4) {
                map_block(h, ebp, scene, p, F_BLOCK_CASCADE0, Some(0));
            } else {
                map_block(h, ebp, scene, p, F_BLOCK_DUMMY0, None);
            }
        }
        for (name, block) in [
            (STR_SM_SCALE_BIAS, F_SCALE_A),
            (STR_SM_SCALE_BIAS2, F_SCALE_B),
        ] {
            let params: u32 = h.read(material);
            let name = h.rebase(name);
            let p = method(h, FIND_PARAM, params, &[name]);
            if p != 0 {
                scale_block(h, ebp, material, p, block);
            }
        }
        let params: u32 = h.read(material);
        let mut found = [0u32; 4];
        for (f, name) in found.iter_mut().zip([
            STR_LIGHT_VIEW_PROJECTION,
            STR_LVP2,
            STR_MAIN_CHR_Y_POS,
            STR_SM_SAMPLER2,
        ]) {
            let name = h.rebase(name);
            *f = method(h, FIND_PARAM, params, &[name]);
        }
        let [matrix0, matrix1, caster, map1] = found;
        put(h, ebp, F_INDEX, matrix0);
        put(h, ebp, F_NEW_DATA, matrix1);
        put(h, ebp, F_TEMP, caster);
        put(h, ebp, F_PARAM_MAP_1, map1);
        if map1 != 0 {
            if h.read::<u32>(scene + 0xf9cc) == 4 {
                map_block(h, ebp, scene, map1, F_BLOCK_CASCADE1, Some(1));
            } else {
                map_block(h, ebp, scene, map1, F_BLOCK_DUMMY1, None);
            }
        }
        for (p, out, i) in [(matrix0, F_MATRIX_0, 0), (matrix1, F_MATRIX_1, 1)] {
            if p != 0 {
                let manager = cdecl(h, SHADOW_INSTANCE, &[]);
                method(h, GET_CASCADE_MATRIX, manager, &[at(ebp, out), i]);
                let buf: u32 = h.read(get(h, ebp, F_ARG) + 4);
                write_param(h, p, buf, at(ebp, out));
            }
        }
        if caster != 0 {
            let buf: u32 = h.read(get(h, ebp, F_ARG) + 4);
            write_param(h, caster, buf, at(ebp, F_CASTER));
        }
        if matrix0 != 0 && matrix1 != 0 {
            put(h, ebp, F_MATERIAL, material);
            for (k, (array, value)) in [
                (0x38c4, F_MATERIAL),
                (0x38a4, F_INDEX),
                (0x38ac, F_NEW_DATA),
                (0x38bc, F_PARAM_MAP_1),
                (0x38b4, F_TEMP),
            ]
            .into_iter()
            .enumerate()
            {
                let count: u32 = h.read(scene + array);
                let size = ((count & 0x7fff_ffff) + 1) << 2;
                let memory = cdecl(h, PHYRE_MALLOC, &[size, 4]);
                let push = if k == 0 { PUSH_INTO_FIRST } else { PUSH_INTO };
                method(h, push, scene + array, &[memory, at(ebp, value)]);
            }
        }
    }

    /// A scale block at `ebp + block`: the border scale twice, the texel
    /// scale twice (six `GetShadowSize` calls, in the original's order),
    /// copied into the material's buffer.
    fn scale_block(h: &mut impl Host, ebp: u32, material: u32, param: u32, block: i32) {
        let mut size = || cdecl(h, GET_SHADOW_SIZE, &[]).cast_signed();
        let (s1, s2) = (size(), size());
        let a = border_scale(s1, s2);
        let (s3, s4) = (size(), size());
        let b = border_scale(s3, s4);
        let c = texel_scale(size());
        let d = texel_scale(size());
        h.write(at(ebp, block), [a, b, c, d]);
        let buf: u32 = h.read(material + 4);
        write_param(h, param, buf, at(ebp, block));
    }
}
