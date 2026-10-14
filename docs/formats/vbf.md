# VBF archives

`data/FFX_Data.vbf`, `data/FFX2_Data.vbf` and `data/metamenu.vbf` hold all
game data. FFX.exe mounts `data\FFX_Data.vbf` at startup
(`Game_InitFileSystem`, `0x006795b0`) and shows a `MessageBoxW` if that fails.

Reader: [`crates/ffx-vbf`](../../crates/ffx-vbf). Every function named below
is in [`symbols/ffx.sym`](../../symbols/ffx.sym).

## Layout

All integers are little-endian.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Magic `0x4b595253` (`"SRYK"`). Anything else prints `BigFile head wrong!` |
| `0x04` | `u32` | Header length `H`. File data starts at `H` |
| `0x08` | `u32` | Entry count `N` |
| `0x0c` | `u32` | Zero in every shipped archive; not read |
| `0x10` | `[u8;16] × N` | MD5 of each entry's path |
| `0x10 + 16N` | `Record × N` | One 32-byte record per digest, same order |
| next | `u32` | `L`: 4 + the length of the path table |
| next | `L − 4` bytes | NUL-terminated paths |
| next | `u16 × B` | Stored size of every block (`B` = total block count). Ends exactly at `H` |
| `H` | | File data |
| end − 16 | `[u8;16]` | MD5 of the first `H` bytes |

Record (32 bytes):

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | Index of the file's first block in the block-size table |
| `0x04` | `u32` | Unknown, not read by `Vbf_Open`. Equals the first-block index in `metamenu.vbf`; in `FFX_Data.vbf` it is `0` for the 7 empty files and `3675126` for all others |
| `0x08` | `u64` | Uncompressed size |
| `0x10` | `u64` | Absolute file offset of the first block. Junk for empty files: the 7 in `FFX_Data.vbf` (names like `temp`, `placeholder`, `*.tmp`, `*.$$$`) have the entry count in the high dword |
| `0x18` | `u64` | Offset of the path in the path table |

Non-empty entries are stored in data order (ascending offset), not sorted
by digest.
The game builds its own lookup map from them (`Vbf_InsertEntry`).

## Blocks

Each file is cut into 64 KiB (`0x10000`) blocks. A file of `size` bytes has
`ceil(size / 0x10000)` blocks, stored back to back from the record's offset.
The block-size table gives each block's stored size:

- `0`: a raw 64 KiB block.
- equal to the block's uncompressed size (only possible for the last block):
  raw.
- anything else: zlib stream (`78 da` header) that inflates to the block's
  uncompressed size.

`Vbf_Open` precomputes this as a one-byte-per-block "compressed" flag table.
`ReadRequest_Inflate` (`0x0061ba80`) `memcpy`s raw blocks and passes the rest
to `zlib_uncompress` (`0x00405e20`, zlib's `uncompress(dst, &dst_len, src,
src_len)`). A size mismatch or zlib error shows message text `0x7836` in a
`MessageBoxW`.

## Lookup

`File_Probe` (`0x00607e00`) shows how the game turns a path into a key: it
converts `\` to `/` and lowercases ASCII `A`–`Z` (`Path_ToLowerAscii`,
`0x00607d50`). `FileSys_Exists` / `FileSys_FindEntry` (`0x0061be40` /
`0x0061bf10`) then skip the root prefix set by `FileSys_SetRootPath`
(`"../../../"` at startup), MD5 the rest and ask each of the up to 5 mounted
archives. Stored paths look like `ffx_data/gamedata/ps3data/...`.

`Archive::find` applies the same separator and case rules. Stripping the root
prefix is up to the caller.

## Integrity

`Vbf_Open` reads the last 16 bytes of the file, MD5s the header with
CryptoAPI (`CALG_MD5`) and refuses the archive unless the two match.

## Writing

`ffx_vbf::pack` (`ffx vbf pack <base.vbf> --mod <dir> … -o <out.vbf>`)
rebuilds an archive from a base plus changed and added files. Each `--mod`
folder mirrors archive paths; later folders win.

- Unchanged files keep their stored blocks, copied verbatim. File data in
  the shipped archives is contiguous and in block order, so packing without
  changes reproduces the base exactly.
- Changed and added files are cut into 64 KiB blocks and zlib-compressed
  where that is smaller; otherwise stored raw.
- New entries go after the existing ones, their paths at the end of the
  path table. Record field `0x04`, which the game does not read, follows the
  base's convention (the first-block index in `metamenu.vbf`, the shared
  constant in the big archives).
- The header's MD5 is recomputed for the trailer, which is all `Vbf_Open`
  checks.

For development, the loose-file mod folder (`docs/hooking.md`) is quicker:
no 20 GB rewrite.

## Verification

- `cargo test -p ffx-vbf` builds archives in memory and round-trips them.
- With `FFX_GAME_DIR` set, it also reads all 150 files in `metamenu.vbf`.
- `cargo test --release -p ffx-vbf --test game_data -- --ignored` decompresses
  all 71,979 files in `FFX_Data.vbf` and checks every size, and repacks it
  without changes: the result has the original's MD5 (about 70 s).
- Without `--ignored`, `metamenu.vbf` is repacked and compared.

## Open questions

- Record field `0x04`.
- Header field `0x0c`.
