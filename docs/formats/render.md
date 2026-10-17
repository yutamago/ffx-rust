# The render layer's data

The remaster's `graphic*` layer sits between the PS2 game logic and
PhyreEngine. These are the structures it keeps that game code reaches;
the code is in [`crates/ffx-graphic`](../../crates/ffx-graphic), checked
against the originals by `crates/ffx-oracle/tests/render.rs`.

PhyreEngine and the C++ objects built on it (meshes, dynamic geometry,
clusters) are library code: the ports call their methods through
`ffx_graphic::host::Host`, never reimplement them. `Host::call_st0`
reads a callee's float result from `ST0` (as the caller's `fstp m32`
stores it), through float-returning function signatures, no inline
assembly.

## Effect render data

### `g_VFXRenderDataTable` (`0x00ccca6c`)

A `ClassVFXRenderDataTable`: a `std::map<u64, DynamicGeometry*>` from an
effect instance to its dynamic geometry, with a one-entry cache.

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | ptr | the map's head node |
| `0x20` | `u64` | the last key found (0: none) |
| `0x28` | ptr | its geometry |

Map nodes (MSVC's tree): `+0` left, `+4` parent, `+8` right, `+0xd`
is-nil (the head), `+0x10` the key, `+0x18` the geometry. Missing
children point at the head. `ClassVFXRenderDataTable_MapFind`
(`0x006a1f60`) is the tree's `find`;
`ClassVFXRenderDataTable_GetDynamicGeometryByInstance` (`0x0069f620`)
and `..._GetVFXDataByInstance` (`0x0069f6e0`) look through the cache
first and refill it on a hit (a miss leaves it).

An instance key is 64 bits (low word, high word as the effect code passes
them). An effect of several parts keeps part `i` at `key + i * 10000`
(the sum carried into the high word: `cdq; add; adc`).

### Dynamic geometry

A C++ object (vtable at `+0`):

| Offset | Type | Meaning |
|---|---|---|
| vtable `+0x08` | method | `update(flags)` (thiscall), called by `graphicVFXUpdate*` |
| vtable `+0x0c` | method | `updateTexture()`, tail-called by `graphicDynamicGeometryUpdateTexture` |
| `0x24` | ptr | the render data |
| `0x51` | `u8` | 1: skipped on the filtered levels below |

### Render data (geometry `+0x24`)

| Offset | Type | Meaning |
|---|---|---|
| `0x90` | `i32` | segment count |
| `0x94` | ptr | segments, `0x6c` bytes each (`+8` a count `InitVFXRenderDataShp` halves after set-up; `+0x20` the texture name) |
| `0xb6` | `u8` | from the effect object's `+0x44` (`updateVFXRenderData`) |
| `0xc0` | `i8` | parts kind: 0 one part, 1 four parts, 2 `+0xc4` parts |
| `0xc1` | `u8` | adds `0x40000000` to the update flags |
| `0xc2` | `u16` | its model buffer index (`ModelBuffer_*`) |
| `0xc4` | `u16` | part count (kind 2) |
| `0xc8` | `u32` | part mask: part `i` updates when bit `i` (rotating) is set |

### `graphicVFXUpdateWithType(lo, hi, flags)` (`0x00645ba0`)

Nothing while `g_GraphicSkipFrame` (`0x012fb7d0`) is set or for the null
key. While `g_GraphicLevelFilter` (`0x00ccb460`) is 1 and the current
level hash (`g_PhyreScene +0xfaf4`) is one of 21 levels
(`ffx_graphic::vfx::FILTERED_LEVELS`), an instance whose first geometry
has `+0x51 == 1` is skipped. Otherwise each part the mask selects gets
`update(flags | (0x40000000 if +0xc1))`. `graphicVFXUpdate` passes
flags 0, `graphicVFXPath3Update` `0x80000000`.

### Set-up (`InitVFXRenderData` and the shape and path-3 variants)

`createVFXRenderDataBuffer` (shape: `...Shp`, with the shape in place
of the object), then `graphicVFXSetup`; on success `updateVFXRenderData`
(or `...Shp`) and the update; on any failure `graphicVFXDestroy`. The
shape variants halve every segment's `+8` between the set-up and the
fill, whether the set-up succeeded or not.

### `updateVFXRenderData(lo, hi, part, obj)` (`0x00718610`)

Fills an instance's render data from a particle part. Returns 1 (also
when the frame is skipped), 0 when the instance or a part has no render
data or a buffer could not be had.

The part's shape (`part +0x1c`): `+4` offset of the segment headers,
`+8`/`+0xc` offsets of default data for `obj +0x38`/`+0x3c` (set for the
call, restored after), `+0x10` (`i16`) the primitive total. Each header is
`0x10` bytes (`+1` type, `+2` `i16` primitive count `n`, `+8` texture word,
`& 0x3fff`) followed by its primitives:

| Type | Vertices, indices | Bytes per primitive | Per part |
|---|---|---|---|
| 0 | `3n`, `3n` | `0x14` | yes |
| 1 | `3n`, `3n` | `0x1c` | yes |
| 2 | `3n`, `3n` | `0x20` | yes |
| 3 | `3n`, `3n` | `0x28` | no |
| 4 | `4n`, `6n` | `0x18` | yes |
| 5 | `4n`, `6n` | `0x20` | no |
| 6 | `4n`, `6n` | `0x28` | yes |
| 7 | `4n`, `6n` | `0x30` | no |

Other types take no buffer and no bytes. For each header (until the
total is used up, at most 28 segments) `graphicDynGeoMemManagerGetBufferForSegment`
is asked for the counts. "Per part" types on a kind-1 effect ask for
each of the four parts and clear its segment's `+4`/`+8`; on a kind-2
effect each part reuses its segment's own `+4`/`+8` and clears its
buffer (`+0x1c`, `+8 * 2` bytes); everything else asks for the first
part only.

The object (a particle program's draw object): `+0` flags (bit 6: point
light), `+8` a byte (`0x48` with the alpha rule below), `+0x10`/`+0x14`
UV offset and RGBA power words, `+0x18..0x1b` colour bytes, `+0x20` a
16-word block, `+0x24` the light colour, `+0x28` the light matrix,
`+0x2c` the camera matrix, `+0x44` an optional 16-word block, `+0x4d`
the light power (0..255).

- The colour is `byte * g_VfxColorScale[i]` (one `f32` rounding each).
- `g_AlphaScale` is 512 for magic files 0x66/0x67 with texture 0x3400,
  and for 0x203/0x29b with texture 0, object byte `0x48`, program 0xb and
  `g_VfxPhase == 0x1f`; else 256. `g_AlphaScaleInv = 0.0078125 / it`.
  Magic file 0x176 with textures `0x3780`/`0x3800`/`0x3880` sets the
  colour bytes to `0x30`.
- Magic file 0x207 with the point-light flag: the light matrix taken
  through the inverse camera matrix row by row (`w` = 1 for the product,
  then the light's own `w`), handed with the colour and
  `power / 255` to `graphicVFXSetPointLightParameters` of every part.
- With `g_VfxYiqMode != -1` on map 0x188 every part gets YIQ.

The fill request (`JobSchedule_pppPart_getFillBufferRequest(total)`,
while the fill buffer is valid):

| Offset | Value |
|---|---|
| `0x000` | 0 (`u8`) |
| `0x001` | `obj +0x44 != 0` |
| `0x010` | `obj +0` |
| `0x014` | `obj +0x38` (as set for the call) |
| `0x018` | the shape |
| `0x01c` | the first texture word (`u16`) |
| `0x01e` | `obj +8` (`u8`) |
| `0x020` | 16 words from `*(obj +0x20)` |
| `0x060` | 16 words from `*(obj +0x44)` when set |
| `0x0a0`, `0x0a4` | `obj +0x10`, `+0x14` |
| `0x0b0` | the colour (4 floats) |
| `0x0c0..0x0cc` | the parts' render data (one part: `+0xc0` only) |
| `0x0d0..0x0ec` | the parts' geometry, twice |
| `0x0f0..0x0fc` | the parts' geometry (one part: `+0xf0` only) |
| `0x100` | `obj +0x3c` (point light) |
| `0x104` | 0 |
| `0x108` | `g_VfxPass` |
| `0x110`, `0x150`, `0x190` | camera, light and colour blocks (point light) |
| `0x1d0` | `obj +0x4d` (point light) |
| `0x1d4` | 16 words of `g_pppFillMatrix` |

### `updateVFXRenderDataShp(lo, hi, part, obj, segment)` (`0x0071b8a0`)

The shape variant. `*(part)` is the shape; `segment` (`u16`) picks the
segment whose `i16` offset sits at `shape + 0x10 + 8 * segment`. A few
magic files set `obj +4..7` to `0x80` grey for one texture each (0x12e:
`0x3960`; 0x274/0x99/0x9a: `0x3400`; 0x259/0x98/0x215: `0x39a0`;
0x123/0x154: `0x3800`). On level `0x76f` with `g_VfxYiqMode` 3..5 a
depth (`*(obj +0x10) +0x38`) below 300 becomes -10000. The first part's
segments' counts are cleared; then either the whole primitive count goes
to the first segment (`+8 += 6n`, `+4 += 4n`, kind 0), or each primitive
(8 bytes from `segment + *(segment +4) + 4`, its first byte naming the
part: `0x48` 1, `0x42` 2, `0x46` 3, `0x41` 4, `0x88` 5, else 0) adds
6 and 4 to its part's first segment and its bit to the mask `+0xc8`.
The fill request: `+0` 1, `+1` `obj +0x1c != 0`, `+0x10` the segment,
`+0x14..+0x28` the six parts' geometry, `+0x2c..+0x40` their render data,
`+0x44` `obj +2` (`u16`), `+0x48` 16 words from `*(obj +0x10)`, `+0x88`
16 from `*(obj +0x1c)` when set, `+0xc8` `obj +4`, `+0xcc` the kind,
`+0xd0`/`+0x110`/`+0x150` `g_pppFillMatrix`/`2`/`3`. Every part the mask
names (up to `+0xc4`) then gets a buffer per segment.

## `g_PhyreScene` (`0x00ccb9d8`)

The `PhyreScene` object. Fields the game's `graphic*` calls use:

| Offset | Type | Meaning |
|---|---|---|
| `0x0000` | `u8` | minimap in blitzball (`graphicIsMiniMapInBlitzBall`) |
| `0x0003` | `u8` | the magic 0146 key (`graphicGetMagic0146Key`) |
| `0x0074` | `u32` | minimap type |
| `0x0078` | `u8` | minimap enabled |
| `0x0079` | `u8` | minimap fog enabled |
| `0x00ac` | ptr | a cell: `+0` pointer, `+4` dead flag |
| `0x00d4` | `u8` | map alpha fast |
| `0x00dc` | ptr | the camera: `+0xb8` near plane, `+0x154` FOV y (floats) |
| `0x0dd8`, `0x0ddc`, `0x0de0` | ptr | video texture state (`getVideoTexture` is `+0xde0`) |
| `0x1195` | `u8` | field map loading |
| `0x1196` | `u8` | battle map (now) |
| `0x1197` | `u8` | battle map (before the last `graphicSetIsBattleMap`) |
| `0x10c9` | `u8` | texture video playing |
| `0xf9bc` | `f32` | cut 2D scene bias |
| `0xf9c7` | `u8` | drawing a UI 2D scene |
| `0xf9cc` | `u32` | forced shadow solution |
| `0xfa34`, `0xfa38` | `f32` | screen offset |
| `0xfaa8`, `0xfacc` | object | two sampler states (`PhyreScene_getLinearSampler(2)` is the second) |
| `0xfaf0` | `PString` | current level name (pointer; bit 0 a flag, masked off) |
| `0xfaf4` | `u32` | current level name hash |
| `0xfaf8` | `u32` | previous level name hash |
| `0xfafc` | `PString` | current magic path |
| `0xfb00` | `PString` | magic path to delete |
| `0xfb45` | `u8` | set by `graphicSetSceneFlagFb45` |
| `0x10d2d` | `u8` | map parameters need a reset |
| `0x10d2e` | `u8` | in the Luca theater |
| `0x10d8c` | `u32` | split-screen state |
| `0x10d90` | `u32` | split-screen type |
| `0x10dcc` | `u8` | fading (with `g_MovieFading`) |
| `0x10dd0` | `i32` | screen width (`graphicFontGetEdgeRatio`: width / 1920, at most 1) |
| `0x1232c` | ptr | a cell like `+0xac` |

## UI coordinates

The remaps take a remaster coordinate to the PS2 screen (512 x 416):
`graphicUiRemapX2`/`Y2` from 1920 x 1080, `graphicUiRemapX`/`Y`
(`0x006447c0`/`0x00644800`) from 1280 x 720. Each is
`f32(x * 512.0 / 1920.0)` with both operations on the x87 at 53 bits
(two `f64` roundings, then one to `f32`).

## The model cache (`Cache_*`)

`g_CacheSlots` (`0x01302f98`): 100 pointers to entries, least recently
used first. `Cache_ToTop` (`0x00840cf0`) moves an entry to the last slot
and sets its age to 5 unless it is pinned (7).

| Offset | Type | Meaning |
|---|---|---|
| `0x00` | `u32` | id (model number) |
| `0x04` | `i8` | kind: 1 character, 2 monster, 3, 4 |
| `0x05` | `u8` | in use (`Cache_Use` / `Cache_Unuse`) |
| `0x06` | `u8` | not read yet (`Cache_Entry_NotYet`; `Cache_Exist` asserts on it) |
| `0x07` | `u8` | locked (`Cache_SetLock`) |
| `0x08` | `u8` | bits 0-2 age (7 pinned), bit 6 unused |
| `0x0c` | ptr | the data; 0 marks a free entry |
| `0x10` | ptr | render data: the model |
| `0x14` | ptr | render data: the second model |
| `0x18` | ptr | cloth mesh data |
| `0x1c` | ptr | cloth mesh mapping data |
| `0x20` | ptr | cloth vertex buffer |
| `0x24` | `u32` | release when read (`Cache_Clear` sets it on unread entries) |

Lookups match id and kind (the kind byte sign-extended); most also need
`+0x0c` set, `Cache_Exist3` and the cloth lookups need their own field set
instead, and `Cache_Entry_RenderData` / `Cache_Entry_ClothVertexBuff`
take any entry. `Cache_Use` types the data's memory 2 (characters) or 3
(monsters) through `Sg_MemSetType`, `Cache_Unuse` and `Cache_Entry` type
it 4. `Cache_Entry` takes the first free entry with flags 5, calling
`Cache_OverFlow` to free the oldest until one is free or it fails.

## Managers

| Global | Object |
|---|---|
| `0x00ccc8a0` | `PostProcessManager` (`+0x123b0..+0x123bb` switches, `+0x123bc` cross-fade weight, `+0x123c0`) |
| `0x00ccc89c` | `PostProcessTweakUtil` |
| `0x00cccaa8` | `TextureAnimationManager` (`+8`, `+0xc` bases, `+0x74` material count (bit 31 a flag), `+0x78` materials) |
| `0x00cdec28` | `ShadowManager` (`+0x60` cascades, `+0xfe8` 64 words, `+0x3fc4` the texture sampler, `+0x4018`/`+0x401c` floats) |
| `0x00cdec1c` | `SpecularMapParamsManager` |
| `0x00cccab0` | `DebugMenuManager` |
| `0x00cccb00` | the pause-game object (`graphicCheckPauseGame`) |
| `0x00cdedf0` | the dynamic-geometry fill buffer |

A `DynGeoMemManager` holds a buffer set at `+0`: `+0` the current
buffer's fill pointer, `+4` its index, `+8` the count, `+0x10` the
buffers (each `+0` size, `+4` start, `+8` fill position, an address),
`+0x14` a mutex.
`DynGeoMemManager_getBufferFromDynGeoMemory(size)` (`0x006de940`) rounds
the fill position up to 16 and hands out `size` bytes there; past the
buffer's end it prints to `stderr`, leaves the rounded position and
returns 0.

`graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices,
indices)` (`0x0063fdb0`) takes one block from the fill buffer's manager
for five arrays: 12, 16, 12 and 8 bytes a vertex and 2 an index, each
rounded to 16, in that order. The segment (`0x6c` bytes) records them:

| Offset | Value |
|---|---|
| `0x04`, `0x08` | vertex and index counts |
| `0x0c` | the first array (the block) |
| `0x14` | the second |
| `0x10` | the third |
| `0x18` | the fourth |
| `0x1c` | the indices |
| `0x2c + 0x1c n` ... `0x44 + 0x1c n` | the same counts and arrays again in buffer `n`'s slot (`n` the current buffer: counts `+0x2c`/`+0x30`, arrays `+0x34`, `+0x3c`, `+0x38`, `+0x40`, `+0x44`) |

`DynGeoTree_Find`/`_Insert` (`0x006dedd0`/`0x006dee50`) keep a search
tree under a head (root at head `+0xc`; nodes `+0` lower, `+4` higher,
`+8` parent with a colour bit, `+0xc` the `u32` key).

A texture animation steps its frame (`+0x18`) modulo its frame count
(`+8`, bit 31 a flag).

## The minimap viewport

`PhyreScene_getMinimapViewportSize`/`X`/`Y` (`0x0065e320`, `0x0065e3f0`,
`0x0065e560`) scale with the render target's height `h` (`+0x1c` of the
live pointer at `*(scene +0x10cc) +0x8c`), truncating:
size `h * 120.25 * 9 / 16 / 416` in blitzball (scene byte 0), else
`f32(104 h) * 9 / 16 / 416`; x `f32(h) * base / 512` with base 39 for
minimap types 1 and 2, 368.75 (blitzball) or 377 for 3 and 4, else 0;
y `f32(h) * (111 in blitzball, else 96) / 512`.

## Texture-animation packets (`tex_anim_*`)

The PS2 code writes GS registers; in the remaster every write lands in
one word, `g_GsRegisterSink` (`0x023056e8`). `loadImage(addr, bytes)`
writes the address (minus `0x80000000` from `0x70000000` up), `bytes /
16` and the command `0x100`; `TexAnim_LoadImage101` the same with
`0x101`; `TexAnim_SyncPath`/`2` `a, 0, 0x105`. `TexAnim_SetDmaTag` packs
`(((((a << 31) | (b & ~0xf)) << 1 | c) << 3 | d) << 2 | e) << 26 | f`;
`TexAnim_SetGifTag` writes `[p8 << 15 | p9, ((((p4 << 2 | p5) << 11 | p6)
<< 1 | p7) << 14, p2, p3]`. `g_ChrTexAnim` (64 x `0x30`): `+0` loaded,
`+4` the work buffer (64 animations of `0x34` bytes, active bytes at
`+0xd08`), `+0xd` the format (0 old, 1 new), `+0xe` the timer.

## Scene state

What `ffx-render` needs from the scene stream lives in a few objects the
`graphic*` calls set (`crates/ffx-graphic/src/state.rs`).

`g_PhyreApplication` (`0x00cc9cd8`, `Phyre_PFramework_PApplication_GetApplication`):

| Offset | Value | Set / read by |
|---|---|---|
| `0x50` | global ambient colour (3 floats) | `graphicSetGlobalAmbientColor` (`0x00643270`), `graphicGetGlobalAmbientColor` (`0x00641230`) |
| `0x60` | fog colour (3 floats) | `graphicGetFogColor` (`0x00641160`) |
| `0x70`, `0x74`, `0x7c` | fog near, far, limit | `graphicGetFogNear`/`Far`/`Limit` (`0x006411d0`, `0x006411b0`, `0x006411c0`) |
| `0x80` | background colour (3 floats) | `graphicSetBackgroundColour` (`0x00642e90`) |
| `0x26c`, `0x270` | window width and height (`u32`) | `PhyreScene_setCameraAspectRatio` |
| `0x3ac` | keyframe animation on (byte) | `graphicFFXApplicationEnableKeyframeAnim` (`0x006401e0`) |

The screen fade, in `PostProcessManager` (`0x00ccc8a0`): `+0x1238c` the
colour (r, g, b, alpha floats), `+0x1239c` the alpha mode, `+0x123a0` a
lock byte (`graphicLockScreenFadeColor` `0x00641e20`; while set,
`graphicSetScreenFadeColor` `0x00643900` and `graphicSetScreenFadeAlphaMode`
`0x006438e0` do nothing; `graphicUnlockScreenFadeColor` `0x006448d0`
clears it and the alpha), `+0x123a4` the last reason. `+0x123a8` and
`+0x123c5` are "draw filter / draw focus ran this frame" bytes.
`graphicSetFocusZ(which, z)` (`0x006430f0`) turns a PS2 z-buffer value
into the depth-of-field focus `((z - 16770000) / -16769999) * 2 - 1`
(`z` 0 gives -2) for `PostProcessManager_setFocusDepth`.

Cameras (scene `+0xdc` main, `+0x654` minimap, `+0x238` reflection when
scene `+0x10d8c` is 1; a camera's `+0xbc` far plane, `+0x150` aspect,
`+0x154` vertical FOV, virtual 1 its update):

- `graphicCameraSetWorldMatrix(view)` (`0x00639860`, the matrix
  `Cam_Apply` passes, `0x02311440`) inverts the game's 4x4 view
  (`sceVu0InversMatrix4x4`), negates x and y of rows 0 and 3 and z of
  rows 1 and 2 (as 14 fld/fchs/fstp: rows 1 and 2's x and y twice), and
  stores `PMatrix4_toWorldMatrix3x4` of it
  at scene `+0xfa04`. A row that is all zero bits makes it use the
  identity; `g_GraphicSkipFrame` set skips the call.
- `PhyreScene_setCameraWorldMatrix(m, alt)` (`0x0066ce70`) copies a 3x4
  matrix to `+0xfa04`, or with `alt` to `+0xfa3c`; the byte `+0xfaa1`
  says which.
- `PhyreScene_setMinimapCameraWorldMatrix(m)` (`0x0066d940`) copies it to
  `+0x7b0`, points the minimap camera's `+0xb4` there and updates it.
- `PhyreScene_setCameraFOVy(fovy)` (`0x0066cda0`) sets the main and
  minimap cameras; in a 3D scene (`+0x93c`) map `0xec1` forces
  `0x3ef126e9` and the minimap camera gets `0x3f2c49ba` on `0x107`,
  `0x3f5126e9` on `0xf48`.
- `PhyreScene_setCameraAspectRatio(a)` (`0x0066cbe0`): in a 3D scene
  `a * 3 * 0.25 * 16/9`; a reflection camera of kind (`+0x10d90`) 0 or 1
  gets that times 0.5 or 2, and then the main camera `window_w * a *
  0.75 / window_h` times the same.
- `PhyreScene_setAbmapCamera(x, y, z, h, scale)` (`0x0066ca60`): the
  area-map camera (`+0xbe0`, world matrix `+0xd34`, pointed at by
  `+0xc90`) looks from `(-z*1920/512, x*1080*16/13/scale - h*1080*4/1536,
  1 - y*1080*16/13/scale)` to `(-z*1920/512, -h*1080*4/1536, 1)`, up
  `(0, y, x)`.
- `setFarPlane(f)` (`0x00645e90`): the main camera's far plane.

Meshes: `PhyreScene_setMeshInstanceWorldMatrix(mesh, m)` (`0x0066d810`,
stdcall) negates x and y of every row of the game's 4x4 and converts it
to the mesh's 3x4 world matrix (`*(mesh +4)`); the draw entry
(`*(mesh +0x1c) +0xc`) points at it.
`graphicCharacterInstanceSetSkeletonMatrices(chr, list, bones)`
(`0x0063be60`): a character's instance list is a count (bit 31 a flag)
and pointers to its meshes' instances; instance `+0` its skeleton
(`+0x18` bone count, `+0x1c` default matrices), `+0xc` its skin
matrices; `list +0x10 +0x14` holds per mesh a bone-index table into the
game's 64-byte bone matrices. Character `0x206a` keeps its own matrices
from bone `0x42` up. `...RestoreDefault(list, bone)` (`0x0063bfc0`)
copies one bone's default back (the first mesh having it, from table
entry 1). `graphicCharacterInstanceSetVisible(list, v)` (`0x0063c390`)
sets `list +0x14` and each mesh's visibility.

Object translation: `PhyreScene_setObjectTranslation(id, pos)`
(`0x0066e3e0`), with the object tree up (`+0x10d4`), finds `id` in the
tree at scene `+0x1198` (`ObjectTree_Find`, `0x0064b9f0`; the node's
`+0x10` a list: `+0` count, `+8` meshes) and gives each mesh's 3x4 world
matrix (`*(mesh +4)`) the translation row `(-x, -y, z)` (words 8-10,
the others rewritten as they were); each mesh's draw entry (`*(mesh
+0x1c) +0xc`) then points at it. Returns 0.

Shade coefficient: `graphicCharacterLightingSetShadeCoeff(list, c)`
(`0x0063cdd0`) takes `Phyre_CharacterLighting_calcShadeCoeffWithConfidence(list,
c, 0)` (an `ST0` result), forced to 1.0 on maps `0x226d` and `0x1739` and
back to `c` on `0x1c45`, `0xf62` and `0xc04` (the current level hash), sets
it on each character and reports it to the tweak utility's debug hook
with mode 1 when `|result - c| > 0.1`, else 2.

2D layers: scene `+0x1140` an array of `+0x1138` entries of 8 bytes (`+0`
shown, `+1` group (signed byte), `+2` id (signed `i16`), `+4` the mesh),
`+0x11d8` the current group, `+0x1128` non-zero when there are layers.
`PhyreScene_show2DLayer`/`hide2DLayer(id)` (`0x0066f2f0`/`0x0065f050`)
set the first matching entry and its mesh's draw pointer (`+0x1c` of
the mesh's `+0x1c`: the mesh, or 0).

`graphicConvertScreenCoords(x, y, *fx, *fy)` (`0x0063de10`) maps PS2
screen units to the 1920x1080 frame: `x * 3.75`, `y * 2.5961537`.

## Replaced by ffx-render

From round 5 the render layer ports only game state and what the scene
stream needs. These functions only feed PhyreEngine or the D3D11 side;
`ffx-render` replaces them, so they stay original until the renderer
takes over, and are not ported:

- `Array_Reserve_*` (`0x00667e00`..`0x00668380`, 12): `PArray` growth.
- `BlurEffect_GetInstance`, `BlurEffectEx_GetInstance`,
  `Distortion_GetInstance`, `MotionBlur_GetInstance`,
  `Distortion_resize`: post-effect objects.
- `DynGeoMemManager_lockForRecordNumInKeLns`/`unlock...`/`swapDynGeoMemory`,
  `graphicDynGeoMemManagerClearBuffer`/`GetBuffer`/`GetCurrentBufferId`:
  vertex-buffer memory.
- `PhyreScene_ChangeToHSVPass`, `ForceTransparent`, `UseVFXOptimizedTrans`,
  `createRWTTTarget`, `setEnableRenderWorldToTexture`,
  `setMaterialColourScale2`, `setupPostProcessParams`, `updateMapYIQ`,
  `startLoadingScreen`: render passes, targets and material constants.
- `PhyreScene_attachChrClusterTo3DWorld`, `deletePMeshInstanceInCluster`,
  `removeMeshInstance`, `removeVFXFromRenderArray`,
  `PhyreScene_isVFXTextureLoaded` (both, `0x006604d0`, `0x00661c80`),
  `unloadAtelVFXTexture`, `onCutbufferShaderLoaded`, `onFontTextureLoaded`,
  `onMenuTextureLoaded`, `getMonsterEtTextureName`, `convertMapPath`:
  PhyreEngine clusters, render arrays and asset loading.
- `PhyreScene_UpdateTextureVideo`, `updateTextureVideoCallback`,
  `graphicTextureVideoPlay`: the video texture.
- `PhyreScene_getViewProjectionMatrix`: PhyreEngine's camera matrices
  (`ffx-render` builds its own from the camera state above).
- `PostProcessManager_renderScreenFade`, `setHDAOSetting`,
  `RadialLine_destroyRadialLine`, `RenderDelegationHelper_setDataToFreeList`,
  `ShaderParamBuffer_SetValue`/`SetValue2`, `graphicShaderGetVertexInfo`,
  `ShadowManager_InsertIntoShadowMap`/`_Double`/`createshadowbuffer`,
  `TextureAnimationManager_sceenTextureAnimationClear`/`unregisterChararcterInstance`.
- `graphicSetHSVParam`, `graphicCharacterInstanceSetBTUFadeCenter`: shader
  parameters (`hsv_scale`, `GradualFade_*`) written into material buffers.
- `graphicCharacterInstanceUpdateSkeletonounds`, `graphicCharacterSetClipPlane`,
  `graphicCharacterRestoreSummonTexture`, `graphicCharacterLightingSetLightMatrix`,
  `graphicPS2LightingSetupEffect`, `graphicAbmapDestory`: PhyreEngine-side
  model, texture and light objects.
- `graphicMagic0708ConvBallChr`, `graphicMagic_DrawColorModel_KaMdldrw`:
  draw an effect's model through PhyreEngine.
- `tex_anim_no_set_old`, `texanim_malloc_memory_set`: PS2 GS packets.

Platform, not render state (the platform layer replaces them):
`graphicGetTime`, `graphicGetTimeInMicroseconds`, `graphicSetFlipVsnc`,
`graphicSetForceNoVsync`, `graphicProcessWindowsMessage`,
`graphicGetFileReadSemaId`, `graphicGetOldFileReadCompleteSemaId`,
`graphicAsyncTaskCallback`, `getCurrentControlMode`, `MagicDLLAsyncLoad`,
`PhyreFIOS_LoadPrxInProgress`, `PhyreFIOS_UnloadPrx`,
`PhyreFIOSStreamFile_seek`.

Already ported in rounds 1-4 and glue by this rule (kept, since their
parity is tested, but `ffx-render` will not call them): the
texture-animation packet writers (`loadImage`, `TexAnim_*`,
`tex_anim_*`: PS2 GS register writes into `g_GsRegisterSink`), the
manager singleton accessors and deleters, `DynGeoMemManager_*` buffers
and trees, `PhyreScene_getLinearSampler`, the `PostProcessManager`
switch setters (`SetByte123b*`, `setAllSwitches`, `isEffectAllowed`,
`setCrossFadeCaptureEnable`), `ShadowManager` map bookkeeping,
`SpecularMapParamsManager_needDrawSpecularPass`, `Distortion_updateVertex`.

Dead by `symbols/classes.tsv` (no reference anywhere in the image), so
not hooked; their behaviour above is still checked by the oracle test:
`graphicCharacterInstanceRemoveFromPipeline`, `graphicPushTip`,
`graphicConvertScreenCoords`, `graphicCharacterLightingSetPointLightColor`/
`Position`/`Range`, `setFarPlane`.

## Debug code

Not ported; listed for mod tools:

- `DebugMenuManager_CreateInstance` (`0x006b4950`): creates the
  developer debug menu singleton (`g_DebugMenuManager`).
- `graphicCharacterLightingSetShadeCoeff_debug` (`0x0063cec0`): forces
  each character's shade coefficient (1.0 on three maps, else the
  tweak default) and reports it to `CharacterLightingTweakUtil`'s debug
  hook.
- `graphicCharacterLightingSetShadeCoeffUnused` (`0x0063cf70`, no
  callers): the shade-coefficient setter's older variant, also with the
  tweak-util debug hook.
- `graphicCharacterInstanceOutputAllVertexBuffers` (`0x0063b290`):
  prints every mesh, segment, vertex stream and its values of a
  character to stdout (a vertex-buffer dump; a model-export start).
- The tweak-util hooks the shipping code still calls
  (`Phyre_CharacterLightingTweakUtil_setShadeCoeff_dbgHook`,
  `PostProcessTweakUtil`): live tuning of lighting and post-processing.

## Helpers

Small functions the render layer's callers rely on
(`crates/ffx-graphic/src/util.rs`):

- `ClipOutcode(v)` (`0x00652470`): for `(x, y, z, w)`, bits 0-2 set when
  `x - w`, `y - w`, `z - w` (each stored as a float) are above 0, bits
  3-5 when `w + x`, `w + y`, `w + z` are below 0; a NaN sets nothing.
- `graphicIsMapInList()` (`0x00642610`): 1 for the map ids `0x118`,
  `0x52`, `0x51`, `0xde`, `0x13c`, `0x17d`, `0x5d`, `0x12`, `0x15d`,
  `0xf8`, `0x99`, `0xb3`, `0x50`, `0x137`, `0x16b`, `0x84` (in
  `0x01330844`).
- `ShadowManager_GetCascadeMatrix(out, i)` (`0x006d23a0`): the 4x4 at
  `+0x210 + 64 i`.
- Containers: `Vec3_UninitializedCopy` (`0x006b4f80`; a null destination
  skips only the first element), `FloatHeap_AdjustHeap` (`0x006c7250`,
  `std::_Adjust_heap` with a `less` callback; floats move through
  `fld`/`fstp`), `MapIterator_Decrement` (`0x0064f360`), the search-tree
  finds `ObjectTree_Find`, `UIntTree_Find` (`0x006be700`, the same code as
  `DynGeoTree_Find`) and `PStringTree_Find` (`0x006be750`, by `PString`
  text: the pointer without its flag bit, `strcmp` order, by pointer when
  either is null).

Not ported: `0x0064e9e0`, a `PString` buffer release, decrements its
reference count with `lock xadd`; game memory access through `Mem` is
not atomic, and loader threads share strings, so it stays original.
