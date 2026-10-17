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

## M2 remaining

Battle-module logic functions (`symbols/classes.tsv`, the module as `tools/progress.py` places it) not yet replaced by a port in the hook table, regenerated each round (`~/ffx-work/agent-battle/m2.py`). *Blockers* are the function's own logic callees still unported; a function with none can be ported now. The module placement of unnamed functions is a guess by neighbourhood, so a few non-battle functions (start-up screens, the password and screen-aspect helpers) are counted here.

**553 functions, 131318 bytes** (364 named). By blockers: 0: 275, 1: 151, 2: 69, 3: 19, 4: 11, 5: 8, 6: 3, 8: 3, 9: 2, 10: 2, 14: 2, 15: 3, 16: 2, 18: 2, 23: 1.

The battle loop's top level (`MsBattleProcess` runs each frame; `MsBattleInit` and `MsBattleExe` set a battle up; encounters and rewards):

| Function | Address | Bytes | Blockers |
|---|---|---|---|
| `MsBattleProcess` | `0x00790c10` | 913 | 23 |
| `MsBattleProcess_Status_007917d0` | `0x007917d0` | 1894 | 15 |
| `MsBattleProcess_InputCue_00792a90` | `0x00792a90` | 718 | 8 |
| `MsBattleProcess_Order_00790fb0` | `0x00790fb0` | 446 | 2 |
| `MsBattleProcess_ChrDamage_007911e0` | `0x007911e0` | 1004 | 16 |
| `MsBattleInit` | `0x00781700` | 1353 | 18 |
| `MsBattleExe` | `0x00781020` | 1288 | 15 |
| `MsBattleFree` | `0x00781590` | 159 | 14 |
| `MsBattleEncountExe` | `0x00780d10` | 705 | 0 |
| `MsBtlReadManage` | `0x00783020` | 1529 | 15 |
| `MsEffectStart` | `0x00788480` | 1081 | 10 |
| `MsMessageCueProcess` | `0x0079cdf0` | 447 | 9 |
| `MsGetItem` | `0x007990d0` | 728 | 1 |
| `BattleRewards_AddGear` | `0x00798c10` | 630 | 3 |

<details><summary>Every remaining function (blockers, then size)</summary>

| Blockers | Bytes | Address | Name | Unported callees |
|---|---|---|---|---|
| 0 | 1 | `0x00780cf0` | `MsBattleDisposeAtEvent` |  |
| 0 | 1 | `0x007846e0` | `MsCheatPlayerPowerfulOff` |  |
| 0 | 1 | `0x00792e70` | `MsSetMouseCamera` |  |
| 0 | 1 | `0x0092cf00` | `MsDebugDrawBoard` |  |
| 0 | 3 | `0x007851d0` | `MsGetSaveConfigEnglish` |  |
| 0 | 6 | `0x007c6db0` | `FUN_007c6db0` |  |
| 0 | 11 | `0x00787390` | `isNeedShowJapanLogo` |  |
| 0 | 12 | `0x007882b0` | `MsEffectSepClearAll` |  |
| 0 | 12 | `0x007bd500` | `FUN_007bd500` |  |
| 0 | 13 | `0x007901f0` | `MsMenuSetWindowSystem` |  |
| 0 | 14 | `0x0079e7b0` | `MsEtEffectStop` |  |
| 0 | 14 | `0x007c0e60` | `setScreenDataAspectZ` |  |
| 0 | 15 | `0x00780d00` | `MsBattleDraw` |  |
| 0 | 15 | `0x00780fe0` | `MsBattleEncountFlag` |  |
| 0 | 15 | `0x007881b0` | `MsEffectReadError` |  |
| 0 | 15 | `0x00788b20` | `MsEffectWaitFlag` |  |
| 0 | 21 | `0x00796660` | `MsSetChrVisible` |  |
| 0 | 21 | `0x007966a0` | `MsSetChrVisibleAppear` |  |
| 0 | 21 | `0x00796860` | `MsSetChrVisibleMain` |  |
| 0 | 21 | `0x0079f210` | `MsBattleError` |  |
| 0 | 23 | `0x00784ae0` | `FUN_00784ae0` |  |
| 0 | 23 | `0x00787370` | `isNeedRenamePlayer` |  |
| 0 | 23 | `0x0078fae0` | `FUN_0078fae0` |  |
| 0 | 23 | `0x0079af40` | `MsGetRamChrStr` |  |
| 0 | 23 | `0x007b7f30` | `FUN_007b7f30` |  |
| 0 | 24 | `0x0078f910` | `FUN_0078f910` |  |
| 0 | 24 | `0x0078fe80` | `MsMenuInvElement` |  |
| 0 | 24 | `0x00790240` | `FUN_00790240` |  |
| 0 | 24 | `0x007b19d0` | `FUN_007b19d0` |  |
| 0 | 24 | `0x007b7ed0` | `FUN_007b7ed0` |  |
| 0 | 25 | `0x00784e40` | `MsGetBattleMessageFlag` |  |
| 0 | 25 | `0x0078f930` | `FUN_0078f930` |  |
| 0 | 25 | `0x0078fac0` | `FUN_0078fac0` |  |
| 0 | 25 | `0x00795640` | `MsGetChrMotionLevel` |  |
| 0 | 25 | `0x007aaff0` | `FUN_007aaff0` |  |
| 0 | 25 | `0x00874200` | `MsSetSaveCommandWithPrefix` |  |
| 0 | 26 | `0x00793290` | `FUN_00793290` |  |
| 0 | 26 | `0x007a86e0` | `Clamp0To6_007a86e0` |  |
| 0 | 27 | `0x0078fed0` | `FUN_0078fed0` |  |
| 0 | 27 | `0x007b75a0` | `FUN_007b75a0` |  |
| 0 | 28 | `0x0079adb0` | `MsGetRamChrFemale` |  |
| 0 | 28 | `0x007b0500` | `FUN_007b0500` |  |
| 0 | 28 | `0x007bc870` | `FUN_007bc870` |  |
| 0 | 29 | `0x00782750` | `FUN_00782750` |  |
| 0 | 29 | `0x00787a40` | `MsEffectEnvFree` |  |
| 0 | 29 | `0x00799ec0` | `FUN_00799ec0` |  |
| 0 | 29 | `0x0079dd70` | `FUN_0079dd70` |  |
| 0 | 29 | `0x0079f990` | `MsNumberFreeMem` |  |
| 0 | 29 | `0x007aaac0` | `FUN_007aaac0` |  |
| 0 | 30 | `0x007902d0` | `PasswordGetTotalIndex` |  |
| 0 | 32 | `0x007a0b90` | `FUN_007a0b90` |  |
| 0 | 32 | `0x007bbdd0` | `MsScreenLinkSet` |  |
| 0 | 33 | `0x007987f0` | `FUN_007987f0` |  |
| 0 | 33 | `0x007c1110` | `FUN_007c1110` |  |
| 0 | 34 | `0x007a2750` | `MsStoneMove` |  |
| 0 | 34 | `0x007b7ef0` | `FUN_007b7ef0` |  |
| 0 | 35 | `0x007af2f0` | `MsCheckDefenseMotion` |  |
| 0 | 36 | `0x007b7e80` | `MsEffectChrMotionSync` |  |
| 0 | 37 | `0x00782940` | `FUN_00782940` |  |
| 0 | 39 | `0x00782670` | `FUN_00782670` |  |
| 0 | 39 | `0x007b2ad0` | `MsDeathYoujinbou` |  |
| 0 | 39 | `0x007bbda0` | `MsScreenFocusResetAll` |  |
| 0 | 40 | `0x00786aa0` | `FUN_00786aa0` |  |
| 0 | 40 | `0x007902a0` | `FUN_007902a0` |  |
| 0 | 40 | `0x007ab700` | `FUN_007ab700` |  |
| 0 | 40 | `0x007b1a70` | `MsRestartMagicCamera` |  |
| 0 | 41 | `0x00793500` | `MsBtlChrFreeMem` |  |
| 0 | 41 | `0x007a0c20` | `MsGearHasAbility_007a0c20` |  |
| 0 | 41 | `0x007a8c20` | `FUN_007a8c20` |  |
| 0 | 42 | `0x007859d0` | `MsSaveAddHP` |  |
| 0 | 42 | `0x00785ae0` | `FUN_00785ae0` |  |
| 0 | 42 | `0x007a8c50` | `FUN_007a8c50` |  |
| 0 | 42 | `0x007ad6e0` | `FUN_007ad6e0` |  |
| 0 | 43 | `0x00798a90` | `FUN_00798a90` |  |
| 0 | 43 | `0x007ad5b0` | `FUN_007ad5b0` |  |
| 0 | 43 | `0x007bc0e0` | `MsScreenOffsetID` |  |
| 0 | 44 | `0x007abb70` | `MsGetSaveChrArmorModel` |  |
| 0 | 44 | `0x007abba0` | `MsGetSaveChrWeaponModel` |  |
| 0 | 44 | `0x007bcf20` | `FUN_007bcf20` |  |
| 0 | 45 | `0x007c11c0` | `FUN_007c11c0` |  |
| 0 | 46 | `0x00781650` | `FUN_00781650` |  |
| 0 | 47 | `0x00795da0` | `FUN_00795da0` |  |
| 0 | 47 | `0x007a0370` | `MsSEPbattleClear` |  |
| 0 | 49 | `0x00784220` | `MsGetRamChrCTBbasic` |  |
| 0 | 49 | `0x00794e80` | `MsGetChrEffectVariable` |  |
| 0 | 49 | `0x007950b0` | `MsGetChrHeadPos` |  |
| 0 | 49 | `0x007b1e40` | `FUN_007b1e40` |  |
| 0 | 50 | `0x00794240` | `FUN_00794240` |  |
| 0 | 50 | `0x00794280` | `FUN_00794280` |  |
| 0 | 50 | `0x00794d70` | `FUN_00794d70` |  |
| 0 | 50 | `0x00794f70` | `MsGetChrFly` |  |
| 0 | 50 | `0x00795a60` | `FUN_00795a60` |  |
| 0 | 52 | `0x00792ee0` | `FUN_00792ee0` |  |
| 0 | 52 | `0x007cf000` | `FUN_007cf000` |  |
| 0 | 53 | `0x007877d0` | `MsEffectCameraGet` |  |
| 0 | 53 | `0x007993e0` | `FUN_007993e0` |  |
| 0 | 54 | `0x00795e70` | `MsSetChrDirLeave` |  |
| 0 | 55 | `0x007907a0` | `FUN_007907a0` |  |
| 0 | 55 | `0x0079ea90` | `MsResetWeaponEffectAttribute` |  |
| 0 | 55 | `0x007af840` | `FUN_007af840` |  |
| 0 | 55 | `0x007c5680` | `MsScreenCheckShake` |  |
| 0 | 56 | `0x007a88e0` | `FUN_007a88e0` |  |
| 0 | 57 | `0x00798f90` | `MsFieldItemFree` |  |
| 0 | 57 | `0x00799f70` | `MsBtlFreeItemMem` |  |
| 0 | 58 | `0x00793dc0` | `FUN_00793dc0` |  |
| 0 | 58 | `0x007c4f10` | `MsCameraCheckShake` |  |
| 0 | 59 | `0x007881c0` | `MsEffectReadStatInit` |  |
| 0 | 59 | `0x0079f9b0` | `MsNumberGetMem` |  |
| 0 | 59 | `0x007ad710` | `FUN_007ad710` |  |
| 0 | 60 | `0x0078f000` | `FUN_0078f000` |  |
| 0 | 60 | `0x00793530` | `MsBtlChrGetMem` |  |
| 0 | 61 | `0x00795d40` | `FUN_00795d40` |  |
| 0 | 61 | `0x0079a080` | `FUN_0079a080` |  |
| 0 | 62 | `0x00793cb0` | `MsCalcChrDirAll` |  |
| 0 | 62 | `0x007a8be0` | `FUN_007a8be0` |  |
| 0 | 62 | `0x007ad570` | `FUN_007ad570` |  |
| 0 | 63 | `0x00798bd0` | `FUN_00798bd0` |  |
| 0 | 65 | `0x00776f00` | `FUN_00776f00` |  |
| 0 | 65 | `0x00792a40` | `MsResetBtlTarget` |  |
| 0 | 65 | `0x007a0040` | `MsMusicSync` |  |
| 0 | 66 | `0x00794db0` | `FUN_00794db0` |  |
| 0 | 67 | `0x00795ff0` | `FUN_00795ff0` |  |
| 0 | 67 | `0x0079ea40` | `MsPauseChrEffect` |  |
| 0 | 68 | `0x00792120` | `FUN_00792120` |  |
| 0 | 68 | `0x00795dd0` | `FUN_00795dd0` |  |
| 0 | 68 | `0x00795e20` | `MsSetChrDirF` |  |
| 0 | 69 | `0x007902f0` | `PasswordTextDispose` |  |
| 0 | 69 | `0x00798950` | `Rng_ClockSeed` |  |
| 0 | 69 | `0x0079d2a0` | `FUN_0079d2a0` |  |
| 0 | 70 | `0x007c13c0` | `FUN_007c13c0` |  |
| 0 | 71 | `0x007bed10` | `MsCameraResetFilter` |  |
| 0 | 72 | `0x007c1000` | `FUN_007c1000` |  |
| 0 | 73 | `0x00796430` | `FUN_00796430` |  |
| 0 | 73 | `0x007bdf90` | `MsCameraMoveStat` |  |
| 0 | 74 | `0x00791780` | `FUN_00791780` |  |
| 0 | 74 | `0x0079df90` | `FUN_0079df90` |  |
| 0 | 74 | `0x007b1540` | `MsLimitTypeWinCheck` |  |
| 0 | 75 | `0x007bd4b0` | `FUN_007bd4b0` |  |
| 0 | 76 | `0x00797a90` | `MsAtelRegistFree` |  |
| 0 | 76 | `0x007bf1a0` | `FUN_007bf1a0` |  |
| 0 | 77 | `0x00796070` | `FUN_00796070` |  |
| 0 | 77 | `0x007a8920` | `addMonDieSe` |  |
| 0 | 78 | `0x00799fb0` | `MsBtlGetItemMem` |  |
| 0 | 78 | `0x007bcdd0` | `FUN_007bcdd0` |  |
| 0 | 79 | `0x007961d0` | `MsSetChrPos2D` |  |
| 0 | 79 | `0x00796d10` | `FUN_00796d10` |  |
| 0 | 80 | `0x00783d80` | `MsBtlReadScene` |  |
| 0 | 80 | `0x007bbfc0` | `MsScreenMoveCheck` |  |
| 0 | 84 | `0x0078fb00` | `MsMenuFreeText` |  |
| 0 | 85 | `0x007829b0` | `FUN_007829b0` |  |
| 0 | 86 | `0x0079a020` | `FUN_0079a020` |  |
| 0 | 88 | `0x00788f30` | `FUN_00788f30` |  |
| 0 | 88 | `0x0078d620` | `MsResetAvoidFlag` |  |
| 0 | 89 | `0x0079fc00` | `FUN_0079fc00` |  |
| 0 | 91 | `0x00782ae0` | `MsGetMapCenter` |  |
| 0 | 91 | `0x007ab910` | `FUN_007ab910` |  |
| 0 | 91 | `0x007c4f50` | `MsCameraResetShake` |  |
| 0 | 93 | `0x007878a0` | `MsEffectCheckMotion2` |  |
| 0 | 93 | `0x007ad5e0` | `MsChangeWeaponInvisible` |  |
| 0 | 94 | `0x00794990` | `FUN_00794990` |  |
| 0 | 95 | `0x0079a860` | `MsGetDamageLimit` |  |
| 0 | 95 | `0x007b7e20` | `MsEffectChrMotionOut` |  |
| 0 | 97 | `0x00792de0` | `MsSetBtlTarget` |  |
| 0 | 97 | `0x007a8ad0` | `FUN_007a8ad0` |  |
| 0 | 98 | `0x007b19f0` | `FUN_007b19f0` |  |
| 0 | 101 | `0x007b26d0` | `MsAutoInsertCommand_007b26d0` |  |
| 0 | 101 | `0x007b2740` | `FUN_007b2740` |  |
| 0 | 103 | `0x007bdde0` | `MsCameraMoveDivide` |  |
| 0 | 105 | `0x007967f0` | `MsSetChrVisibleEvent` |  |
| 0 | 105 | `0x0079a440` | `MsGetChrID` |  |
| 0 | 105 | `0x007b7c80` | `MsSetBallSplineInit` |  |
| 0 | 109 | `0x00787760` | `MsEffectCameraEnd` |  |
| 0 | 109 | `0x0078ee80` | `FUN_0078ee80` |  |
| 0 | 109 | `0x007abca0` | `Equip_RebuildWearers` |  |
| 0 | 110 | `0x00788d60` | `MsEventEffectStart` |  |
| 0 | 110 | `0x007adf10` | `MsExchangePlayer` |  |
| 0 | 112 | `0x00787c30` | `MsEffectMagicMotion` |  |
| 0 | 113 | `0x007c0a30` | `FUN_007c0a30` |  |
| 0 | 114 | `0x00794c20` | `MsGetChrCenterPos` |  |
| 0 | 114 | `0x00799cd0` | `MsSetDeathSentence` |  |
| 0 | 116 | `0x00799ac0` | `MsSetBerserkCommand` |  |
| 0 | 116 | `0x007a0820` | `MsSoundStreamPlay` |  |
| 0 | 117 | `0x0079b470` | `MsSetCommandUse` |  |
| 0 | 118 | `0x007ac940` | `FUN_007ac940` |  |
| 0 | 119 | `0x007a9a40` | `MsGetTargetDist` |  |
| 0 | 120 | `0x007bec90` | `MsCameraResetDirVector` |  |
| 0 | 122 | `0x007ab970` | `MsEquipSaveWeapon` |  |
| 0 | 124 | `0x0079ca80` | `FUN_0079ca80` |  |
| 0 | 124 | `0x0079dde0` | `MsBattleSpecialFree` |  |
| 0 | 124 | `0x007c09b0` | `FUN_007c09b0` |  |
| 0 | 126 | `0x00796bb0` | `FUN_00796bb0` |  |
| 0 | 130 | `0x00787d70` | `FUN_00787d70` |  |
| 0 | 132 | `0x007abeb0` | `FUN_007abeb0` |  |
| 0 | 134 | `0x007ab0b0` | `MsMoveSplineRegist` |  |
| 0 | 135 | `0x007a9fc0` | `MsSetMove` |  |
| 0 | 135 | `0x007cf860` | `FUN_007cf860` |  |
| 0 | 137 | `0x00796220` | `MsSetChrPosReset` |  |
| 0 | 138 | `0x0079fc60` | `FUN_0079fc60` |  |
| 0 | 140 | `0x007850b0` | `MsGetSaveCommandMenuPanel` |  |
| 0 | 140 | `0x00799790` | `MsGetCommandTarget2` |  |
| 0 | 140 | `0x007a0720` | `MsSetVoice` |  |
| 0 | 140 | `0x007bf1f0` | `FUN_007bf1f0` |  |
| 0 | 141 | `0x0079d0e0` | `MsBtlListCDpos` |  |
| 0 | 145 | `0x00795980` | `MsGetChrTopPos` |  |
| 0 | 145 | `0x007bc750` | `FUN_007bc750` |  |
| 0 | 146 | `0x007bfe70` | `MsCameraSetSplineFilter` |  |
| 0 | 147 | `0x007ab010` | `MsMoveSpline` |  |
| 0 | 147 | `0x007baac0` | `MsSplineRegist` |  |
| 0 | 149 | `0x007c0910` | `FUN_007c0910` |  |
| 0 | 150 | `0x007828a0` | `Enc_ResetWeights` |  |
| 0 | 155 | `0x00799a10` | `MsGetSameTarget` |  |
| 0 | 157 | `0x007bce80` | `FUN_007bce80` |  |
| 0 | 159 | `0x007b6f10` | `FUN_007b6f10` |  |
| 0 | 160 | `0x007bc890` | `FUN_007bc890` |  |
| 0 | 162 | `0x0078d8d0` | `MsAutoRelifeProcess` |  |
| 0 | 162 | `0x00799020` | `MsFieldItemRead` |  |
| 0 | 163 | `0x007bf280` | `MsCameraSetFocus` |  |
| 0 | 164 | `0x007afd30` | `MsEffectLimitStart` |  |
| 0 | 165 | `0x00795c90` | `FUN_00795c90` |  |
| 0 | 166 | `0x007bc010` | `MsScreenMoveReset` |  |
| 0 | 169 | `0x007bcf50` | `FUN_007bcf50` |  |
| 0 | 175 | `0x00784610` | `FUN_00784610` |  |
| 0 | 180 | `0x007bbdf0` | `MsScreenMove` |  |
| 0 | 182 | `0x007c56f0` | `MsScreenShake` |  |
| 0 | 189 | `0x0078db70` | `MsSetDamageMotion` |  |
| 0 | 190 | `0x00785d40` | `MsSetSaveCommandMenuPanel` |  |
| 0 | 190 | `0x007981e0` | `MsAtelRequestSystem` |  |
| 0 | 190 | `0x007a9320` | `MsGetChrMoveFrameRest` |  |
| 0 | 197 | `0x007ba780` | `MsAtelGetCameraID` |  |
| 0 | 198 | `0x00791690` | `FUN_00791690` |  |
| 0 | 208 | `0x007a8720` | `MsGetBasicDivide` |  |
| 0 | 211 | `0x0078d7f0` | `MsSetChrWeak` |  |
| 0 | 230 | `0x007bf0b0` | `FUN_007bf0b0` |  |
| 0 | 233 | `0x007a87f0` | `MsGetDirDivide` |  |
| 0 | 239 | `0x007982a0` | `MsAtelRequestSystemCheckExe` |  |
| 0 | 240 | `0x0078cd30` | `MsGetComInfo` |  |
| 0 | 245 | `0x007bff10` | `MsCameraSetStartPos` |  |
| 0 | 249 | `0x0079a760` | `MsGetDamageInfoNext` |  |
| 0 | 250 | `0x007ac840` | `FUN_007ac840` |  |
| 0 | 250 | `0x007b12c0` | `MsLimitTypeStatusCheck` |  |
| 0 | 254 | `0x007b2c90` | `FUN_007b2c90` |  |
| 0 | 262 | `0x00793f10` | `MsDetachWeapon` |  |
| 0 | 264 | `0x00782c00` | `MsSetCountryName` |  |
| 0 | 264 | `0x007b0f80` | `MsLimitTypeDeathCheck` |  |
| 0 | 271 | `0x007bbeb0` | `MsScreenMoveAcc` |  |
| 0 | 280 | `0x0078ac20` | `MsCalcMotionSpeed` |  |
| 0 | 282 | `0x007ab750` | `FUN_007ab750` |  |
| 0 | 294 | `0x007c57b0` | `MsScreenShake2` |  |
| 0 | 296 | `0x007c4fc0` | `MsCameraShake` |  |
| 0 | 297 | `0x007842f0` | `FUN_007842f0` |  |
| 0 | 306 | `0x00784d00` | `MsExchangeSaveParty` |  |
| 0 | 325 | `0x007acb40` | `FUN_007acb40` |  |
| 0 | 332 | `0x007b055a` | `FUN_007b055a` |  |
| 0 | 350 | `0x00799d50` | `MsSetProvokingCommand` |  |
| 0 | 364 | `0x0079fe80` | `FUN_0079fe80` |  |
| 0 | 372 | `0x007ab540` | `MsGetMoveTargetDist` |  |
| 0 | 372 | `0x007b13c0` | `MsLimitTypeTurnCheck` |  |
| 0 | 379 | `0x0079aff0` | `MsMergeDamage` |  |
| 0 | 380 | `0x007ab9f0` | `SwapSaveGear` |  |
| 0 | 419 | `0x007a8c80` | `ms_motion_trans` |  |
| 0 | 429 | `0x007beae0` | `MsCameraReset` |  |
| 0 | 453 | `0x007c4d40` | `FUN_007c4d40` |  |
| 0 | 549 | `0x007c0ae0` | `FUN_007c0ae0` |  |
| 0 | 550 | `0x007b0d50` | `MsLimitTypeDamageCheck` |  |
| 0 | 662 | `0x007bda80` | `MsCameraMoveCalc` |  |
| 0 | 705 | `0x00780d10` | `MsBattleEncountExe` |  |
| 0 | 783 | `0x007c3170` | `MsRotMatrixX` |  |
| 0 | 783 | `0x007c3480` | `MsRotMatrixY` |  |
| 0 | 783 | `0x007c3790` | `MsRotMatrixZ` |  |
| 0 | 844 | `0x007c3d60` | `MsScreenMatrix` |  |
| 0 | 891 | `0x007c58e0` | `MsCamSetBtl` |  |
| 0 | 1157 | `0x0079c5f0` | `MsSetRamChrParam` |  |
| 0 | 1269 | `0x00785410` | `MsGetSaveWindow` |  |
| 0 | 1385 | `0x007c50f0` | `MsCameraShakeProcess` |  |
| 0 | 2366 | `0x007c1be0` | `MsCameraMatrix` |  |
| 1 | 9 | `0x00790290` | `FUN_00790290` | MsEffectSetMotion |
| 1 | 16 | `0x00798b60` | `MsBtlGetInit` | FUN_007993e0 |
| 1 | 16 | `0x0079e7a0` | `MsEtEffectSet` | op_et_battle_effect_init |
| 1 | 21 | `0x0078f8f0` | `MsGetRomBtlText` | MsMenuGetText |
| 1 | 21 | `0x0078fb60` | `MsMenuGetBtlEndText` | MsMenuGetText |
| 1 | 21 | `0x007962b0` | `MsSetChrShadow` | MsSetChrShadowMode |
| 1 | 24 | `0x00788b70` | `MsEffectWeaponInit` | op_et_weapon_init |
| 1 | 24 | `0x007ba9a0` | `MsBindCamera` | FUN_007baa50 |
| 1 | 25 | `0x00780c70` | `FUN_00780c70` | Enc_ResetWeights |
| 1 | 25 | `0x00784b60` | `MsDebugFullSummon` | MsSetSavePlyJoin |
| 1 | 26 | `0x00784ac0` | `MsDebugFullMember` | MsSetSavePlyJoin |
| 1 | 26 | `0x00798fd0` | `MsFieldItemGet` | msFieldItemGet |
| 1 | 29 | `0x0079e4f0` | `MsEffectSendSignal` | FUN_007fbdf0 |
| 1 | 30 | `0x0079ed00` | `MsSetStealEffect` | MsSetChrEffectNext_0079ebe0 |
| 1 | 30 | `0x0079ed20` | `MsSetStealGillEffect` | MsSetChrEffectNext_0079ebe0 |
| 1 | 31 | `0x00783e00` | `MsBtlReadSceneFree` | Sg_FreeA |
| 1 | 33 | `0x00786900` | `MsSetSaveParamAll` | MsSetSaveParam |
| 1 | 34 | `0x00783b90` | `MsBtlReadPlayerModelProcess` | MsBtlChrRegist |
| 1 | 36 | `0x007a00d0` | `MsRegSEfade` | MsSEP3DfadeOut |
| 1 | 36 | `0x007b7650` | `FUN_007b7650` | MsResetAvoidFlag |
| 1 | 38 | `0x00786950` | `MsSetSavePartyMember` | MsExchangeSaveParty |
| 1 | 38 | `0x007873a0` | `resetDefaultPlayerNameStatus` | setDefaultPlayerName |
| 1 | 38 | `0x0079e670` | `MsEtEffectInit` | FUN_0079f1c0 |
| 1 | 39 | `0x00782b70` | `FUN_00782b70` | MsMenuReadText |
| 1 | 40 | `0x0079cb10` | `MsSetRamMonData` | MsSetRamChr |
| 1 | 41 | `0x00796880` | `FUN_00796880` | MsSetChrVisible |
| 1 | 41 | `0x007968b0` | `FUN_007968b0` | MsSetChrVisible |
| 1 | 41 | `0x0079eae0` | `FUN_0079eae0` | MsPauseChrEffect |
| 1 | 42 | `0x007993b0` | `MsSetWeaponName` | MsWeaponNameNum |
| 1 | 43 | `0x00782b40` | `FUN_00782b40` | Enc_ResetWeights |
| 1 | 43 | `0x0078de80` | `MsSetEscapeFlagAll` | MsSetEscapeFlag |
| 1 | 44 | `0x00782640` | `FUN_00782640` | Sg_FreeA |
| 1 | 45 | `0x008be0a0` | `MsExchangeSaveWeapon` | SwapSaveGear |
| 1 | 47 | `0x007a0140` | `MsRegSEplay2` | MsSEP3Dregist |
| 1 | 47 | `0x008741d0` | `FUN_008741d0` | TkCdSize |
| 1 | 48 | `0x00793d90` | `MsChrCreatePacket` | Ch_ModelCreatePacket |
| 1 | 49 | `0x007a08a0` | `MsSEP3DfadeOut` | FUN_007a0b90 |
| 1 | 50 | `0x00781c50` | `MsBattleInitAtEvent` | Enc_ResetWeights |
| 1 | 51 | `0x00782050` | `MsBattleReadData` | MsBattleReadDataNB |
| 1 | 51 | `0x0079fdf0` | `FUN_0079fdf0` | FUN_0079fd40 |
| 1 | 51 | `0x007a0170` | `MsRegSEplay3` | MsRegSEplay |
| 1 | 53 | `0x00788ef0` | `FUN_00788ef0` | MsAtelCamRegist |
| 1 | 54 | `0x007b2910` | `MsAutoRelifeProcess_007b2910` | FUN_007b2740 |
| 1 | 56 | `0x00781550` | `MsBattleExeSetIndex` | MsGetFileSize |
| 1 | 56 | `0x00788b90` | `MsEventEffectFree` | MsEventEffectProcess |
| 1 | 56 | `0x0079eba0` | `FUN_0079eba0` | MsSetChrEffect_0079eb10 |
| 1 | 57 | `0x0078de40` | `MsSetEscapeFlag` | MsSetChrVisibleAppear |
| 1 | 58 | `0x007844c0` | `readBtlMonsterName` | TKMallocAndReadEventMessage |
| 1 | 58 | `0x00798720` | `FUN_00798720` | MsBindCamera |
| 1 | 61 | `0x007a0100` | `MsRegSEplay` | MsSEP3Dregist |
| 1 | 64 | `0x007987b0` | `FUN_007987b0` | AtelGetBattleIdAdrs |
| 1 | 66 | `0x00788430` | `MsEffectSetSpeed` | MsCalcMotionSpeed |
| 1 | 66 | `0x00789160` | `MsAliveProcess` | MsRelife |
| 1 | 66 | `0x0079e590` | `MsEtCriticalEffect` | op_et_eff |
| 1 | 67 | `0x0079dfe0` | `MsSpCameraProcess` | MsTransSpPos |
| 1 | 68 | `0x0079f1c0` | `FUN_0079f1c0` | MsBattleError |
| 1 | 69 | `0x00797490` | `MsAtelActorWorkInit` | AtelInitInternalActorWork |
| 1 | 70 | `0x00799f20` | `MsMemCheckSum` | MsBattleError |
| 1 | 72 | `0x007ae0a0` | `MsFreePlayer` | MsBtlChrFree |
| 1 | 75 | `0x0078e1e0` | `MsSubCTB` | MsNumberRegist |
| 1 | 75 | `0x0079fe30` | `FUN_0079fe30` | FUN_0079fd40 |
| 1 | 76 | `0x00798840` | `FUN_00798840` | FUN_007987f0 |
| 1 | 79 | `0x007baa50` | `FUN_007baa50` | MsAtelGetCameraID |
| 1 | 82 | `0x0079ebe0` | `MsSetChrEffectNext_0079ebe0` | op_et_eff |
| 1 | 83 | `0x0079ec40` | `FUN_0079ec40` | op_et_eff |
| 1 | 83 | `0x0079eca0` | `FUN_0079eca0` | op_et_eff |
| 1 | 89 | `0x00783f60` | `MsBtlReadWeapon` | MsCDreadWeapon |
| 1 | 90 | `0x00797350` | `FUN_00797350` | AtelDecodeCommand |
| 1 | 90 | `0x007973b0` | `FUN_007973b0` | AtelDecodeCommand |
| 1 | 90 | `0x0079f160` | `MsStatusEffectReflect` | op_et_eff |
| 1 | 94 | `0x0078e3b0` | `MsSubMP` | MsNumberRegist |
| 1 | 95 | `0x00782ba0` | `FUN_00782ba0` | Enc_ResetWeights |
| 1 | 98 | `0x00790430` | `FUN_00790430` | TkFree |
| 1 | 98 | `0x0079e2a0` | `FUN_0079e2a0` | yiSetRotTransDObj |
| 1 | 108 | `0x00788200` | `MsEffectReady` | FUN_00788f90 |
| 1 | 108 | `0x007b7d00` | `MsEffectChrGetLoveLoveMax` | MsGetRamChrFemale |
| 1 | 110 | `0x007a2600` | `MsWeaponNameRead` | getExcelBinSize |
| 1 | 110 | `0x007afc90` | `MsEffectLimitCheck` | MsEffectLimitStart |
| 1 | 111 | `0x007a06b0` | `MsSEPtransRegist` | MsSEPtransProcess |
| 1 | 112 | `0x007908c0` | `MsImportantRead` | getExcelBinSize |
| 1 | 114 | `0x007bb840` | `MsAtelScreenShake` | MsScreenShake |
| 1 | 123 | `0x007932b0` | `MesSetChrTargetPosToLimitSparePosy` | FUN_00796bb0 |
| 1 | 127 | `0x007cf8f0` | `ms_free` | FUN_007cf860 |
| 1 | 128 | `0x007c0650` | `FUN_007c0650` | FUN_0086a6e0 |
| 1 | 130 | `0x0078e150` | `MsStoneProcess` | MsRegSEplay |
| 1 | 130 | `0x007b0470` | `MsMenuLimitSet_007b0470` | MsMenuLimitSet |
| 1 | 131 | `0x00784260` | `Mon_GetName` | readBtlMonsterName |
| 1 | 131 | `0x007abf40` | `FUN_007abf40` | FUN_007abeb0 |
| 1 | 132 | `0x00783b00` | `Mon_AttachData` | MsRegistSEdata |
| 1 | 132 | `0x0079ccf0` | `MsRegMotionRead` | FUN_00837710 |
| 1 | 132 | `0x007b7440` | `FUN_007b7440` | FUN_007b7440 |
| 1 | 134 | `0x00874130` | `FUN_00874130` | TkFree |
| 1 | 135 | `0x0079eb10` | `MsSetChrEffect_0079eb10` | op_et_eff |
| 1 | 138 | `0x00787650` | `FUN_00787650` | ms_motion_trans |
| 1 | 143 | `0x007962d0` | `MsSetChrShadowMode` | MsSetChrShadowMode |
| 1 | 148 | `0x00784420` | `FUN_00784420` | MsSetChrVisibleAppear |
| 1 | 150 | `0x007ad640` | `MsChangeWeaponWakka` | MsFieldItemGetMes |
| 1 | 155 | `0x007a8b40` | `FUN_007a8b40` | MsGetCommandTarget2 |
| 1 | 157 | `0x007b27b0` | `MsAutoPhenixProcess` | MsAutoInsertCommand_007b26d0 |
| 1 | 158 | `0x007acc90` | `MsActReCommand` | FUN_007b19d0 |
| 1 | 168 | `0x00796f70` | `FUN_00796f70` | Ch_EngineSetFast |
| 1 | 173 | `0x0078f700` | `MsCheckLeftWindow` | MsGetCommandUse |
| 1 | 183 | `0x0078d3e0` | `MsOrderCalc` | FUN_0078f000 |
| 1 | 185 | `0x00785b60` | `MsGetChrAbilityMap` | FUN_007987f0 |
| 1 | 187 | `0x00782a20` | `MsGetFileSize` | MsGetSaveConfigEnglish |
| 1 | 189 | `0x0078fcf0` | `MsMenuGetText` | MsGetSaveConfigEnglish |
| 1 | 189 | `0x007b1910` | `FUN_007b1910` | FUN_007b19d0 |
| 1 | 189 | `0x007b2850` | `MsAutoPotionProcess` | MsAutoInsertCommand_007b26d0 |
| 1 | 193 | `0x007ba860` | `MsAtelResetMoveCameraID` | MsCameraResetMove |
| 1 | 198 | `0x0079ed40` | `MsSetWeaponEffectAttribute` | op_et_eff |
| 1 | 205 | `0x00787950` | `MsEffectEndMotion` | MsSetChrWeak |
| 1 | 207 | `0x0079a5b0` | `MsGetCommandUse` | FUN_0079a080 |
| 1 | 210 | `0x007a9150` | `FUN_007a9150` | MsGetMoveTargetDist |
| 1 | 210 | `0x007b6fb0` | `MsGetTargetLinkList` | FUN_007b7440 |
| 1 | 222 | `0x00790340` | `PasswordTextRead` | FUN_00790430 |
| 1 | 222 | `0x007b77c0` | `FUN_007b77c0` | MsSetDamageMotion |
| 1 | 227 | `0x00785fc0` | `MsSetSaveData` | MsSetSaveParam |
| 1 | 227 | `0x007c0560` | `FUN_007c0560` | FUN_0086b1e0 |
| 1 | 228 | `0x007bed60` | `MsCameraResetMove` | MsCameraSetRect |
| 1 | 231 | `0x007869b0` | `MsSetSavePlyJoin` | MsChangeWeaponInvisible |
| 1 | 241 | `0x00784120` | `MsCDreadWeapon` | FUN_00784000 |
| 1 | 245 | `0x0078fbb0` | `MsMenuGetHelp` | MsGetSaveConfigEnglish |
| 1 | 249 | `0x008be0d0` | `FUN_008be0d0` | DrawCrossMenuIconWeaponName |
| 1 | 254 | `0x007a0930` | `MsSEP3Dprocess` | FUN_007a0bb0 |
| 1 | 259 | `0x007b7170` | `MsGetTargetRandomList` | MsBattleError |
| 1 | 268 | `0x007848b0` | `MsDebugFullCommand` | Sg_FreeA |
| 1 | 276 | `0x0079ee20` | `MsStatusDefenseEffect` | MsSetChrEffectNext_0079ebe0 |
| 1 | 290 | `0x0078f7c0` | `MsGetMenuChrInfo` | MsGetSaveConfigEnglish |
| 1 | 299 | `0x007882c0` | `MsEffectSetMotion` | MsEffectSetSpeed |
| 1 | 314 | `0x007ac9c0` | `Battle_QueueCommand` | FUN_00796430 |
| 1 | 314 | `0x007bde50` | `MsCameraMoveFrame` | MsCameraSetStartPos |
| 1 | 317 | `0x007bf940` | `MsCameraSetRect` | MsSetCheckChange |
| 1 | 332 | `0x00782df0` | `getExcelBinSize` | TkFree |
| 1 | 336 | `0x00788920` | `MsEffectTerminate` | MsBattleError |
| 1 | 346 | `0x007a0a30` | `MsSEP3Dregist` | FUN_007a0bb0 |
| 1 | 362 | `0x007a0540` | `MsSEPtransProcess` | MsBtlReadSceneFree |
| 1 | 369 | `0x0078e230` | `MsSubHP` | MsNumberRegist |
| 1 | 374 | `0x007b72c0` | `MsGetTargetSortList` | FUN_007b6f10 |
| 1 | 383 | `0x007c0790` | `MsSetCheckChange` | FUN_007bcab0 |
| 1 | 439 | `0x007a01b0` | `MsRegistSEdata` | MsSEPtransRegist |
| 1 | 440 | `0x007b2510` | `MsAutoCureProcess` | MsAutoInsertCommand_007b26d0 |
| 1 | 504 | `0x007b1710` | `FUN_007b1710` | FUN_007b1e40 |
| 1 | 542 | `0x007bd860` | `MsCameraMoveAcc` | MsCameraSetStartPos |
| 1 | 728 | `0x007990d0` | `MsGetItem` | BattleRewards_AddGear |
| 1 | 806 | `0x0079f660` | `MsNumberDrawProcess` | FUN_0079fc00 |
| 1 | 880 | `0x007c0120` | `FUN_007c0120` | spline_get_near_point |
| 1 | 939 | `0x007b78c0` | `MsSetBallSpline` | MsMoveSplineRegist |
| 1 | 1393 | `0x007afe00` | `MsMenuLimitSet` | MsGetTargetRandomList |
| 1 | 2055 | `0x007860f0` | `MsSetSaveParam` | MsGetChrAbilityMap |
| 1 | 2555 | `0x007a1c00` | `MsWeaponNameNum_Armor_007a1c00` | MsGearHasAbility_007a0c20 |
| 1 | 3811 | `0x007a0d10` | `MsWeaponNameNum_Weapon_007a0d10` | MsGearHasAbility_007a0c20 |
| 2 | 20 | `0x00780cc0` | `MsBattleCursorCalc` | MsNumberDrawProcess, MsCalcCursorPos |
| 2 | 23 | `0x00783990` | `FUN_00783990` | Mon_AttachData, MsBtlReadMonster |
| 2 | 30 | `0x00796680` | `FUN_00796680` | FUN_00796880, FUN_007968b0 |
| 2 | 30 | `0x007a0cf0` | `MsWeaponNameNum` | MsWeaponNameNum_Armor_007a1c00, MsWeaponNameNum_Weapon_007a0d10 |
| 2 | 38 | `0x00798ff0` | `MsFieldItemGetMes` | FUN_007993e0, msFieldItemGet |
| 2 | 43 | `0x0079fff0` | `MsMusicReadStart` | MsEffectSepClearAll, MsSEPbattleClear |
| 2 | 44 | `0x0079fcf0` | `FUN_0079fcf0` | MsSEP3DfadeOut, FUN_0079fe80 |
| 2 | 48 | `0x007891b0` | `MsBlowProcess` | MsSetEscapeFlag, MsRelife |
| 2 | 49 | `0x00783fc0` | `FUN_00783fc0` | MsBtlReadWeapon, FUN_00784000 |
| 2 | 59 | `0x00787ab0` | `MsEffectForceFree` | op_ovl_effect_forced_free, MagicFile_GetEffectOverlayTable |
| 2 | 69 | `0x0079fda0` | `FUN_0079fda0` | MsSEP3Dregist, FUN_0079fe80 |
| 2 | 72 | `0x00788d10` | `MsEventEffectReadComplete` | op_ovl_effect_init, MagicFile_GetEffectOverlayTable |
| 2 | 73 | `0x0079d090` | `FUN_0079d090` | MsGetBattleMessageFlag, FUN_0089b1d0 |
| 2 | 78 | `0x0079e5e0` | `MsEtEffectEncount` | MsRegSEplay2, op_et_encount_effect_on |
| 2 | 83 | `0x0079fd40` | `FUN_0079fd40` | MsSEP3Dregist, FUN_0079fe80 |
| 2 | 83 | `0x007a8f60` | `FUN_007a8f60` | MsGetSameTarget, FUN_00796430 |
| 2 | 87 | `0x007ab2a0` | `MsSetBtlPosReset` | MsSetChrPosReset, MsSetChrDirF |
| 2 | 88 | `0x007a0bb0` | `FUN_007a0bb0` | MsGetChrTopPos, MsGetChrCenterPos |
| 2 | 91 | `0x0079e830` | `MsMagicEffectEnd` | MsMagicEffectEnd, op_et_prayeff_send_read_through |
| 2 | 103 | `0x00784000` | `FUN_00784000` | MsRegistSEdata, MsEffectWeaponInit |
| 2 | 103 | `0x00797550` | `MsAtelCamRegist` | MsAtelActorWorkInit, MsAtelSetUp |
| 2 | 114 | `0x0079e510` | `MsEffectSetGuard` | FUN_0079ec40, MsRegSEplay |
| 2 | 117 | `0x00797cd0` | `MsAtelRequestCancel` | AtelExecReturn2, AtelSkipReqLevel2 |
| 2 | 120 | `0x00782770` | `MsEnvInit` | PcFileTblSet, MsSetCountryName |
| 2 | 123 | `0x007cf970` | `FUN_007cf970` | FUN_007cf860, debug_exit_trace |
| 2 | 124 | `0x00793480` | `MsBtlChrFreeAll` | Ch_Dispose, MsDebugFullCommand_00796dc0 |
| 2 | 133 | `0x00782d60` | `MsWalkAbilityProcess` | MsSaveAddHP, FUN_00785ae0 |
| 2 | 134 | `0x0079e6a0` | `MsEtEffectRead` | op_et_battle_init, MsBattleError |
| 2 | 147 | `0x007bad10` | `FUN_007bad10` | MsCameraSetBtl, MsAtelGetCameraID |
| 2 | 147 | `0x007bcd30` | `FUN_007bcd30` | TkMtCalcSpline, TkMtCalcSpline3 |
| 2 | 150 | `0x007ba5b0` | `FUN_007ba5b0` | MsAtelGetCameraID, MsCameraMoveAcc |
| 2 | 152 | `0x007bb210` | `FUN_007bb210` | MsAtelGetCameraID, FUN_007bce80 |
| 2 | 155 | `0x007bb370` | `MsAtelCameraRefAddCircleArea_007bb370` | MsAtelGetCameraID, FUN_007bf1f0 |
| 2 | 165 | `0x00788c60` | `MsEventEffectRead` | MagicFile_Load, op_ovl_effect_init |
| 2 | 165 | `0x007b7d70` | `MsEffectChrMotionIn` | MsSetChrPosReset, MsSetWeaponEffectAttribute |
| 2 | 169 | `0x007bab90` | `FUN_007bab90` | MsAtelGetCameraID, FUN_007beeb0 |
| 2 | 169 | `0x007bb520` | `FUN_007bb520` | MsAtelGetCameraID, MsCameraMoveDivide |
| 2 | 175 | `0x007bb8c0` | `MsAtelCameraShake` | MsAtelGetCameraID, MsCameraShake |
| 2 | 181 | `0x007aab80` | `MsMoveChr` | FUN_007ab700, MsGetChrPredictPos |
| 2 | 189 | `0x007bb2b0` | `FUN_007bb2b0` | MsAtelGetCameraID, FUN_007bf0b0 |
| 2 | 193 | `0x007bac40` | `FUN_007bac40` | FUN_007befc0, MsAtelGetCameraID |
| 2 | 211 | `0x00796c30` | `getSparedLmtPos` | graphicProject3DPointOntoScreenConsider2DScene, FUN_00796bb0 |
| 2 | 213 | `0x007bfd90` | `MsCameraSetSpline2` | FUN_007bcd30, MsSetCheckChange |
| 2 | 215 | `0x007bb970` | `FUN_007bb970` | MsAtelGetCameraID, MsCameraSetSpline |
| 2 | 225 | `0x007bf330` | `FUN_007bf330` | FUN_007c0910, MsSetCheckChange |
| 2 | 226 | `0x007befc0` | `FUN_007befc0` | FUN_007c0650, MsSetCheckChange |
| 2 | 234 | `0x007bfca0` | `MsCameraSetSpline` | FUN_007bcb80, MsSetCheckChange |
| 2 | 238 | `0x0078d530` | `MsRelife` | MsSetEscapeFlag, MsSetChrWeak |
| 2 | 239 | `0x00793e00` | `MsCommandReturnChrStatusAll` | MsStoneMove, MsEffectEndMotion |
| 2 | 240 | `0x0079e1b0` | `MsTransSpPos` | MsRotMatrixZ, MsRotMatrixX |
| 2 | 249 | `0x007aa160` | `MsMotionProcess` | MsCalcMotionSpeed, FUN_007ab750 |
| 2 | 265 | `0x00784720` | `MsCheatPlayerPowerup` | MsNumberRegist, MsSetChrWeak |
| 2 | 267 | `0x007beeb0` | `FUN_007beeb0` | FUN_007c0560, MsSetCheckChange |
| 2 | 269 | `0x00796550` | `MsSetChrTransparentVisible` | MsSetChrVisibleAppear, MsSetChrTransparentVisible |
| 2 | 275 | `0x007c0d40` | `FUN_007c0d40` | spline_get_near_point, spline_get_near_point_init |
| 2 | 302 | `0x0079de60` | `MsBattleSpecialInit` | Yn_SeaCreate, Yn_PSimTest |
| 2 | 308 | `0x007839b0` | `MsBtlReadMonsterProcess` | MsBtlChrRegist, MsSetChrTransparentVisible |
| 2 | 337 | `0x0079e8c0` | `MsMagicEffectStart` | MsRegSEplay, op_et_eff |
| 2 | 343 | `0x007c5c60` | `MsCameraSetBtl` | MsCamSetBtl, MsSetCheckChange |
| 2 | 391 | `0x007a8fc0` | `Battle_ScriptPlayMotion` | MsCalcMotionSpeed, ms_motion_trans |
| 2 | 407 | `0x0079fa00` | `MsNumberRegist` | FUN_0079fc00, FUN_0079fc60 |
| 2 | 412 | `0x00799420` | `msFieldItemGet` | FUN_00798bd0, MsWeaponNameNum |
| 2 | 446 | `0x00790fb0` | `MsBattleProcess_Order_00790fb0` | MsOrderCalc, MsLimitTypeTurnCheck |
| 2 | 566 | `0x0079a200` | `MsGetCTBlist` | FUN_0079a020, MsOrderCalc |
| 2 | 599 | `0x0079f380` | `MsCalcCursorPos` | MsGetChrCenterPos, graphicProject3DPointOntoScreenConsider2DScene |
| 2 | 646 | `0x0078fef0` | `MsMenuReadText` | MsGetFileSize, getExcelBinSize |
| 2 | 732 | `0x007b1b50` | `MsSetReelResult` | FUN_007b19d0, FUN_007b1710 |
| 2 | 1109 | `0x007badb0` | `FUN_007badb0` | MsCameraSetBtl, MsAtelGetCameraID |
| 2 | 1234 | `0x0079c110` | `MsSetRamChrData` | MsGetChrID, MsSetRamChr |
| 3 | 95 | `0x00788c00` | `MsEventEffectProcess` | MsEffectCameraEnd, op_ovl_effect_free, MagicFile_Stop |
| 3 | 100 | `0x007972e0` | `FUN_007972e0` | MsMotionProcess, AtelDecodeCommand, MsBattleSpecial |
| 3 | 138 | `0x007aef20` | `MsReturnSummon` | MsFreePlayer, MsRelife, MsExchangePlayer |
| 3 | 139 | `0x007b75c0` | `FUN_007b75c0` | FUN_0079fd40, MsSetDamage, FUN_00793dc0 |
| 3 | 155 | `0x00788f90` | `FUN_00788f90` | FUN_00789030, FUN_00788f30, MsRegSEfade |
| 3 | 166 | `0x00784070` | `Mon_LoadAll` | FUN_007842f0, MsSetRamMonData, MsAtelSetUp |
| 3 | 176 | `0x007afab0` | `MsPoisonProcess` | MsDamageCheckDeath, MsSubHP, MsSetChrWeak |
| 3 | 201 | `0x007ad750` | `MsWeaponDispose` | ChN_DataDispose, Ch_Dispose, ChN_DisposeSystemMGRP |
| 3 | 206 | `0x007bb5d0` | `FUN_007bb5d0` | Cam_SetPolar, MsAtelGetCameraID, Cam_AddPolar |
| 3 | 234 | `0x00787af0` | `MsEffectFree` | op_ovl_effect_free, MagicFile_Stop, MsAtelRequestCancel |
| 3 | 249 | `0x007bf420` | `FUN_007bf420` | FUN_007c0a30, MsSetCheckChange, FUN_007c09b0 |
| 3 | 270 | `0x007a89c0` | `MsSetChrVisibleBtl2Event` | MsRelife, FUN_007fae90, MsSetRamChr |
| 3 | 271 | `0x007bb410` | `MsAtelCameraEyeAddArea_007bb410` | MsAtelGetCameraID, FUN_007bf420, FUN_007bf330 |
| 3 | 285 | `0x00787e00` | `MsEffectProcess` | MagicFile_GetEffectOverlayTable, op_effect_run_after, op_effect_run_before |
| 3 | 298 | `0x00798390` | `MsAtelSetUp` | AtelGetMyBufferSize, MsBattleError, AtelBattleSetUp |
| 3 | 374 | `0x0079e030` | `FUN_0079e030` | MsRotMatrixY, MsRotMatrixZ, MsRotMatrixX |
| 3 | 408 | `0x007bb6a0` | `setCameraMatrixToPhyreScene` | MsAtelGetCameraID, FUN_0079e2a0, MsCameraSetRect |
| 3 | 623 | `0x007976a0` | `MsAtelChrInit` | FUN_00798720, AtelGetBattleIdAdrs, FUN_007987b0 |
| 3 | 630 | `0x00798c10` | `BattleRewards_AddGear` | FUN_00798bd0, MsWeaponNameNum, FUN_00798a90 |
| 4 | 110 | `0x00793410` | `MsBtlChrFree` | MsSetChrVisibleAppear, MsDebugFullCommand_00796dc0, MsWeaponDispose, MsAtelRequestCancel |
| 4 | 167 | `0x007b74d0` | `FUN_007b74d0` | FUN_007b77c0, FUN_007b75c0, FUN_007b7680, FUN_007b7650 |
| 4 | 213 | `0x00796dc0` | `MsDebugFullCommand_00796dc0` | ChN_DataDispose, Ch_Dispose, Sg_FreeA, ChN_DisposeSystemMGRP |
| 4 | 280 | `0x007b7680` | `FUN_007b7680` | FUN_0079fd40, MsSetDamage, FUN_0079eba0, MsEtCriticalEffect |
| 4 | 301 | `0x00789030` | `FUN_00789030` | MsRegSEplay2, MsStatusEffectReflect, MsLimitTypeStatusCheck, FUN_00788f30 |
| 4 | 315 | `0x00783e20` | `MsBtlReadSetScene` | MsBtlBridgeEvent2Btl, AtelEventRemoveForBattle, MsSetRamChrData, MsGetSaveConfigEnglish |
| 4 | 383 | `0x007ac6a0` | `MsSetChrPosAfterSummon` | MsSetChrPosReset, MsSetChrVisibleAppear, MsSetChrDirF, MsRegSEfade |
| 4 | 615 | `0x00793670` | `MsBtlChrRegist` | Ch_Allocate, MsChrCreatePacket, MsSetChrVisibleAppear, FUN_00796f70 |
| 4 | 631 | `0x007af4f0` | `MsCheckStatusBeforeAction` | MsDamageCheckDeath, FUN_007af840, MsSubHP, MsSetChrWeak |
| 4 | 766 | `0x00795250` | `MsGetChrLimitBarFlag` | graphicProject3DPointOntoScreenConsider2DScene, FUN_00796d10, MesSetChrTargetPosToLimitSparePosy, getSparedLmtPos |
| 4 | 774 | `0x00783680` | `MsBtlReadMonster` | Mon_AttachData, readBtlMonsterName, MsGetChrID, Mon_LoadAll |
| 5 | 288 | `0x007ad820` | `MsWeaponRead` | FUN_007ad570, Sg_FreeA, MsWeaponDispose, MsEquipSaveWeapon, MsSetSaveParam |
| 5 | 293 | `0x00780b40` | `MsBattleAllFree` | FUN_00782640, FUN_00799ec0, MsMenuFreeText, MsFieldItemFree, MsBattleFree |
| 5 | 342 | `0x0078d980` | `MsSetDamage` | FUN_0078ee80, MsRegSEplay, MsDamageCheckDeath, AfterDamageProcess, MsBlowProcess |
| 5 | 406 | `0x00787470` | `setDefaultPlayerName` | FUN_00875220, debugSetRikkuName, FUN_008c5370, FUN_00875370, MsMenuGetText |
| 5 | 437 | `0x00783bc0` | `MsBtlReadPlayerProcess` | MsSetChrPosReset, MsSetChrVisibleAppear, op_et_bindeff2, MsBtlChrRegist, MsSetChrWeak |
| 5 | 976 | `0x00786b00` | `MsSetSaveStartGame` | Sg_FreeA, TkInitSaveWork, eiAbmDataInit, getExcelBinSize, MsWeaponNameNum |
| 5 | 1624 | `0x0079b4f0` | `MsSetRamChr` | FUN_0079ca80, MsSetRamChrParam, MsSetChrWeak, MsGetSaveConfigEnglish, MsSetCommandUse |
| 5 | 2073 | `0x007aa2a0` | `MsGetChrPredictPos` | MsCameraMoveCalc, TkMtCalcSpline, TkMtCalcSplineTangent, MsGetMoveTargetDist, MsSetBallSpline |
| 6 | 733 | `0x0078c740` | `MsDamageCheckDeath` | MsLimitTypeDeathCheck, MsNumberRegist, MsAutoRelifeProcess_007b2910, MsAutoPhenixProcess, MsGetItem, MsSetChrWeak |
| 6 | 1539 | `0x007bc110` | `MsScreenProcess` | MsCameraMoveCalc, Cam_Apply, setNearPlane, MsScreenMatrix, MsCameraMatrix, MsCameraShakeProcess |
| 6 | 2355 | `0x0079d430` | `MsBattleSpecial` | MsRotMatrixY, MsRotMatrixZ, FUN_0079df90, FUN_0079e030, yiSetRotTransDObj, MsRotMatrixX |
| 8 | 347 | `0x007824e0` | `MsBtlBridgeEvent2Btl` | AtelGetEventElemCharIDFromID, AtelSetEventActorLinkWork, AtelGetEventElemHandleFromID, AtelGetEventCharHandleFromID, FUN_0086b440, AtelGetEventBindIdFromID, AtelGetEventActorLinkWork, AtelGetEventCharIDFromID |
| 8 | 702 | `0x007938e0` | `MsBtlWeaponRegist` | ChN_DataDispose, MsChrCreatePacket, Ch_Allocate, MsSetChrPosReset, MsSetChrDirF, MsSetWeaponEffectAttribute, Ch_Dispose, FUN_00796f70 |
| 8 | 718 | `0x00792a90` | `MsBattleProcess_InputCue_00792a90` | MsBattleError, TOSetBtlActiveChr, MsCheatPlayerPowerup, FUN_00792120, MsSetBerserkCommand, MsSetProvokingCommand, MsSetDeathSentence, MsCheckStatusBeforeAction |
| 9 | 302 | `0x00782090` | `MsBattleReadDataNB` | MsImportantRead, MsBattleExe, MsWeaponNameRead, MsBtlReadSetScene, Kernel_LoadTables, eiAbmSetLang, FUN_00782670, FUN_00782b70, MsGetSaveConfigEnglish |
| 9 | 447 | `0x0079cdf0` | `MsMessageCueProcess` | MsRegSEplay, TOBtlDrawGetMoneyMessageWindow, TOBtlDrawLearningMessageWindow, TOBtlDrawStdChrNameMessageWindow, TOBtlDrawGetItemMessageWindow, TOBtlDrawFirstStrikePlayerMessageWindow, TOBtlDrawFirstStrikeEnemyMessageWindow, TOBtlDrawGetLimitTypeMessageWindow, TOBtlDrawCaptureMonsterMessageWindow |
| 10 | 1081 | `0x00788480` | `MsEffectStart` | MsEffectLimitCheck, FUN_00788f90, MsWeaponRead, FUN_00789030, MsMagicEffectEnd, FUN_00787650, SndIsReadStatus, FUN_00787d70, MsSetChrWeak, FUN_00788ef0 |
| 10 | 7007 | `0x007b4b70` | `MsSetChrStatInfo` | MsSetChrVisibleAppear, MsBlowProcess, MsSetSavePlyJoin, MsDamageCheckDeath, MsSetChrShadowMode, MsSetChrVisibleMain, MsAliveProcess, MsSetChrVisible, MsSetRamChrParam, MsSetChrWeak |
| 14 | 159 | `0x00781590` | `MsBattleFree` | MsBtlChrFreeMem, MsBtlChrFreeAll, MsBtlReadSceneFree, MsNumberFreeMem, TOClearBtlWindow, MsAtelRegistFree, MsEtEffectStop, FUN_00782640, MsEffectEnvFree, FUN_00782750, MsBattleSpecialFree, MsSEPbattleClear, MsBtlBridgeBtl2Event, MsBtlFreeItemMem |
| 14 | 746 | `0x007821f0` | `MsBtlBridgeBtl2Event` | MsChrCreatePacket, AtelGetEventCharHandleFromID, AtelSetEventElemPosFromID, AtelSetEventElemCharIDFromID, AtelGetEventActorLinkWork, AtelSetEventActorLinkWork, AtelGetHideStatusFromID, AtelGetEventElemStatus, AtelGetEventCharIDFromID, AtelSetEventElemStatus, AtelSetEventElemHandleFromID, AtelSetEventCharHandleFromID, AtelGetKeepBattlePartyFromID, AtelGetEventBindIdFromID |
| 15 | 1288 | `0x00781020` | `MsBattleExe` | SsSetProjectBase, MsBattleFree, MsEventEffectFree, MsScreenFocusResetAll, MsEtEffectSet, MsBtlReadManage, FUN_007829b0, MsNumberGetMem, MsBtlChrGetMem, MsBtlGetItemMem, AtelEventSetUpForBattleMusic, MsBattleExeSetIndex, MsBtlListCDpos, MsEtEffectEncount, TOInitBtlWindow |
| 15 | 1529 | `0x00783020` | `MsBtlReadManage` | yiFieldMapReadNB, MsBtlReadMonster, TOInitBattleFontPath, MsBtlReadScene, MsAtelSetUp, MsGetRamChrCTBbasic, MsAtelRequestSystemCheckExe, FUN_00784420, MsBtlReadSetScene, MsSetChrVisibleAppear, yiBattleMapReadNB, MsCalcChrDirAll, MsBattleExeSetIndex, MsSetChrVisibleEvent, Ch_CheckNbBattleChr |
| 15 | 1894 | `0x007917d0` | `MsBattleProcess_Status_007917d0` | AtelBGSepFade, MsCommandReturnChrStatusAll, FUN_00791780, FUN_00793290, MsAtelRequestSystemCheckExe, MsReturnSummon, MsEffectSepClearAll, MsEffectForceFree, MsLimitTypeWinCheck, MsMusicSync, SndIsReadStatus, MsDeathYoujinbou, MsEventEffectRead, MsEventEffectStart, MsMusicReadStart |
| 16 | 1004 | `0x007911e0` | `MsBattleProcess_ChrDamage_007911e0` | MsSetDamage, MsCommandReturnChrStatusAll, FUN_00791780, MsSetEscapeFlagAll, MsBtlChrFree, FUN_00791690, MsReturnSummon, MsScreenFocusResetAll, MsSetChrPosAfterSummon, MsPoisonProcess, MsMagicEffectEnd, MsSetEscapeFlag, MsEffectCameraEnd, FUN_00792ee0, MsSetChrWeak, MsEffectFree |
| 16 | 2436 | `0x007be110` | `MsCameraProcess` | FUN_007bcb80, MsCameraMoveCalc, FUN_007c0910, FUN_007c0a30, FUN_007bcd30, FUN_007c09b0, FUN_007bcab0, FUN_007c0d40, FUN_007c4d40, FUN_007c0650, MsSpCameraProcess, FUN_007c0ae0, FUN_007c0560, MsCamSetBtl, FUN_0079dd70, MsCameraShakeProcess |
| 18 | 1353 | `0x00781700` | `MsBattleInit` | MsWeaponNameRead, MsSetSaveStartGame, MsMemCheckSum, MsFieldItemRead, MsBtlReadSetScene, MsBattleExe, MsDebugFullCommand, MsImportantRead, MsDebugFullMember, Kernel_LoadTables, MsBattleInitAtEvent, MsGetSaveConfigEnglish, MsBattleSpecialInit, MsBtlGetInit, FUN_00782670, MsEtEffectInit, MsMenuReadText, FUN_00782b70 |
| 18 | 1585 | `0x0078f060` | `AfterDamageProcess` | MsSetStealEffect, MsNumberRegist, MsAutoCureProcess, MsSetStealGillEffect, MsStatusDefenseEffect, MsSubHP, MsSubMP, MsBlowProcess, MsLimitTypeStatusCheck, MsDamageCheckDeath, MsRegSEplay2, MsLimitTypeDamageCheck, MsAutoPotionProcess, MsStoneProcess, MsAutoRelifeProcess, MsSubCTB, MsAliveProcess, MsSetChrWeak |
| 23 | 913 | `0x00790c10` | `MsBattleProcess` | MsEffectStart, MsMoveChr, MsEventEffectProcess, MsBattleInit, MsBattleProcess_InputCue_00792a90, MsBattleFree, TOControlBtlWindow, TODebugGetPadPrintFlag, MsBtlReadManage, MsBattleExe, MsBattleProcess_Order_00790fb0, MsSEP3Dprocess, MsBattleAllFree, MsSetSaveData, MsSEPtransProcess, MsBattleProcess_Status_007917d0, TkCheckSoftWareReset, AtelBattleEndCallback, MsBattleProcess_ChrDamage_007911e0, MsAtelResetMoveCameraID, opd_exit_from_ovl, MsMessageCueProcess, MsEnvInit |

</details>
