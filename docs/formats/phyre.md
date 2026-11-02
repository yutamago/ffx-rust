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
| 0 | The field: `0x80000000` \| byte offset (used for fields inside embedded members), else the member's index in the namespace member table. The loader resolves both forms through `0x57a280`. |
| 1 | Object index in its list |
| 2 | First target object |
| 3 | Target instance list |
| 4 | Byte offset added to the target (e.g. `+0x30`, a `PMesh`'s embedded `PMaterialSet`) |
| 5 | Number of targets for array pointers; element index for pointer arrays |
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

## Array fixups

Arrays (strings, matrix arrays, …) live in each list's array data. The
array fixups say where. `Cluster_UnpackArrayFixups` (`0x60dac0`) calls
`Fixup_UnpackArrayList` (`0x60a340`) per list, with the list header's
`+0x18` count. Pointer-array fixups use the same function with the `+0x20`
count.

The group encoding is the same as for pointers, with 16-byte records:
word 0 (the field), word 1 (the object), word 2 (read unless flag `0x08`)
and word 3. Word 3 is the byte offset of the array in its list's array
data. The run helpers are copies of the pointer ones for 16-byte records
(`0x60ca10`, `0x60ce70`, `0x60cc20`, `0x60cac0`, `0x60d000`).

For example, a `PString`'s characters are at array data + word 3, NUL
terminated.

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

### Scene

- **Nodes.** `PNode` objects (`m_parent`, `m_name`, and `m_localMatrix`, a
  column-major 4×4) form the hierarchy.
- **Instances.** A `PMeshInstance` places a `PMesh` (`m_mesh`) with a
  material set (`m_materialSet`, a pointer to an embedded `PMaterialSet`,
  normally the mesh's own `m_defaultMaterials`). The instance is attached
  to the node that shares its `PWorldMatrix`: `m_localToWorldMatrix` and
  `PNode.m_worldMatrix` point to the same object. The world matrices
  themselves are zero in the file and computed at load time.
- **Bounds.** A `PMeshInstanceBounds` (`m_min` +0, `m_worldMatrix` +0xc,
  `m_size` +0x10, `m_meshInstance` +0x1c, `m_render_order` +0x20) points
  at each instance: the box the renderer culls with and the sort class
  (`Instance::bounds`; `docs/formats/scene.md`).
- **Uninstanced meshes.** A map can contain meshes that no instance uses
  (copies for dynamic modifiers). Exporting every segment would draw
  those twice, so the export follows the instances.

### Materials

1. `PMesh.m_defaultMaterials` is a pointer array of `PMaterial`. Fixup
   word 5 is the element, and `m_materialIndex` selects it.
2. `PMaterial.m_parameterBuffer` points to a `PParameterBuffer`, each in
   its own instance list.
3. `m_tweakableShaderParameterDefinitions` points to its
   `PShaderParameterDefinition`s. A definition has a name, a
   `m_parameterType` (`0x42` sampler, `0x47` sampler state, `0x40`
   constant) and a `m_bufferLoc` (size << 16 | offset).
4. A sampler's texture is a user fixup at buffer object + 8 + offset + 4.
   Its data is two bytes: a big-endian index into the
   `PAssetReferenceImport` list. That import's `m_id` is the path, for
   example `PS3Data/chr/mon/m009/tex/m009.dds`, stored in the archive as
   `ffx_data/gamedata/ps3data/chr/mon/m009/tex/d3d11/m009.dds.phyre`.

Base color is `TextureSampler`. The other samplers seen are `AnimSampler`,
`EnvironmentMapSampler`, `NormalMapSampler`, `NormalMap2Sampler`,
`EmissionMapSampler`, `SpecularMapSampler` and `TextureSampler1`.

5. A constant's values (`f32`s, `size` bytes) are at buffer object +
   offset, without the samplers' 8. With that base, `MaterialColour` is
   (1, 1, 1, 1) and `AlphaThreshold` 0.5 on Tidus; 8 bytes on, every
   float4 straddles two parameters.

`m_dataType` is 0 float, 1 float2, 2 float3, 3 float4, 0x31 float4x4.
Every material has `MaterialColour`, `MaterialDiffuse`,
`MaterialEmissiveness`, `AlphaThreshold`, `clrScaleFactor`,
`globalColorMask` and `fgEff`. Characters add lighting terms
(`CharacterAmbientLight`, `CharacterShadeColor`, `ffxshade`, …); maps add
`hsv_scale`, `YIQshift`, `YIQscale` and light matrices.

The glTF export maps `MaterialColour` to the base color factor,
`AlphaThreshold` (when 0–1) to an alpha mask cutoff, `NormalMapSampler` to
the normal texture (with `TANGENT` from the stored tangents and
binormals), and `EmissionMapSampler` and `MaterialEmissiveness` to
emission. It keeps every parameter and sampler in the material's
`extras`.

### Skins

| Class | Fields used |
|---|---|
| `PMesh` | `m_matrixNames` (array of `PString`, the hierarchy matrices: they name `PNode`s), `m_skeletonMatrices` (array of `PMatrix4`), `m_matrixParents`, `m_defaultPose` |
| `PMeshSegment` | `m_skinBones` +8 (array of `PSkinBoneRemap`, the segment's bone palette) |
| `PSkinBoneRemap` | `m_hierarchyMatrixIndex` +0, `m_skeletonMatrixIndex` +2 (`u16` each) |

A vertex's `SkinIndices` select entries of its segment's palette. Each
entry names a hierarchy matrix (the joint node) and a skeleton matrix.
The skeleton matrix is the joint's inverse bind matrix: multiplied by the
node's world matrix (the product of `m_localMatrix` up the parents) it
gives the identity. That holds for 48,831 of the 49,543 bones of the
`chr/` models. The others belong to a few models whose stored node pose
is not the bind pose (e.g. the summon `s016`).

The glTF export:

- makes one skin per skinned instance, with the hierarchy matrices as
  joints;
- maps `JOINTS_0` from palette entries to hierarchy indices;
- normalizes the weights;
- writes the skeleton matrices as inverse bind matrices.

### Orientation

Character models (`chr/`: party, monsters, NPCs, weapons, summons) are
stored Y down: they came from the PS2 data. Their source scenes carry
Maya cameras (`persp`, `top`, `front`), with `top` at +Y, so the authoring
scene was Y up and the characters sit in it upside down. The remastered
maps (`map/`, `btlmap/`) are Y up. Nothing in a file says which kind it
is. `ffx model export` decides by path and turns `chr/` models upright
with a half turn about X; a rotation keeps the triangle winding.

```bash
ffx model export ffx_data/gamedata/ps3data/chr/mon/m009/ -o models/
ffx phyre dump ffx_data/gamedata/ps3data/chr/mon/m009/mdl/d3d11/m009.dae.phyre --class PMaterial
```

The export embeds each base color texture as PNG, decoding only the top
mip level and turning it upright. Character models also get their PS2
motions as animations (`docs/formats/motion.md`).

### Writing models back

The index and vertex buffers end the file, so a segment may change its
vertex and triangle counts: `Model::write` rebuilds both buffers and
updates the fields that describe them:

| Object | Fields |
|---|---|
| `PDataBlock` | `m_elementCount`, `m_offsetInVertexBuffer`, `m_dataSize` (stride × count) |
| `PIndexDataBlock` (in `PMeshSegment`) | `m_minimumIndex`, `m_maximumIndex`, `m_elementCount`, `m_offsetInIndexBuffer` (4-byte aligned per segment), `m_dataSize` |
| Header | words 18 and 19 (index and vertex buffer sizes) |

Blocks keep their order in the vertex buffer, back to back; segments keep
theirs in the index buffer. Tangents and binormals are recomputed from the
UVs when a segment's vertex count changes. Materials and bounds are kept as they are;
new segments, palettes and skeleton matrices are below.

`ffx model import` reads the geometry from an exported `.glb`. Every
primitive carries `extras.ffx_segment` and every mesh
`extras.ffx_segments`, so edits made in another tool find their segment
(the joints by name, through the skin). Values the edit did not change
keep their stored bytes.

```bash
ffx model export ffx_data/gamedata/ps3data/chr/mon/m002/mdl/d3d11/m002.dae.phyre -o models/
# … edit models/…/m002.glb …
ffx model import ffx_data/gamedata/ps3data/chr/mon/m002/mdl/d3d11/m002.dae.phyre \
    models/ffx_data/gamedata/ps3data/chr/mon/m002/mdl/d3d11/m002.glb --mod-dir mods/my-mod
```

### New segments and joints

`ffx model import` also takes what an editor adds. It goes through
`ClusterEdit` (`crates/ffx-phyre/src/edit.rs`), which takes the cluster
apart into objects per list, array data, decoded fixups and buffers, and
writes it back with every count and size recomputed. The fixups are packed
as explicit records (group mode 5): about 0.6 % larger than the shipped
packing, but the game reads them the same. All 1,266 models and every 20th
other cluster rewrite and read back with identical objects, fixups and
buffers. Tidus rewritten this way plays the opening as usual.

- **A primitive without `ffx_segment`** becomes a new segment. It is a copy
  of a segment of the same instance with the same glTF material (else its
  first): the `PMeshSegment`, its `PSkinBoneRemap` palette, its eight
  `PDataBlock`s and their `PVertexStream`s (with the streams' user
  fixups). It is inserted after the mesh's last segment, so the mesh's
  `m_meshSegments` range and count grow by one, and every
  `PMeshInstance` of the mesh gets a `PMeshInstanceSegmentContext` in its
  `m_segmentContext` range. Insertions shift the object indices of every
  fixup that points past them.
- **Vertices weighted to joints outside a segment's palette** get them
  appended to a new palette (new `PSkinBoneRemap`s; existing entries keep
  their numbers).
- **Joints the mesh has no skeleton matrix for** get one: the inverse of
  the joint node's world matrix (the product of the `m_localMatrix`es up
  the parents), appended to `m_skeletonMatrices` with a
  `PSkeletonJointBounds` entry naming the joint (`m_hierarchyMatrixIndex`).
  The root joint (`SkeletonNode_0`) is posed differently in game than
  stored, so geometry bound to it alone lands elsewhere; use the joints
  that skinned segments use.

- **A new glTF material** (an index past the model's) on a new primitive
  becomes a new material: a copy of the template segment's `PMaterial`
  with its own copy of the `PParameterBuffer`. Each buffer is a list of
  its own; the copy goes after the last one, the lists after it move up
  one, and its header class entry (the buffers' entries come last in the
  header class tables, one count per buffer: its tweakable parameter
  definitions) is copied after the last. The material joins the mesh's
  material set (`m_defaultMaterials`, a pointer array), and the segment's
  `m_materialIndex` selects it. Textures come from the glTF material's
  `extras.ffx_textures` (sampler → import id): a texture the cluster does
  not import yet gets a `PAssetReferenceImport` (a copy of the one it
  replaces, with the new `m_id`) and a new two-byte user fixup.
- **New textures must be in the character's asset list**,
  `chr/…/<name>/<name>.ahwin32` (see below); `ffx model import --mod-dir`
  writes the updated list next to the model.

Every list's data (objects and arrays) is a multiple of 4 bytes; the
writer pads array data to keep it so. A new import's id of odd length,
unpadded, crashed the game: the loader lays the lists out back to back.

Checked in game: a copy of Tidus's body as a 14th segment stands beside
him and moves with his animation; bound rigidly to his head through a new
skeleton matrix it follows his head; with a new material on Yuna's texture
it wears her colours. The first-scene game test loads that model (and the
asset list naming the texture) every run.

New joints in the hierarchy are not written.

### In the running game (D3D11)

A loaded model keeps the file's layout: each object sits in its instance
list at the size its class gives (`PMeshSegment` 0x6c, `PMeshInstance`
0x84, `PNode` 0x54), with pointers fixed up. The class descriptor
constructors declare those sizes (`0x00486cc0`, `0x004f2200`,
`0x00504ae0`). The table gives the
offsets a reader needs to walk a loaded model. The first two rows are what
the loader adds: the Direct3D buffers. The others are the classes' own
field offsets.

| Object | In memory |
|---|---|
| `PMeshSegment` | `m_indexData` (+0x30) is a `PIndexDataBlockD3D11`: its (`ID3D11Buffer`, SRV) pairs are a PSharray at block +0x14 (count, bit 31 a flag; one pair inline at +0x18, more through the pointer at +0x18), 0x14 bytes a pair (`PIndexDataBlockD3D11_create` `0x00561e90`). `m_vertexData` (+0x28 count, +0x2c pointer) is a `PArray` of the `PDataBlockD3D11` objects themselves, not of pointers (`0x0055edf0`), one every 0x40 bytes (the class size) |
| `PDataBlockD3D11` | the same PSharray at +0x18 (count at +0x18, pairs at +0x1c; `PDataBlockD3D11_create` `0x005620c0`) |
| `PMeshInstance` | `m_mesh` +0, `m_localToWorldMatrix` +4, `m_name` +0x2c (`PString`) (`Phyre_PRendering_PMeshInstance_Bind` `0x004f38a0`) |
| `PMesh` | `m_meshSegments` +0 (count) and +4 (pointer to the segments) (`Phyre_PGeometry_PMesh_Bind` `0x00490250`) |
| `PNode` | `m_worldMatrix` +0xc (the same pointer as its instance's `m_localToWorldMatrix`), `m_name` +0x50 (`PNode_RegisterClass` `0x00505450`) |

`Phyre_PSerialization_PBinary_loadCluster` (`0x0057bce0`) is cdecl
`(stream, cluster, path, key)`: both `Phyre_PCluster_LoadAssetFile`s
(`0x004406c0`, `0x004407e0`) pass the file's path as the third argument.
The cluster's Direct3D objects are created inside that call, on the
loading thread. `loadCluster` calls the thunk `0x00465aa0`, which jumps
to `Phyre_PBinary_loadClusterTextures` (`0x00465ab0`). That function
makes the textures (`0x0058fc20`), index buffers (`0x00561e90`) and
vertex buffers (`0x005620c0`). It also calls
`PShaderFragmentProgramD3D11_create` (`0x0058d1c0`) and
`PShaderVertexProgramD3D11_create` (`0x0058d780`), so the pixel and
vertex shaders of `shaders/d3d11/*.fx*.phyre` are created during their
loads too. In one run, cluster loads created 2,981 of the 3,056 pixel
shaders and 1,395 of the 1,401 vertex shaders; the game makes the rest
outside cluster loads. So the device's create calls made on a thread
while a load runs on that thread belong to that file.

**`PNode.m_name` is at +0x50 in memory.** It is at +0x50 in the
namespace and in FFX.exe's own registration of the class
(`PNode_RegisterClass`, `0x00505450`: `m_next` 0, `m_parent` 4,
`m_firstChild` 8, `m_worldMatrix` 0xc, `m_localMatrix` 0x10, `m_name`
0x50), but every one of its array fixups (5,761 in a sample of 80
clusters) says +0x4c: the last float of `m_localMatrix`. All other
byte-offset fixups are at the member (strings) or its pointer word (+4,
arrays). The loaded node has its name pointer at +0x50 and
`m_localMatrix`'s last element (+0x4c) is untouched: 1.0 in every node
checked (see Verification). The loader does not write at the fixup's
byte offset.
`Phyre_PSerialization_PBinary_PInternal_PClusterReaderBinary_fixupDataPointers`
(`0x0057a320`) applies the array fixups. It sizes its record buffer from
the file header's `+0x18` (word 6, the cluster's total array fixup
count), and `Cluster_UnpackArrayFixups(..., 0)` fills it list by list
with each list header's `+0x18` count (see Array fixups). It resolves
each record to a member of the runtime class with `Fixup_FindMember`
(`0x0057a280`) and sets the array through that member. It prints "Array
fixup refers to array ... not in current runtime" when the member has no
runtime counterpart.

Bit 31 of word 0 (the field) decides how the field is named. With bit 31
clear, word 0 is a member index; with it set, `fixupDataPointers` passes
the low 31 bits as a byte offset and member index -1 (a byte-offset
fixup). `Fixup_FindMember` takes the member from the namespace's member
table when the index is below the member count, so -1 never takes that
branch. A byte-offset fixup gets its member from
`Class_FindMemberByOffset` (`0x0047d5f0`), called directly or through a
cache of the last class and offset. That function calls `0x0047d3e0`,
which looks up the member at the offset. Why the result is +0x50 for
`PNode` is not traced (see Open questions). ffx-phyre's fallback (the node's only array fixup)
reads the right string, so exports are unaffected.

### Character asset lists

`Ch_LoadHdRenderData` (`0x0082a1c0`) reads
`ffx_data/gamedata/ps3data/chr/<kind>/<name>/<name>.ahwin32`, a generated C
header (CRLF lines). Its D3D11 section lists the files to load, in order:
the textures, the model and the shaders; `const Phyre::PUInt32 <name> = n`
lines give the index of named files and `g_fileNamesCount` the count. A
model's imports resolve against loaded assets. A model importing Yuna's
texture with the texture added to Tidus's list shows it on the new
material (checked in the opening); without the entry the opening still
runs, but Yuna's own list loads that texture there, so that says nothing
about scenes without her. `ffx model import` therefore adds new textures
to the list: `ffx_phyre::assets::add_files` puts new entries first and
raises the count and the named indices.

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
triangles. The other 48 `.dae.phyre` files have no `PMeshSegment`.

For textures, 36,993 of the 39,497 segments have a default material, and
36,443 of them have a base color texture. All 22,792 texture references
resolve to files in the archive.

The glTF files were checked in three.js, with textures:

- Tidus (`c001`);
- a monster (`m009`);
- the Bikanel desert battle map (`bika00_a`);
- an airship scene (`swin02`);
- Luca (`luca01`).

All 1,266 exported `.glb` files pass the Khronos glTF validator
(2.0.0-dev.3.10) without errors or warnings. Rotating a joint of Tidus in
three.js deforms the mesh around it.

Writing every model back unchanged, directly and through its exported
`.glb`, reproduces the file byte for byte. Every tenth model is also
written with its first segment doubled and its last cut to one triangle,
and reads back with exactly those vertices and triangles. A `.glb` welded
by `gltf-transform` (m002: 5,934 → 1,565 vertices) imports and validates.

`cargo test -p ffx-oracle` unpacks the pointer and array fixups of 2,233
clusters, all models plus every 50th other cluster. It runs both the Rust
port and the original `0x60db20` / `0x60dac0` under emulation, and the
records must be identical.

The in-memory layout above was checked in the running game on
2026-11-02 by the runtime of ffx-remix, a separate modding project by the
same author that loads FFX through Fahrenheit and used these facts in game
(`runtime/ffxremix.runtime/Scene/AssetIds.cs`, `tools/probe-ids.sh`;
Fahrenheit alpha12 with ffx-rust's harness). The runtime hooked
`loadCluster`, tagged the device's creates made on the loading thread
during each load with its path, then walked the cluster with the offsets
above. It was checked in three frames, each in two runs, and the scene
draws' IDs were identical in both: the campfire opening at event counts
908 and 1208, and the Thunder Plains (`kami0000`, developer save 51) at
count 101. In those frames, a segment found this way named the index
buffer of every scene draw except the character shadow draw
(`chry0shadow`), whose index and vertex buffers are dynamic. Every
texture the draws sampled, other than render targets, came from a
`*.dds.phyre` load. 840 cluster loads, 588 segments, and 3,054 `PNode`s:
3,019 with a name pointer at +0x50, none at +0x4c, 35 without a name,
and +0x4c = 1.0 in all 3,054 (measured in the second run).

## Open questions

- What the remaining material parameters do in the shaders.
- New hierarchy joints (`PNode`s); new texture clusters (a new texture is
  made by copying one of the same size and format and importing pixels).
- Shaders (`*.fx.phyre`) and the other cluster kinds.
- Changing a texture's size or format.
- Why `PNode`'s name array fixup (+0x4c in the file) lands at +0x50 in
  memory: what `Class_FindMemberByOffset` (`0x0047d5f0`, through
  `0x0047d3e0`) returns for `PNode` at +0x4c.
