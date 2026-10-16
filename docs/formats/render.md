# The render layer's data

The remaster's `graphic*` layer sits between the PS2 game logic and
PhyreEngine. These are the structures it keeps that game code reaches;
the code is in [`crates/ffx-graphic`](../../crates/ffx-graphic), checked
against the originals by `crates/ffx-oracle/tests/render.rs`.

PhyreEngine and the C++ objects built on it (meshes, dynamic geometry,
clusters) are library code: the ports call their methods through
`ffx_graphic::host::Host`, never reimplement them.

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
| `0xfaa8`, `0xfacc` | object | two render targets (`PhyreScene_getRenderTarget(2)` is the second) |
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
used first. `Cache_Touch` (`0x00840cf0`) moves an entry to the last slot
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
buffers (each `+4` start, `+8` fill position), `+0x14` a mutex.
A texture animation steps its frame (`+0x18`) modulo its frame count
(`+8`, bit 31 a flag).
