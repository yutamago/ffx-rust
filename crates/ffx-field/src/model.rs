//! `rcbg` scene objects: models and their primitive packets, layers,
//! texture transfers (`trx`), palettes, animations and particle
//! simulations (`docs/formats/field.md`).

use ffx_bridge::Global;

/// Bytes per primitive record of each packet type (`g_GuidePacketStrides`,
/// 8 bytes, in `.data`).
pub const PACKET_STRIDES: Global<u8> = Global::new(0x00c5_e790);

/// A model (`rcbgModel`).
pub mod obj {
    /// `+0x14`: its OMD blob (`rcbgModelCreateFromOMD`).
    pub const OMD: u32 = 0x14;
    /// `+0xc0`: the vertex-to-world matrix (`Guide_MeshObject`'s users).
    pub const MATRIX: u32 = 0xc0;
}

/// A model's OMD blob: primitive packets and vertices.
pub mod omd {
    /// `+0x4`: offset of the first packet (`rcbgModelGetPrimHeader`).
    pub const PACKETS: u32 = 0x4;
    /// `+0x8`: offset of the vertices (three `i16` each).
    pub const VERTICES: u32 = 0x8;
    /// `+0x10`: primitives in all packets (`i16`, `rcbgModelSetAlpha`).
    pub const PRIMS: u32 = 0x10;
    /// `+0x12`: vertex count (`u16`).
    pub const VERTEX_COUNT: u32 = 0x12;
    /// `+0x20`: where `rcbgModelSetAlpha` starts walking the packets (it
    /// does not read [`PACKETS`]).
    pub const FIRST_PACKET: u32 = 0x20;
}

/// A primitive packet: a 16-byte header, then `count` records of the
/// type's stride ([`super::PACKET_STRIDES`]).
pub mod packet {
    /// `+0x1`: the type (`i8`, 0..=7). Types 0..=3 have three vertices, 4..=7
    /// four (`rcbgModelSetAlpha`); type 0 records keep their three vertex
    /// indices at `+0xc` (the minimap).
    pub const TYPE: u32 = 0x1;
    /// `+0x2`: records in the packet (`i16`).
    pub const COUNT: u32 = 0x2;
    /// Header size.
    pub const HEADER: u32 = 0x10;
    /// A record starts with one RGBA colour per vertex; the alpha bytes.
    pub const ALPHAS: [u32; 4] = [0x3, 0x7, 0xb, 0xf];
}

/// Bytes a packet takes, header included: `0x10 + stride * count`, with
/// the original's signed arithmetic.
#[must_use]
pub fn packet_size(stride: u8, count: i16) -> u32 {
    #[allow(clippy::cast_sign_loss)] // two's complement, as the original adds it
    let records = (i32::from(stride) * i32::from(count)) as u32;
    packet::HEADER.wrapping_add(records)
}

/// A layer (`rcbgLayerLoad`).
pub mod layer {
    /// `+0x4`: width (`i32`).
    pub const WIDTH: u32 = 0x4;
    /// `+0x8`: height (`i32`).
    pub const HEIGHT: u32 = 0x8;
    /// Where `rcbgLayerLoadCalcCenter` stores half the width.
    pub const CENTER_X: [u32; 3] = [0x68, 0x64, 0x4c];
    /// Where it stores half the height.
    pub const CENTER_Y: [u32; 3] = [0x70, 0x6c, 0x50];
}

/// Half of a layer dimension, as the original computes it: the `i32`
/// rounded to `f32`, then halved.
#[must_use]
#[allow(clippy::cast_precision_loss)] // the original rounds the same way
pub fn half(n: i32) -> u32 {
    crate::x87::store(f64::from(n as f32) * 0.5)
}

/// A texture transfer object (`rcbgTrxInit`, `0x40` bytes).
pub mod trx {
    /// `+0x0`: the object type.
    pub const KIND: u32 = 0x0;
    /// The type `rcbgTrxInit` gives it.
    pub const KIND_TRX: u32 = 0x0002_0002;
    /// `+0x4`: the delete function (`rcbgTrxDelete`, `0x0092d450`).
    pub const DELETE: u32 = 0x4;
    /// `+0xc`: the draw function (`rcbgTrxDraw`, `0x0092d470`).
    pub const DRAW: u32 = 0xc;
    /// `+0x30`: entries (`rcbgTrxGetCnt`).
    pub const COUNT: u32 = 0x30;
    /// `+0x34`: bytes per entry.
    pub const ENTRY_SIZE: u32 = 0x34;
    /// `+0x38`: the entries (`rcbgTrxGetEntry`); an entry's data follows
    /// its 16-byte header (`rcbgTrxGetEntryData`).
    pub const ENTRIES: u32 = 0x38;
    /// `+0x3c`: the transfer to run on the next draw; -1 for none
    /// (`rcbgTrxSetNo`, cleared by `rcbgTrxDraw`).
    pub const NO: u32 = 0x3c;
    /// The original's delete function.
    pub const DELETE_FN: u32 = 0x0092_d450;
    /// The original's draw function.
    pub const DRAW_FN: u32 = 0x0092_d470;
    /// Words `rcbgTrxInit` clears (`+0x14` it leaves).
    pub const CLEARED: [u32; 11] = [
        0x8, 0x10, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38,
    ];
}

/// A palette (`rcbgPalInit`, 16 bytes).
pub mod pal {
    /// `+0x8`: the palette number (`u16`, `rcbgPalSetNo`).
    pub const NO: u32 = 0x8;
}

/// An animation (`rcbgAnimeInit`, 32 bytes).
pub mod anime {
    /// `+0x6`: the current frame (`i16`).
    pub const CURRENT: u32 = 0x6;
    /// `+0x8`: frames (`i32`).
    pub const COUNT: u32 = 0x8;
    /// `+0xc`: the frame entries, 4 bytes each.
    pub const FRAMES: u32 = 0xc;
    /// `+0x14`: set to `0x100` by `rcbgAnimeInit` (the rest is cleared).
    pub const RATE: u32 = 0x14;
}

/// A particle simulation (`rcbgPSimInit`, `0xb0` bytes).
pub mod psim {
    /// Size.
    pub const SIZE: u32 = 0xb0;
    /// The object type `rcbgPSimInit` gives it.
    pub const KIND_PSIM: u32 = 0x0002_0004;
    /// Its delete, process and draw functions (`+4`, `+8`, `+0xc`; the
    /// second is `rcbgPSimProc`).
    pub const FUNCTIONS: [u32; 3] = [0x0091_fd40, 0x0091_fe70, 0x0091_fd60];
    /// Where its model's blob keeps the input particles (`rcbgPSimGetOmdIn`).
    pub const OMD_IN: u32 = 0x20;
    /// `+0x14`: its model.
    pub const MODEL: u32 = 0x14;
    /// `+0xa4`: the initial count (`rcbgPSimSetInitCnt`).
    pub const INIT_CNT: u32 = 0xa4;
    /// `+0x4c`: a position (`rcbgPSimGetPos` 0).
    pub const POS: u32 = 0x4c;
    /// `+0x8c`: the other position (`rcbgPSimGetPos` 1).
    pub const POS2: u32 = 0x8c;
}

/// `user_malloc` (`0x00687020`), through which `rcAlloc` and `yiMalloc`
/// allocate.
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `user_free`'s jump (`0x00687010`), which `rcRealloc` calls directly.
pub const USER_FREE: u32 = 0x0068_7010;
/// `Sg_MemGetSize` (`0x0083c050`): a heap block's size.
pub const SG_MEM_GET_SIZE: u32 = 0x0083_c050;
/// `Sg_MemSetAlign` (`0x0083c4a0`): the next allocation's alignment.
pub const SG_MEM_SET_ALIGN: u32 = 0x0083_c4a0;
/// `rcbgTrxTexDelete` and `rcbgTrxTexDraw`, the texture transfer's slots.
pub const TRX_TEX_DELETE: u32 = 0x0092_20e0;
/// See [`TRX_TEX_DELETE`].
pub const TRX_TEX_DRAW: u32 = 0x0092_2100;
/// `rcbgModelDelete`, `0x009283e0` and `rcbgModelDraw`, the model's
/// slots.
pub const MODEL_DELETE: u32 = 0x0092_8000;
/// See [`MODEL_DELETE`].
pub const MODEL_UPDATE: u32 = 0x0092_83e0;
/// See [`MODEL_DELETE`].
pub const MODEL_DRAW: u32 = 0x0092_8020;
/// `rcbgSceneDraw(scene)` (`0x00920740`).
pub const SCENE_DRAW: u32 = 0x0092_0740;
/// `Yn_SeaGetAngPtr()` (`0x0091adf0`).
pub const YN_SEA_GET_ANG_PTR: u32 = 0x0091_adf0;
/// `pppFpIDdispOn(id)` (`0x007295b0`).
pub const PPP_FP_ID_DISP_ON: u32 = 0x0072_95b0;
/// `pppFpIDdispOff(id)` (`0x00729570`).
pub const PPP_FP_ID_DISP_OFF: u32 = 0x0072_9570;
/// `Fp_End(id)` (`0x00729660`).
pub const FP_END: u32 = 0x0072_9660;
/// `rcbgFPmGetMatrix(fp, out)` (`0x00922290`).
pub const FPM_GET_MATRIX: u32 = 0x0092_2290;
/// -65536.0 (`f64`, `.rdata`): the shakes' and waves' phase scale.
pub const SWING_SCALE: u32 = 0x00b6_77a0;
/// 6.2831855 (`f64` of an `f32`, `.rdata`): a turn.
pub const SWING_TURN: u32 = 0x00b3_9378;
/// `g_pppSinTable` (4096 `f32`s).
pub const SIN_TABLE: u32 = 0x00c4_4bf0;
/// The import table's `rand` (MSVCR110).
pub const RAND_IMPORT: u32 = 0x00b0_c420;
/// `rcbgSceneWaveC0Stop(scene)` (`0x00921200`).
pub const SCENE_WAVE_C0_STOP: u32 = 0x0092_1200;
/// The jump to `rcbgSceneSetChadist` (`0x00920780`).
pub const SCENE_SET_CHADIST_THUNK: u32 = 0x0092_0780;
/// `Yn_MapPartCount()` (`0x0091a3d0`).
pub const YN_MAP_PART_COUNT: u32 = 0x0091_a3d0;
/// `g_CurrentMapNo`.
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
/// `graphic2DHideLayer(n)` (`0x00638f30`).
pub const GRAPHIC_2D_HIDE_LAYER: u32 = 0x0063_8f30;
/// `graphic2DShowLayer(n)` (`0x00638f60`).
pub const GRAPHIC_2D_SHOW_LAYER: u32 = 0x0063_8f60;
/// `graphicTextureAnimationSetMaterialVisiability(n, visible)`.
pub const TEX_ANIM_SET_VISIBILITY: u32 = 0x0064_4120;
/// The import table's `strstr` (MSVCR110).
pub const STRSTR_IMPORT: u32 = 0x00b0_c378;
/// The path `Yn_PreBgLayerDispOn` searches.
pub const PRE_BG_PATH: u32 = 0x023c_bc60;
/// `"te/test20"` (`.rdata`).
pub const TEST20: u32 = 0x00b6_75c4;
/// Set by `Yn_PreBgLayerDispOn` when the path contains it.
pub const TEST20_SEEN: u32 = 0x025d_5ebc;
/// `rcfxSpringFini` (`0x00928c80`).
pub const RCFX_SPRING_FINI: u32 = 0x0092_8c80;
/// `rcfxNailFini` (`0x00929090`).
pub const RCFX_NAIL_FINI: u32 = 0x0092_9090;
/// `Yn_SceneGetObject(kind, n)` (`0x00921be0`).
pub const YN_SCENE_GET_OBJECT: u32 = 0x0092_1be0;
/// `rcbgFPm173End(obj)` (`0x00922b50`).
pub const FPM173_END: u32 = 0x0092_2b50;
/// `rcbgTrxSetNo(trx, n)` (`0x0092d670`).
pub const TRX_SET_NO: u32 = 0x0092_d670;
/// `Ch_GetPos(chr, &x, &y, &z)` (`0x0082ac90`).
pub const CH_GET_POS: u32 = 0x0082_ac90;
/// `graphicSetChadist(distance)` (`0x00642ef0`, a float argument).
pub const GRAPHIC_SET_CHADIST: u32 = 0x0064_2ef0;
/// 98.0 (`f32`, `.rdata`): the character distance without a player.
pub const CHADIST_DEFAULT: u32 = 0x00b5_eee0;
/// 0.1 (`f64` of an `f32`, `.rdata`): view z to character distance.
pub const CHADIST_SCALE: u32 = 0x00b9_21f0;
/// `rcFree` (`0x009258e0`): a jump to `user_free`; the `rcbg*Fini`
/// functions free through it.
pub const RC_FREE: u32 = 0x0092_58e0;
/// The stream word `rcefSetStream` sets (`0x025d5c14`).
pub const REF_STREAM: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5c14);
/// Set to 3 with a stream (`0x025d5be0`).
pub const REF_STREAM_MODE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5be0);
/// 0.03 (`f64`, `.rdata`): `rcfxParticleInit`'s `+8`.
pub const PARTICLE_RATE: u32 = 0x00b5_4cc0;
/// 0.001 and 0.1 (`f32`s, `.rdata`): `rcfxParticleInit`'s `+0x18` and
/// `+0x20`.
pub const PARTICLE_DEFAULTS: [u32; 2] = [0x00b3_68a4, 0x00b1_0f38];
/// The GIF tag word `rcefSetupDraw` copies to `+0x14`.
pub const REF_GIF_TAG: u32 = 0x00c4_98dc;
/// A spring's functions (`rcfxSpringInit`): delete (`0x00928c60`, `+0x4`),
/// `0x00928d70` (`+0x10`) and `0x00928aa0` (`+0x14`).
pub const SPRING_FUNCTIONS: [u32; 3] = [0x0092_8c60, 0x0092_8d70, 0x0092_8aa0];
/// A new spring's stiffness, 0.8 (`.rdata`, `0x00b34254`).
pub const SPRING_STIFFNESS: u32 = 0x00b3_4254;
/// A new spring's damping, 0.4 (`.rdata`, `0x00b10f3c`).
pub const SPRING_DAMPING: u32 = 0x00b1_0f3c;
/// 1000.0 (`f64`, `.rdata`): `rcfxSpringCalc` divides the rest length
/// by it for the shortest length.
pub const SPRING_REST_SCALE: u32 = 0x00b0_dba0;
/// The debug gravity's z, -0.1 (`.rdata`, `0x00b59644`).
pub const GRAVITY_Z: u32 = 0x00b5_9644;
/// `rcbgSetReg(record)` (`0x00920450`), a `ret` in this build.
pub const RCBG_SET_REG: u32 = 0x0092_0450;
/// `Ch_ElemGetMatrix(chr, elem, out)` (`0x00833c30`).
pub const CH_ELEM_GET_MATRIX: u32 = 0x0083_3c30;
/// `op_rt_m_yxz(out, m, rotation, translation)` (`0x007e9a30`).
pub const OP_RT_M_YXZ: u32 = 0x007e_9a30;
/// `pppFpGetMngPtr(id, run, mng)` (`0x007292f0`).
pub const PPP_FP_GET_MNG_PTR: u32 = 0x0072_92f0;
/// `Fp_Restart(id)` (`0x00729700`).
pub const FP_RESTART: u32 = 0x0072_9700;
/// `pppFpSetStickChr(fp, chr, bone)` (`0x0072a530`).
pub const PPP_FP_SET_STICK_CHR: u32 = 0x0072_a530;
/// `pppFpResetStick(id)` (`0x0072a120`).
pub const PPP_FP_RESET_STICK: u32 = 0x0072_a120;
/// `pppFpSetStickMap(id, part)` (`0x0072a590`).
pub const PPP_FP_SET_STICK_MAP: u32 = 0x0072_a590;
/// `rcbgFPmGetMatrix(fp, out)` (`0x00922290`).
pub const RCBG_FPM_GET_MATRIX: u32 = 0x0092_2290;
/// Map 0x61's FP list (`rcbgFPm097Proc000`: 9 entries at `0x00c5e4b8`)
/// and its next step (`0x00922600`).
pub const FPM097_START: [u32; 3] = [0x00c5_e4b8, 9, 0x0092_2600];
/// The calls of map 0x61's FP step (`0x00922600`): `rcbgChrFindById`,
/// `rcbgFPmGetElemMatrix`, `rcbgFpFadeInit`, `Yn_SeaGetPosPtr`,
/// `rcbgScaleVector_009224b0`, `rcCopyQword_00922510`, `Ch_GetRot` (in
/// `ST0`), `rcMat4fSetScaleVec3f` and `Yn_SeaFpFade`.
pub mod fpm097 {
    /// `rcbgChrFindById(id)`.
    pub const CHR_FIND_BY_ID: u32 = 0x0092_e1e0;
    /// `rcbgFPmGetElemMatrix(out, chr, elem)`.
    pub const GET_ELEM_MATRIX: u32 = 0x0092_2350;
    /// `rcbgFpFadeInit(fade, id, step, frames)`.
    pub const FADE_INIT: u32 = 0x0092_2930;
    /// `Yn_SeaGetPosPtr()`.
    pub const SEA_GET_POS_PTR: u32 = 0x0091_ae10;
    /// `rcbgScaleVector_009224b0(out, v, scale)`.
    pub const SCALE_VECTOR: u32 = 0x0092_24b0;
    /// `rcCopyQword_00922510(dst, src)`.
    pub const COPY_QWORD: u32 = 0x0092_2510;
    /// `Ch_GetRot(chr)`, a float in `ST0`.
    pub const CH_GET_ROT: u32 = 0x0082_ace0;
    /// `rcMat4fSetScaleVec3f(out, v)`.
    pub const SET_SCALE_VEC3F: u32 = 0x0092_5820;
    /// `Yn_SeaFpFade(fade)`.
    pub const SEA_FP_FADE: u32 = 0x0092_2970;
    /// `FFXVu0UnitMatrix(m)`, `FFXVu0MulMatrix(out, a, b)` and
    /// `FFXVu0CopyMatrix(dst, src)`.
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    /// See [`UNIT_MATRIX`].
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// See [`UNIT_MATRIX`].
    pub const COPY_MATRIX: u32 = 0x006e_d360;
    /// The two fades' steps, -0.15 and 0.15 (`f32`s, `.rdata`).
    pub const FADE_STEPS: [u32; 2] = [0x00b6_78f4, 0x00b4_7c60];
    /// 651.8986 (`f32`, `.rdata`): the sea's angle as a rotation.
    pub const SEA_ANGLE_SCALE: u32 = 0x00b6_78f0;
    /// 11.377778 (`f32`, `.rdata`): an FP's rotation scale.
    pub const FP_ANGLE_SCALE: u32 = 0x00b6_78ec;
    /// 651.8986 and 1024.0 (`f64`s, `.rdata`): a character's rotation
    /// (`Ch_GetRot`) to the FP's y rotation, a quarter turn on.
    pub const ROT_TO_ANGLE: [u32; 2] = [0x00b5_1590, 0x00b5_1ba8];
    /// Bytes per FP list entry: `+0` the FP id, `+2` the character id,
    /// `+4` the element (`i16`s), `+6` the kind (3 on the sea, 4 on a
    /// character), `+8` the offset, `+0x18` the rotation, `+0x28` the
    /// scale.
    pub const ENTRY_SIZE: u32 = 0x38;
}

/// Map 0x235's FP list (`rcbgFPm565Proc000`: 3 entries at `0x00c5e6b0`)
/// and its next step (`0x00922fe0`).
pub const FPM565_START: [u32; 3] = [0x00c5_e6b0, 3, 0x0092_2fe0];
/// The calls of map 0x235's FP step (`0x00922fe0`).
pub mod fpm565 {
    /// `Yn_SeaGetMatrix()`.
    pub const SEA_GET_MATRIX: u32 = 0x0091_ae00;
    /// `rcbgFpGetWork(id, part, k)`.
    pub const FP_GET_WORK: u32 = 0x0092_3100;
    /// `rcbgFPmGetPartData(id, n, part, k)`.
    pub const GET_PART_DATA: u32 = 0x0092_3190;
    /// `pppSetMatrix(run)`.
    pub const PPP_SET_MATRIX: u32 = 0x0071_7410;
    /// `g_pppWorldMatrix`.
    pub const PPP_WORLD_MATRIX: u32 = 0x0230_ffa0;
    /// `yiSetMatrixDObj(index, m)`.
    pub const SET_MATRIX_DOBJ: u32 = 0x0090_a520;
    /// 32768.0 (`f64`, `.rdata`): half a turn, the part's angles from.
    pub const HALF_TURN: u32 = 0x00b5_1588;
}
/// `rcbgFPmRestart(fp, chr, bone)` (`0x00922300`).
pub const RCBG_FPM_RESTART: u32 = 0x0092_2300;
/// `Ch_OptposGetElement(chr, n, k)` (`0x00835660`).
pub const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
/// The palette configuration keywords (`0x00c60e94`, a null-ended table).
pub const PAL_CONFIG_KEYS: u32 = 0x00c6_0e94;
/// `rcbgAnimeLoadConfig`'s keywords (`0x00c60e84`: `DataCnt`, ...).
pub const ANIME_CONFIG_KEYS: u32 = 0x00c6_0e84;
/// `rcbgAnimeSetDataCnt(anime, n)` (`0x009308d0`).
pub const ANIME_SET_DATA_CNT: u32 = 0x0093_08d0;
/// The text sub-configuration keywords (`0x00c60e44`).
pub const TEXT_SUB_KEYS: u32 = 0x00c6_0e44;
/// The field programs' degree-to-radian factor (`.rdata` float,
/// `0x00b678ec`).
pub const FPM_ANGLE_SCALE: u32 = 0x00b6_78ec;
/// The two buffers `rcefSetupDraw` points `+0x34` and `+0x38` at.
pub const REF_BUFFERS: [u32; 2] = [0x0230_fd60, 0x0230_fda0];
/// `rcfxNail`'s callbacks (`rcfxNailInit`: `+4`, `+0x10`, `+0x14`).
pub const NAIL_CALLBACKS: [u32; 3] = [0x0092_9070, 0x0092_9190, 0x0092_8f40];
/// `rcfxWind`'s callbacks (`rcfxWindInit`: `+4`, `+0x14`).
pub const WIND_CALLBACKS: [u32; 2] = [0x0092_9840, 0x0092_96d0];
/// `rcfxGravity`'s update callback (`0x00929560`).
pub const GRAVITY_UPDATE: u32 = 0x0092_9560;
/// `rcfxGravity`'s draw callback (`0x00929500`).
pub const GRAVITY_DRAW: u32 = 0x0092_9500;
/// The chunk tags `rcbgChkId` looks up: a null-terminated table of
/// pointers to 4-character ids.
pub const CHUNK_IDS: u32 = 0x00c5_e480;
/// The GS pixel format per TIM2 image type (`rcdtTm2GetPsm`).
pub const TM2_PSM: u32 = 0x00c5_e754;
/// The draw function `rcbgSceneDrawLayer` queues.
pub const DRAW_LAYER_FUNC: u32 = 0x0092_0850;

/// The per-map scene parts of `rcbgFP.c` (`rcbgFPm*Init`) and `rcfxDamp`.
pub mod fp {
    /// The parts' object type.
    pub const KIND: u32 = 0x0002_0008;
    /// `rcbgFPm097Init`'s functions.
    pub const M097: [u32; 2] = [0x0092_2540, 0x0092_25d0];
    /// `rcbgFPm173Init`'s functions.
    pub const M173: [u32; 2] = [0x0092_2b40, 0x0092_2be0];
    /// `rcbgFPm565Init`'s functions.
    pub const M565: [u32; 2] = [0x0092_2f30, 0x0092_2fb0];
    /// `rcfxDampInit`'s functions (`+4`, `+0x14`).
    pub const DAMP_FUNCTIONS: [u32; 2] = [0x0092_93c0, 0x0092_9360];
    /// Its factor, an `f32` in `.rdata`.
    pub const DAMP_FACTOR: u32 = 0x00b9_2154;
}

/// A model's scale (`rcbgModelSetScale` stores ten times it on the
/// matrix diagonal; `rcbgModelGetScale` divides by 10).
pub const SCALE_FACTOR: f64 = 10.0;

/// `rcPad16(n)`: bytes from `n` to the next multiple of 16, with the C
/// remainder (a negative `n` gives 17..=31).
#[must_use]
pub const fn pad16(n: i32) -> i32 {
    let r = n % 16;
    if r != 0 { 16 - r } else { 0 }
}

/// `rcLog2Ceil(n)`: the smallest `k` in 1..=16 with `n <= 1 << k`
/// (16 when none is).
#[must_use]
pub fn log2_ceil(n: i32) -> u32 {
    let mut k = 1u32;
    let mut p = 2u32;
    loop {
        if n <= p.cast_signed() {
            return k;
        }
        p = p.rotate_left(1);
        k += 1;
        if k >= 16 {
            return k;
        }
    }
}

/// The scene objects where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::Reg;
    use ffx_system::stub::ptr32;

    use super::{
        PACKET_STRIDES, anime, half, layer, obj, omd, packet, packet_size, pal, psim, trx,
    };

    fn stride(mem: &impl Mem, kind: i8) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        // a negative type indexes before the table, as the original
        let at = PACKET_STRIDES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(kind) as u32);
        mem.read(at)
    }

    /// `rcbgModelCreateFromOMD(model, omd)` (`0x00927d70`).
    pub fn rcbg_model_create_from_omd(mem: &mut impl Mem, m: u32, blob: u32) {
        mem.write(m + obj::OMD, blob);
    }

    /// `rcbgModelGetPrimHeader(model, n)` (`0x009282c0`): the header of
    /// packet `n` (0 or negative: the first).
    pub fn rcbg_model_get_prim_header(mem: &impl Mem, m: u32, n: i32) -> u32 {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut p = blob.wrapping_add(mem.read::<u32>(blob + omd::PACKETS));
        for _ in 0..n.max(0) {
            let s = stride(mem, mem.read(p + packet::TYPE));
            p = p.wrapping_add(packet_size(s, mem.read(p + packet::COUNT)));
        }
        p
    }

    /// `rcbgModelGetPrim(model, n)` (`0x009282a0`): packet `n`'s records.
    pub fn rcbg_model_get_prim(mem: &impl Mem, m: u32, n: i32) -> u32 {
        rcbg_model_get_prim_header(mem, m, n).wrapping_add(packet::HEADER)
    }

    /// `rcbgModelSetAlpha(model, alpha)` (`0x00928460`): every vertex
    /// colour's alpha, packet by packet until the blob's primitive count is
    /// used up.
    pub fn rcbg_model_set_alpha(mem: &mut impl Mem, m: u32, alpha: u8) {
        let blob = mem.read::<u32>(m + obj::OMD);
        let mut left = i32::from(mem.read::<i16>(blob + omd::PRIMS));
        let mut p = blob + omd::FIRST_PACKET;
        while left > 0 {
            let kind = mem.read::<i8>(p + packet::TYPE);
            let count = i32::from(mem.read::<i16>(p + packet::COUNT));
            let mut r = p.wrapping_add(packet::HEADER);
            for _ in 0..count.max(0) {
                let corners = if kind >= 4 { 4 } else { 3 };
                for &o in &packet::ALPHAS[..corners] {
                    mem.write(r.wrapping_add(o), alpha);
                }
                r = r.wrapping_add(u32::from(stride(mem, kind)));
            }
            left -= count;
            p = r;
        }
    }

    /// `rcbgLayerLoadCalcCenter(layer)` (`0x0092b270`): half the width and
    /// height, each stored three times.
    pub fn rcbg_layer_load_calc_center(mem: &mut impl Mem, l: u32) {
        let x = half(mem.read(l + layer::WIDTH));
        let y = half(mem.read(l + layer::HEIGHT));
        for o in layer::CENTER_X {
            mem.write(l + o, x);
        }
        for o in layer::CENTER_Y {
            mem.write(l + o, y);
        }
    }

    /// `rcbgTrxInit(trx)` (`0x0092d510`).
    pub fn rcbg_trx_init(mem: &mut impl Mem, t: u32) {
        mem.write(t + trx::KIND, trx::KIND_TRX);
        let delete = mem.rebase(trx::DELETE_FN);
        let draw = mem.rebase(trx::DRAW_FN);
        mem.write(t + trx::DELETE, delete);
        mem.write(t + trx::DRAW, draw);
        for o in trx::CLEARED {
            mem.write(t + o, 0u32);
        }
        mem.write(t + trx::NO, u32::MAX);
    }

    /// `rcbgTrxSetNo(trx, no)` (`0x0092d670`).
    pub fn rcbg_trx_set_no(mem: &mut impl Mem, t: u32, no: u32) {
        mem.write(t + trx::NO, no);
    }

    /// `rcbgPalInit(pal)` (`0x009299b0`).
    pub fn rcbg_pal_init(mem: &mut impl Mem, p: u32) {
        mem.write(p, [0u32; 4]);
    }

    /// `rcbgPalSetNo(pal, no)` (`0x00929ab0`).
    pub fn rcbg_pal_set_no(mem: &mut impl Mem, p: u32, no: u16) {
        mem.write(p + pal::NO, no);
    }

    /// `rcbgAnimeInit(anime)` (`0x009308a0`).
    pub fn rcbg_anime_init(mem: &mut impl Mem, a: u32) {
        mem.write(a, [0u32; 8]);
        mem.write(a + anime::RATE, 0x100u32);
    }

    /// `rcbgPSimSetInitCnt(psim, n)` (`0x0091fef0`).
    pub fn rcbg_psim_set_init_cnt(mem: &mut impl Mem, p: u32, n: u32) {
        mem.write(p + psim::INIT_CNT, n);
    }

    /// `rcbgModelGetScale(model)` (`0x00928300`): the matrix's first
    /// element over 10.
    pub fn rcbg_model_get_scale(mem: &impl Mem, m: u32) -> f32 {
        let x = crate::x87::load(mem.read(m + obj::MATRIX));
        f32::from_bits(crate::x87::store(x / super::SCALE_FACTOR))
    }

    /// `rcbgTrxGetCnt(trx)` (`0x0092d4c0`).
    pub fn rcbg_trx_get_cnt(mem: &impl Mem, t: u32) -> u32 {
        mem.read(t + trx::COUNT)
    }

    /// `rcbgTrxGetEntry(trx, n)` (`0x0092d4d0`).
    pub fn rcbg_trx_get_entry(mem: &impl Mem, t: u32, n: u32) -> u32 {
        let size = mem.read::<u32>(t + trx::ENTRY_SIZE);
        size.wrapping_mul(n)
            .wrapping_add(mem.read(t + trx::ENTRIES))
    }

    /// `sceDmaGetChan(n)`, `FlushCache(n)`, `sceDmaSend(chan, packet)` and
    /// `sceDmaSync(chan, mode, timeout)` (`0x0062f6e0`, `0x0062f4c0`,
    /// `0x0062f710`, `0x0062f730`): [`rcbg_trx_draw`]'s DMA calls.
    pub const TRX_DMA_CALLS: [u32; 4] = [0x0062_f6e0, 0x0062_f4c0, 0x0062_f710, 0x0062_f730];

    /// `rcbgTrxDraw(trx)` (`0x0092d230`): the transfer's upload packet at
    /// the GIF packet buffer ([`super::REF_GIF_TAG`]): a 5-quad A+D GIF tag
    /// (`0x10000005`), `BITBLTBUF` (0x50, the transfer's `+0x18`/`+0x1c`),
    /// `TRXPOS` (0x51, `+0x20`/`+0x24`), `TRXREG` (0x52, `+0x28`/`+0x2c`),
    /// `TRXDIR` (0x53, 0), then an image tag of `+0x34 / 16` quads (toward
    /// zero, `u16`) pointing at entry `+0x3c` ([`rcbg_trx_get_entry`]),
    /// sent on DMA channel 2 (`sceDmaGetChan`, `FlushCache(0)`,
    /// `sceDmaSend`, `sceDmaSync(chan, 0, 0)`, whose `eax` it returns).
    pub fn rcbg_trx_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) -> u32 {
        let [get_chan, flush, send, sync] = TRX_DMA_CALLS.map(|f| mem.rebase(f));
        let p = mem.read::<u32>(mem.rebase(super::REF_GIF_TAG));
        let w = |mem: &M, o: u32| mem.read::<u32>(t.wrapping_add(o));
        let regs = [
            [0x1000_0005, 0, 0, 0],
            [w(mem, 0x18), w(mem, 0x1c), 0x50, 0],
            [w(mem, 0x20), w(mem, 0x24), 0x51, 0],
            [w(mem, 0x28), w(mem, 0x2c), 0x52, 0],
            [0, 0, 0x53, 0],
        ];
        for (i, q) in (0u32..).zip(regs) {
            mem.write(p.wrapping_add(16 * i), q);
        }
        let bytes = mem.read::<i32>(t.wrapping_add(0x34));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low word
        mem.write(p.wrapping_add(0x50), (bytes / 16) as u16);
        mem.write(p.wrapping_add(0x52), 0u16);
        let entry = rcbg_trx_get_entry(mem, t, w(mem, 0x3c));
        mem.write(p.wrapping_add(0x54), [entry, 0, 0]);
        let chan = call(mem, get_chan, &[2]);
        call(mem, flush, &[0]);
        let packet = mem.read::<u32>(mem.rebase(super::REF_GIF_TAG));
        call(mem, send, &[chan, packet]);
        call(mem, sync, &[chan, 0, 0])
    }

    /// `rcbgTrxGetEntryData(trx, n)` (`0x0092d4f0`).
    pub fn rcbg_trx_get_entry_data(mem: &impl Mem, t: u32, n: u32) -> u32 {
        rcbg_trx_get_entry(mem, t, n).wrapping_add(0x10)
    }

    /// `rcbgPSimInit(psim)` (`0x0091fe00`).
    pub fn rcbg_psim_init(mem: &mut impl Mem, p: u32) {
        mem.write_bytes(p, &[0; psim::SIZE as usize]);
        mem.write(p, psim::KIND_PSIM);
        for (i, f) in (1u32..).zip(psim::FUNCTIONS) {
            let at = mem.rebase(f);
            mem.write(p + 4 * i, at);
        }
    }

    /// `rcbgPSimGetOmdIn(psim)` (`0x0091fdc0`).
    pub fn rcbg_psim_get_omd_in(mem: &impl Mem, p: u32) -> u32 {
        mem.read::<u32>(p + psim::MODEL).wrapping_add(psim::OMD_IN)
    }

    /// `rcbgFPm173End(fp)` (`0x00922b50`): `+0x24` = 1.
    pub fn rcbg_fpm173_end(mem: &mut impl Mem, f: u32) {
        mem.write(f + 0x24, 1u32);
    }

    /// `rcbgAnimeGetFrame(anime, n)` (`0x00930860`): frame `n`'s entry (4
    /// bytes from `+0xc`), or null when `n` is not below the count at `+8`.
    pub fn rcbg_anime_get_frame(mem: &impl Mem, a: u32, n: i32) -> u32 {
        if n >= mem.read::<i32>(a + anime::COUNT) {
            return 0;
        }
        #[allow(clippy::cast_sign_loss)] // negative indexes back, as the original
        mem.read::<u32>(a + anime::FRAMES)
            .wrapping_add((n as u32).wrapping_mul(4))
    }

    /// `rcbgAnimeGetCurrent(anime)` (`0x00930880`): the entry of the frame
    /// at `+6` (`i16`).
    pub fn rcbg_anime_get_current(mem: &impl Mem, a: u32) -> u32 {
        let n = i32::from(mem.read::<i16>(a + anime::CURRENT));
        #[allow(clippy::cast_sign_loss)]
        mem.read::<u32>(a + anime::FRAMES)
            .wrapping_add((n as u32).wrapping_mul(4))
    }

    /// `rcbgPSimClearOMD(psim)` (`0x0091f9b0`): detaches its model's blob.
    pub fn rcbg_psim_clear_omd(mem: &mut impl Mem, p: u32) {
        let m = mem.read::<u32>(p + psim::MODEL);
        if m != 0 {
            mem.write(m + obj::OMD, 0u32);
        }
    }

    // ---- round 4: particle positions, scene palettes, FP parts, TIM2 ----

    /// `rcbgPSimGetPos(psim, which, out)` (`0x0091fdd0`): the position at
    /// `+0x4c` (`which` 0) or `+0x8c`; returns `FFXVu0CopyVector`'s `eax`.
    pub fn rcbg_psim_get_pos(mem: &mut impl Mem, p: u32, which: u32, out: u32) -> u32 {
        let at = p.wrapping_add(if which == 0 { psim::POS } else { psim::POS2 });
        ffx_ps2::vu0::bridge::copy_vector(mem, out, at)
    }

    /// `rcbgSceneSetPal(scene)` (`0x009210a0`): every layer's palette back
    /// to number 0 (`rcbgPalSetNo` on `+0xb0` of each `0xc0`-byte layer).
    pub fn rcbg_scene_set_pal(mem: &mut impl Mem, s: u32) {
        let layers = mem.read::<u32>(s.wrapping_add(8));
        let mut i = 0i32;
        while i < mem.read::<i32>(s.wrapping_add(4)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let l = layers.wrapping_add(i as u32 * 0xc0);
            rcbg_pal_set_no(mem, l.wrapping_add(0xb0), 0);
            i += 1;
        }
    }

    /// `rcbgChkId(id)` (`0x00921940`): the index of the chunk tag that
    /// `strncmp(id, tag, 4)` matches in the null-terminated table at
    /// `0x00c5e480`, or -1.
    pub fn rcbg_chk_id(mem: &impl Mem, id: u32) -> i32 {
        let table = mem.rebase(super::CHUNK_IDS);
        let mut i = 0u32;
        loop {
            let tag = mem.read::<u32>(table + 4 * i);
            if tag == 0 {
                return -1;
            }
            if strncmp4(mem, id, tag) {
                #[allow(clippy::cast_possible_wrap)]
                return i as i32;
            }
            i += 1;
        }
    }

    /// Whether `strncmp(a, b, 4)` is 0.
    fn strncmp4(mem: &impl Mem, a: u32, b: u32) -> bool {
        for k in 0..4 {
            let (x, y) = (mem.read::<u8>(a + k), mem.read::<u8>(b + k));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
        }
        true
    }

    /// Writes an FP part's header: type `0x20008`, its functions (rebased),
    /// and 0.
    fn fp_header(mem: &mut impl Mem, p: u32, functions: [u32; 2]) {
        mem.write(p, super::fp::KIND);
        let [a, b] = functions.map(|f| mem.rebase(f));
        mem.write(p.wrapping_add(4), a);
        mem.write(p.wrapping_add(8), b);
        mem.write(p.wrapping_add(0xc), 0u32);
    }

    /// `rcbgFPm097Init(part)` (`0x00922560`, map 0x61's part): cleared (64
    /// bytes, `memset`), then the header; returns the part.
    pub fn rcbg_fpm097_init(mem: &mut impl Mem, p: u32) -> u32 {
        mem.write(p, [0u32; 16]);
        fp_header(mem, p, super::fp::M097);
        p
    }

    /// `rcbgFPm173Init(part)` (`0x00922b70`, map 0xad's part): `+0x10..+0x28`
    /// cleared, the header; returns the part.
    pub fn rcbg_fpm173_init(mem: &mut impl Mem, p: u32) -> u32 {
        mem.write(p.wrapping_add(0x10), [0u32; 6]);
        fp_header(mem, p, super::fp::M173);
        p
    }

    /// `AtelGetEventCharHandleFromID(id)` (`0x0086b3a0`, script).
    pub const ATEL_GET_EVENT_CHAR_HANDLE: u32 = 0x0086_b3a0;

    /// `rcbgFPm173Start(part, a, b, c)` (`0x00922ef0`): the characters of
    /// event ids `a` and `b` (`AtelGetEventCharHandleFromID`) at `+0x10`
    /// and `+0x14`, `c` at `+0x18`, then `+0x1c` 0, `+0x20` 1, `+0x24` 0.
    /// Returns `c` (left in `eax`).
    pub fn rcbg_fpm173_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [p, a, b, c]: [u32; 4],
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(ATEL_GET_EVENT_CHAR_HANDLE);
        let h = call(mem, f, &[a]);
        mem.write(p.wrapping_add(0x10), h);
        let h = call(mem, f, &[b]);
        mem.write(p.wrapping_add(0x14), h);
        mem.write(p.wrapping_add(0x18), [c, 0, 1, 0]);
        c
    }

    /// The scene object kind of map 0xad's part (`Yn_SceneGetObject`).
    pub const FPM173_KIND: u32 = 0x0002_0008;

    /// `Yn_FPm173Start_00919a80(a, b, c)` (`0x00919a80`; name ours): the
    /// scene's first [`FPM173_KIND`] object (`Yn_SceneGetObject`), when
    /// there is one, started (`rcbgFPm173Start(part, a, b, c)`, whose `eax`
    /// it returns); else 0.
    pub fn yn_fpm173_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [a, b, c]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let part = yn_scene_get_object(mem, FPM173_KIND, 0);
        if part == 0 {
            return 0;
        }
        let f = mem.rebase(FPM173_START);
        call(mem, f, &[part, a, b, c])
    }

    /// `rcbgFPm173Start(part, a, b, c)` (`0x00922ef0`).
    pub const FPM173_START: u32 = 0x0092_2ef0;

    /// Map 0xad's part's four phases (`+0x18`): `rcbgFPm173ProcJect`,
    /// `rcbgFPm173ProcKonjyo`, `rcbgFPm173ProcSphere` and
    /// `rcbgFPm173ProcKieru` (`0x00922c50`, `0x00922e60`, `0x00922eb0`,
    /// `0x00922dc0`).
    pub const FPM173_PHASES: [u32; 4] = [0x0092_2c50, 0x0092_2e60, 0x0092_2eb0, 0x0092_2dc0];

    /// `rcbgFPm173Proc000(part)` (`0x00922be0`): while started (`+0x20`
    /// non-zero) the phase `+0x18` (0..=3, [`FPM173_PHASES`]) runs on the
    /// part, then the frame count `+0x1c` goes up (also for any other
    /// phase). Returns the phase's `eax`, the phase number for another;
    /// `None` when not started (the caller's `eax` is left).
    pub fn rcbg_fpm173_proc000<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> Option<u32> {
        if mem.read::<u32>(p.wrapping_add(0x20)) == 0 {
            return None;
        }
        let phase = mem.read::<u32>(p.wrapping_add(0x18));
        let eax = match FPM173_PHASES.get(phase as usize) {
            Some(&f) => {
                let f = mem.rebase(f);
                call(mem, f, &[p])
            }
            None => phase,
        };
        let frames = p.wrapping_add(0x1c);
        mem.write(frames, mem.read::<u32>(frames).wrapping_add(1));
        Some(eax)
    }

    /// the header; returns the part.
    pub fn rcbg_fpm565_init(mem: &mut impl Mem, p: u32) -> u32 {
        fp_header(mem, p, super::fp::M565);
        p
    }

    /// `rcfxDampInit(damp)` (`0x009293d0`, `rcfxDamp.c`): type `0x30003`,
    /// its two functions, the factor at `+0x1c` (an `f32` in `.rdata`,
    /// 0.0001); returns the object.
    pub fn rcfx_damp_init(mem: &mut impl Mem, p: u32) -> u32 {
        let k = mem.read::<u32>(mem.rebase(super::fp::DAMP_FACTOR));
        mem.write(p.wrapping_add(0x1c), crate::x87::copy(k));
        mem.write(p, 0x0003_0003u32);
        let a = mem.rebase(super::fp::DAMP_FUNCTIONS[0]);
        mem.write(p.wrapping_add(4), a);
        mem.write(p.wrapping_add(8), [0u32; 3]);
        let b = mem.rebase(super::fp::DAMP_FUNCTIONS[1]);
        mem.write(p.wrapping_add(0x14), b);
        mem.write(p.wrapping_add(0x18), 0u32);
        p
    }

    /// `rcbgFpGetWork(id, part, index)` (`0x00923100`): through the VFX
    /// run record of `id` (`pppFpGetMngPtr`, `run_of`), its data's part
    /// table: `index`'s work offset plus `0xa0` from the data; 0 when a
    /// pointer on the way is null.
    pub fn rcbg_fp_get_work<M: Mem>(
        mem: &mut M,
        id: u32,
        part: u32,
        index: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let run = run_of(mem, id);
        if run == 0 {
            return 0;
        }
        let data = ptr32(mem.read::<u32>(run.wrapping_add(0x30)));
        if data == 0 {
            return 0;
        }
        let parts = ptr32(mem.read::<u32>(data.wrapping_add(8)));
        if parts == 0 {
            return 0;
        }
        let table = ptr32(mem.read::<u32>(parts));
        if table == 0 {
            return 0;
        }
        let entry = (part << 4).wrapping_add(0x28).wrapping_add(table);
        if entry == 0 {
            return 0;
        }
        let works = ptr32(mem.read::<u32>(entry.wrapping_add(0xc)));
        mem.read::<u32>(works.wrapping_add(index.wrapping_mul(4)))
            .wrapping_add(0xa0)
            .wrapping_add(data)
    }

    /// `rcbgSceneDrawLayer(layer)` (`0x00920830`):
    /// `op_func_after_ot3d_set(0x00920850, layer, 0, 1)` (`after_set` with
    /// the rebased function and the layer); returns its result.
    pub fn rcbg_scene_draw_layer<M: Mem>(
        mem: &mut M,
        layer: u32,
        after_set: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let f = mem.rebase(super::DRAW_LAYER_FUNC);
        after_set(mem, f, layer)
    }

    /// `rcbgSceneWaveDCStop(scene)` (`0x00920650`): the wave
    /// `Yn_SceneSetDC` starts ends (`+0xac` 0.0, `+0xdc` 0).
    pub fn rcbg_scene_wave_dc_stop(mem: &mut impl Mem, s: u32) {
        mem.write(s.wrapping_add(0xac), 0f32);
        mem.write(s.wrapping_add(0xdc), 0u32);
    }

    /// `rcbgSceneWaveC0Stop(scene)` (`0x00921200`): the wave
    /// `Yn_SceneSetC0` starts ends (`+0xbc`, `+0xb8` 0.0, `+0xc0` 0).
    pub fn rcbg_scene_wave_c0_stop(mem: &mut impl Mem, s: u32) {
        mem.write(s.wrapping_add(0xbc), 0f32);
        mem.write(s.wrapping_add(0xb8), 0f32);
        mem.write(s.wrapping_add(0xc0), 0u32);
    }

    /// `rcfxParticleClearForces(p)` (`0x009289b0`, from
    /// `rcfxParticleResetForce`): each of the `+0x1c` particles' force
    /// (16 bytes in the array at `+0x2c`) zeroed.
    pub fn rcfx_particle_clear_forces(mem: &mut impl Mem, p: u32) {
        let mut i = 0i32;
        while i < mem.read::<i32>(p.wrapping_add(0x1c)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let off = i as u32 * 0x10;
            for k in [0xc, 8, 4, 0] {
                let a = mem.read::<u32>(p.wrapping_add(0x2c));
                mem.write(a.wrapping_add(off + k), 0f32);
            }
            i += 1;
        }
    }

    /// `rcdtTm2GetPsm(picture)` (`0x00923280`): the GS pixel format of a
    /// TIM2 picture header's image type (`+0x13`) from the table at
    /// `0x00c5e754` (1 → PSMCT16, 2 → PSMCT24, 3 → PSMCT32, 4 → PSMT4,
    /// 5 → PSMT8); 0 for type 0.
    pub fn rcdt_tm2_get_psm(mem: &impl Mem, pic: u32) -> u32 {
        match mem.read::<u8>(pic.wrapping_add(0x13)) {
            0 => 0,
            t => mem.read(mem.rebase(super::TM2_PSM) + 4 * u32::from(t)),
        }
    }

    /// `rcdtTm2ClutAlphaToPs2(clut)` (`0x00923240`): 256 RGBA entries'
    /// alpha from 0..=255 to the GS's 0..=128 (`a * 128 / 255`, rounded
    /// down); an entry with alpha 0 becomes all 0.
    pub fn rcdt_tm2_clut_alpha_to_ps2(mem: &mut impl Mem, clut: u32) {
        for i in 0..256 {
            let at = clut.wrapping_add(4 * i);
            let a = u32::from(mem.read::<u8>(at.wrapping_add(3)));
            if a == 0 {
                mem.write(at, 0u32);
            } else {
                mem.write(at.wrapping_add(3), 0u8);
                let w = mem.read::<u32>(at);
                mem.write(at, w | ((a * 128 / 255) << 24));
            }
        }
    }

    /// `rcdtRsd2Init(rsd)` (`0x00924390`, `rcdtRsd2.c`): eight words
    /// cleared.
    pub fn rcdt_rsd2_init(mem: &mut impl Mem, p: u32) {
        mem.write(p, [0u32; 8]);
    }

    /// `rcbgTrxSetRegs(t, a..g)` (`0x0092d360`, from `rcbgPalAlloc`'s
    /// set-up): the low words of two GS register values built from
    /// sign-extended `i16`s, `+0x1c` = `((c << 8 | b) << 16) | a`, `+0x24`
    /// = `e << 16 | d`, `+0x28` = `f`, `+0x2c` = `g | f >> 31`, `+0x18` and
    /// `+0x20` 0. Returns `f`.
    #[allow(clippy::too_many_arguments, clippy::cast_sign_loss)]
    pub fn rcbg_trx_set_regs(
        mem: &mut impl Mem,
        t: u32,
        a: i16,
        b: i16,
        c: i16,
        d: i16,
        e: i16,
        f: i16,
        g: i16,
    ) -> u32 {
        let x = |v: i16| i32::from(v) as u32;
        let w1c = ((x(c) << 8 | x(b)) << 16) | x(a);
        mem.write(t.wrapping_add(0x1c), w1c);
        mem.write(t.wrapping_add(0x24), (x(e) << 16) | x(d));
        mem.write(t.wrapping_add(0x2c), x(g) | (i32::from(f) >> 31) as u32);
        mem.write(t.wrapping_add(0x18), 0u32);
        mem.write(t.wrapping_add(0x20), 0u32);
        mem.write(t.wrapping_add(0x28), x(f));
        x(f)
    }

    /// `rcbgClipCodeAnd(out, a, b, c, d)` (`0x0092aed0`, from
    /// `rcbgClipVertex`): `out = (a & c, b & d)`; returns `out`.
    pub fn rcbg_clip_code_and(mem: &mut impl Mem, out: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        mem.write(out, [a & c, b & d]);
        out
    }

    /// `pppVu0MulX(dst, v, vec)`, `pppVu0MaddaY(acc, v, vec)`,
    /// `pppVu0MaddaZ(acc, v, vec)` and `pppVu0MaddW(dst, v, vec)` (the
    /// vector by value), as [`rcbg_clip_vertex`] calls them.
    pub const VU0_MUL_X: u32 = 0x0072_c6a0;
    pub const VU0_MADDA_Y: u32 = 0x0072_c590;
    pub const VU0_MADDA_Z: u32 = 0x0072_c5f0;
    pub const VU0_MADD_W: u32 = 0x0072_c650;

    /// `rcbgClipVertex(m, verts, n)` (`0x0092af60`): the matrix `m` into
    /// the emulated VU0 registers `0x00c8f7d0..0x00c8f80f`, then each of
    /// the `n` vertices (16 bytes each; at least one) into `0x00c8f810`
    /// and through it as macro code (`pppVu0MulX`, `pppVu0MaddaY`/`Z` on
    /// the accumulator `0x00c8f790`, `pppVu0MaddW` into `0x00c8f820`).
    /// The clip code starts at 0x3f and keeps only the bits of the VU0 clip flags
    /// `0x00c8facc` (copied to the EE register `0x00c8f538`) per vertex;
    /// the PS2's `vclip` that set those flags is gone, so they are
    /// whatever the last clip test left. Returns the code, 0 as soon as it
    /// is 0.
    pub fn rcbg_clip_vertex<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        verts: u32,
        n: i32,
        call: &mut C,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let rows = r(mem, 0x00c8_f7d0);
        for k in 0..4 {
            let row = mem.read::<[u32; 4]>(m.wrapping_add(0x10 * k));
            mem.write(rows + 0x10 * k, row);
        }
        let (vec, acc, out) = (
            r(mem, 0x00c8_f810),
            r(mem, 0x00c8_f790),
            r(mem, 0x00c8_f820),
        );
        let (flags, ee) = (r(mem, 0x00c8_facc), r(mem, 0x00c8_f538));
        let mut code = 0x3fu32;
        let mut left = n;
        let mut at = verts;
        loop {
            let v = mem.read::<[u32; 4]>(at);
            mem.write(vec, v);
            at = at.wrapping_add(0x10);
            for (f, dst, row) in [
                (VU0_MUL_X, acc, 0),
                (VU0_MADDA_Y, acc, 1),
                (VU0_MADDA_Z, acc, 2),
                (VU0_MADD_W, out, 3),
            ] {
                let [x, y, z, w] = mem.read::<[u32; 4]>(vec);
                let f = r(mem, f);
                call(mem, f, &[dst, rows + 0x10 * row, x, y, z, w]);
            }
            let clip = mem.read::<u32>(flags);
            mem.write(ee, clip);
            code &= clip;
            left -= 1;
            if code == 0 || left <= 0 {
                return code;
            }
        }
    }

    /// `rcfxParticleResetForce(p)` (`0x00928970`): each of its `+0x34`
    /// forces (array at `+0x38`) with a reset function (`+0x10`) gets
    /// `call(mem, function, force, p)`, then [`rcfx_particle_clear_forces`].
    pub fn rcfx_particle_reset_force<M: Mem>(
        mem: &mut M,
        p: u32,
        mut call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let mut i = 0i32;
        while i < mem.read::<i32>(p.wrapping_add(0x34)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let at = mem
                .read::<u32>(p.wrapping_add(0x38))
                .wrapping_add(i as u32 * 4);
            let force = mem.read::<u32>(at);
            let f = mem.read::<u32>(force.wrapping_add(0x10));
            if f != 0 {
                call(mem, f, force, p);
            }
            i += 1;
        }
        rcfx_particle_clear_forces(mem, p);
    }

    /// `rcbgSceneDraw(scene)` (`0x00920740`): each of its `+0xc` objects
    /// (array at `+0x10`) with a draw function (`+0xc`) gets
    /// `call(mem, function, object)`, then the layers are queued
    /// ([`rcbg_scene_draw_layer`]); returns that result.
    pub fn rcbg_scene_draw<M: Mem>(
        mem: &mut M,
        s: u32,
        mut call: impl FnMut(&mut M, u32, u32),
        after_set: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let mut i = 0i32;
        while i < mem.read::<i32>(s.wrapping_add(0xc)) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let at = mem
                .read::<u32>(s.wrapping_add(0x10))
                .wrapping_add(i as u32 * 4);
            let o = mem.read::<u32>(at);
            let f = mem.read::<u32>(o.wrapping_add(0xc));
            if f != 0 {
                call(mem, f, o);
            }
            i += 1;
        }
        rcbg_scene_draw_layer(mem, s, after_set)
    }

    /// `Yn_PSimGetHandle(n)` (`0x009199f0`): the word at
    /// [`rcbg_fp_get_work`]`(n + 0x5b, 6, 2)` (FP work of map part 6).
    pub fn yn_psim_get_handle<M: Mem>(
        mem: &mut M,
        n: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let w = rcbg_fp_get_work(mem, n.wrapping_add(0x5b), 6, 2, run_of);
        ptr32(mem.read::<u32>(w))
    }

    /// `rcbgPSimSetReady(psim)` (`0x0091ffb0`): its particle system's
    /// forces reset ([`rcfx_particle_reset_force`] on `+0x18`), then flag
    /// bit 0 (`+0x10`).
    pub fn rcbg_psim_set_ready<M: Mem>(
        mem: &mut M,
        p: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let sys = mem.read::<u32>(p.wrapping_add(0x18));
        rcfx_particle_reset_force(mem, sys, call);
        let f = mem.read::<u32>(p.wrapping_add(0x10));
        mem.write(p.wrapping_add(0x10), f | 1);
    }

    /// `rcbgPSimSetPos(psim, which, v)` (`0x0091ff10`): position `which`
    /// (0: `+0x4c`, else `+0x8c`) from `v`'s x, y, z; the first time each
    /// is set (`+0x9c`, `+0xa0`) the initial count (`+0xa4`) drops, and
    /// when it reaches 0 the simulation is made ready
    /// ([`rcbg_psim_set_ready`]).
    pub fn rcbg_psim_set_pos<M: Mem>(
        mem: &mut M,
        p: u32,
        which: u32,
        v: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        let (pos, set) = if which == 0 {
            (psim::POS, 0x9c)
        } else {
            (psim::POS2, 0xa0)
        };
        let xyz = mem.read::<[u32; 3]>(v).map(crate::x87::copy);
        mem.write(p.wrapping_add(pos), xyz);
        if mem.read::<u32>(p.wrapping_add(set)) != 0 {
            return;
        }
        let mut ready = false;
        let n = mem.read::<u32>(p.wrapping_add(psim::INIT_CNT));
        if n != 0 {
            mem.write(p.wrapping_add(psim::INIT_CNT), n.wrapping_sub(1));
            ready = n == 1;
        }
        mem.write(p.wrapping_add(set), 1u32);
        if ready {
            rcbg_psim_set_ready(mem, p, call);
        }
    }

    /// `Yn_SeaGetWork()` (`0x00919a10`): [`rcbg_fp_get_work`]`(0x5a, 6,
    /// 2)`, the sea's FP work (`Yn_SeaSetAng`, `Yn_SeaSetPos`,
    /// `Yn_SeaSetUvSpd`).
    pub fn yn_sea_get_work<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        rcbg_fp_get_work(mem, 0x5a, 6, 2, run_of)
    }

    // ---- round 5: palettes, layers, scene objects, FP parts, characters ----

    /// `rcbgPalGetEntryData(pal, n)` (`0x00929990`): entry `n`'s data of
    /// the palette's transfer (`+0xc`, [`rcbg_trx_get_entry_data`]).
    pub fn rcbg_pal_get_entry_data(mem: &impl Mem, pal: u32, n: u32) -> u32 {
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        rcbg_trx_get_entry_data(mem, t, n)
    }

    /// `rcbgPalGetLastEntryData(pal)` (`0x00929930`): the last entry's
    /// (the transfer's count at `+0x30`, less 1).
    pub fn rcbg_pal_get_last_entry_data(mem: &impl Mem, pal: u32) -> u32 {
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        let n = mem.read::<u32>(t.wrapping_add(0x30)).wrapping_sub(1);
        rcbg_trx_get_entry_data(mem, t, n)
    }

    /// `rcbgLayerGetTexData(layer)` (`0x0092acf0`): entry 0's data of the
    /// layer's texture transfer (`+0xbc`), 0 without one.
    pub fn rcbg_layer_get_tex_data(mem: &impl Mem, layer: u32) -> u32 {
        match mem.read::<u32>(layer.wrapping_add(0xbc)) {
            0 => 0,
            t => rcbg_trx_get_entry_data(mem, t, 0),
        }
    }

    /// `rcbgTexGetLog2Size(w, h, &lw, &lh, &tbw)` (`0x00920050`): the
    /// GS log2 sizes (`rcLog2Ceil`) and the buffer width in 64-pixel units,
    /// rounded up to even; each output optional.
    pub fn rcbg_tex_get_log2_size(mem: &mut impl Mem, w: i32, h: i32, lw: u32, lh: u32, tbw: u32) {
        if lw != 0 {
            mem.write(lw, super::log2_ceil(w));
        }
        if lh != 0 {
            mem.write(lh, super::log2_ceil(h));
        }
        if tbw != 0 {
            let n = w.wrapping_add(0x3f);
            let mut v = n.wrapping_add((n >> 31) & 0x3f) >> 6;
            mem.write(tbw, v);
            if v & 1 != 0 {
                v = v.wrapping_add(1);
                mem.write(tbw, v);
            }
        }
    }

    /// The scene, when Yn's field state, its map and the scene exist.
    fn yn_scene(mem: &impl Mem) -> Option<u32> {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            return None;
        }
        Some(mem.read::<u32>(m.wrapping_add(8))).filter(|&c| c != 0)
    }

    /// `Yn_DrawScene()` (`0x0091a240`, through `0x00921a10` and
    /// `0x009215e0`; name ours): `rcbgSceneDraw` of the map's scene (`+8`,
    /// which may be 0) when Yn's field state and its map exist. Returns its
    /// `eax`, or `None` where the original leaves `eax` as it was (no
    /// state) or holding the state (no map).
    pub fn yn_draw_scene<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> Option<u32> {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            return None;
        }
        let scene = mem.read::<u32>(m.wrapping_add(8));
        let f = mem.rebase(super::SCENE_DRAW);
        Some(call(mem, f, &[scene]))
    }

    /// `rcbgSceneSetChadist(scene, work)` (`0x00920790`, also through
    /// the jump at `0x00920780`; name ours): `+0x104` = 1; the character
    /// distance `+0x100` is the player's position (`Ch_GetPos` into
    /// `work`, w 1.0) through the camera matrix (`FFXVu0ApplyMatrix`), its
    /// z times 0.1 (an `f64` of an `f32`), or 98.0 without a player. Clears
    /// `+0x10` of Yn's map (of address 0 without Yn's state, as the
    /// original), then `graphicSetChadist(distance)`, whose `eax` is
    /// returned.
    pub fn rcbg_scene_set_chadist<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        scene: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87;
        mem.write(scene.wrapping_add(0x104), 1u32);
        let f = mem.rebase(crate::camera::SG_GET_PLAYER);
        let ply = call(mem, f, &[]);
        let d = if ply == 0 {
            let c: f32 = mem.read(mem.rebase(super::CHADIST_DEFAULT));
            x87::round_trip(c).to_bits()
        } else {
            let f = mem.rebase(super::CH_GET_POS);
            call(
                mem,
                f,
                &[ply, work, work.wrapping_add(4), work.wrapping_add(8)],
            );
            mem.write(work.wrapping_add(0xc), 1.0f32.to_bits());
            let f = mem.rebase(crate::yi::VU0_APPLY_MATRIX);
            let m = mem.rebase(crate::camera::CAMERA_MATRIX);
            call(mem, f, &[work, m, work]);
            let z = x87::load(mem.read(work.wrapping_add(8)));
            let k: f64 = mem.read(mem.rebase(super::CHADIST_SCALE));
            x87::store(x87::mul(z, k))
        };
        mem.write(scene.wrapping_add(0x100), d);
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        let m = if s == 0 { 0 } else { mem.read::<u32>(s) };
        mem.write(m.wrapping_add(0x10), 0u32);
        let d: f32 = mem.read(scene.wrapping_add(0x100));
        let f = mem.rebase(super::GRAPHIC_SET_CHADIST);
        call(mem, f, &[x87::round_trip(d).to_bits()])
    }

    /// `Yn_SetChadist()` (`0x0091a250`, through `0x00921a30` and
    /// `0x00921610`; name ours): `rcbgSceneSetChadist` (by its thunk) of
    /// the map's scene, as [`yn_draw_scene`].
    pub fn yn_set_chadist<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> Option<u32> {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return None;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            return None;
        }
        let scene = mem.read::<u32>(m.wrapping_add(8));
        let f = mem.rebase(super::SCENE_SET_CHADIST_THUNK);
        Some(call(mem, f, &[scene]))
    }

    /// The scene of Yn's map, or 0 (the parts' owner of the layer
    /// switches; the original then reads address 8).
    fn yn_scene_or_0(mem: &impl Mem) -> u32 {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let m = mem.read::<u32>(s);
        if m == 0 {
            return 0;
        }
        mem.read::<u32>(m.wrapping_add(8))
    }

    /// The pre-BG layer switch shared by `Yn_PreBgLayerDispOff`/`On`: below
    /// the part count, the part's (`0xc0` bytes from the scene's `+8`)
    /// bit 0 set (`hide`) or cleared; below the count (0x1c on maps 0x15b
    /// and 0xd4), the renderer's layer hidden or shown and its texture
    /// animation material made invisible or visible. Returns the last
    /// call's `eax`, or the map number the original loads last.
    fn pre_bg_layer<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: u32,
        count: u32,
        hide: bool,
        call: &mut C,
    ) -> u32 {
        let ni = n.cast_signed();
        let mut limit = count.cast_signed();
        if ni < limit {
            let scene = yn_scene_or_0(mem);
            let parts = mem.read::<u32>(scene.wrapping_add(8));
            let at = parts.wrapping_add(n.wrapping_mul(0xc0));
            let v = mem.read::<u32>(at);
            mem.write(at, if hide { v | 1 } else { v & !1 });
        }
        let map = mem.read::<u32>(mem.rebase(super::CURRENT_MAP_NO));
        if map == 0x15b || map == 0xd4 {
            limit = 0x1c;
        }
        if ni >= limit {
            return map;
        }
        let f = mem.rebase(if hide {
            super::GRAPHIC_2D_HIDE_LAYER
        } else {
            super::GRAPHIC_2D_SHOW_LAYER
        });
        call(mem, f, &[n]);
        let f = mem.rebase(super::TEX_ANIM_SET_VISIBILITY);
        call(mem, f, &[n, u32::from(!hide)])
    }

    /// `Yn_PreBgLayerDispOff(n)` (`0x0091a7c0`, name ours): hides pre-BG
    /// layer `n` (`Yn_MapPartCount` parts); see `Yn_PreBgLayerDispOn`.
    pub fn yn_pre_bg_layer_disp_off<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::YN_MAP_PART_COUNT);
        let count = call(mem, f, &[]);
        pre_bg_layer(mem, n, count, true, call)
    }

    /// `Yn_PreBgLayerDispOn(n)` (`0x0091a830`): shows pre-BG layer `n`;
    /// first sets `0x025d5ebc` when the path at `0x023cbc60` contains
    /// `"te/test20"` (`strstr`, through the import table).
    pub fn yn_pre_bg_layer_disp_on<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        n: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::YN_MAP_PART_COUNT);
        let count = call(mem, f, &[]);
        let f = mem.read::<u32>(mem.rebase(super::STRSTR_IMPORT));
        let hay = mem.rebase(super::PRE_BG_PATH);
        let needle = mem.rebase(super::TEST20);
        if call(mem, f, &[hay, needle]) != 0 {
            let at = mem.rebase(super::TEST20_SEEN);
            mem.write(at, 1u32);
        }
        pre_bg_layer(mem, n, count, false, call)
    }

    /// `rcfxSpringDelete(spring)` (`0x00928c60`, name ours):
    /// `rcfxSpringFini`, then `rcFree`, whose `eax` it returns.
    pub fn rcfx_spring_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::RCFX_SPRING_FINI);
        call(mem, f, &[p]);
        let f = mem.rebase(super::RC_FREE);
        call(mem, f, &[p])
    }

    /// `rcfxNailDelete(nail)` (`0x00929070`, name ours): `rcfxNailFini`,
    /// then `rcFree`, whose `eax` it returns.
    pub fn rcfx_nail_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::RCFX_NAIL_FINI);
        call(mem, f, &[p]);
        let f = mem.rebase(super::RC_FREE);
        call(mem, f, &[p])
    }

    /// `Yn_FPm173End()` (`0x00919ab0`, name ours): `rcbgFPm173End` of the
    /// scene's first object of kind 0x20008 (`Yn_SceneGetObject`), its
    /// `eax`; 0 without one.
    pub fn yn_fpm173_end<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::YN_SCENE_GET_OBJECT);
        let o = call(mem, f, &[0x20008, 0]);
        if o == 0 {
            return 0;
        }
        let f = mem.rebase(super::FPM173_END);
        call(mem, f, &[o])
    }

    /// `rcbgTrxPairSetNo(p, n)` (`0x009221e0`, name ours): `rcbgTrxSetNo`
    /// of the transfers at `+0x20` and `+0x60`; the second's `eax`.
    pub fn rcbg_trx_pair_set_no<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        n: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::TRX_SET_NO);
        call(mem, f, &[p.wrapping_add(0x20), n]);
        call(mem, f, &[p.wrapping_add(0x60), n])
    }

    /// The swing of the screen shakes and waves: the sine table entry
    /// `(0x4000 - trunc(phase * -65536.0 / 6.2831855)) >> 4 & 0xfff`.
    pub fn shake_swing(mem: &impl Mem, phase: u32) -> f64 {
        use ffx_bridge::x87::{div, ftol, load, mul};
        let c = |at: u32| mem.read::<f64>(mem.rebase(at));
        let x = div(
            mul(load(phase), c(super::SWING_SCALE)),
            c(super::SWING_TURN),
        );
        let i = 0x4000i32.wrapping_sub(ftol(mem, x).cast_signed()) >> 4 & 0xfff;
        let at = mem.rebase(super::SIN_TABLE) + 4 * i.cast_unsigned();
        load(mem.read::<u32>(at))
    }

    /// The fade of a shake or wave: `f32(left) / f32(frames) * swing`,
    /// `left = frames - done`, each int stored as `f32`.
    #[must_use]
    pub fn shake_fade(frames: i32, done: i32, swing: f64) -> f64 {
        use ffx_bridge::x87::{div, load, mul, store};
        #[allow(clippy::cast_precision_loss)] // fild, then an f32 store
        let st = |v: i32| load(store(f64::from(v as f32)));
        let r = st(frames.wrapping_sub(done));
        let q = st(frames);
        load(store(mul(div(r, q), swing)))
    }

    /// `rand() % range + base` through the import table (a zero range
    /// faults in the original; the port gives `base`).
    pub fn shake_random<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        range: i32,
        base: i32,
        call: &mut C,
    ) -> i32 {
        let f = mem.read::<u32>(mem.rebase(super::RAND_IMPORT));
        let r = call(mem, f, &[]).cast_signed();
        r.checked_rem(range).unwrap_or(0).wrapping_add(base)
    }

    /// `rcbgSceneWaveC0Update(scene)` (`0x009210e0`, from `0x00920aa0`;
    /// name ours): while a wave runs (mode `+0xc0`), the swing of the
    /// phase `+0xd4` ([`shake_swing`]), faded over the frames `+0xc8`
    /// (done `+0xd8`) for modes 3 and 4, times the amplitude (`+0xcc`,
    /// plus `rand() % +0xd0` when that is positive) into `+0xb8` (odd
    /// modes) or `+0xbc`; the phase advances by `+0xc4`, the count by one,
    /// the wave stops (`rcbgSceneWaveC0Stop`) after its frames, then
    /// `rcbgSceneSetMatrix`.
    pub fn rcbg_scene_wave_c0_update<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        call: &mut C,
    ) {
        use ffx_bridge::x87::{add, load, mul, store};
        let mode = mem.read::<i32>(s.wrapping_add(0xc0));
        if mode == 0 {
            return;
        }
        let mut swing = shake_swing(mem, mem.read(s.wrapping_add(0xd4)));
        let frames = mem.read::<i32>(s.wrapping_add(0xc8));
        if mode.wrapping_sub(1) / 2 == 1 && frames > 0 {
            let done = mem.read::<i32>(s.wrapping_add(0xd8));
            swing = shake_fade(frames, done, swing);
        }
        let range = mem.read::<i32>(s.wrapping_add(0xd0));
        let base = mem.read::<i32>(s.wrapping_add(0xcc));
        let amp = if range > 0 {
            shake_random(mem, range, base, call)
        } else {
            base
        };
        #[allow(clippy::cast_precision_loss)] // fild, then an f32 store
        let amp = load(store(f64::from(amp as f32)));
        let w = store(mul(amp, swing));
        let at = if mode & 1 != 0 { 0xb8 } else { 0xbc };
        mem.write(s.wrapping_add(at), w);
        let p = store(add(
            load(mem.read(s.wrapping_add(0xc4))),
            load(mem.read(s.wrapping_add(0xd4))),
        ));
        let done = mem.read::<i32>(s.wrapping_add(0xd8)).wrapping_add(1);
        mem.write(s.wrapping_add(0xd8), done);
        mem.write(s.wrapping_add(0xd4), p);
        if frames > 0 && done >= frames {
            let f = mem.rebase(super::SCENE_WAVE_C0_STOP);
            call(mem, f, &[s]);
        }
        let f = mem.rebase(crate::scene::SCENE_SET_MATRIX);
        call(mem, f, &[s]);
    }

    /// `rcfxNailUpdate(fx, ctx)` (`0x00929190`, `rcfxNailInit`'s update
    /// callback; name ours): for each of the `+0x1c` points, the source
    /// point (`ctx +0x24` array, index from `+0x20`) to the base array
    /// (`+0x24`), through the matrix `+0x38` (`rcVec3fApplyMatrix`) or
    /// copied to `+0x28`, and that to `+0x2c` (16-byte `rcCopyQword`s);
    /// then the 8 bytes at `ctx` to `+0x30` (an `f64` through the x87
    /// stack). Returns `ctx`.
    pub fn rcfx_nail_update(mem: &mut impl Mem, fx: u32, ctx: u32) -> u32 {
        use crate::geom::bridge::rc_vec3f_apply_matrix;
        use crate::vu0::bridge::rc_copy_qword;
        let src = mem.read::<u32>(ctx.wrapping_add(0x24));
        let mut i = 0i32;
        while i < mem.read::<i32>(fx.wrapping_add(0x1c)) {
            let o = i.cast_unsigned().wrapping_mul(0x10);
            let arr = |mem: &dyn Fn(u32) -> u32, at: u32| mem(at).wrapping_add(o);
            let rd = |at: u32| mem.read::<u32>(fx.wrapping_add(at));
            let idx = mem.read::<u32>(rd(0x20).wrapping_add(i.cast_unsigned().wrapping_mul(4)));
            let base = arr(&rd, 0x24);
            rc_copy_qword(mem, base, src.wrapping_add(idx.wrapping_shl(4)));
            let rd = |at: u32| mem.read::<u32>(fx.wrapping_add(at));
            let m = rd(0x38);
            let (a, b) = (arr(&rd, 0x24), arr(&rd, 0x28));
            if m == 0 {
                rc_copy_qword(mem, b, a);
            } else {
                rc_vec3f_apply_matrix(mem, b, a, m);
            }
            let rd = |at: u32| mem.read::<u32>(fx.wrapping_add(at));
            let (b, c) = (arr(&rd, 0x28), arr(&rd, 0x2c));
            rc_copy_qword(mem, c, b);
            i += 1;
        }
        let mut v = mem.read::<u64>(ctx);
        if v & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000 && v & 0x000f_ffff_ffff_ffff != 0 {
            v |= 0x0008_0000_0000_0000; // a signalling NaN turns quiet
        }
        mem.write(fx.wrapping_add(0x30), v);
        ctx
    }

    /// `Yn_SeaFpFade(fade, work)` (`0x00922970`, from `0x00922600`; name
    /// ours): a field effect faded in and out by the sea's angle
    /// (`Yn_SeaGetAngPtr` `+8`, `lim`) against the fade's angle `a`
    /// (`+8`), its state `+0`, effect id `+4`, frames `+0xc`, count `+0x10`:
    /// - 0 (off): when `lim < a < 0` or `0 < a < lim`, the effect is shown
    ///   and restarted (`pppFpIDdispOn`, `Fp_Restart`), state 1; weight 0.
    /// - 1 (fading in): the count up, weight `count / frames` (both stored
    ///   as `f32` first), state 2 once the count reaches the frames.
    /// - 2 (on): weight 1.0; when `a < 0` and `a < lim`, or `0 < a` and
    ///   `a > lim`, state 3.
    /// - 3 (fading out): the count down, the weight as in 1; at 0 or below
    ///   the effect is hidden and ended (`pppFpIDdispOff`, `Fp_End`), state
    ///   0.
    ///
    /// For a positive id the weight goes to `+0x2c` of its chunk entry
    /// (`0x00c5e480 + 0x38 id`), whose matrix (`rcbgFPmGetMatrix`, into
    /// `work`) is copied to the effect's manager (`pppFpGetMngPtr(entry
    /// id, 0, work + 0x40)`, `+0xc`) when there is one.
    pub fn yn_sea_fp_fade<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        fade: u32,
        work: u32,
        call: &mut C,
    ) {
        use ffx_bridge::x87::{div, load, store};
        let f = mem.rebase(super::YN_SEA_GET_ANG_PTR);
        let ang = call(mem, f, &[]);
        let id16 = u32::from(mem.read::<u16>(fade.wrapping_add(4)));
        let int_f32 = |v: i32| {
            #[allow(clippy::cast_precision_loss)] // fild, then an f32 store
            let x = v as f32;
            load(x.to_bits())
        };
        let ratio = |mem: &M| {
            let c = mem.read::<i32>(fade.wrapping_add(0x10));
            let n = mem.read::<i32>(fade.wrapping_add(0xc));
            store(div(int_f32(c), int_f32(n)))
        };
        let angle = |mem: &M| {
            (
                f64::from(mem.read::<f32>(fade.wrapping_add(8))),
                f64::from(mem.read::<f32>(ang.wrapping_add(8))),
            )
        };
        let mut w = 0u32;
        match mem.read::<u32>(fade) {
            0 => {
                let (a, lim) = angle(mem);
                if (a < 0.0 && a > lim) || (0.0 < a && a < lim) {
                    let f = mem.rebase(super::PPP_FP_ID_DISP_ON);
                    call(mem, f, &[id16]);
                    let f = mem.rebase(super::FP_RESTART);
                    call(mem, f, &[id16]);
                    mem.write(fade, 1u32);
                }
            }
            1 => {
                let c = mem.read::<i32>(fade.wrapping_add(0x10)).wrapping_add(1);
                mem.write(fade.wrapping_add(0x10), c);
                w = ratio(mem);
                if c >= mem.read::<i32>(fade.wrapping_add(0xc)) {
                    mem.write(fade, 2u32);
                }
            }
            2 => {
                w = 1.0f32.to_bits();
                let (a, lim) = angle(mem);
                if (a < 0.0 && a < lim) || (0.0 < a && a > lim) {
                    mem.write(fade, 3u32);
                }
            }
            3 => {
                let c = mem.read::<i32>(fade.wrapping_add(0x10)).wrapping_sub(1);
                mem.write(fade.wrapping_add(0x10), c);
                w = ratio(mem);
                if c <= 0 {
                    let f = mem.rebase(super::PPP_FP_ID_DISP_OFF);
                    call(mem, f, &[id16]);
                    let f = mem.rebase(super::FP_END);
                    call(mem, f, &[id16]);
                    mem.write(fade, 0u32);
                }
            }
            _ => {}
        }
        let id = mem.read::<i32>(fade.wrapping_add(4));
        if id <= 0 {
            return;
        }
        let entry = mem
            .rebase(super::CHUNK_IDS)
            .wrapping_add(id.cast_unsigned().wrapping_mul(0x38));
        mem.write(entry.wrapping_add(0x2c), w);
        let f = mem.rebase(super::FPM_GET_MATRIX);
        call(mem, f, &[entry, work]);
        let f = mem.rebase(super::PPP_FP_GET_MNG_PTR);
        let n = i32::from(mem.read::<i16>(entry)).cast_unsigned();
        call(mem, f, &[n, 0, work.wrapping_add(0x40)]);
        let mng = mem.read::<u32>(work.wrapping_add(0x40));
        if mng != 0 {
            let f = mem.rebase(crate::ke::VU0_COPY_MATRIX);
            call(mem, f, &[mng.wrapping_add(0xc), work]);
        }
    }

    /// `Yn_SceneGetObject(kind, n)` (`0x00921be0`): the scene's
    /// `rcbgSceneGetObject`, 0 without a scene.
    pub fn yn_scene_get_object(mem: &impl Mem, kind: u32, n: u32) -> u32 {
        yn_scene(mem).map_or(0, |s| {
            crate::scene::bridge::rcbg_scene_get_object(mem, s, kind, n)
        })
    }

    /// `Yn_SceneCountObjects(kind)` (`0x00921c20`): the scene's
    /// `rcbgSceneCountObjects`, 0 without a scene.
    pub fn yn_scene_count_objects(mem: &impl Mem, kind: u32) -> u32 {
        yn_scene(mem).map_or(0, |s| {
            crate::scene::bridge::rcbg_scene_count_objects(mem, s, kind)
        })
    }

    /// `Yn_BattleCaptureNum()` (`0x00921910`): while Yn's battle number
    /// (`+0xc`) is 0, takes `MsBattleGetNum` when non-zero and clears
    /// `+8`.
    pub fn yn_battle_capture_num(mem: &mut impl Mem) {
        let s = crate::guide::YN_MAP_SYSTEM.get(mem);
        if mem.read::<u32>(s.wrapping_add(0xc)) != 0 {
            return;
        }
        let n = ffx_battle::state::bridge::ms_battle_get_num(mem);
        if n != 0 {
            let s = crate::guide::YN_MAP_SYSTEM.get(mem);
            mem.write(s.wrapping_add(0xc), n);
            let s = crate::guide::YN_MAP_SYSTEM.get(mem);
            mem.write(s.wrapping_add(8), 0u32);
        }
    }

    /// `rcbgFpFadeInit(fade, id, step, frames)` (`0x00922930`, from
    /// `rcbgFPm097`'s process): hides FP `id` (`pppFpIDdispOff`,
    /// `hide`) and sets up its fade: 0, the id, the step (a float), the
    /// frames, 0.
    pub fn rcbg_fp_fade_init<M: Mem>(
        mem: &mut M,
        fade: u32,
        id: u32,
        step: u32,
        frames: u32,
        hide: impl FnOnce(&mut M, u32),
    ) {
        hide(mem, id);
        mem.write(fade.wrapping_add(8), crate::x87::copy(step));
        mem.write(fade.wrapping_add(4), id);
        mem.write(fade, 0u32);
        mem.write(fade.wrapping_add(0xc), frames);
        mem.write(fade.wrapping_add(0x10), 0u32);
    }

    /// The original's frame for [`rcbg_fpm097_proc001`], at `work` (as
    /// `ebp - 0x1ac`; [`FPM097_FRAME`] bytes).
    mod frame097 {
        /// The FP's matrix record (`pppFpGetMngPtr`'s output).
        pub const MNG: u32 = 0x1ac;
        /// An FP's own matrix (`rcbgFPmGetMatrix`).
        pub const FP_MATRIX: u32 = 0x1a4;
        /// The scale and rotation matrix.
        pub const SCALE_MATRIX: u32 = 0x164;
        /// The lift (a unit matrix moved up by the fade, `+0x14`).
        pub const LIFT_MATRIX: u32 = 0x124;
        /// The offset (a unit matrix moved by the entry's `+8`).
        pub const PLACE_MATRIX: u32 = 0xe4;
        /// The sea's matrix.
        pub const SEA_MATRIX: u32 = 0xa4;
        /// The character element's matrix, then the result.
        pub const ELEM_MATRIX: u32 = 0x64;
        /// The lift vector (its `y` the fade; the rest never set).
        pub const LIFT: u32 = 0x34;
        /// The zero translation (its `w` never set).
        pub const ZERO: u32 = 0x24;
        /// The rotation vector.
        pub const ROTATION: u32 = 0x14;
    }

    /// The bytes of [`rcbg_fpm097_proc001`]'s frame.
    pub const FPM097_FRAME: u32 = 0x1a8;

    /// `rcbgFPm097Proc001(part)` (`0x00922600`, map 0x61's FP step that
    /// `rcbgFPm097Proc000` installs; name ours), over the 9-entry list at
    /// [`super::FPM097_START`] (layout [`super::fpm097::ENTRY_SIZE`]),
    /// with its locals in `work` ([`FPM097_FRAME`] bytes, zeroed: the
    /// original reads some it never sets, see the frame):
    /// - Until `+0x10` is set: each kind 4 entry whose character is found
    ///   (`rcbgChrFindById`) is restarted (`Fp_Restart`); the first one
    ///   gets its element's matrix (unused), `+0x10` 1 and `+0x14` 0.0;
    ///   if one was, the two fades start (`rcbgFpFadeInit(part +0x18, 1,
    ///   -0.15, 30)`, `(part +0x2c, 4, 0.15, 30)`).
    /// - With the sea's angle and position (`Yn_SeaGetAngPtr`,
    ///   `Yn_SeaGetPosPtr`): the sea's matrix is a unit matrix through
    ///   `op_rt_m_yxz` with its angle scaled by 651.8986 (x and y 0.0)
    ///   and its position. Each kind 3 entry with an FP (`pppFpGetMngPtr`)
    ///   gets its own matrix (`rcbgFPmGetMatrix`) times the sea's into the
    ///   FP's `+0xc`; each kind 4 entry with an FP and a character gets
    ///   the element matrix times the entry's scale and y rotation (its
    ///   `+0x18` scaled by 11.377778, y less `Ch_GetRot * 651.8986 +
    ///   1024.0`), its offset (`+8`), the lift by `+0x14` and the sea's,
    ///   copied to the FP's `+0xc`. Then both fades run (`Yn_SeaFpFade`).
    ///
    /// Returns the `eax` it ends with: the last fade's, or the sea's
    /// position when either pointer is null.
    pub fn rcbg_fpm097_proc001<M, C, F>(
        mem: &mut M,
        part: u32,
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        use super::fpm097 as k;
        use frame097 as fr;
        let at = |local: u32| work.wrapping_add(FPM097_FRAME + 4 - local);
        let entry = |mem: &M, i: u32| mem.rebase(super::FPM097_START[0]) + i * k::ENTRY_SIZE;
        let id = |mem: &M, e: u32, o: u32| i32::from(mem.read::<i16>(e + o)).cast_unsigned();
        let mut run = |mem: &mut M, f: u32, args: &[u32]| {
            let f = mem.rebase(f);
            call(mem, f, args)
        };
        if mem.read::<u32>(part.wrapping_add(0x10)) == 0 {
            for i in 0..9 {
                let e = entry(mem, i);
                if mem.read::<u8>(e + 6) != 4 {
                    continue;
                }
                let chr = run(mem, k::CHR_FIND_BY_ID, &[id(mem, e, 2)]);
                if chr == 0 {
                    continue;
                }
                let fp = u32::from(mem.read::<u16>(e));
                run(mem, super::FP_RESTART, &[fp]);
                if mem.read::<u32>(part.wrapping_add(0x10)) == 0 {
                    let elem = id(mem, e, 4);
                    run(mem, k::GET_ELEM_MATRIX, &[at(fr::LIFT_MATRIX), chr, elem]);
                    mem.write(part.wrapping_add(0x10), 1u32);
                    mem.write(part.wrapping_add(0x14), 0u32);
                }
            }
            if mem.read::<u32>(part.wrapping_add(0x10)) != 0 {
                for (fade, (fp, step)) in
                    [(0x18, (1, k::FADE_STEPS[0])), (0x2c, (4, k::FADE_STEPS[1]))]
                {
                    let step = crate::x87::copy(mem.read::<u32>(mem.rebase(step)));
                    run(
                        mem,
                        k::FADE_INIT,
                        &[part.wrapping_add(fade), fp, step, 0x1e],
                    );
                }
            }
        }
        let angle = run(mem, super::YN_SEA_GET_ANG_PTR, &[]);
        let pos = run(mem, k::SEA_GET_POS_PTR, &[]);
        if angle == 0 || pos == 0 {
            return pos;
        }
        let scale = crate::x87::copy(mem.read::<u32>(mem.rebase(k::SEA_ANGLE_SCALE)));
        run(mem, k::SCALE_VECTOR, &[at(fr::ROTATION), angle, scale]);
        mem.write(at(fr::ROTATION - 4), 0u32);
        mem.write(at(fr::ROTATION), 0u32);
        let sea = at(fr::SEA_MATRIX);
        run(mem, k::UNIT_MATRIX, &[sea]);
        run(mem, super::OP_RT_M_YXZ, &[sea, sea, at(fr::ROTATION), pos]);
        mem.write(at(fr::ZERO), [0u32; 3]);
        for i in 0..9 {
            let e = entry(mem, i);
            let mng = at(fr::MNG);
            match mem.read::<u8>(e + 6) {
                3 => {
                    run(mem, super::PPP_FP_GET_MNG_PTR, &[id(mem, e, 0), 0, mng]);
                    let m = mem.read::<u32>(mng);
                    if m != 0 {
                        run(mem, super::RCBG_FPM_GET_MATRIX, &[e, at(fr::FP_MATRIX)]);
                        let m = mem.read::<u32>(mng).wrapping_add(0xc);
                        run(mem, k::MUL_MATRIX, &[m, sea, at(fr::FP_MATRIX)]);
                    }
                }
                4 => {
                    run(mem, super::PPP_FP_GET_MNG_PTR, &[id(mem, e, 0), 0, mng]);
                    if mem.read::<u32>(mng) == 0 {
                        continue;
                    }
                    let chr = run(mem, k::CHR_FIND_BY_ID, &[id(mem, e, 2)]);
                    if chr == 0 {
                        continue;
                    }
                    place_on_chr(mem, part, e, chr, work, &mut run, fcall);
                }
                _ => {}
            }
        }
        run(mem, k::SEA_FP_FADE, &[part.wrapping_add(0x18)]);
        run(mem, k::SEA_FP_FADE, &[part.wrapping_add(0x2c)])
    }

    /// A kind 4 entry `e` of [`rcbg_fpm097_proc001`] on its character.
    fn place_on_chr<M, R, F>(
        mem: &mut M,
        part: u32,
        e: u32,
        chr: u32,
        work: u32,
        run: &mut R,
        fcall: &mut F,
    ) where
        M: Mem,
        R: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        use super::fpm097 as k;
        use crate::x87;
        use frame097 as fr;
        let at = |local: u32| work.wrapping_add(FPM097_FRAME + 4 - local);
        let place = at(fr::PLACE_MATRIX);
        run(mem, k::UNIT_MATRIX, &[place]);
        run(mem, k::COPY_QWORD, &[place + 0x30, e + 8]);
        mem.write(place + 0x3c, 1.0f32.to_bits());
        let scale = x87::copy(mem.read::<u32>(mem.rebase(k::FP_ANGLE_SCALE)));
        let rotation = at(fr::ROTATION);
        run(mem, k::SCALE_VECTOR, &[rotation, e + 0x18, scale]);
        let y = x87::load(mem.read::<u32>(rotation + 4));
        let f = mem.rebase(k::CH_GET_ROT);
        let rot = fcall(mem, f, crate::camera::FArgs::Stack(&[chr]));
        let [kr, quarter] = k::ROT_TO_ANGLE.map(|c| mem.read::<f64>(mem.rebase(c)));
        mem.write(rotation + 4, x87::store(y - (rot * kr + quarter)));
        mem.write(rotation + 8, 0u32);
        mem.write(rotation, 0u32);
        let scaled = at(fr::SCALE_MATRIX);
        run(mem, k::SET_SCALE_VEC3F, &[scaled, e + 0x28]);
        run(
            mem,
            super::OP_RT_M_YXZ,
            &[scaled, scaled, rotation, at(fr::ZERO)],
        );
        let elem = at(fr::ELEM_MATRIX);
        let el = i32::from(mem.read::<i16>(e + 4)).cast_unsigned();
        run(mem, k::GET_ELEM_MATRIX, &[elem, chr, el]);
        let lift = x87::copy(mem.read::<u32>(part.wrapping_add(0x14)));
        mem.write(at(fr::LIFT - 4), lift);
        let lifted = at(fr::LIFT_MATRIX);
        run(mem, k::UNIT_MATRIX, &[lifted]);
        run(mem, k::COPY_QWORD, &[lifted + 0x30, at(fr::LIFT)]);
        run(mem, k::MUL_MATRIX, &[elem, scaled, place]);
        run(mem, k::MUL_MATRIX, &[elem, lifted, elem]);
        run(mem, k::MUL_MATRIX, &[elem, at(fr::SEA_MATRIX), elem]);
        let m = mem.read::<u32>(at(fr::MNG)).wrapping_add(0xc);
        run(mem, k::COPY_MATRIX, &[m, elem]);
    }

    /// The bytes of [`rcbg_fpm565_proc001`]'s frame (as `ebp - 0x4c`: the
    /// FP's run and matrix records, then a matrix).
    pub const FPM565_FRAME: u32 = 0x48;

    /// `rcbgFPm565Proc001()` (`0x00922fe0`, map 0x235's FP step that
    /// `rcbgFPm565Proc000` installs; name ours): with the sea's matrix
    /// (`Yn_SeaGetMatrix`), FP 0x5a's work (`rcbgFpGetWork(0x5a, 6, 2)`)
    /// and part data (`rcbgFPmGetPartData(0x5a, 1, 6, 2)`): the work gets
    /// the sea's matrix and `+0xe5` 1, the part data `+4` and `+0x10`
    /// `32768 - ` the work's `+0xb8` and `+0xc4`; each kind 5 entry of the
    /// 3-entry list ([`super::FPM565_START`]) with an FP (`pppFpGetMngPtr`)
    /// gets `pppSetMatrix(run)` and the sea's matrix times
    /// `g_pppWorldMatrix` in its record's `+0xc`; display object 10 gets
    /// the sea's matrix without its translation (`yiSetMatrixDObj`).
    /// `work` is the original's frame ([`FPM565_FRAME`] bytes). Returns the
    /// `eax` it ends with.
    pub fn rcbg_fpm565_proc001<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use super::fpm565 as k;
        use crate::x87::{load, store};
        let mut run = |mem: &mut M, f: u32, args: &[u32]| {
            let f = mem.rebase(f);
            call(mem, f, args)
        };
        let (records, matrix) = (work, work.wrapping_add(8));
        mem.write(records, [0u32; 2]);
        let sea = run(mem, k::SEA_GET_MATRIX, &[]);
        if sea == 0 {
            return 0;
        }
        let fp = run(mem, k::FP_GET_WORK, &[0x5a, 6, 2]);
        let part = run(mem, k::GET_PART_DATA, &[0x5a, 1, 6, 2]);
        if fp == 0 || part == 0 {
            return part;
        }
        run(mem, super::fpm097::COPY_MATRIX, &[fp, sea]);
        mem.write(fp.wrapping_add(0xe5), 1u8);
        run(mem, super::fpm097::COPY_MATRIX, &[matrix, sea]);
        mem.write(matrix.wrapping_add(0x30), [0u32; 3]);
        let half = mem.read::<f64>(mem.rebase(k::HALF_TURN));
        let a = store(half - load(mem.read::<u32>(fp.wrapping_add(0xb8))));
        mem.write(part.wrapping_add(4), a);
        let c = store(half - load(mem.read::<u32>(fp.wrapping_add(0xc4))));
        mem.write(part.wrapping_add(0x10), c);
        for i in 0..3 {
            let e = mem.rebase(super::FPM565_START[0]) + i * super::fpm097::ENTRY_SIZE;
            if mem.read::<u8>(e + 6) != 5 {
                continue;
            }
            let id = i32::from(mem.read::<i16>(e)).cast_unsigned();
            run(mem, super::PPP_FP_GET_MNG_PTR, &[id, records, records + 4]);
            let fp_run = mem.read::<u32>(records);
            if fp_run == 0 {
                continue;
            }
            run(mem, k::PPP_SET_MATRIX, &[fp_run]);
            let m = mem.read::<u32>(records + 4).wrapping_add(0xc);
            let world = mem.rebase(k::PPP_WORLD_MATRIX);
            run(mem, super::fpm097::MUL_MATRIX, &[m, sea, world]);
        }
        run(mem, k::SET_MATRIX_DOBJ, &[10, matrix])
    }

    /// `rcbgChrFindById(id)` (`0x0092e1e0`): the first character whose
    /// `Ch_GetId` is `id`, through the character handle cursor; 0 for none.
    pub fn rcbg_chr_find_by_id(mem: &mut impl Mem, id: u32) -> u32 {
        use ffx_chara::handle::bridge as h;
        h::ch_handle_rewind(mem);
        let mut c = h::ch_handle_get_next(mem);
        while c != 0 {
            if h::ch_get_id(mem, c) == id {
                return c;
            }
            c = h::ch_handle_get_next(mem);
        }
        0
    }

    /// `rcbgChrHandleIndex(handle)` (`0x00924f30`): the battle slot
    /// (`MsGetChrHandle`, 0..=14 then 20..=27) holding `handle`, or -1.
    pub fn rcbg_chr_handle_index(mem: &impl Mem, handle: u32) -> u32 {
        (0..=14u32)
            .chain(20..=27)
            .find(|&i| ffx_battle::actor::bridge::ms_get_chr_handle(mem, i) == handle)
            .unwrap_or(u32::MAX)
    }

    // ---- round 6 ----

    /// `rcbgTrxLoad(trx, parse)` (`0x0092b710`): three 8-byte GS register
    /// values (`+0x18`, `+0x20`, `+0x28`), the entry count and size
    /// (`+0x30`, `+0x34`) from the parser, the entries at its position
    /// (`+0x38`), then past them (`rcbgTrxSetTag` is empty on PC).
    pub fn rcbg_trx_load(mem: &mut impl Mem, t: u32, p: u32) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x18), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x20), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x28), 8);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(trx::COUNT), 4);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(trx::ENTRY_SIZE), 4);
        let at = mem.read::<u32>(p.wrapping_add(4));
        mem.write(t.wrapping_add(trx::ENTRIES), at);
        let n = mem.read::<u32>(t.wrapping_add(trx::COUNT));
        let size = mem.read::<u32>(t.wrapping_add(trx::ENTRY_SIZE));
        rcdt_parse_advance(mem, p, n.wrapping_mul(size));
    }

    /// `Yn_SeaSetAng(v)` (`0x0091ae40`): the sea's angle (`+0x50` of its
    /// FP work, `FFXVu0CopyVectorXYZ`); returns that `eax`, 0 without the
    /// work.
    pub fn yn_sea_set_ang<M: Mem>(
        mem: &mut M,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => ffx_ps2::vu0::bridge::copy_vector_xyz(mem, w.wrapping_add(0x50), v),
        }
    }

    /// `Yn_SeaSetPos(v)` (`0x0091ae80`): its position (`+0x40`).
    pub fn yn_sea_set_pos<M: Mem>(
        mem: &mut M,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => ffx_ps2::vu0::bridge::copy_vector_xyz(mem, w.wrapping_add(0x40), v),
        }
    }

    /// `Yn_SeaSetUvSpd(u, v)` (`0x0091aea0`): the texture scroll speeds
    /// (`+0x60`, `+0x64`) and their flag (`+0xe4`).
    pub fn yn_sea_set_uv_spd<M: Mem>(
        mem: &mut M,
        u: u32,
        v: u32,
        run_of: impl FnOnce(&mut M, u32) -> u32,
    ) {
        let w = yn_sea_get_work(mem, run_of);
        if w != 0 {
            mem.write(w.wrapping_add(0xe4), 1u8);
            mem.write(w.wrapping_add(0x60), crate::x87::copy(u));
            mem.write(w.wrapping_add(0x64), crate::x87::copy(v));
        }
    }

    /// `Yn_PSimSetPos(psim, which, v)` (`0x00919f10`):
    /// [`rcbg_psim_set_pos`] for a simulation and `which` below 2.
    pub fn yn_psim_set_pos<M: Mem>(
        mem: &mut M,
        p: u32,
        which: i32,
        v: u32,
        call: impl FnMut(&mut M, u32, u32, u32),
    ) {
        if p != 0 && which < 2 {
            rcbg_psim_set_pos(mem, p, which.cast_unsigned(), v, call);
        }
    }

    /// `Yn_SeaGetAngPtr()` (`0x0091adf0`): the sea's angle (`+0x50` of its
    /// FP work), 0 without the work.
    pub fn yn_sea_get_ang_ptr<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => w.wrapping_add(0x50),
        }
    }

    /// `Yn_SeaGetPosPtr()` (`0x0091ae10`): its position (`+0x40`).
    pub fn yn_sea_get_pos_ptr<M: Mem>(mem: &mut M, run_of: impl FnOnce(&mut M, u32) -> u32) -> u32 {
        match yn_sea_get_work(mem, run_of) {
            0 => 0,
            w => w.wrapping_add(0x40),
        }
    }

    /// `rcbgPalCopyToLast(pal)` (`0x009299d0`): the palette's current
    /// entry (`i16` at `+8`) copied into the last (256 words), which is
    /// then made the transfer to run (`rcbgTrxSetNo`).
    pub fn rcbg_pal_copy_to_last(mem: &mut impl Mem, pal: u32) {
        let no = i32::from(mem.read::<i16>(pal.wrapping_add(8))).cast_unsigned();
        let src = rcbg_pal_get_entry_data(mem, pal, no);
        let dst = rcbg_pal_get_last_entry_data(mem, pal);
        for i in 0..256u32 {
            let v = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v);
        }
        let t = mem.read::<u32>(pal.wrapping_add(0xc));
        let n = mem.read::<u32>(t.wrapping_add(trx::COUNT)).wrapping_sub(1);
        rcbg_trx_set_no(mem, t, n);
    }

    /// `rcdtTm2CopyRect(tm2, dst, x, y, bytes, rows)` (`0x00923ae0`): rows
    /// of `bytes` from the image data (`+0x40`, row stride `u16` at
    /// `+0x24`) at byte `x` of row `y`, each to the next 8-byte boundary
    /// of `dst` (`memcpy`).
    pub fn rcdt_tm2_copy_rect(
        mem: &mut impl Mem,
        tm2: u32,
        dst: u32,
        x: u32,
        y: u32,
        bytes: u32,
        rows: i32,
    ) {
        let data = ptr32(mem.read::<u32>(tm2.wrapping_add(0x40)));
        let stride = u32::from(mem.read::<u16>(tm2.wrapping_add(0x24)));
        let mut src = stride.wrapping_mul(y).wrapping_add(data.wrapping_add(x));
        let step = bytes.wrapping_add(7) & !7;
        let mut d = dst;
        let mut buf = vec![0u8; bytes as usize];
        for _ in 0..rows.max(0) {
            mem.read_bytes(src, &mut buf);
            mem.write_bytes(d, &buf);
            d = d.wrapping_add(step);
            let stride = u32::from(mem.read::<u16>(tm2.wrapping_add(0x24)));
            src = src.wrapping_add(stride);
        }
    }

    /// `rcVec4fToFixed(out, v)` (`0x00925950`): `v`'s four floats times 16
    /// (`FFXVu0RotTransPers`'s `f64`) through `_ftol2_sse`, as 12.4 fixed
    /// point; returns the last.
    pub fn rc_vec4f_to_fixed(mem: &mut impl Mem, out: u32, v: u32) -> u32 {
        let k = Reg::constant(ffx_ps2::vu0::bridge::FIXED_SCALE.get(mem));
        let src = mem.read::<[f32; 4]>(v);
        let mut r = 0;
        for (i, x) in (0u32..).zip(src) {
            r = ffx_bridge::x87::ftol(mem, (Reg::load(x) * k).value());
            mem.write(out.wrapping_add(4 * i), r);
        }
        r
    }

    /// `rcMeshPastIndices(obj)` (`0x00928270`): the mesh's (`+0x14`) data
    /// (`+0xc`) past its index triples (`+0x14`, an `i16` count, 6 bytes
    /// each), rounded up to 16 bytes.
    pub fn rc_mesh_past_indices(mem: &impl Mem, obj: u32) -> u32 {
        let mesh = mem.read::<u32>(obj.wrapping_add(0x14));
        let mut n = i32::from(mem.read::<i16>(mesh.wrapping_add(0x14)))
            .wrapping_mul(6)
            .cast_unsigned();
        if n & 0xf != 0 {
            n = (n & !0xf).wrapping_add(0x10);
        }
        mem.read::<u32>(mesh.wrapping_add(0xc)).wrapping_add(n)
    }

    /// `rcfxGravityInit(fx)` (`0x00929570`): kind `0x30002`, its update
    /// (`0x00929560`) and draw (`0x00929500`) callbacks, the rest 0 (the
    /// four floats from `+0x1c` as 0.0). Returns `fx`.
    pub fn rcfx_gravity_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0x0003_0002u32);
        let up = mem.rebase(super::GRAVITY_UPDATE);
        mem.write(fx.wrapping_add(4), up);
        for off in [8u32, 0xc, 0x10] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        let draw = mem.rebase(super::GRAVITY_DRAW);
        mem.write(fx.wrapping_add(0x14), draw);
        for off in [0x18u32, 0x28, 0x24, 0x20, 0x1c] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        fx
    }

    /// `rcbgIdentity(x)` (`0x009202a0`): `x` (called on a saved register
    /// by `rcbgRestoreReg`).
    #[must_use]
    pub const fn rcbg_identity(x: u32) -> u32 {
        x
    }

    /// `rcefAnimeStep(anim)` (`0x0092f400`): adds the speed (`+0x44`) to
    /// the frame time (`+0x40`, `i16`); past the current key's length
    /// (`+0x12` of the 8-byte key `+0x42` in the table `+4`) moves to the
    /// next key, and past the last (`+6` of the table) loops (key flag
    /// 0x80: `+0x40` as a word cleared) or stops (bit 0 of `+0` cleared,
    /// time 0, the key kept). Returns the `eax` it ends with.
    pub fn rcef_anime_step(mem: &mut impl Mem, anim: u32) -> u32 {
        let table = mem.read::<u32>(anim.wrapping_add(4));
        let key = mem.read::<u16>(anim.wrapping_add(0x42));
        let k = table.wrapping_add(i32::from(key.cast_signed()).cast_unsigned().wrapping_mul(8));
        let speed = mem.read::<u16>(anim.wrapping_add(0x44));
        let t = mem.read::<u16>(anim.wrapping_add(0x40)).wrapping_add(speed);
        mem.write(anim.wrapping_add(0x40), t);
        let len = mem.read::<u16>(k.wrapping_add(0x12));
        if t.cast_signed() < len.cast_signed() {
            return u32::from(t);
        }
        let t = t.wrapping_sub(len);
        mem.write(anim.wrapping_add(0x40), t);
        let next = key.wrapping_add(1);
        mem.write(anim.wrapping_add(0x42), next);
        let eax = u32::from(key) + 1;
        if next.cast_signed() < mem.read::<i16>(table.wrapping_add(6)) {
            return eax;
        }
        if mem.read::<u8>(k.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(anim.wrapping_add(0x40), 0u32);
            return 0;
        }
        let f = mem.read::<u32>(anim);
        mem.write(anim, f & !1);
        mem.write(anim.wrapping_add(0x40), 0u16);
        mem.write(anim.wrapping_add(0x42), key);
        u32::from(key)
    }

    /// `rcfxParticleInit(fx)` (`0x009288e0`): the defaults: `+8` 0.03
    /// (`f64`), `+0x18` 0.001 and `+0x20` 0.1 (`f32`s from `.rdata`), the
    /// rest 0. Returns `fx`.
    pub fn rcfx_particle_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0u64);
        mem.write(fx.wrapping_add(0x1c), 0u32);
        let d = mem.rebase(super::PARTICLE_RATE);
        let d = mem.read::<u64>(d);
        mem.write(fx.wrapping_add(8), d);
        for off in [0x24u32, 0x28, 0x2c] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        mem.write(fx.wrapping_add(0x10), 0u64);
        mem.write(fx.wrapping_add(0x30), 0u32);
        let a = mem.rebase(super::PARTICLE_DEFAULTS[0]);
        let a = mem.read::<u32>(a);
        mem.write(fx.wrapping_add(0x34), 0u32);
        mem.write(fx.wrapping_add(0x18), crate::x87::copy(a));
        mem.write(fx.wrapping_add(0x38), 0u32);
        let b = mem.rebase(super::PARTICLE_DEFAULTS[1]);
        let b = mem.read::<u32>(b);
        mem.write(fx.wrapping_add(0x3c), 0u32);
        mem.write(fx.wrapping_add(0x20), crate::x87::copy(b));
        fx
    }

    /// `rcefSetupDraw(draw, tex, a, b)` (`0x0092f6b0`): fills a draw
    /// record: 1, `tex`, a GIF tag (`+0x10` 0x1c, `+0x14` from
    /// `0x00c498dc`, `+0x18` 0x44), `+0x30` its own `+0x88`, `+0x34`/`+0x38`
    /// two fixed buffers (`0x0230fd60`, `0x0230fda0`), `a` at `+0x80` and
    /// `+0x3c`, `b` at `+0x84`. Returns `draw + 0x88`.
    pub fn rcef_setup_draw(mem: &mut impl Mem, draw: u32, tex: u32, a: u32, b: u32) -> u32 {
        mem.write(draw.wrapping_add(4), tex);
        mem.write(draw.wrapping_add(0x84), b);
        mem.write(draw, 1u32);
        mem.write(draw.wrapping_add(0x80), a);
        mem.write(draw.wrapping_add(0x10), 0x1cu32);
        let at = mem.rebase(super::REF_GIF_TAG);
        let g = mem.read::<u32>(at);
        mem.write(draw.wrapping_add(0x14), g);
        mem.write(draw.wrapping_add(0x18), 0x44u8);
        mem.write(draw.wrapping_add(0x30), draw.wrapping_add(0x88));
        let b0 = mem.rebase(super::REF_BUFFERS[0]);
        let b1 = mem.rebase(super::REF_BUFFERS[1]);
        mem.write(draw.wrapping_add(0x34), b0);
        mem.write(draw.wrapping_add(0x38), b1);
        mem.write(draw.wrapping_add(0x3c), a);
        draw.wrapping_add(0x88)
    }

    /// `rcfxNailInit(fx)` (`0x00929100`): kind `0x30004`, its callbacks
    /// (`+4` `0x00929070`, `+0x10` `0x00929190`, `+0x14` `0x00928f40`),
    /// `+0x18` 2, `+0x30` 0.0 (`f64`), the rest 0. Returns `fx`.
    pub fn rcfx_nail_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx.wrapping_add(0x30), 0u64);
        mem.write(fx, 0x0003_0004u32);
        let [a, b, c] = super::NAIL_CALLBACKS.map(|f| mem.rebase(f));
        mem.write(fx.wrapping_add(4), a);
        mem.write(fx.wrapping_add(8), 0u32);
        mem.write(fx.wrapping_add(0xc), 0u32);
        mem.write(fx.wrapping_add(0x10), b);
        mem.write(fx.wrapping_add(0x14), c);
        mem.write(fx.wrapping_add(0x18), 2u32);
        for off in [0x1cu32, 0x20, 0x24, 0x28, 0x2c, 0x38] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        fx
    }

    /// `rcfxWindInit(fx)` (`0x00929850`): kind `0x30006`, its callbacks
    /// (`+4` `0x00929840`, `+0x14` `0x009296d0`), `+0x1c` 1.0, `+0x34` 0.1
    /// (from `.rdata`), `+0x40` 1.0 (`f64`), the rest 0. Returns `fx`.
    pub fn rcfx_wind_init(mem: &mut impl Mem, fx: u32) -> u32 {
        mem.write(fx, 0x0003_0006u32);
        let [a, b] = super::WIND_CALLBACKS.map(|f| mem.rebase(f));
        mem.write(fx.wrapping_add(4), a);
        for off in [8u32, 0xc, 0x10] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        mem.write(fx.wrapping_add(0x14), b);
        mem.write(fx.wrapping_add(0x18), 0u32);
        mem.write(fx.wrapping_add(0x1c), 1.0f32.to_bits());
        for off in [0x20u32, 0x24, 0x28, 0x2c, 0x30] {
            mem.write(fx.wrapping_add(off), 0u32);
        }
        let k = mem.rebase(super::PARTICLE_DEFAULTS[1]);
        let k = mem.read::<u32>(k);
        mem.write(fx.wrapping_add(0x34), crate::x87::copy(k));
        mem.write(fx.wrapping_add(0x38), 0u32);
        mem.write(fx.wrapping_add(0x40), 1.0f64.to_bits());
        mem.write(fx.wrapping_add(0x50), 0u64);
        mem.write(fx.wrapping_add(0x48), 0u64);
        mem.write(fx.wrapping_add(0x5c), 0u32);
        mem.write(fx.wrapping_add(0x58), 0u32);
        fx
    }

    /// `rcFree(p)` (`0x009258e0`, a jump to `user_free`) through the
    /// caller's `call(mem, function, args)`: returns its `eax`.
    pub fn rc_free<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        p: u32,
    ) -> u32 {
        let f = mem.rebase(super::RC_FREE);
        call(mem, f, &[p])
    }

    /// Frees the block a word at `at` points to (when set) and clears the
    /// word: the pattern of the `*Fini` functions. Returns `rcFree`'s
    /// `eax`, or 0 with nothing to free.
    pub fn free_field<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        at: u32,
    ) -> u32 {
        let p = mem.read::<u32>(at);
        if p == 0 {
            return 0;
        }
        let r = rc_free(mem, call, p);
        mem.write(at, 0u32);
        r
    }

    /// `rcbgModelFini(model)` (`0x00928240`): frees its animation data
    /// (`+0x14`).
    pub fn rcbg_model_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        model: u32,
        call: &mut C,
    ) -> u32 {
        free_field(mem, call, model.wrapping_add(0x14))
    }

    /// `rcbgModelDelete(model)` (`0x00928000`): [`rcbg_model_fini`], then
    /// the model itself freed.
    pub fn rcbg_model_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        model: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_model_fini(mem, model, call);
        rc_free(mem, call, model)
    }

    /// `rcbgTrxFini(trx)` (`0x0092d490`): frees the transfer's own data
    /// (`+0x38`) when flag bit 0 (`+0x10`) says it owns it.
    pub fn rcbg_trx_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        trx: u32,
        call: &mut C,
    ) {
        if mem.read::<u8>(trx.wrapping_add(0x10)) & 1 != 0 {
            free_field(mem, call, trx.wrapping_add(0x38));
        }
    }

    /// `rcbgTrxDelete(trx)` (`0x0092d450`): [`rcbg_trx_fini`], then the
    /// transfer freed.
    pub fn rcbg_trx_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        trx: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_trx_fini(mem, trx, call);
        rc_free(mem, call, trx)
    }

    /// `rcbgLayerAnimeFini(anime)` (`0x00930830`): frees `+0xc` (a
    /// layer's `+0x90` block).
    pub fn rcbg_layer_anime_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        call: &mut C,
    ) -> u32 {
        free_field(mem, call, a.wrapping_add(0xc))
    }

    /// `rcbgEmbeddedTrxGetCnt(obj)` (`0x00922140`): the entry count of the
    /// transfer embedded at `+0x20` ([`rcbg_trx_get_cnt`]); reached
    /// through `Yn_EmbeddedTrxGetCnt` (`0x00919bc0`).
    pub fn rcbg_embedded_trx_get_cnt(mem: &impl Mem, obj: u32) -> u32 {
        rcbg_trx_get_cnt(mem, obj.wrapping_add(0x20))
    }

    /// `rcefSetStream(stream)` (`0x00925710`, a table entry at
    /// `0x00c65524`): the stream word (`0x025d5c14`), and with one, mode 3
    /// at `0x025d5be0`. Returns `stream`.
    pub fn rcef_set_stream(mem: &mut impl Mem, s: u32) -> u32 {
        super::REF_STREAM.set(mem, s);
        if s != 0 {
            super::REF_STREAM_MODE.set(mem, 3);
        }
        s
    }

    /// `rcbgRunRecordHook(obj)` (`0x0091fd60`, set by `0x0091fe2e`): calls
    /// the hook at `+0xc` of the record `+0x14` on that record, when set.
    /// Returns its `eax`, else the record.
    pub fn rcbg_run_record_hook<M: Mem>(
        mem: &mut M,
        obj: u32,
        mut call: impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let r = mem.read::<u32>(obj.wrapping_add(0x14));
        let f = mem.read::<u32>(r.wrapping_add(0xc));
        if f == 0 {
            return r;
        }
        call(mem, f, r)
    }

    /// `rcAlloc(size, file, line)` (`0x009258a0`): `user_malloc(size)`
    /// (the file and line are for debug builds). Returns the block.
    pub fn rc_alloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        size: u32,
    ) -> u32 {
        let f = mem.rebase(super::USER_MALLOC);
        call(mem, f, &[size])
    }

    /// `rcCalloc(n, size, file, line)` (`0x009258b0`): [`rc_alloc`] of `n *
    /// size` bytes, zeroed.
    pub fn rc_calloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        n: u32,
        size: u32,
    ) -> u32 {
        let len = n.wrapping_mul(size);
        let p = rc_alloc(mem, call, len);
        mem.write_bytes(p, &vec![0u8; len as usize]);
        p
    }

    /// `rcRealloc(p, size, file, line)` (`0x009258f0`): [`rc_alloc`] without
    /// a block; the same block when its size (`Sg_MemGetSize`) is already
    /// `size`; else a new block with the smaller of the two sizes copied
    /// and the old one freed (`user_free`).
    pub fn rc_realloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        p: u32,
        size: u32,
    ) -> u32 {
        if p == 0 {
            return rc_alloc(mem, call, size);
        }
        let f = mem.rebase(super::SG_MEM_GET_SIZE);
        let old = call(mem, f, &[p]);
        if size == old {
            return p;
        }
        let n = if size.cast_signed() < old.cast_signed() {
            size
        } else {
            old
        };
        let f = mem.rebase(super::USER_MALLOC);
        let q = call(mem, f, &[size]);
        let mut bytes = vec![0u8; n as usize];
        mem.read_bytes(p, &mut bytes);
        mem.write_bytes(q, &bytes);
        let f = mem.rebase(super::USER_FREE);
        call(mem, f, &[p]);
        q
    }

    /// `yiMalloc(size)` (`0x0090bbd0`): a jump to `user_malloc`.
    pub fn yi_malloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        size: u32,
    ) -> u32 {
        rc_alloc(mem, call, size)
    }

    /// `yiMalloc64(size)` (`0x0090bbe0`): `Sg_MemSetAlign(0x40)`, then
    /// `user_malloc(size)`.
    pub fn yi_malloc64<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        size: u32,
    ) -> u32 {
        let f = mem.rebase(super::SG_MEM_SET_ALIGN);
        call(mem, f, &[0x40]);
        rc_alloc(mem, call, size)
    }

    /// `rcbgTrxNew()` (`0x0092d580`): a transfer (`0x40` bytes,
    /// [`rcbg_trx_init`]).
    pub fn rcbg_trx_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let t = rc_alloc(mem, call, 0x40);
        rcbg_trx_init(mem, t);
        t
    }

    /// `rcbgTrxTexInit(tex)` (`0x00922160`): a texture transfer object:
    /// kind `0x20003`, delete `rcbgTrxTexDelete`, draw `rcbgTrxTexDraw`,
    /// `+0x14` -1, two transfers at `+0x20` and `+0x60` ([`rcbg_trx_init`]).
    pub fn rcbg_trx_tex_init(mem: &mut impl Mem, t: u32) {
        for off in [0x10u32, 0x18, 0x1c] {
            mem.write(t.wrapping_add(off), 0u32);
        }
        mem.write(t, 0x0002_0003u32);
        let d = mem.rebase(super::TRX_TEX_DELETE);
        mem.write(t.wrapping_add(4), d);
        mem.write(t.wrapping_add(8), 0u32);
        let d = mem.rebase(super::TRX_TEX_DRAW);
        mem.write(t.wrapping_add(0xc), d);
        mem.write(t.wrapping_add(0x14), u32::MAX);
        rcbg_trx_init(mem, t.wrapping_add(0x20));
        rcbg_trx_init(mem, t.wrapping_add(0x60));
    }

    /// `rcbgTrxTexNew()` (`0x009221b0`): a texture transfer object (`0xa0`
    /// bytes, [`rcbg_trx_tex_init`]).
    pub fn rcbg_trx_tex_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let t = rc_alloc(mem, call, 0xa0);
        rcbg_trx_tex_init(mem, t);
        t
    }

    /// `rcbgFpTrxTexNew()` (`0x009220c0`): [`rcbg_trx_tex_new`] of kind
    /// `0x20006`.
    pub fn rcbg_fp_trx_tex_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let t = rcbg_trx_tex_new(mem, call);
        mem.write(t, 0x0002_0006u32);
        t
    }

    /// `rcbgModelInit(model)` (`0x00928320`): kind `0x20001`, delete
    /// `rcbgModelDelete`, `0x009283e0`, draw `rcbgModelDraw`; `+0x20` (0x70
    /// bytes) cleared, the bounding box `+0x90` empty, `+0xb0..` 0.0, the
    /// matrices `+0xc0` and `+0x100` the identity.
    pub fn rcbg_model_init(mem: &mut impl Mem, m: u32) {
        mem.write(m, 0x0002_0001u32);
        for (off, f) in [
            (4u32, super::MODEL_DELETE),
            (8, super::MODEL_UPDATE),
            (0xc, super::MODEL_DRAW),
        ] {
            let a = mem.rebase(f);
            mem.write(m.wrapping_add(off), a);
        }
        mem.write(m.wrapping_add(0x10), 0u32);
        mem.write(m.wrapping_add(0x14), 0u32);
        mem.write_bytes(m.wrapping_add(0x20), &[0; 0x70]);
        crate::geom::bridge::rc_bbox_init(mem, m.wrapping_add(0x90));
        for off in [0xbcu32, 0xb8, 0xb4, 0xb0] {
            mem.write(m.wrapping_add(off), 0u32);
        }
        ffx_ps2::vu0::bridge::matrix_identity(mem, m.wrapping_add(0xc0));
        ffx_ps2::vu0::bridge::matrix_identity(mem, m.wrapping_add(0x100));
    }

    /// `rcbgModelNew()` (`0x009283b0`): a model (`0x140` bytes,
    /// [`rcbg_model_init`]).
    pub fn rcbg_model_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let m = rc_alloc(mem, call, 0x140);
        rcbg_model_init(mem, m);
        m
    }

    /// `rcbgModelLoad(model, parse)` (`0x0092b2f0`): `+0x10`, then a sized
    /// block (its own allocation at `+0x14`), `+0x20` (0x70 bytes), `+0x90`
    /// (0x20), `+0xb0` (0x10) and the two matrices from the parser.
    pub fn rcbg_model_load<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        p: u32,
        call: &mut C,
    ) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcdt_parse_get_bin(mem, p, m.wrapping_add(0x10), 4);
        let at = mem.read::<u32>(p.wrapping_add(4));
        let size = mem.read::<u32>(at);
        rcdt_parse_advance(mem, p, 4);
        let block = rc_alloc(mem, call, size);
        mem.write(m.wrapping_add(0x14), block);
        rcdt_parse_get_bin(mem, p, block, size);
        for (off, n) in [
            (0x20u32, 0x70u32),
            (0x90, 0x20),
            (0xb0, 0x10),
            (0xc0, 0x40),
            (0x100, 0x40),
        ] {
            rcdt_parse_get_bin(mem, p, m.wrapping_add(off), n);
        }
    }

    /// `rcbgTrxTexLoad(tex, parse)` (`0x0092b780`): two `u16`s (`+0x10`,
    /// `+0x12`), 12 bytes skipped, the two transfers ([`rcbg_trx_load`]).
    pub fn rcbg_trx_tex_load(mem: &mut impl Mem, t: u32, p: u32) {
        use crate::rcdt::bridge::{rcdt_parse_advance, rcdt_parse_get_bin};
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x10), 2);
        rcdt_parse_get_bin(mem, p, t.wrapping_add(0x12), 2);
        rcdt_parse_advance(mem, p, 0xc);
        rcbg_trx_load(mem, t.wrapping_add(0x20), p);
        rcbg_trx_load(mem, t.wrapping_add(0x60), p);
    }

    /// `rcbgAnimeSetDataCnt(anime, n)` (`0x009308d0`): the frame table
    /// (`+0xc`, [`rc_realloc`]) resized to `n` words, new words cleared;
    /// the count to `+8`.
    pub fn rcbg_anime_set_data_cnt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        n: i32,
        call: &mut C,
    ) {
        let old = mem.read::<u32>(a.wrapping_add(0xc));
        let t = rc_realloc(mem, call, old, n.cast_unsigned().wrapping_mul(4));
        let have = mem.read::<i32>(a.wrapping_add(8));
        mem.write(a.wrapping_add(0xc), t);
        if n > have {
            let len = n.wrapping_sub(have).cast_unsigned().wrapping_mul(4);
            mem.write_bytes(
                t.wrapping_add(have.cast_unsigned().wrapping_mul(4)),
                &vec![0u8; len as usize],
            );
        }
        mem.write(a.wrapping_add(8), n);
    }

    /// `rcbgTrxTexFini(tex)` (`0x00922120`): its two transfers finished
    /// ([`rcbg_trx_fini`]).
    pub fn rcbg_trx_tex_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        rcbg_trx_fini(mem, t.wrapping_add(0x20), call);
        rcbg_trx_fini(mem, t.wrapping_add(0x60), call);
    }

    /// `rcbgTrxTexDelete(tex)` (`0x009220e0`): [`rcbg_trx_tex_fini`], then
    /// the object freed. Returns `rcFree`'s `eax`.
    pub fn rcbg_trx_tex_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_trx_tex_fini(mem, t, call);
        rc_free(mem, call, t)
    }

    /// The `*New` constructors of the `rc` modules: [`rc_alloc`] of the
    /// object's size, then its init. Each returns the object.
    fn rc_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        size: u32,
        init: impl FnOnce(&mut M, u32),
    ) -> u32 {
        let p = rc_alloc(mem, call, size);
        init(mem, p);
        p
    }

    /// `rcfxParticleNew()` (`0x00928950`): `0x40` bytes,
    /// [`rcfx_particle_init`].
    pub fn rcfx_particle_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x40, |m, p| {
            rcfx_particle_init(m, p);
        })
    }

    /// `rcfxNailNew()` (`0x00929170`): `0x40` bytes, [`rcfx_nail_init`].
    pub fn rcfx_nail_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x40, |m, p| {
            rcfx_nail_init(m, p);
        })
    }

    /// `rcfxGravityNew()` (`0x009295c0`): `0x2c` bytes,
    /// [`rcfx_gravity_init`].
    pub fn rcfx_gravity_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x2c, |m, p| {
            rcfx_gravity_init(m, p);
        })
    }

    /// `rcfxWindNew()` (`0x009298c0`): `0x60` bytes, [`rcfx_wind_init`].
    pub fn rcfx_wind_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x60, |m, p| {
            rcfx_wind_init(m, p);
        })
    }

    /// `rcfxDampNew()` (`0x00929420`): `0x20` bytes, [`rcfx_damp_init`].
    pub fn rcfx_damp_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x20, |m, p| {
            rcfx_damp_init(m, p);
        })
    }

    /// `rcbgPSimNew()` (`0x0091fe40`): `0xb0` bytes, [`rcbg_psim_init`].
    pub fn rcbg_psim_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0xb0, rcbg_psim_init)
    }

    /// `rcbgFPm097New()` (`0x009225a0`): `0x40` bytes,
    /// [`rcbg_fpm097_init`].
    pub fn rcbg_fpm097_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x40, |m, p| {
            rcbg_fpm097_init(m, p);
        })
    }

    /// `rcbgFPm173New()` (`0x00922bb0`): `0x28` bytes,
    /// [`rcbg_fpm173_init`].
    pub fn rcbg_fpm173_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x28, |m, p| {
            rcbg_fpm173_init(m, p);
        })
    }

    /// `rcbgFPm565New()` (`0x00922f80`): `0x10` bytes,
    /// [`rcbg_fpm565_init`].
    pub fn rcbg_fpm565_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x10, |m, p| {
            rcbg_fpm565_init(m, p);
        })
    }

    /// `rcdtRsd2New()` (`0x00924750`): `0x20` bytes, [`rcdt_rsd2_init`].
    pub fn rcdt_rsd2_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 0x20, rcdt_rsd2_init)
    }

    /// `rcdtChunkNew()` (`0x0092b900`): 8 bytes,
    /// `rcdtChunkInit`.
    pub fn rcdt_chunk_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        rc_new(mem, call, 8, crate::rcdt::bridge::rcdt_chunk_init)
    }

    /// `rcbgModelCreate(model, desc)` (`0x00927be0`): a primitive blob for
    /// `desc` (`n`, a pointer to `n` `{type, count, flags}` triples, the
    /// vertex and normal counts): its size (a 0x20 header, each packet's
    /// 0x10 header and `count` records of the type's stride, the end mark,
    /// six bytes per vertex and per normal, each part rounded up to 16),
    /// [`rc_alloc`]ed and zeroed, the header and packet headers filled, an
    /// end mark `0xffff`; the model's `+0x14` points at it and its GIF tag
    /// and matrices are reset. Returns 0 (the `eax` the original leaves).
    pub fn rcbg_model_create<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        desc: u32,
        call: &mut C,
    ) -> u32 {
        let up = |n: u32| if n & 0xf != 0 { (n & !0xf) + 0x10 } else { n };
        let strides = super::PACKET_STRIDES.ptr(mem).addr;
        let stride = |mem: &M, t: u32| u32::from(mem.read::<u8>(strides.wrapping_add(t)));
        let n = mem.read::<i32>(desc);
        let list = mem.read::<u32>(desc + 4);
        let (mut packets, mut prims) = (0u32, 0u32);
        for i in 0..n.max(0).cast_unsigned() {
            let e = list.wrapping_add(12 * i);
            let count = mem.read::<u32>(e + 4);
            prims = prims.wrapping_add(count);
            let s = stride(mem, mem.read(e));
            packets = packets
                .wrapping_add(0x10)
                .wrapping_add(s.wrapping_mul(count));
        }
        let packets = up(packets.wrapping_add(0x10));
        let verts = up(mem.read::<u32>(desc + 8).wrapping_mul(6));
        let normals = up(mem.read::<u32>(desc + 0xc).wrapping_mul(6));
        let size = normals
            .wrapping_add(0x20)
            .wrapping_add(verts)
            .wrapping_add(packets);
        let b = rc_alloc(mem, call, size);
        for i in 0..size {
            mem.write(b.wrapping_add(i), 0u8);
        }
        mem.write(b + 8, packets + 0x20);
        mem.write(b, 0u32);
        mem.write(b + 0xc, packets.wrapping_add(0x20).wrapping_add(verts));
        #[allow(clippy::cast_possible_truncation)] // `mov word`
        {
            mem.write(b + 0x10, prims as u16);
            mem.write(b + 4, 0x20u32);
            mem.write(b + 0x12, mem.read::<u16>(desc + 8));
            mem.write(b + 0x14, mem.read::<u16>(desc + 0xc));
        }
        mem.write(b + 0x16, 0x44u32);
        mem.write(b + 0x1a, 0u16);
        mem.write(b + 0x1c, 0u32);
        let mut p = b + 0x20;
        for i in 0..n.max(0).cast_unsigned() {
            let e = list.wrapping_add(12 * i);
            let t = mem.read::<u32>(e);
            let count = mem.read::<u32>(e + 4);
            mem.write(p, 0u8);
            mem.write(p.wrapping_add(1), mem.read::<u8>(e));
            mem.write(p.wrapping_add(2), mem.read::<u16>(e + 4));
            mem.write(p.wrapping_add(4), mem.read::<u8>(e + 8));
            mem.write(p.wrapping_add(5), 0u8);
            mem.write(p.wrapping_add(8), 0u32);
            let s = stride(mem, t);
            p = p.wrapping_add(0x10).wrapping_add(s.wrapping_mul(count));
        }
        mem.write(p, 0xffffu16);
        mem.write(m + 0x14, b);
        mem.write(m + 0x20, 0x1cu32);
        let tag = mem.read::<u32>(mem.rebase(super::REF_GIF_TAG));
        mem.write(m + 0x24, tag);
        mem.write(m + 0x28, 0x44u16);
        mem.write(m + 0x2a, 0u8);
        for o in [0x2c, 0x30, 0x34] {
            mem.write(m + o, 0u32);
        }
        mem.write(m + 0x38, 0x8080_8080u32);
        mem.write(m + 0x3c, 0u32);
        mem.write(m + 0x40, crate::geom::SCREEN_WORLD.ptr(mem).addr);
        mem.write(m + 0x64, 0u32);
        mem.write(m + 0x50, crate::geom::VIEW_WORLD.ptr(mem).addr);
        0
    }

    /// `rcbgModelCreateOnePrimH(model, type, count, verts, normals)`
    /// (`0x00927fa0`): [`rcbg_model_create`] with one packet (flags 0);
    /// the description is built in `work` (the original's stack, 0x1c
    /// bytes).
    #[allow(clippy::too_many_arguments)]
    pub fn rcbg_model_create_one_prim_h<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        kind: u32,
        count: u32,
        verts: u32,
        normals: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        mem.write(work, 1u32);
        mem.write(work + 4, work + 0x10);
        mem.write(work + 8, verts);
        mem.write(work + 0xc, normals);
        mem.write(work + 0x10, kind);
        mem.write(work + 0x14, count);
        mem.write(work + 0x18, 0u32);
        rcbg_model_create(mem, m, work, call)
    }

    /// `rcbgModelFToS0(out, points, n, scale)` (`0x009281d0`): each of `n`
    /// points (16 bytes) times `1 / scale` (`rcbgModelScaleVector`), cut to
    /// integers (`rcbgModelVectorToInt`, both into the original's stack:
    /// `work`, 0x20 bytes) and its x, y, z kept as `i16` triples at `out`.
    /// Returns the last z (the `eax` the original leaves), or 0 for none.
    pub fn rcbg_model_f_to_s0(
        mem: &mut impl Mem,
        out: u32,
        points: u32,
        n: i32,
        scale: u32,
        work: u32,
    ) -> u32 {
        use ffx_bridge::x87::Reg;
        let inv = (Reg::ONE / Reg::load(f32::from_bits(scale)))
            .store()
            .to_bits();
        let (scaled, ints) = (work, work + 0x10);
        let mut last = 0;
        let (mut o, mut p) = (out, points);
        for _ in 0..n.max(0) {
            crate::vu0::bridge::rcbg_scale_vector(mem, scaled, p, inv);
            crate::vu0::bridge::rcbg_model_vector_to_int(mem, ints, scaled);
            let x = mem.read::<u16>(ints);
            let y = mem.read::<u16>(ints + 4);
            last = mem.read::<u16>(ints + 8);
            mem.write(o, x);
            mem.write(o.wrapping_add(2), y);
            mem.write(o.wrapping_add(4), last);
            p = p.wrapping_add(0x10);
            o = o.wrapping_add(6);
        }
        u32::from(last)
    }

    /// `rcbgTrxTexSetAddr(tex, image, clut)` (`0x00922210`): the GS
    /// addresses kept at `+0x18`/`+0x1c` and both transfers' registers: the
    /// image (`+0x20`: base `image & 0x3fff`, width `image >> 14 & 0x3f`,
    /// format 0x13, the texture's size `+0x10`/`+0x12`) and the CLUT
    /// (`+0x60`: base `clut >> 5 & 0x3fff`, width 1, 16x16).
    #[allow(clippy::cast_possible_truncation)] // the registers are 16-bit
    pub fn rcbg_trx_tex_set_addr(mem: &mut impl Mem, t: u32, image: u32, clut: u32) {
        mem.write(t + 0x18, image);
        mem.write(t + 0x1c, clut);
        let w = mem.read::<i16>(t + 0x10);
        let h = mem.read::<i16>(t + 0x12);
        rcbg_trx_set_regs(
            mem,
            t + 0x20,
            (image & 0x3fff) as i16,
            (image >> 14 & 0x3f) as i16,
            0x13,
            0,
            0,
            w,
            h,
        );
        rcbg_trx_set_regs(
            mem,
            t + 0x60,
            (clut >> 5 & 0x3fff) as i16,
            1,
            0,
            0,
            0,
            0x10,
            0x10,
        );
    }

    /// The `if (p) { rcFree(p); p = 0; }` the `*Fini` functions repeat.
    fn free_word<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C, at: u32) {
        let p = mem.read::<u32>(at);
        if p != 0 {
            rc_free(mem, call, p);
            mem.write(at, 0u32);
        }
    }

    /// `rcdtListsFini(l)` (`0x009242f0`): `+0x4`, `+0xc`, `+0x14`, each of
    /// the `+0x18` pointers in the table at `+0x1c`, then the table freed
    /// (`rcFree`) and cleared.
    pub fn rcdt_lists_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        l: u32,
        call: &mut C,
    ) {
        for o in [4u32, 0xc, 0x14] {
            free_word(mem, call, l + o);
        }
        let mut i = 0u32;
        while i.cast_signed() < mem.read::<i32>(l + 0x18) {
            let at = mem.read::<u32>(l + 0x1c).wrapping_add(4 * i);
            let p = mem.read::<u32>(at);
            if p != 0 {
                rc_free(mem, call, p);
                let at = mem.read::<u32>(l + 0x1c).wrapping_add(4 * i);
                mem.write(at, 0u32);
            }
            i += 1;
        }
        free_word(mem, call, l + 0x1c);
    }

    /// `rcfxParticleAddForce(ps, force)` (`0x00928660`): the force inserted
    /// into the system's list (`+0x34` count, `+0x38` table, grown by
    /// `rcRealloc`) before the first with a greater priority (`+0x18`), else
    /// at the end.
    pub fn rcfx_particle_add_force<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ps: u32,
        force: u32,
        call: &mut C,
    ) {
        let n = mem.read::<i32>(ps + 0x34);
        let prio = mem.read::<i32>(force + 0x18);
        let list = mem.read::<u32>(ps + 0x38);
        let mut at = None;
        for i in 0..n.max(0) {
            let f = mem.read::<u32>(list.wrapping_add(4 * i.cast_unsigned()));
            if mem.read::<i32>(f + 0x18) > prio {
                at = Some(i);
                break;
            }
        }
        let size = n.wrapping_mul(4).wrapping_add(4).cast_unsigned();
        let new = rc_realloc(mem, call, list, size);
        match at {
            None => {
                mem.write(ps + 0x38, new);
                let n = mem.read::<u32>(ps + 0x34);
                mem.write(new.wrapping_add(4 * n), force);
                mem.write(ps + 0x34, n.wrapping_add(1));
            }
            Some(i) => {
                let n = mem.read::<i32>(ps + 0x34).wrapping_add(1);
                mem.write(ps + 0x34, n);
                mem.write(ps + 0x38, new);
                let mut k = n - 1;
                while k > i {
                    let t = mem
                        .read::<u32>(ps + 0x38)
                        .wrapping_add(4 * k.cast_unsigned());
                    let v = mem.read::<u32>(t.wrapping_sub(4));
                    mem.write(t, v);
                    k -= 1;
                }
                let t = mem
                    .read::<u32>(ps + 0x38)
                    .wrapping_add(4 * i.cast_unsigned());
                mem.write(t, force);
            }
        }
    }

    /// `rcfxParticleFiniForces(ps)` (`0x00928800`): each force's delete
    /// function (`+0x4`, when set) on it, then the table freed.
    pub fn rcfx_particle_fini_forces<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ps: u32,
        call: &mut C,
    ) {
        let mut i = 0u32;
        while i.cast_signed() < mem.read::<i32>(ps + 0x34) {
            let f = mem.read::<u32>(mem.read::<u32>(ps + 0x38).wrapping_add(4 * i));
            let d = mem.read::<u32>(f + 4);
            if d != 0 {
                call(mem, d, &[f]);
            }
            i += 1;
        }
        free_word(mem, call, ps + 0x38);
    }

    /// The `rcRealloc` of `n` units each to the given words (`*SetCnt`).
    fn realloc_words<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        p: u32,
        sizes: &[(u32, u32)],
    ) {
        for &(o, size) in sizes {
            let old = mem.read::<u32>(p + o);
            let new = rc_realloc(mem, call, old, size);
            mem.write(p + o, new);
        }
    }

    /// `rcfxParticleSetCnt(ps, n)` (`0x009289f0`): `n` at `+0x1c` and the
    /// four particle buffers (`+0x24..+0x30`) resized to `n` quadwords.
    pub fn rcfx_particle_set_cnt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ps: u32,
        n: u32,
        call: &mut C,
    ) {
        mem.write(ps + 0x1c, n);
        let s = n.wrapping_shl(4);
        realloc_words(mem, call, ps, &[(0x24, s), (0x28, s), (0x2c, s), (0x30, s)]);
    }

    /// `rcfxSpringFini(s)` (`0x00928c80`): its two buffers (`+0x20`,
    /// `+0x24`) freed and cleared.
    pub fn rcfx_spring_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        call: &mut C,
    ) {
        for o in [0x20u32, 0x24] {
            let p = mem.read::<u32>(s + o);
            if p != 0 {
                rc_free(mem, call, p);
            }
            mem.write(s + o, 0u32);
        }
    }

    /// `rcfxSpringInit(s)` (`0x00928ce0`): kind `0x30005`, its functions
    /// ([`super::SPRING_FUNCTIONS`]), no buffers, stiffness 0.8 and damping
    /// 0.4 (`.rdata` floats).
    pub fn rcfx_spring_init(mem: &mut impl Mem, s: u32) {
        mem.write(s, 0x0003_0005u32);
        for (o, f) in [(4u32, 0), (0x10, 1), (0x14, 2)] {
            let a = mem.rebase(super::SPRING_FUNCTIONS[f]);
            mem.write(s + o, a);
        }
        for o in [8u32, 0xc, 0x18, 0x1c, 0x20, 0x24] {
            mem.write(s + o, 0u32);
        }
        for (o, c) in [
            (0x28u32, super::SPRING_STIFFNESS),
            (0x2c, super::SPRING_DAMPING),
        ] {
            let v = mem.read::<u32>(mem.rebase(c));
            mem.write(s + o, crate::x87::copy(v));
        }
    }

    /// `rcfxSpringSetCnt(s, n)` (`0x00928d80`): `n` at `+0x1c`, the pairs
    /// (`+0x20`, 8 bytes each) and lengths (`+0x24`, 4 bytes) resized.
    pub fn rcfx_spring_set_cnt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        n: u32,
        call: &mut C,
    ) {
        mem.write(s + 0x1c, n);
        realloc_words(
            mem,
            call,
            s,
            &[(0x20, n.wrapping_mul(8)), (0x24, n.wrapping_mul(4))],
        );
    }

    /// 32767.0 and 2.0 (`f64`, `.rdata` `0x00b41d70`, `0x00b92270`):
    /// [`rc_random_dir_xz`]'s scale.
    pub const RANDOM_DIR_CONSTS: [u32; 2] = [0x00b4_1d70, 0x00b9_2270];
    /// `rcVec3fNormalize(v)` (`0x00921360`).
    pub const VEC3F_NORMALIZE: u32 = 0x0092_1360;

    /// `rcRandomDirXZ_0092e4f0(v)` (`0x0092e4f0`; name ours): a random
    /// direction in the XZ plane: x and z from `rand() / 32767.0 * 2.0 -
    /// 1.0` (each `rand()` stored as an `f32` first), y and w 0.0, then
    /// `rcVec3fNormalize(v)`; drawn again while x and z both end exactly
    /// 0.0 (a NaN ends it). The `eax` it leaves is the FPU status word in
    /// the normaliser's; callers drop it.
    pub fn rc_random_dir_xz<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        v: u32,
        call: &mut C,
    ) {
        use crate::x87::{load, store};
        let rand = mem.read::<u32>(mem.rebase(super::RAND_IMPORT));
        let [range, two] = RANDOM_DIR_CONSTS.map(|c| mem.read::<f64>(mem.rebase(c)));
        let normalize = mem.rebase(VEC3F_NORMALIZE);
        loop {
            let mut unit = |mem: &mut M| {
                let r = call(mem, rand, &[]).cast_signed();
                // `fild`, then `fstp m32`
                let r = store(f64::from(r));
                store(load(r) / range * two - 1.0)
            };
            let x = unit(mem);
            mem.write(v, x);
            let z = unit(mem);
            mem.write(v.wrapping_add(8), z);
            mem.write(v.wrapping_add(0xc), 0u32);
            mem.write(v.wrapping_add(4), 0u32);
            call(mem, normalize, &[v]);
            // `fucom` with 0.0: equal (not unordered) draws again
            let zero = |mem: &M, o: u32| f32::from_bits(mem.read::<u32>(v.wrapping_add(o))) == 0.0;
            if !zero(mem, 0) || !zero(mem, 8) {
                return;
            }
        }
    }

    /// `rcbgScaleVector_00928600(out, v, s)` and `rcVu0Add_009285a0(out,
    /// a, b)`, [`rcfx_particle_calc`]'s vector calls.
    pub const PARTICLE_CALC_CALLS: [u32; 2] = [0x0092_8600, 0x0092_85a0];

    /// `rcfxParticleCalc(sys)` (`0x00928710`, from `rcfxParticleSetTime`;
    /// name ours): one step of a particle system. The step `dt` (`+8`,
    /// `f64`, as `f32`), the mass (`+0x20`), the count (`+0x1c`) and the
    /// position (`+0x24`), velocity (`+0x2c`) and force (`+0x30`) arrays
    /// (16 bytes a particle) are taken first. The forces are zeroed, each
    /// of the `+0x34` force objects (`+0x38` table) with a callback
    /// (`+0x14`) runs it (`(force, sys)`), then each particle's velocity
    /// gains `force * (dt / mass)` and its position `velocity * dt`; `+8`
    /// is `dt` again, now exactly an `f32`. `work` is the original's
    /// 16-byte vector. Returns the `eax` it ends with (the last add's,
    /// else the last force object or callback result, else the force
    /// array).
    pub fn rcfx_particle_calc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        sys: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use crate::x87;
        let field = |mem: &M, o: u32| mem.read::<u32>(sys.wrapping_add(o));
        let mass = x87::copy(field(mem, 0x20));
        let count = field(mem, 0x1c).cast_signed();
        let forces = field(mem, 0x30);
        let pos = field(mem, 0x24);
        let dt = x87::store(mem.read::<f64>(sys.wrapping_add(8)));
        let vel = field(mem, 0x2c);
        // (with particles the last add's `eax` replaces this one)
        let mut eax = forces;
        for i in 0..count.max(0).cast_unsigned() {
            mem.write(forces.wrapping_add(i.wrapping_mul(0x10)), [0u32; 4]);
        }
        let mut i = 0;
        while i < field(mem, 0x34).cast_signed() {
            let force =
                mem.read::<u32>(field(mem, 0x38).wrapping_add(i.cast_unsigned().wrapping_mul(4)));
            eax = force;
            let callback = mem.read::<u32>(force.wrapping_add(0x14));
            if callback != 0 {
                eax = call(mem, callback, &[force, sys]);
            }
            i += 1;
        }
        let step = x87::store(x87::load(dt) / x87::load(mass));
        let [scale, add] = PARTICLE_CALC_CALLS.map(|f| mem.rebase(f));
        for i in 0..count.max(0).cast_unsigned() {
            let at = |base: u32| base.wrapping_add(i.wrapping_mul(0x10));
            call(mem, scale, &[work, at(forces), step]);
            call(mem, add, &[at(vel), at(vel), work]);
            call(mem, scale, &[work, at(vel), dt]);
            eax = call(mem, add, &[at(pos), at(pos), work]);
        }
        mem.write(sys.wrapping_add(8), x87::load(dt));
        eax
    }

    /// `rcfxSpringCalc(s, sys)` (`0x00928aa0`, the spring's `+0x14`
    /// callback): for each of the `+0x1c` springs, the pair `(a, b)`
    /// (`+0x20`, two `i32` particle indices) of the particle system's
    /// positions (`sys +0x24`), velocities (`+0x2c`) and forces (`+0x30`,
    /// 16 bytes a particle). With `d = pos[b] - pos[a]` (stored as `f32`s)
    /// and `l` its length, at least the spring's rest length (`+0x24`
    /// table) / 1000, the force is `d / l * (l - rest) * k` (`+0x28`) plus
    /// `(vel[b] - vel[a]) * c` (`+0x2c`), added to `a` and taken from `b`.
    /// The x87 order: `1 / l` and `(l - rest) * k` stored, `d * (1 / l) *
    /// that` stored, the damped sum stored, each force word read and
    /// written in turn (`a`'s x, y, z, then `b`'s). Returns the `eax` it
    /// ends with: the force array, or the rest length table when there
    /// are no springs.
    pub fn rcfx_spring_calc(mem: &mut impl Mem, s: u32, sys: u32) -> u32 {
        use crate::x87;
        fn ld(mem: &impl Mem, at: u32) -> f64 {
            x87::load(mem.read::<u32>(at))
        }
        let forces = mem.read::<u32>(sys.wrapping_add(0x30));
        let pos = mem.read::<u32>(sys.wrapping_add(0x24));
        let vel = mem.read::<u32>(sys.wrapping_add(0x2c));
        let count = mem.read::<i32>(s.wrapping_add(0x1c));
        let pairs = ptr32(mem.read::<u32>(s.wrapping_add(0x20)));
        let rests = ptr32(mem.read::<u32>(s.wrapping_add(0x24)));
        let k = x87::load(x87::copy(mem.read::<u32>(s.wrapping_add(0x28))));
        let c = x87::load(x87::copy(mem.read::<u32>(s.wrapping_add(0x2c))));
        if count <= 0 {
            return rests;
        }
        let mut i = 0u32;
        while i.cast_signed() < count {
            let pair = pairs.wrapping_add(i.wrapping_mul(8));
            let a = mem.read::<u32>(pair).wrapping_mul(0x10);
            let b = mem.read::<u32>(pair.wrapping_add(4)).wrapping_mul(0x10);
            let mut d = [0u32; 3];
            for (j, w) in (0u32..).zip(&mut d) {
                let at = 4 * j;
                *w = x87::store(
                    ld(mem, pos.wrapping_add(b + at)) - ld(mem, pos.wrapping_add(a + at)),
                );
            }
            let len = f32::from_bits(crate::geom::length3(d));
            let rests = ptr32(mem.read::<u32>(s.wrapping_add(0x24)));
            let rest = ld(mem, rests.wrapping_add(4 * i));
            let shortest = f32::from_bits(x87::store(
                rest / mem.read::<f64>(mem.rebase(super::SPRING_REST_SCALE)),
            ));
            // fcom: the shortest only when above the length (not NaN)
            let l = f64::from(if shortest > len { shortest } else { len });
            let inv = x87::load(x87::store(1.0 / l));
            let stretch = x87::load(x87::store((l - rest) * k));
            let pull = d.map(|w| x87::load(x87::store(x87::load(w) * inv * stretch)));
            let mut f = [0u32; 3];
            for ((j, w), pull) in (0u32..).zip(&mut f).zip(pull) {
                let at = 4 * j;
                let dv = ld(mem, vel.wrapping_add(b + at)) - ld(mem, vel.wrapping_add(a + at));
                *w = x87::store(dv * c + pull);
            }
            for (j, w) in (0u32..).zip(f) {
                let at = forces.wrapping_add(a + 4 * j);
                mem.write(at, x87::store(ld(mem, at) + x87::load(w)));
            }
            for (j, w) in (0u32..).zip(f) {
                let at = forces.wrapping_add(b + 4 * j);
                mem.write(at, x87::store(ld(mem, at) - x87::load(w)));
            }
            i += 1;
        }
        forces
    }

    /// `rcVec3fApplyMatrix(out, v, m)`, `rcCopyQword_00928eb0(dst, src)`,
    /// `rcVu0Sub_00928ee0(out, a, b)`, `rcbgScaleVector_00928e50(out, v,
    /// s)` and `rcVu0Add_00928df0(out, a, b)`: [`rcfx_nail_calc`]'s
    /// helpers.
    pub const NAIL_HELPERS: [u32; 5] = [
        0x0092_1300,
        0x0092_8eb0,
        0x0092_8ee0,
        0x0092_8e50,
        0x0092_8df0,
    ];

    /// `rcfxNailCalc(nail, sys)` (`0x00928f40`, the nail's `+0x14`
    /// callback; name ours): each of the `+0x1c` points' targets (`+0x28`)
    /// is its base (`+0x24`) through the matrix `+0x38` (`rcVec3fApplyMatrix`)
    /// or copied; with `t = (time - last) / (end - last)` (the system's
    /// `+0`, `+0x10` and the nail's `+0x30`, `f64`s; stored as `f32`) and
    /// `last` set to the time, each point (`+0x2c`) moves `t` of the way to
    /// its target (in `work`, 0x20 bytes) and is copied to the system's
    /// position (`sys +0x24`) of its particle (`+0x20` index), whose force
    /// (`sys +0x30`) is zeroed. Returns the `eax` it ends with: the force
    /// array, or the count with no points.
    pub fn rcfx_nail_calc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        nail: u32,
        sys: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [apply, copy, sub, scale, add] = NAIL_HELPERS.map(|f| mem.rebase(f));
        let rd = |mem: &M, o: u32| mem.read::<u32>(nail.wrapping_add(o));
        let forces = mem.read::<u32>(sys.wrapping_add(0x30));
        let positions = mem.read::<u32>(sys.wrapping_add(0x24));
        let (base, target, point) = (rd(mem, 0x24), rd(mem, 0x28), rd(mem, 0x2c));
        let count = mem.read::<i32>(nail.wrapping_add(0x1c));
        let n = count.max(0).cast_unsigned();
        let matrix = rd(mem, 0x38);
        for i in 0..n {
            let (b, a) = (target.wrapping_add(16 * i), base.wrapping_add(16 * i));
            if matrix == 0 {
                call(mem, copy, &[b, a]);
            } else {
                let m = rd(mem, 0x38);
                call(mem, apply, &[b, a, m]);
            }
        }
        let time = mem.read::<f64>(sys);
        let last = mem.read::<f64>(nail.wrapping_add(0x30));
        let end = mem.read::<f64>(sys.wrapping_add(0x10));
        let t = crate::x87::store((time - last) / (end - last));
        mem.write(nail.wrapping_add(0x30), time);
        if count <= 0 {
            return count.cast_unsigned();
        }
        let (step, moved) = (work, work.wrapping_add(0x10));
        for i in 0..n {
            let k = mem.read::<u32>(rd(mem, 0x20).wrapping_add(4 * i));
            let p = point.wrapping_add(16 * i);
            call(mem, sub, &[step, target.wrapping_add(16 * i), p]);
            call(mem, scale, &[moved, step, t]);
            call(mem, add, &[p, p, moved]);
            let at = k.wrapping_shl(4);
            call(mem, copy, &[positions.wrapping_add(at), p]);
            mem.write(forces.wrapping_add(at), [0u32; 4]);
        }
        forces
    }

    /// `rcfxNailFini(nail)` (`0x00929090`): its four buffers
    /// (`+0x20..+0x2c`) freed and cleared.
    pub fn rcfx_nail_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        nail: u32,
        call: &mut C,
    ) {
        for o in [0x20u32, 0x24, 0x28, 0x2c] {
            free_word(mem, call, nail + o);
        }
    }

    /// `rcfxNailSetCnt(nail, n)` (`0x00929230`): `n` at `+0x1c`, the
    /// indices (`+0x20`, a word each) and three quadword buffers resized.
    pub fn rcfx_nail_set_cnt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        nail: u32,
        n: u32,
        call: &mut C,
    ) {
        mem.write(nail + 0x1c, n);
        let q = n.wrapping_shl(4);
        realloc_words(
            mem,
            call,
            nail,
            &[(0x20, n.wrapping_mul(4)), (0x24, q), (0x28, q), (0x2c, q)],
        );
    }

    /// `rcbgScaleVector_00929640(out, v, s)`, `rcVu0Add_009295e0(out, a,
    /// b)` and `rcVec3fDot(a, b)` (in `ST0`): [`rcfx_wind_calc`]'s helpers.
    pub const WIND_HELPERS: [u32; 3] = [0x0092_9640, 0x0092_95e0, 0x0092_1220];
    /// 32767.0, 2.0 and 3.0 (`f64`s, `.rdata`): the gust's random range
    /// and the smoothstep's.
    pub const WIND_CONSTS: [u32; 3] = [0x00b4_1d70, 0x00b9_2270, 0x00b2_c5c8];

    /// `rcfxWindCalc(wind, sys)` (`0x009296d0`, the wind's `+0x14`
    /// callback; name ours): once the system's time (`sys +0`, an `f64`)
    /// passes the next gust (`+0x50`), that becomes the last (`+0x48`),
    /// the next is a period (`+0x40`) on, the target strength (`+0x5c`)
    /// becomes the start (`+0x58`) and a new one is `1 - 2 rand() /
    /// 32767` (`f32`). The strength is `from + (to - from) s` with `s` the
    /// smoothstep `u² (3 - 2u)` of `u = (time - last) / period` (stored as
    /// `f32`), the wind `+0x1c + +0x2c * strength` (in `work`, 0x20 bytes:
    /// the scaled vector, then the wind). Each of the `+0x1c` particles'
    /// forces (`+0x30`) gets the wind, or with normals (`+0x28`) the
    /// normal times `dot(wind, normal)`. Returns the `eax` it ends with.
    pub fn rcfx_wind_calc<M, C, F>(
        mem: &mut M,
        wind: u32,
        sys: u32,
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        use crate::x87::{copy, load, store};
        let [scale, add, dot] = WIND_HELPERS.map(|f| mem.rebase(f));
        let [range, two, three] = WIND_CONSTS.map(|c| mem.read::<f64>(mem.rebase(c)));
        let time = mem.read::<f64>(sys);
        let normals = mem.read::<u32>(sys.wrapping_add(0x28));
        let count = mem.read::<i32>(sys.wrapping_add(0x1c));
        let mut force = mem.read::<u32>(sys.wrapping_add(0x30));
        let at = |o: u32| wind.wrapping_add(o);
        // fcom: only strictly past the next gust (not NaN)
        if time > mem.read::<f64>(at(0x50)) {
            let next = mem.read::<f64>(at(0x50));
            mem.write(at(0x48), next);
            let to = copy(mem.read::<u32>(at(0x5c)));
            mem.write(at(0x58), to);
            mem.write(at(0x50), next + mem.read::<f64>(at(0x40)));
            let f = mem.read::<u32>(mem.rebase(super::RAND_IMPORT));
            let r = call(mem, f, &[]).cast_signed();
            let q = load(store(f64::from(r))) / range * two;
            mem.write(at(0x5c), store(1.0 - q));
        }
        let u = (time - mem.read::<f64>(at(0x48))) / mem.read::<f64>(at(0x40));
        let s = load(store((three - u * two) * (u * u)));
        let from = load(mem.read::<u32>(at(0x58)));
        let k = store(s * (load(mem.read::<u32>(at(0x5c))) - from) + from);
        let (scaled, gust) = (work, work.wrapping_add(0x10));
        call(mem, scale, &[scaled, at(0x2c), k]);
        let mut eax = call(mem, add, &[gust, at(0x1c), scaled]);
        if normals == 0 {
            for _ in 0..count.max(0) {
                eax = call(mem, add, &[force, force, gust]);
                force = force.wrapping_add(0x10);
            }
            return if count > 0 { eax } else { normals };
        }
        if count <= 0 {
            return normals;
        }
        let rel = normals.wrapping_sub(force);
        for _ in 0..count {
            let n = rel.wrapping_add(force);
            let d = store(fcall(mem, dot, crate::camera::FArgs::Stack(&[gust, n])));
            call(mem, scale, &[scaled, n, copy(d)]);
            call(mem, add, &[force, force, scaled]);
            force = force.wrapping_add(0x10);
        }
        rel
    }

    /// `rcfxWindSet(wind, dir, pos, power)` (`0x009298e0`): the two vectors
    /// (`rcCopyQword`) to `+0x1c`/`+0x2c` and the power as a double at
    /// `+0x40`.
    pub fn rcfx_wind_set(mem: &mut impl Mem, wind: u32, dir: u32, pos: u32, power: u32) {
        crate::vu0::bridge::rc_copy_qword(mem, wind + 0x1c, dir);
        crate::vu0::bridge::rc_copy_qword(mem, wind + 0x2c, pos);
        mem.write(wind + 0x40, crate::x87::load(power));
    }

    /// The end of the palette animations: the transfer's last entry made
    /// current (`rcbgTrxSetNo`).
    fn pal_commit(mem: &mut impl Mem, pal: u32) {
        let t = mem.read::<u32>(pal + 0xc);
        let n = mem.read::<u32>(t + 0x30).wrapping_sub(1);
        rcbg_trx_set_no(mem, t, n);
    }

    /// The step counter of the scrolling palette animations: `+0x4` counts
    /// up to the speed (`+0xb`); past it the offset (`+0x6`) moves one
    /// entry (wrapping at 0x100) and the counter restarts.
    fn pal_step(mem: &mut impl Mem, pal: u32) {
        let c = mem.read::<i16>(pal + 4).wrapping_add(1);
        mem.write(pal + 4, c);
        if c <= i16::from(mem.read::<u8>(pal + 0xb)) {
            return;
        }
        let o = mem.read::<i16>(pal + 6).wrapping_add(1);
        mem.write(pal + 6, if o < 0x100 { o } else { o.wrapping_sub(0x100) });
        mem.write(pal + 4, 0i16);
    }

    /// `memcpy`/`memset` as the palette animations use them.
    fn copy_bytes(mem: &mut impl Mem, dst: u32, src: Option<u32>, n: u32) {
        for i in 0..n {
            let v = src.map_or(0, |s| mem.read::<u8>(s.wrapping_add(i)));
            mem.write(dst.wrapping_add(i), v);
        }
    }

    /// `rcbgPalRotate(pal)` (`0x00929a10`): the entry (`+0x8`) rotated by
    /// the offset (`+0x6`) into the last entry, made current, then a step
    /// ([`pal_step`]).
    pub fn rcbg_pal_rotate(mem: &mut impl Mem, pal: u32) {
        let n = i32::from(mem.read::<i16>(pal + 8)).cast_unsigned();
        let src = rcbg_pal_get_entry_data(mem, pal, n);
        let dst = rcbg_pal_get_last_entry_data(mem, pal);
        let off = i32::from(mem.read::<i16>(pal + 6));
        let size = (0x100 - off).wrapping_mul(4).cast_unsigned();
        copy_bytes(
            mem,
            dst,
            Some(src.wrapping_add(off.wrapping_mul(4).cast_unsigned())),
            size,
        );
        let off = i32::from(mem.read::<i16>(pal + 6));
        copy_bytes(
            mem,
            dst.wrapping_add(size),
            Some(src),
            (off << 2).cast_unsigned(),
        );
        pal_commit(mem, pal);
        pal_step(mem, pal);
    }

    /// `rcbgPalShift(pal)` (`0x00929ac0`): as [`rcbg_pal_rotate`], the
    /// entries shifted in replaced by zeros.
    pub fn rcbg_pal_shift(mem: &mut impl Mem, pal: u32) {
        let n = i32::from(mem.read::<i16>(pal + 8)).cast_unsigned();
        let src = rcbg_pal_get_entry_data(mem, pal, n);
        let dst = rcbg_pal_get_last_entry_data(mem, pal);
        let off = i32::from(mem.read::<i16>(pal + 6));
        let size = (0x100 - off).wrapping_mul(4).cast_unsigned();
        copy_bytes(
            mem,
            dst,
            Some(src.wrapping_add(off.wrapping_mul(4).cast_unsigned())),
            size,
        );
        let off = i32::from(mem.read::<i16>(pal + 6));
        copy_bytes(
            mem,
            dst.wrapping_add(size),
            None,
            (off << 2).cast_unsigned(),
        );
        pal_commit(mem, pal);
        pal_step(mem, pal);
    }

    /// `rcbgPalWhite(pal)` (`0x00929ba0`): the entry (`+0x8`) with every
    /// colour white (alpha kept) into the last entry, made current.
    pub fn rcbg_pal_white(mem: &mut impl Mem, pal: u32) {
        let n = i32::from(mem.read::<i16>(pal + 8)).cast_unsigned();
        let src = rcbg_pal_get_entry_data(mem, pal, n);
        let dst = rcbg_pal_get_last_entry_data(mem, pal);
        for i in 0..0x100u32 {
            let v = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(dst.wrapping_add(4 * i), v | 0x00ff_ffff);
        }
        pal_commit(mem, pal);
    }

    /// `rcbgTrxAllocEntries(t, format, w, h, n)` (`0x0092d3e0`): `n` at
    /// `+0x30`; the entry size (`+0x34`) for format 0 (`4 w h + 16`) or
    /// 0x13 (`w h + 16`; others keep it); `n` entries [`rc_alloc`]ed to
    /// `+0x38`, flag `+0x10 | 1`.
    pub fn rcbg_trx_alloc_entries<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        format: i16,
        w: i16,
        h: i16,
        n: u32,
        call: &mut C,
    ) {
        mem.write(t + 0x30, n);
        let (w, h) = (i32::from(w), i32::from(h));
        match format {
            0 => mem.write(
                t + 0x34,
                w.wrapping_mul(h).wrapping_mul(4).wrapping_add(0x10),
            ),
            0x13 => mem.write(t + 0x34, w.wrapping_mul(h).wrapping_add(0x10)),
            _ => {}
        }
        let size = mem.read::<u32>(t + 0x34).wrapping_mul(n);
        let p = rc_alloc(mem, call, size);
        let f = mem.read::<u32>(t + 0x10);
        mem.write(t + 0x10, f | 1);
        mem.write(t + 0x38, p);
    }

    /// `rcbgTrxPacketMake(t, out)` (`0x0092d5a0`): the transfer's GS image
    /// upload at `out`: a GIF tag of four A+D registers (BITBLTBUF, TRXPOS,
    /// TRXREG from `+0x18..+0x2c`, TRXDIR 0), then an image tag of
    /// `+0x34 / 16` quadwords (`0x3000`) pointing at the current entry
    /// (`+0x3c`). Returns `out + 0x60`.
    pub fn rcbg_trx_make_packet(mem: &mut impl Mem, t: u32, out: u32) -> u32 {
        let words = [
            0x1000_0005,
            0,
            0,
            0,
            mem.read::<u32>(t + 0x18),
            mem.read::<u32>(t + 0x1c),
            0x50,
            0,
            mem.read::<u32>(t + 0x20),
            mem.read::<u32>(t + 0x24),
            0x51,
            0,
            mem.read::<u32>(t + 0x28),
            mem.read::<u32>(t + 0x2c),
            0x52,
            0,
            0,
            0,
            0x53,
            0,
        ];
        for (i, w) in (0u32..).zip(words) {
            mem.write(out + 4 * i, w);
        }
        #[allow(clippy::cast_possible_truncation)] // `mov word`
        mem.write(out + 0x50, (mem.read::<i32>(t + 0x34) / 16) as i16);
        mem.write(out + 0x52, 0x3000u16);
        let e = rcbg_trx_get_entry(mem, t, mem.read(t + 0x3c));
        mem.write(out + 0x54, e);
        mem.write(out + 0x58, 0u32);
        mem.write(out + 0x5c, 0u32);
        out + 0x60
    }

    /// `rcbgTrxSetRegsAlloc(t, a..g, n)` (`0x0092d320`): [`rcbg_trx_set_regs`]
    /// with the seven register words (their low halves), then
    /// [`rcbg_trx_alloc_entries`] with the format (`c`), the size (`f`,
    /// `g`) and `n` entries.
    #[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)] // the original's
    pub fn rcbg_trx_set_regs_alloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        r: [u32; 7],
        n: u32,
        call: &mut C,
    ) {
        let h = |v: u32| (v as u16).cast_signed();
        rcbg_trx_set_regs(
            mem,
            t,
            h(r[0]),
            h(r[1]),
            h(r[2]),
            h(r[3]),
            h(r[4]),
            h(r[5]),
            h(r[6]),
        );
        rcbg_trx_alloc_entries(mem, t, h(r[2]), h(r[5]), h(r[6]), n, call);
    }

    /// `rcdtListsDelete(l)` (`0x009242d0`): [`rcdt_lists_fini`], then the
    /// block freed (`rcFree`, a tail call).
    pub fn rcdt_lists_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        l: u32,
        call: &mut C,
    ) -> u32 {
        rcdt_lists_fini(mem, l, call);
        rc_free(mem, call, l)
    }

    /// `rcfxSpringNew()` (`0x00928d50`): 0x30 bytes ([`rc_alloc`]),
    /// [`rcfx_spring_init`].
    pub fn rcfx_spring_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let s = rc_alloc(mem, call, 0x30);
        rcfx_spring_init(mem, s);
        s
    }

    /// `rcfxParticleFini(ps)` (`0x00928870`): the four particle buffers
    /// (`+0x24..+0x30`) freed, then [`rcfx_particle_fini_forces`].
    pub fn rcfx_particle_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ps: u32,
        call: &mut C,
    ) {
        for o in [0x24u32, 0x28, 0x2c, 0x30] {
            free_word(mem, call, ps + o);
        }
        rcfx_particle_fini_forces(mem, ps, call);
    }

    /// `rcbgPSimAddDamp(psim, k)` (`0x0091f7a0`): a damping force
    /// (`rcfxDampNew`, `k` at `+0x1c`) added to the simulation's particle
    /// system (`+0x18`).
    pub fn rcbg_psim_add_damp<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        psim: u32,
        k: u32,
        call: &mut C,
    ) {
        let f = rcfx_damp_new(mem, call);
        mem.write(f.wrapping_add(0x1c), crate::x87::copy(k));
        let ps = mem.read::<u32>(psim + 0x18);
        rcfx_particle_add_force(mem, ps, f, call);
    }

    /// `rcbgPSimAddWind(psim, dir, pos, power)` (`0x0091f980`): a wind
    /// (`rcfxWindNew`, [`rcfx_wind_set`]) added to the particle system.
    pub fn rcbg_psim_add_wind<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        psim: u32,
        dir: u32,
        pos: u32,
        power: u32,
        call: &mut C,
    ) {
        let f = rcfx_wind_new(mem, call);
        rcfx_wind_set(mem, f, dir, pos, crate::x87::copy(power));
        let ps = mem.read::<u32>(psim + 0x18);
        rcfx_particle_add_force(mem, ps, f, call);
    }

    /// `rcVu0Add(out, a, b)`, `rcbgScaleVector_0091f6b0(out, v, s)`,
    /// `rcVu0Sub(out, a, b)` and `rcCopyQword_0091f710(dst, src)`:
    /// [`rcbg_psim_add_nail`]'s helpers.
    pub const ADD_NAIL_HELPERS: [u32; 4] = [0x0091_f650, 0x0091_f6b0, 0x0091_f740, 0x0091_f710];

    /// `rcbgPSimAddNail(psim, alt, ids, n)` (`0x0091f850`): a nail
    /// (`rcfxNailNew`, `n` points: [`rcfx_nail_set_cnt`]) on `n` particles
    /// (`ids`, `i32`s) of the simulation's system (`+0x18`, positions
    /// `+0x24`): their centre (the sum through `rcVu0Add` scaled by
    /// `f32(1 / f32(n))`, in `work`) is taken from each position, which is
    /// copied as the nail's base point (`+0x24`); the ids are copied to
    /// `+0x20`; the nail's matrix (`+0x38`) is the simulation's `+0x1c`
    /// (or `+0x5c` with `alt`) set to a unit matrix moved to the centre.
    /// The nail is added to the system ([`rcfx_particle_add_force`]).
    pub fn rcbg_psim_add_nail<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [psim, alt, ids, n]: [u32; 4],
        work: u32,
        call: &mut C,
    ) {
        let [add, scale, sub, copy] = ADD_NAIL_HELPERS.map(|f| mem.rebase(f));
        let nail = rcfx_nail_new(mem, call);
        let centre = work;
        mem.write(centre, [0u32; 4]);
        let sys = mem.read::<u32>(psim.wrapping_add(0x18));
        let positions = mem.read::<u32>(sys.wrapping_add(0x24));
        let count = n.cast_signed().max(0).cast_unsigned();
        let particle = |mem: &M, i: u32| {
            let id = mem.read::<u32>(ids.wrapping_add(4 * i));
            positions.wrapping_add(id.wrapping_shl(4))
        };
        for i in 0..count {
            let p = particle(mem, i);
            call(mem, add, &[centre, centre, p]);
        }
        let k = crate::x87::load(crate::x87::store(f64::from(n.cast_signed())));
        let s = crate::x87::store(1.0 / k);
        call(mem, scale, &[centre, centre, s]);
        rcfx_nail_set_cnt(mem, nail, n, call);
        let mut base = mem.read::<u32>(nail.wrapping_add(0x24));
        for i in 0..count {
            let p = particle(mem, i);
            call(mem, sub, &[p, p, centre]);
            let p = particle(mem, i);
            call(mem, copy, &[base, p]);
            base = base.wrapping_add(0x10);
        }
        let to = mem.read::<u32>(nail.wrapping_add(0x20));
        for b in 0..count * 4 {
            let v = mem.read::<u8>(ids.wrapping_add(b));
            mem.write(to.wrapping_add(b), v);
        }
        let m = psim.wrapping_add(if alt == 0 { 0x1c } else { 0x5c });
        mem.write(nail.wrapping_add(0x38), m);
        let unit = mem.rebase(super::fpm097::UNIT_MATRIX);
        call(mem, unit, &[m]);
        for i in 0..3u32 {
            let v = crate::x87::copy(mem.read::<u32>(centre + 4 * i));
            let m = mem.read::<u32>(nail.wrapping_add(0x38));
            mem.write(m.wrapping_add(0x30 + 4 * i), v);
        }
        let sys = mem.read::<u32>(psim.wrapping_add(0x18));
        rcfx_particle_add_force(mem, sys, nail, call);
    }

    /// `rcbgPSimAddGravity(psim, g)` (`0x0091f7c0`): `g` replaced by the
    /// debug gravity (0, 1, -0.1, 0; two `rcPrint` messages, no-ops), a
    /// gravity force with it (`rcfxGravityNew`, `+0x1c`) added to the
    /// particle system.
    pub fn rcbg_psim_add_gravity<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        psim: u32,
        g: u32,
        call: &mut C,
    ) {
        mem.write(g, 0u32);
        mem.write(g + 4, 1f32);
        let v = mem.read::<u32>(mem.rebase(super::GRAVITY_Z));
        mem.write(g + 8, crate::x87::copy(v));
        mem.write(g + 0xc, 0u32);
        let f = rcfx_gravity_new(mem, call);
        crate::vu0::bridge::rc_copy_qword(mem, f.wrapping_add(0x1c), g);
        let ps = mem.read::<u32>(psim + 0x18);
        rcfx_particle_add_force(mem, ps, f, call);
    }

    /// `rcbgPalProc(pal)` (`0x00929b60`): flag bit 0 [`rcbg_pal_white`],
    /// else by the mode (`+0xa`) 1 [`rcbg_pal_rotate`], 2
    /// [`rcbg_pal_shift`], anything else [`rcbg_pal_copy_to_last`].
    pub fn rcbg_pal_proc(mem: &mut impl Mem, pal: u32) {
        if mem.read::<u8>(pal) & 1 != 0 {
            rcbg_pal_white(mem, pal);
            return;
        }
        match mem.read::<u8>(pal + 0xa) {
            1 => rcbg_pal_rotate(mem, pal),
            2 => rcbg_pal_shift(mem, pal),
            _ => rcbg_pal_copy_to_last(mem, pal),
        }
    }

    /// `rcbgModelSetTexReg(page)` (`0x009284d0`): a GS register record (the
    /// 64-bit `page << 17 | 0x1000f`, register 0x47, then 0, 0, 0, -1, -1)
    /// built on the stack (`work`, 0x20 bytes) for `rcbgSetReg`.
    pub fn rcbg_model_set_tex_reg<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        page: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let v = (i64::from(page.cast_signed()) << 17) | 0x1000f;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the halves
        let (lo, hi) = (v as u32, (v >> 32) as u32);
        for (i, w) in (0u32..).zip([lo, hi, 0x47, 0, 0, 0, u32::MAX, u32::MAX]) {
            mem.write(work + 4 * i, w);
        }
        let f = mem.rebase(super::RCBG_SET_REG);
        call(mem, f, &[work])
    }

    /// `rcbgFPmGetElemMatrix(out, chr, elem)` (`0x00922350`): a character
    /// element's matrix (`Ch_ElemGetMatrix(chr, elem, out)`), its three rows
    /// times 10.
    pub fn rcbg_fpm_get_elem_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        out: u32,
        chr: u32,
        elem: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[chr, elem, out]);
        crate::vu0::bridge::rcbg_scale_rows(mem, out, crate::vu0::SCALE_10);
    }

    /// `rcbgFPmGetMatrix(fp, out)` (`0x00922290`): the object's rotation
    /// (`+0x18`, scaled to radians by the `.rdata` float at `0x00b678ec`
    /// into `work`, 0x10 bytes), `out` the identity scaled by `+0x28`, then
    /// `op_rt_m_yxz(out, out, rotation, +0x8)`.
    pub fn rcbg_fpm_get_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        fp: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let s = crate::x87::copy(mem.read::<u32>(mem.rebase(super::FPM_ANGLE_SCALE)));
        crate::vu0::bridge::rcbg_scale_vector(mem, work, fp + 0x18, s);
        ffx_ps2::vu0::bridge::matrix_identity(mem, out);
        crate::geom::bridge::rc_mat4f_set_scale_vec3f(mem, out, fp + 0x28);
        let f = mem.rebase(super::OP_RT_M_YXZ);
        call(mem, f, &[out, out, work, fp + 8])
    }

    /// `rcbgFPmRestart(id, chr, elem)` (`0x00922300`): with the field
    /// program's manager (`pppFpGetMngPtr(id, 0, &mng)`, `mng` in `work`):
    /// flag 0x100, `Fp_Restart(id)` and the character element's matrix
    /// into `mng + 0xc`.
    pub fn rcbg_fpm_restart<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        chr: u32,
        elem: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::PPP_FP_GET_MNG_PTR);
        call(mem, f, &[id, 0, work]);
        let mng = mem.read::<u32>(work);
        if mng == 0 {
            return 0;
        }
        let v = mem.read::<u32>(mng);
        mem.write(mng, v | 0x100);
        let f = mem.rebase(super::FP_RESTART);
        call(mem, f, &[id]);
        let mng = mem.read::<u32>(work);
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[chr, elem, mng.wrapping_add(0xc)])
    }

    /// `rcbgPalAlloc(pal, entries)` (`0x00929950`): a new transfer
    /// (`rcbgTrxNew`) at `+0xc` for `entries + 1` 16x16 CLUTs at GS page
    /// `0x2e00` ([`rcbg_trx_set_regs_alloc`]).
    pub fn rcbg_pal_alloc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        pal: u32,
        entries: u32,
        call: &mut C,
    ) {
        let t = rcbg_trx_new(mem, call);
        mem.write(pal + 0xc, t);
        rcbg_trx_set_regs_alloc(
            mem,
            t,
            [0x2e00, 1, 0, 0, 0, 0x10, 0x10],
            entries.wrapping_add(1),
            call,
        );
    }

    /// `rcfxParticleDelete(ps)` (`0x00928850`): [`rcfx_particle_fini`],
    /// then the system freed (`rcFree`, a tail call).
    pub fn rcfx_particle_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ps: u32,
        call: &mut C,
    ) -> u32 {
        rcfx_particle_fini(mem, ps, call);
        rc_free(mem, call, ps)
    }

    /// `rcbgFPm173ProcSphere(fp)` (`0x00922eb0`): at step 0x7c (`+0x1c`)
    /// field program 0x4e restarts on the character's (`+0x10`) element 8
    /// (`Ch_OptposGetElement(chr, 8, 0)`, [`rcbg_fpm_restart`]) and `+0x20`
    /// is cleared. `work` as for [`rcbg_fpm_restart`].
    pub fn rcbg_fpm173_proc_sphere<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        fp: u32,
        work: u32,
        call: &mut C,
    ) {
        if mem.read::<u32>(fp + 0x1c) != 0x7c {
            return;
        }
        let chr = mem.read::<u32>(fp + 0x10);
        let f = mem.rebase(super::CH_OPTPOS_GET_ELEMENT);
        let elem = call(mem, f, &[chr, 8, 0]);
        let chr = mem.read::<u32>(fp + 0x10);
        rcbg_fpm_restart(mem, 0x4e, chr, elem, work, call);
        mem.write(fp + 0x20, 0u32);
    }

    /// `rcbgFPmGetPartData(id, n, part, k)` (`0x00923190`): from the field
    /// program's data (`pppFpGetMngPtr(id, &data, 0)`, `data` in `work`):
    /// its `+0x30` chain followed `n` times, that node's `+0x8` table,
    /// its first entry's part `part` (0x10 bytes from `+0x28`), whose
    /// `+0xc` offsets give the `k`th block, `0xa0` past the node; 0 when
    /// any link is missing.
    pub fn rcbg_fpm_get_part_data<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        n: i32,
        part: u32,
        k: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::PPP_FP_GET_MNG_PTR);
        call(mem, f, &[id, work, 0]);
        let data = mem.read::<u32>(work);
        if data == 0 {
            return 0;
        }
        let mut node = mem.read::<u32>(data + 0x30);
        for _ in 0..n.max(0) {
            if node != 0 {
                node = mem.read::<u32>(node);
            }
        }
        if node == 0 {
            return 0;
        }
        let t = mem.read::<u32>(node + 8);
        if t == 0 {
            return 0;
        }
        let e = mem.read::<u32>(t);
        if e == 0 {
            return 0;
        }
        let p = part.wrapping_shl(4).wrapping_add(0x28).wrapping_add(e);
        if p == 0 {
            return 0;
        }
        let offsets = mem.read::<u32>(p.wrapping_add(0xc));
        mem.read::<u32>(offsets.wrapping_add(k.wrapping_mul(4)))
            .wrapping_add(0xa0)
            .wrapping_add(node)
    }

    /// `rcbgPalLoadConfig(pal, parse)` (`0x0093adf0`): after the next
    /// token, keywords from the table at `0x00c60e94` until the parser
    /// ends: 0 the mode byte `+0xa`, 1 the speed byte `+0xb` (integers), 2
    /// stops; others are skipped.
    pub fn rcbg_pal_load_config<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        pal: u32,
        p: u32,
        call: &mut C,
    ) {
        use crate::rcdt::bridge as rb;
        rb::rcdt_parse_next_token(mem, p);
        let table = mem.rebase(super::PAL_CONFIG_KEYS);
        while rb::rcdt_parse_is_end(mem, p) == 0 {
            match rb::rcdt_parse_get_keyword(mem, p, table, call) {
                0 => {
                    #[allow(clippy::cast_possible_truncation)] // `mov byte`
                    let v = rb::rcdt_parse_get_int(mem, p, call) as u8;
                    mem.write(pal + 0xa, v);
                }
                1 => {
                    #[allow(clippy::cast_possible_truncation)] // `mov byte`
                    let v = rb::rcdt_parse_get_int(mem, p, call) as u8;
                    mem.write(pal + 0xb, v);
                }
                2 => return,
                _ => {}
            }
        }
    }

    /// `rcbgSceneLoadTextSubConfigKeys(obj, parse)` (`0x0093aee0`): after
    /// the next token, keywords from the table at `0x00c60e44` until the
    /// parser ends: 0, 1, 2 the integers at `+0x5c`, `+0x60`, `+0x64`; 3
    /// stops; others are skipped.
    pub fn rcbg_scene_load_text_sub_keys<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        p: u32,
        call: &mut C,
    ) {
        use crate::rcdt::bridge as rb;
        rb::rcdt_parse_next_token(mem, p);
        let table = mem.rebase(super::TEXT_SUB_KEYS);
        while rb::rcdt_parse_is_end(mem, p) == 0 {
            let at = match rb::rcdt_parse_get_keyword(mem, p, table, call) {
                0 => 0x5c,
                1 => 0x60,
                2 => 0x64,
                3 => return,
                _ => continue,
            };
            let v = rb::rcdt_parse_get_int(mem, p, call);
            mem.write(o + at, v);
        }
    }

    // ---- round 13 ----

    /// `rcbgPSimFini(p)` (`0x0091fd80`): the model (`+0x14`) and particle
    /// system (`+0x18`) deleted and cleared when set. Returns
    /// `rcfxParticleDelete`'s `eax`, or 0 without a particle system.
    pub fn rcbg_psim_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        let m = mem.read::<u32>(p.wrapping_add(0x14));
        if m != 0 {
            rcbg_model_delete(mem, m, call);
            mem.write(p.wrapping_add(0x14), 0u32);
        }
        let ps = mem.read::<u32>(p.wrapping_add(0x18));
        if ps == 0 {
            return 0;
        }
        let r = rcfx_particle_delete(mem, ps, call);
        mem.write(p.wrapping_add(0x18), 0u32);
        r
    }

    /// `rcbgPSimDelete(p)` (`0x0091fd40`): [`rcbg_psim_fini`], then `p`
    /// freed (a tail call of `rcFree`).
    pub fn rcbg_psim_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        rcbg_psim_fini(mem, p, call);
        rc_free(mem, call, p)
    }

    /// `rcbgAnimeLoadConfig(anime, parse)` (`0x0093a650`): after the next
    /// token, keywords from the table at `0x00c60e84` until the parser
    /// ends: 0 the frame count (`rcbgAnimeSetDataCnt`), 1 that many
    /// four-byte frames (integers) into the data at `+0xc` between two
    /// tokens, 2 stops (returns 0); others are skipped. Returns the
    /// parser's end flag otherwise.
    pub fn rcbg_anime_load_config<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        anime: u32,
        p: u32,
        call: &mut C,
    ) -> u32 {
        use crate::rcdt::bridge as rb;
        let (mut count, mut last) = (0i32, 0i32);
        rb::rcdt_parse_next_token(mem, p);
        let table = mem.rebase(super::ANIME_CONFIG_KEYS);
        loop {
            let end = rb::rcdt_parse_is_end(mem, p);
            if end != 0 {
                return end;
            }
            match rb::rcdt_parse_get_keyword(mem, p, table, call) {
                0 => {
                    count = rb::rcdt_parse_get_int(mem, p, call).cast_signed();
                    last = count;
                    let f = mem.rebase(super::ANIME_SET_DATA_CNT);
                    call(mem, f, &[anime, count.cast_unsigned()]);
                }
                1 => {
                    rb::rcdt_parse_next_token(mem, p);
                    let mut d = mem.read::<u32>(anime.wrapping_add(0xc));
                    if count > 0 {
                        for _ in 0..count {
                            for k in 0..4 {
                                #[allow(clippy::cast_possible_truncation)] // `mov byte`
                                let v = rb::rcdt_parse_get_int(mem, p, call) as u8;
                                mem.write(d.wrapping_add(k), v);
                            }
                            d = d.wrapping_add(4);
                        }
                        count = last;
                    }
                    rb::rcdt_parse_next_token(mem, p);
                }
                2 => return 0,
                _ => {}
            }
        }
    }

    // ---- field round 49: map 0xad's effect steps ----

    /// `rcbgFPm173ProcKonjyo(obj)` (`0x00922e60`): at step (`+0x1c`) 0
    /// the effect 0x58 sticks to the object's character (`+0x10`, bone 2)
    /// and restarts; at 0x32 the effect 0x59 (bone 0x35) does, and `+0x20`
    /// becomes 0. Returns the restart's `eax`, else the step (left in
    /// `eax`).
    pub fn rcbg_fpm173_proc_konjyo<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        call: &mut C,
    ) -> u32 {
        let step = mem.read::<u32>(obj.wrapping_add(0x1c));
        let chr = mem.read::<u32>(obj.wrapping_add(0x10));
        let (fp, bone) = match step {
            0 => (0x58, 2),
            0x32 => (0x59, 0x35),
            _ => return step,
        };
        let stick = mem.rebase(super::PPP_FP_SET_STICK_CHR);
        call(mem, stick, &[fp, chr, bone]);
        let restart = mem.rebase(super::FP_RESTART);
        let r = call(mem, restart, &[fp]);
        if step != 0 {
            mem.write(obj.wrapping_add(0x20), 0u32);
        }
        r
    }

    /// `rcbgFPm173ProcJect(obj)` (`0x00922c50`), by step (`+0x1c`):
    /// - 0x25: `rcbgFPmRestart(0x53, chr, 0x2f)`;
    /// - 0x82: effect 0x54 sticks to the character (bone 2) and restarts;
    /// - 200: `Fp_End(0x54)`;
    /// - 0x100: `rcbgFPmRestart(0x55, chr, 0x5d)` and `+0x20` 0.
    ///
    /// `chr` is `+0x10`. Returns the last call's `eax`; another step
    /// leaves its jump-table index in `eax`: 4 for steps 0x25..=0x100, else
    /// the step less 0x25.
    pub fn rcbg_fpm173_proc_ject<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        call: &mut C,
    ) -> u32 {
        let step = mem.read::<u32>(obj.wrapping_add(0x1c));
        let chr = mem.read::<u32>(obj.wrapping_add(0x10));
        let restart = mem.rebase(super::RCBG_FPM_RESTART);
        match step {
            0x25 => call(mem, restart, &[0x53, chr, 0x2f]),
            0x82 => {
                let stick = mem.rebase(super::PPP_FP_SET_STICK_CHR);
                call(mem, stick, &[0x54, chr, 2]);
                let f = mem.rebase(super::FP_RESTART);
                call(mem, f, &[0x54])
            }
            200 => {
                let f = mem.rebase(super::FP_END);
                call(mem, f, &[0x54])
            }
            0x100 => {
                let r = call(mem, restart, &[0x55, chr, 0x5d]);
                mem.write(obj.wrapping_add(0x20), 0u32);
                r
            }
            _ => {
                let i = step.wrapping_sub(0x25);
                if i <= 0xdb { 4 } else { i }
            }
        }
    }

    // ---- field agent r2, group E: the FP start lists ----

    /// `rcbgFPmStartList(list, n)` (`0x009223b0`, name ours): each of the
    /// `n` 0x38-byte entries (id `i16` at `+0`) starts its FP effect: run
    /// (`pppFpGetMngPtr` into `work`; a missing one ends the list),
    /// `pppFpResetStick`, then by `+6`: 1 stuck to character `+2` (bone
    /// `+4`), 2 to map part `+2`, 3 and 4 run flag 4, 5 run flag 0x100;
    /// run flags 0x60, the matrix (`rcbgFPmGetMatrix(entry, run + 0xc)`),
    /// and `Fp_Restart` with `+7` set, else `Fp_End` (the id unsigned).
    /// Returns 1 once an entry was started, else 0.
    pub fn rcbg_fpm_start_list<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        list: u32,
        n: i32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let mut ret = 0;
        for k in 0..n.max(0).cast_unsigned() {
            let e = list.wrapping_add(0x38 * k);
            let sx =
                |mem: &M, o: u32| i32::from(mem.read::<i16>(e.wrapping_add(o))).cast_unsigned();
            let id = sx(mem, 0);
            let f = mem.rebase(super::PPP_FP_GET_MNG_PTR);
            call(mem, f, &[id, 0, work]);
            if mem.read::<u32>(work) == 0 {
                return ret;
            }
            let f = mem.rebase(super::PPP_FP_RESET_STICK);
            call(mem, f, &[id]);
            let or = |mem: &mut M, bits: u32| {
                let run = mem.read::<u32>(work);
                let v = mem.read::<u32>(run) | bits;
                mem.write(run, v);
            };
            match mem.read::<u8>(e.wrapping_add(6)) {
                1 => {
                    let i = sx(mem, 2);
                    let h = crate::task::bridge::rcef_get_chr_handle(mem, i, call);
                    let f = mem.rebase(super::PPP_FP_SET_STICK_CHR);
                    let bone = sx(mem, 4);
                    call(mem, f, &[id, h, bone]);
                }
                2 => {
                    let f = mem.rebase(super::PPP_FP_SET_STICK_MAP);
                    let part = sx(mem, 2);
                    call(mem, f, &[id, part]);
                }
                3 | 4 => or(mem, 4),
                5 => or(mem, 0x100),
                _ => {}
            }
            or(mem, 0x60);
            let run = mem.read::<u32>(work);
            let f = mem.rebase(super::RCBG_FPM_GET_MATRIX);
            call(mem, f, &[e, run.wrapping_add(0xc)]);
            let uid = u32::from(mem.read::<u16>(e));
            let f = if mem.read::<u8>(e.wrapping_add(7)) == 0 {
                super::FP_END
            } else {
                super::FP_RESTART
            };
            let f = mem.rebase(f);
            call(mem, f, &[uid]);
            ret = 1;
        }
        ret
    }

    /// The start of a map effect with a fixed FP list
    /// (`rcbgFPm097Proc000`, `rcbgFPm565Proc000`): [`rcbg_fpm_start_list`]
    /// of (`list`, `n`); once started, the object's step function (`+8`)
    /// becomes `next`. Returns the object then, else 0.
    pub fn rcbg_fpm_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        [list, n, next]: [u32; 3],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let list = mem.rebase(list);
        let r = rcbg_fpm_start_list(mem, list, n.cast_signed(), work, call);
        if r == 0 {
            return 0;
        }
        let next = mem.rebase(next);
        mem.write(obj.wrapping_add(8), next);
        obj
    }

    // ---- field r14 ----

    /// `rcVu0IntToFloat(out, v)` (`0x00927ba0`) and
    /// `rcbgModelScaleVector(out, v, s)` (`0x00927b00`):
    /// [`rcbg_model_shorts_to_vectors`]'s helpers.
    pub const SHORTS_TO_VECTORS_CALLS: [u32; 2] = [0x0092_7ba0, 0x0092_7b00];

    /// `rcbgModelShortsToVectors_009283f0(out, src, n, s)` (`0x009283f0`,
    /// from `rcbgPSimCreate`; name ours): `n` (signed) points of three
    /// `i16`s at `src` become vectors at `out` (16 bytes each): each point
    /// sign-extended to three words, converted to floats
    /// (`rcVu0IntToFloat`) and scaled by `s` (`rcbgModelScaleVector`,
    /// whose `eax`, the w word, it returns). `work` is the original's two
    /// stack vectors (0x20 bytes: the floats, then the integers). The
    /// original never writes the integers' w word, so the vectors' w is
    /// whatever the stack held; the port writes 0 there (the tests zero
    /// the original's stack). With no points `eax` is the caller's.
    pub fn rcbg_model_shorts_to_vectors<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [out, src, n, s]: [u32; 4],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [to_float, scale] = SHORTS_TO_VECTORS_CALLS.map(|f| mem.rebase(f));
        let (floats, ints) = (work, work.wrapping_add(0x10));
        let mut eax = 0;
        for i in 0..n.cast_signed().max(0).cast_unsigned() {
            let point = src.wrapping_add(6 * i);
            for k in 0..3u32 {
                let v = i32::from(mem.read::<i16>(point.wrapping_add(2 * k)));
                mem.write(ints.wrapping_add(4 * k), v);
            }
            mem.write(ints.wrapping_add(0xc), 0u32);
            call(mem, to_float, &[floats, ints]);
            let s = crate::x87::copy(s);
            eax = call(mem, scale, &[out.wrapping_add(16 * i), floats, s]);
        }
        eax
    }

    /// `rcbgScaleVector_009294a0(out, v, s)` and `rcVu0Add_00929440(out,
    /// a, b)`: [`rcfx_gravity_calc`]'s helpers.
    pub const GRAVITY_CALLS: [u32; 2] = [0x0092_94a0, 0x0092_9440];

    /// `rcfxGravityCalc(gravity, sys)` (`0x00929500`, the gravity's `+0x14`
    /// callback, set by `rcfxGravityInit`; name ours): each of the system's
    /// `+0x1c` particles' forces (`+0x30`, 16 bytes each) gains the
    /// gravity vector (`+0x1c`) times the mass (`sys +0x20`, copied
    /// through the x87 once). `work` is the original's stack vector.
    /// Returns the last add's `eax`, or `sys` with no particles.
    pub fn rcfx_gravity_calc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        gravity: u32,
        sys: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [scale, add] = GRAVITY_CALLS.map(|f| mem.rebase(f));
        let mass = crate::x87::copy(mem.read::<u32>(sys.wrapping_add(0x20)));
        let forces = mem.read::<u32>(sys.wrapping_add(0x30));
        let count = mem.read::<i32>(sys.wrapping_add(0x1c));
        let mut eax = sys;
        for i in 0..count.max(0).cast_unsigned() {
            let force = forces.wrapping_add(16 * i);
            call(mem, scale, &[work, gravity.wrapping_add(0x1c), mass]);
            eax = call(mem, add, &[force, force, work]);
        }
        eax
    }

    /// `rcbgScaleVector_009292a0(out, v, s)` and `rcVu0Sub_00929300(out,
    /// a, b)`: [`rcfx_damp_calc`]'s helpers.
    pub const DAMP_CALLS: [u32; 2] = [0x0092_92a0, 0x0092_9300];

    /// `rcfxDampCalc(damp, sys)` (`0x00929360`, the damping's `+0x14`
    /// callback, set by `rcfxDampInit`; name ours): each of the system's
    /// `+0x1c` particles' forces (`+0x30`) loses its velocity (`+0x2c`)
    /// times the damping (`+0x1c`, read for each particle). `work` is the
    /// original's stack vector. Returns the last subtraction's `eax`, or
    /// `sys` with no particles.
    pub fn rcfx_damp_calc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        damp: u32,
        sys: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [scale, sub] = DAMP_CALLS.map(|f| mem.rebase(f));
        let velocities = mem.read::<u32>(sys.wrapping_add(0x2c));
        let forces = mem.read::<u32>(sys.wrapping_add(0x30));
        let count = mem.read::<i32>(sys.wrapping_add(0x1c));
        let mut eax = sys;
        for i in 0..count.max(0).cast_unsigned() {
            let k = crate::x87::copy(mem.read::<u32>(damp.wrapping_add(0x1c)));
            let force = forces.wrapping_add(16 * i);
            call(mem, scale, &[work, velocities.wrapping_add(16 * i), k]);
            eax = call(mem, sub, &[force, force, work]);
        }
        eax
    }

    /// An `f64` through `fld m64` / `fstp m64`: a signalling NaN comes out
    /// quiet.
    fn copy_f64(x: f64) -> f64 {
        if x.is_nan() {
            f64::from_bits(x.to_bits() | 0x0008_0000_0000_0000)
        } else {
            x
        }
    }

    /// `rcfxParticleCalc(sys)` (`0x00928710`), [`rcfx_particle_set_time`]'s
    /// step.
    pub const PARTICLE_CALC: u32 = 0x0092_8710;

    /// `rcfxParticleSetTime(sys, t)` (`0x00928a60`): the target time `t`
    /// (`f64`) goes to `+0x10`; while the system's time (`+0`) is before
    /// it, the system steps (`rcfxParticleCalc`) and its time advances by
    /// the step (`+8`, re-read after each step: the step stores it as an
    /// `f32`). The original's `eax` is the last `fnstsw`'s status word
    /// over a caller's high half, which no caller reads.
    pub fn rcfx_particle_set_time<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        sys: u32,
        t: f64,
        call: &mut C,
    ) {
        use ffx_bridge::x87;
        let t = copy_f64(t);
        mem.write(sys.wrapping_add(0x10), t);
        let particle_calc = mem.rebase(PARTICLE_CALC);
        while mem.read::<f64>(sys) < t {
            call(mem, particle_calc, &[sys]);
            let step = mem.read::<f64>(sys.wrapping_add(8));
            let time = x87::add(step, mem.read::<f64>(sys));
            mem.write(sys, time);
        }
    }

    /// `rcfxParticleSetTime(sys, t)` (`0x00928a60`) and `rcbgModelFToS0(
    /// out, points, n, scale)` (`0x009281d0`): [`rcbg_psim_proc`]'s calls.
    pub const PSIM_PROC_CALLS: [u32; 2] = [0x0092_8a60, 0x0092_81d0];
    /// 0.5 (`f64`, `.rdata` `0x00b92220`): the time a running simulation
    /// advances each frame.
    pub const PSIM_TIME_STEP: u32 = 0x00b9_2220;

    /// `rcbgPSimProc(p)` (`0x0091fe70`, the particle simulation's process
    /// slot): while running (bit 0 of `+0x10`) its particle system
    /// (`+0x18`) is stepped to the time `+0xa8` (`f64`,
    /// `rcfxParticleSetTime`), which then advances by 0.5; the particles'
    /// positions (the system's `+0x24`) go back into its model's (`+0x14`)
    /// points as `i16` triples (`rcbgModelFToS0(blob + blob[+8], positions,
    /// count, scale)`, the model's blob `+0x14`, its count the `i16` at
    /// `+0x12`, the scale `rcbgModelGetScale(model)` stored as an `f32`).
    /// Then the model's update slot (`+8`), when set, runs on it (a tail
    /// jump), whose `eax` it returns; else the model.
    pub fn rcbg_psim_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87;
        let [set_time, f_to_s0] = PSIM_PROC_CALLS.map(|f| mem.rebase(f));
        if mem.read::<u8>(p.wrapping_add(0x10)) & 1 != 0 {
            let t = copy_f64(mem.read::<f64>(p.wrapping_add(0xa8))).to_bits();
            let sys = mem.read::<u32>(p.wrapping_add(0x18));
            #[allow(clippy::cast_possible_truncation)] // the two halves
            call(mem, set_time, &[sys, t as u32, (t >> 32) as u32]);
            let step = mem.read::<f64>(mem.rebase(PSIM_TIME_STEP));
            let t = x87::add(mem.read::<f64>(p.wrapping_add(0xa8)), step);
            mem.write(p.wrapping_add(0xa8), t);
        }
        let model = mem.read::<u32>(p.wrapping_add(0x14));
        let scale = rcbg_model_get_scale(mem, model).to_bits();
        let blob = mem.read::<u32>(model.wrapping_add(0x14));
        let n = i32::from(mem.read::<i16>(blob.wrapping_add(0x12))).cast_unsigned();
        let sys = mem.read::<u32>(p.wrapping_add(0x18));
        let positions = mem.read::<u32>(sys.wrapping_add(0x24));
        let out = blob.wrapping_add(mem.read::<u32>(blob.wrapping_add(8)));
        call(mem, f_to_s0, &[out, positions, n, scale]);
        let model = mem.read::<u32>(p.wrapping_add(0x14));
        let update = mem.read::<u32>(model.wrapping_add(8));
        if update == 0 {
            return model;
        }
        call(mem, update, &[model])
    }

    /// [`rcbg_psim_create`]'s calls: `rcfxParticleNew()`,
    /// `rcfxParticleSetCnt(sys, n)`, `rcbgModelShortsToVectors_009283f0(out,
    /// src, n, s)`, `rcfxSpringNew()`, `rcbgModelGetPrimHeader(model, i)`,
    /// `rcAlloc(size, file, line)`, `rcfxSpringSetCnt(spring, n)`,
    /// `rcVu0Sub(out, a, b)`, `rcFree(p)`,
    /// `rcfxParticleAddForce(sys, force)` and `rcfxParticleResetForce(sys)`.
    pub const PSIM_CREATE_CALLS: [u32; 11] = [
        0x0092_8950,
        0x0092_89f0,
        0x0092_83f0,
        0x0092_8d50,
        0x0092_82c0,
        0x0092_58a0,
        0x0092_8d80,
        0x0091_f740,
        0x0092_58e0,
        0x0092_8660,
        0x0092_8970,
    ];
    /// `rcVec3fLength(v)` (`0x00921250`, in `ST0`).
    pub const VEC3F_LENGTH: u32 = 0x0092_1250;
    /// `"..\\program\\src\\rcBg\\rcbgPSim.c"` (`0x00b676f0`), the
    /// allocations' file.
    pub const PSIM_FILE: u32 = 0x00b6_76f0;
    /// The bytes per element of each primitive type 0..=7 (the original's
    /// eight stack bytes `0x28201c14`, `0x30282018`): types below 4 are
    /// triangles, the others quads.
    pub const PRIM_STRIDES: [u8; 8] = [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];

    /// A primitive type's element size: [`PRIM_STRIDES`]; a type outside
    /// 0..=7 reads the original's other stack bytes, here 0.
    fn prim_stride(kind: i8) -> u32 {
        usize::try_from(kind)
            .ok()
            .and_then(|k| PRIM_STRIDES.get(k))
            .map_or(0, |&b| u32::from(b))
    }

    /// [`rcbg_psim_create`]'s edge list: a first pass over the model's
    /// primitives counts the edges (3 per triangle, 4 per quad), a second
    /// writes them (two words each) into an `rcAlloc`ed list (rcbgPSim.c
    /// line 0x97). Returns the list and the count.
    fn psim_edges<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        model: u32,
        [prim_header, alloc]: [u32; 2],
        call: &mut C,
    ) -> (u32, i32) {
        let file = mem.rebase(PSIM_FILE);
        // first pass: the number of edges
        let kind_at = |mem: &M, h: u32| mem.read::<i8>(h.wrapping_add(1));
        let count_at = |mem: &M, h: u32| i32::from(mem.read::<i16>(h.wrapping_add(2)));
        let mut edges = 0i32;
        let mut h = call(mem, prim_header, &[model, 0]);
        while kind_at(mem, h) != -1 {
            let kind = kind_at(mem, h);
            let n = count_at(mem, h);
            edges = edges.wrapping_add(n.wrapping_mul(if kind < 4 { 3 } else { 4 }));
            h = h
                .wrapping_add(0x10)
                .wrapping_add(prim_stride(kind).wrapping_mul(n.cast_unsigned()));
        }
        let edge_count = edges.cast_unsigned();

        // second pass: the edges, two words each
        let list = call(mem, alloc, &[edge_count.wrapping_mul(8), file, 0x97]);
        let mut out = list;
        let mut h = call(mem, prim_header, &[model, 0]);
        while kind_at(mem, h) != -1 {
            let kind = kind_at(mem, h);
            let mut e = h.wrapping_add(0x10);
            let mut i = 0i32;
            while i < count_at(mem, h) {
                let index = |mem: &M, o: u32| u32::from(mem.read::<u16>(e.wrapping_add(o)));
                let ends: &[u32] = if kind < 4 {
                    &[0xc, 0xe, 0xe, 0x10, 0x10, 0xc]
                } else {
                    &[0x10, 0x12, 0x12, 0x16, 0x16, 0x14, 0x14, 0x10]
                };
                for &o in ends {
                    mem.write(out, index(mem, o));
                    out = out.wrapping_add(4);
                }
                i += 1;
                e = e.wrapping_add(prim_stride(kind_at(mem, h)));
            }
            h = e;
        }

        (list, edges)
    }

    /// [`rcbg_psim_create`]'s marks: a zeroed `rcAlloc`ed word per edge
    /// (line 0xc6), 1 for an edge equal to an earlier one either way round.
    fn psim_mark_repeats<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        list: u32,
        edges: i32,
        alloc: u32,
        call: &mut C,
    ) -> u32 {
        let file = mem.rebase(PSIM_FILE);
        let edge_count = edges.cast_unsigned();
        let read = |mem: &M, at: u32| mem.read::<u32>(at);
        // an edge equal to an earlier one, either way round, is marked
        let marks = call(mem, alloc, &[edge_count.wrapping_mul(4), file, 0xc6]);
        for k in 0..edge_count.cast_signed().max(0).cast_unsigned() {
            mem.write(marks.wrapping_add(4 * k), 0u32);
        }
        let end = |mem: &M, k: u32, which: u32| read(mem, list.wrapping_add(8 * k + 4 * which));
        for i in 0..(edges - 1).max(0).cast_unsigned() {
            let (a, b) = (end(mem, i, 0), end(mem, i, 1));
            for j in i + 1..edges.cast_unsigned() {
                let (c, d) = (end(mem, j, 0), end(mem, j, 1));
                if (c == a || c == b) && (d == a || d == b) {
                    mem.write(marks.wrapping_add(4 * j), 1u32);
                    break;
                }
            }
        }
        marks
    }

    /// `rcbgPSimCreate(p, model)` (`0x0091f9d0`): the particle simulation of
    /// a cloth-like model.
    /// - `model` goes to `+0x14`; a new particle system
    ///   (`rcfxParticleNew`) to `+0x18`, with as many particles as the
    ///   model's points (the `i16` at blob `+0x12`, the blob the model's
    ///   `+0x14`), placed at the points (`rcbgModelShortsToVectors_009283f0`
    ///   into the system's positions `+0x24`, scaled by
    ///   `rcbgModelGetScale(model)` stored as an `f32`).
    /// - A spring force (`rcfxSpringNew`) gets one spring per distinct
    ///   edge of the model's primitives (`rcbgModelGetPrimHeader(model, 0)`:
    ///   headers of 0x10 bytes, type `+1` (-1 ends), count `+2` (`i16`),
    ///   then the elements, [`PRIM_STRIDES`] bytes each). Triangles (types
    ///   below 4) give the edges (a, b), (b, c), (c, a) of their `u16`
    ///   indices at `+0xc`, `+0xe`, `+0x10`; quads (a, b), (b, d), (d, c),
    ///   (c, a) of `+0x10`, `+0x12`, `+0x14`, `+0x16`. The edges go to an
    ///   `rcAlloc`ed list (rcbgPSim.c line 0x97; the count from a first
    ///   pass: 3 or 4 per element), an edge equal to an earlier one either
    ///   way round is marked (a zeroed `rcAlloc`ed word each, line 0xc6),
    ///   and each unmarked edge becomes a spring (`rcfxSpringSetCnt`, the
    ///   pairs at `get_ptr32_func(+0x20)` (the identity), the rest lengths at
    ///   `get_ptr32_func(+0x24)`: `rcVec3fLength` of position a minus
    ///   position b (`rcVu0Sub` into `work`, 16 bytes), stored as `f32`).
    /// - Both lists are freed (`rcFree`, when allocated), the spring added
    ///   to the system (`rcfxParticleAddForce`) and the forces reset
    ///   (`rcfxParticleResetForce`), whose `eax` it returns.
    pub fn rcbg_psim_create<M, C, F>(
        mem: &mut M,
        [p, model]: [u32; 2],
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        let [
            particle_new,
            particle_set_cnt,
            shorts_to_vectors,
            spring_new,
            prim_header,
            alloc,
            spring_set_cnt,
            vu0_sub,
            free,
            add_force,
            reset_force,
        ] = PSIM_CREATE_CALLS.map(|f| mem.rebase(f));
        let read = |mem: &M, at: u32| mem.read::<u32>(at);

        mem.write(p.wrapping_add(0x14), model);
        let sys = call(mem, particle_new, &[]);
        mem.write(p.wrapping_add(0x18), sys);
        let blob = read(mem, model.wrapping_add(0x14));
        let points = i32::from(mem.read::<i16>(blob.wrapping_add(0x12))).cast_unsigned();
        call(mem, particle_set_cnt, &[sys, points]);
        let positions = read(mem, read(mem, p.wrapping_add(0x18)).wrapping_add(0x24));
        let scale = rcbg_model_get_scale(mem, model).to_bits();
        // the count read again from the same blob, the source from the
        // model's blob as it is now
        let points = i32::from(mem.read::<i16>(blob.wrapping_add(0x12))).cast_unsigned();
        let blob = read(mem, model.wrapping_add(0x14));
        let src = blob.wrapping_add(read(mem, blob.wrapping_add(8)));
        call(mem, shorts_to_vectors, &[positions, src, points, scale]);
        let spring = call(mem, spring_new, &[]);

        let (list, edges) = psim_edges(mem, model, [prim_header, alloc], call);
        let marks = psim_mark_repeats(mem, list, edges, alloc, call);
        let end = |mem: &M, k: u32, which: u32| read(mem, list.wrapping_add(8 * k + 4 * which));
        let unmarked = |mem: &M, k: u32| read(mem, marks.wrapping_add(4 * k)) == 0;
        let all = 0..edges.max(0).cast_unsigned();
        let springs = all.clone().filter(|&k| unmarked(mem, k)).count();
        call(
            mem,
            spring_set_cnt,
            &[spring, u32::try_from(springs).unwrap_or(0)],
        );

        // one spring per unmarked edge, its rest length the current one
        let mut pairs = ptr32(read(mem, spring.wrapping_add(0x20)));
        let mut lengths = ptr32(read(mem, spring.wrapping_add(0x24)));
        let positions = read(mem, read(mem, p.wrapping_add(0x18)).wrapping_add(0x24));
        let length = mem.rebase(VEC3F_LENGTH);
        for k in all {
            if !unmarked(mem, k) {
                continue;
            }
            let (a, b) = (end(mem, k, 0), end(mem, k, 1));
            mem.write(pairs.wrapping_add(4), b);
            mem.write(pairs, a);
            let at = |i: u32| positions.wrapping_add(i << 4);
            call(mem, vu0_sub, &[work, at(a), at(b)]);
            let len = fcall(mem, length, crate::camera::FArgs::Stack(&[work]));
            pairs = pairs.wrapping_add(8);
            mem.write(lengths, ffx_bridge::x87::store(len));
            lengths = lengths.wrapping_add(4);
        }
        if marks != 0 {
            call(mem, free, &[marks]);
        }
        if list != 0 {
            call(mem, free, &[list]);
        }
        let sys = read(mem, p.wrapping_add(0x18));
        call(mem, add_force, &[sys, spring]);
        let sys = read(mem, p.wrapping_add(0x18));
        call(mem, reset_force, &[sys])
    }

    /// [`rcbg_model_draw`]'s calls: `FFXVu0MulMatrix(out, a, b)`,
    /// `rcbgSetWorldMatrix(m)`, `rcbgUpdateMatrix()`,
    /// `rcbgModelScaleVector(out, v, s)`, `rcbgClipVertex(m, verts, n)`,
    /// `rcbgModelSetTexReg(page)`, `graphicVFXGetRenderDataByInstance(lo,
    /// hi)`, `updateGuildMapRenderData(lo, hi, id, data)`,
    /// `graphicGuidemapUpdate(lo, hi)`, `InitGuildMapRenderData(lo, hi, id,
    /// data)` and `rcbgRestoreReg()`.
    pub const MODEL_DRAW_CALLS: [u32; 11] = [
        0x006e_d5c0,
        0x0092_b140,
        0x0092_b160,
        0x0092_7b00,
        0x0092_af60,
        0x0092_84d0,
        0x0064_53d0,
        0x0071_7620,
        0x0064_14f0,
        0x0071_0ed0,
        0x0092_0250,
    ];
    /// `g_RcbgClipWorldMatrix` (`0x025d58a0`).
    pub const CLIP_WORLD_MATRIX: u32 = 0x025d_58a0;

    /// `rcbgModelDraw(model)` (`0x00928020`, the model's draw slot).
    /// - Unless bit 2 of `+0x10`: the world matrix is the model's `+0x100`
    ///   times `+0xc0` (into `work`, the original's stack matrix),
    ///   `rcbgSetWorldMatrix`, `rcbgUpdateMatrix`.
    /// - With bit 0 (a bounded model): the box corners `+0x90` and `+0xa0`
    ///   over the scale (`1 / rcbgModelGetScale`, stored as `f32`;
    ///   `rcbgModelScaleVector`) make four vertices at `work + 0x40` (a, b,
    ///   (a.x, a.y, b.z), (b.x, b.y, a.z), w 1.0); when `rcbgClipVertex(
    ///   g_RcbgClipWorldMatrix, vertices, 4)` clips them the model is not
    ///   drawn and its `eax` returned.
    /// - The texture register page is 1 with bit 1, else 2
    ///   (`rcbgModelSetTexReg`); the renderer's guide map data of instance
    ///   (0, `+0x14`) is updated (`updateGuildMapRenderData`,
    ///   `graphicGuidemapUpdate`) when it exists, else created
    ///   (`InitGuildMapRenderData`), both with the model's `+0x20`; then
    ///   `rcbgRestoreReg`, whose `eax` it returns.
    pub fn rcbg_model_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, store};
        let [
            mul_matrix,
            set_world,
            update_matrix,
            scale_vector,
            clip,
            set_tex_reg,
            render_data,
            update_guide,
            guidemap_update,
            init_guide,
            restore_reg,
        ] = MODEL_DRAW_CALLS.map(|f| mem.rebase(f));
        let flags = |mem: &M| mem.read::<u8>(m.wrapping_add(0x10));
        if flags(mem) & 4 == 0 {
            call(
                mem,
                mul_matrix,
                &[work, m.wrapping_add(0x100), m.wrapping_add(0xc0)],
            );
            call(mem, set_world, &[work]);
            call(mem, update_matrix, &[]);
        }
        if flags(mem) & 1 != 0 {
            let s = load(rcbg_model_get_scale(mem, m).to_bits());
            let inv = store(div(1.0, s));
            let [a, b] = [work.wrapping_add(0x40), work.wrapping_add(0x50)];
            call(mem, scale_vector, &[a, m.wrapping_add(0x90), inv]);
            call(mem, scale_vector, &[b, m.wrapping_add(0xa0), inv]);
            let one = 1f32.to_bits();
            let corner =
                |mem: &M, v: u32, k: u32| crate::x87::copy(mem.read::<u32>(v.wrapping_add(4 * k)));
            mem.write(b.wrapping_add(0xc), one);
            mem.write(a.wrapping_add(0xc), one);
            let c = [corner(mem, a, 0), corner(mem, a, 1), corner(mem, b, 2), one];
            mem.write(work.wrapping_add(0x60), c);
            let d = [corner(mem, b, 0), corner(mem, b, 1), corner(mem, a, 2), one];
            mem.write(work.wrapping_add(0x70), d);
            let matrix = mem.rebase(CLIP_WORLD_MATRIX);
            let r = call(mem, clip, &[matrix, a, 4]);
            if r != 0 {
                return r;
            }
        }
        let page = if flags(mem) & 2 == 0 { 2 } else { 1 };
        call(mem, set_tex_reg, &[page]);
        let id = |mem: &M| mem.read::<u32>(m.wrapping_add(0x14));
        let id0 = id(mem);
        let rd = call(mem, render_data, &[0, id0]);
        let id1 = id(mem);
        let data = m.wrapping_add(0x20);
        if rd == 0 {
            call(mem, init_guide, &[0, id1, id1, data]);
        } else {
            call(mem, update_guide, &[0, id1, id1, data]);
            let id2 = id(mem);
            call(mem, guidemap_update, &[0, id2]);
        }
        call(mem, restore_reg, &[])
    }

    /// `rcbgModelNew()` (`0x009283b0`), `rcbgModelCreateFromOMD(model, omd)`
    /// (`0x00927d70`), `rcbgModelSetScale(model, s)` (`0x00928540`) and
    /// `rcbgPSimCreate(p, model)` (`0x0091f9d0`):
    /// [`rcbg_psim_create_from_omd`]'s calls.
    pub const PSIM_FROM_OMD_CALLS: [u32; 4] = [0x0092_83b0, 0x0092_7d70, 0x0092_8540, 0x0091_f9d0];

    /// `rcbgPSimCreateFromOMD(p, omd, scale)` (`0x0091fd00`): a new model
    /// (`rcbgModelNew`) made from `omd` (`rcbgModelCreateFromOMD`), scaled
    /// by `scale` (an `f32`, copied through the x87; `rcbgModelSetScale`),
    /// becomes `p`'s simulation (`rcbgPSimCreate(p, model)`), whose `eax`
    /// it returns.
    pub fn rcbg_psim_create_from_omd<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [p, omd, scale]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let [new, from_omd, set_scale, create] = PSIM_FROM_OMD_CALLS.map(|f| mem.rebase(f));
        let model = call(mem, new, &[]);
        call(mem, from_omd, &[model, omd]);
        call(mem, set_scale, &[model, crate::x87::copy(scale)]);
        call(mem, create, &[p, model])
    }

    /// `rcbgFPm173ProcKieru(part)` (`0x00922dc0`, from `rcbgFPm173Proc000`'s
    /// jump table): from frame `+0x1c` 0x4b on, the effect 0x61 stuck to
    /// the character `+0x10` moves to `(0, B.y - A.y * 0.1, 0, 0)`
    /// (`pppFpSetStickPos(0x61, v)`; 0.1 the `f64` at `0x00b921f0`), with
    /// `A` = `Ch_ElemGetPos(chr, 0, 0, &A)` and `B` = `Ch_OptposGet(chr, 6,
    /// &B)` fetched on frame 0x4b only, which also sticks the effect
    /// (`pppFpSetStickChr(0x61, chr, 0)`) and restarts it (`Fp_Restart`).
    /// Frame 0x96 clears `+0x20`. `frame` is the original's 0x30-byte
    /// local area (`A` at `+0`, `B` at `+0x10`, `v` at `+0x20`).
    ///
    /// Junk reads: on frames after 0x4b (and when a callee writes nothing)
    /// `A.y` (`[ebp-0x30]`) and `B.y` (`[ebp-0x20]`) are never written in
    /// the original, which uses stale stack words; the port uses 0 (brief
    /// 2026-10-24).
    pub fn rcbg_fpm173_proc_kieru<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        frame: u32,
        call: &mut C,
    ) {
        let t = mem.read::<i32>(p.wrapping_add(0x1c));
        if t < 0x4b {
            return;
        }
        let (a, b, v) = (frame, frame.wrapping_add(0x10), frame.wrapping_add(0x20));
        // junk read: [ebp-0x30], [ebp-0x20] never written after frame 0x4b; 0 (brief 2026-10-24)
        mem.write(a, [0_u32; 4]);
        mem.write(b, [0_u32; 4]);
        if t == 0x4b {
            let chr = mem.read::<u32>(p.wrapping_add(0x10));
            call(mem, mem.rebase(0x0083_3d10), &[chr, 0, 0, a]);
            let chr = mem.read::<u32>(p.wrapping_add(0x10));
            call(mem, mem.rebase(0x0083_54f0), &[chr, 6, b]);
            let chr = mem.read::<u32>(p.wrapping_add(0x10));
            call(mem, mem.rebase(0x0072_a530), &[0x61, chr, 0]);
            call(mem, mem.rebase(0x0072_9700), &[0x61]);
        }
        let k = Reg::constant(mem.read::<f64>(mem.rebase(0x00b9_21f0)));
        let by = Reg::load(mem.read::<f32>(b.wrapping_add(4)));
        let ay = Reg::load(mem.read::<f32>(a.wrapping_add(4)));
        let y = (by - ay * k).store();
        mem.write(v, [0_u32, y.to_bits(), 0, 0]);
        call(mem, mem.rebase(0x0072_a630), &[0x61, v]);
        if mem.read::<u32>(p.wrapping_add(0x1c)) == 0x96 {
            mem.write(p.wrapping_add(0x20), 0_u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_sizes_are_signed() {
        assert_eq!(packet_size(0x14, 3), 0x10 + 0x3c);
        assert_eq!(packet_size(0x14, -1), 0x10u32.wrapping_sub(0x14));
    }

    #[test]
    fn halves_round_like_the_original() {
        assert_eq!(f32::from_bits(half(640)), 320.0);
        assert_eq!(f32::from_bits(half(16_777_217)), 8_388_608.0);
    }
}
