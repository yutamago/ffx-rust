//! `rcef`: the field effects' tasks and objects (`docs/formats/field.md`).
//!
//! A task runs a stack of sequence functions: each frame
//! `rcefTaskProc` calls the one on top (again while it sets the
//! re-run flag), then the task's after-function. A sequence calls a
//! sub-sequence (`rcefTaskCallSeq`), replaces itself (`rcefTaskSetSeq`) or
//! returns to its caller (`rcefTaskRetSeqCont`). Each stack level has 16
//! bytes of arguments (the wait counter of `rcefTaskWait`).
//!
//! A controller holds the tasks in a doubly linked list between a head
//! (priority 0) and a tail (priority `0xffff`), sorted by priority. An
//! effect object is a task with a transform, an attached character, a
//! particle part and an animation.

/// Offsets in a task (`rcefTaskInit`, `0xbc` bytes).
pub mod header {
    /// `+0x0`: the flags ([`super::ALIVE`], [`super::RERUN`], [`super::DEAD`]).
    pub const FLAGS: u32 = 0x0;
    /// `+0x4`: the previous task.
    pub const PREV: u32 = 0x4;
    /// `+0x8`: the next task.
    pub const NEXT: u32 = 0x8;
    /// `+0xc`: the sequence functions, one per stack level.
    pub const SEQ: u32 = 0xc;
    /// `+0x2c`: 16 bytes of arguments per stack level.
    pub const ARGS: u32 = 0x2c;
    /// Bytes of arguments per level.
    pub const ARGS_SIZE: u32 = 0x10;
    /// `+0xac`: the stack level (`i32`, `0..=MAX_LEVEL`).
    pub const LEVEL: u32 = 0xac;
    /// `+0xb0`: called after the sequences each frame unless the task is
    /// dead (`rcefObjProc` for objects).
    pub const AFTER: u32 = 0xb0;
    /// `+0xb4`: called when the task is killed (`rcefTaskKill`).
    pub const DESTRUCTOR: u32 = 0xb4;
    /// `+0xb8`: the priority (`i32`): the list is sorted by it.
    pub const PRIORITY: u32 = 0xb8;
    /// Size.
    pub const SIZE: u32 = 0xbc;
}

/// Flag bit 0: set by `rcefTaskInit`.
pub const ALIVE: u32 = 1;
/// Flag bit 1: run the sequence again this frame (`rcefTaskProc`).
pub const RERUN: u32 = 2;
/// Flag bit 2: kill the task after this frame (`rcefTaskCtrlProc`); also
/// skips its after-function.
pub const DEAD: u32 = 4;
/// The deepest stack level (`rcefTaskCallSeq`): eight levels.
pub const MAX_LEVEL: i32 = 7;
/// A new task's priority.
pub const DEFAULT_PRIORITY: u32 = 0x8000;
/// The original's empty destructor, a new task's (`0x0092ed00`).
pub const NO_DESTRUCTOR: u32 = 0x0092_ed00;
/// `rcefTaskWaitSeq` (`0x0092ee90`), the sequence `rcefTaskWait` calls.
pub const WAIT_SEQ: u32 = 0x0092_ee90;
/// `pppMemFree(heap, spare, p)` (`0x00716a90`).
pub const PPP_MEM_FREE: u32 = 0x0071_6a90;
/// `pppMemAlloc(heap, spare, size)` (`0x00716a30`).
pub const PPP_MEM_ALLOC: u32 = 0x0071_6a30;
/// `pppRunPart(part, arg)` (`0x00712250`).
pub const PPP_RUN_PART: u32 = 0x0071_2250;
/// `Ch_TextureSetClutBase(chr, a, b)` (`0x008313c0`).
pub const CH_TEXTURE_SET_CLUT_BASE: u32 = 0x0083_13c0;
/// `Ch_TextureSetImageBase(chr, a, b)` (`0x00831390`).
pub const CH_TEXTURE_SET_IMAGE_BASE: u32 = 0x0083_1390;
/// `Ch_DrawEnEffect(chr, on)` (`0x0082eb60`).
pub const CH_DRAW_EN_EFFECT: u32 = 0x0082_eb60;
/// `Sg_LightGetColorMatrixPtr()` (`0x008361b0`).
pub const LIGHT_COLOR_MATRIX: u32 = 0x0083_61b0;
/// `Sg_LightGetNormalMatrixPtr()` (`0x00836240`).
pub const LIGHT_NORMAL_MATRIX: u32 = 0x0083_6240;

/// Offsets in a controller (`rcefTaskCtrlInit`).
pub mod ctrl {
    /// `+0x0`: the head task (priority 0).
    pub const HEAD: u32 = 0x0;
    /// `+0xbc`: the tail task (priority `0xffff`).
    pub const TAIL: u32 = super::header::SIZE;
    /// The tail's priority.
    pub const TAIL_PRIORITY: u32 = 0xffff;
    /// `+0x178`: cleared by `rcefTaskCtrlInit`.
    pub const FIELD_178: u32 = 0x178;
}

/// Offsets in an effect object (`rcefObjInit`, `0x490` bytes).
pub mod obj {
    /// `+0xbc`: the mode flags: bit 1 attached to a character
    /// (`rcefObjAttachChr`, which clears bit 2), bits 4..=11 the mode
    /// (`rcefObjSetMode`).
    pub const MODE: u32 = 0xbc;
    /// Mode bit 1: attached to a character.
    pub const ATTACHED: u32 = 0x2;
    /// Mode bit 2: cleared by `rcefObjAttachChr`.
    pub const DETACHED_BIT: u32 = 0x4;
    /// Bits `rcefObjClearMode` clears.
    pub const MODE_MASK: u32 = 0x0000_0ff0;
    /// `+0x270`: the attached character.
    pub const CHR: u32 = 0x270;
    /// `+0x274`: the element (bone) of the character.
    pub const CHR_ELEMENT: u32 = 0x274;
    /// `+0x280`: the particle part (`rcefParInit`, `0xb0` bytes); its first
    /// byte's bit 0 while it runs.
    pub const PAR: u32 = 0x280;
    /// `+0x330`: the animation (`rcefAnimeInit`, `0x90` bytes); its first
    /// byte's bit 0 while it runs.
    pub const ANIME: u32 = 0x330;
    /// `+0x3c0`: the model (`rcefModelInit`, `0xd0` bytes).
    pub const MODEL: u32 = 0x3c0;
    /// Size.
    pub const SIZE: u32 = 0x490;
    /// `rcefObjFini`, its destructor.
    pub const FINI: u32 = 0x0092_fee0;
    /// `rcefObjProc`, its after-function.
    pub const PROC: u32 = 0x0093_0060;
    /// The floats `rcefObjInit` sets: scales and colours 1.0 (`+0x13c`,
    /// `+0x16c` 0.0, which clearing already did).
    pub const INIT: [(u32, f32); 12] = [
        (0x130, 1.0),
        (0x134, 1.0),
        (0x138, 1.0),
        (0x13c, 0.0),
        (0x16c, 0.0),
        (0x160, 1.0),
        (0x164, 1.0),
        (0x168, 1.0),
        (0x170, 1.0),
        (0x174, 1.0),
        (0x178, 1.0),
        (0x17c, 1.0),
    ];
    /// Its three matrices (identity after `rcefObjInit`).
    pub const MATRICES: [u32; 3] = [0x1b0, 0x1f0, 0x230];
}

/// The particle parts `rcefParCreate` remembers for entry 2, by magic
/// effect id (`0x0193c0d0`..).
pub const PAR_KEEP_2: [(u32, u32); 4] = [
    (0x20e, 0x0193_c0d0),
    (0x180, 0x0193_c0d8),
    (0x168, 0x0193_c0e0),
    (0x217, 0x0193_c0e8),
];
/// The same for entry 1.
pub const PAR_KEEP_1: [(u32, u32); 2] = [(0xbf, 0x0193_c0dc), (0x216, 0x0193_c0e4)];

/// Offsets in a particle part (`rcefParInit`).
pub mod par {
    /// Size.
    pub const SIZE: u32 = 0xb0;
    /// The words `rcefParInit` sets after clearing it: four to 1.0, one to
    /// the integer 1, one to 0.0 (which clearing already did).
    pub const INIT: [(u32, u32); 6] = [
        (0x5c, 0x3f80_0000),
        (0x9c, 0x3f80_0000),
        (0x6c, 1),
        (0xa0, 0x3f80_0000),
        (0xa4, 0x3f80_0000),
        (0xa8, 0),
    ];
}

/// Offsets in an animation (`rcefAnimeInit`).
pub mod anime {
    /// Size.
    pub const SIZE: u32 = 0x90;
    /// `+0x0`: 1 while it runs (`rcefAnimeStart`).
    pub const RUNNING: u32 = 0x0;
    /// `+0x4`: the animation data (`rcefAnimeStart`).
    pub const DATA: u32 = 0x4;
    /// `+0x20`: a pointer to [`WORK`].
    pub const CURSOR: u32 = 0x20;
    /// `+0x44`: `u16`, `0x200` after `rcefAnimeInit`.
    pub const RATE: u32 = 0x44;
    /// `+0x48`, `+0x4c`: the two outputs `rcefAnimeStart` is given (the
    /// object's matrices at `+0x1f0` and `+0x170` in `0x00925590`).
    pub const OUT: [u32; 2] = [0x48, 0x4c];
    /// `+0x50`: its work area.
    pub const WORK: u32 = 0x50;
}

/// `3π/2`, the original's `.rdata` double (`0x00b67e18`; the `f32`
/// nearest it, widened).
pub const THREE_HALF_PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b6_7e18);

/// π, the `.rdata` double `rcAngleDiff` compares with (`0x00b92288`).
pub const PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_2288);
/// 2π, the `.rdata` double it wraps with (`0x00b39378`).
pub const TWO_PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b3_9378);

/// The effect targets (`0x00924a20` fills them, up to 16): `0x24` bytes
/// each.
pub mod target {
    /// The table.
    pub const TABLE: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x025d_5c60);
    /// Bytes per entry.
    pub const SIZE: u32 = 0x24;
    /// `+0x0`: flags (`u16`): bit 0 reached (`0x009248c0`), bit 1 done.
    pub const FLAGS: u32 = 0x0;
    /// `+0x8`: horizontal distance to the target.
    pub const DIST: u32 = 0x8;
    /// `+0xc`: its direction, an angle.
    pub const ANGLE: u32 = 0xc;
    /// `+0x10`: its height difference.
    pub const HEIGHT: u32 = 0x10;
    /// The delay a reached target waits (`u16`, `rcefTargetSetDelay`).
    pub const DELAY: ffx_bridge::Global<u16> = ffx_bridge::Global::new(0x025d_5ea2);
    /// The targets' count (`i16`, at most 16; `rcefSetTargets`).
    pub const COUNT: ffx_bridge::Global<i16> = ffx_bridge::Global::new(0x025d_5ea0);
    /// The caster's facing as an angle (`f32`).
    pub const CASTER_ANGLE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5ea4);
    /// The targets' centre (a vector, summed then averaged).
    pub const CENTRE: ffx_bridge::Global<[u32; 4]> = ffx_bridge::Global::new(0x025d_5ea8);
    /// The effect's caster (a character).
    pub const CASTER: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5bb8);
    /// How many targets the effect lists.
    pub const SOURCE_COUNT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5bbc);
    /// The listed targets (characters, 16 words).
    pub const SOURCES: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x025d_5bc0);
    /// `rcefGetChrDir(chr)` (`0x0092dfd0`, in `ST0`).
    pub const GET_CHR_DIR: u32 = 0x0092_dfd0;
    /// `rcefDirToAngle(dir)` (`0x0092de30`, in `ST0`).
    pub const DIR_TO_ANGLE: u32 = 0x0092_de30;
    /// `rcefGetChrPos(i, e, out)` (`0x0092e2f0`).
    pub const GET_CHR_POS: u32 = 0x0092_e2f0;
    /// `rcefGetChrPosOpt(i, n, out)` (`0x0092e340`).
    pub const GET_CHR_POS_OPT: u32 = 0x0092_e340;
    /// `rcVu0Add_00924770(out, a, b)`.
    pub const VU0_ADD: u32 = 0x0092_4770;
    /// `rcVu0Sub_00924860(out, a, b)`.
    pub const VU0_SUB: u32 = 0x0092_4860;
    /// `rcVec3fLength(v)` (`0x00921250`, in `ST0`).
    pub const VEC3F_LENGTH: u32 = 0x0092_1250;
    /// `rcefVecToAngle(v)` (`0x0092ec20`, in `ST0`).
    pub const VEC_TO_ANGLE: u32 = 0x0092_ec20;
    /// `rcbgScaleVector_009247d0(out, v, s)`.
    pub const SCALE_VECTOR: u32 = 0x0092_47d0;
}

/// Offsets in the effect main object (`rcefMainInit`).
pub mod main {
    /// `+0x10`: its effect memory environment (the `ppp` allocator's).
    pub const ENV: u32 = 0x10;
    /// Where the current `ppp` environment pointer lives: `.data`
    /// `0x00c3a4cc` holds its address.
    pub const ENV_SLOT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c3_a4cc);
}

/// The stack level after `rcefTaskCallSeq`, or `None` when the stack is
/// full (nothing changes).
#[must_use]
pub fn call_level(level: i32) -> Option<i32> {
    (level < MAX_LEVEL).then(|| level + 1)
}

/// The stack level after a return (`rcefTaskRetSeqCont_Inner`): one less,
/// never below 0.
#[must_use]
pub fn ret_level(level: i32) -> i32 {
    if level > 0 { level - 1 } else { level }
}

/// Where a level's sequence function is.
#[must_use]
#[allow(clippy::cast_sign_loss)] // the original indexes with the raw level
pub fn seq_slot(t: u32, level: i32) -> u32 {
    t.wrapping_add(header::SEQ)
        .wrapping_add((level as u32).wrapping_mul(4))
}

/// Where a level's arguments are.
#[must_use]
#[allow(clippy::cast_sign_loss)] // as above
pub fn args_slot(t: u32, level: i32) -> u32 {
    t.wrapping_add(header::ARGS)
        .wrapping_add((level as u32).wrapping_mul(header::ARGS_SIZE))
}

/// `rcefTaskCtrlIsEnd(ctrl)` (`0x009302e0`).
pub const TASK_CTRL_IS_END: u32 = 0x0093_02e0;
/// `rcefMainCreateObj(main)` (`0x00925140`).
pub const MAIN_CREATE_OBJ: u32 = 0x0092_5140;
/// `rcefObjIsParEnd(obj)` (`0x00930010`).
pub const OBJ_IS_PAR_END: u32 = 0x0093_0010;
/// `rcefMainSetPar(main, obj, a, b)` (`0x009256d0`).
pub const MAIN_SET_PAR: u32 = 0x0092_56d0;
/// `rcefObjAttachChr(obj, chr, bone)` (`0x0092f970`).
pub const OBJ_ATTACH_CHR: u32 = 0x0092_f970;
/// `rcCopyQword_00924f90(dst, src)` (`0x00924f90`).
pub const COPY_QWORD: u32 = 0x0092_4f90;
/// `MsGetChrHandle(i)` (`0x00795080`).
pub const MS_GET_CHR_HANDLE: u32 = 0x0079_5080;
/// `AtelGetCharHandleFromBindPC(i)` (`0x0086ae90`).
pub const ATEL_CHAR_FROM_BIND_PC: u32 = 0x0086_ae90;
/// `Ch_OptposGetElement(chr, n, k)` (`0x00835660`).
pub const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
/// `Ch_OptposGet(chr, n, out)` (`0x008354f0`).
pub const CH_OPTPOS_GET: u32 = 0x0083_54f0;
/// `Ch_ElemGetMatrix(chr, e, out)` (`0x00833c30`).
pub const CH_ELEM_GET_MATRIX: u32 = 0x0083_3c30;
/// `Ch_SetOrgMatrix(chr, m)` (`0x0082b3e0`).
pub const CH_SET_ORG_MATRIX: u32 = 0x0082_b3e0;
/// `Ch_ClutCreate(chr)` (`0x0083c710`).
pub const CH_CLUT_CREATE: u32 = 0x0083_c710;
/// `Ch_ClutDispose(chr)` (`0x0083c810`).
pub const CH_CLUT_DISPOSE: u32 = 0x0083_c810;
/// `rcefScaleMatrix100(m)` (`0x0092e630`).
pub const SCALE_MATRIX_100: u32 = 0x0092_e630;
/// `rcCopyQword_0092d6f0(dst, src)` (`0x0092d6f0`).
pub const COPY_QWORD_D6F0: u32 = 0x0092_d6f0;
/// `rcbgScaleVector_0092d690(dst, v, s)` (`0x0092d690`).
pub const SCALE_VECTOR_D690: u32 = 0x0092_d690;
/// `rcVu0Sub_0092d720(dst, a, b)` (`0x0092d720`).
pub const VU0_SUB_D720: u32 = 0x0092_d720;
/// `FFXVu0ApplyMatrix(dst, m, v)` (`0x006ed2e0`).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `rcVec3fNormalize(v)` (`0x00921360`).
pub const VEC3F_NORMALIZE: u32 = 0x0092_1360;
/// `get_ptr32_func(p)` (`0x00a44650`).
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `set_ptr32(p)` (`0x00a44700`).
pub const SET_PTR32: u32 = 0x00a4_4700;
/// `pppStartPart(part, rate, data, set_ups)` (`0x007123c0`).
pub const PPP_START_PART: u32 = 0x0071_23c0;
/// `pppSetMatrix(part)` (`0x00717410`).
pub const PPP_SET_MATRIX: u32 = 0x0071_7410;
/// `pppAllFreePObject(part)` (`0x00711f20`).
pub const PPP_ALL_FREE_POBJECT: u32 = 0x0071_1f20;
/// `rcefRunPart(env, part, arg)` (`0x0092eec0`).
pub const RUN_PART: u32 = 0x0092_eec0;
/// `rcefAnimeStep(anim)` (`0x0092f400`).
pub const ANIME_STEP: u32 = 0x0092_f400;
/// `g_pppWorldMatrix` (`0x0230ffa0`).
pub const PPP_WORLD_MATRIX: u32 = 0x0230_ffa0;
/// `g_pppCameraMatrix` (`0x0230ff60`).
pub const PPP_CAMERA_MATRIX: u32 = 0x0230_ff60;
/// `g_pppViewMatrix` (`0x0230ff20`).
pub const PPP_VIEW_MATRIX: u32 = 0x0230_ff20;
/// `g_pppViewMatrixCopy` (`0x0230fe20`).
pub const PPP_VIEW_MATRIX_COPY: u32 = 0x0230_fe20;
/// The part scale `rcefParProc` writes: three floats at `0x0230fd50`.
pub const PPP_SCALE: u32 = 0x0230_fd50;
/// 4096.0 (`f64`, `.rdata`): `rcefParProc`'s colour scale.
pub const PAR_COLOUR_SCALE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_15c8);
/// 128.0 (`f64`, `.rdata`): the colour scale of `rcefAnimeDraw` and
/// `rcefModelProc`.
pub const COLOUR_128: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b4_c170);
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (`0x006453d0`).
pub const VFX_GET_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicVFXUpdate(lo, hi)` (`0x006458a0`).
pub const VFX_UPDATE: u32 = 0x0064_58a0;
/// `graphicVFXPath3Update(lo, hi)` (`0x00645600`).
pub const VFX_PATH3_UPDATE: u32 = 0x0064_5600;
/// `updateVFXRenderDataShp(lo, hi, part, obj, segment)` (`0x0071b8a0`).
pub const UPDATE_RENDER_DATA_SHP: u32 = 0x0071_b8a0;
/// `InitVFXRenderDataShp(lo, hi, part, obj, shape, f)` (`0x00711460`).
pub const INIT_RENDER_DATA_SHP: u32 = 0x0071_1460;
/// `updateVFXRenderData(lo, hi, part, obj)` (`0x00718610`).
pub const UPDATE_RENDER_DATA: u32 = 0x0071_8610;
/// `InitVFXRenderData(lo, hi, part, obj, e)` (`0x00711180`).
pub const INIT_RENDER_DATA: u32 = 0x0071_1180;
/// `InitVFXPath3RenderData(lo, hi, part, obj, e)` (`0x00710fd0`).
pub const INIT_PATH3_RENDER_DATA: u32 = 0x0071_0fd0;
/// `calculateVFXSP_AlphaFunction(f)` (`0x00712c30`).
pub const ALPHA_FUNCTION: u32 = 0x0071_2c30;
/// `calculateVFXSP_ZMat(z)` (`0x00712d00`).
pub const Z_MAT: u32 = 0x0071_2d00;
/// `setVfxShapeEnlargeBufferFactor(n)` (`0x00717610`).
pub const SET_ENLARGE: u32 = 0x0071_7610;
/// `resetVfxShapeEnlargeBufferFactor()` (`0x00717600`).
pub const RESET_ENLARGE: u32 = 0x0071_7600;
/// `rcVu0Add_0092f790(a, b, c)` (`0x0092f790`).
pub const VU0_ADD_F790: u32 = 0x0092_f790;
/// `rcVu0Add4_0092f880(a, b, c)` (`0x0092f880`).
pub const VU0_ADD4_F880: u32 = 0x0092_f880;
/// `rcbgScaleVector_0092f7f0(dst, v, s)` (`0x0092f7f0`).
pub const SCALE_VECTOR_F7F0: u32 = 0x0092_f7f0;
/// `rcMat4fSetScaleVec3f(m, v)` (`0x00925820`).
pub const SET_SCALE_VEC3F: u32 = 0x0092_5820;
/// The effects' base matrix (`0x025d59e0`, name ours): flag 0x4000 of
/// `rcefObjCalcLocalMatrix` multiplies it in.
pub const RCEF_BASE_MATRIX: u32 = 0x025d_59e0;
/// 11.38 (`f32`, `.rdata`): an object's rotation units per radian
/// (`rcefObjCalcLocalMatrix`).
pub const ROT_UNITS: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b6_78ec);
/// -1.0 (`f32`, `.rdata`).
pub const MINUS_ONE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b9_2340);
/// `rcefVecToField(v)` (`0x0092d7e0`).
pub const VEC_TO_FIELD: u32 = 0x0092_d7e0;
/// `Ch_ElemGetBaseMatrix(chr, e, out)` (`0x00833b80`).
pub const CH_ELEM_GET_BASE_MATRIX: u32 = 0x0083_3b80;
/// `rcbgScaleVector_0092d780(dst, v, s)` (`0x0092d780`).
pub const SCALE_VECTOR_D780: u32 = 0x0092_d780;
/// `rcMat4fSetScale0001(m)` (`0x0092de80`).
pub const SET_SCALE_0001: u32 = 0x0092_de80;
/// `MsGetChrPos(i, out)` (`0x007956e0`).
pub const MS_GET_CHR_POS: u32 = 0x0079_56e0;
/// `Ch_GetScale(chr, x, y, z)` (`0x0082ad00`).
pub const CH_GET_SCALE: u32 = 0x0082_ad00;
/// The base matrix `rcefSetChrBaseMatrix` builds without an output
/// (`0x0193c090`, name ours: `g_RcefChrBaseMatrix`).
pub const CHR_BASE_MATRIX: u32 = 0x0193_c090;
/// -0.001 (`f32`, `.rdata`): the element offset scale of
/// `rcefCalcChrElemRot*Matrix`.
pub const MILLI_NEG: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b6_7e20);
/// `Ch_ClutGetNb(chr)` (`0x0083c9a0`).
pub const CH_CLUT_GET_NB: u32 = 0x0083_c9a0;
/// `Ch_ClutGetSize(chr, c)` (`0x0083c9f0`).
pub const CH_CLUT_GET_SIZE: u32 = 0x0083_c9f0;
/// `Ch_ClutGetOrgPtr(chr, c)` (`0x0083c9b0`).
pub const CH_CLUT_GET_ORG_PTR: u32 = 0x0083_c9b0;
/// `Ch_ClutGetPtr(chr, c)` (`0x0083c9d0`).
pub const CH_CLUT_GET_PTR: u32 = 0x0083_c9d0;
/// `FFXVu0UnitMatrix(m)` (`0x006ed9a0`).
pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
/// `FFXVu0CopyMatrix(dst, src)` (`0x006ed360`).
pub const COPY_MATRIX: u32 = 0x006e_d360;
/// `FFXVu0MulMatrix(dst, a, b)` (`0x006ed5c0`).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `yiGetMatrixDObj(index, out)` (`0x00908df0`).
pub const YI_GET_MATRIX_DOBJ: u32 = 0x0090_8df0;
/// `rcCopyQword_0092f850(dst, src)` (`0x0092f850`).
pub const COPY_QWORD_F850: u32 = 0x0092_f850;
/// `rcVec3fMul(out, a, b)` (`0x009212c0`), the cross product.
pub const VEC3F_MUL: u32 = 0x0092_12c0;
/// `op_rt_m_yxz(dst, src, angles, t)` (`0x007e9a30`).
pub const OP_RT_M_YXZ: u32 = 0x007e_9a30;
/// 651.9 (`f64`, `.rdata`): `rcefObjCalcParentMatrix`'s turn per unit of
/// `Ch_GetRot`.
pub const ROT_SCALE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_1590);
/// 1024.0 (`f64`, `.rdata`): the quarter turn it takes off.
pub const ROT_BIAS: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_1ba8);
/// 10.0 as an `f64` (`.rdata`): `rcefGetChrWidth`, `rcefGetChrEyeHeight`.
pub const TEN: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_22b0);
/// 10.0 as an `f32` (`.rdata`): `rcefGetChrElemOff`'s scale.
pub const TEN_F32: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b9_2318);
/// 1000.0 as an `f32` (`.rdata`): `rcefGetChrElemPos`'s scale.
pub const THOUSAND_F32: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b1_f2cc);

/// The tasks and objects where the game keeps them, through
/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{
        ALIVE, DEFAULT_PRIORITY, NO_DESTRUCTOR, RERUN, WAIT_SEQ, anime, args_slot, call_level,
        ctrl, header, obj, par, ret_level, seq_slot,
    };

    /// The first task `rcEffectTaskTaskWait` compares with (read only there).
    pub const TASK_WAIT_A: Global<u32> = Global::new(0x025d_5a44);
    /// The second one.
    pub const TASK_WAIT_B: Global<u32> = Global::new(0x025d_5afc);
    /// The pray effect `rcEffectTaskPrayWait` waits for.
    pub const PRAY_EFFECT: Global<u32> = Global::new(0x025d_5a24);
    /// What `rcEffectTaskFlagWait` waits to be 0 (read only there).
    pub const FLAG_WAIT: Global<u32> = Global::new(0x025d_5c10);
    /// The request `rcefPostRequest` stores: five words at `0x025d5c18`
    /// (the first argument, the float, the third, a zero, the fourth, the
    /// fifth; `+0x10` is the zero). Nothing in the PC build reads them.
    pub const REQUEST: u32 = 0x025d_5c18;
    /// Set by `rcefPostRequest`; `rcEffectTaskRequestWait` waits for it to
    /// be 0, and nothing in the PC build clears it.
    pub const REQUEST_PENDING: Global<u32> = Global::new(0x025d_5c34);
    /// The program index `pppRunPartStd` is running (`g_pppCurrentProgram`).
    pub const CURRENT_PROGRAM: Global<u32> = Global::new(0x0230_fd20);
    /// `Sg_Blur(v)` (`0x0082bee0`).
    pub const SG_BLUR: u32 = 0x0082_bee0;
    /// `MsScreenOffset(x, y)` (`0x007bc0c0`, floats).
    pub const MS_SCREEN_OFFSET: u32 = 0x007b_c0c0;
    /// The pre-computed sine table (`g_pppSinTable`, 4096 `f32`s).
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// 32768.0 (`f64`, `.rdata`).
    pub const LERP_HALF_TURN: u32 = 0x00b5_1588;
    /// 16384.0 (`f64`).
    pub const LERP_QUARTER_TURN: u32 = 0x00b5_1668;
    /// 1.0 (`f64`).
    pub const LERP_ONE: u32 = 0x00b9_2248;
    /// 0.5 (`f64`).
    pub const LERP_HALF: u32 = 0x00b9_2220;
    /// The effect's colour work (`rcefColorSet`/`rcefColorFade`).
    pub const COLOR_WORK: u32 = 0x025d_5a20;
    /// `TkSetVibrateStream(a, b, entry)` (`0x00875fb0`).
    pub const TK_SET_VIBRATE_STREAM: u32 = 0x0087_5fb0;
    /// `op_oef2_stream_cue(entry, arg, mask)` (`0x00800390`).
    pub const OP_OEF2_STREAM_CUE: u32 = 0x0080_0390;
    /// `rcefAnimeStart(anime, data, out0, out1)` (`0x0092f470`).
    pub const RCEF_ANIME_START: u32 = 0x0092_f470;
    /// `rcefSetupDraw(draw, tex, a, b)` (`0x0092f6b0`).
    pub const RCEF_SETUP_DRAW: u32 = 0x0092_f6b0;
    /// `op_oef2_sound_cue_check(mask)` (`0x008000f0`).
    pub const SOUND_CUE_CHECK: u32 = 0x0080_00f0;
    /// `MsEffectCheckMotion(x)` (`0x00787840`).
    pub const MS_EFFECT_CHECK_MOTION: u32 = 0x0078_7840;
    /// `MsSetChrDirTarget` (`0x00795eb0`, six words).
    pub const MS_SET_CHR_DIR_TARGET: u32 = 0x0079_5eb0;
    /// `MsGetWeaponHandle(chr, n)` (`0x00795b40`).
    pub const MS_GET_WEAPON_HANDLE: u32 = 0x0079_5b40;
    /// `Ch_SetHideEffect(handle, hide)` (`0x0082b2a0`).
    pub const CH_SET_HIDE_EFFECT: u32 = 0x0082_b2a0;
    /// The effect's oef2 data (`rcefPlaySE`'s source of sound entries).
    pub const RCEF_OEF2: Global<u32> = Global::new(0x025d_5a28);
    /// `op_oef2_a_get(data, kind, id)` (`0x007e3810`).
    pub const OP_OEF2_A_GET: u32 = 0x007e_3810;
    /// `op_oef2_sep_number_get(entry)` (`0x007fff80`).
    pub const OP_OEF2_SEP_NUMBER_GET: u32 = 0x007f_ff80;
    /// `op_oef2_se_on(entry, a, b)` (`0x007ffe10`).
    pub const OP_OEF2_SE_ON: u32 = 0x007f_fe10;
    /// `rcefMainCreateObj(kind)` (`0x00925140`).
    pub const RCEF_MAIN_CREATE_OBJ: u32 = 0x0092_5140;
    /// `rcefTaskSetSeq(task, seq)` (`0x0092ee20`).
    pub const RCEF_TASK_SET_SEQ: u32 = 0x0092_ee20;
    /// `rcefChrFini(obj)` (`0x00930710`).
    pub const RCEF_CHR_FINI: u32 = 0x0093_0710;
    /// `g_pppEnvSlot`: where `pppInitEnv` keeps the current environment.
    pub const PPP_ENV_SLOT: Global<u32> = Global::new(0x00c3_a4cc);
    /// `graphicVFXDestroy(lo, hi)` (cdecl, a 64-bit instance handle).
    pub const VFX_DESTROY: u32 = 0x0064_5280;
    /// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
    pub const VFX_GET_RENDER_DATA: u32 = 0x0064_53d0;

    fn level(mem: &impl Mem, t: u32) -> i32 {
        mem.read(t + header::LEVEL)
    }

    fn set_flags(mem: &mut impl Mem, t: u32, bits: u32) {
        let f = mem.read::<u32>(t + header::FLAGS);
        mem.write(t + header::FLAGS, f | bits);
    }

    /// `rcefTaskInit(task)` (`0x0092ed10`): cleared, alive, the empty
    /// destructor and the default priority.
    pub fn rcef_task_init(mem: &mut impl Mem, t: u32) {
        mem.write_bytes(t, &[0; header::SIZE as usize]);
        mem.write(t + header::FLAGS, ALIVE);
        let none = mem.rebase(NO_DESTRUCTOR);
        mem.write(t + header::DESTRUCTOR, none);
        mem.write(t + header::PRIORITY, DEFAULT_PRIORITY);
    }

    /// `rcefTaskSetSeq(task, seq)` (`0x0092ee20`): replaces the current
    /// level's sequence.
    pub fn rcef_task_set_seq(mem: &mut impl Mem, t: u32, seq: u32) {
        let l = level(mem, t);
        mem.write(seq_slot(t, l), seq);
    }

    /// `rcefTaskSetSeqCont(task, seq)` (`0x0092ee40`): replaces it and runs
    /// the new one this frame.
    pub fn rcef_task_set_seq_cont(mem: &mut impl Mem, t: u32, seq: u32) {
        rcef_task_set_seq(mem, t, seq);
        set_flags(mem, t, RERUN);
    }

    /// `rcefTaskCallSeq(task, seq)` (`0x0092ecb0`): pushes a sequence
    /// (nothing when all eight levels are taken).
    pub fn rcef_task_call_seq(mem: &mut impl Mem, t: u32, seq: u32) {
        if let Some(l) = call_level(level(mem, t)) {
            mem.write(t + header::LEVEL, l);
            mem.write(seq_slot(t, l), seq);
        }
    }

    /// `rcefTaskRetSeqCont_Inner(task)` (`0x0092ede0`): pops a level.
    pub fn rcef_task_ret_seq_cont_inner(mem: &mut impl Mem, t: u32) {
        let l = level(mem, t);
        if l > 0 {
            mem.write(t + header::LEVEL, ret_level(l));
        }
    }

    /// `rcefTaskRetSeqCont(task)` (`0x0092ee00`): pops a level and runs the
    /// caller this frame.
    pub fn rcef_task_ret_seq_cont(mem: &mut impl Mem, t: u32) {
        rcef_task_ret_seq_cont_inner(mem, t);
        set_flags(mem, t, RERUN);
    }

    /// `rcefTaskWait(task, frames)` (`0x0092ee60`): calls `rcefTaskWaitSeq`
    /// with the count in the new level's arguments (none for 0 or less).
    pub fn rcef_task_wait(mem: &mut impl Mem, t: u32, frames: i32) {
        if frames > 0 {
            let seq = mem.rebase(WAIT_SEQ);
            rcef_task_call_seq(mem, t, seq);
            let l = level(mem, t);
            mem.write(args_slot(t, l), frames);
        }
    }

    /// `rcefTaskWaitSeq(task)` (`0x0092ee90`): counts a frame down; returns
    /// at 0.
    pub fn rcef_task_wait_seq(mem: &mut impl Mem, t: u32) {
        let at = args_slot(t, level(mem, t));
        let n = mem.read::<i32>(at).wrapping_sub(1);
        mem.write(at, n);
        // read again, level included, as the original does
        if mem.read::<i32>(args_slot(t, level(mem, t))) < 1 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefTaskCtrlInit(ctrl)` (`0x00930290`): head and tail linked.
    pub fn rcef_task_ctrl_init(mem: &mut impl Mem, c: u32) {
        let (head, tail) = (c + ctrl::HEAD, c + ctrl::TAIL);
        rcef_task_init(mem, head);
        rcef_task_init(mem, tail);
        mem.write(head + header::NEXT, tail);
        mem.write(head + header::PRIORITY, 0u32);
        mem.write(tail + header::PRIORITY, ctrl::TAIL_PRIORITY);
        mem.write(tail + header::PREV, head);
        mem.write(c + ctrl::FIELD_178, 0u32);
    }

    /// `rcefTaskCtrlIsEnd(ctrl)` (`0x009302e0`): whether no task is left
    /// between head and tail.
    pub fn rcef_task_ctrl_is_end(mem: &impl Mem, c: u32) -> bool {
        mem.read::<u32>(c + ctrl::HEAD + header::NEXT) == c + ctrl::TAIL
    }

    /// `rcefTaskCtrlAdd(first, task)` (`0x00930220`): links `task` before
    /// the first task from `first` on with a greater priority (nothing when
    /// there is none, or `first` is null).
    pub fn rcef_task_ctrl_add(mem: &mut impl Mem, first: u32, t: u32) {
        let mut at = first;
        if at == 0 {
            return;
        }
        let p = mem.read::<i32>(t + header::PRIORITY);
        while mem.read::<i32>(at + header::PRIORITY) <= p {
            at = mem.read(at + header::NEXT);
            if at == 0 {
                return;
            }
        }
        let prev = mem.read::<u32>(at + header::PREV);
        mem.write(prev + header::NEXT, t);
        let prev = mem.read::<u32>(at + header::PREV);
        mem.write(t + header::PREV, prev);
        mem.write(at + header::PREV, t);
        mem.write(t + header::NEXT, at);
    }

    /// `rcefObjIsParEnd(obj)` (`0x00930010`): whether its particle part
    /// stopped.
    pub fn rcef_obj_is_par_end(mem: &impl Mem, o: u32) -> bool {
        mem.read::<u8>(o + obj::PAR) & 1 == 0
    }

    /// `rcefObjIsAnimeEnd(obj)` (`0x0092fff0`): whether its animation
    /// stopped.
    pub fn rcef_obj_is_anime_end(mem: &impl Mem, o: u32) -> bool {
        mem.read::<u8>(o + obj::ANIME) & 1 == 0
    }

    /// `rcefObjAttachChr(obj, chr, element)` (`0x0092f970`).
    pub fn rcef_obj_attach_chr(mem: &mut impl Mem, o: u32, chr: u32, element: u32) {
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m & !obj::DETACHED_BIT | obj::ATTACHED);
        mem.write(o + obj::CHR, chr);
        mem.write(o + obj::CHR_ELEMENT, element);
    }

    /// `rcefObjClearMode(obj)` (`0x009301e0`).
    pub fn rcef_obj_clear_mode(mem: &mut impl Mem, o: u32) {
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m & !obj::MODE_MASK);
    }

    /// `rcefObjSetMode(obj, mode)` (`0x00930200`): the mode bits replaced
    /// (`mode` is or-ed in, unmasked).
    pub fn rcef_obj_set_mode(mem: &mut impl Mem, o: u32, mode: u32) {
        rcef_obj_clear_mode(mem, o);
        let m = mem.read::<u32>(o + obj::MODE);
        mem.write(o + obj::MODE, m | mode);
    }

    /// `rcefParInit(par)` (`0x0092efd0`).
    pub fn rcef_par_init(mem: &mut impl Mem, p: u32) {
        mem.write_bytes(p, &[0; par::SIZE as usize]);
        for (o, v) in par::INIT {
            mem.write(p + o, v);
        }
    }

    /// `rcefAnimeInit(anime)` (`0x0092f4c0`).
    pub fn rcef_anime_init(mem: &mut impl Mem, a: u32) {
        mem.write_bytes(a, &[0; anime::SIZE as usize]);
        mem.write(a + anime::RATE, 0x200u16);
    }

    /// `rcefAnimeStart(anime, data, out0, out1)` (`0x0092f470`).
    pub fn rcef_anime_start(mem: &mut impl Mem, a: u32, data: u32, out0: u32, out1: u32) {
        mem.write(a + anime::DATA, data);
        mem.write(a + anime::OUT[0], out0);
        mem.write(a + anime::OUT[1], out1);
        mem.write(a + anime::RUNNING, 1u32);
        mem.write(a + anime::CURSOR, a + anime::WORK);
    }

    /// `rcefModelInit(model)` (`0x0092f750`): an object's model part,
    /// `0xd0` bytes, cleared.
    pub fn rcef_model_init(mem: &mut impl Mem, m: u32) {
        mem.write_bytes(m, &[0; 0xd0]);
    }

    /// `rcefIsBattle()` (`0x0092e620`): whether a battle runs
    /// (`MsBattleGetStatus` non-zero).
    pub fn rcef_is_battle(mem: &impl Mem) -> bool {
        ffx_battle::state::bridge::ms_battle_get_status(mem) != 0
    }

    /// `rcefDirToAngle(dir)` (`0x0092de30`): `3π/2 - dir`, as `f32`; the
    /// float as bits.
    pub fn rcef_dir_to_angle(mem: &impl Mem, dir: u32) -> f32 {
        let k = super::THREE_HALF_PI.get(mem);
        f32::from_bits(crate::x87::store(k - crate::x87::load(dir)))
    }

    /// `rcAngleDiff(a, b)` (`0x0092e400`): `|a - b|` as `f32`, wrapped to
    /// `2π - d` past π; floats as bits.
    pub fn rc_angle_diff(mem: &impl Mem, a: u32, b: u32) -> f32 {
        use ffx_bridge::x87;
        let d = x87::load(x87::store(x87::sub(x87::load(a), x87::load(b)).abs()));
        if d > super::PI.get(mem) {
            f32::from_bits(x87::store(x87::sub(super::TWO_PI.get(mem), d)))
        } else {
            f32::from_bits(x87::store(d))
        }
    }

    fn target_at(mem: &impl Mem, i: u32, offset: u32) -> u32 {
        super::target::TABLE
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(super::target::SIZE))
            .wrapping_add(offset)
    }

    /// `rcefTargetIsDone(i)` (`0x00924ba0`): flag bit 1 of target `i`.
    pub fn rcef_target_is_done(mem: &impl Mem, i: u32) -> u32 {
        u32::from(mem.read::<u8>(target_at(mem, i, super::target::FLAGS)) >> 1 & 1)
    }

    /// `rcefTargetClear(i)` (`0x00924bc0`): its flags.
    pub fn rcef_target_clear(mem: &mut impl Mem, i: u32) {
        let at = target_at(mem, i, super::target::FLAGS);
        mem.write(at, 0u16);
    }

    fn target_float(mem: &impl Mem, i: u32, offset: u32) -> f32 {
        f32::from_bits(mem.read(target_at(mem, i, offset)))
    }

    /// `rcefTargetGetDist(i)` (`0x00924a00`).
    pub fn rcef_target_get_dist(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::DIST)
    }

    /// `rcefTargetGetAngle(i)` (`0x009249e0`).
    pub fn rcef_target_get_angle(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::ANGLE)
    }

    /// `rcefTargetGetHeight(i)` (`0x009249c0`).
    pub fn rcef_target_get_height(mem: &impl Mem, i: u32) -> f32 {
        target_float(mem, i, super::target::HEIGHT)
    }

    /// `rcefTargetSetDelay(frames)` (`0x00924be0`).
    pub fn rcef_target_set_delay(mem: &mut impl Mem, frames: u16) {
        super::target::DELAY.set(mem, frames);
    }

    /// `rcefMainSetEnv(main)` (`0x009254f0`, an identical copy at
    /// `0x00925510`): makes the effect's memory environment current.
    pub fn rcef_main_set_env(mem: &mut impl Mem, m: u32) {
        let env = mem.read::<u32>(m + super::main::ENV);
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, env);
    }

    /// `rcefObjLinkLast(obj, child)` (`0x0092f920`): `child` after the last
    /// object of `obj`'s chain (`+0xcc` next, `+0xc8` previous).
    pub fn rcef_obj_link_last(mem: &mut impl Mem, o: u32, child: u32) {
        let mut last = o;
        loop {
            let next = mem.read::<u32>(last + 0xcc);
            if next == 0 {
                break;
            }
            last = next;
        }
        mem.write(last + 0xcc, child);
        mem.write(child + 0xc8, last);
    }

    /// `rcEffectTaskParWait(obj)` (`0x00930490`): a sequence that returns
    /// once the particle part stopped.
    pub fn rc_effect_task_par_wait(mem: &mut impl Mem, o: u32) {
        if rcef_obj_is_par_end(mem, o) {
            rcef_task_ret_seq_cont(mem, o);
        }
    }

    /// `rcEffectTaskAnimeWait(obj)` (`0x00930350`): returns once the
    /// animation stopped.
    pub fn rc_effect_task_anime_wait(mem: &mut impl Mem, o: u32) {
        if rcef_obj_is_anime_end(mem, o) {
            rcef_task_ret_seq_cont(mem, o);
        }
    }

    /// `rcEffectTaskTaskWait(task)` (`0x00930520`): returns once both
    /// [`TASK_WAIT_A`] and [`TASK_WAIT_B`] are this task.
    pub fn rc_effect_task_task_wait(mem: &mut impl Mem, t: u32) {
        if TASK_WAIT_A.get(mem) == t && TASK_WAIT_B.get(mem) == t {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcEffectTaskFlagWait(task)` (`0x009303c0`): returns once
    /// [`FLAG_WAIT`] is 0.
    pub fn rc_effect_task_flag_wait(mem: &mut impl Mem, t: u32) {
        if FLAG_WAIT.get(mem) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    // ---- round 21 ----

    /// `rcefPostRequest(a, f, b, c, d)` (`0x0092eba0`, an exported effect
    /// import; name ours): stores the request at [`REQUEST`] (the float
    /// copied through the x87 stack, so a signalling NaN turns quiet) and
    /// sets [`REQUEST_PENDING`]. Returns `d` (left in `eax`).
    pub fn rcef_post_request(mem: &mut impl Mem, args: [u32; 5]) -> u32 {
        let [a, f, b, c, d] = args;
        let f = ffx_bridge::x87::round_trip(f32::from_bits(f)).to_bits();
        let r = mem.rebase(REQUEST);
        mem.write(r, a);
        mem.write(r + 4, f);
        mem.write(r + 8, b);
        mem.write(r + 0x10, c);
        mem.write(r + 0xc, 0u32);
        mem.write(r + 0x14, d);
        REQUEST_PENDING.set(mem, 1);
        d
    }

    /// `rcEffectTaskRequestWait(task)` (`0x009304e0`, name ours): a
    /// sequence that returns once [`REQUEST_PENDING`] is 0.
    pub fn rc_effect_task_request_wait(mem: &mut impl Mem, t: u32) {
        if REQUEST_PENDING.get(mem) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefAnimeFini(id)` (`0x0092f4a0`): `graphicVFXDestroy` of the
    /// handle `id << 32`. Returns its `eax`.
    pub fn rcef_anime_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(VFX_DESTROY);
        call(mem, f, &[0, id])
    }

    /// `rcefModelFini(id)` (`0x0092f710`): the handle `(id << 32) +
    /// program` (the current program sign-extended) destroyed when the
    /// renderer has render data for it. Returns the last call's `eax`.
    pub fn rcef_model_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let p = CURRENT_PROGRAM.get(mem);
        let hi = id.wrapping_add(if p.cast_signed() < 0 { u32::MAX } else { 0 });
        let f = mem.rebase(VFX_GET_RENDER_DATA);
        let r = call(mem, f, &[p, hi]);
        if r == 0 {
            return 0;
        }
        let f = mem.rebase(VFX_DESTROY);
        call(mem, f, &[p, hi])
    }

    // ---- round 22 ----

    /// `rcefObjFini(obj)` (`0x0092fee0`, an object's finaliser
    /// [`obj::FINI`]): the particle part's finaliser (`0x0092efc0`, empty),
    /// [`rcef_anime_fini`] with the animation's address (`+0x330`) as the
    /// id, [`rcef_model_fini`] with the model's (`+0x3c0`), then the empty
    /// destructor. Returns `rcefModelFini`'s `eax`.
    pub fn rcef_obj_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        call: &mut C,
    ) -> u32 {
        rcef_anime_fini(mem, o.wrapping_add(obj::ANIME), call);
        rcef_model_fini(mem, o.wrapping_add(obj::MODEL), call)
    }

    // ---- round 23 ----

    /// `rcEffectTaskSoundCueWait(task)` (`0x00930500`, name ours): a
    /// sequence that returns once `op_oef2_sound_cue_check(0x800)` is 0.
    pub fn rc_effect_task_sound_cue_wait<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(SOUND_CUE_CHECK);
        if call(mem, f, &[0x800]) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcEffectTaskMotionWait(task)` (`0x00930430`): returns once
    /// `MsEffectCheckMotion` of the current level's first argument is -1.
    pub fn rc_effect_task_motion_wait<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        let a: u32 = mem.read(args_slot(t, level(mem, t)));
        let f = mem.rebase(MS_EFFECT_CHECK_MOTION);
        if call(mem, f, &[a]) == u32::MAX {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefSetChrDirTarget(chr, a, b)` (`0x0092e750`, name ours):
    /// `MsSetChrDirTarget(chr, 1, 0, 0, a, b)`, its `eax`.
    pub fn rcef_set_chr_dir_target<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let [chr, a, b] = args;
        let f = mem.rebase(MS_SET_CHR_DIR_TARGET);
        call(mem, f, &[chr, 1, 0, 0, a, b])
    }

    /// `rcefSetChrDirTarget2(chr, a, b, c)` (`0x0092e960`, name ours):
    /// `MsSetChrDirTarget(chr, 2, 0, a, b, c)`, its `eax`.
    pub fn rcef_set_chr_dir_target2<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut C,
    ) -> u32 {
        let [chr, a, b, c] = args;
        let f = mem.rebase(MS_SET_CHR_DIR_TARGET);
        call(mem, f, &[chr, 2, 0, a, b, c])
    }

    /// `rcefWeaponSetHide(chr, show)` (`0x0092eb70`, name ours): with a
    /// weapon handle (`MsGetWeaponHandle(chr, 0)`),
    /// `Ch_SetHideEffect(handle, show == 0)` and its `eax`; else 0.
    pub fn rcef_weapon_set_hide<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        chr: u32,
        show: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(MS_GET_WEAPON_HANDLE);
        let h = call(mem, f, &[chr, 0]);
        if h == 0 {
            return 0;
        }
        let f = mem.rebase(CH_SET_HIDE_EFFECT);
        call(mem, f, &[h, u32::from(show == 0)])
    }

    /// `rcefPlaySE(id)` (`0x0092e680`): the sound entry `id` of the effect
    /// data at [`RCEF_OEF2`] (`op_oef2_a_get(data, 6, id)`),
    /// `op_oef2_sep_number_get` of it (unused), then
    /// `op_oef2_se_on(entry, 0x40, 0x100)` and its `eax`.
    pub fn rcef_play_se<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let data = RCEF_OEF2.get(mem);
        let f = mem.rebase(OP_OEF2_A_GET);
        let e = call(mem, f, &[data, 6, id]);
        let f = mem.rebase(OP_OEF2_SEP_NUMBER_GET);
        call(mem, f, &[e]);
        let f = mem.rebase(OP_OEF2_SE_ON);
        call(mem, f, &[e, 0x40, 0x100])
    }

    /// `rcefMainCreateTask(kind, seq)` (`0x00925170`): `rcefMainCreateObj
    /// (kind)` with the sequence set (`rcefTaskSetSeq`). Returns the
    /// object.
    pub fn rcef_main_create_task<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        kind: u32,
        seq: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(RCEF_MAIN_CREATE_OBJ);
        let o = call(mem, f, &[kind]);
        let f = mem.rebase(RCEF_TASK_SET_SEQ);
        call(mem, f, &[o, seq]);
        o
    }

    /// `rcefChrDelete(obj)` (`0x00930600`, name ours): `rcefChrFini`, then
    /// the object freed into the current ppp environment
    /// (`pppMemFree(env, env + 0x10, obj)`, `env` read through
    /// `g_pppEnvSlot`). Returns the free's `eax`.
    pub fn rcef_chr_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(RCEF_CHR_FINI);
        call(mem, f, &[o]);
        let slot = PPP_ENV_SLOT.get(mem);
        let env = mem.read::<u32>(slot);
        let f = mem.rebase(super::PPP_MEM_FREE);
        call(mem, f, &[env, env.wrapping_add(0x10), o])
    }

    // ---- round 24 ----

    /// The effect data's entry `id` of `kind` (`op_oef2_a_get`).
    fn oef2_entry<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        data: u32,
        kind: u32,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(OP_OEF2_A_GET);
        call(mem, f, &[data, kind, id])
    }

    /// `rcefSetVibrate(id, arg)` (`0x0092ec60`; the Fahrenheit Switch `BSim`
    /// guess was `rcefStreamCue`): `TkSetVibrateStream(7, arg, entry)` of
    /// the effect data's kind 5 entry `id`, its `eax`.
    pub fn rcef_set_vibrate<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let data = RCEF_OEF2.get(mem);
        let e = oef2_entry(mem, data, 5, id, call);
        let f = mem.rebase(TK_SET_VIBRATE_STREAM);
        call(mem, f, &[7, arg, e])
    }

    /// `rcefStreamCue(id, arg)` (`0x0092ebf0`): `op_oef2_stream_cue(entry,
    /// arg, 0x800)` of the kind 6 entry `id`, its `eax`.
    pub fn rcef_stream_cue<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let data = RCEF_OEF2.get(mem);
        let e = oef2_entry(mem, data, 6, id, call);
        let f = mem.rebase(OP_OEF2_STREAM_CUE);
        call(mem, f, &[e, arg, 0x800])
    }

    /// `rcefPlayStream(id)` (`0x0092e6b0`): unless a stream cue is busy
    /// (`op_oef2_sound_cue_check(0x800)`, whose `eax` is then returned),
    /// [`rcef_play_se`]'s `op_oef2_se_on` of the kind 6 entry `id`.
    pub fn rcef_play_stream<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(SOUND_CUE_CHECK);
        let busy = call(mem, f, &[0x800]);
        if busy != 0 {
            return busy;
        }
        let data = RCEF_OEF2.get(mem);
        let e = oef2_entry(mem, data, 6, id, call);
        let f = mem.rebase(OP_OEF2_SE_ON);
        call(mem, f, &[e, 0x40, 0x100])
    }

    /// `rcefObjAnimeStart(env, obj, id)` (`0x00925590`, an effect import;
    /// name ours): `rcefAnimeStart(obj + 0x330, entry, obj + 0x1f0, obj +
    /// 0x170)` with the kind 4 entry `id` of `env`'s data (`+8`).
    pub fn rcef_obj_anime_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut C,
    ) -> u32 {
        obj_start(mem, args, obj::ANIME, RCEF_ANIME_START, call)
    }

    /// `rcefObjSetupDraw(env, obj, id)` (`0x00925690`, an effect import;
    /// name ours): `rcefSetupDraw(obj + 0x3c0, entry, obj + 0x1f0, obj +
    /// 0x170)` with the kind 4 entry `id`.
    pub fn rcef_obj_setup_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut C,
    ) -> u32 {
        obj_start(mem, args, obj::MODEL, RCEF_SETUP_DRAW, call)
    }

    fn obj_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        part: u32,
        f: u32,
        call: &mut C,
    ) -> u32 {
        let [env, o, id] = args;
        let data = mem.read::<u32>(env.wrapping_add(8));
        let e = oef2_entry(mem, data, 4, id, call);
        let f = mem.rebase(f);
        let a = [
            o.wrapping_add(part),
            e,
            o.wrapping_add(0x1f0),
            o.wrapping_add(0x170),
        ];
        call(mem, f, &a)
    }

    // ---- round 25 ----

    /// `rcefLerp(a, b, t, mode)` (`0x0092e580`, an effect import; name
    /// ours): `a + w (b - a)` with `w` = `t` (mode 0 and others),
    /// `sin(t)` (mode 2, the sine table at `0x00c44bf0` indexed by
    /// `trunc(t * 16384) >> 4 & 0xfff`) or `(sin(t * 32768 - 16384) + 1) /
    /// 2` (mode 1, a cosine ease, `w` stored as `f32`). `b - a` and the
    /// result are `f32`; returned in `ST0`.
    pub fn rcef_lerp(mem: &impl Mem, args: [u32; 4]) -> f32 {
        use ffx_bridge::x87::{add, ftol, load, mul, store, sub};
        let [a, b, t, mode] = args;
        let st = |x: f64| load(store(x));
        let (a, t) = (load(a), load(t));
        let d = st(sub(load(b), a));
        let c = |at: u32| mem.read::<f64>(mem.rebase(at));
        let sine = |x: f64| {
            let i = ftol(mem, x).cast_signed() >> 4 & 0xfff;
            let at = mem.rebase(SIN_TABLE) + 4 * i.cast_unsigned();
            load(mem.read::<u32>(at))
        };
        let r = match mode {
            1 => {
                let x = sub(mul(t, c(LERP_HALF_TURN)), c(LERP_QUARTER_TURN));
                let w = st(mul(add(sine(x), c(LERP_ONE)), c(LERP_HALF)));
                add(a, mul(w, d))
            }
            2 => add(a, mul(sine(mul(t, c(LERP_QUARTER_TURN))), d)),
            _ => add(a, mul(d, t)),
        };
        f32::from_bits(store(r))
    }

    /// The colour bytes as floats into `work` (`fild`, exact).
    fn colour_floats(mem: &mut impl Mem, bytes: &[u32], work: u32) {
        for (i, &b) in (0u32..).zip(bytes) {
            #[allow(clippy::cast_possible_truncation)] // a byte argument
            let v = f32::from(b as u8);
            mem.write(work.wrapping_add(4 * i), v.to_bits());
        }
    }

    /// `rcefColorSet(r, g, b, a, v, work)` (`0x0092e9c0`, an effect import;
    /// name ours): the four colour bytes as floats (into `work`, the
    /// original's locals) given to [`rcef_work_set_from_vf00`] on the
    /// effect's colour work at [`COLOR_WORK`].
    pub fn rcef_color_set(mem: &mut impl Mem, args: [u32; 5], work: u32) {
        colour_floats(mem, &args[..4], work);
        let w = mem.rebase(COLOR_WORK);
        rcef_work_set_from_vf00(mem, w, work, args[4]);
    }

    /// `rcefColorFade(r0, g0, b0, a0, r1, g1, b1, a1, v, work)`
    /// (`0x0092ea40`, an effect import; name ours): both colours as floats
    /// (`work`, 32 bytes) given to [`rcef_work_set`] on [`COLOR_WORK`].
    pub fn rcef_color_fade(mem: &mut impl Mem, args: [u32; 9], work: u32) {
        colour_floats(mem, &args[..8], work);
        let w = mem.rebase(COLOR_WORK);
        rcef_work_set(mem, w, work, work.wrapping_add(0x10), args[8]);
    }

    /// `rcCopyQword_009250a0(dst, src)`, `rcVu0Sub4(out, a, b)`,
    /// `rcbgScaleVector4_00925040(out, v, s)` and `rcVu0Add4(out, a, b)`:
    /// the vector helpers of [`rcef_color_update`].
    pub const COLOR_HELPERS: [u32; 4] = [0x0092_50a0, 0x0092_50d0, 0x0092_5040, 0x0092_4fc0];
    /// `graphicAddScreenFadeTask(r, g, b, a, after)` (`0x00639240`).
    pub const ADD_SCREEN_FADE_TASK: u32 = 0x0063_9240;
    /// 1/128 and 255.0 (`f64`s, `.rdata`): the fade's alpha and colour
    /// scales.
    pub const COLOR_SCALES: [u32; 2] = [0x00b1_70b8, 0x00b1_70c0];

    /// `fist` under a truncating control word: `x` toward zero, or the
    /// integer indefinite (`i32::MIN`) when out of range or NaN.
    fn fist_chop(x: f64) -> i32 {
        let t = x.trunc();
        if (-2_147_483_648.0..=2_147_483_647.0).contains(&t) {
            #[allow(clippy::cast_possible_truncation)] // in range
            let i = t as i32;
            i
        } else {
            i32::MIN
        }
    }

    /// `rcefColorUpdate(w)` (`0x00924c70`, from `rcefMainProcRunAfter`
    /// on the effect main object and through `rcefWorkCall_00924c70` on
    /// [`COLOR_WORK`]; name ours): the colour (in `work`, the
    /// original's 16-byte local) is `+0x1c4` at frame 0, `+0x1d4` at the
    /// last frame (`+0x1e4`), else `(to - from) * f32(frame / frames) +
    /// from` through the VU0 helpers ([`COLOR_HELPERS`], frame `+0x1e8`).
    /// `graphicAddScreenFadeTask(r / 255, g / 255, b / 255, a / 128, 0)`
    /// gets it, the alpha 0.0 when its truncated value's low byte is 0.
    /// While running (`+0x1f0`) the frame steps on, stopping at the last.
    /// Returns the `eax` it ends with: the fade task's, or the frame count
    /// when running.
    pub fn rcef_color_update<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        w: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        use crate::x87::{load, store};
        let [copy, sub, scale, add] = COLOR_HELPERS.map(|f| mem.rebase(f));
        let (from, to) = (w.wrapping_add(0x1c4), w.wrapping_add(0x1d4));
        let frame = mem.read::<i32>(w.wrapping_add(0x1e8));
        let frames = mem.read::<i32>(w.wrapping_add(0x1e4));
        if frame == 0 {
            call(mem, copy, &[work, from]);
        } else if frame == frames {
            call(mem, copy, &[work, to]);
        } else {
            let a = load(store(f64::from(frame)));
            let b = load(store(f64::from(frames)));
            let t = store(a / b);
            call(mem, sub, &[work, to, from]);
            call(mem, scale, &[work, work, t]);
            call(mem, add, &[work, work, from]);
        }
        let [alpha_scale, colour_scale] = COLOR_SCALES.map(|c| mem.read::<f64>(mem.rebase(c)));
        let [r, g, b, a] = mem.read::<[u32; 4]>(work).map(load);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low byte
        let alpha = if fist_chop(a) as u8 == 0 {
            0
        } else {
            store(a * alpha_scale)
        };
        let [r, g, b] = [r, g, b].map(|c| store(c / colour_scale));
        let f = mem.rebase(ADD_SCREEN_FADE_TASK);
        let mut eax = call(mem, f, &[r, g, b, alpha, 0]);
        if mem.read::<u32>(w.wrapping_add(0x1f0)) != 0 {
            let frames = mem.read::<i32>(w.wrapping_add(0x1e4));
            let at = w.wrapping_add(0x1e8);
            let next = mem.read::<i32>(at).wrapping_add(1);
            mem.write(at, next);
            if next > frames {
                mem.write(at, frames);
                mem.write(w.wrapping_add(0x1f0), 0u32);
            }
            eax = frames.cast_unsigned();
        }
        eax
    }

    /// `MsCopyCameraMatrix(out)`, `MsCopyScreenMatrix(out)`,
    /// `FFXVu0CopyMatrix(dst, src)`, `FFXVu0InversMatrix(out, m)`,
    /// `rcefTaskCtrlProc(ctrl)`, `MsEffectTerminate(id)`,
    /// `rcefColorUpdate(e)`, `rcefScreenShake(e)` and `rcefBlurUpdate(e)`:
    /// [`rcef_main_proc_run_after`]'s calls.
    pub const RUN_AFTER_CALLS: [u32; 9] = [
        0x007c_0070,
        0x007b_ba50,
        0x006e_d360,
        0x006e_d450,
        0x0093_0300,
        0x0078_8920,
        0x0092_4c70,
        0x0092_4df0,
        0x0092_4bf0,
    ];
    /// `g_pppScreenMatrix` and `g_PppPacketPos`.
    pub const PPP_SCREEN_MATRIX: u32 = 0x0230_fe60;
    /// See [`PPP_SCREEN_MATRIX`].
    pub const PPP_PACKET_POS: u32 = 0x0230_fd2c;

    /// `rcefMainProcRunAfter(e)` (`0x00925440`, the effect main object's
    /// per-frame step): the ppp environment slot gets `e +0x10`, the ppp
    /// camera and screen matrices are copied from the game's
    /// (`MsCopyCameraMatrix`, `MsCopyScreenMatrix`), the effect base
    /// matrix ([`super::RCEF_BASE_MATRIX`]) is the inverse of the camera's
    /// rotation (its translation zeroed first), the packet position
    /// `g_PppPacketPos` is `e +0xc`; the task controller (`+0x1c`) runs and,
    /// once it ends, `MsEffectTerminate(e +4)`; then without `+0x1f4` the
    /// colour fade ([`rcef_color_update`]), the screen shake and the blur,
    /// and `+0x22c` counts the frame. Returns the blur's `eax`.
    pub fn rcef_main_proc_run_after<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        call: &mut C,
    ) -> u32 {
        let [
            camera_copy,
            screen_copy,
            copy,
            invert,
            ctrl_proc,
            terminate,
            colour,
            shake,
            blur,
        ] = RUN_AFTER_CALLS.map(|f| mem.rebase(f));
        let slot = super::main::ENV_SLOT.get(mem);
        let env = mem.read::<u32>(e.wrapping_add(0x10));
        mem.write(slot, env);
        let camera = mem.rebase(super::PPP_CAMERA_MATRIX);
        call(mem, camera_copy, &[camera]);
        call(mem, screen_copy, &[mem.rebase(PPP_SCREEN_MATRIX)]);
        let base = mem.rebase(super::RCEF_BASE_MATRIX);
        call(mem, copy, &[base, camera]);
        mem.write(base + 0x30, [0u32; 3]);
        call(mem, invert, &[base, base]);
        let pos = mem.read::<u32>(e.wrapping_add(0xc));
        mem.write(mem.rebase(PPP_PACKET_POS), pos);
        let ctrl = e.wrapping_add(0x1c);
        call(mem, ctrl_proc, &[ctrl]);
        if rcef_task_ctrl_is_end(mem, ctrl) {
            let id = mem.read::<u32>(e.wrapping_add(4));
            call(mem, terminate, &[id]);
        }
        if mem.read::<u32>(e.wrapping_add(0x1f4)) == 0 {
            call(mem, colour, &[e]);
        }
        call(mem, shake, &[e]);
        let eax = call(mem, blur, &[e]);
        let frames = e.wrapping_add(0x22c);
        mem.write(frames, mem.read::<u32>(frames).wrapping_add(1));
        eax
    }

    // ---- round 26 ----

    /// `rcefScreenShake(e)` (`0x00924df0`, from `rcefMainProcRunAfter`;
    /// name ours): while on (`+0x214`) with a mode (`+0x1f8`), the swing of
    /// the phase `+0x210` (`model::bridge::shake_swing`), faded over the
    /// frames `+0x200` (done `+0x204`) for modes from 3, times `rand() %
    /// +0x20c + +0x208` as the x (odd modes) or y screen offset, the other
    /// 0.0; the phase advances by `+0x1fc`. Within the frames
    /// `MsScreenOffset(x, y)`, after them `MsScreenOffset(0, 0)` and off.
    pub fn rcef_screen_shake<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        call: &mut C,
    ) {
        use crate::model::bridge::{shake_fade, shake_random, shake_swing};
        use ffx_bridge::x87::{add, load, mul, store};
        if mem.read::<u32>(e.wrapping_add(0x214)) == 0 {
            return;
        }
        let mode = mem.read::<i32>(e.wrapping_add(0x1f8));
        if mode == 0 {
            return;
        }
        let mut swing = shake_swing(mem, mem.read(e.wrapping_add(0x210)));
        let frames = mem.read::<i32>(e.wrapping_add(0x200));
        if mode >= 3 && frames > 0 {
            let done = mem.read::<i32>(e.wrapping_add(0x204));
            swing = shake_fade(frames, done, swing);
        }
        let range = mem.read::<i32>(e.wrapping_add(0x20c));
        let base = mem.read::<i32>(e.wrapping_add(0x208));
        let amp = shake_random(mem, range, base, call);
        #[allow(clippy::cast_precision_loss)] // fild, then an f32 store
        let amp = load(store(f64::from(amp as f32)));
        let w = store(mul(amp, swing));
        let (x, y) = if mode & 1 != 0 { (w, 0) } else { (0, w) };
        let done = mem.read::<i32>(e.wrapping_add(0x204)).wrapping_add(1);
        mem.write(e.wrapping_add(0x204), done);
        let p = store(add(
            load(mem.read(e.wrapping_add(0x1fc))),
            load(mem.read(e.wrapping_add(0x210))),
        ));
        mem.write(e.wrapping_add(0x210), p);
        let f = mem.rebase(MS_SCREEN_OFFSET);
        if done <= mem.read::<i32>(e.wrapping_add(0x200)) {
            call(mem, f, &[x, y]);
        } else {
            call(mem, f, &[0, 0]);
            mem.write(e.wrapping_add(0x214), 0u32);
        }
    }

    // ---- round 28 ----

    /// `rcefBlurUpdate(e)` (`0x00924bf0`, from `rcefMainProcRunAfter`;
    /// name ours): while on (`+0x228`) with a mode (`+0x218`), the blur
    /// `+0x21c` (mode 2: faded, `(frames - done) * blur / frames` with the
    /// frames `+0x220` and done `+0x224`) to `Sg_Blur`; the count advances
    /// and after the frames `Sg_Blur(0)` turns it off.
    pub fn rcef_blur_update<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        e: u32,
        call: &mut C,
    ) {
        if mem.read::<u32>(e.wrapping_add(0x228)) == 0 {
            return;
        }
        let mode = mem.read::<u32>(e.wrapping_add(0x218));
        if mode == 0 {
            return;
        }
        let mut v = mem.read::<i32>(e.wrapping_add(0x21c));
        let frames = mem.read::<i32>(e.wrapping_add(0x220));
        if mode == 2 && frames > 0 {
            let done = mem.read::<i32>(e.wrapping_add(0x224));
            v = frames
                .wrapping_sub(done)
                .wrapping_mul(v)
                .wrapping_div(frames);
        }
        let done = mem.read::<i32>(e.wrapping_add(0x224)).wrapping_add(1);
        mem.write(e.wrapping_add(0x224), done);
        let f = mem.rebase(SG_BLUR);
        if done <= mem.read::<i32>(e.wrapping_add(0x220)) {
            call(mem, f, &[v.cast_unsigned()]);
        } else {
            call(mem, f, &[0]);
            mem.write(e.wrapping_add(0x228), 0u32);
        }
    }

    // ---- round 4: running tasks, objects, particle parts ----

    /// `rcefTaskProc(task)` (`0x0092ed90`): the current level's sequence
    /// (`call(mem, function, task)`, again while it sets [`RERUN`]), then
    /// the after-function unless the task is dead. Returns the `eax` it
    /// ends with: 0 without an after-function, the function when dead,
    /// else its result.
    pub fn rcef_task_proc<M: Mem>(
        mem: &mut M,
        t: u32,
        mut call: impl FnMut(&mut M, u32, u32) -> u32,
    ) -> u32 {
        loop {
            let l = level(mem, t);
            let f = mem.read::<u32>(t + header::FLAGS);
            mem.write(t + header::FLAGS, f & !RERUN);
            let seq = mem.read::<u32>(seq_slot(t, l));
            if seq != 0 {
                call(mem, seq, t);
            }
            if mem.read::<u32>(t + header::FLAGS) & RERUN == 0 {
                break;
            }
        }
        let after = mem.read::<u32>(t + header::AFTER);
        if after == 0 || mem.read::<u32>(t + header::FLAGS) & super::DEAD != 0 {
            return after;
        }
        call(mem, after, t)
    }

    /// `rcefTaskDestruct(task)` (`0x0092ec90`): its destructor
    /// (`call(mem, function, task)`, a tail call: its result), or the task
    /// itself without one.
    pub fn rcef_task_destruct<M: Mem>(
        mem: &mut M,
        t: u32,
        call: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        match mem.read::<u32>(t + header::DESTRUCTOR) {
            0 => t,
            f => call(mem, f, t),
        }
    }

    /// `rcEffectTaskPrayWait(task)` (`0x009304b0`): returns to the caller
    /// sequence (`rcefTaskRetSeqCont`) once `op_pray_effect_check` of the
    /// pray effect (`0x025d5a24`; `check`) gives 0.
    pub fn rc_effect_task_pray_wait<M: Mem>(
        mem: &mut M,
        t: u32,
        check: impl FnOnce(&mut M, u32) -> i32,
    ) {
        let e = PRAY_EFFECT.get(mem);
        if check(mem, e) == 0 {
            rcef_task_ret_seq_cont(mem, t);
        }
    }

    /// `rcefObjInit(obj)` (`0x0092ff20`): cleared (`0x490` bytes), a task
    /// with `rcefObjFini` as destructor and `rcefObjProc` after, scales
    /// and colours 1.0, three identity matrices, the particle part, the
    /// animation and the model (`+0x3c0`); returns the model (the
    /// `memset` `rcefModelInit` tail-calls returns it).
    pub fn rcef_obj_init(mem: &mut impl Mem, o: u32) -> u32 {
        mem.write_bytes(o, &[0; obj::SIZE as usize]);
        rcef_task_init(mem, o);
        let fini = mem.rebase(obj::FINI);
        mem.write(o + header::DESTRUCTOR, fini);
        let proc = mem.rebase(obj::PROC);
        mem.write(o + header::AFTER, proc);
        for (at, v) in obj::INIT {
            mem.write(o + at, v);
        }
        for m in obj::MATRICES {
            ffx_ps2::vu0::bridge::matrix_identity(mem, o + m);
        }
        rcef_par_init(mem, o + obj::PAR);
        rcef_anime_init(mem, o + obj::ANIME);
        rcef_model_init(mem, o + obj::MODEL);
        o + obj::MODEL
    }

    /// `rcefParCreate(par, data, index, a, b, c)` (`0x0092eee0`): a running
    /// particle part on entry `index` of `data` (`0x50` bytes each from
    /// `+0x20`), scales 1.0 and the three arguments; parts of some magic
    /// effects are remembered (`MagicFile_GetMagicId`: index 2 of 0x20e,
    /// 0x180, 0x168, 0x217; index 1 of 0xbf, 0x216). Returns `c`.
    pub fn rcef_par_create(
        mem: &mut impl Mem,
        p: u32,
        data: u32,
        index: u32,
        a: u32,
        b: u32,
        c: u32,
    ) -> u32 {
        let id = ffx_system::instance::bridge::magic_id(mem);
        let table: &[(u32, u32)] = match index {
            2 => &super::PAR_KEEP_2,
            1 => &super::PAR_KEEP_1,
            _ => &[],
        };
        if let Some(&(_, at)) = table.iter().find(|&&(m, _)| m == id) {
            mem.write(mem.rebase(at), p);
        }
        mem.write(
            p + 0xc,
            index
                .wrapping_mul(0x50)
                .wrapping_add(0x20)
                .wrapping_add(data),
        );
        mem.write(p + 8, index);
        mem.write(p, 1u32);
        mem.write(p + 4, data);
        mem.write(p + 0x78, 1f32);
        mem.write(p + 0x74, 1f32);
        mem.write(p + 0x70, 1f32);
        mem.write(p + 0x90, a);
        mem.write(p + 0x94, b);
        mem.write(p + 0x98, c);
        c
    }

    /// `rcefMainSetPar(main, obj, data_index, index)` (`0x009256d0`):
    /// [`rcef_par_create`] on the object's particle part with the main
    /// data's entry (`+0x18` table), its matrix `+0x1f0`, scale `+0x160`
    /// and colour `+0x170`; returns the last.
    pub fn rcef_main_set_par(
        mem: &mut impl Mem,
        main: u32,
        o: u32,
        data_index: u32,
        index: u32,
    ) -> u32 {
        let table = mem.read::<u32>(main.wrapping_add(0x18));
        let data = mem.read::<u32>(table.wrapping_add(data_index.wrapping_mul(4)));
        rcef_par_create(
            mem,
            o.wrapping_add(obj::PAR),
            data,
            index,
            o.wrapping_add(0x1f0),
            o.wrapping_add(0x160),
            o.wrapping_add(0x170),
        )
    }

    /// `rcefTaskKill(t)` (`0x0092ed50`): `t` unlinked from its list, its
    /// destructor run ([`rcef_task_destruct`]) and its memory returned to
    /// the current effect heap (`pppMemFree(heap, heap + 0x10, t)`).
    pub fn rcef_task_kill<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) -> u32 {
        let (prev, next) = (mem.read::<u32>(t + 4), mem.read::<u32>(t + 8));
        mem.write(prev.wrapping_add(8), next);
        let (prev, next) = (mem.read::<u32>(t + 4), mem.read::<u32>(t + 8));
        mem.write(next.wrapping_add(4), prev);
        rcef_task_destruct(mem, t, |m, f, a| call(m, f, &[a]));
        let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
        let f = mem.rebase(super::PPP_MEM_FREE);
        call(mem, f, &[heap, heap.wrapping_add(0x10), t])
    }

    /// `rcefChrInit(c)` (`0x00930770`): an effect's character draw block:
    /// no character, a reference GIF tag (`+0x10` 0x1c, `+0x14` from
    /// `0x00c498dc`, `+0x18` 0x44), grey, its two matrices (`+0x80`,
    /// `+0xc0`, the identity) and the light's colour and normal matrices
    /// (`Sg_LightGet*MatrixPtr`). Returns the identity's `eax` (1.0).
    pub fn rcef_chr_init<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) -> u32 {
        mem.write(c, 0u32);
        mem.write(c + 4, 0u32);
        mem.write(c + 8, 0u32);
        mem.write_bytes(c + 0x10, &[0; 0x70]);
        mem.write(c + 0x10, 0x1cu32);
        let tag = mem.read::<u32>(mem.rebase(crate::model::REF_GIF_TAG));
        mem.write(c + 0x14, tag);
        mem.write(c + 0x18, 0x44u8);
        mem.write(c + 0x28, 0x8080_8080u32);
        mem.write(c + 0x30, c + 0x80);
        let f = mem.rebase(super::LIGHT_COLOR_MATRIX);
        let m = call(mem, f, &[]);
        mem.write(c + 0x34, m);
        let f = mem.rebase(super::LIGHT_NORMAL_MATRIX);
        let m = call(mem, f, &[]);
        mem.write(c + 0x38, m);
        mem.write(c + 0x3c, c + 0xc0);
        mem.write(c + 0x5c, 0x8080u16);
        ffx_ps2::vu0::bridge::matrix_identity(mem, c + 0x80);
        ffx_ps2::vu0::bridge::matrix_identity(mem, c + 0xc0)
    }

    /// `rcefWorkSwap(w)` (`0x00925530`): the quadwords at `+0x1c4` and
    /// `+0x1d4` swapped (`rcCopyQword` through `tmp`, the original's 16
    /// stack bytes), `+0x1e8` 0, `+0x1f0` 1.
    pub fn rcef_work_swap(mem: &mut impl Mem, w: u32, tmp: u32) {
        use crate::vu0::bridge::rc_copy_qword;
        rc_copy_qword(mem, tmp, w + 0x1d4);
        rc_copy_qword(mem, w + 0x1d4, w + 0x1c4);
        rc_copy_qword(mem, w + 0x1c4, tmp);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
    }

    /// `rcefWorkSet(w, a, b, v)` (`0x00925630`): `+0x1c0` 1, `a` and `b`
    /// to `+0x1c4`/`+0x1d4` (`rcCopyQword`), `v` at `+0x1e4`, `+0x1e8` 0,
    /// `+0x1f0` 1.
    pub fn rcef_work_set(mem: &mut impl Mem, w: u32, a: u32, b: u32, v: u32) {
        use crate::vu0::bridge::rc_copy_qword;
        mem.write(w + 0x1c0, 1u32);
        rc_copy_qword(mem, w + 0x1c4, a);
        rc_copy_qword(mem, w + 0x1d4, b);
        mem.write(w + 0x1e4, v);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
    }

    /// `rcefRunPart(env, part, arg)` (`0x0092eec0`): `env` made the current
    /// effect heap (`*g_pppEnvSlot`), then `pppRunPart(part, arg)`.
    pub fn rcef_run_part<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        env: u32,
        part: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, env);
        let f = mem.rebase(super::PPP_RUN_PART);
        call(mem, f, &[part, arg])
    }

    /// `rcefObjNew()` (`0x00930030`): 0x490 bytes from the current effect
    /// heap (`pppMemAlloc(heap, heap + 0x10, 0x490)`), [`rcef_obj_init`].
    pub fn rcef_obj_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
        let f = mem.rebase(super::PPP_MEM_ALLOC);
        let o = call(mem, f, &[heap, heap.wrapping_add(0x10), 0x490]);
        rcef_obj_init(mem, o);
        o
    }

    /// `rcefTaskCtrlFini(ctrl)` (`0x00930260`): every task of the list
    /// (`+0xc0` the first, back at `ctrl` when empty) killed
    /// ([`rcef_task_kill`]).
    pub fn rcef_task_ctrl_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctrl: u32,
        call: &mut C,
    ) {
        loop {
            let t = mem.read::<u32>(ctrl + 0xc0);
            if t == ctrl {
                break;
            }
            rcef_task_kill(mem, t, call);
        }
    }

    /// `rcefTaskCtrlProc(first)` (`0x00930300`): each task from `first`
    /// along `+0x8` run ([`rcef_task_proc`]), then the dead ones (flag 4)
    /// killed.
    pub fn rcef_task_ctrl_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        first: u32,
        call: &mut C,
    ) {
        if first == 0 {
            return;
        }
        let mut t = first;
        while t != 0 {
            rcef_task_proc(mem, t, |m, f, a| call(m, f, &[a]));
            t = mem.read::<u32>(t + 8);
        }
        let mut t = first;
        while t != 0 {
            let next = mem.read::<u32>(t + 8);
            if mem.read::<u8>(t) & 4 != 0 {
                rcef_task_kill(mem, t, call);
            }
            t = next;
        }
    }

    /// `rcefChrFini(c)` (`0x00930710`): the character (`+0x4`) gets its
    /// CLUT and image bases back (`Ch_TextureSetClutBase`/`SetImageBase`
    /// with -1, -1) and effects off (`Ch_DrawEnEffect(chr, 0)`); the
    /// block at `+0x8` goes back to the effect heap (`pppMemFree`) and is
    /// cleared.
    pub fn rcef_chr_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) {
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_TEXTURE_SET_CLUT_BASE);
        call(mem, f, &[chr, u32::MAX, u32::MAX]);
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_TEXTURE_SET_IMAGE_BASE);
        call(mem, f, &[chr, u32::MAX, u32::MAX]);
        let chr = mem.read::<u32>(c + 4);
        let f = mem.rebase(super::CH_DRAW_EN_EFFECT);
        call(mem, f, &[chr, 0]);
        let p = mem.read::<u32>(c + 8);
        if p != 0 {
            let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
            let f = mem.rebase(super::PPP_MEM_FREE);
            call(mem, f, &[heap, heap.wrapping_add(0x10), p]);
        }
        mem.write(c + 8, 0u32);
    }

    /// `rcefMainCreateObj(main)` (`0x00925140`): [`rcef_obj_new`], its init
    /// once more ([`rcef_obj_init`]), then added to the main's task list
    /// (`+0x1c`, [`rcef_task_ctrl_add`]). Returns the object.
    pub fn rcef_main_create_obj<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        call: &mut C,
    ) -> u32 {
        let o = rcef_obj_new(mem, call);
        rcef_obj_init(mem, o);
        rcef_task_ctrl_add(mem, m + 0x1c, o);
        o
    }

    /// `rcefMainFini(main)` (`0x009251c0`): its environment current, the
    /// block at `+0x18` back to the heap (`pppMemFree`) and cleared, then
    /// [`rcef_task_ctrl_fini`] of its list (`+0x1c`).
    pub fn rcef_main_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        call: &mut C,
    ) {
        let slot = super::main::ENV_SLOT.get(mem);
        mem.write(slot, mem.read::<u32>(m + super::main::ENV));
        let p = mem.read::<u32>(m + 0x18);
        if p != 0 {
            let heap = mem.read::<u32>(super::main::ENV_SLOT.get(mem));
            let f = mem.rebase(super::PPP_MEM_FREE);
            call(mem, f, &[heap, heap.wrapping_add(0x10), p]);
            mem.write(m + 0x18, 0u32);
        }
        rcef_task_ctrl_fini(mem, m + 0x1c, call);
    }

    /// `rcefWorkSetFromVf00(w, b, v)` (`0x009255d0`): `+0x1c0` 1, `vf00` to
    /// `+0x1c4`, `b` to `+0x1d4` (`rcCopyQword`), `v` at `+0x1e4`, `+0x1e8`
    /// 0, `+0x1f0` 1, `+0x1f4` 0.
    pub fn rcef_work_set_from_vf00(mem: &mut impl Mem, w: u32, b: u32, v: u32) {
        mem.write(w + 0x1c0, 1u32);
        crate::vu0::bridge::rcef_get_vf00(mem, w + 0x1c4);
        crate::vu0::bridge::rc_copy_qword(mem, w + 0x1d4, b);
        mem.write(w + 0x1e4, v);
        mem.write(w + 0x1e8, 0u32);
        mem.write(w + 0x1f0, 1u32);
        mem.write(w + 0x1f4, 0u32);
    }

    // ---- round 13 ----

    /// `rcefMainKickPar(main, a, b, chr, bone, p0, p1, p2, flags, force)`
    /// (`0x009253a0`): a new object when the main's task has ended
    /// (`rcefMainCreateObj`), else its object (`+0x24`), unless that one's
    /// particles still run and `force` is 0 (returns 0 then); then
    /// `rcefMainSetPar(main, obj, a, b)`, `rcefObjAttachChr(obj, chr,
    /// bone)`, the three vectors to `+0xd0`, `+0x100`, `+0x130` and
    /// `flags` into bits 4..12 of `+0xbc`. Returns the new `+0xbc`.
    #[allow(clippy::many_single_char_names)] // the original's arguments
    pub fn rcef_main_kick_par<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 10],
        call: &mut C,
    ) -> u32 {
        let [m, a, b, chr, bone, p0, p1, p2, flags, force] = args;
        let f = mem.rebase(super::TASK_CTRL_IS_END);
        let obj = if call(mem, f, &[m.wrapping_add(0x1c)]) != 0 {
            let f = mem.rebase(super::MAIN_CREATE_OBJ);
            call(mem, f, &[m])
        } else {
            let obj = mem.read::<u32>(m.wrapping_add(0x24));
            if force == 0 {
                let f = mem.rebase(super::OBJ_IS_PAR_END);
                if call(mem, f, &[obj]) == 0 {
                    return 0;
                }
            }
            obj
        };
        let f = mem.rebase(super::MAIN_SET_PAR);
        call(mem, f, &[m, obj, a, b]);
        let f = mem.rebase(super::OBJ_ATTACH_CHR);
        call(mem, f, &[obj, chr, bone]);
        let f = mem.rebase(super::COPY_QWORD);
        call(mem, f, &[obj.wrapping_add(0xd0), p0]);
        call(mem, f, &[obj.wrapping_add(0x100), p1]);
        call(mem, f, &[obj.wrapping_add(0x130), p2]);
        let at = obj.wrapping_add(0xbc);
        let v = (mem.read::<u32>(at) & 0xffff_e00f) | flags;
        mem.write(at, v);
        v
    }

    // ---- field agent r1, group B: the character helpers of `rcefUtil` ----

    /// `rcefGetChrHandle(i)` (`0x0092e1c0`): `MsGetChrHandle(i)` in a battle
    /// (`rcefIsBattle`), else `AtelGetCharHandleFromBindPC(i)` (both tail
    /// calls); returns its `eax`.
    pub fn rcef_get_chr_handle<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        call: &mut C,
    ) -> u32 {
        let f = if rcef_is_battle(mem) {
            super::MS_GET_CHR_HANDLE
        } else {
            super::ATEL_CHAR_FROM_BIND_PC
        };
        let f = mem.rebase(f);
        call(mem, f, &[i])
    }

    /// `rcefGetChrElemNo(i, n)` (`0x0092e000`, Switch name by layout):
    /// `Ch_OptposGetElement(handle, n, 0)`, the element of the
    /// character's option position `n`.
    pub fn rcef_get_chr_elem_no<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        n: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_OPTPOS_GET_ELEMENT);
        call(mem, f, &[h, n, 0])
    }

    /// `rcefGetChrMatrix(i, e, out)` (`0x0092e240`): `Ch_ElemGetMatrix`
    /// of element `e`, then `rcefScaleMatrix100(out)`.
    pub fn rcef_get_chr_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        e: u32,
        out: u32,
        call: &mut C,
    ) {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[h, e, out]);
        let f = mem.rebase(super::SCALE_MATRIX_100);
        call(mem, f, &[out]);
    }

    /// `rcefGetChrMatrixOpt(i, n, out)` (`0x0092e270`): [`rcef_get_chr_matrix`]
    /// of option position `n`'s element ([`rcef_get_chr_elem_no`]).
    pub fn rcef_get_chr_matrix_opt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        n: u32,
        out: u32,
        call: &mut C,
    ) {
        let e = rcef_get_chr_elem_no(mem, i, n, call);
        rcef_get_chr_matrix(mem, i, e, out, call);
    }

    /// `rcefSetTargets(e, opt, te, topt)` (`0x00924a20`, name ours): the
    /// effect target table ([`super::target::TABLE`], 600 bytes from it
    /// cleared) from the caster ([`super::target::CASTER`]) and its
    /// listed targets ([`super::target::SOURCES`], at most 16 of
    /// [`super::target::SOURCE_COUNT`], kept as an `i16` at
    /// [`super::target::COUNT`]). The caster's facing
    /// (`rcefDirToAngle(rcefGetChrDir(..))`) goes to
    /// [`super::target::CASTER_ANGLE`], its position (element `e`, or
    /// option position `e` with `opt`: `rcefGetChrPos`/`rcefGetChrPosOpt`)
    /// to `work`. Each target `i` gets its character at `+4`, its position
    /// (element `te`, with `topt` an option position) added to the centre
    /// ([`super::target::CENTRE`], `rcVu0Add_00924770`), and the offset
    /// from the caster (`rcVu0Sub_00924860` at `+0x14`) split: `+0x10`
    /// its y, `+0x18`/`+0x20` 0, `+8` the length (`rcVec3fLength`), then
    /// normalised and `+0xc` its angle (`rcefVecToAngle`). The centre is
    /// scaled by `1 / count` (`rcbgScaleVector_009247d0`), whose `eax` it
    /// returns. `work` holds the two positions (0x20 bytes).
    pub fn rcef_set_targets<M, C, F>(
        mem: &mut M,
        [e, opt, te, topt]: [u32; 4],
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        use super::target as t;
        use crate::camera::FArgs;
        use crate::x87;
        let table = t::TABLE.ptr(mem).addr;
        for i in 0..600 / 4 {
            mem.write(table + 4 * i, 0u32);
        }
        #[allow(clippy::cast_possible_truncation)] // the low word
        let count = (t::SOURCE_COUNT.get(mem) as u16).cast_signed().min(16);
        t::COUNT.set(mem, count);
        let caster = t::CASTER.get(mem);
        let f = mem.rebase(t::GET_CHR_DIR);
        let dir = x87::store(fcall(mem, f, FArgs::Stack(&[caster])));
        let f = mem.rebase(t::DIR_TO_ANGLE);
        let angle = x87::store(fcall(mem, f, FArgs::Stack(&[dir])));
        mem.write(t::CASTER_ANGLE.ptr(mem).addr, angle);
        let get_pos = |opt: u32| {
            if opt == 0 {
                t::GET_CHR_POS
            } else {
                t::GET_CHR_POS_OPT
            }
        };
        let (from, to) = (work, work.wrapping_add(0x10));
        let caster = t::CASTER.get(mem);
        let f = mem.rebase(get_pos(opt));
        call(mem, f, &[caster, e, from]);
        let centre = t::CENTRE.ptr(mem).addr;
        let mut i = 0i16;
        while i < t::COUNT.get(mem) {
            let k = u32::from(i.cast_unsigned());
            let entry = table + t::SIZE * k;
            let chr = mem.read::<u32>(t::SOURCES.ptr(mem).addr + 4 * k);
            mem.write(entry + 4, chr);
            let f = mem.rebase(get_pos(topt));
            call(mem, f, &[chr, te, to]);
            let f = mem.rebase(t::VU0_ADD);
            call(mem, f, &[centre, centre, to]);
            let v = entry + 0x14;
            let f = mem.rebase(t::VU0_SUB);
            call(mem, f, &[v, to, from]);
            let y = x87::copy(mem.read::<u32>(entry + 0x18));
            mem.write(entry + t::HEIGHT, y);
            mem.write(entry + 0x20, 0u32);
            mem.write(entry + 0x18, 0u32);
            let f = mem.rebase(t::VEC3F_LENGTH);
            let len = x87::store(fcall(mem, f, FArgs::Stack(&[v])));
            mem.write(entry + t::DIST, len);
            let f = mem.rebase(super::VEC3F_NORMALIZE);
            call(mem, f, &[v]);
            let f = mem.rebase(t::VEC_TO_ANGLE);
            let a = x87::store(fcall(mem, f, FArgs::Stack(&[v])));
            mem.write(entry + t::ANGLE, a);
            i += 1;
        }
        let n = f64::from(t::COUNT.get(mem));
        let scale = x87::store(1.0 / n);
        let f = mem.rebase(t::SCALE_VECTOR);
        call(mem, f, &[centre, centre, scale])
    }

    /// `rcefGetChrPos(i, e, out)` (`0x0092e2f0`): element `e`'s matrix
    /// (`Ch_ElemGetMatrix` into `work`, 0x40 bytes: the original's
    /// frame) and its translation row copied to `out`
    /// (`rcCopyQword_0092d6f0`, whose `eax` it returns).
    pub fn rcef_get_chr_pos<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        e: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[h, e, work]);
        let f = mem.rebase(super::COPY_QWORD_D6F0);
        call(mem, f, &[out, work.wrapping_add(0x30)])
    }

    /// `rcefGetChrPosY0(i, e, out)` (`0x0092e380`): [`rcef_get_chr_pos`],
    /// then `out.y` = 0.0.
    pub fn rcef_get_chr_pos_y0<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        e: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let r = rcef_get_chr_pos(mem, i, e, out, work, call);
        mem.write(out.wrapping_add(4), 0u32);
        r
    }

    /// `rcefGetChrPosOpt(i, n, out)` (`0x0092e340`): [`rcef_get_chr_pos`]
    /// of option position `n`'s element.
    pub fn rcef_get_chr_pos_opt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        n: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let e = rcef_get_chr_elem_no(mem, i, n, call);
        rcef_get_chr_pos(mem, i, e, out, work, call)
    }

    /// `rcefGetChrPosOptY0(i, n, out)` (`0x0092e360`): [`rcef_get_chr_pos_y0`]
    /// of option position `n`'s element.
    pub fn rcef_get_chr_pos_opt_y0<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        n: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let e = rcef_get_chr_elem_no(mem, i, n, call);
        rcef_get_chr_pos_y0(mem, i, e, out, work, call)
    }

    /// `rcefGetChrWidth(i)` (`0x0092e3a0`): `Ch_GetThickness` times 10.0
    /// (an `f64` in `.rdata`), stored as `f32`.
    pub fn rcef_get_chr_width<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        call: &mut C,
    ) -> f32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let t = ffx_chara::chr::bridge::ch_get_thickness(&*mem, h);
        let k = super::TEN.get(mem);
        f32::from_bits(crate::x87::store(f64::from(t) * k))
    }

    /// `rcefResetChrBaseMatrix(i)` (`0x0092e730`): `Ch_SetOrgMatrix(handle,
    /// NULL)`.
    pub fn rcef_reset_chr_base_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        call: &mut C,
    ) {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_SET_ORG_MATRIX);
        call(mem, f, &[h, 0]);
    }

    /// `rcefCreateChrClut(i)` (`0x0092deb0`): `Ch_ClutCreate(handle)` (a
    /// tail call); returns its `eax`.
    pub fn rcef_create_chr_clut<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_CLUT_CREATE);
        call(mem, f, &[h])
    }

    /// `rcefDisposeChrClut(i)` (`0x0092df10`): `Ch_ClutDispose(handle)` (a
    /// tail call); returns its `eax`.
    pub fn rcef_dispose_chr_clut<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_CLUT_DISPOSE);
        call(mem, f, &[h])
    }

    /// `rcefGetChrElemPos(i, e, v, out)` (`0x0092e0a0`): element `e`'s
    /// matrix into `work` (0x40 bytes), `v` scaled in place by 1000.0
    /// (`rcbgScaleVector_0092d690`), `v.w` = 1.0, then
    /// `FFXVu0ApplyMatrix(out, work, v)`, whose `eax` it returns.
    pub fn rcef_get_chr_elem_pos<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, e, v, out]: [u32; 4],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[h, e, work]);
        let k = crate::x87::copy(super::THOUSAND_F32.get(mem));
        let f = mem.rebase(super::SCALE_VECTOR_D690);
        call(mem, f, &[v, v, k]);
        mem.write(v.wrapping_add(0xc), 0x3f80_0000u32);
        let f = mem.rebase(super::APPLY_MATRIX);
        call(mem, f, &[out, work, v])
    }

    /// `rcefGetChrEyeHeight(i)` (`0x0092e110`): option positions 0 and 2
    /// (`Ch_OptposGet` into `work`, 0x20 bytes), (y0 - y2) times 10.0,
    /// stored as `f32`.
    pub fn rcef_get_chr_eye_height<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        work: u32,
        call: &mut C,
    ) -> f32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_OPTPOS_GET);
        let (a, b) = (work.wrapping_add(0x10), work);
        call(mem, f, &[h, 0, a]);
        call(mem, f, &[h, 2, b]);
        let ya = crate::x87::load(mem.read::<u32>(a.wrapping_add(4)));
        let yb = crate::x87::load(mem.read::<u32>(b.wrapping_add(4)));
        let k = super::TEN.get(mem);
        f32::from_bits(crate::x87::store((ya - yb) * k))
    }

    /// `rcefGetChrElemOff(i, e, out)` (`0x0092e020`): the matrices of
    /// element `e` and of option position 0's element (into `work`, 0x80
    /// bytes: `e`'s first), `out` = their translation rows' difference
    /// (`rcVu0Sub_0092d720(out, e's, option's)`) times 10.0
    /// (`rcbgScaleVector_0092d690`, whose `eax` it returns).
    pub fn rcef_get_chr_elem_off<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, e, out]: [u32; 3],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let n = rcef_get_chr_elem_no(mem, i, 0, call);
        let (m1, m2) = (work, work.wrapping_add(0x40));
        let f = mem.rebase(super::CH_ELEM_GET_MATRIX);
        call(mem, f, &[h, e, m1]);
        call(mem, f, &[h, n, m2]);
        let f = mem.rebase(super::VU0_SUB_D720);
        call(mem, f, &[out, m1.wrapping_add(0x30), m2.wrapping_add(0x30)]);
        let k = crate::x87::copy(super::TEN_F32.get(mem));
        let f = mem.rebase(super::SCALE_VECTOR_D690);
        call(mem, f, &[out, out, k])
    }

    /// `rcefGetChrFrontVec(i, out)` (`0x0092e170`): option position 0's
    /// matrix ([`rcef_get_chr_matrix_opt`] into `work`, 0x40 bytes), its
    /// third row copied to `out` and normalised (`rcVec3fNormalize`).
    pub fn rcef_get_chr_front_vec<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) {
        rcef_get_chr_matrix_opt(mem, i, 0, work, call);
        let f = mem.rebase(super::COPY_QWORD_D6F0);
        call(mem, f, &[out, work.wrapping_add(0x20)]);
        let f = mem.rebase(super::VEC3F_NORMALIZE);
        call(mem, f, &[out]);
    }

    /// `rcefObjAttachChrOpt(obj, i, n)` (`0x0092f9a0`):
    /// `rcefObjAttachChr(obj, i, Ch_OptposGetElement(handle, n, 0))` (a
    /// tail call).
    pub fn rcef_obj_attach_chr_opt<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        i: u32,
        n: u32,
        call: &mut C,
    ) {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_OPTPOS_GET_ELEMENT);
        let e = call(mem, f, &[h, n, 0]);
        let f = mem.rebase(super::OBJ_ATTACH_CHR);
        call(mem, f, &[obj, i, e]);
    }

    /// `rcefChgChrClut(i, add, mul)` (`0x0092dd30`): every colour of every
    /// CLUT of the character (`Ch_ClutGetNb`, `Ch_ClutGetSize`), from its
    /// original (`Ch_ClutGetOrgPtr`) into the live one (`Ch_ClutGetPtr`):
    /// channel `j` = (`add[j]` + original) * `mul[j]` (`f32` arrays of
    /// four), `_ftol2_sse` and clamped to 0..=255. Returns what the
    /// original leaves in `eax`: the count when not positive, else the last
    /// CLUT's `mul` pointer when it has colours, or its `Ch_ClutGetPtr`.
    pub fn rcef_chg_chr_clut<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        i: u32,
        add: u32,
        mul: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let f = mem.rebase(super::CH_CLUT_GET_NB);
        let nb = call(mem, f, &[h]).cast_signed();
        let mut eax = nb.cast_unsigned();
        for c in 0..nb.max(0).cast_unsigned() {
            let f = mem.rebase(super::CH_CLUT_GET_SIZE);
            let n = call(mem, f, &[h, c]).cast_signed();
            let f = mem.rebase(super::CH_CLUT_GET_ORG_PTR);
            let mut org = call(mem, f, &[h, c]);
            let f = mem.rebase(super::CH_CLUT_GET_PTR);
            let mut ptr = call(mem, f, &[h, c]);
            eax = ptr;
            for _ in 0..n.max(0) {
                for j in 0..4u32 {
                    let o = f64::from(mem.read::<u8>(org.wrapping_add(j)));
                    let a = crate::x87::load(mem.read::<u32>(add.wrapping_add(4 * j)));
                    let m = crate::x87::load(mem.read::<u32>(mul.wrapping_add(4 * j)));
                    let v = ffx_bridge::x87::ftol(&*mem, (a + o) * m).cast_signed();
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
                    mem.write(ptr.wrapping_add(j), v.clamp(0, 0xff) as u8);
                }
                org = org.wrapping_add(4);
                ptr = ptr.wrapping_add(4);
                eax = mul;
            }
        }
        eax
    }

    // ---- field agent r1, group C: an effect object's matrices ----

    /// `rcefObjCalcParentMatrix(obj, out)` (`0x0092fc60`): the matrix an
    /// effect object hangs from, by its flags (`+0xbc`):
    /// - neither 2 nor 4: the parent object's world matrix (`+0xc0`,
    ///   its `+0x1f0`; `FFXVu0CopyMatrix`), or the identity without one;
    /// - 2: the character's (`rcefGetChrMatrix(+0x270, +0x274)`); 4 alone:
    ///   the display object's (`yiGetMatrixDObj(+0x270)`);
    /// - then 0x20: row 2 flattened (y = 0) and normalised; all zero gives
    ///   the identity, else row 2 is it, row 0 = row 2 x (0, 1, 0) and
    ///   row 1 = row 0 x row 2 (`rcVec3fMul`);
    /// - 0x10: the identity turned about y by -`Ch_GetRot` * 651.9 - 1024
    ///   (`.rdata` `f64`s) and moved to the old translation
    ///   (`op_rt_m_yxz`);
    /// - 0x40: the rotation rows reset to the identity;
    /// - without 0x80: rows 0..2 normalised.
    ///
    /// `work` is 0x20 bytes (the original's frame: two vectors).
    #[allow(clippy::too_many_lines)] // one flag after another
    pub fn rcef_obj_calc_parent_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        out: u32,
        work: u32,
        call: &mut C,
    ) {
        let flags = |mem: &M| mem.read::<u32>(obj.wrapping_add(0xbc));
        let unit = mem.rebase(super::UNIT_MATRIX);
        let fl = flags(mem);
        if fl & 6 == 0 {
            let p = mem.read::<u32>(obj.wrapping_add(0xc0));
            if p == 0 {
                call(mem, unit, &[out]);
            } else {
                let f = mem.rebase(super::COPY_MATRIX);
                call(mem, f, &[out, p.wrapping_add(0x1f0)]);
            }
        } else {
            let a = mem.read::<u32>(obj.wrapping_add(0x270));
            if fl & 2 == 0 {
                let f = mem.rebase(super::YI_GET_MATRIX_DOBJ);
                call(mem, f, &[a, out]);
            } else {
                let e = mem.read::<u32>(obj.wrapping_add(0x274));
                rcef_get_chr_matrix(mem, a, e, out, call);
            }
        }
        let copy = mem.rebase(super::COPY_QWORD_F850);
        let norm = mem.rebase(super::VEC3F_NORMALIZE);
        let (t, v) = (work.wrapping_add(0x10), work);
        if flags(mem) & 0x20 != 0 {
            let row2 = out.wrapping_add(0x20);
            call(mem, copy, &[t, row2]);
            mem.write(t.wrapping_add(4), 0u32);
            call(mem, norm, &[t]);
            let zero =
                |mem: &M, k: u32| crate::x87::load(mem.read::<u32>(t.wrapping_add(4 * k))) == 0.0;
            #[allow(clippy::float_cmp)] // fucom: exact
            if zero(mem, 0) && zero(mem, 1) && zero(mem, 2) {
                call(mem, unit, &[out]);
            } else {
                call(mem, copy, &[row2, t]);
                for (k, w) in (0u32..).zip([0u32, 0x3f80_0000, 0, 0]) {
                    mem.write(v.wrapping_add(4 * k), w);
                }
                let mul = mem.rebase(super::VEC3F_MUL);
                call(mem, mul, &[out, t, v]);
                call(mem, mul, &[out.wrapping_add(0x10), out, row2]);
            }
        }
        if flags(mem) & 0x10 != 0 {
            let (s, a) = (v, t);
            call(mem, copy, &[s, out.wrapping_add(0x30)]);
            for k in 0..4 {
                mem.write(a.wrapping_add(4 * k), 0u32);
            }
            let i = mem.read::<u32>(obj.wrapping_add(0x270));
            let h = rcef_get_chr_handle(mem, i, call);
            let rot = f64::from(ffx_chara::chr::bridge::ch_get_rot(&*mem, h));
            let (k, b) = (super::ROT_SCALE.get(mem), super::ROT_BIAS.get(mem));
            mem.write(a.wrapping_add(4), crate::x87::store(-rot * k - b));
            call(mem, unit, &[out]);
            let f = mem.rebase(super::OP_RT_M_YXZ);
            call(mem, f, &[out, out, a, s]);
        }
        if flags(mem) & 0x40 != 0 {
            for k in 0..12u32 {
                let w = if k % 5 == 0 { 0x3f80_0000u32 } else { 0 };
                mem.write(out.wrapping_add(4 * k), w);
            }
        }
        if flags(mem) & 0x80 == 0 {
            for r in 0..3u32 {
                call(mem, norm, &[out.wrapping_add(0x10 * r)]);
            }
        }
    }

    /// `rcefObjCalcWorldMatrix(obj)` (`0x0092fe30`): without flag 0x1000,
    /// or with 0x100, the parent matrix into `+0x230`
    /// ([`rcef_obj_calc_parent_matrix`]) and flag 0x1000 set; then
    /// `+0x1f0` = `+0x230` x `+0x1b0` (`FFXVu0MulMatrix`, whose `eax` it
    /// returns).
    pub fn rcef_obj_calc_world_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        obj: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let at = obj.wrapping_add(0xbc);
        let fl = mem.read::<u32>(at);
        if fl & 0x1000 == 0 || fl & 0x100 != 0 {
            rcef_obj_calc_parent_matrix(mem, obj, obj.wrapping_add(0x230), work, call);
            let v = mem.read::<u32>(at) | 0x1000;
            mem.write(at, v);
        }
        let f = mem.rebase(super::MUL_MATRIX);
        call(
            mem,
            f,
            &[
                obj.wrapping_add(0x1f0),
                obj.wrapping_add(0x230),
                obj.wrapping_add(0x1b0),
            ],
        )
    }

    // ---- field agent r2, group D: an effect object's frame ----

    /// The six particle parts `rcefParCreate` keeps (`g_RcefParKept`) and
    /// the life (`+0x18` of the part body, `> ` when `true`, else `>=`)
    /// past which `rcefParProc` lets each go.
    pub const PAR_KEPT: [(u32, u32, bool); 6] = [
        (0x0193_c0d0, 0x8000, false),
        (0x0193_c0d8, 0x2_6000, true),
        (0x0193_c0dc, 0x7000, false),
        (0x0193_c0e0, 0x3_2000, true),
        (0x0193_c0e4, 0xa000, false),
        (0x0193_c0e8, 0x5_0000, false),
    ];

    /// `rcefParProc(par)` (`0x0092f010`): for a live part (bit 0): the ppp
    /// environment's three data pointers (`get_ptr32_func` of the data's
    /// `+0x14..+0x1c`), the part started once (bit 1: `pppStartPart`),
    /// `pppSetMatrix`, the world matrix from `+0x90`, the scale of
    /// `+0x94` by `+0x9c..` into `0x0230fd50..`, the colour words of
    /// `+0x98` (times 4096, toward zero, negative to 0) at `+0x80`, the
    /// world matrix's row lengths at `+0x70..`, the view matrices, then
    /// `rcefRunPart`; a finished part (or a kept one past its life,
    /// [`PAR_KEPT`], which is forgotten) is freed (`pppAllFreePObject`)
    /// and its flags cleared.
    #[allow(clippy::too_many_lines)] // the original's steps in turn
    pub fn rcef_par_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) {
        if mem.read::<u8>(p) & 1 == 0 {
            return;
        }
        let at = |o: u32| p.wrapping_add(o);
        let body = at(0x10);
        let gp = mem.rebase(super::GET_PTR32);
        for (k, o) in [0x14u32, 0x18, 0x1c].into_iter().enumerate() {
            let d = mem.read::<u32>(at(4));
            let v = call(mem, gp, &[mem.read::<u32>(d.wrapping_add(o))]);
            let env = env_now(mem);
            #[allow(clippy::cast_possible_truncation)] // 0..3
            mem.write(env.wrapping_add(0x20 + 4 * k as u32), v);
        }
        if mem.read::<u8>(p) & 2 == 0 {
            let d = mem.read::<u32>(at(4));
            let t = call(mem, gp, &[mem.read::<u32>(d.wrapping_add(0x10))]);
            let i = mem.read::<u32>(at(8));
            let v = mem.read::<u32>(t.wrapping_add(i.wrapping_mul(4)));
            let f = mem.rebase(super::PPP_START_PART);
            call(mem, f, &[body, 0x1000, v, 1, 0]);
            let w = mem.read::<u32>(p) | 2;
            mem.write(p, w);
        }
        let f = mem.rebase(super::PPP_SET_MATRIX);
        call(mem, f, &[body]);
        let world = mem.rebase(super::PPP_WORLD_MATRIX);
        let m = mem.read::<u32>(at(0x90));
        if m != 0 {
            let f = mem.rebase(super::COPY_MATRIX);
            call(mem, f, &[world, m]);
        }
        let ld = |mem: &M, a: u32| crate::x87::load(mem.read::<u32>(a));
        if mem.read::<u32>(at(0x94)) != 0 {
            let dst = mem.rebase(super::PPP_SCALE);
            for k in 0..3u32 {
                let q = mem.read::<u32>(at(0x94));
                let v =
                    crate::x87::store(ld(mem, at(0x9c + 4 * k)) * ld(mem, q.wrapping_add(4 * k)));
                mem.write(dst.wrapping_add(4 * k), v);
            }
        }
        if mem.read::<u32>(at(0x98)) != 0 {
            let k = super::PAR_COLOUR_SCALE.get(mem);
            for c in 0..4u32 {
                let s = mem.read::<u32>(at(0x98));
                let x = ld(mem, s.wrapping_add(4 * c));
                // fcom 0.0: negative stays 0; NaN converts (to 0x8000_0000)
                #[allow(clippy::cast_possible_truncation)] // the low word, as the original stores
                let v = if x < 0.0 {
                    0
                } else {
                    ffx_bridge::x87::cvttsd2si(x * k) as u16
                };
                mem.write(at(0x80 + 2 * c), v);
            }
        }
        for (k, row) in [0u32, 0x10, 0x20].into_iter().enumerate() {
            let len = crate::geom::bridge::rc_vec3f_length(&*mem, world.wrapping_add(row));
            #[allow(clippy::cast_possible_truncation)] // 0..3
            mem.write(at(0x70 + 4 * k as u32), len.to_bits());
        }
        let (view, cam, copy) = (
            mem.rebase(super::PPP_VIEW_MATRIX),
            mem.rebase(super::PPP_CAMERA_MATRIX),
            mem.rebase(super::PPP_VIEW_MATRIX_COPY),
        );
        let f = mem.rebase(super::MUL_MATRIX);
        call(mem, f, &[view, cam, world]);
        let f = mem.rebase(super::COPY_MATRIX);
        call(mem, f, &[copy, view]);
        let f = mem.rebase(super::RUN_PART);
        let mut done = call(mem, f, &[env_now(mem), body, 0x30]) != 0;
        if !done {
            let life = mem.read::<i32>(at(0x18));
            for (g, limit, strict) in PAR_KEPT {
                let g = mem.rebase(g);
                if mem.read::<u32>(g) != p {
                    continue;
                }
                let limit = limit.cast_signed();
                if (strict && life > limit) || (!strict && life >= limit) {
                    mem.write(g, 0u32);
                    done = true;
                    break;
                }
            }
        }
        if done {
            let f = mem.rebase(super::PPP_ALL_FREE_POBJECT);
            call(mem, f, &[body]);
            mem.write(p, 0u32);
        }
    }

    /// The current ppp environment (`*g_pppEnvSlot`).
    fn env_now(mem: &impl Mem) -> u32 {
        let slot = mem.read::<u32>(mem.rebase(crate::abmap::PPP_ENV_SLOT));
        mem.read::<u32>(slot)
    }

    /// The colour clamp of `rcefAnimeDraw` and `rcefModelProc`: four
    /// floats at `c` times 128.0 (`_ftol2_sse`), clamped to 0..=255, as
    /// bytes at `dst`.
    fn colour_bytes(mem: &mut impl Mem, c: u32, dst: u32) {
        let k = super::COLOUR_128.get(mem);
        for j in 0..4u32 {
            let x = crate::x87::load(mem.read::<u32>(c.wrapping_add(4 * j)));
            let v = ffx_bridge::x87::ftol(&*mem, x * k).cast_signed();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
            mem.write(dst.wrapping_add(j), v.clamp(0, 0xff) as u8);
        }
    }

    /// `rcefAnimeDraw(anim)` (`0x0092f290`, name ours: the drawing half of
    /// `rcefAnimeProc`): the shape's matrix (`+0x50` = camera x `+0x48`),
    /// the colour (`+0x4c`) as bytes at `+0x14`, `+0x24` the VFX packet
    /// position; then the shape's render data (instance `(0, anim)`):
    /// updated and drawn when it exists, else created
    /// (`InitVFXRenderDataShp` with the shape's alpha function, buffers
    /// enlarged 8 times meanwhile). `work` holds the shape pointer
    /// (`set_ptr32` of `+4`) the VFX calls take by address.
    pub fn rcef_anime_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        work: u32,
        call: &mut C,
    ) {
        let at = |o: u32| a.wrapping_add(o);
        let base = mem.read::<u32>(at(4));
        let idx = i32::from(mem.read::<i16>(at(0x42))).cast_unsigned();
        let f = mem.rebase(super::MUL_MATRIX);
        let cam = mem.rebase(super::PPP_CAMERA_MATRIX);
        let m = mem.read::<u32>(at(0x48));
        call(mem, f, &[at(0x50), cam, m]);
        let c = mem.read::<u32>(at(0x4c));
        let body = at(0x10);
        colour_bytes(mem, c, body.wrapping_add(4));
        let pos = crate::ke::PACKET_POS.get(mem);
        mem.write(body.wrapping_add(0x14), pos);
        let off =
            i32::from(mem.read::<i16>(base.wrapping_add(idx.wrapping_mul(8)).wrapping_add(0x10)));
        let shape = base.wrapping_add(off.cast_unsigned());
        let f = mem.rebase(super::SET_PTR32);
        let w = call(mem, f, &[base]);
        mem.write(work, w);
        let f = mem.rebase(super::VFX_GET_RENDER_DATA);
        let seg = u32::from(mem.read::<u16>(at(0x42)));
        if call(mem, f, &[0, a]) != 0 {
            let f = mem.rebase(super::UPDATE_RENDER_DATA_SHP);
            call(mem, f, &[0, a, work, body, seg]);
            let f = mem.rebase(super::VFX_UPDATE);
            call(mem, f, &[0, a]);
        } else {
            let d = mem.read::<u32>(shape.wrapping_add(4));
            let alpha = mem.read::<u8>(d.wrapping_add(shape).wrapping_add(4));
            let f = mem.rebase(super::ALPHA_FUNCTION);
            let af = call(mem, f, &[u32::from(alpha)]);
            let f = mem.rebase(super::SET_ENLARGE);
            call(mem, f, &[8]);
            let f = mem.rebase(super::INIT_RENDER_DATA_SHP);
            call(mem, f, &[0, a, work, body, seg, af]);
            let f = mem.rebase(super::RESET_ENLARGE);
            call(mem, f, &[]);
        }
    }

    /// `rcefAnimeProc(anim)` (`0x0092f4f0`): a live animation (bit 0) is
    /// drawn ([`rcef_anime_draw`]) and stepped (`rcefAnimeStep`).
    pub fn rcef_anime_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        work: u32,
        call: &mut C,
    ) {
        if mem.read::<u8>(a) & 1 != 0 {
            rcef_anime_draw(mem, a, work, call);
            let f = mem.rebase(super::ANIME_STEP);
            call(mem, f, &[a]);
        }
    }

    /// `rcefModelProc(model)` (`0x0092f510`): the model's matrix (`+0x88` =
    /// camera x `+0x80`), the colour (`+0x84`) as bytes at `+0x28`, `+0x44`
    /// the VFX packet position, then its render data (instance
    /// `(g_pppCurrentProgram, model)` as 64 bits): updated
    /// (`updateVFXRenderData`) and drawn (path 3 with flag 4 of `+0x10`),
    /// else created with the alpha function (and the z test of `+0x54`
    /// unless flag 0x2000). `work` is the 0x20-byte block the VFX calls
    /// take, the model data (`set_ptr32` of `+4`) at `+0x1c`. Returns the
    /// last VFX call's `eax`.
    pub fn rcef_model_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let at = |o: u32| m.wrapping_add(o);
        let body = at(0x10);
        let f = mem.rebase(super::MUL_MATRIX);
        let cam = mem.rebase(super::PPP_CAMERA_MATRIX);
        let src = mem.read::<u32>(at(0x80));
        call(mem, f, &[at(0x88), cam, src]);
        let c = mem.read::<u32>(at(0x84));
        colour_bytes(mem, c, at(0x28));
        let pos = crate::ke::PACKET_POS.get(mem);
        mem.write(at(0x44), pos);
        let f = mem.rebase(super::SET_PTR32);
        let d = mem.read::<u32>(at(4));
        let w = call(mem, f, &[d]);
        mem.write(work.wrapping_add(0x1c), w);
        let cur = CURRENT_PROGRAM.get(mem);
        // (cur, m) as a 64-bit sum: m plus cur's sign extension
        let (lo, hi) = (cur, m.wrapping_sub(u32::from(cur.cast_signed() < 0)));
        let path3 = |mem: &M| mem.read::<u8>(body) & 4 != 0;
        let f = mem.rebase(super::VFX_GET_RENDER_DATA);
        if call(mem, f, &[lo, hi]) != 0 {
            let f = mem.rebase(super::UPDATE_RENDER_DATA);
            call(mem, f, &[lo, hi, work, body]);
            let f = if path3(mem) {
                super::VFX_PATH3_UPDATE
            } else {
                super::VFX_UPDATE
            };
            let f = mem.rebase(f);
            call(mem, f, &[lo, hi])
        } else {
            let alpha = u32::from(mem.read::<u8>(at(0x18)));
            let f = mem.rebase(super::ALPHA_FUNCTION);
            let mut e = call(mem, f, &[alpha]);
            if mem.read::<u32>(body) & 0x2000 == 0 {
                let f = mem.rebase(super::Z_MAT);
                let z = mem.read::<u32>(at(0x54));
                e |= call(mem, f, &[z]);
            }
            let f = if path3(mem) {
                super::INIT_PATH3_RENDER_DATA
            } else {
                super::INIT_RENDER_DATA
            };
            let f = mem.rebase(f);
            call(mem, f, &[lo, hi, work, body, e])
        }
    }

    /// `rcefModelProc_0092f770(model)` (`0x0092f770`): [`rcef_model_proc`]
    /// for a live model (bit 0, a tail call); else returns the model.
    pub fn rcef_model_proc_live<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        m: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        if mem.read::<u8>(m) & 1 == 0 {
            return m;
        }
        rcef_model_proc(mem, m, work, call)
    }

    /// `rcefObjCalcLocalMatrix(obj)` (`0x0092f9d0`): the object's motion
    /// step (`rcVu0Add_0092f790`: rotation `+0xd0` += `+0xe0` += `+0xf0`,
    /// position `+0x100` += `+0x110` += `+0x120`, scale `+0x130` += `+0x140`
    /// += `+0x150`; `rcVu0Add4_0092f880`: colour `+0x170` += `+0x180` +=
    /// `+0x190`), then the local matrix `+0x1b0`: scale, flag 0x2000 a
    /// facing from the speed `+0xe0` (normalised times -1, crossed with
    /// up), flag 0x4000 the z turn kept and the base matrix
    /// `0x025d59e0`, rotation `+0x100` times 11.38 (`.rdata` `f32`) at
    /// position `+0xd0` (`op_rt_m_yxz`). `work` is 0xf0 bytes (the
    /// original's frame: three matrices and three vectors).
    #[allow(clippy::too_many_lines)] // the original's steps in turn
    pub fn rcef_obj_calc_local_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        work: u32,
        call: &mut C,
    ) {
        let at = |x: u32| o.wrapping_add(x);
        let w = |x: u32| work.wrapping_add(x);
        let (m3, m2, m1, c, b, a) = (w(0), w(0x40), w(0x80), w(0xc0), w(0xd0), w(0xe0));
        let (e, d) = (w(0x90), w(0xa0));
        let add = mem.rebase(super::VU0_ADD_F790);
        for (dst, src) in [
            (0xe0u32, 0xf0u32),
            (0xd0, 0xe0),
            (0x110, 0x120),
            (0x100, 0x110),
            (0x140, 0x150),
            (0x130, 0x140),
        ] {
            call(mem, add, &[at(dst), at(dst), at(src)]);
        }
        let add4 = mem.rebase(super::VU0_ADD4_F880);
        call(mem, add4, &[at(0x180), at(0x180), at(0x190)]);
        call(mem, add4, &[at(0x170), at(0x170), at(0x180)]);
        let scale = mem.rebase(super::SCALE_VECTOR_F7F0);
        let k = crate::x87::copy(super::ROT_UNITS.get(mem));
        call(mem, scale, &[a, at(0x100), k]);
        let unit = mem.rebase(super::UNIT_MATRIX);
        call(mem, unit, &[at(0x1b0)]);
        call(mem, unit, &[m2]);
        let f = mem.rebase(super::SET_SCALE_VEC3F);
        call(mem, f, &[m2, at(0x130)]);
        let rt = mem.rebase(super::OP_RT_M_YXZ);
        let mul = mem.rebase(super::MUL_MATRIX);
        let norm = mem.rebase(super::VEC3F_NORMALIZE);
        let flags = |mem: &M| mem.read::<u32>(at(0xbc));
        let zero4 = |mem: &mut M, v: u32| {
            for k in 0..4 {
                mem.write(v.wrapping_add(4 * k), 0u32);
            }
        };
        if flags(mem) & 0x2000 != 0 {
            call(mem, unit, &[m1]);
            call(mem, unit, &[m3]);
            zero4(mem, c);
            call(mem, rt, &[m3, m2, a, c]);
            zero4(mem, a);
            let minus = crate::x87::copy(super::MINUS_ONE.get(mem));
            call(mem, scale, &[c, at(0xe0), minus]);
            call(mem, norm, &[c]);
            let f = mem.rebase(super::COPY_QWORD_F850);
            call(mem, f, &[d, c]);
            for (k, v) in (0u32..).zip([0, minus, 0, 0]) {
                mem.write(b.wrapping_add(4 * k), v);
            }
            let cross = mem.rebase(super::VEC3F_MUL);
            call(mem, cross, &[m1, c, b]);
            call(mem, norm, &[m1]);
            call(mem, cross, &[e, m1, d]);
            call(mem, norm, &[e]);
            call(mem, mul, &[m2, m1, m3]);
        }
        if flags(mem) & 0x4000 != 0 {
            mem.write(a.wrapping_add(4), 0u32);
            mem.write(a, 0u32);
            call(mem, unit, &[m1]);
            zero4(mem, b);
            call(mem, rt, &[m1, m1, a, b]);
            call(mem, mul, &[m2, m1, m2]);
            let base = mem.rebase(super::RCEF_BASE_MATRIX);
            call(mem, mul, &[m2, base, m2]);
            mem.write(a.wrapping_add(8), 0u32);
        }
        call(mem, rt, &[at(0x1b0), m2, a, at(0xd0)]);
    }

    /// `rcefObjProc(obj)` (`0x00930060`): the local and world matrices,
    /// then the part (`+0x280`), the animation (`+0x330`) and the model
    /// (`+0x3c0`, a tail call whose `eax` it returns). `work` is 0xf0
    /// bytes.
    pub fn rcef_obj_proc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        o: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        rcef_obj_calc_local_matrix(mem, o, work, call);
        rcef_obj_calc_world_matrix(mem, o, work, call);
        rcef_par_proc(mem, o.wrapping_add(0x280), call);
        rcef_anime_proc(mem, o.wrapping_add(0x330), work, call);
        rcef_model_proc_live(mem, o.wrapping_add(0x3c0), work, call)
    }

    // ---- field agent r2, group E: character matrices for effects ----

    /// The common start of `rcefCalcChrElemRotBaseMatrix` and
    /// `rcefCalcChrElemRotMatrix`: in `work` (0x130 bytes: four matrices
    /// and two vectors at `+0x100`, `+0x110`), `r` = the turn by `v`
    /// (copied, `rcefVecToField`) of identity `m`, then the element's base
    /// matrix into `b` and the identity `t` given `b`'s translation times
    /// -0.001 (`.rdata` `f32`). Returns `(h, r, b, t)`.
    fn elem_rot_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, e, v]: [u32; 3],
        [r, m, t, b]: [u32; 4],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let h = rcef_get_chr_handle(mem, i, call);
        let unit = mem.rebase(super::UNIT_MATRIX);
        call(mem, unit, &[r]);
        call(mem, unit, &[m]);
        let (va, vz) = (work.wrapping_add(0x100), work.wrapping_add(0x110));
        let f = mem.rebase(super::COPY_QWORD_D6F0);
        call(mem, f, &[va, v]);
        let f = mem.rebase(super::VEC_TO_FIELD);
        call(mem, f, &[va]);
        for k in 0..4 {
            mem.write(vz.wrapping_add(4 * k), 0u32);
        }
        let f = mem.rebase(super::OP_RT_M_YXZ);
        call(mem, f, &[r, m, va, vz]);
        call(mem, unit, &[t]);
        let f = mem.rebase(super::CH_ELEM_GET_BASE_MATRIX);
        call(mem, f, &[h, e, b]);
        let k = crate::x87::copy(super::MILLI_NEG.get(mem));
        let f = mem.rebase(super::SCALE_VECTOR_D780);
        call(mem, f, &[t.wrapping_add(0x30), b.wrapping_add(0x30), k]);
        h
    }

    /// `rcefCalcChrElemRotBaseMatrix(i, e, out, v)` (`0x0092d8c0`, Switch
    /// name by layout): `out` = 0.001 scale x the element's base position
    /// (negated, x 0.001) x the turn by `v`. `work` is 0x130 bytes.
    pub fn rcef_calc_chr_elem_rot_base_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, e, out, v]: [u32; 4],
        work: u32,
        call: &mut C,
    ) {
        let (r, t, m) = (work, work.wrapping_add(0x40), work.wrapping_add(0x80));
        elem_rot_start(mem, [i, e, v], [r, m, t, m], work, call);
        let f = mem.rebase(super::SET_SCALE_0001);
        call(mem, f, &[out]);
        let mul = mem.rebase(super::MUL_MATRIX);
        call(mem, mul, &[out, t, out]);
        call(mem, mul, &[out, r, out]);
    }

    /// `rcefCalcChrElemRotMatrix(i, e, out, v)` (`0x0092d9a0`, Switch name
    /// by layout): as [`rcef_calc_chr_elem_rot_base_matrix`] with the turn
    /// about the element (moved there and back, the inverse translation
    /// negated through `fchs`), then placed at the character's position
    /// (`MsGetChrPos`). `work` is 0x130 bytes.
    pub fn rcef_calc_chr_elem_rot_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, e, out, v]: [u32; 4],
        work: u32,
        call: &mut C,
    ) {
        let (r, b, t, p) = (
            work,
            work.wrapping_add(0x40),
            work.wrapping_add(0x80),
            work.wrapping_add(0xc0),
        );
        elem_rot_start(mem, [i, e, v], [r, b, t, b], work, call);
        let unit = mem.rebase(super::UNIT_MATRIX);
        call(mem, unit, &[p]);
        let f = mem.rebase(super::MS_GET_CHR_POS);
        call(mem, f, &[i, p.wrapping_add(0x30)]);
        let f = mem.rebase(super::SET_SCALE_0001);
        call(mem, f, &[out]);
        let mul = mem.rebase(super::MUL_MATRIX);
        call(mem, mul, &[out, t, out]);
        call(mem, mul, &[out, r, out]);
        for k in 0..3 {
            let a = t.wrapping_add(0x30 + 4 * k);
            let x = crate::x87::copy(mem.read::<u32>(a)) ^ 0x8000_0000;
            mem.write(a, x);
        }
        call(mem, mul, &[out, t, out]);
        call(mem, mul, &[out, p, out]);
    }

    /// `rcefSetChrBaseMatrix(i, m, out)` (`0x0092e830`, Switch name by
    /// layout): `m` (into `out`, or `0x0193c090` without one) with its rows
    /// scaled by the character's scale times its data scale
    /// (`Ch_GetScale`, `Ch_GetDataScale`), made the character's base
    /// matrix (`Ch_SetOrgMatrix`). `work` holds the three scale floats.
    pub fn rcef_set_chr_base_matrix<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [i, m, out]: [u32; 3],
        work: u32,
        call: &mut C,
    ) {
        let h = rcef_get_chr_handle(mem, i, call);
        let s = f64::from(ffx_chara::chr::bridge::ch_get_data_scale(&*mem, h));
        let (x, y, z) = (work, work.wrapping_add(4), work.wrapping_add(8));
        let f = mem.rebase(super::CH_GET_SCALE);
        call(mem, f, &[h, x, y, z]);
        let mut k = [0u32; 3];
        for (j, a) in [x, y, z].into_iter().enumerate() {
            k[j] = crate::x87::store(crate::x87::load(mem.read::<u32>(a)) * s);
            mem.write(a, k[j]);
        }
        let dst = if out == 0 {
            mem.rebase(super::CHR_BASE_MATRIX)
        } else {
            out
        };
        let f = mem.rebase(super::COPY_MATRIX);
        call(mem, f, &[dst, m]);
        let f = mem.rebase(super::SCALE_VECTOR_D690);
        for (j, kj) in (0u32..).zip(k) {
            let row = dst.wrapping_add(0x10 * j);
            call(mem, f, &[row, row, kj]);
        }
        let f = mem.rebase(super::CH_SET_ORG_MATRIX);
        call(mem, f, &[h, dst]);
    }
}

#[cfg(test)]
mod tests {
    use ffx_bridge::{Mem, VecMem};

    use super::*;

    #[test]
    fn the_stack_holds_eight_levels() {
        let mut m = VecMem::new(0x1000, 0x1000);
        let t = 0x1000;
        bridge::rcef_task_init(&mut m, t);
        for i in 0..10 {
            bridge::rcef_task_call_seq(&mut m, t, 0x100 + i);
        }
        assert_eq!(m.read::<i32>(t + header::LEVEL), MAX_LEVEL);
        assert_eq!(m.read::<u32>(seq_slot(t, 7)), 0x106);
        bridge::rcef_task_ret_seq_cont(&mut m, t);
        assert_eq!(m.read::<i32>(t + header::LEVEL), 6);
        assert_eq!(m.read::<u32>(t) & RERUN, RERUN);
    }

    #[test]
    fn tasks_sort_by_priority() {
        let mut m = VecMem::new(0x1000, 0x2000);
        let c = 0x1000;
        bridge::rcef_task_ctrl_init(&mut m, c);
        assert!(bridge::rcef_task_ctrl_is_end(&m, c));
        let (t1, t2) = (0x1400, 0x1600);
        bridge::rcef_task_init(&mut m, t1);
        bridge::rcef_task_init(&mut m, t2);
        m.write(t2 + header::PRIORITY, 0x10u32);
        bridge::rcef_task_ctrl_add(&mut m, c, t1);
        bridge::rcef_task_ctrl_add(&mut m, c, t2);
        assert_eq!(m.read::<u32>(c + header::NEXT), t2);
        assert_eq!(m.read::<u32>(t2 + header::NEXT), t1);
        assert_eq!(m.read::<u32>(t1 + header::NEXT), c + ctrl::TAIL);
        assert!(!bridge::rcef_task_ctrl_is_end(&m, c));
    }
}
