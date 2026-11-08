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

A pointer to a `ClassVFXRenderDataTable` (`graphicVFXGetRenderDataByInstance`
loads `mov ecx, [0x00ccca6c]` at `0x006453e1` before the thiscall
`ClassVFXRenderDataTable_GetVFXDataByInstance`): a
`std::map<u64, DynamicGeometry*>` from an effect instance to its dynamic
geometry, with a one-entry cache. The offsets below are the table's.

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

## Character light levels

How the PC lights characters (`phyrechrlitshader.fx`, every lit
variant; read from the shaders by ffx-demaster, `docs/render.md`):

```
lit = GlobalAmbientColor + CharacterAmbientLight + CharacterShadeColor x CharacterShadeCoeff
    + sum over i = 0..2 of Light_i.m_colorIntensity x (dot(L_i, N) x 0.5 + 0.5)^2
colour = texture x vertex colour x MaterialColour x lit, then fog and grey scale
```

No normal map, rim light or general specular; the only specular is the
PS2's `SgSpec_*` spot. The colours and directions are the PS2 code's
(`Sg_LightSetColor` `0x00836310`, `Sg_LightSetDir` `0x00836430`, which
fill the VU0 matrices and pass the same floats to `g_CharacterLighting`
`0x00ccca84`). The PC changes three things on the way:

1. Levels: `Phyre_CharacterLighting_applyIntensityMultiplier`
   (`0x006a6e90`, thiscall(cl; i, flag) ret 8) sets light `i`'s level
   (PLight `+0x24`; the shader's `m_colorIntensity` is colour x level) to
   (1 - glow factor) x 1.2 x intensity x factor x 0.95 (1.05 and `+0x1a4`
   for the ambient, whose colour x level goes to the lighting record
   `+0xac`, `CharacterAmbientLight`); factor is 1, or 1.16 without
   `+0x230`. The intensities (`+0x1a8 + 0xc i`) come from the HD map's
   lighting object (`PhyreScene_setupCharacterLighting_0066eb20`), the
   glow factors from `balanceLumWithGlow`. Besaid beach (map intensity
   1.7): light 0 2.10, lights 1 and 2 1.31, ambient 1.45.
2. Rotations: `Phyre_CharacterLighting_setRotation` (`0x006aaea0`,
   thiscall(cl; i, pitch, yaw) ret 0xc, degrees) turns light `i` per map
   (4 calls from `PhyreScene_setupCharacterLighting_0066eb20`; Besaid's are
   0).
3. The falloff: half-Lambert squared instead of the PS2's `max(0, N.L)`.

The PS2 formula is still in the `PS2Lighting` path magic uses
(`PS2LightingSetup_apply_006e5a40`: `LvMatrix`, `LcMatrix`, `charlight`):
`lit = max(0, LcMatrix x max(0, LvMatrix x N) + ambient) x charlight / 128`.
With level 1 for every light and the ambient (and the luminances
`+0x1d0..`, `+0x1dc` recomputed) and rotations 0, the HD shader gets the
PS2 colours at unit scale; the falloff stays the shader's (ffx-demaster
`docs/render.md`, `render.lighting`; Besaid in the harness: light 0's
luminance 0.608).

### The shaders in detail (ffx-demaster)

`phyrechrlitshader.fx#*` (199 of 220 files) holds 1854 pixel shader
containers that read `Light0`..`Light2` (216 unique per light count 1-3,
648 in all). Each light is `dp3 rA.c, cb0[Light_i.m_direction].xyz, N;
mad rA.c, rA.c, 0.5, 0.5; mul rA.c, rA.c, rA.c` (2-light variants
vectorise the `mul` over two components; 2 shaders use the half-Lambert
unsquared), then `mad lit, m_colorIntensity, rA.c, lit`. The final colour
is `min(texture x COLOR0 x MaterialColour x lit (+ fog), 1)`. The
`LgtMatVec`/`LgtMatClr` vertex shaders (55 unique) compute one more light
(`setLightMatrix`) as half-Lambert squared into `TEXCOORD4`, which the
pixel shader applies as `max(TEXCOORD4, 1)` (ffx-dxbc reflection and WGSL
translation).

The material colour: in `Ch_DrawMain` (at `0x0082f64c`), with
`g_ChrShadeByLight` (`0x00c496ea`, 1 in the shipped `.data`) the shade is
multiplied by (light value `+0x36c` x 0.85 + 0.15) (`k_PointEightFiveF64`
`0x00b501a8`, `k_PointFifteenF64` `0x00b59870`) and passed to
`graphicCharacterInstanceSetMeshColor` (`MaterialColour`); the PS2's
`Ch_DrawCalcShade` uses the light value as it is. `k_ShadeCoeffF64`
(`0x00b48300`, 0.208125) is read only by
`Phyre_CharacterLighting_setShadeCoeff`. The PS2's own vertex lighting
(VU1) is in `docs/formats/ps2-elf.md`, "VU1 programs".

DXBC checksum: MD5 over bytes 20.. with the bit length as the first word
of the last block and `(bits >> 2) | 1` as its last word, instead of MD5's
64-bit length. It matches every container of the game's 793 shader files
(ffx-demaster `converter/src/dxbc.rs`), so an edited shader can be
re-signed.

## Scenery light levels (vertex colours)

How the PC lights map scenery, read from the map shaders' DXBC (`ffx-dxbc`;
ffx-remix's `bake_census` example over every map with 3D geometry, 328 maps,
2026-11-08) and the models' `Color` streams (`ffx-phyre`):

```
c = TextureSampler x MaterialColour x COLOR0 x shadow   (shadow 1 where MySMSampler, MySMSampler2 read 0)
c = mix with fog (fgEff), x clrScaleFactor
out = 2.1 min(c, 0.4) + 1.08 clamp(c, 0.4, 0.58) - 0.47
```

`phyredefaultlitshader.fx#*` and `phyreuvanimationshader.fx#*` are the
scenery's effects: 809 pixel programs read `COLOR0` (350 with one `mul`,
459 with a further `mad`); 15 declare it unread, 91 have none. The curve at
the end (`min`, `max`/`min` to 0.4..0.58, `mad` 2.1, `add` -0.47) is
linear with a gain of 2.1 below 0.4, flatter up to 0.58, then flat: a
colour byte of 128 lights the texture as painted, as the PS2's 0x80 does
(`MaterialColour`, `clrScaleFactor`, `fgEff`, `MySMSampler*` by name from
each program's reflection).

The colours hold light **and** tint. Over the 20.8 million scenery
vertices: all but 9 of 36,171 parts have a `Color` stream; the mean light
is 0.38 of the curve's full light; 13 % are brighter than byte 128 and
0.8 % at 255; the mean saturation of the colour is 0.41 (the Crusaders
Lodge's bright spots are green); alpha is under 255 on 6.5 % (overlay
parts). Four maps (bsvr0200, bsvr0300, znkd0500, dome0700) also draw a
separate shading layer: one vertex-coloured instance over a PS2 8×8
texture laid over the floor (painted shadows and window light).

**Writing the colours back.** `Model::write` rewrites a segment's `Color`
stream in place (same vertex count; `BYTE4`, or `FLOAT4` as bytes / 255)
and keeps every other byte: with every colour of every map model inverted,
all 324 models it reads (the 4 with 32-bit indices excepted) wrote back with
positions, normals, UVs and indices unchanged and the new colours read
back. No segment is drawn by two instances, so each placed part has a
stream of its own. The game draws a rebuilt model's colours without other
changes (ffx-remix's runtime served one for bsvr0200, 2026-11-08). Spare
streams for a second UV set are rare: 1,325 parts (123 maps) have a second
`ST` stream, 4,834 (83 maps) a second `Color` stream; 1,224 instances on 80
maps morph (the game rewrites their positions every frame).

## Samplers (ffx-demaster)

`PTextureD3D11_register_0058cf80` registers `PSamplerStateD3D11` objects:
its tree `g_SamplerStateCache` (`0x00c25374`) is the sampler cache. A
model's material samplers are made inside `loadCluster` of the model's own
cluster (`chr/.../mdl/D3D11/*.dae.phyre`, `map/.../mdl/...`) with min 5,
mag 1, address 0 (Phyre clamp); the effect shaders' (`Vfx*.fx.phyre`) and
the post-process shaders' samplers come from their shader clusters
(Fahrenheit hook log).

`PRendererD3D11_mapSamplerFilter_0058aa10(min, mag, cmp)`:

| min | mag 0 | mag set |
|---|---|---|
| 0, 2 | `MIN_MAG_MIP_POINT` | `MIN_POINT_MAG_LINEAR_MIP_POINT` |
| 1, 3 | `MIN_LINEAR_MAG_MIP_POINT` | `MIN_MAG_LINEAR_MIP_POINT` |
| 4 | `MIN_MAG_POINT_MIP_LINEAR` | `MIN_POINT_MAG_MIP_LINEAR` |
| 5 | `MIN_LINEAR_MAG_POINT_MIP_LINEAR` | `MIN_MAG_MIP_LINEAR` |

then `ANISOTROPIC` whenever `g_SamplerMaxAnisotropy` (`0x00c25370`) is
above 1.0, and `| 0x80` with `cmp`.

## Z pre-pass

`PhyreScene_DrawZPrePass` (`0x00651070`) renders, in pass `ZPrePass`, the
instances of loaded characters that have a material with switch
`ADDITIONAL_ZPREPASS` (and not `BUILT_IN_ZPREPASS`), except on map 0x180.

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

## Lights and fog at run time (ffx-remix R-43)

Read from the code; nothing was run. `app` is `*g_PhyreApplication`
(`0x00cc9cd8` holds a pointer; getter `0x00623b90` is `mov eax,
[0x00cc9cd8]; ret`). The scene context (`*(scene +0xcc)`) is `app +0x50`,
stored by `PhyreScene_initPostProcess_006656e0`. Offsets below are on the
context; add 0x50 for `app`.

**At map load** (`PhyreScene_fieldMapInitialize` `0x0065b8a0`). The map
cluster's `GlobalCamera` is found by `PhyreScene_findGlobalCamera`
(`0x006519f0`, into scene `+0x4f8`). It is a `PCameraPerspective` (class
`0x00c93608`); `PhyreScene_findGlobalMapObject_006518c0` searches scene
`+0x1128` when byte `+0x93c` is set. Its members are registered by
`Phyre_PCamera_Bind` (`0x004537d0`), with the game's spelling; the object
offset is the descriptor's + 4:

| Member | Object | Goes to |
|---|---|---|
| `m_ambientColor` | `+0xc0` | context `+0x00` (`PhyreScene_updateLightingObjects_00651de0`, only when `Phyre_CharacterLighting_isInitSuccessful`; else 0) |
| `m_fogColor`, `m_fogNear`, `m_fogFar`, `m_fogCurve`, `m_fogLimit` | `+0xd4`, `+0xe4`, `+0xe8`, `+0xec`, `+0xf0` | `+0x10`, `+0x20`, `+0x24`, `+0x28`, `+0x2c` (= limit / 255) (`PhyreScene_setupMapFog_0066ee70`) |
| `m_charcterLightIntensity` | `+0x120` (4 floats: lights 0-2, ambient) | `CharacterLighting_setIntensity_006aa8c0` |
| `m_charcterLightPitch`, `m_characterLightYaw` | `+0x130`, `+0x140` (degrees) | `Phyre_CharacterLighting_setRotation` (`0x006aaea0`) |

Without a `GlobalCamera` the fog defaults are colour (0.28, 0.35, 0.6),
near 65536, far 65537, curve 1, limit 1.
`PhyreScene_setupCharacterLighting_0066eb20` (thiscall(scene; flag) ret
4, called only from `0x0065b8a0`, so once per map load) does the
following:

1. It sets the four intensities and rotations, plus the tweak copies
   (`CharacterLighting_setCoefficient_006aa920`, only with its enable
   byte).
2. On map `0x12ea` with `getScenerioFlag` (`0x00787360`) 0 it forces all
   four to 1.
3. It applies the ambient (`CharacterLighting_applyAmbient_006aa650`)
   and the map name (`CharacterLighting_setMapName_006ab0d0`). On
   `znkd06` that makes one PS2 point light,
   `Sg_PLightDefine(0, -17, -1.5, 30, radius 30, colour 3, 3, 3, light 0)`.
4. Unless on `znkd06`, it runs `balanceLumWithGlow` (`0x006a7550`).

**`g_CharacterLighting`** (`*0x00ccca84`, 0x234 bytes):
- `Phyre_CharacterLighting_initialize_006a8f80` (from
  `PhyreScene_bootInit` `0x00657a50`) creates its four `PLight`s
  (`PCluster_create(4, PLight)`) into context `+0xa4`. They are not map
  data.
- Fields: `+0xf0` that array, `+0xf4 + 0x10 i` the direction (x and y
  negated), `+0x124 + 0x10 i` the colour, `+0x190` the ambient colour,
  `+0x1a0`/`+0x1a4` the ambient intensity and level (default 1.2),
  `+0x1a8 + 0xc i` intensity, `+0x1ac`/`+0x1b0` pitch and yaw, `+0x1cc`
  enabled (4 lights), `+0x1d0` luminances, `+0x1e0` glow factors
  (0.01..0.99), `+0x220` the map name, `+0x230` the flag.
- Setters:
  - `Phyre_CharacterLighting_setLightColor_006aa940`, thiscall(cl; i, r, g, b) ret 0x10:
    PLight `+0..+0xc` = (r, g, b, 1) for `PLightType` `0x00c10828`;
  - `CharacterLighting_setIntensity_006aa8c0`, ret 8;
  - `Phyre_CharacterLighting_setRotation` `0x006aaea0`, ret 0xc;
  - `Phyre_CharacterLighting_setLightDir` `0x006aa9f0`, ret 0x10. It builds a basis with up
    (0, 1, 0), or (0, 0, 1) when the negated direction's y >= 0.9, and
    writes it into the light's node (PLight `+0x10`) as a 3x4.

  `applyIntensityMultiplier` writes PLight `+0x24`; its formula is above.
- Map-specific boost: on `znkd10/14/08/09` the light colours, and on
  `znkd14/08/09` the ambient, are pushed away from grey x2.6. For lights
  0-2 that only changes the luminances (`+0x1d0`), not the PLight colours.

**Who changes the lights later.** Nothing re-sets them per frame. The
writers are:
- `Sg_LightSetColor` `0x00836310` (colours at `0x00c49744 + 0x10 i`,
  `FFXVu0LightColorMatrix` `0x006ed4f0` into `0x023c4400`, then
  `graphicCharacterLightingSetLightColor` `0x0063cb00`);
- `Sg_LightSetDir` `0x00836430` (directions at `0x00c49724`,
  `0x00c49734`, `0x013008e8`; `FFXVu0NormalLightMatrix` `0x006ed620`
  into `0x01300978`).

Their callers:
- natives `0x4011` (`Sg_LightSetDirXYZ` `0x008364f0`), `0x4012`
  (`Sg_LightSetColorRGB` `0x00836400`) and `0x4013` (`Light_Set`
  `0x00836520`, colour / 255 and two angles);
- `Sg_EventjumpInit` `0x0083b460`, `opu_ffxlight` `0x00808cf0`,
  `Atel_RestoreLights_0086e770`, `LightGroup_loadSave_0084e3c0` and
  `SgGu_LightApply_0084ebd0`;
- `Sg_LightSetDirXYZ2_00834320`, a copy of `Sg_LightSetDirXYZ` that
  nothing calls.

**Light directions** (ffx-remix R-52, read in the dump and the ports):

1. `Light_Set` (`0x00836520`, native `0x4013`: light, r, g, b, a5, a6, all
   ints) passes the colour `(r, g, b) / 255` to `Sg_LightSetColor`. It turns
   each angle to radians (`deg * 3.141592 / 180`, wrapped by
   `Angle_Wrap_00836720`) and passes the direction
   `p = (sin a5 * cos a6, sin a6, cos a5 * cos a6)` to `Sg_LightSetDir`:
   a5 is the azimuth about y (from +z towards +x) and a6 the elevation.
   (The argument names "pitch" and "yaw" used for `0x4013` elsewhere were
   swapped; `docs/formats/script.md` now says azimuth and elevation.)
   That reaches `Phyre_CharacterLighting_setLightDir` through
   `graphicCharacterLightingSetLightDir` (`0x0063cb50`).
2. `setLightDir` stores `d = (-p.x, -p.y, p.z)` and gives the PLight's node
   `PMatrix4_lookAt` from the origin towards `d`. The rows are `s, t, -f`,
   so the node's -z is `d` normalised.
3. `Phyre_CharacterLighting_applyRotation` (`0x006a7280`) then turns that
   node by the map's per-light yaw and pitch (`+0x1b0`/`+0x1ac + 0xc i`,
   from the `GlobalCamera`'s `m_characterLightYaw`/`m_charcterLightPitch`;
   radians as `deg / 180 * 3.14`), when either is larger than 0.001 in
   size (`fabs`). In the port
   (`ffx_pother_b::p4_light2`) it is the look-at times `Matrix_RotationY`
   (yaw), then times `Matrix_RotationX` (pitch), in the row form of
   `PMatrix4_Multiply_004ca360` (`out = b a`). With column vectors:
   `d_world = Rx(pitch) * Ry(yaw) * normalize(d)`, the direction the light
   travels.
4. The node's -z is the light's direction:
   `Phyre_CharacterLighting_lightDirection_006a84d0` reads it that way
   (`(0, 0, -1)` through the node's matrix; its only caller is the tweak
   utility's `0x006a7850`). The shaders' `Light0`-`Light2.m_direction` come
   from the same three `PLight`s (`docs/formats/scene-chars.md`).

Over the map clusters' cameras, yaw is non-zero only on `mihn01`'s
(`mihn0100`, `isho0000`: light 0 at 25 degrees) and `stbv01`'s
(`stbv0100`, `stmm0100`: lights 0 and 1 at -10). Pitch is 0 on every map.
So on most maps Light0's direction is the last one a writer set ("Who
changes the lights later" above), mostly the event scripts' `0x4013`
calls (no event script calls `0x4011` or `0x4012`). One example: `bsil0000`'s scripts set light 0
with 12 argument sets, most often `(0, 100, 110, 120, 243, 50)` and
`(0, 98, 108, 118, 37, 42)` (11 calls each).
Which call is active at a given moment is script state. Not checked
against a capture: whether `mihn0100`'s painted sun matches its light 0.

**PS2 point lights** (`*0x013008cc`, count `0x013008d0`; layout in
main.md):
- They are filled at run time by:
  - natives `0x4023`/`0x4025` (`0x00a78340`, `0x00a78370`);
  - the effect programs `pppFpPointLightProc` `0x00729f90` and
    `pppFpPointLight` `0x0075d7f0` (and `0x0075d930`, `0x0075db30`,
    `0x0075dd70`, `0x0075df90`);
  - the `znkd06` code above.
- `Sg_PLightDefine` checks `i <= count`, one past the table.
- They are read only by `Ch_PLightCalc` (`0x008305a0`, from `Ch_DrawMain`
  per character). That goes to `graphicCharacterLightingSetLightMatrix`
  (`0x0063cb80`), which writes the character material's `LgtMatVec` and
  `LgtMatClr`. They are never Phyre lights and never reach map shaders.

**Fog**:
- Writers:
  - at map load, `0x0066ee70` (above);
  - the debug tweak `0x00698ab0`;
  - the natives:
    - `Native_FogSetParams` `0x0091b810` (`0x8037`: near, far, limit)
      writes the PS2 fog through `yiSetFogParam` (`0x0090a350`), which
      divides near and far by `g_YiScaleFactor` (32 after `yiInit`; 10 on
      one `Sg_MainLoop` path). It reaches the PC renderer only by map
      number (`g_CurrentMapNo` `0x012fbc40`):
      - on map `0xe0` (in `yiSetFogParam`) as near and far / scale x
        0.001, limit (1 - limit/255) x 0.4, curve 1;
      - on map `0x140` (`0x0091c8a0`) through `graphicSetFogNear`/`Far`/
        `Limit`(/255) and curve 1.
    - `Native_FogSetParam0`/`1`/`2` (`0x8038`/`0x803a`/`0x803c`,
      `0x0091b8b0`/`0x0091b960`/`0x0091ba10`) have no map test. Unless the
      level name hash is `0xca2`, `0x1e38` or `0x699`, each reads the three
      values back (`yiGetFogParam`), replaces its own, calls
      `yiSetFogParam`, and then calls `graphicSetFogNear`, `Far` or
      `Limit`(/255) directly, on every map; none sets the curve. On map
      `0xe0`:
      - `yiGetFogParam` (`0x00908bb0`) returns the renderer's values, so
        the two values a call does not set are rescaled and shrink with
        every call;
      - the value it does set ends up unscaled (the direct call comes
        last).
    - The colour natives `Native_FogSetColor` (`0x803e`, `0x0091bad0`) and
      `Native_FogSetRed`/`Green`/`Blue` (`0x803f`-`0x8041`, `0x0091bbb0`/
      `0x0091bc80`/`0x0091bd50`):
      - The one-channel natives first read the colour back with
        `yiGetFogColor` (`0x00908980`). It takes r, g, b, on every map,
        from `graphicGetFogColor` x 255, or `graphicGetGlobalAmbientColor`
        with render status bit 2 (`0x018ded08 & 4`), truncated. The two
        channels a call does not set are therefore the live context's (in
        whole 1/255 steps) or the global ambient.
      - All four then call `yiSetFogColor` (`0x0090a2c0`), which stores the
        PS2 colour bytes and, on map `0xe0`, calls
        `graphicSetFogColour(c / 255)` with exact fractions.
      - On map `0x140` all four pass `ftol(c / 255)` per channel: each
        channel is 0, or 1 at 255.
- The `graphicSetFog*` setters go through `FogEffectManager` (`*0x00ce5170`,
  getter `0x006e32c0`, context through manager `+0xa0`). Each first asks
  `FogEffectManager_isMoving_006e3480`: the squared distance
  (`0x006e3320`) between the manager's snapshot and the live context over
  colour, near, far, curve and limit. Above 0.1 the setter does nothing.
  Since near and far are world units, a difference of about 0.32 in
  either blocks every setter. The snapshot (`+0` background, `+0x10`
  colour, `+0x20` near, `+0x24` far, `+0x28` limit, `+0x2c` curve: limit
  and curve swapped against the context) is refreshed by the setters and
  by `FogEffectManager_copyParameters_006e3b80`.
- `PhyreScene_animate` calls `FogEffectManager_updateChrFogFade_006e3f40`
  every frame. It writes the materials' `fgEff`, not the context's fog.
  `graphicSetGlobalAmbientColor` (`0x00643270`) has no direct callers.

**What reaches the shaders, and when.**
- `PSceneWideParameters_getBuffer` (`0x004fdec0`, per draw from
  `allocateAndFillParameterBuffers` `0x005b5a50`) copies:
  - 0x10 `GlobalAmbientColor` from `+0x00`;
  - 0x11 from `+0xd0` (`GlobalTime`);
  - 0x16-0x1a fog from `+0x10..+0x2c`;
  - 0x1b `BGColor` from `+0x30`;
  - 0x1c from `+0x40`.
- The buffer is refilled only when its key changes. The key
  (`PSceneWideParametersKey_initialize` `0x004fe940`) holds the camera,
  near and far, the viewport, the ambient (`+0..+8`) and `GlobalTime`,
  but no fog. So a fog-only write shows at the next refill.
- `graphicRender` (`0x00642bb0`) runs once per frame from
  `FFXApplication_render` (`0x0042f930`), after the cloth upload and
  before `VsyncEmu_Wake`. It is skipped while `app +0x3ad` (loading) is
  set or when `TrophyGetError()` == 1. It starts `mov ecx, [0x00ccb9d8]`
  (6 bytes), calls `PhyreScene_render`, then
  `JobScheduleWait_pppPart_fillBuffer` and
  `PhyreScene_doDeleteMeshInstances`, and returns `PhyreScene_render`'s
  result. Its entry is the place to set lights or fog for the frame's
  draws; set the fog snapshot too, or the next script fog change is
  refused.

Open: `0x01300978` is `FFXVu0NormalLightMatrix`'s output but carries the
label `g_Vu0ClipScreen` here. `Phyre_FogEffectManager_setParam20`/`24`/`28`
and `graphicSetFogParam_00643180` are the near, far and curve setters.

## Scene state

What `ffx-render` needs from the scene stream lives in a few objects the
`graphic*` calls set (`crates/ffx-graphic/src/state.rs`).

`g_PhyreApplication` (`0x00cc9cd8`, `Phyre_PFramework_PApplication_GetApplication`):

| Offset | Value | Set / read by |
|---|---|---|
| `0x50` | global ambient colour (3 floats) | `graphicSetGlobalAmbientColor` (`0x00643270`), `graphicGetGlobalAmbientColor` (`0x00641230`) |
| `0x60` | fog colour (3 floats) | `graphicGetFogColor` (`0x00641160`) |
| `0x70`, `0x74`, `0x78`, `0x7c` | fog near, far, curve, limit | `graphicGetFogNear`/`Far`/`Limit` (`0x006411d0`, `0x006411b0`, `0x006411c0`); the curve `graphicSetFogParam_00643180` (`FogCurve`, scene-wide semantic 0x19) |
| `0x80` | background colour (3 floats) | `graphicSetBackgroundColour` (`0x00642e90`) |
| `0xf4` | the character lights: a `PArray<PLight*>` of 4 (count `& 0x7fffffff`, data `+4`) | `Phyre_CharacterLighting_initialize_006a8f80` |
| `0xfc..0x104` | `CharacterAmbientLight` (ambient colour x level) | `Phyre_CharacterLighting_applyIntensityMultiplier` (`0x006a6e90`) |
| `0x120` | `GlobalTime` | `FFXApplication_updateGlobalTime_0042fa00` |
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
  minimap cameras; with the 2D level loaded (byte `+0x93c`: the 2D
  flag, not a 3D one; `docs/formats/scene-maps.md`, "How the game draws
  the 2D layers") level `0xec1` forces `0x3ef126e9` (0.471 rad, 27
  degrees) and the minimap camera gets `0x3f2c49ba` on `0x107`,
  `0x3f5126e9` on `0xf48`.
- `PhyreScene_setCameraAspectRatio(a)` (`0x0066cbe0`): in a 3D scene
  `a * 3 * 0.25 * 16/9`; a reflection camera of kind (`+0x10d90`) 0 or 1
  gets that times 0.5 or 2, and then the main camera `window_w * a *
  0.75 / window_h` times the same.
  The `a` it gets is the PS2's: `Cam_Apply` (`0x007bc980`) passes
  `(512 / (2 fx)) / (448 / (2 fy))` from screen 0's `+0x10`/`+0x14`
  (`0x01137010`/`0x01137014`; 0.8 and 0.9333 on Besaid), which is 4/3.
  So a 3D scene stores 16/9 with the PS2's vertical field of view: the
  PS2 picture is the middle 4:3 of the HD one (240 of 1920 columns cut
  each side). With `g_PhyreScene3D` clear (the 2D-background rooms;
  `PhyreScene_fieldMapLoad` sets it only for 3D maps outside
  `g_PhyreScene2DMaps_b421c8`) the camera keeps `a` (4/3, from
  `rcbgSceneSetMatrix`'s `+0xb4 = +0xb0 * 1.1666666`) on the 16:9 target,
  the PS2 view stretched by 4/3 (ffx-demaster `docs/screen.md`, measured
  in the harness on Besaid and hiku00).
- The output shape: the last copy into the back buffer goes through
  `PostProcessManager_setNon16By9Viewport` (`0x00698750`), which fits the
  frame to `g_PostProcessOutputAspect` (`0x00c34154`, f32 16/9 in `.data`,
  that function its only reader) with bars and gives the Iggy menus the
  same rectangle (`IggyMenuManager_setViewportRect_00a26bd0`). Writing
  4/3 there pillarboxes the frame to 4:3 (linear filter, a squeeze of the
  16:9 targets); with the camera hook above passing `a * 3/4` the 3D comes
  out in PS2 proportions and the PS2-placed 2D lands at its PS2 place
  (ffx-demaster `docs/screen.md`, `render.aspect`; seen at 3440 x 1440
  under Fahrenheit). The sphere grid's camera is set separately, to
  `k_SixteenByNineF32` (`0x00b41cf0`, read by `PhyreScene_RenderABmap` at
  `0x00652a7a`, `graphicGetScreenRatio` and `PhyreScene_initScene`).
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

Object rotation: `PhyreScene_setObjectRotation(id, angles, order)`
(`0x0066db00`) finds the same mesh list and rebuilds each mesh's 3x4 world
matrix: made a `PMatrix4`, its upper 3x3 set to 10 times the unit matrix
(`0x00531a40`; the old rotation and scale are dropped, the translation
kept), then turned about three axes in the order `order` (an `i16`) picks:
0 z y x, 1 z x y, 2 y x z, 3 y z x, 4 x z y, 5 x y z, anything else none.
The x and y turns are by `-angles.x` and `-angles.y` (`Matrix_RotationX`,
applied row by row with `PMatrix4_RowTimes`; `Matrix_RotationY`), the z
turn by `+angles.z` (`0x004cd1f0`); the y and z products go through
`0x005e4400`. Returns 0. Port: `ffx_graphic::state::bridge::scene_set_object_rotation`.

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
The camera the layers are drawn with (`+0x7e0`), their depth and the
16:9 window: `docs/formats/scene-maps.md`, "How the game draws the 2D
layers".

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
- `0x006411e0`/`0x00640e20`: create the free camera (`g_FreeCamera`,
  `FreeCamera_FreeCamera`, 0x340 bytes) on demand and read its mode or
  pass it input: the developer fly camera, which the pause also uses to
  freeze the game (`docs/formats/field.md`, "Free camera").
- `0x006dd3b0`: writes `WIN32_VramDetail_<name>_<n>.csv` with a header
  and "Vram dump is not supported on platforms other than PS3/PSV.": the
  PS3 VRAM report, stubbed on PC.
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

Round 7:

- Scene: `PhyreScene_SetMeshesVisible(on)` (`0x0066e7b0`) sets the draw
  pointer of every mesh in `+0x11d0` (`+0x11c8`); `PhyreScene_SetColourPair`
  (`0x0066ea70`) stores `(r, g, b, 1)` and `(r2, g2, b2, 0)` at `+0xfc8c`,
  or `+0xfcac` for the second pair; `SegmentList_AllReady` (`0x00652250`)
  waits for every segment's scene state (`*(scene +0x60)[i]`) to be 1.
- `ShadowManager_SetQuality(q)` (`0x006ceaa0`): `g_ShadowSize` 4096, 2048
  or 1024 for quality 1-3; `ShadowEntry_Init` (`0x006d3560`).
- `BlurEffectEx_SetPositiveParams` (`0x006c4620`): only floats above 0
  replace `+0x5c..+0x68`; `ParamList_SetAllFloat` (`0x006b47e0`).
- Texture animation: `TexAnim_StartNew`/`Old(i, which, pos)` (`0x0077fc90`,
  `0x00780750`) restart one work animation (at work `+0` or `+0x400`);
  `TextureAnimationManager_FindMaterialIndex` (`0x006b20a0`, -1 when
  absent) and `_FindAnimation` (`0x006b1ef0`) look names up by `PString`
  text.
- `FloatHeap_PushHeap` (`0x006c7730`) and `FloatSort_Median3`
  (`0x006c74d0`), the rest of the float sort with a `less` callback;
  `PhyreFIOS_LoadPrxIdle` (`0x006f9b80`).
- Hooked by main's automatic ports instead (`auto.rs`, names theirs):
  `SetFields_006c47a0` (clears `+0x58`, `+0..+0x20`, `+0x48..+0x54`),
  `SetFields_006f7700` (the nine cheat-overlay `.swf` paths and a byte
  table), `LoadCompute_006f7bd0`/`_006f7c00` (`(+0x30 + +0x34) * 4`),
  `ReturnConst180_006c26d0`, `ReturnConst51_006c2700`,
  `ReturnGlobalSx8_006bc960`, `Compare_006f9b70` (PRX state 3); the
  render bridges for them stay and their parity test with them.

Round 8:

- Cameras: `graphicCameraSetWorldMatrixSub(view, on)` (`0x00639b30`)
  sends the view, converted as by `graphicCameraSetWorldMatrix` but
  without its zero-row and skipped-frame checks, to the alternative camera
  matrix (`PhyreScene_setCameraWorldMatrix(m, 1)`), or with `on` 0 just
  clears `+0xfaa1`; `graphicMinimapCameraSetWorldMatrix(view)`
  (`0x00642410`) does the same conversion for the minimap camera.
  `graphicCameraSetFOVy`/`AspectRatio(&v)` (`0x006395e0`, `0x006395c0`)
  pass a float by pointer to the scene setters.
- Meshes: `graphicCharacterInstanceSetWorldMatrix(list, m)`
  (`0x0063c3e0`) sets every mesh's world matrix;
  `...ByMdlID(list, m, id)` (`0x0063c430`) sets it on each mesh once per
  segment (`0x6c` bytes, model id `+0x18`) with that id, and with no match
  on mesh number `id` if there is one.
- Wrappers: `graphicObjectSetTranslation`, `graphic2DHideLayer`/`ShowLayer`,
  `graphicVFXUpdateType4` (`0x00644db0`, type 4).
- The scene's list cursors (`PhyreSceneCursor_Seek1..4`,
  `0x0065eaf0`..`0x0065ecb0`, check first; `_Next1..4`,
  `0x0065ed20`..`0x0065ee40`, step first): the node `+0x18` steps to
  `*node` (0 at the end node `+0x14`) until `PhyreCursor_IsValid`, then
  `PhyreSceneCursor_SetNode` (`0x00441dc0`), or the first five words are
  cleared without a node.

Classed logic but not ported, with the class I think fits (for the
classifier): `0x0066e4f0` (a render-target surface-format check:
glue), `0x006590d0` (`std::string::compare`), `0x00636d60`
(`std::map` range erase) and `0x0064fa50` (`_CxxThrowException`): `std`
and CRT copies, platform; `0x0066c8a0` (an x-rotation matrix from
`_CIsin`/`_CIcos`, whose results need the CRT's own routines), `0x006426e0`,
`0x00698960`, `0x00638720` (shader-parameter writes into material
buffers: glue); wrappers whose callee is glue: `0x00641cc0`
(`PhyreScene_GetSplitScreenStat`), `0x006427c0`/`0x00645d90`
(`PhyFMVPlayerManager`), `0x00638fd0` (`AbmapManager_GetVertexInfo`),
`graphicAddScreenFadeTask` (`ScreenFadeDelegations_registerDelegation`),
`graphicMagicSetPath`/`SetToBeDeletePath` (`Phyre_PString_Assign`
setters); `0x006c56f0` (adds a VFX draw event from a stack struct with
an uninitialised word) left for later.

Round 9:

- `Matrix_RotationX`/`Y(out, angle)` (`0x0066c8a0`, `0x0066c920`): the
  4x4 rotations, with `sin` and `cos` from the CRT's `_CIsin`/`_CIcos`
  through the game's import thunks (`0x009498c8`, `0x009498c2`):
  `Host::call_intrinsic` puts the angle in `ST0` and stores the result as
  the original's `fstp m32` (result pointer in `edi`, no register operand
  read after the call).
- The list cursor's set-up `PhyreSceneCursor_SetNode(node)` (`0x00441dc0`):
  stride `*(node +0x2c) +0x1c`, position `node +0x34`, count `node +0x3c`,
  `+0x10` from `*(node +0x2c) +0x8c`, link `node +8`; while counted and the
  link equals the position, the position steps by the stride and the link
  follows itself. Three more copies of the cursor check
  (`0x00441e10`, `0x00441e40`, `0x006a29c0`).
- `PostProcessManager_SetFirstEntry(list)` (`0x00698400`),
  `TextureAnimationManager_SetFrame(i, frame, absolute)` (`0x006b42c0`),
  `ResetAndConstruct_006c40c0`.

Not ported in round 9: `0x006411e0`/`0x00640e20` (the free camera,
`g_FreeCamera`: debug), `0x006dd3b0` (a VRAM detail dump to
`WIN32_VramDetail_*.csv`: debug), `0x006370d0` (`std::map` range erase:
platform), `0x00692680` (exception-frame set-up: platform),
`graphicCharacterInstanceUpdateSkeletonounds` (`0x0063c500`, only
forwards to `Phyre_PRendering_PMeshInstance_updateBoundsFromSkeleton`:
glue, by the forwarder rule).

Round 10:

- `PhyreScene_ApplySettings` (`0x0066f010`): the graphics settings
  (`g_Settings` `0x022fb504`) to the scene: `+0x24` as a float to the
  post-process's `+0x12328`, the shadow quality (`+0x30`,
  `ShadowManager_SetQuality`, then the buffers resized), `+0x44 != 0` to
  its byte `+0x123d3`, `+0x48` to `+0x1264`, `+0x38` to its rebind, and the
  anti-aliasing mode `+0x40`: 1 sets the scene's `+0x10dd8` to 1; 2-5 set it
  to 0 and the post-process's `+0x1231c` to 1-4; anything else sets 2.
- `PhyreScene_PushPair3890(a, b)` (`0x00656b90`), `GetASpareGradMeshInstance`
  (`0x006cc980`, 16 slots per set, the set in `0x00cdec24`; prints and
  returns 0 when full), the float sort's `FloatHeap_MakeHeap`
  (`0x006c7410`) and `FloatSort_Insertion` (`0x006c7320`), two more cursor
  steps (`PhyreSceneCursor_Next5`/`6`, `0x00441c60`, `0x00441cc0`).

Not ported in round 10: `0x00667770` (`std::vector<std::string>`
`push_back`: platform), `0x006d5a00` (frees three `PCluster`s and resets
`BrokenScreenEffect`: glue).

Round 11:

- `ShadowManager_ResetSolution(keep)` (`0x006ceb10`): resets the shadow
  state (`+0x403c` from the float at `0x00c34400`, the compare callback
  `+0x4048` = `Compute_006cc8b0`, flags), without `keep` also `+0x4034` = -10;
  then `PhyreScene_ForceSetShadowSolution(keep ? 0 : 2)`.
- `PhyreScene_AddDrawEntry(e, kind)` (`0x00656c10`, two lists of up to 0xc00),
  `PhyreScene_SetNumberText(n)` (`0x0066e5e0`, `sprintf "%d"` into
  `+0xfc68` when `+0xfc00` is set), `BrokenScreenEffect_BufferSize(n)`
  (`0x006393c0`).
- Texture animation: `TexAnim_PosSet(i, which, v)` (`0x0077e9a0`, by the
  entry's format: 0 old `tex_anim_pos_set_old`, 1 new `TexAnim_ResetPos`),
  `TextureAnimationManager_SetPaused` (`0x006b3600`),
  `graphicTextureAnimationSetFrame` (`0x00643a20`).
- `graphicGetCurrentFrame` (`0x008e1c50`): the menu's current player.

Not ported in round 11, with the class that fits: the fog setters
`graphicSetFogColour`/`Far`/`Limit`/`Near` (`0x00643150`..`0x006431e0`) and
the `BrokenScreenEffect` setters `0x00639360`/`0x00639370`/`0x00639390`
(forwarders that only call glue: glue); `0x006377a0`, `0x00637730`,
`0x00441bf0` (cursor steps on the `PhyreCursor_IsValid_copy_00637f80`
check, now classed glue: the cursor family is PhyreEngine's own list
iteration, so the cursor steps and checks ported in rounds 8-10 may count
as glue too); `0x00651800` (`MultiByteToWideChar` conversion: platform);
`0x006582e0` (`MaterialColour` shader parameter: glue); `0x006436e0`
(forwards to `MorphManager`, unclassed); `0x00642c80` (fog-manager state).

Not ported: `0x0064e9e0`, a `PString` buffer release, decrements its
reference count with `lock xadd`; game memory access through `Mem` is
not atomic, and loader threads share strings, so it stays original.

Round 12:

- `TextureAnimationInfo_update(this, dt)` (`0x006b4450`, slot 1 of
  `TextureAnimationInfo_vftable` and `TextureAnimationInfoChar_vftable`):
  while playing (byte `+0x1c`) the timer `+0x14` grows by `dt`, stored as a
  float; once it reaches the period `+0x10` (`fcompp; test ah, 0x41; jp`:
  an ordered `period <= timer`) it restarts at 0 and the frame `+0x18`
  becomes `(frame + 1 + count) % count` (count `+8`, bit 31 a flag).
- `TextureAnimationManager_GetByIndexList(this, list, i, absolute)`
  (`0x006b2140`): index `i` of the list (`+8` count, `+0xc` words, -1 for
  none), plus the manager's `+0x10` unless `absolute`, picks a word of
  `+0x60` (`+0x5c` count); 0 out of range.
- `tex_anim_no_set_old(i, a, v)` (`0x007807c0`): only for `a == 4` and a
  loaded `g_ChrTexAnim[i]`: `+8 = v`, and each of the eight old-format
  animations (`0x34` bytes from work `+0x400`) gets `+0xc`, `+0x1c`,
  `+0x20`, `+0x34` and `+0x38` zeroed (the last two lie in the next
  animation: the compiler's unrolled loop). Its `eax` (the work buffer, or
  `i`) is what `tex_anim_no_set` returns; for `a != 4` it is the caller's,
  which `tex_anim_no_set` (`0x0077e940`) loads with `v` before the call.
- `texanim_malloc_memory_set(i, delta)` (`0x0077ea10`): `dbgPrintf`
  "Sa:Movie Malloc Move %x", then entry `i`'s work buffer moves by `delta`.
- The PRX loader's state `g_PrxState` (`0x00ce83c4`): 1 loading, 2 loading
  asynchronously (`MagicDLLAsyncLoad`), 3 done, 0 unloaded;
  `PhyreFIOS_LoadPrxInProgress` (`0x006f9ba0`) is 1 or 2. `0x00ce83c0` is
  the `HMODULE` (`g_PrxModule`), not a count.
- `SpdCtrl_Init` (`0x006f72f0`), `CheckAutobtlKey`/`CheckInvincibleKey`
  (`0x006f7580`/`0x006f76c0`: buttons `0x20000`/`0x200000`),
  `setLuluODKeyboardInput(left, right)` (`0x00645eb0`: exactly one of
  `0x8000`/`0x2000`), `SplitScreenStart(kind)` (`0x00638d40`),
  `graphicMagic0146SetKey(v)` (`0x00641e40`: the scene's byte `+3` and
  `g_GraphicLevelFilter`), `graphicResetLoadingScreen()` (`0x00642c80`: the
  loading screen `g_LoadingScreen` in state 3 goes to 1, in state 0 is
  reset), `ScreenFadeDelegations_processRenderDelegation(d)` (`0x006ae5b0`:
  unless the post-process's fade colour is locked (`+0x123a0`), `d +0x1c`
  to `+0x1239c` and the four floats `d +0xc..+0x18` to
  `+0x1238c..+0x12398`; then `renderScreenFade(d[0], d[1], d[2],
  g_ScreenFadeByte)`).

Round 13:

- `ShadowManager_AverageCasterValue(this)` (`0x006ccad0`, fastcall, result
  in `ST0`; `PhyreScene_fieldMapInitialize` and `0x00653600` store it): the
  shadow manager's 64 caster slots (index `8 * row + col`) are a mesh
  pointer (`+0xde8`), two flags (`+0xfe8`, `+0x10e8`; a slot counts when
  both are 1), a kind byte (`+0x2ba8`, 12-byte stride: `'c'` or `'n'`) and a
  callback word (`+0x2ea8`). A caster's value is the float
  `mesh +4 ->[0] +4 -> +0x24`, and a mesh with count 0 (`*mesh & 0x7fffffff`)
  has none. The selected slot (`+0x4010` row, `+0x4014` col, both
  non-negative or the result is 0) gives its own value when it counts;
  else the mean of the `'c'` slots, and failing those of the `'n'` slots,
  where with `+0x4030` set only those the callback `+0x4048` (cdecl, `al`)
  accepts for their word count. The running sum is stored as a float after
  each add (`fadd m32; fstp m32`) and the mean as one; 0 when no slot
  counts.
- `graphicAddMeshMatrix(list, matrix, id)` (`0x00639170`): over a list of
  mesh instances (count, array) and each segment of an instance's mesh
  (`0x6c` bytes, `+0x18` the id), `PhyreScene_PushMeshMatrix(inst,
  matrix)` (`0x00656780`: the pair appended to the scene's `+0x10d84`
  array of `0x44`-byte entries): for `id == -1` once per segment, else once
  at the first matching segment, which ends the walk.
- `graphicSetTextureAnimationFrameByName(frame, name)` (`0x00643a40`):
  `TextureAnimationManager_FindMaterialIndex(name)`, then
  `SetFrame(i, frame, 1)`; when that fails it prints "SceneName.%s,
  TextureAnimName.%s." to `stdout` (`__iob_func() + 0x20`) with the level
  name and `name` (the latter is the argument pushed for
  `PhyreScene_getCurrentLevelName`, which takes none and leaves it on the
  stack for `fprintf`).
- `graphicSetColourPair_00643b30(r, g, b, r2, g2, b2, second)`:
  `PhyreScene_SetColourPair` on the scene with `second != 0`; `eax` is
  that flag, which the callee leaves.

## Shader parameters of mesh segments (round 14)

`crates/ffx-graphic/src/material.rs`. A mesh instance list is a count
(bit 31 a flag) and an array of instances. An instance's `+0` is its mesh
(count; `+4` segments of `0x6c` bytes: `+0` the material index, `+0x14`
the hidden byte, `+0x18` the segment id) and `+0x10` its materials (count;
one material inline at `+4`, several through the pointer at `+4`). A
material's `+0` is its shader and `+4` its parameter buffer (a size word,
then the data). `ShaderParams_FindByName(shader, name)` (`0x0056cbf0`,
PhyreEngine's) finds a parameter: `+8` (`u16`) its offset in the buffer,
`+0xa` (`u16 & 0x1fff`) its size. A value is written only when the offset
is at least 4 and offset + size is within the size word
(`ShaderParamBuffer_SetValue`, `0x00638780`, inlined in the callers).

- `graphicCharacterInstanceSetMeshColor(list, _, r, g, b, a)`
  (`0x0063bb30`) and `graphicCharacterInstanceSetMeshSegmentColor(list, id,
  r, g, b, a, hide)` (`0x0063bc80`): `MaterialColour` = `(r, g, b, a)` on
  each (matching) segment's material, then
  `PhyreScene_ForceTransparent(material, a < 1)` (ordered compare), both
  only when the shader has the parameter. With `hide`, a segment whose
  `a` is 0 (either sign) is hidden (`+0x14` = 1) and keeps its colour; a
  positive `a` first shows it (`+0x14` = 0).
- `graphicCharacterInstanceSetMeshAlphaThreshold(list)` (`0x0063b9e0`):
  each segment's `AlphaThreshold` that is not below `0.001` (the `f32` at
  `0x00b368a4`; a NaN counts as not below) becomes 0. For a parameter
  outside its buffer the original compares the float at address 0.
- `PhyreScene_SetLayerMaterialAlpha(scene, layer, alpha)` (`0x006582e0`):
  with the scene's byte `+0x93c` and word `+0x1128` set, the material of
  texture-animation layer `layer` gets `MaterialColour` `(1, 1, 1,
  f32(alpha) * (1/128))` (the `f64` at `0x00b170b8`).

The originals copy `size` bytes from a 16-byte (`AlphaThreshold`: 4-byte)
local; a larger parameter would copy their stack frame. The ports write
zeros past the value.

Round 15:

- `graphicCharacterInstanceSetMeshSegmentAlpha(list, _, id, a)`
  (`0x0063bc50`): `graphicCharacterInstanceSetMeshSegmentColor(list, id, 1,
  1, 1, a, 1)`; `graphicChangeLayerAlpha(layer, alpha)` (`0x00639d20`):
  `PhyreScene_SetLayerMaterialAlpha` on `g_PhyreScene`.
- `graphicCharacterInstanceGetBoundingBox(chr, max, min)` (`0x0063a2b0`):
  `min` starts at `16777216` (the `f32` at `0x00b41da0`) and `max` at
  `-16777216` (`0x00b41db4`) on each axis; then for each mesh instance of
  `Ch_GetChrRenderInstance(chr)` (its `+0x1c` bounds: the corner at
  `+0..+8`, the extent at `+0x10..+0x18`) and axis, `corner + extent`
  (stored as a float) replaces `max` when greater and the corner replaces
  `min` when smaller (ordered compares: NaNs never replace, and equal
  values, `0` and `-0` included, keep the old bits).
- `SplitScreenSetCamera(m)` (`0x00638aa0`): `sceVu0InversMatrix4x4` of a
  copy of the 4x4 `m`; elements 0, 1, 6, 10, 12, 13 change sign (the code
  also negates 4, 5, 8 and 9 twice); `PMatrix4_toWorldMatrix3x4` of the
  identity and then of that matrix into one 3x4, copied to `g_PhyreScene
  +0x10d94..+0x10dc0`. Its one caller (`0x007bc980`) calls it next to
  `SplitScreenStart`.

Round 16:

- `graphicAbmapResetSegments()` (`0x006390c0`): with `g_GraphicAbmap`
  (`0x00ccc838`) set it jumps to the manager's code at `0x006853d0`
  (nothing else reaches it). By the state `+4` (signed): 1 resets the
  render data at `+0xb0`, `+0xb4`, `+0xbc`, `+0xb8`, `+0xc4`, `+0xc0` (in
  that order), 0 or less those at `+0x98`, `+0x9c`, `+0xa4`, `+0xac`,
  `+0xa0`, `+0xa8`, 2 or more none. Resetting a render data zeroes `+4`
  and `+8` of each of its segments (`+0x94`, `0x6c` bytes, `+0x90` of them,
  a signed count read again each step).
- `graphicCharacterLightingSetLightMatrix(list, dirs, colours)`
  (`0x0063cb80`): two 4x4 matrices with one light per column: `dirs`
  (x, y, z, w per 16-byte row) normalised when `x*x` (stored), `+ y*y`
  (stored), `+ z*z` (stored) is above the `f64` `0.0001` at `0x00b3eb50`
  (`_CIsqrt` stored, `1 / it` stored, each component times it stored), `w`
  as it is; `colours` copied. Then
  `Phyre_CharacterLighting_setLightMatrix(inst, dirs, colours)` on the
  lighting singleton for each instance of `list`.

Round 17:

- `graphicCharacterInstanceSetBTUFadeCenter(chr, axis, center)`
  (`0x0063b5d0`): over the character's segments (`Ch_GetChrRenderInstance`),
  `GradualFade_BasicAlpha` is looked up first whatever `axis`; for `axis` 0
  `GradualFade_YCenter`, for 1 `GradualFade_ZCenter` (others: nothing
  more). With both found, the centre gets `center` and then the basic
  alpha 0 (each written only where it fits its buffer).
- `graphicCharacterLightingSetShadeCoeff_debug(list)` (`0x0063cec0`): on
  levels `0x226d`, `0x1739` and `0x869` (`PhyreScene_getCurrentLevelNameHash`)
  each instance gets `Phyre_CharacterLighting_setShadeCoeff(inst, 1.0)`,
  elsewhere `Phyre_CharacterLighting_resetShadeCoeff(inst)` (`0x006aafa0`:
  the default from `0x006a8c40`); then the tweak utility's
  `setShadeCoeff_dbgHook(inst, 0.0, 0)`.

Round 18:

- `SpecularMapParamsManager_SetParams(this, id, a, f0, f1, f2, b, g0, g1,
  g2, g3)` (`0x006c7090`, `thiscall`, `ret 0x28`; `0x006439b0` calls it on
  the manager): nothing while `g_GraphicSkipFrame` is set. An `id` not in
  the four slots `+0..+0xc` takes the first empty one (its flag at
  `+0x5a0 + 4i` set to 1) and gets `SpecularMapParamsManager_AddSpecularEvent(id)`
  (also when no slot is free). Then
  `SpecularMapParamsManager_UpdateParams_006c6c30` (`0x006c6c30`, ten
  words, `ret 0x28`, `ecx` unused) with the same words, the seven floats
  copied through the x87. `ffx_graphic::host::Host::call` passes up to
  twelve words to a `thiscall` for it.

Round 19:

- `BoundingBox_IsVisible_00663a30(box, m)` and
  `BoundingBox_IsInside_006d27e0(box, m)` (`thiscall`, `ret 4`; the same
  code but for one instruction): a box (corner `+0..+8`, extent
  `+0x10..+0x18`) in clip space by the 4x4 `m`. The corner goes through
  `PMatrix4_TransformPoint_004c9cb0` (`t`); the extent gives `a = ext.x *
  row 0`, `b = ext.y * row 1`, `c = ext.z * row 2` (each product stored as
  a float); the eight corners are `t`, `t+a`, `t+b`, `t+c`, `(t+a)+b`,
  `(t+a)+c`, `(t+b)+c`, `((t+a)+b)+c`, every sum stored as a float, so a
  corner on a clip plane can fall either side by rounding. Their
  `ClipOutcode`s are ANDed (visible: 1 unless every corner is outside one
  plane) or ORed (inside: 1 only when every corner is inside); `eax` is
  1 when the result is 0, else 0.
- `graphicSetSpecularMapParamsEx(...)` (`0x006439b0`):
  `SpecularMapParamsManager_SetParams` on `SpecularMapParamsManager_GetInstance()`
  with its ten words (the floats copied through the x87).

Round 20:

- `PhyreScene_GetScreenAdjustedView_00651650(scene, out)` (`0x00651650`,
  `thiscall`, `ret 4`, returns `out`): `out` = the camera's 4x4 at
  `*(scene +0xdc) +0x34` times `[1 0 0 0; 0 4/3 0 0; 0 0 1 0; 0 d 0 1]`
  (row vectors), with `4/3` the `f32` at `0x00b43d34` and `d = 0.33333337
  - 2 * (scene +0xf9c0 + scene +0xf9bc)` (the `f64` at `0x00b43d40`; the
  sum and its double kept at 53 bits, `d` stored as a float). The
  adjustment matrix is first written into `out`; each camera row times it
  (`PMatrix4_RowTimes_004c9c20`, bottom row first) goes to a temporary,
  then the four rows are copied over `out`. `PhyreScene_getViewProjectionMatrix`
  inlines the same adjustment.

Round 21:

- `graphicCharacterLightingSetLightColor(kind, r, g, b)` (`0x0063cb00`):
  `Phyre_CharacterLighting_setLightColor_006aa940(kind, r, g, b)` on the
  lighting (the floats through the x87), then
  `Phyre_CharacterLightingTweakUtil_syncIntensity_006aa260` on the tweak
  utility, and for kind 3 (the ambient) a tail call of
  `balanceLumWithGlow`. Returns the last callee's `eax`, which
  `Sg_LightSetColor` (`0x00836310`) passes on.
- `graphicCharacterShaderSetParameterVector_0063d940(chr, name, x*, y*,
  z*, w*)` (`0x0063d940`, returns 0):
  `Phyre_CharacterShaderManager_setShaderParameterVector_006dfa30(inst,
  name, x*, y*, z*, w*)` for each instance of `Ch_GetChrRenderInstance(chr)`
  (count `& 0x7fffffff` and array `+4`, read again each pass).
- `BoundingBox_IsInsideTransformed_006d2a70(box, m)` (`0x006d2a70`,
  `thiscall`, `ret 4`): the box object's `+0xc` is an optional 3x4
  transform; with one, `BoundingBox_IsInside_006d27e0` runs on `m` times
  its 4x4 (`PMatrix4_FromMatrix3x4_00451080` into a local, then
  `PMatrix4_Multiply_004ca360(out, m, local)`, whose returned `out` is
  passed), without one on a local copy of `m`. Its answer is returned.

Round 22:

- `PhyreScene_Project3DPointOntoScreen(scene, out, p, mode)` (`0x006526a0`,
  `thiscall`, `ret 0xc`; Fahrenheit's name): the screen position of the
  point `p` (3 floats) into `out` (2 ints); returns `1 / w` as a float in
  `ST0`. The matrix:
  - `mode` 2: the scene's own 4x4 at `+0xc50`, with `p.x` and `p.y` first
    scaled in place by 3.75 and 2.8125 (`f64`s at `0x00b41d18`,
    `0x00b43d78`; stored back as floats into the caller's point);
  - a 2D scene (`g_PhyreScene3D`, `0x00ccb9c0`, clear) and `mode` != 0:
    `PMatrix4_Multiply_004ca360(view, inverse)`, `view` the screen-adjusted
    view (`0x00651650`), `inverse` = `PMatrix4_Inverse_004cc0c0` of the
    camera's world transform (the 3x4 at `*(scene +0xdc) +0xb4` made a 4x4
    by `PMatrix4_FromMatrix3x4_00451080`, or the identity
    `PMatrix4_Identity_00451bb0` when the pointer is 0);
  - else the camera's 4x4 at `*(scene +0xdc) +0x74`.

  The row vector `(-p.x, -p.y, p.z, 1)` times it (`PMatrix4_RowTimes_004c9c20`)
  gives `(x, y, z, w)`; `x / w` and `y / w` are stored as floats and negated
  when `w < 0` (a NaN `w` leaves them). `out[0]` = `ftol2_sse(x/w * W *
  0.5)`, `out[1]` = `ftol2_sse(-(y/w) * H * 0.5)`, with `W x H` = 512 x 416
  (`f64`s at `0x00b41d38`, `0x00b41d30`) for `mode` 2, else
  `graphicFontGetScreenWH`'s ints, each rounded to a float first; `0.5` is
  the `f64` at `0x00b92220`.
- `graphicGetAbmapUV_006410a0(kind, u, v, &u_out, &v_out)` (`0x006410a0`,
  cdecl): `SheetCellUV_00684b00` with `ecx` = `g_GraphicAbmap` (the callee
  never reads it) and the two floats through the x87; returns its `eax`.
- `IconPanel_showIcon(panel, i)` (`0x006f7970`, `thiscall`, `ret 4`): the
  panel's seven icon words lie at `+0`, their paths at `+0x1c`, their
  slots (bytes) at `+0x38`, its visibility (byte) at `+0x3f`. For `i < 7`
  (signed; else 0), an absent icon is opened by
  `IggyPlayerOpenScaled_006e9250(path, 10 + 0x35 * slot, 10, 0x2b, 0x2b)`
  and `IggyMenu_setEnable(icon, visible)`; returns the icon word.
- `SpdCtrl_ScaleElapsedTime(dt)` (`0x006f7340`, cdecl, result in `ST0`):
  with `g_SpdCtrl_Ready` set, `dt * g_SpdCtrlScales[level]` (`0x00c38e1c`,
  floats 1, 2, 4; `level` = `g_SpdCtrl_b4`, 0 = off; the product stored as
  a float) when all hold: in battle (`MsBattleGetStatus() == 1`) or
  `getUCStatus()`; `g_WaitFramesByTime` clear; `SpdCtrl_IsMapExcluded(
  g_CurrentMapNo)` clear (`al`; called only when the two before hold); not
  (in battle and `SpdCtrl_IsMagicExcluded()`); `g_BtlEndCtrlDone`
  (`0x00c5a068`, a byte) set; `g_SaveMachineBusy`, `graphicGetAbmap()` and
  `g_MovieFading` clear; `level` != 0. Else `dt`. `g_SpdCtrl_b8` keeps
  whether it scaled: on a change, `IggyMenu_callIggyAS3External(menu, 0,
  "OnEnable" or "OnDisable", 0, 0, 1)` when `g_SpdCtrl_c0` holds a menu,
  then `Sg_SetMuteSound(1 or 0)`. Before the ready check nothing is
  called.
- `Cache_OverFlow()` (`0x008409a0`): returns 1 when no entry is live
  (`+0xc` != 0), read (`+6` clear), unused (`+5` clear) and unpinned (age
  != 7); else frees one and returns 0. The scan keeps an age (from 100)
  and a data word (from `0x0fffffff`, compared signed): an entry of lower
  age takes the place and the age, then one whose age equals the kept one
  takes the place and the data word when its own is lower. The data word
  is not reset when the age drops, so the victim is not always the lowest
  data among its age, and data words at `0x10000000` and above win only
  by age. The victim is logged with `dbgPrintf` (`"Memory overflow!! "`,
  then `"[CHR"`/`"[MOT"`/`"[MAP"`/`"[BGM"` for kinds 1-4, then
  `":%s:%d] disposed from Cache."` with `ch_create_name(id)` and `id >>
  16` arithmetic), then `Cache_FreeEntry(victim)`; last comes a call of
  the empty `Cache_Empty_00840ab0`.

Round 23:

- `Cache_Clear()` (`0x00840030`) and `Cache_ClearFast()` (`0x00840150`),
  cdecl: over the slots, each live entry (`+0xc` set) not in use (`+5`
  clear) — for `Cache_ClearFast` also of age neither 5 nor 7 — gets
  `+0x24` = 1 while it is not read yet (`+6`), else is freed. Both end
  with a tail `jmp` to the empty `Cache_Empty_00840ab0`; `eax` is left
  from the last free (or, for `Cache_ClearFast`, partly the last age
  byte), which no caller reads.
- Freeing an entry (the four releasers share it): each model (`+0x10`,
  then `+0x14`) that is set is unloaded by
  `graphicCharacterUnloadTask_thunk_0063d9a0` unless
  `Cache_IsPersistentChr(id)` (`0x00840950`, always 0 in this build), then
  cleared; `+0x18` is freed by `user_free_thunk` and cleared;
  `Cache_FreeClothVertexBuff_00640e70(&e + 0x20)` when `+0x20` is set (the
  field is left to it); `+0x1c` as `+0x18`; then `Cache_FreeEntry(e)`.
- `Cache_DisposeCheck()` (`0x00840440`, cdecl): frees every live entry of
  age 0, whether in use or read or not.
- `Cache_Dispose(kind, id, check)` (`0x00840250`, cdecl): the first live
  entry `(kind, id)`; 1 when there is none, -1 when it is in use and
  `check` != 0, else it is freed and 0 returned. Before the first model,
  `graphicEnableChrClusterDelayRelease()` runs and the model is read again
  after it and `Cache_IsPersistentChr`.
- `graphicProject3DPointOntoScreenConsider2DScene(out, p)` (`0x006428f0`)
  and `graphicProject3DPointOntoScreenAbmap(out, p)` (`0x006428d0`), cdecl:
  `PhyreScene_Project3DPointOntoScreen(g_PhyreScene, out, p, 1 or 2)`, its
  `1 / w` left in `ST0`. (`graphicProject3DPointOntoScreen`, `0x006428b0`,
  mode 0, is dead.)

Round 23, the 2D scene's layers:

- The walk (`PhyreScene_change2DSceneNum`, `PhyreScene_changeLayerOffset_00658350`,
  `PhyreScene_changeLayerScale_00658570`): with the scene's 2D level loaded
  (byte `+0x93c`) and `+0x1128` set, each of the `+0x112c` lists at
  `+0x1134` gets a cursor on the stack: `+0x14` the end node (the list
  `+0x1c`), `+0x18` its first node (0 when the end node points at itself),
  `+0x1c` the class `0x00ca7228`, the first five words 0.
  `PhyreSceneCursor_Begin_00637730` seeks the first valid node and sets the
  cursor up (`PhyreSceneCursor_SetNode`). While the count (`+0xc`) is not 0,
  the instance at `+0x10` + position (`+0`) is visited; the position then
  steps by the stride (`+8`), the count drops, and where the position meets
  the link (`+4`) the link follows itself (`*link`) and the step repeats,
  so that slot is not visited. At the count's end
  `PhyreSceneCursor_Next_006377a0` moves to the next node.
- An instance's name is the pointer at `+0x2c` less its low bit. The
  original copies it without a bound into a stack buffer (100 bytes in
  `change2DSceneNum`, 52 in the layer functions) that lies just below its
  `sprintf`ed pattern (`"scene%d"`, 52 bytes; `"layer%d"`, 8 bytes, so
  layers past 99 or below -9 overrun into the stack cookie), then
  `strstr`s the pattern in it: a longer name overwrites the pattern, for
  the rest of the call.
- `PhyreScene_change2DSceneNum(scene, n)` (`0x006587d0`, `thiscall`, `ret
  4`): `+0xf9c4` = 1 (byte) when `n` is 1, `+0x11d8` = `n`; every visited
  instance with a target (`+0x1c`) sets the target's `+0x1c` to itself
  when its name holds `"scene<n>"`, else to 0.
- `PhyreScene_changeLayerOffset_00658350(scene, layer, x, y)` (`ret 0xc`)
  and `PhyreScene_changeLayerScale_00658570(scene, layer, x, y, sx, sy)`
  (`ret 0x14`): the first instance of each list whose name holds
  `"layer<layer>"` has its 12 floats (pointer `+4`) changed, which ends
  that list. `x` and `y` are ints made floats (`fild`, `fstp m32`). Offset:
  `[8] += x * [1]`, `[9] += y * [3]`. Scale (2.5 and 0.5 the doubles at
  `0x00b43d70`, `0x00b92220`): `[8] += (2.5 sx + [1]) * x * 0.5`, `[9] +=
  0.5 * ((2.5 sy + [3]) * y)`, then `[1] = -2.5 sx`, `[3] = -2.5 sy`. The
  floats go through the x87 on the way back (a signalling NaN comes out
  quiet).
- `graphicChangeLayerOffset_00639d40(layer, x, y)`,
  `graphicChangeLayerScale_00639d60(layer, x, y, sx, sy)` and
  `graphicChangeSceneNum_00639d90(n)` (cdecl; names ours, sorting after
  `graphicChangeLayerAlpha`): the three on `g_PhyreScene`.

Round 23, texture-animation samplers:

- `TextureAnimationManager_updateSamplers_006b44a0(this)` (`0x006b44a0`,
  fastcall; name ours): nothing unless `+4` is set. For each animated
  material `i` (`+0x78`, `+0x74` of them with a flag bit, the count read
  again each pass; a material has its shader at `+0`, its parameter
  buffer at `+4`, its animation's name, a `PString`, at `+0x10`):
  `TextureAnimation_FindByName` gives the animation; its frame (`+0x18`,
  -1 none, below `+8` with a flag bit) picks a slot in `+0xc` (-1 none),
  plus `+0x10` when `i >= +0xc`; the slot, below `+0x5c` (flag bit),
  picks the texture in `+0x60`. With a texture and a `"TextureSampler"`
  (`0x00b3017c`) parameter (`ShaderParams_FindByName` on the shader),
  `Render_SurfaceFormat_0066e4f0(buffer, param, &record)` gets the record
  `(0xff, 0, sampler, texture)`: `sampler` is `this + 0x38` when the
  parameter's slot (offset `+8` at least 4, size the low 13 bits of
  `+0xa`, offset + size within the buffer's first word) holds a value
  (`+8`) whose byte `+2` is 2, else `this + 0x14`. (The frame's -1 test is
  redundant: -1 is never below the 31-bit count.)

Round 24, the view-projection matrix and the speed control's tick
(`ffx_graphic::util::bridge24`):

- `PhyreScene_getViewProjectionMatrix(scene, out)` (`0x0065e660`,
  thiscall, `ret 4`, returns `out`): the free camera (`g_FreeCamera`,
  `0x00ccc830`) is made on first use (`operator_new(0x340)`,
  `FreeCamera_FreeCamera`, 0 when the allocation fails). While it is off
  (bytes `+0x338`, `+0x33c` and word `+0` clear), the scene's camera
  (`*(scene +0xdc)`) takes `scene +0xfa04` as its transform (`+0xb4`) and
  `Phyre_PCamera_updateViewMatrices` runs on camera `+4`. `out` = the
  camera's 4x4 at `+0x74` (`PMatrix4_Copy_00453340`) times the
  translation `[1 0 0 0; 0 1 0 0; 0 0 1 0; x y 0 1]`, `x`, `y` the floats
  at `scene +0xfa34`, `+0xfa38`. Unless byte `+0xf9c7` or
  `g_PhyreScene3D` is set, `out` then takes the screen adjustment of
  `PhyreScene_GetScreenAdjustedView_00651650` (`[1 0 0 0; 0 4/3 0 0; 0 0
  1 0; 0 d 0 1]`, `d = 0.33333337 - 2 (+0xf9c0 + +0xf9bc)` as a float),
  copied in by `PMatrix4_Copy_00453340`. Each product goes row by row
  (bottom first) through `PMatrix4_RowTimes_004c9c20` into locals, then
  back. Both snapshots have the free camera made and a 3D scene.
- `SpdCtrl_Tick()` (`0x006f7470`): once `g_SpdCtrl_Ready`, the speed-up
  button (`inputIsPressButton(0x10000)`) steps the level (`g_SpdCtrl_b4`,
  signed `(level + 1) % 3`) when `g_IggyMenuManager` (`0x01984c6c`),
  `TipSystem_Get()` and `IconPanel_get()` are all there: both icons hidden
  (`IconPanel_hideIcon` 0 and 1), `g_SpdCtrl_c0` cleared, then for level 1
  or 2 icon `level - 1` shown (`IconPanel_showIcon`, its menu into
  `g_SpdCtrl_c0`) and tip `0x2fd` / `0x2fe` pushed, else tip `0x2ff`. A
  shown icon while the speed-up is not running (`g_SpdCtrl_b8` clear)
  gets `"OnDisable"` (`IggyMenu_callIggyAS3External`). It then
  tail-jumps to `SpdCtrl_UpdateSubtitleMute_006f7230` (no function in
  Ghidra): in battle (`MsBattleCheck`) at a level other than 0 the battle
  subtitles are muted (`MsMuteBattleSubTitle(1)`), else unmuted, each
  only on a change of `g_SpdCtrl_SubtitleMuted` (`0x00ce82bd`, set before
  the call).

Round 24, group 2: shadow meshes and texture-animation names:

- `ShadowDynamicMesh_updateBuffers_006789f0(_, src, streams, indices)`
  (`0x006789f0`, `ShadowDynamicMesh_vftable` slot 5, `ret 0x10`, `ecx`
  unread; name ours): with `n` = `src +0x24`, `12 n` bytes from `src
  +0x10` go to vertex stream 0, `16 n` from `+0x14` to stream 1 (only
  when that stream has data), `8 n` from `+0x18` to stream 2 and `12 n`
  from `+0x1c` to stream 3 (`memcpy`; a stream is an object with its data
  at `+0x3c` and its count at `+4`, `streams` four pointers, 0 for none:
  streams 0, 2, 3 are written even without data). Each present stream's
  count = `n`; the `src +0x28` index words at `src +0x20` go one by one
  to `indices +0x38`, `indices +8` = their count, `src +4` = 0. Returns
  0.
- `graphicTextureAnimationSetAnimatedMaterialsChar_00643eb0(a, b, c, x,
  y, w, h, sw, sh, d, e)` (`0x00643eb0`, cdecl; name ours, sorting
  between `graphicSwitchGPUMode` and
  `graphicTextureAnimationSetMaterialVisiability`): two names, `"G"`
  (`*0x00c322a0`) + `a` and `"Animation"` (`*0x00c322a4`) + `b`, each
  number below 20 (signed: -3 reads the `"Animation"` pointer, other
  negatives null or unmapped ones) taken from the string table at
  `0x00c322b0`, else written by `sprintf "%d"`; then
  `TextureAnimationManager_setAnimatedMaterialsChar(s1, s2, c, x / sw, y
  / sh, w / sw, h / sh, d, e)` on the manager, each quotient a float, 0
  for a zero divisor (a NaN divides). Returns that call's `eax`.
- `graphicTextureAnimationSetMaterialUV_00644150(a, b, u, v, key)`
  (`0x00644150`, cdecl; name ours): the same names, both by `sprintf`,
  then `TextureAnimationManager_setMaterialUV_006b33c0(s1, s2, u, v,
  key)`.
- `TextureAnimationManager_setMaterialUV_006b33c0(mgr, s1, s2, u, v,
  key)` (`0x006b33c0`, thiscall, `ret 0x14`; name ours; the names are not
  read): `u' = u - 0.01`, `v' = v - 0.01` (floats). The manager's list
  (`+0x70`, `+0x6c` entries with a flag bit, 12 bytes: set index, key)
  gives `key`'s set (`+0x68`, 32 bytes each); a set with slots (`+8`)
  has its animation (`+0xc`) marked (byte `+0x34`). Each material of the
  animation (`+0x38`, an array) with a texture index (`+0x74`) and an
  object index (`+0x78`) both non-negative takes the texture
  `PArray_Data_004d45a0(*(key +4)[obj] +0x10)[tex]`; with it and
  `TextureAnimation_FindMaterial(anim, anim +0x18, set +0x18` without
  its low bit`)`, its parameters, each when set, go to the texture's
  buffer (`+4`) in this order: sampler `+4` (record `(0xff, 0, mgr
  +0x38, material)`, `Render_SurfaceFormat_0066e4f0`), scale `+0x50`
  `(1.02, 1.02)`, offset `+0x38` `(u', v')`
  (`ShaderParam_CopyToBuffer_00698960`), value `+0x20` 1.0
  (`ShaderParamBuffer_SetValue2`), then sampler `+8`, scale `+0x54`,
  offset `+0x3c` `(u', v' - 1)`, value `+0x24`.
- `graphicCameraGetViewProjectionMatrix(out)` (`0x00639590`, cdecl):
  `PhyreScene_getViewProjectionMatrix` of `g_PhyreScene` into a local,
  then its 16 words copied to `out`.
- `TexAnim_SetMaterialsChar_0077fbf0(a, b, c, x, y, w, h, sw, sh, d, e)`
  (`0x0077fbf0`, cdecl; name ours; called by `chr_texanim_main_sub_new`
  and `tex_anim_no_set_new`): nothing (eax 0) when `e` is 0, else
  `graphicTextureAnimationSetAnimatedMaterialsChar_00643eb0` with the
  six signed 16-bit values `x`..`sh` made floats; returns its `eax`.
