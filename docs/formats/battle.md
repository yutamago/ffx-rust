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
`g_KernelSummonGrow` (`0x0112a948`, record `i + (5n − 9) · 2` for
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

## M2 remaining

Battle-module logic functions (`symbols/classes.tsv`, the module as `tools/progress.py` places it) not yet replaced by a port in the hook table, regenerated each round (`~/ffx-work/agent-battle/m2.py`). *Blockers* are the function's own logic callees still unported; a function with none can be ported now. The module placement of unnamed functions is a guess by neighbourhood, so a few non-battle functions (start-up screens, the password and screen-aspect helpers) are counted here.

**421 functions, 98734 bytes** (257 named). By blockers: 0: 250, 1: 102, 2: 41, 3: 7, 4: 8, 5: 2, 6: 2, 7: 1, 9: 1, 10: 1, 11: 1, 12: 1, 13: 2, 15: 1, 18: 1.

The battle loop's top level (`MsBattleProcess` runs each frame; `MsBattleInit` and `MsBattleExe` set a battle up; encounters and rewards):

| Function | Address | Bytes | Blockers |
|---|---|---|---|
| `MsBattleProcess` | `0x00790c10` | 913 | 10 |
| `MsBattleInit` | `0x00781700` | 1353 | 18 |
| `MsBattleExe` | `0x00781020` | 1288 | 13 |
| `MsBattleFree` | `0x00781590` | 159 | 11 |
| `MsBtlReadManage` | `0x00783020` | 1529 | 12 |
| `MsBattleAllFree` | `0x00780b40` | 293 | 5 |
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
| 0 | 12 | `0x007bd500` | `FUN_007bd500` |  |
| 0 | 13 | `0x007901f0` | `MsMenuSetWindowSystem` |  |
| 0 | 14 | `0x0079e7b0` | `MsEtEffectStop` |  |
| 0 | 14 | `0x007c0e60` | `setScreenDataAspectZ` |  |
| 0 | 15 | `0x00780d00` | `MsBattleDraw` |  |
| 0 | 15 | `0x00780fe0` | `MsBattleEncountFlag` |  |
| 0 | 15 | `0x007881b0` | `MsEffectReadError` |  |
| 0 | 15 | `0x00788b20` | `MsEffectWaitFlag` |  |
| 0 | 21 | `0x00796860` | `MsSetChrVisibleMain` |  |
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
| 0 | 29 | `0x0079e4f0` | `MsEffectSendSignal` |  |
| 0 | 29 | `0x0079f990` | `MsNumberFreeMem` |  |
| 0 | 29 | `0x007aaac0` | `FUN_007aaac0` |  |
| 0 | 30 | `0x007902d0` | `PasswordGetTotalIndex` |  |
| 0 | 32 | `0x007bbdd0` | `MsScreenLinkSet` |  |
| 0 | 33 | `0x007c1110` | `FUN_007c1110` |  |
| 0 | 34 | `0x007b7ef0` | `FUN_007b7ef0` |  |
| 0 | 35 | `0x007af2f0` | `MsCheckDefenseMotion` |  |
| 0 | 36 | `0x007b7e80` | `MsEffectChrMotionSync` |  |
| 0 | 37 | `0x00782940` | `FUN_00782940` |  |
| 0 | 39 | `0x00782670` | `FUN_00782670` |  |
| 0 | 40 | `0x00786aa0` | `FUN_00786aa0` |  |
| 0 | 40 | `0x007902a0` | `FUN_007902a0` |  |
| 0 | 40 | `0x007b1a70` | `MsRestartMagicCamera` |  |
| 0 | 41 | `0x00793500` | `MsBtlChrFreeMem` |  |
| 0 | 41 | `0x00796880` | `FUN_00796880` |  |
| 0 | 41 | `0x007968b0` | `FUN_007968b0` |  |
| 0 | 41 | `0x007a0c20` | `MsGearHasAbility_007a0c20` |  |
| 0 | 41 | `0x007a8c20` | `FUN_007a8c20` |  |
| 0 | 42 | `0x007859d0` | `MsSaveAddHP` |  |
| 0 | 42 | `0x00785ae0` | `FUN_00785ae0` |  |
| 0 | 42 | `0x007a8c50` | `FUN_007a8c50` |  |
| 0 | 42 | `0x007ad6e0` | `FUN_007ad6e0` |  |
| 0 | 43 | `0x00798a90` | `FUN_00798a90` |  |
| 0 | 43 | `0x007ad5b0` | `FUN_007ad5b0` |  |
| 0 | 43 | `0x007bc0e0` | `MsScreenOffsetID` |  |
| 0 | 44 | `0x00782640` | `FUN_00782640` |  |
| 0 | 44 | `0x007abb70` | `MsGetSaveChrArmorModel` |  |
| 0 | 44 | `0x007abba0` | `MsGetSaveChrWeaponModel` |  |
| 0 | 44 | `0x007bcf20` | `MsCameraGetYaw` |  |
| 0 | 45 | `0x007c11c0` | `FUN_007c11c0` |  |
| 0 | 46 | `0x00781650` | `FUN_00781650` |  |
| 0 | 47 | `0x00795da0` | `FUN_00795da0` |  |
| 0 | 47 | `0x008741d0` | `FUN_008741d0` |  |
| 0 | 48 | `0x00793d90` | `MsChrCreatePacket` |  |
| 0 | 49 | `0x00784220` | `MsGetRamChrCTBbasic` |  |
| 0 | 49 | `0x00794e80` | `MsGetChrEffectVariable` |  |
| 0 | 49 | `0x007950b0` | `MsGetChrHeadPos` |  |
| 0 | 49 | `0x007b1e40` | `FUN_007b1e40` |  |
| 0 | 50 | `0x00794240` | `FUN_00794240` |  |
| 0 | 50 | `0x00794280` | `FUN_00794280` |  |
| 0 | 50 | `0x00794d70` | `FUN_00794d70` |  |
| 0 | 50 | `0x00794f70` | `MsGetChrFly` |  |
| 0 | 50 | `0x00795a60` | `FUN_00795a60` |  |
| 0 | 51 | `0x007a0170` | `MsRegSEplay3` |  |
| 0 | 52 | `0x007cf000` | `FUN_007cf000` |  |
| 0 | 53 | `0x007877d0` | `MsEffectCameraGet` |  |
| 0 | 53 | `0x007993e0` | `FUN_007993e0` |  |
| 0 | 54 | `0x00795e70` | `MsSetChrDirLeave` |  |
| 0 | 55 | `0x007907a0` | `FUN_007907a0` |  |
| 0 | 55 | `0x0079ea90` | `MsResetWeaponEffectAttribute` |  |
| 0 | 55 | `0x007c5680` | `MsScreenCheckShake` |  |
| 0 | 56 | `0x00788b90` | `MsEventEffectFree` |  |
| 0 | 56 | `0x007a88e0` | `FUN_007a88e0` |  |
| 0 | 57 | `0x00798f90` | `MsFieldItemFree` |  |
| 0 | 57 | `0x00799f70` | `MsBtlFreeItemMem` |  |
| 0 | 58 | `0x007844c0` | `readBtlMonsterName` |  |
| 0 | 58 | `0x00793dc0` | `FUN_00793dc0` |  |
| 0 | 58 | `0x007c4f10` | `MsCameraCheckShake` |  |
| 0 | 59 | `0x007881c0` | `MsEffectReadStatInit` |  |
| 0 | 59 | `0x0079f9b0` | `MsNumberGetMem` |  |
| 0 | 59 | `0x007ad710` | `FUN_007ad710` |  |
| 0 | 60 | `0x00793530` | `MsBtlChrGetMem` |  |
| 0 | 61 | `0x00795d40` | `FUN_00795d40` |  |
| 0 | 61 | `0x0079a080` | `FUN_0079a080` |  |
| 0 | 62 | `0x00793cb0` | `MsCalcChrDirAll` |  |
| 0 | 62 | `0x007a8be0` | `FUN_007a8be0` |  |
| 0 | 63 | `0x00798bd0` | `FUN_00798bd0` |  |
| 0 | 64 | `0x007987b0` | `FUN_007987b0` |  |
| 0 | 65 | `0x00776f00` | `FUN_00776f00` |  |
| 0 | 65 | `0x00792a40` | `MsResetBtlTarget` |  |
| 0 | 66 | `0x00794db0` | `FUN_00794db0` |  |
| 0 | 67 | `0x00795ff0` | `FUN_00795ff0` |  |
| 0 | 67 | `0x0079ea40` | `MsPauseChrEffect` |  |
| 0 | 68 | `0x00795dd0` | `FUN_00795dd0` |  |
| 0 | 68 | `0x0079f1c0` | `FUN_0079f1c0` |  |
| 0 | 69 | `0x007902f0` | `PasswordTextDispose` |  |
| 0 | 69 | `0x00798950` | `Rng_ClockSeed` |  |
| 0 | 69 | `0x0079d2a0` | `FUN_0079d2a0` |  |
| 0 | 70 | `0x00799f20` | `MsMemCheckSum` |  |
| 0 | 70 | `0x007c13c0` | `FUN_007c13c0` |  |
| 0 | 71 | `0x007bed10` | `MsCameraResetFilter` |  |
| 0 | 72 | `0x007c1000` | `FUN_007c1000` |  |
| 0 | 73 | `0x00796430` | `FUN_00796430` |  |
| 0 | 73 | `0x007bdf90` | `MsCameraMoveStat` |  |
| 0 | 74 | `0x0079df90` | `FUN_0079df90` |  |
| 0 | 75 | `0x007bd4b0` | `FUN_007bd4b0` |  |
| 0 | 76 | `0x00797a90` | `MsAtelRegistFree` |  |
| 0 | 76 | `0x00798840` | `FUN_00798840` |  |
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
| 0 | 87 | `0x007ab2a0` | `MsSetBtlPosReset` |  |
| 0 | 88 | `0x0078d620` | `MsResetAvoidFlag` |  |
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
| 0 | 98 | `0x00790430` | `FUN_00790430` |  |
| 0 | 98 | `0x007b19f0` | `FUN_007b19f0` |  |
| 0 | 103 | `0x007bdde0` | `MsCameraMoveDivide` |  |
| 0 | 105 | `0x007967f0` | `MsSetChrVisibleEvent` |  |
| 0 | 105 | `0x0079a440` | `MsGetChrID` |  |
| 0 | 105 | `0x007b7c80` | `MsSetBallSplineInit` |  |
| 0 | 109 | `0x007abca0` | `Equip_RebuildWearers` |  |
| 0 | 111 | `0x007a06b0` | `MsSEPtransRegist` |  |
| 0 | 112 | `0x00787c30` | `MsEffectMagicMotion` |  |
| 0 | 113 | `0x007c0a30` | `FUN_007c0a30` |  |
| 0 | 116 | `0x007a0820` | `MsSoundStreamPlay` |  |
| 0 | 117 | `0x0079b470` | `MsSetCommandUse` |  |
| 0 | 118 | `0x007ac940` | `FUN_007ac940` |  |
| 0 | 119 | `0x007a9a40` | `MsGetTargetDist` |  |
| 0 | 120 | `0x007bec90` | `MsCameraResetDirVector` |  |
| 0 | 124 | `0x0079ca80` | `FUN_0079ca80` |  |
| 0 | 124 | `0x0079dde0` | `MsBattleSpecialFree` |  |
| 0 | 124 | `0x007c09b0` | `FUN_007c09b0` |  |
| 0 | 126 | `0x00796bb0` | `FUN_00796bb0` |  |
| 0 | 132 | `0x0079ccf0` | `MsRegMotionRead` |  |
| 0 | 132 | `0x007abeb0` | `FUN_007abeb0` |  |
| 0 | 134 | `0x00874130` | `FUN_00874130` |  |
| 0 | 135 | `0x007a9fc0` | `MsSetMove` |  |
| 0 | 135 | `0x007cf860` | `FUN_007cf860` |  |
| 0 | 140 | `0x007850b0` | `MsGetSaveCommandMenuPanel` |  |
| 0 | 140 | `0x00799790` | `MsGetCommandTarget2` |  |
| 0 | 140 | `0x007a0720` | `MsSetVoice` |  |
| 0 | 140 | `0x007bf1f0` | `FUN_007bf1f0` |  |
| 0 | 141 | `0x0079d0e0` | `MsBtlListCDpos` |  |
| 0 | 145 | `0x007bc750` | `FUN_007bc750` |  |
| 0 | 146 | `0x007bfe70` | `MsCameraSetSplineFilter` |  |
| 0 | 147 | `0x007ab010` | `MsMoveSpline` |  |
| 0 | 147 | `0x007baac0` | `MsSplineRegist` |  |
| 0 | 148 | `0x00784420` | `FUN_00784420` |  |
| 0 | 149 | `0x007c0910` | `FUN_007c0910` |  |
| 0 | 150 | `0x007828a0` | `Enc_ResetWeights` |  |
| 0 | 155 | `0x00799a10` | `MsGetSameTarget` |  |
| 0 | 157 | `0x007bce80` | `FUN_007bce80` |  |
| 0 | 159 | `0x007b6f10` | `FUN_007b6f10` |  |
| 0 | 160 | `0x007bc890` | `FUN_007bc890` |  |
| 0 | 162 | `0x00799020` | `MsFieldItemRead` |  |
| 0 | 163 | `0x007bf280` | `MsCameraSetFocus` |  |
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
| 0 | 208 | `0x007a8720` | `MsGetBasicDivide` |  |
| 0 | 210 | `0x007a9150` | `FUN_007a9150` |  |
| 0 | 227 | `0x007c0560` | `FUN_007c0560` |  |
| 0 | 230 | `0x007bf0b0` | `FUN_007bf0b0` |  |
| 0 | 233 | `0x007a87f0` | `MsGetDirDivide` |  |
| 0 | 240 | `0x0078cd30` | `MsGetComInfo` |  |
| 0 | 245 | `0x007bff10` | `MsCameraSetStartPos` |  |
| 0 | 249 | `0x0079a760` | `MsGetDamageInfoNext` |  |
| 0 | 249 | `0x008be0d0` | `FUN_008be0d0` |  |
| 0 | 250 | `0x007ac840` | `FUN_007ac840` |  |
| 0 | 254 | `0x007b2c90` | `FUN_007b2c90` |  |
| 0 | 259 | `0x007b7170` | `MsGetTargetRandomList` |  |
| 0 | 262 | `0x00793f10` | `MsDetachWeapon` |  |
| 0 | 264 | `0x00782c00` | `MsSetCountryName` |  |
| 0 | 268 | `0x007848b0` | `MsDebugFullCommand` |  |
| 0 | 271 | `0x007bbeb0` | `MsScreenMoveAcc` |  |
| 0 | 280 | `0x0078ac20` | `MsCalcMotionSpeed` |  |
| 0 | 282 | `0x007ab750` | `FUN_007ab750` |  |
| 0 | 294 | `0x007c57b0` | `MsScreenShake2` |  |
| 0 | 296 | `0x007c4fc0` | `MsCameraShake` |  |
| 0 | 297 | `0x007842f0` | `FUN_007842f0` |  |
| 0 | 306 | `0x00784d00` | `MsExchangeSaveParty` |  |
| 0 | 325 | `0x007acb40` | `FUN_007acb40` |  |
| 0 | 332 | `0x00782df0` | `getExcelBinSize` |  |
| 0 | 332 | `0x007b055a` | `FUN_007b055a` |  |
| 0 | 336 | `0x00788920` | `MsEffectTerminate` |  |
| 0 | 364 | `0x0079fe80` | `FUN_0079fe80` |  |
| 0 | 379 | `0x0079aff0` | `MsMergeDamage` |  |
| 0 | 380 | `0x007ab9f0` | `SwapSaveGear` |  |
| 0 | 429 | `0x007beae0` | `MsCameraReset` |  |
| 0 | 453 | `0x007c4d40` | `FUN_007c4d40` |  |
| 0 | 549 | `0x007c0ae0` | `FUN_007c0ae0` |  |
| 0 | 783 | `0x007c3170` | `MsRotMatrixX` |  |
| 0 | 783 | `0x007c3480` | `MsRotMatrixY` |  |
| 0 | 783 | `0x007c3790` | `MsRotMatrixZ` |  |
| 0 | 806 | `0x0079f660` | `MsNumberDrawProcess` |  |
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
| 1 | 34 | `0x00783b90` | `MsBtlReadPlayerModelProcess` | MsBtlChrRegist |
| 1 | 36 | `0x007b7650` | `FUN_007b7650` | MsResetAvoidFlag |
| 1 | 38 | `0x00786950` | `MsSetSavePartyMember` | MsExchangeSaveParty |
| 1 | 38 | `0x007873a0` | `resetDefaultPlayerNameStatus` | setDefaultPlayerName |
| 1 | 38 | `0x0079e670` | `MsEtEffectInit` | FUN_0079f1c0 |
| 1 | 39 | `0x00782b70` | `FUN_00782b70` | MsMenuReadText |
| 1 | 40 | `0x0079cb10` | `MsSetRamMonData` | MsSetRamChr |
| 1 | 41 | `0x0079eae0` | `FUN_0079eae0` | MsPauseChrEffect |
| 1 | 42 | `0x007993b0` | `MsSetWeaponName` | MsWeaponNameNum |
| 1 | 43 | `0x00782b40` | `FUN_00782b40` | Enc_ResetWeights |
| 1 | 44 | `0x0079fcf0` | `FUN_0079fcf0` | FUN_0079fe80 |
| 1 | 45 | `0x008be0a0` | `MsExchangeSaveWeapon` | SwapSaveGear |
| 1 | 50 | `0x00781c50` | `MsBattleInitAtEvent` | Enc_ResetWeights |
| 1 | 51 | `0x00782050` | `MsBattleReadData` | MsBattleReadDataNB |
| 1 | 51 | `0x0079fdf0` | `FUN_0079fdf0` | FUN_0079fd40 |
| 1 | 56 | `0x00781550` | `MsBattleExeSetIndex` | MsGetFileSize |
| 1 | 56 | `0x0079eba0` | `FUN_0079eba0` | MsSetChrEffect_0079eb10 |
| 1 | 58 | `0x00798720` | `FUN_00798720` | MsBindCamera |
| 1 | 66 | `0x00788430` | `MsEffectSetSpeed` | MsCalcMotionSpeed |
| 1 | 66 | `0x0079e590` | `MsEtCriticalEffect` | op_et_eff |
| 1 | 67 | `0x0079dfe0` | `MsSpCameraProcess` | MsTransSpPos |
| 1 | 69 | `0x0079fda0` | `FUN_0079fda0` | FUN_0079fe80 |
| 1 | 73 | `0x0079d090` | `FUN_0079d090` | MsGetBattleMessageFlag |
| 1 | 75 | `0x0079fe30` | `FUN_0079fe30` | FUN_0079fd40 |
| 1 | 78 | `0x0079e5e0` | `MsEtEffectEncount` | op_et_encount_effect_on |
| 1 | 79 | `0x007baa50` | `FUN_007baa50` | MsAtelGetCameraID |
| 1 | 83 | `0x0079ec40` | `FUN_0079ec40` | op_et_eff |
| 1 | 83 | `0x0079eca0` | `FUN_0079eca0` | op_et_eff |
| 1 | 83 | `0x0079fd40` | `FUN_0079fd40` | FUN_0079fe80 |
| 1 | 89 | `0x00783f60` | `MsBtlReadWeapon` | MsCDreadWeapon |
| 1 | 95 | `0x00782ba0` | `FUN_00782ba0` | Enc_ResetWeights |
| 1 | 98 | `0x0079e2a0` | `FUN_0079e2a0` | yiSetRotTransDObj |
| 1 | 108 | `0x007b7d00` | `MsEffectChrGetLoveLoveMax` | MsGetRamChrFemale |
| 1 | 110 | `0x007a2600` | `MsWeaponNameRead` | getExcelBinSize |
| 1 | 112 | `0x007908c0` | `MsImportantRead` | getExcelBinSize |
| 1 | 114 | `0x0079e510` | `MsEffectSetGuard` | FUN_0079ec40 |
| 1 | 114 | `0x007bb840` | `MsAtelScreenShake` | MsScreenShake |
| 1 | 123 | `0x007932b0` | `MesSetChrTargetPosToLimitSparePosy` | FUN_00796bb0 |
| 1 | 124 | `0x00793480` | `MsBtlChrFreeAll` | Ch_Dispose |
| 1 | 127 | `0x007cf8f0` | `ms_free` | FUN_007cf860 |
| 1 | 128 | `0x007c0650` | `FUN_007c0650` | FUN_0086a6e0 |
| 1 | 130 | `0x007b0470` | `MsMenuLimitSet_007b0470` | MsMenuLimitSet |
| 1 | 131 | `0x00784260` | `Mon_GetName` | readBtlMonsterName |
| 1 | 131 | `0x007abf40` | `FUN_007abf40` | FUN_007abeb0 |
| 1 | 132 | `0x00783b00` | `Mon_AttachData` | MsRegistSEdata |
| 1 | 132 | `0x007b7440` | `FUN_007b7440` | FUN_007b7440 |
| 1 | 134 | `0x0079e6a0` | `MsEtEffectRead` | op_et_battle_init |
| 1 | 135 | `0x0079eb10` | `MsSetChrEffect_0079eb10` | op_et_eff |
| 1 | 143 | `0x007962d0` | `MsSetChrShadowMode` | MsSetChrShadowMode |
| 1 | 150 | `0x007ad640` | `MsChangeWeaponWakka` | MsFieldItemGetMes |
| 1 | 155 | `0x007a8b40` | `FUN_007a8b40` | MsGetCommandTarget2 |
| 1 | 158 | `0x007acc90` | `MsActReCommand` | FUN_007b19d0 |
| 1 | 165 | `0x007b7d70` | `MsEffectChrMotionIn` | MsSetWeaponEffectAttribute |
| 1 | 168 | `0x00796f70` | `FUN_00796f70` | Ch_EngineSetFast |
| 1 | 173 | `0x0078f700` | `MsCheckLeftWindow` | MsGetCommandUse |
| 1 | 187 | `0x00782a20` | `MsGetFileSize` | MsGetSaveConfigEnglish |
| 1 | 189 | `0x0078fcf0` | `MsMenuGetText` | MsGetSaveConfigEnglish |
| 1 | 189 | `0x007b1910` | `FUN_007b1910` | FUN_007b19d0 |
| 1 | 198 | `0x0079ed40` | `MsSetWeaponEffectAttribute` | op_et_eff |
| 1 | 207 | `0x0079a5b0` | `MsGetCommandUse` | FUN_0079a080 |
| 1 | 210 | `0x007b6fb0` | `MsGetTargetLinkList` | FUN_007b7440 |
| 1 | 213 | `0x007bfd90` | `MsCameraSetSpline2` | FUN_007bcd30 |
| 1 | 222 | `0x00790340` | `PasswordTextRead` | FUN_00790430 |
| 1 | 222 | `0x007b77c0` | `FUN_007b77c0` | MsSetDamageMotion |
| 1 | 225 | `0x007bf330` | `FUN_007bf330` | FUN_007c0910 |
| 1 | 226 | `0x007befc0` | `FUN_007befc0` | FUN_007c0650 |
| 1 | 231 | `0x007869b0` | `MsSetSavePlyJoin` | MsChangeWeaponInvisible |
| 1 | 234 | `0x007bfca0` | `MsCameraSetSpline` | FUN_007bcb80 |
| 1 | 241 | `0x00784120` | `MsCDreadWeapon` | FUN_00784000 |
| 1 | 245 | `0x0078fbb0` | `MsMenuGetHelp` | MsGetSaveConfigEnglish |
| 1 | 267 | `0x007beeb0` | `FUN_007beeb0` | FUN_007c0560 |
| 1 | 269 | `0x00796550` | `MsSetChrTransparentVisible` | MsSetChrTransparentVisible |
| 1 | 290 | `0x0078f7c0` | `MsGetMenuChrInfo` | MsGetSaveConfigEnglish |
| 1 | 299 | `0x007882c0` | `MsEffectSetMotion` | MsEffectSetSpeed |
| 1 | 314 | `0x007ac9c0` | `Battle_QueueCommand` | FUN_00796430 |
| 1 | 314 | `0x007bde50` | `MsCameraMoveFrame` | MsCameraSetStartPos |
| 1 | 337 | `0x0079e8c0` | `MsMagicEffectStart` | op_et_eff |
| 1 | 343 | `0x007c5c60` | `MsCameraSetBtl` | MsCamSetBtl |
| 1 | 374 | `0x007b72c0` | `MsGetTargetSortList` | FUN_007b6f10 |
| 1 | 391 | `0x007a8fc0` | `Battle_ScriptPlayMotion` | MsCalcMotionSpeed |
| 1 | 439 | `0x007a01b0` | `MsRegistSEdata` | MsSEPtransRegist |
| 1 | 504 | `0x007b1710` | `FUN_007b1710` | FUN_007b1e40 |
| 1 | 542 | `0x007bd860` | `MsCameraMoveAcc` | MsCameraSetStartPos |
| 1 | 566 | `0x0079a200` | `MsGetCTBlist` | FUN_0079a020 |
| 1 | 599 | `0x0079f380` | `MsCalcCursorPos` | graphicProject3DPointOntoScreenConsider2DScene |
| 1 | 728 | `0x007990d0` | `MsGetItem` | BattleRewards_AddGear |
| 1 | 880 | `0x007c0120` | `FUN_007c0120` | spline_get_near_point |
| 1 | 1393 | `0x007afe00` | `MsMenuLimitSet` | MsGetTargetRandomList |
| 1 | 2555 | `0x007a1c00` | `MsWeaponNameNum_Armor_007a1c00` | MsGearHasAbility_007a0c20 |
| 1 | 3811 | `0x007a0d10` | `MsWeaponNameNum_Weapon_007a0d10` | MsGearHasAbility_007a0c20 |
| 2 | 20 | `0x00780cc0` | `MsBattleCursorCalc` | MsNumberDrawProcess, MsCalcCursorPos |
| 2 | 23 | `0x00783990` | `FUN_00783990` | Mon_AttachData, MsBtlReadMonster |
| 2 | 30 | `0x00796680` | `FUN_00796680` | FUN_00796880, FUN_007968b0 |
| 2 | 30 | `0x007a0cf0` | `MsWeaponNameNum` | MsWeaponNameNum_Armor_007a1c00, MsWeaponNameNum_Weapon_007a0d10 |
| 2 | 38 | `0x00798ff0` | `MsFieldItemGetMes` | FUN_007993e0, msFieldItemGet |
| 2 | 49 | `0x00783fc0` | `FUN_00783fc0` | MsBtlReadWeapon, FUN_00784000 |
| 2 | 72 | `0x00788d10` | `MsEventEffectReadComplete` | op_ovl_effect_init, MagicFile_GetEffectOverlayTable |
| 2 | 83 | `0x007a8f60` | `FUN_007a8f60` | MsGetSameTarget, FUN_00796430 |
| 2 | 100 | `0x007972e0` | `FUN_007972e0` | MsMotionProcess, MsBattleSpecial |
| 2 | 103 | `0x00784000` | `FUN_00784000` | MsRegistSEdata, MsEffectWeaponInit |
| 2 | 123 | `0x007cf970` | `FUN_007cf970` | FUN_007cf860, debug_exit_trace |
| 2 | 133 | `0x00782d60` | `MsWalkAbilityProcess` | MsSaveAddHP, FUN_00785ae0 |
| 2 | 139 | `0x007b75c0` | `FUN_007b75c0` | FUN_0079fd40, FUN_00793dc0 |
| 2 | 147 | `0x007bad10` | `FUN_007bad10` | MsCameraSetBtl, MsAtelGetCameraID |
| 2 | 147 | `0x007bcd30` | `FUN_007bcd30` | TkMtCalcSpline, TkMtCalcSpline3 |
| 2 | 150 | `0x007ba5b0` | `FUN_007ba5b0` | MsAtelGetCameraID, MsCameraMoveAcc |
| 2 | 152 | `0x007bb210` | `FUN_007bb210` | MsAtelGetCameraID, FUN_007bce80 |
| 2 | 155 | `0x007bb370` | `MsAtelCameraRefAddCircleArea_007bb370` | MsAtelGetCameraID, FUN_007bf1f0 |
| 2 | 166 | `0x00784070` | `Mon_LoadAll` | FUN_007842f0, MsSetRamMonData |
| 2 | 169 | `0x007bab90` | `FUN_007bab90` | MsAtelGetCameraID, FUN_007beeb0 |
| 2 | 169 | `0x007bb520` | `FUN_007bb520` | MsAtelGetCameraID, MsCameraMoveDivide |
| 2 | 175 | `0x007bb8c0` | `MsAtelCameraShake` | MsAtelGetCameraID, MsCameraShake |
| 2 | 189 | `0x007bb2b0` | `FUN_007bb2b0` | MsAtelGetCameraID, FUN_007bf0b0 |
| 2 | 193 | `0x007bac40` | `FUN_007bac40` | FUN_007befc0, MsAtelGetCameraID |
| 2 | 211 | `0x00796c30` | `getSparedLmtPos` | graphicProject3DPointOntoScreenConsider2DScene, FUN_00796bb0 |
| 2 | 215 | `0x007bb970` | `FUN_007bb970` | MsAtelGetCameraID, MsCameraSetSpline |
| 2 | 240 | `0x0079e1b0` | `MsTransSpPos` | MsRotMatrixZ, MsRotMatrixX |
| 2 | 249 | `0x007aa160` | `MsMotionProcess` | MsCalcMotionSpeed, FUN_007ab750 |
| 2 | 249 | `0x007bf420` | `FUN_007bf420` | FUN_007c0a30, FUN_007c09b0 |
| 2 | 270 | `0x007a89c0` | `MsSetChrVisibleBtl2Event` | FUN_007fae90, MsSetRamChr |
| 2 | 275 | `0x007c0d40` | `FUN_007c0d40` | spline_get_near_point, spline_get_near_point_init |
| 2 | 302 | `0x0079de60` | `MsBattleSpecialInit` | Yn_SeaCreate, Yn_PSimTest |
| 2 | 308 | `0x007839b0` | `MsBtlReadMonsterProcess` | MsBtlChrRegist, MsSetChrTransparentVisible |
| 2 | 408 | `0x007bb6a0` | `setCameraMatrixToPhyreScene` | MsAtelGetCameraID, FUN_0079e2a0 |
| 2 | 412 | `0x00799420` | `msFieldItemGet` | FUN_00798bd0, MsWeaponNameNum |
| 2 | 437 | `0x00783bc0` | `MsBtlReadPlayerProcess` | op_et_bindeff2, MsBtlChrRegist |
| 2 | 623 | `0x007976a0` | `MsAtelChrInit` | FUN_00798720, FUN_007987b0 |
| 2 | 646 | `0x0078fef0` | `MsMenuReadText` | MsGetFileSize, getExcelBinSize |
| 2 | 732 | `0x007b1b50` | `MsSetReelResult` | FUN_007b19d0, FUN_007b1710 |
| 2 | 1109 | `0x007badb0` | `FUN_007badb0` | MsCameraSetBtl, MsAtelGetCameraID |
| 2 | 1234 | `0x0079c110` | `MsSetRamChrData` | MsGetChrID, MsSetRamChr |
| 3 | 206 | `0x007bb5d0` | `FUN_007bb5d0` | Cam_SetPolar, MsAtelGetCameraID, Cam_AddPolar |
| 3 | 271 | `0x007bb410` | `MsAtelCameraEyeAddArea_007bb410` | MsAtelGetCameraID, FUN_007bf420, FUN_007bf330 |
| 3 | 280 | `0x007b7680` | `FUN_007b7680` | FUN_0079fd40, FUN_0079eba0, MsEtCriticalEffect |
| 3 | 285 | `0x00787e00` | `MsEffectProcess` | MagicFile_GetEffectOverlayTable, op_effect_run_after, op_effect_run_before |
| 3 | 374 | `0x0079e030` | `FUN_0079e030` | MsRotMatrixY, MsRotMatrixZ, MsRotMatrixX |
| 3 | 615 | `0x00793670` | `MsBtlChrRegist` | Ch_Allocate, MsChrCreatePacket, FUN_00796f70 |
| 3 | 630 | `0x00798c10` | `BattleRewards_AddGear` | FUN_00798bd0, MsWeaponNameNum, FUN_00798a90 |
| 4 | 167 | `0x007b74d0` | `FUN_007b74d0` | FUN_007b77c0, FUN_007b75c0, FUN_007b7680, FUN_007b7650 |
| 4 | 315 | `0x00783e20` | `MsBtlReadSetScene` | MsBtlBridgeEvent2Btl, AtelEventRemoveForBattle, MsSetRamChrData, MsGetSaveConfigEnglish |
| 4 | 766 | `0x00795250` | `MsGetChrLimitBarFlag` | graphicProject3DPointOntoScreenConsider2DScene, FUN_00796d10, MesSetChrTargetPosToLimitSparePosy, getSparedLmtPos |
| 4 | 774 | `0x00783680` | `MsBtlReadMonster` | Mon_AttachData, readBtlMonsterName, MsGetChrID, Mon_LoadAll |
| 4 | 976 | `0x00786b00` | `MsSetSaveStartGame` | TkInitSaveWork, eiAbmDataInit, getExcelBinSize, MsWeaponNameNum |
| 4 | 1539 | `0x007bc110` | `MsScreenProcess` | Cam_Apply, MsScreenMatrix, MsCameraMatrix, MsCameraShakeProcess |
| 4 | 1624 | `0x0079b4f0` | `MsSetRamChr` | FUN_0079ca80, MsSetRamChrParam, MsGetSaveConfigEnglish, MsSetCommandUse |
| 4 | 7007 | `0x007b4b70` | `MsSetChrStatInfo` | MsSetSavePlyJoin, MsSetChrShadowMode, MsSetChrVisibleMain, MsSetRamChrParam |
| 5 | 293 | `0x00780b40` | `MsBattleAllFree` | FUN_00782640, FUN_00799ec0, MsMenuFreeText, MsFieldItemFree, MsBattleFree |
| 5 | 406 | `0x00787470` | `setDefaultPlayerName` | FUN_00875220, debugSetRikkuName, FUN_008c5370, FUN_00875370, MsMenuGetText |
| 6 | 702 | `0x007938e0` | `MsBtlWeaponRegist` | ChN_DataDispose, MsChrCreatePacket, Ch_Allocate, MsSetWeaponEffectAttribute, Ch_Dispose, FUN_00796f70 |
| 6 | 2355 | `0x0079d430` | `MsBattleSpecial` | MsRotMatrixY, MsRotMatrixZ, FUN_0079df90, FUN_0079e030, yiSetRotTransDObj, MsRotMatrixX |
| 7 | 347 | `0x007824e0` | `MsBtlBridgeEvent2Btl` | AtelGetEventElemCharIDFromID, AtelSetEventActorLinkWork, AtelGetEventElemHandleFromID, FUN_0086b440, AtelGetEventBindIdFromID, AtelGetEventActorLinkWork, AtelGetEventCharIDFromID |
| 9 | 302 | `0x00782090` | `MsBattleReadDataNB` | MsImportantRead, MsBattleExe, MsWeaponNameRead, MsBtlReadSetScene, Kernel_LoadTables, eiAbmSetLang, FUN_00782670, FUN_00782b70, MsGetSaveConfigEnglish |
| 10 | 913 | `0x00790c10` | `MsBattleProcess` | MsBattleInit, MsBattleFree, TOControlBtlWindow, TODebugGetPadPrintFlag, MsBtlReadManage, MsBattleExe, MsBattleAllFree, TkCheckSoftWareReset, AtelBattleEndCallback, opd_exit_from_ovl |
| 11 | 159 | `0x00781590` | `MsBattleFree` | MsBtlChrFreeMem, MsBtlChrFreeAll, MsNumberFreeMem, MsAtelRegistFree, MsEtEffectStop, FUN_00782640, MsEffectEnvFree, FUN_00782750, MsBattleSpecialFree, MsBtlBridgeBtl2Event, MsBtlFreeItemMem |
| 12 | 1529 | `0x00783020` | `MsBtlReadManage` | yiFieldMapReadNB, MsBtlReadMonster, TOInitBattleFontPath, MsBtlReadScene, MsGetRamChrCTBbasic, FUN_00784420, MsBtlReadSetScene, yiBattleMapReadNB, MsCalcChrDirAll, MsBattleExeSetIndex, MsSetChrVisibleEvent, Ch_CheckNbBattleChr |
| 13 | 746 | `0x007821f0` | `MsBtlBridgeBtl2Event` | MsChrCreatePacket, AtelSetEventElemPosFromID, AtelSetEventElemCharIDFromID, AtelGetEventActorLinkWork, AtelSetEventActorLinkWork, AtelGetHideStatusFromID, AtelGetEventElemStatus, AtelGetEventCharIDFromID, AtelSetEventElemStatus, AtelSetEventElemHandleFromID, AtelSetEventCharHandleFromID, AtelGetKeepBattlePartyFromID, AtelGetEventBindIdFromID |
| 13 | 1288 | `0x00781020` | `MsBattleExe` | MsBattleFree, MsEventEffectFree, MsEtEffectSet, MsBtlReadManage, FUN_007829b0, MsNumberGetMem, MsBtlChrGetMem, MsBtlGetItemMem, AtelEventSetUpForBattleMusic, MsBattleExeSetIndex, MsBtlListCDpos, MsEtEffectEncount, TOInitBtlWindow |
| 15 | 2436 | `0x007be110` | `MsCameraProcess` | FUN_007bcb80, FUN_007c0910, FUN_007c0a30, FUN_007bcd30, FUN_007c09b0, FUN_007bcab0, FUN_007c0d40, FUN_007c4d40, FUN_007c0650, MsSpCameraProcess, FUN_007c0ae0, FUN_007c0560, MsCamSetBtl, FUN_0079dd70, MsCameraShakeProcess |
| 18 | 1353 | `0x00781700` | `MsBattleInit` | MsWeaponNameRead, MsSetSaveStartGame, MsMemCheckSum, MsFieldItemRead, MsBtlReadSetScene, MsBattleExe, MsDebugFullCommand, MsImportantRead, MsDebugFullMember, Kernel_LoadTables, MsBattleInitAtEvent, MsGetSaveConfigEnglish, MsBattleSpecialInit, MsBtlGetInit, FUN_00782670, MsEtEffectInit, MsMenuReadText, FUN_00782b70 |

</details>
