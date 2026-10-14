# PhyreEngine clusters (`*.phyre`)

All assets the HD Remaster added or remade (textures, models, shaders,
effects) are PhyreEngine "clusters". PhyreEngine is a Sony SDK without
public documentation; everything here was recovered from the files.

Reader/writer: [`crates/ffx-phyre`](../../crates/ffx-phyre).

## Header

21 little-endian `u32`s (`0x54` bytes); PC files start with `"RYHP"`
(`0x50485952`) and platform `"11XD"` (D3D11). 30 textures still carry a
big-endian PS3 header (`"PHYR"`) and are not read.

| Word | Meaning |
|---|---|
| 0 | Magic |
| 1 | Header size, `0x54` |
| 2 | Packed namespace size |
| 3 | Platform |
| 4 | Instance list count |
| 5, 6 | Array fixup size, count |
| 7, 8 | Pointer fixup size, count |
| 9, 10 | Pointer-array fixup size, count |
| 11 | Pointers in arrays |
| 12, 13 | User fixup count, data size |
| 14 | Total instance data size |
| 20 | 2D textures: size of the top mip level |

Word names follow the strings the namespace itself contains
(`m_packedNamespaceSize`, `m_instanceListCount`, …).

## Layout

```text
0x00          header
0x54          packed namespace: the classes and members used, with names
...           instance data (word 14 bytes)
...           0x24 bytes per instance list
...           user fixup data: NUL-separated strings
...           12 bytes per user fixup
...           array, pointer and pointer-array fixups (packed)
...           buffers (texture pixels)
```

The buffer offset is the sum of those sizes; for textures it is exact in all
44,364 files.

## Textures (`*.dds.phyre`)

The user fixup data starts with the class and format names
(`"PTexture2D\0DXT5\0"`). The texture's fields sit at fixed distances from
the end of the instance data:

| Class | Max mip level | Width | Height |
|---|---|---|---|
| `PTexture2D` | end − `0x20` | end − `0x10` | end − `0x0c` |
| `PTextureCubeMap` | end − `0x14` | end − `0x04` (square) | |

The buffer is the full mip chain, largest first, per face for cube maps.
**Rows are stored bottom first** (the PS3 version's convention): exported
as is, every texture with text is upside down. Export flips each mip level
upright; import flips it back (for DXT formats that means reversing block
rows and the pixel rows inside each block).

| Format | Count | DDS |
|---|---|---|
| DXT5 | 29,452 + 3 cube | `DXT5` |
| DXT1 | 13,404 + 107 cube | `DXT1` |
| ARGB8 | 1,062 | 32-bit, bytes B G R A |
| DXT3 | 335 | `DXT3` |
| L8 | 1 | luminance |

Widths and heights are the HD sizes; file names such as
`13184_19_0_0_128_64.dds.phyre` keep the PS2 ones.

```bash
ffx texture export ffx_data/gamedata/ps3data/menu/ -o textures/
ffx texture import ffx_data/gamedata/ps3data/menu/d3d11/skip_us.dds.phyre skip_us.dds --mod-dir mods/my-mod
```

An imported DDS must keep the original's format, size and mip count; it
replaces only the pixel data.

## Verification

`cargo test -p ffx-phyre` (with `FFX_GAME_DIR`) parses all 44,364 PC
textures, exports each to DDS and imports it back, and checks the result is
the original file byte for byte. Samples of each format were rendered to
PNG and checked by eye (menu art, the keyboard icon sheet, the logo).

## Open questions

- The packed namespace's encoding (classes, members, types).
- Models (`*.dae.phyre`), shaders (`*.fx.phyre`) and the other cluster
  kinds.
- Changing a texture's size or format.
