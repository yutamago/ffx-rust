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

Two text widths use it. `0x0089e410` adds, per glyph, its width plus 1,
and 5 more when the flags byte is nonzero; the sum goes through x87 and
`ftol`. `0x0089dfa0` counts a fixed 12 per glyph, less 2 for the whole
text. Both step through the text with `TOSkipCode`.

## Battle command-window stack (`g_BtlActionStack`, `0x0133c900`)

0xf0 bytes per level, the current level `g_BtlActionDepth` (`0x023cc092`,
`i8`). `TOBtlSetPrevAction` (`0x0089a630`) marks the level it leaves
(`+0x51` = 5, `+0x58`, `+0x66` = 0xff), goes down one level, marks it
(`+0x51` = 2), steps its cursor (`+0x5b`) back when at or past `+0x5a`, and
restores the command list (`0x0133d0d0` from `0x0133d210`, 0x140 bytes)
when that level's `+0x52` is 1.

`TOBtlSetTarget(level, a, b)` (`0x0089a860`) marks the level as targeting
(`+2` = 1) and runs `TOCheckBtlCommandTarget(a, b)`. It then counts the
request at `+0xa6`, keeps the target word `0x0133d1fc` at `+0x42` and
`0x0133d178` at `+0xd8`, and calls `TOBtlSetTargetCallback` with the
level.

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

## Overdrive (limit) windows

The battle overdrive windows share a timer: `0x0133f77c` (1 opened, 2
running), the frame count `0x0133f788` and three times at `0x0133f78c`
(the limit), `0x0133f790` (time left) and `0x0133f794` (shown).

- `TOBtlInit{Auron,Lulu,Tidus}LimitWindow` (`0x00897970`, `0x00897f70`,
  `0x00898180`) read `dumm..` from scene 2, reset the pad data and the
  timer, and clear their window record: Auron `0x0133d6f4`, Lulu
  `0x0133d708`, Tidus `0x0133d734`. Their points and colours come from
  the scene; Tidus' band edges are UI-remapped constants.
- `TOBtlOpen*LimitWindow(cmd)` turn the command into a level (Auron 0x22,
  0x32, 0x42, 0x52; Tidus 0x20..0x50; Lulu's group is `cmd - 0x3078`).
  They take the time from `NkzGet*LimitTime` and start the timer. Auron
  gets the button sequence (0xffff-terminated) and its length; Lulu her
  limit table; Tidus his band from `NkzTidusLimitTableA/B`, centred on
  the window.
- `TOBtlCtrlLimitTimer` (`0x00891ac0`) counts frames while the timer
  runs. The time left is the limit less frames/30 (25 in PAL mode). At 0
  or below both times become 0. Otherwise the shown time gets a random
  hundredths digit from the battle `rnd`, so the display flickers while
  the value it is derived from does not.

## Message-window position (`TOSetMessageWindowPosition`, `0x008af850`)

The window's text size (`+0x40`, `+0x42` of its `+0x14` record) plus the
frame padding gives its size. With the tutorial choice set
(`0x025d16bc`), a text wider than 0x190 becomes 300 wide, with
`width / 300 + 1` rows, and `0x025d0b20[i]` records it. Anchors 1..4 put
the point at the bottom, right, bottom-right or centre. The window is
kept in the 512x412 field (rows to 0x7b in message mode 1). Magic 0x281
on maps 0x5e and 0x12e, and type-1 windows while `0x01327104` is set,
may leave it horizontally. `0x008afa90` is the same with an explicit
size.

## Battle target lists and the saved cursor

`TOBtlMakeTargetTargetBit` (`0x00898640`) fills the party list
`0x0133d180` (20 entries, count `0x0133d17c`) and the monster list
`0x0133d1d4` (8, count `0x0133d1d0`). The entries come from the group
masks of `TOBtlMakeTargetGroupBit`, ANDed with the selectable mask
`0x0133d174`. Bit 1 of the command's `+0x1a` flags selects the monster
list. Group mode 0 starts at the cursor character's entry. The counts are
re-read from memory after each list, so a list longer than its slots
runs into the next count, as in the original.

`TOBtlSetDefaultCursorPos` (`0x00899ac0`) restores what
`TOBtlMemoryCursorPos` saved, but only if the window key still matches.
The scroll is looked up through the found list index used as a window
number. That is how it is compiled, and the port keeps it.
`TOBtlSearchRevNearTarget` (`0x008990f0`) writes nothing. It returns, in
eax, the index of the target it picks; the decompilation misses that
return, and `TOBtlGetTargetPad` reads it. The axis runs between the mean
x/z positions of the two sides; on the monster side only those at
distance 0 count, if there are any. From the cursor character, each
target on the other side scores its angle from that axis (degrees,
`_CIacos`) times its distance, divided by the nearest distance (at least
1). The lowest score wins. If the other side's list is empty, the
original returns an uninitialised word; the port returns 0.

## Achievement counters

- `0x00906b30(n)` keeps the 26 collection bits at `0x011307a0`, in the
  save. Each new bit unlocks achievement 1; with all 26 set, it unlocks
  0x1c. -1 clears the bits and 0xff sets them all.
- `0x008b5680` counts visits to map `0x00c3347c` while its latch
  `0x00c5a03c` is set. The 0x3a-th visit unlocks achievement 0x21.

The Steam call itself (`achievementUnlockAchievement`) is platform. The
counting decides game state, so it is ported.

## Pad repeat (`TOGetPadStatus`, `TOMenuGetPadStatus`)

Both functions build three pad words each for the d-pad (`0x025d09d2`
held, triggered, repeated) and for a stick-made d-pad (`0x025d09e2`; the
left stick below 0x18 or above 0xe8 adds 0x1000, 0x4000, 0x8000 or
0x2000). Repeats are timed with `inputGetCurTime`, not counted in frames.
The per-button state is a count at `0x025d09c2 + k`, a held time at
`0x025d0a00 + 4k` and the last frame's time at `0x025d0a40 + 4k`.
Buttons 0x800 and 0x100 never repeat.

| | first repeat | then every | count after a repeat |
|---|---|---|---|
| `TOGetPadStatus` (`0x008b7cd0`) | 0.2333 s | 0.1 s (restart 0.1333) | 4 |
| `TOMenuGetPadStatus` (`0x008be500`) | 0.4667 s | 0.1667 s (restart 0.3) | 9 |

`TOGetPadStatus` runs only while `0x025d09c1` or `g_TOPadGetCtrl` is set.
It uses the d-pad set if any d-pad button is held, else the stick set,
then copies the chosen set over the other. The menu version always uses
the stick set. It also ORs both pads into `+6..+0xa`, and with the dummy
controller on it takes everything from the dummy.

## Kernel menu accessors (`0x00771b20..0x007744ff`)

These 58 small entries come from a table; nothing calls them directly.
29 ask `MsGetSaveCommand(chr, cmd)` for a fixed command (0x30b8 onward).
24 name the ability in a u16 slot (`+0x11e..+0x14c`) of record `i`
(0x1d8 bytes each) of `*0x0112a034`. Three read a word of
`MsGetSavePlayerPtr(chr)` (`+0x50`, `+0x54`, `+0x58`). The last two
forward to `MsGetSaveItemNum` and `MsImportantGet`.

The second run (`0x00772500..0x007744ff`) has the same shapes. 132 ask
`MsGetSaveCommand`. 39 name a slot, five of them through
`Kernel_GetItemName` (item slots). One forwards to `MsGetMonsterCapture`.
Eleven differ:
- three post a menu request once (`0x0112a038` pending, kind
  `0x0112a058` 1/0/3, value `0x0112a054`);
- event flag bytes and bits of `MsGetSaveEventFlag()`, read and set;
- the price at `+0x194` of a record (0x1f40 for record 0xf9), and the gil
  left after paying it;
- whether a character has joined;
- a sound with bit 31 set.

The table holding these is at `0x00c4119c` in `.data`; no code refers to
its address directly. For the set-type entries the ports return eax as
the original leaves it, because whoever calls through the table may read
it.

## Language set-up and timed messages

`0x008ad900` sets the TO library's language word (`0x01865b5c`), the
language swap flag and the video mode from `TkGetLang()`:

| `TkGetLang` | language word | swap flag | video mode |
|---|---|---|---|
| 0, by `FFXGetAsiaMode` 0 / 1 / 2 | 0 / 0x11 / 9 | 1 | 0 (NTSC) |
| 1 (English) | 1 | 0 | 0 (NTSC) |
| 2..5 | same | 0 | 1 (PAL) |
| 18 | 0x12 | 0 | 1 (PAL) |
| any other | unchanged | unchanged | unchanged |

`0x008b9220` plays timed macro messages. It counts frames while
`0x018663d0` is set. It picks a table for the map (row
`g_GraphicMapId - 0x189`, 0x32 frame numbers per row) and calls
`TOMessageCallF8` each time the count reaches the next frame number. A 0
in the table ends the sequence and resets its state.

The 0x6f1820 group (`0x006f1820..0x006f1890`) is not ported. It is
thiscall delegate glue: each stub loads a bound object (`+0xc`) and
function (`+0x10`) from `this` and calls or jumps through them. It is
classed logic only by default.

## Menu record helpers (`0x00775530..0x007767d0`)

The help and menu screens keep their entries in pages of 0x138-byte
records. The page table is at `*0x0112a01c`: 8 bytes per page, an offset
from `*0x0112a03c` and an i16 record count, with `0x0112a080` pages. The
current page and record are in `0x0112a088` and `0x0112a084`.

- `0x00775ce0` and `0x00775de0` search the pages for a record: by its
  address in the first case, by bit 0 of `+1` in the second. They make the
  record current and copy its value pairs (`+0x1c` pairs from
  `*0x0112a028 + 8 × +0x1e`) into the value table `*0x0112a0a4`.
- `0x007767d0` applies fixed edits to screen 5's first page and to
  screen 0xb's pages 13 and 14: it moves some records and unlinks others.
- `0x00775820` gives the texture size of a menu picture for screens 5 and
  0xb.
- `0x00774a50` tests a record's own condition: the function at
  `+0x20` (none when negative) is called with `+0x22`. Mode `+0x2b` 0
  wants a zero result, 1 a nonzero one, and any other mode fails.
- `0x00774860` expands one 8-byte text code into a record's text. Kind 1
  copies a string from a condition-table function. Kind 0 writes a number
  as six digits, with `:` for leading zeros. The number comes from a
  function or the value table. When the code has a flag byte, the number
  goes to `0x0112a06c`/`0x0112a070` instead of the text.
- `0x00776110` is one step of the list's cursor. A pending request
  (`0x0112a038` = 1) is handed to `TOMenuHelpSetExtMode` and
  `TOMenuTermHelp`, and the state becomes 2. Otherwise the pressed
  direction is followed through the record's 12-byte link (at
  `+0x30 + 12 × direction`) to its target. The move is allowed when the
  link has no condition, or when the condition's result differs from the
  link's flag. A move makes the target current and plays `+0xf6` on a page
  change or `+0xf0` within the page; a refused move plays `+0xf8`.
  Without a move, an entry of kind `+0xfb` 1 (pad 0x40) or 2 (pad 0x20)
  posts a request: its kind `+0xfa` goes to `0x0112a058` and the value
  `+0xfe` (an index into the value table) goes to `0x0112a054`.

`0x00775330` builds a GS packet (GIF tags, `TEX0`/`TEX1`), so it is
skipped.

## Text helpers of the HD menus and battle windows

- `getScaleButtonIconWidth(id, scale)` (`0x009062b0`) gives a button
  icon's width for the HD UI. Icon ids 0x258..=0x263 use 54, 0x264..=0x267
  64, 0x268 64.5 and 0x269 147.4: the scale times that, plus 0.5, goes
  through `ftol`, an f32 and `graphicUiRemapX2`. The character last set in
  `0x025d16e4` then overrides the result: digits and space 9.0, `:` and
  `;` 13.6, `@`..`O` 20.0 when the low nibble is a multiple of 3 and 9.0
  otherwise, with `@` itself 33.5. `TOGetScaleButtonIconWidth`
  (`0x008fa800`) stores the character there and picks the id.
- `TOIntToMes(value, out, digits, mode)` (`0x008b8290`) writes a
  non-negative number as `digits` characters. Mode 0 keeps the low digits
  and pads on the left with a language blank (`]`, `:` or 0xfe). Mode 1
  clamps to the largest value and pads with `0`. Both copy `digits` bytes
  of the `sprintf` buffer, so after a short number they copy stale stack
  bytes behind its terminator; the port copies zeros there instead. The
  text up to the terminator is the same.
- `TOValueToMesFontL` (`0x008b9140`) is the large-font variant without
  truncation: the blank is `^`, `:` or 0xfe, and non-digits other than
  spaces are dropped.
- `TOConvAlbhedWord(in, out)` (`0x00906780`) translates one character of
  Al Bhed. The letters known are the 26 bits of `0x011307a0`, unless
  `0x018763e0` forces none (-1) or all (1). A known letter stays as it is
  (result 2) and an unknown one is translated (1). Japanese and Korean
  texts work on kana ids: the letter is `(id - 0x5e) % 26`, and a
  translated id is shifted by a fixed amount (Korean also through
  `TOShiftListedId`) and re-encoded.
- `TOCheckBtlCommandUse(chr, command)` (`0x0089aca0`) decides whether a
  battle command can be used: -1 when `MsGetCommandUse` blocks it, -2 when
  `MsCheckUseCommand` fails, the item window kind for item-like commands
  (-4/-3 for an empty list), else 0xfe or 0xff.

## Glyph metrics and message sizes

- `TOGetFontInfo(text, out)` (`0x008b7240`) describes one glyph and
  returns the text after it. `out` holds the font page (`+0`), the cell
  parity (`+4`), the cell box (`+8..+0x14`: columns of 14 for every two
  glyphs, rows of 18), the slant (`+0x18`: -1.0 for language 10, 1.0 for
  languages 1-8), the width (`+0x1c`, a quarter of the width-table byte
  from `TOGetFontWidthTblAdrs`) and 18.0 (`+0x20`). Codes from 0x2c use
  the language's dictionary; 0x26..0x2b are two-byte codes of tables 1-3;
  6 is an icon. A `:` that joins its neighbours is 5.0 wide, and a run
  of `G` at the start of a text is measured as one glyph.
- `TOGetChangeMacroAddress(window, code)` (`0x008b6660`) finds the text a
  macro code stands for: a macro slot (`0x01869ab0`, 32 per window), a
  character, item or key-item name, a window string table, or a number
  written by `TOIntToMes`. A slot whose kind is a code byte folds `+4 /
  0xd0` into that code and expands it in turn. The original builds that
  code in its own argument slot and can return a pointer to it; the port
  keeps it in a static, so the text stays readable.
- `TOGetMessageWH` (`0x008b9650`) measures a message: the widest line in
  pixels and the height of the deepest line (lines of 0x10, plus 2). Codes
  0-3 end lines and the text, `7 n` positions and spaces, `0xe 0x40`/`0x41`
  open and close a 5.0 indent, `0xb n` adds a button icon and `0x10 n` the
  window's counter. Macros add `TOGetChangeMacroWidth`. The battle widths
  (`ToGetBtlEasyFontWidth`, `ToGetBtlEasyDigitWidth`,
  `ToGetCrossExtMesFontWidth`, `TOGetWidthMacro_help`) add glyph advances
  (`TOGetFontAdvance`) and take off one letter space at the end.

## Battle command windows

`TOBtlSetNextAction(chr, command)` (`0x0089a330`) moves the command
windows one level deeper. At a targeting level the command becomes the
target pick 0x3115, and the command finishes (`TOBtlFinishCommand`) once
all targets are chosen. `TOCheckBtlCommandUse` decides what opens next:
a sub-window (`TOBtlSetCommand`, 0x80 or less), a target pick (0xfe) or
the finish (0xff). A new target pick with no forced target starts on the
first dead party member.

## Battle font layout (`ToMakeBtlEasy*`)

The battle texts (damage numbers, counters) are laid out with a text
cursor at `0x025d09a0`: `+0` the colour bytes, `+4`/`+8` the pen (f32),
`+0x1a` the icon mode and `+0x1b` the alpha. `ToMakeBtlEasyFont`
(`0x00905ab0`) and `ToMakeBtlEasyEdgeFontA` (`0x00905970`) walk a text:

- Line codes (group 1) move the pen back to the start x and 16.0 down.
- Glyphs (group 4) get their `TOGetFontInfo` record, then their packets,
  and the pen moves on (`TOAddFontWidthSpace`).
- `0xb n` draws a button icon at the pen (`TODrawScaleButtonIconRGBA`, y
  + 1.7 at scale 0.6, or + 3.2 at 0.73 in icon modes 1 and 3) and moves the
  pen by the icon width plus 1.0 (1.5).

The digit writers (`ToMakeBtlEasyDigit`, `…Digit2`, `…DigitRight`,
`…DigitRightAbmap`, `…EdgeDigit`, `…EdgeDigit2`) write the number with
`TOValueToMesFontL`, using as many digits as it has; the right-aligned
ones start the pen at x less the number's width.

The packet writers compute the glyph quad from the pen and the record.
The corners are the pen plus `(0, K_y × scale × slant)` and pen plus
`(width × scale × K_x, K_y × scale × (18 + slant))` (`K_x`, `K_y` at
`0x00c5d778`/`0x00c5d77c`), and the UVs are the cell over the texture
size. The quad then goes to `BeginScissor_XYUV` and `graphicFontDraw`.
The face (`TOMakePktBtlEasyFont`) draws at layer parity + 2. The shadow
(`TOMakePktBtlEasyFontShadow`) is moved by `0x018716e0`/`0x018716e4`.
The edge (`TOMakeBtlEasyFontEdge`) uses `K_x` for both axes and draws
eight copies offset by ±2 over the edge ratio. These are ported; only
`graphicFontDraw` itself emits packets.

`TODrawScaleButtonIconRGBA` (`0x008f8ee0`) picks the icon for a text
character, as `TOGetScaleButtonIconWidth` does. `4`..`7` sit 2.0 higher,
and `@`..`O` stack four direction parts by their low bits.

## Message glyph writers (`TOMakePktFont1612*`)

The 16 × 12 message font uses the same pen layout as the battle texts
(`+0x1a` the kind, `+0x1b` the alpha). `TOMakePktFont1612` (`0x008fc7c0`)
draws one glyph by the kind:

- kind 1: the edge (`TOMakePktFont1612Edge`, `0x008fc870`), then the face;
- kind 3: the italic edge and face (`TOMakePktFontLItalicEdge`/`Font`);
- otherwise the shadow (`TOMakePktFont1612Shadow`, `0x008fcca0`) with UI
  draw type 1 and shadow type 0, then the face
  (`TOMakePktFont1612Font`, `0x008fcad0`) with shadow type 1.

The pen then moves on by the advance spacing `0x00c5d784` (1.0) for kinds
1 and 3, or `0x00b5ee10` (0.78) otherwise; the same value scales the
glyph. Each pass computes the quad from the pen and the `TOGetFontInfo`
cell. Its top is `pen.y + scale × slant`; its size is
`width × scale` by `scale × (height + slant)`, stretched by `K_x`/`K_y`
(`0x00c5d778`/`0x00c5d77c`) for the face and shadow but not the edge. The
shadow is moved by `0x018716e0`/`0x018716e4` and drawn with offsets
`0x00b92268` (2.0) at layer parity + its argument (4 skips the scissor).
The face draws at parity + 2 (parity + 6 without scissor in mode 5). The
edge draws ten copies at offsets (±2, ±2), (0, ±2), (±2, 0) and (±1, 0)
over `graphicFontGetEdgeRatio`. The italic passes (`TOMakePktFontLItalicEdge`/`Font`, `0x008fce90`/
`0x008fd0a0`) use scale `0x00c5d784` and the edge geometry for both,
depth 2001, no scissor and layer 0. The italic edge's offsets follow the
same pattern, read from the image floats -2 (`0x00b41da4`), 2
(`0x00b92268`) and -1 (`0x00b92340`). `0x00906270`/`0x00906290` are the face and
shadow passes with argument 0. Only `graphicFontDraw` emits packets.

`TODrawCrossMenuIconAbilityName` (`0x008f4f10`) and
`TODrawCrossMenuIconCommandName` (`0x008f5000`) draw one cross-menu row.
The entry (`MsGetRomAbility`/`MsGetComData`) gives the name offset (word
`+0`) and the icon (byte `+0x68`/`+0x14`). The icon is drawn by
`DrawCrossMenuIconXYWHRGBA` at the row plus `X2(0x00b49980)` (command:
`0x00b5a480`) and `Y2(0x00b5f2e8)`, sized `X2(0x00b59830)` ×
`Y2(0x00b5f304)`. The name is drawn by `ToMakeBtlEasyFont` at the row plus
(`0x00b5f374`, `0x00b92318`) (command: `0x00b5ef30`, `0x00b92314`), scale
0.78.

`DrawCrossMenuIconXYWHRGBA` (`0x008e6af0`) draws icon `n` from sheet
0x3dc0: 17 icons to a row, each 15 × 18 texels (UVs over the sheet size).
The colour is passed as bytes, the depth is 500, the quad is clipped by
`BeginScissor_XYUV` and drawn by `graphicDrawUIElement(rec, texture, 1, 0,
0)`.

`TOMkpCrossExtMesFontLClutTypeRGBA` (`0x00901700`) writes a cross-menu
text in the battle font with the pen at `0x025d09a0`. A new line moves y
by 13.6 at scale 0.64, otherwise by `scale × 16`. Its control codes (7
spacing, `0xa` mode nibbles, `0xb` button icons at scale 0.63 lifted by
`AdjustButtonIconY`) work as in `ToMakeBtlEasyFont`. Macros go to
`TOAddPktChangeMacro_help` (`0x008e9ca0`), which writes the replacement
text's glyphs and nests for macro codes 0x12..=0x22. `AdjustButtonIconY`
(`0x008e5280`) is -1.5 when the language thunk `0x008ac2f0` is non-zero,
otherwise -0.5.

`TODrawMenuBG` (`0x008f5c10`) draws the menu background as shape -2
(`TOMkpShapeXYWHUV`) over `(0, 0, [0x00b41d9c], [0x00b41d98])` with UVs up
to `([0x00b4a1ac], [0x00b55178])`. `TOMkpCrossExtMesFontLClut`
(`0x009016b0`) is `TOMkpCrossExtMesFontLClutTypeRGBA` with type 0 and grey
`0x80` colour.

## UI shapes and menu plates

`TOMkpShapeXYWHUV` (`0x00903bb0`) and `TOMkpShapeXYWHUVC2` (`0x00903ee0`)
build the quad of a UI shape. The corners are `x, y` and `x + w, y + h`;
`BeginScissor_XYUV` clips them together with the UVs. The UVs are then
scaled for the shape's sheet:

- Shapes -1, -3 and -6 divide by the image size of 0x3e80, 0x3e83 or
  0x3e86.
- Shape -2 uses 1/2048 and 1/1024; shape -4 (C2 only) uses 1/256.
- Shapes 0..=0x289 pick sheet 0x3e00, 0x3f00 or 0x3dc0 by range.

For any other shape the original passes a stale stack word as the
texture; the port passes 0. The quad goes to `graphicDrawUIElement`
with grey corners, or with the C2 colours.

`TODrawMenuPlateXYWHType` (`0x008f5f70`, module `plate`) draws the menu
plates of types 1-14 from four kinds of pieces, all sized through the HD UI
remap:

- **Bands.** A band is three `TODrawCrossBoxXYWHC2` boxes over the
  plate's columns: a left cap `rx(cap)` wide, the middle, and a right cap.
  The caps fade to their edge colours.
  - The banded types are 1 and 2 (caps 300 and 140), 4/5 (160), 7/14 and
    11 (140), and 13 (200).
  - They draw a drop shadow under the plate (6 or 4 high), the tinted
    body, a white highlight one line down and a dark line at the bottom.
  - Types 7 and 11 add a title strip after the body; type 4 adds an icon
    at the top right.
- **The sheet image.** This is shape -4 with UVs `1920 × w / 512` and
  `1080 × h / 416`. Types 1 and 2 cut it into the three columns; types 9
  and 10 stretch it over the plate.
- **The nine-slice frame.** Eight atlas pieces with 7-unit corners, used
  by types 9, 10 and 12.
- **The `0x008f9c20` and `0x008f4df0` panels.** Types 3, 6 and 8 use the
  first. Types 9 and 10 use the second; type 10 picks its colours by
  `graphicGetAbmap`, and draws nothing when the box is narrower than
  `rx(14)`.

The port keeps the original's rounding: sums are formed in `f64` (the x87
at 53 bits) and stored as `f32` where the original stores a dword.
`tools/x87-lift.py` produced the exact first draft that the readable
version was checked against.

`TOMkpShapeXY` (`0x00903a10`) and `TOMkpShapeXYWHC2` (`0x00903b70`) only
pass their arguments (and grey colours) to `TOAdpEasyShapeXYWHC2`. They
compute no corners, colours or UVs, so they remain packet glue.

### Turned shapes and the battle network window

`TOMkpRotateShapeXYWHUVC2` (`0x00902ef0`) turns a shape quad by an angle
about its centre. The corners `centre + R(∓w/2, ∓h/2)` use
`_CIcos`/`_CIsin` of the angle and are passed through the remaps. The four
vertices go to `graphicDrawUIElement(rec, texture, 1, 4, 0)` without
clipping. Shape -1 divides the UVs by image 0x3e80's size; the other
shapes pick their sheet as `TOMkpShapeXYWHUV` does.

`TODrawBtlNetWindowXYWHType` (`0x008f41b0`) is a nine-slice frame of
shape 0x194 cells. The corners are `X2(8)` × `Y2(8)`, and the edges span
the box less `X2(16)`/`Y2(16)`. The u edges are the image floats at
`0x00b66bf0`, `…bf4`, `…bfc`, `…c00`. The v edges come from one of two
strips:

- plain: `0x00b41cb8`, `0x00b66c1c`, `…c20`, `…c24`;
- pulsing: `0x00b66c30`, `0x00b66914`, `0x00b5ee18`, `0x00b66c38`.

Kinds 0/2 draw the border from the plain strip. Kinds 1/3 draw it from
the pulsing strip in grey, with alpha `trunc(cos(time × 0x00b2c5c8) × 32 +
96)`. Kinds 0, 1 and 4 fill the centre.

## UI boxes

- `TODrawCrossBoxXYWHC2` (`0x008f4b20`) draws an untextured gradient box
  with float corners `x, y` and `x + w, y + h`. `BeginScissor_XY` clips
  it, and the colours run from `c0` at the top left to `c1`.
- `TODrawBoxXYWHC2` (`0x008f3db0`) and `TOMakePktBoxXYWHRGBA`
  (`0x008fb150`) are the integer forms. Their corners are wrapping integer
  sums converted with `fild`. `TODrawBoxXYWHC2` is skipped while the
  ability map is up.
- `0x008f8bb0` draws one cell of sheet 0x2ecc. The cells are 170 × 14
  pixels, nine rows per column, and the box is not clipped.

All of these draw through `graphicDrawUIElement`. The record words the
originals leave unset (UVs of untextured boxes) are zero in the port.

## Packet buffer kicks

`g_TOPktBuff` (`0x018716c0`) holds the buffer's start, a second address
and the limit, all set by `TOSetPktBuffAdrs`; the write cursor is
`g_TOPktCursor` (`0x018716cc`).

- `TODrawKick` (`0x008f5bc0`) sends the buffer once the written part
  (cursor less start, rounded down to 16 and compared signed) exceeds 0x20
  bytes: `FlushCache(0)`, `sceGsSyncPath(0, 0)`,
  `sceDmaSend(g_TOPktDmaChannel, start)`, `sceGsSyncPath(0, 0)`. On PC
  these PS2 stand-ins are empty (`ret`, or `xor eax, eax; ret`), so the
  kick does nothing visible; its `eax` is 0 after a send and the rounded
  size otherwise.
- `TOCtrlPkt` (`0x008f3a30`) jumps to `TODrawKick` while `g_TOAddPkt` is
  set and the cursor has reached the limit (unsigned). It leaves `eax` as
  the caller had it when packets are not added, and no caller reads it.

## 16×12-font texts with the cross-menu pen

`TOAddPktBaseMesFontLAbmap` (`0x008e8fb0`) lays a text out with the same
pen as `TOMkpCrossExtMesFontLClutTypeRGBA` (`0x025d09a0`: colour bytes,
x, y, then the kind byte at `+0x1a` and the mode byte at `+0x1b`). It
starts the packet cursor at its first argument and returns the cursor.
The position is given as integers and stored as floats.

- A new line puts x back and adds `0x00b313a0` (16.0) to y.
- Code 7, the `0xa` mode nibbles and the `0xb` button icons work as in
  the cross-menu form. The icons are drawn at scale `0x00b5edf0` (0.65)
  at `y + 3.0 + AdjustButtonIconY`, and the pen then moves by the icon
  width plus 1.
- Glyphs are skipped when the kind is 1. Otherwise they go through
  `TOMakePktFont1612Shadow` and `TOMakePktFont1612Font`, with passes 4
  and 5 when the last argument is positive and 0 and 0 when it is not.
  The advance uses scale `0x00b5ee10` (0.78).
- Macros go to `TOAddPktChangeMacro` (`0x008e9b80`, the Switch name).
  That is `TOAddPktChangeMacro_help` for this font: a kind-1 pen draws the
  edge and `TOMakePktFont1612Font_00906270` and advances at `0x00c5d784`.
  Other pens use the shadow and face with pass 0 and advance at 0.78.

The original first calls `graphicFontGetScreenWH` (`0x00640db0`). That
stores 1920 and 1080 into locals nobody reads, so the port leaves it out.

`TOGetChangeMacroAdvance_008b6bb0(key, code, kind, scale, extra)` adds up
the width of a macro's text. Nested macros add their own width, code 7
with 0xe1..=0xef adds its byte less 0xe0, and any other code adds its
glyph's `TOGetFontAdvance_008fa740(kind, glyph, scale)`. The total is an
`f32`, rounded after each step. The result is the total less
`TOGetFontSpaceInPiexl(kind)`, or 0 when there is no macro.

`TOGetMessageWH1612_008b6cb0(window, text, &w, &h, scale, extra, kind,
flags)` (`0x008b6cb0`, callers `TOMenuDrawYesNoWindow` and `0x008bc100`)
measures a message text in this font. It returns `&h` in `eax`. The line
width is an `f32`, rounded after each step:

- Glyphs add `TOGetFontAdvance_008fa740(kind, glyph, scale)`. For a
  window whose `+0x10` is 0x4000 the glyph first goes through
  `TOConvAlbhedWord`.
- Macros add `TOGetChangeMacroAdvance_008b6bb0`.
- Code 7 with a byte 0x40..0xbf sets the width to `4 × byte - 0x100`.
  With 0xe1..=0xef it adds its byte less 0xe0. `7 0x38` sets the window's
  `+0xe` when its `+0x24` is 0.
- `0xe 0x40` opens a padded run and `0xe 0x41` closes it, adding 5 to a
  positive width. Kind 3 starts with a run open.
- `0xb n` adds `TOGetButtonIconWidth(n)`.
- `0x10 n` (with a window) for n 0x50..0x5f measures the window's counter
  (`[0x01868a90 + 4 × window] + 0xc`) as `TOIntToMes` text of n - 0x4f
  digits, adds its width and sets the window's `+0x1e`. Without flag bit 8
  any n other than 0xff also sets `+0x1e`.
- A line end (codes 0..3) adds 5 to a positive width inside a run, keeps
  the widest line and clears the width.
  - Code 0 ends the text.
  - Codes 1 and 2 start a line of height 16 and end the text when the
    flags' low nibble is set.
  - Code 3 adds 16 to the line height. When the nibble is 3 and the line
    is now the tallest, it ends the text.
  - The tallest height is taken at codes 0, 1 and 2, and at code 3 with
    nibble 3.

`w` is the widest line less `TOGetFontSpaceInPiexl(kind)`, and `h` is
`(tallest + 2) × scale`.

`TOBtlGetChrName` (`0x008976f0`) is a tail jump to `TOGetSaveChrName`
(`0x008ac850`).

`TOCountMessageWordsAbove_008b8140(window, text, limit)` counts words the
way `TOCountMessageWords_008b80b0` does. It stops at the first byte at or
below `limit` (compared signed) rather than at the first line code.

`TOGetMessageSizePacked_008b7170(text)` measures the text with
`TOGetMessageWH(0, text, &w, &h, 0, 0)`. It returns the `i16` height
shifted up 16 bits, OR-ed with the low half of the width's inline
`fistp qword` truncation. That low half is 0 for NaN or out-of-range
widths, and a negative width sets the upper bits too.

## Battle message openers (`TODrawAMES*`)

Four openers each do the same steps on a message slot:
`TOInitMessageSlot_008ada30`, `TOSetWindowKindAndType_008b0000`,
`TOSetMessageWindowTexts_008b89d0` (the text twice),
`TOSetMessageWindowPosition`, `TOSetMessageWindowValue(…, 4)`,
`TOSetMessageWindowActive` and `TOOpenWindowAndMessage`. They return the
last call's `eax`.

| Function | Kind, type | Position (anchor, x, y) |
|---|---|---|
| `TODrawAMES` (`0x0089b140`) | 0, 0 | arguments |
| `TODrawAMES_CENTERMES` (`0x0089b1d0`) | 1, 0x2402 | 4, 0x100, 0xe0 |
| `TODrawAMES_EVJIMAKU` (`0x0089b230`) | 1, 0x2002 | 4, 0x100, 0x1c0 |
| `TODrawAMES_EVJIMAKU_ALBHED` (`0x0089b290`) | 1, 0x6002 | 4, 0x100, 0x1c0 |

On map 0x185, `TODrawAMES` moves a message at (0xfa, 0x96) up to y 0x8c
when the language is 2..=5. The last three names are inferred from the
Switch build's five `TODrawAMES*` names; their position and type bits
fit the names.

`ToMakeBtlEasyEdgeDigitRight(value, x, y, alpha, scale, extra)`
(`0x00905890`) right-aligns a number at `x`. It counts the digits of
`value` (1 for 0, the sign not counted) and gets the width from
`ToGetBtlEasyDigitWidth`. It then draws with `ToMakeBtlEasyEdgeDigit` from
`x - width`, rounded to `f32`. The original stores the width in its own
`value` argument slot.

## Battle help text

`TOBtlSetDirectHelpMes(text)` (`0x00899ec0`) clears the 0x80-byte help
buffer `g_BtlHelpText` (`0x0133f538`) and copies `text` into it up to its
terminator. The copy is not bounded. It then measures the buffer with
`TOGetMessageWH(0, buffer, &g_BtlHelpTextW, &g_BtlHelpTextH, 0, 0)`
(`0x0133f534`, `0x0133f530`). A null text does nothing.

`TOBtlSetCommandHelpMes(command)` (`0x00899970`) looks the command up with
`MsGetComData(command & 0xffff, &strings)`; the help text is at
`strings + u16 [entry + 8]`. It measures the text where it is, then copies
and measures it again the same way.

`TOMkpBtlDigitRight_008f3300` and `_008f3420` (`ret, value, x, y, alpha`)
call `ToMakeBtlEasyDigitRight(value, f32(x) + dx, f32(y) + dy, alpha,
scale, 0.0)` and return `ret`. Each sum is rounded to `f32`; `dx` is a
double.

| Function | dx | dy | scale |
|---|---|---|---|
| `0x008f3300` | 4.0 | -3.0; -2.0 for languages 1..=8 and from 12 | 0.8 |
| `0x008f3420` | 0.0 | -5.5 (language 0), -6.0 (9), else -5.0 | 0.77 for language 0, else 0.8 |

## Play time, cross icons, info window (round 28)

`TOMenuDrawTimer(frames, x, y, alpha, kind)` (`0x008e19f0`) draws a play
time right-aligned at `x` in the battle edge font, scale 0.63 and 1.5
(`0x00b66864`, `0x00b3eb58`): the minutes (`frames / 60 % 60`, `"%02d"`),
a `J` (0x4a, the font's separator) and the hours (`frames / 3600`, at
most 999). Each piece moves left by its width (`ToGetBtlEasyFontWidth`,
`ToGetBtlEasyDigitWidth2`) and one letter space
(`TOGetFontSpaceInPiexl(0)`), two for languages 1..=8. The separator is
drawn only for `kind` 0x25, with 0x25 as its alpha. Every piece's `y` is
`y - graphicUiRemapY2(9.0)`. Sums are doubles, each position rounded to
`f32`.

`DrawCrossCtbIcon(id, x, y, w, h)` (`0x0089e660`) draws battle icons
from the cross-menu sheet with `TOMkpShapeXYWHUV(400, ...)` after
`setButtonData(-1, id)`. Only ids 0x1b6, 0x1b7 and the ones it does not
know end with `resetButtonData()`. UVs are texels over 1024.

| ids | quads |
|---|---|
| 0x1b8..=0x1c2 | one: `u0 = (id % 2 × 129 + 768)`, width 120; `v0 = (id - 0x1b8) / 2 × 96`, height 80 |
| 0x1c3..=0x1ca | a frame, then `x + 0.3w, y + 0.2h, 0.4w × 0.6h` with `u0 = (id - 0x1c3) × 31 + 769`, width 30, `v` 0.8105..0.8555 |
| 0x1cb..=0x1d2 | a frame, then `x + 0.52w, y + 0.55h, 0.288w × 0.4h` with `u0 = (id - 0x1cb) % 4 × 60 + 769` (+1 for 0x1cf), width 47; `v0` 0.873 below 0x1cf else 0.9316, height 50 |
| 0x1d3, 0x1d4 | the frames alone |
| 0x1b6, 0x1b7 | two arrow halves, sized `remapX2(14) × remapY2(15)` and `remapX2(24) × remapY2(12)` |

The multipliers are doubles of `f32` values (0.3 is `0.30000001192…`).

`TOBtlDrawInfoWinMes()` (`0x00895140`) draws the battle info window's
four lines (`g_BtlInfoWinLines`, `0x0133f618`, 0x1c bytes each: `+0`
`i16` shown, `+0xc`/`+0x10` position, `+0x18` text) with
`TOMkpCrossExtMesFontLClut(0, text, x, y, 0, 0.78, 1.0)` inside
`OpenFontScissor` of the `i16` box at `0x0133f694` (x0, y0, x1, y1 at
4-byte steps), then `CloseScissor`. It draws nothing while libra's auto
mode (`0x0133d6e0`) is 1, `g_BtlInfoWinShown` (`0x0133c93d`) is clear or
bit 0x400 of the menu settings (`MsGetSaveMenu`) is clear.

## Battle help window and command pads (round 28)

`TOBtlDrawHelpWin()` (`0x008936d0`) draws the help band
(`TOMkpShapeXYWHUVC2(-1, ...)` at `remapX2(145), remapY2(96)`, size
`remapX2(1600) × remapY2(55)`, grey 0x80808080/0x808080) and its frame
(`TOMkpShapeXYWHUV(0x3e, ...)`, `remapX2(95) × remapY2(22)` at y
`remapY2(78)`) inside `TODrawScissorXYWH(0, 0, 0x200, 0x1a0)`, then the help
text `g_BtlHelpText` with `TOMkpCrossExtMesFontLClut(0, text, x, y, 0,
0.78, 1.0)` at `remapX2(235)`, `remapY2(101)`. Nothing is drawn while
libra's auto mode is 1 or menu bit 0x400 is clear; while the window is
closed (`g_BtlHelpWinOpen` not 1) it only resets `g_BtlHelpScroll`
(`0x0133c940`).

A text wider (`ToGetCrossExtMesFontWidth`) than `remapX2(435)` scrolls
when the info window is shown or libra's auto mode is not 0. Inside
`OpenFontScissor(40, 40, 178, 62)`:

1. `scroll -= 0.8` (stored as `f32`).
2. The text is drawn at `x + scroll`.
3. While `x + scroll + width < 158`, it is drawn again at `x + scroll +
   width + 20`.
4. Once `x + scroll + width < 40`, `scroll = (width + 20) + scroll`.

The sums are doubles of `f32` values, in that order.

Battle command list windows (`w`, the 0xf0-byte levels at `0x0133c900`)
take the TO pad words at `0x025d09d2` (held, triggered `+2`, repeated
`+4`):

| offset | field |
|---|---|
| `+1` | `u8`, stepped when going back a level |
| `+8` | `i16` actor |
| `+0xa` | `i8` picks needed |
| `+0xb` | `i8` picks made |
| `+0xe` | `i16[]` cursor of each pick |
| `+0x16` | `u16[]` command of each pick |
| `+0x1e` | `u16` command under the cursor |
| `+0x20` | the window's `i16` command list |
| `+0x38`, `+0x3a` | `i16` columns, rows shown |
| `+0x40`, `+0x42`, `+0x44` | `i16` first shown entry, cursor, entries |
| `+0x88` | step function (`0x00898e40` idle) |

`TOGetPadStatusPrepare(w)` (`0x0089d610`) runs while the battle windows
are open and the window is idle. Cancel (0x40) undoes the last pick
(slots back to 0xff, `MsSetUseItem(0)`, click 4) or goes back a level
(`TOBtlSetPrevAction`, click 4). Confirm (0x20) asks
`TOCheckBtlCommandUse(actor, command)`: a usable command is recorded as
a pick; with more picks needed `MsSetUseItem(first pick)`, else
`TOBtlSetNextAction(actor, command)`. Otherwise, unless opposite
directions are held, up/down move by a row and wrap into the list,
left/right move by one and clamp. The first shown entry follows the
cursor by a row, with `TOBtlPadScrollDown`/`Up` as the step function,
and is clamped to the list.

`TOBtlGetLeftPad(w)` (`0x008a22f0`) and `TOBtlGetRightPad(w)`
(`0x008a2690`) are the same function with cancel on 0x2000 and 0x8000.
Their confirm checks the current pick's command. 0x10 uses the
character's skip command (`MsMenuGetChrSkipCommand`): `g_BtlCommandRec`
(`0x023cc040`) gets the actor, the command, target 0xff and the actor's
bit (also in `0x0133d178`), `MsSetBtlCommand` takes it, and the windows
close (`TOCloseBtlWindow(g_BtlWindowChr)`, `TOBtlCloseHelpWin`,
`TOSeqInitAllScene(2)`, `g_BtlWindowChr = 0xff`). 0x04 with no aeon out
calls `TOBtlSetNextAction(actor, 0x3002)`. Going back a level clicks 1
here. Only up/down move, and the scroll does not change the step
function.

A zero column count makes the originals divide by zero; the ports take
the quotient as 0.

`MenuItem_ApplyHeal_008c8c40(target)` (`0x008c8c40`, from
`TkMenuCtrlItem`) heals with the item in use (`u16` at `0x0186a3a8`):
`target` alone when `0x0186a391` is 1, else every aeon and then every
party member its record's `+0x22` bits 0x80 and 0x40 select. Each gets
`TkMenuItemHealAmount_008c69a0(item, who, 0)` through `TkMenuAddHPCapped`.

`DrawWaterWaveShapeC2(x, y, w, h, u, v, uw, vh, su, sv, c0, c1)`
(`0x008e7d30`) draws a quad of the water-wave sheet (shape 0x3e81) whose
texture drifts with `graphicGetTime()`. Each coordinate is
`frac((time × speed + start) / sheet size)`, with the time read again for
`v`; every step is rounded to `f32` and the fraction subtracts
`_ftol2_sse`'s integer. `u1 = u + uw / width` and `v1 = v + vh / height`.
The colours `c0` (top-left vertex) and `c1` go in byte by byte. The
`graphicDrawUIElement(record, texture, 1, 0, 0)` record has `+0x90` and
`+0x94` zeroed; its other unused words are stack leftovers in the
original.

## Fitted command labels (round 29)

`QuickRecoverMenuDraw(text, x, y, w, alpha, scale, extra, align, scroll,
slot)` (`0x00904ba0`, called only by `TOBtlDrawCommandWindow`) draws a
battle-font label (`ToMakeBtlEasyEdgeFont`) into a box `w` wide. `align`
1 centres it at `x` (by half of `w` when the text is wider), 0 starts it
at `x`. Any other value leaves the left edge uninitialised (stack
garbage).

A text no wider than `w` is drawn as is. A wider one:

- With `scroll` 1 it scrolls in marquee slot `min(slot, 3)`. State is
  `g_QRScrollOffset` (`0x01876110`), the text's xor hash at `0x01876120`,
  `g_QRScrollPhase` (`0x01871708`) and a frame count at `0x01876130`. A
  text whose hash's low nibble changed restarts it. Phase 0 waits 51
  frames at offset 0, phase 1 slides 0.5 a frame until the text's end
  meets the box's (`w - width`), and phase 2 waits 51 frames. The text is
  drawn at `left + offset` and again 10 past its end, inside
  `OpenFontScissor(left, y - 2, left + w, y + 20)`.
- Otherwise it is cut. The ellipsis is `HHH` for languages 1..=8, else
  `>`. The text is written glyph by glyph through the TO font cursor at
  `0x025d09a0` (colour 0x80808080, `+4` x, `+8` y, `+0x1b` alpha;
  `TOMakeBtlEasyFontEdge`, `TOMakePktBtlEasyFont`, `TOAddFontWidthSpace`).
  Group 1 codes go to `x`, 16 lower. The first glyph whose advance passes
  `left + w - ellipsis width` stops it. A `G` (0x47) there is narrowed by
  the `G` glyph's width until it fits and is drawn. The ellipsis follows
  at the cursor. A text that ends first gets no ellipsis.

## The battle command window (round 30)

`TOBtlDrawCommandWindow(w)` (`0x0089f370`, cdecl, returns 0;
`crates/ffx-system/src/cmdwin.rs`) draws the command, skill and item
lists of a battle. The TO window fields it reads:

| Offset | Type | Meaning |
|---|---|---|
| `+0x08` | i16 | acting character |
| `+0x20`, `+0x24` | u16 *, i32 | command IDs, their count |
| `+0x28` | i16 | parts per row |
| `+0x2a` | i16 | 1: show counts of 0 or less |
| `+0x2c` | byte pairs | the parts; the kind is the second byte |
| `+0x38`, `+0x3a` | i16 | columns, visible rows |
| `+0x40`, `+0x42` | i16 | first visible entry, cursor |
| `+0x62`, `+0x66` | i16 | cell width, label width |
| `+0x68..+0x74` | f32 x 4 | box (x0, y0, x1, y1), the scissor |
| `+0x7c` | f32 | rows' top |
| `+0x88` | code | the window's own draw step, called with `w` |
| `+0xdc`, `+0xe0` | f32 | when it opened, time since (written) |
| `+0xe4` | u32 | labels after an icon (non-zero) or centred (0) |

All sizes are PS2 units through `graphicUiRemapX2`/`Y2`, except the cell
width `+0x62`, which is used as is.

**Panel animation** (two or more columns). `+0xe0` is
`graphicGetTime() - +0xdc` when `+0xdc > 0`, else 0.

- Cursor plate, when `cursor - first` is in `0..3 cols`: the sheet
  (`TOMkpShapeXYWHUVC2` shape -1) at `X2(154) + col × +0x62`,
  `Y2(835 + 52 row)`, `+0x62` × `Y2(50)`, UV 1619, 105 to 1920, 156. Its
  alpha is `trunc(cos(5 t) × 32 + 96)`, the cosine rounded to f32.
- Three plate rows `k` at `t = +0xe0 - 0.1 k` (each step rounded to f32).
  A row's plate slides to `((X2(154) + +0x62) t / 0.1 - +0x62`, held
  between `-+0x62` and `X2(154)`: `Y2(835 + 52k)`, `+0x62` × `Y2(50)`,
  UV 0, 0 to 438, 50, colour 0x60808080. Its gloss strip at the same x:
  `Y2(838 + 52k)`, `X2(234)` × `Y2(44)`, UV 1553, `53k + 270` to 1788,
  `53k + 315`, 0x40808080. A second plate slides to
  `(X2(154) + 2 × +0x62) t / 0.1 - +0x62`, held up to `X2(154) + +0x62`.
- From `+0xe0 ≥ 0.3` (the f32 0.3), the ripples: eight
  `DrawWaterWaveShapeC2` pieces per row, half a cell wide (`+0x62 × 0.5`)
  and `Y2(3)` high, `uw` 210, `vh` 3, speeds 40. Two strips, at
  `Y2(835 + 52k)` (`v = 20(k + 1)`) and `Y2(880 + 52k)` (`v = 50(k + 1)`).
  Each strip has two tiles: the left tile at `X2(154)` and `+ ½` cell,
  with `u` 0 and 210, and the right tile at `+ 1` and `+ 1½` cells, with
  `u` `20k` and `20k + 210`. A tile's first piece fades from 0x00808080
  to 0x40808080, the second back. Pieces go tile by tile, upper strip
  first.

**Frame.** `DrawCrossMenuScrollParts(X2(150) + +0x62, Y2(835), X2(8),
Y2(154), first / cols, rows, lines)`. `lines` is
`(count - 1 + cols) / cols`, its low word sign-extended. Then
`TOMakePktScissor` of the box, each edge truncated (`ftol2_sse`), and
`+0x88(w)`.

**Rows.** Entries run from `max(first, 0)` (the count when `first` is
past it) for `cols × rows`, capped at the count. An entry of 0xff, or
one `TOCheckBtlCommandUse(chr, id)` answers -4, is skipped. The cell's
`x` is `(i % cols) × +0x62 + trunc(+0x68)`. Its `y` is
`trunc(Y2((i - first) / cols × 52) + +0x7c)`. `MsGetComData(id,
&strings)` gives the record, and the label is `strings + record[0]`.
The parts, by kind:

| Kind | Draws |
|---|---|
| 1 | `DrawCrossMenuIconXYWHRGBA(X2(37) + x, Y2(3) + y, X2(29), Y2(36), record[0x14], 0x80 x 4)` |
| 2, 4, 5, 6 | `MsGetSaveItemNum(id)`, `MsGetCommandMP(chr, record)`, `MsGetRamChrHP(id)`, `MsGetRamChrMP(id)`: when above 0 or `+0x2a == 1`, `ToMakeBtlEasyDigitRight(n, (x + +0x62) - X2(28), y, alpha, 0.78, 1.0)` |
| 7 | `MsGetSaveWeapon(id)` record `p`: icon `2 p[4] + 1 + p[5]` at `X2(37) + x`, `Y2(2) + y`; when `p[4] == p[6]` icon 0x31 there too |
| 0x10 | `QuickRecoverMenuDraw(label, x', y, +0x66, alpha, 0.78, 1.0, align, cursor row, 0)`. With `+0xe4` set, `x' = X2(72) + x` and `align` 0; otherwise `x' = +0x62 × 0.5 + x` and `align` 1 |

Other kinds draw nothing. The counts' alpha is the low byte of one word
of the original's frame. The panel's loops leave 0x9c (156) in it, or
after the ripples the bits of 150.0 (low byte 0). A label sets the low
byte to 1 when `TOCheckBtlCommandUse` was negative, else 0, and passes
the whole word as its alpha. With one column, nothing sets it before
the first label: the counts then get stack garbage.

## Drawing a scene's sequences (round 30)

`TOSeqDraw(i, a, b)` (`0x008ae2f0`, cdecl) walks the eight slots of
scene `i` in `g_TOSeqSlots` (`0x01865f20`, 0x100 per scene, 0x20 per
slot). Each slot in state 2 or 3 runs `TOMenuExecLayerHandle(i, a,
handle, b)` (`0x00a7b220`) on its handle (`+0x1c`), in slot order. The
original leaves in `eax` the last slot's call result, or that slot's
state when it is not drawn; the port returns the same.

## Swapping the international environment (round 31)

A language change reloads the language files from the save region's
archive (`0x00c59564`: 4 for English, 0x12 otherwise, set by
`TOSwapInternationalEnvInit`). `TOSwapInternationalEnvExec`
(`0x008b0340`) does it at once with `TODVDFileRead`; `...ExecNonBlock`
(`0x008b0500`) spreads the same reads over frames with
`TODVDFileStartRead` and the CD-busy check `0x008abb00` (CD table
`+8(1)`). Order: file 0xb into `[0x01841e00]`, 0xa into
`TOGetBtlMenuTexAdrs()`, 0xf into `[0x01841d9c]`, 0xe into
`[0x01841d98]`, 8 into `[0x01841e08]`, 6 into `0x025d0d20`, 5 into the
Al Bhed table, 7 into `0x025d0dc0`; `TOSeqInitAllScene(2)`; file 0x16
into `0x01841e10` and `TkMenuSetSceneDataAdrs(2, 0x01841e10)`;
`TOSeqInitParam()`; the battle data (`MsBattleReadData`, or file by file
with `MsBattleReadDataNB(n)` until it returns 0, waiting for `GetCD()`'s
`+8(1)` after each). State: step `0x01841d28` (0..0x1c, compared
unsigned; others do nothing), done `0x01841d2c` (the return value),
busy `0x01841d30`, battle file `0x01841d34`. Step 0x12 falls through
into 0x13; step 0x1c prints "Done" every time it runs. With
`g_DebugPadPrintFlag` the step and file count go to the debug screen.
`...ExecMenu` (`0x008b0440`) reads file 0xd of `0x00c59978` into
`0x0157f130` with its own step (`0x01841d38`, 0..6; 4 and 5 do nothing)
and done flag (`0x01841d3c`).

`TOTransTexture` (`0x008b1020`) then uploads the textures with
`TOTransImageHL(data, dest, format, 0, 0, w, h)` from the texture table
(`0x01841da0`, 16 bytes each: data, CLUT, size, `i16` w, h): entry
`TOGetInternationalBaseDicType()` (not bounds-checked) to 0x3c00; out of
battle entry 1 to 0x3d00 when `AtelGetExpandFTCAdrs()` is non-zero; in
battle entry 2 to 0x3d00 when `MsBattleGetFontAddress()` is non-zero and
entry 4 to 0x3d40 for language 0; then fixed-size images: `[0x01841d94]`
0x80x0x100 to 0x3d80, `[0x01841d98]` 0x100x0x80 to 0x3dc0,
`[0x01841e00]` and the battle-menu texture 0x100x0x100 (format 0x13) to
0x3e00 and 0x3f00, `[0x01841e08]` 0x40x0x40 (format 0) to 0x2ea0. The
menu's form `0x008b1190` uploads entries 0 and 4 for language 0 (else
the dictionary's), entry 3 when `GetU32_0112a010()`, `[0x01841d9c]`
0x200x0x80 to 0x3f00, and the last two only while no reload is busy.

## Decoding a compressed CD file (round 31)

`cd_decode_data(req)` (`0x0076d440`) unpacks a file in place: `req +4`
the data, `+0x2c` its size; the unpacked size is the unaligned `u32` at
data `+1`. Files up to 0x1800 bytes are copied whole to the read buffer
(`[0x01128ac8]`) and `sa_decode(data, buffer)` unpacks them back. A
larger file keeps its last 0x1800 bytes in the buffer, moves its first
`size - 0x1800` bytes up, byte by byte from the end, to end at data `+
unpacked`, and the moved block's first byte (the file's first byte)
names the method: 1 `sa_meth01dec_ex(block, data, n, buffer)`, 2
`sa_meth01dec2_ex` (both stand-ins on PC that only print), other values
nothing (`eax` is then the byte less 2).

## The battle command list's pad (round 31)

`TOBtlGetStdPad(w)` (`0x0089cde0`) is the item/skill list's frame, a
sibling of `TOGetPadStatusPrepare` and the side pads. It runs only while
`g_BtlWindowOpen` and with the idle step function (`+0x88` =
`0x00898e40`). Before the d-pad, by the trigger word `0x025d09d4`: at the
top level (`g_BtlActionDepth` 0) 0x10 takes the skip command
(`MsMenuGetChrSkipCommand`; unlike the side pads it clicks 1 before the
command record is written, and clears `g_BtlHelpWinOpen` itself instead
of calling `TOBtlCloseHelpWin`); 0x04 asks for action 0x3002 (no summon
out); 0x8000 opens the left window (`MsCheckLeftWindow` above 0, action
0xffff) and 0x2000 the right one (action 0xfffe) when
`MsGetSaveWindow(actor, 0xd, &n)` lists anything once its trailing 0xff
entries are dropped (a negative `n` counts as a list). The d-pad works as
in `TOGetPadStatusPrepare` (only exact repeated words move it), then the
pick is recorded before the buttons are read again: Libra's auto mode
(`0x0133d6e0` 1) forces the trigger to 0x40 and sets `0x0133d6e4` unless
it or `g_BtlCommandMissed` is 1; a refused confirm (`TOCheckBtlCommandUse`
negative) clicks 3 and sets `0x0133d6e4` too.

## Laying out a message's choices (round 31)

`TOLayoutMessageChoices_008b9a60(w, sel)` (cdecl) walks a message
window record's text (`w +0x94`, window `i16 +4`, mode `i16 +0x1a`) with a
pen (x an `f32` from 0, y from 0x10) and fills the window's line record
(`0x01868a90[i]`): `+4` and the item count `+5` cleared first, item
`a - 0x30` of a `0x10 a` code at `+0x38 + 8 × (a - 0x30)` (`+0` flag 1,
`+2` width, `+4` x, `+6` y, both truncated with `_ftol2_sse`). An item's
width is the pen's distance from its x when the next item or line code
comes; `0x10 0xff` only closes. `0x10 0x50..0x5f` is a number: `[line
record +0xc]` through `TOIntToMes(value, text, a - 0x4f, 1)`, measured,
and set with `TOSetMessageChoice(i, a - 0x4f, x, y, width)`. Line codes
(group 1) put x back to 0, or centre it for a centred window (`+0xe` 1:
`(right +0x50 - left +0x48 - text width) × 0.5`); code 0 ends, 1 and 2 go
back to y 0x10 and end for `sel & 0xf` non-zero, 3 steps y by 0x10 and
ends for `sel & 0xf` 3. Code 7 aligns (0x38 and 0x31 centre the rest of
the line, 0x40..0xbf put x at `4a - 0x100`, 0xe1..0xef step x by `a -
0xe0`), 0x0b adds a button icon's width, macros (group 3) add
`TOGetChangeMacroWidth` and characters `TOGetFontWidthInterMode` (after
`TOConvAlbhedWord` in an Al Bhed window, `+0x10` 0x4000). At the end the
record's `+8` is the widest item; on map 0x17, for the record with `+0x14`
0x1c7 and `+0x16` 199, the items are stacked instead (x 0, y `+ 21.5k`).
`eax` is the item count, or the line record when there is none.

## Small helpers (round 31)

- The CTB window's line animation runs four 4-frame steps through the
  record's step function (`+0x14`): `TOBtlCtbAnimStep_008a2ad0` (+4, +7,
  +11, +16 to `+0x10`), `TOBtlCtbSlideStep_008a2b10` (a quarter of the
  row change from `i8 +0xd` to `+0xc`, 0x16 per row, to `+0x12`, then
  exactly on the row), `TOBtlCtbAnimStep_008a2b60` (-16, -11, -7, -4),
  then the step function is cleared and a record whose row is 0x10 or
  more switched off (byte `+0`). `Tick4_008a2ba0` waits four frames before
  the same slide (`0x008a2bc0`) and wait (`0x008a2c10`). Steps outside
  0..4 read the original's stack table out of range; the ports add 0.
- `cd_channel_state_0076d0e0(file, archive)`: the first of the 64 CD
  channels (`g_CdChannels`, 0x38 bytes) in use (`+0x31`) for that file
  (`+0x28`) and archive (`+8`): 2 while it is the one being read
  (`g_CdLib +0x70` set, `+0x74` its index), else 1; 0 for none.
- `0x009066d0(x, y, k)` and `0x009066b0(u, v, k)` set vertex `k` (0x20
  bytes) of the UI element record at `0x025d1700` (position at `+0`,
  texture coordinates at `+8`); `eax` is `k << 5`.
- `TOMenuTransFacePlyTex` uploads `0x01c80000` (0x200x0x100, format 0x13)
  to 0x3a00. That push has no relocation entry: the address is literal
  even when the image is relocated.

## A character's status plate (round 32)

`TOMenuDrawChrStatusPlate_008ff490(chr, x, y)` (cdecl, `x`/`y` floats)
draws one character's plate in the menus (the callers are the aeon grow
window `0x008cdba0` and two other menu screens). Sizes go through
`graphicUiRemapX2`/`Y2`; positions are `x + rx(dx)`, `y + ry(dy)` added
at x87 precision and stored as dwords. In order:

1. Background. Party members (`chr < 8`, signed) whose bit is set in
   `TOMenuGetYesNoWindow()` get sheet shape -3 at UV (0, 0) and the
   water: four 250x64 `DrawWaterWaveShapeC2` strips (U 0 and 250 scrolling
   (50, -20) with alpha 0x1a, U 300 and 550 scrolling (-80, -10) with
   alpha 0x33, blue 0xc56870 fading to clear across each pair). The
   others get shape -3 at V 700. Then the right panel (shape -3, 415x70
   at `rx(1090)`). Aeons and higher numbers get sheet -6 (1110x70), the
   panel, then the water.
2. Numbers above 0x11 (unsigned, so negatives too) get only the empty
   labels and frames (shapes 0x21, 0x22, 0xc8, -1, 0x3d at slightly
   different offsets: 842, 1034, `y - ry(5)`, the AP label at 948).
3. An aeon whose `TkMenuGetHP` is 0 or less gets
   `TkMn2DrawSummonPlateCheckHP(ftol(x), ftol(y), chr)` (`ftol(y)` taken
   first). Everyone else gets the name (`ToMakeBtlEasyEdgeFont`, alpha
   from `MenuPlayer_DamageClass2`, scale 0.82, extra 1.0), the overdrive
   label (0x21), `HP/max` as `"%5d/%5d"` through the CRT `sprintf` import
   into a 32-byte stack buffer and `TOMkpCrossEasyStrFontSClut`, the
   Sphere Level label (0x22) and `MP/max` as `"%4d/%4d"`.
4. Party members: the gauge frames (0xc8, -1, 0x3d), the AP label, the
   Sphere Levels centred on `rx(1076)` (`ToGetBtlEasyDigitWidth2` writes
   the width into the same stack word that holds the language gap), the
   label 0xc8 and `MsGetNextAP - MsGetSaveNextAP` right-aligned. Aeons:
   two grey `TOMkpShapeXYWHUVC2` frames and
   `TkMn2DrawCrossLimitParam(x, y, rx(350), ry(8), limit, limit max)`.

Language 11 (`TOGetFFXLang`) widens the labels by 22 (UVs follow),
heightens 0x21 by 1, moves the HP/MP numbers 11 right, and moves the AP
label (x -10, U -35, width +25 or +20, U end +20). The water colours are
pushed as literals (no relocation entries), not addresses.

## Round 32's smaller functions

- `sce::Vectormath::Scalar::Aos::inverse(out, m)` (`0x00573d00`, cdecl,
  returns `out`) is the scalar Vectormath 4x4 inverse by cofactors, a
  debug build that stores every intermediate as an `f32`: the six 2x2
  minors of rows 2-3 (`m10 m3 - m11 m2`, `m14 m7 - m15 m6`, `m10 m1 - m2
  m9`, `m14 m5 - m13 m6`, `m9 m3 - m1 m11`, `m13 m7 - m5 m15`), the first
  column's four cofactors, `1 / det` with det summed innermost first
  (`c3 m12 + (c2 m8 + (c1 m4 + c0 m0))`), twelve reused products, the
  six minors of rows 0-1, the other columns' cofactors, then each
  cofactor times `1 / det`. `out` is written only after all reads.
- `SheetCellUV_00684b00(kind, u, v, &u, &v)` (stdcall) maps a 0..1 UV into
  a cell of a 2x4 sheet by kind (0x3700, 0x3720, 0x3740: quarter-height
  cells; 0x3760, 0x37a0: the bottom halves). Where the original adds no
  offset the port adds none either: `-0 * 0.5` stays `-0`.
- `AbmapManager_ResetLayers_006851d0(mode)` (thiscall) on the area-map
  manager: mode 0 negates `+8` and clears words `+4` and `+8` of each
  0x6c-byte entry of the layer lists `+0xd8`/`+0xdc` (new value positive)
  or `+0xd0`/`+0xd4`; mode 1 list `+0x94`; modes 2/3 set `[+0xc8]+0x90`.
  `eax` (which `0x006390d0` returns) is the last list, `[+0xc8]` or the
  mode.
- `LocalizationManager_Initialize` (thiscall) maps the settings' language
  index (`*g_Settings`) to (kind, game language, western): 1..5 give
  (1, 1/2/4/5/3, 1), 6 and 7 (3, 9 or 10, 0), anything else zeros.
- `TOSeqStopExecScene(i)` stops each running (state 2 or 3) slot of scene
  `i` whose `+2` is not 1: state 4, name byte cleared; `eax` is `i << 8`.
- `TOAddPktCommandIconName_008f0590(pkt, command, x, y, colour)` adds a
  command's icon (`TOAddPktMenuIcon` at `x + 0xc, y + 1`) and its name
  (`TOAddPktBaseMesFontLAbmap` at `x + 0x1a, y + 1`).
- `MagicFile_GetEffectOverlayTable` tail-jumps through `[0x022fb6a4]`
  while a magic file is loaded, else returns 0.
- `0x009db230(high)` and `0x009db250(high, low)` return the 64-bit `(high
  << 32) + low` in `edx:eax` (`low` sign-extended; for the first,
  `pppCurrentProgram`).
- `TOSeqDrawLayer(i, a)` is `TOSeqDraw(i, a, 0)`.
- `TOMenuTransFaceSmnTex` uploads the aeons' face sheet `0x01ca0000`
  (0x200x0x100, format 0x13) to 0x3800; like the party sheet's, the push
  has no relocation entry.
- `IdentityAt34_0061a810` (thiscall) sets the 4x4 at `+0x34` to the
  identity.
- `Vu0ScaleXyz_009dbd90(out, s, v)` goes through the VU0 emulation's
  registers: `0x00c8f820 = s`, `0x00c8f830 = v`, its `x, y, z` times `s`
  (rounded to `f32`), then the register is copied to `out`; `eax` is `w`.
- `TOTextScrollTick_008df570(w)` is a text window's scroll clock: `+0xb4`
  counts down first; at 0, `+0xb0 = ftol(Tk_GetScale(english) ×
  f32(+0xac) + +0xb0)`, and each whole 30 (25 in the 50 Hz mode) moves
  into the line count `+0xa0`.

## Round 33

- `updateVFXKeThSetPoint(mesh, i)` (`0x007669c0`) copies point `i` of a
  Ke thing mesh from three pairs of tables (16-byte rows): points 0–5 from
  `0x02305848`/`0x023058a8`, 6–11 from `0x0230f908`/`0x0230f968` (row
  `i - 6`), 12 on from `0x02305908`/`0x0230a908` (row `i - 12`). Three
  floats go to the positions (`[mesh + 0xc] + 12 i`), four to the colours
  (`[mesh + 0x14] + 16 i`, read after the position stores), all through
  `fld`/`fstp`. `i` is signed; `eax` is the colour base.
- `Cluster_UnpackArrayFixups(out, cursor, lists, pointer_arrays)`
  (`0x0060dac0`) and `Cluster_UnpackPointerFixups(out, cursor, lists)`
  (`0x0060db20`) walk `[lists] & 0x7fffffff` 36-byte list headers at
  `[lists + 4]`, handing each list to `Fixup_UnpackArrayList` /
  `Fixup_UnpackList` as `(cursor, records, n, [header + 4])`: `n` is the
  header's `+0x18` (`+0x20` for pointer arrays, the low byte of the flag
  word) or `+0x1c`; `records` starts at `[out + 4]` and advances 16 or 28
  bytes per entry. Both return 0.
- `PcFileName_00a42c00(loc)` builds the PC file name of entry `loc` of the
  CD's name table (`[GetCD() + 0x30]`: a count, then `count + 1` offsets
  from the table) in `0x01a85c30` and returns it (Ghidra's `void` is
  wrong; `CDfileName_PC` and the file-size readers use the result), or 0
  after `dbgPrintf("file name NULL !!!!!\n")` when there is no table, the
  entry is empty or reversed, or its address is 0; a `loc` at or past the
  count first prints `"Virtuos ERR: bad filename location ..."`. While
  copying, `$USER$` becomes `Sg_GetUserName2()` (skipped when that is
  null) and every set read-name substitution (`set_read_chgname`'s 16
  pairs) its replacement. A substitution overwrites the copy from the
  matched byte with the replacement and its NUL and resumes at the last
  byte matched and written, so the later pairs are tried there too. An
  empty pattern never advances: the original loops forever. Afterwards
  `dbg_` goes before `nagi0000.ebp` when `ReturnGlobalSx8_006bc960()`,
  `200thunder_` before `kami0400.ebp` with `0x01a85ba4`, and `full_`
  before `nagi0700.ebp` with `g_FullArenaLocalization`
  (`sprintf("%s%s%s", head, prefix, name)`: anything after the name is
  cut).
- `TOSeqCtrl(i)` (`0x008ae0f0`) steps scene `i`'s eight sequence slots:
  state 1 runs `TkMenuStartScene(i, handle)` and continues as 2; state 2
  becomes 3 when the `u16` at `+4` is 1, else 4 once
  `TkMenuIsTermScene(i, handle)`; state 4 runs `TkMenuStopScene` and
  frees the slot. The handle (`+0x1c`) is read again for each call. The
  slots in use then move to the front in order through a zeroed 0x100-byte
  local, which is copied back whole. `eax` is left over (a stack address
  at times); no caller reads it.
- `TOAdpButtonIconRGBA_008e99a0(cursor, c, x, y, r, g, b, a)` sets the
  packet cursor `0x018716cc` and draws button icon `c` with
  `TOMkpShapeXYC2(id, x, y, rgba, rgba)` (`rgba` from the four low bytes):
  for `@`..`O` one shape per direction bit (0x258/0x259 for bit 1, then
  0x25a/0x25b, 0x25c/0x25d, 0x25e/0x25f; the second id when set); for the
  digits `c + 0x230`, except that `1` and `2` swap to 0x262/0x261 when
  `TOGetSystemConfirmButtonAssign()` is 1 (and return at once); for `` ` ``
  0x270. It returns the packet cursor. `0x008e9970(cursor, c, x, y)`
  passes grey (0x80 each).
- `TOBtlDrawCrossParamPlate(chr, x, y)` (`0x00892cb0`), drawn like the
  status plate (`rx`/`ry` = `graphicUiRemapX2`/`Y2`, heights first):
  the frame (shape -1, 320 x 45, UVs in texels 984, 98, 1581, 148), then
  `TOBtlDrawCrossLimitGauge(chr, x + rx(10), y + ry(29), rx(300), ry(8))`,
  the HP label 0xe9 and the MP label 0xea at `(x + rx(5 or 185), y +
  ry(2))`, and both numbers with `ToMakeBtlEasyEdgeDigitRight(n, x +
  rx(160 or 305), y - ry(4), colour, 0.65, 0.0)`. Language 11 has wider
  labels (68 x 24 and 66.4 x 24 against 61.5 x 23 and 49 x 23) and takes
  3 (HP) or 6 (MP) off `x` at 53 bits before the store (it also adds 0.0
  to `y`, which changes nothing). The HP's colour comes from
  `MsCalcWeakLevel(hp, max)`: 1 and 2 give 2, 3 gives 0x53, anything else
  0; the MP's is 0. Ghidra shows two of the gauge's five arguments and
  one of the numbers' six.
- `TOSetDrawInfo(code, window, pen)` (`0x008df8f0`) applies one control
  code to a message pen (`+4` x, `+8` y, `+0x1a` icon mode, `+0x1b`
  colour nibbles); the argument byte is read again at every test:
  - 7: 0xe0..=0xef moves the pen right by `n - 0xe0`; otherwise
    `TOGetMessageWH(i16 window+4, code + 2, &w, &h, i16 window+0x1a,
    0x103)` (`w` starts at 0.0; `h` lands in the caller's argument slot),
    then `'8'` (with `window + 0x24` zero) and `'1'` run
    `TOFirstInitMessageNormal(window, [window + 0x8c] + 2, pen)`, `'2'`
    sets `x = window+0x50 - w`, and 0x40..=0xbf `x = window+0x48 + (4n -
    0x100)`.
  - 9: `'0'` zeroes `window + 0xac`, other values below 0xc0 set it to `n
    - '0'`, from 0xc0 `window + 0xb4 = n - 0xc0`.
  - 0xa: the colour nibbles, as in the cross-menu text writer.
  - 0xb: a button icon between `setButtonData(n, -1)` and
    `resetButtonData`. Icon modes 1 and 3 draw it at scale 0.73 under
    `graphicSetButtonIconScale(1.15, 1.15)` at `y + 5.5 +
    AdjustButtonIconY`. Otherwise at 0.65: language 10 at `y + 3.0 +
    adjust` (and the pen moves 8 when the language, asked again, is 11 on
    map 0xb8); language 9 with `graphicGetAbmap() == 1` at `y + 1.0`
    without adjusting; else at `y (+ 2.0 for '4'..'7') + 3.0 + adjust`
    (the pen moves 7 on map 0xb8). After a move, `'8'` on map 0xb8 takes
    1.0 back. `window + 0x9c` counts icons; with `window + 0xac` zero,
    `window + 0xa0` steps up to `window + 0xa8` (signed). The pen then
    moves by `TOGetScaleButtonIconWidth(n, scale)` + 1.5 (modes 1, 3) or
    + 1.0.
  - 0xe: `'0'..'3'` set the icon mode; mode 3 with `'A'` moves the pen
    5.0; `'@'` sets mode 3, `'A'` mode 1.
  - 8, 0xc, 0xd and other codes do nothing.
- The CD read queue's `CdQueueSectorRead_0076ce40(sector, a, b)` takes
  the first free `g_CdChannels` request and returns its id (Ghidra's
  `void` is wrong): `+0x1c` sector, `+0x28 a`, `+8 b`, `+0x30`/`+0x31`
  set, `+4`, `+0xc..+0x18`, `+0`, `+0x20`, `+0x2c` and the `u16` `+0x34`
  zeroed, `+0x24` the next `g_CdReadId` (1 after 0xffff). With all 64
  busy it calls `movie_err("q_cnt != READ_Q_CNT", libmscd.c, 0x96d)` and
  fills request 64 anyway, which starts at `g_CdReadId` (the old id is
  read before that request's `+0` is cleared). `0x0076ce20(sector)`
  passes `a = b = 0`.
- `CdReadDirect_0076d140(sector, size, buffer, mode)` clears `0x01128a87`
  (byte) and `0x01128ab0`, warns about a buffer off 64 bytes, keeps
  `sector, size, buffer, [mode]` at `0x01128acc`, runs `hdd_state_clear`
  and `FlushCache(0)`, and returns `sceCdRead(sector, size / 2048, buffer,
  mode)` (signed division); with `g_FFXHddMode` set it prints
  `"sceCdRead()"` between rules.
- `CdSetModule_0076c510(m)` selects CD module `m`: `g_CdLib_FileBase` =
  the `i16` entry `m` of `[0x02310c6c]`, `0x02310c64 = m`; a negative
  first file stores 0 and returns -1. Before `sa_file_read_init`
  (`0x01128a85` clear) it calls `movie_err("sa_file_read_init!=0",
  libmscd.c, 0x594)` first.
- `PcFileTblSet(path)` frees the old PC file-name table `0x02310c70`
  (`user_free_thunk`) and loads `0x00a42ab0(path, 0)` in its place.
- `TOBtlOpenSignWindow(id)` opens simple help window 1 on command `id`'s
  name (`MsGetComData(id as u16, &strings)`: `u16 +0` + `strings`) when
  bit 2 of the record's `+0x1c` is set (returns 1), else returns 0.
- `TOGetTextWidth_008b8040(text)` adds up the words
  `TOGetFontWidthInterMode(p, 0, &w)` writes while stepping through the
  text, as integers, although that width is an `f32` (`0x008b70c0` reads
  the same callee family's width as a float). `&w` is the function's own
  argument slot, so a call that writes nothing adds the previous word (at
  first the text pointer). `TOGetColonWidth_008dfe20(font)` measures
  `":"` the same way (as an `f32` from 0.0) and returns it through
  `ftol2_sse`.
- `Vec3At24_0061a7f0(out)` (thiscall, `ret 4`) copies the three floats
  at `+0x24` to `out` and returns `out`.

## Round 34

- `TOGetLangColonWidth_008df4f0(kind, font)` is `TOGetColonWidth_008dfe20
  (font)` for kind 0 in language 2 and kind 1 in languages 1..=5, else 0.
- `MagicFile_Init()` prints `"MagicFile: Init()\n"`, sets the magic id,
  the handle, `0x00c64ca8` and `g_BtlCommandWait` to -1 and tail-jumps to
  `MagicFile_ResetPreloaded` (its `eax`).
- The 16x12 message writers share the pen `0x025d09a0` (colour bytes
  `+0..+3`, x/y `f32` at `+4`/`+8`, `+0x1a` kind, `+0x1b` colour nibbles)
  and the glyph record `0x01841d50`, both cleared first; the integer pen
  is stored as `f32`. `TOGetGroup` gets the code byte in a register whose
  upper bytes are left over (the last argument's for the first code in
  `0x008e92a0`, 0 in `0x008ea060`, then the text pointer's).
  - `TOAddPktBaseMesFontLClutTypeRGBA(cursor, key, text, x, y, colour,
    kind, r, g, b, a)`: group 1 returns to the start x and adds 16 to y;
    group 2: `7 0xe1..=0xef` moves x, `0xa` the colour nibbles, `0xb n`
    the icon `0x008e99a0(cursor, n, ftol(x), ftol(y), r, g, b, a)` then x
    `+= f32(TOGetButtonIconWidth(n)) * [0x00c5d778]` (codes other than 7
    end with `resetButtonData`); group 3 `TOAddPktChangeMacro(key, text,
    pen)` unless `key` is -1; group 4 the record, then for kind 1 edge and
    face with `TOAddFontWidthSpace(pen, record, [0x00c5d784])`, else shadow
    and face between `graphicSetUiDrawType(1/0)` and
    `graphicFontSetUiShadowType(0/1)` with the space at 0.78.
  - `0x008ea060(cursor, text, x, y, colour, r, g, b, a)`: glyphs get
    shadow and face and move x by the record's width `+0x1c` + 1.0; `0xb n`
    draws `0x008e9970(cursor, n, ftol(x), ftol(y))` and adds 22; returns
    `cursor` for an empty text.
- `TOAdpCrossNumber_009019f0(value, x, y, colour, scale, gap)` formats
  `"%d"` into a 32-byte local and draws each glyph with
  `TOAdpCrossFontM_FFX(p, x, y, colour, 0x80, 0x80, 0x80, 0x80, scale)`
  from `x - scale * (n * (gap + 10) - gap)` (each step at 53 bits, stored
  `f32`), stepping `(gap + 10) * scale`, a double that is never stored.
- `ShapeDataModify(shape, id)` (shape record: corners `+0/+4`,
  `+0x20/+0x24`; UVs `+8/+0xc`, `+0x28/+0x2c`; colour words
  `+0x10..+0x18`, `+0x1c`, `+0x30..+0x38`, `+0x3c`) fixes shapes for the
  HD port with `graphicUiRemapX/Y` (720p): 0x3e, 0x19b, 0x19c, 0x19f and
  0x1fb get new corners (0x19b/0x19c/0x19f new UVs), 0x1f7 and 0x1fc move
  up 10 (0x1fc also x0 + 15, x1 + 35, y1 + 0.0), 0x1023 narrows by its x0
  and 25 and gets y 560..584, 0x1024/0x102d move up 20/30, take 25 off x1
  and turn alphas from 0x46 into 0x80. 0x19b/0x19c clear both alphas when
  the first colour is black, else grey both colours; with 0x1fb each
  non-zero alpha then pulses as `ftol(f32(cos(graphicGetTime() * 3)) * 10
  + 118)`.
- `MsgExpandCodes_00776940(out, text, arg, &jump, &flag)` copies a
  script message, evaluating the `0xb 0xf0..0xfa` codes (one-byte codes
  are 3 and 0x30 up; the others take two):
  `0xf0` pushes a comparison (16-byte record: `a` from getter `d1 &
  0xffff`(`d1 >> 16`) / variable `d1 >> 16` / 0 by `d0`'s low byte, `b`
  likewise from `d2` / `d3` by byte 2; operator byte 1: 1 `!=`, 2 `>=`, 3
  `<=`, 4 `>`, 5 `<`, 6 `&`, else `==`, signed) on a 64-byte condition
  stack; `0xf1` on a false top and `0xf3` on a true one skip to the
  matching `0xf3`/`0xf2` (only `0xf1` stops at `0xf3`); `0xf2` pops
  (`report_rangecheckfailure` past 64); `0xf4` runs
  `MenuTextCode_00774860(&out, &p, arg)`; with `jump`: `0xf6` sets a
  variable or calls a setter, `0xf7` stores its word +
  `*0x0112a03c` at `0x0112a084`, sets `*jump` and returns 0, `0xf8` calls
  the call table with a getter, a variable, 0 or `d2`; `0xf9` sets
  `0x0112a074`; `0xfa` sets `*flag`. At the NUL it clears `*jump` and
  returns the end past the NUL (Ghidra's `void` is wrong). Each code
  first `yiAssert`s that `out` is under 0x4000 bytes into `*0x0112a02c`.
- `TODrawMesBox_008ff370(text, x, y, extra)` draws a message box sized by
  the `i16` pair at `0x025d1644` (width) / `0x025d1642` (height): a black
  shadow (`TOMakePktBoxXYWHRGBA(x + 4, y + 4, w, h, 0, 0, 0, 0x80)`), the
  frame (`0x008fd390(x, y, w, h, [0x025d163c], [0x025d1638])`) and the
  text (`0x00901cb0(text, x + 15, y + 2, extra)`).
  `TODrawMesBoxFit_008ff3e0` measures the text first (`TOGetMessageWH(0,
  text, &w, &h, 0, 0)`) and uses `ftol(478 - w)` and `ftol(482 - w)` as
  the widths.
- `BtlDrawRatio_008955e0()` draws `0x00901490(f, x + 15, y - 20, colour)`
  while `0x0133f77c` is set (`f` the `f32` at `0x0133f794`, `x`/`y` the
  `i16` at `0x0133f780`/`0x0133f782`, `y` refreshed to
  `ftol(graphicUiRemapY2(350))` when `0x0133c93f` is set); the colour is
  `0x80008080` below 1.0 and `0x80808080` otherwise, NaN included (Ghidra's
  `1.0 <= f` has NaN the other way).
- The writers' wrappers: `0x008ed0a0(cursor, text, x, y, colour)` and
  `0x008ed160(.., r, g, b, a)` pass key -1 and kind 0 to `0x008e92a0` (grey
  for the first), `0x008ee210(cursor, key, ..)` passes its key, and
  `0x008ed060(cursor, text, x, y)` key 0 and colour 0 in grey;
  `0x008ea000` is `0x008ea060` in grey; `0x008f2ff0(cursor, value, x, y,
  colour)` writes `TOValueToMesFontL(value, text, 0)` from a 32-byte
  local like `0x008ed0a0`.

## Round 35

- `prcPacketDataClose(self)` (`pppYanagi.c`, a packet descriptor: start
  `+0`, cursor `+4`, size `+8`) writes the 16-byte end tag `0x8000`,
  `0x1000_0000`, 0, 0 at the cursor, advances it by 0x10 and asserts
  `size <= self->m_size` (signed) with `dbgPrintf`. It returns the size in
  `eax`, or `dbgPrintf`'s result when the assertion fires.
- `BtlEnd_BuildList_008bde00()` builds the battle-end list
  (`g_TkBtlEndList`, 14-byte entries from `0x01869f20`): one entry per
  character counted in `g_BtlCountsBattle`, with its row x
  (`BtlEnd_RowX(n, count)`), `6n`, the character, its damaged colour and
  whether it is in the party; `0x01869f19` is set when any counted
  character has a nonzero `g_TkBtlEndResults` word. It ends in
  `BtlEnd_CompactList` and returns its `eax`.
- `useMultipleInstances_model(model, instance)` returns 1 for a model with
  flag bit 7 set and bit 2 clear while one of eleven spells (0x133, 0x25d,
  0x6a, 0x119, 0x233, 0x264, 0x20a, 0x1bd, 0xa2, 0x24a, 0xe) runs in an
  effect pass (`g_VfxPass`); otherwise 2 on maps 0x183, 0x184 and 0x186
  when the instance's `i16` at `+0x10` is at least 0x834; else 0.
- `MacroDic_Load()` reads `"macrodic"` and turns its 16 section offsets
  into pointers (0 for offsets below 1) in `g_MacroDic`; with the language
  swap flag set it does the same for `TODVDFileLoad(4, 4)` into
  `0x01869eb4`. It returns the swap flag, or that second dictionary's last
  `get_ptr32_func` (Ghidra shows void).
- `TOBtlDrawDamageValue2()` draws the queued damage numbers (16-byte
  entries from `0x0133f110`, `i16` count at `0x0133f108`, cleared after):
  values 0..=9 as digits in the entry's colour, 0x3d/0x3c/0x3b/0x3a as
  battle icons 0..=3; in spell 0x11b kind-3 entries are skipped. Pending
  9999/99999-damage flags unlock achievements 0x12 and 0x15 first.
- `TOBtlDrawSummonPrim(chr, x, y)` and `TOBtlDrawChangePrim(chr, x, y)`
  (integer positions) draw a battle swap window: a frame (the net window
  `X2(344)` by `Y2(114)`, or shape part 0x1c0 for aeons, 0x1d4 for Yuna
  (6) before scenario flag 0x434, else `chr + 0x1b8`, `X2(70)` by
  `Y2(50)` at `(X2(20), Y2(14))`), the character's name (centred in
  `X2(344)` at scale 0.8/0.7, or centred on `X2(210)` at 0.71, both at
  `Y2(16)`), and `TOBtlDrawCrossParamPlate` at `(X2(15), Y2(61))` or
  `(X2(12), Y2(62))`; they return the plate's `eax`. Each sum is
  computed at double precision and stored as `f32`.
- `TOBtlInitStatusWin()` sets up the battle status window: three 0x90-byte
  rows at `0x0133f7d8` (cleared), the party's three `u16` characters from
  `MsGetSaveWindow(0, 0x12, &n)` at `0x0133f7ac` (0xff for none; `n` is
  trimmed past trailing 0xff entries but not used further). Each row
  holds HP at `+0xc`/`+0x10`, MP at `+0x24`/`+0x28`, the limit gauge
  scaled to the `i16` bar width at `0x025d0aa4` (`gauge * width / max`,
  signed; a zero maximum faults in the original, the port gives 0) at
  `+0x34`/`+0x38`, `TOBtlMakeStatusBuff(c, row + 0x42)` at `+0x40` and
  0x14 at `+0x3c`. It returns 0x14 when the last slot is filled, else 0xff.

## Round 36

- `updateVFXZBufClearRenderData(lo, hi, z)` turns an effect instance's
  render data into one full-screen quad at depth `z`: 4 vertices and 6
  indices in its geometry (`+0x94`), buffer from
  `graphicDynGeoMemManagerGetBufferForSegment(data, 0, 4, 6)`, indices 0,
  1, 2, 1, 3, 2, corners (-1, -1), (1, -1), (-1, 1), (1, 1), then
  `graphicVFXSetVisible(lo, hi, 1)`. Returns 1, or 0 without render data
  or buffer. Every pointer is read again before each store.
- `TOAddPktChrNameIcon_008eba80(cursor, chr, x, y, colour)` writes menu
  icon 0x13 at (`x + 3`, `y + 1`) and the character's name in grey at
  (`x + 0x16`, `y`), keeping `g_TOPktCursor` up to date after each; it
  returns the final cursor.
- `TODrawFont3d80Glyph_008eeee0(cursor, text, x, y, nibbles, c0..c3)`
  draws one character of the 0x3d80 sheet twice through
  `graphicFontDraw`: an 0x98-byte sprite record (two corners of x, y, u, v
  and four words `c0..c3`, `+0x88` 0, `+0x90` the 8-byte size
  `TOGetTextureName` writes). Character `c = *text - 0x20` (signed) sits
  in column `(c % 32) / 2` (8 wide) and row `c / 32` (10 high, from 192);
  odd characters take the second texture of a pair. The first draw
  (texture `2 * (nibbles >> 4 & 0xf) + odd`, high flag 1) covers
  (`x + 1`, `y`)..(`x + 11`, `y + 12`), the second (`2 * (nibbles & 0xf)
  + odd`) (`x`, `y - 1`)..(`x + 10`, `y + 11`); UVs divide the cell (right
  edge 3.5 in) by the image sizes, the second pair first. The original
  leaves the record's `+0x40..+0x87` and `+0x8c` uninitialised.
  It returns `cursor + 0x10`.
- `TOSetupMessageWindow_008b8de0(w)` runs the window's callback (`+0xf4`)
  and fits the window to its text (`+0x94`). Without a message (`+8` 0)
  it sets the word counts, measures the text and centres the rect (y 1 in
  message-window mode 1), returning `TOWideWindowRect_008b8d10`'s `eax`.
  With one it counts the choices (`+0x24`, `+0x28` adding the macro text
  `0x025d09bc`'s), measures the first line (flags `| 3`) and the whole
  text (`| 1`). Choice style 1 (`+0x22`) adds the macro width to a
  positive first-line width (kept as `+0x2a`) and widens the window by
  it. It then sets `+0xa8` to the word count (with the macro text's for
  style 1), `+2` to 1 when that equals the choices, and returns `+0xa8`.
  With `+0x1e` 1, `TOLayoutMessageChoices_008b9a60(w, 1)` follows and its
  `eax` is returned. The x position is stored as `f32` before `ftol`, the
  y sum is not.
- `TOBtlDrawCtbWindow(x, y)` draws the battle turn-order (CTB) window
  unless `0x0133d6e0` is 1. Its frame spans `X2(1637)`, `Y2(194)` to
  `X2(1765)`, `Y2(482)`: a net window 8 units outside it, scroll arrows
  0x1b6 (scroll `i8 0x01340446` positive) and 0x1b7 (below 10), the scroll
  bar and the window scissor. Each of the 32 CTB records at `0x01340478`
  (`+1` character, `+4` icon, `+0x10`/`+0x12` `i16` offsets, `+0x14` the
  step callback, `0x008a2b10` while sliding) that is not sliding is drawn
  by `TODrawBtlCtbParts` at `X2(15) + left + dx` and `Y2(dy / 22 * 48 +
  row * 48 / 22) + top` (row `i16 0x01340448`); the first one, at row 0,
  gets the highlight shape 0x1d8. While the command window at
  `g_BtlActionDepth` is open, it adds a packet scissor and, for each of
  the first 16 rows whose character bit is set in `0x01340470`, the
  cursor 0x1d6 at `ftol(x + left + 8)`, `y` and the target frame 0x190
  (UVs by character below 0x14 or monster). It ends with the full-screen
  scissor and returns that call's `eax`.

## Round 37

- `TOMenuDrawChrLayoutPlate_008f0a70(cursor, chr, x, y)` (caller
  `0x008c11e0`, packet cursor in, new cursor out) stores `cursor` in
  `g_TOPktCursor` and draws a character plate placed by the menu layout
  words `TOSeqInitParam` fills (`i16` `(y, x)` pairs at `0x025d1652`
  MP text, `0x025d1656` MP label, `0x025d165a` HP text, `0x025d165e` HP
  label, `0x025d1662` name; Sphere Level at `0x025d164a`/`0x025d164c`,
  AP at `0x025d164e`/`0x025d1650`; colours `0x025d166c`/`0x025d1668`).
  Positions are `x`/`y` plus the words, made `f32`; label sizes go
  through the 1280x720 remaps `graphicUiRemapX`/`Y`. A party member
  (`chr` below 8, unsigned) gets the frame (shapes 4 and 5, 142 and 40
  wide, the second at `x + 0x8e` with colour `0x025d1668` twice, read
  again after the first), the name (`ToMakeBtlEasyFont`, alpha
  `MenuPlayer_DamageClass`), the HP and MP labels with `"%5d/%5d"` and
  `"%4d/%4d"` (`TOMkpCrossEasyStrFontSClut`, colour 0x25, scale 1), the
  gauge row of sheet 0x107 (at `x + 0x14c`, then `x + 315/295/342/438`
  at double precision) and two `TOAdpCrossNumber_009019f0` numbers: the
  Sphere Levels and the AP still needed (scale 1, gap -3). Any other
  number gets the empty plate: `0x008f5e90(0xcc, 0xcd, x, y, width
  0x025d1672, colours)`, labels 0xe9 and 0xea, the bar 0xe7 across the
  screen and label 0x107. It returns `g_TOPktCursor` as the callees left
  it.
- `TOMenuDrawChrSmallPlate_008f51e0(chr, x, y)` (caller `0x008c1260`,
  `x`/`y` floats) is the top half of `TOMenuDrawChrStatusPlate_008ff490`
  without the right panel and gauge row: every part of the name row sits
  one `ry` unit lower (name 10, overdrive label 8, HP 9, Sphere Level
  label 35, MP 37), and language 11's gap of 11 goes inside the numbers'
  remap (`rx(gap + 604)`) instead of after it. `chr` below 8 is unsigned
  here: party members get the clear sheet and water or the dark sheet,
  then the name row; every other number the dark sheet and the two empty
  labels. It returns the last call's `eax`.
- `TOMakePktSeqPrimLayout_008fe0b0(prim)` draws a menu-sequence
  primitive of type 6 (caller `0x00a7bbc0`). The primitive holds four
  colour bytes at `+4..+7` (and the words `+4`, `+8` as corner colours),
  the position `i16 +0xc`/`+0xe`, the scale in percent `i16
  +0x10`/`+0x12` (100 = none) and the shape `u16 +0x14`. Shapes 0x19b,
  0x19c, 0x19f, 0x1a3 and 0x1fb draw nothing (`eax` the shape). 0x1a1 is
  the gauge quad `TOMkpShapeXYWHUVC2(-1, x0 - X2(w / 2), Y2(535 - h /
  2), X2(w), Y2(h), 1759, 1, 1835, 77, +4, +8)` with `w`, `h` = 120 x
  the scale (the scale stored as `f32` first, the products kept at
  double precision) and `x0` 160 at `x` 0x74, else 388; it draws nothing
  below `y` 0x14a (`eax` 0x14a). 0xf0 is shape 200 at `(x - X2(10), y -
  Y2(185))`, `X2(89)` x `Y2(186)`. Other shapes below 600 draw window
  layout `shape` (`TOCopyWindowLayout_008fa7d0(shape)` below 200,
  `(shape - 200)` below 400, sheet 0x3e00; `_008fa450(shape - 400)` on
  sheet 0x3f00 below 600; `TOGetImageWH` gives the sheet size). Each
  12-byte part (`u`, `v`, `i8 dx`, `i8 dy`, `u16 w`, `h`, two flip
  words) becomes a 0x98-byte UI record: corners `ftol(dx / 100 * scale)`
  and `ftol((w + dx) / 100 * scale)` plus `x` (likewise for y), swapped
  when the flip word is 1; UVs `u / width`, `(u + w - 1) / width` (the
  sum stored as `f32` first), `v` likewise; the colour bytes at both
  corners; `+0x80`, `+0x88`, `+0x90`, `+0x94` zeroed once. 0x19e and
  0x1fc pass the record through `ShapeDataModify(record, 0x102d /
  0x1fc)` first; then `graphicDrawUIElement(record, texture, 1, 0, 0)`.
  0x19d draws only the first part, as the cursor `0x008f59c0(ftol(x *
  1920 / 512 - 21), ftol(y * 1080 / 416 - 18), 0x14c, 0x20, 0x112, 0x3d,
  a, a)` and shape 400 beside it (`a` the colour byte `+7`, colour `a <<
  24 | 0x808080`), unless the corner's `y` is past 300 (`eax` is then the
  x87 status word of that comparison, 0x3000 plus sticky flags). 0x1f7
  draws the first part as shape 400 left of and above its corner.
  `eax`: the layout pointer after the parts, the colour byte `+7` for an
  empty layout. From 600 up the original reads a stale stack word as the
  layout; the port draws nothing.
- `fix_mappic(map)` gives the picture the PC save list shows for a save
  made on `map` (`m%d`, or `m%d_1` for 1000 up; the caller turns 0 into
  999 when `isNeedShowJapanLogo`). Maps outside 1..=0x15c give 0; 26 maps
  (2, 0x2e, 0x32, ... 0x152) show the previous map's picture; 3 gives 1,
  7 0x120, 0x13 0x11; 0x59, 0x147..=0x14a, 0x14e and 0x14f give 0;
  0xaf..=0xb1 0xad; 0xe8/0xe9 0xe6; 0x100 0xfe. On 0xd8, 0x1f, 0x101,
  0x11c, 0x81/0x15c, 0xf4/0xf5 and 0xf6/0xf7 it compares the player's
  position (`g_SaveData_PlayerPos`, `0x00ccb9a8`, which `SaveData_Update`
  copies each frame) with two fixed points: the squared distances (x87
  double sums stored as `f32`) pick `map + 1000` when the player is
  nearer the second picture's point, else `map` (ties and NaNs too).
  Any other map is its own picture.
