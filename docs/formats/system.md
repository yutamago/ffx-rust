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

## Language-dependent text tables

`TOGetFFXLangStr` (`0x008ac300`) maps `TOGetFFXLang()` to a two-letter
code: 1 `us`, 2 `fr`, 3 `es`, 4 `de`, 5 `it`, 9 `kr`, 10 `ch`, 18 `uk`,
anything else `jp`. `0x008b56d0` picks the macro dictionary at
`0x01869eb8` the same way: us `0x00b63018`, fr `0x00b63e28`, es
`0x00b642d8`, de `0x00b634c8`, it `0x00b63978`, kr `0x00b650e8`, ch
`0x00b64788`. Japanese and the other values leave the pointer alone.
`0x008b5760` decides whether a text code joins its neighbours.
Japanese joins a `,` pair around a value up to 0x63. Korean and Chinese
join codes from 0x30 whose neighbours both lie in `0x5a..=0x8d`. Other
languages never join.

Names shown in menus come from the kernel and battle tables. The getter
(`MsGetComData`, `MsGetRomAbility`, `MsGetRomPlyCommand`,
`MsImportantPtr`) returns a record plus the base of its string table.
The record's `u16` at `+0` (name) or `+8` (help) is an offset into that
table. The command and key-item getters (`0x008bebb0`, `0x008beb70`,
`0x008bec30`, `0x008bebf0`) use the next word's offset, `+4` or `+0xc`,
when the hiragana setting is on (`MsGetSaveConfigHiragana`, menu config
bit 3).

## Save/load menu set-up

`0x008b3510` (load) and `0x008b3580` (save) prepare the save/load menu.
1. Clear `0x018663c4`, then call `TkMcSetDebugLock(1)`.
2. Clear the menu's `+0x1c`, the word `0x01866250` and the two flag
   bytes at `0x01866268`, then reset the toggles
   (`MenuSaveLoad_ResetToggles`).
3. Set `0x0186628a`: 1 for load, 0 for save.
4. Turn the screen-x2 flag off and set the frame ratio difference to -16.

`0x0186628a` decides which slots of `g_TkSaveSlots` (`0x01866290`)
`0x008b3730` reports as shown. With it clear, only state 1 counts. With
it set, any state up to 2 counts, and so does state 4.

## Debug and development code (not ported)

None of this runs in normal play. It stays unported, and it is listed
here so the useful parts can come back as mod tools.

| Function | What it does |
|---|---|
| `SetDebugHandler` `0x0062f4d0`, `dbgPrintf` `0x0062f500`, `scePrintf` `0x0062fbf0`, `debug_exit_trace` `0x0062f510` | The PS2 SDK's debug-print entry points, left as empty stubs on PC. |
| `sceDeci2Open` `0x0062f680`, `sceDeci2ReqSend` `0x0062f690`, `sceDeci2ExReqSend` `0x0062f660` | DECI2 host-link stubs (the PS2 devkit protocol); they return without doing anything. |
| `printf2` `0x0076dd10` | Formats a message (`vsprintf`) and routes it by its `FFXDEBUG` prefix: to `dbgPrintf`, or (debug config set) to the game's text output. |
| `TODapsInit`/`TODapsDraw`/`TODapsSetPos`/`TODapsString`/`TODapsInteger`/`TODapsPrintf`/`TODapsMakePkt` `0x0089dc10..0x0089dde0` | The on-screen debug text buffer (see "Debug print buffer"). Some were ported in earlier rounds; the rest stay. Useful as a mod overlay. |
| `TODebugAddPadPrintFlag` `0x008abd00`, `TODebugGetPadPrintFlag` `0x008abd20` | Cycle and read the pad-print debug mode (0..3). The menu advances it when both menu pad words are 0x100. |
| `FFXIsDebug` `0x00887c50`, `0x00887c80` | Read and set the debug flag `0x0133c910`. The setter only works when `g_GsDefDrawEnv2` is set. |
| `SaveDataPreDebugSave` `0x006484d0`, `SaveDataManager_preDebugSave` `0x006f2410`, `SaveDataManager_debugSave_Internal` `0x006f0510`, `debugSavedataLoadAll` `0x006492c0` | The debug save path. It stamps the CRC (`SaveDataWriteCrc`) and writes the raw save to a file in the default save folder (`_wfopen`/`fwrite`); the load-all variant reads them back. A candidate mod tool for save editing. |
| `SaveDataTtyWrite` `0x00648820`, `SaveDataTtyWrite2` `0x00648850` | Print save-system status lines to the console (`fprintf`). |
| `debugLoadForViewer` `0x00649040` | Builds a path (`sprintf`) and reads that file into memory (`fopen`/`fread`), printing `SKS_DEBUG` messages on failure. Called from the save menu (`0x008b55e0`). |
| `debugPrintString`/`debugPrintStringToHex`/`debugPrintStringToUTF8` `0x00649180..0x00649220` | Print a string as text, as hex, or as UTF-8 code points (`SKS_DEBUG: %s: ...`). Handy for text-encoding mods. |
| `prcPrintMatrix` `0x00766c50`, `prcPrintPacket` `0x00766d10` | Dump a 4x4 matrix or a GS packet to the console. |
| `ModelBuffer_PrintMemUsage` `0x0072c1a0` | Empty stub that once reported model-buffer memory use. |
| `0x00687850`, `0x00687e90` | Load `dbghelp.dll` from "Debugging Tools for Windows" and write crash minidumps. |
| `0x0092ccf0`, `0x0092cdd0` | Reload a field's FP and guide-map data from the development tree ("DEBUG FP LOAD", `rcbgDebug.c`); nothing calls them. |
| `0x00a42a00`, `0x00a42ab0`, `0x00a42f50` (`read_debug.c`) and `pc_fat_index_filename_set` `0x00a42fc0`, `pc_module_sizetbl_set` `0x00a43080`, `pc_sizetbl_set` `0x00a430c0`, `pc_sizetbl_set_Ex` `0x00a43100` | Point the CD layer at `host0:/ffx/proj/...` index and size tables; `host0:` is the devkit host. |
| `TOInitBattleFontPath` `0x008ad780`, `TOInitEventFontPath` `0x008ad840` | Build font file paths under `host0:/ffx/master/...`. Classed as file layer, not game state: they only choose where fonts are read from. |
| `0x00a7aee0` | Loads `host0:%s.irx` modules (IOP drivers); nothing calls it. |
| `TkyiAssert` `0x00777680` | Asserts: tail-calls the empty `0x0088ec80` when the debug word `0x0112a520` is 0. |

## Message-window placement (`TOPlaceMessageWindow`, `0x008afbb0`)

`TOPlaceMessageWindow(i, x, y, w, h)` places message window `i`. Its
style (`+4` of the object `g_TOMessageWindowSlots[i] + 0x10`) picks a frame
padding (`0x00c59ed8`, i16 left, top, right, bottom per style: 20/10,
32/12, 16/8, 0, then 16/8). Outside message-window mode 1 the padding is
taken off the rectangle first and added back after the x87 rounding.

On wide screens (`graphicFontGetScreenWH` height at least 0x2d0) the
window is kept inside the safe area of the 512x416 layout: the top at
least 31.2, the bottom at most 384.8 (a window of type 1 measures from its
padded edge). Horizontally, type 1 maps its left and right edges to the
screen (`h / 416` scale plus the 4:3 offset `w * (1 - (16/13) / (w/h)) / 2`)
and keeps them within 7.5% and 92.5% of the width, then moves 10 units
in; other types stay within 38.4..473.6. Two fixes are tied to maps: on
map 0x17 a window at (400, 0x110) moves by (-45, +5), and on map 0x15b a
0x1d8 x 0xf6 window is drawn 0x1ba wide (its message area keeps 0x1d8).
`TOSetWindowRect` gets the result and `TOSetMessageArea` (`0x008b8b20`) the
padded area.

## Texture names (`TOGetTextureName`, `0x008acf50`)

`TOGetTextureName(id, nibbles, index, high, &size)` returns the name of
picture `index` of a texture family. A nibble of `nibbles` (the high one
when `high`) gives a column `k` = twice its low three bits. Its bit 3, or
`high`, selects the second table of each pair and the second size table
(`0x00b5f8d0` instead of `0x00b5f890`, two words per entry, entry
`(index + k) / 2`).

| id | tables (plain, second) | indexed by |
|---|---|---|
| 0x3c00 | `TkGetLang()` 0: `0x00c59568`/`0x00c595a8`; else `0x00c595e8`/`0x00c59628` | index + k |
| 0x3d00 | `0x00c59bc0`/`0x00c59c00`, appended to the prefix `0x025d08a0` (in battle `0x025d06a0`) in `0x025d07a0` (`0x025d05a0`) and interned | index + k |
| 0x3d01 | by `TOGetFFXLang()` 0, 9, 10: `0x00c59878`/`0x00c59938`, `0x00c598f8`/`0x00c599c0`, `0x00c598b8`/`0x00c59980` | index (second table and size only by `high`) |
| 0x3d10, 0x3d20, 0x3d30, 0x3d40 | `0x00c59668`/`0x00c596e8`, `0x00c596a8`/`0x00c59728`, `0x00c59768`/`0x00c597a8`, `0x00c595e8`/`0x00c59628` | index + k |
| 0x3d60 | as 0x3d01 with `0x00c59a00`/`0x00c59b00`, `0x00c59ac0`/`0x00c59b80`, `0x00c59a40`/`0x00c59b40` | index |
| 0x3d80 | `0x00c597f8`/`0x00c59838` (second by `high`) | the byte `nibbles` |

Another language for 0x3d01/0x3d60 reads the word at `k + 4 * index` (a
null-page read in the original). Any other id prints "VIRTUOS
ASSERTION:Font not found." and returns `k`.

The help pictures (`getSa_MenuTextureName`, `0x00774ae0`) are
`help%d.dds.phyre` under `help`, `help_us`, `help_fr`, `help_es`,
`help_de`, `help_it` (languages 0..5), `help_kr` and `help_ch` (9, 10).
Languages 6..8 format an uninitialised buffer in the original; the port
writes an empty name.
