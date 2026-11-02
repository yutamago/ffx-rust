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
big-endian PS3 header (`"PHYR"`) and are not read (see
[The big-endian textures](#the-big-endian-textures)).

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
(`"PTexture2D\0DXT5\0"`). The texture object ends the instance data, so its
fields sit at fixed distances from the end of it (`Header::instance_data_end`):

| Class | Object | `m_mipmapCount` (+0xc) | `m_maxMipLevel` (+0x10) | `m_textureFlags` (+0x14) | Width (+0x1c) | Height (+0x20) |
|---|---|---|---|---|---|---|
| `PTexture2D` | end − `0x2c` | end − `0x20` | end − `0x1c` | end − `0x18` | end − `0x10` | end − `0x0c` |
| `PTextureCubeMap` | end − `0x20` | end − `0x14` | end − `0x10` | end − `0x0c` | end − `0x04` (square) | |

`m_mipmapCount` is the number of levels less one. The buffer is the mip
chain, largest first, per face for cube maps. Header word 20 is the size of
the top level (times six for cube maps) in every shipped texture.
**Rows are stored bottom first** (the PS3 version's convention): exported
as is, every texture with text is upside down. Export flips each mip level
upright; import flips it back (for DXT formats that means reversing block
rows and the pixel rows inside each block).

| Format | FFX | FFX-2 | DDS |
|---|---|---|---|
| DXT5 | 29,452 + 3 cube | 55,797 + 2 cube | `DXT5` |
| DXT1 | 13,404 + 107 cube | 15,932 + 89 cube | `DXT1` |
| ARGB8 | 1,062 | 557 + 1 cube | 32-bit, bytes B G R A |
| DXT3 | 335 | 195 | `DXT3` |
| L8 | 1 | 1 | luminance |

Widths and heights are the HD sizes; file names such as
`13184_19_0_0_128_64.dds.phyre` keep the PS2 ones. Of the 116,938 textures
(both archives), 51,477 2D textures have a single level, 333 have a side
that is not a power of two, and the largest is 4096x4096.

```bash
ffx texture export ffx_data/gamedata/ps3data/menu/ -o textures/
ffx texture import ffx_data/gamedata/ps3data/menu/d3d11/skip_us.dds.phyre skip_us.dds --mod-dir mods/my-mod
ffx texture import --resize ffx_data/gamedata/ps3data/menu/d3d11/skip_us.dds.phyre skip_us_4x.dds --mod-dir mods/my-mod
```

Without `--resize` an imported DDS must keep the original's format, size and
mip count, and replaces only the pixel data. With it (`Texture::with_any_dds`,
`Texture::rebuilt`, `Texture::with_rgba`) the DDS may have any size, mip
count and format of the table below.

### How the game loads a texture

`Phyre_PSerialization_PBinary_loadCluster` (`0x0057bce0`) reads the cluster
up to the buffers. `Phyre_PBinary_copyHeaderWords_004647a0` copies header
words 18, 19 and 20 to the loader, and `Phyre_PBinary_loadClusterTextures`
(`0x00465ab0`) then:

1. allocates **twice word 20** for the levels (one buffer for the cluster,
   without a bounds check on the reads into it);
2. gets the texture quality `q` (0 high, 1 medium, 2 low, `TextureQuality`
   in `GameSetting.ini`);
3. reads each level with `readExact` from the file: `bits × w × h / 8`
   bytes, `w` and `h` rounded up to 4 for block formats and halved per
   level down to 1. The size comes from the object (`m_width`, `m_height`,
   `m_mipmapCount`) and the bits from the format record, not from the file
   size: the file only has to hold that much;
4. drops the top `q` levels (reads them into the same place) when
   `m_mipmapCount >= q`. For a 2D texture `q` first backs off while the
   top level shifted by `q` is under 4 wide or high. A texture with fewer
   than `q + 1` levels keeps all of them: **a one-level texture is never
   reduced**;
5. calls `Phyre_PRendering_PTexture2DBase_setDimensions` (`0x004c40c0`)
   with the reduced size, and `PTexture2DD3D11_CreateDeviceTexture_0058fc20`,
   which creates the `ID3D11Texture2D`: `MipLevels` = `m_mipmapCount + 1`,
   format = the format record's `+0x18`, bind flag shader resource, no
   `GENERATE_MIPS` (a one-level texture stays one level). A texture whose
   format is the `UNKNOWN` record is error 7.

`setDimensions` fails (3) when the larger side is under `1 << levels-1`, and
with `m_textureFlags` bit 1 sets `m_maxMipLevel` from the size
(`GetMipCountForTextureSize`, `0x004c2430`). The shipped textures follow two
conventions: full chains have flag 0 and `m_maxMipLevel = m_mipmapCount`;
one-level textures have flag 2 and `m_maxMipLevel` = log2 of the larger
side. `CreateDeviceTexture` places the subresources with the sides halved
without the floor of 1 (a 0 side taken as the top level's), so for a
non-square texture the smallest levels it uploads are not the ones read;
those levels are a few pixels and the reads stay inside the buffer.
`Phyre_PBinary_reloadClusterTextures` (`0x00466da0`) does the same when the
quality changes.

### Formats the game accepts

The format name in the user fixup data is looked up among the 33 records of
`g_TextureFormats_00ca2fcc`, registered by
`PTextureFormatDesc_registerAll_004c1050` and filled by static constructors
through `Phyre_PRendering_Ctor8_005844a0`. A record holds the bits per pixel
(`+0xc`), the block flag (`+0x10` bit 0) and the `DXGI_FORMAT` of the
texture (`+0x18`). The ones useful for textures:

| Name | DXGI | Bits | Shipped |
|---|---|---|---|
| `DXT1`, `DXT3`, `DXT5` | `BC1_UNORM` 71, `BC2_UNORM` 74, `BC3_UNORM` 77 | 4, 8, 8 (blocks) | yes |
| `BC4`, `BC5` | `BC4_UNORM` 80, `BC5_UNORM` 83 | 4, 8 (blocks) | no |
| `ARGB8` | `B8G8R8A8_UNORM` 87 | 32 | yes |
| `RGBA8` | `R8G8B8A8_UNORM` 28 | 32 | no |
| `L8`, `A8` | `R8_UNORM` 61 | 8 | `L8` |
| `LA8`, `RG8` | `R8G8_UNORM` 49 | 16 | no |
| `A2RGB10` | `R10G10B10A2_UNORM` 24 | 32 | no |
| `RGBA16F`, `RGBA32F` | 10, 2 | 64, 128 | no |

The rest are 16-bit and float one- and two-channel formats, `RGBA16`
(`R16G16B16A16_UINT`), depth formats and `UNKNOWN`. There is **no BC6H, BC7
or sRGB format**: a BC7 texture has to go through a pack and the external
renderer, or be converted to BC3. `ffx-phyre` reads and writes `DXT1`,
`DXT3`, `DXT5`, `BC4`, `BC5`, `ARGB8`, `RGBA8` and `L8`; DDS files with a
`DX10` header are read too, sRGB formats as their UNORM twins.

### Changing size, format and mip count

Nothing else in the cluster refers to the texture's size or format, so a
new size, format or mip count rewrites (`Texture::rebuilt`):

- `m_mipmapCount`, `m_maxMipLevel`, `m_textureFlags` bit 1, `m_width`,
  `m_height`, following the two shipped conventions above;
- the format name in the user fixup data, its record's length, the later
  records' offsets and header word 13 (the fixup tables name user fixups by
  index, so they stay valid; everything after moves with the data);
- header word 20: the top level (times six for a cube map), or half the
  chain when that is more (only tiny block textures, where the shipped
  word 20 is short of it too);
- the pixel data.

Limits:

- **Size**: the device is made with feature level 11_0, 10_1 or 10_0
  (`PRendererD3D11_createDeviceAndSwapChain_00595800`), so a side may be up
  to 16384 at 11_0 (8192 at 10_x). `FFX.exe` is large-address aware; the
  loader holds twice word 20 while it reads. Sides need not be powers of
  two.
- **Block formats** (`DXT1`, `DXT3`, `DXT5`, `BC4`, `BC5`): the top level
  the game creates must be a multiple of 4 on both sides. This is
  Direct3D's rule, not the game's: "For mipmaps, the top level map is
  required to be a multiple of 4 size in all dimensions" (Direct3D 11.3
  Functional Specification, 19.5.1 "Overview" of the block compression
  formats; the lower levels may be any size and are padded to whole
  blocks). The specification gives it for every block format with no
  feature level exception, and the game only uses 10_0 and above.
  `CreateTexture2D` returns `E_INVALIDARG` for such a texture
  (`CREATETEXTURE2D_INVALIDDIMENSIONS` in the debug layer), and
  `PTexture2DD3D11_CreateDeviceTexture_0058fc20` passes `m_width` and
  `m_height` to it unchanged (`0x0058fd15`, `0x0058fd22`) and returns 9 when
  it fails (`0x00590009`, `0x005901f2`). The size it gets is the one after
  the texture-quality drop (step 4 above, `0x00466126`..`0x00466148`):
  `w >> q`, `h >> q` with `q` up to 2 (`TQ_LOW`, the last of the three
  `TextureQuality` values `Settings_SetValue`, `0x00401930`, accepts), for
  a 2D texture backed off only while a side would go under 4, for a cube
  map not at all. So a one-level block texture needs sides that are
  multiples of 4, a two-level one multiples of 8 (dropped by at most one
  level), and one with three or more levels multiples of 16, apart from
  small sizes the back-off saves (8x8 with 4 levels is created at 4x4).
  1000x1000 `DXT1` with 10 levels is created at 250x250 at `TQ_LOW` and
  fails; 1008x1008 works.
  `Texture::check_shape` refuses these sizes for every quality and names
  the size to use (`ffx_phyre::created_size` is the loader's computation).
  It lets a texture's own shipped format, size and mip count through, so
  an unchanged texture still writes back as it is.
  `Phyre_PBinary_reloadClusterTextures` (`0x00466da0`, when the quality
  changes while the game runs) drops `q` levels without the back-off
  (`0x00466f50`..`0x00466f70`); 28 shipped full-chain textures in the two
  archives with a side of 4 or 8 (8x8 with 4 levels, 128x8, 32x8, 64x4)
  would come out under 4 there too, so the check follows the load path.
- **Mips**: 1 up to a full chain; partial chains are allowed. Fewer than
  `q + 1` levels turn the texture-quality setting off for that texture.
- **Cube maps**: square, and at least two levels (so at least 2x2). No
  shipped cube map has one level, so the one-level flag convention is
  untested for them; `Texture::check_shape` refuses it.

The only shipped block textures with a side that is not a multiple of 4
are six full-chain (9 levels) 1x256 `DXT5` textures,
`ffx_data/gamedata/ps3data/help_{ch,de,es,fr,it,kr}/mon_boku/2/d3d11/2_256_0_1_257.dds.phyre`
(the `help` and `help_us` copies are `ARGB8`, one level). D3D11 would
refuse them at any quality: a 2D texture backs off to `q = 0` when a side
is under 4. The game never loads them. They sit next to slices of the
monster book's help pages (`mon_boku/<n>/d3d11/<n>_<x>_<y>_<w>_<h>`);
`MenuTextureManager_LoadMon_BokuTextures` (`0x0067cbd0`) loads the four
whole pages `PS3Data/help<_lang>/mon_boku/D3D11/<n>.dds.phyre` from the
table at `0x00c59dcc`, no format string in `FFX.exe` builds the slice
names, and no other file in `FFX_Data.vbf` or `FFX2_Data.vbf` names them.

Fahrenheit's file loader (`FhFileLoaderModule.init` and `h_fopen` in
`src/runtime/fileloader.cs`) hooks `Phyre_PSerialization_PStreamFile_ctor`
and the `BigFileStream` opens and points a game path at a file under a
mod's `efl/` folder; it does not touch the file's contents. A replacement
texture is therefore an ordinary `*.dds.phyre` that `FFX.exe` loads as above.

In the running game (2026-11-02, through ffx-remix's test path, a
Fahrenheit mod with the files under `efl/x/`):

- `chr/pc/c004/tex/d3d11/c004.dds.phyre` (Kimahri, 1024x1024 `DXT5`)
  as `RGBA8`, a format no shipped texture uses, 11 levels, recoloured:
  drawn recoloured in the campfire opening;
- `znkd06_wall_08.dds.phyre` (1024x1024 `DXT1`, 11 levels) at
  2048x2048 as one level of `ARGB8`: drawn, marked;
- `znkd06_sky_01.dds.phyre` (4096x1024 `DXT1`, 13 levels) at
  16384x4096, the feature level 11_0 limit, `DXT1` with 15 levels
  (44.7 MB): drawn, marked. The logo and title screens were the same
  pixels as without the files.

ffx-rust's own hook replaces `Phyre_PSerialization_PStreamFile_ctor`
(`0x00607bc0`, `ported/p1_load.rs`), the function Fahrenheit's file
loader hooks, so with the hook's ports on the loader's files are not
seen; these runs used `FFX_HOOK_ONLY=none`.

### The big-endian textures

The 30 textures with a big-endian PS3 header (`"PHYR"`, platform GCM) are
all in `ffx_data/gamedata/ps3data/magic/magic_0668/tex/d3d11/`. The folder's
`texlist.log` lists them as `R:/FFX_Data/GameData/PS3Data/magic/magic_0668/tex/GCM/…`:
the PS3 build's output, copied into the D3D11 folder. The folder holds
nothing else but the texture lists. This crate does not read them.

Each of the 30 has a PC copy: a little-endian texture with the same file
name in `magic_0611/tex/d3d11/` (all 30) and in `magic_0667` and
`magic_0515` (28 of them; not `14720_19_0_0_256_256` and
`14848_19_0_0_128_256`). All are `DXT5` with full chains at the HD size
(256x256 to 1024x512). The PC copy's pixel data (`Texture::pixels`) is the
last bytes of the big-endian file, byte for byte: for
`13312_19_0_0_256_256` the 349,552 bytes after the PC copy's 2,684-byte
header end the 353,562-byte big-endian file. The same pixels also appear
under other names, for example `magic_0705/tex/d3d11/13888_19_0_0_128_256`
for `14304_19_0_0_128_256`.

## Verification

`cargo test -p ffx-phyre` (with `FFX_GAME_DIR`) parses all 44,364 PC
textures, exports each to DDS and imports it back, and checks the result is
the original file byte for byte, directly and through the writer for new
sizes (`with_any_dds`). Every 20th texture is also written at twice its
size in another uncompressed format, once with a full chain and once with
one level (two for cube maps): it reads back with that size, format and
image, and writing the original image into it gives the original file again
(word 20 aside for a few tiny block textures, whose files match with word 20
masked). Samples of each format were rendered to
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
- Whether the game ever loads `magic_0668`'s textures.
- Why `PNode`'s name array fixup (+0x4c in the file) lands at +0x50 in
  memory: what `Class_FindMemberByOffset` (`0x0047d5f0`, through
  `0x0047d3e0`) returns for `PNode` at +0x4c.
