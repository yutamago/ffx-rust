# Encounters and battle formations

Two kinds of file decide which battles a map can start and what each one
holds:

- `ffx_ps2/ffx/master/jppc/battle/kernel/btl.bin`, the encounter table:
  the battle fields, each with its encounter groups.
- `ffx_ps2/ffx/master/jppc/battle/btl/<field>_<id>/<field>_<id>.bin`, one
  per formation (863).

Reader/writer: [`crates/ffx-battle`](../../crates/ffx-battle);
`ffx battle list/export/import`.

## The encounter table

The game reads it at start-up as a PS2 file; `MsBtlListInit`
(`0x0079d250`) keeps the fields at `*0x0112a9c8` (count `0x0112a9d0`) and
the blocks at `*0x0112a9cc`.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Version, 2 |
| `0x04` | `u32` | Offset of the fields, `0x10` |
| `0x08` | `u32` | Offset of the blocks = end of the fields |
| `0x0c` | `u32` | End of the file (4096); the blocks fill the table up to it |

**Field**, 14 bytes (`MsBtlListField`, `0x0079d190`), 96 of them:

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `i16` | Map number; `MsBtlListFieldNum` (`0x0079d1c0`) finds a map's field by it |
| `0x02` | `i16` | Offset of the field's block from the start of the blocks |
| `0x04` | `i16` | Index of the block's first formation among all formations |
| `0x06` | `char[8]` | Name, NUL padded (`cdsp00`) |

Five blocks are shared by two or three fields (`cdsp01` uses `cdsp00`'s,
`mcfr01` and `mcfr02` use `mcfr00`'s; also `mihn08`, `mihn05` and
`hiku05`). Blocks follow each other in the
order the fields first use them, and the first-formation words are running
totals of the formation counts.

**Block** (`MsBtlListEncount`, `0x0079d170`): `u8` formation count (it
wraps: `zzzz00` has 255), `u8` group count, then the groups.

**Group** (`MsBtlListGroup`, `0x0079d210`), `5 + 2n` bytes:

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u8` | Formation count *n* |
| `0x01` | `u16` | Battle map, `0x0112c254` (`MsBattleExe`): the battle happens on map `& 0x3ff`, loaded unless it is the current map (`MsBtlReadSetScene` compares it with `SaveData +4`; `0x00908600` requests the load); `0x400` set marks a battle map (`0x009083a0`; the debug overlay prints `BM`, else `FM`). For battles without random encounters it is often the field's own map number |
| `0x03` | `u8` | Encounter rate; 0 = no random encounters (`MsBattleEncountExe`, `0x00780d10`) |
| `0x04` | `u8` | Weight total |
| `0x05` | (`u8`, `u8`) × *n* | Formation id, weights |

The low nibble of a formation's weights is its default weight; the high
nibble, 0 in the file, is the weight in use. `Enc_ResetWeights`
(`0x007828a0`) copies the low nibble into the high one when a map is
entered and sums the group's weights into its total; `MsBattleEncountExe` rolls the
formation with the high nibbles. Scripts find a formation's entry with
`MsFldGetScene` (`0x007827f0`, `map << 16 | id`) to change its weight.

The harness test reads the salvage ship's group back from the running game
after loading the save: a mod's weights 3 and 1 show as `0x33` and `0x11`,
total 4.

### Which file a formation loads

`MsBattleExe` (`0x00781020`) asks `MsBtlListCDpos` (`0x0079d0e0`) for
`first + the formations of the earlier groups + formation` and reads that
PS2 file of category `0xe` (`MsBattleExeSetIndex`, `0x00781550`). That file
is `battle/btl/<owner>_<id>`, the owner being the first field with the
block: every one of the 863 entries names a shipped file. The id itself only
names the file and is what scripts look formations up by. A field cannot
gain or lose formations: every later field would load its neighbour's
files. The writer refuses such edits.

## Formation files

The same container as the monster files: `u32` section count (8 in 817
files, 7 in 37, 5 in 9), then the section offsets, 0 for absent
ones, in ascending order. In 15 of the 7-section files the last offsets
point 16 bytes past the end of the file; those sections are empty.
`MsBtlReadSetScene` (`0x00783e20`):

| Section | Use |
|---|---|
| 0 | Battle event script → `0x0112a9ac` (an `ffx-script` script) |
| 1 | Its first byte → `0x0112a9a4` |
| 2 | The formation, 28 bytes → `0x0112a9c0` |
| 3 | Positions: `0x20`-byte records with counts at `+4`, `+5`, `+6` and offsets from `+0x10` → `0x0112a9b0` |

858 files have a formation; the other five (`klyt01_01`–`06`) have a
20-byte section 2 of another layout.

**Formation** (the debug overlay prints bytes 0–2 as `V%dR%dF%d` and the
slots as `M%d:%03d`):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u8` | Unknown. With byte 3 clear, battle signals also go to the second script set (`0x007986a0`). 1 in 393 files |
| `0x01` | `u8` | Unknown. 1 would send request `0xc` with 1 at battle start (`0x0081ec20`); 0 or 2 in the files |
| `0x02` | `u8` | Footstep sound bank (`MsBtlReadScene`, `0x00783d80`) |
| `0x03` | `u8` | Underwater (`MsBattleWater`, `0x007821e0`): set in the underwater fields (`cdsp00`, `bjyt02`, `genk09`, `bsil03`, `mtgz07`, `stbv00`, …) |
| `0x04` | `u8` × 8 | 0 in every file |
| `0x0c` | `u16` × 8 | Monster slots: `0x1000 \| n` for monster m*NNN*, `0xffff` empty (`MsSetRamChrData`, `0x0079c110`). Used slots come first |

Each monster in a slot gets its file `battle/mon/_mNNN` read
(`Mon_LoadAll`, `0x00784070`). In battle, the monster's id is at
`+0x0e` of its actor.

## The battle actor (`chr`, 0xf90 bytes)

`MsGetChr(i)` (`0x00794020`) addresses actors 0-0x1e at `*0x011334d4`
(`crates/ffx-battle/src/actor.rs`); party members are 0-0x11, monsters
0x14-0x1b. The battle scripts' properties (`MsGetChrStatInfo` `0x007b2dc0`,
jump table `0x007b4504`; `MsSetChrStatInfo` `0x007b4b70`, jump table
`0x007b66dc`) read and write most fields one by one, so a property's case
fixes its field's offset and width; `crates/ffx-battle/src/stat.rs` and
`stat_set.rs` port both switches with parity tests. The property names
(`STAT_*`, `ABILITY_*`) are the PS2 header's, from Fahrenheit's
`ids/chr_stat.cs` (`btlhead.src`); the field names are Fahrenheit's
(`battle/chr.cs`, their `Chr` and `ChrRam`, used with permission). The
evidence column gives the property whose case reads the field, or
functions that use the offset (widths from the disassembly of the battle
code, `0x00780000`-`0x007d0000`). `+0x540` to `+0x72c` is the 0x1ec-byte
block `MsSetRamChr` clears and fills (`ChrRam`: Fahrenheit's offsets plus
0x540). Bold notes are where FFX.exe disagrees with Fahrenheit
(2026-10-27, Fahrenheit 4ec1919).

Bit fields, from the properties' cases:
- `+0x590`: bits 0-2 male, female, aeon (properties 0x16-0x18);
- `+0x5b8` (`u16`): bits 0-4 hard, ratio damage, zombie one-hit,
  see-through, Libra; 5-7 physical, magic, all immunity; 9 Zanmato-proof;
  10 bribe (properties 0x20-0x24, 0x107-0x109, 0x10b, 0x122);
- `+0x5da`-`+0x5dd` (absorb, nullify, halve, weak): bits 0-4 fire, ice,
  thunder, water, holy;
- `+0x606` (`u16`): bit 1 zombie, 3 poison (`MsGetChrStatZombie`,
  `MsGetChrStatPoison`); 0 KO and 2 petrify (Fahrenheit, not checked; the
  actor's KO and stone states are the bytes `+0xdcc` and `+0xdce`); 4-7
  power, magic, armor, mental break; 8-11 confusion, berserk, provoke,
  threaten;
- `+0x608`-`+0x614`, 13 counters (`0xff` forever): sleep, silence,
  darkness, shell, protect, reflect, NulTide, NulBlaze, NulShock,
  NulFrost, regen, haste, slow;
- `+0x616` (`u16`): bits 0-14 Libra, the five memories (strength, magic,
  agility, move, ability), dodge, guard, blown away, auto-life, curse,
  defend, cover, sentinel, doom;
- `+0x641`-`+0x659`, the resistance bytes: KO, zombie, petrify, poison,
  then the `+0x606` bits and the 13 counters in their order; `+0x65a`
  the `+0x616` bits (properties 0xaf-0xd6).

| Offset | Type | Fahrenheit | Evidence |
|---|---|---|---|
| `+0x000` | `ptr` | `actor` | `MsBattleEncountExe`, `MsBattleReadDataNB` |
| `+0x004` | `u32` | `model_id` | `TexAnim_StartOld` |
| `+0x00c` | `u16` | `id` | Fahrenheit, not checked |
| `+0x00e` | `u16` | `chr_id` | `MsBtlReadManage`, `MsBtlReadMonster`, `MsBtlReadMonsterProcess` |
| `+0x016` | `u8` | `stat_visible` | property `0x54` `STAT_VISIBLE`; `MsSetSaveParam`, `MsCalcDamage`, `DmgCalc_Defend` |
| `+0x017` | `u8` | `stat_hide` | `MsCalcCommand`, `DmgCalc_ShatterFail`, `DmgCalc_MagicBooster` |
| `+0x018` | `i8` | `stat_visible_eff` | property `0xed` `STAT_VISIBLE_EFF`; `Mon_AttachData` |
| `+0x019` | `u8` | `stat_visible_cam` | property `0xd8` `STAT_VISIBLE_CAM`; `MsSetSaveParam`, `MsSetChrVisibleAppear`, `msSetChrVisibleReq` |
| `+0x01b` | `u8` | `stat_visible_out` | property `0xd9` `STAT_VISIBLE_OUT`; `MsBtlBridgeBtl2Event`, `MsSetSaveParam`, `MsDamageSetMotion` |
| `+0x01c` | `i8` | `stat_visible_out_on` | property `0x150` `STAT_VISIBLE_OUT_ON`; `MsBtlBridgeBtl2Event` |
| `+0x021` | `u8 (bool)` | `stat_motion_dispose_flag` | property `0xee` `STAT_MOTION_DISPOSE_FLAG`; `MsBtlBridgeBtl2Event`, `MsSetSaveData`, `MsSetSaveParam` |
| `+0x022` | `u8 (bool)` | `stat_model_dispose_flag` | property `0xef` `STAT_MODEL_DISPOSE_FLAG`; `MsBtlBridgeBtl2Event`, `MsSetSaveParam`, `MsBtlChrFreeAll` |
| `+0x023` | `u8 (bool)` | `stat_fast_model_flag` | property `0xde` `STAT_FAST_MODEL_FLAG`; `MsSetSaveParam`, `MsCalcDamageCTB`, `MsCalcDamageCommand` |
| `+0x024` | `u8` | `stat_model` | property `0x58` `STAT_MODEL`; `MsBtlReadManage`, `MsBtlReadMonsterProcess`, `MsSaveAddHP` |
| `+0x026` | `i8` | `stat_shadow` | property `0xf1` `STAT_SHADOW`; `MsSetSaveParam`, `MsCalcMagic`, `Battle_Steal` |
| `+0x02d` | `u8 (bool)` | `stat_bodyhit_flag` | property `0x7f` `STAT_BODYHIT_FLAG`; property `0x14c` `STAT_BODYHIT_DIRECT`; `MsSetSaveParam`, `MsGetComInfo`, `DmgCalc` |
| `+0x03b` | `u8 (bool)` | `eternal_autolife` | property `0x14e` `STAT_ETERNAL_RELIFE`; `getBtlchrParam`, `MsSetEternalRelife`, `MsGetChrStatInfo` |
| `+0x048` | `ptr` | `ptr_mon_wep_bin` | `MsBtlReadMonster`, `Mon_AttachData`, `MsAttachWeaponData_00784000` |
| `+0x182` | `-` | `grav_mode` | **not a battle actor field**: `+0x182` of the field character `+0x0` points to (`Ch_GetGravMode` `0x0082abc0`); no battle code reads the actor there |
| `+0x185` | `-` | `field_mode` | **not a battle actor field**: the field character's `+0x185` (`Ch_GetFieldMode` `0x0082ab90`) |
| `+0x186` | `-` | `motion_type` | `MsSetRamChr`; **not a battle actor field**: the field character's motion type; `MsSetRamChr`'s word stores go to `+0x540 + 0x186` = `+0x6c6` |
| `+0x3f4` | `u8` | `death_level` | `MsDeathTerminate`, `MsRelife`, `MsGetChrByte3f4_00795a60` |
| `+0x3f5` | `u8` | `death_pattern` | property `0x4f` `STAT_DEATH_PATTERN`; `MsDamageCheckDeath`, `MsRelife`, `MsSetDeath` |
| `+0x3f8` | `u8 (bool)` | `stat_center_chr_flag` | property `0x129` `STAT_CENTER_CHR_FLAG`; `msCheckSameTarget`, `MsSetRamChrData`, `MsExchangeChr` |
| `+0x3f9` | `u8` | `stat_death_return` | property `0x14a` `STAT_DEATH_RETURN`; `MsCheckDeathMotion`, `MsSetRamChrData`, `Native_7117_BtlQueueCommand` |
| `+0x400` | `u16` | `motion_speed_normal` | property `0xfb` `STAT_MOTION_SPEED_NORMAL`; `MsBtlReadPlayerProcess`; **`u16`**, not `u8`: every access is a word (`MsGetChrMotionSpeed` `0x007956b0`, `MsCalcMotionSpeed`) |
| `+0x402` | `u16` | `motion_speed_effect` | `MsEffectSetSpeed`, `MsCalcMotionSpeed`, `Battle_ScriptPlayMotion` |
| `+0x404` | `u8` | `motion_speed_effect_nonzero` | `MsEffectSetMotion`, `MsEffectSetSpeed`; 1 while `+0x402` is in force (`MsEffectSetSpeed` sets it, `MsMotionSpeedSetNormal` clears it); `MsTerminateMotion` clears `+0x404` and `+0x405` with one word store |
| `+0x405` | `u8` | `motion_speed_normal_init` | property `0xfc` `STAT_MOTION_SPEED_NORMAL_START`; `MsEffectMagicMotion`, `MsEffectStart`, `MsEffectEnd_00788f90` |
| `+0x408` | `u8` | `stat_attack_motion_type` | property `0xe5` `STAT_ATTACK_MOTION_TYPE`; `MsSetRamChrData`, `ms_motion_trans`, `MsParseCommand` |
| `+0x409` | `u8` | `stat_return_motion_type` | property `0x10d` `STAT_RETURN_MOTION_TYPE`; `ms_motion_trans`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x40a` | `u8 (bool)` | `stat_attack_return_flag` | property `0x7b` `STAT_ATTACK_RETURN_FLAG`; `MsEffectSetMotion`, `MsCountChrReady`, `Battle_ScriptPlayMotion` |
| `+0x40c` | `u8` | `stat_attack_normal_frame` | property `0x7c` `STAT_ATTACK_NORMAL_FRAME`; `MsGetMoveTargetPos` |
| `+0x40d` | `u8` | `stat_attack_near_frame` | property `0x116` `STAT_ATTACK_NEAR_FRAME`; `MsCheckTargetsNear`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x40e` | `u8` | `stat_attack_motion_frame` | property `0x123` `STAT_ATTACK_MOTION_FRAME`; `Native_70b0_ChrGetMoveSteps`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x412` | `u8 (bool)` | `stat_move_flag` | property `0x5d` `STAT_MOVE_FLAG`; `MsSetChrMoveFlagAll`, `MsBattleProcess_DebugStatus_00792f20`, `MsSetChrMoveFlag` |
| `+0x416` | `u8` | `stat_move_target` | property `0x5a` `STAT_MOVE_TARGET`; `Atel_BtlMoveTargetDist`, `MsGetDirMode`, `MsGetMoveTargetPos` |
| `+0x41b` | `u8 (bool)` | `stat_direction_fix_flag` | property `0xa4` `STAT_DIRECTION_FIX_FLAG`; `MsEffectMagicMotion`, `MsSetChrDirTarget`, `Native_702c_ChrMoveToTarget` |
| `+0x41c` | `u8 (bool)` | `stat_direction_change_flag` | property `0xa2` `STAT_DIRECTION_CHANGE_FLAG`; `MsBtlReadPlayerProcess` |
| `+0x41d` | `u8` | `stat_direction_change_effect` | property `0xa3` `STAT_DIRECTION_CHANGE_EFFECT`; `MsCheckRestMotion`, `MsBattleProcess_Status_007917d0`, `MsSetRamChrData` |
| `+0x41e` | `u8 (bool)` | `stat_disable_move_flag` | property `0x7d` `STAT_DISABLE_MOVE_FLAG`; `MsChrGroundFlag_00794db0`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x41f` | `u8 (bool)` | `stat_disable_jump_flag` | property `0x7e` `STAT_DISABLE_JUMP_FLAG`; `MsGetChrByte41f_00794d70`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x420` | `u8` | `stat_motionlv` | property `0x5b` `STAT_MOTIONLV`; `MsAttachWeapon` |
| `+0x422` | `u8` | `stat_damage_chr` | property `0x59` `STAT_DAMAGE_CHR`; `MsDamageSetMotion`, `MsSetTargetsMotion`, `MsSetRamChrData` |
| `+0x423` | `u8 (bool)` | `stat_appear_invisible_flag` | property `0xa9` `STAT_APPEAR_INVISIBLE_FLAG`; `MsBtlReadMonsterProcess`, `MsBtlChrRegist`, `MsSetChrTransparentVisible` |
| `+0x424` | `u8` | `stat_appear_count` | property `0xe9` `STAT_APPEAR_COUNT`; `MsMonShuffleVariants_007842f0`, `MsGetChrStatInfo` |
| `+0x425` | `u8 (bool)` | `stat_avoid_flag` | property `0xab` `STAT_AVOID_FLAG`; `MsResetAvoidFlag`, `MsCommandReturnChrStatusAll`, `Native_709d_WaitActorField425` |
| `+0x426` | `u8 (bool)` | `stat_adjust_pos_flag` | property `0x76` `STAT_ADJUST_POS_FLAG`; `MsResetChrPos`, `MsMoveCheckFlag`, `MsGetChrStatInfo` |
| `+0x433` | `u8 (bool)` | `stat_hit_terminate_flag` | property `0xa5` `STAT_HIT_TERMINATE_FLAG`; `MsBtlReadMonsterProcess`, `MsGetRamChrCTBbasic`, `MsDamageSetMotion` |
| `+0x437` | `u8 (bool)` | `stat_neck_target_flag` | property `0x14f` `STAT_NECK_TARGET_FLAG`; `MsSetRamChrData`, `MsSetHeadTarget`, `MsGetChrStatInfo` |
| `+0x438` | `u8` | `neck_target_id` | `MsSetDeath` |
| `+0x43b` | `u8` | `stat_win_pose` | property `0xdc` `STAT_WIN_POSE`; `MsBattleProcess_Status_007917d0`, `MsSetRamChrData`, `MsGetChrStatInfo` |
| `+0x43c` | `i8` | `stat_win_se` | property `0xf5` `STAT_WIN_SE`; `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x43d` | `u8 (bool)` | `stat_appear_motion_flag` | property `0x86` `STAT_APPEAR_MOTION_FLAG`; `MsBtlReadMonsterProcess`, `MsBtlChrRegist`, `MsSetRamChrData` |
| `+0x43e` | `u8` | `stat_live_motion` | property `0x5e` `STAT_LIVE_MOTION`; `MsDamageSetMotion`, `MsRelife`, `MsSetChrVisibleBtl2Event` |
| `+0x442` | `u8` | `stat_num_print_element` | property `0x152` `STAT_NUM_PRINT_ELEMENT`; `MsSetRamChrData`, `MsCalcCursorPos`, `MsGetChrStatInfo` |
| `+0x446` | `u8` | `stat_command_type` | property `0xe0` `STAT_COMMAND_TYPE`; `MsAutoRelifeProcess`, `MsParseCommand`, `MsDeleteCommandCue` |
| `+0x447` | `u8` | `stat_inv_motion` | property `0x77` `STAT_INV_PHYSIC_MOTION`; `DmgCalc_ResultType`, `MsSetRamChrData`, `MsGetChrStatInfo` |
| `+0x448` | `u8 (bool)` | `stat_wait_motion_flag` | property `0x7a` `STAT_WAIT_MOTION_FLAG`; `MsEffectSetMotion`, `MsSetMotion`, `MsGetChrStatInfo` |
| `+0x44a` | `i8` | `stat_near_motion` | property `0xf7` `STAT_NEAR_MOTION`; `MsParseCommand`, `MsCheckChrNear`, `MsGetChrStatInfo` |
| `+0x44b` | `i8` | `stat_near_motion_set` | property `0xfa` `STAT_NEAR_MOTION_SET`; `MsCheckChrNear`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x44e` | `u8` | `stat_dmg_dir` | property `0xa1` `STAT_DMG_DIR`; `MsDamageSetMotion`, `Native_700d_ActorMoveTowardTarget`, `MsGetChrStatInfo` |
| `+0x44f` | `u8` | `stat_weak_motion` | property `0x121` `STAT_WEAK_MOTION`; `MsBtlReadPlayerProcess`, `MsEffectEndMotion`, `MsEffectSetMotion` |
| `+0x451` | `u8` | `stat_magiclv` | property `0x85` `STAT_MAGICLV`; `MsParseCommand`, `MsSetLimitParty`, `MsGetChrStatInfo` |
| `+0x4b4` | `u8` | `stat_own_attack_near` | property `0xfd` `STAT_OWN_ATTACK_NEAR`; `MsCheckTargetsNear`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x4b8` | `u8` | `stat_idle2_prob` | property `0xe4` `STAT_IDLE2_PROB`; `MsSetRamChrData` |
| `+0x4b9` | `u8` | `stat_attack_inc_speed` | property `0xe6` `STAT_ATTACK_INC_SPEED`; `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x4ba` | `u8` | `stat_attack_dec_speed` | property `0xe7` `STAT_ATTACK_DEC_SPEED`; `MsSetRamChrData`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x4bb` | `u8` | `stat_motion_num` | property `0xea` `STAT_MOTION_NUM`; `MsBtlChrRegist`, `MsBtlWeaponRegist`, `MsGetChrStatInfo` |
| `+0x4fc` | `i8` | `area` | property `0x1c` `STAT_AREA`; `MsBtlWeaponRegist`, `MsGetChrSbyte4fc_00794280`; **`i8`**, not `u16`: the property and `MsGetChrBtlPos` read single bytes; `+0x4fd` is a byte of its own (`MsGetChrBtlPos`; `MsPushBtlPos` copies `+0x4fc`-`+0x4fe` to `+0x509`-`+0x50b`) |
| `+0x4fe` | `u8` | `pos` | property `0x1d` `STAT_POS`; `MsBtlWeaponRegist`, `MsGetChrBtlPos`, `MsGetChrEscapePos` |
| `+0x4ff` | `u8` | `stat_far` | property `0x1e` `STAT_FAR`; `MsSetTargetsMotion`, `MsGetChrDist`, `MsExchangeChr` |
| `+0x504` | `u8` | `stat_group` | property `0x1f` `STAT_GROUP`; `MsGetChrGroup` |
| `+0x505` | `u8 (bool)` | `flying` | property `0x1a` `STAT_FLY`; `MsGetChrBottomPos`, `MsGetChrCameraPos`, `MsGetChrCenterPos` |
| `+0x508` | `u8` | `stat_adjust_pos` | property `0x5f` `STAT_ADJUST_POS`; `MsBtlReadPlayerProcess`; not 0: no motion may be set (`MsCheckSetMotion`) |
| `+0x509` | `u8` | `stat_move_area` | property `0x55` `STAT_MOVE_AREA`; `MsSetChrSpeedBytes_00795d40`, `MsPopBtlPos`, `MsPushBtlPos` |
| `+0x50b` | `u8` | `stat_move_pos` | property `0x56` `STAT_MOVE_POS`; `MsPopBtlPos`, `MsPushBtlPos`, `MsExchangeChr` |
| `+0x50d` | `u8` | `stat_height_on` | property `0x60` `STAT_HEIGHT_ON`; `MsEffectSetMotion`, `MsResetChrPos`, `MsGetChrBtlPos` |
| `+0x50e` | `u8 (bool)` | `stat_sleep_recover_flag` | property `0x61` `STAT_SLEEP_RECOVER_FLAG`; `MsDamageSetMotion`, `MsSetDeathProcess`, `MsCommandCueProcess` |
| `+0x540` | `ChrName` | `name` | `MsSetSaveData`, `MsSetRamChr`, `Native_7091_ChrCopyName` |
| `+0x590` | `u8` | `gender` | properties `0x16`-`0x18` (`STAT_MAN` ... `STAT_SUMMON`); `MsGetRamChrFemale`, `MsGetRamChrMan` |
| `+0x592` | `u8` | `wpn_inv_idx` | `MsCDreadWeapon`, `MsSetSaveData`, `MsSetRamChrParam` |
| `+0x593` | `u8` | `arm_inv_idx` | `MsSetRamChrParam` |
| `+0x594` | `i32` | `max_hp` | `MsCheatPlayerPowerup`, `MsDebugFullSummon`, `MsCalcDamageCommand` |
| `+0x598` | `i32` | `max_mp` | `MsCheatPlayerPowerup`, `MsCalcDamageCommand` |
| `+0x59c` | `i32` | `base_max_hp` | `MsLimitStatusProcess`, `MsSetRamChrParam` |
| `+0x5a0` | `i32` | `base_max_mp` | `MsLimitStatusProcess` |
| `+0x5a4` | `i32` | `overkill_threshold` | `MsCalcCommand` |
| `+0x5a8` | `u8` | `strength` | property `0x9` `STAT_STR`; `getBtlchrParam`, `setBtlchrParam` |
| `+0x5a9` | `u8` | `defense` | property `0xa` `STAT_VIT`; `getBtlchrParam`, `setBtlchrParam` |
| `+0x5aa` | `u8` | `magic` | property `0xb` `STAT_MAG`; `getBtlchrParam`, `setBtlchrParam`, `MsCalcDamageCommand` |
| `+0x5ab` | `u8` | `magic_defense` | property `0xc` `STAT_SPIRIT`; `getBtlchrParam`, `setBtlchrParam`, `MsCalcDamageCommand` |
| `+0x5ac` | `u8` | `agility` | property `0xd` `STAT_DEX`; `getBtlchrParam`, `setBtlchrParam` |
| `+0x5ad` | `u8` | `luck` | property `0xe` `STAT_LUCK`; `getBtlchrParam`, `setBtlchrParam`, `DmgCalc_Crit` |
| `+0x5ae` | `u8` | `evasion` | property `0xf` `STAT_AVOID`; `getBtlchrParam`, `setBtlchrParam`, `DmgCalc_CheckHit` |
| `+0x5af` | `u8` | `accuracy` | property `0x10` `STAT_HIT`; `getBtlchrParam`, `setBtlchrParam`, `DmgCalc_CheckHit` |
| `+0x5b0` | `u8` | `strength_up` | property `0x12e` `STAT_STR_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo` |
| `+0x5b1` | `u8` | `defense_up` | property `0x12f` `STAT_VIT_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b2` | `u8` | `magic_up` | property `0x130` `STAT_MAG_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b3` | `u8` | `magic_defense_up` | property `0x131` `STAT_SPIRIT_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b4` | `u8` | `agility_up` | property `0x132` `STAT_DEX_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo` |
| `+0x5b5` | `u8` | `luck_up` | property `0x133` `STAT_LUCK_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b6` | `u8` | `evasion_up` | property `0x134` `STAT_AVOID_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b7` | `u8` | `accuracy_up` | property `0x135` `STAT_HIT_UP`; `MsSetRamChrParam`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5b8` | `ChrResistFlags` | `special_resistances` | properties `0x20`-`0x122` (`STAT_SP_HARD` ... `STAT_SP_WAIRO`); `DmgCalc_Armored`, `DmgCalc_DmgImmunity` |
| `+0x5ba` | `u8` | `poison_dmg` | property `0x11` `STAT_POISON_PER`; `MsPoisonProcess`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x5bb` | `u8` | `limit_mode_selected` | property `0x12` `STAT_LIMIT_TYPE`; `MsGetChrLimitType`, `MsLimitTypeDamageCheck`, `MsLimitTypeDeathCheck` |
| `+0x5bc` | `u8` | `limit_charge` | property `0x13` `STAT_LIMIT_GAUGE`; property `0x118` `STAT_LIMIT_GAUGE_ADD`; `MsCheatPlayerPowerup`, `MsCalcMagic`, `MsCheckUseCommand` |
| `+0x5bd` | `u8` | `limit_charge_max` | property `0x14` `STAT_LIMIT_GAUGE_MAX`; `MsCheatPlayerPowerup`, `MsCheckUseCommand`, `MsUseChrMpLimit` |
| `+0x5c1` | `u8` | `wpn_dmg_formula` | `MsCalcDamageCTB`, `MsGetComInfo`, `DmgCalc` |
| `+0x5c3` | `u8` | `stat_icon_number` | property `0x103` `STAT_ICON_NUMBER`; `MsBtlReadMonsterProcess`, `MsGetRamChrIconNumber`, `MsSetChrVisibleBtl2Event` |
| `+0x5c4` | `u8` | `provoked_by_id` | property `0x101` `STAT_PROV_CHR`; `DmgCalc_InflictStatus`, `MsProvOff`, `MsSetProvokingCommand` |
| `+0x5c5` | `u8` | `threatened_by_id` | `DmgCalc_InflictStatus`, `MsThreatProcess` |
| `+0x5c7` | `u8` | `wpn_power` | `MsCalcDamageCTB`, `DmgCalc`, `MsSetRamChrParam` |
| `+0x5c8` | `u8` | `doom_counter` | property `0xa0` `STAT_DEATH_SENTENCE_COUNT`; `DmgCalc_InflictStatus_Extra`, `MsSetDeathSentence`, `MsNumberDrawProcess` |
| `+0x5c9` | `u8` | `doom_counter_init` | property `0x9f` `STAT_DEATH_SENTENCE_START`; `DmgCalc_InflictStatus_Extra`, `MsNumberDrawProcess`, `MsGetChrStatInfo` |
| `+0x5cb` | `u8 (bool)` | `stat_prov_command_flag` | property `0x100` `STAT_PROV_COMMAND_FLAG`; `MsSetProvokingCommand`, `Native_BtlUseCommand`, `MsGetChrStatInfo` |
| `+0x5d0` | `i32` | `hp` | `MsCheatPlayerPowerup`, `MsDebugFullSummon` |
| `+0x5d4` | `i32` | `mp` | `MsCheatPlayerPowerup`, `getBtlchrParam`, `setBtlchrParam` |
| `+0x5d8` | `u8` | `wpn_crit_bonus` | `DmgCalc_Crit`, `MsSetRamChrParam` |
| `+0x5d9` | `ElementFlags` | `elem_wpn` | `MsGetComInfo`, `DmgCalc`, `MsSetRamChrParam` |
| `+0x5da` | `ElementFlags` | `elem_absorb` | properties `0x62`-`0x66` (`STAT_ABS_FIRE` ... `STAT_ABS_HOLY`); `DmgCalc_Elem`, `MsMenuAbsElement_0078f910`, `MsSetRamChrParam` |
| `+0x5db` | `ElementFlags` | `elem_ignore` | properties `0x67`-`0x6b` (`STAT_INV_FIRE` ... `STAT_INV_HOLY`); `DmgCalc_Elem`, `MsMenuInvElement`, `MsSetRamChrParam` |
| `+0x5dc` | `ElementFlags` | `elem_resist` | properties `0x6c`-`0x70` (`STAT_HALF_FIRE` ... `STAT_HALF_HOLY`); `DmgCalc_Elem`, `MsMenuHalfElement`, `MsSetRamChrParam` |
| `+0x5dd` | `ElementFlags` | `elem_weak` | properties `0x71`-`0x75` (`STAT_WEAK_FIRE` ... `STAT_WEAK_HOLY`); `DmgCalc_Elem`, `MsMenuWeakElement_00790240`, `MsSetRamChrParam` |
| `+0x5de` | `StatusMap` | `status_inflict` | `DmgCalc_InflictStatus`, `MsSetRamChrParam` |
| `+0x5f7` | `StatusDurationMap` | `status_duration_inflict` | `MsSetRamChrParam` |
| `+0x604` | `StatusExtraFlags` | `status_inflict_extra` | `DmgCalc`, `MsSetRamChrParam` |
| `+0x606` | `StatusPermanentFlags` | `status_suffer` | properties `0x25`-`0x2c` (`STAT_POWER_BREAK` ... `STAT_THREAT`); `MsCheatPlayerPowerup`, `MsAliveProcess`, `DmgCalc_PowerMagicBreak` |
| `+0x608` | `StatusDurationMap` | `status_suffer_turns_left` | property `0x2d` `STAT_SLEEP`; `MsCalcDrainResult`, `MsSetChrWeak`, `MsBattleProcess_Status_007917d0` |
| `+0x616` | `StatusExtraFlags` | `status_suffer_extra` | properties `0x8f`-`0x9d` (`STAT_LIVE` ... `STAT_DEATH_SENTENCE`); `DmgCalc_InflictStatus_Extra`, `DmgCalc_ShieldBoost`, `MsCheckUseCommand` |
| `+0x62a` | `StatusPermanentFlags` | `status_auto_full_permanent` | `DmgCalc_InflictStatus`, `MsResetAutoStatus`, `MsSetAutoStatus` |
| `+0x62c` | `StatusTemporalFlags` | `status_auto_full_temporal` | `MsCalcCommandTarget`, `MsResetAutoStatus`, `MsSetAutoStatus` |
| `+0x62e` | `StatusExtraFlags` | `status_auto_full_extra` | `DmgCalc_InflictStatus_Extra`, `MsResetAutoStatus`, `MsSetAutoStatus` |
| `+0x630` | `StatusPermanentFlags` | `status_auto_innate_permanent` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x632` | `StatusTemporalFlags` | `status_auto_innate_temporal` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x634` | `StatusExtraFlags` | `status_auto_innate_extra` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x636` | `StatusPermanentFlags` | `status_auto_sos_permanent` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x638` | `StatusTemporalFlags` | `status_auto_sos_temporal` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x63a` | `StatusExtraFlags` | `status_auto_sos_extra` | `MsSetAutoStatus`, `MsSetRamChrParam` |
| `+0x63d` | `u8` | `weak_level_full` | `MsSetChrWeak`, `MsGetChrStatWeak`, `MsSetChrStatInfo`; the pose `MsSetChrWeak` picks (0 with `+0x450`; 3 while KO'd), read by `MsGetChrStatWeak` |
| `+0x63e` | `u8` | `weak_level_hp` | `MsSetChrWeak`, `MsSetAutoStatus`, `MsCheckAutoStatus`; `MsCalcWeakLevel` of HP and maximum HP (`MsSetChrWeak`); 1 and 2 (below a half) add the SOS auto-statuses (`MsSetAutoStatus`), `+0x63f` the level last applied |
| `+0x641` | `StatusMap` | `status_resist` | property `0xaf` `STAT_DEF_DEATH`; `MsSetRamChrParam`, `MsGetChrStatInfo` |
| `+0x65a` | `StatusExtraFlags` | `status_resist_extra` | properties `0xc8`-`0xd6` (`STAT_DEF_LIVE` ... `STAT_DEF_DEATH_SENTENCE`); `DmgCalc_InflictStatus_Extra`, `MsGetChrLibraWord_0078fac0`, `MsSetRamChrParam` |
| `+0x65c` | `u8` | `ctb` | property `0xe8` `STAT_CTB`; `MsCalcDamage`, `MsCalcDamageCTB`, `DmgCalc_InflictStatus` |
| `+0x65d` | `u8` | `default_turn_delay` | `MsCalcDamageCommand`, `MsRelife`, `MsSetFirstCTB` |
| `+0x65e` | `u8` | `cheer_stacks` | `MsCalcDamageCommand`, `MsClearStatus` |
| `+0x65f` | `u8` | `aim_stacks` | `DmgCalc_CheckHit` |
| `+0x660` | `u8` | `focus_stacks` | `MsCalcDamageCommand` |
| `+0x661` | `u8` | `reflex_stacks` | `DmgCalc_CheckHit` |
| `+0x662` | `u8` | `luck_stacks` | `DmgCalc_Crit`, `DmgCalc_CheckHit`, `MsClearStatus` |
| `+0x663` | `u8` | `jinx_stacks` | `DmgCalc_Crit`, `DmgCalc_CheckHit` |
| `+0x664` | `AbilityMap` | `abilities` | `MsSetRamChrAbility` |
| `+0x670` | `LimitAbilityMap` | `limits` | Fahrenheit, not checked |
| `+0x6bc` | `AutoAbilityEffectsMap` | `auto_ability_effects` | properties `0x3a`-`0x4a` (`ABILITY_SEE` ... `ABILITY_PIERCE`); `DmgCalc_Armored`, `DmgCalc_CheckCounter`, `DmgCalc_MagicBooster` |
| `+0x6ce` | `u8` | `stat_use_mp0` | property `0x102` `STAT_USE_MP0`; `MsGetCommandMP`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x6d1` | `u8` | `summoned_by_id` | property `0x106` `STAT_SUMMONER`; `MsGetSummonerNum`, `Native_70d5_SelSetField6d1`, `MsExchangeSummon` |
| `+0x6d2` | `u8` | `regen_strength` | `AfterDamageProcess`, `MsBattleProcess_Order_00790fb0`, `MsSetAutoStatus`; cleared when the regen counter `+0x612` starts (`MsSetAutoStatus`) |
| `+0x6db` | `u8` | `stat_attack_num` | property `0xf6` `STAT_ATTACK_NUM`; `MsCalcCommand`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x6df` | `u8` | `stat_command_exe_count` | property `0x114` `STAT_COMMAND_EXE_COUNT`; `MsSetChrPosAfterSummon`, `MsParseCommand`, `MsGetChrStatInfo` |
| `+0x6e0` | `u8` | `stat_consent` | property `0x115` `STAT_CONSENT`; `MsCommandCueProcess`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x6e1` | `u8` | `stat_energy` | property `0x117` `STAT_ENERGY`; `MsAddChrGain`, `MsGetSummonEnergy`, `MsGetChrStatInfo` |
| `+0x6e4` | `i32` | `current_hp` | `MsCalcDamage`, `MsCalcDamageCTB`, `MsCalcDamageCommand` |
| `+0x6e8` | `i32` | `current_mp` | `MsCalcDamage`, `MsCalcDamageCTB`, `MsCalcDamageCommand` |
| `+0x6ec` | `i32` | `current_ctb` | `MsCalcDamage`, `MsCalcDamageCTB`, `MsCalcDamageCommand` |
| `+0x700` | `i8` | `stat_death_status` | property `0x126` `STAT_DEATH_STATUS`; `MsDamageCheckDeath`, `MsRelife`, `MsGetChrStatInfo` |
| `+0x704` | `u32` | `bribe_gil_spent` | `DmgCalc_Bribe` |
| `+0x718` | `u8 (bool)` | `stat_limit_bar_flag` | property `0x89` `STAT_LIMIT_BAR_FLAG`; `MsGetChrLimitBarFlag`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0x719` | `u8` | `stat_limit_bar_flag_cam` | property `0x88` `STAT_LIMIT_BAR_FLAG_CAM`; `MsCommandReturnChrStatusAll`, `MsGetChrLimitBarFlag`, `MsGetChrStatInfo` |
| `+0xd34` | `i32` | `damage_hp` | `MsCalcDamage`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xd38` | `i32` | `damage_mp` | `MsCalcDamage`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xd3c` | `i32` | `damage_ctb` | `MsCalcDamage`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xdc8` | `u8` | `in_battle` | property `0x15` `STAT_INBATTLE`; `MsBtlReadManage`, `MsBtlReadWeapon`, `MsGetSaveWindow`; the actor takes part: commands, lists and auto commands need it (`MsAutoCommandStatusCheck`) |
| `+0xdca` | `u8` | `stat_target_list` | property `0x127` `STAT_TARGET_LIST`; `MsGetSaveWindow`, `MsChangeMonsterDir`, `MsGetChrStatInfo` |
| `+0xdcc` | `u8` | `stat_death` | `MsDamageCheckDeath`, `MsRelife`, `MsSetChrWeak`; `MsGetChrStatDeath`; 2 KO'd (`g_BtlCameraKoHeight`) |
| `+0xdcd` | `u8 (bool)` | `stat_escape_flag` | property `0xad` `STAT_ESCAPE_FLAG`; `MsGetSaveWindow`, `MsDamageCheckDeath`, `MsSetDeath` |
| `+0xdce` | `u8` | `stat_stone` | `MsBtlReadPlayerProcess`, `MsEffectSetMotion`, `MsCalcCommand`; petrified: `MsGetChrStatStone` (`0x00795800`) returns it signed; it blocks auto commands (`MsAutoCommandStatusCheck`) |
| `+0xdd0` | `u8 (bool)` | `should_try_capture` | `MsCalcCommand`, `MsDamageCheckDeath`, `MsRelife` |
| `+0xdd1` | `u8 (bool)` | `captured` | property `0x156` `?`; `MsDamageCheckDeath`, `MsGetChrStatInfo` |
| `+0xdd2` | `u8 (bool)` | `stat_exist_flag` | property `0x8e` `STAT_EXIST_FLAG`; `MsBattleProcess_Status_007917d0`, `MsGetBattleEndStatus`, `MsSetRamChrData` |
| `+0xdd3` | `i8` | (`stat_blow_exist_flag`) | property `0xac` `STAT_BLOW_EXIST_FLAG` (case `0x007b392a`: `movsx [edi+0xdd3]`); `MsGetBattleEndStatus` |
| `+0xdd6` | `u8` | `stat_action` | property `0x51` `STAT_ACTION`; `MsBattleProcess_Order_00790fb0`, `MsSetRamChrData`, `MsLimitTypeTurnCheck` |
| `+0xdd7` | `u8 (bool)` | `in_ctb_list` | property `0x53` `STAT_CTB_LIST`; `MsBtlReadMonsterProcess`, `MsGetCTBlist`, `MsGetChrStatInfo` |
| `+0xdd8` | `u8 (bool)` | `in_hp_list` | property `0xd7` `STAT_HP_LIST`; `MsGetSaveWindow`, `MsSetRamChrData`, `MsGetChrStatInfo` |
| `+0xdd9` | `u8` | `stat_cursor` | property `0x52` `STAT_CURSOR`; `MsGetChrCursorFlag`, `MsCheckTargetsGone`, `MsSetRamChrData` |
| `+0xdda` | `u8 (bool)` | `stat_effect_target_flag` | property `0xe1` `STAT_EFFECT_TARGET_FLAG`; `MsEffectCheckTarget`, `MsEffectGetTargetFlag`, `MsGetChrStatInfo` |
| `+0xddb` | `u8 (bool)` | `stat_regen_damage_flag` | property `0x151` `STAT_REGEN_DAMAGE_FLAG`; `MsCheckStatusBeforeAction`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xddd` | `u8` | `stat_event_chr` | property `0x50` `STAT_EVENT_CHR`; `MsGetChrEventFlag`, `Native_704f_SelSetFieldDdd`, `MsGetChrStatInfo` |
| `+0xdde` | `u16` | `stat_blow_exist_flag` | `MsGetChrEventFlag`, `MsSetRamChrData`, `MsGetChrStatInfo`; **a `u16`, not the blow-away flag**: an id (`0xff` none) that `MsGetChrEventFlag(i, id)` (`0x00794f20`) compares: with `+0xddd` set and `+0xdde` neither `0xff` nor `id` it returns 2; property `0x159` reads the word. The blow-away flag (property `0xac`) is `+0xdd3` |
| `+0xdeb` | `u8` | `steal_count` | property `0x79` `STAT_STEAL_COUNT`; `Battle_Steal`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xdec` | `u8` | `stat_will_die` | `MsCalcDamage`, `MsRelife`, `MsGetChrStatInfo`; property `0x1b` `STAT_WILL_DIE` counts it (clear instead of set while `g_BtlStatInvert1B`) |
| `+0xdf9` | `u8` | `stat_cursor_element` | property `0x87` `STAT_CURSOR_ELEMENT`; `MesSetChrTargetPosToLimitSparePosy`, `MsSetRamChrData`, `MsCalAndGetChrTargetPos` |
| `+0xe0c` | `u8` | `stat_effvar` | property `0x80` `STAT_EFFVAR`; `MsGetChrEffectVariable`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xe0e` | `u8` | `stat_effect_hit_num` | property `0xaa` `STAT_EFFECT_HIT_NUM`; `MsSetRamChrData`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xe10` | `u8` | `stat_magic_effect_ground` | property `0xe2` `STAT_MAGIC_EFFECT_GROUND`; `MsMagicEffectSet`, `MsMagicEffectStart`, `MsGetChrStatInfo` |
| `+0xe11` | `u8` | `stat_magic_effect_water` | property `0xe3` `STAT_MAGIC_EFFECT_WATER`; `MsMagicEffectStart`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xe1a` | `i16` | `stat_sound_hit_num` | property `0x104` `STAT_SOUND_HIT_NUM`; `MsBattleProcess_ChrDamage_007911e0`, `MsParseCommand`, `MsGetChrStatInfo` |
| `+0xf74` | `u8` | `stat_info_mes_id` | property `0xeb` `STAT_INFO_MES_ID`; `MsGetMenuChrInfo`, `MsSetRamChrData`, `MsGetChrStatInfo` |
| `+0xf75` | `u8` | `stat_live_mes_id` | property `0xec` `STAT_LIVE_MES_ID`; `MsGetMenuChrInfo`, `MsGetChrStatInfo`, `MsSetChrStatInfo` |
| `+0xf78` | `ptr` | `ptr_script_chunks` | `Mon_AttachData`, `Mon_LoadAll` |
| `+0xf7c` | `ptr` | `ptr_script_data` | `Mon_AttachData`, `MsAtelRequestExe`, `MsAtelGetHandler` |
| `+0xf80` | `ptr` | `ptr_base_stats` | `Mon_AttachData`, `MsSetRamChr` |
| `+0xf84` | `ptr` | `ptr_base_en_stats` | `Mon_AttachData` |
| `+0xf88` | `ptr` | `loot` | `MsCalcCommand`, `MsDamageCheckDeath`, `AfterDamageProcess` |

## Known original bug: more than 16 hits overrun a target's results

Each actor has two action buffers of 0x2d8 bytes, at `+0x774` and `+0xa4c`
(`MsGetActionBuffer`, `0x0078d040`, hands out the first whose `+2` is 0xff
and clears it). A buffer is a results block: a 0x18-byte head (`+0` the
result count, `+1` the count after the call), then 16 results of 0x2c bytes
from `+0x18` (0x18 + 16 · 0x2c = 0x2d8).

`MsCalcDamage` (`0x00789740`) appends `hits_n` results without a limit. The
loop at `0x007897a3`-`0x00789858` runs `i` from the head's count while
`i < count + hits_n` and writes result `i` at `results + 0x18 + i · 0x2c`;
nothing compares `i` with 16. `MsCalcDrainResult` (`0x0078a240`) does check
(`cmp byte [ebx+1], 0x10` at `0x0078a25a`), so the cap was intended.

`hits_n` is the target's count from `MsCalcCommandTarget` (`0x0078ba70`):
the command's hit count (`+0x2b`), with the counts of every target a guard
covers (`MsGetGuardChr`) or a reflect sends back added to one actor. A
command with more than 16 hits on one actor therefore writes past its
buffer: from the second buffer (`+0xa4c`, ending at `+0xd24`) onward into
the actor's later fields. In the oracle tests (`tests/battle_calc.rs`) a
fake record with a large `+0x2b` reached the actor's spoils pointer
(`+0xf88`), which `Battle_Steal` then read as a pointer (an emulator fault
in the original, the same read in the Rust). The game's own commands hit
at most a few times, so this needs many guards or reflects stacking on one
actor. `ffx_battle` keeps the behaviour; the tests limit hits to 0-4 and
three targets per command.

## Applying the results: `AfterDamageProcess`

`MsSetDamage` (`0x0078d980`) runs `AfterDamageProcess` (`0x0078f060`) on
every target of a landing hit; it applies one result of each action buffer
that holds this attacker's hit, per call. The fields it reads (offsets in
the buffer; a result is 0x2c bytes from `+0x18`):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u8` | Results applied so far (counted on) |
| `0x01` | `u8` | Results in the buffer |
| `0x02` | `u8` | Attacker (0xff: free; set to 0xff when done with `flags` bit 10) |
| `0x03` | `u8` | The attacker's hit (`MsGetChrCommand(i) + 2`) |
| `0x04` | `u8` | Transfers, cleared once applied: bit 0 the attacker's overdrive gauge `+0x5bc` moves to the target, bit 1 a command learnt (Lancet: target 3, the command at `+6` of the `+0xf88` of actor `+0x17`) |
| `0x05` | `u8` | Progress: bit 0 something happened, 1 the reaction queued, 2 a result chosen for the motion |
| `0x06` | `u8` | Critical (for `MsLimitTypeDamageCheck`) |
| `0x07` | `u8` | An item was stolen (message 4), its count `+0x08` (`i16`) and ID `+0x0a` |
| `0x0c` | `u8` | Gil was stolen (message 8), the amount `+0x10` (`i32`) |
| `0x17` | `u8` | The actor whose command is learnt |

| Result offset | Type | Meaning |
|---|---|---|
| `0x00` | `i8` | Motion kind (`MsDamageSetMotion`; 5, 6 and 8 randomised with `flags` bit 5) |
| `0x01` | `i8` | Word shown: 1 kind 3, 2 kind 4 (`MsNumberRegist`) |
| `0x02` | `u8` | Number flags |
| `0x03`, `0x04` | `u8` | Status bits and count for `MsLimitTypeStatusCheck` |
| `0x06` | `u8` | Overdrive statuses (`MsLimitStatusProcess`) |
| `0x07`-`0x13` | `u8[13]` | The statuses' counts (actor `+0x608`) |
| `0x14` | `u16` | Statuses (actor `+0x606`) |
| `0x16` | `u16` | Statuses (actor `+0x616`) |
| `0x18` | `u16` | Bits 0-2 HP, MP, CTB lost; higher bits the statuses warded (`MsStatusDefenseEffect`) |
| `0x1c` | `i32` | Amount for overdrive mode 0x13 |
| `0x20`, `0x24`, `0x28` | `i32` | HP, MP and CTB lost |

The original keeps the result pointer in its own `j` argument slot
(`[ebp+0x10]`) after loading `j` into a register; the slot is the
caller's argument area, which no caller reads back.

## The end of a command: `MsBattleProcess_ChrDamage`

Each frame `MsBattleProcess_ChrDamage_007911e0` (`0x007911e0`) walks the
31 actors' command state `+0xdf0` (0 none, 1 ended, 4 waiting for its
next hit, 5 ending, else hitting). A hitting actor lands all its hits
(`MsSetDamage(i, -5, 0x400)` until it reports no entries left), and on
its cue entry's last hit (`MsGetChrCommand2(i)`, `+2` + 1 ≥ `+3`) poison
acts. `MsGetChrCommand2` returns null for an actor without a cue entry
(`+0xde5` 0xff) and the original reads `+2` and `+3` through it
unchecked: such an actor in this state would fault, so the game never has
one. The port keeps the read. An ended command (1) runs the clean-up:
by `+0xdea`, 1 places the aeon (`MsSetChrPosAfterSummon`, `0x007ac6a0`),
2 and 3 bring the party back (`MsReturnSummon`, `0x007aef20`, 3 reviving
the KO'd), 4 and 5 escapes, 6 frees the actor (`MsBtlChrFree`,
`0x00793410`, flags 0xc), 7 the party leaves. `MsMagicEffectEnd`
(`0x0079e830`) ends actors 0x10's and 0x11's effects first for an actor of
kind 8, so one of those two with kind 8 would recurse without end.

## The end of a battle: `MsBattleProcess_Status`

`MsBattleProcess_Status_007917d0` (`0x007917d0`, `ffx_battle::battle_end`)
runs the battle's end from `g_BtlStatus` (`0x0112a8e0`), one step a frame:

| State | Waits for | Then |
|---|---|---|
| 1 | an end status (`MsGetBattleEndStatus`) | 2 |
| 2 | no command queued (`g_BtlCommandCueCount`), no damage numbers | with an aeon out (`g_BtlSummoner`, not `0x0112ca11`, script `0x7118`) and the party gone (1) or unable to act (3): Yojimbo's death action (`MsDeathYoujinbou`), a command for an aeon whose `+0x702` is 1 or the party back (`MsReturnSummon(0)`): 1. No end: 1. Else 0x17 |
| 0x17 | no sound read, stream, shout voice or system message | the end kept in `g_BtlEndForced`, system script check 4: 0x18 |
| 0x18 | no script camera, the same | message windows closed; end 1 with battle-end status bit 9 clear (`AtelIsBattleEndStatusClear9`): music `0x8c`, 0x19; end 2 with bit 8 clear: music `0x8a` (the fanfare), 0x19; else 0x1b |
| 0x19, 0x1a, 0x1c | | the music fades out, the end's music read (`MsMusicReadStart`), played (`MsMusicSync`), then 0x1d |
| 0x1b | no actor moving, turning or changing (`+0x508`, `+0x42e`, `+0x415`, `+0x41c` with `+0x41d`) | 0x1c with music, else 0x1d |
| 0x1d | | end directions (`0x00793290`), the end kind's system script check (0, 1, 2, 3 for ends 2, 1, 3/5, 4) and closing camera (`0x42`-`0x45`), fade frames, overdrive experience (`MsLimitTypeWinCheck`): 0x1e |
| 0x1e | actors' effects (`+0x432`, `+0x436`) and the vfx counter; the closing event effect (`0x0112c94e`, script `0x711c`) read and started | victory motion `0x2d`, 3D sounds faded, the screen fade (`Sg_FadeOut`): 0x1f |
| 0x1f | the fade (`0x0112c9ed`), `0x0112ca01` | the event effect freed (`MsEffectForceFree`): 0x20; else 0x21 |
| 0x20 | no event effect | 0x21 |

States 3-0x16 and anything outside 1-0x20 do nothing. The globals it
keeps are in `symbols/ffx.sym` (`g_BtlEnd*`, `0x0112c9ec`-`0x0112c9f4`).
`dbgPrintf` (state 0x1b's wait message) and `FlushCache` are empty in
this build; the ports leave them out.

## Sound transfers and 3D sounds each frame

`MsSEPtransProcess` (`0x007a0540`) works through the transfer queue at
`0x011360c8` (16 records of 12 bytes: data pointer, size, type (0 sound
effects only, else a wave), actor (0xff the scene's), state (0 queued, 1
sent, 2 wave standing by)), one step a frame while `g_BtlSepTrans`
(`0x0113618c`) is above 0: a queued record is sent
(`SndSepTransSepOnly` or `SndStandbyBattleWave`); a sent one, once
`SndSyncWaveTrans(0)` reports done, either stands its wave by (type 1,
`SndStandbySepWave`) or ends: its actor's count `+0xf2d` down (at 0
`+0xf2c` set) or the scene's (`0x0112c02a`, at 0 `g_SepTransFlag` set and
the scene's buffer freed, `MsBtlReadSceneFree`), the queue index on
(signed `% 16`). The empty `dbgPrintf` there is passed the record's first
data byte, so the original reads through the data pointer even though
nothing uses the value; the port keeps the read.

`MsSEP3Dprocess` (`0x007a0930`) walks `g_Sep3dList` (`0x011361a0`): a node
counts its delay `+3` down first; then one no longer playing is unlinked,
a playing one's volume (0x100, or while fading `(+6 << 8) / +4` after
`+6` counts down, the sound stopped at 0) and pan follow its actor
(`MsSEP3DPos`, `op_3d_vol_pan_get`, whose volume comes back through a
frame local). A node linked to itself is cut when the walk reaches it.

## The model read queue

Characters that join a battle mid-way (party members swapped in, aeons
called: `MsExchangeChr`, `0x007adae0`) get their files read through a
queue of 32 entries of 12 bytes at `0x0112bea0`, the first entry's index
at `0x0112c020` and the count at `0x0112c021` (`MsBtlReadCueRegist`,
`0x007ad3f0`, appends unless full; `MsBtlReadCueManage`, `0x007ad190`,
works through it):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u8` | Kind: 0 the character's data (`Ch_DataRead`), 2 its motion group (`ChN_ReadSystemMGRP`, the group `+0x20` of the actor), 3 its weapon (`MsCDreadWeapon`) |
| `0x01` | `u8` | Actor (`i`, or `i + 0x1f·(k+1)` for weapon `k`) |
| `0x02` | `u8` | State: 0 to start, 1 reading, 2 read |
| `0x03` | `u8` | What follows the read: 1 `MsBtlReadPlayerModelProcess`; 2, 6, 8, 10 `MsBtlReadPlayerProcess`; 3, 7 `MsBtlReadMonsterProcess`; 4, 5 the owner's weapon `k` = 0, 1 registered (`MsBtlWeaponRegist`) |
| `0x04` | `u8` | Owner actor |
| `0x06` | `u16` | File ID |
| `0x08` | `u32` | Cleared, unused here |

An exchange queues the actor's weapon (kind 3, then 0), its data (kind 0)
and motion group (kind 2, then 0xa out of battle, 8 for an aeon, 0 for an
immediate join, 6 for one walking in) and its weapon models
(`MsBtlReadCueRegistWeapon`, `0x007ad470`: kind 0, plus kind 2 for the
models listed at `0x00c43984`, `MsCheckBoundWeapon`, `0x007ad130`). The
manager stops at the first read still running; kind 1 entries and kinds
above 3 are never queued (to start, the manager would spin on them).

## Stack garbage in `MsCheckBtlPos`

`MsCheckBtlPos` (`0x007a9230`) reads an actor's position
(`MsGetChrPos`) and battle position (`MsGetChrBtlPos`, `0x007948e0`)
into two vectors on its stack and compares them with `MsPointLen`
(`0x007c2ec0`), which loads the first into `VF_M`'s first row
(`0x00c8f7d0`), the second into its third row (`0x00c8f7f0`) and leaves
the difference in the first row. When the actor's position record cannot
be located (`MsBtlGetPos`, `0x007abfe0`: group `+0x4fc` beyond the
position file's count, or a bad kind or index), `MsGetChrBtlPos` leaves
the vector's x and z alone and lifts its y: the original then subtracts
whatever its stack held. A write watch on those words (round 13) caught
`MsPointLen` storing x = `0x6e` and `0x71` this way, the first row's x
`0x8000006e` (0 − x). The value is read: `MsPointLen`'s length decides
`MsCheckBtlPos`'s result, so the original's behaviour there depends on
stack contents. The port uses zeros and does not try to reproduce it.

One way a battle reaches it: an aeon summoned in by an exchange
(`MsExchangeChr`, `0x007adae0`) takes the summoner's group (`+0x4fc`), and
with no summoner (0xff) that is the spare record's. The oracle tests
require every actor to be placed before such calls; after one, if an
actor lost its place, they copy those two rows from the original.

## A command's effect: `MsEffectStart`

`MsEffectStart` (`0x00788480`, `ffx_battle::effect`) runs once a frame
from `MsBattleProcess` and moves each actor's effect state `+0xdfb` on:

| State | Waits for | Then |
|---|---|---|
| 1 | by `+0xdea`: 1 (the party swapped, `MsExchangeParty`) and 8 (an overdrive's party, `MsSetLimitParty`) no actor's motion (`MsCheckChrMotionAll(1)`); 2, 3 set `0x0112ca14`; then the actor still (`+0x508`, `+0x436`, `+0x435`) | the turn's end asked, voice 9, the command title closed, 2; the start motion `+0xe00` (0xff: 3 at once), the weak pose with `+0x413`, a weapon special's motion 8 once (`+0xdfc`), the script camera `+0xe04`/`+0xe08` when there is an effect (`+0xf8c`) |
| 3 | `+0xdfd` clear, no target moving (`MsCheckStartEffect`), no reset wait (`0x0112c9d6`), no sound read or stream, no system message, the debug hold (`0x0112a91c`) off, no script camera request running (`g_BtlCameraState` with `0x0112a8e5` 1) | `MsEffectLimitCheck`: 0xc for an overdrive command (`+0x58` 0x20, 0x25, 0x30, 0x40; its window opened for Tidus, Auron and Lulu), else 0xb, handled the same frame; the camera script (`0x00788ef0`: the magic file's `g_BtlEffectReadStat`, else the effect's `+0x1c`, `MsAtelCamRegist`) |
| 5 | | the reflect step (`0x00789030`) |
| 7, 8, 15 | the actor still and its motion `+0x3f0` none of its attack motions (`ms_motion_trans` 12, 13, 2-5) | back to idle (`0x00787d70`): 0 (7 also `+0x405`), 9 for 8 |
| 9 | `+0xf65` frames, not moving | `MsWeaponRead(i, +0xf64, +0xf66)` |
| 10 | `+0xdfd` clear | the end (`0x00788f90`, motion `+0xe01`), 6 |
| 11 | | no effect: the end (motion `+0xe01` with `+0x413`, else 0x33), 6. An effect: 4, the debug camera sleep kept aside, the effect's function (`+0xf8c`'s first word) called with `MsGetEffID(+0xe1c, 2, i)` |
| 12 | the overdrive's window (`+0xd24`) | the screen darkened a step a frame (`0x0112ca07` up to 6, `Sg_FilterSub(0, 0, 0, (16 - n) * 8)`), then 13 |
| 13 | | lightened back, then `Sg_FilterReset` and 11 |
| 14 | | 0, the last magic read (`MsEffectRead`) |

An effect reports back through `MsEffectReady(id, x)` (`0x00788200`):
an actor's (`id >> 8` 2) in state 4 ends (`0x00788f90`) and goes to 5.
The reflect step lands the hit on the cue entry's targets once (cue `+5`
0 → 1: head target reset, overdrive status check, the reflect effect
`MsStatusEffectReflect` and sound 0x3b) and waits until none of them
shows its reflect effect (`+0xea4`, 1 → 2), then ends (`+0xe0d`).
`0x00788f30` rewrites its own two argument slots before its tail call to
`MsEffectCameraCall(-1, 1)`; no caller reads them back.

`ms_motion_trans` returns 0 (and sets `+0x3f3`) for an actor whose model
is listed at `0x01133398` (a count, then models) while the magic file's
ID is 0x1b2 or 0x262; it asks the ID a second time when the first is not
0x1b2.

`MsMagicEffectEnd` in state 11 recurses without end for actors 0x10 or
0x11 of kind 8 (above); the tests leave that out.

## Saved parameters: `MsSetSaveParam`

`MsSetSaveParam(i)` (`0x007860f0`, `ffx_battle::save_param`) rebuilds
character `i`'s record in `g_PlySave` (`0x0113205c`, 0x94 bytes each):

| Offset | Type | Meaning |
|---|---|---|
| `0x04`, `0x08` | `i32` | Base HP and MP (the sphere grid adds 50 per HP node, 5 per MP node: `g_BtlAbilityMap` `+0`, `+4`) |
| `0x0c`-`0x13` | `u8[8]` | Base stats (the grid's `+8`-`+0xf` added) |
| `0x1c`, `0x20` | `i32` | Current HP and MP, clamped to the maximums |
| `0x24`, `0x28` | `i32` | Maximum HP (9999, 99999 with `+0x4c` bit 9) and MP (999, 9999 with bit 10) |
| `0x2c` | `u8` | Bit 0 joined (`MsGetSavePlyJoin`) |
| `0x2d`, `0x2e` | `u8` | Weapon and armour (equipment slots; 0xff none) |
| `0x2f`-`0x36` | `u8[8]` | Stats (strength, defence, magic, magic defence, agility, luck, evasion, accuracy), 1-255; evasion (`0x35`) from 0 |
| `0x3e`-`0x49` | `u16[6]` | Commands learnt (the grid's `+0x10`-`+0x1b` ORed in) |
| `0x4a`-`0x4f` | `u16[3]` | Auto-ability bits of the equipment (the abilities' `+0x62`-`+0x66`); `0x4e` bit 1 no encounters (clears `g_BtlEncountOn` when joined) |

Each of the two pieces' four abilities (equipment `+0xe`) adds its
percentage `+0x55` to the values its bits `+0x56` name (HP, MP, the
stats, then four damage rates written to `0x02311240 + 4i`), and the values
grow by that percentage. An aeon (8 and up) has no grid: its HP, MP and
stats come from Yuna's (character 1) through its `ply_rom` record
(`+0x18`-`+0x29`: per stat a divisor of the sum `t` of Yuna's stats, HP/100
and MP/10, and a multiplier of her stat, /10), Luck (`0x34`) is Yuna's, and
once 30 battles are fought (`0x011307a4`) the floors of
`g_KernelSumAssure` (`0x0112a948`, the kernel's `sum_assure.bin`, record `i + (5n − 9) · 2` for
`n` = battles / 30 up to 20) raise HP, MP and the stats but Luck; its own
record's `+4`-`+0x13` are added on top. A zero divisor faults the original
(a monster's record has them, so it is never called for one); the port
divides to 0 there. Kimahri (5) learns `0x3078`-`0x308a` for each of
`0x3041`-`0x3053` he has.

`MsSetSaveData` (`0x00785fc0`) writes the 18 characters back at a battle's
end: weapons from the actor's `+0x592`, `+0x593`, HP and MP (1 HP for the
KO'd; with a stand-in count `+0x6d8` that runs out, full, the rest kept at
`+0x3d`), overdrive `+0x5bb`-`+0x5bd` to `+0x38`-`+0x3a`, then the aeons'
parameters. `MsEquipSaveWeapon` (`0x007ab970`) writes the piece's owner
(`+6`) through `MsGetSaveWeapon` unchecked: an ID the equipment lacks
writes to address 6 in the original. `MsWeaponRead` (`0x007ad820`, effect
state 9) swaps a weapon mid-battle: the same model fades in over 8 frames,
another is disposed of and its reads queued (`+0xdfd` set meanwhile).

## Battle messages and the script cameras' reset

`MsMessageCueProcess` (`0x0079cdf0`, `ffx_battle::battle_end`) shows the
queued battle messages (`g_BtlMessageCue`, a ring of 4 entries of 12
bytes from `g_BtlMessageCueStart`) one at a time. An entry's kind `+0`
picks the window (`TOBtlDraw*MessageWindow`, system: 1 a name and message,
2 and 3 the first strikes, 4 an item, 5 a capture, 6 a command learnt and
7 an overdrive type, both with sound 0x39, 8 gil; anything else -1); the
window's result goes to `g_BtlMessageWindow` (`0x0112c910`) and `+2`, `+1`
to `g_BtlMessageTime` (`0x0112c946`: frames left, counted down each call,
and the frame count below which a button may close it). A window stays
while its frames last unless pad bit 5 (`g_BtlPad`, `0x0112c904`) is newly
pressed (not in `g_BtlPadOld`, `0x0112c908`) after the delay; then the help
closes (`TOBtlCloseSimpleHelpMes`) and the next message opens in the same
call. A negative result skips the message at once. A window function that
returns 0 makes the original open the same message again without end.
`0x0112c904` and `0x0112c908` are the pad words `MsBattleProcess` stores
each frame (they were named for the battle end before).

`MsAtelResetMoveCameraID(kind)` (`0x007ba860`, `ffx_battle::camera`) resets
the three script cameras of a kind (`0x01136f88`: kind 1 the first three
words, 0 the next three, then 2 and 3) on both sides through
`MsCameraResetMove` (`0x007bed60`): for the camera's current request it
stops the three axis moves (side 1 also the roll and depth moves) and,
unless flag 8, puts the side at its target (`MsCameraSetRect`,
`0x007bf940`, then the depth and roll again). `MsCameraSetRect` writes the
channel's target and, for the current request, the position now and
`MsSetCheckChange` (`0x007c0790`) mode 0, which runs the move step
`0x007bcab0` (not a battle port: polar position, spline) and copies the
targets of axes without a move. Floats are copied through the x87 there
(a signalling NaN comes out quiet).

## Moving the actors: `MsMoveChr`

`MsGetMoveTargetDist` (`0x007ab540`, `ffx_battle::predict`; cdecl, 11
words: `i, kind, with, speed, n, accel, brake, a, b, c, gravity`) starts
a move: frame `+0x452` 0, kind `+0x415` = `kind & 0x3f`, `+0x417` 0, `n`
frames (`+0x45a`, `u16`) split into accelerating `n * accel / 128`
(`+0x454`), braking `n * brake / 128` (`+0x458`, both signed and
truncated) and the rest cruising (`+0x456`); the start speed `+0x45c` is
`speed`, or the speed now (`+0x460`) when `kind` has bit 0x40; `with`
(`+0x416`), `a`, `b`, `c` (`+0x488`, `+0x48c`, `+0x498`), the start
position (`+0x390`, `MsGetChrPos`), the gravity `+0x4a8` (a zero stored
as +0.0 with `+0x419` 0, else `+0x419` 0xff), and the distance to
`MsGetMoveTargetPos`'s point (`+0x4a4`, 2D for a flying actor `+0x34`),
`+0x4a0` 0. For a kind `MsGetMoveTargetPos` writes nothing for (the
summoner's kinds without a summoner) the original measures to its stack
garbage; the port to the zero vector.

`MsCameraMoveCalc(near, &value, start, mv)` (`0x007bda80`,
`ffx_battle::camera`) steps an eased value from `start` towards `*value`
(first brought within pi of `start` with `near`, `MsNearRag`) by the move
block `mv` (eight `i16`): `[0]` the curve, 1 linear (`start + (v - start)
* frame / length`), 3 a cosine (`_CIcos(frame * pi / length)` on a double,
then `start + (1 - c)(v - start) / 2`), 2 accelerate, cruise and brake
(`[2]` the frames whose mean speed starts it, `[3]`, `[4]`, `[5]` the
three phases; the cruising phase also stores its partial sum in `*value`
first); other curves leave `*value`. `[7]` (the frame) then counts up and
past `[6]` (the length) the length becomes 0. 0 for a length of 0
(nothing changes), else 1. `MsGetChrPredictPos` builds its block on its
stack (curve 2, `[1]` left as stack garbage, which curve 2 never reads).

`MsCheckNeckMotion` (`0x007ab700`, was `FUN_007ab700`): 0 for the motions
(`+0x3f0`) 0x20-0x24 and 0x48-0x4c, else -1; `MsMoveChr` sets a neck
target only on -1.

`MsGetChrPredictPos(i)` (`0x007aa2a0`, `ffx_battle::predict`; cdecl, eax
garbage) is one frame of actor `i`'s move, nothing without a character.
By `+0x415 & 0x7f`: 0 none; 7 along move spline `+0x454` (2 is the ball's,
`MsSetBallSpline` first) at `t = frame / length` (`+0x452` after its
increment, `+0x45a`, a double rounded to `f32`): `TkMtCalcSpline` into
the position (`MsSetChrPos`), then `TkMtCalcSplineTangent`, whose yaw
turns `+0x490` by `+0x46c` of the difference (`MsNearRag`; with `+0x42a`
negative a difference past pi/2, positive one below -pi/2, flips sign:
`0x00b11338`/`0x00b3ebb0`) and faces the model (`Ch_SetDirF`); 0x10 a
straight step of `+0x460` along yaw `+0x470`, pitch `+0x47c`
(`MsVectorPolar2Rect`); else towards `MsGetMoveTargetPos`'s point (actor
0's also into `EE_MOVE` `0x00c8f588` and `g_BtlMoveTarget`). Without bit
0x80 the step is `MsGetChrMoveAccelPos` at the incremented frame; with it
the actor steers: the speed `+0x460` grows by `+0x468` up to `+0x464`,
yaw and pitch (pitch only for a walker, `+0x34` 0) turn towards the
target by at most `+0x474`/`+0x480`, which grow by `+0x478`/`+0x484`;
past the length the yaw closes `(frame - length) / (4 length + 1)` of the
rest; past twice the length a new move starts (`MsSetChrDirTarget(i, 8,
0, 0, frames / 4, 1)` and `MsGetMoveTargetDist` of kind `& ~0x80`,
`frames = ftol(dist / +0x464) | 1`, braking 0x20). A move ends (`+0x415`,
`+0x418` 0) at its length. With `+0x419` the height follows the gravity
`+0x4a8` from `+0x394`: `v0 = (y_to - y0 - g n² / 2) / n`, `y = f v0 + y0
+ g f² / 2`. Then, while `+0x42e` counts down, the direction eases: a
curve-2 move block on the stack (`+0x42f`, `+0x430`), `MsGetDirMode` into
`+0x4f4` and, when it has a mode, `MsCameraMoveCalc` from `+0x490` and
`Ch_SetDirF`. The radius words of `MsVectorPolar2Rect` are the caller's
`edi`, `esi`, `ebx` (see `ffx_battle::target`).

`MsMoveChr()` (`0x007aab80`): for each of the 31 actors, one that moves
(`+0x10` or `+0x12`) is bound to the character it rides (`+0x43f`,
element `+0x441`, `Ch_BindToElem`) and steps (`MsGetChrPredictPos`); then
the neck delay `+0x439` counts down, and at 0 the neck follows actor
`+0x438`'s character (`Ch_NeckSetTarget`, unless `MsCheckNeckMotion`
says no) or, for 0xff, stops (`Ch_NeckSetEnable(chr, 0)`). The character
is the one read before the step.

`MsMoveSplineRegist(k, i, kind, point)` (`0x007ab0b0`): for spline `k`
< 3 with fewer than 0x20 points, the count (`g_BtlMoveSplines[k]`,
`0x01136418`, `i32`) grows first and the point goes to `0x01136428 + k *
0x210 + count * 0x10`: `point`'s four words when given (also into
`EE_MOVE`), then `MsGetMoveTargetPos(i, actor i, kind, that point, 0)`
over it (kind 0 writes nothing).

`MsSetBallSpline(i)` (`0x007b78c0`) rebuilds move spline 2 for the ball
thrown by actor `i` to its `+0x14` (the catcher `t`; `MsMoveSplineSet(2)`
first): the catcher's first target `f` (lowest bit of `t`'s `+0xdc0`) and
`t`'s result on it (`MsGetDamageResult(t, f)`, signed `+0x18`). By the
ball state `+0x420` (signed byte; above 3 nothing):

- 0 and 1: an end point. With `f == t` and state 0, 40 units in front of
  `t` (`MsGetChrDir`, `MsVectorPolar2Rect` with 40.0 at `0x00b43db4`).
  Otherwise `MsGetMoveTargetPos` kind 12 (the targets' move positions)
  plus `g_BtlBallOffset` (`0x01136ac4`, set by `MsSetBallSplineInit`);
  with a result of 1 the end moves on by the distance returned
  (`MsVectorNormal2` from the start), else it stays and keeps its height.
  The first two ends take the ground height (`MsGetChrGround` of `t`,
  resp. `f`) unless underwater. The spline: the start `+0x390` twice,
  seven midpoints, the end twice; midpoint `k` (7 down to 1) is
  `MsMiddlePoint(start, end, s, 10 - s)` with `s` growing by a step of
  1.3 that loses 0.001 each time, its height `y + (y0 - y) * k * 0.03125`.
- 2 and 3: from the start twice to `t`'s character's optional position 11
  twice, seven midpoints with the step 1.1, raised by a running sum of
  `v = -1 - len / 3` (`+0x45a`), `v` growing by `-v / 4` each time; then
  state 3 once fewer than 8 frames are left.

Every value is stored as `f32` at each step (`s`, `10 - s`, the step,
the sums). The by-value vectors' spare words (`MsVectorPolar2Rect`'s,
`MsVectorNormal2`'s, `MsMiddlePoint`'s) are the caller's `edi`, `esi`,
`ebx`.

Facts the parity test found (`tests/battle_target.rs`, `move_steps`):
only three move splines exist (`0x01136428 + 3 * 0x210` is
`g_BtlMoveTarget`); a spline move past its length reads points beyond
the table; and several paths read the caller's locals where nothing was
written (a target actor index past 30, a kind without a target with
direction mode 8, an actor without a character whose battle position the
file lacks): the original's stack garbage there, the port's zeros.

## The battle scripts' per-frame updates

`MsAtelSetUp(k)` (`0x00798390`) registers battle script `k` with
`AtelBattleSetUp`, handing it the script control at `0x01134484 + k *
0x54` and a per-frame update: `0x007972e0` for script 0 (it also runs
`MsMotionProcess` and `MsBattleSpecial` between the commands and the
signals), and for scripts 1 and 2 `MsAtelUpdate1` (`0x007973b0`) and
`MsAtelUpdate2` (`0x00797350`), two copies of the same code
(`ffx_battle::atel::ms_atel_update`). Over the control's works (its
`u16` count at `+0xc`, `AtelGetCtrlworkActorTotal`): each work's signals
(`AtelDecodeSignal(k, -1)`), then each one's command (`AtelDecodeCommand(k,
0)`), then the signals again. eax is left as the last `AtelDecodeSignal`'s
result, or the count when there are no works.

## Teardown, motions and the special stage (round 18)

`MsBattleFree` (`0x00781590`) and `MsBattleAllFree` (`0x00780b40`) give the
battle's buffers back (`ffx_battle::free`): each leaf frees its pointer
(`user_free_thunk`) and clears it, except `MsBtlChrFreeMem` (`0x00793500`),
which frees the actors and weapon records and then clears all three tops
(`MsBtlChrInitMem`), and `MsAtelRegistFree` (`0x00797a90`), which frees
both script managers' `+4` and `+0xc` without clearing them before
`MsAtelRegistInit` resets the managers. `0x00782640` frees script record
`i`'s `+4` with `Sg_FreeA(p, 5)`; Switch inlines it and `0x00782750` (the
effect buffer) into its `MsBattleFree`/`MsBattleAllFree`. `MsEtEffectStop`
(`0x0079e7b0`) is only a jump to `op_et_battle_effect_free` while
`g_BtlEtEffectRead` is set. `MsMenuFreeText(i)` (`0x0078fb00`) frees menu
text `i`'s buffer when its `+0` byte is 0 and returns 0 (else -1), then
`MsStructClear`s 12 bytes from the address of its own argument slot: the
caller's stack, not the entry. For `MsBattleAllFree`'s first call that
covers its return address, so the original only survives because text 0
is never freed that way; the port leaves the stack alone.
`MsBtlChrFreeAll` (`0x00793480`) disposes every non-ride model of the
0x5d actors and records, then each actor (`MsDebugFullCommand_00796dc0`,
flags 2 with `+0x22`, 4 with `+0x21`) and `g_BtlChrExtra`.

`MsMotionProcess` (`0x007aa160`, `ffx_battle::motion`) runs over the 0x1f
actors: `MsCalcMotionSpeed` (`0x0078ac20`) sets the speed factor `+0x406`
in eighths (a ride's is its rider's; 0, 4, 8 or 16 by `+0xdce`, `+0x606`
bit 11, `+0x613`, `+0x614` and the motion `+0x3f0`) and the model's speed
(`+0x400`, `Ch_SetMotionSpeed`, starting or stopping the texture
animation timer); then a queued motion `+0x3f2` is set, or an actor with
`+0x508` that `0x007ab750` finds touching nobody (-1) goes back to motion 0
and its counter `+0x428` resets (body part 2 off every fourth time,
`+0x427`, while that counter was below 2); without `+0x508` the counter
counts up to 2. `0x007ab750` is 0 when another of actors 0..0x1b with a
model and `+0x2d` stands closer (2D) to this one's battle position than
the `f32` sum of their widths; that actor goes to `g_BtlEffectWait` with
motion 0xfc.

`MsRotMatrixX/Y/Z(out, m, angle)` (`0x007c3170`, `0x007c3480`,
`0x007c3790`) turn each row's lane pair (`y,z`, `x,z`, `x,y`): `p' = q·(-s)
+ f32(p·c)`, `q' = q·c + f32(p·s)`, through the VU registers (`m` at
`0x00c8f7d0`, the rotation rows at `0x00c8f810`/`0x00c8f820`, the result
from `0x00c8f830`, the last row's products in the accumulator). The
rotation rows' other two lanes each are stored from uninitialized stack
locals; the port leaves them as they were.

The special stage's work (`g_BtlSpecialMem`, kind `g_BtlSpecialKind`: 1
the shaking screen, 2 and 3 the sea) holds 0x1f actor matrices of 0x40
bytes, the sea's matrix at `+0x7c0`, the stage's angles at `+0x800` and the
up vector pointer at `+0x810`. `0x0079df90` takes the sea's matrix from an
optional position of actor 0x14's model (kind 2: 0xe, kind 3: 5),
normalizes its rows and hands it to `Yn_SeaSetMatrix`; `0x0079e030` builds
each actor's matrix (scale `+0x3a0` times 0.01, its direction plus π/2,
then the stage's Z and X angles) at its turned position and makes it the
model's base matrix. Its position's `w` is a local the original never
writes.

`MsBattleSpecial` (`0x0079d430`, round 19) runs the stage each frame
(script 0's update `0x007972e0` calls it after `MsMotionProcess`). Kinds 2
and 3 take the sea's matrix (`0x0079df90`) and set the sea's UV speed to
(0, 6). Kind 1 is the boat: the boat model's (actor 0x17) heading (its
position through `MsVectorRect2Polar`) against the stage's pitch `+0x830`
rocks a roll `+0x838` (±0.00013 a frame, damped by 0.96 and by
`(5 - swings)/5`, the swings `+0x854` counting the roll's sign changes up
to 1 within one heading step), accumulated in `+0x820`; the pitch eases
towards it by 1/16; three swaying parts (`+0x814`, `+0x824`; map objects
0x1c-0x1e, the last level on map 0x3d) follow the heading step; a swell
(`+0x844` from `sin(+0x848)`, `+0x84c` and a `brnd(3)` countdown in
`+0x858`/`+0x85c`/`+0x860`) and the roll's change drive the sea's angles
(`+0x800`), UV speed and waves (object 2, `+0x864`), the boat's objects
0x14, 0x15, 0x18-0x1a and the particle emitter. The original keeps a 0.0
on the x87 stack across most of its body (Ghidra's `extraout_ST0`). Its
locals go to map code (`yiSetRotTransDObj`, `Yn_*`), so the port keeps
them in game memory (a frame on the hook's stack); without the boat model
the original reads three position words it never wrote (zeroes in the
port).

`MsBtlBridgeBtl2Event` (`0x007821f0`) hands the battle's models to the
event script at the end of a battle: status effects end, every model is
reset (thickness -1, its clut freed, shown), then each event actor bound
to a battle character (bind ID 1, a handle) finds its actor (its bind PC
below 0x1c whose `+4` is the event's character ID, else the first such of
0x1f; none: link bit 2), hands over both weapons' and its own model (`+0`
and `+0x22` cleared) and hides them (`Ch_SetHideEvent`) unless its hide
status (0/-2 for a kept party member in or out of the party's first three;
0 with `+0x1c`, -1 without `+0x1b`) is 0, which also sets link bit 6;
scene 0x02370000 on level hash 0x21e8 keeps them hidden either way. Then
`MsBattleFree` and `MsBattleAllFree` run their leaves in order;
`MsBattleAllFree` frees 14 kernel tables without clearing them.
`MsGetSaveConfigEnglish` (`0x007851d0`) is `xor eax, eax; ret`: three
bytes, too small for the hook's jump.

## The battle's start (round 20)

`MsBattleExe(mode, field, group, slot)` (`0x00781020`, `ffx_battle::exe`)
starts a formation: from the field (`mode` 0) the CD object's third method
with 0 first; the event effect ended (`MsEventEffectFree` asserts "EFFECT
SYNC ERROR" if one still runs); the battle file's number
(`MsBtlListCDpos`: the field record's first number `+4` plus the sizes of
the groups before plus the slot, clamping `group` and `slot` in the
caller's own argument slots) and size (`MsBattleExeSetIndex`, directory
0xe, or `MsGetFileSize`'s larger of 0xe and 0xb while
`MsGetCheckEnglish`); nothing without a size. The group's bytes 2 and 1 are
the battle music (`0x0112c254`). The file buffer is a fresh
`Sg_MallocA(n, 0x10, 5, "btlbin", -1)` after `MsBattleFree` from the field,
else script record 0's (or a new aligned `user_malloc`). Ten inits, then 68
stores reset the battle state (their widths overlap neighbours in places),
the scene's list entries (`0x0112a9f0`-`0x0112a9fc`) and the file's name
`"<map>_<nn>"` (`0x0112c25a`, the field record's 8 name bytes and the
formation number). From an event: status 5; from the field: the actors',
items' and numbers' memory, AP, windows, effects (`MsEtEffectSet`), the
encounter effect (`0x007829b0`, `MsEtEffectEncount`), the music, status 4;
then `MsBtlReadManage`.

`MsBtlReadManage` (`0x00783020`) is a state machine over `g_BtlStatus`
(4-0x22 through a jump table): file read, scene, map, actor reads,
scripts, cameras. Its leaves `MsBtlReadScene` (directory 0x29, an async CD
read into `g_BtlRead3`), `MsGetRamChrCTBbasic` (the first actor still
loading `+0x433` to `g_BtlEffectWait` with motion 0xd0), `0x00784420`
(monsters shown, placed, made visible), `MsCalcChrDirAll` and
`MsSetChrVisibleEvent` are ported; the machine itself in round 21.

## Round 21

A test leak found in round 20 (drawing the round-19 steps' random stream
for the new steps made a later `MsEffectStart` divide by zero in the
original, at `0x00786346`: a `g_KernelPlyRom` record's byte `+0x1c`) came
from the test, not the port: `MsBattleExe`'s leaves step handed its heap
stub's 0x8a90-byte block to `MsBtlChrGetMem`, which asks for 0x1e270 bytes
of actors, and `MsBtlChrInit` then initialized all 0x1f actors past the
block's end, over the scratch tables allocated after it (the fake ply_rom
table first). The step's globals were put back, the bytes after the block
were not. The step now gets a block of its own of 0x1e270 bytes; the shared
stream passes on the image and battle.snap.

`MsBtlReadManage` (`0x00783020`, `ffx_battle::read`) runs one step a call
and then the read cue (`MsBtlReadCueManage`). With the debug toggle
`g_BtlDebugReadWait` on, each step first waits `g_BtlReadWait` frames (0x3c
after each camera request); `g_BtlReadHold` counts down every call. The
first visit's path is scenario 0x9ce on level hash 0x223f. By status
(each falls into the next unless it waits; the jump table at `0x0078361c`,
its index bytes at `0x00783660`):

| Status | Step |
|---|---|
| 4 | waits for the script callbacks (`AtelIsCallbackQue`) |
| 5 → 6 | index, name (copied to `g_BtlFileName`), async task `0x00784500`, `g_BtlFileKind` (0 for files 0x102-0x104), the file read into the buffer (CD `+0`, completion `0x00782fe0`), the font path, the special texture unless `TkGetLang` |
| 7 | `FlushCache(0)`, `MsBtlReadSetScene` (0xa skips the map) |
| 8 → 9 | `g_BtlMapRead`, battle map (`g_BtlBattleMap` bit 0x400) or field map, number its low ten bits; `g_BtlMapStarted` |
| 9 → 0x22 | waits for the renderer (bits 0x20 and 0x40), `loadPhyreFieldMap` |
| 0x22 → 0xa | waits for `g_BtlMapRead` |
| 0xa → 0xb | directions, first strike (`MsGetFirstAttack(0x20)`, a script's `g_BtlFirstAttackSet` over it), the two scripts in manager 0 and set up, the reverb (formation byte 1), the battle count (below 0x0fffffff unless the steal rule is 2), camera 0x18, the party shown (none: motion 0x3f), the monsters placed once the map started |
| 0xb → 0xc | each party member's model (and its battle count byte) and weapons queued |
| 0xc | waits while the cue level is set and `g_NewCharacterLoads` > 0; `g_BtlStartReady` (0 on the first visit, else 1 with field mode 0), camera 0x75, `MsBtlReadMonster` |
| 0xe → 0xf | the monster slot counters cleared, each monster (an ID and `+0x24`) model and weapon queued |
| 0xf → 0x10 → 0x11 | waits, camera 0x76; the footsteps (`MsBtlReadScene`, formation byte 2) |
| 0x13 → 0x14 | the party's second models queued (the loads count from 0 on the first visit) |
| 0x14 → 0x15 | waits (the first visit also on the loads); `g_BtlStartReady` 1 on the first visit or with field mode 1 |
| 0x15 | the first strike's message (kind 2 the party's; 3 the monsters', 100 frames on scene 0x01e60046, else 0x19) |
| 0x16 → 1 | waits for the message window, the sound transfers, the actors' loads and the hold; then `Ch_CheckNbBattleChr`, the limit cue, system check 5, camera 0x7c |

Other statuses only run the cue. The port calls the battle callees
ported before directly (cue registration, cameras, motions, visibility,
`MsAtelRegist`, `MsAtelSetUp`) and the map's, graphics', sound's, script's
and chara's functions, the CD object, `MsBtlReadSetScene` and
`MsBtlReadMonster` through the game. `MsBattleExe` keeps calling it
through the game: it tail-calls it, so its `eax` is the read's.

`MsBattleInit` (`0x00781700`, `ffx_battle::init`) runs once at the game's
start. After the inits it reads eight kernel files whole (directory 0xd,
then 0x2f for files 5 and 6): each size is stored as a halfword
(`0x0112a9d2`, the field list's, read back signed; the others,
`0x0112a972`-`0x0112a980`, unsigned) and its block is an aligned
`user_malloc` booked with `MsMemCheckRegist`, read and waited for (CD
`+8`). Then `Kernel_LoadTables` until it returns 0; menu texts 7, 9, 3
(and 0xe-0x10 while `MsGetCheckEnglish`) and the start list
(`0x00c421b4`, `i8` until a negative, `MsMenuReadStartText_00782b70`
with `load` 1); key items, field items, weapon names; the new game's
save; then an empty `MsBattleExe(-1, 0, 0, 1)`, its scene and scripts.
`g_BtlExe` comes from `g_BtlExeKeep` after the memory word's label
(`MsBattleLabelExe`), else 0. `MsMemCheckSum(0x70000)` gives the room
left below 0x70000 past the booked work memory (less 0x10, rounded down
to 64) a block of its own. With the debug menu's toggle `0x0112a905` the
debug fills run (`MsDebugFullMember` is reviewed debug;
`MsDebugFullCommand` stays logic: the PC booster menu `0x006fc970`
reaches it).

`MsMenuReadText(i, load)` (`0x0078fef0`) fills row `i` of `g_BtlMenuText`
(12 bytes: busy byte, size, a 32-bit pointer table handle): thirteen texts
are kernel files read whole (`getExcelBinSize`; 7 and 0xe are both
`config_txt`), copied in and freed; 2, 0xd, 0xf and 0x10 are CD files of
directory 0x27 (`MsGetFileSize(0x27, load ? 9 : 0x27, i)`) read
asynchronously with a request block on the caller's stack whose `+0xc` is
`i`; the completion `0x0078fed0` clears the busy byte. The port takes the
block's address from the shim (a frame on the hook's stack). It returns 0
(-1 past 0x10).

`MsSetSaveStartGame` (`0x00786b00`) builds a new game's save: the first
records of directory 0xd's files 3 (the menu's 7 words), 4 `party_data`
(0x21 words from `g_SaveGil`, the starting items at `+0x44`/`+0x54` and
the seed at `+4`) and 0x1f `menu_panel` (8 words); the weapons from
`wep_data` until the first record comes again (`MsGetExcelData`'s
fallback) or past 0x23 without `g_BtlAllWeapons`, a character's (byte 4
below 7) with its name number (`MsWeaponNameNum`) and icon; 18
`ply_save` records and names (also the default names), each name's
bytes 18 and 19 the language and 1; `MsRandMize(frames + seed)`; the
starting items with a count listed and marked found, the rest of the 0x100
entries 0xff/0.

`MsEtEffectInit` (`0x0079e670`) numbers the three status effect tables
from 0 on (`MsEtEffectNumber_0079f1c0`: entries of 4 bytes up to three
zero bytes; byte 0 0xff is skipped; past 0x18 `MsBattleError("eff_id")`).
`MsBattleSpecialInit(kind)` (`0x0079de60`) makes the special stage's work
for kinds 1-3 (the boat's roll step 0.002 from `0x00b54ca4`, map objects
0x1c-0x1e's animations disposed). `MsBattleInit` reads directory 0xd's file 0xe into `0x0112a948` and
0x2f's file 6 into `0x0112a960` (round 22: both were named
`g_KernelSummonGrow`). Their loaded sizes on both snapshots, 0x980 and
0x280, are `sum_assure.bin`'s 2420 bytes and `sum_grow.bin`'s 636 rounded
up to 64: the first is `g_KernelSumAssure` (the aeons' stat floors,
`MsSetSaveParam`), the second `g_KernelSummonGrow` (`MsGetRomSummonGrow`).

`MsBattleProcess` (`0x00790c10`, `ffx_battle::process`), the battle
module's frame, is ported (round 21); every callee goes through the game
(the battle's are hooked ports), only the state it reads and writes
itself is Rust. Each frame: `MsBtlReadManage`, `g_BtlLeaderAlive` 1 and,
with `g_GsDefDrawEnv2` 1, the debug camera sleep. Outside a battle
(status 0) a debug build's pad combination L1+R1+L2+R2+X (`0x4f`) starts
the debug field's battle (`MsBattleLabelExe`), `g_BtlExe` starts a
battle (`MsBattleExe(0, field, group, slot)`) and the event effect runs.
In a battle: the debug status print, `g_BtlLeaderAlive` (actor 0's
`+0x5d0` above 0), `g_BtlPad`, the 3D sounds (their result in
`0x0112ca01`), the command frame count, the per-frame clears, the pause,
title window and party position counters, the sound transfers; while
`MsBattleCanProgress` the order (at status 1); damage, deaths, stone
effects, hit motions, the two end fades; while it can progress and the
first fade is 0 the status step, and at status 1 the input cue; then
the window, command cue, overdrive, message cue, numbers, movement,
motion positions and `MsEffectStart`. The original keeps the first fade's
new value in `cl` across `MsBattleCanProgress`, which leaves `ecx` alone.
A soft reset ends the battle (kind 6). A debug build with
`g_GsDefDrawEnv2` 1 ends it on Start+Square (status 1,
`g_BtlEndForced` 5); at status 1 the combination restarts the battle
(with Triangle the whole module: free, init, `MsEtEffectRead`) and
returns before the pad is kept; when it was checked and not held, the
status 0x21 end check is skipped. Status 0x21 or end kind 5 or 6 end the
battle (camera, reverb, fps, windows, `MsBtlGetTerm`, the save unless
`g_BtlNoSave`, `MsBattleFree`, `AtelBattleEndCallback(encounter type,
field << 16 | group, music, kind)`, the encounter type cleared). Then
`g_BtlPadOld` and the event effect.

`MsBtlReadMonster` (`0x00783680`, `ffx_battle::read`) reads one monster
file per call (status 0xd while it runs): from `g_BtlNextMonster` on, a
slot with an ID (`+0xe` not 0xffff) and a file (directory 0x10, the size
of `MsGetChrID`'s file) shares the data block of an earlier slot with the
same ID (its `+0x48`; then `+0x11` is 0 and `+0x14` that slot + 0x14),
gets `readBtlMonsterName` and three kernel texts (the record's `u16`
string offsets 0, 2 and 6: name `+0x50`, 0x40 bytes cleared, at most 0x3f
copied; `+0x90`, 0x100/0xff; `+0x190`, 0x200/0x1ff), from
`g_KernelMonster1` for IDs below 0x65, `g_KernelMonster2` for 0x65-0xb4,
else `g_KernelMonster3`. With `g_BtlFileKind` set, IDs 0xfe, 0x11c and
0x11f set it to 8, 9 or 0xc (and take the third table); a kind above 1
then replaces the name with that character's default name and becomes
1. A shared slot is attached (`Mon_AttachData`) and the next one tried;
otherwise its file is read asynchronously into a new `mon_data` block
with a request block on the stack (`+0xc` the slot; the port's comes from
the shim) and the function returns -1 with the next slot noted. With no
slot left: `Mon_LoadAll`, status 0xe, 0.

## Round 22

`MsBtlReadSetScene` (`0x00783e20`, `ffx_battle::read`) sets the battle
file's section globals ("Formation files" above: the script, section 1,
whose first byte goes to the second script record, the formation, whose
byte 3 is `g_BtlUnderwater`, the positions) and clears the file's text
table (`g_BtlFileText`) and `g_BtlFont`; when the file brought a text
(`0x023112c4`) the table is set and, with a font (`0x023112c0`) too, the
font is kept and handed to `TOFTCTransfer`. `0x023112bc` is 1 unless the
file's name is `system_01`. With `MsBattleExe`'s mode set the status
becomes 0 and it returns 0; else `MsActionInit`, `MsSetRamChrData`,
`MsPushBtlPosAll`, `MsBtlBridgeEvent2Btl`, `AtelEventRemoveForBattle`
(the battle map `g_BtlBattleMap`, sign-extended), `Sg_SetPlayer(0)`,
`Sg_IdSetEffectTableIndex(1)`, and the status (returned) is 10 when
`TkGetNowEventJumpMapNo` is that map, else 8. The original also calls
`MsGetSaveConfigEnglish` and `FFXIsDebug` with mode 0 and drops both
results; the port does not call them.

### Rewards

`MsGetItem(chr, actor, rewards, overkill, flags)` (`0x007990d0`,
`ffx_battle::reward`; from `MsDamageCheckDeath` with the victim's `+0xf88`
record, `+0xf5f` overkill and the killing hit's `+0x616` flags) pays a dead
monster's record:

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | Gil |
| `0x02`, `0x04` | `u16` | AP; with overkill |
| `0x08`, `0x09` | `u8` | The two drop slots' chances (of 255) |
| `0x0b` | `u8` | Gear chance |
| `0x0c` | `u16` × 4 | Drops: slot 1 normal, rare; slot 2 normal, rare (`0x18` with overkill) |
| `0x14` | `u8` × 4 | Their counts (`0x20` with overkill) |
| `0x2a` | `u16` | The bribe's item |
| `0x2c` | `u8` | The bribe's factor |
| `0x2d` | `u8` | Gear slots byte |
| `0x2e`-`0x30` | `u8` | Copied to the gear's `+8`, `+0xa`, `+9` |
| `0x31` | `u8` | Gear ability count byte |
| `0x32` | `u16` × 8 × 2 × 7 | Per character a weapon's then armour's abilities (the first always given) |

The AP (×100 with debug `0x0112a912`) goes to the 18 characters through
`MsApUp`, each handed the previous result (1 first). A bribed monster
(actor `+0x701` 2) gives its bribe item: `(r % 11 + 20) · sqrt(gil paid,
+0x708) · factor / 16 / 25`, plus 0 or 1, within 1-99 (x87 at 53 bits; the
root and the sum stored as `f32`, `_ftol2_sse`). Otherwise the gil (×100
with `0x0112a913`) times the last `MsApUp` result goes to the gil won
(at most 999999999) and each drop slot rolls `brnd(10) % 255 < chance`
(any chance above 0 with `g_BtlDebugAlwaysHit`), rare when `brnd(11) & 0xff
< 0x20` or `g_BtlDebugRareSteal`; the hit's flags 2, 4, 8 (and 0x20 without
0x10) replace the item with 0x2046-0x2049. Under language 0xb item 0x2065
becomes 0x2032 (4, or 8 with overkill; the bribe: 0x2048 written back, 4).
Last the gear chance: `BattleRewards_AddGear(chr, rewards, spoils)`
(`0x00798c10`) takes the spoils' next gear record (`0x00798bd0`, 8 at
most) and gives it an owner (one of the joined of the first seven
characters, the killer three times more when it is one of them), weapon or
armour at random (`+5`), `(slots byte + 1..8 − 5) / 4` slots within 1-4
and `(ability byte + 1..8 − 5) / 8` draws from the owner's list, each
ability taken when the gear has none of its group (`g_KernelAAbility`
byte `+0x69`, `0x00798a90`); then its name and icon.

### Gear names

`MsWeaponNameNum(gear)` (`0x007a0cf0`, `ffx_battle::gear`) names a weapon
(`+5` 0) in `0x007a1c00` and armour in `0x007a0d10`; until round 22
ffx.sym had the two halves' names swapped (the first tests only weapon
abilities, the second only armour's, by the kernel's `a_ability.bin`
names). Each is a fixed list of tests over the four abilities (`+0xe`),
the slot count (`+0xb`) and, for weapons, the flags (`+3` bits 4 and 8:
names 0x5000, 0x5001); the first that holds gives the name (weapons
0x5000-0x5046, armour 0x504a-0x509e). The port keeps the list as a table
in the original's order (`weapon_name`, `armor_name`).

### Menu window lists

`MsGetSaveWindow(chr, kind, &count)` (`0x00785410`, `ffx_battle::menu`)
returns the list a battle menu window shows and its count. Kinds 1-4, 7
and 0xc-0xf are the character's own lists in `g_BtlItemMem2` (0x478 bytes
each, character 0 for an invalid `chr`), 6, 0x11, 0x14 the item windows
and the save's items, 0x15/0x16 one command (0x3121, 0x3122); 5, 8-0xb
and 0x12 build a list (`0x0112c268`-`0x0112c376`, 0xff after): the aeons
in (the first sister ends it with 0x30ff: the original sets its loop
counter to 0x30ff), every actor in, the living party slots then the other
living characters (0x12: only those able to switch, `+0xdd8`), the reserve
then the first eight characters, the living monsters. The count of kind
5 is 10 although its list is filled to 0x1f. `MsGetChrStatusBits_0078f930`
returns an actor's `+0x606` in `ax`, the actor pointer's upper half above.

### Party actors, treasures and random target lists

`MsSetRamChrParam(chr)` (`0x0079c5f0`, `ffx_battle::save_param`) sets a
party actor from its save record (`g_PlySave`, 0x94 bytes): maximum HP
and MP (`+0x24`, `+0x28`, also the bases), the eight stats (`+0x5a8`: the
base at `+0x5b0` plus the save's `+0x2f`, within 1-255, the seventh
0-255), the weapon and armour (`+0x2d`, `+0x2e`). It clears the ability
fields (`+0x5d8`-`+0x605`, `+0x62a`-`+0x63c`, `+0x641`-`+0x65b`,
`+0x6bc`-`+0x6c1`; formula 1, power 0x10), then for each piece held
(`MsGetSaveWeapon`) adds the weapon's formula and power (`+8`, `+9`), the
critical bonus (`+0xa`) and each auto-ability's record: flags `+0x11`-
`+0x15` into `+0x5d9`-`+0x5dd`, bytes `+0x16` (25) and `+0x2f` (13)
added to `+0x5de` and `+0x5f7` within 0-0xfe, `+0x5e` into `+0x604`, the
element words `+0x58`-`+0x5c` into `+0x630` (or `+0x636` with `+0x10`
bit 0, setting `+0x63c`), bytes `+0x3c` (25) to `+0x641` within 0-0xff,
the words `+0x60`-`+0x66` into `+0x65a`, `+0x6bc`, `+0x6be`, `+0x6c0`.
Then `MsLimitStatusProcess` and HP, MP within the maxima (the MP is left
in `eax`).

`msFieldItemGet(i, spoils)` (`0x00799420`) hands out treasure `i` of the
`takara` table (kind, amount, `u16` ID): kind 2 an item, 5 a piece of
equipment from the `buki_get` table (0x10-byte records: the gear's bytes
3-5 and 8-0xa, slots byte 7 but at least the abilities held, four
abilities from `+8`, 0 as 0xff), 10 a key item (`+0xd1`, `+0xec`), any
other kind gil (amount × 100 to `+0xcc`). Under language 0xb the record's
item 0x2065 is written back as 0x2060 first.

`MsGetTargetRandomList(chr, &n, shuffle)` (`0x007b7170`,
`ffx_battle::cue`) lists each of the actor's targets
(`MsGetTargetList`) once per hit it has left (`MsGetDamageInfo`) at the
actor's `+0xd80`, at most 0x40
(`MsBattleError("list_over")` past them, which hangs); with `shuffle`, n
swaps of `i = brnd(0x10) % n` and `j = (i + 1 + (n < 3 ? 0 : brnd(0x10) %
(n − 2))) % n`.

### Damage merges and party swaps

`MsMergeDamage(chr, target)` (`0x0079aff0`, `ffx_battle::cue`) folds the
results left in each of the target's buffers for `chr`'s running command
(from `+0` the next to `+1` the count) into one at the next's place,
which becomes the last: a copy of the last result whose bytes `+2`, `+4`
and words `+0x18`, `+0x1a` are the ORs of the folded ones', `+0x1c`-
`+0x28` their sums, and hit kind `+1` 2 when one is 2, 0 when one is 0
(before any 2), else 1. It returns how many it folded.

`MsExchangeSaveParty(chr, slot)` (`0x00784d00`, `ffx_battle::save`) swaps
`chr` into one of the 20 party slots (0-2 the front, 3-19 the reserve;
entries above 7 count as none): `chr`'s own place is found in the reserve,
then the front; a character in neither that may join (`ply_save` `+0x2c`
bit 0) takes the first empty reserve slot, else the first empty front one.
Neither `chr` nor the slot's character may be fixed (bit 2,
`MsGetSavePlyFix`, `0x00785360`). It returns -1 when it swapped.

### Test stubs

`battle_target`'s stub area from `Ms_Gui_BattleConsoleProc` to
`Ms_Gui_BattleConfigProc`'s end (`0x007c86e0`) is full; round 22's stubs
go to `Ms_Gui_BattleChr2Proc` (`0x007de790`, 5369 bytes, a debug window's
procedure no code calls): `GUI_STUBS` in the test.

## Round 23

### Menu texts and `MsGetSaveConfigEnglish`

`MsGetSaveConfigEnglish` (`0x007851d0`) is `xor eax, eax; ret`: three
bytes, too small for a hook's jump. Its callers' ports
(`ffx_battle::menu`: `MsMenuGetText` `0x0078fcf0`, `MsMenuGetHelp`
`0x0078fbb0`, `MsGetMenuChrInfo` `0x0078f7c0`) call the Rust constant
(`ffx_battle::getmem`) directly, and the M2 tooling counts the function
as ported, not hookable.

- `MsMenuGetText(i, id, second)` reads `g_BtlMenuText` row `i` (12
  bytes: `+0` busy, `+8` the block's ptr32 handle) before it checks `i`
  (unsigned, at most 0x10), so rows past the table are read as other
  globals. With the English config, texts 3, 7 and 9 would move to rows
  0x10, 0xe and 0xf; on the PC the config is always 0. The record is
  `MsGetExcelData(id & 0xfff, block)`; the name is its `u16` at
  `4 * (second & 1)` plus the block's strings. `MsMenuGetHelp` reads
  `+8` instead, except text 0, which reads its names.
- `MsGetMenuChrInfo(chr, k)`: IDs with bits above 12 take
  `MsGetComData`'s record's `u32` at `+8 + 4 * kana` plus the name it
  stores (0 for the kinds that store none). Otherwise actor `chr`'s
  `+0xf74` (`k` 1) or `+0xf75` (`k` 2) picks a line in `g_BtlFileText`
  (`u16` at `4 * (kana + 2 * line)` plus the table); without one, a
  character reads `ply_rom`, a monster its actor's `+0x90` or `+0x190`.

### Effect ends, gear swaps, script motions

- `MsEffectTerminate(id)` (`0x00788920`): the switch is on `(id >> 8) &
  0xff` through a byte table; the effect camera mode is cleared when
  `g_BtlEffectCameraCall` equals `id` (`sete`, `dec`, `and`: the mode
  stays for every other effect). Without actors (`MsGetChrTop` 0) the
  actor is null and the kinds that touch it read address 0.
- `SwapSaveGear(first, second)` (`0x007ab9f0`): the two slots trade
  their 0x16 bytes; the characters wearing either piece (`ply_save`
  `+0x2d` weapon, `+0x2e` armour) follow it and the slot gets them as
  owner (`+6`). A `0x7000` ID among the two takes both pieces off (0xff
  in the save and the slot) and compares whole, so it never matches a
  worn byte.
- `Battle_ScriptPlayMotion(worker, args, stack, flag)` (`0x007a8fc0`,
  `ffx_battle::motion`) returns the motion's frames `<< 8` over the
  speed (signed `idiv`; the frames themselves when the speed is 0), and
  1 when the motion does not play or has no frames; `+0x434` stays set
  then. Its script and chara callees go through the game.

### The kernel file reader

`getExcelBinSize(name, buffer)` (`0x00782df0`, `ffx_battle::init`)
returns the size it read in `eax` although Ghidra calls it void
(`MsMenuReadText` and the table readers use it): `sceLseek` to the end,
plus 15, rounded down to 16 as a signed number. The path is
`host0:/ffx/master/new_<lang>pc/battle/kernel/<name>.bin` with the
language from `TOGetFFXLang` (0 `jp`, 2 `fr`, 3 `sp`, 4 `de`, 5 `it`,
9 `kr`, 10 `ch`, anything else `us`), built by `sprintf` into a 128-byte
stack array; the port builds it in Rust in a frame of the hook's and
cuts it at 127 bytes, where the original would run over its stack.
A file that does not open logs "WJ: there is no DE message.bin" and
returns 0.

## Round 24

### Overdrive hits: `MsMenuLimitSet`

`MsMenuLimitSet(chr, user, count)` (`0x007afe00`, `ffx_battle::limit`)
returns the number of results blocks it found in `eax` (Ghidra: void).
The overdrive kind is the user's command's `+0x58`. A block whose `+4`
has bit 2 is the user's own (the drain) and gets no hits. For kind 0x25
the hits taken off are the command's `+0x2b` minus the minigame's count
(at least 1; none when negative); for 0x30 and 0x52 with count 0 the
difference between the command's `+0x2b` and that of `0x30ec` or
`0x310d`, which can be negative and then adds hits to every target.
Without bit 0x8000 of the command's `+0x1c` every target loses them;
with it they go one at a time from a random target among those with the
most left (`brnd(0x10)`, signed remainder). The ports pass the
original's stack locals (hits and misses, the random list's length, the
drain vector) in a hook frame. `0x007b0470` passes the truncated `x` as
a fourth argument that `MsMenuLimitSet` never reads.

## Round 25

### The view matrices: `MsCameraMatrix`, `MsScreenMatrix`

Both are PS2 VU0 macro code the PC port runs on the x87 through the
emulated registers (`ffx_battle::view`); every register they leave is
part of their result. `MsCameraMatrix(out, eye, target, up, roll)`
(`0x007c1be0`) returns `out[15]` in `eax` (1.0; `eye.w` when `eye` and
`target` hold the same 16 bytes, which gives a zero matrix with
`out[15]` = 1 and touches no register). Otherwise: `d` = `target - eye`
normalised (Q = `vf00.w / sqrt(|d|²)`, 0 for a zero sum; `_CIsqrt`'s root
stored as `f32`), `c` = `d × up` and `e` = `d × c`, each normalised the
same way (the outer product as `vopmula`/`vopmsub`: the accumulator
`a.yzx·b.zxy` stored first); each row's `w` becomes `w - w` (an old
infinity or NaN in `0x00c8f7fc`/`0x00c8f83c` survives as NaN); the three
are transposed by adding a `vf00` lane to every off-diagonal element
(so -0 turns +0); `r` = `eye.x·col0 + eye.y·col1 + eye.z·col2` and the
translation `vf00 - r`, `w` 1; then every row is turned by the roll in
its `xy` plane (`_CIcos`, `_CIsin`). The roll rows `vf04`/`vf05` take
their `zw` from four stack words the function never writes.

`MsScreenMatrix(out, screen)` (`0x007c3d60`) returns 0: the diagonal
`((+0x1c · +0x10) · +0x48) / +0x58 + vf00.x` and
`((+0x14 · +0x1c) · +0x4c) / +0x5c` (that quotient is Q) `+ vf00.y`, the
centre `(+0x30 + +0x20) + +0x70 - +0x50` (`xy`, through
`Vec2SubScalarsFlags_0075e870`, which raises VU0's zero and sign flags on
it), rows `(sx,0,0,0)`, `(0,sy,0,0)`, `(cx,cy,+0x28,1)`, `(0,0,+0x18,0)`;
the zero and sign flags also for `sy`.

### Shakes (`ffx_battle::shake`)

A shake record is 0x24 bytes: kind, jitter range, amplitude (`u16`),
frames left and length (`i16`), the last value (`+8`) and the spring
(`+0xc` start, `+0x10` position, `+0x14` speed, `+0x18` pull, `+0x1c`
start pull, `+0x20` steps, `+0x21` fade, `+0x22` random range). Each
camera side has three at `+0x5e0` (one per axis: `MsCameraShake`'s
`which` bits 0-2, not eye and look-at), each screen two at `+0x16c`.
`MsCameraShakeProcess(rec, angle, out)` (`0x007c50f0`): kinds 1, 2, 7
`sin(angle)`, 3, 4 `(sin(angle) + cos(angle + 0.3)) / 2`, 5, 6, 8 the
same with 0.1 (the sum at 53 bits, passed to `_CIcos` as a double), times
the amplitude, `(32 + jitter)` and 1/32, the jitter `range/2 -
brnd(9) % range`; odd kinds 1-5 fade by frames left / length, even ones
by (length - frames left) / length; 5 and 6 hold half the last value on
odd frames or when another `brnd(9)` has a zero low nibble; kind 9 steps
the spring `+0x20` times (`pos += speed`, `speed ± pull`, against the
position's sign). Below 0 frames left the shake stops and writes 0;
kinds 7-9 only with a positive length.

### Screen moves and the camera's setters

Each screen side has four lane records of 0x10 bytes at `+0xdc` + 0x40
side (`+0` kind, 2 accelerated, `+4` frame, `+6`/`+8`/`+0xa` the
accelerating, cruising and braking frames, `+0xc` the total), a position
`+0x40`, a saved one `+0x80` and the move's start `+0xa0` (16 bytes per
side). The camera's request functions find channel `id >> 8` of camera
`id & 0xff`: 0x130 bytes from `+0xf0` of the side; its `+4` is the
filter count (up to five, each 0x20 bytes from `+0x90`), `+7` the move
state, `+0x70` the divide values, `+0x170` the start position
(`MsCameraReset`). `MsCameraSetStartPos` (`0x007bff10`) pushes only five
of `MsVectorPolar2Rect`'s eight argument words; the other three (its own
saved `esi`, `ebp` and return address) reach VU register 4's `yzw`.
`MsScaleAboutPoint_007c4d40` (name ours, `MsCameraProcess`) first copies
the old `vf02` into `vf00`, and only its subtraction (skipped when the
origin is the word `r0` holds) restores `vf00`.

## Round 26

### Battle cameras on actors: `MsCameraSetBtl`, `MsCamSetBtl`

`MsCameraSetBtl(id, side, chr, mode, a, b, lift, turn, pitch, scale)`
(`0x007c5c60`) fills channel `id >> 8` of side `side & 1` (the channel
records: 0x130 bytes from the side's `+0xc0`): byte 0 polar (modes 3,
4), kind `+2` (modes 0-4: 0xb, 0xc, 0xd, 0xf, 0x10), the selectors
`+0x10`/`+0x12` (`u16`), the actor sets `+0x14`/`+0x18` (`id`'s level
nibble 5: `MsGetChrBit(chr, a | b, 1, 1)`, battle actors; else the
selectors themselves, script actors), `lift` `+0x5c`, with a mode
`turn`/`pitch`/`scale` `+0x40`/`+0x44`/`+0x60`, the other side's
position as `+0x30`, `+6` 1 and `+0x64` 0. While `id` is the current
request it calls `MsCamSetBtl(id, mode, side, ch, other side, its
channel)` (`0x007c58e0`) and `MsSetCheckChange(id, side, 0, 0)`.

`MsCamSetBtl` asks `MsGetChrInfo` kinds 9, 10, 7, 11 (camera position,
camera height, direction, camera width) of each set (0x100 for script
actors) and averages what answers: the side's position `+0x50` (`y` less
`lift` times the height), with a mode the yaw (`MsVectorRect2Polar` from
the first set to the second when both answered with different bits; for
script actors from the other side's position when they differ in `x` or
`z` and the side is on; else, for two sets or a kind-0xc other channel
naming the same actors, the channel's saved yaw `+0x64`, taken the first
time `+6` is set), the length `scale` times the width (modes 2 and 4:
`10.0 · scale +` the width) and then either a polar offset added to the
position (`MsVectorPolar2Rect`, yaw plus `turn`) or the side's polar form
`+0x60`..`+0x68`. It returns how many sets answered. Its three
`MsVectorPolar2Rect` calls push five of the eight words: the rest are
its caller's `edi`, `esi`, `ebx`, which `MsCameraSetBtl` makes the
channel, the other side and `id`; the port passes them.

### `MsCameraProcess`' leaves

- `MsCameraAimPolar_007c0910`, `...2_007c09b0`, `...3_007c0a30` (names
  ours): the side's polar form towards the move's point `+0x30` (from the
  side's base `+0x8c`), pitch and length from the move's `+0x44`/`+0x48`
  plus the distance over `+0x54`/`+0x58` (a zero divisor adds nothing; a
  NaN divides), the second keeping the point's own pitch, the third
  stopping short of the point (length = distance less the rest).
- `MsCameraAtelFocus_007c0560`, `MsCameraAtelYaw_007c0650`: the mean
  position or direction of up to two script actors (`+0xc`/`+0xe`,
  0xffff none) added to the move's offset or yaw.
- `MsCameraSplinePos_007bcd30`: the side's position and `g_Ee_GprMove`
  from the move's spline (`+8`; `TkMtCalcSpline3` under flags 3 at
  `+0x1c`) at the other side's `+0x24`.
- `MsCameraMoveAcc` (`0x007bd860`), `MsCameraMoveFrame` (`0x007bde50`):
  the current request's lanes (16 bytes: kind, curve `+4`, accelerating/
  holding/braking frames `+6`/`+8`/`+0xa`, total `+0xc`, frame `+0xe`):
  roll `+0xd78` and depth `+0xd88` of the camera for side 1, then the
  side's x/y/z `+0x580`.. where the channel's flags `+7` (bits 8, 0x10,
  1, 2, 4; re-read before each lane, as channels from 4 on lie over the
  lanes) leave them free; then `MsCameraSetStartPos` and the flags 0.
- `MsCameraAddLineFilter_007bf0b0`: a filter record (up to five, 0x20
  bytes from the channel's `+0x90`): direction, start, `-x0 dx - y0 dy`,
  `dx² + dy²`, a weight. `MsCameraSetDepthMaskMode_007bcf50`
  (`Native_CamSetDepthMaskMode`'s): channel mode `+0xcc8` + 0x30 channel
  (negative: the old one with bit 1), the camera's `+0xcac` for the
  current request.
- The special stage: `MsBattleSpecialCameraKind_0079dd70`,
  `MsTransSpPos` (`0x0079e1b0`: kind 1 turns points by the tilting
  stage's angles `+0x800`/`+0x808` after lowering them by `+0x844`, the
  sea's kinds 2 and 3 by its matrix `+0x7c0` without translation) and
  `MsSpCameraProcess` (`0x0079dfe0`, eye, look-at and up vector while
  `0x0112c9fc`).

### Numbers, sounds, commands

- `MsNumberDrawProcess` (`0x0079f660`): the actors' pulsing counters
  (`+0x5c8` digits; phase `+0xf6c` += `+0x5c9 / (+0x5c8 + 1) / 8`, colour
  from `sin(phase) + 1`) and the damage number list through
  `TOBtlSetDamageValue`; frame table `0x00c424b0` (x, y, alpha by age
  less place); the last 5 frames fade. The original's colour and alpha
  words carry stack garbage above their low bytes, which
  `TOBtlSetDamageValue` drops.
- Actor sounds by kind (`MsChrSeGet_0079fe80`): tables of `u16` per kind
  (`0x00c42550` party, `0x00c425e0` monsters, `0x00c42598` the rest):
  base plus a variant (bits 8/9 cycle `+0x410` mod 4/2, bit 10 the
  underwater flag's bit 0, bit 11 8 under water); the top nibble picks
  the scene's transferred sounds, the number itself or the model owner's
  `+0xeac` sounds. The players over it: fade, play (once a frame,
  `0x0112c8e0`), play once, play unless hasted, play when shown with HP.
- `MsChrDelExtraCommand_007acb40`: a command out of the picked actors'
  extra commands (`+0x71a`, count `+0x6cb`) and their second menu list.
  `MsMonShuffleVariants_007842f0` (`Mon_LoadAll`'s): monsters sharing a
  model draw distinct `+0x424` variants.

The hook's script position callback (`Host::atel_pos`) now passes the
game a stack buffer: bridges hand it frame locals at addresses the game
never maps.

## Round 27

### `MsCameraProcess` (`0x007be110`)

Once a frame, for each of the three screens that is on (`+0xc0`), its
camera (`+0xc1`) runs once (`+0xc89`, set for all three cameras first).
Each side (`cam + 0x30`, 0x61c bytes) uses the channel of the camera's
current slot (`MsCameraGetNum`: `off = 0xc0 + 0x130 n`). Three parts:

1. Requests. A channel with a request kind (`+2`) clears the side's
   `+2`, sets its near flags `+4`/`+5` (kind 7 clears them) and starts
   the move: 1 the position from the channel's `+0x30`, 2 and 7 the
   polar form from `+0x40`, 3 `MsCameraAtelFocus`, 4-6 the aims, 8
   `MsCameraFocusWeights`, 9 the same after the polar form towards the
   channel's point (its yaw plus `+0x50`), 10 `MsCameraAtelYaw`, 11-13,
   15, 16 `MsCamSetBtl` modes 0-4, 14 `MsCameraSplineMove_007bcb80`, 17
   `MsCameraSplinePos_007bcd30`. Then, unless the channel is the camera's
   last one (`+0xc8c`), `MsCameraNearPoints_007c0d40`, and always
   `MsCameraMoveStep_007bcab0(side, other, channel, (i8) channel[0])`,
   whose result is the channel's `+1`.
2. Positions. With the camera matrices on (`0x023114c4`) the sides take
   the matrix look-at (side 0) and eye (side 1). Else a side whose
   channel has a request moves: polar sides (`+1` 1) step yaw, pitch and
   length (`MsCameraMoveCalc`, near flags `+4`/`+5` for the first two;
   the polar form from the position first when the channel's `+1` is 0),
   the others x, y, z; the position from the polar form when the channel
   is polar (or, if a lane moved, the side). A trailing side (`+3`) keeps
   within `+8` of its start (`+0x20`): the lag `+0x40` grows by
   `MsVectorNormal2(position, start, +0xc)`, is clamped to `+8`, and the
   position is start plus lag. On the camera's last channel the position
   eases from the start over the channel's `+0x70`..`+0x78` frames
   (non-zero, NaN too; `+2` set when it moved). The position becomes the
   start, `g_Ee_GprMove` and the channel's `+0x80`; three shakes
   (`+0x5b0`, angles `+0xa0` stepped by `+0xb0`) give `+0x90`; `+0x10` is
   start plus shake, scaled about the camera's `+0x10` by `+0x20` when
   `+0xc8a` is set.
3. Roll and depth. With the special stage's camera on,
   `MsSpCameraProcess(cam + 0x40, cam + 0x65c, cam)` first. The channel's
   roll and depth targets (`cam + 0xcb8`, `+0xcc0` + 0x30 n) and three
   words step (`MsCameraMoveCalc`, the roll near). On the last channel
   the roll eases over side 1's channel `+0x7c` frames the short way
   round: `d = roll − last`, `m = d − 2π·trunc(d / 2π)` (stored `f32`,
   the count through `_ftol2_sse`), `+2π` if negative, `last − 2π` as
   the base when `m ≥ π`. The override's depth or, with the camera
   matrices on, their roll word replaces the depth; both are kept
   (`+0xc90`, `+0xc9c`, the channel's `+0xcd4`, `+0xcd8`), the channel
   becomes the last one and the screen's `+0x1c` takes the depth.

Its by-value vector arguments (`MsVectorPolar2Rect`'s radius,
`MsVectorNormal2`'s scale, `MsMulVector`) take their spare words from
its caller's `edi`, `esi`, `ebx`; the move step's radius takes its own
`esi`, `ebx` (`-off`) and `ebp`, the near points' its `ebx`, `edi`,
`esi`. The port gets those when known (`ProcessEnv`), else leaves the VU
words as they were (the hook).

The steps it calls (names ours):

- `MsCameraSplineMove_007bcb80(id, side_no, _, side, mv)`: the move's
  spline position `mv + 0x68` clamped to 0..1, the spline there and with
  `mv + 0x1c` bit 2 its tangent added (speed `+0x6c` ≥ 0) or subtracted
  (`0x0088aba0` is `TkMtCalcSpline3`'s tangent); the roll is the point's
  w; clamped or at speed 0 `MsCameraSetRect`.
- `MsCameraMoveStep_007bcab0(side, other, chan, step)`: `MsSetCheckChange`'s
  move step (`CAMERA_MOVE_STEP`): step 1 rebuilds the position from the
  polar form; position to `other + 0x30`; a pending near-point search
  (`+0x1d`) starts (`spline_get_near_point_init`); channel limits
  (`0x007c0120`, run as the original) can stop the step; a speed at
  `other + 0x6c` other than 0 goes to the side's y and returns 0.
- `MsCameraNearPoints_007c0d40(_, side, chan)`: for each filter of kind
  4 or 5, the near-point search from the side's position and
  `spline_get_near_point`'s result at `chan + 0x24`; returns their count.

### `MsScreenProcess` (`0x007bc110`)

The screen's frame and the view (see the symbol for the full list).
Returns 1 for a screen number of 3 or more (signed), -1 for a screen
off, else 0. Each side's position is the saved one, its centre half the
size less the display's (`0x00c44240`), links 1-3 (`+0xc2`) as in
`MsScreenLinkSet`; four lanes move it; two shakes; then everything is
truncated to whole numbers. The view comes from the override camera, or
(camera matrices off) the camera: the near plane for magic 0x21c on map
0xd0, `MsCameraMatrix` of the camera's eye and look-at (three special
levels check both squared lengths lie in 0.01..1e9), the screen offset
`((+0x70 + +0x30 − +0x50 + ox) · 2 / 512, (+0x74 − +0x34 − +0x54 + oy) ·
2 / 416)` and `Cam_Apply`. A camera with focus words other than the
screen's hands them to the scene graph.

The hook keeps `MsScreenProcess`'s entry in the hook table: the game
calls `gfx::live::cam_update`, which places the renderer's free camera
in the override and then runs the Rust port (`ffx_battle::screen`)
instead of the original; `Cam_Apply` and the renderer's functions run
through the game.

### Smaller ones

- `MsSetCountryName(out)` (`0x00782c00`): the language's country code
  (`TkGetLang`: 0 jp, 1 us, 2 fr, 3 es, 4 de, 5 it, 0x12 uk) written as
  `sprintf` of the constant does (three bytes, none for a null `out`);
  returns 1 for the European ones. Other languages write nothing, 0.
- `MsDetachWeapon(i, k)` (`0x00793f10`): returns 1 or 0 (Ghidra shows
  void). The special weapon takes the actor's `+0x49c`; a weapon with a
  character while the actor holds weapons (`+0x4b5`) is unbound and left
  at the hand's optional position (weapon `+0xd`), turned as the actor,
  `+0x4b6[k]` set for the re-attach `MsAttachWeapon` performs.
- `MsGetChrMotionInfo_007b2c90(i, k)`: `MsSetChrMotionInfo`'s floats read
  back (an `f32` in ST0, 9 the width, other kinds 0.0); kinds 0 and 1 are
  10.0 more while `MsGetDamageResult` for the first of the actor's target
  bits (`+0xdc0`, selector -16) is 1.
- `MsChrAddExtraCommand_007ac840(chr, command, selector, slot)`: the
  counterpart of round 26's delete: up to four extra commands (`+0x71a`,
  slots `+0x722`, count `+0x6cb`) and the command inserted at the slot
  (0..7) of the second menu list in `g_BtlItemMem2`.
- `MsGetDamageInfoNext`: as `MsGetDamageInfo`, only each buffer's next
  result.
- `FUN_007b055a` is not a function: `MsSetRikkuLimit` (`0x007b0520`)
  jumps into it and it runs on that frame; it was ported with
  `MsSetRikkuLimit` (verdict in `tools/classify.py`).

## Round 31 notes

- **Fast target tests.** `tests/battle_target.rs`'s whole check runs ~40
  sections 400 times (about 300 s per run, ~890 s per mutant over the image
  and both snapshots). Each section also has a `part_<section>` test (the
  `part!` list) that runs it alone, after the sections it reads state from,
  so `MUT_FILTER=part_<section>` selects it: about 1.2 s per mutant for
  `part_target_dist`. `effect_start` has no part test: it needs sections
  later in the whole loop.
- **`MsCameraSetDirVector_007bce80`** (`0x007bce80`) hands
  `MsVectorNormal2` its scale alone. The scale's other three words come
  from its own stack: its caller's `esi` and `ebp`, then its return
  address. They stay in VU register 2. Called from
  `MsAtelCameraDirVector_007bb210`, those are that function's `stack`
  argument, its frame pointer and `0x007bb29e`: fixed for each caller
  except the frame pointer. The port takes them as explicit `junk` words.
  The tests seed them (the harness's `0x22222222`, `0x44444444` and
  sentinel, or the caller's).
- **`MsAtelCameraSetBtlPc_007badb0`** (`0x007badb0`, natives 0x6040,
  0x604c, 0x604d) is `MsAtelCameraSetBtl` with yaw, pitch, a second
  distance and the mode, plus PC fixes keyed by the battle scene and the
  magic ID (the function's doc lists them). Three of them (scenes 0xd40000,
  0x1a30004 and 0x1f60004) test `100 * value` against `f32` products,
  88.99999237, 228.99996948 and 3.99999905. At the game's 53-bit x87
  precision no `f32` times 100 equals any of them (at 24-bit precision
  0.88999993, 2.2899997 and 0.03999999 do), so those fixes never apply on
  PC.
- **`op_et_eff` wrappers** (`ffx_battle::et_effect`): every effect ID
  comes from `MsGetEffID(counter, kind, i)` with a per-actor `u16` counter
  (`+0xe1e` the general one, `+0xe22` kind 1, `+0xe24` the casting effect,
  `+0xe28` the weapon attributes). `0x0112a917` (debug menu toggle 0xcc)
  makes `MsSetChrEffect_0079eb10`'s global effects the battle's own.

## M2 remaining

Battle-module logic functions (`symbols/classes.tsv`, the module as `tools/progress.py` places it) not yet replaced by a port in the hook table, regenerated each round (`~/ffx-work/agent-battle/m2.py`). *Blockers* are the function's own logic callees still unported; a function with none can be ported now. The module placement of unnamed functions is a guess by neighbourhood, so a few non-battle functions (start-up screens, the password and screen-aspect helpers) are counted here. `MsGetSaveConfigEnglish` (`0x007851d0`, 3 bytes) counts as ported though it is not hookable: its callers' ports call the Rust bridge.

**0 functions, 0 bytes.** M2's battle table is empty since round 36 (`MsEffectProcess`).

The battle loop's top level (ported in round 36: the effects' frame `MsEffectProcess`, after vfx's `op_effect_run_after` and `opp_main`; in round 35: the character registration `MsBtlChrRegist` and the read queue steps `MsBtlReadPlayerModelProcess`, `MsBtlReadMonsterProcess` (jump table 0x00783ae4), `MsBtlReadPlayerProcess`, `MsBtlWeaponRegist` (after chara's `Ch_Allocate`, `ChN_DataDispose`, `Ch_Dispose`, ported here), the camera limits `MsCameraLimits_007c0120` (after main's `spline_get_near_point`); in round 34: the actors' set-up `MsSetRamChr` (party and monster actors), `MsSetRamMonData`, `MsSetRamChrData`, `Mon_LoadAll`, `MsSetChrVisibleBtl2Event`, the character set-up `MsSetChrEngine_00796f70`, the event-to-battle hand-over `MsBtlBridgeEvent2Btl`; in round 33: Battle_QueueCommand, setCameraMatrixToPhyreScene, MsAtelChrInit; the default names setDefaultPlayerName, resetDefaultPlayerNameStatus; the camera and character leaves Cam_SetScreenVector2_007bc750, MsCameraSetFocusList_007bf520, MsAtelCameraPolar_007bb5d0, MsBindChrToChr_00795c90, PasswordTextRead; Wakka's reels MsSetReelResult, MsGetReelResultKind_007b1710, MsGetReelTargets_007b1910; the cursors and overdrive bars getSparedLmtPos, MsGetChrLimitBarFlag, MsCalcCursorPos, MsBattleCursorCalc; the reads MsBattleReadDataNB (jump table 0x007821c0), MsBattleReadData, Mon_AttachData, MsBtlReadMonsterDone_00783990, MsAttachWeaponData_00784000, MsCDreadWeapon, MsBtlReadWeapon, MsCDreadWeaponDone_00783fc0; in round 32: the effect signal dispatcher and its damage step (MsEffectSignal_007b74d0, MsEffectSignalDamage_007b7680), the effect motions MsEffectChrMotionIn, MsEffectMagicMotion, the sound transfers MsRegistSEdata, MsSEPtransRegist, MsSetVoice, MsSoundStreamPlay, the splines MsMoveSpline, MsSplineRegist, the script target helpers MsSearchScriptTarget_007a8f60, MsSetCommandScriptTarget_007a8b40, the target lists MsGetTargetSortList, MsGetTargetLinkList, MsGetChrLinkSet_007b7440, MesSetChrTargetPosToLimitSparePosy, Mon_GetName, the reads MsEtEffectRead, MsRegMotionRead; in round 31: MsSetMove, MsGetTargetDist, the op_et_eff wrappers (MsSetChrEffect_0079eb10, MsSetChrEffect8, MsSetOwnerEffect8/7, MsEtCriticalEffect, MsEffectSetGuard, MsSetWeaponEffectAttribute, MsMagicEffectStart), the camera direction native and setter (MsAtelCameraDirVector_007bb210, MsCameraSetDirVector_007bce80), the PC battle camera native MsAtelCameraSetBtlPc_007badb0; in round 30: MsSetBtlPosReset, the encounter and walk-ability wrappers (MsBattleInitAtEvent, Enc_ResetAll, Enc_ResetSceneWeights, MsWalkAbilityProcess), MsDebugFullSummon, addMonDieSe, MsSetChrPos2D, MsEffectChrMotionOut, MsResetBtlTarget, MsSetBtlTarget, MsRestartMagicCamera, MsRegSEplay3, MsPauseChrEffectAll, MsEffectSignalAvoid, MsAtelBindCamera, a MsEffectSetMotion jump, the effect signals 1-8 helpers, MsSetBallSplineInit, MsEffectChrGetLoveLoveMax, MsSetChrExtraSlot, Enc_SetGroupWeight, MsResetSceneObjects, MsLineSideDist, MsGetCommandTarget2, MsGetSameTarget, MsActReCommand, MsChangeWeaponWakka, MsSetSaveCommandMenuPanel; in round 29: small leaves (the effect camera buffers, the screen camera byte, the reel entries, the VU0-style vector helpers, the language checks), two copies of vfx ports, the actor and save accessors and setters around MsGetChr (elements, Libra fields, motion bytes, the fly byte, the head position, gravity, rotation, lift height) and their wrappers (MsGetRomBtlText, MsSetChrVisibleMain, MsFieldItemGet(Mes), MsSetSavePartyMember, PasswordGetTotalIndex), the HP/MP adders, the equipment flag scans and free slot, MsExchangeSaveWeapon, MsEffectCameraGet, readBtlMonsterName, MsEffectReadStatInit, MsGetMapCenter, MsEffectSetSpeed, MsEffectCheckMotion2, MsChangeWeaponInvisible, MsGetDamageLimit, MsResetAvoidFlag, MsPauseChrEffect, MsEventEffectReadComplete, PasswordTextDispose, MsBattleDraw, the battle scripts' camera natives (move, shake, line filter, divide, eye area, focus, yaw, spline, battle camera, circle area) and their camera, the channel flag and circle area setters, the blur check, MsGetChrID, MsSet/GetCommandUse, the CTB list MsGetCTBlist, MsCheckLeftWindow, MsSetSavePlyJoin, Enc_ResetWeights, MsGetSaveCommandMenuPanel, MsSetChrShadow(Mode), MsSetChrTransparentVisible, Equip_RebuildWearers, MsBindCamera, MsSetChrStatInfo (7 KB, every property), MsSetDamageMotion, MsSaveAddAP, MsAtelRequestSystem, MsEffectSetMotion, MsGetChrMoveFrameRest; in round 28: `MsDebugFullCommand`, the camera request setters, `MsCameraSetSpline`(2), `MsAtelGetCameraID`, `MsGetComInfo`, `MsGetBasicDivide`, `MsGetDirDivide`, the results screen's weapon plate; `MsCameraProcess`, `MsScreenProcess` (run by the `gfx::live` wrapper) and `MsCameraProcess`' last steps in round 27, `MsCamSetBtl`, `MsCameraSetBtl`, `MsCameraMoveAcc`, `MsCameraMoveFrame`, `MsTransSpPos`, `MsSpCameraProcess`, `MsNumberDrawProcess` and `MsCameraProcess` leaves in round 26, the camera's `MsCameraMatrix`, `MsScreenMatrix`, shakes, screen moves and setters in round 25, `MsMenuLimitSet`, `MsMenuLimitSet_007b0470` in round 24, `MsMenuGetText`, `MsMenuGetHelp`, `MsGetMenuChrInfo`, `MsEffectTerminate`, `SwapSaveGear`, `Battle_ScriptPlayMotion`, `getExcelBinSize` in round 23, `MsBtlReadSetScene`, `MsGetItem`, `BattleRewards_AddGear` in round 22, `MsBattleProcess`, `MsBattleInit`, `MsBtlReadManage`, `MsBtlReadMonster` in round 21, `MsBattleExe` in round 20) is ported.

## M2 status (round 36)

M2 is closed: every battle-module logic function is replaced by a port in
the hook table (`~/ffx-work/agent-battle/m2.py`: 0 functions, 0 bytes).
The last one, **`MsEffectProcess`** (`0x00787e00`, 285 B), runs the
effects' frame: phase 0 calls the overlay tables' `+0xc`
(`op_effect_run_before` for the battle overlay's,
`g_BtlOverlayFuncs` `0x00c48d88`), phase 1 their `+0x10`
(`op_effect_run_after`, under VFX type 2, then type 4); after the battle
overlay's table come the tables of the actors whose `+0xdfb` is 4 or 5
(`+0xf8c`) and the event effect's (`0x01133360`, state `0x01133364` 4
or 5), each after `FlushCache(0)` and with `g_VfxPass` set while a table
that is the magic file's (`MagicFile_GetEffectOverlayTable(0)`) runs.
Another phase calls no function but still flushes and sets the pass.

## Round 36 notes

- **`Ch_BlkGetHandle`** (`0x00825a40`) and **`Ch_SetShadeID`**
  (`0x0082b720`) are ported (`ffx_chara`): viewer keys `0xfnnn` read
  `/ffx/proj2/chr/prot/<dir>/<name>/mdl/<name>.chr` (or `.mdl`) and
  `tex/<name>.tex` from the PC (`g_ChrViewerModels`, `0x012fbca8`); other
  keys come from the render cache, tagged from the debug lists
  (`g_ChrDebugModels`, `0x012fbc64`) under `g_FFXIsDebug`.
- **Survivors of round 35.** All killed by new test data but these
  equivalents: the near point's `b >= a` (the `b == a` case returned
  before); `MsBtlChrRegist`'s 13/14 order (`h * 13 / 14` and
  `h * (13 / 14)` round to the same `f32`: 13/14 is 2^-54.7 off in
  53 bits, and an exact `13 h / 14` is at least 2^-28.7 away from any
  `f32` midpoint it is not on); its unused `Ch_GetPos` (a frame local);
  `+0x43d` bit 0 already clears `appear` (`& 5`, `& 9`); the hide setters'
  `on` (any non-zero).
- **Tests.** The oracle now has `VirtualAlloc`/`VirtualFree`/
  `VirtualQuery` stand-ins (`docs/hooking.md`, "Virtual memory in the
  oracle"): `Ch_Allocate`'s field rounds no longer skip.
  `Oracle::map_zeroed` lets a test give `Ch_Allocate` a null page for its
  no-free-slot path. `dbgPrintf` (`0x0062f500`) is a bare `ret`.

## M2 status (round 35)

M2's battle table is down to 1 function, 285 bytes:

- **`MsEffectProcess`** (`0x00787e00`, 285 B) waits on vfx's
  `op_effect_run_after` (`0x00800670`, 121 B), which waits on `opp_main`
  (`0x0080bd50`). The effects agent owns both.

Ported this round: the character registration chain (`MsBtlChrRegist`,
`MsBtlReadPlayerModelProcess`, `MsBtlReadMonsterProcess`,
`MsBtlReadPlayerProcess`, `MsBtlWeaponRegist`, in `ffx_battle::regist`),
after chara's `Ch_Allocate`, `ChN_DataDispose` and `Ch_Dispose`
(`ffx_chara::handle`); and `MsCameraLimits_007c0120` (`FUN_007c0120`,
`ffx_battle::camera`), after main's `spline_get_near_point`
(`ffx_sg::spline`). The spline evaluators under it are menu's ports
(menu round 32).

## Round 35 notes

- **`MsCameraLimits_007c0120`** applies a camera channel's limits
  (`chan + 4` of them, 0x20 bytes each from `chan + 0x90`, jump table
  `0x007c0490`): kinds 0 and 2 keep the x-z position on one side of a line
  (2 pulls it linearly by `+0xc`, 0 by `atan(d k) / k` along the
  offset), kinds 1 and 3 keep the height in `+0x18`..`+0x1c` (3 linearly,
  1 soft), kinds 4 and 5 pull it towards the spline's near point (5 the
  cubic spline; its parameter goes to `chan + 0x24`). EAX counts the
  moves; `MsCameraMoveStep_007bcab0` stops the step when it is non-zero.
- **`spline_get_near_point`** keeps its parameter in `g_SplineNearT`
  (`0x01a85f30`) between calls and steps it by `f32(10 / samples) /
  f32(i * i + 1)` (`g_SplineNearSamples`, `0x01a85f34`), at most 0x28
  times. A step that leaves 0..1 returns `t` moved back, not the stored
  one.
- **`Ch_Allocate`** sets flag 2 for the ids in `g_ChrFlag2Ids`
  (`0x00c494d8`), the two floats at `+0x814` from `g_ChrIdFloats`
  (`0x00c495b0`, last match), FLAGS2 `0x80` for ids `0x1088`, `0x1089`,
  `0x108c` and `0x200` for `0x10a6`, `0x10b2`-`0x10b4`, `0x111e`,
  `0x1121`-`0x1123`, `0x3004`, `0x3009`-`0x300b` (jump table
  `0x008255c0`). With no free slot it writes through a null character, as
  the original does. `Ch_BlkGetHandle` (PC-file path) and `Ch_SetShadeID`
  stay unported and go through the game.
- **`MsBtlWeaponRegist`** passes the *new* model to `ChN_DataDispose`
  after disposing the old character, so a changed weapon model disposes
  its own block and the next `Ch_Allocate` reloads it from file.
- **Tests.** `battle_regist.rs` runs each round on a fresh copy of the
  battle snapshot (a new character takes heap the snapshot has little of);
  the field snapshot has no battle actors and skips. `Sg_CacheDisposeChr`
  and `user_free` are stand-ins there: both leave caller-stack bytes in
  globals. The chara tests skip `Ch_Allocate` rounds whose `SG_InitBones`
  needs `VirtualAlloc` (field snapshot: about half).

## Round 34 notes

- **`MsSetRamChr`** (`0x0079b4f0`) clears `+0x540` (0x1ec bytes) and
  sets up a party actor from its `ply_rom` and save records, or a monster
  from its record (`+0xf80`) and data (`+0x48`). With null data a monster
  keeps the cleared fields and gets only its command list in
  `g_BtlItemMem2`: 0x3001 first, then the record's non-zero commands, then
  0xff up to 0x14. Two debug env toggles give new actors HP 1:
  `g_BtlDebugPartyHp1` (`0x0112a91d`) and `g_BtlDebugMonsterHp1`
  (`0x0112a91e`).
- **`g_BtlVoiceEvent`** (`0x0112a9c0`) points to the formation section
  (`MsBtlReadSetScene` section 2). Its first byte is the voice flag, and
  `+0xc` holds the eight monster IDs that `MsSetRamChrData` reads.
- **`MsSetChrVisibleBtl2Event`** returns the `same` word of its
  `MsSetMotion` calls. That word is a frame local the original never
  clears: a request that doesn't run leaves the previous one's value. The
  port keeps one zeroed local across the loop. Its 1 (a request ran for
  `actor` itself) is not reached by the tests; neither is `MsSetMotion`'s
  own `same = 1` in `battle_atel`.
- **`Mon_LoadAll`** writes `m%03d` into a frame buffer that `MsAtelRegist`
  ignores.
- **Tests.** battle_target has two new sections: `revive`, which only its
  own part test runs because it sets every actor up again, and
  `bridge_to_battle`, with table stubs for the element getters.
  `logging_stub` now takes 5 arguments in a 0x80-byte slot.
