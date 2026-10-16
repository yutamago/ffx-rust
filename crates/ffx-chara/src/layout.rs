//! The character object as the game keeps it (`docs/formats/chara.md`):
//! offsets into one element of `g_ChrArray` and into the model block it
//! points to, with the functions that show each one.
//!
//! Only what the ported functions reveal is here; names follow the
//! functions (`Ch_GetJimen` reads [`JIMEN`]).

/// Bytes per character (`Ch_CheckHandle` steps `g_ChrArray` by it).
pub const SIZE: u32 = 0x880;

/// `u16` character id, `kind << 12 | number` (`Ch_GetId`).
pub const ID: u32 = 0x000;
/// `u8`, non-zero while the slot is in use (`Ch_CheckHandle`,
/// `Ch_HandleGetNext`).
pub const ACTIVE: u32 = 0x002;
/// Pointer to the name, inside the model block (`Ch_GetName`; block
/// [`BLK_NAME`]).
pub const NAME: u32 = 0x004;
/// `f32` x, y, z position, `w` after it (`Ch_GetPos`, `Ch_SetPos3`).
pub const POS: u32 = 0x00c;
/// The position again, all four words copied after every `Ch_SetPos`
/// (`Ch_SetPos`, `Ch_SetPos3`).
pub const POS_COPY: u32 = 0x01c;
/// `f32` vertical velocity, `Ch_SetVelY`'s argument × 0.05.
pub const VEL_Y: u32 = 0x050;
/// `f32` x, y, z scale (`Ch_GetScale`, `Ch_SetScale`).
pub const SCALE: u32 = 0x05c;
/// `f32` x, y, z offset (`Ch_SetOffset`).
pub const OFFSET: u32 = 0x06c;
/// `f32` x, y, z ground normal, reset to (0, -1, 0) when the character
/// stops following the ground (`Ch_EnMapID`).
pub const NORMAL: u32 = 0x07c;
/// Light shade multipliers r, g, b (`Ch_SetShadeColEffect`).
pub const SHADE_COL_EFFECT: u32 = 0x0bc;
/// Pointer to a matrix the character is placed by (`Ch_SetOrgMatrix`).
pub const ORG_MATRIX: u32 = 0x10c;
/// `f32` scale for [`ORG_MATRIX`] (`Ch_SetOrgMatrixScale`).
pub const ORG_MATRIX_SCALE: u32 = 0x110;
/// `f32` speed (`Ch_SetSp`).
pub const SP: u32 = 0x154;
/// `f32` rotation about y in radians (`Ch_GetRot` returns it wrapped to
/// ±3.141592).
pub const ROT: u32 = 0x158;
/// `f32` direction (`Ch_SetDir`).
pub const DIR: u32 = 0x168;
/// `f32` ground height under the character (`Ch_SetJimen`).
pub const JIMEN: u32 = 0x16c;
/// `f32` speed from which the character runs; 18 when set to 0 or less
/// (`Ch_MoveSetChgRun`).
pub const CHG_RUN: u32 = 0x170;
/// `u8` hide bits: [`HIDE_EVENT`], [`HIDE_EFFECT`], [`HIDE_BATTLE`], ...
/// (`Chr_SetHideFlags`).
pub const HIDE: u32 = 0x180;
/// `u8`, non-zero hides the character from the calculation pass
/// (`Ch_SetHideCalc`, `Ch_GetHideCalc`).
pub const HIDE_CALC: u32 = 0x181;
/// `i8` ground mode (`Ch_SetGravMode`).
pub const GRAV_MODE: u32 = 0x182;
/// `i8` shadow mode (`Ch_SetShadowMode`).
pub const SHADOW_MODE: u32 = 0x183;
/// `i8` field mode, also the motion slot of the default motion
/// (`Ch_SetFieldMode`).
pub const FIELD_MODE: u32 = 0x185;
/// `u8` motion type (`Ch_SetMotionType`).
pub const MOTION_TYPE: u32 = 0x186;
/// `u32` battle character number (`Ch_SetBattleChrNum`).
pub const BATTLE_CHR_NUM: u32 = 0x18c;
/// `u32` flags, [`F_MAP_ID`] and on (`Ch_EnBodyHit`, `Ch_EnClip`, ...).
pub const FLAGS: u32 = 0x194;
/// `u32` more flags, [`F2_CHOCOBO`] and on.
pub const FLAGS2: u32 = 0x198;
/// Pointer to the character this one takes its shade from
/// (`Ch_SetShadeID`).
pub const SHADE_PARENT: u32 = 0x19c;
/// Pointer to the models, [`MODEL_SIZE`] bytes each (`Ch_ModelGetHide`).
pub const MODELS: u32 = 0x1bc;
/// Pointer to a header with the `u16` model count at `+6`
/// (`Ch_ModelSetHide`, `TkChPatchM160`).
pub const MODEL_HEADER: u32 = 0x1c0;
/// Pointer to the character's model block in `g_ChrBlk`
/// (`Ch_GetBlkDirName`, `Ch_GetDataScale`).
pub const BLK: u32 = 0x1c4;
/// Colour fades r, g, b ([`Fade`](crate::chr::Fade), 12 bytes apart;
/// `Ch_SetShadeCol`).
pub const FADE_SHADE_COL: u32 = 0x330;
/// Alpha fade (`Ch_SetTransparent`; 0 hides, `Ch_GetHideCalc`).
pub const FADE_ALPHA: u32 = 0x354;
/// Shade fade (`Ch_SetShade`).
pub const FADE_SHADE: u32 = 0x360;
/// Specular alpha fade (`Ch_SetSpecularAlpha`).
pub const FADE_SPECULAR: u32 = 0x378;
/// `u32` effect level (`Ch_EffectSetEffectLevel`).
pub const EFFECT_LEVEL: u32 = 0x408;
/// `f32` collision radius scale; 0 or less takes the block's
/// [`BLK_THICKNESS`] (`Ch_SetThickness`).
pub const THICKNESS: u32 = 0x4e4;
/// `f32` battle camera thickness; negative takes [`BLK_THICKNESS_BTL_CAM`]
/// (`Ch_SetThicknessBtlCam`).
pub const THICKNESS_BTL_CAM: u32 = 0x4ec;
/// `f32` fixed height; negative takes [`BLK_FIX_HEIGHT`]
/// (`Ch_SetFixHeight`).
pub const FIX_HEIGHT: u32 = 0x4f0;
/// `f32` camera height; negative takes [`BLK_CAM_HEIGHT`]
/// (`Ch_SetCamHeight`).
pub const CAM_HEIGHT: u32 = 0x4f4;
/// `f32` hit height (`Ch_SetHitHeight`).
pub const HIT_HEIGHT: u32 = 0x4f8;
/// `f32` weight when two characters push each other (`Ch_SetWeight`).
pub const WEIGHT: u32 = 0x500;
/// `f32` gravity (`Ch_AddGrav`, `Ch_GetGrav`).
pub const GRAV: u32 = 0x504;
/// `f32` (`Ch_SetHiju`).
pub const HIJU: u32 = 0x51c;
/// `u32` texture animation id (`Ch_TextureGetAnimId`).
pub const TEXTURE_ANIM_ID: u32 = 0x700;
/// `u16`, cleared with every new motion (`Ch_ResetMotion`).
pub const MOTION_728: u32 = 0x728;
/// `u32`, 1 with every new motion (`Ch_ResetMotion`).
pub const MOTION_730: u32 = 0x730;
/// `u8`, non-zero while a scripted motion plays (`Ch_ResetMotion`).
pub const MOTION_SCRIPTED: u32 = 0x736;
/// `u16` interpolation frames into the next clip (`Ch_MotionSetHokan`).
pub const HOKAN: u32 = 0x73a;
/// `u16` interpolation frames of the next motion; 0 is stored as 1
/// (`Ch_MotionSetNextHokan`).
pub const NEXT_HOKAN: u32 = 0x73c;
/// `i16` motion speed, 8.8 fixed (`Ch_SetMotionSpeed`).
pub const MOTION_SPEED: u32 = 0x750;
/// `u32` motions by slot for standing, walking, running
/// (`Ch_MotionSetMoves`).
pub const MOVES: u32 = 0x75c;
/// `u32` flags; bit 2 marks the character for an update
/// (`Chr_SetHideFlags`, `Ch_MotionSetHokan`).
pub const DIRTY: u32 = 0x820;
/// `i16`, -1 after every `Ch_SetPos` (`Ch_ResetPos`, `Ch_IdGetPNum`).
pub const PNUM: u32 = 0x824;
/// The render instance (`Ch_GetChrRenderInstance`).
pub const RENDER_INSTANCE: u32 = 0x830;

/// [`FLAGS`]: set, the character leaves the ground (`Ch_EnMapID(chr, 0)`).
pub const F_MAP_ID: u32 = 1 << 7;
/// [`FLAGS`]: set, other characters pass through (`Ch_EnBodyHit(chr, 0)`).
pub const F_NO_BODY_HIT: u32 = 1 << 8;
/// [`FLAGS`]: interpolate rotations (`Ch_EnRotHokan`).
pub const F_ROT_HOKAN: u32 = 1 << 10;
/// [`FLAGS`]: hide past the clip distance (`Ch_EnClip`).
pub const F_CLIP: u32 = 1 << 11;
/// [`FLAGS`]: hidden from the calculation pass as well (`Ch_GetHideCalc`).
pub const F_HIDE_CALC: u32 = 1 << 13;
/// [`FLAGS`]: cleared by `Ch_MotionSetHokan`.
pub const F_HOKAN_16: u32 = 1 << 16;
/// [`FLAGS`]: shaded by the map id (`Ch_IdEnableShade`).
pub const F_ID_SHADE: u32 = 1 << 18;
/// [`FLAGS`]: set by `Ch_SetPos`.
pub const F_POS_SET: u32 = 1 << 25;
/// [`FLAGS`]: overlap hit (`Ch_SetOverlapHit`).
pub const F_OVERLAP_HIT: u32 = 1 << 30;
/// [`FLAGS2`]: a chocobo (`Ch_SetChocobo`).
pub const F2_CHOCOBO: u32 = 1 << 2;
/// [`FLAGS2`]: VU1 clip mode (`Ch_Vu1SetClipmode`).
pub const F2_VU1_CLIP: u32 = 1 << 3;
/// [`FLAGS2`]: skipped by `Ch_GetChrRenderInsatanceFromOEF2Content`.
pub const F2_NO_OEF2: u32 = 1 << 5;

/// [`HIDE`] bit: hidden by an event script (`Ch_SetHideEvent`).
pub const HIDE_EVENT: u8 = 1;
/// [`HIDE`] bit: hidden by an effect (`Ch_SetHideEffect`).
pub const HIDE_EFFECT: u8 = 2;
/// [`HIDE`] bit: hidden in battle (`Ch_SetHideBattle`).
pub const HIDE_BATTLE: u8 = 0x10;

/// Bytes per model (`Ch_ModelGetHide` indexes by 0x38).
pub const MODEL_SIZE: u32 = 0x38;
/// Model: pointer to the normals (`Ch_ModelGetNromPtr`).
pub const MODEL_NROM: u32 = 0x04;
/// Model: pointer to the vertices (`Ch_ModelGetVtexPtr`).
pub const MODEL_VTEX: u32 = 0x08;
/// Model: `f32` alpha (`Ch_ModelSetAlpha`).
pub const MODEL_ALPHA: u32 = 0x14;
/// Model: `i8` hidden (`Ch_ModelGetHide`).
pub const MODEL_HIDE: u32 = 0x18;

/// Bytes per model block in `g_ChrBlk` (`Ch_BlkGetHandleFast`).
pub const BLK_SIZE: u32 = 0x12c;
/// Model blocks in `g_ChrBlk` (`Ch_BlkGetHandleFast` stops after them).
pub const BLKS: u32 = 40;
/// Block: `f32` default collision radius (`Ch_SetThickness`).
pub const BLK_THICKNESS: u32 = 0x1c;
/// Block: `f32` battle thickness (`Ch_GetThicknessBtl`).
pub const BLK_THICKNESS_BTL: u32 = 0x20;
/// Block: `f32` default battle camera thickness (`Ch_SetThicknessBtlCam`).
pub const BLK_THICKNESS_BTL_CAM: u32 = 0x24;
/// Block: `f32` default fixed height (`Ch_SetFixHeight`).
pub const BLK_FIX_HEIGHT: u32 = 0x28;
/// Block: `f32` default camera height (`Ch_SetCamHeight`).
pub const BLK_CAM_HEIGHT: u32 = 0x2c;
/// Block: `f32` data scale (`Ch_GetDataScale`).
pub const BLK_DATA_SCALE: u32 = 0x30;
/// Block: the character's name, where [`NAME`] points.
pub const BLK_NAME: u32 = 0x71;
/// Block: the directory name (`Ch_GetBlkDirName` returns its address).
pub const BLK_DIR_NAME: u32 = 0xb8;
/// Block: the resident data, pointer and size
/// (`Ch_EffectGetResidentPtr`,
/// `Ch_GetChrRenderInsatanceFromOEF2Content`).
pub const BLK_RESIDENT: u32 = 0x124;
/// Block: `u32` size of the resident data.
pub const BLK_RESIDENT_SIZE: u32 = 0x128;
