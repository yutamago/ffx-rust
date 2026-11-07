# Monster files

`ffx_ps2/ffx/master/jppc/battle/mon/_mNNN/mNNN.bin`: one file per monster
(361), shared by all languages. File *NNN* is monster ID *NNN*, the same ID
as in the kernel `monster1`–`3` tables.

Reader/writer: [`crates/ffx-monster`](../../crates/ffx-monster).

## Loading

`MsBtlReadMonster` (`0x00783680`) copies the monster's name and texts from the
kernel `monster` tables into the battle actor, then requests the file as
`mon_data` through the PS2 CD-library emulation (`libmscd.c`,
`0x0076b970`), which opens it with `sceOpen` → `fiosOpen` →
`Phyre_PSerialization_PStreamFileWin32_openFile`, so mods can replace it. When it arrives, `Mon_AttachData`
(`0x00783b00`) points the actor at its sections and `MsSetRamChr`
(`0x0079b4f0`) builds the enemy from them.

A file is read once per battle: for a slot whose monster an earlier slot
already has, `MsBtlReadMonster` takes that slot's buffer (`+0x48`; it also
clears the actor's `+0x11` and sets `+0x14` to `0x14` plus the earlier
slot) instead of reading the file again, so both actors' `+0xf78` point at
the same script block. `Mon_LoadAll` (`0x00784070`) registers each slot's
block with `MsAtelRegist(1, …)` (`0x00797a40`), a shared one once per slot,
and `AtelBattleSetUp` sets up a set of works for each registration. Two
monsters of one kind thus share the block's code, shared data (storage 4)
and event data (storage 6), while each has its own works, levels,
registers and private data (storage 3, work `+0xb8`;
[script.md](script.md#variables)).

## Container

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Section count, 8 in every file |
| `0x04` | `u32` × 8 | Section offsets; 0 = absent |

Sections are in ascending order; each runs to the next one. Section 3 is
absent and section 7 empty in every file.

| Section | Size | Use |
|---|---|---|
| 0 | 1–70 KB | Battle AI script, actor `+0xf78` (step 4) |
| 1 | | Handler table: which worker and entry the battle runs for which signal (`Mon_FindHandler`, `0x00797410`; below), `+0xf7c` |
| 2 | 128 + strings | Stat record, `+0xf80`; copied into the actor by `MsSetRamChr` |
| 4 | `0x118` + padding | Rewards, steal and bribe; copied per enemy slot, `+0xf88` |
| 5 | | Passed to `0x007a01b0` when an actor flag is set |
| 6 | 128 + strings | Copy of the stat record, `+0xf84`; identical to section 2 in every file |

## Handler table (section 1)

How the battle runs the script: for a signal `n` (a camera, a motion, an
action) and a value `k`, which worker and which of its entries.
`Mon_FindHandler(file, n, &k)` (`0x00797410`) reads it; battle files'
section 1 has the same layout ([battle.md](battle.md#script-handlers)).

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u8` | the script's work count (actor `+0xdf6`, `Mon_AttachData`): the next monster's works are numbered after these |
| `0x01` | `u8` | signal count `N` |
| `0x02` | `u8` × `N` | each signal's record slot, `0xff` none |
| halfword `(N + 1) / 2` | 4 bytes per slot | `+0` the worker, `+1` the handler's kind, `+2` `i16` offset from the section to its entry list |
| list | `u16` count, `u16` × count | the entry for each `k`; `0xffff` none |

`MsAtelRequestExe` (`0x00797d50`) sends the worker a request for that
entry at level 7. A monster file has four signals (332 of the 361
files have `0x3d`, 218 `0x00`, 195 `0x40`, 153 `0x04`):

| Signal | Kind | Asked for by | `k` |
|---|---|---|---|
| `0x00` | 0 | `MsAtelRequestCamera` (`0x00797bc0`), between the battle file's and `system_01`'s | camera number |
| `0x3d` | 2 | `MsActionRequest` (`0x007aceb0`): the monster's action script | the action ([battle.md](battle.md#script-handlers)) |
| `0x40` | 1 | `MsSetMotion` (`0x007ab360`), before the battle file's motion handler | motion |
| `0x04` | 1 | `MsSetMotion`, after the battle file's motion handler | motion |

`MsAtelChrInit` (`0x007976a0`) gives the workers of `0x3d`, `0x04` and
`0x40` the monster's actor index. In every file a worker handles at most
one signal and an entry at most one `k`.

## Stat record (sections 2 and 6, and the kernel `monster` records)

The first 20 bytes are five string references. In the monster file they
point to Japanese copies; the game takes the shown texts from the kernel
tables. The stats after them are byte-identical to the kernel record of the
same ID in all 361 files, but the battle reads the monster file's.

| Offset | Type | Field | Evidence |
|---|---|---|---|
| `0x14` | `u32` | HP | actor `+0x594` max (`MsGetRamChrHPmax`), also `+0x59c`; the current HP is `+0x5d0` (`MsGetRamChrHP`, `0x0079add0`) |
| `0x18` | `u32` | MP | actor `+0x598` max, also `+0x5a0`; the current MP is `+0x5d4` |
| `0x1c` | `u32` | overkill threshold | actor `+0x5a4`, `+0xf60` |
| `0x20` | `u8` | strength | actor `+0x5a8`; cubed for physical damage in `MsCalcDamageCommand` (`0x00789bf0`) |
| `0x21` | `u8` | defense | actor `+0x5a9`; physical damage reduction in `MsCalcDamageCommand` |
| `0x22` | `u8` | magic | actor `+0x5aa`; magic damage in `MsCalcDamageCommand` |
| `0x23` | `u8` | magic defense | actor `+0x5ab`; magic damage reduction in `MsCalcDamageCommand` |
| `0x24`–`0x27` | `u8` | four more stats | actor `+0x5ac`–`+0x5af`, clamped 1–255 (`0x26`: 0–255) |
| `0x28` | `u16` | | actor `+0x5b8` |
| `0x2a` | `u8` | | actor `+0x5ba` |
| `0x2b`–`0x2e` | `u8` × 4 | | actor `+0x5da`–`+0x5dd` |
| `0x2f`–`0x47` | `u8` × 25 | | added to actor `+0x641`…, one per status |
| `0x74` | `u16` | | actor `+0x8` |

Raldo (ID 1): HP 240, MP 10, overkill 360, strength 19, defense 0, magic
defense 120. Its toughness is the "armored" mechanic, not the defense stat.

## Rewards (section 4)

From `MsGetItem` (`0x007990d0`) and `Battle_Steal`
(`0x0078b760`). Items are `0x2000` + item ID (kernel `item` table).

| Offset | Type | Field |
|---|---|---|
| `0x00` | `u16` | gil |
| `0x02` | `u16` | AP |
| `0x04` | `u16` | AP on an overkill |
| `0x08`, `0x09` | `u8` | chance (x/255) of drop slots 1 and 2 |
| `0x0a` | `u8` | steal chance (x/255), halved after each successful steal |
| `0x0b` | `u8` | equipment drop chance (x/255, `0x00798c10`) |
| `0x0c`–`0x12` | `u16` × 4 | drops: slot 1, slot 1 rare, slot 2, slot 2 rare (rare: 32/256) |
| `0x14`–`0x17` | `u8` × 4 | their quantities |
| `0x18`–`0x23` | | the same eight fields, used on an overkill |
| `0x24`, `0x26` | `u16` | steal item, rare steal item |
| `0x28`, `0x29` | `u8` | their quantities |
| `0x2a` | `u16` | Bribe item |

The rest (equipment drop data, auto-ability lists `0x80xx`) is exported as
hex.

## Editing

```bash
ffx monster list                                       # ID, HP and name
ffx monster export 1 -o raldo.toml                     # [stats] and [spoils]
ffx monster import 1 raldo.toml --mod-dir mods/my-mod  # applies the edits onto the original
```

Stat edits are written to both section 2 and its copy in section 6. Names
and texts are in the kernel `monster` tables (`ffx kernel export monster1`).

## Verification

`cargo test -p ffx-monster` (with `FFX_GAME_DIR`) parses all 361 files,
round-trips each through TOML byte for byte, and checks that every file's
stats match its kernel record.

## Open questions

- Section 5; the stat bytes after `0x27`; the meaning of stats
  `0x24`–`0x27` (presumably agility, luck, evasion, accuracy, but not yet
  confirmed by code).
