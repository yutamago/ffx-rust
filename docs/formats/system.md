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

## The user heap's set-up and entry points

`malloc_pool_init` (`0x006fb9b0`) runs on the heap's first use:
`MemSystem_Ensure` (`0x009428a0`, sets `g_MemSystem` `0x0193cd2c` once),
then a 0x30-byte heap object from the C runtime's `malloc`
(`g_UserHeap`, `0x00ce901c`): a critical section, then the allocator at
`+0x18` set up by `UserHeap_Init` (`0x00942a40`: arena size rounded to
4 KiB, 0, the limit, no arenas, 16-byte minimum). The arena size starts at
0x30000000 with a 0x2ff00000 limit. A first 16-byte block proves the heap
(`0x00ce9020`). While that block fails, the object goes back to `free`
and the arena size drops by 16 MiB. The size that worked is kept at
`0x00ce9024`.

Entry points over it, each under the heap's critical section:

| Function | Behaviour |
|---|---|
| `user_malloc` `0x006fb910` | block filled with `0xcd` |
| `UserHeap_Calloc` `0x006fb820` | `n * size` (32-bit), zeroed |
| `user_aligned_malloc` `0x006fb770` | moved up to the alignment, the distance in the byte below |
| `user_aligned_free` `0x006fb6c0` | the block filled with `0xfe`, then freed |
| `Phyre::PhyreMalloc` `0x0042fb40` | `size + 0x14`, 16-aligned, the block kept in the word below |
| `Phyre::PhyreMalloc` `0x0042fb80` | `size + 4 + align`, aligned, the block kept below |
| `Phyre::PhyreFree` `0x0042fb20` | frees the block kept below |
| `operator new`, `new[]` `0x00630310`, `0x006304c0` | `user_malloc` |

## The `sce*`/`fios*` file wrappers

`sceRead` (`0x0062fc00`) calls `fiosRead` and returns 0. `fiosRead`
(`0x00679800`) returns 0 for a null stream, else
`PStreamFile::read`. `sceFileSize` and `fiosFileSize` return 0 for null.
Otherwise they return `PhyreFIOSStreamFile::getFileSize` (`0x006f9d50`),
which is -1 for a closed stream (`isOpen`: handle `+0` not -1, or a
big-file entry at `+4`). `PStreamFile::getFileSize` (`0x00607dc0`)
returns the entry's 64-bit size, else `GetFileSizeEx` on the handle.

`g_CdLib` (`0x02310c40`, `GetCD`) starts with function pointers. `+0` is
read, `+8` wait, `+0x10` open, `+0x58` name. `TODVDFileRead` and
`TOCDGetFileName` call them in that order. `+0x20` is the module's first
file number, `+0x30` the PC-name switch `GetCDFileName_PC` checks.

## GS register packets (`TOAdp*`)

Each writer packs its fields into one 64-bit register, the first field
lowest. Every field is sign-extended and OR-ed in without masking. The
register address follows the value.

| Function | Register | Field widths from bit 0 |
|---|---|---|
| `TOAdpTEX0` `0x008efa00` | 0x06 | TBP0 14, TBW 6, PSM 6, TW 4, TH 4, TCC 1, TFX 2, CBP 14, CPSM 4, CSM 1, CSA 5, CLD |
| `TOAdpCLAMP` `0x008ef6e0` | 0x08 | WMS 2, WMT 2, MINU 10, MAXU 10, MINV 10, MAXV |
| `TOAdpTEX1` `0x008efad0` | 0x14 | LCM 2, MXL 3, MMAG 1, MMIN 3, MTBA 10, L 13, K |
| `TOAdpXYOFFSET` `0x008efc60` | 0x18 | OFX 32, OFY |
| `TOAdpSCISSOR` `0x008ef910` | 0x40 | 16 each |
| `TOAdpALPHA` `0x008ef670` | 0x42 | A 2, B 2, C 2, D (26: FIX at bit 32), FIX |
| `TOAdpCOLCLAMP` `0x008ef760` | 0x46 | one value |
| `TOAdpTEST` `0x008ef970` | 0x47 | ATE 1, ATST 3, AREF 8, AFAIL 2, DATE 1, DATM 1, ZTE 1, ZTST |
| `TOAdpFBA` `0x008ef7d0` | 0x4a | one value |
| `TOAdpFRAME` `0x008ef880` | 0x4c | FBP 16, FBW 8, PSM 8, FBMSK |

`TOAdpFOG` (`0x008ef800`) puts the fog in the top byte (register 0x0a).
`TOAdpTEXFLUSH` (`0x008efb60`) writes register 0x3f with no data.

## The allocation tracker and `operator delete`

`operator new` and `new[]` are plain `user_malloc`. `operator delete`
(`0x00630390`) hands one block back to the C runtime's `free` instead.
That block's address is held at `0x00ccb160`, which is cleared after.
Every other block goes through `AllocTracker_Release` (`0x00630a60`) and
then `user_free`; `delete[]` (`0x00630540`) does the same. With a tracker
object at `0x00ccb15c`, the release takes the tracker's mutex
(`WaitForSingleObject` on `+0xa8`). It looks the block up in the map at
`0x00c31e9c`, a u32-keyed tree: key `+0xc`, children `+0`/`+4`, root at
`map + 0xc` (`MapFindU32_0062fe50`). It frees the record at the node's
`+0x10`, removes the node (`Phyre::PRedBlackTree::remove`) and pushes it
on the free list at `0x00c31eb4` (`Phyre::PFreeList::deallocate`). Then it
releases the mutex.
