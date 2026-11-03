# The PS2 disc and its file tables

The PC build still reads most of the PS2 game's files by number, through
the PS2 CD library's tables (`docs/formats/system.md`, "DVD layout
constants"; `docs/hooking.md` for the size wrappers). This page puts the
tables together, from FFX.exe and from a PS2 disc image: the German PAL
disc (`SCES_504.92`), read by ffx-demaster's extractor (source: ffx-demaster
`docs/disc.md`).

## The disc

The ISO 9660 file system holds only the boot files: the ELF
(`SCES_504.92`), `SYSTEM.CNF`, the IOP modules (`*.IRX`) and
`IOPRP234.IMG`. Everything else is in raw sectors that the ELF finds
through its own tables, at the sectors the `dvd_cd_*` constants give
(the German disc uses the same values as FFX.exe):

| Sector | Size | What |
|---|---|---|
| `0x117` | `0x30` | ID block: the table sectors and sizes again, `FINAL_FANTASY_X` at `+0x40` |
| `0x118` | `0x20000` | `cdrom.mdg`: one `u32` per file number |
| `0x158` | `0x82` | `cdrom.fid`: 65 `i16`, the first file number of each module (-1: none) |

## `cdrom.mdg` (`g_CdLib_SectorTable`)

One `u32` per file number (`CDfileSize_CD` `0x0076c320`, `cd_file_sector`
`0x0076d510`):

| Bits | Meaning |
|---|---|
| 0-21 | start sector (plus `g_Cd_SectorBase`) |
| 22 | stored compressed (`sa_decode`, `docs/formats/main.md`, "Round 15"); the unpacked size is in `sizetbl.bin` |
| 23 | the file is not on this disc |
| 24-31 | uncompressed files: unused bytes at the end of the last sector, / 8 |

`CDfileSize_CD` returns, for a file with bits 22 and 23 clear,
`((next start - start) * 0x100 - top byte) * 8`; with bit 22 set, the
3-byte entry of `sizetbl.bin` (`g_Cd_SizeTable`, `0x01128ae0`) shifted left
by 3. File numbers are module-relative: the current module's first file
(`g_CdLib_FileBase`, set by `CdSetModule_0076c510` from `cdrom.fid`) is
added first.

## The PC's copies

`pc_fat_index_filename_set` (`0x00a42fc0`) loads the tables from the PC
archive. In the normal (non-HDD) mode it reads
`ffx_ps2/ffx/proj/battle/jp/cddata/cdrom.{mdg,fid,fnd}`; with the byte at
`0x0133c913` set, `ffx_ps2/ffx/proj/prog/cdidx/<country>/cdrom.{mdg,fid}`
with the two country letters from `MsSetCountryName` patched into the
path, and no name table. `pc_sizetbl_set` (`0x00a430c0`) reads
`cdidx/<country>/sizetbl.bin`, 3 bytes per file number.

The `cdidx` folders (`jp`, `us`, `uk`, `de`, `fr`, `it`, `es`) are each a
PS2 disc's tables. The `de` set equals the German disc's sectors
`0x117..` byte for byte. Each disc numbers its files differently, so a
reader of a disc image picks the folder whose `cdrom.mdg` equals the
disc's.

## Names: `cdrom.fnd` (`0x02310c70`)

`PcFileName_00a42c00(n)` looks entry `n` up in the table `GetCD() + 0x30`:
a `u32` count, `count + 1` `u32` offsets from the table's start, then
NUL-terminated `host0:/ffx/master/...` paths. Entry `n` runs from offset
`n` to offset `n + 1`; an empty or negative range has no name.
`CDfileName_PC` (`0x00a42830`) and `CDfileSize_PC` (`0x00a428a0`) pass
`g_CdLib_FileBase + number`.

`cdrom.fnd` is numbered after the Japanese International disc (the
`battle/jp/cddata/cdrom.fid` next to it). Another disc's file `i` in module
`m` (the module whose start in that disc's `cdrom.fid` is the largest at or
below `i`) has the name `jp_fid[m] + (i - disc_fid[m])`. On the German disc
every module starts 49 files earlier than on the Japanese one, except
modules 0, 1 and 3. Checked by content: with this mapping 2,552 `chr` files
and 250 event motion groups of the German disc are byte-identical to the
PC's files of the same name.

## What the German disc holds

7,363 file numbers, 6,774 of them compressed (all decode with methods 1 and
2), 5.2 GB unpacked. 509 have no name in `cdrom.fnd` (mostly the gaps
between modules); 584 are `host0:magic/mag_NNNN.bin` (effects) and 121
`host0:/ffx/proj/sound/...`.

Against the PC's `ffx_ps2/ffx/master/` copies:

| PS2 files | On PC |
|---|---|
| 857 `chr/**/*.chr` | 805 identical but for the texture pixels, which are zeroed; 52 identical |
| 311 field and 56 battle `mapout.txa` (map textures) | missing |
| 311 `mapout.vpa` (map geometry) | 277 the same size with regions zeroed (most PS2 geometry packets and the 2D background image) |
| 584 `mag_NNNN.bin` (effects) | missing; the PC has `magicFiles/FFX/magicNNNN.dll` and Phyre textures |
| `movie/**` | missing; the PC has WebM videos (`docs/formats/ps2-movie.md`) |
| event scripts, kernel, battle data | mostly identical, or only the same size (PAL text) |

The PS2 formats of the missing parts: `docs/formats/ps2-movie.md` (movies),
`docs/formats/ps2-music.md` (music sequences and instrument banks).
