# System-layer structures

In-memory layouts the system module (`crates/ffx-system`) reads and writes,
recovered from FFX.exe. All integers little-endian; addresses are the
image's (`symbols/ffx.sym`).

## DVD layout constants

Nine `dvd_cd_*` functions (`0x0076d6c0` + `0x80 * kind`) each pick one of
seven getters (`0x0077a4f0` + `0x90 * variant` + `0x10 * kind`) by
`MsGetSaveConfigEnglish` (always 0 on PC) and `TkGetLang`. Every variant
returns the same value, so the language never matters:

| Kind | Function | Value |
|---|---|---|
| 0 | `dvd_cd_fat_data_size` | 2 |
| 1 | `dvd_cd_fat_sector` | `0x118` |
| 2 | `dvd_cd_fat_size` | `0x20000` |
| 3 | `dvd_cd_fid_sector` | `0x158` |
| 4 | `dvd_cd_fid_size` | `0x82` |
| 5 | `dvd_cd_id_sector` | `0x117` |
| 6 | `dvd_cd_id_size` | `0x30` |
| 7 | `dvd_cd_unk7` (`0x0076da40`) | `0xf` |
| 8 | `dvd_cd_unk8` (`0x0076dac0`) | `0x10` |

## Menu sequence slots (`g_TOSeqSlots`, `0x01865f20`)

0x100 bytes per menu scene (8 scenes), eight 0x20-byte slots each
(`TOSeqStartScene` `0x008af0d0`, `TOSeqStopScene` `0x008af4f0`,
`TOSeqInitAllScene` `0x008ae5a0`, `TOSeqCheckTermScene` `0x008ae070`):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u16` | State: 0 free, 1–3 running, 4 stopped |
| `0x04` | `u16` | 1 when started by `TOSeqStartSceneDrawContinue` (`0x008af1b0`) |
| `0x08` | C string | Sequence name (cleared when stopped) |
| `0x1c` | `u32` | `TkMenuGetSceneHandle(scene, name)` |

A start fails when a slot already carries the name (in any state) or no
slot is free; the handle is stored even when it is 0.

## Debug print buffer (`TODaps*`)

`g_TODapsBuffer` (`0x0133fab0`, 0x800 bytes) and its fill count
`g_TODapsUsed` (`0x0133faa0`, `i16`). `TODapsString` (`0x0089dec0`) appends a
string with its terminator unless that would reach 0x7c0 bytes;
`TODapsSetPos(x, y)` (`0x0089de30`) appends `08, x/100, x%100, y/100,
y%100` (bytes) whenever the count is below 0x7c0. `TODapsInit` and
`TODapsDraw` (the same code) clear the buffer and set the two `i16`s at
`0x013402b0`, `0x013402b4` to 10.

## Fill-buffer queues (`0x00cec1a8`)

Six queues: bases at `0x00cec1a8`, request counts at `0x00cec1c0`, loads
at `0x00cec1d8` (`i32` each). `JobSchedule_pppPart_getFillBufferRequest`
(`0x00728660`) gives a request to the least-loaded queue with fewer than
0x156 requests (strictly below 100000, first on a tie) and returns its
0x214-byte slot `base + (count - 1) * 0x214`.

## Font glyphs (`g_TOFontGlyphs`, `0x00c58f00`)

Four bytes per glyph: x, y, flags, width. A text byte from `0x30` is glyph
`byte - 0x30`; a lead byte below `0x30` makes the next byte glyph
`byte + 0xa0`. `TOFontGlyphSprite` (`0x0089e360`) turns one into a sprite:
`+8`/`+0xc` x, y; `+0x10` x + width; `+0x14` y + 8.0; `+0x1c` width; `+0x20`
8.0; `+0x26` the flags and `+4` their bit 3.

## Battle command-window stack (`g_BtlActionStack`, `0x0133c900`)

0xf0 bytes per level, the current level `g_BtlActionDepth` (`0x023cc092`,
`i8`). `TOBtlSetPrevAction` (`0x0089a630`) marks the level it leaves
(`+0x51` = 5, `+0x58`, `+0x66` = 0xff), goes down one level, marks it
(`+0x51` = 2), steps its cursor (`+0x5b`) back when at or past `+0x5a`, and
restores the command list (`0x0133d0d0` from `0x0133d210`, 0x140 bytes)
when that level's `+0x52` is 1.

## GS `TEST_2` packets

`TOAdpTEST(p, ate, atst, aref, afail, date, datm, zte, ztst)`
(`0x008ef970`) writes the GS `TEST_2` register (address 0x47): fields from
bit 0 ATE(1), ATST(3), AREF(8), AFAIL(2), DATE(1), DATM(1), ZTE(1), ZTST(2).
Each argument is sign-extended to 64 bits and OR-ed in without masking.
