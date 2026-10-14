# PhyreEngine clusters (`*.phyre`)

All assets the HD Remaster added or remade (textures, models, shaders,
effects) are PhyreEngine "clusters". PhyreEngine is a Sony SDK without
public documentation. Everything here was recovered from the files and
from the engine code linked into `FFX.exe`. That code includes a cluster
analyzer (`PCluster_Analyze`, `0x579310`), which prints every table's size.

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
| 15 | Header class count |
| 16 | Header class fixup count |
| 18 | Models: index buffer size |
| 19 | Models: vertex buffer size |
| 20 | 2D textures: size of the top mip level |

Word names follow the strings the namespace itself contains
(`m_packedNamespaceSize`, `m_instanceListCount`, …).

## Layout

```text
0x00          header
0x54          packed namespace: the classes and members used, with names
...           0x24 bytes per instance list
...           instance data (word 14 bytes): each list's objects, then its arrays
...           user fixup data: NUL-separated strings
...           12 bytes per user fixup: type, length, offset into the data
...           4 bytes per header class count, 16 per header class fixup
...           pointer-array fixups, pointer fixups, array fixups (packed)
...           buffers: texture pixels, or index then vertex buffer
```

The buffer offset is the sum of those sizes. For textures it is exact in all
44,364 files.

## Namespace

```text
+0x00  u32  0x01020304
+0x04  u32  size
+0x08  u32  type count T      (plain types: PUInt32, float, enums…)
+0x0c  u32  class count C
+0x10  u32  member count M
+0x14  u32  string table size (the table ends the namespace)
+0x20  u32 × T          type name offsets
…      36 bytes × C     base class (1-based, 0 = none), 0x20000000 | size,
                        name, member count, …
…      24 bytes × M     name, type (types, then 1-based classes), offset,
                        size, flags, …
```

An instance list header holds:

- the class (1-based);
- the object count, total size, objects size and arrays size;
- the counts of array fixups (`+0x18`), pointer fixups (`+0x1c`) and
  pointer-array fixups (`+0x20`).

Member offsets are found by walking up the base classes. For example,
`PDataBlock.m_stride` is declared on `PDataBlockBase`.

## Pointer fixups

Pointers in the instance data are stored as zero and described by the
pointer fixups. The unpacker is `Cluster_UnpackPointerFixups` (`0x60db20`),
which calls `Fixup_UnpackList` (`0x60a750`) once per instance list. Each
record unpacks to seven `u32`s:

| Word | Meaning |
|---|---|
| 0 | `0x80000000` \| byte offset of the pointer in the object (else a non-pointer field) |
| 1 | Object index in its list |
| 2 | First target object |
| 3 | Target instance list |
| 4 | Unknown (always 0 so far) |
| 5 | Number of targets (array pointers) |
| 6 | User fixup index, or `0xffffffff` |

Records come in groups. A group starts with a byte: the mode is in bits
0–2 and the flags in bits 3–7. Then comes a LEB128 word 0, rotated right
by one bit. With flag `0x20`, a LEB128 word 3 follows; it is shared by the
whole group.

The mode chooses the objects:

- 0: every object;
- 1: subgroups, each with a kind byte (2, 3, 4 or 6) and its own words;
- 2: a listed set;
- 3: all but a listed set;
- 4: a bitmask;
- 5: explicit records;
- 6: an arithmetic run (start, step, count).

Object indices are a byte for lists under 256 objects, LEB128 otherwise.
Each record's remaining words come from `Fixup_ReadRecord` (`0x60e730`).
It reads word 6 + 1 unless flag `0x10` is set; zero means a pointer.
For a pointer it then reads words 2, 3 (skipped by `0x20`) and 4 (skipped
by `0x40`). Last it reads word 5, unless flag `0x08` is set.

## Models (`*.dae.phyre`)

| Class | Fields used |
|---|---|
| `PMeshSegment` | `m_materialIndex` +0, `m_primitiveType` +0x10 (always 2, triangle list), `m_vertexData` +0x28 (array of `PDataBlock`), `m_indexData` +0x30 (embedded `PIndexDataBlock`) |
| `PIndexDataBlock` | `m_elementCount` +8, `m_type` +0xc (12 = `u16`), `m_offsetInIndexBuffer` +0x2c |
| `PDataBlock` | `m_stride` +0, `m_elementCount` +4, `m_streams` +8 (array of `PVertexStream`), `m_offsetInVertexBuffer` +0x30 |
| `PVertexStream` | `m_offset` +0 (within the stride), `m_renderDataType` +4 (user fixup), `m_type` +8 |

A segment's vertex streams are found by following the pointer fixups:

1. `m_vertexData` gives the segment's data blocks.
2. Each block's `m_streams` gives its streams.
3. Each stream's `m_renderDataType` gives a user fixup string such as
   `Vertex`, `SkinnableNormal`, `ST`, `Color`, `SkinWeights` or
   `SkinIndices`.

Several segments can share the same user fixup strings, so the order of
those strings says nothing about the order of the streams. Assigning
streams by order is wrong for about a fifth of the models.

`m_type` values seen: 1 float2, 2 float3, 3 float4, `0x13` four bytes.

| Semantic | Types |
|---|---|
| `Vertex` | float3, float4 |
| `Normal` | float3 |
| `ST` | float2 |
| `Color` | four bytes, float4 |
| `SkinWeights` | float4 |
| `SkinIndices` | four bytes |

Coordinates are Y-down, so the glTF export turns the root node half a
turn about X. Map models are already in world space, sky shells included.

```bash
ffx model export ffx_data/gamedata/ps3data/chr/mon/m009/ -o models/
```

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

The same test exports all 1,266 models that have meshes, about 9.5 million
triangles. The other 48 `.dae.phyre` files have no `PMeshSegment`. A
monster, a playable character and a battle map were rendered and checked
by eye.

`cargo test -p ffx-oracle` unpacks the pointer fixups of 2,233 clusters
with both the Rust port and the original `0x60db20`, run under emulation.
The records must be identical (all models, plus every 50th other cluster).

## Open questions

- Array and pointer-array fixups (decoded by the same family of functions,
  not needed yet).
- Skeletons (`PMesh.m_skeletonMatrices`, `m_matrixParents`), materials
  and their textures, animations.
- Writing models back.
- Shaders (`*.fx.phyre`) and the other cluster kinds.
- Changing a texture's size or format.
