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
